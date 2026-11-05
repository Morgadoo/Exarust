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
    // Steady state: only what changed crosses: in, the lent `Heavy`,
    // `Tally` and command queue, Bevy's four clocks and its frame count; out, the two moving
    // bodies' `Pos`, `Heavy`, `Tally`, the queue and the fixed step's log.
    // The three still bodies stay home.
    let before = world.resource::<FrameBridge>().cost;
    app.update();
    let after = app.world().resource::<FrameBridge>().cost;
    assert_eq!(after.steps, before.steps + 1);
    let values_in = after.values_in - before.values_in;
    let values_out = after.values_out - before.values_out;
    assert_eq!(values_in, 8, "{before:?} -> {after:?}");
    assert_eq!(values_out, 6, "{before:?} -> {after:?}");
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
