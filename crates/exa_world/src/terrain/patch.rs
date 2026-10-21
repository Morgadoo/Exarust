//! The original's terrain grid patch: one triangle list shared by every
//! 6 m block of every tile.
//!
//! A tile is a 240 × 240 grid of 20 cm cells (241 × 241 vertices), split
//! into 8 × 8 blocks of 30 × 30 cells. The original builds one patch over a
//! block's cells, in serpentine order: cell row 0 runs along +X, row 1 back
//! along −X, and so on. Every quad splits along the diagonal from its low
//! corner (i, j) to its high corner (i + 1, j + 1); rows going back emit
//! their two triangles with the vertices in the other order. The patch's
//! indices are then flattened into the tile's 241-vertex rows, and each
//! block draws them from its own first vertex, `(by · 241 + bx) · 30`.
//!
//! A LOD shift `step` makes every quad span 2^step cells and rows, the
//! original's coarse patch; the terrain itself uses step 0 (the shift is part
//! of the patch builder, not of the terrain draw).

/// Tile side in 20 cm cells.
pub const TILE_CELLS: u32 = 240;
/// Tile side in vertices.
pub const TILE_VERTICES: u32 = TILE_CELLS + 1;
/// Cells along a 6 m block's side.
pub const BLOCK_CELLS: u32 = 30;

/// The two triangles of one quad, in the original's order.
///
/// `x0` and `x1` are the quad's low and high columns, `top` and `bottom` the
/// flat indices of its rows' first vertices. A row running along +X starts
/// at its low corner; a row running back starts at its high corner and winds
/// the same way, so both face +Y.
pub fn quad_indices(x0: u32, x1: u32, top: u32, bottom: u32, forward: bool) -> [u32; 6] {
    if forward {
        [
            x0 + top,
            x0 + bottom,
            x1 + bottom,
            x0 + top,
            x1 + bottom,
            x1 + top,
        ]
    } else {
        [
            x1 + top,
            x0 + top,
            x1 + bottom,
            x1 + bottom,
            x0 + top,
            x0 + bottom,
        ]
    }
}

/// A patch over `across` × `down` cells with the LOD shift `step`, as the
/// original's builder makes it: the vertices of the full grid, and a
/// serpentine triangle list whose indices use the original's row stride
/// `(across + 1) << step`. Like the original, it stores its vertex
/// coordinates and indices in 16 bits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridPatch {
    across: u32,
    down: u32,
    step: u32,
    vertices: Vec<[u16; 2]>,
    indices: Vec<u16>,
}

impl GridPatch {
    /// Builds the patch.
    ///
    /// The original emits a row, stops once that row's lower edge reaches
    /// the last vertex row, and otherwise turns back for the next: so the
    /// patch has ⌈down / 2^step⌉ rows (at least one), each of
    /// `across >> step` quads.
    pub fn new(across: u32, down: u32, step: u32) -> Self {
        let step = step.min(15);
        let mut vertices = Vec::with_capacity((across as usize + 1) * (down as usize + 1));
        for y in 0..=down {
            for x in 0..=across {
                vertices.push([x as u16, y as u16]);
            }
        }
        let cells = across >> step;
        let stride = (across + 1) << step;
        let last = (across + 1) * down;
        let column = |i: u32| i << step;
        let mut indices = Vec::with_capacity((across * down * 6) as usize);
        let mut emit = |q: [u32; 6]| indices.extend(q.map(|i| i as u16));
        let (mut top, mut bottom) = (0, stride);
        loop {
            for i in 0..cells {
                emit(quad_indices(column(i), column(i + 1), top, bottom, true));
            }
            if last <= bottom {
                break;
            }
            (top, bottom) = (bottom, bottom + stride);
            for i in (0..cells).rev() {
                emit(quad_indices(column(i), column(i + 1), top, bottom, false));
            }
            if last <= bottom {
                break;
            }
            (top, bottom) = (bottom, bottom + stride);
        }
        Self {
            across,
            down,
            step,
            vertices,
            indices,
        }
    }

    /// The patch every 6 m block shares: 30 × 30 cells at full detail.
    pub fn block() -> Self {
        Self::new(BLOCK_CELLS, BLOCK_CELLS, 0)
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
    pub fn vertices(&self) -> &[[u16; 2]] {
        &self.vertices
    }

    /// The serpentine triangle list, flat indices into the patch's own
    /// vertex grid.
    pub fn indices(&self) -> &[u16] {
        &self.indices
    }

    /// The triangle list as the original's flattener makes it: each patch
    /// index reads its vertex, and the vertex's column and row become
    /// `x · x_scale + y · y_scale`, kept in 16 bits. The terrain flattens
    /// with (1, 241), the tile's vertex row. An index past the grid (the
    /// partial last row of a coarse patch whose side does not divide) has
    /// no vertex to read and is kept as it is.
    pub fn flat_indices(&self, x_scale: u16, y_scale: u16) -> Vec<u16> {
        self.indices
            .iter()
            .map(|&i| match self.vertices.get(i as usize) {
                Some(&[x, y]) => x
                    .wrapping_mul(x_scale)
                    .wrapping_add(y.wrapping_mul(y_scale)),
                None => i,
            })
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
        assert_eq!(quad_indices(0, 1, 0, 241, true), [0, 241, 242, 0, 242, 1]);
        assert_eq!(
            quad_indices(5, 6, 482, 723, false),
            [6 + 482, 5 + 482, 6 + 723, 6 + 723, 5 + 482, 5 + 723]
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
            let q = quad_indices(x, x + 1, top, bottom, forward);
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
    fn the_block_patch_has_every_cell_once() {
        let p = GridPatch::block();
        assert_eq!(p.indices().len(), 30 * 30 * 6);
        assert_eq!(p.vertices().len(), 31 * 31);
        assert_eq!(p.indices()[..6], [0, 31, 32, 0, 32, 1]);
        // The last row (29) runs back, so it ends at its first quad.
        let last = p.indices().len();
        let tail = quad_indices(0, 1, 29 * 31, 30 * 31, false).map(|i| i as u16);
        assert_eq!(p.indices()[last - 6..], tail);
        // Every cell appears once; a quad's smallest index is its low corner.
        let mut seen = [false; 30 * 30];
        for q in p.indices().as_chunks::<6>().0 {
            let low = *q.iter().min().unwrap() as usize;
            let (x, y) = (low % 31, low / 31);
            assert!(!seen[y * 30 + x]);
            seen[y * 30 + x] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn the_lod_shift_widens_every_quad() {
        let p = GridPatch::new(4, 4, 1);
        assert_eq!(p.step(), 1);
        // Two quads a side, each two cells wide and two rows deep; the row
        // stride is two rows of the full grid, 10.
        assert_eq!(
            p.indices(),
            [
                0, 10, 12, 0, 12, 2, // columns 0..2, rows 0..2
                2, 12, 14, 2, 14, 4, // columns 2..4
                14, 12, 24, 24, 12, 22, // rows 2..4, backwards
                12, 10, 22, 22, 10, 20, //
            ]
        );
        // The stride is the full grid's row, so the coarse patch reads the
        // same vertices it names.
        assert_eq!(p.flat_indices(1, 5), p.indices());
        // A side that does not divide gives a last, partial row: 3 cells
        // down at step 1 is two rows, the second reaching past the grid.
        let p = GridPatch::new(2, 3, 1);
        assert_eq!(p.indices().len(), 2 * 6);
        assert_eq!(p.indices()[6..], [8, 6, 14, 14, 6, 12]);
        // A patch with no rows still emits its first.
        assert_eq!(GridPatch::new(2, 0, 0).indices().len(), 2 * 6);
    }

    #[test]
    fn flat_indices_rebase_the_vertex_grid() {
        let p = GridPatch::new(2, 2, 0);
        assert_eq!(
            p.flat_indices(1, 2),
            [
                0, 2, 3, 0, 3, 1, 1, 3, 4, 1, 4, 2, 4, 3, 6, 6, 3, 5, 3, 2, 5, 5, 2, 4
            ]
        );
        // The terrain's flattening puts a block's patch on the tile's rows:
        // the first quad of a block's second row starts at vertex 241 + 30.
        let f = GridPatch::block().flat_indices(1, TILE_VERTICES as u16);
        assert_eq!(f[..6], [0, 241, 242, 0, 242, 1]);
        assert_eq!(
            f[30 * 6..30 * 6 + 6],
            [241 + 30, 241 + 29, 482 + 30, 482 + 30, 241 + 29, 482 + 29]
        );
        // Indices are 16-bit, as the original stores them.
        let wide = GridPatch::new(1, 1, 0).flat_indices(1, u16::MAX);
        assert_eq!(wide[1], u16::MAX);
        assert_eq!(wide[2], 0);
    }
}
