use super::*;

fn clock(step: u32, last: u32) -> Clock {
    Clock {
        step,
        last,
        ..Clock::default()
    }
}

fn tetra() -> [Vec3; 4] {
    [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]
}

/// A simulation is due when the time since its base reaches its step's
/// end; more than four steps past it, its base moves so the step it takes
/// now ends at now (the backlog is dropped); a paused one keeps its base.
#[test]
fn a_late_simulation_drops_its_backlog() {
    let c = clock(12, 0);
    assert!(!c.due(11));
    assert!(c.due(12));
    assert_eq!(c.behind(48), 36);
    let mut c = clock(12, 0);
    c.drop_backlog(60);
    assert_eq!(c.last, 0, "four steps past the end is kept");
    c.drop_backlog(61);
    // 61 − 12: the step due now ends at now.
    assert_eq!(c.last, 49);
    assert!(c.due(61) && c.behind(61) == 0);
    let mut paused = Clock {
        paused: true,
        ..clock(12, 0)
    };
    paused.drop_backlog(72);
    assert_eq!(paused.last, 0);
    let mut c = clock(12, 0);
    c.advance();
    assert_eq!((c.tick, c.time), (1, 12));
    // A later step's end counts from the game time.
    c.time = 1200;
    c.drop_backlog(5000);
    assert_eq!(c.last, 5000 - 1212);
}

/// The driver takes the first simulation that is due and otherwise waits
/// for the soonest end, never more than 100 ms.
#[test]
fn the_driver_takes_the_due_simulation() {
    let mut sims = [clock(12, 10), clock(12, 0)];
    assert_eq!(pick_due(&mut sims, 20), (Due::Now, Some(1)));
    let mut quiet = [clock(12, 100)];
    assert_eq!(pick_due(&mut quiet, 104), (Due::Wait(8), None));
    assert_eq!(pick_due(&mut [], 104), (Due::Wait(IDLE_WAIT_MS), None));
    let mut far = [Clock {
        time: 500,
        ..clock(12, 0)
    }];
    assert_eq!(pick_due(&mut far, 0), (Due::Wait(IDLE_WAIT_MS), None));
    let mut late = [clock(12, 0)];
    assert_eq!(pick_due(&mut late, 1000), (Due::Now, Some(0)));
    assert_eq!(late[0].last, 988);
    // Closed simulations are swept by swapping the last one in.
    let mut list = vec![1, 2, 3, 4];
    drop_closed(&mut list, |&s| s == 1 || s == 3);
    assert_eq!(list, [4, 2]);
}

/// The time-scale helper takes the unsigned game time less the mark as a
/// signed 64-bit difference, scales it and rounds to even; a NaN answer is
/// the conversion's minimum.
#[test]
fn the_clock_scales_a_span() {
    let c = Clock {
        time: 1000,
        ..Clock::default()
    };
    assert_eq!(c.scale_since(940, 0.5), 30);
    assert_eq!(c.scale_since(995, 0.5), 2, "2.5 rounds to even");
    let c = Clock {
        time: 10,
        ..Clock::default()
    };
    assert_eq!(c.scale_since(20, 2.0), -20);
    assert_eq!(c.scale_since(0, f32::NAN), i64::MIN);
    assert_eq!(c.scale_since(0, 1e30), i64::MIN);
    // The unsigned time stays positive: 2³² − 1 less −1 is 2³².
    let c = Clock {
        time: u32::MAX,
        ..Clock::default()
    };
    assert_eq!(c.scale_since(-1, 1.0), 4_294_967_296);
}

/// The default environment keeps 0.426, 0.25 and 0.05 a second above 40,
/// above 20 and at rest; the mechanism environment 0.001 everywhere; the
/// inert one nothing; only the default has gravity.
#[test]
fn the_environments_band_their_retention() {
    for (moved, want) in [
        (50.0 / 83.0, 0.989_77),
        (30.0 / 83.0, 0.983_44),
        (10.0 / 83.0, 0.964_55),
    ] {
        let got = Environment::DEFAULT.retention(moved);
        assert!((got - want).abs() < 1e-5, "{moved} cm: {got}");
    }
    assert!((Environment::MECHANISM.retention(100.0) - 0.920_14).abs() < 1e-4);
    assert_eq!(Environment::INERT.retention(100.0), 0.0);
    let g = Environment::DEFAULT.gravity_per_tick();
    assert!((g.y + 0.001_423_5).abs() < 1e-7, "{g}");
    assert_eq!(Environment::INERT.gravity_per_tick(), Vec3::ZERO);
    assert_eq!(Environment::MECHANISM.gravity_per_tick(), Vec3::ZERO);
}

/// A sector's cell rounds its node's x and z times 1/600 to even, plus the
/// bias; a body's cell rounds down from its sector's; the nine offsets
/// start at the cell itself.
#[test]
fn the_grid_places_sectors_and_bodies() {
    let grid = Grid::new(GridSize {
        width: 5,
        height: 4,
    });
    assert_eq!(grid.bias, [2, 2]);
    assert_eq!(grid.offsets, [0, -6, -5, -4, -1, 1, 4, 5, 6]);
    assert_eq!(grid.sector_cell(Vec3::ZERO), [2, 2]);
    assert_eq!(grid.sector_cell(Vec3::X * 600.0), [3, 2]);
    // −0.5 rounds to even (−0), −0.6667 to −1.
    assert_eq!(grid.sector_cell(Vec3::Z * -300.0), [2, 2]);
    assert_eq!(grid.sector_cell(Vec3::Z * -400.0), [2, 1]);
    // 900 cm is 1.5 cells: even, so 2.
    assert_eq!(grid.sector_cell(Vec3::X * 900.0), [4, 2]);
    // A body rounds down: −1 cm is the cell before.
    assert_eq!(grid.body_cell(Vec3::new(1.0, 0.0, 1.0), [2, 2]), Some(12));
    assert_eq!(grid.body_cell(Vec3::new(-1.0, 0.0, 1.0), [2, 2]), Some(11));
    assert_eq!(grid.body_cell(Vec3::new(0.0, 0.0, 6000.0), [2, 2]), None);
}

/// The flat-table move: cell j takes what j − offset held, the cells left
/// behind are emptied, and the row strip the original walks is cleared.
#[test]
fn the_grid_shifts_as_its_table_moves() {
    let size = GridSize {
        width: 3,
        height: 2,
    };
    let filled = || {
        let mut grid = Grid::new(size);
        for i in 0..6 {
            grid.insert(i, 10 + i);
        }
        grid
    };
    let lists = |grid: &Grid| {
        grid.cells()
            .iter()
            .map(|c| c.first().copied())
            .collect::<Vec<_>>()
    };
    // dx = 1: everything moves one cell on; cell 0 is emptied, and each
    // row's first cell.
    let mut grid = filled();
    grid.shift(1, 0);
    assert_eq!(
        lists(&grid),
        [None, Some(10), Some(11), None, Some(13), Some(14)]
    );
    // dy = 1: a whole row on; row 0 emptied.
    let mut grid = filled();
    grid.shift(0, 1);
    assert_eq!(
        lists(&grid),
        [None, None, None, Some(10), Some(11), Some(12)]
    );
    // dx = −1: everything moves one cell back, the last cell is emptied,
    // and the strip starts at column w − 2 of each row (the original's
    // start), so column 1 is emptied and column 2 keeps what wrapped in.
    let mut grid = filled();
    grid.shift(-1, 0);
    assert_eq!(
        lists(&grid),
        [Some(11), None, Some(13), Some(14), None, None]
    );
    // dy = −1: a row back; the last row emptied.
    let mut grid = filled();
    grid.shift(0, -1);
    assert_eq!(
        lists(&grid),
        [Some(13), Some(14), Some(15), None, None, None]
    );
    // No offset, no move.
    let mut grid = filled();
    grid.shift(0, 0);
    assert_eq!(grid, filled());
}

/// The matrix from four particles is the affine map from the rest points,
/// not orthonormalised.
#[test]
fn the_particle_matrix_maps_rest_to_particles() {
    let rest = [
        Vec3::new(-0.2, 0.0, -0.2),
        Vec3::new(0.2, 0.0, -0.2),
        Vec3::new(0.0, 0.0, 0.2),
        Vec3::new(0.0, 0.4, 0.0),
    ];
    let m = Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0))
        * Affine3A::from_rotation_y(0.5)
        * Affine3A::from_scale(Vec3::new(1.0, 2.0, 1.0));
    let positions = rest.map(|r| m.transform_point3(r));
    let got = particle_matrix(&rest, &positions);
    for (r, p) in rest.iter().zip(positions) {
        assert!(got.transform_point3(*r).distance(p) < 1e-5);
    }
    // The stretch is kept.
    assert!((got.matrix3.y_axis.length() - 2.0).abs() < 1e-5);
    assert_eq!(
        particle_matrix(&[Vec3::ZERO; 4], &positions),
        Affine3A::IDENTITY
    );
}

/// A body tree: a root node without a body; under it a living body, a
/// plain node with a living body below it, and a body with a child.
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
        world.bodies.push(Body::new(tetra()));
        world.bodies[body].class = class;
        world.bodies[body].node = Some(n);
        world.nodes[n].body = Some(body);
    }
    // Body 3 (on `child`) is body 2's child body.
    world.bodies[2].children = vec![3];
    world.bodies[3].parent = Some(2);
    world.bodies[3].root = Some(2);
    (world, root)
}

/// Waking a node sets its body tree's countdown and clears the resting
/// bit; the walk stops at a node without a body.
#[test]
fn waking_follows_the_body_tree() {
    let (mut world, root) = tree();
    let a = world.nodes[root].children[0];
    world.bodies[0].countdown = 100;
    world.bodies[0].state |= state::RESTING;
    world.wake(root, 64);
    assert_eq!(world.bodies[0].countdown, 100, "the root has no body");
    world.wake(a, 64);
    assert_eq!(world.bodies[0].countdown, 64);
    assert_eq!(world.bodies[0].state & state::RESTING, 0);
    let plain = world.nodes[root].children[1];
    world.wake(plain, 64);
    assert_eq!(
        world.bodies[1].countdown, 0,
        "not reached through a bare node"
    );
    let b = world.nodes[root].children[2];
    world.wake(b, 256);
    assert_eq!(world.bodies[2].countdown, 256);
    assert_eq!(world.bodies[3].countdown, 256);
    // From a child body, the root body's node wakes the whole tree.
    world.bodies[2].countdown = 0;
    world.bodies[3].countdown = 200;
    world.wake_at_least(3, 200);
    assert_eq!(world.bodies[2].countdown, 0, "200 is not under 200");
    world.wake_at_least(3, 300);
    assert_eq!(
        (world.bodies[2].countdown, world.bodies[3].countdown),
        (300, 300)
    );
}

/// Settling reaches the living bodies through every node and sets their
/// own countdown, not the sleep one.
#[test]
fn settling_reaches_living_bodies() {
    let (mut world, root) = tree();
    world.settle(root, 16);
    let settled: Vec<_> = world
        .bodies
        .iter()
        .map(|b| (b.state & state::SETTLE != 0, b.settle_ticks, b.countdown))
        .collect();
    assert_eq!(
        settled,
        [(true, 16, 0), (true, 16, 0), (false, 0, 0), (false, 0, 0)]
    );
}

/// Freezing a body clears its sleep bits, sets frozen and kinematic, puts
/// its particles at the matrix (or keeps them), empties its motion, takes
/// the matrix twice, moves its bounds and sets the countdown to two.
#[test]
fn a_body_freezes_at_a_matrix() {
    let (mut world, _) = tree();
    let at = Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0));
    {
        let b = &mut world.bodies[0];
        b.state = state::ASLEEP | state::DEEP | state::SETTLE;
        b.motion[0][1] = Vec3::ONE;
        b.bound_offset = Vec3::new(0.0, 0.5, 0.0);
    }
    world.freeze(0, Some(at));
    let b = &world.bodies[0];
    assert_eq!(b.countdown, 2);
    assert_eq!(b.state, state::FROZEN | state::KINEMATIC | state::SETTLE);
    for i in 0..4 {
        assert_eq!(b.start[i], at.transform_point3(b.rest[i]));
        assert_eq!(b.positions[i], b.start[i]);
        assert_eq!(b.previous[i], b.start[i]);
    }
    assert_eq!(b.motion, [[Vec3::ZERO; 4]; 2]);
    assert_eq!((b.matrix, b.previous_matrix), (at, at));
    assert_eq!(b.bound.centre, Vec3::new(1.0, 2.5, 3.0));
    // The quarter weights on the start points: their centroid.
    assert!(b.start_bound.centre.distance(Vec3::new(1.25, 2.25, 3.25)) < 1e-6);
    world.unfreeze(0);
    assert_eq!(world.bodies[0].state, state::SETTLE);
    world.bodies[0].positions[1] = Vec3::new(3.0, 2.0, 3.0);
    world.freeze(0, None);
    let b = &world.bodies[0];
    assert_eq!(b.start, b.positions);
    assert_eq!(b.matrix, b.transform());
}

/// Disabling and enabling walk the body tree; disabling a node's bodies
/// walks every node and makes living ones kinematic too.
#[test]
fn the_tree_flags_walk_their_trees() {
    let (mut world, root) = tree();
    world.disable_tree(2);
    assert_eq!(world.bodies[2].state, state::DISABLED);
    assert_eq!(world.bodies[3].state, state::DISABLED);
    assert_eq!(world.bodies[0].state, 0);
    world.enable_tree(2);
    assert!(world.bodies.iter().all(|b| b.state == 0));
    world.disable_bodies(root);
    let states: Vec<_> = world.bodies.iter().map(|b| b.state).collect();
    let living = state::DISABLED | state::KINEMATIC;
    assert_eq!(states, [living, living, state::DISABLED, state::DISABLED]);
}

/// The grab layer clears on the body, its parent and its children; a grab
/// link drops and disables the node's body; holding walks the body tree
/// with bit 0x400, and the marks clear through the node tree.
#[test]
fn grabs_and_holds_clear_their_flags() {
    let mut world = World::default();
    for _ in 0..3 {
        world.nodes.push(Node::new(1));
    }
    for i in 0..3 {
        let mut b = Body::new(tetra());
        b.parent = (i == 1).then_some(0);
        b.children = if i == 0 { vec![1, 2] } else { Vec::new() };
        b.layer = if i == 2 {
            LAYER_GRAB
        } else {
            LAYER_GRAB | 0x100
        };
        b.state = state::MARKED;
        b.held_ticks = 9;
        b.class = CLASS_LIVING;
        b.node = Some(i);
        world.bodies.push(b);
        world.nodes[i].body = Some(i);
    }
    world.nodes[0].children = vec![1, 2];
    world.bodies[1].links.push(LinkRef {
        parent: Some(0),
        flags: link::GRAB | link::KEPT,
    });
    world.bodies[1].links.push(LinkRef {
        parent: Some(0),
        flags: link::GRAB,
    });
    world.clear_pair_layer(1);
    assert_eq!(world.bodies[0].layer, 0x100);
    assert_eq!(world.bodies[1].layer, 0x100);
    assert_eq!(
        world.bodies[2].layer, LAYER_GRAB,
        "a sibling keeps its layer"
    );
    world.bodies[1].layer = LAYER_GRAB;
    world.drop_grab_link(1);
    assert_eq!(world.bodies[1].state & state::DISABLED, state::DISABLED);
    assert_eq!(world.bodies[1].links.len(), 1);
    assert_eq!(world.bodies[1].links[0].flags & link::KEPT, link::KEPT);
    world.clear_marks(0);
    assert!(world.bodies.iter().all(|b| b.state & state::MARKED == 0));
    world.hold_tree(0);
    assert!(world.bodies.iter().all(|b| b.state & state::HELD != 0));
    assert!(world.bodies.iter().all(|b| b.held_ticks == 0));
    world.release_tree(0);
    assert!(world.bodies.iter().all(|b| b.state & state::HELD == 0));
    world.bodies[0].state |= state::HELD;
    let mut pairs = vec![
        BodyPair { a: 0, b: 1 },
        BodyPair { a: 1, b: 2 },
        BodyPair { a: 2, b: 2 },
    ];
    world.remove_held_pairs(&mut pairs);
    // The last pair takes the removed one's place.
    assert_eq!(pairs, [BodyPair { a: 2, b: 2 }, BodyPair { a: 1, b: 2 }]);
}

/// A world of two scenes: nodes 0..3, a body on each of nodes 0..2, node 3
/// a static node of scene 2.
fn two_scenes() -> (World, Sector, Grids) {
    let mut world = World::default();
    for scene in [1, 2, 1, 2] {
        world.nodes.push(Node::new(scene));
    }
    for n in 0..3 {
        let mut b = Body::new(tetra());
        b.node = Some(n);
        b.index = n as u16;
        b.cell = 7 + n as u32;
        b.linked = true;
        world.bodies.push(b);
        world.nodes[n].body = Some(n);
        world.nodes[n].state = state::REGISTERED;
    }
    world.nodes[3].state = state::REGISTERED | state::NOTIFY;
    world.nodes[3].cells = vec![1, 2];
    let mut grids = Grids::new(GridSize {
        width: 4,
        height: 4,
    });
    for n in 0..3 {
        grids.bodies.insert(7 + n, n);
    }
    grids.nodes.insert(1, 3);
    grids.nodes.insert(2, 3);
    let mut sector = Sector::new(1);
    sector.uses_grid = true;
    sector.nodes = vec![3];
    sector.bodies = vec![0, 1, 2];
    sector.objects = vec![1, 0];
    sector.pairs = vec![BodyPair { a: 1, b: 1 }, BodyPair { a: 0, b: 2 }];
    sector.flags = sector::SCENE_LEFT;
    (world, sector, grids)
}

/// A sector purges what is not in its scene: the static node (unlinked
/// from every cell, unregistered, noticed), the body (unlinked, the others
/// renumbered in order), the objects; and every pair with a body still in
/// the scene, as the original compares.
#[test]
fn a_sector_purges_what_left_its_scene() {
    let (mut world, mut sector, mut grids) = two_scenes();
    let purged = sector.purge_left(&mut world, &mut grids);
    assert_eq!(sector.flags, 0);
    assert_eq!(
        purged,
        Purged {
            nodes: vec![3],
            bodies: vec![1]
        }
    );
    assert!(sector.nodes.is_empty());
    assert!(grids.nodes.cell(1).is_empty() && grids.nodes.cell(2).is_empty());
    assert_eq!(world.nodes[3].state, state::NOTIFY);
    assert_eq!(sector.bodies, [0, 2]);
    assert_eq!((world.bodies[0].index, world.bodies[2].index), (0, 1));
    assert!(grids.bodies.cell(8).is_empty() && !world.bodies[1].linked);
    assert_eq!(world.nodes[1].state, 0);
    assert_eq!(sector.objects, [0]);
    // The pair of body 1 (scene 2) stays; the one of bodies 0 and 2 goes.
    assert_eq!(sector.pairs, [BodyPair { a: 1, b: 1 }]);
}

/// Purging a scene takes what is of that scene, without notices.
#[test]
fn a_sector_purges_a_scene() {
    let (mut world, mut sector, mut grids) = two_scenes();
    let purged = sector.purge_scene(&mut world, &mut grids, 2);
    assert_eq!(
        purged,
        Purged {
            nodes: vec![],
            bodies: vec![1]
        }
    );
    assert!(sector.nodes.is_empty());
    assert_eq!(sector.bodies, [0, 2]);
    assert_eq!(sector.objects, [0]);
    assert_eq!(sector.pairs, [BodyPair { a: 0, b: 2 }]);
}

/// The update shifts the grids first, then serves each sector's flags in
/// order: a dead sector is removed (the rest keep their order), a reset
/// one emptied, the purges and the pending registration served; then the
/// grids are rebuilt when asked. The turn then advances the clock.
#[test]
fn the_update_serves_the_sector_flags() {
    let (mut world, sector, grids) = two_scenes();
    let size = GridSize {
        width: 4,
        height: 4,
    };
    let mut sim = Simulation::new(STEP_MS, size);
    sim.grids = grids;
    let mut dead = Sector::new(3);
    dead.flags = sector::DEAD;
    let mut reset = Sector::new(4);
    reset.flags = sector::RESET | sector::SUB_SCENE;
    reset.bodies = vec![5];
    let mut served = sector;
    served.flags |= sector::PENDING | sector::SUB_SCENE;
    served.pending = vec![9];
    sim.sectors = vec![dead, reset, served];
    sim.shift = [1, 0];
    let turns = sim.turn(&mut world, 7, 2);
    assert_eq!(sim.shift, [0, 0]);
    // The shift ran before the purge: body 2's cell 9 moved to 10, and
    // body 0's cell 7 (the end of row 1) wrapped to the start of row 2,
    // which the strip clears.
    assert_eq!(sim.grids.bodies.cell(10), [2]);
    assert!(sim.grids.bodies.cell(8).is_empty());
    assert_eq!(sim.sectors.len(), 2);
    assert_eq!((sim.sectors[0].scene, sim.sectors[1].scene), (4, 1));
    let reset = &sim.sectors[0];
    assert!(reset.bodies.is_empty());
    assert_eq!(reset.flags, sector::WAS_RESET | sector::SUB_SCENE);
    assert_eq!(turns[1].pending, [9]);
    assert_eq!(sim.sectors[1].flags, 0);
    assert_eq!((sim.clock.tick, sim.clock.time), (1, STEP_MS));
    assert!(sim.sectors.iter().all(|s| s.tick == 1));
    // A rebuild empties both grids and unlinks everything.
    sim.flags = REBUILD_GRID;
    sim.update(&mut world, 7, 2, size);
    assert!(sim.grids.bodies.cells().iter().all(|c| c.is_empty()));
    assert!(world.bodies.iter().all(|b| !b.linked));
    assert_eq!(sim.flags, 0);
}

/// Two bound spheres are apart when the gap's sign is negative.
#[test]
fn bound_spheres_part_by_the_gaps_sign() {
    let a = Bound {
        centre: Vec3::ZERO,
        radius: 1.0,
    };
    let b = Bound {
        centre: Vec3::X * 2.0,
        radius: 1.0,
    };
    assert!(!a.apart(&b), "touching");
    let c = Bound {
        centre: Vec3::X * 2.5,
        radius: 1.0,
    };
    assert!(a.apart(&c));
}

/// The placement query tests the bodies around the body's cell that are
/// not apart, not of its root, not kept apart by their layers and not
/// disabled; the lower sector index goes first; its child bodies are
/// asked in turn; a body whose cell word is 0 is not tested.
#[test]
fn a_placement_query_walks_the_grid() {
    let mut world = World::default();
    for i in 0..5 {
        let mut b = Body::new(tetra());
        b.layer = 0x10;
        b.index = [5, 3, 9, 1, 4][i];
        b.cell = 6;
        b.bound.radius = 100.0;
        b.start_bound.radius = 100.0;
        world.bodies.push(b);
    }
    world.bodies[2].state = state::DISABLED;
    // Body 3 is in another channel kept apart from 0's: exclusive 0x20.
    world.bodies[3].layer = 0x30;
    let mut grid = Grid::new(GridSize {
        width: 4,
        height: 4,
    });
    // The sector at (2, 2); bodies at the origin fall in cell 2·4 + 2.
    for i in 0..4 {
        grid.insert(10, i);
    }
    let sector = [2, 2];
    let mut asked = Vec::new();
    let mut overlaps = |a: &Body, b: &Body| {
        asked.push((a.index, b.index));
        false
    };
    assert!(!world.space_occupied(&grid, sector, 0, &mut overlaps));
    // Body 1 (index 3) goes first; 2 is disabled, 3 kept apart by layers.
    assert_eq!(asked, [(3, 5)]);
    // Of one root: not tested.
    world.bodies[1].root = Some(0);
    asked.clear();
    let mut overlaps = |a: &Body, b: &Body| {
        asked.push((a.index, b.index));
        true
    };
    assert!(!world.space_occupied(&grid, sector, 0, &mut overlaps));
    // A child body in the next cell (x + 600 cm) that meets body 1.
    world.bodies[1].root = None;
    world.bodies[0].children = vec![4];
    world.bodies[4].bound.centre = Vec3::X * 600.0;
    world.bodies[4].start_bound.centre = Vec3::X * 600.0;
    world.bodies[1].bound.centre = Vec3::X * 550.0;
    world.bodies[0].bound.radius = 0.0;
    world.bodies[0].start_bound.radius = 0.0;
    world.bodies[0].bound.centre = Vec3::X * 5000.0;
    world.bodies[0].start_bound.centre = Vec3::X * 5000.0;
    let mut hit = Vec::new();
    let mut overlaps = |a: &Body, b: &Body| {
        hit.push((a.index, b.index));
        true
    };
    assert!(world.space_occupied(&grid, sector, 0, &mut overlaps));
    assert_eq!(hit, [(3, 4)]);
    // A cell word of 0 is never tested.
    world.bodies[0].cell = 0;
    world.bodies[0].children.clear();
    assert!(!world.space_occupied(&grid, sector, 0, &mut |_, _| true));
}

/// The watch list appends, removes by swapping with the last, and hands
/// its listeners to the caller.
#[test]
fn the_watch_list_removes_by_swap() {
    let mut sim = Simulation::new(
        STEP_MS,
        GridSize {
            width: 1,
            height: 1,
        },
    );
    sim.watch_add(3);
    sim.watch_add(4);
    sim.watch_add(5);
    sim.listeners = vec![9];
    assert_eq!(sim.watch_remove(3), (true, vec![9]));
    assert_eq!(sim.watch, [5, 4]);
    assert_eq!(sim.watch_remove(7), (false, Vec::new()));
}
