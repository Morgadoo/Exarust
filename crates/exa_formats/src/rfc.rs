//! Scene nodes and static meshes inside `.rfc` chunk trees.
//!
//! A file starts with `u16 0xAFCF` and a `u16` format version, followed by
//! root chunks. The node list chunk (`0x3D00_0000`) holds a count and one
//! chunk per node; the chunk tag is the node kind. Every node starts with the
//! same header (name, flags, instance reference, parent index, transform,
//! bounds); mesh nodes (`0x3D03`) continue with vertex streams and a topology
//! chunk. Layout and evidence: `docs/formats/RFC.md`.
//!
//! Geometry is returned in metres, right-handed and Y-up. The stored units are
//! centimetres in the same handedness, so the conversion is a uniform scale.

use crate::characters::{CHARACTERS, LevelCharacters};
use crate::environment::{ENVIRONMENT, Environment};
use crate::key_track::KeyTrack;
use crate::level::{self, LevelGrid, Prop};
use crate::material::Material;
use crate::morph::MorphSet;
use crate::objdb::Item;
use crate::objstrings::Library;
use crate::rdb::Database;
use crate::rigid_body::RigidBody;
use crate::{Error, Result, read_u32};

const SANITY: u16 = 0xAFCF;
/// Format versions after this one store one extra word at the start of a mesh body.
const VERSION_WITHOUT_MESH_PREFIX: u16 = 0x3D21;
const CENTIMETRES_TO_METRES: f32 = 0.01;

pub const NODE_LIST: u32 = 0x3D00_0000;
pub const MATERIAL_LIST: u32 = 0xBA00;
pub const NODE_MESH: u32 = 0x3D03;
pub const NODE_LIGHT: u32 = 0x3D06;
/// A voxel light: light the node adds straight into the GI volume around
/// itself (class `0x41`), never onto surfaces.
pub const NODE_VOXEL_LIGHT: u32 = 0x3D0C;
const TOPOLOGY: u32 = 0x3DC0;
const TRIANGLES: u32 = 0x3DC1;

/// Index maps are matched to streams by the low 16 bits of their key.
const KEY_POSITIONS: u32 = 0x0001;
const KEY_UVS: u32 = 0x0010;
const KEY_SMOOTHING: u32 = 0x0002;

/// Indices are 16-bit when the unified vertex count is below this.
const WIDE_INDEX_THRESHOLD: usize = 0xFFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamFormat {
    Vec3,
    Vec2,
    Rgba8,
    Skin,
}

impl StreamFormat {
    fn from_word(word: u32) -> Option<Self> {
        match (word >> 16) & 0xFF {
            0xF3 => Some(Self::Vec3),
            0xF2 => Some(Self::Vec2),
            0xB4 => Some(Self::Rgba8),
            0xA4 => Some(Self::Skin),
            _ => None,
        }
    }

    fn element_size(self) -> usize {
        match self {
            Self::Vec3 => 12,
            Self::Vec2 => 8,
            Self::Rgba8 => 4,
            Self::Skin => 32,
        }
    }
}

/// A decoded chunk-tree file: its version and its scene nodes.
#[derive(Debug, Clone)]
pub struct Scene {
    pub version: u16,
    pub nodes: Vec<Node>,
    /// Materials defined in the file, by name; undecodable payloads keep the error.
    pub materials: Vec<(String, std::result::Result<Material, String>)>,
    /// Tile grid, in dungeon level files.
    pub grid: Option<LevelGrid>,
    /// Objects placed from the object library, in level files.
    pub props: Vec<Prop>,
    /// Ambient, extent and sun settings, in level files.
    pub environment: Option<Environment>,
    /// Characters placed in the level and their records.
    pub characters: Option<LevelCharacters>,
    /// The level's own object database (items its characters and placed
    /// objects refer to with local ids).
    pub local_items: Vec<Item>,
    /// The level's own string table (sub-chunk `0xDBCD` of its local
    /// database), which the level's items name their text by. A table that
    /// does not decode is reported in [`Scene::level_errors`].
    pub local_strings: Option<Library>,
    /// Why the level's stored string table did not decode, if it did not
    /// (the rest of the level still loads, as in the original).
    pub local_string_error: Option<String>,
    /// Objects from the object databases placed in the level (items, doors,
    /// containers).
    pub placed_objects: Vec<PlacedObject>,
    /// Root chunk tags the reader does not know: the original hands them to
    /// its optional callback and skips them.
    pub unknown_chunks: Vec<u32>,
    /// Level sections that could not be decoded.
    pub level_errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Node {
    /// Chunk tag of the node; [`NODE_MESH`] for meshes.
    pub kind: u32,
    pub name: String,
    pub flags: u32,
    pub flags2: u32,
    /// Index (1-based) of the parent in [`Scene::nodes`]; `None` at the root.
    pub parent: Option<usize>,
    /// Row-major 3×4: three rotation rows, then the translation (metres).
    pub transform: [[f32; 3]; 4],
    /// Bounding box in the node's space (metres): its centre and half
    /// extents. The sphere shares the centre.
    pub bounds_centre: [f32; 3],
    pub bounds_half_extents: [f32; 3],
    pub bounds_radius: f32,
    /// The node's collision body (sub-chunk [`RIGID_BODY`] after a mesh
    /// body): the original makes the node a rigid body. A body that does
    /// not decode keeps its error.
    pub rigid_body: Option<std::result::Result<RigidBody, String>>,
    /// The node carries the cloth marker (sub-chunk [`CLOTH`] after a mesh
    /// body): the original simulates the mesh as cloth when it is loaded
    /// into the game's scene (long hair, banners, the wraith's robe).
    pub cloth_marker: bool,
    /// The node's keyframe tracks (sub-chunks [`KEY_TRACK`] after any
    /// stored node's body), in the file's order: curves the original plays
    /// on the node's translation when the object is used. A track that
    /// does not decode keeps its error.
    pub key_tracks: Vec<std::result::Result<KeyTrack, String>>,
    /// The node's morph set (sub-chunk [`MORPH`] after a mesh body): the
    /// body of a race's `<stem>morph.rfc` carries the physique targets
    /// (`crate::morph`). A set that does not decode keeps its error.
    pub morph: Option<std::result::Result<MorphSet, String>>,
    /// The instance word of the node header: a 1-based index into
    /// [`Scene::nodes`] of the node that owns this one, or `None` for a
    /// node that owns itself. A node with an owner stores no body or
    /// sub-chunks.
    pub instance: Option<usize>,
    /// Sub-chunk tags after the node's body that this reader does not know;
    /// the original hands each to its registered handlers.
    pub unknown_sub_chunks: Vec<u32>,
    pub body: NodeBody,
}

impl Node {
    /// Node flag: the node is level geometry. The original never registers
    /// the rigid body of a node marked so (its meshes stay static
    /// colliders): a level's own nodes can carry it in the file (the tavern
    /// table of `arenao1`), and a prop placed static
    /// ([`crate::level::Prop::STATIC`]) sets it on every node of the prop.
    pub const STATIC: u32 = 0x8_0000;

    /// Node flag: a node the original's object-library clone replaces with a
    /// plain empty node that keeps only the transform (the master keeps its
    /// own data). No shipped model sets it (a scan of every node of the
    /// retail packs finds none), so it only matters to models that do.
    pub const PLACEHOLDER: u32 = 0x8000;

    /// Whether the node is marked as level geometry ([`Node::STATIC`]).
    pub fn is_static(&self) -> bool {
        self.flags & Self::STATIC != 0
    }

    /// Whether a collision body follows the mesh body, decoded or not.
    pub fn has_rigid_body(&self) -> bool {
        self.rigid_body.is_some()
    }

    /// The decoded collision body.
    pub fn rigid(&self) -> Option<&RigidBody> {
        self.rigid_body.as_ref().and_then(|b| b.as_ref().ok())
    }

    /// The decoded keyframe tracks.
    pub fn tracks(&self) -> impl Iterator<Item = &KeyTrack> {
        self.key_tracks.iter().filter_map(|t| t.as_ref().ok())
    }
}

#[derive(Debug, Clone)]
pub enum NodeBody {
    Mesh(Box<Mesh>),
    Light(Light),
    VoxelLight(VoxelLight),
    /// The node reuses another object's data; the word is its owner's
    /// 1-based index in the node list (see [`Node::instance`]).
    Instance(u32),
    /// A camera node (`0x3D08`): the node its view follows (a 1-based
    /// index into the node list, 0 for none) and one word the original
    /// keeps unread.
    Camera {
        target: Option<usize>,
        value: u32,
    },
    /// A sector node (`0x3D0A`): the original makes a scene sector of it.
    /// Exarust's rooms are its sectors; nothing is stored after the header.
    Sector,
    /// A plain node carrying a particle emitter record (`0x3DD0E002` is a
    /// fire, `0x3DD0E004` a level portal): its type and size.
    Emitter {
        kind: u32,
        size: f32,
    },
    /// A node kind that is not a mesh (lights, plain nodes, …).
    NotMesh,
    /// A mesh record that could not be decoded.
    Invalid(String),
}

/// An omni light (`0x3D06`). Values are kept raw where their meaning is
/// only partly established.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    /// Body flags: `0x10` static, `0x10000` flicker, `0x20000` fade.
    pub flags: u32,
    /// sRGB.
    pub color: [f32; 3],
    /// Scales the light's injection into the voxel GI only (0 leaves it out).
    pub multiplier: f32,
    /// Scales the light's colour.
    pub intensity: f32,
    /// Radius of influence in metres.
    pub radius: f32,
    /// Shadow softness: the shadow filter radius is (softness + 1)·0.005.
    pub softness: f32,
    /// Vertical spot shaping (q, p), stored for nodes whose flags masked by
    /// `0x300` are `0x200`.
    pub spot: Option<[f32; 2]>,
    /// Falls off with distance (node flag `0x40000`); otherwise the light
    /// reaches everywhere at full strength.
    pub ranged: bool,
    /// The flicker and fade parameters a body with flag `0x10000` or
    /// `0x20000` stores after the shaping words; `None` for the others,
    /// which keep the light constructor's defaults.
    pub animation: Option<LightAnimation>,
}

/// A light body's flicker and fade parameters, in the order the original's
/// loader reads them (after one skipped word).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightAnimation {
    /// How far the flicker moves the intensity: `(1 + 0.1·x·amplitude)` of
    /// the base for the walk's `x`; the level a fade falls to. The
    /// constructor's default is 0.1.
    pub amplitude: f32,
    /// How fast the walk and a fade move, per frame millisecond (×0.01).
    /// The constructor's default is 0.5.
    pub speed: f32,
    /// How much of the walk the parent mesh's emissive takes. The
    /// constructor's default is 0.5.
    pub emissive: f32,
}

/// A voxel light (`0x3D0C`): a glowing item's light (salves, power
/// crystals, embers, …), which the original adds into its GI volume only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelLight {
    /// sRGB.
    pub color: [f32; 3],
    /// Scales the light by its square root.
    pub intensity: f32,
    /// Stored radius in metres; the GI injection never reads it.
    pub radius: f32,
}

/// Node flags (`flags2`) of a light with vertical spot shaping, under
/// [`SPOT_MASK`].
pub const SPOT: u32 = 0x200;
pub const SPOT_MASK: u32 = 0x300;
/// Node flag (`flags2`): the light falls off with distance.
pub const RANGED: u32 = 0x4_0000;

/// A triangle mesh with unified vertices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Option<Vec<[f32; 2]>>,
    /// Raw per-vertex RGBA8; channel order is not yet verified.
    pub colors: Option<Vec<[u8; 4]>>,
    pub skin: Option<Vec<SkinWeights>>,
    /// Position-stream element of each vertex: vertices sharing a position
    /// share an id (seams split vertices by UV or normal).
    pub position_ids: Vec<u32>,
    /// Triangle list, counter-clockwise front faces.
    pub indices: Vec<u32>,
    pub groups: Vec<MeshGroup>,
    /// Per-triangle flags (sub-chunk `0x3DC3`): smoothing-group bits in the
    /// low 16 bits, seam classes in the high byte, apparel cull bits
    /// `0x200000`/`0x400000`.
    pub triangle_flags: Option<Vec<u32>>,
    /// Per-triangle edge bits (sub-chunk `0x3DC2`, three low bits, one per
    /// edge): kept through edits, not drawn. Bit `i + 3` marks corner `i`
    /// as one the apparel skin builder leaves out of its boundary edges.
    pub triangle_edges: Option<Vec<u8>>,
    /// Points sampling the surfaces on a ~40 cm lattice (sub-chunk
    /// `0x3D0CEC05`, after the mesh body), in metres: they mark the mesh's
    /// cells as solid for the global illumination volume.
    pub gi_points: Vec<[f32; 3]>,
    /// The word before the streams in newer files (0 in older ones): the
    /// node's render flags, which the original copies onto the mesh node
    /// as it reads the body. Bits 16..18 are the shadow-bias level
    /// ([`Mesh::shadow_bias_level`]).
    pub render_flags: u32,
    /// Elements of the position stream: [`Mesh::position_ids`] index it,
    /// and a morph set holds one offset per element.
    pub position_count: usize,
    /// Smoothing key of each vertex: vertices sharing a key share a normal
    /// ([`Mesh::recompute_normals`]).
    pub smoothing: Vec<u32>,
    /// Texture-coordinate stream element of each vertex (empty without
    /// texture coordinates): vertices sharing an element share its value.
    pub uv_ids: Vec<u32>,
    /// Elements of the texture-coordinate stream.
    pub uv_count: usize,
}

/// A mesh's refitted bounds, in metres: the box centre and half extents and
/// the bounding-sphere radius.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MeshBounds {
    pub centre: [f32; 3],
    pub half_extents: [f32; 3],
    pub radius: f32,
}

/// The smallest half extent the original keeps (its 0.001 cm, in metres).
pub const BOUNDS_HALF_FLOOR: f32 = 0.001 * CENTIMETRES_TO_METRES;
/// The box helper's own floor under the half extents (its 1e-4 cm), which
/// also turns a NaN extent into a number before the 0.001 cm floor.
const BOUNDS_BOX_FLOOR: f32 = 1e-4 * CENTIMETRES_TO_METRES;
/// The squared radius (cm²) above which the original takes the square
/// root: its `0x33FFFFD7`, just under 2^−23.
const RADIUS_SQUARED_FLOOR_CM2: f32 = f32::from_bits(0x33FF_FFD7);
/// The face flag of faces that only shape a smoothing group's normal when
/// no face without it shares the group.
pub const FACE_NORMAL_FALLBACK: u32 = 0x1_0000;

impl Mesh {
    /// The node's shadow-bias level, 0..=7 (bits 16..18 of its render
    /// flags): the original scales a level-v caster's stored shadow
    /// distances by 1/(1 − 0.005·v − 0.01) and a receiver's by
    /// 1/(1 − 0.005·v), so its surfaces keep clear of their own shadow.
    pub fn shadow_bias_level(&self) -> u32 {
        (self.render_flags >> 16) & 7
    }

    /// Bit 19 of the render flags: the level scales both sides by
    /// 1 + 0.005·v instead (one shipped mesh, `Pave03Brk`).
    pub fn shadow_bias_inflated(&self) -> bool {
        self.render_flags & 0x8_0000 != 0
    }

    /// Render flag `0x800`: in the shadow depth pass the original culls
    /// this mesh's front faces (it draws the back ones); without the bit the
    /// pass culls the back faces. Only the two shadow-pass draw routines
    /// read the bit, the colour pass does not. No shipped mesh sets it, and
    /// Exarust draws no mesh with it.
    pub fn shadow_cull_front(&self) -> bool {
        self.render_flags & 0x800 != 0
    }

    /// The node's box and bounding sphere from its vertices, as the original
    /// refits them: the component-wise minimum and maximum (`minps`/`maxps`,
    /// so a NaN coordinate yields the vertex's own value), the centre
    /// `(low + high)·0.5`, the half extents `high − centre` held to at
    /// least 1e-4 cm and then 0.001 cm, and the radius as the farthest
    /// vertex from the centre (`maxss` over the squared distances). The
    /// radius is square-rooted above `0x33FFFFD7` cm²; at or under it the
    /// squared value (in cm²) stands for the radius in centimetres, as in
    /// the original. Positions are in metres, so the floors are the
    /// original's centimetre values scaled. A mesh without vertices has a
    /// zero centre and 0.001 cm extents and radius.
    pub fn bounds(&self) -> MeshBounds {
        if self.positions.is_empty() {
            return MeshBounds {
                centre: [0.0; 3],
                half_extents: [BOUNDS_HALF_FLOOR; 3],
                radius: BOUNDS_HALF_FLOOR,
            };
        }
        let mut low = self.positions[0];
        let mut high = self.positions[0];
        for position in &self.positions[1..] {
            for axis in 0..3 {
                low[axis] = minss(low[axis], position[axis]);
                high[axis] = maxss(high[axis], position[axis]);
            }
        }
        let mut centre = [0.0f32; 3];
        let mut half_extents = [0.0f32; 3];
        for axis in 0..3 {
            centre[axis] = (low[axis] + high[axis]) * 0.5;
            let half = maxss(high[axis] - centre[axis], BOUNDS_BOX_FLOOR);
            half_extents[axis] = if half < BOUNDS_HALF_FLOOR {
                BOUNDS_HALF_FLOOR
            } else {
                half
            };
        }
        let mut radius_squared = 0.0f32;
        for position in &self.positions {
            let dx = position[0] - centre[0];
            let dy = position[1] - centre[1];
            let dz = position[2] - centre[2];
            radius_squared = maxss(radius_squared, (dx * dx + dy * dy) + dz * dz);
        }
        // The floor and the unrooted fallback are the original's in cm.
        const CM2_PER_M2: f32 = 1.0 / (CENTIMETRES_TO_METRES * CENTIMETRES_TO_METRES);
        let squared_cm2 = radius_squared * CM2_PER_M2;
        let radius = if squared_cm2 > RADIUS_SQUARED_FLOOR_CM2 {
            radius_squared.sqrt()
        } else {
            squared_cm2 * CENTIMETRES_TO_METRES
        };
        MeshBounds {
            centre,
            half_extents,
            radius,
        }
    }

    /// Reorders the triangle list by material group and rebuilds the group
    /// ranges, as the original's regroup does after a merge. `group_of_triangle`
    /// gives each triangle's group, indexing `self.groups`; groups are emitted
    /// in that index order and empty ones are dropped. Per-triangle flags and
    /// edges follow their triangle. Vertices are left in place; the render
    /// form does not need the editable form's vertex rebasing.
    pub fn regroup_by_material(&mut self, group_of_triangle: &[u32]) -> crate::Result<()> {
        let triangles = self.indices.len() / 3;
        let malformed = |message: &str| crate::Error::Malformed(message.to_string());
        if !self.indices.len().is_multiple_of(3) {
            return Err(malformed("index count is not a multiple of 3"));
        }
        if group_of_triangle.len() != triangles {
            return Err(malformed("material list does not match the triangles"));
        }
        if let Some(flags) = &self.triangle_flags
            && flags.len() != triangles
        {
            return Err(malformed("triangle flags do not match the triangles"));
        }
        if let Some(edges) = &self.triangle_edges
            && edges.len() != triangles
        {
            return Err(malformed("triangle edges do not match the triangles"));
        }
        let mut order: Vec<usize> = (0..triangles).collect();
        order.sort_by_key(|&triangle| group_of_triangle[triangle]);
        let mut indices = Vec::with_capacity(self.indices.len());
        for &triangle in &order {
            indices.extend_from_slice(&self.indices[triangle * 3..triangle * 3 + 3]);
        }
        if let Some(flags) = &mut self.triangle_flags {
            let old = flags.clone();
            *flags = order.iter().map(|&t| old[t]).collect();
        }
        if let Some(edges) = &mut self.triangle_edges {
            let old = edges.clone();
            *edges = order.iter().map(|&t| old[t]).collect();
        }
        let mut groups = Vec::new();
        let mut start = 0usize;
        for (group, template) in self.groups.iter().enumerate() {
            let count = group_of_triangle
                .iter()
                .filter(|&&g| g as usize == group)
                .count();
            if count == 0 {
                continue;
            }
            groups.push(MeshGroup {
                material: template.material.clone(),
                first_index: (start * 3) as u32,
                index_count: (count * 3) as u32,
            });
            start += count;
        }
        if start != triangles {
            return Err(malformed("a triangle names a missing material group"));
        }
        self.indices = indices;
        self.groups = groups;
        Ok(())
    }

    /// The triangle's material group, from the group ranges.
    fn group_of(&self, triangle: usize) -> Option<usize> {
        let at = (triangle * 3) as u32;
        self.groups
            .iter()
            .position(|group| at >= group.first_index && at < group.first_index + group.index_count)
    }

    /// Appends `other` to this mesh, as the original assembles parts and
    /// pieces into one mesh: vertices and triangles are appended, triangles
    /// whose materials match join the existing group (a new group is made
    /// for a material this mesh does not have yet), and the per-vertex
    /// streams and the per-triangle flags and edges follow. The result is
    /// regrouped by material, as the original's regroup does after an
    /// assembly. The mesh's GI points are not touched (they belong to the
    /// node, not to the merge).
    ///
    /// This is the render-form equivalent of the original's assembly: it
    /// works on unified vertices and streams where the original appends the
    /// editable channels and the per-group GPU buffers. Both meshes must be
    /// in the same node space (the original transforms the appended part
    /// from its node into the destination's, which a caller merging two
    /// nodes must do before calling this).
    ///
    /// A stream only one side has is joined the way the original joins its
    /// channels. A channel the destination lacks is made, padded for the
    /// destination's vertices with white (colours) or zero (skin weights),
    /// and the destination's texture coordinates take the part's first
    /// element; a part without texture coordinates uses the destination's
    /// first element. Triangle flags missing on a side join as 1 (smoothing
    /// group 1 only) and edge bits as 7. A part without colours or skin
    /// weights leaves the destination's channel short in the original; here
    /// its vertices get white colours and zero weights. Smoothing keys are
    /// shifted so the two meshes' groups stay apart; the original copies
    /// the face smoothing bits unchanged and rebuilds the key channel from
    /// them. The destination's render flags are kept.
    ///
    /// A malformed input (a triangle index or group range outside the
    /// arrays, or an attribute array whose length does not match the
    /// vertices) returns an error and leaves the mesh unchanged.
    pub fn merge(&mut self, other: &Mesh) -> crate::Result<()> {
        let malformed = |message: &str| crate::Error::Malformed(message.to_string());
        self.validate()?;
        other.validate()?;
        if other.positions.is_empty() {
            return Ok(());
        }
        if !self.indices.is_empty() && self.groups.is_empty() {
            return Err(malformed(
                "the destination triangles have no material group",
            ));
        }
        if !other.indices.is_empty() && other.groups.is_empty() {
            return Err(malformed("the appended triangles have no material group"));
        }
        if self.positions.is_empty() {
            let mut merged = other.clone();
            merged.render_flags = self.render_flags;
            merged.gi_points = std::mem::take(&mut self.gi_points);
            *self = merged;
            return Ok(());
        }

        let base_vertex = self.positions.len() as u32;
        let base_position = self.position_count.max(self.positions.len()) as u32;
        let base_uv = self.uv_count.max(self.uvs.as_ref().map_or(0, Vec::len)) as u32;
        let smoothing_shift = self.smoothing.iter().copied().max().map_or(0, |m| m + 1);

        // Find or make the group each of the other mesh's groups joins.
        let mut group_map = Vec::with_capacity(other.groups.len());
        for group in &other.groups {
            match self
                .groups
                .iter()
                .position(|existing| existing.material == group.material)
            {
                Some(index) => group_map.push(index as u32),
                None => {
                    self.groups.push(MeshGroup {
                        material: group.material.clone(),
                        first_index: 0,
                        index_count: 0,
                    });
                    group_map.push((self.groups.len() - 1) as u32);
                }
            }
        }

        // Per-triangle destination group, before the triangles move.
        let triangles = self.indices.len() / 3;
        let mut group_of_triangle: Vec<u32> = (0..triangles)
            .map(|t| self.group_of(t).map(|g| g as u32).unwrap_or(0))
            .collect();
        for triangle in 0..other.indices.len() / 3 {
            let source = other.group_of(triangle).unwrap_or(0);
            group_of_triangle.push(group_map[source]);
        }

        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.indices
            .extend(other.indices.iter().map(|&i| i + base_vertex));
        self.position_ids
            .extend(other.position_ids.iter().map(|&i| i + base_position));
        self.position_count =
            base_position as usize + other.position_count.max(other.positions.len());

        // The texture-coordinate stream has its own index map. A side
        // without it gets index 0 for its triangles, which names element 0
        // of the joined stream: the other side's first element.
        let first_uv = |mesh: &Mesh| {
            mesh.uvs.as_ref().map_or([0.0; 2], |uvs| {
                mesh.uv_ids
                    .iter()
                    .position(|&id| id == 0)
                    .map_or([0.0; 2], |v| uvs[v])
            })
        };
        let (mine_first, theirs_first) = (first_uv(self), first_uv(other));
        match (&mut self.uvs, &other.uvs) {
            (Some(mine), Some(theirs)) => {
                mine.extend_from_slice(theirs);
                self.uv_ids
                    .extend(other.uv_ids.iter().map(|&i| i + base_uv));
                self.uv_count = base_uv as usize + other.uv_count.max(other.uv_ids.len());
            }
            (None, Some(theirs)) => {
                let mut uvs = vec![theirs_first; base_vertex as usize];
                uvs.extend_from_slice(theirs);
                self.uvs = Some(uvs);
                self.uv_ids = vec![0; base_vertex as usize];
                self.uv_ids.extend_from_slice(&other.uv_ids);
                self.uv_count = other.uv_count.max(other.uv_ids.len());
            }
            (Some(mine), None) => {
                mine.extend(std::iter::repeat_n(mine_first, other.positions.len()));
                self.uv_ids
                    .extend(std::iter::repeat_n(0, other.positions.len()));
            }
            (None, None) => {}
        }

        match (&mut self.colors, &other.colors) {
            (Some(mine), Some(theirs)) => mine.extend_from_slice(theirs),
            (None, Some(theirs)) => {
                self.colors = Some(vec![[255; 4]; self.positions.len() - other.positions.len()]);
                self.colors
                    .as_mut()
                    .expect("just set")
                    .extend_from_slice(theirs);
            }
            (Some(mine), None) => mine.extend(std::iter::repeat_n([255; 4], other.positions.len())),
            (None, None) => {}
        }

        let zero_weights = SkinWeights {
            bones: [0; 4],
            weights: [0.0; 4],
        };
        match (&mut self.skin, &other.skin) {
            (Some(mine), Some(theirs)) => mine.extend_from_slice(theirs),
            (None, Some(theirs)) => {
                self.skin = Some(vec![
                    zero_weights;
                    self.positions.len() - other.positions.len()
                ]);
                self.skin
                    .as_mut()
                    .expect("just set")
                    .extend_from_slice(theirs);
            }
            (Some(mine), None) => {
                mine.extend(std::iter::repeat_n(zero_weights, other.positions.len()))
            }
            (None, None) => {}
        }

        self.smoothing
            .extend(other.smoothing.iter().map(|&k| k + smoothing_shift));
        match (&mut self.triangle_flags, &other.triangle_flags) {
            (Some(mine), Some(theirs)) => mine.extend_from_slice(theirs),
            (None, Some(theirs)) => {
                self.triangle_flags = Some(vec![1; triangles]);
                self.triangle_flags
                    .as_mut()
                    .expect("just set")
                    .extend_from_slice(theirs);
            }
            (Some(mine), None) => mine.extend(std::iter::repeat_n(1, other.indices.len() / 3)),
            (None, None) => {}
        }
        match (&mut self.triangle_edges, &other.triangle_edges) {
            (Some(mine), Some(theirs)) => mine.extend_from_slice(theirs),
            (None, Some(theirs)) => {
                self.triangle_edges = Some(vec![7; triangles]);
                self.triangle_edges
                    .as_mut()
                    .expect("just set")
                    .extend_from_slice(theirs);
            }
            (Some(mine), None) => mine.extend(std::iter::repeat_n(7, other.indices.len() / 3)),
            (None, None) => {}
        }
        self.regroup_by_material(&group_of_triangle)
    }

    /// Checks the mesh's ranges and attribute arrays, as [`Mesh::merge`]
    /// needs them.
    fn validate(&self) -> crate::Result<()> {
        let malformed = |message: &str| crate::Error::Malformed(message.to_string());
        let vertices = self.positions.len();
        if !self.indices.len().is_multiple_of(3) {
            return Err(malformed("index count is not a multiple of 3"));
        }
        if self.normals.len() != vertices {
            return Err(malformed("normals do not match the vertices"));
        }
        if self.position_ids.len() != vertices {
            return Err(malformed("position ids do not match the vertices"));
        }
        if let Some(uvs) = &self.uvs
            && (uvs.len() != vertices || self.uv_ids.len() != vertices)
        {
            return Err(malformed("texture coordinates do not match the vertices"));
        }
        if let Some(colors) = &self.colors
            && colors.len() != vertices
        {
            return Err(malformed("vertex colours do not match the vertices"));
        }
        if let Some(skin) = &self.skin
            && skin.len() != vertices
        {
            return Err(malformed("skin weights do not match the vertices"));
        }
        if !self.smoothing.is_empty() && self.smoothing.len() != vertices {
            return Err(malformed("smoothing keys do not match the vertices"));
        }
        if let Some(&bad) = self.indices.iter().find(|&&i| i as usize >= vertices) {
            return Err(malformed(&format!(
                "triangle index {bad} out of {vertices} vertices"
            )));
        }
        let triangles = self.indices.len() / 3;
        if self.groups.iter().any(|g| {
            g.first_index % 3 != 0
                || u64::from(g.first_index) + u64::from(g.index_count) > self.indices.len() as u64
        }) {
            return Err(malformed("material group range is outside the triangles"));
        }
        if let Some(flags) = &self.triangle_flags
            && flags.len() != triangles
        {
            return Err(malformed("triangle flags do not match the triangles"));
        }
        if let Some(edges) = &self.triangle_edges
            && edges.len() != triangles
        {
            return Err(malformed("triangle edges do not match the triangles"));
        }
        Ok(())
    }

    /// Works the normals out again from the positions, as at load: the
    /// angle-weighted face normals summed over the vertices sharing a
    /// smoothing key. A morphed mesh takes its normals from its new shape.
    pub fn recompute_normals(&mut self) {
        let n = self.positions.len();
        let own: Vec<u32>;
        let keys = if self.smoothing.len() == n {
            &self.smoothing
        } else if self.position_ids.len() == n {
            &self.position_ids
        } else {
            own = (0..n as u32).collect();
            &own
        };
        // Triangles naming a missing vertex are left out, with their flags.
        let flags = self
            .triangle_flags
            .as_ref()
            .filter(|flags| flags.len() * 3 == self.indices.len());
        let mut indices = Vec::with_capacity(self.indices.len());
        let mut kept_flags = Vec::new();
        for (face, t) in self.indices.as_chunks::<3>().0.iter().enumerate() {
            if t.iter().all(|&i| (i as usize) < n) {
                indices.extend_from_slice(t);
                if let Some(flags) = flags {
                    kept_flags.push(flags[face]);
                }
            }
        }
        let face_flags = flags.map(|_| kept_flags.as_slice());
        let own_elements: Vec<u32>;
        let elements = if self.position_ids.len() == n {
            &self.position_ids
        } else {
            own_elements = (0..n as u32).collect();
            &own_elements
        };
        self.normals = smooth_normals(&self.positions, &indices, keys, elements, face_flags);
    }

    /// Rebuilds the normals the way the cloth pass does: each smoothing
    /// group's normal is the unweighted sum of its faces' unit normals,
    /// walked in the original's incidence order, normalised. A group no
    /// face reaches keeps +Y where the original leaves a zero normal (a
    /// labelled safety deviation, as in [`Mesh::recompute_normals`]).
    pub fn recompute_flat_normals(&mut self) {
        let n = self.positions.len();
        let own_keys: Vec<u32>;
        let keys = if self.smoothing.len() == n {
            &self.smoothing
        } else if self.position_ids.len() == n {
            &self.position_ids
        } else {
            own_keys = (0..n as u32).collect();
            &own_keys
        };
        let own_elements: Vec<u32>;
        let elements = if self.position_ids.len() == n {
            &self.position_ids
        } else {
            own_elements = (0..n as u32).collect();
            &own_elements
        };
        let mut indices = Vec::with_capacity(self.indices.len());
        for t in self.indices.as_chunks::<3>().0 {
            if t.iter().all(|&i| (i as usize) < n) {
                indices.extend_from_slice(t);
            }
        }
        self.normals = flat_face_normals(&self.positions, &indices, keys, elements);
    }
}

/// A level's local database: sub-chunks for its objects, strings and
/// placed objects.
pub const LOCAL_DATABASE: u32 = 0xDBC0;
const LOCAL_OBJECTS: u32 = 0xDBDB;
const LOCAL_STRINGS: u32 = 0xDBCD;

const PLACED_OBJECTS: u32 = 0xDBCE;
const PLACED_OBJECT: u32 = 0xDB01;

/// An object-database entry placed in a level.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedObject {
    /// `0x80000000`: the level's own database; `0x40000000`: the world
    /// database; otherwise `objdb.rdb`. The low 30 bits are the record id.
    pub id: u32,
    /// The placement flags word: condition in bits 16–23; bit `0x10` sets
    /// node flag `0x80000` on the object's root.
    pub flags: u32,
    /// The object's own flags word: its bits under `0xB80000` are copied onto
    /// its root node (`0x80000` marks it level geometry; the rest are not
    /// decoded).
    pub object_flags: u32,
    /// Row-major 3×4: three rotation rows, then the translation (metres).
    pub transform: [[f32; 3]; 4],
    /// Slot in a containing object (0 in the world).
    pub slot: i32,
    /// The word after the slot, which the original copies under the object
    /// node's `0xB80000` flags (only its placement bit matters in Exarust).
    pub second_word: u32,
    /// The extra word a version `0x3D22` file stores after the second word.
    pub word_3d22: Option<u32>,
    /// What the placement changes in the object's own nodes, as for a prop
    /// ([`crate::level::PropAttachment`]).
    pub attachments: Vec<crate::level::PropAttachment>,
}

impl PlacedObject {
    pub const ID_MASK: u32 = 0x3FFF_FFFF;
    /// The placement flags bit that marks the object's root node as level
    /// geometry (the flag is not spread to the nodes under it).
    pub const STATIC: u32 = 0x10;
    /// The object flag that marks the root node level geometry.
    pub const OBJECT_STATIC: u32 = 0x8_0000;

    pub fn is_local(&self) -> bool {
        self.id & 0x8000_0000 != 0
    }

    /// Whether the object's root node is level geometry: the placement's
    /// static bit or the object's own static flag, either of which the
    /// original turns into node flag `0x80000` on its root (and on no other
    /// node).
    pub fn is_static(&self) -> bool {
        self.flags & Self::STATIC != 0 || self.object_flags & Self::OBJECT_STATIC != 0
    }
}

/// A level's local database; `placed` reads its placed objects too (the
/// database-only entry point leaves them).
fn read_local_database(b: &[u8], version: u16, placed: bool) -> Result<LocalDatabase> {
    let mut db = LocalDatabase::default();
    let mut pos = 0;
    while pos + 8 <= b.len() {
        let tag = read_u32(b, pos)?;
        let size = read_u32(b, pos + 4)? as usize;
        let body = b.get(pos + 8..pos + 8 + size).ok_or(Error::Truncated {
            offset: pos + 8,
            needed: size,
            len: b.len(),
        })?;
        match tag {
            LOCAL_OBJECTS => db.items = crate::objdb::items(&Database::parse(body)?),
            LOCAL_STRINGS => match Library::parse(body) {
                Ok(library) => db.strings = Some(library),
                Err(e) => db.string_error = Some(e.to_string()),
            },
            PLACED_OBJECTS if placed => db.placed_objects = read_placed_objects(body, version)?,
            _ => {}
        }
        pos += 8 + size;
    }
    Ok(db)
}

/// The sections of a level's local database (`0xDBC0`): its items, its
/// string table and the objects it places.
#[derive(Debug, Clone, Default)]
pub struct LocalDatabase {
    pub items: Vec<Item>,
    pub strings: Option<Library>,
    /// Why a stored string table did not decode; the original's loader
    /// reports it and leaves the table out, and the rest of the database
    /// still loads.
    pub string_error: Option<String>,
    pub placed_objects: Vec<PlacedObject>,
}

fn read_placed_objects(b: &[u8], version: u16) -> Result<Vec<PlacedObject>> {
    let mut c = Cursor::new(b);
    let count = c.u32()? as usize;
    let mut out = Vec::with_capacity(count.min(65_536));
    while c.remaining() >= 8 {
        let (tag, mut o) = c.chunk()?;
        if tag != PLACED_OBJECT {
            continue;
        }
        let id = o.u32()?;
        let flags = if version >= 0x3D23 { o.u32()? } else { 0 };
        let object_flags = o.u32()?;
        let mut transform = [[0.0f32; 3]; 4];
        for row in &mut transform {
            for v in row.iter_mut() {
                *v = o.f32()?;
            }
        }
        for v in &mut transform[3] {
            *v *= CENTIMETRES_TO_METRES;
        }
        let slot = o.u32()? as i32;
        let second_word = o.u32()?;
        // Version 0x3D22 stores one more word before the sub-chunks.
        let word_3d22 = if version == 0x3D22 {
            Some(o.u32()?)
        } else {
            None
        };
        let mut attachments = Vec::new();
        while o.remaining() >= 8 {
            let Ok((tag, sub)) = o.chunk() else { break };
            if let Some(a) = level::read_attachment(tag, sub.b) {
                attachments.push(a);
            }
        }
        out.push(PlacedObject {
            id,
            flags,
            object_flags,
            transform,
            slot,
            second_word,
            word_3d22,
            attachments,
        });
    }
    Ok(out)
}

/// Per-triangle flag sub-chunk of the triangle list.
pub const TRIANGLE_FLAGS: u32 = 0x3DC3;
/// Per-triangle edge-bit sub-chunk of the triangle list (one byte each).
pub const TRIANGLE_EDGES: u32 = 0x3DC2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinWeights {
    pub bones: [u32; 4],
    pub weights: [f32; 4],
}

/// A run of triangles sharing one material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshGroup {
    pub material: String,
    /// Offset into [`Mesh::indices`].
    pub first_index: u32,
    pub index_count: u32,
}

/// A node sub-chunk handler, as the original's registry holds them: the
/// tag, the chunk's body, and whether the handler took it.
pub type ChunkHandler = fn(u32, &[u8]) -> bool;

/// The registered node sub-chunk handlers, in registration order. The
/// original appends a tag and a callback per registration; in the reader
/// mode that uses them, every node's sub-chunks go to the first handler of
/// their tag instead of the reader's own sub-chunk loop (root chunks never
/// reach it). Exarust's game registers none, so `Scene::parse` reads as
/// before.
#[derive(Default)]
pub struct ChunkHandlers {
    entries: Vec<(u32, ChunkHandler)>,
}

impl ChunkHandlers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether no handler is registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Registers a handler. A tag registered twice keeps both, the first
    /// one winning, as the original's appended list does.
    pub fn register(&mut self, tag: u32, handler: ChunkHandler) {
        self.entries.push((tag, handler));
    }

    /// Hands a chunk to the first handler registered for its tag; false
    /// when none takes it.
    pub fn dispatch(&self, tag: u32, body: &[u8]) -> bool {
        self.entries
            .iter()
            .find(|(known, _)| *known == tag)
            .is_some_and(|(_, handler)| handler(tag, body))
    }
}

impl Scene {
    /// Parses a file with the reader's own sub-chunk loop.
    pub fn parse(b: &[u8]) -> Result<Self> {
        Self::parse_with(b, &ChunkHandlers::new())
    }

    /// Parses a file; with handlers registered, every node sub-chunk goes to
    /// the first handler of its tag in place of the reader's own sub-chunk
    /// loop (the original's registry mode). An empty registry reads as
    /// [`Scene::parse`].
    pub fn parse_with(b: &[u8], handlers: &ChunkHandlers) -> Result<Self> {
        let mut c = Cursor::new(b);
        if c.u16()? != SANITY {
            return Err(Error::Malformed(
                "not a chunk tree (bad sanity word)".into(),
            ));
        }
        let version = c.u16()?;
        let mut nodes = Vec::new();
        let mut materials = Vec::new();
        let mut grid = None;
        let mut props = Vec::new();
        let mut environment = None;
        let mut characters = None;
        let mut local_items = Vec::new();
        let mut local_strings = None;
        let mut local_string_error = None;
        let mut placed_objects = Vec::new();
        let mut unknown_chunks = Vec::new();
        let mut level_errors = Vec::new();
        while c.remaining() >= 8 {
            let (tag, mut chunk) = c.chunk()?;
            if tag == level::GRID {
                match level::read_grid(chunk.b) {
                    Ok(g) => grid = Some(g),
                    Err(e) => level_errors.push(format!("grid: {e}")),
                }
            } else if tag == level::PROPS {
                match level::read_props(chunk.b) {
                    Ok(p) => props.extend(p),
                    Err(e) => level_errors.push(format!("props: {e}")),
                }
            } else if tag == CHARACTERS {
                match LevelCharacters::parse(chunk.b) {
                    Ok(c) => characters = Some(c),
                    Err(e) => level_errors.push(format!("characters: {e}")),
                }
            } else if tag == LOCAL_DATABASE {
                match read_local_database(chunk.b, version, true) {
                    Ok(db) => {
                        local_items = db.items;
                        local_strings = db.strings;
                        local_string_error = db.string_error;
                        placed_objects = db.placed_objects;
                    }
                    Err(e) => level_errors.push(format!("local database: {e}")),
                }
            } else if tag == ENVIRONMENT {
                match Environment::parse(chunk.b) {
                    Ok(e) => environment = Some(e),
                    Err(e) => level_errors.push(format!("environment: {e}")),
                }
            } else if tag == MATERIAL_LIST {
                if let Err(e) = read_materials(&mut chunk, &mut materials) {
                    materials.push((String::new(), Err(format!("material list: {e}"))));
                }
            } else if tag == NODE_LIST {
                let count = chunk.count(8)?;
                nodes.reserve(count);
                for _ in 0..count {
                    let (kind, node) = chunk.chunk()?;
                    nodes.push(read_node(kind, node, version, handlers)?);
                }
            } else {
                unknown_chunks.push(tag);
            }
        }
        Ok(Self {
            version,
            nodes,
            materials,
            grid,
            props,
            environment,
            characters,
            local_items,
            local_strings,
            local_string_error,
            placed_objects,
            unknown_chunks,
            level_errors,
        })
    }

    /// Reads only a level's local database records and string table and
    /// leaves the rest of the file alone: the original's database-only entry
    /// point, for reading a level's items without building its scene. It
    /// walks every root `0xDBC0` chunk and takes their `0xDBDB` records and
    /// `0xDBCD` strings; placed objects (`0xDBCE`) need the scene, so it
    /// skips them.
    pub fn parse_database(b: &[u8]) -> Result<LocalDatabase> {
        let mut c = Cursor::new(b);
        if c.u16()? != SANITY {
            return Err(Error::Malformed(
                "not a chunk tree (bad sanity word)".into(),
            ));
        }
        let version = c.u16()?;
        let mut db = LocalDatabase::default();
        while c.remaining() >= 8 {
            let (tag, chunk) = c.chunk()?;
            if tag == LOCAL_DATABASE {
                let LocalDatabase {
                    items,
                    strings,
                    string_error,
                    placed_objects: _,
                } = read_local_database(chunk.b, version, false)?;
                if !items.is_empty() {
                    db.items = items;
                }
                if strings.is_some() || string_error.is_some() {
                    (db.strings, db.string_error) = (strings, string_error);
                }
            }
        }
        Ok(db)
    }

    pub fn material(&self, name: &str) -> Option<&Material> {
        self.materials
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .and_then(|(_, m)| m.as_ref().ok())
    }

    pub fn meshes(&self) -> impl Iterator<Item = (&Node, &Mesh)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(move |(i, node)| self.mesh_of(i).map(|m| (node, m)))
    }

    /// The mesh a node draws: its own, or for an instance node the mesh of
    /// the node its `instance` word names (1-based), following instance
    /// chains. The original's instance node shares its owner's geometry and
    /// has no body of its own.
    pub fn mesh_of(&self, index: usize) -> Option<&Mesh> {
        let mut at = index;
        for _ in 0..=self.nodes.len() {
            match &self.nodes.get(at)?.body {
                NodeBody::Mesh(m) => return Some(m.as_ref()),
                NodeBody::Instance(_) => {
                    let owner = self.nodes[at].instance?.checked_sub(1)?;
                    if owner >= self.nodes.len() || owner == at {
                        return None;
                    }
                    at = owner;
                }
                _ => return None,
            }
        }
        None
    }
}

fn read_materials(
    c: &mut Cursor<'_>,
    out: &mut Vec<(String, std::result::Result<Material, String>)>,
) -> Result<()> {
    let count = c.count(24)?;
    for _ in 0..count {
        let (_, mut record) = c.chunk()?;
        let name = record.name()?;
        let payload = record.bytes(record.remaining())?;
        out.push((name, Material::parse(payload).map_err(|e| e.to_string())));
    }
    Ok(())
}

/// Sub-chunk of GI sample points after a mesh body: `u32 spacing (cm),
/// u32 count, count × f32[3]` object-space points (cm).
pub const GI_POINTS: u32 = 0x3D0C_EC05;
/// Sub-chunk of a collision body after a mesh body: the node is a rigid
/// body (`rigid_body::RigidBody`).
pub const RIGID_BODY: u32 = 0x3DD0_B001;
/// Sub-chunk marking a mesh node as cloth (one word, 0 in every shipped
/// file).
pub const CLOTH: u32 = 0x3DD0_C001;
/// The scene flag under which a cloth marker gives its node a cloth
/// controller: the game sets it when it creates the scene the node is
/// loaded into (so every cloth-marked node in play gets one).
pub const CLOTH_SCENE: u32 = 0x1_0000;

/// The cloth importer's gate: a marked node in a scene with the cloth flag
/// gets a controller; elsewhere the marker is ignored (the original also
/// clears two node flag bits).
pub fn cloth_import(marker: bool, scene_flags: u32) -> bool {
    marker && scene_flags & CLOTH_SCENE != 0
}
/// Sub-chunk of a keyframe track (`key_track::KeyTrack`), which may follow
/// any stored node's body.
pub const KEY_TRACK: u32 = 0x3AC1;
/// Sub-chunk of a morph set (`morph::MorphSet`) after a mesh body: the
/// physique targets of a race's morph body.
pub const MORPH: u32 = 0xEDC0_C589;
/// Light body flags under which the body ends with four more words: one
/// skipped, then the flicker and fade parameters ([`LightAnimation`]).
const LIGHT_EXTRA_FLAGS: u32 = 0x3_0000;
/// A camera node (`0x3D08`): its body is the node its view follows (a
/// 1-based index) and one word.
const NODE_3D08: u32 = 0x3D08;
/// The node word's deprecated fading bit, which the mesh reader replaces by
/// [`FADING`].
const DEPRECATED_FADING: u32 = 0x8000_0000;
/// The fading bit that replaces [`DEPRECATED_FADING`].
const FADING: u32 = 0x0100_0000;
/// A sector node (`0x3D0A`): no body.
const NODE_SECTOR: u32 = 0x3D0A;

/// Sub-chunk tags after a node's body that the reader knows.
const KNOWN_SUB_CHUNKS: [u32; 5] = [RIGID_BODY, CLOTH, KEY_TRACK, MORPH, GI_POINTS];

/// What the sub-chunks after a node's body hold.
#[derive(Default)]
struct SubChunks {
    /// GI sample points (none when the chunk is absent or malformed).
    gi_points: Vec<[f32; 3]>,
    rigid_body: Option<std::result::Result<RigidBody, String>>,
    cloth_marker: bool,
    key_tracks: Vec<std::result::Result<KeyTrack, String>>,
    /// The first morph set's payload, decoded once the mesh's position
    /// count is known.
    morph: Option<Vec<u8>>,
    /// Tags of the sub-chunks this reader does not know.
    unknown: Vec<u32>,
}

fn read_sub_chunks(c: &mut Cursor<'_>, handlers: &ChunkHandlers) -> SubChunks {
    let mut out = SubChunks::default();
    let mut points = None;
    while c.remaining() >= 8 {
        let Ok((tag, mut sub)) = c.chunk() else {
            break;
        };
        if !handlers.is_empty() {
            handlers.dispatch(tag, sub.b);
            continue;
        }
        if !KNOWN_SUB_CHUNKS.contains(&tag) {
            out.unknown.push(tag);
        }
        if tag == RIGID_BODY && out.rigid_body.is_none() {
            out.rigid_body = Some(RigidBody::parse(sub.b).map_err(|e| e.to_string()));
        }
        if tag == CLOTH {
            out.cloth_marker = true;
        }
        if tag == KEY_TRACK {
            out.key_tracks
                .push(KeyTrack::parse(sub.b).map_err(|e| e.to_string()));
        }
        if tag == MORPH && out.morph.is_none() {
            out.morph = Some(sub.b.to_vec());
        }
        if tag != GI_POINTS || points.is_some() {
            continue;
        }
        let (Ok(_spacing), Ok(count)) = (sub.u32(), sub.u32()) else {
            break;
        };
        let count = count as usize;
        if sub.remaining() < count * 12 {
            break;
        }
        points = Some(
            (0..count)
                .filter_map(|_| {
                    let p = [sub.f32().ok()?, sub.f32().ok()?, sub.f32().ok()?];
                    Some(p.map(|v| v * CENTIMETRES_TO_METRES))
                })
                .collect(),
        );
    }
    out.gi_points = points.unwrap_or_default();
    out
}

/// Emitter records end a plain node's body: `u32 type, u32 4, f32 size`.
const EMITTER: u32 = 0x3DD0_E000;
pub const FIRE: u32 = 0x3DD0_E002;
/// The level portal's marker (`_PortalPrt` in `uwportal01`).
pub const PORTAL: u32 = 0x3DD0_E004;

fn emitter(body: &[u8]) -> Option<NodeBody> {
    let tail = body.len().checked_sub(12).map(|at| &body[at..])?;
    let word = |o: usize| u32::from_le_bytes(tail[o..o + 4].try_into().expect("4 bytes"));
    let kind = word(0);
    (kind & 0xFFFF_FF00 == EMITTER && word(4) == 4).then(|| NodeBody::Emitter {
        kind,
        size: f32::from_bits(word(8)),
    })
}

fn read_node(kind: u32, mut c: Cursor<'_>, version: u16, handlers: &ChunkHandlers) -> Result<Node> {
    let name = c.name()?;
    let flags = c.u32()?;
    let mut flags2 = c.u32()?;
    let instance = c.u32()?;
    let parent = c.u32()? as usize;
    let mut transform = [[0.0; 3]; 4];
    for row in &mut transform {
        for v in row.iter_mut() {
            *v = c.f32()?;
        }
    }
    for v in &mut transform[3] {
        *v *= CENTIMETRES_TO_METRES;
    }
    // Bounds: half extents, centre, radius; then one unidentified word.
    let bounds_half_extents = [c.f32()?, c.f32()?, c.f32()?].map(|v| v * CENTIMETRES_TO_METRES);
    let bounds_centre = [c.f32()?, c.f32()?, c.f32()?].map(|v| v * CENTIMETRES_TO_METRES);
    let bounds_radius = c.f32()? * CENTIMETRES_TO_METRES;
    c.skip(4)?;
    // The sub-chunks follow the body of every stored node, whatever its
    // kind; an instance stores nothing more.
    let mut sub = SubChunks::default();
    let mut morph = None;
    let body = if instance != 0 {
        NodeBody::Instance(instance)
    } else if kind == NODE_MESH {
        // The mesh body reader replaces the deprecated fading bit: it logs
        // "Deprecated fading flag", sets bit 24 and clears bit 31. Other
        // kinds, and instances, keep the word as stored.
        if flags2 & DEPRECATED_FADING != 0 {
            flags2 = (flags2 | FADING) & !DEPRECATED_FADING;
        }
        match read_mesh(&mut c, version) {
            Ok(mut m) => {
                sub = read_sub_chunks(&mut c, handlers);
                m.gi_points = std::mem::take(&mut sub.gi_points);
                morph = sub
                    .morph
                    .take()
                    .map(|b| MorphSet::parse(&b, m.position_count).map_err(|e| e.to_string()));
                NodeBody::Mesh(Box::new(m))
            }
            Err(e) => NodeBody::Invalid(e.to_string()),
        }
    } else if kind == NODE_LIGHT {
        match read_light(&mut c, flags2) {
            Ok(mut l) => {
                if l.flags & LIGHT_EXTRA_FLAGS == 0 {
                    sub = read_sub_chunks(&mut c, handlers);
                } else if let Ok(animation) = read_light_animation(&mut c) {
                    l.animation = Some(animation);
                    sub = read_sub_chunks(&mut c, handlers);
                }
                NodeBody::Light(l)
            }
            Err(e) => NodeBody::Invalid(e.to_string()),
        }
    } else if kind == NODE_VOXEL_LIGHT {
        match read_voxel_light(&mut c) {
            Ok(l) => {
                sub = read_sub_chunks(&mut c, handlers);
                NodeBody::VoxelLight(l)
            }
            Err(e) => NodeBody::Invalid(e.to_string()),
        }
    } else if kind == NODE_3D08 {
        let target = c.u32()?;
        let value = c.u32()?;
        sub = read_sub_chunks(&mut c, handlers);
        NodeBody::Camera {
            target: (target > 0).then_some(target as usize),
            value,
        }
    } else if kind == NODE_SECTOR {
        sub = read_sub_chunks(&mut c, handlers);
        NodeBody::Sector
    } else {
        let mut chunks = Cursor::new(c.b);
        chunks.pos = c.pos;
        sub = read_sub_chunks(&mut chunks, handlers);
        emitter(c.bytes(c.remaining())?).unwrap_or(NodeBody::NotMesh)
    };
    Ok(Node {
        kind,
        name,
        flags,
        flags2,
        parent: (parent > 0).then_some(parent),
        transform,
        bounds_centre,
        bounds_half_extents,
        bounds_radius,
        rigid_body: sub.rigid_body,
        cloth_marker: sub.cloth_marker,
        key_tracks: sub.key_tracks,
        morph,
        instance: (instance != 0).then_some(instance as usize),
        unknown_sub_chunks: sub.unknown,
        body,
    })
}

fn read_light(c: &mut Cursor<'_>, node_flags: u32) -> Result<Light> {
    let flags = c.u32()?;
    let color = [c.f32()?, c.f32()?, c.f32()?];
    c.skip(4)?;
    let multiplier = c.f32()?;
    let intensity = c.f32()?;
    let radius = c.f32()? * CENTIMETRES_TO_METRES;
    // One unread value, then the softness.
    c.skip(4)?;
    let softness = c.f32()?;
    let spot = if node_flags & SPOT_MASK == SPOT {
        Some([c.f32()?, c.f32()?])
    } else {
        None
    };
    Ok(Light {
        flags,
        color,
        multiplier,
        intensity,
        radius,
        softness,
        spot,
        ranged: node_flags & RANGED != 0,
        animation: None,
    })
}

/// The flicker and fade words of a light body whose flags ask for them:
/// one skipped word, the amplitude, the speed and the emissive share.
fn read_light_animation(c: &mut Cursor<'_>) -> Result<LightAnimation> {
    c.skip(4)?;
    Ok(LightAnimation {
        amplitude: c.f32()?,
        speed: c.f32()?,
        emissive: c.f32()?,
    })
}

/// A voxel light's body: a word (0 in the corpus), the sRGB colour, an
/// unread value (1), the intensity, the radius in centimetres and a last
/// word (0).
fn read_voxel_light(c: &mut Cursor<'_>) -> Result<VoxelLight> {
    c.skip(4)?;
    let color = [c.f32()?, c.f32()?, c.f32()?];
    c.skip(4)?;
    let intensity = c.f32()?;
    let radius = c.f32()? * CENTIMETRES_TO_METRES;
    c.skip(4)?;
    Ok(VoxelLight {
        color,
        intensity,
        radius,
    })
}

struct Stream<'a> {
    id: u32,
    format: StreamFormat,
    count: usize,
    data: &'a [u8],
}

fn read_mesh(c: &mut Cursor<'_>, version: u16) -> Result<Mesh> {
    let render_flags = if version > VERSION_WITHOUT_MESH_PREFIX {
        c.u32()?
    } else {
        0
    };
    let stream_count = c.count_minus_one(12)?;
    let mut streams = Vec::with_capacity(stream_count);
    for _ in 0..stream_count {
        let id = c.u32()?;
        let word = c.u32()?;
        let count = c.count_minus_one(1)?;
        let format = StreamFormat::from_word(word)
            .ok_or_else(|| Error::Unsupported(format!("vertex stream format {word:#010x}")))?;
        let data = c.bytes(count * format.element_size())?;
        streams.push(Stream {
            id,
            format,
            count,
            data,
        });
    }

    let (tag, mut topo) = c.chunk()?;
    if tag != TOPOLOGY {
        return Err(Error::Malformed(format!(
            "expected topology chunk, got {tag:#x}"
        )));
    }
    topo.skip(4)?;
    let vertex_count = topo.count_minus_one(1)?;
    let wide = vertex_count >= WIDE_INDEX_THRESHOLD;
    let group_count = topo.count_minus_one(28)?;
    let mut materials = Vec::with_capacity(group_count);
    for _ in 0..group_count {
        topo.skip(4 + 8)?;
        materials.push(topo.name()?);
    }
    let map_count = topo.count_minus_one(4)?;
    let mut maps = Vec::with_capacity(map_count);
    for _ in 0..map_count {
        let key = topo.u32()?;
        maps.push((key, topo.indices(vertex_count, wide)?));
    }

    let mut indices = Vec::new();
    let mut tri_ranges = Vec::new();
    let mut triangle_flags = None;
    let mut triangle_edges = None;
    if topo.remaining() > 0 {
        let (tag, mut tris) = topo.chunk()?;
        if tag != TRIANGLES {
            return Err(Error::Malformed(format!(
                "expected triangle chunk, got {tag:#x}"
            )));
        }
        tris.skip(4)?;
        let tri_count = tris.count_minus_one(6)?;
        indices = tris.indices(tri_count * 3, wide)?;
        for _ in 0..group_count {
            tri_ranges.push((tris.u32()? as usize, tris.u32()? as usize));
        }
        while tris.remaining() >= 8 {
            let Ok((tag, mut sub)) = tris.chunk() else {
                break;
            };
            if tag == TRIANGLE_FLAGS && sub.remaining() >= tri_count * 4 {
                triangle_flags = Some((0..tri_count).map(|_| sub.u32()).collect::<Result<_>>()?);
            } else if tag == TRIANGLE_EDGES && sub.remaining() >= tri_count {
                triangle_edges = Some(sub.bytes(tri_count)?.to_vec());
            }
        }
        if let Some(&bad) = indices.iter().find(|&&i| i as usize >= vertex_count) {
            return Err(Error::Malformed(format!(
                "triangle index {bad} out of {vertex_count} vertices"
            )));
        }
    }

    let map = |key: u32| maps.iter().find(|(k, _)| k & 0xFFFF == key).map(|(_, m)| m);
    let stream = |id_low: u32, format: StreamFormat| {
        streams
            .iter()
            .find(|s| s.id & 0xFFFF == id_low && s.format == format)
    };
    let gather = |s: &Stream<'_>, m: &[u32]| -> Result<Vec<usize>> {
        m.iter()
            .map(|&i| {
                let i = i as usize;
                if i < s.count {
                    Ok(i)
                } else {
                    Err(Error::Malformed(format!(
                        "vertex map index {i} out of {} elements",
                        s.count
                    )))
                }
            })
            .collect()
    };

    let pos_stream = stream(1, StreamFormat::Vec3)
        .ok_or_else(|| Error::Malformed("mesh has no position stream".into()))?;
    let pos_map =
        map(KEY_POSITIONS).ok_or_else(|| Error::Malformed("mesh has no position map".into()))?;
    let pos_idx = gather(pos_stream, pos_map)?;
    let positions: Vec<[f32; 3]> = pos_idx
        .iter()
        .map(|&i| vec3(pos_stream.data, i).map(|v| v * CENTIMETRES_TO_METRES))
        .collect();

    let mut uv_ids = Vec::new();
    let mut uv_count = 0;
    let uvs = match (stream(0x10, StreamFormat::Vec2), map(KEY_UVS)) {
        (Some(s), Some(m)) => {
            let idx = gather(s, m)?;
            uv_ids = idx.iter().map(|&i| i as u32).collect();
            uv_count = s.count;
            Some(
                idx.into_iter()
                    .map(|i| {
                        let o = i * 8;
                        [f32_at(s.data, o), f32_at(s.data, o + 4)]
                    })
                    .collect(),
            )
        }
        _ => None,
    };
    let colors = stream(1, StreamFormat::Rgba8)
        .map(|s| -> Result<Vec<[u8; 4]>> {
            Ok(gather(s, pos_map)?
                .into_iter()
                .map(|i| s.data[i * 4..i * 4 + 4].try_into().expect("4 bytes"))
                .collect())
        })
        .transpose()?;
    let skin = stream(1, StreamFormat::Skin)
        .map(|s| -> Result<Vec<SkinWeights>> {
            Ok(gather(s, pos_map)?
                .into_iter()
                .map(|i| {
                    let o = i * 32;
                    SkinWeights {
                        bones: std::array::from_fn(|k| u32_at(s.data, o + k * 4)),
                        weights: std::array::from_fn(|k| f32_at(s.data, o + 16 + k * 4)),
                    }
                })
                .collect())
        })
        .transpose()?;

    let smoothing: Vec<u32> = match map(KEY_SMOOTHING) {
        Some(m) => m.clone(),
        None => pos_idx.iter().map(|&i| i as u32).collect(),
    };
    let element_ids: Vec<u32> = pos_idx.iter().map(|&i| i as u32).collect();
    let normals = smooth_normals(
        &positions,
        &indices,
        &smoothing,
        &element_ids,
        triangle_flags.as_deref(),
    );
    let position_count = pos_stream.count;

    let tri_count = indices.len() / 3;
    let mut groups = Vec::with_capacity(group_count);
    for (material, &(first, last)) in materials.into_iter().zip(&tri_ranges) {
        // An empty group stores `last = first - 1`.
        if last + 1 == first {
            continue;
        }
        if first > last || last >= tri_count {
            return Err(Error::Malformed(format!(
                "group triangle range {first}..={last} out of {tri_count}"
            )));
        }
        groups.push(MeshGroup {
            material,
            first_index: (first * 3) as u32,
            index_count: ((last - first + 1) * 3) as u32,
        });
    }

    Ok(Mesh {
        positions,
        normals,
        uvs,
        colors,
        skin,
        position_ids: pos_idx.iter().map(|&i| i as u32).collect(),
        indices,
        groups,
        triangle_flags,
        triangle_edges,
        gi_points: Vec::new(),
        render_flags,
        position_count,
        smoothing,
        uv_ids,
        uv_count,
    })
}

/// `maxss`: the first operand when it is greater, else the second (so a
/// NaN in either takes the second).
fn maxss(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// `minss`: the first operand when it is smaller, else the second.
fn minss(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// The processor's reciprocal square root estimate (about 12 bits), which
/// the original's vector code uses.
pub fn reciprocal_sqrt(value: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: SSE is part of the x86_64 baseline.
        unsafe {
            use std::arch::x86_64::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
            _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(value)))
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1.0 / value.sqrt()
    }
}

/// The processor's reciprocal estimate (about 12 bits).
fn reciprocal(value: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: SSE is part of the x86_64 baseline.
        unsafe {
            use std::arch::x86_64::{_mm_cvtss_f32, _mm_rcp_ss, _mm_set_ss};
            _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(value)))
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1.0 / value
    }
}

/// `x·y + … ` in the order the original's packed dot sums lanes: x and y,
/// then z.
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}

/// `a × b`, as the original's shuffled packed cross product.
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A face's unit normal as the original's face-plane record makes it: the
/// cross product of its two edges from the first vertex, scaled by the
/// reciprocal square root estimate of its squared length (summed x, y,
/// then z). A degenerate face keeps a zero normal.
fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let cross = cross3(sub(b, a), sub(c, a));
    let squared = dot3(cross, cross);
    let scale = if squared == 0.0 {
        0.0
    } else {
        reciprocal_sqrt(squared)
    };
    cross.map(|x| x * scale)
}

/// The engine's vector normalisation: the squares summed x, z then y and
/// the vector scaled by one reciprocal square root; `None` for a zero
/// vector (which the original leaves zero). The original's root is the
/// processor's estimate, which only scales the length (by under 4e-4) and
/// leaves the direction; Exarust takes the exact one so normals stay unit.
fn normalised(v: [f32; 3]) -> Option<[f32; 3]> {
    let sum = (v[0] * v[0] + v[2] * v[2]) + v[1] * v[1];
    if sum == 0.0 {
        return None;
    }
    let scale = 1.0 / sum.sqrt();
    Some(v.map(|x| x * scale))
}

/// The barycentric coordinates of a point against a triangle, as the
/// original's face helper computes them: the point is first dropped onto
/// the face's plane along the unit face normal, its offset from the first
/// vertex is expressed in the two edge vectors, and the face's stored
/// `1/(d00·d11 − d01²)` is the processor's reciprocal estimate. A
/// degenerate triangle gives infinite or NaN weights.
pub fn barycentric(vertices: [[f32; 3]; 3], point: [f32; 3]) -> [f32; 3] {
    Face::new(vertices[0], vertices[1], vertices[2]).barycentric(point)
}

/// A triangle's precomputed face record, as the mesh class builds it: the
/// unit normal and the plane offset, the corners, the first edge's squared
/// length and the reciprocal of the triangle's squared area. The queries
/// below all work from this record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    /// The unit face normal (the original's reciprocal-square-root
    /// estimate; a degenerate face keeps a zero normal).
    pub normal: [f32; 3],
    /// `normal · corner[0]`: the plane `normal · x = plane`.
    pub plane: f32,
    /// The three corners in winding order.
    pub corners: [[f32; 3]; 3],
    /// `|corner[1] − corner[0]|²`, the record's stored first edge.
    pub edge1_sq: f32,
    /// `1/(|e1|²·|e2|² − (e1·e2)²)`, the processor's reciprocal.
    pub inv_area_sq: f32,
}

/// The smallest value the ray and approach tests accept as a denominator.
const FACE_EPSILON: f32 = 1.192_089_982_282_595_9e-07;
/// The sphere tests' smaller epsilon (the mesh test's).
const SPHERE_EPSILON: f32 = f32::from_bits(0x2EDB_E6FF);

impl Face {
    /// Builds a face record from three corners, as the original's face
    /// helper does: the cross product of the two edges from the first
    /// corner, scaled by one reciprocal square root, and the edges' Gram
    /// terms.
    pub fn new(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Self {
        let e1 = sub(b, a);
        let e2 = sub(c, a);
        let normal = face_normal(a, b, c);
        let edge1_sq = dot3(e1, e1);
        let d12 = dot3(e1, e2);
        Self {
            normal,
            plane: dot3(a, normal),
            corners: [a, b, c],
            edge1_sq,
            inv_area_sq: reciprocal(edge1_sq * dot3(e2, e2) - d12 * d12),
        }
    }

    /// The signed distance of `p` in front of the face's plane.
    pub fn plane_distance(&self, p: [f32; 3]) -> f32 {
        dot3(p, self.normal) - self.plane
    }

    /// The barycentric coordinates of `p`, as the original's face helper
    /// computes them: the point is dropped onto the plane along the normal
    /// and its offset from the first corner expressed in the two edges,
    /// with the record's reciprocal area. A degenerate face gives infinite
    /// or NaN weights.
    pub fn barycentric(&self, p: [f32; 3]) -> [f32; 3] {
        let [a, b, c] = self.corners;
        let e1 = sub(b, a);
        let e2 = sub(c, a);
        let distance = -self.plane_distance(p);
        let on_plane = [
            p[0] + self.normal[0] * distance,
            p[1] + self.normal[1] * distance,
            p[2] + self.normal[2] * distance,
        ];
        let v = sub(on_plane, a);
        let d01 = dot3(e1, e2);
        let d11 = dot3(e2, e2);
        let d20 = dot3(v, e1);
        let d21 = dot3(v, e2);
        let second = (d11 * d20 - d01 * d21) * self.inv_area_sq;
        let third = (self.edge1_sq * d21 - d01 * d20) * self.inv_area_sq;
        [(1.0 - second) - third, second, third]
    }

    /// The point of the triangle closest to `p`, whose signed distance to
    /// the face's plane is `plane_distance`, as the original's closest-point
    /// helper finds it (the interior projection, then the three edges and
    /// corners). Weights are clamped to 0…1 where the original clamps them.
    pub fn closest_point(&self, p: [f32; 3], plane_distance: f32) -> [f32; 3] {
        let [a, b, c] = self.corners;
        let e1 = sub(b, a);
        let e2 = sub(c, a);
        let e2_sq = dot3(e2, e2);
        let d12 = dot3(e1, e2);
        let w = sub(
            [
                p[0] - self.normal[0] * plane_distance,
                p[1] - self.normal[1] * plane_distance,
                p[2] - self.normal[2] * plane_distance,
            ],
            a,
        );
        let d1w = dot3(e1, w);
        let d2w = dot3(e2, w);
        let denom = self.edge1_sq * d2w - d12 * d1w;
        if denom > 0.0 {
            let t2 = d1w * e2_sq - d12 * d2w;
            if t2 > 0.0 {
                if (denom + t2) * self.inv_area_sq <= 1.0 {
                    return [a[0] + w[0], a[1] + w[1], a[2] + w[2]];
                }
            } else if d2w <= e2_sq {
                // The original's `<= 0` clamp keeps a NaN, unlike `max`.
                let v = d2w * reciprocal(e2_sq);
                let v = if v <= 0.0 { 0.0 } else { v };
                return [a[0] + e2[0] * v, a[1] + e2[1] * v, a[2] + e2[2] * v];
            }
        } else {
            if d1w <= 0.0 {
                let v = (d2w * reciprocal(e2_sq)).clamp(0.0, 1.0);
                return [a[0] + e2[0] * v, a[1] + e2[1] * v, a[2] + e2[2] * v];
            }
            if d1w <= self.edge1_sq {
                let v = d1w * reciprocal(self.edge1_sq);
                return [a[0] + e1[0] * v, a[1] + e1[1] * v, a[2] + e1[2] * v];
            }
        }
        // The edge from the second corner to the third.
        let e = sub(c, b);
        let v = (dot3(sub(w, e1), e) * reciprocal(dot3(e, e))).clamp(0.0, 1.0);
        [b[0] + e[0] * v, b[1] + e[1] * v, b[2] + e[2] * v]
    }

    /// The distance along `direction` (unit length) at which a ray from
    /// `origin` meets the face's front, if it does before `best`. This is
    /// the original's ray test: the signed volumes of the ray and the two
    /// edges, one-sided (the ray must run against the face normal), with
    /// the denominator compared against the engine's epsilon.
    pub fn ray(&self, origin: [f32; 3], direction: [f32; 3], best: f32) -> Option<f32> {
        let [a, b, c] = self.corners;
        let e1 = sub(b, a);
        let e2 = sub(c, a);
        let h = cross3(direction, e2);
        let denom = dot3(e1, h);
        // The range rejections are "below"/"above" compares a NaN passes;
        // only the distance checks at the end reject a NaN.
        if denom < FACE_EPSILON {
            return None;
        }
        let s = sub(origin, a);
        let u = dot3(s, h);
        if u < 0.0 || denom < u {
            return None;
        }
        let q = cross3(s, e1);
        let v = dot3(direction, q);
        if v < 0.0 || denom < u + v {
            return None;
        }
        let t = dot3(e2, q) / denom;
        if 0.0 < t && t < best { Some(t) } else { None }
    }

    /// The original's second ray variant: the barycentric weights are
    /// divided by the denominator first and the hit is accepted at
    /// `t == best` too.
    pub fn ray_inclusive(&self, origin: [f32; 3], direction: [f32; 3], best: f32) -> Option<f32> {
        let [a, b, c] = self.corners;
        let e1 = sub(b, a);
        let e2 = sub(c, a);
        let h = cross3(direction, e2);
        let denom = dot3(e1, h);
        if denom.abs() < FACE_EPSILON {
            return None;
        }
        let inverse = 1.0 / denom;
        let s = sub(origin, a);
        let u = dot3(s, h) * inverse;
        if u < 0.0 || u > 1.0 {
            return None;
        }
        let q = cross3(s, e1);
        let v = dot3(direction, q) * inverse;
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = dot3(e2, q) * inverse;
        if 0.0 < t && t <= best { Some(t) } else { None }
    }

    /// The push-out vector of a sphere at `centre` with `radius` that
    /// reaches the face's front, as the original's mesh-side sphere test
    /// reports it: from the centre to the closest point of the triangle,
    /// lengthened to the radius less the distance (or straight along the
    /// normal when the centre is on the triangle).
    #[allow(clippy::neg_cmp_op_on_partial_ord)] // a NaN takes the original's branch
    pub fn sphere_push(&self, centre: [f32; 3], radius: f32) -> Option<[f32; 3]> {
        let distance = self.plane_distance(centre);
        if distance < 0.0 || distance >= radius {
            return None;
        }
        let closest = self.closest_point(centre, distance);
        let mut diff = sub(centre, closest);
        let squared = dot3(diff, diff);
        let room = radius - SPHERE_EPSILON;
        if room * room <= squared {
            return None;
        }
        if !(squared < SPHERE_EPSILON) {
            let scale = (radius - squared.sqrt()) / squared.sqrt();
            diff = diff.map(|x| x * scale);
        } else {
            diff = self.normal.map(|x| x * radius);
        }
        Some(diff)
    }

    /// The original's "a point is within `max` of the face" test, used by
    /// the kd-tree's sphere walks: the point's plane distance must be
    /// within the reach and its squared distance to the closest point under
    /// `max² − the engine's epsilon`.
    pub fn within(&self, point: [f32; 3], max: f32) -> Option<f32> {
        let distance = self.plane_distance(point);
        if distance.abs() > max {
            return None;
        }
        let closest = self.closest_point(point, distance);
        let v = sub(point, closest);
        let squared = dot3(v, v);
        if squared < max * max - FACE_EPSILON {
            Some(squared.sqrt())
        } else {
            None
        }
    }

    /// The distance along a sphere's sweep at which it first touches the
    /// face's front, if it does before `best`. `direction` is the unit
    /// sweep direction and `length` its travel; `radius` is the sphere's.
    /// `start` is the centre. The original finds the moment the centre is
    /// one radius in front of the plane, the closest point there, and the
    /// gap from that centre along the sweep; a sweep that only grazes the
    /// plane (a zero change) uses its end.
    #[allow(clippy::neg_cmp_op_on_partial_ord)] // a NaN takes the original's branch
    pub fn swept_sphere(
        &self,
        start: [f32; 3],
        direction: [f32; 3],
        length: f32,
        radius: f32,
        best: f32,
    ) -> Option<f32> {
        let start_distance = self.plane_distance(start);
        if start_distance < 0.0 {
            return None;
        }
        let delta = dot3(direction, self.normal) * length;
        if delta > 0.0 {
            return None;
        }
        if start_distance + delta > radius {
            return None;
        }
        let centre = if !(delta.abs() < FACE_EPSILON) {
            let travel = ((radius - start_distance) / delta) * length;
            [
                start[0] + direction[0] * travel,
                start[1] + direction[1] * travel,
                start[2] + direction[2] * travel,
            ]
        } else {
            [
                start[0] + direction[0] * length,
                start[1] + direction[1] * length,
                start[2] + direction[2] * length,
            ]
        };
        let distance = self.plane_distance(centre);
        let closest = self.closest_point(centre, distance);
        let v = sub(closest, centre);
        let along = dot3(v, direction);
        let proj = [
            v[0] - direction[0] * along,
            v[1] - direction[1] * along,
            v[2] - direction[2] * along,
        ];
        let perpendicular = dot3(proj, proj);
        if !(perpendicular < radius * radius) {
            return None;
        }
        let t = along - (radius * radius - perpendicular).sqrt();
        if 0.0 < t && t < best { Some(t) } else { None }
    }
}

impl Mesh {
    /// The face record of triangle `triangle`, if the mesh has it.
    pub fn face(&self, triangle: usize) -> Option<Face> {
        let t = self.indices.as_chunks::<3>().0.get(triangle)?;
        let point = |i: u32| self.positions.get(i as usize).copied();
        Some(Face::new(point(t[0])?, point(t[1])?, point(t[2])?))
    }

    /// Every triangle's face record, in index order; triangles naming a
    /// missing vertex are left out.
    pub fn faces(&self) -> Vec<Face> {
        self.indices
            .as_chunks::<3>()
            .0
            .iter()
            .filter_map(|t| {
                let point = |i: u32| self.positions.get(i as usize).copied();
                Some(Face::new(point(t[0])?, point(t[1])?, point(t[2])?))
            })
            .collect()
    }

    /// The nearest face a ray meets in front of it, with the distance and
    /// the triangle's index.
    pub fn ray_hit(&self, origin: [f32; 3], direction: [f32; 3], max: f32) -> Option<(u32, f32)> {
        let mut best = max;
        let mut found = None;
        for triangle in 0..self.indices.len() / 3 {
            if let Some(face) = self.face(triangle)
                && let Some(t) = face.ray(origin, direction, best)
            {
                best = t;
                found = Some((triangle as u32, t));
            }
        }
        found
    }

    /// The nearest face a ray meets, accepting a hit exactly at the
    /// previous best distance (the original's second ray walk).
    pub fn ray_hit_inclusive(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max: f32,
    ) -> Option<(u32, f32)> {
        let mut best = max;
        let mut found = None;
        for triangle in 0..self.indices.len() / 3 {
            if let Some(face) = self.face(triangle)
                && let Some(t) = face.ray_inclusive(origin, direction, best)
            {
                best = t;
                found = Some((triangle as u32, t));
            }
        }
        found
    }

    /// The first face a sphere reaches, with the push-out vector the
    /// original reports (see [`Face::sphere_push`]).
    pub fn sphere_hit(&self, centre: [f32; 3], radius: f32) -> Option<(u32, [f32; 3])> {
        (0..self.indices.len() / 3).find_map(|triangle| {
            let face = self.face(triangle)?;
            face.sphere_push(centre, radius)
                .map(|push| (triangle as u32, push))
        })
    }

    /// The first face within `reach` of `point`, with the distance to its
    /// closest point (see [`Face::within`]).
    pub fn within_hit(&self, point: [f32; 3], reach: f32) -> Option<(u32, f32)> {
        (0..self.indices.len() / 3).find_map(|triangle| {
            let face = self.face(triangle)?;
            face.within(point, reach)
                .map(|distance| (triangle as u32, distance))
        })
    }

    /// The nearest face a sphere's sweep meets, with the distance along the
    /// sweep (see [`Face::swept_sphere`]).
    pub fn swept_sphere_hit(
        &self,
        start: [f32; 3],
        direction: [f32; 3],
        length: f32,
        radius: f32,
    ) -> Option<(u32, f32)> {
        let mut best = length;
        let mut found = None;
        for triangle in 0..self.indices.len() / 3 {
            if let Some(face) = self.face(triangle)
                && let Some(t) = face.swept_sphere(start, direction, length, radius, best)
            {
                best = t;
                found = Some((triangle as u32, t));
            }
        }
        found
    }
}

/// The corner angle's weight in the original's normal averaging: the angle
/// θ between two edge vectors by the arc-cosine approximation
/// `√(1 − cos θ)·(1.571 − 0.1945·cos θ + 0.111·cos² θ − 0.111·cos³ θ)`,
/// and zero when the product of the squared lengths is not above
/// `0x33FFFFD7` or `cos θ` is not below one. The constants are the
/// original's f32 values.
pub fn corner_angle_weight(a: [f32; 3], b: [f32; 3]) -> f32 {
    const EPSILON: f32 = f32::from_bits(0x33FF_FFD7);
    const ONE: f32 = 1.570_999_979_972_839_4;
    const COS: f32 = 0.194_499_999_284_744_26;
    const CUBE: f32 = 0.111_000_001_430_511_47;
    let length_squared = dot3(b, b) * dot3(a, a);
    if length_squared > EPSILON {
        let cos = dot3(a, b) / length_squared.sqrt();
        if cos < 1.0 {
            return ((cos * cos * CUBE + (ONE - cos * COS)) - cos * cos * cos * CUBE)
                * (1.0 - cos).sqrt();
        }
    }
    0.0
}

/// Angle-weighted face normals accumulated over vertices sharing a smoothing
/// key, as the original's normal builder does: each face contributes its
/// unit normal (the reciprocal square root estimate's, which weights the
/// faces as in the original) times the corner angle at that vertex, and the
/// sum is normalised. With face flags, faces carrying
/// [`FACE_NORMAL_FALLBACK`] only count for a key no unflagged face
/// reaches. A key with no usable face keeps +Y where the original leaves a
/// zero normal (a labelled deviation: only degenerate faces use it).
fn smooth_normals(
    positions: &[[f32; 3]],
    indices: &[u32],
    keys: &[u32],
    elements: &[u32],
    face_flags: Option<&[u32]>,
) -> Vec<[f32; 3]> {
    let key_count = keys.iter().copied().max().map_or(0, |m| m as usize + 1);
    let (counts, ring, element_count) = element_rings(indices, elements, keys);
    let face_normals = face_normals(positions, indices);
    // Per key: the sum over unflagged faces, the sum over flagged faces,
    // and whether an unflagged face was seen.
    let mut acc = vec![[0.0f32; 3]; key_count];
    let mut fallback = vec![[0.0f32; 3]; key_count];
    let mut reached = vec![false; key_count];
    // The original builds one normal per position element, walking that
    // element's corners in ring order; an element whose vertices were split
    // by smoothing group is one element per group here, so the corners are
    // summed into their own keys.
    const NEXT: [usize; 3] = [2, 0, 1];
    const PREV: [usize; 3] = [1, 2, 0];
    for e in 0..element_count {
        for &(face, corner) in &ring[counts[e] as usize..counts[e + 1] as usize] {
            let face = face as usize;
            let corner = corner as usize;
            let Some(t) = indices.as_chunks::<3>().0.get(face) else {
                continue;
            };
            let corners = [t[0] as usize, t[1] as usize, t[2] as usize];
            if corners.iter().any(|&i| i >= positions.len()) {
                continue;
            }
            let normal = face_normals[face];
            let flagged = face_flags
                .and_then(|flags| flags.get(face))
                .is_some_and(|&flags| flags & FACE_NORMAL_FALLBACK != 0);
            let vertex = corners[corner];
            let key = keys.get(vertex).copied().unwrap_or(0) as usize;
            if key >= key_count || (flagged && reached[key]) {
                continue;
            }
            let (here, next, prev) = (
                positions[vertex],
                positions[corners[NEXT[corner]]],
                positions[corners[PREV[corner]]],
            );
            let weight = corner_angle_weight(sub(next, here), sub(prev, here));
            let target = if flagged {
                &mut fallback[key]
            } else {
                reached[key] = true;
                &mut acc[key]
            };
            for axis in 0..3 {
                target[axis] += normal[axis] * weight;
            }
        }
    }
    keys.iter()
        .map(|&k| {
            let k = k as usize;
            let sum = if reached.get(k).copied().unwrap_or(false) {
                acc[k]
            } else {
                fallback.get(k).copied().unwrap_or([0.0; 3])
            };
            normalised(sum).unwrap_or([0.0, 1.0, 0.0])
        })
        .collect()
}

/// The corners of every position element, in the original's incidence
/// order: faces are visited last first and each face's corners last first,
/// so an element's list starts with its last corner. Each element's list is
/// then split into smoothing-key runs the way the original does it in
/// place: the first remaining corner opens a run and every later corner
/// with the same key is swapped into the run's next slot. The swap is not
/// stable, so a second key's corners can come out reordered, which sets
/// the float order of the normal sums. Returns the prefix bounds, the
/// corner list and the element count.
fn element_rings(
    indices: &[u32],
    elements: &[u32],
    keys: &[u32],
) -> (Vec<u32>, Vec<(u32, u8)>, usize) {
    let element_count = elements
        .iter()
        .copied()
        .max()
        .map_or(0, |m| m as usize + 1)
        .max(1);
    let mut counts = vec![0u32; element_count + 1];
    for &v in indices {
        let Some(&e) = elements.get(v as usize) else {
            continue;
        };
        counts[e as usize + 1] += 1;
    }
    for e in 0..element_count {
        counts[e + 1] += counts[e];
    }
    let mut fill = counts.clone();
    let mut ring = vec![(0u32, 0u8); indices.len()];
    for (face, t) in indices.as_chunks::<3>().0.iter().enumerate().rev() {
        for corner in (0..3).rev() {
            let v = t[corner] as usize;
            let Some(&e) = elements.get(v) else {
                continue;
            };
            ring[fill[e as usize] as usize] = (face as u32, corner as u8);
            fill[e as usize] += 1;
        }
    }
    let key_of = |(face, corner): (u32, u8)| {
        indices
            .get(face as usize * 3 + corner as usize)
            .and_then(|&v| keys.get(v as usize))
            .copied()
    };
    for e in 0..element_count {
        let end = counts[e + 1] as usize;
        let mut start = counts[e] as usize;
        while start < end {
            let key = key_of(ring[start]);
            start += 1;
            // The scan starts after the run's opening corner and keeps
            // going to the element's end while the run's slot advances.
            let mut slot = start;
            for i in start..end {
                if key_of(ring[i]) == key {
                    ring.swap(i, slot);
                    slot += 1;
                }
            }
            start = slot;
        }
    }
    (counts, ring, element_count)
}

/// Every triangle's unit normal, once per face (a triangle naming a
/// missing vertex gets a zero normal and is skipped by its callers).
fn face_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| match t.map(|i| positions.get(i as usize).copied()) {
            [Some(a), Some(b), Some(c)] => face_normal(a, b, c),
            _ => [0.0; 3],
        })
        .collect()
}

/// The cloth pass's normal builder: each smoothing group's normal is the
/// unweighted sum of its faces' unit normals, walked in the incidence order
/// (the first face's normal sets it and the rest are added), normalised. A
/// group no face reaches keeps +Y (a labelled safety deviation).
fn flat_face_normals(
    positions: &[[f32; 3]],
    indices: &[u32],
    keys: &[u32],
    elements: &[u32],
) -> Vec<[f32; 3]> {
    let key_count = keys.iter().copied().max().map_or(0, |m| m as usize + 1);
    let (counts, ring, element_count) = element_rings(indices, elements, keys);
    let face_normals = face_normals(positions, indices);
    let mut acc = vec![[0.0f32; 3]; key_count];
    for e in 0..element_count {
        for &(face, corner) in &ring[counts[e] as usize..counts[e + 1] as usize] {
            let Some(t) = indices.as_chunks::<3>().0.get(face as usize) else {
                continue;
            };
            let corners = [t[0] as usize, t[1] as usize, t[2] as usize];
            if corners.iter().any(|&i| i >= positions.len()) {
                continue;
            }
            let normal = face_normals[face as usize];
            let key = keys.get(corners[corner as usize]).copied().unwrap_or(0) as usize;
            if key < key_count {
                for axis in 0..3 {
                    acc[key][axis] += normal[axis];
                }
            }
        }
    }
    keys.iter()
        .map(|&k| {
            normalised(acc.get(k as usize).copied().unwrap_or([0.0; 3])).unwrap_or([0.0, 1.0, 0.0])
        })
        .collect()
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"))
}

fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"))
}

fn vec3(b: &[u8], i: usize) -> [f32; 3] {
    let o = i * 12;
    [f32_at(b, o), f32_at(b, o + 4), f32_at(b, o + 8)]
}

/// Bounds-checked little-endian reader over one chunk's payload.
struct Cursor<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.b.len() - self.pos
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.b.get(self.pos..self.pos + n).ok_or(Error::Truncated {
            offset: self.pos,
            needed: n,
            len: self.b.len(),
        })?;
        self.pos += n;
        Ok(s)
    }

    fn skip(&mut self, n: usize) -> Result<()> {
        self.bytes(n).map(drop)
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.bytes(2)?.try_into().expect("2 bytes"),
        ))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.bytes(4)?.try_into().expect("4 bytes"),
        ))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn name(&mut self) -> Result<String> {
        let raw = self.bytes(16)?;
        let end = raw.iter().position(|&c| c == 0).unwrap_or(16);
        Ok(raw[..end].iter().map(|&c| c as char).collect())
    }

    /// A stored count, rejected when `count * min_item_size` cannot fit.
    fn count(&mut self, min_item_size: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        self.check_fits(n, min_item_size)
    }

    /// A stored "count minus one".
    fn count_minus_one(&mut self, min_item_size: usize) -> Result<usize> {
        let n = (self.u32()? as usize)
            .checked_add(1)
            .ok_or_else(|| Error::Malformed("count overflow".into()))?;
        self.check_fits(n, min_item_size)
    }

    fn check_fits(&self, n: usize, min_item_size: usize) -> Result<usize> {
        match n.checked_mul(min_item_size) {
            Some(bytes) if bytes <= self.remaining() => Ok(n),
            _ => Err(Error::Malformed(format!(
                "count {n} does not fit in {} remaining bytes",
                self.remaining()
            ))),
        }
    }

    fn indices(&mut self, n: usize, wide: bool) -> Result<Vec<u32>> {
        if wide {
            let raw = self.bytes(n * 4)?;
            Ok(raw
                .as_chunks::<4>()
                .0
                .iter()
                .map(|x| u32::from_le_bytes(*x))
                .collect())
        } else {
            let raw = self.bytes(n * 2)?;
            Ok(raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|x| u32::from(u16::from_le_bytes(*x)))
                .collect())
        }
    }

    /// The next chunk: its tag and a cursor over its payload.
    fn chunk(&mut self) -> Result<(u32, Cursor<'a>)> {
        let tag = self.u32()?;
        let size = self.u32()? as usize;
        let data = self.bytes(size)?;
        Ok((tag, Cursor::new(data)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(tag: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(payload);
        v
    }

    fn u32s(v: &[u32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn f32s(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn name(s: &str) -> Vec<u8> {
        let mut n = [0u8; 16];
        n[..s.len()].copy_from_slice(s.as_bytes());
        n.to_vec()
    }

    /// One mesh node: a unit quad in the XZ plane (centimetres) as two
    /// triangles, two materials, a UV stream.
    fn quad_file(tri_index_override: Option<u16>) -> Vec<u8> {
        file_of(&quad_body(tri_index_override))
    }

    fn file_of(body: &[u8]) -> Vec<u8> {
        let mut list = u32s(&[1]);
        list.extend(chunk(NODE_MESH, body));
        let mut file = vec![0xCF, 0xAF, 0x23, 0x3D];
        file.extend(chunk(NODE_LIST, &list));
        file
    }

    /// The quad's node chunk body; its box is 50 × 0 × 50 cm about
    /// (50, 0, −50).
    fn quad_body(tri_index_override: Option<u16>) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend(name("Quad"));
        body.extend(u32s(&[0x89, 0, 0, 0]));
        body.extend(f32s(&[
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 100.0, 0.0, 0.0,
        ]));
        body.extend(f32s(&[50.0, 0.0, 50.0, 50.0, 0.0, -50.0, 70.7]));
        body.extend(u32s(&[0]));
        body.extend(u32s(&[0])); // render flags
        body.extend(u32s(&[1])); // two streams
        body.extend(u32s(&[0x1, 0xF3_0001, 3]));
        body.extend(f32s(&[
            0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 100.0, 0.0, -100.0, 0.0, 0.0, -100.0,
        ]));
        body.extend(u32s(&[0x10, 0xF2_0010, 3]));
        body.extend(f32s(&[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]));

        let mut topo = u32s(&[0, 3, 1]); // skip, 4 vertices, 2 groups
        for (first, mat) in [(0u32, "mata"), (0, "matb")] {
            topo.extend(u32s(&[0, first, 3]));
            topo.extend(name(mat));
        }
        topo.extend(u32s(&[2])); // three maps
        for key in [0x0001_0001, 0x0001_0010, KEY_SMOOTHING] {
            topo.extend(u32s(&[key]));
            topo.extend([0u16, 1, 2, 3].iter().flat_map(|x| x.to_le_bytes()));
        }
        let mut tris = u32s(&[0, 1]); // skip, 2 triangles
        let last = tri_index_override.unwrap_or(3);
        tris.extend(
            [0u16, 1, 2, 0, 2, last]
                .iter()
                .flat_map(|x| x.to_le_bytes()),
        );
        tris.extend(u32s(&[0, 0, 1, 1]));
        tris.extend(chunk(0x3DC2, &[0, 0]));
        topo.extend(chunk(TRIANGLES, &tris));
        body.extend(chunk(TOPOLOGY, &topo));
        body
    }

    #[test]
    fn decodes_mesh_in_metres() {
        let scene = Scene::parse(&quad_file(None)).unwrap();
        assert_eq!(scene.version, 0x3D23);
        let (node, mesh) = scene.meshes().next().unwrap();
        assert_eq!(node.name, "Quad");
        assert_eq!(node.transform[3], [1.0, 0.0, 0.0]);
        assert_eq!(mesh.positions[2], [1.0, 0.0, -1.0]);
        assert_eq!(mesh.uvs.as_ref().unwrap()[1], [1.0, 0.0]);
        assert_eq!(mesh.uv_ids, [0, 1, 2, 3]);
        assert_eq!(mesh.uv_count, 4);
        assert_eq!(mesh.triangle_edges.as_deref(), Some(&[0u8, 0][..]));
        assert_eq!(mesh.indices, [0, 1, 2, 0, 2, 3]);
        assert_eq!(mesh.groups.len(), 2);
        assert_eq!(mesh.groups[1].material, "matb");
        assert_eq!(
            (mesh.groups[1].first_index, mesh.groups[1].index_count),
            (3, 3)
        );
        for n in &mesh.normals {
            assert!((n[1] - 1.0).abs() < 1e-6, "quad faces +Y: {n:?}");
        }
    }

    /// The word before the streams is the node's render flags; bits
    /// 16..18 are its shadow-bias level (`uw01_floorbrk*` ship 0x20000,
    /// level 2). Bit 19 is not part of the level.
    #[test]
    fn the_render_flags_carry_the_shadow_bias_level() {
        let mut file = quad_file(None);
        let streams: Vec<u8> = [0u32, 1, 0x1, 0xF3_0001]
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        let at = file
            .windows(streams.len())
            .position(|w| w == streams)
            .expect("the render flags before the streams");
        file[at..at + 4].copy_from_slice(&0x000A_0000u32.to_le_bytes());
        let scene = Scene::parse(&file).unwrap();
        let (_, mesh) = scene.meshes().next().unwrap();
        assert_eq!(mesh.render_flags, 0x000A_0000);
        assert_eq!(mesh.shadow_bias_level(), 2);
        assert!(mesh.shadow_bias_inflated());
        assert!(!mesh.shadow_cull_front());
        assert_eq!(Mesh::default().shadow_bias_level(), 0);
        // The shadow-pass cull-front bit is the render word's 0x800.
        let mut mesh = (*mesh).clone();
        mesh.render_flags = 0x0000_0800;
        assert!(mesh.shadow_cull_front());
        assert_eq!(mesh.shadow_bias_level(), 0);
        assert!(!mesh.shadow_bias_inflated());
    }

    /// Node flag `0x80000` marks level geometry (`arenao1`'s tavern table
    /// stores flags `0x8090089`; the level's other bodies `0x8010089`).
    #[test]
    fn reads_the_static_node_flag() {
        let mut node = Scene::parse(&quad_file(None)).unwrap().nodes.remove(0);
        assert!(!node.is_static());
        node.flags = 0x0809_0089;
        assert!(node.is_static());
        node.flags = 0x0801_0089;
        assert!(!node.is_static());
    }

    #[test]
    fn keeps_the_box_and_finds_a_collision_body() {
        let scene = Scene::parse(&quad_file(None)).unwrap();
        let node = &scene.nodes[0];
        assert_eq!(node.bounds_half_extents, [0.5, 0.0, 0.5]);
        assert_eq!(node.bounds_centre, [0.5, 0.0, -0.5]);
        assert!(!node.has_rigid_body() && !node.cloth_marker);
        // GI points, then a collision body and the cloth marker, after the
        // mesh body.
        let mut body = quad_body(None);
        let mut gi = u32s(&[40, 1]);
        gi.extend(f32s(&[100.0, 200.0, 300.0]));
        body.extend(chunk(GI_POINTS, &gi));
        let mut rigid = u32s(&[0x1000_0000]);
        rigid.extend(f32s(&[2.5]));
        rigid.extend(vec![0; 84]);
        body.extend(chunk(RIGID_BODY, &rigid));
        body.extend(chunk(CLOTH, &[0; 4]));
        let scene = Scene::parse(&file_of(&body)).unwrap();
        let node = &scene.nodes[0];
        assert!(node.has_rigid_body() && node.cloth_marker);
        let rigid = node.rigid().unwrap();
        assert_eq!((rigid.class, rigid.mobility), (0x1000_0000, 2.5));
        let NodeBody::Mesh(mesh) = &node.body else {
            panic!("not a mesh");
        };
        assert_eq!(mesh.gi_points, [[1.0, 2.0, 3.0]]);
        // A body too short to decode keeps its error; the node still has
        // one.
        let mut body = quad_body(None);
        body.extend(chunk(RIGID_BODY, &[0; 8]));
        let scene = Scene::parse(&file_of(&body)).unwrap();
        let node = &scene.nodes[0];
        assert!(node.has_rigid_body() && node.rigid().is_none());
        assert!(matches!(&node.rigid_body, Some(Err(e)) if e.contains("truncated")));
    }

    #[test]
    fn out_of_range_index_marks_mesh_invalid() {
        let scene = Scene::parse(&quad_file(Some(9))).unwrap();
        assert!(matches!(&scene.nodes[0].body, NodeBody::Invalid(e) if e.contains("out of 4")));
    }

    /// A file holding one voxel-light node with `body` after the header.
    fn voxel_light_file(body: &[u8]) -> Vec<u8> {
        let mut node = name("@Omni001");
        node.extend(u32s(&[0x41, 0x10, 0, 0]));
        node.extend(f32s(&[
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 3.0, 0.0,
        ]));
        node.extend(f32s(&[0.0; 7]));
        node.extend(u32s(&[0]));
        node.extend_from_slice(body);
        let mut list = u32s(&[1]);
        list.extend(chunk(NODE_VOXEL_LIGHT, &node));
        let mut file = vec![0xCF, 0xAF, 0x23, 0x3D];
        file.extend(chunk(NODE_LIST, &list));
        file
    }

    #[test]
    fn decodes_voxel_light() {
        let mut body = u32s(&[0]);
        body.extend(f32s(&[0.2157, 0.6784, 1.0, 1.0, 16.0, 300.0]));
        body.extend(u32s(&[0]));
        let scene = Scene::parse(&voxel_light_file(&body)).unwrap();
        let node = &scene.nodes[0];
        assert_eq!(
            (node.kind, node.name.as_str()),
            (NODE_VOXEL_LIGHT, "@Omni001")
        );
        assert_eq!(node.flags & 0xFF, 0x41);
        assert_eq!(node.transform[3], [0.0, 0.03, 0.0]);
        let NodeBody::VoxelLight(light) = node.body else {
            panic!("not a voxel light: {:?}", node.body);
        };
        assert_eq!(light.color, [0.2157, 0.6784, 1.0]);
        assert_eq!(light.intensity, 16.0);
        assert!((light.radius - 3.0).abs() < 1e-6);
    }

    #[test]
    fn short_voxel_light_is_invalid() {
        let mut body = u32s(&[0]);
        body.extend(f32s(&[0.2157, 0.6784, 1.0, 1.0, 16.0]));
        let scene = Scene::parse(&voxel_light_file(&body)).unwrap();
        assert!(matches!(&scene.nodes[0].body, NodeBody::Invalid(_)));
    }

    #[test]
    fn truncated_file_is_an_error() {
        let mut f = quad_file(None);
        f.truncate(f.len() - 10);
        assert!(Scene::parse(&f).is_err());
    }

    /// A file of `nodes` (kind, chunk body).
    fn file_of_nodes(nodes: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut list = u32s(&[nodes.len() as u32]);
        for (kind, body) in nodes {
            list.extend(chunk(*kind, body));
        }
        let mut file = vec![0xCF, 0xAF, 0x23, 0x3D];
        file.extend(chunk(NODE_LIST, &list));
        file
    }

    /// A stored node's chunk body: the header (identity transform, empty
    /// bounds) with node flags 2 `flags2`, then `body`.
    fn node_with(node: &str, flags2: u32, body: &[u8]) -> Vec<u8> {
        let mut b = name(node);
        b.extend(u32s(&[0, flags2, 0, 0]));
        b.extend(f32s(&[
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
        ]));
        b.extend(f32s(&[0.0; 7]));
        b.extend(u32s(&[0]));
        b.extend_from_slice(body);
        b
    }

    /// Keyframe tracks follow the body of any stored node: a light's 40
    /// bytes, 8 more for a spot (node flags 2 `0x200`) and 16 more under
    /// its body flags `0x30000`; kind `0x3D08`'s 8; nothing for a plain
    /// node; a mesh's body. A truncated track keeps its error and the rest
    /// of the file decodes.
    #[test]
    fn reads_key_tracks_after_any_node_body() {
        let mut payload = u32s(&[0x3AE2, 1]);
        payload.extend(f32s(&[3.0]));
        payload.extend(u32s(&[0]));
        payload.extend(f32s(&[0.0, 100.0, 200.0, 300.0]));
        let track = chunk(KEY_TRACK, &payload);
        let light = |flags: u32, spot: bool| {
            let mut b = u32s(&[flags]);
            b.extend(f32s(&[1.0, 0.5, 0.25]));
            b.extend(u32s(&[0]));
            b.extend(f32s(&[1.0, 2.0, 500.0, 0.0, 0.5]));
            if spot {
                b.extend(f32s(&[30.0, 45.0]));
            }
            if flags & LIGHT_EXTRA_FLAGS != 0 {
                b.extend(u32s(&[0]));
                b.extend(f32s(&[0.25, 0.5, 0.75]));
            }
            b.extend(&track);
            b
        };
        let mut odd = u32s(&[7, 9]);
        odd.extend(&track);
        let mut plain = track.clone();
        plain.extend(u32s(&[FIRE, 4]));
        plain.extend(f32s(&[20.0]));
        let mut mesh = quad_body(None);
        mesh.extend(chunk(KEY_TRACK, &payload[..payload.len() - 1]));
        mesh.extend(chunk(CLOTH, &[0; 4]));
        let file = file_of_nodes(&[
            (NODE_LIGHT, node_with("Omni", 0, &light(0, false))),
            (NODE_LIGHT, node_with("Spot", SPOT, &light(0, true))),
            (NODE_LIGHT, node_with("Fade", 0, &light(0x1_0000, false))),
            (NODE_LIGHT, node_with("Both", SPOT, &light(0x2_0000, true))),
            (NODE_3D08, node_with("Odd", 0, &odd)),
            (0x3D01, node_with("Fire", 0, &plain)),
            (NODE_MESH, quad_body(None)),
            (NODE_MESH, mesh),
        ]);
        let scene = Scene::parse(&file).unwrap();
        assert_eq!(scene.nodes.len(), 8);
        for node in &scene.nodes[..6] {
            let tracks: Vec<_> = node.tracks().collect();
            assert_eq!(tracks.len(), 1, "{}", node.name);
            assert_eq!(tracks[0].keys[0].position, [1.0, 2.0, 3.0]);
        }
        assert!(matches!(
            scene.nodes[1].body,
            NodeBody::Light(Light { spot: Some(_), .. })
        ));
        // The flicker and fade words: amplitude, speed, emissive share.
        let animation = |n: usize| match &scene.nodes[n].body {
            NodeBody::Light(l) => l.animation,
            _ => panic!("light"),
        };
        assert_eq!(animation(0), None);
        let flicker = animation(2).unwrap();
        assert_eq!(
            (flicker.amplitude, flicker.speed, flicker.emissive),
            (0.25, 0.5, 0.75)
        );
        assert_eq!(animation(3), Some(flicker));
        assert!(matches!(
            scene.nodes[5].body,
            NodeBody::Emitter {
                kind: FIRE,
                size: 20.0
            }
        ));
        assert!(scene.nodes[6].key_tracks.is_empty());
        let truncated = &scene.nodes[7];
        assert!(matches!(&truncated.key_tracks[..], [Err(e)] if e.contains("truncated")));
        assert!(truncated.cloth_marker && matches!(truncated.body, NodeBody::Mesh(_)));
    }

    /// The importer's other node kinds and level sections: an instanced
    /// node names its 1-based owner, a camera node its target, a sector
    /// node stores no body, an unknown root chunk is reported, and the
    /// level's string table decodes on its own.
    #[test]
    fn instances_cameras_sectors_strings_and_unknown_chunks() {
        let first = node_with("Owner", 0, &[]);
        // node_with writes the header with instance 0; the second node's
        // instance word (the fourth word of its header) names node 1.
        let mut second = node_with("Child", 0, &[]);
        let at = name("Child").len() + 8;
        second[at..at + 4].copy_from_slice(&1u32.to_le_bytes());
        let mut camera = u32s(&[2, 77]);
        camera.extend(chunk(KEY_TRACK, &u32s(&[0x3AE2, 0])));
        let mut sectors = node_with("Sector", 0, &[]);
        sectors.extend(chunk(0x1234_5678, &[9; 4]));
        let file = {
            let mut list = u32s(&[4]);
            list.extend(chunk(0x3D01, &first));
            list.extend(chunk(0x3D01, &second));
            list.extend(chunk(NODE_3D08, &node_with("Cam", 0, &camera)));
            list.extend(chunk(NODE_SECTOR, &sectors));
            let mut file = vec![0xCF, 0xAF, 0x23, 0x3D];
            file.extend(chunk(NODE_LIST, &list));
            file.extend(chunk(0xDEAD_BEEF, &[1, 2, 3]));
            file
        };
        let scene = Scene::parse(&file).unwrap();
        assert_eq!(scene.nodes.len(), 4);
        assert_eq!(scene.nodes[0].instance, None);
        assert_eq!(scene.nodes[1].instance, Some(1));
        assert!(matches!(scene.nodes[1].body, NodeBody::Instance(1)));
        assert!(matches!(
            scene.nodes[2].body,
            NodeBody::Camera {
                target: Some(2),
                value: 77
            }
        ));
        assert!(matches!(scene.nodes[3].body, NodeBody::Sector));
        assert_eq!(scene.nodes[3].unknown_sub_chunks, [0x1234_5678]);
        assert_eq!(scene.unknown_chunks, [0xDEAD_BEEF]);

        // The level's own string table, alone and inside a scene.
        let mut library = Library::default();
        let id = library.add(b"Brass key");
        let table = chunk(LOCAL_STRINGS, &library.write());
        let mut file = vec![0xCF, 0xAF, 0x23, 0x3D];
        file.extend(chunk(LOCAL_DATABASE, &table));
        let scene = Scene::parse(&file).unwrap();
        let strings = scene.local_strings.as_ref().unwrap();
        assert_eq!(strings.get(id).as_deref(), Some("Brass key"));
        let db = Scene::parse_database(&file).unwrap();
        assert_eq!(db.strings.unwrap().get(id).as_deref(), Some("Brass key"));
        assert!(db.items.is_empty() && db.placed_objects.is_empty());
        assert!(Scene::parse_database(b"not an rfc").is_err());
    }

    #[test]
    fn the_deprecated_fading_flag_is_replaced() {
        // The second header word (flags2) is at byte 20 of the node.
        let mut mesh = quad_body(None);
        mesh[20..24].copy_from_slice(&0x8000_0001u32.to_le_bytes());
        let scene = Scene::parse(&file_of_nodes(&[(NODE_MESH, mesh)])).unwrap();
        assert_eq!(scene.nodes[0].flags2, 0x0100_0001);
        let mut body = node_with("New", 0x0000_0001, &[]);
        let file = file_of_nodes(&[(NODE_MESH, std::mem::take(&mut body))]);
        assert_eq!(Scene::parse(&file).unwrap().nodes[0].flags2, 1);
        // Only the mesh body reader replaces it: a plain node keeps it.
        let plain = node_with("Old", 0x8000_0001, &[]);
        let scene = Scene::parse(&file_of_nodes(&[(0x3D01, plain)])).unwrap();
        assert_eq!(scene.nodes[0].flags2, 0x8000_0001);
    }

    /// An instance node draws its owner's mesh, as the original shares the
    /// owner's object with it.
    #[test]
    fn instance_nodes_share_their_owners_mesh() {
        let mut child = node_with("Copy", 0, &[]);
        let at = name("Copy").len() + 8;
        child[at..at + 4].copy_from_slice(&1u32.to_le_bytes());
        let file = file_of_nodes(&[(NODE_MESH, quad_body(None)), (NODE_MESH, child.clone())]);
        let scene = Scene::parse(&file).unwrap();
        assert_eq!(scene.meshes().count(), 2);
        assert_eq!(
            scene.mesh_of(1).unwrap().positions,
            scene.mesh_of(0).unwrap().positions
        );
        assert!(matches!(scene.nodes[1].body, NodeBody::Instance(1)));
        // A chain and a self-reference resolve to the first stored body.
        let mut later = node_with("Later", 0, &[]);
        later[at..at + 4].copy_from_slice(&2u32.to_le_bytes());
        let file = file_of_nodes(&[
            (NODE_MESH, quad_body(None)),
            (NODE_MESH, child.clone()),
            (NODE_MESH, later),
        ]);
        let scene = Scene::parse(&file).unwrap();
        assert!(scene.mesh_of(2).is_some());
        let mut own = node_with("Self", 0, &[]);
        own[at..at + 4].copy_from_slice(&1u32.to_le_bytes());
        let scene = Scene::parse(&file_of_nodes(&[(NODE_MESH, own)])).unwrap();
        assert!(scene.mesh_of(0).is_none());
    }

    /// A level's placed objects carry the same node overrides as props.
    #[test]
    fn placed_objects_keep_their_attachment() {
        let mut body = u32s(&[0x8000_0001, 0, 0x8_0000]);
        body.extend(f32s(&[
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 10.0, 20.0, 30.0,
        ]));
        body.extend(u32s(&[1, 0]));
        let mut attachment = name("DoorRing");
        attachment.extend(u32s(&[3]));
        attachment.extend(f32s(&[
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 100.0, 0.0, 0.0,
        ]));
        body.extend(chunk(0x3DE0_ECAC, &attachment));
        let mut objects = u32s(&[1]);
        objects.extend(chunk(0xDB01, &body));
        let mut file = vec![0xCF, 0xAF, 0x23, 0x3D];
        file.extend(chunk(LOCAL_DATABASE, &chunk(PLACED_OBJECTS, &objects)));
        let scene = Scene::parse(&file).unwrap();
        assert!(scene.level_errors.is_empty(), "{:?}", scene.level_errors);
        let o = &scene.placed_objects[0];
        // The third word is the object's own flags: bit 19 makes it level
        // geometry, as the placement's bit 4 does.
        assert_eq!(o.object_flags, 0x8_0000);
        assert_eq!((o.slot, o.second_word, o.word_3d22), (1, 0, None));
        assert!(o.is_static());
        for (got, want) in o.transform[3].iter().zip([0.1, 0.2, 0.3]) {
            assert!((got - want).abs() < 1e-6, "{got} != {want}");
        }
        let crate::level::PropAttachment::Transform {
            name,
            word,
            transform,
            ..
        } = &o.attachments[0]
        else {
            panic!("not a transform");
        };
        assert_eq!((name.as_str(), *word), ("DoorRing", 3));
        assert_eq!(transform[3], [1.0, 0.0, 0.0]);
    }

    /// The installed levels' props carry node overrides (skipped without the
    /// game): every level has transform records (the arena's dummy bases),
    /// and exanima01 replaces a banner mesh's 228 positions.
    #[test]
    fn shipped_props_keep_their_placement_overrides() {
        use exa_rpk::PackKind;
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let pack = game.pack(PackKind::Resource).unwrap();
        let exanima01 = Scene::parse(pack.get("exanima01.rfc").unwrap()).unwrap();
        let mut transforms = 0;
        let mut banner = None;
        for prop in &exanima01.props {
            for attachment in &prop.attachments {
                match attachment {
                    crate::level::PropAttachment::Transform { .. } => transforms += 1,
                    crate::level::PropAttachment::Tree { .. } => {}
                    crate::level::PropAttachment::MeshPositions {
                        name, positions, ..
                    } => {
                        if name.starts_with("UW01_Bann") {
                            banner = Some(positions.len());
                        }
                    }
                }
            }
        }
        assert!(transforms > 0, "exanima01's metal rings");
        assert_eq!(banner, Some(228));
        let arena = Scene::parse(pack.get("arenahub.rfc").unwrap()).unwrap();
        assert!(
            arena.props.iter().any(|p| !p.attachments.is_empty()),
            "the dummy bases"
        );

        // Every record of exanima01 against its object: the node is the
        // root's (or the enclosing record's node's) first direct child of
        // that exact name, and a position list matches the mesh's
        // position-stream count.
        let objlib = game.pack(PackKind::Objlib).unwrap();
        #[derive(Default, Debug)]
        struct Count {
            transforms: usize,
            nested: usize,
            unresolved: usize,
            positions: usize,
            matching: usize,
        }
        fn walk(
            scene: &Scene,
            at: usize,
            list: &[crate::level::PropAttachment],
            nested: bool,
            n: &mut Count,
        ) {
            use crate::level::PropAttachment as A;
            for a in list {
                let target = if a.name().is_empty() {
                    Some(at)
                } else {
                    scene
                        .nodes
                        .iter()
                        .position(|c| c.parent == Some(at + 1) && c.name == a.name())
                };
                match a {
                    A::Transform { .. } => {
                        n.transforms += 1;
                        n.nested += usize::from(nested);
                    }
                    A::MeshPositions { .. } => n.positions += 1,
                    A::Tree { .. } => {}
                }
                let Some(t) = target else {
                    n.unresolved += 1;
                    continue;
                };
                match a {
                    A::Transform { children, .. } => walk(scene, t, children, true, n),
                    A::MeshPositions { positions, .. } => {
                        if let NodeBody::Mesh(m) = &scene.nodes[t].body
                            && m.position_count == positions.len()
                        {
                            n.matching += 1;
                        }
                    }
                    A::Tree { entries } => walk(scene, t, entries, nested, n),
                }
            }
        }
        let mut n = Count::default();
        for prop in &exanima01.props {
            if prop.attachments.is_empty() {
                continue;
            }
            let object = Scene::parse(objlib.get(&prop.object).unwrap()).unwrap();
            let root = object
                .nodes
                .iter()
                .position(|n| n.parent.is_none())
                .unwrap();
            walk(&object, root, &prop.attachments, false, &mut n);
        }
        eprintln!("exanima01 placement records: {n:?}");
        assert!(n.nested > 0, "transform records nest: {n:?}");
        assert_eq!(n.unresolved, 0, "{n:?}");
        assert_eq!((n.positions, n.matching), (14, 14), "{n:?}");
    }

    #[test]
    fn the_cloth_gate_needs_the_marker_and_the_scene_flag() {
        assert!(cloth_import(true, CLOTH_SCENE));
        assert!(!cloth_import(false, CLOTH_SCENE));
        assert!(!cloth_import(true, 0));
        assert!(!cloth_import(true, CLOTH_SCENE - 1));
    }

    /// The cloth markers of the installed packs (skipped without the
    /// game): the long hair, four banners and the wraith's robe.
    #[test]
    fn shipped_cloth_markers() {
        use exa_rpk::PackKind;
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let marked = |kind: PackKind| -> Vec<(String, usize)> {
            let pack = game.pack(kind).unwrap();
            pack.entries()
                .iter()
                .filter(|e| crate::FileKind::detect(pack.bytes(e)) == crate::FileKind::ChunkTree)
                .filter_map(|e| {
                    let scene = Scene::parse(pack.bytes(e)).ok()?;
                    let n = scene.nodes.iter().filter(|n| n.cloth_marker).count();
                    (n > 0).then(|| (e.name.clone(), n))
                })
                .collect()
        };
        let hair = marked(PackKind::CharExt);
        assert_eq!(hair.len(), 6);
        assert!(
            hair.iter()
                .all(|(name, n)| name.starts_with("hair hu") && *n == 2)
        );
        let banners: Vec<_> = marked(PackKind::Objlib)
            .into_iter()
            .map(|(m, _)| m)
            .collect();
        assert_eq!(
            banners,
            [
                "bannerubma01",
                "mawbanner01",
                "uw01_banner01",
                "uw01_banner02"
            ]
        );
        assert_eq!(
            marked(PackKind::Resource),
            [("wraithhabase.rfc".to_string(), 1)]
        );
        assert!(marked(PackKind::Objects).is_empty());
    }

    #[test]
    fn refitted_bounds_use_the_box_centre_and_farthest_vertex() {
        let mesh = Mesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 2.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            ..Default::default()
        };
        let bounds = mesh.bounds();
        assert_eq!(bounds.centre, [1.0, 1.0, 0.5]);
        assert_eq!(bounds.half_extents, [1.0, 1.0, 0.5]);
        assert!((bounds.radius - 1.5).abs() < 1e-6);
        let empty = Mesh::default().bounds();
        assert_eq!(empty.half_extents, [BOUNDS_HALF_FLOOR; 3]);
        assert_eq!(empty.radius, BOUNDS_HALF_FLOOR);
        // A single point: extents floored at 0.001 cm, and a zero squared
        // radius kept unrooted.
        let point = Mesh {
            positions: vec![[0.5, 0.25, -1.0]],
            ..Default::default()
        }
        .bounds();
        assert_eq!(point.centre, [0.5, 0.25, -1.0]);
        assert_eq!(point.half_extents, [BOUNDS_HALF_FLOOR; 3]);
        assert_eq!(point.radius, 0.0);
    }

    /// Every mesh of the installed object and character packs gets finite
    /// normals (skipped without the game), and how many faces carry the
    /// normal fallback flag.
    #[test]
    fn shipped_meshes_get_finite_normals() {
        use exa_rpk::PackKind;
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let (mut meshes, mut faces, mut flagged) = (0usize, 0usize, 0usize);
        for kind in [PackKind::Objlib, PackKind::CharExt, PackKind::Resource] {
            let pack = game.pack(kind).unwrap();
            for entry in pack.entries() {
                let bytes = pack.bytes(entry);
                if crate::FileKind::detect(bytes) != crate::FileKind::ChunkTree {
                    continue;
                }
                let Ok(scene) = Scene::parse(bytes) else {
                    continue;
                };
                for node in &scene.nodes {
                    let NodeBody::Mesh(mesh) = &node.body else {
                        continue;
                    };
                    meshes += 1;
                    faces += mesh.indices.len() / 3;
                    if let Some(flags) = &mesh.triangle_flags {
                        flagged += flags
                            .iter()
                            .filter(|&&f| f & FACE_NORMAL_FALLBACK != 0)
                            .count();
                    }
                    for normal in &mesh.normals {
                        assert!(normal.iter().all(|x| x.is_finite()), "{}", entry.name);
                        let length = dot3(*normal, *normal).sqrt();
                        assert!((length - 1.0).abs() < 1e-5, "{}: {normal:?}", entry.name);
                    }
                }
            }
        }
        eprintln!("{meshes} meshes, {faces} faces, {flagged} with the normal fallback flag");
        assert!(meshes > 100);
    }

    #[test]
    fn regrouping_orders_triangles_by_material() {
        let mut mesh = Mesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            indices: vec![0, 1, 2, 1, 3, 2],
            groups: vec![
                MeshGroup {
                    material: "a".into(),
                    first_index: 0,
                    index_count: 3,
                },
                MeshGroup {
                    material: "b".into(),
                    first_index: 3,
                    index_count: 3,
                },
            ],
            triangle_flags: Some(vec![1, 2]),
            ..Default::default()
        };
        mesh.regroup_by_material(&[1, 0]).unwrap();
        assert_eq!(mesh.indices, [1, 3, 2, 0, 1, 2]);
        assert_eq!(mesh.triangle_flags, Some(vec![2, 1]));
        assert_eq!(mesh.groups.len(), 2);
        assert_eq!(mesh.groups[0].first_index, 0);
        assert_eq!(mesh.groups[0].index_count, 3);
        assert_eq!(mesh.groups[1].first_index, 3);
        // A group with no triangles disappears.
        mesh.regroup_by_material(&[0, 0]).unwrap();
        assert_eq!(mesh.groups.len(), 1);
        assert_eq!(mesh.groups[0].index_count, 6);
        assert!(mesh.regroup_by_material(&[0]).is_err());
    }

    #[test]
    fn barycentric_coordinates_split_the_triangle() {
        let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3);
        let triangle = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        // The stored inverse is the reciprocal estimate, good to about
        // 2^-12, so the weights are close to the exact ones, not equal.
        let weights = barycentric(triangle, [0.25, 0.25, 0.0]);
        assert!(near(weights, [0.5, 0.25, 0.25]), "{weights:?}");
        // A point off the plane is dropped onto it first.
        let lifted = barycentric(triangle, [0.25, 0.25, 3.0]);
        assert_eq!(lifted, weights);
        let corner = barycentric(triangle, [1.0, 0.0, 0.0]);
        assert!(near(corner, [0.0, 1.0, 0.0]), "{corner:?}");
        // The three always sum to one by construction: first = 1 − b − c.
        assert_eq!(corner[0], (1.0 - corner[1]) - corner[2]);
    }

    /// The face record holds the unit normal, the plane offset and the
    /// edges' Gram terms; the closest point walks the interior and the
    /// three edge regions.
    #[test]
    fn the_face_record_and_its_closest_point() {
        let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3);
        // A right triangle in the XZ plane, normal +Y.
        let face = Face::new([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 0.0, -2.0]);
        assert!(near(face.normal, [0.0, 1.0, 0.0]));
        assert_eq!(face.plane, 0.0);
        assert_eq!(face.edge1_sq, 4.0);
        assert!(
            (face.inv_area_sq - 0.0625).abs() < 1e-4,
            "{}",
            face.inv_area_sq
        );
        // Above the interior: the projection.
        let p = [0.5, 3.0, -0.5];
        let distance = face.plane_distance(p);
        assert!((distance - 3.0).abs() < 1e-3, "{distance}");
        assert!(near(face.closest_point(p, distance), [0.5, 0.0, -0.5]));
        // Above the first corner: the corner.
        let p = [-1.0, 2.0, 1.0];
        assert!(near(
            face.closest_point(p, face.plane_distance(p)),
            [0.0, 0.0, 0.0]
        ));
        // Above the middle of the first edge: that edge's point.
        let p = [1.0, 2.0, 1.0];
        assert!(near(
            face.closest_point(p, face.plane_distance(p)),
            [1.0, 0.0, 0.0]
        ));
        // Above the hypotenuse: its nearest point, (1, 0, −1).
        let p = [2.0, 1.0, -2.0];
        assert!(near(
            face.closest_point(p, face.plane_distance(p)),
            [1.0, 0.0, -1.0]
        ));
    }

    /// The original's range rejections are "below"/"above" compares a NaN
    /// passes; only the final distance checks reject it.
    #[test]
    fn a_nan_passes_the_range_rejections_but_not_the_distance_checks() {
        let face = Face::new([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 0.0, -2.0]);
        // A NaN direction reaches the distance check and fails it there.
        assert_eq!(
            face.ray([0.5, 5.0, -0.5], [f32::NAN, -1.0, 0.0], 10.0),
            None
        );
        // A NaN centre passes every sphere rejection and is pushed by NaN.
        let push = face.sphere_push([f32::NAN, 0.5, -0.5], 1.0);
        assert!(
            push.is_some_and(|p| p.iter().any(|x| x.is_nan())),
            "{push:?}"
        );
        // The reach test's last compare is a strict "under", so NaN misses.
        assert_eq!(face.within([f32::NAN, 0.5, -0.5], 1.0), None);
    }

    /// The ray test is one-sided and front-only: a ray running against the
    /// normal hits where it crosses, one running with it misses, and the
    /// nearest of two faces wins.
    #[test]
    fn a_ray_meets_the_front_of_a_face() {
        let face = Face::new([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 0.0, -2.0]);
        let t = face.ray([0.5, 5.0, -0.5], [0.0, -1.0, 0.0], 10.0);
        assert!(t.is_some_and(|t| (t - 5.0).abs() < 1e-4), "{t:?}");
        // From below the ray runs with the normal: no front hit.
        assert_eq!(face.ray([0.5, -5.0, -0.5], [0.0, 1.0, 0.0], 10.0), None);
        // Past the first corner: the barycentric test rejects it.
        assert_eq!(face.ray([5.0, 5.0, -5.0], [0.0, -1.0, 0.0], 10.0), None);
        // A parallel ray never crosses the plane.
        assert_eq!(face.ray([0.5, 5.0, -0.5], [1.0, 0.0, 0.0], 10.0), None);
        // Not nearer than the limit already found.
        assert_eq!(face.ray([0.5, 5.0, -0.5], [0.0, -1.0, 0.0], 5.0), None);
        // The inclusive variant accepts a hit exactly at the limit.
        let exact = face.ray_inclusive([0.5, 5.0, -0.5], [0.0, -1.0, 0.0], 5.0);
        assert!(exact.is_some_and(|t| (t - 5.0).abs() < 1e-4), "{exact:?}");
    }

    /// The mesh walks its triangles and reports the nearest face, and a
    /// sphere reports the push-out from the face it reaches.
    #[test]
    fn mesh_queries_walk_the_triangles() {
        // Two triangles side by side in the XZ plane.
        let mesh = Mesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
            ],
            normals: vec![[0.0, 1.0, 0.0]; 4],
            position_ids: vec![0, 1, 2, 3],
            indices: vec![0, 1, 2, 0, 2, 3],
            groups: vec![MeshGroup {
                material: "m".into(),
                first_index: 0,
                index_count: 6,
            }],
            smoothing: vec![0; 4],
            position_count: 4,
            ..Mesh::default()
        };
        assert_eq!(mesh.faces().len(), 2);
        let hit = mesh.ray_hit([0.75, 4.0, -0.25], [0.0, -1.0, 0.0], 10.0);
        let (triangle, t) = hit.expect("a hit");
        assert!(t > 3.9 && t < 4.1, "{t}");
        assert!(triangle < 2, "{triangle}");
        // A sphere one metre wide sitting half a metre over the centre.
        let (_, push) = mesh.sphere_hit([0.5, 0.5, -0.5], 1.0).expect("a sphere");
        assert!((push[1] - 0.5).abs() < 1e-4, "{push:?}");
        // The closest point's reciprocal estimate leaves a sub-millimetre
        // sideways term.
        assert!(push[0].abs() < 1e-3 && push[2].abs() < 1e-3, "{push:?}");
        // Well above the surface it reaches nothing.
        assert!(mesh.sphere_hit([0.5, 3.0, -0.5], 1.0).is_none());
        // The same square is within a metre of a point half a metre up, and
        // not of one three metres away.
        let near = mesh.within_hit([0.5, 0.5, -0.5], 1.0).expect("a face");
        assert!((near.1 - 0.5).abs() < 1e-3, "{near:?}");
        assert!(mesh.within_hit([0.5, 3.0, -0.5], 1.0).is_none());
        // A point beside the square reaches its nearest corner.
        let beside = mesh.within_hit([1.5, 0.0, -0.5], 1.0).expect("a face");
        assert!((beside.1 - 0.5).abs() < 1e-3, "{beside:?}");
    }

    /// A sphere swept straight down meets a face where its centre is one
    /// radius over the plane, the closest point there sets the gap, and a
    /// sweep that stops short or moves away misses.
    #[test]
    fn a_swept_sphere_meets_the_face_it_reaches() {
        let face = Face::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]);
        // Offset past the hypotenuse, so the closest point at the contact
        // moment is the first corner and the gap is positive:
        // (1, 0, 0) − (1.3, 0.5, −0.3) = (−0.3, −0.5, 0.3), whose
        // perpendicular part is (−0.3, 0, 0.3) (0.18) and along part 0.5.
        let t = face.swept_sphere([1.3, 1.5, -0.3], [0.0, -1.0, 0.0], 5.0, 0.5, 5.0);
        assert!(t.is_some_and(|t| (t - 0.235_425).abs() < 1e-3), "{t:?}");
        // The same sweep only 0.2 long stops well above the face.
        assert_eq!(
            face.swept_sphere([1.3, 1.5, -0.3], [0.0, -1.0, 0.0], 0.2, 0.5, 0.2),
            None
        );
        // Upward it runs away from the front.
        assert_eq!(
            face.swept_sphere([1.3, 1.5, -0.3], [0.0, 1.0, 0.0], 5.0, 0.5, 5.0),
            None
        );
        // A perpendicular sweep touches at the contact moment: no gap.
        assert_eq!(
            face.swept_sphere([0.3, 1.5, -0.3], [0.0, -1.0, 0.0], 5.0, 0.5, 5.0),
            None
        );
        let mesh = Mesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]],
            normals: vec![[0.0, 1.0, 0.0]; 3],
            position_ids: vec![0, 1, 2],
            indices: vec![0, 1, 2],
            groups: vec![MeshGroup {
                material: "m".into(),
                first_index: 0,
                index_count: 3,
            }],
            smoothing: vec![0; 3],
            position_count: 3,
            ..Mesh::default()
        };
        let hit = mesh.swept_sphere_hit([1.3, 1.5, -0.3], [0.0, -1.0, 0.0], 5.0, 0.5);
        assert!(
            hit.is_some_and(|(_, t)| (t - 0.235_425).abs() < 1e-3),
            "{hit:?}"
        );
    }

    /// A face with the fallback flag shapes a smoothing group only when no
    /// face without it reaches the group.
    #[test]
    fn flagged_faces_only_shape_a_normal_alone() {
        // Two faces share vertex 0 through key 0: an XY face (normal +Z)
        // and a flagged XZ face (normal +Y).
        let positions = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let indices = [0, 1, 2, 0, 3, 1];
        let keys = [0, 1, 2, 3];
        let flags = [0, FACE_NORMAL_FALLBACK];
        let elements = [0, 1, 2, 3];
        let normals = smooth_normals(&positions, &indices, &keys, &elements, Some(&flags));
        assert!((normals[0][2] - 1.0).abs() < 1e-3, "{normals:?}");
        assert!(normals[0][1].abs() < 1e-6, "{normals:?}");
        // Vertex 3 is reached by the flagged face alone, which then counts.
        assert!((normals[3][1] - 1.0).abs() < 1e-3, "{normals:?}");
        // Without flags both faces count at vertex 0.
        let plain = smooth_normals(&positions, &indices, &keys, &elements, None);
        assert!(plain[0][1] > 0.5 && plain[0][2] > 0.5, "{plain:?}");
    }

    /// An element's corners are listed last face first, then split into
    /// smoothing-key runs by the original's in-place swap: with keys
    /// A B A B A down the list, the A run keeps its order and the B run
    /// comes out reversed.
    #[test]
    fn element_rings_split_runs_by_the_swap_partition() {
        // Five faces; corner 0 of face f is vertex f, all one element.
        let mut indices = Vec::new();
        for f in 0..5u32 {
            indices.extend_from_slice(&[f, 5 + 2 * f, 6 + 2 * f]);
        }
        let mut elements = vec![0u32; 5];
        elements.extend(1..=10);
        let mut keys: Vec<u32> = (0..5).map(|f| f % 2).collect();
        keys.extend(2..=11);
        let (counts, ring, _) = element_rings(&indices, &elements, &keys);
        let first: Vec<u32> = ring[counts[0] as usize..counts[1] as usize]
            .iter()
            .map(|&(face, corner)| {
                assert_eq!(corner, 0);
                face
            })
            .collect();
        assert_eq!(first, [4, 2, 0, 1, 3]);
    }

    /// The cloth pass's normals are the unweighted average of each group's
    /// face normals: two faces on different planes, in different groups,
    /// keep their own normals, and a flat quad's group averages to +Y.
    #[test]
    fn the_cloth_normal_builder_averages_face_normals() {
        let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3);
        let mut m = Mesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, -1.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            position_ids: vec![0, 1, 2, 3, 4, 5],
            indices: vec![0, 1, 2, 3, 4, 5],
            smoothing: vec![0, 0, 0, 1, 1, 1],
            position_count: 6,
            ..Mesh::default()
        };
        m.recompute_flat_normals();
        assert!(near(m.normals[0], [0.0, 1.0, 0.0]), "{:?}", m.normals);
        assert!(near(m.normals[3], [1.0, 0.0, 0.0]), "{:?}", m.normals);
        // A flat quad in one group averages its two +Y faces to +Y.
        let mut quad = Mesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
            ],
            position_ids: vec![0, 1, 2, 3],
            indices: vec![0, 1, 2, 0, 2, 3],
            smoothing: vec![0; 4],
            position_count: 4,
            ..Mesh::default()
        };
        quad.recompute_flat_normals();
        assert!(quad.normals.iter().all(|n| near(*n, [0.0, 1.0, 0.0])));
    }

    #[test]
    fn the_corner_weight_is_the_original_angle_approximation() {
        // A right angle: cos 0, so the polynomial alone, near π/2.
        let weight = corner_angle_weight([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert!((weight - 1.570_999_979_972_839_4).abs() < 1e-6, "{weight}");
        // Parallel edges (cos 1) carry no weight; opposite ones (cos −1)
        // the approximation of π: √2·(1.571 + 0.1945 + 0.111 + 0.111).
        assert_eq!(corner_angle_weight([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]), 0.0);
        let opposite = corner_angle_weight([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]);
        assert!((opposite - 2.810_749).abs() < 1e-5, "{opposite}");
        assert_eq!(corner_angle_weight([0.0; 3], [1.0, 0.0, 0.0]), 0.0);
        // A 60° corner: cos = ½, so √½·(1.571 − 0.09725 + 0.02775 −
        // 0.013875) = 1.05191, the approximation of π/3 = 1.04720.
        let weight = corner_angle_weight([1.0, 0.0, 0.0], [0.5, 0.75f32.sqrt(), 0.0]);
        assert!((weight - 1.051_91).abs() < 1e-4, "{weight}");
        assert!((weight - std::f32::consts::FRAC_PI_3).abs() < 5e-3);
        // A product of squared lengths under the original's 1.19e-7 floor
        // (here 1e-16) carries no weight.
        assert_eq!(corner_angle_weight([1e-4, 0.0, 0.0], [0.0, 1e-4, 0.0]), 0.0);
    }

    #[test]
    fn impossible_counts_are_rejected() {
        let mut c = Cursor::new(&[0xFF, 0xFF, 0xFF, 0x7F]);
        assert!(c.count_minus_one(4).is_err());
    }

    /// A one-triangle mesh in the node's space, with three vertices and
    /// every attribute stream, for the assembly tests.
    fn piece(material: &str, offset: f32, colour: [u8; 4], smoothing: u32) -> Mesh {
        Mesh {
            positions: vec![
                [offset, 0.0, 0.0],
                [offset + 1.0, 0.0, 0.0],
                [offset, 1.0, 0.0],
            ],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            uvs: Some(vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]),
            colors: Some(vec![colour; 3]),
            skin: None,
            position_ids: vec![0, 1, 2],
            indices: vec![0, 1, 2],
            groups: vec![MeshGroup {
                material: material.into(),
                first_index: 0,
                index_count: 3,
            }],
            triangle_flags: Some(vec![0x20]),
            triangle_edges: Some(vec![1]),
            gi_points: Vec::new(),
            render_flags: 0,
            position_count: 3,
            smoothing: vec![smoothing; 3],
            uv_ids: vec![0, 1, 2],
            uv_count: 3,
        }
    }

    #[test]
    fn merging_appends_parts_by_material() {
        let mut left = piece("mat", 0.0, [10, 20, 30, 255], 4);
        let right = piece("mat", 10.0, [40, 50, 60, 128], 9);
        left.merge(&right).unwrap();
        assert_eq!(left.positions.len(), 6);
        assert_eq!(left.positions[3], [10.0, 0.0, 0.0]);
        assert_eq!(left.normals.len(), 6);
        assert_eq!(left.indices, [0, 1, 2, 3, 4, 5]);
        assert_eq!(left.position_ids, [0, 1, 2, 3, 4, 5]);
        assert_eq!(left.position_count, 6);
        assert_eq!(left.uv_ids, [0, 1, 2, 3, 4, 5]);
        assert_eq!(left.uv_count, 6);
        assert_eq!(left.colors.as_deref().unwrap()[3], [40, 50, 60, 128]);
        // Smoothing keys stay apart: the second part's 9 becomes 9 + 4 + 1.
        assert_eq!(left.smoothing, [4, 4, 4, 14, 14, 14]);
        assert_eq!(left.triangle_flags.as_deref(), Some(&[0x20, 0x20][..]));
        assert_eq!(left.triangle_edges.as_deref(), Some(&[1u8, 1][..]));
        // The triangles join one material group, its range extended.
        assert_eq!(left.groups.len(), 1);
        assert_eq!(
            (left.groups[0].material.as_str(), left.groups[0].first_index),
            ("mat", 0)
        );
        assert_eq!(left.groups[0].index_count, 6);
    }

    #[test]
    fn merging_a_new_material_adds_a_group_and_fills_missing_streams() {
        let mut left = piece("mat", 0.0, [10, 20, 30, 255], 0);
        left.uvs.as_mut().unwrap()[0] = [0.25, 0.5];
        let mut other = piece("other", 10.0, [40, 50, 60, 128], 0);
        other.uvs = None;
        other.colors = None;
        other.triangle_flags = None;
        other.triangle_edges = None;
        left.merge(&other).unwrap();
        assert_eq!(left.groups.len(), 2);
        assert_eq!(left.groups[1].material, "other");
        assert_eq!(
            (left.groups[1].first_index, left.groups[1].index_count),
            (3, 3)
        );
        // The part has no texture coordinates: its triangles index element
        // 0 of the destination's stream, which is the first vertex's.
        assert_eq!(left.uvs.as_deref().unwrap()[3], [0.25, 0.5]);
        assert_eq!(left.uv_ids, [0, 1, 2, 0, 0, 0]);
        assert_eq!(left.uv_count, 3);
        // The missing vertex colours join white.
        assert_eq!(left.colors.as_deref().unwrap()[3], [255; 4]);
        // Missing triangle flags join 1 (smoothing group 1) and edge bits 7.
        assert_eq!(left.triangle_flags.as_deref(), Some(&[0x20, 1][..]));
        assert_eq!(left.triangle_edges.as_deref(), Some(&[1u8, 7][..]));
        // A malformed part leaves the mesh as it was.
        let before = left.clone();
        let mut bad = piece("other", 20.0, [0; 4], 0);
        bad.indices[2] = 7;
        assert!(left.merge(&bad).is_err());
        assert_eq!(left, before);
    }

    /// A destination without texture coordinates takes the part's: its own
    /// vertices index element 0 of the joined stream. Flags it lacks join
    /// as 1 for its own triangles.
    #[test]
    fn merging_a_part_with_streams_the_destination_lacks() {
        let mut left = piece("mat", 0.0, [0; 4], 0);
        left.uvs = None;
        left.uv_ids = Vec::new();
        left.uv_count = 0;
        left.colors = None;
        left.triangle_flags = None;
        let mut part = piece("mat", 10.0, [40, 50, 60, 128], 0);
        part.uvs.as_mut().unwrap()[0] = [0.75, 0.125];
        left.merge(&part).unwrap();
        assert_eq!(left.uvs.as_deref().unwrap()[0], [0.75, 0.125]);
        assert_eq!(left.uvs.as_deref().unwrap()[4], [1.0, 0.0]);
        assert_eq!(left.uv_ids, [0, 0, 0, 0, 1, 2]);
        assert_eq!(left.uv_count, 3);
        assert_eq!(left.colors.as_deref().unwrap()[0], [255; 4]);
        assert_eq!(left.colors.as_deref().unwrap()[3], [40, 50, 60, 128]);
        assert_eq!(left.triangle_flags.as_deref(), Some(&[1, 0x20][..]));
    }

    #[test]
    fn registered_handlers_take_every_node_sub_chunk() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static SEEN: AtomicUsize = AtomicUsize::new(0);
        fn handler(tag: u32, body: &[u8]) -> bool {
            if tag == 0x1234_5678 && body == [9, 8, 7, 6] {
                SEEN.fetch_add(1, Ordering::SeqCst);
            }
            true
        }
        let mut sector = node_with("Sector", 0, &[]);
        sector.extend(chunk(0x1234_5678, &[9, 8, 7, 6]));
        let file = {
            let mut list = u32s(&[1]);
            list.extend(chunk(NODE_SECTOR, &sector));
            let mut file = vec![0xCF, 0xAF, 0x23, 0x3D];
            file.extend(chunk(NODE_LIST, &list));
            // A root chunk never reaches the registry.
            file.extend(chunk(0x1234_5678, &[9, 8, 7, 6]));
            file
        };
        let mut handlers = ChunkHandlers::new();
        handlers.register(0x1234_5678, handler);
        let scene = Scene::parse_with(&file, &handlers).unwrap();
        assert_eq!(SEEN.load(Ordering::SeqCst), 1);
        // The registry replaces the reader's own sub-chunk loop.
        assert!(scene.nodes[0].unknown_sub_chunks.is_empty());
        assert_eq!(scene.unknown_chunks, [0x1234_5678]);
        // Without handlers the reader records the tag itself.
        let scene = Scene::parse(&file).unwrap();
        assert_eq!(scene.nodes[0].unknown_sub_chunks, [0x1234_5678]);
        assert_eq!(SEEN.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn merging_into_an_empty_mesh_takes_the_part() {
        let mut empty = Mesh {
            render_flags: 0x0002_0000,
            ..Default::default()
        };
        let part = piece("mat", 0.0, [1, 2, 3, 4], 0);
        empty.merge(&part).unwrap();
        assert_eq!(empty.positions, part.positions);
        assert_eq!(empty.indices, part.indices);
        assert_eq!(empty.render_flags, 0x0002_0000);
        // An empty part changes nothing.
        let before = empty.clone();
        empty.merge(&Mesh::default()).unwrap();
        assert_eq!(empty, before);
    }
}
