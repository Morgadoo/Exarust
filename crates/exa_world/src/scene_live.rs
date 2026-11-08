//! The scene-node unit on the live path: a [`Scene`] kept for the loaded
//! tile-grid level, with every drawn mesh registered in its sector, and
//! the original's per-frame scene update deciding what the main view draws
//! and in which sector order the renderer walks the blended surfaces.
//!
//! - **Sectors.** The scene's own sector comes first; then one sector per
//!   room of the room flood fill, appended in room order through the batch
//!   insertion ([`Scene::add_batch`]), as the original's room manager
//!   appends its room sectors. Each room sector keeps the doorways of its
//!   door links ([`crate::rooms::Rooms::portal`]: the quad on the cells'
//!   shared edge from −50 cm to the opening's top), facing the room behind
//!   them, and its linked sectors.
//! - **Membership.** Every mesh entity of the main world (not a room's
//!   black shell, not on an off-screen layer) is a mesh node. A mesh with a
//!   room ([`InRoom`]) is registered in that room's sector, a mesh under a
//!   moving root ([`RoomTracked`], the characters) in the sector of the
//!   room at the root's position, anything else in the scene's own sector.
//!   When a mesh's room changes it moves sector ([`Scene::move_to_sector`]),
//!   so the old sector's lists drop it at the next membership rebuild; a
//!   despawned mesh is queued for removal ([`Scene::queue_removal`]) and
//!   leaves at the frame's end. Node slots are reused.
//! - **Frames and bounds.** A mesh's node takes its world box (the mesh's
//!   bounds centre, rotation and scaled half extents) whenever its
//!   transform or bounds change; a sector whose meshes changed rebuilds its
//!   bounds, and the scene bounds follow ([`Scene::rebuild_bounds`] over
//!   the changed sectors). A mesh whose material casts shadows bounds its
//!   sector's first box ([`BOUNDS_SPLIT`]), as the original's mesh
//!   preparation sets the bit when one of its parts' materials casts.
//! - **Frame.** Each frame the room states go to the sectors and
//!   [`Scene::frame`] runs with the main camera's view volume: active
//!   sectors against the view, adjacent sectors through their doorways to
//!   active ones, the membership rebuild, the hidden and visible passes and
//!   the removal queue.
//! - **What it drives.** A registered mesh the visible pass leaves undrawn (no main,
//!   dithered or partial bit) is taken out of the main camera's visible
//!   list, as the original draws only entries whose word meets the
//!   renderer's mask; shadow views are not touched (the original's shadow
//!   maps have their own lists). Meshes Bevy must never cull
//!   ([`NoFrustumCulling`]) stay. And each mesh's sector goes to
//!   [`DrawSectors`], which orders the transparent phase sector by sector
//!   (`exa_render::sector_walk`).
//!
//! Levels without a grid have no room sectors; the original's frame walks a
//! sectorless scene as one list, and nothing here runs for them.
//! `--no-sector-cull` keeps the scene but leaves the main view's list as
//! Bevy made it.

use std::sync::Arc;

use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{
    NoFrustumCulling, RenderLayers, VisibilitySystems, VisibleEntities,
};
use bevy::camera::{CameraProjection, Projection};
use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
use bevy::light::NotShadowCaster;
use bevy::math::{Mat3, Vec2, Vec3};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use exa_physics::intersect::ConvexVolume;
use exa_render::sector_walk::DrawSectors;
use exa_render::sun_fit::frustum_volume;
use exa_render::supersample::SceneCamera;

use crate::room_view::{InRoom, LevelRooms, RoomShell, RoomTracked};
use crate::rooms::{CELL_M, DoorPortal, PORTAL_BOTTOM_CM, Rooms};
use crate::scene_nodes::{BOUNDS_SPLIT, Draw, NodeKind, Portal, SCENE_SECTOR, Scene, SectorState};

pub struct SceneLivePlugin;

impl Plugin for SceneLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LiveScene>()
            .init_resource::<SectorCull>()
            .add_systems(
                PostUpdate,
                run_scene
                    .after(TransformSystems::Propagate)
                    .after(VisibilitySystems::CalculateBounds)
                    .after(VisibilitySystems::UpdateFrusta)
                    .after(crate::room_view::track_rooms)
                    .before(VisibilitySystems::CheckVisibility),
            )
            .add_systems(
                PostUpdate,
                cull_undrawn
                    .after(VisibilitySystems::CheckVisibility)
                    .before(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible),
            );
    }
}

/// Whether the visible pass's answer trims the main camera's visible list
/// (`--no-sector-cull` turns it off, to compare).
#[derive(Resource, Clone, Copy, Debug)]
pub struct SectorCull(pub bool);

impl Default for SectorCull {
    fn default() -> Self {
        Self(true)
    }
}

/// The live scene of the loaded grid level.
#[derive(Resource, Default)]
pub struct LiveScene {
    pub scene: Option<Scene>,
    /// The rooms the scene was built for (a new level replaces them).
    rooms: Option<Arc<Rooms>>,
    node_of: EntityHashMap<usize>,
    entity_of: Vec<Option<Entity>>,
    free: Vec<usize>,
    /// Slots of meshes taken out this frame, freed after it.
    freeing: Vec<usize>,
    /// The moving root each tracked mesh follows.
    tracked: EntityHashMap<Entity>,
    /// Each moving root's sector and its meshes.
    roots: EntityHashMap<(usize, Vec<Entity>)>,
    /// Meshes Bevy must never cull.
    exempt: EntityHashSet,
    /// Meshes the last visible pass drew (in a sector it drew).
    pub drawn: EntityHashSet,
    /// The camera the scene was classified for.
    camera: Option<Entity>,
    /// Sectors whose bounds need rebuilding.
    dirty: Vec<bool>,
    /// The sectors' memberships changed since [`DrawSectors`] was written.
    sectors_changed: bool,
    /// Frames run, for the periodic log line.
    frames: u64,
    /// Meshes the last cull took out of the main view's list, and that
    /// list's length after it.
    culled: (usize, usize),
    /// Since the last log line: frames whose cull took a mesh out, and the
    /// most it took in one frame.
    culled_frames: u32,
    culled_most: usize,
    /// Time spent (ms) since the last log line: bookkeeping, the scene's
    /// frame, the outputs; and the frames counted.
    timing: [f32; 3],
    timing_frames: u32,
    /// Frames refreshed since the last log line.
    refreshed: usize,
}

impl LiveScene {
    /// The number of meshes registered in each sector (the scene's own
    /// first), for logs and tests.
    pub fn sector_counts(&self) -> Vec<usize> {
        self.scene
            .as_ref()
            .map(|s| s.sectors.iter().map(|s| s.meshes.len()).collect())
            .unwrap_or_default()
    }

    /// The sector a mesh entity is registered in.
    pub fn sector_of(&self, entity: Entity) -> Option<usize> {
        let node = *self.node_of.get(&entity)?;
        self.scene.as_ref()?.nodes[node].sector
    }

    fn reset(&mut self) {
        *self = Self {
            drawn: EntityHashSet::default(),
            ..Self::default()
        };
    }

    fn mark(&mut self, sector: Option<usize>) {
        if let Some(s) = sector
            && let Some(d) = self.dirty.get_mut(s)
        {
            *d = true;
        }
        self.sectors_changed = true;
    }
}

/// A door portal as the visibility pass sees it from the room it leads
/// out of: the quad on the shared edge from −50 cm up to the opening's
/// top, facing the room behind (where the eye must be), joining the two
/// cells, leading to that room's sector.
pub fn sector_portal(rooms: &Rooms, portal: &DoorPortal) -> Option<Portal> {
    let behind = rooms.room_of_index(portal.link.to)?;
    let bottom = PORTAL_BOTTOM_CM * 0.01;
    let mut out = Portal::between_cells(
        portal.link.to as usize,
        portal.link.from as usize,
        rooms.cell_centre(portal.link.to),
        rooms.cell_centre(portal.link.from),
        Vec2::new(portal.size.x * 2.0, portal.size.y - bottom),
        bottom,
    );
    debug_assert!((out.quad[0].xz() - portal.centre).length() <= CELL_M);
    out.neighbour = usize::from(behind) + 1;
    Some(out)
}

/// A scene with the level's rooms appended as sectors, each with its
/// doorways and linked sectors.
pub fn build_scene(rooms: &Rooms, portals: &[DoorPortal]) -> Scene {
    let mut scene = Scene::new((rooms.width() as usize, rooms.height() as usize));
    scene.quiet_passes = true;
    let roots: Vec<usize> = (0..rooms.len())
        .map(|_| scene.add_node(NodeKind::Sector, None))
        .collect();
    scene.add_batch(&roots);
    for portal in portals {
        let Some(room) = rooms.room_of_index(portal.link.from) else {
            continue;
        };
        let Some(entry) = sector_portal(rooms, portal) else {
            continue;
        };
        let sector = usize::from(room) + 1;
        if !scene.sectors[sector].linked.contains(&entry.neighbour) {
            scene.sectors[sector].linked.push(entry.neighbour);
        }
        scene.add_portal(sector, entry);
    }
    scene
}

/// The node frame of a mesh: its bounds centre in the world, its rotation
/// and its half extents scaled by the transform.
fn mesh_frame(transform: &GlobalTransform, aabb: &Aabb) -> (Vec3, Mat3, Vec3) {
    let affine = transform.affine();
    let centre = Vec3::from(affine.transform_point3a(aabb.center));
    let m = affine.matrix3;
    let scale = Vec3::new(m.x_axis.length(), m.y_axis.length(), m.z_axis.length());
    let unit = |v: bevy::math::Vec3A, s: f32| {
        if s > 0.0 {
            Vec3::from(v) / s
        } else {
            Vec3::ZERO
        }
    };
    let rotation = Mat3::from_cols(
        unit(m.x_axis, scale.x),
        unit(m.y_axis, scale.y),
        unit(m.z_axis, scale.z),
    );
    (centre, rotation, Vec3::from(aabb.half_extents) * scale)
}

/// The moving root a mesh follows, if it hangs under one.
fn tracked_root(
    entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &Query<(), With<RoomTracked>>,
) -> Option<Entity> {
    let mut at = entity;
    for _ in 0..64 {
        if roots.contains(at) {
            return Some(at);
        }
        at = parents.get(at).ok()?.parent();
    }
    None
}

type MeshQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        &'static Aabb,
        Option<&'static InRoom>,
        Has<NoFrustumCulling>,
        Has<NotShadowCaster>,
        Option<&'static RenderLayers>,
    ),
    (With<Mesh3d>, Without<RoomShell>),
>;
type MovedQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static GlobalTransform, &'static Aabb),
    (With<Mesh3d>, Or<(Changed<GlobalTransform>, Changed<Aabb>)>),
>;

/// The scene's per-frame update: registration, moves, removals, frames and
/// bounds, then the original's frame with the room states and the main
/// camera.
#[allow(clippy::too_many_arguments)]
fn run_scene(
    mut live: ResMut<LiveScene>,
    mut draw_sectors: ResMut<DrawSectors>,
    level: Option<Res<LevelRooms>>,
    time: Res<Time<Real>>,
    meshes: MeshQuery,
    added: Query<Entity, (With<Mesh3d>, Added<Aabb>)>,
    moved: MovedQuery,
    rooms_changed: Query<(Entity, &InRoom), Changed<InRoom>>,
    mut rooms_removed: RemovedComponents<InRoom>,
    mut despawned: RemovedComponents<Mesh3d>,
    parents: Query<&ChildOf>,
    tracked_roots: Query<(), With<RoomTracked>>,
    root_transforms: Query<&GlobalTransform, With<RoomTracked>>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, &Projection), SceneCamera>,
) {
    let started = std::time::Instant::now();
    let Some(level) = level else {
        if live.scene.is_some() {
            live.reset();
            draw_sectors.map.clear();
            draw_sectors.generation += 1;
        }
        return;
    };
    let live = &mut *live;
    let fresh = !live
        .rooms
        .as_ref()
        .is_some_and(|r| Arc::ptr_eq(r, &level.rooms))
        || live.scene.as_ref().is_some_and(|s| {
            s.sectors.iter().all(|s| s.portals.is_empty()) && !level.portals.is_empty()
        });
    let mut register: Vec<Entity> = Vec::new();
    if fresh {
        live.reset();
        live.scene = Some(build_scene(&level.rooms, &level.portals));
        live.rooms = Some(level.rooms.clone());
        live.dirty = vec![true; level.rooms.len() + 1];
        register.extend(meshes.iter().map(|m| m.0));
        despawned.clear();
        rooms_removed.clear();
    } else {
        register.extend(added.iter());
    }
    let rooms = level.rooms.clone();
    let room_sector = |room: Option<u16>| room.map_or(SCENE_SECTOR, |r| usize::from(r) + 1);

    // Despawned meshes leave through the removal queue.
    for entity in despawned.read() {
        let Some(node) = live.node_of.remove(&entity) else {
            continue;
        };
        live.entity_of[node] = None;
        if let Some(root) = live.tracked.remove(&entity)
            && let Some((_, members)) = live.roots.get_mut(&root)
        {
            members.retain(|&m| m != entity);
            if members.is_empty() {
                live.roots.remove(&root);
            }
        }
        live.exempt.remove(&entity);
        let scene = live.scene.as_mut().expect("scene");
        let sector = scene.nodes[node].sector;
        if !scene.queue_removal(node, true) {
            // A full ring keeps the marking; the membership rebuild drops
            // the node and its slot is freed after the frame.
            live.freeing.push(node);
        }
        live.mark(sector);
    }

    // New meshes.
    for entity in register {
        if live.node_of.contains_key(&entity) {
            continue;
        }
        let Ok((_, transform, aabb, room, exempt, shadowless, layers)) = meshes.get(entity) else {
            continue;
        };
        if layers.is_some_and(|l| !l.intersects(&RenderLayers::default())) {
            continue;
        }
        let root = if room.is_none() {
            tracked_root(entity, &parents, &tracked_roots)
        } else {
            None
        };
        let sector = match (room, root) {
            (Some(r), _) => room_sector(Some(r.0)),
            (None, Some(root)) => room_sector(root_transforms.get(root).ok().and_then(|t| {
                let p = t.translation();
                rooms.room_of_node(Vec2::new(p.x, p.z))
            })),
            (None, None) => SCENE_SECTOR,
        };
        let scene = live.scene.as_mut().expect("scene");
        let parent = scene.sectors[sector].node;
        let node = match live.free.pop() {
            Some(slot) => scene.reuse_node(slot, NodeKind::Mesh, parent),
            None => scene.add_node(NodeKind::Mesh, parent),
        };
        if !shadowless {
            scene.nodes[node].flags |= BOUNDS_SPLIT;
        }
        let (centre, rotation, half) = mesh_frame(transform, aabb);
        scene.set_frame(node, centre, rotation, half);
        scene.refresh(node);
        scene.register(node, sector);
        if live.entity_of.len() <= node {
            live.entity_of.resize(node + 1, None);
        }
        live.entity_of[node] = Some(entity);
        live.node_of.insert(entity, node);
        if let Some(root) = root {
            live.tracked.insert(entity, root);
            live.roots
                .entry(root)
                .or_insert_with(|| (sector, Vec::new()))
                .1
                .push(entity);
        }
        if exempt {
            live.exempt.insert(entity);
        }
        live.mark(Some(sector));
    }

    // Room changes move meshes between sectors.
    let mut moves: Vec<(usize, usize)> = Vec::new();
    for (entity, room) in &rooms_changed {
        if let Some(&node) = live.node_of.get(&entity) {
            moves.push((node, room_sector(Some(room.0))));
        }
    }
    for entity in rooms_removed.read() {
        if let Some(&node) = live.node_of.get(&entity) {
            moves.push((node, SCENE_SECTOR));
        }
    }
    for (&root, (sector, members)) in live.roots.iter_mut() {
        let Ok(t) = root_transforms.get(root) else {
            continue;
        };
        let p = t.translation();
        let now = room_sector(rooms.room_of_node(Vec2::new(p.x, p.z)));
        if now == *sector {
            continue;
        }
        *sector = now;
        for entity in members.iter() {
            if let Some(&node) = live.node_of.get(entity) {
                moves.push((node, now));
            }
        }
    }
    for (node, sector) in moves {
        let scene = live.scene.as_mut().expect("scene");
        let old = scene.nodes[node].sector;
        if old == Some(sector) {
            continue;
        }
        scene.move_to_sector(node, sector);
        live.mark(old);
        live.mark(Some(sector));
    }

    // Moved meshes take their new frame.
    for (entity, transform, aabb) in &moved {
        let Some(&node) = live.node_of.get(&entity) else {
            continue;
        };
        let scene = live.scene.as_mut().expect("scene");
        let (centre, rotation, half) = mesh_frame(transform, aabb);
        let n = &scene.nodes[node];
        if n.position == centre && n.rotation == rotation && n.half_extents == half {
            continue;
        }
        scene.set_frame(node, centre, rotation, half);
        scene.refresh(node);
        live.refreshed += 1;
        // A room's first box decides whether the room meets the view; a
        // moved caster that leaves it rebuilds the room's bounds. (The
        // scene's own sector is drawn with every view whatever its box.)
        if let Some(s) = scene.nodes[node].sector
            && s != SCENE_SECTOR
            && scene.nodes[node].flags & BOUNDS_SPLIT != 0
        {
            let child = scene.nodes[node].child_box();
            let first = scene.sectors[s].bounds.first;
            let outside = (child.centre - child.extent).cmplt(first.min()).any()
                || (child.centre + child.extent).cmpgt(first.max()).any();
            if outside && let Some(d) = live.dirty.get_mut(s) {
                *d = true;
            }
        }
    }

    // Bounds of the changed sectors, then the scene's.
    let scene = live.scene.as_mut().expect("scene");
    let mut any_dirty = false;
    for (sector, dirty) in live.dirty.iter_mut().enumerate() {
        if *dirty && sector < scene.sectors.len() {
            scene.rebuild_sector_bounds(sector);
            *dirty = false;
            any_dirty = true;
        }
    }
    if any_dirty {
        let rooms_bounds: Vec<_> = scene.sectors[1..]
            .iter()
            .map(|s| (s.bounds, s.rotation))
            .collect();
        let own = scene.sectors[SCENE_SECTOR].bounds;
        scene.sectors[SCENE_SECTOR].bounds = crate::scene_nodes::scene_bounds(own, &rooms_bounds);
    }

    // The room states and the main view.
    let states: Vec<_> = (0..rooms.len())
        .map(|r| level.tracker.state(r as u16))
        .collect();
    scene.set_room_states(&states);
    // The scene's own sector is not a room: the room manager never hides
    // it, so it is drawn with every view (content in no room always is).
    scene.sectors[SCENE_SECTOR].state = SectorState::Active;
    let camera = cameras.iter().find(|(_, c, _, _)| c.is_active);
    let Some((camera_entity, camera, camera_transform, perspective)) =
        camera.and_then(|(e, c, t, p)| match p {
            Projection::Perspective(perspective) => Some((e, c, t, perspective)),
            _ => None,
        })
    else {
        // No frame this time: the refreshed frames are taken up by the
        // next full pass.
        for sector in &mut scene.sectors {
            sector.woken.clear();
            sector.quiet = false;
        }
        live.camera = None;
        return;
    };
    let corners = perspective
        .get_frustum_corners(-perspective.near, -perspective.far)
        .map(|c| camera_transform.transform_point(Vec3::from(c)));
    let view: ConvexVolume = frustum_volume(corners);
    let eye = camera_transform.translation();
    let project = |p: Vec3| {
        camera
            .world_to_viewport(camera_transform, p)
            .unwrap_or(Vec2::ZERO)
    };
    let dt_ms = time.delta().as_millis().min(i32::MAX as u128) as i32;
    let frame_started = std::time::Instant::now();
    let events = scene.frame(dt_ms, eye, std::slice::from_ref(&view), &project);
    live.camera = Some(camera_entity);

    // Slots taken out this frame are free again.
    let freeing = std::mem::take(&mut live.freeing);
    for node in events.removed.into_iter().chain(freeing) {
        if !live.free.contains(&node) {
            live.free.push(node);
        }
    }

    let frame_done = std::time::Instant::now();

    // What the visible pass draws: only meshes of the sectors it walked
    // can carry a draw bit (the hidden pass clears them).
    let scene = live.scene.as_ref().expect("scene");
    live.drawn.clear();
    for sector in scene.sectors.iter() {
        if sector.mask == 0 && sector.visible_portals.is_empty() {
            continue;
        }
        for &node in &sector.meshes {
            if scene.nodes[node].draw() != Draw::None
                && let Some(Some(entity)) = live.entity_of.get(node)
            {
                live.drawn.insert(*entity);
            }
        }
    }

    live.frames += 1;
    if live.frames % 240 == 1 {
        let portal = scene.sectors.iter().filter(|s| s.through_portal).count();
        let drawn = scene.sectors.iter().filter(|s| s.mask != 0).count();
        info!(
            "scene sectors: {} rooms, {} meshes registered, {} undrawn by the visible pass; {} sectors drawn, {} through doorways; the main view's list lost {} and kept {} (it lost meshes in {} frames since the last line, at most {})",
            scene.sectors.len() - 1,
            live.node_of.len(),
            live.node_of.len() - live.drawn.len(),
            drawn,
            portal,
            live.culled.0,
            live.culled.1,
            live.culled_frames,
            live.culled_most
        );
        live.culled_frames = 0;
        live.culled_most = 0;
        let n = live.timing_frames.max(1) as f32;
        let awake = scene.sectors.iter().filter(|s| !s.quiet).count();
        info!(
            "scene sectors timing (ms a frame): bookkeeping {:.3}, scene frame {:.3}, outputs {:.3}; {:.1} frames refreshed a frame, {} tracked meshes, {} in the scene's own sector, {} sectors awake",
            live.timing[0] / n,
            live.timing[1] / n,
            live.timing[2] / n,
            live.refreshed as f32 / n,
            live.tracked.len(),
            scene.sectors[SCENE_SECTOR].meshes.len(),
            awake
        );
        live.refreshed = 0;
        live.timing = [0.0; 3];
        live.timing_frames = 0;
    }

    // The renderer's sector walk.
    if live.sectors_changed {
        live.sectors_changed = false;
        draw_sectors.map.clear();
        for (node, entity) in live.entity_of.iter().enumerate() {
            if let (Some(entity), Some(sector)) = (entity, scene.nodes[node].sector)
                && sector != SCENE_SECTOR
            {
                draw_sectors.map.insert(*entity, sector as u32);
            }
        }
        draw_sectors.generation += 1;
    }
    let done = std::time::Instant::now();
    live.timing[0] += (frame_started - started).as_secs_f32() * 1000.0;
    live.timing[1] += (frame_done - frame_started).as_secs_f32() * 1000.0;
    live.timing[2] += (done - frame_done).as_secs_f32() * 1000.0;
    live.timing_frames += 1;
}

/// Takes the registered meshes the visible pass did not draw out of the
/// main camera's visible list.
fn cull_undrawn(
    mut live: ResMut<LiveScene>,
    cull: Res<SectorCull>,
    mut cameras: Query<&mut VisibleEntities>,
) {
    if !cull.0 || live.node_of.is_empty() {
        return;
    }
    let Some(mut visible) = live.camera.and_then(|c| cameras.get_mut(c).ok()) else {
        return;
    };
    let list = visible.get_mut(std::any::TypeId::of::<Mesh3d>());
    let before = list.len();
    list.retain(|e| {
        !live.node_of.contains_key(e) || live.drawn.contains(e) || live.exempt.contains(e)
    });
    let after = list.len();
    live.culled = (before - after, after);
    if before > after {
        live.culled_frames += 1;
        live.culled_most = live.culled_most.max(before - after);
    }
}

#[cfg(test)]
mod tests {
    use exa_formats::level::LevelGrid;

    use super::*;
    use crate::rooms::CELL_OPEN;

    /// Two rooms of two cells each along X with a door between: cells 1–2
    /// and 3–4 of row 1 (x −12…12 m, z −3…3 m).
    fn two_rooms() -> (Rooms, LevelGrid) {
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
        (Rooms::flood(&grid), grid)
    }

    #[test]
    fn rooms_become_sectors_with_their_doorways() {
        let (rooms, grid) = two_rooms();
        assert_eq!(rooms.len(), 2);
        let portals: Vec<DoorPortal> = rooms
            .rooms()
            .iter()
            .flat_map(|r| r.links.iter())
            .filter_map(|&l| rooms.portal(&grid, l))
            .collect();
        assert_eq!(portals.len(), 2);
        let scene = build_scene(&rooms, &portals);
        assert_eq!(scene.sectors.len(), 3);
        for (room, sector) in scene.sectors[1..].iter().enumerate() {
            assert!(sector.in_scene);
            assert_eq!(sector.portals.len(), 1);
            assert_eq!(sector.linked, vec![2 - room]);
            for portal in &sector.portals {
                assert_ne!(portal.neighbour, room + 1);
                // The plane faces the room behind.
                let behind = rooms.cell_centre(portal.cells[0] as u32);
                let here = rooms.cell_centre(portal.cells[1] as u32);
                assert!(portal.distance(Vec3::new(behind.x, 1.0, behind.y)) > 0.0);
                assert!(portal.distance(Vec3::new(here.x, 1.0, here.y)) < 0.0);
            }
        }
    }

    #[test]
    fn a_mesh_frame_is_its_bounds_in_the_world() {
        let transform = GlobalTransform::from(
            Transform::from_xyz(1.0, 2.0, 3.0)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::splat(2.0)),
        );
        let aabb = Aabb::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 1.0));
        let (centre, rotation, half) = mesh_frame(&transform, &aabb);
        assert!((half - Vec3::new(2.0, 1.0, 1.0)).length() < 1e-5);
        assert!((rotation.determinant() - 1.0).abs() < 1e-5);
        // Local centre (1, 0.5, 0.5), scaled by 2 and turned a quarter.
        assert!((centre - Vec3::new(2.0, 3.0, 1.0)).length() < 1e-4);
    }
}
