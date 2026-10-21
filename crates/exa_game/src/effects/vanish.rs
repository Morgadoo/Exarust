//! The displacement vanish: a body and its gear go out of sight while the
//! power holds, and come back when a phase of it says so.
//!
//! - **Hide** ([`Vanish::hide`]): each shown body loses bit 4 of its render
//!   flags and its shown amount goes to 0 (its show call gets 0); each item's
//!   shown amount goes to 0.
//! - **Show** ([`Vanish::show`]): each body's shown amount goes to 1 (its
//!   show call gets 0x30); each item's goes to 1. The flags stay as they
//!   are.
//! - **Start** ([`Vanish::start`]): a running constant displacement
//!   (power 0x20603) in the caster's list lends its phased body and its
//!   second object, whose flags gain 0x20; otherwise the vanish makes a
//!   phased body of its own with render flags 0x8000_0000 (and, unless it
//!   starts instantly, a second object with the same flags). An instant
//!   start sets the mode word to 0x10; otherwise the mode and the phase are
//!   1. The start time is the game clock.
//! - **The phase hook** ([`Vanish::phase_hook`]): the phased body calls it
//!   when a phase ends. At the end of the phase table the vanish hides
//!   again and lets go of the body; otherwise the phase becomes 4 and the
//!   vanish shows.
//! - **Finishing** ([`Vanish::finish`]): an own phased body whose fade
//!   counter is still under 4 keeps the vanish running. A lent body loses
//!   flags 0x30 and is hidden again before the vanish frees itself.
//!
//! The phase table, the phased body and the two calls the start ends with
//! belong to the displacement unit outside this group.
//!
//! Evidence: the effect's show, hide, start, finish and phase-hook
//! routines, read from `Exanima.exe`.

pub use crate::thaum::displacement::CONSTANT as CONSTANT_DISPLACEMENT;

/// The render-flag bit a hide clears.
pub const SHOWN_BIT: u32 = 0x4;
/// The render flags of a phased body the vanish makes.
pub const PHASED_FLAGS: u32 = 0x8000_0000;
/// The flag a lent body gains, and the two flags its finish clears.
pub const LENT_FLAG: u32 = 0x20;
pub const LENT_CLEAR: u32 = 0x30;
/// The show call's argument when shown, and when hidden.
pub const SHOW_CALL: u32 = 0x30;
pub const HIDE_CALL: u32 = 0;
/// The mode word of an instant start, and of a phased one.
pub const INSTANT_MODE: u32 = 0x10;
pub const PHASED_MODE: u32 = 1;
/// The phase a phase hook sets before showing.
pub const SHOWN_PHASE: u32 = 4;
/// An own phased body keeps the vanish running while its fade counter is
/// under this.
pub const FADE_DONE: u32 = 4;

/// One shown body of the vanish.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanishBody {
    /// Its render flags.
    pub flags: u32,
    /// Its shown amount, 0 or 1.
    pub shown: f32,
    /// The argument of its last show call.
    pub call: u32,
}

/// The vanish over a phased body, its shown bodies and their items.
#[derive(Debug, Clone, PartialEq)]
pub struct Vanish {
    /// Whether the phased body was lent by a running constant displacement.
    pub lent: bool,
    /// Whether the vanish started instantly.
    pub instant: bool,
    /// The phased body's render flags, while the vanish holds one.
    pub phased: Option<u32>,
    /// The second object's render flags, when there is one.
    pub second: Option<u32>,
    /// The mode word and the phase.
    pub mode: u32,
    pub phase: u32,
    /// The game clock at the start, ms.
    pub since_ms: u32,
    /// The bodies shown and hidden.
    pub bodies: Vec<VanishBody>,
    /// The items' shown amounts.
    pub items: Vec<f32>,
}

impl Vanish {
    /// The start. `lent` is the constant displacement's phased body and
    /// second object flags when it runs; the bodies and items are those the
    /// vanish hides.
    pub fn start(
        lent: Option<(u32, Option<u32>)>,
        instant: bool,
        now_ms: u32,
        bodies: Vec<VanishBody>,
        items: Vec<f32>,
    ) -> Self {
        let (phased, second) = match lent {
            Some((flags, second)) => (flags | LENT_FLAG, second),
            None => (PHASED_FLAGS, (!instant).then_some(PHASED_FLAGS)),
        };
        let (mode, phase) = if instant {
            (INSTANT_MODE, 0)
        } else {
            (PHASED_MODE, PHASED_MODE)
        };
        Self {
            lent: lent.is_some(),
            instant,
            phased: Some(phased),
            second,
            mode,
            phase,
            since_ms: now_ms,
            bodies,
            items,
        }
    }

    /// Hide the bodies and the items.
    pub fn hide(&mut self) {
        for body in &mut self.bodies {
            body.flags &= !SHOWN_BIT;
            body.shown = 0.0;
            body.call = HIDE_CALL;
        }
        self.items.fill(0.0);
    }

    /// Show the bodies and the items.
    pub fn show(&mut self) {
        for body in &mut self.bodies {
            body.shown = 1.0;
            body.call = SHOW_CALL;
        }
        self.items.fill(1.0);
    }

    /// The phased body's hook at the end of a phase: `table_end` when the
    /// phase table has run out.
    pub fn phase_hook(&mut self, table_end: bool) {
        if table_end {
            self.hide();
            self.phased = None;
        } else {
            self.phase = SHOWN_PHASE;
            self.show();
        }
    }

    /// The finish: true while the vanish keeps running. `fade` is the own
    /// phased body's fade counter.
    pub fn finish(&mut self, fade: u32) -> bool {
        if self.lent {
            if let Some(flags) = &mut self.phased {
                *flags &= !LENT_CLEAR;
            }
            self.hide();
            false
        } else {
            self.phased.is_some() && fade < FADE_DONE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> VanishBody {
        VanishBody {
            flags: 0xf,
            shown: 1.0,
            call: SHOW_CALL,
        }
    }

    #[test]
    fn an_own_start_makes_a_phased_body_and_a_lent_one_flags_it() {
        let own = Vanish::start(None, false, 100, vec![body()], vec![1.0]);
        assert_eq!(own.phased, Some(PHASED_FLAGS));
        assert_eq!(own.second, Some(PHASED_FLAGS));
        assert_eq!((own.mode, own.phase, own.since_ms), (1, 1, 100));
        let instant = Vanish::start(None, true, 0, vec![], vec![]);
        assert_eq!(instant.second, None);
        assert_eq!(instant.mode, INSTANT_MODE);
        let lent = Vanish::start(Some((0x1, Some(0x2))), false, 0, vec![], vec![]);
        assert_eq!(lent.phased, Some(0x21));
        assert_eq!(lent.second, Some(0x2));
        assert_eq!(CONSTANT_DISPLACEMENT, 0x20603);
    }

    #[test]
    fn hiding_clears_bit_four_and_showing_leaves_the_flags() {
        let mut vanish = Vanish::start(None, false, 0, vec![body()], vec![1.0, 1.0]);
        vanish.hide();
        assert_eq!(vanish.bodies[0].flags, 0xb);
        assert_eq!(vanish.bodies[0].shown, 0.0);
        assert_eq!(vanish.items, vec![0.0, 0.0]);
        vanish.show();
        assert_eq!(vanish.bodies[0].flags, 0xb);
        assert_eq!(vanish.bodies[0].shown, 1.0);
        assert_eq!(vanish.bodies[0].call, SHOW_CALL);
        assert_eq!(vanish.items, vec![1.0, 1.0]);
    }

    #[test]
    fn the_phase_hook_shows_until_the_table_ends() {
        let mut vanish = Vanish::start(None, false, 0, vec![body()], vec![]);
        vanish.hide();
        vanish.phase_hook(false);
        assert_eq!(vanish.phase, SHOWN_PHASE);
        assert_eq!(vanish.bodies[0].shown, 1.0);
        vanish.phase_hook(true);
        assert_eq!(vanish.bodies[0].shown, 0.0);
        assert_eq!(vanish.phased, None);
    }

    #[test]
    fn an_own_body_finishes_once_faded_and_a_lent_one_at_once() {
        let mut own = Vanish::start(None, false, 0, vec![], vec![]);
        assert!(own.finish(3));
        assert!(!own.finish(4));
        own.phased = None;
        assert!(!own.finish(0));
        let mut lent = Vanish::start(Some((0x31, None)), false, 0, vec![body()], vec![]);
        assert!(!lent.finish(0));
        assert_eq!(lent.phased, Some(0x1));
        assert_eq!(lent.bodies[0].shown, 0.0);
    }
}
