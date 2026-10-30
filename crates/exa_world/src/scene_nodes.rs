//! The original's scene-node unit: node kinds and registration, frames and
//! bounds, the 6 m cell a node sits in, per-frame sector classification
//! through portals, node visibility and fades.
//!
//! A scene owns **sectors**. The scene is a sector itself (its fields sit
//! where a sector's do) and comes first in its own sector list
//! ([`SCENE_SECTOR`]); a node without an owner is registered in it. In a
//! tile-grid level the other sectors are the rooms of the room flood fill.
//! A node outside every scene hangs from the unit's global `_World` sector,
//! which refuses registration ([`LOCKED`]); here that is an owner of
//! `None`. A sector keeps one list per node kind (meshes, lights, auxiliary
//! nodes; geometry goes to the scene) and its doorways. Every frame the
//! room manager gives each room a state (hidden, active, adjacent) and this
//! module turns it into a per-sector view list and per-mesh visibility:
//!
//! - the scene's own sector carries every view bit and the main views;
//! - an **active** sector keeps the main views that meet its box, moved
//!   into the sector's space;
//! - an **adjacent** sector counts its doorways to active sectors, and each
//!   of those whose plane faces the eye is clipped by the first main view
//!   into a frustum from the eye; the frusta's screen bounds are its
//!   scissor;
//! - a sector with no view bit and no doorway to an active sector runs the
//!   hidden pass (masks cleared, moved meshes' cells kept current);
//! - a mesh is visible when it meets one of its sector's views. A mesh in
//!   one of a doorway's two cells whose box reaches 20 cm in front of the
//!   doorway plane is tested against the sector beyond instead, and so
//!   escapes the scissor.
//!
//! Meshes also carry the original's cut-away fade: an amount that moves
//! towards its target at 0.003 per millisecond, turning a solid draw into a
//! dithered or partial one. [`Scene::detect_cut_away`] is the camera's
//! detector that marks the meshes to fade: those the segment from beyond
//! the eye to a metre short of the player meets, and those over the
//! player.
//!
//! Frames are kept as the original keeps them: a node directly under its
//! sector has its local frame as its frame, so every node frame is in its
//! sector's space and a sector's `origin` only moves views into that space
//! and places the sector's box in the world. The transform update itself
//! (parent times local, the "moved" marker on the whole subtree) belongs to
//! the original's node class, outside this unit; the model refreshes a
//! stale frame before it uses it.
//!
//! All lengths are metres (the original's centimetres divided by 100) and
//! angles are radians; the cell and doorway tests multiply by 100 as the
//! original works in centimetres. The model is pure data: [`crate::rooms`]
//! supplies the room states and [`crate::light_flood`] shares
//! [`cell_spill`]. Exarust still draws through `room_view`; nothing here is
//! on the draw path yet.

// The original's branches on partially ordered comparisons are deliberate:
// a NaN takes the processor's side, so the negated form spells it out.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::{Mat3, Vec2, Vec3};
use exa_physics::intersect::{
    ConvexVolume, Obb, point_obb_distance_squared, segment_sphere, spheres_separated,
};

use crate::light_flood::CellLights;
use crate::rooms::RoomState;

/// Scene word: the node is registered in a live scene (lights, meshes and
/// auxiliary nodes when they are registered, sectors when a batch appends
/// them).
pub const IN_SCENE: u32 = 0x0004_0000;
/// Scene word: owned elsewhere; the unit's cleanup hook leaves the node and
/// its subtree alone.
pub const OWNED: u32 = 0x0200_0000;
/// Scene word: the camera's cut-away detector tests this mesh.
pub const CUT_AWAY_CANDIDATE: u32 = 0x2000_0000;
/// Scene word: the cut-away kind, three bits that include
/// [`CUT_AWAY_CANDIDATE`].
pub const CUT_AWAY_KIND: u32 = 0x7000_0000;
/// Cut-away kind: the camera's segment must meet the bounding sphere and
/// then the mesh's own segment callback.
pub const CUT_AWAY_TESTED: u32 = 0x2000_0000;
/// Cut-away kind: meeting the bounding sphere is enough.
pub const CUT_AWAY_SPHERE: u32 = 0x3000_0000;
/// Cut-away kind: a mesh over the player, met by a 10 m segment straight
/// down onto the looked-at point and then by the mesh's ray callback.
pub const CUT_AWAY_OVERHEAD: u32 = 0x6000_0000;
/// Scene word: a marked or cleared mesh marks or clears its parent too.
pub const CUT_AWAY_PARENT: u32 = 0x0100_0000;
/// Scene word: a marked or cleared mesh marks or clears its mesh children
/// too (the same bit as [`OWNED`]; the detector reads it for this).
pub const CUT_AWAY_CHILDREN: u32 = 0x0200_0000;
/// How far above the looked-at point the overhead probe starts (m).
pub const CUT_AWAY_OVERHEAD_HEIGHT: f32 = 10.0;

/// Option word: the node has been prepared (its one-time set-up ran when
/// it was first registered).
pub const PREPARED: u32 = 0x1;
/// Option word: a light that belongs to the world rather than to a sector;
/// it joins no sector light list.
pub const WORLD_LIGHT: u32 = 0x2;
/// Option word: a mesh that bounds its sector's first box; the others only
/// widen the second.
pub const BOUNDS_SPLIT: u32 = 0x1_0000;

/// Sector option word: registration into the sector is skipped. The unit's
/// global `_World` sector carries it.
pub const LOCKED: u32 = 0x10_0000;

/// Sector change word: a light link left the sector (and every move out).
pub const LINKS_CHANGED: u32 = 0x10_0000;
/// Sector change word: a node moved out of the sector.
pub const MOVED_OUT: u32 = 0x20_0000;
/// Sector change word: a light was registered.
pub const LIGHT_ADDED: u32 = 0x40_0000;
/// Sector change word: a mesh was registered (its draw buckets are pending).
pub const MESH_ADDED: u32 = 0x80_0000;
/// The change bits that make the frame update rebuild the membership.
pub const MEMBERSHIP_CHANGED: u32 = LINKS_CHANGED | MOVED_OUT | LIGHT_ADDED;

/// Mesh visibility: drawn (in a view).
pub const MAIN: u32 = 0x1;
/// Mesh visibility: seen by at least one view (shadow faces may be needed).
pub const SEEN: u32 = 0x0100_0000;
/// Mesh visibility: drawn through a portal scissor.
pub const SCISSOR: u32 = 0x1000_0000;
/// Mesh visibility: drawn dithered while fading.
pub const DITHERED: u32 = 0x2000_0000;
/// Mesh visibility: drawn partially once the fade settles below 0.999.
pub const PARTIAL: u32 = 0x4000_0000;
/// Mesh visibility: the one bit the hidden-sector pass keeps (its meaning
/// is not established).
pub const HIDDEN_KEEPS: u32 = 0x8000_0000;

/// Frame word: the mesh follows a fading parent; it keeps only the seen
/// bit, and its fade still moves.
pub const INHERIT_FADE: u32 = 0x10;
/// Frame word: the mesh follows a faded-out parent; it keeps only the seen
/// bit and skips the rest of the pass. With [`INHERIT_FADE`] it loses the
/// seen bit too.
pub const INHERIT_HIDDEN: u32 = 0x20;

/// Light word ([`SceneNode::light_flags`]): the light's activation keeps
/// no position snapshot.
pub const LIGHT_NO_SNAPSHOT: u32 = 0x10;
/// Option word: the node's per-alias buckets are built before it is drawn.
pub const NEEDS_BUCKETS: u32 = 0x3_0000;
/// Light word ([`SceneNode::light_flags`]): the light flickers.
pub const LIGHT_FLICKER: u32 = 0x1_0000;
/// Option word ([`SceneNode::flags`]): the alias gate's bit. A light that
/// carries it only spreads an alias to a sector its sphere reaches. The
/// name is a guess: the bit sits next to [`NEEDS_BUCKETS`] in the option
/// word, the alias refresh copies it with the word's third byte, and the
/// model reads it as the light carrying a shadow cube map.
pub const LIGHT_SHADOW_CUBE: u32 = 0x4_0000;
/// Option word bits that pick how a light's activation snapshot is taken;
/// [`SNAPSHOT_FLAT`] in them drops the snapshot's height.
pub const SNAPSHOT_MODE: u32 = 0x300;
/// The [`SNAPSHOT_MODE`] value whose snapshot keeps a zero height.
pub const SNAPSHOT_FLAT: u32 = 0x200;

/// The class code (over 12 bits) the original's batch insertion tests for a
/// sector.
pub const CLASS_SECTOR: u32 = 0x11;

/// The far sort key a reset node takes: the original's `-100000.0`.
pub const SORT_KEY_NONE: f32 = -100_000.0;

/// Scene word: the node is queued for deletion (its subtree is marked with
/// it when the queue entry detaches instead of hiding).
pub const DELETING: u32 = 0x8000_0000;
/// Sector change word a queued removal sets: the node left and its light
/// links changed.
pub const REMOVAL_CHANGES: u32 = MOVED_OUT | LINKS_CHANGED;
/// How many removal entries the scene queue holds; the original's ring is
/// 256 entries and refuses the 256th.
pub const REMOVAL_QUEUE: usize = 256;

/// Payload flag bits that make a despawn take the node under the sector
/// instead of the node itself (both bits must be set; guess: they mark the
/// pieces of a larger node, the original does not name them).
pub const PAYLOAD_CLIMBS: u32 = 0x1200_0000;
/// The payload flags' class byte.
pub const PAYLOAD_CLASS_MASK: u32 = 0xff00_0000;
/// The class byte of a character's payload, which owns character data.
pub const PAYLOAD_CHARACTER: u32 = 0x1a00_0000;
/// The object-database word's top bits: set when the node holds a live
/// database slot.
pub const OBJECT_SLOT_LIVE: u32 = 0xc000_0000;

/// What a presented frame of the scene asks the rest of the engine to do,
/// in the original's call order. The model itself keeps the visibility,
/// fade and membership state; these lists are the render-side work.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameEvents {
    /// A visible pass saw a change: an alias moved, a seen mesh moved, or
    /// a seen mesh's part moved (the original's visible-pass answer).
    pub moved_seen: bool,
    /// Lights activated this frame in grid mode: their cells are flooded
    /// (an omni light) or filled directionally.
    pub activated: Vec<usize>,
    /// Lights activated outside grid mode: their alias's links are rebuilt
    /// instead of a cell pass.
    pub alias_links: Vec<usize>,
    /// Meshes the node passes handled as moved: the scene's moved-node
    /// callback runs for each.
    pub moved: Vec<usize>,
    /// Flicker steps: (light, whether it already ran).
    pub flicker: Vec<(usize, bool)>,
    /// Nodes dropped from a sector because they no longer belong to it and
    /// were not in the scene.
    pub dropped: Vec<usize>,
    /// Nodes whose light links were refreshed.
    pub relinked: Vec<usize>,
    /// A mesh part's own transform changed in a seen mesh.
    pub parts_moved: bool,
    /// Lights whose alias copies were refreshed after activation.
    pub alias_updates: Vec<usize>,
    /// Nodes taken out of the scene by the removal queue.
    pub removed: Vec<usize>,
    /// Deleted nodes whose owner was `_World`: the original logs "World
    /// sector object deleted" with each one's name.
    pub world_deleted: Vec<usize>,
    /// Nodes whose release is deferred to their owner (the original's free
    /// list).
    pub freed: Vec<usize>,
    /// Materials the renderers release at the frame end.
    pub materials_removed: Vec<usize>,
}

/// Clears the world-light flags of a mesh node's parts, as
/// [`Scene::clear_world_light`] does.
fn clear_parts(node: &mut SceneNode) {
    node.part_word = 0;
    for part in &mut node.parts {
        part.flags |= 0x10;
        part.value = 0;
    }
}

/// What a queued removal does at the frame end (the original's entry state).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemovalKind {
    /// The node leaves the scene for one frame: it hangs from `_World`
    /// while queued and gets its owner back at the frame end.
    Hide,
    /// The subtree is deleted: it waits while it carries [`OWNED`], then
    /// gets its owner back and is taken out, or deferred to the free list
    /// while another owner holds it.
    Delete,
    /// The node is freed without the ownership wait (the node destructor's
    /// own entry), or deferred to the free list while another owner holds
    /// it.
    Free,
}

/// A node waiting in the scene's removal queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Removal {
    pub node: usize,
    pub kind: RemovalKind,
    /// The sector the node was in when it was queued (its owner, which a
    /// hidden or deleted node gets back at the frame end).
    pub sector: Option<usize>,
}

/// The scene's removal queue: the original's 256-entry ring that callers
/// write into, drained at the start of every frame into the pending list
/// the frame end works through.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemovalQueue {
    /// The ring, oldest first.
    entries: Vec<Removal>,
    /// The drained entries the frame end has not finished with.
    pending: Vec<Removal>,
}

impl RemovalQueue {
    /// Whether the ring refuses another entry (255 entries is full: the
    /// original's write index must not meet its read index).
    pub fn is_full(&self) -> bool {
        self.entries.len() + 1 >= REMOVAL_QUEUE
    }

    /// Queues a removal in the ring; false when it was full.
    pub fn push(&mut self, removal: Removal) -> bool {
        if self.is_full() {
            return false;
        }
        self.entries.push(removal);
        true
    }

    /// Entries in the ring and the pending list together.
    pub fn len(&self) -> usize {
        self.entries.len() + self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.pending.is_empty()
    }

    /// The pending entries, then the ring's.
    pub fn iter(&self) -> impl Iterator<Item = &Removal> + '_ {
        self.pending.iter().chain(self.entries.iter())
    }

    /// The drained entries the frame end still holds, in their order.
    pub fn pending(&self) -> &[Removal] {
        &self.pending
    }
}

/// One timed node transition: a controller evaluates against the time since
/// the entry was added.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub node: usize,
    pub controller: u32,
    /// The game clock when the entry was added (ms).
    pub start: u32,
}

/// What releasing a scene node found, for the caller to free.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Released {
    /// The node whose removal was queued: the node under the sector when
    /// the payload flags asked for it.
    pub node: usize,
    /// The character data a character payload held.
    pub character: Option<u64>,
    /// The object-database word, when it held a live slot.
    pub object_slot: Option<u32>,
    /// Whether the queue accepted the entry.
    pub queued: bool,
}

/// The detail scale a new scene starts with (the scale
/// [`Scene::set_detail`] later replaces).
pub const DEFAULT_DETAIL_SCALE: f32 = 0.05;

/// The scene's own sector, first in its sector list.
pub const SCENE_SECTOR: usize = 0;

/// Cell size, centimetres.
pub const CELL_CM: f32 = 600.0;
/// `1/600` as an `f32`, the multiplier the original uses to find a cell.
pub const INV_CELL_CM: f32 = 0.001_666_666_7;
/// The marker a computed cell word starts with.
pub const CELL_MARK: u16 = 0x10;
/// The cell side of a node spills towards low coordinates.
pub const SPILL_LOW: u16 = 0x1;
/// The cell side of a node spills towards high coordinates.
pub const SPILL_HIGH: u16 = 0x4;
/// The fade speed per millisecond (0.003 × dt is a full fade in about
/// 333 ms).
pub const FADE_RATE: f32 = 0.003;
/// A fade back in that reaches this (compared in double precision) ends.
pub const FADE_DONE: f64 = 0.001;
/// A settled fade below this (double precision) draws partially.
pub const FADE_PARTIAL: f64 = 0.999;
/// A fade out snaps to its target once it is within this of it.
pub const FADE_EPSILON: f32 = 1.192_089_982_282_595_9e-7;
/// A mesh whose box reaches this far (centimetres) in front of a doorway
/// plane is tested against the sector beyond.
pub const DOORWAY_REACH: f32 = 20.0;
/// A doorway plane faces the eye when the eye is at least this far in
/// front of it (compared in double precision).
pub const PORTAL_FACING: f64 = 1e-6;

/// What a node is, from the class code in its scene word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// A plain scene node (class 0): registration only sets its owner.
    Node,
    /// A node the scene's "node registered" callback is called for
    /// (class 1).
    Callback,
    /// Geometry (class 3): joins the scene's geometry list.
    Geometry,
    /// A light (class 5): joins its sector's light list unless it is a
    /// world light.
    Light,
    /// A mesh with material parts (class 9): joins its sector's mesh list,
    /// the list the bounds and the visibility passes walk.
    Mesh,
    /// A sector (class 0x11): a top-level scene entry with its own nodes.
    Sector,
    /// The auxiliary list's kind (class 0x41); registration copies its
    /// position and calls the scene's handler (role not named in the
    /// evidence).
    Auxiliary,
}

impl NodeKind {
    /// From the class code in a node's scene word. A sector's code is
    /// tested over all 12 bits, as the original's batch insertion does;
    /// the other kinds over the 0x5f mask its registration uses.
    pub fn from_class(class: u32) -> Self {
        if class & 0xfff == CLASS_SECTOR {
            return Self::Sector;
        }
        match class & 0x5f {
            0x1 => Self::Callback,
            0x3 => Self::Geometry,
            0x5 => Self::Light,
            0x9 => Self::Mesh,
            0x41 => Self::Auxiliary,
            _ => Self::Node,
        }
    }
}

/// What a sector shows this frame. The numbers match the original's sector
/// state field: 0 hidden, 1 active, 2 adjacent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SectorState {
    #[default]
    Hidden,
    Active,
    Adjacent,
}

impl From<RoomState> for SectorState {
    fn from(state: RoomState) -> Self {
        match state {
            RoomState::Hidden => Self::Hidden,
            RoomState::Adjacent => Self::Adjacent,
            RoomState::Active => Self::Active,
        }
    }
}

/// A box with its bounding radius.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bounds {
    pub centre: Vec3,
    pub half_extents: Vec3,
    pub radius: f32,
}

impl Bounds {
    /// The empty box at the origin.
    pub const ZERO: Self = Self {
        centre: Vec3::ZERO,
        half_extents: Vec3::ZERO,
        radius: 0.0,
    };

    /// Bounds from a centre and half extents.
    pub fn new(centre: Vec3, half_extents: Vec3) -> Self {
        Self {
            centre,
            half_extents,
            radius: radius_of(half_extents),
        }
    }

    /// From running low and high corners, as the original turns them into
    /// a box: the centre is `low + (high − low) · 0.5`, the extents run to
    /// the high corner.
    pub fn from_min_max(low: Vec3, high: Vec3) -> Self {
        let centre = low + (high - low) * 0.5;
        Self::new(centre, high - centre)
    }

    pub fn min(&self) -> Vec3 {
        self.centre - self.half_extents
    }

    pub fn max(&self) -> Vec3 {
        self.centre + self.half_extents
    }
}

/// The original's box radius: `√((x² + z²) + y²)`.
fn radius_of(v: Vec3) -> f32 {
    (v.x * v.x + v.z * v.z + v.y * v.y).sqrt()
}

/// A box's extent along world axis `axis`: `|r₀|·hx + |r₁|·hy + |r₂|·hz`
/// over that row of the rotation, summed in that order.
fn axis_extent(rotation: Mat3, half_extents: Vec3, axis: usize) -> f32 {
    let row = rotation.row(axis).abs();
    row.x * half_extents.x + row.y * half_extents.y + row.z * half_extents.z
}

/// A box's extents along the three world axes ([`axis_extent`] per axis).
fn world_extent(rotation: Mat3, half_extents: Vec3) -> Vec3 {
    Vec3::new(
        axis_extent(rotation, half_extents, 0),
        axis_extent(rotation, half_extents, 1),
        axis_extent(rotation, half_extents, 2),
    )
}

/// Grows running corners by a box's per-axis extent, with the original's
/// ordered comparisons (a NaN leaves a corner alone).
fn grow(low: &mut Vec3, high: &mut Vec3, centre: Vec3, extent: Vec3) {
    for axis in 0..3 {
        let side = centre[axis] - extent[axis];
        if side < low[axis] {
            low[axis] = side;
        }
        let side = centre[axis] + extent[axis];
        if side > high[axis] {
            high[axis] = side;
        }
    }
}

/// Caps each axis extent at the box's radius, as the original does before
/// it grows a box by a child.
fn clamp_to_radius(mut extent: Vec3, radius: f32) -> Vec3 {
    for axis in 0..3 {
        if extent[axis] > radius {
            extent[axis] = radius;
        }
    }
    extent
}

/// The radius about `centre` of the box from `low` to `high`: each axis
/// takes the side farther from the centre (the high side unless the low
/// one is strictly farther), and the length sums `(x² + y²) + z²`.
fn union_radius(centre: Vec3, low: Vec3, high: Vec3) -> f32 {
    let mut side = Vec3::ZERO;
    for axis in 0..3 {
        let low_side = low[axis] - centre[axis];
        let high_side = high[axis] - centre[axis];
        side[axis] = if high_side.abs() < low_side.abs() {
            low_side
        } else {
            high_side
        };
    }
    (side.x * side.x + side.y * side.y + side.z * side.z).sqrt()
}

/// A sector's bounds, as the original keeps them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SectorBounds {
    /// The box of the [`BOUNDS_SPLIT`] meshes, in the sector's space.
    pub first: Bounds,
    /// `first`'s centre in the world (moved by the sector's origin).
    pub world_centre: Vec3,
    /// The box of every mesh (the running corners carry on from the first
    /// box), in the sector's space. Its radius is not part of the original.
    pub second: Bounds,
    /// The radius about `world_centre` of the box that holds every mesh.
    pub union_radius: f32,
}

/// One mesh as the bounds see it: its box centre, its extent along each
/// world axis (from its rotation), its bounding radius and whether it bounds
/// the first box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChildBox {
    pub centre: Vec3,
    pub extent: Vec3,
    pub radius: f32,
    pub split: bool,
}

/// A sector's bounds from its meshes, as the original computes them; `None`
/// when it has none (the bounds are then left as they are).
///
/// 1. The running corners start at zero, or at the first split mesh's
///    centre when there is one; every split mesh grows them by its extents
///    capped at its radius. They give the first box.
/// 2. The other meshes grow the same corners further, which gives the
///    second box.
/// 3. The union radius is measured about the first box's world centre,
///    against corners that are in the sector's space.
pub fn sector_bounds(children: &[ChildBox], origin: Vec3) -> Option<SectorBounds> {
    if children.is_empty() {
        return None;
    }
    let mut low = Vec3::ZERO;
    let mut high = Vec3::ZERO;
    let mut seeded = false;
    for child in children.iter().filter(|c| c.split) {
        if !seeded {
            low = child.centre;
            high = child.centre;
            seeded = true;
        }
        grow(
            &mut low,
            &mut high,
            child.centre,
            clamp_to_radius(child.extent, child.radius),
        );
    }
    let first = Bounds::from_min_max(low, high);
    let world_centre = first.centre + origin;
    for child in children.iter().filter(|c| !c.split) {
        grow(
            &mut low,
            &mut high,
            child.centre,
            clamp_to_radius(child.extent, child.radius),
        );
    }
    let second = Bounds::from_min_max(low, high);
    Some(SectorBounds {
        first,
        world_centre,
        second,
        union_radius: union_radius(world_centre, low, high),
    })
}

/// The scene's bounds, as the original's scene-bounds pass writes them into
/// the scene's own sector (`own`) from the other sectors (`rooms`):
///
/// Each room comes with its rotation (the sector node's frame), which turns
/// its half extents into world-axis extents as a mesh's are
/// (`|r₀|·hx + |r₁|·hy + |r₂|·hz` over each row, summed in that order):
///
/// 1. the first box starts as `own`'s first box and grows by every room's
///    first box about its world centre (those extents capped at its
///    radius); its world centre is its centre (the scene is not moved);
/// 2. the union corners start as `own`'s second box's centre and half
///    extents taken as corners (the original's arithmetic), grow by every
///    room's second box about its centre in the room's own space (not
///    moved by the room's origin, as the original reads it) with its
///    rotated extents, uncapped, and give the union radius about the first
///    centre;
/// 3. `own`'s second box is left as it is.
pub fn scene_bounds(own: SectorBounds, rooms: &[(SectorBounds, Mat3)]) -> SectorBounds {
    let mut low = own.first.min();
    let mut high = own.first.max();
    for (room, rotation) in rooms {
        grow(
            &mut low,
            &mut high,
            room.world_centre,
            clamp_to_radius(
                world_extent(*rotation, room.first.half_extents),
                room.first.radius,
            ),
        );
    }
    let first = Bounds::from_min_max(low, high);
    let mut low = own.second.centre;
    let mut high = own.second.half_extents;
    for (room, rotation) in rooms {
        grow(
            &mut low,
            &mut high,
            room.second.centre,
            world_extent(*rotation, room.second.half_extents),
        );
    }
    SectorBounds {
        first,
        world_centre: first.centre,
        second: own.second,
        union_radius: union_radius(first.centre, low, high),
    }
}

/// A node's cell in the 6 m grid and which sides it spills over.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellSpill {
    /// Grid cell (column, row), clamped to the grid's inner cells.
    pub cell: Option<(i64, i64)>,
    /// [`CELL_MARK`] plus the [`SPILL_LOW`]/[`SPILL_HIGH`] bits.
    pub mask: u16,
}

impl CellSpill {
    /// The word the original leaves on a node before the cell is worked out.
    pub const MARKED: Self = Self {
        cell: None,
        mask: CELL_MARK,
    };
}

/// The cell holding `position` and the sides of it the box spills over, as
/// the original works it out for a node's box (centimetres inside).
///
/// The cell is `floor(x · (1/600))` plus half the grid, kept in one byte
/// (a far-out position wraps), then 0 becomes 1 and anything from
/// `count − 1` up becomes `count − 2`. A side spills low when the box
/// reaches more than 1 cm below the cell's low edge, otherwise high when it
/// reaches past 601 cm (the original's asymmetry). `None` when the grid is
/// too small to hold inner cells (an Exarust guard).
pub fn cell_spill(
    position: Vec3,
    half_extents: Vec3,
    rotation: Mat3,
    grid: (usize, usize),
) -> CellSpill {
    let (width, height) = grid;
    if width < 3 || height < 3 {
        return CellSpill::MARKED;
    }
    let mut mask = CELL_MARK;
    let mut cells = [0i64; 2];
    for (slot, world_axis, count) in [(0usize, 0usize, width), (1, 2, height)] {
        let centimetres = position[world_axis] * 100.0;
        let extent = axis_extent(rotation, half_extents, world_axis) * 100.0;
        let index = (centimetres * INV_CELL_CM).floor();
        let local = centimetres - index * CELL_CM;
        if local - extent < -1.0 {
            mask |= SPILL_LOW << slot;
        } else if local + extent > 601.0 {
            mask |= SPILL_HIGH << slot;
        }
        let byte = ((count / 2) as i64 + index as i64) as u8;
        let byte = if byte == 0 {
            1
        } else if byte as i64 >= count as i64 - 1 {
            (count as i64 - 2) as u8
        } else {
            byte
        };
        cells[slot] = byte as i64;
    }
    CellSpill {
        cell: Some((cells[0], cells[1])),
        mask,
    }
}

/// A doorway of a sector, as the visibility pass sees it: the opening's
/// quad, its plane (facing the sector the eye looks from), the two grid
/// cells it joins and the sector it leads to.
#[derive(Clone, Debug, PartialEq)]
pub struct Portal {
    /// The four corner points of the opening, in order.
    pub quad: [Vec3; 4],
    /// Plane normal, pointing to the side the eye must be on.
    pub normal: Vec3,
    /// Plane offset: `normal · x + offset = 0` on the doorway.
    pub offset: f32,
    /// The flat grid cell indices on either side of the door.
    pub cells: [usize; 2],
    /// The sector the doorway leads to (the one that must be active).
    pub neighbour: usize,
}

impl Portal {
    /// A doorway standing on the edge between two cells (an Exarust
    /// constructor; the original reads a stored opening). `front_centre`
    /// and `back_centre` are the cells' centres (metres, X/Z), `size` the
    /// opening (width, height), `floor` its floor height. The plane faces
    /// the front cell.
    pub fn between_cells(
        front_cell: usize,
        back_cell: usize,
        front_centre: Vec2,
        back_centre: Vec2,
        size: Vec2,
        floor: f32,
    ) -> Self {
        let across = (front_centre - back_centre).normalize_or_zero();
        let normal = Vec3::new(across.x, 0.0, across.y);
        let along = Vec3::new(-normal.z, 0.0, normal.x);
        let centre = Vec3::new(
            (front_centre.x + back_centre.x) * 0.5,
            floor,
            (front_centre.y + back_centre.y) * 0.5,
        );
        let half_width = along * (size.x * 0.5);
        let up = Vec3::Y * size.y;
        let quad = [
            centre - half_width,
            centre + half_width,
            centre + half_width + up,
            centre - half_width + up,
        ];
        let offset = -normal.dot(centre);
        Self {
            quad,
            normal,
            offset,
            cells: [front_cell, back_cell],
            neighbour: 0,
        }
    }

    /// The signed distance of `point` in front of the doorway plane, summed
    /// in the original's order: `(x·nx + y·ny) + z·nz`, then the offset.
    pub fn distance(&self, point: Vec3) -> f32 {
        (point.x * self.normal.x + point.y * self.normal.y) + point.z * self.normal.z + self.offset
    }
}

/// A screen-space rectangle: the four rounded bounds of a portal scissor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenRect {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

/// One sub-object of a node: its own flags word and the value word a world
/// light clears. The original's scene-node unit touches only these two
/// words of a mesh's parts ([`Scene::clear_world_light`]) and asks each
/// part whether its own transform moved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SubMesh {
    pub flags: u32,
    pub value: u32,
    /// The part's own transform changed since the last pass (the part's
    /// transform callback's answer).
    pub moved: bool,
}

/// One light alias of a sector: the light's copy in that sector, as the
/// original creates one when a light is registered or moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightAlias {
    /// The light node the alias copies.
    pub light: usize,
    /// The sector that owns the alias; `None` once its light dropped it.
    /// The membership rebuild removes an alias this sector does not own.
    pub owner: Option<usize>,
    /// The light moved since the last pass (a new alias starts moved).
    pub moved: bool,
    /// The link face mask the per-frame pass builds.
    pub mask: u32,
    /// The shadow map's needed-face word, when the light has a shadow map
    /// (reset to `0x3f`, all six faces, when the alias moved).
    pub shadow_faces: Option<u32>,
    /// The shadow map's linked-face word: the visible pass ORs the alias's
    /// link mask into it (only while the light has a shadow map).
    pub shadow_links: u32,
    /// The light's update stamp this alias copied. An alias whose stamp
    /// matches its light is current; one that does not is refreshed and
    /// marked moved, as the original compares the two update words.
    pub stamp: u32,
}

/// One entry of a light node's link list: the weight one of the light's
/// sector aliases gives it and the doubly linked list's neighbours, as the
/// original's 24-byte records hold them. The list runs heaviest first and
/// a NaN weight stops the sort's walk wherever it meets one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LightLink {
    /// The weight the alias's link method gave this link (the original's
    /// float at the record's start).
    pub weight: f32,
    /// The heavier neighbour, or −1 at the head.
    pub prev: i32,
    /// The lighter neighbour, or −1 at the end.
    pub next: i32,
}

/// One entry of a node's per-alias bucket list: a doubly linked list over
/// the sector's aliases, as [`Scene::rebuild_buckets`] builds it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bucket {
    /// The entry's leading word (a weight the alias test resets).
    pub weight: u32,
    pub prev: i32,
    pub next: i32,
    /// The face mask the alias test fills in.
    pub faces: u32,
    /// The alias the entry stands for.
    pub alias: usize,
}

impl Bucket {
    /// A zero-filled entry, as growing the original's array leaves one.
    pub const ZERO: Self = Self {
        weight: 0,
        prev: 0,
        next: 0,
        faces: 0,
        alias: 0,
    };
}

/// One node of the scene.
#[derive(Clone, Debug)]
pub struct SceneNode {
    pub kind: NodeKind,
    /// Scene word bits: [`IN_SCENE`], [`OWNED`] and the cut-away bits
    /// ([`CUT_AWAY_KIND`], [`CUT_AWAY_PARENT`], [`CUT_AWAY_CHILDREN`]).
    pub scene_flags: u32,
    /// Option word bits: [`PREPARED`], [`WORLD_LIGHT`], [`BOUNDS_SPLIT`].
    pub flags: u32,
    /// The parent node; `None` hangs the node directly from its sector.
    pub parent: Option<usize>,
    /// Child slots; `None` is a slot the child validation drops.
    pub children: Vec<Option<usize>>,
    /// Position relative to the parent (to the sector at the top).
    pub position: Vec3,
    pub rotation: Mat3,
    /// Half extents of the node's box, about its origin.
    pub half_extents: Vec3,
    /// The frame in the sector's space, kept by [`Scene::refresh`].
    pub world: Vec3,
    pub world_rotation: Mat3,
    /// The frame has not been worked out since the local frame changed.
    pub stale: bool,
    /// The frame changed since the last visibility pass (the original's
    /// "moved this frame" marker, set on a whole subtree by its transform
    /// update).
    pub moved: bool,
    /// The sector that owns the node; `None` is the global `_World`.
    pub sector: Option<usize>,
    /// The flags word of the node's payload (0 without one); see
    /// [`PAYLOAD_CLIMBS`] and [`PAYLOAD_CHARACTER`].
    pub payload_flags: u32,
    /// The character data a character payload holds.
    pub character: Option<u64>,
    /// The node's object-database word; see [`OBJECT_SLOT_LIVE`].
    pub object_slot: u32,
    /// How many owners still hold the node: a queued removal
    /// with a live reference is deferred to the free list.
    pub refs: i32,
    /// The node was flushed from the removal queue.
    pub removed: bool,
    /// A light's aliases (copies of it registered in other sectors).
    pub aliases: Vec<usize>,
    /// The values a light's last alias sync copied (colour, range, cone,
    /// flags, intensity, scale); `None` until one runs.
    pub alias_values: Option<crate::light_source::AliasValues>,
    /// The node's cell and spill mask.
    pub cell: CellSpill,
    /// The frame word bits [`INHERIT_FADE`]/[`INHERIT_HIDDEN`].
    pub inherited: u32,
    /// Visibility mask from the last pass.
    pub mask: u32,
    /// The cut-away detector put the node in front of the player.
    pub cut_away: bool,
    /// Fade amount, 0 solid to 1 cut away.
    pub fade: f32,
    /// What the fade moves towards.
    pub fade_target: f32,
    /// The far sort key the node draw pass reads (`-100000` when reset).
    pub sort_key: f32,
    /// The node's sub-objects (a mesh's parts), for the world-light flag
    /// clear and the per-material lists.
    pub parts: Vec<SubMesh>,
    /// The material id of each part, in part order.
    pub part_materials: Vec<usize>,
    /// The node's own word cleared with its parts by a world light.
    pub part_word: u32,
    /// The node's state word: 0 never worked out, 1 its transform was just
    /// updated ([`Scene::refresh`] writes it, as the original's transform
    /// update does), 2 or more seen by a pass since. A light or auxiliary
    /// node below 2 is activated by the frame; a mesh at 1 is handled as
    /// moved by the node passes.
    pub light_state: u8,
    /// The light's own word: [`LIGHT_NO_SNAPSHOT`], [`LIGHT_FLICKER`].
    pub light_flags: u32,
    /// The light's kind word; 3 is omni and floods the cells.
    pub light_type: u32,
    /// The position copied when a light or auxiliary node activated.
    pub active_frame: Vec3,
    /// The head of the node's bucket list (−1 for none).
    pub bucket_head: i32,
    /// One bucket per sector alias.
    pub buckets: Vec<Bucket>,
    /// The head of the light's link list (−1 for none): one link per sector
    /// alias, heaviest first.
    pub link_head: i32,
    /// The light's link records, in alias order before
    /// [`Scene::order_links`] sorts their chain.
    pub links: Vec<LightLink>,
    /// A light's update stamp: the tick its links were last built, with 1
    /// meaning they are due. An alias copies it so a spread walk knows
    /// whether the alias is current.
    pub link_stamp: u32,
}

impl SceneNode {
    fn new(kind: NodeKind) -> Self {
        Self {
            kind,
            scene_flags: 0,
            flags: 0,
            parent: None,
            children: Vec::new(),
            position: Vec3::ZERO,
            rotation: Mat3::IDENTITY,
            half_extents: Vec3::ZERO,
            world: Vec3::ZERO,
            world_rotation: Mat3::IDENTITY,
            stale: true,
            moved: false,
            sector: None,
            payload_flags: 0,
            character: None,
            object_slot: 0,
            refs: 0,
            removed: false,
            aliases: Vec::new(),
            alias_values: None,
            cell: CellSpill::MARKED,
            inherited: 0,
            mask: 0,
            cut_away: false,
            fade: 0.0,
            fade_target: 1.0,
            sort_key: SORT_KEY_NONE,
            parts: Vec::new(),
            part_materials: Vec::new(),
            part_word: 0,
            light_state: 0,
            light_flags: 0,
            light_type: 0,
            active_frame: Vec3::ZERO,
            bucket_head: -1,
            buckets: Vec::new(),
            link_head: -1,
            links: Vec::new(),
            link_stamp: 1,
        }
    }

    /// The node's oriented box, with its bounding radius as the original
    /// keeps it.
    pub fn obb(&self) -> (Obb, f32) {
        let axes = [
            self.world_rotation.x_axis,
            self.world_rotation.y_axis,
            self.world_rotation.z_axis,
        ];
        (
            Obb::new(self.world, self.half_extents, axes),
            radius_of(self.half_extents),
        )
    }

    /// The node's box as the bounds see it.
    pub fn child_box(&self) -> ChildBox {
        ChildBox {
            centre: self.world,
            extent: world_extent(self.world_rotation, self.half_extents),
            radius: radius_of(self.half_extents),
            split: self.flags & BOUNDS_SPLIT != 0,
        }
    }

    /// The cell and spill mask of the node's frame.
    pub fn cell_spill(&self, grid: (usize, usize)) -> CellSpill {
        cell_spill(self.world, self.half_extents, self.world_rotation, grid)
    }

    /// Whether the node should be submitted solid, dithered, partially, or
    /// not at all (the visibility mask's draw bits).
    pub fn draw(&self) -> Draw {
        if self.mask & MAIN != 0 {
            Draw::Solid
        } else if self.mask & DITHERED != 0 {
            Draw::Dithered
        } else if self.mask & PARTIAL != 0 {
            Draw::Partial
        } else {
            Draw::None
        }
    }

    /// Whether any view sees the node (shadow faces may be needed).
    pub fn seen(&self) -> bool {
        self.mask & SEEN != 0
    }
}

/// How a node is submitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Draw {
    None,
    Solid,
    Dithered,
    Partial,
}

/// One sector: a room, or the scene's own sector.
#[derive(Clone, Debug)]
pub struct Sector {
    pub name: String,
    /// The sector's node (its children are its top nodes); the scene's own
    /// sector has none.
    pub node: Option<usize>,
    /// Option word: [`LOCKED`].
    pub flags: u32,
    /// Change word: [`LINKS_CHANGED`], [`MOVED_OUT`], [`LIGHT_ADDED`],
    /// [`MESH_ADDED`].
    pub changes: u32,
    /// Whether the sector is attached to the scene.
    pub in_scene: bool,
    /// The registered meshes, in registration order.
    pub meshes: Vec<usize>,
    /// How many of them have had their per-alias buckets built.
    /// The frame pass sees only this many until the additions are finished.
    pub meshes_done: usize,
    /// The sector's light aliases (one per light).
    pub aliases: Vec<LightAlias>,
    /// How many aliases the last visibility pass saw.
    pub aliases_done: usize,
    /// The registered (non-world) lights.
    pub lights: Vec<usize>,
    /// The registered auxiliary nodes.
    pub auxiliary: Vec<usize>,
    /// For each material id, the (node, part) pairs that use it,
    /// as registration appends them.
    pub materials: Vec<Vec<(usize, usize)>>,
    /// The sector is drawn through a portal: its meshes start
    /// with the scissor bit.
    pub through_portal: bool,
    /// The sector's doorways.
    pub portals: Vec<Portal>,
    /// The doorways (indices into `portals`) that lead to an active sector
    /// this frame, facing the eye or not.
    pub visible_portals: Vec<usize>,
    /// The room manager's state for this frame.
    pub state: SectorState,
    /// The view bitmask from the last classification.
    pub mask: u32,
    /// The sector's views for this frame.
    pub views: Vec<ConvexVolume>,
    /// The portal scissor rectangle when the sector is seen through doors.
    pub scissor: Option<ScreenRect>,
    /// The sector's translation in the world.
    pub origin: Vec3,
    /// The sector's rotation in the world (its node's frame; its rows are
    /// the box axes): the scene bounds turn the sector's boxes into world
    /// extents by it and the active-sector test meets its box along it.
    /// Exarust's rooms are unrotated, so it stays the identity unless a
    /// caller sets it.
    pub rotation: Mat3,
    /// The bounds of its meshes.
    pub bounds: SectorBounds,
    /// The sectors this one is linked to through its doorways, as the
    /// original keeps a sector's linked-sector list. A light's aliases
    /// spread along it.
    pub linked: Vec<usize>,
    /// A spread walk is inside this sector (the original's busy byte): a
    /// link back to it is skipped so a loop terminates.
    pub spreading: bool,
}

impl Sector {
    fn new(name: impl Into<String>, node: Option<usize>) -> Self {
        Self {
            name: name.into(),
            node,
            flags: 0,
            changes: 0,
            in_scene: false,
            meshes: Vec::new(),
            meshes_done: 0,
            aliases: Vec::new(),
            aliases_done: 0,
            lights: Vec::new(),
            auxiliary: Vec::new(),
            materials: Vec::new(),
            through_portal: false,
            portals: Vec::new(),
            visible_portals: Vec::new(),
            state: SectorState::Hidden,
            mask: 0,
            views: Vec::new(),
            scissor: None,
            origin: Vec3::ZERO,
            rotation: Mat3::IDENTITY,
            bounds: SectorBounds::default(),
            linked: Vec::new(),
            spreading: false,
        }
    }

    /// Empties the sector's lists, as its destructor empties the original's
    /// arrays (memory is Rust's to free, so nothing else changes: the
    /// destructor does not touch the sector's node, scene membership or
    /// scissor).
    pub fn destroy(&mut self) {
        self.meshes.clear();
        self.meshes_done = 0;
        self.aliases.clear();
        self.aliases_done = 0;
        self.lights.clear();
        self.auxiliary.clear();
        self.materials.clear();
        self.portals.clear();
        self.visible_portals.clear();
        self.views.clear();
    }
}

/// The whole scene graph.
#[derive(Clone, Debug)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
    /// The scene's own sector first, then the appended sectors.
    pub sectors: Vec<Sector>,
    /// The scene's geometry list.
    pub geometry: Vec<usize>,
    /// Cell grid (columns, rows).
    pub grid: (usize, usize),
    /// The scene works in 6 m cells (a grid was set up): the visibility
    /// passes keep moved meshes' cells current.
    pub grid_mode: bool,
    /// The detail level and its 0.01 scale (the original's two words the
    /// item and character views set).
    pub detail: i32,
    pub detail_scale: f32,
    /// The timed node transitions.
    pub transitions: Vec<Transition>,
    /// Nodes queued for removal.
    pub removals: RemovalQueue,
    /// The renderers registered with the scene.
    pub renderers: Vec<u32>,
    /// The scene plugins.
    pub plugins: Vec<u32>,
    /// The scene-wide lights.
    pub lights: Vec<u32>,
    /// The material manager's material count, as the add/remove flush grows
    /// the sectors' per-material arrays to it.
    pub material_count: usize,
    /// Materials queued for the renderers (the add queue).
    pub material_adds: Vec<usize>,
    /// Materials queued for release (the remove queue).
    pub material_removes: Vec<usize>,
    /// Nodes whose release was deferred to their owner (the scene's free
    /// list).
    pub pending_free: Vec<usize>,
}

impl Scene {
    /// A scene with only its own sector. A non-empty `grid` puts it in grid
    /// mode, as setting up the cell arrays does.
    pub fn new(grid: (usize, usize)) -> Self {
        let mut own = Sector::new("scene", None);
        own.in_scene = true;
        Self {
            nodes: Vec::new(),
            sectors: vec![own],
            geometry: Vec::new(),
            grid,
            grid_mode: grid.0 > 0 && grid.1 > 0,
            detail: 0,
            detail_scale: DEFAULT_DETAIL_SCALE,
            transitions: Vec::new(),
            removals: RemovalQueue::default(),
            renderers: Vec::new(),
            plugins: Vec::new(),
            lights: Vec::new(),
            material_count: 0,
            material_adds: Vec::new(),
            material_removes: Vec::new(),
            pending_free: Vec::new(),
        }
    }

    /// Adds a node, with `parent` none for a node that hangs from its
    /// sector. Its frame is stale until it is refreshed.
    pub fn add_node(&mut self, kind: NodeKind, parent: Option<usize>) -> usize {
        let id = self.nodes.len();
        let mut node = SceneNode::new(kind);
        node.parent = parent;
        self.nodes.push(node);
        if let Some(parent) = parent {
            self.nodes[parent].children.push(Some(id));
        }
        id
    }

    /// Sets a node's local frame and box.
    pub fn set_frame(&mut self, node: usize, position: Vec3, rotation: Mat3, half_extents: Vec3) {
        let n = &mut self.nodes[node];
        n.position = position;
        n.rotation = rotation;
        n.half_extents = half_extents;
        n.stale = true;
    }

    /// Adds a doorway to a sector. The caller sets its `neighbour`.
    pub fn add_portal(&mut self, sector: usize, portal: Portal) {
        self.sectors[sector].portals.push(portal);
    }

    /// The node directly under its sector: climbs the parents until the
    /// next one is a sector, as the light and player helpers do before they
    /// take a node's sector frame.
    pub fn sector_child(&self, node: usize) -> usize {
        let mut at = node;
        while let Some(parent) = self.nodes[at].parent {
            if self.nodes[parent].kind == NodeKind::Sector {
                break;
            }
            at = parent;
        }
        at
    }

    /// Whether the node hangs directly from a sector.
    fn under_sector(&self, node: usize) -> bool {
        self.nodes[node]
            .parent
            .is_none_or(|p| self.nodes[p].kind == NodeKind::Sector)
    }

    /// Takes an alias out of its light's alias list, as the original does
    /// when the alias's sector drops it: the last alias takes its place,
    /// the alias's sector is marked [`LINKS_CHANGED`] and the alias loses
    /// its owner. Returns whether the alias was in the list.
    pub fn remove_alias(&mut self, light: usize, alias: usize) -> bool {
        let Some(at) = self.nodes[light].aliases.iter().position(|&a| a == alias) else {
            return false;
        };
        self.nodes[light].aliases.swap_remove(at);
        // The alias's owner is its sector, whose change word is marked.
        if let Some(sector) = self.nodes[alias].sector {
            self.sectors[sector].changes |= LINKS_CHANGED;
        }
        self.nodes[alias].sector = None;
        true
    }

    /// Copies a light's current values to every alias, as the original's
    /// alias sync does: the aliases are marked moved (so their shadow faces
    /// reset) and keep the copied values. Returns how many were written.
    pub fn sync_light_aliases(
        &mut self,
        light: usize,
        values: &crate::light_source::AliasValues,
    ) -> usize {
        let aliases = self.nodes[light].aliases.clone();
        self.nodes[light].alias_values = Some(*values);
        for &alias in &aliases {
            if let Some(a) = self.nodes.get_mut(alias) {
                a.alias_values = Some(*values);
                a.moved = true;
                a.mask = 0;
            }
        }
        aliases.len()
    }

    /// Sets the room sectors' states (room `i` is sector `i + 1`) from the
    /// room manager's states, leaving the scene's own sector alone.
    pub fn set_room_states(&mut self, states: &[RoomState]) {
        for (index, &state) in states.iter().enumerate() {
            if let Some(sector) = self.sectors.get_mut(index + 1) {
                sector.state = state.into();
            }
        }
    }

    /// Inserts a batch of top-level nodes, as the original's add-nodes call
    /// does. Each leading sector entry is appended to the sector list
    /// (marked in scene, its children registered in it). The first entry
    /// that is not a sector registers the **whole batch** in that entry's
    /// owner (the scene's own sector when it has none) and ends the loop.
    /// Returns whether any sector was appended.
    pub fn add_batch(&mut self, roots: &[usize]) -> bool {
        if !self.material_adds.is_empty() {
            self.flush_materials();
        }
        let mut added_sector = false;
        for &root in roots {
            if self.nodes[root].kind != NodeKind::Sector {
                let owner = self.nodes[root].sector.unwrap_or(SCENE_SECTOR);
                self.register_batch(owner, roots);
                break;
            }
            let sector = self.sectors.len();
            let mut entry = Sector::new(format!("sector {sector}"), Some(root));
            entry.in_scene = true;
            self.sectors.push(entry);
            self.nodes[root].scene_flags |= IN_SCENE;
            let children: Vec<usize> = self.nodes[root]
                .children
                .iter()
                .flatten()
                .copied()
                .collect();
            self.register_batch(sector, &children);
            added_sector = true;
        }
        added_sector
    }

    /// Registers each node of a list in `sector` unless the sector is
    /// [`LOCKED`].
    pub fn register_batch(&mut self, sector: usize, nodes: &[usize]) {
        if self.sectors[sector].flags & LOCKED != 0 {
            return;
        }
        for &node in nodes {
            self.register(node, sector);
        }
    }

    /// Registers a node and its subtree in `sector`, as the original's
    /// registration walk does. The node always takes the sector as its
    /// owner; then by kind:
    ///
    /// - a geometry node joins the scene's geometry list;
    /// - a light that is not a world light joins the sector's light list,
    ///   is prepared, gets [`IN_SCENE`] and marks [`LIGHT_ADDED`] (a world
    ///   light's part flags are render-side);
    /// - a mesh joins the mesh list, gets [`IN_SCENE`] and is prepared the
    ///   first time, and marks [`MESH_ADDED`];
    /// - an auxiliary node joins the auxiliary list, is prepared and gets
    ///   [`IN_SCENE`];
    /// - plain, callback and sector nodes only take the owner.
    ///
    /// Lists are appended to as the original does, without a duplicate
    /// check.
    pub fn register(&mut self, node: usize, sector: usize) {
        self.nodes[node].sector = Some(sector);
        match self.nodes[node].kind {
            NodeKind::Geometry => self.geometry.push(node),
            NodeKind::Light if self.nodes[node].flags & WORLD_LIGHT == 0 => {
                self.sectors[sector].lights.push(node);
                let n = &mut self.nodes[node];
                n.flags |= PREPARED;
                n.scene_flags |= IN_SCENE;
                self.sectors[sector].changes |= LIGHT_ADDED;
            }
            NodeKind::Light => {
                // A world light clears the shadow flags of its fixture's
                // parts instead of joining the sector's list.
                self.clear_world_light(node);
            }
            NodeKind::Mesh => {
                self.sectors[sector].meshes.push(node);
                let first = self.nodes[node].scene_flags & IN_SCENE == 0;
                let materials = if first {
                    Some(self.nodes[node].part_materials.clone())
                } else {
                    None
                };
                {
                    let n = &mut self.nodes[node];
                    if first {
                        n.scene_flags |= IN_SCENE;
                        n.flags |= PREPARED;
                    }
                }
                if let Some(materials) = materials {
                    for (part, &material) in materials.iter().enumerate() {
                        let lists = &mut self.sectors[sector].materials;
                        if lists.len() <= material {
                            lists.resize_with(material + 1, Vec::new);
                        }
                        lists[material].push((node, part));
                    }
                }
                self.sectors[sector].changes |= MESH_ADDED;
            }
            NodeKind::Auxiliary => {
                self.sectors[sector].auxiliary.push(node);
                let n = &mut self.nodes[node];
                n.flags |= PREPARED;
                n.scene_flags |= IN_SCENE;
            }
            _ => {}
        }
        let children: Vec<usize> = self.nodes[node]
            .children
            .iter()
            .flatten()
            .copied()
            .collect();
        for child in children {
            self.register(child, sector);
        }
    }

    /// Takes a node and its subtree out of the scene: back to `_World`, out
    /// of its sector's lists, without [`IN_SCENE`]. (The original queues
    /// the removal and rebuilds the sector's lists; the node class clears
    /// the flag.)
    pub fn remove(&mut self, node: usize) {
        if let Some(sector) = self.nodes[node].sector {
            let s = &mut self.sectors[sector];
            s.meshes.retain(|&n| n != node);
            s.lights.retain(|&n| n != node);
            s.auxiliary.retain(|&n| n != node);
        }
        self.geometry.retain(|&n| n != node);
        self.nodes[node].sector = None;
        self.nodes[node].scene_flags &= !IN_SCENE;
        let children: Vec<usize> = self.nodes[node]
            .children
            .iter()
            .flatten()
            .copied()
            .collect();
        for child in children {
            self.remove(child);
        }
    }

    /// Takes a subtree out of the scene while leaving its owner and parent
    /// links alone, as the content helper's detach does: every node loses
    /// [`IN_SCENE`], leaves its sector's lists and the scene's geometry
    /// list, and a light drops its aliases. Returns how many nodes changed.
    pub fn detach_subtree(&mut self, node: usize) -> usize {
        let Some(n) = self.nodes.get(node) else {
            return 0;
        };
        let children = n.children.clone();
        let sector = n.sector;
        let kind = n.kind;
        let mut count = 0;
        if let Some(s) = sector.and_then(|s| self.sectors.get_mut(s)) {
            s.meshes.retain(|&m| m != node);
            s.lights.retain(|&l| l != node);
            s.auxiliary.retain(|&a| a != node);
            if kind == NodeKind::Light {
                s.changes |= LINKS_CHANGED;
            }
        }
        self.geometry.retain(|&g| g != node);
        let aliases = std::mem::take(&mut self.nodes[node].aliases);
        for alias in aliases {
            if let Some(alias) = self.nodes.get_mut(alias) {
                alias.sector = None;
            }
        }
        let was_in_scene = self.nodes[node].scene_flags & IN_SCENE != 0;
        self.nodes[node].scene_flags &= !IN_SCENE;
        if was_in_scene {
            count += 1;
        }
        for child in children.into_iter().flatten() {
            count += self.detach_subtree(child);
        }
        count
    }

    /// Releases a subtree at once, without the removal queue's ownership
    /// wait: the nodes leave their sector and parent, are marked removed
    /// and lose the scene bit. Returns how many were newly removed.
    pub fn release_subtree(&mut self, node: usize) -> usize {
        let sector = self.nodes.get(node).and_then(|n| n.sector);
        self.remove_subtree(node, sector)
    }

    /// Attaches `nodes` as the children of `parent`, as the content helper's
    /// attach does: each node leaves its old parent's list, takes `parent`
    /// and the parent's owner, and has its frame refreshed. The list is
    /// appended in order. Returns how many nodes were attached.
    pub fn attach_nodes(&mut self, parent: usize, nodes: &[usize]) -> usize {
        let owner = self.owner_of(parent);
        let mut attached = 0;
        for &node in nodes {
            if node >= self.nodes.len() || node == parent {
                continue;
            }
            if let Some(old) = self.nodes[node].parent
                && let Some(old) = self.nodes.get_mut(old)
            {
                old.children.retain(|&child| child != Some(node));
            }
            self.nodes[node].parent = Some(parent);
            self.nodes[node].sector = owner;
            self.nodes[node].stale = true;
            self.nodes[parent].children.push(Some(node));
            self.refresh(node);
            attached += 1;
        }
        attached
    }

    /// The sector a node's child belongs to: the sector whose node is
    /// `parent`, else the parent's own owner.
    fn owner_of(&self, parent: usize) -> Option<usize> {
        if let Some(sector) = self.sectors.iter().position(|s| s.node == Some(parent)) {
            return Some(sector);
        }
        self.nodes.get(parent).and_then(|n| n.sector)
    }

    /// Clears the scene's content as the content helper does: every
    /// top-level node's subtree is detached and released, sector lists,
    /// geometry, transitions, removals and the free list are emptied. The
    /// scene's own sector stays. Returns the released nodes' indices.
    pub fn clear_content(&mut self) -> Vec<usize> {
        let roots: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.parent.is_none())
            .map(|(i, _)| i)
            .collect();
        let mut removed = Vec::new();
        for root in roots {
            let subtree = self.subtree_nodes(root);
            self.detach_subtree(root);
            self.release_subtree(root);
            removed.extend(subtree);
        }
        self.sectors.truncate(1);
        self.geometry.clear();
        self.transitions.clear();
        self.removals = RemovalQueue::default();
        self.pending_free.clear();
        self.material_adds.clear();
        self.material_removes.clear();
        if let Some(own) = self.sectors.first_mut() {
            own.meshes.clear();
            own.lights.clear();
            own.auxiliary.clear();
            own.aliases.clear();
            own.materials.clear();
        }
        removed
    }

    /// A node and its descendants, in pre-order.
    pub fn subtree_nodes(&self, node: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(at) = stack.pop() {
            let Some(n) = self.nodes.get(at) else {
                continue;
            };
            out.push(at);
            for &child in n.children.iter().flatten() {
                stack.push(child);
            }
        }
        out
    }

    /// Moves a node's subtree to `sector` keeping its place in the world,
    /// as the original's move does: the local position gains the old
    /// sector's origin less the new one's, the frame is refreshed, the old
    /// sector is marked [`LINKS_CHANGED`] | [`MOVED_OUT`] (its lists drop
    /// the node at the next membership rebuild), the node hangs from the new
    /// sector, and a node in the scene is registered there. Returns whether
    /// the node was in the scene. (Physics bodies and shadow objects that
    /// the original also shifts are outside the model.)
    pub fn move_to_sector(&mut self, node: usize, sector: usize) -> bool {
        let old = self.nodes[node].sector;
        let old_origin = old.map_or(Vec3::ZERO, |s| self.sectors[s].origin);
        let offset = old_origin - self.sectors[sector].origin;
        self.nodes[node].position += offset;
        self.refresh(node);
        if let Some(old) = old {
            self.sectors[old].changes |= LINKS_CHANGED | MOVED_OUT;
        }
        if let Some(parent) = self.nodes[node].parent {
            self.nodes[parent].children.retain(|&c| c != Some(node));
        }
        self.nodes[node].parent = self.sectors[sector].node;
        if let Some(parent) = self.nodes[node].parent {
            self.nodes[parent].children.push(Some(node));
        }
        let in_scene = self.nodes[node].scene_flags & IN_SCENE != 0;
        if in_scene {
            self.register_batch(sector, &[node]);
        }
        in_scene
    }

    /// Refreshes a node's frame and its subtree's, as the node class's
    /// transform update does: a node directly under its sector takes its
    /// local frame, any other composes its parent's; the whole subtree is
    /// marked moved.
    pub fn refresh(&mut self, node: usize) {
        let parent = self.nodes[node]
            .parent
            .filter(|&p| self.nodes[p].kind != NodeKind::Sector);
        if let Some(parent) = parent
            && self.nodes[parent].stale
        {
            self.refresh(parent);
        }
        let frame = parent.map(|p| (self.nodes[p].world, self.nodes[p].world_rotation));
        let body = &mut self.nodes[node];
        match frame {
            None => {
                body.world = body.position;
                body.world_rotation = body.rotation;
            }
            Some((parent_world, parent_rotation)) => {
                body.world = parent_world + parent_rotation * body.position;
                body.world_rotation = parent_rotation * body.rotation;
            }
        }
        body.stale = false;
        body.moved = true;
        body.light_state = 1;
        let children: Vec<usize> = body.children.iter().flatten().copied().collect();
        for child in children {
            self.refresh(child);
        }
    }

    /// Validates a node's child list as the original does every frame for
    /// each sector: an empty slot takes the last child's place (the list
    /// shrinks and the slot is checked again); a child whose frame was
    /// never worked out is refreshed. Returns the number of dropped slots.
    /// (The original logs each one; Exarust's hierarchy has no empty
    /// slots, so only malformed model data reaches this.)
    pub fn validate_children(&mut self, node: usize) -> usize {
        let mut dropped = 0;
        let mut at = 0;
        while at < self.nodes[node].children.len() {
            match self.nodes[node].children[at] {
                None => {
                    self.nodes[node].children.swap_remove(at);
                    dropped += 1;
                }
                Some(child) => {
                    if self.nodes[child].stale {
                        self.refresh(child);
                    }
                    at += 1;
                }
            }
        }
        dropped
    }

    /// Whether the node or any descendant carries [`OWNED`] (the original's
    /// check before freeing a subtree).
    pub fn owns_subtree(&self, node: usize) -> bool {
        if self.nodes[node].scene_flags & OWNED != 0 {
            return true;
        }
        self.nodes[node]
            .children
            .iter()
            .flatten()
            .any(|&child| self.owns_subtree(child))
    }

    /// Works out a node's cell and spill mask. Returns whether the
    /// original would call the scene's cell handler: the cell (not the
    /// mask) changed and the node hangs directly from its sector.
    pub fn update_cell(&mut self, node: usize) -> bool {
        if self.nodes[node].stale {
            self.refresh(node);
        }
        let changed = store_cell(&mut self.nodes[node], self.grid);
        changed && self.under_sector(node)
    }

    /// Recomputes a sector's bounds from its meshes ([`sector_bounds`]); a
    /// sector without meshes keeps its bounds.
    pub fn rebuild_sector_bounds(&mut self, sector: usize) {
        let meshes = self.sectors[sector].meshes.clone();
        for &mesh in &meshes {
            if self.nodes[mesh].stale {
                self.refresh(mesh);
            }
        }
        let children: Vec<ChildBox> = meshes.iter().map(|&m| self.nodes[m].child_box()).collect();
        let origin = self.sectors[sector].origin;
        if let Some(bounds) = sector_bounds(&children, origin) {
            self.sectors[sector].bounds = bounds;
        }
    }

    /// Recomputes every sector's bounds, then the scene's bounds into its
    /// own sector ([`scene_bounds`]).
    pub fn rebuild_bounds(&mut self) {
        for sector in 0..self.sectors.len() {
            self.rebuild_sector_bounds(sector);
        }
        let rooms: Vec<(SectorBounds, Mat3)> = self.sectors[1..]
            .iter()
            .map(|s| (s.bounds, s.rotation))
            .collect();
        let own = self.sectors[SCENE_SECTOR].bounds;
        self.sectors[SCENE_SECTOR].bounds = scene_bounds(own, &rooms);
    }

    /// Classifies every sector for this frame, as the per-frame scene update
    /// does, from its room state, the scene's main views (world space) and
    /// the eye; `project` maps a world point to screen coordinates for a
    /// scissor.
    ///
    /// - A sector not in the scene or hidden gets no view bit and no
    ///   doorways.
    /// - The scene's own sector, in any other state, gets every bit and the
    ///   main views.
    /// - An adjacent sector lists its doorways to active sectors; each that
    ///   faces the eye is clipped by the first main view into a frustum
    ///   from the eye ([`ConvexVolume::clip_polygon`]). With any frustum it
    ///   gets bit 1 and the scissor of its frusta.
    /// - An active sector gets bit `j` for each main view `j` that meets its
    ///   world box, and keeps those views moved into its space. (The
    ///   original keeps one view slot per main view, and a slot it does not
    ///   refresh keeps its last copy; the model keeps only this frame's.)
    ///
    /// Then the sector node's children are validated.
    pub fn classify(&mut self, eye: Vec3, main: &[ConvexVolume], project: &dyn Fn(Vec3) -> Vec2) {
        let states: Vec<SectorState> = self.sectors.iter().map(|s| s.state).collect();
        for (index, &state) in states.iter().enumerate() {
            let sector = &mut self.sectors[index];
            sector.scissor = None;
            sector.through_portal = false;
            if !sector.in_scene || state == SectorState::Hidden {
                sector.mask = 0;
                sector.visible_portals.clear();
            } else if index == SCENE_SECTOR {
                sector.mask = u32::MAX;
                sector.views = main.to_vec();
            } else if state == SectorState::Adjacent {
                sector.visible_portals.clear();
                sector.views.clear();
                for (at, portal) in sector.portals.iter().enumerate() {
                    if states.get(portal.neighbour) != Some(&SectorState::Active) {
                        continue;
                    }
                    sector.visible_portals.push(at);
                    let facing = f64::from(portal.distance(eye));
                    if facing < PORTAL_FACING {
                        continue;
                    }
                    if let Some(view) = main.first().and_then(|v| v.clip_polygon(&portal.quad, eye))
                    {
                        sector.views.push(view);
                    }
                }
                if sector.views.is_empty() {
                    sector.mask = 0;
                } else {
                    sector.mask = 1;
                    sector.scissor = Some(scissor_rect(&sector.views, project));
                    sector.through_portal = true;
                }
            } else {
                sector.visible_portals.clear();
                let bounds = sector.bounds;
                let rotation = sector.rotation;
                let obb = Obb::new(
                    bounds.world_centre,
                    bounds.first.half_extents,
                    [rotation.x_axis, rotation.y_axis, rotation.z_axis],
                );
                sector.mask = 0;
                sector.views.clear();
                for (bit, view) in main.iter().enumerate().take(32) {
                    if view.intersects_obb(&obb, bounds.first.radius) {
                        sector.mask |= 1 << bit;
                        sector.views.push(view.relative_to(sector.origin));
                    }
                }
            }
            if let Some(node) = self.sectors[index].node {
                self.validate_children(node);
            }
        }
    }

    /// Rebuilds a sector's membership as the original's per-frame rebuild
    /// does, in its order:
    ///
    /// 1. Aliases past the processed count that the sector does not own
    ///    ([`LightAlias::owner`]) are removed, the last taking their place.
    /// 2. Among the processed aliases, each one the sector does not own is
    ///    removed and its slot is taken by the last kept, not yet moved
    ///    alias after it (so the kept aliases do **not** keep their order);
    ///    a removed alias's owner, if any, is marked [`LINKS_CHANGED`]. The
    ///    unprocessed aliases then follow the compacted ones.
    /// 3. Meshes the sector still owns stay, in order. A processed one that
    ///    needs buckets has its bucket chain remapped to the moved aliases
    ///    (when any alias went) and, when new aliases are waiting, linked to
    ///    them ([`Scene::rebuild_buckets`]). A mesh that left is dropped,
    ///    taken out of the per-material lists when it is not in the scene;
    ///    a processed one lowers the processed count and gives each kept
    ///    alias with a shadow map its bucket's faces as needed faces, so the
    ///    shadow redraws where it was.
    ///
    /// Every alias then counts as processed and the membership change bits
    /// clear. Returns whether a processed mesh was dropped (the original
    /// then runs the sector's callback).
    ///
    /// The original keeps a removed slot's index in the map negated, so a
    /// removed first alias with nothing to take its place maps to 0 and is
    /// read as kept by the bucket remap; that quirk is kept.
    pub fn rebuild_membership(&mut self, sector: usize) -> bool {
        let owned = |alias: &LightAlias| alias.owner == Some(sector);
        // 1. Unprocessed aliases that left.
        {
            let s = &mut self.sectors[sector];
            let mut at = s.aliases_done.min(s.aliases.len());
            while at < s.aliases.len() {
                if owned(&s.aliases[at]) {
                    at += 1;
                } else {
                    s.aliases.swap_remove(at);
                }
            }
        }
        // 2. The processed aliases' index map.
        let (done, pending) = {
            let s = &self.sectors[sector];
            let done = s.aliases_done.min(s.aliases.len());
            (done, done < s.aliases.len())
        };
        let mut map: Vec<i64> = (0..done as i64).collect();
        let mut removed: Vec<usize> = Vec::new();
        for i in 0..done {
            if owned(&self.sectors[sector].aliases[i]) {
                continue;
            }
            removed.push(i);
            let mut j = done;
            while j > i + 1 {
                j -= 1;
                if owned(&self.sectors[sector].aliases[j]) && map[j] == j as i64 {
                    map[i] = -map[j];
                    map[j] = i as i64;
                    break;
                }
            }
            if map[i] == i as i64 {
                map[i] = -(i as i64);
            }
        }
        let done_now = done - removed.len();
        if !removed.is_empty() {
            for &i in &removed {
                if let Some(owner) = self.sectors[sector].aliases[i].owner {
                    self.sectors[owner].changes |= LINKS_CHANGED;
                }
                let from = (-map[i]) as usize;
                let s = &mut self.sectors[sector];
                s.aliases[i] = s.aliases[from];
            }
            let s = &mut self.sectors[sector];
            let mut aliases = s.aliases[..done_now].to_vec();
            aliases.extend_from_slice(&s.aliases[done..]);
            s.aliases = aliases;
        }
        self.sectors[sector].aliases_done = done_now;
        // 3. The meshes.
        let old_meshes = self.sectors[sector].meshes.clone();
        let meshes_done = self.sectors[sector].meshes_done.min(old_meshes.len());
        let mut meshes = Vec::with_capacity(old_meshes.len());
        let mut processed = meshes_done;
        let mut dropped = false;
        for (i, &node) in old_meshes.iter().enumerate() {
            if self.nodes[node].sector == Some(sector) {
                if self.nodes[node].flags & NEEDS_BUCKETS != 0 && i < meshes_done {
                    if !removed.is_empty() {
                        remap_buckets(&mut self.nodes[node], &map, &removed, done_now);
                    }
                    if pending {
                        self.rebuild_buckets(sector, node, done_now);
                    }
                }
                meshes.push(node);
                continue;
            }
            if self.nodes[node].scene_flags & IN_SCENE == 0 {
                self.remove_from_materials(sector, node);
            }
            if i < meshes_done {
                processed -= 1;
                dropped = true;
                if done_now > 0 && self.nodes[node].bucket_head >= 0 {
                    for (k, &to) in map.iter().enumerate() {
                        let faces = self.nodes[node].buckets.get(k).map_or(0, |b| b.faces);
                        if to < 0 {
                            continue;
                        }
                        if let Some(alias) = self.sectors[sector].aliases.get_mut(to as usize)
                            && let Some(needed) = alias.shadow_faces
                        {
                            alias.shadow_faces = Some(needed | faces);
                        }
                    }
                }
            }
        }
        let s = &mut self.sectors[sector];
        s.meshes = meshes;
        s.meshes_done = processed;
        s.aliases_done = s.aliases.len();
        s.changes &= !MEMBERSHIP_CHANGED;
        dropped
    }

    /// Runs the second half of the per-frame scene update: sectors whose
    /// membership changed are rebuilt, new meshes' buckets are finished,
    /// and each sector runs the hidden pass (no view bit and no doorway to
    /// an active sector) or the visible pass, moving cut-away fades by
    /// `dt_ms` (whole milliseconds, as the original's frame delta is).
    /// `faces` is the alias shadow test (node, alias) → face mask.
    /// Returns whether a visible pass saw a change (an alias moved, a seen
    /// mesh or a seen mesh's part moved).
    pub fn update_visibility(&mut self, dt_ms: i32) -> bool {
        let mut no_faces = |_: usize, _: usize| 0u32;
        self.update_visibility_with(dt_ms, &mut no_faces)
    }

    /// [`Scene::update_visibility`] with the alias shadow test supplied.
    pub fn update_visibility_with(
        &mut self,
        dt_ms: i32,
        faces: &mut dyn FnMut(usize, usize) -> u32,
    ) -> bool {
        for sector in 0..self.sectors.len() {
            if self.sectors[sector].changes & MEMBERSHIP_CHANGED != 0 {
                self.rebuild_membership(sector);
            }
        }
        for sector in 0..self.sectors.len() {
            if self.sectors[sector].changes & MESH_ADDED != 0 {
                self.finish_new_meshes(sector);
            }
        }
        let mut moved_seen = false;
        let mut events = FrameEvents::default();
        for sector in 0..self.sectors.len() {
            let s = &self.sectors[sector];
            if s.mask == 0 && s.visible_portals.is_empty() {
                self.hidden_pass(sector, faces, &mut events);
            } else if self.visible_pass(sector, dt_ms, faces, &mut events) {
                moved_seen = true;
            }
        }
        moved_seen
    }

    /// One presented frame of the scene, in the original's order: sector
    /// classification, moved-light activation, the membership and bucket
    /// rebuilds, the node passes, the queued removals, and the material
    /// removes. The room callback and the render-side light callbacks are
    /// the caller's; the returned [`FrameEvents`] lists the work they would
    /// do.
    pub fn frame(
        &mut self,
        dt_ms: i32,
        eye: Vec3,
        main: &[ConvexVolume],
        project: &dyn Fn(Vec3) -> Vec2,
    ) -> FrameEvents {
        let mut no_faces = |_: usize, _: usize| 0u32;
        self.frame_with(dt_ms, eye, main, project, &mut no_faces)
    }

    /// [`Scene::frame`] with the alias shadow test supplied.
    pub fn frame_with(
        &mut self,
        dt_ms: i32,
        eye: Vec3,
        main: &[ConvexVolume],
        project: &dyn Fn(Vec3) -> Vec2,
        faces: &mut dyn FnMut(usize, usize) -> u32,
    ) -> FrameEvents {
        let mut events = FrameEvents::default();
        self.drain_removals();
        self.classify(eye, main, project);
        for sector in 0..self.sectors.len() {
            self.activate_lights(sector, &mut events);
        }
        for sector in 0..self.sectors.len() {
            if self.sectors[sector].changes & MEMBERSHIP_CHANGED != 0 {
                self.rebuild_membership(sector);
            }
        }
        for sector in 0..self.sectors.len() {
            if self.sectors[sector].changes & MESH_ADDED != 0 {
                self.finish_new_meshes(sector);
            }
        }
        for sector in 0..self.sectors.len() {
            let s = &self.sectors[sector];
            if s.mask == 0 && s.visible_portals.is_empty() {
                self.hidden_pass(sector, faces, &mut events);
            } else if self.visible_pass(sector, dt_ms, faces, &mut events) {
                events.moved_seen = true;
            }
        }
        let mut world_deleted = Vec::new();
        events.removed = self.process_removals_into(&mut world_deleted);
        events.world_deleted = world_deleted;
        events.freed = std::mem::take(&mut self.pending_free);
        events.materials_removed = self.flush_material_removes();
        events
    }

    /// The frame's first step on the removal queue: every entry in the ring
    /// moves, oldest first, to the end of the pending list, and its node
    /// hangs from `_World` (owner `None`) until the frame end hands it back.
    pub fn drain_removals(&mut self) {
        let drained = std::mem::take(&mut self.removals.entries);
        for entry in drained {
            if let Some(n) = self.nodes.get_mut(entry.node) {
                n.sector = None;
            }
            self.removals.pending.push(entry);
        }
    }

    /// Works through the pending removals as the frame's last step does.
    /// A finished entry is replaced by the last one, which is looked at
    /// next, so the order is not kept:
    ///
    /// - [`RemovalKind::Hide`] gives the node its owner back.
    /// - [`RemovalKind::Delete`] waits for the next frame while the subtree
    ///   carries [`OWNED`]; otherwise the node gets its owner back (an
    ///   owner of `_World` is the original's "World sector object deleted"
    ///   warning, listed in `world_deleted`) and is taken out with its
    ///   subtree, or, while another owner holds it (`refs >= 1`), marked
    ///   [`DELETING`] and put on the free list.
    /// - [`RemovalKind::Free`] takes the node out, or puts a held node on
    ///   the free list.
    ///
    /// Returns the nodes taken out. (The node's own destructor modes are
    /// another unit's; guess: taking the subtree out stands for the
    /// destroy, and the deferred mode only sets [`DELETING`].)
    pub fn process_removals(&mut self) -> Vec<usize> {
        self.process_removals_into(&mut Vec::new())
    }

    fn process_removals_into(&mut self, world_deleted: &mut Vec<usize>) -> Vec<usize> {
        let mut removed = Vec::new();
        let mut at = 0;
        while at < self.removals.pending.len() {
            let entry = self.removals.pending[at];
            let held = self
                .nodes
                .get(entry.node)
                .is_some_and(|node| node.refs >= 1);
            match entry.kind {
                RemovalKind::Hide => {
                    if let Some(n) = self.nodes.get_mut(entry.node) {
                        n.sector = entry.sector;
                    }
                }
                RemovalKind::Delete => {
                    if self.owns_subtree(entry.node) {
                        at += 1;
                        continue;
                    }
                    if let Some(n) = self.nodes.get_mut(entry.node) {
                        n.sector = entry.sector;
                    }
                    if entry.sector.is_none() {
                        world_deleted.push(entry.node);
                    }
                    if held {
                        self.defer_free(entry.node);
                    } else {
                        removed.push(entry.node);
                        self.remove_subtree(entry.node, entry.sector);
                    }
                }
                RemovalKind::Free => {
                    if held {
                        self.pending_free.push(entry.node);
                    } else {
                        removed.push(entry.node);
                        self.remove_subtree(entry.node, entry.sector);
                    }
                }
            }
            self.removals.pending.swap_remove(at);
        }
        removed
    }

    fn defer_free(&mut self, node: usize) {
        if let Some(n) = self.nodes.get_mut(node) {
            n.scene_flags |= DELETING;
        }
        self.pending_free.push(node);
    }

    /// Sets the detail level and its 0.01 scale (the original's item and
    /// character views call this).
    pub fn set_detail(&mut self, detail: i32) {
        self.detail = detail;
        self.detail_scale = (f64::from(detail) * 0.01) as f32;
    }

    /// Registers a renderer and points it at the scene.
    pub fn add_renderer(&mut self, renderer: u32) {
        self.renderers.push(renderer);
    }

    /// Registers a scene plugin (the original's plugin list; the added-nodes
    /// calls are the caller's).
    pub fn add_plugin(&mut self, plugin: u32) {
        self.plugins.push(plugin);
    }

    /// Sets the 6 m cell grid and puts the scene in grid mode, as allocating
    /// the cell arrays does.
    pub fn set_grid(&mut self, width: usize, height: usize) {
        self.grid = (width, height);
        self.grid_mode = width > 0 && height > 0;
    }

    /// Resets the far sort key of `node` and its whole subtree, as the
    /// original's recursive sort-key reset does.
    pub fn reset_sort_keys(&mut self, node: usize) {
        self.nodes[node].sort_key = SORT_KEY_NONE;
        let children = self.nodes[node].children.clone();
        for child in children.into_iter().flatten() {
            self.reset_sort_keys(child);
        }
    }

    /// Clears the part flags a world light's fixture carries
    /// ([`SubMesh::flags`] bit `0x10`, value word zeroed): every part of
    /// the light's parent when it is a mesh, with the parent's own
    /// [`SceneNode::part_word`]; and for each mesh child, its own word and
    /// the **first** part only. (The original takes a mesh child's first
    /// part through the child's geometry link, which the model does not
    /// carry; here it is the child's own first part, a guess.)
    pub fn clear_world_light(&mut self, light: usize) {
        let parent = self.nodes[light]
            .parent
            .filter(|&p| self.nodes[p].kind == NodeKind::Mesh);
        if let Some(parent) = parent {
            clear_parts(&mut self.nodes[parent]);
        }
        let children: Vec<usize> = self.nodes[light]
            .children
            .iter()
            .flatten()
            .copied()
            .filter(|&c| self.nodes[c].kind == NodeKind::Mesh)
            .collect();
        for child in children {
            let node = &mut self.nodes[child];
            if let Some(part) = node.parts.first_mut() {
                part.flags |= 0x10;
                part.value = 0;
            }
            node.part_word = 0;
        }
    }

    /// Takes a node out of its sector's per-material lists, as the original
    /// does when a node leaves the scene: for each of its parts, the first
    /// entry naming the node is replaced by the list's last and the list is
    /// freed when it empties.
    pub fn remove_from_materials(&mut self, sector: usize, node: usize) {
        let materials = self.nodes[node].part_materials.clone();
        for material in materials {
            let Some(list) = self.sectors[sector].materials.get_mut(material) else {
                continue;
            };
            if let Some(at) = list.iter().position(|&(n, _)| n == node) {
                list.swap_remove(at);
            }
        }
    }

    /// Rebuilds a node's per-alias bucket list, as the original's routine
    /// does: `from` is the first alias index to (re)link, zero rebuilds the
    /// whole chain. The entry of every alias from `from` on is reset with
    /// its neighbours, the old chain is joined to the new first entry, and
    /// the last entry ends the list.
    pub fn rebuild_buckets(&mut self, sector: usize, node: usize, from: usize) {
        let count = self.sectors[sector].aliases.len();
        let n = &mut self.nodes[node];
        let mut previous = n.bucket_head;
        if from < 1 {
            previous = -1;
            n.bucket_head = -1;
        }
        // A grown dynamic array is zero-filled.
        n.buckets.resize(count, Bucket::ZERO);
        if count == 0 {
            // The head is left as it was (reset above when `from` is 0).
            return;
        }
        if n.bucket_head < 0 {
            n.bucket_head = 0;
        }
        let head = n.bucket_head as usize;
        n.buckets[head].prev = -1;
        if previous >= 0 {
            let mut at = head;
            while n.buckets[at].next >= 0 {
                at = n.buckets[at].next as usize;
            }
            n.buckets[at].next = from as i32;
        }
        for (i, bucket) in n.buckets.iter_mut().enumerate().skip(from) {
            bucket.prev = i as i32 - 1;
            bucket.next = i as i32 + 1;
            bucket.weight = 0;
            bucket.faces = 0;
            bucket.alias = i;
        }
        n.buckets[count - 1].next = -1;
    }

    /// Sets the light's link records from one weight per sector alias, in
    /// alias order, and chains them in that order with the first as the
    /// head — the state the per-alias link writes leave before
    /// [`Scene::order_links`] runs. A light with no aliases has no head.
    pub fn reset_links(&mut self, light: usize, weights: &[f32]) {
        let n = &mut self.nodes[light];
        n.links.clear();
        n.links.reserve(weights.len());
        for (i, &weight) in weights.iter().enumerate() {
            n.links.push(LightLink {
                weight,
                prev: i as i32 - 1,
                next: if i + 1 == weights.len() {
                    -1
                } else {
                    i as i32 + 1
                },
            });
        }
        n.link_head = if weights.is_empty() { -1 } else { 0 };
    }

    /// Orders the light's link list heaviest first, as the original's
    /// weight sort does: every entry after the head is taken off the chain
    /// and walked back while the entry before it is strictly lighter
    /// (`before < entry`, an ordered compare: an equal weight stops it and
    /// a NaN on either side stops it too), then linked in behind the entry
    /// it stopped at — at the head when it walked past them all. Without
    /// NaN weights the result is the stable order by descending weight
    /// whatever the starting chain; a NaN entry stays behind the entry it
    /// first meets and blocks every later entry from passing it.
    ///
    /// A model whose chain is malformed (an index out of range or a cycle)
    /// is left as it is instead of trapping; the original trusts its list.
    pub fn order_links(&mut self, light: usize) {
        let n = &mut self.nodes[light];
        if n.link_head < 0 || !links_valid(n) {
            return;
        }
        let head = n.link_head as usize;
        let mut next = n.links[head].next;
        while next >= 0 {
            let item = next as usize;
            next = n.links[item].next;
            let before = n.links[item].prev;
            // Take the entry off the chain it is on.
            if next >= 0 {
                n.links[next as usize].prev = before;
            }
            n.links[before as usize].next = next;
            // Walk back while the entry before is strictly lighter (an
            // unordered compare stops the walk).
            let mut at = before;
            loop {
                if !(n.links[at as usize].weight < n.links[item].weight) {
                    break;
                }
                at = n.links[at as usize].prev;
                if at < 0 {
                    break;
                }
            }
            let after = if at < 0 {
                let old_head = n.link_head;
                n.link_head = item as i32;
                old_head
            } else {
                let after = n.links[at as usize].next;
                n.links[at as usize].next = item as i32;
                after
            };
            n.links[item].prev = at;
            n.links[item].next = after;
            if after >= 0 {
                n.links[after as usize].prev = item as i32;
            }
        }
    }

    /// Whether a light's bounding sphere reaches a sector's box, as the
    /// original's alias gate tests it: the sphere test first (touching
    /// counts as reaching, the sign bit of reach² − distance²), then the
    /// squared point-to-box distance against the squared radius, where a
    /// distance that is not greater than the radius reaches (a NaN also
    /// reaches, as the original's unordered compare falls through to it).
    pub fn light_reaches(&self, light: usize, sector: usize) -> bool {
        let sphere = self.nodes[light].world;
        let radius = radius_of(self.nodes[light].half_extents);
        let Some(node) = self.sectors[sector].node else {
            // The scene's own sector has no node box in this model; the
            // light is taken to reach it (a guess: the original's scene
            // sector carries a frame block of its own).
            return true;
        };
        let (box_, box_radius) = self.nodes[node].obb();
        if spheres_separated(sphere, radius, box_.centre, box_radius) {
            return false;
        }
        let distance = point_obb_distance_squared(sphere, &box_);
        !(radius * radius < distance)
    }

    /// Finds the light's alias in `sector`, or makes one, as the original's
    /// alias routine does. A light that carries a shadow cube only reaches a
    /// sector its sphere meets ([`Scene::light_reaches`]). An alias already
    /// copying the light's update stamp is current and gives nothing back
    /// (the spread walk stops there); an alias with an older stamp is
    /// refreshed — its stamp copied and marked moved — and its index is
    /// returned; without an alias one is made, owned by the sector, marked
    /// moved, and it marks [`LIGHT_ADDED`]. A new alias of a light with the
    /// gate bit carries a shadow map whose needed faces start at all six
    /// (a guess: the original copies the light's shadow map into the
    /// alias, which the model has no record of).
    pub fn find_or_create_alias(&mut self, light: usize, sector: usize) -> Option<usize> {
        if self.nodes[light].flags & LIGHT_SHADOW_CUBE != 0 && !self.light_reaches(light, sector) {
            return None;
        }
        let stamp = self.nodes[light].link_stamp;
        let shadow = self.nodes[light].flags & LIGHT_SHADOW_CUBE != 0;
        if let Some(index) = self.sectors[sector]
            .aliases
            .iter()
            .position(|a| a.light == light)
        {
            if self.sectors[sector].aliases[index].stamp == stamp {
                return None;
            }
            // The refresh copies the light's stamp and marks the alias
            // moved; the next node pass resets a moved alias's needed
            // faces ([`Scene::frame`]).
            let alias = &mut self.sectors[sector].aliases[index];
            alias.stamp = stamp;
            alias.moved = true;
            return Some(index);
        }
        self.sectors[sector].aliases.push(LightAlias {
            light,
            owner: Some(sector),
            moved: true,
            mask: 0,
            shadow_faces: shadow.then_some(0x3f),
            shadow_links: 0,
            stamp,
        });
        self.sectors[sector].changes |= LIGHT_ADDED;
        Some(self.sectors[sector].aliases.len() - 1)
    }

    /// Spreads a light's aliases from `sector` along the linked sectors, as
    /// the original's recursive walk does: the sector is marked busy (a
    /// link back to it is skipped, so a loop ends), each linked sector not
    /// busy gets [`Scene::find_or_create_alias`], and the walk goes on into
    /// it only when an alias was made or refreshed there. A current alias
    /// prunes the walk past its sector. The busy marks are cleared on the
    /// way out.
    pub fn spread_aliases(&mut self, light: usize, sector: usize) {
        self.sectors[sector].spreading = true;
        let linked = self.sectors[sector].linked.clone();
        for neighbour in linked {
            if self.sectors[neighbour].spreading {
                continue;
            }
            if self.find_or_create_alias(light, neighbour).is_some() {
                self.spread_aliases(light, neighbour);
            }
        }
        self.sectors[sector].spreading = false;
    }

    /// Finishes the sectors' newly registered meshes: each whose option word
    /// asks for buckets gets them, then the processed count takes the whole
    /// list and the "mesh added" change bit clears.
    pub fn finish_new_meshes(&mut self, sector: usize) {
        let done = self.sectors[sector]
            .meshes_done
            .min(self.sectors[sector].meshes.len());
        let meshes = self.sectors[sector].meshes[done..].to_vec();
        for node in meshes {
            if self.nodes[node].flags & NEEDS_BUCKETS != 0 {
                self.rebuild_buckets(sector, node, 0);
            }
        }
        self.sectors[sector].meshes_done = self.sectors[sector].meshes.len();
        self.sectors[sector].changes &= !MESH_ADDED;
    }

    /// Activates the sector's new or moved lights and auxiliary nodes, as
    /// the original's per-sector light pass does.
    ///
    /// A light whose owner is no longer the sector leaves the list (the
    /// last takes its place); when it is not in the scene either, it also
    /// drops all its aliases (each alias's sector is marked
    /// [`LINKS_CHANGED`] and loses the alias) and is reported in
    /// [`FrameEvents::dropped`] for the scene to remove. A light whose
    /// [`SceneNode::light_state`] is 2 or more only takes a flicker step
    /// when it flickers. Below 2 the light is activated: it takes state 2,
    /// snapshots its position unless [`LIGHT_NO_SNAPSHOT`] (with a zero
    /// height in the [`SNAPSHOT_FLAT`] mode), finds or creates its alias in
    /// the sector (a new one is owned by the sector, starts moved and marks
    /// [`LIGHT_ADDED`]); in grid mode the light's cell is stored and it is
    /// reported in [`FrameEvents::activated`] (an omni light floods the
    /// cells, another kind fills them), otherwise its alias spreads along
    /// the sector's linked sectors ([`Scene::spread_aliases`]) and is
    /// relinked ([`FrameEvents::alias_links`]); then a flickering light takes its
    /// first step and its alias copies are refreshed. Auxiliary nodes below
    /// state 2 take state 2 and snapshot their position.
    pub fn activate_lights(&mut self, sector: usize, events: &mut FrameEvents) {
        let mut at = 0;
        while at < self.sectors[sector].lights.len() {
            let light = self.sectors[sector].lights[at];
            if self.nodes[light].sector != Some(sector) {
                if self.nodes[light].scene_flags & IN_SCENE == 0 {
                    self.drop_light_aliases(light);
                    events.dropped.push(light);
                }
                self.sectors[sector].lights.swap_remove(at);
                continue;
            }
            if self.nodes[light].light_state >= 2 {
                if self.nodes[light].light_flags & LIGHT_FLICKER != 0 {
                    events.flicker.push((light, true));
                }
                at += 1;
                continue;
            }
            let n = &mut self.nodes[light];
            n.light_state = 2;
            if n.light_flags & LIGHT_NO_SNAPSHOT == 0 {
                n.active_frame = n.world;
                if n.flags & SNAPSHOT_MODE == SNAPSHOT_FLAT {
                    n.active_frame.y = 0.0;
                }
            }
            if !self.sectors[sector]
                .aliases
                .iter()
                .any(|a| a.light == light)
            {
                let stamp = self.nodes[light].link_stamp;
                self.sectors[sector].aliases.push(LightAlias {
                    light,
                    owner: Some(sector),
                    moved: true,
                    mask: 0,
                    shadow_faces: None,
                    shadow_links: 0,
                    stamp,
                });
                self.sectors[sector].changes |= LIGHT_ADDED;
            }
            if self.grid_mode {
                store_cell(&mut self.nodes[light], self.grid);
                events.activated.push(light);
            } else {
                // The original spreads the light's alias along the
                // sector's linked sectors before its links are rebuilt.
                self.spread_aliases(light, sector);
                events.alias_links.push(light);
            }
            if self.nodes[light].light_flags & LIGHT_FLICKER != 0 {
                events.flicker.push((light, false));
            }
            events
                .alias_updates
                .extend(self.nodes[light].aliases.iter().copied());
            at += 1;
        }
        let mut at = 0;
        while at < self.sectors[sector].auxiliary.len() {
            let node = self.sectors[sector].auxiliary[at];
            if self.nodes[node].sector != Some(sector) {
                if self.nodes[node].scene_flags & IN_SCENE == 0 {
                    events.dropped.push(node);
                }
                self.sectors[sector].auxiliary.swap_remove(at);
                continue;
            }
            let n = &mut self.nodes[node];
            if n.light_state < 2 {
                n.light_state = 2;
                n.active_frame = n.world;
            }
            at += 1;
        }
    }

    /// A light that left the scene drops its aliases: each alias's sector
    /// is marked [`LINKS_CHANGED`] and the alias loses its owner (the
    /// sector's membership rebuild then removes it). The model keeps a
    /// light's alias copies as nodes ([`SceneNode::aliases`]) and the
    /// sectors' [`LightAlias`] entries; both are released.
    fn drop_light_aliases(&mut self, light: usize) {
        for alias in std::mem::take(&mut self.nodes[light].aliases) {
            if let Some(owner) = self.nodes.get(alias).and_then(|n| n.sector) {
                self.sectors[owner].changes |= LINKS_CHANGED;
            }
            if let Some(node) = self.nodes.get_mut(alias) {
                node.sector = None;
            }
        }
        for index in 0..self.sectors.len() {
            let mut marked = false;
            for alias in &mut self.sectors[index].aliases {
                if alias.light == light && alias.owner.is_some() {
                    alias.owner = None;
                    marked = true;
                }
            }
            if marked {
                self.sectors[index].changes |= LINKS_CHANGED;
            }
        }
    }

    /// Adds a material to the manager's add queue; its id grows the
    /// material count.
    pub fn queue_material_add(&mut self, material: usize) {
        self.material_count = self.material_count.max(material + 1);
        self.material_adds.push(material);
    }

    /// Queues a material for release.
    pub fn queue_material_remove(&mut self, material: usize) {
        self.material_removes.push(material);
    }

    /// Flushes the material manager's queues as the add-nodes call does:
    /// removed materials first, then the added ones, and every sector's
    /// per-material array grows to the material count. Returns the removed
    /// and added ids in call order.
    pub fn flush_materials(&mut self) -> (Vec<usize>, Vec<usize>) {
        let removed = std::mem::take(&mut self.material_removes);
        let added = std::mem::take(&mut self.material_adds);
        for sector in &mut self.sectors {
            if sector.materials.len() < self.material_count {
                sector.materials.resize_with(self.material_count, Vec::new);
            }
        }
        (removed, added)
    }

    /// Flushes only the removed materials, as the frame end does (the add
    /// queue waits for the next add-nodes call).
    pub fn flush_material_removes(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.material_removes)
    }

    /// The size a light's shadow object is created with: the scene's detail
    /// level plus the light's own size word. A light that asks for a shadow
    /// and has none yet gets one of this size before it joins the scene;
    /// the shadow object itself is the renderer's, so the caller makes it.
    pub fn shadow_size(&self, light_size: i32) -> i32 {
        self.detail.wrapping_add(light_size)
    }

    /// Adds a scene-wide light: `added` is the scene's added-light hook (it
    /// runs before the light is listed).
    pub fn add_light(&mut self, light: u32, added: impl FnOnce(u32)) {
        added(light);
        self.lights.push(light);
    }

    /// Removes a scene-wide light. A scene that keeps the cell lights
    /// (`cells`) first takes the light out of every cell, whether or not it
    /// is listed. A listed light then runs the removed-light hook `removed`
    /// and is swap-removed (the last light takes its place). Returns
    /// whether the light was listed.
    pub fn remove_light(
        &mut self,
        light: u32,
        cells: Option<&mut CellLights>,
        removed: impl FnOnce(u32),
    ) -> bool {
        if let Some(cells) = cells {
            cells.remove(light);
        }
        let Some(at) = self.lights.iter().position(|&l| l == light) else {
            return false;
        };
        removed(light);
        self.lights.swap_remove(at);
        true
    }

    /// Adds a timed transition starting at `now` (the game clock, ms). An
    /// entry for the same node and controller exists already, so nothing
    /// happens; otherwise the entry is appended and the controller's state
    /// word flips between 1 and 2 (any other value stays). Returns whether
    /// the entry was added.
    pub fn add_transition(
        &mut self,
        now: u32,
        node: usize,
        controller: u32,
        state: &mut i32,
    ) -> bool {
        if self
            .transitions
            .iter()
            .any(|t| t.node == node && t.controller == controller)
        {
            return false;
        }
        self.transitions.push(Transition {
            node,
            controller,
            start: now,
        });
        match *state {
            1 => *state = 2,
            2 => *state = 1,
            _ => {}
        }
        true
    }

    /// Advances every transition at `now` (the game clock, ms): `evaluate`
    /// gets the controller and the seconds since the entry started (the
    /// 64-bit difference of the two clocks, times 0.001 in double), and
    /// returns whether the controller is still running. Whatever it
    /// returns, the moved word of the node under the entry's sector is
    /// cleared. A finished entry is overwritten by the last one (the
    /// count drops and the same slot is evaluated again); a running one
    /// advances the index. Returns how many entries finished.
    pub fn advance_transitions(
        &mut self,
        now: u32,
        mut evaluate: impl FnMut(u32, f32) -> bool,
    ) -> usize {
        let mut i = 0;
        let mut finished = 0;
        while i < self.transitions.len() {
            let t = self.transitions[i];
            let seconds = ((i64::from(now) - i64::from(t.start)) as f64 * 0.001) as f32;
            let running = evaluate(t.controller, seconds);
            if t.node < self.nodes.len() {
                let top = self.sector_child(t.node);
                self.nodes[top].moved = false;
            }
            if running {
                i += 1;
            } else {
                self.transitions.swap_remove(i);
                finished += 1;
            }
        }
        finished
    }

    /// Queues a node for removal: [`RemovalKind::Delete`] with `detach`,
    /// otherwise [`RemovalKind::Hide`] ([`Scene::queue_removal_as`]).
    pub fn queue_removal(&mut self, node: usize, detach: bool) -> bool {
        let kind = if detach {
            RemovalKind::Delete
        } else {
            RemovalKind::Hide
        };
        self.queue_removal_as(node, kind)
    }

    /// Queues a node for removal with its present owner. The owner's
    /// change word gains the removal bits; a [`RemovalKind::Hide`] node
    /// leaves the scene and its visibility and moved word are cleared,
    /// otherwise the node and its whole subtree are marked deleting, lose
    /// the scene bit and move to the global world. A full ring keeps the
    /// marking and reports false.
    pub fn queue_removal_as(&mut self, node: usize, kind: RemovalKind) -> bool {
        let Some(n) = self.nodes.get(node) else {
            return false;
        };
        let sector = n.sector;
        let queued = self.removals.push(Removal { node, kind, sector });
        if let Some(s) = sector.and_then(|s| self.sectors.get_mut(s)) {
            s.changes |= REMOVAL_CHANGES;
        }
        if kind != RemovalKind::Hide {
            self.mark_deleting(node);
        } else if let Some(n) = self.nodes.get_mut(node) {
            n.scene_flags &= !IN_SCENE;
            n.mask = 0;
            n.moved = false;
        }
        queued
    }

    fn mark_deleting(&mut self, node: usize) {
        let children = if let Some(n) = self.nodes.get_mut(node) {
            n.scene_flags = (n.scene_flags & !IN_SCENE) | DELETING;
            n.mask = 0;
            n.moved = true;
            n.sector = None;
            n.children.clone()
        } else {
            return;
        };
        for child in children.into_iter().flatten() {
            self.mark_deleting(child);
        }
    }

    /// Takes every queued entry (ring and pending) and its subtree out at
    /// once, whatever its kind, without the frame's ownership wait or owner
    /// hand-back (Exarust's own teardown, not the original's frame step:
    /// that is [`Scene::process_removals`]). Returns how many nodes were
    /// marked removed (node slots stay, so indices remain valid).
    pub fn flush_removals(&mut self) -> usize {
        let entries: Vec<Removal> = self.removals.iter().copied().collect();
        self.removals = RemovalQueue::default();
        let mut removed = 0;
        for entry in entries {
            removed += self.remove_subtree(entry.node, entry.sector);
        }
        removed
    }

    fn remove_subtree(&mut self, node: usize, sector: Option<usize>) -> usize {
        let Some(n) = self.nodes.get(node) else {
            return 0;
        };
        let children = n.children.clone();
        let sector = sector.or(n.sector);
        let parent = n.parent;
        let mut count = 0;
        if let Some(s) = sector.and_then(|s| self.sectors.get_mut(s)) {
            s.meshes.retain(|&m| m != node);
            s.lights.retain(|&l| l != node);
            s.auxiliary.retain(|&a| a != node);
        }
        self.geometry.retain(|&g| g != node);
        if let Some(p) = parent.and_then(|p| self.nodes.get_mut(p)) {
            for slot in p.children.iter_mut() {
                if *slot == Some(node) {
                    *slot = None;
                }
            }
        }
        if let Some(n) = self.nodes.get_mut(node) {
            if !n.removed {
                count += 1;
            }
            n.removed = true;
            n.scene_flags &= !IN_SCENE;
            n.mask = 0;
            n.sector = None;
        }
        for child in children.into_iter().flatten() {
            count += self.remove_subtree(child, sector);
        }
        count
    }

    /// The scene-node despawn: a node whose payload flags carry
    /// [`PAYLOAD_CLIMBS`] is replaced by the node under its sector; a payload
    /// of the character class reports its character data, and a node with
    /// a live object-database word reports that slot, for the caller to
    /// free; then the removal is queued with the detach flag.
    pub fn release_node(&mut self, node: usize) -> Released {
        let Some(first) = self.nodes.get(node) else {
            return Released::default();
        };
        let node = if first.payload_flags & PAYLOAD_CLIMBS == PAYLOAD_CLIMBS {
            self.sector_child(node)
        } else {
            node
        };
        let n = &self.nodes[node];
        let character = (n.payload_flags & PAYLOAD_CLASS_MASK == PAYLOAD_CHARACTER)
            .then_some(n.character)
            .flatten();
        let object_slot = (n.object_slot & OBJECT_SLOT_LIVE != 0).then_some(n.object_slot);
        let queued = self.queue_removal(node, true);
        Released {
            node,
            character,
            object_slot,
            queued,
        }
    }

    /// The camera's cut-away detector, as the original's probe walks the
    /// scene: every sector the first view draws (its mask has [`MAIN`]),
    /// every mesh registered in it that carries [`CUT_AWAY_CANDIDATE`],
    /// with the segments moved into the sector's space (less its origin).
    ///
    /// An [`CUT_AWAY_OVERHEAD`] mesh is tested with the segment from
    /// [`CUT_AWAY_OVERHEAD_HEIGHT`] above `focus` straight down to it; every
    /// other kind with the camera's segment from `from` along the unit
    /// `direction` for `length` (the camera unit's `cut_away_segment`). The
    /// segment must meet the mesh's bounding sphere (its box's centre and
    /// the box's radius); a [`CUT_AWAY_SPHERE`] mesh is then marked, and a
    /// [`CUT_AWAY_TESTED`] mesh would be asked its own segment callback
    /// (guess: with no CPU triangles Exarust stops at the sphere). The
    /// overhead mesh's own ray callback is likewise the sphere test. Any
    /// other kind is not marked at all.
    ///
    /// Marking carries to the parent ([`CUT_AWAY_PARENT`]) on every marked
    /// kind, but to the mesh children ([`CUT_AWAY_CHILDREN`]) only from the
    /// overhead kind; clearing a mark carries to the parent and always to
    /// the children, as the original's two branches do. A mesh that was not
    /// marked is left alone, children included.
    ///
    /// Returns how many meshes changed their mark (the original returns its
    /// found byte, always zero by then).
    pub fn detect_cut_away(
        &mut self,
        from: Vec3,
        direction: Vec3,
        length: f32,
        focus: Vec3,
    ) -> usize {
        let segment = direction * length;
        let top = focus + Vec3::Y * CUT_AWAY_OVERHEAD_HEIGHT;
        let down = Vec3::Y * -CUT_AWAY_OVERHEAD_HEIGHT;
        let mut changed = 0;
        for sector in 0..self.sectors.len() {
            if self.sectors[sector].mask & MAIN == 0 {
                continue;
            }
            let origin = self.sectors[sector].origin;
            let (from, top) = (from - origin, top - origin);
            let meshes = self.sectors[sector].meshes.clone();
            self.refresh_stale(&meshes);
            for mesh in meshes {
                let node = &self.nodes[mesh];
                if node.kind != NodeKind::Mesh || node.scene_flags & CUT_AWAY_CANDIDATE == 0 {
                    continue;
                }
                let flags = node.scene_flags;
                let (centre, radius) = (node.world, radius_of(node.half_extents));
                let kind = flags & CUT_AWAY_KIND;
                let cut = if kind == CUT_AWAY_OVERHEAD {
                    segment_sphere(top, down, centre, radius)
                } else {
                    segment_sphere(from, segment, centre, radius)
                        && (kind == CUT_AWAY_TESTED || kind == CUT_AWAY_SPHERE)
                };
                if !cut && !node.cut_away {
                    continue;
                }
                let mut marked = vec![mesh];
                if flags & CUT_AWAY_PARENT != 0
                    && let Some(parent) = node.parent
                {
                    marked.push(parent);
                }
                // The mark cascades to the children from the overhead kind
                // only; a clear cascades from every kind.
                if flags & CUT_AWAY_CHILDREN != 0 && (!cut || kind == CUT_AWAY_OVERHEAD) {
                    marked.extend(
                        node.children
                            .iter()
                            .flatten()
                            .copied()
                            .filter(|&child| self.nodes[child].kind == NodeKind::Mesh),
                    );
                }
                for node in marked {
                    if self.nodes[node].cut_away != cut {
                        self.nodes[node].cut_away = cut;
                        changed += 1;
                    }
                }
            }
        }
        changed
    }

    fn refresh_stale(&mut self, nodes: &[usize]) {
        for &node in nodes {
            if self.nodes[node].stale {
                self.refresh(node);
            }
        }
    }

    /// Clears the sector's aliases' masks and collects the moved ones,
    /// resetting their shadow face words, as both node passes start.
    fn begin_alias_pass(&mut self, sector: usize) -> Vec<usize> {
        let count = self.sectors[sector]
            .aliases_done
            .min(self.sectors[sector].aliases.len());
        let mut moved = Vec::new();
        for at in 0..count {
            let alias = &mut self.sectors[sector].aliases[at];
            alias.mask = 0;
            if alias.moved {
                moved.push(at);
                alias.moved = false;
                if alias.shadow_faces.is_some() {
                    alias.shadow_faces = Some(0x3f);
                }
            }
        }
        moved
    }

    /// ORs the face masks of a node's bucket chain into its aliases' link
    /// masks, as the original's moved-node link refresh does. The walk stops
    /// at the first bucket without a face mask, following the original's
    /// chain test.
    fn link_bucket_faces(&mut self, node: usize) {
        let Some(sector) = self.nodes[node].sector else {
            return;
        };
        let mut at = self.nodes[node].bucket_head;
        while at >= 0 {
            let Some(bucket) = self.nodes[node].buckets.get(at as usize).copied() else {
                return;
            };
            if bucket.faces == 0 {
                return;
            }
            if let Some(alias) = self.sectors[sector].aliases.get_mut(bucket.alias) {
                alias.mask |= bucket.faces;
            }
            at = bucket.next;
        }
    }

    /// The alias shadow test for one node: runs the caller's face test and
    /// stores its faces on the node's bucket for that alias. For a node
    /// whose option word carries [`BOUNDS_SPLIT`] (the bit the original's
    /// test checks on the node) and an alias with a shadow map, the needed
    /// faces gain both the new and the old faces, so a face the node left
    /// is redrawn too. Returns whether the test asks for a relink (guess:
    /// the original's test answers from its own distance comparison, which
    /// the model leaves to the caller; here any face counts).
    fn alias_shadow(
        &mut self,
        node: usize,
        alias: usize,
        faces: &mut dyn FnMut(usize, usize) -> u32,
    ) -> bool {
        let mask = faces(node, alias);
        let mut old = 0;
        if let Some(bucket) = self.nodes[node].buckets.get_mut(alias) {
            old = bucket.faces;
            bucket.faces = mask;
        }
        if self.nodes[node].flags & BOUNDS_SPLIT != 0
            && let Some(sector) = self.nodes[node].sector
            && let Some(entry) = self.sectors[sector].aliases.get_mut(alias)
            && let Some(shadow) = entry.shadow_faces
        {
            entry.shadow_faces = Some(shadow | mask | old);
        }
        mask != 0
    }

    /// The meshes a node pass walks: the sector's processed meshes (the
    /// ones whose buckets were built).
    fn processed_meshes(&self, sector: usize) -> Vec<usize> {
        let s = &self.sectors[sector];
        s.meshes[..s.meshes_done.min(s.meshes.len())].to_vec()
    }

    /// Whether a mesh's state word says its transform was just updated (the
    /// model also keeps the [`SceneNode::moved`] marker), clearing both.
    fn take_moved(&mut self, node: usize) -> bool {
        let n = &mut self.nodes[node];
        let moved = n.moved || n.light_state == 1;
        if moved {
            n.moved = false;
            n.light_state = 2;
        }
        moved
    }

    /// The hidden-sector pass over the processed meshes: every mesh keeps
    /// only [`HIDDEN_KEEPS`] of its mask. A moved mesh stores its cell in
    /// grid mode, runs every alias's shadow test and is relinked when it
    /// needs buckets, and is reported to the moved-node callback; another
    /// mesh that needs buckets runs the moved aliases' tests and is
    /// relinked when one asks. The pass reports nothing moved (the original
    /// returns zero).
    fn hidden_pass(
        &mut self,
        sector: usize,
        faces: &mut dyn FnMut(usize, usize) -> u32,
        events: &mut FrameEvents,
    ) {
        let meshes = self.processed_meshes(sector);
        self.refresh_stale(&meshes);
        let moved_aliases = self.begin_alias_pass(sector);
        let alias_count = self.sectors[sector]
            .aliases_done
            .min(self.sectors[sector].aliases.len());
        let (grid, grid_mode) = (self.grid, self.grid_mode);
        for &id in &meshes {
            self.nodes[id].mask &= HIDDEN_KEEPS;
            let needs = self.nodes[id].flags & NEEDS_BUCKETS != 0;
            if self.take_moved(id) {
                if grid_mode {
                    store_cell(&mut self.nodes[id], grid);
                }
                if needs {
                    for alias in 0..alias_count {
                        self.alias_shadow(id, alias, faces);
                    }
                    events.relinked.push(id);
                }
                events.moved.push(id);
            } else if needs && !moved_aliases.is_empty() {
                let mut relink = false;
                for &alias in &moved_aliases {
                    if self.alias_shadow(id, alias, faces) {
                        relink = true;
                    }
                }
                if relink {
                    events.relinked.push(id);
                }
            }
        }
    }

    /// The visible-sector pass over the processed meshes. Returns whether
    /// the frame changed what the sector shows: an alias moved, a seen mesh
    /// has a moved part, or a seen mesh moved.
    fn visible_pass(
        &mut self,
        index: usize,
        dt_ms: i32,
        faces: &mut dyn FnMut(usize, usize) -> u32,
        events: &mut FrameEvents,
    ) -> bool {
        let meshes = self.processed_meshes(index);
        self.refresh_stale(&meshes);
        let moved_aliases = self.begin_alias_pass(index);
        let alias_count = self.sectors[index]
            .aliases_done
            .min(self.sectors[index].aliases.len());
        let sector = self.sectors[index].clone();
        let (grid, grid_mode) = (self.grid, self.grid_mode);
        let scissor = if sector.through_portal { SCISSOR } else { 0 };
        let mut through_door = false;
        let mut changed = !moved_aliases.is_empty();
        for &id in &meshes {
            let beyond = if sector.visible_portals.is_empty() {
                None
            } else {
                let portals = sector.visible_portals.iter().map(|&at| &sector.portals[at]);
                doorway_portal(&self.nodes[id], portals, grid.0)
                    .and_then(|neighbour| self.sectors.get(neighbour).cloned())
            };
            let inherited_hidden;
            {
                let node = &mut self.nodes[id];
                node.mask = scissor;
                if doorway(beyond.as_ref(), node) {
                    through_door = true;
                } else {
                    let (obb, radius) = node.obb();
                    if sector.views.iter().any(|v| v.intersects_obb(&obb, radius)) {
                        node.mask |= MAIN | SEEN;
                    }
                }
                let inherited = node.inherited & (INHERIT_FADE | INHERIT_HIDDEN);
                if inherited != 0 {
                    let both = INHERIT_FADE | INHERIT_HIDDEN;
                    node.mask = if node.mask & SEEN != 0 && inherited != both {
                        SEEN
                    } else {
                        0
                    };
                }
                inherited_hidden = inherited & INHERIT_HIDDEN != 0;
            }
            if inherited_hidden {
                continue;
            }
            let seen = self.nodes[id].mask & SEEN != 0;
            if seen && self.nodes[id].parts.iter().any(|part| part.moved) {
                events.parts_moved = true;
                changed = true;
            }
            let needs = self.nodes[id].flags & NEEDS_BUCKETS != 0;
            if self.take_moved(id) {
                if grid_mode {
                    store_cell(&mut self.nodes[id], grid);
                }
                if needs {
                    // Every alias is tested; the relink and the face links
                    // follow whatever the tests answered.
                    for alias in 0..alias_count {
                        self.alias_shadow(id, alias, faces);
                    }
                    events.relinked.push(id);
                    if seen {
                        self.link_bucket_faces(id);
                    }
                }
                events.moved.push(id);
                if seen {
                    changed = true;
                }
            } else if needs {
                if !moved_aliases.is_empty() {
                    let mut relink = false;
                    for &alias in &moved_aliases {
                        if self.alias_shadow(id, alias, faces) {
                            relink = true;
                        }
                    }
                    if relink {
                        events.relinked.push(id);
                    }
                }
                if seen {
                    self.link_bucket_faces(id);
                }
            }
            fade_node(&mut self.nodes[id], dt_ms);
        }
        if through_door {
            // A mesh seen through a doorway marks its sector as seen.
            self.sectors[index].mask |= MAIN | SEEN;
        }
        // Every alias's link mask goes into its shadow map's linked faces.
        let count = self.sectors[index]
            .aliases_done
            .min(self.sectors[index].aliases.len());
        for alias in &mut self.sectors[index].aliases[..count] {
            if alias.shadow_faces.is_some() {
                alias.shadow_links |= alias.mask;
            }
        }
        changed
    }
}

/// Whether a node's link chain is safe to walk: the head is in range and
/// following `next` reaches the end without leaving the array or cycling.
/// The original trusts its list; the model checks so a malformed one is
/// left unordered rather than trapping.
fn links_valid(node: &SceneNode) -> bool {
    let len = node.links.len();
    let mut at = node.link_head;
    let mut steps = 0;
    while at >= 0 {
        if at as usize >= len || steps > len {
            return false;
        }
        at = node.links[at as usize].next;
        steps += 1;
    }
    true
}

/// Remaps a node's bucket chain after the membership rebuild moved the
/// sector's processed aliases (`map`: old slot → new slot, negative for a
/// removed slot, as the rebuild builds it), as the original does: the head
/// skips removed entries and takes its new index; each kept entry's `next`
/// skips removed entries and the entry it reaches takes the kept entry's
/// new index as `prev`; the entries of the aliases that moved into removed
/// slots move with them; the array shrinks to `done` entries. A chain index
/// past the map is read as kept in place (the original would read past its
/// map; the model's chains never reach there).
fn remap_buckets(node: &mut SceneNode, map: &[i64], removed: &[usize], done: usize) {
    let mapped = |k: i32| map.get(k as usize).copied().unwrap_or(i64::from(k));
    let b = &mut node.buckets;
    let next_of = |b: &[Bucket], k: i32| b.get(k as usize).map_or(-1, |e| e.next);
    while node.bucket_head >= 0 && mapped(node.bucket_head) < 0 {
        node.bucket_head = next_of(b, node.bucket_head);
    }
    if done < 1 {
        node.bucket_head = -1;
    } else if node.bucket_head >= 0 {
        if let Some(e) = b.get_mut(node.bucket_head as usize) {
            e.prev = -1;
        }
        node.bucket_head = mapped(node.bucket_head) as i32;
    }
    for k in 0..map.len().min(b.len()) {
        if map[k] < 0 {
            continue;
        }
        while b[k].next >= 0 && mapped(b[k].next) < 0 {
            b[k].next = next_of(b, b[k].next);
        }
        let next = b[k].next;
        if next >= 0 {
            if let Some(e) = b.get_mut(next as usize) {
                e.prev = map[k] as i32;
            }
            b[k].next = mapped(next) as i32;
        }
    }
    for &i in removed {
        let from = (-map[i]) as usize;
        if let Some(entry) = b.get(from).copied()
            && let Some(slot) = b.get_mut(i)
        {
            // The entry follows its alias into the slot.
            *slot = Bucket { alias: i, ..entry };
        }
    }
    if done > 0
        && node.bucket_head >= 0
        && let Some(e) = b.get_mut(node.bucket_head as usize)
    {
        e.prev = -1;
    }
    b.resize(done, Bucket::ZERO);
}

/// Stores a node's cell and spill mask; returns whether the cell changed.
fn store_cell(node: &mut SceneNode, grid: (usize, usize)) -> bool {
    let spill = node.cell_spill(grid);
    let changed = node.cell.cell != spill.cell;
    node.cell = spill;
    changed
}

/// The doorway test of the visible pass: a mesh that straddles one of the
/// sector's doorways to an active sector and meets one of that sector's
/// views is drawn in the main view without the scissor. The caller resolved
/// the sector beyond the first doorway the mesh straddles.
fn doorway(beyond: Option<&Sector>, node: &mut SceneNode) -> bool {
    let Some(beyond) = beyond else {
        return false;
    };
    let (obb, radius) = node.obb();
    if beyond.views.iter().any(|v| v.intersects_obb(&obb, radius)) {
        node.mask = (node.mask & !SCISSOR) | MAIN | SEEN;
        true
    } else {
        false
    }
}

/// The sector beyond the first doorway a node straddles: the node's cell
/// (`row · width + column`) is one of the doorway's two cells, its sphere
/// comes within [`DOORWAY_REACH`] of the plane (`20 − r ≤ d`, a NaN
/// passing) and its box reaches at least that far in front of it
/// (`support + d ≥ 20`, a NaN passing).
pub fn doorway_portal<'a>(
    node: &SceneNode,
    portals: impl IntoIterator<Item = &'a Portal>,
    grid_width: usize,
) -> Option<usize> {
    let (x, z) = node.cell.cell?;
    let (Ok(x), Ok(z)) = (usize::try_from(x), usize::try_from(z)) else {
        return None;
    };
    let cell = z.saturating_mul(grid_width).saturating_add(x);
    let (obb, radius) = node.obb();
    for portal in portals {
        if portal.cells[0] != cell && portal.cells[1] != cell {
            continue;
        }
        // The original works in centimetres.
        let distance = portal.distance(node.world) * 100.0;
        if DOORWAY_REACH - radius * 100.0 > distance {
            continue;
        }
        // The box's support along the plane normal: the normal in the box's
        // frame times the half extents carrying its signs.
        let local = obb.local(portal.normal);
        let support = (local.x.abs() * obb.half_extents.x
            + local.y.abs() * obb.half_extents.y
            + local.z.abs() * obb.half_extents.z)
            * 100.0;
        if support + distance < DOORWAY_REACH {
            continue;
        }
        return Some(portal.neighbour);
    }
    None
}

/// The screen rectangle of a set of portal frusta: the first vertex of
/// every face (the clipped opening's corners) projected, starting from the
/// first face's, the bounds rounded to the nearest pixel with ties to even
/// (the processor's default conversion).
pub fn scissor_rect(views: &[ConvexVolume], project: &dyn Fn(Vec3) -> Vec2) -> ScreenRect {
    let mut corners = views
        .iter()
        .flat_map(|view| view.planes.iter())
        .filter_map(|plane| plane.vertices.first().copied());
    let Some(first) = corners.next() else {
        // No vertex at all (the original always has one).
        return ScreenRect {
            min_x: 0,
            min_y: 0,
            max_x: 0,
            max_y: 0,
        };
    };
    let mut min = project(first);
    let mut max = min;
    for vertex in corners {
        let p = project(vertex);
        for axis in 0..2 {
            if p[axis] < min[axis] {
                min[axis] = p[axis];
            } else if p[axis] > max[axis] {
                max[axis] = p[axis];
            }
        }
    }
    ScreenRect {
        min_x: min.x.round_ties_even() as i32,
        min_y: min.y.round_ties_even() as i32,
        max_x: max.x.round_ties_even() as i32,
        max_y: max.y.round_ties_even() as i32,
    }
}

/// One step of a mesh's cut-away fade, as the visible pass runs it for a
/// mesh in a view:
///
/// - not cut away: an amount above 0 drops by `dt · 0.003`; at or below
///   0.001 (double precision, a NaN too) it ends at 0, otherwise the mesh
///   is drawn dithered instead of solid;
/// - cut away: the amount rises by `dt · 0.003`; below the target less
///   [`FADE_EPSILON`] (a NaN too) a solid mesh is drawn dithered; otherwise
///   the amount snaps to the target, and below 0.999 (double precision, a
///   NaN too) a solid mesh is drawn partially, else not at all.
fn fade_node(node: &mut SceneNode, dt_ms: i32) {
    if node.mask & (MAIN | SEEN) == 0 {
        return;
    }
    // The frame delta is read as an unsigned 32-bit count of milliseconds.
    let step = dt_ms as u32 as f32 * FADE_RATE;
    if !node.cut_away {
        if 0.0 < node.fade {
            node.fade -= step;
            if !(f64::from(node.fade) > FADE_DONE) {
                node.fade = 0.0;
            } else {
                // Unlike the cut-away branch, no test of the main bit: a
                // seen-only mesh becomes dithered too.
                node.mask = (node.mask & !MAIN) | DITHERED;
            }
        }
        return;
    }
    node.fade += step;
    if !(node.fade >= node.fade_target - FADE_EPSILON) {
        if node.mask & MAIN != 0 {
            node.mask = (node.mask & !MAIN) | DITHERED;
        }
        return;
    }
    node.fade = node.fade_target;
    if !(f64::from(node.fade) >= FADE_PARTIAL) {
        if node.mask & MAIN != 0 {
            node.mask = (node.mask & !MAIN) | PARTIAL;
        }
    } else {
        node.mask &= !MAIN;
    }
}

/// The unit's scenes and its global default sector, as the original's unit
/// keeps them: a newly created scene joins a global list; releasing a scene
/// that has an owner only lists it for the unit's cleanup hook, which frees
/// the listed scenes not marked [`OWNED`]; the global `_World` sector is the
/// default parent of nodes without a scene (locked against registration).
#[derive(Default)]
pub struct SceneSet {
    /// The global default sector (`_World`).
    pub world: Option<Sector>,
    /// The live scenes; a freed scene's slot is left empty.
    pub scenes: Vec<Option<Scene>>,
    /// Each scene's [`OWNED`] mark: a listed scene with it survives the
    /// cleanup hook.
    pub owned: Vec<bool>,
    /// Scenes released while they had an owner, waiting for the cleanup
    /// hook (in list order; the hook takes them out by swapping in the
    /// last).
    pub released: Vec<usize>,
    /// The exit hook that was installed before the unit's, which the
    /// unit's cleanup runs last.
    cleanup: Option<Box<dyn FnMut() -> bool>>,
}

impl SceneSet {
    /// The unit's initialisation: the `_World` default sector, locked, and
    /// no scenes.
    pub fn new() -> Self {
        let mut world = Sector::new("_World", None);
        world.flags |= LOCKED;
        Self {
            world: Some(world),
            ..Self::default()
        }
    }

    /// Creates a scene, appends it to the global list and returns its id.
    pub fn create(&mut self, grid: (usize, usize)) -> usize {
        let id = self.scenes.len();
        self.scenes.push(Some(Scene::new(grid)));
        self.owned.push(false);
        id
    }

    /// Frees a scene and its nodes. Returns whether the id held a scene.
    pub fn free(&mut self, id: usize) -> bool {
        match self.scenes.get_mut(id) {
            Some(slot) if slot.is_some() => {
                *slot = None;
                true
            }
            _ => false,
        }
    }

    /// Releases a scene as the original does: an ownerless scene is freed
    /// at once; one with an owner is listed for the cleanup hook. Returns
    /// whether the id held a scene.
    pub fn release(&mut self, id: usize, has_owner: bool) -> bool {
        if !has_owner {
            return self.free(id);
        }
        if !self.scenes.get(id).is_some_and(Option::is_some) {
            return false;
        }
        self.released.push(id);
        true
    }

    /// Stores the exit hook that was installed before the unit's, as the
    /// unit's initialisation chains it.
    pub fn install_cleanup(&mut self, previous: Box<dyn FnMut() -> bool>) {
        self.cleanup = Some(previous);
    }

    /// The cleanup hook: each listed scene not marked [`OWNED`] is freed
    /// and leaves the list (the last listed takes its place and is checked
    /// next), then the previous hook runs. Returns the previous hook's
    /// answer, or true without one.
    pub fn cleanup(&mut self) -> bool {
        let mut at = 0;
        while at < self.released.len() {
            let id = self.released[at];
            if self.owned.get(id).copied().unwrap_or(false) {
                at += 1;
            } else {
                self.free(id);
                self.released.swap_remove(at);
            }
        }
        match self.cleanup.as_mut() {
            Some(hook) => hook(),
            None => true,
        }
    }

    /// The unit's finaliser: frees the default sector and the scene list.
    pub fn finish(&mut self) {
        self.world = None;
        self.scenes.clear();
        self.owned.clear();
        self.released.clear();
        self.cleanup = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_view(half: f32) -> ConvexVolume {
        box_view_at(Vec3::ZERO, half)
    }

    fn box_view_at(centre: Vec3, half: f32) -> ConvexVolume {
        use exa_physics::intersect::Plane;
        let plane = |normal: Vec3| Plane::new(normal, -half - normal.dot(centre));
        ConvexVolume {
            planes: vec![
                plane(Vec3::X),
                plane(-Vec3::X),
                plane(Vec3::Y),
                plane(-Vec3::Y),
                plane(Vec3::Z),
                plane(-Vec3::Z),
            ],
        }
    }

    fn no_project(_: Vec3) -> Vec2 {
        Vec2::ZERO
    }

    /// A scene with one room sector (index 1) and a 1 m mesh at (1, 1, 1)
    /// registered in it.
    fn node_scene() -> (Scene, usize, usize) {
        let mut scene = Scene::new((32, 40));
        let sector = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[sector]);
        let node = scene.add_node(NodeKind::Mesh, None);
        scene.set_frame(
            node,
            Vec3::new(1.0, 1.0, 1.0),
            Mat3::IDENTITY,
            Vec3::splat(0.5),
        );
        scene.register(node, 1);
        (scene, 1, node)
    }

    fn child(centre: Vec3, extent: Vec3, split: bool) -> ChildBox {
        ChildBox {
            centre,
            extent,
            radius: radius_of(extent),
            split,
        }
    }

    #[test]
    fn kind_codes_follow_the_class_mask() {
        assert_eq!(NodeKind::from_class(0x11), NodeKind::Sector);
        assert_eq!(NodeKind::from_class(0x3), NodeKind::Geometry);
        assert_eq!(NodeKind::from_class(0x5), NodeKind::Light);
        assert_eq!(NodeKind::from_class(0x9), NodeKind::Mesh);
        assert_eq!(NodeKind::from_class(0x41), NodeKind::Auxiliary);
        assert_eq!(NodeKind::from_class(0x1), NodeKind::Callback);
        assert_eq!(NodeKind::from_class(0x0), NodeKind::Node);
        // A sector code with higher bits is not a sector for the batch.
        assert_eq!(NodeKind::from_class(0x111), NodeKind::Node);
    }

    #[test]
    fn box_centres_come_from_the_low_corner() {
        let b = Bounds::new(Vec3::ZERO, Vec3::new(3.0, 4.0, 12.0));
        assert_eq!(b.radius, 13.0);
        // low + (high − low) · 0.5, not (low + high) · 0.5: with these
        // corners the two differ in the last bit.
        let (low, high) = (1.0f32, 16_777_218.0f32);
        let lerp = low + (high - low) * 0.5;
        let mid = (low + high) * 0.5;
        assert_ne!(lerp, mid);
        let b = Bounds::from_min_max(Vec3::splat(low), Vec3::splat(high));
        assert_eq!(b.centre, Vec3::splat(lerp));
        assert_eq!(b.half_extents, Vec3::splat(high - lerp));
    }

    #[test]
    fn sector_bounds_seed_at_the_first_split_mesh() {
        // A split mesh at x = 5 (1 m extents) seeds the corners at its own
        // centre, so the origin is not inside the first box.
        let split = child(Vec3::new(5.0, 0.0, 0.0), Vec3::ONE, true);
        // The second group carries on from the first box's corners.
        let rest = child(Vec3::new(10.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), false);
        let b = sector_bounds(&[split, rest], Vec3::ZERO).unwrap();
        assert_eq!(b.first.centre, Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(b.first.half_extents, Vec3::ONE);
        assert_eq!(b.world_centre, Vec3::new(5.0, 0.0, 0.0));
        // x runs 4..11, y and z −1..1.
        assert_eq!(b.second.centre, Vec3::new(7.5, 0.0, 0.0));
        assert_eq!(b.second.half_extents, Vec3::new(3.5, 1.0, 1.0));
        // About x = 5 the far side is 6; y and z take 1.
        assert_eq!(b.union_radius, 38.0f32.sqrt());
    }

    #[test]
    fn sector_bounds_without_split_meshes_start_at_the_origin() {
        let rest = child(Vec3::new(10.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), false);
        let b = sector_bounds(&[rest], Vec3::ZERO).unwrap();
        assert_eq!(b.first, Bounds::ZERO);
        assert_eq!(b.second.centre, Vec3::new(5.5, 0.0, 0.0));
        assert_eq!(b.second.half_extents, Vec3::new(5.5, 0.0, 0.0));
        assert_eq!(b.union_radius, 11.0);
        // No meshes: no bounds (the sector keeps its old ones).
        assert_eq!(sector_bounds(&[], Vec3::ZERO), None);
    }

    #[test]
    fn sector_bounds_cap_an_extent_at_the_radius() {
        let wide = ChildBox {
            centre: Vec3::ZERO,
            extent: Vec3::new(2.0, 0.0, 0.0),
            radius: 1.0,
            split: true,
        };
        let b = sector_bounds(&[wide], Vec3::ZERO).unwrap();
        assert_eq!(b.first.half_extents, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(b.first.radius, 1.0);
    }

    #[test]
    fn sector_bounds_measure_the_union_about_the_world_centre() {
        // The corners stay in the sector's space while the centre is moved
        // by the origin: with the sector at x = 100, the low side 4 − 105 is
        // the farther one.
        let split = child(Vec3::new(5.0, 0.0, 0.0), Vec3::ONE, true);
        let b = sector_bounds(&[split], Vec3::new(100.0, 0.0, 0.0)).unwrap();
        assert_eq!(b.world_centre, Vec3::new(105.0, 0.0, 0.0));
        assert_eq!(b.union_radius, (101.0f32 * 101.0 + 1.0 + 1.0).sqrt());
    }

    #[test]
    fn a_rotated_mesh_bounds_by_its_world_extents() {
        let mut scene = Scene::new((32, 40));
        let mesh = scene.add_node(NodeKind::Mesh, None);
        scene.set_frame(
            mesh,
            Vec3::ZERO,
            Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(2.0, 0.5, 0.25),
        );
        scene.nodes[mesh].flags |= BOUNDS_SPLIT;
        scene.register(mesh, 0);
        scene.rebuild_sector_bounds(0);
        // Turned a quarter about Y, the 2 m side lies along Z.
        let half = scene.sectors[0].bounds.first.half_extents;
        assert!((half - Vec3::new(0.25, 0.5, 2.0)).abs().max_element() < 1e-6);
    }

    #[test]
    fn scene_bounds_grow_from_the_scene_sector() {
        let room = SectorBounds {
            first: Bounds::new(Vec3::new(6.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0)),
            world_centre: Vec3::new(6.0, 0.0, 0.0),
            second: Bounds::new(Vec3::new(6.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0)),
            union_radius: 0.0,
        };
        let scene = scene_bounds(SectorBounds::default(), &[(room, Mat3::IDENTITY)]);
        // The scene's own empty box seeds the corners at 0: x runs 0..9.
        assert_eq!(scene.first.centre, Vec3::new(4.5, 0.0, 0.0));
        assert_eq!(scene.first.half_extents, Vec3::new(4.5, 1.0, 1.0));
        assert_eq!(scene.world_centre, scene.first.centre);
        // Equal sides take the high one: 4.5, 1, 1.
        assert_eq!(scene.union_radius, (4.5f32 * 4.5 + 1.0 + 1.0).sqrt());
        // The union corners start from the own second box's centre and half
        // extents taken as corners: a half extent of 20 is a high corner at
        // x = 20, so the far side about 4.5 is 15.5.
        let own = SectorBounds {
            second: Bounds::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(20.0, 0.0, 0.0)),
            ..SectorBounds::default()
        };
        let scene = scene_bounds(own, &[(room, Mat3::IDENTITY)]);
        assert_eq!(scene.union_radius, (15.5f32 * 15.5 + 1.0 + 1.0).sqrt());
        assert_eq!(scene.second, own.second, "the second box is not written");
    }

    #[test]
    fn rebuild_bounds_leaves_the_scene_sector_out_of_the_rooms() {
        let (mut scene, sector, node) = node_scene();
        scene.nodes[node].flags |= BOUNDS_SPLIT;
        scene.sectors[sector].origin = Vec3::new(10.0, 0.0, 0.0);
        scene.rebuild_bounds();
        let room = scene.sectors[sector].bounds;
        assert_eq!(room.first.centre, Vec3::ONE);
        assert_eq!(room.world_centre, Vec3::new(11.0, 1.0, 1.0));
        // The scene box runs from its empty seed at 0 to the room's world
        // box at 10.5..11.5.
        let own = scene.sectors[SCENE_SECTOR].bounds;
        assert_eq!(own.first.min(), Vec3::ZERO);
        assert_eq!(own.first.max(), Vec3::new(11.5, 1.5, 1.5));
    }

    #[test]
    fn cells_round_down_and_spill_at_the_edges() {
        let grid = (32, 40);
        // x = 0 is the low edge of column 16 (0..6 m), so a 25 cm box spills
        // low on both axes.
        let cell = cell_spill(Vec3::ZERO, Vec3::splat(0.25), Mat3::IDENTITY, grid);
        assert_eq!(cell.cell, Some((16, 20)));
        assert_eq!(cell.mask, CELL_MARK | SPILL_LOW | (SPILL_LOW << 1));
        // At the middle of a cell nothing spills.
        let cell = cell_spill(
            Vec3::new(3.0, 0.0, 3.0),
            Vec3::splat(0.25),
            Mat3::IDENTITY,
            grid,
        );
        assert_eq!(cell.cell, Some((16, 20)));
        assert_eq!(cell.mask, CELL_MARK);
        // Near the high edge (6 m), the 601 cm threshold spills high.
        let cell = cell_spill(
            Vec3::new(5.9, 0.0, 3.0),
            Vec3::splat(0.25),
            Mat3::IDENTITY,
            grid,
        );
        assert_eq!(cell.mask, CELL_MARK | SPILL_HIGH);
        // A 4 m box in the middle reaches past both edges: the low side wins.
        let cell = cell_spill(
            Vec3::new(3.0, 0.0, 3.0),
            Vec3::new(4.0, 0.0, 0.0),
            Mat3::IDENTITY,
            grid,
        );
        assert_eq!(cell.mask, CELL_MARK | SPILL_LOW);
        // Outside the grid the inner-cell clamp applies.
        let cell = cell_spill(
            Vec3::new(130.0, 0.0, 130.0),
            Vec3::ZERO,
            Mat3::IDENTITY,
            grid,
        );
        assert_eq!(cell.cell, Some((30, 38)));
        // The cell is one byte: at x = −1599 m the index is −267, and
        // 16 − 267 wraps to column 5.
        let cell = cell_spill(
            Vec3::new(-1599.0, 0.0, 3.0),
            Vec3::ZERO,
            Mat3::IDENTITY,
            grid,
        );
        assert_eq!(cell.cell, Some((5, 20)));
        // A 1 m box turned 45° has a 1 m x extent.
        let rotation = Mat3::from_rotation_y(std::f32::consts::FRAC_PI_4);
        let cell = cell_spill(
            Vec3::new(3.0, 0.0, 3.0),
            Vec3::new(0.70710678, 0.70710678, 0.70710678),
            rotation,
            grid,
        );
        assert_eq!(cell.cell, Some((16, 20)));
        assert_eq!(cell.mask, CELL_MARK);
        // A grid too small for an inner cell has no cell.
        let cell = cell_spill(Vec3::ZERO, Vec3::ONE, Mat3::IDENTITY, (2, 10));
        assert_eq!(cell, CellSpill::MARKED);
    }

    #[test]
    fn the_cell_handler_runs_when_the_cell_changes() {
        let mut scene = Scene::new((32, 40));
        let node = scene.add_node(NodeKind::Mesh, None);
        scene.set_frame(
            node,
            Vec3::new(3.0, 0.0, 3.0),
            Mat3::IDENTITY,
            Vec3::splat(0.1),
        );
        scene.register(node, 0);
        assert!(scene.update_cell(node));
        assert_eq!(scene.nodes[node].cell.mask, CELL_MARK);
        assert!(!scene.update_cell(node));
        // A wider box spills, but the cell is the same: no handler.
        scene.set_frame(
            node,
            Vec3::new(3.0, 0.0, 3.0),
            Mat3::IDENTITY,
            Vec3::splat(3.5),
        );
        assert!(!scene.update_cell(node));
        assert_eq!(scene.nodes[node].cell.mask, CELL_MARK | SPILL_LOW | 0x2);
        scene.set_frame(
            node,
            Vec3::new(9.0, 0.0, 3.0),
            Mat3::IDENTITY,
            Vec3::splat(0.1),
        );
        assert!(scene.update_cell(node));
        assert_eq!(scene.nodes[node].cell.cell, Some((17, 20)));
        // A node under another node gets its cell but no handler.
        let leaf = scene.add_node(NodeKind::Mesh, Some(node));
        assert!(!scene.update_cell(leaf));
        assert_eq!(scene.nodes[leaf].cell.cell, Some((17, 20)));
    }

    #[test]
    fn batch_insertion_appends_sectors_and_registers_the_batch() {
        let mut scene = Scene::new((32, 40));
        let a = scene.add_node(NodeKind::Sector, None);
        let mesh = scene.add_node(NodeKind::Mesh, Some(a));
        let b = scene.add_node(NodeKind::Sector, None);
        assert!(scene.add_batch(&[a, b]));
        assert_eq!(scene.sectors.len(), 3, "the scene's own sector and two");
        assert!(scene.sectors[1].in_scene);
        assert_eq!(scene.sectors[1].node, Some(a));
        assert_eq!(scene.nodes[a].scene_flags & IN_SCENE, IN_SCENE);
        // A sector's children are registered in it.
        assert_eq!(scene.sectors[1].meshes, vec![mesh]);
        assert_eq!(scene.nodes[mesh].sector, Some(1));
        // The first entry that is not a sector registers the whole batch in
        // its owner (the scene's own sector here) and stops.
        let x = scene.add_node(NodeKind::Mesh, None);
        let y = scene.add_node(NodeKind::Mesh, None);
        assert!(!scene.add_batch(&[x, y]));
        assert_eq!(scene.sectors[SCENE_SECTOR].meshes, vec![x, y]);
        // An owned node takes the batch to its owner.
        let z = scene.add_node(NodeKind::Mesh, None);
        scene.nodes[z].sector = Some(2);
        scene.add_batch(&[z]);
        assert_eq!(scene.sectors[2].meshes, vec![z]);
        // A locked sector registers nothing.
        scene.sectors[2].flags |= LOCKED;
        let w = scene.add_node(NodeKind::Mesh, None);
        scene.nodes[w].sector = Some(2);
        scene.add_batch(&[w]);
        assert_eq!(scene.sectors[2].meshes, vec![z]);
    }

    #[test]
    fn registration_sorts_nodes_by_kind() {
        let mut scene = Scene::new((32, 40));
        let sector = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[sector]);
        let root = scene.add_node(NodeKind::Node, None);
        let light = scene.add_node(NodeKind::Light, Some(root));
        let mesh = scene.add_node(NodeKind::Mesh, Some(root));
        let geometry = scene.add_node(NodeKind::Geometry, Some(root));
        let aux = scene.add_node(NodeKind::Auxiliary, Some(root));
        scene.register(root, 1);
        let s = &scene.sectors[1];
        assert_eq!(s.lights, vec![light]);
        assert_eq!(s.meshes, vec![mesh]);
        assert_eq!(s.auxiliary, vec![aux]);
        assert_eq!(scene.geometry, vec![geometry]);
        assert_eq!(s.changes, LIGHT_ADDED | MESH_ADDED);
        // Every node takes the owner; a plain node or geometry is not
        // marked in scene, the listed kinds are, and are prepared.
        for n in [root, light, mesh, geometry, aux] {
            assert_eq!(scene.nodes[n].sector, Some(1));
        }
        assert_eq!(scene.nodes[root].scene_flags & IN_SCENE, 0);
        assert_eq!(scene.nodes[geometry].scene_flags & IN_SCENE, 0);
        for n in [light, mesh, aux] {
            assert_eq!(scene.nodes[n].scene_flags & IN_SCENE, IN_SCENE);
            assert_eq!(scene.nodes[n].flags & PREPARED, PREPARED);
        }
        // Registering again appends again, as the original does.
        scene.register(root, 1);
        assert_eq!(scene.sectors[1].meshes, vec![mesh, mesh]);
        // A world light joins no light list.
        let world = scene.add_node(NodeKind::Light, None);
        scene.nodes[world].flags |= WORLD_LIGHT;
        scene.register(world, 1);
        assert_eq!(scene.sectors[1].lights, vec![light, light]);
        assert_eq!(scene.nodes[world].sector, Some(1));
    }

    #[test]
    fn an_owned_subtree_is_found_from_its_root() {
        let mut scene = Scene::new((32, 40));
        let root = scene.add_node(NodeKind::Node, None);
        let child = scene.add_node(NodeKind::Mesh, Some(root));
        let leaf = scene.add_node(NodeKind::Mesh, Some(child));
        assert!(!scene.owns_subtree(root));
        scene.nodes[leaf].scene_flags |= OWNED;
        assert!(scene.owns_subtree(root));
        assert!(scene.owns_subtree(child));
        assert!(scene.owns_subtree(leaf));
    }

    #[test]
    fn an_alias_leaves_its_light() {
        let mut scene = Scene::new((32, 40));
        let sector = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[sector]);
        let light = scene.add_node(NodeKind::Light, None);
        let a = scene.add_node(NodeKind::Light, None);
        let b = scene.add_node(NodeKind::Light, None);
        let c = scene.add_node(NodeKind::Light, None);
        scene.nodes[light].aliases = vec![a, b, c];
        scene.nodes[a].sector = Some(1);
        scene.sectors[1].changes = 0;
        assert!(scene.remove_alias(light, a));
        // The last alias takes the removed one's place.
        assert_eq!(scene.nodes[light].aliases, vec![c, b]);
        assert_eq!(scene.nodes[a].sector, None);
        // The alias's own sector is marked, not the scene's.
        assert_eq!(scene.sectors[1].changes, LINKS_CHANGED);
        assert_eq!(scene.sectors[SCENE_SECTOR].changes, 0);
        assert!(!scene.remove_alias(light, a), "already gone");
    }

    #[test]
    fn the_sector_child_of_a_deep_node_is_its_top_node() {
        let mut scene = Scene::new((32, 40));
        let sector = scene.add_node(NodeKind::Sector, None);
        let top = scene.add_node(NodeKind::Mesh, Some(sector));
        let middle = scene.add_node(NodeKind::Node, Some(top));
        let leaf = scene.add_node(NodeKind::Node, Some(middle));
        assert_eq!(scene.sector_child(top), top, "a top node is its own");
        assert_eq!(scene.sector_child(leaf), top);
    }

    #[test]
    fn room_states_drive_the_sectors() {
        let mut scene = Scene::new((32, 40));
        for _ in 0..2 {
            let room = scene.add_node(NodeKind::Sector, None);
            scene.add_batch(&[room]);
        }
        scene.set_room_states(&[RoomState::Active, RoomState::Hidden, RoomState::Adjacent]);
        assert_eq!(scene.sectors[1].state, SectorState::Active);
        assert_eq!(scene.sectors[2].state, SectorState::Hidden);
        assert_eq!(scene.sectors[0].state, SectorState::Hidden);
        assert_eq!(
            SectorState::from(RoomState::Adjacent),
            SectorState::Adjacent
        );
    }

    #[test]
    fn moving_a_subtree_keeps_its_place_in_the_world() {
        let (mut scene, sector, node) = node_scene();
        let child = scene.add_node(NodeKind::Mesh, Some(node));
        scene.register(child, sector);
        scene.sectors[sector].origin = Vec3::new(10.0, 0.0, 0.0);
        let other = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[other]);
        scene.sectors[2].origin = Vec3::new(100.0, 0.0, 0.0);
        scene.sectors[sector].changes = 0;
        assert!(scene.move_to_sector(node, 2));
        // The local position gains 10 − 100: the world place (sector origin
        // plus frame) stays at x = 11.
        assert_eq!(scene.nodes[node].position, Vec3::new(-89.0, 1.0, 1.0));
        assert_eq!(
            scene.sectors[2].origin + scene.nodes[node].world,
            Vec3::new(11.0, 1.0, 1.0)
        );
        assert_eq!(scene.nodes[node].parent, Some(other));
        assert_eq!(scene.nodes[node].sector, Some(2));
        assert_eq!(scene.nodes[child].sector, Some(2));
        assert_eq!(scene.sectors[2].meshes, vec![node, child]);
        // The old sector is marked and drops the subtree when rebuilt.
        assert_eq!(scene.sectors[sector].changes, LINKS_CHANGED | MOVED_OUT);
        assert_eq!(scene.sectors[sector].meshes, vec![node, child]);
        scene.update_visibility(16);
        assert!(scene.sectors[sector].meshes.is_empty());
        assert_eq!(scene.sectors[sector].changes, 0);
        // Removing clears the in-scene flag and the membership at once.
        scene.remove(child);
        assert_eq!(scene.nodes[child].sector, None);
        assert_eq!(scene.nodes[child].scene_flags & IN_SCENE, 0);
        assert_eq!(scene.sectors[2].meshes, vec![node]);
    }

    #[test]
    fn validate_children_swaps_the_last_child_into_an_empty_slot() {
        let mut scene = Scene::new((32, 40));
        let root = scene.add_node(NodeKind::Node, None);
        let a = scene.add_node(NodeKind::Mesh, Some(root));
        let b = scene.add_node(NodeKind::Mesh, Some(root));
        let c = scene.add_node(NodeKind::Mesh, Some(root));
        scene.nodes[root].children = vec![Some(a), None, Some(b), Some(c)];
        assert_eq!(scene.validate_children(root), 1);
        assert_eq!(scene.nodes[root].children, vec![Some(a), Some(c), Some(b)]);
        assert!(!scene.nodes[b].stale && !scene.nodes[c].stale);
    }

    #[test]
    fn frames_are_in_the_sector_space() {
        let mut scene = Scene::new((32, 40));
        let root = scene.add_node(NodeKind::Node, None);
        scene.set_frame(
            root,
            Vec3::new(1.0, 0.0, 0.0),
            Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::ONE,
        );
        let child = scene.add_node(NodeKind::Mesh, Some(root));
        scene.set_frame(child, Vec3::new(2.0, 0.0, 0.0), Mat3::IDENTITY, Vec3::ONE);
        // A top node's frame is its local frame, whatever the sector's
        // origin: the origin only places the sector in the world.
        scene.sectors[0].origin = Vec3::new(10.0, 0.0, 0.0);
        scene.register(root, 0);
        scene.refresh(root);
        assert_eq!(scene.nodes[root].world, Vec3::new(1.0, 0.0, 0.0));
        // 90° about Y turns +X into −Z.
        assert!((scene.nodes[child].world - Vec3::new(1.0, 0.0, -2.0)).length() < 1e-5);
        // The update marks the whole subtree moved.
        assert!(scene.nodes[root].moved && scene.nodes[child].moved);
    }

    #[test]
    fn active_sector_keeps_the_views_that_see_its_box() {
        let (mut scene, sector, _) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.sectors[sector].origin = Vec3::new(5.0, 0.0, 0.0);
        scene.rebuild_bounds();
        // The second view sees the sector's world box at x = 5, the first
        // does not: bit 1 only, and its copy is moved into sector space.
        let views = [box_view_at(Vec3::new(-50.0, 0.0, 0.0), 1.0), box_view(10.0)];
        scene.classify(Vec3::ZERO, &views, &no_project);
        assert_eq!(scene.sectors[sector].mask, 0b10);
        assert_eq!(
            scene.sectors[sector].views,
            vec![views[1].relative_to(Vec3::new(5.0, 0.0, 0.0))]
        );
        // The scene's own sector, when not hidden, carries every bit and
        // the main views.
        scene.sectors[SCENE_SECTOR].state = SectorState::Adjacent;
        scene.classify(Vec3::ZERO, &views, &no_project);
        assert_eq!(scene.sectors[SCENE_SECTOR].mask, u32::MAX);
        assert_eq!(scene.sectors[SCENE_SECTOR].views, views.to_vec());
    }

    #[test]
    fn mesh_visibility_needs_a_view() {
        let (mut scene, sector, node) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        assert!(scene.update_visibility(16), "a seen mesh moved");
        assert_eq!(scene.nodes[node].mask, MAIN | SEEN);
        assert!(!scene.update_visibility(16), "nothing moved since");
        // Out of every view: no mask.
        scene.set_frame(
            node,
            Vec3::new(100.0, 0.0, 0.0),
            Mat3::IDENTITY,
            Vec3::splat(0.5),
        );
        assert!(!scene.update_visibility(16), "moved but not seen");
        assert_eq!(scene.nodes[node].mask, 0);
        assert!(!scene.nodes[node].seen());
        // The cell followed the move (grid mode).
        assert_eq!(scene.nodes[node].cell.cell, Some((30, 20)));
    }

    #[test]
    fn the_hidden_pass_keeps_the_top_bit() {
        let (mut scene, sector, node) = node_scene();
        // A hidden sector runs the hidden pass.
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        scene.nodes[node].mask = HIDDEN_KEEPS | MAIN | SEEN;
        scene.update_visibility(16);
        assert_eq!(scene.nodes[node].mask, HIDDEN_KEEPS);
        // So does an active sector no view sees.
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        scene.classify(
            Vec3::ZERO,
            &[box_view_at(Vec3::splat(50.0), 1.0)],
            &no_project,
        );
        assert_eq!(scene.sectors[sector].mask, 0);
        scene.nodes[node].mask = HIDDEN_KEEPS | SEEN;
        scene.update_visibility(16);
        assert_eq!(scene.nodes[node].mask, HIDDEN_KEEPS);
    }

    #[test]
    fn inherited_frame_bits_keep_only_the_seen_bit() {
        let (mut scene, sector, node) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        scene.nodes[node].cut_away = true;
        // Following a fading parent: seen only, and the fade still moves.
        scene.nodes[node].inherited = INHERIT_FADE;
        scene.update_visibility(100);
        assert_eq!(scene.nodes[node].mask, SEEN);
        assert_eq!(scene.nodes[node].draw(), Draw::None);
        assert!((scene.nodes[node].fade - 0.3).abs() < 1e-6);
        // Following a faded-out parent: seen only, and no fade.
        scene.nodes[node].inherited = INHERIT_HIDDEN;
        scene.update_visibility(100);
        assert_eq!(scene.nodes[node].mask, SEEN);
        assert!((scene.nodes[node].fade - 0.3).abs() < 1e-6);
        // Both: not even seen.
        scene.nodes[node].inherited = INHERIT_FADE | INHERIT_HIDDEN;
        scene.update_visibility(100);
        assert_eq!(scene.nodes[node].mask, 0);
    }

    #[test]
    fn the_camera_cuts_away_the_candidate_meshes_between_it_and_the_player() {
        let mut scene = Scene::new((32, 40));
        let sector = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[sector]);
        let sector = 1;
        scene.sectors[sector].origin = Vec3::new(100.0, 0.0, 0.0);
        scene.sectors[sector].mask = MAIN;
        let mesh = |scene: &mut Scene, at: Vec3, flags: u32| {
            let node = scene.add_node(NodeKind::Mesh, None);
            scene.set_frame(node, at, Mat3::IDENTITY, Vec3::splat(1.0));
            scene.register(node, sector);
            scene.nodes[node].scene_flags |= flags;
            node
        };
        let on_line = mesh(&mut scene, Vec3::ZERO, CUT_AWAY_SPHERE);
        let tested = mesh(&mut scene, Vec3::ZERO, CUT_AWAY_TESTED);
        let beside = mesh(&mut scene, Vec3::new(10.0, 0.0, 0.0), CUT_AWAY_SPHERE);
        let unflagged = mesh(&mut scene, Vec3::ZERO, 0);
        let other_kind = mesh(&mut scene, Vec3::ZERO, CUT_AWAY_KIND);
        let overhead = mesh(&mut scene, Vec3::new(0.0, 5.0, 30.0), CUT_AWAY_OVERHEAD);
        // The camera's segment, in world space: the sector's origin moves
        // it to z −50..50 on the sector's axis. The player stands at z 30
        // under the overhead mesh.
        let from = Vec3::new(100.0, 0.0, -50.0);
        let focus = Vec3::new(100.0, 0.0, 30.0);
        assert_eq!(scene.detect_cut_away(from, Vec3::Z, 100.0, focus), 3);
        for (node, cut) in [
            (on_line, true),
            (tested, true),
            (beside, false),
            (unflagged, false),
            (other_kind, false),
            (overhead, true),
        ] {
            assert_eq!(scene.nodes[node].cut_away, cut, "node {node}");
        }
        // Without a change the count stays 0.
        assert_eq!(scene.detect_cut_away(from, Vec3::Z, 100.0, focus), 0);
        // A segment elsewhere clears the marks for the fade back in.
        let away = Vec3::new(100.0, 0.0, -100.0);
        assert_eq!(
            scene.detect_cut_away(Vec3::new(100.0, 0.0, 50.0), Vec3::Z, 10.0, away),
            3
        );
        assert!(!scene.nodes[on_line].cut_away && !scene.nodes[overhead].cut_away);
        // A sector the first view does not draw is not probed.
        scene.sectors[sector].mask = 0;
        scene.nodes[on_line].cut_away = true;
        assert_eq!(scene.detect_cut_away(from, Vec3::Z, 100.0, focus), 0);
        assert!(scene.nodes[on_line].cut_away);
    }

    #[test]
    fn a_cut_away_mark_carries_to_the_parent_and_the_mesh_children() {
        let (mut scene, sector, _) = node_scene();
        scene.sectors[sector].mask = MAIN;
        let group = scene.add_node(NodeKind::Mesh, None);
        scene.set_frame(group, Vec3::ZERO, Mat3::IDENTITY, Vec3::splat(1.0));
        let piece = scene.add_node(NodeKind::Mesh, Some(group));
        scene.set_frame(piece, Vec3::ZERO, Mat3::IDENTITY, Vec3::splat(1.0));
        scene.register(piece, sector);
        scene.nodes[piece].scene_flags |= CUT_AWAY_SPHERE | CUT_AWAY_PARENT | CUT_AWAY_CHILDREN;
        let bit = scene.add_node(NodeKind::Mesh, Some(piece));
        let light = scene.add_node(NodeKind::Light, Some(piece));
        let focus = Vec3::new(0.0, 0.0, 30.0);
        // The sphere kind marks on the camera's segment and carries to the
        // parent, but not to the children: only the overhead kind's mark
        // cascades down.
        assert_eq!(
            scene.detect_cut_away(Vec3::new(0.0, 0.0, -5.0), Vec3::Z, 10.0, focus),
            2
        );
        assert!(scene.nodes[group].cut_away);
        assert!(!scene.nodes[bit].cut_away);
        assert!(!scene.nodes[light].cut_away);
        // Clearing carries to the parent and to the children.
        scene.nodes[bit].cut_away = true;
        assert_eq!(
            scene.detect_cut_away(Vec3::new(0.0, 0.0, 50.0), Vec3::Z, 1.0, focus),
            3
        );
        assert!(!scene.nodes[group].cut_away && !scene.nodes[bit].cut_away);
        // A mesh that was not marked leaves its relatives alone.
        scene.nodes[bit].cut_away = true;
        assert_eq!(
            scene.detect_cut_away(Vec3::new(0.0, 0.0, 50.0), Vec3::Z, 1.0, focus),
            0
        );
        assert!(scene.nodes[bit].cut_away);
    }

    #[test]
    fn an_overhead_mark_carries_to_the_mesh_children() {
        let (mut scene, sector, _) = node_scene();
        scene.sectors[sector].mask = MAIN;
        let roof = scene.add_node(NodeKind::Mesh, None);
        scene.set_frame(
            roof,
            Vec3::new(0.0, 0.0, 30.0),
            Mat3::IDENTITY,
            Vec3::splat(1.0),
        );
        scene.register(roof, sector);
        scene.nodes[roof].scene_flags |= CUT_AWAY_OVERHEAD | CUT_AWAY_CHILDREN;
        let bit = scene.add_node(NodeKind::Mesh, Some(roof));
        let focus = Vec3::new(0.0, 0.0, 30.0);
        // The overhead segment starts ten metres above the focus: its hit
        // marks the roof and cascades to the roof's mesh children.
        assert_eq!(
            scene.detect_cut_away(Vec3::new(0.0, 0.0, -50.0), Vec3::Z, 1.0, focus),
            2
        );
        assert!(scene.nodes[roof].cut_away && scene.nodes[bit].cut_away);
        // A focus elsewhere clears the mark and the children with it.
        let away = Vec3::new(0.0, 0.0, 200.0);
        assert_eq!(
            scene.detect_cut_away(Vec3::new(0.0, 0.0, -50.0), Vec3::Z, 1.0, away),
            2
        );
        assert!(!scene.nodes[roof].cut_away && !scene.nodes[bit].cut_away);
    }

    #[test]
    fn a_fade_moves_at_three_thousandths_a_millisecond() {
        let (mut scene, sector, node) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        scene.nodes[node].cut_away = true;
        scene.nodes[node].fade_target = 1.0;
        scene.update_visibility(100);
        assert!((scene.nodes[node].fade - 0.3).abs() < 1e-6);
        assert_eq!(scene.nodes[node].draw(), Draw::Dithered);
        scene.update_visibility(300);
        // 1.2 passes the target: the amount snaps to 1 and the mesh is not
        // drawn.
        assert_eq!(scene.nodes[node].fade, 1.0);
        assert_eq!(scene.nodes[node].draw(), Draw::None);
        // Back the other way: fading in to zero restores the solid draw.
        scene.nodes[node].cut_away = false;
        scene.update_visibility(400);
        assert_eq!(scene.nodes[node].fade, 0.0);
        assert_eq!(scene.nodes[node].draw(), Draw::Solid);
    }

    #[test]
    fn fade_thresholds_follow_the_original_comparisons() {
        let (mut scene, sector, node) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        scene.update_visibility(0);
        // Fading in: 0.001 as an f32 is above 0.001 in double precision, so
        // the fade does not end yet.
        scene.nodes[node].fade = 0.001;
        scene.update_visibility(0);
        assert_eq!(scene.nodes[node].fade, 0.001);
        assert_eq!(scene.nodes[node].draw(), Draw::Dithered);
        // Fading out: an amount exactly at the target less the epsilon
        // snaps (the original's test is strict).
        scene.nodes[node].cut_away = true;
        scene.nodes[node].fade_target = 1.0;
        scene.nodes[node].fade = 1.0 - FADE_EPSILON;
        scene.update_visibility(0);
        assert_eq!(scene.nodes[node].fade, 1.0);
        assert_eq!(scene.nodes[node].draw(), Draw::None);
    }

    #[test]
    fn a_partial_fade_settles_below_one() {
        let (mut scene, sector, node) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        scene.nodes[node].cut_away = true;
        scene.nodes[node].fade_target = 0.5;
        scene.update_visibility(200);
        assert_eq!(scene.nodes[node].fade, 0.5);
        assert_eq!(scene.nodes[node].draw(), Draw::Partial);
    }

    /// An active sector 1 around the origin and an adjacent sector 2 behind
    /// a 2 m × 3 m doorway at x = 10 that faces the active side (−X).
    fn door_scene(cells: [usize; 2]) -> Scene {
        let mut scene = Scene::new((32, 40));
        for _ in 0..2 {
            let room = scene.add_node(NodeKind::Sector, None);
            scene.add_batch(&[room]);
        }
        scene.sectors[1].state = SectorState::Active;
        scene.sectors[2].state = SectorState::Adjacent;
        let mut portal = Portal::between_cells(
            cells[0],
            cells[1],
            Vec2::new(0.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(2.0, 3.0),
            0.0,
        );
        portal.neighbour = 1;
        scene.add_portal(2, portal);
        scene
    }

    #[test]
    fn a_doorway_faces_the_eye_and_builds_a_scissor() {
        let mut scene = door_scene([100, 101]);
        // A second doorway to the active sector, facing away: it counts but
        // gives no frustum.
        let mut back = Portal::between_cells(
            102,
            103,
            Vec2::new(20.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 3.0),
            0.0,
        );
        back.neighbour = 1;
        scene.add_portal(2, back);
        scene.rebuild_bounds();
        let eye = Vec3::new(-1.0, 1.5, 0.0);
        // The doorway's corners land on half pixels; the eye far to the
        // left, so that projecting it would widen the rectangle.
        let project =
            |p: Vec3| Vec2::new(p.z * 10.0 + 0.5 + (p.x - 10.0) * 100.0, p.y * 10.0 + 0.5);
        // Only the first main view clips the doorway.
        scene.classify(eye, &[box_view(200.0), box_view(200.0)], &project);
        assert_eq!(scene.sectors[2].visible_portals, vec![0, 1]);
        assert_eq!(scene.sectors[2].mask, 1);
        assert!(scene.sectors[2].through_portal);
        assert_eq!(scene.sectors[2].views.len(), 1);
        // ±10.5 and 0.5..30.5 round to even: −10, 10, 0, 30.
        assert_eq!(
            scene.sectors[2].scissor,
            Some(ScreenRect {
                min_x: -10,
                min_y: 0,
                max_x: 10,
                max_y: 30,
            })
        );
        // Off both doorways' fronts nothing is seen, but the doorways
        // still lead to an active sector, so the visible pass runs.
        // (An eye on the plane faces neither doorway.)
        scene.classify(Vec3::new(10.0, 1.5, 5.0), &[box_view(200.0)], &project);
        assert_eq!(scene.sectors[2].mask, 0);
        assert!(!scene.sectors[2].through_portal);
        assert!(scene.sectors[2].views.is_empty());
        assert_eq!(scene.sectors[2].visible_portals, vec![0, 1]);
        let mesh = scene.add_node(NodeKind::Mesh, None);
        scene.set_frame(
            mesh,
            Vec3::new(15.0, 1.0, 0.0),
            Mat3::IDENTITY,
            Vec3::splat(0.4),
        );
        scene.register(mesh, 2);
        scene.nodes[mesh].mask = HIDDEN_KEEPS;
        scene.update_visibility(16);
        assert_eq!(
            scene.nodes[mesh].mask, 0,
            "the visible pass clears the top bit"
        );
        // With the active sector hidden, no doorway counts.
        scene.sectors[1].state = SectorState::Hidden;
        scene.classify(eye, &[box_view(200.0)], &project);
        assert!(scene.sectors[2].visible_portals.is_empty());
        assert_eq!(scene.sectors[2].mask, 0);
    }

    #[test]
    fn a_doorway_mesh_escapes_the_scissor() {
        // The mesh's cell is (17, 20), flat index 20·32 + 17.
        let mut scene = door_scene([657, 658]);
        scene.rebuild_bounds();
        let node = scene.add_node(NodeKind::Mesh, None);
        scene.set_frame(
            node,
            Vec3::new(9.9, 1.0, 0.0),
            Mat3::IDENTITY,
            Vec3::splat(0.4),
        );
        scene.register(node, 2);
        scene.update_cell(node);
        assert_eq!(scene.nodes[node].cell.cell, Some((17, 20)));
        scene.classify(Vec3::new(-1.0, 1.5, 0.0), &[box_view(200.0)], &no_project);
        assert!(scene.sectors[2].scissor.is_some());
        scene.update_visibility(16);
        // 10 cm in front of the plane with a 40 cm half extent: the box
        // reaches 50 cm, so it is tested against the active sector and drawn
        // without the scissor; the sector is marked seen.
        assert_eq!(scene.nodes[node].mask, MAIN | SEEN);
        assert_eq!(scene.sectors[2].mask, 1 | MAIN | SEEN);
        // Two metres behind the plane (its cell is the doorway's other one),
        // it is left to the portal frustum and keeps the scissor.
        scene.set_frame(
            node,
            Vec3::new(12.0, 1.0, 0.0),
            Mat3::IDENTITY,
            Vec3::splat(0.4),
        );
        scene.update_cell(node);
        assert_eq!(scene.nodes[node].cell.cell, Some((18, 20)));
        scene.classify(Vec3::new(-1.0, 1.5, 0.0), &[box_view(200.0)], &no_project);
        scene.update_visibility(16);
        assert_eq!(scene.nodes[node].mask, SCISSOR | MAIN | SEEN);
    }

    #[test]
    fn portal_geometry_from_two_cells() {
        let portal = Portal::between_cells(
            3,
            4,
            Vec2::new(-3.0, 0.0),
            Vec2::new(3.0, 0.0),
            Vec2::new(1.0, 2.5),
            0.0,
        );
        assert_eq!(portal.normal, Vec3::NEG_X);
        assert_eq!(portal.offset, 0.0);
        assert_eq!(portal.cells, [3, 4]);
        // The quad's corners lie on x = 0, 1 m across in z, 2.5 m high.
        for p in portal.quad {
            assert!(p.x.abs() < 1e-6);
        }
        assert_eq!(portal.quad[0].z, 0.5);
        assert_eq!(portal.quad[1].z, -0.5);
        assert_eq!(portal.quad[2].y, 2.5);
        assert!((portal.distance(Vec3::new(-1.0, 0.0, 0.0)) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_sector_destroy_drops_its_lists() {
        let mut sector = Sector::new("room", Some(3));
        sector.meshes.push(4);
        sector.lights.push(5);
        sector.auxiliary.push(6);
        sector.portals.push(Portal::between_cells(
            0,
            1,
            Vec2::ZERO,
            Vec2::X,
            Vec2::new(1.0, 2.0),
            0.0,
        ));
        sector.in_scene = true;
        sector.destroy();
        assert!(sector.meshes.is_empty() && sector.lights.is_empty());
        assert!(sector.auxiliary.is_empty() && sector.portals.is_empty());
        assert!(sector.in_scene && sector.node == Some(3));
    }

    #[test]
    fn the_detail_level_sets_its_scale() {
        let mut scene = Scene::new((0, 0));
        assert_eq!(scene.detail_scale, DEFAULT_DETAIL_SCALE);
        scene.set_detail(300);
        assert_eq!(scene.detail, 300);
        assert_eq!(scene.detail_scale, 3.0);
        scene.set_detail(-50);
        assert_eq!(scene.detail_scale, -0.5);
        assert_eq!(scene.shadow_size(7), -43);
    }

    #[test]
    fn a_transition_flips_the_controller_state_once() {
        let mut scene = Scene::new((0, 0));
        let a = scene.add_node(NodeKind::Auxiliary, None);
        let b = scene.add_node(NodeKind::Auxiliary, None);
        let mut state = 1;
        assert!(scene.add_transition(100, a, 9, &mut state));
        assert_eq!(state, 2);
        // The same pair is refused and leaves the state alone.
        assert!(!scene.add_transition(200, a, 9, &mut state));
        assert_eq!(state, 2);
        // Another node on the same controller flips it back.
        assert!(scene.add_transition(300, b, 9, &mut state));
        assert_eq!(state, 1);
        // A state that is neither 1 nor 2 stays.
        let mut other = 5;
        assert!(scene.add_transition(300, a, 10, &mut other));
        assert_eq!(other, 5);
        assert_eq!(scene.transitions.len(), 3);
        assert_eq!(scene.transitions[1].start, 300);
    }

    #[test]
    fn finished_transitions_are_overwritten_by_the_last_entry() {
        let mut scene = Scene::new((0, 0));
        let nodes: Vec<usize> = (0..3)
            .map(|_| scene.add_node(NodeKind::Auxiliary, None))
            .collect();
        for (i, &node) in nodes.iter().enumerate() {
            let mut state = 0;
            scene.add_transition(1000 * (i as u32 + 1), node, i as u32, &mut state);
            scene.nodes[node].moved = true;
        }
        let mut seen = Vec::new();
        // At 3500 ms the entries are 2.5, 1.5 and 0.5 s old; the first
        // finishes, and the last entry then takes its slot and is
        // evaluated in turn.
        let finished = scene.advance_transitions(3500, |controller, seconds| {
            seen.push((controller, seconds));
            seconds < 1.0
        });
        assert_eq!(finished, 2);
        assert_eq!(seen, [(0, 2.5), (2, 0.5), (1, 1.5)]);
        assert_eq!(scene.transitions.len(), 1);
        assert_eq!(scene.transitions[0].controller, 2);
        assert!(nodes.iter().all(|&n| !scene.nodes[n].moved));
    }

    #[test]
    fn a_transition_age_is_a_signed_difference() {
        let mut scene = Scene::new((0, 0));
        let node = scene.add_node(NodeKind::Auxiliary, None);
        let mut state = 0;
        scene.add_transition(2000, node, 1, &mut state);
        let mut seen = 0.0;
        scene.advance_transitions(500, |_, seconds| {
            seen = seconds;
            true
        });
        assert_eq!(seen, -1.5);
    }

    #[test]
    fn a_transition_clears_the_moved_word_of_the_node_under_the_sector() {
        let mut scene = Scene::new((0, 0));
        let top = scene.add_node(NodeKind::Mesh, None);
        let deep = scene.add_node(NodeKind::Mesh, Some(top));
        scene.nodes[top].moved = true;
        scene.nodes[deep].moved = true;
        let mut state = 0;
        scene.add_transition(0, deep, 1, &mut state);
        scene.advance_transitions(10, |_, _| true);
        assert!(!scene.nodes[top].moved);
        assert!(scene.nodes[deep].moved);
    }

    #[test]
    fn a_removal_marks_the_sector_and_the_subtree() {
        let mut scene = Scene::new((2, 2));
        let sector = 1;
        scene.sectors.push(Sector::new("room", None));
        scene.sectors[sector].in_scene = true;
        let root = scene.add_node(NodeKind::Mesh, None);
        let child = scene.add_node(NodeKind::Mesh, Some(root));
        scene.register(root, sector);
        scene.register(child, sector);
        assert!(scene.queue_removal(root, true));
        assert_eq!(
            scene.sectors[sector].changes & REMOVAL_CHANGES,
            REMOVAL_CHANGES
        );
        assert_eq!(scene.nodes[root].scene_flags & IN_SCENE, 0);
        assert_ne!(scene.nodes[root].scene_flags & DELETING, 0);
        assert_ne!(scene.nodes[child].scene_flags & DELETING, 0);
        assert_eq!(scene.nodes[child].sector, None);
        assert_eq!(scene.flush_removals(), 2);
        assert!(scene.nodes[root].removed && scene.nodes[child].removed);
        assert!(scene.sectors[sector].meshes.is_empty());
        assert!(scene.nodes[root].children.is_empty() || scene.nodes[root].children[0].is_none());
    }

    #[test]
    fn a_full_removal_queue_keeps_marking() {
        let mut scene = Scene::new((0, 0));
        let node = scene.add_node(NodeKind::Mesh, None);
        for _ in 0..(REMOVAL_QUEUE - 1) {
            assert!(scene.queue_removal(node, false));
        }
        assert!(scene.removals.is_full());
        assert!(!scene.queue_removal(node, false));
        assert_eq!(scene.nodes[node].scene_flags & IN_SCENE, 0);
        assert_eq!(scene.nodes[node].mask, 0);
    }

    #[test]
    fn releasing_a_piece_queues_the_node_under_the_sector() {
        let mut scene = Scene::new((0, 0));
        let top = scene.add_node(NodeKind::Mesh, None);
        let piece = scene.add_node(NodeKind::Mesh, Some(top));
        scene.nodes[piece].payload_flags = PAYLOAD_CLIMBS;
        scene.nodes[top].object_slot = 0x4000_0011;
        let released = scene.release_node(piece);
        assert_eq!(released.node, top);
        assert_eq!(released.object_slot, Some(0x4000_0011));
        assert_eq!(released.character, None);
        assert!(released.queued);
        assert_ne!(scene.nodes[top].scene_flags & DELETING, 0);
        // Without the flags the node itself is queued.
        let lone = scene.add_node(NodeKind::Mesh, None);
        let child = scene.add_node(NodeKind::Mesh, Some(lone));
        assert_eq!(scene.release_node(child).node, child);
    }

    #[test]
    fn a_character_payload_reports_its_data_and_a_slot_independently() {
        let mut scene = Scene::new((0, 0));
        let top = scene.add_node(NodeKind::Mesh, None);
        let node = scene.add_node(NodeKind::Mesh, Some(top));
        // A character payload has the climbing bits, so the top node's
        // payload is the one checked.
        scene.nodes[node].payload_flags = PAYLOAD_CHARACTER;
        scene.nodes[node].character = Some(1);
        scene.nodes[top].payload_flags = PAYLOAD_CHARACTER;
        scene.nodes[top].character = Some(42);
        scene.nodes[top].object_slot = 0x0000_0011;
        let released = scene.release_node(node);
        assert_eq!(released.node, top);
        assert_eq!(released.character, Some(42));
        assert_eq!(released.object_slot, None, "the top bits are clear");
    }

    #[test]
    fn scene_lights_add_and_remove_by_swap() {
        let mut scene = Scene::new((0, 0));
        let mut hooks = Vec::new();
        scene.add_light(4, |l| hooks.push(("added", l)));
        scene.add_light(5, |l| hooks.push(("added", l)));
        scene.add_light(6, |l| hooks.push(("added", l)));
        assert!(scene.remove_light(4, None, |l| hooks.push(("removed", l))));
        // The last light takes the freed slot.
        assert_eq!(scene.lights, [6, 5]);
        // An unlisted light runs no hook.
        assert!(!scene.remove_light(9, None, |l| hooks.push(("removed", l))));
        assert_eq!(
            hooks,
            [("added", 4), ("added", 5), ("added", 6), ("removed", 4)]
        );
    }

    #[test]
    fn sort_keys_reset_from_the_root_down() {
        let mut scene = Scene::new((0, 0));
        let root = scene.add_node(NodeKind::Node, None);
        let child = scene.add_node(NodeKind::Mesh, Some(root));
        let leaf = scene.add_node(NodeKind::Mesh, Some(child));
        for node in [root, child, leaf] {
            scene.nodes[node].sort_key = 7.0;
        }
        scene.reset_sort_keys(root);
        for node in [root, child, leaf] {
            assert_eq!(scene.nodes[node].sort_key, -100_000.0);
        }
        assert_eq!(SORT_KEY_NONE, -100_000.0);
    }

    #[test]
    fn a_world_light_clears_its_fixture_parts() {
        let mut scene = Scene::new((0, 0));
        let fixture = scene.add_node(NodeKind::Mesh, None);
        scene.nodes[fixture].part_word = 9;
        scene.nodes[fixture].parts = [SubMesh {
            flags: 1,
            value: 5,
            moved: false,
        }; 2]
            .to_vec();
        let light = scene.add_node(NodeKind::Light, Some(fixture));
        let shade = scene.add_node(NodeKind::Mesh, Some(light));
        scene.nodes[shade].part_word = 3;
        scene.nodes[shade].parts = [SubMesh {
            flags: 0,
            value: 7,
            moved: false,
        }; 3]
            .to_vec();
        scene.clear_world_light(light);
        assert_eq!(scene.nodes[fixture].part_word, 0);
        assert_eq!(scene.nodes[shade].part_word, 0);
        for part in &scene.nodes[fixture].parts {
            assert_eq!(part.flags & 0x10, 0x10);
            assert_eq!(part.value, 0);
        }
        // A mesh child loses only its first part's flags.
        let shade_parts = &scene.nodes[shade].parts;
        assert_eq!((shade_parts[0].flags, shade_parts[0].value), (0x10, 0));
        for part in &shade_parts[1..] {
            assert_eq!((part.flags, part.value), (0, 7));
        }
    }

    #[test]
    fn a_bucket_chain_follows_the_alias_order() {
        let (mut scene, sector, mesh) = node_scene();
        scene.sectors[sector].aliases = vec![
            LightAlias {
                light: mesh,
                moved: false,
                mask: 0,
                shadow_faces: None,
                shadow_links: 0,
                stamp: 0,
                owner: Some(sector),
            };
            3
        ];
        scene.rebuild_buckets(sector, mesh, 0);
        assert_eq!(scene.nodes[mesh].bucket_head, 0);
        let links: Vec<(i32, i32, usize)> = scene.nodes[mesh]
            .buckets
            .iter()
            .map(|b| (b.prev, b.next, b.alias))
            .collect();
        assert_eq!(links, [(-1, 1, 0), (0, 2, 1), (1, -1, 2)]);
        // A new alias links onto the old chain's tail.
        scene.sectors[sector].aliases.push(LightAlias {
            light: mesh,
            moved: false,
            mask: 0,
            shadow_faces: None,
            shadow_links: 0,
            stamp: 0,
            owner: Some(sector),
        });
        scene.rebuild_buckets(sector, mesh, 3);
        assert_eq!(scene.nodes[mesh].buckets.len(), 4);
        assert_eq!(scene.nodes[mesh].buckets[2].next, 3);
        assert_eq!(scene.nodes[mesh].buckets[3].prev, 2);
        assert_eq!(scene.nodes[mesh].buckets[3].next, -1);
    }

    #[test]
    fn a_new_mesh_waits_for_its_bucket_pass() {
        let (mut scene, sector, mesh) = node_scene();
        // Registration leaves the mesh unprocessed.
        assert_eq!(scene.sectors[sector].meshes_done, 0);
        assert_eq!(scene.sectors[sector].changes & MESH_ADDED, MESH_ADDED);
        scene.nodes[mesh].flags |= NEEDS_BUCKETS;
        scene.sectors[sector].aliases = vec![
            LightAlias {
                light: mesh,
                moved: false,
                mask: 0,
                shadow_faces: None,
                shadow_links: 0,
                stamp: 0,
                owner: Some(sector),
            };
            2
        ];
        scene.finish_new_meshes(sector);
        assert_eq!(scene.sectors[sector].meshes_done, 1);
        assert_eq!(scene.sectors[sector].changes & MESH_ADDED, 0);
        assert_eq!(scene.nodes[mesh].buckets.len(), 2);
        assert_eq!(scene.nodes[mesh].bucket_head, 0);
    }

    #[test]
    fn a_moved_light_activates_once_and_joins_an_alias() {
        let (mut scene, sector, _) = node_scene();
        let light = scene.add_node(NodeKind::Light, None);
        scene.set_frame(light, Vec3::new(4.0, 1.0, 0.0), Mat3::IDENTITY, Vec3::ONE);
        scene.refresh(light);
        scene.nodes[light].light_type = 3;
        scene.register(light, sector);
        scene.sectors[sector].changes = 0;
        let mut events = FrameEvents::default();
        scene.activate_lights(sector, &mut events);
        assert_eq!(events.activated, [light]);
        assert_eq!(scene.nodes[light].light_state, 2);
        assert_eq!(scene.nodes[light].active_frame, Vec3::new(4.0, 1.0, 0.0));
        assert_eq!(scene.sectors[sector].aliases.len(), 1);
        assert_eq!(scene.sectors[sector].aliases[0].light, light);
        assert_eq!(scene.sectors[sector].aliases[0].owner, Some(sector));
        assert!(
            scene.sectors[sector].aliases[0].moved,
            "a new alias starts moved"
        );
        assert_eq!(scene.sectors[sector].changes, LIGHT_ADDED);
        // An activated light is not activated again.
        let mut events = FrameEvents::default();
        scene.activate_lights(sector, &mut events);
        assert!(events.activated.is_empty());
        assert_eq!(scene.sectors[sector].aliases.len(), 1);
        // A light that moved to another sector leaves this sector's list.
        scene.nodes[light].sector = Some(SCENE_SECTOR);
        let mut events = FrameEvents::default();
        scene.activate_lights(sector, &mut events);
        assert!(events.dropped.is_empty(), "the light is in the scene");
        assert!(scene.sectors[sector].lights.is_empty());
    }

    fn alias(light: usize, owner: Option<usize>) -> LightAlias {
        LightAlias {
            light,
            owner,
            moved: false,
            mask: 0,
            shadow_faces: None,
            shadow_links: 0,
            stamp: 0,
        }
    }

    #[test]
    fn a_membership_rebuild_fills_a_hole_from_the_end_and_remaps_buckets() {
        let (mut scene, sector, mesh) = node_scene();
        let other = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[other]);
        // A, B, C, D processed; E waiting. B now belongs to sector 2.
        scene.sectors[sector].aliases = vec![
            alias(10, Some(sector)),
            alias(11, Some(2)),
            alias(12, Some(sector)),
            alias(13, Some(sector)),
        ];
        scene.nodes[mesh].flags |= NEEDS_BUCKETS;
        scene.rebuild_buckets(sector, mesh, 0);
        scene.nodes[mesh].buckets[3].faces = 0x5;
        scene.sectors[sector].aliases.push(alias(14, Some(sector)));
        scene.sectors[sector].aliases_done = 4;
        scene.sectors[sector].meshes_done = 1;
        scene.sectors[sector].changes = LINKS_CHANGED;
        scene.sectors[2].changes = 0;
        assert!(!scene.rebuild_membership(sector));
        // The last kept alias (D) takes B's slot; E follows the processed
        // ones.
        let lights: Vec<usize> = scene.sectors[sector]
            .aliases
            .iter()
            .map(|a| a.light)
            .collect();
        assert_eq!(lights, [10, 13, 12, 14]);
        assert_eq!(scene.sectors[sector].aliases_done, 4);
        // B's owner is marked.
        assert_eq!(scene.sectors[2].changes, LINKS_CHANGED);
        // The chain still visits A, C, D, then links E (whose `prev` is its
        // index less one, as the original writes it).
        let b = &scene.nodes[mesh].buckets;
        assert_eq!(scene.nodes[mesh].bucket_head, 0);
        let links: Vec<(i32, i32)> = b.iter().map(|e| (e.prev, e.next)).collect();
        assert_eq!(links, [(-1, 2), (2, 3), (0, 1), (2, -1)]);
        // D's bucket moved with it.
        assert_eq!(b[1].faces, 0x5);
        assert_eq!(b[1].alias, 1);
        assert_eq!(scene.sectors[sector].changes & MEMBERSHIP_CHANGED, 0);
        assert_eq!(scene.sectors[SCENE_SECTOR].changes & LINKS_CHANGED, 0);
    }

    #[test]
    fn a_dropped_mesh_gives_its_faces_to_the_aliases_shadows() {
        let (mut scene, sector, mesh) = node_scene();
        let mut a = alias(10, Some(sector));
        a.shadow_faces = Some(0x2);
        scene.sectors[sector].aliases = vec![a, alias(11, Some(sector))];
        scene.sectors[sector].aliases_done = 2;
        scene.rebuild_buckets(sector, mesh, 0);
        scene.nodes[mesh].buckets[0].faces = 0x9;
        scene.nodes[mesh].buckets[1].faces = 0x4;
        scene.sectors[sector].meshes_done = 1;
        scene.nodes[mesh].sector = Some(SCENE_SECTOR);
        assert!(scene.rebuild_membership(sector));
        assert!(scene.sectors[sector].meshes.is_empty());
        assert_eq!(scene.sectors[sector].meshes_done, 0);
        // Only the alias with a shadow map takes the faces.
        assert_eq!(scene.sectors[sector].aliases[0].shadow_faces, Some(0xb));
        assert_eq!(scene.sectors[sector].aliases[1].shadow_faces, None);
    }

    #[test]
    fn activation_without_a_grid_spreads_the_alias_to_linked_sectors() {
        let mut scene = Scene::new((0, 0));
        let first = scene.add_node(NodeKind::Sector, None);
        let second = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[first, second]);
        scene.sectors[1].linked = vec![2];
        scene.sectors[2].linked = vec![1];
        let light = scene.add_node(NodeKind::Light, None);
        scene.register(light, 1);
        let mut events = FrameEvents::default();
        scene.activate_lights(1, &mut events);
        // The sector's own alias, then the spread one in the linked room,
        // carrying the light's stamp; the walk does not come back.
        assert_eq!(scene.sectors[1].aliases.len(), 1);
        assert_eq!(scene.sectors[2].aliases.len(), 1);
        assert_eq!(scene.sectors[2].aliases[0].light, light);
        assert_eq!(scene.sectors[2].aliases[0].stamp, 1);
        assert!(scene.sectors[2].aliases[0].moved);
        assert_eq!(events.alias_links, [light]);
    }

    #[test]
    fn activation_snapshots_flat_and_relinks_without_a_grid() {
        let mut scene = Scene::new((0, 0));
        let sector = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[sector]);
        let light = scene.add_node(NodeKind::Light, None);
        scene.set_frame(light, Vec3::new(4.0, 2.5, 1.0), Mat3::IDENTITY, Vec3::ONE);
        scene.refresh(light);
        assert_eq!(scene.nodes[light].light_state, 1, "the transform update");
        scene.nodes[light].flags |= SNAPSHOT_FLAT;
        scene.nodes[light].light_flags |= LIGHT_FLICKER;
        scene.register(light, 1);
        let mut events = FrameEvents::default();
        scene.activate_lights(1, &mut events);
        assert_eq!(scene.nodes[light].active_frame, Vec3::new(4.0, 0.0, 1.0));
        assert!(events.activated.is_empty());
        assert_eq!(events.alias_links, [light]);
        assert_eq!(events.flicker, [(light, false)]);
        // A moved light activates again; a flickering active one steps.
        let mut events = FrameEvents::default();
        scene.activate_lights(1, &mut events);
        assert_eq!(events.flicker, [(light, true)]);
        // Without a snapshot the old position stays.
        scene.nodes[light].light_flags |= LIGHT_NO_SNAPSHOT;
        scene.set_frame(light, Vec3::new(9.0, 9.0, 9.0), Mat3::IDENTITY, Vec3::ONE);
        scene.refresh(light);
        let mut events = FrameEvents::default();
        scene.activate_lights(1, &mut events);
        assert_eq!(events.alias_links, [light]);
        assert_eq!(scene.nodes[light].active_frame, Vec3::new(4.0, 0.0, 1.0));
    }

    #[test]
    fn a_light_leaving_the_scene_drops_its_aliases() {
        let (mut scene, sector, _) = node_scene();
        let light = scene.add_node(NodeKind::Light, None);
        scene.register(light, sector);
        let mut events = FrameEvents::default();
        scene.activate_lights(sector, &mut events);
        scene.sectors[sector].changes = 0;
        scene.nodes[light].sector = None;
        scene.nodes[light].scene_flags &= !IN_SCENE;
        let mut events = FrameEvents::default();
        scene.activate_lights(sector, &mut events);
        assert_eq!(events.dropped, [light]);
        assert_eq!(scene.sectors[sector].aliases[0].owner, None);
        assert_eq!(scene.sectors[sector].changes, LINKS_CHANGED);
        assert!(!scene.rebuild_membership(sector));
        assert!(scene.sectors[sector].aliases.is_empty());
    }

    #[test]
    fn a_membership_rebuild_removes_a_mesh_that_left() {
        let (mut scene, sector, mesh) = node_scene();
        scene.sectors[sector].meshes_done = 1;
        scene.nodes[mesh].part_materials = vec![0];
        scene.sectors[sector].materials = vec![vec![(mesh, 0)]];
        scene.nodes[mesh].sector = None;
        scene.nodes[mesh].scene_flags = 0;
        scene.rebuild_membership(sector);
        assert!(scene.sectors[sector].meshes.is_empty());
        assert_eq!(scene.sectors[sector].meshes_done, 0);
        assert!(scene.sectors[sector].materials[0].is_empty());
    }

    #[test]
    fn per_material_lists_follow_registration_and_removal() {
        let (mut scene, sector, mesh) = node_scene();
        scene.nodes[mesh].part_materials = vec![0, 2];
        scene.nodes[mesh].parts = vec![SubMesh::default(); 2];
        scene.nodes[mesh].scene_flags &= !IN_SCENE;
        scene.register(mesh, sector);
        assert_eq!(scene.sectors[sector].materials.len(), 3);
        assert_eq!(scene.sectors[sector].materials[0], [(mesh, 0)]);
        assert_eq!(scene.sectors[sector].materials[2], [(mesh, 1)]);
        // The queue flush grows every sector's array to the material count.
        scene.queue_material_add(5);
        let (removed, added) = scene.flush_materials();
        assert!(removed.is_empty() && added == [5]);
        assert_eq!(scene.sectors[sector].materials.len(), 6);
        scene.queue_material_remove(2);
        assert_eq!(scene.flush_material_removes(), [2]);
        scene.remove_from_materials(sector, mesh);
        assert!(scene.sectors[sector].materials[0].is_empty());
        assert!(scene.sectors[sector].materials[2].is_empty());
    }

    #[test]
    fn a_queued_deletion_waits_for_an_owned_subtree() {
        let mut scene = Scene::new((0, 0));
        let root = scene.add_node(NodeKind::Mesh, None);
        let child = scene.add_node(NodeKind::Mesh, Some(root));
        scene.register(root, 0);
        scene.register(child, 0);
        scene.nodes[child].scene_flags |= OWNED;
        assert!(scene.queue_removal(root, true));
        scene.drain_removals();
        assert!(scene.process_removals().is_empty());
        assert_eq!(scene.removals.len(), 1);
        // The ownership mark is cleared: the next frame frees the subtree.
        scene.nodes[child].scene_flags &= !OWNED;
        assert_eq!(scene.process_removals(), [root]);
        assert!(scene.nodes[root].removed && scene.nodes[child].removed);
        assert!(scene.removals.is_empty());
    }

    #[test]
    fn the_frame_runs_the_light_and_membership_stages() {
        let (mut scene, sector, mesh) = node_scene();
        let light = scene.add_node(NodeKind::Light, None);
        scene.set_frame(light, Vec3::ONE, Mat3::IDENTITY, Vec3::ONE);
        scene.refresh(light);
        scene.register(light, sector);
        scene.nodes[mesh].flags |= NEEDS_BUCKETS;
        scene.queue_material_remove(4);
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        let report = scene.frame(16, Vec3::ZERO, &[box_view(10.0)], &no_project);
        // The light activated, got its alias, and the membership rebuild
        // made the alias and the new mesh visible to the pass.
        assert_eq!(report.activated, [light]);
        assert_eq!(scene.sectors[sector].aliases.len(), 1);
        assert_eq!(scene.sectors[sector].aliases_done, 1);
        assert_eq!(scene.sectors[sector].meshes_done, 1);
        assert_eq!(report.materials_removed, [4]);
        assert_eq!(scene.nodes[light].light_state, 2);
        assert_eq!(scene.nodes[mesh].buckets.len(), 1);
    }

    #[test]
    fn the_scene_set_initialises_cleans_and_finishes() {
        let mut set = SceneSet::new();
        assert_eq!(set.world.as_ref().unwrap().name, "_World");
        assert_ne!(set.world.as_ref().unwrap().flags & LOCKED, 0);
        let a = set.create((4, 4));
        let b = set.create((0, 0));
        let c = set.create((0, 0));
        let d = set.create((0, 0));
        // An ownerless release frees at once; one with an owner waits.
        assert!(set.release(d, false));
        assert!(set.scenes[d].is_none());
        assert!(set.release(a, true) && set.release(b, true));
        set.owned[b] = true;
        assert!(set.scenes[a].is_some());
        assert!(set.cleanup());
        // The listed scene without the mark goes; the marked one and the
        // never-released one stay.
        assert!(set.scenes[a].is_none());
        assert!(set.scenes[b].is_some() && set.scenes[c].is_some());
        assert_eq!(set.released, [b]);
        // The previous hook runs after the sweep.
        set.install_cleanup(Box::new(|| false));
        assert!(!set.cleanup());
        set.finish();
        assert!(set.world.is_none() && set.scenes.is_empty());
    }

    #[test]
    fn an_inherited_seen_only_mesh_fading_in_becomes_dithered() {
        let (mut scene, sector, node) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        scene.nodes[node].inherited = INHERIT_FADE;
        scene.nodes[node].fade = 0.5;
        scene.update_visibility(100);
        // Fading back in without the main bit: the amount moves and the
        // dithered bit is set all the same (the original tests no bit on
        // this branch).
        assert_eq!(scene.nodes[node].mask, SEEN | DITHERED);
        assert!((scene.nodes[node].fade - 0.2).abs() < 1e-6);
    }

    #[test]
    fn a_world_light_registration_clears_its_fixture() {
        let (mut scene, sector, _) = node_scene();
        let fixture = scene.add_node(NodeKind::Mesh, None);
        scene.nodes[fixture].parts = vec![SubMesh {
            flags: 1,
            value: 3,
            moved: false,
        }];
        let light = scene.add_node(NodeKind::Light, Some(fixture));
        scene.nodes[light].flags |= WORLD_LIGHT;
        scene.register(light, sector);
        assert!(scene.sectors[sector].lights.is_empty());
        assert_eq!(scene.nodes[light].sector, Some(sector));
        assert_eq!(scene.nodes[fixture].parts[0].flags & 0x10, 0x10);
        assert_eq!(scene.nodes[fixture].parts[0].value, 0);
    }

    #[test]
    fn a_referenced_removal_goes_to_the_free_list() {
        let mut scene = Scene::new((0, 0));
        let node = scene.add_node(NodeKind::Mesh, None);
        scene.register(node, 0);
        scene.nodes[node].refs = 1;
        assert!(scene.queue_removal(node, true));
        scene.drain_removals();
        assert!(scene.process_removals().is_empty());
        assert_eq!(scene.pending_free, [node]);
        // The deferred node gets its owner back.
        assert_eq!(scene.nodes[node].sector, Some(0));
        assert_ne!(scene.nodes[node].scene_flags & DELETING, 0);
        assert!(scene.removals.is_empty());
        // The frame hands the free list out and clears it.
        let report = scene.frame(0, Vec3::ZERO, &[], &no_project);
        assert_eq!(report.freed, [node]);
        assert!(scene.pending_free.is_empty());
    }

    #[test]
    fn a_seen_mesh_with_a_moved_part_reports_it() {
        let (mut scene, sector, node) = node_scene();
        scene.nodes[node].parts = vec![SubMesh {
            flags: 0,
            value: 0,
            moved: true,
        }];
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        let report = scene.frame(16, Vec3::ZERO, &[box_view(10.0)], &no_project);
        assert!(report.parts_moved);
    }

    #[test]
    fn activation_refreshes_the_lights_alias_copies() {
        let (mut scene, sector, _) = node_scene();
        let light = scene.add_node(NodeKind::Light, None);
        scene.register(light, sector);
        let copy = scene.add_node(NodeKind::Light, None);
        scene.nodes[light].aliases = vec![copy];
        let mut events = FrameEvents::default();
        scene.activate_lights(sector, &mut events);
        assert_eq!(events.alias_updates, [copy]);
    }

    #[test]
    fn a_hidden_node_hangs_from_the_world_for_one_frame() {
        let mut scene = Scene::new((0, 0));
        scene.sectors.push(Sector::new("room", None));
        scene.sectors[1].in_scene = true;
        let node = scene.add_node(NodeKind::Mesh, None);
        scene.register(node, 1);
        assert!(scene.queue_removal_as(node, RemovalKind::Hide));
        assert_eq!(scene.nodes[node].sector, Some(1));
        scene.drain_removals();
        assert_eq!(scene.nodes[node].sector, None);
        assert_eq!(scene.removals.pending().len(), 1);
        assert!(scene.process_removals().is_empty());
        assert_eq!(scene.nodes[node].sector, Some(1));
        assert!(!scene.nodes[node].removed);
        assert!(scene.removals.is_empty());
    }

    #[test]
    fn the_frame_end_takes_the_last_entry_into_a_finished_slot() {
        let mut scene = Scene::new((0, 0));
        let a = scene.add_node(NodeKind::Mesh, None);
        let b = scene.add_node(NodeKind::Mesh, None);
        let c = scene.add_node(NodeKind::Mesh, None);
        let d = scene.add_node(NodeKind::Mesh, None);
        for n in [a, b, c, d] {
            scene.register(n, 0);
        }
        scene.nodes[b].scene_flags |= OWNED;
        for n in [a, b, c, d] {
            assert!(scene.queue_removal(n, true));
        }
        scene.drain_removals();
        // A is done and D takes its slot and is done; C takes it next and is
        // done; B waits.
        assert_eq!(scene.process_removals(), [a, d, c]);
        assert_eq!(scene.removals.pending().len(), 1);
        assert_eq!(scene.removals.pending()[0].node, b);
    }

    #[test]
    fn a_waiting_entry_keeps_its_place_ahead_of_newer_ones() {
        let mut scene = Scene::new((0, 0));
        let a = scene.add_node(NodeKind::Mesh, None);
        let b = scene.add_node(NodeKind::Mesh, None);
        let c = scene.add_node(NodeKind::Mesh, None);
        for n in [a, b, c] {
            scene.register(n, 0);
        }
        scene.nodes[a].scene_flags |= OWNED;
        scene.nodes[b].scene_flags |= OWNED;
        assert!(scene.queue_removal(a, true));
        assert!(scene.queue_removal(b, true));
        scene.drain_removals();
        assert!(scene.process_removals().is_empty());
        assert!(scene.queue_removal(c, true));
        scene.drain_removals();
        let order: Vec<usize> = scene.removals.pending().iter().map(|r| r.node).collect();
        assert_eq!(order, [a, b, c]);
        scene.nodes[a].scene_flags &= !OWNED;
        // A goes, C takes its slot and goes, B waits.
        assert_eq!(scene.process_removals(), [a, c]);
    }

    #[test]
    fn a_free_entry_skips_the_ownership_wait() {
        let mut scene = Scene::new((0, 0));
        let node = scene.add_node(NodeKind::Mesh, None);
        let held = scene.add_node(NodeKind::Mesh, None);
        scene.register(node, 0);
        scene.register(held, 0);
        scene.nodes[node].scene_flags |= OWNED;
        scene.nodes[held].refs = 2;
        assert!(scene.queue_removal_as(node, RemovalKind::Free));
        assert!(scene.queue_removal_as(held, RemovalKind::Free));
        assert_ne!(scene.nodes[node].scene_flags & DELETING, 0);
        let report = scene.frame(0, Vec3::ZERO, &[], &no_project);
        assert_eq!(report.removed, [node]);
        assert_eq!(report.freed, [held]);
        assert!(report.world_deleted.is_empty());
        // A freed node is not handed back to its owner.
        assert_eq!(scene.nodes[held].sector, None);
    }

    #[test]
    fn deleting_a_world_node_is_reported() {
        let mut scene = Scene::new((0, 0));
        let node = scene.add_node(NodeKind::Mesh, None);
        assert_eq!(scene.nodes[node].sector, None);
        assert!(scene.queue_removal(node, true));
        let report = scene.frame(0, Vec3::ZERO, &[], &no_project);
        assert_eq!(report.removed, [node]);
        assert_eq!(report.world_deleted, [node]);
    }

    #[test]
    fn the_ring_holds_255_entries_per_frame() {
        let mut scene = Scene::new((0, 0));
        let node = scene.add_node(NodeKind::Mesh, None);
        for _ in 0..(REMOVAL_QUEUE - 1) {
            assert!(scene.queue_removal(node, false));
        }
        assert!(!scene.queue_removal(node, false));
        scene.drain_removals();
        // Draining frees the ring; the pending list has no cap.
        assert!(scene.queue_removal(node, false));
        assert_eq!(scene.removals.len(), REMOVAL_QUEUE);
    }

    #[test]
    fn a_rotated_room_bounds_the_scene_along_the_world_axes() {
        let room = SectorBounds {
            first: Bounds::new(Vec3::ZERO, Vec3::new(3.0, 1.0, 1.0)),
            world_centre: Vec3::new(10.0, 0.0, 0.0),
            second: Bounds::new(Vec3::ZERO, Vec3::new(3.0, 1.0, 1.0)),
            union_radius: 0.0,
        };
        // A quarter turn about y, exact: the room's x extent lies along
        // world z and its z extent along world x.
        let quarter = Mat3::from_cols(Vec3::new(0.0, 0.0, -1.0), Vec3::Y, Vec3::X);
        let scene = scene_bounds(SectorBounds::default(), &[(room, quarter)]);
        // The first box: x 0…11, y −1…1, z −3…3.
        assert_eq!(scene.first.centre, Vec3::new(5.5, 0.0, 0.0));
        assert_eq!(scene.first.half_extents, Vec3::new(5.5, 1.0, 3.0));
        // The union corners run (−1, −1, −3)…(1, 1, 3) about the unmoved
        // room centre; the farther sides from x = 5.5 are −6.5, 1 and 3.
        assert_eq!(scene.union_radius, 52.25f32.sqrt());
        // Unrotated, the x extent stays 3: the first box runs x 0…13.
        let flat = scene_bounds(SectorBounds::default(), &[(room, Mat3::IDENTITY)]);
        assert_eq!(flat.first.half_extents, Vec3::new(6.5, 1.0, 1.0));
    }

    #[test]
    fn a_light_spreads_aliases_along_the_linked_sectors() {
        // Three rooms in a ring: 1 - 2 - 3 - 1.
        let mut scene = Scene::new((0, 0));
        for _ in 0..3 {
            let sector = scene.add_node(NodeKind::Sector, None);
            scene.add_batch(&[sector]);
        }
        scene.sectors[1].linked = vec![2, 3];
        scene.sectors[2].linked = vec![1, 3];
        scene.sectors[3].linked = vec![1, 2];
        let light = scene.add_node(NodeKind::Light, None);
        scene.register(light, 1);
        scene.nodes[light].link_stamp = 5;
        scene.sectors[1].aliases.clear();
        scene.spread_aliases(light, 1);
        let aliases = |scene: &Scene| {
            let mut of: Vec<(usize, usize, bool, u32)> = scene
                .sectors
                .iter()
                .enumerate()
                .flat_map(|(s, sector)| {
                    sector
                        .aliases
                        .iter()
                        .map(move |a| (s, a.light, a.moved, a.stamp))
                })
                .collect();
            of.sort();
            of
        };
        // The walk reaches every room once and ends on the ring's loop.
        assert_eq!(
            aliases(&scene),
            vec![(2, light, true, 5), (3, light, true, 5)]
        );
        assert!(scene.sectors.iter().all(|s| !s.spreading));
        assert_ne!(scene.sectors[2].changes & LIGHT_ADDED, 0);
        assert_ne!(scene.sectors[3].changes & LIGHT_ADDED, 0);
    }

    #[test]
    fn a_current_alias_prunes_the_walk_and_an_old_one_is_refreshed() {
        let mut scene = Scene::new((0, 0));
        for _ in 0..3 {
            let sector = scene.add_node(NodeKind::Sector, None);
            scene.add_batch(&[sector]);
        }
        scene.sectors[1].linked = vec![2];
        scene.sectors[2].linked = vec![3];
        let light = scene.add_node(NodeKind::Light, None);
        scene.register(light, 1);
        scene.nodes[light].link_stamp = 7;
        // Sector 2 already holds a current alias: the walk stops there and
        // never reaches sector 3.
        scene.sectors[2].aliases.push(LightAlias {
            light,
            owner: Some(2),
            moved: false,
            mask: 0,
            shadow_faces: None,
            shadow_links: 0,
            stamp: 7,
        });
        scene.spread_aliases(light, 1);
        assert!(scene.sectors[3].aliases.is_empty());
        // An older stamp is refreshed in place and the walk goes on.
        scene.nodes[light].link_stamp = 9;
        scene.spread_aliases(light, 1);
        assert_eq!(scene.sectors[2].aliases[0].stamp, 9);
        assert!(scene.sectors[2].aliases[0].moved);
        assert_eq!(scene.sectors[3].aliases.len(), 1);
        assert_eq!(scene.sectors[3].aliases[0].stamp, 9);
    }

    #[test]
    fn a_shadow_cube_light_only_spreads_where_it_reaches() {
        let mut scene = Scene::new((0, 0));
        let near = scene.add_node(NodeKind::Sector, None);
        let far = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[near, far]);
        scene.set_frame(near, Vec3::ZERO, Mat3::IDENTITY, Vec3::splat(5.0));
        scene.set_frame(
            far,
            Vec3::new(12.0, 0.0, 0.0),
            Mat3::IDENTITY,
            Vec3::splat(5.0),
        );
        scene.refresh(near);
        scene.refresh(far);
        scene.sectors[1].linked = vec![2];
        let light = scene.add_node(NodeKind::Light, None);
        scene.register(light, 1);
        scene.set_frame(light, Vec3::ZERO, Mat3::IDENTITY, Vec3::ONE);
        scene.refresh(light);
        // Without a shadow cube the far sector gets an alias whatever the
        // distance.
        scene.spread_aliases(light, 1);
        assert_eq!(scene.sectors[2].aliases.len(), 1);
        assert_eq!(scene.sectors[2].aliases[0].shadow_faces, None);
        // With one, the sphere (radius sqrt 3 at the origin) misses the far
        // box: no alias.
        scene.nodes[light].flags |= LIGHT_SHADOW_CUBE;
        scene.nodes[light].link_stamp = 2;
        scene.sectors[2].aliases.clear();
        scene.spread_aliases(light, 1);
        assert!(scene.sectors[2].aliases.is_empty());
        // Moved to 6 from the box face, the sphere's squared radius (3)
        // covers the squared overhang (1): the alias is made and needs all
        // six faces.
        scene.set_frame(light, Vec3::new(6.0, 0.0, 0.0), Mat3::IDENTITY, Vec3::ONE);
        scene.refresh(light);
        scene.nodes[light].link_stamp = 3;
        scene.spread_aliases(light, 1);
        assert_eq!(scene.sectors[2].aliases.len(), 1);
        assert_eq!(scene.sectors[2].aliases[0].shadow_faces, Some(0x3f));
    }

    #[test]
    fn the_reach_gate_is_the_sphere_then_the_squared_box_distance() {
        let mut scene = Scene::new((0, 0));
        let sector = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[sector]);
        scene.set_frame(sector, Vec3::ZERO, Mat3::IDENTITY, Vec3::splat(2.0));
        scene.refresh(sector);
        let light = scene.add_node(NodeKind::Light, None);
        scene.set_frame(light, Vec3::new(4.0, 0.0, 0.0), Mat3::IDENTITY, Vec3::ONE);
        scene.refresh(light);
        // Centre 4, radius 1, box half 2: distance 2 > 1: out of reach.
        assert!(!scene.light_reaches(light, 1));
        // Centre 3: the sphere touches the box face (touching reaches),
        // and the squared point distance 1 equals the squared radius.
        scene.set_frame(light, Vec3::new(3.0, 0.0, 0.0), Mat3::IDENTITY, Vec3::ONE);
        scene.refresh(light);
        assert!(scene.light_reaches(light, 1));
    }

    #[test]
    fn a_link_list_is_ordered_heaviest_first() {
        let mut scene = Scene::new((0, 0));
        let light = scene.add_node(NodeKind::Light, None);
        scene.reset_links(light, &[1.0, 3.0, 2.0]);
        scene.order_links(light);
        let chain = chain_of(&scene.nodes[light]);
        assert_eq!(chain, vec![1, 2, 0]);
        assert_eq!(scene.nodes[light].link_head, 1);
        // The lightest entry ends the list and the head starts it.
        assert_eq!(scene.nodes[light].links[0].next, -1);
        assert_eq!(scene.nodes[light].links[1].prev, -1);
    }

    #[test]
    fn equal_weights_keep_their_order_and_a_nan_is_heaviest() {
        let mut scene = Scene::new((0, 0));
        let light = scene.add_node(NodeKind::Light, None);
        scene.reset_links(light, &[2.0, 2.0, 1.0]);
        scene.order_links(light);
        assert_eq!(chain_of(&scene.nodes[light]), vec![0, 1, 2]);
        scene.reset_links(light, &[1.0, f32::NAN, 2.0]);
        scene.order_links(light);
        // The NaN entry stops at once behind 1.0, and 2.0 stops at the NaN
        // before it: every compare with a NaN is unordered and ends the
        // walk, so nothing moves.
        assert_eq!(chain_of(&scene.nodes[light]), vec![0, 1, 2]);
        // A NaN further back still blocks: 3.0 passes 1.0 but stops at
        // the NaN.
        scene.reset_links(light, &[f32::NAN, 1.0, 3.0]);
        scene.order_links(light);
        assert_eq!(chain_of(&scene.nodes[light]), vec![0, 2, 1]);
    }

    #[test]
    fn an_empty_and_a_malformed_link_list_are_left_alone() {
        let mut scene = Scene::new((0, 0));
        let light = scene.add_node(NodeKind::Light, None);
        scene.reset_links(light, &[]);
        scene.order_links(light);
        assert_eq!(scene.nodes[light].link_head, -1);
        // A chain that points past the array is not walked.
        scene.reset_links(light, &[1.0, 2.0]);
        scene.nodes[light].links[0].next = 7;
        scene.order_links(light);
        assert_eq!(scene.nodes[light].links[0].next, 7);
    }

    fn chain_of(node: &SceneNode) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut at = node.link_head;
        while at >= 0 {
            chain.push(at as usize);
            at = node.links[at as usize].next;
        }
        chain
    }

    #[test]
    fn a_rotated_active_room_meets_views_along_its_axes() {
        let mut scene = Scene::new((0, 0));
        scene.sectors.push(Sector::new("room", None));
        let room = &mut scene.sectors[1];
        room.in_scene = true;
        room.state = SectorState::Active;
        // A long thin room along world z once turned a quarter about y.
        room.rotation = Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2);
        room.bounds.world_centre = Vec3::ZERO;
        room.bounds.first = Bounds::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 1.0));
        // A view box 1 m wide at z = 8 m: met only by the turned room.
        let view = box_view_at(Vec3::new(0.0, 0.0, 8.0), 0.5);
        scene.classify(Vec3::ZERO, std::slice::from_ref(&view), &no_project);
        assert_eq!(scene.sectors[1].mask, 1);
        scene.sectors[1].rotation = Mat3::IDENTITY;
        scene.classify(Vec3::ZERO, std::slice::from_ref(&view), &no_project);
        assert_eq!(scene.sectors[1].mask, 0);
    }

    #[test]
    fn an_alias_sync_copies_the_light_values_and_marks_the_alias_moved() {
        let mut scene = Scene::new((0, 0));
        let light = scene.add_node(NodeKind::Light, None);
        let alias = scene.add_node(NodeKind::Light, None);
        scene.nodes[light].aliases.push(alias);
        let values = crate::light_source::AliasValues {
            flags: 1,
            colour: [0.5, 0.25, 0.125],
            intensity: 3.0,
            radius: 4.0,
            softness: 0.25,
            spot: Some([1.0, 2.0]),
            ranged: true,
            scale: 0.5,
            shadow_offset: 1,
        };
        assert_eq!(scene.sync_light_aliases(light, &values), 1);
        assert_eq!(scene.nodes[light].alias_values, Some(values));
        assert_eq!(scene.nodes[alias].alias_values, Some(values));
        assert!(scene.nodes[alias].moved);
    }

    #[test]
    fn detach_attach_and_clear_content_follow_the_helpers() {
        let (mut scene, _sector, mesh) = node_scene();
        let child = scene.add_node(NodeKind::Geometry, Some(mesh));
        scene.register(child, 1);
        assert_eq!(scene.sectors[1].meshes, [mesh]);
        assert_eq!(scene.geometry, [child]);
        // Detaching takes the subtree out of the lists but keeps the owner.
        assert_eq!(scene.detach_subtree(mesh), 1);
        assert!(scene.sectors[1].meshes.is_empty());
        assert!(scene.geometry.is_empty());
        assert_eq!(scene.nodes[mesh].sector, Some(1));
        assert_eq!(scene.nodes[child].sector, Some(1));
        assert_eq!(scene.nodes[mesh].scene_flags & IN_SCENE, 0);
        // Attaching under a parent takes the parent's owner and appends.
        let parent = scene.add_node(NodeKind::Node, None);
        scene.nodes[parent].sector = Some(1);
        assert_eq!(scene.attach_nodes(parent, &[mesh]), 1);
        assert_eq!(scene.nodes[mesh].parent, Some(parent));
        assert_eq!(scene.nodes[parent].children, [Some(mesh)]);
        assert_eq!(scene.nodes[mesh].sector, Some(1));
        // Clearing releases every root and its subtree and empties the
        // scene's lists; the scene's own sector stays.
        let removed = scene.clear_content();
        assert!(removed.contains(&mesh) && removed.contains(&child) && removed.contains(&parent));
        assert!(scene.nodes[mesh].removed);
        assert_eq!(scene.sectors.len(), 1);
    }
}
