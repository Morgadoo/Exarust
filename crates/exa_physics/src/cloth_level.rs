//! A cloth against the level: the cloth's static-mesh contact, as the
//! sector hands it each static collider its object stage finds.
//!
//! For one level mesh the cloth walks its particles in order. A particle
//! that moved more than 0.1 cm² over the tick (in the mesh's frame) sweeps
//! a 2 cm sphere (the cloth's padding) from where it was to where it is;
//! one that barely moved is a 2 cm sphere where it is. Either is first
//! tested against the mesh's box, then handed to the mesh's own sphere or
//! sweep query, which visits every triangle of the mesh in order and pushes
//! the particle off each front face it reaches: the particle's end moves by
//! each push before the next triangle is tried. A particle that was pushed
//! then slides: its travel since the last tick is shortened by 0.1 cm, or
//! cancelled when it is no longer than that. This friction is what lets a
//! cloth resting on a wall stop and fall asleep.
//!
//! What the original does not do matters as much for its cost:
//!
//! - The cloth never meets the static mesh of its own scene node (the mesh
//!   it is drawn from, when that mesh is also level collision).
//! - A level push never wakes a sleeping cloth; only a body does.
//! - The stage runs for a sleeping cloth too, but there a particle sits
//!   where the last tick left it and the slide rule returns it to its
//!   last position, so a still, sleeping cloth over still geometry is a
//!   fixed point and the caller may skip it.
//!
//! The arithmetic follows the mesh queries' instructions: plane distances
//! sum `x + y`, then `z`, then subtract the plane's offset; squared lengths
//! sum the same way; compares take the side the processor's unordered
//! result takes (a NaN distance goes on to the next test). The closest
//! point on a triangle is parry's projection, a representation difference
//! from the original's own edge walk (the same point for a finite input).
//!
//! Evidence: the cloth's static-collider slot, its push callback, the
//! segment builder it calls first, the mesh's sphere and sweep queries and
//! the vector helpers they call (REA assembly), the cloth constructor's
//! padding (2 cm) and slide (0.1 cm) fields.

use bevy::math::Vec3;

use crate::cloth::{Cloth, LevelTri, PADDING_CM};
use crate::intersect;

/// A particle that moved more than this (cm², squared) over the tick sweeps
/// its sphere through the mesh instead of testing it where it is.
pub const LEVEL_SWEEP_TRAVEL2: f32 = 0.1;

/// How far (cm) a pushed particle's travel is shortened: the cloth's slide.
pub const LEVEL_SLIDE_CM: f32 = 0.1;

/// The mesh queries' smallest squared push (cm²) taken as a direction; a
/// shorter one pushes along the face's normal instead.
pub const PUSH_FLOOR2: f32 = 1.0e-6;

/// The sphere query's margin under the radius (cm).
pub const SPHERE_MARGIN: f32 = 1.0e-10;

/// The sweep query's margin under the radius (cm), and the plane distance
/// at or under which a sweep's end has reached the face: the engine's
/// small-float constant, a hair under `f32::EPSILON`.
pub const SWEEP_MARGIN: f32 = f32::from_bits(0x33ff_ffd7);

/// One level mesh as the cloth sees it: the box about its triangles (cm,
/// the cloth's frame) and where its triangles end in [`Cloth::level`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelPiece {
    /// The box's centre.
    pub centre: Vec3,
    /// Its half extents.
    pub half: Vec3,
    /// One past the piece's last triangle in the cloth's level list.
    pub end: usize,
}

impl LevelPiece {
    /// The piece holding `triangles` (cm), ending at `end`.
    pub fn around(triangles: &[LevelTri], end: usize) -> Self {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for t in triangles {
            for v in [t.a, t.b, t.c] {
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        if triangles.is_empty() {
            (lo, hi) = (Vec3::ZERO, Vec3::ZERO);
        }
        Self {
            centre: (lo + hi) * 0.5,
            half: (hi - lo) * 0.5,
            end,
        }
    }
}

/// A triangle's face plane: its unit normal (the winding's front) and its
/// offset, so a point's distance in front is `normal · point − offset`.
pub fn face_plane(t: &LevelTri) -> Option<(Vec3, f32)> {
    let n = (t.b - t.a).cross(t.c - t.a).try_normalize()?;
    Some((n, dot(n, t.a)))
}

/// `a · b`, summed `x + y`, then `z`, as the queries' scalar code does.
fn dot(a: Vec3, b: Vec3) -> f32 {
    let xy = a.x * b.x + a.y * b.y;
    xy + a.z * b.z
}

/// The closest point on the triangle to `x`.
fn closest_point(t: &LevelTri, x: Vec3) -> Vec3 {
    use rapier3d::parry::query::PointQuery;
    rapier3d::parry::shape::Triangle::new(t.a, t.b, t.c)
        .project_local_point(x, false)
        .point
}

/// The mesh's sphere query for one triangle: a sphere at `centre` of
/// `radius` in front of the face (its plane distance in `[0, radius)`) and
/// nearer than `radius − 1e-10` to the triangle is pushed off it, along the
/// closest-point direction to `radius` (along the normal when it sits on
/// the triangle). Returns the push.
pub fn sphere_push(t: &LevelTri, plane: (Vec3, f32), centre: Vec3, radius: f32) -> Option<Vec3> {
    let (n, offset) = plane;
    let s = dot(centre, n) - offset;
    // An unordered distance goes on, as the branches do.
    if s < 0.0 || s >= radius {
        return None;
    }
    let v = centre - closest_point(t, centre);
    let d2 = dot(v, v);
    let reach = radius - SPHERE_MARGIN;
    if reach * reach <= d2 {
        return None;
    }
    if d2 < SPHERE_MARGIN {
        return Some(n * radius);
    }
    let d = d2.sqrt();
    Some(v * ((radius - d) / d))
}

/// The mesh's sweep query for one triangle: a sphere of `radius` moving
/// from `start` to `end`. The start must be on the front (plane distance
/// not negative) and the end within `radius` of the face. An end still in
/// front is pushed off the closest point as the sphere query does (with
/// the sweep's margin). An end that reached the face is measured from the
/// point where the centre was one radius out (or from the start, when it
/// began closer): the closest point there must lie within `radius` of the
/// sweep, and the end is put `radius` out along the direction from it,
/// unless that leaves the end further than `radius` from the plane, when
/// the end is only taken back to `radius` in front of the plane. Returns
/// the push on the end.
pub fn sweep_push(
    t: &LevelTri,
    plane: (Vec3, f32),
    start: Vec3,
    end: Vec3,
    radius: f32,
) -> Option<Vec3> {
    let (n, offset) = plane;
    let s0 = dot(start, n) - offset;
    if s0 < 0.0 {
        return None;
    }
    let s1 = dot(end, n) - offset;
    if s1 >= radius {
        return None;
    }
    // A NaN end counts as reached, as the unordered branch goes.
    let reached = s1 <= SWEEP_MARGIN || s1.is_nan();
    let reference = if !reached {
        end
    } else if s0 > radius {
        start + (end - start) * ((s0 - radius) / (s0 - s1))
    } else {
        start
    };
    let q = closest_point(t, reference);
    if reached {
        let sweep = intersect::Sweep::new(start, end);
        let gap2 = sweep.point_distance_squared(q);
        if radius * radius <= gap2 {
            return None;
        }
        let v = reference - q;
        let v2 = dot(v, v);
        if v2 <= PUSH_FLOOR2 || v2.is_nan() {
            return Some(n * (radius - s1));
        }
        let u = v * (1.0 / v2.sqrt());
        let along = dot(end, u) - dot(u, q);
        if along >= radius {
            return None;
        }
        let push = u * (radius - along);
        let out = dot(push, n) + s1;
        if out > radius {
            return Some(n * (radius - s1));
        }
        Some(push)
    } else {
        let v = end - q;
        let v2 = dot(v, v);
        let reach = radius - SWEEP_MARGIN;
        if reach * reach <= v2 {
            return None;
        }
        if v2 < PUSH_FLOOR2 {
            return Some(n * radius);
        }
        let d = v2.sqrt();
        Some(v * ((radius - d) / d))
    }
}

/// The slide after a push: the travel from `previous` to `position` loses
/// [`LEVEL_SLIDE_CM`], or all of it when it is no longer than that. A NaN
/// travel takes the shortening branch, as the original's unordered compare
/// does.
pub fn slide(previous: Vec3, position: Vec3) -> Vec3 {
    let d = position - previous;
    let d2 = dot(d, d);
    if d2 > LEVEL_SLIDE_CM * LEVEL_SLIDE_CM || d2.is_nan() {
        let length = d2.sqrt();
        previous + d * ((length - LEVEL_SLIDE_CM) / length)
    } else {
        previous
    }
}

/// The cloth's static stage over its level list: every piece in order (one
/// piece holding the whole list when the cloth has none), and each particle
/// against it as the module describes. Returns whether any particle was
/// pushed. It never wakes the cloth.
pub fn touch_level(cloth: &mut Cloth) -> bool {
    cloth.level_tests = 0;
    if cloth.level.is_empty() {
        return false;
    }
    let whole = [LevelPiece {
        centre: Vec3::ZERO,
        half: Vec3::splat(f32::INFINITY),
        end: cloth.level.len(),
    }];
    let pieces = if cloth.level_pieces.is_empty() {
        &whole[..]
    } else {
        &cloth.level_pieces[..]
    };
    let planes: Vec<Option<(Vec3, f32)>> = cloth.level.iter().map(face_plane).collect();
    let radius = PADDING_CM;
    let mut any = false;
    let mut tests = 0;
    let mut first = 0;
    for piece in pieces {
        let range = first.min(cloth.level.len())..piece.end.min(cloth.level.len());
        first = piece.end;
        if range.is_empty() {
            continue;
        }
        for particle in &mut cloth.particles {
            let start = particle.previous;
            let mut end = particle.position;
            if !start.is_finite() || !end.is_finite() {
                continue;
            }
            let travel = end - start;
            let swept = dot(travel, travel) > LEVEL_SWEEP_TRAVEL2;
            let meets = if swept {
                let sweep = intersect::Sweep::new(start, end);
                intersect::swept_sphere_aabb_hit(&sweep, radius, piece.centre, piece.half)
            } else {
                !intersect::sphere_aabb_separated(end, radius, piece.centre, piece.half)
            };
            if !meets {
                continue;
            }
            let mut hit = false;
            tests += range.len();
            for index in range.clone() {
                let Some(plane) = planes[index] else {
                    continue;
                };
                let tri = &cloth.level[index];
                let push = if swept {
                    sweep_push(tri, plane, start, end, radius)
                } else {
                    sphere_push(tri, plane, end, radius)
                };
                if let Some(push) = push {
                    end += push;
                    hit = true;
                }
            }
            if hit {
                particle.position = slide(start, end);
                any = true;
            }
        }
    }
    cloth.level_tests = tests;
    any
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloth::Particle;

    fn floor() -> LevelTri {
        LevelTri {
            a: Vec3::new(-10.0, -10.0, 0.0),
            b: Vec3::new(20.0, -10.0, 0.0),
            c: Vec3::new(-10.0, 20.0, 0.0),
        }
    }

    #[test]
    fn a_resting_sphere_is_pushed_out_to_its_radius() {
        let t = floor();
        let plane = face_plane(&t).unwrap();
        assert_eq!(plane, (Vec3::Z, 0.0));
        // 0.5 cm over the face: pushed 1.5 cm up.
        let push = sphere_push(&t, plane, Vec3::new(1.0, 1.0, 0.5), 2.0).unwrap();
        assert!(push.abs_diff_eq(Vec3::new(0.0, 0.0, 1.5), 1e-6), "{push}");
        // Behind the face, or 2 cm out, it is not touched.
        assert_eq!(sphere_push(&t, plane, Vec3::new(1.0, 1.0, -0.5), 2.0), None);
        assert_eq!(sphere_push(&t, plane, Vec3::new(1.0, 1.0, 2.0), 2.0), None);
        // On the face it goes out along the normal by the whole radius.
        let push = sphere_push(&t, plane, Vec3::new(1.0, 1.0, 0.0), 2.0).unwrap();
        assert_eq!(push, Vec3::new(0.0, 0.0, 2.0));
        // Beside the triangle, past the radius from its edge.
        assert_eq!(sphere_push(&t, plane, Vec3::new(30.0, 1.0, 0.5), 2.0), None);
    }

    #[test]
    fn a_sweep_through_the_face_ends_a_radius_out() {
        let t = floor();
        let plane = face_plane(&t).unwrap();
        let start = Vec3::new(2.0, 2.0, 8.0);
        let end = Vec3::new(2.0, 2.0, -8.0);
        // Touches at z = 2: the end goes back up to 2 cm, a push of 10.
        let push = sweep_push(&t, plane, start, end, 2.0).unwrap();
        assert!(push.abs_diff_eq(Vec3::new(0.0, 0.0, 10.0), 1e-5), "{push}");
        // A sweep starting behind the face, or ending 2 cm out, misses.
        assert_eq!(sweep_push(&t, plane, -start, Vec3::ZERO, 2.0), None);
        assert_eq!(
            sweep_push(&t, plane, start, Vec3::new(2.0, 2.0, 2.0), 2.0),
            None
        );
        // An end still in front is the static push with the sweep's margin.
        let push = sweep_push(&t, plane, start, Vec3::new(2.0, 2.0, 1.0), 2.0).unwrap();
        assert!(push.abs_diff_eq(Vec3::new(0.0, 0.0, 1.0), 1e-6), "{push}");
        // A sweep that passes beside the triangle misses.
        assert_eq!(
            sweep_push(
                &t,
                plane,
                Vec3::new(40.0, 2.0, 8.0),
                Vec3::new(40.0, 2.0, -8.0),
                2.0
            ),
            None
        );
    }

    #[test]
    fn a_pushed_particle_slides_a_tenth_of_a_centimetre() {
        let previous = Vec3::new(0.0, 0.0, 8.0);
        let moved = slide(previous, Vec3::new(0.0, 0.0, 2.0));
        assert!(moved.abs_diff_eq(Vec3::new(0.0, 0.0, 2.1), 1e-5), "{moved}");
        // 0.1 cm or less is cancelled.
        assert_eq!(slide(previous, Vec3::new(0.0, 0.0, 7.95)), previous);
        assert_eq!(slide(previous, Vec3::new(0.1, 0.0, 8.0)), previous);
    }

    #[test]
    fn the_stage_pushes_slides_and_never_wakes() {
        let mut cloth = Cloth::new(vec![
            Particle {
                previous: Vec3::new(2.0, 2.0, 8.0),
                position: Vec3::new(2.0, 2.0, -8.0),
            },
            // At rest 1 cm over the face: pushed to 2 cm, but the push is
            // over 0.1 cm so it keeps 0.9 cm of it.
            Particle {
                previous: Vec3::new(4.0, 4.0, 1.0),
                position: Vec3::new(4.0, 4.0, 1.0),
            },
            // Far beside: the piece's box keeps it out.
            Particle {
                previous: Vec3::new(100.0, 2.0, 8.0),
                position: Vec3::new(100.0, 2.0, -8.0),
            },
        ]);
        cloth.sleep = 0;
        cloth.level.push(floor());
        cloth.level_pieces.push(LevelPiece::around(&cloth.level, 1));
        assert!(touch_level(&mut cloth));
        // Two particles reach the piece's box; the third is culled.
        assert_eq!(cloth.level_tests, 2);
        assert!(
            cloth.particles[0]
                .position
                .abs_diff_eq(Vec3::new(2.0, 2.0, 2.1), 1e-4),
            "{:?}",
            cloth.particles[0]
        );
        assert!(
            cloth.particles[1]
                .position
                .abs_diff_eq(Vec3::new(4.0, 4.0, 1.9), 1e-5),
            "{:?}",
            cloth.particles[1]
        );
        assert_eq!(cloth.particles[2].position, Vec3::new(100.0, 2.0, -8.0));
        assert_eq!(cloth.sleep, 0);
        // A second pass from where a still particle sits is a fixed point:
        // the push from 1.9 cm is 0.1 cm and the slide cancels it.
        let mut still = cloth.clone();
        still.particles[1].previous = Vec3::new(4.0, 4.0, 1.0);
        touch_level(&mut still);
        assert!(
            still.particles[1]
                .position
                .abs_diff_eq(Vec3::new(4.0, 4.0, 1.9), 1e-5),
            "{:?}",
            still.particles[1]
        );
    }

    #[test]
    fn a_piece_box_holds_its_triangles() {
        let piece = LevelPiece::around(&[floor()], 1);
        assert_eq!(piece.centre, Vec3::new(5.0, 5.0, 0.0));
        assert_eq!(piece.half, Vec3::new(15.0, 15.0, 0.0));
        assert_eq!(piece.end, 1);
        assert_eq!(LevelPiece::around(&[], 0).half, Vec3::ZERO);
    }
}
