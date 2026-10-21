//! Where the ground is: a tile's heights and holes, and the original's
//! world-space height query.
//!
//! The query (`GroundTile` and [`TerrainGround`]) follows the original's
//! height slot: the tile is found by flooring the point's tile coordinate
//! (the engine's "round" helper rounds down), the 6 m block by flooring the
//! fraction times eight, and the surface by bilinearly interpolating the
//! four vertices around it in the original's order. A point is ground only
//! inside a block the level builds, outside the hole mask's 2 × 2 dilation,
//! and above −100 m: below that the original answers "no ground".

use std::sync::Arc;

use bevy::math::Vec2;
use bevy::prelude::Resource;

/// Vertices along a terrain tile's side.
pub const TILE_VERTICES: usize = exa_formats::terrain::TILE_VERTICES;
/// Blocks of 6 m along a terrain tile's side.
pub const TILE_BLOCKS: usize = 8;
/// Below this height (metres) the original reports no ground: the height
/// slot answers −1000 m for a hole and checks its result against −100 m.
pub const NO_GROUND: f32 = -100.0;
/// The original's tile-coordinate step, 1/4800 of a 48 m tile, in
/// centimetres (its literal single-precision constant).
const INV_TILE_CM: f32 = 0.000_208_333_34;

/// A terrain tile's ground as the original samples it: the vertex heights
/// (displaced by the height pass, as the game moves its vertices), the
/// holes, and which of its 6 m blocks the level builds.
#[derive(Debug, Clone)]
pub struct GroundTile {
    /// Tile grid coordinates.
    pub x: u32,
    pub y: u32,
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
        let side = TILE_VERTICES;
        let centre = terrain.tile_centre(tile);
        let half = (side - 1) as f32 * 0.5 * terrain.spacing;
        let holes = (0..side * side)
            .map(|v| tile.is_hole(v % side, v / side))
            .collect();
        Self {
            x: tile.x,
            y: tile.y,
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
        let side = TILE_VERTICES;
        let heights = (0..side * side)
            .map(|v| tile.height(v % side, v / side))
            .collect();
        Self::new(terrain, tile, heights)
    }

    /// Height (metres) at vertex coordinates (`u`, `v`), bilinearly between
    /// the four vertices around. The sums follow the original's order:
    /// `((h10 + h00) + h01) + h11`, each product times `u`/`v` in the order
    /// the code multiplies them.
    ///
    /// The original reads its hole mask a byte at a time: a vertex is a hole
    /// when it or its +X neighbour is set in its row or the row below, and
    /// the +X neighbour is not tested where it starts the next byte. Any of
    /// the four corners being a hole makes the point "no ground".
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
            h(i + 1, j) * fu * (1.0 - fv)
                + h(i, j) * (1.0 - fu) * (1.0 - fv)
                + h(i, j + 1) * (1.0 - fu) * fv
                + h(i + 1, j + 1) * fu * fv,
        )
    }
}

/// A level's terrain as a world-space height field, keyed by tile grid
/// coordinates. Built as the level's terrain tiles are, so gameplay can ask
/// where the ground is (the original's terrain height slot).
#[derive(Debug, Clone, Default, Resource)]
pub struct TerrainGround {
    pub width: u32,
    pub height: u32,
    /// Distance between vertices, metres.
    pub spacing: f32,
    /// Tile grid slots, row-major; holes for tiles the `.rft` leaves out.
    tiles: Vec<Option<Arc<GroundTile>>>,
}

impl TerrainGround {
    /// An empty field of `width` × `height` tile slots.
    pub fn new(width: u32, height: u32, spacing: f32) -> Self {
        Self {
            width,
            height,
            spacing,
            tiles: (0..(width as usize * height as usize))
                .map(|_| None)
                .collect(),
        }
    }

    /// Adds a tile's ground to its grid slot.
    pub fn insert(&mut self, tile: Arc<GroundTile>) -> bool {
        if tile.x >= self.width || tile.y >= self.height {
            return false;
        }
        let slot = tile.y as usize * self.width as usize + tile.x as usize;
        self.tiles[slot] = Some(tile);
        true
    }

    /// The tile at grid (`x`, `y`), if any.
    pub fn tile(&self, x: u32, y: u32) -> Option<&Arc<GroundTile>> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.tiles[y as usize * self.width as usize + x as usize].as_ref()
    }

    /// Ground height (metres) under world (`x`, `z`), or `None` where the
    /// original reports none.
    ///
    /// The tile coordinate is the engine's floor of `width/2 + x / tile
    /// side`; the block is the floor of the fraction times eight; the
    /// surface is [`GroundTile::height`] at the vertex coordinates. A point
    /// with no tile, no block there, a hole, or a height at or below
    /// [`NO_GROUND`] has no ground.
    pub fn height(&self, x: f32, z: f32) -> Option<f32> {
        if self.spacing <= 0.0 || self.width == 0 || self.height == 0 {
            return None;
        }
        // The original's arithmetic is in centimetres; keep its multiply
        // and its constant rather than dividing by the tile side.
        let fx = self.width as f32 * 0.5 + (x * 100.0) * INV_TILE_CM;
        let fz = self.height as f32 * 0.5 + (z * 100.0) * INV_TILE_CM;
        let (tx, tz) = (fx.floor(), fz.floor());
        // The original rejects indices outside 0 .. width × height.
        if tx < 0.0 || tz < 0.0 {
            return None;
        }
        let (tx, tz) = (tx as u32, tz as u32);
        let tile = self.tile(tx, tz)?;
        let (u, v) = ((fx - tx as f32) * 8.0, (fz - tz as f32) * 8.0);
        let (bx, by) = (u.floor(), v.floor());
        if bx < 0.0 || by < 0.0 {
            return None;
        }
        let (bx, by) = (bx as usize, by as usize);
        if bx >= TILE_BLOCKS || by >= TILE_BLOCKS || !tile.has_block(bx, by) {
            return None;
        }
        let cells = (TILE_VERTICES - 1) as f32;
        let h = tile.height((fx - tx as f32) * cells, (fz - tz as f32) * cells)?;
        (h > NO_GROUND).then_some(h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(x: u32, y: u32, heights: Vec<f32>) -> GroundTile {
        GroundTile {
            x,
            y,
            corner: Vec2::ZERO,
            spacing: 0.2,
            side: TILE_VERTICES,
            heights,
            holes: vec![false; TILE_VERTICES * TILE_VERTICES],
            blocks: GroundTile::ALL_BLOCKS,
        }
    }

    /// Heights `h(i, j) = c + i·di + j·dj`, so the four corners of a cell
    /// differ and the sum order shows.
    fn sloped(c: f32, di: f32, dj: f32) -> Vec<f32> {
        (0..TILE_VERTICES * TILE_VERTICES)
            .map(|v| c + (v % TILE_VERTICES) as f32 * di + (v / TILE_VERTICES) as f32 * dj)
            .collect()
    }

    #[test]
    fn the_height_sums_in_the_originals_order() {
        let t = tile(0, 0, sloped(1.0, 2.0, 4.0));
        // Vertex coordinates (1.5, 0.25): h00 = 3, h10 = 5, h01 = 7,
        // h11 = 9.
        let h = t.height(1.5, 0.25).unwrap();
        let fu = 0.5;
        let fv = 0.25;
        let expect = 5.0 * fu * (1.0 - fv)
            + 3.0 * (1.0 - fu) * (1.0 - fv)
            + 7.0 * (1.0 - fu) * fv
            + 9.0 * fu * fv;
        assert_eq!(h, expect);
        // At a vertex the four weights pick it alone.
        assert_eq!(t.height(1.0, 1.0), Some(7.0));
    }

    #[test]
    fn holes_and_the_two_by_two_dilation() {
        let mut t = tile(0, 0, sloped(0.0, 0.0, 0.0));
        let mut holes = vec![false; TILE_VERTICES * TILE_VERTICES];
        holes[5 * TILE_VERTICES + 5] = true;
        t.holes = holes;
        // A cell touching the hole at any corner has no ground.
        assert_eq!(t.height(4.5, 4.5), None);
        assert_eq!(t.height(5.5, 4.5), None);
        assert_eq!(t.height(4.5, 5.5), None);
        assert_eq!(t.height(5.5, 5.5), None);
        // The vertex just outside the byte's right edge does not test its
        // +X neighbour.
        let mut t = tile(0, 0, sloped(0.0, 0.0, 0.0));
        let mut holes = vec![false; TILE_VERTICES * TILE_VERTICES];
        holes[8 * TILE_VERTICES + 8] = true;
        t.holes = holes;
        assert_eq!(t.height(7.5, 8.5), Some(0.0));
        assert_eq!(t.height(8.5, 8.5), None);
    }

    #[test]
    fn the_query_finds_blocks_and_tiles() {
        let mut g = TerrainGround::new(2, 1, 0.2);
        assert!(g.insert(Arc::new(tile(0, 0, sloped(1.0, 0.0, 0.0)))));
        // Tile 0 is centred at −24 m; its centre vertex is (120, 120).
        assert_eq!(g.height(-24.0, 0.0), Some(1.0));
        // Tile 1 at +24 m.
        assert!(g.insert(Arc::new(tile(1, 0, sloped(2.0, 0.0, 0.0)))));
        assert_eq!(g.height(24.0, 0.0), Some(2.0));
        // Outside every tile.
        assert_eq!(g.height(-100.0, 0.0), None);
        // No ground below −100 m.
        let mut low = TerrainGround::new(1, 1, 0.2);
        low.insert(Arc::new(tile(
            0,
            0,
            vec![NO_GROUND; TILE_VERTICES * TILE_VERTICES],
        )));
        assert_eq!(low.height(0.0, 0.0), None);
    }

    #[test]
    fn absent_blocks_have_no_ground() {
        let mut t = tile(0, 0, sloped(1.0, 0.0, 0.0));
        t.blocks = [0; TILE_BLOCKS];
        t.blocks[4] = 1 << 4;
        let mut g = TerrainGround::new(1, 1, 0.2);
        g.insert(Arc::new(t));
        // The centre of the tile is block (4, 4), present.
        assert_eq!(g.height(0.0, 0.0), Some(1.0));
        // A point 7 m out is block (5, 4), absent.
        assert_eq!(g.height(7.0, 0.0), None);
        assert_eq!(g.height(-7.0, 0.0), None);
    }

    /// The real levels answer their tile centres: the query at a tile's
    /// centre is its (120, 120) vertex (skipped without the game).
    #[test]
    fn shipped_levels_answer_their_tile_centres() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_assets::PackKind::Resource).unwrap();
        let bytes = pack.get("exanima01.rft").unwrap();
        let terrain = exa_formats::terrain::Terrain::parse(bytes).unwrap();
        let mut g = TerrainGround::new(terrain.width, terrain.height, terrain.spacing);
        let mut tiles = Vec::new();
        for t in &terrain.tiles {
            let tile = Arc::new(GroundTile::stored(&terrain, t));
            g.insert(tile.clone());
            tiles.push((t, tile));
        }
        for (t, tile) in tiles {
            let c = terrain.tile_centre(t);
            // 10 cm inside the tile, off the block and vertex boundaries.
            let at = Vec2::new(c[0] + 0.1, c[1] + 0.1);
            let sample = (TILE_VERTICES - 1) as f32 / 2.0 + 0.5;
            let expected = tile.height(sample, sample).filter(|h| *h > NO_GROUND);
            let got = g.height(at.x, at.y);
            match (got, expected) {
                (Some(a), Some(b)) => assert!((a - b).abs() < 1e-3, "tile {} {}", t.x, t.y),
                (a, b) => assert_eq!(a.is_some(), b.is_some(), "tile {} {}", t.x, t.y),
            }
        }
    }
}
