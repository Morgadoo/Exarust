//! The feet of a living body, as the original plants them: each leg's foot
//! contact (its ankle) has a state, an anchor where it is held and a grip,
//! set in the character tick and enforced in every solver iteration.
//!
//! In the character tick, before the feet are measured: a body whose
//! condition is not walking plants each foot that does not touch where it
//! is (touching, its anchor its ankle) once it meets the level; then,
//! standing (not moving), a body lying down has no foot touching, one
//! getting up (until nine tenths of its clip) or without support stands
//! each foot on its stance spot, where its clip puts its ankle (not
//! touching in a get-up's first fifth), and one standing with support
//! lands a foot that reached its anchor and steps at most one foot to its
//! stance spot.
//! While the feet are measured each leg's friction follows its foot's
//! state (0 not touching, 2.0 touching, 0 anchored, on its lower leg and
//! its foot), each foot's anchor height is its smoothed ground plus the
//! sole height (at most 1 m over the ankle), and its toe's floor is that
//! ground.
//!
//! In each iteration, a foot more than 10 cm under the pelvis, of a body
//! over half its tone with support: touching and not lifted, it is pulled
//! toward its anchor, by a velocity servo while walking (never down) and by
//! a PD grip otherwise, whose hold rises to full over 1000 iterations and
//! fades as the foot rises 1 to 9 cm over its anchor; an anchored foot
//! reaches its anchor within 5 cm; a foot not touching or lifted is only
//! pushed up. Each foot's toe is pushed up to its floor by at most 0.5 cm
//! an iteration (0.1 unless it stands), times S, then servoed sideways
//! toward where its animated orientation places it.
//!
//! The rules are the original's, in its centimetres and ticks (evidence:
//! `docs/formats/RSQ.md`, Physical body, Feet). The footwork chooses by
//! the stance score. A walking body's feet plant and lift by the walk's
//! phase. A foot is planted only once it meets the level when it is not
//! walking.
//! (Included from body.rs: `#[path = "body_feet.rs"] mod feet;`)

use bevy::prelude::*;

use super::balance::{Leg, cm};
use super::steps::{self, FootView, ScoreCtx, Work};
use super::{Bearing, CM, Drive, Part, Ragdoll};

/// A foot contact's state, as the original's bits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct FootState(pub u32);

impl FootState {
    /// On the ground: it grips its anchor, with friction.
    pub const TOUCHING: u32 = 0x2;
    /// Moving to a new anchor (a step), friction 0.
    pub const ANCHORED: u32 = 0x4;
    /// Its anchor reached, within 5 cm.
    pub const REACHED: u32 = 0x8;
    /// Lifted for the swing: only pushed up.
    pub const LIFTED: u32 = 0x10;
    /// No ground within reach.
    pub const NO_GROUND: u32 = 0x20;
    /// Its leg's grip ramp held at full.
    pub const RAMP_HELD: u32 = 0x40;
    /// Its leg's grip ramp rising ten times slower, until full.
    pub const RAMP_SLOW: u32 = 0x80;
    /// Its step stalled.
    pub const STALLED: u32 = 0x100;

    /// Whether any of `bits` is set.
    pub fn has(self, bits: u32) -> bool {
        self.0 & bits != 0
    }

    /// Touching and not lifted: it stands on its anchor.
    pub fn stands(self) -> bool {
        self.0 & (Self::TOUCHING | Self::LIFTED) == Self::TOUCHING
    }

    /// Anchored and its anchor not yet reached: a step, which widens the
    /// thorax pins' reach.
    pub(super) fn stepping(self) -> bool {
        self.0 & (Self::ANCHORED | Self::REACHED) == Self::ANCHORED
    }
}

/// A leg's friction touching the ground, not anchored.
const STANDING_FRICTION: f32 = 2.0;
/// A get-up's feet do not touch under this share of its clip, and stand on
/// their stance spots up to the second (both compared in double).
const GET_UP_UNTOUCHED: f64 = 0.2;
const GET_UP_FOOTWORK: f64 = 0.9;
/// An anchor is at most this high (cm) over its ankle.
const ANCHOR_ABOVE: f32 = 100.0;
/// A step whose smoothed approach falls under this has stalled.
const STALLED_BELOW: f64 = 1e-5;
/// An anchored foot's probe moves this share a tick toward its anchor, up
/// to the second; the probe's sphere is 5 cm plus 10 cm times that share
/// (a walking foot's is 10 cm).
const PROBE_BLEND: f64 = 0.05;
const PROBE_BLEND_MAX: f32 = 0.5;
const PROBE_RADIUS: f32 = 5.0;
const PROBE_RADIUS_BLEND: f32 = 10.0;
/// The stance's level follows 1 − v² by this share a tick (double).
const LEVEL_KEEP: f64 = 0.9;
const LEVEL_TAKE: f64 = 0.1;
/// The toe: 20 cm (× scale) ahead of the ankle along the foot, at the
/// sole's depth.
const TOE_AHEAD: f32 = 20.0;
/// The drive acts on a foot at least this far (cm) under the pelvis.
const UNDER_PELVIS: f32 = 10.0;
/// The drive acts on a body over this tone.
const TONE_FLOOR: f32 = 0.5;
/// An anchored foot reaches its anchor within this (cm, × scale).
const REACH: f32 = 5.0;
/// The approach speed keeps this share and takes the other (double).
const APPROACH_KEEP: f64 = 0.95;
const APPROACH_TAKE: f64 = 0.05;
/// The leg's grip ramp: +0.01 an iteration anchored, +0.001 touching,
/// +0.0001 rising slowly, at most 0.25; while moving the anchor follows the
/// ankle by 1 − 4 × the ramp.
const RAMP_ANCHORED: f64 = 0.01;
const RAMP_TOUCHING: f64 = 0.001;
const RAMP_SLOWLY: f64 = 0.0001;
const RAMP_MAX: f32 = 0.25;
const RAMP_FOLLOW: f32 = 4.0;
/// The grip rises 0.001 an iteration touching, to 1.
const GRIP_RISE: f64 = 0.001;
const GRIP_MAX: f32 = 1.0;
/// The servo's gain is (0.3·run + 0.2) times the ramp (touching) or the
/// grip × S (lifted, walking); its rate 0.2 (touching), 0.1 (lifted,
/// walking) or 0.4 (lifted) × scale; it leaves alone a point already
/// closing 0.05 of its gap.
const SERVO_RUN: f64 = 0.3;
const SERVO_WALK: f64 = 0.2;
const RATE_TOUCHING: f64 = 0.2;
const RATE_LIFTED_WALKING: f64 = 0.1;
const RATE_LIFTED: f64 = 0.4;
const SERVO_CLOSING: f64 = 0.05;
/// The PD grip: spring 0.1·p + 0.004·g(1 − p), damping p + 0.03·g(1 − p)
/// of the horizontal displacement; p is kept up to 1 cm over the anchor,
/// gone at 9 cm, and an anchor below is not pulled to.
const GRIP_SPRING: f64 = 0.1;
const SOFT_SPRING: f64 = 0.004;
const SOFT_DAMPING: f64 = 0.03;
const ABOVE_HELD: f32 = -1.0;
const ABOVE_GONE: f32 = -9.0;
const ABOVE_SLOPE: f32 = 0.125;
/// Squared gaps (cm²) the drive leaves alone.
const EPSILON2: f32 = 1.192_09e-7;
/// The sole push's caps (cm an iteration): standing, and otherwise (the
/// second compared in double).
const SOLE_STANDING: f32 = 0.5;
const SOLE_OTHERWISE: f64 = 0.1;
/// The sole turn closes this share of its horizontal gap a tick (× scale,
/// in double, then narrowed).
const SOLE_TURN_RATE: f64 = 0.4;

/// A leg's friction by its foot's state: none not touching, 2.0 touching,
/// none anchored (the anchor's pull moves it).
pub(super) fn leg_friction(state: FootState) -> f32 {
    if !state.has(FootState::TOUCHING) || state.has(FootState::ANCHORED) {
        0.0
    } else {
        STANDING_FRICTION
    }
}

/// The toe's point in the foot's space (m): 20 cm × `scale` ahead of the
/// ankle along the foot's forward axis, `sole` (m) under it.
pub(super) fn toe(sole: f32, scale: f32) -> Vec3 {
    Vec3::new(0.0, -sole, TOE_AHEAD * scale / CM)
}

/// The velocity servo, in cm: a point `moved` this tick with gap `d` to its
/// target already closes q = (moved·d)/|d|² of it; under 0.05 it is pushed
/// by d·(rate − q)·gain, so it closes `rate` of its gap a tick.
pub(super) fn servo(d: Vec3, moved: Vec3, gain: f32, rate: f32) -> Option<Vec3> {
    let len2 = d.length_squared();
    if len2.is_nan() || len2 < EPSILON2 {
        return None;
    }
    let q = moved.dot(d) / len2;
    (f64::from(q) < SERVO_CLOSING).then(|| d * ((rate - q) * gain))
}

/// The PD grip, in cm: gap `d` to the anchor, the point `moved` this tick,
/// gain `g` and hold `p`. Over the anchor the hold is kept within 1 cm,
/// fades to nothing at 9 cm, and the anchor is not pulled down to; the
/// push is d·(0.1p + 0.004g(1 − p)) less the horizontal motion times
/// (p + 0.03g(1 − p)). A full hold pulls a tenth of the gap and stops the
/// point across the ground: it grips.
pub(super) fn grip(d: Vec3, moved: Vec3, g: f32, p: f32) -> Option<Vec3> {
    let len2 = d.length_squared();
    if len2.is_nan() || len2 < EPSILON2 {
        return None;
    }
    let (mut d, mut p) = (d, p);
    if d.y < 0.0 {
        p = if d.y >= ABOVE_HELD {
            p
        } else if d.y >= ABOVE_GONE {
            (-ABOVE_GONE + d.y) * p * ABOVE_SLOPE
        } else {
            0.0
        };
        d.y = 0.0;
    }
    let soft = f64::from(1.0 - p) * f64::from(g);
    let spring = (f64::from(p) * GRIP_SPRING + soft * SOFT_SPRING) as f32;
    let damping = (f64::from(p) + soft * SOFT_DAMPING) as f32;
    Some(d * spring - moved.with_y(0.0) * damping)
}

/// What a body's feet take from it in an iteration.
#[derive(Debug, Clone, Copy)]
pub(super) struct FootDrive {
    /// Over half its tone, with support: the contacts are driven.
    pub driven: bool,
    /// Support S.
    pub support: f32,
    /// Off its feet (down or getting up): the soles are left alone.
    pub off_feet: bool,
    /// Has support: the soles are held over their floors.
    pub has_support: bool,
    pub walking: bool,
    pub moving: bool,
    pub run: f32,
    pub scale: f32,
    /// The PD grip's gain g: scale × pin factor × leg extension.
    pub gain: f32,
    /// The sole turn's gain before support: P × the pin factor. P is the
    /// support pins' power, √E·T·E⁴, or 1 in a shock stagger.
    pub sole_gain: f32,
    /// The pelvis's height (m) when the feet were measured.
    pub pelvis: f32,
}

impl FootDrive {
    /// What the feet of a body with muscles `drive` take from it this
    /// tick: driven over half its tone with support.
    pub(super) fn of(drive: &Drive) -> Self {
        let bearing = &drive.bearing;
        let has_support = drive.balance.has_support();
        Self {
            driven: drive.tone > TONE_FLOOR && has_support,
            support: drive.support,
            off_feet: bearing.off_feet || bearing.getting_up,
            has_support,
            walking: bearing.walking,
            moving: bearing.moving,
            run: bearing.run,
            scale: drive.stats.scale,
            gain: drive.stats.scale * drive.stats.pin_factor * drive.balance.extension,
            sole_gain: super::pins::power(
                drive.energy,
                drive.tone_target(),
                drive.bearing.shock_stagger,
            ) * drive.stats.pin_factor,
            pelvis: drive.balance.pelvis,
        }
    }
}

/// One iteration of a leg's foot contact on its lower leg `shin`: its
/// point is refreshed, then, driven and 10 cm under the pelvis, pulled by
/// its state. Returns whether it reached its anchor.
pub(super) fn drive_contact(shin: &mut Part, leg: &mut Leg, fd: &FootDrive) -> bool {
    leg.before = leg.at;
    leg.at = shin.point(&leg.contact);
    if !fd.driven || cm(leg.at.y) > cm(fd.pelvis) - UNDER_PELVIS {
        return false;
    }
    let d = cm(leg.anchor - leg.at);
    let moved = cm(shin.slip(&leg.contact));
    let mut reached = false;
    let push = if leg.state.stands() {
        let len2 = if d.y >= 0.0 {
            d.length_squared()
        } else {
            d.x * d.x + d.z * d.z
        };
        if leg.state.stepping() {
            let reach = REACH * fd.scale;
            if reach * reach <= len2 {
                let closing = cm(leg.at - leg.before).dot(d) / len2.sqrt();
                leg.approach = (f64::from(closing) * APPROACH_TAKE
                    + f64::from(leg.approach) * APPROACH_KEEP)
                    as f32;
            } else {
                leg.approach = (f64::from(leg.approach) * APPROACH_KEEP + APPROACH_TAKE) as f32;
                leg.state.0 |= FootState::REACHED;
                reached = true;
                leg.anchor.x = leg.at.x * 0.5 + leg.anchor.x * 0.5;
                leg.anchor.z = leg.at.z * 0.5 + leg.anchor.z * 0.5;
            }
            leg.grip = 0.0;
            leg.ramp = raise(leg.ramp, RAMP_ANCHORED);
            follow(leg, fd.moving);
        } else if !leg.state.has(FootState::RAMP_SLOW) {
            leg.grip = ((f64::from(leg.grip) + GRIP_RISE) as f32).min(GRIP_MAX);
            if leg.state.has(FootState::RAMP_HELD) {
                leg.ramp = RAMP_MAX;
            } else {
                leg.ramp = raise(leg.ramp, RAMP_TOUCHING);
                follow(leg, fd.moving);
            }
            leg.approach = 1.0;
        } else {
            let ramp = (RAMP_SLOWLY + f64::from(leg.ramp)) as f32;
            leg.ramp = if ramp > RAMP_MAX {
                leg.state.0 &= !FootState::RAMP_SLOW;
                RAMP_MAX
            } else {
                ramp
            };
            leg.approach = 1.0;
        }
        if len2 > EPSILON2 {
            if fd.walking {
                let gain = (f64::from(fd.run) * SERVO_RUN + SERVO_WALK) * f64::from(leg.ramp);
                let rate = f64::from(fd.scale) * RATE_TOUCHING;
                servo(d.with_y(d.y.max(0.0)), moved, gain as f32, rate as f32)
            } else {
                grip(d, moved, fd.gain, leg.grip)
            }
        } else {
            None
        }
    } else {
        let up = Vec3::new(0.0, d.y.max(0.0), 0.0);
        let push = if fd.walking {
            let gain = f64::from(fd.support)
                * (f64::from(fd.run) * SERVO_RUN + SERVO_WALK)
                * f64::from(leg.grip);
            let rate = f64::from(fd.scale) * RATE_LIFTED_WALKING;
            servo(up, moved, gain as f32, rate as f32)
        } else {
            let rate = f64::from(fd.scale) * RATE_LIFTED;
            servo(up, moved, leg.grip * fd.support, rate as f32)
        };
        leg.ramp = 0.0;
        push
    };
    if let Some(push) = push.filter(|p| p.is_finite()) {
        shin.push(&leg.contact, push / CM);
    }
    reached
}

/// The ramp raised by `step` (in double), at most 0.25.
fn raise(ramp: f32, step: f64) -> f32 {
    ((step + f64::from(ramp)) as f32).min(RAMP_MAX)
}

/// While the body moves, a standing foot's anchor follows its ankle across
/// the ground by 1 − 4 × its ramp: fully at first, not at all once full.
fn follow(leg: &mut Leg, moving: bool) {
    if moving {
        let f = 1.0 - leg.ramp * RAMP_FOLLOW;
        leg.anchor.x = leg.at.x * f + (1.0 - f) * leg.anchor.x;
        leg.anchor.z = leg.at.z * f + (1.0 - f) * leg.anchor.z;
    }
}

/// One iteration of a leg's sole on its `foot` part: with support and on
/// its feet, a toe under its floor is pushed straight up by the gap × S,
/// at most 0.5 cm while the foot stands, else 0.1 cm.
pub(super) fn push_sole(foot: &mut Part, leg: &Leg, fd: &FootDrive) {
    if fd.off_feet || !fd.has_support {
        return;
    }
    let y = cm(foot.point(&leg.toe).y);
    let floor = cm(leg.floor);
    if y >= floor {
        return;
    }
    let mut up = (floor - y) * fd.support;
    if leg.state.0 & (FootState::TOUCHING | FootState::ANCHORED | FootState::LIFTED)
        == FootState::TOUCHING
    {
        up = up.min(SOLE_STANDING);
    } else if f64::from(up) > SOLE_OTHERWISE {
        up = SOLE_OTHERWISE as f32;
    }
    foot.push(&leg.toe, Vec3::Y * (up / CM));
}

/// One iteration of a foot's sole turning toward its animated orientation
/// `turn`. On its feet and with support, once the foot has a contact
/// state, and only while its forward lies within a right angle of that
/// orientation's forward, the horizontal gap from the toe to where the
/// orientation places it is projected onto the orientation's side axis and
/// closed by the velocity servo: rate 0.4 × scale, gain
/// [`FootDrive::sole_gain`] × S. A gap under 1.19e-7 cm², or one the toe
/// already closes by 0.05, is left. A foot with no contact state is left
/// too: turning it while it settles keeps its box off the floor.
pub(super) fn turn_sole(foot: &mut Part, leg: &Leg, fd: &FootDrive, turn: Quat) {
    // Exarust's own. The original also turns a foot with no contact state.
    if fd.off_feet || !fd.has_support || leg.state.0 == 0 {
        return;
    }
    let now = foot.transform();
    let forward = Vec3::from(now.matrix3.z_axis);
    let ahead = forward.dot(turn * Vec3::Z);
    if ahead.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return;
    }
    let local = foot.rest_point(&leg.toe);
    let origin = foot.point(&foot.own_origin);
    let aimed = origin + turn * local;
    let flat = (aimed - foot.point(&leg.toe)).with_y(0.0);
    let side = turn * Vec3::X;
    let along = side * flat.dot(side);
    let rate = (f64::from(fd.scale) * SOLE_TURN_RATE) as f32;
    let gain = fd.sole_gain * fd.support;
    if let Some(push) =
        servo(cm(along), cm(foot.slip(&leg.toe)), gain, rate).filter(|p| p.is_finite())
    {
        foot.push(&leg.toe, push / CM);
    }
}

/// What one walk phase does to a foot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GaitPlant {
    /// Not touching, and the phase is in the plant window: touch, anchor
    /// across the ground on the ankle.
    Plant,
    /// Touching, and the phase is in the swing: clear the state.
    Lift,
    /// Leave the state as it is.
    Hold,
}

/// The leg's phase in the walk: the fraction of progress + 1.2 − 0.5×leg,
/// summed in double and narrowed before the (truncating) fraction.
pub(super) fn gait_phase(progress: f32, leg: usize) -> f32 {
    let x = (f64::from(progress) + 1.2 - f64::from(0.5 * leg as f32)) as f32;
    x - x.trunc()
}

/// The sampler's plant and lift of a walking foot, its phase compared in
/// double. Not touching, it plants unless the phase is past 0.4; under 0.2
/// only when its speed along the facing is at most 0.3 cm a tick
/// (backpedalling, at least −0.2). A NaN phase or speed plants. Touching,
/// it lifts unless the phase is strictly between 0 and 0.55.
pub(super) fn gait_plant(phase: f32, touching: bool, along_cm: f32, backwards: bool) -> GaitPlant {
    let p = f64::from(phase);
    let along = f64::from(along_cm);
    if !touching {
        let plants = if p > 0.4 {
            false
        } else if p < 0.2 {
            if backwards {
                along >= -0.2 || along.is_nan()
            } else {
                along <= 0.3 || along.is_nan()
            }
        } else {
            true
        };
        if plants {
            GaitPlant::Plant
        } else {
            GaitPlant::Hold
        }
    } else if p > 0.0 && p < 0.55 {
        GaitPlant::Hold
    } else {
        GaitPlant::Lift
    }
}

/// The feet's states before they are measured, as the original's sampler
/// and footwork leave them. A body that is not walking plants each foot
/// that does not touch where it is (`ankles`), but only once the foot
/// meets the level (`down`: its part has level contacts; Exarust's own
/// until the leg IK lowers a planted foot onto its anchor). A walking
/// body's feet plant and lift by [`gait_plant`].
/// Standing (not moving): lying down, no foot touches; getting up (until
/// nine tenths of its clip) or without support (`had_support`, last
/// tick's), each foot touches on its stance spot (`spots`), not touching in
/// a get-up's first fifth. Standing with support the footwork runs: a foot
/// that reached its anchor lands (a support foot again), one whose step
/// stalled lands where it is; then one foot at a time steps to its stance
/// spot (anchored, its anchor re-taken every tick at its height), which
/// the drive pulls it to. Which foot steps is the footwork's stance score
/// ([`steps::pick`]): stay, foot 0 or foot 1, the highest, and a foot
/// under 5 cm of its spot (10 cm after a stall, outside a recovery step)
/// scores as staying. Returns whether the other foot landing ended the
/// recovery step.
#[allow(clippy::too_many_arguments)]
pub(super) fn plant(
    legs: &mut [Leg],
    ankles: &[Vec3],
    spots: &[Option<Vec3>],
    moves: &[Vec3],
    bearing: &Bearing,
    had_support: bool,
    down: &[bool],
    work: &mut Work,
) -> bool {
    for (i, ((leg, &at), &down)) in legs.iter_mut().zip(ankles).zip(down).enumerate() {
        leg.planted = false;
        if bearing.walking {
            let along = moves.get(i).map_or(0.0, |m| cm(*m).dot(bearing.facing));
            let along = if bearing.backpedal { -along } else { along };
            match gait_plant(
                gait_phase(bearing.progress, i),
                leg.state.has(FootState::TOUCHING),
                along,
                bearing.backpedal,
            ) {
                GaitPlant::Plant => {
                    leg.anchor.x = at.x;
                    leg.anchor.z = at.z;
                    leg.state = FootState(FootState::TOUCHING);
                    leg.planted = true;
                }
                GaitPlant::Lift => leg.state = FootState(0),
                GaitPlant::Hold => {}
            }
        } else if !leg.state.has(FootState::TOUCHING) && down {
            leg.anchor = at;
            leg.state = FootState(FootState::TOUCHING);
        }
    }
    if bearing.walking || bearing.moving {
        return false;
    }
    let off_feet = bearing.off_feet || bearing.getting_up;
    if off_feet && !bearing.getting_up {
        for leg in legs.iter_mut() {
            leg.state = FootState(0);
        }
        return false;
    }
    let progress = f64::from(bearing.progress);
    let bad = off_feet && !(bearing.getting_up && progress > GET_UP_FOOTWORK);
    if bad || !had_support {
        for (leg, spot) in legs.iter_mut().zip(spots) {
            leg.state = FootState(FootState::TOUCHING);
            if let Some(spot) = spot {
                leg.anchor = *spot;
            }
        }
        if bearing.getting_up && progress < GET_UP_UNTOUCHED {
            for leg in legs.iter_mut() {
                leg.state.0 &= !FootState::TOUCHING;
            }
        }
        false
    } else if bearing.kicking {
        // A kick puts every foot on its stance spot, touching (the
        // original's kick-motion branch).
        for (leg, spot) in legs.iter_mut().zip(spots) {
            leg.state = FootState(FootState::TOUCHING);
            if let Some(spot) = spot {
                leg.anchor = *spot;
            }
        }
        false
    } else {
        footwork(legs, ankles, spots, work)
    }
}

/// Where one pass of the footwork left the feet after a stagger target.
#[derive(Clone, Debug, PartialEq)]
pub struct WalkedStep {
    /// The stance origin: the target, pulled back toward the frame.
    pub origin: Vec3,
    /// The foot that stepped, when one did.
    pub foot: Option<usize>,
    pub anchors: Vec<Vec3>,
    pub states: Vec<u32>,
    /// The other foot's landing ended the step.
    pub ended: bool,
}

/// Walk the feet toward a collapse stagger's target. The stance origin is
/// that target, pulled back to within 1.1 step lengths of `frame`, at the
/// frame's height. Each stance spot moves with it. One footwork pass then
/// anchors at most one foot on its spot, at the ankle's height.
pub fn walk_stagger(
    legs: &mut [Leg],
    ankles: &[Vec3],
    spots: &[Option<Vec3>],
    target: Vec3,
    frame: Vec3,
    side: Vec3,
    scale: f32,
) -> WalkedStep {
    let step_cm = steps::step_length(scale, 1.0);
    let origin = steps::stance_origin(target, frame, step_cm, scale);
    let shifted: Vec<Option<Vec3>> = spots
        .iter()
        .map(|spot| {
            spot.map(|spot| {
                let local = spot - frame;
                Vec3::new(origin.x + local.x, spot.y, origin.z + local.z)
            })
        })
        .collect();
    let mut work = Work {
        now_ms: 0.0,
        scale,
        crouch: 0.0,
        reference: frame,
        root: frame,
        origin,
        side,
        step_cm,
        episode: true,
        episode_ms: 0.0,
        chest_to_target_cm: cm((target - frame).with_y(0.0)).length(),
        size: 1.0,
        still_at: f32::NAN,
        still_plus: f32::NAN,
    };
    let ended = footwork(legs, ankles, &shifted, &mut work);
    let foot = legs
        .iter()
        .position(|leg| leg.state.has(FootState::ANCHORED));
    WalkedStep {
        origin,
        foot,
        anchors: legs.iter().map(|leg| leg.anchor).collect(),
        states: legs.iter().map(|leg| leg.state.0).collect(),
        ended,
    }
}

/// The footwork of a body standing with support: the landing pass, then the
/// stance score moves at most one foot ([`plant`]). A reached foot lands
/// at once. Returns whether the other foot's landing ended the recovery
/// step (it had run for more than 200 ms and the chest was within 20·scale
/// cm of the target).
fn footwork(legs: &mut [Leg], ankles: &[Vec3], spots: &[Option<Vec3>], work: &mut Work) -> bool {
    for (leg, &at) in legs.iter_mut().zip(ankles) {
        if leg.state.has(FootState::REACHED) {
            leg.state.0 &= !(FootState::ANCHORED | FootState::REACHED);
            leg.changed_ms = work.now_ms;
        } else if leg.state.has(FootState::ANCHORED) && f64::from(leg.approach) < STALLED_BELOW {
            leg.state.0 =
                leg.state.0 & FootState::RAMP_SLOW | FootState::TOUCHING | FootState::STALLED;
            leg.anchor = at;
            // A stalled foot's landing opens the stand-still window at
            // once: the later stamp is the landing time itself.
            work.still_plus = work.now_ms;
            leg.changed_ms = work.now_ms;
        }
    }
    if legs.len() != 2 {
        return false;
    }
    let Some(pair) = two_spots(spots) else {
        return false;
    };
    let side = work.side.with_y(0.0).normalize_or(Vec3::X);
    let views = [
        foot_view(&legs[0], ankles[0]),
        foot_view(&legs[1], ankles[1]),
    ];
    let spots_cm = pair.map(cm);
    let chosen = steps::pick(
        &views,
        &spots_cm,
        &ScoreCtx {
            now_ms: work.now_ms,
            scale: work.scale,
            crouch: work.crouch,
            reference: cm(work.reference),
            side,
            root: cm(work.root),
            origin: cm(work.origin),
            step: work.step_cm,
            episode: work.episode,
            size: work.size,
            still_at: work.still_at,
            still_plus: work.still_plus,
        },
    );
    let Some(k) = chosen else {
        return false;
    };
    let Some(spot) = spots.get(k).copied().flatten() else {
        return false;
    };
    let other = 1 - k;
    let mut ended = false;
    if legs[other].state.has(FootState::ANCHORED) {
        // The landing writes the body's later stand-still stamp (now +
        // 100 ms, which the trailing plant also writes) and the foot's own
        // change stamp; the earlier stamp is the last step start's.
        work.still_plus = work.now_ms + 100.0;
        legs[other].changed_ms = work.now_ms;
        if !legs[other].state.has(FootState::REACHED) {
            legs[other].anchor = ankles[other];
        }
        let keep = legs[other].state.0 & (FootState::RAMP_HELD | FootState::RAMP_SLOW);
        legs[other].state = FootState(FootState::TOUCHING | keep);
        ended =
            work.episode && work.episode_ms > 200.0 && work.chest_to_target_cm <= 20.0 * work.scale;
    }
    let mut anchor = Vec3::new(spot.x, ankles[k].y, spot.z);
    anchor = steps::clamp_anchor(anchor, ankles[other], work.step_cm);
    if !legs[k].state.has(FootState::ANCHORED) {
        legs[k].state.0 =
            legs[k].state.0 & !(FootState::REACHED | FootState::RAMP_HELD) | FootState::ANCHORED;
        legs[k].approach = 1.0;
        // The step's length feeds its sound's loudness.
        legs[k].step_length_cm = cm(anchor - ankles[k]).length();
        // A foot that starts moving stamps the body's earlier stand-still
        // stamp.
        work.still_at = work.now_ms;
    }
    legs[k].anchor = anchor;
    ended
}

/// The projection the rearmost foot's search starts under.
const REAR_START: f32 = f32::from_bits(0x7f7f_c99e);

impl Ragdoll {
    /// Leaving the walk: the rearmost foot (the smallest projection of its
    /// current contact point along the frame's forward; the first of equal
    /// ones, and the first when none is under 3.4e38 or all are NaN) is
    /// anchored at its stance spot under the frame, keeping its height,
    /// and every other foot lands where it is (whether or not it was
    /// anchored). Both keep their slow ramp bit. It stamps the step clock
    /// the next score reads (that clock advances before the score) and
    /// that clock plus 100 ms; a footwork landing writes the same pair.
    pub fn plant_trailing_foot(&mut self) {
        let Ragdoll {
            parts, legs, drive, ..
        } = self;
        let Some(drive) = drive.as_mut() else {
            return;
        };
        let Some(frame) = drive.frame else {
            return;
        };
        if legs.is_empty() || drive.targets.len() != parts.len() {
            return;
        }
        let now = drive.steps.clock.saturating_add(1) as f32 * steps::MS_PER_TICK;
        drive.steps.still_at = now;
        drive.steps.still_plus = now + 100.0;
        let forward = Vec3::from(frame.matrix3.z_axis);
        // A strict compare against the running smallest: a NaN never wins.
        let mut smallest = REAR_START;
        let mut rear = 0;
        for (i, leg) in legs.iter().enumerate() {
            let at = parts[leg.shin].point(&leg.contact);
            // The projection sums z + (y + x), as the original does.
            let along = at.z * forward.z + (at.y * forward.y + at.x * forward.x);
            if smallest > along {
                smallest = along;
                rear = i;
            }
        }
        for (i, leg) in legs.iter_mut().enumerate() {
            let at = parts[leg.shin].point(&leg.contact);
            if i == rear {
                let posed = drive.targets[leg.shin].transform_point3(leg.point);
                leg.anchor = Vec3::new(posed.x, at.y, posed.z);
                leg.at = at;
                leg.before = at;
                leg.state = FootState(
                    leg.state.0 & FootState::RAMP_SLOW | FootState::TOUCHING | FootState::ANCHORED,
                );
            } else {
                if leg.state.has(FootState::ANCHORED) {
                    leg.anchor = at;
                    leg.at = at;
                    leg.before = at;
                }
                leg.state = FootState(leg.state.0 & FootState::RAMP_SLOW | FootState::TOUCHING);
            }
        }
    }
}

/// The two stance spots, when both feet have one.
fn two_spots(spots: &[Option<Vec3>]) -> Option<[Vec3; 2]> {
    Some([
        spots.first().copied().flatten()?,
        spots.get(1).copied().flatten()?,
    ])
}

/// One foot in centimetres, for the stance score.
fn foot_view(leg: &Leg, ankle: Vec3) -> FootView {
    FootView {
        anchored: leg.state.has(FootState::ANCHORED),
        stalled: leg.state.has(FootState::STALLED),
        anchor: cm(leg.anchor),
        point: cm(ankle),
        leg: cm(leg.length),
        changed_ms: leg.changed_ms,
    }
}

/// An anchored foot's probe blend this tick: +0.05 up to 0.5; none for a
/// foot that is not anchored.
pub(super) fn probe_blend(leg: &Leg) -> f32 {
    if leg.state.has(FootState::ANCHORED) {
        ((f64::from(leg.blend) + PROBE_BLEND) as f32).min(PROBE_BLEND_MAX)
    } else {
        0.0
    }
}

/// A step's loudness from its length (cm): 0.01 + 0.0139 × the length up
/// to 140 cm, 1 beyond. The footstep sound's volume starts here.
pub fn step_loudness(length_cm: f32) -> f32 {
    if length_cm <= 140.0 {
        length_cm * 0.0139 + 0.01
    } else {
        1.0
    }
}

/// How far the floor may lie under the ground while the foot touches and
/// the Ground Query hits (cm). The query itself is a point ray from 30 cm
/// over the ground 60 cm down against floor pieces (`Level::floor_below`);
/// a miss leaves the ground.
const FLOOR_TOUCHING: f32 = 10.0;

/// The loudness of a walking footstep before the random share is this body
/// gain times the character's footstep volume (the init's: the body scale,
/// doubled over 1.3, the ogre's 5). The gain field has no traced writer
/// in the character init; 1 is a guess.
const STEP_GAIN: f32 = 1.0;

/// A walking footstep's gain and pitch from its speed along the heading
/// (cm a tick) and the engine's two draws in [0, 1), the pitch's first:
/// the pitch is `(0.2·p + 0.9)(0.05·speed + 0.85)` and the gain
/// `gain · volume · (0.5·g + 0.5) · ((0.08·speed)² + 0.01)`, the draw's share in
/// single precision and the rest in double, narrowed once, as the
/// original's are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaitStep {
    pub gain: f32,
    pub pitch: f32,
}

/// The walk's footstep gain and pitch from the character's footstep volume,
/// the pitch draw `p` and the gain draw `g`, drawn in that order.
pub fn gait_step(speed_cm: f32, volume: f32, p: f32, g: f32) -> GaitStep {
    let speed = f64::from(speed_cm);
    let pitch = ((0.2 * f64::from(p) + 0.9) * (0.05 * speed + 0.85)) as f32;
    let share = f64::from(g * 0.5 + 0.5);
    let loud = 0.08 * speed * (0.08 * speed) + 0.01;
    let gain = (f64::from(STEP_GAIN) * f64::from(volume) * (share * loud)) as f32;
    GaitStep { gain, pitch }
}

/// The clothing rustle a walking plant makes beside its footstep, when the
/// body has a foley set: the volume `base · ((0.5·g + 0.5)(0.02·speed +
/// 0.01))` and the pitch `(0.2·p + 0.9)(0.2·speed + 0.7) / size`, from the
/// foley's base volume, the speed along the heading (cm a tick), the
/// body's size and the engine's two draws in [0, 1), the volume's first;
/// the draw's share in single precision and the rest in double, narrowed
/// once. The played gain is the volume times the sound gain, and the
/// foley clock then waits 400 ms plus `(roll · 100) >> 32`, the engine's
/// generator (`roll`) stepped once. Not wired: Exarust keeps no walking
/// foley voice; a tested rule with the original's numbers.
pub fn walk_rustle(
    base: f32,
    speed_cm: f32,
    size: f32,
    g: f32,
    p: f32,
    now_ms: u32,
    roll: &mut u32,
) -> WalkRustle {
    let speed = f64::from(speed_cm);
    let share = f64::from(g * 0.5 + 0.5);
    let volume = (f64::from(base) * (share * (speed * 0.02 + 0.01))) as f32;
    let pitch = ((f64::from(p) * 0.2 + 0.9) * (speed * 0.2 + 0.7) / f64::from(size)) as f32;
    WalkRustle {
        volume,
        pitch,
        next_ms: sound_clock(now_ms, RUSTLE_WAIT_MS, roll),
    }
}

/// A walking plant's clothing rustle ([`walk_rustle`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkRustle {
    /// The volume the character keeps (the played gain is it times the
    /// sound gain).
    pub volume: f32,
    pub pitch: f32,
    /// The foley clock's next free time (ms).
    pub next_ms: u32,
}

/// The engine's generator stepped once and its `(roll · 100) >> 32`
/// jitter (ms) added to `now_ms + wait_ms`: the next free time of a foot
/// sound clock.
fn sound_clock(now_ms: u32, wait_ms: u32, roll: &mut u32) -> u32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    let jitter = ((u64::from(*roll) * 100) >> 32) as u32;
    now_ms.wrapping_add(wait_ms).wrapping_add(jitter)
}

/// The balance routine's clothing rustle clock wait (after a foot anchors or a walking plant)
/// (ms), and its footstep clock wait after a foot lets go.
pub const RUSTLE_WAIT_MS: u32 = 400;
pub const RELEASE_STEP_WAIT_MS: u32 = 250;

/// A foot sound's volume (the played gain is it times the sound gain),
/// pitch and its clock's next free time (ms).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootSound {
    pub volume: f32,
    pub pitch: f32,
    pub next_ms: u32,
}

/// The clothing rustle the balance routine plays when a foot becomes
/// anchored and its rustle clock is free: `k = min(1, 0.0001·d² + 0.2)`
/// (d² the squared distance in cm² between the foot's contact and its
/// previous point, scaled and offset in double), doubled while attacking,
/// then the volume `k · base · (0.02·g + 0.02)` (double, narrowed once)
/// and the pitch `0.3·p + 0.8`, the volume's draw first. The played gain
/// is the volume times the sound gain; the clock waits 400 ms plus the
/// generator's jitter. A NaN distance keeps the unclamped NaN.
pub fn anchor_rustle(
    base: f32,
    gap_sq_cm: f32,
    attacking: bool,
    g: f32,
    p: f32,
    now_ms: u32,
    roll: &mut u32,
) -> FootSound {
    let mut k = (f64::from(gap_sq_cm) * 0.0001 + 0.2) as f32;
    if k > 1.0 {
        k = 1.0;
    }
    if attacking {
        k *= 2.0;
    }
    let share = f64::from(g) * 0.02 + 0.02;
    let volume = (f64::from(k) * f64::from(base) * share) as f32;
    let pitch = (f64::from(p) * 0.3 + 0.8) as f32;
    FootSound {
        volume,
        pitch,
        next_ms: sound_clock(now_ms, RUSTLE_WAIT_MS, roll),
    }
}

/// The footstep the balance routine plays when an anchored foot lets go
/// and the footstep clock is free, from the step's loudness (`loudness`,
/// [`step_loudness`]), the character's footstep volume (`volume`), the
/// body gain and the engine's draws, the pitch's first: the pitch
/// `0.25·p + (0.2·loudness + 0.5)` and the gain `gain · loudness · volume ·
/// (0.005·g + 0.02)`, the draw's scaling in single precision for the
/// pitch and the rest in double, narrowed once.
pub fn release_step(loudness: f32, volume: f32, p: f32, g: f32) -> GaitStep {
    let pitch = (f64::from(p * 0.25) + (f64::from(loudness) * 0.2 + 0.5)) as f32;
    let share = f64::from(g) * 0.005 + 0.02;
    let gain = (f64::from(STEP_GAIN) * f64::from(loudness) * f64::from(volume) * share) as f32;
    GaitStep { gain, pitch }
}

/// The wet step that follows [`release_step`] when the water surface
/// (`water_cm`, the level's) lies over the foot's contact height
/// (`foot_cm`): its depth share `min(1, 0.05·(−water − 1 − foot))` (the
/// sum in single precision, scaled in double), then the pitch
/// `0.35·p + (0.2·loudness + 0.4)` and the gain `gain · depth · loudness ·
/// volume · (0.01·g + 0.02)` in double, the pitch's draw first. `None`
/// when the foot is not under the surface (or either height is NaN).
pub fn wet_release_step(
    water_cm: f32,
    foot_cm: f32,
    loudness: f32,
    volume: f32,
    p: f32,
    g: f32,
) -> Option<GaitStep> {
    if water_cm.partial_cmp(&foot_cm) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    let mut depth = (f64::from(-water_cm - 1.0 - foot_cm) * 0.05) as f32;
    if depth > 1.0 {
        depth = 1.0;
    }
    let pitch = (f64::from(p) * 0.35 + (f64::from(loudness) * 0.2 + 0.4)) as f32;
    let share = f64::from(g) * 0.01 + 0.02;
    let gain =
        (f64::from(STEP_GAIN) * f64::from(depth) * f64::from(loudness) * f64::from(volume) * share)
            as f32;
    Some(GaitStep { gain, pitch })
}

/// The footstep clock's next free time after [`release_step`] (and its
/// wet step): 250 ms plus the generator's jitter, stepped once.
pub fn release_step_clock(now_ms: u32, roll: &mut u32) -> u32 {
    sound_clock(now_ms, RELEASE_STEP_WAIT_MS, roll)
}

/// Where a footstep sounds from: the character's place, this far (m)
/// along its motion: `3·speed + 4` cm.
pub fn step_position(character: Vec3, direction: Vec3, speed_cm: f32) -> Vec3 {
    character + direction * ((3.0 * speed_cm + 4.0) / CM)
}

/// A walking footstep, as the body's sampler makes it: the sound's place,
/// the speed along the heading (cm a tick), the character's footstep volume
/// and the race's footstep set (empty when the race has none).
#[derive(Debug, Clone, Copy, Message, PartialEq)]
pub struct Footstep {
    pub position: Vec3,
    pub speed_cm: f32,
    /// The character's footstep volume (its stats' `footstep_volume`).
    pub volume: f32,
    /// The race's footstep set's name in `Sound.rpk`; none for a race
    /// without one.
    pub set: Option<&'static str>,
}

/// A foot's sole floor (cm) from the Ground Query's `hit`: the higher of
/// the hit and the ground (10 cm lower while the foot touches) plus the
/// leg's own offset, or the ground plus the offset where the query misses;
/// never above the knee. The original's Ground Query is a point ray at the
/// toe, starting 30 cm over the ground and looking 60 cm down
/// (`Level::floor_below`).
pub fn sole_floor(
    ground_cm: f32,
    touching: bool,
    offset_cm: f32,
    hit: Option<f32>,
    knee_cm: f32,
) -> f32 {
    let low = ground_cm - if touching { FLOOR_TOUCHING } else { 0.0 } + offset_cm;
    let floor = match hit {
        Some(hit) => hit.max(low),
        None => ground_cm + offset_cm,
    };
    floor.min(knee_cm)
}

/// What a queued step does: its new target, whether the AI is told, and the
/// countdown the queue takes (`−2` before the first landing, `−1` after).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueuedStep {
    pub target: Vec3,
    pub tell_ai: bool,
    pub countdown: i32,
}

/// A queued footwork step (a positive queue count): with the body's feet
/// on their stance spots and the queue waiting, the step is taken when the
/// cosine between the queued vector and the line from the chest to the
/// step target is under 0.7, or the target is nearer than 40 × scale cm. It aims at the chest
/// plus the queued vector and tells the AI when that vector points along
/// the facing (the dot summed z + (y + x), strictly above 0). The original
/// clamps the target through its pathfinder; the caller does. The
/// countdown becomes −2 before the first landing (`landings < 1`) and −1
/// after it. The player's combat footwork (`exa_game::footwork`) takes its
/// queue through it when its interim step lands.
pub fn queued_step(
    chest: Vec3,
    target: Vec3,
    queued: Vec3,
    facing: Vec3,
    scale: f32,
    landings: i32,
) -> Option<QueuedStep> {
    let d = (target - chest).with_y(0.0);
    // The engine's cosine between two vectors: their dot over the root of
    // the product of their squared lengths, 0 when that product is at most
    // 1e-8 (the chest on the target, or no queued vector).
    let lengths = cm(d).length_squared() * cm(queued).length_squared();
    let cos = if lengths > 1e-8 {
        let (a, b) = (cm(d), cm(queued));
        (a.z * b.z + (a.y * b.y + a.x * b.x)) / lengths.sqrt()
    } else {
        0.0
    };
    // Off the line by more than the double 0.7 takes the queue; a NaN
    // cosine falls through to the distance test.
    let off_line = f64::from(cos) < 0.7;
    let near = (40.0 * scale) * (40.0 * scale) > cm(d).length_squared();
    if off_line || near {
        let end = step_end(true, queued, facing, chest);
        Some(QueuedStep {
            target: end.target,
            tell_ai: end.tell_ai,
            countdown: if landings < 1 { -2 } else { -1 },
        })
    } else {
        None
    }
}

/// The radius (cm) of a foot's ground probe when the body is not walking:
/// 5 cm plus 10 times its blend. A walking foot uses [`gait_probe`].
pub(super) fn probe_radius(blend: f32) -> f32 {
    PROBE_RADIUS + PROBE_RADIUS_BLEND * blend
}

/// The walk's two standing-ground weights. `last` is the stored signed
/// height difference between the feet (cm), smoothed 0.9/0.1 in double
/// toward the difference the clip's phase picks (`g0 − g1` in the first
/// half of the phase, else `g1 − g0`, from last tick's grounds). Moving,
/// the first foot's weight is (1 − r)·0.5 + (1 − |2t − 1|)·r, r = 1 −
/// run/2, t the fraction of the progress plus a bias from the difference
/// (0.4 above 0, 0.65 below −20 cm, else 0.4 − 0.05·(0.25·difference) in
/// double); the second foot takes the rest. Not moving, the difference
/// only fades by 0.9 and each foot weighs 0.5. Returns
/// `(difference, first, second)`.
pub(super) fn gait_weights(
    progress: f32,
    run: f32,
    moving: bool,
    g0: f32,
    g1: f32,
    last: f32,
) -> (f32, f32, f32) {
    if !moving {
        let last = (f64::from(last) * 0.9) as f32;
        return (last, 0.5, 0.5);
    }
    // The phase is the fraction of the sum narrowed to single precision.
    let phase = frac32((f64::from(progress) + 0.45) as f32);
    let diff = if phase > 0.5 { g1 - g0 } else { g0 - g1 };
    let last = (f64::from(diff) * 0.1 + f64::from(last) * 0.9) as f32;
    let bias = if last > 0.0 {
        0.4
    } else if last < -20.0 {
        0.65
    } else {
        (0.4 - f64::from(0.25 * last) * 0.05) as f32
    };
    let t = frac32(progress + bias);
    let r = 1.0 - run * 0.5;
    let first = (1.0 - r) * 0.5 + (1.0 - (t * 2.0 - 1.0).abs()) * r;
    (last, first, 1.0 - first)
}

/// The fraction of a single, in the engine's (truncating) sense.
fn frac32(x: f32) -> f32 {
    x - x.trunc()
}

/// The signed height difference (cm) the walk's gait phase picks from last
/// tick's two grounds, before its smoothing: the feet routine's
/// |difference| is the late foot's swing threshold's addition. The phase
/// is the fraction of progress + 0.45, compared in double as the original
/// compares it; a NaN phase picks the first.
pub(super) fn gait_gap(progress: f32, g0: f32, g1: f32) -> f32 {
    let phase = frac32((f64::from(progress) + 0.45) as f32);
    if f64::from(phase) > 0.5 {
        g1 - g0
    } else {
        g0 - g1
    }
}

/// One walking foot's ground probe, from the leg's index and the clip
/// progress. The phase is the fraction of 2 − 0.5·index − progress
/// (truncating toward zero). From it, s = max(0, 16p² − 64p⁴) and the
/// bump is (2 − s)·s: the sphere is 10 + 10·bump cm (20 at the peak). The
/// probe sways s·(36·scale + 8·speed) cm along the facing, the other way
/// when walking backwards. A ground higher than the smoothed one is taken
/// at once; a lower one is taken by the fall share
/// 1 − 0.95·(2 − r)·r, r = 0.12·max(0, 24p − 2048p⁸), the two products
/// in double precision and narrowed once.
pub(super) fn gait_probe(
    leg: usize,
    progress: f32,
    scale: f32,
    speed: f32,
    backwards: bool,
) -> GaitProbe {
    let half = (leg as f32) * 0.5;
    let phase = super::locomotion::fraction((2.0 - half) - progress);
    let p2 = phase * phase;
    let four = p2 * 4.0;
    let fourth = four * four;
    let raised_raw = phase * 24.0 - (fourth * fourth) * 8.0;
    let raised_raw = if raised_raw > 0.0 { raised_raw } else { 0.0 };
    let raised = (f64::from(raised_raw) * 0.12) as f32;
    let shaped = (2.0 - raised) * raised;
    let fall = (1.0 - f64::from(shaped) * 0.95) as f32;
    let wide_raw = p2 * 16.0 - fourth * 4.0;
    let share = (if wide_raw > 0.0 { wide_raw } else { 0.0 }) * 1.0;
    let bump = (2.0 - share) * share;
    let mut reach = speed * 8.0 + 36.0 * scale;
    if backwards {
        reach = -reach;
    }
    GaitProbe {
        radius: 10.0 * bump + 10.0,
        fall,
        sway: reach * share,
    }
}

/// A walking foot's ground probe: the sphere's radius (cm), how much of a
/// lower ground it takes this tick, and the sway along the facing (cm).
pub(super) struct GaitProbe {
    pub radius: f32,
    pub fall: f32,
    pub sway: f32,
}

/// A foot's smoothed ground (cm) from the last one and the ground found
/// now: it rises at once and falls by `fall` (1 outside a walk; a walk's
/// is the gait's).
pub(super) fn smooth_ground(last: f32, found: f32, fall: f32) -> f32 {
    if last <= found {
        found
    } else {
        (1.0 - fall) * last + found * fall
    }
}

/// A foot's anchor height (cm): its ground plus the sole height, at most
/// 1 m over its ankle at `ankle`.
pub(super) fn anchor_height(ground: f32, sole: f32, ankle: f32) -> f32 {
    (ground + sole).min(ankle + ANCHOR_ABOVE)
}

/// The stance's level after a tick with support: 0.9 of it and 0.1 of
/// 1 − v², v the share of the feet's height difference `dy` (cm) in their
/// squared distance (with `dx`, `dz` across the ground): 1 on level
/// ground.
pub(super) fn level(last: f32, dy: f32, dx: f32, dz: f32) -> f32 {
    let dy2 = dy * dy;
    // The horizontal squares are summed first, as the original does; two
    // feet on one spot give 0/0 and a NaN level (the original propagates
    // it).
    let v = dy2 / (dy2 + (dz * dz + dx * dx));
    let take = 1.0 - v * v;
    (f64::from(last) * LEVEL_KEEP + f64::from(take) * LEVEL_TAKE) as f32
}

/// The legs' extension factor: the stance's `level` times
/// (`over` / `reach`)², capped at 1, `over` the pelvis's height over the
/// highest anchor and `reach` the pose's pelvis over its mean ankle.
pub(super) fn extension(level: f32, over: f32, reach: f32) -> f32 {
    let x = if reach <= over {
        1.0
    } else {
        let x = over / reach;
        x * x
    };
    if x.is_nan() { level } else { x * level }
}

/// What the end of a step leaves: where the step target is (the base
/// point moved by the step vector), and whether the AI is told (the step
/// pointing along the body's facing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepEnd {
    pub target: Vec3,
    pub tell_ai: bool,
}

/// The step's end (the original's step-end routine): with an AI target the
/// AI is told when the step vector points along the facing (the dot summed
/// z + (y + x), strictly above 0); the target is the base point plus the
/// step. The stepping flags, the lift reset and the foot timer are the
/// caller's to set.
pub fn step_end(ai_target: bool, step: Vec3, facing: Vec3, base: Vec3) -> StepEnd {
    let dot = step.z * facing.z + (step.y * facing.y + step.x * facing.x);
    StepEnd {
        target: base + step,
        tell_ai: ai_target && dot > 0.0,
    }
}

#[cfg(test)]
mod step_end_tests {
    use super::*;

    #[test]
    fn the_step_end_moves_the_base_by_the_step() {
        let end = step_end(true, Vec3::Z, Vec3::Z, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(end.target, Vec3::new(1.0, 2.0, 4.0));
        assert!(end.tell_ai);
        assert!(!step_end(false, Vec3::Z, Vec3::Z, Vec3::ZERO).tell_ai);
        assert!(!step_end(true, Vec3::NEG_Z, Vec3::Z, Vec3::ZERO).tell_ai);
        // A step across the facing (dot exactly 0) or a NaN step tells no one.
        assert!(!step_end(true, Vec3::X, Vec3::Z, Vec3::ZERO).tell_ai);
        assert!(!step_end(true, Vec3::NAN, Vec3::Z, Vec3::ZERO).tell_ai);
    }
}

impl Ragdoll {
    /// Clears bits `mask` from every foot contact's state, as the original
    /// does when a motion starts or a get-up begins.
    pub fn clear_foot_state(&mut self, mask: u32) {
        for leg in &mut self.legs {
            leg.state = FootState(leg.state.0 & !mask);
        }
    }
}

/// What a full tone reset writes: the stance factor carried into two
/// fields, tone 1, the stagger speed 1 and decay 0, and the getting-up
/// condition word (assigned, not or-ed) for the two get-up motions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FullTone {
    pub stance: f32,
    pub tone: f32,
    pub stagger_speed: f32,
    pub stagger_decay: f32,
    pub getting_up: bool,
}

/// The reset values (the original's routine): the stance factor is copied
/// from the body's current one (`stance`), the tone and stagger speed are 1,
/// the stagger's decay 0, and for motions 0x0E and 0x0F the whole condition
/// word is assigned 0x300 (`getting_up`).
pub fn full_tone(motion: i32, stance: f32) -> FullTone {
    FullTone {
        stance,
        tone: 1.0,
        stagger_speed: 1.0,
        stagger_decay: 0.0,
        getting_up: motion == 0x0E || motion == 0x0F,
    }
}

/// Which of its three modes the feet routine runs for a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeetMode {
    /// The condition's flat-floor bit: every ground at 0, no probes.
    FlatFloor,
    /// A race whose feet hover (the wraith): one probe under the pelvis.
    Hovering,
    /// Every leg probes its own ground.
    Walking,
}

/// The feet routine's mode: the flat-floor bit wins over the hover flag.
pub fn feet_mode(flat_floor: bool, hovers: bool) -> FeetMode {
    if flat_floor {
        FeetMode::FlatFloor
    } else if hovers {
        FeetMode::Hovering
    } else {
        FeetMode::Walking
    }
}

/// A flat-floor foot's state: touching, reached and its ramp held.
pub const FLAT_FLOOR_FOOT: FootState =
    FootState(FootState::TOUCHING | FootState::REACHED | FootState::RAMP_HELD);

/// A flat-floor leg: its smoothed ground 0, its anchor's height the sole
/// height plus 1 cm, its state [`FLAT_FLOOR_FOOT`]; its foot's floor and
/// its companion are 1 (cm, relative to the foot), and its reach is its
/// rest reach.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatFloorLeg {
    pub ground: f32,
    pub anchor_height: f32,
    pub state: FootState,
    pub floor: f32,
}

/// [`FlatFloorLeg`] for a body of sole height `sole` (cm).
pub fn flat_floor_leg(sole: f32) -> FlatFloorLeg {
    FlatFloorLeg {
        ground: 0.0,
        anchor_height: sole + 1.0,
        state: FLAT_FLOOR_FOOT,
        floor: 1.0,
    }
}

/// On a flat floor the pelvis aims 10 cm under its standing height (cm);
/// the body's ground is 0, it has support and its support, footing and
/// balance factors are 1.
pub fn flat_floor_target(standing: f32) -> f32 {
    standing - 10.0
}

/// The hovering probe's reach down (cm) and radius (cm): one sphere cast
/// under the pelvis.
pub const HOVER_PROBE_REACH: f32 = 2000.0;
pub const HOVER_PROBE_RADIUS: f32 = 20.0;
/// How far the probe starts above the pelvis (cm), and how much higher it
/// starts after every miss.
pub const HOVER_PROBE_RISE: f32 = 20.0;
pub const HOVER_PROBE_MISS_RISE: f32 = 5.0;

/// A hovering body's ground probe: it starts 20 cm above the pelvis plus
/// what its misses added, 5 cm each, until one hits.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HoverProbe {
    /// The extra rise after the misses so far (cm).
    pub raise: f32,
}

impl HoverProbe {
    /// Where the probe starts (cm): `(pelvis + 20) + raise`, single
    /// precision in that order.
    pub fn start(&self, pelvis_y: f32) -> f32 {
        pelvis_y + HOVER_PROBE_RISE + self.raise
    }

    /// The probe's outcome: a hit is the body's ground (and its previous
    /// ground) and clears the raise; a miss leaves the ground as it was
    /// and raises the next probe 5 cm. The pelvis then aims at its
    /// standing height; support and its factors are 1.
    pub fn land(&mut self, hit: Option<f32>) -> Option<f32> {
        match hit {
            Some(y) => {
                self.raise = 0.0;
                Some(y)
            }
            None => {
                self.raise += HOVER_PROBE_MISS_RISE;
                None
            }
        }
    }
}

#[cfg(test)]
#[path = "body_feet_test.rs"]
mod tests;
