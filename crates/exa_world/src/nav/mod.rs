//! The navigation grid of the original's pathfinder, built for every level
//! at load: 30 cm cells over the level's tile grid, each a column of 256
//! voxels of 10 cm, reduced to at most two walkable layers (an upper floor
//! over a lower one) with the cost bytes the planner reads.
//!
//! The build, in the original's order:
//! 1. **Columns.** Terrain fills each cell's column from the bottom to the
//!    ground's height at the cell centre, under the 6 m terrain blocks the
//!    level builds (in a tile-grid level only those its grid flags).
//!    Every static mesh's GI sample points (and the points generated for
//!    each tile-grid cell's floor, lifted half a voxel) mark one voxel
//!    each.
//! 2. **Spans.** Scanned from the top a byte (8 voxels) at a time, a solid
//!    slab with more than 12 free voxels (120 cm) above starts a span;
//!    slabs closer below join it. A span keeps its floor (the slab's top
//!    voxel; 255 above voxel 240) and its underside (under its lowest
//!    slab, as far down its byte as the scan goes). At most three spans;
//!    one whose underside is voxel 235 or higher is dropped unless it is
//!    the last.
//! 3. **Layers.** The top span is the upper layer and the next the lower;
//!    a single span is the lower layer.
//! 4. **Cost.** Per layer: roughness (the hardest step onto a neighbour,
//!    0–15 weighted 256 orthogonally and 181 diagonally, over 64), a blur
//!    with the neighbours (weights over 512), and a dilation into the
//!    clearance byte the planner tests (180/(size class + 1) passes).
//!
//! Evidence (reverse engineering of `Exanima.exe` 0.9.5, the corpus) and
//! the rules in full: `docs/NAVIGATION.md`. The planner is [`path`].

pub mod finder;
pub mod footprint;
pub mod headless;
pub mod mover;
pub mod path;
pub mod query;
pub mod source;
pub mod stamp;
pub mod view;
pub mod walkable;

use bevy::math::{DVec3, Vec2, Vec3};

pub use crate::terrain::ground::GroundTile;
pub use finder::{AbstractPathFinder, FALLBACK_SIZE, InstalledPathfinder};
pub use footprint::{NavFootprints, remove_footprint, stamp_footprint};
pub use mover::{
    CellRect, FootprintAdd, FootprintList, FootprintRect, Movers, plan_around, plan_footprint,
    queue_moved, queued_moves, raise_from_movers, rect_around, refresh_movers, ring_contents,
    ring_moved, start_full_refresh,
};
pub use path::{ACCEPT_GOAL_CELL, Look, Path, disturbance, find_path, find_path_with, look_ahead};
pub use query::{LineHit, line_walk, obstacle, straight_walk};
pub use source::NavSource;
pub use stamp::{NavPatch, stamp_line, unstamp_line};
pub use view::{
    MovingMesh, NavOverlay, NavPlugin, NavState, PendingNav, unload_level as unload_nav,
};
pub use walkable::{cost_at, layer_at, nearest_walkable};

/// Edge of a cell, metres (30 cm).
pub const CELL: f32 = 0.30;
/// Height of a voxel, metres (10 cm).
pub const VOXEL: f32 = 0.10;
/// Cells along a 6 m tile-grid room.
pub const CELLS_PER_ROOM: usize = 20;
/// Rooms across a level whose tile grid gives no size.
pub const DEFAULT_ROOMS: usize = 16;
/// Voxels in a column.
pub const COLUMN_VOXELS: usize = 256;
/// The columns start this far below the level's grid top (255 voxels).
pub const DEPTH: f32 = 25.5;
/// A slab starts a span when more than this many free voxels lie above it.
pub const HEADROOM_VOXELS: i32 = 12;
/// Spans kept per column.
pub const MAX_SPANS: usize = 3;
/// A floor above this voxel counts as open sky.
pub const SKY_FROM: u8 = 240;
/// The floor byte of open sky.
pub const SKY: u8 = 255;
/// A span (not the last of its column) whose underside is at or above this
/// voxel is dropped.
pub const UNDERSIDE_LIMIT: u8 = 235;
/// Neighbour weights of the cost passes.
pub const ORTHOGONAL: u32 = 256;
pub const DIAGONAL: u32 = 181;
/// Roughness of a layer with no floor, and of open sky.
pub const NO_FLOOR_ROUGHNESS: u8 = 120;
pub const SKY_ROUGHNESS: u8 = 60;
/// Clearance of a layer with no floor (or open sky).
pub const NO_FLOOR_CLEARANCE: u8 = 240;
/// The dilation reads a neighbour's blurred value capped at this.
pub const DILATION_CAP: u32 = 120;
/// The clearance a walker of size class 0 may enter; size class `s` takes
/// `PASSABLE / (s + 1)`.
pub const PASSABLE: u32 = 180;
/// Generated floor points are stamped this many voxels higher.
pub const FLOOR_LIFT: f64 = 0.5;

/// The original's units, centimetres.
const CELL_CM: f64 = 30.0;
const VOXEL_CM: f64 = 10.0;

/// Metres to centimetres, rid of the float noise of Exarust's metres, so
/// that a point on a cell or voxel boundary rounds as the exact tie it is
/// in the original's centimetres.
fn centimetres_of(metres: f64) -> f64 {
    (metres * 100_000.0).round() / 1000.0
}

fn centimetres(p: DVec3) -> DVec3 {
    DVec3::new(
        centimetres_of(p.x),
        centimetres_of(p.y),
        centimetres_of(p.z),
    )
}

/// The eight neighbours (dx, dz) and their weights.
const NEIGHBOURS: [(i32, i32, u32); 8] = [
    (-1, -1, DIAGONAL),
    (0, -1, ORTHOGONAL),
    (1, -1, DIAGONAL),
    (-1, 0, ORTHOGONAL),
    (1, 0, ORTHOGONAL),
    (-1, 1, DIAGONAL),
    (0, 1, ORTHOGONAL),
    (1, 1, DIAGONAL),
];

/// One of a column's two walkable layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// The column's only span, or the one under the upper floor.
    Lower = 0,
    /// The top span of a column that has two or more.
    Upper = 1,
}

impl Layer {
    pub const BOTH: [Layer; 2] = [Layer::Lower, Layer::Upper];
}

/// A span of a column: the top voxel of its slab and the voxel the scan
/// took as its underside ([`column_spans`]; 0 when the slab fills whole
/// bytes to the bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub floor: u8,
    pub underside: u8,
}

/// What a cell holds on one layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NavCell {
    /// Top voxel of the floor: 0 none, [`SKY`] open sky.
    pub floor: u8,
    /// Underside of the floor's slab ([`column_spans`]).
    pub underside: u8,
    pub roughness: u8,
    pub blurred: u8,
    /// What the planner tests: ≤ [`PASSABLE`]/(size class + 1) passes.
    pub clearance: u8,
    /// Footprints and props (added at run time).
    pub penalty: u8,
}

impl NavCell {
    /// A floor a walker can stand on (not missing, not open sky).
    pub fn has_floor(&self) -> bool {
        self.floor != 0 && self.floor != SKY
    }

    /// Whether a walker of `size` class may enter.
    pub fn passable(&self, size: u8) -> bool {
        self.has_floor() && u32::from(self.clearance) <= PASSABLE / (u32::from(size) + 1)
    }
}

/// One layer of the grid, a byte per cell (X fastest).
#[derive(Debug, Clone, Default)]
struct LayerBytes {
    floor: Vec<u8>,
    /// The floors as the build left them, before any mover raised one.
    built: Vec<u8>,
    underside: Vec<u8>,
    roughness: Vec<u8>,
    blurred: Vec<u8>,
    clearance: Vec<u8>,
    penalty: Vec<u8>,
}

impl LayerBytes {
    fn new(cells: usize) -> Self {
        Self {
            floor: vec![0; cells],
            built: vec![0; cells],
            underside: vec![0; cells],
            roughness: vec![0; cells],
            blurred: vec![0; cells],
            clearance: vec![0; cells],
            penalty: vec![0; cells],
        }
    }
}

/// The voxel columns of a grid being built: 256 bits per cell, bit `v` set
/// when voxel `v` is solid. Voxel `v` reaches up to `base + v·VOXEL`.
pub struct Columns {
    width: usize,
    depth: usize,
    base: f64,
    bits: Vec<[u8; COLUMN_VOXELS / 8]>,
}

impl Columns {
    /// Empty columns for a `width` × `depth` grid whose voxel 0 tops out at
    /// `base` metres.
    pub fn new(width: usize, depth: usize, base: f32) -> Self {
        Self {
            width,
            depth,
            base: f64::from(base),
            bits: vec![[0; COLUMN_VOXELS / 8]; width * depth],
        }
    }

    /// Grid coordinates of a world point (metres): fractional cells along X
    /// and Z (cell `i` spans `[i, i + 1)`), and voxels above the base, at
    /// the original's half-voxel offset (`lift` more for generated floor
    /// points). The caller rounds to the nearest, ties to even.
    fn grid(&self, p: DVec3, lift: f64) -> DVec3 {
        let cm = centimetres(p);
        DVec3::new(
            cm.x / CELL_CM - 0.5 + (self.width / 2) as f64,
            (cm.y - centimetres_of(self.base)) / VOXEL_CM + 0.5 + lift,
            cm.z / CELL_CM - 0.5 + (self.depth / 2) as f64,
        )
    }

    /// Marks the voxel holding a static mesh's sample point (world,
    /// metres); points on the grid's border cells or outside voxels
    /// 0–254 mark nothing.
    pub fn stamp(&mut self, p: DVec3, lift: f64) {
        let g = self.grid(p, lift);
        let (x, y, z) = (
            g.x.round_ties_even(),
            g.y.round_ties_even(),
            g.z.round_ties_even(),
        );
        if x <= 0.0
            || x >= (self.width - 1) as f64
            || z <= 0.0
            || z >= (self.depth - 1) as f64
            || !(0.0..(COLUMN_VOXELS - 1) as f64).contains(&y)
        {
            return;
        }
        let i = z as usize * self.width + x as usize;
        let v = y as usize;
        self.bits[i][v / 8] |= 1 << (v % 8);
    }

    /// Fills cell (`x`, `z`) from the bottom up to the voxel at `height`
    /// (metres), when it lies within the column.
    pub fn fill_ground(&mut self, x: usize, z: usize, height: f64) {
        if x >= self.width || z >= self.depth {
            return;
        }
        let above = centimetres_of(height) - centimetres_of(self.base);
        if !(0.0..=(COLUMN_VOXELS - 1) as f64 * VOXEL_CM).contains(&above) {
            return;
        }
        let top = (above / VOXEL_CM).round_ties_even() as usize;
        let column = &mut self.bits[z * self.width + x];
        for v in 0..=top.min(COLUMN_VOXELS - 1) {
            column[v / 8] |= 1 << (v % 8);
        }
    }

    /// Whether voxel `v` of cell (`x`, `z`) is solid.
    pub fn solid(&self, x: usize, z: usize, v: usize) -> bool {
        self.bits[z * self.width + x][v / 8] >> (v % 8) & 1 != 0
    }

    /// The column of cell (`x`, `z`).
    fn column(&self, x: usize, z: usize) -> &[u8; COLUMN_VOXELS / 8] {
        &self.bits[z * self.width + x]
    }
}

/// The spans of one column, top first (at most [`MAX_SPANS`]), found as
/// the original scans it: from the top, a byte (8 voxels) at a time, with
/// a cursor on one bit of the byte.
///
/// The scan alternates between looking for the next solid voxel (the top
/// of a slab) and the next free one (the voxel under it), each time only
/// at the cursor's bit and below in the byte, moving the cursor under the
/// voxel it finds. Where the rest of the byte holds none, it takes the
/// voxel above the cursor as the change and switches anyway, so the free
/// voxel recorded under a slab creeps down to the bottom of its byte (a
/// lone voxel at 221 has its underside at 216). A whole byte of what it is
/// passing over (all free, or all solid) is skipped, the cursor keeping
/// its bit. The room above a slab is counted from the last change, so it
/// too stops at a byte's bottom: a ceiling at 235 over a floor at 221
/// leaves 11 voxels, not 13, and joins the floor's span.
pub fn column_spans(column: &[u8; COLUMN_VOXELS / 8]) -> Vec<Span> {
    let mut spans = Vec::with_capacity(MAX_SPANS);
    let keep = |spans: &mut Vec<Span>, floor: i32, underside: i32| {
        spans.push(Span {
            floor: if floor > i32::from(SKY_FROM) {
                SKY
            } else {
                floor as u8
            },
            underside: underside as u8,
        });
    };
    let (mut byte, mut bit) = (column.len() as i32 - 1, 7);
    let mut in_air = true;
    // The span being gathered (its floor 0 until the first slab), and the
    // voxel of the last change between free and solid.
    let (mut floor, mut underside, mut last_change) = (0, 0, i32::MAX);
    while byte >= 0 {
        let bits = column[byte as usize];
        if bits == if in_air { 0 } else { u8::MAX } {
            byte -= 1;
            continue;
        }
        if let Some(k) = (0..=bit).rev().find(|&k| (bits >> k & 1 != 0) == in_air) {
            (byte, bit) = if k == 0 { (byte - 1, 7) } else { (byte, k - 1) };
        }
        let at = byte * 8 + bit + 1;
        in_air = !in_air;
        if in_air {
            underside = at;
        } else if last_change - at > HEADROOM_VOXELS {
            // A slab with room above starts a span; the one gathered so far
            // is kept unless it has no floor or a high underside.
            if floor > 0 && underside < i32::from(UNDERSIDE_LIMIT) {
                keep(&mut spans, floor, underside);
                if spans.len() == MAX_SPANS {
                    return spans;
                }
            }
            (floor, underside) = (at, 0);
        }
        last_change = at;
    }
    if floor > 0 {
        keep(&mut spans, floor, underside);
    }
    spans
}

/// How hard the step from a floor at `own` (1–254) onto a neighbour is,
/// in voxels (0–15): its height difference when it can be stepped on, 8
/// for a wall (its ground more than 40 cm up with too little room under
/// it), 15 where it has no ground or drops 150 cm or more.
pub fn step_measure(own: u8, lower: Span, upper: Span) -> u32 {
    let own = i32::from(own);
    let (fl, ul) = (i32::from(lower.floor), i32::from(lower.underside));
    let (fu, uu) = (i32::from(upper.floor), i32::from(upper.underside));
    let drop_or_step = |d: i32| if d < -14 { 15 } else { d.unsigned_abs() };
    if fl == 0 {
        return 15;
    }
    if fl > own + 4 {
        if ul < own + 5 || fu == 0 {
            return 8;
        }
        return (own - fu).clamp(8, 15) as u32;
    }
    if fu < fl {
        drop_or_step(fl - own)
    } else if uu < own + 5 {
        let d = fu - own;
        if d >= 5 { 8 } else { drop_or_step(d) }
    } else if uu < own + 15 {
        let d = fl - own;
        if d < -14 {
            15
        } else if d < -8 {
            d.unsigned_abs()
        } else {
            8
        }
    } else {
        drop_or_step(fl - own)
    }
}

/// Which of a neighbour's layers a walker on a floor at `own` meets: the
/// lower one unless the upper floor (or its underside) is within 150 cm
/// above `own`.
pub fn met_layer(own: u8, lower: Span, upper: Span) -> Layer {
    let own = u32::from(own);
    if upper.floor == 0 {
        return Layer::Lower;
    }
    if lower.floor < upper.floor {
        let reach = if upper.underside == 0 {
            upper.floor
        } else {
            upper.underside
        };
        if own + 15 < u32::from(reach) {
            Layer::Lower
        } else {
            Layer::Upper
        }
    } else if own + 15 < u32::from(lower.underside) {
        Layer::Upper
    } else {
        Layer::Lower
    }
}

/// The level's navigation grid.
#[derive(Debug, Clone, bevy::prelude::Resource)]
pub struct NavGrid {
    width: usize,
    depth: usize,
    base: f32,
    layers: [LayerBytes; 2],
}

/// What a grid holds, for logs and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NavStats {
    pub width: usize,
    pub depth: usize,
    /// Columns by their number of spans (0–3).
    pub columns_by_spans: [usize; 4],
    /// Cells with a floor (not open sky), per layer.
    pub floors: [usize; 2],
    /// Of those, the cells a walker of size class 0 may enter.
    pub passable: [usize; 2],
    /// Cells whose floor is open sky, per layer.
    pub sky: [usize; 2],
}

impl NavGrid {
    /// A grid with no floors.
    pub fn empty(width: usize, depth: usize, base: f32) -> Self {
        Self {
            width,
            depth,
            base,
            layers: [
                LayerBytes::new(width * depth),
                LayerBytes::new(width * depth),
            ],
        }
    }

    /// The grid's layers and cost bytes from its voxel columns.
    pub fn from_columns(columns: &Columns) -> Self {
        Self::from_columns_raised(columns, std::iter::empty())
    }

    /// The grid's layers from its voxel columns, its floors raised by the
    /// moving props' sample points (world, metres;
    /// [`mover::raise_from_movers`]), then its cost bytes, in the
    /// original's order.
    pub fn from_columns_raised(columns: &Columns, movers: impl IntoIterator<Item = Vec3>) -> Self {
        let (w, d) = (columns.width, columns.depth);
        let mut grid = Self::empty(w, d, columns.base as f32);
        let mut spans_found = vec![0u8; w * d];
        for z in 1..d.saturating_sub(1) {
            for x in 1..w.saturating_sub(1) {
                let spans = column_spans(columns.column(x, z));
                let i = z * w + x;
                spans_found[i] = spans.len() as u8;
                let put = |grid: &mut Self, layer: Layer, s: Span| {
                    let l = &mut grid.layers[layer as usize];
                    l.floor[i] = s.floor;
                    l.built[i] = s.floor;
                    l.underside[i] = s.underside;
                };
                match spans.as_slice() {
                    [] => {}
                    [only] => put(&mut grid, Layer::Lower, *only),
                    [top, next, ..] => {
                        put(&mut grid, Layer::Upper, *top);
                        put(&mut grid, Layer::Lower, *next);
                    }
                }
            }
        }
        mover::raise_from_movers(&mut grid, movers);
        grid.derive_costs(1, 1, w.saturating_sub(2), d.saturating_sub(2));
        grid
    }

    /// Cells along X and Z.
    pub fn size(&self) -> (usize, usize) {
        (self.width, self.depth)
    }

    /// Height (metres) the top of voxel 0 reaches.
    pub fn base(&self) -> f32 {
        self.base
    }

    /// Height (metres) of a floor byte.
    pub fn floor_height(&self, floor: u8) -> f32 {
        self.base + f32::from(floor) * VOXEL
    }

    /// The cell holding a world point (metres), as the original maps it.
    pub fn cell_at(&self, p: Vec3) -> Option<(usize, usize)> {
        let at = |c: f32, n: usize| {
            let g =
                (centimetres_of(f64::from(c)) / CELL_CM - 0.5 + (n / 2) as f64).round_ties_even();
            (g >= 0.0 && g < n as f64).then_some(g as usize)
        };
        Some((at(p.x, self.width)?, at(p.z, self.depth)?))
    }

    /// Centre of cell (`x`, `z`) on the X/Z plane, metres.
    pub fn cell_centre(&self, x: usize, z: usize) -> Vec2 {
        Vec2::new(
            (x as f32 - (self.width / 2) as f32 + 0.5) * CELL,
            (z as f32 - (self.depth / 2) as f32 + 0.5) * CELL,
        )
    }

    /// Writes what cell (`x`, `z`) holds on `layer`, its floor as built
    /// too. Outside the grid, nothing.
    pub fn set_cell(&mut self, x: usize, z: usize, layer: Layer, cell: NavCell) {
        if x >= self.width || z >= self.depth {
            return;
        }
        let i = z * self.width + x;
        let l = &mut self.layers[layer as usize];
        l.floor[i] = cell.floor;
        l.built[i] = cell.floor;
        l.underside[i] = cell.underside;
        l.roughness[i] = cell.roughness;
        l.blurred[i] = cell.blurred;
        l.clearance[i] = cell.clearance;
        l.penalty[i] = cell.penalty;
    }

    /// The floor of cell (`x`, `z`) on `layer` as the build left it (0
    /// outside the grid); movers raise the current floor over it.
    pub fn built_floor(&self, x: usize, z: usize, layer: Layer) -> u8 {
        if x >= self.width || z >= self.depth {
            return 0;
        }
        self.layers[layer as usize].built[z * self.width + x]
    }

    /// Writes the current floor of cell (`x`, `z`) on `layer`, leaving the
    /// built one. True when it changed.
    pub(crate) fn set_floor(&mut self, x: usize, z: usize, layer: Layer, floor: u8) -> bool {
        if x >= self.width || z >= self.depth {
            return false;
        }
        let at = &mut self.layers[layer as usize].floor[z * self.width + x];
        let changed = *at != floor;
        *at = floor;
        changed
    }

    /// Adds `amount` to (or takes it from) cell (`x`, `z`)'s penalty on
    /// `layer`, wrapping as the original's byte does. False outside the grid.
    pub fn shift_penalty(
        &mut self,
        x: usize,
        z: usize,
        layer: Layer,
        amount: u8,
        add: bool,
    ) -> bool {
        if x >= self.width || z >= self.depth {
            return false;
        }
        let at = &mut self.layers[layer as usize].penalty[z * self.width + x];
        *at = if add {
            at.wrapping_add(amount)
        } else {
            at.wrapping_sub(amount)
        };
        true
    }

    /// What cell (`x`, `z`) holds on `layer` (nothing outside the grid).
    pub fn cell(&self, x: usize, z: usize, layer: Layer) -> NavCell {
        if x >= self.width || z >= self.depth {
            return NavCell::default();
        }
        let i = z * self.width + x;
        let l = &self.layers[layer as usize];
        NavCell {
            floor: l.floor[i],
            underside: l.underside[i],
            roughness: l.roughness[i],
            blurred: l.blurred[i],
            clearance: l.clearance[i],
            penalty: l.penalty[i],
        }
    }

    fn span(&self, i: usize, layer: Layer) -> Span {
        let l = &self.layers[layer as usize];
        Span {
            floor: l.floor[i],
            underside: l.underside[i],
        }
    }

    /// Counts of what the grid holds.
    pub fn stats(&self) -> NavStats {
        let mut s = NavStats {
            width: self.width,
            depth: self.depth,
            ..Default::default()
        };
        for z in 0..self.depth {
            for x in 0..self.width {
                let mut spans = 0;
                for layer in Layer::BOTH {
                    let c = self.cell(x, z, layer);
                    let k = layer as usize;
                    if c.floor != 0 {
                        spans += 1;
                    }
                    if c.floor == SKY {
                        s.sky[k] += 1;
                    } else if c.has_floor() {
                        s.floors[k] += 1;
                        s.passable[k] += usize::from(c.passable(0));
                    }
                }
                s.columns_by_spans[spans] += 1;
            }
        }
        s
    }

    /// Roughness, blur and clearance over the rectangle `x0..=x1` ×
    /// `z0..=z1` (each pass one cell further in, as the original runs
    /// them). A rectangle too small for a pass is left alone.
    pub(crate) fn derive_costs(&mut self, x0: usize, z0: usize, x1: usize, z1: usize) {
        let w = self.width;
        let neighbours = |i: usize| {
            NEIGHBOURS.map(|(dx, dz, wt)| {
                (
                    (i as isize + dz as isize * w as isize + dx as isize) as usize,
                    wt,
                )
            })
        };
        let inner = |k: usize| (x0 + k, z0 + k, x1.saturating_sub(k), z1.saturating_sub(k));
        // Roughness.
        let (ax, az, bx, bz) = inner(1);
        let bx = bx.min(w.saturating_sub(1));
        let bz = bz.min(self.depth.saturating_sub(1));
        if ax <= bx && az <= bz {
            for z in az..=bz {
                for x in ax..=bx {
                    let i = z * w + x;
                    for layer in Layer::BOTH {
                        let own = self.layers[layer as usize].floor[i];
                        let value = match own {
                            0 => NO_FLOOR_ROUGHNESS,
                            SKY => SKY_ROUGHNESS,
                            _ => neighbours(i)
                                .iter()
                                .map(|&(n, wt)| {
                                    let m = step_measure(
                                        own,
                                        self.span(n, Layer::Lower),
                                        self.span(n, Layer::Upper),
                                    );
                                    (m * wt) >> 6
                                })
                                .max()
                                .unwrap_or(0) as u8,
                        };
                        self.layers[layer as usize].roughness[i] = value;
                    }
                }
            }
        }
        // Blur.
        let (ax, az, bx, bz) = inner(2);
        if ax <= bx && az <= bz {
            for z in az..=bz {
                for x in ax..=bx {
                    let i = z * w + x;
                    for layer in Layer::BOTH {
                        let own = self.layers[layer as usize].floor[i];
                        let value = match own {
                            0 => NO_FLOOR_ROUGHNESS,
                            SKY => SKY_ROUGHNESS,
                            _ => {
                                let mut sum = u32::from(self.layers[layer as usize].roughness[i]);
                                for (n, wt) in neighbours(i) {
                                    let met = met_layer(
                                        own,
                                        self.span(n, Layer::Lower),
                                        self.span(n, Layer::Upper),
                                    );
                                    let r = u32::from(self.layers[met as usize].roughness[n]);
                                    sum += (r * wt) >> 9;
                                }
                                (sum >> 1) as u8
                            }
                        };
                        self.layers[layer as usize].blurred[i] = value;
                    }
                }
            }
        }
        // Dilation into the clearance.
        let (ax, az, bx, bz) = inner(3);
        if ax <= bx && az <= bz {
            for z in az..=bz {
                for x in ax..=bx {
                    let i = z * w + x;
                    for layer in Layer::BOTH {
                        let own = self.layers[layer as usize].floor[i];
                        let value = match own {
                            0 | SKY => NO_FLOOR_CLEARANCE,
                            _ => {
                                let mut c = u32::from(self.layers[layer as usize].blurred[i]);
                                for (n, wt) in neighbours(i) {
                                    let met = met_layer(
                                        own,
                                        self.span(n, Layer::Lower),
                                        self.span(n, Layer::Upper),
                                    );
                                    let b = u32::from(self.layers[met as usize].blurred[n])
                                        .min(DILATION_CAP);
                                    c = c.max((b * wt) >> 9);
                                }
                                c.min(255) as u8
                            }
                        };
                        self.layers[layer as usize].clearance[i] = value;
                    }
                }
            }
        }
    }
}

/// The pathfinder's opening metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathfinderMetrics {
    pub cell: f32,
    pub depth: f32,
    pub inverse_cell: f32,
    pub seed: u8,
    pub counter: i32,
    pub base_calls: u32,
}

/// Stores the pathfinder's cell metrics and seed.
pub fn pathfinder_metrics(seed: u8) -> PathfinderMetrics {
    PathfinderMetrics {
        cell: 30.0,
        depth: 10.0,
        inverse_cell: 1.0 / 30.0,
        seed,
        counter: 0,
        base_calls: 1,
    }
}

/// The footprint extent's inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathExtentInput {
    pub slot: i32,
    pub inverse_cell: f32,
    pub axes: [i32; 2],
    pub local: [f32; 9],
    pub extents: [f32; 3],
}

/// The footprint's ring slot and clamped bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathExtent {
    pub slot: i32,
    pub mins: [i32; 2],
    pub maxs: [i32; 2],
}

/// Derives the rotated footprint bounds for both axes.
pub fn path_extents(input: PathExtentInput) -> PathExtent {
    let slot = (input.slot + 1) % 16;
    let mut mins = [0; 2];
    let mut maxs = [0; 2];
    for axis in 0..2usize {
        let column: [usize; 3] = if axis == 0 { [0, 3, 6] } else { [2, 5, 8] };
        let radius = input.local[column[0]].abs() * input.extents[0]
            + input.local[column[1]].abs() * input.extents[1]
            + input.local[column[2]].abs() * input.extents[2];
        let coordinate = input.local[column[0]];
        let half = input.axes[axis] >> 1;
        let low =
            half + ((coordinate - radius) * input.inverse_cell - 0.5).round_ties_even() as i32;
        let high =
            half + ((coordinate + radius) * input.inverse_cell + 0.5).round_ties_even() as i32;
        mins[axis] = low.max(3);
        maxs[axis] = high.min(input.axes[axis] - 4);
    }
    PathExtent { slot, mins, maxs }
}

#[cfg(test)]
mod tests;
