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
