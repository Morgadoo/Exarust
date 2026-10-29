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
//!   Once running (flags 0xb_0000), its live points push the limb spheres
//!   of other bodies they reach ([`strike_streams`]), and a struck
//!   character takes [`torrent_knock`] of the gathered impulse once a tick:
//!   the energy, a knockback step, its instability counter +1 and concern
//!   1 at 0.01 toward the caster. The live points also meet the level
//!   (a swept ball at the static reach) and move half way to the contact.
//! - **Leaving**: the frame that would build no triangles removes it
//!   ([`Torrent::builds_nothing`]): fizzled, or every stream spent.
//!
//! [inf: the frame's turned aim is the blast aim here (the player
//! behaviour's turn spring is not wired), and the contact is Exarust's
//! sphere test against a point's reach (its width times the limb scale,
//! plus the margin, plus 10), as for the blast.] The stream ribbon draws
//! with the glow stand-in ([`draw_torrents`]). Not wired: the support-pin
//! gain and the 5 s recoil the hit sets (no body field takes them), the
//! simulated objects' points, the sounds and the light.
//!
//! Evidence: the torrent's creator, start, hook, tick and frame, read from
//! `Exanima.exe`.

use bevy::prelude::*;
use exa_ai::brain::Npc;
use exa_ai::combatant::Combatant;
use exa_physics::Physics;
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::Locomotor;

use super::barricade::WAKE_TICKS;
use super::blast_live::{
    LimbSphere, blast_view, body_mass, bounds_miss, gather_limbs, most_reach, reach_box,
    refresh_limbs,
};
use super::cast::PowerCaster;
use super::casting::{self, CastBody, CastHook, spend_tick};
use super::live::{EffectDraws, EffectMesh, EffectRoll, mesh_of_ribbon};
use crate::effects::blast::{EPSILON, HIT_HOSTILITY, blast_aim};
use crate::effects::torrent::{
    CONTACT_EASE, POINTS, RUNNING_FLAGS, STATIC_CONTACT, Stream, StreamPoint, TORRENT_MOTION,
    Torrent, TorrentEvents, TorrentView, limb_impulse, limb_terms, torrent_knock, torrent_reach,
};
use crate::player::CursorGround;
use crate::thaum::engine::lerp;

/// Centimetres a metre.
const CM: f32 = 100.0;
/// The torrent's power id.
pub const TORRENT: u32 = 0x10203;
/// The torrent's hook: −1 to every event.
pub const TORRENT_HOOK: CastHook = CastHook::NEVER_KEEPS;

/// A torrent in the scene.
#[derive(Component)]
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

/// The level's static contact: every live point is swept as a ball of its
/// static reach and one that meets the level moves half way to the
/// surface. [inf: the original walks the level's static faces; Exarust
/// asks rapier for the swept ball.]
fn strike_level(torrent: &mut Torrent, physics: &Physics) -> usize {
    let mut hits = 0;
    for index in 0..torrent.count {
        let head = torrent.records[index].head;
        let mut k = head;
        loop {
            let point = torrent.records[index].points[k];
            if point.life < 1 {
                break;
            }
            let reach = torrent_reach(point.width, STATIC_CONTACT.scale, torrent.margin);
            let motion = point.at - point.previous;
            let squared = (motion.x * motion.x + motion.y * motion.y) + motion.z * motion.z;
            if EPSILON < squared {
                let length = squared.sqrt();
                let direction = motion * (1.0 / length);
                if let Some(toi) =
                    physics.level_sweep(point.previous / CM, direction, length / CM, reach / CM)
                {
                    let contact = point.previous + direction * (toi * CM);
                    let point = &mut torrent.records[index].points[k];
                    point.at = lerp(point.at, contact, CONTACT_EASE);
                    hits += 1;
                }
            }
            k = if k == 0 { POINTS - 1 } else { k - 1 };
            if k == head {
                break;
            }
        }
    }
    hits
}

/// One tick of a running torrent's live points against `body`'s limb
/// spheres. `limbs` is the caller's scratch buffer; a body whose limbs' box
/// misses the live points' box grown by their most reach is skipped (the
/// points do not move here). Returns the pushes made and the impulse a
/// character gathered.
pub(crate) fn strike_streams(
    torrent: &Torrent,
    body: &mut Ragdoll,
    limbs: &mut Vec<LimbSphere>,
) -> (usize, Vec3) {
    let Some(effect) = reach_box(torrent.records[..torrent.count].iter().flat_map(|stream| {
        live_points(stream).map(|point| {
            let reach = most_reach(|scale| torrent_reach(point.width, scale, torrent.margin));
            (point.at, reach)
        })
    })) else {
        return (0, Vec3::ZERO);
    };
    let (low, high) = gather_limbs(body, limbs);
    if bounds_miss(low, high, effect) {
        return (0, Vec3::ZERO);
    }
    let character = body.drive.is_some();
    let mass = body_mass(body);
    let mut pushes = 0;
    let mut gathered = Vec3::ZERO;
    for stream in &torrent.records[..torrent.count] {
        for &point in live_points(stream) {
            let share = point.life as f32 / torrent.point_life as f32;
            let mut start = 0;
            while start < limbs.len() {
                let part_index = limbs[start].part;
                let mut end = start;
                while end < limbs.len() && limbs[end].part == part_index {
                    end += 1;
                }
                let inverse = limbs[start].inverse_mass;
                let (scale, push, _) = limb_terms(character, inverse, mass);
                let reach = torrent_reach(point.width, scale, torrent.margin);
                let mut pushed = false;
                for limb in &limbs[start..end] {
                    let (centre, radius, velocity, sphere) =
                        (limb.centre, limb.radius, limb.velocity, limb.sphere);
                    let d = point.at - centre;
                    let distance = d.length();
                    if distance >= reach + radius || distance <= 0.0 || distance.is_nan() {
                        continue;
                    }
                    let contact = centre + d * (radius / distance);
                    let impulse = limb_impulse(&point, contact, velocity, inverse, push, share);
                    if !impulse.is_finite() || impulse == Vec3::ZERO {
                        continue;
                    }
                    body.push_sphere(part_index, sphere, impulse / CM);
                    if character {
                        gathered += impulse;
                    }
                    pushes += 1;
                    pushed = true;
                }
                if pushed {
                    refresh_limbs(body, part_index, limbs, start, end);
                }
                start = end;
            }
        }
    }
    (pushes, gathered)
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
    mut limbs: Local<Vec<LimbSphere>>,
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
            if let Some(physics) = physics.as_deref() {
                strike_level(&mut live.torrent, physics);
            }
            for (other, mut body) in &mut bodies {
                if other == live.caster {
                    continue;
                }
                let (pushes, impulse) = strike_streams(&live.torrent, &mut body, &mut limbs);
                if pushes == 0 {
                    continue;
                }
                body.touch(WAKE_TICKS);
                let mass = body_mass(&body);
                let Some(knock) = body.drive.as_mut().map(|drive| {
                    let knock = torrent_knock(mass, drive.energy, impulse);
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

    /// The strike before its body transforms were shared and its bodies
    /// skipped by their boxes, kept to pin the new one to it.
    fn strike_streams_unshared(torrent: &Torrent, body: &mut Ragdoll) -> (usize, Vec3) {
        let character = body.drive.is_some();
        let mass = body_mass(body);
        let mut pushes = 0;
        let mut gathered = Vec3::ZERO;
        for stream in &torrent.records[..torrent.count] {
            let mut k = stream.head;
            loop {
                let point = stream.points[k];
                if point.life < 1 {
                    break;
                }
                let share = point.life as f32 / torrent.point_life as f32;
                for part_index in 0..body.parts.len() {
                    let part = &body.parts[part_index];
                    let inverse = part.inverse_mass;
                    let (scale, push, _) = limb_terms(character, inverse, mass);
                    let reach = torrent_reach(point.width, scale, torrent.margin);
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
                        let d = point.at - centre;
                        let distance = d.length();
                        if distance >= reach + radius || distance <= 0.0 || distance.is_nan() {
                            continue;
                        }
                        let contact = centre + d * (radius / distance);
                        let impulse = limb_impulse(&point, contact, velocity, inverse, push, share);
                        if !impulse.is_finite() || impulse == Vec3::ZERO {
                            continue;
                        }
                        body.push_sphere(part_index, s, impulse / CM);
                        if character {
                            gathered += impulse;
                        }
                        pushes += 1;
                    }
                }
                k = if k == 0 { POINTS - 1 } else { k - 1 };
                if k == stream.head {
                    break;
                }
            }
        }
        (pushes, gathered)
    }

    #[test]
    fn the_shared_strike_matches_the_unshared_one_to_the_bit() {
        use crate::thaum::blast_live::test_bodies::{
            between, near, same, same_particles, scattered,
        };
        let mut random = Mt19937::new(11);
        let (mut struck, mut skipped) = (0, 0);
        for round in 0..200 {
            let mut body = CastBody::default();
            let (mut live, _, _) = live(&mut body);
            let torrent = &mut live.torrent;
            torrent.margin = between(&mut random, 0.0, 20.0);
            let count = torrent.count;
            for (index, stream) in torrent.records[..count].iter_mut().enumerate() {
                stream.head = (random.unit() * POINTS as f64) as usize % POINTS;
                // The first stream of a round lives round its whole ring.
                let living = if index == 0 {
                    POINTS
                } else {
                    (random.unit() * POINTS as f64) as usize
                };
                for point in &mut stream.points {
                    point.life = 0;
                }
                for n in 0..living {
                    let point = &mut stream.points[(stream.head + POINTS - n) % POINTS];
                    point.life = 1 + (random.unit() * 40.0) as i32;
                    point.at = near(&mut random, Vec3::new(0.0, 100.0, 0.0), 150.0);
                    point.previous = point.at - near(&mut random, Vec3::ZERO, 20.0);
                    point.width = between(&mut random, 1.0, 30.0);
                }
            }
            for _ in 0..3 {
                let mut old_body = scattered(&mut random, Vec3::new(0.0, 1.0, 0.0), 3.0);
                let mut new_body = old_body.clone();
                let mut limbs = Vec::new();
                let old = strike_streams_unshared(&live.torrent, &mut old_body);
                let new = strike_streams(&live.torrent, &mut new_body, &mut limbs);
                assert_eq!(old.0, new.0, "round {round}");
                assert!(same(old.1, new.1), "round {round}");
                assert!(same_particles(&old_body, &new_body), "round {round}");
                if old.0 > 0 {
                    struck += 1;
                } else {
                    skipped += 1;
                }
            }
        }
        assert!(
            struck > 50 && skipped > 50,
            "{struck} struck, {skipped} not"
        );
    }
}
