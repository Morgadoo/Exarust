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
//! an iteration (0.1 unless it stands), times S.
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
const WALKING_RADIUS: f32 = 10.0;
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

/// The leg's phase in the walk: the fraction of progress + 1.2 − 0.5×leg.
pub(super) fn gait_phase(progress: f32, leg: usize) -> f32 {
    (progress + 1.2 - 0.5 * leg as f32).rem_euclid(1.0)
}

/// Plant in [0.2, 0.4] while the foot does not touch. Lift at phase 0 or
/// at least 0.55 while it does.
pub(super) fn gait_plant(phase: f32, touching: bool) -> GaitPlant {
    if !touching && (0.2..=0.4).contains(&phase) {
        GaitPlant::Plant
    } else if touching && (phase <= 0.0 || phase >= 0.55) {
        GaitPlant::Lift
    } else {
        GaitPlant::Hold
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
pub(super) fn plant(
    legs: &mut [Leg],
    ankles: &[Vec3],
    spots: &[Option<Vec3>],
    bearing: &Bearing,
    had_support: bool,
    down: &[bool],
    work: &Work,
) -> bool {
    for (i, ((leg, &at), &down)) in legs.iter_mut().zip(ankles).zip(down).enumerate() {
        if bearing.walking {
            match gait_plant(
                gait_phase(bearing.progress, i),
                leg.state.has(FootState::TOUCHING),
            ) {
                GaitPlant::Plant => {
                    leg.anchor.x = at.x;
                    leg.anchor.z = at.z;
                    leg.state = FootState(FootState::TOUCHING);
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
    let ended = footwork(
        legs,
        ankles,
        &shifted,
        &Work {
            now_ms: 0.0,
            scale,
            crouch: 0.0,
            reference: frame,
            root: frame,
            origin,
            step_cm,
            episode: true,
            episode_ms: 0.0,
            chest_to_target_cm: cm((target - frame).with_y(0.0)).length(),
            size: 1.0,
        },
    );
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
fn footwork(legs: &mut [Leg], ankles: &[Vec3], spots: &[Option<Vec3>], work: &Work) -> bool {
    for (leg, &at) in legs.iter_mut().zip(ankles) {
        if leg.state.has(FootState::REACHED) {
            leg.state.0 &= !(FootState::ANCHORED | FootState::REACHED);
            leg.changed_ms = work.now_ms;
        } else if leg.state.has(FootState::ANCHORED) && f64::from(leg.approach) < STALLED_BELOW {
            leg.state.0 =
                leg.state.0 & FootState::RAMP_SLOW | FootState::TOUCHING | FootState::STALLED;
            leg.anchor = at;
            leg.landed_ms = work.now_ms;
            leg.changed_ms = work.now_ms;
        }
    }
    if legs.len() != 2 {
        return false;
    }
    let Some(pair) = two_spots(spots) else {
        return false;
    };
    let side = (pair[1] - pair[0]).with_y(0.0).normalize_or(Vec3::X);
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
        legs[other].landed_ms = work.now_ms + 100.0;
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
        legs[k].started_ms = work.now_ms;
    }
    legs[k].anchor = anchor;
    ended
}

impl Ragdoll {
    /// Leaving the walk: the rearmost foot (the smallest projection of its
    /// animated contact along the frame's forward) is anchored at its
    /// stance spot under the frame, keeping its height, and the other foot
    /// lands where it is. Both keep their slow ramp bit.
    pub fn plant_trailing_foot(&mut self) {
        let Ragdoll {
            parts, legs, drive, ..
        } = self;
        let Some(drive) = drive.as_ref() else {
            return;
        };
        let Some(frame) = drive.frame else {
            return;
        };
        if legs.is_empty() || drive.targets.len() != parts.len() {
            return;
        }
        let forward = Vec3::from(frame.matrix3.z_axis);
        let rear = legs
            .iter()
            .enumerate()
            .map(|(i, leg)| {
                let posed = drive.targets[leg.shin].transform_point3(leg.point);
                (i, posed.dot(forward))
            })
            .fold(None, |best: Option<(usize, f32)>, (i, along)| {
                Some(match best {
                    Some((_, b)) if b <= along => best.unwrap(),
                    _ => (i, along),
                })
            });
        let Some((rear, _)) = rear else {
            return;
        };
        for (i, leg) in legs.iter_mut().enumerate() {
            if i == rear {
                let posed = drive.targets[leg.shin].transform_point3(leg.point);
                let at = parts[leg.shin].point(&leg.contact);
                leg.anchor = Vec3::new(posed.x, at.y, posed.z);
                leg.at = at;
                leg.before = at;
                leg.state = FootState(
                    leg.state.0 & FootState::RAMP_SLOW | FootState::TOUCHING | FootState::ANCHORED,
                );
            } else if leg.state.has(FootState::ANCHORED) {
                let at = parts[leg.shin].point(&leg.contact);
                leg.anchor = at;
                leg.at = at;
                leg.before = at;
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
        landed_ms: leg.landed_ms,
        started_ms: leg.started_ms,
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

/// The radius (cm) of a foot's ground probe: a walking foot's 10 cm (its
/// gait's bump is not ported), otherwise 5 cm plus 10 times its blend.
pub(super) fn probe_radius(walking: bool, blend: f32) -> f32 {
    if walking {
        WALKING_RADIUS
    } else {
        PROBE_RADIUS + PROBE_RADIUS_BLEND * blend
    }
}

/// A foot's smoothed ground (cm) from the last one and the ground found
/// now: it rises at once and falls by `fall` (1 outside walking).
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
    // Two feet on one spot would make v 0/0 (Exarust's guard: level).
    let v = dy2 / (dy2 + dz * dz + dx * dx);
    let take = if v.is_nan() { 1.0 } else { 1.0 - v * v };
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

#[cfg(test)]
#[path = "body_feet_test.rs"]
mod tests;

/// What the end of a step leaves: where the step target is (the original
/// subtracts the goal from the chest and lets the level clamp it), and
/// whether the AI is told (its step direction turned toward its facing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepEnd {
    pub target: Vec3,
    pub tell_ai: bool,
}

/// The step's end (the original's step-end routine): with an AI target the
/// AI is told when the step direction points along the facing; the target
/// is the chest less the goal, which the level's scene and height queries
/// then clamp. The stepping flag, its lift and the foot timers are the
/// caller's to set.
pub fn step_end(ai_target: bool, step_dir: Vec3, facing: Vec3, chest: Vec3, goal: Vec3) -> StepEnd {
    StepEnd {
        target: chest - goal,
        tell_ai: ai_target && step_dir.dot(facing) > 0.0,
    }
}

#[cfg(test)]
mod step_end_tests {
    use super::*;

    #[test]
    fn the_step_end_targets_the_chest_less_the_goal() {
        let end = step_end(true, Vec3::Z, Vec3::Z, Vec3::new(1.0, 2.0, 3.0), Vec3::Z);
        assert_eq!(end.target, Vec3::new(1.0, 2.0, 2.0));
        assert!(end.tell_ai);
        assert!(!step_end(false, Vec3::Z, Vec3::Z, Vec3::ZERO, Vec3::ZERO).tell_ai);
        assert!(!step_end(true, Vec3::NEG_Z, Vec3::Z, Vec3::ZERO, Vec3::ZERO).tell_ai);
    }
}

impl Ragdoll {
    /// Clears bits `mask` from every foot contact's state, as the original
    /// clears part state bits when a motion starts or a get-up begins.
    pub fn clear_foot_state(&mut self, mask: u32) {
        for leg in &mut self.legs {
            leg.state = FootState(leg.state.0 & !mask);
        }
    }
}

/// The tone and stance fields a full reset writes: the stance factor and
/// tone 1, the stagger speed 1 and decay 0, and the getting-up condition
/// for the two get-up motions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FullTone {
    pub stance: f32,
    pub tone: f32,
    pub stagger_speed: f32,
    pub stagger_decay: f32,
    pub getting_up: bool,
}

/// The reset values (the original's routine): all 1, the stagger's decay 0,
/// and `true` for motions 0x0E and 0x0F (which sets condition 0x300).
pub fn full_tone(motion: i32) -> FullTone {
    FullTone {
        stance: 1.0,
        tone: 1.0,
        stagger_speed: 1.0,
        stagger_decay: 0.0,
        getting_up: motion == 0x0E || motion == 0x0F,
    }
}
