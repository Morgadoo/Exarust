//! The planner's footprint and the mover's dirty rectangles.
//!
//! A plan lifts the standing footprint, searches, and stamps it again, so
//! the body does not pay for the cell it is standing on. A moved rectangle
//! joins an existing one when the union padded by six on each side is
//! strictly smaller than the two padded rectangles, else it is appended.
//! A ring of sixteen rectangles queues objects as they move and never
//! merges; a refresh drains it into the pending list and grows each side by
//! three cells. Each rectangle's working floors start from the floors as
//! built; the samples the caller supplies (a mover's vertices, voxel
//! `y·0.1 + ½ − base·0.1` rounded) raise them: a missing floor takes any
//! higher sample, one less than fifteen above takes it, and one fifteen or
//! more above, or at or under the lower slab's underside, raises the upper
//! floor when that is missing or the sample less than fifteen above it.
//! Where a working floor differs from the grid's, the rectangle's floors
//! are written and the cost passes run again. A scene-wide countdown holds
//! the merge (not the ring) and the refresh, and then hands the whole grid
//! less its twenty-cell rim in as one rectangle. Evidence: the movement
//! notes' planner and movers, and the refresh read instruction by
//! instruction.

use bevy::math::Vec3;

use super::query::axis;
use super::{Layer, NavGrid, Path, find_path, remove_footprint, stamp_footprint};

/// Ring slots.
const SLOTS: usize = 16;
const PAD: i32 = 3;
const BOX_PAD: f32 = 2.5;

/// A rectangle of cells, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRect {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
}

/// The moved objects' ring and the rectangles waiting for a refresh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movers {
    /// The pending list: merged rectangles, grown as needed.
    rects: Vec<CellRect>,
    ring: [CellRect; SLOTS],
    read: usize,
    write: usize,
    /// A scene-wide refresh counts down; while it runs, both queues hold.
    countdown: i32,
}

impl Default for Movers {
    fn default() -> Self {
        Self {
            rects: Vec::new(),
            ring: [CellRect {
                x0: 0,
                z0: 0,
                x1: 0,
                z1: 0,
            }; SLOTS],
            read: 0,
            write: 0,
            countdown: 0,
        }
    }
}

/// The cell rectangle around an axis-aligned box (centre and half extents,
/// metres), as the original marks a moved object: on each axis
/// `W/2 + round((c ∓ e) · (1/30) ∓ 2.5)` in centimetres (single precision,
/// ties to even), the low side at least 3 and the high side at most
/// `W − 4`.
pub fn rect_around(grid: &NavGrid, centre: Vec3, half: Vec3) -> CellRect {
    let (width, depth) = grid.size();
    let cm = |m: f32| super::centimetres_of(f64::from(m)) as f32;
    let inverse = 1.0f32 / super::CELL_CM as f32;
    let edge = |coord: f32, extent: f32, n: usize, sign: f32| {
        let at = (cm(coord) + sign * cm(extent)) * inverse + sign * BOX_PAD;
        (n / 2) as i32 + at.round_ties_even() as i32
    };
    let rect = CellRect {
        x0: edge(centre.x, half.x, width, -1.0).max(PAD),
        x1: edge(centre.x, half.x, width, 1.0).min(width as i32 - PAD - 1),
        z0: edge(centre.z, half.z, depth, -1.0).max(PAD),
        z1: edge(centre.z, half.z, depth, 1.0).min(depth as i32 - PAD - 1),
    };
    rect.normalized()
}

/// Merge `rect` into the first pending rectangle whose padded union is
/// strictly smaller than the two padded areas. Otherwise append it. The
/// scene-wide countdown holds the merge while it runs.
pub fn queue_moved(movers: &mut Movers, rect: CellRect) {
    if movers.countdown >= 1 {
        return;
    }
    let rect = rect.normalized();
    for existing in &mut movers.rects {
        let joined = existing.union(rect);
        if padded_area(joined) < padded_area(*existing) + padded_area(rect) {
            *existing = joined;
            return;
        }
    }
    movers.rects.push(rect);
}

/// The pending rectangles.
pub fn queued_moves(movers: &Movers) -> &[CellRect] {
    &movers.rects
}

/// Queue one moved object in the sixteen-slot ring, clamping it to the
/// grid's three-cell border. The oldest slot is overwritten; nothing merges.
/// Unlike the merge, the ring takes rectangles during the scene-wide
/// countdown, and the refresh drains them into the whole-grid pass.
pub fn ring_moved(movers: &mut Movers, grid: &NavGrid, rect: CellRect) {
    let (width, depth) = grid.size();
    let rect = rect.normalized();
    let rect = CellRect {
        x0: rect.x0.max(PAD),
        z0: rect.z0.max(PAD),
        x1: rect.x1.min(width as i32 - PAD - 1),
        z1: rect.z1.min(depth as i32 - PAD - 1),
    };
    movers.ring[movers.write] = rect;
    movers.write = (movers.write + 1) % SLOTS;
}

/// The rectangles in the ring, oldest first.
pub fn ring_contents(movers: &Movers) -> Vec<CellRect> {
    let mut out = Vec::new();
    let mut at = movers.read;
    while at != movers.write {
        out.push(movers.ring[at]);
        at = (at + 1) % SLOTS;
    }
    out
}

/// Starts a scene-wide refresh: the next `ticks` calls hold the queues,
/// then the whole grid (less its twenty-cell rim) is refreshed.
pub fn start_full_refresh(movers: &mut Movers, ticks: i32) {
    movers.countdown = ticks.max(1);
}

/// A footprint rectangle on the planner grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FootprintRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl FootprintRect {
    fn union(self, other: Self) -> Self {
        Self {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }

    /// Six cells of padding, as the original measures it.
    fn padded_area(self) -> i32 {
        (self.x1 - self.x0 + 6) * (self.y1 - self.y0 + 6)
    }
}

/// The planner's footprint rectangles and the spare capacity it keeps.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FootprintList {
    pub rects: Vec<FootprintRect>,
    pub capacity: i32,
}

/// What planning a footprint did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FootprintAdd {
    Blocked,
    Merged(usize),
    Appended(usize),
}

/// Merge `rect` into the first footprint whose padded union is smaller than
/// the two padded areas, or append it and grow the capacity by eight when it
/// is full. A busy planner leaves the list alone.
pub fn plan_footprint(busy: bool, list: &mut FootprintList, rect: FootprintRect) -> FootprintAdd {
    if busy {
        return FootprintAdd::Blocked;
    }
    let area = rect.padded_area();
    for (index, existing) in list.rects.iter_mut().enumerate() {
        let current = *existing;
        let joined = current.union(rect);
        if area + current.padded_area() > joined.padded_area() {
            *existing = joined;
            return FootprintAdd::Merged(index);
        }
    }
    if list.rects.len() as i32 >= list.capacity {
        list.capacity += 8;
    }
    let index = list.rects.len();
    list.rects.push(rect);
    FootprintAdd::Appended(index)
}

/// Lift the footprint at `standing`, search to `goal`, and stamp it again.
pub fn plan_around(grid: &mut NavGrid, standing: Vec3, goal: Vec3, size: u8) -> Path {
    remove_footprint(grid, standing);
    let path = find_path(grid, standing, goal, size);
    stamp_footprint(grid, standing);
    path
}

/// Grow each queued rectangle by three cells, raise floors from `samples`,
/// and rerun the cost passes where a floor changed. The ring is drained
/// into the pending list first. The queues are then empty, except in the
/// frames of the scene-wide countdown, which returns early.
pub fn refresh_movers(grid: &mut NavGrid, movers: &mut Movers, samples: &[Vec3]) {
    let (width, depth) = grid.size();
    if movers.countdown > 0 {
        movers.countdown -= 1;
        if movers.countdown != 0 {
            return;
        }
        movers.rects = vec![CellRect {
            x0: 20,
            z0: 20,
            x1: width as i32 - 21,
            z1: depth as i32 - 21,
        }];
    }
    while movers.read != movers.write {
        movers.rects.push(movers.ring[movers.read]);
        movers.read = (movers.read + 1) % SLOTS;
    }
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
        // The working floors start from the floors as built, so a mover
        // that has gone leaves its cells as they were.
        for z in z0..=z1 {
            for x in x0..=x1 {
                let i = at(x, z);
                lower[i] = grid.built_floor(x as usize, z as usize, Layer::Lower);
                upper[i] = grid.built_floor(x as usize, z as usize, Layer::Upper);
            }
        }
        for sample in samples {
            let sx = axis(sample.x, width).round_ties_even() as i32;
            let sz = axis(sample.z, depth).round_ties_even() as i32;
            let v = sample_voxel(sample.y, grid.base());
            if !(1..255).contains(&v) || sx < x0 || sz < z0 || sx > x1 || sz > z1 {
                continue;
            }
            let i = at(sx, sz);
            let underside = grid.cell(sx as usize, sz as usize, Layer::Lower).underside;
            raise_static(&mut lower[i], &mut upper[i], underside, v as u8);
        }
        let mut changed = false;
        for z in z0..=z1 {
            for x in x0..=x1 {
                let i = at(x, z);
                changed |= grid.set_floor(x as usize, z as usize, Layer::Lower, lower[i]);
                changed |= grid.set_floor(x as usize, z as usize, Layer::Upper, upper[i]);
            }
        }
        if changed {
            grid.derive_costs(x0 as usize, z0 as usize, x1 as usize, z1 as usize);
        }
    }
}

/// Raise the grid's floors from the moving props' sample points (world,
/// metres) as the original's build does once the layers are written and
/// before the cost passes: each point that falls in cells 1 to `W − 3`
/// (and `D − 3`) and voxels 0–254 raises the current floor of its cell by
/// the refresh's rule ([`raise_static`]), one point after another, against
/// the lower slab's underside. The floors as built stay as they were, so a
/// prop that moves away later leaves its cells to the built floors.
pub fn raise_from_movers(grid: &mut NavGrid, samples: impl IntoIterator<Item = Vec3>) {
    let (width, depth) = grid.size();
    for sample in samples {
        let sx = axis(sample.x, width).round_ties_even() as i32;
        let sz = axis(sample.z, depth).round_ties_even() as i32;
        let v = sample_voxel(sample.y, grid.base());
        if sx < 1
            || sz < 1
            || sx >= width as i32 - 2
            || sz >= depth as i32 - 2
            || !(0..255).contains(&v)
        {
            continue;
        }
        let (x, z) = (sx as usize, sz as usize);
        let mut lower = grid.cell(x, z, Layer::Lower).floor;
        let mut upper = grid.cell(x, z, Layer::Upper).floor;
        let underside = grid.cell(x, z, Layer::Lower).underside;
        raise_static(&mut lower, &mut upper, underside, v as u8);
        grid.set_floor(x, z, Layer::Lower, lower);
        grid.set_floor(x, z, Layer::Upper, upper);
    }
}

/// A sample's voxel as the refresh rounds a mover's vertex:
/// `y · 0.1 + ½ − base · 0.1` (centimetres, single precision), ties to even.
fn sample_voxel(y: f32, base: f32) -> i32 {
    let cm = |m: f32| super::centimetres_of(f64::from(m)) as f32;
    ((cm(y) * 0.1 + 0.5) - cm(base) * 0.1).round_ties_even() as i32
}

/// Raise a working floor from a sample voxel, as the refresh does.
///
/// A missing working floor takes any sample above it; otherwise a sample
/// less than fifteen above it takes it, and one fifteen or more above goes
/// on to the upper floor. A sample at or under the working floor goes on to
/// the upper floor when it is at or under the lower slab's underside. The
/// upper floor takes a higher sample when it is missing or the sample is
/// less than fifteen above it.
fn raise_static(lower: &mut u8, upper: &mut u8, underside: u8, sample: u8) {
    let stood = i32::from(*lower);
    let height = i32::from(sample);
    let mut upper_check = false;
    if stood < height {
        if stood == 0 {
            *lower = sample;
        } else if stood + 15 <= height {
            upper_check = true;
        } else {
            *lower = sample;
        }
    } else if height <= i32::from(underside) {
        upper_check = true;
    }
    if upper_check {
        let above = i32::from(*upper);
        if above < height && (above == 0 || height < above + 15) {
            *upper = sample;
        }
    }
}

fn padded_area(rect: CellRect) -> i64 {
    let width = i64::from(rect.x1 - rect.x0);
    let depth = i64::from(rect.z1 - rect.z0);
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
    fn a_footprint_plan_merges_appends_and_grows() {
        let mut list = FootprintList::default();
        let a = FootprintRect {
            x0: 0,
            y0: 0,
            x1: 9,
            y1: 9,
        };
        assert_eq!(
            plan_footprint(false, &mut list, a),
            FootprintAdd::Appended(0)
        );
        assert_eq!(list.capacity, 8);

        let b = FootprintRect {
            x0: 5,
            y0: 5,
            x1: 14,
            y1: 14,
        };
        assert_eq!(plan_footprint(false, &mut list, b), FootprintAdd::Merged(0));
        assert_eq!(
            list.rects,
            vec![FootprintRect {
                x0: 0,
                y0: 0,
                x1: 14,
                y1: 14,
            }]
        );

        let c = FootprintRect {
            x0: 100,
            y0: 100,
            x1: 109,
            y1: 109,
        };
        assert_eq!(
            plan_footprint(false, &mut list, c),
            FootprintAdd::Appended(1)
        );
        assert_eq!(list.capacity, 8);
        assert_eq!(plan_footprint(true, &mut list, c), FootprintAdd::Blocked);
        assert_eq!(list.rects.len(), 2);

        let mut full = FootprintList {
            rects: vec![a],
            capacity: 1,
        };
        assert_eq!(
            plan_footprint(false, &mut full, c),
            FootprintAdd::Appended(1)
        );
        assert_eq!(full.capacity, 9);
    }

    #[test]
    fn a_plan_lifts_its_footprint_and_stamps_it_again() {
        // The planner keeps ten cells off the grid's half size, and a start
        // under voxel 15 would take the empty upper layer: a 48-cell grid at
        // floor 100.
        let mut grid = NavGrid::empty(48, 48, 0.0);
        for z in 0..48 {
            for x in 0..48 {
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
        let standing = at(&grid, 20, 20, 10.0);
        let goal = at(&grid, 24, 20, 10.0);
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
        assert_eq!(grid.cell(20, 20, Layer::Lower).penalty, 63);
        assert_eq!(grid.cell(20, 19, Layer::Lower).penalty, 32);
    }

    #[test]
    fn a_dirty_rectangle_raises_the_floor_it_covers() {
        let mut grid = yard();
        let around = rect_around(&grid, at(&grid, 12, 12, 1.0), Vec3::ZERO);
        assert!(around.x0 <= 12 && around.x1 >= 12);
        assert!(around.z0 <= 12 && around.z1 >= 12);
        assert!(around.x1 > around.x0, "the pad did not reach a neighbour");
        assert!(around.x0 >= 3 && around.x1 <= 20);
        // Cell 12's centre is 15 cm: 12 + round(0.5 − 2.5) and
        // 12 + round(0.5 + 2.5).
        assert_eq!(
            (around.x0, around.x1, around.z0, around.z1),
            (10, 15, 10, 15)
        );
        let wide = rect_around(&grid, Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.6, 0.0, 9.0));
        assert_eq!((wide.x0, wide.x1, wide.z0, wide.z1), (8, 16, 3, 20));

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

        // The ring holds sixteen slots and overwrites the oldest; it never
        // merges. The pending merge keeps every rectangle.
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
        assert_eq!(queued_moves(&many).len(), 17);
        let mut ring = Movers::default();
        for i in 0..17 {
            let x = 3 + i;
            ring_moved(
                &mut ring,
                &grid,
                CellRect {
                    x0: x,
                    z0: 3,
                    x1: x,
                    z1: 3,
                },
            );
        }
        // The reader is not moved when the writer wraps: after seventeen
        // pushes the read index is 0 and the write index 1, so only the
        // slot just written is drained. The original relies on the ring
        // being emptied every frame, before it can wrap.
        let contents = ring_contents(&ring);
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].x0, 19);
        assert_eq!(ring_contents(&Movers::default()).len(), 0);

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

    /// A door leaf standing in a doorway: its points, one voxel apart and
    /// lowest first, each less than fifteen over the floor they find, walk
    /// the cell's floor up to the leaf's top, as the original's build
    /// raises it; the floor as built stays. A point fifteen over the floor
    /// it meets raises the upper floor, and one on the cells next to the
    /// border (from `W − 2`) raises nothing.
    #[test]
    fn a_door_leaf_raises_its_doorway_to_its_top() {
        let mut grid = yard();
        let leaf: Vec<Vec3> = (0..15)
            .map(|k| at(&grid, 10, 10, 1.05 + 0.1 * k as f32))
            .collect();
        raise_from_movers(&mut grid, leaf);
        assert_eq!(grid.cell(10, 10, Layer::Lower).floor, 25);
        assert_eq!(grid.built_floor(10, 10, Layer::Lower), 10);
        let points = [at(&grid, 5, 5, 2.45), at(&grid, 22, 5, 1.05)];
        raise_from_movers(&mut grid, points);
        assert_eq!(grid.cell(5, 5, Layer::Lower).floor, 10);
        assert_eq!(grid.cell(5, 5, Layer::Upper).floor, 25);
        assert_eq!(grid.cell(22, 5, Layer::Lower).floor, 10);
    }

    #[test]
    fn a_sample_raises_the_working_or_the_upper_floor() {
        let mut lower = 0u8;
        let mut upper = 0u8;
        raise_static(&mut lower, &mut upper, 0, 7);
        assert_eq!((lower, upper), (7, 0));

        let mut lower = 10u8;
        let mut upper = 0u8;
        raise_static(&mut lower, &mut upper, 10, 20);
        assert_eq!((lower, upper), (20, 0));

        let mut lower = 10u8;
        let mut upper = 0u8;
        raise_static(&mut lower, &mut upper, 10, 25);
        assert_eq!((lower, upper), (10, 25));

        let mut lower = 20u8;
        let mut upper = 0u8;
        raise_static(&mut lower, &mut upper, 20, 12);
        assert_eq!((lower, upper), (20, 12));

        let mut lower = 10u8;
        let mut upper = 20u8;
        raise_static(&mut lower, &mut upper, 10, 30);
        assert_eq!((lower, upper), (10, 30));

        let mut lower = 10u8;
        let mut upper = 20u8;
        raise_static(&mut lower, &mut upper, 10, 12);
        assert_eq!((lower, upper), (12, 20));

        let mut lower = 10u8;
        let mut upper = 5u8;
        raise_static(&mut lower, &mut upper, 10, 18);
        assert_eq!((lower, upper), (18, 5));

        let mut lower = 20u8;
        let mut upper = 0u8;
        raise_static(&mut lower, &mut upper, 10, 12);
        assert_eq!((lower, upper), (20, 0));
    }

    #[test]
    fn a_ring_rectangle_is_drained_by_the_refresh() {
        let mut grid = yard();
        let mut movers = Movers::default();
        ring_moved(
            &mut movers,
            &grid,
            CellRect {
                x0: 12,
                z0: 12,
                x1: 12,
                z1: 12,
            },
        );
        assert_eq!(ring_contents(&movers).len(), 1);
        assert!(queued_moves(&movers).is_empty());
        let samples = [at(&grid, 12, 12, 2.0)];
        refresh_movers(&mut grid, &mut movers, &samples);
        assert!(ring_contents(&movers).is_empty());
        assert!(queued_moves(&movers).is_empty());
        assert_eq!(grid.cell(12, 12, Layer::Lower).floor, 20);
    }

    #[test]
    fn a_scene_wide_refresh_waits_then_takes_the_whole_grid() {
        let mut grid = NavGrid::empty(60, 60, 0.0);
        for z in 0..60 {
            for x in 0..60 {
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
        let mut movers = Movers::default();
        start_full_refresh(&mut movers, 2);
        queue_moved(
            &mut movers,
            CellRect {
                x0: 12,
                z0: 12,
                x1: 12,
                z1: 12,
            },
        );
        ring_moved(
            &mut movers,
            &grid,
            CellRect {
                x0: 13,
                z0: 12,
                x1: 13,
                z1: 12,
            },
        );
        // The merge holds; the ring does not.
        assert!(queued_moves(&movers).is_empty());
        assert_eq!(ring_contents(&movers).len(), 1);

        refresh_movers(&mut grid, &mut movers, &[]);
        assert_eq!(movers.countdown, 1);
        assert!(queued_moves(&movers).is_empty());
        assert_eq!(ring_contents(&movers).len(), 1);

        refresh_movers(&mut grid, &mut movers, &[]);
        assert_eq!(movers.countdown, 0);
        assert!(queued_moves(&movers).is_empty());
        assert!(ring_contents(&movers).is_empty());
    }

    /// A refresh starts from the floors as built: once the mover is gone,
    /// the next refresh lowers the floor it raised.
    #[test]
    fn a_gone_mover_leaves_the_built_floor() {
        let mut grid = yard();
        let mut movers = Movers::default();
        let rect = CellRect {
            x0: 12,
            z0: 12,
            x1: 12,
            z1: 12,
        };
        queue_moved(&mut movers, rect);
        let sample = at(&grid, 12, 12, 2.0);
        refresh_movers(&mut grid, &mut movers, &[sample]);
        assert_eq!(grid.cell(12, 12, Layer::Lower).floor, 20);
        assert_eq!(grid.built_floor(12, 12, Layer::Lower), 10);
        queue_moved(&mut movers, rect);
        refresh_movers(&mut grid, &mut movers, &[]);
        assert_eq!(grid.cell(12, 12, Layer::Lower).floor, 10);
        assert_eq!(grid.cell(12, 12, Layer::Lower).roughness, 0);
    }

    /// The sample's voxel is `y·0.1 + ½` rounded ties to even: 2.0 m is the
    /// tie 20.5 (20), 2.04 m already reads 21 and 1.96 m 20.
    #[test]
    fn a_sample_rounds_half_a_voxel_up() {
        assert_eq!(sample_voxel(2.0, 0.0), 20);
        assert_eq!(sample_voxel(2.04, 0.0), 21);
        assert_eq!(sample_voxel(1.96, 0.0), 20);
        assert_eq!(sample_voxel(2.16, 0.1), 21);
    }
}
