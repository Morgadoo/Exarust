//! Physical bodies. A body model made [`Physical`] becomes a simulated body
//! whose muscles follow its animation; one asked to [`GoLimp`] loses its
//! muscles (or becomes a ragdoll from its current pose) and stops being
//! animated and steered.
//!
//! Each fixed tick runs every living body's character tick before the body
//! solver, the player's last, as the original orders them: the body's clip
//! clock advances by one tick and its clip is sampled, and its targets are
//! the body root's world transform times the clip's pose.

use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_anim::{AnimSystems, Animator, BodyPart, ClipPose, Motions, Stature, TickedClips};
use exa_assets::SkinnedPart;
use exa_formats::rsq::BoneBody;
use exa_physics::CharacterBody;
use exa_physics::body::{
    BODY_HZ, Bearing, CharacterTicks, Fallen, GoLimp, PartSetup, Physical, RaceMuscles, Ragdoll,
    world_with,
};

use crate::player::PlayerBody;

pub struct LimpPlugin;

impl Plugin for LimpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (build_bodies, go_limp).chain().after(AnimSystems))
            .add_systems(FixedUpdate, tick_characters.in_set(CharacterTicks));
    }
}

/// Half extent of a part without a mesh, metres.
const FALLBACK_HALF: f32 = 0.05;

/// Builds the simulated bodies of physical and limp body models once they
/// are posed.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn build_bodies(
    mut commands: Commands,
    motions: Option<Res<Motions>>,
    mut requests: Query<
        (Entity, Option<&mut Physical>, Option<&mut GoLimp>),
        (Without<Ragdoll>, Or<(With<Physical>, With<GoLimp>)>),
    >,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    parts: Query<(&BodyPart, &GlobalTransform)>,
    locals: Query<&Transform>,
    skinned: Query<&SkinnedPart>,
    meshes: Res<Assets<Mesh>>,
) {
    let Some(motions) = motions else { return };
    for (root, physical, limp) in &mut requests {
        let race = physical.as_ref().map_or(RaceMuscles::HUMAN, |p| p.race);
        // Wait until the body has been placed and posed.
        let delay = match (physical, limp) {
            (_, Some(l)) => &mut l.into_inner().delay,
            (Some(p), None) => &mut p.into_inner().delay,
            (None, None) => continue,
        };
        if *delay > 0 {
            *delay -= 1;
            continue;
        }
        let found: Vec<(Entity, usize, GlobalTransform)> = children
            .iter_descendants(root)
            .filter_map(|e| parts.get(e).ok().map(|(p, g)| (e, p.bone, *g)))
            .collect();
        if found.is_empty() {
            continue;
        }
        let index: HashMap<Entity, usize> =
            found.iter().enumerate().map(|(i, f)| (f.0, i)).collect();
        let setup: Vec<PartSetup> = found
            .iter()
            .map(|&(entity, bone, world)| {
                let parent = parents
                    .iter_ancestors(entity)
                    .take_while(|&a| a != root)
                    .find_map(|a| index.get(&a).copied());
                let (centre, half) = part_box(entity, bone, &children, &skinned, &meshes);
                // The part's origin in its parent: the model's translation
                // of the part (rotations play no part in joining them).
                let origin = parent.map_or(Vec3::ZERO, |p| {
                    rest_under(entity, found[p].0, &parents, &parts, &locals)
                        .translation
                        .into()
                });
                part_setup(
                    bone,
                    entity,
                    parent,
                    world.affine(),
                    (centre, half),
                    origin,
                    motions.1.get(&bone),
                )
            })
            .collect();
        // A limp request is honoured by `go_limp` next. From now on the
        // body's ticks advance and sample its clips.
        commands
            .entity(root)
            .insert((
                Ragdoll::new(&setup).with_drive().with_race(race),
                TickedClips,
            ))
            .remove::<Physical>();
    }
}

/// What building a part's body needs: its links to its parent as the
/// skeleton stores them (the parent's points still measured from the part's
/// origin, which `origin` places in the parent), and its foot contact point
/// if it carries one.
pub(crate) fn part_setup(
    bone: usize,
    entity: Entity,
    parent: Option<usize>,
    world: Affine3A,
    (centre, half): (Vec3, Vec3),
    origin: Vec3,
    body: Option<&BoneBody>,
) -> PartSetup {
    let links = match (parent, body) {
        (Some(_), Some(body)) => body
            .links
            .iter()
            .map(|l| {
                (
                    Vec3::from_array(l.on_part),
                    Vec3::from_array(l.on_parent),
                    l.slack,
                )
            })
            .collect(),
        _ => Vec::new(),
    };
    PartSetup {
        bone,
        entity,
        parent,
        world,
        centre,
        half,
        origin,
        links,
        foot: body
            .and_then(|b| b.contacts.first())
            .map(|&c| Vec3::from_array(c)),
    }
}

/// Takes the muscles of bodies asked to go limp; they stop being animated
/// and their character stops being steered.
fn go_limp(
    mut commands: Commands,
    mut bodies: Query<(Entity, &GoLimp, &mut Ragdoll)>,
    parents: Query<&ChildOf>,
    controlled: Query<(), With<CharacterBody>>,
) {
    for (root, limp, mut body) in &mut bodies {
        body.go_limp();
        body.set_velocity(limp.velocity);
        commands
            .entity(root)
            .remove::<(GoLimp, Physical, Animator, TickedClips)>();
        if let Some(character) = parents
            .iter_ancestors(root)
            .find(|&a| controlled.contains(a))
        {
            commands.entity(character).remove::<CharacterBody>();
        }
    }
}

/// The character tick of every living physical body, the player's last:
/// its clip clock advances by one tick, its clip is sampled, and its
/// targets are set to the body root's world transform times the clip's pose
/// (the body is carried with the root's motion since the last tick). The
/// body learns what its character does ([`Bearing`]) first.
#[allow(clippy::type_complexity)]
fn tick_characters(
    motions: Option<Res<Motions>>,
    mut bodies: Query<
        (
            Entity,
            &mut Ragdoll,
            &mut Animator,
            Option<&Stature>,
            Option<&PlayerBody>,
            Has<Fallen>,
        ),
        With<TickedClips>,
    >,
    parents: Query<&ChildOf>,
    nodes: Query<(&Transform, Option<&BodyPart>)>,
    controllers: Query<&CharacterBody>,
    mut order: Local<Vec<(bool, Entity)>>,
) {
    let Some(motions) = motions else { return };
    order.clear();
    order.extend(
        bodies
            .iter()
            .map(|(e, _, _, _, player, _)| (player.is_some(), e)),
    );
    player_last(&mut order);
    for &(_, root) in order.iter() {
        let Ok((_, mut body, mut anim, stature, player, fallen)) = bodies.get_mut(root) else {
            continue;
        };
        if body.drive.is_none() {
            continue;
        }
        let controller = parents
            .iter_ancestors(root)
            .find_map(|a| controllers.get(a).ok());
        body.set_bearing(bearing(controller, player, &anim, fallen));
        let pose = clip_tick(&motions, &mut anim, stature.map_or(1.0, |s| s.0));
        let root_world = world_with(root, &parents, |e| {
            nodes.get(e).ok().map(|(t, _)| t.compute_affine())
        });
        match pose {
            Some(pose) => {
                let targets: Vec<Affine3A> = body
                    .parts
                    .iter()
                    .map(|p| root_world * clip_pose(p.entity, root, &pose, &parents, &nodes))
                    .collect();
                body.aim(root_world, targets);
            }
            None => body.hold(root_world),
        }
    }
}

/// What a body's character does this tick, for its muscles: it moves while
/// its controller is asked to, runs by the player's run blend, strides in
/// the walk and run cycles, is off balance from its fall until it has got
/// up, stands on the ground when its controller does (or has none), and
/// turns by what is left of the player's turn.
fn bearing(
    controller: Option<&CharacterBody>,
    player: Option<&PlayerBody>,
    anim: &Animator,
    fallen: bool,
) -> Bearing {
    Bearing {
        moving: controller.is_some_and(|c| c.desired_velocity != Vec3::ZERO),
        run: player.map_or(0.0, PlayerBody::run),
        striding: matches!(anim.action.as_str(), "Walk" | "Run"),
        off_balance: fallen,
        grounded: controller.is_none_or(|c| c.grounded),
        turn: player.map_or(0.0, PlayerBody::turn_left),
    }
}

/// Puts the player's body (flagged `true`) after every other, which keep
/// their order: the original ticks its characters from the last registered
/// to the first, the player.
fn player_last(order: &mut [(bool, Entity)]) {
    order.sort_by_key(|&(player, _)| player);
}

/// Advances `anim`'s clip clock by one body tick and samples its clip; a
/// clip that ends into its follow-up gives the follow-up's first pose.
fn clip_tick(motions: &Motions, anim: &mut Animator, stature: f32) -> Option<ClipPose> {
    let set = &*motions.0;
    let clip = set.clip(&anim.moveset, &anim.action)?;
    let clip = if anim.advance(clip, 1.0 / BODY_HZ) {
        clip
    } else {
        set.clip(&anim.moveset, &anim.action)?
    };
    ClipPose::at(clip, anim.time, stature)
}

/// Where `pose` puts `part` in the space of its body root `root`: the
/// part's posed local transform composed with its ancestors' up to the root
/// (body parts posed by the clip from their rest transforms, other nodes as
/// they are).
pub(crate) fn clip_pose(
    part: Entity,
    root: Entity,
    pose: &ClipPose,
    parents: &Query<&ChildOf>,
    nodes: &Query<(&Transform, Option<&BodyPart>)>,
) -> Affine3A {
    let mut placement = Affine3A::IDENTITY;
    let mut node = part;
    while node != root {
        let local = match nodes.get(node) {
            Ok((_, Some(bp))) => {
                let mut t = bp.rest;
                pose.place(bp, &mut t);
                t
            }
            Ok((t, None)) => *t,
            Err(_) => Transform::IDENTITY,
        };
        placement = local.compute_affine() * placement;
        match parents.get(node) {
            Ok(up) => node = up.parent(),
            Err(_) => break,
        }
    }
    placement
}

/// Rest placement of `part` in the space of its ancestor `parent`: body
/// parts contribute their rest transforms, other nodes their current ones.
/// Only its translation joins the part to its parent.
fn rest_under(
    part: Entity,
    parent: Entity,
    parents: &Query<&ChildOf>,
    parts: &Query<(&BodyPart, &GlobalTransform)>,
    locals: &Query<&Transform>,
) -> Affine3A {
    let mut placement = Affine3A::IDENTITY;
    let mut node = part;
    while node != parent {
        let local = match parts.get(node) {
            Ok((p, _)) => p.rest,
            Err(_) => locals.get(node).copied().unwrap_or_default(),
        };
        placement = local.compute_affine() * placement;
        match parents.get(node) {
            Ok(up) => node = up.parent(),
            Err(_) => break,
        }
    }
    placement
}

/// Box of a part's own skinned body meshes, in its space.
fn part_box(
    part: Entity,
    bone: usize,
    children: &Query<&Children>,
    skinned: &Query<&SkinnedPart>,
    meshes: &Assets<Mesh>,
) -> (Vec3, Vec3) {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for child in children.get(part).into_iter().flat_map(|c| c.iter()) {
        let Ok(s) = skinned.get(child) else {
            continue;
        };
        if s.bone as usize != bone {
            continue;
        }
        let Some(positions) = meshes
            .get(&s.mesh)
            .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION))
            .and_then(|a| a.as_float3())
        else {
            continue;
        };
        for p in positions {
            lo = lo.min(Vec3::from(*p));
            hi = hi.max(Vec3::from(*p));
        }
    }
    if lo.x > hi.x {
        return (Vec3::ZERO, Vec3::splat(FALLBACK_HALF));
    }
    ((lo + hi) * 0.5, (hi - lo) * 0.5)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
    use exa_physics::Physics;
    use exa_physics::body::{BodyPlugin, BodySystems};

    use super::*;

    /// Where the character starts, and how far it moves each tick.
    const START: Vec3 = Vec3::new(4.0, 0.0, -3.0);
    const STRIDE: f32 = 0.01;
    /// The body root's turn under the character.
    const YAW: f32 = 0.5;
    const PELVIS_REST: Vec3 = Vec3::new(0.0, 1.0, 0.0);
    const THIGH_REST: Vec3 = Vec3::new(0.1, -0.1, 0.0);

    /// The thigh's swing and the root's height of the looping "Idle" clip
    /// at `time` frames: 0.6 rad and 5 cm up at frame 15 of 30.
    fn idle_at(time: f32) -> (f32, f32) {
        let f = 1.0 - (time - 15.0).abs() / 15.0;
        (0.6 * f, 1.0 + 0.05 * f)
    }

    /// A two-bone motion set whose "Idle" swings the thigh and bobs the
    /// root over 30 frames.
    pub(crate) fn motions() -> Motions {
        let key = |time: f32| {
            let (swing, y) = idle_at(time);
            let q = |r: Quat| [r.x, r.y, r.z, r.w];
            Key {
                time,
                root: [0.0, y, 0.0],
                rotations: vec![q(Quat::IDENTITY), q(Quat::from_rotation_x(swing))],
                flags: vec![0, 0],
            }
        };
        let idle = Clip {
            mode: 3,
            keys: vec![key(0.0), key(15.0), key(30.0)],
        };
        let set = MotionSet {
            bone_count: 2,
            actions: vec![Action {
                name: "Idle".into(),
                params: [0, 0],
            }],
            movesets: vec![Moveset {
                name: "Default".into(),
                parent: String::new(),
                clips: vec![Some(idle)],
            }],
        };
        Motions(Arc::new(set), Arc::new(HashMap::new()), 0.0)
    }

    /// Moves the character as its controller would, once a tick.
    #[derive(Component)]
    struct Walker;

    fn walk(mut characters: Query<&mut Transform, With<Walker>>) {
        for mut t in &mut characters {
            t.translation.x += STRIDE;
        }
    }

    /// An app ticking a physical body that plays "Idle" under a walking
    /// character, `frame` seconds per update; returns it and the body root.
    pub(crate) fn app(frame: f32) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BodyPlugin))
            .insert_resource(Time::<Fixed>::from_hz(exa_physics::STEP_HZ))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                frame,
            )))
            .insert_resource(Physics::default())
            .insert_resource(motions())
            .add_systems(FixedPreUpdate, walk)
            .add_systems(FixedUpdate, tick_characters.in_set(CharacterTicks));
        let world = app.world_mut();
        let character = world
            .spawn((Transform::from_translation(START), Walker))
            .id();
        let root = spawn_body(world, character);
        (app, root)
    }

    /// A physical body playing "Idle" under `character`, which stands at
    /// [`START`], its root turned by [`YAW`]; returns the body root.
    fn spawn_body(world: &mut World, character: Entity) -> Entity {
        let root = world
            .spawn((
                Transform::from_rotation(Quat::from_rotation_y(YAW)),
                ChildOf(character),
            ))
            .id();
        let rest = |at: Vec3, bone: usize| {
            let t = Transform::from_translation(at);
            (t, BodyPart { bone, rest: t })
        };
        let pelvis = world.spawn((rest(PELVIS_REST, 1), ChildOf(root))).id();
        let thigh = world.spawn((rest(THIGH_REST, 2), ChildOf(pelvis))).id();
        let placed = Affine3A::from_translation(START) * Affine3A::from_rotation_y(YAW);
        let pelvis_world = placed * Affine3A::from_translation(PELVIS_REST);
        let setup = [
            PartSetup {
                bone: 1,
                entity: pelvis,
                parent: None,
                world: pelvis_world,
                centre: Vec3::ZERO,
                half: Vec3::new(0.15, 0.1, 0.1),
                origin: Vec3::ZERO,
                links: vec![],
                foot: None,
            },
            // Pinned at its origin, the hip.
            PartSetup {
                bone: 2,
                entity: thigh,
                parent: Some(0),
                world: pelvis_world * Affine3A::from_translation(THIGH_REST),
                centre: Vec3::new(0.0, -0.2, 0.0),
                half: Vec3::new(0.07, 0.25, 0.07),
                origin: THIGH_REST,
                links: vec![],
                foot: None,
            },
        ];
        world.entity_mut(root).insert((
            Ragdoll::new(&setup).with_drive(),
            Animator::new("Default", "Idle"),
            TickedClips,
        ));
        root
    }

    /// The human body's joints from the installed `humanoid.rsq` and
    /// `humanbase.rfc` (skipped without the game), built as the game builds
    /// them: every part takes its stored links and its origin pivot (a
    /// femur 3, a hand 4); every stored parent point is as far from the
    /// part's origin as its part's point (it is already turned into the
    /// parent's axes); and the parent points are the stored ones plus the
    /// model's translation of the part, unturned.
    #[test]
    fn shipped_links_join_at_the_model_origins() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let data = Arc::new(exa_assets::GameData::open(&dir).unwrap());
        let motions = Motions::load(&exa_assets::Game(data.clone())).expect("humanoid.rsq");
        let bytes = data
            .pack(exa_assets::PackKind::Resource)
            .and_then(|p| p.get(exa_assets::model::BODY_MODEL))
            .expect("humanbase.rfc");
        let scene = exa_formats::rfc::Scene::parse(bytes).unwrap();
        let bone_of = |i: usize| exa_formats::rsq::bone_index(&scene.nodes[i].name);
        let nodes: Vec<usize> = (0..scene.nodes.len())
            .filter(|&i| bone_of(i).is_some())
            .collect();
        assert_eq!(nodes.len(), 18);
        let setup: Vec<PartSetup> = nodes
            .iter()
            .map(|&n| {
                let bone = bone_of(n).unwrap();
                // The nearest ancestor node that is a part.
                let mut up = scene.nodes[n].parent.map(|p| p - 1);
                while let Some(u) = up.filter(|&u| bone_of(u).is_none()) {
                    up = scene.nodes[u].parent.map(|p| p - 1);
                }
                let parent = up.and_then(|u| nodes.iter().position(|&x| x == u));
                let origin = Vec3::from(scene.nodes[n].transform[3]);
                part_setup(
                    bone,
                    Entity::PLACEHOLDER,
                    parent,
                    Affine3A::IDENTITY,
                    (Vec3::ZERO, Vec3::splat(FALLBACK_HALF)),
                    origin,
                    motions.1.get(&bone),
                )
            })
            .collect();
        let body = Ragdoll::new(&setup);
        let index = |bone: usize| setup.iter().position(|s| s.bone == bone).unwrap();
        for (i, s) in setup.iter().enumerate() {
            let stored = motions.1.get(&s.bone).map_or(0, |b| b.links.len());
            let want = if s.parent.is_some() { stored + 1 } else { 0 };
            assert_eq!(body.links_of(i).len(), want, "bone {}", s.bone);
            for &(on_part, on_parent, _) in &s.links {
                assert!(
                    (on_part.length() - on_parent.length()).abs() < 1e-4,
                    "bone {}: {on_part} against {on_parent}",
                    s.bone
                );
            }
        }
        for bone in [2, 5] {
            assert_eq!(body.links_of(index(bone)).len(), 3, "femur {bone}");
        }
        for bone in [17, 18] {
            assert_eq!(body.links_of(index(bone)).len(), 4, "hand {bone}");
        }
        let cm = |x: f32, y: f32, z: f32| Vec3::new(x, y, z) * 0.01;
        let parent_point = |bone: usize, k: usize| {
            let links = body.links_of(index(bone));
            let k = k.min(links.len() - 1);
            body.link_points(&links[k]).1
        };
        // The right humerus's first link: stored (−9.42, −8.6, 7.89) plus
        // its translation (−12.16, 0.04, −1.38) in the shoulder.
        let humerus = parent_point(12, 0);
        assert!(
            humerus.abs_diff_eq(cm(-21.58, -8.56, 6.51), 5e-4),
            "{}",
            humerus * 100.0
        );
        // The pivots of the right hip and knee.
        let hip = parent_point(2, usize::MAX);
        assert!(
            hip.abs_diff_eq(cm(-9.07, -8.46, 3.5), 5e-4),
            "{}",
            hip * 100.0
        );
        let knee = parent_point(3, usize::MAX);
        assert!(
            knee.abs_diff_eq(cm(-0.3, -43.19, -1.09), 5e-4),
            "{}",
            knee * 100.0
        );
        // The lower legs carry the foot contacts, so their long links limit
        // the legs.
        for bone in [3, 6] {
            let k = body.parts[index(bone)].leg_limit.expect("leg limit");
            assert_eq!(body.links[k].strength, 0.2);
            assert!(body.links[k].slack > 0.12);
        }
        // Limp, every slack link damps its length with a smoothing from
        // 0.640 (the humerus's point 15 cm out) to 0.914 (8.32 cm out): a
        // link closing in full keeps (1 − s)/(2 − s) of a stretch a tick,
        // 0.27 to 0.08.
        let smoothing: Vec<f32> = body
            .links
            .iter()
            .filter(|l| l.slack > 0.0)
            .map(|l| l.smoothing)
            .collect();
        let (least, most) = smoothing
            .iter()
            .fold((1.0f32, 0.0f32), |(a, b), &s| (a.min(s), b.max(s)));
        assert!((least - 0.640).abs() < 1e-3, "{smoothing:?}");
        assert!((most - 0.914).abs() < 1e-3, "{smoothing:?}");
        let kept = |s: f32| (1.0 - s) / (2.0 - s);
        assert!((kept(least) - 0.265).abs() < 1e-3 && (kept(most) - 0.079).abs() < 1e-3);
    }

    #[test]
    fn the_player_is_ticked_last() {
        let e = |i: u32| Entity::from_raw_u32(i).unwrap();
        let mut order = vec![(false, e(3)), (true, e(1)), (false, e(7)), (false, e(2))];
        player_last(&mut order);
        let ticked: Vec<Entity> = order.iter().map(|&(_, x)| x).collect();
        assert_eq!(ticked, [e(3), e(7), e(2), e(1)]);
    }

    /// Each tick's clip clock and targets.
    #[derive(Resource, Default)]
    struct Ticks(Vec<(f32, Vec<Affine3A>)>);

    fn record(mut ticks: ResMut<Ticks>, bodies: Query<(&Animator, &Ragdoll)>) {
        for (anim, body) in &bodies {
            let targets = body.drive.as_ref().map(|d| d.targets().to_vec());
            ticks.0.push((anim.time, targets.unwrap_or_default()));
        }
    }

    fn run_at(fps: f32, count: usize) -> Vec<(f32, Vec<Affine3A>)> {
        let (mut app, _) = app(1.0 / fps);
        app.init_resource::<Ticks>()
            .add_systems(FixedUpdate, record.after(BodySystems));
        while app.world().resource::<Ticks>().0.len() < count {
            app.update();
        }
        let mut ticks = app.world_mut().remove_resource::<Ticks>().unwrap().0;
        ticks.truncate(count);
        ticks
    }

    /// A physical body's clip clock advances by one tick's worth of frames
    /// in every tick, whatever the frame rate, and each tick's targets are
    /// the body root's world transform (where the character's controller
    /// put it that tick) times the clip's pose at that tick.
    #[test]
    fn clips_are_sampled_every_tick_and_placed_on_the_root() {
        let count = 60;
        let reference = run_at(60.0, count);
        for fps in [30.0, 144.0, 240.0] {
            assert_eq!(run_at(fps, count), reference, "{fps} FPS");
        }
        for (k, (time, targets)) in reference.iter().enumerate() {
            let tick = (k + 1) as f32;
            let clock = (tick * 60.0 / BODY_HZ) % 30.0;
            assert!((time - clock).abs() < 1e-3, "tick {tick}: clock {time}");
            let (swing, y) = idle_at(*time);
            let root = Affine3A::from_translation(START + Vec3::X * STRIDE * tick)
                * Affine3A::from_rotation_y(YAW);
            let pelvis = root * Affine3A::from_translation(Vec3::Y * y);
            let thigh =
                pelvis * Affine3A::from_translation(THIGH_REST) * Affine3A::from_rotation_x(swing);
            for (got, want) in targets.iter().zip([pelvis, thigh]) {
                assert!(
                    got.abs_diff_eq(want, 1e-4),
                    "tick {tick}: target {got:?}, clip pose {want:?}"
                );
            }
        }
    }

    /// Where each tick leaves the character (read inside the tick) and the
    /// particles of its body.
    #[derive(Resource, Default)]
    struct Walked(Vec<(Vec3, Vec<[Vec3; 4]>)>);

    fn record_walk(
        mut walked: ResMut<Walked>,
        characters: Query<&Transform, With<CharacterBody>>,
        bodies: Query<&Ragdoll>,
    ) {
        for (t, body) in characters.iter().zip(&bodies) {
            let particles = body.parts.iter().map(|p| p.positions).collect();
            walked.0.push((t.translation, particles));
        }
    }

    /// Steers the character as the player's steering does, once a tick
    /// before the controllers: it walks, turning its body, then stops.
    fn steer(
        mut ticks: Local<u32>,
        mut characters: Query<&mut CharacterBody>,
        mut roots: Query<&mut Transform, With<Ragdoll>>,
    ) {
        if characters.is_empty() {
            return;
        }
        *ticks += 1;
        for mut c in &mut characters {
            c.desired_velocity = if *ticks < 80 {
                Vec3::new(1.4, 0.0, 0.4)
            } else {
                Vec3::ZERO
            };
        }
        for mut t in &mut roots {
            t.rotate_y(0.02);
        }
    }

    /// A character walking on a floor under the game's controller and
    /// physics plugins at `fps`: the first `count` ticks.
    fn walk_at(fps: f32, count: usize) -> Vec<(Vec3, Vec<[Vec3; 4]>)> {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, exa_physics::PhysicsPlugin))
            .init_resource::<Assets<Mesh>>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                1.0 / fps,
            )))
            .insert_resource(motions())
            .init_resource::<Walked>()
            .add_systems(FixedPreUpdate, steer)
            .add_systems(FixedUpdate, tick_characters.in_set(CharacterTicks))
            .add_systems(FixedUpdate, record_walk.after(BodySystems));
        // The floor joins the level before the character arrives.
        let floor = Plane3d::default().mesh().size(40.0, 40.0);
        let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(floor);
        let level = app
            .world_mut()
            .spawn((exa_physics::StaticGeometry, Transform::default()))
            .id();
        app.world_mut()
            .spawn((Mesh3d(mesh), GlobalTransform::default(), ChildOf(level)));
        for _ in 0..5000 {
            if !app
                .world()
                .entity(level)
                .contains::<exa_physics::StaticGeometry>()
            {
                break;
            }
            app.update();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            !app.world()
                .entity(level)
                .contains::<exa_physics::StaticGeometry>()
        );
        let character = app
            .world_mut()
            .spawn((Transform::from_translation(START), CharacterBody::default()))
            .id();
        let root = spawn_body(app.world_mut(), character);
        let mut between = 0;
        while app.world().resource::<Walked>().0.len() < count {
            app.update();
            // Each part is drawn where its last two ticks put it, under the
            // drawn (interpolated) character: interpolated once, not twice.
            let world = app.world();
            let alpha = world.resource::<Time<Fixed>>().overstep_fraction();
            let body = world.get::<Ragdoll>(root).unwrap();
            for part in &body.parts {
                let mut drawn = Affine3A::IDENTITY;
                let mut node = Some(part.entity);
                while let Some(n) = node {
                    drawn = world.get::<Transform>(n).unwrap().compute_affine() * drawn;
                    node = world.get::<ChildOf>(n).map(ChildOf::parent);
                }
                let want = part.drawn(alpha);
                assert!(
                    drawn.translation.abs_diff_eq(want.translation, 1e-4),
                    "bone {} drawn at {}, not {}",
                    part.bone,
                    drawn.translation,
                    want.translation
                );
                if !want.abs_diff_eq(part.drawn(1.0), 1e-4) {
                    between += 1;
                }
            }
        }
        assert!(between > 0, "nothing was drawn between ticks");
        let mut walked = app.world_mut().remove_resource::<Walked>().unwrap().0;
        walked.truncate(count);
        walked
    }

    /// The controller moves the character first in each tick and holds its
    /// simulated position while the character tick carries the body, so
    /// the character and every particle of its body land, tick after tick,
    /// exactly where they do at any other frame rate (a drawn, interpolated
    /// position read inside the tick would differ with the frame rate).
    #[test]
    fn a_walking_controller_ticks_the_same_at_any_frame_rate() {
        let count = 120;
        let reference = walk_at(60.0, count);
        let (start, end) = (reference[0].0, reference[count - 1].0);
        assert!(end.x - start.x > 0.8, "walked from {start} to {end}");
        assert!(end.y.abs() < 0.05, "on the floor: {end}");
        // The body stays with the character it is carried by.
        let pelvis = reference[count - 1].1[0][0];
        assert!(
            (pelvis - end).xz().length() < 0.4,
            "pelvis {pelvis}, feet {end}"
        );
        for fps in [30.0, 144.0, 240.0] {
            let other = walk_at(fps, count);
            let first = reference.iter().zip(&other).position(|(a, b)| a != b);
            assert_eq!(
                first, None,
                "at {fps} FPS the walk parts from 60 FPS at tick"
            );
        }
    }
}
