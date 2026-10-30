//! The pathfinder's point queries: the cost of the cell under a point and
//! the nearest walkable point, as the original answers them.
//!
//! - **Cost** ([`cost_at`]): the clearance and penalty bytes over 255 on
//!   the layer whose slab the point's height meets (the upper one when the
//!   height lies above its underside less 3 voxels and below its floor
//!   plus 15).
//! - **Nearest walkable point** ([`nearest_walkable`]): the point itself,
//!   on its floor, when its cell's clearance plus penalty is under 80;
//!   else the cell of least cost a best-first flood by the planner's step
//!   rules reaches (at most 220 cells, stopping at one under 80), from the
//!   point's cell or, off the floors, from the nearest cell by the squared
//!   3-D distance to the point that has room for a walker.
//!
//! Points in and out are Exarust's metres; the rules are the original's
//! centimetres and 10 cm voxels. Evidence: `docs/NAVIGATION.md` (Queries).

use std::collections::HashSet;

use bevy::math::Vec3;

use super::{CELL_CM, Layer, NavCell, NavGrid, SKY, VOXEL_CM, centimetres_of};

/// A cell whose clearance plus penalty is under this is walkable.
pub const WALKABLE_COST: u32 = 80;
/// The flood meets at most this many cells.
pub const FLOOD_CELLS: usize = 220;
/// The flood steps up less than 50 cm and down less than 2.1 m (voxels),
/// from a floor within the line walk's reach: 40 cm up, or under a slab
/// with at least 100 cm beneath it.
const FLOOD_RISE: i32 = 5;
const FLOOD_DROP: i32 = -21;
const STEP_RISE: i32 = 4;
const STEP_ROOM: i32 = 10;
/// Off the floors, a cell counts only with more than this many voxels
/// between its floor and the other layer's slab (or the other floor not
/// above it).
const FLOOR_ROOM: i32 = 10;
/// A point's layer by its height: the upper one when the height lies above
/// its slab's underside less 3 voxels and below its floor plus 15.
const UPPER_BELOW: i32 = 3;
const UPPER_ABOVE: i32 = 15;
/// Picking a cell's layer: the one whose floor is nearer, unless fewer
/// than this many voxels lie between the lower floor and the upper slab
/// (the upper one) or under the lower slab over the upper floor (the
/// lower one).
const LAYER_GAP: i32 = 11;
/// Cost bytes are read as shares of this.
const BYTE: f64 = 1.0 / 255.0;
/// The flood's window: 256 cells, its start at cell 127 of it.
const WINDOW: i32 = 256;
const WINDOW_CENTRE: u8 = 127;

/// The eight neighbours (dx, dz), in the original's order.
const NEIGHBOURS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// A point (m) in the original's centimetres.
fn cm(p: Vec3) -> [f32; 3] {
    [
        centimetres_of(f64::from(p.x)) as f32,
        centimetres_of(f64::from(p.y)) as f32,
        centimetres_of(f64::from(p.z)) as f32,
    ]
}

/// The grid's base in centimetres.
fn base_cm(grid: &NavGrid) -> f32 {
    centimetres_of(f64::from(grid.base())) as f32
}

/// A height (cm) in voxels above the grid's base, rounded as the original
/// rounds it (the difference in single precision, the scale in double).
fn voxels(grid: &NavGrid, y: f32) -> i32 {
    (f64::from(y - base_cm(grid)) * (1.0 / VOXEL_CM)).round_ties_even() as i32
}

/// What cell (`x`, `z`) holds on `layer`; nothing outside the grid.
fn at(grid: &NavGrid, x: i32, z: i32, layer: Layer) -> NavCell {
    if x < 0 || z < 0 {
        return NavCell::default();
    }
    grid.cell(x as usize, z as usize, layer)
}

/// The layer whose slab a height `h` (voxels) meets at cell (`x`, `z`).
fn layer_by_height(grid: &NavGrid, x: i32, z: i32, h: i32) -> Layer {
    let upper = at(grid, x, z, Layer::Upper);
    if i32::from(upper.underside) - UPPER_BELOW < h && h < i32::from(upper.floor) + UPPER_ABOVE {
        Layer::Upper
    } else {
        Layer::Lower
    }
}

/// The layer a walker at height `h` (voxels) stands on at cell (`x`, `z`),
/// as the original picks it: none without a lower floor (or with only
/// open sky); the lower one without an upper floor; the upper one when
/// fewer than 11 voxels lie between the lower floor and the upper slab,
/// the lower one when fewer than 11 lie under the lower slab over the
/// upper floor; else the one whose floor is nearer `h`.
pub fn layer_at(grid: &NavGrid, x: i32, z: i32, h: i32) -> Option<Layer> {
    let lower = at(grid, x, z, Layer::Lower);
    if lower.floor == 0 {
        return None;
    }
    let upper = at(grid, x, z, Layer::Upper);
    if upper.floor == 0 || upper.floor == SKY {
        return (lower.floor != SKY).then_some(Layer::Lower);
    }
    let (fl, fu) = (i32::from(lower.floor), i32::from(upper.floor));
    if fl < fu {
        if i32::from(upper.underside) - fl < LAYER_GAP {
            return Some(Layer::Upper);
        }
    } else if i32::from(lower.underside) - fu < LAYER_GAP {
        return Some(Layer::Lower);
    }
    Some(if (h - fu).abs() < (h - fl).abs() {
        Layer::Upper
    } else {
        Layer::Lower
    })
}

/// The cell (x, z) a point (cm) falls in, as the cost query maps it.
fn cell_of(grid: &NavGrid, p: [f32; 3]) -> (i32, i32) {
    let (w, d) = grid.size();
    let inverse = 1.0 / CELL_CM as f32;
    (
        (p[0] * inverse - 0.5).round_ties_even() as i32 + (w >> 1) as i32,
        (p[2] * inverse - 0.5).round_ties_even() as i32 + (d >> 1) as i32,
    )
}

/// The cost of the cell under `point` (m): its clearance and its penalty
/// over 255, on the layer the point's height meets.
pub fn cost_at(grid: &NavGrid, point: Vec3) -> (f32, f32) {
    let p = cm(point);
    let (x, z) = cell_of(grid, p);
    let c = at(grid, x, z, layer_by_height(grid, x, z, voxels(grid, p[1])));
    let share = |b: u8| (f64::from(b) * BYTE) as f32;
    (share(c.clearance), share(c.penalty))
}

/// The nearest walkable point to `point` (m): the point on its floor when
/// its cell is walkable, else the cheapest cell the original's flood meets
/// (see the module).
pub fn nearest_walkable(grid: &NavGrid, point: Vec3) -> Vec3 {
    let p = Flood::new(grid, cm(point)).run();
    Vec3::new(p[0], p[1], p[2]) / 100.0
}

/// One cell the flood has met.
#[derive(Debug, Clone, Copy)]
struct Met {
    /// Its place in the flood's window (cells; the start at 127, 127).
    at: (u8, u8),
    layer: Layer,
    /// What orders the flood: the distance from the start.
    key: i64,
}

/// The original's nearest-walkable search, in its window of 256 × 256
/// cells around the point's cell.
struct Flood<'a> {
    grid: &'a NavGrid,
    point: [f32; 3],
    /// The window's first cell in the grid, and the point's cell in the
    /// original's rounding (x, z).
    corner: (i32, i32),
    origin: (i32, i32),
    /// The point's height (voxels).
    height: i32,
    /// The cells met (index 0 unused), which of them wait to be expanded
    /// (in the order they were met), and which window cells are met.
    met: Vec<Met>,
    waiting: Vec<usize>,
    seen: HashSet<(u8, u8, Layer)>,
}

impl<'a> Flood<'a> {
    fn new(grid: &'a NavGrid, point: [f32; 3]) -> Self {
        let (w, d) = grid.size();
        let inverse = 1.0 / CELL_CM as f32;
        let origin = (
            (point[0] * inverse + 0.5).round_ties_even() as i32,
            (point[2] * inverse + 0.5).round_ties_even() as i32,
        );
        let corner = (
            origin.0 + ((w as i32 - WINDOW) >> 1),
            origin.1 + ((d as i32 - WINDOW) >> 1),
        );
        Self {
            grid,
            point,
            corner,
            origin,
            height: voxels(grid, point[1]),
            met: Vec::new(),
            waiting: Vec::new(),
            seen: HashSet::new(),
        }
    }

    /// The grid cell of a window place.
    fn cell(&self, at: (u8, u8)) -> (i32, i32) {
        (
            self.corner.0 + i32::from(at.0),
            self.corner.1 + i32::from(at.1),
        )
    }

    fn holds(&self, at: (u8, u8), layer: Layer) -> NavCell {
        let (x, z) = self.cell(at);
        self::at(self.grid, x, z, layer)
    }

    /// A cell's cost: its clearance plus its penalty on `layer`.
    fn cost(&self, at: (u8, u8), layer: Layer) -> u32 {
        let c = self.holds(at, layer);
        u32::from(c.clearance) + u32::from(c.penalty)
    }

    fn floor(&self, at: (u8, u8), layer: Layer) -> i32 {
        i32::from(self.holds(at, layer).floor)
    }

    /// Where the search lands: a cell's centre on its floor (cm).
    fn landing(&self, m: Met) -> [f32; 3] {
        let cell = CELL_CM as f32;
        let centre = |a: u8, o: i32| ((i32::from(a) + o) as f32 - 127.5) * cell;
        [
            centre(m.at.0, self.origin.0),
            self.floor(m.at, m.layer) as f32 * VOXEL_CM as f32 + base_cm(self.grid),
            centre(m.at.1, self.origin.1),
        ]
    }

    /// Starts over from `first`: nothing met but it.
    fn restart(&mut self, first: Met) {
        self.met = vec![first, first];
        self.waiting.clear();
        self.seen.clear();
        self.seen.insert((first.at.0, first.at.1, first.layer));
    }

    /// Meets a new cell, unless the flood has met its most; returns its
    /// index (the cells met count from 1).
    fn meet(&mut self, m: Met) -> Option<usize> {
        if self.met.len() > FLOOD_CELLS {
            return None;
        }
        self.seen.insert((m.at.0, m.at.1, m.layer));
        self.met.push(m);
        Some(self.met.len() - 1)
    }

    /// Takes the waiting cell nearest the start (of equals, the one met
    /// last).
    fn take_nearest(&mut self) -> Option<Met> {
        let (slot, _) = self
            .waiting
            .iter()
            .enumerate()
            .rev()
            .min_by_key(|&(_, &i)| self.met[i].key)?;
        let i = self.waiting.remove(slot);
        Some(self.met[i])
    }

    fn window_offset(at: (u8, u8), dx: i32, dz: i32) -> (u8, u8) {
        (
            (i32::from(at.0) + dx).rem_euclid(WINDOW) as u8,
            (i32::from(at.1) + dz).rem_euclid(WINDOW) as u8,
        )
    }

    /// The window place's offset from the start, squared across the ground.
    fn spread(at: (u8, u8)) -> i64 {
        let ax = i64::from(at.0) - i64::from(WINDOW_CENTRE);
        let az = i64::from(at.1) - i64::from(WINDOW_CENTRE);
        ax * ax + az * az
    }

    fn run(mut self) -> [f32; 3] {
        let start_at = (WINDOW_CENTRE, WINDOW_CENTRE);
        let (sx, sz) = self.cell(start_at);
        let layer = layer_at(self.grid, sx, sz, self.height);
        let mut best_cost = u32::MAX;
        if let Some(layer) = layer {
            best_cost = self.cost(start_at, layer);
            if best_cost < WALKABLE_COST {
                let floor = self.floor(start_at, layer);
                return [
                    self.point[0],
                    floor as f32 * VOXEL_CM as f32 + base_cm(self.grid),
                    self.point[2],
                ];
            }
        }
        let first = Met {
            at: start_at,
            layer: layer.unwrap_or(Layer::Lower),
            key: 0,
        };
        self.restart(first);
        let off_floor = layer.is_none_or(|l| at(self.grid, sx, sz, l).clearance == u8::MAX);
        if off_floor {
            match self.nearest_floor(first, best_cost) {
                Some(found) => {
                    let found = Met { key: 0, ..found };
                    best_cost = self.cost(found.at, found.layer);
                    self.restart(found);
                }
                None => return self.point,
            }
        }
        let best = self.flood(best_cost);
        self.landing(best)
    }

    /// From a point off the floors: the cell with a floor nearest across
    /// the ground (of at most [`FLOOD_CELLS`] met) that is not open sky and
    /// has room under the other layer's slab. The original adds to each
    /// distance the square of a value it never sets in this search (a
    /// register left over from its caller), the same for every cell;
    /// Exarust leaves it out. As the original, it counts a lower layer
    /// without a floor too.
    fn nearest_floor(&mut self, first: Met, start_cost: u32) -> Option<Met> {
        let mut bound = i64::from(start_cost.min(i32::MAX as u32));
        let mut best = None;
        let mut current = first;
        loop {
            for (dx, dz) in NEIGHBOURS {
                let at = Self::window_offset(current.at, dx, dz);
                let cells = [self.holds(at, Layer::Lower), self.holds(at, Layer::Upper)];
                for (k, layer) in Layer::BOTH.into_iter().enumerate() {
                    let (own, other) = (cells[k], cells[1 - k]);
                    if self.seen.contains(&(at.0, at.1, layer))
                        || (own.floor == 0 && layer == Layer::Upper)
                    {
                        continue;
                    }
                    let distance = Self::spread(at);
                    // The original orders these by the squared 3-D distance
                    // (the cell's floor less the point's height) but only
                    // accepts a cell whose ground distance is within the
                    // bound so far.
                    let up = i64::from(i32::from(own.floor) - self.height);
                    let key = distance + up * up;
                    let m = Met { at, layer, key };
                    let Some(i) = self.meet(m) else {
                        return best;
                    };
                    self.waiting.push(i);
                    let roomy = other.floor <= own.floor
                        || i32::from(other.underside) - i32::from(own.floor) > FLOOR_ROOM;
                    if own.floor != SKY && roomy && distance <= bound && key < bound {
                        bound = key;
                        best = Some(m);
                    }
                }
            }
            match self.take_nearest() {
                Some(next) => current = next,
                None => return best,
            }
        }
    }

    /// From the cell met first: best-first by the distance from the point
    /// (in 3-D, × 10), over steps a walker takes (up less than 50 cm, down
    /// less than 2.1 m), keeping the cell of least cost; stops once that is
    /// under 80, or with nothing left to expand, or after [`FLOOD_CELLS`]
    /// cells.
    fn flood(&mut self, start_cost: u32) -> Met {
        let mut best = self.met[1];
        let mut best_cost = start_cost;
        let mut current = self.met[1];
        loop {
            let own = self.floor(current.at, current.layer);
            for (dx, dz) in NEIGHBOURS {
                let at = Self::window_offset(current.at, dx, dz);
                let (x, z) = self.cell(at);
                let Some((layer, floor)) = self.step_onto(x, z, own) else {
                    continue;
                };
                let rise = floor - own;
                if rise >= FLOOD_RISE
                    || rise <= FLOOD_DROP
                    || self.seen.contains(&(at.0, at.1, layer))
                {
                    continue;
                }
                let up = i64::from(floor - self.height);
                let key = ((((Self::spread(at) + up * up) as f64).sqrt()) * 10.0).round_ties_even()
                    as i64;
                let m = Met { at, layer, key };
                let Some(i) = self.meet(m) else {
                    return best;
                };
                self.waiting.push(i);
                let cost = self.cost(at, layer);
                if cost < best_cost {
                    best = m;
                    best_cost = cost;
                }
            }
            if best_cost < WALKABLE_COST {
                return best;
            }
            match self.take_nearest() {
                Some(next) => current = next,
                None => return best,
            }
        }
    }

    /// The floor a walker on a floor at `own` steps onto at cell (`x`,
    /// `z`), by the line walk's reach: the lower floor within reach, or the
    /// upper one when it is within reach and higher; none where a slab
    /// blocks or no floor is within reach.
    fn step_onto(&self, x: i32, z: i32, own: i32) -> Option<(Layer, i32)> {
        let lower = at(self.grid, x, z, Layer::Lower);
        if lower.floor == 0 {
            return None;
        }
        let (fl, ul) = (i32::from(lower.floor), i32::from(lower.underside));
        let mut chosen = None;
        if own + STEP_RISE < fl {
            if ul < own + STEP_ROOM {
                return None;
            }
        } else {
            chosen = Some((Layer::Lower, fl));
        }
        let upper = at(self.grid, x, z, Layer::Upper);
        let (fu, uu) = (i32::from(upper.floor), i32::from(upper.underside));
        if fu == 0 {
            return chosen;
        }
        if own + STEP_RISE < fu {
            if uu < own + STEP_ROOM && (uu != 0 || fu < fl) {
                return None;
            }
            return chosen;
        }
        if fu <= fl {
            return chosen;
        }
        Some((Layer::Upper, fu))
    }
}

#[cfg(test)]
#[path = "walkable_test.rs"]
mod tests;
