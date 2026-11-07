//! The force blast and bolt in the running game: the effect entity a cast
//! makes, ticked against its caster's body and the scene's other bodies.
//!
//! - **Start** ([`start_blast`]): the blast's start plays cast motion 0x22
//!   with the setup's power id; when that motion starts, the caster's cast
//!   hook becomes the blast's, which answers −1 (it never keeps another cast
//!   from starting). A refused motion leaves the caster without a hook, and
//!   the blast still charges.
//! - **The hook** ([`hook_called`]): any call of the hook while the blast
//!   charges makes it fizzle. A later cast's start calls it (it replaces the
//!   hook and the power id), and so do the player's buttons and the body
//!   tick's release ([`casting::player_hooks`], [`casting::release_cast`]),
//!   which end the cast.
//! - **Each tick** (after the body tick): the blast reads its view from the
//!   caster's right hand, its facing and its aim point (the AI's target's
//!   chest, else the player's cursor), pays its drain from the caster's
//!   focus and ticks ([`Blast::tick`]); a thrown blast strikes the other
//!   bodies' limbs and leaves the scene when [`BlastEvents::removed`] says so.
//! - **Contacts** (after the tick, in the sector's order: the level, the
//!   simulated objects, the bodies' parts; each contact leaves its reach
//!   scale for the next): a thrown blast meets the level ([`strike_level`]),
//!   stirs the level's cloths ([`stir_cloth`]: [`hit_sim_object`] over the
//!   particles in the cloth's frame, the cloth woken to 256 ticks and its
//!   rustle envelope taking the stir), and meets each other body part by
//!   part ([`strike_part`]): the chunk taken into the part's frame, its
//!   reach `reach·scale + margin + 10` tested against the node's sphere and
//!   box, the node mesh asked its sphere query with the chunk pushed out of
//!   every face it reaches ([`push_out`]), and the pushed chunk handed back
//!   to the limb callback ([`limb_push`] with the part's velocity at the
//!   chunk's particle weights, the impulse pushed at those weights with the
//!   strain, the chunk's weight loss). A part met wakes its body to 256
//!   ticks; a character's part also takes [`character_knock`] of that
//!   part's gathered impulse: the energy, a knockback step, its instability
//!   counter +1, concern 1 at 0.01 toward the caster and the loosened pins.
//!   [inf: a part built without a node mesh answers with its collision
//!   spheres; the sector's offer of nearby parts is a sphere test against
//!   the blast's bounds; the knock is a knockback step.]
//! - **Release and level**: the release alerts the caster's known
//!   characters ([`alert_amount`]); thrown chunks meet the level as a swept
//!   ball ([`rebound`], or the bolt's sampled contact); the thrown ribbon
//!   draws with the glow stand-in ([`draw_blasts`]).
//!
//! - **The struck body** ([`loosen_pins`]): after each part's knock, the
//!   body's support-pin gain becomes the knock's `(1 − k)·0.08` (0.008 past
//!   0.08, 0 below 0) and its balance's landing momentum takes its full
//!   weight 5.
//! - **The light** (`super::effect_light`): the blast registers its GI
//!   multi-point shape when it starts and frees it when its frame builds
//!   nothing or it leaves the scene.
//!
//! Not wired: the sounds. Another motion
//! replacing the cast motion calls the hook (`cast::call_motion_hooks`),
//! which fizzles a charging blast.
//!
//! Evidence: the blast's cast, creator, start and hook, the cast-motion
//! start, the body tick's release, the spend, the cast end and the
//! character hit, read from `Exanima.exe`.

// NaN-sensitive compares are kept as the original makes them.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::prelude::*;
use exa_ai::brain::Npc;
use exa_ai::combatant::Combatant;
use exa_physics::Physics;
use exa_physics::body::locomotion::Locomotor;
use exa_physics::body::{Drive, Ragdoll};

use super::cast::PowerCaster;
use super::casting::{self, CastBody, CastHook};
use super::force::{BLAST_MOTION, BlastSetup};
use super::live::{EffectDraws, EffectMesh, mesh_of_ribbon};
use crate::effects::Frame as ViewFrame;
use crate::effects::blast::{
    ALERT_GAIN, Blast, BlastChunk, BlastView, CHARGING, CONTACT_WEAR, CharacterKnock,
    HIT_HOSTILITY, HIT_LANDING_WEIGHT, STATIC_EXTRA, SimPoints, THROWN, alert_amount,
    character_knock, contact_reach, contact_weight, hit_sim_object, limb_push, limb_terms,
    push_out, rebound,
};
use crate::player::CursorGround;

/// Centimetres a metre.
const CM: f32 = 100.0;
const CHEST: usize = 9;
const RIGHT_HAND: usize = 17;
/// The blast's hook: −1 to every event.
pub const BLAST_HOOK: CastHook = CastHook::NEVER_KEEPS;
/// The extra reach a limb is tested with.
const LIMB_EXTRA: f32 = 10.0;
/// The aim point a caster with no target and no cursor looks at, cm ahead.
const AHEAD: f32 = 600.0;

/// A blast in the scene.
#[derive(Component, Clone, Debug)]
pub struct LiveBlast {
    pub blast: Blast,
    pub caster: Entity,
    /// The body the AI aims at; the player aims at the cursor.
    pub target: Option<Entity>,
    /// The caster's cast hook is this blast's.
    pub hooked: bool,
    /// The cast generation whose hook the blast took.
    pub generation: u64,
    /// The reach scale the blast's last contact set (0 before any): the
    /// simulated objects' walk reads it.
    pub contact_scale: f32,
}

/// The blast's start on its caster: cast motion 0x22 with the setup's power
/// id, and the hook when the motion starts.
pub fn start_blast(body: &mut CastBody, setup: &BlastSetup) -> bool {
    let started = casting::start_cast(body, BLAST_MOTION, setup.power);
    if started {
        body.cast_hook = Some(BLAST_HOOK);
    }
    started
}

/// Whether the caster called the blast's hook since the last tick: the hook
/// is no longer the blast's, since every call of it answers −1 and so ends
/// the cast or a later cast's start replaced it.
pub fn hook_called(live: &mut LiveBlast, body: &mut CastBody) -> bool {
    if !live.hooked {
        return false;
    }
    if body.cast_generation != live.generation
        || body.casting != live.blast.setup.power
        || body.cast_hook != Some(BLAST_HOOK)
    {
        live.hooked = false;
        return true;
    }
    false
}

/// The world place (cm) of a point given in a part's space (cm).
fn part_point(body: &Ragdoll, bone: usize, local_cm: Vec3) -> Option<Vec3> {
    let part = body.parts.iter().find(|p| p.bone == bone)?;
    Some(part.transform().transform_point3(local_cm / CM) * CM)
}

/// A body's chest point in the world (cm), the aim its blasts and torrents
/// take: the chest part's origin.
pub(crate) fn chest_point(body: &Ragdoll) -> Option<Vec3> {
    part_point(body, CHEST, Vec3::ZERO)
}

/// The view a blast reads from its caster: the right hand's part (the
/// hand's node and its place), the facing and the aim point.
pub fn blast_view(caster: &Ragdoll, facing: Vec3, target: Option<Vec3>) -> Option<BlastView> {
    let hand = part_point(caster, RIGHT_HAND, Vec3::ZERO)?;
    Some(BlastView {
        target: target.unwrap_or(hand + facing * AHEAD),
        hand_node: hand,
        hand,
        facing,
    })
}

/// What a character hit leaves on the struck body's drive, after its
/// knock: the support pins' gain is set to the knock's
/// ([`CharacterKnock::pin_gain`], recovering 0.001 a tick from there) and
/// the balance's landing momentum takes its full weight
/// ([`HIT_LANDING_WEIGHT`]). The blast's and the torrent's hits both end
/// this way.
pub(crate) fn loosen_pins(drive: &mut Drive, knock: &CharacterKnock) {
    drive.pin_gain = knock.pin_gain;
    drive.land();
    debug_assert_eq!(drive.balance().landing_weight(), HIT_LANDING_WEIGHT);
    debug!(
        "force hit: pin gain {:.4}, landing weight {:.1}",
        drive.pin_gain,
        drive.balance().landing_weight()
    );
}

/// A body's mass: its parts' masses summed.
pub(crate) fn body_mass(body: &Ragdoll) -> f32 {
    body.parts
        .iter()
        .filter(|p| p.inverse_mass > 0.0)
        .map(|p| 1.0 / p.inverse_mass)
        .sum()
}

/// A limb sphere transformed to the world (cm), with the part values a
/// strike reads. The scratch buffers of the live effects hold these.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LimbSphere {
    pub part: usize,
    pub sphere: usize,
    pub centre: Vec3,
    pub radius: f32,
    /// The part's mean particle motion this tick (cm).
    pub velocity: Vec3,
    pub inverse_mass: f32,
    /// The part's transform's translation (cm); the bubble's own test uses
    /// it.
    pub middle: Vec3,
}

/// The most a limb's reach scale takes ([`limb_terms`]'s cap, and the
/// torrent's); the least is above 0.
pub(crate) const LIMB_SCALE_MOST: f32 = 3.0;

/// The slack (cm) a box test leaves for the rounding of the sphere tests it
/// stands before.
const BOUNDS_SLACK: f32 = 1.0;

/// The most a reach of `reach(scale)` takes over the limb scales, which lie
/// between 0 and [`LIMB_SCALE_MOST`]: a reach is linear in its scale, so one
/// end holds the most. Never below 0.
pub(crate) fn most_reach(reach: impl Fn(f32) -> f32) -> f32 {
    reach(0.0).max(reach(LIMB_SCALE_MOST)).max(0.0)
}

/// The box (cm) of `points`, each grown by its own reach: no limb sphere
/// outside it, grown by the sphere's radius, is within a point's reach.
/// `None` without points; a point or reach that is not finite gives a box
/// that never misses.
pub(crate) fn reach_box(points: impl IntoIterator<Item = (Vec3, f32)>) -> Option<(Vec3, Vec3)> {
    let mut any = false;
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    for (at, reach) in points {
        any = true;
        if !at.is_finite() || !reach.is_finite() {
            return Some((Vec3::splat(f32::NAN), Vec3::splat(f32::NAN)));
        }
        low = low.min(at - Vec3::splat(reach));
        high = high.max(at + Vec3::splat(reach));
    }
    any.then_some((low, high))
}

/// Transforms `body`'s limb spheres into `out`, once, and returns their
/// world box (cm).
pub(crate) fn gather_limbs(body: &Ragdoll, out: &mut Vec<LimbSphere>) -> (Vec3, Vec3) {
    out.clear();
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    for (part_index, part) in body.parts.iter().enumerate() {
        let world = part.transform();
        let middle = Vec3::from(world.translation) * CM;
        let here: Vec3 = part.positions.iter().copied().sum::<Vec3>() * 0.25;
        let before: Vec3 = part.previous.iter().copied().sum::<Vec3>() * 0.25;
        let velocity = (here - before) * CM;
        for (sphere, &(c, r)) in part.spheres.iter().enumerate() {
            let centre = world.transform_point3(c) * CM;
            let radius = r * CM;
            out.push(LimbSphere {
                part: part_index,
                sphere,
                centre,
                radius,
                velocity,
                inverse_mass: part.inverse_mass,
                middle,
            });
            low = low.min(centre - Vec3::splat(radius));
            high = high.max(centre + Vec3::splat(radius));
        }
    }
    (low, high)
}

/// Refreshes the spheres of `part_index`, at `limbs[start..end]`, after a
/// push moved its particles: the next chunk or point reads the moved part as
/// the per-part transform did before.
pub(crate) fn refresh_limbs(
    body: &Ragdoll,
    part_index: usize,
    limbs: &mut [LimbSphere],
    start: usize,
    end: usize,
) {
    let part = &body.parts[part_index];
    let world = part.transform();
    let middle = Vec3::from(world.translation) * CM;
    let here: Vec3 = part.positions.iter().copied().sum::<Vec3>() * 0.25;
    let before: Vec3 = part.previous.iter().copied().sum::<Vec3>() * 0.25;
    let velocity = (here - before) * CM;
    for limb in &mut limbs[start..end] {
        let (c, r) = part.spheres[limb.sphere];
        limb.centre = world.transform_point3(c) * CM;
        limb.radius = r * CM;
        limb.velocity = velocity;
        limb.inverse_mass = part.inverse_mass;
        limb.middle = middle;
    }
}

/// Whether a body's box misses an effect's box (both cm, low and high
/// corners) by more than the rounding slack: a non-finite box never misses.
pub(crate) fn bounds_miss(low: Vec3, high: Vec3, effect: (Vec3, Vec3)) -> bool {
    let (el, eh) = effect;
    if !low.is_finite() || !high.is_finite() || !el.is_finite() || !eh.is_finite() {
        return false;
    }
    let (el, eh) = (
        el - Vec3::splat(BOUNDS_SLACK),
        eh + Vec3::splat(BOUNDS_SLACK),
    );
    high.x < el.x || low.x > eh.x || high.y < el.y || low.y > eh.y || high.z < el.z || low.z > eh.z
}

/// The chunks marked alive against `body`'s limb spheres (the force wave's
/// contact, `super::wave_live`): `limbs` is the scratch buffer. The
/// body is skipped when its limbs' box misses the live chunks' box grown by
/// their most reach; a chunk moves only when it meets a limb, so that test
/// skips only bodies no chunk could meet.
pub(crate) fn strike_chunks(
    chunks: &mut [BlastChunk],
    alive: &[bool],
    setup: &BlastSetup,
    body: &mut Ragdoll,
    limbs: &mut Vec<LimbSphere>,
) -> Option<Vec3> {
    let setup = *setup;
    let effect = reach_box(chunks.iter().zip(alive).filter(|&(_, &live)| live).map(
        |(chunk, _)| {
            let reach = most_reach(|scale| contact_reach(chunk, scale, setup.margin, LIMB_EXTRA));
            (chunk.at, reach)
        },
    ))?;
    let (low, high) = gather_limbs(body, limbs);
    if bounds_miss(low, high, effect) {
        return None;
    }
    let character = body.drive.is_some();
    let mass = body_mass(body);
    let mut gathered = Vec3::ZERO;
    let mut struck = false;
    for (index, chunk) in chunks.iter_mut().enumerate() {
        if !alive.get(index).copied().unwrap_or(false) {
            continue;
        }
        let mut start = 0;
        while start < limbs.len() {
            let k = limbs[start].part;
            let mut end = start;
            while end < limbs.len() && limbs[end].part == k {
                end += 1;
            }
            let inverse = limbs[start].inverse_mass;
            let (share, scale, push) = limb_terms(&setup, character, inverse, mass);
            let reach = contact_reach(chunk, scale, setup.margin, LIMB_EXTRA);
            let mut pushed = false;
            for limb in &limbs[start..end] {
                let (centre, radius, velocity, sphere) =
                    (limb.centre, limb.radius, limb.velocity, limb.sphere);
                let d = chunk.at - centre;
                let distance = d.length();
                if distance >= reach + radius || distance <= 0.0 || distance.is_nan() {
                    continue;
                }
                let point = centre + d * (radius / distance);
                let Some(hit) = limb_push(chunk, point, velocity, inverse, share, push) else {
                    continue;
                };
                chunk.at = hit.at;
                if let Some(impulse) = hit.impulse {
                    body.push_sphere(k, sphere, impulse / CM);
                    if character {
                        gathered += impulse;
                    }
                }
                let (weight, wear) = contact_weight(chunk.weight, setup.piercing, character);
                chunk.weight = weight;
                if 0.0 < wear {
                    chunk.wear = wear;
                }
                struck = true;
                pushed = true;
            }
            if pushed {
                refresh_limbs(body, k, limbs, start, end);
            }
            start = end;
        }
    }
    struck.then_some(gathered)
}

/// The countdown a struck body is woken to (the engine's wake of 256 ticks).
const BODY_WAKE: i32 = 256;
/// The reach scale a static body's contact sets.
const STATIC_SCALE: f32 = 2.0;

/// What one part's contact left: whether a chunk met it, the impulse the
/// part gathered (cm) and the reach scale the contact set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PartStrike {
    pub struck: bool,
    pub impulse: Vec3,
    pub scale: f32,
}

/// The node mesh a part's contact walks, in the part's frame (m): its
/// triangles (through its kd-tree when it has one), or, for a part built
/// without a mesh, its collision spheres as Exarust's stand-in.
enum PartMesh {
    Triangles(
        std::sync::Arc<Vec<[Vec3; 3]>>,
        Option<std::sync::Arc<exa_physics::mesh_tree::MeshTree>>,
    ),
    Spheres(std::sync::Arc<Vec<(Vec3, f32)>>),
}

impl PartMesh {
    /// The mesh's sphere query with the push-out callback: each surface
    /// point within the query's radius pushes its centre out to that radius
    /// ([`push_out`]). Returns whether a face was met.
    fn query(&self, sphere: &mut (Vec3, f32)) -> bool {
        let push = |s: &mut (Vec3, f32), point: Vec3| {
            if let Some(moved) = push_out(s.0, s.1, point) {
                s.0 = moved;
            }
        };
        match self {
            PartMesh::Triangles(triangles, tree) => {
                exa_physics::mesh_tree::sphere_contacts(triangles, tree.as_deref(), sphere, push)
            }
            // [inf: a part without a node mesh answers with its spheres:
            // each one's surface point toward the query, met within the
            // query's radius, as a face's closest point would be.]
            PartMesh::Spheres(spheres) => {
                let mut hit = false;
                for &(centre, radius) in spheres.iter() {
                    let d = sphere.0 - centre;
                    let distance = d.length();
                    if !(distance > 0.0) || !(distance < radius + sphere.1) {
                        continue;
                    }
                    let point = centre + d * (radius / distance);
                    push(sphere, point);
                    hit = true;
                }
                hit
            }
        }
    }
}

/// The particle weights of a world point (cm) over a part's particles where
/// they stand (cm), by the engine's fast barycentric.
fn part_weights(part: &exa_physics::body::Part, at_cm: Vec3) -> [f32; 4] {
    let tet = part.positions.map(|p| (p * CM).to_array());
    exa_physics::engine_math::barycentric_fast(at_cm, &tet)
}

/// How far the weighted point moved this tick (cm): the weighted sum of the
/// particles less the weighted sum of where they were, each summed in
/// particle order.
fn weighted_motion(part: &exa_physics::body::Part, w: [f32; 4]) -> Vec3 {
    let sum = |points: &[Vec3; 4]| {
        let p = points.map(|p| p * CM);
        p[0] * w[0] + p[1] * w[1] + p[2] * w[2] + p[3] * w[3]
    };
    sum(&part.positions) - sum(&part.previous)
}

/// One part's limb contact, as the original's per-part contact runs it:
/// the part's terms ([`limb_terms`]: share, reach scale and push), then
/// each live chunk taken into the part's frame, its reach
/// `reach·scale + margin + 10` tested against the node's bounding sphere
/// and box, and the node mesh's sphere query asked with the chunk pushed
/// out of every face it reaches. A chunk the mesh meets is handed, at its
/// pushed place back in the world, to the limb callback: [`limb_push`]'s
/// arithmetic with the part's velocity at the chunk (its particles'
/// weighted motion), the impulse given to the particles at the chunk's
/// weights with the push strain, the chunk's move and its weight's loss
/// ([`contact_weight`]). The part's frame is read once, before the walk, as
/// the original reads the part's matrix.
pub(crate) fn strike_part(
    blast: &mut Blast,
    body: &mut Ragdoll,
    part: usize,
    character: bool,
    mass: f32,
) -> PartStrike {
    let setup = blast.setup;
    let Some(p) = body.parts.get(part) else {
        return PartStrike {
            struck: false,
            impulse: Vec3::ZERO,
            scale: 0.0,
        };
    };
    let inverse = p.inverse_mass;
    let (share, scale, push) = limb_terms(&setup, character, inverse, mass);
    let frame = p.transform();
    let to_part = frame.inverse();
    let (bounds, mesh) = match &p.triangles {
        Some(triangles) => (
            p.pairing.mesh,
            PartMesh::Triangles(triangles.clone(), p.tree.clone()),
        ),
        None => (p.pairing.sphere, PartMesh::Spheres(p.spheres.clone())),
    };
    let mut gathered = Vec3::ZERO;
    let mut struck = false;
    for index in 0..blast.chunks.len() {
        if blast.lives[index].ticks < 1 {
            continue;
        }
        let chunk = blast.chunks[index];
        let local = to_part.transform_point3(chunk.at / CM);
        let reach = contact_reach(&chunk, scale, setup.margin, LIMB_EXTRA) / CM;
        let span = reach + bounds.radius;
        if !(local.distance_squared(bounds.centre) <= span * span) {
            continue;
        }
        let outside =
            exa_physics::intersect::point_aabb_distance_squared(local, bounds.centre, bounds.half);
        if !(outside <= reach * reach) {
            continue;
        }
        let mut sphere = (local, reach);
        if !mesh.query(&mut sphere) {
            continue;
        }
        let point = frame.transform_point3(sphere.0) * CM;
        let part_now = &body.parts[part];
        let w = part_weights(part_now, chunk.at);
        let velocity = weighted_motion(part_now, w);
        let Some(hit) = limb_push(&chunk, point, velocity, inverse, share, push) else {
            continue;
        };
        if let Some(impulse) = hit.impulse {
            gathered += impulse;
            body.push_part_weighted(part, w, impulse / CM);
        }
        let chunk = &mut blast.chunks[index];
        chunk.at = hit.at;
        let (weight, wear) = contact_weight(chunk.weight, setup.piercing, character);
        chunk.weight = weight;
        if 0.0 < wear {
            chunk.wear = wear;
        }
        struck = true;
    }
    PartStrike {
        struck,
        impulse: gathered,
        scale,
    }
}

/// Whether the sector would hand the blast a part for its contact: the
/// part's mesh sphere (cm) meets the blast's bounding sphere.
/// [inf: the original's sector walk offers the parts in its grid cells
/// near the blast; this sphere test stands in for that walk.]
fn part_near(blast: &Blast, part: &exa_physics::body::Part) -> bool {
    let bounds = if part.triangles.is_some() {
        part.pairing.mesh
    } else {
        part.pairing.sphere
    };
    let centre = part.transform().transform_point3(bounds.centre) * CM;
    let span = bounds.radius * CM + blast.bound_radius;
    !(centre.distance_squared(blast.centre) > span * span)
}

/// The blast's walk over a cloth: the chunks stir its particles
/// ([`hit_sim_object`]) in the cloth's own frame (cm), with the node's
/// bounds the particles' box; a cloth met is woken to 256 ticks and its
/// rustle envelope takes the stir. [inf: the original compares the chunks
/// with the object's points directly; Exarust's cloth keeps its particles
/// in its node's frame, so the chunks are taken into it.]
fn stir_cloth(blast: &Blast, scale: f32, cloth: &mut exa_world::cloth::ClothInstance) -> bool {
    let frame = cloth.frame.unwrap_or(bevy::math::Affine3A::IDENTITY);
    let to_cloth = frame.inverse();
    let mut local = blast.clone();
    for chunk in &mut local.chunks {
        chunk.at = to_cloth.transform_point3(chunk.at / CM) * CM;
        chunk.previous = to_cloth.transform_point3(chunk.previous / CM) * CM;
    }
    let (low, high) = exa_physics::cloth::bounds(&cloth.cloth.particles);
    let centre = (low + high) * 0.5;
    let half = high - centre;
    let mut object = SimPoints {
        points: cloth.cloth.particles.iter().map(|p| p.position).collect(),
        stir: cloth.cloth.rustle_tail,
        centre,
        radius: half.length(),
        half,
    };
    if !hit_sim_object(&local, &mut object, scale) {
        return false;
    }
    for (particle, point) in cloth.cloth.particles.iter_mut().zip(&object.points) {
        particle.position = *point;
    }
    cloth.cloth.rustle_tail = object.stir;
    if BODY_WAKE > cloth.cloth.sleep {
        cloth.cloth.sleep = BODY_WAKE;
    }
    cloth.stale_mesh = true;
    true
}

/// The level's static contact: every live chunk is swept as a ball of its
/// contact reach along its move. One that meets the level rebounds half way
/// to the contact and keeps nine tenths of its weight; a piercing blast
/// (the bolt) takes the sampled walk instead, which puts the chunk on the
/// contact, zeroes its weight and wears it. (The limb contacts take the
/// face walk for every blast.) [inf: the original walks the level's static
/// bodies' faces or sample points for the closest point to the chunk;
/// Exarust asks rapier for the swept ball and uses its centre at the hit.]
fn strike_level(blast: &mut Blast, physics: &Physics) -> usize {
    let mut hits = 0;
    for index in 0..blast.chunks.len() {
        if blast.lives[index].ticks < 1 {
            continue;
        }
        let chunk = &blast.chunks[index];
        // The sampled walk reaches `reach·2 + margin`; the face walk ten more.
        let extra = if blast.setup.piercing {
            0.0
        } else {
            STATIC_EXTRA
        };
        let reach = contact_reach(chunk, 2.0, blast.setup.margin, extra);
        let motion = chunk.at - chunk.previous;
        let squared = (motion.x * motion.x + motion.y * motion.y) + motion.z * motion.z;
        if !(crate::effects::blast::EPSILON < squared) {
            continue;
        }
        let length = squared.sqrt();
        let direction = motion * (1.0 / length);
        let Some(toi) =
            physics.level_sweep(chunk.previous / CM, direction, length / CM, reach / CM)
        else {
            continue;
        };
        let contact = chunk.previous + direction * (toi * CM);
        let chunk = &mut blast.chunks[index];
        if blast.setup.piercing {
            chunk.at = contact;
            chunk.weight = 0.0;
            chunk.wear = CONTACT_WEAR;
        } else {
            rebound(chunk, contact);
        }
        hits += 1;
    }
    hits
}

/// What a struck character takes from one part's contact, as the
/// original's part contact ends: its energy and the knock from the part's
/// gathered impulse ([`character_knock`]), its instability counter +1, the
/// concern 1 at 0.01 toward the caster, and the pins loosened
/// ([`loosen_pins`]).
/// [unread: the contact also clears a body byte and, for a part of kind
/// 0x10, sets its node's 0x80 bit and zeroes a part word; their readers are
/// not traced, so Exarust does neither.]
fn knock_character(
    body: &mut Ragdoll,
    mass: f32,
    impulse: Vec3,
    caster: Entity,
    struck: Entity,
    combatants: &mut Query<&mut Combatant>,
    npcs: &Query<&Npc>,
) {
    let Some(knock) = body.drive.as_mut().map(|drive| {
        let knock = character_knock(mass, drive.energy, impulse);
        drive.energy = knock.energy;
        let counter = drive.balance().counter() + 1;
        drive.balance_mut().set_counter(counter);
        knock
    }) else {
        return;
    };
    if let Some((direction, size)) = knock.knock {
        let flat = Vec3::new(direction.x, 0.0, direction.z);
        body.begin_knockback(flat * (size / CM));
    }
    if let Ok(mut combatant) = combatants.get_mut(struck) {
        let aggression = npcs
            .get(struck)
            .ok()
            .and_then(|npc| npc.personality)
            .map_or(0.0, |personality| personality.restraint);
        combatant.add_concern(caster, aggression, HIT_HOSTILITY.0, HIT_HOSTILITY.1);
    }
    if let Some(drive) = body.drive.as_mut() {
        loosen_pins(drive, &knock);
    }
}

/// Ticks every blast in the scene.
#[allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::too_many_lines
)]
pub(crate) fn tick_blasts(
    mut commands: Commands,
    mut blasts: Query<(Entity, &mut LiveBlast)>,
    mut casters: Query<(
        &mut PowerCaster,
        Option<&Locomotor>,
        Option<&GlobalTransform>,
    )>,
    mut bodies: Query<(Entity, &mut Ragdoll)>,
    places: Query<&GlobalTransform>,
    cursor: Option<Res<CursorGround>>,
    physics: Option<Res<Physics>>,
    mut combatants: Query<&mut Combatant>,
    npcs: Query<&Npc>,
    mut draws: ResMut<EffectDraws>,
    mut cloths: Query<&mut exa_world::cloth::ClothInstance>,
) {
    for (entity, mut live) in &mut blasts {
        let live = &mut *live;
        let target = live
            .target
            .and_then(|t| {
                bodies
                    .get(t)
                    .ok()
                    .and_then(|(_, body)| chest_point(body))
                    .or_else(|| places.get(t).ok().map(|p| p.translation() * CM))
            })
            .or_else(|| {
                live.target
                    .is_none()
                    .then(|| cursor.as_deref().and_then(|c| c.0).map(|p| p * CM))
                    .flatten()
            });
        let mut scratch = CastBody::default();
        let (body, facing) = match casters.get_mut(live.caster) {
            Ok((caster, locomotor, at)) => {
                let facing = match (locomotor, at) {
                    (Some(l), _) => Vec3::new(l.yaw.sin(), 0.0, l.yaw.cos()),
                    (None, Some(at)) => at.rotation() * Vec3::Z,
                    (None, None) => Vec3::Z,
                };
                (Some(caster), facing)
            }
            Err(_) => (None, Vec3::Z),
        };
        let view = bodies
            .get(live.caster)
            .ok()
            .and_then(|(_, body)| blast_view(body, facing, target));
        let mut caster = body;
        let cast_body = match caster.as_deref_mut() {
            Some(caster) => &mut caster.body,
            None => {
                live.blast.release();
                &mut scratch
            }
        };
        if hook_called(live, cast_body) {
            live.blast.release();
        }
        let Some(view) = view.or_else(|| {
            (live.blast.state == THROWN).then_some(BlastView {
                target: Vec3::ZERO,
                hand_node: Vec3::ZERO,
                hand: Vec3::ZERO,
                facing,
            })
        }) else {
            if live.hooked {
                casting::end_cast(cast_body);
            }
            commands.entity(entity).despawn();
            continue;
        };
        let events = live.blast.tick(&view, cast_body, live.hooked, &mut draws.0);
        if live.blast.state != CHARGING {
            live.hooked = false;
        }
        if let Some(pitch) = events.released {
            debug!("blast {entity:?} thrown, release pitch {pitch:.3}");
            // The release alerts the caster's AI's known characters: each
            // within 1800 cm is told of the caster by the draw's weight.
            let known: Vec<Entity> = combatants.get(live.caster).map_or_else(
                |_| Vec::new(),
                |combatant| {
                    combatant
                        .contacts
                        .iter()
                        .filter(|contact| contact.kind == 1)
                        .map(|contact| contact.character)
                        .collect()
                },
            );
            let origin = bodies
                .get(live.caster)
                .ok()
                .and_then(|(_, body)| chest_point(body));
            if let Some(origin) = origin {
                for other in known {
                    if other == live.caster {
                        continue;
                    }
                    let at = bodies
                        .get(other)
                        .ok()
                        .and_then(|(_, body)| chest_point(body));
                    let Some(amount) = at.and_then(|at| alert_amount(origin, at, live.blast.aim))
                    else {
                        continue;
                    };
                    if let Ok(mut combatant) = combatants.get_mut(other) {
                        let aggression = npcs
                            .get(other)
                            .ok()
                            .and_then(|npc| npc.personality)
                            .map_or(0.0, |personality| personality.restraint);
                        combatant.add_concern(live.caster, aggression, amount, ALERT_GAIN);
                    }
                }
            }
        }
        if let Some(caster) = caster.as_deref_mut() {
            caster.sync_body();
        }
        if live.blast.state == THROWN {
            // The sector's object contacts in the order its step makes
            // them: the level, the simulated objects, then the bodies'
            // parts. Each contact leaves its reach scale for the next.
            if let Some(physics) = physics.as_deref() {
                strike_level(&mut live.blast, physics);
                live.contact_scale = STATIC_SCALE;
            }
            for mut cloth in &mut cloths {
                if stir_cloth(&live.blast, live.contact_scale, &mut cloth) {
                    debug!("blast {entity:?} stirs a cloth");
                }
            }
            for (other, mut body) in &mut bodies {
                if other == live.caster {
                    continue;
                }
                let character = body.drive.is_some();
                let mass = body_mass(&body);
                for part in 0..body.parts.len() {
                    if !part_near(&live.blast, &body.parts[part]) {
                        continue;
                    }
                    let strike = strike_part(&mut live.blast, &mut body, part, character, mass);
                    live.contact_scale = strike.scale;
                    if !strike.struck {
                        continue;
                    }
                    debug!(
                        "blast {entity:?} strikes {other:?} part {part}: impulse {:.3?} cm",
                        strike.impulse
                    );
                    if character {
                        knock_character(
                            &mut body,
                            mass,
                            strike.impulse,
                            live.caster,
                            other,
                            &mut combatants,
                            &npcs,
                        );
                    }
                    body.touch(BODY_WAKE);
                }
            }
        }
        if events.removed {
            commands.entity(entity).despawn();
        }
    }
}

/// Draws every blast's ribbon, built in the camera's space each frame, as
/// the original's linked geometry draw does, with `effectforce_f` and the
/// frame's `TexOfs` ([`super::effect_draw::show_ribbon`]).
#[allow(clippy::type_complexity)]
pub(crate) fn draw_blasts(
    mut commands: Commands,
    mut blasts: Query<(
        Entity,
        &LiveBlast,
        Option<(&EffectMesh, &super::effect_draw::RibbonMaterial)>,
    )>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    time: Res<Time<Fixed>>,
    noise: Res<super::effect_draw::ForceNoise>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::force_effects::ForceRibbonMaterial>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let view = ViewFrame {
        x: camera.right().as_vec3(),
        y: camera.up().as_vec3(),
        z: camera.back().as_vec3(),
        at: camera.translation() * CM,
    };
    let alpha = time.overstep_fraction();
    for (entity, live, drawn) in &mut blasts {
        // The ribbon draws the thrown trail; while the blast charges, the
        // original's entity object draws the chunks and the trail has no
        // step to sample (a zero step would divide the widths by zero).
        if live.blast.state != THROWN {
            continue;
        }
        let ribbon = live.blast.ribbon(alpha, &view);
        if ribbon.is_empty() {
            continue;
        }
        let mesh = mesh_of_ribbon(&ribbon);
        super::effect_draw::show_ribbon(
            &mut commands,
            entity,
            drawn,
            mesh,
            live.blast.frame(alpha).texture_offset,
            false,
            &noise,
            &mut meshes,
            &mut materials,
        );
    }
}

/// Bodies for the strike tests: rigid bodies of a few parts scattered about.
#[cfg(test)]
pub(crate) mod test_bodies {
    use bevy::math::Affine3A;
    use bevy::prelude::*;
    use exa_physics::body::{Ragdoll, RigidPart};

    use crate::arena::Mt19937;

    /// A value from `low` to `high`.
    pub fn between(random: &mut Mt19937, low: f32, high: f32) -> f32 {
        low + (high - low) * random.unit() as f32
    }

    /// A point within `half` metres of `centre` on every axis.
    pub fn near(random: &mut Mt19937, centre: Vec3, half: f32) -> Vec3 {
        centre
            + Vec3::new(
                between(random, -half, half),
                between(random, -half, half),
                between(random, -half, half),
            )
    }

    /// A rigid body of one to four parts, each with one to three spheres,
    /// within `half` metres of `centre`, moving a little.
    pub fn scattered(random: &mut Mt19937, centre: Vec3, half: f32) -> Ragdoll {
        let count = 1 + (random.unit() * 4.0) as usize % 4;
        let parts: Vec<RigidPart> = (0..count)
            .map(|_| {
                let at = near(random, centre, half);
                let spheres = (0..1 + (random.unit() * 3.0) as usize % 3)
                    .map(|_| (near(random, Vec3::ZERO, 0.3), between(random, 0.05, 0.3)))
                    .collect();
                let mass = between(random, 0.5, 20.0);
                RigidPart {
                    entity: Entity::PLACEHOLDER,
                    parent: None,
                    world: Affine3A::from_translation(at),
                    rest: [
                        Vec3::new(-0.2, 0.0, -0.2),
                        Vec3::new(0.2, 0.0, -0.2),
                        Vec3::new(0.0, 0.0, 0.2),
                        Vec3::new(0.0, 0.4, 0.0),
                    ],
                    point_masses: [mass; 4],
                    parent_world: Affine3A::IDENTITY,
                    mobility: 1.0,
                    spheres,
                    origin: Vec3::ZERO,
                    links: Vec::new(),
                    mesh: None,
                    triangles: None,
                }
            })
            .collect();
        let mut body = Ragdoll::rigid(&parts);
        for part in &mut body.parts {
            let step = near(random, Vec3::ZERO, 0.02);
            for previous in &mut part.previous {
                *previous -= step;
            }
        }
        body
    }

    /// Whether two bodies' particles are the same to the bit.
    pub fn same_particles(a: &Ragdoll, b: &Ragdoll) -> bool {
        a.parts.iter().zip(&b.parts).all(|(p, q)| {
            p.positions
                .iter()
                .chain(&p.previous)
                .zip(q.positions.iter().chain(&q.previous))
                .all(|(x, y)| same(*x, *y))
                && p.inverse_mass.to_bits() == q.inverse_mass.to_bits()
        })
    }

    /// Whether two vectors are the same to the bit.
    pub fn same(a: Vec3, b: Vec3) -> bool {
        a.to_array()
            .iter()
            .zip(b.to_array())
            .all(|(x, y)| x.to_bits() == y.to_bits())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::blast::FIZZLING;
    use crate::thaum::force::{BLAST_SETUP, BOLT_SETUP};

    #[test]
    fn the_start_plays_the_motion_and_takes_the_hook() {
        let mut body = CastBody::default();
        assert!(start_blast(&mut body, &BOLT_SETUP));
        assert_eq!(body.casting, 0x202);
        assert_eq!(body.cast_motion, Some(BLAST_MOTION));
        assert_eq!(body.cast_hook, Some(BLAST_HOOK));
        // The hook answers −1, so a later cast still starts.
        assert!(casting::start_cast(&mut body, 0x23, 0x401));
        // A running cast whose hook answers 0 refuses the blast's motion.
        let mut busy = CastBody {
            cast_hook: Some(CastHook { keeps: 0x20 }),
            casting: 0x401,
            ..CastBody::default()
        };
        assert!(!start_blast(&mut busy, &BLAST_SETUP));
        assert_eq!(
            (busy.casting, busy.cast_hook),
            (0x401, Some(CastHook { keeps: 0x20 }))
        );
    }

    /// A struck character's drive takes the knock's pin gain over its own
    /// and its balance's landing momentum at 5: a 50 kg body (k = 0.04)
    /// keeps `(1 − 0.04)·0.08` (single, then double) = 0.0768, and the body
    /// tick's recovery brings it back to 0.08 by 0.001 a tick; a 1.5 kg
    /// body (k > 1) is left at 0, a −10 kg one at the clamp 0.008. The
    /// torrent's `k = 1/mass` takes the same path.
    #[test]
    fn a_hit_writes_the_pin_gain_and_the_landing_weight() {
        use crate::effects::torrent::torrent_knock;
        use exa_physics::body::{PIN_GAIN, recover_pins};
        let mut drive = Drive::default();
        assert_eq!(drive.pin_gain, PIN_GAIN);
        assert_eq!(drive.balance().landing_weight(), 0.0);
        let knock = character_knock(50.0, drive.energy, Vec3::new(40.0, 0.0, 0.0));
        loosen_pins(&mut drive, &knock);
        let want = (f64::from(1.0f32 - 2.0 / 50.0) * 0.08) as f32;
        assert_eq!(drive.pin_gain, want);
        assert!((want - 0.0768).abs() < 1e-7);
        assert_eq!(drive.balance().landing_weight(), HIT_LANDING_WEIGHT);
        // The recovery: 0.001 a tick (double) while under the double 0.08.
        let mut gain = drive.pin_gain;
        let mut ticks = 0;
        while f64::from(gain) < 0.08 {
            gain = recover_pins(gain, false);
            ticks += 1;
        }
        assert_eq!(ticks, 4);
        assert!((0.08..0.0811).contains(&gain));
        // Light and broken bodies take the floor and the clamp.
        let mut drive = Drive::default();
        loosen_pins(&mut drive, &character_knock(1.5, 1.0, Vec3::ZERO));
        assert_eq!(drive.pin_gain, 0.0);
        loosen_pins(&mut drive, &character_knock(-10.0, 1.0, Vec3::ZERO));
        assert_eq!(drive.pin_gain, crate::effects::blast::PIN_GAIN_CLAMP);
        // The torrent: k = 1/50, (1 − 0.02)·0.08 = 0.0784.
        let mut drive = Drive::default();
        loosen_pins(&mut drive, &torrent_knock(50.0, 1.0, Vec3::ZERO));
        assert_eq!(
            drive.pin_gain,
            (f64::from(1.0f32 - 1.0 / 50.0) * 0.08) as f32
        );
        assert_eq!(drive.balance().landing_weight(), 5.0);
    }

    fn live(body: &mut CastBody) -> LiveBlast {
        let hooked = start_blast(body, &BLAST_SETUP);
        let view = BlastView {
            target: Vec3::new(0.0, 150.0, 600.0),
            hand_node: Vec3::new(0.0, 150.0, 0.0),
            hand: Vec3::new(0.0, 150.0, 0.0),
            facing: Vec3::Z,
        };
        LiveBlast {
            blast: Blast::new(
                BLAST_SETUP,
                0.1,
                12,
                &view,
                &mut crate::arena::Mt19937::new(1),
            ),
            caster: Entity::PLACEHOLDER,
            target: None,
            hooked,
            generation: body.cast_generation,
            contact_scale: 0.0,
        }
    }

    #[test]
    fn a_later_cast_or_the_release_calls_the_hook() {
        let mut body = CastBody::default();
        let mut blast = live(&mut body);
        assert!(!hook_called(&mut blast, &mut body));
        assert!(casting::start_cast(&mut body, 0x23, 0x401));
        assert!(hook_called(&mut blast, &mut body));
        assert!(!blast.hooked);
        assert!(!hook_called(&mut blast, &mut body));

        let mut body = CastBody::default();
        let mut blast = live(&mut body);
        body.release = true;
        assert!(!hook_called(&mut blast, &mut body));
        assert!(casting::release_cast(&mut body));
        assert!(hook_called(&mut blast, &mut body));
        assert_eq!(
            (body.casting, body.cast_hook, body.release),
            (0, None, false)
        );
        blast.blast.release();
        assert_eq!(blast.blast.state, FIZZLING);
    }

    #[test]
    fn a_same_id_recast_is_a_new_hook() {
        let mut body = CastBody::default();
        let mut blast = live(&mut body);
        assert!(!hook_called(&mut blast, &mut body));
        // A recast of the same power id and hook: only the cast's generation
        // tells it from the blast's own.
        assert!(casting::start_cast(
            &mut body,
            BLAST_MOTION,
            BLAST_SETUP.power
        ));
        body.cast_hook = Some(BLAST_HOOK);
        assert!(hook_called(&mut blast, &mut body));
        assert!(!blast.hooked);
    }

    /// A one-part crate with one sphere at its middle.
    fn crate_body() -> Ragdoll {
        use bevy::math::Affine3A;
        use exa_physics::body::RigidPart;
        Ragdoll::rigid(&[RigidPart {
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::IDENTITY,
            rest: [
                Vec3::new(-0.2, 0.0, -0.2),
                Vec3::new(0.2, 0.0, -0.2),
                Vec3::new(0.0, 0.0, 0.2),
                Vec3::new(0.0, 0.4, 0.0),
            ],
            point_masses: [4.0; 4],
            parent_world: Affine3A::IDENTITY,
            mobility: 1.0,
            spheres: vec![(Vec3::new(0.0, 0.2, 0.0), 0.2)],
            origin: Vec3::ZERO,
            links: Vec::new(),
            mesh: None,
            triangles: None,
        }])
    }

    #[test]
    fn a_body_is_transformed_once_and_refreshed_after_a_push() {
        let mut body = crate_body();
        let mut limbs = Vec::new();
        let (low, high) = gather_limbs(&body, &mut limbs);
        assert_eq!(limbs.len(), 1);
        assert_eq!(limbs[0].centre, Vec3::new(0.0, 20.0, 0.0));
        assert_eq!(limbs[0].radius, 20.0);
        assert_eq!(low, Vec3::new(-20.0, 0.0, -20.0));
        assert_eq!(high, Vec3::new(20.0, 40.0, 20.0));
        // A box 100 cm away misses; one on the body does not.
        assert!(bounds_miss(
            low,
            high,
            (Vec3::splat(100.0), Vec3::splat(200.0))
        ));
        assert!(!bounds_miss(
            low,
            high,
            (Vec3::splat(-50.0), Vec3::splat(50.0))
        ));
        // A push moves the part; refreshing the scratch sees the move.
        body.push_sphere(0, 0, Vec3::new(0.0, 0.1, 0.0));
        refresh_limbs(&body, 0, &mut limbs, 0, 1);
        assert!(limbs[0].centre.y > 20.0);
    }

    /// The strike before its body transforms were shared and its bodies
    /// skipped by their boxes, kept to pin the new one to it.
    fn strike_chunks_unshared(
        chunks: &mut [BlastChunk],
        alive: &[bool],
        setup: &BlastSetup,
        body: &mut Ragdoll,
    ) -> Option<Vec3> {
        let character = body.drive.is_some();
        let mass = body_mass(body);
        let setup = *setup;
        let mut gathered = Vec3::ZERO;
        let mut struck = false;
        for (index, chunk) in chunks.iter_mut().enumerate() {
            if !alive.get(index).copied().unwrap_or(false) {
                continue;
            }
            for k in 0..body.parts.len() {
                let part = &body.parts[k];
                let inverse = part.inverse_mass;
                let (share, scale, push) = limb_terms(&setup, character, inverse, mass);
                let reach = contact_reach(chunk, scale, setup.margin, LIMB_EXTRA);
                let world = part.transform();
                let here: Vec3 = part.positions.iter().copied().sum::<Vec3>() * 0.25;
                let before: Vec3 = part.previous.iter().copied().sum::<Vec3>() * 0.25;
                let velocity = (here - before) * CM;
                let spheres: Vec<(Vec3, f32)> = part
                    .spheres
                    .iter()
                    .map(|&(c, r)| (world.transform_point3(c) * CM, r * CM))
                    .collect();
                for (s, (centre, radius)) in spheres.into_iter().enumerate() {
                    let d = chunk.at - centre;
                    let distance = d.length();
                    if distance >= reach + radius || distance <= 0.0 || distance.is_nan() {
                        continue;
                    }
                    let point = centre + d * (radius / distance);
                    let Some(hit) = limb_push(chunk, point, velocity, inverse, share, push) else {
                        continue;
                    };
                    chunk.at = hit.at;
                    if let Some(impulse) = hit.impulse {
                        body.push_sphere(k, s, impulse / CM);
                        if character {
                            gathered += impulse;
                        }
                    }
                    let (weight, wear) = contact_weight(chunk.weight, setup.piercing, character);
                    chunk.weight = weight;
                    if 0.0 < wear {
                        chunk.wear = wear;
                    }
                    struck = true;
                }
            }
        }
        struck.then_some(gathered)
    }

    #[test]
    fn the_shared_strike_matches_the_unshared_one_to_the_bit() {
        use super::test_bodies::{between, near, same, same_particles, scattered};
        let mut random = crate::arena::Mt19937::new(7);
        let (mut struck, mut skipped) = (0, 0);
        for round in 0..300 {
            let setup = if round % 2 == 0 {
                BLAST_SETUP
            } else {
                BOLT_SETUP
            };
            let mut body = CastBody::default();
            let mut blast = live(&mut body).blast;
            for chunk in &mut blast.chunks {
                chunk.at = near(&mut random, Vec3::new(0.0, 100.0, 0.0), 150.0);
                chunk.previous = chunk.at - near(&mut random, Vec3::ZERO, 20.0);
                chunk.reach = between(&mut random, 1.0, 40.0);
            }
            let alive: Vec<bool> = (0..blast.chunks.len())
                .map(|_| random.unit() < 0.8)
                .collect();
            let mut old_chunks = blast.chunks.clone();
            let mut new_chunks = blast.chunks.clone();
            let mut limbs = Vec::new();
            for _ in 0..4 {
                let mut old_body = scattered(&mut random, Vec3::new(0.0, 1.0, 0.0), 3.0);
                let mut new_body = old_body.clone();
                let old = strike_chunks_unshared(&mut old_chunks, &alive, &setup, &mut old_body);
                let new = strike_chunks(&mut new_chunks, &alive, &setup, &mut new_body, &mut limbs);
                assert_eq!(old.is_some(), new.is_some(), "round {round}");
                if let (Some(a), Some(b)) = (old, new) {
                    assert!(same(a, b), "round {round}");
                    struck += 1;
                } else {
                    skipped += 1;
                }
                assert!(same_particles(&old_body, &new_body), "round {round}");
                for (a, b) in old_chunks.iter().zip(&new_chunks) {
                    assert!(same(a.at, b.at), "round {round}");
                    assert_eq!(a.weight.to_bits(), b.weight.to_bits());
                    assert_eq!(a.wear.to_bits(), b.wear.to_bits());
                }
            }
        }
        assert!(
            struck > 50 && skipped > 50,
            "{struck} struck, {skipped} not"
        );
    }

    /// A rigid part at the origin whose node mesh is one face in the plane
    /// x = 0.3 m facing +x.
    fn wall_body() -> Ragdoll {
        use bevy::math::Affine3A;
        use exa_physics::body::RigidPart;
        Ragdoll::rigid(&[RigidPart {
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::IDENTITY,
            rest: [
                Vec3::new(-0.2, 0.0, -0.2),
                Vec3::new(0.2, 0.0, -0.2),
                Vec3::new(0.0, 0.0, 0.2),
                Vec3::new(0.0, 0.4, 0.0),
            ],
            point_masses: [4.0; 4],
            parent_world: Affine3A::IDENTITY,
            mobility: 1.0,
            spheres: vec![(Vec3::new(0.0, 0.2, 0.0), 0.2)],
            origin: Vec3::ZERO,
            links: Vec::new(),
            mesh: Some(exa_physics::sector::Bounds::of_box(
                Vec3::new(0.3, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 1.0),
            )),
            triangles: Some(std::sync::Arc::new(vec![[
                Vec3::new(0.3, -1.0, -1.0),
                Vec3::new(0.3, 1.0, -1.0),
                Vec3::new(0.3, 0.0, 1.0),
            ]])),
        }])
    }

    /// A thrown blast whose first chunk alone lives, at `at` (cm) moving by
    /// `motion`, reach 4.
    fn one_chunk(at: Vec3, motion: Vec3) -> Blast {
        let mut body = CastBody::default();
        let mut blast = live(&mut body).blast;
        blast.state = THROWN;
        for (index, life) in blast.lives.iter_mut().enumerate() {
            life.ticks = i32::from(index == 0);
        }
        let chunk = &mut blast.chunks[0];
        chunk.at = at;
        chunk.previous = at - motion;
        chunk.reach = 4.0;
        chunk.weight = 1.0;
        blast
    }

    #[test]
    fn a_part_contact_pushes_the_chunk_out_of_the_mesh_in_the_parts_frame() {
        let mut body = wall_body();
        let before = body.parts[0].positions;
        let mut blast = one_chunk(Vec3::new(35.0, 0.0, 0.0), Vec3::new(-5.0, 0.0, 0.0));
        let mass = body_mass(&body);
        let strike = strike_part(&mut blast, &mut body, 0, false, mass);
        // A prop part of mobility 1: share 0.2, scale min(0.5 + 2, 3) = 2.5,
        // so the reach is 4·2.5 + 10 = 20 cm. The chunk 5 cm in front of the
        // face is pushed out to 20 cm from it (x = 50): the callback's
        // `to` is +15 along x against the move of −5.
        assert!(strike.struck);
        assert_eq!(strike.scale, 2.5);
        let share = 0.2f32 / (1.0 + 0.2);
        let expected = 35.0 + 15.0 * (share * 0.25) + -5.0 * -0.05;
        assert!((blast.chunks[0].at.x - expected).abs() < 1e-4);
        // The part at rest has no velocity at the chunk, so f = 1 and the
        // limb takes move·(−k)·0.2 with k = −(1/1.2)·0.5.
        let k = (-1.0f32 / (1.0 + 0.2)) * BLAST_SETUP.body_push;
        assert!((strike.impulse.x - (-5.0 * (-k * 0.2))).abs() < 1e-5);
        assert_eq!(blast.chunks[0].weight, (0.9f64) as f32);
        // The push moved the particles along −x by their weights.
        let moved: f32 = body.parts[0]
            .positions
            .iter()
            .zip(before)
            .map(|(a, b)| a.x - b.x)
            .sum();
        assert!(moved < 0.0);
    }

    #[test]
    fn a_part_contact_misses_beyond_the_reach_and_behind_the_face() {
        let mut body = wall_body();
        let mass = body_mass(&body);
        let mut far = one_chunk(Vec3::new(55.0, 0.0, 0.0), Vec3::new(-5.0, 0.0, 0.0));
        assert!(!strike_part(&mut far, &mut body, 0, false, mass).struck);
        assert_eq!(far.chunks[0].at.x, 55.0);
        // Behind the face (the query needs the chunk in front of it).
        let mut behind = one_chunk(Vec3::new(25.0, 0.0, 0.0), Vec3::new(5.0, 0.0, 0.0));
        assert!(!strike_part(&mut behind, &mut body, 0, false, mass).struck);
        // A spent chunk is not walked.
        let mut spent = one_chunk(Vec3::new(35.0, 0.0, 0.0), Vec3::new(-5.0, 0.0, 0.0));
        spent.lives[0].ticks = 0;
        assert!(!strike_part(&mut spent, &mut body, 0, false, mass).struck);
    }

    #[test]
    fn a_part_without_a_mesh_answers_with_its_spheres() {
        let mut body = crate_body();
        let mass = body_mass(&body);
        // The sphere at y = 20 cm, radius 20: a chunk 30 cm above its
        // centre, falling, is within the 20 cm reach of its top.
        let mut blast = one_chunk(Vec3::new(0.0, 50.0, 0.0), Vec3::new(0.0, -5.0, 0.0));
        let strike = strike_part(&mut blast, &mut body, 0, false, mass);
        assert!(strike.struck);
        // Pushed out to 20 cm above the top (y = 60), so it moves up.
        assert!(blast.chunks[0].at.y > 50.0);
    }

    #[test]
    fn a_thrown_chunk_stirs_a_cloth_in_its_frame_and_wakes_it() {
        use exa_physics::cloth::{Cloth, Particle};
        let particles = vec![
            Particle {
                position: Vec3::ZERO,
                previous: Vec3::ZERO,
            },
            Particle {
                position: Vec3::new(0.0, 0.0, 30.0),
                previous: Vec3::new(0.0, 0.0, 30.0),
            },
        ];
        let mut cloth = exa_world::cloth::ClothInstance::new(
            Cloth::new(particles),
            Handle::default(),
            vec![0, 1],
        );
        cloth.cloth.sleep = 0;
        // The cloth's node stands 2 m along x: the chunk is taken into it.
        cloth.frame = Some(bevy::math::Affine3A::from_translation(Vec3::new(
            2.0, 0.0, 0.0,
        )));
        let mut blast = one_chunk(Vec3::new(200.0, 0.0, 10.0), Vec3::new(0.0, 0.0, 10.0));
        for chunk in &mut blast.chunks[1..] {
            chunk.at = Vec3::new(9000.0, 0.0, 0.0);
            chunk.previous = chunk.at;
        }
        assert!(stir_cloth(&blast, 2.0, &mut cloth));
        // Reach 4·2 + 40 = 48: both points move along the chunk's +z move.
        assert!(cloth.cloth.particles[0].position.z > 0.0);
        assert!(cloth.cloth.particles[1].position.z > 30.0);
        assert!(cloth.cloth.rustle_tail > 0.0);
        assert_eq!(cloth.cloth.sleep, BODY_WAKE);
        assert!(cloth.stale_mesh);
        // Far from the node, nothing moves.
        let mut far = one_chunk(Vec3::new(-900.0, 0.0, 10.0), Vec3::new(0.0, 0.0, 10.0));
        for chunk in &mut far.chunks[1..] {
            chunk.at = Vec3::new(9000.0, 0.0, 0.0);
            chunk.previous = chunk.at;
        }
        let before = cloth.cloth.particles.clone();
        assert!(!stir_cloth(&far, 2.0, &mut cloth));
        assert_eq!(cloth.cloth.particles, before);
    }
}
