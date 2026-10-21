//! The locomotion request a character's behaviour writes (the player's
//! input, an NPC's AI) and the movement decision its body tick makes from
//! it each tick, as the original's body tick makes it: the move vector
//! (free walk along the facing, or toward a goal point), the leg mode, the
//! brake within the gait's stop distance, the gates that let a body walk
//! (full energy, steady, not stepping), the stop once it is within 25 cm of
//! its goal at no more than 0.5 cm a tick, the body's turn toward its
//! heading by the original's table of rates, and the alignment of its
//! pelvis with its heading. Lengths are stored in metres; the rules are the
//! original's, in centimetres and centimetres a tick.
//!
//! Until the body walks by its carry, the character's capsule executes the
//! decision at Exarust's walking, running and backpedalling speeds
//! ([`capsule_velocity`]). Evidence: `docs/PLAYER.md` (Movement) and
//! `docs/formats/RSQ.md` (Physical body, Movement decision).

use bevy::math::Affine3A;
use bevy::prelude::*;

use super::balance::cm;
use super::{Drive, Ragdoll};

#[path = "body_gait.rs"]
mod gait;
pub use gait::{
    GaitRow, GaitTable, PHASE_UNSET, STOPS, Start, Stride, advance_clock, fraction, lean, locomote,
    stand,
};
#[path = "body_twist.rs"]
mod twist;
pub use twist::{
    TwistBy, acos_approx, asin_approx, euler_of, from_euler, head_look, heading_of, look_rotation,
    twist, twist_spine,
};

/// Bits of the request's flag word, the original's.
pub mod flags {
    /// The behaviour asks to move.
    pub const MOVE: u32 = 0x1;
    /// Backwards: the body backpedals.
    pub const BACK: u32 = 0x2;
    /// Run: the body runs, and never brakes for its goal.
    pub const RUN: u32 = 0x4;
    /// The sprint latch, which Shift sets every behaviour tick.
    pub const SPRINT: u32 = 0x8;
    /// Brake: the body slows to a stop.
    pub const BRAKE: u32 = 0x10;
    /// Stopping: a soft stop is under way.
    pub const STOPPING: u32 = 0x20;
    /// Heading-driven: the keys (not the mouse) steer.
    pub const KEYS: u32 = 0x40;
    /// The brake turns into a backpedal once the body is slow enough.
    pub const TURN_ABOUT: u32 = 0x200;
}

/// The body turns at most this per tick before any factor: 240°/s over 83
/// ticks (the original's unit init divides 4π/3 by its tick rate).
pub const BASE_TURN: f32 = (4.188_790_204_786_391_f64 / 83.0) as f32;

/// In free walk the move vector is the facing this long (cm): a free walk
/// never comes within its stop distance.
const FREE_WALK_REACH: f32 = 200.0;
/// A moving body stops once its move vector is within this, squared (cm²,
/// 25 cm) and it is no faster than [`STOP_SPEED`] (cm a tick).
const STOP_WITHIN: f32 = 625.0;
const STOP_SPEED: f32 = 0.5;
/// A braking body slower than this (cm a tick) may stop.
const BRAKE_SPEED: f32 = 1.0;
/// A walking body that loses its balance cannot walk again for this many
/// ticks.
const COOLDOWN: i32 = 40;
/// A moving body walks on only with this much balance (double precision);
/// a standing one starts only with [`STANDING_BALANCE`], at most
/// [`AIRBORNE_MOST`] ticks without support and no more than
/// [`REBALANCE_MOST`] ticks of Rebalance left.
const MOVING_BALANCE: f64 = 0.9;
const STANDING_BALANCE: f64 = 0.99;
const AIRBORNE_MOST: u32 = 80;
const REBALANCE_MOST: i32 = 20;
/// The turn's factors (double precision where the original's are):
/// free walk (1 − 0.1·speed)·B; the slow turn 0.5·B; otherwise
/// (1 − 0.6·enc)·1.25·B/√scale; ×0.6 crouched beyond 0.5; ×|err|/0.6 over
/// the last 0.6 rad; ×0.9 moving by the keys; a turn within 1e-4 rad ends.
const FREE_WALK_SLOWING: f64 = 0.1;
const SLOW_TURN: f32 = 0.5;
const TURN_ENCUMBRANCE: f64 = 0.6;
const BRISK_TURN: f32 = 1.25;
const CROUCHED: f32 = 0.5;
const CROUCH_TURN: f64 = 0.6;
const TURN_EASE: f64 = 0.6;
const TURN_EASE_GAIN: f64 = 1.666_666_666_666_666_7;
const KEYS_TURN: f64 = 0.9;
const TURN_SNAP: f64 = 1e-4;
/// The alignment's target and smoothing: 0.4 + 0.6·t moving, kept 0.97 a
/// tick (double precision).
const ALIGN_FLOOR: f64 = 0.4;
const ALIGN_GAIN: f64 = 0.6;
const ALIGN_KEEP: f64 = 0.97;
const ALIGN_TAKE: f64 = 0.03;
/// Until the body walks by its carry, a braking capsule keeps this share
/// of its speed a tick: the original's brake drive, 0.2 of the human push
/// gain (550/83² a tick²), and its living limbs' retention, 0.98977.
const CAPSULE_BRAKE_KEEP: f32 = (1.0 - 0.2 * 550.0 / (83.0 * 83.0)) * 0.98977;
/// A braking capsule's goal ahead counts as at least this far (cm), so one
/// on it halves its speed a tick.
const CAPSULE_STOP_WITHIN: f32 = 1.0;
/// Body ticks a second.
const TICKS_A_SECOND: f32 = 83.0;
/// Exarust's capsule speeds (m/s): walking settles near 1.4 m/s; running
/// at a full run blend 3.6 m/s (a measured guess until the jog row is
/// simulated); backpedalling is the walk scaled by the gait table's
/// thrusts (77.8 against 88.9).
const CAPSULE_WALK: f32 = 1.4;
const CAPSULE_RUN: f32 = 3.6;
const CAPSULE_BACK: f32 = CAPSULE_WALK * 77.8 / 88.9;

/// The locomotion request: what a character's behaviour asks of its body
/// this tick (on the body model root). The player's input writes it (the
/// NPCs' path driver does not yet); the body tick's movement decision
/// reads it and clears what it refuses.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
#[require(Locomotor, CapsuleGaits)]
pub struct Locomotion {
    /// The heading the body turns toward and walks along (a yaw, rad; the
    /// model faces +Z).
    pub heading: f32,
    /// Where the head looks (a yaw), for the head's look.
    pub look: f32,
    /// The smoothed cursor direction (a yaw); a character without a cursor
    /// keeps its heading here.
    pub cursor: f32,
    /// The wanted direction (a yaw): the raw cursor direction, which the
    /// alignment reads when the keys do not steer.
    pub desired: f32,
    /// The flag word ([`flags`]).
    pub flags: u32,
    /// The behaviour moves the body: the decision walks it, or stops it and
    /// clears this.
    pub moving: bool,
    /// Free walk: the body walks along its facing with no goal (it never
    /// brakes for one); otherwise toward [`Locomotion::goal`].
    pub free_walk: bool,
    /// The goal point (m, world), outside free walk.
    pub goal: Vec3,
    /// The slow turn (0.5·B): the exploration behaviour asks for it.
    pub slow_turn: bool,
    /// Where a behaviour looks at (m, world) and until when (ms): while the
    /// deadline is in the past the look turns toward the point instead of
    /// the cursor. None keeps the cursor.
    pub look_target: Option<Vec3>,
    pub look_until: u64,
    /// Exarust's interim footwork: while set the body steps (it cannot
    /// locomote) and its capsule takes this velocity (m/s), until the
    /// original's steps replace it.
    pub stepping: Option<Vec3>,
}

impl Default for Locomotion {
    fn default() -> Self {
        Self {
            heading: 0.0,
            look: 0.0,
            cursor: 0.0,
            desired: 0.0,
            flags: 0,
            moving: false,
            free_walk: false,
            goal: Vec3::ZERO,
            slow_turn: false,
            look_target: None,
            look_until: 0,
            stepping: None,
        }
    }
}

impl Locomotion {
    /// The request and the body's state of a character standing still,
    /// facing `yaw`.
    pub fn facing(yaw: f32) -> (Locomotion, Locomotor) {
        (
            Locomotion {
                heading: yaw,
                look: yaw,
                cursor: yaw,
                desired: yaw,
                ..default()
            },
            Locomotor {
                yaw,
                stance_facing: yaw,
                frame_yaw: yaw,
                ..default()
            },
        )
    }

    /// Whether every bit of `f` is set.
    pub fn has(&self, f: u32) -> bool {
        self.flags & f == f
    }

    /// What is left of the body's turn toward the heading (rad).
    pub fn turn_left(&self, body: &Locomotor) -> f32 {
        turn_between(body.yaw, self.heading)
    }
}

/// How the legs move this tick (the original's leg mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegMode {
    Walk,
    Run,
    Backpedal,
    #[default]
    Standing,
}

/// Why the body stands this tick, or that it moves: the movement
/// decision's reason, for the trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stop {
    /// It moves.
    Moving,
    /// Nothing asks it to move.
    #[default]
    Standing,
    /// It lost its balance walking less than 40 ticks ago.
    Cooldown,
    /// Its energy is under its most (dazed): it cannot walk until full.
    Tired,
    /// It steps.
    Stepping,
    /// Walking, its balance fell under 0.9.
    Unsteady,
    /// Standing, it has had no support for more than 80 ticks.
    Airborne,
    /// Standing, its balance is under 0.99.
    Unbalanced,
    /// More than 20 ticks of Rebalance are left.
    Rebalancing,
    /// It braked below 1 cm a tick.
    Braked,
    /// It is within 25 cm of its goal at no more than 0.5 cm a tick.
    Arrived,
}

impl Stop {
    /// A short name for the trace.
    pub fn name(self) -> &'static str {
        match self {
            Stop::Moving => "moving",
            Stop::Standing => "standing",
            Stop::Cooldown => "cooldown",
            Stop::Tired => "tired",
            Stop::Stepping => "stepping",
            Stop::Unsteady => "unsteady",
            Stop::Airborne => "airborne",
            Stop::Unbalanced => "unbalanced",
            Stop::Rebalancing => "rebalancing",
            Stop::Braked => "braked",
            Stop::Arrived => "arrived",
        }
    }
}

/// What the body tick keeps of a character's locomotion (on the body
/// model root): its yaw and how it turns, its speed along its heading, the
/// alignment of its pelvis with its heading, its leg mode, the cooldown
/// after a stumble, why it stands, its run blend, its gait's columns and
/// what its locomotion routine last made of them (the drive, the boost
/// ramp, the clock's step, where its walk's clock was).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Locomotor {
    /// The body's yaw (rad; the model faces +Z).
    pub yaw: f32,
    /// The signed heading error this tick (0 while the turn is held).
    pub error: f32,
    /// The speed along the heading (cm a tick).
    pub speed: f32,
    /// How well the pelvis faces the heading, 0.4 to 1 moving (smoothed),
    /// 1 − enc standing: the drive along the heading is scaled by it.
    pub align: f32,
    /// The locomotion amount: the move vector's length squared (cm²)
    /// during the decision, then 1 moving × the alignment, 0 standing.
    pub amount: f32,
    /// How far the move vector reached this tick (cm): 200 in free walk,
    /// the goal's distance while moving toward it, else 1.
    pub reach: f32,
    /// The leg mode.
    pub leg: LegMode,
    /// Ticks until a body that stumbled walking may walk again (it counts
    /// down every tick, below 0 too).
    pub cooldown: i32,
    /// The body is turning toward its heading.
    pub turning: bool,
    /// The most the body turned this tick (rad): its turn rate after every
    /// factor, the step ÷ √scale.
    pub turn_rate: f32,
    /// Why it stands this tick, or that it moves.
    pub stop: Stop,
    /// The run blend, 0 walking to 1 running.
    pub run: f32,
    /// The gait's columns: the walk row's standing, blended toward the
    /// jog's (or the sprint's) by the run blend while moving.
    pub gait: GaitRow,
    /// The boost ramp, 0 to 1: it rises while the walk plays.
    pub boost: f32,
    /// The locomotion drive this tick (cm a tick²): the push along the
    /// heading less the damping of the root's displacement. Until the body
    /// walks by its carry nothing applies it.
    pub drive: Vec3,
    /// The direction of the root's displacement along the heading.
    pub going: Vec3,
    /// The locomotion clock's step (frames a tick).
    pub clock_step: f32,
    /// The locomotion routine ran this tick: its clock's step is the clip
    /// clock's.
    pub locomoted: bool,
    /// The walk's phase kept while the body is without support
    /// ([`PHASE_UNSET`] once it has support).
    pub phase: f32,
    /// The walk's clip clock after the last locomotion tick, and when that
    /// was (ms of the game clock): a walk started within 600 ms resumes it.
    pub last_clock: f32,
    pub last_ran: u64,
    /// The stance facing (a yaw): the heading, kept within a quarter turn
    /// of the body's yaw.
    pub stance_facing: f32,
    /// The torso twist's yaw in the character frame, which follows the
    /// look, and the twist (a rotation in the character frame; the
    /// identity while it holds) the sampler turns the spine by.
    pub twist_yaw: f32,
    pub twist: Quat,
    /// The character frame's yaw this tick's sampler reads: the body's
    /// yaw before this tick's turn (the original rebuilds its frame after
    /// its sampler).
    pub frame_yaw: f32,
    /// The head look's weight, rising 0.01 a tick to 1 while the head
    /// looks, and the look yaw it follows.
    pub head_weight: f32,
    pub head_yaw: f32,
    /// The held items' grip mode the routine sets: running the general
    /// mode, otherwise the walk's (or the general one in combat).
    pub grip_mode: i32,
    /// The original's exertion value, which the run blend's cap lowers and
    /// standing bleeds, and whose square and sign make the head tilt. The
    /// original's feet routine also writes it from the feet's pull; that
    /// writer is not ported, so it stays 0 and the tilt stays inert.
    pub exertion: f32,
    /// The head tilt the torso twist reads (the original's run lean). It
    /// persists between ticks: `lean` steps it
    /// each tick and the airborne locomotion ramps it.
    pub head_tilt: f32,
    /// The jump flag and its timer: neither is written anywhere in the
    /// original, so the jump branch is dead; kept for the rule.
    pub jump: bool,
    pub jump_progress: f32,
    /// A start callback is registered (none is in Exarust): the routine
    /// then calls it every tick another clip plays.
    pub start_callback: bool,
    /// Another clip played this tick and the original cleared its held
    /// items' motion flag (and called the callback) there.
    pub start_notice: bool,
    /// The game clock (ms) at this tick: the look target's deadline is
    /// compared with it.
    pub now: u64,
}

impl Default for Locomotor {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            error: 0.0,
            speed: 0.0,
            align: 1.0,
            amount: 0.0,
            reach: 0.0,
            leg: LegMode::Standing,
            cooldown: 0,
            turning: false,
            turn_rate: 0.0,
            stop: Stop::Standing,
            run: 0.0,
            gait: GaitTable::HUMAN_ROWS[0],
            boost: 0.0,
            drive: Vec3::ZERO,
            going: Vec3::ZERO,
            clock_step: 0.0,
            locomoted: false,
            phase: PHASE_UNSET,
            last_clock: 0.0,
            last_ran: 0,
            stance_facing: 0.0,
            twist_yaw: 0.0,
            twist: Quat::IDENTITY,
            frame_yaw: 0.0,
            head_weight: 0.0,
            head_yaw: 0.0,
            grip_mode: 0,
            exertion: 0.0,
            head_tilt: 0.0,
            jump: false,
            jump_progress: 0.0,
            start_callback: false,
            start_notice: false,
            now: 0,
        }
    }
}

impl Locomotor {
    /// The body moves this tick (the decision took its move branch).
    pub fn moving(&self) -> bool {
        self.stop == Stop::Moving
    }

    /// The horizontal direction the body faces.
    pub fn facing(&self) -> Vec3 {
        forward(self.yaw)
    }
}

/// What the movement decision reads of the body this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Footing {
    /// Where the character stands (m, world).
    pub standing: Vec3,
    /// Its speed along its heading (cm a tick).
    pub speed: f32,
    /// Its energy and its most.
    pub energy: f32,
    pub max_energy: f32,
    /// Its balance, and the ticks it has gone without support.
    pub balance: f32,
    pub airborne: u32,
    /// Ticks of Rebalance left (none kept yet: 0).
    pub rebalance: i32,
    /// Its gait table.
    pub gaits: GaitTable,
}

impl Footing {
    /// A steady, rested body standing at `standing`, `speed` cm a tick
    /// along its heading.
    pub fn steady(standing: Vec3, speed: f32) -> Self {
        Self {
            standing,
            speed,
            energy: 1.0,
            max_energy: 1.0,
            balance: 1.0,
            airborne: 0,
            rebalance: 0,
            gaits: GaitTable::default(),
        }
    }

    /// The footing of `body`: its energy and its most from its drive, its
    /// balance steady (a body without a drive is steady and rested).
    pub fn of(body: Option<&Ragdoll>, standing: Vec3, speed: f32) -> Self {
        let mut at = Footing::steady(standing, speed);
        if let Some(d) = body.and_then(|b| b.drive.as_ref()) {
            at.energy = d.energy;
            at.max_energy = d.max_energy();
        }
        at
    }

    /// The footing with `body`'s balance and the ticks it has gone without
    /// support, as its character tick measured them.
    pub fn balanced_by(mut self, body: &Ragdoll) -> Self {
        if let Some(d) = body.drive.as_ref() {
            self.balance = d.balance.value;
            self.airborne = d.balance.airborne();
        }
        self
    }
}

/// What the caller does to the body after the decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Decided {
    /// A walking body lost its balance: its landing momentum takes its full
    /// weight ([`Drive::land`]).
    pub land: bool,
}

/// The body tick's movement decision, as the original's: the move vector
/// (the facing × 200 cm in free walk, else toward the goal while moving,
/// else the facing), whose length squared is the locomotion amount; while
/// moving the leg mode and the brake (backwards: backpedal, brake within
/// the stop distance; already braking: walk; running: run, never braking;
/// else walk, brake within the stop distance — never in free walk); the
/// gates ([`Stop`]), which on refusal clear the amount, the flags and the
/// speed; a brake below 1 cm a tick stops the body, or turns into a
/// backpedal with the turn-about bit. Then the move branch (moving, and
/// the amount over 625 cm² or the speed over 0.5 cm a tick): amount 1; or
/// the stop branch: amount 0, moving cleared, the stop distance back to
/// the walk's.
pub fn decide(req: &mut Locomotion, body: &mut Locomotor, at: &Footing) -> Decided {
    use flags::*;
    body.speed = at.speed;
    let facing = body.facing();
    let to = if req.free_walk {
        facing * FREE_WALK_REACH
    } else if req.moving {
        cm(req.goal - at.standing)
    } else {
        facing
    };
    body.amount = to.with_y(0.0).length_squared();
    body.reach = body.amount.sqrt();
    body.locomoted = false;
    let stop = body.gait.stop_cm();
    let near = stop * stop > body.amount && !req.free_walk;
    if req.moving {
        if req.has(BACK) {
            body.leg = LegMode::Backpedal;
            if near {
                req.flags = (req.flags | BRAKE) & !TURN_ABOUT;
            }
        } else if req.has(BRAKE) {
            body.leg = LegMode::Walk;
        } else if req.has(RUN) {
            body.leg = LegMode::Run;
            req.flags &= !BRAKE;
        } else {
            body.leg = LegMode::Walk;
            if near {
                req.flags |= BRAKE;
            }
        }
    }
    let (gate, land) = gates(req.moving, req.stepping.is_some(), body, at);
    let mut braked = false;
    if gate.is_some() {
        body.amount = 0.0;
        req.flags = 0;
        body.speed = 0.0;
    } else if req.has(BRAKE) && body.speed < BRAKE_SPEED {
        if req.has(TURN_ABOUT) {
            req.flags = (req.flags & !BRAKE) | BACK;
        } else {
            body.amount = 0.0;
            braked = true;
        }
    }
    if req.moving && (body.amount > STOP_WITHIN || body.speed > STOP_SPEED) {
        body.amount = 1.0;
        body.stop = Stop::Moving;
    } else {
        body.amount = 0.0;
        body.stop = match gate {
            Some(reason) => reason,
            None if !req.moving => Stop::Standing,
            None if braked => Stop::Braked,
            None => Stop::Arrived,
        };
        if req.moving {
            req.moving = false;
            body.gait = *at.gaits.walk();
        }
        body.leg = LegMode::Standing;
    }
    Decided { land }
}

/// The gates of a body that is to locomote, in the original's order: the
/// cooldown after a stumble; full energy; not stepping (which refuses but
/// lets the next tests run); moving, balance 0.9 or more (else the
/// 40-tick cooldown and the landing's full weight); standing, at most 80
/// ticks without support and balance 0.99 or more; at most 20 ticks of
/// Rebalance. Returns the first refusal and whether the body landed.
fn gates(moving: bool, stepping: bool, body: &mut Locomotor, at: &Footing) -> (Option<Stop>, bool) {
    if body.cooldown > 0 {
        return (Some(Stop::Cooldown), false);
    }
    if at.energy < at.max_energy {
        return (Some(Stop::Tired), false);
    }
    let no = stepping.then_some(Stop::Stepping);
    let balance = f64::from(at.balance);
    if moving {
        if balance < MOVING_BALANCE {
            body.cooldown = COOLDOWN;
            return (Some(no.unwrap_or(Stop::Unsteady)), true);
        }
    } else if at.airborne > AIRBORNE_MOST {
        return (Some(no.unwrap_or(Stop::Airborne)), false);
    } else if balance < STANDING_BALANCE {
        return (Some(no.unwrap_or(Stop::Unbalanced)), false);
    }
    if at.rebalance > REBALANCE_MOST {
        return (Some(no.unwrap_or(Stop::Rebalancing)), false);
    }
    (no, false)
}

/// What sets a body's turn rate this tick, besides its speed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurnBy {
    /// It walks free (the request's free walk).
    pub free_walk: bool,
    /// The request asks for the slow turn.
    pub slow: bool,
    /// It attacks: the attack law's rate (rad a tick).
    pub attack: Option<f32>,
    pub encumbrance: f32,
    pub scale: f32,
    /// How far it crouches, 0 to 1.
    pub crouch: f32,
}

impl TurnBy {
    /// A standing human out of every special state: the brisk turn.
    pub const BRISK: TurnBy = TurnBy {
        free_walk: false,
        slow: false,
        attack: None,
        encumbrance: 0.0,
        scale: 1.0,
        crouch: 0.0,
    };
}

/// The body's turn rate a tick by the original's table, before the ease
/// over the last 0.6 rad, the keys' factor and the √scale: in free walk
/// (1 − 0.1·speed)·B (speed in cm a tick, no floor); the slow turn 0.5·B
/// (120°/s); attacking, its law's rate; otherwise (1 − 0.6·enc)·1.25·B/
/// √scale (300°/s); then ×0.6 crouched beyond 0.5.
pub fn turn_rate(by: &TurnBy, speed: f32) -> f32 {
    let rate = if by.free_walk {
        ((1.0 - f64::from(speed) * FREE_WALK_SLOWING) * f64::from(BASE_TURN)) as f32
    } else if by.slow {
        BASE_TURN * SLOW_TURN
    } else if let Some(attack) = by.attack {
        attack
    } else {
        ((1.0 - f64::from(by.encumbrance) * TURN_ENCUMBRANCE) * f64::from(BASE_TURN * BRISK_TURN)
            / f64::from(by.scale.sqrt())) as f32
    };
    if by.crouch > CROUCHED {
        (f64::from(rate) * CROUCH_TURN) as f32
    } else {
        rate
    }
}

/// One tick of the body's turn toward `heading` at `rate` (from
/// [`turn_rate`]) for a body of `scale`, moving by the keys or not: the
/// error is the heading less the yaw, brought once within ±π; the stance
/// facing is the heading within a quarter turn of the yaw, else the yaw a
/// quarter turn toward it; within 0.6 rad the rate shrinks by |err|/0.6;
/// moving by the keys ×0.9; the step is the rate ÷ √scale; a turn within
/// 1e-4 rad, or within one step, ends on the heading, else the yaw is
/// brought once within ±π. Both wraps compare in double precision against
/// the double π (an error of the single-precision π, just above it, is
/// wrapped).
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn turn(body: &mut Locomotor, heading: f32, rate: f32, scale: f32, keys_moving: bool) {
    use std::f32::consts::{FRAC_PI_2, TAU};
    use std::f64::consts::PI;
    let heading = wrap(heading);
    let mut error = heading - body.yaw;
    if f64::from(error) < -PI {
        error += TAU;
    } else if f64::from(error) > PI {
        error -= TAU;
    }
    body.error = error;
    body.turning = true;
    // Beyond a quarter turn (not NaN) the facing is the yaw a quarter turn
    // toward the heading.
    body.stance_facing = if !(error.abs() > FRAC_PI_2) {
        heading
    } else {
        wrap(sign(error) * FRAC_PI_2 + body.yaw)
    };
    let size = f64::from(error.abs());
    let mut rate = rate;
    if size < TURN_EASE {
        rate = (f64::from(error.abs() * rate) * TURN_EASE_GAIN) as f32;
    }
    if keys_moving {
        rate = (f64::from(rate) * KEYS_TURN) as f32;
    }
    let step = rate / scale.sqrt();
    body.turn_rate = step;
    if size < TURN_SNAP || step > error.abs() {
        body.yaw = heading;
        body.turning = false;
    } else {
        let mut yaw = sign(error) * step + body.yaw;
        if f64::from(yaw) > PI {
            yaw -= TAU;
        } else if f64::from(yaw) < -PI {
            yaw += TAU;
        }
        body.yaw = yaw;
    }
}

/// The original's sign: 1 or −1 by the sign bit, 0 for zero (NaN keeps
/// its sign bit's 1).
fn sign(x: f32) -> f32 {
    if x == 0.0 { 0.0 } else { 1.0_f32.copysign(x) }
}

/// The turn held this tick (its condition is helpless, or a lock holds
/// it): no error, no turn.
pub fn hold_turn(body: &mut Locomotor) {
    body.error = 0.0;
    body.turn_rate = 0.0;
}

/// The alignment's target: standing 1 − enc; moving by the keys 0.4 +
/// 0.6·(heading·pelvis forward)²; moving by the mouse 0.4 + 0.6·t, t = the
/// wanted direction · the facing, 0.4 when t is negative.
pub fn align_target(
    moving: bool,
    keys: bool,
    heading: Vec3,
    pelvis_forward: Vec3,
    desired: Vec3,
    facing: Vec3,
    encumbrance: f32,
) -> f32 {
    if !moving {
        return 1.0 - encumbrance;
    }
    let t = if keys {
        let d = heading.dot(pelvis_forward);
        d * d
    } else {
        desired.dot(facing)
    };
    if 0.0 > t {
        ALIGN_FLOOR as f32
    } else {
        (f64::from(t) * ALIGN_GAIN + ALIGN_FLOOR) as f32
    }
}

/// One tick of the alignment toward `target` (0.97 kept, double
/// precision), which then scales the locomotion amount.
pub fn align(body: &mut Locomotor, target: f32) {
    body.align = (f64::from(body.align) * ALIGN_KEEP + f64::from(target) * ALIGN_TAKE) as f32;
    body.amount *= body.align;
}

/// One tick of the cooldown after a stumble (it counts on below 0).
pub fn count_down(body: &mut Locomotor) {
    body.cooldown -= 1;
}

/// How fast a character's capsule moves (m/s) while it executes the
/// decision (Exarust's own, until the body walks by its carry).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct CapsuleGaits {
    pub walk: f32,
    pub run: f32,
    pub back: f32,
}

impl Default for CapsuleGaits {
    fn default() -> Self {
        Self {
            walk: CAPSULE_WALK,
            run: CAPSULE_RUN,
            back: CAPSULE_BACK,
        }
    }
}

/// The velocity (m/s) a character's capsule standing at `standing` takes
/// this tick from the decision: the interim footwork's while it steps;
/// none standing; braking, `last` (its velocity last tick) slowed by the
/// original's brake drive and limb retention, and toward a goal ahead at
/// least enough to stop on it (v − v²/2d a tick, d the goal's distance
/// along its way, in cm; Exarust's own, as the capsule has no feet to
/// plant);
/// backwards, the backpedal against the heading; else along the heading at
/// the walk, blended toward the run by the run blend.
pub fn capsule_velocity(
    req: &Locomotion,
    body: &Locomotor,
    gaits: &CapsuleGaits,
    last: Vec3,
    standing: Vec3,
) -> Vec3 {
    if let Some(step) = req.stepping {
        return step;
    }
    if !body.moving() {
        return Vec3::ZERO;
    }
    let heading = forward(req.heading);
    if req.has(flags::BRAKE) {
        let kept = last * CAPSULE_BRAKE_KEEP;
        let Some(way) = last.try_normalize().filter(|_| !req.free_walk) else {
            return kept;
        };
        let left = cm((req.goal - standing).dot(way));
        if left <= 0.0 {
            return kept;
        }
        let speed = cm(last.length()) / TICKS_A_SECOND;
        let stopping = (speed - speed * speed / (2.0 * left.max(CAPSULE_STOP_WITHIN))).max(0.0);
        let speed = stopping.min(speed * CAPSULE_BRAKE_KEEP);
        way * (speed * TICKS_A_SECOND / cm(1.0))
    } else if req.has(flags::BACK) {
        -heading * gaits.back
    } else {
        heading * (gaits.walk + (gaits.run - gaits.walk) * body.run)
    }
}

impl Drive {
    /// Gives the balance's landing momentum its full weight (a walking body
    /// that lost its balance).
    pub fn land(&mut self) {
        self.balance.land_fully();
    }
}

impl Ragdoll {
    /// The root part's displacement over the last tick (m): its origin on
    /// its particles less its origin where they were a tick before (none
    /// without a root).
    pub fn root_moved(&self) -> Vec3 {
        let Some(root) = self.parts.iter().find(|p| p.parent.is_none()) else {
            return Vec3::ZERO;
        };
        let w = root.own_origin;
        (0..4)
            .map(|k| (root.positions[k] - root.previous[k]) * w[k])
            .sum()
    }

    /// The pelvis's forward across the ground for a character frame facing
    /// `facing`: the facing turned by how far the pelvis is turned from its
    /// target this tick (the facing itself without one).
    pub fn pelvis_forward(&self, facing: Vec3) -> Vec3 {
        let Some(d) = self.drive.as_ref() else {
            return facing;
        };
        let Some(root) = self.parts.iter().position(|p| p.parent.is_none()) else {
            return facing;
        };
        let Some(target) = d.targets.get(root) else {
            return facing;
        };
        let turn = rotation(&self.parts[root].transform()) * rotation(target).inverse();
        (turn * facing).with_y(0.0).normalize_or(facing)
    }
}

/// The rotation of an affine transform.
fn rotation(t: &Affine3A) -> Quat {
    t.to_scale_rotation_translation().1
}

/// The horizontal direction of a yaw (the model faces +Z).
pub fn forward(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

/// The yaw of a horizontal direction.
pub fn yaw_of(d: Vec3) -> f32 {
    d.x.atan2(d.z)
}

/// A yaw brought within (−π, π].
pub fn wrap(a: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    if a > -PI && a <= PI {
        return a;
    }
    let w = (a + PI).rem_euclid(TAU) - PI;
    if w <= -PI { w + TAU } else { w }
}

/// The signed shortest turn from `from` to `to` (rad).
pub fn turn_between(from: f32, to: f32) -> f32 {
    wrap(to - from)
}

#[cfg(test)]
#[path = "body_locomotion_test.rs"]
mod tests;
