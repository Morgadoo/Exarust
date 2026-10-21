//! A line stamped on the navigation penalty, as a barricade stamps its wall.
//!
//! The caller gives a patch (its middle cell and its size) and the points
//! along the line, in centimetres. Each point's cell, on the layer whose
//! height band holds the point, weighs 255; the eight cells around each
//! line cell, on the layer that holds the first point's height, weigh 128
//! (255 when reached twice); the four cells beside each of those that the
//! first two passes left untouched gain 64 each time they are reached (at
//! most 255). Every weighed cell then adds a quarter of its weight to its
//! layer's penalty byte, wrapping; taking the stamp back subtracts the same.
//!
//! A cell's height band is the original's: the height in 10 cm steps above
//! the grid's base must be more than three under the cell's underside byte
//! and less than its floor byte plus 25. Cells outside the grid are not
//! weighed. Evidence: the navigation grid's line stamp and its removal in
//! `Exanima.exe`, checked against their instructions.

use bevy::math::Vec3;

use super::{Layer, NavGrid};

/// The eight cells around a cell, in the original's order; its first four
/// are the cells beside it.
const OFFSETS: [(i32, i32); 8] = [
    (0, -1),
    (-1, 0),
    (1, 0),
    (0, 1),
    (-1, -1),
    (1, -1),
    (-1, 1),
    (1, 1),
];

/// The cells per centimetre the grid's 30 cm cells give.
const SCALE: f32 = 1.0 / 30.0;

/// A stamped line: its patch's place and size and each cell's layer and
/// weight, row by row. Removing it ([`unstamp_line`]) needs exactly this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavPatch {
    /// The grid cell of the patch's origin (its first row's first cell).
    pub origin: (i32, i32),
    pub size: (u32, u32),
    /// Per patch cell: the layer weighed (`None` for none) and its weight.
    pub cells: Vec<(Option<Layer>, u8)>,
}

/// The processor's conversion to an integer with the default rounding (to
/// nearest, ties to even), 64 bits wide and kept to its low 32: a NaN or an
/// out-of-range value gives 0.
fn convert(v: f64) -> i32 {
    let r = v.round_ties_even();
    if r.is_nan() || !(-9.223_372_036_854_775_808e18..9.223_372_036_854_775_808e18).contains(&r) {
        0
    } else {
        (r as i64) as i32
    }
}

/// The layer whose height band holds `height` at a grid cell, the lower
/// first.
fn band(grid: &NavGrid, x: i32, z: i32, height: i32) -> Option<Layer> {
    let (width, depth) = grid.size();
    if x < 0 || z < 0 || x as usize >= width || z as usize >= depth {
        return None;
    }
    Layer::BOTH.into_iter().find(|&layer| {
        let cell = grid.cell(x as usize, z as usize, layer);
        i64::from(cell.underside) - 3 < i64::from(height)
            && i64::from(height) < i64::from(cell.floor) + 25
    })
}

/// A point's cell: `width·½ + (x·scale − ½)` in single precision, converted.
fn cell_of(grid: &NavGrid, p: Vec3) -> (i32, i32) {
    let (width, depth) = grid.size();
    let x = width as i32 as f32 * 0.5 + (p.x * SCALE - 0.5);
    let z = depth as i32 as f32 * 0.5 + (p.z * SCALE - 0.5);
    (convert(f64::from(x)), convert(f64::from(z)))
}

/// A point's height in 10 cm steps above the grid's base (its difference in
/// single precision, the step in double).
fn height_of(grid: &NavGrid, p: Vec3) -> i32 {
    convert(f64::from(p.y - grid.base() * 100.0) * 0.1)
}

/// Stamps the line through `points` (cm) on the grid's penalty, in a patch
/// of `size` cells around `centre`, and returns the patch to take it back
/// with.
pub fn stamp_line(
    grid: &mut NavGrid,
    centre: (i32, i32),
    size: (u32, u32),
    points: &[Vec3],
) -> NavPatch {
    let (w, h) = size;
    let origin = (centre.0 - (w >> 1) as i32, centre.1 - (h >> 1) as i32);
    let n = (w as usize).saturating_mul(h as usize);
    let mut cells: Vec<(Option<Layer>, u8)> = vec![(None, 0); n];
    let mut marks = vec![0u8; n];
    let index = |lx: i32, lz: i32| -> Option<usize> {
        let i = (w as i32).wrapping_mul(lz).wrapping_add(lx);
        (0 <= lx && lx < w as i32 && 0 <= lz && lz < h as i32).then_some(i as usize)
    };
    // The line cells.
    let mut queue: Vec<(i32, i32)> = Vec::new();
    for &p in points {
        let (x, z) = cell_of(grid, p);
        let Some(layer) = band(grid, x, z, height_of(grid, p)) else {
            continue;
        };
        let (lx, lz) = (x - origin.0, z - origin.1);
        if let Some(i) = index(lx, lz) {
            cells[i] = (Some(layer), 0xff);
            marks[i] = 2;
        }
        queue.push((lx, lz));
    }
    let line = queue.len();
    if line > 0 {
        let first = points.first().map_or(0, |&p| height_of(grid, p));
        // The ring around the line, at the first point's height.
        for q in 0..line {
            let (qx, qz) = queue[q];
            for (dx, dz) in OFFSETS {
                let (lx, lz) = (qx + dx, qz + dz);
                let Some(i) = index(lx, lz) else { continue };
                if marks[i] > 1 {
                    continue;
                }
                let Some(layer) = band(grid, lx + origin.0, lz + origin.1, first) else {
                    continue;
                };
                marks[i] = 1;
                let weight = if cells[i].1 <= 0x7f { 0x80 } else { 0xff };
                cells[i] = (Some(layer), weight);
                queue.push((lx, lz));
            }
        }
        // The cells beside the ring that neither pass marked.
        for q in line..queue.len() {
            let (qx, qz) = queue[q];
            for &(dx, dz) in &OFFSETS[..4] {
                let (lx, lz) = (qx + dx, qz + dz);
                let Some(i) = index(lx, lz) else { continue };
                if marks[i] != 0 {
                    continue;
                }
                let Some(layer) = band(grid, lx + origin.0, lz + origin.1, first) else {
                    continue;
                };
                cells[i] = (Some(layer), cells[i].1.saturating_add(0x40));
            }
        }
    }
    let patch = NavPatch {
        origin,
        size,
        cells,
    };
    if line > 0 {
        apply(grid, &patch, true);
    }
    patch
}

/// Takes a stamped line back off the grid's penalty.
pub fn unstamp_line(grid: &mut NavGrid, patch: &NavPatch) {
    apply(grid, patch, false);
}

fn apply(grid: &mut NavGrid, patch: &NavPatch, add: bool) {
    let (w, _) = patch.size;
    for (i, &(layer, weight)) in patch.cells.iter().enumerate() {
        let Some(layer) = layer else { continue };
        let x = patch.origin.0 + (i % w as usize) as i32;
        let z = patch.origin.1 + (i / w as usize) as i32;
        if x < 0 || z < 0 {
            continue;
        }
        grid.shift_penalty(x as usize, z as usize, layer, weight >> 2, add);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::NavCell;

    /// A 20 × 20 grid with a floor at byte 10 on the lower layer and an
    /// upper layer at byte 100 over its right half.
    fn yard() -> NavGrid {
        let mut grid = NavGrid::empty(20, 20, 0.0);
        for z in 0..20 {
            for x in 0..20 {
                let floor = NavCell {
                    floor: 10,
                    underside: 0,
                    ..NavCell::default()
                };
                grid.set_cell(x, z, Layer::Lower, floor);
                if x >= 10 {
                    let upper = NavCell {
                        floor: 100,
                        underside: 90,
                        ..NavCell::default()
                    };
                    grid.set_cell(x, z, Layer::Upper, upper);
                }
            }
        }
        grid
    }

    /// The centre (cm) of grid cell (x, z) of a 20-wide grid, at `y` cm.
    fn at(x: i32, z: i32, y: f32) -> Vec3 {
        Vec3::new(
            (x as f32 - 10.0 + 0.5) * 30.0,
            y,
            (z as f32 - 10.0 + 0.5) * 30.0,
        )
    }

    fn penalty(grid: &NavGrid, x: usize, z: usize) -> u8 {
        grid.cell(x, z, Layer::Lower).penalty
    }

    #[test]
    fn a_line_weighs_its_cells_its_ring_and_beyond() {
        let mut grid = yard();
        let points = [at(5, 5, 50.0), at(6, 5, 50.0)];
        let patch = stamp_line(&mut grid, (6, 5), (8, 7), &points);
        assert_eq!(patch.origin, (2, 2));
        assert_eq!((penalty(&grid, 5, 5), penalty(&grid, 6, 5)), (63, 63));
        // (5, 4) is beside both line cells: reached twice, it weighs 255.
        assert_eq!(penalty(&grid, 5, 4), 63);
        // (4, 4) is a corner of the first cell only.
        assert_eq!(penalty(&grid, 4, 4), 32);
        // Beside the ring: (3, 5) is reached once, from (4, 5); (5, 3) twice,
        // as (5, 4) joins the ring once for each line cell.
        assert_eq!(penalty(&grid, 3, 5), 0x40 >> 2);
        assert_eq!(penalty(&grid, 5, 3), 0x80 >> 2);
        assert_eq!(penalty(&grid, 9, 9), 0);
        assert_eq!(grid.cell(5, 5, Layer::Upper).penalty, 0);
        unstamp_line(&mut grid, &patch);
        for z in 0..20 {
            for x in 0..20 {
                assert_eq!(penalty(&grid, x, z), 0);
            }
        }
    }

    #[test]
    fn a_line_finds_the_layer_by_height() {
        let mut grid = yard();
        // 960 cm is 96 steps: under the upper floor plus 25, over its
        // underside less 3, and past the lower floor's band.
        let patch = stamp_line(&mut grid, (14, 10), (7, 7), &[at(14, 10, 960.0)]);
        assert_eq!(grid.cell(14, 10, Layer::Upper).penalty, 63);
        assert_eq!(grid.cell(14, 10, Layer::Lower).penalty, 0);
        assert!(patch.cells.iter().any(|&(l, _)| l == Some(Layer::Upper)));
        // A point in no band weighs nothing.
        let none = stamp_line(&mut grid, (5, 5), (7, 7), &[at(5, 5, 5000.0)]);
        assert!(none.cells.iter().all(|&(l, _)| l.is_none()));
    }

    #[test]
    fn the_penalty_wraps_as_a_byte() {
        let mut grid = yard();
        let mut cell = grid.cell(5, 5, Layer::Lower);
        cell.penalty = 250;
        grid.set_cell(5, 5, Layer::Lower, cell);
        let patch = stamp_line(&mut grid, (5, 5), (7, 7), &[at(5, 5, 50.0)]);
        assert_eq!(
            grid.cell(5, 5, Layer::Lower).penalty,
            250u8.wrapping_add(63)
        );
        unstamp_line(&mut grid, &patch);
        assert_eq!(grid.cell(5, 5, Layer::Lower).penalty, 250);
        assert_eq!(convert(f64::NAN), 0);
        assert_eq!(convert(2.5), 2);
    }
}
