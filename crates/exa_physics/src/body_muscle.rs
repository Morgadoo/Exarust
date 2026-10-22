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

use super::locomotion::{GaitRow, GaitTable, LegMode};
use super::{
    CM, Drive, LINK_EPSILON2, Link, MAX_CORRECTION, MAX_ENERGY, PIVOT_SLACK, Part, Ragdoll, weights,
};

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
/// D = (E − 0.5)² unless that difference compares below 0, when it is 0.
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
/// 0.7 + 0.3·run (double precision); a dormant or shock-staggered body's
/// is 1.
const WALK_STANCE: f64 = 0.7;
const RUN_STANCE: f64 = 0.3;
const STANCE_RISE: f64 = 0.01;
const FULL_STANCE: f32 = 1.0;
/// Limbs' factors scale by 1.25 times the body's scale
/// ([`BodyStats::scale`]).
const LIMB_SCALE: f32 = 1.25;
/// A thigh's factor: this walking, [`THIGH_RUNNING`] running, times the
/// limbs' 1.25·scale (double precision) standing or backpedalling.
const THIGH_FACTOR: f32 = 0.2;
const THIGH_RUNNING: f32 = 0.6;
const THIGH_STANDING: f64 = 0.2;
const THIGH_JUMP: f64 = 0.4;
/// A leg limit link's strength s ← 0.9s + 0.1t (double precision) each
/// tick: t = 0.5·S² standing or backpedalling, 0.4 walking or running;
/// while stepping with the foot anchored s ← 0.8s + 0.04 instead.
const LEG_LIMIT_KEEP: f64 = 0.9;
const LEG_LIMIT_TAKE: f64 = 0.1;
const LEG_LIMIT_STANDING: f32 = 0.5;
const LEG_LIMIT_MOVING: f32 = 0.4;
const STEP_LIMIT_KEEP: f64 = 0.8;
const STEP_LIMIT_TAKE: f64 = 0.04;
/// A powered golem suit halves the standing thigh factor and quarters the
/// leg limit's standing target.
const GOLEM_THIGH: f32 = 0.5;
const GOLEM_LIMIT: f32 = 0.25;
/// Arm stiffness: off balance, and dazed (tone under 0.85) or striding
/// (the walking arms of a character that is not feral). A body is off
/// balance below this balance.
const ARM_OFF_BALANCE: f32 = 0.5;
const OFF_BALANCE_BELOW: f32 = 0.6;
const ARM_LOOSE: f32 = 0.2;
const ARM_DAZED_TONE: f32 = 0.85;
/// A human's support pins' factor (its race's 1.25).
const HUMAN_PINS: f32 = 1.25;
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
    /// Limb damping ratio r (`races.rdb` +0x80). A living limb's fast
    /// retention is multiplied by u², or by 0.5·u + 0.5 while walking or
    /// off its feet, with u = (1 − balance)(1 − r) + r (1 for a human,
    /// 0.95 for a golem).
    pub damping_ratio: f32,
    /// The base rate of the clip clock: frames a tick before what the body
    /// does multiplies it (1 for a human).
    pub clip_rate: f32,
    /// The walk, jog, sprint and backpedal rows of its gait table, per
    /// tick (`GaitRow::scaled`; a human's clock rates 27, 16.6, 13.5 and
    /// 28 frames a second).
    pub gaits: [GaitRow; 4],
}

impl RaceMuscles {
    pub const HUMAN: Self = Self {
        stance: 0.5,
        arm_relaxation: 1.0,
        balance_bonus: 1.0,
        damping_ratio: 1.0,
        clip_rate: 1.0,
        gaits: GaitTable::HUMAN_ROWS,
    };
}

impl Default for RaceMuscles {
    fn default() -> Self {
        Self::HUMAN
    }
}

/// What a living body takes from its own character, as the character's
/// init derives it when the character spawns (from its record's height
/// and feral level), beside its race's factors ([`RaceMuscles`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyStats {
    /// The most energy it recovers to after a daze: 1, or 1 − 0.1 × its
    /// feral level.
    pub max_energy: f32,
    /// Its body scale: its limbs' muscle factors are 1.25 times it.
    pub scale: f32,
    /// Scales its head's pose-drive gain: 1, or 1 − its feral level.
    pub head_gain: f32,
    /// Scales its head's muscle factor: 1, or 1 − 0.8 × its feral level.
    pub head_factor: f32,
    /// Its arms' stiffness in the walk and run cycles: 0.2, or 0.2 − 0.15
    /// × its feral level, at least 0.05.
    pub walking_arms: f32,
    /// Its support pins' factor (its race's, less 0.25 × its feral
    /// level²): it scales its feet's grip and its support pins.
    pub pin_factor: f32,
}

impl BodyStats {
    /// A human of average height who is not feral.
    pub const HUMAN: Self = Self {
        max_energy: MAX_ENERGY,
        scale: 1.0,
        head_gain: 1.0,
        head_factor: 1.0,
        walking_arms: ARM_LOOSE,
        pin_factor: HUMAN_PINS,
    };
}

impl Default for BodyStats {
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
    /// The speed along the heading (cm a tick). Moving, the two-foot
    /// blend starts at min(0.5, 0.2·speed) and is then multiplied by
    /// min(1, (0.3·speed)²).
    pub speed: f32,
    /// How its legs move this tick, its movement decision's leg mode
    /// (standing unless it locomotes): the thighs' factors and the leg
    /// limits follow it, and its stance factor rises only while it is not
    /// standing.
    pub legs: LegMode,
    /// The run blend before this tick's locomotion, which the stance
    /// factor's target reads.
    pub stance_run: f32,
    /// Its condition is dormant or its state is exactly the shock's
    /// stagger: standing, its stance factor is 1.
    pub full_stance: bool,
    /// The shock-stagger flag is set: the support pins' power is 1.
    pub shock_stagger: bool,
    /// It plays the walk or the run cycle: its arms hang loosely. A shield
    /// in the off hand doubles that arm.
    pub striding: bool,
    /// It has lost its balance: falling, lying or getting up.
    pub off_balance: bool,
    /// It plays its get-up clip: its balance rises and it has support near
    /// the ground.
    pub getting_up: bool,
    /// Its get-up clip is in its first quarter: its frame follows its pelvis
    /// within 10 cm, and not at all for the rest of the get-up.
    pub rising: bool,
    /// How far it crouches, 0 to 1: its chest is measured 10 cm back at a
    /// full crouch.
    pub crouch: f32,
    /// Its character controller stands on the ground (a body without one
    /// counts as carried). Exarust's own: the controller's capsule carries
    /// a standing body across the ground and may stand where its feet do
    /// not (perched on a stone), so a carried standing body keeps its
    /// balance while no foot is grounded, dropping onto its feet; a fallen
    /// one is not carried. The lift reads each foot's own ground; the
    /// controller's floor does not raise it, and the legs bend onto it.
    pub carried: bool,
    /// What is left of its turn toward its heading (rad).
    pub turn: f32,
    /// How high (m) its lowest ankle sits in the gait pose a moving body
    /// stands on: the walk's first key, that key slerped 0.7 of the way
    /// toward the run's first key, or the run's first key.
    pub gait_floor: Option<f32>,
    /// It collapses from a lethal wound: its frame follows its pelvis
    /// exactly, its lift fades by its tone squared, its arms' stiffness is
    /// 0.2, its feet are probed standing, and its support pins' gain does
    /// not recover.
    pub collapsing: bool,
    /// Its condition is walking (the walk or the run cycle plays at full
    /// tone): its feet's contacts are pushed by the velocity servo, not
    /// gripped, and are probed as a walking foot's. Its support pins do
    /// not pull.
    pub walking: bool,
    /// Its condition is off its feet: down, getting up, dormant or dead
    /// (the condition's bit 0x200). Its feet are probed from their ankles
    /// and their soles are not held over the floor. Its support pins do
    /// not pull.
    pub off_feet: bool,
    /// How far its clip has played, 0 to 1 (by last tick's clock): a
    /// get-up's feet do not touch in its first fifth, and stand on their
    /// stance spots until nine tenths.
    pub progress: f32,
    /// It plays its Idle clip: the thorax lead's clip share is 0.4 then,
    /// and 0.7 otherwise.
    pub idle: bool,
    /// Its head looks this tick (its sampler's head look ran): the head's
    /// link anchors are placed relative to the thorax where it physically
    /// is, not where its pose puts it.
    pub head_looks: bool,
    /// The locomotion drive this tick (cm a tick²) and its push gain (the
    /// gait table's push column), which the carry adds to every part while
    /// it moves: drive × S² × push × the feet's lag.
    pub push: Vec3,
    pub push_gain: f32,
    /// The feet's lag factor, 0 to 1: a foot that trails its animated spot
    /// cuts the push.
    pub lag: f32,
    /// The body's facing across the ground (negated while it backpedals),
    /// which the feet's lag is measured along.
    pub facing: Vec3,
    /// It is stepping: an anchored stepping foot's leg limit eases to 0.2
    /// instead of following the leg mode.
    pub stepping: bool,
    /// It plays a kick, at the tone a standing body keeps. The arms are
    /// left as the last tick wrote them.
    pub kicking: bool,
    /// It is parrying. Each stored arm eases by 0.8x + 0.04 unless the
    /// body is collapsing, off balance, kicking, or an attack is setting
    /// the arms. A walk, a daze, a power and a weapon switch do not.
    /// The clip rate is twice the race base unless the body is off
    /// balance, getting up, attacking or weak.
    pub parrying: bool,
    /// It plays a power. Both arms are set to 0.3.
    pub casting: bool,
    /// It plays a weapon switch. Both arms are set to 0.4.
    pub switching: bool,
    /// Its golem suit is powered: the standing thigh factor and the leg
    /// limit's target are halved a second time.
    pub golem_powered: bool,
    /// A shield is in the off hand. While the body strides at full tone
    /// and on balance, that arm's stiffness is twice the walking arms and
    /// the weapon arm keeps them. A parry, a daze, a kick and an attack
    /// do not take this.
    pub shield: bool,
    /// The jump flag and its timer: the original never writes either, so
    /// the leg mode's jump share is dead; kept for the rule.
    pub jump: bool,
    pub jump_progress: f32,
}

impl Default for Bearing {
    fn default() -> Self {
        Self {
            moving: false,
            run: 0.0,
            speed: 0.0,
            legs: LegMode::Standing,
            stance_run: 0.0,
            full_stance: false,
            shock_stagger: false,
            striding: false,
            off_balance: false,
            getting_up: false,
            rising: false,
            crouch: 0.0,
            carried: true,
            turn: 0.0,
            gait_floor: None,
            collapsing: false,
            walking: false,
            off_feet: false,
            progress: 0.0,
            idle: false,
            head_looks: false,
            push: Vec3::ZERO,
            push_gain: 0.0,
            lag: 1.0,
            facing: Vec3::Z,
            stepping: false,
            kicking: false,
            parrying: false,
            casting: false,
            switching: false,
            golem_powered: false,
            shield: false,
            jump: false,
            jump_progress: 0.0,
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
/// head a quarter of its thorax's times `head` (its character's head
/// factor, 1 unless feral), every other part 0.5.
pub(super) fn set_part_factors(parts: &mut [Part], head: f32) {
    for i in 0..parts.len() {
        let share = match role(parts[i].bone) {
            Role::Foot => FOOT_SHARE,
            Role::Head => HEAD_SHARE * head,
            _ => continue,
        };
        let parent = parts[i]
            .parent
            .and_then(|j| parts.get(j))
            .map_or(PART_FACTOR, |p| p.factor);
        parts[i].factor = parent * share;
    }
}

/// The stance factor after one tick: a standing body's is its race's (1
/// dormant or in the shock's stagger); a locomoting body's rises 0.01 a
/// tick toward 0.7 + 0.3·run (double precision, the run blend before this
/// tick's locomotion), never past it, and keeps what it reached above it.
pub(super) fn stance_factor(s: f32, bearing: &Bearing, race: &RaceMuscles) -> f32 {
    if bearing.legs == LegMode::Standing {
        return if bearing.full_stance {
            FULL_STANCE
        } else {
            race.stance
        };
    }
    let target = (f64::from(bearing.stance_run) * RUN_STANCE + WALK_STANCE) as f32;
    if s < target {
        let s = (f64::from(s) + STANCE_RISE) as f32;
        if s > target { target } else { s }
    } else {
        s
    }
}

/// The thighs' factor and the leg limits' target this tick by the leg
/// mode: walking 0.2 and 0.4, running 0.6 and 0.4, otherwise 0.2 × the
/// limbs' factor (double precision) and 0.5·S².
fn leg_gains(legs: LegMode, limbs: f32, support: f32, bearing: &Bearing) -> (f32, f32) {
    match legs {
        LegMode::Walk => (THIGH_FACTOR, LEG_LIMIT_MOVING),
        LegMode::Run => (THIGH_RUNNING, LEG_LIMIT_MOVING),
        LegMode::Backpedal | LegMode::Standing => {
            // A jump raises the standing thigh factor from 0.2 to 0.4 while
            // its timer is under 1.5 (the flag is never written).
            let share = if bearing.jump && bearing.jump_progress < 1.5 {
                THIGH_JUMP
            } else {
                THIGH_STANDING
            };
            let mut thigh = (f64::from(limbs) * share) as f32;
            let mut limit = support * support * LEG_LIMIT_STANDING;
            if bearing.golem_powered {
                thigh *= GOLEM_THIGH;
                limit *= GOLEM_LIMIT;
            }
            (thigh, limit)
        }
    }
}

/// A power's arms, both of them.
const ARM_POWER: f32 = 0.3;
/// A weapon switch's arms, both of them.
const ARM_SWITCH: f32 = 0.4;

/// One arm's stiffness after one tick, `x` last tick's: 0.2 collapsing;
/// 0.5 off balance; 0.2 dazed (tone under 0.85); striding, the
/// character's walking arms (0.2, looser for the feral); a kick leaves
/// `x`; a power sets 0.3 and a weapon switch 0.4, each floored by the
/// attack setter's compare; else easing to 0.4·(0.5 + 0.5·race arm
/// relaxation). While the body turns, except on a kick, it is at least
/// min(0.4·|turn left|, 0.4). The muscle tune eases the two stored
/// stiffnesses with [`attack_arms`] on that last branch; this eases one
/// value, which the other branches still share.
pub(super) fn arm_stiffness(
    x: f32,
    bearing: &Bearing,
    tone: f32,
    race: &RaceMuscles,
    stats: &BodyStats,
) -> f32 {
    if bearing.collapsing {
        return floor_arm(ARM_LOOSE, bearing.turn);
    }
    if bearing.off_balance {
        return floor_arm(ARM_OFF_BALANCE, bearing.turn);
    }
    if tone < ARM_DAZED_TONE {
        return floor_arm(ARM_LOOSE, bearing.turn);
    }
    if bearing.striding {
        return floor_arm(stats.walking_arms, bearing.turn);
    }
    // A kick does not write the arms, so the turn does not floor them.
    if bearing.kicking {
        return x;
    }
    if bearing.casting {
        return attack_stiffness_floor(ARM_POWER, bearing.turn);
    }
    if bearing.switching {
        return attack_stiffness_floor(ARM_SWITCH, bearing.turn);
    }
    floor_arm(
        ARM_KEEP * x + ARM_EASE * (0.5 + 0.5 * race.arm_relaxation),
        bearing.turn,
    )
}

/// The idle floor: at least 0.4·|turn|, at most 0.4, in single precision.
fn floor_arm(x: f32, turn: f32) -> f32 {
    x.max((ARM_TURN * turn.abs()).min(ARM_TURN))
}

/// An arm part's anchor relaxation cap at stiffness `x`.
pub(super) fn arm_relax_cap(x: f32) -> f32 {
    ARM_CAP_TOP - (x - ARM_CAP_FROM) * ARM_CAP_SLOPE
}

/// One attack tick's two arms, right then left, as the swing hands them to
/// the muscle tune.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackArmTune {
    /// Each arm's stiffness before the turn floor.
    pub stiffness: (f32, f32),
    /// Each arm's drive, which sets that side's saturation.
    pub drive: (f32, f32),
}

/// The attack setter's floor of a stiffness: 0.4·|turn| stored as a float
/// and capped at 0.4. It replaces the stiffness only when the floor
/// compares above it, so a NaN on either side leaves the stiffness.
pub(super) fn attack_stiffness_floor(stiffness: f32, turn: f32) -> f32 {
    let mut floor = (f64::from(turn.abs()) * 0.4) as f32;
    if f64::from(floor) > 0.4 {
        floor = 0.4;
    }
    if floor.partial_cmp(&stiffness) == Some(std::cmp::Ordering::Greater) {
        floor
    } else {
        stiffness
    }
}

/// An attacking arm's anchor relaxation cap: 0.6 − (stiffness − 0.1)·0.8,
/// the stiffness widened and the rest in double.
pub(super) fn attack_relax_cap(stiffness: f32) -> f32 {
    (0.6 - (f64::from(stiffness) - 0.1) * 0.8) as f32
}

/// An attacking arm's saturation: 0.1 − 0.08·drive, the drive widened.
pub(super) fn attack_saturation(drive: f32) -> f32 {
    (0.1 - f64::from(drive) * 0.08) as f32
}

/// Which attack arm a bone takes: 0 the right, 1 the left. The right side
/// is the negative-x arm (shoulder, upper arm, forearm, hand).
fn attack_side(bone: usize) -> Option<usize> {
    match bone {
        11 | 12 | 13 | 17 => Some(0),
        14 | 15 | 16 | 18 => Some(1),
        _ => None,
    }
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
    /// link muscles. The root is taken as the energy is, so a NaN or a
    /// negative stays a NaN. D = (E − 0.5)² unless that difference compares
    /// below 0, when it is 0 (a NaN is squared and stays). k = 50 while
    /// moving, else 40 over the race's stance factor.
    pub(super) fn muscle_gains(&self) -> (f32, f32) {
        let strength = self.energy.sqrt() * self.tone_target();
        let below = self.energy - DAMPED_FROM;
        let d = if below < 0.0 { 0.0 } else { below * below };
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
            legs,
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
        // Standing, at full tone, not in a walk or a run: a kick leaves
        // the two stored stiffnesses, and a power or a weapon switch
        // writes both the same through the attack setter. Otherwise each
        // stored stiffness eases on its own. A collapse, a daze and a
        // stride write both the same, and a shielded stride doubles the
        // off arm. An attack stores its floored pair
        // so the next standing tick starts there, unless a kick is holding.
        let posed = !unsteady.collapsing
            && !unsteady.off_balance
            && drive.tone >= ARM_DAZED_TONE
            && !unsteady.striding;
        let hold_arms = posed && unsteady.kicking;
        let snap = posed && (unsteady.casting || unsteady.switching);
        let easing = posed && !unsteady.kicking && !unsteady.casting && !unsteady.switching;
        // A parry eases each stored arm by 0.8x + 0.04. It is ahead of the
        // walk, the daze, a power and a weapon switch, and it does not run
        // while the body collapses, is off balance, kicks, or an attack
        // sets the arms.
        let parry_ease = bearing.parrying
            && !unsteady.collapsing
            && !unsteady.off_balance
            && !unsteady.kicking
            && drive.attack_arms.is_none();
        // A shielded stride doubles the off arm. It is the walk's own
        // branch: a parry, a daze, a kick, a collapse and lost balance
        // are already taken.
        let stride_shield = unsteady.striding
            && unsteady.shield
            && !unsteady.collapsing
            && !unsteady.off_balance
            && !unsteady.kicking
            && drive.tone >= ARM_DAZED_TONE
            && drive.attack_arms.is_none()
            && !parry_ease;
        // An attack's drives stay on the body: a later power, weapon
        // switch or standing tick saturates each arm from the one it stored.
        if let Some(tune) = drive.attack_arms {
            drive.arm_drive = tune.drive;
        }
        if hold_arms {
            drive.arm = drive.arms.0;
        } else if let Some(tune) = drive.attack_arms {
            let right = attack_stiffness_floor(tune.stiffness.0, unsteady.turn);
            let left = attack_stiffness_floor(tune.stiffness.1, unsteady.turn);
            drive.arms = (right, left);
            drive.arm = right;
        } else if parry_ease {
            let right =
                attack_stiffness_floor(crate::motile::parry_blend(drive.arms.0), unsteady.turn);
            let left =
                attack_stiffness_floor(crate::motile::parry_blend(drive.arms.1), unsteady.turn);
            drive.arms = (right, left);
            drive.arm = right;
        } else if easing {
            let eased = attack_arms(
                drive.arms,
                false,
                false,
                drive.race.arm_relaxation,
                unsteady.turn,
            );
            drive.arms = eased;
            drive.arm = eased.0;
        } else if stride_shield {
            let right = floor_arm(drive.stats.walking_arms, unsteady.turn);
            let left = floor_arm(drive.stats.walking_arms * 2.0, unsteady.turn);
            drive.arms = (right, left);
            drive.arm = right;
        } else {
            let x = arm_stiffness(drive.arm, &unsteady, drive.tone, &drive.race, &drive.stats);
            drive.arms = (x, x);
            drive.arm = x;
        }
        let limbs = LIMB_SCALE * drive.stats.scale;
        let (thigh, limit) = leg_gains(bearing.legs, limbs, drive.support, &bearing);
        for i in 0..parts.len() {
            match role(parts[i].bone) {
                // A collapsing body's legs keep their gains.
                Role::Shin if parts[i].foot.is_some() && !bearing.collapsing => {
                    if let Some(k) = parts[i].leg_limit.filter(|&k| k < links.len()) {
                        let s = &mut links[k].strength;
                        // A stepping foot that is anchored eases to 0.2
                        // whatever its leg mode.
                        let anchored = bearing.stepping
                            && legs.iter().any(|l| {
                                l.shin == i && l.state.has(super::feet::FootState::ANCHORED)
                            });
                        *s = if anchored {
                            (f64::from(*s) * STEP_LIMIT_KEEP + STEP_LIMIT_TAKE) as f32
                        } else {
                            (f64::from(limit) * LEG_LIMIT_TAKE + f64::from(*s) * LEG_LIMIT_KEEP)
                                as f32
                        };
                    }
                    if let Some(j) = parts[i].parent.filter(|&j| j < parts.len()) {
                        parts[j].factor = thigh;
                    }
                }
                Role::Arm => {
                    // A kick leaves whatever the last tick wrote.
                    if hold_arms {
                        continue;
                    }
                    let bone = parts[i].bone;
                    let p = &mut parts[i];
                    // An attack tick's arms replace the stored stiffness on
                    // every bone of that side. A standing tick eases those
                    // stored values apart; the swing's pair is what this
                    // tick's muscles pull with. A power or a weapon switch
                    // writes the one stiffness on both sides, and each side
                    // keeps the saturation of the drive its last attack stored.
                    if let Some(tune) = drive.attack_arms
                        && let Some(side) = attack_side(bone)
                    {
                        let (stiff, drv) = if side == 0 {
                            (tune.stiffness.0, tune.drive.0)
                        } else {
                            (tune.stiffness.1, tune.drive.1)
                        };
                        let stiff = attack_stiffness_floor(stiff, drive.bearing.turn);
                        p.factor = stiff * limbs;
                        p.relax_cap = attack_relax_cap(stiff);
                        p.saturation = attack_saturation(drv);
                    } else if snap
                        && !parry_ease
                        && let Some(side) = attack_side(bone)
                    {
                        let drv = if side == 0 {
                            drive.arm_drive.0
                        } else {
                            drive.arm_drive.1
                        };
                        p.factor = drive.arm * limbs;
                        p.relax_cap = attack_relax_cap(drive.arm);
                        p.saturation = attack_saturation(drv);
                    } else if (easing || parry_ease)
                        && let Some(side) = attack_side(bone)
                    {
                        // Each stored stiffness, eased this tick, with the
                        // attack setter's cap and the drive the last attack
                        // stored. Until an attack writes one, that drive is
                        // 0 and the saturation is the idle 0.1.
                        let (stiff, drv) = if side == 0 {
                            (drive.arms.0, drive.arm_drive.0)
                        } else {
                            (drive.arms.1, drive.arm_drive.1)
                        };
                        p.factor = stiff * limbs;
                        p.relax_cap = attack_relax_cap(stiff);
                        p.saturation = attack_saturation(drv);
                    } else {
                        let stiff = if stride_shield && let Some(side) = attack_side(bone) {
                            if side == 0 {
                                drive.arms.0
                            } else {
                                drive.arms.1
                            }
                        } else {
                            drive.arm
                        };
                        p.factor = stiff * limbs;
                        p.relax_cap = arm_relax_cap(stiff);
                        p.saturation = SATURATION;
                    }
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
            bearing.head_looks,
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
/// turn, at the part's origin; a looking head's relative to its parent's
/// physical transform), as weights on the parent. Each part's λ then
/// moves toward 0.6 − 32|Δa|² (the most one of its anchors moved since the
/// last tick, cm²) times the race's factor, capped by the part's cap.
fn sample_anchors(
    parts: &mut [Part],
    links: &mut [Link],
    links_of: &[std::ops::Range<usize>],
    targets: &[Affine3A],
    relaxation: f32,
    head_looks: bool,
) {
    if targets.len() != parts.len() {
        return;
    }
    for i in 0..parts.len() {
        let Some(j) = parts[i].parent.filter(|&j| j < parts.len() && j != i) else {
            continue;
        };
        let parent = if head_looks && role(parts[i].bone) == Role::Head {
            parts[j].transform()
        } else {
            targets[j]
        };
        let relative = parent.inverse() * targets[i];
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

/// The two arms' stiffness targets through an attack tick, with the
/// floor the arms are set with: in a tangle both are 0.5; stepping they
/// ease by 0.8·x + 0.08; otherwise by 0.8·x + (0.5·relaxation + 0.5)·0.08
/// (the sum in single precision), the rest in double precision. The floor
/// is 0.4·|turn| capped at 0.4; a target is raised to it only when the
/// floor is above it (a NaN on either side leaves the target).
pub fn attack_arms(
    next: (f32, f32),
    stepping: bool,
    tangle: bool,
    relaxation: f32,
    turn: f32,
) -> (f32, f32) {
    const EASE: f64 = 0.8;
    // The original's constant, one unit in the last place above 0.08.
    const STEP: f64 = 0.080_000_000_000_000_02;
    let (right, left) = if tangle {
        (0.5, 0.5)
    } else if stepping {
        (
            (f64::from(next.0) * EASE + STEP) as f32,
            (f64::from(next.1) * EASE + STEP) as f32,
        )
    } else {
        let add = (f64::from(relaxation * 0.5 + 0.5) * STEP) as f32;
        (
            (f64::from(add) + f64::from(next.0) * EASE) as f32,
            (f64::from(add) + f64::from(next.1) * EASE) as f32,
        )
    };
    (
        attack_stiffness_floor(right, turn),
        attack_stiffness_floor(left, turn),
    )
}

#[cfg(test)]
mod attack_arm_tests {
    use super::*;

    #[test]
    fn the_attack_arm_targets_are_the_originals() {
        assert_eq!(attack_arms((0.5, 0.5), false, true, 0.0, 0.0), (0.5, 0.5));
        let (r, l) = attack_arms((0.5, 0.5), true, false, 0.0, 0.0);
        assert!((r - 0.48).abs() < 1e-6 && (l - 0.48).abs() < 1e-6);
        let (r, _) = attack_arms((0.5, 0.5), false, false, 0.0, 0.0);
        assert!((r - 0.44).abs() < 1e-6, "{r}");
        let (r, l) = attack_arms((0.0, 0.0), false, false, 0.0, 1.0);
        assert_eq!((r, l), (0.4, 0.4));
    }
}
