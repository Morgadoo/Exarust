//! The occupancy grid of the original's voxel global illumination: a block
//! of 20 cm voxels, 256 × 256 across (48 m plus margins) and 64 or 128
//! high, below the level's highest point. Meshes mark their cells solid by
//! splatting their GI sample points, each tile-grid cell's floor gets the
//! original's floor heightfield points (`exa_physics::voxel`), a solid layer
//! under its surface, and the top two slices form a soft lid.
//! Light then spreads through the open cells each frame (`exa_render::gi`).
//! Rules: `docs/RENDERING.md` (Voxel GI).
//!
//! A block is built on the async compute pool from a shared copy of the
//! level's GI inputs, and handed to the renderer when it is done: the build
//! takes seconds on a large outdoor level, and the window must keep
//! answering meanwhile.

use std::sync::Arc;

use std::collections::BTreeMap;

use bevy::camera::primitives::{Aabb, Frustum};
use bevy::math::Affine3A;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use bevy::transform::TransformSystems;
use exa_physics::voxel::{self, Bounds};
use exa_render::gi::{GiSector, GiSectorLights, GiSectors, GiVolume};

use crate::shadow_links::LevelWalls;

pub struct GiVolumePlugin;

impl Plugin for GiVolumePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GiSettings>()
            .init_resource::<GiState>()
            .init_resource::<GiSectorLights>()
            .add_systems(
                PostUpdate,
                (collect_source, follow_camera, publish_sector_lights)
                    .chain()
                    .after(TransformSystems::Propagate)
                    .after(crate::shadow_links::LightCells)
                    .before(exa_render::gi::GiSystems),
            );
    }
}

/// The GI sample points of a mesh node, in its space (metres).
#[derive(Component, Clone)]
pub struct GiPoints(pub Arc<Vec<Vec3>>);

/// A tile-grid floor mesh: with the other floor meshes of its grid cell it
/// makes the cell's floor, whose upward faces become a solid layer of
/// voxels just under the surface, as the original generates for each
/// cell's merged floor.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct GiFloor {
    /// The grid cell the mesh belongs to: its centre on the grid, in whole
    /// centimetres (x, z).
    pub cell: IVec2,
    /// From the mesh to the cell's own space: origin at the cell centre,
    /// the grid's axes, metres.
    pub to_cell: Affine3A,
}

/// A terrain tile mesh: the voxels below its surface are solid.
#[derive(Component, Clone, Copy)]
pub struct GiTerrain;

/// A level waiting for its GI volume, once its nodes have world transforms.
#[derive(Resource, Debug, Clone)]
pub struct PendingGi {
    /// Centre of the level (X, Z used), metres.
    pub centre: Vec3,
    /// The level's highest and lowest points, metres.
    pub top: f32,
    pub bottom: f32,
    /// The level record's secondary ambient (0x30), as stored.
    pub base_ambient: [f32; 3],
    /// The 6 m cells the manager reads.
    pub grid: GiGrid,
}

/// The level's 6 m cells as the GI manager reads them: plane A's open flag
/// `0x80000`, row-major. A level without a grid uses the original's
/// synthetic 8 × 8 grid of open cells.
#[derive(Debug, Clone)]
pub struct GiGrid {
    pub width: u32,
    pub height: u32,
    /// Plane A per cell, row-major.
    pub edges: Arc<Vec<u32>>,
}

impl GiGrid {
    /// The synthetic grid of a level without one.
    pub fn synthetic() -> Self {
        Self {
            width: 8,
            height: 8,
            edges: Arc::new(vec![OPEN; 8 * 8]),
        }
    }

    /// Whether a cell is built (plane A's open flag).
    pub fn open(&self, x: i64, z: i64) -> bool {
        x >= 0
            && z >= 0
            && (x as u32) < self.width
            && (z as u32) < self.height
            && self
                .edges
                .get((z as u32 * self.width + x as u32) as usize)
                .is_some_and(|e| e & OPEN != 0)
    }
}

/// Plane A's bit for a built cell.
const OPEN: u32 = 0x8_0000;

/// Whether levels get the voxel GI, and how many propagation steps run at
/// once when one is built (light otherwise builds up one step per frame).
#[derive(Resource, Debug, Clone, Copy)]
pub struct GiSettings {
    pub enabled: bool,
    pub warm_up: u32,
}

impl Default for GiSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            warm_up: 600,
        }
    }
}

/// Where the level's GI block stands, for whoever waits for it (the
/// loading screen, which then waits for the warm-up,
/// `exa_render::gi::GiWarmUp`).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GiState {
    /// No level asked for GI, or GI is off.
    #[default]
    Off,
    /// The level's fires are still ramping up; the block waits for them.
    Settling,
    /// A block is being built, or one is wanted around the camera.
    Building,
    /// The block around the camera is built and handed to the renderer.
    Built,
}

/// Asks for the block around the camera's present focus, even when the
/// block built is within the rebuild distance of it. A loading screen asks
/// once the view the level starts with is in place, so the first frame
/// shown is lit around it, as the original builds its block after loading.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct GiRefocus;

/// The level record's secondary ambient (`0x30`) scale for the per-step base
/// ambient, at the original's top shading quality.
pub const BASE_AMBIENT: f32 = 0.001;

/// The level's GI inputs in world space, collected once its nodes have
/// settled: every sample point, every floor point and every terrain
/// triangle. Shared with the block builds.
#[derive(Resource)]
struct GiSource {
    inputs: SourceInputs,
    level: PendingGi,
}

/// The inputs, put in world space on the async compute pool (millions of
/// points and triangles on a large level).
enum SourceInputs {
    Collecting(Task<GiInputs>),
    Ready(Arc<GiInputs>),
}

struct GiInputs {
    meshes: usize,
    points: Vec<Vec3>,
    /// The grid cells' floor points, world space.
    floor_points: Vec<Vec3>,
    terrain: Vec<[Vec3; 3]>,
}

/// A grid cell's floor as the original merges it: the triangles of every
/// floor mesh of the cell, in the cell's own space (metres, origin at the
/// cell centre), and where that space sits in the world.
struct FloorCell {
    triangles: Vec<[Vec3; 3]>,
    to_world: Affine3A,
}

impl FloorCell {
    /// The cell's floor points (world, metres), from the original's floor
    /// heightfield at its 20 cm parameter, run once per cell in the cell's
    /// space as the original runs it on the cell's `+Floor` mesh.
    ///
    /// The generator keeps the original's centimetres: the triangles and
    /// their bounds are scaled up, and the points scaled back.
    fn points(&self) -> Vec<Vec3> {
        let triangles: Vec<[Vec3; 3]> = self
            .triangles
            .iter()
            .map(|t| t.map(|p| p * CM_PER_M))
            .collect();
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for p in triangles.iter().flatten() {
            min = min.min(*p);
            max = max.max(*p);
        }
        if triangles.is_empty() {
            return Vec::new();
        }
        let bounds = Bounds {
            centre: (min + max) * 0.5,
            half_extents: (max - min) * 0.5,
        };
        voxel::floor_points(bounds, &triangles, FLOOR_PARAMETER)
            .into_iter()
            .map(|p| self.to_world.transform_point3(p / CM_PER_M))
            .collect()
    }
}

/// Merges the floor meshes into their grid cells, in cell order (the
/// meshes of a cell in the order given).
fn floor_cells(meshes: &[(MeshCopy, GiFloor)]) -> Vec<FloorCell> {
    let mut cells: BTreeMap<(i32, i32), FloorCell> = BTreeMap::new();
    for (mesh, floor) in meshes {
        let cell = cells
            .entry((floor.cell.x, floor.cell.y))
            .or_insert_with(|| FloorCell {
                triangles: Vec::new(),
                // The mesh's world placement less its place in the cell.
                to_world: mesh.at * floor.to_cell.inverse(),
            });
        cell.triangles.extend(triangles_of(mesh, floor.to_cell));
    }
    cells.into_values().collect()
}

/// A mesh's positions and triangle corners with its world transform, owned
/// for the collection.
struct MeshCopy {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
    at: bevy::math::Affine3A,
}

/// The block now built: the sector-aligned origin it was built at.
#[derive(Resource)]
struct GiBlock {
    origin: Vec3,
}

/// The block being built off the main thread; at most one at a time.
#[derive(Resource)]
struct GiBuild(Task<BuiltBlock>);

/// A finished block and what its build counted.
struct BuiltBlock {
    occupancy: Occupancy,
    floors: Vec<u8>,
    terrain_columns: usize,
    floor_points: usize,
}

/// How far ahead of the camera the block is centred (metres), as the
/// original keeps its blocks around a point 20 m in front of the camera.
const FOCUS_AHEAD: f32 = 20.0;
/// The block is rebuilt once its centre would move by more than this many
/// sectors.
const REBUILD_SECTORS: f32 = 2.0;

#[allow(clippy::too_many_arguments)]
fn collect_source(
    mut commands: Commands,
    pending: Option<Res<PendingGi>>,
    settings: Res<GiSettings>,
    points: Query<(&GiPoints, &GlobalTransform)>,
    floors: Query<(&Mesh3d, &GlobalTransform, &GiFloor)>,
    terrain: Query<(&Mesh3d, &GlobalTransform), With<GiTerrain>>,
    meshes: Res<Assets<Mesh>>,
    fires: Query<&crate::fire::Fire>,
    mut state: ResMut<GiState>,
) {
    let Some(pending) = pending else { return };
    // The warm-up injects the lights as they are now: wait for the fires to
    // ramp up.
    if fires.iter().any(|f| !f.settled()) {
        *state = GiState::Settling;
        return;
    }
    commands.remove_resource::<PendingGi>();
    commands.remove_resource::<GiBlock>();
    // A build for the previous level is dropped, which cancels it.
    commands.remove_resource::<GiBuild>();
    commands.remove_resource::<exa_render::gi::GiTextures>();
    commands.remove_resource::<exa_render::gi::GiPendingTextures>();
    if !settings.enabled {
        commands.remove_resource::<GiVolume>();
        commands.remove_resource::<GiSource>();
        *state = GiState::Off;
        return;
    }
    *state = GiState::Building;
    // Only shared or copied here; the world-space work runs elsewhere.
    let points: Vec<_> = points
        .iter()
        .map(|(gi, at)| (gi.0.clone(), at.affine()))
        .collect();
    let floors: Vec<(MeshCopy, GiFloor)> = floors
        .iter()
        .filter_map(|(mesh, at, floor)| Some((mesh_copy(mesh, at, &meshes)?, *floor)))
        .collect();
    let ground = mesh_copies(terrain.iter(), &meshes);
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let mut world = Vec::new();
        for (gi, m) in &points {
            world.extend(gi.iter().map(|&p| m.transform_point3(p)));
        }
        GiInputs {
            meshes: points.len(),
            points: world,
            floor_points: floor_cells(&floors)
                .iter()
                .flat_map(FloorCell::points)
                .collect(),
            terrain: world_triangles(&ground),
        }
    });
    commands.insert_resource(GiSource {
        inputs: SourceInputs::Collecting(task),
        level: pending.clone(),
    });
}

/// The positions and triangles of `meshes`, copied with their transforms.
fn mesh_copies<'a>(
    parts: impl Iterator<Item = (&'a Mesh3d, &'a GlobalTransform)>,
    meshes: &Assets<Mesh>,
) -> Vec<MeshCopy> {
    parts
        .filter_map(|(mesh, at)| mesh_copy(mesh, at, meshes))
        .collect()
}

/// One mesh's positions and triangles, copied with its transform.
fn mesh_copy(mesh: &Mesh3d, at: &GlobalTransform, meshes: &Assets<Mesh>) -> Option<MeshCopy> {
    let mesh = meshes.get(&mesh.0)?;
    Some(MeshCopy {
        positions: mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)?
            .as_float3()?
            .to_vec(),
        indices: mesh.indices()?.iter().map(|i| i as u32).collect(),
        at: at.affine(),
    })
}

/// The triangles of `meshes` in world space.
fn world_triangles(meshes: &[MeshCopy]) -> Vec<[Vec3; 3]> {
    let mut triangles = Vec::new();
    for mesh in meshes {
        triangles.extend(triangles_of(mesh, mesh.at));
    }
    triangles
}

/// A mesh's triangles, placed by `at`.
fn triangles_of(mesh: &MeshCopy, at: Affine3A) -> Vec<[Vec3; 3]> {
    let corners: Vec<Vec3> = mesh
        .positions
        .iter()
        .map(|&p| at.transform_point3(Vec3::from(p)))
        .collect();
    let mut triangles = Vec::new();
    for t in mesh.indices.as_chunks::<3>().0 {
        let corner = |i: u32| corners.get(i as usize).copied();
        if let (Some(a), Some(b), Some(c)) = (corner(t[0]), corner(t[1]), corner(t[2])) {
            triangles.push([a, b, c]);
        }
    }
    triangles
}

/// Keeps the block around the point ahead of the camera: starts a build
/// when that point has moved on, and hands the block to the renderer when
/// the build is done. A newer focus waits for the build in flight.
#[allow(clippy::too_many_arguments)]
fn follow_camera(
    mut commands: Commands,
    source: Option<ResMut<GiSource>>,
    block: Option<Res<GiBlock>>,
    build: Option<ResMut<GiBuild>>,
    refocus: Option<Res<GiRefocus>>,
    mut state: ResMut<GiState>,
    settings: Res<GiSettings>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
) {
    let Some(mut source) = source else { return };
    if let SourceInputs::Collecting(task) = &mut source.inputs {
        let Some(inputs) = block_on(poll_once(task)) else {
            return;
        };
        info!(
            "GI source: {} meshes, {} points, {} floor points, {} terrain triangles",
            inputs.meshes,
            inputs.points.len(),
            inputs.floor_points.len(),
            inputs.terrain.len()
        );
        source.inputs = SourceInputs::Ready(Arc::new(inputs));
    }
    let SourceInputs::Ready(inputs) = &source.inputs else {
        return;
    };
    let inputs = inputs.clone();
    let level = source.level.clone();
    let mut origin = block.as_ref().map(|b| b.origin);
    if let Some(mut build) = build {
        let Some(built) = block_on(poll_once(&mut build.0)) else {
            return;
        };
        commands.remove_resource::<GiBuild>();
        let occupancy = built.occupancy;
        info!(
            "GI block: {} terrain columns, {} floor voxels, origin {:.2}, {} slices",
            built.terrain_columns, built.floor_points, occupancy.origin, occupancy.depth
        );
        origin = Some(occupancy.origin);
        commands.insert_resource(GiBlock {
            origin: occupancy.origin,
        });
        commands.insert_resource(GiVolume {
            origin: occupancy.origin,
            size: UVec3::new(SIDE as u32, SIDE as u32, occupancy.depth as u32),
            voxel: VOXEL,
            floors: Arc::new(built.floors),
            cells: Arc::new(occupancy.cells),
            base_ambient: Vec3::from(level.base_ambient) * BASE_AMBIENT,
            sector: occupancy.sector,
            warm_up: settings.warm_up,
        });
        *state = GiState::Built;
    }
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let focus = camera.translation() + camera.forward().as_vec3() * FOCUS_AHEAD;
    let (wanted, _, _) = Occupancy::placement(
        focus,
        level.top,
        level.bottom,
        (level.grid.width, level.grid.height),
    );
    if refocus.is_some() {
        commands.remove_resource::<GiRefocus>();
    }
    if let Some(origin) = origin {
        let moved = (wanted - origin).xz().abs().max_element();
        let allowed = if refocus.is_some() {
            0.0
        } else {
            REBUILD_SECTORS * SECTOR_M
        };
        if moved <= allowed {
            return;
        }
    }
    let task = AsyncComputeTaskPool::get().spawn(async move { build_block(focus, level, &inputs) });
    commands.insert_resource(GiBuild(task));
    *state = GiState::Building;
}

/// Marks the block's sectors the camera needs and lists each sector's 6 m
/// cell lights, as the original marks the sectors within six of the focus
/// point (37 cells squared), inside the view and the level's built cells,
/// then widens the marks by one sector; each block then draws a sector with
/// the lights its cell's flood window lists.
#[allow(clippy::too_many_arguments)]
fn publish_sector_lights(
    mut lights: ResMut<GiSectorLights>,
    source: Option<Res<GiSource>>,
    textures: Option<Res<exa_render::gi::GiTextures>>,
    pending: Option<Res<exa_render::gi::GiPendingTextures>>,
    walls: Option<Res<LevelWalls>>,
    cameras: Query<(&GlobalTransform, &Frustum), exa_render::supersample::SceneCamera>,
    point_lights: Query<&exa_render::lights::LightId>,
) {
    let Some(source) = source else { return };
    let textures = pending.as_ref().map(|p| &p.0).or(textures.as_deref());
    let Some(textures) = textures else { return };
    let grid = &source.level.grid;
    let camera = cameras.iter().next();
    let focus = camera.map(|(at, _)| at.translation() + at.forward().as_vec3() * FOCUS_AHEAD);
    let focus_cell = focus.map(|f| {
        IVec2::new(
            grid.width as i32 / 2 + (f.x / SECTOR_M).floor() as i32,
            grid.height as i32 / 2 + (f.z / SECTOR_M).floor() as i32,
        )
    });
    // Every built block fills the floors of its sectors; a sector of a block
    // not resident here still has 255, as the manager starts them all.
    let floor = |x: i32, z: i32| {
        let (bx, bz) = (x - textures.sector.x, z - textures.sector.y);
        if (0..SECTORS as i32).contains(&bx) && (0..SECTORS as i32).contains(&bz) {
            textures.sector_floor(bx as usize, bz as usize)
        } else {
            // A sector no built block has filled has no floor.
            u8::MAX
        }
    };
    let depth = textures.dims().z as f32;
    let marks = match (focus_cell, camera) {
        (Some(focus), Some((_, frustum))) => sector_marks(
            grid,
            focus,
            textures.sector,
            textures.origin,
            depth,
            floor,
            |centre, half| frustum_intersects(frustum, centre, half),
        ),
        // No scene camera: light the whole block (loading screens and
        // headless views have no frustum).
        _ => [true; SECTORS * SECTORS],
    };
    let mut state = GiSectors {
        drawn: marks,
        ..Default::default()
    };
    // A level without a wall grid lists every light in every sector: the
    // original's non-grid path uses one group of the scene's lights.
    let all: Vec<u32> = if walls.is_none() {
        let mut ids: Vec<u32> = point_lights.iter().map(|id| id.0).collect();
        ids.sort_unstable();
        ids
    } else {
        Vec::new()
    };
    for z in 0..SECTORS as i32 {
        for x in 0..SECTORS as i32 {
            let i = x as usize + z as usize * SECTORS;
            let ids: Vec<u32> = match &walls {
                Some(walls) => walls
                    .lights(
                        (textures.sector.x + x) as usize,
                        (textures.sector.y + z) as usize,
                    )
                    .collect(),
                None => all.clone(),
            };
            state.sectors[i] = GiSector::new(ids);
        }
    }
    *lights = GiSectorLights(Arc::new(state));
}

/// Whether a box centred at `centre` with `half` extents meets the view's
/// volume (the original tests the sector's 300 cm × half-span × 300 cm box
/// against the camera's convex volume).
fn frustum_intersects(frustum: &Frustum, centre: Vec3, half: Vec3) -> bool {
    frustum.intersects_obb(
        &Aabb::from_min_max(centre - half, centre + half),
        &Affine3A::IDENTITY,
        true,
        true,
    )
}

/// The sectors the manager draws this frame: cells within six of the focus
/// cell and 37 cells squared of it, built in plane A, whose mid-height point
/// the view sees; then widened by one sector. The original marks the
/// sectors in two passes over the same window, the second adding a marked
/// neighbour.
#[allow(clippy::too_many_arguments)]
fn sector_marks(
    grid: &GiGrid,
    focus: IVec2,
    block: IVec2,
    origin: Vec3,
    depth: f32,
    floor: impl Fn(i32, i32) -> u8,
    meets_view: impl Fn(Vec3, Vec3) -> bool,
) -> [bool; SECTORS * SECTORS] {
    let (w, h) = (grid.width as i32, grid.height as i32);
    let (focus_x, focus_z) = (focus.x, focus.y);
    let (lo_x, hi_x) = ((focus_x - 6).max(1), (focus_x + 6).min(w - 2));
    let (lo_z, hi_z) = ((focus_z - 6).max(1), (focus_z + 6).min(h - 2));
    let first = |x: i32, z: i32| -> bool {
        if x < lo_x || x > hi_x || z < lo_z || z > hi_z {
            return false;
        }
        let (dx, dz) = (x - focus_x, z - focus_z);
        if dx * dx + dz * dz >= 37 {
            return false;
        }
        if !grid.open(i64::from(x), i64::from(z)) {
            return false;
        }
        // The sector's box: its columns span half a 6 m cell either side of
        // its centre, and its slices from the floor to the volume's top.
        let floor = f32::from(floor(x, z));
        let centre = Vec3::new(
            origin.x + 4.6 + (x - block.x) as f32 * SECTOR_M,
            origin.y + (floor + depth) * 0.5 * VOXEL,
            origin.z + 4.6 + (z - block.y) as f32 * SECTOR_M,
        );
        let half = Vec3::new(
            SECTOR_M * 0.5,
            (depth - floor) * 0.5 * VOXEL,
            SECTOR_M * 0.5,
        );
        meets_view(centre, half)
    };
    let mut marks = [false; SECTORS * SECTORS];
    for z in 0..SECTORS as i32 {
        for x in 0..SECTORS as i32 {
            let (cx, cz) = (block.x + x, block.y + z);
            // The second pass runs over the same window only: a sector on
            // the grid's border is not drawn even if its neighbour is.
            let in_window = cx >= lo_x && cx <= hi_x && cz >= lo_z && cz <= hi_z;
            marks[(x + z * SECTORS as i32) as usize] = in_window
                && (first(cx, cz)
                    || first(cx - 1, cz)
                    || first(cx + 1, cz)
                    || first(cx, cz - 1)
                    || first(cx, cz + 1));
        }
    }
    marks
}

/// Builds the block centred on `focus`: terrain first, then the meshes'
/// points, then the floors, as the original builds a block.
fn build_block(focus: Vec3, level: PendingGi, inputs: &GiInputs) -> BuiltBlock {
    let mut occupancy = Occupancy::new(
        focus,
        level.top,
        level.bottom,
        (level.grid.width, level.grid.height),
    );
    let mut ground = TerrainColumns::new(&occupancy);
    for &t in &inputs.terrain {
        ground.add(t);
    }
    let terrain_columns = ground.apply(&mut occupancy);
    for &p in &inputs.points {
        occupancy.splat(p);
    }
    for &p in &inputs.floor_points {
        occupancy.splat(p);
    }
    let floor_points = inputs.floor_points.len();
    let floors = occupancy.floors();
    if let Some(path) = std::env::var_os("EXA_GI_DUMP") {
        let _ = std::fs::write(&path, &occupancy.cells);
        let mut floors_path = path.clone();
        floors_path.push(".floors");
        let _ = std::fs::write(floors_path, &floors);
    }
    BuiltBlock {
        occupancy,
        floors,
        terrain_columns,
        floor_points,
    }
}

/// Voxel edge, metres.
pub const VOXEL: f32 = 0.20;
/// Voxels across a block (X and Z).
pub const SIDE: usize = 256;
/// Voxels a sample point adds to its cells, before the trilinear split.
const POINT_WEIGHT: f32 = 20.0;
/// The soft lid: the top slice and the one below.
const LID: [u8; 2] = [26, 5];
/// Columns of margin on each side, then sectors of 30 columns (6 m).
const MARGIN: usize = 8;
const SECTOR: usize = 30;
const SECTORS: usize = 8;
const SECTOR_M: f32 = SECTOR as f32 * VOXEL;
/// Occupancy of a voxel below the terrain surface.
const TERRAIN_SOLID: u8 = 20;
/// The floor generator's parameter, in centimetres: the original's 20.
const FLOOR_PARAMETER: f32 = 20.0;
/// Centimetres to the metre.
const CM_PER_M: f32 = 100.0;

/// The top of the terrain over the block, per whole column (X fastest).
struct TerrainColumns {
    origin: Vec3,
    tops: Vec<f32>,
}

impl TerrainColumns {
    fn new(o: &Occupancy) -> Self {
        Self {
            origin: o.origin,
            tops: vec![f32::NEG_INFINITY; SIDE * SIDE],
        }
    }

    /// Raises the columns under an upward-facing triangle to its height.
    fn add(&mut self, t: [Vec3; 3]) {
        // Only faces turned up (counter-clockwise from above).
        let n = (t[1] - t[0]).cross(t[2] - t[0]);
        if n.y <= 1e-7 * n.length() {
            return;
        }
        let to_grid = |p: Vec3| ((p - self.origin) / VOXEL).xz();
        let [a, b, c] = t.map(to_grid);
        let lo = a.min(b).min(c).floor().max(Vec2::ZERO);
        let hi = a.max(b).max(c).ceil().min(Vec2::splat(SIDE as f32));
        let area = (b - a).perp_dot(c - a);
        for z in lo.y as usize..hi.y as usize {
            for x in lo.x as usize..hi.x as usize {
                let q = Vec2::new(x as f32 + 0.5, z as f32 + 0.5);
                let wa = (b - q).perp_dot(c - q) / area;
                let wb = (c - q).perp_dot(a - q) / area;
                let wc = 1.0 - wa - wb;
                if wa < -1e-5 || wb < -1e-5 || wc < -1e-5 {
                    continue;
                }
                let h = wa * t[0].y + wb * t[1].y + wc * t[2].y;
                let top = &mut self.tops[x + z * SIDE];
                *top = top.max(h);
            }
        }
    }

    /// Fills terrain as the original does (whole-voxel columns): voxels below
    /// the surface hold 20, the surface voxel its covered fraction of 20; the
    /// margins are left alone. Returns the columns filled.
    fn apply(&self, o: &mut Occupancy) -> usize {
        let mut filled = 0;
        for z in MARGIN..SIDE - MARGIN {
            for x in MARGIN..SIDE - MARGIN {
                let top = self.tops[x + z * SIDE];
                if !top.is_finite() {
                    continue;
                }
                let g = (top - self.origin.y) / VOXEL;
                if g <= 0.0 {
                    continue;
                }
                let surface = (g.floor() as usize).min(o.depth);
                for y in 0..surface {
                    o.cells[x + z * SIDE + y * SIDE * SIDE] = TERRAIN_SOLID;
                }
                if surface < o.depth {
                    let frac = g - surface as f32;
                    o.cells[x + z * SIDE + surface * SIDE * SIDE] =
                        (frac * TERRAIN_SOLID as f32).round_ties_even() as u8;
                }
                let column = &mut o.lowest[x + z * SIDE];
                *column = (*column).min(surface.min(o.depth - 1) as u8);
                filled += 1;
            }
        }
        filled
    }
}

/// A block's occupancy, X fastest, then Z, then Y (the texture's s, t, r),
/// with the lowest occupied slice of every column.
#[derive(Debug, Clone)]
pub struct Occupancy {
    /// World position of the block's lowest corner, metres.
    pub origin: Vec3,
    /// Voxels along Y.
    pub depth: usize,
    pub cells: Vec<u8>,
    /// Lowest slice a point fell in, per column (X fastest); 255 for none.
    pub lowest: Vec<u8>,
    /// The block's first 6 m cell in the level's sector grid.
    pub sector: IVec2,
}

impl Occupancy {
    /// An empty block centred on `centre` (X, Z) whose top is the level's
    /// highest point `top`; `bottom` (the lowest point) picks its height,
    /// and `grid` is the level's sector grid size (X, Z).
    pub fn new(centre: Vec3, top: f32, bottom: f32, grid: (u32, u32)) -> Self {
        let (origin, depth, sector) = Self::placement(centre, top, bottom, grid);
        let mut cells = vec![0u8; SIDE * SIDE * depth];
        for (slice, &v) in LID.iter().enumerate() {
            let y = depth - 1 - slice;
            cells[y * SIDE * SIDE..(y + 1) * SIDE * SIDE].fill(v);
        }
        Self {
            origin,
            depth,
            cells,
            lowest: vec![255; SIDE * SIDE],
            sector,
        }
    }

    /// Where a block centred on `centre` sits, how many voxels high it is
    /// and its first sector in the level's grid ([`Occupancy::new`]).
    ///
    /// The block covers eight sectors centred on the focus's cell, clamped
    /// inside the level's grid; the original's blocks are the same eight
    /// sectors, laid out over the built cells. Its top is the level's
    /// highest point, lowered to a whole voxel, and its height picks 64
    /// slices, or 128 when the level spans more than 12.8 m.
    pub fn placement(
        centre: Vec3,
        top: f32,
        bottom: f32,
        grid: (u32, u32),
    ) -> (Vec3, usize, IVec2) {
        // The level record's points are whole centimetres; the original's
        // integer division picks the height and the vertical offset.
        let top_cm = (top * CM_PER_M).round() as i32;
        let bottom_cm = (bottom * CM_PER_M).round() as i32;
        let depth: usize = if (top_cm - bottom_cm) / 20 > 64 {
            128
        } else {
            64
        };
        let below = (depth as i32 * 20 - top_cm) / 20;
        // The focus's cell, then a block of eight centred on it, clamped
        // inside the grid as the original's blocks are.
        let (w, h) = (grid.0.max(8) as i32, grid.1.max(8) as i32);
        let cell = |c: f32| (c / SECTOR_M).floor() as i32;
        let sector_x = (w / 2 + cell(centre.x) - SECTORS as i32 / 2).clamp(0, w - SECTORS as i32);
        let sector_z = (h / 2 + cell(centre.z) - SECTORS as i32 / 2).clamp(0, h - SECTORS as i32);
        let corner =
            |s: i32, n: i32| (s as f32 - n as f32 * 0.5) * SECTOR_M - MARGIN as f32 * VOXEL;
        let origin = Vec3::new(
            corner(sector_x, w),
            -(below as f32) * VOXEL,
            corner(sector_z, h),
        );
        (origin, depth, IVec2::new(sector_x, sector_z))
    }

    /// Size of the block, metres.
    pub fn extent(&self) -> Vec3 {
        Vec3::new(SIDE as f32, self.depth as f32, SIDE as f32) * VOXEL
    }

    fn add(&mut self, x: usize, y: usize, z: usize, w: f32) {
        let i = x + z * SIDE + y * SIDE * SIDE;
        self.cells[i] = (self.cells[i] as i32 + w.round_ties_even() as i32).clamp(0, 255) as u8;
    }

    /// Splats one sample point (world, metres) into its eight neighbouring
    /// cells, each share rounded on its own. Points too close to the
    /// block's sides or in its top slice are dropped.
    pub fn splat(&mut self, p: Vec3) {
        let g = (p - self.origin) / VOXEL - Vec3::splat(0.5);
        let base = g.trunc();
        let f = g - base;
        let (x, y, z) = (base.x as i64, base.y as i64, base.z as i64);
        let side = SIDE as i64;
        if x < 1 || x > side - 2 || z < 1 || z > side - 2 || y < 0 || y > self.depth as i64 - 2 {
            return;
        }
        let (x, y, z) = (x as usize, y as usize, z as usize);
        // Each layer's weight is (1 − fy)·w or fy·w, and each corner's share
        // is (1 − fx)(1 − fz) (or fx, fz) times it, as the original forms
        // the two products.
        for (dy, wy) in [(0, 1.0 - f.y), (1, f.y)] {
            let layer = wy * POINT_WEIGHT;
            for (dz, wz) in [(0, 1.0 - f.z), (1, f.z)] {
                for (dx, wx) in [(0, 1.0 - f.x), (1, f.x)] {
                    self.add(x + dx, y + dy, z + dz, (wx * wz) * layer);
                }
            }
        }
        let column = &mut self.lowest[x + z * SIDE];
        *column = (*column).min(y as u8);
    }

    /// The lowest slice light propagates in, per column: the lowest occupied
    /// slice of its 6 m sector (30 × 30 columns inside the 8-voxel margin),
    /// or 255 where the sector holds nothing (or in the margin).
    pub fn floors(&self) -> Vec<u8> {
        let mut sector = [[255u8; SECTORS]; SECTORS];
        for z in MARGIN..MARGIN + SECTORS * SECTOR {
            for x in MARGIN..MARGIN + SECTORS * SECTOR {
                let s = &mut sector[(z - MARGIN) / SECTOR][(x - MARGIN) / SECTOR];
                *s = (*s).min(self.lowest[x + z * SIDE]);
            }
        }
        let mut floors = vec![255u8; SIDE * SIDE];
        for z in MARGIN..MARGIN + SECTORS * SECTOR {
            for x in MARGIN..MARGIN + SECTORS * SECTOR {
                floors[x + z * SIDE] = sector[(z - MARGIN) / SECTOR][(x - MARGIN) / SECTOR];
            }
        }
        floors
    }

    /// The occupancy of the cell holding `p` (0..255).
    pub fn at(&self, p: Vec3) -> Option<u8> {
        let g = ((p - self.origin) / VOXEL).floor();
        if g.min_element() < 0.0 {
            return None;
        }
        let (x, y, z) = (g.x as usize, g.y as usize, g.z as usize);
        (x < SIDE && z < SIDE && y < self.depth).then(|| self.cells[x + z * SIDE + y * SIDE * SIDE])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `app` until `done` holds (the build runs on another thread).
    fn update_until(app: &mut App, done: impl Fn(&World) -> bool) {
        for _ in 0..2000 {
            if done(app.world()) {
                return;
            }
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("timed out");
    }

    /// A level grid `size` cells across, all built.
    fn open_grid(size: u32) -> GiGrid {
        GiGrid {
            width: size,
            height: size,
            edges: Arc::new(vec![OPEN; (size * size) as usize]),
        }
    }

    #[test]
    fn blocks_are_built_off_the_main_thread_around_the_camera() {
        let mut app = App::new();
        app.add_plugins((TaskPoolPlugin::default(), GiVolumePlugin))
            .init_resource::<Assets<Mesh>>();
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), GlobalTransform::default()))
            .id();
        let level = PendingGi {
            centre: Vec3::ZERO,
            top: 6.0,
            bottom: 0.0,
            base_ambient: [0.04, 0.04, 0.04],
            grid: open_grid(32),
        };
        app.insert_resource(level.clone());
        app.update();
        assert_eq!(*app.world().resource::<GiState>(), GiState::Building);
        update_until(&mut app, |w| w.contains_resource::<GiVolume>());
        assert_eq!(*app.world().resource::<GiState>(), GiState::Built);
        // Around the point ahead of the camera (it looks along −Z).
        let ahead = Vec3::NEG_Z * FOCUS_AHEAD;
        let (origin, depth, _) = Occupancy::placement(ahead, 6.0, 0.0, (32, 32));
        let volume = app.world().resource::<GiVolume>();
        assert_eq!(volume.origin, origin);
        assert_eq!(
            volume.size,
            UVec3::new(SIDE as u32, SIDE as u32, depth as u32)
        );
        // A small move keeps the block; a far one rebuilds it.
        *app.world_mut().get_mut::<GlobalTransform>(camera).unwrap() =
            GlobalTransform::from_xyz(5.0, 0.0, 0.0);
        app.update();
        assert_eq!(*app.world().resource::<GiState>(), GiState::Built);
        *app.world_mut().get_mut::<GlobalTransform>(camera).unwrap() =
            GlobalTransform::from_xyz(40.0, 0.0, 0.0);
        app.update();
        assert_eq!(*app.world().resource::<GiState>(), GiState::Building);
        update_until(&mut app, |w| *w.resource::<GiState>() == GiState::Built);
        let (moved, _, _) =
            Occupancy::placement(Vec3::new(40.0, 0.0, 0.0) + ahead, 6.0, 0.0, (32, 32));
        assert_eq!(app.world().resource::<GiVolume>().origin, moved);
        // A refocus rebuilds even for a small move.
        *app.world_mut().get_mut::<GlobalTransform>(camera).unwrap() =
            GlobalTransform::from_xyz(46.0, 0.0, 0.0);
        app.insert_resource(GiRefocus);
        app.update();
        assert_eq!(*app.world().resource::<GiState>(), GiState::Building);
        update_until(&mut app, |w| *w.resource::<GiState>() == GiState::Built);
        assert!(!app.world().contains_resource::<GiRefocus>());
    }

    #[test]
    fn placement_is_where_a_new_block_sits() {
        let grid = (16, 16);
        // x = 10 m is cell 9, so a block of eight starts at cell 5, and
        // z = −5 m is cell 7, so it starts at cell 3. Blocks sit on the
        // grid's 6 m cells, with the 8-voxel margin before the first.
        let (origin, depth, sector) =
            Occupancy::placement(Vec3::new(10.0, 0.0, -5.0), 6.0, 0.0, grid);
        assert_eq!(sector, IVec2::new(5, 3));
        assert_eq!(depth, 64);
        assert!((origin.x - ((5.0 - 8.0) * 6.0 - 1.6)).abs() < 1e-5);
        assert!((origin.z - ((3.0 - 8.0) * 6.0 - 1.6)).abs() < 1e-5);
        for (centre, top, bottom) in [
            (Vec3::splat(-33.0), 20.0, 0.0),
            (Vec3::new(0.0, 0.0, 0.0), 3.4, 0.0),
        ] {
            let (origin, depth, sector) = Occupancy::placement(centre, top, bottom, grid);
            let o = Occupancy::new(centre, top, bottom, grid);
            assert_eq!((origin, depth, sector), (o.origin, o.depth, o.sector));
        }
        // A block never starts outside the grid: near its edge it slides in.
        let (_, _, sector) = Occupancy::placement(Vec3::new(100.0, 0.0, 0.0), 6.0, 0.0, grid);
        assert_eq!(sector.x, 8);
        // A level without a grid has the original's one 8 × 8 block at the
        // world origin, and it never moves.
        let synthetic = (GiGrid::synthetic(), (8, 8));
        let (a, _, s) = Occupancy::placement(Vec3::new(0.0, 0.0, 0.0), 6.0, 0.0, synthetic.1);
        let (b, _, _) = Occupancy::placement(Vec3::new(30.0, 0.0, 0.0), 6.0, 0.0, (8, 8));
        assert_eq!(s, IVec2::ZERO);
        assert_eq!(a, b);
        assert!((a.x + 25.6).abs() < 1e-5);
        assert_eq!(synthetic.0.width, 8);
    }

    #[test]
    fn block_placement_and_lid() {
        let o = Occupancy::new(Vec3::new(10.0, 0.0, -5.0), 6.0, 0.0, (16, 16));
        assert_eq!(o.depth, 64);
        assert!((o.origin.y - (6.0 - 12.8)).abs() < 1e-4);
        assert_eq!(o.at(Vec3::new(10.0, 5.9, -5.0)), Some(26));
        assert_eq!(o.at(Vec3::new(10.0, 5.7, -5.0)), Some(5));
        assert_eq!(o.at(Vec3::new(10.0, 1.0, -5.0)), Some(0));
        let tall = Occupancy::new(Vec3::ZERO, 20.0, 0.0, (16, 16));
        assert_eq!(tall.depth, 128);
    }

    #[test]
    fn a_point_on_a_cell_centre_fills_that_cell() {
        let mut o = Occupancy::new(Vec3::ZERO, 6.0, 0.0, (8, 8));
        let centre = o.origin + Vec3::new(100.5, 10.5, 100.5) * VOXEL;
        o.splat(centre);
        assert_eq!(o.at(centre), Some(20));
        // Points between cells share their weight.
        let between = o.origin + Vec3::new(50.0, 10.5, 50.5) * VOXEL;
        o.splat(between);
        assert_eq!(o.at(between - Vec3::X * 0.05), Some(10));
        // Saturates.
        for _ in 0..20 {
            o.splat(centre);
        }
        assert_eq!(o.at(centre), Some(255));
    }

    #[test]
    fn a_splat_share_rounds_ties_to_even() {
        let mut o = Occupancy::new(Vec3::ZERO, 6.0, 0.0, (8, 8));
        // A point halfway between two cells on every axis splits its 20 into
        // eight shares of 2.5, each stored as 2.
        let p = o.origin + Vec3::new(50.0, 10.0, 50.0) * VOXEL;
        o.splat(p);
        let base = ((p - o.origin) / VOXEL - Vec3::splat(0.5))
            .trunc()
            .as_ivec3();
        for dy in 0..2 {
            for dz in 0..2 {
                for dx in 0..2 {
                    let cell = base + IVec3::new(dx, dy, dz);
                    let at = o.origin + (cell.as_vec3() + Vec3::splat(0.5)) * VOXEL;
                    assert_eq!(o.at(at), Some(2), "cell {cell}");
                }
            }
        }
    }

    #[test]
    fn a_flat_floor_fills_the_voxels_just_under_it() {
        let mut o = Occupancy::new(Vec3::ZERO, 3.4, 0.0, (16, 16));
        // Sectors sit on multiples of 6 m.
        assert!(((o.origin.x + 1.6) / 6.0).fract().abs() < 1e-4);
        // One 6 m cell, counter-clockwise seen from above.
        let (a, b, c, d) = (
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 6.0),
            Vec3::new(6.0, 0.0, 6.0),
            Vec3::new(6.0, 0.0, 0.0),
        );
        // The cell's centre is (3, 0, 3); its floor in its own space.
        let to_cell = Affine3A::from_translation(Vec3::new(-3.0, 0.0, -3.0));
        let to_world = to_cell.inverse();
        let cell = |triangles: Vec<[Vec3; 3]>| FloorCell {
            triangles: triangles
                .into_iter()
                .map(|t| t.map(|p| to_cell.transform_point3(p)))
                .collect(),
            to_world,
        };
        let floor = cell(vec![[a, b, c], [a, c, d]]);
        let points = floor.points();
        assert_eq!(points.len(), 30 * 30);
        for &p in &points {
            o.splat(p);
        }
        assert_eq!(o.at(Vec3::new(3.0, -0.1, 3.0)), Some(20));
        assert_eq!(o.at(Vec3::new(3.0, 0.1, 3.0)), Some(0));
        assert_eq!(o.at(Vec3::new(3.0, -0.3, 3.0)), Some(0));
        // A downward face gives nothing.
        assert!(cell(vec![[a, c, b], [a, d, c]]).points().is_empty());
    }

    #[test]
    fn a_cells_floor_meshes_are_merged_before_the_heightfield() {
        // Two halves of a 6 m floor, split at 10 cm past the cell centre
        // (inside a 20 cm block), as two meshes of one cell, and a third
        // mesh in the next cell along x.
        let half = |x0: f32, x1: f32| -> Vec<[f32; 3]> {
            vec![
                [x0, 0.0, -3.0],
                [x0, 0.0, 3.0],
                [x1, 0.0, 3.0],
                [x1, 0.0, -3.0],
            ]
        };
        let quad = |positions: Vec<[f32; 3]>, at: Affine3A| MeshCopy {
            positions,
            indices: vec![0, 1, 2, 0, 2, 3],
            at,
        };
        let centre = Vec3::new(9.0, 0.0, 3.0);
        let next = Vec3::new(15.0, 0.0, 3.0);
        let floor = |c: Vec3| GiFloor {
            cell: (c.xz() * 100.0).round().as_ivec2(),
            to_cell: Affine3A::IDENTITY,
        };
        let meshes = vec![
            (
                quad(half(-3.0, 0.1), Affine3A::from_translation(centre)),
                floor(centre),
            ),
            (
                quad(half(0.1, 3.0), Affine3A::from_translation(centre)),
                floor(centre),
            ),
            (
                quad(half(-3.0, 3.0), Affine3A::from_translation(next)),
                floor(next),
            ),
        ];
        let cells = floor_cells(&meshes);
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].triangles.len(), 4);
        // Merged, the halves cover every block: the full 30 × 30.
        let points = cells[0].points();
        assert_eq!(points.len(), 30 * 30);
        // In the world, about the cell centre, 10 cm under the floor.
        assert!(points.iter().all(|p| (p.x - 9.0).abs() < 3.0
            && (p.z - 3.0).abs() < 3.0
            && (p.y + 0.1).abs() < 1e-4));
        assert_eq!(cells[1].points().len(), 30 * 30);
        // Each half alone would lose the blocks across the split.
        let alone = floor_cells(&meshes[..1]);
        assert_eq!(alone[0].points().len(), 15 * 30);
    }

    #[test]
    fn edge_points_are_dropped_and_sectors_get_floors() {
        let mut o = Occupancy::new(Vec3::ZERO, 6.0, 0.0, (8, 8));
        let edge = o.origin + Vec3::new(0.5, 10.5, 100.5) * VOXEL;
        o.splat(edge);
        assert_eq!(o.at(edge), Some(0));
        let p = o.origin + Vec3::new(20.5, 12.5, 20.5) * VOXEL;
        o.splat(p);
        let floors = o.floors();
        // The point's sector (columns 8..38) floors at its slice.
        assert_eq!(floors[30 + 30 * SIDE], 12);
        assert_eq!(floors[8 + 37 * SIDE], 12);
        assert_eq!(floors[38 + 20 * SIDE], 255);
        assert_eq!(floors[2 + 20 * SIDE], 255);
    }

    #[test]
    fn a_sector_is_drawn_when_the_view_sees_its_mid_height_point() {
        let grid = open_grid(16);
        let block = IVec2::new(4, 4);
        // Sector (4, 4) of the block stands at cells (10, 10), whose centre
        // is at world (14.6, ·, 14.6) from this block's corner.
        let origin = Vec3::new(-1.6, 0.0, -1.6);
        let depth = 64.0;
        let floor = |_: i32, _: i32| 0u8;
        // The point's height is the middle of the span from slice 0 to the
        // top: (0 + 64) / 2 × 20 cm = 6.4 m.
        let at = Vec3::new(origin.x + 4.6 + 6.0 * 6.0, 6.4, origin.z + 4.6 + 6.0 * 6.0);
        let marks = sector_marks(
            &grid,
            IVec2::new(8, 8),
            block,
            origin,
            depth,
            floor,
            |centre, half| {
                (centre - at).length() < 0.01 && (half - Vec3::new(3.0, 6.4, 3.0)).length() < 1e-6
            },
        );
        let drawn: Vec<usize> = (0..64).filter(|&i| marks[i]).collect();
        // That sector and the four neighbours the original's second pass
        // adds (the diagonal corners are two sectors out).
        assert_eq!(drawn.len(), 5);
        for i in [6 * 8 + 6, 5 * 8 + 6, 7 * 8 + 6, 6 * 8 + 5, 6 * 8 + 7] {
            assert!(marks[i], "sector {i}");
        }
        // A cell outside the six-sector window is never marked, even when
        // its point is in view.
        let marks = sector_marks(
            &grid,
            IVec2::new(2, 2),
            IVec2::ZERO,
            origin,
            depth,
            floor,
            |_, _| true,
        );
        assert!(!marks[0]);
        assert!(marks[1 + 8]);
    }

    #[test]
    fn sector_marks_skip_unbuilt_cells_and_keep_to_the_reach() {
        let mut grid = open_grid(16);
        let block = IVec2::new(4, 4);
        let origin = Vec3::new(-1.6, 0.0, -1.6);
        // Focus on the block's own sector (4, 4): the reach (< 37 cells
        // squared) covers the block's nearest corner but not its far one.
        let marks = sector_marks(
            &grid,
            IVec2::new(8, 8),
            block,
            origin,
            64.0,
            |_, _| 0,
            |_, _| true,
        );
        assert!(marks[0]);
        assert!(marks[63]);
        // Close the cell (4, 4); its neighbours still mark it by dilation,
        // but closing all five leaves it dark.
        let mut edges = (*grid.edges).clone();
        for (x, z) in [(4, 4), (5, 4), (3, 4), (4, 5), (4, 3)] {
            edges[(z * 16 + x) as usize] = 0;
        }
        grid.edges = Arc::new(edges);
        let marks = sector_marks(
            &grid,
            IVec2::new(8, 8),
            block,
            origin,
            64.0,
            |_, _| 0,
            |_, _| true,
        );
        assert!(
            !marks[0],
            "the block's corner sector has no built neighbour"
        );
        assert!(marks[63]);
    }
}
