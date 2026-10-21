//! CPU-side skin data: the skin matrices that bind a mesh's vertices to a
//! skeleton, the weighted deformation of one vertex, the joint numbering
//! and inverse binds a skinned mesh registers, and the translation of a
//! mesh's vertices.
//!
//! The original's geometry controller keeps a list of bone matrix
//! pointers, the bind-inverse matrices relative to the mesh's own part and
//! the current skin matrices (`joint world × inverse bind`), and its
//! per-frame deform applies them to every vertex with up to four weights.
//! Exarust lets Bevy's GPU skinning do the per-frame deform; these are the
//! same numbers on the CPU for the features that read a vertex's world
//! place (the wound painter, `exa_character::blood`) and for the bind the
//! controller registers.
//!
//! Bone numbering follows the original: bone `n` is joint `n`, bone 0 is
//! the mesh's own part and every gap bone takes the own part with an
//! identity inverse. The inverse bind is the bone's rest transform under
//! the mesh part's, inverted, so the mesh's rest pose skins to itself.

use std::collections::HashMap;

use bevy::prelude::*;

/// The skin matrix of a joint: its world matrix times the joint's inverse
/// bind pose.
pub fn skin_matrix(joint_world: Mat4, inverse_bind: Mat4) -> Mat4 {
    joint_world * inverse_bind
}

/// One vertex skinned by its joints: the weighted sum of each joint's skin
/// matrix applied to `position`, in the same order and with the same
/// four weights the GPU uses. Zero weights are skipped; a joint past the
/// matrices is skipped.
pub fn skin_vertex(position: Vec3, joints: [u16; 4], weights: [f32; 4], matrices: &[Mat4]) -> Vec3 {
    let mut out = Vec3::ZERO;
    for k in 0..4 {
        if weights[k] == 0.0 {
            continue;
        }
        let Some(m) = matrices.get(usize::from(joints[k])) else {
            continue;
        };
        out += m.transform_point3(position) * weights[k];
    }
    out
}

/// The joint numbering and inverse binds of a mesh on bone `own_bone`
/// whose bones' rest transforms under the mesh's own part are `rest`:
/// joint `n` is bone `n` (0 to the highest bone), an absent bone taking
/// the own part's entity index with an identity inverse. `own_rest` is the
/// own part's rest transform under itself (the identity in the mesh's
/// space).
pub fn bindings(
    rest: &HashMap<usize, Mat4>,
    own_bone: usize,
    own_rest: Mat4,
) -> (Vec<usize>, Vec<Mat4>) {
    let count = rest.keys().max().map_or(0, |b| b + 1);
    (0..count)
        .map(|bone| match rest.get(&bone) {
            Some(&m) => (bone, m.inverse() * own_rest),
            None => (own_bone, Mat4::IDENTITY),
        })
        .unzip()
}

/// Moves every vertex of `mesh` by `by` (the original's mesh translate,
/// which also refits its bounds; Bevy derives a mesh's bounds from the
/// positions, so only the stream changes here).
pub fn translate_mesh(mesh: &mut Mesh, by: Vec3) {
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    for p in positions {
        let moved = Vec3::from(*p) + by;
        *p = [moved.x, moved.y, moved.z];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A translation joint with no inverse bind skins a point to the
    /// translated place; an inverse bind of the joint's rest brings it back
    /// to where it started.
    #[test]
    fn the_skin_matrix_is_the_joint_times_its_inverse_bind() {
        let rest = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let moved = Mat4::from_translation(Vec3::new(4.0, 5.0, 6.0));
        let p = Vec3::new(0.5, 0.0, -1.0);
        let free = skin_matrix(moved, Mat4::IDENTITY).transform_point3(p);
        assert_eq!(free, Vec3::new(4.5, 5.0, 5.0));
        let bound = skin_matrix(moved, rest.inverse()).transform_point3(p);
        assert!((bound - Vec3::new(3.5, 3.0, 2.0)).length() < 1e-6);
        // At rest the skin is the identity.
        let rest_skin = skin_matrix(rest, rest.inverse());
        assert!((rest_skin.transform_point3(p) - p).length() < 1e-6);
    }

    /// Two joints blend by their weights; zero weights and missing joints
    /// contribute nothing.
    #[test]
    fn a_vertex_takes_its_weighted_joints() {
        let a = Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0));
        let b = Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0));
        let p = Vec3::ZERO;
        let at = skin_vertex(p, [0, 1, 2, 3], [0.75, 0.25, 0.0, 0.0], &[a, b]);
        assert_eq!(at, Vec3::new(0.75, 0.5, 0.0));
        let one = skin_vertex(p, [0, 0, 0, 0], [1.0, 0.0, 0.0, 0.0], &[a]);
        assert_eq!(one, Vec3::new(1.0, 0.0, 0.0));
        // A missing matrix is left out.
        assert_eq!(
            skin_vertex(p, [9, 0, 0, 0], [1.0, 0.0, 0.0, 0.0], &[a]),
            Vec3::ZERO
        );
    }

    /// The binding list numbers every bone from 0, a gap taking the own
    /// bone and identity, and each inverse is the bone's rest under the
    /// mesh's.
    #[test]
    fn bindings_number_the_bones_and_take_gaps_from_the_own_part() {
        let mut rest = HashMap::new();
        rest.insert(0, Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0)));
        rest.insert(2, Mat4::from_translation(Vec3::new(0.0, 3.0, 0.0)));
        let (bones, inverses) = bindings(&rest, 0, Mat4::IDENTITY);
        assert_eq!(bones, [0, 0, 2]);
        // Bone 1 is a gap: identity.
        assert_eq!(inverses[1], Mat4::IDENTITY);
        assert!(
            (inverses[0].transform_point3(Vec3::ZERO) - Vec3::new(0.0, -1.0, 0.0)).length() < 1e-6
        );
        assert!(
            (inverses[2].transform_point3(Vec3::ZERO) - Vec3::new(0.0, -3.0, 0.0)).length() < 1e-6
        );
        let (bones, inverses) = bindings(&HashMap::new(), 7, Mat4::IDENTITY);
        assert!(bones.is_empty());
        assert!(inverses.is_empty());
    }

    /// Translating a mesh moves every position by the offset.
    #[test]
    fn a_mesh_translates_its_vertices() {
        let mut mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0.0f32, 0.0, 0.0], [1.0, 2.0, 3.0]],
        );
        translate_mesh(&mut mesh, Vec3::new(0.5, -1.0, 2.0));
        let p = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap()
            .to_vec();
        assert_eq!(p, vec![[0.5, -1.0, 2.0], [1.5, 1.0, 5.0]]);
    }
}
