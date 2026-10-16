//! What the navigation grid is built from: the level's size and height
//! reference, its terrain, the GI sample points of its static meshes and
//! the points generated for its tile-grid floors. Collected on the main
//! thread as shared copies; put in world space and voxelised by
//! [`NavSource::build`], on the async compute pool in game.

use std::sync::Arc;

use bevy::math::{Affine3A, DVec3, Vec2, Vec3, Vec3Swizzles};

use super::{CELL, CELLS_PER_ROOM, Columns, DEFAULT_ROOMS, DEPTH, FLOOR_LIFT, NavGrid};

/// Edge of a generated floor column, centimetres (the original's heightfield
/// generator at its constant P = 20: half of P).
pub const FLOOR_COLUMN_CM: i64 = 10;
/// Edge of a generated floor point's cube, centimetres (P).
pub const FLOOR_BLOCK_CM: i64 = 20;
/// Smallest upward component of a floor face's unit normal.
const UPWARD: f32 = 1.2e-7;
/// Blocks of 6 m along a terrain tile's side.
pub const TILE_BLOCKS: usize = crate::terrain::TileBlocks::SIDE;

/// A terrain tile's ground as the original samples it: the vertex heights
/// (displaced by the height pass, as the game moves its vertices), the
/// holes, and which of its 6 m blocks the level builds.
#[derive(Debug, Clone)]
pub struct GroundTile {
    /// World X and Z of vertex (0, 0), metres.
    pub corner: Vec2,
    /// Distance between vertices, metres.
    pub spacing: f32,
    /// Vertices along each side.
    pub side: usize,
    /// `side × side` heights, metres, rows along Z.
    pub heights: Vec<f32>,
    /// Vertices cut out of the surface, same layout.
    pub holes: Vec<bool>,
    /// The tile's [`TILE_BLOCKS`] × [`TILE_BLOCKS`] blocks that exist: bit
    /// `bx` of row `by` (along X and Z). Every block outside a tile-grid
    /// level; in one, only those the terrain builds.
    pub blocks: [u8; TILE_BLOCKS],
}

impl GroundTile {
    /// Every block of a tile.
    pub const ALL_BLOCKS: [u8; TILE_BLOCKS] = [u8::MAX; TILE_BLOCKS];

    /// The ground of `tile` with `heights` (displaced, `side × side`),
    /// every block present.
    pub fn new(
        terrain: &exa_formats::terrain::Terrain,
        tile: &exa_formats::terrain::TerrainTile,
        heights: Vec<f32>,
    ) -> Self {
        let side = exa_formats::terrain::TILE_VERTICES;
        let centre = terrain.tile_centre(tile);
        let half = (side - 1) as f32 * 0.5 * terrain.spacing;
        let holes = (0..side * side)
            .map(|v| tile.is_hole(v % side, v / side))
            .collect();
        Self {
            corner: Vec2::new(centre[0] - half, centre[1] - half),
            spacing: terrain.spacing,
            side,
            heights,
            holes,
            blocks: Self::ALL_BLOCKS,
        }
    }

    /// The same ground with only the blocks `present` (bit `bx` of row
    /// `by`), as a tile-grid level builds them.
    pub fn with_blocks(mut self, present: [u8; TILE_BLOCKS]) -> Self {
        self.blocks = present;
        self
    }

    /// Whether block (`bx`, `by`) exists.
    pub fn has_block(&self, bx: usize, by: usize) -> bool {
        self.blocks
            .get(by)
            .is_some_and(|row| bx < TILE_BLOCKS && row & (1 << bx) != 0)
    }

    /// The ground of `tile` at its stored heights (no height pass).
    pub fn stored(
        terrain: &exa_formats::terrain::Terrain,
        tile: &exa_formats::terrain::TerrainTile,
    ) -> Self {
        let side = exa_formats::terrain::TILE_VERTICES;
        let heights = (0..side * side)
            .map(|v| tile.height(v % side, v / side))
            .collect();
        Self::new(terrain, tile, heights)
    }

    /// Height (metres) at vertex coordinates (`u`, `v`), bilinear between
    /// the four vertices around; `None` when one of them is a hole as the
    /// original reads its hole mask (a byte at a time, so the right-hand
    /// vertex is not tested where it starts the next byte).
    pub fn height(&self, u: f32, v: f32) -> Option<f32> {
        if u < 0.0 || v < 0.0 {
            return None;
        }
        let (i, j) = (u as usize, v as usize);
        if i + 1 >= self.side || j + 1 >= self.side {
            return None;
        }
        let at = |i: usize, j: usize| j * self.side + i;
        let right = i % 8 != 7;
        let hole = |j: usize| self.holes[at(i, j)] || (right && self.holes[at(i + 1, j)]);
        if hole(j) || hole(j + 1) {
            return None;
        }
        let (fu, fv) = (u - i as f32, v - j as f32);
        let h = |i: usize, j: usize| self.heights[at(i, j)];
        Some(
            h(i + 1, j + 1) * fu * fv
                + h(i, j + 1) * (1.0 - fu) * fv
                + h(i + 1, j) * fu * (1.0 - fv)
                + h(i, j) * (1.0 - fu) * (1.0 - fv),
        )
    }
}

/// A floor mesh's positions and triangles with its world transform.
#[derive(Debug, Clone)]
pub struct FloorMesh {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub at: Affine3A,
}

impl FloorMesh {
    /// Its triangles in world space.
    fn triangles(&self) -> impl Iterator<Item = [Vec3; 3]> + '_ {
        let corners: Vec<Vec3> = self
            .positions
            .iter()
            .map(|&p| self.at.transform_point3(Vec3::from(p)))
            .collect();
        self.indices.as_chunks::<3>().0.iter().filter_map(move |t| {
            let c = |i: u32| corners.get(i as usize).copied();
            Some([c(t[0])?, c(t[1])?, c(t[2])?])
        })
    }
}

/// Everything a level's grid is built from.
#[derive(Debug, Clone, Default)]
pub struct NavSource {
    /// Cells along X and Z.
    pub width: usize,
    pub depth: usize,
    /// Height (metres) voxel 0 tops out at: the grid top less 25.5 m.
    pub base: f32,
    /// Static meshes' GI sample points (their own space, metres) with
    /// their world transforms.
    pub meshes: Vec<(Arc<Vec<Vec3>>, Affine3A)>,
    /// The tile grid's floor meshes.
    pub floors: Vec<FloorMesh>,
    /// The terrain tiles the level builds, with their blocks.
    pub ground: Vec<Arc<GroundTile>>,
}

/// Cells along X and Z of a level whose tile grid is `rooms` (columns,
/// rows): 20 per room, or 320 × 320 when it gives no width.
pub fn grid_size(rooms: Option<(u32, u32)>) -> (usize, usize) {
    match rooms {
        Some((w, h)) if w != 0 => (w as usize * CELLS_PER_ROOM, h as usize * CELLS_PER_ROOM),
        _ => (
            DEFAULT_ROOMS * CELLS_PER_ROOM,
            DEFAULT_ROOMS * CELLS_PER_ROOM,
        ),
    }
}

/// Height voxel 0 tops out at for a level whose grid top is `grid_top`
/// (metres).
pub fn grid_base(grid_top: f32) -> f32 {
    grid_top - DEPTH
}

impl NavSource {
    /// Builds the grid: terrain, then the meshes' points, then the floors'.
    pub fn build(&self) -> NavGrid {
        let mut columns = Columns::new(self.width, self.depth, self.base);
        for ground in &self.ground {
            fill_ground(&mut columns, ground, self.width, self.depth);
        }
        for (points, at) in &self.meshes {
            for &p in points.iter() {
                columns.stamp(at.transform_point3(p).as_dvec3(), 0.0);
            }
        }
        for p in floor_points(self.floors.iter().flat_map(FloorMesh::triangles)) {
            columns.stamp(p, FLOOR_LIFT);
        }
        NavGrid::from_columns(&columns)
    }
}

/// Fills the columns under a terrain tile a 6 m block at a time, as the
/// original walks the blocks its terrain built: each cell of a present
/// block from the bottom to the ground's height at its centre (no ground
/// in holes); absent blocks fill nothing.
fn fill_ground(columns: &mut Columns, ground: &GroundTile, width: usize, depth: usize) {
    // Vertices along a block (30), its edge (6 m) and its cells (20).
    let per_block = (ground.side - 1) / TILE_BLOCKS;
    let block = per_block as f32 * ground.spacing;
    let cells = (block / CELL).round() as usize;
    let step = CELL / ground.spacing;
    let first = |corner: f32, b: usize, n: usize| {
        ((corner + b as f32 * block) / CELL).round() as i64 + (n / 2) as i64
    };
    for by in 0..TILE_BLOCKS {
        for bx in 0..TILE_BLOCKS {
            if !ground.has_block(bx, by) {
                continue;
            }
            let x0 = first(ground.corner.x, bx, width);
            let z0 = first(ground.corner.y, by, depth);
            let (u0, v0) = ((bx * per_block) as f32, (by * per_block) as f32);
            for k in 0..cells {
                let z = z0 + k as i64;
                if z < 0 || z >= depth as i64 {
                    continue;
                }
                for i in 0..cells {
                    let x = x0 + i as i64;
                    if x < 0 || x >= width as i64 {
                        continue;
                    }
                    let u = (i as f32 + 0.5) * step + u0;
                    let v = (k as f32 + 0.5) * step + v0;
                    if let Some(h) = ground.height(u, v) {
                        columns.fill_ground(x as usize, z as usize, f64::from(h));
                    }
                }
            }
        }
    }
}

/// The points the original generates for tile-grid floors (world,
/// metres): the highest upward face over each 10 cm column marks the
/// 10 cm cell under it (the one whose top is the nearest 10 cm boundary),
/// and every 20 cm cube on the world's 20 cm lattice whose four columns
/// mark it gets a point at its centre.
pub fn floor_points(triangles: impl Iterator<Item = [Vec3; 3]>) -> Vec<DVec3> {
    let column = FLOOR_COLUMN_CM as f64;
    // Upward faces in centimetres, and the even-aligned columns they cover.
    let faces: Vec<[DVec3; 3]> = triangles
        .filter(|t| {
            let n = (t[1] - t[0]).cross(t[2] - t[0]);
            let len = n.length();
            len > 0.0 && n.y / len >= UPWARD
        })
        .map(|t| t.map(|p| super::centimetres(p.as_dvec3())))
        .collect();
    let first = |v: f64| (v / column).floor() as i64;
    let Some((lo, hi)) = faces.iter().flatten().fold(None, |acc, p| {
        let c = (first(p.x), first(p.z));
        Some(match acc {
            None => (c, c),
            Some(((ax, az), (bx, bz))) => ((c.0.min(ax), c.1.min(az)), (c.0.max(bx), c.1.max(bz))),
        })
    }) else {
        return Vec::new();
    };
    // Whole 2 × 2 blocks: start on an even column.
    let (x0, z0) = (lo.0.div_euclid(2) * 2, lo.1.div_euclid(2) * 2);
    let nx = ((hi.0 - x0 + 2) / 2 * 2) as usize;
    let nz = ((hi.1 - z0 + 2) / 2 * 2) as usize;
    let mut tops = vec![f32::NEG_INFINITY; nx * nz];
    for cm in &faces {
        let (a, b, c) = (cm[0].xz(), cm[1].xz(), cm[2].xz());
        let area = (b - a).perp_dot(c - a);
        if area == 0.0 {
            continue;
        }
        let lo = cm[0].min(cm[1]).min(cm[2]);
        let hi = cm[0].max(cm[1]).max(cm[2]);
        for kz in first(lo.z)..=first(hi.z) {
            for kx in first(lo.x)..=first(hi.x) {
                let q =
                    bevy::math::DVec2::new((kx as f64 + 0.5) * column, (kz as f64 + 0.5) * column);
                let wa = (b - q).perp_dot(c - q) / area;
                let wb = (c - q).perp_dot(a - q) / area;
                let wc = 1.0 - wa - wb;
                if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                    continue;
                }
                let h = (wa * cm[0].y + wb * cm[1].y + wc * cm[2].y) as f32;
                let top = &mut tops[(kz - z0) as usize * nx + (kx - x0) as usize];
                *top = top.max(h);
            }
        }
    }
    // The 20 cm band a column marks: its 10 cm cell is the one whose top is
    // the nearest 10 cm boundary to the surface.
    let band = |top: f32| {
        let cell = (f64::from(top) / column).round_ties_even() as i64 - 1;
        cell.div_euclid(2)
    };
    let block = FLOOR_BLOCK_CM as f64;
    let mut points = Vec::new();
    for bz in 0..nz / 2 {
        for bx in 0..nx / 2 {
            let top = |dx: usize, dz: usize| tops[(2 * bz + dz) * nx + 2 * bx + dx];
            let corners = [top(0, 0), top(1, 0), top(0, 1), top(1, 1)];
            if corners.iter().any(|t| !t.is_finite()) {
                continue;
            }
            let j = band(corners[0]);
            if corners.iter().all(|&t| band(t) == j) {
                points.push(
                    DVec3::new(
                        (x0 as f64 / 2.0 + bx as f64 + 0.5) * block,
                        (j as f64 + 0.5) * block,
                        (z0 as f64 / 2.0 + bz as f64 + 0.5) * block,
                    ) / 100.0,
                );
            }
        }
    }
    points
}
