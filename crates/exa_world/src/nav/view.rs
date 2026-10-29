//! The grid in the game: built for each level once its nodes have world
//! transforms (on the async compute pool: a large level's columns take a
//! second), kept as the [`NavGrid`] resource, and drawn on demand
//! (`--debug-nav`, F6): every floor cell within 8 m of where the camera
//! looks, a square at its floor's height, green through yellow as its
//! clearance rises to what a walker passes, red beyond; the upper layer in
//! cyan through blue, magenta beyond.
//!
//! **The refresh.** Once per fixed tick, after the bodies have stepped (the
//! original refreshes once per pass of its physics thread, after every
//! simulation has stepped), the grid's queued rectangles are refreshed
//! ([`super::refresh_movers`]) from the bodies under the level's moving
//! meshes, all of them in that one call. A rectangle comes from
//! [`NavMoved`]: the original queues one when an item the cursor lifted is
//! let go (at the frame the lift took it from), and nothing else in play
//! queues one, so a door swung open or a crate knocked aside keeps the
//! floors the build gave until a refresh near it runs.
//! `--nav-refresh-moved` ([`NavRefreshMoved`], Exarust's debug option, not
//! the original's) also queues a moving mesh whose node has come to rest
//! somewhere new, so a door opened by the hand frees its doorway.

use std::collections::HashMap;
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
            .init_resource::<NavRefreshMoved>()
            .add_message::<NavMoved>()
            .add_systems(
                PostUpdate,
                (collect_source, finish_build)
                    .chain()
                    .after(TransformSystems::Propagate),
            )
            .add_systems(
                FixedPostUpdate,
                (
                    watch_moving_meshes.run_if(|o: Res<NavRefreshMoved>| o.0),
                    refresh_grid,
                )
                    .chain(),
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

/// A mesh node that carries a rigid body. The original's body attach sets
/// the node's second flag word's `0x10`, and the grid build stamps no point
/// of such a mesh unless the node is level geometry (`0x80000`, a prop
/// placed static: `stamped`). Every such mesh's points raise the floors
/// under it once the layers are written ([`super::raise_from_movers`]):
/// doors, gates, levers, chains, sacks, barrels and carts.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MovingMesh {
    /// The node is level geometry, so its points are stamped too.
    pub stamped: bool,
}

/// A node whose box should be refreshed on the grid: its world frame and
/// its box (centre and half extents in the node's space, metres). The
/// original's let-go of a lifted item queues the item's node at the frame
/// the lift saved, in the refresh's ring ([`super::ring_moved`]).
#[derive(Message, Debug, Clone, Copy)]
pub struct NavMoved {
    pub at: bevy::math::Affine3A,
    pub centre: Vec3,
    pub half: Vec3,
}

/// The rectangles waiting for the grid's refresh, kept with the level's
/// grid (a new grid starts with none).
#[derive(Resource, Debug, Default)]
pub struct NavMovers(pub super::Movers);

/// Exarust's debug option `--nav-refresh-moved`: a moving mesh whose node
/// comes to rest away from where the grid last saw it queues the
/// rectangles of both places, so doors and props moved in play update the
/// grid. The original queues no such body.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct NavRefreshMoved(pub bool);

/// Fixed ticks a moved node must hold still before
/// [`NavRefreshMoved`] queues it (about 0.4 s at 83 Hz).
const REST_TICKS: u32 = 32;
/// A node counts as moved when a point of its frame shifts this far,
/// metres.
const MOVED: f32 = 0.01;

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

/// Drops the last level's grid and any build still running: a level change
/// starts from [`NavState::None`] and installs the fallback pathfinder until
/// the next level's grid is built. The overlay flag is kept (a debug tool's
/// setting, not level state).
pub fn unload_level(
    commands: &mut Commands,
    state: &mut NavState,
    installed: &mut InstalledPathfinder,
) {
    commands.remove_resource::<PendingNav>();
    commands.remove_resource::<NavGrid>();
    commands.remove_resource::<NavMovers>();
    commands.remove_resource::<NavBuild>();
    *state = NavState::None;
    *installed = InstalledPathfinder::install_fallback();
}

#[allow(clippy::too_many_arguments)]
fn collect_source(
    mut commands: Commands,
    pending: Option<Res<PendingNav>>,
    children: Query<&Children>,
    points: Query<(&GiPoints, &GlobalTransform, Option<&MovingMesh>)>,
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
    commands.remove_resource::<NavMovers>();
    // A build for the previous level is dropped, which cancels it.
    commands.remove_resource::<NavBuild>();
    let mut source = NavSource {
        width: pending.width,
        depth: pending.depth,
        base: pending.base,
        ..default()
    };
    for e in std::iter::once(pending.root).chain(children.iter_descendants(pending.root)) {
        if let Ok((gi, at, moving)) = points.get(e) {
            let mesh = (gi.0.clone(), at.affine());
            if moving.is_none_or(|m| m.stamped) {
                source.meshes.push(mesh.clone());
            }
            if moving.is_some() {
                source.movers.push(mesh);
            }
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
        "navigation grid: {} × {} cells from {} meshes' points, {} meshes with bodies, {} floor meshes, {} terrain tiles",
        source.width,
        source.depth,
        source.meshes.len(),
        source.movers.len(),
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
    commands.insert_resource(NavMovers::default());
    *state = NavState::Built;
    *installed = InstalledPathfinder::install_level();
}

/// Queues the moved nodes and refreshes the grid's rectangles from the
/// bodies under the level's moving meshes. With nothing queued it touches
/// nothing.
fn refresh_grid(
    grid: Option<ResMut<NavGrid>>,
    movers: Option<ResMut<NavMovers>>,
    mut moved: MessageReader<NavMoved>,
    bodies: Query<(&GiPoints, &GlobalTransform), With<MovingMesh>>,
) {
    let (Some(mut grid), Some(mut movers)) = (grid, movers) else {
        moved.clear();
        return;
    };
    for m in moved.read() {
        let rect = super::rect_of_box(&grid, m.at, m.centre, m.half);
        super::ring_moved(&mut movers.0, &grid, rect);
    }
    if super::movers_idle(&movers.0) {
        return;
    }
    let samples: Vec<super::MoverPoints> = bodies
        .iter()
        .map(|(points, at)| super::MoverPoints {
            at: at.affine(),
            points: points.0.as_slice(),
        })
        .collect();
    super::refresh_movers(&mut grid, &mut movers.0, &samples);
}

/// Where [`watch_moving_meshes`] last saw a node: the frame the grid was
/// last refreshed for, the frame of the last tick and how many ticks it has
/// held it.
struct Watched {
    refreshed: bevy::math::Affine3A,
    last: bevy::math::Affine3A,
    still: u32,
}

/// Whether two frames of a node differ by more than [`MOVED`] at its
/// origin or one metre along an axis.
fn frames_differ(a: bevy::math::Affine3A, b: bevy::math::Affine3A) -> bool {
    let probe = [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z];
    probe
        .iter()
        .any(|&p| a.transform_point3(p).distance(b.transform_point3(p)) > MOVED)
}

/// The rectangle a node's points cover under a frame (their box across
/// the ground), or none without points.
fn points_rect(
    grid: &NavGrid,
    points: &[Vec3],
    at: bevy::math::Affine3A,
) -> Option<super::CellRect> {
    let mut world = points.iter().map(|&p| at.transform_point3(p));
    let first = world.next()?;
    let (lo, hi) = world.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p)));
    Some(super::rect_around(grid, (lo + hi) * 0.5, (hi - lo) * 0.5))
}

/// `--nav-refresh-moved`: a moving mesh whose node has held a new frame
/// for [`REST_TICKS`] queues the rectangles of where the grid last saw it
/// and of where it rests, in the pending list (not the ring, which a
/// crowd of movers would wrap).
fn watch_moving_meshes(
    grid: Option<Res<NavGrid>>,
    movers: Option<ResMut<NavMovers>>,
    bodies: Query<(Entity, &GiPoints, &GlobalTransform), With<MovingMesh>>,
    mut seen: Local<HashMap<Entity, Watched>>,
) {
    let (Some(grid), Some(mut movers)) = (grid, movers) else {
        seen.clear();
        return;
    };
    if grid.is_added() {
        seen.clear();
    }
    for (e, points, at) in &bodies {
        let now = at.affine();
        let watched = seen.entry(e).or_insert(Watched {
            refreshed: now,
            last: now,
            still: 0,
        });
        if frames_differ(now, watched.last) {
            watched.last = now;
            watched.still = 0;
            continue;
        }
        watched.still = watched.still.saturating_add(1);
        if watched.still != REST_TICKS || !frames_differ(watched.last, watched.refreshed) {
            continue;
        }
        for frame in [watched.refreshed, watched.last] {
            if let Some(rect) = points_rect(&grid, &points.0, frame) {
                super::queue_moved(&mut movers.0, rect);
            }
        }
        watched.refreshed = watched.last;
    }
    seen.retain(|e, _| bodies.contains(*e));
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

#[cfg(test)]
mod tests {
    use bevy::ecs::message::Messages;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::math::Affine3A;

    use super::*;
    use crate::nav::NavCell;

    /// A 24 × 24 grid of floor at voxel 10, with a sack's points (a
    /// column 1.05–2.45 m high) standing on cell (14, 14) raising it.
    fn yard(world: &mut World) -> (Vec3, Arc<Vec<Vec3>>) {
        let mut grid = NavGrid::empty(24, 24, 0.0);
        for z in 0..24 {
            for x in 0..24 {
                let cell = NavCell {
                    floor: 10,
                    ..NavCell::default()
                };
                grid.set_cell(x, z, Layer::Lower, cell);
            }
        }
        let points: Arc<Vec<Vec3>> = Arc::new(
            (0..15)
                .map(|k| Vec3::new(0.0, 1.05 + 0.1 * k as f32, 0.0))
                .collect(),
        );
        let c = grid.cell_centre(14, 14);
        let at = Vec3::new(c.x, 0.0, c.y);
        super::super::raise_from_movers(&mut grid, points.iter().map(|&p| p + at));
        assert_eq!(grid.cell(14, 14, Layer::Lower).floor, 25);
        world.insert_resource(grid);
        world.insert_resource(NavMovers::default());
        world.init_resource::<Messages<NavMoved>>();
        (at, points)
    }

    /// An item let go queues its box at the frame its lift took it from:
    /// the refresh puts that floor back and raises it where the item rests.
    #[test]
    fn a_let_go_refreshes_the_floor_the_item_left() {
        let mut world = World::new();
        let (at, points) = yard(&mut world);
        let rests = at + Vec3::new(0.6, 0.0, 0.0);
        world.spawn((
            GiPoints(points),
            GlobalTransform::from_translation(rests),
            MovingMesh::default(),
        ));
        world.run_system_once(refresh_grid).unwrap();
        assert_eq!(
            world.resource::<NavGrid>().cell(14, 14, Layer::Lower).floor,
            25,
            "nothing queued, nothing refreshed"
        );
        world.write_message(NavMoved {
            at: Affine3A::from_translation(at),
            centre: Vec3::new(0.0, 1.75, 0.0),
            half: Vec3::new(0.05, 0.7, 0.05),
        });
        world.run_system_once(refresh_grid).unwrap();
        let grid = world.resource::<NavGrid>();
        assert_eq!(grid.cell(14, 14, Layer::Lower).floor, 10);
        assert_eq!(grid.cell(16, 14, Layer::Lower).floor, 25);
        assert!(super::super::movers_idle(&world.resource::<NavMovers>().0));
    }

    /// `--nav-refresh-moved`: a moving mesh that has held a new frame for
    /// [`REST_TICKS`] queues where it was and where it rests; one still
    /// moving queues nothing.
    #[test]
    fn a_mesh_come_to_rest_is_queued_with_the_debug_option() {
        let mut world = World::new();
        let (at, points) = yard(&mut world);
        let leaf = world
            .spawn((
                GiPoints(points),
                GlobalTransform::from_translation(at),
                MovingMesh::default(),
            ))
            .id();
        // One system, so its record of the nodes lasts between ticks.
        let watch = world.register_system(watch_moving_meshes);
        world.run_system(watch).unwrap();
        *world.get_mut::<GlobalTransform>(leaf).unwrap() =
            GlobalTransform::from_translation(at + Vec3::new(0.6, 0.0, 0.0));
        for _ in 0..REST_TICKS {
            world.run_system(watch).unwrap();
            assert!(super::super::movers_idle(&world.resource::<NavMovers>().0));
        }
        world.run_system(watch).unwrap();
        assert!(!super::super::movers_idle(&world.resource::<NavMovers>().0));
        world.run_system_once(refresh_grid).unwrap();
        let grid = world.resource::<NavGrid>();
        assert_eq!(grid.cell(14, 14, Layer::Lower).floor, 10);
        assert_eq!(grid.cell(16, 14, Layer::Lower).floor, 25);
    }
}
