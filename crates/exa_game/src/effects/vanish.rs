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
//! The displacement unit's phase table ([`PHASE_TABLE`], [`VanishPhase`])
//! and its own step ([`Vanish::advance`]: an entry whose end has passed
//! runs its callback and the phase moves on; the last entry hides and lets
//! go) and its shown amount ([`Vanish::phase_fade`]: a 0x400 entry fades
//! between its start and end, reversed without 0x800; 0x1000 shows one).
//! The phased body object itself and the two calls the start ends with
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
/// The show call's argument when shown, and when hidden, and the staged
/// phase's.
pub const SHOW_CALL: u32 = 0x30;
pub const HIDE_CALL: u32 = 0;
pub const STAGE_CALL: u32 = 0x10;
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

/// What a phase of the displacement unit's table does to the bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseCall {
    /// Nothing.
    None,
    /// Sets render bit 4 and calls each body's show with 0x10 (the staged
    /// phase).
    Stage,
    /// Shows the bodies (bit 4 already set, the show call with 0x30).
    Show,
    /// Hides the bodies (bit 4 cleared, the show call with 0).
    Hide,
}

/// One entry of the displacement unit's phase table: its flags (0x400
/// fades over the entry, 0x800 reverses the fade, 0x1000 shows at one),
/// the phase's start and end in milliseconds, and its callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanishPhase {
    pub flags: u32,
    pub start: u32,
    /// The end, an unsigned comparison against the elapsed time; a −1
    /// marks the table's last entry (read as a signed compare by the
    /// hook).
    pub end: i64,
    pub call: PhaseCall,
}

/// The displacement unit's phase table, as stored (eight entries of 24
/// bytes: the flags, the start, the end and the callback).
pub const PHASE_TABLE: [VanishPhase; 8] = [
    VanishPhase {
        flags: 0,
        start: 0,
        end: -16,
        call: PhaseCall::None,
    },
    VanishPhase {
        flags: 0x1100,
        start: 0,
        end: 350,
        call: PhaseCall::Stage,
    },
    VanishPhase {
        flags: 0x600,
        start: 350,
        end: 450,
        call: PhaseCall::Stage,
    },
    VanishPhase {
        flags: 0x300,
        start: 450,
        end: -16,
        call: PhaseCall::Show,
    },
    VanishPhase {
        flags: 0,
        start: 500,
        end: -16,
        call: PhaseCall::None,
    },
    VanishPhase {
        flags: 0x200,
        start: 0,
        end: 200,
        call: PhaseCall::Stage,
    },
    VanishPhase {
        flags: 0xe00,
        start: 200,
        end: 400,
        call: PhaseCall::Hide,
    },
    VanishPhase {
        flags: 0x1100,
        start: 400,
        end: -1,
        call: PhaseCall::None,
    },
];

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

    /// The current table entry.
    pub fn phase_entry(&self) -> VanishPhase {
        PHASE_TABLE[self.phase as usize % PHASE_TABLE.len()]
    }

    #[allow(clippy::manual_clamp)]
    /// The shown amount the current entry's flags give at `now_ms`: a 0x400
    /// entry fades from its start to its end (clamped to 0..1, the original
    /// comparing with `<=` so a not-a-number passes), reversed when 0x800 is
    /// clear; 0x1000 shows one; anything else zero.
    pub fn phase_fade(&self, now_ms: u32) -> f32 {
        let entry = self.phase_entry();
        if entry.flags & 0x400 != 0 {
            let elapsed = now_ms.wrapping_sub(self.since_ms) as f32;
            let end = entry.end as u32 as f32;
            let start = entry.start as f32;
            let mut fade = (f64::from(elapsed - start) / f64::from(end - start)) as f32;
            if fade <= 0.0 {
                fade = 0.0;
            }
            if 1.0 <= fade {
                fade = 1.0;
            }
            if entry.flags & 0x800 == 0 {
                fade = 1.0 - fade;
            }
            fade
        } else if entry.flags & 0x1000 != 0 {
            1.0
        } else {
            0.0
        }
    }

    /// The table's own step at `now_ms` (the original's phase tick ran
    /// before the phase hook): when the current entry's end has passed, its
    /// callback runs and the phase advances; a table entry whose end is −1
    /// hides and lets go of the body (the phase hook's end).
    pub fn advance(&mut self, now_ms: u32) -> bool {
        let entry = self.phase_entry();
        // The last entry (its end −1) is the table's end: hide and let go.
        if entry.end == -1 {
            self.phase_hook(true);
            return true;
        }
        let elapsed = now_ms.wrapping_sub(self.since_ms);
        if entry.end >= 0 && (entry.end as u64) < u64::from(elapsed) {
            match entry.call {
                PhaseCall::None => {}
                PhaseCall::Stage => {
                    for body in &mut self.bodies {
                        body.flags |= SHOWN_BIT;
                        body.call = STAGE_CALL;
                    }
                }
                PhaseCall::Show => self.show(),
                PhaseCall::Hide => self.hide(),
            }
            self.phase = self.phase.wrapping_add(1);
        }
        false
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
    fn the_phase_table_runs_its_entries_and_shows_at_four() {
        // The table's eight entries, as stored.
        assert_eq!(PHASE_TABLE.len(), 8);
        assert_eq!(
            (
                PHASE_TABLE[1].flags,
                PHASE_TABLE[1].start,
                PHASE_TABLE[1].end
            ),
            (0x1100, 0, 350)
        );
        assert_eq!(
            (
                PHASE_TABLE[2].flags,
                PHASE_TABLE[2].start,
                PHASE_TABLE[2].end
            ),
            (0x600, 350, 450)
        );
        assert_eq!(PHASE_TABLE[3].call, PhaseCall::Show);
        assert_eq!((PHASE_TABLE[6].start, PHASE_TABLE[6].end), (200, 400));
        assert_eq!(PHASE_TABLE[6].call, PhaseCall::Hide);
        assert_eq!(PHASE_TABLE[7].end, -1);
        let mut vanish = Vanish::start(None, false, 1_000, vec![body()], vec![1.0]);
        assert_eq!(vanish.phase, 1);
        // Phase 1 shows (0x1100 carries 0x1000, not 0x400).
        assert_eq!(vanish.phase_fade(1_000), 1.0);
        assert_eq!(vanish.phase_fade(1_175), 1.0);
        assert_eq!(vanish.phase_fade(1_350), 1.0);
        // Phase 2's 0x600 entry fades out from 350 to 450 without 0x1000.
        vanish.phase = 2;
        assert_eq!(vanish.phase_fade(1_350), 1.0);
        assert_eq!(vanish.phase_fade(1_400), 0.5);
        assert_eq!(vanish.phase_fade(1_450), 0.0);
        // The third entry has neither a fade nor a show bit: zero.
        vanish.phase = 3;
        assert_eq!(vanish.phase_fade(1_400), 0.0);
        // The sixth fades the other way, 0 at 200 ms to 1 at 400.
        vanish.phase = 6;
        assert_eq!(vanish.phase_fade(1_250), 0.25);
        assert_eq!(vanish.phase_fade(1_300), 0.5);
        assert_eq!(vanish.phase_fade(1_450), 1.0);
        // Advancing runs the callbacks: phase 1 stages, phase 2 stages,
        // phase 3 shows, and the last entry hides and lets go.
        vanish.phase = 1;
        let mut staged = Vanish::start(None, false, 0, vec![body()], vec![]);
        staged.hide();
        assert!(!staged.advance(350), "the end is not passed yet");
        assert!(!staged.advance(351));
        assert_eq!(staged.phase, 2);
        assert_eq!(staged.bodies[0].flags, 0xf);
        assert_eq!(staged.bodies[0].call, STAGE_CALL);
        assert!(!staged.advance(451));
        assert_eq!(staged.phase, 3);
        // Entries 3 and 4 wait on the phased body's own hook (their end is
        // −16, never passed): the hook shows and moves to phase 4.
        assert!(!staged.advance(501));
        assert_eq!(staged.phase, 3);
        staged.phase_hook(false);
        assert_eq!(staged.phase, 4);
        assert_eq!(staged.bodies[0].shown, 1.0);
        assert_eq!(staged.bodies[0].call, SHOW_CALL);
        staged.phase = 7;
        assert!(staged.advance(1));
        assert_eq!(staged.bodies[0].shown, 0.0);
        assert_eq!(staged.phased, None);
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
