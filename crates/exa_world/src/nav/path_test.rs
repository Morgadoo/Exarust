//! The planner's tests (included from path.rs).

use super::*;
use crate::nav::{NavCell, NavGrid};

/// A 40 × 40 grid (planner cells −10 to 10 in reach) whose `rows` carry
/// floor 100 (10 m) at base 0. A start under voxel 15 would take the empty
/// upper layer, as the original's start rule does; shipped floors lie far
/// higher.
fn floors(rows: impl Fn(usize, usize) -> bool) -> NavGrid {
    let mut grid = NavGrid::empty(40, 40, 0.0);
    for z in 0..40 {
        for x in 0..40 {
            if rows(x, z) {
                grid.set_cell(
                    x,
                    z,
                    Layer::Lower,
                    NavCell {
                        floor: 100,
                        ..NavCell::default()
                    },
                );
            }
        }
    }
    grid
}

fn corridor() -> NavGrid {
    floors(|_, z| z == 20)
}

/// A cell's centre at height `y` (m).
fn at(grid: &NavGrid, x: usize, z: usize, y: f32) -> Vec3 {
    let centre = grid.cell_centre(x, z);
    Vec3::new(centre.x, y, centre.y)
}

fn cell_of(grid: &NavGrid, p: Vec3) -> (usize, usize) {
    grid.cell_at(p).unwrap()
}

fn set(grid: &mut NavGrid, x: usize, z: usize, edit: impl Fn(&mut NavCell)) {
    let mut cell = grid.cell(x, z, Layer::Lower);
    edit(&mut cell);
    grid.set_cell(x, z, Layer::Lower, cell);
}

#[test]
fn the_look_ahead_follows_the_facing() {
    assert!((look_ahead(0.0, Look::Walk) - 0.75).abs() < 1e-6);
    assert!((look_ahead(2.0, Look::Walk) - 0.99).abs() < 1e-6);
    assert!((look_ahead(2.0, Look::Combat) - 0.69).abs() < 1e-6);
    assert!((look_ahead(9.0, Look::Back) + 0.70).abs() < 1e-6);
}

#[test]
fn the_disturbance_sampler_is_a_stub() {
    assert_eq!(disturbance().to_bits(), 0.0f32.to_bits());
}

/// `cm · (1/30) + ½` and `(y − base) · 0.1` both round ties to even: 0 cm
/// is the tie 0.5 (cell 0), 15 cm a hair over 1, 5 cm of height the tie
/// 0.5 (voxel 0), 15 cm the tie 1.5 (voxel 2).
#[test]
fn planner_cells_and_heights_round_ties_to_even() {
    assert_eq!(planner_cell(0.0), 0);
    assert_eq!(planner_cell(0.15), 1);
    assert_eq!(planner_cell(0.30), 2);
    assert_eq!(planner_cell(-0.15), 0);
    assert_eq!(planner_cell(-0.45), -1);
    assert_eq!(planner_height(1.0, 0.0), 10);
    assert_eq!(planner_height(0.05, 0.0), 0);
    assert_eq!(planner_height(0.15, 0.0), 2);
    assert_eq!(planner_height(0.25, 0.0), 2);
    assert_eq!(planner_height(-0.15, 0.0), -2);
    assert_eq!(heuristic(2, 0, 5), 54);
    assert_eq!(heuristic(3, 4, 0), 50);
}

#[test]
fn a_step_takes_the_floor_in_reach() {
    let cell = |floor: u8, underside: u8| NavCell {
        floor,
        underside,
        ..NavCell::default()
    };
    let none = NavCell::default();
    assert_eq!(step_onto(none, cell(12, 0), 10), None);
    assert_eq!(step_onto(cell(12, 0), none, 10), Some((Layer::Lower, 12)));
    // Five up: refused under a slab less than ten above, left to the upper
    // floor otherwise, which is missing.
    assert_eq!(step_onto(cell(15, 15), none, 10), None);
    assert_eq!(step_onto(cell(15, 25), none, 10), None);
    // An upper floor in reach and higher is entered.
    assert_eq!(
        step_onto(cell(2, 0), cell(13, 0), 10),
        Some((Layer::Upper, 13))
    );
    // An upper floor out of reach: its slab must leave ten, unless nothing
    // is measured under it.
    assert_eq!(step_onto(cell(10, 0), cell(20, 15), 10), None);
    assert_eq!(
        step_onto(cell(10, 0), cell(20, 25), 10),
        Some((Layer::Lower, 10))
    );
    assert_eq!(
        step_onto(cell(10, 0), cell(20, 0), 10),
        Some((Layer::Lower, 10))
    );
}

/// Five steps east at 5 each; the goal cell's heuristic is 0, under the
/// accept distance, and the one run gives two points.
#[test]
fn a_straight_corridor_is_walked_to_the_goal() {
    let grid = corridor();
    let path = find_path(&grid, at(&grid, 12, 20, 10.0), at(&grid, 17, 20, 10.0), 0);
    assert!(path.reached);
    assert_eq!(path.cost, 25);
    assert_eq!(
        path.points,
        vec![at(&grid, 12, 20, 10.0), at(&grid, 17, 20, 10.0)]
    );
}

/// A penalty byte of 4 is paid once, shifted by the query's shift; shifted
/// past 600 it refuses the cell.
#[test]
fn the_penalty_is_shifted_and_capped() {
    let mut grid = corridor();
    set(&mut grid, 15, 20, |c| c.penalty = 4);
    let (from, to) = (at(&grid, 12, 20, 10.0), at(&grid, 17, 20, 10.0));
    assert_eq!(find_path(&grid, from, to, 0).cost, 29);
    assert_eq!(find_path_with(&grid, from, to, 0, 10, 1).cost, 33);
    // 4 << 8 is 1024: the cell is refused and the closest node, cell 14,
    // lies 90 cm from the goal at 17: reached by the 100 cm rule.
    let blocked = find_path_with(&grid, from, to, 0, 10, 8);
    assert!(blocked.reached);
    assert_eq!(blocked.cost, 10);
    assert_eq!(cell_of(&grid, *blocked.points.last().unwrap()), (14, 20));
    // The shift is masked to 31, as the original's shift count is.
    assert_eq!(find_path_with(&grid, from, to, 0, 10, 33).cost, 33);
}

/// The heuristic's height leg is measured to the goal's height: a goal
/// 50 cm over the floor gives cell 15 the heuristic 54, the first under an
/// accept distance of 55 (from the start's height, cell 13 would give 40).
#[test]
fn the_height_leg_runs_to_the_goal() {
    let grid = corridor();
    let path = find_path_with(
        &grid,
        at(&grid, 12, 20, 10.0),
        at(&grid, 17, 20, 10.5),
        0,
        55,
        0,
    );
    assert!(path.reached);
    assert_eq!(path.cost, 15);
    assert_eq!(cell_of(&grid, *path.points.last().unwrap()), (15, 20));
}

/// With a gap at cell 16 the closest node, cell 15, lies 120 cm from the
/// goal at 19: not reached, and the path stops at the parent of the rough
/// node nearest the start.
#[test]
fn a_failed_path_stops_before_rough_ground() {
    let mut grid = corridor();
    set(&mut grid, 16, 20, |c| c.floor = 0);
    let (from, to) = (at(&grid, 12, 20, 10.0), at(&grid, 19, 20, 10.0));
    let plain = find_path(&grid, from, to, 0);
    assert!(!plain.reached);
    assert_eq!(plain.cost, 15);
    assert_eq!(cell_of(&grid, *plain.points.last().unwrap()), (15, 20));

    // Clearance 70 (over 64) on the closest node: the path ends one back.
    set(&mut grid, 15, 20, |c| c.clearance = 70);
    let rough = find_path(&grid, from, to, 0);
    assert!(!rough.reached);
    assert_eq!(rough.cost, 10);
    assert_eq!(rough.points, vec![from, at(&grid, 14, 20, 10.0)]);

    // Rough ground next to the start: the end is the start, no points.
    set(&mut grid, 13, 20, |c| c.clearance = 70);
    let none = find_path(&grid, from, to, 0);
    assert!(!none.reached);
    assert!(none.points.is_empty());
    assert_eq!(none.cost, 0);
    // Size class 1 halves the rough limit to 32 but also the clearance
    // limit to 90: cell 13 is still entered.
    assert!(find_path(&grid, from, to, 1).points.is_empty());
}

/// The line walk steps every cell after the far corner up to and including
/// the kept point's, holds the far corner's floor and doubles the penalty
/// against the budget plus ten.
#[test]
fn the_line_walk_holds_its_floor_and_counts_every_step() {
    let mut grid = corridor();
    let far = (10, 20);
    let anchor = (15, 20);
    // Five steps of 5.
    assert!(!line_blocked(&grid, far, Layer::Lower, anchor, 15, 0));
    assert!(line_blocked(&grid, far, Layer::Lower, anchor, 14, 0));
    set(&mut grid, 12, 20, |c| c.penalty = 3);
    assert!(!line_blocked(&grid, far, Layer::Lower, anchor, 21, 0));
    assert!(line_blocked(&grid, far, Layer::Lower, anchor, 20, 0));
    // A floor of 102 from cell 11 on: each step rises 2 from the far
    // corner's 100 (40 + 5), 231 in all with the doubled penalty 6.
    for x in 11..=15 {
        set(&mut grid, x, 20, |c| c.floor = 102);
    }
    assert!(!line_blocked(&grid, far, Layer::Lower, anchor, 221, 0));
    assert!(line_blocked(&grid, far, Layer::Lower, anchor, 220, 0));
    // A gap blocks whatever the budget.
    set(&mut grid, 13, 20, |c| c.floor = 0);
    assert!(line_blocked(&grid, far, Layer::Lower, anchor, 10_000, 0));
}

#[test]
fn no_path_starts_outside_the_margin_or_too_far() {
    let grid = corridor();
    // Cell 8 is planner cell −11, past the margin of ten.
    let outside = find_path(&grid, at(&grid, 8, 20, 10.0), at(&grid, 12, 20, 10.0), 0);
    assert_eq!(outside, Path::none());
    let big = NavGrid::empty(640, 640, 0.0);
    let far = find_path(&big, at(&big, 100, 320, 10.0), at(&big, 354, 320, 10.0), 0);
    assert_eq!(far, Path::none());
}

#[test]
fn a_path_goes_around_a_wall() {
    let grid = floors(|x, z| !(x == 20 && (14..=26).contains(&z)));
    let (from, to) = (at(&grid, 16, 20, 10.0), at(&grid, 24, 20, 10.0));
    let path = find_path(&grid, from, to, 0);
    assert!(path.reached, "{:?}", path.points);
    assert!(path.points.len() > 2, "{:?}", path.points);
    assert_eq!(path.points.first(), Some(&from));
    assert_eq!(path.points.last(), Some(&to));
    for point in &path.points {
        let (x, z) = cell_of(&grid, *point);
        assert!(
            !(x == 20 && (14..=26).contains(&z)),
            "{point} is in the wall"
        );
    }
    let walked: f32 = path.points.windows(2).map(|w| w[0].distance(w[1])).sum();
    assert!(walked > from.distance(to));
}

/// A search runs many times on one thread without its buffers leaking
/// between searches.
#[test]
fn searches_reuse_their_buffers() {
    let grid = corridor();
    let (from, to) = (at(&grid, 12, 20, 10.0), at(&grid, 17, 20, 10.0));
    let first = find_path(&grid, from, to, 0);
    let walled = floors(|x, z| !(x == 20 && (14..=26).contains(&z)));
    let _ = find_path(
        &walled,
        at(&walled, 16, 20, 10.0),
        at(&walled, 24, 20, 10.0),
        0,
    );
    assert_eq!(find_path(&grid, from, to, 0), first);
}
