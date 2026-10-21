//! Physics: a fixed-step rapier world, static colliders built from level
//! geometry, and kinematic character bodies.
//!
//! This is the collision foundation. The articulated, motor-driven character
//! body of the original (`docs/PLAN.md`, Phase 3) builds on it.

pub mod body;
pub mod cloth;
pub mod contact;
pub mod door;
pub mod dynamics;
pub mod intersect;
pub mod lever;
pub mod mechanism;
pub mod motile;
pub mod part;
pub mod plate;
pub mod prop;
pub mod relay;
pub mod sector;
pub mod shape;
pub mod signal;
pub mod volume;
pub mod voxel;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use rapier3d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use rapier3d::prelude::*;

/// Simulation rate of the fixed step: the original's 83 Hz (12 ms) time-
/// critical thread.
pub const STEP_HZ: f64 = 83.0;
/// Steps a slow frame may catch up on; the original drops the rest of its
/// backlog (game time slows instead of spiralling).
pub const MAX_STEPS_BEHIND: f64 = 4.0;
const GRAVITY: f32 = -9.81;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        let mut world = PhysicsWorld::new();
        world.gravity = Vec3::new(0.0, GRAVITY, 0.0);
        world.integration_parameters.dt = (1.0 / STEP_HZ) as f32;
        app.insert_resource(Physics(world))
            .init_resource::<PendingKinematics>()
            .init_resource::<StaticBuild>()
            .insert_resource(Time::<Fixed>::from_hz(STEP_HZ))
            .add_systems(Startup, limit_backlog)
            .add_systems(
                PostUpdate,
                build_static_colliders.after(TransformSystems::Propagate),
            )
            // Each tick moves the character controllers first, before the
            // pre-step; the bodies' character ticks then read where they are.
            .add_systems(
                FixedUpdate,
                (register_characters, move_characters, step_world)
                    .chain()
                    .before(body::BodyPreStep),
            )
            .add_systems(
                RunFixedMainLoop,
                show_characters.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
            )
            .add_systems(Update, release_characters)
            .add_plugins((body::BodyPlugin, prop::PropPlugin));
    }
}

/// Caps the game time a frame may advance at four fixed steps.
fn limit_backlog(mut time: ResMut<Time<Virtual>>) {
    time.set_max_delta(std::time::Duration::from_secs_f64(
        MAX_STEPS_BEHIND / STEP_HZ,
    ));
}

#[derive(Resource)]
pub struct Physics(pub PhysicsWorld);

impl Default for Physics {
    /// An empty world.
    fn default() -> Self {
        Self(PhysicsWorld::new())
    }
}

impl Physics {
    /// How far along `direction` (unit) from `origin` the first fixed
    /// collider lies, within `max` metres.
    pub fn fixed_hit(&self, origin: Vec3, direction: Vec3, max: f32) -> Option<f32> {
        let ray = Ray::new(origin, direction);
        self.0
            .cast_ray(&ray, max, true, QueryFilter::only_fixed())
            .map(|(_, toi)| toi)
    }

    /// The first fixed collider along `direction` (unit) from `origin`
    /// within `max` metres, and how far along it lies: a prop's stand-in
    /// ([`prop::StandIns::owner_of`] names its body) or the level.
    pub fn fixed_pick(
        &self,
        origin: Vec3,
        direction: Vec3,
        max: f32,
    ) -> Option<(ColliderHandle, f32)> {
        let ray = Ray::new(origin, direction);
        self.0.cast_ray(&ray, max, true, QueryFilter::only_fixed())
    }

    /// How far along `direction` (unit) from `origin` a ball of `radius`
    /// first meets the level (fixed colliders but the props' stand-ins),
    /// within `max` metres.
    pub fn level_sweep(&self, origin: Vec3, direction: Vec3, max: f32, radius: f32) -> Option<f32> {
        let ball = Ball::new(radius);
        let at = Pose::from_translation(origin);
        let not_props = |_: ColliderHandle, c: &Collider| c.user_data != prop::STAND_IN;
        let filter = QueryFilter::only_fixed().predicate(&not_props);
        self.0
            .cast_shape(
                &at,
                direction,
                &ball,
                rapier3d::parry::query::ShapeCastOptions::with_max_time_of_impact(max),
                filter,
            )
            .map(|(_, hit)| hit.time_of_impact)
    }
}

/// Marks a subtree whose meshes become static triangle-mesh colliders once
/// their world transforms are known. Removed after the colliders are built,
/// which for a large level takes several frames ([`StaticBuild`]). The
/// meshes a rigid body moves (under a [`prop::BodyNode`]) become its
/// stand-ins instead ([`prop::StandIns`]).
#[derive(Component)]
pub struct StaticGeometry;

/// A mesh under a [`StaticGeometry`] root that gets no triangle-mesh
/// collider: terrain vegetation, which the original draws but never collides
/// with. Its subtree is skipped whole.
#[derive(Component)]
pub struct NoStaticCollider;

/// The static colliders being built. A level's meshes are copied a batch at
/// a time, within [`STATIC_BUILD_BUDGET`] per frame, and built into
/// triangle-mesh colliders on the async compute pool: one large mesh (a
/// terrain tile) takes tens of milliseconds to build, a level's thousands
/// of meshes seconds, and the window must keep answering meanwhile.
#[derive(Resource, Default)]
pub struct StaticBuild {
    roots: Vec<RootBuild>,
    built: usize,
    total: usize,
}

/// One root's meshes, how many are handed to builds, and the builds in
/// flight with the number of meshes each covers.
struct RootBuild {
    root: Entity,
    parts: Vec<Entity>,
    handed: usize,
    tasks: Vec<(usize, Task<Vec<BuiltCollider>>)>,
}

/// A built collider and, for a rigid body's stand-in, the body's root node
/// and the part it follows.
type BuiltCollider = (Collider, Option<(Entity, usize)>);

impl StaticBuild {
    /// The share of the meshes queued since the last idle moment that are
    /// built (1 when nothing is being built).
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            1.0
        } else {
            self.built as f32 / self.total as f32
        }
    }
}

/// Time one frame may spend handing meshes to collider builds.
pub const STATIC_BUILD_BUDGET: std::time::Duration = std::time::Duration::from_millis(4);
/// Triangles a collider build takes at most, unless one mesh has more.
const BUILD_TRIANGLES: usize = 60_000;
/// Builds in flight at once; more wait, so the pool stays free for others.
const BUILDS_IN_FLIGHT: usize = 4;

/// The share of the way to its follow point ([`CharacterBody::follow`]) a
/// character's feet go in a fixed step, at most [`FOLLOW_MOST`]: a body's
/// frame moves in steps (5 and 10 cm dead-bands, a get-up's re-anchoring
/// by up to a metre), and the character, the player's position, glides
/// after it.
pub const FOLLOW_SHARE: f32 = 0.1;
/// The farthest (m) a character's feet follow in a fixed step: 2 cm, about
/// a brisk walk (1.66 m/s).
pub const FOLLOW_MOST: f32 = 0.02;
/// A follow point closer than this (m) across the ground is not followed.
const FOLLOW_MIN: f32 = 0.001;

/// A kinematic capsule moved by the character controller.
#[derive(Component, Debug, Clone)]
pub struct CharacterBody {
    pub radius: f32,
    /// Distance between the capsule's sphere centres.
    pub half_height: f32,
    /// Desired horizontal velocity (m/s), set by input or AI.
    pub desired_velocity: Vec3,
    pub vertical_velocity: f32,
    pub grounded: bool,
    /// A place across the ground the next fixed step draws the feet toward,
    /// [`FOLLOW_SHARE`] of the way (at most [`FOLLOW_MOST`]), through the
    /// controller, before the character's own walk: its
    /// body's frame, which a living character follows (Exarust's own: the
    /// original has no capsule). Taken by the step.
    pub follow: Option<Vec3>,
    /// How far the last step's follow moved the feet (m): the body, which
    /// is carried with its character, has made that move already.
    followed: Vec3,
    /// Where the last fixed step left the feet, and where the step before
    /// left them: the drawn position lies between the two.
    steps: Option<(Vec3, Vec3)>,
    /// The position last drawn; a transform moved elsewhere (a teleport)
    /// replaces the simulated one. Inside the fixed step the transform holds
    /// the simulated position, so that what is carried on the character
    /// (its body) reads where it really is.
    shown: Vec3,
}

impl Default for CharacterBody {
    fn default() -> Self {
        Self {
            radius: 0.28,
            half_height: 0.62,
            desired_velocity: Vec3::ZERO,
            vertical_velocity: 0.0,
            grounded: false,
            follow: None,
            followed: Vec3::ZERO,
            steps: None,
            shown: Vec3::ZERO,
        }
    }
}

impl CharacterBody {
    /// How far the last fixed step's follow moved the feet (m).
    pub fn followed(&self) -> Vec3 {
        self.followed
    }

    /// How far the character walked in the last fixed step (m): its move
    /// less the follow.
    pub fn walked(&self) -> Vec3 {
        self.steps
            .map_or(Vec3::ZERO, |(now, before)| now - before - self.followed)
    }

    /// Where the character's feet are in the simulation (m, world), given
    /// its entity's transform: what the next fixed step starts from (the
    /// transform shows a drawn position between steps; one moved elsewhere
    /// is a teleport, and wins).
    pub fn feet(&self, transform: &Transform) -> Vec3 {
        self.simulated(transform)
    }

    /// The simulated feet position, given the entity's transform.
    fn simulated(&self, transform: &Transform) -> Vec3 {
        match self.steps {
            Some((now, _)) if transform.translation == self.shown => now,
            _ => transform.translation,
        }
    }

    /// Records a fixed step that moved the feet from `from` to `to`.
    fn stepped(&mut self, from: Vec3, to: Vec3) {
        self.steps = Some((to, from));
    }

    /// The feet position to draw `alpha` of the way into the next step.
    fn drawn(&self, alpha: f32) -> Option<Vec3> {
        self.steps.map(|(now, before)| before.lerp(now, alpha))
    }
}

/// The character's own body and capsule in the physics world, so other
/// characters collide with it.
#[derive(Component, Debug, Clone, Copy)]
pub struct CharacterHandles {
    pub body: RigidBodyHandle,
    pub collider: ColliderHandle,
}

fn register_characters(
    mut commands: Commands,
    mut physics: ResMut<Physics>,
    new: Query<(Entity, &CharacterBody, &Transform), Without<CharacterHandles>>,
) {
    for (e, body, t) in &new {
        let centre = t.translation + Vec3::Y * (body.half_height + body.radius);
        let rb = RigidBodyBuilder::kinematic_position_based()
            .translation(centre)
            .build();
        let handle = physics.0.insert_body(rb);
        let collider = ColliderBuilder::capsule_y(body.half_height, body.radius).build();
        let collider = physics.0.insert_collider(collider, Some(handle));
        commands.entity(e).insert(CharacterHandles {
            body: handle,
            collider,
        });
    }
}

/// Removes the capsules of characters that stopped being controlled (a
/// body gone limp keeps no capsule).
fn release_characters(
    mut commands: Commands,
    mut physics: ResMut<Physics>,
    released: Query<(Entity, &CharacterHandles), Without<CharacterBody>>,
) {
    for (e, handles) in &released {
        physics.0.remove_body_with_colliders(handles.body, true);
        commands.entity(e).remove::<CharacterHandles>();
    }
}

#[allow(clippy::too_many_arguments)]
fn build_static_colliders(
    mut commands: Commands,
    mut physics: ResMut<Physics>,
    mut build: ResMut<StaticBuild>,
    mut stand_ins: ResMut<prop::StandIns>,
    roots: Query<Entity, With<StaticGeometry>>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    body_nodes: Query<&prop::BodyNode>,
    shapes: Query<(&Mesh3d, &GlobalTransform)>,
    no_collider: Query<(), With<NoStaticCollider>>,
    meshes: Res<Assets<Mesh>>,
) {
    for root in &roots {
        if build.roots.iter().any(|r| r.root == root) {
            continue;
        }
        let parts: Vec<Entity> = children
            .iter_descendants(root)
            .filter(|&e| shapes.contains(e) && !no_collider.contains(e))
            .collect();
        build.total += parts.len();
        build.roots.push(RootBuild {
            root,
            parts,
            handed: 0,
            tasks: Vec::new(),
        });
    }
    let started = std::time::Instant::now();
    let build = &mut *build;
    // Finished builds join the world.
    for root in &mut build.roots {
        root.tasks.retain_mut(|(count, task)| {
            let Some(colliders) = block_on(poll_once(task)) else {
                return true;
            };
            for (mut collider, owner) in colliders {
                if owner.is_some() {
                    collider.user_data = prop::STAND_IN;
                }
                let handle = physics.0.insert_collider(collider, None);
                if let Some((body, part)) = owner {
                    stand_ins.0.entry(body).or_default().push((handle, part));
                }
            }
            build.built += *count;
            false
        });
    }
    // New builds, a batch of copied meshes each.
    let pool = AsyncComputeTaskPool::get();
    let mut in_flight: usize = build.roots.iter().map(|r| r.tasks.len()).sum();
    for root in &mut build.roots {
        while root.handed < root.parts.len()
            && in_flight < BUILDS_IN_FLIGHT
            && started.elapsed() < STATIC_BUILD_BUDGET
        {
            let mut batch = Vec::new();
            let mut triangles = 0;
            let first = root.handed;
            while root.handed < root.parts.len()
                && (batch.is_empty() || triangles < BUILD_TRIANGLES)
            {
                let e = root.parts[root.handed];
                root.handed += 1;
                if let Ok((mesh, global)) = shapes.get(e)
                    && let Some(mut copy) =
                        meshes.get(&mesh.0).and_then(|m| MeshCopy::of(m, global))
                {
                    copy.owner = moved_by(e, root.root, &parents, &body_nodes);
                    triangles += copy.indices.len() / 3;
                    batch.push(copy);
                }
            }
            let count = root.handed - first;
            let task = pool.spawn(async move {
                batch
                    .into_iter()
                    .filter_map(|copy| {
                        let owner = copy.owner;
                        copy.collider().map(|c| (c, owner))
                    })
                    .collect()
            });
            root.tasks.push((count, task));
            in_flight += 1;
        }
    }
    build.roots.retain(|root| {
        let finished = root.handed == root.parts.len() && root.tasks.is_empty();
        if finished {
            debug!("static colliders built: {} meshes", root.parts.len());
            if roots.contains(root.root) {
                commands.entity(root.root).remove::<StaticGeometry>();
            }
        }
        !finished
    });
    if build.roots.is_empty() {
        build.built = 0;
        build.total = 0;
    }
}

/// The rigid body (root node and part) that moves the mesh `e`: the
/// nearest node over it, up to the static root, that a rigid body moves.
fn moved_by(
    e: Entity,
    root: Entity,
    parents: &Query<&ChildOf>,
    body_nodes: &Query<&prop::BodyNode>,
) -> Option<(Entity, usize)> {
    let mut node = Some(e);
    while let Some(n) = node.filter(|&n| n != root) {
        if let Ok(b) = body_nodes.get(n) {
            return Some((b.body, b.part));
        }
        node = parents.get(n).ok().map(ChildOf::parent);
    }
    None
}

/// A mesh's positions and triangles with its world transform, owned so a
/// collider can be built from it on another thread, and the rigid body
/// that moves it, if any.
struct MeshCopy {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
    transform: Mat4,
    owner: Option<(Entity, usize)>,
}

impl MeshCopy {
    fn of(mesh: &Mesh, global: &GlobalTransform) -> Option<Self> {
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
        let indices: Vec<u32> = mesh.indices()?.iter().map(|i| i as u32).collect();
        (indices.len() >= 3).then(|| Self {
            positions: positions.to_vec(),
            indices,
            transform: global.to_matrix(),
            owner: None,
        })
    }

    /// The mesh as a static triangle-mesh collider in world space.
    fn collider(self) -> Option<Collider> {
        let m = self.transform;
        let vertices: Vec<Vec3> = self
            .positions
            .iter()
            .map(|p| m.transform_point3(Vec3::from(*p)))
            .collect();
        // Bodies collide with the front of a face only, the side its winding
        // faces in the mesh's own space; a mirrored placement keeps it there.
        let mirrored = m.determinant() < 0.0;
        let triangles: Vec<[u32; 3]> = self
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .copied()
            .filter(|t| t.iter().all(|&i| (i as usize) < vertices.len()))
            .map(|[a, b, c]| if mirrored { [a, c, b] } else { [a, b, c] })
            .collect();
        ColliderBuilder::trimesh(vertices, triangles)
            .ok()
            .map(|b| b.build())
    }
}

#[cfg(test)]
fn trimesh_collider(mesh: &Mesh, global: &GlobalTransform) -> Option<Collider> {
    MeshCopy::of(mesh, global)?.collider()
}

/// Kinematic targets computed this step, applied before the world steps.
#[derive(Resource, Default)]
pub struct PendingKinematics(pub Vec<(RigidBodyHandle, Vec3)>);

fn move_characters(
    time: Res<Time>,
    physics: Res<Physics>,
    mut bodies: Query<(
        &mut CharacterBody,
        &mut Transform,
        Option<&CharacterHandles>,
    )>,
    mut moves: Local<Vec<(RigidBodyHandle, Vec3)>>,
    mut physics_mut: ResMut<PendingKinematics>,
) {
    let dt = time.delta_secs();
    let controller = KinematicCharacterController {
        up: Vec3::Y,
        offset: CharacterLength::Absolute(0.02),
        slide: true,
        autostep: Some(CharacterAutostep {
            max_height: CharacterLength::Absolute(0.35),
            min_width: CharacterLength::Absolute(0.2),
            include_dynamic_bodies: false,
        }),
        max_slope_climb_angle: 50f32.to_radians(),
        min_slope_slide_angle: 40f32.to_radians(),
        snap_to_ground: Some(CharacterLength::Absolute(0.3)),
        ..Default::default()
    };
    moves.clear();
    for (mut body, mut transform, handles) in &mut bodies {
        let start = body.simulated(&transform);
        body.followed = Vec3::ZERO;
        let gap = body
            .follow
            .take()
            .map_or(Vec3::ZERO, |f| (f - start).with_y(0.0));
        let following = gap.length() > FOLLOW_MIN;
        // A character standing still on the ground, with nothing to follow,
        // stays where it is: no sweep needed.
        if body.grounded && body.desired_velocity == Vec3::ZERO && !following {
            body.stepped(start, start);
            transform.translation = start;
            continue;
        }
        let filter = match handles {
            Some(h) => QueryFilter::default().exclude_rigid_body(h.body),
            None => QueryFilter::default(),
        };
        let queries = physics.0.query_pipeline_with_filter(filter);
        let shape = Capsule::new_y(body.half_height, body.radius);
        // The entity's origin is at the feet; the capsule is centred above.
        let centre_offset = body.half_height + body.radius;
        let mut at = start;
        // First the follow, across the ground, through the controller: it
        // is stopped by walls and steps up as a walk is.
        if following {
            let pose = Pose::from_translation(at + Vec3::Y * centre_offset);
            let step = (gap * FOLLOW_SHARE).clamp_length_max(FOLLOW_MOST);
            let moved = controller.move_shape(dt, &queries, &shape, &pose, step, |_| {});
            at += moved.translation;
            body.followed = moved.translation;
            body.grounded = moved.grounded;
        }
        let end = if body.grounded && body.desired_velocity == Vec3::ZERO {
            at
        } else {
            let pose = Pose::from_translation(at + Vec3::Y * centre_offset);
            body.vertical_velocity = if body.grounded {
                GRAVITY * dt
            } else {
                (body.vertical_velocity + GRAVITY * dt).max(-50.0)
            };
            let desired = body.desired_velocity * dt + Vec3::Y * body.vertical_velocity * dt;
            let moved = controller.move_shape(dt, &queries, &shape, &pose, desired, |_| {});
            body.grounded = moved.grounded;
            at + moved.translation
        };
        body.stepped(start, end);
        transform.translation = end;
        if body.grounded {
            body.vertical_velocity = 0.0;
        }
        if let Some(h) = handles {
            moves.push((h.body, end + Vec3::Y * centre_offset));
        }
    }
    physics_mut.0.clone_from(&moves);
}

/// Draws characters between their last two fixed steps, so they move
/// smoothly at any frame rate while the simulation keeps its fixed rate.
fn show_characters(
    time: Res<Time<Fixed>>,
    mut bodies: Query<(&mut CharacterBody, &mut Transform)>,
) {
    let alpha = time.overstep_fraction();
    for (mut body, mut transform) in &mut bodies {
        if let Some(at) = body.drawn(alpha) {
            transform.translation = at;
            body.shown = at;
        }
    }
}

fn step_world(mut physics: ResMut<Physics>, mut pending: ResMut<PendingKinematics>) {
    for (handle, centre) in pending.0.drain(..) {
        if let Some(rb) = physics.0.bodies.get_mut(handle) {
            rb.set_next_kinematic_translation(centre);
        }
    }
    physics.0.step();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ball swept up meets a level ceiling a radius short of it, passes
    /// through a prop's stand-in, and meets nothing beyond its reach.
    #[test]
    fn a_swept_ball_meets_the_level_but_not_the_props() {
        let mut world = PhysicsWorld::new();
        world.insert_collider(
            ColliderBuilder::cuboid(5.0, 0.1, 5.0)
                .translation(Vec3::new(0.0, 2.1, 0.0))
                .build(),
            None,
        );
        world.insert_collider(
            ColliderBuilder::cuboid(0.2, 0.2, 0.2)
                .translation(Vec3::new(0.0, 1.0, 0.0))
                .user_data(prop::STAND_IN)
                .build(),
            None,
        );
        world.step();
        let physics = Physics(world);
        let t = physics.level_sweep(Vec3::ZERO, Vec3::Y, 3.0, 0.1).unwrap();
        assert!((t - 1.9).abs() < 1e-3, "{t}");
        assert_eq!(physics.level_sweep(Vec3::ZERO, Vec3::Y, 1.5, 0.1), None);
        assert!(physics.fixed_hit(Vec3::ZERO, Vec3::Y, 3.0).unwrap() < 1.0);
    }

    /// A mirrored placement of a level mesh keeps its faces' front where
    /// the mesh's own winding puts it.
    #[test]
    fn mirrored_level_meshes_keep_their_front() {
        use crate::contact::Level;
        let mesh = Plane3d::default().mesh().size(2.0, 2.0).build();
        for scale in [Vec3::ONE, Vec3::new(1.0, 1.0, -1.0)] {
            let at = GlobalTransform::from(Transform::from_scale(scale));
            let mut world = PhysicsWorld::new();
            world.insert_collider(trimesh_collider(&mesh, &at).unwrap(), None);
            world.step();
            let (mut pieces, mut tris) = (Vec::new(), Vec::new());
            world.pieces(Vec3::splat(-1.0), Vec3::splat(1.0), &mut pieces);
            world.triangles(pieces[0], Vec3::splat(-1.0), Vec3::splat(1.0), &mut tris);
            for t in tris {
                let n = (t[1] - t[0]).cross(t[2] - t[0]);
                assert!(n.y > 0.0, "scale {scale}: a face turned down");
            }
        }
    }

    #[test]
    fn static_colliders_are_built_then_the_marker_goes() {
        let mut app = App::new();
        app.add_plugins(TaskPoolPlugin::default())
            .insert_resource(Physics(PhysicsWorld::new()))
            .init_resource::<StaticBuild>()
            .init_resource::<prop::StandIns>()
            .init_resource::<Assets<Mesh>>()
            .add_systems(Update, build_static_colliders);
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Plane3d::default().mesh().size(2.0, 2.0));
        let root = app
            .world_mut()
            .spawn((StaticGeometry, Transform::default()))
            .id();
        for i in 0..40 {
            app.world_mut().spawn((
                Mesh3d(mesh.clone()),
                GlobalTransform::from_xyz(i as f32 * 3.0, 0.0, 0.0),
                ChildOf(root),
            ));
        }
        // The colliders are built on another thread.
        for _ in 0..2000 {
            if !app.world().entity(root).contains::<StaticGeometry>() {
                break;
            }
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!app.world().entity(root).contains::<StaticGeometry>());
        assert_eq!(app.world().resource::<Physics>().0.colliders.len(), 40);
        assert_eq!(app.world().resource::<StaticBuild>().fraction(), 1.0);
    }

    /// A standing character drawn toward a follow point goes a tenth of the
    /// way each step, but no more than 2 cm, through the controller, and
    /// says how far it went (its body has made that move already); its
    /// body's frame sets the point again each tick. Half a metre away it
    /// goes 2 cm a step until it is 20 cm away, then a tenth of the rest. A
    /// point within a millimetre is not followed, and a point is followed
    /// only in the step after it was set.
    #[test]
    fn a_character_glides_after_its_follow_point() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(1.0 / STEP_HZ),
            ))
            .init_resource::<PendingKinematics>()
            .add_systems(Update, move_characters);
        let mesh = Plane3d::default().mesh().size(40.0, 40.0).build();
        let mut world = PhysicsWorld::new();
        world.insert_collider(
            trimesh_collider(&mesh, &GlobalTransform::default()).unwrap(),
            None,
        );
        world.step();
        app.insert_resource(Physics(world));
        let character = app
            .world_mut()
            .spawn((
                Transform::from_xyz(5.0, 0.02, -3.0),
                CharacterBody {
                    grounded: true,
                    ..default()
                },
            ))
            .id();
        // Away from the floor's diagonal, whose edge the controller's step
        // would catch.
        let goal = Vec3::new(5.3, 5.0, -3.4);
        let mut gap = 0.5;
        for step in 0..40 {
            app.world_mut()
                .get_mut::<CharacterBody>(character)
                .unwrap()
                .follow = Some(goal);
            app.update();
            let world = app.world();
            let body = world.get::<CharacterBody>(character).unwrap();
            let at = world.get::<Transform>(character).unwrap().translation;
            let left = (goal - at).xz().length();
            let went = (gap * FOLLOW_SHARE).min(FOLLOW_MOST);
            assert!(
                (left - (gap - went)).abs() < 2e-4,
                "step {step}: {left} m left of {gap}"
            );
            assert!((body.followed().xz().length() - went).abs() < 2e-4);
            if step < 15 {
                assert!((went - 0.02).abs() < 1e-6, "step {step}: {went}");
            }
            assert!(body.follow.is_none());
            gap = left;
        }
        // Unset, nothing is followed; within a millimetre, nothing either.
        app.update();
        assert_eq!(
            app.world()
                .get::<CharacterBody>(character)
                .unwrap()
                .followed(),
            Vec3::ZERO
        );
        let at = app.world().get::<Transform>(character).unwrap().translation;
        app.world_mut()
            .get_mut::<CharacterBody>(character)
            .unwrap()
            .follow = Some(at + Vec3::X * 0.0009);
        app.update();
        let world = app.world();
        assert_eq!(world.get::<Transform>(character).unwrap().translation, at);
        assert_eq!(
            world.get::<CharacterBody>(character).unwrap().followed(),
            Vec3::ZERO
        );
    }

    /// The meshes under a node a rigid body moves are built as its
    /// stand-ins: tagged movable and listed for the body; the level's own
    /// stay static.
    #[test]
    fn meshes_of_rigid_bodies_become_stand_ins() {
        use crate::contact::Level;
        let mut app = App::new();
        app.add_plugins(TaskPoolPlugin::default())
            .insert_resource(Physics(PhysicsWorld::new()))
            .init_resource::<StaticBuild>()
            .init_resource::<prop::StandIns>()
            .init_resource::<Assets<Mesh>>()
            .add_systems(Update, build_static_colliders);
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Plane3d::default().mesh().size(2.0, 2.0));
        let root = app
            .world_mut()
            .spawn((StaticGeometry, Transform::default()))
            .id();
        app.world_mut().spawn((
            Mesh3d(mesh.clone()),
            GlobalTransform::IDENTITY,
            ChildOf(root),
        ));
        let node = app.world_mut().spawn(ChildOf(root)).id();
        app.world_mut().entity_mut(node).insert(prop::BodyNode {
            body: node,
            part: 0,
        });
        app.world_mut().spawn((
            Mesh3d(mesh),
            GlobalTransform::from_xyz(0.0, 1.0, 0.0),
            ChildOf(node),
        ));
        for _ in 0..2000 {
            if !app.world().entity(root).contains::<StaticGeometry>() {
                break;
            }
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let world = &app.world().resource::<Physics>().0;
        assert_eq!(world.colliders.len(), 2);
        let stand_ins = &app.world().resource::<prop::StandIns>().0;
        assert_eq!(stand_ins.len(), 1);
        let (handle, part) = stand_ins[&node][0];
        assert_eq!(part, 0);
        assert!(world.movable(handle.into_raw_parts().0));
        let static_count = world
            .colliders
            .iter()
            .filter(|(h, _)| !world.movable(h.into_raw_parts().0))
            .count();
        assert_eq!(static_count, 1);
    }

    #[test]
    fn characters_are_drawn_between_their_steps() {
        let mut body = CharacterBody::default();
        let t = Transform::from_xyz(1.0, 0.0, 0.0);
        // No step yet: the transform is the simulated position.
        assert_eq!(body.simulated(&t), Vec3::X);
        body.stepped(Vec3::X, Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(body.drawn(0.25), Some(Vec3::new(1.25, 0.0, 0.0)));
        // While the drawn position stands, the simulation carries on from
        // the last step.
        body.shown = Vec3::new(1.25, 0.0, 0.0);
        let drawn = Transform::from_translation(body.shown);
        assert_eq!(body.simulated(&drawn), Vec3::new(2.0, 0.0, 0.0));
        // A transform moved elsewhere (a teleport) wins.
        let moved = Transform::from_xyz(9.0, 0.0, 0.0);
        assert_eq!(body.simulated(&moved), Vec3::new(9.0, 0.0, 0.0));
    }
}
