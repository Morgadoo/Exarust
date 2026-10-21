//! The sector's bookkeeping, pinned to the original's numbers
//! (`context/re/body-tick-order.md` §1, `body-pairs.md` §3): the 600 cm
//! cells and their floor/round-to-even arithmetic, the wide span and its
//! one-cell margin, the row overflow guard, the static and swept sphere–box
//! meet tests, and the step's stage order.
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
    assert_eq!(Bounds::of_spheres(&[]).radius, 0.0);
}

/// A point's cell floors at 600 cm and keeps its sign.
#[test]
fn cells_are_600_cm_and_floor() {
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
/// from one below the lower edge to the upper edge, both axes, with the
/// original's capacity.
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
    assert_eq!(cells.len(), 25);
    assert_eq!(cells[0], (0, 0));
    assert_eq!(*cells.last().unwrap(), (4, 4));
    assert_eq!(wide.capacity(), 36);

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
/// guard; an x outside the row wraps and is kept.
#[test]
fn the_overflow_guard_is_on_rows_only() {
    let (columns, rows) = (8, 6);
    for index in [0, 1, 8, 40, 41, -1] {
        assert!(row_overflow(index, columns, rows), "{index}");
    }
    for index in [9, 20, 39] {
        assert!(!row_overflow(index, columns, rows), "{index}");
    }
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

/// A sphere meets a box at its end when it barely moved (at most 0.1 cm² of
/// travel), and is swept when it moved more: a fast pass through a box
/// hits even with both ends outside it.
#[test]
fn a_moving_sphere_meets_a_box_swept() {
    let (box_centre, box_half) = (Vec3::new(100.0, 0.0, 0.0), Vec3::splat(10.0));
    assert!(sphere_box_meet(
        Vec3::new(79.5, 0.0, 0.0),
        Vec3::new(81.0, 0.0, 0.0),
        10.0,
        box_centre,
        box_half
    ));
    assert!(!sphere_box_meet(
        Vec3::new(79.0, 0.0, 0.0),
        Vec3::new(79.0, 0.0, 0.0),
        10.0,
        box_centre,
        box_half
    ));
    assert!(sphere_box_meet(
        Vec3::new(-100.0, 0.0, 0.0),
        Vec3::new(83.0, 0.0, 0.0),
        10.0,
        box_centre,
        box_half
    ));
    assert!(!sphere_box_meet(
        Vec3::new(-100.0, 0.0, 0.0),
        Vec3::new(79.0, 0.0, 0.0),
        10.0,
        box_centre,
        box_half
    ));
    // A NaN travel falls to the static answer, as the unordered compare
    // does.
    assert!(sphere_box_meet(
        Vec3::new(f32::NAN, 0.0, 0.0),
        Vec3::new(81.0, 0.0, 0.0),
        10.0,
        box_centre,
        box_half
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
        centre,
        half,
        radius
    ));
    assert!(!static_overlap_meet(
        Vec3::new(61.0, 0.0, 0.0),
        10.0,
        centre,
        half,
        centre,
        half,
        radius
    ));
}

/// The weight helpers: negatives dropped, the sum normalised, then the
/// largest normalised to 1 with φ its reciprocal (1 on one particle, 4 at
/// the centroid); no positive weight gives nothing.
#[test]
fn the_weight_normalisations_are_the_originals() {
    assert_eq!(largest_abs([-1.0, 2.0, -3.0, 0.5]), 3.0);
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
