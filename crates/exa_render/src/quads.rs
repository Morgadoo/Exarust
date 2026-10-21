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
    pub colours: Vec<[u8; 4]>,
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

    /// The batch as a Bevy triangle-list mesh.
    pub fn mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
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
        let quads = batch.positions.len() / QUAD_VERTICES;
        if batch.positions.len() > quads * QUAD_VERTICES {
            batch.positions.truncate(quads * QUAD_VERTICES);
            batch.uvs.truncate(quads * QUAD_VERTICES);
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
}

impl LineBatch {
    /// The batch as a Bevy `LineList` mesh.
    pub fn mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::LineList,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
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
        if !batch.positions.len().is_multiple_of(2) {
            batch.positions.pop();
            batch.colours.pop();
        }
        batch
    }
}

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
