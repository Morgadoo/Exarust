//! The occupancy grid of the original's voxel global illumination: a block
//! of 20 cm voxels, 256 × 256 across (48 m plus margins) and 64 or 128
//! high, below the level's highest point. Meshes mark their cells solid by
//! splatting their GI sample points, tile-grid floors become a solid layer
//! under their surface, and the top two slices form a soft lid.
//! Light then spreads through the open cells each frame (`exa_render::gi`).
//! Rules: `docs/RENDERING.md` (Voxel GI).

use std::sync::Arc;

use bevy::prelude::*;
use bevy::transform::TransformSystems;
use exa_render::gi::GiVolume;

pub struct GiVolumePlugin;

impl Plugin for GiVolumePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GiSettings>()
            .add_systems(PostUpdate, build_volume.after(TransformSystems::Propagate));
    }
}

/// The GI sample points of a mesh node, in its space (metres).
#[derive(Component, Clone)]
pub struct GiPoints(pub Arc<Vec<Vec3>>);

/// A tile-grid floor mesh: its upward faces become a solid layer of voxels
/// just under the surface, as the original generates for each cell's floor.
#[derive(Component, Clone, Copy)]
pub struct GiFloor;

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

/// The level record's secondary ambient (`0x30`) scale for the per-step base
/// ambient, at the original's top shading quality.
pub const BASE_AMBIENT: f32 = 0.001;

fn build_volume(
    mut commands: Commands,
    pending: Option<Res<PendingGi>>,
    settings: Res<GiSettings>,
    points: Query<(&GiPoints, &GlobalTransform)>,
    floors: Query<(&Mesh3d, &GlobalTransform), With<GiFloor>>,
    meshes: Res<Assets<Mesh>>,
    fires: Query<&crate::fire::Fire>,
) {
    let Some(pending) = pending else { return };
    // The warm-up injects the lights as they are now: wait for the fires to
    // ramp up.
    if fires.iter().any(|f| !f.settled()) {
        return;
    }
    commands.remove_resource::<PendingGi>();
    if !settings.enabled {
        commands.remove_resource::<GiVolume>();
        return;
    }
    let mut occupancy = Occupancy::new(pending.centre, pending.top, pending.bottom);
    let (mut nodes, mut splats) = (0, 0);
    for (gi, at) in &points {
        let m = at.affine();
        for &p in gi.0.iter() {
            occupancy.splat(m.transform_point3(p));
        }
        nodes += 1;
        splats += gi.0.len();
    }
    let mut surface = FloorSurface::new(&occupancy);
    for (mesh, at) in &floors {
        let Some(mesh) = meshes.get(&mesh.0) else {
            continue;
        };
        let m = at.affine();
        let Some(positions) = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
        else {
            continue;
        };
        let Some(indices) = mesh.indices() else {
            continue;
        };
        let world: Vec<Vec3> = positions
            .iter()
            .map(|&p| m.transform_point3(Vec3::from(p)))
            .collect();
        let corners: Vec<usize> = indices.iter().collect();
        for t in corners.as_chunks::<3>().0 {
            surface.add([world[t[0]], world[t[1]], world[t[2]]]);
        }
    }
    let floor_points = surface.apply(&mut occupancy);
    info!(
        "GI volume: {nodes} meshes, {splats} points, {floor_points} floor voxels, origin {:.2}, {} slices",
        occupancy.origin, occupancy.depth
    );
    if let Some(path) = std::env::var_os("EXA_GI_DUMP") {
        let _ = std::fs::write(path, &occupancy.cells);
    }
    commands.insert_resource(GiVolume {
        origin: occupancy.origin,
        size: UVec3::new(SIDE as u32, SIDE as u32, occupancy.depth as u32),
        voxel: VOXEL,
        floors: Arc::new(occupancy.floors()),
        cells: Arc::new(occupancy.cells),
        base_ambient: pending.base_ambient,
        warm_up: settings.warm_up,
    });
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

/// The top of the floors over the block, per 10 cm column (X fastest),
/// for the floor layer.
struct FloorSurface {
    origin: Vec3,
    tops: Vec<f32>,
}

impl FloorSurface {
    fn new(o: &Occupancy) -> Self {
        Self {
            origin: o.origin,
            tops: vec![f32::NEG_INFINITY; SIDE * FINE * SIDE * FINE],
        }
    }

    /// Raises the columns under an upward-facing triangle to its height.
    fn add(&mut self, t: [Vec3; 3]) {
        // Only faces turned up (counter-clockwise from above).
        let n = (t[1] - t[0]).cross(t[2] - t[0]);
        if n.y <= 1e-7 * n.length() {
            return;
        }
        let fine = VOXEL / FINE as f32;
        let to_grid = |p: Vec3| ((p - self.origin) / fine).xz();
        let [a, b, c] = t.map(to_grid);
        let lo = a.min(b).min(c).floor().max(Vec2::ZERO);
        let hi = a
            .max(b)
            .max(c)
            .ceil()
            .min(Vec2::splat((SIDE * FINE) as f32));
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
                let top = &mut self.tops[x + z * SIDE * FINE];
                *top = top.max(h);
            }
        }
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
        let depth = if top - bottom > TALL_FROM { 128 } else { 64 };
        // Sectors fall on world multiples of 6 m, the block four sectors
        // either side of the one nearest the centre, plus the margin.
        let corner = |c: f32| {
            ((c / SECTOR_M).round() - SECTORS as f32 / 2.0) * SECTOR_M - MARGIN as f32 * VOXEL
        };
        // The top sits a whole number of voxels above the origin.
        let below = ((depth as f32 * VOXEL - top) / VOXEL).trunc();
        let origin = Vec3::new(corner(centre.x), -below * VOXEL, corner(centre.z));
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
