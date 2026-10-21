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
use exa_physics::intersect::{ConvexVolume, Obb, segment_sphere};

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

/// The class code (over 12 bits) the original's batch insertion tests for a
/// sector.
pub const CLASS_SECTOR: u32 = 0x11;

/// Scene word: the node is queued for deletion (its subtree is marked with
/// it when the queue entry detaches instead of hiding).
pub const DELETING: u32 = 0x8000_0000;
/// Sector change word a queued removal sets: the node left and its light
/// links changed.
pub const REMOVAL_CHANGES: u32 = MOVED_OUT | LINKS_CHANGED;
/// How many removal entries the scene queue holds; the original's ring is
/// 256 entries and refuses the 256th.
pub const REMOVAL_QUEUE: usize = 256;

/// What a released scene node owned, so the caller can free it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NodeOwner {
    #[default]
    None,
    /// A character, by the identity its data carries.
    Character(u64),
    /// An object-database entry, by its slot.
    Object(u32),
}

/// A node waiting in the scene's removal queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Removal {
    pub node: usize,
    /// Whether the entry detaches the subtree (the original's flag) or only
    /// hides the node.
    pub detach: bool,
    /// The sector the node was in when it was queued.
    pub sector: Option<usize>,
}

/// A queued scene node removal: the original's 256-entry ring.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemovalQueue {
    entries: Vec<Removal>,
}

impl RemovalQueue {
    /// Whether the queue refuses another entry (255 entries is full: the
    /// original's write index must not meet its read index).
    pub fn is_full(&self) -> bool {
        self.entries.len() + 1 >= REMOVAL_QUEUE
    }

    /// Queues a removal; false when the queue was full.
    pub fn push(&mut self, removal: Removal) -> bool {
        if self.is_full() {
            return false;
        }
        self.entries.push(removal);
        true
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Removal> + '_ {
        self.entries.iter()
    }
}

/// One timed node transition: a controller evaluates against the time since
/// the entry was added.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub node: usize,
    pub controller: u32,
    /// The scene's game time when the entry was added (ms).
    pub start: u32,
    /// The controller's 1/2 state, which adding the entry flips.
    pub state: i32,
}

/// What releasing a scene node found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Released {
    pub owner: NodeOwner,
    /// The node whose removal was queued (the instance's master when it had
    /// one).
    pub node: usize,
    /// Whether the queue accepted the entry.
    pub queued: bool,
}

/// The scene's constructor defaults: the render fraction of a placed
/// object's cells and the far plane, as the original writes them.
pub const DEFAULT_DETAIL_SCALE: f32 = 0.05;
/// The original's far plane (cm).
pub const DEFAULT_FAR_PLANE: f32 = -100_000.0;
/// The scene time scale's default.
pub const DEFAULT_TIME_SCALE: f32 = 1.0;

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
/// 1. the first box starts as `own`'s first box and grows by every room's
///    first box about its world centre (extents capped at its radius); its
///    world centre is its centre (the scene is not moved);
/// 2. the union corners start as `own`'s second box's centre and half
///    extents taken as corners (the original's arithmetic), grow by every
///    room's second box, and give the union radius about the first centre;
/// 3. `own`'s second box is left as it is.
pub fn scene_bounds(own: SectorBounds, rooms: &[SectorBounds]) -> SectorBounds {
    let mut low = own.first.min();
    let mut high = own.first.max();
    for room in rooms {
        grow(
            &mut low,
            &mut high,
            room.world_centre,
            clamp_to_radius(room.first.half_extents, room.first.radius),
        );
    }
    let first = Bounds::from_min_max(low, high);
    let mut low = own.second.centre;
    let mut high = own.second.half_extents;
    for room in rooms {
        grow(
            &mut low,
            &mut high,
            room.second.centre,
            room.second.half_extents,
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

    /// The signed distance of `point` in front of the doorway plane.
    pub fn distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.offset
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
    /// The instance's master node, when this node shares its geometry.
    pub master: Option<usize>,
    /// What the node's geometry owns (a character's data or a database
    /// slot), released when the node is despawned.
    pub owner: NodeOwner,
    /// The node was flushed from the removal queue.
    pub removed: bool,
    /// A light's aliases (copies of it registered in other sectors).
    pub aliases: Vec<usize>,
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
            master: None,
            owner: NodeOwner::None,
            removed: false,
            aliases: Vec::new(),
            cell: CellSpill::MARKED,
            inherited: 0,
            mask: 0,
            cut_away: false,
            fade: 0.0,
            fade_target: 1.0,
        }
    }

    /// The node's world matrix as three rows (the scene's matrix copy from
    /// the node's world words): the rotation's columns with the frame's
    /// translation in the fourth column.
    pub fn world_matrix(&self) -> [[f32; 4]; 3] {
        let c = self.world_rotation.to_cols_array_2d();
        [
            [c[0][0], c[1][0], c[2][0], self.world.x],
            [c[0][1], c[1][1], c[2][1], self.world.y],
            [c[0][2], c[1][2], c[2][2], self.world.z],
        ]
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
            extent: Vec3::new(
                axis_extent(self.world_rotation, self.half_extents, 0),
                axis_extent(self.world_rotation, self.half_extents, 1),
                axis_extent(self.world_rotation, self.half_extents, 2),
            ),
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
    /// The registered (non-world) lights.
    pub lights: Vec<usize>,
    /// The registered auxiliary nodes.
    pub auxiliary: Vec<usize>,
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
    /// The bounds of its meshes.
    pub bounds: SectorBounds,
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
            lights: Vec::new(),
            auxiliary: Vec::new(),
            portals: Vec::new(),
            visible_portals: Vec::new(),
            state: SectorState::Hidden,
            mask: 0,
            views: Vec::new(),
            scissor: None,
            origin: Vec3::ZERO,
            bounds: SectorBounds::default(),
        }
    }

    /// The sector's destructor: child node, lights, portals and the
    /// material lists are dropped and it leaves the scene. (The original's
    /// slot 0 also releases the two callbacks and the dynarrays.)
    pub fn destroy(&mut self) {
        self.meshes.clear();
        self.lights.clear();
        self.auxiliary.clear();
        self.portals.clear();
        self.visible_portals.clear();
        self.views.clear();
        self.in_scene = false;
        self.node = None;
        self.scissor = None;
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
    /// The game time the scene's clock reads (ms).
    pub time: u32,
    /// The detail level and its 0.01 scale (the original's two words the
    /// item and character views set).
    pub detail: i32,
    pub detail_scale: f32,
    /// The timed node transitions.
    pub transitions: Vec<Transition>,
    /// The controller states, by controller id.
    pub controller_states: Vec<(u32, i32)>,
    /// Nodes queued for removal.
    pub removals: RemovalQueue,
    /// The renderers registered with the scene.
    pub renderers: Vec<u32>,
    /// The scene-wide lights.
    pub lights: Vec<u32>,
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
            time: 0,
            detail: 0,
            detail_scale: DEFAULT_DETAIL_SCALE,
            transitions: Vec::new(),
            controller_states: Vec::new(),
            removals: RemovalQueue::default(),
            renderers: Vec::new(),
            lights: Vec::new(),
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
        if let Some(sector) = self.nodes[alias].sector {
            self.sectors[sector].changes |= LINKS_CHANGED;
        }
        self.nodes[alias].sector = None;
        true
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
            NodeKind::Mesh => {
                self.sectors[sector].meshes.push(node);
                let n = &mut self.nodes[node];
                if n.scene_flags & IN_SCENE == 0 {
                    n.scene_flags |= IN_SCENE;
                    n.flags |= PREPARED;
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
        let rooms: Vec<SectorBounds> = self.sectors[1..].iter().map(|s| s.bounds).collect();
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
                    let facing = (portal.normal.dot(eye) + portal.offset) as f64;
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
                }
            } else {
                sector.visible_portals.clear();
                let bounds = sector.bounds;
                let obb = Obb::axis_aligned(bounds.world_centre, bounds.first.half_extents);
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

    /// Drops the meshes and lights a sector no longer owns, keeping the
    /// others in order, and clears its membership change bits (the list
    /// part of the original's membership rebuild; draw buckets and light
    /// aliases are render-side).
    pub fn rebuild_membership(&mut self, sector: usize) {
        let nodes = &self.nodes;
        let s = &mut self.sectors[sector];
        s.meshes.retain(|&m| nodes[m].sector == Some(sector));
        s.lights.retain(|&l| nodes[l].sector == Some(sector));
        s.changes &= !MEMBERSHIP_CHANGED;
    }

    /// Runs the second half of the per-frame scene update: sectors whose
    /// membership changed are rebuilt, new meshes' draw buckets are taken
    /// as done, and each sector runs the hidden pass (no view bit and no
    /// doorway to an active sector) or the visible pass, moving cut-away
    /// fades by `dt_ms`. Returns whether a seen mesh moved this frame (the
    /// original also reports moved lights and part callbacks, which are
    /// render-side).
    pub fn update_visibility(&mut self, dt_ms: f32) -> bool {
        for sector in 0..self.sectors.len() {
            if self.sectors[sector].changes & MEMBERSHIP_CHANGED != 0 {
                self.rebuild_membership(sector);
            }
        }
        for sector in &mut self.sectors {
            sector.changes &= !MESH_ADDED;
        }
        let mut moved_seen = false;
        for sector in 0..self.sectors.len() {
            let s = &self.sectors[sector];
            if s.mask == 0 && s.visible_portals.is_empty() {
                self.hidden_pass(sector);
            } else if self.visible_pass(sector, dt_ms) {
                moved_seen = true;
            }
        }
        moved_seen
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
    /// [`CUT_AWAY_TESTED`] or overhead mesh would be asked its own segment
    /// or ray callback (guess: with no CPU triangles Exarust stops at the
    /// sphere). Any other kind, or a miss, clears a marked mesh. Marking or
    /// clearing carries to the parent ([`CUT_AWAY_PARENT`]) and to the mesh
    /// children ([`CUT_AWAY_CHILDREN`]); a mesh that was not marked is left
    /// alone, children included.
    ///
    /// Returns how many meshes changed their mark (the original returns
    /// nothing useful).
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

    /// Registers a scene-wide light (the shadow object is the caller's).
    pub fn add_light(&mut self, light: u32) {
        self.lights.push(light);
    }

    /// Removes a scene-wide light, swapping the last into its place and
    /// calling `removed` for the removal hook. The cell lists are
    /// [`crate::light_flood::CellLights::remove`]'s.
    pub fn remove_light(&mut self, light: u32, mut removed: impl FnMut(u32)) -> bool {
        let Some(at) = self.lights.iter().position(|&l| l == light) else {
            return false;
        };
        removed(light);
        self.lights.swap_remove(at);
        true
    }

    /// Adds a timed transition. An entry for the same node and controller
    /// exists already, so the call does nothing; otherwise the controller's
    /// 1/2 state flips and the entry starts at the scene's clock.
    pub fn add_transition(&mut self, node: usize, controller: u32) -> bool {
        if self
            .transitions
            .iter()
            .any(|t| t.node == node && t.controller == controller)
        {
            return false;
        }
        let state = match self
            .controller_states
            .iter_mut()
            .find(|(id, _)| *id == controller)
        {
            Some((_, state)) => {
                *state = if *state == 1 { 2 } else { 1 };
                *state
            }
            None => {
                self.controller_states.push((controller, 1));
                1
            }
        };
        self.transitions.push(Transition {
            node,
            controller,
            start: self.time,
            state,
        });
        true
    }

    /// Advances every transition: `evaluate` gets the controller and the
    /// seconds since its start, and returns whether it is still running.
    /// A finished entry is swap-removed (the original's count decrement),
    /// and its node is marked unchanged; a running one advances the index.
    pub fn advance_transitions(&mut self, mut evaluate: impl FnMut(u32, f32) -> bool) -> usize {
        let now = self.time;
        let mut i = 0;
        let mut finished = 0;
        while i < self.transitions.len() {
            let t = self.transitions[i];
            let seconds = (f64::from(now.wrapping_sub(t.start)) * 0.001) as f32;
            let running = evaluate(t.controller, seconds);
            if let Some(node) = self.nodes.get_mut(t.node) {
                node.moved = false;
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

    /// Queues a node for removal. The sector's change word gains the
    /// removal bits; with `detach` false the node leaves the scene and its
    /// visibility and moved word are cleared, otherwise the node and its
    /// whole subtree are marked deleting, lose the scene bit and move to
    /// the global world. A full queue keeps the marking and reports false.
    pub fn queue_removal(&mut self, node: usize, detach: bool) -> bool {
        let Some(n) = self.nodes.get(node) else {
            return false;
        };
        let sector = n.sector;
        let queued = self.removals.push(Removal {
            node,
            detach,
            sector,
        });
        if let Some(s) = sector.and_then(|s| self.sectors.get_mut(s)) {
            s.changes |= REMOVAL_CHANGES;
        }
        if detach {
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

    /// Flushes the removal queue: each entry and its subtree is taken out
    /// of its sector's lists, the geometry list and its parent's slots, and
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

    /// The original's scene-node despawn: an instance is replaced by its
    /// master, the owner is reported (character data or a database slot),
    /// and the node's removal is queued with the detach flag.
    pub fn release_node(&mut self, node: usize) -> Released {
        let master = self
            .nodes
            .get(node)
            .and_then(|n| n.master)
            .filter(|&m| m < self.nodes.len());
        let node = master.unwrap_or(node);
        let owner = self.nodes.get(node).map_or(NodeOwner::None, |n| n.owner);
        let queued = self.queue_removal(node, true);
        Released {
            owner,
            node,
            queued,
        }
    }

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
                if flags & CUT_AWAY_CHILDREN != 0 {
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

    /// The hidden-sector pass: every mesh keeps only [`HIDDEN_KEEPS`] of its
    /// mask, and a moved mesh's cell is kept current in grid mode.
    fn hidden_pass(&mut self, sector: usize) {
        let meshes = self.sectors[sector].meshes.clone();
        self.refresh_stale(&meshes);
        let (grid, grid_mode) = (self.grid, self.grid_mode);
        for &id in &meshes {
            let node = &mut self.nodes[id];
            node.mask &= HIDDEN_KEEPS;
            if node.moved {
                if grid_mode {
                    store_cell(node, grid);
                }
                node.moved = false;
            }
        }
    }

    /// The visible-sector pass over the sector's meshes. Returns whether a
    /// seen mesh moved.
    fn visible_pass(&mut self, index: usize, dt_ms: f32) -> bool {
        let meshes = self.sectors[index].meshes.clone();
        self.refresh_stale(&meshes);
        let (grid, grid_mode) = (self.grid, self.grid_mode);
        let sectors = &self.sectors;
        let sector = &sectors[index];
        let scissor = if sector.scissor.is_some() { SCISSOR } else { 0 };
        let mut through_door = false;
        let mut moved_seen = false;
        for &id in &meshes {
            let node = &mut self.nodes[id];
            node.mask = scissor;
            if !sector.visible_portals.is_empty() && doorway(sectors, sector, node, grid.0) {
                through_door = true;
            } else {
                let (obb, radius) = node.obb();
                if sector.views.iter().any(|v| v.intersects_obb(&obb, radius)) {
                    node.mask |= MAIN | SEEN;
                }
            }
            let inherited = node.inherited & (INHERIT_FADE | INHERIT_HIDDEN);
            if inherited != 0 {
                node.mask = if node.mask & SEEN != 0 && inherited != INHERIT_FADE | INHERIT_HIDDEN {
                    SEEN
                } else {
                    0
                };
                if inherited & INHERIT_HIDDEN != 0 {
                    continue;
                }
            }
            if node.moved {
                if grid_mode {
                    store_cell(node, grid);
                }
                if node.mask & SEEN != 0 {
                    moved_seen = true;
                }
                node.moved = false;
            }
            fade_node(node, dt_ms);
        }
        if through_door {
            // A mesh seen through a doorway marks its sector as seen.
            self.sectors[index].mask |= MAIN | SEEN;
        }
        moved_seen
    }
}

/// Stores a node's cell and spill mask; returns whether the cell changed.
fn store_cell(node: &mut SceneNode, grid: (usize, usize)) -> bool {
    let spill = node.cell_spill(grid);
    let changed = node.cell.cell != spill.cell;
    node.cell = spill;
    changed
}

/// The doorway test of the visible pass: when the mesh straddles one of the
/// sector's doorways to an active sector ([`doorway_portal`]) and meets one
/// of that sector's views, it is drawn in the main view without the
/// scissor. The first doorway the mesh straddles decides.
fn doorway(sectors: &[Sector], sector: &Sector, node: &mut SceneNode, grid_width: usize) -> bool {
    let portals = sector.visible_portals.iter().map(|&at| &sector.portals[at]);
    let Some(beyond) = doorway_portal(node, portals, grid_width).and_then(|s| sectors.get(s))
    else {
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
fn fade_node(node: &mut SceneNode, dt_ms: f32) {
    if node.mask & (MAIN | SEEN) == 0 {
        return;
    }
    if !node.cut_away {
        if 0.0 < node.fade {
            node.fade -= dt_ms * FADE_RATE;
            if !(node.fade as f64 > FADE_DONE) {
                node.fade = 0.0;
            } else {
                node.mask = (node.mask & !MAIN) | DITHERED;
            }
        }
        return;
    }
    node.fade += dt_ms * FADE_RATE;
    if !(node.fade >= node.fade_target - FADE_EPSILON) {
        if node.mask & MAIN != 0 {
            node.mask = (node.mask & !MAIN) | DITHERED;
        }
        return;
    }
    node.fade = node.fade_target;
    if !(node.fade as f64 >= FADE_PARTIAL) {
        if node.mask & MAIN != 0 {
            node.mask = (node.mask & !MAIN) | PARTIAL;
        }
    } else {
        node.mask &= !MAIN;
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
        let scene = scene_bounds(SectorBounds::default(), &[room]);
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
        let scene = scene_bounds(own, &[room]);
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
        assert_eq!(scene.sectors[1].changes, LINKS_CHANGED);
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
        scene.update_visibility(16.0);
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
        assert!(scene.update_visibility(16.0), "a seen mesh moved");
        assert_eq!(scene.nodes[node].mask, MAIN | SEEN);
        assert!(!scene.update_visibility(16.0), "nothing moved since");
        // Out of every view: no mask.
        scene.set_frame(
            node,
            Vec3::new(100.0, 0.0, 0.0),
            Mat3::IDENTITY,
            Vec3::splat(0.5),
        );
        assert!(!scene.update_visibility(16.0), "moved but not seen");
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
        scene.update_visibility(16.0);
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
        scene.update_visibility(16.0);
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
        scene.update_visibility(100.0);
        assert_eq!(scene.nodes[node].mask, SEEN);
        assert_eq!(scene.nodes[node].draw(), Draw::None);
        assert!((scene.nodes[node].fade - 0.3).abs() < 1e-6);
        // Following a faded-out parent: seen only, and no fade.
        scene.nodes[node].inherited = INHERIT_HIDDEN;
        scene.update_visibility(100.0);
        assert_eq!(scene.nodes[node].mask, SEEN);
        assert!((scene.nodes[node].fade - 0.3).abs() < 1e-6);
        // Both: not even seen.
        scene.nodes[node].inherited = INHERIT_FADE | INHERIT_HIDDEN;
        scene.update_visibility(100.0);
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
        assert_eq!(
            scene.detect_cut_away(Vec3::new(0.0, 0.0, -5.0), Vec3::Z, 10.0, focus),
            3
        );
        assert!(scene.nodes[group].cut_away && scene.nodes[bit].cut_away);
        assert!(!scene.nodes[light].cut_away);
        // Clearing carries too.
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
    fn a_fade_moves_at_three_thousandths_a_millisecond() {
        let (mut scene, sector, node) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        scene.nodes[node].cut_away = true;
        scene.nodes[node].fade_target = 1.0;
        scene.update_visibility(100.0);
        assert!((scene.nodes[node].fade - 0.3).abs() < 1e-6);
        assert_eq!(scene.nodes[node].draw(), Draw::Dithered);
        scene.update_visibility(300.0);
        // 1.2 passes the target: the amount snaps to 1 and the mesh is not
        // drawn.
        assert_eq!(scene.nodes[node].fade, 1.0);
        assert_eq!(scene.nodes[node].draw(), Draw::None);
        // Back the other way: fading in to zero restores the solid draw.
        scene.nodes[node].cut_away = false;
        scene.update_visibility(400.0);
        assert_eq!(scene.nodes[node].fade, 0.0);
        assert_eq!(scene.nodes[node].draw(), Draw::Solid);
    }

    #[test]
    fn fade_thresholds_follow_the_original_comparisons() {
        let (mut scene, sector, node) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        scene.update_visibility(0.0);
        // Fading in: 0.001 as an f32 is above 0.001 in double precision, so
        // the fade does not end yet.
        scene.nodes[node].fade = 0.001;
        scene.update_visibility(0.0);
        assert_eq!(scene.nodes[node].fade, 0.001);
        assert_eq!(scene.nodes[node].draw(), Draw::Dithered);
        // Fading out: an amount exactly at the target less the epsilon
        // snaps (the original's test is strict).
        scene.nodes[node].cut_away = true;
        scene.nodes[node].fade_target = 1.0;
        scene.nodes[node].fade = 1.0 - FADE_EPSILON;
        scene.update_visibility(0.0);
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
        scene.update_visibility(200.0);
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
        scene.update_visibility(16.0);
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
        scene.update_visibility(16.0);
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
        scene.update_visibility(16.0);
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
    fn a_node_world_matrix_carries_rotation_and_translation() {
        let mut scene = Scene::new((0, 0));
        let node = scene.add_node(NodeKind::Mesh, None);
        scene.set_frame(
            node,
            Vec3::new(2.0, 3.0, 4.0),
            Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::ONE,
        );
        scene.refresh(node);
        let m = scene.nodes[node].world_matrix();
        assert_eq!([m[0][3], m[1][3], m[2][3]], [2.0, 3.0, 4.0]);
        assert!((m[0][0]).abs() < 1e-6 && (m[2][0] + 1.0).abs() < 1e-6);
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
        assert!(!sector.in_scene && sector.node.is_none());
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
    }

    #[test]
    fn transitions_flip_a_controller_and_drop_finished_entries() {
        let mut scene = Scene::new((0, 0));
        let node = scene.add_node(NodeKind::Auxiliary, None);
        assert!(scene.add_transition(node, 9));
        assert!(!scene.add_transition(node, 9));
        assert_eq!(scene.transitions[0].state, 1);
        assert!(scene.add_transition(node, 10));
        assert_eq!(scene.transitions[1].state, 1);
        scene.time = 500;
        let node2 = scene.add_node(NodeKind::Auxiliary, None);
        assert!(scene.add_transition(node2, 10));
        assert_eq!(scene.transitions[2].state, 2, "the same controller flips");
        scene.time = 2500;
        let mut seen = Vec::new();
        let finished = scene.advance_transitions(|controller, seconds| {
            seen.push((controller, seconds));
            seconds < 1.0
        });
        // The first two started at 500 ms and are 2 s old; the third at
        // 500 too (the clock was set before adding), so all finish.
        assert_eq!(finished, 3);
        assert_eq!(seen[0], (9, 2.5));
        assert_eq!(seen[2], (10, 2.5));
        assert!(scene.transitions.is_empty());
        assert!(!scene.nodes[node].moved);
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
    fn releasing_an_instance_queues_its_master() {
        let mut scene = Scene::new((0, 0));
        let master = scene.add_node(NodeKind::Mesh, None);
        let instance = scene.add_node(NodeKind::Mesh, None);
        scene.nodes[master].owner = NodeOwner::Object(17);
        scene.nodes[instance].master = Some(master);
        let released = scene.release_node(instance);
        assert_eq!(released.owner, NodeOwner::Object(17));
        assert_eq!(released.node, master);
        assert!(released.queued);
        assert!(!scene.nodes[master].removed);
    }

    #[test]
    fn scene_lights_add_and_remove_by_swap() {
        let mut scene = Scene::new((0, 0));
        scene.add_light(4);
        scene.add_light(5);
        let mut removed = Vec::new();
        assert!(scene.remove_light(4, |l| removed.push(l)));
        assert_eq!(scene.lights, [5]);
        assert_eq!(removed, [4]);
        assert!(!scene.remove_light(9, |l| removed.push(l)));
    }
}
