//! The Displacement powers' active classes: the vanish effect they share,
//! Vanish, the Long Door, Constant Displacement and the Summon.
//!
//! - **The shared vanish effect** ([`VanishBase`]): puts a displacement
//!   shell on the body (or reuses the one a running Constant Displacement
//!   already has), leaves the after-image and starts the phase machine. The
//!   phases are: 1 hide the body and set its vanished bit; 2 fade out over
//!   300 ms; 3 hold while the power is sustained (or, released, move the
//!   body and start the fade-in); 4 fade in over 400 ms and clear the
//!   vanished bit; 5 wait 300 ms and clean up.
//! - **Vanish** ([`Vanish`]): phase 3 spends half the power's tick cost
//!   while the caster stays away; a release moves nothing (the body never
//!   left its place) and fades it back in.
//! - **The Long Door** ([`LongDoor`]): starts at phase 0x10 asking the
//!   navigation grid for the aim's place; a found place starts the phases,
//!   an impossible one ends at once, and phase 3 moves the body there.
//! - **Constant Displacement** ([`ConstantDisplacement`]): a persistent
//!   class whose shell's shown value grows 0.002 a tick up to 0.12; ending
//!   marks the shell closing and releases the focus.
//! - **The Summon** ([`Summon`]): picks a place and a demon, then grows the
//!   effect over 3 s, fades over 1 s, waits 1 s and cleans up.
//!
//! Evidence: the vanish base, the three instant classes and the constant
//! power's control handler, read from `Exanima.exe`.

use super::PowerBase;
use crate::thaum::casting::{CastBody, start_cast};

/// The Displacement power ids the classes run.
pub const VANISH: u32 = 0x10601;
pub const LONG_DOOR: u32 = 0x602;
pub const CONSTANT_DISPLACEMENT: u32 = 0x20603;
pub const SUMMON: u32 = 0x604;

/// The cast motion Constant Displacement starts.
pub const CONSTANT_MOTION: u32 = 0x28;

/// The vanish effect's first phase: hidden, before the fade-out.
pub const HIDDEN: i32 = 1;
/// Fading out.
pub const FADING_OUT: i32 = 2;
/// Held or moved.
pub const HOLDING: i32 = 3;
/// Fading in.
pub const FADING_IN: i32 = 4;
/// The tail before cleanup.
pub const TAIL: i32 = 5;
/// The answer when the effect is done (0x12 in the original).
pub const DONE: i32 = 0x12;

/// The AI's first phase: resolving the destination.
pub const AI_PICK: i32 = 0x10;

/// How long the fade-out lasts, in milliseconds.
pub const FADE_OUT_MS: u32 = 300;
/// How long the move and the fade-in last, in milliseconds.
pub const FADE_IN_MS: u32 = 400;
/// How long the tail lasts, in milliseconds.
pub const TAIL_MS: u32 = 300;
/// How long the summon's effect grows, in milliseconds.
pub const SUMMON_GROW_MS: u32 = 3000;
/// How long the summon's effect fades, in milliseconds.
pub const SUMMON_FADE_MS: u32 = 1000;
/// How long the summon's tail lasts, in milliseconds.
pub const SUMMON_TAIL_MS: u32 = 1000;
/// The shell bit that marks a shell shared with Constant Displacement.
pub const SHARED_SHELL: u32 = 0x20;
/// How much the constant displacement's shell grows a tick.
pub const CONSTANT_GROW: f32 = 0.002;
/// The shown value the constant displacement's shell grows to.
pub const CONSTANT_FULL: f32 = 0.12;

/// The vanish effect's phase state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanishBase {
    pub phase: i32,
    pub deadline: u32,
    /// The player let the power go.
    pub released: bool,
    /// The shell belongs to a running Constant Displacement.
    pub shared: bool,
    /// The body's published visibility, 1 shown and 0 hidden.
    pub shown: f32,
    /// How many fades have run (the original's `+0x50`).
    pub step: i32,
}

impl VanishBase {
    /// Starts the effect at the phase the original's activation leaves:
    /// phase 1 for a player, phase 0x10 for the AI.
    pub fn begin(now: u32, shared: bool, ai: bool) -> Self {
        Self {
            phase: if ai { AI_PICK } else { HIDDEN },
            deadline: now,
            released: false,
            shared,
            shown: 1.0,
            step: 1,
        }
    }
}

/// What a vanish tick asks the game to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanishStep {
    /// The answer: [`DONE`] when the effect has finished, else 0.
    pub result: i32,
    pub shown: f32,
    /// Hide the body and set its vanished bit.
    pub hide: bool,
    /// Show the body and clear its vanished bit.
    pub show: bool,
    /// Move the body to the destination now.
    pub moved: bool,
    /// Focus this tick spends.
    pub spend: f32,
    /// Drop the shell this tick.
    pub drop_shell: bool,
    /// Mark the shell closing.
    pub close_shell: bool,
}

impl VanishStep {
    fn quiet(shown: f32) -> Self {
        Self {
            result: 0,
            shown,
            hide: false,
            show: false,
            moved: false,
            spend: 0.0,
            drop_shell: false,
            close_shell: false,
        }
    }
}

/// Steps the shared vanish effect to the end of the fade-in, moving the
/// body at the release. `drain` is the power's focus a tick.
pub fn step_vanish(base: &mut VanishBase, now: u32, drain: f32) -> VanishStep {
    match base.phase {
        HIDDEN => {
            base.phase = FADING_OUT;
            base.deadline = now.wrapping_add(FADE_OUT_MS);
            base.shown = 1.0;
            VanishStep {
                hide: true,
                close_shell: true,
                ..VanishStep::quiet(1.0)
            }
        }
        FADING_OUT => {
            if now < base.deadline {
                base.shown = (base.deadline - now) as f32 / FADE_OUT_MS as f32;
                VanishStep {
                    spend: drain,
                    ..VanishStep::quiet(base.shown)
                }
            } else {
                base.shown = 0.0;
                base.phase = HOLDING;
                base.deadline = now.wrapping_add(FADE_IN_MS);
                VanishStep {
                    drop_shell: true,
                    ..VanishStep::quiet(0.0)
                }
            }
        }
        HOLDING => {
            if !base.released {
                VanishStep {
                    spend: drain * 0.5,
                    ..VanishStep::quiet(0.0)
                }
            } else {
                base.phase = FADING_IN;
                base.deadline = now.wrapping_add(FADE_IN_MS);
                base.step = base.step.wrapping_add(1);
                VanishStep {
                    moved: true,
                    close_shell: true,
                    ..VanishStep::quiet(0.0)
                }
            }
        }
        FADING_IN => {
            if now < base.deadline {
                base.shown = 1.0 - (base.deadline - now) as f32 / FADE_IN_MS as f32;
                VanishStep::quiet(base.shown)
            } else {
                base.shown = 1.0;
                base.phase = TAIL;
                base.deadline = now.wrapping_add(TAIL_MS);
                VanishStep {
                    show: true,
                    drop_shell: true,
                    ..VanishStep::quiet(1.0)
                }
            }
        }
        TAIL => {
            if base.deadline <= now {
                VanishStep {
                    result: DONE,
                    ..VanishStep::quiet(1.0)
                }
            } else {
                VanishStep::quiet(1.0)
            }
        }
        _ => VanishStep::quiet(base.shown),
    }
}

/// One tick of Vanish.
#[derive(Debug, Clone, PartialEq)]
pub struct Vanish {
    pub base: VanishBase,
    /// The focus a vanished tick spends (`+0xb8`).
    pub drain: f32,
}

impl Vanish {
    /// A vanish effect beginning at `now`.
    pub fn new(now: u32, shared: bool, ai: bool, drain: f32) -> Self {
        Self {
            base: VanishBase::begin(now, shared, ai),
            drain,
        }
    }

    /// Lets the power go (the original's end event).
    pub fn release(&mut self) {
        self.base.released = true;
    }

    /// One control tick.
    pub fn control(&mut self, event: i32, now: u32) -> VanishStep {
        if event == 2 {
            self.base.released = true;
        }
        step_vanish(&mut self.base, now, self.drain)
    }
}

/// What the navigation grid answered when the Long Door or the Summon asked
/// for a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavAnswer {
    /// Still resolving.
    Waiting,
    /// A place was found.
    Found,
    /// No place is reachable.
    Impossible,
}

/// The Long Door: the phases plus the place it moves to.
#[derive(Debug, Clone, PartialEq)]
pub struct LongDoor {
    pub base: VanishBase,
    pub drain: f32,
    pub destination: Option<[f32; 3]>,
}

impl LongDoor {
    /// A long door that has not asked the navigation grid yet; a player's
    /// destination is known at the cast.
    pub fn new(now: u32, destination: Option<[f32; 3]>, ai: bool, drain: f32) -> Self {
        let mut base = VanishBase::begin(now, false, ai);
        if !ai {
            base.phase = HIDDEN;
        }
        Self {
            base,
            drain,
            destination,
        }
    }

    /// Lets the power go (the original's end event).
    pub fn release(&mut self) {
        self.base.released = true;
    }

    /// One control tick. At phase 0x10 the grid's answer decides.
    pub fn control(&mut self, event: i32, now: u32, nav: NavAnswer) -> VanishStep {
        if event == 2 {
            self.base.released = true;
        }
        if self.base.phase == AI_PICK {
            match nav {
                NavAnswer::Waiting => return VanishStep::quiet(1.0),
                NavAnswer::Impossible => {
                    return VanishStep {
                        result: DONE,
                        ..VanishStep::quiet(1.0)
                    };
                }
                NavAnswer::Found => {
                    self.base.phase = HIDDEN;
                    self.base.deadline = now.wrapping_add(FADE_IN_MS);
                    return VanishStep::quiet(1.0);
                }
            }
        }
        step_vanish(&mut self.base, now, self.drain)
    }
}

/// Constant Displacement: the persistent partly-intangible state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstantDisplacement {
    pub state: i32,
    pub held: f32,
    /// The shell's shown value, from 0.12 to 1.
    pub shown: f32,
}

/// What a Constant Displacement tick did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstantStep {
    pub result: i32,
    pub released: bool,
    /// The first tick's cast motion was taken.
    pub motion: bool,
    /// The shell is marked closing (its flags gain 0x300).
    pub closing: bool,
}

impl ConstantDisplacement {
    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
            shown: 0.0,
        }
    }

    /// The shell's shown value grows 0.002 a tick while it is under 0.12.
    pub fn grow(&mut self) {
        if self.shown < CONSTANT_FULL {
            self.shown += CONSTANT_GROW;
        }
    }

    /// One control tick.
    pub fn control(&mut self, event: i32, body: &mut CastBody) -> ConstantStep {
        let quiet = ConstantStep {
            result: 0,
            released: false,
            motion: false,
            closing: false,
        };
        if event == 2 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            let released = 0.0 < self.held;
            if released {
                PowerBase {
                    id: CONSTANT_DISPLACEMENT,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            return ConstantStep {
                result: 2,
                released,
                closing: true,
                ..quiet
            };
        }
        if self.state == super::READY {
            let motion = start_cast(body, CONSTANT_MOTION, CONSTANT_DISPLACEMENT);
            self.state = super::RUNNING;
            return ConstantStep { motion, ..quiet };
        }
        quiet
    }
}

impl Default for ConstantDisplacement {
    fn default() -> Self {
        Self::new()
    }
}

/// The Summon's phases.
pub const SUMMON_PICK: i32 = 0x10;
/// The demon is spawned.
pub const SUMMON_SPAWN: i32 = 0x11;
/// The effect grows.
pub const SUMMON_GROW: i32 = 3;
/// The effect fades.
pub const SUMMON_FADE: i32 = 4;
/// The tail before cleanup.
pub const SUMMON_TAIL: i32 = 5;

/// What a summon tick asks the game to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummonStep {
    pub result: i32,
    /// The demon's body is created.
    pub spawn: bool,
    /// The effect's phase changed.
    pub phase: i32,
    /// The effect is dropped.
    pub cleanup: bool,
}

/// The Summon: one demon brought in through the vanish effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summon {
    pub phase: i32,
    pub deadline: u32,
}

impl Summon {
    /// A summon beginning at `now`; an AI first picks its place.
    pub fn new(now: u32, ai: bool) -> Self {
        Self {
            phase: if ai { SUMMON_PICK } else { SUMMON_SPAWN },
            deadline: now,
        }
    }

    /// One control tick. Event 2 ends the summon at once.
    pub fn control(&mut self, event: i32, now: u32, nav: NavAnswer, created: bool) -> SummonStep {
        let quiet = SummonStep {
            result: 0,
            spawn: false,
            phase: self.phase,
            cleanup: false,
        };
        if event == 2 {
            return SummonStep {
                result: DONE,
                cleanup: true,
                ..quiet
            };
        }
        match self.phase {
            SUMMON_PICK => {
                if nav == NavAnswer::Impossible {
                    self.phase = DONE;
                    return SummonStep {
                        result: DONE,
                        cleanup: true,
                        ..quiet
                    };
                }
                if nav == NavAnswer::Found {
                    self.phase = SUMMON_SPAWN;
                    return SummonStep {
                        spawn: true,
                        phase: self.phase,
                        ..quiet
                    };
                }
                quiet
            }
            SUMMON_SPAWN => {
                if created {
                    self.phase = SUMMON_GROW;
                    self.deadline = now.wrapping_add(SUMMON_GROW_MS);
                    return SummonStep {
                        phase: self.phase,
                        ..quiet
                    };
                }
                quiet
            }
            SUMMON_GROW => {
                if self.deadline <= now {
                    self.phase = SUMMON_FADE;
                    self.deadline = now.wrapping_add(SUMMON_FADE_MS);
                    return SummonStep {
                        phase: self.phase,
                        ..quiet
                    };
                }
                quiet
            }
            SUMMON_FADE => {
                if self.deadline <= now {
                    self.phase = SUMMON_TAIL;
                    self.deadline = now.wrapping_add(SUMMON_TAIL_MS);
                    return SummonStep {
                        phase: self.phase,
                        ..quiet
                    };
                }
                quiet
            }
            SUMMON_TAIL => {
                if self.deadline <= now {
                    self.phase = DONE;
                    return SummonStep {
                        result: DONE,
                        cleanup: true,
                        ..quiet
                    };
                }
                quiet
            }
            _ => quiet,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vanish_fades_out_holds_and_comes_back() {
        let mut vanish = Vanish::new(0, false, false, 0.001);
        let hide = vanish.control(0, 0);
        assert!(hide.hide);
        assert_eq!(vanish.base.phase, FADING_OUT);
        let half = vanish.control(0, 150);
        assert!((half.shown - 0.5).abs() < 1e-6);
        assert_eq!(half.spend, 0.001);
        let out = vanish.control(0, 300);
        assert_eq!(out.shown, 0.0);
        assert!(out.drop_shell);
        assert_eq!(vanish.base.phase, HOLDING);

        // Held: half the tick cost.
        let held = vanish.control(0, 700);
        assert!((held.spend - 0.0005).abs() < 1e-9);

        vanish.release();
        let moved = vanish.control(0, 701);
        assert!(moved.moved);
        assert_eq!(vanish.base.phase, FADING_IN);
        let rising = vanish.control(0, 901);
        assert!((rising.shown - 0.5).abs() < 1e-6);
        let shown = vanish.control(0, 1101);
        assert!(shown.show);
        assert_eq!(vanish.base.phase, TAIL);
        assert_eq!(vanish.control(0, 1401).result, DONE);
    }

    #[test]
    fn a_long_door_waits_for_the_grid_and_moves() {
        let mut door = LongDoor::new(0, None, true, 0.002);
        assert_eq!(door.base.phase, AI_PICK);
        assert_eq!(door.control(0, 10, NavAnswer::Waiting).result, 0);
        let found = door.control(0, 20, NavAnswer::Found);
        assert_eq!(found.result, 0);
        assert_eq!(door.base.phase, HIDDEN);
        let hide = door.control(0, 21, NavAnswer::Found);
        assert!(hide.hide);
        door.control(0, 400, NavAnswer::Found);
        door.release();
        let moved = door.control(0, 401, NavAnswer::Found);
        assert!(moved.moved);
        let impossible = LongDoor::new(0, None, true, 0.002).control(0, 1, NavAnswer::Impossible);
        assert_eq!(impossible.result, DONE);
    }

    #[test]
    fn constant_displacement_grows_its_shell_and_closes() {
        let mut body = CastBody::default();
        let mut power = ConstantDisplacement::new();
        power.held = 0.35;
        body.spent = 0.35;
        for _ in 0..100 {
            power.grow();
        }
        assert!(power.shown > CONSTANT_FULL && power.shown < CONSTANT_FULL + CONSTANT_GROW + 1e-6);
        assert!(power.control(0, &mut body).motion);
        let end = power.control(2, &mut body);
        assert_eq!(end.result, 2);
        assert!(end.released && end.closing);
        assert_eq!(body.spent, 0.0);
    }

    #[test]
    fn a_summon_runs_its_five_phases() {
        let mut summon = Summon::new(0, true);
        assert_eq!(summon.control(0, 1, NavAnswer::Waiting, false).result, 0);
        assert!(summon.control(0, 2, NavAnswer::Found, false).spawn);
        assert_eq!(summon.phase, SUMMON_SPAWN);
        assert_eq!(
            summon.control(0, 3, NavAnswer::Found, false).phase,
            SUMMON_SPAWN
        );
        assert_eq!(
            summon.control(0, 4, NavAnswer::Found, true).phase,
            SUMMON_GROW
        );
        assert_eq!(
            summon.control(0, 3003, NavAnswer::Found, true).phase,
            SUMMON_GROW
        );
        assert_eq!(
            summon.control(0, 3004, NavAnswer::Found, true).phase,
            SUMMON_FADE
        );
        assert_eq!(
            summon.control(0, 4004, NavAnswer::Found, true).phase,
            SUMMON_TAIL
        );
        let done = summon.control(0, 5005, NavAnswer::Found, true);
        assert_eq!(done.result, DONE);
        assert!(done.cleanup);

        let mut player = Summon::new(0, false);
        assert_eq!(player.phase, SUMMON_SPAWN);
        assert_eq!(player.control(2, 0, NavAnswer::Found, true).result, DONE);
    }
}
