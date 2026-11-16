//! A debug timing of a shipped level's fires (ignored by default; skipped
//! without an installation): `exanima01` spawned headless with the physics,
//! the fires and the flames' plugins, the fires' particle texture loaded so
//! every flame builds its mesh, and a camera at the level's first fire.
//! The headless load lights no fire of its own (the level's fires come
//! from the game's start: the story's torch), so each run adds fires at
//! the level's first lights: `FLAME_RUNS` lists `level:count:size` runs
//! (default none, the level alone, then one torch, the story's, then
//! sixteen 0.8 fires): the difference to the run without fires is the
//! fires' cost. Each run prints the mean and the median frame.
//! After 300 frames (the fires settle), 600 frames are timed: the mean wall
//! time of a frame's update and the process CPU time a frame (both threads
//! in the free mode), in one world, behind the lockstep bridge on the
//! frame's thread, and with the simulation running free. Run with
//! `cargo test --release -p exa_world --test flame_frame_timing -- --ignored --nocapture`.
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use exa_assets::{Game, GameData, ModelCache, ModelLoader};
use exa_physics::sim_app::{SimMode, SimPause, SimShellPlugin};
use exa_physics::sim_bridge::SimBridgePlugin;

/// The process's user and system CPU time so far (Linux `/proc`).
fn cpu_time() -> Duration {
    let Ok(stat) = std::fs::read_to_string("/proc/self/stat") else {
        return Duration::ZERO;
    };
    // The fields after the command's closing parenthesis; utime and stime
    // are the 14th and 15th of the line, in clock ticks (100 a second).
    let rest = stat.rsplit_once(')').map_or("", |(_, r)| r);
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let ticks: u64 = [11, 12]
        .iter()
        .filter_map(|&i| fields.get(i).and_then(|f| f.parse::<u64>().ok()))
        .sum();
    Duration::from_millis(ticks * 10)
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    OneWorld,
    Lockstep,
    Free,
}

fn level_app(game: &std::sync::Arc<GameData>, level: &str, mode: Mode) -> App {
    bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin))
        .add_plugins(bevy::log::LogPlugin::default())
        .add_plugins(exa_physics::PhysicsPlugin)
        .add_plugins((exa_world::fire::FirePlugin, exa_world::flame::FlamePlugin))
        .insert_resource(Game(game.clone()))
        .init_resource::<ModelCache>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<exa_render::material::ExaMaterial>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<exa_render::flame::FlameMaterial>>();
    match mode {
        Mode::OneWorld => {}
        Mode::Lockstep | Mode::Free => {
            app.add_plugins((
                SimShellPlugin {
                    mode: if matches!(mode, Mode::Free) {
                        SimMode::Free
                    } else {
                        SimMode::Inline
                    },
                },
                SimBridgePlugin { extra: Vec::new() },
            ));
        }
    }
    if !matches!(mode, Mode::Free) {
        // Sixty frames a second of game time, whatever the frames take.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
    }
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let world = app.world_mut();
        world.resource_scope(|world, mut cache: Mut<ModelCache>| {
            world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                world.resource_scope(
                    |world, mut materials: Mut<Assets<exa_render::material::ExaMaterial>>| {
                        world.resource_scope(|world, mut images: Mut<Assets<Image>>| {
                            let mut loader = ModelLoader {
                                game,
                                cache: &mut cache,
                                meshes: &mut meshes,
                                materials: &mut materials,
                                images: &mut images,
                                bc_supported: true,
                            };
                            let mut commands = Commands::new(&mut queue, world);
                            exa_world::spawn_level(&mut commands, &mut loader, level)
                                .expect("the level is in Resource.rpk");
                        })
                    },
                )
            })
        });
    }
    queue.apply(app.world_mut());
    if let Some(mut pause) = app.world_mut().get_resource_mut::<SimPause>() {
        pause.hold(false);
    }
    app
}

fn run(game: &std::sync::Arc<GameData>, level: &str, count: usize, size: f32, mode: Mode) {
    let mut app = level_app(game, level, mode);
    // A few frames for the hierarchy, then the camera at the first fire.
    for _ in 0..3 {
        app.update();
    }
    // The fires, one at each of the level's first lights.
    let lights: Vec<Vec3> = app
        .world_mut()
        .query_filtered::<&GlobalTransform, With<PointLight>>()
        .iter(app.world())
        .take(count)
        .map(GlobalTransform::translation)
        .collect();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    for at in lights {
        let node = app
            .world_mut()
            .spawn((
                Transform::from_translation(at),
                GlobalTransform::from_translation(at),
            ))
            .id();
        let mut commands = Commands::new(&mut queue, app.world());
        exa_world::fire::spawn_fire(&mut commands, node, size);
    }
    queue.apply(app.world_mut());
    for _ in 0..2 {
        app.update();
    }
    let fires: Vec<Vec3> = app
        .world_mut()
        .query_filtered::<&GlobalTransform, With<exa_world::flame::Flame>>()
        .iter(app.world())
        .map(GlobalTransform::translation)
        .collect();
    let at = fires.first().copied().unwrap_or(Vec3::ZERO);
    app.world_mut().spawn((
        Camera3d::default(),
        Transform::from_translation(at + Vec3::new(2.0, 1.5, 2.0)).looking_at(at, Vec3::Y),
    ));
    let frame = |app: &mut App| {
        app.update();
        if matches!(mode, Mode::Free) {
            std::thread::sleep(Duration::from_millis(16));
        }
    };
    for _ in 0..300 {
        frame(&mut app);
    }
    let frames = 900u32;
    let cpu = cpu_time();
    let mut walls = Vec::with_capacity(frames as usize);
    for _ in 0..frames {
        let started = Instant::now();
        app.update();
        walls.push(started.elapsed());
        if matches!(mode, Mode::Free) {
            std::thread::sleep(Duration::from_millis(16));
        }
    }
    let cpu = cpu_time() - cpu;
    let wall: Duration = walls.iter().sum();
    walls.sort_unstable();
    let median = walls[walls.len() / 2];
    let world = app.world_mut();
    let live: usize = world
        .query::<&exa_world::flame::Flame>()
        .iter(world)
        .map(|f| f.emitter().live())
        .sum();
    let meshes = world
        .query_filtered::<(), (With<exa_world::flame::Flame>, With<Mesh3d>)>()
        .iter(world)
        .count();
    let (bodies, parts, awake) = world
        .query::<&exa_physics::body::Ragdoll>()
        .iter(world)
        .fold((0, 0, 0), |(b, p, a), r| {
            let up = r.sleep == exa_physics::body::Sleep::Awake;
            (b + 1, p + r.parts.len(), a + usize::from(up))
        });
    println!(
        "{level} ({count} x {size}) {mode:?}: {} flames ({meshes} with meshes), {live} live particles, {bodies} bodies of {parts} parts in the frame ({awake} awake); update {:?} a frame (median {:?}), process CPU {:?} a frame",
        fires.len(),
        wall / frames,
        median,
        cpu / frames,
    );
    if matches!(mode, Mode::Free) {
        exa_physics::sim_app::hold_and_drain(app.world_mut());
    }
}

#[test]
#[ignore]
fn shipped_level_fire_frame_time() {
    let Some(dir) = exa_assets::locate_install(None) else {
        println!("no installation: skipped");
        return;
    };
    // The particle stage's own counts, logged every thousand ticks (the
    // debug aid of `exa_world::particle_stage`).
    // SAFETY: set before any thread of this test reads the environment.
    unsafe { std::env::set_var("EXA_PARTICLE_TIMING", "1") };
    let game = std::sync::Arc::new(GameData::open(&dir).unwrap());
    let runs = std::env::var("FLAME_RUNS")
        .unwrap_or_else(|_| "exanima01:0:0,exanima01:1:0.2,exanima01:16:0.8".into());
    for one in runs.split(',') {
        let mut parts = one.split(':');
        let level = parts.next().unwrap_or("exanima01");
        let count = parts.next().and_then(|c| c.parse().ok()).unwrap_or(1);
        let size = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.2);
        for mode in [Mode::OneWorld, Mode::Lockstep, Mode::Free] {
            run(&game, level, count, size, mode);
        }
    }
}
