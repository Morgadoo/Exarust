//! The bridge keeps what each side's systems see: a scripted run in one
//! world, and the same run with the fixed step moved to the simulation's
//! world (inline and on its thread), log the same changes, additions,
//! removals, spawns, despawns and messages on both sides.
//! (Included from sim_bridge.rs.)
use super::*;
use crate::sim_app::{LockstepCheck, SimMode, SimShellPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

/// A test object's name, stable across runs (entity ids are not).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Tag(u32);
#[derive(Component, Clone, Debug, Default, PartialEq)]
struct Pos(f32);
#[derive(Component, Clone, Debug, Default, PartialEq)]
struct Vel(f32);
/// Lent: moved to the simulation for the step.
#[derive(Component, Debug, Default, PartialEq)]
struct Heavy(Vec<u32>);
/// Spawned by the simulation for each ping; despawned there at 99.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct Spark(u32);
/// An entity the fixed step points at by id.
#[derive(Component, Clone, Copy, Debug)]
struct Follows(Entity);

#[derive(Message, Clone, Copy, Debug, PartialEq)]
struct Ping(u32);
#[derive(Message, Clone, Copy, Debug, PartialEq)]
struct Pong(u32);

/// What the fixed step saw each step.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
struct SimSeen(Vec<String>);
/// What the frame saw each frame.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
struct FrameSeen(Vec<String>);
/// Lent resource: the fixed step pushes to it.
#[derive(Resource, Debug, Default, PartialEq)]
struct Tally(Vec<u32>);

fn tags_of(entities: impl Iterator<Item = Entity>, tags: &Query<&Tag>) -> Vec<String> {
    let mut v: Vec<String> = entities
        .map(|e| tags.get(e).map_or("gone".into(), |t| t.0.to_string()))
        .collect();
    v.sort();
    v
}

#[allow(clippy::too_many_arguments)]
fn sim_watch(
    changed: Query<Entity, Changed<Pos>>,
    added: Query<Entity, Added<Vel>>,
    mut removed: RemovedComponents<Vel>,
    heavy: Query<(&Tag, Ref<Heavy>)>,
    follows: Query<(&Tag, &Follows)>,
    tags: Query<&Tag>,
    kids: Query<&Children>,
    mut seen: ResMut<SimSeen>,
) {
    let kids: usize = kids.iter().map(|c| c.len()).sum();
    let mut heavy_changed: Vec<u32> = heavy
        .iter()
        .filter(|(_, h)| h.is_changed())
        .map(|(t, _)| t.0)
        .collect();
    heavy_changed.sort();
    let mut followed: Vec<(u32, String)> = follows
        .iter()
        .map(|(t, f)| {
            (
                t.0,
                tags.get(f.0).map_or("gone".into(), |t| t.0.to_string()),
            )
        })
        .collect();
    followed.sort();
    seen.0.push(format!(
        "changed {:?} added {:?} removed {:?} heavy {heavy_changed:?} follows {followed:?} kids {kids}",
        tags_of(changed.iter(), &tags),
        tags_of(added.iter(), &tags),
        tags_of(removed.read(), &tags),
    ));
}

fn integrate(mut bodies: Query<(&mut Pos, &Vel)>, mut heavy: Query<&mut Heavy>) {
    for (mut p, v) in &mut bodies {
        if v.0 != 0.0 {
            p.0 += v.0;
        }
    }
    for mut h in &mut heavy {
        if h.0.len() < 3 {
            h.0.push(7);
        }
    }
}

fn sparks(
    mut commands: Commands,
    mut pings: MessageReader<Ping>,
    mut pongs: MessageWriter<Pong>,
    sparks: Query<(Entity, &Spark)>,
    mut tally: ResMut<Tally>,
) {
    for (e, s) in &sparks {
        if s.0 == 99 {
            commands.entity(e).despawn();
        }
    }
    for p in pings.read() {
        commands.spawn((Spark(p.0), Tag(1000 + p.0)));
        pongs.write(Pong(p.0));
        tally.0.push(p.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn frame_watch(
    changed: Query<Entity, Changed<Pos>>,
    added: Query<&Spark, Added<Spark>>,
    mut removed: RemovedComponents<Spark>,
    mut pongs: MessageReader<Pong>,
    tags: Query<&Tag>,
    mut seen: ResMut<FrameSeen>,
) {
    let mut sparks: Vec<u32> = added.iter().map(|s| s.0).collect();
    sparks.sort();
    let pongs: Vec<u32> = pongs.read().map(|p| p.0).collect();
    seen.0.push(format!(
        "changed {:?} sparks {sparks:?} unsparked {} pongs {pongs:?}",
        tags_of(changed.iter(), &tags),
        removed.read().count(),
    ));
}

fn app(shell: Option<SimMode>) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(crate::STEP_HZ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / crate::STEP_HZ,
        )))
        .add_message::<Ping>()
        .add_message::<Pong>()
        .init_resource::<SimSeen>()
        .init_resource::<FrameSeen>()
        .init_resource::<Tally>()
        .add_systems(FixedUpdate, (sim_watch, integrate, sparks).chain())
        .add_systems(Update, frame_watch);
    crate::sim_app::add_fixed_commands(&mut app);
    if let Some(mode) = shell {
        app.add_plugins((SimShellPlugin { mode }, SimBridgePlugin { extra: vec![] }))
            .bridge_copy::<Tag>()
            .bridge_copy::<Pos>()
            .bridge_copy::<Vel>()
            .bridge_lend::<Heavy>()
            .bridge_copy::<Spark>()
            .bridge_copy::<Follows>()
            .bridge_copy_resource::<SimSeen>()
            .bridge_lend_resource::<Tally>()
            .bridge_message::<Ping>()
            .bridge_message::<Pong>();
    }
    app
}

/// The script: what the frame does before each of its updates.
fn script(app: &mut App, frame: u32, named: &mut Vec<Entity>) {
    let world = app.world_mut();
    match frame {
        0 => {
            for t in 0..5 {
                let e = world.spawn((Tag(t), Pos(t as f32), Vel(0.0))).id();
                named.push(e);
            }
            world.entity_mut(named[4]).insert(Heavy::default());
            let target = named[2];
            world.entity_mut(named[0]).insert(Follows(target));
        }
        1 => {
            // A parent the bridge carries nothing of: it follows its child.
            let parent = world.spawn_empty().id();
            world.entity_mut(named[1]).insert(ChildOf(parent));
        }
        2 => {
            world.get_mut::<Pos>(named[1]).unwrap().0 = 10.0;
            world.get_mut::<Vel>(named[3]).unwrap().0 = 1.0;
        }
        3 => {
            world.entity_mut(named[2]).remove::<Vel>();
            world.write_message(Ping(5));
        }
        4 => {
            world.entity_mut(named[2]).insert(Vel(0.5));
            world.write_message(Ping(99));
            world.write_message(Ping(6));
        }
        5 => {
            // A frame decision posted as a write: it lands at the start of
            // the next step, in either world.
            let target = named[1];
            world
                .resource_mut::<crate::sim_app::FixedCommands>()
                .post(move |w: &mut World| {
                    if let Some(mut p) = w.get_mut::<Pos>(target) {
                        p.0 = 42.0;
                    }
                });
        }
        6 => {
            world.entity_mut(named[3]).despawn();
            world.get_mut::<Heavy>(named[4]).unwrap().0.clear();
        }
        7 => {
            // A new entity takes the follower's target.
            let e = world.spawn((Tag(9), Pos(0.0))).id();
            world.entity_mut(named[0]).insert(Follows(e));
            world.entity_mut(named[2]).despawn();
        }
        _ => {}
    }
}

struct Run {
    sim: Vec<String>,
    frame: Vec<String>,
    end: Vec<(u32, String)>,
    tally: Vec<u32>,
    steps: u64,
}

fn run(shell: Option<SimMode>) -> Run {
    let mut app = app(shell);
    let mut named = Vec::new();
    for frame in 0..12 {
        script(&mut app, frame, &mut named);
        app.update();
    }
    let world = app.world_mut();
    let mut end: Vec<(u32, String)> = world
        .query::<(&Tag, Option<&Pos>, Option<&Vel>, Option<&Heavy>)>()
        .iter(world)
        .map(|(t, p, v, h)| (t.0, format!("{p:?} {v:?} {h:?}")))
        .collect();
    end.sort_by_key(|a| a.0);
    let steps = world.get_resource::<LockstepCheck>().map_or(0, |c| c.steps);
    Run {
        sim: world.resource::<SimSeen>().0.clone(),
        frame: world.resource::<FrameSeen>().0.clone(),
        end,
        tally: world.resource::<Tally>().0.clone(),
        steps,
    }
}

#[test]
fn the_bridge_shows_each_side_what_one_world_would() {
    let alone = run(None);
    assert!(alone.sim.len() >= 10, "{:?}", alone.sim);
    // The script reached the fixed step: frame 2's move, frame 3's removal.
    assert!(alone.sim.iter().any(|s| s.contains("removed [\"2\"]")));
    assert!(alone.sim.last().unwrap().ends_with("kids 1"));
    // The posted write reached the fixed step.
    assert!(
        alone
            .end
            .iter()
            .any(|(t, v)| *t == 1 && v.contains("Pos(42.0)"))
    );
    for mode in [SimMode::Inline, SimMode::Thread] {
        let bridged = run(Some(mode));
        assert_eq!(bridged.steps, alone.sim.len() as u64);
        assert_eq!(alone.sim, bridged.sim, "{mode:?}: the fixed step's view");
        assert_eq!(alone.frame, bridged.frame, "{mode:?}: the frame's view");
        assert_eq!(alone.end, bridged.end, "{mode:?}");
        assert_eq!(alone.tally, bridged.tally, "{mode:?}");
    }
}

#[test]
fn ids_are_shared_and_unchanged_values_stay_home() {
    let mut app = app(Some(SimMode::Inline));
    let mut named = Vec::new();
    for frame in 0..6 {
        script(&mut app, frame, &mut named);
        app.update();
    }
    let world = app.world();
    let bridge = world.resource::<FrameBridge>();
    let coverage = bridge.coverage.clone().unwrap();
    assert!(coverage.missing.is_empty(), "{:?}", coverage.missing);
    assert!(coverage.schedules.iter().any(|s| s == "FixedUpdate"));
    assert!(world.resource::<Schedules>().get(FixedUpdate).is_none());
    // The sparks the simulation spawned have the same ids on both sides.
    let shell = world.resource::<crate::sim_app::SimShell>();
    let sim_sparks = shell
        .with_world(|sim| {
            let mut v: Vec<(Entity, u32)> = sim
                .try_query::<(Entity, &Spark)>()
                .unwrap()
                .iter(sim)
                .map(|(e, s)| (e, s.0))
                .collect();
            v.sort();
            v
        })
        .unwrap();
    let world = app.world_mut();
    let mut frame_sparks: Vec<(Entity, u32)> = world
        .query::<(Entity, &Spark)>()
        .iter(world)
        .map(|(e, s)| (e, s.0))
        .collect();
    frame_sparks.sort();
    assert_eq!(sim_sparks, frame_sparks);
    assert_eq!(frame_sparks.iter().map(|s| s.1).collect::<Vec<_>>(), [5, 6]);
    // Steady state: only what changed crosses: in, the lent `Heavy` and
    // `Tally`, Bevy's four clocks and its frame count; out, the two moving
    // bodies' `Pos`, `Heavy`, `Tally` and the fixed step's log (each world
    // keeps its own command queue, the frame's posts going in its packet).
    // The three still bodies stay home.
    let before = world.resource::<FrameBridge>().cost;
    app.update();
    let after = app.world().resource::<FrameBridge>().cost;
    assert_eq!(after.steps, before.steps + 1);
    let values_in = after.values_in - before.values_in;
    let values_out = after.values_out - before.values_out;
    assert_eq!(values_in, 7, "{before:?} -> {after:?}");
    assert_eq!(values_out, 5, "{before:?} -> {after:?}");
}

/// A type a moved system touches that no plugin registered stops a debug
/// or test build at the first step.
#[test]
#[cfg_attr(
    debug_assertions,
    should_panic(expected = "touch types no plugin registered")
)]
fn an_unregistered_type_stops_a_debug_build() {
    #[derive(Component)]
    struct Unregistered;
    fn touch(_q: Query<&Tag, With<Unregistered>>) {}
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins((
            SimShellPlugin {
                mode: SimMode::Inline,
            },
            SimBridgePlugin { extra: vec![] },
        ))
        .bridge_copy::<Tag>()
        .add_systems(FixedUpdate, touch);
    app.world_mut().run_schedule(bevy::app::FixedMain);
}

/// A snapshot: the simulation owns it, the frame reads it.
#[derive(Component, Clone, Debug, Default, PartialEq)]
struct Snap(f32);

/// What the fixed step and the frame saw of the snapshot, per step and
/// per frame.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
struct SnapSeen(Vec<String>);

fn snap_step(mut snaps: Query<&mut Snap>, mut seen: ResMut<SnapSeen>) {
    for mut s in &mut snaps {
        seen.0.push(format!("sim {}", s.0));
        s.0 += 1.0;
    }
}

fn snap_app(shell: Option<SimMode>) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(crate::STEP_HZ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / crate::STEP_HZ,
        )))
        .init_resource::<SnapSeen>()
        .add_systems(FixedUpdate, snap_step);
    crate::sim_app::add_fixed_commands(&mut app);
    if let Some(mode) = shell {
        app.add_plugins((SimShellPlugin { mode }, SimBridgePlugin { extra: vec![] }))
            .bridge_snapshot::<Snap>()
            .bridge_copy_resource::<SnapSeen>();
    }
    app
}

/// The frame's view of the snapshot after each update, with the frame's
/// posted write at frame 3 (it reads its own write at once) and, at frame
/// 6, a write the frame did not post.
fn snap_run(shell: Option<SimMode>, unposted: bool) -> (Vec<String>, Vec<f32>) {
    let mut app = snap_app(shell);
    let e = app.world_mut().spawn(Snap(0.0)).id();
    let mut frame = Vec::new();
    for f in 0..10 {
        let world = app.world_mut();
        if f == 3 {
            world.resource_scope(|world, mut fixed: Mut<crate::sim_app::FixedCommands>| {
                let mut snap = world.get_mut::<Snap>(e).unwrap();
                let read = fixed.write(e, &mut snap, |s| {
                    s.0 = 100.0;
                    s.0
                });
                assert_eq!(read, 100.0);
                assert_eq!(snap.0, 100.0);
            });
        }
        if f == 6 && unposted {
            world.get_mut::<Snap>(e).unwrap().0 = -1.0;
        }
        app.update();
        frame.push(app.world().get::<Snap>(e).unwrap().0);
    }
    (app.world().resource::<SnapSeen>().0.clone(), frame)
}

#[test]
fn a_snapshot_takes_posted_writes_and_keeps_the_frames_own_home() {
    let (alone_sim, alone_frame) = snap_run(None, false);
    assert!(alone_sim.iter().any(|s| s == "sim 100"), "{alone_sim:?}");
    for mode in [SimMode::Inline, SimMode::Thread] {
        // The posted write lands at the next step, before the step runs:
        // both sides see what one world sees.
        let (sim, frame) = snap_run(Some(mode), false);
        assert_eq!(alone_sim, sim, "{mode:?}");
        assert_eq!(alone_frame, frame, "{mode:?}");
        // A write the frame did not post never reaches the simulation, and
        // the next copy out replaces it.
        let (sim, frame) = snap_run(Some(mode), true);
        assert_eq!(alone_sim, sim, "{mode:?}: an unposted write crossed");
        assert_eq!(alone_frame, frame, "{mode:?}");
    }
}

/// The frame's hierarchy and transforms cross only for the entities the
/// simulation holds: one a bridged type crossed for, and its ancestors.
#[test]
fn only_shared_entities_hierarchy_crosses() {
    let mut app = app(Some(SimMode::Inline));
    let world = app.world_mut();
    let parent = world.spawn(Transform::from_xyz(1.0, 0.0, 0.0)).id();
    let body = world
        .spawn((Tag(1), Pos(0.0), Transform::default(), ChildOf(parent)))
        .id();
    let mesh = world.spawn((Transform::default(), ChildOf(body))).id();
    let lone = world.spawn(Transform::default()).id();
    for _ in 0..3 {
        app.update();
    }
    let in_sim = |app: &App, e: Entity| {
        app.world()
            .resource::<crate::sim_app::SimShell>()
            .with_world(|sim| sim.get::<Transform>(e).copied())
            .unwrap()
    };
    assert_eq!(
        in_sim(&app, parent),
        Some(Transform::from_xyz(1.0, 0.0, 0.0))
    );
    assert!(in_sim(&app, body).is_some());
    assert_eq!(in_sim(&app, mesh), None, "a frame-only child crossed");
    assert_eq!(in_sim(&app, lone), None, "a frame-only entity crossed");
    // The body's parent keeps crossing when it moves; the frame's own
    // entities do not, and their order among the body's children is kept.
    app.world_mut()
        .get_mut::<Transform>(parent)
        .unwrap()
        .translation
        .x = 2.0;
    app.world_mut()
        .get_mut::<Transform>(lone)
        .unwrap()
        .translation
        .x = 3.0;
    app.update();
    assert_eq!(in_sim(&app, parent).unwrap().translation.x, 2.0);
    assert_eq!(in_sim(&app, lone), None);
    assert_eq!(
        app.world().get::<Children>(body).map(|c| c.to_vec()),
        Some(vec![mesh])
    );
}

/// A resource the frame owns and the simulation reads.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
struct Input(u32);
/// A resource the simulation owns and the frame reads.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
struct Output(u32);

/// Each step reads the input and counts the output up.
fn one_way_step(input: Res<Input>, mut output: ResMut<Output>, mut seen: ResMut<SnapSeen>) {
    seen.0.push(format!("in {} out {}", input.0, output.0));
    output.0 += 1;
}

/// A write of the input, which the frame owns, by the simulation.
fn rogue_step(mut input: ResMut<Input>) {
    input.0 = 999;
}

/// The frame writes the input every frame and reads the output; at frame 6
/// it writes the output too (not its own).
fn one_way_run(shell: Option<SimMode>, rogue: bool) -> (Vec<String>, Vec<(u32, u32)>) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(crate::STEP_HZ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / crate::STEP_HZ,
        )))
        .init_resource::<SnapSeen>()
        .init_resource::<Input>()
        .init_resource::<Output>()
        .add_systems(FixedUpdate, one_way_step);
    if rogue {
        app.add_systems(
            FixedUpdate,
            rogue_step
                .after(one_way_step)
                .run_if(|o: Res<Output>| o.0 == 5),
        );
    }
    crate::sim_app::add_fixed_commands(&mut app);
    if let Some(mode) = shell {
        app.add_plugins((SimShellPlugin { mode }, SimBridgePlugin { extra: vec![] }))
            .bridge_copy_in_resource::<Input>()
            .bridge_snapshot_resource::<Output>()
            .bridge_copy_resource::<SnapSeen>();
    }
    let mut frame = Vec::new();
    for f in 0..10 {
        app.world_mut().resource_mut::<Input>().0 = f * 10;
        if f == 6 && rogue {
            app.world_mut().resource_mut::<Output>().0 = 777;
        }
        app.update();
        let w = app.world();
        frame.push((w.resource::<Input>().0, w.resource::<Output>().0));
    }
    (app.world().resource::<SnapSeen>().0.clone(), frame)
}

/// A copied-in resource reaches the simulation each step its frame changed
/// it and never comes back; a snapshot resource reaches the frame each
/// step and the frame's own write to it never reaches the simulation.
/// Without writes on the wrong side, both match one world.
#[test]
fn one_way_resources_cross_one_way() {
    let (alone_sim, alone_frame) = one_way_run(None, false);
    for mode in [SimMode::Inline, SimMode::Thread] {
        assert_eq!(
            one_way_run(Some(mode), false),
            (alone_sim.clone(), alone_frame.clone())
        );
        let (sim, frame) = one_way_run(Some(mode), true);
        // The simulation's write of the input stays there: the frame keeps
        // its own value.
        assert!(frame.iter().all(|(i, _)| *i != 999), "{mode:?}: {frame:?}");
        // The frame's write of the output never reached the simulation,
        // and the next copy out replaced it.
        assert!(
            sim.iter().all(|s| !s.contains("out 777")),
            "{mode:?}: {sim:?}"
        );
        assert_ne!(frame[6].1, 777, "{mode:?}");
    }
}

/// A resource the frame owns that a fixed step also writes.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
struct Owned(Vec<u32>);

/// Each step appends its count through the frame's queue, and reads what
/// it has seen so far.
fn owned_step(
    mut owned: ResMut<Owned>,
    mut frame: ResMut<crate::sim_app::FrameCommands>,
    mut seen: ResMut<SnapSeen>,
    mut n: Local<u32>,
) {
    *n += 1;
    let k = *n;
    seen.0.push(format!("{:?}", owned.0));
    frame.write_resource(&mut owned, move |o| o.0.push(k));
}

/// The frame appends 100 + its frame number each frame too.
fn owned_run(shell: Option<SimMode>) -> (Vec<String>, Vec<Vec<u32>>) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(crate::STEP_HZ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / crate::STEP_HZ,
        )))
        .init_resource::<SnapSeen>()
        .init_resource::<Owned>()
        .add_systems(FixedUpdate, owned_step);
    crate::sim_app::add_fixed_commands(&mut app);
    crate::sim_app::add_frame_commands(&mut app);
    if let Some(mode) = shell {
        app.add_plugins((SimShellPlugin { mode }, SimBridgePlugin { extra: vec![] }))
            .bridge_copy_in_resource::<Owned>()
            .bridge_copy_resource::<SnapSeen>();
    }
    let mut frame = Vec::new();
    for f in 0..8 {
        app.world_mut().resource_mut::<Owned>().0.push(100 + f);
        app.update();
        frame.push(app.world().resource::<Owned>().0.clone());
    }
    (app.world().resource::<SnapSeen>().0.clone(), frame)
}

/// A step's write to data the frame owns reaches the frame after the step
/// and the step sees it at once: both sides see what one world sees.
#[test]
fn a_steps_write_to_frame_data_lands_after_the_step() {
    let alone = owned_run(None);
    assert!(alone.1.last().unwrap().contains(&3), "{alone:?}");
    for mode in [SimMode::Inline, SimMode::Thread] {
        assert_eq!(owned_run(Some(mode)), alone, "{mode:?}");
    }
}

/// Moves every `Pos` on each fixed step (each step's packet names them).
fn drift(mut all: Query<&mut Pos>) {
    for mut p in &mut all {
        p.0 += 1.0;
    }
}

/// A frame resource spawned after a teardown (it may take a despawned
/// entity's index).
#[derive(Resource, Default)]
struct AfterTeardown;

/// A free-running simulation's step names entities the frame then tears
/// down: the teardown holds the simulation and applies its queued steps
/// first, so no step arrives later to name a despawned entity, and nothing
/// of the frame's (a newer entity at the same index, a resource) is
/// despawned for one.
#[test]
fn a_teardown_holds_the_free_simulation_first() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(SimShellPlugin {
            mode: SimMode::Free,
        })
        .add_plugins(SimBridgePlugin { extra: Vec::new() })
        .bridge_copy::<Pos>()
        .bridge_copy::<Tag>()
        .add_systems(FixedUpdate, drift);
    let world = app.world_mut();
    let bodies: Vec<Entity> = (0..64)
        .map(|k| world.spawn((Tag(k), Pos(0.0))).id())
        .collect();
    world.resource_mut::<crate::sim_app::SimPause>().hold(false);
    let run = |app: &mut App, ms: u64| {
        let until = std::time::Instant::now() + Duration::from_millis(ms);
        while std::time::Instant::now() < until {
            app.update();
            std::thread::sleep(Duration::from_millis(3));
        }
    };
    run(&mut app, 150);
    let world = app.world_mut();
    assert!(world.resource::<crate::sim_app::FreeLink>().steps > 0);
    // The teardown: hold and drain, then despawn and spawn anew (reusing
    // the indices).
    crate::sim_app::hold_and_drain(world);
    for &e in &bodies {
        world.despawn(e);
    }
    let fresh: Vec<Entity> = (0..64).map(|k| world.spawn(Tag(100 + k)).id()).collect();
    world.init_resource::<AfterTeardown>();
    run(&mut app, 80);
    app.world_mut()
        .resource_mut::<crate::sim_app::SimPause>()
        .release_teardown();
    run(&mut app, 150);
    let world = app.world();
    assert!(world.contains_resource::<AfterTeardown>());
    for e in fresh {
        assert!(world.get_entity(e).is_ok(), "{e} survives");
    }
    for e in bodies {
        assert!(world.get_entity(e).is_err(), "{e} stays gone");
    }
    assert_eq!(world.resource::<crate::sim_app::FreeLink>().gaps, 0);
}

/// A value the simulation sent for an entity the frame has since replaced
/// at the same index (a newer generation) is dropped on the frame's side.
#[test]
fn a_stale_value_never_despawns_the_frames_newer_entity() {
    let mut world = World::new();
    let old = world.spawn_empty().id();
    world.despawn(old);
    // The index taken again at a later generation.
    let newer = Entity::from_index_and_generation(old.index(), old.generation().after_versions(1));
    assert!(ensure_spawned(&mut world, newer));
    assert!(!ensure_spawned_on_frame(&mut world, old));
    assert!(world.get_entity(newer).is_ok());
}

/// A component the frame removes while the free simulation keeps writing
/// it (its steps in flight still carry it) stays removed on both sides; the
/// same for one the simulation removes while the frame writes it.
#[test]
fn a_removal_is_not_undone_by_a_value_in_flight() {
    #[derive(Component, Clone, Debug, Default, PartialEq)]
    struct Clip(u32);
    /// The simulation drops `Vel` from a body once its `Pos` passes 30.
    fn advance(
        mut clips: Query<&mut Clip>,
        mut commands: Commands,
        vels: Query<(Entity, &Pos), With<Vel>>,
    ) {
        for mut c in &mut clips {
            c.0 += 1;
        }
        for (e, p) in &vels {
            if p.0 > 30.0 {
                commands.entity(e).remove::<Vel>();
            }
        }
    }
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(SimShellPlugin {
            mode: SimMode::Free,
        })
        .add_plugins(SimBridgePlugin { extra: Vec::new() })
        .bridge_copy::<Clip>()
        .bridge_copy::<Pos>()
        .bridge_copy::<Vel>()
        .add_systems(FixedUpdate, (drift, advance));
    let world = app.world_mut();
    let clipped: Vec<Entity> = (0..16)
        .map(|k| world.spawn((Tag(k), Clip(0))).id())
        .collect();
    let moving: Vec<Entity> = (0..16)
        .map(|k| world.spawn((Tag(100 + k), Pos(0.0), Vel(1.0))).id())
        .collect();
    world.resource_mut::<crate::sim_app::SimPause>().hold(false);
    let run = |app: &mut App, ms: u64, touch: bool| {
        let until = std::time::Instant::now() + Duration::from_millis(ms);
        while std::time::Instant::now() < until {
            app.update();
            if touch {
                // The frame keeps writing `Vel` (a stale value once the
                // simulation dropped it).
                let world = app.world_mut();
                let mut q = world.query::<&mut Vel>();
                for mut v in q.iter_mut(world) {
                    v.0 += 0.0;
                    v.set_changed();
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    };
    run(&mut app, 120, false);
    for &e in &clipped {
        app.world_mut().entity_mut(e).remove::<Clip>();
    }
    run(&mut app, 700, true);
    let world = app.world();
    for e in clipped {
        assert!(world.get::<Clip>(e).is_none(), "{e} keeps its Clip removed");
    }
    for e in moving {
        assert!(world.get::<Vel>(e).is_none(), "{e} keeps its Vel removed");
    }
}

/// A frame-only component: the frame's loading leaves some of its entities
/// in archetypes the others are not in.
#[derive(Component, Clone, Copy, Debug)]
struct Loaded;

/// The order the fixed step visits the tagged entities with a `Vel` and a
/// `Heavy`.
fn order_step(
    moving: Query<&Tag, With<Vel>>,
    heavy: Query<&Tag, With<Heavy>>,
    mut seen: ResMut<SnapSeen>,
) {
    let moving: Vec<u32> = moving.iter().map(|t| t.0).collect();
    let heavy: Vec<u32> = heavy.iter().map(|t| t.0).collect();
    seen.0.push(format!("vel {moving:?} heavy {heavy:?}"));
}

/// Four entities spawned in tag order; `loaded` marks some of them
/// [`Loaded`] first (the frame's own data, as a mesh that finished loading),
/// which puts them after the others in the frame's queries. Then each gets
/// a copied `Vel` and a lent `Heavy` in the same frame.
fn order_run(mode: SimMode, loaded: &[usize]) -> (Vec<String>, Vec<u32>) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(crate::STEP_HZ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / crate::STEP_HZ,
        )))
        .init_resource::<SnapSeen>()
        .add_systems(FixedUpdate, order_step);
    crate::sim_app::add_fixed_commands(&mut app);
    app.add_plugins((SimShellPlugin { mode }, SimBridgePlugin { extra: vec![] }))
        .bridge_copy::<Tag>()
        .bridge_copy::<Vel>()
        .bridge_lend::<Heavy>()
        .bridge_copy_resource::<SnapSeen>();
    let named: Vec<Entity> = (0..4).map(|t| app.world_mut().spawn(Tag(t)).id()).collect();
    app.update();
    for &k in loaded {
        app.world_mut().entity_mut(named[k]).insert(Loaded);
    }
    app.update();
    let world = app.world_mut();
    let frame_order: Vec<Entity> = world
        .query_filtered::<Entity, With<Tag>>()
        .iter(world)
        .collect();
    for e in frame_order {
        world.entity_mut(e).insert((Vel(1.0), Heavy::default()));
    }
    app.update();
    app.update();
    // The tags in entity order (by index).
    let mut by_id: Vec<(u32, u32)> = (0..4).map(|t| (named[t as usize].index_u32(), t)).collect();
    by_id.sort();
    (
        app.world().resource::<SnapSeen>().0.clone(),
        by_id.into_iter().map(|(_, t)| t).collect(),
    )
}

/// The simulation's queries visit entities in an order that does not hang
/// on the frame's own archetypes: values cross in entity order.
#[test]
fn the_frames_archetypes_do_not_order_the_simulations_queries() {
    for mode in [SimMode::Inline, SimMode::Thread] {
        let (plain, by_id) = order_run(mode, &[]);
        let sorted = format!("vel {by_id:?} heavy {by_id:?}");
        assert!(plain.contains(&sorted), "{mode:?}: {plain:?}");
        for loaded in [&[0][..], &[0, 2], &[1, 3], &[3]] {
            let (seen, _) = order_run(mode, loaded);
            assert_eq!(plain, seen, "{mode:?}: loaded {loaded:?}");
        }
    }
}

/// A moved resource only the fixed step names.
#[derive(Resource, Debug, Default)]
struct MovedOnly;

/// Bridging a resource registers its type in the frame's world, so the
/// frame's resource hold at the second update holds its entity: its first
/// insert, whenever it comes, then takes no fresh id.
#[test]
fn a_bridged_resource_is_held_by_the_frames_hold() {
    let mut app = App::new();
    app.bridge_move_resource::<MovedOnly>();
    let world = app.world_mut();
    assert!(world.component_id::<MovedOnly>().is_some());
    crate::resource_hold::hold_resource_entities(world);
    let before = world.entities().len();
    world.insert_resource(MovedOnly);
    assert_eq!(world.entities().len(), before);
}
