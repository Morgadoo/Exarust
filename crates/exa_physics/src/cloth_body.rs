//! A cloth against the rigid bodies the sector hands it: the cloth's
//! body-contact call and the push it applies after the body stage.
//!
//! The original never pushes a cloth particle out of a body sphere one by
//! one. For each body it visits the body's spheres (the last one first),
//! each as a 2 cm-padded sphere from where the last tick left it to where it
//! is now, and pushes cloth *triangles* off it:
//!
//! - A sphere that moved more than 0.01 cm² (squared, compared in double
//!   precision) sweeps; one that barely moved is tested where it was. Either
//!   must reach the cloth's box first.
//! - The cloth's triangles are walked with the cloth's swept query
//!   ([`crate::cloth::swept_hit`]): each triangle's record is the sphere
//!   about it at the last tick and now (its midpoint, motion and radius),
//!   the body sphere's is its sweep's midpoint, travel and radius, with a
//!   coarse radius of the padded radius plus a half (when it moves).
//! - A triangle the query returns is taken when the sphere came from its
//!   front and ends within the padded radius in front of it, or came from
//!   behind and ends within that radius behind it. The sweep's midpoint is
//!   put on the triangle's plane, the closest point of the triangle to it
//!   is found (with its three weights), and the direction from the sphere's
//!   start to that point decides the depth: the sphere's end along that
//!   direction plus its radius, less the point's. A positive depth pushes
//!   the three vertices along the direction by `depth · weight / largest
//!   weight` each, into the cloth's push buffer, where a push that already
//!   points the same way is not added twice (only the part beyond the
//!   larger of the two counts).
//! - After a push the cloth and the body trade sleep countdowns: the lower
//!   one takes the higher. A sleeping body (countdown 0) never wakes or
//!   keeps a cloth awake; a moving one does.
//!
//! An awake cloth then applies its buffer once, after the body stage: a
//! vertex with a push is moved out by it along its direction, after its
//! travel across that direction is cancelled (when its square is at most
//! 0.2 cm²) or shortened by `√0.1` cm. The push's plane is kept for the
//! three constraint passes that follow (the cloth's iteration count less
//! one), each of which ends by putting a vertex behind its plane back on
//! it. The buffer is then cleared.
//!
//! What this port leaves out, and why: the body's share of the contact (the
//! impulse and friction the cloth hands back to the body, 0.002, 0.01 or
//! 0.015 of the summed push, and the rustle energy from the body's speed)
//! needs a mutable body in the cloth stage, which the game does not give it
//! yet; the cloth's own rules are complete. The original culls the cloth's
//! triangles against the body's oriented box before the walk; this port
//! culls them against the sphere about the body's sweeps, which keeps every
//! triangle the walk could return (the walk's own tests decide).
//!
//! The arithmetic follows the instructions: dot products sum `x + y`, then
//! `z`; the closest-point branches take the processor's unordered sides
//! (a NaN goes the way the jumps send it); the largest weight is `MAXSS`;
//! the face plane and the push direction normalise with the processor's
//! approximate reciprocal square root, and the circumscribed sphere with
//! its approximate reciprocal, as the original does.
//!
//! Evidence: the cloth's body-contact slot, its triangle bound and plane
//! builders, its swept triangle query, the post-body push pass and its
//! constraint pass, and the sector's object stage that calls them (REA
//! assembly).

// The original's compares deliberately send NaN down one branch.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::Vec3;

use crate::cloth::{Cloth, ClothTri, SweptProbe, swept_hit};
use crate::engine_math;

/// A push buffer entry shorter than this squared (cm²) is no push.
pub const PUSH_FLOOR2: f32 = 1.0e-6;

/// The squared travel across a push (cm², double precision) at or under
/// which the push cancels it.
pub const PUSH_STOP2: f64 = 0.2;

/// The squared length (cm², double) whose root a longer travel across a
/// push loses.
pub const PUSH_SLIDE2: f64 = 0.1;

/// The engine's small-float constant: a motion this short has no direction.
const MOTION_FLOOR2: f32 = f32::from_bits(0x33ff_ffd7);

/// The constraint passes after the body push: the shipped cloth's
/// iteration count (4) less the one the step already made.
pub const PUSH_PASSES: usize = 3;

/// One body sphere as the cloth's body stage takes it, in the cloth's frame
/// and centimetres: where the last tick left it, where it is, its radius
/// with the cloth's padding, and whether it moved enough to sweep.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodySweep {
    pub from: Vec3,
    pub to: Vec3,
    /// The sphere's radius plus the cloth's padding.
    pub radius: f32,
    /// Whether it moved more than
    /// [`crate::sector::CLOTH_SWEEP_TRAVEL2`].
    pub moving: bool,
}

/// One rigid body handed to the cloth: its spheres that reach the cloth's
/// box, in the order the original visits them (the body's last sphere
/// first), and its sleep countdown.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClothBody {
    pub spheres: Vec<BodySweep>,
    /// The body's sleep countdown: 0 asleep.
    pub countdown: i32,
}

/// A cloth triangle as the body stage sees it this tick: its face plane
/// from the particles now, and the record the swept query walks (the sphere
/// about it at the last tick and now).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriangleBound {
    /// The face's unit normal (approximately: the original's reciprocal
    /// square root).
    pub normal: Vec3,
    /// The plane's offset: a point's distance in front is `normal · p −
    /// offset`.
    pub offset: f32,
    /// The swept query's record for it.
    pub probe: SweptProbe,
}

/// `a · b`, summed `x + y`, then `z`.
fn dot(a: Vec3, b: Vec3) -> f32 {
    let xy = a.x * b.x + a.y * b.y;
    xy + a.z * b.z
}

/// The processor's approximate reciprocal (`RCPSS`).
fn reciprocal(value: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: SSE is part of the x86_64 baseline.
        unsafe {
            use std::arch::x86_64::{_mm_cvtss_f32, _mm_rcp_ss, _mm_set_ss};
            _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(value)))
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1.0 / value
    }
}

/// The face plane of a cloth triangle: `(b − a) × (c − a)` scaled by the
/// approximate reciprocal square root of its squared length, and the
/// plane's offset through `a`.
pub fn face_plane(a: Vec3, b: Vec3, c: Vec3) -> (Vec3, f32) {
    let n = (b - a).cross(c - a);
    let normal = n * engine_math::reciprocal_sqrt(dot(n, n));
    (normal, dot(a, normal))
}

/// The smallest sphere about a triangle, as the cloth's bound builder finds
/// it: when the longest edge's square is at least the sum of the other
/// two, the sphere on that edge; otherwise the circumscribed sphere (its
/// centre found with the approximate reciprocal).
pub fn triangle_sphere(p0: Vec3, p1: Vec3, p2: Vec3) -> (Vec3, f32) {
    let a = p0 - p1;
    let b = p2 - p1;
    let c = p0 - p2;
    let (la, lb, lc) = (dot(a, a), dot(b, b), dot(c, c));
    // The branches jump on "below", which an unordered compare also takes.
    let below = |x: f32, y: f32| !(x >= y);
    // Which edge is the longest: the one the sphere may sit on.
    let longest_c = if below(la, lb) {
        if !below(lb, lc) {
            if !below(lb, lc + la) {
                return (b * 0.5 + p1, lb.sqrt() * 0.5);
            }
            false
        } else {
            true
        }
    } else if !below(la, lc) {
        if !below(la, lc + lb) {
            return (a * 0.5 + p1, la.sqrt() * 0.5);
        }
        false
    } else {
        true
    };
    if longest_c && !below(lc, la + lb) {
        return (c * 0.5 + p2, lc.sqrt() * 0.5);
    }
    let n = b.cross(a);
    let v = a * lb - b * la;
    let k = reciprocal(dot(n, n)) * 0.5;
    let offset = v.cross(n) * k;
    (p1 + offset, dot(offset, offset).sqrt())
}

/// Each cloth triangle's plane and swept record for this tick, built from
/// the particles' last and current positions.
pub fn triangle_bounds(cloth: &Cloth) -> Vec<TriangleBound> {
    let count = cloth.particles.len();
    let mut out = Vec::with_capacity(cloth.triangles.len());
    for tri in &cloth.triangles {
        if tri.a >= count || tri.b >= count || tri.c >= count {
            out.push(TriangleBound {
                normal: Vec3::ZERO,
                offset: f32::NAN,
                probe: SweptProbe {
                    position: Vec3::splat(f32::NAN),
                    radius: f32::NAN,
                    velocity: Vec3::ZERO,
                    radius_rate: f32::NAN,
                },
            });
            continue;
        }
        let now = |i: usize| cloth.particles[i].position;
        let then = |i: usize| cloth.particles[i].previous;
        let (normal, offset) = face_plane(now(tri.a), now(tri.b), now(tri.c));
        let (c0, r0) = triangle_sphere(then(tri.a), then(tri.b), then(tri.c));
        let (c1, r1) = triangle_sphere(now(tri.a), now(tri.b), now(tri.c));
        let largest = if r0 < r1 { r1 } else { r0 };
        let motion = c1 - c0;
        let m2 = dot(motion, motion);
        let coarse = if m2 > MOTION_FLOOR2 {
            -largest - m2.sqrt()
        } else {
            -largest
        };
        out.push(TriangleBound {
            normal,
            offset,
            probe: SweptProbe {
                position: c0 + motion * 0.5,
                radius: coarse,
                velocity: motion,
                radius_rate: -largest,
            },
        });
    }
    out
}

/// The closest point of triangle `(v0, v1, v2)` to `point` (a point on its
/// plane), with the three weights the push shares out, in the original's
/// branch order.
pub fn closest_point(v0: Vec3, v1: Vec3, v2: Vec3, point: Vec3) -> (Vec3, [f32; 3]) {
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let p = point - v0;
    let a = dot(e1, e1);
    let b = dot(e1, e2);
    let d = dot(p, e1);
    let e = dot(p, e2);
    let s = a * e - b * d;
    let edge12 = || {
        let e3 = v2 - v1;
        let q = point - v1;
        let u = engine_math::clamp(dot(e3, q) / dot(e3, e3), 0.0, 1.0);
        (v1 + e3 * u, [0.0, 1.0 - u, u])
    };
    if !(s <= 0.0) {
        let c = dot(e2, e2);
        let t = c * d - b * e;
        if t < 0.0 {
            if c < e {
                return edge12();
            }
            let u = engine_math::max(e / c, 0.0);
            return (v0 + e2 * u, [1.0 - u, 0.0, u]);
        }
        let inv = 1.0 / (a * c - b * b);
        if (t + s) * inv > 1.0 {
            return edge12();
        }
        let (w1, w2) = (t * inv, s * inv);
        return (point, [1.0 - (w1 + w2), w1, w2]);
    }
    if !(d <= 0.0) {
        if a < d {
            return edge12();
        }
        let u = d / a;
        return (v0 + e1 * u, [1.0 - u, u, 0.0]);
    }
    let c = dot(e2, e2);
    let u = engine_math::clamp(e / c, 0.0, 1.0);
    (v0 + e2 * u, [1.0 - u, 0.0, u])
}

/// Adds `push` to a vertex's buffered push without counting twice the part
/// both point along: when they point the same way, the projection on the
/// longer of the two is taken back out.
pub fn add_push(buffered: &mut Vec3, push: Vec3) {
    let along = dot(push, *buffered);
    if !(along <= 0.0) {
        let pp = dot(push, push);
        let bb = dot(*buffered, *buffered);
        let back = if bb < pp {
            push * (along / pp)
        } else {
            *buffered * (along / bb)
        };
        *buffered += push;
        *buffered -= back;
    } else {
        *buffered += push;
    }
}

/// What one body did to the cloth.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BodyTouch {
    /// Whether any of its spheres pushed a triangle.
    pub pushed: bool,
    /// The summed push on the body (cm, the cloth's frame): the original
    /// hands it back to the body, scaled; this port only reports it.
    pub reaction: Vec3,
    /// Triangles the swept query looked at.
    pub tests: usize,
}

/// One body's spheres against the cloth's triangles (`bounds` from
/// [`triangle_bounds`]), adding to `pushes` (one per particle). The
/// countdown trade is the caller's ([`trade_countdowns`]).
pub fn touch_body(
    particles: &[crate::cloth::Particle],
    triangles: &[ClothTri],
    bounds: &[TriangleBound],
    body: &ClothBody,
    pushes: &mut [Vec3],
) -> BodyTouch {
    let mut touch = BodyTouch::default();
    if body.spheres.is_empty() || triangles.is_empty() {
        return touch;
    }
    // The candidates: triangles whose coarse sphere meets the sphere about
    // this body's coarse sweeps, in triangle order.
    let probes: Vec<SweptProbe> = body.spheres.iter().map(sphere_probe).collect();
    let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    for p in &probes {
        lo = lo.min(p.position - Vec3::splat(p.radius));
        hi = hi.max(p.position + Vec3::splat(p.radius));
    }
    let centre = (lo + hi) * 0.5;
    // This port's own cull (standing in for the body-box cull), with a
    // hair of room for rounding so it never drops a triangle the walk's
    // coarse test would keep.
    let reach = (hi - lo).length() * 0.5 + 1.0e-3;
    let mut picked = Vec::new();
    let mut records = Vec::new();
    for (index, bound) in bounds.iter().enumerate().take(triangles.len()) {
        let gap = bound.probe.position - centre;
        let room = reach - bound.probe.radius;
        // A NaN keeps the triangle: the walk's own tests decide.
        if !(dot(gap, gap) > room * room) || !reach.is_finite() {
            picked.push(index);
            records.push(bound.probe);
        }
    }
    let count = particles.len();
    for (sphere, probe) in body.spheres.iter().zip(&probes) {
        let r = sphere.radius;
        let mut from = 0;
        let mut pushed = false;
        while from < records.len() {
            let Some(found) = swept_hit(&records, probe, from, records.len()) else {
                touch.tests += records.len() - from;
                break;
            };
            touch.tests += found + 1 - from;
            from = found + 1;
            let index = picked[found];
            let tri = triangles[index];
            if tri.a >= count || tri.b >= count || tri.c >= count {
                continue;
            }
            let bound = &bounds[index];
            let near = dot(sphere.from, bound.normal) - bound.offset;
            let far = dot(sphere.to, bound.normal) - bound.offset;
            let takes = if near > 0.0 { !(far > r) } else { !(far < -r) };
            if !takes {
                continue;
            }
            let mid = probe.position;
            let on_plane = mid + bound.normal * -((near + far) * 0.5);
            let (v0, v1, v2) = (
                particles[tri.a].position,
                particles[tri.b].position,
                particles[tri.c].position,
            );
            let (closest, w) = closest_point(v0, v1, v2, on_plane);
            let toward = closest - sphere.from;
            let length2 = (toward.x * toward.x + toward.z * toward.z) + toward.y * toward.y;
            let direction = if length2 == 0.0 {
                Vec3::ZERO
            } else {
                toward * engine_math::reciprocal_sqrt(length2)
            };
            let reach = dot(sphere.to, direction) + r;
            let at = dot(closest, direction);
            if !(reach > at) {
                continue;
            }
            let depth = reach - at;
            touch.reaction += direction * -depth;
            let largest = engine_math::max(engine_math::max(w[0], w[1]), w[2]);
            let share = depth / largest;
            for (vertex, weight) in [(tri.a, w[0]), (tri.b, w[1]), (tri.c, w[2])] {
                add_push(&mut pushes[vertex], direction * (share * weight));
            }
            pushed = true;
        }
        touch.pushed |= pushed;
    }
    touch
}

/// A body sphere's record for the swept query: its sweep's midpoint, its
/// coarse radius (the padded radius plus a half when it moves), its travel
/// and its padded radius.
fn sphere_probe(sphere: &BodySweep) -> SweptProbe {
    let travel = sphere.to - sphere.from;
    let span = if sphere.moving { 1.0 } else { 0.0 };
    SweptProbe {
        position: sphere.from + travel * 0.5,
        radius: span * 0.5 + sphere.radius,
        velocity: travel,
        radius_rate: sphere.radius,
    }
}

/// The countdown trade after a body pushed the cloth: the cloth takes the
/// body's countdown when the body's is higher (waking it), and the body is
/// owed the cloth's when the cloth's is higher. Returns whether the cloth's
/// countdown rose, and the countdown the body is owed (when it is).
pub fn trade_countdowns(cloth_countdown: &mut i32, body_countdown: i32) -> (bool, Option<i32>) {
    if *cloth_countdown < body_countdown {
        *cloth_countdown = body_countdown;
        (true, None)
    } else if body_countdown < *cloth_countdown {
        (false, Some(*cloth_countdown))
    } else {
        (false, None)
    }
}

/// The awake cloth's push pass after the body stage: each buffered push
/// moves its vertex out along it, after the vertex's travel across it is
/// stopped or shortened, and becomes the vertex's plane for the following
/// constraint passes (a vertex without a push gets no plane). The buffer is
/// cleared. Returns the planes, one per particle (`None` for no push).
pub fn apply_pushes(cloth: &mut Cloth) -> Vec<Option<(Vec3, f32)>> {
    let count = cloth.particles.len();
    cloth.pushes.resize(count, Vec3::ZERO);
    let mut planes = vec![None; count];
    for (index, particle) in cloth.particles.iter_mut().enumerate() {
        let push = cloth.pushes[index];
        let length2 = dot(push, push);
        // An unordered length goes on to the push, as the branch does.
        if length2 < PUSH_FLOOR2 {
            continue;
        }
        let length = length2.sqrt();
        let normal = push * (1.0 / length);
        let offset = dot(particle.position, normal) + length;
        planes[index] = Some((normal, offset));
        let behind = dot(particle.position, normal) - offset;
        if !(behind < 0.0) {
            continue;
        }
        let travel = particle.position - particle.previous;
        let along = (travel.x * normal.x + travel.z * normal.z) + travel.y * normal.y;
        let across = travel - normal * along;
        let across2 = dot(across, across);
        if f64::from(across2) > PUSH_STOP2 {
            let scale = -(PUSH_SLIDE2 / f64::from(across2)).sqrt();
            particle.position += across * scale as f32;
        } else {
            particle.position -= across;
        }
        particle.position += normal * -behind;
    }
    cloth.pushes.fill(Vec3::ZERO);
    planes
}

/// The constraint pass's last step with the push planes: a vertex behind
/// its plane is put back on it.
pub fn hold_planes(cloth: &mut Cloth, planes: &[Option<(Vec3, f32)>]) {
    for (particle, plane) in cloth.particles.iter_mut().zip(planes) {
        let Some((normal, offset)) = *plane else {
            continue;
        };
        let behind = dot(particle.position, normal) - offset;
        if behind < 0.0 {
            particle.position += normal * -behind;
        }
    }
}

#[cfg(test)]
#[path = "cloth_body_test.rs"]
mod tests;
