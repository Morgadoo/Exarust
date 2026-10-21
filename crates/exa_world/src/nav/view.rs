//! The grid in the game: built for each level once its nodes have world
//! transforms (on the async compute pool: a large level's columns take a
//! second), kept as the [`NavGrid`] resource, and drawn on demand
//! (`--debug-nav`, F6): every floor cell within 8 m of where the camera
//! looks, a square at its floor's height, green through yellow as its
//! clearance rises to what a walker passes, red beyond; the upper layer in
//! cyan through blue, magenta beyond.

use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use bevy::transform::TransformSystems;

use super::finder::InstalledPathfinder;
use super::source::FloorMesh;
use super::{CELL, GroundTile, Layer, NavGrid, NavSource, PASSABLE};
use crate::gi_volume::{GiFloor, GiPoints};

pub struct NavPlugin;

impl Plugin for NavPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavState>()
            .init_resource::<InstalledPathfinder>()
            .init_resource::<NavOverlay>()
            .add_systems(
                PostUpdate,
                (collect_source, finish_build)
                    .chain()
                    .after(TransformSystems::Propagate),
            )
            .add_systems(
                Update,
                (overlay_key, draw_overlay.run_if(|o: Res<NavOverlay>| o.on)).chain(),
            );
    }
}

/// A level waiting for its grid, once its nodes have world transforms.
#[derive(Resource, Debug, Clone, Copy)]
pub struct PendingNav {
    /// The level's root: the grid is built from what is under it.
    pub root: Entity,
    /// Cells along X and Z ([`super::source::grid_size`]).
    pub width: usize,
    pub depth: usize,
    /// Height (metres) voxel 0 tops out at ([`super::source::grid_base`]).
    pub base: f32,
}

/// Where the level's grid stands.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NavState {
    /// No level asked for one: queries answer as the original's fallback
    /// (no obstacle, no cost).
    #[default]
    None,
    Building,
    Built,
}

/// The grid being built off the main thread.
#[derive(Resource)]
struct NavBuild(Task<NavGrid>);

/// A terrain tile's ground, on its entity.
#[derive(Component, Clone)]
pub struct Ground(pub Arc<GroundTile>);

/// Whether the grid is drawn.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct NavOverlay {
    pub on: bool,
}

/// Shows and hides the overlay.
pub const OVERLAY_KEY: KeyCode = KeyCode::F6;
/// How far from its centre the overlay draws cells, metres.
const OVERLAY_RADIUS: f32 = 8.0;
/// How far the camera's view is followed to find the floor it looks at.
const LOOK_REACH: f32 = 60.0;
/// The squares sit this far above their floor and this far inside their
/// cell, metres.
const MARK_LIFT: f32 = 0.02;
const MARK_INSET: f32 = 0.03;

#[allow(clippy::too_many_arguments)]
fn collect_source(
    mut commands: Commands,
    pending: Option<Res<PendingNav>>,
    children: Query<&Children>,
    points: Query<(&GiPoints, &GlobalTransform)>,
    floors: Query<(&Mesh3d, &GlobalTransform), With<GiFloor>>,
    ground: Query<&Ground>,
    meshes: Res<Assets<Mesh>>,
    mut state: ResMut<NavState>,
    mut installed: ResMut<InstalledPathfinder>,
) {
    let Some(pending) = pending else { return };
    let pending = *pending;
    commands.remove_resource::<PendingNav>();
    commands.remove_resource::<NavGrid>();
    // A build for the previous level is dropped, which cancels it.
    commands.remove_resource::<NavBuild>();
    let mut source = NavSource {
        width: pending.width,
        depth: pending.depth,
        base: pending.base,
        ..default()
    };
    for e in std::iter::once(pending.root).chain(children.iter_descendants(pending.root)) {
        if let Ok((gi, at)) = points.get(e) {
            source.meshes.push((gi.0.clone(), at.affine()));
        }
        if let Ok((mesh, at)) = floors.get(e)
            && let Some(floor) = floor_mesh(meshes.get(&mesh.0), at.affine())
        {
            source.floors.push(floor);
        }
        if let Ok(g) = ground.get(e) {
            source.ground.push(g.0.clone());
        }
    }
    info!(
        "navigation grid: {} × {} cells from {} meshes' points, {} floor meshes, {} terrain tiles",
        source.width,
        source.depth,
        source.meshes.len(),
        source.floors.len(),
        source.ground.len()
    );
    let task = AsyncComputeTaskPool::get().spawn(async move { source.build() });
    commands.insert_resource(NavBuild(task));
    *state = NavState::Building;
    // The level's pathfinder is not installed until its grid is built: the
    // global falls back to the abstract one meanwhile.
    *installed = InstalledPathfinder::install_fallback();
}

/// A floor mesh's copy for the build.
pub(super) fn floor_mesh(mesh: Option<&Mesh>, at: bevy::math::Affine3A) -> Option<FloorMesh> {
    let mesh = mesh?;
    Some(FloorMesh {
        positions: mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)?
            .as_float3()?
            .to_vec(),
        indices: mesh.indices()?.iter().map(|i| i as u32).collect(),
        at,
    })
}

fn finish_build(
    mut commands: Commands,
    build: Option<ResMut<NavBuild>>,
    mut state: ResMut<NavState>,
    mut installed: ResMut<InstalledPathfinder>,
) {
    let Some(mut build) = build else { return };
    let Some(grid) = block_on(poll_once(&mut build.0)) else {
        return;
    };
    commands.remove_resource::<NavBuild>();
    let s = grid.stats();
    info!(
        "navigation grid built: columns by spans {:?}, floors {:?} (lower, upper), passable {:?}, open sky {:?}",
        s.columns_by_spans, s.floors, s.passable, s.sky
    );
    commands.insert_resource(grid);
    *state = NavState::Built;
    *installed = InstalledPathfinder::install_level();
}

fn overlay_key(keys: Option<Res<ButtonInput<KeyCode>>>, mut overlay: ResMut<NavOverlay>) {
    if keys.is_some_and(|k| k.just_pressed(OVERLAY_KEY)) {
        overlay.on = !overlay.on;
    }
}

/// The colour of a floor cell: by layer and clearance.
pub fn cell_colour(layer: Layer, clearance: u8) -> Color {
    let share = (f32::from(clearance) / PASSABLE as f32).min(1.0);
    match (layer, u32::from(clearance) <= PASSABLE) {
        (Layer::Lower, true) => Color::srgb(share, 1.0, 0.0),
        (Layer::Lower, false) => Color::srgb(1.0, 0.0, 0.0),
        (Layer::Upper, true) => Color::srgb(0.0, 1.0 - share, 1.0),
        (Layer::Upper, false) => Color::srgb(1.0, 0.0, 1.0),
    }
}

/// The first floor the ray from `origin` along `dir` (unit) passes down
/// through, within `reach` metres.
pub fn floor_along(grid: &NavGrid, origin: Vec3, dir: Vec3, reach: f32) -> Option<Vec3> {
    let step = CELL * 0.5;
    let mut before = origin;
    for k in 1..=(reach / step) as usize {
        let p = origin + dir * (k as f32 * step);
        if let Some((x, z)) = grid.cell_at(p) {
            for layer in Layer::BOTH {
                let c = grid.cell(x, z, layer);
                if c.has_floor() {
                    let h = grid.floor_height(c.floor);
                    if before.y >= h && p.y <= h {
                        return Some(Vec3::new(p.x, h, p.z));
                    }
                }
            }
        }
        before = p;
    }
    None
}

fn draw_overlay(
    grid: Option<Res<NavGrid>>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    mut gizmos: Gizmos,
) {
    let (Some(grid), Some(camera)) = (grid, cameras.iter().next()) else {
        return;
    };
    let Some(centre) = floor_along(
        &grid,
        camera.translation(),
        camera.forward().as_vec3(),
        LOOK_REACH,
    ) else {
        return;
    };
    let (w, d) = grid.size();
    let reach = (OVERLAY_RADIUS / CELL).ceil() as i64;
    let Some((cx, cz)) = grid.cell_at(centre) else {
        return;
    };
    let half = CELL * 0.5 - MARK_INSET;
    for z in (cz as i64 - reach).max(0)..=(cz as i64 + reach).min(d as i64 - 1) {
        for x in (cx as i64 - reach).max(0)..=(cx as i64 + reach).min(w as i64 - 1) {
            let (x, z) = (x as usize, z as usize);
            let at = grid.cell_centre(x, z);
            if at.distance(centre.xz()) > OVERLAY_RADIUS {
                continue;
            }
            for layer in Layer::BOTH {
                let c = grid.cell(x, z, layer);
                if !c.has_floor() {
                    continue;
                }
                let y = grid.floor_height(c.floor) + MARK_LIFT;
                gizmos.rect(
                    Isometry3d::new(
                        Vec3::new(at.x, y, at.y),
                        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                    ),
                    Vec2::splat(half * 2.0),
                    cell_colour(layer, c.clearance),
                );
            }
        }
    }
}
