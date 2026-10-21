use super::*;

fn clock(step: u32, last: u32) -> Clock {
    Clock {
        step,
        last,
        ..Clock::default()
    }
}

/// A simulation is due when the time since it last ran reaches its step's
/// end; more than four steps behind, its last run time jumps forward to now
/// less the step (the backlog is dropped); a paused one never drops.
#[test]
fn a_late_simulation_drops_its_backlog() {
    let c = clock(12, 0);
    assert!(!c.due(11));
    assert!(c.due(12));
    assert_eq!(c.behind(48), 36);
    let mut c = clock(12, 0);
    c.drop_backlog(48);
    assert_eq!(c.last, 0, "three steps late is kept");
    c.drop_backlog(72);
    assert_eq!(c.last, 60, "five steps late drops to now less the step");
    let mut paused = Clock {
        paused: true,
        ..clock(12, 0)
    };
    paused.drop_backlog(72);
    assert_eq!(paused.last, 0);
    let mut c = clock(12, 0);
    c.advance();
    assert_eq!((c.tick, c.time), (1, 12));
}

/// The driver takes the simulation whose step is due and otherwise waits
/// for the soonest end.
#[test]
fn the_driver_takes_the_due_simulation() {
    let mut sims = [clock(12, 0), clock(12, 100)];
    // The first is due; the second's next step is at 112, not yet.
    assert_eq!(pick_due(&mut sims, 20), Due::Now);
    assert_eq!(sims[0].last, 0);
    let mut quiet = [clock(12, 100)];
    assert_eq!(pick_due(&mut quiet, 104), Due::Wait(8));
    // Overdue by more than four steps: the last run time moves forward.
    let mut late = [clock(12, 0)];
    assert_eq!(pick_due(&mut late, 1000), Due::Now);
    assert_eq!(late[0].last, 988);
}

/// The time-scale helper takes the unsigned game time less the mark as a
/// signed difference, scales it and rounds to even (NaN and out-of-range
/// answers are zero).
#[test]
fn the_clock_scales_a_span() {
    let c = Clock {
        time: 1000,
        ..Clock::default()
    };
    assert_eq!(c.scale_since(940, 0.5), 30);
    assert_eq!(c.scale_since(1000, 0.5), 0);
    let c = Clock {
        time: 10,
        ..Clock::default()
    };
    assert_eq!(c.scale_since(20, 2.0), -20);
    assert_eq!(c.scale_since(0, f32::NAN), 0);
    assert_eq!(c.scale_since(0, 1e30), 0);
}

/// The default environment keeps 0.426, 0.25 and 0.05 a second above 40,
/// above 20 and at rest; the mechanism environment 0.001 everywhere and no
/// gravity; gravity is g·dt² a tick.
#[test]
fn the_environments_band_their_retention() {
    // The thresholds are in cm/s, the motion in cm a tick.
    for (moved, want) in [
        (50.0 / 83.0, 0.989_77),
        (30.0 / 83.0, 0.983_44),
        (10.0 / 83.0, 0.964_55),
    ] {
        let got = Environment::DEFAULT.retention(moved);
        assert!((got - want).abs() < 1e-5, "{moved} cm: {got}");
    }
    assert!((Environment::MECHANISM.retention(100.0) - 0.9201).abs() < 1e-3);
    let g = Environment::DEFAULT.gravity_per_tick();
    assert!((g.y + 0.001_423_5).abs() < 1e-7, "{g}");
    assert_eq!(Environment::ZERO_GRAVITY.gravity_per_tick(), Vec3::ZERO);
    assert_eq!(Environment::MECHANISM.gravity_per_tick(), Vec3::ZERO);
}

/// The grid places a node's translation in a 600 cm cell (rounded to even)
/// moved by the half, and shifts by flat rows with the strip clear.
#[test]
fn the_grid_places_and_shifts_bodies() {
    let grid = Grid::new(GridSize {
        width: 5,
        height: 5,
    });
    assert_eq!(grid.bias, [2, 2]);
    assert_eq!(grid.locate(Vec3::ZERO), Some((2, 2)));
    assert_eq!(grid.locate(Vec3::X * 600.0), Some((3, 2)));
    // −0.5 rounds to even, which is −0.
    assert_eq!(grid.locate(Vec3::Z * -300.0), Some((2, 2)));
    assert_eq!(grid.locate(Vec3::Z * -400.0), Some((2, 1)));
    assert_eq!(grid.locate(Vec3::X * 5000.0), None);
    assert_eq!(grid.neighbours((2, 2)).count(), 9);
    assert_eq!(grid.neighbours((0, 0)).count(), 4);

    let mut grid = Grid::new(GridSize {
        width: 3,
        height: 2,
    });
    grid.insert((0, 0), 7);
    grid.insert((2, 0), 9);
    grid.insert((2, 1), 8);
    let at = |grid: &Grid, cell| grid.cell(cell).to_vec();
    grid.shift(1, 0);
    assert_eq!(at(&grid, (1, 0)), [9]);
    assert_eq!(at(&grid, (0, 1)), []);
    assert_eq!(at(&grid, (1, 1)), [8]);
    grid.shift(0, 1);
    assert_eq!(at(&grid, (1, 0)), [8]);
    assert!(at(&grid, (1, 1)).is_empty());

    let mut grid = Grid::new(GridSize {
        width: 3,
        height: 2,
    });
    grid.insert((0, 0), 7);
    grid.insert((1, 1), 8);
    grid.shift(-1, 0);
    assert_eq!(at(&grid, (1, 0)), [7]);
    // The last column of each row is cleared: the body that wrapped into
    // the next row's last cell is gone.
    assert!(at(&grid, (2, 1)).is_empty());
}

/// A body tree of four bodies; bodies wake only through child nodes that
/// have a body, but settling and disabling pass every node.
fn tree() -> (World, usize) {
    let mut world = World::default();
    let node = |world: &mut World, parent: Option<usize>| {
        let n = world.nodes.len();
        world.nodes.push(Node::new(1));
        if let Some(p) = parent {
            world.nodes[p].children.push(n);
        }
        n
    };
    let root = node(&mut world, None);
    let a = node(&mut world, Some(root));
    let plain = node(&mut world, Some(root));
    let under = node(&mut world, Some(plain));
    let b = node(&mut world, Some(root));
    let child = node(&mut world, Some(b));
    for (n, class) in [(a, CLASS_LIVING), (under, CLASS_LIVING), (b, 0), (child, 0)] {
        let body = world.bodies.len();
        world
            .bodies
            .push(Body::new([Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]));
        world.bodies[body].class = class;
        world.bodies[body].node = Some(n);
        world.nodes[n].body = Some(body);
    }
    (world, root)
}

/// Waking a node sets its body tree's countdown and clears the awake bit;
/// the walk stops at a node without a body.
#[test]
fn waking_follows_the_body_tree() {
    let (mut world, root) = tree();
    let a = world.nodes[root].children[0];
    let body = world.body_of(a).unwrap();
    world.bodies[body].countdown = 100;
    world.bodies[body].state |= state::AWAKE;
    world.wake(root, 64);
    // The root has no body: nothing wakes from it.
    assert_eq!(world.bodies[body].countdown, 100);
    world.wake(a, 64);
    assert_eq!(world.bodies[body].countdown, 64);
    assert_eq!(world.bodies[body].state & state::AWAKE, 0);
    // The plain node's body under it is not reached.
    let plain = world.nodes[root].children[1];
    let under = world.nodes[plain].children[0];
    let under_body = world.body_of(under).unwrap();
    world.wake(plain, 64);
    assert_eq!(world.bodies[under_body].countdown, 0);
    // The root's own child with a body is reached through the root.
    let b = world.nodes[root].children[2];
    let b_body = world.body_of(b).unwrap();
    world.bodies[b_body].countdown = 3;
    world.wake(b, 256);
    assert_eq!(world.bodies[b_body].countdown, 256);
    let child = world.nodes[b].children[0];
    assert_eq!(world.bodies[world.body_of(child).unwrap()].countdown, 256);
    // A body that is not due takes no wake.
    world.bodies[b_body].countdown = 200;
    world.wake_at_least(b_body, 100);
    assert_eq!(world.bodies[b_body].countdown, 200);
    world.wake_at_least(b_body, 300);
    assert_eq!(world.bodies[b_body].countdown, 300);
}

/// The settling state reaches living bodies through every node.
#[test]
fn settling_reaches_living_bodies() {
    let (mut world, root) = tree();
    world.settle(root, 16);
    let mut settled = Vec::new();
    for b in &world.bodies {
        if b.state & state::SETTLE != 0 {
            settled.push((b.class, b.countdown));
        }
    }
    assert_eq!(settled, [(CLASS_LIVING, 16), (CLASS_LIVING, 16)]);
}

/// Freezing a body clears its sleep bits, sets frozen and kinematic,
/// places its previous particles at the matrix (or where it is) and sets
/// the countdown to two; unfreezing clears the two bits.
#[test]
fn a_body_freezes_at_a_matrix() {
    let (mut world, root) = tree();
    let node = world.nodes[root].children[0];
    let body = world.body_of(node).unwrap();
    let at = Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0));
    world.bodies[body].state = state::ASLEEP | state::DEEP | state::SETTLE;
    world.freeze(node, Some(at));
    let b = &world.bodies[body];
    assert_eq!(b.countdown, 2);
    assert_eq!(b.state, state::FROZEN | state::KINEMATIC | state::SETTLE);
    for i in 0..4 {
        assert_eq!(b.positions[i], at.transform_point3(b.rest[i]));
        assert_eq!(b.previous[i], b.positions[i]);
    }
    assert_eq!(b.matrix, at);
    world.unfreeze(node);
    assert_eq!(world.bodies[body].state, state::SETTLE);
    // Without a matrix the particles stay and the matrix comes from them.
    world.bodies[body].positions[1] = Vec3::new(2.0, 1.0, 1.0);
    world.freeze(node, None);
    let b = &world.bodies[body];
    assert_eq!(b.previous, b.positions);
    assert_eq!(b.matrix, b.transform());
}

/// The tree flags: disabling marks every node, enabling clears them, and a
/// living body is made kinematic when its bodies are disabled.
#[test]
fn the_tree_flags_walk_every_node() {
    let (mut world, root) = tree();
    world.disable_tree(root);
    assert!(world.nodes.iter().all(|n| n.state & state::DISABLED != 0));
    world.enable_tree(root);
    assert!(world.nodes.iter().all(|n| n.state & state::DISABLED == 0));
    world.disable_bodies(root);
    assert_eq!(world.bodies[0].state, state::DISABLED | state::KINEMATIC);
    assert_eq!(world.bodies[1].state, state::DISABLED | state::KINEMATIC);
    assert_eq!(world.bodies[2].state, state::DISABLED);
    assert_eq!(world.bodies[3].state, state::DISABLED);
}

/// The grab layer clears on the body, its parent and its children; a grab
/// link drops and disables the node's body, and the held bit clears from a
/// whole subtree.
#[test]
fn grabs_and_holds_clear_their_flags() {
    let mut world = World::default();
    for _ in 0..3 {
        world.nodes.push(Node::new(1));
    }
    for i in 0..3 {
        let mut b = Body::new([Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]);
        b.parent = (i == 1).then_some(0);
        b.children = if i == 0 { vec![1, 2] } else { Vec::new() };
        b.layer = if i == 2 {
            LAYER_GRAB
        } else {
            LAYER_GRAB | 0x100
        };
        b.state = state::HELD;
        b.held = 9;
        b.class = CLASS_LIVING;
        world.bodies.push(b);
        world.nodes[i].body = Some(i);
    }
    world.nodes[0].children.push(1);
    world.nodes[0].children.push(2);
    world.bodies[1].links.push(LinkRef {
        parent: Some(0),
        flags: link::GRAB,
    });
    world.bodies[1].links.push(LinkRef {
        parent: Some(0),
        flags: link::GRAB | link::REANCHORED,
    });
    world.clear_pair_layer(1);
    assert_eq!(world.bodies[0].layer, 0x100);
    assert_eq!(world.bodies[1].layer, 0x100);
    // A sibling keeps its layer: only the parent and the children clear.
    assert_eq!(world.bodies[2].layer, LAYER_GRAB);
    world.bodies[1].layer = LAYER_GRAB;
    world.drop_grab_link(1);
    assert_eq!(world.bodies[1].state & state::DISABLED, state::DISABLED);
    assert_eq!(world.bodies[1].links.len(), 1);
    assert_eq!(
        world.bodies[1].links[0].flags & link::REANCHORED,
        link::REANCHORED
    );
    world.clear_held(0);
    assert!(world.bodies.iter().all(|b| b.state & state::HELD == 0));
    world.hold_tree(0);
    assert!(world.bodies.iter().all(|b| b.state & state::HELD != 0));
    assert_eq!(world.bodies[0].held, 0);
    world.release_tree(0);
    assert!(world.bodies.iter().all(|b| b.state & state::HELD == 0));
    world.bodies[0].state |= state::HELD;
    world.pairs = vec![BodyPair { a: 0, b: 1 }, BodyPair { a: 1, b: 2 }];
    world.remove_held_pairs();
    assert_eq!(world.pairs, [BodyPair { a: 1, b: 2 }]);
}

/// A sector purges the nodes, bodies, pairs, objects and other nodes that
/// are not in its scene (or that are, for a removed sub-scene), and takes
/// them out of its grid.
#[test]
fn a_sector_purges_what_left_its_scene() {
    let mut world = World::default();
    world.nodes.push(Node::new(1));
    world.nodes.push(Node::new(2));
    world.nodes[0].body = Some(0);
    world.nodes[1].body = Some(1);
    world.nodes[0].position = Vec3::ZERO;
    world.nodes[1].position = Vec3::X * 600.0;
    for _ in 0..2 {
        world
            .bodies
            .push(Body::new([Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]));
    }
    world.bodies[0].node = Some(0);
    world.bodies[0].scene = 1;
    world.bodies[0].cell = Some((2, 2));
    world.bodies[1].node = Some(1);
    world.bodies[1].scene = 2;
    world.bodies[1].cell = Some((3, 2));
    world.pairs = vec![BodyPair { a: 0, b: 0 }, BodyPair { a: 1, b: 1 }];
    world.sim_objects = vec![SimObject { node: 0 }, SimObject { node: 1 }];
    let mut sector = Sector::new(1);
    sector.nodes = vec![0, 1];
    sector.bodies = vec![0, 1];
    sector.pairs = vec![0, 1];
    sector.sim_objects = vec![0, 1];
    sector.others = vec![0, 1];
    let size = GridSize {
        width: 5,
        height: 5,
    };
    sector.build_grid(&world, size);
    assert_eq!(sector.grid.as_ref().unwrap().cell((2, 2)), [0]);
    assert_eq!(sector.grid.as_ref().unwrap().cell((3, 2)), [1]);
    world.nodes[1].state = state::REGISTERED;
    sector.purge_left(&mut world);
    assert_eq!(sector.nodes, [0]);
    assert_eq!(sector.bodies, [0]);
    assert_eq!(sector.pairs, [0]);
    assert_eq!(sector.sim_objects, [0]);
    assert_eq!(sector.others, [0]);
    assert_eq!(world.nodes[1].state & state::REGISTERED, 0);
    assert!(sector.grid.as_ref().unwrap().cell((3, 2)).is_empty());

    // Removing the sub-scene of scene 1 takes what purge_left kept.
    sector.nodes.push(1);
    sector.bodies.push(1);
    world.nodes[1].scene = 1;
    sector.purge_scene(&mut world, 1);
    assert!(sector.nodes.is_empty() && sector.bodies.is_empty());
}

/// A sector's flags are served in order; a dead sector reports itself.
#[test]
fn the_update_serves_the_sector_flags() {
    let mut world = World::default();
    world.nodes.push(Node::new(1));
    world.nodes.push(Node::new(2));
    world.nodes[0].body = Some(0);
    world.nodes[1].body = Some(1);
    world
        .bodies
        .push(Body::new([Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]));
    world
        .bodies
        .push(Body::new([Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]));
    world.bodies[0].node = Some(0);
    world.bodies[1].node = Some(1);
    world.bodies[1].links.push(LinkRef {
        parent: None,
        flags: 0,
    });
    world.bodies[1].links.push(LinkRef {
        parent: Some(0),
        flags: 0,
    });
    let mut sector = Sector::new(2);
    sector.nodes = vec![0, 1];
    sector.bodies = vec![0, 1];
    sector.flags = sector::WORLD_LINKS | sector::SCENE_LEFT;
    assert!(!sector.update(&mut world));
    assert_eq!(sector.nodes, [1]);
    assert_eq!(sector.bodies, [1]);
    assert_eq!(world.bodies[1].links.len(), 1);
    assert_eq!(sector.flags, 0);
    sector.flags = sector::DEAD;
    assert!(sector.update(&mut world));
    assert!(sector.nodes.is_empty() && sector.bodies.is_empty());
}

/// Pair masks meet when they share a layer bit (not the lowest) and their
/// group bits shifted one meet either body.
#[test]
fn pair_masks_meet_by_layer_and_group() {
    assert!(masks_meet(0x30, 0x30));
    assert!(masks_meet(0x30, 0x10));
    assert!(!masks_meet(0x30, 0x2));
    assert!(!masks_meet(1, 0x30));
    assert!(!masks_meet(0, 0));
}

/// The placement query walks the nine cells, skips the body's own tree,
/// disabled bodies and masks that cannot meet, and asks the closure in the
/// order's direction.
#[test]
fn a_placement_query_walks_the_grid() {
    let mut world = World::default();
    for _ in 0..3 {
        world
            .bodies
            .push(Body::new([Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]));
    }
    for (i, b) in world.bodies.iter_mut().enumerate() {
        b.layer = 0x30;
        b.order = [5, 3, 9][i] as u32;
        b.cell = Some((2, 2));
        b.scene = 1;
    }
    world.bodies[2].state = state::DISABLED;
    let mut grid = Grid::new(GridSize {
        width: 5,
        height: 5,
    });
    for i in 0..3 {
        grid.insert((2, 2), i);
    }
    let mut asked = Vec::new();
    let free = world.space_free(&grid, 0, |a, b| {
        asked.push((a.order, b.order));
        a.order == 3
    });
    assert!(!free);
    // Body 1 (order 3) moves first; body 2 is disabled.
    assert_eq!(asked, [(3, 5)]);
    // Its own tree is skipped.
    world.bodies[1].root = Some(0);
    world.bodies[0].root = Some(0);
    assert!(world.space_free(&grid, 0, |_, _| true));
    // And with nothing overlapping, the placement is free.
    grid.clear();
    assert!(world.space_free(&grid, 0, |_, _| true));
    // A body with no cell is free.
    world.bodies[0].cell = None;
    assert!(world.space_free(&grid, 0, |_, _| true));
}

/// The watch list appends, removes by swapping with the last, and hands
/// its listeners to the caller.
#[test]
fn the_watch_list_removes_by_swap() {
    let mut sim = Simulation::new(STEP_MS);
    sim.watch_add(3);
    sim.watch_add(4);
    sim.listeners = vec![9];
    assert_eq!(sim.watch_remove(3), (true, vec![9]));
    assert_eq!(sim.watch, [4]);
    assert_eq!(sim.watch_remove(7), (false, Vec::new()));
    assert_eq!(sim.watch, [4]);
}

/// A simulation's turn advances the clock and drops its dead sectors.
#[test]
fn a_simulation_turn_advances_and_drops_dead_sectors() {
    let mut sim = Simulation::new(STEP_MS);
    sim.sectors.push(Sector::new(1));
    let mut dead = Sector::new(2);
    dead.flags = sector::DEAD;
    sim.sectors.push(dead);
    let mut world = World::default();
    sim.turn(&mut world);
    assert_eq!((sim.clock.tick, sim.clock.time), (1, STEP_MS));
    assert_eq!(sim.sectors.len(), 1);
    assert_eq!(sim.sectors[0].scene, 1);
}
