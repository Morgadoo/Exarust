//! The abstract terrain manager (`TTerrainManager`): its construction
//! numbers, the tile record, the per-vertex normals and heights, the 16 × 16
//! height bounds and the block descriptors the culling reads.
//!
//! A tile is 240 cells of 20 cm (4800 cm) with 241 vertices a side; its
//! heights are stored with an apron, 244 samples a side, one before the tile
//! and two after. The running terrain builder (`terrain::TerrainBuild`) owns
//! the mesh and texture work in Bevy and shares the detail displacement
//! ([`detail_offset`]) with this module. Its own Sobel normal still uses an
//! exact square root over metre heights: the original's [`window_normal`]
//! normalises with an approximate reciprocal square root over raw heights,
//! which moves one floor cell of the shipped nav golden, so it is kept apart
//! until that golden can be checked against the retail grid. Lengths are
//! centimetres where the original keeps them; the block trees, which
//! `exa_physics` builds in metres, take the same placement divided by 100.
//!
//! Floating-point order follows the instructions, not the decompiler: the
//! normal is normalised with the processor's approximate reciprocal square
//! root (`RSQRTSS`, [`exa_physics::engine_math::reciprocal_sqrt`]), the byte
//! conversions round to nearest even, and the running bounds keep the
//! original's NaN-safe compares.

use bevy::math::Vec3;
use exa_physics::engine_math::{reciprocal_sqrt, round_to_i64};
use exa_physics::kd_tree::TerrainKdTree;

/// Cells along a tile's side (240).
pub const TILE_CELLS: usize = 240;
/// Vertices along a tile's side (241).
pub const TILE_VERTICES: usize = TILE_CELLS + 1;
/// Height samples along a record's side, apron included (244).
pub const TILE_APRON: usize = TILE_CELLS + 4;
/// Elements allocated for the vertex channel's side (245); rows are read
/// with the apron stride, [`CHANNEL_STRIDE`].
pub const TILE_CHANNEL: usize = TILE_CELLS + 5;
/// Row stride of the vertex channel as it is read (244).
pub const CHANNEL_STRIDE: usize = TILE_APRON;
/// The cell size of a tile (20 cm).
pub const CELL_CM: f32 = 20.0;
/// A tile's side (48 m).
pub const TILE_CM: f32 = TILE_CELLS as f32 * CELL_CM;
/// One over a tile's side (the manager's constant-return slot).
pub const INVERSE_TILE: f32 = 1.0 / TILE_CM;
/// Blocks along a tile's side.
pub const BLOCK_SIDE: usize = 8;
/// Cells along a 6 m block's side (30).
pub const BLOCK_CELLS: usize = TILE_CELLS / BLOCK_SIDE;
/// Bounds cells along a side (16 × 16, each over 16 × 16 vertices).
pub const BOUND_CELLS: usize = 16;
/// Vertices between two bounds cells' corners (15; neighbours share one row
/// and column of vertices).
pub const BOUND_STEP: usize = 15;
/// The default channel value the record fills (0x7f, no displacement).
pub const CHANNEL_FILL: u8 = 0x7f;
/// Side of the compressed-normal and opacity arrays (256).
pub const COMPRESSED_STRIDE: usize = 256;
/// Bytes of a tile record as the manager allocates it.
pub const RECORD_BYTES: usize = 0x1_fae8;
/// Bytes of a record's vertex buffer: the positions, then the normals
/// (24 bytes a vertex; the first 12 bytes of each vertex are the position
/// half).
pub const VERTEX_BUFFER_BYTES: usize = TILE_VERTICES * TILE_VERTICES * 24;
/// Centimetres per step of the detail channel, before the slope factor.
pub const DETAIL_CM_PER_STEP: f32 = 1.25;
/// The detail channel's value for no displacement.
const DETAIL_ZERO: i64 = 0x7f;

/// The terrain quality tables, by the original's quality level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quality {
    /// The level's two table values.
    pub first: i32,
    pub second: i32,
}

impl Quality {
    /// The table the constructor selects from the quality level (a setting
    /// of its own, not the texture-detail one): below 0 and above 4 it is
    /// (0, 1); 0 to 2 give (3, 3); 3 gives (2, 3); 4 gives (1, 2).
    pub fn from_level(level: i32) -> Self {
        let (first, second) = match level {
            0..=2 => (3, 3),
            3 => (2, 3),
            4 => (1, 2),
            _ => (0, 1),
        };
        Self { first, second }
    }
}

/// What the constructor reads: two global settings and the configuration
/// record the caller passes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManagerSettings {
    /// The texture-detail setting: below 3 the detail size is 4096, else
    /// 8192.
    pub texture_detail: i32,
    /// The quality level that picks the quality tables (4 by default).
    pub quality_level: i32,
    /// The global that, with the record's bit 0x100 clear, makes the
    /// manager create GPU buffers for its tiles.
    pub gpu_buffers: bool,
    /// The record's vertical scale (the word after the first).
    pub vertical_scale: f32,
    /// The record's palette scale; zero derives it from the detail size.
    pub palette_scale: i32,
    /// The record's flags: bit 2 is kept as the manager's second flag,
    /// bit 0x100 turns the GPU buffers off.
    pub flags: u32,
    /// The record's detail-texture scale (times the cell size).
    pub detail_scale: f32,
    /// The record's trailing word, kept as it is.
    pub extra: f32,
}

impl ManagerSettings {
    /// The default configuration record the plain manager is built with.
    pub const DEFAULT: Self = Self {
        texture_detail: 4,
        quality_level: 4,
        gpu_buffers: true,
        vertical_scale: 1.0,
        palette_scale: 4,
        flags: 0x13,
        detail_scale: 1.0,
        extra: 0.0,
    };

    /// The configuration record the static terrain builds its manager with
    /// (palette scale 4, flags 0x7b, trailing word 0.015).
    pub const STATIC: Self = Self {
        flags: 0x7b,
        extra: f32::from_bits(0x3c75_c28f),
        ..Self::DEFAULT
    };
}

/// The manager's construction numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManagerConfig {
    /// A tile's side (cm).
    pub tile_cm: f32,
    /// The cell size (cm).
    pub cell_cm: f32,
    /// Cells along a tile (240).
    pub cells: i32,
    /// Vertices along a tile (241).
    pub vertices: i32,
    /// Height rows with the apron (244).
    pub apron: i32,
    /// Vertices in the tile's vertex grid (241²).
    pub vertex_count: i32,
    /// The detail texture's size (4096, or 8192 from texture detail 3).
    pub detail_size: u32,
    /// The palette's scale: the configured integer, or detail size / 2048.
    pub palette_scale: f32,
    /// The quality tables.
    pub quality: Quality,
    /// The detail texture's source scale (20 × the record's word).
    pub detail_scale: f32,
    /// Centimetres per raw height unit: the vertical scale times 20 × 0.1,
    /// the 0.1 a double (2.0 for a vertical scale of 1).
    pub height_scale: f32,
    /// Bit 2 of the record's flags.
    pub second_flag: bool,
    /// Whether tiles get GPU buffers.
    pub gpu_buffers: bool,
    /// The record's trailing word.
    pub extra: f32,
}

impl ManagerConfig {
    /// The manager's virtual slot that answers whether it is ready: false
    /// in the base.
    pub fn ready(&self) -> bool {
        false
    }

    /// The manager's constant-return slot: one over the tile side.
    pub fn inverse_tile(&self) -> f32 {
        INVERSE_TILE
    }

    /// The constructor's values.
    pub fn new(settings: &ManagerSettings) -> Self {
        let detail_size = super::composite::detail_size(settings.texture_detail);
        let palette_scale = if settings.palette_scale == 0 {
            (detail_size >> 11) as f32
        } else {
            settings.palette_scale as f32
        };
        let height_scale = (f64::from(settings.vertical_scale) * (20.0_f64 * 0.1_f64)) as f32;
        Self {
            tile_cm: TILE_CM,
            cell_cm: CELL_CM,
            cells: TILE_CELLS as i32,
            vertices: TILE_VERTICES as i32,
            apron: TILE_APRON as i32,
            vertex_count: (TILE_VERTICES * TILE_VERTICES) as i32,
            detail_size,
            palette_scale,
            quality: Quality::from_level(settings.quality_level),
            detail_scale: CELL_CM * settings.detail_scale,
            height_scale,
            second_flag: settings.flags & 2 != 0,
            gpu_buffers: settings.gpu_buffers && settings.flags & 0x100 == 0,
            extra: settings.extra,
        }
    }

    /// The factor the Sobel sums are scaled by: the height scale over the
    /// cell size, times one eighth.
    pub fn normal_factor(&self) -> f32 {
        (self.height_scale / CELL_CM) * 0.125
    }
}

/// Which optional arrays a tile record carries (the manager's globals and
/// flags at the time of the call).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TileOptions {
    /// Normals packed to two bytes a vertex.
    pub compressed_normals: bool,
    /// The per-vertex opacity bytes.
    pub opacity: bool,
}

/// The vector length: `sqrt((x² + z²) + y²)`, summed in that order.
fn length(x: f32, y: f32, z: f32) -> f32 {
    ((x * x + z * z) + y * y).sqrt()
}

/// Scales a vector by the approximate reciprocal square root of its squared
/// length, summed `(x² + z²) + y²`; a zero length scales by zero.
fn normalise(v: [f32; 3]) -> [f32; 3] {
    let sum = (v[0] * v[0] + v[2] * v[2]) + v[1] * v[1];
    let scale = if sum == 0.0 {
        0.0
    } else {
        reciprocal_sqrt(sum)
    };
    [v[0] * scale, v[1] * scale, v[2] * scale]
}

/// The surface normal from a 3 × 3 window of heights, `window[row][col]`
/// with the vertex at the centre: a Sobel filter, `x` the left column less
/// the right, `z` the top row less the bottom, `y` one, each sum scaled by
/// `factor` ([`ManagerConfig::normal_factor`] for raw heights) and the
/// result normalised.
pub fn window_normal(window: [[f32; 3]; 3], factor: f32) -> [f32; 3] {
    let a = window;
    let x = ((((2.0 * a[1][0] + a[0][0]) + a[2][0]) - a[0][2]) - 2.0 * a[1][2]) - a[2][2];
    let z = ((((2.0 * a[0][1] + a[0][0]) + a[0][2]) - a[2][0]) - 2.0 * a[2][1]) - a[2][2];
    normalise([x * factor, 1.0, z * factor])
}

/// The detail channel's displacement of a vertex, centimetres:
/// `(byte − 127) × 1.25 × (2 − normal y)`. A level vertex on a flat normal
/// moves by the byte's steps alone; steeper ground moves further.
pub fn detail_offset(byte: f32, normal_y: f32) -> f32 {
    (byte - DETAIL_ZERO as f32) * DETAIL_CM_PER_STEP * (2.0 - normal_y)
}

/// One tile's record: its state, heights with the apron, the centred
/// vertices and their normals (the two halves of the original's vertex
/// buffer), the vertex channel and the optional packed arrays.
#[derive(Clone, Debug, PartialEq)]
pub struct TileRecord {
    /// The record's state word (7 when raw data is present).
    pub state: u32,
    /// Height samples, `TILE_APRON²`, row-major, raw (centimetres before the
    /// height scale).
    pub heights: Vec<f32>,
    /// The vertex positions, `TILE_VERTICES²`, centimetres, centred.
    pub positions: Vec<[f32; 3]>,
    /// The vertex normals, `TILE_VERTICES²`.
    pub normals: Vec<[f32; 3]>,
    /// The vertex channel (`TILE_CHANNEL²`, filled with 0x7f), read with
    /// [`CHANNEL_STRIDE`] columns a row.
    pub channel: Vec<u8>,
    /// Normals packed to two bytes a vertex, `COMPRESSED_STRIDE` vertices a
    /// row (when the option is on).
    pub compressed_normals: Option<Vec<u8>>,
    /// The opacity bytes, `COMPRESSED_STRIDE` vertices a row (when the
    /// option is on).
    pub opacity: Option<Vec<u8>>,
    /// The 16 × 16 height bounds.
    pub bounds: HeightBounds,
}

impl TileRecord {
    /// Allocates the record as the manager does: heights zero, the vertex
    /// grid centred at `((i − 120)·20, 0, (j − 120)·20)` cm, the channel
    /// filled with 0x7f, the normals zero.
    pub fn allocate(options: TileOptions) -> Self {
        let mut positions = Vec::with_capacity(TILE_VERTICES * TILE_VERTICES);
        let half = TILE_CELLS as f32 * 0.5;
        for j in 0..TILE_VERTICES {
            for i in 0..TILE_VERTICES {
                positions.push([
                    (i as f32 - half) * CELL_CM,
                    0.0,
                    (j as f32 - half) * CELL_CM,
                ]);
            }
        }
        Self {
            state: 7,
            heights: vec![0.0; TILE_APRON * TILE_APRON],
            positions,
            normals: vec![[0.0; 3]; TILE_VERTICES * TILE_VERTICES],
            channel: vec![CHANNEL_FILL; TILE_CHANNEL * TILE_CHANNEL],
            compressed_normals: options
                .compressed_normals
                .then(|| vec![0; COMPRESSED_STRIDE * COMPRESSED_STRIDE * 2]),
            opacity: options
                .opacity
                .then(|| vec![0; COMPRESSED_STRIDE * COMPRESSED_STRIDE]),
            bounds: [[(0.0, 0.0); BOUND_CELLS]; BOUND_CELLS],
        }
    }

    /// The normals pass: every vertex's Sobel normal from the apron window
    /// whose centre is `(row + 1, col + 1)`.
    pub fn compute_normals(&mut self, config: &ManagerConfig) {
        let factor = config.normal_factor();
        for row in 0..TILE_VERTICES {
            for col in 0..TILE_VERTICES {
                let at = |dr: usize, dc: usize| self.heights[(row + dr) * TILE_APRON + col + dc];
                let window = [
                    [at(0, 0), at(0, 1), at(0, 2)],
                    [at(1, 0), at(1, 1), at(1, 2)],
                    [at(2, 0), at(2, 1), at(2, 2)],
                ];
                self.normals[row * TILE_VERTICES + col] = window_normal(window, factor);
            }
        }
    }

    /// The compressed-normals pass: each normal's x and z, `(n + 1) × 127.5`
    /// rounded to nearest even, as the low byte of the integer.
    fn compress_normals(&mut self) {
        let Some(out) = &mut self.compressed_normals else {
            return;
        };
        for row in 0..TILE_VERTICES {
            for col in 0..TILE_VERTICES {
                let n = self.normals[row * TILE_VERTICES + col];
                let at = (row * COMPRESSED_STRIDE + col) * 2;
                out[at] = round_to_i64((n[0] + 1.0) * 127.5) as u8;
                out[at + 1] = round_to_i64((n[2] + 1.0) * 127.5) as u8;
            }
        }
    }

    /// The normals-only path: the normals and, when on, their packed form,
    /// with no vertex heights.
    pub fn finalise_normals(&mut self, config: &ManagerConfig) {
        self.compute_normals(config);
        self.compress_normals();
    }

    /// The finalise pass: the normals, then each vertex's height
    /// (`detail_offset(channel byte, normal y)` plus the apron height times
    /// the height scale), the packed normals, the opacity bytes and the
    /// height bounds.
    pub fn finalise(&mut self, config: &ManagerConfig) {
        self.compute_normals(config);
        for row in 0..TILE_VERTICES {
            for col in 0..TILE_VERTICES {
                let v = row * TILE_VERTICES + col;
                let byte = i64::from(self.channel[row * CHANNEL_STRIDE + col]);
                let detail =
                    ((byte - DETAIL_ZERO) as f32 * DETAIL_CM_PER_STEP) * (2.0 - self.normals[v][1]);
                let ground = self.heights[(row + 1) * TILE_APRON + col + 1] * config.height_scale;
                self.positions[v][1] = detail + ground;
            }
        }
        self.compress_normals();
        if let Some(opacity) = &mut self.opacity {
            for row in 0..TILE_VERTICES {
                for col in 0..TILE_VERTICES {
                    let y = self.positions[row * TILE_VERTICES + col][1];
                    opacity[row * COMPRESSED_STRIDE + col] = opacity_byte(y);
                }
            }
        }
        self.bounds = height_bounds(&self.positions);
    }

    /// The destroy slot: the state's bit 8 is cleared; true when the worker
    /// is to be woken (the manager's worker word has bit 2 set).
    pub fn release(&mut self, worker_flags: u32) -> bool {
        self.state &= !8;
        worker_flags & 2 != 0
    }
}

/// The 8 × 8 shared block trees the manager creates at initialisation.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockTrees {
    pub trees: Vec<TerrainKdTree>,
}

impl BlockTrees {
    /// Creates one 30-cell tree per 6 m block and translates it to the
    /// block's centre. The first centre is at −21 m, the step 6 m.
    pub fn new() -> Self {
        let mut trees = Vec::with_capacity(BLOCK_SIDE * BLOCK_SIDE);
        for row in 0..BLOCK_SIDE {
            for col in 0..BLOCK_SIDE {
                trees.push(
                    TerrainKdTree::build(BLOCK_CELLS, CELL_CM / 100.0)
                        .translated(Self::offset(col, row)),
                );
            }
        }
        Self { trees }
    }

    /// The translation of block (column, row), metres: the original's
    /// centimetre placement, `tile/8 × index + (tile/16 − tile/2)`, over 100.
    pub fn offset(col: usize, row: usize) -> Vec3 {
        let base = TILE_CM * 0.0625 - TILE_CM * 0.5;
        Vec3::new(
            (TILE_CM * 0.125 * col as f32 + base) / 100.0,
            0.0,
            (TILE_CM * 0.125 * row as f32 + base) / 100.0,
        )
    }
}

impl Default for BlockTrees {
    fn default() -> Self {
        Self::new()
    }
}

/// The 16 × 16 height bounds: one (min, max) pair per bounds cell.
pub type HeightBounds = [[(f32, f32); BOUND_CELLS]; BOUND_CELLS];

/// The height bounds of a tile, read from the finished vertex positions:
/// bounds cell (row, col) covers the 16 × 16 vertices from (15·row, 15·col).
/// The running pair starts at the first vertex's height and keeps the
/// original's NaN-safe test (`v < min`, else `v > max`).
pub fn height_bounds(positions: &[[f32; 3]]) -> HeightBounds {
    let mut out = [[(0.0, 0.0); BOUND_CELLS]; BOUND_CELLS];
    for (row, line) in out.iter_mut().enumerate() {
        for (col, cell) in line.iter_mut().enumerate() {
            let at = |dr: usize, dc: usize| {
                positions[(BOUND_STEP * row + dr) * TILE_VERTICES + BOUND_STEP * col + dc][1]
            };
            let first = at(0, 0);
            let (mut lo, mut hi) = (first, first);
            for dr in 0..BOUND_CELLS {
                for dc in 0..BOUND_CELLS {
                    let v = at(dr, dc);
                    if v < lo {
                        lo = v;
                    } else if v > hi {
                        hi = v;
                    }
                }
            }
            *cell = (lo, hi);
        }
    }
    out
}

/// A block's culling volume: a centre, half extents and the bounding radius,
/// all centimetres from the tile's centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockDescriptor {
    /// Depth in the block tree: 0 is the whole tile, 3 a 6 m block.
    pub level: u32,
    /// The block's first bounds cell, column and row.
    pub column: u16,
    pub row: u16,
    pub centre: [f32; 3],
    pub half: [f32; 3],
    pub radius: f32,
}

impl BlockDescriptor {
    /// The descriptor of the block at `level` whose first bounds cell is
    /// (`column`, `row`). With the tile's `bounds` the vertical extent is
    /// the min and max over the block's `(16 >> level)²` bounds cells
    /// (the max compare is a separate, NaN-safe test); without them the
    /// centre is 0 and the half extent 10. One is added to the vertical half
    /// extent either way.
    pub fn new(level: u32, column: u16, row: u16, bounds: Option<&HeightBounds>) -> Self {
        let (centre_y, mut half_y) = match bounds {
            Some(bounds) => {
                let span = (BOUND_CELLS >> level).max(1);
                let (r0, c0) = (usize::from(row), usize::from(column));
                let first = bounds[r0][c0];
                let (mut lo, mut hi) = first;
                for line in bounds.iter().skip(r0).take(span) {
                    for &(cell_lo, cell_hi) in line.iter().skip(c0).take(span) {
                        if cell_lo < lo {
                            lo = cell_lo;
                        }
                        if cell_hi > hi {
                            hi = cell_hi;
                        }
                    }
                }
                let half = (hi - lo) * 0.5;
                (lo + half, half)
            }
            None => (0.0, 10.0),
        };
        half_y += 1.0;
        let half_extent = TILE_CM / ((2u32 << level) as f32);
        let origin = half_extent - TILE_CM * 0.5;
        let step = TILE_CM / (1u32 << 4) as f32;
        let centre_x = step * f32::from(column) + origin;
        let centre_z = step * f32::from(row) + origin;
        Self {
            level,
            column,
            row,
            centre: [centre_x, centre_y, centre_z],
            half: [half_extent, half_y, half_extent],
            radius: length(half_extent, half_y, half_extent),
        }
    }
}

/// The opacity byte of a vertex from its height `y`: `(160 − (15 − y)) ×
/// 0.00625`, zero below 0, 255 above 1, else `v × 255` rounded to nearest
/// even.
pub fn opacity_byte(y: f32) -> u8 {
    let v = (160.0 - (15.0 - y)) * OPACITY_SLOPE;
    if v < 0.0 {
        0
    } else if v > 1.0 {
        255
    } else {
        round_to_i64(v * 255.0) as u8
    }
}

/// The opacity slope, 0.00625 as the single-precision constant.
const OPACITY_SLOPE: f32 = 0.006_25;

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ManagerConfig {
        ManagerConfig::new(&ManagerSettings::DEFAULT)
    }

    #[test]
    fn the_constructor_numbers() {
        let c = config();
        assert_eq!(c.tile_cm, 4800.0);
        assert_eq!(c.cell_cm, 20.0);
        assert_eq!(c.cells, 240);
        assert_eq!(c.vertices, 241);
        assert_eq!(c.apron, 244);
        assert_eq!(c.vertex_count, 58081);
        assert_eq!(c.detail_size, 8192);
        assert_eq!(c.palette_scale, 4.0);
        assert_eq!(c.height_scale, 2.0);
        assert_eq!(c.detail_scale, 20.0);
        assert!(c.second_flag && c.gpu_buffers);
        assert_eq!(
            c.quality,
            Quality {
                first: 1,
                second: 2
            }
        );
        assert_eq!(INVERSE_TILE.to_bits(), 0x395a_740e);
        assert!(!c.ready());
    }

    #[test]
    fn detail_size_and_quality_tables_have_their_own_settings() {
        let low = ManagerSettings {
            texture_detail: 2,
            quality_level: 0,
            palette_scale: 0,
            ..ManagerSettings::DEFAULT
        };
        let c = ManagerConfig::new(&low);
        assert_eq!(c.detail_size, 4096);
        // A zero palette scale derives detail size / 2048.
        assert_eq!(c.palette_scale, 2.0);
        assert_eq!(c.quality, Quality::from_level(2));
        let table = |level| {
            let q = Quality::from_level(level);
            (q.first, q.second)
        };
        assert_eq!(table(-1), (0, 1));
        assert_eq!(table(0), (3, 3));
        assert_eq!(table(3), (2, 3));
        assert_eq!(table(4), (1, 2));
        assert_eq!(table(5), (0, 1));
        // The detail size and the quality level do not follow each other.
        let mixed = ManagerSettings {
            texture_detail: 3,
            quality_level: 0,
            ..ManagerSettings::DEFAULT
        };
        let c = ManagerConfig::new(&mixed);
        assert_eq!((c.detail_size, c.quality.first), (8192, 3));
    }

    #[test]
    fn the_static_record_and_flags() {
        let c = ManagerConfig::new(&ManagerSettings::STATIC);
        assert!(c.second_flag && c.gpu_buffers);
        assert_eq!(c.palette_scale, 4.0);
        assert_eq!(c.extra.to_bits(), 0x3c75_c28f);
        let plain = ManagerSettings {
            flags: 0x100,
            vertical_scale: 2.0,
            detail_scale: 2.5,
            ..ManagerSettings::DEFAULT
        };
        let c = ManagerConfig::new(&plain);
        assert!(!c.second_flag && !c.gpu_buffers);
        assert_eq!(c.height_scale, 4.0);
        assert_eq!(c.detail_scale, 50.0);
        let no_gpu = ManagerSettings {
            gpu_buffers: false,
            ..ManagerSettings::DEFAULT
        };
        assert!(!ManagerConfig::new(&no_gpu).gpu_buffers);
    }

    #[test]
    fn the_tile_record_has_the_original_sizes() {
        let tile = TileRecord::allocate(TileOptions::default());
        assert_eq!(tile.heights.len(), 244 * 244);
        assert_eq!(tile.positions.len(), 241 * 241);
        assert_eq!(tile.normals.len(), 241 * 241);
        assert_eq!(tile.channel.len(), 245 * 245);
        assert!(tile.channel.iter().all(|&b| b == 0x7f));
        assert!(tile.compressed_normals.is_none() && tile.opacity.is_none());
        assert_eq!(RECORD_BYTES, 129_768);
        assert_eq!(VERTEX_BUFFER_BYTES, 58081 * 24);
        assert_eq!(tile.positions[0], [-2400.0, 0.0, -2400.0]);
        assert_eq!(tile.positions[241 * 241 - 1], [2400.0, 0.0, 2400.0]);
        assert_eq!(tile.positions[120 * 241 + 120], [0.0, 0.0, 0.0]);
        assert_eq!(tile.state, 7);
        let full = TileRecord::allocate(TileOptions {
            compressed_normals: true,
            opacity: true,
        });
        assert_eq!(full.compressed_normals.unwrap().len(), 256 * 256 * 2);
        assert_eq!(full.opacity.unwrap().len(), 256 * 256);
    }

    #[test]
    fn the_normal_factor_and_flat_normals() {
        let c = config();
        // 2 / 20 × 0.125 = 0.0125.
        assert!((c.normal_factor() - 0.0125).abs() < 1e-9);
        let mut tile = TileRecord::allocate(TileOptions::default());
        tile.compute_normals(&c);
        // A flat window gives (0, 1, 0) scaled by the approximate
        // reciprocal square root of one.
        let up = tile.normals[120 * 241 + 120];
        assert_eq!(up, [0.0, reciprocal_sqrt(1.0), 0.0]);
        assert!((up[1] - 1.0).abs() < 4e-4);
    }

    #[test]
    fn a_slope_tilts_the_normal_away_from_the_rise() {
        let c = config();
        let mut tile = TileRecord::allocate(TileOptions::default());
        // Eight raw units a column: x is 4 × (left − right) = −64 times the
        // factor, so the normal leans to −x: −0.8 over √1.64 is about −0.6247.
        for row in 0..244 {
            for col in 0..244 {
                tile.heights[row * 244 + col] = col as f32 * 8.0;
            }
        }
        tile.compute_normals(&c);
        let n = tile.normals[120 * 241 + 120];
        assert!((n[0] + 0.6247).abs() < 5e-4, "{n:?}");
        assert!((n[1] - 0.7809).abs() < 5e-4, "{n:?}");
        assert_eq!(n[2], 0.0);
        // Rows rising the same way tilt z, with the sign of the top less the
        // bottom.
        for row in 0..244 {
            for col in 0..244 {
                tile.heights[row * 244 + col] = row as f32 * 8.0;
            }
        }
        tile.compute_normals(&c);
        let n = tile.normals[120 * 241 + 120];
        assert!((n[2] + 0.6247).abs() < 5e-4 && n[0] == 0.0, "{n:?}");
    }

    #[test]
    fn the_window_normal_sums_in_the_original_order() {
        // Weights: x = 2·left + top-left + bottom-left − top-right −
        // 2·right − bottom-right.
        let w = [[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        assert_eq!(window_normal(w, 1.0), normalise([1.0, 1.0, 1.0]));
        let w = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        assert_eq!(window_normal(w, 1.0), normalise([2.0, 1.0, 0.0]));
        let w = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        assert_eq!(window_normal(w, 1.0), normalise([0.0, 1.0, -2.0]));
        // A zero vector stays zero.
        assert_eq!(normalise([0.0; 3]), [0.0; 3]);
    }

    #[test]
    fn finalise_places_the_vertices_and_packs_the_arrays() {
        let c = config();
        let mut tile = TileRecord::allocate(TileOptions {
            compressed_normals: true,
            opacity: true,
        });
        // A raw height of 100 on the centre vertex's apron cell, and a
        // channel byte of 128 (one step up) there.
        tile.heights[121 * 244 + 121] = 100.0;
        tile.channel[120 * CHANNEL_STRIDE + 120] = 128;
        tile.finalise(&c);
        let y = tile.positions[120 * 241 + 120][1];
        // Flat ground under it keeps the approximate unit normal: one step of
        // 1.25 × (2 − ny) and 100 × 2.
        assert!((y - 201.25).abs() < 0.01, "{y}");
        // An untouched channel (0x7f) adds nothing, so a flat vertex rests on
        // its height.
        assert_eq!(tile.positions[0][1], 0.0);
        // The packed normal of an up normal is 127.5 → 128 (nearest even),
        // not 127.
        let packed = tile.compressed_normals.as_ref().unwrap();
        assert_eq!(packed[(10 * 256 + 10) * 2], 128);
        assert_eq!(packed[(10 * 256 + 10) * 2 + 1], 128);
        // Opacity of a vertex at height 0 is 231 and of one at 201 is 255.
        let opacity = tile.opacity.as_ref().unwrap();
        assert_eq!(opacity[0], 231);
        assert_eq!(opacity[120 * 256 + 120], 255);
        // The centre vertex raises the bounds that contain it.
        assert!(tile.bounds[8][8].1 > 200.0);
        assert_eq!(tile.bounds[0][0], (0.0, 0.0));
    }

    #[test]
    fn the_normals_only_path_leaves_the_heights_alone() {
        let c = config();
        let mut tile = TileRecord::allocate(TileOptions {
            compressed_normals: true,
            opacity: false,
        });
        tile.heights[121 * 244 + 121] = 100.0;
        tile.finalise_normals(&c);
        assert_eq!(tile.positions[120 * 241 + 120][1], 0.0);
        assert!(tile.compressed_normals.is_some());
    }

    #[test]
    fn releasing_a_record_clears_bit_eight() {
        let mut tile = TileRecord::allocate(TileOptions::default());
        tile.state = 0xf;
        assert!(tile.release(2));
        assert_eq!(tile.state, 7);
        assert!(!tile.release(5));
    }

    #[test]
    fn the_block_trees_cover_eight_by_eight_blocks() {
        let blocks = BlockTrees::new();
        assert_eq!(blocks.trees.len(), 64);
        assert_eq!(BlockTrees::offset(0, 0), Vec3::new(-21.0, 0.0, -21.0));
        assert_eq!(BlockTrees::offset(7, 7), Vec3::new(21.0, 0.0, 21.0));
        assert_eq!(BlockTrees::offset(4, 0), Vec3::new(3.0, 0.0, -21.0));
        // Each tree has a translated root split plane: the first at the
        // block centre.
        assert_eq!(blocks.trees[0].nodes[0].value, -21.0);
        assert_eq!(blocks.trees[63].nodes[0].value, 21.0);
    }

    #[test]
    fn height_bounds_scan_sixteen_vertices_from_every_fifteenth() {
        // Height = vertex index: bounds cell (0, 0) spans rows and columns
        // 0 to 15, cell (1, 0) starts at row 15.
        let positions: Vec<[f32; 3]> = (0..241 * 241).map(|i| [0.0, i as f32, 0.0]).collect();
        let bounds = height_bounds(&positions);
        assert_eq!(bounds[0][0], (0.0, (15 * 241 + 15) as f32));
        assert_eq!(bounds[1][0], ((15 * 241) as f32, (30 * 241 + 15) as f32));
        assert_eq!(bounds[0][1].0, 15.0);
        // The last cell ends on the last vertex.
        assert_eq!(
            bounds[15][15],
            ((225 * 241 + 225) as f32, (240 * 241 + 240) as f32)
        );
    }

    #[test]
    fn a_nan_height_leaves_the_running_pair_alone() {
        let mut positions: Vec<[f32; 3]> =
            (0..241 * 241).map(|i| [0.0, (i % 7) as f32, 0.0]).collect();
        positions[5][1] = f32::NAN;
        let bounds = height_bounds(&positions);
        assert_eq!(bounds[0][0], (0.0, 6.0));
    }

    #[test]
    fn a_block_descriptor_with_and_without_bounds() {
        let mut bounds = [[(0.0f32, 0.0f32); BOUND_CELLS]; BOUND_CELLS];
        bounds[0][0] = (-10.0, 30.0);
        bounds[15][15] = (0.0, 1000.0);
        let whole = BlockDescriptor::new(0, 0, 0, Some(&bounds));
        // Min −10, max 1000: centre 495, half extent 505 + 1.
        assert_eq!(whole.centre, [0.0, 495.0, 0.0]);
        assert_eq!(whole.half, [2400.0, 506.0, 2400.0]);
        let radius = ((2400.0f32 * 2400.0 * 2.0) + 506.0 * 506.0).sqrt();
        assert!((whole.radius - radius).abs() < 1e-2);
        // A 6 m block (level 3) at bounds cell (2, 5) without bounds: the
        // centre y is 0 and the half height 11.
        let block = BlockDescriptor::new(3, 2, 5, None);
        assert_eq!(block.centre, [-1500.0, 0.0, -600.0]);
        assert_eq!(block.half, [300.0, 11.0, 300.0]);
        // With bounds, that block covers two cells a side: only cell (5, 2)
        // to (6, 3) count.
        let mut local = [[(0.0f32, 0.0f32); BOUND_CELLS]; BOUND_CELLS];
        local[6][3] = (-4.0, 8.0);
        local[7][3] = (-100.0, 100.0);
        let block = BlockDescriptor::new(3, 2, 5, Some(&local));
        assert_eq!(block.centre[1], 2.0);
        assert_eq!(block.half[1], 7.0);
    }

    #[test]
    fn the_opacity_byte_rules() {
        assert_eq!(opacity_byte(15.0), 255);
        assert_eq!(opacity_byte(-145.0), 0);
        assert_eq!(opacity_byte(-200.0), 0);
        assert_eq!(opacity_byte(100.0), 255);
        // (160 − 15) × 0.00625 = 0.90625; × 255 = 231.09 rounds to 231.
        assert_eq!(opacity_byte(0.0), 231);
        // (160 − 35) × 0.00625 = 0.78125; × 255 = 199.22 rounds to 199.
        assert_eq!(opacity_byte(-20.0), 199);
        assert_eq!(OPACITY_SLOPE.to_bits(), 0x3bcc_cccd);
    }

    #[test]
    fn the_detail_offset_follows_the_slope() {
        assert_eq!(detail_offset(127.0, 0.3), 0.0);
        // One step up on flat ground: 1.25 cm.
        assert_eq!(detail_offset(128.0, 1.0), 1.25);
        // On a normal y of 0.5 the step is 1.25 × 1.5.
        assert_eq!(detail_offset(128.0, 0.5), 1.875);
        assert_eq!(detail_offset(117.0, 1.0), -12.5);
    }
}
