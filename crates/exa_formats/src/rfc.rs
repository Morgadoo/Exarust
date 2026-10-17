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
    /// Objects from the object databases placed in the level (items, doors,
    /// containers).
    pub placed_objects: Vec<PlacedObject>,
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
    pub body: NodeBody,
}

impl Node {
    /// Node flag: the node is level geometry. The original never registers
    /// the rigid body of a node marked so (its meshes stay static
    /// colliders): a level's own nodes can carry it in the file (the tavern
    /// table of `arenao1`), and a prop placed static
    /// ([`crate::level::Prop::STATIC`]) sets it on every node of the prop.
    pub const STATIC: u32 = 0x8_0000;

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
    /// The node reuses another object's data; nothing else is stored.
    Instance(u32),
    /// A plain node carrying a particle emitter record (`0x3DD0E002` is a
    /// fire): its type and size.
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
    /// Per-triangle flags (sub-chunk `0x3DC3`): seam classes in the high
    /// byte, apparel cull bits `0x200000`/`0x400000`.
    pub triangle_flags: Option<Vec<u32>>,
    /// Points sampling the surfaces on a ~40 cm lattice (sub-chunk
    /// `0x3D0CEC05`, after the mesh body), in metres: they mark the mesh's
    /// cells as solid for the global illumination volume.
    pub gi_points: Vec<[f32; 3]>,
    /// The word before the streams in newer files: skinning mode flags.
    pub skin_flags: u32,
    /// Elements of the position stream: [`Mesh::position_ids`] index it,
    /// and a morph set holds one offset per element.
    pub position_count: usize,
    /// Smoothing key of each vertex: vertices sharing a key share a normal
    /// ([`Mesh::recompute_normals`]).
    pub smoothing: Vec<u32>,
}

impl Mesh {
    /// Works the normals out again from the positions, as at load: the
    /// area-weighted face normals summed over the vertices sharing a
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
        // Triangles naming a missing vertex are left out.
        let indices: Vec<u32> = self
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .filter(|t| t.iter().all(|&i| (i as usize) < n))
            .flatten()
            .copied()
            .collect();
        self.normals = smooth_normals(&self.positions, &indices, keys);
    }
}

/// A level's local database: sub-chunks for its objects, strings and
/// placed objects.
pub const LOCAL_DATABASE: u32 = 0xDBC0;
const LOCAL_OBJECTS: u32 = 0xDBDB;

const PLACED_OBJECTS: u32 = 0xDBCE;
const PLACED_OBJECT: u32 = 0xDB01;

/// An object-database entry placed in a level.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedObject {
    /// `0x80000000`: the level's own database; `0x40000000`: the world
    /// database; otherwise `objdb.rdb`. The low 30 bits are the record id.
    pub id: u32,
    pub flags: u32,
    /// Row-major 3×4: three rotation rows, then the translation (metres).
    pub transform: [[f32; 3]; 4],
    /// Slot in a containing object (0 in the world).
    pub slot: i32,
}

impl PlacedObject {
    pub const ID_MASK: u32 = 0x3FFF_FFFF;
    pub fn is_local(&self) -> bool {
        self.id & 0x8000_0000 != 0
    }
}

fn read_local_database(b: &[u8], version: u16) -> Result<(Vec<Item>, Vec<PlacedObject>)> {
    let (mut items, mut objects) = (Vec::new(), Vec::new());
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
            LOCAL_OBJECTS => items = crate::objdb::items(&Database::parse(body)?),
            PLACED_OBJECTS => objects = read_placed_objects(body, version)?,
            _ => {}
        }
        pos += 8 + size;
    }
    Ok((items, objects))
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
        o.u32()?; // object flags
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
        out.push(PlacedObject {
            id,
            flags,
            transform,
            slot,
        });
    }
    Ok(out)
}

/// Per-triangle flag sub-chunk of the triangle list.
pub const TRIANGLE_FLAGS: u32 = 0x3DC3;

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

impl Scene {
    pub fn parse(b: &[u8]) -> Result<Self> {
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
        let mut placed_objects = Vec::new();
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
                match read_local_database(chunk.b, version) {
                    Ok((items, objects)) => (local_items, placed_objects) = (items, objects),
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
                    nodes.push(read_node(kind, node, version)?);
                }
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
            placed_objects,
            level_errors,
        })
    }

    pub fn material(&self, name: &str) -> Option<&Material> {
        self.materials
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .and_then(|(_, m)| m.as_ref().ok())
    }

    pub fn meshes(&self) -> impl Iterator<Item = (&Node, &Mesh)> {
        self.nodes.iter().filter_map(|n| match &n.body {
            NodeBody::Mesh(m) => Some((n, m.as_ref())),
            _ => None,
        })
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
/// Sub-chunk of a keyframe track (`key_track::KeyTrack`), which may follow
/// any stored node's body.
pub const KEY_TRACK: u32 = 0x3AC1;
/// Sub-chunk of a morph set (`morph::MorphSet`) after a mesh body: the
/// physique targets of a race's morph body.
pub const MORPH: u32 = 0xEDC0_C589;
/// Light body flags under which the body ends with four more words (one
/// skipped, then three values the original keeps for the flicker and the
/// fade; not decoded).
const LIGHT_EXTRA_FLAGS: u32 = 0x3_0000;
/// Bytes of those words.
const LIGHT_EXTRA_LEN: usize = 16;
/// Bytes of the body of node kind `0x3D08`: two words, not decoded.
const NODE_3D08_BODY_LEN: usize = 8;
const NODE_3D08: u32 = 0x3D08;

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
}

fn read_sub_chunks(c: &mut Cursor<'_>) -> SubChunks {
    let mut out = SubChunks::default();
    let mut points = None;
    while c.remaining() >= 8 {
        let Ok((tag, mut sub)) = c.chunk() else {
            break;
        };
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

fn emitter(body: &[u8]) -> Option<NodeBody> {
    let tail = body.len().checked_sub(12).map(|at| &body[at..])?;
    let word = |o: usize| u32::from_le_bytes(tail[o..o + 4].try_into().expect("4 bytes"));
    let kind = word(0);
    (kind & 0xFFFF_FF00 == EMITTER && word(4) == 4).then(|| NodeBody::Emitter {
        kind,
        size: f32::from_bits(word(8)),
    })
}

fn read_node(kind: u32, mut c: Cursor<'_>, version: u16) -> Result<Node> {
    let name = c.name()?;
    let flags = c.u32()?;
    let flags2 = c.u32()?;
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
        match read_mesh(&mut c, version) {
            Ok(mut m) => {
                sub = read_sub_chunks(&mut c);
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
            Ok(l) => {
                if l.flags & LIGHT_EXTRA_FLAGS == 0 || c.skip(LIGHT_EXTRA_LEN).is_ok() {
                    sub = read_sub_chunks(&mut c);
                }
                NodeBody::Light(l)
            }
            Err(e) => NodeBody::Invalid(e.to_string()),
        }
    } else if kind == NODE_VOXEL_LIGHT {
        match read_voxel_light(&mut c) {
            Ok(l) => {
                sub = read_sub_chunks(&mut c);
                NodeBody::VoxelLight(l)
            }
            Err(e) => NodeBody::Invalid(e.to_string()),
        }
    } else {
        let mut chunks = Cursor::new(c.b);
        chunks.pos = c.pos;
        if kind != NODE_3D08 || chunks.skip(NODE_3D08_BODY_LEN).is_ok() {
            sub = read_sub_chunks(&mut chunks);
        }
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
    let skin_flags = if version > VERSION_WITHOUT_MESH_PREFIX {
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

    let uvs = match (stream(0x10, StreamFormat::Vec2), map(KEY_UVS)) {
        (Some(s), Some(m)) => Some(
            gather(s, m)?
                .into_iter()
                .map(|i| {
                    let o = i * 8;
                    [f32_at(s.data, o), f32_at(s.data, o + 4)]
                })
                .collect(),
        ),
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
    let normals = smooth_normals(&positions, &indices, &smoothing);
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
        gi_points: Vec::new(),
        skin_flags,
        position_count,
        smoothing,
    })
}

/// Area-weighted face normals accumulated over vertices sharing a smoothing key.
fn smooth_normals(positions: &[[f32; 3]], indices: &[u32], keys: &[u32]) -> Vec<[f32; 3]> {
    let key_count = keys.iter().copied().max().map_or(0, |m| m as usize + 1);
    let mut acc = vec![[0.0f32; 3]; key_count];
    for t in indices.as_chunks::<3>().0 {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| positions[i as usize]);
        let u = sub(b, a);
        let v = sub(c, a);
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for &i in t {
            let k = keys[i as usize] as usize;
            for d in 0..3 {
                acc[k][d] += n[d];
            }
        }
    }
    keys.iter()
        .map(|&k| {
            let n = acc[k as usize];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > 0.0 {
                n.map(|x| x / len)
            } else {
                [0.0, 1.0, 0.0]
            }
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
        body.extend(u32s(&[0])); // version prefix
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
    fn impossible_counts_are_rejected() {
        let mut c = Cursor::new(&[0xFF, 0xFF, 0xFF, 0x7F]);
        assert!(c.count_minus_one(4).is_err());
    }
}
