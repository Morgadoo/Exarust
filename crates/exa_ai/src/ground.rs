//! The pathfinder's answers the combat state asks for its places, in the
//! combat state's centimetres.
//!
//! The original's combat state asks the one installed pathfinder four
//! things: the line walk with its clamp (each place from its base, the
//! long step's continuation and the run stop), the obstacle test (the
//! chase check's sight from the fighter's ground point to its target
//! point), the cost sample (each place's ground cost, through the
//! combatant's cost formula, [`crate::combatant::candidate_cost`]) and the
//! snap (the fighter's own point at the start of a tick and each step
//! goal). The level's pathfinder never overrides the snap, so it stays the
//! abstract class's: it leaves the point where it was. Without a level
//! grid the abstract pathfinder answers: nothing blocks, no line is
//! clamped, and the cost sample is zero, which costs a place 1.
//!
//! The AI tick lifts the fighter's own footprint before it runs the state
//! and stamps it again after, so the cost sample and the obstacle test see
//! the other characters' footprints, not its own; the duellist does the
//! same around its combat ticks.
//!
//! Evidence: the combat state's calls through the installed pathfinder's
//! slots (line walk, obstacle, cost sample, snap), the combatant's cost
//! routine (the cost sample's two words, `b·40 + ((a² + a)·120 + 1)`), and
//! the AI tick's footprint lift and stamp around its state call, read from
//! the instructions.

use bevy::math::Vec3;
use exa_world::nav::{self, NavGrid};

use crate::combatant::candidate_cost;

/// Centimetres per metre: the grid answers in Exarust's metres.
const CM: f32 = 100.0;

/// A place's ground cost: the cost sample under `point_cm` (its clearance
/// and penalty) through the combatant's formula; 1 without a grid.
pub fn ground_cost(grid: Option<&NavGrid>, point_cm: Vec3) -> f32 {
    let (clearance, penalty) = grid.map_or((0.0, 0.0), |g| nav::cost_at(g, point_cm / CM));
    candidate_cost(clearance, penalty)
}

/// The line walk from `from_cm` toward `to_cm`: true when the grid stops
/// it, with `to_cm` pulled back to where it stopped (its height kept);
/// without a grid, or when nothing stops it, `to_cm` is left as it was.
pub fn walk_line(grid: Option<&NavGrid>, from_cm: Vec3, to_cm: &mut Vec3) -> bool {
    let Some(g) = grid else {
        return false;
    };
    let hit = nav::query::line_walk(g, from_cm / CM, *to_cm / CM);
    if hit.blocked {
        *to_cm = Vec3::new(hit.end.x * CM, to_cm.y, hit.end.z * CM);
    }
    hit.blocked
}

/// The obstacle test between two points: the line from `from_cm` (a
/// ground point) to `to_cm` meets a wall or a heavy penalty; never without
/// a grid.
pub fn out_of_sight(grid: Option<&NavGrid>, from_cm: Vec3, to_cm: Vec3) -> bool {
    grid.is_some_and(|g| nav::query::obstacle(g, from_cm / CM, to_cm / CM))
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_world::nav::{Layer, NavCell};

    /// A 40 × 40 grid with its floors at voxel 100 (10 m over base 0) and
    /// a wall across it (floors 3 m higher) along cell column 24.
    fn walled() -> NavGrid {
        let mut grid = NavGrid::empty(40, 40, 0.0);
        for z in 0..40 {
            for x in 0..40 {
                let floor = if x == 24 { 130 } else { 100 };
                grid.set_cell(
                    x,
                    z,
                    Layer::Lower,
                    NavCell {
                        floor,
                        ..NavCell::default()
                    },
                );
            }
        }
        grid
    }

    fn at(grid: &NavGrid, x: usize, z: usize, y_cm: f32) -> Vec3 {
        let c = grid.cell_centre(x, z);
        Vec3::new(c.x * CM, y_cm, c.y * CM)
    }

    #[test]
    fn without_a_grid_the_abstract_pathfinder_answers() {
        assert_eq!(ground_cost(None, Vec3::new(10.0, 5.0, 3.0)), 1.0);
        let mut to = Vec3::new(500.0, 130.0, 0.0);
        assert!(!walk_line(None, Vec3::ZERO, &mut to));
        assert_eq!(to, Vec3::new(500.0, 130.0, 0.0));
        assert!(!out_of_sight(None, Vec3::ZERO, to));
    }

    #[test]
    fn open_ground_costs_one_and_a_penalty_costs_forty_a_share() {
        let mut grid = walled();
        let p = at(&grid, 10, 10, 1130.0);
        assert_eq!(ground_cost(Some(&grid), p), 1.0);
        grid.set_cell(
            10,
            10,
            Layer::Lower,
            NavCell {
                floor: 100,
                penalty: 255,
                clearance: 0,
                ..NavCell::default()
            },
        );
        assert_eq!(ground_cost(Some(&grid), p), 41.0);
        grid.set_cell(
            10,
            10,
            Layer::Lower,
            NavCell {
                floor: 100,
                clearance: 255,
                ..NavCell::default()
            },
        );
        assert_eq!(ground_cost(Some(&grid), p), 241.0);
    }

    #[test]
    fn a_wall_pulls_a_place_back_and_keeps_its_height() {
        let grid = walled();
        let from = at(&grid, 20, 20, 1000.0);
        let mut to = at(&grid, 28, 20, 1130.0);
        assert!(walk_line(Some(&grid), from, &mut to));
        // Pulled back to the cell before the wall, at the asked height.
        let before = at(&grid, 23, 20, 1130.0);
        assert!((to - before).length() < 0.5, "{to:?} vs {before:?}");
        // A line short of the wall is left alone.
        let mut short = at(&grid, 22, 20, 1130.0);
        let asked = short;
        assert!(!walk_line(Some(&grid), from, &mut short));
        assert_eq!(short, asked);
    }

    #[test]
    fn a_wall_hides_the_target() {
        let grid = walled();
        let from = at(&grid, 20, 20, 1000.0);
        assert!(out_of_sight(Some(&grid), from, at(&grid, 28, 20, 1130.0)));
        assert!(!out_of_sight(Some(&grid), from, at(&grid, 23, 20, 1130.0)));
    }
}
