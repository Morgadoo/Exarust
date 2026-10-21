//! The abstract terrain manager's configuration and records
//! (`TTerrainManager`): the 48 m tile of 240 × 20 cm cells, the detail-size
//! and palette tables, the shared 8 × 8 block kd-trees, the tile record's
//! allocation (heights, centred vertices, a vertex channel), the 16 × 16
//! height bounds, the 6 m block bounds and the per-tile opacity byte.
//!
//! The running terrain builder (`terrain::TerrainBuild`) owns the actual
//! mesh and texture work in Bevy; this module holds the original's layout
//! and arithmetic as plain data so the constants and the number formats can
//! be tested on their own. Lengths are centimetres where the original keeps
//! them, metres where the block trees are asked in metres.

use bevy::math::Vec3;
use exa_physics::kd_tree::TerrainKdTree;

/// Cells along a tile's side (240).
pub const TILE_CELLS: usize = 240;
/// Vertices along a tile's side (241).
pub const TILE_VERTICES: usize = TILE_CELLS + 1;
/// Height rows in the record, apron included (244).
pub const TILE_APRON: usize = TILE_CELLS + 4;
/// A vertex channel's side (245).
pub const TILE_CHANNEL: usize = TILE_CELLS + 5;
/// The cell size of a tile (20 cm).
pub const CELL_CM: f32 = 20.0;
/// A tile's side (48 m).
pub const TILE_CM: f32 = TILE_CELLS as f32 * CELL_CM;
/// One over a tile's side (the manager's constant-return stub).
pub const INVERSE_TILE: f32 = 1.0 / TILE_CM;
/// Blocks along a tile's side.
pub const BLOCK_SIDE: usize = 8;
/// Cells along a 6 m block's side (30).
pub const BLOCK_CELLS: usize = TILE_CELLS / BLOCK_SIDE;
/// Bounds cells along a side (16 × 16, each over 16 × 16 vertices).
pub const BOUND_CELLS: usize = 16;
/// The default channel value the record fills (0x7f).
pub const CHANNEL_FILL: u8 = 0x7f;
/// The constant written into a fully transparent part's vertex channel
/// (0.0).
pub const TRANSPARENT_MARK: f32 = 0.0;
/// A part is fully transparent below this alpha.
pub const TRANSPARENT_ALPHA: f32 = 0.01;

/// The terrain quality tables, by the original's texture-detail setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quality {
    /// The setting's two table values.
    pub first: i32,
    pub second: i32,
}

impl Quality {
    /// The table the constructor selects from the texture-detail setting.
    pub fn from_texture_detail(setting: i32) -> Self {
        match setting {
            s if (0..3).contains(&s) => Self {
                first: 3,
                second: 3,
            },
            3 => Self {
                first: 2,
                second: 3,
            },
            4 => Self {
                first: 1,
                second: 2,
            },
            _ => Self {
                first: 0,
                second: 1,
            },
        }
    }
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
    /// The detail texture's size (4096, or 8192 at high quality).
    pub detail_size: u32,
    /// The palette's scale: its configured value, or detail size / 2048.
    pub palette_scale: f32,
    /// The quality tables.
    pub quality: Quality,
    /// The detail texture's source scale (20 × the config word).
    pub detail_scale: f32,
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

    /// The constructor's values. `texture_detail` is the original's texture
    /// quality setting and `palette_scale` its configured palette scale
    /// (zero means derive it from the detail size).
    pub fn new(texture_detail: i32, palette_scale: f32) -> Self {
        let detail_size = if texture_detail < 3 { 4096 } else { 8192 };
        let palette_scale = if palette_scale == 0.0 {
            (detail_size >> 11) as f32
        } else {
            palette_scale
        };
        Self {
            tile_cm: TILE_CM,
            cell_cm: CELL_CM,
            cells: TILE_CELLS as i32,
            vertices: TILE_VERTICES as i32,
            apron: TILE_APRON as i32,
            vertex_count: (TILE_VERTICES * TILE_VERTICES) as i32,
            detail_size,
            palette_scale,
            quality: Quality::from_texture_detail(texture_detail),
            detail_scale: CELL_CM * 1.0,
        }
    }
}

/// One tile's record: its state, heights with the apron, centred vertices,
/// the vertex channel and the pending flag the worker wakes on.
#[derive(Clone, Debug, PartialEq)]
pub struct TileRecord {
    /// The record's state word (7 when raw data is present).
    pub state: u32,
    /// Height rows, `TILE_APRON²` samples, row-major.
    pub heights: Vec<f32>,
    /// The vertex grid, `TILE_VERTICES²` positions (cm, centred).
    pub vertices: Vec<Vec3>,
    /// The channel byte array (`TILE_CHANNEL²`, filled with 0x7f).
    pub channel: Vec<u8>,
    /// Byte stride of a vertex in the GPU buffer: 12, or 24 when the
    /// quality flag is set.
    pub vertex_stride: usize,
    /// The tile's worker state has a pending request.
    pub pending: bool,
}

impl TileRecord {
    /// Allocates the record as the manager does, including the ~130 KB the
    /// original's GetMem asks for.
    pub fn allocate(config: &ManagerConfig, high_quality: bool) -> Self {
        let apron = config.apron as usize;
        let side = config.vertices as usize;
        Self {
            state: 7,
            heights: vec![0.0; apron * apron],
            vertices: vec![Vec3::ZERO; side * side],
            channel: vec![CHANNEL_FILL; TILE_CHANNEL * TILE_CHANNEL],
            vertex_stride: if high_quality { 24 } else { 12 },
            pending: false,
        }
    }

    /// The record's own allocation size, bytes, as the original reserves
    /// it.
    pub fn allocation_bytes() -> usize {
        0x1_fae8
    }

    /// Centres the vertex grid: vertex (i, j) sits at
    /// `((i − 120)·20, 0, (j − 120)·20)` cm.
    pub fn centre_vertices(&mut self) {
        let cells = TILE_CELLS as f32;
        for j in 0..TILE_VERTICES {
            for i in 0..TILE_VERTICES {
                self.vertices[j * TILE_VERTICES + i] = Vec3::new(
                    (i as f32 - cells * 0.5) * CELL_CM,
                    0.0,
                    (j as f32 - cells * 0.5) * CELL_CM,
                );
            }
        }
    }

    /// Clears the tile's pending flag and wakes the worker (the original's
    /// finalise wake: slot 24).
    pub fn clear_pending(&mut self) {
        self.pending = false;
    }

    /// The per-vertex normals over the 241 × 241 grid, from the 3 × 3 Sobel
    /// filter on the apron heights. Vertex (i, j) reads the height at
    /// `(j + 1)·244 + (i + 1)`. A flat tile's normals are (0, 1, 0).
    pub fn normals(&self, spacing: f32) -> Vec<Vec3> {
        let h = |i: i32, j: i32| {
            let row = (j + 1).clamp(0, TILE_APRON as i32 - 1) as usize;
            let col = (i + 1).clamp(0, TILE_APRON as i32 - 1) as usize;
            self.heights[row * TILE_APRON + col]
        };
        let scale = 1.0 / (8.0 * spacing);
        let mut out = Vec::with_capacity(TILE_VERTICES * TILE_VERTICES);
        for j in 0..TILE_VERTICES as i32 {
            for i in 0..TILE_VERTICES as i32 {
                let gx = (h(i + 1, j - 1) + 2.0 * h(i + 1, j) + h(i + 1, j + 1))
                    - (h(i - 1, j - 1) + 2.0 * h(i - 1, j) + h(i - 1, j + 1));
                let gz = (h(i - 1, j + 1) + 2.0 * h(i, j + 1) + h(i + 1, j + 1))
                    - (h(i - 1, j - 1) + 2.0 * h(i, j - 1) + h(i + 1, j - 1));
                out.push(Vec3::new(-gx * scale, 1.0, -gz * scale).normalize());
            }
        }
        out
    }

    /// Finalises a loaded tile: its normals are computed, the pending flag
    /// is cleared and the tile is ready (the original's slot 23).
    pub fn finalise(&mut self, spacing: f32) -> Vec<Vec3> {
        let normals = self.normals(spacing);
        self.clear_pending();
        normals
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
        let tile = TILE_CM;
        let base = tile * 0.0625 - tile * 0.5;
        for row in 0..BLOCK_SIDE {
            for col in 0..BLOCK_SIDE {
                let x = (tile * 0.125 * col as f32 + base) / 100.0;
                let z = (tile * 0.125 * row as f32 + base) / 100.0;
                trees.push(
                    TerrainKdTree::build(BLOCK_CELLS, CELL_CM / 100.0)
                        .translated(Vec3::new(x, 0.0, z)),
                );
            }
        }
        Self { trees }
    }

    /// The translation of block (column, row), metres.
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

/// The height bounds of a tile, as the original walks them: bounds cell
/// (row, col) scans the 16 × 16 vertices from its own corner, and the
/// running min/max keeps the original's comparison order (a NaN leaves both
/// unchanged).
pub fn height_bounds(heights: &[f32], stride: usize) -> HeightBounds {
    let mut out = [[(f32::MAX, f32::MIN); BOUND_CELLS]; BOUND_CELLS];
    for row in 0..BOUND_CELLS {
        for col in 0..BOUND_CELLS {
            let (mut lo, mut hi) = {
                let first = heights[row * stride + col];
                (first, first)
            };
            for dy in 0..BOUND_CELLS {
                for dx in 0..BOUND_CELLS {
                    let at = (row + dy) * stride + col + dx;
                    let v = heights[at];
                    if lo <= v {
                        if hi <= v && v != hi {
                            hi = v;
                        }
                    } else {
                        lo = v;
                    }
                }
            }
            out[row][col] = (lo, hi);
        }
    }
    out
}

/// A 6 m block's vertical bounds from the height bounds: the square of
/// `2^(4 − level)` bounds cells starting at the block's corner, the radius
/// half the span, the centre moved by it, then one centimetre added to the
/// radius. `level` 0 covers the full 16 × 16 bounds.
pub fn block_bounds(bounds: &HeightBounds, level: u32) -> (f32, f32) {
    let size = (0x10usize >> level).max(1);
    let first = bounds[0][0];
    let (mut lo, mut hi) = (first.0, first.1);
    for line in bounds.iter().take(size.min(BOUND_CELLS)) {
        for &(a, b) in line.iter().take(size.min(BOUND_CELLS)) {
            if lo <= a {
                if hi <= b && b != hi {
                    hi = b;
                }
            } else {
                lo = a;
            }
        }
    }
    let radius = (hi - lo) * 0.5;
    (lo + radius, radius + 1.0)
}

/// The normal y the layered surface detail adds to a vertex:
/// `(byte − 127)·1.25·(2 − old_y) + sampled_y·height`. The byte is the
/// detail normal map's channel, `old_y` the geometric normal's y, `sampled_y`
/// the layer's normal y and `height` the layer's strength.
pub fn detail_normal_y(byte: u8, old_y: f32, sampled_y: f32, height: f32) -> f32 {
    (f32::from(byte) - 127.0) * 1.25 * (2.0 - old_y) + sampled_y * height
}

/// The opacity byte of a vertex from its normal y: the original's
/// `(160 − (15 − y)) · 0.00625`, clamped to [0, 1] and scaled to 0..255
/// with the engine's floor round.
pub fn opacity_byte(normal_y: f32) -> u8 {
    let v = (160.0 - (15.0 - normal_y)) * 0.006_25;
    if v < 0.0 {
        0
    } else if v > 1.0 {
        255
    } else {
        (v * 255.0).floor() as u8
    }
}

/// The opaque channel value a fully transparent part's vertices take: this
/// is the constant mark the original writes when a part's alpha is below
/// 0.01.
pub fn transparent_mark(alpha: f32) -> Option<f32> {
    (alpha < TRANSPARENT_ALPHA).then_some(TRANSPARENT_MARK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constructor_numbers() {
        let c = ManagerConfig::new(4, 0.0);
        assert_eq!(c.tile_cm, 4800.0);
        assert_eq!(c.cell_cm, 20.0);
        assert_eq!(c.cells, 240);
        assert_eq!(c.vertices, 241);
        assert_eq!(c.apron, 244);
        assert_eq!(c.vertex_count, 58081);
        assert_eq!(c.detail_size, 8192);
        assert_eq!(c.palette_scale, 4.0);
        assert_eq!(
            c.quality,
            Quality {
                first: 1,
                second: 2
            }
        );
        assert_eq!(
            ManagerConfig::new(0, 0.0).quality,
            Quality {
                first: 3,
                second: 3
            }
        );
        assert_eq!(ManagerConfig::new(3, 2.5).palette_scale, 2.5);
        assert_eq!(ManagerConfig::new(3, 0.0).detail_size, 8192);
        assert_eq!(c.inverse_tile(), 0.000_208_333_34);
        assert!(!c.ready());
    }

    #[test]
    fn the_tile_record_has_the_original_sizes() {
        let config = ManagerConfig::new(0, 0.0);
        let mut tile = TileRecord::allocate(&config, false);
        assert_eq!(tile.heights.len(), 244 * 244);
        assert_eq!(tile.vertices.len(), 241 * 241);
        assert_eq!(tile.channel.len(), 245 * 245);
        assert!(tile.channel.iter().all(|&b| b == 0x7f));
        assert_eq!(tile.vertex_stride, 12);
        assert_eq!(TileRecord::allocate(&config, true).vertex_stride, 24);
        assert_eq!(TileRecord::allocation_bytes(), 129_768);
        tile.centre_vertices();
        assert_eq!(tile.vertices[0], Vec3::new(-2400.0, 0.0, -2400.0));
        assert_eq!(
            tile.vertices[TILE_VERTICES * TILE_VERTICES - 1],
            Vec3::new(2400.0, 0.0, 2400.0)
        );
        assert_eq!(tile.state, 7);
    }

    #[test]
    fn a_flat_tile_has_up_normals_and_a_slope_tilts_them() {
        let config = ManagerConfig::new(0, 0.0);
        let mut tile = TileRecord::allocate(&config, false);
        let normals = tile.finalise(20.0);
        assert_eq!(normals.len(), 241 * 241);
        assert_eq!(normals[0], Vec3::Y);
        assert_eq!(normals[241 * 120 + 120], Vec3::Y);
        // A one-centimetre step per vertex along +x: the normal leans to
        // -x.
        for row in 0..244 {
            for col in 0..244 {
                tile.heights[row * 244 + col] = col as f32;
            }
        }
        let normals = tile.normals(20.0);
        let n = normals[241 * 120 + 120];
        assert!(n.x < 0.0 && n.y > 0.9, "{n:?}");
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
    fn height_bounds_scan_sixteen_vertices() {
        let heights: Vec<f32> = (0..244 * 244).map(|i| i as f32).collect();
        let bounds = height_bounds(&heights, 244);
        assert_eq!(bounds[0][0].0, 0.0);
        assert_eq!(bounds[0][0].1, 15.0 * 244.0 + 15.0);
        // The next cell starts one vertex along.
        assert_eq!(bounds[1][0].0, 244.0);
    }

    #[test]
    fn a_block_radius_and_centre() {
        let mut bounds = [[(0.0f32, 0.0f32); BOUND_CELLS]; BOUND_CELLS];
        bounds[0][0] = (-10.0, 30.0);
        bounds[15][15] = (0.0, 1000.0);
        let (centre, radius) = block_bounds(&bounds, 0);
        // min over the square is -10, max is 1000: centre 495, radius 506.
        assert_eq!(centre, 495.0);
        assert_eq!(radius, 506.0);
    }

    #[test]
    fn the_opacity_and_normal_channel_rules() {
        assert_eq!(opacity_byte(15.0), 255);
        assert_eq!(opacity_byte(-145.0), 0);
        assert_eq!(opacity_byte(0.0), 231);
        assert_eq!(detail_normal_y(127, 0.5, 1.0, 2.0), 2.0);
        assert_eq!(detail_normal_y(128, 0.5, 0.0, 2.0), 1.875);
        assert_eq!(transparent_mark(0.009), Some(0.0));
        assert_eq!(transparent_mark(0.01), None);
    }
}
