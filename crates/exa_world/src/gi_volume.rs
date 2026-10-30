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

use std::collections::{BTreeMap, HashMap};

use bevy::camera::primitives::Frustum;
use bevy::camera::visibility::VisibilitySystems;
use bevy::math::Affine3A;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use bevy::transform::TransformSystems;
use exa_physics::voxel::{self, Bounds};
use exa_render::gi::{
    GiNeighbourSectorLights, GiNeighbourVolumes, GiPick, GiSector, GiSectorLights, GiSectors,
    GiVolume,
};

use crate::gi_residency::GiResidency;
use crate::shadow_links::LevelWalls;

pub struct GiVolumePlugin;

impl Plugin for GiVolumePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GiSettings>()
            .init_resource::<GiState>()
            .init_resource::<GiSectorLights>()
            .init_resource::<GiNeighbourSectorLights>()
            .init_resource::<GiBuiltBlocks>()
            .init_resource::<exa_render::gi::GiOccupancyPatches>()
            .add_systems(
                PostUpdate,
                (
                    feed_shading_quality,
                    collect_source,
                    follow_camera,
                    publish_neighbour_volumes,
                    update_dynamic,
                    publish_sector_lights,
                )
                    .chain()
                    .after(TransformSystems::Propagate)
                    .after(VisibilitySystems::UpdateFrusta)
                    .after(crate::shadow_links::LightCells)
                    .before(exa_render::gi::GiSystems),
            );
    }
}

/// The GI meshes the level's source collects, with their thin and centred
/// flags.
type GiSourceMeshes<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GiPoints,
        &'static GlobalTransform,
        Has<GiThinMesh>,
        Has<GiCentredMesh>,
    ),
>;

/// The GI meshes the dynamic update follows: those the level's source
/// collected, each with the frames [`GiMoved`] keeps.
type GiMoverQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static GiPoints,
        &'static GlobalTransform,
        &'static mut GiMoved,
        Has<GiThinMesh>,
        Has<GiCentredMesh>,
    ),
>;

/// Takes a moving GI mesh's old points out of every built block it meets and
/// puts its new ones in, as the original's dynamic update does: nothing runs
/// while a block build is in flight (the original returns while its build
/// count is above zero, keeping the old frame), the box is the old and new
/// frames' padded extent, only the built blocks it meets are touched, the
/// weight is ∓20 (a quarter for a thin mesh), the old points come out before
/// the new go in, and the changed region goes to the renderer as a
/// sub-upload. A block built after a mesh moved holds the mesh where the
/// level's source collected it, so its first pass here moves the mesh from
/// that frame to its last one.
fn update_dynamic(
    mut movers: GiMoverQuery,
    build: Option<Res<GiBuild>>,
    mut built: ResMut<GiBuiltBlocks>,
    mut patches: Option<ResMut<exa_render::gi::GiOccupancyPatches>>,
) {
    // Each frame's patches are applied once by the renderer.
    if let Some(patches) = patches.as_mut()
        && !patches.0.is_empty()
    {
        patches.0.clear();
    }
    if build.is_some() {
        return;
    }
    let mut sink = patches.as_deref_mut().map(|p| &mut p.0);
    // Blocks built since the last pass: bring every mesh that has moved
    // since the source was collected to where it last was.
    for block in built.0.values_mut() {
        if block.synced {
            continue;
        }
        Arc::make_mut(block).synced = true;
        for (points, _, moved, thin, centred) in &movers {
            if moved.source != moved.last {
                move_points(
                    block,
                    &points.0,
                    moved.source,
                    moved.last,
                    thin,
                    centred,
                    &mut sink,
                );
            }
        }
    }
    for (points, at, mut moved, thin, centred) in &mut movers {
        let now = at.affine();
        if moved.last == now {
            continue;
        }
        for block in built.0.values_mut() {
            move_points(block, &points.0, moved.last, now, thin, centred, &mut sink);
        }
        moved.last = now;
    }
}

/// Moves one mesh's points from frame `old` to frame `new` in one built
/// block when the two frames' padded boxes meet it, and queues the changed
/// region's bytes.
fn move_points(
    block: &mut Arc<BuiltBlock>,
    points: &[Vec3],
    old: Affine3A,
    new: Affine3A,
    thin: bool,
    centred: bool,
    sink: &mut Option<&mut Vec<exa_render::gi::GiOccupancyPatch>>,
) {
    let (Some(before), Some(after)) = (point_bounds(points, old), point_bounds(points, new)) else {
        return;
    };
    // The union of the two frames' boxes, which a touched block must meet.
    let bounds = (before.0.min(after.0), before.1.max(after.1));
    if !boxes_meet(bounds, block.bounds()) {
        return;
    }
    let old_points: Vec<Vec3> = points.iter().map(|&p| old.transform_point3(p)).collect();
    let new_points: Vec<Vec3> = points.iter().map(|&p| new.transform_point3(p)).collect();
    let block = Arc::make_mut(block);
    let dims = UVec3::new(SIDE as u32, SIDE as u32, block.depth as u32);
    let sector = block.sector;
    // The cells are edited in place unless a volume still shares them.
    let cells = Arc::make_mut(&mut block.cells);
    let mut occupancy = Occupancy {
        origin: block.origin,
        depth: block.depth,
        cells: std::mem::take(cells),
        // The dynamic update never writes the per-column floors; the
        // block's own stay as built.
        lowest: vec![255u8; SIDE * SIDE],
        sector,
    };
    let region = occupancy.update_dynamic(&old_points, &new_points, thin, centred);
    *cells = occupancy.cells;
    let Some(region) = region else { return };
    if let (Some(sink), Some(bytes)) = (sink.as_mut(), sub_upload(cells, dims, region)) {
        sink.push(exa_render::gi::GiOccupancyPatch {
            sector,
            lo: region.lo,
            hi: region.hi,
            bytes,
        });
    }
}

/// Takes the settings screen's shading quality into the GI: the original's
/// per-frame GI entry is told whether the scene's shading quality is above
/// three, and halves the base ambient and drops the manager's light share to
/// 0.9 when it is.
fn feed_shading_quality(
    quality: Option<Res<exa_render::quality::ShadingQuality>>,
    mut settings: ResMut<GiSettings>,
) {
    let Some(quality) = quality else { return };
    let high = quality.high();
    if settings.high_quality != high {
        settings.high_quality = high;
    }
}

/// The GI sample points of a mesh node, in its space (metres).
#[derive(Component, Clone)]
pub struct GiPoints(pub Arc<Vec<Vec3>>);

/// A GI mesh the original counts as thin: its sample points weigh a quarter
/// (the node's first surface's material is a class-`0x4000` or opacity-cut
/// material, `Material::is_thin_for_gi`).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GiThinMesh;

/// A GI mesh with the original's centred vertical variant: its points gain
/// half a voxel in height, and they leave the column's lowest slice alone.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GiCentredMesh;

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
    /// The environment record's flag `0x20` (the outdoor terrain arenas):
    /// the light steps reach the top slice instead of clearing it.
    pub top_slice: bool,
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
    /// A level without its own grid: the original's synthetic 8 × 8 grid.
    pub synthetic: bool,
}

impl GiGrid {
    /// The synthetic grid of a level without one.
    pub fn synthetic() -> Self {
        Self {
            width: 8,
            height: 8,
            edges: Arc::new(vec![OPEN; 8 * 8]),
            synthetic: true,
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

/// The manager's blocks: up to eight 6 m sectors on a side, over the level
/// grid's built cells. The manager makes the list once per level and never
/// rebuilds it; the camera only picks which block it draws.
#[derive(Clone, Debug, Default)]
pub struct GiBlockLayout {
    /// The blocks with built cells, in creation order (X fastest).
    pub blocks: Vec<GiBlockSpec>,
    /// Blocks across and down.
    pub columns: i32,
    pub rows: i32,
    /// The level grid's size (X, Z), cells.
    pub grid: (u32, u32),
    /// The first block's world edge, cm.
    pub cells_origin_cm: Vec2,
}

/// One of the manager's blocks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GiBlockSpec {
    /// Its first 6 m cell in the level grid.
    pub sector: IVec2,
    /// Its block-grid slot, X fastest.
    pub slot: usize,
    /// The lowest corner of its voxels, metres.
    pub origin: Vec3,
    /// Its centre (X and Z used), metres.
    pub centre: Vec3,
    /// Voxels along Y.
    pub depth: usize,
}

impl GiBlockLayout {
    /// The blocks over `grid`'s built cells, as the original lays them out:
    /// from the bounding box of the built cells, rounded up to whole blocks
    /// of eight cells and slid back inside the grid. A level without its
    /// own grid gets the original's single block at cells (3, 3).
    pub fn new(grid: &GiGrid, top: f32, bottom: f32) -> Self {
        let (depth, below) = level_height(top, bottom, (VOXEL * CM_PER_M) as i32);
        let (w, h) = (grid.width as i32, grid.height as i32);
        let (columns, rows, first) = if grid.synthetic {
            (1, 1, IVec2::new(3, 3))
        } else {
            let (x0, z0, x1, z1) = built_bounds(grid);
            let mut columns = ((x1 - x0 + 1) + 7) / 8;
            let mut rows = ((z1 - z0 + 1) + 7) / 8;
            let mut first = IVec2::new(x0, z0);
            if w < first.x + columns * 8 {
                first.x -= first.x + columns * 8 - w;
            }
            if h < first.y + rows * 8 {
                first.y -= first.y + rows * 8 - h;
            }
            columns = columns.max(1);
            rows = rows.max(1);
            (columns, rows, first)
        };
        let mut blocks = Vec::new();
        for z in 0..rows {
            for x in 0..columns {
                let sector = first + IVec2::new(x * 8, z * 8);
                if !block_has_cells(grid, sector) {
                    continue;
                }
                let slot = (x + z * columns) as usize;
                let centre = block_centre(sector, grid.width, grid.height) / CM_PER_M;
                blocks.push(GiBlockSpec {
                    sector,
                    slot,
                    origin: block_origin(sector, grid.width, grid.height, VOXEL, below),
                    centre: Vec3::new(centre.x, 0.0, centre.y),
                    depth,
                });
            }
        }
        Self {
            blocks,
            columns,
            rows,
            grid: (grid.width, grid.height),
            // The grid's half size in float here (unlike the blocks' own
            // integer halves), so an odd grid's edge sits half a cell off.
            cells_origin_cm: Vec2::new(
                (first.x as f32 - w as f32 * 0.5) * 600.0,
                (first.y as f32 - h as f32 * 0.5) * 600.0,
            ),
        }
    }

    /// The block a world point (metres) falls in, as the original's
    /// per-object lookup floors the offset times 1/4800 cm into the block
    /// grid. `None` when that block has no geometry.
    pub fn block_at(&self, at: Vec3) -> Option<usize> {
        let index = GiPick::cell_of(self.cells_origin_cm, at);
        if index.x < 0 || index.y < 0 || index.x >= self.columns || index.y >= self.rows {
            return None;
        }
        let slot = (index.x + index.y * self.columns) as usize;
        self.blocks.iter().position(|b| b.slot == slot)
    }

    /// Where block `index` sits for the per-object pick: the grid's low
    /// edge and the block's cell in the block grid.
    pub fn pick(&self, index: usize) -> Option<GiPick> {
        let block = self.blocks.get(index)?;
        let columns = self.columns.max(1) as usize;
        Some(GiPick {
            cells_origin_cm: self.cells_origin_cm,
            cell: IVec2::new((block.slot % columns) as i32, (block.slot / columns) as i32),
        })
    }

    /// A block's centre in the ground plane (X, Z), world centimetres:
    /// `((cell − grid/2) + 4) · 600` per axis.
    pub fn centre_cm(&self, block: &GiBlockSpec) -> Vec2 {
        block_centre(block.sector, self.grid.0, self.grid.1)
    }

    /// The nearest block to a world point (metres), ties in creation order.
    /// The distance ignores height and measures from the block's 24 m half
    /// extent, as the original's sort does.
    pub fn nearest(&self, at: Vec3) -> Option<usize> {
        let p = at.xz() * CM_PER_M;
        self.blocks
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let centre = block_centre(b.sector, self.grid.0, self.grid.1);
                let dx = (p.x - centre.x).abs() - 2400.0;
                let dz = (p.y - centre.y).abs() - 2400.0;
                (i, dx.max(0.0).powi(2) + dz.max(0.0).powi(2))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            .map(|(i, _)| i)
    }
}

/// A block's per-object GI sample transform: the original's `VXGIPos` and
/// `VXGIScl` uniforms, so a surface at a world point `p` reads the light
/// volume at `texture(p · scale + offset)` with `p` in world **centimetres**
/// (the original's vertex stage). The texture axes are X, Z depth, Y height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GiSample {
    /// `VXGIPos`: X and Z are half a texture minus the block's centre over
    /// the block width; Y is the block's bottom slice over its depth, plus
    /// half a slice unless the draw drops the half-voxel offset.
    pub offset: Vec3,
    /// `VXGIScl`: one over the block width for X and Z, one over its
    /// height (slice count × voxel) for Y.
    pub scale: Vec3,
}

impl GiBlockLayout {
    /// A block's per-object GI sample transform, as the original builds it
    /// for an object inside the block: the offset's X is
    /// `((grid width/2 − block cell) − 4) · 30/256 + 0.5` (the block's
    /// centre in texture units), its Y `(below + 0.5)/depth`, or
    /// `below/depth` when the draw drops the half-voxel offset (the
    /// original's per-object flag; its terrain and grass paths set it), and
    /// its Z the same as X from the grid's height. The scale is
    /// `(1/(W·fc), 1/(D·fc), 1/(H·fc))`.
    pub fn sample_transform(&self, block: &GiBlockSpec, half_voxel: bool) -> GiSample {
        let (w, h) = (self.grid.0 as f32, self.grid.1 as f32);
        let below = -(block.origin.y / VOXEL).round();
        let depth = block.depth.max(1) as f32;
        // The cell difference is taken in single precision, then scaled by
        // 30/256 and lifted by half a texture in double and rounded once,
        // as the original does.
        let centre = |half: f32, cell: i32| -> f32 {
            (f64::from((half - cell as f32) - 4.0) * (30.0 / 256.0) + 0.5) as f32
        };
        GiSample {
            offset: Vec3::new(
                centre(w * 0.5, block.sector.x),
                (below + if half_voxel { 0.5 } else { 0.0 }) / depth,
                centre(h * 0.5, block.sector.y),
            ),
            scale: Vec3::new(1.0 / 5120.0, 1.0 / (depth * 20.0), 1.0 / 5120.0),
        }
    }

    /// The texture position a surface at `at` (metres) in `block` samples,
    /// as the original's `p · VXGIScl + VXGIPos` with `p` in centimetres.
    /// Texture axes are X, Z depth, Y height.
    pub fn sample_at(&self, block: &GiBlockSpec, at: Vec3, half_voxel: bool) -> Vec3 {
        let sample = self.sample_transform(block, half_voxel);
        (at * CM_PER_M) * sample.scale + sample.offset
    }
}

/// The bounding box of the grid's built cells, as the original finds it:
/// the border cells are skipped, and the first row and column come from the
/// first built cell. Each later row moves the left edge to its first built
/// cell left of it, or else checks the current span (both ends included);
/// either reaches the bottom edge to that row, as does any built cell right
/// of the span, which also moves the right edge. An empty grid gives the
/// original's (-4, -4) to (3, 3).
#[allow(clippy::mut_range_bound)]
pub fn built_bounds(grid: &GiGrid) -> (i32, i32, i32, i32) {
    let (w, h) = (grid.width as i32, grid.height as i32);
    let mut found = None;
    'scan: for z in 1..=h - 2 {
        for x in 1..=w - 2 {
            if grid.open(i64::from(x), i64::from(z)) {
                found = Some((x, z));
                break 'scan;
            }
        }
    }
    let Some((mut x0, z0)) = found else {
        return (-4, -4, 3, 3);
    };
    let mut x1 = x0;
    let mut z1 = z0;
    for z in z0..=h - 2 {
        let mut extended = false;
        for x in 1..x0 {
            if grid.open(i64::from(x), i64::from(z)) {
                x0 = x;
                z1 = z;
                extended = true;
                break;
            }
        }
        if !extended && (x0..=x1).any(|x| grid.open(i64::from(x), i64::from(z))) {
            z1 = z;
        }
        for x in x1 + 1..=w - 2 {
            if grid.open(i64::from(x), i64::from(z)) {
                x1 = x;
                z1 = z;
            }
        }
    }
    (x0, z0, x1, z1)
}

/// The block height and vertical offset the original's level record picks:
/// 64 slices, or 128 when the level spans more than 12.8 m.
fn level_height(top: f32, bottom: f32, voxel_cm: i32) -> (usize, i32) {
    let top_cm = (top * CM_PER_M).round() as i32;
    let bottom_cm = (bottom * CM_PER_M).round() as i32;
    let depth = if (top_cm - bottom_cm) / voxel_cm > 64 {
        128
    } else {
        64
    };
    (depth, (depth as i32 * voxel_cm - top_cm) / voxel_cm)
}

/// Whether any cell of an eight-by-eight block is built.
fn block_has_cells(grid: &GiGrid, sector: IVec2) -> bool {
    (0..8).any(|z| (0..8).any(|x| grid.open(i64::from(sector.x + x), i64::from(sector.y + z))))
}

/// The lowest corner of a block starting at `sector`, metres: the original
/// counts whole 6 m cells from the grid's middle cell (its size halved,
/// rounded down), eight voxels of margin, and the level's vertical offset.
fn block_origin(sector: IVec2, width: u32, height: u32, voxel: f32, below: i32) -> Vec3 {
    let cells_per_world = (600.0 / (voxel * CM_PER_M)) as i32;
    Vec3::new(
        ((sector.x - (width as i32) / 2) * cells_per_world - MARGIN as i32) as f32 * voxel,
        -(below as f32) * voxel,
        ((sector.y - (height as i32) / 2) * cells_per_world - MARGIN as i32) as f32 * voxel,
    )
}

/// A block's centre (X, Z) in world centimetres.
fn block_centre(sector: IVec2, width: u32, height: u32) -> Vec2 {
    Vec2::new(
        (sector.x - (width as i32) / 2 + 4) as f32 * 600.0,
        (sector.y - (height as i32) / 2 + 4) as f32 * 600.0,
    )
}

/// Whether levels get the voxel GI, and how many propagation steps run at
/// once when one is built (light otherwise builds up one step per frame).
#[derive(Resource, Debug, Clone, Copy)]
pub struct GiSettings {
    pub enabled: bool,
    pub warm_up: u32,
    /// The original's high shading quality (4 or above, its default): the
    /// base ambient is the level record's `0x30` × 0.001, else × 0.002, and
    /// the manager's light share is 0.09 rather than 0.1. Set from
    /// [`exa_render::quality::ShadingQuality`] each frame.
    pub high_quality: bool,
}

impl Default for GiSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            warm_up: 600,
            high_quality: true,
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
/// ambient at shading quality 4 or above, the retail default.
pub const BASE_AMBIENT: f32 = 0.001;
/// The scale below that quality: the original's manager doubles it.
pub const BASE_AMBIENT_LOW_QUALITY: f32 = 0.002;

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
    /// Every mesh's world points, with its thin and centred flags.
    points: Vec<(Vec3, bool, bool)>,
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

/// The block now active: its index in the layout ([`usize::MAX`] for the
/// fallback block of an empty grid).
#[derive(Resource)]
struct GiBlock {
    index: usize,
}

/// The manager's static block layout for the level.
#[derive(Resource, Clone, Default)]
pub struct GiBlocks(pub Arc<GiBlockLayout>);

/// The block being built off the main thread; at most one at a time. A
/// background build (a block the residency asks for, not the camera's)
/// gives way to a camera block that needs building.
#[derive(Resource)]
struct GiBuild {
    task: Task<Arc<BuiltBlock>>,
    /// The layout index of the block, or `usize::MAX` for the fallback
    /// block of an empty grid.
    block: usize,
    background: bool,
}

/// Finished blocks by layout index. Like the original manager, which keeps
/// every block's occupancy once it is built, nothing is evicted until the
/// level ends.
#[derive(Resource, Default)]
struct GiBuiltBlocks(HashMap<usize, Arc<BuiltBlock>>);

/// A finished block and what its build counted.
#[derive(Clone)]
struct BuiltBlock {
    origin: Vec3,
    depth: usize,
    sector: IVec2,
    cells: Arc<Vec<u8>>,
    floors: Arc<Vec<u8>>,
    terrain_columns: usize,
    floor_points: usize,
    /// Whether the dynamic update has brought the meshes that moved since
    /// the source was collected to their last frames in this block.
    synced: bool,
}

impl BuiltBlock {
    /// The world box the block's voxels cover.
    fn bounds(&self) -> (Vec3, Vec3) {
        let lo = self.origin;
        let hi = self.origin + Vec3::new(SIDE as f32, self.depth as f32, SIDE as f32) * VOXEL;
        (lo, hi)
    }
}

/// Whether two world boxes (lowest and highest corner) meet.
fn boxes_meet(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> bool {
    a.0.x <= b.1.x
        && b.0.x <= a.1.x
        && a.0.y <= b.1.y
        && b.0.y <= a.1.y
        && a.0.z <= b.1.z
        && b.0.z <= a.1.z
}

/// A GI mesh's frames for the dynamic update, set when the level's source
/// collects it: `source` is where the block builds splat its points and
/// `last` where the update last put them, as the original's node keeps its
/// old and new frames. The original only updates nodes whose kind is a GI
/// mesh and that hold points; a mesh spawned after the collection is not
/// followed.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct GiMoved {
    pub source: Affine3A,
    pub last: Affine3A,
}

/// The extent of a point set transformed by `at`, padded 1.5 voxels each
/// side in world space (the original's dirty box).
fn point_bounds(points: &[Vec3], at: Affine3A) -> Option<(Vec3, Vec3)> {
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for &p in points {
        let p = at.transform_point3(p);
        lo = lo.min(p);
        hi = hi.max(p);
    }
    (lo.is_finite() && hi.is_finite())
        .then_some((lo - Vec3::splat(1.5 * VOXEL), hi + Vec3::splat(1.5 * VOXEL)))
}

/// How far ahead of the camera the manager's focus sits (metres): the sector
/// marks and the block distance sort measure from it.
const FOCUS_AHEAD: f32 = 20.0;

#[allow(clippy::too_many_arguments)]
fn collect_source(
    mut commands: Commands,
    pending: Option<Res<PendingGi>>,
    settings: Res<GiSettings>,
    points: GiSourceMeshes,
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
    commands.remove_resource::<GiResidency>();
    commands.insert_resource(GiBuiltBlocks::default());
    // A build for the previous level is dropped, which cancels it.
    commands.remove_resource::<GiBuild>();
    commands.remove_resource::<exa_render::gi::GiTextures>();
    commands.remove_resource::<exa_render::gi::GiPendingTextures>();
    commands.remove_resource::<GiNeighbourVolumes>();
    // The last level's light textures must not stand in for this one's
    // blocks at the same cells.
    commands.insert_resource(exa_render::gi::GiTextureCache::default());
    if !settings.enabled {
        commands.remove_resource::<GiVolume>();
        commands.remove_resource::<GiSource>();
        commands.remove_resource::<GiBlocks>();
        *state = GiState::Off;
        return;
    }
    build_level(
        &mut commands,
        &pending,
        points,
        floors,
        terrain,
        meshes,
        &mut state,
    );
}

/// Drops everything the last level left in the GI manager: its pending and
/// running build, its block layout and volumes, and the render textures the
/// block was drawn into. The next level's `PendingGi` starts from a clean
/// [`GiState::Off`], so a level change does not keep the old level's blocks
/// or GPU memory alive.
pub fn unload_level(commands: &mut Commands, state: &mut GiState) {
    commands.remove_resource::<PendingGi>();
    commands.remove_resource::<GiBlock>();
    commands.remove_resource::<GiBuild>();
    commands.remove_resource::<GiRefocus>();
    commands.remove_resource::<GiVolume>();
    commands.remove_resource::<GiSource>();
    commands.remove_resource::<GiBlocks>();
    commands.remove_resource::<GiResidency>();
    commands.remove_resource::<exa_render::gi::GiTextures>();
    commands.remove_resource::<exa_render::gi::GiPendingTextures>();
    commands.remove_resource::<GiNeighbourVolumes>();
    commands.insert_resource(exa_render::gi::GiTextureCache::default());
    commands.insert_resource(GiBuiltBlocks::default());
    *state = GiState::Off;
}

#[allow(clippy::too_many_arguments)]
fn build_level(
    commands: &mut Commands,
    pending: &PendingGi,
    points: GiSourceMeshes,
    floors: Query<(&Mesh3d, &GlobalTransform, &GiFloor)>,
    terrain: Query<(&Mesh3d, &GlobalTransform), With<GiTerrain>>,
    meshes: Res<Assets<Mesh>>,
    state: &mut GiState,
) {
    *state = GiState::Building;
    // The manager lays its blocks out once, from the level's grid and
    // height; the camera only picks which one it draws.
    let layout = GiBlockLayout::new(&pending.grid, pending.top, pending.bottom);
    info!(
        "GI blocks: {} over {} x {} cells, first at {} cm",
        layout.blocks.len(),
        layout.columns,
        layout.rows,
        layout.cells_origin_cm
    );
    commands.insert_resource(GiResidency::new(&layout));
    commands.insert_resource(GiBlocks(Arc::new(layout)));
    // Only shared or copied here; the world-space work runs elsewhere.
    // Each mesh's frame now is where the block builds splat it; the dynamic
    // update follows it from there.
    let points: Vec<_> = points
        .iter()
        .map(|(entity, gi, at, thin, centred)| {
            let at = at.affine();
            commands.entity(entity).insert(GiMoved {
                source: at,
                last: at,
            });
            (gi.0.clone(), at, thin, centred)
        })
        .collect();
    let floors: Vec<(MeshCopy, GiFloor)> = floors
        .iter()
        .filter_map(|(mesh, at, floor)| Some((mesh_copy(mesh, at, &meshes)?, *floor)))
        .collect();
    let ground = mesh_copies(terrain.iter(), &meshes);
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let mut world = Vec::new();
        for (gi, m, thin, centred) in &points {
            world.extend(gi.iter().map(|&p| (m.transform_point3(p), *thin, *centred)));
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
/// the build is done. Meanwhile it ranks the level's blocks as the
/// original's residency does (`GiResidency`) and, while the camera's block
/// stands, builds the block the residency names next in the background, so
/// a camera that walks into a neighbouring block finds it built.
#[allow(clippy::too_many_arguments)]
fn follow_camera(
    mut commands: Commands,
    source: Option<ResMut<GiSource>>,
    block: Option<Res<GiBlock>>,
    blocks: Option<Res<GiBlocks>>,
    mut residency: Option<ResMut<GiResidency>>,
    mut built_blocks: ResMut<GiBuiltBlocks>,
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
    let cache = &mut *built_blocks;
    let mut active = block.as_ref().map(|b| b.index);
    let mut building = None;
    if let Some(mut build) = build {
        match block_on(poll_once(&mut build.task)) {
            Some(built) => {
                commands.remove_resource::<GiBuild>();
                info!(
                    "GI block{}: {} terrain columns, {} floor voxels, origin {:.2}, {} slices",
                    if build.background {
                        " (background)"
                    } else {
                        ""
                    },
                    built.terrain_columns,
                    built.floor_points,
                    built.origin,
                    built.depth
                );
                if build.block != usize::MAX {
                    cache.0.insert(build.block, built.clone());
                    if let Some(residency) = residency.as_deref_mut() {
                        residency.mark_built(build.block);
                    }
                }
                if !build.background {
                    commands.insert_resource(GiBlock { index: build.block });
                    let pick = blocks.as_ref().and_then(|b| b.0.pick(build.block));
                    commands.insert_resource(volume_of(&built, &level, &settings, pick));
                    active = Some(build.block);
                    *state = GiState::Built;
                }
            }
            None => building = Some((build.block, build.background)),
        }
    }
    let Some(blocks) = blocks else { return };
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let at = camera.translation();
    let focus = at + camera.forward().as_vec3() * FOCUS_AHEAD;
    if let Some(residency) = residency.as_deref_mut() {
        residency.rank(focus);
        // The texture handoffs only matter to a renderer holding several
        // blocks; Exarust's draws the camera's block alone.
        residency.hand_out_textures();
    }
    // The camera's own block when it has geometry; otherwise the block the
    // original's residency sort ranks first, nearest the point 20 m ahead.
    let wanted = blocks.0.block_at(at).or_else(|| blocks.0.nearest(focus));
    let refocusing = refocus.is_some();
    if refocusing {
        commands.remove_resource::<GiRefocus>();
    }
    let settled = !refocusing && wanted.is_some() && wanted == active;
    if settled || (active == Some(usize::MAX) && wanted.is_none() && !refocusing) {
        // The camera's block stands: build what the residency asks for
        // next, one block at a time.
        if building.is_none()
            && let Some(residency) = residency.as_deref_mut()
            && let Some(next) = residency.take_next_build()
            && let Some(&spec) = blocks.0.blocks.get(next)
        {
            let task = AsyncComputeTaskPool::get()
                .spawn(async move { build_block(Some(spec), at, level, &inputs) });
            commands.insert_resource(GiBuild {
                task,
                block: next,
                background: true,
            });
        }
        return;
    }
    // A newer focus waits for the camera block in flight; a background
    // build gives way to it (dropping the task cancels it, and the
    // residency asks for that block again later).
    if let Some((_, background)) = building {
        if !background {
            return;
        }
        commands.remove_resource::<GiBuild>();
    }
    if let Some(index) = wanted {
        if refocusing {
            cache.0.remove(&index);
        }
        let cached = cache.0.get(&index).cloned();
        if let Some(built) = cached {
            commands.insert_resource(GiBlock { index });
            commands.insert_resource(volume_of(&built, &level, &settings, blocks.0.pick(index)));
            *state = GiState::Built;
            return;
        }
        let spec = blocks.0.blocks[index];
        let task = AsyncComputeTaskPool::get()
            .spawn(async move { build_block(Some(spec), at, level, &inputs) });
        commands.insert_resource(GiBuild {
            task,
            block: index,
            background: false,
        });
        *state = GiState::Building;
        return;
    }
    // A grid with no built cells: the old focus-centred block stands in.
    let task =
        AsyncComputeTaskPool::get().spawn(async move { build_block(None, focus, level, &inputs) });
    commands.insert_resource(GiBuild {
        task,
        block: usize::MAX,
        background: false,
    });
    *state = GiState::Building;
}

/// The renderer's volume for a built block, with where it sits for the
/// per-object pick (`None` for the fallback block of an empty grid).
fn volume_of(
    block: &BuiltBlock,
    level: &PendingGi,
    settings: &GiSettings,
    pick: Option<GiPick>,
) -> GiVolume {
    GiVolume {
        origin: block.origin,
        size: UVec3::new(SIDE as u32, SIDE as u32, block.depth as u32),
        voxel: VOXEL,
        floors: block.floors.clone(),
        cells: block.cells.clone(),
        base_ambient: Vec3::from(level.base_ambient)
            * if settings.high_quality {
                BASE_AMBIENT
            } else {
                BASE_AMBIENT_LOW_QUALITY
            },
        sector: block.sector,
        warm_up: settings.warm_up,
        top_slice: level.top_slice,
        pick,
    }
}

/// Lists the other blocks the manager steps this frame for the renderer:
/// every block the residency steps (built and holding one of the nine light
/// textures) except the camera's, in layout order. The list is left alone
/// while it names the same builds, so the surfaces keep their bindings.
#[allow(clippy::too_many_arguments)]
fn publish_neighbour_volumes(
    mut commands: Commands,
    source: Option<Res<GiSource>>,
    block: Option<Res<GiBlock>>,
    blocks: Option<Res<GiBlocks>>,
    residency: Option<Res<GiResidency>>,
    built: Res<GiBuiltBlocks>,
    settings: Res<GiSettings>,
    current: Option<Res<GiNeighbourVolumes>>,
) {
    let (Some(source), Some(block), Some(blocks), Some(residency)) =
        (source, block, blocks, residency)
    else {
        if current.is_some() {
            commands.remove_resource::<GiNeighbourVolumes>();
        }
        return;
    };
    let mut wanted: Vec<usize> = residency
        .stepping()
        .map(|b| b.block)
        .filter(|&i| i != block.index && built.0.contains_key(&i))
        .collect();
    wanted.sort_unstable();
    let volumes: Vec<GiVolume> = wanted
        .iter()
        .map(|&i| volume_of(&built.0[&i], &source.level, &settings, blocks.0.pick(i)))
        .collect();
    let same = |a: &GiVolume, b: &GiVolume| {
        a.sector == b.sector
            && Arc::ptr_eq(&a.floors, &b.floors)
            && a.base_ambient == b.base_ambient
            && a.top_slice == b.top_slice
    };
    if let Some(current) = &current
        && current.0.len() == volumes.len()
        && current.0.iter().zip(&volumes).all(|(a, b)| same(a, b))
    {
        return;
    }
    commands.insert_resource(GiNeighbourVolumes(volumes));
}

/// Marks each stepping block's sectors the camera needs and lists each
/// sector's 6 m cell lights, as the original does every frame: the cells
/// within six of the focus cell (and 37 cells squared), built in plane A,
/// whose box meets the view, then every such cell next to a marked one;
/// each block then draws a sector with the lights its cell's flood lists.
/// The camera's block (or the block warming behind it) fills
/// [`GiSectorLights`], each resident neighbour [`GiNeighbourSectorLights`].
#[allow(clippy::too_many_arguments)]
fn publish_sector_lights(
    mut lights: ResMut<GiSectorLights>,
    mut neighbour_lights: Option<ResMut<GiNeighbourSectorLights>>,
    source: Option<Res<GiSource>>,
    textures: Option<Res<exa_render::gi::GiTextures>>,
    pending: Option<Res<exa_render::gi::GiPendingTextures>>,
    neighbours: Option<Res<exa_render::gi::GiNeighbours>>,
    walls: Option<Res<LevelWalls>>,
    cameras: Query<(&GlobalTransform, &Frustum), exa_render::supersample::SceneCamera>,
    point_lights: Query<&exa_render::lights::LightId>,
    mut scene_lights: Local<Vec<u32>>,
) {
    let Some(source) = source else { return };
    let textures = pending.as_ref().map(|p| &p.0).or(textures.as_deref());
    let Some(textures) = textures else { return };
    let grid = &source.level.grid;
    let camera = cameras.iter().next();
    if walls.is_none() {
        // A level without a wall grid draws the scene's lights in its one
        // group. [guess: in their registration order, ascending ids]
        scene_lights.clear();
        scene_lights.extend(point_lights.iter().map(|id| id.0));
        scene_lights.sort_unstable();
    }
    let fill = |state: &mut GiSectors, textures: &exa_render::gi::GiTextures| {
        block_sectors(
            state,
            textures,
            grid,
            camera,
            walls.as_deref(),
            &scene_lights,
        );
    };
    fill(Arc::make_mut(&mut lights.0), textures);
    let Some(out) = neighbour_lights.as_deref_mut() else {
        return;
    };
    let blocks = neighbours.as_deref().map_or(&[][..], |n| &n.blocks[..]);
    out.0.resize_with(blocks.len(), Default::default);
    for (state, textures) in out.0.iter_mut().zip(blocks) {
        fill(Arc::make_mut(state), textures);
    }
}

/// One block's sector marks and cell lights for this frame.
fn block_sectors(
    state: &mut GiSectors,
    textures: &exa_render::gi::GiTextures,
    grid: &GiGrid,
    camera: Option<(&GlobalTransform, &Frustum)>,
    walls: Option<&LevelWalls>,
    scene_lights: &[u32],
) {
    let block = textures.sector;
    // Every built block fills the floors of its sectors; a cell no built
    // block covers keeps 255, as the manager starts them all.
    let floor = |x: i32, z: i32| {
        let (bx, bz) = (x - block.x, z - block.y);
        if (0..SECTORS as i32).contains(&bx) && (0..SECTORS as i32).contains(&bz) {
            textures.sector_floor(bx as usize, bz as usize)
        } else {
            u8::MAX
        }
    };
    let span = BlockSpan {
        top: textures.dims().z as f32,
        below: (-textures.origin.y / VOXEL).round(),
    };
    let marks = match camera {
        Some((at, frustum)) => {
            let focus = at.translation() + at.forward().as_vec3() * FOCUS_AHEAD;
            sector_marks(
                grid,
                focus_cell(grid, focus),
                block,
                span,
                floor,
                |centre, half| box_meets_view(frustum, centre, half),
            )
        }
        // No scene camera: light the whole block (loading screens and
        // headless views have no frustum).
        None => [true; SECTORS * SECTORS],
    };
    state.block = block;
    state.from_middle = block - IVec2::new((grid.width >> 1) as i32, (grid.height >> 1) as i32);
    state.grid = walls.is_some();
    state.drawn = marks;
    match walls {
        Some(walls) => {
            for z in 0..SECTORS {
                for x in 0..SECTORS {
                    let cell = ((block.x + x as i32) as usize, (block.y + z as i32) as usize);
                    state.sectors[x + z * SECTORS] = GiSector::new(walls.lights(cell.0, cell.1));
                }
            }
        }
        None => {
            state.sectors = [GiSector::new(scene_lights.iter().copied()); SECTORS * SECTORS];
        }
    }
}

/// A block's height as the sector boxes read it: its top slice count and
/// how many voxels its slice 0 sits below the world's zero.
#[derive(Clone, Copy, Debug)]
struct BlockSpan {
    top: f32,
    below: f32,
}

/// The level cell holding a world point (metres), as the original finds
/// the focus's cell: its centimetres over 600, rounded down, counted from
/// the grid's middle cell (the size halved, rounded down).
fn focus_cell(grid: &GiGrid, at: Vec3) -> IVec2 {
    let cell = |c: f32, n: u32| {
        ((c * CM_PER_M * PER_CELL_CM).floor() as i32).saturating_add((n >> 1) as i32)
    };
    IVec2::new(cell(at.x, grid.width), cell(at.z, grid.height))
}

/// Whether a sector's box (centre and half extents, centimetres) meets the
/// view, as the original tests it against the camera's convex volume, plane
/// by plane: the box is out when its centre lies beyond a plane by more
/// than the 6 m cell's diagonal, or by more than the box reaches
/// towards the plane. Each half extent takes the plane normal's sign bit on
/// top of its own, so a negative height (a cell without geometry, floor
/// 255) reaches away from the plane instead.
fn box_meets_view(frustum: &Frustum, centre: Vec3, half: Vec3) -> bool {
    const SIGN: u32 = 0x8000_0000;
    let signed = |h: f32, n: f32| f32::from_bits(h.to_bits() | (n.to_bits() & SIGN));
    frustum.half_spaces.iter().all(|plane| {
        // Bevy's planes face inwards and are in metres; the original's face
        // outwards, in centimetres.
        let n = -plane.normal();
        let w = -plane.d() * CM_PER_M;
        let d = ((centre.x * n.x + centre.y * n.y) + centre.z * n.z) + w;
        if d > SECTOR_RADIUS_CM {
            return false;
        }
        if d > 0.0 {
            let reach =
                (n.x * signed(half.x, n.x) + n.y * signed(half.y, n.y)) + n.z * signed(half.z, n.z);
            if d > reach {
                return false;
            }
        }
        true
    })
}

/// The sectors of the block at `block` the manager draws this frame. The
/// first pass marks a cell within six of the focus cell (and 1 to size − 2),
/// within 37 cells squared of it and built in plane A, when its box meets
/// the view: 300 cm either side of the cell's centre, from its floor slice
/// to the block's top. The second pass, over the same cells and with the
/// same window, reach and plane A tests, keeps a cell whose own mark or a
/// neighbour's is set; the neighbours are read at the cell's index ± 1 and
/// ± the grid's height (the original's stride: the row length only on a
/// square grid).
fn sector_marks(
    grid: &GiGrid,
    focus: IVec2,
    block: IVec2,
    span: BlockSpan,
    floor: impl Fn(i32, i32) -> u8,
    meets_view: impl Fn(Vec3, Vec3) -> bool,
) -> [bool; SECTORS * SECTORS] {
    let (w, h) = (grid.width as i32, grid.height as i32);
    let (lo_x, hi_x) = (
        focus.x.saturating_sub(6).max(1),
        focus.x.saturating_add(6).min(w - 2),
    );
    let (lo_z, hi_z) = (
        focus.y.saturating_sub(6).max(1),
        focus.y.saturating_add(6).min(h - 2),
    );
    let candidate = |x: i32, z: i32| {
        let (dx, dz) = (i64::from(x - focus.x), i64::from(z - focus.y));
        (lo_x..=hi_x).contains(&x)
            && (lo_z..=hi_z).contains(&z)
            && grid.open(i64::from(x), i64::from(z))
            && dz * dz + dx * dx <= 36
    };
    let top = VOXEL_CM * (span.top - span.below);
    let (middle_x, middle_z) = ((grid.width >> 1) as f32, (grid.height >> 1) as f32);
    let first = |x: i32, z: i32| {
        if !candidate(x, z) {
            return false;
        }
        let bottom = VOXEL_CM * (f32::from(floor(x, z)) - span.below);
        let half_y = (top - bottom) * 0.5;
        let centre = Vec3::new(
            ((x as f32 + 0.5) - middle_x) * CELL_CM,
            bottom + half_y,
            ((z as f32 + 0.5) - middle_z) * CELL_CM,
        );
        meets_view(centre, Vec3::new(HALF_CELL_CM, half_y, HALF_CELL_CM))
    };
    let (cells, row) = (i64::from(w) * i64::from(h), i64::from(w));
    let first_at = |i: i64| (0..cells).contains(&i) && first((i % row) as i32, (i / row) as i32);
    let stride = i64::from(h);
    let mut marks = [false; SECTORS * SECTORS];
    for z in 0..SECTORS as i32 {
        for x in 0..SECTORS as i32 {
            let (cx, cz) = (block.x + x, block.y + z);
            let i = i64::from(cz) * row + i64::from(cx);
            marks[(x + z * SECTORS as i32) as usize] = candidate(cx, cz)
                && [-stride, -1, 0, 1, stride]
                    .iter()
                    .any(|&step| first_at(i + step));
        }
    }
    marks
}

/// Builds the block centred on `focus`: terrain first, then the meshes'
/// points, then the floors, as the original builds a block.
fn build_block(
    spec: Option<GiBlockSpec>,
    focus: Vec3,
    level: PendingGi,
    inputs: &GiInputs,
) -> Arc<BuiltBlock> {
    let mut occupancy = match spec {
        Some(spec) => Occupancy::new_at(spec.origin, spec.depth, spec.sector),
        None => Occupancy::new(
            focus,
            level.top,
            level.bottom,
            (level.grid.width, level.grid.height),
        ),
    };
    let mut ground = TerrainColumns::new(&occupancy);
    for &t in &inputs.terrain {
        ground.add(t);
    }
    let terrain_columns = ground.apply(&mut occupancy);
    for &(p, thin, centred) in &inputs.points {
        occupancy.splat_styled(p, 1.0, thin, centred);
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
    let built = BuiltBlock {
        origin: occupancy.origin,
        depth: occupancy.depth,
        sector: occupancy.sector,
        cells: Arc::new(occupancy.cells),
        floors: Arc::new(floors),
        terrain_columns,
        floor_points,
        synced: false,
    };
    Arc::new(built)
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
/// Occupancy of a voxel below the terrain surface.
const TERRAIN_SOLID: u8 = 20;
/// The floor generator's parameter, in centimetres: the original's 20.
const FLOOR_PARAMETER: f32 = 20.0;
/// Centimetres to the metre.
const CM_PER_M: f32 = 100.0;
/// The original's reciprocal of a 6 m cell in centimetres (1/600 as f32).
const PER_CELL_CM: f32 = 1.0 / 600.0;
/// A 6 m cell, half of one and a voxel, in centimetres.
const CELL_CM: f32 = 600.0;
const HALF_CELL_CM: f32 = 300.0;
const VOXEL_CM: f32 = 20.0;
/// A sector box's bounding radius in the view test: 600·√2 cm (848.528 14),
/// the 6 m cell's diagonal, as the original stores it.
const SECTOR_RADIUS_CM: f32 = f32::from_bits(0x4454_21cd);

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

    /// Fills terrain as the original does (whole-voxel columns): with `k`
    /// the surface height in voxels truncated towards zero, a column below
    /// slice 0 (`k` < 0) is skipped; below the top slice voxel `k` holds its
    /// covered fraction of 20 (rounded to even) and slices 0..k hold 20; at
    /// or above it `k` clamps to the top slice, no surface byte is written
    /// and slices 0..D−1 hold 20 (so the lid's 5 in slice D−2 becomes 20 and
    /// the top slice's 26 stays). The column's lowest slice becomes `k`. The
    /// margins are left alone. Returns the columns filled.
    fn apply(&self, o: &mut Occupancy) -> usize {
        let mut filled = 0;
        let top_slice = o.depth as i32 - 1;
        for z in MARGIN..SIDE - MARGIN {
            for x in MARGIN..SIDE - MARGIN {
                let top = self.tops[x + z * SIDE];
                if !top.is_finite() {
                    continue;
                }
                let g = (top - self.origin.y) / VOXEL;
                // The original truncates (towards zero), so a surface less
                // than a voxel below slice 0 still counts as slice 0.
                let mut k = g as i32;
                if k < 0 {
                    continue;
                }
                if k < top_slice {
                    // A surface in (-1, 0) voxels gives a negative byte,
                    // which the original stores wrapped; kept as it is.
                    let frac = g - k as f32;
                    o.cells[x + z * SIDE + k as usize * SIDE * SIDE] =
                        (frac * TERRAIN_SOLID as f32).round_ties_even() as i32 as u8;
                } else {
                    k = top_slice;
                }
                for y in 0..k as usize {
                    o.cells[x + z * SIDE + y * SIDE * SIDE] = TERRAIN_SOLID;
                }
                // Terrain is filled first, so the original's plain store
                // is the column's lowest slice.
                o.lowest[x + z * SIDE] = k as u8;
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
        Self::new_at(origin, depth, sector)
    }

    /// An empty block at a laid-out origin, height and first sector
    /// ([`GiBlockLayout`]).
    pub fn new_at(origin: Vec3, depth: usize, sector: IVec2) -> Self {
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
        let cell = |c: f32| (c * CM_PER_M * PER_CELL_CM).floor() as i32;
        let sector_x = (w / 2 + cell(centre.x) - SECTORS as i32 / 2).clamp(0, w - SECTORS as i32);
        let sector_z = (h / 2 + cell(centre.z) - SECTORS as i32 / 2).clamp(0, h - SECTORS as i32);
        // The block's first voxel column, counted in whole voxels from the
        // grid's middle cell (the size halved, rounded down, as the
        // original's integer division does), less the margin.
        let corner = |s: i32, n: i32| ((s - n / 2) * SECTOR as i32 - MARGIN as i32) as f32 * VOXEL;
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
        self.splat_styled(p, 1.0, false, false);
    }

    /// Splats a sample point with the original's thin-mesh and centred
    /// variants: a thin mesh (a cloth or a sheet) weighs a quarter, and a
    /// centred one adds half a voxel to its height (after the half-voxel
    /// shift every point gets) and leaves the column's lowest slice alone;
    /// both keep the same bounds test. `weight` is 1 for
    /// a normal point (the original's 20) and −1 to take an old position back
    /// out.
    pub fn splat_styled(&mut self, p: Vec3, weight: f32, thin: bool, centred: bool) {
        let weight = weight * POINT_WEIGHT * if thin { 0.25 } else { 1.0 };
        let g = (p - self.origin) / VOXEL - Vec3::splat(0.5)
            + if centred { Vec3::Y * 0.5 } else { Vec3::ZERO };
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
            let layer = wy * weight;
            for (dz, wz) in [(0, 1.0 - f.z), (1, f.z)] {
                for (dx, wx) in [(0, 1.0 - f.x), (1, f.x)] {
                    self.add(x + dx, y + dy, z + dz, (wx * wz) * layer);
                }
            }
        }
        if !centred {
            let column = &mut self.lowest[x + z * SIDE];
            *column = (*column).min(y as u8);
        }
    }

    /// The region of the volume a point set touches, for a sub-upload: the
    /// original grows the box by 1.5 voxels on each side in world voxels
    /// (CVTSS2SI, rounding ties to even), takes the block's first voxel off,
    /// clamps it to the block (its end to one short of the size), and leaves
    /// `None` when it is empty. The
    /// original boxes the object's oriented bounds (old and new); this takes
    /// the extent of the points it is given. [adaptation: point extent]
    pub fn region_of(&self, points: impl IntoIterator<Item = Vec3>) -> Option<VoxelRegion> {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for p in points {
            lo = lo.min(p);
            hi = hi.max(p);
        }
        if !lo.is_finite() || !hi.is_finite() {
            return None;
        }
        let inv = Vec3::splat(1.0 / VOXEL);
        let first = (self.origin * inv).round();
        let lo = (lo * inv - Vec3::splat(1.5)).map(f32::round_ties_even) - first;
        let hi = (hi * inv + Vec3::splat(1.5)).map(f32::round_ties_even) - first;
        // Texture axes: X, then Z, then height.
        let dims = UVec3::new(SIDE as u32, SIDE as u32, self.depth as u32);
        let lo = Vec3::new(lo.x.max(0.0), lo.z.max(0.0), lo.y.max(0.0));
        let hi = Vec3::new(hi.x.max(0.0), hi.z.max(0.0), hi.y.max(0.0));
        // The upload keeps its end below the last voxel on each axis (an end
        // at or past the size becomes size − 1), as the original's does.
        let lo = lo.min(dims.as_vec3());
        let hi = hi.min(dims.as_vec3() - Vec3::ONE);
        let lo = UVec3::new(lo.x as u32, lo.y as u32, lo.z as u32);
        let hi = UVec3::new(hi.x as u32, hi.y as u32, hi.z as u32);
        (lo.x < hi.x && lo.y < hi.y && lo.z < hi.z).then_some(VoxelRegion { lo, hi })
    }

    /// Takes an object's old point set out of the grid and puts its new one
    /// in, as the original's dynamic update does with weights ∓20, and
    /// returns the region both touch. The mesh's flags choose the thin and
    /// centred variants.
    pub fn update_dynamic(
        &mut self,
        old: &[Vec3],
        new: &[Vec3],
        thin: bool,
        centred: bool,
    ) -> Option<VoxelRegion> {
        let region = self.region_of(old.iter().chain(new).copied())?;
        for &p in old {
            self.splat_styled(p, -1.0, thin, centred);
        }
        for &p in new {
            self.splat_styled(p, 1.0, thin, centred);
        }
        Some(region)
    }

    /// A sector's floor for each column: the lowest slice any column of its
    /// 6 m sector (all 30 × 30 columns inside the 8-voxel margin) holds, or
    /// 255 where the sector holds nothing (or in the margin). The original
    /// keeps one byte per sector; Exarust repeats it over the sector's
    /// columns.
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

/// A half-open voxel region in texture axes (X, then Z, then height).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoxelRegion {
    pub lo: UVec3,
    pub hi: UVec3,
}

impl VoxelRegion {
    /// Whether the region holds no voxel.
    pub fn is_empty(&self) -> bool {
        self.lo.x >= self.hi.x || self.lo.y >= self.hi.y || self.lo.z >= self.hi.z
    }
}

/// The bytes of `cells` inside `region`, X fastest, then Z, then height, as
/// the original's sub-upload reads them. `None` for an empty or out-of-range
/// region.
pub fn sub_upload(cells: &[u8], dims: UVec3, region: VoxelRegion) -> Option<Vec<u8>> {
    let lo = region.lo.min(dims);
    let hi = region.hi.min(dims);
    if lo.x >= hi.x || lo.y >= hi.y || lo.z >= hi.z {
        return None;
    }
    let mut out = Vec::with_capacity(((hi.x - lo.x) * (hi.y - lo.y) * (hi.z - lo.z)) as usize);
    for y in lo.z..hi.z {
        for z in lo.y..hi.y {
            let row = (z * dims.x + y * dims.x * dims.y) as usize;
            let start = row + lo.x as usize;
            let end = row + hi.x as usize;
            out.extend_from_slice(cells.get(start..end)?);
        }
    }
    Some(out)
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
            synthetic: false,
        }
    }

    /// A block holding one point at the centre of voxel (100, 100) of slice
    /// 5, so the point fills exactly that cell.
    fn one_point_block(at: Vec3, synced: bool) -> GiBuiltBlocks {
        let mut occupancy = Occupancy::new_at(Vec3::ZERO, 64, IVec2::new(4, 4));
        occupancy.splat(at);
        let block = Arc::new(BuiltBlock {
            origin: occupancy.origin,
            depth: occupancy.depth,
            sector: occupancy.sector,
            cells: Arc::new(occupancy.cells.clone()),
            floors: Arc::new(occupancy.floors()),
            terrain_columns: 1,
            floor_points: 1,
            synced,
        });
        let mut built = GiBuiltBlocks::default();
        built.0.insert(0, block);
        built
    }

    /// The occupancy byte of the cell holding `p` in block 0.
    fn cell_at(app: &App, p: Vec3) -> u8 {
        let cells = &app.world().resource::<GiBuiltBlocks>().0[&0].cells;
        let g = (p / VOXEL).floor().as_ivec3();
        cells[(g.x + g.y * SIDE as i32 * SIDE as i32 + g.z * SIDE as i32) as usize]
    }

    /// A moving GI mesh takes its old points out of every built block it
    /// meets and puts its new ones in, and the changed region goes out as a
    /// sub-upload once; a mesh that has not moved is left alone.
    #[test]
    fn a_moving_gi_mesh_updates_the_blocks_it_meets() {
        use exa_render::gi::{GiOccupancyPatch, GiOccupancyPatches};

        let at = Vec3::new(20.1, 1.1, 20.1);
        let mut app = App::new();
        app.insert_resource(one_point_block(at, true))
            .insert_resource(GiOccupancyPatches::default())
            .add_systems(Update, update_dynamic);
        let e = app
            .world_mut()
            .spawn((
                GiPoints(Arc::new(vec![at])),
                GlobalTransform::IDENTITY,
                GiMoved {
                    source: Affine3A::IDENTITY,
                    last: Affine3A::IDENTITY,
                },
            ))
            .id();
        // Unmoved: nothing changes.
        app.update();
        assert!(app.world().resource::<GiOccupancyPatches>().0.is_empty());
        assert_eq!(cell_at(&app, at), 20);
        // Moving it one voxel along x takes the old point out and puts the
        // new one in.
        let moved = Affine3A::from_translation(Vec3::new(VOXEL, 0.0, 0.0));
        *app.world_mut().get_mut::<GlobalTransform>(e).unwrap() = moved.into();
        app.update();
        let patches = app.world().resource::<GiOccupancyPatches>();
        assert_eq!(patches.0.len(), 1);
        let patch: &GiOccupancyPatch = &patches.0[0];
        assert_eq!(patch.sector, IVec2::new(4, 4));
        // The region spans the old and new cells, padded 1.5 voxels each
        // side and rounded ties to even: x 100.5 − 1.5 = 99 and 101.5 + 1.5
        // = 103; the texture's Z (world z) 99 and 102; the height 4 and 7.
        assert_eq!(patch.lo, UVec3::new(99, 99, 4));
        assert_eq!(patch.hi, UVec3::new(103, 102, 7));
        assert_eq!(patch.bytes.len(), 4 * 3 * 3);
        // The old cell is empty, the new one holds the point's 20.
        assert_eq!(cell_at(&app, at), 0);
        assert_eq!(cell_at(&app, at + Vec3::X * VOXEL), 20);
        let frames = app.world().get::<GiMoved>(e).unwrap();
        assert_eq!((frames.source, frames.last), (Affine3A::IDENTITY, moved));
        // The next frame drains the patch: the renderer applies each once.
        app.update();
        assert!(app.world().resource::<GiOccupancyPatches>().0.is_empty());
    }

    /// While a block build is in flight the update waits and keeps the old
    /// frame, as the original returns while its build count is above zero;
    /// a block built afterwards gets the mesh moved from where the source
    /// collected it to where it last was.
    #[test]
    fn the_dynamic_update_waits_for_a_build_and_syncs_new_blocks() {
        use exa_render::gi::GiOccupancyPatches;

        let at = Vec3::new(20.1, 1.1, 20.1);
        let moved = Affine3A::from_translation(Vec3::new(VOXEL, 0.0, 0.0));
        let mut app = App::new();
        app.add_plugins(TaskPoolPlugin::default())
            .insert_resource(GiBuiltBlocks::default())
            .insert_resource(GiOccupancyPatches::default())
            .add_systems(Update, update_dynamic);
        let e = app
            .world_mut()
            .spawn((
                GiPoints(Arc::new(vec![at])),
                GlobalTransform::from(moved),
                GiMoved {
                    source: Affine3A::IDENTITY,
                    last: Affine3A::IDENTITY,
                },
            ))
            .id();
        let task = AsyncComputeTaskPool::get()
            .spawn(async move { one_point_block(at, false).0[&0].clone() });
        app.world_mut().insert_resource(GiBuild {
            task,
            block: 0,
            background: true,
        });
        app.update();
        assert_eq!(
            app.world().get::<GiMoved>(e).unwrap().last,
            Affine3A::IDENTITY
        );
        // With no build the move is followed (no block to edit yet).
        app.world_mut().remove_resource::<GiBuild>();
        app.update();
        assert_eq!(app.world().get::<GiMoved>(e).unwrap().last, moved);
        // A block built now holds the point at the source frame; its first
        // pass brings it to the mesh's last frame.
        app.insert_resource(one_point_block(at, false));
        assert_eq!(cell_at(&app, at), 20);
        app.update();
        assert_eq!(app.world().resource::<GiOccupancyPatches>().0.len(), 1);
        assert!(app.world().resource::<GiBuiltBlocks>().0[&0].synced);
        assert_eq!(cell_at(&app, at), 0);
        assert_eq!(cell_at(&app, at + Vec3::X * VOXEL), 20);
    }

    /// The settings screen's shading quality reaches the GI settings: 4 and
    /// up use the high path, 3 the low one.
    #[test]
    fn the_shading_quality_feeds_the_gi_settings() {
        use exa_render::quality::ShadingQuality;
        let mut app = App::new();
        app.init_resource::<GiSettings>()
            .add_systems(Update, feed_shading_quality);
        app.insert_resource(ShadingQuality(3));
        app.update();
        assert!(!app.world().resource::<GiSettings>().high_quality);
        app.insert_resource(ShadingQuality(5));
        app.update();
        assert!(app.world().resource::<GiSettings>().high_quality);
        // Without the resource the default stays.
        app.world_mut().remove_resource::<ShadingQuality>();
        app.update();
        assert!(app.world().resource::<GiSettings>().high_quality);
    }

    #[test]
    fn each_block_s_pick_names_the_block_its_points_fall_in() {
        // On an odd grid the block grid's edge (the grid's half size in
        // float) sits half a cell off the blocks' own centres (its integer
        // half), so only points within 21 m of a centre are sure to pick it.
        for (size, reach) in [(32, 23.9), (33, 20.9)] {
            let layout = GiBlockLayout::new(&open_grid(size), 6.0, 0.0);
            assert!(layout.blocks.len() > 1);
            for (i, block) in layout.blocks.iter().enumerate() {
                let pick = layout.pick(i).unwrap();
                assert_eq!(pick.cells_origin_cm, layout.cells_origin_cm);
                // The block's centre and points near its edges pick it, by
                // the same rule the shader runs per object.
                let centre = layout.centre_cm(block) / 100.0;
                for (dx, dz) in [(0.0, 0.0), (-reach, -reach), (reach, reach)] {
                    let at = Vec3::new(centre.x + dx, 1.0, centre.y + dz);
                    assert_eq!(GiPick::cell_of(pick.cells_origin_cm, at), pick.cell);
                    assert_eq!(layout.block_at(at), Some(i));
                }
            }
            assert!(layout.pick(layout.blocks.len()).is_none());
        }
    }

    #[test]
    fn the_resident_blocks_beside_the_camera_s_are_published() {
        let mut app = App::new();
        app.add_plugins((TaskPoolPlugin::default(), GiVolumePlugin))
            .init_resource::<Assets<Mesh>>();
        app.world_mut()
            .spawn((Camera3d::default(), GlobalTransform::default()));
        let level = PendingGi {
            centre: Vec3::ZERO,
            top: 6.0,
            bottom: 0.0,
            base_ambient: [0.04, 0.04, 0.04],
            grid: open_grid(32),
            top_slice: false,
        };
        app.insert_resource(level.clone());
        update_until(&mut app, |w| {
            w.get_resource::<GiNeighbourVolumes>()
                .is_some_and(|n| n.0.len() >= 2)
        });
        let layout = GiBlockLayout::new(&level.grid, level.top, level.bottom);
        let active = app.world().resource::<GiVolume>().clone();
        let at = layout.block_at(Vec3::ZERO).unwrap();
        assert_eq!(active.pick, layout.pick(at));
        let neighbours = &app.world().resource::<GiNeighbourVolumes>().0;
        let mut last = None;
        for volume in neighbours {
            // Not the camera's block, listed in layout order, each with its
            // own pick.
            assert_ne!(volume.sector, active.sector);
            let i = layout
                .blocks
                .iter()
                .position(|b| b.sector == volume.sector)
                .unwrap();
            assert!(last.is_none_or(|l| l < i));
            last = Some(i);
            assert_eq!(volume.pick, layout.pick(i));
            assert_eq!(volume.origin, layout.blocks[i].origin);
        }
        assert!(neighbours.len() < exa_render::gi::RESIDENT_BLOCKS);
    }

    #[test]
    fn blocks_are_built_off_the_main_thread_where_the_camera_stands() {
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
            top_slice: false,
        };
        app.insert_resource(level.clone());
        app.update();
        assert_eq!(*app.world().resource::<GiState>(), GiState::Building);
        update_until(&mut app, |w| w.contains_resource::<GiVolume>());
        assert_eq!(*app.world().resource::<GiState>(), GiState::Built);
        // The camera at the origin is in block (2, 2) of the 4 × 4 blocks:
        // its first sector is (16, 16), so its corner is 1.6 m short of the
        // world origin on each side.
        let layout = GiBlockLayout::new(&level.grid, level.top, level.bottom);
        let slot = 2 + 2 * layout.columns as usize;
        let which = layout.blocks.iter().position(|b| b.slot == slot).unwrap();
        let spec = layout.blocks[which];
        assert_eq!(spec.sector, IVec2::new(16, 16));
        let volume = app.world().resource::<GiVolume>();
        assert_eq!(volume.origin, spec.origin);
        assert_eq!(volume.sector, spec.sector);
        assert_eq!(
            volume.size,
            UVec3::new(SIDE as u32, SIDE as u32, spec.depth as u32)
        );
        // A move inside the block keeps it; a far one moves to the next.
        *app.world_mut().get_mut::<GlobalTransform>(camera).unwrap() =
            GlobalTransform::from_xyz(5.0, 0.0, 0.0);
        app.update();
        assert_eq!(*app.world().resource::<GiState>(), GiState::Built);
        *app.world_mut().get_mut::<GlobalTransform>(camera).unwrap() =
            GlobalTransform::from_xyz(60.0, 0.0, 0.0);
        app.update();
        assert_eq!(*app.world().resource::<GiState>(), GiState::Building);
        update_until(&mut app, |w| *w.resource::<GiState>() == GiState::Built);
        let far = layout.block_at(Vec3::new(60.0, 0.0, 0.0)).unwrap();
        assert_eq!(
            app.world().resource::<GiVolume>().origin,
            layout.blocks[far].origin
        );
        // Walking back into the first block reuses its cached occupancy.
        *app.world_mut().get_mut::<GlobalTransform>(camera).unwrap() =
            GlobalTransform::from_xyz(0.0, 0.0, 0.0);
        app.update();
        assert_eq!(*app.world().resource::<GiState>(), GiState::Built);
        assert_eq!(app.world().resource::<GiVolume>().origin, spec.origin);
        // A refocus rebuilds even when the block is already active.
        app.insert_resource(GiRefocus);
        app.update();
        assert_eq!(*app.world().resource::<GiState>(), GiState::Building);
        update_until(&mut app, |w| *w.resource::<GiState>() == GiState::Built);
        assert!(!app.world().contains_resource::<GiRefocus>());
    }

    #[test]
    fn blocks_cover_the_built_cells_in_eight_cell_steps() {
        // A 32 × 32 grid, all built inside the border: bounds (1, 1)–(30,
        // 30), four blocks a side, slid back to cell 0.
        let grid = open_grid(32);
        assert_eq!(built_bounds(&grid), (1, 1, 30, 30));
        let layout = GiBlockLayout::new(&grid, 6.0, 0.0);
        assert_eq!((layout.columns, layout.rows), (4, 4));
        assert_eq!(layout.cells_origin_cm, Vec2::splat(-9600.0));
        assert_eq!(layout.blocks.len(), 16);
        assert_eq!(layout.blocks[0].sector, IVec2::ZERO);
        assert_eq!(layout.blocks[0].origin, Vec3::new(-97.6, -6.8, -97.6));
        assert_eq!(layout.block_at(Vec3::ZERO), Some(10));
        assert_eq!(layout.block_at(Vec3::new(-30.0, 0.0, -30.0)), Some(5));
        assert_eq!(layout.block_at(Vec3::new(60.0, 0.0, 0.0)), Some(11));
        // 30 m in is 2.625 blocks from the edge: the floor keeps block 2.
        assert_eq!(layout.block_at(Vec3::new(30.0, 0.0, 0.0)), Some(10));
        // An odd grid's edge counts from its half size in float.
        let odd = GiBlockLayout::new(&open_grid(17), 6.0, 0.0);
        assert_eq!(odd.cells_origin_cm.x, (1.0 - 8.5) * 600.0);
        // An empty grid has the original's (-4, -4)–(3, 3) bounds.
        let empty = GiGrid {
            width: 16,
            height: 16,
            edges: Arc::new(vec![0; 16 * 16]),
            synthetic: false,
        };
        assert_eq!(built_bounds(&empty), (-4, -4, 3, 3));
        // A level without a grid gets one block at cell (3, 3).
        let synthetic = GiGrid::synthetic();
        let layout = GiBlockLayout::new(&synthetic, 6.0, 0.0);
        assert_eq!(layout.blocks.len(), 1);
        assert_eq!(layout.blocks[0].sector, IVec2::new(3, 3));
        assert_eq!(layout.blocks[0].origin, Vec3::new(-7.6, -6.8, -7.6));
        assert_eq!(layout.cells_origin_cm, Vec2::splat(-600.0));
    }

    #[test]
    fn the_built_bounds_grow_to_the_last_cell_in_a_row() {
        // Cells (1, 1), (5, 1) and (2, 3): x spans 1–5; row 3's leftward
        // extension moves x0 but the row lies in the old span, so z1 is 3.
        let mut edges = vec![0; 8 * 8];
        for (x, z) in [(1, 1), (5, 1), (2, 3)] {
            edges[x + z * 8] = OPEN;
        }
        let grid = GiGrid {
            width: 8,
            height: 8,
            edges: Arc::new(edges),
            synthetic: false,
        };
        assert_eq!(built_bounds(&grid), (1, 1, 5, 3));
        // A single built cell bounds itself.
        let mut edges = vec![0; 8 * 8];
        edges[9] = OPEN;
        let grid = GiGrid {
            width: 8,
            height: 8,
            edges: Arc::new(edges),
            synthetic: false,
        };
        assert_eq!(built_bounds(&grid), (1, 1, 1, 1));
        // The span's right end counts: cells (1, 1) and (1, 4) reach row 4.
        // A leftward extension reaches its row too: (3, 1) then (1, 5).
        for (cells, bounds) in [
            ([(1, 1), (1, 4)], (1, 1, 1, 4)),
            ([(3, 1), (1, 5)], (1, 1, 3, 5)),
        ] {
            let mut edges = vec![0; 8 * 8];
            for (x, z) in cells {
                edges[x + z * 8] = OPEN;
            }
            let grid = GiGrid {
                width: 8,
                height: 8,
                edges: Arc::new(edges),
                synthetic: false,
            };
            assert_eq!(built_bounds(&grid), bounds);
        }
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
        // An odd grid counts from its middle cell, the size halved and
        // rounded down: on a 17-cell grid x = 10 m is cell 1 + 8 = 9, the
        // block starts at cell 5, (5 − 8) × 30 − 8 = −98 voxels, so its
        // sector 4 (cell 9) spans 6 to 12 m, the cell's own span.
        let (origin, _, sector) =
            Occupancy::placement(Vec3::new(10.0, 0.0, 0.0), 6.0, 0.0, (17, 17));
        assert_eq!(sector.x, 5);
        assert!((origin.x + 19.6).abs() < 1e-5);
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
    fn a_thin_point_weighs_a_quarter_and_a_centred_one_straddles_slices() {
        let mut o = Occupancy::new(Vec3::ZERO, 6.0, 0.0, (8, 8));
        let p = o.origin + Vec3::new(100.5, 10.5, 100.5) * VOXEL;
        o.splat_styled(p, 1.0, true, false);
        assert_eq!(o.at(p), Some(5));
        // The centred variant adds half a voxel in height and leaves the
        // column's floor where it was.
        let mut o = Occupancy::new(Vec3::ZERO, 6.0, 0.0, (8, 8));
        o.splat_styled(p, 1.0, false, true);
        let base = o.origin + Vec3::new(100.5, 10.5, 100.5) * VOXEL;
        // Height 10 + 0.5 splits evenly between slices 10 and 11; the
        // horizontal neighbours and the slice below stay empty.
        assert_eq!(o.at(base), Some(10));
        assert_eq!(o.at(base + Vec3::Y * VOXEL), Some(10));
        assert_eq!(o.at(base - Vec3::Y * VOXEL), Some(0));
        assert_eq!(o.at(base + Vec3::Z * VOXEL), Some(0));
        assert_eq!(o.lowest[100 + 100 * SIDE], 255);
    }

    #[test]
    fn a_dynamic_update_takes_the_old_points_out_and_puts_the_new_ones_in() {
        let mut o = Occupancy::new_at(Vec3::ZERO, 64, IVec2::ZERO);
        let old = Vec3::new(100.5, 10.5, 100.5) * VOXEL;
        o.splat(old);
        let new = old + Vec3::X * VOXEL;
        let region = o.update_dynamic(&[old], &[new], false, false).unwrap();
        assert_eq!(o.at(old), Some(0));
        assert_eq!(o.at(new), Some(20));
        assert_eq!(region.lo, UVec3::new(99, 99, 9));
        assert_eq!(region.hi, UVec3::new(103, 102, 12));
    }

    #[test]
    fn a_dirty_region_grows_by_one_and_a_half_voxels() {
        let o = Occupancy::new_at(Vec3::ZERO, 64, IVec2::ZERO);
        let region = o
            .region_of([Vec3::new(100.5, 10.5, 100.5) * VOXEL])
            .unwrap();
        assert_eq!(region.lo, UVec3::new(99, 99, 9));
        assert_eq!(region.hi, UVec3::new(102, 102, 12));
        assert!(o.region_of([]).is_none());
        // 4 voxels in: 4 − 1.5 = 2.5 rounds to the even 2, 4 + 1.5 to 6.
        let region = o.region_of([Vec3::splat(4.0 * VOXEL)]).unwrap();
        assert_eq!(region.lo, UVec3::splat(2));
        assert_eq!(region.hi, UVec3::splat(6));
        // A box past the block's far side ends one voxel short of it.
        let far = o
            .region_of([Vec3::new(255.5, 10.5, 255.5) * VOXEL])
            .unwrap();
        assert_eq!(far.lo, UVec3::new(254, 254, 9));
        assert_eq!(far.hi, UVec3::new(255, 255, 12));
    }

    #[test]
    fn a_sub_upload_reads_x_then_z_then_height() {
        let dims = UVec3::new(2, 2, 2);
        let mut cells = Vec::new();
        for y in 0..2 {
            for z in 0..2 {
                for x in 0..2 {
                    cells.push((x + z * 2 + y * 4) as u8);
                }
            }
        }
        let region = VoxelRegion {
            lo: UVec3::ZERO,
            hi: dims,
        };
        assert_eq!(sub_upload(&cells, dims, region).unwrap(), cells);
        let region = VoxelRegion {
            lo: UVec3::new(1, 0, 1),
            hi: UVec3::new(2, 2, 2),
        };
        assert_eq!(sub_upload(&cells, dims, region).unwrap(), vec![5, 7]);
    }

    #[test]
    fn a_blocks_gi_position_is_relative_to_its_centre() {
        let grid = open_grid(32);
        let layout = GiBlockLayout::new(&grid, 6.0, 0.0);
        let block = layout.blocks[layout.block_at(Vec3::ZERO).unwrap()];
        // `p · scale + offset` with p in centimetres: the block's centre is
        // texture 0.5 in X and Z, and its bottom slice's middle in Y.
        let p = layout.sample_at(&block, block.centre, true);
        assert_eq!((p.x, p.z), (0.5, 0.5));
        // The block's lowest point reads half a slice up (a voxel's centre)
        // or its bottom edge.
        let bottom = Vec3::new(block.centre.x, block.origin.y, block.centre.z);
        let centre = layout.sample_at(&block, bottom, true);
        assert!((centre.y - 0.5 / block.depth as f32).abs() < 1e-6);
        assert!(layout.sample_at(&block, bottom, false).y.abs() < 1e-6);
        // The world origin lies half a block before the block's centre.
        let origin = layout.sample_at(&block, Vec3::ZERO, true);
        assert!((origin.x - (0.5 - 24.0 / 51.2)).abs() < 1e-6);
    }

    #[test]
    fn the_per_object_gi_uniform_follows_the_block_cells() {
        // The synthetic 8 × 8 grid's one block at cells (3, 3), a level
        // 6 m tall: 64 slices, 34 of them below the world's zero.
        let grid = GiGrid::synthetic();
        let layout = GiBlockLayout::new(&grid, 6.0, 0.0);
        let block = layout.blocks[0];
        assert_eq!(block.depth, 64);
        let sample = layout.sample_transform(&block, true);
        // ((8/2 − 3) − 4) · 30/256 + 0.5 = −3 · 0.1171875 + 0.5.
        assert_eq!(sample.offset.x, 0.1484375);
        assert_eq!(sample.offset.z, 0.1484375);
        // (34 + 0.5)/64, and 34/64 without the half-voxel offset.
        assert_eq!(sample.offset.y, 0.5390625);
        assert_eq!(layout.sample_transform(&block, false).offset.y, 0.53125);
        // One texture unit per 5120 cm across and per 1280 cm of height.
        assert_eq!(
            sample.scale,
            Vec3::new(1.0 / 5120.0, 1.0 / 1280.0, 1.0 / 5120.0)
        );
        // Its own bottom is 6.8 m below the world's zero.
        assert!((block.origin.y + 6.8).abs() < 1e-6);
        // A point outside the block grid has no block (the original binds
        // texture 0 and the zero offset there).
        assert!(layout.block_at(Vec3::splat(-10_000.0)).is_none());
    }

    #[test]
    fn terrain_at_or_above_the_lid_clamps_to_the_top_slice() {
        let column = 20 + 20 * SIDE;
        let fill = |top: f32| {
            let mut o = Occupancy::new_at(Vec3::ZERO, 64, IVec2::ZERO);
            let mut tops = vec![f32::NEG_INFINITY; SIDE * SIDE];
            tops[column] = top;
            let ground = TerrainColumns {
                origin: Vec3::ZERO,
                tops,
            };
            assert_eq!(ground.apply(&mut o), 1);
            o
        };
        let at = |o: &Occupancy, y: usize| o.cells[column + y * SIDE * SIDE];
        // 13.4 m is voxel 67, above the top slice: k clamps to 63, slices
        // 0..=62 hold 20 (the lid's 5 is overwritten), the top slice keeps
        // its 26 and no surface byte is written.
        let o = fill(13.4);
        assert_eq!(at(&o, 61), 20);
        assert_eq!(at(&o, 62), 20);
        assert_eq!(at(&o, 63), 26);
        assert_eq!(o.lowest[column], 63);
        // 12.5 m is voxel 62.5: below the top slice, so slice 62 holds
        // round(0.5 · 20) = 10 and slices 0..=61 hold 20.
        let o = fill(12.5);
        assert_eq!(at(&o, 61), 20);
        assert_eq!(at(&o, 62), 10);
        assert_eq!(at(&o, 63), 26);
        assert_eq!(o.lowest[column], 62);
        // 12.7 m is voxel 63.5: at the top slice, clamped, the 26 stays.
        let o = fill(12.7);
        assert_eq!(at(&o, 62), 20);
        assert_eq!(at(&o, 63), 26);
        assert_eq!(o.lowest[column], 63);
        // 3 cm below slice 0 (−0.15 voxel) truncates to slice 0: nothing is
        // filled under it and slice 0 takes round(−0.15 · 20) = −3, stored
        // as the byte 253.
        let o = fill(-0.03);
        assert_eq!(at(&o, 0), 253);
        assert_eq!(at(&o, 1), 0);
        assert_eq!(o.lowest[column], 0);
        // A whole voxel below is skipped.
        let mut o = Occupancy::new_at(Vec3::ZERO, 64, IVec2::ZERO);
        let mut tops = vec![f32::NEG_INFINITY; SIDE * SIDE];
        tops[column] = -0.25;
        let ground = TerrainColumns {
            origin: Vec3::ZERO,
            tops,
        };
        assert_eq!(ground.apply(&mut o), 0);
        assert_eq!(o.lowest[column], 255);
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
    fn a_sectors_last_column_lowers_its_floor() {
        let mut o = Occupancy::new_at(Vec3::ZERO, 64, IVec2::ZERO);
        // Voxel column 8 + 29 (the sector's last) at slice 30: the
        // original's scan runs over all 30 columns and rows of the sector.
        let p = o.origin + Vec3::new(37.5, 30.5, 37.5) * VOXEL;
        o.splat(p);
        assert_eq!(o.lowest[37 + 37 * SIDE], 30);
        assert_eq!(o.floors()[8 + 8 * SIDE], 30);
        assert_eq!(o.floors()[37 + 37 * SIDE], 30);
    }

    /// A block 64 slices high whose slice 0 sits at the world's zero.
    const LOW_BLOCK: BlockSpan = BlockSpan {
        top: 64.0,
        below: 0.0,
    };

    #[test]
    fn a_sector_is_drawn_when_the_view_meets_its_box() {
        let grid = open_grid(16);
        let block = IVec2::new(4, 4);
        let floor = |_: i32, _: i32| 0u8;
        // Cell (10, 10) is centred (10.5 − 8) × 600 = 1500 cm out on both
        // axes; its box runs from the floor (0) to the block's top
        // (64 × 20 cm), so it is centred at 640 cm, 640 cm half high.
        let at = Vec3::new(1500.0, 640.0, 1500.0);
        let marks = sector_marks(&grid, IVec2::new(8, 8), block, LOW_BLOCK, floor, |c, h| {
            c == at && h == Vec3::new(300.0, 640.0, 300.0)
        });
        let drawn: Vec<usize> = (0..64).filter(|&i| marks[i]).collect();
        // That sector (6, 6) of the block and the four neighbours the
        // second pass adds (the diagonal ones are not neighbours).
        assert_eq!(drawn, vec![46, 53, 54, 55, 62]);
        // A cell outside the six-cell window (or on the grid's border) is
        // never marked, even when the view sees it.
        let marks = sector_marks(
            &grid,
            IVec2::new(2, 2),
            IVec2::ZERO,
            LOW_BLOCK,
            floor,
            |_, _| true,
        );
        assert!(!marks[0]);
        assert!(marks[1 + 8]);
    }

    #[test]
    fn sector_marks_need_the_reach_and_a_built_cell_in_both_passes() {
        let mut grid = open_grid(16);
        let block = IVec2::new(4, 4);
        let floor = |_: i32, _: i32| 0u8;
        let seen = |_: Vec3, _: Vec3| true;
        // From the focus cell (9, 9), cell (4, 4) is 25 + 25 = 50 cells
        // squared away: past the reach. (5, 5) at 32 is within it.
        let marks = sector_marks(&grid, IVec2::new(9, 9), block, LOW_BLOCK, floor, seen);
        assert!(!marks[0]);
        assert!(marks[1 + 8]);
        // A cell not built in plane A is not marked, though its neighbours
        // are.
        let mut edges = (*grid.edges).clone();
        edges[4 * 16 + 4] = 0;
        grid.edges = Arc::new(edges);
        let marks = sector_marks(&grid, IVec2::new(8, 8), block, LOW_BLOCK, floor, seen);
        assert!(!marks[0]);
        assert!(marks[1] && marks[8]);
    }

    #[test]
    fn the_second_pass_steps_rows_by_the_grids_height() {
        // A grid 16 cells wide and 12 deep: the original steps ± 12 cells
        // through the row-major marks, not ± 16.
        let grid = GiGrid {
            width: 16,
            height: 12,
            edges: Arc::new(vec![OPEN; 16 * 12]),
            synthetic: false,
        };
        let block = IVec2::new(4, 2);
        // Only cell (10, 6), centred at (1500, ·, 300) cm, is in view.
        let marks = sector_marks(
            &grid,
            IVec2::new(8, 6),
            block,
            LOW_BLOCK,
            |_, _| 0,
            |c, _| c.x == 1500.0 && c.z == 300.0,
        );
        let drawn: Vec<IVec2> = (0..64)
            .filter(|&i| marks[i])
            .map(|i| block + IVec2::new(i as i32 % 8, i as i32 / 8))
            .collect();
        // Index 106 ± 1 are its row neighbours and 106 + 12 is cell (6, 7);
        // 106 − 12 is cell (14, 5), outside the block (and 37 cells squared
        // from the focus). Cells (10, 5) and (10, 7) are not marked.
        assert_eq!(
            drawn,
            vec![
                IVec2::new(9, 6),
                IVec2::new(10, 6),
                IVec2::new(11, 6),
                IVec2::new(6, 7)
            ]
        );
    }

    /// A view whose one real plane is `plane` (Bevy's: inward normal, metres)
    /// and whose other planes are 1 km away.
    fn view(plane: Vec4) -> Frustum {
        use bevy::shape::{HalfSpace, ViewFrustum};
        let far = |n: Vec3| HalfSpace::new(n.extend(1000.0));
        Frustum(ViewFrustum {
            half_spaces: [
                HalfSpace::new(plane),
                far(Vec3::X),
                far(Vec3::NEG_X),
                far(Vec3::Y),
                far(Vec3::Z),
                far(Vec3::NEG_Z),
            ],
        })
    }

    #[test]
    fn a_box_is_cut_by_its_reach_and_its_radius() {
        let half = Vec3::new(300.0, 640.0, 300.0);
        // The view keeps x < 0: a box centred 500 cm past it reaches only
        // 300 cm back; one 200 cm past it reaches in.
        let x = view(Vec4::new(-1.0, 0.0, 0.0, 0.0));
        assert!(!box_meets_view(&x, Vec3::new(500.0, 0.0, 0.0), half));
        assert!(box_meets_view(&x, Vec3::new(200.0, 0.0, 0.0), half));
        // The view keeps y < 0: a box centred 900 cm above reaches 640 cm
        // down but its centre is past the 848.5 cm radius.
        let y = view(Vec4::new(0.0, -1.0, 0.0, 0.0));
        assert!(!box_meets_view(
            &y,
            Vec3::new(0.0, 900.0, 0.0),
            Vec3::new(300.0, 1280.0, 300.0)
        ));
        assert!(box_meets_view(
            &y,
            Vec3::new(0.0, 800.0, 0.0),
            Vec3::new(300.0, 1280.0, 300.0)
        ));
        // A negative half height keeps its sign: against an outward normal
        // pointing down it reaches 500 cm; pointing up, −500 cm.
        let up = view(Vec4::new(0.0, 1.0, 0.0, 0.0));
        let low = Vec3::new(300.0, -500.0, 300.0);
        assert!(box_meets_view(&up, Vec3::new(0.0, -100.0, 0.0), low));
        assert!(!box_meets_view(&y, Vec3::new(0.0, 100.0, 0.0), low));
        // A centre that is not a number is kept, as the unordered compares
        // keep it.
        assert!(box_meets_view(&x, Vec3::splat(f32::NAN), half));
        assert!((SECTOR_RADIUS_CM - 600.0 * 2f32.sqrt()).abs() < 1e-3);
    }

    #[test]
    fn the_focus_cell_is_the_floor_of_centimetres_over_600() {
        assert_eq!(PER_CELL_CM.to_bits(), 0x3ada_740e);
        let grid = open_grid(16);
        assert_eq!(
            focus_cell(&grid, Vec3::new(-0.01, 0.0, 5.99)),
            IVec2::new(7, 8)
        );
        assert_eq!(
            focus_cell(&grid, Vec3::new(6.0, 0.0, -6.0)),
            IVec2::new(9, 7)
        );
        // An odd grid's middle cell is its size halved, rounded down.
        let odd = GiGrid {
            width: 17,
            height: 17,
            edges: Arc::new(vec![OPEN; 17 * 17]),
            synthetic: false,
        };
        assert_eq!(focus_cell(&odd, Vec3::ZERO), IVec2::new(8, 8));
    }
}
