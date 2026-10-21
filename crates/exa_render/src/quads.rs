//! The original's immediate-mode 2-D batcher in pixel coordinates, the data
//! path of its GL immediate-draw helpers.
//!
//! `begin` takes the viewport's pixel rectangle and computes `1/width` and
//! `1/height`; every vertex is scaled by them with the y axis flipped:
//! `((x + tx)·sx, (ty − y)·sy)`, where `tx` is the current translation's x
//! and `ty` the viewport height less its y, both taken when the batch
//! begins. The translation is cumulative: each `translate` adds to it and
//! saves it on a 16-level stack, `pop` restores the previous level's. A
//! colour is RGBA bytes; a texture coordinate (or four at once) belongs to
//! the next vertex slot. `flush` draws every four vertices as one quad, two
//! triangles from the index template `(0, 1, 2, 0, 2, 3)`; the line draw
//! takes the same vertices in pairs.
//!
//! The vertex program the batch is drawn with is one of six built-ins,
//! picked by the vertex-format word ([`builtin_for`], [`BUILTIN_SHADERS`]),
//! with the projection [`NORMALIZED_PROJECTION`] for these 0..1 vertices or
//! [`QuadBatcher::projection`] for raw pixel vertices. Exarust keeps the
//! arithmetic as data and turns a batch into a Bevy `Mesh`; nothing draws
//! one yet.
//!
//! Rules: `docs/RENDERING.md` (pixel-space batcher).

use bevy::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexFormat};
use bevy::prelude::*;

/// The translation stack's depth.
pub const TRANSLATION_LEVELS: usize = 16;
/// How many vertices one quad takes.
pub const QUAD_VERTICES: usize = 4;
/// A quad's two triangles over its four vertices.
pub const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];
/// The texture coordinates a quad gets by default, vertex by vertex.
pub const QUAD_TEXCOORDS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

/// The projection the built-in programs take for normalised vertices: x
/// and y from 0..1 to −1..1, depth scaled by −0.001.
pub const NORMALIZED_PROJECTION: Mat4 = Mat4::from_cols_array(&[
    2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, -0.001, 0.0, -1.0, -1.0, -0.001, 1.0,
]);

/// The vertex-colour attribute a batch carries: RGBA bytes.
pub const COLOR: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Color", 1, VertexFormat::Unorm8x4);

/// A batch's vertices: positions (z 0), two texture-coordinate channels
/// and RGBA bytes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuadBatch {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// The second texture-coordinate channel.
    pub uv2s: Vec<[f32; 2]>,
    pub colours: Vec<[u8; 4]>,
    /// The quad indices ([`QuadBatcher::flush`]); empty for a line batch.
    pub indices: Vec<u16>,
}

impl QuadBatch {
    /// The number of vertices.
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// The batch as a Bevy mesh with `projection` applied to its positions
    /// on the CPU: a triangle list over its indices, or a line list when it
    /// has none.
    pub fn mesh_with(&self, projection: Mat4) -> Mesh {
        let topology = if self.indices.is_empty() {
            PrimitiveTopology::LineList
        } else {
            PrimitiveTopology::TriangleList
        };
        let mut mesh = Mesh::new(topology, bevy::asset::RenderAssetUsages::RENDER_WORLD);
        let projected: Vec<[f32; 3]> = self
            .positions
            .iter()
            .map(|p| {
                let v = projection * Vec4::new(p[0], p[1], p[2], 1.0);
                [v.x, v.y, v.z]
            })
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, projected);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, self.uv2s.clone());
        mesh.insert_attribute(
            COLOR,
            bevy::mesh::VertexAttributeValues::Unorm8x4(self.colours.clone()),
        );
        if !self.indices.is_empty() {
            mesh.insert_indices(Indices::U16(self.indices.clone()));
        }
        mesh
    }
}

/// The batcher's state.
#[derive(Debug, Clone, PartialEq)]
pub struct QuadBatcher {
    /// `1 / viewport width` and `1 / viewport height`.
    pub scale_x: f32,
    pub scale_y: f32,
    /// The current translation and the saved one of every level.
    current: Vec2,
    translation: [Vec2; TRANSLATION_LEVELS],
    level: usize,
    /// The `(tx, ty)` the batch's vertices use, taken at `begin`.
    offset: Vec2,
    /// The current colour (RGBA bytes).
    pub colour: [u8; 4],
    batch: QuadBatch,
}

impl Default for QuadBatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl QuadBatcher {
    pub fn new() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            current: Vec2::ZERO,
            translation: [Vec2::ZERO; TRANSLATION_LEVELS],
            level: 0,
            offset: Vec2::ZERO,
            colour: [255; 4],
            batch: QuadBatch::default(),
        }
    }

    /// Starts a batch for the viewport `(left, top)..(right, bottom)` in
    /// pixels: `sx = 1/(right − left)`, `sy = 1/(bottom − top)`, and the
    /// vertices' offset from the current translation.
    pub fn begin(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        let width = (i64::from(right) - i64::from(left)) as f32;
        let height = (i64::from(bottom) - i64::from(top)) as f32;
        self.scale_x = 1.0 / width;
        self.scale_y = 1.0 / height;
        self.offset = Vec2::new(self.current.x, height - self.current.y);
        self.batch = QuadBatch::default();
    }

    /// Adds `(dx, dy)` to the current translation and saves it one level
    /// up; at the top level the last slot is overwritten.
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.level = (self.level + 1).min(TRANSLATION_LEVELS - 1);
        self.current.x += dx;
        self.current.y += dy;
        self.translation[self.level] = self.current;
    }

    /// Goes down a level (not below zero) and takes its translation.
    pub fn pop(&mut self) {
        self.level = self.level.saturating_sub(1);
        self.current = self.translation[self.level];
    }

    /// Clears the current translation and the level.
    pub fn reset(&mut self) {
        self.current = Vec2::ZERO;
        self.level = 0;
    }

    /// The current translation.
    pub fn translation(&self) -> Vec2 {
        self.current
    }

    /// Sets the current colour, RGBA bytes.
    pub fn set_colour(&mut self, colour: [u8; 4]) {
        self.colour = colour;
    }

    /// Sets the current colour from an RGB triple and an alpha byte.
    pub fn set_rgb(&mut self, rgb: [u8; 3], alpha: u8) {
        self.colour = [rgb[0], rgb[1], rgb[2], alpha];
    }

    /// The next vertex slot's texture coordinate.
    pub fn texcoord(&mut self, u: f32, v: f32) {
        let at = self.batch.positions.len();
        if self.batch.uvs.len() <= at {
            self.batch.uvs.resize(at + 1, [0.0; 2]);
        }
        self.batch.uvs[at] = [u, v];
    }

    /// The texture coordinates of the next four vertex slots.
    pub fn quad_texcoords(&mut self, uvs: [[f32; 2]; 4]) {
        let at = self.batch.positions.len();
        if self.batch.uvs.len() < at + 4 {
            self.batch.uvs.resize(at + 4, [0.0; 2]);
        }
        self.batch.uvs[at..at + 4].copy_from_slice(&uvs);
    }

    /// The next four vertex slots get [`QUAD_TEXCOORDS`].
    pub fn quad_texcoords_default(&mut self) {
        self.quad_texcoords(QUAD_TEXCOORDS);
    }

    /// The next vertex slot's second texture coordinate.
    pub fn texcoord2(&mut self, u: f32, v: f32) {
        let at = self.batch.positions.len();
        if self.batch.uv2s.len() <= at {
            self.batch.uv2s.resize(at + 1, [0.0; 2]);
        }
        self.batch.uv2s[at] = [u, v];
    }

    /// The projection for raw pixel vertices ([`QuadBatcher::vertex_raw`]):
    /// [`NORMALIZED_PROJECTION`] with x scaled by `sx`, y by `−sy`, and
    /// translated by `tx·sx·2 − 1` and `1 − ty·sy·2` for the current
    /// translation.
    pub fn projection(&self) -> Mat4 {
        let mut m = NORMALIZED_PROJECTION.to_cols_array();
        m[0] *= self.scale_x;
        m[5] *= -self.scale_y;
        m[12] = self.current.x * self.scale_x * 2.0 - 1.0;
        m[13] = 1.0 - self.current.y * self.scale_y * 2.0;
        Mat4::from_cols_array(&m)
    }

    fn push(&mut self, position: [f32; 3]) {
        let at = self.batch.positions.len();
        self.batch.positions.push(position);
        if self.batch.uvs.len() <= at {
            self.batch.uvs.push([0.0; 2]);
        }
        if self.batch.uv2s.len() <= at {
            self.batch.uv2s.push([0.0; 2]);
        }
        self.batch.colours.push(self.colour);
    }

    /// Adds a vertex in pixel coordinates, normalised to 0..1 with y
    /// flipped.
    pub fn vertex(&mut self, x: f32, y: f32) {
        self.push([
            (x + self.offset.x) * self.scale_x,
            (self.offset.y - y) * self.scale_y,
            0.0,
        ]);
    }

    /// Adds a vertex as given (for [`QuadBatcher::projection`]).
    pub fn vertex_raw(&mut self, x: f32, y: f32) {
        self.push([x, y, 0.0]);
    }

    /// The vertices added so far.
    pub fn len(&self) -> usize {
        self.batch.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.batch.positions.is_empty()
    }

    /// The batch as quads: every whole four vertices become two triangles
    /// from [`QUAD_INDICES`]; a trailing partial quad is not drawn. The
    /// batcher's vertices are taken.
    pub fn flush(&mut self) -> QuadBatch {
        let mut batch = std::mem::take(&mut self.batch);
        let quads = batch.positions.len() / QUAD_VERTICES;
        let whole = quads * QUAD_VERTICES;
        batch.positions.truncate(whole);
        batch.uvs.truncate(whole);
        batch.uv2s.truncate(whole);
        batch.colours.truncate(whole);
        batch.indices = (0..quads)
            .flat_map(|q| QUAD_INDICES.map(|i| i + (q * QUAD_VERTICES) as u16))
            .collect();
        batch
    }

    /// The batch as lines: vertex pairs, a trailing odd vertex not drawn.
    pub fn flush_lines(&mut self) -> QuadBatch {
        let mut batch = std::mem::take(&mut self.batch);
        let whole = batch.positions.len() & !1;
        batch.positions.truncate(whole);
        batch.uvs.truncate(whole);
        batch.uv2s.truncate(whole);
        batch.colours.truncate(whole);
        batch
    }
}

/// The six built-in immediate-draw vertex programs, as the original stores
/// them (CRLF line endings), indexed as [`builtin_for`] picks them.
pub const BUILTIN_SHADERS: [&str; 6] = [
    "in vec3 vPos;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\n}",
    "in vec3 vPos;\r\nin vec2 vTex;\r\nout vec2 vrTex;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrTex = vTex;\r\n}",
    "in vec3 vPos;\r\nin vec4 vCol;\r\nin vec2 vTex;\r\nout vec4 vrCol;\r\nout vec2 vrTex;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrTex = vTex;\r\nvrCol = vCol * 0.003921568;\r\n}",
    "in vec3 vPos;\r\nin vec4 vCol;\r\nout vec4 vrCol;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrCol = vCol * 0.003921568;\r\n}",
    "in vec3 vPos;\r\nin vec2 vTex;\r\nin vec2 vTx2;\r\nout vec2 vrTex;\r\nout vec2 vrTx2;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrTex = vTex;\r\nvrTx2 = vTx2;\r\n}",
    "in vec3 vPos;\r\nin vec2 vTex;\r\nin vec2 vTx2;\r\nin vec4 vCol;\r\nout vec2 vrTex;\r\nout vec2 vrTx2;\r\nout vec4 vrCol;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrTex = vTex;\r\nvrTx2 = vTx2;\r\nvrCol = vCol * 0.003921568;\r\n}",
];

/// The built-in program a vertex-format word picks: bit `0x20` (second
/// texture channel) first, then `0x4` (colour), then `0x10` (texture),
/// tested in the original's order.
pub fn builtin_for(word: u32) -> usize {
    if word & 0x20 == 0 {
        if word & 4 == 0 {
            if word & 0x10 == 0 { 0 } else { 1 }
        } else if word & 0x10 == 0 {
            3
        } else {
            2
        }
    } else if word & 4 == 0 {
        4
    } else {
        5
    }
}

/// The attribute locations the original binds before linking, with the
/// vertex-format bit that enables each (0 for the position, always bound).
pub const ATTRIBUTE_LOCATIONS: [(&str, u32, u32); 7] = [
    ("vPos", 0, 0),
    ("vTex", 1, 0x10),
    ("vTx2", 4, 0x20),
    ("vNrm", 2, 0x2),
    ("vCol", 3, 0x4),
    ("vTng", 6, 0x8),
    ("vDlt", 7, 0x40),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_map_to_the_unit_square_with_the_y_axis_flipped() {
        let mut b = QuadBatcher::new();
        b.begin(0, 0, 800, 600);
        b.vertex(0.0, 0.0);
        b.vertex(800.0, 0.0);
        b.vertex(800.0, 600.0);
        b.vertex(0.0, 600.0);
        let batch = b.flush();
        assert_eq!(batch.positions[0], [0.0, 1.0, 0.0]);
        assert_eq!(batch.positions[1], [1.0, 1.0, 0.0]);
        assert_eq!(batch.positions[2], [1.0, 0.0, 0.0]);
        assert_eq!(batch.positions[3], [0.0, 0.0, 0.0]);
        // The normalised projection puts the top-left pixel at (−1, 1).
        let p = NORMALIZED_PROJECTION * Vec4::new(0.0, 1.0, 0.0, 1.0);
        assert_eq!((p.x, p.y, p.z), (-1.0, 1.0, -0.001));
    }

    #[test]
    fn translations_add_up_and_the_batch_takes_them_at_begin() {
        let mut b = QuadBatcher::new();
        b.translate(10.0, 20.0);
        b.begin(100, 50, 500, 250);
        b.vertex(100.0, 50.0);
        // A translation after begin waits for the next batch.
        b.translate(100.0, 100.0);
        assert_eq!(b.translation(), Vec2::new(110.0, 120.0));
        b.vertex(100.0, 50.0);
        let batch = b.flush_lines();
        // (100 + 10)/400 = 0.275; (200 − 20 − 50)/200 = 0.65.
        assert_eq!(batch.positions[0], [0.275, 0.65, 0.0]);
        assert_eq!(batch.positions[1], [0.275, 0.65, 0.0]);
        b.pop();
        assert_eq!(b.translation(), Vec2::new(10.0, 20.0));
        b.pop();
        assert_eq!(b.translation(), Vec2::ZERO);
        b.pop();
        assert_eq!(b.translation(), Vec2::ZERO);
        b.begin(100, 50, 500, 250);
        b.vertex(100.0, 50.0);
        b.vertex(100.0, 50.0);
        assert_eq!(b.flush_lines().positions[0], [0.25, 0.75, 0.0]);
        // Past the top level the last slot keeps taking the sum.
        for _ in 0..20 {
            b.translate(1.0, 1.0);
        }
        assert_eq!(b.translation(), Vec2::new(20.0, 20.0));
        b.pop();
        assert_eq!(b.translation(), Vec2::new(14.0, 14.0));
        b.reset();
        assert_eq!(b.translation(), Vec2::ZERO);
    }

    #[test]
    fn a_flush_indexes_each_quad_as_two_triangles() {
        let mut b = QuadBatcher::new();
        b.begin(0, 0, 2, 2);
        for (x, y) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
            b.vertex(x, y);
        }
        for (x, y) in [(1.0, 0.0), (2.0, 0.0), (2.0, 1.0), (1.0, 1.0)] {
            b.vertex(x, y);
        }
        b.vertex(0.0, 0.0);
        let batch = b.flush();
        assert_eq!(batch.indices, [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
        assert_eq!(batch.len(), 8);
        assert_eq!(
            batch.mesh_with(NORMALIZED_PROJECTION).primitive_topology(),
            PrimitiveTopology::TriangleList
        );
    }

    #[test]
    fn colours_and_texture_coordinates_belong_to_the_next_vertex() {
        let mut b = QuadBatcher::new();
        b.begin(0, 0, 1, 1);
        b.set_colour([1, 2, 3, 4]);
        b.texcoord(0.25, 0.75);
        b.vertex(0.0, 0.0);
        b.quad_texcoords_default();
        b.set_rgb([9, 8, 7], 6);
        b.vertex(1.0, 0.0);
        b.vertex(1.0, 1.0);
        b.vertex(0.0, 1.0);
        let batch = b.flush();
        assert_eq!(batch.uvs[0], [0.25, 0.75]);
        assert_eq!(&batch.uvs[1..4], &QUAD_TEXCOORDS[..3]);
        assert_eq!(batch.colours[0], [1, 2, 3, 4]);
        assert_eq!(batch.colours[1], [9, 8, 7, 6]);
    }

    #[test]
    fn the_line_draw_takes_vertex_pairs() {
        let mut b = QuadBatcher::new();
        b.begin(0, 0, 100, 100);
        b.vertex(0.0, 0.0);
        b.vertex(100.0, 100.0);
        b.vertex(50.0, 0.0);
        let batch = b.flush_lines();
        assert_eq!(batch.positions, [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0]]);
        assert!(batch.indices.is_empty());
        assert_eq!(
            batch.mesh_with(NORMALIZED_PROJECTION).primitive_topology(),
            PrimitiveTopology::LineList
        );
    }

    #[test]
    fn the_raw_projection_maps_the_viewports_pixels_to_clip() {
        let mut b = QuadBatcher::new();
        b.begin(0, 0, 800, 600);
        let m = b.projection();
        assert_eq!(m.x_axis.x, 2.0 * (1.0 / 800.0));
        assert_eq!(m.y_axis.y, -(1.0 / 600.0) * 2.0);
        assert_eq!((m.z_axis.z, m.w_axis.z), (-0.001, -0.001));
        assert_eq!((m.w_axis.x, m.w_axis.y), (-1.0, 1.0));
        let p = m * Vec4::new(800.0, 600.0, 0.0, 1.0);
        assert!((p.x - 1.0).abs() < 1e-6 && (p.y + 1.0).abs() < 1e-6);
        // A translation moves the origin.
        b.translate(400.0, 300.0);
        let m = b.projection();
        assert!(m.w_axis.x.abs() < 1e-6 && m.w_axis.y.abs() < 1e-6);
    }

    #[test]
    fn the_vertex_words_pick_the_six_builtin_programs() {
        assert!(BUILTIN_SHADERS[2].contains("vCol * 0.003921568"));
        assert_eq!(builtin_for(0), 0);
        assert_eq!(builtin_for(0x10), 1);
        assert_eq!(builtin_for(0x4 | 0x10), 2);
        assert_eq!(builtin_for(0x4), 3);
        assert_eq!(builtin_for(0x20), 4);
        assert_eq!(builtin_for(0x20 | 0x10), 4);
        assert_eq!(builtin_for(0x20 | 0x4 | 0x10), 5);
        assert_eq!(ATTRIBUTE_LOCATIONS[5], ("vTng", 6, 0x8));
        assert_eq!(ATTRIBUTE_LOCATIONS[6], ("vDlt", 7, 0x40));
    }
}
