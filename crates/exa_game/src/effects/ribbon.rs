//! The force effects' ribbon meshes: the vertex, index, texture-coordinate
//! and colour arrays the original's two quad emitters build, and the shared
//! strip bookkeeping.
//!
//! A ribbon is a strip of quads. Every emitter call adds two vertices and
//! two triangles; the triangles take the two new vertices plus the two the
//! previous call added, so consecutive calls share their edge:
//!
//! ```text
//! call n:  (base, base+1, base+2) and (base+2, base+1, base+3), base += 2
//! ```
//!
//! The first call of a strip therefore refers forward to its own two
//! vertices, and a closing call with the last point twice ends the strip.
//! The blast's strips chain across chunks: the first call of a chunk reuses
//! the two vertices of the previous chunk's closing quad, and the draw
//! drops that quad's two triangles. The torrent closes each stream instead.
//!
//! - **Blast quad** ([`RibbonMesh::push_blast_quad`]): the two vertices sit
//!   at `a ± d`, where `d` is `(b − previous a) × (0, 0, 1)` scaled by
//!   `half/|d|` — the strip widens across the view plane and the camera's
//!   axis is the screen normal. The first texture coordinate is `(0, v)`
//!   and `(1, v)`, the second the chunk's `(s, row)`.
//! - **Torrent quad** ([`RibbonMesh::push_torrent_quad`]): the two vertices
//!   sit at `a ± normal·width`, the first texture coordinate `(0, w)` and
//!   `(1, w)`, the second `(weight, v)`; `v` accumulates the distance
//!   along the strip by 0.005 while the torrent charges and 0.0003 while
//!   it runs.
//!
//! Evidence: the blast's quad emitter and the torrent's stream ribbon,
//! read from `Exanima.exe` (the divisions and the sum orders are the
//! instructions').

use bevy::prelude::*;

/// A ribbon mesh: centimetres, two texture-coordinate sets, colour bytes
/// and 16-bit triangles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RibbonMesh {
    pub positions: Vec<Vec3>,
    /// The first coordinate set: across the strip, then along it.
    pub uvs: Vec<[f32; 2]>,
    /// The second coordinate set: the effect's texture row.
    pub uvs2: Vec<[f32; 2]>,
    pub colours: Vec<[u8; 4]>,
    pub indices: Vec<u16>,
}

impl RibbonMesh {
    /// The ribbon vertex count.
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Whether nothing was emitted.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// The base vertex pair index of the next call: the last two vertices,
    /// or the first pair on an empty mesh.
    fn base(&self) -> u16 {
        self.positions.len() as u16
    }

    /// Appends the two triangles a strip call makes, before its vertices.
    fn push_triangles(&mut self) {
        let base = self.base();
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }

    /// One blast quad: the point `a` widened by `half` across
    /// `(b − previous_a) × (0, 0, 1)`, its texture `v` and the `(s, row)`
    /// pair shared by the whole chunk. `previous_a` becomes `a`, so an inner
    /// call widens across the central difference of its neighbours.
    #[allow(clippy::too_many_arguments)]
    pub fn push_blast_quad(
        &mut self,
        a: Vec3,
        b: Vec3,
        previous_a: &mut Vec3,
        half: f32,
        colour: [u8; 4],
        v: f32,
        uv2: [f32; 2],
    ) {
        self.push_triangles();
        let delta = b - *previous_a;
        let cross = Vec3::new(delta.y, -delta.x, 0.0);
        let length = (cross.x * cross.x + cross.y * cross.y) + cross.z * cross.z;
        let length = length.sqrt();
        // Only coincident samples give a zero cross; the original then
        // divides by zero. Keep the pair at the point so the mesh stays
        // finite. [deviation, unreachable for a moving trail]
        let dir = if length > 0.0 {
            cross * (half / length)
        } else {
            Vec3::ZERO
        };
        self.positions.push(a - dir);
        self.positions.push(a + dir);
        self.uvs.push([0.0, v]);
        self.uvs.push([1.0, v]);
        self.uvs2.push(uv2);
        self.uvs2.push(uv2);
        self.colours.push(colour);
        self.colours.push(colour);
        *previous_a = a;
    }

    /// One torrent quad: the point `a` widened by `normal·width`, the first
    /// texture coordinate `(0, w)`/`(1, w)` and the second `(weight, v)`.
    #[allow(clippy::too_many_arguments)]
    pub fn push_torrent_quad(
        &mut self,
        a: Vec3,
        normal: Vec3,
        width: f32,
        colour: [u8; 4],
        uv_v: f32,
        weight: f32,
        v: f32,
    ) {
        self.push_triangles();
        let dir = normal * width;
        self.positions.push(a - dir);
        self.positions.push(a + dir);
        self.uvs.push([0.0, uv_v]);
        self.uvs.push([1.0, uv_v]);
        self.uvs2.push([weight, v]);
        self.uvs2.push([weight, v]);
        self.colours.push(colour);
        self.colours.push(colour);
    }

    /// Drops the last `count` triangles, as a chunk's chained strip does
    /// with the previous chunk's closing quad.
    pub fn drop_triangles(&mut self, count: usize) {
        let keep = self.indices.len().saturating_sub(count * 3);
        self.indices.truncate(keep);
    }

    /// Appends `other` after this mesh, `base` added to every index and
    /// `map` applied to every position (view space to world, or the
    /// reverse).
    pub fn append(&mut self, other: &Self, base: i32, map: impl Fn(Vec3) -> Vec3) {
        self.positions
            .extend(other.positions.iter().map(|&p| map(p)));
        self.uvs.extend_from_slice(&other.uvs);
        self.uvs2.extend_from_slice(&other.uvs2);
        self.colours.extend_from_slice(&other.colours);
        self.indices
            .extend(other.indices.iter().map(|&i| (i32::from(i) + base) as u16));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blast_quad_widens_across_the_view_plane() {
        let mut mesh = RibbonMesh::default();
        let mut last = Vec3::ZERO;
        mesh.push_blast_quad(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            &mut last,
            2.0,
            [1, 2, 3, 4],
            0.5,
            [0.25, 0.75],
        );
        // b − last is +10 along x, so its cross with +z is −10 along y and
        // the widening points down: the first vertex is a minus it.
        assert_eq!(
            mesh.positions,
            [Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, -2.0, 0.0)]
        );
        assert_eq!(mesh.uvs, [[0.0, 0.5], [1.0, 0.5]]);
        assert_eq!(mesh.uvs2, [[0.25, 0.75]; 2]);
        assert_eq!(mesh.colours, [[1, 2, 3, 4]; 2]);
        assert_eq!(mesh.indices, [0, 1, 2, 2, 1, 3]);
        assert_eq!(last, Vec3::ZERO);
    }

    #[test]
    fn an_inner_blast_quad_widens_across_its_neighbours() {
        let mut mesh = RibbonMesh::default();
        let mut last = Vec3::new(-10.0, 0.0, 0.0);
        let first = Vec3::ZERO;
        let second = Vec3::new(10.0, 0.0, 0.0);
        let third = Vec3::new(6.0, 8.0, 0.0);
        mesh.push_blast_quad(first, second, &mut last, 5.0, [0; 4], 0.0, [0.0; 2]);
        mesh.push_blast_quad(second, third, &mut last, 5.0, [0; 4], 0.0, [0.0; 2]);
        // The second call's delta is the third point less the first, (6, 8),
        // whose cross with +z is (8, −6): five along it is (4, −3).
        assert_eq!(
            &mesh.positions[2..],
            &[Vec3::new(6.0, 3.0, 0.0), Vec3::new(14.0, -3.0, 0.0)]
        );
        assert_eq!(last, second);
    }

    #[test]
    fn a_torrent_quad_uses_its_normal_and_width() {
        let mut mesh = RibbonMesh::default();
        mesh.push_torrent_quad(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::Y,
            3.0,
            [9, 8, 7, 6],
            0.4,
            0.2,
            1.5,
        );
        assert_eq!(
            mesh.positions,
            [Vec3::new(0.0, -3.0, 5.0), Vec3::new(0.0, 3.0, 5.0)]
        );
        assert_eq!(mesh.uvs, [[0.0, 0.4], [1.0, 0.4]]);
        assert_eq!(mesh.uvs2, [[0.2, 1.5]; 2]);
        assert_eq!(mesh.indices, [0, 1, 2, 2, 1, 3]);
    }

    #[test]
    fn consecutive_calls_share_their_edge_and_the_last_can_be_dropped() {
        let mut mesh = RibbonMesh::default();
        let mut last = Vec3::ZERO;
        mesh.push_blast_quad(Vec3::ZERO, Vec3::X, &mut last, 1.0, [0; 4], 1.0, [0.0; 2]);
        mesh.push_blast_quad(
            Vec3::X,
            Vec3::X * 2.0,
            &mut last,
            1.0,
            [0; 4],
            1.0,
            [0.0; 2],
        );
        assert_eq!(mesh.positions.len(), 4);
        // The second call's base is the first call's pair.
        assert_eq!(&mesh.indices[6..], &[2, 3, 4, 4, 3, 5]);
        mesh.drop_triangles(2);
        assert_eq!(mesh.indices.len(), 6);
    }
}
