//! The original's scene-node unit: node kinds and membership, world
//! transforms and bounds, the 6 m cell a node sits in, per-frame sector
//! classification through portals, node visibility and fades.
//!
//! A scene owns **sectors**. In a tile-grid level one sector is one room of
//! the room flood fill, plus the default `_World` sector (index 0) that is
//! never freed. A node is registered in the sector that owns it; a sector
//! lists its nodes and its portals (doorway quads to the neighbouring
//! sector). Every frame the room manager gives each sector a state
//! (hidden, active, adjacent to an active one); this module turns the state
//! into a per-sector view list and a per-node visibility mask:
//!
//! - an **active** sector is tested against every scene view frustum, and
//!   the frusta that see its bounds are copied for its nodes;
//! - an **adjacent** sector is seen only through its portals: each portal
//!   quad of an active neighbour whose plane faces the eye is clipped by the
//!   main frustum, the eye and the clipped quad make a portal frustum, and
//!   the 2-D bounds of those frusta give the sector's screen scissor;
//! - a **hidden** sector draws nothing; its nodes keep only their light
//!   links current (the caller refreshes those);
//! - a node is visible when it meets any of its sector's views. Geometry
//!   straddling a doorway (in one of the portal's two cells and reaching at
//!   least 20 cm in front of the portal plane) is tested against the main
//!   view instead, and so escapes the scissor.
//!
//! Nodes also carry the original's cut-away fade: an amount that moves
//! towards its target at 0.003 per millisecond, turning a solid draw into a
//! dithered or partial one. The detector that sets the target (geometry
//! between the camera and the player) is outside this unit.
//!
//! All lengths are metres (the original's centimetres divided by 100) and
//! angles are radians; the cell tests multiply by 100 exactly as the
//! original works in centimetres. The model is pure data: [`crate::rooms`]
//! supplies the room states and [`crate::light_flood`] shares
//! [`cell_spill`]. The render side is Bevy's entity hierarchy; no Bevy
//! types other than maths live here.
//!
//! Rules and evidence: `docs/RENDERING.md` (Rooms and black fill), the
//! research notes on the scene pass (sector classification, node
//! visibility, fades).

// The original's branches on partially ordered comparisons are deliberate:
// a NaN takes the processor's side, so the negated form spells it out.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::{Mat3, Vec2, Vec3};
use exa_physics::intersect::{ConvexVolume, Obb};

use crate::rooms::RoomState;

/// Node flag: registered in a live scene.
pub const IN_SCENE: u32 = 0x0004_0000;
/// Node flag: never freed by the unit's cleanup hook.
pub const NEVER_FREE: u32 = 0x0010_0000;
/// Node flag: owned by the engine (the cleanup hook leaves it alone).
pub const OWNED: u32 = 0x0200_0000;
/// Node flag: the node belongs to the world rather than to one sector (a
/// world light is not copied into a sector's light list).
pub const WORLD_LIGHT: u32 = 0x2;
/// Node flag: the node's frame has been worked out (the original's "moved
/// this frame" marker is the world frame counter).
pub const TRANSFORM_DONE: u32 = 0x1;
/// Bounds split flag: children carrying it bound separately from the rest.
pub const BOUNDS_SPLIT: u32 = 0x1_0000;

/// Node visibility: visible in the main view.
pub const MAIN: u32 = 0x1;
/// Node visibility: seen by at least one view (needed for shadow faces).
pub const SEEN: u32 = 0x0100_0000;
/// Node visibility: drawn through a portal scissor.
pub const SCISSOR: u32 = 0x1000_0000;
/// Node visibility: drawn dithered while fading.
pub const DITHERED: u32 = 0x2000_0000;
/// Node visibility: drawn partially while fading.
pub const PARTIAL: u32 = 0x4000_0000;

/// Node frame bit: the node inherits a fading parent's partial draw.
pub const INHERIT_FADE: u32 = 0x10;
/// Node frame bit: the node inherits a faded-out parent and is not drawn.
pub const INHERIT_HIDDEN: u32 = 0x20;

/// The class code the original tests when a batch entry is a sector.
pub const CLASS_SECTOR: u32 = 0x11;

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
/// A fade that reaches this is complete.
pub const FADE_DONE: f32 = 0.001;
/// A fade amount below this draws partially instead of fully.
pub const FADE_PARTIAL: f32 = 0.999;
/// The original compares the fade against the target less this epsilon.
pub const FADE_EPSILON: f32 = 1.192_089_982_282_595_9e-7;
/// Geometry within this of a portal plane (centimetres) is tested against
/// the main view instead of the scissor.
pub const DOORWAY_REACH: f32 = 20.0;
/// A portal plane faces the eye at least this much (the original's 1e-6).
pub const PORTAL_FACING: f32 = 1e-6;

/// What a node is, from the class code in its flags word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// A plain scene node (class 1): added to the scene's node list.
    Node,
    /// A node whose scene-added callback runs on registration (class 1 with
    /// the callback flag observed there).
    Callback,
    /// Geometry (class 3): added to the scene's geometry list.
    Geometry,
    /// A light (class 5): added to the scene's and its sector's light list
    /// unless it is a world light.
    Light,
    /// A mesh with material parts (class 9): its parts join the sector's
    /// per-material draw buckets.
    Mesh,
    /// A sector (class 0x11): a top-level scene entry with its own nodes.
    Sector,
    /// The auxiliary per-frame list (class 0x41); its exact role is not
    /// named in the evidence (guess: a second sector-like entry).
    Auxiliary,
}

impl NodeKind {
    /// From the class code in a node's flags word. A sector's code is
    /// tested over all 12 bits, as the original's batch insertion does; the
    /// other kinds over the original's 0x5f mask.
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

/// A world-space box with its bounding radius.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bounds {
    pub centre: Vec3,
    pub half_extents: Vec3,
    pub radius: f32,
}

impl Bounds {
    /// The empty box at the origin: the original starts a sector's bounds
    /// from zeros, so the sector's origin is always inside them.
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

    /// From the pair of opposite corners, as the original turns its running
    /// minimum and maximum into a box: the centre is the midpoint, the
    /// extents run to the maximum corner.
    pub fn from_min_max(low: Vec3, high: Vec3) -> Self {
        let centre = (low + high) * 0.5;
        let half_extents = high - centre;
        Self {
            centre,
            half_extents,
            radius: radius_of(half_extents),
        }
    }

    pub fn min(&self) -> Vec3 {
        self.centre - self.half_extents
    }

    pub fn max(&self) -> Vec3 {
        self.centre + self.half_extents
    }

    /// Grows the running low and high corners with the box's world extent
    /// along each axis, with the original's ordered comparisons (a NaN
    /// leaves the corner alone).
    pub fn grow_corners(&self, low: &mut Vec3, high: &mut Vec3) {
        let lo = self.min();
        let hi = self.max();
        for axis in 0..3 {
            if lo[axis] < low[axis] {
                low[axis] = lo[axis];
            }
            if high[axis] < hi[axis] {
                high[axis] = hi[axis];
            }
        }
    }

    /// The smallest box about `self.centre` holding both boxes: each axis
    /// takes the farther of its two sides. This is the original's second
    /// bounds field.
    pub fn around(&self, low: Vec3, high: Vec3) -> Bounds {
        let mut half_extents = Vec3::ZERO;
        for axis in 0..3 {
            let low_side = (low[axis] - self.centre[axis]).abs();
            let high_side = (high[axis] - self.centre[axis]).abs();
            // The comparison is the original's ordered compare: a NaN takes
            // the high side.
            half_extents[axis] = if low_side > high_side {
                low_side
            } else {
                high_side
            };
        }
        Bounds::new(self.centre, half_extents)
    }
}

/// The original's radius: `√(x² + z² + y²)`, summed in that order.
fn radius_of(v: Vec3) -> f32 {
    (v.x * v.x + v.z * v.z + v.y * v.y).sqrt()
}

/// A sector's two bounds: the one its visible children give it (`first`),
/// the other children's box (`second`), and the radius of the box about
/// `first`'s centre that holds both.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SectorBounds {
    pub first: Bounds,
    pub second: Bounds,
    pub union_radius: f32,
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
/// the original works it out for a node's world box (centimetres inside).
///
/// The cell is `floor(x / 600)` plus half the grid, clamped to
/// `[1, count − 2]`. A side spills when the box reaches more than 1 cm past
/// the cell's low edge, or more than 1 cm past its high edge measured from
/// 601 cm (the original's asymmetry). The low side wins when both do.
/// `None` when the grid is too small to hold inner cells.
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
    let extent_x = rotation.row(0).abs().dot(half_extents);
    let extent_z = rotation.row(2).abs().dot(half_extents);
    let mut mask = CELL_MARK;
    let mut cells = [0i64; 2];
    for (axis, coordinate, extent, count) in [
        (0usize, position.x, extent_x, width),
        (1usize, position.z, extent_z, height),
    ] {
        let centimetres = coordinate * 100.0;
        let extent = extent * 100.0;
        let index = (centimetres * INV_CELL_CM).floor();
        let local = centimetres - index * CELL_CM;
        if local - extent < -1.0 {
            mask |= SPILL_LOW << axis;
        } else if local + extent > 601.0 {
            mask |= SPILL_HIGH << axis;
        }
        // The original stores the cell in one byte: a far-out position
        // wraps it, then the inner-cell clamp applies. The wrap is kept so
        // a position outside the grid reads the same cell as the original.
        let byte = ((count / 2) as i64 + index as i64) as u8;
        let byte = if byte == 0 {
            1
        } else if byte >= (count as i64 - 1) as u8 {
            (count as i64 - 2) as u8
        } else {
            byte
        };
        cells[axis] = byte as i64;
    }
    CellSpill {
        cell: Some((cells[0], cells[1])),
        mask,
    }
}

/// A doorway between two sectors, as the visibility pass sees it: the quad
/// on the doorway plane, its plane, the two grid cells it joins and the
/// sector on the other side.
#[derive(Clone, Debug, PartialEq)]
pub struct Portal {
    /// The four corner points of the opening, in order.
    pub quad: [Vec3; 4],
    /// Outward plane normal, pointing into this sector.
    pub normal: Vec3,
    /// Plane offset: `normal · x + offset = 0` on the doorway.
    pub offset: f32,
    /// The grid cell indices on either side of the door.
    pub cells: [usize; 2],
    /// The sector beyond the door.
    pub neighbour: usize,
}

impl Portal {
    /// A doorway between two cells, standing on the edge between them.
    /// `own_centre` and `other_centre` are the cells' centres (metres, X/Z),
    /// `size` the opening (width, height), `floor` its floor height. The
    /// quad's normal points towards the own cell, as the original's does.
    pub fn between_cells(
        own_cell: usize,
        other_cell: usize,
        own_centre: Vec2,
        other_centre: Vec2,
        size: Vec2,
        floor: f32,
    ) -> Self {
        let across = (own_centre - other_centre).normalize_or_zero();
        let normal = Vec3::new(across.x, 0.0, across.y);
        let along = Vec3::new(-normal.z, 0.0, normal.x);
        let centre = Vec3::new(
            (own_centre.x + other_centre.x) * 0.5,
            floor,
            (own_centre.y + other_centre.y) * 0.5,
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
            cells: [own_cell, other_cell],
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
    /// Flags word (the original's `+0xc`): [`WORLD_LIGHT`],
    /// [`TRANSFORM_DONE`], ...
    pub flags: u32,
    pub parent: Option<usize>,
    /// Child slots; `None` is a slot the original would report and drop.
    pub children: Vec<Option<usize>>,
    /// Position relative to the parent (or to the owning sector at the top).
    pub position: Vec3,
    pub rotation: Mat3,
    /// Local half extents of the node's box.
    pub half_extents: Vec3,
    /// World translation and rotation, kept current by [`Scene::refresh`].
    pub world: Vec3,
    pub world_rotation: Mat3,
    /// The world transform is out of date.
    pub stale: bool,
    /// The sector that owns the node once it is in the scene.
    pub sector: Option<usize>,
    /// The light keys linked to a mesh (the original's link chain), for the
    /// shadow relink pass.
    pub lights: Vec<u32>,
    /// The node's cell and spill mask.
    pub cell: CellSpill,
    /// The inherited frame bits [`INHERIT_FADE`]/[`INHERIT_HIDDEN`].
    pub inherited: u32,
    /// Visibility mask from the last frame.
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
            flags: 0,
            parent: None,
            children: Vec::new(),
            position: Vec3::ZERO,
            rotation: Mat3::IDENTITY,
            half_extents: Vec3::ZERO,
            world: Vec3::ZERO,
            world_rotation: Mat3::IDENTITY,
            stale: false,
            sector: None,
            lights: Vec::new(),
            cell: CellSpill::MARKED,
            inherited: 0,
            mask: 0,
            cut_away: false,
            fade: 0.0,
            fade_target: 1.0,
        }
    }

    /// The node's world-space box.
    pub fn bounds(&self) -> Bounds {
        Bounds::new(self.world, self.half_extents)
    }

    /// The node's world-oriented box, with its bounding radius as the
    /// original keeps it.
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

/// One sector: a room, or the scene's default `_World` sector.
#[derive(Clone, Debug)]
pub struct Sector {
    pub name: String,
    /// Flags word: [`NEVER_FREE`] for the default sector.
    pub flags: u32,
    /// Whether the sector is attached to the scene.
    pub in_scene: bool,
    /// The nodes registered in the sector, in insertion order.
    pub nodes: Vec<usize>,
    /// Light keys registered in the sector (its runtime-light list).
    pub lights: Vec<usize>,
    /// The sector's doorways.
    pub portals: Vec<Portal>,
    /// The room manager's state for this frame.
    pub state: SectorState,
    /// The view bitmask from the last classification.
    pub mask: u32,
    /// The sector's views for this frame (world space; the original keeps
    /// them in sector space, which tests the same points).
    pub views: Vec<ConvexVolume>,
    /// The portal scissor rectangle when the sector is adjacent.
    pub scissor: Option<ScreenRect>,
    /// The sector's world translation.
    pub origin: Vec3,
    /// The bounds of its children.
    pub bounds: SectorBounds,
}

impl Sector {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            flags: 0,
            in_scene: false,
            nodes: Vec::new(),
            lights: Vec::new(),
            portals: Vec::new(),
            state: SectorState::Hidden,
            mask: 0,
            views: Vec::new(),
            scissor: None,
            origin: Vec3::ZERO,
            bounds: SectorBounds::default(),
        }
    }

    /// The sector's world box, as the view tests use it.
    pub fn bounds(&self) -> Bounds {
        self.bounds.first
    }
}

/// The whole scene graph.
#[derive(Clone, Debug)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
    pub sectors: Vec<Sector>,
    /// Cell grid (columns, rows).
    pub grid: (usize, usize),
    /// The scene's bounds from its sectors.
    pub bounds: SectorBounds,
}

impl Scene {
    /// A scene with only the default `_World` sector, as the unit creates it
    /// at start-up. The default sector is never freed.
    pub fn new(grid: (usize, usize)) -> Self {
        let mut world = Sector::new("_World");
        world.flags |= NEVER_FREE;
        world.in_scene = true;
        Self {
            nodes: Vec::new(),
            sectors: vec![world],
            grid,
            bounds: SectorBounds::default(),
        }
    }

    /// The index of the default `_World` sector.
    pub fn world(&self) -> usize {
        0
    }

    /// Adds a node, with `parent` none for a top-level node.
    pub fn add_node(&mut self, kind: NodeKind, parent: Option<usize>) -> usize {
        let id = self.nodes.len();
        let mut node = SceneNode::new(kind);
        node.parent = parent;
        node.sector = parent.and_then(|p| self.nodes[p].sector);
        self.nodes.push(node);
        if let Some(parent) = parent {
            self.nodes[parent].children.push(Some(id));
            self.nodes[id].stale = true;
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

    /// Adds a portal to a sector. The caller sets its `neighbour`.
    pub fn add_portal(&mut self, sector: usize, portal: Portal) {
        self.sectors[sector].portals.push(portal);
    }

    /// The sector that owns `node`, falling back to the default sector for
    /// a node that is not registered yet.
    pub fn owner(&self, node: usize) -> usize {
        self.nodes[node].sector.unwrap_or(self.world())
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

    /// Takes a light out of a sector's runtime-light list, as the original
    /// does when a light moves out of the sector or is removed: the light's
    /// owner link is cleared and the list shrinks by swapping the last entry
    /// into its place. Returns whether the light was in the list.
    pub fn remove_light(&mut self, sector: usize, light: usize) -> bool {
        let Some(at) = self.sectors[sector].lights.iter().position(|&n| n == light) else {
            return false;
        };
        self.sectors[sector].lights.swap_remove(at);
        if self.nodes[light].sector == Some(sector) {
            self.nodes[light].sector = None;
            self.nodes[light].flags &= !IN_SCENE;
        }
        true
    }

    /// Sets the room sectors' states (room `i` is sector `i + 1`) from the
    /// room manager's states, leaving the default sector alone.
    pub fn set_room_states(&mut self, states: &[RoomState]) {
        for (index, &state) in states.iter().enumerate() {
            if let Some(sector) = self.sectors.get_mut(index + 1) {
                sector.state = state.into();
            }
        }
    }

    /// Inserts a batch of top-level nodes, as the original's add-nodes call
    /// does: a sector is appended to the scene's sector list (marked in
    /// scene, given the scene as its back pointer and its children
    /// registered), and the first non-sector entry is registered in its own
    /// sector and ends the batch. Returns whether any sector was appended.
    pub fn add_batch(&mut self, roots: &[usize]) -> bool {
        let mut added_sector = false;
        for &root in roots {
            if self.nodes[root].kind != NodeKind::Sector {
                let sector = self.owner(root);
                self.register(root, sector);
                break;
            }
            let sector = self.sectors.len();
            let entry = Sector::new(format!("_Sector{sector}"));
            self.sectors.push(entry);
            self.nodes[root].sector = Some(sector);
            self.nodes[root].flags |= IN_SCENE;
            self.sectors[sector].in_scene = true;
            self.sectors[sector].nodes.push(root);
            let children = self.nodes[root].children.clone();
            for child in children.into_iter().flatten() {
                self.register(child, sector);
            }
            added_sector = true;
        }
        added_sector
    }

    /// Registers a node and its subtree in `sector`, as the original's
    /// registration walk does: the node gets the sector as its owner and
    /// the in-scene flag, a light joins the sector's light list (unless it
    /// is a world light) and the children follow. A node already in the
    /// same sector is left where it is, so a repeated walk does not
    /// duplicate entries.
    pub fn register(&mut self, node: usize, sector: usize) {
        self.nodes[node].sector = Some(sector);
        self.nodes[node].flags |= IN_SCENE;
        if !self.sectors[sector].nodes.contains(&node) {
            self.sectors[sector].nodes.push(node);
        }
        if self.nodes[node].kind == NodeKind::Light
            && self.nodes[node].flags & WORLD_LIGHT == 0
            && !self.sectors[sector].lights.contains(&node)
        {
            self.sectors[sector].lights.push(node);
        }
        let children = self.nodes[node].children.clone();
        for child in children.into_iter().flatten() {
            self.register(child, sector);
        }
    }

    /// Takes a node (and its subtree) out of its sector and clears its
    /// in-scene flag, without freeing it.
    pub fn remove(&mut self, node: usize) {
        let is_light = self.nodes[node].kind == NodeKind::Light;
        if let Some(s) = self.nodes[node]
            .sector
            .and_then(|sector| self.sectors.get_mut(sector))
        {
            s.nodes.retain(|&n| n != node);
            if is_light {
                s.lights.retain(|&n| n != node);
            }
        }
        self.nodes[node].sector = None;
        self.nodes[node].flags &= !IN_SCENE;
        let children = self.nodes[node].children.clone();
        for child in children.into_iter().flatten() {
            self.remove(child);
        }
    }

    /// Moves a node's subtree to `sector`, keeping its world position as the
    /// original's move does. Returns whether the node was in the scene.
    pub fn move_to_sector(&mut self, node: usize, sector: usize) -> bool {
        self.refresh(node);
        let world = self.nodes[node].world;
        let in_scene = self.nodes[node].flags & IN_SCENE != 0;
        if self.nodes[node].sector.is_some() {
            self.remove(node);
        }
        self.nodes[node].parent = None;
        self.nodes[node].position = world - self.sectors[sector].origin;
        self.nodes[node].stale = true;
        // The new sector is the node's frame origin, so its world position
        // survives the move.
        self.nodes[node].sector = Some(sector);
        self.refresh(node);
        if in_scene {
            self.register(node, sector);
        }
        in_scene
    }

    /// Refreshes a node's world frame and its subtree's, as the transform
    /// update does. A top-level node hangs from its sector's origin.
    pub fn refresh(&mut self, node: usize) {
        let (parent_world, parent_rotation) = match self.nodes[node].parent {
            Some(parent) => {
                if self.nodes[parent].stale {
                    self.refresh(parent);
                }
                (self.nodes[parent].world, self.nodes[parent].world_rotation)
            }
            None => (
                self.nodes[node]
                    .sector
                    .map_or(Vec3::ZERO, |s| self.sectors[s].origin),
                Mat3::IDENTITY,
            ),
        };
        let position = self.nodes[node].position;
        let rotation = self.nodes[node].rotation;
        let body = &mut self.nodes[node];
        body.world = parent_world + parent_rotation * position;
        body.world_rotation = parent_rotation * rotation;
        body.stale = false;
        let children = body.children.clone();
        for child in children.into_iter().flatten() {
            self.refresh(child);
        }
    }

    /// Validates a node's child list as the original does: drops empty
    /// slots and refreshes a child whose world frame is stale. Returns the
    /// number of dropped slots. (The original logs each dropped null child;
    /// Exarust's hierarchy has no null slots, so only malformed model data
    /// reaches this.)
    pub fn validate_children(&mut self, node: usize) -> usize {
        let before = self.nodes[node].children.len();
        self.nodes[node].children.retain(Option::is_some);
        let dropped = before - self.nodes[node].children.len();
        let children = self.nodes[node].children.clone();
        for child in children.into_iter().flatten() {
            if self.nodes[child].stale {
                self.refresh(child);
            }
        }
        dropped
    }

    /// Whether the node or any descendant carries [`OWNED`] (the original's
    /// check before freeing a subtree).
    pub fn owns_subtree(&self, node: usize) -> bool {
        if self.nodes[node].flags & OWNED != 0 {
            return true;
        }
        self.nodes[node]
            .children
            .iter()
            .flatten()
            .any(|&child| self.owns_subtree(child))
    }

    /// Works out a node's cell and spill mask from its world frame, and
    /// returns whether the word changed (the original calls the scene
    /// callback then, when the node's parent is a sector).
    pub fn update_cell(&mut self, node: usize) -> bool {
        if self.nodes[node].stale {
            self.refresh(node);
        }
        let node_ref = &self.nodes[node];
        let cell = cell_spill(
            node_ref.world,
            node_ref.half_extents,
            node_ref.world_rotation,
            self.grid,
        );
        let changed = self.nodes[node].cell != cell;
        self.nodes[node].cell = cell;
        changed
    }

    /// Updates the cells of every registered node, returning how many
    /// changed.
    pub fn update_cells(&mut self) -> usize {
        let nodes: Vec<usize> = self
            .sectors
            .iter()
            .flat_map(|s| s.nodes.iter().copied())
            .collect();
        nodes.into_iter().filter(|&n| self.update_cell(n)).count()
    }

    /// Recomputes a sector's bounds from its children, splitting them by
    /// [`BOUNDS_SPLIT`], exactly as the original does (including the origin
    /// in both running boxes and the axis extent clamp to each child's
    /// radius). The sector's world origin is added to the result when the
    /// children are stored relative to the sector; Exarust stores world
    /// frames, so the result is already in world space.
    pub fn rebuild_sector_bounds(&mut self, sector: usize) {
        let nodes: Vec<usize> = self.sectors[sector].nodes.clone();
        let children: Vec<(Bounds, bool)> = nodes
            .iter()
            .filter_map(|&n| {
                let node = self.nodes.get(n)?;
                if node.kind == NodeKind::Sector {
                    return None;
                }
                Some((
                    node.bounds(),
                    node.flags & BOUNDS_SPLIT != 0 || node.inherited & BOUNDS_SPLIT != 0,
                ))
            })
            .collect();
        self.sectors[sector].bounds = sector_bounds(&children);
    }

    /// Recomputes every sector's bounds.
    pub fn rebuild_bounds(&mut self) {
        for sector in 0..self.sectors.len() {
            self.rebuild_sector_bounds(sector);
        }
        let sectors: Vec<SectorBounds> = self.sectors.iter().map(|s| s.bounds).collect();
        self.bounds = scene_bounds(self.bounds, &sectors);
    }

    /// Classifies every sector for this frame from its room state, the
    /// scene's main views and the eye position, as the per-frame scene
    /// update does; `project` maps a world point to screen coordinates for
    /// an adjacent sector's scissor.
    ///
    /// Active sectors keep the main views that meet their bounds. Adjacent
    /// sectors build one frustum per portal of an active neighbour whose
    /// plane faces the eye, by clipping the portal quad against each main
    /// view and fanning from the eye
    /// ([`ConvexVolume::clip_polygon`] is the original's clipping routine).
    /// Sector 0 always carries every view bit when it is not hidden.
    pub fn classify(&mut self, eye: Vec3, main: &[ConvexVolume], project: &dyn Fn(Vec3) -> Vec2) {
        let states: Vec<SectorState> = self.sectors.iter().map(|s| s.state).collect();
        let world = self.world();
        for (index, &state) in states.iter().enumerate() {
            match state {
                SectorState::Hidden => {
                    let sector = &mut self.sectors[index];
                    sector.mask = 0;
                    sector.views.clear();
                    sector.scissor = None;
                }
                SectorState::Active => {
                    let bounds = self.sectors[index].bounds.first;
                    let obb = Obb::axis_aligned(bounds.centre, bounds.half_extents);
                    let mut views = Vec::new();
                    let mut mask = 0u32;
                    for (bit, view) in main.iter().enumerate().take(32) {
                        if view.intersects_obb(&obb, bounds.radius) {
                            mask |= 1 << bit;
                            views.push(view.clone());
                        }
                    }
                    let sector = &mut self.sectors[index];
                    sector.mask = if index == world { u32::MAX } else { mask };
                    sector.views = views;
                    sector.scissor = None;
                    if index == world && sector.views.is_empty() {
                        sector.views = main.to_vec();
                    }
                }
                SectorState::Adjacent => {
                    let portals = self.sectors[index].portals.clone();
                    let mut views = Vec::new();
                    for portal in &portals {
                        if states.get(portal.neighbour) != Some(&SectorState::Active) {
                            continue;
                        }
                        if !(PORTAL_FACING <= portal.normal.dot(eye) + portal.offset) {
                            continue;
                        }
                        for view in main {
                            if let Some(portal_view) = view.clip_polygon(&portal.quad, eye) {
                                views.push(portal_view);
                            }
                        }
                    }
                    let sector = &mut self.sectors[index];
                    if views.is_empty() {
                        sector.mask = 0;
                        sector.scissor = None;
                        sector.views.clear();
                    } else {
                        // An adjacent sector carries the main view only: the
                        // portal frusta are its views.
                        sector.mask = 1;
                        sector.scissor = Some(scissor_rect(&views, project));
                        sector.views = views;
                    }
                }
            }
        }
    }

    /// Runs the per-frame node visibility pass for every sector and moves
    /// each cut-away node's fade by `dt_ms`. Hidden sectors clear their
    /// nodes' masks and leave their light links to the caller, as the
    /// original's hidden-sector pass does. Returns whether any node is seen
    /// (the shadow maps need faces then).
    pub fn update_visibility(&mut self, dt_ms: f32) -> bool {
        let mut seen_any = false;
        let states: Vec<SectorState> = self.sectors.iter().map(|s| s.state).collect();
        for (index, &state) in states.iter().enumerate() {
            let nodes = self.sectors[index].nodes.clone();
            if state == SectorState::Hidden {
                for &id in &nodes {
                    self.nodes[id].mask = 0;
                }
                continue;
            }
            let views = self.sectors[index].views.clone();
            let portals = self.sectors[index].portals.clone();
            let scissor = self.sectors[index].scissor.is_some();
            // A doorway node is tested against the active sector beyond it.
            let beyond: Vec<(usize, Vec<ConvexVolume>)> = portals
                .iter()
                .map(|p| (p.neighbour, self.sectors[p.neighbour].views.clone()))
                .collect();
            for &id in &nodes {
                if self.nodes[id].stale {
                    self.refresh(id);
                }
                let (obb, radius) = self.nodes[id].obb();
                let mut mask = if scissor { SCISSOR } else { 0 };
                let mut visible = false;
                if !portals.is_empty()
                    && let Some(neighbour) = doorway_portal(&self.nodes[id], &portals, self.grid.0)
                {
                    for (sector, views) in &beyond {
                        if *sector != neighbour {
                            continue;
                        }
                        if views.iter().any(|view| view.intersects_obb(&obb, radius)) {
                            // Straddling the doorway escapes the scissor.
                            mask = (mask & !SCISSOR) | MAIN | SEEN;
                            visible = true;
                            break;
                        }
                    }
                }
                if !visible {
                    for view in &views {
                        if view.intersects_obb(&obb, radius) {
                            mask |= MAIN | SEEN;
                            break;
                        }
                    }
                }
                let inherited = self.nodes[id].inherited & (INHERIT_FADE | INHERIT_HIDDEN);
                if inherited != 0 {
                    // An inherited frame keeps the seen bit (shadow faces
                    // stay needed) and drops the main-view draw bit.
                    mask = if mask & SEEN == 0 { 0 } else { SEEN };
                }
                self.nodes[id].mask = mask;
                if inherited & INHERIT_HIDDEN == 0 {
                    fade_node(&mut self.nodes[id], dt_ms);
                }
                if mask & SEEN != 0 {
                    seen_any = true;
                }
            }
        }
        seen_any
    }
}

/// A sector's bounds from its children, split into the [`BOUNDS_SPLIT`]
/// group and the rest as the original computes them:
/// 1. the first group's box starts at the origin, grows by each child's
///    world axis extent clamped to the child's radius;
/// 2. its centre and extents come from the running corners;
/// 3. the second group's box is built the same way;
/// 4. the radius of the smallest box about the first centre holding both is
///    kept.
pub fn sector_bounds(children: &[(Bounds, bool)]) -> SectorBounds {
    let mut low = Vec3::ZERO;
    let mut high = Vec3::ZERO;
    for (bounds, split) in children {
        if *split {
            grow_clamped(bounds, &mut low, &mut high);
        }
    }
    let first = Bounds::from_min_max(low, high);
    let mut low = Vec3::ZERO;
    let mut high = Vec3::ZERO;
    for (bounds, split) in children {
        if !*split {
            grow_clamped(bounds, &mut low, &mut high);
        }
    }
    let second = Bounds::from_min_max(low, high);
    let union = first.around(low, high);
    SectorBounds {
        first,
        second,
        union_radius: union.radius,
    }
}

/// Grows running corners by a child box whose axis extent is clamped to its
/// bounding radius first (the original's per-axis clamp).
fn grow_clamped(bounds: &Bounds, low: &mut Vec3, high: &mut Vec3) {
    let mut extents = bounds.half_extents;
    for axis in 0..3 {
        if extents[axis] > bounds.radius {
            extents[axis] = bounds.radius;
        }
    }
    let centre = bounds.centre;
    for axis in 0..3 {
        let side = centre[axis] - extents[axis];
        if side < low[axis] {
            low[axis] = side;
        }
        let side = centre[axis] + extents[axis];
        if high[axis] < side {
            high[axis] = side;
        }
    }
}

/// The scene's bounds from its sectors, as the original's scene-bounds
/// pass: the first box is the union of the sectors' second boxes (the
/// previous first box seeds it), and the second box is the smallest box
/// about the first's centre that holds it.
pub fn scene_bounds(current: SectorBounds, sectors: &[SectorBounds]) -> SectorBounds {
    let mut low = current.first.min();
    let mut high = current.first.max();
    for sector in sectors {
        sector.second.grow_corners(&mut low, &mut high);
    }
    let first = Bounds::from_min_max(low, high);
    let mut low = current.second.min();
    let mut high = current.second.max();
    for sector in sectors {
        sector.second.grow_corners(&mut low, &mut high);
    }
    let second = first.around(low, high);
    SectorBounds {
        first,
        second,
        union_radius: second.radius,
    }
}

/// The neighbour sector of the doorway a node straddles: it lies in one of
/// the portal's two cells and its box reaches at least [`DOORWAY_REACH`]
/// centimetres in front of the portal plane. Such a node is tested against
/// the active sector beyond the door and drawn without the portal scissor.
/// `grid_width` turns the node's cell bytes into the original's flat cell
/// index.
pub fn doorway_portal(node: &SceneNode, portals: &[Portal], grid_width: usize) -> Option<usize> {
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
        let radius = radius * 100.0;
        // A node whose sphere does not come within 20 cm of the plane is
        // left to the scissor.
        if !(DOORWAY_REACH - radius <= distance) {
            continue;
        }
        // Its box's support along the plane normal, with the original's
        // sign-bit extent flip.
        let local = obb.local(portal.normal);
        let support = (local.x.abs() * obb.half_extents.x
            + local.y.abs() * obb.half_extents.y
            + local.z.abs() * obb.half_extents.z)
            * 100.0;
        if !(distance + support < DOORWAY_REACH) {
            return Some(portal.neighbour);
        }
    }
    None
}

/// The screen rectangle of a set of views: every face polygon's vertices
/// projected and rounded to the window's pixel grid.
pub fn scissor_rect(views: &[ConvexVolume], project: &dyn Fn(Vec3) -> Vec2) -> ScreenRect {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for view in views {
        for plane in &view.planes {
            for &vertex in &plane.vertices {
                let p = project(vertex);
                if p.x < min.x {
                    min.x = p.x;
                }
                if p.y < min.y {
                    min.y = p.y;
                }
                if max.x < p.x {
                    max.x = p.x;
                }
                if max.y < p.y {
                    max.y = p.y;
                }
            }
        }
    }
    if min.x > max.x {
        return ScreenRect {
            min_x: 0,
            min_y: 0,
            max_x: 0,
            max_y: 0,
        };
    }
    ScreenRect {
        min_x: min.x.round() as i32,
        min_y: min.y.round() as i32,
        max_x: max.x.round() as i32,
        max_y: max.y.round() as i32,
    }
}

/// One step of a node's cut-away fade, as the visible-sector pass runs it:
/// the amount moves towards the target at [`FADE_RATE`] per millisecond,
/// and the draw bit is downgraded from solid to dithered (while fading) or
/// to partial (when it is nearly complete). The original keeps the seen bit
/// and only changes the draw bits.
fn fade_node(node: &mut SceneNode, dt_ms: f32) {
    if node.mask & (MAIN | SEEN) == 0 {
        return;
    }
    if !node.cut_away {
        if node.fade > 0.0 {
            node.fade -= dt_ms * FADE_RATE;
            if node.fade <= FADE_DONE {
                node.fade = 0.0;
            } else {
                node.mask = (node.mask & !MAIN) | DITHERED;
            }
        }
    } else {
        node.fade += dt_ms * FADE_RATE;
        let target = node.fade_target - FADE_EPSILON;
        if !(node.fade > target) {
            if node.mask & MAIN != 0 {
                node.mask = (node.mask & !MAIN) | DITHERED;
            }
        } else {
            node.fade = node.fade_target;
            if node.fade < FADE_PARTIAL {
                if node.mask & MAIN != 0 {
                    node.mask = (node.mask & !MAIN) | PARTIAL;
                }
            } else {
                node.mask &= !MAIN;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_view(half: f32) -> ConvexVolume {
        use exa_physics::intersect::Plane;
        ConvexVolume {
            planes: vec![
                Plane::new(Vec3::X, -half),
                Plane::new(-Vec3::X, -half),
                Plane::new(Vec3::Y, -half),
                Plane::new(-Vec3::Y, -half),
                Plane::new(Vec3::Z, -half),
                Plane::new(-Vec3::Z, -half),
            ],
        }
    }

    fn no_project(_: Vec3) -> Vec2 {
        Vec2::ZERO
    }

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

    #[test]
    fn kind_codes_follow_the_class_mask() {
        assert_eq!(NodeKind::from_class(0x11), NodeKind::Sector);
        assert_eq!(NodeKind::from_class(0x3), NodeKind::Geometry);
        assert_eq!(NodeKind::from_class(0x5), NodeKind::Light);
        assert_eq!(NodeKind::from_class(0x9), NodeKind::Mesh);
        assert_eq!(NodeKind::from_class(0x41), NodeKind::Auxiliary);
        assert_eq!(NodeKind::from_class(0x0), NodeKind::Node);
    }

    #[test]
    fn radius_sums_x_then_z_then_y() {
        let b = Bounds::new(Vec3::ZERO, Vec3::new(3.0, 4.0, 12.0));
        assert_eq!(b.radius, 13.0);
    }

    #[test]
    fn bounds_corners_and_around() {
        let a = Bounds::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(a.min(), Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(a.max(), Vec3::new(1.0, 2.0, 3.0));
        let mut low = Vec3::new(-5.0, 0.0, 0.0);
        let mut high = Vec3::new(5.0, 0.0, 0.0);
        a.grow_corners(&mut low, &mut high);
        assert_eq!(low, Vec3::new(-5.0, -2.0, -3.0));
        assert_eq!(high, Vec3::new(5.0, 2.0, 3.0));
        let around = a.around(low, high);
        // About x = 0, the far side is 5; y and z take ±3 and ±2.
        assert_eq!(around.centre, Vec3::ZERO);
        assert_eq!(around.half_extents, Vec3::new(5.0, 2.0, 3.0));
        assert_eq!(around.radius, radius_of(Vec3::new(5.0, 2.0, 3.0)));
    }

    #[test]
    fn sector_bounds_split_and_include_the_origin() {
        let split = (Bounds::new(Vec3::ZERO, Vec3::new(100.0, 50.0, 50.0)), true);
        let rest = (
            Bounds::new(Vec3::new(600.0, 0.0, 0.0), Vec3::new(100.0, 0.0, 0.0)),
            false,
        );
        let b = sector_bounds(&[split, rest]);
        assert_eq!(b.first.centre, Vec3::ZERO);
        assert_eq!(b.first.half_extents, Vec3::new(100.0, 50.0, 50.0));
        assert_eq!(
            b.first.radius,
            (100.0f32 * 100.0 + 50.0 * 50.0 + 50.0 * 50.0).sqrt()
        );
        // The second group starts at the origin too: x runs 0..700.
        assert_eq!(b.second.centre, Vec3::new(350.0, 0.0, 0.0));
        assert_eq!(b.second.half_extents, Vec3::new(350.0, 0.0, 0.0));
        // The union about the first centre takes the far x side, 700.
        assert_eq!(b.union_radius, 700.0);
    }

    #[test]
    fn sector_bounds_clamp_an_extent_to_the_child_radius() {
        // A child whose x extent is larger than its radius: the clamp cuts
        // it to the radius before the corners grow.
        let child = (
            Bounds {
                centre: Vec3::ZERO,
                half_extents: Vec3::new(200.0, 0.0, 0.0),
                radius: 100.0,
            },
            true,
        );
        let b = sector_bounds(&[child]);
        assert_eq!(b.first.half_extents, Vec3::new(100.0, 0.0, 0.0));
        assert_eq!(b.first.radius, 100.0);
        // The second group is empty and stays at the origin, so the union
        // about the first centre has no extent.
        assert_eq!(b.union_radius, 0.0);
    }

    #[test]
    fn scene_bounds_union_the_sectors_second_boxes() {
        let sector = SectorBounds {
            first: Bounds::new(Vec3::new(600.0, 0.0, 0.0), Vec3::new(300.0, 100.0, 100.0)),
            second: Bounds::new(Vec3::new(600.0, 0.0, 0.0), Vec3::new(300.0, 100.0, 100.0)),
            union_radius: 0.0,
        };
        let scene = scene_bounds(SectorBounds::default(), &[sector]);
        // The empty scene starts at the origin, so the scene box runs
        // 0..900 in x instead of 300..900.
        assert_eq!(scene.first.centre, Vec3::new(450.0, 0.0, 0.0));
        assert_eq!(scene.first.half_extents, Vec3::new(450.0, 100.0, 100.0));
        assert_eq!(scene.union_radius, radius_of(scene.second.half_extents));
    }

    #[test]
    fn cells_round_down_and_spill_at_the_edges() {
        let grid = (32, 40);
        // x = 0 is the low edge of cell 16 (column 16 spans 0..6 m), so a
        // 25 cm box spills low on both axes.
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
        // The low side wins when both would spill.
        let cell = cell_spill(
            Vec3::new(5.99, 0.0, 3.0),
            Vec3::splat(0.5),
            Mat3::IDENTITY,
            grid,
        );
        assert_eq!(cell.mask, CELL_MARK | SPILL_HIGH);
        // The cell is one byte: far outside the grid it wraps, then the
        // inner-cell clamp applies (the original's behaviour).
        let cell = cell_spill(
            Vec3::new(-1000.0, 0.0, -1000.0),
            Vec3::ZERO,
            Mat3::IDENTITY,
            grid,
        );
        assert_eq!(cell.cell, Some((30, 38)));
        let cell = cell_spill(
            Vec3::new(130.0, 0.0, 130.0),
            Vec3::ZERO,
            Mat3::IDENTITY,
            grid,
        );
        assert_eq!(cell.cell, Some((30, 38)));
        // A rotated 1 m box has a 1 m x extent when turned 45°.
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
    fn batch_insertion_appends_sectors_and_registers_one_node_tree() {
        let mut scene = Scene::new((32, 40));
        let a = scene.add_node(NodeKind::Sector, None);
        let b = scene.add_node(NodeKind::Sector, None);
        assert!(scene.add_batch(&[a, b]));
        assert_eq!(scene.sectors.len(), 3, "the default sector and two rooms");
        assert!(scene.sectors[1].in_scene);
        assert_eq!(scene.nodes[a].flags & IN_SCENE, IN_SCENE);
        // A batch that starts with a node registers it in its sector (the
        // default sector here) and stops.
        let node = scene.add_node(NodeKind::Mesh, None);
        assert!(!scene.add_batch(&[node, a]));
        assert_eq!(scene.nodes[node].sector, Some(0));
        assert_eq!(scene.sectors[0].nodes, vec![node]);
        assert_eq!(scene.sectors[1].nodes, vec![a]);
    }

    #[test]
    fn registration_recurses_and_lights_join_the_sector() {
        let mut scene = Scene::new((32, 40));
        let sector = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[sector]);
        let root = scene.add_node(NodeKind::Node, None);
        let light = scene.add_node(NodeKind::Light, Some(root));
        let child = scene.add_node(NodeKind::Mesh, Some(root));
        scene.register(root, 1);
        assert_eq!(scene.sectors[1].nodes, vec![sector, root, light, child]);
        assert_eq!(scene.sectors[1].lights, vec![light]);
        // Registering again does not duplicate.
        scene.register(root, 1);
        assert_eq!(scene.sectors[1].nodes, vec![sector, root, light, child]);
        // A world light does not join the sector's light list.
        let world = scene.add_node(NodeKind::Light, None);
        scene.nodes[world].flags |= WORLD_LIGHT;
        scene.register(world, 1);
        assert_eq!(scene.sectors[1].lights, vec![light]);
    }

    #[test]
    fn an_owned_subtree_is_found_from_its_root() {
        let mut scene = Scene::new((32, 40));
        let root = scene.add_node(NodeKind::Node, None);
        let child = scene.add_node(NodeKind::Mesh, Some(root));
        let leaf = scene.add_node(NodeKind::Mesh, Some(child));
        assert!(!scene.owns_subtree(root));
        scene.nodes[leaf].flags |= OWNED;
        assert!(scene.owns_subtree(root));
        assert!(scene.owns_subtree(child));
        assert!(scene.owns_subtree(leaf));
    }

    #[test]
    fn a_light_leaves_its_sector_list() {
        let mut scene = Scene::new((32, 40));
        let sector = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[sector]);
        let light = scene.add_node(NodeKind::Light, None);
        scene.register(light, 1);
        assert_eq!(scene.sectors[1].lights, vec![light]);
        assert!(scene.remove_light(1, light));
        assert!(scene.sectors[1].lights.is_empty());
        assert_eq!(scene.nodes[light].flags & IN_SCENE, 0);
        assert!(!scene.remove_light(1, light), "already gone");
    }

    #[test]
    fn the_sector_child_of_a_deep_node_is_its_top_node() {
        let mut scene = Scene::new((32, 40));
        let sector = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[sector]);
        let top = scene.add_node(NodeKind::Mesh, None);
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
    fn removing_and_moving_a_subtree() {
        let (mut scene, sector, node) = node_scene();
        let child = scene.add_node(NodeKind::Mesh, Some(node));
        scene.register(child, sector);
        assert_eq!(scene.sectors[sector].nodes.len(), 3);
        // Moving the root into another sector keeps its world position and
        // re-registers the whole subtree there.
        let other = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[other]);
        scene.sectors[2].origin = Vec3::new(100.0, 0.0, 0.0);
        assert!(scene.move_to_sector(node, 2));
        assert_eq!(scene.nodes[node].sector, Some(2));
        assert!((scene.nodes[node].world - Vec3::new(1.0, 1.0, 1.0)).length() < 1e-5);
        assert_eq!(scene.sectors[2].nodes, vec![other, node, child]);
        assert_eq!(
            scene.sectors[sector].nodes,
            vec![0],
            "the sector node alone"
        );
        // Removing clears the in-scene flag and the sector membership.
        scene.remove(child);
        assert_eq!(scene.nodes[child].sector, None);
        assert_eq!(scene.nodes[child].flags & IN_SCENE, 0);
    }

    #[test]
    fn validate_children_drops_null_slots_and_refreshes_stale_children() {
        let mut scene = Scene::new((32, 40));
        let root = scene.add_node(NodeKind::Node, None);
        let child = scene.add_node(NodeKind::Mesh, Some(root));
        scene.nodes[root].children.push(None);
        scene.nodes[child].stale = true;
        assert_eq!(scene.validate_children(root), 1);
        assert_eq!(scene.nodes[root].children, vec![Some(child)]);
        assert!(!scene.nodes[child].stale);
    }

    #[test]
    fn world_frames_follow_parents_and_sector_origins() {
        let mut scene = Scene::new((32, 40));
        scene.sectors[0].origin = Vec3::new(0.0, 0.0, 0.0);
        let root = scene.add_node(NodeKind::Node, None);
        scene.set_frame(
            root,
            Vec3::new(1.0, 0.0, 0.0),
            Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::ONE,
        );
        let child = scene.add_node(NodeKind::Mesh, Some(root));
        scene.set_frame(child, Vec3::new(2.0, 0.0, 0.0), Mat3::IDENTITY, Vec3::ONE);
        scene.register(root, 0);
        scene.refresh(root);
        // 90° about Y turns +X into −Z.
        assert!((scene.nodes[root].world - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
        assert!((scene.nodes[child].world - Vec3::new(1.0, 0.0, -2.0)).length() < 1e-5);
        // A top-level node hangs from its sector's origin.
        let top = scene.add_node(NodeKind::Mesh, None);
        scene.set_frame(top, Vec3::new(1.0, 0.0, 0.0), Mat3::IDENTITY, Vec3::ONE);
        scene.sectors[0].origin = Vec3::new(10.0, 0.0, 0.0);
        scene.register(top, 0);
        scene.refresh(top);
        assert_eq!(scene.nodes[top].world, Vec3::new(11.0, 0.0, 0.0));
    }

    #[test]
    fn update_cell_reports_changes() {
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
        scene.set_frame(
            node,
            Vec3::new(9.0, 0.0, 3.0),
            Mat3::IDENTITY,
            Vec3::splat(0.1),
        );
        assert!(scene.update_cell(node));
        assert_eq!(scene.nodes[node].cell.cell, Some((17, 20)));
    }

    #[test]
    fn active_sector_keeps_the_views_that_see_its_bounds() {
        let (mut scene, sector, _) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        let views = [box_view(10.0)];
        scene.classify(Vec3::ZERO, &views, &no_project);
        assert_eq!(scene.sectors[sector].mask, 1);
        assert_eq!(scene.sectors[sector].views.len(), 1);
        // A sector outside the view gets no bits and no views.
        let other = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[other]);
        scene.sectors[2].state = SectorState::Active;
        scene.sectors[2].bounds.first = Bounds::new(Vec3::new(100.0, 0.0, 0.0), Vec3::splat(1.0));
        scene.classify(Vec3::ZERO, &views, &no_project);
        assert_eq!(scene.sectors[sector].mask, 1);
        assert_eq!(scene.sectors[other].mask, 0);
        assert!(scene.sectors[other].views.is_empty());
    }

    #[test]
    fn node_visibility_needs_a_view() {
        let (mut scene, sector, node) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        scene.update_visibility(16.0);
        assert_eq!(scene.nodes[node].mask & (MAIN | SEEN), MAIN | SEEN);
        // Move the node out of every view: no mask.
        scene.set_frame(
            node,
            Vec3::new(100.0, 0.0, 0.0),
            Mat3::IDENTITY,
            Vec3::splat(0.5),
        );
        scene.update_visibility(16.0);
        assert_eq!(scene.nodes[node].mask, 0);
        assert!(!scene.nodes[node].seen());
        // A hidden sector clears the mask even for a node inside a view.
        scene.set_frame(node, Vec3::ZERO, Mat3::IDENTITY, Vec3::splat(0.5));
        scene.sectors[sector].state = SectorState::Hidden;
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        scene.update_visibility(16.0);
        assert_eq!(scene.nodes[node].mask, 0);
    }

    #[test]
    fn inherited_frame_bits_keep_seen_but_drop_the_draw() {
        let (mut scene, sector, node) = node_scene();
        scene.sectors[sector].state = SectorState::Active;
        scene.rebuild_bounds();
        scene.classify(Vec3::ZERO, &[box_view(10.0)], &no_project);
        scene.nodes[node].inherited = INHERIT_FADE;
        scene.update_visibility(16.0);
        // The inherited frame keeps the seen bit only, as the original sets
        // 0x1000000 and not the main-view bit.
        assert_eq!(scene.nodes[node].mask, SEEN);
        assert_eq!(scene.nodes[node].draw(), Draw::None);
        scene.nodes[node].inherited = INHERIT_FADE | INHERIT_HIDDEN;
        scene.update_visibility(16.0);
        assert_eq!(scene.nodes[node].mask, SEEN);
        assert_eq!(scene.nodes[node].draw(), Draw::None);
        // A node no view sees loses the mask entirely.
        scene.nodes[node].inherited = INHERIT_FADE;
        scene.set_frame(
            node,
            Vec3::new(100.0, 0.0, 0.0),
            Mat3::IDENTITY,
            Vec3::splat(0.5),
        );
        scene.update_visibility(16.0);
        assert_eq!(scene.nodes[node].mask, 0);
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
        // 1.2 passes the target: the amount snaps to 1 and the node stops
        // being drawn solid.
        assert_eq!(scene.nodes[node].fade, 1.0);
        assert_eq!(scene.nodes[node].mask & MAIN, 0);
        assert_eq!(scene.nodes[node].draw(), Draw::None);
        // Back the other way: fading in to zero restores the solid draw as
        // soon as the amount reaches 0.001.
        scene.nodes[node].cut_away = false;
        scene.update_visibility(400.0);
        assert_eq!(scene.nodes[node].fade, 0.0);
        assert_eq!(scene.nodes[node].draw(), Draw::Solid);
        // A fade back in from just above the done threshold ends it after
        // two 1 ms steps (0.005 → 0.002 → 0).
        scene.nodes[node].cut_away = true;
        scene.nodes[node].fade = 0.002;
        scene.nodes[node].fade_target = 1.0;
        scene.update_visibility(1.0);
        assert!((scene.nodes[node].fade - 0.005).abs() < 1e-6);
        assert_eq!(scene.nodes[node].draw(), Draw::Dithered);
        scene.nodes[node].cut_away = false;
        scene.update_visibility(1.0);
        assert!((scene.nodes[node].fade - 0.002).abs() < 1e-6);
        assert_eq!(scene.nodes[node].draw(), Draw::Dithered);
        scene.update_visibility(1.0);
        assert_eq!(scene.nodes[node].fade, 0.0);
        assert_eq!(scene.nodes[node].draw(), Draw::Solid);
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

    #[test]
    fn a_portal_faces_the_eye_and_builds_a_scissor() {
        let mut scene = Scene::new((32, 40));
        let active = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[active]);
        let adjacent = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[adjacent]);
        // The active sector is centred at 0; the adjacent one at +20 m. The
        // doorway stands between them at x = 10, normal pointing to the
        // active sector (−X).
        scene.sectors[1].state = SectorState::Active;
        scene.sectors[2].state = SectorState::Adjacent;
        scene.sectors[1].origin = Vec3::ZERO;
        scene.sectors[2].origin = Vec3::new(20.0, 0.0, 0.0);
        let mut portal = Portal::between_cells(
            100,
            101,
            Vec2::new(0.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(2.0, 3.0),
            0.0,
        );
        portal.neighbour = 1;
        scene.add_portal(2, portal);
        scene.rebuild_bounds();
        // Eye on the active side; the main view is wide enough to hold the
        // portal.
        let eye = Vec3::new(-1.0, 1.5, 0.0);
        let project = |p: Vec3| Vec2::new(p.z * 10.0, p.y * 10.0);
        scene.classify(eye, &[box_view(200.0)], &project);
        assert_eq!(scene.sectors[2].mask, 1);
        assert_eq!(scene.sectors[2].views.len(), 1);
        let scissor = scene.sectors[2].scissor.unwrap();
        // The doorway is 2 m wide along ±Z and 3 m high: ±10 × 10 units,
        // 0..30 in y.
        assert_eq!(scissor.min_x, -10);
        assert_eq!(scissor.max_x, 10);
        assert_eq!(scissor.min_y, 0);
        assert_eq!(scissor.max_y, 30);
        // From behind the portal plane, the doorway is not seen.
        scene.classify(Vec3::new(30.0, 1.5, 0.0), &[box_view(200.0)], &project);
        assert_eq!(scene.sectors[2].mask, 0);
        assert!(scene.sectors[2].views.is_empty());
    }

    #[test]
    fn a_doorway_node_escapes_the_scissor() {
        let mut scene = Scene::new((32, 40));
        let active = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[active]);
        let adjacent = scene.add_node(NodeKind::Sector, None);
        scene.add_batch(&[adjacent]);
        scene.sectors[1].state = SectorState::Active;
        scene.sectors[2].state = SectorState::Adjacent;
        // The node's cell is (17, 20), so its flat index is 20·32 + 17.
        let mut portal = Portal::between_cells(
            657,
            658,
            Vec2::new(0.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(2.0, 3.0),
            0.0,
        );
        portal.neighbour = 1;
        scene.add_portal(2, portal);
        scene.rebuild_bounds();
        let node = scene.add_node(NodeKind::Mesh, None);
        scene.set_frame(
            node,
            Vec3::new(9.9, 1.0, 0.0),
            Mat3::IDENTITY,
            Vec3::new(0.4, 0.4, 0.4),
        );
        scene.register(node, 2);
        scene.update_cell(node);
        assert_eq!(scene.nodes[node].cell.cell, Some((17, 20)));
        scene.classify(Vec3::new(-1.0, 1.5, 0.0), &[box_view(200.0)], &no_project);
        scene.sectors[2].views = vec![box_view(200.0)];
        scene.update_visibility(16.0);
        assert_eq!(scene.nodes[node].mask & (MAIN | SEEN), MAIN | SEEN);
        assert_eq!(scene.nodes[node].mask & SCISSOR, 0);
        // Far behind the doorway plane, the scissor bit stays.
        scene.set_frame(
            node,
            Vec3::new(5.0, 1.0, 0.0),
            Mat3::IDENTITY,
            Vec3::new(0.4, 0.4, 0.4),
        );
        scene.update_cell(node);
        scene.sectors[2].views = vec![box_view(200.0)];
        scene.update_visibility(16.0);
        assert_eq!(scene.nodes[node].mask & SCISSOR, SCISSOR);
        assert_eq!(
            scene.nodes[node].mask & (MAIN | SEEN),
            MAIN | SEEN,
            "the node is still inside the portal frustum"
        );
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
}
