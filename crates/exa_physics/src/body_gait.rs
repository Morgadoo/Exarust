//! The original's locomotion routine, which a moving body runs once its
//! movement decision, turn and alignment are done: the run blend (its
//! bleeds by the alignment, the stance level and vertical travel), the
//! gait columns blended from the walk toward the jog (or the sprint) row,
//! the walk's start key or its resume, the boost ramp, the drive along the
//! heading, the speed along the heading, and the locomotion clock's step.
//! A standing body's run blend falls instead ([`stand`]).
//!
//! The gait table is the race's (`races.rdb`), scaled per tick as the
//! original's loader scales it ([`GaitRow::scaled`]). Lengths are the
//! original's centimetres and centimetres a tick; the stop distance is the
//! loader's built-in, in thousands of centimetres. Evidence: reverse
//! engineering of the locomotion routine and the body tick's standing
//! branch (`docs/PLAYER.md`, Movement; `docs/formats/RSQ.md`, Locomotion).
//! (Included from body_locomotion.rs: `#[path = "body_gait.rs"] mod gait;`)

use bevy::prelude::*;

use super::{LegMode, Locomotion, Locomotor, flags};

/// The loader's per-tick factor, 1/83 in single precision: thrust and clock
/// are multiplied by it, the push gain by its square.
const PER_TICK: f32 = 1.0 / 83.0;
/// The loader's built-in stop distances (thousands of cm): walk, jog,
/// sprint, backpedal.
pub const STOPS: [f32; 4] = [0.05, 0.2, 0.25, 0.05];
/// The decision reads the stop distance × this as centimetres.
const STOP_CM: f32 = 1000.0;
/// The row of the backpedal, whose thrust and clock the loader negates.
const BACK_ROW: usize = 3;

/// Running, the run blend rises this much a tick (double precision) to
/// 1 − 0.3·enc, 1 − 0.4·enc with the sprint latch.
const RUN_RISE: f64 = 0.05;
const RUN_CAP_WORN: f64 = 0.3;
const RUN_CAP_LATCHED: f64 = 0.4;
/// Otherwise it falls this much a tick (double); braking or stopping it is
/// at most f·(2 − f), f = max(0.2·(speed − 2), 0).
const RUN_FALL: f64 = 0.015;
const RUN_SPEED_FROM: f32 = 2.0;
const RUN_SPEED_SHARE: f64 = 0.2;
/// Vertical travel bleeds it by this much per cm a tick (double).
const RUN_VERTICAL: f64 = 0.005;
/// A standing body's run blend falls this much a tick (double).
const RUN_STAND_FALL: f64 = 0.02;
/// The sprint row needs the latch, a stance level above this (double) and
/// a speed above [`SPRINT_FROM`] (cm a tick).
const SPRINT_LEVEL: f64 = 0.9;
const SPRINT_FROM: f32 = 3.5;
/// With the latch the clock's rate is × (1 − 0.1·enc) (double); crouched
/// beyond 0.1 × (1 − 0.1·crouch) (double).
const LATCHED_CLOCK: f64 = 0.1;
const CROUCH_FROM: f64 = 0.1;
const CROUCH_CLOCK: f64 = 0.1;
/// The single-precision epsilon the original's square roots test against.
const EPSILON: f32 = 1.192_09e-7;
/// The exertion value (the original's) is kept ×0.98 a tick while the run
/// blend is capped, ×0.99 otherwise (both double precision). It is the run
/// blend's own margin: it falls while the body is held under its cap.
const EXERTION_CAPPED: f64 = 0.98;
const EXERTION_KEEP: f64 = 0.99;
/// The head tilt the torso twist reads: the sign of the exertion × its
/// square × 2.
const TILT_SHARE: f32 = 2.0;
/// What the exertion value keeps a tick of standing (double precision).
const STAND_EXERTION_KEEP: f64 = 0.95;
/// What the head tilt keeps each tick (double precision).
const TILT_KEEP: f64 = 0.95;
/// What the head tilt gains a tick airborne (double precision).
const TILT_RAMP: f64 = 0.05;
/// A jump (a flag and a timer the original never writes) decelerates the
/// clock by this much a tick (double) once its timer passes a half.
const JUMP_DECAY: f64 = 0.013;
const JUMP_FROM: f32 = 0.5;
/// The grip mode the locomotion routine sets: the general mode (a run, or
/// an in-combat behaviour's walk), else the idle/walk mode. Its only
/// readers are the held items' grips, which Exarust does not model as
/// bodies yet.
pub const GRIP_MODE: i32 = 1;
pub const GRIP_MODE_WALKING: i32 = 2;
/// A walk that ran within this long (ms) resumes its clock.
const RESUME_WITHIN: u64 = 600;
/// The walk's start keys: backwards; with the heading more than 1 rad on
/// the yaw's negative side, running (run blend above 0.5) or walking;
/// otherwise.
const KEY_BACK: usize = 4;
const KEY_TURN_RUN: usize = 5;
const KEY_TURN_WALK: usize = 8;
const KEY_AHEAD: usize = 2;
const KEY_TURN: f32 = 1.0;
const KEY_RUN: f32 = 0.5;
/// The boost ramp rises 0.5/83 a tick to 1 while the walk plays.
const BOOST_RISE: f32 = 0.006_024_096;
/// The phase push: (1.1 − 0.6·|0.5 − frac(2p + 1)|) (double) ×
/// (2 − align²); the boost compares 8 × the speed along the heading.
const PUSH_TOP: f64 = 1.1;
const PUSH_DIP: f64 = 0.6;
const BOOST_SPEED: f32 = 8.0;
/// The drive's damping: −0.4·tone × the displacement, −0.2·tone braking
/// (double).
const DRIVE_DAMPING: f64 = -0.4;
const BRAKE_DAMPING: f64 = -0.2;
/// The clock: (0.8·speed + 1)·rate (double) × the alignment's phase
/// envelope K = (1 − k)·(1 − 1.8·|0.5 − frac(2p + 3)|)·4 + k, k = 1 −
/// (1 − align)²; stopping at no more than 5 cm a tick, rate·speed·K.
const CLOCK_PACE: f64 = 0.8;
const CLOCK_PHASE: f32 = 1.5;
const CLOCK_DIP: f64 = 1.8;
const CLOCK_SWING: f32 = 4.0;
const STOPPING_FROM: f32 = 5.0;
/// Without support: (0.8·speed + 0.9)·rate (double) shaped by the phase
/// kept from when the body lost it, 0.85 − 0.5·round(frac(p + 1)).
const AIRBORNE_PACE: f64 = 0.9;
const AIRBORNE_PHASE: f64 = 0.85;
/// The phase is unset (the supported clock sets this); any value under −1
/// is unset.
pub const PHASE_UNSET: f32 = -100.0;

/// One row of the gait table, per tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GaitRow {
    /// Thrust along the heading (cm a tick²).
    pub thrust: f32,
    /// Gain of the drive into the carry.
    pub push: f32,
    /// Stop distance in thousands of cm (× 1000 is centimetres).
    pub stop: f32,
    /// The locomotion clock's rate (frames a tick; negative backwards).
    pub clock: f32,
    /// Speed (8 × cm a tick) under which the phase push is boosted.
    pub boost: f32,
}

impl GaitRow {
    /// Row `row` of a race's gait table as the loader scales it: the
    /// thrust and the clock rate per second × 1/83, the push gain per
    /// second² × (1/83)², the built-in stop distance, the boost as stored;
    /// the backpedal's thrust and clock negated.
    pub const fn scaled(row: usize, thrust: f32, push: f32, clock: f32, boost: f32) -> Self {
        let sign = if row == BACK_ROW { -1.0 } else { 1.0 };
        GaitRow {
            thrust: sign * (thrust * PER_TICK),
            push: PER_TICK * PER_TICK * push,
            stop: STOPS[if row < 4 { row } else { 0 }],
            clock: sign * (clock * PER_TICK),
            boost,
        }
    }

    /// The stop distance (cm).
    pub fn stop_cm(&self) -> f32 {
        self.stop * STOP_CM
    }

    /// Every column `share` of the way to `other` (`self·(1 − share) +
    /// other·share`).
    fn toward(&self, other: &GaitRow, share: f32) -> GaitRow {
        let rest = 1.0 - share;
        let mix = |a: f32, b: f32| rest * a + b * share;
        GaitRow {
            thrust: mix(self.thrust, other.thrust),
            push: mix(self.push, other.push),
            stop: mix(self.stop, other.stop),
            clock: mix(self.clock, other.clock),
            boost: mix(self.boost, other.boost),
        }
    }
}

/// The gait table a body's locomotion reads: its race's four rows, its own
/// sprint row and its clock factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaitTable {
    /// Walk, jog, sprint and backpedal, per tick.
    pub rows: [GaitRow; 4],
    /// Its character's feral level.
    pub feral: f32,
    /// Its own sprint row: the race's moved toward its jog by its feral
    /// level (the race's sprint when it is not feral).
    pub sprint: GaitRow,
    /// Its clock factor, 1/(1 − 0.1·height).
    pub clock: f32,
}

impl GaitTable {
    /// A human's rows (thrust 88.9, 350, 472.2 and 77.8 cm/s, push 550 a
    /// second², clock 27, 16.6, 13.5 and 28 frames/s).
    pub const HUMAN_ROWS: [GaitRow; 4] = [
        GaitRow::scaled(0, 88.9, 550.0, 27.0, 0.0),
        GaitRow::scaled(1, 350.0, 550.0, 16.6, 0.0),
        GaitRow::scaled(2, 472.2, 550.0, 13.5, 0.0),
        GaitRow::scaled(3, 77.8, 550.0, 28.0, 0.0),
    ];

    /// The table of a character of feral level `feral` and clock factor
    /// `clock` on its race's `rows`: its sprint row is (1 − f)·sprint +
    /// f·jog when it is feral.
    pub fn new(rows: [GaitRow; 4], feral: f32, clock: f32) -> Self {
        let sprint = if feral > 0.0 {
            rows[2].toward(&rows[1], feral)
        } else {
            rows[2]
        };
        GaitTable {
            rows,
            feral,
            sprint,
            clock,
        }
    }

    /// The walk row.
    pub fn walk(&self) -> &GaitRow {
        &self.rows[0]
    }
}

impl Default for GaitTable {
    fn default() -> Self {
        GaitTable::new(GaitTable::HUMAN_ROWS, 0.0, 1.0)
    }
}

/// What the locomotion routine reads of the body this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stride {
    /// A foot stands on the ground.
    pub supported: bool,
    /// The stance's level (1 on level ground).
    pub level: f32,
    pub encumbrance: f32,
    /// How far it crouches, 0 to 1.
    pub crouch: f32,
    /// Its muscle tone.
    pub tone: f32,
    /// How far its clip has played, 0 to 1 (as its last sample wrote it).
    pub progress: f32,
    /// Its root's displacement over the last tick (cm).
    pub moved: Vec3,
    /// The heading (a unit vector across the ground).
    pub heading: Vec3,
    /// The walk clip plays.
    pub walking: bool,
    /// The character's behaviour is in combat: under locomotion its grip
    /// mode is the general one instead of the walk's.
    pub in_combat: bool,
    /// The game clock (ms).
    pub now: u64,
}

/// How the walk starts when locomotion finds another clip playing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Start {
    /// From this key: its clock is the key before it's.
    Key(usize),
    /// With no start key (turning left beyond 1 rad): from the clip's
    /// start (inferred: the original reads its clock from before the key
    /// list, a value of about 0).
    Unkeyed,
    /// Within 600 ms of the last locomotion: from where its clock was.
    Resume(f32),
}

/// The original's fraction: `x` less its integer part toward zero (negative
/// for negative `x`; the processor's truncation, `i32::MIN` out of range).
pub fn fraction(x: f32) -> f32 {
    let whole = if x.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&x) {
        i32::MIN
    } else {
        x as i32
    };
    x - whole as f32
}

/// The processor's maximum: `a` when it is greater, else `b` (NaN gives
/// `b`).
fn max_second(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// One tick of the locomotion routine for a moving body, in the original's
/// order; returns how the walk starts when another clip plays. The run
/// blend: running, with support √align·level × it, then +0.05 to its cap;
/// otherwise −0.015 (braking or stopping at most f·(2 − f)), floor 0; then
/// −0.005·|vertical travel|, floor 0. The gait columns: the backpedal row
/// backwards; else the walk blended by the run blend toward the jog row, or
/// the sprint row with the latch, a stance level above 0.9 and over 3.5 cm
/// a tick (the clock by √run outside the run; × (1 − 0.1·enc) with the
/// latch; thrust and push × the level); the clock × the clock factor, ×
/// (1 − 0.1·crouch) beyond 0.1. A start resets the boost ramp; walking it
/// rises 0.5/83 to 1; it is × the alignment. The drive: braking −0.2·tone
/// × the displacement; else the heading × amount·thrust·tone·P, P the
/// phase push (boosted under the row's boost speed), − 0.4·tone × the
/// displacement. The speed is the displacement along the heading. The
/// clock's step as [`clock_step`].
pub fn locomote(
    req: &Locomotion,
    body: &mut Locomotor,
    table: &GaitTable,
    at: &Stride,
) -> Option<Start> {
    use flags::*;
    let latch = req.has(SPRINT);
    let back = req.has(BACK);
    let v = at.moved;
    body.start_notice = false;
    // The run blend.
    if body.leg == LegMode::Run {
        body.grip_mode = GRIP_MODE;
        if at.supported {
            body.run = body.align.sqrt() * body.run * at.level;
        }
        let per = if latch { RUN_CAP_LATCHED } else { RUN_CAP_WORN };
        let cap = (1.0 - f64::from(at.encumbrance) * per) as f32;
        body.run = (f64::from(body.run) + RUN_RISE) as f32;
        if cap < body.run {
            body.run = cap;
            body.exertion = (f64::from(body.exertion) * EXERTION_CAPPED) as f32;
        }
    } else {
        body.grip_mode = if at.in_combat {
            GRIP_MODE
        } else {
            GRIP_MODE_WALKING
        };
        if req.flags & (BRAKE | STOPPING) == 0 {
            body.run = (f64::from(body.run) - RUN_FALL) as f32;
        } else {
            let f = max_second(
                (f64::from(body.speed - RUN_SPEED_FROM) * RUN_SPEED_SHARE) as f32,
                0.0,
            );
            let f = f * (2.0 - f);
            if f < body.run {
                body.run = f;
            }
        }
        if body.run < 0.0 {
            body.run = 0.0;
        }
        body.exertion = (f64::from(body.exertion) * EXERTION_KEEP) as f32;
    }
    body.run = (f64::from(body.run) - f64::from(v.y.abs()) * RUN_VERTICAL) as f32;
    if body.run < 0.0 {
        body.run = 0.0;
    }
    // The gait columns.
    let rest = 1.0 - body.run;
    let root = if body.run > EPSILON {
        body.run.sqrt()
    } else {
        0.0
    };
    let walk = *table.walk();
    if back {
        body.gait = table.rows[BACK_ROW];
    } else {
        let row = if latch && f64::from(at.level) > SPRINT_LEVEL && body.speed > SPRINT_FROM {
            if table.feral > 0.0 {
                table.sprint
            } else {
                table.rows[2]
            }
        } else {
            table.rows[1]
        };
        let run = body.run;
        let blend = |r: f32, w: f32| r * run + w * rest;
        let mut clock = if body.leg == LegMode::Run {
            blend(row.clock, walk.clock)
        } else {
            row.clock * root + (1.0 - root) * walk.clock
        };
        if latch {
            clock = ((1.0 - f64::from(at.encumbrance) * LATCHED_CLOCK) * f64::from(clock)) as f32;
        }
        body.gait = GaitRow {
            thrust: blend(row.thrust, walk.thrust) * at.level,
            push: blend(row.push, walk.push) * at.level,
            stop: blend(row.stop, walk.stop),
            clock,
            boost: blend(row.boost, walk.boost),
        };
    }
    body.gait.clock *= table.clock;
    if f64::from(at.crouch) > CROUCH_FROM {
        body.gait.clock =
            ((1.0 - f64::from(at.crouch) * CROUCH_CLOCK) * f64::from(body.gait.clock)) as f32;
    }
    // The start, or the boost ramp of a walk under way.
    let start = if at.walking {
        body.boost += BOOST_RISE;
        if body.boost > 1.0 {
            body.boost = 1.0;
        }
        None
    } else {
        body.boost = 0.0;
        // The original calls its registered start callback here (with the
        // argument 4, clearing it when the call returns a negative value)
        // and clears the held items' motion flag on every tick another clip
        // plays. No callback is registered in Exarust: the notice tells the
        // character tick to clear the held items' flag.
        body.start_notice = body.start_callback;
        Some(if body.last_ran + RESUME_WITHIN < at.now {
            if back {
                Start::Key(KEY_BACK)
            } else if body.error < -KEY_TURN {
                Start::Key(if body.run > KEY_RUN {
                    KEY_TURN_RUN
                } else {
                    KEY_TURN_WALK
                })
            } else if body.error > KEY_TURN {
                Start::Unkeyed
            } else {
                Start::Key(KEY_AHEAD)
            }
        } else {
            Start::Resume(body.last_clock)
        })
    };
    body.boost *= body.align;
    // The drive.
    let h = at.heading;
    let tone = at.tone;
    body.drive = if req.has(BRAKE) {
        v * (f64::from(tone) * BRAKE_DAMPING) as f32
    } else {
        let p = fraction(at.progress * 2.0 + 1.0);
        let mut push = ((PUSH_TOP - f64::from((0.5 - p).abs()) * PUSH_DIP) as f32)
            * (2.0 - body.align * body.align);
        let mut along = (h.x * v.x + h.y * v.y + h.z * v.z) * BOOST_SPEED;
        if back {
            along = -along;
        }
        if along < body.gait.boost {
            push += (body.gait.boost - along) * body.boost;
        }
        let thrust = body.amount * body.gait.thrust * tone * push;
        h * thrust + v * (f64::from(tone) * DRIVE_DAMPING) as f32
    };
    // The speed along the heading and its direction.
    let dot = v.x * h.x + v.z * h.z + v.y * h.y;
    let along = h * dot;
    let squared = along.x * along.x + along.y * along.y + along.z * along.z;
    body.speed = if squared > EPSILON {
        squared.sqrt()
    } else {
        squared
    };
    body.going = if body.speed > EPSILON {
        along * (1.0 / body.speed)
    } else {
        along
    };
    clock_step(req, body, at);
    body.locomoted = true;
    body.last_ran = at.now;
    start
}

/// The locomotion clock's step this tick (kept as the body's clock step).
/// With support: (0.8·speed + 1)·rate (double) × K, K =
/// (1 − k)·(1 − 1.8·|0.5 − frac((p + 1.5)·2)|)·4 + k, k = 1 − (1 −
/// align)² (1 when aligned); stopping at no more than 5 cm a tick,
/// rate·speed·K; the phase is unset. Without support, once a phase is set
/// (0.85 − 0.5·round(frac(p + 1)), when unset): u = frac(1 + phase − p),
/// (2 − u)·u·(0.8·speed + 0.9)·rate.
fn clock_step(req: &Locomotion, body: &mut Locomotor, at: &Stride) {
    let rate = body.gait.clock;
    let speed = body.speed;
    // The jump branch: the flags it reads are never written in the
    // original, so it is dead; kept for the rule.
    if body.jump && body.jump_progress > JUMP_FROM {
        // The original decays the clock step kept from last tick.
        let step = (f64::from(body.clock_step) - JUMP_DECAY) as f32;
        body.clock_step = if step < 0.0 { 0.0 } else { step };
        return;
    }
    if at.supported {
        let p = fraction((at.progress + CLOCK_PHASE) * 2.0);
        let envelope = (1.0 - f64::from((0.5 - p).abs()) * CLOCK_DIP) as f32;
        let off = 1.0 - body.align;
        let k = 1.0 - off * off;
        let swing = (1.0 - k) * envelope * CLOCK_SWING + k;
        // Stopping, a speed that is not over 5 cm a tick (NaN too) slows the
        // clock with the body.
        let fast = speed.partial_cmp(&STOPPING_FROM) == Some(std::cmp::Ordering::Greater);
        body.clock_step = if req.has(flags::STOPPING) && !fast {
            rate * speed * swing
        } else {
            (f64::from(swing) * ((f64::from(speed) * CLOCK_PACE + 1.0) * f64::from(rate))) as f32
        };
        body.phase = PHASE_UNSET;
    } else {
        if body.phase < -1.0 {
            let whole = round_even(fraction(at.progress + 1.0));
            body.phase = (AIRBORNE_PHASE - f64::from(whole as f32 * 0.5)) as f32;
        }
        let base = ((f64::from(speed) * CLOCK_PACE + AIRBORNE_PACE) * f64::from(rate)) as f32;
        let u = fraction((1.0 + body.phase) - at.progress);
        body.clock_step = (2.0 - u) * (base * u);
        // Airborne, the head tilt ramps up 0.05 a tick (double) to at most 1.
        body.head_tilt = (f64::from(body.head_tilt) + TILT_RAMP) as f32;
        if body.head_tilt > 1.0 {
            body.head_tilt = 1.0;
        }
    }
}

/// The head tilt the twist reads, stepped once a tick before the body moves
/// or stands: while it has support the tilt is the sign of the exertion
/// times its square times two, then it always falls by 5% (double
/// precision). Airborne, the locomotion routine ramps it up instead.
pub fn lean(body: &mut Locomotor, supported: bool) {
    if supported {
        body.head_tilt = tilt_of(body.exertion);
    }
    body.head_tilt = (f64::from(body.head_tilt) * TILT_KEEP) as f32;
}

/// The head tilt an exertion value makes: the original's copysign(1, x)
/// (0 at zero, either sign for NaN) × x² × 2.
fn tilt_of(exertion: f32) -> f32 {
    let sign = if exertion == 0.0 {
        0.0
    } else {
        1.0_f32.copysign(exertion)
    };
    sign * exertion * exertion * TILT_SHARE
}

/// The processor's conversion to a 64-bit integer under its default
/// rounding: to nearest, ties to even (`i64::MIN` out of range or NaN).
fn round_even(x: f32) -> i64 {
    if x.is_nan() || !(-9.223_372e18..9.223_372e18).contains(&x) {
        i64::MIN
    } else {
        x.round_ties_even() as i64
    }
}

/// The clip clock after a locomotion tick: `clock` + `step`, below 0
/// wrapped once by the clip's `length` (its last key's time); kept as
/// where a resumed walk starts.
pub fn advance_clock(body: &mut Locomotor, clock: f32, length: f32) -> f32 {
    let mut clock = clock + body.clock_step;
    if clock < 0.0 {
        clock += length;
    }
    body.last_clock = clock;
    clock
}

/// One tick of a standing body's run blend (the body tick's branch for a
/// body that does not locomote): the exertion value keeps 95% (double
/// precision); getting up the run blend is 0, otherwise it falls 0.02 a
/// tick (double), never under 0. The locomotion drive is zero: the body
/// tick clears it every tick before the locomotion call, so a body that
/// does not locomote (one taking a recovery step included) is not pushed
/// by its last walking tick's drive.
pub fn stand(body: &mut Locomotor, getting_up: bool) {
    body.locomoted = false;
    body.drive = Vec3::ZERO;
    body.exertion = (f64::from(body.exertion) * STAND_EXERTION_KEEP) as f32;
    if getting_up {
        body.run = 0.0;
        return;
    }
    body.run = (f64::from(body.run) - RUN_STAND_FALL) as f32;
    if body.run < 0.0 {
        body.run = 0.0;
    }
}

#[cfg(test)]
#[path = "body_gait_test.rs"]
mod tests;
