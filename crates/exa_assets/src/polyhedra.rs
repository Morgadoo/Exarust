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
}
