//! The original's immediate-mode 2-D quad batcher, in pixel coordinates,
//! and its textured and line variants.
//!
//! `begin` takes the viewport's pixel rectangle and computes
//! `1/width`, `1/height`; every vertex is scaled by those and drawn with
//! the y axis flipped: `ndc = ((x + tx)·sx, (ty − y)·sy)`, where the
//! translation stack (`translate`, `push`, `pop`, 16 levels) shifts the
//! coordinates. A colour is RGBA bytes; a texture coordinate belongs to the
//! next vertex. `flush` turns every four vertices into one quad, its two
//! triangles indexed `(0,1,2)` and `(0,2,3)`, with a `u16` index buffer.
//!
//! The textured variant is the same batcher with UVs filled. The line
//! variant emits vertex pairs and no indices, for `LineList` topologies.
//! Exarust keeps the arithmetic as data and turns a flush into a Bevy
//! `Mesh` ([`QuadBatch::mesh`]); shader compile and link are wgpu's.
//!
//! Rules: this is the GL immediate-draw helper cluster's behaviour; the
//! batcher's arithmetic is pinned by tests.

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexFormat};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

/// The translation stack's depth (the original's 16 entries).
pub const TRANSLATION_LEVELS: usize = 16;
/// How many vertices one quad takes.
pub const QUAD_VERTICES: usize = 4;
/// How many indices one quad takes.
pub const QUAD_INDICES: usize = 6;
/// The maximum `u16` vertex count a flushed batch can index.
pub const MAX_U16_VERTICES: usize = u16::MAX as usize + 1;

/// The vertex-colour attribute the batcher writes: RGBA bytes.
pub const COLOR: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Color", 1, VertexFormat::Unorm8x4);

/// A flushed quad batch: positions in clip space (z 0), texture
/// coordinates and RGBA bytes, with the quad index list.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuadBatch {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// The second texture-coordinate channel (the terrain texrender's).
    pub uv2s: Vec<[f32; 2]>,
    pub colours: Vec<[u8; 4]>,
    pub indices: Vec<u16>,
    /// The viewport's projection, which the original's vertex program
    /// applies (`MPrj`); [`QuadBatch::mesh`] applies it on the CPU because
    /// Exarust's fixed shader takes clip positions.
    pub projection: Mat4,
}

impl QuadBatch {
    /// The number of vertices.
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// The batch as a Bevy triangle-list mesh with its own projection.
    pub fn mesh(&self) -> Mesh {
        self.mesh_with(self.projection)
    }

    /// The batch as a Bevy triangle-list mesh with `projection` applied to
    /// its positions.
    pub fn mesh_with(&self, projection: Mat4) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
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
        mesh.insert_indices(Indices::U16(self.indices.clone()));
        mesh
    }
}

/// The batcher's state.
#[derive(Debug, Clone, PartialEq)]
pub struct QuadBatcher {
    /// `1 / viewport width` and `1 / viewport height`.
    pub scale_x: f32,
    pub scale_y: f32,
    /// The current translation and the stack of saved ones (`(x, y)`).
    translation: [Vec2; TRANSLATION_LEVELS],
    level: usize,
    /// The translation and y flip the current batch uses: set by `begin`
    /// from the stack's current level, as the original's vertex path reads
    /// its two offset words kept from the begin call.
    offset: Vec2,
    /// The current colour (RGBA bytes).
    pub colour: [u8; 4],
    /// Whether vertices take the current colour and a texture coordinate.
    pub textured: bool,
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
            translation: [Vec2::ZERO; TRANSLATION_LEVELS],
            level: 0,
            offset: Vec2::ZERO,
            colour: [255; 4],
            textured: true,
            batch: QuadBatch::default(),
        }
    }

    /// Starts a batch for a viewport whose pixel rectangle is
    /// `(left, top)..(right, bottom)`: `sx = 1/(right − left)`,
    /// `sy = 1/(bottom − top)`, with the translation on the stack's level
    /// zero.
    pub fn begin(&mut self, left: f32, top: f32, right: f32, bottom: f32) {
        let _ = top;
        self.scale_x = 1.0 / (right - left);
        self.scale_y = 1.0 / (bottom - top);
        let t = self.translation[self.level];
        self.offset = Vec2::new(t.x, (bottom - top) - t.y);
        self.batch = QuadBatch::default();
    }

    /// Adds a translation. The stack is 16 deep; past it the level stays at
    /// the top and the position is not saved (the original clamps).
    pub fn translate(&mut self, dx: f32, dy: f32) {
        if self.level + 1 >= TRANSLATION_LEVELS {
            return;
        }
        self.level += 1;
        let current = self.translation[self.level];
        self.translation[self.level] = current + Vec2::new(dx, dy);
    }

    /// Pops a translation.
    pub fn pop(&mut self) {
        if self.level > 0 {
            self.level -= 1;
        }
    }

    /// Clears the stack (the original's unit start-up).
    pub fn reset(&mut self) {
        self.level = 0;
        self.translation = [Vec2::ZERO; TRANSLATION_LEVELS];
    }

    /// Sets the current colour, RGBA bytes.
    pub fn set_colour(&mut self, colour: [u8; 4]) {
        self.colour = colour;
    }

    /// The next vertex's texture coordinate. It belongs to the vertex
    /// [`QuadBatcher::vertex`] adds next, as the immediate-mode call order
    /// has it.
    pub fn texcoord(&mut self, u: f32, v: f32) {
        let at = self.batch.positions.len();
        if self.batch.uvs.len() <= at {
            self.batch.uvs.resize(at + 1, [0.0; 2]);
        }
        self.batch.uvs[at] = [u, v];
    }

    /// The next four vertices' texture coordinates in one call (the
    /// original's per-quad setter).
    pub fn quad_texcoords(&mut self, uvs: [[f32; 2]; 4]) {
        let at = self.batch.positions.len();
        self.batch.uvs.resize(at + 4, [0.0; 2]);
        self.batch.uvs[at..at + 4].copy_from_slice(&uvs);
    }

    /// The next vertex's second texture coordinate.
    pub fn texcoord2(&mut self, u: f32, v: f32) {
        let at = self.batch.positions.len();
        if self.batch.uv2s.len() <= at {
            self.batch.uv2s.resize(at + 1, [0.0; 2]);
        }
        self.batch.uv2s[at] = [u, v];
    }

    /// The next four vertices' second texture coordinates.
    pub fn quad_texcoord2s(&mut self, uvs: [[f32; 2]; 4]) {
        let at = self.batch.positions.len();
        self.batch.uv2s.resize(at + 4, [0.0; 2]);
        self.batch.uv2s[at..at + 4].copy_from_slice(&uvs);
    }

    /// The projection the original builds from the viewport
    /// (`0x1001f97a0`): the template's 2 and −2 scaled by `1/width` and
    /// `1/height`, translated by `2·tx/width − 1` and `1 − 2·ty/height`.
    /// The original uses it for the raw pixel-vertex batcher; Exarust's
    /// pixel vertices are already divided by the viewport, so [`flush`]
    /// uses [`QuadBatcher::normalized_projection`] instead.
    pub fn projection(&self) -> Mat4 {
        Mat4::from_cols_array(&[
            2.0 * self.scale_x,
            0.0,
            0.0,
            0.0,
            0.0,
            -2.0 * self.scale_y,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            self.translation[self.level].x * self.scale_x * 2.0 - 1.0,
            1.0 - self.translation[self.level].y * self.scale_y * 2.0,
            0.0,
            1.0,
        ])
    }

    /// The projection for our coordinates already in 0..1, counted from
    /// the top: `x·2 − 1` and `y·2 − 1`, so 0 is the left/bottom clip edge
    /// and 1 the right/top.
    pub fn normalized_projection(&self) -> Mat4 {
        Mat4::from_cols_array(&[
            2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0,
        ])
    }

    /// A vertex copied as given (the original's raw batcher,
    /// `0x1001f9980`): x/y unchanged, z zero, with the current colour. A
    /// caller filling a batch this way gives it [`QuadBatcher::projection`]
    /// through [`QuadBatch::mesh_with`].
    pub fn vertex_raw(&mut self, x: f32, y: f32) {
        let at = self.batch.positions.len();
        self.batch.positions.push([x, y, 0.0]);
        if self.batch.uvs.len() <= at {
            self.batch.uvs.push([0.0; 2]);
        }
        if self.batch.uv2s.len() <= at {
            self.batch.uv2s.push([0.0; 2]);
        }
        self.batch.colours.push(self.colour);
    }

    /// Adds a vertex in pixel coordinates, with the current colour and the
    /// texture coordinate already set for this slot.
    pub fn vertex(&mut self, x: f32, y: f32) {
        let position = [
            (x + self.offset.x) * self.scale_x,
            (self.offset.y - y) * self.scale_y,
            0.0,
        ];
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

    /// The vertices added so far.
    pub fn len(&self) -> usize {
        self.batch.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.batch.positions.is_empty()
    }

    /// Finishes the batch: every four vertices become a quad with two
    /// triangles, indexed `(0,1,2)` and `(0,2,3)`. A trailing partial quad
    /// is dropped, as the original's `count / 4` does. Returns the batch,
    /// clearing the batcher's.
    pub fn flush(&mut self) -> QuadBatch {
        let mut batch = std::mem::take(&mut self.batch);
        batch.projection = self.normalized_projection();
        let quads = batch.positions.len() / QUAD_VERTICES;
        if batch.positions.len() > quads * QUAD_VERTICES {
            batch.positions.truncate(quads * QUAD_VERTICES);
            batch.uvs.truncate(quads * QUAD_VERTICES);
            batch.uv2s.truncate(quads * QUAD_VERTICES);
            batch.colours.truncate(quads * QUAD_VERTICES);
        }
        batch.indices.reserve(quads * QUAD_INDICES);
        for q in 0..quads {
            let base = (q * QUAD_VERTICES) as u16;
            batch
                .indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        batch
    }
}

/// One line batch: vertex pairs in pixel coordinates, for `LineList`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LineBatch {
    pub positions: Vec<[f32; 3]>,
    pub colours: Vec<[u8; 4]>,
    pub projection: Mat4,
}

impl LineBatch {
    /// The batch as a Bevy `LineList` mesh.
    pub fn mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::LineList,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        let projected: Vec<[f32; 3]> = self
            .positions
            .iter()
            .map(|p| {
                let v = self.projection * Vec4::new(p[0], p[1], p[2], 1.0);
                [v.x, v.y, v.z]
            })
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, projected);
        mesh.insert_attribute(
            COLOR,
            bevy::mesh::VertexAttributeValues::Unorm8x4(self.colours.clone()),
        );
        mesh
    }
}

/// The line variant: the same pixel-to-clip arithmetic as the quad batcher,
/// appending vertex pairs, with 16-level translations.
#[derive(Debug, Clone, PartialEq)]
pub struct LineBatcher {
    pub batcher: QuadBatcher,
    batch: LineBatch,
}

impl Default for LineBatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl LineBatcher {
    pub fn new() -> Self {
        Self {
            batcher: QuadBatcher::new(),
            batch: LineBatch::default(),
        }
    }

    pub fn begin(&mut self, left: f32, top: f32, right: f32, bottom: f32) {
        self.batcher.begin(left, top, right, bottom);
        self.batch = LineBatch::default();
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.batcher.translate(dx, dy);
    }

    pub fn pop(&mut self) {
        self.batcher.pop();
    }

    pub fn set_colour(&mut self, colour: [u8; 4]) {
        self.batcher.set_colour(colour);
    }

    /// Adds a vertex to the line list.
    pub fn vertex(&mut self, x: f32, y: f32) {
        self.batcher.vertex(x, y);
        let position = *self.batcher.batch.positions.last().unwrap_or(&[0.0; 3]);
        self.batch.positions.push(position);
        self.batch.colours.push(self.batcher.colour);
    }

    pub fn len(&self) -> usize {
        self.batch.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.batch.positions.is_empty()
    }

    /// Returns the line list, dropping an odd trailing vertex.
    pub fn flush(&mut self) -> LineBatch {
        let mut batch = std::mem::take(&mut self.batch);
        batch.projection = self.batcher.normalized_projection();
        if !batch.positions.len().is_multiple_of(2) {
            batch.positions.pop();
            batch.colours.pop();
        }
        batch
    }
}

/// The six built-in immediate-draw vertex programs, as the original's
/// library stores them (CRLF line endings). The fragment program is the
/// caller's own or the built-in `shading` one; Exarust's batcher draws with
/// `quad.wesl` and these are kept for comparison.
pub const BUILTIN_SHADERS: [&str; 6] = [
    "in vec3 vPos;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\n}",
    "in vec3 vPos;\r\nin vec2 vTex;\r\nout vec2 vrTex;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrTex = vTex;\r\n}",
    "in vec3 vPos;\r\nin vec4 vCol;\r\nin vec2 vTex;\r\nout vec4 vrCol;\r\nout vec2 vrTex;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrTex = vTex;\r\nvrCol = vCol * 0.003921568;\r\n}",
    "in vec3 vPos;\r\nin vec4 vCol;\r\nout vec4 vrCol;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrCol = vCol * 0.003921568;\r\n}",
    "in vec3 vPos;\r\nin vec2 vTex;\r\nin vec2 vTx2;\r\nout vec2 vrTex;\r\nout vec2 vrTx2;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrTex = vTex;\r\nvrTx2 = vTx2;\r\n}",
    "in vec3 vPos;\r\nin vec2 vTex;\r\nin vec2 vTx2;\r\nin vec4 vCol;\r\nout vec2 vrTex;\r\nout vec2 vrTx2;\r\nout vec4 vrCol;\r\nuniform mat4 MPrj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrTex = vTex;\r\nvrTx2 = vTx2;\r\nvrCol = vCol * 0.003921568;\r\n}",
];

/// The built-in program a vertex-format word picks: bit `0x20` adds the
/// second texture channel, bit `0x4` the colour and bit `0x10` the first
/// texture channel. The order of the tests is the original's.
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

/// The attribute locations the original binds before linking: position 0,
/// texture coordinate 1, normal 2, colour 3, second texture coordinate 4,
/// then attributes 6 and 7.
pub const ATTRIBUTE_LOCATIONS: [(&str, u32); 7] = [
    ("vPos", 0),
    ("vTex", 1),
    ("vNrm", 2),
    ("vCol", 3),
    ("vTx2", 4),
    ("vAttr6", 6),
    ("vAttr7", 7),
];

/// The material a flushed batch draws with: an untextured colour quad, or a
/// textured one.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct QuadMaterial {
    #[uniform(0)]
    pub colour: Vec4,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
}

fn quad_shader() -> ShaderRef {
    ShaderRef::Path(AssetPath::from_path_buf(embedded_path!("quad.wesl")).with_source("embedded"))
}

impl Material for QuadMaterial {
    fn fragment_shader() -> ShaderRef {
        quad_shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

pub struct QuadPlugin;

impl Plugin for QuadPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "quad.wesl");
        app.add_plugins(MaterialPlugin::<QuadMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_map_to_clip_with_the_y_axis_flipped() {
        let mut b = QuadBatcher::new();
        b.begin(0.0, 0.0, 800.0, 600.0);
        b.vertex(0.0, 0.0);
        b.vertex(800.0, 0.0);
        b.vertex(800.0, 600.0);
        b.vertex(0.0, 600.0);
        let batch = b.flush();
        assert_eq!(batch.positions[0], [0.0, 1.0, 0.0]);
        assert_eq!(batch.positions[1], [1.0, 1.0, 0.0]);
        assert_eq!(batch.positions[2], [1.0, 0.0, 0.0]);
        assert_eq!(batch.positions[3], [0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_viewport_origin_and_its_translation_stack_shift_the_pixels() {
        let mut b = QuadBatcher::new();
        // The translation is set before the batch begins: begin snapshots
        // it, and a translation after begin waits for the next begin, as
        // the original's offset words are set by its begin call.
        b.translate(10.0, 20.0);
        b.begin(100.0, 50.0, 500.0, 250.0);
        b.vertex(100.0, 50.0);
        b.translate(100.0, 100.0);
        b.vertex(100.0, 50.0);
        b.vertex(500.0, 250.0);
        b.vertex(500.0, 250.0);
        let batch = b.flush();
        // (100 + 10)/400 = 0.275; (200 − 70)/200 = 0.65.
        assert_eq!(batch.positions[0], [0.275, 0.65, 0.0]);
        // The second vertex used the same snapshot.
        assert_eq!(batch.positions[1], [0.275, 0.65, 0.0]);
        b.pop();
        b.pop();
        b.begin(100.0, 50.0, 500.0, 250.0);
        for _ in 0..4 {
            b.vertex(100.0, 50.0);
        }
        let batch = b.flush();
        assert_eq!(batch.positions[0], [0.25, 0.75, 0.0]);
        // The stack caps at 16 levels.
        for _ in 0..40 {
            b.translate(1.0, 1.0);
        }
        b.vertex(0.0, 0.0);
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn a_flush_indexes_each_quad_as_two_triangles() {
        let mut b = QuadBatcher::new();
        b.begin(0.0, 0.0, 2.0, 2.0);
        for (x, y) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
            b.vertex(x, y);
        }
        for (x, y) in [(1.0, 0.0), (2.0, 0.0), (2.0, 1.0), (1.0, 1.0)] {
            b.vertex(x, y);
        }
        let batch = b.flush();
        assert_eq!(batch.indices, [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
        // A partial quad is dropped.
        b.vertex(0.0, 0.0);
        b.vertex(1.0, 0.0);
        assert!(b.flush().is_empty());
    }

    #[test]
    fn colours_and_texture_coordinates_belong_to_the_next_vertex() {
        let mut b = QuadBatcher::new();
        b.begin(0.0, 0.0, 1.0, 1.0);
        b.set_colour([1, 2, 3, 4]);
        b.texcoord(0.25, 0.75);
        b.vertex(0.0, 0.0);
        b.quad_texcoords([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        b.vertex(1.0, 0.0);
        b.vertex(1.0, 1.0);
        b.vertex(0.0, 1.0);
        let batch = b.flush();
        assert_eq!(batch.uvs[0], [0.25, 0.75]);
        assert_eq!(batch.uvs[1], [0.0, 0.0]);
        assert_eq!(batch.uvs[2], [1.0, 0.0]);
        assert_eq!(batch.uvs[3], [1.0, 1.0]);
        assert_eq!(batch.colours[0], [1, 2, 3, 4]);
        // The quad setter started at the current next vertex, so its fourth
        // pair lands past this quad; the flush keeps it (four vertices).
        assert_eq!(batch.positions.len(), 4);
    }

    #[test]
    fn the_line_variant_emits_pairs_without_indices() {
        let mut l = LineBatcher::new();
        l.begin(0.0, 0.0, 100.0, 100.0);
        l.set_colour([9, 9, 9, 255]);
        l.vertex(0.0, 0.0);
        l.vertex(100.0, 100.0);
        l.vertex(50.0, 0.0);
        let batch = l.flush();
        assert_eq!(batch.positions.len(), 2);
        assert_eq!(batch.positions[1], [1.0, 0.0, 0.0]);
        assert_eq!(batch.colours[0], [9, 9, 9, 255]);
        assert_eq!(
            batch.mesh().primitive_topology(),
            PrimitiveTopology::LineList
        );
    }

    #[test]
    fn the_projection_matrix_maps_the_viewport_to_clip() {
        let mut b = QuadBatcher::new();
        b.begin(0.0, 0.0, 800.0, 600.0);
        let m = b.projection();
        assert!((m.x_axis.x - 2.0 / 800.0).abs() < 1e-6);
        assert!((m.y_axis.y + 2.0 / 600.0).abs() < 1e-6);
        assert_eq!(m.z_axis.z, 1.0);
        assert_eq!((m.w_axis.x, m.w_axis.y), (-1.0, 1.0));
        let p = m * Vec4::new(0.0, 0.0, 0.0, 1.0);
        assert_eq!((p.x, p.y), (-1.0, 1.0));
        let p = m * Vec4::new(800.0, 600.0, 0.0, 1.0);
        assert!((p.x - 1.0).abs() < 1e-6 && (p.y + 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_vertex_words_pick_the_six_builtin_programs() {
        assert_eq!(BUILTIN_SHADERS.len(), 6);
        assert!(BUILTIN_SHADERS[2].contains("vCol * 0.003921568"));
        assert_eq!(builtin_for(0), 0);
        assert_eq!(builtin_for(0x10), 1);
        assert_eq!(builtin_for(0x4 | 0x10), 2);
        assert_eq!(builtin_for(0x4), 3);
        assert_eq!(builtin_for(0x20), 4);
        assert_eq!(builtin_for(0x20 | 0x4 | 0x10), 5);
        assert_eq!(ATTRIBUTE_LOCATIONS[0], ("vPos", 0));
        assert_eq!(ATTRIBUTE_LOCATIONS[3], ("vCol", 3));
        assert_eq!(ATTRIBUTE_LOCATIONS[6], ("vAttr7", 7));
    }

    #[test]
    fn the_second_texture_channel_and_clip_vertices() {
        let mut b = QuadBatcher::new();
        b.begin(0.0, 0.0, 100.0, 100.0);
        b.texcoord2(0.5, 0.25);
        b.vertex(0.0, 0.0);
        b.vertex_raw(0.25, 0.75);
        b.vertex(100.0, 100.0);
        b.vertex_raw(0.25, 0.75);
        let batch = b.flush();
        assert_eq!(batch.uv2s[0], [0.5, 0.25]);
        assert_eq!(batch.positions[1], [0.25, 0.75, 0.0]);
        assert_eq!(batch.positions[0], [0.0, 1.0, 0.0]);
        // The mesh applies the projection: pixel 0,0 is clip -1,1.
        let projected = batch.mesh();
        match projected.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
            bevy::mesh::VertexAttributeValues::Float32x3(p) => {
                assert_eq!(p[0], [-1.0, 1.0, 0.0])
            }
            _ => panic!("positions"),
        }
    }

    #[test]
    fn the_batch_becomes_a_triangle_mesh_with_bytes_of_colour() {
        let mut b = QuadBatcher::new();
        b.begin(0.0, 0.0, 1.0, 1.0);
        for (x, y) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
            b.vertex(x, y);
        }
        let mesh = b.flush().mesh();
        assert_eq!(mesh.primitive_topology(), PrimitiveTopology::TriangleList);
        assert_eq!(mesh.count_vertices(), 4);
    }
}
