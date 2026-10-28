//! Balance and support of a living body, measured in its character tick as
//! the original measures them, before the body is aimed.
//!
//! The balance point is the chest: the mean of the thorax pins on the
//! body's particles. Its velocity is smoothed twice a tick. The support
//! point lies on the segment between the two foot contacts (the ankles),
//! and a standing body's is moved halfway to the feet centre. The lean is
//! the unit offset from the support point to the chest (led by a landing's
//! momentum, by the gait, and back by a crouch); its height u_y gives the
//! score q = u_y⁸ standing, u_y⁴ moving, plus 0.05 × the race's balance
//! bonus. An instability counter grows while q ≤ 0.99 and sets the balance
//! to q once it reaches 100; below 100 balance recovers slowly.
//!
//! The stance spots the feet read are the clip's ankles turned from the
//! character frame's yaw onto the stance facing, keeping their height.
//! Then the feet's states are set (`feet`), and each leg takes its
//! friction from its foot's state, asks the level how high the ground is
//! under its foot (the Height Query: a walking sphere of 10 cm widened by
//! the gait up to 20 cm and swayed along the facing, 5 cm standing),
//! anchors its foot at that ground plus the sole height, and is grounded
//! when its contact is within 20 cm of it (30 cm times the body scale while
//! moving), its ground offset included (its sole height standing; moving,
//! its ankle's lift in the walk as well). Support S is grounded × balance,
//! and a standing body with no
//! foot on the ground has no balance. The stance's level follows how much
//! higher one foot stands than the other. Without support it asks for the
//! ground under its pelvis (the fall check reads it, and a body getting up
//! near it has support).
//!
//! The rules are the original's, in its centimetres and ticks (evidence:
//! `docs/formats/RSQ.md`, Physical body).
//! (Included from body.rs: `#[path = "body_balance.rs"] mod balance;`)

use std::ops::Mul;

use bevy::math::Affine3A;
use bevy::prelude::*;

use super::feet::{self, FootState};
use super::steps::{self, Plan};
use super::{CM, Part, PartSetup, Ragdoll, weights};
use crate::contact::Level;
use crate::engine_math;

/// The chest's velocity follows its motion by these shares a tick: fast
/// and slow.
const FAST_SMOOTHING: f32 = 0.4;
const SLOW_SMOOTHING: f32 = 0.1;
/// On regaining support a standing chest leads by its slow velocity times
/// a weight that starts at 5 and fades 0.05 a tick.
const LANDING_WEIGHT: f32 = 5.0;
const LANDING_FADE: f64 = 0.05;
/// A moving body with support leads by this many ticks of its fast
/// velocity.
const MOVING_LEAD: f32 = 3.0;
/// A crouch takes the chest back by this much (cm) at full crouch.
const CROUCH_BACK: f32 = 10.0;
/// Standing, the support point moves this share of the way to the feet
/// centre.
const TOWARD_CENTRE: f32 = 0.5;
/// The score is the lean's height to this power standing, and moving.
const STANDING_POWER: i32 = 8;
const MOVING_POWER: i32 = 4;
/// Each unit of the race's balance bonus adds this to the score.
const BONUS_SCORE: f64 = 0.05;
/// A slow vertical chest speed (cm/tick) above this sets the counter to
/// 100 and, grounded, cuts the score by (1 − 0.333(v − 3))², to nothing
/// above 6.
const VERTICAL_FROM: f32 = 3.0;
const VERTICAL_TO: f32 = 6.0;
const VERTICAL_SLOPE: f64 = 0.333;
/// A score above this is steady.
const STEADY: f64 = 0.99;
/// At this count balance is the score; standing still and steady the
/// counter falls by 5 to no less than 80, otherwise by 10 to 0.
const COUNTER_FULL: i32 = 100;
const COUNTER_FLOOR: i32 = 80;
const STILL_EASE: i32 = 5;
const EASE: i32 = 10;
/// Below a full count balance recovers by (5 − v)·0.01 a tick at a slow
/// chest speed v under 5 cm/tick, 0.05 at rest (v² at most this, cm²).
const RECOVERY_SPEED: f32 = 5.0;
const RECOVERY_RATE: f64 = 0.01;
const RECOVERY_AT_REST: f32 = 0.05;
const AT_REST2: f32 = 1.192_09e-7;
/// Getting up, balance rises this much a tick and the counter falls by 5
/// while above 2.
const GET_UP_RISE: f64 = 0.05;
const GET_UP_EASE_ABOVE: i32 = 2;
/// Each leg's Height Query (cm): 5 cm (× scale) along the lower leg's
/// forward axis from its contact, starting 70 cm above the contact, a
/// sphere (`feet::probe_radius`: 10 cm walking, 5 cm and more otherwise)
/// swept 2000 cm down. While the body is down, lying or getting up, it
/// starts at the contact.
const PROBE_AHEAD: f32 = 5.0;
const PROBE_ABOVE: f32 = 70.0;
const PROBE_DEPTH: f32 = 2000.0;
/// With support the character's ground is the lowest foot's, from the
/// pelvis plus this (cm) down.
const LOWEST_ABOVE: f32 = 100.0;
/// The ground a query that meets nothing gives (cm).
const NO_GROUND: f32 = -10000.0;
/// Ground is within reach at least this share of the leg's length under
/// the pelvis.
const REACH_SHARE: f32 = 0.5;
/// A foot is grounded with its contact at most this high (cm) above its
/// ground and sole height. Standing is this, flat. Moving is this times
/// the body scale.
const GROUNDED_STANDING: f32 = 20.0;
/// The moving window before the body scale.
const GROUNDED_MOVING: f32 = 30.0;
/// Without support the ground under the pelvis (the character's ground)
/// is asked from 20 cm above it with a 20 cm sphere; a body getting up
/// within 30 cm of it has support at half its balance.
const PELVIS_ABOVE: f32 = 20.0;
const PELVIS_RADIUS: f32 = 20.0;
const GET_UP_HEIGHT: f32 = 30.0;
const GET_UP_SUPPORT: f32 = 0.5;
/// The sole height is the depth of the foot's box under its origin less
/// this (cm).
const SOLE_INSET: f32 = 0.5;
/// A walking foot's swing clearance decays by this (cm) a tick while it
/// touches the ground (in double precision).
const SWING_DECAY: f64 = 0.3;
/// A run blend under this (compared in double) makes the late foot's
/// swing threshold rise by the gait's ground difference.
const WALK_RUN_BELOW: f64 = 0.01;

/// Metres (or metres per tick) in the original's centimetres: every rule
/// below is written in cm.
pub(super) fn cm<T: Mul<f32, Output = T>>(x: T) -> T {
    x * CM
}

/// A pin on a part: the part and the point's weights on its particles.
pub(super) type Pin = (usize, [f32; 4]);

/// A leg as the balance reads it: a lower leg carrying a foot contact.
#[derive(Debug, Clone)]
pub struct Leg {
    /// The lower leg's part.
    pub shin: usize,
    /// The foot contact in the lower leg's space (m): its foot's origin,
    /// the ankle, when the leg has a foot part.
    pub point: Vec3,
    pub(super) contact: [f32; 4],
    /// Hip to knee plus knee to contact (m).
    pub length: f32,
    /// How high (m) its contact stands over its ground when the foot is
    /// down: the feet's sole height standing; moving, also its animated
    /// ankle's height over the gait pose's lowest (set by the lift).
    pub offset: f32,
    /// The ground under the foot this tick (m), smoothed (it rises at once
    /// and falls by the gait's share, at once outside the walk), or none
    /// while it is not within reach.
    pub ground: Option<f32>,
    /// The foot stands on its ground.
    pub grounded: bool,
    /// The foot contact's state: touching, anchored, lifted, …
    pub state: FootState,
    /// Where the foot is held (m, world): across the ground where it was
    /// planted, its height its ground plus the sole height.
    pub anchor: Vec3,
    /// The contact's world point in the solver's last iteration, and in
    /// the one before.
    pub(super) at: Vec3,
    pub(super) before: Vec3,
    /// An anchored foot's smoothed approach to its anchor (cm an
    /// iteration; 1 while it stands).
    pub approach: f32,
    /// How firmly a standing foot grips its anchor, 0 to 1: it rises 0.001
    /// an iteration while the foot touches, and is 0 while it is anchored.
    pub grip: f32,
    /// The lower leg's grip ramp, 0 to 0.25: the walking servo's gain.
    pub ramp: f32,
    /// How far an anchored foot's ground probe has moved toward its anchor.
    pub(super) blend: f32,
    /// The smoothed ground (m), kept while the ground is out of reach.
    pub(super) smoothed: f32,
    /// The leg's foot part, if it has one.
    pub foot: Option<usize>,
    /// The foot's pitch (rad) toward the ground, and the rate the smoother
    /// keeps: at most 0.01 rad a tick plus the rate, the rate then half of
    /// that sum within ±0.1.
    pub pitch: f32,
    pub pitch_rate: f32,
    /// The toe: the foot's sole point 20 cm ahead of the ankle, as weights
    /// on the foot part.
    pub(super) toe: [f32; 4],
    /// The floor the toe is held over (m).
    pub floor: f32,
    /// When the footwork last changed this foot's state (ms): it landed,
    /// stalled or reached its spot. Not a number until it has happened.
    pub(super) changed_ms: f32,
    /// The knee and hip corrections the next tick's pose takes (rad), and
    /// the rates the smoother carries (rad a tick).
    pub(super) knee: f32,
    pub(super) knee_rate: f32,
    pub(super) hip: f32,
    pub(super) hip_rate: f32,
    /// The thigh's extra turn toward a step. Identity when it is not swung.
    pub(super) turn: Quat,
    /// The step lift (m), added to the reach. Negative while a foot rises.
    pub(super) lift: f32,
    /// The foot planted this tick (the walk's gait plant), which the
    /// footstep sound follows.
    pub planted: bool,
    /// The clip's hip-to-ankle offset (m) before this tick's bend.
    pub(super) posed_offset: Vec3,
    /// Where the clip's ankle and hip are (world, m), as the locomotion IK
    /// reads them before the corrections are folded in.
    pub(super) anim_contact: Vec3,
    pub(super) anim_hip: Vec3,
    /// The bend target last solved (m).
    pub(super) goal: Vec3,
    /// The unsmoothed knee of the reach triangle (rad).
    pub(super) reach_knee: f32,
    /// How far (cm) the contact is lowered before the locomotion IK reads
    /// it: the original's per-leg lag term. The standing IK clears it every
    /// tick, so it stays 0 until the feet routine raises it.
    pub(super) contact_lag: f32,
    /// How far (cm) the foot stands over its ground (or, planted, over its
    /// sole floor), which the foot pitch levels, as the IK wrote it.
    pub(super) pitch_height: f32,
    /// The IK's corrections are the identity this tick while the stored
    /// angles ease back to zero (the original resets a part's correction
    /// quaternion before its angle smoother reaches zero).
    pub(super) knee_shown: bool,
    pub(super) hip_shown: bool,
    /// The length of the step this foot started (cm), which the footstep
    /// sound's loudness follows: 0.01 + 0.0139 × it up to 140 cm, else 1.
    pub step_length_cm: f32,
}

/// A living body's balance, kept from tick to tick.
#[derive(Debug, Clone)]
pub struct Balance {
    /// 1 when steady, 0 when lost.
    pub value: f32,
    counter: i32,
    /// The chest point last tick and the tick before (m).
    chest: Option<Vec3>,
    before: Option<Vec3>,
    /// The chest's smoothed velocity (m a tick): fast and slow.
    fast: Vec3,
    slow: Vec3,
    /// The landing momentum's weight.
    landing: f32,
    /// The unit offset from the support point to the chest, and its length
    /// (m).
    lean: Vec3,
    offset: f32,
    /// A foot stood on the ground last tick.
    grounded: bool,
    has_support: bool,
    /// Ticks without support.
    airborne: u32,
    /// The ground under the pelvis (m), asked while no foot gives support.
    ground: Option<f32>,
    /// The character's ground (m): with support the lowest foot's (their
    /// mean while walking), without it the ground under the pelvis. A foot
    /// with no ground in reach is anchored at its height.
    under: f32,
    /// The pelvis's height (m) when the feet were last measured.
    pub(super) pelvis: f32,
    /// The stance's level, 1 on level ground: less while one foot stands
    /// higher than the other.
    pub(super) level: f32,
    /// The legs' extension factor: the level times how far the pelvis
    /// stands over the highest anchor, as a share of how far the pose's
    /// stands over its mean ankle, squared (at most 1).
    pub(super) extension: f32,
    /// Feet that reached their anchors.
    landings: u32,
    /// The mean of the foot contacts (m), the feet centre the support
    /// point and the stance-origin clamps read.
    feet_centre: Vec3,
    /// The support point (m): the chest led by landings, gait and crouch,
    /// projected onto the feet's segment (halfway to their centre while it
    /// does not move).
    support_point: Vec3,
    /// The imbalance accumulator: how far past the step trigger
    /// the chest has leaned, 0 to 1, decaying 0.01 a tick in the feet
    /// routine. No caller of the original reads it; kept as the rule.
    imbalance: f32,
    /// The walking feet's stored signed height difference (cm), smoothed
    /// toward the difference the clip's phase picks.
    gait_height_cm: f32,
    /// The two feet's grounds (cm) as last tick left them, which the walk's
    /// height difference is read from before this tick's feet move.
    gait_grounds_cm: [f32; 2],
}

impl Default for Balance {
    fn default() -> Self {
        Self {
            value: 1.0,
            counter: 0,
            chest: None,
            before: None,
            fast: Vec3::ZERO,
            slow: Vec3::ZERO,
            landing: 0.0,
            lean: Vec3::Y,
            offset: 0.0,
            grounded: true,
            has_support: true,
            airborne: 0,
            ground: None,
            under: 0.0,
            pelvis: 0.0,
            level: 1.0,
            extension: 1.0,
            landings: 0,
            feet_centre: Vec3::ZERO,
            support_point: Vec3::ZERO,
            imbalance: 0.0,
            gait_height_cm: 0.0,
            gait_grounds_cm: [0.0; 2],
        }
    }
}

impl Balance {
    /// The instability counter, 0 to 100.
    pub fn counter(&self) -> i32 {
        self.counter
    }

    /// Sets the instability counter, as an effect that throws the body off
    /// balance does (a force barricade sets it to 100).
    pub fn set_counter(&mut self, counter: i32) {
        self.counter = counter;
    }

    /// The unit offset from the support point to the chest.
    pub fn lean(&self) -> Vec3 {
        self.lean
    }

    /// The offset's length (m).
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// The chest point (m), once measured.
    pub fn chest(&self) -> Option<Vec3> {
        self.chest
    }

    /// Keeps the chest point and smooths its fast and slow velocity from
    /// where it was last tick and the tick before.
    pub fn record_chest(&mut self, now: Vec3) {
        self.follow_chest(now);
    }

    /// The chest's slow smoothed velocity (m a tick).
    pub fn slow_velocity(&self) -> Vec3 {
        self.slow
    }

    /// The chest's fast smoothed velocity (m a tick).
    pub fn fast_velocity(&self) -> Vec3 {
        self.fast
    }

    /// A foot stands on the ground.
    pub fn has_support(&self) -> bool {
        self.has_support
    }

    /// Ticks the body has gone without support.
    pub fn airborne(&self) -> u32 {
        self.airborne
    }

    /// Gives the landing momentum its full weight, as a walking body that
    /// loses its balance gets it (the movement decision's).
    pub(super) fn land_fully(&mut self) {
        self.landing = LANDING_WEIGHT;
    }

    /// The ground under the pelvis (m; 100 m down when the query meets
    /// nothing within its 20 m), as last asked without the feet's support;
    /// none while a foot gives it.
    pub fn ground(&self) -> Option<f32> {
        self.ground
    }

    /// The stance's level: 1 on level ground, less while one foot stands
    /// higher than the other.
    pub fn level(&self) -> f32 {
        self.level
    }

    /// The legs' extension factor, which scales the feet's grip: 1 with
    /// the legs as straight as the pose's.
    pub fn extension(&self) -> f32 {
        self.extension
    }

    /// How many feet have reached their anchors.
    pub fn landings(&self) -> u32 {
        self.landings
    }

    /// The mean of the foot contacts (m).
    pub fn feet_centre(&self) -> Vec3 {
        self.feet_centre
    }

    /// The support point (m): where the lean is measured from.
    pub fn support_point(&self) -> Vec3 {
        self.support_point
    }

    /// The imbalance accumulator, 0 to 1.
    pub fn imbalance(&self) -> f32 {
        self.imbalance
    }

    /// One tick of the imbalance accumulator, from the chest's horizontal
    /// error past the step trigger.
    pub(super) fn add_imbalance(&mut self, h_cm: f32, radius_cm: f32, factor: f32, scale: f32) {
        self.imbalance = steps::add_imbalance(self.imbalance, h_cm, radius_cm, factor, scale);
    }

    /// The feet routine's decay of the imbalance accumulator.
    pub(super) fn decay_imbalance(&mut self) {
        self.imbalance = steps::decay_imbalance(self.imbalance);
    }

    /// Counts a foot that reached its anchor.
    pub(super) fn land(&mut self) {
        self.landings += 1;
    }

    /// One tick of the counter and the balance for score `q`, at a slow
    /// chest speed² `slow2` (cm² a tick²), standing still or not: q ≤ 0.99
    /// adds round(10 − 10q); a steady score lowers the counter by 5 to no
    /// less than 80 standing still, else by 10 to 0. At 100 balance is q;
    /// below it balance recovers.
    pub(super) fn settle(&mut self, q: f32, slow2: f32, still: bool) {
        // Only a score over 0.99 is steady: a NaN takes the gain, whose
        // 64-bit conversion keeps 0 in its low word (Rust's cast gives 0).
        let steady = f64::from(q) > STEADY;
        self.counter = if !steady {
            self.counter + counter_gain(q)
        } else if still {
            (self.counter - STILL_EASE).max(COUNTER_FLOOR)
        } else {
            (self.counter - EASE).max(0)
        };
        if self.counter >= COUNTER_FULL {
            self.value = q;
            self.counter = COUNTER_FULL;
        } else if let Some(rise) = recovery(slow2) {
            // The cap compares in single precision, so a NaN is kept.
            let v = self.value + rise;
            self.value = if v > 1.0 { 1.0 } else { v };
        }
    }

    /// One tick of getting up: balance rises by 0.05, the counter falls by
    /// 5 while above 2.
    pub(super) fn get_up(&mut self) {
        // The rise is the double 0.05; the cap keeps a NaN.
        let v = (f64::from(self.value) + GET_UP_RISE) as f32;
        self.value = if v > 1.0 { 1.0 } else { v };
        if self.counter > GET_UP_EASE_ABOVE {
            self.counter -= STILL_EASE;
        }
    }

    /// Smooths the chest's velocity from where it was last tick and the
    /// tick before, then keeps where it is now.
    fn follow_chest(&mut self, now: Vec3) {
        if let Some(last) = self.chest {
            let d = last - self.before.unwrap_or(last);
            self.before = Some(last);
            self.fast += (d - self.fast) * FAST_SMOOTHING;
            self.slow += (d - self.slow) * SLOW_SMOOTHING;
        }
        self.chest = Some(now);
    }
}

/// The score of a lean whose unit offset has height `uy`: u_y⁸ standing,
/// u_y⁴ moving, plus 0.05 × the race's balance bonus. The sum is taken in
/// double and narrowed once; the cap compares in single precision, so a
/// NaN is kept.
pub(super) fn score(uy: f32, moving: bool, bonus: f32) -> f32 {
    let power = if moving { MOVING_POWER } else { STANDING_POWER };
    let q = uy.powi(power);
    let sum = (f64::from(bonus) * BONUS_SCORE + f64::from(q)) as f32;
    if sum > 1.0 { 1.0 } else { sum }
}

/// The score at a slow vertical chest speed `v` (cm a tick): a grounded
/// body's is cut by (1 − 0.333(v − 3))² above 3 and is 0 above 6.
pub(super) fn vertical_cut(q: f32, v: f32, grounded: bool) -> f32 {
    if v <= VERTICAL_FROM || !grounded {
        q
    } else if v <= VERTICAL_TO {
        let f = 1.0 - f64::from(v - VERTICAL_FROM) * VERTICAL_SLOPE;
        (f * f * f64::from(q)) as f32
    } else {
        0.0
    }
}

/// What a score of at most 0.99 adds to the counter: 10 − 10q, rounded half
/// to even (the processor's rounding), so nothing until q < 0.95.
pub(super) fn counter_gain(q: f32) -> i32 {
    (10.0 - q * 10.0).round_ties_even() as i32
}

/// Balance regained in a tick at a slow chest speed² `slow2` (cm² a
/// tick²): (5 − v)·0.01 under 5 cm a tick, 0.05 at rest; none from 5.
pub(super) fn recovery(slow2: f32) -> Option<f32> {
    if slow2 >= RECOVERY_SPEED * RECOVERY_SPEED || slow2.is_nan() {
        None
    } else if slow2 <= AT_REST2 {
        Some(RECOVERY_AT_REST)
    } else {
        Some((f64::from(RECOVERY_SPEED - slow2.sqrt()) * RECOVERY_RATE) as f32)
    }
}

/// Whether ground at `ground` (cm) is within a leg's reach: at least half
/// the leg's `length` under the pelvis at `pelvis` (cm), or always while
/// getting up.
pub(super) fn within_reach(ground: f32, pelvis: f32, length: f32, getting_up: bool) -> bool {
    // The original's compare jumps to the "within reach" branch when the
    // height is unordered, so a NaN ground is in reach.
    getting_up || ground.is_nan() || ground <= pelvis - REACH_SHARE * length
}

/// Whether a foot whose contact stands `height` (cm) above its ground and
/// ground offset is grounded. Standing, within 20 cm. Moving, within 30 cm
/// times the body `scale`. A NaN height, or a NaN window while moving,
/// stays grounded: the original's compare keeps the foot when unordered.
pub(super) fn foot_grounded(height: f32, moving: bool, scale: f32) -> bool {
    let window = if moving {
        GROUNDED_MOVING * scale
    } else {
        GROUNDED_STANDING
    };
    height <= window || height.is_nan() || window.is_nan()
}

/// The closest point to `m` on the segment `a`–`b`, the parameter taken in
/// the horizontal plane, the point keeping the contacts' heights.
fn support_point(m: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let e = Vec3::new(b.x - a.x, 0.0, b.z - a.z);
    let along = e.length_squared();
    let t = if along > 0.0 {
        ((m - a).dot(e) / along).clamp(0.0, 1.0)
    } else {
        0.0
    };
    a + (b - a) * t
}

/// The chest pins, legs and sole height of a body built from `setup`. The
/// first part with pins carries the chest. Every part with a foot contact
/// is a leg, in part order; the original's muscle setup moves the contact
/// to the origin of the leg's foot part, the ankle, which is also where a
/// supported leg's reach is measured from. The sole height is the depth of
/// that foot's box under its origin, less 0.5 cm.
pub(super) fn build(setup: &[PartSetup], parts: &mut [Part]) -> (Vec<Pin>, Vec<Leg>, f32) {
    let chest = setup
        .iter()
        .enumerate()
        .find(|(_, s)| !s.pins.is_empty())
        .map(|(i, s)| s.pins.iter().map(|&p| (i, weights(&parts[i], p))).collect())
        .unwrap_or_default();
    let mut legs = Vec::new();
    let mut sole = 0.0;
    for (i, s) in setup.iter().enumerate() {
        let Some(stored) = s.foot else {
            continue;
        };
        let foot = setup
            .iter()
            .position(|f| f.parent == Some(i) && matches!(f.bone, 4 | 7));
        let point = foot.map_or(stored, |j| setup[j].origin);
        let mut toe = [0.0; 4];
        if let Some(j) = foot {
            let f = &setup[j];
            sole = f.half.y - f.centre.y - SOLE_INSET / CM;
            toe = weights(&parts[j], feet::toe(sole, 1.0));
        }
        parts[i].foot = Some(point);
        let contact = weights(&parts[i], point);
        let at = parts[i].point(&contact);
        // The clip's ankle as last sampled (the feet routine's `+0x30`):
        // the model's own pose until the first sample writes one.
        let anim_hip = parts[i]
            .parent
            .map_or(Vec3::ZERO, |p| Vec3::from(parts[p].transform().translation));
        legs.push(Leg {
            shin: i,
            point,
            contact,
            length: s.origin.length() + point.length(),
            offset: 0.0,
            ground: None,
            grounded: true,
            state: FootState::default(),
            anchor: at,
            at,
            before: at,
            approach: 0.0,
            grip: 0.0,
            ramp: 0.0,
            blend: 0.0,
            smoothed: 0.0,
            foot,
            toe,
            floor: 0.0,
            changed_ms: f32::NAN,
            knee: 0.0,
            knee_rate: 0.0,
            hip: 0.0,
            hip_rate: 0.0,
            turn: Quat::IDENTITY,
            lift: 0.0,
            planted: false,
            posed_offset: Vec3::ZERO,
            anim_contact: at,
            anim_hip,
            goal: Vec3::ZERO,
            reach_knee: 0.0,
            contact_lag: 0.0,
            pitch_height: 0.0,
            knee_shown: true,
            hip_shown: true,
            step_length_cm: 0.0,
            pitch: 0.0,
            pitch_rate: 0.0,
        });
    }
    for leg in &mut legs {
        leg.offset = sole;
    }
    (chest, legs, sole)
}

impl Ragdoll {
    /// Whether the body's balance is measured: a living body with two legs.
    /// Any other keeps support 1 while it stands, 0 once fallen.
    pub(super) fn measures_balance(&self) -> bool {
        self.drive.is_some() && self.legs.len() >= 2
    }

    /// The legs the balance reads.
    pub fn legs(&self) -> &[Leg] {
        &self.legs
    }

    /// The feet's landing count in the running step episode.
    pub fn landings(&self) -> u32 {
        self.drive.as_ref().map_or(0, |d| d.balance.landings)
    }

    /// The feet's sole height (m).
    pub fn sole(&self) -> f32 {
        self.sole
    }

    /// Leg `leg`'s foot contact (its ankle) where the particles are now.
    pub fn ankle(&self, leg: usize) -> Option<Vec3> {
        let l = self.legs.get(leg)?;
        Some(self.parts[l.shin].point(&l.contact))
    }

    /// The chest point on the body's particles: the mean of its thorax
    /// pins, or its root part's origin without pins.
    pub fn chest_point(&self) -> Option<Vec3> {
        if self.chest.is_empty() {
            let root = self.parts.iter().find(|p| p.parent.is_none())?;
            return Some(root.transform().translation.into());
        }
        let sum: Vec3 = self
            .chest
            .iter()
            .map(|(i, w)| self.parts[*i].point(w))
            .sum();
        Some(sum / self.chest.len() as f32)
    }

    /// The balance and the feet in the character tick, before the body is
    /// aimed, as the original measures them: the chest's velocity from its
    /// last two points, the chest point, the lean against the support point,
    /// the score, counter and balance (or a get-up's rise); then the feet's
    /// states (planted, lifted, on their stance spots: `feet::plant`), and
    /// for each leg its friction by its foot's state, its ground from a
    /// Height Query against `level`, smoothed, its foot's anchor height and
    /// its toe's floor; the grounded feet, support and balance without
    /// ground, S = grounded × balance, the stance's level, and without
    /// support the ground under the pelvis.
    pub fn measure_balance<L: Level + ?Sized>(&mut self, level: &L) {
        if !self.measures_balance() {
            return;
        }
        let Some(chest) = self.chest_point() else {
            return;
        };
        let Some(pelvis) = self.parts.iter().find(|p| p.parent.is_none()) else {
            return;
        };
        let pelvis = Vec3::from(pelvis.transform().translation);
        let contacts: Vec<Vec3> = self
            .legs
            .iter()
            .map(|l| self.parts[l.shin].point(&l.contact))
            .collect();
        let ahead: Vec<Vec3> = self
            .legs
            .iter()
            .map(|l| self.parts[l.shin].transform().matrix3.z_axis.into())
            .collect();
        let sole = self.sole;
        let Ragdoll {
            parts,
            legs,
            drive,
            chest: chest_pins,
            ..
        } = self;
        let Some(drive) = drive.as_mut() else {
            return;
        };
        let bearing = drive.bearing;
        // The walk's ground difference this tick, read from last tick's
        // grounds before this tick's feet move: the late foot's swing
        // threshold adds its magnitude.
        let gait_gap = feet::gait_gap(
            bearing.progress,
            drive.balance.gait_grounds_cm[0],
            drive.balance.gait_grounds_cm[1],
        );
        // The locomotion lag factor: each walking leg whose contact trails
        // its animated spot cuts the push, from 1.
        let mut foot_lag = 1.0;
        let bonus = drive.race.balance_bonus;
        let scale = drive.stats.scale;
        // Down: lying, or getting up (a collapsing body, unsupported as it
        // is, still probes its feet standing).
        let down = (!drive.supported && !bearing.collapsing) || bearing.getting_up;
        // Only a standing body rides its controller.
        let carried = bearing.carried && drive.supported;
        let forward = drive.root.map_or(Vec3::Z, |r| {
            Vec3::from(r.matrix3.z_axis)
                .with_y(0.0)
                .normalize_or(Vec3::Z)
        });
        // Where last tick's pose put each ankle: its stance spot, turned
        // from the character frame's yaw onto the stance facing.
        let posed = drive.targets.len() == parts.len();
        let mut spots: Vec<Option<Vec3>> = legs
            .iter()
            .map(|l| posed.then(|| drive.targets[l.shin].transform_point3(l.point)))
            .collect();
        if drive.stance_yaw.is_finite()
            && let Some(frame) = drive.frame
        {
            let origin = Vec3::from(frame.translation);
            let frame_yaw = horizontal_yaw(frame);
            let stance_yaw = drive.stance_yaw;
            for spot in spots.iter_mut().flatten() {
                *spot = stance_spot(*spot, origin, frame_yaw, stance_yaw);
            }
        }
        let b = &mut drive.balance;
        b.follow_chest(chest);

        // The lean, against the support point.
        let centre = contacts.iter().copied().sum::<Vec3>() / contacts.len() as f32;
        b.feet_centre = centre;
        let mut m = chest;
        // The landing's momentum fades by the double 0.05 a tick.
        if f64::from(b.landing) > LANDING_FADE {
            if !bearing.moving {
                m += b.slow * b.landing;
            }
            b.landing = (f64::from(b.landing) - LANDING_FADE) as f32;
        }
        if bearing.moving && b.has_support {
            m += b.fast * MOVING_LEAD;
        }
        m -= forward * (CROUCH_BACK / CM * bearing.crouch);
        let mut p = support_point(m, contacts[0], contacts[1]);
        if !bearing.moving {
            p = p.lerp(centre, TOWARD_CENTRE);
        }
        b.support_point = p;
        let o = m - p;
        b.offset = o.length();
        if b.offset > 0.0 {
            b.lean = o / b.offset;
        }
        let h_cm = cm(o.with_y(0.0)).length();
        let was_stepping = drive.steps.stepping;
        if bearing.getting_up {
            b.get_up();
        } else {
            let mut q = score(b.lean.y, bearing.moving, bonus);
            let v = cm(b.slow.y).abs();
            if v > VERTICAL_FROM {
                q = vertical_cut(q, v, b.grounded);
                b.counter = COUNTER_FULL;
            }
            // Standing still is not moving and not already stepping.
            b.settle(
                q,
                cm(b.slow).length_squared(),
                !bearing.moving && !was_stepping,
            );
            // The imbalance accumulator, once the chest has left the step
            // trigger: the support pins' power E⁴S scales the radius. A
            // shock stagger and a body under 0.2 (a double compare a NaN
            // passes) do not accumulate.
            let fallen = f64::from(b.value) < 0.2;
            if !fallen && b.has_support && !bearing.shock_stagger {
                let factor = drive.energy.powi(4) * drive.support;
                let r = steps::radius(
                    drive.energy,
                    drive.steps.radius_base,
                    scale,
                    drive.steps.stepping,
                    bonus,
                );
                if (r * 1.5 * factor) * (r * 1.5 * factor) < h_cm * h_cm {
                    b.add_imbalance(h_cm, r, factor, scale);
                }
            }
        }

        // The jolt: the chest's fast velocity less its slow one, less the
        // decaying term. Above its threshold the step ends and, once per
        // episode, the standing position moves to the feet's centre plus
        // 100 × the jolt.
        {
            let steps_state = &mut drive.steps;
            if let Some(jolt) = steps::jolt(
                b.fast,
                b.slow,
                steps_state,
                bearing.walking,
                bearing.moving,
                bearing.off_feet,
                !b.has_support,
                scale,
                b.feet_centre,
            ) {
                steps_state.stepping = false;
                if let Some(standing) = jolt.standing {
                    steps_state.standing = standing;
                    steps_state.placed = true;
                    steps_state.still_at = steps_state.clock as f32 * steps::MS_PER_TICK - 500.0;
                }
            }
        }

        // The recovery step, from this tick's balance and last tick's
        // support, before the feet are moved.
        let fast = b.fast;
        let frame = drive.anchor.unwrap_or(centre);
        let trigger = steps::Trigger {
            balance: b.value,
            supported: b.has_support,
            getting_up: bearing.getting_up,
            locomoting: bearing.moving || bearing.walking,
            down: bearing.off_feet || !b.has_support,
            h_cm,
            chest,
            fast,
            energy: drive.energy,
            support: drive.support,
            scale,
            bonus,
            reach: drive.stats.foot_reach(),
            frame,
            landings: b.landings,
            shock_stagger: bearing.shock_stagger,
        };
        let posed_spots = spots.clone();
        let plan = steps::consider(&mut drive.steps, &trigger);
        let origin = match plan {
            Plan::Stepping(origin) => Some(origin),
            Plan::Ended => {
                end_step(
                    &mut drive.steps,
                    legs,
                    &posed_spots,
                    &contacts,
                    frame,
                    chest,
                    fast,
                );
                None
            }
            Plan::Idle => None,
        };
        if let Some(origin) = origin {
            for spot in spots.iter_mut().flatten() {
                let local = *spot - frame;
                *spot = Vec3::new(origin.x + local.x, spot.y, origin.z + local.z);
            }
        }
        let pin_mean = (posed && !chest_pins.is_empty()).then(|| {
            let n = chest_pins.len() as f32;
            chest_pins
                .iter()
                .map(|(i, w)| drive.targets[*i].transform_point3(parts[*i].rest_point(w)))
                .sum::<Vec3>()
                / n
        });
        let stance_origin = origin.unwrap_or(frame);
        let side = drive
            .frame
            .map_or(Vec3::X, |f| Vec3::from(f.matrix3.x_axis).with_y(0.0));
        let mut work = steps::Work {
            now_ms: drive.steps.clock as f32 * steps::MS_PER_TICK,
            scale,
            crouch: bearing.crouch,
            reference: steps::score_reference(
                chest,
                pin_mean,
                drive.steps.stepping,
                fast,
                forward,
                bearing.crouch,
            ),
            root: frame,
            origin: stance_origin,
            side,
            step_cm: steps::foot_step_length(drive.stats.foot_reach(), bearing.crouch),
            episode: drive.steps.stepping,
            episode_ms: if drive.steps.stepping {
                steps::elapsed_ms(&drive.steps)
            } else {
                0.0
            },
            chest_to_target_cm: cm((chest - drive.steps.target).with_y(0.0)).length(),
            size: drive.stats.size.max(1.0),
            still_at: drive.steps.still_at,
            still_plus: drive.steps.still_plus,
        };
        // The feet's states, from last tick's support.
        let met: Vec<bool> = legs
            .iter()
            .map(|l| !parts[l.foot.unwrap_or(l.shin)].contacts.is_empty())
            .collect();
        let moves: Vec<Vec3> = legs
            .iter()
            .map(|l| parts[l.shin].slip(&l.contact))
            .collect();
        let foot_ended = feet::plant(
            legs,
            &contacts,
            &spots,
            &moves,
            &bearing,
            trigger.supported,
            &met,
            &mut work,
        );
        // A footwork landing stamps the stand-still clocks the next
        // score reads, as the trailing-foot plant does.
        drive.steps.still_at = work.still_at;
        drive.steps.still_plus = work.still_plus;
        if drive.steps.stepping {
            let dist = cm((chest - drive.steps.target).with_y(0.0)).length();
            let toward = cm(fast.with_y(0.0))
                .dot((drive.steps.target - chest).with_y(0.0).normalize_or_zero());
            let any = legs.iter().any(|l| l.state.has(FootState::ANCHORED));
            drive.steps.near = steps::note_near(drive.steps.near, dist, scale);
            let landed = b.landings.saturating_sub(drive.steps.landed_from);
            if foot_ended
                || steps::episode_over(
                    steps::elapsed_ms(&drive.steps),
                    any,
                    dist,
                    toward,
                    drive.steps.near,
                    landed,
                    scale,
                )
            {
                end_step(
                    &mut drive.steps,
                    legs,
                    &posed_spots,
                    &contacts,
                    frame,
                    chest,
                    fast,
                );
            }
        }

        // A queued footwork step, taken at the landing after the episode
        // that was running when it was requested (the original's balance
        // routine calls its queued-step taker there). The AI is told when
        // the step points along the facing.
        let landed = b.landings != drive.steps.landings_seen;
        drive.steps.landings_seen = b.landings;
        drive.steps.told_ai = false;
        if landed && drive.steps.queued != Vec3::ZERO {
            // The queued step is taken through the body's own rule, so its
            // target and its notice to the AI are one place.
            let queued = drive.steps.queued;
            drive.steps.queued = Vec3::ZERO;
            let landings = if b.landings < 1 { 0 } else { 1 };
            if let Some(step) =
                steps::take_queued(queued, chest, drive.steps.target, forward, scale, landings)
            {
                drive.steps.target = step.target;
                drive.steps.stepping = true;
                drive.steps.started = drive.steps.clock;
                drive.steps.near = 0;
                drive.steps.landed_from = b.landings;
                drive.steps.told_ai = step.tell_ai;
            } else {
                drive.steps.queued = queued;
            }
        }

        // The feet.
        b.pelvis = pelvis.y;
        let under = b.under;
        let mut grounded = 0;
        for (i, ((leg, &c), &axis)) in legs.iter_mut().zip(&contacts).zip(&ahead).enumerate() {
            let friction = feet::leg_friction(leg.state);
            parts[leg.shin].friction = friction;
            if let Some(f) = leg.foot {
                parts[f].friction = friction;
            }
            // The per-leg lag: how far the contact trails its animated spot
            // along the facing, in cm. The original measures the animated
            // contact in the character frame against the physical one, and
            // negates the lead while the walk is backwards; a lead that is
            // not ahead (or is a NaN) is none. A lead inside 60·size cuts
            // the push by 1 − (lead/60·size)², and one at or past it ends
            // the push.
            let lead = if bearing.walking && !down {
                let mut along = cm(leg.anim_contact - c).dot(bearing.facing);
                if drive.gait_back {
                    along = -along;
                }
                if along > 0.0 { along } else { 0.0 }
            } else {
                0.0
            };
            if bearing.walking && !down && lead > 0.0 {
                let limit = 60.0 * drive.stats.size;
                if lead < limit {
                    foot_lag *= 1.0 - (lead / limit) * (lead / limit);
                } else {
                    foot_lag = 0.0;
                }
            }
            // A walking foot's probe is at its ankle, swayed along the
            // facing, its sphere 10 cm widened by the gait up to 20. Any
            // other's is at its ankle, or toward its anchor while anchored,
            // 5 cm wide and more so.
            leg.blend = feet::probe_blend(leg);
            let gait = (bearing.walking && !down).then(|| {
                feet::gait_probe(
                    i,
                    drive.gait_progress,
                    scale,
                    drive.gait_speed,
                    drive.gait_back,
                )
            });
            let mut probe = if bearing.walking {
                c
            } else {
                c.lerp(leg.anchor, leg.blend)
            };
            if let Some(gait) = &gait {
                probe += forward * (gait.sway / CM);
            }
            probe += axis * (PROBE_AHEAD * scale / CM);
            let radius = gait
                .as_ref()
                .map_or_else(|| feet::probe_radius(leg.blend), |g| g.radius);
            probe.y = c.y + if down { 0.0 } else { PROBE_ABOVE / CM };
            let found = level
                .height_below(probe, radius / CM, PROBE_DEPTH / CM)
                .map_or(NO_GROUND, cm);
            if within_reach(found, cm(pelvis.y), cm(leg.length), bearing.getting_up) {
                leg.state.0 &= !FootState::NO_GROUND;
                // A walk falls by the gait's share; any other ground at once.
                let fall = gait.as_ref().map_or(1.0, |g| g.fall);
                let ground = feet::smooth_ground(cm(leg.smoothed), found, fall);
                leg.smoothed = ground / CM;
                leg.ground = Some(leg.smoothed);
                leg.anchor.y = feet::anchor_height(ground, cm(sole), cm(c.y)) / CM;
                // The sole's floor: the original's Ground Query is a ray
                // against floor-flagged meshes only, which no Exarust
                // level has, so it misses and the floor is the ground plus
                // the leg's offset, never over the lower leg's origin.
                let knee = parts[leg.shin].transform().translation.y;
                let touching = leg.state.has(FootState::TOUCHING);
                leg.floor =
                    feet::sole_floor(ground, touching, leg.contact_lag, None, cm(knee)) / CM;
                if bearing.walking && !down && touching {
                    // Locomoting and touching: the swing clearance decays
                    // by the double 0.3 a tick, the step lift is none, and
                    // the late foot's swing flag is set by its lead. The
                    // threshold is 10·scale×(0.25 − frac(0.5·leg + progress
                    // + 1)); at a walk it first rises by the gait's ground
                    // difference to at least 25 cm. The flag is set while
                    // the lead times the heading alignment is over it.
                    leg.contact_lag =
                        engine_math::max((f64::from(leg.contact_lag) - SWING_DECAY) as f32, 0.0);
                    let phase =
                        super::locomotion::fraction(i as f32 * 0.5 + (bearing.progress + 1.0));
                    let mut threshold = (10.0 * scale) * (0.25 - phase);
                    if f64::from(bearing.run) < WALK_RUN_BELOW {
                        threshold = engine_math::max(threshold + gait_gap.abs(), 25.0);
                    }
                    if threshold < lead * bearing.align {
                        leg.state.0 |= FootState::LIFTED;
                    } else {
                        leg.state.0 &= !FootState::LIFTED;
                    }
                    leg.lift = 0.0;
                } else {
                    // Any other foot is not lifted; a walking foot that
                    // does not touch swings with its gait.
                    leg.state.0 &= !FootState::LIFTED;
                }
            } else {
                leg.state.0 |= FootState::LIFTED | FootState::NO_GROUND;
                leg.anchor.y = under;
                leg.ground = None;
            }
            leg.grounded = leg
                .ground
                .is_some_and(|g| foot_grounded(cm(c.y - g - leg.offset), bearing.moving, scale));
            grounded += usize::from(leg.grounded);
        }
        // The push waits for the feet: the product of every walking leg's
        // lag cut, 1 when no lead is positive (or the body does not walk).
        drive.bearing.lag = foot_lag;
        if grounded > 0 {
            if !b.has_support {
                b.landing = LANDING_WEIGHT;
            }
            b.has_support = true;
            b.airborne = 0;
        } else {
            b.has_support = false;
            b.airborne += 1;
        }
        b.grounded = grounded > 0;
        // A standing body with no foot on the ground has no balance, unless
        // its character controller carries it (Exarust's own, until the
        // body stands on its feet).
        if !bearing.moving && !b.has_support && !carried {
            b.value = 0.0;
        }
        drive.support = if b.grounded { b.value } else { 0.0 };
        let b = &mut drive.balance;
        if b.has_support {
            let height = |l: &Leg| l.ground.unwrap_or(under);
            let (h0, h1) = (height(&legs[0]), height(&legs[1]));
            let d = contacts[0] - contacts[1];
            b.level = feet::level(b.level, cm(h0 - h1), cm(d.x), cm(d.z));
        }
        b.ground = None;
        // The walk's ground weights, from last tick's grounds, every tick:
        // a body moving under its own power blends them by the gait phase,
        // any other keeps 0.5 each and lets the stored difference fade.
        let [last0, last1] = b.gait_grounds_cm;
        let (gait_height, w0, w1) = feet::gait_weights(
            bearing.progress,
            bearing.run,
            bearing.moving,
            last0,
            last1,
            b.gait_height_cm,
        );
        b.gait_height_cm = gait_height;
        if !b.has_support {
            let found = level
                .height_below(
                    pelvis + Vec3::Y * (PELVIS_ABOVE / CM),
                    PELVIS_RADIUS / CM,
                    PROBE_DEPTH / CM,
                )
                .map_or(NO_GROUND, cm);
            b.ground = Some(found / CM);
            b.under = found / CM;
            if bearing.getting_up && cm(pelvis.y) < found + GET_UP_HEIGHT {
                b.has_support = true;
                drive.support = GET_UP_SUPPORT * b.value;
            }
        } else if !bearing.off_feet && !bearing.getting_up {
            let ground = |l: &Leg| l.ground.unwrap_or(under);
            let g0 = legs.first().map_or(under, ground);
            let g1 = legs.get(1).map_or(under, ground);
            if bearing.walking {
                // Walking, the character's ground is the two feet's
                // smoothed grounds blended by the gait's weights.
                b.under = g1 * w1 + g0 * w0;
            } else {
                // Stepping, the last anchored foot's ground; otherwise the
                // lowest.
                b.under = if bearing.stepping {
                    legs.iter()
                        .rev()
                        .find(|l| l.state.has(FootState::ANCHORED))
                        .map_or(under, ground)
                } else {
                    legs.iter()
                        .map(ground)
                        .fold(pelvis.y + LOWEST_ABOVE / CM, f32::min)
                };
            }
        }
        let ground = |l: &Leg| cm(l.ground.unwrap_or(under));
        b.gait_grounds_cm = [
            legs.first().map_or(cm(under), ground),
            legs.get(1).map_or(cm(under), ground),
        ];
        // The feet routine's decay of the imbalance accumulator.
        b.decay_imbalance();
    }
}

/// The episode ended: one support foot is put on its spot and the standing
/// position moves so that spot is where the foot is. `spots` are the pose's
/// ankles, not the step's shifted ones.
fn end_step(
    steps: &mut steps::Steps,
    legs: &mut [Leg],
    spots: &[Option<Vec3>],
    ankles: &[Vec3],
    frame: Vec3,
    chest: Vec3,
    fast: Vec3,
) {
    steps.stepping = false;
    steps.out_of_balance = false;
    steps.near = 0;
    let q = steps::end_query(chest, fast);
    let mut best: Option<(usize, f32)> = None;
    for (i, leg) in legs.iter().enumerate() {
        if leg.state.has(FootState::ANCHORED) {
            continue;
        }
        let Some(spot) = spots.get(i).copied().flatten() else {
            continue;
        };
        let want = q + (spot - frame).with_y(0.0);
        let d = (ankles[i] - want).with_y(0.0).length_squared();
        if best.as_ref().is_none_or(|(_, b)| d < *b) {
            best = Some((i, d));
        }
    }
    if let Some((i, _)) = best {
        let local = (spots[i].unwrap() - frame).with_y(0.0);
        if !legs[i].state.has(FootState::REACHED) {
            legs[i].anchor = ankles[i];
        }
        let keep = legs[i].state.0 & (FootState::RAMP_HELD | FootState::RAMP_SLOW);
        legs[i].state = FootState(FootState::TOUCHING | keep);
        steps.standing = ankles[i] - local;
        steps.placed = true;
    }
}

/// The yaw of a frame's horizontal forward. The model faces +Z, so a pure
/// yaw's forward is (sin yaw, 0, cos yaw).
fn horizontal_yaw(frame: Affine3A) -> f32 {
    let z = Vec3::from(frame.matrix3.z_axis).with_y(0.0);
    if z.length_squared() < 1e-12 {
        0.0
    } else {
        z.x.atan2(z.z)
    }
}

/// The footwork's stance spot: `spot` turned from the character frame's
/// yaw onto the stance facing, about `origin`, keeping its height. A yaw
/// that is not finite leaves the spot where the clip put it.
pub(super) fn stance_spot(spot: Vec3, origin: Vec3, frame_yaw: f32, stance_yaw: f32) -> Vec3 {
    if !(frame_yaw.is_finite() && stance_yaw.is_finite()) {
        return spot;
    }
    let flat = (spot - origin).with_y(0.0);
    let aimed = Quat::from_rotation_y(stance_yaw) * (Quat::from_rotation_y(-frame_yaw) * flat);
    Vec3::new(origin.x + aimed.x, spot.y, origin.z + aimed.z)
}

/// The heading a reset takes from the chest over the pelvis:
/// the chest minus the pelvis, normalised, flattened and
/// normalised again; zero when the two coincide (or the direction is
/// vertical). The original then stores it as the body's facing with no
/// pitch.
pub fn reset_heading(chest: Vec3, pelvis: Vec3) -> Vec2 {
    let d = chest - pelvis;
    let len = d.length();
    if len <= 0.0 {
        return Vec2::ZERO;
    }
    let n = d / len;
    Vec2::new(n.x, n.z).normalize_or_zero()
}

#[cfg(test)]
#[path = "body_balance_test.rs"]
mod tests;
