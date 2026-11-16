//! A mesh's per-triangle query tests, in the original's centimetres: the
//! tests its query slots run on each triangle (the mesh's own brute-force
//! callbacks over every triangle in order, or the kd-tree's leaves), each
//! handing a callback the triangle's closest point and, for the push-out
//! tests, the push. The simulation's contacts ([`crate::contact`]), the
//! pair stage, the Height Query and the effects' contacts all go through
//! them.
//!
//! # What the original does
//!
//! A mesh keeps one record per triangle: the unit normal, the plane's
//! offset, the corners, the first edge's squared length and the reciprocal
//! of the edges' Gram determinant ([`Face`]). Its query slots are:
//!
//! - **The two-sided sphere query** ([`two_sided_point`]): a triangle whose
//!   plane is within the sphere's radius on either side (`|s| ≤ r`; a NaN
//!   distance goes on) and whose closest point is nearer than `r² − ε`
//!   (ε the engine's small float [`SMALL`]; a NaN misses) hands the
//!   callback the sphere and that point.
//! - **The front sphere query** ([`front_push`]): a triangle with the
//!   sphere's centre in front (`0 ≤ s < r`; a NaN goes on) and its closest
//!   point nearer than `r − floor` hands the callback the point and the
//!   push off it: the closest-point direction out to `r`, or the face
//!   normal times `r` when the squared distance is under the floor. The
//!   kd-tree's leaf test takes the floor 1e-6 ([`PUSH_FLOOR`]), the mesh's
//!   own brute-force test 1e-10 ([`BRUTE_PUSH_FLOOR`]).
//! - **The swept query** ([`swept_push`]): the record's start must be in
//!   front and its end within the radius of the plane. An end still clear
//!   of the face is pushed off its closest point as the front query does;
//!   an end that reached the face is measured from where the centre stood
//!   one radius out (or from the start when it began closer), and the push
//!   brings the end back to where the sphere first touched (the closest
//!   point within the sweep, or the ray's entry into the sphere about it),
//!   or, when the closest point lies past the end, the start's offset to
//!   the point scaled by the end's gap, lifted to the face when it would
//!   sink it. The leaf and brute-force forms are one routine.
//! - **The swept nearest query** ([`swept_nearest`]): the Height Query's
//!   and the picks' sweep. Where the centre is one radius in front of the
//!   plane (the sweep's end when it runs along the plane) the closest
//!   point is taken, and the sweep meets it where the ray enters the
//!   sphere of the radius about it, measured from the start:
//!   `q·d − start·d − √(r² − p²)`, `p` the point's distance off the ray.
//!   Each hit shortens the sweep for the next triangle.
//!
//! The closest point is the record's own region test
//! ([`Face::nearest_exact`]). Every compare keeps the instruction's
//! unordered (NaN) branch, noted at each test. Dot products sum `x + y`,
//! then `z`.
//!
//! [inf: the original takes the processor's reciprocal and reciprocal
//! square root estimates in the record and the closest point; Exarust
//! computes them exactly while DEC-7 is open.]
//!
//! Evidence: the mesh constructor's eight query slots, the tree
//! installer's seven and the per-triangle tests they run, read from the
//! instructions of `Exanima.exe`.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::Vec3;

pub use crate::volume::Face;

/// The engine's small float (a hair under `f32::EPSILON`): the two-sided
/// query's margin under `r²`, the swept query's reached-the-face test and
/// the swept nearest query's along-the-plane test.
pub const SMALL: f32 = f32::from_bits(0x33ff_ffd7);
/// The kd-tree leaf's front and swept push floor (1e-6): a squared
/// distance under it pushes along the face normal, and the front query's
/// reach is `r` less it.
pub const PUSH_FLOOR: f32 = f32::from_bits(0x3586_37bd);
/// The mesh's own brute-force front query's floor (1e-10), the same role
/// as [`PUSH_FLOOR`].
pub const BRUTE_PUSH_FLOOR: f32 = f32::from_bits(0x2edb_e6ff);

/// `a · b`, summed `x + y`, then `z`.
fn dot(a: Vec3, b: Vec3) -> f32 {
    let xy = a.x * b.x + a.y * b.y;
    xy + a.z * b.z
}

/// A triangle's query record ([`Face::exact`]).
pub fn face(t: &[Vec3; 3]) -> Face {
    Face::exact(*t)
}

/// The signed distance of `p` in front of the face's plane.
fn distance(face: &Face, p: Vec3) -> f32 {
    dot(p, face.normal) - face.plane
}

/// The two-sided sphere query's test of one triangle: the closest point
/// when the plane is within `radius` of `centre` on either side and the
/// point nearer than `radius² − margin`. `margin` is [`SMALL`] in
/// centimetres (the caller scales it for other units).
pub fn two_sided_point(t: &[Vec3; 3], centre: Vec3, radius: f32, margin: f32) -> Option<Vec3> {
    two_sided_face(&face(t), centre, radius, margin)
}

/// [`two_sided_point`] on a record.
pub fn two_sided_face(face: &Face, centre: Vec3, radius: f32, margin: f32) -> Option<Vec3> {
    let s = distance(face, centre);
    // A "greater" compare: a NaN distance goes on.
    if s.abs() > radius {
        return None;
    }
    let q = face.nearest_exact(centre, s);
    let v = q - centre;
    let d2 = dot(v, v);
    (radius * radius - margin > d2).then_some(q)
}

/// The front sphere query's test of one triangle with the kd-tree leaf's
/// floor: the closest point and the push off it (centimetres).
pub fn front_push(t: &[Vec3; 3], centre: Vec3, radius: f32) -> Option<(Vec3, Vec3)> {
    front_push_face(&face(t), centre, radius, PUSH_FLOOR)
}

/// The front sphere query's test of one record, with the floor of the
/// form that runs it ([`PUSH_FLOOR`] or [`BRUTE_PUSH_FLOOR`]).
pub fn front_push_face(face: &Face, centre: Vec3, radius: f32, floor: f32) -> Option<(Vec3, Vec3)> {
    let s = distance(face, centre);
    // "Below" and "at or above" compares: a NaN distance goes on.
    if s < 0.0 || s >= radius {
        return None;
    }
    let q = face.nearest_exact(centre, s);
    let v = centre - q;
    let d2 = dot(v, v);
    let reach = radius - floor;
    // A NaN squared distance goes on.
    if reach * reach <= d2 {
        return None;
    }
    Some((q, off_point(face, v, d2, radius, floor)))
}

/// The push off a closest point: `v` (the centre less the point, squared
/// length `d2`) out to `radius`, or the face normal times `radius` when
/// `d2` is under the floor (a NaN takes the root).
fn off_point(face: &Face, v: Vec3, d2: f32, radius: f32, floor: f32) -> Vec3 {
    if d2 < floor {
        face.normal * radius
    } else {
        let d = d2.sqrt();
        v * ((radius - d) / d)
    }
}

/// A swept sphere as the swept queries read it: the start, the end, the
/// radius, the unit direction and the length the walk is cut to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Swept {
    pub start: Vec3,
    pub end: Vec3,
    pub radius: f32,
    pub direction: Vec3,
    pub length: f32,
}

/// The swept query's test of one triangle: the closest point and the push
/// on the end (centimetres).
pub fn swept_push(t: &[Vec3; 3], s: &Swept) -> Option<(Vec3, Vec3)> {
    swept_push_face(&face(t), s)
}

/// [`swept_push`] on a record.
pub fn swept_push_face(face: &Face, s: &Swept) -> Option<(Vec3, Vec3)> {
    let r = s.radius;
    let n = face.normal;
    let s0 = distance(face, s.start);
    // A NaN start goes on.
    if s0 < 0.0 {
        return None;
    }
    let s1 = distance(face, s.end);
    // A NaN end goes on.
    if s1 >= r {
        return None;
    }
    if s1 > SMALL {
        // The end is still clear of the face (a NaN end is not).
        let q = face.nearest_exact(s.end, s1);
        let v = s.end - q;
        let d2 = dot(v, v);
        let reach = r - PUSH_FLOOR;
        if reach * reach <= d2 {
            return None;
        }
        return Some((q, off_point(face, v, d2, r, PUSH_FLOOR)));
    }
    // The end reached the face: measure from where the centre was one
    // radius out, or from the start when it began closer (or is NaN).
    let (reference, at) = if s0 > r {
        let t = (s0 - r) / (s0 - s1);
        (s.start + (s.end - s.start) * t, r)
    } else {
        (s.start, s0)
    };
    let q = face.nearest_exact(reference, at);
    let a = q - s.start;
    let b = q - s.end;
    // A NaN takes the along-the-sweep branch.
    if !(dot(a, b) > 0.0) {
        // The closest point lies along the sweep.
        let d2 = dot(a, a);
        if d2 < r * r {
            let touch = if d2 < PUSH_FLOOR {
                s.start + n * r
            } else {
                let d = d2.sqrt();
                s.start + a * ((d - r) / d)
            };
            return Some((q, touch - s.end));
        }
        let along = dot(a, s.direction);
        if along < 0.0 {
            return None;
        }
        let across = a - s.direction * along;
        let h = r * r - dot(across, across);
        if h < SMALL {
            return None;
        }
        let t_hit = along - h.sqrt();
        if t_hit > s.length {
            return None;
        }
        let touch = s.start + s.direction * t_hit;
        return Some((q, touch - s.end));
    }
    // The closest point is past the end. [quirk: the push scales the
    // start's offset to the point by the end's gap, as the instructions
    // do.]
    let d2 = dot(b, b);
    if !(d2 < r * r) {
        return None;
    }
    let push = if d2 < PUSH_FLOOR {
        n * r
    } else {
        let d = d2.sqrt();
        let mut push = a * ((d - r) / d);
        let sink = dot(push, n);
        if sink < 0.0 {
            push += n * -sink;
        }
        push
    };
    Some((q, push))
}

/// The swept nearest query's test of one record: the distance along the
/// sweep (unit `direction` from `start`, radius `radius`) at which the
/// sphere meets the face, when it is inside `(0, best)`; `best` is the
/// sweep's length and is cut to the hit, as the original's walk keeps the
/// two in one field.
pub fn swept_nearest(
    face: &Face,
    start: Vec3,
    direction: Vec3,
    radius: f32,
    best: &mut f32,
) -> bool {
    let length = *best;
    let s0 = distance(face, start);
    // "Below" and "above" compares: a NaN goes on.
    if s0 < 0.0 {
        return false;
    }
    let delta = dot(direction, face.normal) * length;
    if delta > 0.0 || s0 + delta > radius {
        return false;
    }
    // A sweep along the plane is taken at its end (a NaN change is not).
    let travel = if delta.abs() < SMALL {
        length
    } else {
        (-(s0 - radius) / delta) * length
    };
    let centre = start + direction * travel;
    let q = face.nearest_exact(centre, distance(face, centre));
    let v = q - centre;
    let along = dot(v, direction);
    let across = Vec3::new(
        v.x - along * direction.x,
        v.y - along * direction.y,
        v.z - along * direction.z,
    );
    let p2 = dot(across, across);
    if !(p2 < radius * radius) {
        return false;
    }
    let t = (dot(q, direction) - dot(start, direction)) - (radius * radius - p2).sqrt();
    if 0.0 < t && t < length {
        *best = t;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A floor triangle at y = 0, its front up.
    fn floor() -> [Vec3; 3] {
        [
            Vec3::new(-100.0, 0.0, -100.0),
            Vec3::new(0.0, 0.0, 100.0),
            Vec3::new(100.0, 0.0, -100.0),
        ]
    }

    #[test]
    fn the_two_sided_query_meets_either_side() {
        let t = floor();
        assert!(face(&t).normal.y > 0.99);
        let above = two_sided_point(&t, Vec3::new(0.0, 5.0, 0.0), 10.0, SMALL);
        let below = two_sided_point(&t, Vec3::new(0.0, -5.0, 0.0), 10.0, SMALL);
        assert_eq!(above, Some(Vec3::ZERO));
        assert_eq!(below, Some(Vec3::ZERO));
        assert_eq!(
            two_sided_point(&t, Vec3::new(0.0, 11.0, 0.0), 10.0, SMALL),
            None
        );
        assert_eq!(
            two_sided_point(&t, Vec3::new(0.0, f32::NAN, 0.0), 10.0, SMALL),
            None
        );
    }

    #[test]
    fn the_front_query_pushes_out_to_the_radius_and_skips_the_back() {
        let t = floor();
        let (q, push) = front_push(&t, Vec3::new(0.0, 4.0, 0.0), 10.0).unwrap();
        assert_eq!(q, Vec3::ZERO);
        assert!((push - Vec3::new(0.0, 6.0, 0.0)).length() < 1e-5);
        assert_eq!(front_push(&t, Vec3::new(0.0, -4.0, 0.0), 10.0), None);
        // On the face: the normal times the radius.
        let (_, push) = front_push(&t, Vec3::ZERO, 10.0).unwrap();
        assert!((push - Vec3::new(0.0, 10.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn a_sweep_through_the_face_is_brought_back_to_the_touch() {
        let t = floor();
        let start = Vec3::new(0.0, 30.0, 0.0);
        let end = Vec3::new(0.0, -30.0, 0.0);
        let sweep = Swept {
            start,
            end,
            radius: 10.0,
            direction: Vec3::NEG_Y,
            length: 60.0,
        };
        let (_, push) = swept_push(&t, &sweep).unwrap();
        // The touch is 10 above the face.
        assert!(((end + push) - Vec3::new(0.0, 10.0, 0.0)).length() < 1e-4);
        // Started behind the face: nothing.
        let behind = Swept {
            start: Vec3::new(0.0, -1.0, 0.0),
            ..sweep
        };
        assert_eq!(swept_push(&t, &behind), None);
        // An end still clear of the face is pushed off as the front query.
        let short = Swept {
            end: Vec3::new(0.0, 4.0, 0.0),
            ..sweep
        };
        let (_, push) = swept_push(&t, &short).unwrap();
        assert!((push - Vec3::new(0.0, 6.0, 0.0)).length() < 1e-4);
    }

    /// The closest point is the record's region test, not a projection: a
    /// point beyond the second corner's side but in front of the first
    /// edge's span goes to the far edge, clamped.
    #[test]
    fn the_closest_point_is_the_records_region_test() {
        let t = floor();
        let f = face(&t);
        // Over the face: straight down.
        assert_eq!(
            f.nearest_exact(Vec3::new(0.0, 5.0, 0.0), 5.0),
            Vec3::new(0.0, 0.0, 0.0)
        );
        // Past the far corner: the corner.
        let p = Vec3::new(0.0, 0.0, 300.0);
        let q = f.nearest_exact(p, distance(&f, p));
        assert!((q - Vec3::new(0.0, 0.0, 100.0)).length() < 1e-3, "{q}");
    }

    /// The front query's floors: the leaf's 1e-6 and the brute-force
    /// form's 1e-10 pick the normal push for a centre within the floor of
    /// the face, and the root push past it.
    #[test]
    fn the_front_query_floor_picks_the_normal_push() {
        let t = floor();
        let f = face(&t);
        let centre = Vec3::new(0.0, 1e-4, 0.0);
        // d² = 1e-8: under the leaf's floor, over the brute-force one's.
        let (_, leaf) = front_push_face(&f, centre, 10.0, PUSH_FLOOR).unwrap();
        assert_eq!(leaf, Vec3::new(0.0, 10.0, 0.0));
        let (_, brute) = front_push_face(&f, centre, 10.0, BRUTE_PUSH_FLOOR).unwrap();
        assert!((brute - Vec3::new(0.0, 10.0 - 1e-4, 0.0)).length() < 1e-4);
    }

    /// The instructions' unordered branches: a NaN centre goes past the
    /// plane tests of the front query and reaches the push.
    #[test]
    fn a_nan_centre_reaches_the_front_push() {
        let t = floor();
        let hit = front_push(&t, Vec3::new(0.0, f32::NAN, 0.0), 10.0);
        assert!(hit.is_some_and(|(_, push)| push.is_nan()));
    }

    /// An end that passed beside the face within reach of its closest
    /// point is pushed along the start's offset to the point, scaled by
    /// the end's gap, and lifted to the face.
    #[test]
    fn a_sweep_past_the_edge_pushes_along_the_starts_offset() {
        let t = floor();
        // From above the far corner's side, down past the face's edge.
        let start = Vec3::new(0.0, 20.0, 130.0);
        let end = Vec3::new(0.0, -2.0, 108.0);
        let travel = end - start;
        let sweep = Swept {
            start,
            end,
            radius: 10.0,
            direction: travel / travel.length(),
            length: travel.length(),
        };
        let (q, push) = swept_push(&t, &sweep).unwrap();
        let a = q - start;
        let b = q - end;
        assert!(a.dot(b) > 0.0, "the point lies past the end");
        let d = b.length();
        let mut expected = a * ((d - 10.0) / d);
        let sink = expected.dot(Vec3::Y);
        if sink < 0.0 {
            expected += Vec3::Y * -sink;
        }
        assert!((push - expected).length() < 1e-4, "{push} {expected}");
    }

    /// The swept nearest query: a sphere dropped onto the face's middle
    /// meets it one radius over it, and the sweep is cut to the hit; one
    /// that would reach it only past the length misses.
    #[test]
    fn the_swept_nearest_query_cuts_the_sweep() {
        let t = floor();
        let f = face(&t);
        let start = Vec3::new(0.0, 30.0, 0.0);
        let mut best = 50.0;
        assert!(swept_nearest(&f, start, Vec3::NEG_Y, 10.0, &mut best));
        assert!((best - 20.0).abs() < 1e-4, "{best}");
        let mut short = 15.0;
        assert!(!swept_nearest(&f, start, Vec3::NEG_Y, 10.0, &mut short));
        assert_eq!(short, 15.0);
        // Moving away from the face: no hit.
        let mut away = 50.0;
        assert!(!swept_nearest(&f, start, Vec3::Y, 10.0, &mut away));
    }
}
