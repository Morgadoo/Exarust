//! The Force powers' active classes: the shell that halves a blow's energy,
//! the bubble and the barricade that end when their caster walks 20 m away,
//! Force Strikes with its weapon auras and combat factors, Kinesis with its
//! steering, and Throw with its lift and launch. What each cast creates is
//! [`super::super::force`]; these are the classes that run afterwards.
//!
//! - **Shell** ([`Shell`]): activation sets the body's hit damping to one
//!   half (the body's hit damping). The first control tick starts cast motion 0x24 and
//!   registers the hit-ripple hook; ending clears the damping.
//! - **Bubble and barricade** ([`RemotePower`]): a control event carrying
//!   bit 1 or 3 ends the effect; otherwise, once the effect's centre is more
//!   than 20 m from the caster, the power marks itself ending.
//! - **Strikes** ([`Strikes`]): activation puts an aura on every held item,
//!   sets the attacker's strike factor to 1.0 and its swing factor to 3.0;
//!   ending removes the auras and clears both. A weapon put away loses its
//!   aura; a weapon taken gains one while fewer than four carry auras, and a
//!   swing of an aura-carrying weapon reports the contact.
//! - **Kinesis** ([`Kinesis`]): a sustained power that starts cast motion
//!   0x23, installs the pick callbacks and, each tick, spends its cost and
//!   steers the held body toward the aim. The aim's velocity is the unit
//!   direction times `1/dist` (0.02 within 50 cm) times the body's reach
//!   (half its radius plus one); the turn keys rotate it by
//!   `reach² · 0.001` a tick; a held body remembers its user for 2 s.
//! - **Throw** ([`Throw`]): lifts its target (cast motion 0x23), points it
//!   at the aim and launches it at `mass/(mass + 0.1)` of the direction (a
//!   twentieth of the direction within 50 cm), dropping the shell after
//!   800 ms and restoring the body's materials 200 ms later.
//!
//! Evidence: the Force classes' control handlers, the Strike callbacks, the
//! Kinesis steering and the Throw phases, read from `Exanima.exe`.

use super::PowerBase;
use crate::thaum::casting::{CastBody, start_cast};

/// The Force power ids the classes run.
pub const SHELL: u32 = 0x20208;
pub const BUBBLE: u32 = 0x20207;
pub const BARRICADE: u32 = 0x20206;
pub const STRIKES: u32 = 0x2020B;
pub const KINESIS: u32 = 0x10209;
pub const THROW: u32 = 0x20A;

/// The cast motions the classes start.
pub const SHELL_MOTION: u32 = 0x24;
pub const BUBBLE_MOTION: u32 = 0x24;
pub const BARRICADE_MOTION: u32 = 0x24;
pub const STRIKES_MOTION: u32 = 0x25;
pub const KINESIS_MOTION: u32 = 0x23;
pub const THROW_MOTION: u32 = 0x23;

/// The squared distance at which a bubble or a barricade ends (20 m, cm²).
pub const REMOTE_RANGE_SQ: f32 = 4.0e6;
/// The hit damping a shell gives its body.
pub const SHELL_DAMPING: f32 = 0.5;
/// The attacker's strike factor while Force Strikes runs.
pub const STRIKES_FACTOR: f32 = 1.0;
/// The attacker's swing factor while Force Strikes runs.
pub const STRIKES_SWING_FACTOR: f32 = 3.0;
/// The swing factor a body has without Force Strikes.
pub const PLAIN_SWING_FACTOR: f32 = 1.0;
/// How many weapons can carry a strike aura.
pub const MOST_AURAS: usize = 4;
/// A kinesis object's reach: half its radius plus one.
pub const KINESIS_REACH_BIAS: f32 = 1.0;
/// The squared distance under which a kinesis or throw direction is not
/// unit-scaled.
pub const KINESIS_CLOSE_SQ: f32 = 2500.0;
/// The small factor a close direction is scaled by.
pub const CLOSE_FACTOR: f32 = 0.02;
/// The turn a movement key gives a kinesis object, per unit of reach².
pub const KINESIS_TURN: f32 = 0.001;
/// The spin a kinesis gives the held body.
pub const KINESIS_SPIN: f32 = -0.2;
/// How long a held body remembers its user, in milliseconds.
pub const HOLD_MS: u32 = 2_000;
/// The throw's shell-drop delay, in milliseconds.
pub const THROW_DROP_MS: u32 = 800;
/// The throw's material-restore delay, in milliseconds.
pub const THROW_RESTORE_MS: u32 = 200;
/// The throw's minimum distance before it is let go, in cm.
pub const THROW_DROP_DIST: f32 = 100.0;
/// The throw's mass denominator bias.
pub const THROW_MASS_BIAS: f64 = 0.1;
/// The throw's speed scale after the lift.
pub const THROW_FLY_SCALE: f32 = 6.0;
/// The throw's lift scale.
pub const THROW_LIFT_SCALE: f32 = 0.00125;

/// The shell's answer for one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellStep {
    pub result: i32,
    pub released: bool,
    /// The first tick's cast motion was taken.
    pub motion: bool,
}

/// Force Shell: sets the body's hit damping and clears it at the end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shell {
    pub state: i32,
    pub held: f32,
}

impl Shell {
    /// The HUD word of a running shell: always the active word.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }

    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
        }
    }

    /// The activation handler: the body keeps half a blow's energy while
    /// the shell runs.
    pub fn apply(&self, body: &mut CastBody) {
        body.hit_damping = SHELL_DAMPING;
    }

    /// One control tick.
    pub fn control(&mut self, event: i32, body: &mut CastBody) -> ShellStep {
        let quiet = ShellStep {
            result: 0,
            released: false,
            motion: false,
        };
        if event == 2 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            let released = 0.0 < self.held;
            if released {
                PowerBase {
                    id: SHELL,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            body.hit_damping = 0.0;
            ShellStep {
                result: 2,
                released,
                motion: false,
            }
        } else if self.state == super::READY {
            let motion = start_cast(body, SHELL_MOTION, SHELL);
            self.state = super::RUNNING;
            ShellStep { motion, ..quiet }
        } else {
            quiet
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

/// A bubble or a barricade: its state, its held focus and the effect's
/// centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RemotePower {
    pub state: i32,
    pub held: f32,
    pub at: [f32; 3],
}

impl RemotePower {
    /// The HUD word of a running bubble or barricade.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }

    /// A power whose effect sits at `at`.
    pub fn new(at: [f32; 3]) -> Self {
        Self {
            state: super::RUNNING,
            held: 0.0,
            at,
        }
    }
}

/// One remote power's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteStep {
    pub result: i32,
    pub released: bool,
}

impl RemotePower {
    /// One control tick. An event carrying bit 1 or 3 ends the power.
    pub fn control(&mut self, event: i32, caster: [f32; 3], body: &mut CastBody) -> RemoteStep {
        if event & 0xa != 0 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            let released = 0.0 < self.held;
            if released {
                PowerBase {
                    id: 0,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            return RemoteStep {
                result: 2,
                released,
            };
        }
        let delta = [
            self.at[0] - caster[0],
            self.at[1] - caster[1],
            self.at[2] - caster[2],
        ];
        let dist_sq = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
        if REMOTE_RANGE_SQ < dist_sq {
            self.state = super::ENDING;
        }
        RemoteStep {
            result: 0,
            released: false,
        }
    }
}

/// One weapon aura: the item and the body holding it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aura {
    pub item: u64,
    pub body: u64,
}

/// Force Strikes and its auras.
#[derive(Debug, Clone, PartialEq)]
pub struct Strikes {
    pub state: i32,
    pub held: f32,
    pub auras: Vec<Aura>,
}

impl Strikes {
    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
            auras: Vec::new(),
        }
    }

    /// The activation handler: an aura on every held item and the two
    /// combat factors.
    pub fn apply(&mut self, body: &mut CastBody, items: &[(u64, u64)]) {
        self.auras = items
            .iter()
            .map(|&(item, owner)| Aura { item, body: owner })
            .collect();
        body.strike_factor = STRIKES_FACTOR;
        body.swing_factor = STRIKES_SWING_FACTOR;
    }

    /// One control tick.
    pub fn control(&mut self, event: i32, body: &mut CastBody) -> ShellStep {
        let quiet = ShellStep {
            result: 0,
            released: false,
            motion: false,
        };
        if event == 2 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            let released = 0.0 < self.held;
            if released {
                PowerBase {
                    id: STRIKES,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            self.auras.clear();
            body.swing_factor = PLAIN_SWING_FACTOR;
            body.strike_factor = 0.0;
            ShellStep {
                result: 2,
                released,
                motion: false,
            }
        } else if self.state == super::READY {
            let motion = start_cast(body, STRIKES_MOTION, STRIKES);
            self.state = super::RUNNING;
            ShellStep { motion, ..quiet }
        } else {
            quiet
        }
    }

    /// A weapon put away (false) loses its aura; a weapon taken (true)
    /// gains one while fewer than four carry auras.
    pub fn weapon_change(&mut self, item: u64, owner: u64, taken: bool) -> bool {
        if !taken {
            if let Some(at) = self.auras.iter().position(|aura| aura.item == item) {
                self.auras.swap_remove(at);
            }
            return false;
        }
        if self.auras.len() < MOST_AURAS {
            self.auras.push(Aura { item, body: owner });
            return true;
        }
        false
    }

    /// Whether a swing's weapon is one this power has auras on.
    pub fn swung(&self, item: u64) -> bool {
        self.auras.iter().any(|aura| aura.item == item)
    }
}

impl Default for Strikes {
    fn default() -> Self {
        Self::new()
    }
}

/// Kinesis' one target: where its centre is and how large it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KinesisTarget {
    pub position: [f32; 3],
    /// The held body's largest radius, in cm.
    pub radius: f32,
}

/// Kinesis' control answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KinesisStep {
    pub result: i32,
    /// The first tick's cast motion was taken.
    pub motion: bool,
    /// The pick callbacks are installed.
    pub pick: bool,
    /// The tick spends its focus.
    pub drain: bool,
}

/// Kinesis: a sustained power that steers what its pick holds.
#[derive(Debug, Clone, PartialEq)]
pub struct Kinesis {
    pub state: i32,
    pub held: f32,
    pub target: Option<u64>,
}

impl Kinesis {
    /// The HUD word of a running kinesis.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }

    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
            target: None,
        }
    }

    /// One control tick. `tick_cost` is what a running tick spends.
    pub fn control(&mut self, event: i32, body: &mut CastBody, tick_cost: f32) -> KinesisStep {
        if event & 0xa != 0 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            self.target = None;
            return KinesisStep {
                result: 2,
                motion: false,
                pick: false,
                drain: false,
            };
        }
        if self.state == super::READY {
            let motion = start_cast(body, KINESIS_MOTION, KINESIS);
            self.state = super::RUNNING;
            return KinesisStep {
                result: 0,
                motion,
                pick: true,
                drain: false,
            };
        }
        super::drain(body, tick_cost);
        KinesisStep {
            result: 0,
            motion: false,
            pick: false,
            drain: true,
        }
    }
}

impl Default for Kinesis {
    fn default() -> Self {
        Self::new()
    }
}

/// A kinesis object's reach: half its largest radius plus one.
pub fn kinesis_reach(radius: f32) -> f32 {
    radius * 0.5 + KINESIS_REACH_BIAS
}

/// The velocity a kinesis gives its target toward `aim`: the unit direction
/// times the reach, or the direction times a fiftieth within 50 cm.
pub fn kinesis_velocity(target: &KinesisTarget, aim: [f32; 3]) -> [f32; 3] {
    let delta = [
        aim[0] - target.position[0],
        aim[1] - target.position[1],
        aim[2] - target.position[2],
    ];
    let dist_sq = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
    let factor = if dist_sq <= KINESIS_CLOSE_SQ {
        CLOSE_FACTOR
    } else {
        1.0 / dist_sq.sqrt()
    };
    let scale = factor * kinesis_reach(target.radius);
    [delta[0] * scale, delta[1] * scale, delta[2] * scale]
}

/// The two turn speeds the movement keys give a kinesis object, along x and
/// y. Bits 1 and 4 turn around one axis, bits 2 and 8 around the other.
pub fn kinesis_turn(keys: u32, radius: f32) -> [f32; 2] {
    let step = kinesis_reach(radius) * kinesis_reach(radius) * KINESIS_TURN;
    let mut x = 0.0;
    let mut y = 0.0;
    if keys & 1 != 0 {
        x += step;
    }
    if keys & 4 != 0 {
        x -= step;
    }
    if keys & 2 != 0 {
        y += step;
    }
    if keys & 8 != 0 {
        y -= step;
    }
    [x, y]
}

/// The spin a kinesis gives the held body from a steering direction.
pub fn kinesis_spin(direction: [f32; 3]) -> [f32; 3] {
    [
        direction[0] * KINESIS_SPIN,
        direction[1] * KINESIS_SPIN,
        direction[2] * KINESIS_SPIN,
    ]
}

/// When a held body's user stamp runs out.
pub fn hold_until(now: u32) -> u32 {
    now.wrapping_add(HOLD_MS)
}

/// What a kinesis pick puts on the body it grabs: the force shell's speed
/// scale, the special physics material and the 2 s user stamp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KinesisPick {
    pub item: u64,
    /// The shell's scale from the item's speed.
    pub shell_scale: f32,
    /// The body remembers its user until then.
    pub hold_until: u32,
    /// The body takes the special kinesis physics material.
    pub material: bool,
}

/// The pick's shell is the force shell scaled by `(1/speed + 0.015)·0.5`.
pub fn kinesis_pick(item: u64, speed: f32, now: u32) -> KinesisPick {
    KinesisPick {
        item,
        shell_scale: super::super::kinesis_scale(speed),
        hold_until: hold_until(now),
        material: true,
    }
}

/// Throw's phases.
pub const THROW_LIFT: i32 = 0;
/// Flying under its own momentum.
pub const THROW_FLY: i32 = 1;
/// Dropped, waiting to restore its materials.
pub const THROW_FALL: i32 = 2;
/// Done.
pub const THROW_DONE: i32 = 3;

/// Throw: lift, launch and let go of one body.
#[derive(Debug, Clone, PartialEq)]
pub struct Throw {
    pub state: i32,
    pub held: f32,
    pub phase: i32,
    pub stamp: u32,
    /// The lift's strength.
    pub lift: f32,
}

impl Throw {
    pub fn new(lift: f32) -> Self {
        Self {
            state: super::READY,
            held: 0.0,
            phase: THROW_LIFT,
            stamp: 0,
            lift,
        }
    }
}

/// What a throw's tick asks the game to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThrowStep {
    pub result: i32,
    pub released: bool,
    /// The cast motion was taken.
    pub motion: bool,
    /// The lift's shell and sound start now.
    pub lift: bool,
    /// The body is launched along its aim.
    pub launch: bool,
    /// The shell drops and the body coasts.
    pub coast: bool,
    /// The body's materials come back.
    pub restore: bool,
}

impl Throw {
    /// One control tick. `close` is whether the target is within 100 cm of
    /// its aim (the original lets go once it is that near).
    pub fn control(&mut self, event: i32, now: u32, close: bool, body: &mut CastBody) -> ThrowStep {
        let quiet = ThrowStep {
            result: 0,
            released: false,
            motion: false,
            lift: false,
            launch: false,
            coast: false,
            restore: false,
        };
        if event & 0xa != 0 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            let released = 0.0 < self.held;
            if released {
                PowerBase {
                    id: THROW,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            return ThrowStep {
                result: 2,
                released,
                restore: true,
                ..quiet
            };
        }
        if self.state == super::READY {
            let motion = start_cast(body, THROW_MOTION, THROW);
            let lift = self.phase == THROW_LIFT;
            if lift {
                self.phase = THROW_FLY;
            }
            self.stamp = now;
            self.state = super::RUNNING;
            return ThrowStep {
                motion,
                lift,
                ..quiet
            };
        }
        // The launch runs from the lift's start; the phase machine counts
        // from each stamp.
        let elapsed = now.wrapping_sub(self.stamp);
        if self.phase == THROW_FLY {
            if elapsed > THROW_DROP_MS {
                self.phase = THROW_FALL;
                self.stamp = now;
                if close {
                    self.state = super::ENDING;
                    self.phase = THROW_DONE;
                }
                return ThrowStep {
                    launch: true,
                    ..quiet
                };
            }
            return quiet;
        }
        if self.phase == THROW_FALL && elapsed > THROW_RESTORE_MS {
            self.phase = THROW_DONE;
            self.state = super::ENDING;
            return ThrowStep {
                restore: true,
                ..quiet
            };
        }
        if self.phase == THROW_DONE {
            self.state = super::ENDING;
        }
        quiet
    }

    /// The lift's strength from the caster's record word and the cast's
    /// strength.
    pub fn lift_strength(record_word: u32, strength: f32) -> f32 {
        (record_word as f32) * strength * THROW_LIFT_SCALE
    }
}

/// A launched body's velocity and whether the aim is already close enough
/// to let go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThrowLaunch {
    pub velocity: [f32; 3],
    pub close: bool,
}

/// The velocity a throw gives its body: the unit direction toward the aim,
/// scaled by the mass's share `mass/(mass + 0.1)` (a fiftieth of the
/// direction within 50 cm).
pub fn throw_launch(from: [f32; 3], to: [f32; 3], mass: f32) -> ThrowLaunch {
    let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let dist_sq = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
    let factor = if dist_sq <= KINESIS_CLOSE_SQ {
        CLOSE_FACTOR
    } else {
        1.0 / dist_sq.sqrt()
    };
    let mass_share = (f64::from(mass) / (f64::from(mass) + THROW_MASS_BIAS)) as f32;
    let scale = factor * mass_share;
    let distance = dist_sq.sqrt();
    ThrowLaunch {
        velocity: [delta[0] * scale, delta[1] * scale, delta[2] * scale],
        close: distance < THROW_DROP_DIST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_halves_the_hit_and_clears_it() {
        let mut body = CastBody::default();
        let mut shell = Shell::new();
        shell.held = 0.35;
        body.spent = 0.35;
        shell.apply(&mut body);
        assert_eq!(body.hit_damping, 0.5);
        let first = shell.control(0, &mut body);
        assert!(first.motion);
        assert_eq!(shell.state, super::super::RUNNING);
        let end = shell.control(2, &mut body);
        assert_eq!(end.result, 2);
        assert!(end.released);
        assert_eq!(body.hit_damping, 0.0);
        assert_eq!(body.spent, 0.0);
    }

    #[test]
    fn a_dome_ends_beyond_twenty_metres() {
        let mut body = CastBody::default();
        let mut bubble = RemotePower::new([0.0, 0.0, 0.0]);
        assert_eq!(bubble.control(0, [2000.0, 0.0, 0.0], &mut body).result, 0);
        assert_eq!(bubble.state, super::super::RUNNING);
        bubble.control(0, [2000.1, 0.0, 0.0], &mut body);
        assert_eq!(bubble.state, super::super::ENDING);
        assert_eq!(bubble.control(0, [2000.1, 0.0, 0.0], &mut body).result, 2);
        let mut other = RemotePower::new([0.0; 3]);
        assert_eq!(other.control(0xa, [0.0; 3], &mut body).result, 2);
    }

    #[test]
    fn strikes_sets_its_factors_and_tracks_its_auras() {
        let mut body = CastBody::default();
        let mut strikes = Strikes::new();
        strikes.apply(&mut body, &[(11, 1), (12, 1)]);
        assert_eq!(body.strike_factor, 1.0);
        assert_eq!(body.swing_factor, 3.0);
        assert_eq!(strikes.auras.len(), 2);
        assert!(strikes.swung(11));
        assert!(!strikes.swung(13));

        assert!(!strikes.weapon_change(12, 1, false));
        assert_eq!(strikes.auras.len(), 1);
        assert!(strikes.weapon_change(13, 1, true));
        assert!(strikes.weapon_change(14, 1, true));
        assert!(strikes.weapon_change(15, 1, true));
        assert!(!strikes.weapon_change(16, 1, true));
        assert_eq!(strikes.auras.len(), 4);

        assert!(strikes.control(0, &mut body).motion);
        let end = strikes.control(2, &mut body);
        assert_eq!(end.result, 2);
        assert!(strikes.auras.is_empty());
        assert_eq!(body.strike_factor, 0.0);
        assert_eq!(body.swing_factor, 1.0);
    }

    #[test]
    fn kinesis_steers_by_its_reach() {
        assert_eq!(kinesis_reach(100.0), 51.0);
        let target = KinesisTarget {
            position: [0.0; 3],
            radius: 100.0,
        };
        let far = kinesis_velocity(&target, [100.0, 0.0, 0.0]);
        assert!((far[0] - 51.0).abs() < 1e-5);
        let near = kinesis_velocity(&target, [40.0, 0.0, 0.0]);
        assert!((near[0] - 40.0 * 0.02 * 51.0).abs() < 1e-3);
        assert_eq!(kinesis_turn(1, 100.0), [2601.0 * 0.001, 0.0]);
        assert_eq!(kinesis_turn(4, 100.0), [-2.601, 0.0]);
        assert_eq!(kinesis_turn(2, 100.0), [0.0, 2.601]);
        assert_eq!(kinesis_turn(8, 100.0), [0.0, -2.601]);
        assert_eq!(kinesis_spin([1.0, 2.0, 3.0]), [-0.2, -0.4, -0.6]);
        assert_eq!(hold_until(100), 2100);
        let pick = kinesis_pick(7, 2.0, 100);
        assert_eq!(pick.item, 7);
        assert!((pick.shell_scale - 0.2575).abs() < 1e-6);
        assert_eq!(pick.hold_until, 2100);
        assert!(pick.material);
    }

    #[test]
    fn kinesis_starts_once_then_drains_each_tick() {
        let mut body = CastBody::default();
        let mut kinesis = Kinesis::new();
        let first = kinesis.control(0, &mut body, 0.001);
        assert!(first.motion && first.pick && !first.drain);
        assert_eq!(kinesis.state, super::super::RUNNING);
        let tick = kinesis.control(0, &mut body, 0.001);
        assert!(tick.drain);
        assert!((body.spent - 0.001).abs() < 1e-9);
        let end = kinesis.control(0xa, &mut body, 0.001);
        assert_eq!(end.result, 2);
        assert_eq!(kinesis.target, None);
    }

    #[test]
    fn a_throw_lifts_launches_and_restores() {
        let launch = throw_launch([0.0; 3], [100.0, 0.0, 0.0], 0.9);
        assert!((launch.velocity[0] - 0.9).abs() < 1e-6);
        assert!(!launch.close);
        let near = throw_launch([0.0; 3], [40.0, 0.0, 0.0], 0.9);
        assert!((near.velocity[0] - 40.0 * 0.02 * 0.9).abs() < 1e-4);
        assert!(near.close);
        let just_short = throw_launch([0.0; 3], [99.0, 0.0, 0.0], 0.9);
        assert!((just_short.velocity[0] - 0.9).abs() < 1e-6);
        assert!(just_short.close);
        assert_eq!(Throw::lift_strength(100, 0.5), 0.0625);

        let mut body = CastBody::default();
        let mut power = Throw::new(1.0);
        let lift = power.control(0, 1000, false, &mut body);
        assert!(lift.motion && lift.lift);
        let flying = power.control(0, 1500, false, &mut body);
        assert!(!flying.launch);
        let drop = power.control(0, 1801, false, &mut body);
        assert!(drop.launch);
        assert_eq!(power.phase, THROW_FALL);
        let restore = power.control(0, 2002, false, &mut body);
        assert!(restore.restore);
        assert_eq!(power.state, super::super::ENDING);

        let mut ending = Throw::new(1.0);
        ending.held = 0.1;
        body.spent = 0.1;
        let end = ending.control(2, 0, false, &mut body);
        assert_eq!(end.result, 2);
        assert!(end.restore);
    }
}
