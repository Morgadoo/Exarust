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
    blend_skin_matrix(&SkinBinding { joints, weights }, matrices)
        .map_or(Vec3::ZERO, |m| transform_point(m, position))
}

/// A vertex's joints as the mesh stores them: four one-based joint numbers
/// and their weights.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinBinding {
    pub joints: [i32; 4],
    pub weights: [f32; 4],
}

/// A point transformed by a 12-float matrix, summed as the original sums
/// it: `(x·a + y·b) + (z·c + t)` per component.
pub fn transform_point(m: SkinRows, p: Vec3) -> Vec3 {
    let p = p.to_array();
    Vec3::from_array(std::array::from_fn(|c| {
        (p[0] * m[0][c] + p[1] * m[1][c]) + (p[2] * m[2][c] + m[3][c])
    }))
}

/// A direction rotated by a 12-float matrix, without its translation:
/// `(x·a + y·b) + z·c`.
pub fn transform_vector(m: SkinRows, p: Vec3) -> Vec3 {
    let p = p.to_array();
    Vec3::from_array(std::array::from_fn(|c| {
        (p[0] * m[0][c] + p[1] * m[1][c]) + p[2] * m[2][c]
    }))
}

/// A direction turned back by a rotation matrix (the transpose): its dot
/// with each axis row, summed `(x·a + y·b) + z·c`.
pub fn inverse_transform_vector(m: SkinRows, v: Vec3) -> Vec3 {
    Vec3::from_array(std::array::from_fn(|c| {
        (v.x * m[c][0] + v.y * m[c][1]) + v.z * m[c][2]
    }))
}

/// The inverse of a point for a rotation matrix: the point less the
/// translation, turned back by [`inverse_transform_vector`] (the original's
/// placement helper).
pub fn inverse_transform_point(m: SkinRows, p: Vec3) -> Vec3 {
    inverse_transform_vector(m, p - Vec3::from_array(m[3]))
}

/// The product of two 12-float matrices in the original's order: `a` is
/// applied first, then `b` (each row of `a` turned by `b`'s axes, summed
/// `(x·b₀ + y·b₁) + z·b₂`, and `a`'s translation carried through `b` as a
/// point). In Bevy's terms this is `b * a`.
pub fn compose(a: SkinRows, b: SkinRows) -> SkinRows {
    let turn = |r: [f32; 3]| -> [f32; 3] {
        std::array::from_fn(|c| (r[0] * b[0][c] + r[1] * b[1][c]) + r[2] * b[2][c])
    };
    [
        turn(a[0]),
        turn(a[1]),
        turn(a[2]),
        transform_point(b, Vec3::from_array(a[3])).to_array(),
    ]
}

/// The length of a vector as the original's helper sums it:
/// `√((x² + z²) + y²)`.
pub fn length(v: Vec3) -> f32 {
    ((v.x * v.x + v.z * v.z) + v.y * v.y).sqrt()
}

/// A direction scaled by the reciprocal square root of its length², summed
/// as `(x² + z²) + y²`; a zero direction stays zero. The original uses the
/// hardware reciprocal square root, which is a few bits short of
/// `1/√len²`; Exarust uses the exact reciprocal (a deviation of at most one
/// unit in the last place of a normal).
pub fn normalised(v: Vec3) -> Vec3 {
    let len2 = (v.x * v.x + v.z * v.z) + v.y * v.y;
    if len2 == 0.0 {
        Vec3::ZERO
    } else {
        v * (1.0 / len2.sqrt())
    }
}

/// A joint's matrix in the mesh node's frame, as the original computes it
/// before each skin pass: every axis row of the joint turned back by the
/// node's rotation ([`inverse_transform_vector`]) and the joint's
/// translation placed in the node ([`inverse_transform_point`]); the node's
/// rotation is taken to be orthonormal.
pub fn joint_in_node_frame(joint: SkinRows, node: SkinRows) -> SkinRows {
    let back = |r: [f32; 3]| inverse_transform_vector(node, Vec3::from_array(r)).to_array();
    [
        back(joint[0]),
        back(joint[1]),
        back(joint[2]),
        inverse_transform_point(node, Vec3::from_array(joint[3])).to_array(),
    ]
}

/// The per-joint skin matrices of a frame, in bone order: each base matrix
/// (the joint's inverse bind) applied first, then its joint's matrix
/// ([`compose`]). `joints_local` says the joint matrices are already in the
/// node's frame (the original's mesh flag for it); otherwise each is first
/// brought into it with [`joint_in_node_frame`].
pub fn frame_matrices(
    base: &[SkinRows],
    joints: &[SkinRows],
    node: SkinRows,
    joints_local: bool,
) -> Vec<SkinRows> {
    base.iter()
        .zip(joints)
        .map(|(&base, &joint)| {
            let joint = if joints_local {
                joint
            } else {
                joint_in_node_frame(joint, node)
            };
            compose(base, joint)
        })
        .collect()
}

/// The blend of the skin matrices themselves, the original's first step in
/// the per-frame deform: the first joint's matrix times its weight, then
/// every further joint's matrix times its weight added element by element,
/// stopping at the first joint number at or below zero. `None` when the
/// first joint has no matrix (the original would read past its list).
pub fn blend_skin_matrix(binding: &SkinBinding, matrices: &[SkinRows]) -> Option<SkinRows> {
    let at = |j: i32| usize::try_from(j - 1).ok().and_then(|i| matrices.get(i));
    let first = at(binding.joints[0])?;
    let mut m = first.map(|row| row.map(|v| v * binding.weights[0]));
    for k in 1..4 {
        if binding.joints[k] <= 0 {
            break;
        }
        let Some(next) = at(binding.joints[k]) else {
            break;
        };
        for (row, other) in m.iter_mut().zip(next) {
            for (v, o) in row.iter_mut().zip(other) {
                *v += o * binding.weights[k];
            }
        }
    }
    Some(m)
}

/// One vertex skinned by transforming it with each joint's matrix first and
/// blending the results in the original's second order: the first joint's
/// transformed point times its weight, then each further joint's transformed
/// point times its weight added, component by component, up to the first
/// joint number at or below zero. Joint numbers are one-based.
pub fn skin_vertex_by_positions(
    position: Vec3,
    binding: &SkinBinding,
    matrices: &[SkinRows],
) -> Vec3 {
    let at = |j: i32| usize::try_from(j - 1).ok().and_then(|i| matrices.get(i));
    let Some(first) = at(binding.joints[0]) else {
        return Vec3::ZERO;
    };
    let mut out = transform_point(*first, position) * binding.weights[0];
    for k in 1..4 {
        if binding.joints[k] <= 0 {
            break;
        }
        let Some(next) = at(binding.joints[k]) else {
            break;
        };
        out += transform_point(*next, position) * binding.weights[k];
    }
    out
}

/// Every rest position skinned the position-blended way, in vertex order:
/// the pass that bakes a pose into a mesh's own position stream (it checks
/// no joint number; a first joint without a matrix gives the origin here).
pub fn bake_skin_positions(
    rest: &[Vec3],
    bindings: &[SkinBinding],
    matrices: &[SkinRows],
) -> Vec<Vec3> {
    rest.iter()
        .zip(bindings)
        .map(|(&p, binding)| skin_vertex_by_positions(p, binding, matrices))
        .collect()
}

/// The previous-position stream a part's motion pass starts from, filled
/// the position-blended way ([`skin_vertex_by_positions`]) for every index
/// of the part's stream (`previous[slot]` from the rest position of vertex
/// `indices[slot]`). A vertex whose first joint number is below 1 or past
/// the matrices has that joint number set to 1 and its slot left as it
/// was, as the original does.
pub fn prime_previous_positions(
    indices: &[u32],
    rest: &[Vec3],
    bindings: &mut [SkinBinding],
    matrices: &[SkinRows],
    previous: &mut [Vec3],
) {
    for (slot, &vertex) in indices.iter().enumerate() {
        let (Some(binding), Some(&p), Some(out)) = (
            bindings.get_mut(vertex as usize),
            rest.get(vertex as usize),
            previous.get_mut(slot),
        ) else {
            continue;
        };
        let first = binding.joints[0];
        if first < 1 || usize::try_from(first).is_ok_and(|n| n > matrices.len()) {
            binding.joints[0] = 1;
            continue;
        }
        *out = skin_vertex_by_positions(p, binding, matrices);
    }
}

/// Moves a skinned mesh's previous-position streams by `offset`, part by
/// part, as the original does when the world origin shifts: a part with no
/// such stream ends the walk (the parts after it keep theirs). The mesh's
/// centres move with the same addition (the caller's).
pub fn shift_previous_positions(parts: &mut [Option<Vec<Vec3>>], offset: Vec3) {
    for part in parts {
        let Some(previous) = part else {
            return;
        };
        for p in previous {
            *p += offset;
        }
    }
}

/// The normal and tangent one vertex carries after a deform, and the
/// translation-less tangent's unchanged w.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeformedVertex {
    pub position: Vec3,
    pub normal: Option<Vec3>,
    pub tangent: Option<[f32; 4]>,
}

/// One vertex deformed by a blended matrix: the position takes the matrix
/// with its translation, the normal the matrix's rotation then a
/// normalisation, the tangent the rotation alone (its w kept).
pub fn deform_vertex(
    position: Vec3,
    normal: Option<Vec3>,
    tangent: Option<[f32; 4]>,
    m: SkinRows,
) -> DeformedVertex {
    DeformedVertex {
        position: transform_point(m, position),
        normal: normal.map(|n| normalised(transform_vector(m, n))),
        tangent: tangent.map(|t| {
            let v = transform_vector(m, Vec3::new(t[0], t[1], t[2]));
            [v.x, v.y, v.z, t[3]]
        }),
    }
}

/// What a part's deform wrote: one entry per index of the part's stream,
/// not per vertex. A part without the matching stream leaves it `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct PartDeform {
    pub positions: Vec<Vec3>,
    pub normals: Option<Vec<Vec3>>,
    pub tangents: Option<Vec<[f32; 4]>>,
    /// The position the part had before this deform less the new one, which
    /// the original keeps for its motion-blurred pass.
    pub deltas: Option<Vec<Vec3>>,
}

/// A part's index list skinned as the original's per-frame deform does it:
/// for every index (in stream order) the binding is the vertex's, the skin
/// matrix is blended from it, the position is the rest position of that
/// vertex transformed with the blend, and the normal and tangent are rotated
/// by it. `normal_indices` gives each stream index's normal source; `tangents`
/// and `previous` are per stream index. Positions, normals and tangents of an
/// index whose first joint has no matrix stay zero (the original would read
/// past its list).
pub fn deform_part(
    indices: &[u32],
    rest: &[Vec3],
    bindings: &[SkinBinding],
    matrices: &[SkinRows],
    normals: Option<(&[Vec3], &[u32])>,
    tangents: Option<&[[f32; 4]]>,
    previous: Option<&[Vec3]>,
) -> PartDeform {
    let mut out = PartDeform {
        positions: vec![Vec3::ZERO; indices.len()],
        normals: normals.map(|_| vec![Vec3::ZERO; indices.len()]),
        tangents: tangents.map(|_| vec![[0.0; 4]; indices.len()]),
        deltas: previous.map(|_| vec![Vec3::ZERO; indices.len()]),
    };
    for (slot, &vertex) in indices.iter().enumerate() {
        let Some(&rest) = rest.get(vertex as usize) else {
            continue;
        };
        let Some(binding) = bindings.get(vertex as usize) else {
            continue;
        };
        let Some(m) = blend_skin_matrix(binding, matrices) else {
            continue;
        };
        let position = transform_point(m, rest);
        out.positions[slot] = position;
        if let (Some(normals), Some((source, source_indices))) = (&mut out.normals, normals)
            && let Some(&n) = source_indices
                .get(slot)
                .and_then(|&i| source.get(i as usize))
        {
            normals[slot] = normalised(transform_vector(m, n));
        }
        if let (Some(out), Some(source)) = (&mut out.tangents, tangents)
            && let Some(&t) = source.get(slot)
        {
            let v = transform_vector(m, Vec3::new(t[0], t[1], t[2]));
            out[slot] = [v.x, v.y, v.z, t[3]];
        }
        if let (Some(deltas), Some(previous)) = (&mut out.deltas, previous)
            && let Some(&old) = previous.get(slot)
        {
            deltas[slot] = old - position;
        }
    }
    out
}

/// The element-buffer size the draw pass uploads for `index_count` indices
/// (four bytes each).
pub fn element_bytes(index_count: usize) -> usize {
    index_count * 4
}

/// A skinned mesh's bounds after its parts are enveloped: the local box, the
/// centre moved into world space and the half extents and radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldBounds {
    pub min: Vec3,
    pub max: Vec3,
    /// The box's centre in world space.
    pub centre: Vec3,
    pub half: Vec3,
    pub radius: f32,
}

/// One part as the bounds pass reads it: its world centre, its half extents
/// along its own axes, its own world matrix and its bounding radius, which
/// caps its projected extent. The original walks the root node first, then
/// its child nodes of the skinned kind depth first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundsPart {
    pub centre: Vec3,
    pub half: Vec3,
    pub matrix: SkinRows,
    /// The part's own bounding radius (cm), the cap on its projected extent.
    pub reach_limit: f32,
}

/// The bounds of a skinned mesh grown from the mesh's own centre over its
/// parts, in the original's order: each part's centre is brought into the
/// node's frame, its half extents are projected onto the node's three axes
/// (the absolute sums of the part's axes against each), the radius is capped
/// and the box is grown. A NaN part or half is skipped, as every compare is
/// ordered. The centre is the box's middle, moved into world space by the
/// node; the half extents are the box's upper half; the radius is their
/// length.
pub fn bounds_from_parts(centre: Vec3, node: SkinRows, parts: &[BoundsPart]) -> WorldBounds {
    let mut min = centre.to_array();
    let mut max = centre.to_array();
    for part in parts {
        let local = inverse_transform_point(node, part.centre);
        let half = part.half.to_array();
        for axis in 0..3 {
            let n = Vec3::from_array(node[axis]);
            // The projected extent: each of the part's axes against this
            // node axis, absolute, times the matching half extent, summed
            // in the original's order.
            let x = Vec3::from_array(part.matrix[0]).dot(n).abs();
            let y = Vec3::from_array(part.matrix[1]).dot(n).abs();
            let z = Vec3::from_array(part.matrix[2]).dot(n).abs();
            let mut extent = x * half[0] + y * half[1] + z * half[2];
            // A NaN extent or limit is not clamped (an ordered compare).
            if extent > part.reach_limit {
                extent = part.reach_limit;
            }
            let low = local[axis] - extent;
            if low < min[axis] {
                min[axis] = low;
            }
            let high = local[axis] + extent;
            if high > max[axis] {
                max[axis] = high;
            }
        }
    }
    let min = Vec3::from_array(min);
    let max = Vec3::from_array(max);
    // The middle as the original's interpolation takes it: min + (max − min)·½.
    let local_centre = min + (max - min) * 0.5;
    let half = max - local_centre;
    let centre = transform_point(node, local_centre);
    let radius = length(half);
    WorldBounds {
        min,
        max,
        centre,
        half,
        radius,
    }
}

/// A vertex-light: the original's light position moved back half its
/// direction, its direction and its range. The direction's length divides
/// the vertex's dot with it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TintLight {
    pub origin: Vec3,
    pub direction: Vec3,
    /// The reach, compared with the vertex's squared distance (the
    /// original's units: cm²).
    pub range: f32,
}

impl TintLight {
    /// The light as the original lays it out: `position − 0.5 · direction`.
    pub fn new(position: Vec3, direction: Vec3, range: f32) -> Self {
        Self {
            origin: position + direction * -0.5,
            direction,
            range,
        }
    }

    /// The light brought into a mesh node's frame (its origin less the
    /// node's translation, turned by the node's rotation).
    pub fn in_node_frame(mut self, node: SkinRows) -> Self {
        self.origin = inverse_transform_point(node, self.origin);
        self
    }
}

/// One vertex's alpha after the near-light tint, in the original's order:
/// the vertex less the light's origin gives the squared distance `s` and
/// the dot with the direction over its length; a positive dot scales `s` by
/// `1 − dot/√s`; unless the range is below `s` (a NaN `s` passes) the alpha
/// becomes `(1 − ((1 − f)·r + f)²·f)·255` with `f = 1 − s/range` in single
/// and the rest in double with one uniform draw `r`, kept only when it is
/// below the stored alpha (ordered) and rounded half to even. `None` when
/// the vertex is out of range or the value is not below the stored alpha.
pub fn tint_alpha(vertex: Vec3, stored: u8, light: &TintLight, roll: f64) -> Option<u8> {
    let d = vertex - light.origin;
    let mut distance = (d.x * d.x + d.y * d.y) + d.z * d.z;
    let dot = ((d.x * light.direction.x + d.y * light.direction.y) + d.z * light.direction.z)
        / length(light.direction);
    if dot > 0.0 {
        distance *= 1.0 - dot / distance.sqrt();
    }
    if distance > light.range {
        return None;
    }
    let f = 1.0 - distance / light.range;
    let d = (1.0 - f as f64) * roll + f as f64;
    let value = ((1.0 - d * d * f as f64) * 255.0) as f32;
    if value < f32::from(stored) {
        let rounded = value.round_ties_even() as i64 as u8;
        Some(rounded)
    } else {
        None
    }
}

/// The alpha a mesh attached at `attached` shows at `now` while it fades
/// in: `1 − (now − attached)/600` in double, single, floored at zero with
/// an ordered compare.
pub fn fade_alpha(now: u32, attached: u32) -> f32 {
    let ticks = i64::from(now) - i64::from(attached);
    let alpha = (1.0 - ticks as f64 * (1.0 / 600.0)) as f32;
    if alpha < 0.0 { 0.0 } else { alpha }
}

/// The nodes the original's joint capture takes, with their bone numbers,
/// in its order: a depth-first walk in child order from `roots`. A node
/// sets its bone's frame only when no node before it did, and only then
/// are its children visited: a node of a bone number already captured ends
/// its branch. A node of bone 0, or one `bone` gives `None` for (an Exarust
/// entity that is not a body part), is passed through to its children.
/// The original ends the branch at a bone-0 node too; Exarust reads bone
/// numbers from node names, which may not match the original's field for a
/// `00` node, so it keeps walking there (a guess, kept so a body whose top
/// node is numbered `00` still binds).
pub fn capture_joints<N: Copy>(
    roots: &[N],
    children: impl Fn(N) -> Vec<N>,
    bone: impl Fn(N) -> Option<usize>,
) -> Vec<(usize, N)> {
    let mut captured = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut stack: Vec<N> = roots.iter().rev().copied().collect();
    while let Some(node) = stack.pop() {
        match bone(node) {
            Some(b) if b >= 1 && seen.insert(b) => captured.push((b, node)),
            Some(b) if b >= 1 => continue,
            _ => {}
        }
        stack.extend(children(node).into_iter().rev());
    }
    captured
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

    /// The joint capture walks depth first in child order: the first node
    /// met for a bone sets its frame, and a repeated bone ends its branch,
    /// so the children under it are never reached; bone 0 passes through.
    #[test]
    fn the_joint_capture_walks_depth_first_and_prunes_repeats() {
        // Node: (bone, children). 0 → [1, 4]; 1 → [2, 3]; 4 → [5];
        // 5 → [6]; 7 is under the bone-0 node 8, reached from 4 last.
        let tree: [(Option<usize>, &[usize]); 9] = [
            (Some(1), &[1, 4]),
            (Some(2), &[2, 3]),
            (Some(3), &[]),
            (Some(0), &[7]),
            (Some(3), &[5, 8]),
            (Some(6), &[]),
            (Some(9), &[]),
            (Some(4), &[]),
            (Some(0), &[6]),
        ];
        let order = capture_joints(&[0], |n| tree[n].1.to_vec(), |n| tree[n].0);
        // Depth first: 0, 1, 2, then 3 (bone 0, passed through) to 7. Node
        // 4 repeats bone 3, so its subtree (5, 8 and 8's child 6) is never
        // walked: bones 6 and 9 stay uncaptured.
        assert_eq!(order, [(1, 0), (2, 1), (3, 2), (4, 7)]);
        // A breadth-first walk would have met node 4 (bone 3) before node 2.
        let a = Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0));
        let b = Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0));
        let frames: HashMap<usize, Mat4> = [(3, a), (5, b)].into_iter().collect();
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

    /// The transform helpers sum as the original's vector code does:
    /// `(x·a + y·b) + (z·c + t)` for a point and `(x·a + y·b) + z·c` for a
    /// direction. Large terms show the grouping.
    #[test]
    fn the_transforms_keep_the_originals_sum_order() {
        let m = [
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0e8, 0.0, 0.0],
        ];
        let p = transform_point(m, Vec3::new(1.0e8, 0.5, 0.5));
        assert_eq!(p.x, (1.0e8f32 + 0.5) + (0.5 + -1.0e8));
        let v = transform_vector(m, Vec3::new(1.0e8, 0.5, 0.5));
        assert_eq!(v.x, (1.0e8f32 + 0.5) + 0.5);
        // The inverse placement subtracts the translation first: a rotation
        // of 90° about z sends (2, 1, 0) to (1, −2, 0) at the origin.
        let turned = skin_rows(Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2));
        let local = inverse_transform_point(turned, Vec3::new(2.0, 1.0, 0.0));
        assert!((local - Vec3::new(1.0, -2.0, 0.0)).length() < 1e-6);
    }

    /// A matrix mix: the first matrix scaled by its weight, the next added
    /// element by element while its joint number is positive.
    #[test]
    fn a_matrix_blend_takes_the_weighted_joints() {
        let a = skin_rows(Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)));
        let b = skin_rows(Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0)));
        let binding = SkinBinding {
            joints: [1, 2, 0, 9],
            weights: [0.75, 0.25, 1.0, 1.0],
        };
        let m = blend_skin_matrix(&binding, &[a, b]).unwrap();
        assert_eq!(m[3], [0.75, 0.5, 0.0]);
        let d = deform_vertex(
            Vec3::new(1.0, 1.0, 1.0),
            Some(Vec3::X),
            Some([0.0, 0.0, 1.0, -1.0]),
            m,
        );
        assert_eq!(d.position, Vec3::new(1.75, 1.5, 1.0));
        assert_eq!(d.normal, Some(Vec3::X));
        assert_eq!(d.tangent, Some([0.0, 0.0, 1.0, -1.0]));
        // A joint number past the list stops the blend; a first one gives
        // no matrix at all.
        let short = blend_skin_matrix(
            &SkinBinding {
                joints: [1, 7, 0, 0],
                weights: [1.0, 1.0, 0.0, 0.0],
            },
            &[a],
        )
        .unwrap();
        assert_eq!(short, a);
        assert!(
            blend_skin_matrix(
                &SkinBinding {
                    joints: [0, 1, 0, 0],
                    weights: [1.0, 1.0, 0.0, 0.0]
                },
                &[a]
            )
            .is_none()
        );
    }

    /// The position-blended skin pass reaches the same place as the matrix
    /// blend for two translations, and stops at the same joint.
    #[test]
    fn a_vertex_blends_transformed_positions_too() {
        let a = skin_rows(Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)));
        let b = skin_rows(Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0)));
        let p = Vec3::new(1.0, 1.0, 1.0);
        let binding = SkinBinding {
            joints: [1, 2, 0, 1],
            weights: [0.75, 0.25, 0.5, 0.5],
        };
        assert_eq!(
            skin_vertex_by_positions(p, &binding, &[a, b]),
            Vec3::new(1.75, 1.5, 1.0)
        );
        assert_eq!(
            skin_vertex(p, binding.joints, binding.weights, &[a, b]),
            Vec3::new(1.75, 1.5, 1.0)
        );
        assert_eq!(
            skin_vertex_by_positions(
                p,
                &SkinBinding {
                    joints: [2, 0, 0, 0],
                    weights: [1.0, 0.0, 0.0, 0.0],
                },
                &[a, b],
            ),
            Vec3::new(1.0, 3.0, 1.0)
        );
        // The bake runs the same pass over every rest vertex.
        let baked = bake_skin_positions(&[p, Vec3::ZERO], &[binding, binding], &[a, b]);
        assert_eq!(baked[0], Vec3::new(1.75, 1.5, 1.0));
        assert_eq!(baked[1], Vec3::new(0.75, 0.5, 0.0));
    }

    /// The per-frame matrices: the joint is first placed in the node's
    /// frame (turned back by the node's rotation), then the base is applied
    /// before it.
    #[test]
    fn the_frame_matrices_compose_through_the_node() {
        // The node sits at (0, 10, 0) with its x axis along world +y.
        let node = skin_rows(
            Mat4::from_translation(Vec3::new(0.0, 10.0, 0.0))
                * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2),
        );
        let joint = skin_rows(Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0)));
        let local = joint_in_node_frame(joint, node);
        // The joint is 9 below the node along world y, which is the node's
        // −x: (−9, 0, 0) in the node's frame, and its axes turn back by
        // the node's rotation (world x is the node's −y).
        assert!((Vec3::from_array(local[3]) - Vec3::new(-9.0, 0.0, 0.0)).length() < 1e-5);
        assert!((Vec3::from_array(local[0]) - Vec3::new(0.0, -1.0, 0.0)).length() < 1e-6);
        // The base is applied first: a base translation of (1, 0, 0) under
        // a joint turned 90° about z lands on (0, 1, 0), not (1, 0, 0)
        // turned after the fact.
        let base = skin_rows(Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)));
        let turn = skin_rows(Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2));
        let frames = frame_matrices(&[base], &[turn], node, true);
        assert!((Vec3::from_array(frames[0][3]) - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-6);
        let same = skin_rows(
            Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2)
                * Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)),
        );
        for (row, other) in frames[0].iter().zip(same) {
            assert!((Vec3::from_array(*row) - Vec3::from_array(other)).length() < 1e-6);
        }
        // Without the local flag the joint goes through the node first.
        let through = frame_matrices(&[skin_rows(Mat4::IDENTITY)], &[joint], node, false);
        assert!((Vec3::from_array(through[0][3]) - Vec3::new(-9.0, 0.0, 0.0)).length() < 1e-5);
    }

    /// The previous positions start from the position-blended skin of
    /// each index's vertex; a bad first joint is reset to 1 and its slot
    /// kept. A shift moves every stream until a part without one.
    #[test]
    fn the_previous_positions_prime_and_shift() {
        let a = skin_rows(Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)));
        let mut bindings = [
            SkinBinding {
                joints: [1, 0, 0, 0],
                weights: [0.5, 0.0, 0.0, 0.0],
            },
            SkinBinding {
                joints: [2, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            },
        ];
        let rest = [Vec3::new(1.0, 2.0, 0.0), Vec3::ZERO];
        let mut previous = [Vec3::splat(7.0); 3];
        prime_previous_positions(&[0, 1, 0], &rest, &mut bindings, &[a], &mut previous);
        assert_eq!(previous[0], Vec3::new(1.0, 1.0, 0.0));
        assert_eq!(previous[1], Vec3::splat(7.0));
        assert_eq!(bindings[1].joints[0], 1);
        // The third slot reads vertex 0 again.
        assert_eq!(previous[2], Vec3::new(1.0, 1.0, 0.0));
        let mut parts = vec![Some(vec![Vec3::ZERO]), None, Some(vec![Vec3::ONE])];
        shift_previous_positions(&mut parts, Vec3::X);
        assert_eq!(parts[0], Some(vec![Vec3::X]));
        assert_eq!(parts[2], Some(vec![Vec3::ONE]));
    }

    /// The bounds grow from the mesh's centre by each part's projected
    /// extent, capped and with NaN parts skipped.
    #[test]
    fn the_bounds_grow_over_the_parts() {
        let node = skin_rows(Mat4::IDENTITY);
        let part = BoundsPart {
            centre: Vec3::new(10.0, 0.0, 0.0),
            half: Vec3::new(1.0, 2.0, 3.0),
            matrix: skin_rows(Mat4::IDENTITY),
            reach_limit: 100.0,
        };
        let b = bounds_from_parts(Vec3::ZERO, node, &[part]);
        assert_eq!(b.min, Vec3::new(0.0, -2.0, -3.0));
        assert_eq!(b.max, Vec3::new(11.0, 2.0, 3.0));
        assert_eq!(b.centre, Vec3::new(5.5, 0.0, 0.0));
        assert_eq!(b.half, Vec3::new(5.5, 2.0, 3.0));
        assert!((b.radius - (43.25f32).sqrt()).abs() < 1e-6);
        // The cap limits each axis' projected radius.
        let capped = bounds_from_parts(
            Vec3::ZERO,
            node,
            &[BoundsPart {
                reach_limit: 0.5,
                ..part
            }],
        );
        assert_eq!(capped.max, Vec3::new(10.5, 0.5, 0.5));
        // A NaN half extent or centre changes nothing.
        let nan = bounds_from_parts(
            Vec3::ZERO,
            node,
            &[BoundsPart {
                half: Vec3::new(f32::NAN, 2.0, 3.0),
                ..part
            }],
        );
        assert_eq!(nan.min, Vec3::ZERO);
        assert_eq!(nan.max, Vec3::ZERO);
        // The middle is min + (max − min)·½, which differs from the plain
        // average in the last bit for a box from −0.1 to 0.3.
        let small = bounds_from_parts(
            Vec3::ZERO,
            node,
            &[BoundsPart {
                centre: Vec3::new(0.1, 0.0, 0.0),
                half: Vec3::new(0.2, 0.0, 0.0),
                ..part
            }],
        );
        assert_eq!(small.min.x, 0.1f32 - 0.2);
        assert_eq!(small.centre.x, 0.1);
        assert_ne!(small.centre.x, (small.min.x + small.max.x) * 0.5);
        // A node placement moves the centre into world space.
        let moved = bounds_from_parts(
            Vec3::ZERO,
            skin_rows(Mat4::from_translation(Vec3::new(0.0, 0.0, 100.0))),
            &[part],
        );
        // The seed box is in the node's own space: at the origin there, the
        // part at z = −100 grows it to z ∈ [−103, 0], whose middle is
        // −51.5, and the node's place moves that to 48.5.
        assert_eq!(moved.centre, Vec3::new(5.5, 0.0, 48.5));
    }

    /// The near-light tint: the light sits half its direction back, a
    /// positive dot pulls the vertex onto the light's plane, and the
    /// remaining distance gives the alpha.
    #[test]
    fn the_vertex_tint_darkens_near_the_light() {
        // The light at the origin pointing −z: its origin is (0,0,0.5) and
        // the length 1 divides the dot.
        let light = TintLight::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0), 100.0);
        assert_eq!(light.origin, Vec3::new(0.0, 0.0, 0.5));
        // A vertex on the light's axis 10.5 ahead: dot = −10.5, no pull,
        // distance² = 110.25 > 100, so nothing.
        assert_eq!(
            tint_alpha(Vec3::new(0.0, 0.0, 11.0), 200, &light, 0.5),
            None
        );
        // Behind the light along its direction: the dot pulls it onto the
        // origin's plane, distance 0, f = 1 and the draw leaves 0.
        let v = Vec3::new(0.0, 0.0, -10.0);
        assert_eq!(tint_alpha(v, 200, &light, 0.5), Some(0));
        // A stored alpha below the tint keeps the stored one.
        assert_eq!(tint_alpha(v, 0, &light, 0.5), None);
        // Perpendicular to the axis 5 units from the light's origin: the
        // dot is zero, distance² = 25, f = 1 − 25/100 = 0.75, and with a
        // roll of 0 the value is (1 − 0.75³)·255.
        let half = Vec3::new(5.0, 0.0, 0.5);
        let value = ((1.0 - 0.75f64 * 0.75 * 0.75) * 255.0) as f32;
        assert_eq!(
            tint_alpha(half, 255, &light, 0.0),
            Some(value.round_ties_even() as u8)
        );
        // A node placement brings the light into the mesh's frame.
        let node = skin_rows(Mat4::from_translation(Vec3::new(0.0, 0.0, 10.0)));
        let shifted =
            TintLight::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0), 100.0).in_node_frame(node);
        assert_eq!(shifted.origin, Vec3::new(0.0, 0.0, -9.5));
    }

    /// The fade-in: one over 600 ticks, single, floored at zero.
    #[test]
    fn a_mesh_fades_in_over_six_hundred_ticks() {
        assert_eq!(fade_alpha(400, 400), 1.0);
        assert_eq!(fade_alpha(700, 400), 0.5);
        assert_eq!(fade_alpha(1_000, 400), 0.0);
        assert_eq!(fade_alpha(1_001, 400), 0.0);
        assert_eq!(fade_alpha(0, 0), 1.0);
    }

    /// A part deform writes one entry per index: the rest position of the
    /// indexed vertex, its normal rotated and normalised, its tangent
    /// rotated with w kept, and the previous position less the new one.
    #[test]
    fn a_part_deform_writes_its_streams_by_index() {
        let turn = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let matrices = [skin_rows(turn)];
        let binding = SkinBinding {
            joints: [1, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        };
        let rest = [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0)];
        let bindings = [binding, binding];
        let normals = [Vec3::X, Vec3::Y];
        let normal_indices = [1u32, 0];
        let tangents = [[0.0, 0.0, 1.0, 9.0], [0.0, 1.0, 0.0, 8.0]];
        let previous = [Vec3::new(1.0, 1.0, 1.0), Vec3::new(2.0, 2.0, 2.0)];
        let out = deform_part(
            &[1, 0],
            &rest,
            &bindings,
            &matrices,
            Some((&normals, &normal_indices)),
            Some(&tangents),
            Some(&previous),
        );
        // Vertex 1 (0,2,0) turns to (−2,0,0); vertex 0 (1,0,0) to (0,1,0).
        assert!((out.positions[0] - Vec3::new(-2.0, 0.0, 0.0)).length() < 1e-6);
        assert!((out.positions[1] - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-6);
        // The normals follow the per-index normal source: index 0 takes
        // normal 1 (+y → −x), index 1 takes normal 0 (+x → +y).
        assert!((out.normals.as_ref().unwrap()[0] - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-6);
        assert!((out.normals.as_ref().unwrap()[1] - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-6);
        assert_eq!(out.tangents.as_ref().unwrap()[0], [0.0, 0.0, 1.0, 9.0]);
        assert_eq!(out.tangents.as_ref().unwrap()[1][3], 8.0);
        // The delta is the old position less the new one.
        assert!((out.deltas.as_ref().unwrap()[0] - Vec3::new(3.0, 1.0, 1.0)).length() < 1e-6);
        assert_eq!(element_bytes(6), 24);
    }

    /// The normalisation zeroes a zero direction and leaves a NaN alone; a
    /// unit axis stays itself.
    #[test]
    fn normals_normalise_with_the_zero_guard() {
        assert_eq!(normalised(Vec3::ZERO), Vec3::ZERO);
        assert!(normalised(Vec3::new(f32::NAN, 0.0, 0.0)).x.is_nan());
        assert_eq!(normalised(Vec3::new(0.0, 4.0, 0.0)), Vec3::Y);
    }
}
