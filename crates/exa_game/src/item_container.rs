//! The free-form item panel of the retail GUI (`TGUIItemContainer`).
//!
//! Entries carry absolute cell positions; the array is a z-order, last on
//! top. There is no grid, no scroll and no selection: the layout passes
//! scatter the entries, relax their overlaps and clamp each cell's centre
//! inside the panel. The numbers here are the original's: a 40-unit cell
//! (160 px at the base unit of 4), a sixteenth-cell pad, an eighth-cell
//! jitter and the engine's `0x8088405` linear congruential generator.

/// The cell is 40 GUI units (160 px at the base unit of 4).
pub const CELL_UNITS: i32 = 40;
/// The layout pad is a sixteenth of the cell.
pub const PAD_DIV: i32 = 16;
/// The scatter jitter is an eighth of the cell.
pub const JITTER_DIV: i32 = 8;
/// The engine's linear congruential multiplier.
pub const LCG_MUL: u32 = 0x0808_8405;
/// The engine's linear congruential addend.
pub const LCG_ADD: u32 = 1;
/// The seed the "scatter everything" pass uses.
pub const SCATTER_SEED: u32 = 0x0080_b71a;
/// The seed the worn-equipment doll uses.
pub const DOLL_SEED: u32 = 0x0000_ba28;
/// Units per saved character-list position.
pub const LIST_UNITS: f64 = 16.0;
/// Units per saved DB-container position.
pub const DB_UNITS: f64 = 80.0;

/// The engine's linear congruential generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng(pub u32);

impl Rng {
    /// One step, then the high half of `state · k`.
    pub fn below(&mut self, k: u32) -> u32 {
        self.0 = self.0.wrapping_mul(LCG_MUL).wrapping_add(LCG_ADD);
        ((self.0 as u64 * k as u64) >> 32) as u32
    }
}

/// A stored cell position (the cell's top-left, client pixels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

impl Cell {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// The grid's columns and rows: `cols = round(√(n·W/H))`, `rows = ceil(n/cols)`.
pub fn grid_shape(count: usize, usable_w: i32, usable_h: i32) -> (i32, i32) {
    if count == 0 {
        return (0, 0);
    }
    let aspect = usable_w as f64 / usable_h as f64;
    let cols = ((count as f32) * aspect as f32)
        .sqrt()
        .round_ties_even()
        .max(1.0) as i32;
    let rows = (count as f64 / cols as f64).ceil() as i32;
    (cols, rows)
}

/// The ideal slot for `index` (top-left), before jitter and relaxation.
/// Entries fill backwards: the last entry sits in row 0, column 0.
pub fn grid_cell(count: usize, usable_w: i32, usable_h: i32, cell: i32, index: usize) -> Cell {
    let (cols, rows) = grid_shape(count, usable_w, usable_h);
    if cols <= 0 || rows <= 0 || index >= count {
        return Cell::default();
    }
    let half = cell >> 1;
    let pad = cell / PAD_DIV;
    let mut remaining = count as i32;
    for row in 0..rows {
        for col in 0..cols {
            if remaining <= 0 {
                break;
            }
            let div = cols.min(remaining);
            let cx = (((col as f64) + 0.5) * usable_w as f64 / div as f64).round_ties_even() as i32
                - pad;
            let cy = (((row as f64) + 0.5) * usable_h as f64 / rows as f64).round_ties_even()
                as i32
                - pad;
            if remaining as usize - 1 == index {
                return Cell::new(cx - half, cy - half);
            }
            remaining -= 1;
        }
    }
    Cell::default()
}

/// The full grid pass: ideal centres, jitter, five relax passes, clamp, then
/// the centre-to-top-left shift.
pub fn grid_layout(
    count: usize,
    usable_w: i32,
    usable_h: i32,
    cell: i32,
    halves: &[[i32; 2]],
    seed: u32,
) -> Vec<Cell> {
    let (cols, rows) = grid_shape(count, usable_w, usable_h);
    if count == 0 || cols <= 0 || rows <= 0 {
        return Vec::new();
    }
    let pad = cell / PAD_DIV;
    let jitter = cell / JITTER_DIV;
    let mut rng = Rng(seed);
    let mut pos: Vec<[i32; 2]> = Vec::with_capacity(count);
    let mut remaining = count as i32;
    'outer: for row in 0..rows {
        for col in 0..cols {
            if remaining <= 0 {
                break 'outer;
            }
            let div = cols.min(remaining);
            let cx = (((col as f64) + 0.5) * usable_w as f64 / div as f64).round_ties_even() as i32
                - pad
                + rng.below(jitter as u32) as i32;
            let cy = (((row as f64) + 0.5) * usable_h as f64 / rows as f64).round_ties_even()
                as i32
                - pad
                + rng.below(jitter as u32) as i32;
            pos.push([cx, cy]);
            remaining -= 1;
        }
    }
    for pass in 4..=8 {
        for i in 0..count {
            for j in i + 1..count {
                relax_pair(&mut pos, halves, i, j, pass);
            }
        }
    }
    clamp_centres(&mut pos, halves, usable_w, usable_h);
    pos.into_iter()
        .map(|[x, y]| Cell::new(x - (cell >> 1), y - (cell >> 1)))
        .collect()
}

/// Pushes one pair apart by their overlap divided by `pass` (the original's
/// idiv truncation); slack is 2 when the second is to the right, else 1.
fn relax_pair(pos: &mut [[i32; 2]], halves: &[[i32; 2]], i: usize, j: usize, pass: i32) {
    for axis in 0..2 {
        let hi = halves[i][axis];
        let hj = halves[j][axis];
        let push = if pos[j][axis] > pos[i][axis] {
            (pos[i][axis] + hi) - (pos[j][axis] - hj) + 2
        } else {
            (pos[j][axis] + hj) - (pos[i][axis] - hi) + 1
        };
        if push <= pass {
            continue;
        }
        let p = if pos[j][axis] > pos[i][axis] {
            push / pass
        } else {
            -(push / pass)
        };
        pos[i][axis] -= p;
        pos[j][axis] += p;
    }
}

/// Pushes `target` out of every other entry over five passes, then clamps.
/// `pos` holds centres.
pub fn relax_one(
    pos: &mut [[i32; 2]],
    halves: &[[i32; 2]],
    target: usize,
    usable_w: i32,
    usable_h: i32,
) {
    let count = pos.len();
    if target >= count {
        return;
    }
    for pass in 1..=5 {
        for other in 0..count {
            if other == target {
                continue;
            }
            let ht = halves[target];
            let ho = halves[other];
            for axis in 0..2 {
                let push = if pos[other][axis] > pos[target][axis] {
                    (pos[target][axis] + ht[axis]) - (pos[other][axis] - ho[axis]) + 2
                } else {
                    (pos[other][axis] + ho[axis]) - (pos[target][axis] - ht[axis]) + 1
                };
                if push <= pass {
                    continue;
                }
                let p = if pos[other][axis] > pos[target][axis] {
                    -(push / pass)
                } else {
                    push / pass
                };
                pos[target][axis] += p;
                pos[other][axis] -= p;
            }
        }
    }
    clamp_centres(pos, halves, usable_w, usable_h);
}

/// Scatters every entry from `seed` and resolves every pair. `pos` holds
/// centres; both entries move.
pub fn scatter_all(
    pos: &mut [[i32; 2]],
    halves: &[[i32; 2]],
    usable_w: i32,
    usable_h: i32,
    seed: u32,
) {
    if pos.is_empty() {
        return;
    }
    let mut rng = Rng(seed);
    let half_w = usable_w >> 1;
    let half_h = usable_h >> 1;
    for p in pos.iter_mut() {
        p[0] = rng.below(half_w as u32) as i32 + half_w / 2;
        p[1] = rng.below(half_h as u32) as i32 + half_h / 2;
    }
    for pass in 4..=8 {
        let q = pass / 2;
        for i in 0..pos.len() {
            for k in i + 1..pos.len() {
                for axis in 0..2 {
                    let hi = halves[i][axis];
                    let hk = halves[k][axis];
                    let push = if pos[k][axis] > pos[i][axis] {
                        (pos[i][axis] + hi) - (pos[k][axis] - hk) + 2
                    } else {
                        (pos[k][axis] + hk) - (pos[i][axis] - hi) + 1
                    };
                    if push <= 0 {
                        continue;
                    }
                    let p = if pos[k][axis] > pos[i][axis] {
                        push / q
                    } else {
                        -(push / q)
                    };
                    pos[i][axis] -= p;
                    pos[k][axis] += p;
                }
            }
        }
    }
    clamp_centres(pos, halves, usable_w, usable_h);
}

/// Keeps each centre inside `[half, usable − half]`, max first then min.
fn clamp_centres(pos: &mut [[i32; 2]], halves: &[[i32; 2]], usable_w: i32, usable_h: i32) {
    for (p, h) in pos.iter_mut().zip(halves) {
        if p[0] > usable_w - h[0] {
            p[0] = usable_w - h[0];
        }
        if p[0] < h[0] {
            p[0] = h[0];
        }
        if p[1] > usable_h - h[1] {
            p[1] = usable_h - h[1];
        }
        if p[1] < h[1] {
            p[1] = h[1];
        }
    }
}

/// The topmost entry whose `cell × cell` box contains the point and whose
/// alpha is positive. Entries with `objflags & 0x100` are skipped; drag
/// ghosts are not. `entry` returns (position, objflags, alpha).
pub fn hit_test<F>(count: usize, cell: i32, x: i32, y: i32, mut entry: F) -> Option<usize>
where
    F: FnMut(usize) -> (Cell, u32, bool),
{
    if cell <= 0 {
        return None;
    }
    for i in (0..count).rev() {
        let (p, objflags, alpha) = entry(i);
        if objflags & 0x100 != 0 {
            continue;
        }
        if x >= p.x && x < p.x + cell && y >= p.y && y < p.y + cell && alpha {
            return Some(i);
        }
    }
    None
}

/// Removes index `i`, shifting the tail left; indices above `i` drop by one.
pub fn remove_entry<T>(items: &mut Vec<T>, i: usize) -> Option<T> {
    if i >= items.len() {
        return None;
    }
    Some(items.remove(i))
}

/// Raises index `i` to the end (drawn last, on top) and returns its new index.
pub fn raise<T>(items: &mut Vec<T>, i: usize) -> Option<usize> {
    let item = remove_entry(items, i)?;
    items.push(item);
    Some(items.len() - 1)
}

/// The drop position: cursor minus the grab offset, then clamped around the
/// icon's half extents. `cell` is the layout cell (its half is the grab
/// centre).
pub fn drop_cell(
    cursor: Cell,
    grab: Cell,
    half_w: i32,
    half_h: i32,
    usable_w: i32,
    usable_h: i32,
    cell: i32,
) -> Cell {
    let half = cell >> 1;
    let mut x = cursor.x - grab.x;
    let mut y = cursor.y - grab.y;
    if x + half + half_w > usable_w {
        x = usable_w - half_w - half;
    }
    if x + half < half_w {
        x = half_w - half;
    }
    if y + half < half_h {
        y = half_h - half;
    }
    if y + half + half_h > usable_h {
        y = usable_h - half_h - half;
    }
    Cell::new(x, y)
}

/// What a container accepts from an `0xFB01` drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accept {
    pub capacity: i32,
    pub used: i32,
    pub max_item_size: i32,
    pub reject_mask: u32,
    pub required_mask: u32,
    pub required_value: u32,
    pub only_own: bool,
    pub container_id: u32,
    pub linked: Option<u32>,
    /// The container's own id (the `this` pointer in the original).
    pub this: u32,
}

/// The drag payload's item facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drag {
    pub src: Option<u32>,
    pub dst: Option<u32>,
    pub objid: u32,
    pub objflags: u32,
    pub icon: u32,
    /// The model record's body byte at `+0x1c` (the item's size), if any.
    pub bulk: u8,
    pub model_found: bool,
}

/// The container's accept test, without callbacks. `own_icon` reports
/// whether one of this container's entries carries the drag's icon.
pub fn accepts<F>(a: &Accept, d: &Drag, mut own_icon: F) -> bool
where
    F: FnMut(u32) -> bool,
{
    if d.objflags & a.reject_mask != 0 {
        return false;
    }
    if d.objid == a.container_id {
        return false;
    }
    if !d.model_found {
        return false;
    }
    if a.max_item_size > 0 && d.bulk as i32 > a.max_item_size {
        return false;
    }
    let from_linked = a.linked.is_some() && d.src == a.linked;
    if !from_linked
        && d.src != Some(a.this)
        && a.capacity > 0
        && d.bulk as i32 + a.used > a.capacity
    {
        return false;
    }
    if d.objflags & a.required_mask != a.required_value {
        return false;
    }
    if !a.only_own {
        return true;
    }
    own_icon(d.icon)
}

/// A character-list position in sixteenths of a GUI unit.
pub fn to_list_pos(px: i32, unit: i32, vscale: f32) -> (i16, i16) {
    let x = (px as f64 / unit as f64 * LIST_UNITS).round() as i16;
    let y = (px as f64 / unit as f64 * LIST_UNITS / vscale as f64).round() as i16;
    (x, y)
}

/// The inverse of [`to_list_pos`].
pub fn from_list_pos(v: (i16, i16), unit: i32, vscale: f32) -> Cell {
    Cell::new(
        (v.0 as f64 / LIST_UNITS * unit as f64).round() as i32,
        (v.1 as f64 / LIST_UNITS * unit as f64 * vscale as f64).round() as i32,
    )
}

/// A DB-container position in eightieths of a GUI unit.
pub fn to_db_pos(px: i32, unit: i32) -> i16 {
    (px as f64 / unit as f64 * DB_UNITS).round() as i16
}

/// The inverse of [`to_db_pos`].
pub fn from_db_pos(v: i16, unit: i32) -> i32 {
    (v as f64 / DB_UNITS * unit as f64).round() as i32
}

/// A popup container's inner panel from its record's size in cells.
pub fn popup_inner(size_x: u16, size_y: u16, unit: i32, border: i32) -> (i32, i32) {
    (
        size_x as i32 * unit + 2 * border,
        size_y as i32 * unit + 2 * border,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_shape_follows_the_panel_aspect() {
        assert_eq!(grid_shape(4, 400, 300), (2, 2));
        assert_eq!(grid_shape(1, 400, 300), (1, 1));
        assert_eq!(grid_shape(3, 400, 300), (2, 2));
        assert_eq!(grid_shape(0, 400, 300), (0, 0));
    }

    #[test]
    fn the_grid_fills_backwards_from_the_last_entry() {
        assert_eq!(grid_cell(3, 400, 300, 160, 2), Cell::new(10, -15));
        assert_eq!(grid_cell(3, 400, 300, 160, 1), Cell::new(210, -15));
        assert_eq!(grid_cell(3, 400, 300, 160, 0), Cell::new(110, 135));
    }

    #[test]
    fn the_generator_matches_the_engine() {
        let mut rng = Rng(0);
        assert_eq!(rng.below(20), 0);
        let mut rng = Rng(0x0080_b71a);
        let a = rng.below(20);
        let b = rng.below(20);
        assert!(a < 20 && b < 20);
        assert_ne!((a, b), (0, 0));
    }

    #[test]
    fn one_entry_relaxes_away_from_another() {
        let mut pos = [[150, 100], [100, 100]];
        relax_one(&mut pos, &[[80, 80], [80, 80]], 0, 400, 400);
        assert_eq!(pos[0], [261, 261]);
        assert_eq!(pos[1], [80, 80]);
    }

    #[test]
    fn the_grid_layout_is_deterministic_and_inside_the_panel() {
        let halves = [[80, 80]; 6];
        let a = grid_layout(6, 400, 400, 160, &halves, 7);
        let b = grid_layout(6, 400, 400, 160, &halves, 7);
        assert_eq!(a, b);
        assert_eq!(a.len(), 6);
        for cell in a {
            assert!(cell.x >= 0 && cell.y >= 0);
            assert!(cell.x + 160 <= 400 && cell.y + 160 <= 400);
        }
    }

    #[test]
    fn scattering_is_deterministic_and_clamped() {
        let halves = [[80, 80]; 5];
        let mut a = vec![[0, 0]; 5];
        let mut b = vec![[0, 0]; 5];
        scatter_all(&mut a, &halves, 400, 400, SCATTER_SEED);
        scatter_all(&mut b, &halves, 400, 400, SCATTER_SEED);
        assert_eq!(a, b);
        for p in a {
            assert!((80..=320).contains(&p[0]));
            assert!((80..=320).contains(&p[1]));
        }
    }

    #[test]
    fn the_hit_test_takes_the_topmost_cell() {
        let cells = [Cell::new(10, -15), Cell::new(210, -15)];
        let hit = hit_test(2, 160, 215, -15, |i| (cells[i], 0, true));
        assert_eq!(hit, Some(1));
        assert_eq!(hit_test(2, 160, 200, -5, |i| (cells[i], 0, true)), None);
        assert_eq!(hit_test(2, 160, 11, -14, |i| (cells[i], 0, true)), Some(0));
        assert_eq!(
            hit_test(2, 160, 215, -15, |i| (cells[i], 0x100, true)),
            None
        );
        let same = [Cell::new(0, 0), Cell::new(0, 0)];
        assert_eq!(hit_test(2, 160, 5, 5, |i| (same[i], 0, true)), Some(1));
    }

    #[test]
    fn removing_and_raising_remap_the_tail() {
        let mut items = vec!['A', 'B', 'C'];
        assert_eq!(raise(&mut items, 1), Some(2));
        assert_eq!(items, vec!['A', 'C', 'B']);
        assert_eq!(remove_entry(&mut items, 1), Some('C'));
        assert_eq!(items, vec!['A', 'B']);
        assert_eq!(remove_entry(&mut items, 9), None);
    }

    #[test]
    fn the_drop_position_is_clamped_around_the_icon() {
        let p = drop_cell(Cell::new(100, 50), Cell::new(80, 80), 30, 20, 400, 300, 160);
        assert_eq!(p, Cell::new(20, -30));
        let p = drop_cell(
            Cell::new(390, 290),
            Cell::new(80, 80),
            30,
            20,
            400,
            300,
            160,
        );
        assert_eq!(p, Cell::new(290, 200));
    }

    #[test]
    fn the_accept_rules() {
        let a = Accept {
            capacity: 10,
            used: 6,
            max_item_size: 8,
            reject_mask: 0x4000,
            required_mask: 0x2,
            required_value: 0x2,
            only_own: false,
            container_id: 77,
            linked: Some(5),
            this: 9,
        };
        let mut d = Drag {
            src: Some(3),
            dst: None,
            objid: 12,
            objflags: 0x2,
            icon: 4,
            bulk: 5,
            model_found: true,
        };
        assert!(!accepts(&a, &d, |_| false));
        d.src = Some(5);
        assert!(accepts(&a, &d, |_| false));
        d.src = Some(3);
        d.objflags |= 0x4000;
        assert!(!accepts(&a, &d, |_| false));
        d.objflags = 0x2;
        d.objid = 77;
        assert!(!accepts(&a, &d, |_| false));
        d.objid = 12;
        d.bulk = 9;
        assert!(!accepts(&a, &d, |_| false));
    }

    #[test]
    fn the_position_codecs_round_trip() {
        assert_eq!(to_list_pos(140, 4, 1.0), (560, 560));
        assert_eq!(to_db_pos(140, 4), 2800);
        assert_eq!(from_list_pos((560, 560), 4, 1.0), Cell::new(140, 140));
        assert_eq!(from_db_pos(2800, 4), 140);
        assert_eq!(to_list_pos(140, 4, 2.0).1, 280);
    }

    #[test]
    fn the_popup_inner_panel() {
        assert_eq!(popup_inner(10, 6, 4, 4), (48, 32));
    }
}

/// What a mouse-down on the panel asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseIntent {
    /// Nothing to do (or the base form handles it).
    None,
    /// Start dragging the hit entry; the grab offset is the cell's centre.
    StartDrag { grab: Cell },
    /// Use the hit item.
    Use,
    /// Open the hit container.
    Open,
    /// Show the hit item's card.
    Card,
}

/// The original's mouse-down decision for the panel. `button` is the event's
/// bit field: 0x2 right, 0x4000 drag start, 0x8000 use/open, 0x2000 card.
/// `hit` is the 1-based hover index minus one (None for 0 or −1).
pub fn mouse_intent(
    button: u16,
    hit: Option<usize>,
    allow_drag: bool,
    is_container: bool,
    cell: i32,
) -> MouseIntent {
    if button & 0x2 != 0 {
        return MouseIntent::None;
    }
    if button & 0x4000 != 0 {
        if !allow_drag || hit.is_none() {
            return MouseIntent::None;
        }
        return MouseIntent::StartDrag {
            grab: drag_grab(cell),
        };
    }
    if button & 0x8000 != 0 {
        if hit.is_none() {
            return MouseIntent::None;
        }
        return if is_container {
            MouseIntent::Open
        } else {
            MouseIntent::Use
        };
    }
    if button & 0x2000 != 0 && hit.is_some() {
        return MouseIntent::Card;
    }
    MouseIntent::None
}

/// The grab offset once a drag has started: the cell's centre, not the
/// click's offset inside it.
pub fn drag_grab(cell: i32) -> Cell {
    Cell::new(cell >> 1, cell >> 1)
}

/// What the source panel does when a drag ends without a drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragEnd {
    /// The cancelled drag keeps the entry in place.
    Cancel,
    /// Drag-out is refused: the entry snaps back with the payload's flags.
    SnapBack,
    /// The entry moved to the linked panel.
    Transfer,
    /// The entry leaves the panel.
    Remove,
}

/// The drag-ended decision. `dragged` says whether an entry carries the drag
/// flag; `linked` says the payload's destination is this panel's linked one.
pub fn drag_end(
    dragged: bool,
    cancelled: bool,
    allowed_out: bool,
    linked: bool,
) -> Option<DragEnd> {
    if !dragged {
        return None;
    }
    if cancelled {
        return Some(DragEnd::Cancel);
    }
    if !allowed_out {
        return Some(DragEnd::SnapBack);
    }
    if linked {
        return Some(DragEnd::Transfer);
    }
    Some(DragEnd::Remove)
}

#[cfg(test)]
mod mouse_tests {
    use super::*;

    #[test]
    fn the_mouse_down_intents() {
        assert_eq!(
            mouse_intent(0x2, Some(1), true, false, 160),
            MouseIntent::None
        );
        assert_eq!(
            mouse_intent(0x4000, Some(1), true, false, 160),
            MouseIntent::StartDrag {
                grab: Cell::new(80, 80)
            }
        );
        assert_eq!(
            mouse_intent(0x4000, Some(1), false, false, 160),
            MouseIntent::None
        );
        assert_eq!(
            mouse_intent(0x8000, Some(1), true, true, 160),
            MouseIntent::Open
        );
        assert_eq!(
            mouse_intent(0x8000, Some(1), true, false, 160),
            MouseIntent::Use
        );
        assert_eq!(
            mouse_intent(0x2000, Some(0), true, false, 160),
            MouseIntent::Card
        );
        assert_eq!(
            mouse_intent(0x1, Some(0), true, false, 160),
            MouseIntent::None
        );
        assert_eq!(drag_grab(160), Cell::new(80, 80));
    }

    #[test]
    fn the_drag_end_decisions() {
        assert_eq!(drag_end(false, false, true, false), None);
        assert_eq!(drag_end(true, true, true, false), Some(DragEnd::Cancel));
        assert_eq!(drag_end(true, false, false, false), Some(DragEnd::SnapBack));
        assert_eq!(drag_end(true, false, true, true), Some(DragEnd::Transfer));
        assert_eq!(drag_end(true, false, true, false), Some(DragEnd::Remove));
    }
}

/// One entry of a panel: its cell (top-left), the item, its icon slot, its
/// flags and its drag mark (bit 0 while it is being dragged).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Entry {
    pub cell: Cell,
    pub id: u32,
    pub icon: usize,
    pub flags: u32,
    pub drag: u32,
}

/// An entry with this flag is the item in a hand: it is neither drawn nor
/// hit.
pub const HELD: u32 = 0x100;

/// The tint an entry is drawn with: green for the green highlight mask,
/// yellow for the yellow one alone, at half opacity when the yellow mask
/// matches, else black at full opacity. Dragged, held and icon-less
/// entries are not drawn (`None`).
pub fn tint(e: &Entry, green: u32, yellow: u32) -> Option<[u8; 4]> {
    if e.drag & 1 != 0 || e.flags & HELD != 0 || e.icon == 0 {
        return None;
    }
    let (g, y) = (e.flags & green != 0, e.flags & yellow != 0);
    Some(match (g, y) {
        (true, false) => [0, 0xFF, 0, 0xFF],
        (true, true) => [0, 0xFF, 0, 0x7F],
        (false, true) => [0xFF, 0xFF, 0, 0x7F],
        (false, false) => [0, 0, 0, 0xFF],
    })
}

/// Keeps an entry's icon inside the panel: its centre (the cell plus half
/// the icon size) at least its half extent from each edge of the client
/// area `width` by `height` (already less the borders). The right edge is
/// checked before the left and the bottom before the top, as the panel's
/// load does.
pub fn clamp_entry(cell: &mut Cell, half: (i32, i32), icon_size: i32, width: i32, height: i32) {
    let h = icon_size >> 1;
    if width < half.0 + cell.x + h {
        cell.x = width - half.0 - h;
    }
    if h + cell.x < half.0 {
        cell.x = half.0 - h;
    }
    if height < half.1 + cell.y + h {
        cell.y = height - half.1 - h;
    }
    if h + cell.y < half.1 {
        cell.y = half.1 - h;
    }
}

/// The cell a new entry takes: the panel's centre less half an icon.
pub fn centre_cell(width: u32, height: u32, icon_size: u32) -> Cell {
    Cell::new(
        ((width >> 1) - (icon_size >> 1)) as i32,
        ((height >> 1) - (icon_size >> 1)) as i32,
    )
}

/// What replacing an entry's item needs to know.
pub struct Replacement<'a> {
    pub id: u32,
    pub icon: usize,
    pub flags: u32,
    pub bulk: &'a dyn Fn(u32) -> i32,
    /// The new icon's half extents, the icon size and the client area.
    pub half: (i32, i32),
    pub icon_size: i32,
    pub client: (i32, i32),
}

/// Puts another item in entry `at`: the panel's used bulk loses the old
/// item's and gains the new one's, and the entry is kept inside the panel
/// (right, left, top, then bottom). Returns the old icon slot, which the
/// caller releases.
pub fn replace_entry(
    entries: &mut [Entry],
    used: &mut i32,
    at: usize,
    r: &Replacement<'_>,
) -> Option<usize> {
    let e = entries.get_mut(at)?;
    *used -= (r.bulk)(e.id);
    let old = e.icon;
    e.id = r.id;
    e.icon = r.icon;
    e.flags = r.flags;
    let h = r.icon_size >> 1;
    if r.client.0 < r.half.0 + e.cell.x + h {
        e.cell.x = r.client.0 - r.half.0 - h;
    }
    if h + e.cell.x < r.half.0 {
        e.cell.x = r.half.0 - h;
    }
    if e.cell.y + h < r.half.1 {
        e.cell.y = r.half.1 - h;
    }
    if r.client.1 < h + e.cell.y + r.half.1 {
        e.cell.y = r.client.1 - r.half.1 - h;
    }
    *used += (r.bulk)(r.id);
    Some(old)
}

/// A held entry whose item comes back to the panel is shown again: the
/// first entry of `id` marked held loses the mark. False when none is.
pub fn take_back_held(entries: &mut [Entry], id: u32) -> bool {
    match entries
        .iter_mut()
        .find(|e| e.id == id && e.flags & HELD != 0)
    {
        Some(e) => {
            e.flags &= !HELD;
            true
        }
        None => false,
    }
}

/// The dragged entry was taken into a hand: its drag mark goes and it is
/// marked held. Returns its index; with nothing dragged the original
/// returns the last index.
pub fn hold_dragged(entries: &mut [Entry]) -> Option<usize> {
    match entries.iter().position(|e| e.drag & 1 != 0) {
        Some(i) => {
            entries[i].drag = 0;
            entries[i].flags |= HELD;
            Some(i)
        }
        None => entries.len().checked_sub(1),
    }
}

/// Removes the held entry with icon slot `icon`; false when there is none.
pub fn remove_held_icon(entries: &mut Vec<Entry>, icon: usize) -> bool {
    match entries
        .iter()
        .position(|e| e.icon == icon && e.flags & HELD != 0)
    {
        Some(i) => {
            entries.remove(i);
            true
        }
        None => false,
    }
}

/// A mouse release clears every drag mark.
pub fn clear_drags(entries: &mut [Entry]) {
    for e in entries {
        e.drag &= !1;
    }
}

/// The bulk of an item: a copy's (an id with a database tag) is its
/// template's, read through `template`; then the model's bulk byte, or 0
/// without a model.
pub fn bulk_of(
    id: u32,
    template: &dyn Fn(u32) -> Option<u32>,
    model_bulk: &dyn Fn(u32) -> Option<u8>,
) -> u8 {
    let base = if id & 0xC000_0000 != 0 {
        match template(id) {
            Some(t) => t,
            None => return 0,
        }
    } else {
        id
    };
    model_bulk(base).unwrap_or(0)
}

/// The worn and shield panel of a body being looted: each worn item and
/// each held shield whose icon the cache gives (`icon` returns its slot and
/// half extents; an item without one is left out and draws no random
/// numbers) is placed at a random spot in the middle half of the panel,
/// then the entries push each other apart over four rounds (overlap
/// divided by 6, 5, 4 and 3, twice each), each pass ending with every
/// centre kept inside the panel; the centres then become cells.
pub fn worn_layout(
    items: &[(u32, u32)],
    icon: &mut dyn FnMut(u32) -> Option<(usize, (i32, i32))>,
    client: (i32, i32),
    icon_size: i32,
    state: &mut u32,
) -> Vec<Entry> {
    let (hw, hh) = (client.0 >> 1, client.1 >> 1);
    let mut out: Vec<Entry> = Vec::new();
    let mut pos: Vec<[i32; 2]> = Vec::new();
    let mut half: Vec<[i32; 2]> = Vec::new();
    for &(id, flags) in items {
        let Some((slot, (a, b))) = icon(id) else {
            continue;
        };
        let mut rng = Rng(*state);
        let x = (hw as u32 >> 1) as i32 + rng.below(hw as u32) as i32;
        let y = (hh as u32 >> 1) as i32 + rng.below(hh as u32) as i32;
        *state = rng.0;
        pos.push([x, y]);
        half.push([a, b]);
        out.push(Entry {
            cell: Cell::new(x, y),
            id,
            icon: slot,
            flags,
            drag: 0,
        });
    }
    let n = pos.len();
    let mut d = 7;
    loop {
        d -= 1;
        for _ in 0..2 {
            for i in 0..n {
                for j in i + 1..n {
                    for axis in 0..2 {
                        if pos[i][axis] < pos[j][axis] {
                            let o =
                                (half[i][axis] + pos[i][axis]) - (pos[j][axis] - half[j][axis]) + 1;
                            if d < o {
                                let p = o / d;
                                pos[i][axis] -= p;
                                pos[j][axis] += p;
                            }
                        } else {
                            let o =
                                (pos[j][axis] + half[j][axis]) - (pos[i][axis] - half[i][axis]) + 1;
                            if d < o {
                                let p = o / d;
                                pos[i][axis] += p;
                                pos[j][axis] -= p;
                            }
                        }
                    }
                }
            }
            for i in 0..n {
                if client.0 < half[i][0] + pos[i][0] {
                    pos[i][0] = client.0 - half[i][0];
                }
                if pos[i][0] < half[i][0] {
                    pos[i][0] = half[i][0];
                }
                if pos[i][1] < half[i][1] {
                    pos[i][1] = half[i][1];
                }
                if client.1 < half[i][1] + pos[i][1] {
                    pos[i][1] = client.1 - half[i][1];
                }
            }
        }
        if d <= 3 {
            break;
        }
    }
    let shift = icon_size >> 1;
    for (e, p) in out.iter_mut().zip(&pos) {
        e.cell = Cell::new(p[0] - shift, p[1] - shift);
    }
    out
}

/// A map's parchment: its size is the map texture's in sixths of a unit.
pub fn parchment_size(texture: (u16, u16), unit: i32) -> (i32, i32) {
    let k = unit as f32 * 0.166_666_67;
    (
        (f32::from(texture.0) * k).round_ties_even() as i32,
        (f32::from(texture.1) * k).round_ties_even() as i32,
    )
}

/// The container's form flags and style word from its constructor.
pub const FORM_FLAGS: u32 = 0x80;
pub const STYLE: u32 = 0x0010_0010;

/// One child of a container record: the item, its flags and its stored
/// position in eightieths of a GUI unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContainerChild {
    pub id: u32,
    pub flags: u32,
    pub x: i16,
    pub y: i16,
}

/// The fields a panel loads from a container's record section: the panel's
/// size in units, its capacity and item limit, and its children. The
/// original's section is an object-database record; these are the fields
/// the panel reads (at the header offsets the pseudo-code shows). Bytes
/// past the children are not kept.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContainerRecord {
    pub size: (u16, u16),
    pub capacity: u16,
    pub max_item: u16,
    pub children: Vec<ContainerChild>,
}

/// The header of a container record section, in bytes.
pub const CONTAINER_HEADER: usize = 0x20;
/// One child entry of a container record, in bytes.
pub const CONTAINER_CHILD: usize = 12;

impl ContainerRecord {
    /// Reads the fields the panel uses. `None` for a section shorter than
    /// its header or whose child count does not fit.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < CONTAINER_HEADER {
            return None;
        }
        let field = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let count = u32::from_le_bytes([bytes[0x1c], bytes[0x1d], bytes[0x1e], bytes[0x1f]]);
        let count = usize::try_from(count).ok()?;
        let end = CONTAINER_HEADER.checked_add(count.checked_mul(CONTAINER_CHILD)?)?;
        if end > bytes.len() {
            return None;
        }
        let mut children = Vec::with_capacity(count);
        for at in 0..count {
            let base = CONTAINER_HEADER + at * CONTAINER_CHILD;
            children.push(ContainerChild {
                id: u32::from_le_bytes([
                    bytes[base],
                    bytes[base + 1],
                    bytes[base + 2],
                    bytes[base + 3],
                ]),
                flags: u32::from_le_bytes([
                    bytes[base + 4],
                    bytes[base + 5],
                    bytes[base + 6],
                    bytes[base + 7],
                ]),
                x: i16::from_le_bytes([bytes[base + 8], bytes[base + 9]]),
                y: i16::from_le_bytes([bytes[base + 10], bytes[base + 11]]),
            });
        }
        Some(Self {
            size: (field(0x8), field(0x0a)),
            capacity: field(0x10),
            max_item: field(0x12),
            children,
        })
    }

    /// Writes the fields back: the size at `0x8`/`0xa`, the capacity at
    /// `0x10`, the item limit at `0x12` and the children after the header.
    pub fn write(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; CONTAINER_HEADER];
        bytes[0x08..0x0a].copy_from_slice(&self.size.0.to_le_bytes());
        bytes[0x0a..0x0c].copy_from_slice(&self.size.1.to_le_bytes());
        bytes[0x10..0x12].copy_from_slice(&self.capacity.to_le_bytes());
        bytes[0x12..0x14].copy_from_slice(&self.max_item.to_le_bytes());
        bytes[0x1c..0x20].copy_from_slice(&(self.children.len() as u32).to_le_bytes());
        for child in &self.children {
            bytes.extend_from_slice(&child.id.to_le_bytes());
            bytes.extend_from_slice(&child.flags.to_le_bytes());
            bytes.extend_from_slice(&child.x.to_le_bytes());
            bytes.extend_from_slice(&child.y.to_le_bytes());
        }
        bytes
    }
}

/// A character-list entry: the item, its flags and its position in
/// sixteenths of a GUI unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ListChild {
    pub id: u32,
    pub flags: u32,
    pub x: i16,
    pub y: i16,
}

/// Reads a character's saved inventory list: 12-byte entries, `id`, `flags`,
/// `x`, `y`. `None` when the length is not a multiple of twelve.
pub fn parse_list(bytes: &[u8]) -> Option<Vec<ListChild>> {
    if !bytes.len().is_multiple_of(12) {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 12);
    for at in (0..bytes.len()).step_by(12) {
        out.push(ListChild {
            id: u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]),
            flags: u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]),
            x: i16::from_le_bytes([bytes[at + 8], bytes[at + 9]]),
            y: i16::from_le_bytes([bytes[at + 10], bytes[at + 11]]),
        });
    }
    Some(out)
}

/// Writes a character's saved inventory list.
pub fn write_list(entries: &[ListChild]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(entries.len() * 12);
    for child in entries {
        bytes.extend_from_slice(&child.id.to_le_bytes());
        bytes.extend_from_slice(&child.flags.to_le_bytes());
        bytes.extend_from_slice(&child.x.to_le_bytes());
        bytes.extend_from_slice(&child.y.to_le_bytes());
    }
    bytes
}

/// What a drop did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropResult {
    /// The entry's index after the drop.
    pub index: usize,
    /// The item whose bulk entered the panel, when one did.
    pub added: Option<u32>,
    /// The game function and argument the AR crystal drop fires.
    pub fired: Option<(u32, u32)>,
}

/// A free-form panel's state, kept as the original keeps it.
#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    pub entries: Vec<Entry>,
    pub used: i32,
    pub capacity: i32,
    pub max_item: i32,
    /// The container record this panel shows (0 for a character's pack).
    pub container_id: u32,
    /// The panel a move to or from does not charge capacity.
    pub linked: Option<u32>,
    /// The character that owns the panel, if any.
    pub owner: Option<u32>,
    pub reject_mask: u32,
    pub required_mask: u32,
    pub required_value: u32,
    pub green_mask: u32,
    pub yellow_mask: u32,
    /// The vertical position scale.
    pub vscale: f32,
    pub only_own: bool,
    pub loaded: bool,
    /// The client area the entries stay inside.
    pub client: (i32, i32),
    pub icon_size: i32,
}

impl Default for Container {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            used: 0,
            capacity: 0,
            max_item: 0,
            container_id: 0,
            linked: None,
            owner: None,
            reject_mask: 0,
            required_mask: 0,
            required_value: 0,
            green_mask: 0,
            yellow_mask: 0,
            vscale: 1.0,
            only_own: false,
            loaded: false,
            client: (228, 180),
            icon_size: 160,
        }
    }
}

/// A drop from a drag payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropDrag {
    /// The panel the drag came from (0 for a new item from elsewhere).
    pub src: u32,
    /// The source is this panel's linked one.
    pub linked: bool,
    pub id: u32,
    pub icon: usize,
    pub flags: u32,
    pub bulk: i32,
}

impl Container {
    /// The accept test of this panel. `own_icon` reports whether an entry
    /// carries the dragged icon.
    pub fn accept(&self, drag: &Drag, own_icon: impl FnMut(u32) -> bool) -> bool {
        accepts(
            &Accept {
                capacity: self.capacity,
                used: self.used,
                max_item_size: self.max_item,
                reject_mask: self.reject_mask,
                required_mask: self.required_mask,
                required_value: self.required_value,
                only_own: self.only_own,
                container_id: self.container_id,
                linked: self.linked,
                this: 0,
            },
            drag,
            own_icon,
        )
    }

    /// Adds an item at the panel's centre. An item with no icon is refused
    /// (the original's "user item" error path) and nothing changes.
    pub fn add_at_centre(&mut self, id: u32, icon: usize, flags: u32, bulk: i32) -> bool {
        if icon == 0 {
            return false;
        }
        let half = self.icon_size >> 1;
        let cell = Cell::new((self.client.0 >> 1) - half, (self.client.1 >> 1) - half);
        self.entries.push(Entry {
            cell,
            id,
            icon,
            flags,
            drag: 0,
        });
        self.used += bulk;
        true
    }

    /// Removes entry `at` and its bulk, shifting the tail left.
    pub fn remove_at(&mut self, at: usize, bulk: i32) -> Option<Entry> {
        let entry = remove_entry(&mut self.entries, at)?;
        self.used -= bulk;
        Some(entry)
    }

    /// A character-list load: every resolved child takes its saved place,
    /// its bulk counts, and the whole panel is then clamped by the icons'
    /// half extents (`halves` maps an icon slot to them). Unresolved items
    /// are skipped.
    pub fn load_list<F, H>(
        &mut self,
        children: &[ListChild],
        mut resolve: F,
        mut halves: H,
    ) -> usize
    where
        F: FnMut(u32, u32) -> Option<(usize, i32)>,
        H: FnMut(usize) -> Option<(i32, i32)>,
    {
        self.entries.clear();
        self.used = 0;
        let unit = 4;
        for child in children {
            let Some((icon, bulk)) = resolve(child.id, child.flags) else {
                continue;
            };
            self.entries.push(Entry {
                cell: from_list_pos((child.x, child.y), unit, self.vscale),
                id: child.id,
                icon,
                flags: child.flags,
                drag: 0,
            });
            self.used += bulk;
        }
        self.clamp_all(&mut halves);
        self.loaded = true;
        self.entries.len()
    }

    /// The inverse of [`Container::load_list`], in sixteenths of a unit.
    pub fn save_list(&self) -> Vec<ListChild> {
        let unit = 4;
        self.entries
            .iter()
            .map(|entry| {
                let (x, y) = to_list_pos(entry.cell.x, unit, self.vscale);
                ListChild {
                    id: entry.id,
                    flags: entry.flags,
                    x,
                    y,
                }
            })
            .collect()
    }

    /// A container-record load: the size sets the client, the capacity and
    /// item limit come from the record, and each resolved child takes its
    /// place in eightieths of a unit.
    pub fn load_record<F>(&mut self, record: &ContainerRecord, mut resolve: F) -> usize
    where
        F: FnMut(u32, u32) -> Option<(usize, i32)>,
    {
        self.container_id = self.container_id.max(1);
        self.capacity = i32::from(record.capacity);
        self.max_item = i32::from(record.max_item);
        self.entries.clear();
        self.used = 0;
        let unit = 4;
        for child in &record.children {
            let Some((icon, bulk)) = resolve(child.id, child.flags & 0x0FFF_FFFF) else {
                continue;
            };
            self.entries.push(Entry {
                cell: Cell::new(from_db_pos(child.x, unit), from_db_pos(child.y, unit)),
                id: child.id,
                icon,
                flags: child.flags,
                drag: 0,
            });
            self.used += bulk;
        }
        self.loaded = true;
        self.entries.len()
    }

    /// The inverse of [`Container::load_record`]. The size, capacity and
    /// item limit come from what the panel loaded.
    pub fn save_record(&self, size: (u16, u16)) -> ContainerRecord {
        let unit = 4;
        ContainerRecord {
            size,
            capacity: self.capacity as u16,
            max_item: self.max_item as u16,
            children: self
                .entries
                .iter()
                .map(|entry| ContainerChild {
                    id: entry.id,
                    flags: entry.flags,
                    x: to_db_pos(entry.cell.x, unit),
                    y: to_db_pos(entry.cell.y, unit),
                })
                .collect(),
        }
    }

    /// Keeps every entry inside the panel, each by its icon's half extents.
    pub fn clamp_all<H>(&mut self, mut halves: H)
    where
        H: FnMut(usize) -> Option<(i32, i32)>,
    {
        for entry in &mut self.entries {
            if entry.icon == 0 {
                continue;
            }
            if let Some(half) = halves(entry.icon) {
                clamp_entry(
                    &mut entry.cell,
                    half,
                    self.icon_size,
                    self.client.0,
                    self.client.1,
                );
            }
        }
    }

    /// A drop of a dragged entry or a new item. `src` is the panel the drag
    /// came from, `linked` says it is this panel's linked one. A drop from
    /// this panel or its linked one only moves the dragged entry; any other
    /// adds it unless this panel is already showing it as held. The item
    /// id 499 (the AR crystal) fires the game's function 2 with the
    /// container's id, or function 3 for a character-owned panel.
    pub fn drop_entry(&mut self, drag: DropDrag, grab: Cell, cursor: Cell) -> Option<DropResult> {
        let DropDrag {
            src,
            linked,
            id,
            icon,
            flags,
            bulk,
        } = drag;
        let mut index = None;
        if linked
            && let Some(at) = self
                .entries
                .iter()
                .position(|e| e.id == id && e.flags & HELD != 0)
        {
            self.entries[at].flags &= !HELD;
            self.entries[at].cell = Cell::new(cursor.x - grab.x, cursor.y - grab.y);
            index = Some(at);
        }
        let index = match index {
            Some(at) => at,
            None => {
                if src == 0 {
                    self.entries.push(Entry {
                        cell: Cell::new(cursor.x - grab.x, cursor.y - grab.y),
                        id,
                        icon,
                        flags,
                        drag: 0,
                    });
                    self.used += bulk;
                    self.entries.len() - 1
                } else {
                    let at = self
                        .entries
                        .iter()
                        .position(|e| e.drag & 1 != 0 && e.id == id)
                        .or_else(|| self.entries.iter().position(|e| e.id == id))?;
                    self.entries[at].drag = 0;
                    self.entries[at].cell = Cell::new(cursor.x - grab.x, cursor.y - grab.y);
                    at
                }
            }
        };
        let index = raise(&mut self.entries, index)?;
        if let Some(entry) = self.entries.get_mut(index) {
            clamp_entry(
                &mut entry.cell,
                (self.icon_size >> 1, self.icon_size >> 1),
                self.icon_size,
                self.client.0,
                self.client.1,
            );
        }
        let fired = (id == 499).then(|| {
            if self.container_id == 0 {
                if self.owner.is_some() { (3, 0) } else { (0, 0) }
            } else {
                (2, self.container_id)
            }
        });
        Some(DropResult {
            index,
            added: (src == 0).then_some(id),
            fired: fired.filter(|f| f.0 != 0),
        })
    }

    /// The drag ended over another panel. `source` is that panel's id,
    /// `linked` says it is this panel's linked one. A cancelled drag only
    /// clears the mark; a refused drag-out keeps the entry and takes the
    /// payload's flags; otherwise the entry leaves, charging this panel's
    /// bulk unless it moved to the linked panel.
    pub fn finish_drag(
        &mut self,
        src: u32,
        cancelled: bool,
        allowed_out: bool,
        payload_flags: u32,
        bulk: i32,
    ) -> Option<DragEnd> {
        let at = self.entries.iter().position(|e| e.drag & 1 != 0)?;
        if cancelled {
            self.entries[at].drag = 0;
            return Some(DragEnd::Cancel);
        }
        if !allowed_out {
            self.entries[at].flags = payload_flags;
            return Some(DragEnd::SnapBack);
        }
        let same = self.linked.is_some_and(|linked_id| linked_id == src);
        if same {
            self.entries.remove(at);
            return Some(DragEnd::Transfer);
        }
        self.remove_at(at, bulk);
        Some(DragEnd::Remove)
    }

    /// A UI-scale change rescales every entry's place from `old_unit` to
    /// the current unit (`round(unit · place / old)`).
    pub fn rescale(&mut self, old_unit: i32, unit: i32) {
        if old_unit <= 0 {
            return;
        }
        for entry in &mut self.entries {
            entry.cell.x = (f64::from(unit) * (f64::from(entry.cell.x) / f64::from(old_unit)))
                .round_ties_even() as i32;
            entry.cell.y = (f64::from(unit) * (f64::from(entry.cell.y) / f64::from(old_unit)))
                .round_ties_even() as i32;
        }
    }

    /// Draws each entry that is not dragged, held or icon-less, from
    /// `origin` (where its icon goes) with its tint.
    pub fn paint<F, G>(&self, mut origin: F, mut draw: G)
    where
        F: FnMut(usize) -> Option<(i32, i32)>,
        G: FnMut(usize, (i32, i32), [u8; 4]),
    {
        for entry in &self.entries {
            let Some(tint) = tint(entry, self.green_mask, self.yellow_mask) else {
                continue;
            };
            if let Some(at) = origin(entry.icon) {
                draw(entry.icon, at, tint);
            }
        }
    }

    /// The topmost entry whose rectangle and icon alpha hold the point.
    /// `alpha` reports whether an icon's pixel is opaque.
    pub fn hit_alpha<F>(&self, x: i32, y: i32, mut alpha: F) -> Option<usize>
    where
        F: FnMut(usize, i32, i32) -> bool,
    {
        for (i, entry) in self.entries.iter().enumerate().rev() {
            if entry.flags & HELD != 0 || entry.icon == 0 {
                continue;
            }
            let h = self.icon_size;
            if x >= entry.cell.x
                && x < entry.cell.x + h
                && y >= entry.cell.y
                && y < entry.cell.y + h
                && alpha(entry.icon, x - entry.cell.x, y - entry.cell.y)
            {
                return Some(i);
            }
        }
        None
    }
}

#[cfg(test)]
mod entry_tests {
    use super::*;

    fn entry(id: u32, flags: u32) -> Entry {
        Entry {
            id,
            icon: id as usize,
            flags,
            ..Default::default()
        }
    }

    #[test]
    fn the_tints() {
        let e = entry(1, 0x400);
        assert_eq!(tint(&e, 0x400, 0x200), Some([0, 255, 0, 255]));
        assert_eq!(tint(&entry(1, 0x600), 0x400, 0x200), Some([0, 255, 0, 127]));
        assert_eq!(
            tint(&entry(1, 0x200), 0x400, 0x200),
            Some([255, 255, 0, 127])
        );
        assert_eq!(tint(&entry(1, 0), 0x400, 0x200), Some([0, 0, 0, 255]));
        assert_eq!(tint(&entry(1, HELD), 0, 0), None);
        assert_eq!(tint(&entry(0, 0), 0, 0), None);
        let mut d = entry(1, 0);
        d.drag = 1;
        assert_eq!(tint(&d, 0, 0), None);
    }

    #[test]
    fn entries_stay_inside_and_held_items_come_back() {
        let mut c = Cell::new(300, -100);
        clamp_entry(&mut c, (50, 40), 160, 228, 180);
        // x: 228 − 50 − 80 = 98; y: 40 − 80 = −40.
        assert_eq!(c, Cell::new(98, -40));
        assert_eq!(centre_cell(228, 180, 160), Cell::new(34, 10));
        let mut es = vec![entry(5, HELD), entry(5, 0), entry(6, HELD)];
        assert!(take_back_held(&mut es, 5));
        assert_eq!(es[0].flags, 0);
        assert!(!take_back_held(&mut es, 5));
        es[1].drag = 1;
        assert_eq!(hold_dragged(&mut es), Some(1));
        assert_eq!((es[1].drag, es[1].flags), (0, HELD));
        assert_eq!(hold_dragged(&mut es), Some(2));
        assert!(remove_held_icon(&mut es, 6));
        assert!(!remove_held_icon(&mut es, 6));
        es[0].drag = 3;
        clear_drags(&mut es);
        assert_eq!(es[0].drag, 2);
    }

    #[test]
    fn replacing_an_entry_moves_its_bulk() {
        let mut es = vec![Entry {
            cell: Cell::new(200, 0),
            ..entry(5, 0)
        }];
        let mut used = 30;
        let bulk = |id: u32| id as i32 * 2;
        let r = Replacement {
            id: 7,
            icon: 9,
            flags: 0x10,
            bulk: &bulk,
            half: (40, 40),
            icon_size: 160,
            client: (228, 180),
        };
        assert_eq!(replace_entry(&mut es, &mut used, 0, &r), Some(5));
        assert_eq!(used, 30 - 10 + 14);
        assert_eq!((es[0].id, es[0].icon, es[0].flags), (7, 9, 0x10));
        assert_eq!(es[0].cell, Cell::new(108, 0));
        assert_eq!(replace_entry(&mut es, &mut used, 3, &r), None);
    }

    #[test]
    fn a_copy_weighs_as_its_template() {
        let template = |id: u32| (id == 0x8000_0003).then_some(12);
        let model = |id: u32| (id == 12).then_some(9u8);
        assert_eq!(bulk_of(0x8000_0003, &template, &model), 9);
        assert_eq!(bulk_of(0x8000_0004, &template, &model), 0);
        assert_eq!(bulk_of(12, &template, &model), 9);
        assert_eq!(bulk_of(13, &template, &model), 0);
    }

    #[test]
    fn the_worn_panel_spreads_its_items() {
        let items = [(1, 0), (2, 0), (3, 0x10)];
        let mut icon = |id: u32| Some((id as usize, (40, 40)));
        let mut state = 0x1234;
        let a = worn_layout(&items, &mut icon, (228, 228), 160, &mut state);
        let mut state2 = 0x1234;
        let b = worn_layout(&items, &mut icon, (228, 228), 160, &mut state2);
        assert_eq!(a, b);
        assert_eq!(state, state2);
        assert_eq!(a.len(), 3);
        assert_eq!((a[2].flags, a[2].icon), (0x10, 3));
        // An item the cache cannot draw is left out and takes no draws: the
        // second item then lands where the first did.
        let mut state3 = 0x1234;
        let mut no_first = |id: u32| (id != 1).then_some((id as usize, (40, 40)));
        let c = worn_layout(&items[..2], &mut no_first, (228, 228), 160, &mut state3);
        let mut state4 = 0x1234;
        let d = worn_layout(&items[1..2], &mut icon, (228, 228), 160, &mut state4);
        assert_eq!(c, d);
        assert_eq!(state3, state4);
        for e in &a {
            // Centres inside [40, 188].
            let (cx, cy) = (e.cell.x + 80, e.cell.y + 80);
            assert!(
                (40..=188).contains(&cx) && (40..=188).contains(&cy),
                "{e:?}"
            );
        }
        assert!(worn_layout(&[], &mut icon, (228, 228), 160, &mut state).is_empty());
        assert_eq!(parchment_size((768, 512), 4), (512, 341));
    }
}

/// The pronoun kinds an item description's template can name.
pub const PRONOUN_SUBJECT: u32 = 5;
pub const PRONOUN_OBJECT: u32 = 6;
pub const PRONOUN_POSSESSIVE: u32 = 7;

/// The word for a pronoun kind, by the owner's sex bit (bit 0 of its body
/// word at `+0x54`).
pub fn pronoun(kind: u32, female: bool) -> Option<&'static str> {
    Some(match (kind, female) {
        (PRONOUN_SUBJECT, false) => "he",
        (PRONOUN_SUBJECT, true) => "she",
        (PRONOUN_OBJECT, false) => "him",
        (PRONOUN_OBJECT, true) => "her",
        (PRONOUN_POSSESSIVE, false) => "his",
        (PRONOUN_POSSESSIVE, true) => "her",
        _ => return None,
    })
}

/// The card builder's text box: the title's height above it (`0xb4`), the
/// box's own vertical inset (`0xa0`), and the two-unit text margin.
pub const CARD_TEXT_TITLE: i32 = 0xb4;
pub const CARD_TEXT_INSET: i32 = 0xa0;
pub const CARD_TEXT_MARGIN: i32 = 4;

/// The height of the card's text box from the page's client size (its
/// inner size less twice the border and twice the text margin).
pub fn card_text_width(client_width: i32, border: i32) -> i32 {
    client_width - border * 2 - CARD_TEXT_MARGIN * 2
}

/// The item card the mouse opens: a released "use" on types 0–2 shows the
/// card after running the game's function for type 2, and a right click
/// toggles the card of the same item. A container is opened instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardAction {
    /// No card.
    None,
    /// Show the card; `run_use` when a type-2 usable item runs first.
    Show { run_use: bool },
    /// Close the card already open for this item.
    Toggle,
    /// Open the item as a container.
    OpenContainer,
}

/// The card action for a use (a double click) or a right click.
pub fn card_action(right_click: bool, usable_type: Option<u8>, has_card: bool) -> CardAction {
    if right_click {
        return if has_card {
            CardAction::Toggle
        } else {
            CardAction::Show { run_use: false }
        };
    }
    match usable_type {
        Some(2) => CardAction::Show { run_use: true },
        Some(0 | 1) => CardAction::Show { run_use: false },
        Some(_) => CardAction::None,
        None => CardAction::None,
    }
}

/// The card's delayed opening: the timer asked for two frames and, when it
/// fires, the card is built for the item and flags remembered at the
/// toggle. A zero item cancels nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CardTimer {
    pub pending: u32,
    pub item: u32,
    pub flags: u32,
    /// The card's top, as the popped-up form gets it.
    pub top: i32,
}

impl CardTimer {
    /// Asks for a card after two frames, cancelling any pending one.
    pub fn request(item: u32, flags: u32, top: i32) -> Self {
        Self {
            pending: 2,
            item,
            flags,
            top,
        }
    }

    /// One frame passes; returns the card to build this frame.
    pub fn tick(&mut self) -> Option<(u32, u32)> {
        if self.pending == 0 {
            return None;
        }
        self.pending -= 1;
        (self.pending == 0).then_some((self.item, self.flags))
    }
}

#[cfg(test)]
mod container_tests {
    use super::*;

    fn child(id: u32, flags: u32, x: i16, y: i16) -> ListChild {
        ListChild { id, flags, x, y }
    }

    #[test]
    fn pronouns_follow_the_owner_sex_bit() {
        assert_eq!(pronoun(PRONOUN_SUBJECT, false), Some("he"));
        assert_eq!(pronoun(PRONOUN_SUBJECT, true), Some("she"));
        assert_eq!(pronoun(PRONOUN_OBJECT, true), Some("her"));
        assert_eq!(pronoun(PRONOUN_POSSESSIVE, false), Some("his"));
        assert_eq!(pronoun(8, true), None);
        assert_eq!(card_text_width(228, 8), 204);
        assert_eq!(card_action(true, None, true), CardAction::Toggle);
        assert_eq!(
            card_action(true, None, false),
            CardAction::Show { run_use: false }
        );
        assert_eq!(
            card_action(false, Some(2), false),
            CardAction::Show { run_use: true }
        );
        assert_eq!(
            card_action(false, Some(1), false),
            CardAction::Show { run_use: false }
        );
        assert_eq!(card_action(false, Some(3), false), CardAction::None);
        let mut timer = CardTimer::request(7, 0x10, 33);
        assert_eq!(timer.tick(), None);
        assert_eq!(timer.tick(), Some((7, 0x10)));
        assert_eq!(timer.tick(), None);
    }

    #[test]
    fn a_container_record_round_trips_and_refuses_short_input() {
        let record = ContainerRecord {
            size: (12, 8),
            capacity: 400,
            max_item: 50,
            children: vec![ContainerChild {
                id: 7,
                flags: 0x100,
                x: 40,
                y: -80,
            }],
        };
        let bytes = record.write();
        assert_eq!(ContainerRecord::parse(&bytes), Some(record.clone()));
        let mut short = bytes.clone();
        short.truncate(0x1f);
        assert_eq!(ContainerRecord::parse(&short), None);
        let mut over = bytes.clone();
        over[0x1c] = 9;
        assert_eq!(ContainerRecord::parse(&over), None);
        assert_eq!(ContainerRecord::parse(&[0u8; 3]), None);
    }

    #[test]
    fn a_character_list_round_trips_and_rejects_a_ragged_length() {
        let entries = [child(3, 0x100, 560, -32), child(4, 0, 1, 2)];
        let bytes = write_list(&entries);
        assert_eq!(bytes.len(), 24);
        assert_eq!(parse_list(&bytes), Some(entries.to_vec()));
        assert_eq!(parse_list(&bytes[..11]), None);
        assert_eq!(parse_list(&[]), Some(Vec::new()));
    }

    #[test]
    fn a_list_load_counts_bulk_and_clamps_the_icons() {
        let mut panel = Container {
            client: (228, 180),
            icon_size: 160,
            vscale: 1.0,
            ..Default::default()
        };
        let children = [child(5, 0x10, 560, 560), child(6, 0, -1000, 0)];
        let mut halves = |icon: usize| (icon == 5).then_some((40, 40));
        let loaded = panel.load_list(
            &children,
            |id, flags| (id != 6).then_some((id as usize, flags as i32)),
            &mut halves,
        );
        assert_eq!(loaded, 1);
        assert!(panel.loaded);
        assert_eq!(panel.used, 0x10);
        // 560 / 16 × 4 = 140; the centre stays 40 inside each edge.
        assert_eq!(panel.entries[0].cell, Cell::new(108, 60));
        assert_eq!(panel.save_list(), vec![child(5, 0x10, 432, 432)]);
    }

    #[test]
    fn a_record_load_uses_eightieths_of_a_unit() {
        let mut panel = Container {
            container_id: 9,
            ..Default::default()
        };
        let record = ContainerRecord {
            size: (10, 6),
            capacity: 120,
            max_item: 30,
            children: vec![ContainerChild {
                id: 12,
                flags: 0x2,
                x: 2800,
                y: -80,
            }],
        };
        let loaded = panel.load_record(&record, |id, flags| {
            (id == 12 && flags == 2).then_some((3, 7))
        });
        assert_eq!(loaded, 1);
        assert_eq!((panel.capacity, panel.max_item), (120, 30));
        assert_eq!(panel.entries[0].cell, Cell::new(140, -4));
        assert_eq!(
            panel.save_record((10, 6)).children,
            vec![ContainerChild {
                id: 12,
                flags: 2,
                x: 2800,
                y: -80
            }]
        );
    }

    #[test]
    fn adding_removing_and_rescaling_keep_the_bulk() {
        let mut panel = Container {
            client: (228, 180),
            icon_size: 160,
            ..Default::default()
        };
        assert!(!panel.add_at_centre(1, 0, 0, 4));
        assert!(panel.add_at_centre(1, 2, 0x10, 4));
        assert_eq!(panel.entries[0].cell, Cell::new(34, 10));
        assert_eq!(panel.used, 4);
        panel.entries[0].cell = Cell::new(100, 50);
        panel.rescale(4, 5);
        assert_eq!(panel.entries[0].cell, Cell::new(125, 62));
        assert_eq!(panel.remove_at(0, 4).unwrap().id, 1);
        assert_eq!(panel.used, 0);
        assert_eq!(panel.remove_at(0, 4), None);
    }

    #[test]
    fn a_drop_moves_a_linked_item_and_raises_an_external_one() {
        let mut panel = Container {
            client: (228, 180),
            icon_size: 160,
            owner: Some(9),
            ..Default::default()
        };
        panel.entries.push(Entry {
            cell: Cell::new(10, 10),
            id: 5,
            icon: 2,
            flags: HELD,
            drag: 0,
        });
        let r = panel
            .drop_entry(
                DropDrag {
                    src: 4,
                    linked: true,
                    id: 5,
                    icon: 2,
                    flags: HELD,
                    bulk: 4,
                },
                Cell::new(0, 0),
                Cell::new(100, 20),
            )
            .unwrap();
        assert_eq!((r.index, r.added), (0, None));
        assert_eq!(panel.entries[0].flags, 0);
        assert_eq!(panel.entries[0].cell, Cell::new(68, 20));
        assert_eq!(panel.used, 0);

        let r = panel
            .drop_entry(
                DropDrag {
                    src: 0,
                    linked: false,
                    id: 499,
                    icon: 3,
                    flags: 0,
                    bulk: 5,
                },
                Cell::new(80, 80),
                Cell::new(20, 20),
            )
            .unwrap();
        assert_eq!(r.index, 1);
        assert_eq!(r.added, Some(499));
        assert_eq!(r.fired, Some((3, 0)));
        assert_eq!(panel.used, 5);
        assert_eq!(panel.entries[1].cell, Cell::new(0, 0));
        panel.container_id = 77;
        let r = panel
            .drop_entry(
                DropDrag {
                    src: 0,
                    linked: false,
                    id: 499,
                    icon: 3,
                    flags: 0,
                    bulk: 5,
                },
                Cell::new(80, 80),
                Cell::new(20, 20),
            )
            .unwrap();
        assert_eq!(r.fired, Some((2, 77)));
    }

    #[test]
    fn a_drag_end_clears_snaps_back_transfers_or_removes() {
        let mut panel = Container {
            linked: Some(4),
            ..Default::default()
        };
        panel.entries.push(Entry {
            cell: Cell::new(1, 2),
            id: 5,
            icon: 1,
            flags: 0,
            drag: 1,
        });
        assert_eq!(
            panel.finish_drag(9, true, true, 0, 4),
            Some(DragEnd::Cancel)
        );
        assert_eq!(panel.entries[0].drag, 0);
        panel.entries[0].drag = 1;
        assert_eq!(
            panel.finish_drag(9, false, false, 0x10, 4),
            Some(DragEnd::SnapBack)
        );
        assert_eq!(panel.entries[0].flags, 0x10);
        assert_eq!(
            panel.finish_drag(4, false, true, 0, 4),
            Some(DragEnd::Transfer)
        );
        assert!(panel.entries.is_empty());
        panel.entries.push(Entry {
            cell: Cell::new(1, 2),
            id: 6,
            icon: 1,
            flags: 0,
            drag: 1,
        });
        panel.used = 4;
        assert_eq!(
            panel.finish_drag(9, false, true, 0, 4),
            Some(DragEnd::Remove)
        );
        assert_eq!(panel.used, 0);
        assert_eq!(panel.finish_drag(9, false, true, 0, 4), None);
    }

    #[test]
    fn a_paint_skips_held_dragged_and_iconless_entries() {
        let panel = Container {
            green_mask: 0x400,
            yellow_mask: 0x200,
            entries: vec![
                Entry {
                    cell: Cell::new(0, 0),
                    id: 1,
                    icon: 1,
                    flags: 0x400,
                    drag: 0,
                },
                Entry {
                    cell: Cell::new(1, 1),
                    id: 2,
                    icon: 2,
                    flags: 0x200,
                    drag: 1,
                },
                Entry {
                    cell: Cell::new(2, 2),
                    id: 3,
                    icon: 0,
                    flags: 0,
                    drag: 0,
                },
            ],
            ..Default::default()
        };
        let mut drawn = Vec::new();
        panel.paint(
            |icon| (icon != 0).then_some((icon as i32, 0)),
            |icon, at, tint| drawn.push((icon, at, tint)),
        );
        assert_eq!(drawn, vec![(1, (1, 0), [0, 255, 0, 255])]);
    }

    #[test]
    fn the_alpha_hit_test_takes_the_topmost_opaque_icon() {
        let panel = Container {
            icon_size: 10,
            entries: vec![
                Entry {
                    cell: Cell::new(0, 0),
                    id: 1,
                    icon: 1,
                    flags: 0,
                    drag: 0,
                },
                Entry {
                    cell: Cell::new(5, 5),
                    id: 2,
                    icon: 2,
                    flags: 0,
                    drag: 0,
                },
            ],
            ..Default::default()
        };
        assert_eq!(panel.hit_alpha(6, 6, |_, _, _| true), Some(1));
        assert_eq!(panel.hit_alpha(6, 6, |icon, _, _| icon == 1), Some(0));
        assert_eq!(panel.hit_alpha(6, 6, |_, _, _| false), None);
        assert_eq!(panel.hit_alpha(20, 20, |_, _, _| true), None);
    }
}
