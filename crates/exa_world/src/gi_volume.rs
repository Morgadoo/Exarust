//! The occupancy grid of the original's voxel global illumination: a block
//! of 20 cm voxels, 256 × 256 across (48 m plus margins) and 64 or 128
//! high, below the level's highest point. Meshes mark their cells solid by
//! splatting their GI sample points, tile-grid floors become a solid layer
//! under their surface, and the top two slices form a soft lid.
//! Light then spreads through the open cells each frame (`exa_render::gi`).
//! Rules: `docs/RENDERING.md` (Voxel GI).
//!
//! A block is built on the async compute pool from a shared copy of the
//! level's GI inputs, and handed to the renderer when it is done: the build
//! takes seconds on a large outdoor level, and the window must keep
//! answering meanwhile.

use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use bevy::transform::TransformSystems;
use exa_render::gi::GiVolume;

pub struct GiVolumePlugin;

impl Plugin for GiVolumePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GiSettings>()
            .init_resource::<GiState>()
            .add_systems(
                PostUpdate,
                (collect_source, follow_camera)
                    .chain()
                    .after(TransformSystems::Propagate)
                    .before(exa_render::gi::GiSystems),
            );
    }
}

/// The GI sample points of a mesh node, in its space (metres).
#[derive(Component, Clone)]
pub struct GiPoints(pub Arc<Vec<Vec3>>);

/// A tile-grid floor mesh: its upward faces become a solid layer of voxels
/// just under the surface, as the original generates for each cell's floor.
#[derive(Component, Clone, Copy)]
pub struct GiFloor;

/// A terrain tile mesh: the voxels below its surface are solid.
#[derive(Component, Clone, Copy)]
pub struct GiTerrain;

/// A level waiting for its GI volume, once its nodes have world transforms.
#[derive(Resource, Debug, Clone, Copy)]
pub struct PendingGi {
    /// Centre of the level (X, Z used), metres.
    pub centre: Vec3,
    /// The level's highest and lowest points, metres.
    pub top: f32,
    pub bottom: f32,
    pub base_ambient: f32,
}

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
/// settled: every sample point and every upward floor triangle. Shared with
/// the block builds.
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
    floors: Vec<[Vec3; 3]>,
    terrain: Vec<[Vec3; 3]>,
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
    floors: Query<(&Mesh3d, &GlobalTransform), With<GiFloor>>,
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
    let floors = mesh_copies(floors.iter(), &meshes);
    let ground = mesh_copies(terrain.iter(), &meshes);
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let mut world = Vec::new();
        for (gi, m) in &points {
            world.extend(gi.iter().map(|&p| m.transform_point3(p)));
        }
        GiInputs {
            meshes: points.len(),
            points: world,
            floors: world_triangles(&floors),
            terrain: world_triangles(&ground),
        }
    });
    commands.insert_resource(GiSource {
        inputs: SourceInputs::Collecting(task),
        level: *pending,
    });
}

/// The positions and triangles of `meshes`, copied with their transforms.
fn mesh_copies<'a>(
    parts: impl Iterator<Item = (&'a Mesh3d, &'a GlobalTransform)>,
    meshes: &Assets<Mesh>,
) -> Vec<MeshCopy> {
    parts
        .filter_map(|(mesh, at)| {
            let mesh = meshes.get(&mesh.0)?;
            Some(MeshCopy {
                positions: mesh
                    .attribute(Mesh::ATTRIBUTE_POSITION)?
                    .as_float3()?
                    .to_vec(),
                indices: mesh.indices()?.iter().map(|i| i as u32).collect(),
                at: at.affine(),
            })
        })
        .collect()
}

/// The triangles of `meshes` in world space.
fn world_triangles(meshes: &[MeshCopy]) -> Vec<[Vec3; 3]> {
    let mut triangles = Vec::new();
    for mesh in meshes {
        let corners: Vec<Vec3> = mesh
            .positions
            .iter()
            .map(|&p| mesh.at.transform_point3(Vec3::from(p)))
            .collect();
        for t in mesh.indices.as_chunks::<3>().0 {
            let corner = |i: u32| corners.get(i as usize).copied();
            if let (Some(a), Some(b), Some(c)) = (corner(t[0]), corner(t[1]), corner(t[2])) {
                triangles.push([a, b, c]);
            }
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
            "GI source: {} meshes, {} points, {} floor and {} terrain triangles",
            inputs.meshes,
            inputs.points.len(),
            inputs.floors.len(),
            inputs.terrain.len()
        );
        source.inputs = SourceInputs::Ready(Arc::new(inputs));
    }
    let SourceInputs::Ready(inputs) = &source.inputs else {
        return;
    };
    let inputs = inputs.clone();
    let level = source.level;
    let mut origin = block.map(|b| b.origin);
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
            base_ambient: level.base_ambient,
            warm_up: settings.warm_up,
        });
        *state = GiState::Built;
    }
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let focus = camera.translation() + camera.forward().as_vec3() * FOCUS_AHEAD;
    let (wanted, _) = Occupancy::placement(focus, level.top, level.bottom);
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

/// Builds the block centred on `focus`: terrain first, then the meshes'
/// points, then the floors, as the original builds a block.
fn build_block(focus: Vec3, level: PendingGi, inputs: &GiInputs) -> BuiltBlock {
    let mut occupancy = Occupancy::new(focus, level.top, level.bottom);
    let mut ground = FloorSurface::with_columns(&occupancy, 1);
    for &t in &inputs.terrain {
        ground.add(t);
    }
    let terrain_columns = ground.apply_terrain(&mut occupancy);
    for &p in &inputs.points {
        occupancy.splat(p);
    }
    let mut surface = FloorSurface::new(&occupancy);
    for &t in &inputs.floors {
        surface.add(t);
    }
    let floor_points = surface.apply(&mut occupancy);
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
/// Height ranges above this (metres) get the taller volume.
const TALL_FROM: f32 = 12.8;
/// Columns of margin on each side, then sectors of 30 columns (6 m).
const MARGIN: usize = 8;
const SECTOR: usize = 30;
const SECTORS: usize = 8;
const SECTOR_M: f32 = SECTOR as f32 * VOXEL;
/// Floor columns are half a voxel wide.
const FINE: usize = 2;
/// Occupancy of a voxel below the terrain surface.
const TERRAIN_SOLID: u8 = 20;

/// The top of a surface over the block, per column (X fastest): 10 cm
/// columns for the floor layer, whole voxels for terrain.
struct FloorSurface {
    origin: Vec3,
    /// Columns per voxel along each axis.
    fine: usize,
    tops: Vec<f32>,
}

impl FloorSurface {
    fn new(o: &Occupancy) -> Self {
        Self::with_columns(o, FINE)
    }

    fn with_columns(o: &Occupancy, fine: usize) -> Self {
        Self {
            origin: o.origin,
            fine,
            tops: vec![f32::NEG_INFINITY; SIDE * fine * SIDE * fine],
        }
    }

    /// Raises the columns under an upward-facing triangle to its height.
    fn add(&mut self, t: [Vec3; 3]) {
        // Only faces turned up (counter-clockwise from above).
        let n = (t[1] - t[0]).cross(t[2] - t[0]);
        if n.y <= 1e-7 * n.length() {
            return;
        }
        let columns = SIDE * self.fine;
        let fine = VOXEL / self.fine as f32;
        let to_grid = |p: Vec3| ((p - self.origin) / fine).xz();
        let [a, b, c] = t.map(to_grid);
        let lo = a.min(b).min(c).floor().max(Vec2::ZERO);
        let hi = a.max(b).max(c).ceil().min(Vec2::splat(columns as f32));
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
                let top = &mut self.tops[x + z * columns];
                *top = top.max(h);
            }
        }
    }

    /// Fills terrain as the original does (whole-voxel columns): voxels below
    /// the surface hold 20, the surface voxel its covered fraction of 20; the
    /// margins are left alone. Returns the columns filled.
    fn apply_terrain(&self, o: &mut Occupancy) -> usize {
        debug_assert_eq!(self.fine, 1);
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
                        (frac * TERRAIN_SOLID as f32).round() as u8;
                }
                let column = &mut o.lowest[x + z * SIDE];
                *column = (*column).min(surface.min(o.depth - 1) as u8);
                filled += 1;
            }
        }
        filled
    }

    /// Fills each voxel holding the 10 cm cells under at least four of its
    /// columns' tops (all four columns in the same 20 cm band); returns how
    /// many voxels were filled.
    fn apply(&self, o: &mut Occupancy) -> usize {
        let fine = VOXEL / FINE as f32;
        let mut filled = 0;
        for z in 0..SIDE {
            for x in 0..SIDE {
                let mut counts = std::collections::HashMap::new();
                for dz in 0..FINE {
                    for dx in 0..FINE {
                        let top = self.tops[x * FINE + dx + (z * FINE + dz) * SIDE * FINE];
                        if top.is_finite() {
                            // The 10 cm cell whose top is the first boundary
                            // at or above the surface.
                            let cell = ((top - self.origin.y) / fine).ceil() as i64 - 1;
                            *counts.entry(cell.div_euclid(FINE as i64)).or_insert(0) += 1;
                        }
                    }
                }
                for (y, n) in counts {
                    if n >= 4 {
                        let centre = self.origin
                            + Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5) * VOXEL;
                        o.splat(centre);
                        filled += 1;
                    }
                }
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
}

impl Occupancy {
    /// An empty block centred on `centre` (X, Z) whose top is the level's
    /// highest point `top`; `bottom` (the lowest point) picks its height.
    pub fn new(centre: Vec3, top: f32, bottom: f32) -> Self {
        let (origin, depth) = Self::placement(centre, top, bottom);
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
        }
    }

    /// Where a block centred on `centre` sits and how many voxels high it
    /// is ([`Occupancy::new`]).
    pub fn placement(centre: Vec3, top: f32, bottom: f32) -> (Vec3, usize) {
        let depth = if top - bottom > TALL_FROM { 128 } else { 64 };
        // Sectors fall on world multiples of 6 m, the block four sectors
        // either side of the one nearest the centre, plus the margin.
        let corner = |c: f32| {
            ((c / SECTOR_M).round() - SECTORS as f32 / 2.0) * SECTOR_M - MARGIN as f32 * VOXEL
        };
        // The top sits a whole number of voxels above the origin.
        let below = ((depth as f32 * VOXEL - top) / VOXEL).trunc();
        let origin = Vec3::new(corner(centre.x), -below * VOXEL, corner(centre.z));
        (origin, depth)
    }

    /// Size of the block, metres.
    pub fn extent(&self) -> Vec3 {
        Vec3::new(SIDE as f32, self.depth as f32, SIDE as f32) * VOXEL
    }

    fn add(&mut self, x: usize, y: usize, z: usize, w: f32) {
        let i = x + z * SIDE + y * SIDE * SIDE;
        self.cells[i] = (self.cells[i] as i32 + w.round() as i32).clamp(0, 255) as u8;
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
        for (dy, wy) in [(0, 1.0 - f.y), (1, f.y)] {
            for (dz, wz) in [(0, 1.0 - f.z), (1, f.z)] {
                for (dx, wx) in [(0, 1.0 - f.x), (1, f.x)] {
                    self.add(x + dx, y + dy, z + dz, POINT_WEIGHT * wx * wy * wz);
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
            base_ambient: 0.01,
        };
        app.insert_resource(level);
        app.update();
        assert_eq!(*app.world().resource::<GiState>(), GiState::Building);
        update_until(&mut app, |w| w.contains_resource::<GiVolume>());
        assert_eq!(*app.world().resource::<GiState>(), GiState::Built);
        // Around the point ahead of the camera (it looks along −Z).
        let ahead = Vec3::NEG_Z * FOCUS_AHEAD;
        let (origin, depth) = Occupancy::placement(ahead, 6.0, 0.0);
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
        let (moved, _) = Occupancy::placement(Vec3::new(40.0, 0.0, 0.0) + ahead, 6.0, 0.0);
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
        for (centre, top) in [
            (Vec3::new(10.0, 0.0, -5.0), 6.0),
            (Vec3::splat(-33.0), 20.0),
        ] {
            let o = Occupancy::new(centre, top, 0.0);
            assert_eq!(Occupancy::placement(centre, top, 0.0), (o.origin, o.depth));
        }
    }

    #[test]
    fn block_placement_and_lid() {
        let o = Occupancy::new(Vec3::new(10.0, 0.0, -5.0), 6.0, 0.0);
        assert_eq!(o.depth, 64);
        assert!((o.origin.y - (6.0 - 12.8)).abs() < 1e-4);
        // Nearest sector boundary to x = 10 is 12; four sectors to its left.
        assert!((o.origin.x - (12.0 - 24.0 - 1.6)).abs() < 1e-4);
        assert_eq!(o.at(Vec3::new(10.0, 5.9, -5.0)), Some(26));
        assert_eq!(o.at(Vec3::new(10.0, 5.7, -5.0)), Some(5));
        assert_eq!(o.at(Vec3::new(10.0, 1.0, -5.0)), Some(0));
        let tall = Occupancy::new(Vec3::ZERO, 20.0, 0.0);
        assert_eq!(tall.depth, 128);
    }

    #[test]
    fn a_point_on_a_cell_centre_fills_that_cell() {
        let mut o = Occupancy::new(Vec3::ZERO, 6.0, 0.0);
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
    fn a_flat_floor_fills_the_voxels_just_under_it() {
        let mut o = Occupancy::new(Vec3::ZERO, 3.4, 0.0);
        // Sectors sit on multiples of 6 m.
        assert!(((o.origin.x + 1.6) / 6.0).fract().abs() < 1e-4);
        let mut floor = FloorSurface::new(&o);
        // One 6 m cell, counter-clockwise seen from above.
        let (a, b, c, d) = (
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 6.0),
            Vec3::new(6.0, 0.0, 6.0),
            Vec3::new(6.0, 0.0, 0.0),
        );
        floor.add([a, b, c]);
        floor.add([a, c, d]);
        // Turned down: ignored.
        floor.add([a + Vec3::Y, c + Vec3::Y, b + Vec3::Y]);
        assert_eq!(floor.apply(&mut o), 30 * 30);
        assert_eq!(o.at(Vec3::new(3.0, -0.1, 3.0)), Some(20));
        assert_eq!(o.at(Vec3::new(3.0, 0.1, 3.0)), Some(0));
        assert_eq!(o.at(Vec3::new(3.0, -0.3, 3.0)), Some(0));
    }

    #[test]
    fn edge_points_are_dropped_and_sectors_get_floors() {
        let mut o = Occupancy::new(Vec3::ZERO, 6.0, 0.0);
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
}
