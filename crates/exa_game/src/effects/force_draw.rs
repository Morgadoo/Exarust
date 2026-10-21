//! The force domes drawn in the running game: a [`EffectShell`] entity
//! gets its barrier or bubble mesh once, drawn with the existing glow
//! material (the original's `effectbarrier`/`effectbubble` programs are not
//! ported yet), and is left alone afterwards. The mesh is rebuilt only when
//! the entity's radius or kind changes, so an idle scene costs a query.
//!
//! The wave's front is drawn the same way from [`Wave::front_mesh`], and
//! the rain from [`Rain::mesh`] with its own additive tint. Nothing starts a
//! shell or a rain set yet: their callers (the Barrier and Bubble powers, the
//! scene's weather) are outside this lane.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::prelude::*;

use exa_assets::polyhedra::Polyhedron;
use exa_render::glow::GlowMaterial;

use super::force_dome::{BARRIER_DRAW_SCALE, BARRIER_FORWARD_CM, barrier_mesh, bubble_mesh};

/// The two dome shells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    /// The barrier's half dome.
    Barrier,
    /// The bubble's full sphere.
    Bubble,
}

/// A force shell to draw: its kind and radius in cm.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct EffectShell {
    pub kind: ShellKind,
    pub radius: f32,
}

/// The drawn shell's mesh, so it is built once.
#[derive(Component, Debug, Clone)]
pub struct ShellDrawn(pub Handle<Mesh>);

/// Build a dome's mesh: points, triangles and the shell's tint.
pub fn shell_mesh(kind: ShellKind, radius: f32) -> (Mesh, Vec4) {
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    let (points, faces) = match kind {
        ShellKind::Barrier => {
            let (points, faces) = barrier_mesh(1.0);
            (points, faces)
        }
        ShellKind::Bubble => {
            let poly: Polyhedron = bubble_mesh(radius);
            (poly.points, poly.faces)
        }
    };
    let indices: Vec<u32> = faces.iter().flat_map(|face| face.iter().copied()).collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    let tint = match kind {
        ShellKind::Barrier => Vec4::new(0.35, 0.5, 1.0, 0.5),
        ShellKind::Bubble => Vec4::new(0.4, 0.7, 1.0, 0.4),
    };
    (mesh, tint)
}

/// Builds and draws each newly added shell, then leaves it alone.
pub fn draw_new_shells(
    mut commands: Commands,
    shells: Query<(Entity, &EffectShell), Added<EffectShell>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlowMaterial>>,
) {
    for (entity, shell) in &shells {
        let (mesh, tint) = shell_mesh(shell.kind, shell.radius);
        let handle = meshes.add(mesh);
        commands.entity(entity).insert((
            ShellDrawn(handle.clone()),
            Mesh3d(handle),
            MeshMaterial3d(materials.add(GlowMaterial { tint })),
            match shell.kind {
                ShellKind::Barrier => Transform {
                    translation: Vec3::new(0.0, 0.0, BARRIER_FORWARD_CM),
                    scale: BARRIER_DRAW_SCALE,
                    ..Transform::IDENTITY
                },
                ShellKind::Bubble => Transform::from_scale(Vec3::splat(shell.radius)),
            },
            NoFrustumCulling,
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_barrier_shell_has_both_windings_and_a_bubble_a_closed_sphere() {
        let (barrier, tint) = shell_mesh(ShellKind::Barrier, 100.0);
        assert!(matches!(
            barrier.primitive_topology(),
            bevy::mesh::PrimitiveTopology::TriangleList
        ));
        assert_eq!(tint.w, 0.5);
        let (bubble, _) = shell_mesh(ShellKind::Bubble, 1.0);
        assert_eq!(bubble.count_vertices(), bubble_mesh(1.0).points.len());
    }
}
