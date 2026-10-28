//! Recovery steps. A standing body whose chest leaves its feet starts one
//! step: the target sits under the chest, ahead by its velocity, the
//! footwork's stance score moves one foot, and the episode ends by its own
//! rules. While the chest is not out of balance, each thorax pin target
//! is shifted by the horizontal error from the pin to that target plus
//! the step offset.
//!
//! Distances below are centimetres and times are milliseconds, the
//! original's. A tick is one body step (83 Hz).
//! (Included from body.rs: `#[path = "body_steps.rs"] mod steps;`.)

use bevy::prelude::*;

use super::CM;
use super::balance::cm;

/// Ticks in one second, and the milliseconds in one tick.
const HZ: f32 = 83.0;
pub(super) const MS_PER_TICK: f32 = 1000.0 / HZ;
/// Balance below this ends a step and starts none.
const BALANCE_FLOOR: f32 = 0.2;
/// The radius base (cm): the standing value, and the value after being down.
const BASE_STANDING: f32 = 30.0;
const BASE_AFTER_DOWN: f32 = 65.0;
/// The base narrows by this (cm) each standing tick.
const BASE_DECAY: f32 = 0.5;
/// A fresh step is wanted past this many radii.
const TRIGGER_RADII: f32 = 1.5;
/// The radius grows by this while a step is already going.
const STEPPING_RADIUS: f32 = 1.25;
/// The target must clear the standing position by this (cm, × scale).
const TARGET_CLEAR: f32 = 10.0;
/// Slow lead: this many ticks of the fast velocity, at or under the second
/// (cm a tick, × scale). Faster, a fixed reach along the velocity.
const LEAD_TICKS: f32 = 15.0;
const LEAD_FAST: f32 = 2.0;
const LEAD_REACH: f32 = 30.0;
/// The stance origin stays within this many step lengths of the frame, and
/// a foot's anchor within the second of the other foot.
const ORIGIN_LENGTHS: f32 = 1.1;
const ANCHOR_LENGTHS: f32 = 1.8;
/// Step length (cm) at scale 1, before the leg extension. The trait and
/// the fatigue that shorten it are 0.
const STEP_LENGTH: f32 = 80.0;
/// The episode. No anchored foot after the first, the chest near the
/// target for more than the second, the chest no longer closing after the
/// third, and more landings than the fourth.
const OPEN_MS: f32 = 100.0;
const NEAR_TICKS: u32 = 32;
const COAST_MS: f32 = 200.0;
const NEAR_CM: f32 = 20.0;
const COAST_CM: f32 = 0.05;
const LANDINGS_END: u32 = 1;
/// Stance score. The reference term loses one point per 30 cm (the first is
/// the single-precision 1/30 it multiplies by); a
/// crouch wider than the second (less the third per crouch) is penalised;
/// a spacing near the fourth and a side term near the fifth score well.
const REF_RECIP: f32 = 0.033_333_335;
const CROUCH_SPAN: f32 = 120.0;
const CROUCH_NARROW: f32 = 30.0;
const STANCE_WIDTH: f32 = 60.0;
const STANCE_SIDE: f32 = 55.0;
/// Below this the side term is not rooted (a crossed stance): the
/// original's general epsilon, just under 2⁻²³.
const SIDE_ROOT: f32 = 1.192_09e-7;
/// Foot scores. Under the first (cm, × scale; the second after a stall) a
/// free foot scores the third and stays. A moving foot scores the fourth,
/// plus the later bonuses inside their windows.
const STEP_FROM: f32 = 5.0;
const STALLED_FROM: f32 = 10.0;
const STAY_SCORE: f32 = -10.0;
const MOVE_SCORE: f32 = 1.5;
const FRESH_BONUS: f32 = 10.0;
const EARLY_BONUS: f32 = 2.0;
const LATE_BONUS: f32 = 0.5;
const FRESH_MS: f32 = 100.0;
const EARLY_MS: f32 = 300.0;
const LATE_MS: f32 = 600.0;
const CHANGE_MS: f32 = 200.0;
/// The reach penalty starts past this share of the leg, measured from this
/// blend of the frame and the stance origin, and from this share of the
/// step length.
const REACH_SHARE: f32 = 0.6;
const REACH_FROM: f32 = 0.65;
const REACH_WEIGHT: f32 = 0.1;
/// A wide crouch loses this (double) per centimetre past its span.
const CROUCH_WEIGHT: f64 = 0.1;
const ORIGIN_BLEND: f32 = 0.25;
const ROOT_BLEND: f32 = 0.75;
/// Thorax lead's clip share: Idle, and every other clip.
pub(super) const LEAD_IDLE: f32 = 0.4;
pub(super) const LEAD_GAIN: f32 = 0.7;
/// A recovery step's kind factor. A knockback step uses 1.
pub(super) const LEAD_KIND: f32 = 1.2;
/// A knockback step's kind factor.
pub(super) const KNOCKBACK_KIND: f32 = 1.0;
/// Direction: (h²/60 + 0.25) · 0.36 · how forward the error is.
const LEAD_DIR_OVER: f32 = 60.0;
const LEAD_DIR_BIAS: f32 = 0.25;
const LEAD_DIR_WEIGHT: f32 = 0.36;
/// The direction term is lifted by this before it scales the error.
const LEAD_DIR_LIFT: f32 = 0.5;
/// Speed along the error: the lead is multiplied by (2 − 25·s).
const LEAD_SPEED: f32 = 25.0;
const LEAD_SPEED_BASE: f32 = 2.0;
/// This pin pair's pool factor.
const LEAD_POOL: f32 = 1.0;
/// Below this squared length (cm²) the speed term is 0. The binary skips
/// it under a tiny epsilon; this guard is that role.
const LEAD_TINY: f32 = 1e-4;
/// The score's chest reference leads the fast velocity by this many ticks,
/// and a crouch by this many centimetres along the facing.
const SCORE_LEAD: f32 = 10.0;
const SCORE_CROUCH: f32 = 100.0;
/// At the end of a step the standing position is judged from the chest
/// plus this many ticks of the fast velocity.
const END_LEAD: f32 = 5.0;

/// A body's recovery step, kept from tick to tick.
#[derive(Debug, Clone, Copy)]
pub(super) struct Steps {
    /// A stepping episode is running.
    pub stepping: bool,
    /// The chest is past the trigger this tick.
    pub out_of_balance: bool,
    /// The step target (m).
    pub target: Vec3,
    /// The standing position (m): where the stance sits between steps.
    pub standing: Vec3,
    /// `standing` has been set.
    pub placed: bool,
    /// The radius base (cm).
    pub radius_base: f32,
    /// The clock, in body ticks, and the tick the episode started.
    pub clock: u32,
    pub started: u32,
    /// Ticks the chest has stayed near the target.
    pub near: u32,
    /// The landing count when the episode started.
    pub landed_from: u32,
    /// A blow started this step. It is not the balance trigger, and walking
    /// does not clear it.
    pub knockback: bool,
    /// The body's stand-still stamps (ms), which an anchored foot's score
    /// reads: when a foot last started moving, and a landing's time plus
    /// 100 ms (a stalled landing's time itself). The trailing-foot plant
    /// writes both, its clock and that plus 100 ms; a step's start closes
    /// the later one. NaN until written.
    pub still_at: f32,
    pub still_plus: f32,
    /// A queued footwork step's vector (m), zero when none waits, and the
    /// landings seen when it was queued.
    pub queued: Vec3,
    pub queued_landings: u32,
    /// The landings the feet last reported, so a new landing is recognised.
    pub landings_seen: u32,
    /// The AI was told about the queued step this tick.
    pub told_ai: bool,
    /// The jolt's decaying term, its one-per-episode latch, the countdown
    /// that clears the latch
    /// and whether the behaviour was told this tick.
    jolt_decay: Vec3,
    jolt_latch: bool,
    jolt_quiet: i32,
    pub jolt_told: bool,
}

impl Default for Steps {
    fn default() -> Self {
        Self {
            stepping: false,
            out_of_balance: false,
            target: Vec3::ZERO,
            standing: Vec3::ZERO,
            placed: false,
            radius_base: BASE_STANDING,
            clock: 0,
            started: 0,
            near: 0,
            landed_from: 0,
            knockback: false,
            still_at: f32::NAN,
            still_plus: f32::NAN,
            queued: Vec3::ZERO,
            queued_landings: 0,
            landings_seen: 0,
            told_ai: false,
            jolt_decay: Vec3::ZERO,
            jolt_latch: false,
            jolt_quiet: 0,
            jolt_told: false,
        }
    }
}

/// Starts a knockback step whose target is already the chest plus the push.
pub(super) fn begin_knockback(steps: &mut Steps, target: Vec3) {
    steps.knockback = true;
    steps.stepping = true;
    steps.out_of_balance = false;
    steps.target = target;
    if !steps.placed {
        steps.standing = target;
        steps.placed = true;
    }
    steps.started = steps.clock;
    steps.near = 0;
}

/// What the trigger reads this tick.
pub(super) struct Trigger {
    pub balance: f32,
    pub supported: bool,
    pub getting_up: bool,
    pub locomoting: bool,
    pub down: bool,
    /// The chest's horizontal offset from the support point (cm).
    pub h_cm: f32,
    pub chest: Vec3,
    /// The chest's fast velocity (m a tick).
    pub fast: Vec3,
    pub energy: f32,
    pub support: f32,
    pub scale: f32,
    pub bonus: f32,
    /// The footwork's reach (cm): 80·√scale·(1 − 0.2·encumbrance)·
    /// (1 − 0.3·feral), in place of the extension-scaled length.
    pub reach: f32,
    /// The character frame's origin (m).
    pub frame: Vec3,
    pub landings: u32,
    /// The shock-stagger flag: the original's step trigger and its
    /// imbalance accumulator do not run while it is set.
    pub shock_stagger: bool,
}

/// Where the stance goes this tick.
#[derive(Debug)]
pub(super) enum Plan {
    /// No episode. The feet use the frame.
    Idle,
    /// The episode runs; the stance origin is this (m).
    Stepping(Vec3),
    /// The episode ended this tick.
    Ended,
}

/// The radius R (cm): (0.75E + 0.25) · (base · scale) · 0.5, times 1.25
/// while `stepping`, then times (1 + bonus). The products are in the
/// original's order: the base times the scale first, that times the energy
/// term, then the half.
pub(super) fn radius(energy: f32, base: f32, scale: f32, stepping: bool, bonus: f32) -> f32 {
    let mut r = (0.75 * energy + 0.25) * (base * scale) * 0.5;
    if stepping {
        r *= STEPPING_RADIUS;
    }
    r * (1.0 + bonus)
}

/// One tick of the imbalance accumulator, the share of the chest's
/// horizontal error past the step trigger that pushes it toward 1: (√h² −
/// R·E⁴S) / (5 × scale) in single precision, times the double 0.005 and
/// added to the last value in double, narrowed once. The cap compares in
/// single precision, so a NaN is kept.
pub(super) fn add_imbalance(last: f32, h_cm: f32, radius_cm: f32, factor: f32, scale: f32) -> f32 {
    let excess = (h_cm - radius_cm * factor) / (5.0 * scale);
    let next = (f64::from(excess) * 0.005 + f64::from(last)) as f32;
    if 1.0 < next { 1.0 } else { next }
}

/// The feet routine's decay of the imbalance accumulator: 0.01 a tick in
/// double precision, floored at 0; a NaN stays.
pub(super) fn decay_imbalance(last: f32) -> f32 {
    let next = (f64::from(last) - 0.01) as f32;
    if next < 0.0 { 0.0 } else { next }
}

/// How far (cm) the chest must be from the support point before a step is
/// wanted: 1.5 · R · E⁴ · S.
pub(super) fn trigger_distance(
    energy: f32,
    base: f32,
    scale: f32,
    stepping: bool,
    bonus: f32,
    support: f32,
) -> f32 {
    let factor = energy.powi(4) * support;
    TRIGGER_RADII * radius(energy, base, scale, stepping, bonus) * factor
}

/// The step length (cm): 80 · √scale · the leg extension.
pub fn step_length(scale: f32, extension: f32) -> f32 {
    STEP_LENGTH * scale.max(0.0).sqrt() * extension.max(0.0)
}

/// The footwork's own reach (cm): the body's reach times the crouch's
/// stride, with no leg extension. The original's stance-origin clamp, its
/// foot scores and its step lift read this length.
pub fn foot_step_length(reach: f32, crouch: f32) -> f32 {
    reach * crouch_stride(crouch)
}

/// Crouched deeper than this, the footwork's steps are shorter.
const CROUCH_SHORTENS_FROM: f32 = 0.5;
/// A crouched step's share of the step length.
const CROUCHED_STRIDE: f64 = 0.7;

/// The footwork's step length factor at `crouch`: 0.7 above 0.5, else 1.
pub fn crouch_stride(crouch: f32) -> f32 {
    if crouch > CROUCH_SHORTENS_FROM {
        CROUCHED_STRIDE as f32
    } else {
        1.0
    }
}

/// The target's horizontal offset from the chest (m), from the fast
/// velocity (m a tick).
pub(super) fn lead(fast: Vec3, support: f32, energy: f32, scale: f32) -> Vec3 {
    let factor = energy.powi(4) * support;
    let k = (2.0 - support) * (2.0 - factor);
    let flat = fast.with_y(0.0);
    let speed = cm(flat).length();
    if speed <= LEAD_FAST * scale {
        flat * (LEAD_TICKS * scale * k)
    } else {
        flat.normalize_or_zero() * (LEAD_REACH * scale * k / CM)
    }
}

/// The radius base next tick: 65 while `down`, else 0.5 cm narrower, never
/// under 30.
pub(super) fn next_base(base: f32, down: bool) -> f32 {
    if down {
        BASE_AFTER_DOWN
    } else {
        (base - BASE_DECAY).max(BASE_STANDING)
    }
}

/// One tick of the trigger. Advances the step clock.
pub(super) fn consider(steps: &mut Steps, t: &Trigger) -> Plan {
    steps.clock = steps.clock.saturating_add(1);
    steps.radius_base = next_base(steps.radius_base, t.down || t.getting_up);
    // Getting up clears the step and places no foot from its footwork: the
    // original returns before the game tick's footwork block (the feet
    // still follow their stance spots in `feet::plant`).
    if t.getting_up {
        steps.knockback = false;
        steps.stepping = false;
        steps.out_of_balance = false;
        return Plan::Idle;
    }
    if steps.knockback {
        if t.balance < BALANCE_FLOOR {
            steps.knockback = false;
            steps.stepping = false;
            return Plan::Ended;
        }
        let length = foot_step_length(t.reach, 0.0);
        return Plan::Stepping(stance_origin(steps.target, t.frame, length, t.scale));
    }
    steps.out_of_balance = false;
    if !steps.placed {
        steps.standing = t.frame;
        steps.target = t.frame;
        steps.placed = true;
    }
    if t.balance < BALANCE_FLOOR || t.locomoting {
        let ended = steps.stepping;
        steps.stepping = false;
        return if ended { Plan::Ended } else { Plan::Idle };
    }
    let distance = trigger_distance(
        t.energy,
        steps.radius_base,
        t.scale,
        steps.stepping,
        t.bonus,
        t.support,
    );
    // The comparison is made squared, as the original's is: a NaN offset
    // wants no step.
    let wants = t.supported && !t.shock_stagger && t.h_cm * t.h_cm > distance * distance;
    steps.out_of_balance = wants;
    if wants {
        let from = if steps.stepping {
            steps.target
        } else {
            steps.standing
        };
        let offset = lead(t.fast, t.support, t.energy, t.scale);
        // The target keeps the height of the point it is measured from: the
        // running target while stepping, else the standing position.
        let new_target = Vec3::new(t.chest.x + offset.x, from.y, t.chest.z + offset.z);
        let clear = cm((new_target - from).with_y(0.0)).length();
        if clear > TARGET_CLEAR * t.scale {
            if !steps.stepping {
                steps.stepping = true;
                steps.started = steps.clock;
                steps.near = 0;
                steps.landed_from = t.landings;
                // A step's start closes the later stand-still stamp.
                steps.still_plus = f32::NAN;
            }
            steps.target = new_target;
        }
    }
    if steps.stepping {
        let length = foot_step_length(t.reach, 0.0);
        Plan::Stepping(stance_origin(steps.target, t.frame, length, t.scale))
    } else {
        Plan::Idle
    }
}

/// The stance origin (m): the target, pulled back to within 1.1 step
/// lengths × scale of the frame, at the frame's height.
pub(super) fn stance_origin(target: Vec3, frame: Vec3, step_cm: f32, scale: f32) -> Vec3 {
    let limit = ORIGIN_LENGTHS * step_cm * scale / CM;
    let d = (target - frame).with_y(0.0);
    let flat = if !limit.is_finite() || d.length() <= limit || limit <= 0.0 {
        target
    } else {
        frame + d.normalize_or_zero() * limit
    };
    Vec3::new(flat.x, frame.y, flat.z)
}

/// Whether the episode ends. `near` is the count after this tick.
pub(super) fn episode_over(
    elapsed_ms: f32,
    any_anchored: bool,
    chest_cm: f32,
    toward_cm: f32,
    near: u32,
    landings: u32,
    scale: f32,
) -> bool {
    (elapsed_ms > OPEN_MS && !any_anchored)
        || near > NEAR_TICKS
        || (elapsed_ms > COAST_MS && chest_cm > NEAR_CM * scale && toward_cm < COAST_CM)
        || landings > LANDINGS_END
}

/// The near-target count after a tick whose chest is `chest_cm` from the
/// target.
pub(super) fn note_near(near: u32, chest_cm: f32, scale: f32) -> u32 {
    if chest_cm <= NEAR_CM * scale {
        near.saturating_add(1)
    } else {
        0
    }
}

/// Milliseconds since the episode started, at `clock`.
pub(super) fn elapsed_ms(steps: &Steps) -> f32 {
    steps.clock.saturating_sub(steps.started) as f32 * MS_PER_TICK
}

/// The chest reference the stance score uses (m): the chest (while
/// stepping, the midpoint of the chest and `pins`), plus 10 ticks of the
/// fast velocity, plus 100·crouch cm along `forward`.
pub(super) fn score_reference(
    chest: Vec3,
    pins: Option<Vec3>,
    stepping: bool,
    fast: Vec3,
    forward: Vec3,
    crouch: f32,
) -> Vec3 {
    let mut at = match pins {
        Some(pins) if stepping => (chest + pins) * 0.5,
        _ => chest,
    };
    at += fast * SCORE_LEAD;
    // The original adds the crouch along the frame's facing as stored; it
    // does not normalise it again.
    at + forward.with_y(0.0) * (SCORE_CROUCH * crouch / CM)
}

/// The stance score of feet at `a` and `b` (cm) against `reference` (cm).
/// `side` is horizontal; a positive `(b − a) · side` is the natural order
/// and is rooted, a negative one is a crossed stance and is not. The
/// original's unordered edge cases are kept: a NaN reference distance takes
/// the root and gives a NaN; a footing of no length gives a NaN distance
/// (the division by zero), as the original's does.
pub(super) fn stance_score(
    a: Vec3,
    b: Vec3,
    reference: Vec3,
    side: Vec3,
    crouch: f32,
    scale: f32,
) -> f32 {
    // The original zeroes the feet's vertical difference first: the span,
    // the projection and the side term all read it across the ground.
    let e = (b - a).with_y(0.0);
    // (z² + (y² + x²)) with y 0: z² + x².
    let ee = e.z * e.z + (e.y * e.y + e.x * e.x);
    let length = if ee > SIDE_ROOT { ee.sqrt() } else { ee };
    // The reference's projection onto the feet's segment, by the original's
    // x, y, z sum, clamped to it (a NaN stays, as its unordered jumps do).
    let rv = reference - a;
    let t = ((e.y * rv.y + e.x * rv.x) + e.z * rv.z) / ee;
    let t = t.clamp(0.0, 1.0);
    let d = (a + e * t) - reference;
    // The same horizontal distance as the span, its y zeroed.
    let r2 = d.z * d.z + (0.0 + d.x * d.x);
    // The original takes the root unless the squared distance is under its
    // epsilon (a NaN takes it): 1 − r/30, the single-precision 0.033333335.
    let mut along = if r2 < SIDE_ROOT {
        1.0
    } else {
        1.0 - r2.sqrt() * REF_RECIP
    };
    let span = CROUCH_SPAN - CROUCH_NARROW * crouch;
    if crouch > 0.0 && span < length {
        along = (f64::from(along) - f64::from(length - span) * CROUCH_WEIGHT) as f32;
    }
    let width = (STANCE_WIDTH * scale - length).abs() / (STANCE_WIDTH * scale);
    let mut w = e.z * side.z + (e.y * side.y + e.x * side.x);
    if w > SIDE_ROOT {
        w = w.sqrt();
    }
    let side_term = (STANCE_SIDE * scale - w).abs() / (STANCE_SIDE * scale);
    (1.0 - side_term * side_term) * 2.0 + ((1.0 - width * width) + along)
}

/// One foot, as the choice reads it. Positions are centimetres.
pub(super) struct FootView {
    pub anchored: bool,
    pub stalled: bool,
    pub anchor: Vec3,
    pub point: Vec3,
    pub leg: f32,
    pub changed_ms: f32,
}

/// The queued footwork step's rule, run at the landing after the step
/// ended: `chest` and `target` are the body's, `facing` its forward, and
/// `step` takes the new target. `landings` is 0 before the first landing
/// and 1 after it (the original's countdown). Returns whether the AI is to
/// be told the step points its way.
pub(super) fn take_queued(
    queued: Vec3,
    chest: Vec3,
    target: Vec3,
    facing: Vec3,
    scale: f32,
    landings: i32,
) -> Option<super::feet::QueuedStep> {
    if queued == Vec3::ZERO {
        return None;
    }
    super::feet::queued_step(chest, target, queued, facing, scale, landings)
}

/// What one tick's jolt asks of the body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Jolt {
    /// Where the standing position moves (m), once per episode.
    pub standing: Option<Vec3>,
}

/// The jolt a body takes from its chest's velocity: the difference
/// between its fast and slow chest velocities across the ground less the
/// decaying term (which first loses 15 %), compared with `2.5·scale` cm a
/// tick in the walking condition and `125·scale` in any other (a thousand
/// times either while it has no support; the original's jump flag has no
/// Exarust counterpart). Above it the step ends (`Some`), and once per
/// episode the standing position moves to the feet's centre plus 100 ×
/// the jolt, at most 100·scale cm away, and the behaviour is told while
/// the body locomotes. Each jolt above the threshold holds the latch for
/// 80 more ticks; the latch clears on the quiet tick after the countdown
/// reaches 0. A body off its feet takes no jolt and loses its latch.
#[allow(clippy::too_many_arguments)]
pub(super) fn jolt(
    fast: Vec3,
    slow: Vec3,
    steps: &mut Steps,
    walking: bool,
    locomoting: bool,
    off_feet: bool,
    unsupported: bool,
    scale: f32,
    feet_centre: Vec3,
) -> Option<Jolt> {
    steps.jolt_told = false;
    steps.jolt_decay *= 0.85;
    let jolt = (fast - slow).with_y(0.0) - steps.jolt_decay;
    if off_feet {
        steps.jolt_latch = false;
        return None;
    }
    let base = if walking { 2.0 } else { 100.0 } * 1.25;
    let base = if unsupported { base * 1000.0 } else { base };
    let limit = base * scale;
    let jolt2 = cm(jolt).length_squared();
    if limit * limit < jolt2 {
        let standing = if steps.jolt_latch {
            None
        } else {
            steps.jolt_latch = true;
            steps.jolt_told = locomoting;
            let offset = if scale * scale < jolt2 {
                cm(jolt) * (100.0 * scale / jolt2.sqrt())
            } else {
                cm(jolt) * 100.0
            };
            Some((cm(feet_centre) + offset) / CM)
        };
        steps.jolt_quiet = 80;
        Some(Jolt { standing })
    } else {
        steps.jolt_quiet -= 1;
        if steps.jolt_quiet < 0 {
            steps.jolt_quiet = 0;
            steps.jolt_latch = false;
        }
        None
    }
}

/// What one footwork pass reads. Positions are metres.
pub(super) struct Work {
    pub now_ms: f32,
    pub scale: f32,
    pub crouch: f32,
    /// The score's chest reference, the frame origin and the stance origin.
    pub reference: Vec3,
    pub root: Vec3,
    pub origin: Vec3,
    /// The character frame's horizontal side axis, which the stance
    /// score's side term is measured along.
    pub side: Vec3,
    pub step_cm: f32,
    pub episode: bool,
    pub episode_ms: f32,
    /// The chest's horizontal distance to the step target (cm).
    pub chest_to_target_cm: f32,
    pub size: f32,
    /// The body's stand-still stamps (ms), which the footwork writes back
    /// as feet start and land.
    pub still_at: f32,
    pub still_plus: f32,
}

impl Work {
    /// A standing pass with the reference at the origin: the feet tests.
    #[cfg(test)]
    pub(super) fn plain(scale: f32) -> Self {
        Self {
            now_ms: 0.0,
            scale,
            crouch: 0.0,
            reference: Vec3::ZERO,
            root: Vec3::ZERO,
            origin: Vec3::ZERO,
            side: Vec3::X,
            step_cm: step_length(scale, 1.0),
            episode: false,
            episode_ms: 0.0,
            chest_to_target_cm: f32::MAX,
            size: 1.0,
            still_at: f32::NAN,
            still_plus: f32::NAN,
        }
    }
}

/// What the choice reads besides the two feet. Positions are centimetres.
pub(super) struct ScoreCtx {
    pub now_ms: f32,
    pub scale: f32,
    pub crouch: f32,
    pub reference: Vec3,
    pub side: Vec3,
    pub root: Vec3,
    pub origin: Vec3,
    pub step: f32,
    pub episode: bool,
    pub size: f32,
    /// Stand-still stamps (ms), as the footwork's landing left them.
    pub still_at: f32,
    pub still_plus: f32,
}

/// The footwork's choice for two feet: the highest of stay, foot 0 and
/// foot 1. Ties keep the earlier one, so `None` is stay.
pub(super) fn pick(feet: &[FootView; 2], spots: &[Vec3; 2], ctx: &ScoreCtx) -> Option<usize> {
    let stay = stance_score(
        feet[0].point,
        feet[1].point,
        ctx.reference,
        ctx.side,
        ctx.crouch,
        ctx.scale,
    ) - 1.0;
    let mut best = stay;
    let mut chosen = None;
    for i in 0..2 {
        let score = foot_score(i, feet, spots, ctx);
        if score > best {
            best = score;
            chosen = Some(i);
        }
    }
    chosen
}

/// A foot's anchor (m), kept within 1.8 step lengths of the other foot.
pub(super) fn clamp_anchor(anchor: Vec3, other: Vec3, step_cm: f32) -> Vec3 {
    let limit = ANCHOR_LENGTHS * step_cm / CM;
    let d = (anchor - other).with_y(0.0);
    if !limit.is_finite() || d.length() <= limit || limit <= 0.0 {
        anchor
    } else {
        Vec3::new(other.x, anchor.y, other.z) + d.normalize_or_zero() * limit
    }
}

/// Where the step-end routine puts the standing position: the chest plus
/// 5 ticks of the fast velocity.
pub(super) fn end_query(chest: Vec3, fast: Vec3) -> Vec3 {
    chest + fast * END_LEAD
}

/// What the thorax lead reads besides the step. `fast` is metres a tick.
/// `side` is the character frame's horizontal side axis.
#[derive(Clone, Copy)]
pub(super) struct ChestLead {
    pub idle: bool,
    pub level: f32,
    pub scale: f32,
    pub fast: Vec3,
    pub side: Vec3,
    /// 1.2 for a recovery step, 1 for a knockback step.
    pub kind: f32,
}

/// A thorax pin's posed target while a step leads it. The shift is the
/// horizontal error from the pin's world point to the posed target plus
/// the step offset, times the clip share, the direction term, the speed
/// term, the recovery kind, the square root of body scale and the stance
/// level.
pub(super) fn lead_pose(
    posed: Vec3,
    target: Vec3,
    standing: Vec3,
    at: Vec3,
    lead: &ChestLead,
) -> Vec3 {
    let error = (posed + (target - standing) - at).with_y(0.0);
    let error_cm = cm(error);
    let h2 = error_cm.length_squared();
    if h2 == 0.0 {
        return posed;
    }
    let g = if lead.idle { LEAD_IDLE } else { LEAD_GAIN };
    let s = if h2 < LEAD_TINY {
        0.0
    } else {
        (cm(lead.fast).dot(error_cm) / h2).max(0.0)
    };
    let forwardness = 1.0 - lead.side.dot(error_cm / h2.sqrt()).abs();
    let direction = (h2 / LEAD_DIR_OVER + LEAD_DIR_BIAS) * LEAD_DIR_WEIGHT * forwardness;
    let factor = g
        * (direction + LEAD_DIR_LIFT)
        * (LEAD_SPEED_BASE - LEAD_SPEED * s)
        * lead.kind
        * LEAD_POOL
        * lead.scale.max(0.0).sqrt()
        * lead.level;
    posed + error * factor
}

fn foot_score(i: usize, feet: &[FootView; 2], spots: &[Vec3; 2], ctx: &ScoreCtx) -> f32 {
    let off = (spots[i] - feet[i].anchor).with_y(0.0).length();
    let tolerance = if feet[i].stalled && !ctx.episode {
        STALLED_FROM
    } else {
        STEP_FROM
    } * ctx.scale;
    if !feet[i].anchored && off < tolerance {
        return STAY_SCORE;
    }
    let score = if feet[i].anchored {
        anchored_base(ctx.still_plus, ctx.still_at, ctx.now_ms)
    } else {
        MOVE_SCORE
    };
    score + common(i, feet, spots, ctx)
}

/// An anchored foot's base, from the body's two stand-still stamps:
/// `plus` (a landing's time plus 100 ms, a stalled landing's time, or the
/// trailing plant's clock plus 100 ms) and `at` (when a foot last started
/// moving, or the trailing plant's clock). Under `plus` plus 100 ms the
/// base is 11.5, else under `at` plus 300 ms it is 3.5, else under `plus`
/// plus 600 ms it is 2, else 1.5. Each bound is strict, and a stamp that
/// is not a number (never written, or closed by a step's start) opens no
/// band.
pub(super) fn anchored_base(plus: f32, at: f32, now: f32) -> f32 {
    if now < plus + FRESH_MS {
        MOVE_SCORE + FRESH_BONUS
    } else if now < at + EARLY_MS {
        MOVE_SCORE + EARLY_BONUS
    } else if now < plus + LATE_MS {
        MOVE_SCORE + LATE_BONUS
    } else {
        MOVE_SCORE
    }
}

fn common(i: usize, feet: &[FootView; 2], spots: &[Vec3; 2], ctx: &ScoreCtx) -> f32 {
    let mut score = 0.0;
    let window = CHANGE_MS * ctx.size.max(1.0);
    if recent(feet[i].changed_ms, ctx.now_ms, window) {
        score -= FRESH_BONUS;
    }
    let g = (ctx.root * ROOT_BLEND + ctx.origin * ORIGIN_BLEND - feet[i].anchor).with_y(0.0);
    score += reach_penalty(g.length(), feet[i].leg, ctx.step);
    let (a, b) = if i == 0 {
        (spots[0], feet[1].point)
    } else {
        (feet[0].point, spots[1])
    };
    score + stance_score(a, b, ctx.reference, ctx.side, ctx.crouch, ctx.scale)
}

/// The reach penalty added to a foot's score. Lengths are centimetres.
pub(super) fn reach_penalty(g_cm: f32, leg_cm: f32, step_cm: f32) -> f32 {
    if g_cm > REACH_SHARE * leg_cm {
        let extra = (g_cm - REACH_FROM * step_cm) * REACH_WEIGHT;
        extra * extra
    } else {
        0.0
    }
}

fn recent(stamp: f32, now: f32, window: f32) -> bool {
    stamp.is_finite() && now >= stamp && now - stamp < window
}

#[cfg(test)]
#[path = "body_steps_test.rs"]
mod tests;
