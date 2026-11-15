//! The pathfinder's grid queries.
//!
//! The line walk steps from the cell after the start along the longer axis
//! (Z on a tie of |dx| and |dz|), up to and including the asked end's own
//! cell. A floor more than four above the current height blocks when
//! its underside is within ten. The straight walk uses five and fifteen,
//! and follows a floor down. A line of 40 cm or less is not an obstacle; a
//! longer one is when a floor is more than seven too high or an early
//! cell's penalty is above 60. The point queries (a cell's cost, the
//! nearest walkable point) are `super::walkable`'s. Evidence: the movement
//! notes' grid queries and the three line tests' instructions.

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
    let Some(mut walk) = raster(grid, start, end, false) else {
        return LineHit {
            blocked: false,
            end,
        };
    };
    let mut height = voxels(start.y, grid.base());
    let mut offered = walk.count as i32;
    // The original walks the cells after the start up to and including the
    // asked end's cell (its loop runs once per cell of the major span).
    for k in 0..walk.count {
        let Some((x, z)) = walk.advance() else {
            break;
        };
        match step_cell(grid, x, z, height, offered) {
            Step::Next => {}
            Step::Blocked => return walk.clamp(grid, k, start, end),
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

/// The blocked line walk's end: the backed-up cell point (grid units,
/// the minor axis unrounded) put on the floor in centimetres, projected
/// onto the start-to-end direction, which is normalised with the
/// processor's reciprocal square root estimate (a zero direction gives a
/// zero vector, so the end is the start). Only the ground axes move; the
/// height stays the asked end's. Evidence: the line walk's block arm.
fn project_back(start: Vec3, end: Vec3, x: f32, z: f32, width: usize, depth: usize) -> Vec3 {
    let cm = |m: f32| super::centimetres_of(f64::from(m)) as f32;
    let cell = super::CELL_CM as f32;
    let (sx, sz) = (cm(start.x), cm(start.z));
    let px = ((x + 0.5) - (width / 2) as f32) * cell;
    let pz = ((z + 0.5) - (depth / 2) as f32) * cell;
    let (ox, oz) = (px - sx, pz - sz);
    let (mut dx, mut dz) = (cm(end.x) - sx, cm(end.z) - sz);
    let length_sq = dz * dz + dx * dx;
    let scale = if length_sq == 0.0 {
        0.0
    } else {
        exa_formats::rfc::reciprocal_sqrt(length_sq)
    };
    dx *= scale;
    dz *= scale;
    let along = ox * dx + oz * dz;
    let metres = |c: f32| (f64::from(c) / 100.0) as f32;
    Vec3::new(metres(dx * along + sx), end.y, metres(dz * along + sz))
}

struct Raster {
    x_major: bool,
    step: i32,
    slope: f32,
    intercept: f32,
    major: i32,
    count: u32,
}

/// The walks' raster, set up as the original's: in centimetres and single
/// precision, the start's grid position `W/2 + (x·(1/30) − ½)`, the end's
/// cell `round(x·(1/30) − ½) + W/2`, the major axis, slope and step from the
/// world direction (Z on a tie or a NaN), the intercept through the start.
fn raster(grid: &NavGrid, start: Vec3, end: Vec3, sign_bit: bool) -> Option<Raster> {
    let (width, depth) = grid.size();
    let inverse = 1.0f32 / super::CELL_CM as f32;
    let cm = |m: f32| super::centimetres_of(f64::from(m)) as f32;
    let halves = [(width / 2) as i32, (depth / 2) as i32];
    let from = [cm(start.x), cm(start.z)];
    let to = [cm(end.x), cm(end.z)];
    let g = [0, 1].map(|i| halves[i] as f32 + (from[i] * inverse - 0.5));
    let end_cell = [0, 1].map(|i| ((to[i] * inverse - 0.5).round_ties_even() as i32) + halves[i]);
    let way = [to[0] - from[0], to[1] - from[1]];
    let x_major = way[1].abs() < way[0].abs();
    let (major, minor) = if x_major { (0, 1) } else { (1, 0) };
    // [Exarust: a zero direction raster nothing; the original's slope is
    // then NaN, only reached when rounding puts the end in another cell.]
    if way[major] == 0.0 {
        return None;
    }
    let start_i = g[major].round_ties_even() as i32;
    let count = (end_cell[major] - start_i).unsigned_abs();
    if count == 0 {
        return None;
    }
    let slope = way[minor] / way[major];
    let step = if sign_bit {
        if way[major].is_sign_negative() { -1 } else { 1 }
    } else if way[major] < 0.0 {
        -1
    } else {
        1
    };
    Some(Raster {
        x_major,
        step,
        slope,
        intercept: g[minor] - g[major] * slope,
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
            end: project_back(start, end, x, z, width, depth),
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

    /// A blocked walk ends at the backed-up cell point (the minor axis
    /// unrounded) projected onto the start-to-end direction, normalised
    /// with the reciprocal square root estimate: the cell (7, 30/11 + 7·3/11)
    /// of a walk from cell (1, 3) to (12, 6) walled at x = 8, in the
    /// instruction order (cm, f32), back in metres.
    #[test]
    fn a_blocked_walk_projects_its_end_with_the_estimate() {
        let mut grid = flat(16, 16);
        for z in 0..16 {
            grid.set_cell(
                8,
                z,
                Layer::Lower,
                NavCell {
                    floor: 40,
                    ..NavCell::default()
                },
            );
        }
        let (start, end) = (at(&grid, 1, 3), at(&grid, 12, 6));
        let hit = line_walk(&grid, start, end);
        assert!(hit.blocked);
        // Start (-195, -135) cm, end (135, -45) cm; the backed-up point.
        let slope = 3.0f32 / 11.0;
        let intercept = 3.0f32 - slope;
        let minor = 7.0f32 * slope + intercept;
        let (px, pz) = ((7.5f32 - 8.0) * 30.0, ((minor + 0.5) - 8.0) * 30.0);
        let (ox, oz) = (px - -195.0, pz - -135.0);
        let (dx, dz) = (330.0f32, 90.0f32);
        let r = exa_formats::rfc::reciprocal_sqrt(dz * dz + dx * dx);
        let (dx, dz) = (dx * r, dz * r);
        let along = ox * dx + oz * dz;
        let want = Vec3::new(
            ((dx * along + -195.0) as f64 / 100.0) as f32,
            end.y,
            ((dz * along + -135.0) as f64 / 100.0) as f32,
        );
        assert_eq!(hit.end, want);
        // The point already lies on the line: only the estimate moves it,
        // by well under a millimetre.
        assert!((hit.end.x - px / 100.0).abs() < 1e-3, "{:?}", hit.end);
        assert!((hit.end.z - pz / 100.0).abs() < 1e-3, "{:?}", hit.end);
        // The straight walk's block takes the same projection.
        let straight = straight_walk(&grid, start, end);
        assert!(straight.blocked);
        assert_eq!(straight.end, want);
    }

    /// The raster rounds as the original's single precision: at -180 cm on
    /// a 16-cell grid, `8 + (-180·(1/30) − ½)` is 1.4999995, so the walk
    /// starts in cell 1 (the exact 1.5 would round to cell 2) and its first
    /// step meets a floor too high in cell 2: blocked at once, it keeps its
    /// start.
    #[test]
    fn the_raster_rounds_in_single_precision() {
        let mut grid = flat(16, 16);
        grid.set_cell(
            2,
            3,
            Layer::Lower,
            NavCell {
                floor: 40,
                ..NavCell::default()
            },
        );
        let start = Vec3::new(-1.8, 1.0, at(&grid, 1, 3).z);
        let end = at(&grid, 6, 3);
        let hit = line_walk(&grid, start, end);
        assert!(hit.blocked);
        assert_eq!(hit.end, start);
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
    fn a_line_walk_enters_the_asked_cells_own_cell() {
        // The original's loop runs once per cell of the major span, so the
        // end cell is walked: a wall there blocks the line, one just past
        // it does not.
        let wall = NavCell {
            floor: 40,
            ..NavCell::default()
        };
        let mut grid = flat(8, 8);
        grid.set_cell(7, 3, Layer::Lower, wall);
        let start = at(&grid, 1, 3);
        let end = at(&grid, 6, 3);
        assert!(!line_walk(&grid, start, end).blocked);
        assert!(!straight_walk(&grid, start, end).blocked);
        assert!(!obstacle(&grid, start, end));
        grid.set_cell(6, 3, Layer::Lower, wall);
        assert!(line_walk(&grid, start, end).blocked);
        assert!(straight_walk(&grid, start, end).blocked);
        assert!(obstacle(&grid, start, end));
    }

    /// On a tie of |dx| and |dz| the original walks along Z: a wall on the
    /// diagonal's first Z step blocks, whichever axis would reach it.
    #[test]
    fn a_diagonal_line_steps_along_z() {
        let grid = flat(8, 8);
        let start = at(&grid, 1, 1);
        let end = at(&grid, 4, 4);
        let walk = raster(&grid, start, end, true).unwrap();
        assert!(!walk.x_major);
        assert_eq!(walk.count, 3);
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
