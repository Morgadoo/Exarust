//! The portal destination screen: the reachable levels laid on a ring
//! around a hub, its reveal and close fades, the pointer's hover, the
//! journey's bubble and the travel that follows it.
//!
//! Evidence: the destination screen's routines in the retail image. The
//! ring's angle runs from `2.5` nodes at the top, clockwise, over one turn
//! divided by `count − 1`; a node sits at `ring radius × the unit` on each
//! axis, and the whole ring scales with the screen's reveal value. The
//! screen opens over 500 ms (`1 − remaining × 0.002` eased by `x(2−x)`),
//! a close adds a 256 ms tail, and a hit lands within 14 units of a node.
//! Times are the original's milliseconds, positions its pixels.

use bevy::prelude::Resource;

/// The symbol atlas the screen draws.
pub const SCREEN_ATLAS: &str = "prtlsmbla.rfi";
/// The reveal takes this long (ms).
pub const REVEAL_MS: u32 = 500;
/// The close tail takes this long (ms).
pub const CLOSE_MS: u32 = 256;
/// The reveal's per-millisecond share of the double.
pub const REVEAL_PER_MS: f64 = 0.002;
/// The close tail's divisor.
pub const CLOSE_DIVISOR: f32 = 256.0;
/// The ring's radius in units.
pub const RING_RADIUS_UNITS: f32 = 40.0;
/// A node is hit within this many units.
pub const HIT_RADIUS_UNITS: i64 = 14;
/// A symbol is this many units square.
pub const SYMBOL_UNITS: i64 = 32;
/// The alpha a node's symbol is drawn with.
pub const SYMBOL_ALPHA: f32 = 0.6;
/// A node's wobble keeps within this magnitude.
pub const WOBBLE_LIMIT: f32 = 1.0;
/// A hovered node's colour fades from its full strength over this (ms).
pub const HOVER_FADE_MS: u32 = 200;
/// The wobble's per-millisecond flash scale (double).
pub const FLASH_PER_MS: f64 = 0.02;
/// A node's arrival fade is this share of a millisecond (f64 0.005).
pub const ARRIVE_FADE_PER_MS: f64 = 0.005;
/// The screen's reveal state word: bit 0 is the reveal, bit 1 the close
/// tail, and word 4 the closed screen.
pub mod state {
    pub const REVEALING: u16 = 1;
    pub const CLOSING: u16 = 2;
    pub const CLOSED: u16 = 4;
}

/// One destination row's animation state: its flash deadline, the symbol
/// it shows and the two wobble values the screen jitters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DestinationSlot {
    /// The time this slot's hover flash ends (0x100273b10's stamp).
    pub flash_until: u32,
    /// The symbol the row shows: an index into the level ids.
    pub symbol: usize,
    /// Wobble angle and speed on each axis; the screen nudges both.
    pub wobble: [f32; 2],
    pub drift: [f32; 2],
}

impl DestinationSlot {
    /// A slot with a symbol, no flash and no wobble.
    pub fn new(symbol: usize) -> Self {
        Self {
            flash_until: 0,
            symbol,
            wobble: [0.0; 2],
            drift: [0.0; 2],
        }
    }
}

/// The rows the screen shows, filled in the original's order: entry 0 goes
/// to row 1, later entries to rows 2..., and an empty entry or an entry met
/// once the other rows are taken goes back to row 0 (the last such wins).
/// Every row nothing reaches keeps entry 0.
pub fn fill_rows(destinations: &[u32], rows: usize) -> Vec<usize> {
    if rows == 0 {
        return Vec::new();
    }
    let mut entries = vec![0usize; rows];
    let mut next = 1usize;
    for (i, destination) in destinations.iter().copied().enumerate().take(rows) {
        if destination == 0 || next >= rows {
            entries[0] = i;
        } else {
            entries[next] = i;
            next += 1;
        }
    }
    entries
}

/// The state the screen keeps while it is open.
#[derive(Debug, Clone, PartialEq, Resource)]
pub struct DestinationScreen {
    /// The symbols the rows show, one per row.
    pub slots: Vec<DestinationSlot>,
    /// Bit 0 while the reveal runs, bit 1 while the close trail runs, word
    /// 4 once closed.
    pub state: u16,
    /// The reveal's deadline.
    pub reveal_until: u32,
    /// The close trail's deadline.
    pub close_until: u32,
    /// The hovered (or clicked) row, 1-based; 0 is none.
    pub selected: i32,
    /// The reveal's scale, 0..1.
    pub turn: f32,
    /// The screen's alpha, 0..1.
    pub alpha: f32,
    /// The clock the wobble's last flash was measured from.
    pub flash_clock: u32,
    /// The level id each symbol names, in row order.
    pub level_ids: Vec<u32>,
    /// Whether the screen draws its ring of nodes (the compact flag).
    pub ring: bool,
}

impl DestinationScreen {
    /// The screen of a destination record: `rows` rows over `level_ids`,
    /// opening at `now`.
    pub fn open(level_ids: Vec<u32>, rows: usize, ring: bool, now: u32) -> Self {
        let entries = fill_rows(&level_ids, rows);
        Self {
            slots: entries.into_iter().map(DestinationSlot::new).collect(),
            state: state::REVEALING,
            reveal_until: now.wrapping_add(REVEAL_MS),
            close_until: 0,
            selected: 0,
            turn: 0.0,
            alpha: 1.0,
            flash_clock: now,
            level_ids,
            ring,
        }
    }

    /// The number of rows.
    pub fn rows(&self) -> usize {
        self.slots.len()
    }

    /// Starts the close: the action layer is popped, bit 1 is set and the
    /// tail runs for 256 ms.
    pub fn close(&mut self, now: u32) {
        self.state |= state::CLOSING;
        self.close_until = now.wrapping_add(CLOSE_MS);
    }

    /// The leave rule: closing waits for its tail and the reveal; a closed
    /// screen leaves immediately.
    pub fn may_leave(&self, now: u32) -> bool {
        if self.state & state::REVEALING != 0 && self.reveal_until.wrapping_sub(400) > now {
            return false;
        }
        self.state & state::CLOSING == 0
    }

    /// Hovers a row (1-based). The row's flash is restarted, and the ring
    /// moves toward it.
    pub fn hover(&mut self, row: i32, now: u32) -> i32 {
        if row < 1 && self.selected < 1 {
            return 0;
        }
        let result = 3;
        if row > 0 {
            bump_flash(&mut self.slots[row as usize - 1], now);
        }
        if self.selected > 0 {
            bump_flash(&mut self.slots[self.selected as usize - 1], now);
        }
        self.selected = row;
        result
    }

    /// One frame of the reveal and close: returns the close that started
    /// this frame, if any.
    pub fn frame(&mut self, now: u32) -> Frame {
        let mut frame = Frame {
            turn: 1.0,
            alpha: 1.0,
            closed: false,
        };
        if self.state == state::CLOSED {
            frame.turn = 0.0;
            frame.alpha = 0.0;
            return frame;
        }
        if self.state & state::REVEALING == 0 {
            frame.turn = 1.0;
        } else if self.reveal_until > now {
            let left = 1.0 - f64::from(self.reveal_until - now) * REVEAL_PER_MS;
            let left = left as f32;
            let turn = left * (2.0 - left);
            frame.turn = turn;
            let alpha = turn * 1.0;
            frame.alpha = if alpha > 1.0 { 1.0 } else { alpha };
        } else {
            self.state &= !state::REVEALING;
            frame.turn = 1.0;
        }
        if self.state & state::CLOSING != 0 {
            if self.close_until > now {
                frame.alpha = (self.close_until - now) as f32 * frame.alpha / CLOSE_DIVISOR;
            } else {
                self.state = state::CLOSED;
                frame.closed = true;
                return frame;
            }
        }
        self.turn = frame.turn;
        self.alpha = frame.alpha;
        frame
    }
}

/// One frame's reveal values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub turn: f32,
    pub alpha: f32,
    pub closed: bool,
}

/// Restarts a slot's flash: from a deadline still ahead the deadline is
/// reflected through now plus the hover's 200 ms, else it is now + 200.
pub fn bump_flash(slot: &mut DestinationSlot, now: u32) {
    if now < slot.flash_until {
        slot.flash_until = now
            .wrapping_mul(2)
            .wrapping_add(HOVER_FADE_MS)
            .wrapping_sub(slot.flash_until);
    } else {
        slot.flash_until = now.wrapping_add(HOVER_FADE_MS);
    }
}

/// A quarter-turn sine: the original's fast sine helper maps 0 to 0 and 1
/// to 1 over a quarter turn (the reveal's ease and the cards use it).
pub fn quarter_sine(x: f32) -> f32 {
    (f64::from(x) * std::f64::consts::FRAC_PI_2).sin() as f32
}

/// Places a node on the ring. Node 0 is the centre. `turn` scales the
/// radius and the offsets.
pub fn place_node(
    index: usize,
    count: usize,
    width: i32,
    height: i32,
    unit: i32,
    turn: f32,
) -> (f32, f32) {
    let centre_x = width / 2;
    let centre_y = height / 2;
    if index == 0 || count <= 1 {
        return (centre_x as f32, centre_y as f32);
    }
    let angle =
        (1.0 - turn) + ((2.5 - index as f32) * std::f32::consts::TAU) / (count as f32 - 1.0);
    let (sin, cos) = (f64::from(angle).sin(), f64::from(angle).cos());
    let x = (f64::from(unit) * cos * f64::from(RING_RADIUS_UNITS)).round_ties_even() as f32;
    let y = (f64::from(unit) * sin * f64::from(RING_RADIUS_UNITS)).round_ties_even() as f32;
    (x * turn + centre_x as f32, y * turn + centre_y as f32)
}

/// Tests a point against the ring's nodes 1.. (the centre is never hit):
/// rows are checked in order and the last within 14 units wins. The result
/// is the 1-based row, or -1 (the base's value).
pub fn hit_node(
    point: (i32, i32),
    count: usize,
    width: i32,
    height: i32,
    unit: i32,
    turn: f32,
) -> i32 {
    let mut hit = -1;
    for index in 1..count {
        let (x, y) = place_node(index, count, width, height, unit, turn);
        let dx = point.0 as f32 - x;
        let dy = point.1 as f32 - y;
        let limit = (unit as i64 * HIT_RADIUS_UNITS) as f32;
        if dx * dx + dy * dy <= limit * limit {
            hit = index as i32 + 1;
        }
    }
    hit
}

/// The four corners of a symbol's quad in screen pixels, and the atlas cell
/// (column, row) of its 2 × 2 sheet.
pub fn symbol_quad(centre: (f32, f32), unit: i32, turn: f32, symbol: usize) -> SymbolQuad {
    let size = (unit as f32 * SYMBOL_UNITS as f32) * turn;
    let left = centre.0 - size * 0.5;
    let top = centre.1 - size * 0.5;
    let cell = (
        (symbol % 2) as f32 * 0.5,
        1.0 - ((symbol / 2 + 1) as f32) * 0.5,
    );
    SymbolQuad {
        corners: [left, top, left + size, top + size],
        cell,
        tex_ofs: [0.0, 0.0],
    }
}

/// A symbol's quad: its box and its atlas cell origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SymbolQuad {
    /// Left, top, right and bottom.
    pub corners: [f32; 4],
    /// The atlas cell's origin (u, v).
    pub cell: (f32, f32),
    /// The atlas texture offset.
    pub tex_ofs: [f32; 2],
}

/// One step of the screen's paint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionDraw {
    /// The full-screen backdrop skin and its shader.
    Backdrop,
    /// A destination symbol: its row, box and colour mix.
    Symbol {
        row: usize,
        corners: [f32; 4],
        mix: f32,
    },
}

/// The screen's paint: the backdrop, then every row's symbol with its
/// arrival fade and hover mix. The card's size and colour are applied by
/// the caller from the interface's palette.
pub fn draw_order(
    screen: &DestinationScreen,
    width: i32,
    height: i32,
    unit: i32,
    now: u32,
) -> Vec<SelectionDraw> {
    let mut draw = vec![SelectionDraw::Backdrop];
    for (row, slot) in screen.slots.iter().enumerate() {
        let (x, y) = place_node(row, screen.rows(), width, height, unit, screen.turn);
        let fade = arrival_fade(slot, now);
        let mix = node_mix(fade, row, screen.selected);
        let quad = symbol_quad((x, y), unit, 1.0, slot.symbol);
        draw.push(SelectionDraw::Symbol {
            row,
            corners: quad.corners,
            mix,
        });
    }
    draw
}

/// Steps a slot's wobble by one frame's flash. `random` returns [0, 1) as
/// the original's generator does; it is called twice per axis, in order.
pub fn wobble_step(slot: &mut DestinationSlot, flash: f32, mut random: impl FnMut() -> f32) {
    for axis in 0..2 {
        let value = slot.wobble[axis];
        if value < 0.9 {
            if value > -0.9 {
                let nudge = 2.0 * (random() - 0.5) * flash;
                slot.drift[axis] += nudge;
            } else {
                slot.drift[axis] += random() * flash;
            }
        } else {
            slot.drift[axis] -= random() * flash;
        }
        let r = random();
        slot.wobble[axis] += flash * 2.0 * slot.drift[axis] * r;
        slot.wobble[axis] = slot.wobble[axis].clamp(-WOBBLE_LIMIT, WOBBLE_LIMIT);
    }
}

/// The flash strength of a frame: the time since the screen's last clock
/// stamp times 0.02. The caller stores the new stamp.
pub fn flash_amount(now: u32, last: u32) -> f32 {
    (f64::from(now.wrapping_sub(last)) * FLASH_PER_MS) as f32
}

/// A slot's arrival fade: 1 until its flash deadline, then falling by
/// 0.005 a millisecond from the remaining time.
pub fn arrival_fade(slot: &DestinationSlot, now: u32) -> f32 {
    if slot.flash_until < now {
        1.0
    } else {
        (1.0 - f64::from(slot.flash_until - now) * ARRIVE_FADE_PER_MS) as f32
    }
}

/// The colour mix a node draws with: `fade` toward the highlight for the
/// hovered row, away from it for the others. Returns the mix share.
pub fn node_mix(fade: f32, row: usize, selected: i32) -> f32 {
    if selected == row as i32 + 1 {
        fade
    } else {
        1.0 - fade
    }
}

/// Where the journey's bubble sits: the clicked form's position plus 98 on
/// y. The two stored screen coordinates are the form's x and y.
pub fn bubble_origin(screen: (f32, f32)) -> (f32, f32) {
    (screen.0, screen.1 + 98.0)
}

/// Which destination a click starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    /// Nothing (or a row outside the screen).
    None,
    /// The right button (or the click mask's bit 1) just closes.
    Dismiss,
    /// Start the journey to this level id.
    Travel(u32),
}

/// The click rule: a closed screen and a right click close; a left click on
/// a hovered row starts the journey to that row's level.
pub fn click(screen: &DestinationScreen, mask: u16, left_is_down: bool) -> Click {
    if screen.selected < -1 {
        return Click::None;
    }
    if mask & 2 != 0 {
        return Click::Dismiss;
    }
    if screen.state == 0 && left_is_down && screen.selected > 0 {
        let row = screen.selected as usize - 1;
        if let Some(&level) = screen
            .level_ids
            .get(screen.slots.get(row).map(|s| s.symbol).unwrap_or(0))
        {
            return Click::Travel(level);
        }
    }
    Click::None
}

/// The journey's timing and crystal effects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Journey {
    /// The bubble holds instead of shrinking after it grows.
    pub hold: bool,
    /// The delay the travel tick starts at (ms).
    pub start_ms: u32,
    /// The delay the travel ends at (ms).
    pub end_ms: u32,
    /// The destination row whose level id travel removes.
    pub level_id: u32,
    /// The crystal's stored colour offset.
    pub color_offset: u8,
}

/// The journey a click starts: the large-crystal adapter (0x58F) holds the
/// bubble, takes 1.5 s to start and 7 s to end; any other crystal lets it
/// fade, starting after 1 s and ending after 1.4 s at the crystal's
/// destination colour.
pub fn journey(adapter: bool, level_id: u32, color_offset: u8) -> Journey {
    if adapter {
        Journey {
            hold: true,
            start_ms: 1_500,
            end_ms: 7_000,
            level_id,
            color_offset,
        }
    } else {
        Journey {
            hold: false,
            start_ms: 1_000,
            end_ms: 1_400,
            level_id,
            color_offset,
        }
    }
}

/// The crystal's charge a travel tick writes: the stored byte plus the
/// elapsed time over 12, clamped to 255. The result replaces the middle
/// byte of the crystal record's flags.
pub fn crystal_charge(stored: u8, elapsed: u32) -> u8 {
    let added = elapsed / 12;
    let charge = u32::from(stored).wrapping_add(added);
    charge.min(255) as u8
}

/// What the travel tick did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TravelStep {
    /// The crystal's charge changed to this byte, if it did.
    pub charge: Option<u8>,
    /// The whole crystal record flags word to write.
    pub flags: Option<u32>,
    /// The screen asks the session to end (mode word 4).
    pub end_session: bool,
    /// The player's fade-out reached its threshold and the game saves.
    pub save: bool,
    /// The destination marker the level id removes.
    pub remove_level: Option<u32>,
    /// The journey is over and its holder is freed.
    pub done: bool,
}

/// One tick of the travel once the bubble has been stepped. `mode` is the
/// session's mode word: 4 ends the session, 6 saves and returns to the
/// level. `player_fade` is the player's fade value (read only on mode 6).
pub fn travel_step(
    crystal: Option<u32>,
    stored_color: u8,
    started: u32,
    now: u32,
    mode: u32,
    level_id: u32,
    player_fade: f32,
) -> TravelStep {
    let mut step = TravelStep::default();
    if let Some(flags) = crystal
        && flags & 0xffff == 0x1af
    {
        let charge = crystal_charge(stored_color, now.wrapping_sub(started));
        let old = (flags >> 16) & 0xff;
        if u32::from(charge) != old {
            let new_flags = (flags & 0xff00_ffff) | (u32::from(charge) << 16);
            step.charge = Some(charge);
            step.flags = Some(new_flags);
        }
    }
    match mode {
        4 => step.end_session = true,
        6 => {
            step.remove_level = Some(level_id);
            if player_fade > 0.01 {
                step.save = true;
            }
            step.done = true;
        }
        _ => {}
    }
    step
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_fill_in_the_originals_order() {
        // The first entry takes row 1, the rest rows 2 and 3; row 0 takes
        // the last entry once the other rows are full.
        assert_eq!(fill_rows(&[7, 10, 12, 13], 4), [3, 0, 1, 2]);
        assert_eq!(fill_rows(&[0, 7, 12, 13], 4), [0, 1, 2, 3]);
        assert_eq!(fill_rows(&[7, 0, 12, 13], 4), [1, 0, 2, 3]);
        assert_eq!(fill_rows(&[7, 10, 0, 13], 4), [2, 0, 1, 3]);
        assert_eq!(fill_rows(&[0, 0, 0, 7], 4), [2, 3, 0, 0]);
        assert_eq!(fill_rows(&[7, 10, 12, 13], 1), [0]);
        assert_eq!(fill_rows(&[7, 10, 12, 13], 0), Vec::<usize>::new());
        // A fifth entry is not looked at.
        assert_eq!(fill_rows(&[7, 10, 12, 13, 14], 4), [3, 0, 1, 2]);
    }

    #[test]
    fn a_screen_opens_reveals_and_closes_on_its_timings() {
        let mut screen = DestinationScreen::open(vec![7, 10, 12, 13], 4, true, 1_000);
        assert_eq!(screen.reveal_until, 1_500);
        assert_eq!(screen.state, state::REVEALING);
        assert_eq!(screen.rows(), 4);
        assert_eq!(screen.slots[0].symbol, 3);
        assert_eq!(screen.slots[1].symbol, 0);
        // At once the reveal is at zero.
        let frame = screen.frame(1_000);
        assert_eq!(frame.turn, 0.0);
        assert_eq!(frame.alpha, 0.0);
        // Half way (250 ms left): 0.5 eased to 0.75.
        let frame = screen.frame(1_250);
        assert!((frame.turn - 0.75).abs() < 1e-6);
        assert!((frame.alpha - 0.75).abs() < 1e-6);
        // Past the deadline the reveal bit clears and the screen is full.
        let frame = screen.frame(1_500);
        assert_eq!(screen.state, 0);
        assert_eq!(frame.turn, 1.0);
        assert_eq!(frame.alpha, 1.0);
        // Closing: the 256 ms tail scales the alpha.
        screen.close(2_000);
        assert_eq!(screen.close_until, 2_256);
        assert_eq!(screen.frame(2_000).alpha, 1.0);
        let frame = screen.frame(2_156);
        assert!((frame.alpha - (100.0 / 256.0)).abs() < 1e-6);
        let frame = screen.frame(2_256);
        assert!(frame.closed);
        assert_eq!(screen.state, state::CLOSED);
        assert_eq!(screen.frame(2_300).turn, 0.0);
    }

    #[test]
    fn the_leave_rule_waits_for_the_tail_and_the_reveal() {
        let mut screen = DestinationScreen::open(vec![1], 1, true, 1_000);
        assert!(!screen.may_leave(1_000));
        assert!(!screen.may_leave(1_099));
        assert!(screen.may_leave(1_100));
        screen.close(2_000);
        assert!(!screen.may_leave(2_100));
        let mut closed = DestinationScreen::open(vec![1], 1, true, 1_000);
        closed.state = state::CLOSED;
        assert!(closed.may_leave(1_000));
    }

    #[test]
    fn nodes_sit_on_the_ring_and_scale_with_the_turn() {
        // Four rows: the centre, then left, upper right, lower right.
        assert_eq!(place_node(0, 4, 200, 100, 4, 1.0), (100.0, 50.0));
        let (x, y) = place_node(1, 4, 200, 100, 4, 1.0);
        assert!((x + 60.0).abs() < 1e-4, "{x}");
        assert!((y - 50.0).abs() < 1e-4, "{y}");
        let (x, y) = place_node(2, 4, 200, 100, 4, 1.0);
        assert!((x - 180.0).abs() < 1e-4, "{x}");
        assert!((y - 189.0).abs() < 1e-4, "{y}");
        // Half turn: the radius shrinks; the 1 - turn angle offset too.
        let (x, y) = place_node(1, 4, 200, 100, 4, 0.5);
        assert!((x - 30.0).abs() < 1e-3, "{x}");
        assert!((y - 11.5).abs() < 1e-3, "{y}");
        let (x, y) = place_node(2, 4, 200, 100, 4, 0.5);
        assert!((x - 102.0).abs() < 1e-3, "{x}");
        assert!((y - 130.0).abs() < 1e-3, "{y}");
        // A one-row screen has only the centre.
        assert_eq!(place_node(1, 1, 200, 100, 4, 1.0), (100.0, 50.0));
    }

    #[test]
    fn hits_land_on_the_ring_nodes_only() {
        let hit = hit_node((-60, 50), 4, 200, 100, 4, 1.0);
        assert_eq!(hit, 2);
        assert_eq!(hit_node((180, 189), 4, 200, 100, 4, 1.0), 3);
        assert_eq!(hit_node((180, -89), 4, 200, 100, 4, 1.0), 4);
        // The centre is not a target.
        assert_eq!(hit_node((100, 50), 4, 200, 100, 4, 1.0), -1);
        // 14 units (56 px) is inside, a pixel more is not.
        assert_eq!(hit_node((-4, 50), 4, 200, 100, 4, 1.0), 2);
        assert_eq!(hit_node((-3, 50), 4, 200, 100, 4, 1.0), -1);
    }

    #[test]
    fn a_symbol_is_thirty_two_units_and_uses_a_two_by_two_sheet() {
        let quad = symbol_quad((100.0, 50.0), 4, 1.0, 3);
        assert_eq!(quad.corners, [36.0, -14.0, 164.0, 114.0]);
        assert_eq!(quad.cell, (0.5, 0.0));
        let quad = symbol_quad((100.0, 50.0), 4, 0.5, 1);
        assert_eq!(quad.corners, [68.0, 18.0, 132.0, 82.0]);
        assert_eq!(quad.cell, (0.5, 0.5));
        let quad = symbol_quad((0.0, 0.0), 4, 1.0, 0);
        assert_eq!(quad.cell, (0.0, 0.5));
    }

    #[test]
    fn the_flash_reflects_a_deadline_ahead_or_restarts_it() {
        let mut slot = DestinationSlot::new(0);
        bump_flash(&mut slot, 1_000);
        assert_eq!(slot.flash_until, 1_200);
        // A deadline ahead reflects: 2 * 1000 + 200 - 1100.
        slot.flash_until = 1_100;
        bump_flash(&mut slot, 1_000);
        assert_eq!(slot.flash_until, 1_100);
        slot.flash_until = 1_500;
        bump_flash(&mut slot, 1_000);
        assert_eq!(slot.flash_until, 700);
    }

    #[test]
    fn hover_restarts_both_rows_and_returns_three() {
        let mut screen = DestinationScreen::open(vec![7, 10, 12], 3, true, 1_000);
        assert_eq!(screen.hover(1, 1_000), 3);
        assert_eq!(screen.selected, 1);
        assert_eq!(screen.slots[0].flash_until, 1_200);
        assert_eq!(screen.hover(2, 1_000), 3);
        assert_eq!(screen.selected, 2);
        assert_eq!(screen.slots[0].flash_until, 1_000);
        assert_eq!(screen.slots[1].flash_until, 1_200);
        assert_eq!(screen.hover(0, 1_000), 3);
        assert_eq!(screen.selected, 0);
        // Leaving with no row hovered is forwarded.
        assert_eq!(screen.hover(0, 1_000), 0);
    }

    #[test]
    fn a_slots_fade_falls_from_its_deadline() {
        let mut slot = DestinationSlot::new(0);
        slot.flash_until = 1_100;
        assert_eq!(arrival_fade(&slot, 1_000), 0.5);
        assert_eq!(arrival_fade(&slot, 1_050), 0.75);
        assert_eq!(arrival_fade(&slot, 1_101), 1.0);
        assert_eq!(node_mix(0.5, 1, 2), 0.5);
        assert_eq!(node_mix(0.5, 1, 1), 0.5);
        assert_eq!(node_mix(0.5, 0, 2), 0.5);
    }

    #[test]
    fn the_wobble_follows_its_three_branches() {
        let mut slot = DestinationSlot::new(0);
        // A value inside the band takes the two-sided nudge: 2*(0.75-0.5)*0.2.
        slot.wobble[0] = 0.0;
        let mut draws = [0.75f32, 0.5, 0.0, 0.0].into_iter();
        wobble_step(&mut slot, 0.2, || draws.next().unwrap());
        assert!((slot.drift[0] - 0.1).abs() < 1e-6);
        assert!((slot.wobble[0] - 0.1 * 0.2 * 2.0 * 0.5).abs() < 1e-6);
        // Below -0.9 adds one-sided; above 0.9 subtracts.
        let mut slot = DestinationSlot::new(0);
        slot.wobble[0] = -0.95;
        slot.drift[0] = 0.0;
        let mut draws = [0.5f32, 0.0, 0.0, 0.0].into_iter();
        wobble_step(&mut slot, 0.2, || draws.next().unwrap());
        assert!((slot.drift[0] - 0.1).abs() < 1e-6);
        let mut slot = DestinationSlot::new(0);
        slot.wobble[0] = 0.95;
        slot.drift[0] = 0.0;
        let mut draws = [0.5f32, 0.0, 0.0, 0.0].into_iter();
        wobble_step(&mut slot, 0.2, || draws.next().unwrap());
        assert!((slot.drift[0] + 0.1).abs() < 1e-6);
        // The clamp keeps the wobble in range.
        slot.wobble[0] = 1.0;
        slot.drift[0] = 100.0;
        let mut draws = [1.0f32, 0.0, 0.0, 0.0].into_iter();
        wobble_step(&mut slot, 10.0, || draws.next().unwrap());
        assert_eq!(slot.wobble[0], 1.0);
        // The flash is 0.02 a millisecond.
        assert!((flash_amount(1_050, 1_000) - 1.0).abs() < 1e-6);
        assert_eq!(flash_amount(1_000, 1_000), 0.0);
    }

    #[test]
    fn the_paint_draws_the_backdrop_then_every_symbol() {
        let mut screen = DestinationScreen::open(vec![7, 10, 12, 13], 4, true, 1_000);
        screen.turn = 1.0;
        screen.selected = 2;
        let draw = draw_order(&screen, 200, 100, 4, 1_000);
        assert_eq!(draw[0], SelectionDraw::Backdrop);
        assert_eq!(draw.len(), 5);
        // Row 0 is the centre, row 1 the left node and the selected one
        // (selected is 1-based); the others keep the base ink.
        assert_eq!(
            draw[1],
            SelectionDraw::Symbol {
                row: 0,
                corners: [36.0, -14.0, 164.0, 114.0],
                mix: 0.0,
            }
        );
        assert_eq!(
            draw[2],
            SelectionDraw::Symbol {
                row: 1,
                corners: [-124.0, -14.0, 4.0, 114.0],
                mix: 1.0,
            }
        );
        assert_eq!(
            draw[3],
            SelectionDraw::Symbol {
                row: 2,
                corners: [116.0, 125.0, 244.0, 253.0],
                mix: 0.0,
            }
        );
    }

    #[test]
    fn clicks_start_travel_or_dismiss() {
        let mut screen = DestinationScreen::open(vec![7, 10, 12], 3, true, 1_000);
        screen.state = 0;
        screen.selected = 2;
        // Row 1 shows the first entry (7) after the fill, row 2 entry 0.
        assert_eq!(click(&screen, 0, true), Click::Travel(7));
        screen.selected = 1;
        assert_eq!(click(&screen, 0, true), Click::Travel(12));
        assert_eq!(click(&screen, 2, true), Click::Dismiss);
        assert_eq!(click(&screen, 0, false), Click::None);
        screen.selected = 0;
        assert_eq!(click(&screen, 0, true), Click::None);
    }

    #[test]
    fn a_journey_times_its_bubble_and_charge() {
        let adapter = journey(true, 7, 9);
        assert!(adapter.hold);
        assert_eq!((adapter.start_ms, adapter.end_ms), (1_500, 7_000));
        assert_eq!(adapter.level_id, 7);
        let small = journey(false, 7, 9);
        assert!(!small.hold);
        assert_eq!((small.start_ms, small.end_ms), (1_000, 1_400));
        assert_eq!(bubble_origin((100.0, 50.0)), (100.0, 148.0));
        // 12 ms a point, clamped at 255.
        assert_eq!(crystal_charge(0, 0), 0);
        assert_eq!(crystal_charge(0, 11), 0);
        assert_eq!(crystal_charge(0, 12), 1);
        assert_eq!(crystal_charge(250, 120), 255);
        assert_eq!(crystal_charge(255, 0), 255);
    }

    #[test]
    fn a_travel_tick_charges_removes_and_saves() {
        let flags = 0x0000_01af;
        let step = travel_step(Some(flags), 0, 1_000, 1_120, 0, 7, 0.0);
        assert_eq!(step.charge, Some(10));
        assert_eq!(step.flags, Some(0x000a_01af));
        assert!(!step.end_session);
        assert!(!step.done);
        // The same charge writes nothing.
        let step = travel_step(Some(0x000a_01af), 0, 1_000, 1_120, 0, 7, 0.0);
        assert_eq!(step.charge, None);
        assert_eq!(step.flags, None);
        // Another crystal is left alone.
        let step = travel_step(Some(0x0000_01f4), 0, 1_000, 1_120, 0, 7, 0.0);
        assert_eq!(step.flags, None);
        // Mode 4 asks the session to end; mode 6 removes the level and
        // saves once the player's fade passes 0.01.
        assert!(travel_step(None, 0, 0, 0, 4, 7, 0.0).end_session);
        let step = travel_step(None, 0, 0, 0, 6, 7, 0.005);
        assert_eq!(step.remove_level, Some(7));
        assert!(!step.save);
        assert!(step.done);
        let step = travel_step(None, 0, 0, 0, 6, 7, 0.02);
        assert!(step.save);
        assert!(!travel_step(None, 0, 0, 0, 9, 7, 1.0).done);
    }
}
