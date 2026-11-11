//! The force torrent in the running game: the effect entity a torrent cast
//! makes, its streams fed from the caster's right hand while the cast is
//! held.
//!
//! - **Creator**: the power's cost a second, the caster and the power id
//!   0x10203.
//! - **Start** ([`start_torrent`]): cast motion 0x26 with that power id;
//!   when the motion starts the caster's cast hook becomes the torrent's
//!   ([`TORRENT_HOOK`]), which answers −1 and fizzles a charging torrent or
//!   ends a running one ([`Torrent::release`]).
//! - **The hook** ([`hook_called`]): a later cast's start calls it, and so
//!   do the player's buttons and the body tick's release, which end the
//!   cast. Running does not clear the hook: the torrent pours until the
//!   caster lets go or runs out.
//! - **Each tick** ([`tick_torrent`]): the torrent pays its drain while it
//!   charges and half of it while it runs; a shortfall calls the hook and
//!   ends the cast. Its spawn delays draw from the effects' shared roll.
//!   Once running (flags 0xb_0000), its live points take the sector's
//!   contacts in its step's order: the level's static meshes, then each
//!   other body part by part ([`walk_torrent_mesh`]: each stream's live
//!   points from the head back, in the mesh's frame, past its bounding
//!   sphere and box, asked of the mesh's two-sided sphere query with the
//!   push-out callback that gathers the surface points and pushes; a point
//!   met eases toward its pushed place and raises the stream loop's volume
//!   and pitch). A part's limb callback ([`strike_torrent_part`]) pushes the
//!   part's particles at the gathered surface's weights by
//!   [`limb_impulse`]; a struck character takes [`torrent_knock`] of the
//!   part's gathered impulse: the energy, a knockback step, its instability
//!   counter +1, the loosened pins and concern 1 at 0.01 toward the caster;
//!   a body met is woken.
//! - **Leaving**: the frame that would build no triangles removes it
//!   ([`Torrent::builds_nothing`]): fizzled, or every stream spent.
//!
//! [inf: the frame's turned aim is the blast aim here (the player
//! behaviour's turn spring is not wired); the level's meshes are walked
//! triangle by triangle without their kd-trees; a part without a node mesh
//! answers with its spheres.] The stream ribbon draws with
//! `effecttorrent_f` ([`draw_torrents`]). The hit ends as the blast's does
//! ([`super::blast_live::loosen_pins`]). The torrent's GI multi-point shape
//! lives with it (`super::effect_light`). Not wired: the simulated objects'
//! points and the sounds' voices.
//!
//! Evidence: the torrent's creator, start, hook, tick and frame, read from
//! `Exanima.exe`.

use bevy::prelude::*;
use exa_ai::brain::Npc;
use exa_ai::combatant::Combatant;
use exa_physics::Physics;
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::Locomotor;

use super::blast_live::{
    BODY_WAKE, PartMesh, blast_view, body_mass, level_meshes, part_weights, reach_box,
    weighted_motion,
};
use super::cast::PowerCaster;
use super::casting::{self, CastBody, CastHook, spend_tick};
use super::live::{EffectDraws, EffectMesh, EffectRoll, mesh_of_ribbon};
use crate::effects::blast::{EPSILON, HIT_HOSTILITY, blast_aim};
use crate::effects::torrent::{
    POINTS, RUNNING_FLAGS, STATIC_CONTACT, Stream, StreamPoint, TORRENT_MOTION, Torrent,
    TorrentEvents, TorrentView, limb_impulse, limb_terms, torrent_knock, torrent_reach,
};
use crate::player::CursorGround;
use crate::thaum::engine::lerp;
use crate::thaum::torrent_hit;

/// Centimetres a metre.
const CM: f32 = 100.0;
/// The torrent's power id.
pub const TORRENT: u32 = 0x10203;
/// The torrent's hook: −1 to every event.
pub const TORRENT_HOOK: CastHook = CastHook::NEVER_KEEPS;

/// A torrent in the scene.
#[derive(Component, Clone, Debug)]
pub struct LiveTorrent {
    pub torrent: Torrent,
    pub caster: Entity,
    /// The body the AI aims at; the player aims at the cursor.
    pub target: Option<Entity>,
    /// The caster's cast hook is this torrent's.
    pub hooked: bool,
    /// The cast generation whose hook the torrent took.
    pub generation: u64,
}

/// The torrent's start on its caster: cast motion 0x26 with the power id,
/// and the hook when the motion starts.
pub fn start_torrent(body: &mut CastBody) -> bool {
    let started = casting::start_cast(body, TORRENT_MOTION, TORRENT);
    if started {
        body.cast_hook = Some(TORRENT_HOOK);
    }
    started
}

/// The view a torrent reads from its caster: the blast's, turned along
/// its aim.
pub fn torrent_view(caster: &Ragdoll, facing: Vec3, target: Option<Vec3>) -> Option<TorrentView> {
    let view = blast_view(caster, facing, target)?;
    Some(TorrentView {
        target: view.target,
        hand_node: view.hand_node,
        hand: view.hand,
        facing,
        turned_aim: blast_aim(view.hand_node, view.target, facing),
    })
}

/// A torrent for `caster` costing `cost` a second.
#[allow(clippy::too_many_arguments)]
pub fn new_torrent(
    body: &mut CastBody,
    caster: Entity,
    target: Option<Entity>,
    cost: f32,
    view: &TorrentView,
    tick_ms: u32,
    draws: &mut EffectDraws,
    roll: &mut EffectRoll,
) -> LiveTorrent {
    let hooked = start_torrent(body);
    let generation = body.cast_generation;
    let torrent = Torrent::new(cost, tick_ms, view, &mut draws.0, roll.0);
    roll.0 = torrent.roll;
    LiveTorrent {
        torrent,
        caster,
        target,
        hooked,
        generation,
    }
}

/// Whether the caster called the torrent's hook since the last tick: it is
/// no longer the torrent's (a call answers −1 and ends the cast, or a later
/// cast's start replaced it). The torrent then fizzles or ends.
pub fn hook_called(live: &mut LiveTorrent, body: &mut CastBody) -> bool {
    if !live.hooked {
        return false;
    }
    let called = body.cast_generation != live.generation
        || body.casting != TORRENT
        || body.cast_hook != Some(TORRENT_HOOK);
    if called {
        live.hooked = false;
        live.torrent.release();
    }
    called
}

/// One tick of a torrent on its caster's body: the hook, then the tick
/// paying its drain.
pub fn tick_torrent(
    live: &mut LiveTorrent,
    body: &mut CastBody,
    view: &TorrentView,
    draws: &mut EffectDraws,
    roll: &mut EffectRoll,
) -> TorrentEvents {
    hook_called(live, body);
    let hooked = live.hooked;
    let mut short = false;
    live.torrent.roll = roll.0;
    let events = live.torrent.tick_paying(view, &mut draws.0, |cost| {
        let shortfall = spend_tick(body, cost);
        short |= shortfall;
        shortfall && hooked
    });
    roll.0 = live.torrent.roll;
    if short {
        live.hooked = false;
        casting::end_cast(body);
    }
    events
}

/// A stream's live points, newest first: from its head back while a point
/// lives, once round the ring at most.
fn live_points(stream: &Stream) -> impl Iterator<Item = &StreamPoint> {
    let mut k = stream.head;
    let mut done = false;
    std::iter::from_fn(move || {
        if done {
            return None;
        }
        let point = &stream.points[k];
        if point.life < 1 {
            done = true;
            return None;
        }
        k = if k == 0 { POINTS - 1 } else { k - 1 };
        done = k == stream.head;
        Some(point)
    })
}

/// What a torrent's mesh walk gathers over one stream's points (reset at
/// each stream): the surface points its pushes met, summed, the pushes
/// taken back, summed, and how many there were.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Gathered {
    pub surface: Vec3,
    pub pushed: Vec3,
    pub count: i32,
}

/// The torrent's push-out callback: a surface point within the sphere's
/// radius (and not at its centre) pushes the sphere out to it; the push is
/// taken from the gathered pushes, the point added to the gathered surface
/// and the count raised.
fn gather_push(gathered: &mut Gathered, sphere: &mut (Vec3, f32), point: Vec3) {
    let d = sphere.0 - point;
    let d2 = (d.x * d.x + d.y * d.y) + d.z * d.z;
    if d2 < sphere.1 * sphere.1 && EPSILON <= d2 {
        let d = d * (sphere.1 / d2.sqrt() - 1.0);
        sphere.0 += d;
        gathered.pushed -= d;
        gathered.surface += point;
        gathered.count += 1;
    }
}

/// The torrent's mesh walk over one mesh, as the original runs it for the
/// level and for a part: each stream's live points from its head back,
/// each taken into the mesh's frame (`to_mesh`, cm) with its reach
/// `width·scale + margin + 10`, tested against the mesh's bounding sphere
/// and box (`bounds`: centre, half extents, radius, cm), and asked of the
/// mesh's two-sided sphere query (`query`) with the push-out callback
/// ([`gather_push`]). A point the mesh met hands its pushed place, back in
/// the world, to `limb` (the part contact's limb callback; none for the
/// level), then eases toward that place by `ease`, and the stream loop's
/// volume and pitch rise by its life share ([`torrent_hit`]). The
/// gathered sums are reset at each stream, not each point. Returns whether
/// a point met the mesh.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn walk_torrent_mesh(
    torrent: &mut Torrent,
    scale: f32,
    ease: f32,
    to_mesh: impl Fn(Vec3) -> Vec3,
    to_world: impl Fn(Vec3) -> Vec3,
    bounds: (Vec3, Vec3, f32),
    mut query: impl FnMut(&mut (Vec3, f32), &mut dyn FnMut(&mut (Vec3, f32), Vec3)) -> bool,
    mut limb: Option<&mut dyn FnMut(&mut Gathered, Vec3, &StreamPoint)>,
) -> bool {
    let (centre, half, radius) = bounds;
    let margin = torrent.margin;
    let point_life = torrent.point_life;
    let mut struck = false;
    for index in 0..torrent.count {
        let mut gathered = Gathered::default();
        let head = torrent.records[index].head;
        let mut k = head;
        loop {
            let point = torrent.records[index].points[k];
            if point.life < 1 {
                break;
            }
            let local = to_mesh(point.at);
            let reach = torrent_reach(point.width, scale, margin);
            let span = reach + radius;
            let near = local.distance_squared(centre) <= span * span
                && exa_physics::intersect::point_aabb_distance_squared(local, centre, half)
                    <= reach * reach;
            if near {
                let mut sphere = (local, reach);
                let met = query(&mut sphere, &mut |s, q| gather_push(&mut gathered, s, q));
                if met {
                    let world = to_world(sphere.0);
                    if let Some(limb) = limb.as_deref_mut() {
                        limb(&mut gathered, world, &point);
                    }
                    let sounds = &mut torrent.sounds;
                    let hit = torrent_hit(
                        point.life,
                        point_life,
                        sounds.loop_volume,
                        sounds.loop_pitch,
                    );
                    sounds.loop_volume = hit.left;
                    sounds.loop_pitch = hit.right;
                    let point = &mut torrent.records[index].points[k];
                    point.at = lerp(point.at, world, ease);
                    struck = true;
                }
            }
            k = if k == 0 { POINTS - 1 } else { k - 1 };
            if k == head {
                break;
            }
        }
    }
    struck
}

/// The torrent's static contact on the level's meshes: the reach scale
/// 0.25, the ease 0.5 and no limb callback ([`walk_torrent_mesh`]), each
/// mesh in the world's frame.
fn strike_level(
    torrent: &mut Torrent,
    statics: &exa_physics::StaticColliders,
    physics: &Physics,
    places: &Query<&GlobalTransform>,
    triangles: &mut Vec<[Vec3; 3]>,
) -> usize {
    let margin = torrent.margin;
    let Some((low, high)) = reach_box(torrent.records[..torrent.count].iter().flat_map(|stream| {
        live_points(stream).map(|point| {
            (
                point.at,
                torrent_reach(point.width, STATIC_CONTACT.scale, margin),
            )
        })
    })) else {
        return 0;
    };
    let around = (torrent.centre, torrent.bound_radius);
    let mut hits = 0;
    level_meshes(
        statics,
        physics,
        places,
        around,
        (low, high),
        triangles,
        |mesh| {
            let met = walk_torrent_mesh(
                torrent,
                STATIC_CONTACT.scale,
                STATIC_CONTACT.ease,
                |p| p,
                |p| p,
                (mesh.centre, mesh.half, mesh.radius),
                |sphere, on_point| {
                    let mut met = false;
                    for t in mesh.triangles {
                        if let Some(q) =
                            exa_physics::mesh_query::two_sided_point(t, sphere.0, sphere.1, EPSILON)
                        {
                            on_point(sphere, q);
                            met = true;
                        }
                    }
                    met
                },
                None,
            );
            hits += usize::from(met);
        },
    );
    hits
}

/// One part's contact, as the original's torrent part contact runs it:
/// the part's terms ([`limb_terms`]: reach scale, push and ease), the walk
/// over the part's node mesh in its frame ([`walk_torrent_mesh`]), and the
/// limb callback for each point met: the gathered surface points' mean
/// (the sum divided in place by the count), the part's velocity at that
/// mean (its particles' weighted motion at the mean's weights), the
/// impulse ([`limb_impulse`] with the point's pushed place and life share)
/// gathered and pushed on the particles at those weights. Returns whether a
/// point met the part and the impulse it gathered (cm).
pub(crate) fn strike_torrent_part(
    torrent: &mut Torrent,
    body: &mut Ragdoll,
    part: usize,
    character: bool,
    mass: f32,
) -> (bool, Vec3) {
    let Some(p) = body.parts.get(part) else {
        return (false, Vec3::ZERO);
    };
    let inverse = p.inverse_mass;
    let (scale, push, ease) = limb_terms(character, inverse, mass);
    let frame = p.transform();
    let to_part = frame.inverse();
    let (bounds, mesh) = PartMesh::of_part(p);
    let bounds_cm = (bounds.centre * CM, bounds.half * CM, bounds.radius * CM);
    let point_life = torrent.point_life;
    let mut impulse_sum = Vec3::ZERO;
    let mut limb = |gathered: &mut Gathered, world: Vec3, point: &StreamPoint| {
        if gathered.count < 1 {
            return;
        }
        gathered.surface *= 1.0 / gathered.count as f32;
        let surface = frame.transform_point3(gathered.surface / CM) * CM;
        let part_now = &body.parts[part];
        let w = part_weights(part_now, surface);
        let velocity = weighted_motion(part_now, w);
        let share = point.life as f32 / point_life as f32;
        let impulse = limb_impulse(point, world, velocity, inverse, push, share);
        impulse_sum += impulse;
        body.push_part_weighted(part, w, impulse / CM);
    };
    let struck = walk_torrent_mesh(
        torrent,
        scale,
        ease,
        |p| to_part.transform_point3(p / CM) * CM,
        |p| frame.transform_point3(p / CM) * CM,
        bounds_cm,
        |sphere, on_point| mesh.query_cm(sphere, on_point),
        Some(&mut limb),
    );
    (struck, impulse_sum)
}

/// Whether the sector would hand the torrent a part: the part's mesh
/// sphere (cm) meets the torrent's bounding sphere. [inf: as the blast's,
/// a sphere test stands in for the sector's cell walk.]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn part_near(torrent: &Torrent, part: &exa_physics::body::Part) -> bool {
    let bounds = if part.triangles.is_some() {
        part.pairing.mesh
    } else {
        part.pairing.sphere
    };
    let centre = part.transform().transform_point3(bounds.centre) * CM;
    let span = bounds.radius * CM + torrent.bound_radius;
    !(centre.distance_squared(torrent.centre) > span * span)
}

/// What a struck character takes from one part's torrent contact: the
/// energy, the knock from the part's gathered impulse ([`torrent_knock`]),
/// its instability counter +1, the loosened pins and concern 1 at 0.01
/// toward the caster.
fn knock_torrent_character(
    body: &mut Ragdoll,
    mass: f32,
    impulse: Vec3,
    caster: Entity,
    struck: Entity,
    combatants: &mut Query<&mut Combatant>,
    npcs: &Query<&Npc>,
) {
    let Some(knock) = body.drive.as_mut().map(|drive| {
        let knock = torrent_knock(mass, drive.energy, impulse);
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
        super::blast_live::loosen_pins(drive, &knock);
    }
}

/// Ticks every torrent in the scene.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick_torrents(
    mut commands: Commands,
    mut torrents: Query<(Entity, &mut LiveTorrent)>,
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
    mut roll: ResMut<EffectRoll>,
    (statics, mut triangles): (
        Option<Res<exa_physics::StaticColliders>>,
        Local<Vec<[Vec3; 3]>>,
    ),
) {
    for (entity, mut live) in &mut torrents {
        let live = &mut *live;
        let target = live
            .target
            .and_then(|t| {
                bodies
                    .get(t)
                    .ok()
                    .and_then(|(_, body)| super::blast_live::chest_point(body))
                    .or_else(|| places.get(t).ok().map(|p| p.translation() * CM))
            })
            .or_else(|| {
                live.target
                    .is_none()
                    .then(|| cursor.as_deref().and_then(|c| c.0).map(|p| p * CM))
                    .flatten()
            });
        let mut scratch = CastBody::default();
        let mut caster = casters.get_mut(live.caster).ok();
        let facing = match caster.as_ref() {
            Some((_, Some(l), _)) => Vec3::new(l.yaw.sin(), 0.0, l.yaw.cos()),
            Some((_, None, Some(at))) => at.rotation() * Vec3::Z,
            _ => Vec3::Z,
        };
        let view = bodies
            .get(live.caster)
            .ok()
            .and_then(|(_, body)| torrent_view(body, facing, target));
        let body = match caster.as_mut() {
            Some((caster, _, _)) => &mut caster.body,
            None => {
                live.hooked = false;
                live.torrent.release();
                &mut scratch
            }
        };
        let Some(view) = view else {
            if live.hooked {
                casting::end_cast(body);
            }
            commands.entity(entity).despawn();
            continue;
        };
        tick_torrent(live, body, &view, &mut draws, &mut roll);
        if let Some((caster, _, _)) = caster.as_mut() {
            caster.sync_body();
        }
        if live.torrent.flags & RUNNING_FLAGS != 0 {
            // The sector's contacts in its step's order: the level, then
            // the bodies' parts.
            if let (Some(physics), Some(statics)) = (physics.as_deref(), statics.as_deref()) {
                let hits =
                    strike_level(&mut live.torrent, statics, physics, &places, &mut triangles);
                if 0 < hits {
                    debug!("torrent {entity:?} meets the level: {hits} meshes");
                }
            }
            for (other, mut body) in &mut bodies {
                if other == live.caster {
                    continue;
                }
                let character = body.drive.is_some();
                let mass = body_mass(&body);
                for part in 0..body.parts.len() {
                    if !part_near(&live.torrent, &body.parts[part]) {
                        continue;
                    }
                    let (struck, impulse) =
                        strike_torrent_part(&mut live.torrent, &mut body, part, character, mass);
                    if !struck {
                        continue;
                    }
                    debug!(
                        "torrent {entity:?} strikes {other:?} part {part}: impulse {impulse:.3?} cm"
                    );
                    if character {
                        knock_torrent_character(
                            &mut body,
                            mass,
                            impulse,
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
        if live.torrent.builds_nothing() {
            if let Ok((mut caster, _, _)) = casters.get_mut(live.caster)
                && live.hooked
            {
                casting::end_cast(&mut caster.body);
                caster.sync_body();
            }
            commands.entity(entity).despawn();
        }
    }
}

/// Draws every torrent's stream ribbon, built from this frame's samples,
/// with `effecttorrent_f` and the frame's `TexOfs`
/// ([`super::effect_draw::show_ribbon`]).
#[allow(clippy::type_complexity)]
pub(crate) fn draw_torrents(
    mut commands: Commands,
    mut torrents: Query<(
        Entity,
        &mut LiveTorrent,
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
    // The original carries the camera into the effect node's frame; the
    // node is unturned here, so its place (cm) is the eye.
    let eye = camera.translation() * CM;
    let alpha = time.overstep_fraction();
    for (entity, mut live, drawn) in &mut torrents {
        let frame = live.torrent.frame(alpha);
        let ribbon = live.torrent.ribbon(&frame, eye);
        if ribbon.is_empty() {
            continue;
        }
        let mesh = mesh_of_ribbon(&ribbon);
        super::effect_draw::show_ribbon(
            &mut commands,
            entity,
            drawn,
            mesh,
            frame.texture_offset,
            true,
            &noise,
            &mut meshes,
            &mut materials,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Mt19937;
    use crate::effects::torrent::{CHARGING, ENDING, FIZZLING, RUNNING};
    use crate::thaum::engine::Roll;

    fn view() -> TorrentView {
        TorrentView {
            target: Vec3::new(0.0, 150.0, 1000.0),
            hand_node: Vec3::new(0.0, 150.0, 0.0),
            hand: Vec3::new(0.0, 150.0, 0.0),
            facing: Vec3::Z,
            turned_aim: Vec3::Z,
        }
    }

    fn live(body: &mut CastBody) -> (LiveTorrent, EffectDraws, EffectRoll) {
        let mut draws = EffectDraws(Mt19937::new(1));
        let mut roll = EffectRoll(Roll(7));
        let torrent = new_torrent(
            body,
            Entity::PLACEHOLDER,
            None,
            0.1,
            &view(),
            12,
            &mut draws,
            &mut roll,
        );
        (torrent, draws, roll)
    }

    #[test]
    fn the_start_plays_the_motion_and_takes_the_hook() {
        let mut body = CastBody::default();
        let (torrent, _, roll) = live(&mut body);
        assert!(torrent.hooked);
        assert_eq!(body.casting, TORRENT);
        assert_eq!(body.cast_motion, Some(TORRENT_MOTION));
        assert_eq!(body.cast_hook, Some(TORRENT_HOOK));
        assert_eq!(torrent.torrent.state, CHARGING);
        assert_eq!(torrent.torrent.drain, 0.1 * 0.012);
        // The start's sixteen delays drew from the shared roll.
        assert_eq!(roll.0, torrent.torrent.roll);
        assert_ne!(roll.0, Roll(7));
    }

    #[test]
    fn a_later_cast_fizzles_a_charging_torrent_and_a_shortfall_ends_the_cast() {
        let mut body = CastBody {
            focus: 1.0,
            ..CastBody::default()
        };
        let (mut torrent, mut draws, mut roll) = live(&mut body);
        assert!(casting::start_cast(&mut body, 0x22, 0x201));
        tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
        assert!(!torrent.hooked);
        assert!(torrent.torrent.state < CHARGING);
        assert_eq!(body.casting, 0x201);

        let mut body = CastBody {
            focus: 0.0,
            ..CastBody::default()
        };
        let (mut torrent, mut draws, mut roll) = live(&mut body);
        tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
        assert_eq!(torrent.torrent.state, FIZZLING);
        assert_eq!((body.casting, body.cast_hook), (0, None));
        assert_eq!(body.condition, 0);
        assert!(casting::take_stance_return(&mut body), "the stance return");
    }

    #[test]
    fn a_running_torrent_that_runs_short_ends_the_cast_and_the_stance_returns() {
        let mut body = CastBody {
            focus: 1.0,
            ..CastBody::default()
        };
        let (mut torrent, mut draws, mut roll) = live(&mut body);
        let mut ticks = 0;
        while torrent.torrent.state == CHARGING {
            tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
            ticks += 1;
            assert!(ticks < 200);
        }
        assert_eq!(torrent.torrent.state, RUNNING);
        let drain = torrent.torrent.drain * 0.5;
        body.spent = body.focus - drain * 0.5;
        tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
        assert_eq!(torrent.torrent.state, ENDING);
        assert_eq!((body.casting, body.cast_hook), (0, None));
        assert_eq!(body.condition, 0);
        assert!(casting::take_stance_return(&mut body), "the stance return");
    }

    #[test]
    fn a_running_torrent_keeps_the_hook_until_the_release_ends_it() {
        let mut body = CastBody {
            focus: 1.0,
            ..CastBody::default()
        };
        let (mut torrent, mut draws, mut roll) = live(&mut body);
        let mut ticks = 0;
        while torrent.torrent.state == CHARGING {
            tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
            ticks += 1;
            assert!(ticks < 200);
        }
        assert_eq!(torrent.torrent.state, RUNNING);
        assert!(torrent.hooked);
        assert_eq!(body.cast_hook, Some(TORRENT_HOOK));
        assert!(!torrent.torrent.builds_nothing());
        body.release = true;
        assert!(casting::release_cast(&mut body));
        tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
        assert_eq!(torrent.torrent.state, ENDING);
        assert_eq!(
            (body.casting, body.cast_hook, body.release),
            (0, None, false)
        );
        let mut ticks = 0;
        while !torrent.torrent.builds_nothing() {
            tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
            ticks += 1;
            assert!(ticks < 200);
        }
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

    /// A running torrent whose first stream's head alone lives, at `at`
    /// (cm) moving by `motion`, width 4.
    fn one_point(at: Vec3, motion: Vec3) -> Torrent {
        let mut body = CastBody::default();
        let (live, _, _) = live(&mut body);
        let mut torrent = live.torrent;
        torrent.margin = 0.0;
        for stream in &mut torrent.records {
            for point in &mut stream.points {
                point.life = 0;
            }
        }
        torrent.count = 1;
        let stream = &mut torrent.records[0];
        let point = &mut stream.points[stream.head];
        point.life = torrent.point_life / 2;
        point.at = at;
        point.previous = at - motion;
        point.width = 4.0;
        point.straightness = 0.5;
        torrent
    }

    #[test]
    fn a_part_contact_walks_the_parts_mesh_and_pushes_the_part() {
        let mut body = wall_body();
        let before = body.parts[0].positions;
        let mass = body_mass(&body);
        let mut torrent = one_point(Vec3::new(35.0, 0.0, 0.0), Vec3::new(-5.0, 0.0, 0.0));
        let volume = torrent.sounds.loop_volume;
        let (struck, impulse) = strike_torrent_part(&mut torrent, &mut body, 0, false, mass);
        assert!(struck);
        // The point 5 cm in front of the face is pushed out to its reach and
        // eases part of the way there.
        let inverse = body.parts[0].inverse_mass;
        let (scale, _, ease) = limb_terms(false, inverse, mass);
        let reach = 4.0 * scale + 10.0;
        let head = torrent.records[0].head;
        let at = torrent.records[0].points[head].at;
        let pushed = 30.0 + reach;
        assert!(
            (at.x - (35.0 + (pushed - 35.0) * ease)).abs() < 1e-3,
            "{at:?}"
        );
        assert!(impulse.x < 0.0, "{impulse:?}");
        let moved: f32 = body.parts[0]
            .positions
            .iter()
            .zip(before)
            .map(|(a, b)| a.x - b.x)
            .sum();
        assert!(moved < 0.0);
        assert!(torrent.sounds.loop_volume > volume);
        // Out of reach: nothing.
        let mut far = one_point(Vec3::new(50.0, 0.0, 0.0), Vec3::new(-5.0, 0.0, 0.0));
        assert!(!strike_torrent_part(&mut far, &mut body, 0, false, mass).0);
    }

    #[test]
    fn the_push_out_gathers_the_surface_and_the_push() {
        let mut gathered = Gathered::default();
        let mut sphere = (Vec3::new(5.0, 0.0, 0.0), 13.0);
        gather_push(&mut gathered, &mut sphere, Vec3::ZERO);
        assert!((sphere.0.x - 13.0).abs() < 1e-5);
        assert!((gathered.pushed.x + 8.0).abs() < 1e-5);
        assert_eq!((gathered.surface, gathered.count), (Vec3::ZERO, 1));
        // At the centre: nothing.
        let centre = sphere.0;
        gather_push(&mut gathered, &mut sphere, centre);
        assert_eq!(gathered.count, 1);
    }
}
