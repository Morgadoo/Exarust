//! The level's cloth: every mesh node the RFC marks as cloth (long hair,
//! banners, the wraith's robe) becomes a simulated cloth over its mesh.
//!
//! The original gives a mesh node a cloth controller when its material is
//! flagged as cloth and the node carries the cloth marker; each mesh vertex
//! is a particle, the edges of its triangles are structural links and the
//! two vertices opposite a shared edge are a bending link. The cloth hangs
//! from pins on its owner: the shipped cloth meshes carry no skin weights,
//! so every vertex is pinned toward its rest point. The pin's weight is a
//! guess [?]: the original's free-vertex list marks the vertices without a
//! fully weighted bone bind, and its loose pin weight is 0.1; unskinned
//! cloth takes that weight on every vertex, which holds the cloth near its
//! rest pose while gravity, wind and collisions sway it. A skinned cloth
//! pins the vertices with a zero weight slot and leaves the others free.
//!
//! Each 83 Hz body tick advances the cloth ([`exa_physics::cloth::step`])
//! with gravity, and the mesh's positions take the particles' (the mesh is
//! in metres, the cloth in centimetres). The simulation is local to the
//! mesh's node, so the node's own transform carries the cloth with the
//! body. Evidence: the original's cloth construction (a particle per vertex,
//! triangle edges and bend links) and its step; the pin rule is a labelled
//! guess. The draw's two passes are [`crate::cloth_draw`]'s.

use bevy::math::Affine3A;
use bevy::prelude::*;

use exa_physics::Physics;
use exa_physics::body::Ragdoll;
use exa_physics::cloth::{self, Cloth};
use exa_physics::contact::Level;
use exa_physics::sector::{self, ObjectBodyPart, ObjectBounds, ObjectContacts};

/// Marks a mesh part whose RFC node carries the cloth marker.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ClothNode;

/// A cloth node whose mesh cannot be a cloth (no positions, no triangles),
/// so the builder does not look at it again.
#[derive(Component, Debug, Clone, Copy)]
pub struct ClothSkipped;

/// The pin weight unskinned cloth takes on every vertex [guess: the
/// original's loose pin weight].
pub const LOOSE_PIN: f32 = 0.1;

/// A cloth simulated over a mesh part.
#[derive(Component, Debug)]
pub struct ClothInstance {
    pub cloth: Cloth,
    /// The mesh whose vertices the cloth drives.
    pub mesh: Handle<Mesh>,
    /// The node's world transform at the cloth's last tick: a sleeping
    /// cloth whose frame moved meets the level again.
    pub frame: Option<Affine3A>,
}

/// The sector's live simulated-object list: every cloth of the level in the
/// original's 600 cm object grid, in a stable entity order (the order the
/// original's object list keeps, oldest first). A cloth migrates to its
/// new cell after each tick.
#[derive(Resource, Debug)]
pub struct ClothObjects {
    /// The cloths, oldest entity first.
    pub order: Vec<Entity>,
    /// Their cells.
    pub grid: sector::ObjectGrid,
}

impl Default for ClothObjects {
    fn default() -> Self {
        Self::new()
    }
}

impl ClothObjects {
    /// Columns of the object grid: wide enough for a level's cells.
    pub const COLUMNS: i32 = 4096;
    /// Rows of the object grid.
    pub const ROWS: i32 = 4096;
    /// The grid's origin cell, centred on the world origin.
    pub const ORIGIN: (i32, i32) = (-2048, -2048);

    /// An empty list.
    pub fn new() -> Self {
        Self {
            order: Vec::new(),
            grid: sector::ObjectGrid::new(Self::COLUMNS, Self::ROWS, Self::ORIGIN),
        }
    }
}

/// Whether the level's cloths gather body and level contacts each tick.
/// On by default; `EXA_CLOTH_CONTACTS=0` turns them off. Their cost is the
/// original's: a cloth never meets its own node's mesh, the level never
/// wakes it and its slide stops it, so a cloth at rest sleeps within its
/// 32-tick countdown, and a sleeping cloth takes only awake bodies whose
/// padded spheres reach its box ([`sector::object_takes_body`]).
#[derive(Resource, Debug, Clone, Copy)]
pub struct ClothContacts(pub bool);

impl Default for ClothContacts {
    fn default() -> Self {
        Self(std::env::var_os("EXA_CLOTH_CONTACTS").is_none_or(|v| v != "0"))
    }
}

/// A cloth's world bound in centimetres, from its particles' local bounds
/// and its node's transform: the centre and the sphere around the box.
fn cloth_bound(cloth: &Cloth, at: &GlobalTransform) -> (ObjectBounds, Affine3A) {
    let (centre, half) = cloth::bounds(&cloth.particles);
    let scale = at.scale().abs().max_element();
    let world = at.transform_point(centre * 0.01);
    (
        ObjectBounds {
            centre: world * 100.0,
            // The particles are already in centimetres.
            radius: half.length() * scale,
        },
        at.affine().inverse(),
    )
}

/// One vertex's pin index and weight from its skin weights: a vertex with a
/// zero weight slot (or with no skin at all) is pinned, at the loose weight.
pub fn pins_from_skin(vertices: usize, skin: Option<&[[f32; 4]]>) -> Vec<(usize, f32)> {
    match skin {
        None => (0..vertices).map(|i| (i, LOOSE_PIN)).collect(),
        Some(weights) => weights
            .iter()
            .take(vertices)
            .enumerate()
            .filter(|(_, w)| w.contains(&0.0))
            .map(|(i, _)| (i, LOOSE_PIN))
            .collect(),
    }
}

/// Builds the cloth of a marked mesh from its positions (metres) and
/// triangles, pinned as [`pins_from_skin`] says.
pub fn cloth_of(vertices: &[Vec3], triangles: &[[usize; 3]], pins: &[(usize, f32)]) -> Cloth {
    let cm: Vec<Vec3> = vertices.iter().map(|v| *v * 100.0).collect();
    let indices: Vec<usize> = pins.iter().map(|(i, _)| *i).collect();
    let mut cloth = Cloth::from_mesh(&cm, triangles, &indices);
    for (pin, (_, weight)) in cloth.pins.iter_mut().zip(pins) {
        pin.weight = *weight;
    }
    cloth
}

/// Cloth nodes still waiting for their cloth: not built, not given up on.
type PendingCloth = (
    With<ClothNode>,
    Without<ClothInstance>,
    Without<ClothSkipped>,
);

/// Gives each cloth node its cloth, once its mesh is in place (a skinned
/// part gets its mesh when it is bound to its skeleton, after it spawns).
///
/// The cloth moves its mesh's vertices, and one mesh asset is shared by
/// every copy of a model, so each cloth takes a copy of the mesh for its
/// node alone.
fn build_cloth_nodes(
    mut commands: Commands,
    nodes: Query<(Entity, &Mesh3d), PendingCloth>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, mesh) in &nodes {
        let Some(mesh_data) = meshes.get(&mesh.0) else {
            continue;
        };
        let Some(positions) = mesh_data
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
        else {
            commands.entity(entity).insert(ClothSkipped);
            continue;
        };
        let Some(indices) = mesh_data.indices() else {
            commands.entity(entity).insert(ClothSkipped);
            continue;
        };
        let indices: Vec<usize> = indices.iter().collect();
        let vertices: Vec<Vec3> = positions.iter().map(|p| Vec3::from_array(*p)).collect();
        let triangles: Vec<[usize; 3]> = indices
            .as_chunks::<3>()
            .0
            .iter()
            .filter(|c| c.iter().all(|&i| i < vertices.len()))
            .copied()
            .collect();
        if triangles.is_empty() {
            commands.entity(entity).insert(ClothSkipped);
            continue;
        }
        let pins = pins_from_skin(vertices.len(), None);
        let cloth = cloth_of(&vertices, &triangles, &pins);
        let copy = mesh_data.clone();
        let own = meshes.add(copy);
        commands.entity(entity).insert((
            Mesh3d(own.clone()),
            ClothInstance {
                cloth,
                mesh: own,
                frame: None,
            },
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn step_cloths(
    mut instances: Query<(Entity, &mut ClothInstance, &GlobalTransform)>,
    bodies: Query<&Ragdoll>,
    parents: Query<&ChildOf>,
    scene_nodes: Query<(), With<exa_physics::StaticNode>>,
    physics: Option<Res<Physics>>,
    statics: Option<Res<exa_physics::StaticColliders>>,
    mut objects: ResMut<ClothObjects>,
    live_contacts: Res<ClothContacts>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let gravity = cloth::gravity_cm(1.0);
    // The object list's stable order: the entity id, oldest first. A new or
    // removed cloth rebuilds the grid; otherwise each object migrates after
    // its tick, as the original's list does.
    let mut order: Vec<Entity> = instances.iter().map(|(entity, ..)| entity).collect();
    order.sort_unstable();
    let rebuilt = objects.order != order;
    if rebuilt {
        objects.order = order.clone();
        objects.grid = sector::ObjectGrid::new(
            ClothObjects::COLUMNS,
            ClothObjects::ROWS,
            ClothObjects::ORIGIN,
        );
    }
    let order = objects.order.clone();
    // Every part of every body (asleep too) that can meet an object, with
    // its body's sleep, built once a tick for all the cloths.
    let mut all_parts = Vec::new();
    if live_contacts.0 {
        for body in &bodies {
            let asleep = body.asleep();
            for part in &body.parts {
                if part.pairing.out || part.spheres.is_empty() {
                    continue;
                }
                let (centre, radius) = part.sphere_bound();
                all_parts.push((
                    part.entity,
                    asleep,
                    ObjectBodyPart {
                        now: part.transform(),
                        previous: part.settled_transform(),
                        spheres: &part.spheres,
                        centre,
                        radius,
                        mesh: &part.pairing.mesh,
                        friction: part.friction,
                    },
                ));
            }
        }
    }
    let mut near = Vec::new();
    let mut triangles = Vec::new();
    for (id, &entity) in order.iter().enumerate() {
        let Ok((_, mut instance, at)) = instances.get_mut(entity) else {
            continue;
        };
        let (object, to_object) = cloth_bound(&instance.cloth, at);
        if rebuilt {
            objects.grid.insert(object.centre);
        } else {
            objects.grid.migrate(id, object.centre);
        }
        let asleep = instance.cloth.sleep <= 0;
        let frame = at.affine();
        let moved = instance.frame != Some(frame);
        instance.frame = Some(frame);
        // The cloth's own character, and the scene node whose static mesh
        // the cloth is drawn from: neither pushes it. The original's static
        // stage skips the mesh of the cloth's own node; the body skip is a
        // guess [the cloth's body-contact call is taken to skip the parts
        // above its node].
        let mut own = Vec::new();
        let mut own_node = scene_nodes.contains(entity).then_some(entity);
        let mut above = entity;
        while let Ok(parent) = parents.get(above) {
            above = parent.parent();
            own.push(above);
            if own_node.is_none() && scene_nodes.contains(above) {
                own_node = Some(above);
            }
            if own.len() > 64 {
                break;
            }
        }
        let mut contacts = ObjectContacts::default();
        if let (true, Some(physics), Some(statics)) = (live_contacts.0, &physics, &statics) {
            // Objects against bodies: the parts passing the sector's
            // pre-test ([`sector::object_body_meet`]), less the cloth's own
            // character; a sleeping cloth takes awake bodies only.
            let object_box = cloth::bounds(&instance.cloth.particles);
            let parts = all_parts
                .iter()
                .filter(|(owner, body_asleep, _)| {
                    sector::object_takes_body(asleep, *body_asleep) && !own.contains(owner)
                })
                .map(|&(_, _, part)| part);
            contacts.gather_bodies(&to_object, object, object_box, parts);
            // A sleeping cloth that no body reaches, in a frame that did
            // not move, is the level stage's fixed point: nothing to do.
            if asleep && !moved && contacts.spheres.is_empty() && contacts.swept.is_empty() {
                continue;
            }
            // Objects against the level: the static grid's records around
            // the object's cell that pass its pre-test, less the cloth's own
            // node's meshes, each one's triangles within the object's bound
            // box as one piece.
            let centre_m = object.centre * 0.01;
            let radius_m = object.radius * 0.01;
            let (lo, hi) = (
                centre_m - Vec3::splat(radius_m),
                centre_m + Vec3::splat(radius_m),
            );
            let own_index = own_node.map(|n| n.index().index());
            near.clear();
            statics.near_object(object.centre, object.radius, &mut near);
            for &record_id in &near {
                let Some(record) = statics.grid().get(record_id) else {
                    continue;
                };
                if own_index.is_some_and(|n| n != 0 && n == record.node) {
                    continue;
                }
                triangles.clear();
                physics.0.triangles(record.piece, lo, hi, &mut triangles);
                contacts.gather_level(&to_object, triangles.iter().copied());
            }
        } else if asleep {
            // A sleeping cloth with no contacts does not step.
            continue;
        }
        instance.cloth.spheres = std::mem::take(&mut contacts.spheres);
        instance.cloth.swept = std::mem::take(&mut contacts.swept);
        instance.cloth.level = std::mem::take(&mut contacts.level);
        instance.cloth.level_pieces = std::mem::take(&mut contacts.pieces);
        cloth::step(&mut instance.cloth, gravity);
        // The object list's re-bucket after its step, as the original's.
        let (after, _) = cloth_bound(&instance.cloth, at);
        objects.grid.migrate(id, after.centre);
        let Some(mut mesh) = meshes.get_mut(&instance.mesh) else {
            continue;
        };
        let positions: Vec<Vec3> = instance
            .cloth
            .vertices()
            .iter()
            .map(|p| *p * 0.01)
            .collect();
        let Some(attribute) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) else {
            continue;
        };
        if let bevy::mesh::VertexAttributeValues::Float32x3(values) = attribute {
            for (slot, p) in values.iter_mut().zip(&positions) {
                *slot = p.to_array();
            }
        }
    }
}

/// Simulates the cloth of every cloth-marked mesh part.
pub struct ClothPlugin;

impl Plugin for ClothPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClothObjects>()
            .init_resource::<ClothContacts>()
            .add_systems(Update, build_cloth_nodes)
            // The sector runs its simulated objects after the bodies' tick
            // (the original's object step and contacts are inside it).
            .add_systems(
                FixedUpdate,
                step_cloths.after(exa_physics::body::BodyPostStep),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unskinned_cloth_pins_every_vertex_loosely() {
        let pins = pins_from_skin(4, None);
        assert_eq!(pins.len(), 4);
        assert!(pins.iter().all(|(_, w)| *w == LOOSE_PIN));
        let weights = [
            [1.0, 0.0, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
            [0.4, 0.3, 0.2, 0.1],
            [1.0, 0.0, 0.0, 0.0],
        ];
        let pins = pins_from_skin(4, Some(&weights));
        assert_eq!(pins, vec![(0, LOOSE_PIN), (1, LOOSE_PIN), (3, LOOSE_PIN)]);
    }

    /// The cloth of a small square pins at its pins and keeps the mesh's
    /// edges as structural links.
    #[test]
    fn a_mesh_becomes_a_pinned_cloth() {
        let vertices = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ];
        let triangles = [[0, 1, 2], [0, 2, 3]];
        let pins = pins_from_skin(4, None);
        let cloth = cloth_of(&vertices, &triangles, &pins);
        assert_eq!(cloth.particles.len(), 4);
        assert_eq!(cloth.triangles.len(), 2);
        // Four edges and the diagonal, plus one bend link.
        assert_eq!(cloth.edges.len(), 6);
        assert!(
            cloth.particles[0]
                .position
                .abs_diff_eq(Vec3::new(0.0, 100.0, 0.0), 1e-4)
        );
        assert!(cloth.pins.iter().all(|p| p.weight == LOOSE_PIN));
    }

    /// A free cloth falls through the air and comes to rest on the level's
    /// triangle: the sector's static grid finds the floor, the object's
    /// level contacts are in its own centimetres, and the object list knows
    /// the cloth's cell.
    #[test]
    fn a_free_cloth_lands_on_the_static_grids_level() {
        let (mut app, entity) = falling_cloth(false);
        let instance = app.world().entity(entity).get::<ClothInstance>().unwrap();
        assert!(!instance.cloth.level.is_empty());
        let low = lowest_m(&app, entity);
        assert!(low > 0.5, "the cloth fell through the level: {low} m");
        let objects = app.world().resource::<ClothObjects>();
        assert_eq!(objects.order, [entity]);
        assert_eq!(objects.grid.objects().len(), 1);
        // Landed and still, it falls asleep: the level never wakes it.
        for _ in 0..200 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        let instance = app.world().entity(entity).get::<ClothInstance>().unwrap();
        assert_eq!(instance.cloth.sleep, 0, "the landed cloth never slept");
    }

    /// The level mesh of the cloth's own scene node does not push it, as
    /// the original's static stage skips the cloth's own node.
    #[test]
    fn a_cloth_passes_through_its_own_nodes_mesh() {
        let (app, entity) = falling_cloth(true);
        let low = lowest_m(&app, entity);
        assert!(low < 0.5, "the cloth's own mesh held it: {low} m");
    }

    /// The lowest vertex of a cloth, in metres.
    fn lowest_m(app: &App, entity: Entity) -> f32 {
        let instance = app.world().entity(entity).get::<ClothInstance>().unwrap();
        instance
            .cloth
            .vertices()
            .iter()
            .map(|p| p.y)
            .fold(f32::MAX, f32::min)
            * 0.01
    }

    /// A 1 m cloth dropped from 1-2 m onto a floor quad at y = 0.5 m, in
    /// the world and in the sector's grid, after 150 ticks. With `own`, the
    /// floor belongs to the scene node the cloth hangs under.
    fn falling_cloth(own: bool) -> (App, Entity) {
        use exa_physics::Physics;
        use exa_physics::StaticColliders;
        use rapier3d::prelude::ColliderBuilder;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ClothPlugin))
            .init_resource::<Assets<Mesh>>()
            .insert_resource(Physics::default())
            .insert_resource(StaticColliders::default())
            .insert_resource(ClothContacts(true));
        let node = app
            .world_mut()
            .spawn((exa_physics::StaticNode, GlobalTransform::default()))
            .id();
        let vertices = vec![
            Vec3::new(-2.0, 0.5, -2.0),
            Vec3::new(2.0, 0.5, -2.0),
            Vec3::new(2.0, 0.5, 2.0),
            Vec3::new(-2.0, 0.5, 2.0),
        ];
        let indices = vec![[0u32, 2, 1], [0, 3, 2]];
        let collider = ColliderBuilder::trimesh(vertices, indices).unwrap().build();
        let handle = app
            .world_mut()
            .resource_mut::<Physics>()
            .0
            .insert_collider(collider, None);
        let floor_node = if own { node.index().index() } else { 0 };
        app.world_mut()
            .resource_mut::<StaticColliders>()
            .insert_aabb(
                handle.into_raw_parts().0,
                Vec3::new(-2.0, 0.5, -2.0),
                Vec3::new(2.0, 0.5, 2.0),
                floor_node,
                exa_physics::sector::MASK_SMALL,
            );
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            ));
        let corners = [
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let cloth = cloth_of(&corners, &[[0, 1, 2], [0, 2, 3]], &[]);
        let entity = app
            .world_mut()
            .spawn((
                ClothInstance {
                    cloth,
                    mesh,
                    frame: None,
                },
                GlobalTransform::default(),
                ChildOf(node),
            ))
            .id();
        for _ in 0..150 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        (app, entity)
    }
}
