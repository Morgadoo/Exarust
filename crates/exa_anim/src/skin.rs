//! CPU-side skin data: the skin matrices that bind a mesh's vertices to a
//! skeleton, the weighted deformation of one vertex, the joint numbering
//! and inverse binds a skinned mesh registers, and the translation of a
//! mesh's vertices.
//!
//! The original's geometry controller keeps a list of bone matrix
//! pointers, the bind-inverse matrices relative to the mesh's own part and
//! the current skin matrices (`joint world × inverse bind`), and its
//! per-frame deform applies them to every vertex with up to four weights.
//! Exarust lets Bevy's GPU skinning do the per-frame deform (it blends all
//! four joints, in GPU arithmetic);
//! [`skin_vertex`] is the original's CPU order for code that needs it, not
//! yet called. The bind uses [`bindings`].
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

/// A skin matrix as the original keeps it: three axis rows and the
/// translation, each three floats (`[x, y, z, t]`).
pub type SkinRows = [[f32; 3]; 4];

/// A [`Mat4`] (affine) as [`SkinRows`].
pub fn skin_rows(m: Mat4) -> SkinRows {
    [
        m.x_axis.truncate().to_array(),
        m.y_axis.truncate().to_array(),
        m.z_axis.truncate().to_array(),
        m.w_axis.truncate().to_array(),
    ]
}

/// One vertex skinned as the original's per-frame deform does it: the
/// skin matrices of its joints are blended first (the first joint's
/// matrix times its weight, then each further joint's matrix times its
/// weight added element by element, stopping at the first joint number
/// below 1), and the point is transformed once by the blend, each
/// component summed as `(x·a + y·b) + (z·c + t)`. Joint numbers are the
/// record's, one-based (`matrices[n − 1]`); the first joint is used
/// whatever its weight. A number past the matrices ends the blend (and a
/// first one gives the origin), where the original would read past its
/// list.
pub fn skin_vertex(
    position: Vec3,
    joints: [i32; 4],
    weights: [f32; 4],
    matrices: &[SkinRows],
) -> Vec3 {
    let at = |j: i32| usize::try_from(j - 1).ok().and_then(|i| matrices.get(i));
    let Some(first) = at(joints[0]) else {
        return Vec3::ZERO;
    };
    let mut m = first.map(|row| row.map(|v| v * weights[0]));
    for k in 1..4 {
        if joints[k] < 1 {
            break;
        }
        let Some(next) = at(joints[k]) else {
            break;
        };
        for (row, other) in m.iter_mut().zip(next) {
            for (v, o) in row.iter_mut().zip(other) {
                *v += o * weights[k];
            }
        }
    }
    let p = position.to_array();
    Vec3::from_array(std::array::from_fn(|c| {
        (p[0] * m[0][c] + p[1] * m[1][c]) + (p[2] * m[2][c] + m[3][c])
    }))
}

/// The first value met for each bone, as the original's joint capture
/// takes it: the first node reached for a bone number sets its frame and a
/// later node of the same bone leaves it alone. The walk's order is the
/// caller's (the original recurses in child order, depth first).
pub fn first_of<T>(iter: impl Iterator<Item = (usize, T)>) -> HashMap<usize, T> {
    let mut values = HashMap::new();
    for (bone, value) in iter {
        values.entry(bone).or_insert(value);
    }
    values
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

    /// The joint capture keeps the first frame met for a bone and sizes the
    /// list by the highest bone number, an absent bone taking the own part.
    #[test]
    fn the_first_frame_of_a_bone_wins() {
        let a = Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0));
        let b = Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0));
        let frames = first_of([(3, a), (5, b), (3, b)].into_iter());
        assert_eq!(frames[&3], a);
        assert_eq!(frames[&5], b);
        let own = Mat4::from_translation(Vec3::new(0.5, 0.0, 0.0));
        let (bones, inverse) = bindings(&frames, 3, own);
        // Joints 0 to 5: the gaps take the own bone with an identity bind.
        assert_eq!(bones, [3, 3, 3, 3, 3, 5]);
        assert_eq!(inverse[3], a.inverse() * own);
        assert_eq!(inverse[0], Mat4::IDENTITY);
        assert_eq!(inverse[5], b.inverse() * own);
    }

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

    /// Two joints blend by their weights (one-based numbers); the blend
    /// stops at the first number below 1, whatever the weights after it.
    #[test]
    fn a_vertex_takes_its_blended_joints() {
        let a = skin_rows(Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)));
        let b = skin_rows(Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0)));
        let p = Vec3::new(1.0, 1.0, 1.0);
        let at = skin_vertex(p, [1, 2, 0, 1], [0.75, 0.25, 0.5, 0.5], &[a, b]);
        assert_eq!(at, Vec3::new(1.75, 1.5, 1.0));
        let one = skin_vertex(p, [1, 0, 2, 2], [1.0, 0.0, 1.0, 1.0], &[a, b]);
        assert_eq!(one, Vec3::new(2.0, 1.0, 1.0));
        // No first matrix: nothing.
        assert_eq!(skin_vertex(p, [9, 0, 0, 0], [1.0; 4], &[a]), Vec3::ZERO);
        // The per-component sum is (x·a + y·b) + (z·c + t).
        let big = [
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0e8, 0.0, 0.0],
        ];
        let q = skin_vertex(
            Vec3::new(1.0e8, 0.5, 0.5),
            [1, 0, 0, 0],
            [1.0, 0.0, 0.0, 0.0],
            &[big],
        );
        assert_eq!(q.x, (1.0e8f32 + 0.5) + (0.5 + -1.0e8));
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
