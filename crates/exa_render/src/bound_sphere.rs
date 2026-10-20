//! The smallest sphere around a point set, as the original fits the view of
//! its sun shadow map: the points are widened to f64, the run grows a
//! sphere from a support set of one to four points by move-to-front, and the
//! result is narrowed back to f32.
//!
//! - The sphere starts at the first point with radius 0 and the support set
//!   {0}. The scan visits the points in order; a point already in the
//!   support set, or one whose squared distance to the centre minus r² is
//!   at most 1e-5, is skipped. Otherwise the support set and the point give
//!   a new sphere (below). When the new r² is above the old, the sphere is
//!   replaced and the scan restarts at point 0; either way the support set
//!   keeps what the step made of it.
//! - Candidates: with support points P and the new point q, every sphere
//!   through q and one, two or three of P, in the original's fixed orders
//!   (the support tables below). A candidate is
//!   considered only when its r² is below the best so far; it becomes the
//!   best when it holds the other support points (the 1e-5 slack), else it
//!   is remembered when its excess (the failed point's squared distance
//!   minus r²) is the smallest yet. The best wins, else the least-excess
//!   one; the support set becomes the winner's points then q.
//! - Two points: the midpoint, r² = d²/4. Three: the circumcentre in their
//!   plane, or the sphere on an edge when the centre falls outside the
//!   triangle, r² = 3.4e38 when the triangle is flat (|2·det| ≤ 1e-7).
//!   Four: the circumcentre by a 3×3 solve, centre 0 and r² = 3.4e38 when
//!   |det| ≤ 1e-7.
//! - No points: only the radius is written, 4000 in the points' units.
//!
//! The original switches the x87 control word around the run; its maths is
//! SSE2 double precision, which that word does not reach, so plain `f64`
//! arithmetic in the same order gives the same bits. Rules:
//! `docs/RENDERING.md` (Sun shadow fit).

use bevy::math::{DVec3, Vec3};

/// The radius written when there are no points.
pub const EMPTY_RADIUS: f32 = 4000.0;
/// How far (in squared units) a point may sit outside a sphere and still
/// count as inside.
pub const INSIDE_SLACK: f64 = 1e-5;
/// The determinant at or below which three or four points count as flat.
pub const FLAT: f64 = 1e-7;
/// The r² of a flat triangle or tetrahedron, and the start of every
/// "smallest so far" (an f32 constant widened to f64).
pub const NO_SPHERE: f32 = 3.4e38;

/// A bounding sphere, as the shadow fit reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub centre: Vec3,
    pub radius: f32,
}

/// A sphere of the run: centre and squared radius in f64.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Ball {
    pub centre: DVec3,
    pub radius_sq: f64,
}

/// The smallest sphere around `points`; `None` without points.
// `!(a > b)` on purpose: a NaN distance counts as inside, as in the original.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn bounding_sphere(points: &[Vec3]) -> Option<Sphere> {
    let first = *points.first()?;
    let points = widen(points);
    let mut support = Support {
        len: 1,
        points: [0; 4],
    };
    let mut ball = Ball {
        centre: first.as_dvec3(),
        radius_sq: 0.0,
    };
    let mut i = 1;
    while i < points.len() {
        if support.has(i) || !(excess(points[i], &ball) > INSIDE_SLACK) {
            i += 1;
            continue;
        }
        let grown = match support.len {
            1 => grow_from_one(&mut support, &points, i),
            2 => grow_from_two(&mut support, &points, i),
            3 => grow_from_more(&mut support, &points, i, &THREE_SUPPORT),
            _ => grow_from_more(&mut support, &points, i, &FOUR_SUPPORT),
        };
        if grown.radius_sq > ball.radius_sq {
            ball = grown;
            i = 0;
        } else {
            i += 1;
        }
    }
    Some(Sphere {
        centre: narrow(ball.centre),
        radius: ball.radius_sq.sqrt() as f32,
    })
}

/// Fits `sphere` around `points` as the original writes its result: both
/// fields, or only the radius ([`EMPTY_RADIUS`]) when there are no points.
pub fn fit_bounding_sphere(points: &[Vec3], sphere: &mut Sphere) {
    match bounding_sphere(points) {
        Some(fitted) => *sphere = fitted,
        None => sphere.radius = EMPTY_RADIUS,
    }
}

/// The points widened to f64, component by component.
pub fn widen(points: &[Vec3]) -> Vec<DVec3> {
    points.iter().map(|p| p.as_dvec3()).collect()
}

/// A point narrowed to f32, component by component (round to nearest).
pub fn narrow(v: DVec3) -> Vec3 {
    Vec3::new(v.x as f32, v.y as f32, v.z as f32)
}

/// `a + (b − a)·t`, per component as `(b − a)·t + a`.
pub fn lerp(a: DVec3, b: DVec3, t: f64) -> DVec3 {
    DVec3::new(
        (b.x - a.x) * t + a.x,
        (b.y - a.y) * t + a.y,
        (b.z - a.z) * t + a.z,
    )
}

/// `(x + y) + z` of the products, the original's order.
fn dot(a: DVec3, b: DVec3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// Squared distance of `a − b`.
fn distance_sq(a: DVec3, b: DVec3) -> f64 {
    let (x, y, z) = (a.x - b.x, a.y - b.y, a.z - b.z);
    x * x + y * y + z * z
}

/// How far `p` is outside `ball`: squared distance minus r².
fn excess(p: DVec3, ball: &Ball) -> f64 {
    distance_sq(p, ball.centre) - ball.radius_sq
}

/// Whether `p` is inside `ball` (with the slack), and its excess.
fn holds(p: DVec3, ball: &Ball) -> (bool, f64) {
    let e = excess(p, ball);
    (e <= INSIDE_SLACK, e)
}

/// The sphere with `a` and `b` at the ends of a diameter.
pub fn sphere_of_two(a: DVec3, b: DVec3) -> Ball {
    Ball {
        centre: lerp(a, b, 0.5),
        radius_sq: distance_sq(a, b) * 0.25,
    }
}

/// The smallest sphere through `a`, `b` and `c` written into `out`: the
/// circumcentre `a + s·(b − a) + t·(c − a)` while `s > 0`, `t > 0` and
/// `s + t < 1`, otherwise the edge sphere of `a`–`c` (`s ≤ 0`), `a`–`b`
/// (`t ≤ 0`) or `b`–`c`. A flat triangle sets only r² ([`NO_SPHERE`]) and
/// leaves the centre as it was.
pub fn circle_of_three(out: &mut Ball, a: DVec3, b: DVec3, c: DVec3) {
    let e1 = b - a;
    let e2 = c - a;
    let d11 = dot(e1, e1);
    let d12 = dot(e1, e2);
    let d22 = dot(e2, e2);
    let den = (d11 * d22 - d12 * d12) * 2.0;
    if den.abs() <= FLAT {
        out.radius_sq = f64::from(NO_SPHERE);
        return;
    }
    let s = (d11 * d22 - d22 * d12) / den;
    let t = (d22 * d11 - d11 * d12) / den;
    let on = if s <= 0.0 {
        out.centre = lerp(a, c, 0.5);
        a
    } else if t <= 0.0 {
        out.centre = lerp(a, b, 0.5);
        a
    } else if s + t >= 1.0 {
        out.centre = lerp(b, c, 0.5);
        b
    } else {
        let mid = DVec3::new(e1.x * s + a.x, e1.y * s + a.y, e1.z * s + a.z);
        out.centre = DVec3::new(e2.x * t + mid.x, e2.y * t + mid.y, e2.z * t + mid.z);
        a
    };
    out.radius_sq = distance_sq(out.centre, on);
}

/// The sphere through `a`, `b`, `c` and `d`: with the edges from `d`, the
/// barycentric weights solve `M·λ = ½·diag(M)` for the Gram matrix `M` by
/// its adjugate. A flat tetrahedron gives centre 0 and r² [`NO_SPHERE`].
// `!(a > b)` on purpose: a NaN determinant counts as flat, as in the original.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn sphere_of_four(a: DVec3, b: DVec3, c: DVec3, d: DVec3) -> Ball {
    let edges = [a - d, b - d, c - d];
    let mut m = [[0.0f64; 3]; 3];
    for (r, row) in m.iter_mut().enumerate() {
        for (k, cell) in row.iter_mut().enumerate() {
            *cell = dot(edges[r], edges[k]);
        }
    }
    let rhs = [0.5 * m[0][0], 0.5 * m[1][1], 0.5 * m[2][2]];
    let mut adj = [
        [
            m[1][1] * m[2][2] - m[1][2] * m[2][1],
            m[0][2] * m[2][1] - m[0][1] * m[2][2],
            m[0][1] * m[1][2] - m[0][2] * m[1][1],
        ],
        [
            m[1][2] * m[2][0] - m[1][0] * m[2][2],
            m[0][0] * m[2][2] - m[0][2] * m[2][0],
            m[0][2] * m[1][0] - m[0][0] * m[1][2],
        ],
        [
            m[1][0] * m[2][1] - m[1][1] * m[2][0],
            m[0][1] * m[2][0] - m[0][0] * m[2][1],
            m[0][0] * m[1][1] - m[0][1] * m[1][0],
        ],
    ];
    let det = m[0][0] * adj[0][0] + m[0][1] * adj[1][0] + m[0][2] * adj[2][0];
    if !(det.abs() > FLAT) {
        return Ball {
            centre: DVec3::ZERO,
            radius_sq: f64::from(NO_SPHERE),
        };
    }
    let inv = 1.0 / det;
    for row in &mut adj {
        for cell in row.iter_mut() {
            *cell *= inv;
        }
    }
    let mut weight = [0.0f64; 3];
    for (w, row) in weight.iter_mut().zip(&adj) {
        for (cell, r) in row.iter().zip(&rhs) {
            *w += cell * r;
        }
    }
    let last = 1.0 - weight[0] - weight[1] - weight[2];
    let mut centre = DVec3::new(a.x * weight[0], a.y * weight[0], a.z * weight[0]);
    for (p, w) in [(b, weight[1]), (c, weight[2]), (d, last)] {
        centre = DVec3::new(p.x * w + centre.x, p.y * w + centre.y, p.z * w + centre.z);
    }
    let e = edges[0];
    let mut offset = DVec3::new(e.x * weight[0], e.y * weight[0], e.z * weight[0]);
    for (e, w) in [(edges[1], weight[1]), (edges[2], weight[2])] {
        offset = DVec3::new(e.x * w + offset.x, e.y * w + offset.y, e.z * w + offset.z);
    }
    Ball {
        centre,
        radius_sq: dot(offset, offset),
    }
}

/// The points the current sphere rests on: indices into the point set, in
/// the slots the candidate tables address.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Support {
    len: usize,
    points: [usize; 4],
}

impl Support {
    fn has(&self, i: usize) -> bool {
        self.points[..self.len].contains(&i)
    }
}

/// How a stage's candidates are built from its chosen support points and q.
#[derive(Debug, Clone, Copy)]
enum Through {
    /// One support point and q: [`sphere_of_two`].
    Two,
    /// Two support points and q: [`circle_of_three`].
    Three,
    /// Three support points and q: [`sphere_of_four`].
    Four,
}

/// One kind of candidate: each row names support slots, the first
/// `chosen` build the sphere and the rest must lie inside it.
struct Stage {
    through: Through,
    chosen: usize,
    rows: &'static [[usize; 4]],
}

/// The candidates for a support set of two: each slot with q, then the
/// triangle. Its winner replaces slot 1 (the first row), slot 0 (the
/// second) or joins as slot 2 (the triangle); `grow_from_two` keeps that
/// slot order.
const TWO_SUPPORT: [Stage; 2] = [
    Stage {
        through: Through::Two,
        chosen: 1,
        rows: &[[0, 1, 0, 0], [1, 0, 0, 0]],
    },
    Stage {
        through: Through::Three,
        chosen: 2,
        rows: &[[0, 1, 0, 0]],
    },
];

/// The candidates for a support set of three: pairs with q, triangles with
/// q, then the tetrahedron.
const THREE_SUPPORT: [Stage; 3] = [
    Stage {
        through: Through::Two,
        chosen: 1,
        rows: &[[0, 1, 2, 0], [1, 0, 2, 0], [2, 0, 1, 0]],
    },
    Stage {
        through: Through::Three,
        chosen: 2,
        rows: &[[0, 1, 2, 0], [0, 2, 1, 0], [1, 2, 0, 0]],
    },
    Stage {
        through: Through::Four,
        chosen: 3,
        rows: &[[0, 1, 2, 0]],
    },
];

/// The candidates for a support set of four: 4 pairs, 6 triangles and 4
/// tetrahedra with q.
const FOUR_SUPPORT: [Stage; 3] = [
    Stage {
        through: Through::Two,
        chosen: 1,
        rows: &[[0, 1, 2, 3], [1, 0, 2, 3], [2, 0, 1, 3], [3, 0, 1, 2]],
    },
    Stage {
        through: Through::Three,
        chosen: 2,
        rows: &[
            [0, 1, 2, 3],
            [0, 2, 1, 3],
            [0, 3, 1, 2],
            [1, 2, 0, 3],
            [1, 3, 0, 2],
            [2, 3, 0, 1],
        ],
    },
    Stage {
        through: Through::Four,
        chosen: 3,
        rows: &[[0, 1, 2, 3], [0, 1, 3, 2], [0, 2, 3, 1], [1, 2, 3, 0]],
    },
];

/// The most candidates a step builds (a support set of four).
const MAX_CANDIDATES: usize = 14;

/// A remembered candidate: its index and the support slots it rests on.
#[derive(Debug, Clone, Copy)]
struct Pick {
    index: usize,
    chosen: [usize; 3],
    len: usize,
}

/// Every candidate of `stages`, with the best (holds the other support
/// points) and the least-excess one.
struct Search {
    candidates: [Ball; MAX_CANDIDATES],
    best: Option<Pick>,
    least_excess: Option<Pick>,
}

fn search(stages: &[Stage], support: &Support, points: &[DVec3], q: usize) -> Search {
    let p: [DVec3; 4] = std::array::from_fn(|k| points[support.points[k.min(support.len - 1)]]);
    let q = points[q];
    let mut found = Search {
        candidates: [Ball::default(); MAX_CANDIDATES],
        best: None,
        least_excess: None,
    };
    let mut best_sq = f64::from(NO_SPHERE);
    let mut least = f64::from(NO_SPHERE);
    // The original builds every candidate in one buffer; a flat triangle
    // leaves the previous centre in it.
    let mut scratch = Ball::default();
    let mut index = 0;
    for stage in stages {
        for row in stage.rows {
            match stage.through {
                Through::Two => scratch = sphere_of_two(p[row[0]], q),
                Through::Three => circle_of_three(&mut scratch, p[row[0]], p[row[1]], q),
                Through::Four => scratch = sphere_of_four(p[row[0]], p[row[1]], p[row[2]], q),
            }
            found.candidates[index] = scratch;
            if scratch.radius_sq < best_sq {
                let mut inside = true;
                let mut over = 0.0;
                for &slot in &row[stage.chosen..support.len] {
                    let (holds, e) = holds(p[slot], &scratch);
                    over = e;
                    if !holds {
                        inside = false;
                        break;
                    }
                }
                let mut chosen = [0; 3];
                chosen[..stage.chosen].copy_from_slice(&row[..stage.chosen]);
                let pick = Pick {
                    index,
                    chosen,
                    len: stage.chosen,
                };
                if inside {
                    best_sq = scratch.radius_sq;
                    found.best = Some(pick);
                } else if over < least {
                    least = over;
                    found.least_excess = Some(pick);
                }
            }
            index += 1;
        }
    }
    found
}

/// Support set {s}: the sphere on s and q; the set becomes {s, q}.
fn grow_from_one(support: &mut Support, points: &[DVec3], q: usize) -> Ball {
    let ball = sphere_of_two(points[support.points[0]], points[q]);
    support.len = 2;
    support.points[1] = q;
    ball
}

/// Support set {s0, s1}: the winner of the pairs and the triangle, the
/// first candidate when none qualifies. q takes slot 1 (s0 kept), slot 0
/// (s1 kept) or joins as slot 2.
fn grow_from_two(support: &mut Support, points: &[DVec3], q: usize) -> Ball {
    let found = search(&TWO_SUPPORT, support, points, q);
    let index = found
        .best
        .or(found.least_excess)
        .map_or(0, |pick| pick.index);
    match index {
        0 => support.points[1] = q,
        1 => support.points[0] = q,
        _ => {
            support.len = 3;
            support.points[2] = q;
        }
    }
    found.candidates[index]
}

/// Support set of three or four: the winner of `stages`; the set becomes
/// the winner's support points, in its row's order, then q.
///
/// Exarust's guard: when no candidate qualifies (only with NaN or
/// near-f32-max coordinates) the original reads past its candidate table;
/// here the set stays and the sphere has a NaN r², which the scan never
/// takes.
fn grow_from_more(support: &mut Support, points: &[DVec3], q: usize, stages: &[Stage]) -> Ball {
    let found = search(stages, support, points, q);
    let Some(pick) = found.best.or(found.least_excess) else {
        return Ball {
            centre: DVec3::ZERO,
            radius_sq: f64::NAN,
        };
    };
    let kept: [usize; 3] = std::array::from_fn(|k| support.points[pick.chosen[k]]);
    support.points[..pick.len].copy_from_slice(&kept[..pick.len]);
    support.points[pick.len] = q;
    support.len = pick.len + 1;
    found.candidates[pick.index]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(x: f64, y: f64, z: f64) -> DVec3 {
        DVec3::new(x, y, z)
    }

    #[test]
    fn constants_match_the_original() {
        assert_eq!(NO_SPHERE.to_bits(), 0x7f7f_c99e);
        assert_eq!(f64::from(NO_SPHERE), 3.3999999521443642e38);
        assert_eq!(INSIDE_SLACK.to_bits(), 0x3ee4_f8b5_88e3_68f1);
        assert_eq!(FLAT.to_bits(), 0x3e7a_d7f2_9abc_af48);
        assert_eq!(EMPTY_RADIUS.to_bits(), 0x457a_0000);
    }

    #[test]
    fn no_points_writes_only_the_radius() {
        assert_eq!(bounding_sphere(&[]), None);
        let mut sphere = Sphere {
            centre: Vec3::new(1.0, 2.0, 3.0),
            radius: 5.0,
        };
        fit_bounding_sphere(&[], &mut sphere);
        assert_eq!(sphere.centre, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(sphere.radius, 4000.0);
    }

    #[test]
    fn one_point_is_a_zero_sphere() {
        let s = bounding_sphere(&[Vec3::new(1.5, -2.0, 3.0)]).unwrap();
        assert_eq!(s.centre, Vec3::new(1.5, -2.0, 3.0));
        assert_eq!(s.radius, 0.0);
    }

    #[test]
    fn vector_helpers() {
        assert_eq!(
            widen(&[Vec3::new(0.1, 1.0, -2.5)]),
            vec![d(f64::from(0.1f32), 1.0, -2.5)]
        );
        assert_eq!(
            narrow(d(0.1, 1.0, 1.0 / 3.0)),
            Vec3::new(0.1, 1.0, 1.0 / 3.0)
        );
        assert_eq!(
            lerp(d(1.0, 2.0, 3.0), d(3.0, 6.0, -1.0), 0.5),
            d(2.0, 4.0, 1.0)
        );
    }

    #[test]
    fn two_points_round_through_f64() {
        // r² = 3·0.25 = 0.75; √0.75 in f64, then narrowed.
        let s = bounding_sphere(&[Vec3::ZERO, Vec3::ONE]).unwrap();
        assert_eq!(s.centre, Vec3::splat(0.5));
        assert_eq!(s.radius, 0.75f64.sqrt() as f32);
        assert_eq!(s.radius.to_bits(), 0x3f5d_b3d7);
    }

    #[test]
    fn three_points_interior_and_edges() {
        let mut out = Ball::default();
        // e1 = (2,0,0), e2 = (1,2,0): d11 4, d12 2, d22 5, den 32,
        // s = 10/32, t = 12/32: centre (1, 0.75, 0), r² 1.5625.
        circle_of_three(
            &mut out,
            d(0.0, 0.0, 0.0),
            d(2.0, 0.0, 0.0),
            d(1.0, 2.0, 0.0),
        );
        assert_eq!(
            out,
            Ball {
                centre: d(1.0, 0.75, 0.0),
                radius_sq: 1.5625
            }
        );
        // t = (32 − 64)/32 = −1: the a–b edge.
        circle_of_three(
            &mut out,
            d(0.0, 0.0, 0.0),
            d(4.0, 0.0, 0.0),
            d(1.0, 1.0, 0.0),
        );
        assert_eq!(
            out,
            Ball {
                centre: d(2.0, 0.0, 0.0),
                radius_sq: 4.0
            }
        );
        // s = −1: the a–c edge.
        circle_of_three(
            &mut out,
            d(0.0, 0.0, 0.0),
            d(1.0, 1.0, 0.0),
            d(4.0, 0.0, 0.0),
        );
        assert_eq!(
            out,
            Ball {
                centre: d(2.0, 0.0, 0.0),
                radius_sq: 4.0
            }
        );
        // A right angle at a: s = t = ½, s + t = 1, the b–c edge.
        circle_of_three(
            &mut out,
            d(0.0, 0.0, 0.0),
            d(4.0, 0.0, 0.0),
            d(0.0, 3.0, 0.0),
        );
        assert_eq!(
            out,
            Ball {
                centre: d(2.0, 1.5, 0.0),
                radius_sq: 6.25
            }
        );
    }

    #[test]
    fn flat_triangle_keeps_the_centre() {
        let mut out = Ball {
            centre: d(7.0, 8.0, 9.0),
            radius_sq: 1.0,
        };
        circle_of_three(
            &mut out,
            d(0.0, 0.0, 0.0),
            d(1.0, 0.0, 0.0),
            d(2.0, 0.0, 0.0),
        );
        assert_eq!(out.centre, d(7.0, 8.0, 9.0));
        assert_eq!(out.radius_sq, 3.3999999521443642e38);
    }

    #[test]
    fn four_points() {
        // Edges from d: 2·axes; M = 4·I, λ = ½ each, the last −½.
        let b = sphere_of_four(
            d(2.0, 0.0, 0.0),
            d(0.0, 2.0, 0.0),
            d(0.0, 0.0, 2.0),
            d(0.0, 0.0, 0.0),
        );
        assert_eq!(
            b,
            Ball {
                centre: d(1.0, 1.0, 1.0),
                radius_sq: 3.0
            }
        );
        let flat = sphere_of_four(
            d(1.0, 0.0, 0.0),
            d(0.0, 1.0, 0.0),
            d(1.0, 1.0, 0.0),
            d(0.0, 0.0, 0.0),
        );
        assert_eq!(
            flat,
            Ball {
                centre: DVec3::ZERO,
                radius_sq: 3.3999999521443642e38
            }
        );
    }

    #[test]
    fn two_support_keeps_its_slot_order() {
        // (0,0,0), (4,0,0), then (0,3,0): the pair on slot 0 misses (4,0,0)
        // by 16, the pair on slot 1 holds (0,0,0) with r² 6.25, the
        // triangle is its b–c edge, also 6.25, not smaller. q takes slot 0.
        let points = widen(&[
            Vec3::ZERO,
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
        ]);
        let mut support = Support {
            len: 2,
            points: [0, 1, 0, 0],
        };
        let ball = grow_from_two(&mut support, &points, 2);
        assert_eq!(
            ball,
            Ball {
                centre: d(2.0, 1.5, 0.0),
                radius_sq: 6.25
            }
        );
        assert_eq!(
            support,
            Support {
                len: 2,
                points: [2, 1, 0, 0]
            }
        );
        let s = bounding_sphere(&[
            Vec3::ZERO,
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
        ])
        .unwrap();
        assert_eq!(
            s,
            Sphere {
                centre: Vec3::new(2.0, 1.5, 0.0),
                radius: 2.5
            }
        );
    }

    #[test]
    fn three_support_takes_the_tetrahedron() {
        // The regular tetrahedron's support of three plus its fourth corner:
        // every pair and triangle misses a point, the tetrahedron (r² 3)
        // wins and the set becomes all four, q last.
        let points = widen(&[
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
        ]);
        let mut support = Support {
            len: 3,
            points: [0, 1, 2, 0],
        };
        let ball = grow_from_more(&mut support, &points, 3, &THREE_SUPPORT);
        assert!((ball.radius_sq - 3.0).abs() < 1e-12);
        assert!(ball.centre.length() < 1e-12);
        assert_eq!(
            support,
            Support {
                len: 4,
                points: [0, 1, 2, 3]
            }
        );
    }

    #[test]
    fn four_support_falls_back_to_a_smaller_set() {
        // Four corners of a regular tetrahedron and a far point on +z: the
        // winner rests on fewer support points, q last.
        let points = widen(&[
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(0.0, 0.0, 10.0),
        ]);
        let mut support = Support {
            len: 4,
            points: [0, 1, 2, 3],
        };
        // The smallest candidate holding the rest is the triangle on slots
        // 1 and 2 with q (the fourth triangle row): centre (0, 0, 97/22),
        // r² 31.258…; the set becomes those two, then q.
        let ball = grow_from_more(&mut support, &points, 4, &FOUR_SUPPORT);
        assert_eq!(
            ball,
            Ball {
                centre: d(0.0, 0.0, 4.409090909090909),
                radius_sq: 31.25826446280992
            }
        );
        assert_eq!(
            support,
            Support {
                len: 3,
                points: [1, 2, 4, 3]
            }
        );
        for p in &points {
            assert!(excess(*p, &ball) <= INSIDE_SLACK);
        }
    }

    #[test]
    fn two_support_without_a_candidate_takes_the_first() {
        // Both pairs have r² 2.5e39, above the 3.4e38 start, and the
        // triangle is flat: no candidate qualifies, so the pair on slot 0
        // is taken and q replaces slot 1.
        let points = vec![d(0.0, 0.0, 0.0), d(1.0, 0.0, 0.0), d(1e20, 0.0, 0.0)];
        let mut support = Support {
            len: 2,
            points: [0, 1, 0, 0],
        };
        let ball = grow_from_two(&mut support, &points, 2);
        assert_eq!(
            ball,
            Ball {
                centre: d(5e19, 0.0, 0.0),
                radius_sq: 1e20 * 1e20 * 0.25
            }
        );
        assert_eq!(
            support,
            Support {
                len: 2,
                points: [0, 2, 0, 0]
            }
        );
    }

    /// An independent smallest sphere: every sphere on 2–4 points, the
    /// smallest that holds them all.
    fn brute_force(points: &[DVec3]) -> f64 {
        let n = points.len();
        let mut best = f64::INFINITY;
        let mut keep = |c: DVec3, r2: f64| {
            if r2 < best
                && points
                    .iter()
                    .all(|p| p.distance_squared(c) <= r2 * (1.0 + 1e-9))
            {
                best = r2;
            }
        };
        for i in 0..n {
            for j in i + 1..n {
                let c = (points[i] + points[j]) * 0.5;
                keep(c, c.distance_squared(points[i]));
                for k in j + 1..n {
                    let (a, b, c3) = (points[i], points[j], points[k]);
                    let (ab, ac) = (b - a, c3 - a);
                    let n3 = ab.cross(ac);
                    if n3.length_squared() > 1e-12 {
                        let o = a
                            + (n3.cross(ab) * ac.length_squared()
                                + ac.cross(n3) * ab.length_squared())
                                / (2.0 * n3.length_squared());
                        keep(o, o.distance_squared(a));
                    }
                    for &dd in &points[k + 1..] {
                        let rows = [b - a, c3 - a, dd - a];
                        let rhs = [
                            (b.length_squared() - a.length_squared()) * 0.5,
                            (c3.length_squared() - a.length_squared()) * 0.5,
                            (dd.length_squared() - a.length_squared()) * 0.5,
                        ];
                        let det = rows[0].dot(rows[1].cross(rows[2]));
                        if det.abs() > 1e-9 {
                            let o = (rows[1].cross(rows[2]) * rhs[0]
                                + rows[2].cross(rows[0]) * rhs[1]
                                + rows[0].cross(rows[1]) * rhs[2])
                                / det;
                            keep(o, o.distance_squared(a));
                        }
                    }
                }
            }
        }
        best
    }

    #[test]
    fn matches_an_exhaustive_search() {
        let mut seed = 0x2545_f491u32;
        let mut next = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 20.0 - 10.0
        };
        for _ in 0..20 {
            let points: Vec<Vec3> = (0..12).map(|_| Vec3::new(next(), next(), next())).collect();
            let s = bounding_sphere(&points).unwrap();
            let wide = widen(&points);
            let exact = brute_force(&wide).sqrt();
            assert!(
                (f64::from(s.radius) - exact).abs() < 1e-4 * exact,
                "{} vs {exact}",
                s.radius
            );
            for p in &points {
                assert!(p.distance(s.centre) <= s.radius * (1.0 + 1e-5));
            }
        }
    }

    #[test]
    fn seeded_sets_to_the_bit() {
        // The first three 12-point sets of the exhaustive check, worked
        // through a separate port of the original's routines: centre and
        // radius bits.
        let mut seed = 0x2545_f491u32;
        let mut next = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 20.0 - 10.0
        };
        let expected: [([u32; 3], u32); 3] = [
            ([0x3eee_1486, 0x4099_d098, 0x3f06_bf16], 0x413b_4436),
            ([0x3f9b_c4cf, 0x3e66_2895, 0xbf24_77d2], 0x4130_c84f),
            ([0xbefb_46be, 0x3f37_75ca, 0x403c_ccff], 0x4143_8d48),
        ];
        for (centre, radius) in expected {
            let points: Vec<Vec3> = (0..12).map(|_| Vec3::new(next(), next(), next())).collect();
            let s = bounding_sphere(&points).unwrap();
            assert_eq!(s.centre.to_array().map(f32::to_bits), centre);
            assert_eq!(s.radius.to_bits(), radius);
        }
    }

    #[test]
    fn cube_corners() {
        let points: Vec<Vec3> = (0..8)
            .map(|i| {
                Vec3::new(
                    if i & 1 == 0 { -1.0 } else { 1.0 },
                    if i & 2 == 0 { -1.0 } else { 1.0 },
                    if i & 4 == 0 { -1.0 } else { 1.0 },
                )
            })
            .collect();
        let s = bounding_sphere(&points).unwrap();
        assert!(s.centre.length() < 1e-6);
        assert!((s.radius - 3f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn points_within_the_slack_do_not_grow_it() {
        // (0,0,0)–(2,0,0) gives centre (1,0,0), r² 1; (1, 1.000004, 0) is
        // 1.000008 away squared, 8e-6 over, inside the 1e-5 slack.
        let s = bounding_sphere(&[
            Vec3::ZERO,
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(1.0, 1.000004, 0.0),
        ])
        .unwrap();
        assert_eq!(
            s,
            Sphere {
                centre: Vec3::new(1.0, 0.0, 0.0),
                radius: 1.0
            }
        );
    }

    #[test]
    fn nan_points_end_the_run() {
        let s = bounding_sphere(&[Vec3::ZERO, Vec3::NAN, Vec3::new(2.0, 0.0, 0.0)]).unwrap();
        assert_eq!(s.radius, 1.0);
    }
}
