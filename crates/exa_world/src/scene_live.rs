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
//! - **Lights.** Every point light with a light id is a light node,
//!   registered in the sector of the room at its position (the scene's
//!   own sector off the rooms), as the room manager's node hook moves a
//!   node into the room of its cell. A moved light takes its new frame,
//!   which makes the frame activate it again; a light whose room changes
//!   moves sector first. The activation makes the light's alias in its
//!   sector and floods it: the cells the light flood recorded it in
//!   ([`crate::shadow_links::LevelWalls::cells_of`], flooded just before by
//!   [`crate::shadow_links`]) give their rooms' sectors an alias stamped
//!   this frame through the room cell hook ([`Scene::flood_hook`]), and the
//!   light's aliases no cell stamped are dropped. A sector's aliases are
//!   the lights a mesh registered there can be lit by
//!   ([`LiveScene::light_aliased`], read by the link pass).
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
use exa_render::sector_walk::{DrawSectors, DrawSlot, SectorScissors};
use exa_render::sun_fit::frustum_volume;
use exa_render::supersample::{SceneCamera, Supersample};
use exa_render::view::project_to_screen;

use bevy::platform::collections::HashMap;
use exa_render::lights::{LightId, LightsSystems};

use crate::light_alias::AliasLight;
use crate::light_flood::NodeBounds;
use crate::room_view::{InRoom, LevelRooms, RoomShell, RoomTracked};
use crate::rooms::{CELL_M, DoorPortal, PORTAL_BOTTOM_CM, Rooms};
use crate::scene_nodes::{
    BOUNDS_SPLIT, Draw, HAS_COLOUR, LIGHT_RANGED, LINK_CUTOFF, NodeKind, Portal, SCENE_SECTOR,
    SCISSOR, SNAPSHOT_FLAT, Scene, SceneNode, SectorState,
};
use crate::shadow_links::{LevelWalls, ShadowReach};
use exa_render::material::ExaMaterial;

/// The scene's light kind for a point light (an omni light, which the
/// activation floods through the cells).
const OMNI_LIGHT: u32 = 3;

pub struct SceneLivePlugin;

impl Plugin for SceneLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LiveScene>()
            .init_resource::<SectorCull>()
            .init_resource::<AliasLinks>()
            .add_systems(
                PostUpdate,
                run_scene
                    .after(TransformSystems::Propagate)
                    .after(VisibilitySystems::CalculateBounds)
                    .after(VisibilitySystems::UpdateFrusta)
                    .after(crate::room_view::track_rooms)
                    .after(LightsSystems::Ids)
                    .before(VisibilitySystems::CheckVisibility)
                    .in_set(exa_physics::FramePostPhase::View),
            )
            .add_systems(
                PostUpdate,
                cull_undrawn
                    .after(VisibilitySystems::CheckVisibility)
                    .before(bevy::light::SimulationLightSystems::CheckLightVisibility)
                    .before(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible)
                    .in_set(exa_physics::FramePostPhase::View),
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

/// Whether the node passes run the original's alias link methods on the
/// live lights and materials (`--no-alias-links` turns them off, to
/// compare: no alias sync, no colour bounds, a stand-in test that links
/// nothing).
#[derive(Resource, Clone, Copy, Debug)]
pub struct AliasLinks(pub bool);

impl Default for AliasLinks {
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
    /// The cell bytes of each moving root and moved light, as the scene's
    /// per-frame cell update last wrote them; the room hook runs only when
    /// they change.
    node_cells: EntityHashMap<(i64, i64)>,
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
    /// The time (ms) the last frame's scene update took (the original's
    /// frame: activation, membership, node passes and link tests).
    pub last_frame_ms: f32,
    /// Meshes registered or moved to another sector by this frame's run,
    /// for the light links ([`crate::shadow_links`]): a mesh is lit only by
    /// the lights its sector lists.
    pub sector_changes: Vec<Entity>,
    /// The light nodes: each light entity's node, and each node's light id.
    light_node: EntityHashMap<usize>,
    light_id_of: HashMap<usize, u32>,
    /// The node of each light id.
    node_of_light: HashMap<u32, usize>,
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

    /// Whether light `id` has an alias owned by `sector` (the sector's
    /// aliases are the lights a mesh registered there may be lit by);
    /// `None` when the scene holds no node for the light.
    pub fn light_aliased(&self, id: u32, sector: usize) -> Option<bool> {
        let node = *self.node_of_light.get(&id)?;
        Some(self.scene.as_ref()?.alias_in(node, sector).is_some())
    }

    /// The sectors holding an alias of light `id`, in the order its aliases
    /// were made; `None` when the scene holds no node for the light.
    pub fn light_sectors(&self, id: u32) -> Option<&[usize]> {
        let node = *self.node_of_light.get(&id)?;
        Some(&self.scene.as_ref()?.nodes[node].aliases)
    }

    /// The number of light nodes registered.
    pub fn light_count(&self) -> usize {
        self.light_node.len()
    }

    /// Whether the sector a registered mesh stands in was seen by the main
    /// view in the last classification (the sector word's main-view bit,
    /// whatever the frustum did to the mesh itself); `None` for a mesh the
    /// scene does not hold, which stands in the scene's own sector, always
    /// walked.
    pub fn sector_seen(&self, entity: Entity) -> Option<bool> {
        let node = *self.node_of.get(&entity)?;
        let scene = self.scene.as_ref()?;
        let sector = scene.nodes[node].sector?;
        Some(scene.sectors[sector].mask & crate::scene_nodes::MAIN != 0)
    }

    /// Whether the last visible pass drew a registered mesh (its node's
    /// main-view bit); `None` for a mesh the scene does not hold.
    pub fn drawn_by_main(&self, entity: Entity) -> Option<bool> {
        self.node_of
            .contains_key(&entity)
            .then(|| self.drawn.contains(&entity))
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
        Option<&'static MeshMaterial3d<ExaMaterial>>,
    ),
    (With<Mesh3d>, Without<RoomShell>),
>;
type MovedQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static GlobalTransform, &'static Aabb),
    (With<Mesh3d>, Or<(Changed<GlobalTransform>, Changed<Aabb>)>),
>;

type LightQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        &'static LightId,
        &'static PointLight,
        Option<&'static ShadowReach>,
    ),
>;

/// The light inputs of [`run_scene`]: the lights, the new ones, the gone
/// ones and the level's walls (whose cell lists the flood left).
type LightParams<'w, 's> = (
    LightQuery<'w, 's>,
    Query<'w, 's, Entity, Added<LightId>>,
    RemovedComponents<'w, 's, LightId>,
    Option<Res<'w, LevelWalls>>,
);

/// A light node's frame: its position, and half extents whose box radius
/// is the light's radius (the scene's sphere tests read it).
fn light_frame(
    transform: &GlobalTransform,
    light: &PointLight,
    reach: Option<&ShadowReach>,
) -> (Vec3, Vec3) {
    let radius = reach.map_or(light.range, |r| r.radius);
    (transform.translation(), Vec3::splat(radius / 3f32.sqrt()))
}

/// The radius the original gives an unranged light's node (3.4e38).
const UNRANGED_RADIUS: f32 = f32::MAX;

/// A light's link-test values, as its activation leaves them for the alias
/// sync: the direct colour (linear colour times the current intensity), its
/// luma as the intensity gate, the cutoff, the falloff `0.8/R` of a ranged
/// light (`1/R` in single precision, the product in double, as the light's
/// activation computes it; zero unranged), its radius (3.4e38 unranged)
/// and spot shaping. The light node's option word takes the ranged bit,
/// which picks the gated test, and the spot bits.
fn set_link_light(node: &mut SceneNode, light: &PointLight, reach: Option<&ShadowReach>) {
    let ranged = reach.is_none_or(|r| r.ranged);
    let radius = reach.map_or(light.range, |r| r.radius);
    let spot = reach.and_then(|r| r.spot);
    let linear = light.color.to_linear();
    let intensity = light.intensity / crate::LUMENS_PER_INTENSITY;
    let colour = Vec3::new(linear.red, linear.green, linear.blue) * intensity;
    let falloff = if ranged && radius > 0.0 {
        (f64::from(1.0 / radius) * 0.8) as f32
    } else {
        0.0
    };
    let flags = if spot.is_some() { SNAPSHOT_FLAT } else { 0 };
    node.link_light = Some(AliasLight {
        position: Vec3::ZERO,
        radius: if ranged { radius } else { UNRANGED_RADIUS },
        colour,
        intensity: exa_formats::colour::luma(colour.to_array()),
        cutoff: LINK_CUTOFF,
        falloff,
        flags,
        spot,
        has_mask: false,
    });
    node.flags &= !(LIGHT_RANGED | crate::scene_nodes::SNAPSHOT_MODE);
    node.flags |= flags;
    if ranged {
        node.flags |= LIGHT_RANGED;
    }
}

/// A mesh's colour bound from its material: the specular colour times its
/// level plus the diffuse (both linear, the diffuse ×1.5 for a legacy
/// material), as the material's activation sums them and the geometry
/// node's activation takes the largest over its parts (one part here).
fn mesh_colour(params: &exa_render::material::ExaParams) -> Vec3 {
    params.specular.truncate() + params.diffuse.truncate()
}

/// Whether a colour bound counts: its squared length, summed
/// `(x² + y²) + z²`, above the engine's epsilon.
fn colour_bound(c: Vec3) -> bool {
    c.z * c.z + (c.y * c.y + c.x * c.x) > crate::geometry_node::BOUND_EPSILON
}

/// A scene node's box as the cell flood's wall test reads it.
fn node_bounds(node: &SceneNode) -> NodeBounds {
    let (obb, radius) = node.obb();
    let m = node.world_rotation;
    let h = node.half_extents;
    NodeBounds {
        centre: obb.centre,
        radius,
        extent_x: m.row(0).abs().dot(h),
        extent_z: m.row(2).abs().dot(h),
    }
}

/// The scene's per-frame update: registration, moves, removals, frames and
/// bounds, then the original's frame with the room states and the main
/// camera.
#[allow(clippy::too_many_arguments)]
pub fn run_scene(
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
    cameras: Query<
        (
            Entity,
            &Camera,
            &GlobalTransform,
            &Projection,
            Has<Supersample>,
        ),
        SceneCamera,
    >,
    (mut scissors, materials, links): (
        ResMut<SectorScissors>,
        Option<Res<Assets<ExaMaterial>>>,
        Res<AliasLinks>,
    ),
    (lights, added_lights, mut lights_removed, walls): LightParams,
) {
    let started = std::time::Instant::now();
    let Some(level) = level else {
        scissors.camera = None;
        scissors.rects.clear();
        if live.scene.is_some() {
            live.reset();
            draw_sectors.map.clear();
            draw_sectors.generation += 1;
        }
        return;
    };
    let live = &mut *live;
    live.sector_changes.clear();
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
        if let Some(scene) = live.scene.as_mut() {
            scene.alias_sync = links.0;
        }
        live.rooms = Some(level.rooms.clone());
        live.dirty = vec![true; level.rooms.len() + 1];
        register.extend(meshes.iter().map(|m| m.0));
        despawned.clear();
        rooms_removed.clear();
        lights_removed.clear();
    } else {
        register.extend(added.iter());
    }
    let mut new_lights: Vec<Entity> = if fresh {
        lights.iter().map(|l| l.0).collect()
    } else {
        added_lights.iter().collect()
    };
    // Registered in entity index (spawn) order, not the queries' table
    // order, so the sectors' light and mesh lists do not move with a
    // build's types.
    new_lights.sort_unstable_by_key(|e| e.index_u32());
    register.sort_unstable_by_key(|e| e.index_u32());
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
                live.node_cells.remove(&root);
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

    // Lights gone leave through the removal queue too.
    for entity in lights_removed.read() {
        let Some(node) = live.light_node.remove(&entity) else {
            continue;
        };
        live.node_cells.remove(&entity);
        if let Some(id) = live.light_id_of.remove(&node)
            && live.node_of_light.get(&id) == Some(&node)
        {
            live.node_of_light.remove(&id);
        }
        live.entity_of[node] = None;
        let scene = live.scene.as_mut().expect("scene");
        if !scene.queue_removal(node, true) {
            live.freeing.push(node);
        }
    }

    // New lights: registered in the sector of the room they stand in.
    for entity in new_lights {
        if live.light_node.contains_key(&entity) {
            continue;
        }
        let Ok((_, transform, id, light, reach)) = lights.get(entity) else {
            continue;
        };
        let (position, half) = light_frame(transform, light, reach);
        let sector = room_sector(rooms.room_of_node(Vec2::new(position.x, position.z)));
        let scene = live.scene.as_mut().expect("scene");
        let parent = scene.sectors[sector].node;
        let node = match live.free.pop() {
            Some(slot) => scene.reuse_node(slot, NodeKind::Light, parent),
            None => scene.add_node(NodeKind::Light, parent),
        };
        // Every Exarust light is a point light: the omni kind, flooded.
        scene.nodes[node].light_type = OMNI_LIGHT;
        set_link_light(&mut scene.nodes[node], light, reach);
        scene.set_frame(node, position, Mat3::IDENTITY, half);
        scene.refresh(node);
        scene.register(node, sector);
        if live.entity_of.len() <= node {
            live.entity_of.resize(node + 1, None);
        }
        live.entity_of[node] = Some(entity);
        live.light_node.insert(entity, node);
        live.light_id_of.insert(node, id.0);
        live.node_of_light.insert(id.0, node);
    }

    // Moved lights take their new frame (the frame activates them again);
    // one whose room changed moves sector first, as the room manager's
    // node hook moves a node into the room of its new cell.
    for (entity, transform, _, light, reach) in &lights {
        let Some(&node) = live.light_node.get(&entity) else {
            continue;
        };
        let (position, half) = light_frame(transform, light, reach);
        let scene = live.scene.as_mut().expect("scene");
        let n = &scene.nodes[node];
        if n.position == position && n.half_extents == half {
            continue;
        }
        // The per-frame cell update: the room hook runs only when the
        // light's cell bytes change.
        let cell = rooms.cell_of_moving_node(Vec2::new(position.x, position.z));
        if live.node_cells.insert(entity, cell) != Some(cell) {
            let sector = room_sector(rooms.room_of_cell(cell.0, cell.1));
            if scene.nodes[node].sector != Some(sector) {
                scene.move_to_sector(node, sector);
            }
        }
        set_link_light(&mut scene.nodes[node], light, reach);
        scene.set_frame(node, position, Mat3::IDENTITY, half);
        scene.refresh(node);
    }

    // New meshes.
    for entity in register {
        if live.node_of.contains_key(&entity) {
            continue;
        }
        let Ok((_, transform, aabb, room, exempt, shadowless, layers, material)) =
            meshes.get(entity)
        else {
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
        // The colour bound the alias link tests multiply by; a mesh with
        // one takes light buckets even when it casts nothing.
        let colour = material
            .and_then(|m| materials.as_ref()?.get(&m.0))
            .map_or(Vec3::ZERO, |m| mesh_colour(&m.params));
        scene.nodes[node].colour = colour;
        if links.0 && colour_bound(colour) {
            scene.nodes[node].flags |= HAS_COLOUR;
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
        live.sector_changes.push(entity);
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
        // The per-frame cell update writes the root's cell bytes (clamped
        // one cell in from the edge) and runs the room hook only when they
        // change.
        let cell = rooms.cell_of_moving_node(Vec2::new(p.x, p.z));
        if live.node_cells.insert(root, cell) == Some(cell) {
            continue;
        }
        let now = room_sector(rooms.room_of_cell(cell.0, cell.1));
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
        if let Some(entity) = live.entity_of.get(node).copied().flatten() {
            live.sector_changes.push(entity);
        }
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
    let camera = cameras.iter().find(|(_, c, _, _, _)| c.is_active);
    let Some((camera_entity, camera, camera_transform, perspective, supersampled)) = camera
        .and_then(|(e, c, t, p, s)| match p {
            Projection::Perspective(perspective) => Some((e, c, t, perspective, s)),
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
        scissors.camera = None;
        scissors.rects.clear();
        return;
    };
    let corners = perspective
        .get_frustum_corners(-perspective.near, -perspective.far)
        .map(|c| camera_transform.transform_point(Vec3::from(c)));
    let view: ConvexVolume = frustum_volume(corners);
    let eye = camera_transform.translation();
    // The portal scissor is in window pixels, as the original projects it
    // into its view (each super-sampled sub-frame is window-sized).
    let target = camera.physical_viewport_size().unwrap_or(UVec2::ONE);
    let window = if supersampled {
        (target / exa_render::supersample::SCALE).max(UVec2::ONE)
    } else {
        target
    };
    let clip = camera.clip_from_view();
    let view_from_world = camera_transform.affine().inverse();
    let project = |p: Vec3| {
        let v = view_from_world.transform_point3(p);
        let (x, y, _) = project_to_screen(
            window,
            Vec2::new(clip.x_axis.x, clip.y_axis.y),
            v.to_array(),
        );
        Vec2::new(x, y)
    };
    let dt_ms = time.delta().as_millis().min(i32::MAX as u128) as i32;
    let frame_started = std::time::Instant::now();
    // The flood: the rooms of the cells the light flood recorded the light
    // in this frame, in the order it recorded them.
    let walls = walls.filter(|w| w.size().0 == rooms.width() as usize);
    let light_id_of = &live.light_id_of;
    let mut flood = |_: &Scene, node: usize| -> Option<Vec<usize>> {
        let walls = walls.as_deref()?;
        let id = *light_id_of.get(&node)?;
        Some(
            walls
                .cells_of(id)
                .iter()
                .filter_map(|&cell| rooms.room_of_index(cell as u32))
                .map(|room| usize::from(room) + 1)
                .collect(),
        )
    };
    // The alias link methods, with the cell flood's wall test as the gated
    // test's gate (a light the walls do not know passes).
    let gate = |scene: &Scene, light: usize, node: usize| -> bool {
        let (Some(walls), Some(&id)) = (walls.as_deref(), light_id_of.get(&light)) else {
            return true;
        };
        walls.sees(
            id,
            scene.nodes[light].world,
            &node_bounds(&scene.nodes[node]),
        )
    };
    let on = links.0;
    let mut link = |scene: &Scene, node: usize, alias: usize| {
        if on {
            scene.alias_reach(node, alias, &gate)
        } else {
            crate::scene_nodes::AliasLink::default()
        }
    };
    let events = scene.frame_with(
        dt_ms,
        eye,
        std::slice::from_ref(&view),
        &project,
        &mut link,
        &mut flood,
    );
    live.camera = Some(camera_entity);
    // The original's scene manager warns of a deleted node that hung from
    // the world sector (a node removed before it joined a scene).
    for &node in &events.world_deleted {
        warn!(
            "World sector object deleted: node {node} ({:?})",
            live.entity_of.get(node).copied().flatten()
        );
    }

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

    // The renderer's sector walk: each mesh's sector and its position in
    // the sector's list (registration order).
    if live.sectors_changed {
        live.sectors_changed = false;
        let registration_order = draw_sectors.registration_order;
        draw_sectors.map.clear();
        for (index, sector) in scene.sectors.iter().enumerate() {
            for (order, &node) in sector.meshes.iter().enumerate() {
                if let Some(Some(entity)) = live.entity_of.get(node) {
                    draw_sectors.map.insert(
                        *entity,
                        DrawSlot {
                            sector: index as u32,
                            order: if registration_order { order as u32 } else { 0 },
                        },
                    );
                }
            }
        }
        draw_sectors.generation += 1;
    }

    // The portal scissor: the rectangles of the sectors seen through
    // doorways and the meshes drawn with the scissor bit.
    scissors.camera = Some(camera_entity);
    scissors.supersampled = supersampled;
    scissors.rects.clear();
    let mut entries = EntityHashMap::default();
    if scissors.enabled {
        for (index, sector) in scene.sectors.iter().enumerate() {
            let Some(rect) = sector.scissor.filter(|_| sector.through_portal) else {
                continue;
            };
            scissors.rects.push((
                index as u32,
                [rect.min_x, rect.min_y, rect.max_x, rect.max_y],
            ));
            for &node in &sector.meshes {
                if scene.nodes[node].mask & SCISSOR != 0
                    && let Some(Some(entity)) = live.entity_of.get(node)
                {
                    entries.insert(*entity, index as u32);
                }
            }
        }
    }
    if entries != scissors.entries {
        scissors.entries = entries;
        scissors.generation += 1;
    }
    let done = std::time::Instant::now();
    live.timing[0] += (frame_started - started).as_secs_f32() * 1000.0;
    live.timing[1] += (frame_done - frame_started).as_secs_f32() * 1000.0;
    live.last_frame_ms = (frame_done - frame_started).as_secs_f32() * 1000.0;
    live.timing[2] += (done - frame_done).as_secs_f32() * 1000.0;
    live.timing_frames += 1;
}

/// Takes the registered meshes the visible pass did not draw out of the
/// main camera's visible list.
pub(crate) fn cull_undrawn(
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
    use bevy::camera::PerspectiveProjection;
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
    fn a_moving_nodes_cell_bytes_stay_one_cell_in_from_the_edge() {
        let (rooms, _) = two_rooms();
        let (w, h) = (rooms.width() as i64, rooms.height() as i64);
        // Far off the grid's low and high sides.
        assert_eq!(
            rooms.cell_of_moving_node(Vec2::new(-1000.0, -1000.0)),
            (1, 1)
        );
        assert_eq!(
            rooms.cell_of_moving_node(Vec2::new(1000.0, 1000.0)),
            (w - 2, h - 2)
        );
        // The load-time sort takes the edge cell itself.
        assert_eq!(rooms.cell_of_node(Vec2::new(-1000.0, -1000.0)), (0, 0));
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

    /// A headless app with the real scene and light-link plugins on a
    /// grid level: its rooms (with their doorways), its walls and a camera.
    fn light_app(rooms: Rooms, grid: &LevelGrid) -> App {
        let portals: Vec<DoorPortal> = rooms
            .rooms()
            .iter()
            .flat_map(|r| r.links.iter())
            .filter_map(|&l| rooms.portal(grid, l))
            .collect();
        let mut level = LevelRooms::new(Arc::new(rooms));
        level.portals = Arc::new(portals);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin))
            .init_resource::<DrawSectors>()
            .init_resource::<SectorScissors>()
            .init_resource::<exa_render::cube_shadows::PointShadows>()
            .init_resource::<Assets<Mesh>>()
            .add_plugins((SceneLivePlugin, crate::shadow_links::ShadowLinksPlugin))
            .insert_resource(level)
            .insert_resource(LevelWalls::new(grid).expect("walls"));
        app.world_mut().spawn((
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::from_xyz(0.0, 1.5, 0.0),
        ));
        app
    }

    /// A ranged point light with its id at `at` (m).
    fn spawn_light(app: &mut App, id: u32, at: Vec3, radius: f32) -> Entity {
        app.world_mut()
            .spawn((
                PointLight {
                    range: radius,
                    ..default()
                },
                ShadowReach::new(Vec3::ONE, 1.0, radius),
                LightId(id),
                Transform::from_translation(at),
            ))
            .id()
    }

    #[test]
    fn a_level_light_floods_its_aliases_into_the_rooms_it_reaches() {
        let (rooms, grid) = two_rooms();
        let a = rooms.cell_centre(6 + 1);
        let b = rooms.cell_centre(6 + 4);
        let mut app = light_app(rooms, &grid);
        // Light 1 stands in room A and reaches room B through the door;
        // light 2 is short and stays in room B.
        let far = spawn_light(&mut app, 1, Vec3::new(a.x, 2.0, a.y), 30.0);
        spawn_light(&mut app, 2, Vec3::new(b.x, 2.0, b.y), 2.0);
        app.update();
        let live = app.world().resource::<LiveScene>();
        assert_eq!(live.light_count(), 2);
        let scene = live.scene.as_ref().unwrap();
        // Registered in the room it stands in, aliased there and in every
        // room its flood recorded, each alias stamped this frame.
        assert_eq!(live.light_sectors(1), Some(&[1, 2][..]));
        assert_eq!(live.light_sectors(2), Some(&[2][..]));
        assert_eq!(live.light_aliased(1, 2), Some(true));
        assert_eq!(live.light_aliased(2, 1), Some(false));
        assert_eq!(live.light_aliased(9, 1), None);
        for sector in [1, 2] {
            assert!(
                scene.sectors[sector]
                    .aliases
                    .iter()
                    .all(|a| a.flooded == scene.frame_count)
            );
        }
        // The membership rebuild took the aliases as processed.
        assert_eq!(scene.sectors[2].aliases_done, 2);
        // The room lists the link pass used to read agree.
        let lists = &app
            .world()
            .resource::<crate::shadow_links::RoomLightLists>()
            .lists;
        assert_eq!(lists.rooms_of(1), &[0, 1]);
        assert_eq!(lists.rooms_of(2), &[1]);

        // Light 1 moves into room B with a short reach: it changes sector,
        // its alias in room A goes and room A's sector drops it.
        app.world_mut()
            .entity_mut(far)
            .get_mut::<Transform>()
            .unwrap()
            .translation = Vec3::new(b.x + 1.0, 2.0, b.y);
        app.world_mut()
            .entity_mut(far)
            .get_mut::<ShadowReach>()
            .unwrap()
            .radius = 2.0;
        app.update();
        app.update();
        let live = app.world().resource::<LiveScene>();
        let scene = live.scene.as_ref().unwrap();
        assert_eq!(live.light_sectors(1), Some(&[2][..]));
        assert_eq!(live.light_aliased(1, 1), Some(false));
        assert!(
            scene.sectors[1]
                .aliases
                .iter()
                .all(|a| a.light != 1 || a.owner.is_some())
        );
        assert!(
            !scene.sectors[1]
                .lights
                .iter()
                .any(|&l| scene.nodes[l].sector != Some(1))
        );

        // A despawned light leaves the scene and its aliases go.
        app.world_mut().entity_mut(far).despawn();
        app.update();
        app.update();
        let live = app.world().resource::<LiveScene>();
        assert_eq!(live.light_count(), 1);
        assert_eq!(live.light_aliased(1, 2), None);
        let scene = live.scene.as_ref().unwrap();
        assert_eq!(scene.sectors[2].aliases.len(), 1);
    }

    /// The live link pass: the activation's alias sync gives each alias its
    /// light's values, and the node passes run the alias link methods on
    /// the real lights and materials (the gated reach for a ranged light
    /// with the walls' answer as its gate, the plain luma test for an
    /// unranged one), so a mesh's buckets carry the original's weights and
    /// are ordered heaviest first.
    #[test]
    fn a_mesh_behind_the_eye_stands_in_a_seen_sector() {
        let (rooms, grid) = two_rooms();
        let a = rooms.cell_centre(6 + 1);
        let mut app = light_app(rooms, &grid);
        // The eye in the first room, looking along -X, away from the door.
        let eye = Vec3::new(a.x + 2.0, 1.5, a.y);
        let mut cameras = app
            .world_mut()
            .query_filtered::<&mut Transform, With<Camera3d>>();
        for mut t in cameras.iter_mut(app.world_mut()) {
            *t = Transform::from_translation(eye).looking_to(Vec3::NEG_X, Vec3::Y);
        }
        let mesh = |app: &mut App, at: Vec3, room: u16| {
            app.world_mut()
                .spawn((
                    Mesh3d::default(),
                    Aabb::from_min_max(Vec3::splat(-0.2), Vec3::splat(0.2)),
                    Transform::from_translation(at),
                    InRoom(room),
                ))
                .id()
        };
        // The first room is the active one.
        {
            let mut level = app.world_mut().resource_mut::<LevelRooms>();
            let rooms = level.rooms.clone();
            level.tracker.update(&rooms, a, 0);
        }
        // One ahead of the eye, one behind it (outside the frustum).
        let ahead = mesh(&mut app, Vec3::new(a.x - 1.0, 0.5, a.y), 0);
        let behind = mesh(&mut app, Vec3::new(a.x + 4.0, 0.5, a.y), 0);
        let loose = app.world_mut().spawn_empty().id();
        app.update();
        app.update();
        let live = app.world().resource::<LiveScene>();
        assert_eq!(live.sector_seen(ahead), Some(true));
        // The mesh behind the eye is not drawn, but its sector was seen, so
        // the camera's wall probe still asks it.
        assert_eq!(live.drawn_by_main(behind), Some(false));
        assert_eq!(live.sector_seen(behind), Some(true));
        // A mesh the scene does not hold stands in the scene's own sector.
        assert_eq!(live.sector_seen(loose), None);
    }

    #[test]
    fn a_plant_chunk_takes_its_rooms_view_test() {
        let (rooms, grid) = two_rooms();
        let a = rooms.cell_centre(6 + 1);
        let b = rooms.cell_centre(6 + 4);
        let mut app = light_app(rooms, &grid);
        let eye = Vec3::new(a.x + 2.0, 1.5, a.y);
        let mut cameras = app
            .world_mut()
            .query_filtered::<&mut Transform, With<Camera3d>>();
        for mut t in cameras.iter_mut(app.world_mut()) {
            *t = Transform::from_translation(eye).looking_to(Vec3::NEG_X, Vec3::Y);
        }
        {
            let mut level = app.world_mut().resource_mut::<LevelRooms>();
            let rooms = level.rooms.clone();
            level.tracker.update(&rooms, a, 0);
        }
        // The terrain's plant chunks are meshes of their block's room: the
        // grass hook's view-mask test is the room's view test here.
        let chunk = |app: &mut App, at: Vec2, room: u16| {
            app.world_mut()
                .spawn((
                    crate::terrain::PlantChunk,
                    Mesh3d::default(),
                    Aabb::from_min_max(Vec3::splat(-0.2), Vec3::splat(0.2)),
                    Transform::from_xyz(at.x - 1.0, 0.2, at.y),
                    InRoom(room),
                ))
                .id()
        };
        let seen = chunk(&mut app, a, 0);
        let unseen = chunk(&mut app, b, 1);
        app.update();
        app.update();
        let live = app.world().resource::<LiveScene>();
        assert_eq!(live.drawn_by_main(seen), Some(true));
        assert_eq!(live.drawn_by_main(unseen), Some(false));
    }

    #[test]
    fn the_live_link_pass_weighs_a_room_mesh_by_its_lights() {
        use crate::light_alias::{AliasNode, reach, reach_plain};
        let (rooms, grid) = two_rooms();
        let a = rooms.cell_centre(6 + 1);
        let mut app = light_app(rooms, &grid);
        app.init_resource::<Assets<ExaMaterial>>();
        let material = app
            .world_mut()
            .resource_mut::<Assets<ExaMaterial>>()
            .add(ExaMaterial {
                params: exa_render::material::ExaParams {
                    diffuse: Vec4::new(0.4, 0.3, 0.2, 1.0),
                    specular: Vec4::new(0.1, 0.1, 0.1, 8.0),
                    ..default()
                },
                ..default()
            });
        let ranged = spawn_light(&mut app, 1, Vec3::new(a.x + 1.0, 2.0, a.y), 12.0);
        let mut unranged = ShadowReach::new(Vec3::ONE, 1.0, 5.0);
        unranged.ranged = false;
        app.world_mut().spawn((
            PointLight {
                intensity: 0.5 * crate::LUMENS_PER_INTENSITY,
                range: 5.0,
                ..default()
            },
            unranged,
            LightId(2),
            Transform::from_xyz(a.x - 1.0, 2.0, a.y),
        ));
        let mesh = app
            .world_mut()
            .spawn((
                Mesh3d::default(),
                MeshMaterial3d(material),
                Aabb::from_min_max(Vec3::splat(-0.5), Vec3::splat(0.5)),
                Transform::from_xyz(a.x, 0.5, a.y + 1.0),
                InRoom(0),
            ))
            .id();
        app.update();
        app.update();
        let live = app.world().resource::<LiveScene>();
        let scene = live.scene.as_ref().unwrap();
        let node = live.node_of[&mesh];
        let n = &scene.nodes[node];
        // The colour bound is the material's specular plus diffuse, which
        // asks for buckets even of a mesh that casts nothing.
        assert_eq!(n.colour, Vec3::new(0.5, 0.4, 0.3));
        assert_ne!(n.flags & HAS_COLOUR, 0);
        assert_eq!(scene.sectors[1].aliases.len(), 2);
        let (obb, radius) = n.obb();
        let target = AliasNode {
            centre: obb.centre,
            radius,
            half: obb.half_extents,
            axes: obb.axes,
            colour: n.colour,
            mask: 0,
        };
        let mut expected = Vec::new();
        for (at, alias) in scene.sectors[1].aliases.iter().enumerate() {
            let link = alias.link.expect("synced");
            let light_node = alias.light;
            assert_eq!(link.position, scene.nodes[light_node].world);
            let hit = if scene.nodes[light_node].flags & LIGHT_RANGED != 0 {
                reach(&link, &target, true)
            } else {
                reach_plain(&link, &target)
            };
            assert!(hit.lit && hit.weight > 0.0, "alias {at}: {hit:?}");
            assert_eq!(n.buckets[at].weight, hit.weight, "alias {at}");
            expected.push((hit.weight, at));
        }
        expected.sort_by(|x, y| y.0.total_cmp(&x.0));
        let mut chain = Vec::new();
        let mut at = n.bucket_head;
        while at >= 0 {
            chain.push(at as usize);
            at = n.buckets[at as usize].next;
        }
        assert_eq!(chain, expected.iter().map(|e| e.1).collect::<Vec<_>>());
        // The ranged light's alias reaches the mesh by the falloff: moved
        // farther off, its weight falls.
        let ranged_of = |scene: &Scene| {
            scene.sectors[1]
                .aliases
                .iter()
                .position(|al| scene.nodes[al.light].flags & LIGHT_RANGED != 0)
                .unwrap()
        };
        let before = n.buckets[ranged_of(scene)].weight;
        app.world_mut()
            .entity_mut(ranged)
            .get_mut::<Transform>()
            .unwrap()
            .translation = Vec3::new(a.x + 1.0, 2.0, a.y - 2.5);
        app.update();
        let live = app.world().resource::<LiveScene>();
        let scene = live.scene.as_ref().unwrap();
        let after = scene.nodes[node].buckets[ranged_of(scene)].weight;
        assert!(after < before, "{after} < {before}");
    }

    /// A headless app with the scene and light-link plugins on shipped
    /// level `name`, its lights given ids in spawn order as the renderer
    /// gives them, the alias link tests on or off.
    fn shipped_level(game: &exa_assets::GameData, name: &str, links: bool) -> (App, Vec<Entity>) {
        use exa_assets::{ModelCache, ModelLoader};
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin))
            .init_resource::<DrawSectors>()
            .init_resource::<SectorScissors>()
            .init_resource::<exa_render::cube_shadows::PointShadows>()
            .insert_resource(AliasLinks(links))
            .add_plugins((SceneLivePlugin, crate::shadow_links::ShadowLinksPlugin));
        app.world_mut().spawn((
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::from_xyz(0.0, 1.5, 0.0),
        ));
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        {
            let mut loader = ModelLoader {
                game,
                cache: &mut cache,
                meshes: &mut meshes,
                materials: &mut materials,
                images: &mut images,
                bc_supported: true,
            };
            let mut commands = Commands::new(&mut queue, app.world());
            crate::spawn_level(&mut commands, &mut loader, name)
                .unwrap_or_else(|| panic!("{name} is in Resource.rpk"));
        }
        queue.apply(app.world_mut());
        // The bounds Bevy's visibility plugin would compute from the meshes.
        let parts: Vec<(Entity, Handle<Mesh>)> = app
            .world_mut()
            .query::<(Entity, &Mesh3d)>()
            .iter(app.world())
            .map(|(e, m)| (e, m.0.clone()))
            .collect();
        for (entity, mesh) in parts {
            use bevy::camera::primitives::MeshAabb;
            if let Some(aabb) = meshes.get(&mesh).and_then(MeshAabb::get_aabb) {
                app.world_mut().entity_mut(entity).insert(aabb);
            }
        }
        app.insert_resource(materials).insert_resource(meshes);
        let mut lights: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<PointLight>>()
            .iter(app.world())
            .collect();
        lights.sort();
        for (id, &entity) in lights.iter().enumerate() {
            app.world_mut()
                .entity_mut(entity)
                .insert(LightId(id as u32));
        }
        (app, lights)
    }

    /// The live link pass's cost on the shipped levels, in release builds
    /// (`cargo test --release -p exa_world -- --ignored link_pass_cost
    /// --nocapture`): a carried torch circles in the first light's room
    /// with the camera, every frame, with the alias link tests on and off;
    /// the scene's frame and the whole update are timed. Skipped without
    /// `EXANIMA_DIR`.
    #[test]
    #[ignore]
    fn shipped_level_link_pass_cost() {
        use exa_assets::GameData;
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        for name in ["exanima01", "exanima02", "exanima06"] {
            let mut report = Vec::new();
            for links in [false, true, false, true] {
                let (mut app, lights) = shipped_level(&game, name, links);
                let start = app
                    .world()
                    .get::<Transform>(lights[0])
                    .map_or(Vec3::ZERO, |t| t.translation);
                let torch = app
                    .world_mut()
                    .spawn((
                        PointLight {
                            intensity: 2.0 * crate::LUMENS_PER_INTENSITY,
                            range: 8.0,
                            ..default()
                        },
                        ShadowReach::new(Vec3::new(1.0, 0.6, 0.3), 2.0, 8.0),
                        LightId(lights.len() as u32),
                        Transform::from_translation(start),
                    ))
                    .id();
                let camera = app
                    .world_mut()
                    .query_filtered::<Entity, With<Camera3d>>()
                    .single(app.world())
                    .unwrap();
                for _ in 0..3 {
                    app.update();
                }
                let frames = 300;
                let mut update = Vec::with_capacity(frames);
                let mut scene_ms = 0.0;
                for frame in 0..frames {
                    let angle = frame as f32 * 0.05;
                    let at = start + Vec3::new(angle.cos(), 0.0, angle.sin()) * 1.5;
                    app.world_mut()
                        .get_mut::<Transform>(torch)
                        .unwrap()
                        .translation = at;
                    *app.world_mut().get_mut::<Transform>(camera).unwrap() =
                        Transform::from_translation(at + Vec3::Y * 0.2)
                            .looking_at(at + Vec3::new(-angle.sin(), 0.0, angle.cos()), Vec3::Y);
                    let t = std::time::Instant::now();
                    app.update();
                    update.push(t.elapsed().as_secs_f64() * 1e3);
                    let live = app.world().resource::<LiveScene>();
                    scene_ms += f64::from(live.last_frame_ms);
                }
                update.sort_by(f64::total_cmp);
                let live = app.world().resource::<LiveScene>();
                let scene = live.scene.as_ref().unwrap();
                let buckets: usize = scene.nodes.iter().filter(|n| !n.buckets.is_empty()).count();
                let coloured = scene
                    .nodes
                    .iter()
                    .filter(|n| n.kind == NodeKind::Mesh && n.colour != Vec3::ZERO)
                    .count();
                let torch_node = live.light_node[&torch];
                let torch_aliases = &scene.nodes[torch_node].aliases;
                let torch_meshes: usize = torch_aliases
                    .iter()
                    .map(|&s| scene.sectors[s].meshes.len())
                    .sum();
                let lit = scene
                    .nodes
                    .iter()
                    .filter(|n| n.buckets.iter().any(|b| b.weight > 0.0))
                    .count();
                report.push(format!(
                    "links {links}: update median {:.3} ms, p95 {:.3} ms; scene frame mean {:.3} ms; {} meshes ({} coloured), {} bucket meshes, {} with a weight; torch in {} sectors holding {} meshes",
                    update[frames / 2],
                    update[frames * 95 / 100],
                    scene_ms / frames as f64,
                    live.node_of.len(),
                    coloured,
                    buckets,
                    lit,
                    torch_aliases.len(),
                    torch_meshes,
                ));
            }
            println!("{name}:\n  {}", report.join("\n  "));
        }
    }

    /// The shipped tile-grid levels: every level light is a light node of
    /// the live scene, and the scene's sector aliases — the lights a mesh
    /// in each sector may take — are the room lists the link pass read
    /// before (the light's room and the rooms of the cells its flood
    /// recorded), but for what the original does differently: a light
    /// standing in no room loses its scene-sector alias at the end of its
    /// flood (no cell hook stamps the scene's own sector). The cell lists
    /// are the flood's, unchanged. Skipped without `EXANIMA_DIR`.
    #[test]
    fn shipped_level_light_aliases_follow_the_floods() {
        use exa_assets::GameData;
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        for name in ["exanima01", "exanima02", "exanima06"] {
            let (mut app, lights) = shipped_level(&game, name, true);
            app.update();
            app.update();
            let world = app.world();
            let live = world.resource::<LiveScene>();
            let lists = &world
                .resource::<crate::shadow_links::RoomLightLists>()
                .lists;
            let level = world.resource::<LevelRooms>();
            let walls = world.resource::<LevelWalls>();
            assert_eq!(live.light_count(), lights.len(), "{name}");
            let mut off_room = 0;
            let mut differ = Vec::new();
            for id in 0..lights.len() as u32 {
                let sectors = live.light_sectors(id).expect("a light node");
                let listed: Vec<usize> = lists
                    .rooms_of(id)
                    .iter()
                    .map(|&r| usize::from(r) + 1)
                    .collect();
                let node = live.node_of_light[&id];
                let scene = live.scene.as_ref().unwrap();
                let home = scene.nodes[node].sector;
                if home == Some(SCENE_SECTOR) {
                    off_room += 1;
                    // The original: the scene-sector alias is dropped.
                    assert!(!sectors.contains(&SCENE_SECTOR), "{name} light {id}");
                    assert!(lists.reaches(id, None), "{name} light {id}");
                }
                let mut a = sectors.to_vec();
                let mut b = listed.clone();
                a.sort_unstable();
                b.sort_unstable();
                if a != b {
                    differ.push((id, sectors.to_vec(), listed));
                }
                // Every alias the light has is current.
                for &s in sectors {
                    assert_eq!(
                        scene.alias_in(node, s).map(|a| a.flooded),
                        Some(scene.frame_count - 1),
                        "{name} light {id} sector {s}"
                    );
                }
            }
            let cells = walls.size().0 * walls.size().1;
            let full = (0..cells)
                .filter(|&c| {
                    walls.lights(c % walls.size().0, c / walls.size().0).count()
                        >= crate::light_flood::MAX_CELL_LIGHTS
                })
                .count();
            println!(
                "{name}: {} lights, {} rooms, {} lights in no room, {} lights whose aliases differ from the room lists {:?}, {} full cells",
                lights.len(),
                level.rooms.len(),
                off_room,
                differ.len(),
                differ.iter().take(4).collect::<Vec<_>>(),
                full
            );
            assert!(differ.is_empty(), "{name}: {differ:?}");
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
