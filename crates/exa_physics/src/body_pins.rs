//! The support pins. In each solver iteration a living body with support
//! pulls its two thorax pins and its pelvis pin horizontally toward targets
//! on the character frame, clamped near the feet centre. With no support
//! the pull is zero and the targets only shift forward.
//!
//! One application, in the original's centimetres ([`cm`]): after the
//! clamp, raw is 0.2·e·|e|/60 when |e| < 60 cm and 12 cm along the error
//! otherwise; subtract the point's horizontal displacement this tick; cap
//! the length at 8 cm; multiply by S² · pin-gain · P · the spawned pin
//! factor, and by 0.3 while getting up. P is √E·T·E⁴, or 1 while the
//! shock-stagger flag is set. The 0.3 is only the
//! get-up factor: it is not stacked on the pin-gain while the body stands.
//! Evidence: `docs/formats/RSQ.md` (Physical body).
//! (Included from body.rs: `#[path = "body_pins.rs"] mod pins;`)

use bevy::prelude::*;

use super::balance::{Leg, cm};
use super::steps;
use super::{CM, Drive, Part, Ragdoll};

/// The spring's length (cm) below which it is 0.2·e·|e|/60, and the
/// constant pull (cm) along the error at that length and beyond.
const SOFT_REACH: f32 = 60.0;
const SOFT_GAIN: f32 = 0.2;
const FAR_PULL: f32 = 12.0;
/// The correction (cm), before the gains, is no longer than this.
const CAP: f32 = 8.0;
/// While getting up the pull is this much of the standing one.
const GET_UP: f32 = 0.3;
/// With no support a target shifts this far (cm) forward, times scale.
const UNSUPPORTED_AHEAD: f32 = 5.0;
/// The pelvis pin stays within this (cm) of the feet centre, times scale.
const PELVIS_REACH: f32 = 60.0;
/// A thorax target's reach (cm) before crouch and scale: 60 · 0.6, and the
/// crouch takes the whole of it off standing, half of it while stepping.
const THORAX_SPAN: f32 = 60.0;
const THORAX_SHARE: f32 = 0.6;
const CROUCH_OFF: f32 = 60.0;
const STEP_CROUCH_OFF: f32 = 30.0;

/// How far (cm) a thorax target may sit from the feet centre.
pub(super) fn thorax_reach(crouch: f32, stepping: bool, scale: f32) -> f32 {
    let off = if stepping {
        STEP_CROUCH_OFF
    } else {
        CROUCH_OFF
    };
    ((THORAX_SPAN - off * crouch) * THORAX_SHARE * scale).max(0.0)
}

/// How far (cm) the pelvis pin may sit from the feet centre.
pub(super) fn pelvis_reach(scale: f32) -> f32 {
    (PELVIS_REACH * scale).max(0.0)
}

/// P = √E · T · E⁴, E the energy and T the tone target, unless
/// `shock_stagger` is set, when it is 1. The root is taken as the energy
/// is, so a NaN or a negative stays a NaN.
pub(super) fn power(energy: f32, tone_target: f32, shock_stagger: bool) -> f32 {
    if shock_stagger {
        return 1.0;
    }
    let squared = energy * energy;
    energy.sqrt() * tone_target * squared * squared
}

/// The gains one pin application scales by.
pub(super) struct Factors {
    pub has_support: bool,
    pub support: f32,
    pub pin_gain: f32,
    pub power: f32,
    pub pin_factor: f32,
    pub getting_up: bool,
}

/// One application of a support pin's horizontal pull, in centimetres.
/// `error` is the gap from the point to its clamped target and
/// `displacement` is how far the point has moved this tick, both
/// horizontal. Nothing without support, and nothing that is not finite.
pub(super) fn pull(error: Vec3, displacement: Vec3, gain: Factors) -> Vec3 {
    if !gain.has_support {
        return Vec3::ZERO;
    }
    let e = error.with_y(0.0);
    let moved = displacement.with_y(0.0);
    let len = e.length();
    if !len.is_finite() || !moved.is_finite() {
        return Vec3::ZERO;
    }
    let raw = if len < SOFT_REACH {
        e * (SOFT_GAIN * len / SOFT_REACH)
    } else {
        e * (FAR_PULL / len)
    };
    let mut corr = raw - moved;
    let clen = corr.length();
    if !clen.is_finite() {
        return Vec3::ZERO;
    }
    if clen > CAP {
        corr *= CAP / clen;
    }
    let mut scale = gain.support * gain.support * gain.pin_gain * gain.power * gain.pin_factor;
    if gain.getting_up {
        scale *= GET_UP;
    }
    if !scale.is_finite() {
        return Vec3::ZERO;
    }
    corr * scale
}

/// The target a pin is pulled toward. With support, `posed` clamped
/// horizontally to `radius_cm` of the feet `centre`. Without it, `posed`
/// shifted 5·scale cm along `forward`, and the pull itself is zero.
pub(super) fn target(
    posed: Vec3,
    centre: Vec3,
    radius_cm: f32,
    has_support: bool,
    forward: Vec3,
    scale: f32,
) -> Vec3 {
    if !has_support {
        let ahead = forward.with_y(0.0).normalize_or(Vec3::Z);
        return posed + ahead * (UNSUPPORTED_AHEAD * scale / CM);
    }
    let mut d = (posed - centre).with_y(0.0);
    let len = cm(d).length();
    if radius_cm <= 0.0 {
        d = Vec3::ZERO;
    } else if len > radius_cm && len.is_finite() {
        d *= radius_cm / len;
    }
    Vec3::new(centre.x + d.x, posed.y, centre.z + d.z)
}

/// The feet centre: the mean of the ankle contacts.
fn feet_centre(legs: &[Leg], parts: &[Part]) -> Option<Vec3> {
    if legs.is_empty() {
        return None;
    }
    let sum = legs
        .iter()
        .map(|l| parts[l.shin].point(&l.contact))
        .sum::<Vec3>();
    Some(sum / legs.len() as f32)
}

/// The frame's horizontal facing, or the root's, or forward.
fn facing(drive: &Drive) -> Vec3 {
    drive
        .frame
        .or(drive.root)
        .map(|t| {
            Vec3::from(t.matrix3.z_axis)
                .with_y(0.0)
                .normalize_or(Vec3::Z)
        })
        .unwrap_or(Vec3::Z)
}

/// The frame's horizontal side axis: the yaw frame's first axis.
fn side_axis(drive: &Drive) -> Vec3 {
    drive
        .frame
        .or(drive.root)
        .map(|t| {
            Vec3::from(t.matrix3.x_axis)
                .with_y(0.0)
                .normalize_or(Vec3::X)
        })
        .unwrap_or(Vec3::X)
}

impl Ragdoll {
    /// One pass of the support pins: the two thorax pins and the pelvis
    /// pin, each by [`pull`] toward [`target`].
    pub(super) fn pull_support_pins(&mut self) {
        let Some(drive) = self.drive.as_ref() else {
            return;
        };
        if drive.targets.len() != self.parts.len() || self.legs.is_empty() {
            return;
        }
        let Some(centre) = feet_centre(&self.legs, &self.parts) else {
            return;
        };
        let has_support = drive.balance.has_support();
        let support = drive.support;
        let pin_gain = drive.pin_gain;
        let power = power(
            drive.energy,
            drive.tone_target(),
            drive.bearing.shock_stagger,
        );
        let factor = drive.stats.pin_factor;
        let scale = drive.stats.scale;
        let getting_up = drive.bearing.getting_up;
        let lead = (drive.steps.stepping && !drive.steps.out_of_balance).then(|| {
            (
                drive.steps.target,
                drive.steps.standing,
                steps::ChestLead {
                    idle: drive.bearing.idle,
                    level: drive.balance.level(),
                    scale,
                    fast: drive.balance.fast_velocity(),
                    side: side_axis(drive),
                    kind: if drive.steps.knockback {
                        steps::KNOCKBACK_KIND
                    } else {
                        steps::LEAD_KIND
                    },
                },
            )
        });
        let reach = thorax_reach(
            drive.bearing.crouch,
            self.legs.iter().any(|l| l.state.stepping()),
            scale,
        );
        let pelvis = pelvis_reach(scale);
        let forward = facing(drive);
        let targets = drive.targets.clone();
        let mut pins: Vec<(usize, [f32; 4], f32, bool)> = self
            .chest
            .iter()
            .copied()
            .map(|(i, w)| (i, w, reach, true))
            .collect();
        if let Some(i) = self.parts.iter().position(|p| p.bone == 1) {
            pins.push((i, self.parts[i].own_origin, pelvis, false));
        }
        // Read every pin before any push, so each application sees the
        // same tick and not a sibling pin's correction.
        let planned: Vec<(usize, [f32; 4], Vec3)> = pins
            .into_iter()
            .filter(|(i, _, _, _)| *i < targets.len())
            .map(|(i, w, radius, thorax)| {
                let at = self.parts[i].point(&w);
                let mut posed = targets[i].transform_point3(self.parts[i].rest_point(&w));
                if thorax && let Some((step_at, standing, chest)) = &lead {
                    posed = steps::lead_pose(posed, *step_at, *standing, at, chest);
                }
                let goal = target(posed, centre, radius, has_support, forward, scale);
                let disp = self.parts[i].slip(&w);
                let corr = pull(
                    cm((goal - at).with_y(0.0)),
                    cm(disp.with_y(0.0)),
                    Factors {
                        has_support,
                        support,
                        pin_gain,
                        power,
                        pin_factor: factor,
                        getting_up,
                    },
                );
                (i, w, corr)
            })
            .filter(|(_, _, c)| c.is_finite() && c.length_squared() > 0.0)
            .collect();
        for (i, w, corr) in planned {
            self.parts[i].push(&w, corr / CM);
        }
    }
}
