//! The carry of a living body, as the original's: what holds it up and
//! where it stands, in one vector that every limb particle takes in place
//! of gravity.
//!
//! The character tick resets the carry to gravity, then a pelvis-height
//! servo (the lift) takes g·f·S off it, f growing as the pelvis sinks
//! under its target height: f = scale·0.075(h + 15) standing, at most 1.5,
//! or scale·(0.4·run + 0.2)(h + 5) moving, at most 2, h the target less the
//! pelvis height (cm). The scale product is taken in double and narrowed
//! once, before the clamp. f is then damped by the pelvis's vertical speed
//! and scaled by the tone over a half. Each particle of a part whose tone is over a half then
//! moves by g(1 − a) + carry·a: a = tone × the part's support share
//! (pelvis and thighs 1, lower legs 0.5, feet 0.25) or tone × energy (the
//! upper body). Standing at f = 1, 1.7 cm over its target, a body floats on
//! its legs; without support the carry is gravity.
//!
//! The target is the lowest foot's ground plus the clip root's height over
//! the ankle and the foot's ground offset (its sole height standing),
//! blended with the other foot's, capped at the ground plus sole plus leg
//! length, plus 5 cm. The blend's weight is a half, all of it when the
//! lowest foot is lifted and none when the other is. Moving, that half is
//! min(0.5, 0.2·speed) and the weight is multiplied by min(1, (0.3·speed)²).
//! A negative slow vertical chest speed v (cm a tick) multiplies it once
//! more by max(0, 1 + 0.5·v); a rise, a hold and a NaN leave it. The
//! spread is (1 − k)·lowest ground + k·(lowest contact + other contact
//! − other ground), k = min(1, 0.2·speed) moving and 1 standing.
//! A full crouch is 60 cm times the body scale over that ground. The rules
//! are the original's,
//! in its centimetres and ticks (evidence: `docs/formats/RSQ.md`, Physical
//! body). Each foot's ground is its own: the controller's floor does not
//! raise it. The leg bend stores the lift term before clamping and lowers
//! an unanchored target by twice that (`bend`).
//! (Included from body.rs: `#[path = "body_lift.rs"] mod lift;`)

use bevy::math::Affine3A;
use bevy::prelude::*;

use super::balance::cm;
use super::feet;
use super::{BODY_HZ, Bearing, CM, GRAVITY, Ragdoll};

/// Standing, f = 0.075 per cm of h + 15, at most 1.5, times the body scale.
const LIFT_PER_CM: f64 = 0.075;
/// Standing leads (cm): the plain 15, 10 while a foot is stepping and 20
/// for a dormant statue or golem (the stepping test comes first).
const STANDING_LEAD: f32 = 15.0;
const STEPPING_LEAD: f32 = 10.0;
const DORMANT_LEAD: f32 = 20.0;
const MAX_STANDING_LIFT: f32 = 1.5;
/// Moving, f = (0.4·run + 0.2)(h + 5), at most 2, times the body scale.
const RUN_LIFT: f64 = 0.4;
const WALK_LIFT: f64 = 0.2;
const MOVING_LEAD: f32 = 5.0;
const MAX_MOVING_LIFT: f32 = 2.0;
/// The damping takes (1 − |f|/(|f| + 2))·E²·v_y off f.
const DAMPING_SPREAD: f32 = 2.0;
/// Crouched deeper than this a standing body's f may go down to −10;
/// otherwise a negative f leaves the carry at gravity.
const DEEP_CROUCH: f64 = 0.9;
const MIN_CROUCHED_LIFT: f32 = -10.0;
/// Over this tone a part takes the carry, and the lift scales by twice the
/// tone above it.
const TONE_FLOOR: f32 = 0.5;
/// A crouching body's target is this high over its ground at full crouch
/// (cm), times the body scale.
const CROUCHED_HEIGHT: f32 = 60.0;
/// The other foot's target is blended in with this weight, by all of it
/// when the lowest foot is lifted and by none when the other is (that
/// flag wins when both are), by none once the feet's spread (cm) reaches
/// the second, and by all of the weight under the first; between, by 0.05
/// a cm times the weight. Moving, the half is min(0.5, 0.2·speed) and the
/// weight is then multiplied by min(1, (0.3·speed)²). The spread is
/// (1 − k)·lowest ground + k·(lowest contact + other contact − other
/// ground), k = min(1, 0.2·speed) moving and 1 standing.
const STANDING_BLEND: f32 = 0.5;
/// A lifted lowest foot hands the blend this weight before the slope.
const LIFTED_LOWEST: f32 = 1.0;
/// Moving, the base weight is this share of the speed along the heading
/// (cm a tick), and the whole weight is multiplied by the square of the
/// second share. Both are taken in double and narrowed once.
const SPEED_BASE: f64 = 0.2;
const SPEED_SQUARE: f64 = 0.3;
/// A negative slow vertical chest speed (cm a tick) multiplies the moving
/// blend by max(0, 1 + this × the speed). Zero, a rise and a NaN do not.
const DROP_PER_CM: f32 = 0.5;
const BLEND_FULL: f32 = 40.0;
const BLEND_NONE: f32 = 60.0;
const BLEND_SLOPE: f64 = 0.05;
/// A standing target is raised this much (cm) once capped.
const TARGET_RAISE: f32 = 5.0;
/// A foot is the lowest only with ground under this height over the
/// pelvis (cm).
const LOWEST_ABOVE: f32 = 100.0;
/// The target when no foot has ground (cm): no lift.
pub(super) const NO_TARGET: f32 = -100_000.0;

/// Gravity per tick (m a tick²): what each particle falls by.
pub(super) fn gravity_per_tick() -> Vec3 {
    let dt = 1.0 / BODY_HZ;
    Vec3::Y * GRAVITY * dt * dt
}

/// The share of the carry a part of `bone` takes, of its tone: the pelvis
/// and thighs 1, the lower legs 0.5, the feet 0.25; none for the upper
/// body, which takes its energy instead.
pub(super) fn support_share(bone: usize) -> Option<f32> {
    match bone {
        1 | 2 | 5 => Some(1.0),
        3 | 6 => Some(0.5),
        4 | 7 => Some(0.25),
        _ => None,
    }
}

/// A body larger than this, in double precision, takes no carry on its
/// pelvis or legs.
const LARGE_BODY: f64 = 1.3;

/// How much of the carry a part of `bone` takes at `tone` and `energy`:
/// a = tone × its support share, or tone × energy; none at a tone of a
/// half or less (it falls by gravity alone). A body of `scale` over 1.3
/// zeroes the pelvis and leg shares.
pub(super) fn carry_share(bone: usize, tone: f32, energy: f32, scale: f32) -> Option<f32> {
    if tone.partial_cmp(&TONE_FLOOR) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    let share = match support_share(bone) {
        Some(_) if f64::from(scale) > LARGE_BODY => 0.0,
        Some(share) => share,
        None => energy,
    };
    Some(tone * share)
}

/// The lift term before clamping, in centimetres: standing,
/// scale·0.075(h + 15), or with 10 while a foot is stepping and 20 for a
/// dormant body; moving, scale·(0.4·run + 0.2)(h + 5). `scale` is the
/// body scale. The product is taken in double and narrowed once. The leg
/// bend reads this number.
pub(super) fn lift_unclamped(h: f32, bearing: &Bearing, scale: f32) -> f32 {
    let lead = if bearing.moving {
        MOVING_LEAD
    } else if bearing.stepping {
        STEPPING_LEAD
    } else if bearing.dormant {
        DORMANT_LEAD
    } else {
        STANDING_LEAD
    };
    let coeff = if bearing.moving {
        f64::from(bearing.run) * RUN_LIFT + WALK_LIFT
    } else {
        LIFT_PER_CM
    };
    (f64::from(scale) * (coeff * f64::from(h + lead))) as f32
}

/// The lift f before the tone, in the original's cm and ticks: `h` the
/// target less the pelvis height, `rise` the pelvis's vertical displacement
/// over the last tick, `energy` E, `scale` the body scale. Standing,
/// f = scale·0.075(h + 15), none when negative (but down to −10 crouched
/// deeper than 0.9), at most 1.5; moving, f = scale·(0.4·run + 0.2)(h + 5),
/// none when negative, at most 2. The scale product is a double narrowed
/// once, before that clamp. Then f −= (1 − |f|/(|f| + 2))·E²·rise.
pub(super) fn lift_factor(
    h: f32,
    rise: f32,
    energy: f32,
    bearing: &Bearing,
    scale: f32,
) -> Option<f32> {
    let raw = lift_unclamped(h, bearing, scale);
    let f = if bearing.moving {
        if raw < 0.0 || raw.is_nan() {
            return None;
        }
        raw.min(MAX_MOVING_LIFT)
    } else if f64::from(bearing.crouch) <= DEEP_CROUCH {
        if raw < 0.0 || raw.is_nan() {
            return None;
        }
        raw.min(MAX_STANDING_LIFT)
    } else {
        let f = raw.max(MIN_CROUCHED_LIFT);
        f.min(MAX_STANDING_LIFT)
    };
    let spread = f.abs() / (f.abs() + DAMPING_SPREAD);
    Some(f - (1.0 - spread) * energy * energy * rise)
}

/// One leg as the target height reads it, in cm: its ground (none out of
/// reach), its contact's height, the clip root's height over its animated
/// ankle plus its ground offset, its length, and whether its foot is
/// lifted for the swing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LegHeight {
    pub ground: Option<f32>,
    pub contact: f32,
    pub stance: f32,
    pub length: f32,
    /// The foot contact's lifted bit: the swing, only pushed up.
    pub lifted: bool,
}

/// The target pelvis height (cm) from the first two legs, crouching by
/// `crouch`, with the feet's sole height `sole` and the pelvis at `pelvis`:
/// each leg's is crouch·(60·scale) + (1 − crouch)·stance + ground, `scale`
/// the body scale. It is the lowest foot L's, blended toward the other
/// O's by half, by 1 when L is lifted and by 0 when O is (O's flag wins
/// when both are; 0 as well when O has no ground). Moving, that half is
/// min(0.5, 0.2·`speed`) and the weight is then multiplied by
/// min(1, (0.3·`speed`)²), `speed` the cm a tick along the heading.
/// A negative slow vertical chest speed `drop` (cm a tick) multiplies
/// that weight by max(0, 1 + 0.5·`drop`); zero, a rise and a NaN leave
/// it, and standing ignores it.
/// The spread is (1 − k)·L's ground + k·(L's contact + O's contact − O's
/// ground), k = min(1, 0.2·`speed`) moving and 1 standing. The weight
/// applies in full while that spread is under 40 cm, by 0.05 a cm of it
/// less over 40, not at all from 60. Standing, it is
/// capped at L's ground plus sole plus leg length and raised 5 cm; moving,
/// neither. [`NO_TARGET`] when no foot has ground.
#[allow(clippy::too_many_arguments)]
pub(super) fn target_height(
    legs: &[LegHeight],
    sole: f32,
    crouch: f32,
    pelvis: f32,
    moving: bool,
    scale: f32,
    speed: f32,
    drop: f32,
) -> f32 {
    let mut lowest = pelvis + LOWEST_ABOVE;
    let mut low = None;
    for (i, leg) in legs.iter().take(2).enumerate() {
        if let Some(g) = leg.ground
            && g < lowest
        {
            lowest = g;
            low = Some(i);
        }
    }
    let Some(i) = low else {
        return NO_TARGET;
    };
    let crouched = CROUCHED_HEIGHT * scale;
    let value =
        |leg: &LegHeight, ground: f32| crouched * crouch + (1.0 - crouch) * leg.stance + ground;
    let l = &legs[i];
    let mut target = value(l, lowest);
    if let Some(o) = legs.get(1 - i) {
        // The other's lifted flag is read second, so it wins when both
        // feet are in the swing. A foot with no ground is not blended.
        // Moving, the half is min(0.5, 0.2·speed) before those flags.
        let base = if moving {
            ((f64::from(speed) * SPEED_BASE) as f32).min(STANDING_BLEND)
        } else {
            STANDING_BLEND
        };
        let (weight, ground) = match o.ground {
            Some(g) if o.lifted => (0.0, g),
            Some(g) if l.lifted => (LIFTED_LOWEST, g),
            Some(g) => (base, g),
            None => (0.0, lowest),
        };
        // k mixes the contact spread toward the lowest foot's ground.
        // Standing, and a moving speed at or over 5 cm a tick, k is 1, so
        // the spread stays the contact test. A NaN speed saturates k.
        let mix = if moving {
            ((f64::from(speed) * SPEED_BASE) as f32).min(1.0)
        } else {
            1.0
        };
        let lowest_mixed = (1.0 - mix) * lowest + mix * l.contact;
        let other_mixed = (1.0 - mix) * ground + mix * o.contact;
        let spread = other_mixed - (ground - lowest_mixed);
        if spread < BLEND_NONE {
            let mut weight = if spread > BLEND_FULL {
                (f64::from((BLEND_NONE - spread) * weight) * BLEND_SLOPE) as f32
            } else {
                weight
            };
            if moving {
                let scaled = f64::from(speed) * SPEED_SQUARE;
                weight *= ((scaled * scaled) as f32).min(1.0);
                // A chest on the way down gives the other foot less say.
                // Zero, a rise and a NaN compare as not under zero.
                if drop < 0.0 {
                    weight *= (drop * DROP_PER_CM + 1.0).max(0.0);
                }
            }
            target = value(o, ground) * weight + (1.0 - weight) * target;
        }
    }
    if !moving {
        target = target.min(lowest + sole + l.length) + TARGET_RAISE;
    }
    target
}

impl Ragdoll {
    /// The character tick's carry, once the body is aimed: gravity, less
    /// the lift. Each leg's ground offset is set (its sole height standing;
    /// moving, its animated ankle's height over the gait pose's lowest,
    /// plus the sole), then the target pelvis height from the ground under the feet
    /// (each foot's own; the controller's floor does not raise it) and the
    /// clip root's height over each ankle, then the lift servo, with
    /// support only. The pre-clamp term is kept for the leg bend.
    pub(super) fn lift(&mut self) {
        let measured = self.measures_balance();
        let root = self.parts.iter().position(|p| p.parent.is_none());
        let Ragdoll {
            parts,
            legs,
            drive,
            sole,
            ..
        } = self;
        let Some(drive) = drive.as_mut() else {
            return;
        };
        let gravity = gravity_per_tick();
        drive.carry = gravity;
        drive.lift = 0.0;
        drive.lift_target = None;
        let (Some(root), true) = (root, measured) else {
            return;
        };
        let bearing = drive.bearing;
        let posed = drive.targets.len() == parts.len();
        let pelvis_target = posed.then(|| Vec3::from(drive.targets[root].translation));
        let floor = drive.root.map(|r: Affine3A| r.translation.y);
        let heights: Vec<LegHeight> = legs
            .iter_mut()
            .map(|leg| {
                let ankle = posed.then(|| drive.targets[leg.shin].transform_point3(leg.point));
                // The original's ground offset: the sole height standing;
                // moving, the ankle's height over the gait pose's lowest.
                leg.offset = match (bearing.moving, bearing.gait_floor, ankle, floor) {
                    (true, Some(lowest), Some(a), Some(f)) => a.y - f - lowest + *sole,
                    _ => *sole,
                };
                let over_ankle = pelvis_target.zip(ankle).map_or(0.0, |(p, a)| p.y - a.y);
                LegHeight {
                    ground: leg.ground.map(cm),
                    contact: cm(parts[leg.shin].point(&leg.contact).y),
                    stance: cm(over_ankle + leg.offset),
                    length: cm(leg.length),
                    lifted: leg.state.has(feet::FootState::LIFTED),
                }
            })
            .collect();
        let pelvis = &parts[root];
        let height = cm(pelvis.transform().translation.y);
        // The legs' extension: the pelvis over the highest anchor, against
        // the pose's pelvis over its mean ankle.
        if let Some(p) = pelvis_target {
            let ankles: f32 = legs
                .iter()
                .map(|l| drive.targets[l.shin].transform_point3(l.point).y)
                .sum();
            let reach = cm(p.y - ankles / legs.len() as f32);
            let highest = legs.iter().map(|l| l.anchor.y).fold(f32::MIN, f32::max);
            let b = &mut drive.balance;
            b.extension = feet::extension(b.level, height - cm(highest), reach);
        }
        let target = target_height(
            &heights,
            cm(*sole),
            bearing.crouch,
            height,
            bearing.moving,
            drive.stats.scale,
            bearing.speed,
            cm(drive.balance.slow_velocity().y),
        );
        drive.lift_target = Some(target / CM);
        if !drive.balance.has_support() {
            return;
        }
        let h = target - height;
        // Kept even when the clamped factor is none: the bend reads it.
        let scale = drive.stats.scale;
        drive.lift_raw = lift_unclamped(h, &bearing, scale);
        let w = pelvis.own_origin;
        let rise: f32 = (0..4)
            .map(|k| (pelvis.positions[k].y - pelvis.previous[k].y) * w[k])
            .sum();
        let Some(f) = lift_factor(h, cm(rise), drive.energy, &bearing, scale) else {
            return;
        };
        if drive.tone > TONE_FLOOR {
            let mut f = 2.0 * (drive.tone - TONE_FLOOR) * f;
            // A collapsing body's lift fades by its tone squared too: gone
            // at half its tone.
            if bearing.collapsing {
                f *= drive.tone * drive.tone;
            }
            drive.lift = f;
            drive.carry -= gravity * (f * drive.support);
        }
        // The locomotion drive, as the original adds it to the carry while
        // the body moves: drive × S² × the gait's push gain × the feet's
        // lag, in cm a tick². The feet routine measured the lag from last
        // tick's animated contacts; a body that does not walk keeps 1.
        if bearing.moving && drive.support > 0.0 {
            let push =
                bearing.push * (drive.support * drive.support * bearing.push_gain * bearing.lag);
            drive.carry += push / CM;
        }
    }
}

#[cfg(test)]
#[path = "body_lift_test.rs"]
mod tests;

#[cfg(test)]
#[path = "body_climb_test.rs"]
mod climb_tests;
