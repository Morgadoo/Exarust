//! The flames as simulation objects, with the real physics and flame
//! plugins: their step in the solver's object stage, their body pass, a
//! carried flame's ride on its part, and the frame's draw of the step's
//! snapshot in one world, in lockstep behind the bridge and free of it.
use std::time::Duration;

use bevy::math::Affine3A;
use bevy::time::TimeUpdateStrategy;
use exa_physics::body::{PartSetup, Sleep};
use exa_physics::sim_app::{SimMode, SimPause, SimShellPlugin};
use exa_physics::sim_bridge::SimBridgePlugin;

use super::*;
use crate::fire::{self, Fire};
use crate::particle_stage::Dose;

/// The doses the solver's particle stage made, taken after the solver.
#[derive(Resource, Default)]
struct Taken(Vec<Dose>);

fn take(mut stages: ResMut<SolverStages>, mut taken: ResMut<Taken>) {
    if let Some(stage) = stages.get_mut::<ParticleSolve>() {
        taken.0.extend(stage.take_doses());
    }
}

/// A cube's twelve triangles of half extent `h` (m), wound outwards.
fn cube(h: f32) -> Vec<[Vec3; 3]> {
    let mut out = Vec::new();
    for axis in 0..3 {
        for sign in [-1.0f32, 1.0] {
            let n = Vec3::AXES[axis] * sign;
            let u = Vec3::AXES[(axis + 1) % 3];
            let v = Vec3::AXES[(axis + 2) % 3];
            let (u, v) = if sign > 0.0 { (u, v) } else { (v, u) };
            let c = n * h;
            out.push([c - u * h - v * h, c + u * h - v * h, c + u * h + v * h]);
            out.push([c - u * h - v * h, c + u * h + v * h, c - u * h + v * h]);
        }
    }
    out
}

/// A one-part body (a lumbar part, bone 8) at `at` (m) with a 30 cm cube
/// node mesh; its part's entity carries the part's transform.
fn body(app: &mut App, at: Vec3, sleep: Sleep) -> (Entity, Entity) {
    let part = app
        .world_mut()
        .spawn((
            Transform::from_translation(at),
            GlobalTransform::from_translation(at),
        ))
        .id();
    let mut ragdoll = Ragdoll::new(&[PartSetup {
        bone: 8,
        entity: part,
        parent: None,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.15),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: Some(vec![(Vec3::ZERO, 0.15)]),
    }]);
    ragdoll.set_part_triangles(0, cube(0.15));
    ragdoll.sleep = sleep;
    let root = app
        .world_mut()
        .spawn((ragdoll, Transform::IDENTITY, GlobalTransform::IDENTITY))
        .id();
    (root, part)
}

/// A fire of `size` under a node at `at` (m), as a level's fire marker
/// spawns it.
fn fire_at(app: &mut App, at: Vec3, size: f32) -> Entity {
    let node = app
        .world_mut()
        .spawn((
            Transform::from_translation(at),
            GlobalTransform::from_translation(at),
        ))
        .id();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    fire::spawn_fire(&mut commands, node, size);
    queue.apply(app.world_mut());
    node
}

/// The flames under `node`, the flame (the one with the fire's light)
/// first, then its bed.
fn flames_of(app: &mut App, node: Entity) -> Vec<Entity> {
    let mut flames: Vec<(bool, Entity)> = app
        .world_mut()
        .query_filtered::<(Entity, &ChildOf, Has<Fire>), With<Flame>>()
        .iter(app.world())
        .filter(|(_, c, _)| c.parent() == node)
        .map(|(e, _, fire)| (!fire, e))
        .collect();
    flames.sort();
    flames.into_iter().map(|(_, e)| e).collect()
}

/// The real plugins at sixty frames a second of game time, in one world
/// or behind the bridge.
fn plugins(mode: Option<SimMode>) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin))
        .add_plugins((exa_physics::PhysicsPlugin, FlamePlugin))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<ModelCache>()
        .init_resource::<Assets<exa_render::material::ExaMaterial>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<FlameMaterial>>();
    if let Some(mode) = mode {
        use exa_physics::sim_bridge::SimBridgeAppExt;
        app.add_plugins((
            SimShellPlugin { mode },
            SimBridgePlugin { extra: Vec::new() },
        ))
        // The physics' fixed systems touch these too; the game's other
        // plugins register them.
        .bridge_snapshot::<exa_physics::body::plate_body::PlateBody>()
        .bridge_copy::<exa_physics::body::slide_body::SlideBody>()
        .bridge_copy::<exa_physics::grip::GripRelease>()
        .bridge_copy::<exa_physics::prop::PropBody>()
        .bridge_copy_in_resource::<exa_physics::prop::StandIns>()
        .bridge_copy::<exa_physics::weapon_mass::WeaponMass>();
    }
    if mode != Some(SimMode::Free) {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            Duration::from_secs_f64(1.0 / 60.0),
        ));
    }
    app
}

/// A medium fire's flame (dose 1) meets a sleeping body above it in the
/// solver's object stage, each tick of the fixed step; its bed meets none
/// (its class bit is cleared). Both step in the solver, a tick each.
#[test]
fn the_fires_step_in_the_solver_and_dose_the_bodies_above() {
    let mut app = plugins(None);
    app.init_resource::<Taken>()
        .add_systems(FixedUpdate, take.after(BodySystems));
    let node = fire_at(&mut app, Vec3::ZERO, 0.8);
    let (_, part) = body(&mut app, Vec3::new(0.0, 0.35, 0.0), Sleep::Deep);
    for _ in 0..240 {
        app.update();
    }
    let flames = flames_of(&mut app, node);
    assert_eq!(flames.len(), 2);
    let world = app.world();
    let (flame, bed) = (
        world.get::<Flame>(flames[0]).unwrap(),
        world.get::<Flame>(flames[1]).unwrap(),
    );
    assert!(flame.contacts() && !bed.contacts());
    // 240 frames of 1/60 s at 83 ticks a second.
    let ticks = flame.emitter().ticks;
    assert!((330..=333).contains(&ticks), "{ticks} ticks");
    assert_eq!(bed.emitter().ticks, ticks);
    assert!(flame.emitter().live() > 10);
    let fire = world.get::<Fire>(flames[0]).unwrap();
    assert_eq!(fire.ticks, ticks as u32);
    assert!(fire.settled() && fire.density > 0.0);
    // The flame's particles met the part, every dose a whole number of
    // the template's 1.0.
    let doses = &world.resource::<Taken>().0;
    assert!(!doses.is_empty());
    assert!(doses.iter().all(|d| d.part == part && d.skin_bone == 7));
    assert!(doses.iter().all(|d| d.dose.fract() == 0.0));
    assert!(doses.iter().map(|d| d.dose).sum::<f32>() > 0.0);
}

/// A carried flame rides its body part: the emitter's node is the part's
/// pose in the solver, tick by tick, while the body falls, and its own part
/// is never met.
#[test]
fn a_carried_flame_rides_its_part_in_the_solver() {
    let mut app = plugins(None);
    app.init_resource::<Taken>()
        .add_systems(FixedUpdate, take.after(BodySystems));
    let start = Vec3::new(0.0, 2.0, 0.0);
    let (root, part) = body(&mut app, start, Sleep::Awake);
    app.world_mut().entity_mut(part).insert((CarriedFire, ChildOf(root)));
    let emitter = Emitter::new(
        fire::preset(0.8).flame.template,
        0.8,
        vec![crate::particles::Source::at_node()],
    );
    let flame = app
        .world_mut()
        .spawn((
            Flame::new(emitter, 0.0, LARGE_TEXTURE, 5),
            Transform::IDENTITY,
            ChildOf(part),
        ))
        .id();
    for _ in 0..30 {
        app.update();
    }
    let world = app.world();
    let pose = world.get::<Ragdoll>(root).unwrap().parts[0].transform();
    let node = world.get::<Flame>(flame).unwrap().emitter().node.position / 100.0;
    let fell = start.y - pose.translation.y;
    assert!(fell > 0.1, "fell {fell} m");
    // The node is the part's pose at the object stage, before the level and
    // the friction passes moved it on (no level here: the same pose).
    assert!(
        (node - Vec3::from(pose.translation)).length() < 0.01,
        "{node} against {}",
        pose.translation
    );
    assert!(world.resource::<Taken>().0.is_empty());
}

/// What the frame saw of a flame, frame after frame.
#[derive(Default)]
struct Seen {
    ticks: Vec<i32>,
    live: Vec<usize>,
    viewed: usize,
}

fn watch(app: &App, flame: Entity, seen: &mut Seen) {
    let world = app.world();
    let f = world.get::<Flame>(flame).unwrap();
    seen.ticks.push(f.emitter().ticks);
    seen.live.push(f.emitter().live());
    seen.viewed += usize::from(world.get::<FlameView>(flame).is_some());
}

/// Behind the bridge the frame draws the step's flame as a snapshot: in
/// lockstep and free of the frame alike the frame's copy is a whole emitter
/// every frame (its ticks never go back, its live particles never drop out
/// once it burns: no flicker), and the frame keeps its own view.
#[test]
fn the_frame_draws_the_steps_flame_in_both_modes() {
    for mode in [SimMode::Inline, SimMode::Free] {
        let mut app = plugins(Some(mode));
        let node = fire_at(&mut app, Vec3::ZERO, 0.2);
        if let Some(mut pause) = app.world_mut().get_resource_mut::<SimPause>() {
            pause.hold(false);
        }
        let flame = flames_of(&mut app, node)[0];
        let mut seen = Seen::default();
        let frames = if mode == SimMode::Free { 90 } else { 240 };
        for _ in 0..frames {
            app.update();
            if mode == SimMode::Free {
                std::thread::sleep(Duration::from_millis(16));
            }
            watch(&app, flame, &mut seen);
        }
        if mode == SimMode::Free {
            exa_physics::sim_app::hold_and_drain(app.world_mut());
        }
        let last = *seen.ticks.last().unwrap();
        assert!(last > 100, "{mode:?}: {last} ticks");
        assert!(
            seen.ticks.windows(2).all(|w| w[1] >= w[0]),
            "{mode:?}: the ticks went back: {:?}",
            seen.ticks
        );
        // From a second in, the torch keeps 20-40 particles every frame.
        let burning: Vec<usize> = seen
            .ticks
            .iter()
            .zip(&seen.live)
            .filter(|(t, _)| **t > 83)
            .map(|(_, l)| *l)
            .collect();
        assert!(!burning.is_empty(), "{mode:?}");
        assert!(
            burning.iter().all(|&l| l > 10),
            "{mode:?}: the flame dropped out: {burning:?}"
        );
        assert!(seen.viewed >= seen.ticks.len() - 1, "{mode:?}");
        let fire = app.world().get::<Fire>(flame).unwrap();
        assert!(fire.intensity > 0.0, "{mode:?}");
    }
}

/// The GI wait's pre-roll with the simulation held free of the frame: the
/// frame's copy settles at once, and the step's copy takes the same ticks
/// (the value the frame inserted carries them), so once the hold ends the
/// step goes on from the settled flame and the frame never sees it go back.
#[test]
fn a_held_free_load_settles_the_fires_on_both_sides() {
    let mut app = plugins(Some(SimMode::Free));
    app.world_mut().resource_mut::<SimPause>().hold(true);
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    app.insert_resource(crate::gi_volume::PendingGi {
        centre: Vec3::ZERO,
        top: 1.0,
        bottom: 0.0,
        base_ambient: [0.0; 3],
        grid: crate::gi_volume::GiGrid::synthetic(),
        top_slice: false,
    });
    let node = fire_at(&mut app, Vec3::ZERO, 0.2);
    for _ in 0..3 {
        app.update();
    }
    let flame = flames_of(&mut app, node)[0];
    let settled = fire::SETTLE_TICKS as i32;
    assert_eq!(
        app.world().get::<Flame>(flame).unwrap().emitter().ticks,
        settled
    );
    assert!(app.world().get::<Fire>(flame).unwrap().settled());
    app.world_mut().remove_resource::<crate::gi_volume::PendingGi>();
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    app.world_mut().resource_mut::<SimPause>().hold(false);
    let mut seen = Seen::default();
    for _ in 0..30 {
        app.update();
        std::thread::sleep(Duration::from_millis(16));
        watch(&app, flame, &mut seen);
    }
    exa_physics::sim_app::hold_and_drain(app.world_mut());
    assert!(
        seen.ticks.iter().all(|&t| t >= settled),
        "the step's copy started over: {:?}",
        seen.ticks
    );
    assert!(*seen.ticks.last().unwrap() > settled + 10, "{:?}", seen.ticks);
}
