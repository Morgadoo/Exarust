//! The player controls' rules that are not the hand's drags nor the hotkey
//! dispatch: the values an item used on the world leaves (a pour, a
//! cleaning or a repair kit) and the timed list that carries a kit's change
//! over two seconds, and the start and end of play. Rules: `docs/PLAYER.md`
//! (Using what the cursor is over, Using an item).
//!
//! **Using an item.** The usable section's subtype names the use: 8 pours a
//! liquid that warms the character it is poured on by 0.2 (0.1 for a
//! liquid of type 50), 14 pours one that marks it and stamps an 1800 ms
//! timer, 9 cleans and 10 repairs. A cleaning kit takes an item's dirt byte
//! down to 0x20 when it is over that; a repair kit lowers the wear byte by
//! 255 over the material's rating `(r + 5) / 10` (at least 1), never under
//! zero. Both schedule a timed value that moves 0.0005 of the gap a
//! millisecond (two seconds); a list entry is dropped when it arrives.
//!
//! **Start and end of play.** The start makes the AI's activation centre
//! the player with a 40 m radius (its square, 16 000 000 cm²); the end puts
//! the centre back at the origin with 200 m (400 000 000 cm²).
//!
//! Not in Exarust: the click and double-click handlers (the item card, the
//! narrator's line, the inventory, loot and talk windows they open), the
//! dispatcher that picks the target of a use, and a consumer for the
//! activation centre.
//!
//! Evidence: the use routines, the timed list and the start and end of
//! play, read in the instructions (`context/re/player-control-hand.md`).

use bevy::math::Vec3;
use bevy::prelude::Resource;

/// The usable section's subtypes the use dispatcher reads.
pub const POUR: u8 = 8;
pub const POUR_MARKS: u8 = 14;
pub const CLEAN: u8 = 9;
pub const REPAIR: u8 = 10;
/// The liquid type that warms by the smaller step.
pub const SMALL_POUR_TYPE: u16 = 50;
/// The timer (ms of the game clock) a marking pour stamps on its target.
pub const POUR_MARK_MS: u32 = 1800;
/// A cleaning kit leaves the dirt byte at this.
pub const CLEAN_TO: u8 = 0x20;

/// The warmth a pour leaves on the character it is poured on: `warmth` plus
/// 0.1 for a liquid of type 50 and 0.2 for any other, added in double
/// precision.
pub fn poured(warmth: f32, liquid_type: u16) -> f32 {
    let add = if liquid_type == SMALL_POUR_TYPE {
        0.1
    } else {
        0.2
    };
    (f64::from(warmth) + add) as f32
}

/// The cleaning kit's target: the dirt byte goes to 0x20 when it is over
/// it; at or under it nothing is cleaned.
pub fn clean_to(dirt: u8) -> Option<u8> {
    (dirt > CLEAN_TO).then_some(CLEAN_TO)
}

/// The repair kit's target: the wear byte drops by 255 over the material's
/// rating step `(rating + 5) / 10` (at least 1), never under zero; an
/// unworn item is not repaired.
pub fn repair_to(wear: u8, rating: u8) -> Option<u8> {
    if wear == 0 {
        return None;
    }
    let step = ((i32::from(rating) + 5) / 10).max(1);
    Some((i32::from(wear) - 255 / step).max(0) as u8)
}

/// A timed value on an item: the byte's kind (cleaning or repair), the
/// value it starts at and the value it stops at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timed {
    /// [`CLEAN`] or [`REPAIR`].
    pub kind: u8,
    pub from: u8,
    pub to: u8,
    /// The game clock (ms) it started.
    pub started_ms: u32,
}

/// The fraction of its gap a timed value moves each millisecond (in double
/// precision), so a full value takes two seconds.
pub const TIMED_RATE: f64 = 0.0005;

impl Timed {
    /// The value at `now_ms`: from toward to by 0.0005 of the gap a ms,
    /// rounded to the nearest (ties to even), or the end value once the gap
    /// is spent. Returns the value and whether the entry is done.
    pub fn value(&self, now_ms: u32) -> (u8, bool) {
        let elapsed = f64::from(now_ms) - f64::from(self.started_ms);
        let f = (elapsed * TIMED_RATE) as f32;
        if f < 1.0 {
            let from = f32::from(self.from);
            let to = f32::from(self.to);
            (((to - from) * f + from).round_ties_even() as u8, false)
        } else {
            (self.to, true)
        }
    }
}

/// The timed list: entries applied to an item each frame, removed when they
/// arrive. The original holds them in an array, takes a finished entry out
/// by moving the last into its place, and counts its entries with a length.
/// (It writes a value only while it lowers the item's byte or arrives;
/// Exarust returns every value and leaves that to the writer.)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimedList {
    pub entries: Vec<TimedItem>,
}

/// A timed entry and the item it changes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimedItem {
    pub timed: Timed,
    pub item: u32,
}

impl TimedList {
    /// Adds an entry, returning its index.
    pub fn push(&mut self, item: TimedItem) -> usize {
        self.entries.push(item);
        self.entries.len() - 1
    }

    /// One frame at `now_ms`: every entry's value, and the entries that
    /// arrive are removed (the last moved into the slot), as the original's
    /// loop does. Returns the values to write, in the order the loop reads
    /// the shrinking array.
    pub fn tick(&mut self, now_ms: u32) -> Vec<(u32, u8)> {
        let mut writes = Vec::with_capacity(self.entries.len());
        let mut i = 0;
        while i < self.entries.len() {
            let entry = self.entries[i];
            let (value, done) = entry.timed.value(now_ms);
            writes.push((entry.item, value));
            if done {
                self.entries.swap_remove(i);
            } else {
                i += 1;
            }
        }
        writes
    }
}

/// The squared activation radius (cm²) the start of play gives the AI
/// around the player: 16 000 000, 40 m.
pub const START_ACTIVATION_RADIUS2: f32 = 16_000_000.0;
/// The squared activation radius (cm²) the end of play resets it to:
/// 400 000 000, 200 m.
pub const END_ACTIVATION_RADIUS2: f32 = 400_000_000.0;

/// The session state the start and end of play set: the AI activation
/// centre (m) and squared radius (cm²), and whether the player's controls
/// are in play.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct PlaySession {
    pub activation_centre: Vec3,
    pub activation_radius2: f32,
    /// The player's controls run (the original's playing flag).
    pub playing: bool,
}

impl Default for PlaySession {
    fn default() -> Self {
        Self {
            activation_centre: Vec3::ZERO,
            activation_radius2: END_ACTIVATION_RADIUS2,
            playing: false,
        }
    }
}

impl PlaySession {
    /// The start of play at the player's `centre`: the controls run and the
    /// AI activates around the player (a 40 m radius).
    pub fn begin(&mut self, centre: Vec3) {
        self.playing = true;
        self.activation_centre = centre;
        self.activation_radius2 = START_ACTIVATION_RADIUS2;
    }

    /// The end of play: the controls stop, the activation centre goes back
    /// to the origin with a 200 m radius.
    pub fn end(&mut self) {
        self.playing = false;
        self.activation_centre = Vec3::ZERO;
        self.activation_radius2 = END_ACTIVATION_RADIUS2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pour warms by 0.2, or 0.1 for the type-50 liquid, in double
    /// precision: 0.3 + 0.2 is 0.5 exactly once rounded back.
    #[test]
    fn a_pour_warms_in_double_precision() {
        assert_eq!(poured(0.3, 1), 0.5);
        assert_eq!(poured(0.3, SMALL_POUR_TYPE), 0.4);
        assert_eq!(poured(0.0, 7), 0.2);
    }

    /// The cleaning kit takes the dirt to 0x20 from over it; the repair kit
    /// lowers the wear by 255 over the rating step, floored at zero, and
    /// leaves an unworn item.
    #[test]
    fn cleaning_and_repairing() {
        assert_eq!(clean_to(0x30), Some(0x20));
        assert_eq!(clean_to(0x21), Some(0x20));
        assert_eq!(clean_to(0x20), None);
        assert_eq!(clean_to(0), None);
        // rating 25 → (25 + 5)/10 = 3 → 255/3 = 85 off.
        assert_eq!(repair_to(100, 25), Some(15));
        assert_eq!(repair_to(50, 25), Some(0));
        // rating 0 → step 1 → 255 off, floored at 0.
        assert_eq!(repair_to(100, 0), Some(0));
        // rating 45 → 5 → 51 off.
        assert_eq!(repair_to(100, 45), Some(49));
        assert_eq!(repair_to(0, 45), None);
    }

    /// A timed value moves 0.0005 of its gap a millisecond, rounding to the
    /// nearest byte (ties to even), and is exactly its end at two seconds;
    /// a clock behind its start runs it backwards.
    #[test]
    fn a_timed_value_eases_over_two_seconds() {
        let t = Timed {
            kind: 10,
            from: 100,
            to: 0,
            started_ms: 1000,
        };
        assert_eq!(t.value(1000), (100, false));
        assert_eq!(t.value(2000), (50, false));
        assert_eq!(t.value(2500), (25, false));
        assert_eq!(t.value(3000), (0, true));
        assert_eq!(t.value(9000), (0, true));
        // 0.0005 of a 32 gap at 15 ms is 0.24: 48 − 0.24 = 47.76, which
        // the nearest-even cast takes to 48.
        let small = Timed {
            kind: 9,
            from: 0x30,
            to: 0x10,
            started_ms: 0,
        };
        assert_eq!(small.value(15), (48, false));
        // 100 ms before its start: f = −0.05, 100 + 5 = 105.
        assert_eq!(t.value(900), (105, false));
    }

    /// The list applies every entry and takes out the finished ones by
    /// moving the last into their slot.
    #[test]
    fn the_timed_list_drops_finished_entries() {
        let mut list = TimedList::default();
        list.push(TimedItem {
            timed: Timed {
                kind: 10,
                from: 100,
                to: 0,
                started_ms: 0,
            },
            item: 7,
        });
        list.push(TimedItem {
            timed: Timed {
                kind: 9,
                from: 0x30,
                to: 0x10,
                started_ms: 0,
            },
            item: 9,
        });
        let at = |list: &mut TimedList, ms| list.tick(ms);
        assert_eq!(at(&mut list, 1000), vec![(7, 50), (9, 0x20)]);
        assert_eq!(list.entries.len(), 2);
        assert_eq!(at(&mut list, 2000), vec![(7, 0), (9, 0x10)]);
        assert!(list.entries.is_empty());
    }

    /// The start of play centres the AI's activation on the player with
    /// 40 m; the end resets it to the origin with 200 m.
    #[test]
    fn the_start_and_end_of_play() {
        let mut s = PlaySession::default();
        assert!(!s.playing);
        s.begin(Vec3::new(10.0, 0.0, 20.0));
        assert!(s.playing);
        assert_eq!(s.activation_centre, Vec3::new(10.0, 0.0, 20.0));
        assert_eq!(s.activation_radius2, 16_000_000.0);
        s.end();
        assert!(!s.playing);
        assert_eq!(s.activation_centre, Vec3::ZERO);
        assert_eq!(s.activation_radius2, 400_000_000.0);
        // The squared radii root to 40 m and 200 m.
        assert_eq!(START_ACTIVATION_RADIUS2.sqrt(), 4000.0);
        assert_eq!(END_ACTIVATION_RADIUS2.sqrt(), 20_000.0);
    }
}
