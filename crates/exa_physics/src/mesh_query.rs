//! A mesh's per-leaf triangle tests as an effect's contact asks them: the
//! three query slots a mesh carries besides the pair stage's (the two-sided
//! sphere query, the front sphere query and the swept query), each handing
//! a callback the triangle's closest point and, for the last two, the push.
//!
//! # What the original does
//!
//! When a mesh gets its kd-tree, the tree's installer sets seven query
//! slots on the mesh and every instance of it. Three of them are the
//! effects' contacts (the force blast's and the force wave's static and
//! part walks); they walk the tree's leaves (the sphere's box, or the
//! swept sphere's path) and test each leaf's triangles in leaf order:
//!
//! - **The two-sided sphere query** ([`two_sided_point`]): a triangle whose
//!   plane is within the sphere's radius on either side (`|s| ≤ r`, a NaN
//!   distance missing) and whose closest point is nearer than
//!   `r² − ε` (ε the engine's small float [`SMALL`]) hands the callback the
//!   sphere and that point. The blast's mesh walk pushes its sphere out of
//!   the point there, so the next triangle sees the moved sphere.
//! - **The front sphere query** ([`front_push`]): a triangle with the
//!   sphere's centre in front (`0 ≤ s < r`) and its closest point nearer
//!   than `r − 1e-6` hands the callback the point and the push off it:
//!   the closest-point direction out to `r`, or the face normal times `r`
//!   when the centre is within `1e-6` (squared) of the face.
//! - **The swept query** ([`swept_push`]): the record's start must be in
//!   front and its end within the radius of the plane. An end still clear
//!   of the face is pushed off its closest point as the front query does;
//!   an end that reached the face is measured from where the centre stood
//!   one radius out (or from the start when it began closer), and the push
//!   brings the end back to where the sphere first touched (the closest
//!   point within the sweep, or the ray's entry into the sphere about it),
//!   or off the closest point when that lies past the end, lifted to the
//!   face when the push would sink it. The walk then goes on with the
//!   record as the callback left it.
//!
//! The arithmetic follows the instructions: plane distances sum `x + y`,
//! then `z`, then take the plane's offset; squared lengths sum the same
//! way. The closest point on a triangle is parry's projection, as in
//! [`crate::cloth_level`] (the engine's own edge walk gives the same point
//! for a finite input).
//!
//! Evidence: the tree installer, the three tree queries and the mesh's
//! three per-leaf tests, read from `Exanima.exe`.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::Vec3;

/// The engine's small float (a hair under `f32::EPSILON`): the two-sided
/// query's margin under `r²` and the swept query's reached-the-face test.
pub const SMALL: f32 = f32::from_bits(0x33ff_ffd7);
/// The front and swept queries' push floor: a squared distance under it
/// pushes along the face normal, and the front query's reach is `r` less
/// it.
pub const PUSH_FLOOR: f32 = f32::from_bits(0x3586_37bd);

/// `a · b`, summed `x + y`, then `z`.
fn dot(a: Vec3, b: Vec3) -> f32 {
    let xy = a.x * b.x + a.y * b.y;
    xy + a.z * b.z
}

/// A triangle's face plane: the unit normal of its winding's front and the
/// offset, so a point's distance in front is `n · p − offset`.
pub fn plane(t: &[Vec3; 3]) -> Option<(Vec3, f32)> {
    let n = (t[1] - t[0]).cross(t[2] - t[0]).try_normalize()?;
    Some((n, dot(n, t[0])))
}

fn closest_point(t: &[Vec3; 3], x: Vec3) -> Vec3 {
    use rapier3d::parry::query::PointQuery;
    rapier3d::parry::shape::Triangle::new(t[0], t[1], t[2])
        .project_local_point(x, false)
        .point
}

/// The two-sided sphere query's test of one triangle: the closest point
/// when the plane is within `radius` of `centre` on either side and the
/// point nearer than `radius² − margin`. `margin` is [`SMALL`] in
/// centimetres (the caller scales it for other units).
pub fn two_sided_point(t: &[Vec3; 3], centre: Vec3, radius: f32, margin: f32) -> Option<Vec3> {
    let (n, offset) = plane(t)?;
    let s = dot(centre, n) - offset;
    if !(s.abs() <= radius) {
        return None;
    }
    let q = closest_point(t, centre);
    let v = q - centre;
    let d2 = dot(v, v);
    (d2 < radius * radius - margin).then_some(q)
}

/// The front sphere query's test of one triangle: the closest point and
/// the push off it (centimetres).
pub fn front_push(t: &[Vec3; 3], centre: Vec3, radius: f32) -> Option<(Vec3, Vec3)> {
    let (n, offset) = plane(t)?;
    let s = dot(centre, n) - offset;
    if !(0.0 <= s && s < radius) {
        return None;
    }
    let q = closest_point(t, centre);
    let v = centre - q;
    let d2 = dot(v, v);
    let reach = radius - PUSH_FLOOR;
    if !(d2 < reach * reach) {
        return None;
    }
    let push = if PUSH_FLOOR <= d2 {
        let d = d2.sqrt();
        v * ((radius - d) / d)
    } else {
        n * radius
    };
    Some((q, push))
}

/// A swept sphere as the swept query reads it: the start, the end, the
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
    let (n, offset) = plane(t)?;
    let r = s.radius;
    let s0 = dot(s.start, n) - offset;
    if !(0.0 <= s0) {
        return None;
    }
    let s1 = dot(s.end, n) - offset;
    if !(s1 < r) {
        return None;
    }
    if !(s1 <= SMALL) {
        // The end is still clear of the face.
        let q = closest_point(t, s.end);
        let v = s.end - q;
        let d2 = dot(v, v);
        let reach = r - PUSH_FLOOR;
        if !(d2 < reach * reach) {
            return None;
        }
        let push = if PUSH_FLOOR <= d2 {
            let d = d2.sqrt();
            v * ((r - d) / d)
        } else {
            n * r
        };
        return Some((q, push));
    }
    // The end reached the face: measure from where the centre was one
    // radius out, or from the start when it began closer.
    let reference = if s0 <= r {
        s.start
    } else {
        s.start + (s.end - s.start) * ((s0 - r) / (s0 - s1))
    };
    let q = closest_point(t, reference);
    let a = q - s.start;
    let b = q - s.end;
    if dot(a, b) <= 0.0 {
        // The closest point lies along the sweep.
        let d2 = dot(a, a);
        if d2 < r * r {
            let touch = if PUSH_FLOOR <= d2 {
                let d = d2.sqrt();
                s.start + a * ((d - r) / d)
            } else {
                s.start + n * r
            };
            return Some((q, touch - s.end));
        }
        let along = dot(a, s.direction);
        if 0.0 <= along {
            let across = a - s.direction * along;
            let h = r * r - dot(across, across);
            if SMALL <= h {
                let t_hit = along - h.sqrt();
                if t_hit <= s.length {
                    let touch = s.start + s.direction * t_hit;
                    return Some((q, touch - s.end));
                }
            }
        }
        None
    } else {
        // The closest point is past the end. [quirk: the push scales the
        // start's offset to the point by the end's distance, as the
        // instructions do.]
        let d2 = dot(b, b);
        if !(d2 < r * r) {
            return None;
        }
        let push = if PUSH_FLOOR <= d2 {
            let d = d2.sqrt();
            let mut push = a * ((d - r) / d);
            let sink = dot(push, n);
            if sink < 0.0 {
                push += n * -sink;
            }
            push
        } else {
            n * r
        };
        Some((q, push))
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
        assert!(plane(&t).unwrap().0.y > 0.99);
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
}
