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
pub use crate::thaum::displacement::{
    CONSTANT as CONSTANT_DISPLACEMENT, LONG_DOOR, SUMMON, VANISH,
};

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

/// How long the effect waits, spending focus, before the body vanishes, in
/// milliseconds (on the body's scene clock).
pub const CHARGE_MS: u32 = 400;
/// How long the fade-out lasts, in milliseconds.
pub const FADE_OUT_MS: u32 = 300;
/// How long the move and the fade-in last, in milliseconds.
pub const FADE_IN_MS: u32 = 400;
/// How long the tail lasts, in milliseconds.
pub const TAIL_MS: u32 = 300;
/// How long the summon's effect waits before it starts to fade, in
/// milliseconds.
pub const SUMMON_WAIT_MS: u32 = 2000;
/// How long the summon's effect fades, in milliseconds.
pub const SUMMON_FADE_MS: u32 = 3000;
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
    /// How many fades have run (the fade counter).
    pub step: i32,
}

impl VanishBase {
    /// Starts the effect at the phase the original's activation leaves:
    /// phase 1 for a player, phase 0x10 for the AI.
    /// The power's own start sets the first deadline 400 ms on; the AI's
    /// fade counter is only set once its place is found.
    pub fn begin(now: u32, shared: bool, ai: bool) -> Self {
        Self {
            phase: if ai { AI_PICK } else { HIDDEN },
            deadline: now.wrapping_add(CHARGE_MS),
            released: false,
            shared,
            shown: 1.0,
            step: if ai { 0 } else { 1 },
        }
    }
}

/// One body image the vanish effect shows and hides: the shell's shown
/// value and its flags (bit 2 is the distortion's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanishPart {
    pub shown: f32,
    pub flags: u32,
    /// The after-image's shown value.
    pub trail: f32,
}

/// The flag a hidden distortion keeps off.
pub const PART_DISTORTION: u32 = 4;

impl VanishPart {
    /// Shows the part at full strength. The original also asks its draw
    /// routine for a 0x30-long fade (a guess at that argument's meaning).
    pub fn show(&mut self) {
        self.shown = 1.0;
    }

    /// Hides the part: its shown value and trail fall to 0 and its
    /// distortion flag clears.
    pub fn hide(&mut self) {
        self.flags &= !PART_DISTORTION;
        self.shown = 0.0;
        self.trail = 0.0;
    }
}

/// The demon the summon brings in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DemonSummon {
    pub record: &'static str,
    pub role: u32,
    pub relation: f32,
    pub topic: u32,
}

/// The demon's record name.
pub const DEMON_RECORD: &str = "demon a01";
/// The role it takes.
pub const DEMON_ROLE: u32 = 0x21;
/// The relation the two characters gain toward each other.
pub const DEMON_RELATION: f32 = 300.0;
/// The relation topic.
pub const DEMON_TOPIC: u32 = 0x1c;

/// The demon record the summon loads and the roles it is given.
pub fn demon_summon() -> DemonSummon {
    DemonSummon {
        record: DEMON_RECORD,
        role: DEMON_ROLE,
        relation: DEMON_RELATION,
        topic: DEMON_TOPIC,
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
    /// The displacement shell's state word: 2 when the body vanishes,
    /// 1 when it moves, 2 when it reappears with a shell of its own (a
    /// shared shell goes back to its Constant Displacement instead); 0
    /// leaves it alone.
    pub shell_state: i32,
    /// The cast hook and the cast id are cleared (the Long Door as the body
    /// reappears, Vanish when it is done and the hook is still its own).
    pub clear_hook: bool,
}

/// What ends the vanish effect's hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldEnd {
    /// Vanish: the player lets the power go; until then each tick spends
    /// half the power's tick cost.
    Release,
    /// The Long Door: the hold's 400 ms deadline, spending nothing.
    Deadline,
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
            shell_state: 0,
            clear_hook: false,
        }
    }
}

/// Steps the shared vanish effect. Phase 1 waits for its deadline,
/// spending `drain` each tick, then hides the body (its vanished bit set)
/// and fades it out over 300 ms, still spending `drain`; the hold ends as
/// `hold` says and moves the body; the fade-in takes 400 ms and shows the
/// body again; the tail waits 300 ms and answers [`DONE`]. Every deadline
/// is compared unsigned with the 32-bit clock, and each fade is the double
/// `(deadline − now) / length`, narrowed (the fade-in takes it from 1 in
/// single).
pub fn step_vanish(base: &mut VanishBase, now: u32, drain: f32, hold: HoldEnd) -> VanishStep {
    match base.phase {
        HIDDEN => {
            if base.deadline > now {
                return VanishStep {
                    spend: drain,
                    ..VanishStep::quiet(base.shown)
                };
            }
            base.phase = FADING_OUT;
            base.deadline = now.wrapping_add(FADE_OUT_MS);
            VanishStep {
                hide: true,
                shell_state: 2,
                ..VanishStep::quiet(base.shown)
            }
        }
        FADING_OUT => {
            if base.deadline > now {
                let left = f64::from(base.deadline - now) * (1.0 / f64::from(FADE_OUT_MS));
                base.shown = left as f32;
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
            let moves = match hold {
                HoldEnd::Release => base.released,
                HoldEnd::Deadline => base.deadline <= now,
            };
            if !moves {
                let spend = match hold {
                    HoldEnd::Release => drain * 0.5,
                    HoldEnd::Deadline => 0.0,
                };
                return VanishStep {
                    spend,
                    ..VanishStep::quiet(base.shown)
                };
            }
            base.phase = FADING_IN;
            base.deadline = now.wrapping_add(FADE_IN_MS);
            base.step = base.step.wrapping_add(1);
            VanishStep {
                moved: true,
                shell_state: 1,
                ..VanishStep::quiet(base.shown)
            }
        }
        FADING_IN => {
            if base.deadline > now {
                let left = f64::from(base.deadline - now) * (1.0 / f64::from(FADE_IN_MS));
                base.shown = 1.0 - left as f32;
                VanishStep::quiet(base.shown)
            } else {
                // The body's visibility keeps the fade's last value; the
                // show routine brings it back.
                base.phase = TAIL;
                base.deadline = now.wrapping_add(TAIL_MS);
                VanishStep {
                    show: true,
                    drop_shell: !base.shared,
                    shell_state: if base.shared { 0 } else { 2 },
                    clear_hook: hold == HoldEnd::Deadline,
                    ..VanishStep::quiet(base.shown)
                }
            }
        }
        TAIL => {
            if base.deadline <= now {
                VanishStep {
                    result: DONE,
                    clear_hook: hold == HoldEnd::Release,
                    ..VanishStep::quiet(base.shown)
                }
            } else {
                VanishStep::quiet(base.shown)
            }
        }
        _ => VanishStep::quiet(base.shown),
    }
}

/// One tick of Vanish.
#[derive(Debug, Clone, PartialEq)]
pub struct Vanish {
    pub base: VanishBase,
    /// The focus a vanished tick spends.
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

    /// One control tick. Event 2 lets the power go.
    pub fn control(&mut self, event: i32, now: u32) -> VanishStep {
        if event == 2 {
            self.base.released = true;
        }
        step_vanish(&mut self.base, now, self.drain, HoldEnd::Release)
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
        Self {
            base: VanishBase::begin(now, false, ai),
            drain,
            destination,
        }
    }

    /// One control tick. The Long Door has no end event: its hold ends on
    /// its deadline. At phase 0x10 the grid's answer decides: a found
    /// place starts the wait 400 ms on with the fade counter at 1 and the
    /// shell moving; an impossible one clears the cast hook and is done.
    pub fn control(&mut self, now: u32, nav: NavAnswer) -> VanishStep {
        if self.base.phase == AI_PICK {
            match nav {
                NavAnswer::Waiting => return VanishStep::quiet(1.0),
                NavAnswer::Impossible => {
                    return VanishStep {
                        result: DONE,
                        clear_hook: true,
                        ..VanishStep::quiet(1.0)
                    };
                }
                NavAnswer::Found => {
                    self.base.phase = HIDDEN;
                    self.base.deadline = now.wrapping_add(CHARGE_MS);
                    self.base.step = 1;
                    return VanishStep {
                        shell_state: 1,
                        ..VanishStep::quiet(1.0)
                    };
                }
            }
        }
        step_vanish(&mut self.base, now, self.drain, HoldEnd::Deadline)
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
    /// The HUD word of a running power.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }

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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SummonStep {
    pub result: i32,
    /// The demon's body is created.
    pub spawn: bool,
    /// The effect's phase changed.
    pub phase: i32,
    /// The effect is dropped.
    pub cleanup: bool,
    /// The caster's visibility while the effect fades (phase 4):
    /// `1 − (deadline − now)/3000`, the share in double narrowed first.
    pub shown: Option<f32>,
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
            shown: None,
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
                    self.deadline = now.wrapping_add(SUMMON_WAIT_MS);
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
                if self.deadline > now {
                    let left = f64::from(self.deadline - now) * (1.0 / f64::from(SUMMON_FADE_MS));
                    return SummonStep {
                        shown: Some(1.0 - left as f32),
                        ..quiet
                    };
                }
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
    fn a_vanish_waits_fades_out_holds_and_comes_back() {
        let mut vanish = Vanish::new(0, false, false, 0.001);
        // 400 ms of charge, spending the tick cost, before the body goes.
        let wait = vanish.control(0, 399);
        assert!(!wait.hide);
        assert_eq!(wait.spend, 0.001);
        let hide = vanish.control(0, 400);
        assert!(hide.hide);
        assert_eq!(hide.shell_state, 2);
        assert_eq!(vanish.base.phase, FADING_OUT);
        let half = vanish.control(0, 550);
        assert!((half.shown - 0.5).abs() < 1e-6);
        assert_eq!(half.spend, 0.001);
        let out = vanish.control(0, 700);
        assert_eq!(out.shown, 0.0);
        assert!(out.drop_shell);
        assert_eq!(vanish.base.phase, HOLDING);

        // Held: half the tick cost.
        let held = vanish.control(0, 1100);
        assert!((held.spend - 0.0005).abs() < 1e-9);

        let moved = vanish.control(2, 1101);
        assert!(moved.moved);
        assert_eq!(moved.shell_state, 1);
        assert_eq!(vanish.base.phase, FADING_IN);
        assert_eq!(vanish.base.step, 2);
        let rising = vanish.control(0, 1201);
        // 1 − (300/400 in double, narrowed), in single.
        assert_eq!(rising.shown, 1.0 - 0.75f32);
        let shown = vanish.control(0, 1501);
        assert!(shown.show && shown.drop_shell);
        assert_eq!(shown.shell_state, 2);
        assert!(!shown.clear_hook);
        assert_eq!(vanish.base.phase, TAIL);
        let done = vanish.control(0, 1801);
        assert_eq!(done.result, DONE);
        assert!(done.clear_hook);
    }

    #[test]
    fn a_shared_shell_goes_back_to_its_constant_displacement() {
        let mut vanish = Vanish::new(0, true, false, 0.0);
        vanish.base.phase = FADING_IN;
        vanish.base.deadline = 0;
        let back = vanish.control(0, 1);
        assert!(back.show);
        assert!(!back.drop_shell);
        assert_eq!(back.shell_state, 0);
    }

    #[test]
    fn a_long_door_waits_for_the_grid_and_moves_on_its_deadline() {
        let mut door = LongDoor::new(0, None, true, 0.002);
        assert_eq!(door.base.phase, AI_PICK);
        assert_eq!(door.base.step, 0);
        assert_eq!(door.control(10, NavAnswer::Waiting).result, 0);
        let found = door.control(20, NavAnswer::Found);
        assert_eq!(found.result, 0);
        assert_eq!(found.shell_state, 1);
        assert_eq!(door.base.phase, HIDDEN);
        assert_eq!(door.base.step, 1);
        assert_eq!(door.control(419, NavAnswer::Found).spend, 0.002);
        assert!(door.control(420, NavAnswer::Found).hide);
        assert!(door.control(720, NavAnswer::Found).drop_shell);
        // The hold spends nothing and ends 400 ms on by itself.
        let held = door.control(1119, NavAnswer::Found);
        assert!(!held.moved);
        assert_eq!(held.spend, 0.0);
        assert!(door.control(1120, NavAnswer::Found).moved);
        // The cast hook is cleared as the body reappears.
        assert!(door.control(1520, NavAnswer::Found).clear_hook);
        let impossible = LongDoor::new(0, None, true, 0.002).control(1, NavAnswer::Impossible);
        assert_eq!(impossible.result, DONE);
        assert!(impossible.clear_hook);
        assert_eq!(LongDoor::new(0, None, false, 0.0).base.phase, HIDDEN);
    }

    #[test]
    fn constant_displacement_grows_its_shell_and_closes() {
        let mut body = CastBody::default();
        let mut power = ConstantDisplacement::new();
        power.held = 0.35;
        body.reserved = 0.35;
        for _ in 0..100 {
            power.grow();
        }
        assert!(power.shown > CONSTANT_FULL && power.shown < CONSTANT_FULL + CONSTANT_GROW + 1e-6);
        assert!(power.control(0, &mut body).motion);
        let end = power.control(2, &mut body);
        assert_eq!(end.result, 2);
        assert!(end.released && end.closing);
        assert_eq!(body.reserved, 0.0);
    }

    #[test]
    fn vanish_parts_show_and_hide() {
        let mut part = VanishPart {
            shown: 0.0,
            flags: PART_DISTORTION,
            trail: 0.0,
        };
        part.show();
        assert_eq!(part.shown, 1.0);
        assert_eq!(part.flags, PART_DISTORTION);
        part.hide();
        assert_eq!(part.shown, 0.0);
        assert_eq!(part.flags, 0);
    }

    #[test]
    fn the_demon_carries_its_roles() {
        let demon = demon_summon();
        assert_eq!(demon.record, "demon a01");
        assert_eq!(demon.role, 0x21);
        assert_eq!(demon.relation, 300.0);
        assert_eq!(demon.topic, 0x1c);
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
            summon.control(0, 2003, NavAnswer::Found, true).phase,
            SUMMON_GROW
        );
        assert_eq!(
            summon.control(0, 2004, NavAnswer::Found, true).phase,
            SUMMON_FADE
        );
        assert_eq!(
            summon.control(0, 3504, NavAnswer::Found, true).shown,
            Some(0.5)
        );
        assert_eq!(
            summon.control(0, 5004, NavAnswer::Found, true).phase,
            SUMMON_TAIL
        );
        let done = summon.control(0, 6005, NavAnswer::Found, true);
        assert_eq!(done.result, DONE);
        assert!(done.cleanup);

        let mut player = Summon::new(0, false);
        assert_eq!(player.phase, SUMMON_SPAWN);
        assert_eq!(player.control(2, 0, NavAnswer::Found, true).result, DONE);
    }
}
