//! Physics: a fixed-step rapier world, static colliders built from level
//! geometry, and kinematic character bodies.
//!
//! This is the collision foundation. The articulated, motor-driven character
//! body of the original (`docs/PLAN.md`, Phase 3) builds on it.

pub mod body;

use bevy::prelude::*;
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
            .insert_resource(Time::<Fixed>::from_hz(STEP_HZ))
            .add_systems(Startup, limit_backlog)
            .add_systems(
                PostUpdate,
                build_static_colliders.after(TransformSystems::Propagate),
            )
            .add_systems(
                FixedUpdate,
                (register_characters, move_characters, step_world).chain(),
            )
            .add_systems(
                RunFixedMainLoop,
                show_characters.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
            )
            .add_message::<body::Shove>()
            .add_systems(Update, release_characters)
            .add_systems(Update, body::step_ragdolls.in_set(body::BodySystems));
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

/// Marks a subtree whose meshes become static triangle-mesh colliders once
/// their world transforms are known. Removed after the colliders are built.
#[derive(Component)]
pub struct StaticGeometry;

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
    /// Where the last fixed step left the feet, and where the step before
    /// left them: the drawn position lies between the two.
    steps: Option<(Vec3, Vec3)>,
    /// The position last drawn; a transform moved elsewhere (a teleport)
    /// replaces the simulated one.
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
            steps: None,
            shown: Vec3::ZERO,
        }
    }
}

impl CharacterBody {
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

fn build_static_colliders(
    mut commands: Commands,
    mut physics: ResMut<Physics>,
    roots: Query<Entity, With<StaticGeometry>>,
    children: Query<&Children>,
    shapes: Query<(&Mesh3d, &GlobalTransform)>,
    meshes: Res<Assets<Mesh>>,
) {
    for root in &roots {
        let mut built = 0;
        for e in children.iter_descendants(root) {
            let Ok((mesh, global)) = shapes.get(e) else {
                continue;
            };
            let Some(mesh) = meshes.get(&mesh.0) else {
                continue;
            };
            if let Some(collider) = trimesh_collider(mesh, global) {
                physics.0.insert_collider(collider, None);
                built += 1;
            }
        }
        debug!("static colliders built: {built}");
        commands.entity(root).remove::<StaticGeometry>();
    }
}

fn trimesh_collider(mesh: &Mesh, global: &GlobalTransform) -> Option<Collider> {
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
    let indices: Vec<u32> = mesh.indices()?.iter().map(|i| i as u32).collect();
    if indices.len() < 3 {
        return None;
    }
    let m = global.to_matrix();
    let vertices: Vec<Vec3> = positions
        .iter()
        .map(|p| m.transform_point3(Vec3::from(*p)))
        .collect();
    let triangles: Vec<[u32; 3]> = indices
        .as_chunks::<3>()
        .0
        .iter()
        .copied()
        .filter(|t| t.iter().all(|&i| (i as usize) < vertices.len()))
        .collect();
    ColliderBuilder::trimesh(vertices, triangles)
        .ok()
        .map(|b| b.build())
}

/// Kinematic targets computed this step, applied before the world steps.
#[derive(Resource, Default)]
pub struct PendingKinematics(pub Vec<(RigidBodyHandle, Vec3)>);

fn move_characters(
    time: Res<Time>,
    physics: Res<Physics>,
    mut bodies: Query<(&mut CharacterBody, &Transform, Option<&CharacterHandles>)>,
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
    for (mut body, transform, handles) in &mut bodies {
        let start = body.simulated(transform);
        // A character standing still on the ground stays where it is: no
        // sweep needed.
        if body.grounded && body.desired_velocity == Vec3::ZERO {
            body.stepped(start, start);
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
        let pose = Pose::from_translation(start + Vec3::Y * centre_offset);
        body.vertical_velocity = if body.grounded {
            GRAVITY * dt
        } else {
            (body.vertical_velocity + GRAVITY * dt).max(-50.0)
        };
        let desired = body.desired_velocity * dt + Vec3::Y * body.vertical_velocity * dt;
        let moved = controller.move_shape(dt, &queries, &shape, &pose, desired, |_| {});
        let end = start + moved.translation;
        body.stepped(start, end);
        body.grounded = moved.grounded;
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
