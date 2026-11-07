//! The support pins. In each solver iteration a living body with support
//! pulls its two thorax pins once, and its pelvis pin twice, horizontally
//! toward goals the body tick's carry sets once a tick, as the original's
//! sets its pin records' targets: the posed points on the character frame,
//! shifted forward without support (the thorax pins), led toward a
//! recovery step, then clamped about the feet centre. The second pelvis
//! pull sees the first push. With no support the pull is zero.
//!
//! One application, in the original's centimetres ([`cm`]): after the
//! clamp, raw is 0.2·e·|e|/60 when |e| < 60 cm and 12 cm along the error
//! otherwise; subtract the point's horizontal displacement this tick; cap
//! the length at 8 cm; multiply by S² · pin-gain · P · the spawned pin
//! factor. A horizontal gap under 1.19e-7 cm² leaves the pin, damper
//! included. The pull does not run while the body is walking or off its
//! feet (down, getting up, dormant or dead). P is √E·T·E⁴, or 1 while the
//! shock-stagger flag is set.
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
/// A horizontal gap whose squared length is below this (cm², the
/// single-precision epsilon) skips the spring and the damper.
const GAP_EPSILON2: f32 = f32::from_bits(0x33ff_ffd7);
/// With no support a target shifts this far (cm) forward, times scale.
const UNSUPPORTED_AHEAD: f32 = 5.0;
/// The pelvis pin stays within this (cm) of the feet centre, times scale.
const PELVIS_REACH: f32 = 60.0;
/// A thorax target's reach (cm) before crouch and scale: 60 · 0.6, and the
/// crouch takes the whole of it off standing, half of it while stepping.
const THORAX_SPAN: f32 = 60.0;
const THORAX_SHARE: f64 = 0.6;
const CROUCH_OFF: f32 = 60.0;
const STEP_CROUCH_OFF: f32 = 30.0;

/// How far (cm) a thorax target may sit from the feet centre: (60 − 60·x),
/// or (60 − 30·x) while the body's step episode runs, times 0.6 in double,
/// times scale (x the crouch). Not floored: the clamp squares it.
pub(super) fn thorax_reach(crouch: f32, stepping: bool, scale: f32) -> f32 {
    let span = if stepping {
        THORAX_SPAN - crouch * STEP_CROUCH_OFF
    } else {
        THORAX_SPAN - crouch * CROUCH_OFF
    };
    (f64::from(span) * THORAX_SHARE) as f32 * scale
}

/// How far (cm) the pelvis pin may sit from the feet centre.
pub(super) fn pelvis_reach(scale: f32) -> f32 {
    PELVIS_REACH * scale
}

/// P = √E · T · E⁴, E the energy and T the tone target, unless
/// `shock_stagger` is set, when it is 1. The root is taken as the energy
/// is, so a NaN or a negative stays a NaN.
pub(super) fn power(energy: f32, tone_target: f32, shock_stagger: bool) -> f32 {
    if shock_stagger {
        return 1.0;
    }
    let squared = energy * energy;
    let fourth = squared * squared;
    energy.sqrt() * tone_target * fourth
}

/// The gains one pin application scales by.
#[derive(Clone, Copy)]
pub(super) struct Factors {
    pub has_support: bool,
    pub support: f32,
    pub pin_gain: f32,
    pub power: f32,
    pub pin_factor: f32,
}

/// One application of a support pin's horizontal pull, in centimetres.
/// `error` is the gap from the point to its clamped target and
/// `displacement` is how far the point has moved this tick, both
/// horizontal. A gap under 1.19e-7 cm² leaves it, damper included.
/// Nothing without support, and nothing that is not finite.
pub(super) fn pull(error: Vec3, displacement: Vec3, gain: Factors) -> Vec3 {
    if !gain.has_support {
        return Vec3::ZERO;
    }
    let e = error.with_y(0.0);
    let moved = displacement.with_y(0.0);
    // z² + (y² + x²), and only a gap ordered below the epsilon returns.
    // A NaN is not below it.
    let len2 = e.z * e.z + (e.y * e.y + e.x * e.x);
    if len2 < GAP_EPSILON2 {
        return Vec3::ZERO;
    }
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
    let scale = pin_scale(gain);
    if !scale.is_finite() {
        return Vec3::ZERO;
    }
    corr * scale
}

/// The gains' product in the original's order: S²·(G·(P·F)).
pub(super) fn pin_scale(gain: Factors) -> f32 {
    let squared = gain.support * gain.support;
    squared * (gain.pin_gain * (gain.power * gain.pin_factor))
}

/// A thorax goal without support (or while jumping, which 0.9.5 never
/// does): `posed` shifted 5·scale cm along the frame's `forward` axis.
pub(super) fn unsupported_shift(posed: Vec3, forward: Vec3, scale: f32) -> Vec3 {
    posed + forward * (UNSUPPORTED_AHEAD * scale / CM)
}

/// A goal clamped across the ground to `reach_cm` of the feet `centre`:
/// past it (the squared reach ordered below the squared horizontal
/// distance) the goal is the centre plus the horizontal offset scaled to
/// the reach, at the centre's height; otherwise it stays as it is.
pub(super) fn clamp_goal(goal: Vec3, centre: Vec3, reach_cm: f32) -> Vec3 {
    let d = cm((goal - centre).with_y(0.0));
    let d2 = d.length_squared();
    if reach_cm * reach_cm < d2 {
        centre + d * (reach_cm / d2.sqrt()) / CM
    } else {
        goal
    }
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

/// The frame's forward axis: the yaw frame's third axis, as it is.
fn forward_axis(drive: &Drive) -> Vec3 {
    drive
        .frame
        .or(drive.root)
        .map_or(Vec3::Z, |t| Vec3::from(t.matrix3.z_axis))
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

/// One support pin's goal for the tick: the part, the pin's weights on
/// it, the goal (world, m) and whether it is a thorax pin (else the
/// pelvis's).
#[derive(Debug, Clone, Copy)]
pub(super) struct PinGoal {
    pub part: usize,
    pub weights: [f32; 4],
    pub goal: Vec3,
    pub thorax: bool,
}

/// The goals of one carry: the pins' goals, the thorax goals' mean and the
/// exertion the last thorax lead wrote.
pub(super) struct Goals {
    pub pins: Vec<PinGoal>,
    pub mean: Vec3,
    pub exertion: Option<f32>,
}

impl Ragdoll {
    /// The support pins' goals, as the body tick's carry sets the pin
    /// records' targets once a tick (read in the assembly), in its order:
    /// the pelvis pin and then the thorax pins. Each starts at its posed
    /// point on the character frame. A thorax goal without support moves
    /// 5·scale cm along the frame's forward axis. While a step leads (the
    /// episode runs and the chest is not out of balance) a goal is led
    /// toward the step from the pin's world point ([`steps::lead_pose`],
    /// [`steps::pelvis_lead_pose`]), the thorax leads writing the exertion
    /// value. Then each is clamped about the feet centre: the thorax
    /// goals to [`thorax_reach`], the pelvis's to [`pelvis_reach`]. The
    /// thorax goals' mean is kept (the frame's origin without any).
    pub(super) fn pin_goals(&self) -> Option<Goals> {
        let drive = self.drive.as_ref()?;
        if drive.targets.len() != self.parts.len() {
            return None;
        }
        // The feet centre: the mean of the contacts the balance routine
        // averages; a body with fewer than two legs (the hovering wraith)
        // has the one its balance routine left, its pelvis.
        let centre = if self.legs.len() < 2 && drive.race.hovers {
            drive.balance.feet_centre()
        } else {
            feet_centre(&self.legs, &self.parts)?
        };
        let has_support = drive.balance.has_support();
        let scale = drive.stats.scale;
        let lead = (drive.steps.stepping && !drive.steps.out_of_balance).then(|| {
            // The step's offset is the target less the standing point the
            // anchor routine keeps, which a step sets onto the pelvis every
            // tick (the frame's origin): the lead fades as the body reaches
            // its target.
            (
                drive.steps.target,
                drive.anchor.unwrap_or(drive.steps.standing),
                steps::ChestLead {
                    idle: drive.bearing.idle,
                    level: drive.balance.level(),
                    scale,
                    fast: drive.balance.fast_velocity(),
                    side: side_axis(drive),
                    forward: forward_axis(drive),
                    kind: if drive.steps.knockback {
                        steps::KNOCKBACK_KIND
                    } else {
                        steps::LEAD_KIND
                    },
                    long: drive.steps.knockback,
                    length_cm: drive.stats.foot_reach() * drive.balance.extension(),
                },
            )
        });
        let forward = facing(drive);
        let mut pins = Vec::with_capacity(self.chest.len() + 1);
        if let Some(i) = self.parts.iter().position(|p| p.bone == 1) {
            let part = &self.parts[i];
            let w = part.own_origin;
            let mut goal = drive.targets[i].transform_point3(part.rest_point(&w));
            if let Some((step_at, standing, chest)) = &lead {
                goal = steps::pelvis_lead_pose(goal, *step_at, *standing, part.point(&w), chest);
            }
            pins.push(PinGoal {
                part: i,
                weights: w,
                goal: clamp_goal(goal, centre, pelvis_reach(scale)),
                thorax: false,
            });
        }
        // The thorax reach narrows by the crouch less while the body's
        // step episode runs.
        let reach = thorax_reach(drive.bearing.crouch, drive.steps.stepping, scale);
        let mut sum = Vec3::ZERO;
        let mut count = 0;
        let mut exertion = None;
        for &(i, w) in &self.chest {
            if i >= self.parts.len() {
                continue;
            }
            let part = &self.parts[i];
            let mut goal = drive.targets[i].transform_point3(part.rest_point(&w));
            if !has_support {
                goal = unsupported_shift(goal, forward, scale);
            }
            if let Some((step_at, standing, chest)) = &lead {
                let (led, wrote) =
                    steps::lead_pose(goal, *step_at, *standing, part.point(&w), chest);
                goal = led;
                exertion = wrote.or(exertion);
            }
            let goal = clamp_goal(goal, centre, reach);
            sum += goal;
            count += 1;
            pins.push(PinGoal {
                part: i,
                weights: w,
                goal,
                thorax: true,
            });
        }
        let mean = if count > 0 {
            sum * (1.0 / count as f32)
        } else {
            drive
                .frame
                .map_or(Vec3::from(drive.targets[0].translation), |f| {
                    Vec3::from(f.translation)
                })
        };
        Some(Goals {
            pins,
            mean,
            exertion,
        })
    }

    /// The body tick's carry of the pin goals: [`Ragdoll::pin_goals`] kept
    /// for the solver's pulls, the mean for the stance score, the exertion
    /// for the character's locomotion state.
    pub(super) fn set_pin_goals(&mut self) {
        let goals = self.pin_goals();
        let Some(drive) = self.drive.as_mut() else {
            return;
        };
        match goals {
            Some(g) => {
                drive.pin_goals = g.pins;
                drive.pin_mean = Some(g.mean);
                if g.exertion.is_some() {
                    drive.lead_exertion = g.exertion;
                }
            }
            None => drive.pin_goals.clear(),
        }
    }

    /// The exertion value the last carry's thorax lead wrote, taken once.
    pub fn take_lead_exertion(&mut self) -> Option<f32> {
        self.drive.as_mut().and_then(|d| d.lead_exertion.take())
    }

    /// One pass of the support pins: the two thorax pins once and the
    /// pelvis pin twice, each by [`pull`] toward its goal of the tick
    /// ([`Ragdoll::set_pin_goals`]; a body not carried this tick takes
    /// them afresh). The second pelvis pull is read after the first push.
    pub(super) fn pull_support_pins(&mut self) {
        let Some(drive) = self.drive.as_ref() else {
            return;
        };
        // Walking, or off its feet (down, getting up, dormant or dead), the
        // pin loops do not run. A shock stagger does not reopen them.
        if drive.bearing.walking || drive.bearing.off_feet {
            return;
        }
        if drive.targets.len() != self.parts.len() {
            return;
        }
        let has_support = drive.balance.has_support();
        let gain = Factors {
            has_support,
            support: drive.support,
            pin_gain: drive.pin_gain,
            power: power(
                drive.energy,
                drive.tone_target(),
                drive.bearing.shock_stagger,
            ),
            pin_factor: drive.stats.pin_factor,
        };
        let goals = if drive.pin_goals.is_empty() {
            match self.pin_goals() {
                Some(g) => g.pins,
                None => return,
            }
        } else {
            drive.pin_goals.clone()
        };
        // The pin records are applied in sequence, so each pull reads the
        // pushes before it: the second thorax pin sees the first's push.
        // The pelvis origin is then pulled a second time, after its first.
        let order = goals
            .iter()
            .filter(|g| g.thorax)
            .chain(goals.iter().filter(|g| !g.thorax))
            .chain(goals.iter().filter(|g| !g.thorax));
        for g in order {
            let part = &self.parts[g.part];
            let at = part.point(&g.weights);
            let disp = part.slip(&g.weights);
            let corr = pull(cm((g.goal - at).with_y(0.0)), cm(disp.with_y(0.0)), gain);
            if corr.is_finite() && corr.length_squared() > 0.0 {
                self.parts[g.part].push(&g.weights, corr / CM);
            }
        }
    }
}
