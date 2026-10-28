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
//! - **Strikes** ([`strike_bodies`]): each live thrown chunk meets a limb
//!   sphere within its contact reach (`reach·scale + margin + 10`); the
//!   limb takes [`limb_push`]'s impulse at the sphere's surface point
//!   toward the chunk, the chunk moves and keeps [`contact_weight`] of its
//!   weight, and a character struck takes [`character_knock`] of its
//!   gathered impulse: the energy, a knockback step along the knock, its
//!   instability counter +1 and concern 1 at 0.01 toward the caster.
//!   [inf: the original's mesh and sphere tests that find a contact are not
//!   in the notes; this sphere test, the body's mass as the sum of its
//!   parts' masses, and the knock as a knockback step are Exarust's.]
//!
//! - **Release and level**: the release alerts the caster's known
//!   characters ([`alert_amount`]); thrown chunks meet the level as a swept
//!   ball ([`rebound`], or the bolt's sampled contact); the thrown ribbon
//!   draws with the glow stand-in ([`draw_blasts`]).
//!
//! Not wired: the sounds, the light, the support-pin gain and the 5 s
//! recoil a struck character takes (no body field takes them) and the
//! simulated objects' points. Another motion replacing the cast motion
//! calls the hook (`cast::call_motion_hooks`), which fizzles a charging
//! blast.
//!
//! Evidence: the blast's cast, creator, start and hook, the cast-motion
//! start, the body tick's release, the spend and the cast end, read from
//! `Exanima.exe`.

// NaN-sensitive compares are kept as the original makes them.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::prelude::*;
use exa_ai::brain::Npc;
use exa_ai::combatant::Combatant;
use exa_physics::Physics;
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::Locomotor;

use super::barricade::WAKE_TICKS;
use super::cast::PowerCaster;
use super::casting::{self, CastBody, CastHook};
use super::force::{BLAST_MOTION, BlastSetup};
use super::live::{EffectDraws, EffectMesh, mesh_of_ribbon, show};
use crate::effects::Frame as ViewFrame;
use crate::effects::blast::{
    ALERT_GAIN, Blast, BlastChunk, BlastView, CHARGING, CONTACT_WEAR, HIT_HOSTILITY, STATIC_EXTRA,
    THROWN, alert_amount, character_knock, contact_reach, contact_weight, limb_push, limb_terms,
    rebound,
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
#[derive(Component)]
pub struct LiveBlast {
    pub blast: Blast,
    pub caster: Entity,
    /// The body the AI aims at; the player aims at the cursor.
    pub target: Option<Entity>,
    /// The caster's cast hook is this blast's.
    pub hooked: bool,
    /// The cast generation whose hook the blast took.
    pub generation: u64,
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

/// One tick of a thrown blast's chunks against `body`: each live chunk
/// pushes the limb spheres it reaches. Returns the impulse a character
/// gathered, `None` when nothing was struck.
pub(crate) fn strike_bodies(
    blast: &mut Blast,
    body: &mut Ragdoll,
    alive: &mut Vec<bool>,
    limbs: &mut Vec<LimbSphere>,
) -> Option<Vec3> {
    alive.clear();
    alive.extend(blast.lives.iter().map(|l| l.ticks >= 1));
    strike_chunks(&mut blast.chunks, alive, &blast.setup, body, limbs)
}

/// The chunks marked alive against `body`'s limb spheres, as
/// [`strike_bodies`] does for a blast: `limbs` is the scratch buffer. The
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
        let reach = contact_reach(chunk, 2.0, blast.setup.margin, STATIC_EXTRA);
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
    mut alive: Local<Vec<bool>>,
    mut limbs: Local<Vec<LimbSphere>>,
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
            if let Some(physics) = physics.as_deref() {
                strike_level(&mut live.blast, physics);
            }
            for (other, mut body) in &mut bodies {
                if other == live.caster {
                    continue;
                }
                let Some(impulse) =
                    strike_bodies(&mut live.blast, &mut body, &mut alive, &mut limbs)
                else {
                    continue;
                };
                body.touch(WAKE_TICKS);
                let mass = body_mass(&body);
                let Some(knock) = body.drive.as_mut().map(|drive| {
                    let knock = character_knock(mass, drive.energy, impulse);
                    drive.energy = knock.energy;
                    let counter = drive.balance().counter() + 1;
                    drive.balance_mut().set_counter(counter);
                    knock
                }) else {
                    continue;
                };
                if let Some((direction, size)) = knock.knock {
                    let flat = Vec3::new(direction.x, 0.0, direction.z);
                    body.begin_knockback(flat * (size / CM));
                }
                if let Ok(mut combatant) = combatants.get_mut(other) {
                    let aggression = npcs
                        .get(other)
                        .ok()
                        .and_then(|npc| npc.personality)
                        .map_or(0.0, |personality| personality.restraint);
                    combatant.add_concern(
                        live.caster,
                        aggression,
                        HIT_HOSTILITY.0,
                        HIT_HOSTILITY.1,
                    );
                }
            }
        }
        if events.removed {
            commands.entity(entity).despawn();
        }
    }
}

/// The glow tint the ribbon draws with: the vertices carry the original's
/// colour bytes.
const RIBBON_TINT: Vec4 = Vec4::ONE;

/// Draws every blast's ribbon, built in the camera's space each frame, as
/// the original's linked geometry draw does.
pub(crate) fn draw_blasts(
    mut commands: Commands,
    mut blasts: Query<(Entity, &LiveBlast, Option<&EffectMesh>)>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    time: Res<Time<Fixed>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::glow::GlowMaterial>>,
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
        show(
            &mut commands,
            entity,
            drawn,
            mesh,
            RIBBON_TINT,
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
}
