//! The renderer's sector-major walk of the decal and blended lists, the
//! registration order within a state, and the portal scissor of the
//! sectors seen through doorways.
//!
//! **The walk.** The original draws each render-state list sector by
//! sector: the frame walks the scene's sectors in scene order (the scene's
//! own sector first, then the rooms in the order they were appended),
//! skips a sector whose visibility word shares no bit with the renderer's
//! mask, sets the sector's rectangle as the region when it has one, and
//! then walks the list's states in list order, drawing that sector's
//! entries of each state from the sector's per-material list, in the order
//! registration appended them. The decal list is drawn this way for every
//! sector before the blended list is, and the blended list again sector by
//! sector (`docs/RENDERING.md`, Pass order). So two blended surfaces in
//! different rooms draw in room order, not in state order across the whole
//! scene, and two surfaces of one state in one room in registration order,
//! not by distance.
//!
//! Bevy owns the transparent phase. [`crate::passes::order_bias`] already
//! puts each surface's state rank on its sort value (decals, then blended
//! ranks, then everything without a state by distance); this module adds
//! the sector and the registration order: after Bevy's sort, the phase is
//! re-sorted, stably, by [`walk_key`] (list, sector, rank, registration
//! position), where the list and rank are read back from the sort value
//! and the sector and position come from [`DrawSectors`], which the live
//! scene (`exa_world::scene_live`) keeps for every mesh it registered. A
//! surface the scene does not hold keeps Bevy's distance order after the
//! registered ones of its rank. A scene with no rooms (every arena and
//! outdoor level) leaves the phase alone.
//!
//! **The scissor.** Each frame the scene classification gives a sector
//! seen only through doorways the screen rectangle of its doorway frusta,
//! and its meshes the `0x1000_0000` bit unless they straddle the doorway.
//! The original's list walk queues those entries with a scissor flag; the
//! queue's flush turns the flag into a draw clipped to the rectangle only
//! when the view is super-sampled: each scissored entry is drawn, in each
//! of the four sub-frames, into a viewport the size of the rectangle with
//! the projection cropped to it, which clips it to the rectangle's pixels
//! of that sub-frame. In a single view the flush notes the flag change and
//! does nothing with it (the call is compiled out), so nothing is clipped.
//! Exarust's super-sampled scene is one image at twice the window's size,
//! each window pixel a 2 × 2 block of sub-frame samples, so the rectangle
//! in window pixels, doubled, is the same set of samples; it is set with
//! the render pass's scissor ([`SectorScissors`], [`sub_frame_scissor`]).
//! The original clips the alpha-tested and opaque lists the same way. Bevy
//! batches its opaque and alpha-mask phases into multi-draws, where one
//! draw cannot carry its own scissor, so each visible scissored opaque or
//! alpha-tested mesh of the scene view leaves its phase and draws, with its
//! own pipeline, on the transparent phase in the list blocks ahead of
//! everything else there ([`crate::passes::PhaseList`]); depth writes make
//! the later draw invisible but for the clip. The scissor goes only on the
//! lists the original walks by sector (the alpha-tested, opaque, decal and
//! blended ones), not on the grass hook, the faded pass or the particles.

use bevy::core_pipeline::core_3d::{
    AlphaMask3d, Opaque3d, Transparent3d, TransparentSortingInfo3d,
};
use bevy::ecs::entity::EntityHashMap;
use bevy::pbr::{
    MainPassTransparentDrawFunction, MeshInputUniform, MeshUniform, PreparedMaterial,
    RenderMaterialInstances, RenderMeshInstances, SpecializedMaterialPipelineCache,
    get_mesh_instance_world_from_local,
};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::Extract;
use bevy::render::batching::gpu_preprocessing::BatchedInstanceBuffers;
use bevy::render::camera::{DirtySpecializationSystems, DirtySpecializations};
use bevy::render::erased_render_asset::ErasedRenderAssets;
use bevy::render::mesh::RenderMesh;
use bevy::render::mesh::allocator::MeshAllocator;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::ViewBinnedRenderPhases;
use bevy::render::render_phase::{
    Draw, DrawError, DrawFunctionId, DrawFunctions, PhaseItem, PhaseItemExtraIndex,
    TrackedRenderPass, ViewSortedRenderPhases, sort_phase_system,
};
use bevy::render::render_resource::CachedRenderPipelineId;
use bevy::render::sync_world::MainEntity;
use bevy::render::sync_world::MainEntityHashSet;
use bevy::render::view::ExtractedView;
use bevy::render::view::RenderVisibleEntities;
use bevy::render::{ExtractSchedule, Render, RenderApp, RenderSystems};

#[cfg(test)]
use crate::passes::order_bias;
use crate::passes::{MAX_RANKS, PhaseList, RANK_SPACING};

/// A registered mesh's place in the walk: the position of its sector in the
/// scene's sector list (0 for the scene's own sector) and its position in
/// that sector's mesh list (registration order).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrawSlot {
    pub sector: u32,
    pub order: u32,
}

/// Every mesh the live scene registered, with its [`DrawSlot`].
/// `generation` changes whenever the map does, so the render world copies
/// it only then.
#[derive(Resource, Clone, Debug)]
pub struct DrawSectors {
    pub map: EntityHashMap<DrawSlot>,
    pub generation: u64,
    /// `--no-registration-order` leaves every slot's order at 0, so one
    /// state in one sector keeps Bevy's distance order (to compare).
    pub registration_order: bool,
}

impl Default for DrawSectors {
    fn default() -> Self {
        Self {
            map: EntityHashMap::default(),
            generation: 0,
            registration_order: true,
        }
    }
}

/// A sector's portal rectangle in window pixels: left, top, right, bottom
/// (right and bottom exclusive), as the scene classification rounds them.
pub type WindowRect = [i32; 4];

/// The portal scissor of this frame, written by the live scene.
#[derive(Resource, Clone, Debug)]
pub struct SectorScissors {
    /// `--no-sector-scissor` turns the scissor off, to compare.
    pub enabled: bool,
    /// The scene camera the rectangles were projected for.
    pub camera: Option<Entity>,
    /// Whether that camera draws super-sampled (the only case in which the
    /// original clips).
    pub supersampled: bool,
    /// Each sector that has a rectangle this frame, by its position in the
    /// scene's sector list.
    pub rects: Vec<(u32, WindowRect)>,
    /// The meshes drawn with the scissor bit, each with its sector.
    pub entries: EntityHashMap<u32>,
    /// Changes whenever `entries` does.
    pub generation: u64,
}

impl Default for SectorScissors {
    fn default() -> Self {
        Self {
            enabled: true,
            camera: None,
            supersampled: false,
            rects: Vec::new(),
            entries: EntityHashMap::default(),
            generation: 0,
        }
    }
}

/// The render world's copy of [`DrawSectors`].
#[derive(Resource, Clone, Debug, Default)]
struct RenderDrawSectors {
    map: EntityHashMap<DrawSlot>,
    generation: u64,
}

/// The render world's copy of [`SectorScissors`].
#[derive(Resource, Clone, Debug, Default)]
struct RenderSectorScissors {
    active: bool,
    camera: Option<Entity>,
    rects: Vec<(u32, WindowRect)>,
    entries: EntityHashMap<u32>,
    generation: u64,
}

/// The scissor rectangles the scissor items of this frame set, in target
/// pixels (`x`, `y`, `width`, `height`); the first is the whole viewport.
#[derive(Resource, Clone, Debug, Default)]
struct ScissorRects(Vec<[u32; 4]>);

/// The draw function of the scissor items.
#[derive(Resource, Clone, Copy, Debug)]
struct ScissorDrawFunction(DrawFunctionId);

/// Render-world entities that key the scissor items in the phase.
#[derive(Resource, Clone, Debug, Default)]
struct ScissorKeys(Vec<Entity>);

pub struct SectorWalkPlugin;

impl Plugin for SectorWalkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawSectors>()
            .init_resource::<SectorScissors>();
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<RenderDrawSectors>()
            .init_resource::<RenderSectorScissors>()
            .init_resource::<ScissorRects>()
            .init_resource::<ScissorKeys>()
            .init_resource::<MovedOpaque>()
            .add_systems(
                ExtractSchedule,
                (
                    extract_sectors,
                    extract_scissors.in_set(DirtySpecializationSystems::CheckForChanges),
                ),
            )
            .add_systems(
                Render,
                move_scissored_opaque
                    .in_set(RenderSystems::QueueMeshes)
                    .after(bevy::pbr::queue_material_meshes),
            )
            .add_systems(
                Render,
                (walk_sectors, insert_scissors)
                    .chain()
                    .in_set(RenderSystems::PhaseSort)
                    .after(sort_phase_system::<Transparent3d>),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        let world = render_app.world_mut();
        let Some(functions) = world.get_resource::<DrawFunctions<Transparent3d>>() else {
            return;
        };
        let id = functions.write().add(ScissorDraw);
        world.insert_resource(ScissorDrawFunction(id));
    }
}

fn extract_sectors(main: Extract<Option<Res<DrawSectors>>>, mut render: ResMut<RenderDrawSectors>) {
    let Some(main) = main.as_deref() else {
        return;
    };
    if main.generation != render.generation {
        render.map.clone_from(&main.map);
        render.generation = main.generation;
    }
}

/// Copies the scissor of this frame. The scissored opaque and alpha-tested
/// meshes leave Bevy's opaque phases for the transparent one
/// ([`move_scissored_opaque`]); a mesh that stops being scissored is marked
/// for Bevy to specialize and queue again, which takes its moved item away
/// and puts it back in its own phase.
fn extract_scissors(
    main: Extract<Option<Res<SectorScissors>>>,
    mut render: ResMut<RenderSectorScissors>,
    mut moved: ResMut<MovedOpaque>,
    mut dirty: ResMut<DirtySpecializations>,
) {
    let Some(main) = main.as_deref() else {
        render.active = false;
        return;
    };
    render.active = main.enabled && main.supersampled && !main.rects.is_empty();
    render.camera = main.camera;
    render.rects.clone_from(&main.rects);
    if main.generation != render.generation {
        render.entries.clone_from(&main.entries);
        render.generation = main.generation;
    }
    let active = render.active;
    let entries = &render.entries;
    moved.0.retain(|entity| {
        let keep = active && entries.contains_key(&entity.id());
        if !keep {
            dirty.changed_renderables.insert(*entity);
        }
        keep
    });
}

/// The meshes [`move_scissored_opaque`] took out of Bevy's opaque phases.
#[derive(Resource, Clone, Debug, Default)]
struct MovedOpaque(MainEntityHashSet);

/// The original walks its alpha-tested and opaque lists sector by sector
/// too, and clips a sector seen through doorways to its doorway rectangle
/// for them as for the blended ones. Bevy's opaque phases are batched into
/// multi-draws, which cannot carry a scissor per mesh, so each visible
/// scissored mesh of the scene view leaves its opaque or alpha-mask phase
/// and is drawn, with the same pipeline (depth writes on, no blending), on
/// the transparent phase in the list block ahead of the grass hook
/// ([`PhaseList::ScissoredAlphaTest`], [`PhaseList::ScissoredOpaque`]),
/// where [`insert_scissors`] clips it. Depth writes make the later draw
/// invisible except for the clip. Bevy takes the moved item away whenever
/// it dequeues the mesh (hidden, or marked by [`extract_scissors`]).
#[allow(clippy::too_many_arguments)]
fn move_scissored_opaque(
    scissors: Res<RenderSectorScissors>,
    views: Query<(&ExtractedView, &RenderVisibleEntities)>,
    mut opaque_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    mut alpha_mask_phases: ResMut<ViewBinnedRenderPhases<AlphaMask3d>>,
    mut transparent_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    pipelines: Res<SpecializedMaterialPipelineCache>,
    material_instances: Res<RenderMaterialInstances>,
    materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    mesh_instances: Res<RenderMeshInstances>,
    meshes: Res<RenderAssets<RenderMesh>>,
    allocator: Res<MeshAllocator>,
    instance_buffers: Option<Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>>,
    mut moved: ResMut<MovedOpaque>,
) {
    let Some(camera) = scissors.camera else {
        return;
    };
    if !scissors.active || scissors.entries.is_empty() {
        return;
    }
    let Some((view, visible)) = views.iter().find(|(v, _)| {
        let retained = v.retained_view_entity;
        retained.main_entity.id() == camera
            && retained.auxiliary_entity.is_none()
            && retained.subview_index == 0
    }) else {
        return;
    };
    let key = view.retained_view_entity;
    let (Some(opaque), Some(alpha_mask), Some(transparent), Some(view_pipelines)) = (
        opaque_phases.get_mut(&key),
        alpha_mask_phases.get_mut(&key),
        transparent_phases.get_mut(&key),
        pipelines.get(&key),
    ) else {
        return;
    };
    let Some(visible) = visible.get::<Mesh3d>() else {
        return;
    };
    for (_, &main) in visible.iter_visible() {
        if !scissors.entries.contains_key(&main.id()) {
            continue;
        }
        let item_key = (Entity::PLACEHOLDER, main);
        if transparent.items.contains_key(&item_key) {
            if moved.0.contains(&main) {
                // Bevy may have queued it again in its own phase.
                opaque.remove(main);
                alpha_mask.remove(main);
            }
            continue;
        }
        let Some(&pipeline) = view_pipelines.get(&main) else {
            continue;
        };
        let Some(material) = material_instances
            .instances
            .get(&main)
            .and_then(|instance| materials.get(instance.asset_id))
        else {
            continue;
        };
        let properties = &material.properties;
        let list = match properties.alpha_mode {
            AlphaMode::Opaque => PhaseList::ScissoredOpaque,
            AlphaMode::Mask(_) => PhaseList::ScissoredAlphaTest,
            _ => continue,
        };
        let Some(draw_function) = properties.get_draw_function(MainPassTransparentDrawFunction)
        else {
            continue;
        };
        let Some(mesh) = mesh_instances.render_mesh_queue_data(main) else {
            continue;
        };
        let Some(slabs) = allocator.mesh_slabs(&mesh.mesh_asset_id()) else {
            continue;
        };
        let centre = meshes
            .get(mesh.mesh_asset_id())
            .map_or(Vec3::ZERO, |m| m.aabb_center);
        let mesh_center = get_mesh_instance_world_from_local(
            main,
            mesh.current_uniform_index,
            &mesh_instances,
            instance_buffers.as_deref(),
        )
        .transform_point3(centre);
        opaque.remove(main);
        alpha_mask.remove(main);
        transparent.add_retained(Transparent3d {
            sorting_info: TransparentSortingInfo3d::Sorted {
                mesh_center,
                depth_bias: list.bias(),
            },
            distance: 0.0,
            pipeline,
            entity: item_key,
            draw_function,
            batch_range: 0..1,
            extra_index: PhaseItemExtraIndex::None,
            indexed: slabs.index_slab_id.is_some(),
        });
        moved.0.insert(main);
    }
}

/// Which of the walk's lists a transparent-phase sort value belongs to
/// ([`PhaseList::of_sort_value`]).
pub fn list_of_sort_value(value: f32) -> PhaseList {
    PhaseList::of_sort_value(value)
}

/// The state rank a sort value carries in its list: its offset from the
/// list's base in whole rank spacings, the view depth (well inside half a
/// spacing either way) rounded off. Only the decal and blended lists have
/// ranks; every other list's entries are rank 0.
pub fn rank_of_sort_value(value: f32, list: PhaseList) -> u32 {
    if !matches!(list, PhaseList::Decal | PhaseList::Blended) {
        return 0;
    }
    let rank = ((value - list.bias()) / RANK_SPACING + 0.5).floor();
    rank.clamp(0.0, MAX_RANKS as f32) as u32
}

/// The walk's key for one phase item: its list, and for the lists walked by
/// sector its sector, its state rank and its registration position in the
/// sector (the hooks and the late passes are not walked by sector). A
/// surface the scene does not hold is in the scene's own sector after the
/// registered ones of its rank.
pub fn walk_key(value: f32, slot: Option<DrawSlot>) -> (PhaseList, u32, u32, u32) {
    let list = list_of_sort_value(value);
    if !list.sector_walked() {
        return (list, 0, 0, 0);
    }
    let rank = rank_of_sort_value(value, list);
    match slot {
        Some(slot) => (list, slot.sector, rank, slot.order),
        None => (list, 0, rank, u32::MAX),
    }
}

/// Re-sorts each view's transparent phase into the sector-major walk:
/// stably by [`walk_key`], so Bevy's order (state rank, then distance)
/// stays for what the key does not tell apart.
fn walk_sectors(
    sectors: Res<RenderDrawSectors>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
) {
    if sectors.map.is_empty() {
        return;
    }
    for phase in phases.values_mut() {
        let slot_of = |item: &Transparent3d| sectors.map.get(&item.entity.1.id()).copied();
        let any = phase.items.values().any(|item| {
            list_of_sort_value(item.distance).sector_walked() && slot_of(item).is_some()
        });
        if !any {
            continue;
        }
        phase
            .items
            .sort_by_cached_key(|_, item| walk_key(item.distance, slot_of(item)));
    }
}

/// A sector's rectangle in one sub-frame of the super-sampled target:
/// window pixels times `scale` (the sub-frame samples per window pixel on
/// each axis), offset by the view's viewport (`x`, `y`, `width`, `height`)
/// and held inside it, as `x`, `y`, `width`, `height` for the render
/// pass's scissor. A rectangle turned inside out is empty.
pub fn sub_frame_scissor(rect: WindowRect, scale: u32, viewport: UVec4) -> [u32; 4] {
    let scale = i64::from(scale);
    let axis = |min: i32, max: i32, origin: u32, size: u32| {
        let lo = i64::from(origin);
        let hi = lo + i64::from(size);
        let a = (lo + i64::from(min) * scale).clamp(lo, hi);
        let b = (lo + i64::from(max) * scale).clamp(a, hi);
        (a as u32, (b - a) as u32)
    };
    let (x, width) = axis(rect[0], rect[2], viewport.x, viewport.z);
    let (y, height) = axis(rect[1], rect[3], viewport.y, viewport.w);
    [x, y, width, height]
}

/// Where the scissor changes along the phase: for each item in draw order
/// the rectangle it is drawn with (`None` the whole view), the scissor
/// starting off. Returns (item position, rectangle) for each change, the
/// switch going right before that item.
pub fn scissor_changes(keys: &[Option<usize>]) -> Vec<(usize, Option<usize>)> {
    let mut current = None;
    let mut out = Vec::new();
    for (at, &key) in keys.iter().enumerate() {
        if key != current {
            out.push((at, key));
            current = key;
        }
    }
    out
}

/// Counts for the periodic log line.
#[derive(Default)]
struct ScissorStats {
    frames: u64,
    clipping: u32,
    most: usize,
    switches: usize,
}

/// Puts a scissor item in the scene view's transparent phase wherever the
/// scissor changes: before a run of entries of one sector seen through
/// doorways, and before the first entry after them without the bit. The
/// items are transient, so the next frame's phase drops them.
#[allow(clippy::too_many_arguments)]
fn insert_scissors(
    mut commands: Commands,
    scissors: Res<RenderSectorScissors>,
    draw: Option<Res<ScissorDrawFunction>>,
    views: Query<&ExtractedView>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    mut rects: ResMut<ScissorRects>,
    mut keys: ResMut<ScissorKeys>,
    mut stats: Local<ScissorStats>,
) {
    rects.0.clear();
    let (Some(draw), Some(camera)) = (draw, scissors.camera) else {
        return;
    };
    if !scissors.active || scissors.entries.is_empty() {
        return;
    }
    let Some(view) = views.iter().find(|v| {
        let retained = v.retained_view_entity;
        retained.main_entity.id() == camera
            && retained.auxiliary_entity.is_none()
            && retained.subview_index == 0
    }) else {
        return;
    };
    let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
        return;
    };
    let viewport = view.viewport;
    rects
        .0
        .push([viewport.x, viewport.y, viewport.z, viewport.w]);
    let mut slot_of_sector: HashMap<u32, usize> = HashMap::default();
    for &(sector, rect) in &scissors.rects {
        slot_of_sector.insert(sector, rects.0.len());
        rects
            .0
            .push(sub_frame_scissor(rect, crate::supersample::SCALE, viewport));
    }
    let item_keys: Vec<Option<usize>> = phase
        .items
        .values()
        .map(|item| {
            // The hooks, the faded pass and the particles draw unclipped.
            if !list_of_sort_value(item.distance).sector_walked() {
                return None;
            }
            scissors
                .entries
                .get(&item.entity.1.id())
                .and_then(|sector| slot_of_sector.get(sector).copied())
        })
        .collect();
    let changes = scissor_changes(&item_keys);
    let clipped = item_keys.iter().filter(|k| k.is_some()).count();
    stats.frames += 1;
    if clipped > 0 {
        stats.clipping += 1;
        stats.most = stats.most.max(clipped);
        stats.switches = stats.switches.max(changes.len());
    }
    if stats.frames.is_multiple_of(240) {
        info!(
            "sector scissor: {} of the last 240 frames clipped transparent surfaces to a doorway rectangle (at most {} surfaces and {} switches in a frame); now {} of {} surfaces, {} rectangles, the first {:?} (x, y, width, height) in the viewport {:?}",
            stats.clipping,
            stats.most,
            stats.switches,
            clipped,
            item_keys.len(),
            scissors.rects.len(),
            rects.0.get(1),
            rects.0.first(),
        );
        stats.clipping = 0;
        stats.most = 0;
        stats.switches = 0;
    }
    if changes.is_empty() {
        return;
    }
    while keys.0.len() < changes.len() {
        keys.0.push(commands.spawn_empty().id());
    }
    let old = std::mem::take(&mut phase.items);
    phase.items.reserve(old.len() + changes.len());
    let mut next = changes.iter().enumerate().peekable();
    for (at, (key, item)) in old.into_iter().enumerate() {
        while let Some(&(n, &(change_at, rect))) = next.peek() {
            if change_at != at {
                break;
            }
            next.next();
            let index = rect.unwrap_or(0) as u32;
            let scissor_key = (keys.0[n], MainEntity::from(Entity::PLACEHOLDER));
            phase.items.insert(
                scissor_key,
                Transparent3d {
                    sorting_info: TransparentSortingInfo3d::AlwaysOnTop,
                    distance: item.distance,
                    pipeline: CachedRenderPipelineId::INVALID,
                    entity: scissor_key,
                    draw_function: draw.0,
                    batch_range: index..index + 1,
                    extra_index: PhaseItemExtraIndex::None,
                    indexed: false,
                },
            );
            phase.transient_items.push(scissor_key);
        }
        phase.items.insert(key, item);
    }
}

/// The scissor item's draw: sets the render pass's scissor to its
/// rectangle (the item's batch range indexes [`ScissorRects`]) and draws
/// nothing.
struct ScissorDraw;

impl Draw<Transparent3d> for ScissorDraw {
    fn draw<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        _view: Entity,
        item: &Transparent3d,
    ) -> Result<(), DrawError> {
        let rects = &world.resource::<ScissorRects>().0;
        let rect = rects
            .get(item.batch_range().start as usize)
            .or_else(|| rects.first());
        if let Some(&[x, y, width, height]) = rect {
            pass.set_scissor_rect(x, y, width, height);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(sector: u32, order: u32) -> Option<DrawSlot> {
        Some(DrawSlot { sector, order })
    }

    #[test]
    fn the_lists_read_back_from_the_bias() {
        use PhaseList as L;
        assert_eq!(list_of_sort_value(order_bias(true, 0) + 3.0), L::Decal);
        assert_eq!(
            list_of_sort_value(order_bias(true, MAX_RANKS) + 1000.0),
            L::Decal
        );
        assert_eq!(list_of_sort_value(order_bias(false, 0)), L::Blended);
        assert_eq!(
            list_of_sort_value(order_bias(false, MAX_RANKS) + 1000.0),
            L::Blended
        );
        assert_eq!(list_of_sort_value(0.5), L::Late);
        assert_eq!(list_of_sort_value(-0.5), L::Late);
        assert_eq!(list_of_sort_value(f32::NAN), L::Late);
        for list in [
            L::ScissoredAlphaTest,
            L::ScissoredOpaque,
            L::GrassHook,
            L::Decal,
            L::Blended,
        ] {
            for depth in [0.0, 0.5, 999.0] {
                assert_eq!(list_of_sort_value(list.bias() + depth), list);
            }
        }
        for late in [
            crate::passes::FADED_DEPTH_BIAS,
            crate::passes::FADED_COLOUR_BIAS,
        ] {
            assert_eq!(list_of_sort_value(late + 999.0), L::Late);
        }
    }

    #[test]
    fn the_rank_reads_back_from_the_bias() {
        for rank in [0, 1, 7, 300, MAX_RANKS] {
            for depth in [-5.0, 0.0, 0.3, 999.0] {
                assert_eq!(
                    rank_of_sort_value(order_bias(true, rank) + depth, PhaseList::Decal),
                    rank as u32
                );
                assert_eq!(
                    rank_of_sort_value(order_bias(false, rank) + depth, PhaseList::Blended),
                    rank as u32
                );
            }
        }
    }

    #[test]
    fn decals_then_blended_walk_by_sector() {
        // (sort value, slot) in Bevy's order.
        let items = [
            (order_bias(true, 0) + 5.0, slot(2, 0)),
            (order_bias(true, 1) + 1.0, slot(1, 0)),
            (order_bias(false, 0) + 9.0, slot(2, 0)),
            (order_bias(false, 0) + 2.0, None),
            (order_bias(false, 3) + 1.0, slot(1, 0)),
            (4.0, slot(2, 0)),
            (7.0, slot(1, 0)),
        ];
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by_key(|&i| walk_key(items[i].0, items[i].1));
        // Decals: sector 1 then 2; blended: sector 0, 1, 2; the rest by
        // distance, sectors ignored.
        assert_eq!(order, vec![1, 0, 3, 4, 2, 5, 6]);
    }

    /// The scissored alpha-tested meshes, then the scissored opaque ones,
    /// each walked by sector; then the grass hook (not by sector), the
    /// decals, and the faded pass last.
    #[test]
    fn the_scissored_lists_walk_by_sector_ahead_of_the_hooks() {
        let items = [
            (PhaseList::Decal.bias() + 1.0, slot(1, 0)),
            (PhaseList::GrassHook.bias() + 5.0, slot(2, 0)),
            (PhaseList::ScissoredOpaque.bias() + 3.0, slot(2, 1)),
            (PhaseList::ScissoredOpaque.bias() + 9.0, slot(1, 4)),
            (PhaseList::ScissoredAlphaTest.bias() + 1.0, slot(2, 0)),
            (crate::passes::FADED_DEPTH_BIAS + 1.0, None),
            (PhaseList::GrassHook.bias() + 2.0, slot(1, 0)),
        ];
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by_key(|&i| walk_key(items[i].0, items[i].1));
        // The grass keeps Bevy's order (here, the input's).
        assert_eq!(order, vec![4, 3, 2, 1, 6, 0, 5]);
    }

    #[test]
    fn one_state_in_one_sector_walks_in_registration_order() {
        // Bevy's order is by distance; the walk takes registration order
        // within a state and sector.
        let items = [
            (order_bias(false, 2) + 1.0, slot(3, 9)),
            (order_bias(false, 2) + 2.0, None),
            (order_bias(false, 2) + 3.0, slot(3, 4)),
            (order_bias(false, 2) + 4.0, slot(3, 6)),
            (order_bias(false, 1) + 50.0, slot(3, 7)),
        ];
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by_key(|&i| walk_key(items[i].0, items[i].1));
        // The unregistered one is in the scene's own sector, walked first.
        assert_eq!(order, vec![1, 4, 2, 3, 0]);
    }

    #[test]
    fn a_sector_rectangle_doubles_into_the_sub_frames() {
        let viewport = UVec4::new(0, 0, 2560, 1440);
        assert_eq!(
            sub_frame_scissor([100, 50, 300, 400], 2, viewport),
            [200, 100, 400, 700]
        );
        // Held inside the viewport, and offset by it.
        assert_eq!(
            sub_frame_scissor([-10, 700, 1300, 800], 2, viewport),
            [0, 1400, 2560, 40]
        );
        assert_eq!(
            sub_frame_scissor([5, 5, 10, 10], 2, UVec4::new(100, 20, 50, 50)),
            [110, 30, 10, 10]
        );
        // Inside out: empty.
        assert_eq!(sub_frame_scissor([300, 50, 100, 400], 2, viewport)[2], 0);
    }

    #[test]
    fn the_scissor_switches_where_the_rectangle_changes() {
        assert!(scissor_changes(&[None, None]).is_empty());
        assert_eq!(
            scissor_changes(&[None, Some(1), Some(1), Some(2), None, None, Some(2)]),
            vec![(1, Some(1)), (3, Some(2)), (4, None), (6, Some(2))]
        );
    }
}
