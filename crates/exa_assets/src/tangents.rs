//! Tangent generation, as the original's `SplitTangents` does it: one
//! tangent and one bitangent accumulate per vertex over the triangles that
//! use it, weighted by the triangle's UV area; then the tangent is made
//! perpendicular to the surface normal, normalised, and given the original's
//! handedness sign.
//!
//! For a triangle with positions `p0,p1,p2` and texture coordinates
//! `q0,q1,q2`:
//!
//! ```text
//! dp1 = p1 − p0, dp2 = p2 − p0, duv1 = q1 − q0, duv2 = q2 − q0
//! det = duv1.x·duv2.y − duv2.x·duv1.y
//! t   = (duv2.y·dp1 − duv1.y·dp2) / det
//! b   = (duv1.x·dp2 − duv2.x·dp1) / det
//! ```
//!
//! A determinant below `1e-10` (compared in f64) uses an inverse of one
//! instead. The final tangent is
//! `normalize(t − n·(n·t))`, and its fourth component is `−1` when
//! `n · (t × b) ≥ 0`, else `1`: the original's bitangent runs along −v.
//!
//! Evidence: reverse engineering of the tangent splitter and its bit-exact
//! float order (REA); [`docs/RENDERING.md`](../../../docs/RENDERING.md)
//! (Surfaces).

/// The determinant below which the splitter uses an inverse of one. The
/// original compares the f32 determinant after widening it to f64.
pub const DETERMINANT_FLOOR: f64 = 1e-10;

/// One vertex's tangent and handedness, in the original's sign convention.
pub type Tangent = [f32; 4];

/// The per-vertex tangents of a mesh with positions, surface normals, one
/// texture-coordinate channel and a triangle list. Returns `None` when any
/// array is too short for the indices (a malformed mesh).
pub fn split_tangents(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    indices: &[u32],
) -> Option<Vec<Tangent>> {
    let count = positions.len();
    if normals.len() < count || uvs.len() < count || !indices.len().is_multiple_of(3) {
        return None;
    }
    let mut tangents = vec![[0.0f32; 3]; count];
    let mut bitangents = vec![[0.0f32; 3]; count];
    for triangle in indices.as_chunks::<3>().0 {
        let [a, b, c] = triangle.map(|i| i as usize);
        if a >= count || b >= count || c >= count {
            return None;
        }
        let p0 = positions[a];
        let p1 = positions[b];
        let p2 = positions[c];
        let q0 = uvs[a];
        let q1 = uvs[b];
        let q2 = uvs[c];
        let dp1 = sub3(p1, p0);
        let dp2 = sub3(p2, p0);
        let duv1 = [q1[0] - q0[0], q1[1] - q0[1]];
        let duv2 = [q2[0] - q0[0], q2[1] - q0[1]];
        let det = duv1[0] * duv2[1] - duv2[0] * duv1[1];
        let inverse = if (f64::from(det.abs())) < DETERMINANT_FLOOR {
            1.0
        } else {
            1.0 / det
        };
        let tangent = [
            (duv2[1] * dp1[0] - duv1[1] * dp2[0]) * inverse,
            (duv2[1] * dp1[1] - duv1[1] * dp2[1]) * inverse,
            (duv2[1] * dp1[2] - duv1[1] * dp2[2]) * inverse,
        ];
        let bitangent = [
            (duv1[0] * dp2[0] - duv2[0] * dp1[0]) * inverse,
            (duv1[0] * dp2[1] - duv2[0] * dp1[1]) * inverse,
            (duv1[0] * dp2[2] - duv2[0] * dp1[2]) * inverse,
        ];
        for vertex in [a, b, c] {
            add3(&mut tangents[vertex], tangent);
            add3(&mut bitangents[vertex], bitangent);
        }
    }
    let mut out = Vec::with_capacity(count);
    for vertex in 0..count {
        let n = normals[vertex];
        let t = tangents[vertex];
        // The original subtracts the normal's share in this order:
        // z, then y, then x.
        let share = -(t[2] * n[2] + (t[1] * n[1] + t[0] * n[0]));
        let mut tangent = [
            t[0] + n[0] * share,
            t[1] + n[1] * share,
            t[2] + n[2] * share,
        ];
        let length =
            (tangent[0] * tangent[0] + tangent[1] * tangent[1] + tangent[2] * tangent[2]).sqrt();
        tangent = [
            tangent[0] / length,
            tangent[1] / length,
            tangent[2] / length,
        ];
        let b = bitangents[vertex];
        let cross = [
            n[1] * tangent[2] - n[2] * tangent[1],
            n[2] * tangent[0] - n[0] * tangent[2],
            n[0] * tangent[1] - n[1] * tangent[0],
        ];
        let handedness = cross[0] * b[0] + cross[1] * b[1] + cross[2] * b[2];
        out.push([
            tangent[0],
            tangent[1],
            tangent[2],
            if handedness >= 0.0 { -1.0 } else { 1.0 },
        ]);
    }
    Some(out)
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add3(target: &mut [f32; 3], add: [f32; 3]) {
    target[0] += add[0];
    target[1] += add[1];
    target[2] += add[2];
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit square in the XY plane with +Z normals and a standard UV
    /// layout: both triangles agree on +X tangents and the original's −1
    /// handedness.
    #[test]
    fn a_flat_quad_gets_x_tangents_and_the_original_sign() {
        let positions = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let normals = [[0.0, 0.0, 1.0]; 4];
        let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let tangents = split_tangents(&positions, &normals, &uvs, &[0, 1, 2, 0, 2, 3]).unwrap();
        for tangent in tangents {
            assert_eq!(tangent, [1.0, 0.0, 0.0, -1.0]);
        }
    }

    /// Mirroring the texture across u flips the tangent and the handedness.
    #[test]
    fn a_mirrored_uv_flips_the_tangent_and_the_sign() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = [[0.0, 0.0, 1.0]; 3];
        let uvs = [[1.0, 0.0], [0.0, 0.0], [1.0, 1.0]];
        let tangents = split_tangents(&positions, &normals, &uvs, &[0, 1, 2]).unwrap();
        for tangent in tangents {
            assert_eq!(tangent, [-1.0, 0.0, 0.0, 1.0]);
        }
    }

    /// A degenerate UV triangle uses the inverse of one, so its tangent is
    /// `duv2.y·dp1 − duv1.y·dp2` unscaled.
    #[test]
    fn a_degenerate_uv_uses_an_inverse_of_one() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = [[0.0, 0.0, 1.0]; 3];
        let uvs = [[0.0, 0.0]; 3];
        let tangents = split_tangents(&positions, &normals, &uvs, &[0, 1, 2]).unwrap();
        // Zero tangents normalise to NaN, as the original's divide does.
        assert!(tangents[0][0].is_nan());
    }

    #[test]
    fn a_malformed_index_list_is_refused() {
        let positions = [[0.0; 3]; 2];
        let normals = [[0.0; 3]; 2];
        let uvs = [[0.0; 2]; 2];
        assert!(split_tangents(&positions, &normals, &uvs, &[0, 1, 2]).is_none());
        assert!(split_tangents(&positions, &normals, &uvs, &[0, 1]).is_none());
    }
}
