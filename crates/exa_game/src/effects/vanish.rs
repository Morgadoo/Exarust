//! The displacement vanish: the body and its gear go out of sight while the
//! power holds, and come back when it ends.
//!
//! - **Hide** ([`Vanish::hide`]): each of the effect's bodies loses its
//!   visible flag (bit 2 of its render flags) and its show byte goes to 0;
//!   each of its items' show byte goes to 0.
//! - **Show** ([`Vanish::show`]): each body's show byte goes to 1 and its
//!   show call runs (0x30); each item's show byte goes to 1.
//! - **Start** ([`Vanish::start`]): a running constant displacement (id
//!   0x20603) lends its bodies and items and gains body flag 0x20; else the
//!   effect makes a phased body of its own, hides it and sets render flag
//!   0x8000_0000. Started instantly (the instant flag 0x10) the body only
//!   hides; otherwise it lives through its phases.
//! - **Each update** ([`Vanish::update`]): a phase that has run out shows
//!   the body again and the effect ends; a phase still running keeps it
//!   hidden.
//!
//! Evidence: the effect's show, hide, start, finish and update routines,
//! read from `Exanima.exe`. The phase table and the phased body itself
//! belong to the displacement unit outside this group.

/// The render flag that makes a body invisible.
pub const HIDDEN_FLAG: u32 = 0x8000_0000;
/// The bit a hide clears in the body's flags.
pub const VISIBLE_BIT: u32 = 0x4;
/// The body flags set while the constant displacement lends the body.
pub const PHASING_FLAG: u32 = 0x20;
/// The instant start's mask.
pub const INSTANT_MASK: u32 = 0x10;

/// The vanish's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanishState {
    /// The body is hidden and the power runs.
    Hidden = 1,
    /// The power is ending.
    Ending = 2,
    /// The body is coming back.
    Showing = 3,
    /// Done.
    Done = 4,
}

/// The vanish effect over a body and its gear.
#[derive(Debug, Clone)]
pub struct Vanish {
    pub state: VanishState,
    /// The body render flags.
    pub body_flags: u32,
    /// The body's show byte.
    pub body_shown: u8,
    /// The items' show bytes.
    pub items_shown: Vec<u8>,
    /// Whether the constant displacement lent its body.
    pub lent: bool,
    /// The game clock the effect started, ms.
    pub since_ms: u32,
}

impl Vanish {
    /// A vanish of `items` items, hidden from `now_ms`.
    pub fn start(items: usize, lent: bool, now_ms: u32) -> Self {
        let mut vanish = Self {
            state: VanishState::Hidden,
            body_flags: HIDDEN_FLAG | PHASING_FLAG,
            body_shown: 1,
            items_shown: vec![1; items],
            lent,
            since_ms: now_ms,
        };
        vanish.hide();
        vanish
    }

    /// Hide the body and its items again.
    pub fn hide(&mut self) {
        self.body_flags &= !VISIBLE_BIT;
        self.body_shown = 0;
        self.items_shown.fill(0);
    }

    /// Show the body and its items.
    pub fn show(&mut self) {
        self.body_flags &= !PHASING_FLAG;
        self.body_shown = 1;
        self.items_shown.fill(1);
    }

    /// One update: a phase that has run out shows the body and ends the
    /// effect; otherwise the body stays hidden.
    pub fn update(&mut self, phase_over: bool) {
        if phase_over {
            self.state = VanishState::Done;
            self.show();
        } else {
            self.state = VanishState::Showing;
            self.hide();
        }
    }

    /// The finish check: with the instant flag set the effect always
    /// finishes; otherwise only once the body's fade is past four.
    pub fn can_finish(&self, instant: bool, body_fade: i32) -> bool {
        instant || body_fade >= 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_started_vanish_hides_the_body_and_its_items() {
        let vanish = Vanish::start(2, false, 100);
        assert_eq!(vanish.body_flags & VISIBLE_BIT, 0);
        assert_eq!(vanish.body_flags & HIDDEN_FLAG, HIDDEN_FLAG);
        assert_eq!(vanish.body_shown, 0);
        assert_eq!(vanish.items_shown, vec![0, 0]);
        assert_eq!(vanish.state, VanishState::Hidden);
        assert_eq!(vanish.since_ms, 100);
    }

    #[test]
    fn the_update_shows_only_once_the_phase_is_over() {
        let mut vanish = Vanish::start(1, true, 0);
        vanish.update(false);
        assert_eq!(vanish.state, VanishState::Showing);
        assert_eq!(vanish.body_shown, 0);
        vanish.update(true);
        assert_eq!(vanish.state, VanishState::Done);
        assert_eq!(vanish.body_shown, 1);
        assert_eq!(vanish.items_shown, vec![1]);
        assert_eq!(vanish.body_flags & PHASING_FLAG, 0);
    }

    #[test]
    fn a_lent_body_keeps_the_phasing_flag_until_it_shows() {
        let mut vanish = Vanish::start(0, true, 0);
        assert_eq!(vanish.body_flags & PHASING_FLAG, PHASING_FLAG);
        vanish.show();
        assert_eq!(vanish.body_flags & PHASING_FLAG, 0);
    }

    #[test]
    fn an_instant_vanish_finishes_at_any_fade() {
        let vanish = Vanish::start(0, false, 0);
        assert!(vanish.can_finish(true, 0));
        assert!(!vanish.can_finish(false, 3));
        assert!(vanish.can_finish(false, 4));
    }
}
