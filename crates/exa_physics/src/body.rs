//! The physical body: each body part is a tetrahedron of four particles kept
//! rigid by its edges, carrying sphere colliders, and joined to its parent
//! by a pivot at its origin (its joint centre) and by the skeleton's links
//! (a point on each part, allowed to drift apart by a slack: the joint's
//! range). Particles move by Verlet integration at the original's 83 Hz,
//! then constraints and contacts are solved by moving positions.
//!
//! A limp body (the dead, the knocked out) is only that: its joints are
//! damped inside their range, and each part keeps its velocity by how fast
//! it moves. A living body adds muscles ([`Drive`]): every slack link
//! becomes a link muscle that turns its joint toward the animated angle
//! (`muscle`), a pose drive turns the pelvis, spine, head and feet toward
//! their animated orientation about where each part is (it has no position
//! target), and a carry holds it up on its feet: its limb particles fall by
//! gravity less a lift that holds its pelvis at a target height over the
//! ground under its feet (`lift`). Its legs bend onto that ground on the
//! next tick (`bend`).
//! Its character tick measures its balance and the ground under its feet,
//! which give its support (`balance`), and puts its animated pose on its
//! character frame, which follows its pelvis by the original's rules
//! (`frame`). Blows shove parts and daze the body, which weakens its
//! muscles until it recovers. A prop's or an item's rigid body is a body of
//! the same kind, built from its model's stored rest points, spheres and
//! links and registered asleep (`rigid`).
//!
//! Bodies tick in the fixed step, as one world in the original's order:
//! every living body's character tick (its clip sampled and its targets
//! set, the player's last, [`CharacterTicks`]), then the solver over every
//! awake body ([`BodySystems`]): blows, integration, 20 iterations, level
//! detection, 8 friction passes, post-step. Parts are drawn between their
//! last two ticks. A limp body sleeps by the original's countdown: 256
//! ticks, two off each tick its parts move under 20 cm/s, held at 30–32
//! between 20 and 40 cm/s and back to 256 above; a blow wakes it, and a
//! living body never sleeps. Evidence: `docs/formats/RSQ.md` (Physical
//! body).

use std::sync::Arc;

use bevy::math::{Affine3A, Mat3};
use bevy::prelude::*;

use crate::Physics;
use crate::contact::{CONTACT_SLOP, Level, LevelContact, evicted, rub, sphere_hit, swept_hit};
use crate::part::PartState;

#[path = "body_sound.rs"]
mod sound;
pub use sound::{ImpactEvent, ImpactState, ScrapeEvent, ScrapeSource, impact_sound};
#[path = "body_muscle.rs"]
mod muscle;
pub use muscle::{AttackArmTune, Bearing, BodyStats, RaceMuscles, attack_arms};

#[path = "body_balance.rs"]
mod balance;
pub use balance::{Balance, Leg, reset_heading};
#[path = "body_bend.rs"]
mod bend;
#[path = "body_limp.rs"]
mod limp_sampler;
pub use bend::{FemurPosture, femur_posture_turn, foot_pitch_target};
pub use limp_sampler::{limp_plant, limp_pose_height, snap_part};
#[path = "body_hunch.rs"]
mod hunch;
pub use hunch::{ease_hunch, hunch_applies, hunch_turn, step_hunch};
#[path = "body_feet.rs"]
mod feet;
#[path = "body_stagger.rs"]
pub mod stagger;
pub use feet::{
    FootState, Footstep, FullTone, GaitStep, QueuedStep, StepEnd, WalkedStep, full_tone, gait_step,
    queued_step, step_end, step_loudness, step_position,
};
#[path = "body_pins.rs"]
mod pins;
#[path = "body_stages.rs"]
pub mod stages;
#[path = "body_steps.rs"]
mod steps;
#[path = "body_timers.rs"]
pub mod timers;
pub use timers::BodyTimers;

pub use stages::{
    BodyPostStep, BodyPreStep, CharacterStage, CharacterTick, SolverStage, SolverStages,
    TickedCharacter, run_character_ticks, solve_among, solve_with,
};
pub use steps::step_length;
#[path = "body_frame.rs"]
mod frame;
#[path = "body_inspect.rs"]
pub mod inspect;
#[path = "body_lift.rs"]
mod lift;
#[path = "body_locomotion.rs"]
pub mod locomotion;
#[path = "body_pairs.rs"]
pub mod pairs;
pub use pairs::{Pairing, Pairs, Touch};
#[path = "body_drag.rs"]
mod drag;
#[path = "body_rigid.rs"]
mod rigid;
pub use drag::{Grab, drag_pull};
pub use rigid::{FALL_OUT_HEIGHT, RigidLink, RigidPart};

/// Simulation rate of the body.
pub const BODY_HZ: f32 = 83.0;
/// The solver's gravity (m/s²): the original's environment value.
pub const GRAVITY: f32 = -9.806_65;
/// Velocity a limp part keeps per second, by its speed: fast, slow and
/// resting (the original's default environment, which every body uses).
pub const RETENTION: [f32; 3] = [0.426, 0.25, 0.05];
/// Speeds (cm/s) a limp part's last tick must exceed to be fast, or slow.
pub const RETENTION_SPEEDS: [f32; 2] = [40.0, 20.0];
/// u = (1 − b)(1 − r) + r, from a race's limb damping ratio r and the
/// balance b. A living limb keeps the fast retention times [`retention_factor`]:
/// u² standing, and 0.5·u + 0.5 while walking or off its feet. A human's r
/// of 1 makes both 1 at any balance.
pub(crate) const LINK_ITERATIONS: usize = 20;
pub(crate) const FRICTION_PASSES: usize = 8;
/// A held item body's motion flag: a walk's start clears it.
pub const HELD_MOTION_FLAG: u32 = 0x10000;
/// A held item body's static-only flag: every level query uses the current
/// pose, never a sweep.
pub const HELD_STATIC_ONLY: u32 = 0x80000;
/// The grab layer bit the bite-grab raises on a part and its neighbours.
pub const GRAB_LAYER: u32 = 0x40;
/// The sleep countdown a woken body starts from, and a living body's every
/// tick: at rest it falls asleep 128 ticks (1.54 s) later.
pub const WAKE_COUNT: i32 = 256;
/// While its fastest part moves between 20 and 40 cm/s, a body's countdown
/// is lifted to this from below and falls by one a tick from above, so it
/// is held at 30–32 and never runs out.
const SLOW_COUNT: i32 = 32;
/// The post-step puts a body to sleep once its countdown is down to this.
const SLEEP_COUNT: i32 = 1;
/// Relative inverse mass per part (bone − 1): pelvis, legs, torso, head,
/// arms, hands.
const INVERSE_MASS: [f32; 18] = [
    0.45, 0.7, 1.0, 2.8, 0.7, 1.0, 2.8, 0.45, 0.5, 1.0, 1.0, 1.2, 1.5, 1.0, 1.2, 1.5, 2.5, 2.5,
];
/// A link end's mobility is this times its part's [`INVERSE_MASS`]; only
/// the ratio between the two parts matters, so it cancels between limbs.
const LINK_MOBILITY: f32 = 0.175;
/// Centimetres per metre: the links' softening and smoothing constants are
/// the original's, in cm.
const CM: f32 = 100.0;
/// A link with at most this slack (m, 0.01 cm) is a pivot in a living
/// body; in a limp one, a link with at most 1.19e-7 cm.
const PIVOT_SLACK: f32 = 1e-4;
const LIMP_PIVOT_SLACK: f32 = 1.19e-9;
/// A limp slack link whose length is this close (m, 1.19e-7 cm) to its
/// smoothed length is left alone.
const LENGTH_EPSILON: f32 = 1.19e-9;
/// Softening of a pivot per cm of its gap (a living body's) or per cm² (a
/// limp body's).
const PIVOT_SOFTENING: f32 = 0.001;
/// The original's general epsilon (1.19209e-7): squared gaps under it are
/// ignored, and friction limits are compared against squared slips.
const GENERAL_EPSILON: f32 = 1.192_09e-7;
/// Link flags a rigid body's links can carry. A link made by code (the
/// origin pivot, a grab) carries [`LINK_CODE`]; a disabled link is skipped.
pub const LINK_BODY: u32 = 0x1_0000;
pub const LINK_SOFT_WELD: u32 = 0x8;
pub const LINK_BREAKABLE: u32 = 0x1000;
pub const LINK_ITERATION_LIMIT: u32 = 0x4000;
pub const LINK_DISABLED: u32 = 0x10_0000;
pub const LINK_REANCHOR: u32 = 0x20_0000;
pub const LINK_CODE: u32 = 0x40_0000;
/// A runtime link's pivot flag: a link without it is a limit link
/// (compared by its squared slack, moving its part alone). The skeleton's
/// registration sets it on every link of a part with a parent (keeping only
/// the soft-weld bit of the stored word), so a character's links are never
/// limit links; the shipped skeletons store the word as 0.
pub const MODEL_LINK_PIVOT: u32 = 0x1_0000;
/// A limit link's horizontal-only flag: the link ignores the vertical gap
/// and runs in the tick's first two iterations.
pub const MODEL_LINK_HORIZONTAL: u32 = 0x200;
/// Squared gaps (m², 1.19e-7 cm²) a link, or the pose drive, leaves alone.
const LINK_EPSILON2: f32 = 1.19e-11;
/// A living body's slack-link correction is cut to this (m, 80 cm); a
/// pivot farther apart is reported but corrected in full.
const MAX_CORRECTION: f32 = 0.8;
/// A lower leg's link with more slack than this (m, 10 cm) limits the leg;
/// it pulls at this strength while the body lives.
const LEG_LIMIT_SLACK: f32 = 0.1;
const LEG_LIMIT_STRENGTH: f32 = 0.2;
/// The original's drive scale K = 19200/83² (2.787): the pose drive's
/// gains are shares of it.
const DRIVE_SCALE: f32 = 19200.0 / (BODY_HZ * BODY_HZ);
/// Pose-drive gains, as shares of [`DRIVE_SCALE`]: the pelvis, lumbar,
/// thorax and feet (0.0836), and the head (0.0279; a feral character's by
/// its head gain, [`BodyStats::head_gain`]).
const TRUNK_GAIN: f32 = 0.03;
const HEAD_GAIN: f32 = 0.01;
/// The standing pose push's vertical scale: the double 0.1, narrowed once.
const VERTICAL_PUSH: f64 = 0.1;
/// The least energy a blow leaves ([`daze_energy`]). Recovery adds the
/// double 0.005 each tick, up to the body's most ([`recover_energy`]).
const MIN_ENERGY: f32 = 0.05;
/// The double that floor is compared with. The float 0.05 promotes to a
/// hair above it, so a blow that lands on the float is not raised.
const MIN_ENERGY_F64: f64 = 0.05;
/// The most energy of a body that is not feral (a feral body's is less,
/// [`BodyStats::max_energy`]).
pub const MAX_ENERGY: f32 = 1.0;
/// Most of its energy a single blow takes.
pub const MAX_DAZE: f32 = 0.9;
/// The double a daze is compared with before it is capped at [`MAX_DAZE`].
/// The float 0.9 promotes to a hair under it, so that float is not capped.
const MAX_DAZE_F64: f64 = 0.9;
/// The support pins' gain factor a living body starts with, and below
/// which it recovers 0.001 a tick unless it collapses. The start is the
/// float 0.08, compared with the double 0.08, so a living body's first
/// tick lifts it to 0.081, where it stays.
pub const PIN_GAIN: f32 = 0.08;
const PIN_GAIN_BELOW: f64 = 0.08;
const PIN_RECOVERY: f64 = 0.001;
/// A body is dazed while its energy is more than this under its most,
/// compared in double precision as the original does (so an energy of
/// 0.95f is dazed).
const DAZED_BELOW_MAX: f64 = 0.05;
/// Friction of a limb against the level.
const LIMB_FRICTION: f32 = 2.5;
/// Friction of a foot (bones 4 and 7) as a body is built; a living body's
/// lower legs and feet take theirs from their foot's state every tick
/// (`feet`).
const FOOT_FRICTION: f32 = 0.5;
const MIN_EXTENT: f32 = 0.06;
/// Level contacts a part keeps between ticks.
const MAX_CONTACTS: usize = 4;
/// Squared travel (m²) of a sphere centre over a tick above which the sphere
/// is swept from where the last tick left it.
const SWEEP_TRAVEL2: f32 = 1e-5;
/// Pushes shorter than this (squared, m²) are ignored.
const MIN_PUSH2: f32 = 1e-9;
/// Depth (m) under a stored contact plane that is corrected.
const MIN_DEPTH: f32 = 1e-7;
/// Push length (m) at detection, and depth (m) in the plane pass, summed
/// before a part's edges are restored.
const PUSH_STRAIN: f32 = 0.001;
/// A move (a translate) adds up to this (0.1 cm) before the edges are
/// restored.
const TRANSLATE_STRAIN: f32 = 0.001;
const DEPTH_STRAIN: f32 = 0.01;
/// A level push longer than this (m) is reported, not clamped.
const EXCESSIVE_PUSH: f32 = 1.6;

/// Regular tetrahedron, centred on the origin.
const TEMPLATE: [Vec3; 4] = [
    Vec3::new(std::f32::consts::SQRT_2, -1.0, 0.0),
    Vec3::new(-std::f32::consts::SQRT_2, -1.0, 0.0),
    Vec3::new(0.0, 1.0, std::f32::consts::SQRT_2),
    Vec3::new(0.0, 1.0, -std::f32::consts::SQRT_2),
];
/// The template, laid out over the part's box, is enlarged by this.
const TEMPLATE_SCALE: f32 = 1.25;

/// A rigid part of a simulated body.
#[derive(Debug, Clone)]
pub struct Part {
    pub bone: usize,
    pub entity: Entity,
    /// Entity whose world transform the part's local transform is relative
    /// to: its parent part's, or the body root's.
    pub parent: Option<usize>,
    /// Rest positions of the particles in the part's space.
    pub rest: [Vec3; 4],
    pub positions: [Vec3; 4],
    pub previous: [Vec3; 4],
    /// A limb's relative inverse mass; a rigid body's mobility (larger
    /// moves more).
    pub inverse_mass: f32,
    /// Each particle's share of gravity: 1 on a limb; on a rigid body,
    /// shares summing to 4 that put gravity at its centre of mass.
    pub gravity: [f32; 4],
    /// Collision spheres in the part's space: centre, radius.
    pub spheres: Vec<(Vec3, f32)>,
    /// The part's collision mesh in its space, when its node's model gives
    /// one: the triangles the other side's spheres are tested against (the
    /// original's mesh query). Empty for a part whose mesh was never handed
    /// over; the pair stage then falls back to the part's spheres.
    pub triangles: Option<Arc<Vec<[Vec3; 3]>>>,
    /// How it meets other parts, of other bodies and of its own.
    pub pairing: Pairing,
    rest_inverse: Mat3,
    /// Friction against the level.
    pub friction: f32,
    /// Whether the part skips level detection: a root body with world links
    /// (its static mask is 1), as the original's hung props and mechanisms.
    pub no_level: bool,
    /// Particles at the start of the tick (where the last tick left them).
    start: [Vec3; 4],
    /// Each particle's displacement over the last tick, as this tick's
    /// integration found it (before its inertia): the velocity the pose
    /// drive damps.
    moved: [Vec3; 4],
    /// The original's smoothed velocity: a copy of [`moved`] lerped a tenth
    /// of the way each tick, which the pair and level impacts read.
    smoothed: [Vec3; 4],
    /// The weights of the part's own origin (its joint, the point (0, 0, 0)
    /// of its space) on its particles: where the pose drive turns it about.
    own_origin: [f32; 4],
    /// The part's transform where the last tick left it.
    settled: Affine3A,
    /// Where the tick before last and the last tick left the part, untouched
    /// by the carry: it is drawn between the two.
    shown: (Affine3A, Affine3A),
    /// The particles where the tick before last and the last tick left them:
    /// drawing interpolates these, then builds the matrix, as the original
    /// does.
    drawn_particles: ([Vec3; 4], [Vec3; 4]),
    /// Level contacts found last tick, enforced in this tick's iterations.
    pub contacts: Vec<LevelContact>,
    /// Pushes and depths summed since the edges were last restored (or
    /// the tick began).
    strain: f32,
    /// The link (an index into [`Ragdoll::links`]) that limits the leg: a
    /// lower leg's and its femur's first with over 10 cm of slack.
    pub leg_limit: Option<usize>,
    /// Where the part's origin sits in its parent's space (the model's
    /// translation of the part): its animated joint is placed there.
    origin: Vec3,
    /// A lower leg's foot contact point, in its space.
    foot: Option<Vec3>,
    /// The part's link muscle factor: the share of the body's strength
    /// its link muscles pull with.
    pub factor: f32,
    /// Saturation c of its link muscles' error, per cm: e/(1 + c|e|).
    saturation: f32,
    /// Anchor relaxation λ: how far its link muscles' anchors are drawn
    /// toward where its points already are, and its cap.
    pub relax: f32,
    relax_cap: f32,
    /// The part's turn relative to its parent in the model's rest pose,
    /// which the debug tools measure its joint from
    /// ([`Ragdoll::with_rest_turns`]).
    rest_turn: Quat,
    /// The part's state word: whether it strikes, is a held item, an arm or
    /// a muscled limb ([`PartState`]).
    pub state: PartState,
    /// Its level queries are static (the current pose only, no sweep), as
    /// a held item body's are in the original's static-only mode.
    pub static_only: bool,
    /// The original's class word, which carries the body's
    /// collision class bits (the grab layer, the strike window and the
    /// static-only mode among them).
    pub class_word: u32,
    /// The original's controller state word (on the node's
    /// controller), which the body tick sets bits of (bit 4 marks a part
    /// in a tangle or a hit reaction).
    pub state_word: u32,
    /// The original's second flag word on the body (its node's word beside
    /// the class word): a lower leg's children take [`crate::part::FOOT_CHILD_FLAG`]
    /// when the leg joins the world.
    pub flags2: u32,
    /// When this part may play its impact sound again, on the 32-bit game
    /// clock. A blow on it stamps [`crate::part::part_sound_ready`].
    pub sound_ready_ms: u32,
    /// The strength times two of the last collision sound the part played;
    /// while its clock still holds, a weaker impact stays silent.
    pub last_impact: f32,
    /// Whether the part takes the level's impact call (the original's class
    /// bit 0x40000, set at add-to-world on every part but a foot and its
    /// children; a rigid body has it when its material has a collision
    /// sound, which Exarust takes as always).
    pub reacts: bool,
    /// This tick's accepted level hits of the part: their count, summed
    /// world points and pushes, and the fastest smoothed velocity with its
    /// square, from which the level impact is made.
    level_hits: u32,
    level_point: Vec3,
    level_push: Vec3,
    level_travel: Vec3,
    level_travel2: f32,
    /// The original's extra particle array a placed body's pair solves
    /// record: each push it applies is recorded back with the opposite
    /// sign, so the placed body's node controller can read how far a
    /// contact wanted to move it. The two pushes cancel, so nothing moves
    /// by this array; [`crate::body_pairs`] keeps it for the node owner.
    pub(crate) placed_offset: [Vec3; 4],
}

/// A level impact: a part hit the static level hard enough for the
/// original's impact call (a fall or a scrape), with the mean world contact
/// point and the strength `|v|²·(−cos(v, Σδ))`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelImpact {
    pub part: usize,
    /// The mean contact point (world, m).
    pub point: Vec3,
    /// The impact strength (cm² a tick²).
    pub strength: f32,
}

/// A level contact's fastest smoothed velocity squared must pass this to
/// make an impact (the original's 0.1 cm² a tick²).
pub const LEVEL_STRIKE_TRAVEL2: f32 = 0.1;
/// The level impact's strength must pass this to be called (0.1).
pub const LEVEL_STRIKE_IMPACT: f32 = 0.1;

/// A level impact record is made for a part whose reaction class is set
/// (the original's class bit 0x40000, cleared on a foot and its children)
/// and whose fastest smoothed velocity squared `travel2` (cm² a tick²)
/// passes 0.1: the strength is that square times the negated cosine
/// against the summed pushes, and must pass 0.1.
fn level_impact(
    part: usize,
    point: Vec3,
    push: Vec3,
    travel: Vec3,
    travel2: f32,
) -> Option<LevelImpact> {
    if travel2.is_nan() || travel2 <= LEVEL_STRIKE_TRAVEL2 {
        return None;
    }
    let strength = travel2 * -crate::engine_math::cosine(travel, balance::cm(push));
    if strength > LEVEL_STRIKE_IMPACT {
        Some(LevelImpact {
            part,
            point,
            strength,
        })
    } else {
        None
    }
}

/// The share of a push at a point of a body that acts, as the original's
/// push-at-point: `moved` is how far the point travelled this tick (the weighted particle displacement) and `4·(push·moved)` is compared with the push's squared
/// length. A push whose squared length is below the epsilon, or one moving
/// the point at least as far along it (or a NaN), gets 0; a negative or tiny
/// value gets the full push (1); between them the push is scaled by 1 minus
/// that share. Both dots sum z + (y + x).
pub fn push_fraction(push: Vec3, moved: Vec3) -> f32 {
    let len2 = push.z * push.z + (push.y * push.y + push.x * push.x);
    // A NaN length falls through, as the original's compare does; every
    // test below then fails and nothing moves.
    if len2 < PUSH_EPSILON {
        return 0.0;
    }
    let dot = push.z * moved.z + (push.y * moved.y + push.x * moved.x);
    let scaled = dot * PUSH_WEIGHT;
    // Only a strictly small value (not a NaN) takes the full push.
    if scaled < PUSH_EPSILON {
        1.0
    } else if len2 > scaled {
        1.0 - scaled / len2
    } else {
        0.0
    }
}

/// The push weighting's quarter units and the single-precision epsilon.
const PUSH_WEIGHT: f32 = 4.0;
const PUSH_EPSILON: f32 = f32::from_bits(0x33ff_ffd7);

/// Barycentric weights of a point given in a part's space.
fn weights(part: &Part, local: Vec3) -> [f32; 4] {
    let w = part.rest_inverse * (local - part.rest[0]);
    [1.0 - w.x - w.y - w.z, w.x, w.y, w.z]
}

impl Part {
    /// Where the part's origin sits in its parent's space (the model's
    /// translation of the part), which the clip's chain uses.
    pub fn rest_offset(&self) -> Vec3 {
        self.origin
    }

    fn new(s: &PartSetup) -> Self {
        let PartSetup {
            bone,
            entity,
            parent,
            centre,
            half,
            world,
            ..
        } = *s;
        let stored = s.spheres.clone();
        let largest = half.max_element();
        let half = half.max(Vec3::splat(MIN_EXTENT.max(0.2 * largest)));
        let rest = TEMPLATE.map(|t| centre + t * half * TEMPLATE_SCALE);
        let edges = Mat3::from_cols(rest[1] - rest[0], rest[2] - rest[0], rest[3] - rest[0]);
        let rest_inverse = edges.inverse();
        let own = rest_inverse * -rest[0];
        let positions = rest.map(|r| world.transform_point3(r));
        // Spheres along the longest axis of the box.
        let axis = if half.x >= half.y && half.x >= half.z {
            Vec3::X
        } else if half.y >= half.z {
            Vec3::Y
        } else {
            Vec3::Z
        };
        let long = half.dot(axis);
        let thick = (half - axis * long).max_element().max(MIN_EXTENT * 0.5);
        let count = ((long / thick).round() as usize).clamp(1, 4);
        let span = (long - thick).max(0.0);
        let spheres = match stored {
            Some(stored) if !stored.is_empty() => stored,
            _ => (0..count)
                .map(|k| {
                    let t = if count == 1 {
                        0.0
                    } else {
                        k as f32 / (count - 1) as f32 * 2.0 - 1.0
                    };
                    (centre + axis * span * t, thick)
                })
                .collect::<Vec<_>>(),
        };
        let inverse_mass = INVERSE_MASS
            .get(bone.wrapping_sub(1))
            .copied()
            .unwrap_or(1.0);
        let pairing = Pairing::new(
            &rest,
            &rest_inverse,
            &spheres,
            inverse_mass * pairs::LIMB_CONTACT_SCALE,
        )
        .with_mesh_box(centre, s.half);
        Self {
            bone,
            entity,
            parent,
            rest,
            positions,
            previous: positions,
            inverse_mass,
            gravity: [1.0; 4],
            spheres,
            triangles: None,
            pairing,
            rest_inverse,
            no_level: false,
            friction: if matches!(bone, 4 | 7) {
                FOOT_FRICTION
            } else {
                LIMB_FRICTION
            },
            start: positions,
            moved: [Vec3::ZERO; 4],
            smoothed: [Vec3::ZERO; 4],
            own_origin: [1.0 - own.x - own.y - own.z, own.x, own.y, own.z],
            settled: Affine3A::IDENTITY,
            shown: (Affine3A::IDENTITY, Affine3A::IDENTITY),
            drawn_particles: ([Vec3::ZERO; 4], [Vec3::ZERO; 4]),
            contacts: Vec::new(),
            strain: 0.0,
            leg_limit: None,
            origin: s.origin,
            foot: s.foot,
            factor: muscle::PART_FACTOR,
            saturation: muscle::SATURATION,
            relax: 0.0,
            relax_cap: muscle::RELAX_CAP,
            rest_turn: Quat::IDENTITY,
            state: PartState::LIMB_FRESH,
            static_only: false,
            class_word: 0,
            state_word: 0,
            flags2: 0,
            sound_ready_ms: 0,
            last_impact: 0.0,
            reacts: true,
            level_hits: 0,
            level_point: Vec3::ZERO,
            level_push: Vec3::ZERO,
            level_travel: Vec3::ZERO,
            level_travel2: 0.0,
            placed_offset: [Vec3::ZERO; 4],
        }
        .settle()
    }

    /// Records where the part is as where the last ticks left it.
    fn settle(mut self) -> Self {
        self.settled = self.transform();
        self.shown = (self.settled, self.settled);
        self.drawn_particles = (self.positions, self.positions);
        self
    }

    /// The part's world values the sector refreshes each tick: its matrix,
    /// the world box around its mesh bound, the world sphere around its
    /// spheres and the grid cell of the mesh box centre
    /// ([`crate::sector::BodyPose`]).
    pub(crate) fn pose(&self, origin: (i32, i32)) -> crate::sector::BodyPose {
        crate::sector::body_pose(
            &self.rest,
            &self.rest_inverse,
            &self.positions,
            &self.pairing.mesh,
            &self.pairing.sphere,
            origin,
        )
    }

    /// The part's transform `alpha` of the way from where the tick before
    /// last left it to where the last tick did: the original interpolates
    /// the four particles and builds the matrix from them, so the drawn
    /// pose has no slerp or scale drift of its own.
    pub fn drawn(&self, alpha: f32) -> Affine3A {
        let (from, to) = self.drawn_particles;
        if from == to {
            return affine_from_points(&self.rest, &self.rest_inverse, &to);
        }
        let mut points = [Vec3::ZERO; 4];
        for i in 0..4 {
            // a + (b − a)·t, as the original sums it (not a(1 − t) + bt).
            points[i] = from[i] + (to[i] - from[i]) * alpha;
        }
        affine_from_points(&self.rest, &self.rest_inverse, &points)
    }

    fn point(&self, w: &[f32; 4]) -> Vec3 {
        (0..4).map(|i| self.positions[i] * w[i]).sum()
    }

    /// The point with weights `w` where the tick began.
    fn point_at_start(&self, w: &[f32; 4]) -> Vec3 {
        (0..4).map(|i| self.start[i] * w[i]).sum()
    }

    /// The point with weights `w` in the part's own space.
    fn rest_point(&self, w: &[f32; 4]) -> Vec3 {
        (0..4).map(|i| self.rest[i] * w[i]).sum()
    }

    /// Moves each particle by `delta` times its weight in `w`, unclamped (a
    /// negative weight moves its particle the other way, as the original's
    /// does): the point itself moves by Σw² of `delta`.
    fn push(&mut self, w: &[f32; 4], delta: Vec3) {
        for (p, wi) in self.positions.iter_mut().zip(w) {
            *p += delta * *wi;
        }
    }

    /// Records `delta` times the weights `w` in the placed-offset array
    /// (the pair pass clamps its weights at zero first; the placed hit
    /// does not).
    fn offset_push(&mut self, w: &[f32; 4], delta: Vec3) {
        for (p, wi) in self.placed_offset.iter_mut().zip(w) {
            *p += delta * *wi;
        }
    }

    /// Weights of the world point `x` over the current particles.
    fn weights_now(&self, x: Vec3) -> Option<[f32; 4]> {
        let edges = Mat3::from_cols(
            self.positions[1] - self.positions[0],
            self.positions[2] - self.positions[0],
            self.positions[3] - self.positions[0],
        );
        if edges.determinant().abs() < 1e-12 {
            return None;
        }
        let w = edges.inverse() * (x - self.positions[0]);
        Some([1.0 - w.x - w.y - w.z, w.x, w.y, w.z])
    }

    /// How far the point with weights `w` moved this tick.
    fn slip(&self, w: &[f32; 4]) -> Vec3 {
        (0..4)
            .map(|i| (self.positions[i] - self.start[i]) * w[i])
            .sum()
    }

    /// The part's smoothed velocity (m a tick) at the point with weights
    /// `w`: the original's impact travel reads its smoothed array so.
    fn smoothed_velocity(&self, w: &[f32; 4]) -> Vec3 {
        (0..4).map(|i| self.smoothed[i] * w[i]).sum()
    }

    /// A level hit on sphere `centre` (part space): the closest point `q` on
    /// the triangle and the push `delta` that takes the sphere off it. The
    /// part is pushed at once and the contact kept for the next tick.
    fn hit(&mut self, centre: Vec3, q: Vec3, delta: Vec3) -> bool {
        let len2 = delta.length_squared();
        // The original's `eps <= |δ|²` lets a NaN through to its own check,
        // which logs and returns; +∞ passes and is used unclamped.
        if len2.is_nan() || len2 < MIN_PUSH2 {
            return false;
        }
        if len2 > EXCESSIVE_PUSH * EXCESSIVE_PUSH {
            warn_once!("excessive level contact push: {:.2} m", len2.sqrt());
        }
        // The deepest point of the sphere, or its centre when that lies far
        // outside the tetrahedron.
        let w = self
            .weights_now(q - delta)
            .filter(|w| w.iter().all(|x| x.abs() <= 1.0))
            .unwrap_or_else(|| weights(self, centre));
        let normal = delta / len2.sqrt();
        self.offer(LevelContact {
            weights: w,
            point: q,
            normal,
            plane: normal.dot(q) - CONTACT_SLOP,
        });
        // Spread so that the heaviest particle takes the whole push.
        let clamped = w.map(|x| x.max(0.0));
        let top = clamped.iter().copied().fold(0.0, f32::max);
        if top > 0.0 {
            for (x, c) in self.positions.iter_mut().zip(clamped) {
                *x += delta * (c / top);
            }
        }
        // The level impact's bookkeeping: the smoothed velocity at the
        // contact weights as found (the original reads it before it
        // normalises them for the push), and the point and push sums.
        let travel = balance::cm(self.smoothed_velocity(&w));
        let travel2 = travel.z * travel.z + (travel.y * travel.y + travel.x * travel.x);
        self.level_hits += 1;
        self.level_point += q;
        self.level_push += delta;
        if travel2 > self.level_travel2 {
            self.level_travel = travel;
            self.level_travel2 = travel2;
        }
        self.strain += len2.sqrt();
        if self.strain > PUSH_STRAIN {
            keep_rigid(self);
        }
        true
    }

    /// Keeps a new contact, or, with four kept, the four spread widest over
    /// the part.
    fn offer(&mut self, contact: LevelContact) {
        if self.contacts.len() < MAX_CONTACTS {
            self.contacts.push(contact);
            return;
        }
        let mut all = [contact.weights; MAX_CONTACTS + 1];
        for (w, c) in all.iter_mut().zip(&self.contacts) {
            *w = c.weights;
        }
        let out = evicted(&all);
        if out < MAX_CONTACTS {
            self.contacts.swap_remove(out);
            self.contacts.push(contact);
        }
    }

    /// Enforces the stored contacts once: a point under its plane is lifted
    /// onto it, and its slip this tick is cancelled below the friction
    /// limit and damped above it.
    fn hold_contacts(&mut self) {
        let mut depth = 0.0;
        for k in 0..self.contacts.len() {
            let c = self.contacts[k];
            let mut correction = rub(self.slip(&c.weights), c.normal, self.friction);
            let below = c.plane - c.normal.dot(self.point(&c.weights));
            if below > MIN_DEPTH {
                correction += c.normal * below;
                depth += below;
            }
            for (x, w) in self.positions.iter_mut().zip(c.weights) {
                *x += correction * w.max(0.0);
            }
            self.strain += depth;
            if self.strain > DEPTH_STRAIN {
                keep_rigid(self);
            }
        }
    }

    /// Friction only, from the contacts found this tick.
    fn rub_contacts(&mut self) {
        for k in 0..self.contacts.len() {
            let c = self.contacts[k];
            let correction = rub(self.slip(&c.weights), c.normal, self.friction);
            for (x, w) in self.positions.iter_mut().zip(c.weights) {
                *x += correction * w;
            }
        }
    }

    /// The part's rigid world transform from its particles: their affine
    /// frame through the rest basis, with the nearest rotation taken by
    /// Gram–Schmidt (the edges keep it near rigid).
    pub fn transform(&self) -> Affine3A {
        let edges = Mat3::from_cols(
            self.positions[1] - self.positions[0],
            self.positions[2] - self.positions[0],
            self.positions[3] - self.positions[0],
        );
        let m = edges * self.rest_inverse;
        let x = m.x_axis.normalize_or(Vec3::X);
        let y = (m.y_axis - x * x.dot(m.y_axis)).normalize_or(Vec3::Y);
        let z = x.cross(y);
        let r = Mat3::from_cols(x, y, z);
        let rest_centre: Vec3 = self.rest.iter().copied().sum::<Vec3>() * 0.25;
        let centre: Vec3 = self.positions.iter().copied().sum::<Vec3>() * 0.25;
        Affine3A::from_mat3_translation(r, centre - r * rest_centre)
    }

    /// The part's transform where the last tick left it: where a swept contact
    /// of a simulated object starts.
    pub fn settled_transform(&self) -> Affine3A {
        self.settled
    }

    /// The world centre and radius of the sphere around the part's collision
    /// spheres, as a simulated object's body pre-test reads it: the bound
    /// travelled by the part's transform now.
    pub fn sphere_bound(&self) -> (Vec3, f32) {
        (
            self.transform()
                .transform_point3(self.pairing.sphere.centre),
            self.pairing.sphere.radius,
        )
    }
}

/// On a living body model root off its balance: from the tick its posture
/// stage takes one of its off-balance branches (balance under 0.6: a fall,
/// a stagger, the Defence stance or Rebalance; lying; getting up) until it
/// stands again. Meanwhile it is not steered (the original turns its
/// locomotion off there), engages no attack, and its stance does not
/// switch.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Fallen;

/// Asks for the body under this entity (a body model root) to go limp:
/// its parts become a [`Ragdoll`] from their current pose.
#[derive(Component, Debug, Clone, Copy)]
pub struct GoLimp {
    /// A velocity given to the whole body, m/s; none keeps the motion it
    /// has (a death).
    pub velocity: Option<Vec3>,
    /// Frames to wait first (a body spawned this frame is not posed yet).
    pub delay: u32,
}

/// Makes the body under this entity (a body model root) physical: its parts
/// become a [`Ragdoll`] with muscles following its animation.
#[derive(Component, Debug, Clone, Copy)]
pub struct Physical {
    /// Frames to wait first (a body spawned this frame is not posed yet).
    pub delay: u32,
    /// The race's muscle factors.
    pub race: RaceMuscles,
    /// What the body takes from its own character's stats.
    pub stats: BodyStats,
}

impl Default for Physical {
    fn default() -> Self {
        Self {
            delay: 3,
            race: RaceMuscles::HUMAN,
            stats: BodyStats::HUMAN,
        }
    }
}

impl Default for GoLimp {
    fn default() -> Self {
        Self {
            velocity: None,
            delay: 3,
        }
    }
}

/// A body model root a level placed from a saved pose: its parts already
/// sit where the pose puts them, and it is built limp with no muscles and
/// registered at rest ([`Ragdoll::register_placed`]), so it runs one tick
/// and sleeps. `delay` frames let the nodes' world transforms propagate
/// before the particles are placed.
#[derive(Component, Debug, Clone, Copy)]
pub struct PlacedCorpse {
    pub delay: u32,
}

impl Default for PlacedCorpse {
    fn default() -> Self {
        Self { delay: 3 }
    }
}

/// One pass of a breakable grab between a weapon part and the part it bit.
///
/// The pull is the stored strength divided by 1 plus 0.001 times the gap in
/// centimetres, shared equally and oppositely. The strain falls by
/// 1 + 2q/|d| when q is above 0, and by 1 otherwise. Below 0 the grab
/// lets go. Flag 0x200000 moves the struck point onto the weapon point
/// once and pulls on the passes after that.
#[derive(Debug, Clone)]
pub struct GrabPull {
    pub strain: f32,
    pub flags: u32,
    pub strength: f32,
    pub axis: Vec3,
    pub on_weapon: [f32; 4],
    pub on_struck: [f32; 4],
}

impl Ragdoll {
    /// Pulls part `weapon` and part `struck` through one grab pass.
    pub fn pull_grab(&mut self, weapon: usize, struck: usize, grab: &mut GrabPull) {
        if weapon == struck || weapon >= self.parts.len() || struck >= self.parts.len() {
            return;
        }
        let (weapon_part, struck_part) = if weapon < struck {
            let (left, right) = self.parts.split_at_mut(struck);
            (&mut left[weapon], &mut right[0])
        } else {
            let (left, right) = self.parts.split_at_mut(weapon);
            (&mut right[0], &mut left[struck])
        };
        pull_grab(weapon_part, struck_part, grab);
    }
}

fn pull_grab(weapon: &mut Part, struck: &mut Part, grab: &mut GrabPull) {
    if grab.flags & 0x100000 != 0 {
        return;
    }
    if grab.flags & 0x200000 != 0 {
        grab.flags &= !0x200000;
        let at = weapon.point(&grab.on_weapon);
        if let Some(weights) = struck.weights_now(at) {
            grab.on_struck = weights;
        }
        return;
    }
    let gap = struck.point(&grab.on_struck) - weapon.point(&grab.on_weapon);
    let len2 = gap.length_squared();
    if len2 <= LINK_EPSILON2 {
        return;
    }
    let len_cm = len2.sqrt() * CM;
    let local = weapon
        .transform()
        .to_scale_rotation_translation()
        .1
        .inverse()
        * gap;
    let q = 1.0 + local.dot(grab.axis);
    let drop = if q > 0.0 { 1.0 + 2.0 * q / len_cm } else { 1.0 };
    grab.strain -= drop;
    if grab.strain < 0.0 {
        grab.flags |= 0x100000;
        return;
    }
    let factor = grab.strength / (1.0 + PIVOT_SOFTENING * len_cm);
    let share = 0.5 * factor;
    weapon.push(&grab.on_weapon, gap * share);
    struck.push(&grab.on_struck, gap * -share);
}

/// A link between a part and its parent part: a point on each, allowed to
/// drift apart by a slack. A link with no slack is a pivot.
#[derive(Debug, Clone)]
pub struct Link {
    pub part: usize,
    pub parent: usize,
    on_part: [f32; 4],
    on_parent: [f32; 4],
    pub slack: f32,
    /// 1, or 0.2 on a living lower leg's limit link.
    pub strength: f32,
    /// The link's kind: [`LINK_BODY`], with [`LINK_SOFT_WELD`] or
    /// [`LINK_BREAKABLE`], and the flags code sets ([`LINK_DISABLED`],
    /// [`LINK_ITERATION_LIMIT`], [`LINK_REANCHOR`], [`LINK_CODE`]).
    pub flags: u32,
    /// A pivot's softening per cm² of its gap (the original's 0.001).
    pub softness: f32,
    /// A link that runs only in the passes up to this one (flag
    /// [`LINK_ITERATION_LIMIT`]).
    pub iteration_limit: Option<u32>,
    /// How fast a limp joint's smoothed length follows its links inside the
    /// slack: (1 − (r² − 25)·0.001)², at most 0.99, r the point's distance
    /// from the part's origin in cm (0 within √0.1 cm of it).
    pub smoothing: f32,
    /// The smoothed length (m) a limp slack link holds its points to: the
    /// posed distance when the body is built, the distance within the
    /// slack when it goes limp.
    pub length: f32,
    /// The shares of a correction the part and its parent take, by their
    /// mobilities: m/(m + m_parent) and m_parent/(m + m_parent).
    shares: (f32, f32),
    /// The part's and its parent's mobilities (a soft weld shares by these
    /// directly).
    mobility: (f32, f32),
    /// The pivot at the part's origin, not one of the skeleton's links.
    pub origin: bool,
    /// A limit link: a skeleton link without the pivot flag. Its slack is
    /// compared with the gap's square, and its correction moves the part
    /// alone, at most 10 cm (2 cm when horizontal) past it.
    pub limit: bool,
    /// A limit link that only holds horizontally, in the first two
    /// iterations of the tick.
    pub horizontal: bool,
    /// A living slack link's muscle: where the animated joint angle puts
    /// the part's point, as weights on the parent's particles (none before
    /// the body's first pose), and that point in the parent's space when
    /// last sampled.
    anchor: Option<[f32; 4]>,
    kept: Option<Vec3>,
    /// The part's point and the (relaxed) anchor where the tick began: the
    /// muscle damps how the gap between them changes over the tick.
    history: (Vec3, Vec3),
}

/// The limp link law on gap `d` (the parent's point minus the part's) for
/// a link of `slack`, `strength`, pivot `softness` and length `smoothing`,
/// with its smoothed `length`: [`Link`]'s limp law, shared with the
/// cross-body grip links of a held item (`crate::grip_solve`).
pub(crate) fn limp_law(
    d: Vec3,
    slack: f32,
    strength: f32,
    softness: f32,
    smoothing: f32,
    length: &mut f32,
) -> Option<Vec3> {
    let len2 = d.length_squared();
    // The original has no guard on a pivot's gap; here gaps under the
    // epsilon (3.4 µm) are left alone, as they always were: closing them
    // too only lengthens a resting limp body's settling.
    if len2 <= LINK_EPSILON2 {
        return None;
    }
    if slack <= LIMP_PIVOT_SLACK {
        return Some(d * (strength / (len2 * CM * CM * softness + 1.0)));
    }
    let len = len2.sqrt();
    if len > slack {
        *length = slack;
        return Some(d * ((len - slack) / len * strength));
    }
    let old = *length;
    // A NaN length change is left alone too (the original's compare).
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let still = !((len - old).abs() > LENGTH_EPSILON);
    if still {
        return None;
    }
    *length = old * smoothing + (1.0 - smoothing) * len;
    Some(d * ((len - old) / len * strength))
}

impl Link {
    /// The correction of a living body's link with gap `d` (the parent's
    /// point minus the part's): a pivot closes d/(1 + 0.001|d|) (cm), a
    /// slack link takes back its excess beyond the slack, cut to 80 cm.
    fn living(&self, d: Vec3) -> Option<Vec3> {
        let len2 = d.length_squared();
        if len2 <= LINK_EPSILON2 {
            return None;
        }
        let len = len2.sqrt();
        if self.slack <= PIVOT_SLACK {
            if len > MAX_CORRECTION {
                warn_once!("excessive pivot delta: {:.2} m", len);
            }
            return Some(d / (1.0 + PIVOT_SOFTENING * len * CM));
        }
        if len <= self.slack {
            return None;
        }
        let excess = d * ((len - self.slack) / len);
        if excess.length_squared() > MAX_CORRECTION * MAX_CORRECTION {
            warn_once!("excessive constraint delta: {:.2} m", excess.length());
            return Some(excess.clamp_length_max(MAX_CORRECTION));
        }
        Some(excess)
    }

    /// The correction of a living body's limit link with gap `d` (the
    /// parent's point minus the part's). The link's slack is a squared
    /// distance in the original's centimetres, so the test is
    /// `|d|² > slack` there (`|d|²·100 > slack` here, metres). Past it the
    /// part alone moves by `strength·d`, its magnitude cut to 10 cm once
    /// the gap is over 10 cm (2 cm and horizontal-only, the vertical y gap
    /// zeroed, for a horizontal link). The squared length sums x² + y²
    /// first, then z². Only an ordered "inside the slack" compare returns
    /// nothing: a NaN gap goes on to the strength-scaled move, as the
    /// original's branch does (the caller drops a non-finite move).
    fn limit_correction(&self, d: Vec3) -> Option<Vec3> {
        let d = if self.horizontal {
            Vec3::new(d.x, 0.0, d.z)
        } else {
            d
        };
        let len2 = (d.x * d.x + d.y * d.y) + d.z * d.z;
        if len2 * CM <= self.slack {
            return None;
        }
        let cap = if self.horizontal { 2.0 } else { 10.0 };
        let cap2 = (cap / CM) * (cap / CM);
        if len2 > cap2 {
            Some(d * (self.strength * cap / CM / len2.sqrt()))
        } else {
            Some(d * self.strength)
        }
    }

    /// The correction of a limp body's link with gap `d`: a pivot closes
    /// strength/(1 + 0.001|d|²) of it (cm²). A slack link inside its slack
    /// is a damper: its smoothed length L follows the gap, L ← s·L + (1 −
    /// s)·|d|, and the points are taken back to the L they had, by strength
    /// × (|d| − L)/|d|; beyond the slack it takes back strength times its
    /// excess, and L is the slack.
    fn limp(&mut self, d: Vec3) -> Option<Vec3> {
        limp_law(
            d,
            self.slack,
            self.strength,
            self.softness,
            self.smoothing,
            &mut self.length,
        )
    }

    /// Starts a limp slack link's smoothed length at the gap `d`, within
    /// the slack, as the original's death and placement do.
    fn reset_length(&mut self, d: Vec3) {
        if self.slack <= LIMP_PIVOT_SLACK {
            return;
        }
        let len2 = d.length_squared();
        // The slack is tried first, so a gap past it takes the slack even
        // when both are under the epsilon; a NaN gap falls through to 0.
        self.length = if self.slack * self.slack < len2 {
            self.slack
        } else if len2 > LINK_EPSILON2 {
            len2.sqrt()
        } else {
            0.0
        };
    }

    /// The correction of a soft weld with gap `d` (m) whose two link points
    /// moved `moved` (m) in total this tick: a quarter of the strength times
    /// the gap, less 0.4 of the points' motion. The shares are
    /// [`Link::weld_shares`]. None under the general epsilon (a NaN gap is
    /// not under it).
    fn welded(&self, d: Vec3, moved: Vec3) -> Option<Vec3> {
        if d.length_squared() < GENERAL_EPSILON / (CM * CM) {
            return None;
        }
        Some(d * (self.strength * 0.25) + moved * -0.4)
    }

    /// The shares a soft weld's correction takes on the part and its parent:
    /// each mobility times a tenth over their sum, the tenth divided in
    /// double as the original's constant is.
    fn weld_shares(&self) -> (f32, f32) {
        let (m, m_parent) = self.mobility;
        let share = (0.1_f64 / f64::from(m + m_parent)) as f32;
        (m * share, m_parent * share)
    }
}

/// The per-tick retention of `r` kept per second: r^(1/83), none for r ≤ 0.
pub(crate) fn per_tick(r: f32) -> f32 {
    if r <= 0.0 { 0.0 } else { r.powf(1.0 / BODY_HZ) }
}

/// How fast a part moved in the last tick: the band its retention and the
/// sleep countdown go by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Speed {
    Resting,
    Slow,
    Fast,
}

/// The band of a part whose farthest particle moved `moved` cm in the last
/// tick: fast strictly over 40/83 cm, slow strictly over 20/83 cm, else
/// resting.
fn speed_band(moved: f32) -> Speed {
    if moved > RETENTION_SPEEDS[0] / BODY_HZ {
        Speed::Fast
    } else if moved > RETENTION_SPEEDS[1] / BODY_HZ {
        Speed::Slow
    } else {
        Speed::Resting
    }
}

/// The velocity a limp part keeps this tick, by the farthest one of its
/// particles moved in the last tick (`moved`, cm): the fast retention above
/// 40/83 cm, the slow one above 20/83 cm, else the resting one.
fn limp_retention(moved: f32) -> f32 {
    let band = match speed_band(moved) {
        Speed::Fast => 0,
        Speed::Slow => 1,
        Speed::Resting => 2,
    };
    per_tick(RETENTION[band])
}

/// A limp body's sleep countdown after a tick whose fastest part moved in
/// `band` last tick: fast wakes it to 256; slow lifts it to 32 from below,
/// takes one off above 32 and two at 32, so it is held at 30–32; resting
/// takes two off. It never falls below 1.
fn count_down(countdown: i32, band: Speed) -> i32 {
    match band {
        Speed::Fast => countdown.max(WAKE_COUNT),
        Speed::Slow if countdown < SLOW_COUNT => SLOW_COUNT,
        Speed::Slow if countdown > SLOW_COUNT => (countdown - 1).max(SLEEP_COUNT),
        _ => (countdown - 2).max(SLEEP_COUNT),
    }
}

/// Whether a body is simulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sleep {
    #[default]
    Awake,
    /// Fell asleep in its last post-step: its next tick only takes its
    /// velocity away (its positions become its previous ones).
    Asleep,
    /// Asleep for over a tick: nothing is done until something wakes it.
    Deep,
}

/// A dormant statue or golem keeps this share of the living retention
/// (the original's double 0.95, applied in double before the body's
/// factor).
const DORMANT_KEEP: f64 = 0.95;
/// A body scale over this (double) in combat, outside an attack, keeps
/// 0.97 of u² instead of u.
const LARGE_SCALE: f64 = 1.3;
const HEAVY_KEEP: f64 = 0.97;

/// The factor a living limb's fast retention is multiplied by. `u` is
/// (1 − balance)(1 − r) + r, all in single precision. While `eased` (the
/// condition is walking, or it holds bit 0x200: down, getting up, dormant
/// or dead) it is 0.5·u + 0.5. Standing it is u², but in combat (`in_combat`,
/// the behaviour's in-combat flag) it is u, or 0.97·u² in double when the
/// body scale compares over 1.3 and the condition is not the attack's. A
/// NaN balance or ratio stays NaN.
pub(crate) fn retention_factor(
    balance: f32,
    ratio: f32,
    eased: bool,
    in_combat: bool,
    attacking: bool,
    scale: f32,
) -> f32 {
    let u = (1.0 - balance) * (1.0 - ratio) + ratio;
    if eased {
        u * 0.5 + 0.5
    } else if in_combat {
        if f64::from(scale) > LARGE_SCALE && !attacking {
            (f64::from(u * u) * HEAVY_KEEP) as f32
        } else {
            u
        }
    } else {
        u * u
    }
}

/// The velocity a living body's limbs keep each tick, whatever their
/// speed: the fast retention × `factor` ([`retention_factor`]), ×
/// [`DORMANT_KEEP`] while `dormant`.
fn living_retention(factor: f32, dormant: bool) -> f32 {
    let fast = per_tick(RETENTION[0]);
    let keep = if dormant {
        (f64::from(fast) * DORMANT_KEEP) as f32
    } else {
        fast
    };
    keep * factor
}

/// The original's smoothing for a link point `r` (m) from its part's origin.
fn setup_smoothing(r: Vec3) -> f32 {
    let r2 = (r * CM).length_squared();
    if r2 <= 0.1 {
        return 0.0;
    }
    (1.0 - (r2 - 25.0) * 0.001).powi(2).min(0.99)
}

/// The flags of a root body's links to its node (the world). Registration
/// keeps them as they are; a slider keeps its axis and travel.
pub mod world_link {
    /// A slider: the link's slack is its travel and its axis the direction.
    pub const SLIDER: u32 = 0x4;
    /// A damped spring (a soft weld).
    pub const SPRING: u32 = 0x8;
    /// A horizontal drag (a hand pulling a body across the ground).
    pub const HORIZONTAL: u32 = 0x200;
    /// A plane: the correction's axis component mostly removed.
    pub const PLANE: u32 = 0x800;
    /// The link is solved in the passes up to its last one only.
    pub const ITERATION_LIMIT: u32 = 0x4000;
    /// A horizontal drag's step cap per root of the part's mobility, and its
    /// most (cm).
    pub(super) const DRAG_PER_ROOT_MOBILITY: f32 = 3.0;
    pub(super) const DRAG_MAX_STEP: f32 = 5.0;
    /// A zero-slack link's longest full pull per unit of strength (cm), and
    /// the squared length (cm²) its longer pulls are cut to.
    pub(super) const PLAIN_MAX_STEP: f32 = 10.0;
    pub(super) const PLAIN_CAP_SQUARED: f32 = 100.0;
    /// The share of the gap along a plane's axis a plane link removes (a
    /// double in the original).
    pub(super) const PLANE_REMOVAL: f64 = 0.8;
}
use world_link::{
    DRAG_MAX_STEP, DRAG_PER_ROOT_MOBILITY, PLAIN_CAP_SQUARED, PLAIN_MAX_STEP, PLANE_REMOVAL,
};

/// A link from a body part to its node (the world): a point on the part
/// held to an anchor in world space. A hung body's root has these instead
/// of body links; the world does not move, so the correction is unshared.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldLink {
    /// The part the link holds.
    pub part: usize,
    /// The point's weights on the part's particles.
    pub on_part: [f32; 4],
    /// The anchor in world space (m).
    pub anchor: Vec3,
    /// The free length (m): 0 for a pin; a slider's travel.
    pub slack: f32,
    pub strength: f32,
    pub smoothing: f32,
    pub flags: u32,
    /// A slider's axis in world space, its length the travel; a plane's
    /// normal.
    pub axis: Vec3,
    /// The slider's fraction of its travel; a slack link's taut length.
    pub length: f32,
    /// Whether this is the part's origin pivot.
    pub origin: bool,
    /// With [`world_link::ITERATION_LIMIT`], the last iteration pass the
    /// link runs in.
    pub last_pass: Option<u32>,
}

impl WorldLink {
    /// The correction of this link for its part: the original's world-link
    /// law, worked in centimetres in its branch order (horizontal drag,
    /// slider, damped spring, plain pin or slack link). The point's motion
    /// this tick is how far it moved from where the tick began. A slider
    /// also updates its fraction; a plain slack link records the distance
    /// once it is taut. None when the link is inside its slack or the
    /// points coincide.
    fn correction(&mut self, p: &Part) -> Option<Vec3> {
        let point = p.point(&self.on_part);
        let mut gap = (self.anchor - point) * CM;
        let moved = (point - p.point_at_start(&self.on_part)) * CM;
        if self.flags & world_link::HORIZONTAL != 0 {
            gap.y = 0.0;
            let len2 = gap.x * gap.x + gap.z * gap.z;
            if len2 < GENERAL_EPSILON {
                return None;
            }
            let len = len2.sqrt();
            let unit = gap * (1.0 / len);
            let mut cap = p.inverse_mass.sqrt() * DRAG_PER_ROOT_MOBILITY;
            if cap > DRAG_MAX_STEP {
                cap = DRAG_MAX_STEP;
            }
            let mut pull = if cap < len {
                gap * (cap / len * self.strength * 2.0)
            } else {
                gap * (self.strength * 2.0)
            };
            let toward = moved.dot(unit);
            if toward > GENERAL_EPSILON {
                pull -= unit * toward;
            }
            return Some(pull / CM);
        }
        if self.flags & world_link::SLIDER != 0 {
            let travel = self.slack;
            if travel <= 0.0 || !travel.is_finite() || !self.axis.is_finite() {
                return None;
            }
            // Positions in metres (the fraction is a ratio, so it has no
            // unit): the point is drawn `smoothing` of the way to where its
            // last fraction put it, then projected on the axis.
            let target = self.anchor + self.axis * self.length;
            let pull = (target - point) * self.smoothing;
            let q = point + pull;
            let t = (self.axis.dot(q - self.anchor) / (travel * travel)).clamp(0.0, 1.0);
            self.length = t;
            let slid = self.anchor + self.axis * t;
            return Some((slid - q) + pull);
        }
        if self.flags & world_link::SPRING != 0 {
            let len2 = gap.length_squared();
            if len2 < GENERAL_EPSILON {
                return None;
            }
            // A tenth of the strength, in double as the original's constant.
            let mut pull = gap * (f64::from(self.strength) * 0.1) as f32;
            // The original normalises the scaled pull by the raw gap's
            // length, so this "unit" is a tenth of the strength long.
            let unit = pull * (1.0 / len2.sqrt());
            let toward = moved.dot(unit);
            if toward > GENERAL_EPSILON {
                pull -= unit * toward;
            }
            pull += moved * -0.4;
            return Some(pull / CM);
        }
        if self.flags & world_link::PLANE != 0 {
            let along = (f64::from(gap.dot(self.axis)) * PLANE_REMOVAL) as f32;
            gap -= self.axis * along;
        }
        let len2 = gap.length_squared();
        let slack = self.slack * CM;
        if slack * slack >= len2 {
            self.length = 0.0;
            return None;
        }
        // A NaN slack takes the no-slack branch, as the original's compare does.
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        let no_slack = !(slack > GENERAL_EPSILON);
        let factor = if no_slack {
            let reach = self.strength * PLAIN_MAX_STEP;
            if reach * reach >= len2 {
                self.strength
            } else {
                (PLAIN_CAP_SQUARED / len2).sqrt() * self.strength * self.strength
            }
        } else {
            let len = len2.sqrt();
            self.length = len / CM;
            (len - slack) / len * self.strength
        };
        Some(gap * factor / CM)
    }
}

/// A simulated body: limp, or driven by its muscles when it has a
/// [`Drive`].
#[derive(Component, Debug, Clone)]
pub struct Ragdoll {
    pub parts: Vec<Part>,
    /// Every part's links, the skeleton's then its origin pivot, part by part.
    pub links: Vec<Link>,
    /// The links of a hung root body to its node (the world), solved with the
    /// body's links every iteration.
    pub world: Vec<WorldLink>,
    /// The links of each part.
    links_of: Vec<std::ops::Range<usize>>,
    /// The parts parents first, depth first (a limp body's order), and
    /// children first (a living body's).
    outward: Vec<usize>,
    inward: Vec<usize>,
    /// The thorax pins whose mean is the chest (part, weights), the legs
    /// and the feet's sole height (m): what the balance reads.
    chest: Vec<balance::Pin>,
    legs: Vec<Leg>,
    sole: f32,
    pub drive: Option<Drive>,
    /// Ticks the body was solved since the frame began.
    pub ticks: u32,
    /// The sleep countdown of the whole body: held at 256 while it lives,
    /// counted down by its speed once limp ([`count_down`]); the post-step
    /// puts it to sleep at 1.
    countdown: i32,
    /// Ticks still to run with gravity but no inertia, as the original
    /// settles a re-placed character (its temporary state's countdown).
    settle: i32,
    /// A sleeping body is not simulated until something wakes it.
    pub sleep: Sleep,
    /// Whether the body has been simulated since it was built: a body
    /// registered asleep is drawn where its nodes already are.
    simulated: bool,
    /// Its root part fell below [`FALL_OUT_HEIGHT`], where the original
    /// removes a body's node.
    fell_out: bool,
    /// The hand's grabs on its parts ([`Ragdoll::grab`]).
    grabs: Vec<Grab>,
    /// Scratch lists for the level detection, kept across ticks so a tick
    /// does not allocate.
    pieces: Vec<u32>,
    triangles: Vec<[Vec3; 3]>,
    /// The iteration pass this tick is in, from 0: a grab with a last pass
    /// reads it.
    pass: u32,
    /// The class flags of the bodies it holds (two hands at most), as the
    /// original keeps them on the held item bodies: bit 0x10000 is the
    /// motion flag a walk's start clears, bit 0x80000 makes every level
    /// query static (no sweep). The grip links now own the held items;
    /// this word is kept for them and no part reads it yet.
    held_flags: [u32; 2],
    held_count: usize,
    /// The body of the character holding this one (a held item): the two
    /// never meet, the original's all-ones group mask on the item.
    pub held_by: Option<Entity>,
    /// The level impacts this tick's detection found (a fall or scrape hard
    /// enough to sound and hurt), one per part.
    level_impacts: Vec<LevelImpact>,
    /// The character frame the body had when it lost its muscles: the
    /// muscles-off sampler places its clip pose on it.
    limp_frame: Option<Affine3A>,
    /// The race's base clip rate, kept when the muscles go: the
    /// muscles-off sampler runs at 0.9 of it while the body has no support
    /// (the original's unsupported, non-locomoting clock).
    limp_clip_rate: f32,
    /// The standing height the muscles-off sampler eases (cm): 0.05 of its
    /// target a tick.
    limp_height: f32,
    /// The body scale the muscles-off sampler's height target reads.
    limp_scale: f32,
    /// The body's scrape and roll sound sources, each following a point in
    /// its part's frame ([`Ragdoll::add_scrape_on`]).
    scrapes: Vec<ScrapeSource>,
    /// What this tick's scrape sources reported, for the audio side.
    scrape_events: Vec<ScrapeEvent>,
}

/// Muscles of a living body: the animated pose it follows.
#[derive(Debug, Clone)]
pub struct Drive {
    /// Animated world transform of each part this tick, the clip's pose on
    /// the character frame: the pose drive reads its rotation, the link
    /// muscles each part's turn relative to its parent's, the support pins
    /// the thorax pins' and the pelvis's place.
    targets: Vec<Affine3A>,
    /// The body root's world transform when last aimed, less the moves its
    /// controller made to follow the body since.
    root: Option<Affine3A>,
    /// The character frame's origin: the character's place, where the
    /// pelvis was less the pose's root offset when last anchored, at the
    /// pelvis's height then (`frame`).
    anchor: Option<Vec3>,
    /// The character frame: the body root's turn, at the anchor across the
    /// ground.
    frame: Option<Affine3A>,
    /// The stance facing (a yaw) the footwork turns its spots onto. Not a
    /// number until the turn writes one, and then the spots stay in the
    /// character frame.
    stance_yaw: f32,
    /// The head's parent as posed before the torso twist, in the same
    /// space as [`Drive::targets`]. While the head is not looking, its
    /// link anchors are placed from its pose relative to this, so the
    /// twist in the parent's target does not turn them. None while the
    /// head looks (its anchors follow the physical thorax) and before a
    /// sample names one.
    head_pose_parent: Option<Affine3A>,
    /// [`Drive::note_head_pose_parent`] stored this tick's placement, so
    /// the carry leaves it where it was put.
    head_parent_fresh: bool,
    /// Muscle tone, 0..=1: muscle strength and gravity compensation. A
    /// fall leaves it as it is.
    pub tone: f32,
    /// Daze: 1 when alert; blows lower it and it recovers over time. The
    /// pose drive follows its fourth power, the link muscles its root.
    pub energy: f32,
    /// The character tick already wrote this tick's recovery, so the
    /// integration step leaves the energy.
    energy_recovered: bool,
    /// Standing on its controller, which carries it across the ground and
    /// turns it. The support pins hold it over its feet. A fallen body is
    /// not carried, and its balance is measured lying.
    pub supported: bool,
    /// The race's factors the muscles read.
    pub race: RaceMuscles,
    /// What the body takes from its own character's stats.
    pub stats: BodyStats,
    /// What the character tick knows of the body's character this tick.
    pub bearing: Bearing,
    /// Stance factor s: the share of its tone the body uses (T = tone·s).
    stance: f32,
    /// The right arm's stiffness, which the readout reports. Standing, the
    /// two arms ease apart and this is the right one ([`Self::arms`]).
    arm: f32,
    /// The two stored arm stiffnesses, right then left. A standing tick
    /// eases each; a kick leaves them; a collapse, a daze, a stride, a
    /// power and a weapon switch write both the same. An attack stores the
    /// floored pair so the next standing tick starts there. A shielded
    /// stride writes the walking arms on the weapon arm and twice that
    /// on the off arm.
    arms: (f32, f32),
    /// Support S, 0..=1.
    support: f32,
    /// The balance its character tick measures.
    balance: Balance,
    /// What its limb particles take this tick in place of gravity once
    /// their tone is over a half (m a tick²): gravity less the lift.
    carry: Vec3,
    /// The pelvis height the lift holds (m), once the character tick has
    /// measured the ground under its feet.
    lift_target: Option<f32>,
    /// The lift's factor f this tick, tone included (0 without support).
    lift: f32,
    /// The lift term before clamping: standing, 0.075(h + 15). The leg
    /// bend reads it as centimetres and lowers an unanchored target by
    /// twice it. 0 until the servo runs.
    lift_raw: f32,
    /// Blow sag s, 0..=1: how much an unsteady body's targets give way to
    /// the pose it holds after a blow (the damage routine sets it through
    /// [`Shove::sag`]); it fades 0.01 a tick in double precision, and is 0
    /// while the condition is down, getting up, dormant, collapsing or dead.
    pub sag: f32,
    /// The support pins' gain factor (the 0.08 of the pins' gain
    /// S²·pin-gain·P·pin factor): it recovers toward 0.08 every tick while
    /// the body lives ([`recover_pins`]) and fades while it collapses. The
    /// support pins read it each iteration.
    pub pin_gain: f32,
    /// The thorax hunch: a pinned part's local forward axis loses this on
    /// its height. It rises 0.01 a tick to 0.3 while the body is on its
    /// feet and falls 0.01 to 0 off them; an attack clip takes 0.02 more off.
    pub hunch: f32,
    /// Ticks the posture switch waits before it starts the body's stance
    /// again (it starts it only under 1): a Rebalance started off balance
    /// sets 41, one started unsteady 27, and every posture stage takes one
    /// off.
    pub posture_wait: i32,
    /// Its recovery step, if the chest has left the feet.
    steps: steps::Steps,
    /// The clip rate the posture block last wrote. A weak tick, and a walk
    /// or a run, leave it; the clock reads it. It starts at the human base.
    pub posture_rate: f32,
    /// The body tick's adapters, look rate and counters.
    pub timers: BodyTimers,
    /// A dormant statue or golem. Its limbs keep [`DORMANT_KEEP`] of the
    /// living retention, its sleep countdown runs, and its standing lift
    /// lead is 20 cm.
    pub dormant: bool,
    /// This tick's attack arms, when a swing is playing: the muscle tune
    /// writes each side's arm parts from it. Absent, the eased stiffness
    /// stands. The swing stage clears it at the start of every tick.
    attack_arms: Option<muscle::AttackArmTune>,
    /// The two arm-drive slots, right then left: zeroed at the start of
    /// every body tick before the arm state machine, and written only by
    /// an attack's branch. Each arm's saturation is 0.1 − 0.08·drive, so
    /// every tick that is not an attack saturates at 0.1.
    arm_drive: (f32, f32),
    /// Each target's rotation, taken once when the targets are installed
    /// (the solver's sole turns read it every iteration).
    turns: Vec<Quat>,
    /// The clip progress the walking ground probe reads (the last
    /// sample's), the locomotion speed along the heading (cm a tick) and
    /// whether that walk is backwards. [`feet::gait_probe`] turns them into
    /// the sphere, the sway and the ground's fall.
    gait_progress: f32,
    gait_speed: f32,
    gait_back: bool,
    /// The walk's travel phase: the pelvis's travel in cm times
    /// 0.0002 a tick, wrapped at 1, and its value before the tick.
    travel_phase: f32,
    travel_phase_before: f32,
}

/// One tick of the walk's travel phase: the pelvis's squared travel this
/// tick (cm²) adds its single-precision root times the double 0.0002 when
/// that is over the engine's epsilon, and a phase over 1 loses 1, the
/// phase before it too. A NaN squared travel leaves the phase.
pub fn travel_phase(last: f32, before: &mut f32, moved2_cm: f32) -> f32 {
    let mut phase = last;
    if moved2_cm > f32::from_bits(0x33ff_ffd7) {
        phase = (f64::from(moved2_cm.sqrt()) * 0.0002 + f64::from(phase)) as f32;
    }
    if phase > 1.0 {
        phase -= 1.0;
        *before -= 1.0;
    }
    phase
}

/// One tick of energy. The base is u = energy + the double 0.005; while
/// getting up t = (1 − progress)·0.01 is computed first, the difference
/// taken in single precision, and added to u. The sum narrows to single
/// precision and a value above `max_energy` becomes it; a NaN stays.
pub fn recover_energy(energy: f32, max_energy: f32, getting_up: bool, progress: f32) -> f32 {
    let sum = if getting_up {
        f64::from(1.0 - progress) * 0.01 + (f64::from(energy) + 0.005)
    } else {
        f64::from(energy) + 0.005
    };
    let next = sum as f32;
    if next > max_energy { max_energy } else { next }
}

/// One tick of blow sag. Above 0 it is 0 while `down` (the condition is
/// down, getting up, dormant, collapsing or dead), and otherwise loses the
/// double 0.01, floored at 0. A NaN, and a sag that is not above 0, stays.
pub fn fade_sag(sag: f32, down: bool) -> f32 {
    // A NaN, like a sag that does not compare above 0, stays.
    if sag.is_nan() || sag <= 0.0 {
        return sag;
    }
    if down {
        return 0.0;
    }
    let next = (f64::from(sag) - 0.01) as f32;
    if next < 0.0 { 0.0 } else { next }
}

/// One tick of the support pins' gain factor: under 0.08 (in double
/// precision) it gains 0.001, unless the body collapses.
pub fn recover_pins(gain: f32, collapsing: bool) -> f32 {
    if !collapsing && f64::from(gain) < PIN_GAIN_BELOW {
        (f64::from(gain) + PIN_RECOVERY) as f32
    } else {
        gain
    }
}

/// A body's energy after a blow that dazes it by `daze`. The loss is
/// `energy − energy × daze` in single precision. A daze that compares
/// above the double 0.9 becomes 0.9; a result that compares below the
/// double 0.05 becomes 0.05. A NaN stays, and a negative daze is not
/// raised to 0.
pub fn daze_energy(energy: f32, daze: f32) -> f32 {
    let daze = if f64::from(daze) > MAX_DAZE_F64 {
        MAX_DAZE
    } else {
        daze
    };
    let next = energy - energy * daze;
    if f64::from(next) < MIN_ENERGY_F64 {
        MIN_ENERGY
    } else {
        next
    }
}

impl Default for Drive {
    fn default() -> Self {
        Self {
            targets: Vec::new(),
            root: None,
            anchor: None,
            frame: None,
            stance_yaw: f32::NAN,
            head_pose_parent: None,
            head_parent_fresh: false,
            tone: 1.0,
            energy: 1.0,
            energy_recovered: false,
            supported: true,
            race: RaceMuscles::HUMAN,
            stats: BodyStats::HUMAN,
            bearing: Bearing::default(),
            stance: RaceMuscles::HUMAN.stance,
            arm: 0.0,
            arms: (0.0, 0.0),
            support: 1.0,
            balance: Balance::default(),
            carry: lift::gravity_per_tick(),
            lift_target: None,
            lift: 0.0,
            lift_raw: 0.0,
            sag: 0.0,
            pin_gain: PIN_GAIN,
            hunch: 0.0,
            posture_wait: 0,
            steps: steps::Steps::default(),
            posture_rate: RaceMuscles::HUMAN.clip_rate,
            timers: BodyTimers::default(),
            dormant: false,
            attack_arms: None,
            arm_drive: (0.0, 0.0),
            turns: Vec::new(),
            gait_progress: 0.0,
            gait_speed: 0.0,
            gait_back: false,
            travel_phase: 0.0,
            travel_phase_before: 0.0,
        }
    }
}

impl Drive {
    /// What the walking ground probe reads this tick: the clip `progress`
    /// (the last sample's), the locomotion `speed` along the heading (cm a
    /// tick) and whether the walk is `backwards`.
    pub fn note_gait(&mut self, progress: f32, speed: f32, backwards: bool) {
        self.gait_progress = progress;
        self.gait_speed = speed;
        self.gait_back = backwards;
    }

    /// The speed the walking ground probe's sway reads (cm a tick), as the
    /// movement stage last noted it.
    pub fn gait_speed(&self) -> f32 {
        self.gait_speed
    }

    /// The stance facing this tick (a yaw). The footwork turns its stance
    /// spots from the character frame onto it.
    pub fn note_stance_yaw(&mut self, yaw: f32) {
        self.stance_yaw = yaw;
    }

    /// The head's parent before the torso twist, in the same space as the
    /// targets the next aim installs. While the head is not looking, its
    /// anchors are placed against this. `None` while it looks.
    pub fn note_head_pose_parent(&mut self, parent: Option<Affine3A>) {
        self.head_pose_parent = parent;
        self.head_parent_fresh = true;
    }

    /// The head's parent before the twist, once a sample named one.
    pub(super) fn head_pose_parent(&self) -> Option<Affine3A> {
        self.head_pose_parent
    }

    /// Each part's animated world transform this tick (none before the
    /// first aim): the clip's pose on the character frame.
    pub fn targets(&self) -> &[Affine3A] {
        &self.targets
    }

    /// The character frame's origin (m): the character's place across the
    /// ground, at the pelvis's height when last anchored (none before the
    /// first aim). The character's controller follows it.
    /// The body root's place when last aimed (m).
    pub fn root_point(&self) -> Option<Vec3> {
        self.root.map(|r| Vec3::from(r.translation))
    }

    pub fn anchor(&self) -> Option<Vec3> {
        self.anchor
    }

    /// The character frame: the body's turn about the vertical, its origin
    /// across the ground at the anchor, at the body root's height.
    pub fn frame(&self) -> Option<Affine3A> {
        self.frame
    }

    /// Places the character frame, whose side axis the footwork's stance
    /// score reads. The body tick and a placement write it; a test of a
    /// footwork pass without one writes it here.
    pub fn set_frame(&mut self, frame: Affine3A) {
        self.frame = Some(frame);
    }

    /// The pose drive's strength this tick, which its gains are scaled by:
    /// t²·T·E⁴·(0.5T + 0.5)·S², t the tone, T = tone·s the tone target, E
    /// the energy and S the support.
    pub fn pose_strength(&self) -> f32 {
        let tone = self.tone_target();
        self.tone
            * self.tone
            * tone
            * self.energy.powi(4)
            * (0.5 * tone + 0.5)
            * self.support
            * self.support
    }

    /// The pose drive's gain G on the part of `bone` this tick: its share
    /// of K times [`Drive::pose_strength`] (an idle human's pelvis 0.0314,
    /// its head 0.0105, times its character's head gain); none on the
    /// thighs, shins, arms and hands.
    pub fn pose_gain(&self, bone: usize) -> f32 {
        let share = match bone {
            HEAD => pose_share(bone) * self.stats.head_gain,
            _ => pose_share(bone),
        };
        share * DRIVE_SCALE * self.pose_strength()
    }

    /// The stance factor s: the race's standing, rising toward 0.7 walking
    /// and 1 running.
    pub fn stance(&self) -> f32 {
        self.stance
    }

    /// The tone target T = tone·s, which scales the link muscles.
    pub fn tone_target(&self) -> f32 {
        self.tone * self.stance
    }

    /// Support S: grounded × balance, as its character tick measures them
    /// ([`Ragdoll::measure_balance`]). A body without two legs has 1 while
    /// it stands and 0 once fallen.
    pub fn support(&self) -> f32 {
        self.support
    }

    /// Takes each target's rotation once, for the solver's sole turns.
    fn refresh_turns(&mut self) {
        self.turns.clear();
        self.turns.extend(
            self.targets
                .iter()
                .map(|t| t.to_scale_rotation_translation().1),
        );
    }

    /// The body's balance.
    pub fn balance(&self) -> &Balance {
        &self.balance
    }

    /// The body's balance, to record a chest point.
    pub fn balance_mut(&mut self) -> &mut Balance {
        &mut self.balance
    }

    /// A recovery or stagger step is already running.
    pub fn stepping(&self) -> bool {
        self.steps.stepping
    }

    /// Queues a footwork step whose vector is `vector` (m) for the landing
    /// after the step that is running (the original's step queue).
    pub fn queue_step(&mut self, vector: Vec3) {
        self.steps.queued = vector;
        self.steps.queued_landings = self.balance.landings();
    }

    /// The queued step taken this tick told the AI its vector points along
    /// the facing.
    pub fn step_told_ai(&self) -> bool {
        self.steps.told_ai
    }

    /// The right arm's stiffness (its link muscle factor is 1.25 times it).
    /// Standing, the two arms ease apart and this is the right one.
    pub fn arm_stiffness(&self) -> f32 {
        self.arm
    }

    /// What the limb particles take this tick in place of gravity (m a
    /// tick²): gravity less g·f·S.
    pub fn carry(&self) -> Vec3 {
        self.carry
    }

    /// The pelvis height the lift holds (m): none for a body whose balance
    /// is not measured, −1000 m when no foot has ground.
    pub fn lift_target(&self) -> Option<f32> {
        self.lift_target
    }

    /// The lift's factor f this tick, its tone included: 1 holds a body up
    /// on its legs, 0 lifts nothing.
    pub fn lift(&self) -> f32 {
        self.lift
    }

    /// The most energy it recovers to: its character's
    /// ([`BodyStats::max_energy`]), 1 unless feral.
    pub fn max_energy(&self) -> f32 {
        self.stats.max_energy
    }

    /// Writes this tick's energy recovery. The integration step then leaves
    /// it, so the pose drive and the link muscles read the same value.
    pub fn write_energy(&mut self, getting_up: bool, progress: f32) {
        self.energy = recover_energy(self.energy, self.max_energy(), getting_up, progress);
        self.energy_recovered = true;
    }

    /// Its energy as a share of its most, E/Emax.
    pub fn energy_share(&self) -> f32 {
        self.energy / self.stats.max_energy
    }

    /// Dazed: its energy is more than 0.05 under its most (in double
    /// precision: 0.95f, a hair under the double 0.95, is dazed; a zombie
    /// whose most is 0.9 is dazed under 0.85).
    pub fn dazed(&self) -> bool {
        f64::from(self.energy) < f64::from(self.stats.max_energy) - DAZED_BELOW_MAX
    }

    /// How far its targets give way to the pose it holds this tick (its
    /// clip's pose is pulled toward it by this): dazed, 1 − (1 − S)²·(1 −
    /// s)·(E/Emax)⁴, so a dazed body standing on its feet holds its pose
    /// in full; else (1 − S)·s, S its support and s its blow sag.
    pub fn hold_weight(&self) -> f32 {
        let loose = 1.0 - self.support;
        if self.dazed() {
            let e = self.energy_share();
            1.0 - loose * loose * (1.0 - self.sag) * e.powi(4)
        } else {
            loose * self.sag
        }
    }

    /// Whether it captures the pose it holds this tick: dazed, its blow
    /// sag still on, or its support under 1.
    pub fn holds_pose(&self) -> bool {
        self.dazed() || self.sag > 0.0 || self.support < 1.0
    }
}

/// A blow to a body part: a change of velocity (m/s) and how much it dazes.
/// Blows are queued ([`ShoveQueue`]) and dealt by the next tick, whichever
/// frame sends them.
#[derive(Message, Debug, Clone, Copy)]
pub struct Shove {
    /// The body model root.
    pub body: Entity,
    pub bone: usize,
    pub velocity: Vec3,
    pub daze: f32,
    /// Horizontal knockback from the chest (m), when the blow's impact is
    /// over one half. None keeps the velocity shove.
    pub step: Option<Vec3>,
    /// The blow sag the blow sets on a living body (min(2·I, 1) for a blow
    /// on a limb); none leaves it.
    pub sag: Option<f32>,
}

/// Blows waiting for the next tick.
#[derive(Resource, Debug, Default)]
pub struct ShoveQueue(pub Vec<Shove>);

/// The blows the last tick dealt, for the debug tools (the joint trace).
#[derive(Resource, Debug, Default)]
pub struct DealtShoves(pub Vec<Shove>);

/// Ticks the bodies have run since the start.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct BodyTicks(pub u64);

/// A body taken out of the scene (an AI character out of its activation
/// range): the solver, its character tick and the drawing leave it.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OutOfScene;

/// The original's game clock advances this many milliseconds a body tick
/// (12 ms, the 83 Hz tick rounded), however long the tick took: its timers
/// in milliseconds (attack cooldowns, the double click) run on it, so they
/// stop while the simulation is paused and slow with it.
pub const GAME_MS_PER_TICK: u64 = 12;

impl BodyTicks {
    /// The game clock: milliseconds of game time the ticks so far make.
    pub fn millis(&self) -> u64 {
        self.0 * GAME_MS_PER_TICK
    }
}

/// The character ticks: every living character's tick, the player's last,
/// each character running every stage of [`CharacterTick`] before the next
/// one starts ([`stages`]): its clip advanced and sampled, and its targets
/// set (`Ragdoll::aim`). In the fixed step, after the character controllers
/// and [`BodyPreStep`], before [`BodySystems`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CharacterTicks;

/// The body solver: one tick of every awake body, with the
/// [`SolverStages`]. In the fixed step, before [`BodyPostStep`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BodySystems;

/// Ticks bodies in the fixed step and draws them between ticks. The game
/// adds it with [`crate::PhysicsPlugin`], which also sets the fixed rate.
pub struct BodyPlugin;

impl Plugin for BodyPlugin {
    fn build(&self, app: &mut App) {
        stages::configure(app);
        app.world_mut()
            .resource_mut::<SolverStages>()
            .add(Pairs::default());
        app.add_message::<Shove>()
            .add_message::<Footstep>()
            .init_resource::<ShoveQueue>()
            .init_resource::<DealtShoves>()
            .init_resource::<BodyTicks>()
            .add_systems(FixedUpdate, step_bodies.in_set(BodySystems))
            .add_systems(
                RunFixedMainLoop,
                begin_frame.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
            )
            // After every frame system that moves a character or a body
            // root (the drawn controller, turns, a get-up's teleport).
            .add_systems(PostUpdate, draw_bodies.before(TransformSystems::Propagate));
    }
}

/// What building a body needs for each part.
#[derive(Debug, Clone)]
pub struct PartSetup {
    pub bone: usize,
    pub entity: Entity,
    /// Index of the parent part, if any.
    pub parent: Option<usize>,
    /// The part's current world transform.
    pub world: Affine3A,
    /// Box of the part's mesh in its space.
    pub centre: Vec3,
    pub half: Vec3,
    /// The part's stored collision spheres (centre, radius, in its space,
    /// metres), when its skeleton carries them; `None` lets the builder fit
    /// spheres to the box.
    pub spheres: Option<Vec<(Vec3, f32)>>,
    /// Where the part's origin (its joint centre) sits in its parent's
    /// space: the model's local translation of the part. The part is pinned
    /// there, and the parent's link points are measured from it.
    pub origin: Vec3,
    /// The skeleton's links to the parent, as stored: the point on this
    /// part, the parent's point (the same point turned into the parent's
    /// axes, measured from this part's origin), and the slack.
    pub links: Vec<(Vec3, Vec3, f32)>,
    /// The part's foot contact point, in its space, when it carries one (a
    /// lower leg): its long link limits the leg, and it stands on it.
    pub foot: Option<Vec3>,
    /// The part's pins, in its space (the thorax's two: the chest).
    pub pins: Vec<Vec3>,
}

impl Ragdoll {
    /// Builds the body as the original registers it. Each part with a
    /// parent takes the skeleton's links, whose parent points are the
    /// stored points plus the part's origin in the parent (no rotation: the
    /// stored point is already turned into the parent's axes), then a pivot
    /// from its origin to that place. A lower leg's long link and its
    /// femur's limit the leg. Each part's link muscles get its factor. The
    /// thorax pins and the legs' foot contacts (at the ankles) are kept for
    /// the balance.
    pub fn new(setup: &[PartSetup]) -> Self {
        let mut parts: Vec<Part> = setup.iter().map(Part::new).collect();
        let mut links = Vec::new();
        let mut links_of = Vec::with_capacity(setup.len());
        for (i, s) in setup.iter().enumerate() {
            let first = links.len();
            if let Some(parent) = s.parent.filter(|&j| j < parts.len() && j != i) {
                let (m, m_parent) = (
                    LINK_MOBILITY * parts[i].inverse_mass,
                    LINK_MOBILITY * parts[parent].inverse_mass,
                );
                let shares = (m / (m + m_parent), m_parent / (m + m_parent));
                let mobility = (m, m_parent);
                let pivot = (Vec3::ZERO, Vec3::ZERO, 0.0);
                for (k, &(on_part, on_parent, slack)) in s.links.iter().chain([&pivot]).enumerate()
                {
                    let origin = k == s.links.len();
                    links.push(Link {
                        part: i,
                        parent,
                        on_part: weights(&parts[i], on_part),
                        on_parent: weights(&parts[parent], on_parent + s.origin),
                        slack,
                        strength: 1.0,
                        flags: if origin {
                            LINK_BODY | LINK_CODE
                        } else {
                            LINK_BODY
                        },
                        softness: PIVOT_SOFTENING,
                        iteration_limit: None,
                        smoothing: setup_smoothing(on_part),
                        length: 0.0,
                        shares,
                        mobility,
                        origin,
                        limit: false,
                        horizontal: false,
                        anchor: None,
                        kept: None,
                        history: (Vec3::ZERO, Vec3::ZERO),
                    });
                }
            }
            links_of.push(first..links.len());
        }
        // Each slack link's smoothed length starts at its posed distance.
        for l in &mut links {
            if l.slack > LIMP_PIVOT_SLACK {
                l.length = parts[l.parent]
                    .point(&l.on_parent)
                    .distance(parts[l.part].point(&l.on_part));
            }
        }
        // A lower leg's long link limits the leg, at a fifth of its
        // strength; its femur's first long link is the femur's limit.
        let long = |links: &[Link], range: &std::ops::Range<usize>| {
            range
                .clone()
                .find(|&k| !links[k].origin && links[k].slack > LEG_LIMIT_SLACK)
        };
        for (i, s) in setup.iter().enumerate() {
            if s.foot.is_none() {
                continue;
            }
            if let Some(k) = long(&links, &links_of[i]) {
                links[k].strength *= LEG_LIMIT_STRENGTH;
                parts[i].leg_limit = Some(k);
            }
            if let Some(j) = parts[i].parent.filter(|&j| j < parts.len()) {
                parts[j].leg_limit = long(&links, &links_of[j]);
            }
        }
        muscle::set_part_factors(&mut parts, BodyStats::HUMAN.head_factor);
        pairs::mask_groups(&mut parts);
        pairs::register_legs(&mut parts);
        // The parts join the world with the original's class bits: a part
        // without foot contacts keeps level contacts, a lower leg has none
        // and is no longer dynamic, nor is its foot child.
        let feet: Vec<usize> = (0..parts.len())
            .filter(|&i| parts[i].foot.is_some())
            .collect();
        for part in &mut parts {
            part.state = crate::part::world_state(part.state, part.foot.is_some());
        }
        for leg in feet {
            let children: Vec<usize> = parts
                .iter()
                .enumerate()
                .filter(|(_, part)| part.parent == Some(leg))
                .map(|(child, _)| child)
                .collect();
            for child in children {
                parts[child].state = crate::part::foot_child_state(parts[child].state);
                parts[child].flags2 |= crate::part::FOOT_CHILD_FLAG;
            }
        }
        let (outward, inward) = tree_orders(&parts);
        let (chest, legs, sole) = balance::build(setup, &mut parts);
        Self {
            parts,
            links,
            world: Vec::new(),
            links_of,
            outward,
            inward,
            chest,
            legs,
            sole,
            drive: None,
            ticks: 0,
            // Built woken. The original registers a body at 0, asleep after
            // its first tick, and wakes a living one to 256 at once; placed
            // corpses still collapse from standing in Exarust.
            countdown: WAKE_COUNT,
            settle: 0,
            sleep: Sleep::Awake,
            simulated: false,
            fell_out: false,
            grabs: Vec::new(),
            pieces: Vec::new(),
            triangles: Vec::new(),
            pass: 0,
            held_flags: [0; 2],
            held_count: 0,
            held_by: None,
            level_impacts: Vec::new(),
            limp_frame: None,
            limp_clip_rate: 1.0,
            limp_height: 0.0,
            limp_scale: 1.0,
            scrapes: Vec::new(),
            scrape_events: Vec::new(),
        }
    }

    /// Marks part `part`'s stored links with their runtime flag words: a
    /// link without [`MODEL_LINK_PIVOT`] is a limit link, one with
    /// [`MODEL_LINK_HORIZONTAL`] ignores its vertical gap. The origin pivot
    /// the builder appends after them stays a pivot. A part without links,
    /// or a flag list longer than it has stored links, is left alone and
    /// the call returns false. Not for a skeleton's stored words: the
    /// original's registration turns every parented part's link into a
    /// pivot link, so the game never calls this for a character.
    pub fn set_link_flags(&mut self, part: usize, flags: &[u32]) -> bool {
        let Some(range) = self.links_of.get(part).cloned() else {
            return false;
        };
        let stored = range.len().saturating_sub(1);
        if flags.len() > stored {
            return false;
        }
        for (k, &f) in flags.iter().enumerate() {
            let link = &mut self.links[range.start + k];
            link.limit = f & MODEL_LINK_PIVOT == 0;
            link.horizontal = f & MODEL_LINK_HORIZONTAL != 0;
        }
        true
    }

    /// The character frame the body had when its muscles went; the
    /// muscles-off sampler places the clip pose on it.
    pub fn limp_frame(&self) -> Option<Affine3A> {
        self.limp_frame
    }

    /// The race's base clip rate the muscles-off sampler runs from.
    pub fn limp_clip_rate(&self) -> f32 {
        self.limp_clip_rate
    }

    /// The standing height (cm) the muscles-off sampler eases toward its
    /// target, 0.05 a tick.
    pub fn limp_height(&self) -> f32 {
        self.limp_height
    }

    /// Sets the eased standing height.
    pub fn set_limp_height(&mut self, height: f32) {
        self.limp_height = height;
    }

    /// The body scale the muscles-off sampler's height target reads.
    pub fn limp_scale(&self) -> f32 {
        self.limp_scale
    }

    /// A recovery or stagger step is running.
    pub fn stepping(&self) -> bool {
        self.drive.as_ref().is_some_and(|d| d.stepping())
    }

    /// The original's force release on a shocked body: the step that was
    /// running ends (its feet are placed next tick), the clip rate is set
    /// to 0 and the body's next allowed shock is stamped. The caller sets
    /// the condition to the shocked state; a release in combat also
    /// resets the swing's readiness, which waits on the combat stage.
    /// Returns what the release asked of the parts Exarust does not drive.
    pub fn force_release(&mut self, now_ms: u32) -> crate::shock::Release {
        let stepping = self.stepping();
        let release = crate::shock::force_release(now_ms, stepping, 0);
        if let Some(drive) = self.drive.as_mut() {
            if stepping {
                drive.steps.stepping = false;
                drive.steps.knockback = false;
            }
            drive.posture_rate = release.clip_rate;
            drive.bearing.shocked = true;
        }
        release
    }

    /// Queues a footwork step whose vector is `vector` (m) for the landing
    /// after the step that is running.
    pub fn queue_step(&mut self, vector: Vec3) {
        if let Some(drive) = self.drive.as_mut() {
            drive.queue_step(vector);
        }
    }

    /// The queued step taken this tick told the AI its vector points along
    /// the facing.
    pub fn step_told_ai(&self) -> bool {
        self.drive.as_ref().is_some_and(|d| d.step_told_ai())
    }

    /// This tick's jolt told the body's behaviour (it locomoted).
    pub fn jolt_told(&self) -> bool {
        self.drive.as_ref().is_some_and(|d| d.steps.jolt_told)
    }

    /// The legs whose feet planted this tick (the walk's gait plants), in
    /// the body's leg order.
    pub fn planted_feet(&self) -> impl Iterator<Item = usize> + '_ {
        self.legs
            .iter()
            .enumerate()
            .filter(|(_, l)| l.planted)
            .map(|(i, _)| i)
    }

    /// How far (cm) leg `leg`'s contact sits over its ground and sole, as
    /// the IK left it for the foot pitch.
    /// 0 until the IK has run.
    pub fn foot_contact_height(&self, leg: usize) -> f32 {
        self.legs.get(leg).map_or(0.0, |l| l.pitch_height)
    }

    /// One tick of leg `leg`'s foot pitch toward `target` (rad): at most
    /// 0.01 plus the stored rate, never past the target, the rate then
    /// half of that sum within ±0.1. Returns the pitch the foot takes.
    /// A placement uses [`Self::place_foot_pitches`] instead, which stores
    /// the angle at once and clears the rate.
    pub fn step_foot_pitch(&mut self, leg: usize, target: f32) -> f32 {
        let step = match self.drive.as_ref() {
            Some(d) if d.steps.stepping => bend::PITCH_STEP_STEPPING,
            _ => bend::PITCH_STEP,
        };
        let Some(leg) = self.legs.get_mut(leg) else {
            return 0.0;
        };
        let (angle, rate) = bend::ease(leg.pitch, leg.pitch_rate, target, step);
        leg.pitch = angle;
        leg.pitch_rate = rate;
        angle
    }

    /// A placement's foot pitch: each foot on its feet stores the angle
    /// that levels its toe, and the rate is cleared. The following ticks
    /// ease from there. Off its feet the stored pitch stays.
    pub fn place_foot_pitches(mut self) -> Self {
        if self.drive.as_ref().is_some_and(|d| d.bearing.off_feet) {
            return self;
        }
        let scale = self.drive.as_ref().map(|d| d.stats.scale).unwrap_or(1.0);
        let count = self.legs.len();
        for i in 0..count {
            let (shin, foot) = {
                let Some(leg) = self.legs.get(i) else {
                    continue;
                };
                let Some(foot) = leg.foot else {
                    continue;
                };
                (leg.shin, foot)
            };
            if shin >= self.parts.len() || foot >= self.parts.len() {
                continue;
            }
            let parent = self.parts[shin].transform();
            let placed = self.parts[foot].transform();
            let local = parent.inverse() * placed;
            let height =
                (self.parts[shin].point(&self.legs[i].contact).y - self.sole - self.legs[i].floor)
                    * CM;
            let target = bend::foot_pitch_target(
                parent.matrix3.y_axis.y,
                parent.matrix3.z_axis.y,
                local.matrix3.z_axis.y,
                height,
                scale,
            );
            if let Some(leg) = self.legs.get_mut(i) {
                leg.pitch_height = height;
                leg.pitch = target;
                leg.pitch_rate = 0.0;
            }
        }
        self
    }

    /// Gives the body muscles following its animation. A living body never
    /// sleeps: its countdown is held at 256.
    pub fn with_drive(mut self) -> Self {
        for p in &mut self.parts {
            p.state = crate::part::ragdoll_state(p.state);
        }
        self.drive = Some(Drive::default());
        self.wake();
        self
    }

    /// The part that carries the support pins (the thorax, the first part
    /// whose setup has pins), if any. The thorax hunch pitches that part.
    pub fn support_pin_part(&self) -> Option<usize> {
        self.chest.first().map(|(i, _)| *i)
    }

    /// Sets both stored arm stiffnesses to `x`, as the original's recoil
    /// does when its stance returns: the next standing tick eases each
    /// back up from there.
    pub fn set_arm_stiffness(&mut self, x: f32) {
        if let Some(d) = self.drive.as_mut() {
            d.arm = x;
            d.arms = (x, x);
        }
    }

    /// The right arm's stiffness, or none for a limp body (the debug
    /// readout and the recoil's rest). Standing, the two arms ease apart
    /// and this is the right one.
    pub fn arm_stiffness(&self) -> Option<f32> {
        self.drive.as_ref().map(|d| d.arm)
    }

    /// Sets this tick's attack arms, or clears them. The muscle tune writes
    /// each side's shoulder, upper arm, forearm and hand from the pair;
    /// absent, those parts keep the eased stiffness.
    pub fn set_attack_arms(&mut self, tune: Option<AttackArmTune>) {
        if let Some(d) = self.drive.as_mut() {
            d.attack_arms = tune;
        }
    }

    /// Marks the part `root`'s chain as striking, or clears it: the
    /// original's strike flags on a weapon chain.
    pub fn set_striking(&mut self, root: usize, on: bool) {
        crate::part::set_striking(&mut self.parts, root, on);
    }

    /// Sets or clears only the strike-window bit on part `root` and its
    /// chain (the original's class bit 1, which the weapon impact branch
    /// reads), leaving the chain bit alone so the pair stage still measures
    /// weapon-body impacts.
    pub fn set_striking_window(&mut self, root: usize, on: bool) {
        if root >= self.parts.len() {
            return;
        }
        let mut stack = vec![root];
        while let Some(i) = stack.pop() {
            if i >= self.parts.len() {
                continue;
            }
            let p = &mut self.parts[i];
            p.state = if on {
                p.state.with(crate::part::PartState::STRIKING)
            } else {
                p.state.without(crate::part::PartState::STRIKING)
            };
            for (j, q) in self.parts.iter().enumerate() {
                if q.parent == Some(i) {
                    stack.push(j);
                }
            }
        }
    }

    /// Gives every part the held item's all-ones group mask, as the
    /// original does when a character arms itself: its own weapons never
    /// meet its own body (and, through [`Ragdoll::held_by`], no other held
    /// item does either).
    pub fn set_held_group(&mut self) {
        for part in &mut self.parts {
            part.pairing.group = crate::part::HELD_GROUP_MASK;
        }
    }

    /// Rebuilds the body's own group masks after it stops being held (or
    /// after a link change), as the original's group builder does.
    pub fn reset_groups(&mut self) {
        pairs::mask_groups(&mut self.parts);
    }

    /// Gives a living body its race's muscle factors.
    pub fn with_race(mut self, race: RaceMuscles) -> Self {
        if let Some(d) = self.drive.as_mut() {
            d.race = race;
            d.stance = race.stance;
        }
        self
    }

    /// Gives a living body what it takes from its own character's stats:
    /// its most energy, its limbs' scale, its head's gain and muscle factor
    /// and its walking arms.
    pub fn with_stats(mut self, stats: BodyStats) -> Self {
        if let Some(d) = self.drive.as_mut() {
            d.stats = stats;
            muscle::set_part_factors(&mut self.parts, stats.head_factor);
            for leg in &mut self.legs {
                if let Some(f) = leg.foot {
                    leg.toe = weights(&self.parts[f], feet::toe(self.sole, stats.scale));
                }
            }
        }
        self
    }

    /// Clears every foot contact's state, as a get-up's start does: no foot
    /// touches until it is planted again.
    pub fn clear_feet(&mut self) {
        for leg in &mut self.legs {
            leg.state = FootState::default();
        }
    }

    /// Registers a held item body's flags (two at most), as the original
    /// keeps them on the item: the walk's start clears bit 0x10000 and the
    /// armed move branch sets bit 0x80000 (static-only level queries).
    pub fn hold_item(&mut self, flags: u32) {
        if self.held_count < self.held_flags.len() {
            self.held_flags[self.held_count] = flags;
            self.held_count += 1;
        }
    }

    /// Releases the held items' flags.
    pub fn drop_items(&mut self) {
        self.held_count = 0;
    }

    /// The held item bodies' class flags.
    pub fn held_flags(&self) -> &[u32] {
        &self.held_flags[..self.held_count]
    }

    /// Writes the original's class word to every part, and `word | 0x30`
    /// to each part's first child (as the original's class writer does).
    pub fn set_class_word(&mut self, word: u32) {
        let first: Vec<Option<usize>> =
            (0..self.parts.len()).map(|i| self.first_child(i)).collect();
        for part in &mut self.parts {
            part.class_word = word;
        }
        for child in first.into_iter().flatten() {
            self.parts[child].class_word = word | 0x30;
        }
    }

    /// The death routine's lower-leg pass: for each lower leg (a part that
    /// carries a foot contact), its first child (the foot) gets the
    /// level-contact word 0 and loses the strike-chain bit. A part that is
    /// not a lower leg's first child keeps the bit. The same routine sets
    /// each leg limit link's strength to 1, which [`Ragdoll::go_limp`]
    /// does; the lower legs' collision layer is that routine's class
    /// writer, [`pairs::set_leg_layers`].
    pub fn clear_death_strikes(&mut self) {
        for leg in 0..self.parts.len() {
            if self.parts[leg].foot.is_none() {
                continue;
            }
            let Some(child) = self.first_child(leg) else {
                continue;
            };
            self.parts[child].no_level = false;
            self.parts[child].state.0 &= !crate::part::PartState::STRIKE_CHAIN;
        }
    }

    /// Stamps the impact-sound clock on the part of `bone`. A bone with no
    /// part leaves the body as it is.
    pub fn mark_part_sound(&mut self, bone: usize, ready_ms: u32) {
        if let Some(part) = self.parts.iter_mut().find(|p| p.bone == bone) {
            part.sound_ready_ms = ready_ms;
        }
    }

    /// Raises the grab layer's bit (0x40) on a part, its parent and its
    /// direct children, as the original's bite-grab does.
    pub fn raise_grab_layer(&mut self, index: usize) {
        let parent = self.parts.get(index).and_then(|p| p.parent);
        let children: Vec<usize> = self
            .parts
            .iter()
            .enumerate()
            .filter(|(_, p)| p.parent == Some(index))
            .map(|(i, _)| i)
            .collect();
        for i in parent
            .into_iter()
            .chain(std::iter::once(index))
            .chain(children)
        {
            if let Some(p) = self.parts.get_mut(i) {
                p.class_word |= GRAB_LAYER;
            }
        }
    }

    /// ORs `flag` into every part's controller state word, as the original
    /// walks the whole body tree (bit 4 for a tangle or a hit
    /// reaction).
    pub fn raise_state_flag(&mut self, flag: u32) {
        for p in &mut self.parts {
            p.state_word |= flag;
        }
    }

    /// The first child of part `i` in index order.
    fn first_child(&self, i: usize) -> Option<usize> {
        self.parts.iter().position(|p| p.parent == Some(i))
    }

    /// A walk's start (another clip playing while it locomotes) clears the
    /// held items' motion flag.
    pub fn walk_started(&mut self) {
        for flags in &mut self.held_flags[..self.held_count] {
            *flags &= !HELD_MOTION_FLAG;
        }
    }

    /// The move branch's static-only mode for the held items, which makes
    /// every level query static: set while armed and moving. Exarust keeps
    /// the held item bodies' word, not their queries yet.
    pub fn set_held_static_only(&mut self, on: bool) {
        for flags in &mut self.held_flags[..self.held_count] {
            if on {
                *flags |= HELD_STATIC_ONLY;
            } else {
                *flags &= !HELD_STATIC_ONLY;
            }
        }
    }

    /// One tick of the body tick's adapters, look rate and counters, from
    /// what the last tick measured.
    pub fn tick_timers(&mut self) {
        let Some(d) = self.drive.as_mut() else {
            return;
        };
        let input = timers::TimerInput {
            support: d.support,
            balance: d.balance.value,
            energy: d.energy,
            moving: d.bearing.moving,
            getting_up: d.bearing.getting_up,
            progress: d.bearing.progress,
        };
        d.timers.tick(&input);
    }

    /// Tells a living body what its character does this tick (before it is
    /// aimed).
    pub fn set_bearing(&mut self, bearing: Bearing) {
        if let Some(d) = self.drive.as_mut() {
            d.bearing = bearing;
        }
    }

    /// Starts every limp slack link's smoothed length at its points'
    /// distance, within the link's slack, as the original's death and
    /// placement reset does. Pivots and the weld and grab links are left
    /// alone.
    pub fn reset_link_lengths(&mut self) {
        for l in &mut self.links {
            if l.flags & (LINK_DISABLED | LINK_SOFT_WELD | LINK_BREAKABLE) != 0 {
                continue;
            }
            if l.flags & LINK_BODY == 0 {
                continue;
            }
            let d = self.parts[l.parent].point(&l.on_parent) - self.parts[l.part].point(&l.on_part);
            l.reset_length(d);
        }
    }

    /// Takes the body's muscles: it becomes a plain ragdoll, solved from
    /// the pelvis out. As the original's death does, its leg limit links
    /// pull at full strength again, each slack link's smoothed length
    /// starts where the link is, within its slack, so the joints neither
    /// jolt nor spring back, and its lower legs leave the living lower
    /// legs' layer (they meet other characters' again). It is awake with
    /// the living body's countdown (256) and sleeps by its speed from then
    /// on.
    pub fn go_limp(&mut self) {
        self.wake();
        self.become_limp();
    }

    /// The limp state itself: no muscles, the legs' limits at full
    /// strength, every slack link's smoothed length at its posed distance,
    /// the lower legs off the living layer, and each limb's limp state and
    /// mobility. The callers set the sleep countdown.
    fn become_limp(&mut self) {
        // What the muscles-off sampler needs, before the drive goes.
        if let Some(d) = self.drive.as_ref() {
            self.limp_frame = d.frame;
            self.limp_clip_rate = d.race.clip_rate;
            self.limp_height = d.lift_target.unwrap_or(0.0) * CM;
            self.limp_scale = d.stats.scale;
        }
        self.drive = None;
        pairs::set_leg_layers(&mut self.parts, 0);
        for p in &self.parts {
            if let Some(k) = p.leg_limit {
                self.links[k].strength = 1.0;
            }
        }
        self.reset_link_lengths();
        for p in &mut self.parts {
            p.state = crate::part::ragdoll_state(p.state);
            // The original gives every limp limb its own mobility back; the
            // part already carries it, so the restore is the identity.
            p.pairing.mobility = crate::part::ragdoll_mobility(p.pairing.mobility);
        }
    }

    /// Registers the body where its parts already are, limp, as the
    /// original's placed body is registered: every velocity zero and the
    /// sleep countdown at zero, so the body runs one tick and that tick's
    /// post-step puts it to sleep, with deep sleep from the next. It never
    /// wakes on its own.
    pub fn register_placed(&mut self) {
        self.become_limp();
        self.countdown = 0;
        self.sleep = Sleep::Awake;
        self.simulated = false;
        self.set_velocity(Vec3::ZERO);
        for p in &mut self.parts {
            p.settled = p.transform();
            p.shown = (p.settled, p.settled);
            p.drawn_particles = (p.positions, p.positions);
        }
    }

    /// Whether the body sleeps (and is not simulated).
    pub fn asleep(&self) -> bool {
        self.sleep != Sleep::Awake
    }

    /// The body's sleep countdown, in ticks: 256 woken or alive, 0 asleep.
    pub fn countdown(&self) -> i32 {
        self.countdown
    }

    /// Resets a body that re-enters the scene, as the original's body reset
    /// does: every velocity word, the stepping episode, the tone and energy
    /// to 1, the balance to 1, the blow sag and posture wait to 0.
    pub fn reset_presence(&mut self) {
        self.set_velocity(Vec3::ZERO);
        if let Some(d) = self.drive.as_mut() {
            d.tone = 1.0;
            d.energy = 1.0;
            d.balance.value = 1.0;
            d.steps = steps::Steps::default();
            d.sag = 0.0;
            d.posture_wait = 0;
            d.carry = Vec3::ZERO;
        }
    }

    /// Lets the body settle for `ticks` ticks with gravity but no inertia:
    /// the original's temporary state after a character is placed (a
    /// respawn or a level change), where each particle takes a tick's
    /// gravity and keeps none of its velocity for that long.
    pub fn settle_for(&mut self, ticks: i32) {
        self.settle = ticks.max(0);
    }

    /// Ticks still to run with gravity but no inertia.
    pub fn settling(&self) -> i32 {
        self.settle
    }

    /// Whether the body is placed (the original's state bit 0x1000): the
    /// original's integration keeps the bit only while the settling
    /// countdown runs and clears it once the countdown is spent, so the
    /// placed state is [`Ragdoll::settling`] above zero. A contact with
    /// another, unplaced body moves only the placed side.
    pub fn placed(&self) -> bool {
        self.settle > 0
    }

    /// Moves the whole body by `offset` (m), as the original's translate
    /// does: every particle set (current, previous, the tick's start) and
    /// the drawn transforms, contacts and foot anchors travel with it, so
    /// the move is a pure displacement with no velocity and no strain.
    pub fn translate(&mut self, offset: Vec3) {
        for p in &mut self.parts {
            for x in p
                .positions
                .iter_mut()
                .chain(p.previous.iter_mut())
                .chain(p.start.iter_mut())
            {
                *x += offset;
            }
            p.settled.translation += Vec3A::from(offset);
            p.shown.0.translation += Vec3A::from(offset);
            p.shown.1.translation += Vec3A::from(offset);
            for x in p
                .drawn_particles
                .0
                .iter_mut()
                .chain(p.drawn_particles.1.iter_mut())
            {
                *x += offset;
            }
            for c in &mut p.contacts {
                c.point += offset;
                c.plane = c.normal.dot(c.point) - CONTACT_SLOP;
            }
        }
        for leg in &mut self.legs {
            leg.anchor += offset;
            leg.at += offset;
            leg.before += offset;
        }
    }

    /// Moves each part's particles by `offset` times `weights` and, with
    /// `restore`, adds the offset's length to the part's strain and restores
    /// its edges past 0.1 cm: the original's weighted push with its strain
    /// flag.
    pub fn push_weighted(&mut self, offset: Vec3, weights: [f32; 4], restore: bool) {
        // The length as the original's vector length sums it: (x² + z²) + y².
        let length = ((offset.x * offset.x + offset.z * offset.z) + offset.y * offset.y).sqrt();
        for p in &mut self.parts {
            for (x, w) in p.positions.iter_mut().zip(weights) {
                *x = Vec3::new(x.x + w * offset.x, x.y + w * offset.y, x.z + w * offset.z);
            }
            if restore {
                p.strain += length;
                if p.strain > TRANSLATE_STRAIN {
                    keep_rigid(p);
                }
            }
        }
    }

    /// Places every part at a node's world transform, as the original's
    /// place-at-matrix does: its particles at the matrix times its rest
    /// points (summed `(x·X + y·Y) + (z·Z + T)` as the original's point
    /// transform does), with the previous and the tick's start points there
    /// too so it is at rest, and its drawn transforms there. Its contacts,
    /// strain and sleep countdown are left as they are, as the original
    /// leaves them; a caller that re-places a body wakes it.
    pub fn place_at(&mut self, worlds: &[Affine3A]) {
        for (p, world) in self.parts.iter_mut().zip(worlds) {
            let [x, y, z] = [world.x_axis, world.y_axis, world.z_axis].map(Vec3::from);
            let t = Vec3::from(world.translation);
            p.positions = p.rest.map(|r| (x * r.x + y * r.y) + (z * r.z + t));
            p.previous = p.positions;
            p.start = p.positions;
            p.settled = p.transform();
            p.shown = (p.settled, p.settled);
            p.drawn_particles = (p.positions, p.positions);
        }
    }

    /// Knocks the body out: it wakes at 256
    /// and its drive loses its tone and stance factor while the support
    /// pins fall back to the living gain (0.08). The character's condition,
    /// state bits and wake timer are `exa_combat::condition`'s.
    pub fn knock_out(&mut self) {
        self.wake();
        if let Some(d) = self.drive.as_mut() {
            d.tone = 0.0;
            d.stance = 0.0;
            d.pin_gain = PIN_GAIN;
        }
    }

    /// Wakes the body to 256 (see [`Ragdoll::wake_to`]).
    pub fn wake(&mut self) {
        self.wake_to(WAKE_COUNT);
    }

    /// Raises the sleep countdown to `count` if it is lower. A sleeping body
    /// wakes and is simulated from the next tick where it lies (from rest
    /// once its first sleeping tick has run), keeping the level contacts it
    /// had: it has not moved, so they still hold.
    pub fn wake_to(&mut self, count: i32) {
        if self.countdown >= count {
            return;
        }
        self.countdown = count;
        if self.asleep() {
            self.sleep = Sleep::Awake;
            for p in &mut self.parts {
                p.settled = p.transform();
                p.shown = (p.settled, p.settled);
                p.drawn_particles = (p.positions, p.positions);
            }
        }
    }

    /// A touch from a body whose countdown is `count` (a blow, which is the
    /// weapon's contact in the original): wakes the body as
    /// [`Ragdoll::wake_to`] does, but a sleeper also drops its stored level
    /// contacts and is tested afresh where it lies.
    pub fn touch(&mut self, count: i32) {
        if self.countdown < count && self.asleep() {
            for p in &mut self.parts {
                p.contacts.clear();
            }
        }
        self.wake_to(count);
    }

    /// Sets this tick's animated pose (the character tick does, every tick
    /// before the solver): the root's world transform, and each part's
    /// animated world transform, the root's times the clip's pose. A
    /// standing body is first carried along with the root's motion across
    /// the ground since the last aim (Exarust's own, its controller carries
    /// it). Then the character frame follows the pelvis by the original's
    /// rules, and the pose is put on it: every target moves across the
    /// ground by the frame's offset from the root. Only the targets'
    /// rotations are followed by the muscles: the pose drive turns parts
    /// about where they are, the link muscles turn joints; the lift reads
    /// the clip root's height over the ankles, the support pins the thorax
    /// pins' and the pelvis's place.
    pub fn aim(&mut self, root: Affine3A, targets: Vec<Affine3A>) {
        if self.drive.is_none() {
            return;
        }
        self.carry(root);
        if let Some(drive) = self.drive.as_mut() {
            drive.targets = targets;
            drive.refresh_turns();
        }
        self.anchor_frame(root);
        // The sampler folded last tick's bend into the pose
        // (`Ragdoll::fold_leg_ik`); its clip record goes in the world here,
        // after the carry (a pose given without a sample is folded here).
        self.pose_leg_bend(root);
        self.tune_muscles();
        self.lift();
    }

    /// Keeps last tick's pose, carried with the root, when this tick has
    /// none to give; its frame follows the pelvis as in [`Ragdoll::aim`].
    pub fn hold(&mut self, root: Affine3A) {
        self.carry(root);
        if let Some(drive) = self.drive.as_mut() {
            drive.refresh_turns();
        }
        let base = self.drive.as_ref().and_then(|d| d.frame).unwrap_or(root);
        self.anchor_frame(base);
        self.tune_muscles();
        self.lift();
    }

    /// The character's controller moved `moved` since the last aim to
    /// follow the body's frame (the character tick tells it before the
    /// aim): the carry leaves that move out, since the body has made it
    /// already.
    pub fn followed(&mut self, moved: Vec3) {
        if let Some(root) = self.drive.as_mut().and_then(|d| d.root.as_mut()) {
            root.translation += Vec3A::from(moved.with_y(0.0));
        }
    }

    /// Carries a standing body with its root's motion since the last aim,
    /// across the ground and about the vertical only (its height is the
    /// lift's): its particles, where the last tick left each part, its
    /// level contacts, its targets and its frame.
    fn carry(&mut self, root: Affine3A) {
        let Some(drive) = self.drive.as_mut() else {
            return;
        };
        if !drive.supported {
            drive.root = None;
        }
        let fresh = drive.head_parent_fresh;
        drive.head_parent_fresh = false;
        let Some(previous) = drive.root.replace(root) else {
            return;
        };
        // The root turns about the vertical, so dropping the rise keeps
        // every height.
        let mut carry = root * previous.inverse();
        carry.translation.y = 0.0;
        for p in &mut self.parts {
            for x in p.positions.iter_mut().chain(p.previous.iter_mut()) {
                *x = carry.transform_point3(*x);
            }
            // Where the last tick left it moves too, or the carry would
            // read as a sweep; its contacts travel with it.
            p.settled = carry * p.settled;
            for c in &mut p.contacts {
                c.point = carry.transform_point3(c.point);
                c.normal = carry.transform_vector3(c.normal).normalize_or(c.normal);
                c.plane = c.normal.dot(c.point) - CONTACT_SLOP;
            }
        }
        // The feet's anchors and points ride along (until the body walks
        // by its carry, C18).
        for leg in &mut self.legs {
            leg.anchor = carry.transform_point3(leg.anchor);
            leg.at = carry.transform_point3(leg.at);
            leg.before = carry.transform_point3(leg.before);
            // The animated contact is in the character frame, which the
            // carry moves: the feet's lag compares it with the physical
            // contact after this tick's walk.
            leg.anim_contact = carry.transform_point3(leg.anim_contact);
            leg.anim_hip = carry.transform_point3(leg.anim_hip);
        }
        for t in &mut drive.targets {
            *t = carry * *t;
        }
        if !fresh && let Some(parent) = drive.head_pose_parent.as_mut() {
            *parent = carry * *parent;
        }
        // The anchor is kept from tick to tick, so it is carried relative
        // to the root: a root that has not moved leaves it exactly where
        // it is (through the root's inverse, rounding far from the world's
        // origin would creep it).
        drive.anchor = drive.anchor.map(|a| {
            let at = root.translation + carry.matrix3 * (Vec3A::from(a) - previous.translation);
            Vec3::new(at.x, a.y, at.z)
        });
        drive.frame = drive.frame.map(|f| carry * f);
    }

    /// Loses the controller's support: the body is no longer carried and
    /// its balance is measured lying. Its tone is kept, as the original's
    /// fall keeps it: once it lies its support is 0, so the pose drive
    /// stops and its link muscles alone play the fall.
    pub fn fall(&mut self) {
        if let Some(d) = self.drive.as_mut() {
            d.supported = false;
            d.root = None;
        }
    }

    /// Stands again on the controller, from wherever the body lies.
    pub fn stand(&mut self) {
        if let Some(d) = self.drive.as_mut() {
            d.supported = true;
            d.root = None;
        }
    }

    /// Its character turned the body root to a new heading (toward a fall,
    /// or for a get-up), not its controller: the next aim takes the turn
    /// into the frame without carrying the body through it.
    pub fn turned(&mut self) {
        if let Some(d) = self.drive.as_mut() {
            d.root = None;
        }
    }

    /// The links of the part at `index`: the skeleton's, then its origin
    /// pivot.
    pub fn links_of(&self, index: usize) -> &[Link] {
        self.links_of
            .get(index)
            .map_or(&[], |r| &self.links[r.clone()])
    }

    /// Where a link's two points sit on their parts: in the part's own
    /// space and in its parent's (from the rest particles).
    pub fn link_points(&self, l: &Link) -> (Vec3, Vec3) {
        let at = |p: &Part, w: &[f32; 4]| (0..4).map(|k| p.rest[k] * w[k]).sum::<Vec3>();
        (
            at(&self.parts[l.part], &l.on_part),
            at(&self.parts[l.parent], &l.on_parent),
        )
    }

    /// The weights of a point `local` (m, in the part's space) over the
    /// part's rest particles: the original's way of turning a grip point
    /// into a link end. `None` when the part does not exist.
    pub fn weights_at(&self, part: usize, local: Vec3) -> Option<[f32; 4]> {
        self.parts.get(part).map(|p| weights(p, local))
    }

    /// Hands part `part` its collision mesh in the part's space: the
    /// triangles the pair stage tests the other side's spheres against,
    /// where the original queries a body's node mesh. Empty when the
    /// loader has no triangles for it, which leaves the sphere fallback.
    pub fn set_part_triangles(&mut self, part: usize, triangles: Vec<[Vec3; 3]>) {
        if let Some(p) = self.parts.get_mut(part) {
            p.triangles = Some(Arc::new(triangles));
        }
    }

    /// Part `part`'s collision mesh, in its space, when it has one.
    pub fn part_triangles(&self, part: usize) -> Option<&[[Vec3; 3]]> {
        self.parts
            .get(part)
            .and_then(|p| p.triangles.as_deref())
            .map(Vec::as_slice)
    }

    /// The pushes a placed body's contacts this tick wanted to move part
    /// `part` by (m, on its four particles), the original's contact offset array:
    /// each force added here was applied to the particles with the opposite
    /// sign, so the array is a record for the node controller, not a
    /// displacement. The original clears it in the integration while the
    /// body is placed and when it sleeps; Exarust does the same
    /// ([`Ragdoll::step`], [`Ragdoll::post_step`]).
    pub fn placed_offsets(&self, part: usize) -> Option<[Vec3; 4]> {
        self.parts.get(part).map(|p| p.placed_offset)
    }

    /// Every part's contact offset array ([`Ragdoll::placed_offsets`]).
    pub fn placed_offsets_all(&self) -> impl Iterator<Item = (usize, [Vec3; 4])> + '_ {
        self.parts
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.placed_offset))
    }

    /// Clears every part's contact offset array, as the original's
    /// integration does while the body is placed and sleep does.
    pub fn clear_placed_offsets(&mut self) {
        for p in &mut self.parts {
            p.placed_offset = [Vec3::ZERO; 4];
        }
    }

    /// The point with weights `w` in the part's rest pose (m). `None` when
    /// the part does not exist.
    pub fn rest_point_at(&self, part: usize, w: &[f32; 4]) -> Option<Vec3> {
        self.parts
            .get(part)
            .map(|p| (0..4).map(|k| p.rest[k] * w[k]).sum())
    }

    /// The point with weights `w` where the part is now (m).
    pub fn point_at(&self, part: usize, w: &[f32; 4]) -> Option<Vec3> {
        self.parts.get(part).map(|p| p.point(w))
    }

    /// Moves part `part` by `delta` (m) over the particles' weights `w`:
    /// one link end's push, exactly as the body's own links apply it.
    pub fn push_weights(&mut self, part: usize, w: &[f32; 4], delta: Vec3) {
        if let Some(p) = self.parts.get_mut(part) {
            p.push(w, delta);
        }
    }

    /// Where the part's own origin is now (m).
    pub fn origin_at(&self, part: usize) -> Option<Vec3> {
        self.parts.get(part).map(|p| p.point(&p.own_origin))
    }

    /// How far the part's own origin moved over the last tick (m): each
    /// particle's displacement times its origin weight.
    pub fn origin_moved(&self, part: usize) -> Option<Vec3> {
        self.parts
            .get(part)
            .map(|p| (0..4).map(|k| p.moved[k] * p.own_origin[k]).sum())
    }

    /// Moves each of the part's particles by `delta` (m) times its origin
    /// weight.
    pub fn push_origin(&mut self, part: usize, delta: Vec3) {
        if let Some(p) = self.parts.get_mut(part) {
            let w = p.own_origin;
            p.push(&w, delta);
        }
    }

    /// Turns the part's particles about `centre` (m) by `turn`.
    pub fn turn_part(&mut self, part: usize, centre: Vec3, turn: Mat3) {
        if let Some(p) = self.parts.get_mut(part) {
            for x in &mut p.positions {
                *x = turn * (*x - centre) + centre;
            }
        }
    }

    /// A part's link mobility: its inverse mass times the link share
    /// ([`LINK_MOBILITY`]), as the original keeps it on a living limb.
    pub fn link_mobility(&self, part: usize) -> Option<f32> {
        self.parts.get(part).map(|p| p.inverse_mass * LINK_MOBILITY)
    }

    /// The part of `bone`.
    pub fn part_of(&self, bone: usize) -> Option<&Part> {
        self.parts.iter().find(|p| p.bone == bone)
    }

    /// Centre of mass and its velocity (m/s).
    pub fn centre_of_mass(&self) -> (Vec3, Vec3) {
        let (mut mass, mut centre, mut velocity) = (0.0, Vec3::ZERO, Vec3::ZERO);
        for p in &self.parts {
            let m = 1.0 / p.inverse_mass;
            let here: Vec3 = p.positions.iter().copied().sum::<Vec3>() * 0.25;
            let before: Vec3 = p.previous.iter().copied().sum::<Vec3>() * 0.25;
            mass += m;
            centre += here * m;
            velocity += (here - before) * m;
        }
        if mass <= 0.0 {
            return (Vec3::ZERO, Vec3::ZERO);
        }
        (centre / mass, velocity / mass * BODY_HZ)
    }

    /// The summed limb motion of the last tick (cm), which the posture stage
    /// reads to tell a body lying still from one still falling: for each
    /// part, how far its farthest particle moved, summed over the parts.
    pub fn exertion(&self) -> f32 {
        self.parts
            .iter()
            .map(|p| {
                (0..4)
                    .map(|i| (p.positions[i] - p.previous[i]).length())
                    .fold(0.0, f32::max)
            })
            .sum::<f32>()
            * CM
    }

    /// A push at a world point of part `index`, as the original's
    /// push-at-point: the point's weights over the part's particles are
    /// found, how far that point moved this tick is weighed against the
    /// push by [`push_fraction`], and the scaled push is given to the
    /// particles by their raw weights. No strain is kept and nothing is
    /// restored here; the next tick's edge restore sees the stretch.
    pub fn push_at(&mut self, index: usize, world_point: Vec3, delta: Vec3) {
        let Some(part) = self.parts.get_mut(index) else {
            return;
        };
        let local = part.transform().inverse().transform_point3(world_point);
        let w = weights(part, local);
        let share = push_fraction(delta, part.point(&w) - part.point_at_start(&w));
        if share <= 0.0 {
            return;
        }
        part.push(&w, delta * share);
    }

    /// An effect's push on part `index` at its sphere `sphere` (a force
    /// barricade's): each particle moves by `delta` (m) times its weight of
    /// the sphere's centre, unclamped, the push counts toward the part's
    /// strain and past [`PUSH_STRAIN`] its edges are restored, as the
    /// original's push with its strain flag does. Nothing moves for a part
    /// or sphere the body lacks.
    pub fn push_sphere(&mut self, index: usize, sphere: usize, delta: Vec3) {
        let Some(part) = self.parts.get_mut(index) else {
            return;
        };
        let Some(&(centre, _)) = part.spheres.get(sphere) else {
            return;
        };
        let w = weights(part, centre);
        part.push(&w, delta);
        part.strain += delta.length();
        if part.strain > PUSH_STRAIN {
            keep_rigid(part);
        }
    }

    /// Applies a blow to the part of `bone`. It wakes the body as a touch
    /// from a living body (countdown 256) does.
    pub fn shove(&mut self, bone: usize, velocity: Vec3, daze: f32) {
        self.touch(WAKE_COUNT);
        let step = velocity / BODY_HZ;
        if let Some(p) = self.parts.iter_mut().find(|p| p.bone == bone) {
            for prev in &mut p.previous {
                *prev -= step;
            }
        }
        if let Some(d) = self.drive.as_mut() {
            d.energy = daze_energy(d.energy, daze);
        }
    }

    /// A blow on a limb sags a living body's stance by `sag` (the damage
    /// routine's min(2·I, 1)); it fades 0.01 a tick from the next step.
    pub fn sag_from_blow(&mut self, sag: f32) {
        if let Some(d) = self.drive.as_mut() {
            d.sag = sag;
        }
    }

    /// One footwork pass toward a collapse stagger's `target`. The feet
    /// keep the state they have. `spots` are the clip's ankles under
    /// `frame`. Returns the stance origin and where each foot is held.
    pub fn walk_stagger(
        &mut self,
        target: Vec3,
        frame: Vec3,
        spots: &[Option<Vec3>],
        scale: f32,
    ) -> feet::WalkedStep {
        let ankles: Vec<Vec3> = self
            .legs
            .iter()
            .map(|leg| self.parts[leg.shin].point(&leg.contact))
            .collect();
        let side = self
            .drive
            .as_ref()
            .and_then(|d| d.frame)
            .map_or(Vec3::X, |f| Vec3::from(f.matrix3.x_axis).with_y(0.0));
        let walked = feet::walk_stagger(&mut self.legs, &ankles, spots, target, frame, side, scale);
        if let Some(drive) = self.drive.as_mut() {
            drive.steps.stepping = true;
            drive.steps.target = target;
            drive.steps.knockback = false;
        }
        walked
    }

    /// Starts a knockback step at the chest plus `offset` (m, horizontal).
    pub fn begin_knockback(&mut self, offset: Vec3) {
        let Some(drive) = self.drive.as_mut() else {
            return;
        };
        let chest = drive.balance.chest().unwrap_or(Vec3::ZERO);
        steps::begin_knockback(&mut drive.steps, chest + offset);
    }

    /// Gives every particle a velocity (m/s) by placing its previous
    /// position behind it.
    pub fn set_velocity(&mut self, velocity: Vec3) {
        let back = velocity / BODY_HZ;
        for p in &mut self.parts {
            for i in 0..4 {
                p.previous[i] = p.positions[i] - back;
            }
        }
    }

    /// One tick of this body alone against the static `level`: the world's
    /// solver ([`solve`]) with one body, or a tick it sleeps through.
    pub fn tick<L: Level + Sync + ?Sized>(&mut self, level: &L) {
        if self.asleep() {
            self.sleep_tick();
        } else {
            solve(&mut [self], level);
        }
    }

    /// The integration step: the sleep countdown and a living body's pin
    /// gain and energy recovery (the original's character tick, before the
    /// body moves, so the pose drive and the link muscles read the same
    /// energy), inertia and gravity or the carry, then the pose drive's
    /// push; a living body's link muscles remember
    /// where the tick began.
    fn step(&mut self) {
        self.simulated = true;
        self.pass = 0;
        // The original's Verlet step zeroes a placed body's contact offset
        // array before the tick's detection writes this tick's contacts.
        if self.placed() {
            self.clear_placed_offsets();
        }
        self.tick_countdown();
        if let Some(drive) = self.drive.as_mut() {
            drive.pin_gain = recover_pins(drive.pin_gain, drive.bearing.collapsing);
            if drive.energy_recovered {
                drive.energy_recovered = false;
            } else {
                drive.energy = recover_energy(
                    drive.energy,
                    drive.stats.max_energy,
                    drive.bearing.getting_up,
                    drive.bearing.progress,
                );
            }
            let down = drive.bearing.off_feet || drive.bearing.collapsing;
            drive.sag = fade_sag(drive.sag, down);
        }
        self.integrate();
        if self.drive.is_some() {
            self.remember_anchors();
        }
        if let Some(drive) = self.drive.as_mut() {
            drive_parts(&mut self.parts, drive);
        }
    }

    /// One friction pass over the level contacts found this tick.
    fn rub(&mut self) {
        for p in &mut self.parts {
            if p.friction > 0.0 {
                p.rub_contacts();
            }
        }
    }

    /// This tick's sleep countdown, from the last tick's motion, before the
    /// body is integrated. A living body's is held at 256. A dormant statue
    /// or golem has its whole tree counted down by one, floored at 1,
    /// whatever its motion. A limp body counts down by the band of its
    /// fastest part ([`count_down`]).
    fn tick_countdown(&mut self) {
        if let Some(drive) = self.drive.as_ref() {
            if !drive.dormant {
                self.countdown = self.countdown.max(WAKE_COUNT);
            } else {
                self.countdown = (self.countdown - 1).max(SLEEP_COUNT);
            }
            return;
        }
        let band = self
            .parts
            .iter()
            .map(|p| {
                let moved = (0..4)
                    .map(|i| (p.positions[i] - p.previous[i]).length())
                    .fold(0.0, f32::max);
                speed_band(moved * CM)
            })
            .max()
            .unwrap_or(Speed::Resting);
        self.countdown = count_down(self.countdown, band);
    }

    /// The post-step: where the tick leaves each part, and sleep once the
    /// countdown is down to 1 (a living body's never is).
    fn post_step(&mut self) {
        // The scrape and roll sources follow their points through the tick
        // before the body's matrices are recorded, as the original does.
        self.step_scrapes();
        // The walk's travel phase, from the root's movement since the last
        // tick, before the parts settle.
        let moved2 = self
            .parts
            .iter()
            .find(|p| p.parent.is_none())
            .map_or(0.0, |p| {
                let moved = (p.positions[0] - p.previous[0]) * CM;
                moved.length_squared()
            });
        if let Some(drive) = self.drive.as_mut() {
            drive.travel_phase_before = drive.travel_phase;
            drive.travel_phase =
                travel_phase(drive.travel_phase, &mut drive.travel_phase_before, moved2);
        }
        for p in &mut self.parts {
            p.settled = p.transform();
            p.shown = (p.shown.1, p.settled);
            p.drawn_particles.0 = p.drawn_particles.1;
            p.drawn_particles.1 = p.positions;
        }
        if self.countdown <= SLEEP_COUNT {
            self.countdown = 0;
            self.sleep = Sleep::Asleep;
            // Sleep clears the placed body's contact offset array, and the
            // original's post-step empties its motion and smoothed buffers.
            self.clear_placed_offsets();
            for p in &mut self.parts {
                p.moved = [Vec3::ZERO; 4];
                p.smoothed = [Vec3::ZERO; 4];
            }
        }
        if let Some(&root) = self.outward.first()
            && self.parts[root].positions[0].y < FALL_OUT_HEIGHT
        {
            self.fell_out = true;
        }
        self.ticks += 1;
        // Next tick's bend. The pose this tick's muscles followed stays.
        self.bend_legs();
    }

    /// Whether the body's root part fell below [`FALL_OUT_HEIGHT`] (the
    /// original removes the body's node then).
    pub fn fell_out(&self) -> bool {
        self.fell_out
    }

    /// Adds a scrape or roll source at `point` in the body's root frame:
    /// its volume per unit of speed and its pitch per unit of speed over
    /// the source's base. The point starts where the body has it now, so
    /// the first tick measures no jump. The original makes one for each of
    /// the body's stored sound slots when it joins the world; Exarust's
    /// callers have no audio hookup yet, so nothing adds one in the game.
    pub fn add_scrape(&mut self, point: Vec3, volume: f32, pitch: f32) {
        self.add_scrape_on(0, point, volume, pitch);
    }

    /// Adds a scrape source on part `part` at `point` in that part's frame.
    pub fn add_scrape_on(&mut self, part: usize, point: Vec3, volume: f32, pitch: f32) {
        let world = self
            .parts
            .get(part)
            .map_or(point, |p| p.settled.transform_point3(point));
        self.scrapes
            .push(ScrapeSource::new(part, point, volume, pitch, world));
    }

    /// The body's scrape sources.
    pub fn scrapes(&self) -> &[ScrapeSource] {
        &self.scrapes
    }

    /// What the last post-step's scrape sources reported.
    pub fn scrape_events(&self) -> &[ScrapeEvent] {
        &self.scrape_events
    }

    /// One tick of every scrape source. The original runs this at the head
    /// of its post-step, before the tick's matrix is recorded, so the point
    /// is put in the world by the matrix the last tick left (the part's
    /// settled transform) and the playing source sits at that matrix's
    /// translation (each original body is one part here). Bodies without
    /// sources (all but a few shipped props) return at once.
    fn step_scrapes(&mut self) {
        self.scrape_events.clear();
        if self.scrapes.is_empty() {
            return;
        }
        let Some(root) = self.parts.first().map(|p| p.settled) else {
            return;
        };
        for source in &mut self.scrapes {
            let frame = self.parts.get(source.part).map_or(root, |p| p.settled);
            let world = frame.transform_point3(source.point);
            let origin = Vec3::from(frame.translation);
            self.scrape_events.push(source.step(world, origin));
        }
    }

    /// The level impacts this tick's detection found, one per part, for the
    /// sound and damage the original's impact call makes of them.
    pub fn level_impacts(&self) -> &[LevelImpact] {
        &self.level_impacts
    }

    /// A tick the body sleeps through. The first one takes its velocity
    /// away (its positions become its previous ones) and puts it in deep
    /// sleep, where nothing is done. It is drawn where it lies, its level
    /// contacts are neither tested nor enforced, and its countdown stays 0
    /// (the original counts down one left on a sleeper first; nothing in
    /// Exarust leaves one).
    fn sleep_tick(&mut self) {
        self.level_impacts.clear();
        if self.sleep == Sleep::Asleep {
            for p in &mut self.parts {
                p.previous = p.positions;
            }
            self.sleep = Sleep::Deep;
        }
        for p in &mut self.parts {
            p.shown.0 = p.shown.1;
            p.drawn_particles.0 = p.drawn_particles.1;
        }
    }

    fn integrate(&mut self) {
        let gravity = lift::gravity_per_tick();
        // While the settling countdown runs, particles take gravity with no
        // inertia and no retention (the original's temporary state).
        let settling = self.settle > 0;
        if settling {
            self.settle -= 1;
        }
        // A living part over half its tone falls by g(1 − a) + carry·a, a
        // its share of the carry; any other by gravity.
        let carried = self
            .drive
            .as_ref()
            .map(|d| (d.tone, d.energy, d.carry, d.stats.scale));
        // A limp part keeps its velocity by how fast it moved last tick; a
        // living limb keeps the fast retention times its race's factor,
        // whatever its speed.
        let living = self.drive.as_ref().map(|d| {
            let eased = d.bearing.walking || d.bearing.off_feet;
            let factor = retention_factor(
                d.balance.value,
                d.race.damping_ratio,
                eased,
                d.bearing.in_combat,
                d.bearing.attacking,
                d.stats.scale,
            );
            living_retention(factor, d.dormant)
        });
        for p in &mut self.parts {
            p.start = p.positions;
            p.strain = 0.0;
            let pull = carried
                .and_then(|(tone, energy, carry, scale)| {
                    let a = lift::carry_share(p.bone, tone, energy, scale)?;
                    Some(gravity * (1.0 - a) + carry * a)
                })
                .unwrap_or(gravity);
            if settling {
                // No velocity is carried or recorded: the next tick's
                // velocity is this tick's move, and the placed offset array
                // is zeroed, as the original's Verlet step does.
                p.moved = [Vec3::ZERO; 4];
                p.placed_offset = [Vec3::ZERO; 4];
                for i in 0..4 {
                    p.previous[i] = p.positions[i];
                    p.positions[i] += pull * p.gravity[i];
                }
                continue;
            }
            let keep = living.unwrap_or_else(|| {
                let moved = (0..4)
                    .map(|i| (p.positions[i] - p.previous[i]).length())
                    .fold(0.0, f32::max);
                limp_retention(moved * CM)
            });
            for i in 0..4 {
                let v = p.positions[i] - p.previous[i];
                p.moved[i] = v;
                // The original's smoothed velocity: a tenth of the way to
                // this tick's displacement.
                p.smoothed[i] += (v - p.smoothed[i]) * 0.1;
                p.previous[i] = p.positions[i];
                // Two adds, not one: the tick's kept motion first, then the
                // gravity displacement the original computes into its own
                // buffer and adds to the result.
                p.positions[i] += v * keep;
                p.positions[i] += pull * p.gravity[i];
            }
        }
    }

    /// A hung root's links to its node: unshared, the world does not move.
    /// A slider's correction is applied twice.
    fn pull_world_links(&mut self, i: usize) {
        let (world, parts) = (&mut self.world, &mut self.parts);
        for link in world.iter_mut().filter(|l| l.part == i) {
            if link.flags & LINK_DISABLED != 0
                || (link.flags & world_link::ITERATION_LIMIT != 0
                    && link.last_pass.is_some_and(|last| self.pass > last))
            {
                continue;
            }
            let Some(c) = link.correction(&parts[i]).filter(|c| c.is_finite()) else {
                continue;
            };
            parts[i].push(&link.on_part, c);
            if link.flags & world_link::SLIDER != 0 {
                parts[i].push(&link.on_part, c);
            }
        }
    }

    /// One constraint iteration over the body tree. A living body is solved
    /// children first, from its hands and feet in to its pelvis; a limp one
    /// parents first, from the pelvis out. Each part takes its links to its
    /// parent, then its stored level contacts, then its edges. In a living
    /// body each slack link with an anchor is also a link muscle. A link's
    /// correction is shared by the two parts' mobilities and spread over
    /// each part's particles by the point's weights, so the parts' momenta
    /// change by equal and opposite amounts.
    fn iterate(&mut self) {
        self.pull_support_pins();
        let gains = self.drive.as_ref().map(Drive::muscle_gains);
        let feet = self
            .drive
            .as_ref()
            .filter(|_| self.measures_balance())
            .map(feet::FootDrive::of);
        let mut landed = 0;
        for k in 0..self.parts.len() {
            let i = if gains.is_some() {
                self.inward[k]
            } else {
                self.outward[k]
            };
            for l in &mut self.links[self.links_of[i].clone()] {
                if l.flags & LINK_DISABLED != 0 {
                    continue;
                }
                if l.flags & LINK_ITERATION_LIMIT != 0
                    && l.iteration_limit.is_some_and(|limit| self.pass > limit)
                {
                    continue;
                }
                let (part, parent) = (&self.parts[l.part], &self.parts[l.parent]);
                let a = part.point(&l.on_part);
                let b = parent.point(&l.on_parent);
                let d = b - a;
                if l.limit {
                    // A horizontal limit link runs only in the first two
                    // iterations, and only a living body takes it.
                    if gains.is_none() || (l.horizontal && self.pass >= 2) {
                        continue;
                    }
                    if let Some(c) = l.limit_correction(d).filter(|c| c.is_finite()) {
                        self.parts[l.part].push(&l.on_part, c);
                    }
                    continue;
                }
                let correction = match (gains, l.anchor) {
                    (Some(gains), Some(anchor)) if l.slack > PIVOT_SLACK => {
                        l.muscle(part, parent, anchor, gains)
                    }
                    (Some(_), _) => l.living(d),
                    (None, _) if l.flags & LINK_SOFT_WELD != 0 => l.welded(
                        d,
                        (a - part.point_at_start(&l.on_part))
                            + (b - parent.point_at_start(&l.on_parent)),
                    ),
                    (None, _) => l.limp(d),
                };
                let Some(c) = correction.filter(|c| c.is_finite()) else {
                    continue;
                };
                let (own, other) = if l.flags & LINK_SOFT_WELD != 0 {
                    l.weld_shares()
                } else {
                    l.shares
                };
                self.parts[l.part].push(&l.on_part, c * own);
                self.parts[l.parent].push(&l.on_parent, -c * other);
            }
            self.pull_world_links(i);
            // The hand's grabs come last among a limp part's links.
            if gains.is_none() && !self.grabs.is_empty() {
                self.pull_grabs(i);
            }
            // A lower leg's foot contact, then a foot's sole. The target
            // rotations were taken once when the targets were installed.
            if let Some(fd) = &feet {
                for leg in &mut self.legs {
                    if leg.shin == i && feet::drive_contact(&mut self.parts[i], leg, fd) {
                        landed += 1;
                    }
                    if leg.foot == Some(i) {
                        feet::push_sole(&mut self.parts[i], leg, fd);
                        if let Some(turn) =
                            self.drive.as_ref().and_then(|d| d.turns.get(i)).copied()
                        {
                            feet::turn_sole(&mut self.parts[i], leg, fd, turn);
                        }
                    }
                }
            }
            let p = &mut self.parts[i];
            p.hold_contacts();
            keep_rigid(p);
        }
        if landed > 0
            && let Some(d) = self.drive.as_mut()
        {
            for _ in 0..landed {
                d.balance.land();
            }
        }
        self.pass += 1;
    }

    /// Repairs a part whose tetrahedron is inverted, as the original's
    /// post-iteration repair does. The inside-out state is the vector
    /// between the midpoints of two opposite edges (p0p1 and p2p3) leaning
    /// along the cross of those edges. The edges are restored, the part
    /// goes back to the shape its tick began with, moved by how far its box
    /// centre has come since, its body matrix is built from those points
    /// and the rest basis, each axis that leans against the cross of the
    /// other two (`dot < 0.1`, in the order y z, z x, x y) is re-formed as
    /// that cross made unit length, and the particles are put back on the
    /// rest tetrahedron under that matrix and the edges restored again. The
    /// previous points (the velocity) are left alone.
    ///
    /// The original tests the sign alone, its files' rest tetrahedra all
    /// having the same handedness; here the sign is compared with the rest
    /// tetrahedron's own, so a body built the other way round is not
    /// repaired every tick.
    fn repair_inverted(&mut self) {
        for p in &mut self.parts {
            let current = opposite_edges_lean(&p.positions);
            let rest = opposite_edges_lean(&p.rest);
            if !((current > 0.0 && rest < 0.0) || (current < 0.0 && rest > 0.0)) {
                continue;
            }
            keep_rigid(p);
            let centre = p.pairing.centre_weights();
            let shift = p.point(&centre) - p.point_at_start(&centre);
            for (x, start) in p.positions.iter_mut().zip(p.start) {
                *x = start + shift;
            }
            let m = affine_from_points(&p.rest, &p.rest_inverse, &p.positions);
            let mut axes = [m.matrix3.x_axis, m.matrix3.y_axis, m.matrix3.z_axis].map(Vec3::from);
            for (i, (a, b)) in [(1, 2), (2, 0), (0, 1)].into_iter().enumerate() {
                let cross = axes[a].cross(axes[b]);
                if dot_zyx(cross, axes[i]) < 0.1 {
                    axes[i] = cross.normalize_or_zero();
                }
            }
            let (m, translation) = (Mat3::from_cols(axes[0], axes[1], axes[2]), m.translation);
            p.positions = p.rest.map(|r| m * r + Vec3::from(translation));
            keep_rigid(p);
        }
    }

    /// Tests every sphere against the level: swept from where the last tick
    /// left it when it moved far enough, else where it is. Each part's hits
    /// this tick make up to one level impact ([`LevelImpact`]).
    /// A part's body mask for the level detection
    /// ([`crate::sector::body_mask`]): 1 on a part kept off the level, 4 on
    /// a living character's foot. Only a living body's feet carry the 4:
    /// the original clears it when the body collapses or dies, so a limp
    /// body's feet meet thin plates again.
    pub fn level_mask(&self, part: usize) -> u32 {
        let Some(p) = self.parts.get(part) else {
            return 0;
        };
        crate::sector::body_mask(p.no_level, self.drive.is_some() && p.pairing.out)
    }

    fn detect<L: Level + ?Sized>(&mut self, level: &L) {
        let mut pieces = std::mem::take(&mut self.pieces);
        let mut triangles = std::mem::take(&mut self.triangles);
        let mut impacts = std::mem::take(&mut self.level_impacts);
        impacts.clear();
        for p in &mut self.parts {
            p.level_hits = 0;
            p.level_point = Vec3::ZERO;
            p.level_push = Vec3::ZERO;
            p.level_travel = Vec3::ZERO;
            p.level_travel2 = 0.0;
        }
        let masks: Vec<u32> = (0..self.parts.len()).map(|k| self.level_mask(k)).collect();
        for (k, p) in self.parts.iter_mut().enumerate() {
            if p.no_level {
                continue;
            }
            let before = p.settled;
            let (mut lo, mut hi) = (Vec3::MAX, Vec3::MIN);
            let now = p.transform();
            for &(c, r) in &p.spheres {
                for x in [before.transform_point3(c), now.transform_point3(c)] {
                    lo = lo.min(x - Vec3::splat(2.0 * r));
                    hi = hi.max(x + Vec3::splat(2.0 * r));
                }
            }
            pieces.clear();
            level.pieces(lo, hi, &mut pieces);
            // A rigid body's stand-ins are the character capsules': bodies
            // meet the rigid body itself through their pairs. A piece whose
            // level mask shares a bit with the part's body mask is skipped
            // (a living foot ignores thin plates and flagged meshes).
            let own_mask = masks[k];
            pieces.retain(|&piece| {
                !level.movable(piece) && crate::sector::level_meets(own_mask, level.mask(piece))
            });
            for &piece in &pieces {
                for k in 0..p.spheres.len() {
                    let (centre, r) = p.spheres[k];
                    let start = before.transform_point3(centre);
                    let mut end = p.transform().transform_point3(centre);
                    let swept = !p.static_only && start.distance_squared(end) > SWEEP_TRAVEL2;
                    // Room for the pushes of earlier triangles.
                    let reach = Vec3::splat(2.0 * r);
                    let (lo, hi) = if swept {
                        (start.min(end) - reach, start.max(end) + reach)
                    } else {
                        (end - reach, end + reach)
                    };
                    triangles.clear();
                    level.triangles(piece, lo, hi, &mut triangles);
                    for t in &triangles {
                        let hit = if swept {
                            swept_hit(t, start, end, r)
                        } else {
                            sphere_hit(t, end, r)
                        };
                        if let Some((q, delta)) = hit
                            && p.hit(centre, q, delta)
                        {
                            end = p.transform().transform_point3(centre);
                        }
                    }
                }
            }
        }
        for (i, p) in self.parts.iter().enumerate() {
            if p.level_hits == 0 || !p.reacts {
                continue;
            }
            // The original scales the sum by the count's reciprocal.
            let point = p.level_point * (1.0 / p.level_hits as f32);
            if let Some(impact) =
                level_impact(i, point, p.level_push, p.level_travel, p.level_travel2)
            {
                impacts.push(impact);
            }
        }
        self.level_impacts = impacts;
        self.pieces = pieces;
        self.triangles = triangles;
    }
}

/// The parts in two orders over their tree, siblings in index order: parents
/// first, depth first, and children first.
fn tree_orders(parts: &[Part]) -> (Vec<usize>, Vec<usize>) {
    let n = parts.len();
    let parent = |i: usize| parts[i].parent.filter(|&j| j < n && j != i);
    let mut children = vec![Vec::new(); n];
    let mut roots = Vec::new();
    for i in 0..n {
        match parent(i) {
            Some(j) => children[j].push(i),
            None => roots.push(i),
        }
    }
    let (mut outward, mut inward) = (Vec::with_capacity(n), Vec::with_capacity(n));
    let mut seen = vec![false; n];
    // (part, children visited); a cycle in bad input is broken by `seen`.
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for start in roots.into_iter().chain(0..n) {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        outward.push(start);
        stack.push((start, 0));
        while let Some((i, next)) = stack.last_mut() {
            let i = *i;
            if let Some(&c) = children[i].get(*next) {
                *next += 1;
                if !seen[c] {
                    seen[c] = true;
                    outward.push(c);
                    stack.push((c, 0));
                }
            } else {
                inward.push(i);
                stack.pop();
            }
        }
    }
    (outward, inward)
}

/// One tick of the world's solver over `bodies` against the static `level`,
/// in the original's order, without solver stages ([`solve_with`]).
pub fn solve<L: Level + Sync + ?Sized>(bodies: &mut [&mut Ragdoll], level: &L) {
    solve_with(bodies, level, &mut []);
}

/// The wake of a contact between two bodies, as the original's pair
/// contacts do once a pair's contacts are found ([`Pairs`]): the body with
/// the lower sleep countdown takes the other's, as a touch
/// ([`Ragdoll::touch`]). So a living body (256) that touches a sleeper
/// wakes it for at least 1.5 s.
pub fn equalise_sleep(a: &mut Ragdoll, b: &mut Ragdoll) {
    let (ca, cb) = (a.countdown, b.countdown);
    if ca < cb {
        a.touch(cb);
    } else if cb < ca {
        b.touch(ca);
    }
}

/// A part's pose-drive gain as a share of K, by its bone: the pelvis, the
/// feet, the lumbar and the thorax 0.03, the head 0.01; the thighs, shins,
/// arms and hands have no pose drive.
fn pose_share(bone: usize) -> f32 {
    match bone {
        1 | 4 | 7 | 8 | 9 => TRUNK_GAIN,
        HEAD => HEAD_GAIN,
        _ => 0.0,
    }
}

/// The head's bone.
const HEAD: usize = 10;

/// One particle's pose push, in the original's cm and ticks: `offset` o
/// from the particle to its target, `moved` v its displacement over the
/// last tick, `stance` the stance factor s and `gain` G. The share of o
/// that v already closes, c = o·v/(s|o|²), is taken off: G·o·(1 − c), only
/// when c < 1 (a particle closing fast enough is left alone, one moving
/// away is pushed harder). On its feet the vertical is the double 0.1
/// narrowed once. Off its feet (`vertical_only`: down, getting up, dormant
/// or dead) the horizontal is dropped and the vertical is kept whole.
/// None under 1.19e-7 cm².
fn pose_push(
    offset: Vec3,
    moved: Vec3,
    stance: f32,
    gain: f32,
    vertical_only: bool,
) -> Option<Vec3> {
    let len2 = offset.length_squared();
    if len2.is_nan() || len2 < LINK_EPSILON2 * CM * CM {
        return None;
    }
    let c = offset.dot(moved) / len2 / stance;
    if c.is_nan() || c >= 1.0 {
        return None;
    }
    let mut push = offset * (1.0 - c);
    if vertical_only {
        push.x = 0.0;
        push.z = 0.0;
    } else {
        push.y = (f64::from(push.y) * VERTICAL_PUSH) as f32;
    }
    Some(push * gain)
}

/// The pose drive, after inertia. It turns the pelvis, the feet, the
/// lumbar, the thorax and the head toward their animated orientation about
/// where each part's origin is now: a particle's target is its rest vertex
/// under the part's animated rotation, placed at the part's origin measured
/// on its particles, so it has no position or height target. Each particle
/// is pushed by [`pose_push`] against its own velocity (in the frame its
/// controller carries, while one carries it), with the gain
/// [`Drive::pose_gain`]; only while the body has support. Off its feet the
/// push keeps only its vertical. The other parts are turned by their link
/// muscles alone.
fn drive_parts(parts: &mut [Part], drive: &Drive) {
    if drive.targets.len() == parts.len() && drive.balance.has_support() {
        let stance = drive.stance.max(muscle::MIN_STANCE);
        let vertical_only = drive.bearing.off_feet;
        for (p, target) in parts.iter_mut().zip(&drive.targets) {
            let gain = drive.pose_gain(p.bone);
            if gain.is_nan() || gain <= 0.0 {
                continue;
            }
            let (_, turn, _) = target.to_scale_rotation_translation();
            let origin = p.point(&p.own_origin);
            for k in 0..4 {
                let offset = origin + turn * p.rest[k] - p.positions[k];
                if let Some(push) =
                    pose_push(offset * CM, p.moved[k] * CM, stance, gain, vertical_only)
                {
                    p.positions[k] += push / CM;
                }
            }
        }
    }
}

/// The affine map taking `rest` to `points`, from the rest edges' inverse:
/// the original's body matrix built from the particles and their inverse
/// rest basis (it maps all four points exactly).
pub(crate) fn affine_from_points(
    rest: &[Vec3; 4],
    rest_inverse: &Mat3,
    points: &[Vec3; 4],
) -> Affine3A {
    let edges = Mat3::from_cols(
        points[1] - points[0],
        points[2] - points[0],
        points[3] - points[0],
    );
    let m = edges * *rest_inverse;
    Affine3A::from_mat3_translation(m, points[0] - m * rest[0])
}

/// The dot product summed as the original's inverted-tetrahedron test sums
/// it: z, then y, then x.
fn dot_zyx(a: Vec3, b: Vec3) -> f32 {
    a.z * b.z + a.y * b.y + a.x * b.x
}

/// How far the vector between the midpoints of the tetrahedron's edges p0p1
/// and p2p3 leans along those edges' cross product: positive when the
/// original takes the tetrahedron for inverted.
fn opposite_edges_lean(p: &[Vec3; 4]) -> f32 {
    let a = p[0] - p[1];
    let b = p[2] - p[3];
    let between = (p[1] + a * 0.5) - (p[3] + b * 0.5);
    dot_zyx(between, a.cross(b))
}

/// The length of a particle edge as the original's edge restore and its
/// distance helper build it: the squares summed x, then z, then y.
fn edge_length(v: Vec3) -> f32 {
    ((v.x * v.x + v.z * v.z) + v.y * v.y).sqrt()
}

/// Restores a part's six edge lengths.
fn keep_rigid(p: &mut Part) {
    const EDGES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    for (a, b) in EDGES {
        let rest = edge_length(p.rest[b] - p.rest[a]);
        let d = p.positions[b] - p.positions[a];
        let len = edge_length(d);
        if len <= 1e-6 {
            continue;
        }
        // (len − rest) / (len + len), times the gap.
        let c = d * ((len - rest) / (len + len));
        p.positions[a] += c;
        p.positions[b] -= c;
    }
    // As in the original, restoring the edges starts the strain afresh.
    p.strain = 0.0;
}

/// World transform of `entity` from the local transforms up its ancestry
/// (current, unlike the propagated global transforms: inside the fixed step
/// a character controller holds its simulated position).
pub fn world_of(
    entity: Entity,
    parents: &Query<&ChildOf>,
    transforms: &Query<&mut Transform>,
) -> Affine3A {
    world_with(entity, parents, |e| {
        transforms.get(e).ok().map(Transform::compute_affine)
    })
}

/// World transform of `entity` from the local transforms `local` gives up
/// its ancestry.
pub fn world_with(
    entity: Entity,
    parents: &Query<&ChildOf>,
    local: impl Fn(Entity) -> Option<Affine3A>,
) -> Affine3A {
    let mut world = Affine3A::IDENTITY;
    let mut node = Some(entity);
    while let Some(e) = node {
        if let Some(t) = local(e) {
            world = t * world;
        }
        node = parents.get(e).ok().map(ChildOf::parent);
    }
    world
}

/// Before a frame's ticks: queues the blows sent since the last frame, and
/// starts counting the frame's ticks afresh.
fn begin_frame(
    mut shoves: MessageReader<Shove>,
    mut queue: ResMut<ShoveQueue>,
    mut bodies: Query<&mut Ragdoll, Without<OutOfScene>>,
) {
    queue.0.extend(shoves.read().copied());
    for mut body in &mut bodies {
        if body.ticks != 0 {
            body.ticks = 0;
        }
    }
}

/// The level as body detection reads it: the physics world's pieces, their
/// masks from [`crate::StaticColliders`] when it is loaded.
struct MaskedLevel<'a> {
    world: &'a rapier3d::prelude::PhysicsWorld,
    masks: Option<&'a crate::StaticColliders>,
}

impl Level for MaskedLevel<'_> {
    fn pieces(&self, lo: Vec3, hi: Vec3, out: &mut Vec<u32>) {
        self.world.pieces(lo, hi, out);
    }

    fn triangles(&self, piece: u32, lo: Vec3, hi: Vec3, out: &mut Vec<[Vec3; 3]>) {
        self.world.triangles(piece, lo, hi, out);
    }

    fn mask(&self, piece: u32) -> u32 {
        self.masks
            .map_or(crate::sector::MASK_SMALL, |m| m.mask(piece))
    }

    fn movable(&self, piece: u32) -> bool {
        self.world.movable(piece)
    }

    fn floor_piece(&self, piece: u32) -> bool {
        self.world.floor_piece(piece)
    }
}

/// The solver's tick over every body (the character ticks ran before it):
/// the queued blows, then one tick of the world's solver over the awake
/// bodies, with the solver stages, which see the sleeping bodies too (a
/// contact wakes them). A sleeper is marked changed only once something
/// woke it.
pub fn step_bodies(
    physics: Res<Physics>,
    statics: Option<Res<crate::StaticColliders>>,
    mut queue: ResMut<ShoveQueue>,
    mut dealt: ResMut<DealtShoves>,
    mut clock: ResMut<BodyTicks>,
    mut stages: ResMut<SolverStages>,
    mut bodies: Query<&mut Ragdoll, Without<OutOfScene>>,
) {
    dealt.0.clear();
    for s in queue.0.drain(..) {
        if let Ok(mut body) = bodies.get_mut(s.body) {
            body.shove(s.bone, s.velocity, s.daze);
            if let Some(sag) = s.sag {
                body.sag_from_blow(sag);
            }
            if let Some(offset) = s.step {
                body.begin_knockback(offset);
            }
            dealt.0.push(s);
        }
    }
    let (mut awake, mut sleeping): (Vec<Mut<Ragdoll>>, Vec<Mut<Ragdoll>>) =
        (Vec::new(), Vec::new());
    for mut body in &mut bodies {
        match body.sleep {
            Sleep::Awake => awake.push(body),
            Sleep::Asleep => {
                body.sleep_tick();
                sleeping.push(body);
            }
            Sleep::Deep => sleeping.push(body),
        }
    }
    {
        let mut awake: Vec<&mut Ragdoll> = awake.iter_mut().map(|b| &mut **b).collect();
        // Untouched, so a deep sleeper is not marked changed.
        let mut sleeping: Vec<&mut Ragdoll> = sleeping
            .iter_mut()
            .map(|b| b.bypass_change_detection())
            .collect();
        // The level as body detection reads it: the world's pieces with the
        // static colliders' stored masks (a thin plate's 4 keeps a foot's
        // mask-4 part off it).
        let level = MaskedLevel {
            world: &physics.0,
            masks: statics.as_deref(),
        };
        solve_among(&mut awake, &mut sleeping, &level, stages.all());
    }
    for body in &mut sleeping {
        if body.sleep == Sleep::Awake {
            body.set_changed();
        }
    }
    clock.0 += 1;
}

/// Draws every body's parts between their last two ticks, by the fraction
/// of the next tick already elapsed: each part's local transform under its
/// drawn parent part, or under its node's parent as the frame left it (the
/// character controller is drawn between its steps after the fixed loop;
/// turns and teleports of the frame come after). Runs before transforms
/// propagate, so every part is drawn exactly at its interpolated place.
pub fn draw_bodies(
    time: Res<Time<Fixed>>,
    bodies: Query<Ref<Ragdoll>>,
    parents: Query<&ChildOf>,
    mut transforms: Query<&mut Transform>,
) {
    let alpha = time.overstep_fraction();
    for body in &bodies {
        // A deep sleeper stays where it was last drawn, and one registered
        // asleep where its nodes are: only awake bodies are drawn between
        // ticks, as the original's are.
        if body.sleep == Sleep::Deep && (!body.simulated || !body.is_changed()) {
            continue;
        }
        let worlds: Vec<Affine3A> = body.parts.iter().map(|p| p.drawn(alpha)).collect();
        for (i, p) in body.parts.iter().enumerate() {
            let parent = match p.parent {
                Some(j) => worlds[j].inverse(),
                None => match parents.get(p.entity) {
                    Ok(up) => world_of(up.parent(), &parents, &transforms).inverse(),
                    Err(_) => Affine3A::IDENTITY,
                },
            };
            if let Ok(mut t) = transforms.get_mut(p.entity) {
                *t = Transform::from_matrix((parent * worlds[i]).into());
            }
        }
    }
}

#[cfg(test)]
#[path = "body_floor_test.rs"]
mod floor_test;

#[cfg(test)]
#[path = "body_contact_test.rs"]
mod contact_test;

#[cfg(test)]
#[path = "body_tick_test.rs"]
mod tick_test;

#[cfg(test)]
#[path = "body_repair_test.rs"]
mod repair_test;

#[cfg(test)]
#[path = "body_joint_test.rs"]
mod joint_test;

#[cfg(test)]
#[path = "body_sleep_test.rs"]
mod sleep_test;

#[cfg(test)]
#[path = "body_drive_test.rs"]
mod drive_test;

#[cfg(test)]
#[path = "body_pins_test.rs"]
mod pins_test;

#[cfg(test)]
mod tests {
    use super::*;

    /// A square floor at height `y`, facing up.
    pub(super) fn floor_quad(y: f32, half: f32) -> Vec<[Vec3; 3]> {
        let at = |x: f32, z: f32| Vec3::new(x * half, y, z * half);
        vec![
            [at(-1.0, -1.0), at(-1.0, 1.0), at(1.0, 1.0)],
            [at(-1.0, -1.0), at(1.0, 1.0), at(1.0, -1.0)],
        ]
    }

    const NONE: &[[Vec3; 3]] = &[];

    /// A get-up's share is computed first and added to the base E + 0.005,
    /// all in double. E = 0.009986606 at progress 0.032463625 gives
    /// 0.024661969; the left-to-right sum rounds to 0.024661971. The 0.005
    /// is double, so 0.5 becomes the float 0.5049999952316284.
    #[test]
    fn the_get_up_energy_adds_its_share_to_the_base() {
        assert_eq!(
            recover_energy(0.009_986_606, 1.0, true, 0.032_463_625),
            0.024_661_969
        );
        assert_eq!(recover_energy(0.5, 1.0, false, 0.0), 0.504_999_99);
        assert_eq!(recover_energy(1.0, 1.0, true, 0.0), 1.0);
    }

    fn two_parts(slack: f32) -> Ragdoll {
        let e = Entity::PLACEHOLDER;
        let at = |y: f32| Affine3A::from_translation(Vec3::new(0.0, y, 0.0));
        Ragdoll::new(&[
            PartSetup {
                bone: 1,
                entity: e,
                parent: None,
                world: at(1.0),
                centre: Vec3::ZERO,
                half: Vec3::splat(0.1),
                origin: Vec3::ZERO,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            },
            // Pinned at its middle, 40 cm under the parent's; its top meets
            // the parent's bottom.
            PartSetup {
                bone: 2,
                entity: e,
                parent: Some(0),
                world: at(0.6),
                centre: Vec3::ZERO,
                half: Vec3::new(0.05, 0.2, 0.05),
                origin: Vec3::new(0.0, -0.4, 0.0),
                links: vec![(Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.0, 0.2, 0.0), slack)],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            },
        ])
    }

    /// The sole turns the solver reads are taken once when the targets are
    /// installed: each is its target's rotation, and a hold keeps them.
    #[test]
    fn the_sole_turns_are_taken_when_the_targets_are_installed() {
        let mut body = two_parts(0.0).with_drive();
        let pose = vec![
            Affine3A::from_rotation_y(0.3),
            Affine3A::from_rotation_x(-0.2),
        ];
        body.aim(Affine3A::IDENTITY, pose.clone());
        let turns = body.drive.as_ref().unwrap().turns.clone();
        assert_eq!(turns.len(), pose.len());
        for (turn, target) in turns.iter().zip(&pose) {
            assert_eq!(*turn, target.to_scale_rotation_translation().1);
        }
        body.hold(Affine3A::IDENTITY);
        assert_eq!(body.drive.as_ref().unwrap().turns, turns);
    }

    #[test]
    fn parts_fall_and_rest_on_the_ground() {
        let mut body = two_parts(0.0);
        let ground = floor_quad(0.0, 5.0);
        for _ in 0..400 {
            body.tick(ground.as_slice());
        }
        for p in &body.parts {
            let lowest = p.positions.iter().map(|v| v.y).fold(f32::MAX, f32::min);
            assert!(lowest > -0.15, "fell through: {lowest}");
            let t = p.transform();
            assert!(
                t.translation.y < 0.3,
                "still in the air: {}",
                t.translation.y
            );
        }
    }

    #[test]
    fn links_hold_parts_together_and_edges_keep_shape() {
        let mut body = two_parts(0.0);
        for _ in 0..50 {
            body.tick(NONE);
        }
        let l = &body.links[0];
        let a = body.parts[1].point(&l.on_part);
        let b = body.parts[0].point(&l.on_parent);
        assert!(a.distance(b) < 0.01, "link stretched {}", a.distance(b));
        let p = &body.parts[1];
        let rest = (p.rest[1] - p.rest[0]).length();
        let now = (p.positions[1] - p.positions[0]).length();
        assert!((rest - now).abs() < 0.01);
    }

    /// A link whose points lie unevenly inside their tetrahedra (the
    /// parent's outside it) moves the two parts by equal and opposite
    /// momenta, so the constraints alone never set a body moving.
    #[test]
    fn links_keep_the_bodys_momentum() {
        // A pivot, and a slack link damping inside its slack.
        for slack in [0.0, 0.3] {
            let mut body = two_parts(slack);
            body.links[0].length = 0.1;
            let l = &body.links[0];
            let spread = |w: &[f32; 4]| w.iter().map(|x| x * x).sum::<f32>();
            assert!((spread(&l.on_part) - spread(&l.on_parent)).abs() > 0.2);
            // Pull the lower part off its joint, twisted.
            let twist = Affine3A::from_rotation_translation(
                Quat::from_rotation_z(0.4),
                Vec3::new(0.15, -0.05, 0.1),
            );
            for x in &mut body.parts[1].positions {
                *x = twist.transform_point3(*x);
            }
            let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
            for _ in 0..LINK_ITERATIONS {
                body.iterate();
            }
            // Each particle carries its part's mass.
            let momentum: Vec3 = body
                .parts
                .iter()
                .zip(&before)
                .flat_map(|(p, b)| (0..4).map(move |i| (p.positions[i] - b[i]) / p.inverse_mass))
                .sum();
            assert!(
                momentum.length() < 1e-4,
                "constraints moved the body by {momentum} (slack {slack})"
            );
        }
    }

    /// Velocity retention as the original's default environment gives it:
    /// 0.426, 0.25 and 0.05 per second, 0.98977, 0.98344 and 0.96455 per
    /// tick. A limp part takes its band from the farthest one of its
    /// particles moved last tick, strictly over 40/83 cm (fast) or 20/83 cm
    /// (slow); a living limb keeps the fast retention times its factor,
    /// 0.98977 for a human, whatever its speed. A golem (r = 0.95) at full
    /// balance keeps r² standing and 0.5·r + 0.5 while walking or down; in
    /// combat it keeps u, or 0.97·u² in double when it is larger than 1.3
    /// and not attacking. A dormant body takes 0.95 of the fast retention
    /// in double first.
    #[test]
    fn velocity_is_kept_as_the_default_environment_gives_it() {
        let close = |a: f32, b: f32| (a - b).abs() < 5e-6;
        let [fast, slow, resting] = RETENTION.map(per_tick);
        assert!(close(fast, 0.98977) && close(slow, 0.98344) && close(resting, 0.96455));
        assert_eq!(per_tick(0.0), 0.0);
        for (moved, want) in [
            (0.49, fast),
            (40.0 / BODY_HZ, slow),
            (0.25, slow),
            (20.0 / BODY_HZ, resting),
            (0.0, resting),
        ] {
            assert_eq!(limp_retention(moved), want, "{moved} cm in a tick");
        }
        assert!(close(living_retention(1.0, false), 0.98977));
        assert!(close(living_retention(1.0, true), 0.98977 * 0.95));
        // The keep is the double 0.95, not the float widened.
        assert_eq!(
            living_retention(1.0, true),
            (f64::from(per_tick(RETENTION[0])) * 0.95) as f32
        );
        assert_eq!(living_retention(0.9025, true), 0.848_605_5);
        // The fast value takes 0.95 in double and narrows before the
        // factor: 0.00005 gives 0.000047014153, not 0.000047014157.
        assert_eq!(living_retention(0.000_05, true), 0.000_047_014_153);
        let golem = 0.95f32;
        let still = |b, eased, combat, attacking, scale| {
            retention_factor(b, golem, eased, combat, attacking, scale)
        };
        let standing = still(1.0, false, false, false, 1.0);
        let walking = still(1.0, true, false, false, 1.0);
        assert_eq!(standing.to_bits(), (golem * golem).to_bits());
        assert_eq!(walking.to_bits(), (golem * 0.5 + 0.5).to_bits());
        // In combat, standing, u is 0.975: u, or 0.97·u² on a large body
        // that is not attacking, and the eased branch first.
        assert_eq!(still(0.5, false, true, false, 1.0), 0.975);
        assert_eq!(still(0.5, false, true, false, 1.4), 0.922_106_3);
        assert_eq!(still(0.5, false, true, true, 1.4), 0.975);
        assert_eq!(still(0.5, true, true, false, 1.4), 0.987_5);
        // The scale test is strict: 1.3 is not over.
        assert_eq!(still(0.5, false, true, false, 1.3), 0.975);
        // Lost balance makes u 1, so both factors are 1.
        assert_eq!(still(0.0, false, false, false, 1.0), 1.0);
        assert_eq!(still(0.0, true, false, false, 1.0), 1.0);
        // A human's ratio of 1 leaves the factor at 1, walking or not.
        assert_eq!(retention_factor(0.4, 1.0, false, false, false, 1.0), 1.0);
        assert_eq!(retention_factor(0.4, 1.0, true, false, false, 1.0), 1.0);
        assert!(retention_factor(f32::NAN, golem, false, false, false, 1.0).is_nan());
        assert!(retention_factor(f32::NAN, golem, true, false, false, 1.0).is_nan());
        assert!(retention_factor(1.0, f32::NAN, false, false, false, 1.0).is_nan());
        let mut awake = Ragdoll::new(&[]).with_drive();
        awake.countdown = 10;
        awake.tick_countdown();
        assert_eq!(awake.countdown(), WAKE_COUNT);
        let mut statue = Ragdoll::new(&[]).with_drive();
        statue.drive.as_mut().unwrap().dormant = true;
        statue.countdown = 10;
        statue.tick_countdown();
        assert_eq!(statue.countdown(), 9);
        // One tick of a lone part drifting sideways at 6 cm/s (resting) and
        // at 60 cm/s (fast), limp and living (held up by its tone).
        for (speed, living, want) in [
            (0.06, false, resting),
            (0.6, false, fast),
            (0.06, true, fast),
        ] {
            let mut body = Ragdoll::new(&[PartSetup {
                bone: 11,
                entity: Entity::PLACEHOLDER,
                parent: None,
                world: Affine3A::IDENTITY,
                centre: Vec3::ZERO,
                half: Vec3::splat(0.06),
                origin: Vec3::ZERO,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            }]);
            if living {
                body = body.with_drive();
            }
            body.set_velocity(Vec3::X * speed);
            let x = body.parts[0].positions[0].x;
            body.integrate();
            let kept = (body.parts[0].positions[0].x - x) / (speed / BODY_HZ);
            assert!(
                (kept - want).abs() < 1e-4,
                "{speed} m/s, living {living}: kept {kept}"
            );
        }
        // A golem (r = 0.95) at full balance keeps that fast share times r²
        // standing, and times 0.5·r + 0.5 while walking, even when slow.
        let mut golem_body = Ragdoll::new(&[PartSetup {
            bone: 11,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.06),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        }])
        .with_drive();
        golem_body.drive.as_mut().unwrap().race.damping_ratio = golem;
        let expect = |body: &mut Ragdoll, eased: bool| {
            body.drive.as_mut().unwrap().bearing.walking = eased;
            body.set_velocity(Vec3::X * 0.06);
            let x = body.parts[0].positions[0].x;
            body.integrate();
            (body.parts[0].positions[0].x - x) / (0.06 / BODY_HZ)
        };
        let standing_kept = expect(&mut golem_body, false);
        assert!(
            (standing_kept - fast * golem * golem).abs() < 1e-4,
            "standing golem kept {standing_kept}"
        );
        let walking_kept = expect(&mut golem_body, true);
        assert!(
            (walking_kept - fast * (golem * 0.5 + 0.5)).abs() < 1e-4,
            "walking golem kept {walking_kept}"
        );
    }

    /// A legless body has no lift: it lies on the floor while its pose
    /// moves, and stays there.
    #[test]
    fn driven_body_follows_its_pose() {
        let mut body = two_parts(0.0).with_drive();
        let floor = floor_quad(0.0, 5.0);
        let targets = |z: f32| {
            let r = Quat::from_rotation_y(z * 10.0);
            vec![
                Affine3A::from_rotation_translation(r, Vec3::new(0.0, 1.0, z)),
                Affine3A::from_rotation_translation(r, Vec3::new(0.0, 0.6, z)),
            ]
        };
        for tick in 0..415 {
            // The clip's clock at 60 frames per second, sampled every tick.
            let frame = (tick as f32 * 60.0 / BODY_HZ) as usize;
            let z = (frame % 40) as f32 * 0.02;
            body.aim(Affine3A::IDENTITY, targets(z));
            body.tick(floor.as_slice());
            let c = body.parts[0].transform().translation;
            assert!(c.is_finite() && c.length() < 3.0, "tick {tick}: {c}");
        }
    }

    /// A two-part body has no feet, so the support pins do not run and
    /// nothing pulls a shove back to the origin. Its energy still recovers.
    #[test]
    fn a_footless_shove_is_not_pulled_back() {
        let mut body = two_parts(0.0).with_drive();
        let pose = vec![
            Affine3A::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            Affine3A::from_translation(Vec3::new(0.0, 0.6, 0.0)),
        ];
        let step = |body: &mut Ragdoll, ticks: usize| {
            for _ in 0..ticks {
                body.aim(Affine3A::IDENTITY, pose.clone());
                body.tick(NONE);
            }
            body.parts[0].transform().translation.x
        };
        step(&mut body, 20);
        body.shove(1, Vec3::X * 3.0, 0.7);
        // The blow's energy is the original's product, not a clamped share.
        assert_eq!(body.drive.as_ref().unwrap().energy, 1.0 - 0.7);
        let pushed = step(&mut body, 10);
        assert!(pushed > 0.05, "barely moved: {pushed}");
        let back = step(&mut body, 400);
        // No feet, so no support pins, and the pelvis spring is gone: the
        // shove is not pulled back to the origin. The energy still recovers.
        assert!(back.abs() > 0.2, "pulled back to {back}");
        assert_eq!(body.drive.as_ref().unwrap().energy, 1.0);
    }

    /// A blow's energy loss is `energy − energy × daze`. A daze above the
    /// double 0.9 becomes 0.9, and a result below the double 0.05 becomes
    /// 0.05. A NaN stays, and a negative daze is not raised to 0.
    #[test]
    fn a_blow_leaves_energy_as_the_original_does() {
        assert_eq!(daze_energy(1.0, 0.5), 0.5);
        assert_eq!(daze_energy(1.0, 0.9), 1.0 - 0.9);
        assert_eq!(daze_energy(1.0, 0.9f32.next_up()), 1.0 - 0.9);
        assert_eq!(daze_energy(0.2, 0.9), MIN_ENERGY);
        assert_eq!(daze_energy(0.04, 0.0), MIN_ENERGY);
        assert_eq!(daze_energy(MIN_ENERGY, 0.0).to_bits(), MIN_ENERGY.to_bits());
        assert!(daze_energy(f32::NAN, 0.5).is_nan());
        assert!(daze_energy(0.8, f32::NAN).is_nan());
        #[allow(clippy::neg_multiply)]
        {
            assert_eq!(daze_energy(0.5, -1.0), 0.5 - 0.5 * -1.0);
        }
        // The product is a multiply then a subtract, so a second blow on
        // what the first left is not the single-precision complement.
        let energy = f32::from_bits(0x3f26_cecc);
        let daze = f32::from_bits(0x3f35_b8cc);
        let once = daze_energy(energy, daze);
        assert_eq!(once.to_bits(), 0x3e41_98b0);
        assert_ne!(once, energy * (1.0 - daze));
    }

    /// A level impact needs a smoothed travel squared over 0.1 cm² and an
    /// impact strength over 0.1: 1 cm a tick into a push straight against
    /// it is the whole square, and equal values do not pass either gate.
    #[test]
    fn level_impacts_take_the_original_gates() {
        let up = Vec3::new(0.0, 0.01, 0.0);
        let down = Vec3::new(0.0, -1.0, 0.0);
        let impact = level_impact(3, Vec3::ZERO, up, down, 1.0).unwrap();
        assert_eq!(impact.part, 3);
        assert_eq!(impact.strength, 1.0);
        assert_eq!(level_impact(0, Vec3::ZERO, up, down, 0.1), None);
        // A travel square of 0.09 is under the travel gate.
        assert_eq!(level_impact(0, Vec3::ZERO, up, down, 0.09), None);
        // No push gives the original's zero cosine, hence no strength.
        assert_eq!(level_impact(0, Vec3::ZERO, Vec3::ZERO, down, 1.0), None);
        // A push along the travel gives a negative cosine, hence none.
        assert_eq!(level_impact(0, Vec3::ZERO, -up, down, 1.0), None);
    }

    /// A ball dropped onto a floor makes a level impact once it falls fast
    /// enough for the smoothed velocity (a tenth of the way each tick) to
    /// pass the gates, and nothing on its resting ticks.
    #[test]
    fn a_falling_body_makes_a_level_impact() {
        let ground = floor_quad(0.0, 5.0);
        let e = Entity::PLACEHOLDER;
        // One cube a metre over the floor.
        let mut body = Ragdoll::new(&[PartSetup {
            bone: 1,
            entity: e,
            parent: None,
            world: Affine3A::from_translation(Vec3::Y),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.15),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        }]);
        let mut impact = None;
        for _ in 0..60 {
            body.tick(ground.as_slice());
            if let Some(&i) = body.level_impacts().first() {
                impact = Some(i);
                break;
            }
        }
        let impact = impact.expect("no level impact on landing");
        assert!(impact.strength > 0.1);
        // Once it lies still, no further impact is made.
        for _ in 0..300 {
            body.tick(ground.as_slice());
        }
        for _ in 0..20 {
            body.tick(ground.as_slice());
            assert!(
                body.level_impacts().is_empty(),
                "{:?}",
                body.level_impacts()
            );
        }
    }
}

/// A physics environment's bands, as the original's environment records
/// store them: velocity retention a second at the fast, medium and slow
/// speeds, the speeds themselves (cm/s), and gravity (cm/s²).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    pub fast: f32,
    pub medium: f32,
    pub slow: f32,
    pub gravity_y: f32,
    pub fast_speed: f32,
    pub slow_speed: f32,
}

/// The "motile" environment the class init builds from the default's bands
/// × 0.6 (retention 0.426/0.25/0.05 → 0.2556/0.15/0.03), gravity
/// −980.665, thresholds 40 and 20 cm/s. No body ever uses it (the shipped
/// code never assigns it), so it is data only.
pub const MOTILE_ENVIRONMENT: Environment = Environment {
    fast: 0.426 * 0.6,
    medium: 0.25 * 0.6,
    slow: 0.05 * 0.6,
    gravity_y: -980.665,
    fast_speed: 40.0,
    slow_speed: 20.0,
};

#[cfg(test)]
mod environment_tests {
    use super::*;

    #[test]
    fn the_motile_environment_is_the_defaults_times_six_tenths() {
        assert!((MOTILE_ENVIRONMENT.fast - 0.2556).abs() < 1e-6);
        assert!((MOTILE_ENVIRONMENT.medium - 0.15).abs() < 1e-6);
        assert!((MOTILE_ENVIRONMENT.slow - 0.03).abs() < 1e-6);
        assert_eq!(MOTILE_ENVIRONMENT.gravity_y, -980.665);
        assert_eq!(
            (MOTILE_ENVIRONMENT.fast_speed, MOTILE_ENVIRONMENT.slow_speed),
            (40.0, 20.0)
        );
    }
}
