//! The planner's footprint and the mover's dirty rectangles.
//!
//! A plan lifts the standing footprint, searches, and stamps it again, so
//! the body does not pay for the cell it is standing on. A moved rectangle
//! joins an existing one when the union padded by six on each side is
//! strictly smaller than the two padded rectangles. Sixteen are kept. A
//! refresh grows each side by three cells and raises floors from samples
//! the caller supplies: the lower floor when it is missing or the sample
//! is less than fifteen above it, otherwise the upper floor. A sample at
//! or under the lower floor and its underside raises an empty upper floor.
//! A changed floor is written and the cost passes run again. Evidence: the
//! movement notes' planner and movers. The scene-wide countdown is not
//! here, and the samples are not read out of a scene graph.

use bevy::math::Vec3;

use super::query::{axis, voxels};
use super::{Layer, NavGrid, Path, find_path, remove_footprint, stamp_footprint};

const SLOTS: usize = 16;
const PAD: i32 = 3;
const BOX_PAD: f64 = 2.5;

/// A rectangle of cells, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRect {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
}

/// Rectangles waiting for a refresh. At most [`SLOTS`].
#[derive(Debug, Default)]
pub struct Movers {
    rects: Vec<CellRect>,
}

/// The cell rectangle around an axis-aligned box, padded by two and a half
/// cells and clamped to the margin of three.
pub fn rect_around(grid: &NavGrid, centre: Vec3, half: Vec3) -> CellRect {
    let (width, depth) = grid.size();
    let edge = |coord: f32, extent: f32, n: usize, sign: f64| {
        let cells = f64::from(extent) / f64::from(super::CELL);
        let at = f64::from(axis(coord, n)) + sign * (cells + BOX_PAD);
        at.round_ties_even() as i32
    };
    let rect = CellRect {
        x0: edge(centre.x, half.x, width, -1.0).max(PAD),
        x1: edge(centre.x, half.x, width, 1.0).min(width as i32 - PAD - 1),
        z0: edge(centre.z, half.z, depth, -1.0).max(PAD),
        z1: edge(centre.z, half.z, depth, 1.0).min(depth as i32 - PAD - 1),
    };
    rect.normalized()
}

/// Queue `rect`, merging it into the first rectangle whose padded union is
/// strictly smaller than the two padded areas. Otherwise append it, dropping
/// the oldest past sixteen.
pub fn queue_moved(movers: &mut Movers, rect: CellRect) {
    let rect = rect.normalized();
    for existing in &mut movers.rects {
        let joined = existing.union(rect);
        if padded_area(joined) < padded_area(*existing) + padded_area(rect) {
            *existing = joined;
            return;
        }
    }
    if movers.rects.len() == SLOTS {
        movers.rects.remove(0);
    }
    movers.rects.push(rect);
}

/// The rectangles waiting for a refresh.
pub fn queued_moves(movers: &Movers) -> &[CellRect] {
    &movers.rects
}

/// Lift the footprint at `standing`, search to `goal`, and stamp it again.
pub fn plan_around(grid: &mut NavGrid, standing: Vec3, goal: Vec3, size: u8) -> Path {
    remove_footprint(grid, standing);
    let path = find_path(grid, standing, goal, size);
    stamp_footprint(grid, standing);
    path
}

/// Grow each queued rectangle by three cells, raise floors from `samples`,
/// and rerun the cost passes where a floor changed. The queue is then empty.
pub fn refresh_movers(grid: &mut NavGrid, movers: &mut Movers, samples: &[Vec3]) {
    let (width, depth) = grid.size();
    let rects = std::mem::take(&mut movers.rects);
    for rect in rects {
        let x0 = (rect.x0 - PAD).max(0);
        let z0 = (rect.z0 - PAD).max(0);
        let x1 = (rect.x1 + PAD).min(width as i32 - 1);
        let z1 = (rect.z1 + PAD).min(depth as i32 - 1);
        if x0 > x1 || z0 > z1 {
            continue;
        }
        let span_x = (x1 - x0 + 1) as usize;
        let span_z = (z1 - z0 + 1) as usize;
        let mut lower = vec![0u8; span_x * span_z];
        let mut upper = vec![0u8; span_x * span_z];
        let at = |x: i32, z: i32| ((z - z0) as usize) * span_x + (x - x0) as usize;
        for z in z0..=z1 {
            for x in x0..=x1 {
                let i = at(x, z);
                lower[i] = grid.cell(x as usize, z as usize, Layer::Lower).floor;
                upper[i] = grid.cell(x as usize, z as usize, Layer::Upper).floor;
            }
        }
        for sample in samples {
            let sx = axis(sample.x, width).round_ties_even() as i32;
            let sz = axis(sample.z, depth).round_ties_even() as i32;
            let v = voxels(sample.y, grid.base());
            if !(1..255).contains(&v) || sx < x0 || sz < z0 || sx > x1 || sz > z1 {
                continue;
            }
            let underside = grid.cell(sx as usize, sz as usize, Layer::Lower).underside;
            let i = at(sx, sz);
            raise(&mut lower[i], &mut upper[i], underside, v as u8);
        }
        let mut changed = false;
        for z in z0..=z1 {
            for x in x0..=x1 {
                let i = at(x, z);
                changed |= write_floor(grid, x as usize, z as usize, Layer::Lower, lower[i]);
                changed |= write_floor(grid, x as usize, z as usize, Layer::Upper, upper[i]);
            }
        }
        if changed {
            grid.derive_costs(x0 as usize, z0 as usize, x1 as usize, z1 as usize);
        }
    }
}

fn write_floor(grid: &mut NavGrid, x: usize, z: usize, layer: Layer, floor: u8) -> bool {
    let mut cell = grid.cell(x, z, layer);
    if cell.floor == floor {
        return false;
    }
    cell.floor = floor;
    grid.set_cell(x, z, layer, cell);
    true
}

/// Raise a working floor from a sample voxel.
///
/// The lower floor takes the sample when it is missing or the sample is
/// less than fifteen above it. A higher sample raises the upper floor. A
/// sample at or under the lower floor and its underside raises an empty
/// upper floor, or one it is less than fifteen above.
fn raise(lower: &mut u8, upper: &mut u8, underside: u8, sample: u8) {
    let stood = i32::from(*lower);
    let height = i32::from(sample);
    let above = i32::from(*upper);
    let under = i32::from(underside);
    if stood < height {
        if stood == 0 || height < stood + 15 {
            *lower = sample;
        } else if above < height {
            *upper = sample;
        }
    } else if height <= stood
        && height <= under
        && (above == 0 || height < above + 15)
        && above < height
    {
        *upper = sample;
    }
}

fn padded_area(rect: CellRect) -> i64 {
    let width = i64::from(rect.x1 - rect.x0 + 1);
    let depth = i64::from(rect.z1 - rect.z0 + 1);
    (width + 6) * (depth + 6)
}

impl CellRect {
    fn normalize(mut self) -> Self {
        if self.x0 > self.x1 {
            std::mem::swap(&mut self.x0, &mut self.x1);
        }
        if self.z0 > self.z1 {
            std::mem::swap(&mut self.z0, &mut self.z1);
        }
        self
    }

    fn normalized(self) -> Self {
        self.normalize()
    }

    fn union(self, other: Self) -> Self {
        Self {
            x0: self.x0.min(other.x0),
            z0: self.z0.min(other.z0),
            x1: self.x1.max(other.x1),
            z1: self.z1.max(other.z1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::NavCell;

    fn yard() -> NavGrid {
        let mut grid = NavGrid::empty(24, 24, 0.0);
        for z in 0..24 {
            for x in 0..24 {
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

    fn at(grid: &NavGrid, x: usize, z: usize, y: f32) -> Vec3 {
        let centre = grid.cell_centre(x, z);
        Vec3::new(centre.x, y, centre.y)
    }

    fn one(x: i32) -> CellRect {
        CellRect {
            x0: x,
            z0: 10,
            x1: x,
            z1: 10,
        }
    }

    #[test]
    fn a_plan_lifts_its_footprint_and_stamps_it_again() {
        let mut grid = yard();
        let standing = at(&grid, 8, 8, 1.0);
        let goal = at(&grid, 12, 8, 1.0);
        assert!(stamp_footprint(&mut grid, standing));
        let with_foot = find_path(&grid, standing, goal, 0);
        let planned = plan_around(&mut grid, standing, goal, 0);
        assert!(with_foot.reached, "{:?}", with_foot.points);
        assert!(planned.reached, "{:?}", planned.points);
        assert!(
            planned.cost < with_foot.cost,
            "lifted {} still down {}",
            planned.cost,
            with_foot.cost
        );
        assert_eq!(grid.cell(8, 8, Layer::Lower).penalty, 63);
        assert_eq!(grid.cell(8, 7, Layer::Lower).penalty, 32);
    }

    #[test]
    fn a_dirty_rectangle_raises_the_floor_it_covers() {
        let mut grid = yard();
        let around = rect_around(&grid, at(&grid, 12, 12, 1.0), Vec3::ZERO);
        assert!(around.x0 <= 12 && around.x1 >= 12);
        assert!(around.z0 <= 12 && around.z1 >= 12);
        assert!(around.x1 > around.x0, "the pad did not reach a neighbour");
        assert!(around.x0 >= 3 && around.x1 <= 20);

        let mut near = Movers::default();
        queue_moved(&mut near, one(10));
        queue_moved(&mut near, one(12));
        assert_eq!(queued_moves(&near).len(), 1);
        assert_eq!(queued_moves(&near)[0].x0, 10);
        assert_eq!(queued_moves(&near)[0].x1, 12);

        let mut far = Movers::default();
        queue_moved(&mut far, one(10));
        queue_moved(&mut far, one(17));
        assert_eq!(queued_moves(&far).len(), 2);

        let mut many = Movers::default();
        for i in 0..17 {
            let x = 3 + i * 7;
            queue_moved(
                &mut many,
                CellRect {
                    x0: x,
                    z0: 3,
                    x1: x,
                    z1: 3,
                },
            );
        }
        let queued = queued_moves(&many);
        assert_eq!(queued.len(), 16);
        assert_eq!(queued[0].x0, 10);
        assert_eq!(queued[15].x0, 3 + 16 * 7);

        let mut cell = grid.cell(14, 12, Layer::Lower);
        cell.floor = 20;
        cell.underside = 15;
        grid.set_cell(14, 12, Layer::Lower, cell);

        let mut movers = Movers::default();
        queue_moved(
            &mut movers,
            CellRect {
                x0: 12,
                z0: 12,
                x1: 12,
                z1: 12,
            },
        );
        let samples = [
            at(&grid, 12, 12, 2.0),
            at(&grid, 13, 12, 3.0),
            at(&grid, 14, 12, 1.2),
            at(&grid, 15, 12, 25.5),
            at(&grid, 11, 12, 0.0),
            at(&grid, 16, 12, 2.0),
        ];
        refresh_movers(&mut grid, &mut movers, &samples);
        assert!(queued_moves(&movers).is_empty());
        let raised = grid.cell(12, 12, Layer::Lower);
        assert_eq!(raised.floor, 20);
        assert!(raised.roughness > 0, "the cost pass did not run");
        assert_eq!(grid.cell(13, 12, Layer::Lower).floor, 10);
        assert_eq!(grid.cell(13, 12, Layer::Upper).floor, 30);
        assert_eq!(grid.cell(14, 12, Layer::Lower).floor, 20);
        assert_eq!(grid.cell(14, 12, Layer::Upper).floor, 12);
        assert_eq!(grid.cell(15, 12, Layer::Upper).floor, 0);
        assert_eq!(grid.cell(11, 12, Layer::Lower).floor, 10);
        assert_eq!(grid.cell(16, 12, Layer::Lower).floor, 10);
        assert_eq!(grid.cell(16, 12, Layer::Lower).roughness, 0);
    }
}
