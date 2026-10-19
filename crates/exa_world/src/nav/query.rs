//! The pathfinder's line walk.
//!
//! It steps from the cell after the start along the longer axis. A floor
//! more than four above the current height blocks when its underside is
//! within ten. The first blocked step keeps the start. A later one stops
//! on the previous cell and keeps the asked height. Evidence: the movement
//! notes' line walk.

use bevy::math::Vec3;

use super::{Layer, NavGrid};

/// Where a line walk stopped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineHit {
    pub blocked: bool,
    /// The asked end, or the point the walk clamped back to.
    pub end: Vec3,
}

/// Walk the straight line from `start` to `end` on `grid`.
pub fn line_walk(grid: &NavGrid, start: Vec3, end: Vec3) -> LineHit {
    let (width, depth) = grid.size();
    let gx0 = axis(start.x, width);
    let gz0 = axis(start.z, depth);
    let gx1 = axis(end.x, width);
    let gz1 = axis(end.z, depth);
    let dx = gx1 - gx0;
    let dz = gz1 - gz0;
    let x_major = dx.abs() >= dz.abs();
    let (d_major, d_minor, g_major, g_minor) = if x_major {
        (dx, dz, gx0, gz0)
    } else {
        (dz, dx, gz0, gx0)
    };
    if d_major == 0.0 {
        return LineHit {
            blocked: false,
            end,
        };
    }
    let start_i = g_major.round_ties_even() as i32;
    let end_i = (g_major + d_major).round_ties_even() as i32;
    let count = (end_i - start_i).unsigned_abs();
    if count == 0 {
        return LineHit {
            blocked: false,
            end,
        };
    }
    let step = if d_major < 0.0 { -1 } else { 1 };
    let slope = d_minor / d_major;
    let intercept = g_minor - g_major * slope;
    let mut major = start_i;
    let mut height = voxels(start.y, grid.base());
    let mut offered = count as i32;
    for k in 0..count {
        major += step;
        let minor = (major as f32 * slope + intercept).round_ties_even() as i32;
        let (x, z) = if x_major {
            (major, minor)
        } else {
            (minor, major)
        };
        match step_cell(grid, x, z, height, offered) {
            Step::Next => {}
            Step::Blocked => {
                if k == 0 {
                    return LineHit {
                        blocked: true,
                        end: start,
                    };
                }
                let back = major - step;
                let minor = back as f32 * slope + intercept;
                let (x, z) = if x_major {
                    (back as f32, minor)
                } else {
                    (minor, back as f32)
                };
                return LineHit {
                    blocked: true,
                    end: Vec3::new(place(x, width), end.y, place(z, depth)),
                };
            }
            Step::Stand {
                height: next,
                offered: kept,
            } => {
                height = next;
                offered = kept;
            }
        }
    }
    LineHit {
        blocked: false,
        end,
    }
}

enum Step {
    Next,
    Blocked,
    Stand { height: i32, offered: i32 },
}

fn step_cell(grid: &NavGrid, x: i32, z: i32, height: i32, mut offered: i32) -> Step {
    let (lower, upper) = floors(grid, x, z);
    if lower.0 == 0 {
        return Step::Next;
    }
    let fl = i32::from(lower.0);
    let ul = i32::from(lower.1);
    let fu = i32::from(upper.0);
    let uu = i32::from(upper.1);
    let reached = if fl > height + 4 {
        if ul < height + 10 {
            return Step::Blocked;
        }
        -1
    } else {
        offered = fl;
        0
    };
    if upper.0 == 0 {
        if reached < 0 {
            return Step::Next;
        }
    } else if fu > height + 4 {
        if uu < height + 10 && (uu > 0 || fu < fl) {
            return Step::Blocked;
        } else if reached < 0 {
            return Step::Next;
        }
    } else {
        offered = offered.max(fu);
    }
    Step::Stand {
        height: height.max(offered),
        offered,
    }
}

fn floors(grid: &NavGrid, x: i32, z: i32) -> ((u8, u8), (u8, u8)) {
    let (width, depth) = grid.size();
    if x < 0 || z < 0 || x as usize >= width || z as usize >= depth {
        return ((0, 0), (0, 0));
    }
    let lower = grid.cell(x as usize, z as usize, Layer::Lower);
    let upper = grid.cell(x as usize, z as usize, Layer::Upper);
    (
        (lower.floor, lower.underside),
        (upper.floor, upper.underside),
    )
}

fn axis(metres: f32, n: usize) -> f32 {
    (super::centimetres_of(f64::from(metres)) / super::CELL_CM - 0.5 + (n / 2) as f64) as f32
}

fn voxels(y: f32, base: f32) -> i32 {
    (f64::from(y - base) / super::VOXEL_CM * 100.0).round_ties_even() as i32
}

fn place(g: f32, n: usize) -> f32 {
    ((g + 0.5) - (n / 2) as f32) * super::CELL
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::{Layer, NavCell, NavGrid};

    fn flat(width: usize, depth: usize) -> NavGrid {
        let mut grid = NavGrid::empty(width, depth, 0.0);
        for z in 0..depth {
            for x in 0..width {
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

    fn at(grid: &NavGrid, x: usize, z: usize) -> Vec3 {
        let centre = grid.cell_centre(x, z);
        Vec3::new(centre.x, 1.0, centre.y)
    }

    #[test]
    fn a_clear_line_keeps_its_end_and_a_high_floor_blocks() {
        let grid = flat(8, 8);
        let start = at(&grid, 1, 3);
        let end = at(&grid, 6, 3);
        let clear = line_walk(&grid, start, end);
        assert!(!clear.blocked);
        assert_eq!(clear.end, end);

        let mut walled = flat(8, 8);
        walled.set_cell(
            4,
            3,
            Layer::Lower,
            NavCell {
                floor: 40,
                underside: 0,
                ..NavCell::default()
            },
        );
        let hit = line_walk(&walled, start, end);
        assert!(hit.blocked);
        assert_eq!(hit.end.y, end.y);
        let (x, _) = walled.cell_at(hit.end).unwrap();
        assert!(x < 4, "the walk entered the wall at {}", hit.end.x);

        let face = at(&walled, 3, 3);
        let first = line_walk(&walled, face, end);
        assert!(first.blocked);
        assert_eq!(first.end, face);
    }
}
