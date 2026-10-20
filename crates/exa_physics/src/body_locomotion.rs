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
/// The run blend while running rises this much a tick (to its cap) and
/// otherwise falls this much; braking or stopping it follows the speed
/// down: at most f·(2 − f), f = 0.2·(speed − 2).
const RUN_RISE: f64 = 0.05;
const RUN_FALL: f64 = 0.015;
const RUN_SPEED_FROM: f32 = 2.0;
const RUN_SPEED_SHARE: f64 = 0.2;
/// With the sprint latch the gait is the sprint row once the body is
/// faster than this (cm a tick) on level feet.
const SPRINT_FROM: f32 = 3.5;
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
            Locomotor { yaw, ..default() },
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
/// after a stumble, why it stands, its run blend and its gait's stop
/// distance.
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
    /// The gait's stop distance (m): the walk's 0.5 standing, blended
    /// toward the jog's (or the sprint's) by the run blend while moving.
    pub stop_distance: f32,
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
            stop_distance: StopDistances::HUMAN.walk,
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

/// The stop distance of each gait row (m): walk, jog, sprint (a feral
/// character's, moved toward its jog), backpedal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StopDistances {
    pub walk: f32,
    pub jog: f32,
    pub sprint: f32,
    pub back: f32,
}

impl StopDistances {
    /// The rows of every race but a feral character's sprint: 50, 200, 250
    /// and 50 cm.
    pub const HUMAN: StopDistances = StopDistances {
        walk: 0.5,
        jog: 2.0,
        sprint: 2.5,
        back: 0.5,
    };
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
    /// Its gait rows' stop distances.
    pub stops: StopDistances,
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
            stops: StopDistances::HUMAN,
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
    let stop = cm(body.stop_distance);
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
            body.stop_distance = at.stops.walk;
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
/// error is the signed shortest turn; within 0.6 rad the rate shrinks by
/// |err|/0.6; moving by the keys ×0.9; the step is the rate ÷ √scale; a
/// turn within 1e-4 rad, or within one step, ends on the heading.
pub fn turn(body: &mut Locomotor, heading: f32, rate: f32, scale: f32, keys_moving: bool) {
    let error = turn_between(body.yaw, heading);
    body.error = error;
    body.turning = true;
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
        body.yaw = wrap(heading);
        body.turning = false;
    } else {
        body.yaw = wrap(body.yaw + step.copysign(error));
    }
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

/// The run blend and the gait's stop distance of a moving body this tick
/// (the original's locomotion, in the part Exarust's capsule reads):
/// running, the blend rises 0.05 to `cap` (the encumbrance's, lower with
/// the sprint latch); else braking or stopping it is at most f·(2 − f),
/// f = max(0.2·(speed − 2), 0), otherwise it falls 0.015, never under 0
/// (the original's bleed by the alignment and by vertical travel comes
/// with the walking drive). The stop distance is the backpedal's backwards, else the walk's blended
/// toward the jog's (the sprint's with the latch once faster than 3.5 cm a
/// tick) by the run blend. A standing body keeps both.
pub fn locomote(req: &Locomotion, body: &mut Locomotor, cap: f32, stops: &StopDistances) {
    use flags::*;
    if !body.moving() {
        return;
    }
    body.run = if body.leg == LegMode::Run {
        ((f64::from(body.run) + RUN_RISE) as f32).min(cap)
    } else if req.flags & (BRAKE | STOPPING) != 0 {
        let f = ((RUN_SPEED_SHARE * f64::from(body.speed - RUN_SPEED_FROM)) as f32).max(0.0);
        body.run.min(f * (2.0 - f))
    } else {
        (f64::from(body.run) - RUN_FALL) as f32
    }
    .max(0.0);
    body.stop_distance = if req.has(BACK) {
        stops.back
    } else {
        let row = if req.has(SPRINT) && body.speed > SPRINT_FROM {
            stops.sprint
        } else {
            stops.jog
        };
        stops.walk + (row - stops.walk) * body.run
    };
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
