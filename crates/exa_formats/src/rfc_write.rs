//! The `.rfc` writer: a scene written back into a chunk tree, as the
//! original does when it stores a visited level inside a save.
//!
//! The original's exporter walks its live scene, sorts the exported nodes
//! into four lists (scene nodes, props, placed objects, characters), numbers
//! them, then writes the root chunks in a fixed order: the tile grid, the
//! materials the meshes use that no shared library holds, the scene nodes,
//! the props, the local database with the placed objects, the characters and
//! the environment. [`Exporter::export`] reproduces that walk and every
//! chunk's encoding over a plain description of the scene
//! ([`ExportScene`]); [`ChunkWriter`] is the tag/size framing underneath it.
//! Everything is written in the file's units (centimetres) and the current
//! format version `0x3D23`, so [`crate::rfc::Scene::parse`] reads the result
//! back. Layout and evidence: `docs/formats/RFC.md` (Writing chunk trees).
//!
//! The scene is given as the original holds it: counts are lists, links of
//! a body may be kept as barycentric weights ([`ExportBody::weighted_links`]),
//! nodes carry their stale flags and export number from before the walk.
//! Inconsistent input (a mesh node without a mesh, a stream shorter than its
//! count, a reference past the node list) is an error, never a panic.

use crate::save_chunk::{SAVE_SETTINGS_ID, SettingsChunk};
use crate::{Error, Result};

/// The first word of a chunk tree: the sanity half `0xAFCF`, then the
/// version half the writer always stores, `0x3D23`.
pub const SANITY: u16 = 0xAFCF;
pub const VERSION: u16 = 0x3D23;

/// Root and sub-chunk tags the writer emits.
pub mod tag {
    pub const GRID: u32 = 0x3DED;
    pub const MATERIAL_LIST: u32 = 0xBA00;
    pub const MATERIAL: u32 = 0xBA01;
    pub const NODE_LIST: u32 = 0x3D00_0000;
    pub const NODE_PLAIN: u32 = 0x3D01;
    pub const NODE_MESH: u32 = 0x3D03;
    pub const NODE_LIGHT: u32 = 0x3D06;
    pub const NODE_TARGET: u32 = 0x3D08;
    pub const NODE_GROUP: u32 = 0x3D0A;
    pub const NODE_VOXEL_LIGHT: u32 = 0x3D0C;
    pub const TOPOLOGY: u32 = 0x3DC0;
    pub const TRIANGLES: u32 = 0x3DC1;
    pub const TRIANGLE_BYTES: u32 = 0x3DC2;
    pub const TRIANGLE_FLAGS: u32 = 0x3DC3;
    pub const GI_POINTS: u32 = 0x3D0C_EC05;
    pub const RIGID_BODY: u32 = 0x3DD0_B001;
    pub const CLOTH: u32 = 0x3DD0_C001;
    pub const FIRE: u32 = 0x3DD0_E002;
    pub const EMITTER_E004: u32 = 0x3DD0_E004;
    pub const KEY_TRACK: u32 = 0x3AC1;
    /// The word every keyframe track starts with.
    pub const KEY_TRACK_WORD: u32 = 0x3AE2;
    pub const SPHERES: u32 = 0xCD00;
    pub const LINKS: u32 = 0xDDB0;
    pub const SOUNDS: u32 = 0xDDB3;
    pub const PROPS: u32 = 0x3DE1_0000;
    pub const PROP: u32 = 0x3DE1_0100;
    /// A body part's placement under a prop or placed object.
    pub const PART_BODY: u32 = 0x3DE0_ECAC;
    /// A cloth part's particles under a prop or placed object.
    pub const PART_CLOTH: u32 = 0x3DE0_ECDB;
    pub const LOCAL_DATABASE: u32 = 0xDBC0;
    pub const LOCAL_OBJECTS: u32 = 0xDBDB;
    pub const LOCAL_STRINGS: u32 = 0xDBCD;
    pub const PLACED_OBJECTS: u32 = 0xDBCE;
    pub const PLACED_OBJECT: u32 = 0xDB01;
    pub const CHARACTERS: u32 = 0xCAC0_0001;
    pub const CHARACTER_DB: u32 = 0xCECD_DB00;
    pub const PLACEMENT: u32 = 0xCAC0_CA00;
    pub const POSE_TREE: u32 = 0xCAC0_EB00;
    pub const POSE_FLAT: u32 = 0xCAC0_E100;
    pub const MOTION: u32 = 0xCAC0_EA00;
    pub const AI_HOME: u32 = 0xCACE_0F77;
    pub const ATTACHED: u32 = 0xCAC0_EC00;
    pub const ENVIRONMENT: u32 = 0xEC00_DC00;
}

/// Magic words of the two databases the exporter embeds.
pub const OBJECT_DB_MAGIC: u32 = 0xDBAC_0D09;
pub const CHARACTER_DB_MAGIC: u32 = 0xDBAC_CD00;

/// Bytes of the environment record.
pub const ENVIRONMENT_LEN: usize = 0xF8;
/// The environment record is left out when its first word has this bit.
pub const ENVIRONMENT_UNSAVED: u32 = 0x100;

/// Material name a mesh part without a material stores.
pub const DEFAULT_MATERIAL: [u8; 16] = *b"_Default\0\0\0\0\0\0\0\0";

/// Row-major 3×4: three rotation rows, then the translation (cm).
pub type Matrix = [[f32; 3]; 4];
/// The identity placement a missing held item's slot stores.
pub const IDENTITY: Matrix = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0; 3]];

/// Node flags the walk reads.
pub mod flags {
    /// Low 12 bits: the node's class.
    pub const CLASS: u32 = 0xFFF;
    /// A node owned by a prop or placed object: never walked; under one, it
    /// can give a part record.
    pub const OWNED: u32 = 0x1000;
    pub const PROP: u32 = 0x2000;
    /// With [`OWNED`] and [`PROP`] clear: a placed object.
    pub const OBJECT: u32 = 0x4000;
    pub const OBJECT_MASK: u32 = 0x7000;
    /// The node carries an attachment (body, cloth, emitter) to write.
    pub const ATTACHED: u32 = 0x1_0000;
    /// The node is saved even when its class is not 1.
    pub const SAVED: u32 = 0x4_0000;
    /// Set by the walk on every node it exports.
    pub const EXPORTED: u32 = 0x100_0000;
    pub const CHARACTER: u32 = 0x1000_0000;
    /// Stored of a node's flags.
    pub const HEADER_MASK: u32 = 0x7CF9_FFFF;
    /// Stored of a node's second flag word.
    pub const HEADER_MASK2: u32 = 0xFFFF_FF70;
    /// Stored of a prop's or placed object's flags.
    pub const PLACEMENT_MASK: u32 = 0xB8_0000;
}

/// Node classes that pick a chunk tag.
pub mod class {
    pub const PLAIN: u32 = 1;
    pub const TARGET: u32 = 3;
    pub const LIGHT: u32 = 5;
    pub const GROUP: u32 = 0x11;
    pub const VOXEL_LIGHT: u32 = 0x41;
    pub const MESH: u32 = 0x89;
}

/// Little-endian output with the original's chunk framing: opening a chunk
/// writes its tag and a zero size and remembers where the size sits;
/// closing it stores the bytes written since the size word.
#[derive(Debug, Clone, Default)]
pub struct ChunkWriter {
    buf: Vec<u8>,
    open: Vec<usize>,
}

impl ChunkWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bytes written so far.
    pub fn position(&self) -> usize {
        self.buf.len()
    }

    /// Chunks opened and not yet closed.
    pub fn depth(&self) -> usize {
        self.open.len()
    }

    pub fn bytes_written(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn i32(&mut self, v: i32) {
        self.u32(v as u32);
    }

    pub fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }

    pub fn vec3(&mut self, v: [f32; 3]) {
        for x in v {
            self.f32(x);
        }
    }

    pub fn matrix(&mut self, m: &Matrix) {
        for row in m {
            self.vec3(*row);
        }
    }

    /// Opens a chunk: its tag and a size word filled in by [`Self::close`].
    pub fn open(&mut self, tag: u32) {
        self.u32(tag);
        self.open.push(self.buf.len());
        self.u32(0);
    }

    /// Closes the innermost chunk, storing its payload's length.
    pub fn close(&mut self) -> Result<()> {
        let at = self.pop()?;
        let size = (self.buf.len() - at - 4) as u32;
        self.patch_u32(at, size)
    }

    /// Takes the innermost chunk back out, tag included, as if it had
    /// never been opened.
    pub fn discard(&mut self) -> Result<()> {
        let at = self.pop()?;
        self.buf.truncate(at - 4);
        Ok(())
    }

    /// Overwrites a word already written.
    pub fn patch_u32(&mut self, at: usize, v: u32) -> Result<()> {
        let len = self.buf.len();
        let slot = self.buf.get_mut(at..at + 4).ok_or(Error::Truncated {
            offset: at,
            needed: 4,
            len,
        })?;
        slot.copy_from_slice(&v.to_le_bytes());
        Ok(())
    }

    fn pop(&mut self) -> Result<usize> {
        self.open
            .pop()
            .ok_or_else(|| Error::Malformed("closing a chunk that was never opened".into()))
    }
}

/// A scene to export: its nodes in tree order, each naming its parent.
#[derive(Debug, Clone, Default)]
pub struct ExportScene {
    /// Children follow their parents' order of appearance in this list.
    pub nodes: Vec<SceneNode>,
    pub grid: Option<ExportGrid>,
    /// Names held by the shared material libraries: a mesh's material
    /// listed here is not written into the file.
    pub shared_materials: Vec<[u8; 16]>,
    /// The scene's own material library: name and payload.
    pub material_library: Vec<([u8; 16], Vec<u8>)>,
    /// The level's own object database, written when it holds a record.
    pub object_db: Option<RdbImage>,
    /// The object database's strings, as their own writer produced them.
    pub object_strings: Vec<u8>,
    /// The level's own character database, written when it holds a record.
    pub character_db: Option<RdbImage>,
    /// The 248-byte environment record.
    pub environment: Option<Vec<u8>>,
    /// The export covers the game's whole scene: only then is the
    /// environment written.
    pub whole_scene: bool,
}

/// A scene node as the walk sees it.
#[derive(Debug, Clone, Default)]
pub struct SceneNode {
    pub name: [u8; 16],
    /// [`flags`]; the low 12 bits are the class ([`class`]). Bit
    /// [`flags::EXPORTED`] may be stale from an earlier export: the walk
    /// sets or clears it only on the nodes it reaches.
    pub flags: u32,
    pub flags2: u32,
    /// Index of the parent in [`ExportScene::nodes`]; `None` under the
    /// scene's root.
    pub parent: Option<usize>,
    /// Placement in the parent's frame (cm).
    pub local: Matrix,
    /// Placement in the world (cm): written for a node whose parent is not
    /// exported.
    pub world: Matrix,
    pub half_extents: [f32; 3],
    pub centre: [f32; 3],
    pub radius: f32,
    /// The word after the radius (a prop's extra word, a placed object's
    /// field).
    pub tail: u32,
    /// The node's export number before the walk: the walk renumbers only
    /// scene nodes, props and placed objects.
    pub index: i32,
    /// The node whose data this one shares (`None`: its own). The exported
    /// sharer with the lowest number is stored, and the others store only
    /// a reference to it.
    pub shares: Option<usize>,
    pub body: NodeData,
    pub attachment: Option<Attachment>,
    pub tracks: Vec<ExportTrack>,
    /// For a character node ([`flags::CHARACTER`]).
    pub character: Option<ExportCharacter>,
    /// For a placed object.
    pub object: Option<ObjectIds>,
}

/// What follows a node's header, by class.
#[derive(Debug, Clone, Default)]
pub enum NodeData {
    #[default]
    None,
    Mesh(Box<ExportMesh>),
    Light(ExportLight),
    VoxelLight(ExportVoxelLight),
    /// Class 3: another node's export number and a word (guess: an aim
    /// target).
    Target {
        node: usize,
        value: u32,
    },
}

/// A mesh as the original holds it: streams, unified vertices made of
/// index maps, parts and triangles.
#[derive(Debug, Clone, Default)]
pub struct ExportMesh {
    /// The node's render word; the low byte is not stored.
    pub render_word: u32,
    pub streams: Vec<VertexStream>,
    /// Unified vertices: every index map holds this many indices.
    pub vertex_count: u32,
    pub parts: Vec<MeshPart>,
    pub maps: Vec<IndexMap>,
    /// Triangle corners into the unified vertices (three per triangle);
    /// empty: no triangle chunk.
    pub triangles: Vec<u32>,
    /// One byte per triangle (`0x3DC2`).
    pub triangle_bytes: Option<Vec<u8>>,
    /// One word per triangle (`0x3DC3`).
    pub triangle_flags: Option<Vec<u32>>,
    /// A sub-chunk the mesh keeps as it was read, written back whole after
    /// the attachment (tag, payload; which tags occur is not traced).
    pub kept_chunk: Option<(u32, Vec<u8>)>,
    pub gi_points: Option<GiPoints>,
}

/// A vertex stream: id, format word and `count` elements.
#[derive(Debug, Clone, Default)]
pub struct VertexStream {
    pub id: u32,
    /// Bits 16..23 give the element size (`0xF3` 12, `0xF2` 8, `0xB4` 4,
    /// `0xA4` 32; any other stores no data); bit 24 keeps the stream out
    /// of the file.
    pub format: u32,
    pub count: u32,
    pub data: Vec<u8>,
}

/// Format bit of a stream that is not saved.
pub const STREAM_UNSAVED: u32 = 0x100_0000;

#[derive(Debug, Clone, Default)]
pub struct MeshPart {
    /// First and last unified vertex.
    pub vertices: [u32; 2],
    /// First and last triangle.
    pub triangles: [u32; 2],
    /// `None`: stored as [`DEFAULT_MATERIAL`].
    pub material: Option<[u8; 16]>,
}

#[derive(Debug, Clone, Default)]
pub struct IndexMap {
    pub key: u32,
    pub indices: Vec<u32>,
}

/// Unified vertex counts from this one on store 32-bit indices.
pub const WIDE_INDICES: i32 = 0xFFFF;

#[derive(Debug, Clone, Default)]
pub struct GiPoints {
    /// Lattice spacing (cm).
    pub spacing: u32,
    pub points: Vec<[f32; 3]>,
}

/// An omni light's body.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExportLight {
    /// Body flags; under [`LIGHT_EXTRA`] the animation follows.
    pub flags: u32,
    pub color: [f32; 3],
    pub multiplier: f32,
    pub intensity: f32,
    /// cm.
    pub radius: f32,
    /// The word the original keeps unread.
    pub unread: f32,
    pub softness: f32,
    /// Written when the node's `flags2 & 0x300` is `0x200`.
    pub spot: [f32; 2],
    /// Written after a zero word when `flags & LIGHT_EXTRA`.
    pub animation: LightAnimation,
}

pub const LIGHT_EXTRA: u32 = 0x3_0000;
pub const SPOT_MASK: u32 = 0x300;
pub const SPOT: u32 = 0x200;

/// A light body's flicker parameters, stored after a zero word when its
/// flags have any of [`LIGHT_EXTRA`], in the order the file holds them.
#[derive(Debug, Clone, Copy, Default)]
pub struct LightAnimation {
    /// How far the flicker moves the intensity.
    pub amplitude: f32,
    /// How fast the walk and a fade move per frame millisecond.
    pub speed: f32,
    /// How much of the walk the parent mesh's emissive takes.
    pub emissive: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExportVoxelLight {
    pub color: [f32; 3],
    pub intensity: f32,
    /// cm.
    pub radius: f32,
}

/// Something attached to a node: a rigid body, cloth, an emitter. The
/// class word decides which record is written.
#[derive(Debug, Clone)]
pub struct Attachment {
    pub class: u32,
    /// The word a part record stores after the part's name.
    pub id: u32,
    pub data: AttachmentData,
}

#[derive(Debug, Clone)]
pub enum AttachmentData {
    Body(Box<ExportBody>),
    Cloth {
        /// The cloth marker's word.
        marker: u32,
        /// Particle positions (cm).
        particles: Vec<[f32; 3]>,
    },
    Emitter {
        /// `0x3DD0E002` (fire) or `0x3DD0E004`.
        kind: u32,
        size: f32,
    },
    None,
}

/// Top byte of an attachment's class word: which node sub-chunk it writes.
pub const ATTACH_KIND: u32 = 0xFF00_0000;
pub const ATTACH_BODY: u32 = 0x1000_0000;
pub const ATTACH_CLOTH: u32 = 0x6000_0000;
pub const ATTACH_EMITTER: u32 = 0xA000_0000;

/// A rigid body as the original holds it.
#[derive(Debug, Clone, Default)]
pub struct ExportBody {
    pub mobility: f32,
    /// The particles' rest points (cm).
    pub rest: [[f32; 3]; 4],
    pub point_masses: [f32; 4],
    pub friction: f32,
    pub group_mask: u32,
    /// The last header word (bits as held).
    pub last: u32,
    /// Collision spheres: centre (cm), radius (cm).
    pub spheres: Vec<[f32; 4]>,
    /// The links keep their points as barycentric weights over the rest
    /// points (body flag `0x10000`): the writer turns them back into points.
    pub weighted_links: bool,
    /// Subtracted from a weighted link's parent point (cm).
    pub centre: [f32; 3],
    pub links: Vec<ExportLink>,
    pub sounds: Vec<ExportSound>,
}

#[derive(Debug, Clone, Default)]
pub struct ExportLink {
    /// Link flags: `0x4` slider (an axis follows), `0x10000` the parent
    /// point is weights over the parent body's rest points, `0x400000`
    /// ends the saved links.
    pub flags: u32,
    /// The point on this body: a point (cm, three values), or four weights.
    pub on_body: [f32; 4],
    /// The point on the parent: a point, or four weights.
    pub on_parent: [f32; 4],
    /// The parent body's rest points, for parent weights.
    pub parent_rest: Option<[[f32; 3]; 4]>,
    pub slack: f32,
    pub strength: f32,
    pub smoothing: f32,
    pub axis: [f32; 3],
}

/// Link flags the writer reads.
pub const LINK_SLIDER: u32 = 0x4;
pub const LINK_PARENT_WEIGHTS: u32 = 0x1_0000;
pub const LINK_UNSAVED: u32 = 0x40_0000;
/// Body flag: the links hold weights.
pub const BODY_WEIGHTED_LINKS: u32 = 0x1_0000;

#[derive(Debug, Clone, Default)]
pub struct ExportSound {
    pub id: u32,
    pub point: [f32; 3],
    pub sound: [u8; 16],
    pub volume: f32,
    pub pitch: f32,
}

/// A keyframe controller of a node.
#[derive(Debug, Clone, Default)]
pub struct ExportTrack {
    /// The controller drives the node's translation; others are not saved.
    pub on_translation: bool,
    pub mode: u32,
    pub duration: f32,
    /// Time (s) and position (cm).
    pub keys: Vec<(f32, [f32; 3])>,
}

/// The tile grid of a dungeon level.
#[derive(Debug, Clone, Default)]
pub struct ExportGrid {
    pub wall_sets: Vec<[u8; 16]>,
    pub floor_sets: Vec<[u8; 16]>,
    pub width: u32,
    pub height: u32,
    pub plane_a: Vec<u32>,
    pub plane_b: Vec<u32>,
    pub plane_c: Vec<u32>,
}

/// The grid's two fixed words after the size.
pub const GRID_WORDS: [u32; 2] = [0, 600];

/// A database ready to write: `count` records, a table of `count + 1`
/// 16-byte entries and the data.
#[derive(Debug, Clone, Default)]
pub struct RdbImage {
    pub count: u32,
    pub table: Vec<u8>,
    pub data: Vec<u8>,
}

/// A placed object's database ids.
#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectIds {
    pub id: u32,
    pub placement: u32,
}

/// A character's state as the exporter reads it.
#[derive(Debug, Clone, Default)]
pub struct ExportCharacter {
    pub record: u32,
    /// The record's kind word: bits `0xFF00` equal to `0x1000` let the AI
    /// home be saved (guess: a story character).
    pub record_kind: u32,
    /// x and z of the stored position (cm); its y is not stored.
    pub body_position: [f32; 3],
    /// The stored position's y (cm).
    pub floor_height: f32,
    pub heading: f32,
    pub condition: i32,
    /// Bit [`STATE_POSED`] saves the pose of a living character.
    pub state_flags: u32,
    pub motion_state: u32,
    pub motion_value: f32,
    /// The body's parts as a tree ([`tag::POSE_TREE`]).
    pub pose: Option<PoseNode>,
    /// The body's parts in order ([`tag::POSE_FLAT`]).
    pub parts: Vec<Matrix>,
    /// The two held items' placements; an empty hand stores [`IDENTITY`].
    pub held: [Option<Matrix>; 2],
    pub ai: Option<ExportAi>,
    /// A class-1 node's stored bytes, written after its condition.
    pub tail: Vec<u8>,
    /// What each attached object wrote with its own writer; `None`: it
    /// wrote nothing.
    pub attachments: Vec<Option<Vec<u8>>>,
}

/// Condition of a dead character.
pub const DEAD: i32 = 0xA00;
pub const STATE_POSED: u32 = 0x10;
/// Record kind bits that allow the AI home, and their value.
pub const RECORD_KIND_MASK: u32 = 0xFF00;
pub const RECORD_KIND_HOME: u32 = 0x1000;

#[derive(Debug, Clone, Copy, Default)]
pub struct ExportAi {
    /// The AI is the character controller class (not another AI kind).
    pub motile: bool,
    /// Set, the home is not saved (meaning not traced).
    pub skip_home: bool,
    /// 1: the pose and the motion state are saved (meaning not traced).
    pub mode: i32,
    pub home: [f32; 3],
    pub home_heading: f32,
}

/// A body part of a pose tree.
#[derive(Debug, Clone, Default)]
pub struct PoseNode {
    pub matrix: Matrix,
    pub word: u32,
    /// Only children whose class has top byte `0x12` are written.
    pub class: u32,
    /// A part of kind [`POSE_LEAF_KIND`] writes no children.
    pub kind: u32,
    pub children: Vec<PoseNode>,
}

pub const POSE_CHILD_CLASS: u32 = 0x1200_0000;
pub const POSE_LEAF_KIND: u32 = 0x18;

/// The caller's own chunks, written after the environment.
pub type ExtraChunks<'a> = dyn FnMut(&mut ChunkWriter) -> Result<()> + 'a;

/// Export switches.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExportOptions {
    /// Texture layers of kind `0x10` are renamed to `<stem>_rfi` and given
    /// kind `0x40`, and the rename recorded ([`Exporter::texture_renames`]).
    pub rename_textures: bool,
    /// Without `rename_textures`: such layers take the extension `.rfi`.
    pub rfi_extension: bool,
    /// Leave the per-triangle sub-chunks out.
    pub skip_triangle_data: bool,
    /// Skip the meshes' part-preparation pass: the walk then calls no
    /// [`ExportHooks::prepare_part`] before collecting materials. The
    /// switch's purpose is not established; only its effect is (the
    /// original's option bit 3). Material collection still runs.
    pub skip_part_prepare: bool,
    /// Save-game export: characters store their AI home and motion state.
    pub save_mode: bool,
}

/// The scene's own objects get a chance to bring themselves up to date
/// before the exporter writes them, as the original's virtual calls do.
/// Every method does nothing by default; an exporter without hooks writes
/// the scene exactly as given.
pub trait ExportHooks {
    /// A mesh's part, before the node is collected (the original calls each
    /// part's own preparation method with its node, last part first). The
    /// part index is into [`ExportMesh::parts`].
    fn prepare_part(&mut self, _node: usize, _part: usize) {}

    /// A node's attached body, cloth or emitter, just before the node's
    /// chunk is opened. The original calls the attachment's own method
    /// there (guarded by node flag [`flags::ATTACHED`]).
    fn prepare_attachment(&mut self, _node: usize) {}

    /// A character written with its body (a class-1 node is written without
    /// one and is not offered), before the character chunks. The original
    /// refreshes the character record's condition words here
    /// ([`character_condition_words`]).
    fn prepare_character(&mut self, _node: usize) {}
}

/// Hooks that do nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoHooks;

impl ExportHooks for NoHooks {}

/// The four condition words the original refreshes on a character's record
/// before an export, in the order it stores them: the health share of the
/// maximum, the dazed share (zero unless it is above zero), the third word
/// scaled by 65535, and the fourth word, which is `0xFFFF` because the
/// original's helper returns its argument unchanged.
///
/// A share is `ROUND(value / max * 65535)` with the engine's `CVTSS2SI`:
/// halves round to even and a value the conversion cannot represent gives
/// the integer indefinite word (whose low half is zero).
pub fn character_condition_words(health: f32, dazed: f32, max: f32, third: f32) -> [u16; 4] {
    let dazed = if dazed > 0.0 {
        rounded_word(dazed / max * 65535.0)
    } else {
        0
    };
    [
        rounded_word(health / max * 65535.0),
        dazed,
        rounded_word(third * 65535.0),
        0xFFFF,
    ]
}

/// `CVTSS2SI` on a 32-bit float: round to nearest even and keep the low
/// half of the 64-bit result; NaN, an infinity or an out-of-range value
/// gives the integer indefinite value, `i64::MIN`, whose low half is zero.
fn rounded_word(value: f32) -> u16 {
    let rounded = value.round_ties_even();
    const LIMIT: f32 = 9_223_372_036_854_775_808.0;
    if rounded.is_nan() || !(-LIMIT..LIMIT).contains(&rounded) {
        0
    } else {
        rounded as i64 as u16
    }
}

/// A texture layer renamed by [`ExportOptions::rename_textures`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureRename {
    /// Lowercase `<stem>.rfi`.
    pub source: [u8; 16],
    /// `<stem>_rfi`, as the layer now names it.
    pub renamed: [u8; 16],
}

/// Texture layers: header word, length word; kind at 32, name at 40.
const LAYERS_START: usize = 0xB8;
const LAYER_SLOT_MASK: u32 = 0x0FFF_0000;
const LAYER_KIND: usize = 0x20;
const LAYER_NAME: usize = 0x28;
const LAYER_KIND_SOURCE: u32 = 0x10;
const LAYER_KIND_RENAMED: u32 = 0x40;
const SUFFIX_RENAMED: &[u8; 4] = b"_rfi";
const SUFFIX_RFI: &[u8; 4] = b".rfi";

/// Writes a level section for a save, with the story's settings-chunk hook:
/// the scene's chunk tree, then the `0xECADCA01` chunk with its twelve-byte
/// tag and two words. The arena, and a story save the gate
/// (`save_chunk::settings_chunk_written`) rejects, pass `None`.
pub fn export_save_level(
    scene: &ExportScene,
    options: &ExportOptions,
    settings: Option<SettingsChunk>,
) -> Result<Vec<u8>> {
    let mut extra = |w: &mut ChunkWriter| -> Result<()> {
        if let Some(settings) = settings {
            w.open(SAVE_SETTINGS_ID);
            w.bytes(&settings.tag);
            w.u32(settings.first);
            w.u32(settings.saved);
            w.close()?;
        }
        Ok(())
    };
    Exporter::new().export(scene, options, Some(&mut extra))
}

/// The exporter. Texture renames build up over every export it runs, as
/// the original's list does over a session.
#[derive(Debug, Clone, Default)]
pub struct Exporter {
    pub texture_renames: Vec<TextureRename>,
}

/// The lists the walk fills and the numbers it gives.
struct Walk<'a> {
    scene: &'a ExportScene,
    children: Vec<Vec<usize>>,
    flags: Vec<u32>,
    index: Vec<i32>,
    materials: Vec<[u8; 16]>,
    nodes: Vec<usize>,
    props: Vec<usize>,
    objects: Vec<usize>,
    characters: Vec<usize>,
}

/// A scene-node list entry: the node, its exported parent and the node
/// whose data it shares.
struct Entry {
    node: usize,
    parent: Option<usize>,
    source: Option<usize>,
}

impl Exporter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Writes a whole chunk tree. `extra`, when given, writes after the
    /// environment (the original's hook for the caller's own chunks).
    pub fn export(
        &mut self,
        scene: &ExportScene,
        options: &ExportOptions,
        extra: Option<&mut ExtraChunks<'_>>,
    ) -> Result<Vec<u8>> {
        let mut hooks = NoHooks;
        self.export_with(scene, options, extra, &mut hooks)
    }

    /// [`Self::export`] with the scene's preparation hooks: the walk offers
    /// each mesh part and the character writer offers each character with a
    /// body, and the node writer offers each attachment, exactly where the
    /// original calls the scene's own methods.
    pub fn export_with(
        &mut self,
        scene: &ExportScene,
        options: &ExportOptions,
        extra: Option<&mut ExtraChunks<'_>>,
        hooks: &mut dyn ExportHooks,
    ) -> Result<Vec<u8>> {
        let mut walk = Walk::new(scene)?;
        let roots: Vec<usize> = (0..scene.nodes.len())
            .filter(|&i| scene.nodes[i].parent.is_none())
            .collect();
        for r in roots {
            if scene.nodes[r].flags & flags::CLASS == class::GROUP {
                for c in walk.children[r].clone() {
                    walk.visit(c, options, hooks)?;
                }
            } else {
                walk.visit(r, options, hooks)?;
            }
        }
        let entries = walk.number()?;

        let mut w = ChunkWriter::new();
        w.u16(SANITY);
        w.u16(VERSION);
        if let Some(grid) = &scene.grid {
            write_grid(&mut w, grid)?;
        }
        if !walk.materials.is_empty() {
            w.open(tag::MATERIAL_LIST);
            w.u32(walk.materials.len() as u32);
            for name in &walk.materials {
                self.write_material(&mut w, scene, name, options)?;
            }
            w.close()?;
        }
        if !entries.is_empty() {
            w.open(tag::NODE_LIST);
            w.u32(entries.len() as u32);
            for e in &entries {
                walk.write_node(&mut w, e, options, hooks)?;
            }
            w.close()?;
        }
        if !walk.props.is_empty() {
            walk.write_props(&mut w)?;
        }
        if !walk.objects.is_empty() {
            walk.write_local_database(&mut w)?;
        }
        if !walk.characters.is_empty() {
            w.open(tag::CHARACTERS);
            w.u32(walk.characters.len() as u32);
            walk.write_characters(&mut w, options, hooks)?;
            w.close()?;
        }
        if scene.whole_scene
            && let Some(env) = &scene.environment
        {
            let record = env.get(..ENVIRONMENT_LEN).ok_or(Error::Truncated {
                offset: 0,
                needed: ENVIRONMENT_LEN,
                len: env.len(),
            })?;
            if u32::from_le_bytes(record[..4].try_into().expect("4 bytes")) & ENVIRONMENT_UNSAVED
                == 0
            {
                w.open(tag::ENVIRONMENT);
                w.bytes(record);
                w.close()?;
            }
        }
        if let Some(extra) = extra {
            extra(&mut w)?;
        }
        if w.depth() != 0 {
            return Err(Error::Malformed(format!(
                "{} chunks left open after the export",
                w.depth()
            )));
        }
        Ok(w.into_bytes())
    }

    /// A material of the list: written only when the scene's library holds
    /// it (the list's count includes it either way).
    fn write_material(
        &mut self,
        w: &mut ChunkWriter,
        scene: &ExportScene,
        name: &[u8; 16],
        options: &ExportOptions,
    ) -> Result<()> {
        let Some((_, payload)) = scene
            .material_library
            .iter()
            .find(|(n, _)| lowercase_name(n) == *name)
        else {
            return Ok(());
        };
        let mut payload = payload.clone();
        if options.rename_textures {
            self.rename_layers(&mut payload);
        } else if options.rfi_extension {
            for_each_source_layer(&mut payload, |layer| {
                let renamed = replace_extension(&name_at(layer), SUFFIX_RFI);
                layer[LAYER_NAME..LAYER_NAME + 16].copy_from_slice(&renamed);
            });
        }
        w.open(tag::MATERIAL);
        w.bytes(name);
        w.bytes(&payload);
        w.close()
    }

    fn rename_layers(&mut self, payload: &mut [u8]) {
        let renames = &mut self.texture_renames;
        for_each_source_layer(payload, |layer| {
            layer[LAYER_KIND..LAYER_KIND + 4].copy_from_slice(&LAYER_KIND_RENAMED.to_le_bytes());
            let name = name_at(layer);
            let renamed = replace_extension(&name, SUFFIX_RENAMED);
            let source = lowercase_name(&replace_extension(&name, SUFFIX_RFI));
            if !renames.iter().any(|r| r.source == source) {
                renames.push(TextureRename { source, renamed });
            }
            layer[LAYER_NAME..LAYER_NAME + 16].copy_from_slice(&renamed);
        });
    }
}

/// Runs `f` on every texture layer record of a material payload whose slot
/// bits are set and whose kind is `0x10`. The walk stops at a record of
/// length 0 or one that does not fit (the original would loop or read on).
fn for_each_source_layer(payload: &mut [u8], mut f: impl FnMut(&mut [u8])) {
    let mut at = LAYERS_START;
    while at < payload.len() {
        let Some(head) = payload.get(at..at + 8) else {
            return;
        };
        let slot = u32::from_le_bytes(head[..4].try_into().expect("4 bytes"));
        let len = u32::from_le_bytes(head[4..].try_into().expect("4 bytes")) as usize;
        if len == 0 {
            return;
        }
        if slot & LAYER_SLOT_MASK != 0 {
            let Some(layer) = payload.get_mut(at..at + LAYER_NAME + 16) else {
                return;
            };
            let kind = u32::from_le_bytes(
                layer[LAYER_KIND..LAYER_KIND + 4]
                    .try_into()
                    .expect("4 bytes"),
            );
            if kind == LAYER_KIND_SOURCE {
                f(layer);
            }
        }
        at += len;
    }
}

fn name_at(layer: &[u8]) -> [u8; 16] {
    layer[LAYER_NAME..LAYER_NAME + 16]
        .try_into()
        .expect("16 bytes")
}

/// A 16-byte name with `A`..`Z` lowered, as the original compares names.
pub fn lowercase_name(name: &[u8; 16]) -> [u8; 16] {
    name.map(|c| c.to_ascii_lowercase())
}

/// A name with its extension replaced: from the last `.` (or the end of
/// the name) on, the bytes become `suffix` and then zeros, cut at 16.
pub fn replace_extension(name: &[u8; 16], suffix: &[u8; 4]) -> [u8; 16] {
    let at = name
        .iter()
        .rposition(|&c| c == b'.')
        .unwrap_or_else(|| name.iter().position(|&c| c == 0).unwrap_or(16));
    let mut out = *name;
    for (i, c) in out.iter_mut().enumerate().skip(at) {
        *c = suffix.get(i - at).copied().unwrap_or(0);
    }
    out
}

/// A point from four barycentric weights over four rest points, summed in
/// the original's order.
pub fn weighted_point(w: [f32; 4], rest: &[[f32; 3]; 4]) -> [f32; 3] {
    std::array::from_fn(|k| {
        w[0] * rest[0][k] + w[1] * rest[1][k] + w[2] * rest[2][k] + w[3] * rest[3][k]
    })
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// `count − 1` as the original stores list sizes (an empty list stores −1).
fn last_index(len: usize) -> u32 {
    (len as u32).wrapping_sub(1)
}

fn short(what: &str, needed: usize, have: usize) -> Error {
    Error::Malformed(format!("{what}: needs {needed}, has {have}"))
}

impl<'a> Walk<'a> {
    fn new(scene: &'a ExportScene) -> Result<Self> {
        let n = scene.nodes.len();
        let mut children = vec![Vec::new(); n];
        for (i, node) in scene.nodes.iter().enumerate() {
            if let Some(p) = node.parent {
                children
                    .get_mut(p)
                    .ok_or_else(|| Error::Malformed(format!("node {i}: parent {p} out of {n}")))?
                    .push(i);
            }
        }
        Ok(Self {
            scene,
            children,
            flags: scene.nodes.iter().map(|n| n.flags).collect(),
            index: scene.nodes.iter().map(|n| n.index).collect(),
            materials: Vec::new(),
            nodes: Vec::new(),
            props: Vec::new(),
            objects: Vec::new(),
            characters: Vec::new(),
        })
    }

    fn node(&self, i: usize) -> Result<&'a SceneNode> {
        self.scene
            .nodes
            .get(i)
            .ok_or_else(|| Error::Malformed(format!("node {i} out of {}", self.scene.nodes.len())))
    }

    fn exported(&self, i: usize) -> bool {
        self.flags[i] & (flags::EXPORTED | flags::OWNED) == flags::EXPORTED
    }

    /// The collection pass over one node and its subtree.
    fn visit(
        &mut self,
        i: usize,
        options: &ExportOptions,
        hooks: &mut dyn ExportHooks,
    ) -> Result<()> {
        let f = self.flags[i];
        if f & flags::OWNED != 0 {
            return Ok(());
        }
        if f & flags::SAVED == 0 && f & flags::CLASS != class::PLAIN {
            self.flags[i] = f & !flags::EXPORTED;
            return Ok(());
        }
        let f = f | flags::EXPORTED;
        self.flags[i] = f;
        if f & flags::CHARACTER != 0 {
            // A character's subtree is not walked: its nodes keep their
            // flags and are not exported.
            self.characters.push(i);
            return Ok(());
        }
        if f & flags::PROP != 0 {
            self.props.push(i);
        } else if f & flags::OBJECT_MASK == flags::OBJECT {
            self.objects.push(i);
        } else {
            if f & flags::CLASS == class::MESH {
                let NodeData::Mesh(mesh) = &self.scene.nodes[i].body else {
                    return Err(Error::Malformed(format!("mesh node {i} has no mesh")));
                };
                // The original keeps a second part list for this pass and
                // offers its parts last-first; Exarust has one part list.
                if !options.skip_part_prepare && !mesh.parts.is_empty() {
                    for part in (0..mesh.parts.len()).rev() {
                        hooks.prepare_part(i, part);
                    }
                }
                for part in &mesh.parts {
                    if let Some(m) = &part.material {
                        self.add_material(m);
                    }
                }
            }
            self.nodes.push(i);
            self.index[i] = self.nodes.len() as i32;
        }
        for c in self.children[i].clone() {
            self.visit(c, options, hooks)?;
        }
        Ok(())
    }

    fn add_material(&mut self, name: &[u8; 16]) {
        let name = lowercase_name(name);
        if self.materials.contains(&name)
            || self
                .scene
                .shared_materials
                .iter()
                .any(|s| lowercase_name(s) == name)
        {
            return;
        }
        self.materials.push(name);
    }

    /// Numbers props and placed objects after the scene nodes (the first
    /// prop takes the last scene node's number), then finds each scene
    /// node's exported parent and data source.
    fn number(&mut self) -> Result<Vec<Entry>> {
        let mut next = self.nodes.len() as i32 - 1;
        for &i in self.props.iter().chain(&self.objects) {
            next += 1;
            self.index[i] = next;
        }
        let mut out = Vec::with_capacity(self.nodes.len());
        for &i in &self.nodes {
            let node = self.node(i)?;
            let parent = node.parent.filter(|&p| self.exported(p));
            let source = if self.flags[i] & flags::PROP == 0 {
                let master = node.shares.unwrap_or(i);
                self.node(master)?;
                let mut best = (i, self.index[i]);
                if self.exported(master) && self.index[master] < best.1 {
                    best = (master, self.index[master]);
                }
                for (s, n) in self.scene.nodes.iter().enumerate() {
                    if n.shares == Some(master) && self.exported(s) && self.index[s] < best.1 {
                        best = (s, self.index[s]);
                    }
                }
                (best.0 != i).then_some(best.0)
            } else {
                None
            };
            out.push(Entry {
                node: i,
                parent,
                source,
            });
        }
        Ok(out)
    }

    /// A prop's, placed object's or character's parent number: 0 unless the
    /// parent is exported.
    fn parent_number(&self, node: &SceneNode) -> u32 {
        node.parent
            .filter(|&p| self.exported(p))
            .map_or(0, |p| self.index[p] as u32)
    }

    fn write_node(
        &self,
        w: &mut ChunkWriter,
        e: &Entry,
        options: &ExportOptions,
        hooks: &mut dyn ExportHooks,
    ) -> Result<()> {
        let node = self.node(e.node)?;
        let f = self.flags[e.node];
        let kind = if f & flags::PROP != 0 {
            tag::NODE_PLAIN
        } else {
            match f & flags::CLASS {
                class::TARGET => tag::NODE_TARGET,
                class::LIGHT => tag::NODE_LIGHT,
                class::GROUP => tag::NODE_GROUP,
                class::VOXEL_LIGHT => tag::NODE_VOXEL_LIGHT,
                class::MESH => tag::NODE_MESH,
                _ => tag::NODE_PLAIN,
            }
        };
        // The original lets the attachment bring itself up to date before
        // its node's chunk is opened, whatever the node's data source.
        if f & flags::ATTACHED != 0 && node.attachment.is_some() {
            hooks.prepare_attachment(e.node);
        }
        w.open(kind);
        w.bytes(&node.name);
        w.u32(f & flags::HEADER_MASK);
        w.u32(node.flags2 & flags::HEADER_MASK2);
        w.u32(e.source.map_or(0, |s| self.index[s] as u32));
        if node.parent.is_none() || node.parent == e.parent {
            w.u32(node.parent.map_or(0, |p| self.index[p] as u32));
            w.matrix(&node.local);
        } else {
            w.u32(0);
            w.matrix(&node.world);
        }
        w.vec3(node.half_extents);
        w.vec3(node.centre);
        w.f32(node.radius);
        w.u32(node.tail);
        if e.source.is_none() {
            self.write_body(w, node, kind, options)?;
            if f & flags::ATTACHED != 0
                && let Some(a) = &node.attachment
            {
                write_attachment(w, a)?;
            }
            if f & flags::CLASS == class::MESH
                && let NodeData::Mesh(mesh) = &node.body
            {
                if let Some((t, payload)) = &mesh.kept_chunk {
                    w.u32(*t);
                    w.u32(payload.len() as u32);
                    w.bytes(payload);
                }
                if let Some(gi) = mesh.gi_points.as_ref().filter(|g| !g.points.is_empty()) {
                    w.open(tag::GI_POINTS);
                    w.u32(gi.spacing);
                    w.u32(gi.points.len() as u32);
                    for p in &gi.points {
                        w.vec3(*p);
                    }
                    w.close()?;
                }
            }
            for t in node.tracks.iter().filter(|t| t.on_translation) {
                write_track(w, t)?;
            }
        }
        w.close()
    }

    fn write_body(
        &self,
        w: &mut ChunkWriter,
        node: &SceneNode,
        kind: u32,
        options: &ExportOptions,
    ) -> Result<()> {
        let wrong = || Error::Malformed(format!("node kind {kind:#x} without its body"));
        match kind {
            tag::NODE_MESH => match &node.body {
                NodeData::Mesh(m) => write_mesh(w, m, options),
                _ => Err(wrong()),
            },
            tag::NODE_LIGHT => match &node.body {
                NodeData::Light(l) => {
                    write_light(w, l, node.flags2);
                    Ok(())
                }
                _ => Err(wrong()),
            },
            tag::NODE_TARGET => match &node.body {
                NodeData::Target { node: t, value } => {
                    self.node(*t)?;
                    w.u32(self.index[*t] as u32);
                    w.u32(*value);
                    Ok(())
                }
                _ => Err(wrong()),
            },
            tag::NODE_VOXEL_LIGHT => match &node.body {
                NodeData::VoxelLight(l) => {
                    w.u32(0);
                    w.vec3(l.color);
                    w.f32(1.0);
                    w.f32(l.intensity);
                    w.f32(l.radius);
                    w.u32(0);
                    Ok(())
                }
                _ => Err(wrong()),
            },
            _ => Ok(()),
        }
    }

    fn write_props(&self, w: &mut ChunkWriter) -> Result<()> {
        w.open(tag::PROPS);
        w.u32(self.props.len() as u32);
        for &i in &self.props {
            let p = self.node(i)?;
            w.open(tag::PROP);
            w.u32(self.flags[i] & flags::PLACEMENT_MASK);
            w.bytes(&p.name);
            w.matrix(&p.local);
            w.u32(self.parent_number(p));
            w.u32(p.tail);
            if !self.children[i].is_empty() {
                self.write_parts(w, i)?;
            }
            w.close()?;
        }
        w.close()
    }

    /// Part records of the owned children of a prop or placed object. A
    /// child whose attachment is neither a body nor cloth ends the records
    /// of its siblings.
    fn write_parts(&self, w: &mut ChunkWriter, i: usize) -> Result<()> {
        for &c in &self.children[i] {
            let child = self.node(c)?;
            let Some(a) = child
                .attachment
                .as_ref()
                .filter(|_| self.flags[c] & flags::OWNED != 0)
            else {
                continue;
            };
            if a.class & ATTACH_BODY == ATTACH_BODY {
                w.open(tag::PART_BODY);
                w.bytes(&child.name);
                w.u32(a.id);
                w.matrix(&child.world);
            } else if a.class & ATTACH_CLOTH == ATTACH_CLOTH {
                let AttachmentData::Cloth { particles, .. } = &a.data else {
                    return Err(Error::Malformed(format!(
                        "cloth part {c} without particles"
                    )));
                };
                w.open(tag::PART_CLOTH);
                w.bytes(&child.name);
                w.u32(a.id);
                w.u32(particles.len() as u32);
                for p in particles {
                    w.vec3(*p);
                }
            } else {
                return Ok(());
            }
            if !self.children[c].is_empty() {
                self.write_parts(w, c)?;
            }
            w.close()?;
        }
        Ok(())
    }

    fn write_local_database(&self, w: &mut ChunkWriter) -> Result<()> {
        w.open(tag::LOCAL_DATABASE);
        if let Some(db) = self.scene.object_db.as_ref().filter(|d| d.count > 0) {
            w.open(tag::LOCAL_OBJECTS);
            write_rdb(w, OBJECT_DB_MAGIC, db)?;
            w.close()?;
            w.open(tag::LOCAL_STRINGS);
            w.bytes(&self.scene.object_strings);
            w.close()?;
        }
        w.open(tag::PLACED_OBJECTS);
        w.u32(self.objects.len() as u32);
        for &i in &self.objects {
            let o = self.node(i)?;
            let ids = o
                .object
                .ok_or_else(|| Error::Malformed(format!("placed object {i} without ids")))?;
            w.open(tag::PLACED_OBJECT);
            w.u32(ids.id);
            w.u32(ids.placement);
            w.u32(self.flags[i] & flags::PLACEMENT_MASK);
            w.matrix(&o.local);
            w.u32(self.parent_number(o));
            w.u32(o.tail);
            if !self.children[i].is_empty() {
                self.write_parts(w, i)?;
            }
            w.close()?;
        }
        w.close()?;
        w.close()
    }

    fn write_characters(
        &self,
        w: &mut ChunkWriter,
        options: &ExportOptions,
        hooks: &mut dyn ExportHooks,
    ) -> Result<()> {
        // Every character written with its body refreshes its record first;
        // a class-1 node is written without a body and is not offered.
        for &i in &self.characters {
            if self.flags[i] & flags::CLASS != class::PLAIN {
                hooks.prepare_character(i);
            }
        }
        if let Some(db) = self.scene.character_db.as_ref().filter(|d| d.count > 0) {
            w.open(tag::CHARACTER_DB);
            write_rdb(w, CHARACTER_DB_MAGIC, db)?;
            w.close()?;
        }
        for &i in &self.characters {
            let node = self.node(i)?;
            let c = node
                .character
                .as_ref()
                .ok_or_else(|| Error::Malformed(format!("character node {i} without a state")))?;
            w.open(tag::PLACEMENT);
            w.u32(c.record);
            if self.flags[i] & flags::CLASS == class::PLAIN {
                w.vec3(node.local[3]);
                w.f32(c.heading);
                w.i32(c.condition);
                w.bytes(&c.tail);
                w.close()?;
                continue;
            }
            write_position(w, c);
            if c.condition == DEAD {
                write_pose_tree(w, c)?;
            } else if !options.save_mode {
                if c.state_flags & STATE_POSED != 0 {
                    write_pose_tree(w, c)?;
                }
            } else {
                let ai = ai_of(c)?;
                write_home(w, c, &ai)?;
                if ai.mode == 1 {
                    write_pose_tree(w, c)?;
                    write_motion(w, c)?;
                } else if c.state_flags & STATE_POSED != 0 {
                    write_pose_tree(w, c)?;
                }
            }
            w.close()?;
        }
        Ok(())
    }
}

fn ai_of(c: &ExportCharacter) -> Result<ExportAi> {
    c.ai.ok_or_else(|| Error::Malformed("a saved character without its AI".into()))
}

/// Position (body x and z, floor y), heading and condition.
fn write_position(w: &mut ChunkWriter, c: &ExportCharacter) {
    w.vec3([c.body_position[0], c.floor_height, c.body_position[2]]);
    w.f32(c.heading);
    w.i32(c.condition);
}

fn write_home(w: &mut ChunkWriter, c: &ExportCharacter, ai: &ExportAi) -> Result<()> {
    if ai.motile && c.record_kind & RECORD_KIND_MASK == RECORD_KIND_HOME && !ai.skip_home {
        w.open(tag::AI_HOME);
        w.vec3(ai.home);
        w.f32(ai.home_heading);
        w.u32(0);
        w.close()?;
    }
    Ok(())
}

fn write_motion(w: &mut ChunkWriter, c: &ExportCharacter) -> Result<()> {
    w.open(tag::MOTION);
    w.u32(c.motion_state);
    w.f32(c.motion_value);
    w.u32(0);
    w.close()
}

fn write_pose_tree(w: &mut ChunkWriter, c: &ExportCharacter) -> Result<()> {
    let pose = c
        .pose
        .as_ref()
        .ok_or_else(|| Error::Malformed("a posed character without a pose".into()))?;
    w.open(tag::POSE_TREE);
    write_pose_node(w, pose)?;
    w.close()
}

fn write_pose_node(w: &mut ChunkWriter, p: &PoseNode) -> Result<()> {
    w.matrix(&p.matrix);
    w.u32(p.word);
    let count_at = w.position();
    w.u32(0);
    let mut count = 0u32;
    if p.kind != POSE_LEAF_KIND {
        for c in p
            .children
            .iter()
            .filter(|c| c.class & ATTACH_KIND == POSE_CHILD_CLASS)
        {
            write_pose_node(w, c)?;
            count += 1;
        }
    }
    w.patch_u32(count_at, count)
}

fn write_pose_flat(w: &mut ChunkWriter, c: &ExportCharacter) -> Result<()> {
    w.open(tag::POSE_FLAT);
    for m in &c.parts {
        w.matrix(m);
    }
    for h in &c.held {
        w.matrix(h.as_ref().unwrap_or(&IDENTITY));
    }
    w.close()
}

/// One character's placement as the save game writes it outside a level
/// export: dead bodies store their parts in order, and a save also stores
/// the AI home, the motion state and what the character's attached
/// objects write (a chunk with their count, taken back out when none
/// wrote).
pub fn write_character_state(
    w: &mut ChunkWriter,
    c: &ExportCharacter,
    save_mode: bool,
) -> Result<()> {
    w.open(tag::PLACEMENT);
    w.u32(c.record);
    write_position(w, c);
    if c.condition == DEAD {
        write_pose_flat(w, c)?;
    } else if save_mode {
        let ai = ai_of(c)?;
        write_home(w, c, &ai)?;
        if ai.mode == 1 {
            write_pose_flat(w, c)?;
            write_motion(w, c)?;
        }
        if !c.attachments.is_empty() {
            w.open(tag::ATTACHED);
            let count_at = w.position();
            w.u32(0);
            let mut count = 0u32;
            for b in c.attachments.iter().flatten() {
                w.bytes(b);
                count += 1;
            }
            if count < 1 {
                w.discard()?;
            } else {
                w.close()?;
                w.patch_u32(count_at, count)?;
            }
        }
    }
    w.close()
}

fn write_rdb(w: &mut ChunkWriter, magic: u32, db: &RdbImage) -> Result<()> {
    let table_len = (db.count as usize + 1) * 16;
    w.u32(magic);
    w.u32(table_len as u32);
    if db.count > 0 {
        let table = db
            .table
            .get(..table_len)
            .ok_or_else(|| short("database table", table_len, db.table.len()))?;
        w.bytes(table);
        w.bytes(&db.data);
    }
    Ok(())
}

fn write_grid(w: &mut ChunkWriter, g: &ExportGrid) -> Result<()> {
    let cells = (g.width as usize)
        .checked_mul(g.height as usize)
        .ok_or_else(|| Error::Malformed("grid size overflows".into()))?;
    let planes = [&g.plane_a, &g.plane_b, &g.plane_c];
    if let Some(p) = planes.iter().find(|p| p.len() < cells) {
        return Err(short("grid plane", cells, p.len()));
    }
    w.open(tag::GRID);
    for sets in [&g.wall_sets, &g.floor_sets] {
        w.u32(sets.len() as u32);
        for s in sets {
            w.bytes(s);
        }
    }
    w.u32(g.width);
    w.u32(g.height);
    for v in GRID_WORDS {
        w.u32(v);
    }
    for p in planes {
        for &v in &p[..cells] {
            w.u32(v);
        }
    }
    w.close()
}

fn element_size(format: u32) -> usize {
    match format & 0xFF_0000 {
        0xA4_0000 => 32,
        0xB4_0000 => 4,
        0xF2_0000 => 8,
        0xF3_0000 => 12,
        _ => 0,
    }
}

fn write_indices(w: &mut ChunkWriter, what: &str, v: &[u32], n: usize, narrow: bool) -> Result<()> {
    let v = v.get(..n).ok_or_else(|| short(what, n, v.len()))?;
    for &i in v {
        if narrow {
            w.u16(i as u16);
        } else {
            w.u32(i);
        }
    }
    Ok(())
}

fn write_mesh(w: &mut ChunkWriter, m: &ExportMesh, options: &ExportOptions) -> Result<()> {
    w.u32(m.render_word & 0xFFFF_FF00);
    let count_at = w.position();
    w.u32(last_index(m.streams.len()));
    let mut written = 0usize;
    for s in m.streams.iter().filter(|s| s.format & STREAM_UNSAVED == 0) {
        written += 1;
        w.u32(s.id);
        w.u32(s.format);
        w.u32(s.count.wrapping_sub(1));
        let n = (s.count as usize)
            .checked_mul(element_size(s.format))
            .ok_or_else(|| Error::Malformed("stream size overflows".into()))?;
        w.bytes(
            s.data
                .get(..n)
                .ok_or_else(|| short("vertex stream", n, s.data.len()))?,
        );
    }
    w.patch_u32(count_at, last_index(written))?;

    let vertices = m.vertex_count as i32;
    let narrow = vertices < WIDE_INDICES;
    w.open(tag::TOPOLOGY);
    w.u32(0);
    w.i32(vertices.wrapping_sub(1));
    w.u32(last_index(m.parts.len()));
    for p in &m.parts {
        w.u32(0);
        w.u32(p.vertices[0]);
        w.u32(p.vertices[1]);
        w.bytes(p.material.as_ref().unwrap_or(&DEFAULT_MATERIAL));
    }
    w.u32(last_index(m.maps.len()));
    for map in &m.maps {
        w.u32(map.key);
        if !narrow || vertices > 0 {
            write_indices(w, "index map", &map.indices, vertices as usize, narrow)?;
        }
    }
    if !m.triangles.is_empty() {
        if !m.triangles.len().is_multiple_of(3) {
            return Err(Error::Malformed(format!(
                "{} triangle corners",
                m.triangles.len()
            )));
        }
        let tris = m.triangles.len() / 3;
        w.open(tag::TRIANGLES);
        w.u32(0);
        w.u32(last_index(tris));
        write_indices(w, "triangles", &m.triangles, tris * 3, narrow)?;
        for p in &m.parts {
            w.u32(p.triangles[0]);
            w.u32(p.triangles[1]);
        }
        if !options.skip_triangle_data {
            if let Some(b) = &m.triangle_bytes {
                w.open(tag::TRIANGLE_BYTES);
                w.bytes(
                    b.get(..tris)
                        .ok_or_else(|| short("triangle bytes", tris, b.len()))?,
                );
                w.close()?;
            }
            if let Some(f) = &m.triangle_flags {
                w.open(tag::TRIANGLE_FLAGS);
                for &v in f
                    .get(..tris)
                    .ok_or_else(|| short("triangle flags", tris, f.len()))?
                {
                    w.u32(v);
                }
                w.close()?;
            }
        }
        w.close()?;
    }
    w.close()
}

fn write_light(w: &mut ChunkWriter, l: &ExportLight, flags2: u32) {
    w.u32(l.flags);
    w.vec3(l.color);
    w.f32(1.0);
    w.f32(l.multiplier);
    w.f32(l.intensity);
    w.f32(l.radius);
    w.f32(l.unread);
    w.f32(l.softness);
    if flags2 & SPOT_MASK == SPOT {
        w.f32(l.spot[0]);
        w.f32(l.spot[1]);
    }
    if l.flags & LIGHT_EXTRA != 0 {
        w.u32(0);
        w.f32(l.animation.amplitude);
        w.f32(l.animation.speed);
        w.f32(l.animation.emissive);
    }
}

fn write_attachment(w: &mut ChunkWriter, a: &Attachment) -> Result<()> {
    let wrong = || Error::Malformed(format!("attachment class {:#x} without its data", a.class));
    match a.class & ATTACH_KIND {
        ATTACH_BODY => {
            let AttachmentData::Body(b) = &a.data else {
                return Err(wrong());
            };
            w.open(tag::RIGID_BODY);
            write_body(w, a.class, b)?;
            w.close()
        }
        ATTACH_CLOTH => {
            let AttachmentData::Cloth { marker, .. } = &a.data else {
                return Err(wrong());
            };
            w.open(tag::CLOTH);
            w.u32(*marker);
            w.close()
        }
        ATTACH_EMITTER => {
            let AttachmentData::Emitter { kind, size } = &a.data else {
                return Err(wrong());
            };
            if *kind != tag::EMITTER_E004 && *kind != tag::FIRE {
                // The original closes a chunk it never opened here.
                return Err(Error::Unsupported(format!("emitter kind {kind:#x}")));
            }
            w.open(*kind);
            w.f32(*size);
            w.close()
        }
        _ => Ok(()),
    }
}

/// A rigid body's payload: the header, then the spheres, the saved links
/// and the sound slots, each as a tag and size around its records.
fn write_body(w: &mut ChunkWriter, class: u32, b: &ExportBody) -> Result<()> {
    w.u32(class);
    w.f32(b.mobility);
    for p in &b.rest {
        w.vec3(*p);
    }
    for m in b.point_masses {
        w.f32(m);
    }
    w.f32(b.friction);
    w.f32(1.0);
    w.u32(0);
    w.u32(b.group_mask);
    w.u32(b.last);
    if !b.spheres.is_empty() {
        w.open(tag::SPHERES);
        w.u32(0);
        w.u32(b.spheres.len() as u32);
        for s in &b.spheres {
            for v in s {
                w.f32(*v);
            }
        }
        w.close()?;
    }
    if !b.links.is_empty() {
        w.open(tag::LINKS);
        let count_at = w.position();
        w.u32(b.links.len() as u32);
        let mut count = 0u32;
        for l in &b.links {
            if l.flags & LINK_UNSAVED != 0 {
                break;
            }
            count += 1;
            w.u32(l.flags);
            if !b.weighted_links {
                w.vec3([l.on_body[0], l.on_body[1], l.on_body[2]]);
                w.vec3([l.on_parent[0], l.on_parent[1], l.on_parent[2]]);
            } else {
                w.vec3(weighted_point(l.on_body, &b.rest));
                let parent = if l.flags & LINK_PARENT_WEIGHTS == 0 {
                    [l.on_parent[0], l.on_parent[1], l.on_parent[2]]
                } else {
                    let rest = l.parent_rest.as_ref().ok_or_else(|| {
                        Error::Malformed("a link on parent weights without the parent body".into())
                    })?;
                    weighted_point(l.on_parent, rest)
                };
                w.vec3(sub(parent, b.centre));
            }
            w.f32(l.slack);
            w.f32(l.strength);
            w.f32(l.smoothing);
            if l.flags & LINK_SLIDER != 0 {
                w.vec3(l.axis);
            }
        }
        w.close()?;
        w.patch_u32(count_at, count)?;
    }
    if !b.sounds.is_empty() {
        w.open(tag::SOUNDS);
        w.u32(b.sounds.len() as u32);
        for s in &b.sounds {
            w.u32(s.id);
            w.vec3(s.point);
            w.bytes(&s.sound);
            w.f32(s.volume);
            w.f32(s.pitch);
        }
        w.close()?;
    }
    Ok(())
}

/// A track without keys stores −1 as its last key and no keys, as the
/// original does.
fn write_track(w: &mut ChunkWriter, t: &ExportTrack) -> Result<()> {
    w.open(tag::KEY_TRACK);
    w.u32(tag::KEY_TRACK_WORD);
    w.u32(t.mode);
    w.f32(t.duration);
    w.u32(last_index(t.keys.len()));
    for (time, p) in &t.keys {
        w.f32(*time);
        w.vec3(*p);
    }
    w.close()
}

#[cfg(test)]
#[path = "rfc_write_test.rs"]
mod tests;
