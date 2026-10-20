//! Character footprints on the navigation penalty.
//!
//! The centre cell gains 63, its eight neighbours 32, and the twelve cells
//! beyond them 16, on the layer that contains the standing height. The outer
//! five cells of the grid are left alone. Removing a footprint takes the
//! same amounts back. Evidence: the movement notes' footprints.

use std::collections::HashMap;

use bevy::math::Vec3;
use bevy::prelude::{Entity, Resource};

use super::query::{axis, voxels};
use super::{Layer, NavGrid};

const MARGIN: i32 = 5;
const CENTRE: u8 = 255;
const RING: u8 = 128;
const OUTER: u8 = 64;

/// The eight cells around a centre, orthogonal first.
const RING_OFFSETS: [(i32, i32); 8] = [
    (0, -1),
    (-1, 0),
    (1, 0),
    (0, 1),
    (-1, -1),
    (1, -1),
    (-1, 1),
    (1, 1),
];

const AXIS_OFFSETS: [(i32, i32); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];

/// Add a character's footprint at `standing`. Returns whether a cell was written.
pub fn stamp_footprint(grid: &mut NavGrid, standing: Vec3) -> bool {
    apply(grid, standing, true)
}

/// Take back a footprint stamped at `standing`.
pub fn remove_footprint(grid: &mut NavGrid, standing: Vec3) -> bool {
    apply(grid, standing, false)
}

/// Where each character's footprint is stamped on the grid, so that it can
/// be taken back before the character stamps it again where it stands
/// now, or when it leaves.
#[derive(Resource, Debug, Default)]
pub struct NavFootprints {
    stamped: HashMap<Entity, Vec3>,
}

impl NavFootprints {
    /// Moves `who`'s footprint to `standing`: the old one taken back, the
    /// new one stamped (none recorded off the grid's inner cells).
    pub fn stamp(&mut self, grid: &mut NavGrid, who: Entity, standing: Vec3) {
        self.lift(grid, who);
        if stamp_footprint(grid, standing) {
            self.stamped.insert(who, standing);
        }
    }

    /// Takes `who`'s footprint off the grid, if it has one.
    pub fn lift(&mut self, grid: &mut NavGrid, who: Entity) {
        if let Some(at) = self.stamped.remove(&who) {
            remove_footprint(grid, at);
        }
    }

    /// Takes off the footprints of the characters `keep` refuses (those
    /// that left).
    pub fn retain(&mut self, grid: &mut NavGrid, keep: impl Fn(Entity) -> bool) {
        let gone: Vec<Entity> = self.stamped.keys().copied().filter(|e| !keep(*e)).collect();
        for who in gone {
            self.lift(grid, who);
        }
    }

    /// Forgets every footprint: a new grid has none on it.
    pub fn forget(&mut self) {
        self.stamped.clear();
    }

    /// Where `who`'s footprint is stamped.
    pub fn of(&self, who: Entity) -> Option<Vec3> {
        self.stamped.get(&who).copied()
    }
}

fn apply(grid: &mut NavGrid, standing: Vec3, add: bool) -> bool {
    let (width, depth) = grid.size();
    let x = axis(standing.x, width).round_ties_even() as i32;
    let z = axis(standing.z, depth).round_ties_even() as i32;
    if x < MARGIN || z < MARGIN || x > width as i32 - MARGIN || z > depth as i32 - MARGIN {
        return false;
    }
    let height = voxels(standing.y, grid.base());
    let Some(layer) = layer_at(grid, x, z, height) else {
        return false;
    };
    let patch = patch(grid, x, z, height, layer);
    if patch.is_empty() {
        return false;
    }
    for (cx, cz, value) in patch {
        let mut cell = grid.cell(cx as usize, cz as usize, layer);
        let amount = value >> 2;
        cell.penalty = if add {
            cell.penalty.wrapping_add(amount)
        } else {
            cell.penalty.wrapping_sub(amount)
        };
        grid.set_cell(cx as usize, cz as usize, layer, cell);
    }
    true
}

fn patch(grid: &NavGrid, x: i32, z: i32, height: i32, layer: Layer) -> Vec<(i32, i32, u8)> {
    let mut out = Vec::new();
    if !on_layer(grid, x, z, height, layer) {
        return out;
    }
    out.push((x, z, CENTRE));
    let mut ring = Vec::new();
    for (dx, dz) in RING_OFFSETS {
        let (nx, nz) = (x + dx, z + dz);
        if on_layer(grid, nx, nz, height, layer) {
            ring.push((nx, nz));
            out.push((nx, nz, RING));
        }
    }
    let mut outer: HashMap<(i32, i32), u8> = HashMap::new();
    let listed: std::collections::HashSet<(i32, i32)> = ring.iter().copied().collect();
    for (rx, rz) in &ring {
        for (dx, dz) in AXIS_OFFSETS {
            let p = (rx + dx, rz + dz);
            if p == (x, z) || listed.contains(&p) {
                continue;
            }
            if !on_layer(grid, p.0, p.1, height, layer) {
                continue;
            }
            let value = outer.entry(p).or_insert(0);
            *value = value.saturating_add(OUTER);
        }
    }
    out.extend(outer.into_iter().map(|((x, z), value)| (x, z, value)));
    out
}

fn layer_at(grid: &NavGrid, x: i32, z: i32, height: i32) -> Option<Layer> {
    if on_layer(grid, x, z, height, Layer::Lower) {
        Some(Layer::Lower)
    } else if on_layer(grid, x, z, height, Layer::Upper) {
        Some(Layer::Upper)
    } else {
        None
    }
}

fn on_layer(grid: &NavGrid, x: i32, z: i32, height: i32, layer: Layer) -> bool {
    let (width, depth) = grid.size();
    if x < 0 || z < 0 || x as usize >= width || z as usize >= depth {
        return false;
    }
    let cell = grid.cell(x as usize, z as usize, layer);
    if cell.floor == 0 {
        return false;
    }
    let underside = i32::from(cell.underside);
    let floor = i32::from(cell.floor);
    underside - 3 < height && height < floor + 25
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::{Layer, NavCell, obstacle};

    fn yard() -> NavGrid {
        let mut grid = NavGrid::empty(16, 16, 0.0);
        for z in 0..16 {
            for x in 0..16 {
                grid.set_cell(
                    x,
                    z,
                    Layer::Lower,
                    NavCell {
                        floor: 10,
                        ..NavCell::default()
                    },
                );
            }
        }
        grid
    }

    fn stand(grid: &NavGrid, x: usize, z: usize) -> Vec3 {
        let centre = grid.cell_centre(x, z);
        Vec3::new(centre.x, 1.0, centre.y)
    }

    fn penalty(grid: &NavGrid, x: usize, z: usize) -> u8 {
        grid.cell(x, z, Layer::Lower).penalty
    }

    #[test]
    fn a_footprint_adds_sixty_three_and_the_obstacle_test_sees_it() {
        let mut grid = yard();
        let standing = stand(&grid, 8, 8);
        assert!(stamp_footprint(&mut grid, standing));
        assert_eq!(penalty(&grid, 8, 8), 63);
        assert_eq!(penalty(&grid, 8, 7), 32);
        assert_eq!(penalty(&grid, 7, 7), 32);
        assert_eq!(penalty(&grid, 8, 6), 16);
        assert_eq!(penalty(&grid, 0, 0), 0);

        let from = stand(&grid, 6, 8);
        let to = stand(&grid, 11, 8);
        assert!(obstacle(&grid, from, to));

        assert!(remove_footprint(&mut grid, standing));
        assert_eq!(penalty(&grid, 8, 8), 0);
        assert_eq!(penalty(&grid, 8, 7), 0);
        assert_eq!(penalty(&grid, 8, 6), 0);
        assert!(!obstacle(&grid, from, to));

        let edge = stand(&grid, 2, 8);
        assert!(!stamp_footprint(&mut grid, edge));
        assert_eq!(penalty(&grid, 2, 8), 0);

        let mut high = yard();
        for z in 0..16 {
            for x in 0..16 {
                high.set_cell(
                    x,
                    z,
                    Layer::Upper,
                    NavCell {
                        floor: 50,
                        underside: 40,
                        ..NavCell::default()
                    },
                );
            }
        }
        let above = {
            let centre = high.cell_centre(8, 8);
            Vec3::new(centre.x, 5.0, centre.y)
        };
        assert!(stamp_footprint(&mut high, above));
        assert_eq!(high.cell(8, 8, Layer::Lower).penalty, 0);
        assert_eq!(high.cell(8, 8, Layer::Upper).penalty, 63);
    }
}
