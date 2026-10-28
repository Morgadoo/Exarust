//! The sector's bookkeeping, pinned to the original's numbers
//! (`context/re/body-tick-order.md` §1, `body-pairs.md` §2.2 and §3, the
//! sector routines' assembly): the 600 cm cells and their round-down and
//! round-to-even arithmetic, the wide span and its one-cell margin, the
//! row overflow guards, the slab meet test and the overlap tests, and the
//! step's stage order.
use bevy::math::{Affine3A, Vec3};

use super::*;

/// A sphere set's bounds are the box around the spheres themselves (their
/// radii included) and the sphere enclosing them all.
#[test]
fn sphere_bounds_include_their_radii() {
    let b = Bounds::of_spheres(&[(Vec3::ZERO, 10.0), (Vec3::new(100.0, 0.0, 0.0), 20.0)]);
    assert_eq!(b.centre, Vec3::new(55.0, 0.0, 0.0));
    assert_eq!(b.half, Vec3::new(65.0, 20.0, 20.0));
    assert_eq!(b.radius, 65.0);
    let empty = Bounds::of_spheres(&[]);
    assert_eq!((empty.half, empty.radius), (Vec3::ZERO, 0.0));
}

/// The enclosing radius ignores a sphere's offset within the original's
/// epsilon (cm²) of the centre, and a single sphere's box is its own,
/// with no minimum extent.
#[test]
fn sphere_bounds_follow_the_originals_epsilon() {
    // Both centres are 1e-6 m (1e-8 cm²) from the box centre.
    let b = Bounds::of_spheres(&[(Vec3::ZERO, 1.0), (Vec3::new(2e-6, 0.0, 0.0), 1.0)]);
    assert_eq!(b.radius, 1.0);
    let point = Bounds::of_spheres(&[(Vec3::new(1.0, 2.0, 3.0), 0.0)]);
    assert_eq!(point.centre, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(point.half, Vec3::ZERO);
}

/// A point's cell floors at 600 cm (multiplied by the single-precision
/// 1/600) and keeps its sign; NaN is the processor's indefinite.
#[test]
fn cells_are_600_cm_and_floor() {
    assert_eq!(INVERSE_CELL_SIZE_CM.to_bits(), 0x3ada_740e);
    assert_eq!(cell_coordinate(f32::NAN), i32::MIN);
    assert_eq!(cell_coordinate(1e30), i32::MIN);
    assert_eq!(cell_coordinate(0.0), 0);
    assert_eq!(cell_coordinate(599.9), 0);
    assert_eq!(cell_coordinate(600.0), 1);
    assert_eq!(cell_coordinate(-0.1), -1);
    assert_eq!(cell_coordinate(-600.0), -1);
    assert_eq!(cell_coordinate(-600.1), -2);
    assert_eq!(cell_of(Vec3::new(1250.0, 9.0, -1.0), (3, -2)), (5, -3));
    assert_eq!(cell_index((2, 3), 8), 26);
}

/// The static insertion's span: a bound inside one cell goes into the
/// centre cell alone; one reaching past it is wide and covers the cells
/// from one below the lower edge to one above the upper edge, both axes,
/// exactly the original's capacity.
#[test]
fn a_wide_bound_covers_its_cells_and_margin() {
    let single = grid_span(
        Vec3::new(1500.0, 0.0, 1500.0),
        Vec3::splat(100.0),
        8,
        6,
        (0, 0),
    );
    assert_eq!(single.centre, (2, 2));
    assert!(!single.wide);
    assert_eq!(single.insertion_cells(), [(2, 2)]);

    let wide = grid_span(
        Vec3::new(1500.0, 0.0, 1500.0),
        Vec3::new(700.0, 100.0, 700.0),
        8,
        6,
        (0, 0),
    );
    assert!(wide.wide);
    // Lower edge 800/600 = 1.333 rounds to 1; upper 2200/600 = 3.667 to 4.
    assert_eq!(
        (wide.min_x, wide.max_x, wide.min_z, wide.max_z),
        (1, 4, 1, 4)
    );
    let cells = wide.insertion_cells();
    assert_eq!(cells.len(), 36);
    assert_eq!(cells.len(), wide.capacity());
    assert_eq!(cells[..2], [(0, 0), (1, 0)]);
    assert_eq!(*cells.last().unwrap(), (5, 5));

    // The upper edge is capped at the last cell but one; the lower floor is
    // cell 1.
    let edge = grid_span(
        Vec3::new(1200.0, 0.0, 300.0),
        Vec3::new(800.0, 100.0, 100.0),
        4,
        4,
        (0, 0),
    );
    assert!(edge.wide);
    assert_eq!((edge.min_x, edge.max_x), (1, 2));
    let floor = grid_span(
        Vec3::new(-900.0, 0.0, 100.0),
        Vec3::new(100.0, 100.0, 100.0),
        8,
        8,
        (0, 0),
    );
    assert_eq!(floor.min_x, 1);
}

/// Only the first and last grid rows are the original's position-overflow
/// guard; an x outside the row wraps and is kept. A sim object's guard
/// also takes the first cell of the second row.
#[test]
fn the_overflow_guard_is_on_rows_only() {
    let (columns, rows) = (8, 6);
    for index in [0, 1, 7, 40, 41, -1] {
        assert!(body_cell_overflow(index, columns, rows), "{index}");
        assert!(object_cell_overflow(index, columns, rows), "{index}");
    }
    for index in [9, 20, 39] {
        assert!(!body_cell_overflow(index, columns, rows), "{index}");
        assert!(!object_cell_overflow(index, columns, rows), "{index}");
    }
    assert!(!body_cell_overflow(8, columns, rows));
    assert!(object_cell_overflow(8, columns, rows));
}

/// A sparse cell grid inserts, migrates and removes ids, and reaches the
/// occupied cells of the 3 × 3 block around a part.
#[test]
fn the_cell_grid_buckets_and_migrates() {
    let mut grid = CellGrid::new();
    grid.insert((2, 2), 4);
    grid.insert((2, 2), 5);
    assert!(!grid.insert_unique((2, 2), 4));
    assert!(grid.insert_unique((2, 2), 8));
    assert_eq!(grid.get((2, 2)), [4, 5, 8]);
    grid.insert((1, 2), 6);
    grid.insert((2, 3), 7);
    assert_eq!(grid.get((2, 2)), [4, 5, 8]);
    assert_eq!(grid.occupied(), 3);
    let near: Vec<(i32, i32)> = grid.near((2, 2)).map(|(cell, _)| cell).collect();
    assert_eq!(near, [(1, 2), (2, 2), (2, 3)]);
    grid.migrate((2, 2), (3, 3), 4);
    assert_eq!(grid.get((2, 2)), [5, 8]);
    assert_eq!(grid.get((3, 3)), [4]);
    grid.remove((2, 2), 5);
    assert_eq!(grid.get((2, 2)), [8]);
    assert_eq!(grid.occupied(), 4);
    grid.insert_span(
        GridSpan {
            centre: (4, 4),
            min_x: 3,
            max_x: 5,
            min_z: 3,
            max_z: 5,
            wide: true,
        },
        9,
    );
    assert_eq!(grid.get((2, 3)), [7, 9]);
    assert_eq!(grid.get((5, 5)), [9]);
    grid.clear();
    assert!(grid.is_empty());
}

/// The slab test: the segment against the box grown by the radius on each
/// axis, in units of the travel. A pass through the box hits with both ends
/// outside it; an entry at the end of the travel misses; a resting sphere
/// meets the grown box, corners included.
#[test]
fn a_moving_sphere_meets_a_box_by_slabs() {
    let (box_centre, box_half) = (Vec3::new(100.0, 0.0, 0.0), Vec3::splat(10.0));
    let hit = |start: Vec3, end: Vec3| swept_box_hit(start, end, 10.0, box_centre, box_half);
    // Entry at a third of the travel.
    assert!(hit(Vec3::new(79.5, 0.0, 0.0), Vec3::new(81.0, 0.0, 0.0)));
    assert!(!hit(Vec3::new(79.0, 0.0, 0.0), Vec3::new(79.0, 0.0, 0.0)));
    assert!(hit(Vec3::new(-100.0, 0.0, 0.0), Vec3::new(83.0, 0.0, 0.0)));
    // Entry at 180/179 of the travel: past its end.
    assert!(!hit(Vec3::new(-100.0, 0.0, 0.0), Vec3::new(79.0, 0.0, 0.0)));
    // Entry exactly at the end misses.
    assert!(!hit(Vec3::new(70.0, 0.0, 0.0), Vec3::new(80.0, 0.0, 0.0)));
    // A resting sphere 11.3 cm from the box's corner is inside the grown
    // box: the corners count, unlike a true sphere–box distance.
    let corner = Vec3::new(118.0, 18.0, 0.0);
    assert!(hit(corner, corner));
    assert!(intersect::sphere_aabb_separated(
        corner, 10.0, box_centre, box_half
    ));
    // A NaN start drops its axis from the entry, as the processor's
    // maximum does, and the rest decides.
    assert!(hit(
        Vec3::new(f32::NAN, 0.0, 0.0),
        Vec3::new(81.0, 0.0, 0.0)
    ));
}

fn meeting(sphere: Bounds, mesh: Bounds, at: Vec3) -> Meeting {
    Meeting {
        sphere,
        mesh,
        previous: Affine3A::from_translation(at),
        current: Affine3A::from_translation(at),
    }
}

/// The two-way meet test: the first sphere against the second mesh box,
/// then the second mesh sphere against the first sphere box; both must
/// pass, and a swept first sphere catches a pass the end points miss.
#[test]
fn the_meet_test_checks_both_ways() {
    let a = meeting(
        Bounds::of_spheres(&[(Vec3::ZERO, 100.0)]),
        Bounds::of_box(Vec3::ZERO, Vec3::splat(100.0)),
        Vec3::ZERO,
    );
    let b = meeting(
        Bounds::of_spheres(&[(Vec3::ZERO, 10.0)]),
        Bounds::of_box(Vec3::ZERO, Vec3::splat(50.0)),
        Vec3::new(150.0, 0.0, 0.0),
    );
    assert!(meet(&a, &b));
    let far = meeting(
        Bounds::of_spheres(&[(Vec3::ZERO, 10.0)]),
        Bounds::of_box(Vec3::ZERO, Vec3::splat(50.0)),
        Vec3::new(500.0, 0.0, 0.0),
    );
    assert!(!meet(&a, &far));
    let mut swept = meeting(
        Bounds::of_spheres(&[(Vec3::ZERO, 10.0)]),
        Bounds::of_box(Vec3::ZERO, Vec3::splat(10.0)),
        Vec3::ZERO,
    );
    swept.previous = Affine3A::from_translation(Vec3::new(-200.0, 0.0, 0.0));
    swept.current = Affine3A::from_translation(Vec3::new(200.0, 0.0, 0.0));
    let crossed = meeting(
        Bounds::of_spheres(&[(Vec3::ZERO, 10.0)]),
        Bounds::of_box(Vec3::ZERO, Vec3::splat(10.0)),
        Vec3::ZERO,
    );
    assert!(meet(&swept, &crossed));
    // The static variant is the same test without the sweep.
    assert!(!static_meet(&swept, &crossed));
    assert!(!static_meet(&swept, &far));
}

/// The overlap-only test measures a point's overhang by its last axis
/// outside the box, not their sum: 5 cm over on x and y is 25 cm², within
/// a 6 cm sphere (36 cm²) although the summed 50 cm² is not.
#[test]
fn the_static_meet_takes_the_last_overhang() {
    let a = meeting(
        Bounds::of_spheres(&[(Vec3::ZERO, 6.0)]),
        Bounds::of_box(Vec3::ZERO, Vec3::splat(6.0)),
        Vec3::ZERO,
    );
    let b = meeting(
        Bounds::of_spheres(&[(Vec3::ZERO, 1.0)]),
        Bounds::of_box(Vec3::ZERO, Vec3::splat(10.0)),
        Vec3::new(15.0, 15.0, 0.0),
    );
    assert!(static_meet(&a, &b));
    let c = meeting(
        Bounds::of_spheres(&[(Vec3::ZERO, 1.0)]),
        Bounds::of_box(Vec3::ZERO, Vec3::splat(10.0)),
        Vec3::new(15.0, 17.0, 0.0),
    );
    assert!(!static_meet(&a, &c));
}

/// The placement overlap inflates the other box by 2 cm and its bounding
/// radius; the static overlap uses the bare radii.
#[test]
fn placement_and_static_overlaps_use_their_padding() {
    let centre = Vec3::ZERO;
    let half = Vec3::splat(50.0);
    let radius = half.length();
    assert!(placement_meet(
        Vec3::new(60.0, 0.0, 0.0),
        10.0,
        centre,
        half,
        radius
    ));
    assert!(!placement_meet(
        Vec3::new(70.0, 0.0, 0.0),
        10.0,
        centre,
        half,
        radius
    ));
    assert!(!placement_meet(
        Vec3::new(98.0, 0.0, 0.0),
        10.0,
        centre,
        half,
        radius
    ));
    assert!(static_overlap_meet(
        Vec3::new(60.0, 0.0, 0.0),
        10.0,
        centre,
        half,
        radius
    ));
    assert!(!static_overlap_meet(
        Vec3::new(61.0, 0.0, 0.0),
        10.0,
        centre,
        half,
        radius
    ));
    // The box touches, but the centres are further apart than the two
    // radii.
    assert!(!static_overlap_meet(
        Vec3::new(60.0, 0.0, 0.0),
        10.0,
        centre,
        half,
        40.0
    ));
}

/// The all-pairs step's point-to-mesh-box distance: the point in the
/// part's frame, the overhangs squared and summed.
#[test]
fn the_mesh_box_distance_sums_the_overhangs() {
    let mesh = Bounds::of_box(Vec3::ZERO, Vec3::splat(10.0));
    let at = Affine3A::from_translation(Vec3::new(100.0, 0.0, 0.0));
    assert_eq!(
        mesh_box_distance2(Vec3::new(113.0, 14.0, 0.0), &mesh, &at),
        25.0
    );
    assert_eq!(
        mesh_box_distance2(Vec3::new(105.0, 0.0, 0.0), &mesh, &at),
        0.0
    );
    let turned = Affine3A::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let far = mesh_box_distance2(Vec3::new(0.0, 13.0, 0.0), &mesh, &turned);
    assert!((far - 9.0).abs() < 1e-4, "{far}");
}

/// The weight helpers: negatives dropped, the sum normalised, then the
/// largest normalised to 1 with φ its reciprocal (1 on one particle, 4 at
/// the centroid); no positive weight gives nothing.
#[test]
fn the_weight_normalisations_are_the_originals() {
    assert_eq!(largest_abs([-1.0, 2.0, -3.0, 0.5]), 3.0);
    // The second weight is compared with the first, and the processor's
    // maximum gives its second operand on a NaN: a NaN first weight carries
    // into the next compare and gives way there, a NaN second one at once.
    assert_eq!(largest_abs([f32::NAN, 2.0, 0.0, 0.0]), 0.0);
    assert_eq!(largest_abs([2.0, f32::NAN, 0.0, 0.0]), 2.0);
    assert_eq!(
        normalise_sum([1.0, 0.0, 0.0, 0.0]),
        Some([1.0, 0.0, 0.0, 0.0])
    );
    assert_eq!(
        normalise_sum([1.0, 0.0, 0.0, 1.0]),
        Some([0.5, 0.0, 0.0, 0.5])
    );
    assert_eq!(normalise_sum([-1.0, 0.0, 0.0, 0.0]), None);
    assert_eq!(normalise_sum([0.0; 4]), None);
    assert_eq!(
        normalise_to_max([1.0, 0.0, 0.0, 0.0]),
        Some(([1.0, 0.0, 0.0, 0.0], 1.0))
    );
    assert_eq!(normalise_to_max([0.25; 4]), Some(([1.0; 4], 4.0)));
    assert_eq!(normalise_to_max([-1.0, -1.0, -1.0, -1.0]), None);
}

/// The span uses the processor's round-half-to-even, not the floor used
/// for a point: half a cell rounds to the even cell (0), a hair past it to
/// the next; the grid's origin is added afterwards.
#[test]
fn the_span_rounds_half_to_even_and_keeps_the_origin() {
    let half = grid_span(Vec3::ZERO, Vec3::splat(300.0), 32, 32, (10, 20));
    assert_eq!(half.centre, (10, 20));
    assert_eq!((half.min_x, half.max_x), (10, 10));
    assert!(!half.wide);
    let past = grid_span(Vec3::ZERO, Vec3::splat(301.0), 32, 32, (10, 20));
    assert_eq!((past.min_x, past.max_x), (9, 11));
    assert!(past.wide);
}

/// The step runs in the original's order, and its budgets are the
/// original's 20 iterations, 8 friction passes and 4 pair points.
#[test]
fn the_step_stages_and_budgets_are_the_originals() {
    assert_eq!(
        Stage::ORDER,
        [
            Stage::SimObjectsAgainstBodies,
            Stage::CharacterTicks,
            Stage::Bodies,
            Stage::Iterations,
            Stage::ClearPairs,
            Stage::BoundsAndGrid,
            Stage::SimObjects,
            Stage::BodyPairs,
            Stage::SimObjectContacts,
            Stage::BodyLevel,
            Stage::Friction,
            Stage::PostStep,
        ]
    );
    assert_eq!(ITERATIONS, 20);
    assert_eq!(FRICTION_PASSES, 8);
    assert_eq!(PAIR_POINTS, 4);
    assert_eq!(CELL_SIZE_CM, 600.0);
}

/// The sector's cached body values are the matrix from the particles and
/// the rest basis, the world box and sphere of the bounds under it, and the
/// cell of the box centre (cm, floored): a translated unit body keeps its
/// half extents, a quarter-turned one swaps x and y.
#[test]
fn a_body_pose_is_its_matrix_bounds_and_cell() {
    let rest = [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z];
    let inverse = bevy::math::Mat3::from_cols(Vec3::X, Vec3::Y, Vec3::Z).inverse();
    let mesh = Bounds {
        centre: Vec3::ZERO,
        half: Vec3::new(1.0, 2.0, 3.0),
        radius: 3.7416575,
    };
    let spheres = Bounds {
        centre: Vec3::new(0.1, 0.0, 0.0),
        half: Vec3::ZERO,
        radius: 0.2,
    };
    let moved = rest.map(|p| p + Vec3::new(1.0, 2.0, 3.0));
    let pose = body_pose(&rest, &inverse, &moved, &mesh, &spheres, (2, -3));
    assert_eq!(
        pose.matrix,
        Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0))
    );
    assert_eq!(pose.box_centre, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(pose.box_half, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(pose.box_radius, 3.7416575);
    assert_eq!(pose.sphere_centre, Vec3::new(1.1, 2.0, 3.0));
    assert_eq!(pose.sphere_radius, 0.2);
    // x = 100 cm → floor(1/6) = 0, plus 2; z = 300 cm → floor(1/2) = 0, −3.
    assert_eq!(pose.cell, (2, -3));

    // A quarter turn about z: x′ = y, y′ = −x; the box's half extents swap.
    let turned = [Vec3::ZERO, Vec3::Y, Vec3::NEG_X, Vec3::Z];
    let pose = body_pose(&rest, &inverse, &turned, &mesh, &spheres, (0, 0));
    assert_eq!(pose.box_half, Vec3::new(2.0, 1.0, 3.0));
}

/// An overflowed body goes back to its particles of the tick before last,
/// and parks just past the first grid row.
#[test]
fn an_overflow_resets_to_the_tick_before_last() {
    let before = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(1.0, 0.0, 1.0),
    ];
    let mut current = before.map(|p| p + Vec3::splat(9.0));
    let mut previous = before.map(|p| p + Vec3::splat(4.0));
    reset_overflow(&mut current, &mut previous, &before);
    assert_eq!(current, before);
    assert_eq!(previous, before);
    assert_eq!(overflow_cell(32), 33);
    assert_eq!(overflow_cell(-1), 0);
}

/// The registration walk: a body node builds its body (unless the scene
/// already built it), a simulated object registers with the sector's
/// object list, a mesh node registers a static collider, and everything
/// else is skipped.
#[test]
fn node_flags_decide_what_registers() {
    assert_eq!(
        registration(0x1_0000, 0x1000_0000, true, 0, 0),
        Registration::Body
    );
    assert_eq!(
        registration(0x1_0000, 0x1000_0000, true, 0x1_0000, 0),
        Registration::GridOnly
    );
    assert_eq!(
        registration(0x1_0000, 0x2000_0000, true, 0, 0),
        Registration::SimObject
    );
    assert_eq!(
        registration(0x1_0000, 0x1000_0000, false, 0, 0),
        Registration::None
    );
    assert_eq!(
        registration(0x9_0000, 0x1000_0000, true, 0, 0),
        Registration::None
    );
    assert_eq!(
        registration(0x0, 0x1000_0000, true, 0, 0),
        Registration::None
    );
    assert_eq!(
        registration(0x89, 0x1000_0000, false, 0, 0x20_0000),
        Registration::Static
    );
    assert_eq!(
        registration(0x89, 0x1000_0000, false, 0, 0x10_0000),
        Registration::None
    );
}

/// The registration's impact sound scales: v = min(2m, 1) + 1, its squared
/// reciprocal, and the mean of v and its root.
#[test]
fn impact_scales_follow_the_mobility() {
    let slow = impact_scale(0.05, true);
    assert!(slow.reactions);
    assert!((slow.squared_reciprocal - 1.0 / 1.21).abs() < 1e-7);
    assert!((slow.volume - (1.1_f32.sqrt() + 1.1) * 0.5).abs() < 1e-7);
    let fast = impact_scale(1.0, false);
    assert!(!fast.reactions);
    assert_eq!(fast.squared_reciprocal, 0.25);
    assert_eq!(fast.volume, 1.7071068);
    // The processor's minimum gives way to 1 on a NaN mobility.
    assert_eq!(impact_scale(f32::NAN, true).volume, 1.7071068);
}

/// The level detection's masks: a collider is 1 and gains 4 when flagged or
/// thin and horizontal; a hung body is 1, a living foot 4, and a pair is
/// skipped only when the masks share a bit.
#[test]
fn level_masks_follow_the_collider_and_body_rules() {
    assert_eq!(static_mask(false, 10.0, 0.0), MASK_SMALL);
    assert_eq!(static_mask(true, 10.0, 0.0), MASK_SMALL | MASK_THIN);
    // Thin is under 1 cm, and horizontal must be over 0.99.
    assert_eq!(static_mask(false, 0.5, 0.995), MASK_SMALL | MASK_THIN);
    assert_eq!(static_mask(false, 1.0, 0.995), MASK_SMALL);
    assert_eq!(static_mask(false, 0.5, 0.99), MASK_SMALL);
    assert_eq!(body_mask(true, false), MASK_SMALL);
    assert_eq!(body_mask(false, true), MASK_THIN);
    assert_eq!(body_mask(true, true), MASK_SMALL | MASK_THIN);
    assert!(!level_meets(MASK_SMALL, MASK_SMALL));
    assert!(!level_meets(MASK_THIN, MASK_THIN));
    assert!(level_meets(MASK_THIN, MASK_SMALL));
    assert!(level_meets(0, MASK_SMALL | MASK_THIN));
}

/// The static grid's insertion: a collider inside its centre's cell goes
/// there alone; a wider one takes the wide bit and every cell of its span
/// with one cell of margin, as the capacity says. A scan finds its own
/// cell's colliders first, takes a wide one only from the own cell, and
/// rejects shared masks and the body's own parent node.
#[test]
fn the_static_grid_scans_own_cell_then_neighbours() {
    let mut grid = StaticGrid::new(8, 8, (0, 0));
    let small = grid.insert(StaticBody {
        centre_cm: Vec3::new(300.0, 0.0, 300.0),
        half_cm: Vec3::splat(100.0),
        mask: MASK_SMALL,
        node: 7,
    });
    let wide = grid.insert(StaticBody {
        centre_cm: Vec3::new(1500.0, 0.0, 1500.0),
        half_cm: Vec3::new(700.0, 50.0, 700.0),
        mask: MASK_THIN,
        node: 9,
    });
    assert_eq!(grid.len(), 2);
    assert_eq!(grid.get(small).unwrap().mask, MASK_SMALL);
    assert_eq!(grid.get(wide).unwrap().mask, MASK_THIN | WIDE_COLLIDER);
    // The small collider's own cell is (0, 0); the wide one covers
    // x and z 0..=5 (its edges 1 − 1 … 4 + 1).
    assert_eq!(grid.cells().get((0, 0)), [small, wide]);
    assert_eq!(grid.cells().get((5, 5)), [wide]);
    let mut out = Vec::new();
    grid.near((0, 0), 0, 0, &mut out);
    assert_eq!(out, [small, wide]);
    // From (5, 5) the wide collider is the own cell's; a body further out
    // does not see it again. The wide one is not appended twice.
    out.clear();
    grid.near((5, 5), 0, 0, &mut out);
    assert_eq!(out, [wide]);
    out.clear();
    grid.near((7, 7), 0, 0, &mut out);
    assert!(out.is_empty());
    // A body under node 7 never sees the small collider; a mask-1 body
    // skips the mask-1 small one and keeps the mask-4 wide one.
    out.clear();
    grid.near((0, 0), 0, 7, &mut out);
    assert_eq!(out, [wide]);
    out.clear();
    grid.near((0, 0), MASK_SMALL, 0, &mut out);
    assert_eq!(out, [wide]);
    out.clear();
    grid.near((0, 0), MASK_THIN, 0, &mut out);
    assert_eq!(out, [small]);
    // The own cell is scanned before the neighbours, in the original's
    // table order: (-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0),
    // (-1, 1), (0, 1), (1, 1).
    let mut order = StaticGrid::new(8, 8, (0, 0));
    let a = order.insert(StaticBody {
        centre_cm: Vec3::new(300.0, 0.0, -300.0),
        half_cm: Vec3::splat(50.0),
        mask: 0,
        node: 1,
    });
    let b = order.insert(StaticBody {
        centre_cm: Vec3::new(-300.0, 0.0, 0.0),
        half_cm: Vec3::splat(50.0),
        mask: 0,
        node: 1,
    });
    let c = order.insert(StaticBody {
        centre_cm: Vec3::new(900.0, 0.0, 900.0),
        half_cm: Vec3::splat(50.0),
        mask: 0,
        node: 1,
    });
    let mut got = Vec::new();
    order.near((0, 0), 0, 0, &mut got);
    assert_eq!(got, [a, b, c], "(0,-1), (-1,0), (1,1) in table order");
}

/// An object's cell index parks at `columns + 1` when it is on the first
/// cell of the second row or past the last row, exactly the sector's object
/// guard; a normal move migrates its link, and removing keeps the order of
/// the rest.
#[test]
fn the_object_list_parks_an_overflowing_cell() {
    let mut objects = ObjectGrid::new(8, 6, (0, 0));
    // Cell (0, 1): index 8, the first cell of the second row → parked.
    let low = objects.insert(Vec3::new(100.0, 0.0, 700.0));
    assert_eq!(objects.objects()[low].cell, 9);
    // Cell (2, 2) is a normal cell.
    let normal = objects.insert(Vec3::new(1500.0, 0.0, 1500.0));
    assert_eq!(objects.objects()[normal].cell, 2 * 8 + 2);
    assert_eq!(objects.at((2, 2)), [normal]);
    // Its step moves it to cell (3, 2); the link migrates.
    objects.migrate(normal, Vec3::new(2100.0, 0.0, 1500.0));
    assert_eq!(objects.objects()[normal].cell, 2 * 8 + 3);
    assert!(objects.at((2, 2)).is_empty());
    assert_eq!(objects.at((3, 2)), [normal]);
    // A cell past the last row parks at columns + 1 too.
    objects.migrate(normal, Vec3::new(100.0, 0.0, 10_000.0));
    assert_eq!(objects.objects()[normal].cell, 9);
    objects.remove(low);
    assert_eq!(objects.objects().len(), 1);
    assert_eq!(objects.objects()[0].id, 0);
    // The surviving object was id 1; it is renumbered and its link points
    // at the new id, so a later migration still finds it.
    assert_eq!(objects.at((1, 1)), [0]);
    // Cell (0, 0) is the border cell the object guard treats as an
    // overflow; a real cell in the grid, (2, 1), is index 10.
    objects.migrate(0, Vec3::new(1500.0, 0.0, 900.0));
    assert_eq!(objects.objects()[0].cell, 10);
    assert_eq!(objects.at((2, 1)), [0]);
}

/// A cell index splits into x and z with the engine's `z * columns + x`;
/// the remainder keeps an x outside the row wrapping as the 32-bit add does.
#[test]
fn a_cell_index_splits_into_its_row_and_column() {
    assert_eq!(cell_of_index(26, 8), (2, 3));
    assert_eq!(cell_of_index(-1, 8), (-1, 0));
    assert_eq!(cell_of_index(9, 8), (1, 1));
    assert_eq!(cell_of_index(5, 0), (5, 0));
}
