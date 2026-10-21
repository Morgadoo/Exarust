//! Polyhedra for the round effect shells, as the original builds them: the
//! bubble and force-bubble effects start from an icosahedron, the force
//! barrier from its five-face top cap, and each is subdivided three times.
//!
//! The icosahedron has a point at each pole and two rings of five at
//! latitude ±atan ½, the lower ring turned by 36°. Subdividing splits every
//! edge at its midpoint pushed out to unit length and every face into four
//! (the three corner faces, then the middle one). Edges are kept with the
//! faces so each midpoint is made once. Evidence: `docs/RENDERING.md`
//! (Effect shells).

use crate::mesh_edit::{NEXT, PREV};

/// The latitude of the rings: atan ½ as the original stores it (a double
/// with a single's precision).
const RING_LATITUDE: f64 = f64::from_bits(0x3FDD_AC67_0000_0000);
/// The angle between ring points, 2π/5 as a single.
const RING_STEP: f32 = 1.256_637_1;
/// The lower ring's first angle, π/5 as a single.
const LOWER_RING_START: f32 = 0.628_318_55;

/// The icosahedron's faces: five around the top, ten around the middle,
/// five around the bottom.
pub const ICOSAHEDRON_FACES: [[u32; 3]; 20] = [
    [0, 2, 1],
    [0, 3, 2],
    [0, 4, 3],
    [0, 5, 4],
    [0, 1, 5],
    [1, 2, 6],
    [2, 3, 7],
    [3, 4, 8],
    [4, 5, 9],
    [5, 1, 10],
    [6, 2, 7],
    [7, 3, 8],
    [8, 4, 9],
    [9, 5, 10],
    [10, 1, 6],
    [11, 6, 7],
    [11, 7, 8],
    [11, 8, 9],
    [11, 9, 10],
    [11, 10, 6],
];

/// A triangle mesh with its edges: each face names the edge before each
/// of its corners.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyhedron {
    /// As many points as the original counts. After subdividing an open
    /// shape (the cap) the count runs past the points made, which stay at
    /// the origin.
    pub points: Vec<[f32; 3]>,
    pub faces: Vec<[u32; 3]>,
    pub face_edges: Vec<[u32; 3]>,
    pub edges: Vec<[u32; 2]>,
}

/// A point of a ring at `latitude`, angle `angle` round the vertical.
fn ring_point(latitude: f64, angle: f32, radius: f32) -> [f32; 3] {
    let a = angle as f64;
    let r = radius as f64;
    [
        (latitude.cos() * a.sin() * r) as f32,
        (latitude.sin() * r) as f32,
        (latitude.cos() * a.cos() * -r) as f32,
    ]
}

/// The edges of `faces`, each made by the first face to reach it: the
/// edge before each corner is looked up the other way round among those
/// made so far, else added.
fn link_edges(faces: &[[u32; 3]]) -> (Vec<[u32; 3]>, Vec<[u32; 2]>) {
    let mut edges: Vec<[u32; 2]> = Vec::new();
    let mut face_edges = vec![[0u32; 3]; faces.len()];
    for (f, t) in faces.iter().enumerate() {
        for k in 0..3 {
            let p = PREV[k];
            face_edges[f][k] = match edges.iter().position(|e| t[k] == e[0] && t[p] == e[1]) {
                Some(e) => e as u32,
                None => {
                    edges.push([t[p], t[k]]);
                    (edges.len() - 1) as u32
                }
            };
        }
    }
    (face_edges, edges)
}

fn upper_ring(points: &mut Vec<[f32; 3]>, radius: f32) {
    let mut angle = 0.0f32;
    for _ in 0..5 {
        points.push(ring_point(RING_LATITUDE, angle, radius));
        angle += RING_STEP;
    }
}

impl Polyhedron {
    /// The icosahedron of `radius` (1 in every use).
    pub fn icosahedron(radius: f32) -> Self {
        let mut points = vec![[0.0, radius, 0.0]];
        upper_ring(&mut points, radius);
        let mut angle = LOWER_RING_START;
        for _ in 0..5 {
            points.push(ring_point(-RING_LATITUDE, angle, radius));
            angle += RING_STEP;
        }
        points.push([0.0, -radius, 0.0]);
        let faces = ICOSAHEDRON_FACES.to_vec();
        let (face_edges, edges) = link_edges(&faces);
        Self {
            points,
            faces,
            face_edges,
            edges,
        }
    }

    /// The icosahedron's top cap: the pole, the upper ring and the five
    /// faces between them.
    pub fn cap(radius: f32) -> Self {
        let mut points = vec![[0.0, radius, 0.0]];
        upper_ring(&mut points, radius);
        let faces = ICOSAHEDRON_FACES[..5].to_vec();
        let (face_edges, edges) = link_edges(&faces);
        Self {
            points,
            faces,
            face_edges,
            edges,
        }
    }

    /// Splits every face into four. Point count becomes 4n − 6 (exact for
    /// closed shapes), edges 2e + 3f, faces 4f.
    pub fn subdivided(&self) -> Self {
        let n = self.points.len();
        let mut points = self.points.clone();
        points.resize((4 * n).saturating_sub(6).max(n), [0.0; 3]);
        let mut next_point = n;
        // Per old edge: its midpoint (0 until made) and its two halves.
        let mut split = vec![(0u32, 0u32, 0u32); self.edges.len()];
        let mut edges = Vec::with_capacity(2 * self.edges.len() + 3 * self.faces.len());
        let mut faces = Vec::with_capacity(4 * self.faces.len());
        let mut face_edges = Vec::with_capacity(4 * self.faces.len());
        for (t, te) in self.faces.iter().zip(&self.face_edges) {
            let mut mid = [0u32; 3];
            let mut halves = [(0u32, 0u32); 3];
            for k in 0..3 {
                let e = te[k] as usize;
                let [a, b] = self.edges[e];
                if split[e].0 == 0 {
                    let m = next_point as u32;
                    edges.push([a, m]);
                    edges.push([m, b]);
                    let half = edges.len() as u32;
                    split[e] = (m, half - 2, half - 1);
                    let p = self.points[a as usize];
                    let q = self.points[b as usize];
                    let lerp: [f32; 3] = std::array::from_fn(|d| p[d] + (q[d] - p[d]) * 0.5);
                    if let Some(slot) = points.get_mut(next_point) {
                        *slot = normalised(lerp);
                    }
                    next_point += 1;
                }
                // The half at the corner before, then the half at the corner.
                halves[k] = if t[k] == b {
                    (split[e].1, split[e].2)
                } else {
                    (split[e].2, split[e].1)
                };
                mid[k] = split[e].0;
            }
            let inner = edges.len() as u32;
            for k in 0..3 {
                let j = NEXT[k];
                faces.push([t[k], mid[j], mid[k]]);
                face_edges.push([halves[k].1, halves[j].0, inner + j as u32]);
            }
            faces.push(mid);
            face_edges.push([inner, inner + 1, inner + 2]);
            for k in 0..3 {
                edges.push([mid[PREV[k]], mid[k]]);
            }
        }
        Self {
            points,
            faces,
            face_edges,
            edges,
        }
    }
}

/// `v` scaled to unit length (zero stays zero). The original uses the
/// processor's approximate reciprocal square root; this is exact.
fn normalised(v: [f32; 3]) -> [f32; 3] {
    let len2 = (v[1] * v[1] + 0.0) + (v[0] * v[0] + v[2] * v[2]);
    if len2 == 0.0 {
        return [0.0; 3];
    }
    let inv = 1.0 / len2.sqrt();
    v.map(|x| x * inv)
}

/// The engine's fast sine: a quadrant fold (multiply by 4/π, round to an
/// even quadrant) and a degree-7 polynomial for the sine or cosine of the
/// remainder. The result is the original's, bit for bit (its final sign is
/// the input's sign xor the quadrant's bit 2).
pub fn fast_sin(x: f32) -> f32 {
    const FOUR_OVER_PI: f32 = 1.273_239_5;
    const QUARTER_PI: f32 = 0.785_398_2;
    let bits = x.to_bits();
    let magnitude = f32::from_bits(bits & 0x7FFF_FFFF);
    let t = magnitude * FOUR_OVER_PI;
    // CVTTSS2SI's out-of-range result is the integer's smallest value.
    let truncated = if t.is_nan() || t >= 2_147_483_648.0 || t < -2_147_483_648.0 {
        i32::MIN
    } else {
        t as i32
    };
    let quadrant = truncated.wrapping_add(1) as u32 & !1;
    let f = magnitude - (quadrant as i32 as f32) * QUARTER_PI;
    let f2 = f * f;
    let sine = (((-0.000_195_152_96 * f2 + 0.008_332_161) * f2 - 0.166_666_55) * f2 * f) + f;
    let cosine =
        (((2.443_315_7e-5 * f2 - 0.001_388_731_6) * f2 + 0.041_666_646) * f2 * f2 - 0.5 * f2) + 1.0;
    let selected = if quadrant & 2 == 0 { sine } else { cosine };
    let sign = (bits & 0x8000_0000) ^ ((quadrant & 4) << 29);
    f32::from_bits(selected.to_bits() ^ sign)
}

/// The UV sphere's attribute mask: per-vertex normals.
pub const SPHERE_NORMALS: u32 = 0x2;
/// The UV sphere's attribute mask: a colour word per vertex (always
/// 0xFFFF_FFFF).
pub const SPHERE_COLOURS: u32 = 0x4;
/// The UV sphere's attribute mask: a tangent (with w = −1) per vertex.
pub const SPHERE_TANGENTS: u32 = 0x8;
/// The UV sphere's attribute mask: texture coordinates per vertex.
pub const SPHERE_UVS: u32 = 0x10;

/// A sphere built as the mind effects' geometry: a column of triangle
/// strips. The vertices are written pair by pair, and a strip's indices
/// point at vertices of the next strip (the original's GL strips), so the
/// list of indices is kept rather than turned into independent triangles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UvSphere {
    pub positions: Vec<[f32; 3]>,
    /// Filled when [`SPHERE_NORMALS`] is set.
    pub normals: Vec<[f32; 3]>,
    /// Filled when [`SPHERE_UVS`] is set.
    pub uvs: Vec<[f32; 2]>,
    /// Filled when [`SPHERE_TANGENTS`] is set.
    pub tangents: Vec<[f32; 4]>,
    /// Filled when [`SPHERE_COLOURS`] is set.
    pub colours: Vec<u32>,
    /// One triangle strip per column pair; its indices name [`positions`].
    pub strips: Vec<Vec<u32>>,
}

/// Builds the UV sphere of `segments` columns (even in every use) and
/// `segments / 2` rows per hemisphere. `u_scale` and `v_scale` tile the
/// texture coordinates; `flags` is the attribute mask (see the
/// `SPHERE_*` constants). The angle step is 2π / segments on both axes,
/// so the rows run from the north pole (row 0) to the south pole
/// (`segments / 2`). An invalid segment count builds nothing.
pub fn uv_sphere(segments: usize, radius: f32, u_scale: f32, v_scale: f32, flags: u32) -> UvSphere {
    let rows = segments / 2;
    if segments == 0 || rows == 0 {
        return UvSphere::default();
    }
    let inv_segments = 1.0f32 / segments as f32;
    let inv_rows = 1.0f32 / rows as f32;
    let mut out = UvSphere::default();
    let write = |col: usize, row: usize, out: &mut UvSphere| -> u32 {
        let index = out.positions.len() as u32;
        let col_angle = (col as f32 * std::f32::consts::TAU) * inv_segments;
        let row_angle = (row as f32 * std::f32::consts::TAU) * inv_segments;
        if row == 0 || row == rows {
            let u = (col as f32 * inv_segments - inv_segments * 0.5) * u_scale;
            let v = row as f32 * -inv_rows * v_scale;
            let cosine = (row_angle as f64).cos() as f32;
            out.positions
                .push([0.0, (radius as f64 * f64::from(cosine)) as f32, 0.0]);
            if flags & SPHERE_NORMALS != 0 {
                out.normals.push([0.0, cosine, 0.0]);
            }
            if flags & SPHERE_UVS != 0 {
                out.uvs.push([u, v]);
            }
        } else {
            let u = col as f32 * inv_segments * u_scale;
            let v = row as f32 * -inv_rows * v_scale;
            let sin_col = fast_sin(col_angle);
            let sin_row = fast_sin(row_angle);
            let cos_col = (col_angle as f64).cos() as f32;
            let cos_row = (row_angle as f64).cos() as f32;
            out.positions.push([
                sin_row * sin_col * radius,
                (radius as f64 * f64::from(cos_row)) as f32,
                radius * sin_row * cos_col,
            ]);
            if flags & SPHERE_TANGENTS != 0 {
                let next_angle = ((col as f32 + 1.0) * std::f32::consts::TAU) * inv_segments;
                let a = [fast_sin(col_angle), 0.0, cos_col];
                let b = [fast_sin(next_angle), 0.0, (next_angle as f64).cos() as f32];
                let d = normalised([b[0] - a[0], b[1] - a[1], b[2] - a[2]]);
                out.tangents.push([d[0], d[1], d[2], -1.0]);
            }
            if flags & SPHERE_NORMALS != 0 {
                out.normals
                    .push([sin_row * sin_col, cos_row, sin_row * cos_col]);
            }
            if flags & SPHERE_UVS != 0 {
                out.uvs.push([u, v]);
            }
        }
        if flags & SPHERE_COLOURS != 0 {
            out.colours.push(0xFFFF_FFFF);
        }
        index
    };

    let mut col = 0usize;
    loop {
        if col >= segments {
            break;
        }
        let mut strip = Vec::new();
        strip.push(write(col, 0, &mut out));
        for row in 1..rows {
            strip.push(write(col, row, &mut out));
            strip.push(write(col + 1, row, &mut out));
        }
        strip.push(write(col, rows, &mut out));
        let next = col + 1;
        if next == segments {
            out.strips.push(strip);
            break;
        }
        let mut continuation = Vec::new();
        let first = write(next, rows, &mut out);
        continuation.push(first);
        if next == segments - 1 {
            // The closing strip: one writer per row, its index stepping
            // back three vertices each time (the writer's own index is
            // replaced by the patched one).
            let mut row = rows;
            while row > 1 {
                row -= 1;
                let vertex = write(next, row, &mut out);
                continuation.push(vertex.wrapping_sub(3 * (rows - row) as u32));
            }
            continuation.push(write(next, 0, &mut out));
        } else {
            // The strip borrows vertices of the next column pair: two
            // patched indices per row, then its own bottom vertex.
            let mut patched = vec![0u32; segments];
            patched[0] = first;
            let mut position = 0usize;
            let mut row = rows;
            while row > 1 {
                row -= 1;
                position += 2;
                patched[position - 1] = first.wrapping_add(1 + 2 * row as u32);
                patched[position] = first.wrapping_sub(2 * (rows - row) as u32);
            }
            let last = write(next, 0, &mut out);
            patched[segments - 1] = last;
            continuation = patched;
        }
        out.strips.push(strip);
        out.strips.push(continuation);
        if next == segments - 1 {
            break;
        }
        col += 2;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn length(p: [f32; 3]) -> f32 {
        (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt()
    }

    #[test]
    fn the_icosahedron_has_twelve_points_on_the_sphere() {
        let ico = Polyhedron::icosahedron(1.0);
        assert_eq!(ico.points.len(), 12);
        assert_eq!(ico.faces.len(), 20);
        assert_eq!(ico.edges.len(), 30);
        for p in &ico.points {
            assert!((length(*p) - 1.0).abs() < 1e-6, "{p:?}");
        }
        // The upper ring sits at sin(atan ½) = 1/√5, its first point at -z.
        assert!((ico.points[1][1] - 0.447_213_6).abs() < 1e-6);
        assert!(ico.points[1][0].abs() < 1e-6 && ico.points[1][2] < 0.0);
        // The lower ring is turned by 36°.
        let a = ico.points[6][0].atan2(-ico.points[6][2]);
        assert!((a - std::f32::consts::PI / 5.0).abs() < 1e-5);
        assert_eq!(ico.points[11], [0.0, -1.0, 0.0]);
    }

    #[test]
    fn every_icosahedron_edge_has_two_faces_the_other_way_round() {
        let ico = Polyhedron::icosahedron(1.0);
        let mut uses = vec![0; ico.edges.len()];
        for (t, te) in ico.faces.iter().zip(&ico.face_edges) {
            for k in 0..3 {
                let e = ico.edges[te[k] as usize];
                let (a, b) = (t[PREV[k]], t[k]);
                assert!(e == [a, b] || e == [b, a]);
                uses[te[k] as usize] += 1;
            }
        }
        assert!(uses.iter().all(|&u| u == 2));
        // The first face makes its edges in corner order.
        assert_eq!(&ico.edges[..3], &[[1, 0], [0, 2], [2, 1]]);
    }

    #[test]
    fn subdividing_three_times_makes_the_bubble_sphere() {
        let mut s = Polyhedron::icosahedron(1.0);
        for _ in 0..3 {
            s = s.subdivided();
        }
        assert_eq!(s.faces.len(), 1280);
        assert_eq!(s.points.len(), 642);
        assert_eq!(s.edges.len(), 1920);
        for p in &s.points {
            assert!((length(*p) - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn a_face_splits_into_its_corners_then_its_middle() {
        let ico = Polyhedron::icosahedron(1.0);
        let one = ico.subdivided();
        // Face 0 (0, 2, 1): its edges before each corner are 0 (1→0),
        // 1 (0→2) and 2 (2→1), split into points 12, 13 and 14.
        assert_eq!(one.faces[0], [0, 13, 12]);
        assert_eq!(one.faces[1], [2, 14, 13]);
        assert_eq!(one.faces[2], [1, 12, 14]);
        assert_eq!(one.faces[3], [12, 13, 14]);
        // Halves: edge 0 (1→0) becomes 1→12 and 12→0, edge 1 (0→2) 0→13
        // and 13→2; the first inner edge runs from the third midpoint to
        // the first.
        assert_eq!(
            &one.edges[..6],
            &[[1, 12], [12, 0], [0, 13], [13, 2], [2, 14], [14, 1]]
        );
        assert_eq!(one.edges[6], [14, 12]);
        assert_eq!(one.face_edges[0], [1, 2, 7]);
        assert_eq!(one.face_edges[3], [6, 7, 8]);
        // Midpoints are on the unit sphere.
        assert!((length(one.points[12]) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_cap_counts_past_its_points_as_the_original_does() {
        let cap = Polyhedron::cap(1.0);
        assert_eq!(
            (cap.points.len(), cap.faces.len(), cap.edges.len()),
            (6, 5, 10)
        );
        let one = cap.subdivided();
        // 4·6 − 6 = 18 counted, 6 + 10 made.
        assert_eq!(one.points.len(), 18);
        assert_eq!(one.faces.len(), 20);
        assert_eq!(one.edges.len(), 35);
        assert_eq!(one.points[16], [0.0; 3]);
        assert!(one.faces.iter().flatten().all(|&p| p < 16));
    }

    #[test]
    fn the_fast_sine_is_the_originals_polynomial() {
        assert_eq!(fast_sin(0.0), 0.0);
        assert_eq!(fast_sin(1.0), 0.841_470_96);
        assert_eq!(fast_sin(0.5), 0.479_425_55);
        assert_eq!(fast_sin(std::f32::consts::FRAC_PI_2), 1.0);
        assert_eq!(fast_sin(-std::f32::consts::FRAC_PI_2), -1.0);
        // The polynomial rounds 2.0 a little above the library sine.
        assert_eq!(fast_sin(2.0), 0.909_297_47);
        // The quadrant fold makes π a signed zero.
        assert_eq!(fast_sin(std::f32::consts::PI).to_bits(), 0x8000_0000);
    }

    #[test]
    fn the_uv_sphere_writes_strips_that_borrow_the_next_columns_vertices() {
        let s = uv_sphere(
            4,
            1.0,
            1.0,
            1.0,
            SPHERE_NORMALS | SPHERE_UVS | SPHERE_TANGENTS | SPHERE_COLOURS,
        );
        assert_eq!(s.positions.len(), 13);
        assert_eq!(
            s.strips,
            vec![
                vec![0, 1, 2, 3],
                vec![4, 7, 2, 5],
                vec![6, 7, 8, 9],
                vec![10, 8, 12],
            ]
        );
        // Poles at ±y, the equator ring at the four quarter turns.
        assert_eq!(s.positions[0], [0.0, 1.0, 0.0]);
        assert_eq!(s.positions[3], [0.0, -1.0, 0.0]);
        assert_eq!(s.positions[1], [0.0, -4.371_139e-8, 1.0]);
        assert_eq!(s.positions[2], [1.0, -4.371_139e-8, -4.371_139e-8]);
        assert_eq!(s.positions[6], [0.0, 1.0, 0.0]);
        assert_eq!(s.positions[8], [-1.0, -4.371_139e-8, 1.192_488_1e-8]);
        // UVs tile once across and once down; poles shift half a column.
        assert_eq!(s.uvs[0], [-0.125, 0.0]);
        assert_eq!(s.uvs[1], [0.0, -0.5]);
        assert_eq!(s.uvs[3], [-0.125, -1.0]);
        assert_eq!(s.uvs[2], [0.25, -0.5]);
        // The tangent runs from a column point to the next round the equator.
        let t = s.tangents[0];
        assert!((t[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        assert!((t[2] + std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        assert_eq!(t[3], -1.0);
        assert!(s.colours.iter().all(|&c| c == 0xFFFF_FFFF));
        for p in &s.positions {
            assert!((length(*p) - 1.0).abs() < 1e-6, "{p:?}");
        }
    }

    #[test]
    fn the_uv_sphere_leaves_out_the_attributes_it_is_not_asked_for() {
        let s = uv_sphere(8, 2.0, 1.0, 1.0, SPHERE_NORMALS);
        assert!(s.uvs.is_empty());
        assert!(s.tangents.is_empty());
        assert!(s.colours.is_empty());
        assert_eq!(s.normals.len(), s.positions.len());
        assert_eq!(s.positions[0], [0.0, 2.0, 0.0]);
        assert!(uv_sphere(0, 1.0, 1.0, 1.0, 0).positions.is_empty());
    }
}
