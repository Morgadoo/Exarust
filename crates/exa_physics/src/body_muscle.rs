//! Link muscles of a living body, as the original's. Every slack link
//! pulls its point on the part toward its anchor: where the animated joint
//! angle would put that point, given where the parent actually is. The
//! pull is a PD muscle with saturation, scaled by the link's strength, the
//! part's factor and the body's strength √E·T; beyond the slack the link's
//! excess is taken back at full weight on top. The character tick sets the
//! gains every tick: the stance factor and tone target, the thighs' and
//! arms' factors, the leg limit's strength, the anchors and how far they
//! relax toward the current pose.
//!
//! The rules are the original's, in its centimetres and ticks (evidence:
//! `docs/formats/RSQ.md`, Physical body).
//! (Included from body.rs: `#[path = "body_muscle.rs"] mod muscle;`)

use bevy::math::Affine3A;
use bevy::prelude::*;

use super::{CM, Drive, LINK_EPSILON2, Link, MAX_CORRECTION, PIVOT_SLACK, Part, Ragdoll, weights};

/// A part's link muscle factor, until the character tick sets the thighs'
/// and arms'.
pub(super) const PART_FACTOR: f32 = 0.5;
/// A foot's factor is this share of its lower leg's, the head's this share
/// of the thorax's.
const FOOT_SHARE: f32 = 0.5;
const HEAD_SHARE: f32 = 0.25;
/// Saturation c of a link muscle's error, per cm: e/(1 + c|e|).
pub(super) const SATURATION: f32 = 0.1;
/// Cap of a part's anchor relaxation, but the arms'.
pub(super) const RELAX_CAP: f32 = 1.0;
/// The damping gain is D·k/(0.5|e|² + 1.4), e in cm.
const DAMPING_SPREAD: f32 = 0.5;
const DAMPING_FLOOR: f32 = 1.4;
/// D = (E − 0.5)², and 0 below this energy.
const DAMPED_FROM: f32 = 0.5;
/// k while the body moves, and the numerator of the standing k (divided by
/// the race's stance factor: 80 for a human).
const MOVING_K: f32 = 50.0;
const STANDING_K: f32 = 40.0;
/// The pelvis's and the legs' damping share (bones below 8).
const LEG_DAMPING: f32 = 0.6;
/// The lowest stance factor the standing k divides by (a guard against
/// broken race data).
pub(super) const MIN_STANCE: f32 = 0.01;
/// A moving body's stance factor rises this much a tick toward
/// 0.7 + 0.3·run.
const WALK_STANCE: f32 = 0.7;
const RUN_STANCE: f32 = 0.3;
const STANCE_RISE: f32 = 0.01;
/// Limbs' factors scale by 1.25 times the body's scale (1 in Exarust).
const LIMB_SCALE: f32 = 1.25;
/// A thigh's factor: this moving, times [`LIMB_SCALE`] standing.
const THIGH_FACTOR: f32 = 0.2;
/// A leg limit link's strength moves this share of the way to its target
/// each tick: 0.5·S² standing, 0.4 moving.
const LEG_LIMIT_BLEND: f32 = 0.1;
const LEG_LIMIT_STANDING: f32 = 0.5;
const LEG_LIMIT_MOVING: f32 = 0.4;
/// Arm stiffness: off balance, and dazed (tone under 0.85) or striding. A
/// body is off balance below this balance.
const ARM_OFF_BALANCE: f32 = 0.5;
const OFF_BALANCE_BELOW: f32 = 0.6;
const ARM_LOOSE: f32 = 0.2;
const ARM_DAZED_TONE: f32 = 0.85;
/// Otherwise it eases each tick, x ← 0.8x + 0.08·(0.5 + 0.5·race arm
/// relaxation): to 0.4 for a human.
const ARM_KEEP: f32 = 0.8;
const ARM_EASE: f32 = 0.08;
/// While the body turns it is at least 0.4 × the turn left (rad), at most
/// 0.4.
const ARM_TURN: f32 = 0.4;
/// An arm part's anchor relaxation cap: 0.6 − (x − 0.1)·0.8.
const ARM_CAP_TOP: f32 = 0.6;
const ARM_CAP_FROM: f32 = 0.1;
const ARM_CAP_SLOPE: f32 = 0.8;
/// Anchor relaxation: the target is 0.6 − 32|Δa|² (cm², the most an anchor
/// moved since the last tick) times the race's factor, capped by the part's
/// cap; λ falls to it at once and rises 0.015 a tick.
const RELAX_TOP: f32 = 0.6;
const RELAX_MOTION: f32 = 32.0;
const RELAX_RISE: f32 = 0.015;
/// A supported leg's: clamp(0.6 − 0.001·| |hip − foot contact|² − (2·r)² |,
/// 0, 0.2) in cm², r the contact's distance from the lower leg's origin;
/// rising 0.01 a tick.
const LEG_RELAX_SPREAD: f32 = 0.001;
const LEG_RELAX_CAP: f32 = 0.2;
const LEG_RELAX_RISE: f32 = 0.01;
/// The leg's rest reach is this many times the contact's distance.
const LEG_REACH: f32 = 2.0;

/// The race's factors a body's muscles read (`races.rdb`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceMuscles {
    /// Stance factor: the share of its tone a standing body uses, and the
    /// divisor of its standing link-muscle damping (human 0.5).
    pub stance: f32,
    /// Scales how far link muscle anchors relax toward the current pose
    /// while the pose holds still, and the arms' relaxed stiffness (human
    /// 1).
    pub arm_relaxation: f32,
    /// Adds 0.05 × it to the balance score (1 in every race record).
    pub balance_bonus: f32,
}

impl RaceMuscles {
    pub const HUMAN: Self = Self {
        stance: 0.5,
        arm_relaxation: 1.0,
        balance_bonus: 1.0,
    };
}

impl Default for RaceMuscles {
    fn default() -> Self {
        Self::HUMAN
    }
}

/// What a living body's character tick knows of its character this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bearing {
    /// It moves under its own power: its stance factor rises toward
    /// 0.7 + 0.3·run, and its link muscles damp with k = 50.
    pub moving: bool,
    /// Run blend, 0 walking to 1 running.
    pub run: f32,
    /// It plays the walk or the run cycle: its arms hang loosely.
    pub striding: bool,
    /// It has lost its balance: falling, lying or getting up.
    pub off_balance: bool,
    /// It plays its get-up clip: its balance rises and it has support near
    /// the ground.
    pub getting_up: bool,
    /// How far it crouches, 0 to 1: its chest is measured 10 cm back at a
    /// full crouch.
    pub crouch: f32,
    /// Its character controller stands on the ground (a body without one
    /// counts as carried). Exarust's own: until the body stands on its own
    /// feet, the controller's capsule carries a standing body and may hold
    /// its feet off the ground (perched on a stone), so a carried standing
    /// body keeps its balance when no foot is grounded; a fallen one is not
    /// carried.
    pub carried: bool,
    /// What is left of its turn toward its heading (rad).
    pub turn: f32,
}

impl Default for Bearing {
    fn default() -> Self {
        Self {
            moving: false,
            run: 0.0,
            striding: false,
            off_balance: false,
            getting_up: false,
            crouch: 0.0,
            carried: true,
            turn: 0.0,
        }
    }
}

/// What a part is to the muscles, by its bone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Thigh,
    Shin,
    Foot,
    Head,
    Arm,
    Trunk,
}

fn role(bone: usize) -> Role {
    match bone {
        2 | 5 => Role::Thigh,
        3 | 6 => Role::Shin,
        4 | 7 => Role::Foot,
        10 => Role::Head,
        // Shoulders, upper arms, forearms and hands.
        11..=18 => Role::Arm,
        _ => Role::Trunk,
    }
}

/// The part factors a body starts with: a foot half its lower leg's, the
/// head a quarter of its thorax's, every other part 0.5.
pub(super) fn set_part_factors(parts: &mut [Part]) {
    for i in 0..parts.len() {
        let share = match role(parts[i].bone) {
            Role::Foot => FOOT_SHARE,
            Role::Head => HEAD_SHARE,
            _ => continue,
        };
        let parent = parts[i]
            .parent
            .and_then(|j| parts.get(j))
            .map_or(PART_FACTOR, |p| p.factor);
        parts[i].factor = parent * share;
    }
}

/// The stance factor after one tick: a standing body's is its race's; a
/// moving body's rises 0.01 a tick toward 0.7 + 0.3·run and keeps what it
/// reached above that.
pub(super) fn stance_factor(s: f32, bearing: &Bearing, race: &RaceMuscles) -> f32 {
    if !bearing.moving {
        return race.stance;
    }
    let target = WALK_STANCE + RUN_STANCE * bearing.run.clamp(0.0, 1.0);
    if s < target {
        (s + STANCE_RISE).min(target)
    } else {
        s
    }
}

/// The arms' stiffness after one tick, `x` last tick's: 0.5 off balance;
/// 0.2 dazed (tone under 0.85) or striding; else easing to
/// 0.4·(0.5 + 0.5·race arm relaxation). While the body turns it is at least
/// min(0.4·|turn left|, 0.4).
pub(super) fn arm_stiffness(x: f32, bearing: &Bearing, tone: f32, race: &RaceMuscles) -> f32 {
    let x = if bearing.off_balance {
        ARM_OFF_BALANCE
    } else if tone < ARM_DAZED_TONE || bearing.striding {
        ARM_LOOSE
    } else {
        ARM_KEEP * x + ARM_EASE * (0.5 + 0.5 * race.arm_relaxation)
    };
    x.max((ARM_TURN * bearing.turn.abs()).min(ARM_TURN))
}

/// An arm part's anchor relaxation cap at stiffness `x`.
pub(super) fn arm_relax_cap(x: f32) -> f32 {
    ARM_CAP_TOP - (x - ARM_CAP_FROM) * ARM_CAP_SLOPE
}

/// A relaxation `lambda` after one tick toward `target`: it rises by `rise`
/// and falls to the target at once; a negative target leaves it.
pub(super) fn relax(lambda: f32, target: f32, rise: f32) -> f32 {
    if target > lambda {
        lambda + rise
    } else if target >= 0.0 {
        target
    } else {
        lambda
    }
}

/// A supported leg's relaxation target, from the squared distance (cm²)
/// between its hip and its foot contact and the contact's distance `r`
/// (cm) from the lower leg's origin.
pub(super) fn leg_relax_target(hip_to_contact2: f32, r: f32) -> f32 {
    let reach2 = (LEG_REACH * r).powi(2);
    (RELAX_TOP - LEG_RELAX_SPREAD * (hip_to_contact2 - reach2).abs()).clamp(0.0, LEG_RELAX_CAP)
}

/// A link muscle's pull in one pass, in the original's centimetres: the gap
/// `error` from the part's point to its (relaxed) anchor, plus `motion`,
/// how that gap changed since the tick began, times
/// `damping`/(0.5|e|² + 1.4); then saturated to e/(1 + c|e|). `None` when
/// it is under 1.19e-7 cm², and the link then does nothing this pass.
pub(super) fn muscle_pull(
    error: Vec3,
    motion: Vec3,
    damping: f32,
    saturation: f32,
) -> Option<Vec3> {
    let e = error + motion * (damping / (DAMPING_SPREAD * error.length_squared() + DAMPING_FLOOR));
    let len2 = e.length_squared();
    if len2.is_nan() || len2 < LINK_EPSILON2 * CM * CM {
        return None;
    }
    Some(e / (1.0 + saturation * len2.sqrt()))
}

impl Drive {
    /// The strength scale A = √E·T and the damping gain D·k of this tick's
    /// link muscles: D = (E − 0.5)² (0 below E = 0.5), k = 50 while moving,
    /// else 40 over the race's stance factor.
    pub(super) fn muscle_gains(&self) -> (f32, f32) {
        let strength = self.energy.max(0.0).sqrt() * self.tone_target();
        let d = if self.energy >= DAMPED_FROM {
            (self.energy - DAMPED_FROM).powi(2)
        } else {
            0.0
        };
        let k = if self.bearing.moving {
            MOVING_K
        } else {
            STANDING_K / self.race.stance.max(MIN_STANCE)
        };
        (strength, d * k)
    }
}

impl Link {
    /// A living slack link's correction with its muscle, `anchor` being its
    /// weights on the parent: the part's pull toward the anchor (relaxed by
    /// the part's λ toward where its point is) with damping of how the gap
    /// changed since the tick began, saturated, times the link's strength,
    /// the part's factor and the body's strength; plus, beyond the slack,
    /// the excess at full weight. Cut to 80 cm.
    pub(super) fn muscle(
        &self,
        part: &Part,
        parent: &Part,
        anchor: [f32; 4],
        (strength, damping): (f32, f32),
    ) -> Option<Vec3> {
        let a = part.point(&self.on_part);
        let fixed = parent.point(&anchor);
        let target = fixed + (a - fixed) * part.relax;
        let (was, target_was) = self.history;
        let motion = (target - target_was) - (a - was);
        let damping = if part.bone < 8 {
            damping * LEG_DAMPING
        } else {
            damping
        };
        let pull = muscle_pull((target - a) * CM, motion * CM, damping, part.saturation)? / CM;
        let mut c = pull * (self.strength * part.factor * strength);
        let d = parent.point(&self.on_parent) - a;
        let len = d.length();
        if len > self.slack {
            c += d * ((len - self.slack) / len);
        }
        if !c.is_finite() {
            warn_once!("invalid link muscle correction");
            return None;
        }
        if c.length_squared() > MAX_CORRECTION * MAX_CORRECTION {
            warn_once!("excessive constraint delta: {:.2} m", c.length());
            return Some(c.clamp_length_max(MAX_CORRECTION));
        }
        Some(c)
    }
}

impl Ragdoll {
    /// The character tick's settings for the link muscles, once the pose
    /// is set and the support measured, in the original's order: stance
    /// factor, the legs' factors and limit strengths, the arms' stiffness,
    /// then the anchors and their relaxation (a supported leg's from its
    /// reach). A body whose balance is not measured has support 1 while it
    /// stands and 0 once fallen.
    pub(super) fn tune_muscles(&mut self) {
        let measured = self.measures_balance();
        let Ragdoll {
            parts,
            links,
            links_of,
            drive,
            ..
        } = self;
        let Some(drive) = drive.as_mut() else {
            return;
        };
        let bearing = drive.bearing;
        if !measured {
            drive.support = if drive.supported { 1.0 } else { 0.0 };
        }
        drive.stance = stance_factor(drive.stance, &bearing, &drive.race);
        let unsteady = Bearing {
            off_balance: bearing.off_balance
                || !drive.supported
                || drive.balance.value < OFF_BALANCE_BELOW,
            ..bearing
        };
        drive.arm = arm_stiffness(drive.arm, &unsteady, drive.tone, &drive.race);
        let (thigh, limit) = if bearing.moving {
            (THIGH_FACTOR, LEG_LIMIT_MOVING)
        } else {
            (
                THIGH_FACTOR * LIMB_SCALE,
                LEG_LIMIT_STANDING * drive.support * drive.support,
            )
        };
        for i in 0..parts.len() {
            match role(parts[i].bone) {
                Role::Shin if parts[i].foot.is_some() => {
                    if let Some(k) = parts[i].leg_limit.filter(|&k| k < links.len()) {
                        let s = &mut links[k].strength;
                        *s += (limit - *s) * LEG_LIMIT_BLEND;
                    }
                    if let Some(j) = parts[i].parent.filter(|&j| j < parts.len()) {
                        parts[j].factor = thigh;
                    }
                }
                Role::Arm => {
                    let p = &mut parts[i];
                    p.factor = drive.arm * LIMB_SCALE;
                    p.relax_cap = arm_relax_cap(drive.arm);
                    p.saturation = SATURATION;
                }
                _ => {}
            }
        }
        sample_anchors(
            parts,
            links,
            links_of,
            &drive.targets,
            drive.race.arm_relaxation,
        );
        if drive.support > 0.0 {
            relax_legs(parts);
        }
    }

    /// Where each living link muscle's anchor and point were when the tick
    /// began (after integration), the anchor relaxed by its part's λ: the
    /// muscle damps how the gap between them changes over the tick.
    pub(super) fn remember_anchors(&mut self) {
        let Ragdoll { parts, links, .. } = self;
        for l in links.iter_mut() {
            let Some(anchor) = l.anchor else {
                continue;
            };
            let (part, parent) = (&parts[l.part], &parts[l.parent]);
            let was = part.point_at_start(&l.on_part);
            let fixed = parent.point_at_start(&anchor);
            l.history = (was, fixed + (was - fixed) * part.relax);
        }
    }
}

/// Samples each slack link's anchor from this tick's pose: the part's point
/// placed by its animated transform relative to its parent's (the clip's
/// turn, at the part's origin), as weights on the parent. Each part's λ
/// then moves toward 0.6 − 32|Δa|² (the most one of its anchors moved since
/// the last tick, cm²) times the race's factor, capped by the part's cap.
fn sample_anchors(
    parts: &mut [Part],
    links: &mut [Link],
    links_of: &[std::ops::Range<usize>],
    targets: &[Affine3A],
    relaxation: f32,
) {
    if targets.len() != parts.len() {
        return;
    }
    for i in 0..parts.len() {
        let Some(j) = parts[i].parent.filter(|&j| j < parts.len() && j != i) else {
            continue;
        };
        let relative = targets[j].inverse() * targets[i];
        let (_, turn, _) = relative.to_scale_rotation_translation();
        let joint = Affine3A::from_rotation_translation(turn, parts[i].origin);
        let mut lowest = RELAX_TOP;
        let mut muscles = false;
        for l in &mut links[links_of[i].clone()] {
            if l.slack <= PIVOT_SLACK {
                continue;
            }
            muscles = true;
            let anchor = joint.transform_point3(parts[i].rest_point(&l.on_part));
            if let Some(kept) = l.kept {
                let moved = ((anchor - kept) * CM).length_squared();
                lowest = lowest.min(RELAX_TOP - RELAX_MOTION * moved);
            }
            l.kept = Some(anchor);
            l.anchor = Some(weights(&parts[j], anchor));
        }
        if muscles {
            let p = &mut parts[i];
            let target = (lowest * relaxation).min(p.relax_cap);
            p.relax = relax(p.relax, target, RELAX_RISE);
        }
    }
}

/// A supported leg's relaxation, from how far its hip is from its foot
/// contact against twice the contact's distance from the lower leg's
/// origin; the thigh takes the lower leg's.
fn relax_legs(parts: &mut [Part]) {
    for i in 0..parts.len() {
        let Some(contact) = parts[i].foot else {
            continue;
        };
        let Some(j) = parts[i].parent.filter(|&j| j < parts.len() && j != i) else {
            continue;
        };
        let hip = Vec3::from(parts[j].transform().translation);
        let at = parts[i].transform().transform_point3(contact);
        let target = leg_relax_target(((at - hip) * CM).length_squared(), contact.length() * CM);
        let lambda = relax(parts[i].relax, target, LEG_RELAX_RISE);
        parts[i].relax = lambda;
        parts[j].relax = lambda;
    }
}

#[cfg(test)]
#[path = "body_muscle_test.rs"]
mod tests;
