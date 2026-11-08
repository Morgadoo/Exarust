//! What the room manager shows each frame, in the Bevy world: the rooms the
//! anchor (the player) can see are drawn, their door neighbours are drawn
//! under their black lids, every other room is not drawn at all, and rooms
//! fading in or out get their lid as a black overlay. The occluders filling
//! the wall hollows of every room not hidden are always drawn. Rules and
//! state: [`crate::rooms`]; `docs/RENDERING.md` (Rooms and black fill).
//!
//! Only meshes are hidden: lights, GI emitters and shadow links stay as
//! they are, and so does collision.
//!
//! Doors: each placed object of the door kind is bound at load to the door
//! link its position and facing select, with the body of its object node's
//! first child (the leaf), as the original's level loader does
//! ([`crate::rooms::DoorBodies::bind`]). Every frame each bound link takes
//! its open state from that body's pair flag ([`crate::rooms::link_open`]),
//! so a shut door keeps the room behind it from turning active: that room
//! stays adjacent, under its black lid.

use std::sync::Arc;

use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::VisibilitySystems;
use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use exa_physics::body::{DoorBody, Ragdoll, SlideBody};
use exa_render::cube_shadows::{CubeLid, CubeShadow};
use exa_render::shell::ShellMaterial;

use crate::grid::{ShellStrips, ray_blocks};
use crate::rooms::{
    DoorBodies, DoorLink, DoorPortal, Fade, RoomState, RoomTracker, Rooms, link_open,
};

pub struct RoomViewPlugin;

impl Plugin for RoomViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoomView>()
            .init_resource::<ForcedRoom>()
            .init_resource::<DoorToggle>()
            .add_systems(Update, toggle_nearest_door)
            .add_systems(
                PostUpdate,
                (assign_static_rooms, inherit_rooms)
                    .after(TransformSystems::Propagate)
                    .after(VisibilitySystems::CalculateBounds),
            )
            .add_systems(
                PostUpdate,
                (
                    bind_placed_doors,
                    follow_door_bodies,
                    give_shell_materials,
                    track_rooms,
                )
                    .chain()
                    .after(TransformSystems::Propagate)
                    .before(VisibilitySystems::VisibilityPropagate),
            )
            .add_systems(
                PostUpdate,
                give_lights_lids
                    .after(TransformSystems::Propagate)
                    .before(exa_render::lights::LightsSystems::Cubes),
            );
    }
}

/// The level's rooms and which of them show.
#[derive(Resource, Clone, Debug)]
pub struct LevelRooms {
    pub rooms: Arc<Rooms>,
    pub tracker: RoomTracker,
    /// The door body bound to each door link (entity bits of the leaf).
    pub doors: DoorBodies,
    /// Each room's occluder and lid triangles (world, metres), once the
    /// grid has built them.
    pub shells: Arc<Vec<ShellStrips>>,
    /// Every room's door portals, one per door link in the rooms' link
    /// order ([`Rooms::portal`]); empty until the level builder sets them
    /// from its grid.
    pub portals: Arc<Vec<DoorPortal>>,
}

impl LevelRooms {
    pub fn new(rooms: Arc<Rooms>) -> Self {
        let tracker = RoomTracker::new(rooms.len());
        Self {
            rooms,
            tracker,
            doors: DoorBodies::new(),
            shells: Arc::default(),
            portals: Arc::default(),
        }
    }

    /// The room manager's shell ray query on the rooms active now (the
    /// tracker's [`RoomTracker::is_active`]): whether a ray from `origin`
    /// along `dir` (world, metres) meets the occluder (`lid` false) or the
    /// lid (`lid` true) of one of them before `max_t` lengths of `dir`
    /// ([`ray_blocks`]). Before the shells exist nothing blocks.
    pub fn shell_ray(&self, origin: Vec3, dir: Vec3, max_t: f32, lid: bool) -> bool {
        !self.shells.is_empty()
            && ray_blocks(
                &self.shells,
                |room| self.tracker.is_active(room),
                origin,
                dir,
                max_t,
                lid,
            )
    }
}

/// A placed door object waiting to be bound to its door link: the first
/// child of its object node (the node whose body the original binds), its
/// position (metres, X/Z) and the X and Z of its third matrix row.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlacedDoor {
    pub leaf: Entity,
    pub position: Vec2,
    pub facing: Vec2,
}

/// On a bound door's leaf node: the door link it stands in.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundDoor(pub DoorLink);

/// Binds each waiting placed door to the link its cell and facing select,
/// once the level's rooms exist. A level without a grid binds nothing.
fn bind_placed_doors(
    mut commands: Commands,
    level: Option<ResMut<LevelRooms>>,
    placed: Query<(Entity, &PlacedDoor)>,
) {
    if placed.is_empty() {
        return;
    }
    let mut level = level;
    let (mut bound, mut waiting) = (0, 0);
    for (entity, door) in &placed {
        waiting += 1;
        if let Some(level) = level.as_deref_mut() {
            let LevelRooms { rooms, doors, .. } = level;
            if let Some(link) = doors.bind(rooms, door.position, door.facing, door.leaf.to_bits()) {
                commands.entity(door.leaf).try_insert(BoundDoor(link));
                bound += 1;
            }
        }
        commands.entity(entity).remove::<PlacedDoor>();
    }
    debug!("door binding: {bound} of {waiting} placed doors bound to a link");
}

/// Debug (`--toggle-door FRAME`): that many frames after the level's rooms
/// and the anchor exist, the bound hinged leaf whose door point is nearest
/// the anchor gets the door's own toggle (the command a lever's signal
/// gives: a shut leaf is driven open) and is woken; its angle and its link's
/// state are then logged every [`DOOR_LOG_FRAMES`] frames, so a check can
/// watch the room behind a door appear as it opens.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DoorToggle {
    pub at_frame: Option<u32>,
}

/// How often the debug toggle logs its leaf.
pub const DOOR_LOG_FRAMES: u32 = 60;

/// The debug toggle's frame count and the leaf it watches.
#[derive(Default)]
struct ToggleState {
    frames: u32,
    leaf: Option<Entity>,
    /// The frame the toggle landed on.
    toggled_at: u32,
    /// "No leaf yet" was logged.
    waiting: bool,
}

fn toggle_nearest_door(
    toggle: Res<DoorToggle>,
    level: Option<Res<LevelRooms>>,
    anchors: Query<&GlobalTransform, With<RoomAnchor>>,
    leaves: Query<(Entity, &BoundDoor, &DoorBody)>,
    mut state: Local<ToggleState>,
    mut fixed: ResMut<exa_physics::sim_app::FixedCommands>,
) {
    let Some(at_frame) = toggle.at_frame else {
        return;
    };
    let (Some(level), Some(anchor)) = (level, anchors.iter().next()) else {
        return;
    };
    state.frames += 1;
    let feet = anchor.translation();
    let rooms = &level.rooms;
    // From the asked frame on, the first frame a bound hinged leaf exists
    // (door bodies register with the level's other bodies, which can be
    // well after the rooms exist).
    if state.frames >= at_frame && state.leaf.is_none() {
        // The leaf whose link's door point (1.5 m inside its cell) is
        // nearest the anchor.
        let nearest = leaves
            .iter()
            .map(|(e, BoundDoor(link), _)| {
                let own = rooms.cell_centre(link.from);
                let point = own + (rooms.cell_centre(link.to) - own) * 0.25;
                (e, point.distance_squared(Vec2::new(feet.x, feet.z)))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((leaf, d2)) = nearest
            && let Ok((_, BoundDoor(link), _)) = leaves.get(leaf)
        {
            // The toggle writes to the leaf's body: it lands at the start
            // of the next fixed step.
            fixed.run(toggle_door, leaf);
            state.leaf = Some(leaf);
            state.toggled_at = state.frames;
            info!(
                "door toggle at frame {}: leaf {leaf}, door point {:.2} m away, link {}->{}, open {}",
                state.frames,
                d2.sqrt(),
                link.from,
                link.to,
                level.tracker.is_open(*link)
            );
        } else if !state.waiting {
            state.waiting = true;
            info!("door toggle: no bound hinged leaf yet; waiting for one");
        }
    }
    let Some(leaf) = state.leaf else {
        return;
    };
    if state
        .frames
        .saturating_sub(state.toggled_at)
        .is_multiple_of(DOOR_LOG_FRAMES)
        && let Ok((_, BoundDoor(link), glue)) = leaves.get(leaf)
    {
        info!(
            "door toggle: angle {:.3} (shut at {:.3}), pair flag {}, link open {}",
            glue.door.angle,
            glue.door.shape.closed(),
            glue.door.both_passable,
            level.tracker.is_open(*link)
        );
    }
}

/// `--toggle-door`'s toggle of `leaf` (run at the start of a fixed step).
fn toggle_door(In(leaf): In<Entity>, mut leaves: Query<(&mut DoorBody, &mut Ragdoll)>) {
    if let Ok((mut glue, mut body)) = leaves.get_mut(leaf) {
        glue.door.toggle();
        body.wake_to(exa_physics::body::WAKE_COUNT);
    }
}

/// Gives each bound link the open state of its door body, both directions.
/// A leaf without a door body (a static leaf) leaves its link open, as a
/// link without a body is in the original; a link a later door was bound
/// to follows that door only.
fn follow_door_bodies(
    level: Option<ResMut<LevelRooms>>,
    leaves: Query<(Entity, &BoundDoor, Option<&DoorBody>, Option<&SlideBody>)>,
) {
    let Some(mut level) = level else {
        return;
    };
    for (entity, BoundDoor(link), hinged, sliding) in &leaves {
        if level.doors.body(*link) != Some(entity.to_bits()) {
            continue;
        }
        let flag = hinged
            .map(|g| g.door.both_passable)
            .or(sliding.map(|g| g.door.both_passable));
        let open = link_open(flag);
        if level.tracker.is_open(*link) != open {
            level.tracker.set_door(*link, open);
            debug!(
                "door link {}->{} {}",
                link.from,
                link.to,
                if open { "opens" } else { "shuts" }
            );
        }
    }
}

/// How rooms are shown.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct RoomView {
    /// Every room active, no lids, no fades (a debug view of the whole
    /// level).
    pub all: bool,
}

/// The room walk's forced-room branch. While `active` (the view follows no
/// player: the Spectate mode an arena match enters when no entrant is the
/// player's), the walk ignores the [`RoomAnchor`] and takes the room at
/// `at` (metres, X/Z: the arena's spawn spot, which the original's match
/// start turns into its stored room), alone; with no room there (or no
/// point) nothing changes, as the original returns when its stored room is
/// none. The game sets it from its spectate session.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ForcedRoom {
    pub active: bool,
    pub at: Option<Vec2>,
}

/// The point the active rooms follow: the player, as the original follows
/// its camera's target.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RoomAnchor;

/// A mesh drawn only while its room is not hidden.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InRoom(pub u16);

/// A static root whose meshes belong to a room (none: always drawn);
/// meshes added under it later join the room.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomRoot(pub Option<u16>);

/// A static root (a prop, a placed object, the level's model) waiting to be
/// given the room of its bounds' centre, as the original sorts a level's
/// nodes into rooms.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PendingRoom {
    /// Each child is a root of its own (the level model's top nodes).
    pub split: bool,
    /// Frames waited for its meshes' bounds.
    pub waited: u8,
}

/// A moving root (a character), drawn only while the room at its position
/// is not hidden.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RoomTracked;

/// One of a room's black shells.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomShell {
    pub room: u16,
    pub kind: ShellKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellKind {
    /// The `[…` boxes in the room's wall hollows: drawn while the room is
    /// not hidden.
    Occluder,
    /// The room's ceiling strips: drawn black for a room not hidden that
    /// is not fading (an adjacent room).
    Lid,
    /// The lid again, drawn last as a black overlay while the room fades.
    Overlay,
}

/// Frames a root waits for its meshes' bounds before it is placed by its
/// own position.
const BOUNDS_WAIT: u8 = 8;

/// The world-space bounds of `root`'s meshes, or `None` while a mesh has
/// none yet.
fn mesh_bounds(
    root: Entity,
    children: &Query<&Children>,
    meshes: &Query<(Option<&Aabb>, &GlobalTransform), With<Mesh3d>>,
) -> Option<Option<(Vec3, Vec3)>> {
    let mut lo = Vec3A::splat(f32::MAX);
    let mut hi = Vec3A::splat(f32::MIN);
    let mut any = false;
    for e in std::iter::once(root).chain(children.iter_descendants(root)) {
        let Ok((aabb, transform)) = meshes.get(e) else {
            continue;
        };
        let aabb = aabb?;
        let affine = transform.affine();
        let centre = affine.transform_point3a(aabb.center);
        let m = affine.matrix3;
        let half = m.x_axis.abs() * aabb.half_extents.x
            + m.y_axis.abs() * aabb.half_extents.y
            + m.z_axis.abs() * aabb.half_extents.z;
        lo = lo.min(centre - half);
        hi = hi.max(centre + half);
        any = true;
    }
    Some(any.then(|| (lo.into(), hi.into())))
}

/// Gives each waiting static root the room of its meshes' bounds centre
/// (its own position without meshes), and its meshes that room.
fn assign_static_rooms(
    mut commands: Commands,
    level: Option<Res<LevelRooms>>,
    mut pending: Query<(Entity, &mut PendingRoom)>,
    children: Query<&Children>,
    meshes: Query<(Option<&Aabb>, &GlobalTransform), With<Mesh3d>>,
    transforms: Query<&GlobalTransform>,
) {
    for (entity, mut wait) in &mut pending {
        let Some(level) = &level else {
            commands.entity(entity).remove::<PendingRoom>();
            continue;
        };
        let roots: Vec<Entity> = if wait.split {
            children.get(entity).map(|c| c.to_vec()).unwrap_or_default()
        } else {
            vec![entity]
        };
        let bounds: Vec<Option<Option<(Vec3, Vec3)>>> = roots
            .iter()
            .map(|&r| mesh_bounds(r, &children, &meshes))
            .collect();
        if bounds.iter().any(Option::is_none) && wait.waited < BOUNDS_WAIT {
            wait.waited += 1;
            continue;
        }
        for (&root, bounds) in roots.iter().zip(bounds) {
            let centre = match bounds.flatten() {
                Some((lo, hi)) => (lo + hi) * 0.5,
                None => transforms.get(root).map_or(Vec3::ZERO, |t| t.translation()),
            };
            let room = level.rooms.room_of_node(Vec2::new(centre.x, centre.z));
            commands.entity(root).insert(RoomRoot(room));
            if let Some(room) = room {
                for e in std::iter::once(root).chain(children.iter_descendants(root)) {
                    if meshes.contains(e) {
                        commands.entity(e).insert(InRoom(room));
                    }
                }
            }
        }
        commands.entity(entity).remove::<PendingRoom>();
    }
}

/// A mesh added this frame without a room.
type AddedMesh = (Added<Mesh3d>, Without<InRoom>, Without<RoomShell>);

/// Meshes added under a root that has its room (a fire's flames, say) join
/// that room.
fn inherit_rooms(
    mut commands: Commands,
    added: Query<Entity, AddedMesh>,
    parents: Query<&ChildOf>,
    roots: Query<&RoomRoot>,
) {
    for e in &added {
        let mut at = e;
        while let Ok(parent) = parents.get(at) {
            at = parent.parent();
            if let Ok(RoomRoot(room)) = roots.get(at) {
                if let Some(room) = room {
                    commands.entity(e).insert(InRoom(*room));
                }
                break;
            }
        }
    }
}

/// Each light's cube takes the lid of the room holding it, as the original's
/// room manager draws, into every face rendered for an omni light, the
/// ceiling shell of the room at the light's position (the world-point query,
/// [`Rooms::room_at`]); a light outside every room, or in a room without a
/// lid, takes none. A light whose lid changes (it moved to another room, or
/// the shells arrived) has its faces put out of date.
fn give_lights_lids(
    mut commands: Commands,
    level: Option<Res<LevelRooms>>,
    shells: Query<(&RoomShell, &Mesh3d)>,
    added: Query<(), Added<RoomShell>>,
    mut lights: Query<(
        Entity,
        Ref<GlobalTransform>,
        Option<&CubeLid>,
        &mut CubeShadow,
    )>,
    mut lids: Local<Vec<Option<Handle<Mesh>>>>,
) {
    let Some(level) = level else {
        if !lids.is_empty() {
            lids.clear();
            for (entity, _, lid, mut cube) in &mut lights {
                if lid.is_some_and(|l| l.0.is_some()) {
                    commands.entity(entity).insert(CubeLid(None));
                    cube.invalidate();
                }
            }
        }
        return;
    };
    let fresh = !added.is_empty() || level.is_added();
    if fresh {
        *lids = vec![None; level.rooms.len()];
        for (shell, mesh) in &shells {
            if shell.kind == ShellKind::Lid
                && let Some(slot) = lids.get_mut(shell.room as usize)
            {
                *slot = Some(mesh.0.clone());
            }
        }
    }
    let (mut lidded, mut changed) = (0, 0);
    for (entity, at, lid, mut cube) in &mut lights {
        if !(fresh || at.is_changed() || lid.is_none()) {
            continue;
        }
        let p = at.translation();
        let want = level
            .rooms
            .room_at(Vec2::new(p.x, p.z))
            .and_then(|room| lids.get(room as usize).cloned().flatten());
        if lid.map(|l| &l.0) != Some(&want) {
            changed += 1;
            lidded += usize::from(want.is_some());
            if lid.is_some_and(|l| l.0 != want) || want.is_some() {
                cube.invalidate();
            }
            commands.entity(entity).insert(CubeLid(want));
        }
    }
    if changed > 0 {
        info!(
            "cube lids: {changed} lights took a new lid, {lidded} of them a room's ({} rooms lidded)",
            lids.iter().flatten().count()
        );
    }
}

/// Shells get their material: one opaque black for occluders and lids, one
/// of its own for each overlay (its alpha changes as it fades).
fn give_shell_materials(
    mut commands: Commands,
    added: Query<(Entity, &RoomShell), Without<MeshMaterial3d<ShellMaterial>>>,
    materials: Option<ResMut<Assets<ShellMaterial>>>,
    mut black: Local<Option<Handle<ShellMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    for (e, shell) in &added {
        let material = match shell.kind {
            ShellKind::Overlay => materials.add(ShellMaterial::overlay(0.0)),
            _ => black
                .get_or_insert_with(|| materials.add(ShellMaterial::black()))
                .clone(),
        };
        commands.entity(e).insert(MeshMaterial3d(material));
    }
}

type MemberQuery<'w, 's> = Query<
    'w,
    's,
    (Ref<'static, InRoom>, &'static mut Visibility),
    (Without<RoomShell>, Without<RoomTracked>),
>;
type TrackedQuery<'w, 's> = Query<
    'w,
    's,
    (&'static GlobalTransform, &'static mut Visibility),
    (With<RoomTracked>, Without<InRoom>, Without<RoomShell>),
>;
type ShellQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static RoomShell,
        &'static mut Visibility,
        Option<&'static MeshMaterial3d<ShellMaterial>>,
    ),
    (Without<InRoom>, Without<RoomTracked>),
>;

/// Moves the active rooms with the anchor and shows what each room's state
/// asks for.
#[allow(clippy::too_many_arguments)]
pub(crate) fn track_rooms(
    time: Res<Time<Real>>,
    view: Res<RoomView>,
    level: Option<ResMut<LevelRooms>>,
    forced: Res<ForcedRoom>,
    anchors: Query<&GlobalTransform, With<RoomAnchor>>,
    mut members: MemberQuery,
    mut tracked: TrackedQuery,
    mut shells: ShellQuery,
    materials: Option<ResMut<Assets<ShellMaterial>>>,
    mut drawn: Local<Vec<bool>>,
    mut last_active: Local<Vec<u16>>,
) {
    let Some(mut level) = level else {
        drawn.clear();
        return;
    };
    let now = time.elapsed().as_millis() as u64;
    let LevelRooms { rooms, tracker, .. } = &mut *level;
    if view.all {
        tracker.show_all();
    } else if forced.active {
        if let Some(room) = forced.at.and_then(|at| rooms.room_at(at)) {
            tracker.update_forced(rooms, room, now);
            if tracker.active() != last_active.as_slice() {
                *last_active = tracker.active().to_vec();
                debug!("active rooms {:?} (forced room {room})", tracker.active());
            }
        }
    } else if let Some(anchor) = anchors.iter().next() {
        let p = anchor.translation();
        tracker.update(rooms, Vec2::new(p.x, p.z), now);
        if tracker.active() != last_active.as_slice() {
            *last_active = tracker.active().to_vec();
            let at = Vec2::new(p.x, p.z);
            // Each door of the anchor's room: the room behind, whether it is
            // open, and its door point (1.5 m inside the cell).
            let doors: Vec<(u16, bool, [i32; 2])> = rooms
                .room_at(at)
                .and_then(|r| rooms.room(r))
                .map(|r| {
                    r.links
                        .iter()
                        .filter_map(|l| {
                            let own = rooms.cell_centre(l.from);
                            let point = own + (rooms.cell_centre(l.to) - own) * 0.25;
                            Some((
                                rooms.room_of_index(l.to)?,
                                tracker.is_open(*l),
                                [(point.x * 10.0) as i32, (point.y * 10.0) as i32],
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default();
            debug!(
                "active rooms {:?} (anchor at {:.2}, {:.2}, room {:?}; its doors (room, open, door point dm) {:?})",
                tracker.active(),
                p.x,
                p.z,
                rooms.room_at(at),
                doors
            );
        }
    }
    let shown: Vec<bool> = (0..rooms.len())
        .map(|r| tracker.state(r as u16) != RoomState::Hidden)
        .collect();
    let changed = *drawn != shown;
    let visibility = |room: Option<u16>| match room {
        Some(r) if !shown.get(r as usize).copied().unwrap_or(true) => Visibility::Hidden,
        _ => Visibility::Inherited,
    };
    for (room, mut v) in &mut members {
        if changed || room.is_added() {
            v.set_if_neq(visibility(Some(room.0)));
        }
    }
    for (transform, mut v) in &mut tracked {
        let p = transform.translation();
        v.set_if_neq(visibility(rooms.room_of_node(Vec2::new(p.x, p.z))));
    }
    let mut materials = materials;
    for (shell, mut v, material) in &mut shells {
        let state = tracker.state(shell.room);
        let show = match shell.kind {
            ShellKind::Occluder => state != RoomState::Hidden,
            ShellKind::Lid => {
                !view.all && state != RoomState::Hidden && tracker.fade(shell.room) == Fade::None
            }
            ShellKind::Overlay => {
                let alpha = tracker.overlay(shell.room, now);
                if let (Some(alpha), Some(material), Some(materials)) =
                    (alpha, material, materials.as_deref_mut())
                    && materials
                        .get(&material.0)
                        .is_some_and(|m| m.colour.w != alpha)
                    && let Some(mut m) = materials.get_mut(&material.0)
                {
                    m.colour.w = alpha;
                }
                alpha.is_some()
            }
        };
        v.set_if_neq(if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    *drawn = shown;
}

#[cfg(test)]
mod tests {
    use exa_formats::level::LevelGrid;

    use super::*;
    use crate::grid::Strip;
    use crate::rooms::CELL_OPEN;

    /// Two rooms of two cells each along X with a door between: cells 1–2
    /// (x −12…0 m) and 3–4 (x 0…12 m), row 1 (z 0…6 m).
    fn two_rooms() -> Rooms {
        let mut grid = LevelGrid {
            wall_sets: vec!["GEWS_UWA01".into()],
            floor_sets: vec!["GEFS_UWA01".into()],
            width: 6,
            height: 3,
            edges: vec![0; 18],
            plane_b: vec![0; 18],
            plane_c: vec![0; 18],
        };
        for (x, sides) in [
            (1, [7, 7, 7, 0]),
            (2, [7, 0, 7, 0xB]),
            (3, [7, 0xB, 7, 0]),
            (4, [7, 0, 7, 7]),
        ] {
            grid.edges[6 + x] =
                CELL_OPEN | sides[0] | sides[1] << 4 | sides[2] << 8 | sides[3] << 12;
        }
        Rooms::flood(&grid)
    }

    /// A lid triangle at 4 m over x −12…0, z 0…12, facing up.
    fn lid() -> Strip {
        Strip {
            positions: vec![[-12.0, 4.0, 0.0], [-12.0, 4.0, 12.0], [0.0, 4.0, 0.0]],
            indices: vec![0, 1, 2],
        }
    }

    /// The shell query asks only the rooms active now, and the lid or the
    /// occluder as asked; an active room without that shell ends it.
    #[test]
    fn the_shell_ray_asks_the_active_rooms() {
        let rooms = Arc::new(two_rooms());
        let west = rooms.room_at(Vec2::new(-9.0, 3.0)).unwrap();
        let mut level = LevelRooms::new(rooms.clone());
        let mut shells = vec![ShellStrips::default(); rooms.len()];
        shells[west as usize].lid = lid();
        shells[west as usize].occluder = lid();
        level.shells = Arc::new(shells);
        let (from, down) = (Vec3::new(-9.0, 10.0, 3.0), Vec3::NEG_Y);
        // Not active yet: nothing blocks.
        assert!(!level.shell_ray(from, down, 10.0, true));
        level.tracker.update(&rooms, Vec2::new(-9.0, 3.0), 0);
        level.tracker.update(&rooms, Vec2::new(-9.0, 3.0), 2000);
        assert!(level.tracker.is_active(west));
        assert!(level.shell_ray(from, down, 10.0, true));
        assert!(level.shell_ray(from, down, 10.0, false));
        // Short of the lid, or from below (its back), the ray passes.
        assert!(!level.shell_ray(from, down, 5.0, true));
        assert!(!level.shell_ray(Vec3::new(-9.0, 1.0, 3.0), Vec3::Y, 10.0, true));
    }

    /// A light takes the lid of the room at its position, and none outside
    /// a lidded room.
    #[test]
    fn a_light_takes_its_rooms_lid() {
        let rooms = Arc::new(two_rooms());
        let west = rooms.room_at(Vec2::new(-9.0, 3.0)).unwrap();
        let mut world = World::new();
        let mut meshes = Assets::<Mesh>::default();
        let mesh = meshes.add(lid().mesh().unwrap());
        world.insert_resource(LevelRooms::new(rooms));
        world.spawn((
            RoomShell {
                room: west,
                kind: ShellKind::Lid,
            },
            Mesh3d(mesh.clone()),
        ));
        let light = world
            .spawn((
                GlobalTransform::from_translation(Vec3::new(-9.0, 2.0, 3.0)),
                CubeShadow::default(),
            ))
            .id();
        let mut schedule = Schedule::default();
        schedule.add_systems(give_lights_lids);
        schedule.run(&mut world);
        assert_eq!(world.get::<CubeLid>(light), Some(&CubeLid(Some(mesh))));
        *world.get_mut::<GlobalTransform>(light).unwrap() =
            GlobalTransform::from_translation(Vec3::new(3.0, 2.0, 3.0));
        schedule.run(&mut world);
        assert_eq!(world.get::<CubeLid>(light), Some(&CubeLid(None)));
    }
}
