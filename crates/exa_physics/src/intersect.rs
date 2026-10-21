//! Shared geometric intersection tests: rays and segments, spheres,
//! axis-aligned and oriented boxes, point sets and the convex volumes the
//! original clips portals and shadow volumes with. It corresponds to the
//! original's `RF_Intersection` unit.
//!
//! Every routine is pure: it takes plain vectors and returns a number or a
//! bool, and it never allocates except where it builds a result volume.
//! The tests are scale-free; the few fixed lengths (the 1e-4 bounds floor,
//! the 5 cm² vertex match, the 1 cm² shortest silhouette edge, the 10 000
//! extrusion) are the original's centimetres.
//!
//! The arithmetic follows the original's instructions, not the shape of a
//! textbook test:
//!
//! - Dot products sum `x + y`, then `z`, as glam's do.
//! - Packed and scalar minima and maxima are the processor's: the first
//!   operand when it is strictly smaller (larger), otherwise the second, so
//!   a NaN picks the second ([`min_first`], [`max_first`]).
//! - An "apart" answer is the sign bit of `reach² − distance²`, so touching
//!   counts as overlapping and a NaN gives its sign.
//! - A branch on a comparison takes the side the processor's unordered
//!   result takes; the polygon clippers treat a NaN distance as outside.
//! - The oriented-box distance compares the overhangs as integers, as the
//!   original compares the floats' bit patterns.
//!
//! The original normalises sweeps and plane normals with the processor's
//! approximate reciprocal square root, and fills a sweep's reciprocal
//! direction and the oriented-box slab test's with its approximate
//! reciprocal (about 12 significant bits, and not the same on every
//! processor). Rust's `sqrt` and reciprocal are correctly rounded, so those
//! values agree with the original to that approximation's precision, not
//! bit for bit; every other operation is the original's, in its order.
//!
//! Not here: the cube-shadow face mask a light gives each node it reaches
//! is the cube-shadow cache's (`exa_render::cube_shadows::sphere_faces`),
//! its only user. The unit's scene-graph bookkeeping (the light-range
//! globals, the weight-ordered node list, light-alias creation and its
//! spreading over a subtree) belongs with the light aliases, not with this
//! maths.
//!
//! Evidence: REA (Ghidra) pseudo-code and assembly of the unit's routines
//! and their vector helpers, the side-plane tables read from the shipped
//! executable, and the callers that fix what each argument is (the
//! directional shadow passes a unit light direction with w = 0).

// `!(a > b)` is deliberate here: it is the original's branch with a NaN.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::{Vec3, Vec4};

/// The smallest half extent a point set's bounds gets (original cm, 1e-4).
pub const MIN_BOUNDS_EXTENT: f32 = 1e-4;

/// Two polygon vertices closer than this (squared, cm²) are the same point
/// when the shadow and clipping routines match edges and chain segments.
const SAME_POINT2: f32 = 5.0;

/// The shortest shared edge (squared, cm²) that gets a silhouette plane.
const MIN_EDGE2: f32 = 1.0;

/// How far a silhouette edge is extruded toward the light (cm).
const EXTRUSION: f32 = -10000.0;

/// A chained cap closes when its ends are nearer than this (squared, cm²),
/// compared in double precision as the original does.
const CLOSED2: f64 = 1e-4;

/// The smaller of `a` and `b` as the processor's `minss` gives it: `a` when
/// it is strictly smaller, otherwise `b` (a NaN in either gives `b`).
pub fn min_first(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// The larger of `a` and `b` as the processor's `maxss` gives it: `a` when
/// it is strictly larger, otherwise `b` (a NaN in either gives `b`).
pub fn max_first(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// [`min_first`] on each component.
fn min_each(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        min_first(a.x, b.x),
        min_first(a.y, b.y),
        min_first(a.z, b.z),
    )
}

/// [`max_first`] on each component.
fn max_each(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        max_first(a.x, b.x),
        max_first(a.y, b.y),
        max_first(a.z, b.z),
    )
}

/// The largest component, folded `x`, `y`, `z` with [`max_first`].
fn largest(v: Vec3) -> f32 {
    max_first(max_first(v.x, v.y), v.z)
}

/// The smallest component, folded `x`, `y`, `z` with [`min_first`].
fn smallest(v: Vec3) -> f32 {
    min_first(min_first(v.x, v.y), v.z)
}

/// `magnitude` with the sign bit of `sign` added to its own, as the
/// original ORs a box extent with a normal's sign before multiplying: the
/// product with `sign` is then |sign| · magnitude for a non-negative
/// magnitude.
fn with_sign_of(magnitude: f32, sign: f32) -> f32 {
    f32::from_bits((sign.to_bits() & 0x8000_0000) | magnitude.to_bits())
}

/// A box's support along `normal`: Σ normalᵢ · (extentᵢ with normalᵢ's sign).
fn support(half_extents: Vec3, normal: Vec3) -> f32 {
    normal.dot(Vec3::new(
        with_sign_of(half_extents.x, normal.x),
        with_sign_of(half_extents.y, normal.y),
        with_sign_of(half_extents.z, normal.z),
    ))
}

/// `v` scaled to unit length as the original normalises: the squared length
/// summed `x² + z²`, then `y²`, and a zero vector left zero.
fn normalised(v: Vec3) -> Vec3 {
    let squared = (v.x * v.x + v.z * v.z) + v.y * v.y;
    let inverse = if squared == 0.0 {
        0.0
    } else {
        squared.sqrt().recip()
    };
    v * inverse
}

/// A sweep along a segment: origin, length, unit direction and the
/// reciprocal of each direction component, as the original stores them for
/// its slab tests. `new` sets them from the segment's ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sweep {
    pub origin: Vec3,
    /// The segment's length, in the caller's units.
    pub length: f32,
    /// The unit direction from the origin to the end.
    pub direction: Vec3,
    /// `1 / direction`, component by component.
    pub inverse_direction: Vec3,
}

impl Sweep {
    /// A sweep from `start` to `end`. A zero-length sweep has length and
    /// direction NaN, as the original's reciprocal square root of zero does.
    pub fn new(start: Vec3, end: Vec3) -> Self {
        let delta = end - start;
        let squared = delta.length_squared();
        let inverse = squared.sqrt().recip();
        let direction = delta * inverse;
        Self {
            origin: start,
            length: squared * inverse,
            direction,
            inverse_direction: direction.recip(),
        }
    }

    /// The squared distance from `point` to this sweep's segment.
    pub fn point_distance_squared(&self, point: Vec3) -> f32 {
        point_segment_distance_squared(point, self.origin, self.direction, self.length)
    }
}

/// An oriented box: a centre, half extents and one orthonormal axis per
/// row. The axes map a world offset into the box's frame, as the original's
/// matrix does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb {
    pub centre: Vec3,
    pub half_extents: Vec3,
    pub axes: [Vec3; 3],
}

impl Obb {
    /// A box whose rows are `axes`.
    pub fn new(centre: Vec3, half_extents: Vec3, axes: [Vec3; 3]) -> Self {
        Self {
            centre,
            half_extents,
            axes,
        }
    }

    /// A box aligned with the world axes.
    pub fn axis_aligned(centre: Vec3, half_extents: Vec3) -> Self {
        Self::new(centre, half_extents, [Vec3::X, Vec3::Y, Vec3::Z])
    }

    /// `offset` in the box's frame: `(offset · axis[0], …)`.
    pub fn local(&self, offset: Vec3) -> Vec3 {
        Vec3::new(
            offset.dot(self.axes[0]),
            offset.dot(self.axes[1]),
            offset.dot(self.axes[2]),
        )
    }

    /// The absolute frame coordinates of `point` about the centre.
    pub fn abs_in_frame(&self, point: Vec3) -> Vec3 {
        self.local(point - self.centre).abs()
    }
}

/// The squared distance from `point` to the segment from `origin` along
/// `direction` (unit) of the given `length`. The projection is clamped to
/// the segment with the processor's maximum and minimum, so a NaN
/// projection lands on the far end.
pub fn point_segment_distance_squared(
    point: Vec3,
    origin: Vec3,
    direction: Vec3,
    length: f32,
) -> f32 {
    let offset = point - origin;
    let along = min_first(max_first(offset.dot(direction), 0.0), length);
    (offset - direction * along).length_squared()
}

/// Whether two spheres are apart: the sign bit of `(ra + rb)² − d²`, so
/// touching counts as overlapping.
pub fn spheres_separated(a: Vec3, ra: f32, b: Vec3, rb: f32) -> bool {
    let reach = ra + rb;
    (reach * reach - (a - b).length_squared()).is_sign_negative()
}

/// The distance² a point lies outside an axis-aligned box: zero inside,
/// otherwise the squared sum of the per-axis overhangs.
pub fn point_aabb_distance_squared(point: Vec3, centre: Vec3, half_extents: Vec3) -> f32 {
    let d = (point - centre).abs();
    let over = |d: f32, extent: f32| {
        if extent < d {
            (d - extent) * (d - extent)
        } else {
            0.0
        }
    };
    over(d.x, half_extents.x) + over(d.y, half_extents.y) + over(d.z, half_extents.z)
}

/// Whether a sphere and an axis-aligned box are apart. Touching counts as
/// overlapping, as in [`spheres_separated`].
pub fn sphere_aabb_separated(
    centre: Vec3,
    radius: f32,
    box_centre: Vec3,
    half_extents: Vec3,
) -> bool {
    (radius * radius - point_aabb_distance_squared(centre, box_centre, half_extents))
        .is_sign_negative()
}

/// Whether a sphere and a swept sphere (a capsule: a segment with a radius)
/// are apart. The segment is the sweep's origin and direction over its
/// length; `segment_radius` is the swept sphere's radius.
pub fn sphere_segment_separated(
    sweep: &Sweep,
    segment_radius: f32,
    centre: Vec3,
    radius: f32,
) -> bool {
    let reach = segment_radius + radius;
    (reach * reach - sweep.point_distance_squared(centre)).is_sign_negative()
}

/// Whether a sphere of `radius` swept along `sweep` meets an axis-aligned
/// box: the slab test with the box inflated by the radius, in sweep-length
/// units. An entry at or past the sweep's length misses; a negative entry
/// counts from 0; it hits when the entry is before the exit (the sign bit
/// of their difference).
pub fn swept_sphere_aabb_hit(
    sweep: &Sweep,
    radius: f32,
    box_centre: Vec3,
    half_extents: Vec3,
) -> bool {
    let offset = box_centre - sweep.origin;
    let extent = half_extents + Vec3::splat(radius);
    let high = (extent + offset) * sweep.inverse_direction;
    let low = (offset - extent) * sweep.inverse_direction;
    let far = max_each(low, high);
    let near = min_each(high, low);
    let entry = largest(near);
    if !(sweep.length > entry) {
        return false;
    }
    (max_first(entry, 0.0) - smallest(far)).is_sign_negative()
}

/// The slab test against an oriented box with the extents `extent`, in
/// units of the travel: hits when the clamped entry is before the exit and
/// the entry is before 1. Both compares are ordered, as the original's
/// oriented-box routines branch (unlike the sector's meet test, whose NaN
/// entry goes on): a NaN misses. An entry can only be NaN when the exit is
/// too (the NaN sits in the high lane, which both folds carry), so the
/// order of the two compares does not change an answer. The reciprocal of
/// the travel is the original's approximate one.
fn slabs_hit(offset: Vec3, travel: Vec3, extent: Vec3) -> bool {
    let inverse = travel.recip();
    let low = (offset - extent) * inverse;
    let high = (offset + extent) * inverse;
    let entry = largest(min_each(low, high));
    let exit = smallest(max_each(low, high));
    max_first(entry, 0.0) < exit && entry < 1.0
}

/// Whether a sphere of `radius` swept from `start` to `end` meets an
/// oriented box. The travel is normalised, so the entry is rejected at 1.
pub fn swept_sphere_obb_hit(start: Vec3, end: Vec3, radius: f32, obb: &Obb) -> bool {
    let offset = obb.local(obb.centre - start);
    let travel = obb.local(end - start);
    slabs_hit(offset, travel, obb.half_extents + Vec3::splat(radius))
}

/// Whether the segment from `start` along `vector` meets an oriented box
/// (the slab test with the box's own extents).
pub fn segment_obb_hit(start: Vec3, vector: Vec3, obb: &Obb) -> bool {
    let offset = obb.local(obb.centre - start);
    slabs_hit(offset, obb.local(vector), obb.half_extents)
}

/// The squared distance from `point` to an oriented box: zero inside,
/// otherwise the squared sum of the overhangs in the box's frame. The
/// original compares each overhang's float bits as integers, which orders
/// non-negative floats as the floats do and puts a NaN past every extent.
pub fn point_obb_distance_squared(point: Vec3, obb: &Obb) -> f32 {
    let local = obb.abs_in_frame(point);
    let mut sum = 0.0f32;
    for (d, extent) in [
        (local.x, obb.half_extents.x),
        (local.y, obb.half_extents.y),
        (local.z, obb.half_extents.z),
    ] {
        if (d.to_bits() as i32) > (extent.to_bits() as i32) {
            sum += (d - extent) * (d - extent);
        }
    }
    sum
}

/// Whether the sphere `centre`, `radius` meets the segment from `start`
/// along `vector`. `vector` is not normalised: its length is the segment's
/// length. Touching counts as meeting. A NaN projection measures from the
/// start, a NaN length from the end, as the original's branches fall.
pub fn segment_sphere(start: Vec3, vector: Vec3, centre: Vec3, radius: f32) -> bool {
    let offset = centre - start;
    let along = vector.dot(offset);
    let distance = if !(along > 0.0) {
        offset.length_squared()
    } else {
        let travel = vector.length_squared();
        if !(travel > along) {
            (offset - vector).length_squared()
        } else {
            offset.length_squared() - along * along / travel
        }
    };
    distance <= radius * radius
}

/// Whether the segment from `start` along `vector` meets an oriented box,
/// by the separating-axis test: a box axis separates them when the start is
/// outside its slab and not moving toward it, and a cross axis of the
/// segment and a box axis separates them when the projections do not
/// overlap. A NaN on either axis kind separates nothing.
pub fn segment_obb_overlap(start: Vec3, vector: Vec3, obb: &Obb) -> bool {
    let local = start - obb.centre;
    let travel = obb.axes.map(|axis| vector.dot(axis));
    for (i, axis) in obb.axes.iter().enumerate() {
        let along = local.dot(*axis);
        if along.abs() > obb.half_extents[i] && along * travel[i] >= 0.0 {
            return false;
        }
    }
    let travel = travel.map(f32::abs);
    let cross = vector.cross(local);
    for i in 0..3 {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        let extent = obb.half_extents[j] * travel[k] + obb.half_extents[k] * travel[j];
        if extent < cross.dot(obb.axes[i]).abs() {
            return false;
        }
    }
    true
}

/// The centre and half extents of a point set. The extents have a floor of
/// [`MIN_BOUNDS_EXTENT`], as the original's have. The original reads at
/// least two points; one point gives its own position and the floor, and
/// no points a zero centre and the floor (Exarust's guards).
pub fn bounds_of_points(points: &[Vec3]) -> (Vec3, Vec3) {
    let Some(&first) = points.first() else {
        return (Vec3::ZERO, Vec3::splat(MIN_BOUNDS_EXTENT));
    };
    let (mut low, mut high) = (first, first);
    for &point in &points[1..] {
        low = min_each(low, point);
        high = max_each(high, point);
    }
    let centre = (low + high) * 0.5;
    let extents = max_each(high - centre, Vec3::splat(MIN_BOUNDS_EXTENT));
    (centre, extents)
}

/// Grows the bounds `low..high` to include `point`, as the original expands
/// a box while it collects a mesh's vertices.
pub fn expand_bounds(low: &mut Vec3, high: &mut Vec3, point: Vec3) {
    *low = min_each(*low, point);
    *high = max_each(*high, point);
}

/// One face of a convex volume: its outward plane `normal · x + offset = 0`
/// and the polygon that bounds it, in order. A plane may carry no polygon
/// (a volume used only for its half-spaces).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub offset: f32,
    pub vertices: Vec<Vec3>,
}

impl Plane {
    /// A plane with no polygon.
    pub fn new(normal: Vec3, offset: f32) -> Self {
        Self {
            normal,
            offset,
            vertices: Vec::new(),
        }
    }

    /// The signed distance of `point` from the face's plane. Inside is
    /// negative, as in the original.
    pub fn distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.offset
    }
}

/// A convex volume: the intersection of its faces' inner half-spaces.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConvexVolume {
    pub planes: Vec<Plane>,
}

/// One polygon cut by a plane: the part on the inner side, and the cut
/// segment when the polygon crosses the plane.
struct Cut {
    points: Vec<Vec3>,
    /// The two points where the polygon's boundary crosses the plane: the
    /// entry then the exit when the polygon's last vertex is outside, the
    /// exit then the entry when it is inside.
    segment: Option<[Vec3; 2]>,
    /// Whether the polygon's last vertex is inside.
    last_inside: bool,
}

/// Cuts a convex polygon by a plane whose signed distance is `distance`
/// (inside ≤ 0), as the original walks it: from the last vertex's side,
/// the first run on the inner side and the crossings at its ends. A vertex
/// on the plane is kept once and makes its crossing; a NaN distance is
/// outside. An empty polygon gives nothing.
fn cut(polygon: &[Vec3], distance: impl Fn(Vec3) -> f32) -> Cut {
    let Some(&last) = polygon.last() else {
        return Cut {
            points: Vec::new(),
            segment: None,
            last_inside: false,
        };
    };
    let crossing = |a: Vec3, b: Vec3, t: f32| a + (b - a) * t;
    let n = polygon.len();
    let mut points = Vec::with_capacity(n + 1);
    let mut previous = n - 1;
    let mut before = distance(last);
    let last_inside = before <= 0.0;
    let mut first = None;
    let mut second = None;
    if !last_inside {
        // Outside to start: find the entry, keep the inner run, stop at
        // the exit.
        for (k, &vertex) in polygon.iter().enumerate() {
            let d = distance(vertex);
            if d <= 0.0 {
                first = Some(if d < 0.0 {
                    let p = crossing(polygon[previous], vertex, before / (before - d));
                    points.push(p);
                    p
                } else {
                    vertex
                });
                points.push(vertex);
                previous = k;
                before = d;
                break;
            }
            previous = k;
            before = d;
        }
        for (k, &vertex) in polygon.iter().enumerate().skip(previous + 1) {
            let d = distance(vertex);
            if !(d <= 0.0) {
                second = Some(if before < 0.0 {
                    let p = crossing(polygon[previous], vertex, before / (before - d));
                    points.push(p);
                    p
                } else {
                    polygon[previous]
                });
                break;
            }
            points.push(vertex);
            previous = k;
            before = d;
        }
    } else {
        // Inside to start: keep the run up to the exit, skip the outer
        // run, keep the rest from the entry.
        for (k, &vertex) in polygon.iter().enumerate() {
            let d = distance(vertex);
            if !(d <= 0.0) {
                first = Some(if before < 0.0 {
                    let p = crossing(polygon[previous], vertex, before / (before - d));
                    points.push(p);
                    p
                } else {
                    polygon[previous]
                });
                previous = k;
                before = d;
                break;
            }
            points.push(vertex);
            previous = k;
            before = d;
        }
        for (k, &vertex) in polygon.iter().enumerate().skip(previous + 1) {
            let d = distance(vertex);
            if d <= 0.0 {
                second = Some(if d < 0.0 {
                    let p = crossing(polygon[k - 1], vertex, before / (before - d));
                    points.push(p);
                    p
                } else {
                    vertex
                });
                previous = k - 1;
                break;
            }
            before = d;
        }
        points.extend_from_slice(&polygon[previous + 1..]);
    }
    let segment = match (first, second) {
        (Some(a), Some(b)) => Some([a, b]),
        _ => None,
    };
    Cut {
        points,
        segment,
        last_inside,
    }
}

/// Whether two vertices are the same point for edge and segment matching.
fn same_point(a: Vec3, b: Vec3) -> bool {
    (a - b).length_squared() < SAME_POINT2
}

/// Chains the cut segments into the cap polygon, from the first segment on:
/// each round takes the first remaining segment with an end at the chain's
/// end, appends its other end and moves the last segment into its place.
fn chain(segments: &mut Vec<[Vec3; 2]>, points: &mut Vec<Vec3>) {
    let mut i = 1;
    while i < segments.len() {
        let end = *points.last().expect("a chain starts with two points");
        let [a, b] = segments[i];
        let next = if same_point(a, end) {
            b
        } else if same_point(b, end) {
            a
        } else {
            i += 1;
            continue;
        };
        points.push(next);
        segments.swap_remove(i);
        i = 1;
    }
}

impl ConvexVolume {
    /// This volume in coordinates relative to `origin`: each plane's offset
    /// gains `origin · normal`, so the copy holds `x` when this volume
    /// holds `x + origin`. The polygons are not carried, as when the
    /// original rebases a volume.
    pub fn relative_to(&self, origin: Vec3) -> Self {
        Self {
            planes: self
                .planes
                .iter()
                .map(|plane| Plane::new(plane.normal, origin.dot(plane.normal) + plane.offset))
                .collect(),
        }
    }

    /// Rebuilds every face's plane from its polygon: the normal is
    /// `(v2 − v1) × (v0 − v1)`, normalised, and the offset puts `v0` on the
    /// plane. A polygon wound counter-clockwise seen from outside gets an
    /// outward normal. Faces with fewer than three vertices are left alone
    /// (the original reads three regardless).
    pub fn refresh(&mut self) {
        for plane in &mut self.planes {
            if plane.vertices.len() < 3 {
                continue;
            }
            let [a, b, c] = [plane.vertices[0], plane.vertices[1], plane.vertices[2]];
            plane.normal = normalised((c - b).cross(a - b));
            plane.offset = -plane.normal.dot(a);
        }
    }

    /// Whether the volume meets the oriented box, with the box's bounding
    /// sphere `radius`. Each face first rejects the box's bounding sphere
    /// (`radius < distance`) and then its support along the face normal,
    /// so the answer is conservative near a face's edges.
    pub fn intersects_obb(&self, obb: &Obb, radius: f32) -> bool {
        for plane in &self.planes {
            let distance = plane.distance(obb.centre);
            if radius < distance {
                return false;
            }
            if 0.0 < distance && support(obb.half_extents, obb.local(plane.normal)) < distance {
                return false;
            }
        }
        true
    }

    /// Whether the volume meets the world-aligned box `centre`,
    /// `half_extents` with bounding sphere `radius`, with the same
    /// rejections as [`ConvexVolume::intersects_obb`].
    pub fn intersects_aabb(&self, centre: Vec3, radius: f32, half_extents: Vec3) -> bool {
        for plane in &self.planes {
            let distance = plane.distance(centre);
            if distance > radius {
                return false;
            }
            if distance > 0.0 && distance > support(half_extents, plane.normal) {
                return false;
            }
        }
        true
    }

    /// How far the box lies outside the volume at its most exposed face,
    /// zero when it is inside or crosses a face: the largest face distance
    /// left after the box's support along that face is taken off.
    pub fn outside_distance(&self, obb: &Obb) -> f32 {
        let mut farthest = 0.0f32;
        for plane in &self.planes {
            let distance = plane.distance(obb.centre);
            if distance < 0.0 {
                continue;
            }
            let beyond = distance - support(obb.half_extents, obb.local(plane.normal));
            if !(beyond < 0.0) && farthest < beyond {
                farthest = beyond;
            }
        }
        farthest
    }

    /// Clips `polygon` by every face of this volume, in face order
    /// (stopping once nothing is left), and builds the pyramid from `apex`
    /// through what remains: one face per edge of the clipped polygon `p`,
    /// face i through `p[i − 1]`, the apex and `p[i]` (`p[−1]` the last),
    /// its plane refreshed from those three points ([`Self::refresh`]).
    /// Returns `None` when fewer than three points remain.
    pub fn clip_polygon(&self, polygon: &[Vec3], apex: Vec3) -> Option<ConvexVolume> {
        let mut points = polygon.to_vec();
        for plane in &self.planes {
            if points.is_empty() {
                break;
            }
            points = cut(&points, |p| plane.distance(p)).points;
        }
        if points.len() < 3 {
            return None;
        }
        let count = points.len();
        let mut volume = Self {
            planes: (0..count)
                .map(|i| Plane {
                    normal: Vec3::ZERO,
                    offset: 0.0,
                    vertices: vec![points[(i + count - 1) % count], apex, points[i]],
                })
                .collect(),
        };
        volume.refresh();
        Some(volume)
    }

    /// Cuts this volume by the plane `normal · x + offset = 0`, keeping the
    /// side where it is ≤ 0. Each face keeps its plane and the part of its
    /// polygon on that side, and is dropped when fewer than three points
    /// remain. The cut segments are chained into a cap polygon, starting
    /// from the first segment (turned round when the first face's last
    /// vertex is inside); when the chain does not close (its ends at least
    /// 0.01 cm apart) it is also extended from its start. A cap of three or
    /// more points becomes the last face, on the cutting plane. Faces
    /// without a polygon are dropped (the original reads one regardless).
    pub fn clip_by_plane(&mut self, normal: Vec3, offset: f32) {
        let distance = |p: Vec3| normal.dot(p) + offset;
        let mut planes = Vec::with_capacity(self.planes.len() + 1);
        let mut segments = Vec::new();
        let mut turn_first = false;
        for (i, plane) in self.planes.iter().enumerate() {
            let cut = cut(&plane.vertices, distance);
            if i == 0 && !plane.vertices.is_empty() {
                turn_first = cut.last_inside;
            }
            segments.extend(cut.segment);
            if cut.points.len() >= 3 {
                planes.push(Plane {
                    normal: plane.normal,
                    offset: plane.offset,
                    vertices: cut.points,
                });
            }
        }
        if segments.len() >= 2 {
            let [a, b] = segments[0];
            let mut cap = if turn_first { vec![b, a] } else { vec![a, b] };
            chain(&mut segments, &mut cap);
            let ends = (cap[0] - cap[cap.len() - 1]).length_squared();
            if f64::from(ends) < CLOSED2 {
                cap.pop();
            } else {
                cap.reverse();
                chain(&mut segments, &mut cap);
                cap.reverse();
            }
            if cap.len() >= 3 {
                planes.push(Plane {
                    normal,
                    offset,
                    vertices: cap,
                });
            }
        }
        self.planes = planes;
    }

    /// The volume a directional light's shadow casters can lie in for this
    /// (view) volume: `light` is the homogeneous light vector (the
    /// directional shadow passes the unit direction with w = 0). A face
    /// whose `normal · light.xyz + offset · light.w` is ≤ 0 is open to the
    /// light; the others are kept as planes without polygons. For each
    /// open face and each kept face, the first edge of the open face whose
    /// ends match an edge of the kept face (reversed, within √5 cm) and is
    /// at least 1 cm long becomes a side plane: its normal
    /// `light.xyz × edge`, normalised, through the edge, with the polygon
    /// of the edge and the edge moved 10 000 along `−light.xyz`. Returns
    /// the volume, side planes first, and how many side planes it has.
    pub fn shadow_volume(&self, light: Vec4) -> (ConvexVolume, usize) {
        let towards = light.truncate();
        let open: Vec<bool> = self
            .planes
            .iter()
            .map(|p| {
                let n = p.normal;
                (n.x * light.x + n.z * light.z) + (n.y * light.y + p.offset * light.w) <= 0.0
            })
            .collect();
        let mut planes = Vec::new();
        for (a, _) in self.planes.iter().zip(&open).filter(|(_, o)| **o) {
            for (b, _) in self.planes.iter().zip(&open).filter(|(_, o)| !**o) {
                if let Some(side) = silhouette_side(a, b, towards) {
                    planes.push(side);
                }
            }
        }
        let sides = planes.len();
        planes.extend(
            self.planes
                .iter()
                .zip(&open)
                .filter(|(_, o)| !**o)
                .map(|(p, _)| Plane::new(p.normal, p.offset)),
        );
        (ConvexVolume { planes }, sides)
    }
}

/// The side plane of [`ConvexVolume::shadow_volume`] for the open face `a`
/// and the kept face `b`, if they share a long enough edge.
fn silhouette_side(a: &Plane, b: &Plane, light: Vec3) -> Option<Plane> {
    let (va, vb) = (&a.vertices, &b.vertices);
    for k in 0..va.len() {
        let next = if k + 1 == va.len() { 0 } else { k + 1 };
        let shared = (0..vb.len()).any(|m| {
            let before = if m == 0 { vb.len() - 1 } else { m - 1 };
            same_point(va[k], vb[m]) && same_point(va[next], vb[before])
        });
        if !shared {
            continue;
        }
        let edge = va[next] - va[k];
        if edge.length_squared() < MIN_EDGE2 {
            continue;
        }
        let normal = normalised(light.cross(edge));
        return Some(Plane {
            normal,
            offset: -normal.dot(va[next]),
            vertices: vec![
                va[k],
                va[next],
                va[next] + light * EXTRUSION,
                va[k] + light * EXTRUSION,
            ],
        });
    }
    None
}

#[cfg(test)]
#[path = "intersect_test.rs"]
mod tests;
