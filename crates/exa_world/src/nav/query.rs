//! The pathfinder's grid queries.
//!
//! The line walk steps from the cell after the start along the longer axis.
//! A floor more than four above the current height blocks when its underside
//! is within ten. The straight walk uses five and fifteen, and follows a
//! floor down. A line of 40 cm or less is not an obstacle; a longer one is
//! when a floor is more than seven too high or an early cell's penalty is
//! above 60. The point queries (a cell's cost, the nearest walkable point)
//! are `super::walkable`'s. Evidence: the movement notes' grid queries.

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

/// The floors as built (before any mover raised one) with the current
/// undersides: what the straight walk reads.
fn built_floors(grid: &NavGrid, x: i32, z: i32) -> ((u8, u8), (u8, u8)) {
    let ((_, lower), (_, upper)) = floors(grid, x, z);
    if x < 0 || z < 0 {
        return ((0, lower), (0, upper));
    }
    let (x, z) = (x as usize, z as usize);
    (
        (grid.built_floor(x, z, Layer::Lower), lower),
        (grid.built_floor(x, z, Layer::Upper), upper),
    )
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

pub(crate) fn axis(metres: f32, n: usize) -> f32 {
    (super::centimetres_of(f64::from(metres)) / super::CELL_CM - 0.5 + (n / 2) as f64) as f32
}

pub(crate) fn voxels(y: f32, base: f32) -> i32 {
    (f64::from(y - base) / super::VOXEL_CM * 100.0).round_ties_even() as i32
}

fn place(g: f32, n: usize) -> f32 {
    ((g + 0.5) - (n / 2) as f32) * super::CELL
}

struct Raster {
    x_major: bool,
    step: i32,
    slope: f32,
    intercept: f32,
    major: i32,
    count: u32,
}

fn raster(grid: &NavGrid, start: Vec3, end: Vec3, sign_bit: bool) -> Option<Raster> {
    let (width, depth) = grid.size();
    let gx0 = axis(start.x, width);
    let gz0 = axis(start.z, depth);
    let dx = axis(end.x, width) - gx0;
    let dz = axis(end.z, depth) - gz0;
    let x_major = dx.abs() >= dz.abs();
    let (d_major, d_minor, g_major, g_minor) = if x_major {
        (dx, dz, gx0, gz0)
    } else {
        (dz, dx, gz0, gx0)
    };
    if d_major == 0.0 {
        return None;
    }
    let start_i = g_major.round_ties_even() as i32;
    let end_i = (g_major + d_major).round_ties_even() as i32;
    let count = (end_i - start_i).unsigned_abs();
    if count == 0 {
        return None;
    }
    let slope = d_minor / d_major;
    let step = if sign_bit {
        if d_major.is_sign_negative() { -1 } else { 1 }
    } else if d_major < 0.0 {
        -1
    } else {
        1
    };
    Some(Raster {
        x_major,
        step,
        slope,
        intercept: g_minor - g_major * slope,
        major: start_i,
        count,
    })
}

/// Walk a straight line. A rise of five stays walkable. The height becomes
/// the floor just stood on, so a later rise is measured from there. It reads
/// the floors as built, as the original's does, not those movers raised.
pub fn straight_walk(grid: &NavGrid, start: Vec3, end: Vec3) -> LineHit {
    let Some(mut walk) = raster(grid, start, end, true) else {
        return LineHit {
            blocked: false,
            end,
        };
    };
    let mut height = straight_height(grid, start);
    for k in 0..walk.count {
        let Some((x, z)) = walk.advance() else {
            break;
        };
        match straight_cell(grid, x, z, height) {
            Step::Next => {}
            Step::Blocked => return walk.clamp(grid, k, start, end),
            Step::Stand { height: next, .. } => height = next,
        }
    }
    LineHit {
        blocked: false,
        end,
    }
}

/// Whether the line meets an obstacle. Forty centimetres or less does not.
/// A longer line does when a floor is more than seven too high, or when a
/// cell before the last two has a penalty above 60.
pub fn obstacle(grid: &NavGrid, start: Vec3, end: Vec3) -> bool {
    let dx = (end.x - start.x) * 100.0;
    let dz = (end.z - start.z) * 100.0;
    if dx * dx + dz * dz <= 1_600.0 {
        return false;
    }
    let Some(mut walk) = raster(grid, start, end, true) else {
        return false;
    };
    let mut height = voxels(start.y, grid.base());
    for k in 0..walk.count {
        let Some((x, z)) = walk.advance() else {
            break;
        };
        match obstacle_cell(grid, x, z, height, k + 2 < walk.count) {
            Step::Next => {}
            Step::Blocked => return true,
            Step::Stand { height: next, .. } => height = next,
        }
    }
    false
}

impl Raster {
    fn advance(&mut self) -> Option<(i32, i32)> {
        self.major += self.step;
        let minor = (self.major as f32 * self.slope + self.intercept).round_ties_even() as i32;
        Some(if self.x_major {
            (self.major, minor)
        } else {
            (minor, self.major)
        })
    }

    fn clamp(&mut self, grid: &NavGrid, k: u32, start: Vec3, end: Vec3) -> LineHit {
        if k == 0 {
            return LineHit {
                blocked: true,
                end: start,
            };
        }
        let back = self.major - self.step;
        let minor = back as f32 * self.slope + self.intercept;
        let (x, z) = if self.x_major {
            (back as f32, minor)
        } else {
            (minor, back as f32)
        };
        let (width, depth) = grid.size();
        LineHit {
            blocked: true,
            end: Vec3::new(place(x, width), end.y, place(z, depth)),
        }
    }
}

fn straight_height(grid: &NavGrid, start: Vec3) -> i32 {
    let (width, depth) = grid.size();
    let x = axis(start.x, width).round_ties_even() as i32;
    let z = axis(start.z, depth).round_ties_even() as i32;
    let h0 = voxels(start.y, grid.base());
    let (lower, upper) = built_floors(grid, x, z);
    if upper.0 != 0 {
        let underside = i32::from(upper.1);
        let floor = i32::from(upper.0);
        if underside - 3 < h0 && h0 < floor + 15 {
            return floor;
        }
    }
    i32::from(lower.0)
}

fn straight_cell(grid: &NavGrid, x: i32, z: i32, height: i32) -> Step {
    let (lower, upper) = built_floors(grid, x, z);
    if lower.0 == 0 {
        return Step::Next;
    }
    let fl = i32::from(lower.0);
    let ul = i32::from(lower.1);
    let fu = i32::from(upper.0);
    let uu = i32::from(upper.1);
    let (reached, mut offered) = if fl > height + 5 {
        if ul < height + 15 {
            return Step::Blocked;
        }
        (-1, 0)
    } else {
        (0, fl)
    };
    if upper.0 == 0 {
        if reached < 0 {
            return Step::Next;
        }
    } else if fu > height + 5 {
        if uu < height + 15 {
            return Step::Blocked;
        } else if reached < 0 {
            return Step::Next;
        }
    } else {
        offered = offered.max(fu);
    }
    Step::Stand {
        height: offered,
        offered,
    }
}

fn obstacle_cell(grid: &NavGrid, x: i32, z: i32, height: i32, penalise: bool) -> Step {
    let (lower, upper) = floors(grid, x, z);
    if lower.0 == 0 {
        return Step::Next;
    }
    let fl = i32::from(lower.0);
    let ul = i32::from(lower.1);
    let fu = i32::from(upper.0);
    let uu = i32::from(upper.1);
    let (mut layer, mut floor) = if fl > height + 7 {
        if ul < height + 15 {
            return Step::Blocked;
        }
        (-1, fl)
    } else {
        (0, fl)
    };
    if upper.0 == 0 {
        if layer < 0 {
            return Step::Next;
        }
    } else if fu > height + 7 {
        if uu < height + 15 && (uu != 0 || fu < fl) {
            return Step::Blocked;
        } else if layer < 0 {
            return Step::Next;
        }
    } else if fu > fl {
        layer = 1;
        floor = fu;
    }
    if penalise
        && layer >= 0
        && let Some(cell) = cell_at(grid, x, z, layer)
        && cell.penalty > 60
    {
        return Step::Blocked;
    }
    Step::Stand {
        height: floor,
        offered: floor,
    }
}

fn cell_at(grid: &NavGrid, x: i32, z: i32, layer: i32) -> Option<super::NavCell> {
    let (width, depth) = grid.size();
    if x < 0 || z < 0 || x as usize >= width || z as usize >= depth || !(0..=1).contains(&layer) {
        return None;
    }
    let which = if layer == 0 {
        Layer::Lower
    } else {
        Layer::Upper
    };
    Some(grid.cell(x as usize, z as usize, which))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::{Layer, NavCell, NavGrid, nearest_walkable};

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

    #[test]
    fn a_straight_walk_allows_five_and_follows_the_floor_down() {
        let mut rise = flat(8, 8);
        rise.set_cell(
            3,
            3,
            Layer::Lower,
            NavCell {
                floor: 15,
                ..NavCell::default()
            },
        );
        let start = at(&rise, 1, 3);
        let end = at(&rise, 5, 3);
        assert!(line_walk(&rise, start, end).blocked);
        assert!(!straight_walk(&rise, start, end).blocked);

        rise.set_cell(
            3,
            3,
            Layer::Lower,
            NavCell {
                floor: 16,
                ..NavCell::default()
            },
        );
        assert!(straight_walk(&rise, start, end).blocked);

        let mut down = flat(8, 8);
        down.set_cell(
            3,
            3,
            Layer::Lower,
            NavCell {
                floor: 6,
                ..NavCell::default()
            },
        );
        down.set_cell(
            4,
            3,
            Layer::Lower,
            NavCell {
                floor: 12,
                ..NavCell::default()
            },
        );
        assert!(straight_walk(&down, at(&down, 1, 3), at(&down, 6, 3)).blocked);
    }

    #[test]
    fn forty_centimetres_is_not_an_obstacle_and_a_penalty_above_sixty_is() {
        let mut grid = flat(8, 8);
        let start = at(&grid, 1, 3);
        let next = at(&grid, 2, 3);
        grid.set_cell(
            2,
            3,
            Layer::Lower,
            NavCell {
                floor: 40,
                ..NavCell::default()
            },
        );
        assert!(!obstacle(&grid, start, next));
        assert!(obstacle(&grid, start, at(&grid, 3, 3)));

        let mut costly = flat(8, 8);
        costly.set_cell(
            2,
            3,
            Layer::Lower,
            NavCell {
                floor: 10,
                penalty: 61,
                ..NavCell::default()
            },
        );
        assert!(obstacle(&costly, at(&costly, 1, 3), at(&costly, 5, 3)));
        costly.set_cell(
            2,
            3,
            Layer::Lower,
            NavCell {
                floor: 10,
                penalty: 60,
                ..NavCell::default()
            },
        );
        costly.set_cell(
            5,
            3,
            Layer::Lower,
            NavCell {
                floor: 10,
                penalty: 61,
                ..NavCell::default()
            },
        );
        assert!(!obstacle(&costly, at(&costly, 1, 3), at(&costly, 5, 3)));
    }

    #[test]
    fn a_cheap_cell_keeps_its_place_and_a_costly_one_steps_aside() {
        let grid = flat(8, 8);
        let centre = grid.cell_centre(3, 3);
        let point = Vec3::new(centre.x + 0.05, 2.0, centre.y - 0.04);
        // The flood works in the original's centimetres: the point comes
        // back within a rounding of where it was, on its floor.
        let kept = nearest_walkable(&grid, point);
        assert!(kept.distance(point.with_y(1.0)) < 1e-5, "{kept}");

        let mut costly = flat(8, 8);
        costly.set_cell(
            3,
            3,
            Layer::Lower,
            NavCell {
                floor: 10,
                penalty: 80,
                ..NavCell::default()
            },
        );
        let aside = nearest_walkable(&costly, Vec3::new(centre.x, 1.0, centre.y));
        let neighbour = costly.cell_centre(2, 2);
        assert!(aside.distance(Vec3::new(neighbour.x, 1.0, neighbour.y)) < 1e-5);
    }
}
