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

use bevy::prelude::*;

use exa_physics::cloth::{self, Cloth};

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

/// Gives each cloth node its cloth, once its mesh is in place (a skinned
/// part gets its mesh when it is bound to its skeleton, after it spawns).
///
/// The cloth moves its mesh's vertices, and one mesh asset is shared by
/// every copy of a model, so each cloth takes a copy of the mesh for its
/// node alone.
fn build_cloth_nodes(
    mut commands: Commands,
    nodes: Query<
        (Entity, &Mesh3d),
        (
            With<ClothNode>,
            Without<ClothInstance>,
            Without<ClothSkipped>,
        ),
    >,
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
        commands
            .entity(entity)
            .insert((Mesh3d(own.clone()), ClothInstance { cloth, mesh: own }));
    }
}

fn step_cloths(mut instances: Query<&mut ClothInstance>, mut meshes: ResMut<Assets<Mesh>>) {
    let gravity = cloth::gravity_cm(1.0);
    for mut instance in &mut instances {
        if instance.cloth.sleep <= 0 {
            continue;
        }
        cloth::step(&mut instance.cloth, gravity);
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
        app.add_systems(Update, build_cloth_nodes)
            .add_systems(FixedUpdate, step_cloths);
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
}
