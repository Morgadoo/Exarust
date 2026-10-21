//! The original's terrain grid patch: one triangle list shared by every
//! 6 m block of a tile.
//!
//! A tile is a 240 × 240 grid of 20 cm cells (241 × 241 vertices). The
//! original builds the patch once, in serpentine order: cell row 0 runs
//! along +X, row 1 back along −X, and so on. Every quad splits along the
//! diagonal from its low corner (i, j) to its high corner (i + 1, j + 1);
//! rows going back emit their two triangles with the vertices in the other
//! order. Each 6 m block is a 30 × 30 cell window of this patch, drawn from
//! the tile's shared vertex grid.
//!
//! A LOD shift `step` selects every 2^step-th cell and row, the original's
//! coarse patch; the terrain itself uses step 0 (the shift is part of the
//! patch builder, not of the terrain draw).

/// Tile side in 20 cm cells.
pub const TILE_CELLS: u32 = 240;
/// Tile side in vertices.
pub const TILE_VERTICES: u32 = TILE_CELLS + 1;
/// Cells along a 6 m block's side.
pub const BLOCK_CELLS: u32 = 30;

/// The two triangles of one quad, in the original's order.
///
/// `x` is the quad's low column in flat vertex indices (the column at
/// `x + top`); `top` and `bottom` are the flat indices of the quad rows'
/// first vertices. A row running along +X (even) starts at its low corner;
/// a row running back (odd) starts at its high corner and winds the same
/// way, so both parities face +Y.
pub fn quad_indices(x: u32, top: u32, bottom: u32, forward: bool) -> [u32; 6] {
    if forward {
        [
            x + top,
            x + bottom,
            x + 1 + bottom,
            x + top,
            x + 1 + bottom,
            x + 1 + top,
        ]
    } else {
        [
            x + 1 + top,
            x + top,
            x + 1 + bottom,
            x + 1 + bottom,
            x + top,
            x + bottom,
        ]
    }
}

/// A patch over `across` × `down` cells with the LOD shift `step`, as the
/// original's builder makes it: the vertices of the full grid, and a
/// serpentine triangle list whose indices use the original's row stride
/// `(across + 1) << step`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridPatch {
    across: u32,
    down: u32,
    step: u32,
    vertices: Vec<[u32; 2]>,
    indices: Vec<u32>,
}

impl GridPatch {
    /// Builds the patch. Large sides are clamped to the vertices a 16-bit
    /// index can reach, as the original stores its indices in 16 bits.
    pub fn new(across: u32, down: u32, step: u32) -> Self {
        let across = across.min(u16::MAX as u32 / 4);
        let down = down.min(u16::MAX as u32 / 4);
        let step = step.min(15);
        let mut vertices = Vec::with_capacity((across as usize + 1) * (down as usize + 1));
        for y in 0..=down {
            for x in 0..=across {
                vertices.push([x, y]);
            }
        }
        let cells_x = across >> step;
        let cells_y = down >> step;
        // The original advances a whole row of the full grid per shift.
        let stride = (across + 1) << step;
        let mut indices = Vec::with_capacity((cells_x * cells_y * 6) as usize);
        for row in 0..cells_y {
            let top = row * stride;
            let bottom = top + stride;
            let x = 1u32 << step;
            if row % 2 == 0 {
                for i in 0..cells_x {
                    indices.extend(quad_indices(i * x, top, bottom, true));
                }
            } else {
                for i in (0..cells_x).rev() {
                    indices.extend(quad_indices(i * x, top, bottom, false));
                }
            }
        }
        indices.retain(|&i| i <= u16::MAX as u32);
        Self {
            across,
            down,
            step,
            vertices,
            indices,
        }
    }

    /// The tile's patch: 240 × 240 cells at full detail.
    pub fn tile() -> Self {
        Self::new(TILE_CELLS, TILE_CELLS, 0)
    }

    pub fn across(&self) -> u32 {
        self.across
    }

    pub fn down(&self) -> u32 {
        self.down
    }

    pub fn step(&self) -> u32 {
        self.step
    }

    /// The full vertex grid, row-major, in cell steps.
    pub fn vertices(&self) -> &[[u32; 2]] {
        &self.vertices
    }

    /// The serpentine triangle list, flat indices as the original's patch
    /// builder makes them.
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// The triangle list as the original's patch flattener makes it: each
    /// patch index reads a vertex, and the vertex's row and column are
    /// scaled into the caller's vertex grid (`y · row stride + x · x scale`
    /// and `y · y scale`). Indices that reach outside the vertex list are
    /// dropped.
    pub fn flat_indices(&self, row_stride: u32, x_scale: u32, y_scale: u32) -> Vec<u32> {
        self.indices
            .iter()
            .filter_map(|&i| self.vertices.get(i as usize))
            .map(|v| v[1] * y_scale * row_stride + v[0] * x_scale)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quads_split_low_to_high_and_face_up() {
        // The low corner's diagonal is shared by the two triangles, and
        // both wind counter-clockwise seen from +Y (x along +X, y along +Z).
        assert_eq!(quad_indices(0, 0, 241, true), [0, 241, 242, 0, 242, 1]);
        assert_eq!(
            quad_indices(5, 482, 723, false),
            [
                5 + 1 + 482,
                5 + 482,
                5 + 1 + 723,
                5 + 1 + 723,
                5 + 482,
                5 + 723
            ]
        );
        let normal = |t: [u32; 3]| {
            let p = |i: u32| {
                let v = [i % 241, i / 241];
                [v[0] as f32, 0.0, v[1] as f32]
            };
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2])
        };
        for (x, top, bottom, forward) in [(0, 0, 241, true), (3, 482, 723, false)] {
            let q = quad_indices(x, top, bottom, forward);
            assert!(normal([q[0], q[1], q[2]]) > 0.0);
            assert!(normal([q[3], q[4], q[5]]) > 0.0);
            // The shared edge runs low corner → high corner.
            let diagonal = [x + top, x + 1 + bottom];
            assert!(q.contains(&diagonal[0]) && q.contains(&diagonal[1]));
        }
    }

    #[test]
    fn a_two_by_two_patch_is_serpentine() {
        let p = GridPatch::new(2, 2, 0);
        assert_eq!(p.vertices().len(), 9);
        // Row 0 runs along +X, row 1 back along −X. The row stride is 3.
        assert_eq!(
            p.indices(),
            [
                0, 3, 4, 0, 4, 1, // (0, 0)
                1, 4, 5, 1, 5, 2, // (1, 0)
                5, 4, 8, 8, 4, 7, // (1, 1), backwards: (1+1,1), (1,1), (1+1,2)
                4, 3, 7, 7, 3, 6, // (0, 1), backwards
            ]
        );
    }

    #[test]
    fn the_tile_patch_has_every_cell_once() {
        let p = GridPatch::tile();
        assert_eq!(p.indices().len(), (240 * 240 * 6) as usize);
        assert_eq!(p.vertices().len(), 241 * 241);
        assert_eq!(p.indices()[..6], [0, 241, 242, 0, 242, 1]);
        let last = p.indices().len();
        assert_eq!(
            p.indices()[last - 6..],
            quad_indices(0, 239 * 241, 240 * 241, false)
        );
        // Every cell appears once: a forward row's low corner is its first
        // index, a backward row's is its second.
        let mut seen = vec![false; 240 * 240];
        for (k, q) in p.indices().as_chunks::<6>().0.iter().enumerate() {
            let forward = (k / 240) % 2 == 0;
            let low = if forward { q[0] } else { q[1] };
            let (x, y) = ((low % 241) as usize, (low / 241) as usize);
            assert!(!seen[y * 240 + x]);
            seen[y * 240 + x] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn the_lod_shift_takes_every_other_cell_and_row() {
        let p = GridPatch::new(4, 4, 1);
        assert_eq!(p.step(), 1);
        // Two cells a side, rows and columns two apart, row stride 10.
        assert_eq!(
            p.indices(),
            [
                0, 10, 11, 0, 11, 1, // (0, 0)
                2, 12, 13, 2, 13, 3, // (2, 0)
                13, 12, 23, 23, 12, 22, // (2, 2), backwards
                11, 10, 21, 21, 10, 20, // (0, 2), backwards
            ]
        );
        // The stride is the full grid's row, doubled, so the coarse patch
        // reads the same vertices it names.
        assert_eq!(p.flat_indices(5, 1, 1), p.indices());
        assert_eq!(GridPatch::tile().step(), 0);
    }

    #[test]
    fn flat_indices_rebase_the_vertex_grid() {
        let p = GridPatch::new(2, 2, 0);
        let f = p.flat_indices(2, 1, 1);
        assert_eq!(
            f,
            [
                0, 2, 3, 0, 3, 1, 1, 3, 4, 1, 4, 2, 4, 3, 6, 6, 3, 5, 3, 2, 5, 5, 2, 4
            ]
        );
    }
}
