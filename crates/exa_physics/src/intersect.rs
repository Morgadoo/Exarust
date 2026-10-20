//! Shared geometric intersection tests: rays and segments, spheres,
//! axis-aligned and oriented boxes, point sets and the convex volumes the
//! original clips portals and shadow volumes with. It corresponds to the
//! original's `RF_Intersection` unit.
//!
//! Lengths are the original's centimetres. Every routine is pure: it takes
//! plain vectors and returns a number or a bool, and it never allocates
//! except where it builds a result volume.
//!
//! The original normalises sweeps and stores the reciprocal direction with
//! the processor's approximate reciprocal instructions. Rust's `sqrt` and
//! reciprocal are correctly rounded, so the results agree with the original
//! to that approximation's precision, not bit for bit.
//!
//! Evidence: the intersection pack's decompilation (function bodies,
//! constants and callers) and the body-contact notes, which name the box,
//! sphere, sweep and segment tests this module reproduces.

use bevy::math::Vec3;

/// The smallest half extent a point set's bounds gets (original cm, 1e-4).
pub const MIN_BOUNDS_EXTENT: f32 = 1e-4;

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
    pub fn local_abs(&self, point: Vec3) -> Vec3 {
        self.local(point - self.centre).abs()
    }

    /// `normal` in the box's frame, whose components scale the extents when
    /// the box's support along the normal is taken.
    pub fn rotate(&self, normal: Vec3) -> Vec3 {
        self.local(normal)
    }
}

/// The squared distance from `point` to the segment from `origin` along
/// `direction` (unit) of the given `length`.
pub fn point_segment_distance_squared(
    point: Vec3,
    origin: Vec3,
    direction: Vec3,
    length: f32,
) -> f32 {
    let offset = point - origin;
    let mut along = offset.dot(direction);
    if along <= 0.0 {
        along = 0.0;
    }
    if length <= along {
        along = length;
    }
    (offset - direction * along).length_squared()
}

/// Whether two spheres are apart: the original returns the sign of
/// `(ra + rb)² − d²`, so touching counts as overlapping.
pub fn spheres_separated(a: Vec3, ra: f32, b: Vec3, rb: f32) -> bool {
    let reach = ra + rb;
    reach * reach - (b - a).length_squared() < 0.0
}

/// The distance² a point lies outside an axis-aligned box: zero inside,
/// otherwise the squared sum of the per-axis overhangs.
pub fn point_aabb_distance_squared(point: Vec3, centre: Vec3, half_extents: Vec3) -> f32 {
    let d = (point - centre).abs();
    let over = Vec3::new(
        if half_extents.x < d.x {
            d.x - half_extents.x
        } else {
            0.0
        },
        if half_extents.y < d.y {
            d.y - half_extents.y
        } else {
            0.0
        },
        if half_extents.z < d.z {
            d.z - half_extents.z
        } else {
            0.0
        },
    );
    over.length_squared()
}

/// Whether a sphere and an axis-aligned box are apart. Touching counts as
/// overlapping, as in [`spheres_separated`].
pub fn sphere_aabb_separated(
    centre: Vec3,
    radius: f32,
    box_centre: Vec3,
    half_extents: Vec3,
) -> bool {
    radius * radius - point_aabb_distance_squared(centre, box_centre, half_extents) < 0.0
}

/// Whether a sphere and a swept sphere (a capsule: a segment with a radius)
/// are apart. The segment is the sweep's origin and direction over its
/// length; `segment_radius` is the original's `+0x1c` field.
pub fn sphere_segment_separated(
    sweep: &Sweep,
    segment_radius: f32,
    centre: Vec3,
    radius: f32,
) -> bool {
    let reach = segment_radius + radius;
    reach * reach - sweep.point_distance_squared(centre) < 0.0
}

/// The per-axis slab entry and exit of a sweep against a box inflated by
/// `radius`, in sweep-length units. `offset` is the box centre relative to
/// the sweep origin, already in the box's frame.
fn slab_interval(offset: Vec3, extent: Vec3, inverse: Vec3) -> (f32, f32) {
    let low = (offset - extent) * inverse;
    let high = (offset + extent) * inverse;
    let entry = low.min(high).max_element();
    let exit = low.max(high).min_element();
    (entry, exit)
}

/// Whether the slab interval `entry..exit` meets a sweep, in sweep-length
/// units. The original clamps a negative entry to zero and rejects an exit
/// at or before it; touching at the entry therefore counts as a hit only
/// when something remains beyond it.
fn slabs_hit(entry: f32, exit: f32) -> bool {
    let start = if entry <= 0.0 { 0.0 } else { entry };
    exit > start
}

/// Whether a sphere of `radius` swept from `start` to `end` meets an
/// axis-aligned box. This is the original's slab test with the box inflated
/// by the radius; an entry at or past the sweep's length is rejected.
pub fn swept_sphere_aabb_hit(
    sweep: &Sweep,
    radius: f32,
    box_centre: Vec3,
    half_extents: Vec3,
) -> bool {
    let offset = box_centre - sweep.origin;
    let extent = half_extents + Vec3::splat(radius);
    let (entry, exit) = slab_interval(offset, extent, sweep.inverse_direction);
    entry < sweep.length && slabs_hit(entry, exit)
}

/// Whether a sphere of `radius` swept from `start` to `end` meets an
/// oriented box. The travel is normalised, so the entry is rejected at 1.
pub fn swept_sphere_obb_hit(start: Vec3, end: Vec3, radius: f32, obb: &Obb) -> bool {
    let offset = obb.local(obb.centre - start);
    let travel = obb.local(end - start);
    let extent = obb.half_extents + Vec3::splat(radius);
    let (entry, exit) = slab_interval(offset, extent, travel.recip());
    entry < 1.0 && slabs_hit(entry, exit)
}

/// Whether the segment from `start` to `end` meets an oriented box.
pub fn segment_obb_hit(start: Vec3, end: Vec3, obb: &Obb) -> bool {
    let offset = obb.local(obb.centre - start);
    let travel = obb.local(end - start);
    let (entry, exit) = slab_interval(offset, obb.half_extents, travel.recip());
    entry < 1.0 && slabs_hit(entry, exit)
}

/// The squared distance from `point` to an oriented box: zero inside,
/// otherwise the squared sum of the overhangs in the box's frame.
pub fn point_obb_distance_squared(point: Vec3, obb: &Obb) -> f32 {
    let local = obb.local_abs(point);
    let over = Vec3::new(
        if obb.half_extents.x < local.x {
            local.x - obb.half_extents.x
        } else {
            0.0
        },
        if obb.half_extents.y < local.y {
            local.y - obb.half_extents.y
        } else {
            0.0
        },
        if obb.half_extents.z < local.z {
            local.z - obb.half_extents.z
        } else {
            0.0
        },
    );
    over.length_squared()
}

/// Whether the sphere `centre`, `radius` meets the segment from `start`
/// along `vector`. `vector` is not normalised: its length is the segment's
/// length. Touching counts as meeting.
pub fn segment_sphere(start: Vec3, vector: Vec3, centre: Vec3, radius: f32) -> bool {
    let offset = centre - start;
    let along = vector.dot(offset);
    let squared = offset.length_squared();
    let distance = if along <= 0.0 {
        squared
    } else {
        let travel = vector.length_squared();
        if travel <= along {
            (offset - vector).length_squared()
        } else {
            squared - along * along / travel
        }
    };
    distance <= radius * radius
}

/// Whether the segment from `start` along `vector` meets an oriented box,
/// by the separating-axis test: the segment's start must not already be
/// outside a box slab and moving away from it, and the cross axes of the
/// segment and the box's axes must overlap too.
pub fn segment_obb_overlap(start: Vec3, vector: Vec3, obb: &Obb) -> bool {
    let local = start - obb.centre;
    let mut along = [0.0f32; 3];
    let mut travel = [0.0f32; 3];
    for i in 0..3 {
        along[i] = local.dot(obb.axes[i]);
        travel[i] = vector.dot(obb.axes[i]);
        // The start inside the slab, or the segment moving toward it, keeps
        // this axis from separating them.
        if !(along[i].abs() <= obb.half_extents[i] || along[i] * travel[i] < 0.0) {
            return false;
        }
    }
    let cross = local.cross(vector);
    for i in 0..3 {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        let extent = obb.half_extents[j] * travel[k].abs() + obb.half_extents[k] * travel[j].abs();
        if cross.dot(obb.axes[i]).abs() > extent {
            return false;
        }
    }
    true
}

/// The centre and half extents of a point set. The extents have a floor of
/// [`MIN_BOUNDS_EXTENT`], as the original's have.
pub fn bounds_of_points(points: &[Vec3]) -> (Vec3, Vec3) {
    let Some(&first) = points.first() else {
        return (Vec3::ZERO, Vec3::splat(MIN_BOUNDS_EXTENT));
    };
    let (mut low, mut high) = (first, first);
    for &point in &points[1..] {
        low = low.min(point);
        high = high.max(point);
    }
    let centre = (low + high) * 0.5;
    let extents = (high - centre).max(Vec3::splat(MIN_BOUNDS_EXTENT));
    (centre, extents)
}

/// Grows the bounds `low..high` to include `point`, as the original expands
/// a box while it collects a mesh's vertices.
pub fn expand_bounds(low: &mut Vec3, high: &mut Vec3, point: Vec3) {
    *low = low.min(point);
    *high = high.max(point);
}

/// The unit diagonals of the cube's side planes, in the original's table
/// order: the four planes of the +X face use the first four directions.
const CUBE_DIAGONALS: [Vec3; 6] = [
    Vec3::new(
        std::f32::consts::FRAC_1_SQRT_2,
        std::f32::consts::FRAC_1_SQRT_2,
        0.0,
    ),
    Vec3::new(
        std::f32::consts::FRAC_1_SQRT_2,
        -std::f32::consts::FRAC_1_SQRT_2,
        0.0,
    ),
    Vec3::new(
        std::f32::consts::FRAC_1_SQRT_2,
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
    ),
    Vec3::new(
        std::f32::consts::FRAC_1_SQRT_2,
        0.0,
        -std::f32::consts::FRAC_1_SQRT_2,
    ),
    Vec3::new(
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
        std::f32::consts::FRAC_1_SQRT_2,
    ),
    Vec3::new(
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
        -std::f32::consts::FRAC_1_SQRT_2,
    ),
];

/// The four side planes of each cube face, as (diagonal index, sign): the
/// face is not needed when the sphere lies wholly beyond one of them. Face
/// bits follow the original's face table: +X, −X, −Y, +Y, −Z, +Z.
const CUBE_FACE_PLANES: [[(usize, f32); 4]; 6] = [
    [(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0)],
    [(0, -1.0), (1, -1.0), (2, -1.0), (3, -1.0)],
    [(0, -1.0), (1, 1.0), (4, -1.0), (5, -1.0)],
    [(1, -1.0), (0, 1.0), (5, 1.0), (4, 1.0)],
    [(2, -1.0), (3, 1.0), (4, -1.0), (5, 1.0)],
    [(3, -1.0), (2, 1.0), (5, -1.0), (4, 1.0)],
];

/// The cube faces whose 90° frustum a node's bounding sphere touches, as a
/// mask of the six faces above. The light is the frustum's apex.
pub fn cube_face_mask(light: Vec3, centre: Vec3, radius: f32) -> u8 {
    let mut diagonal = [0.0f32; 6];
    for (i, axis) in CUBE_DIAGONALS.iter().enumerate() {
        // The original subtracts the two dot products, rather than taking
        // the dot product of the difference.
        let from_light = light.z * axis.z + light.y * axis.y + light.x * axis.x;
        let from_centre = centre.z * axis.z + centre.y * axis.y + centre.x * axis.x;
        diagonal[i] = from_centre - from_light;
    }
    let mut mask = 0x3f;
    for (face, planes) in CUBE_FACE_PLANES.iter().enumerate() {
        for &(axis, sign) in planes {
            let distance = -sign * diagonal[axis];
            // A sphere exactly on a side plane still touches it.
            if radius <= distance && distance != radius {
                mask &= !(1 << face);
                break;
            }
        }
    }
    mask
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

impl ConvexVolume {
    /// A copy of this volume with every face's plane moved by `by`: the
    /// polygon lists are not carried, as when the original translates a
    /// volume. The polygon lists are dropped because moving a plane does not
    /// move its polygon.
    pub fn translated(&self, by: Vec3) -> Self {
        Self {
            planes: self
                .planes
                .iter()
                .map(|plane| Plane::new(plane.normal, plane.offset + by.dot(plane.normal)))
                .collect(),
        }
    }

    /// Rebuilds every face's normal from its first three vertices and sets
    /// its offset from the first vertex. Faces with fewer than three
    /// vertices are left alone (the original has no such guard; malformed
    /// input would read past its list).
    pub fn refresh(&mut self) {
        for plane in &mut self.planes {
            if plane.vertices.len() < 3 {
                continue;
            }
            let [a, b, c] = [plane.vertices[0], plane.vertices[1], plane.vertices[2]];
            let normal = (a - b).cross(c - b);
            plane.normal = normal / normal.length();
            plane.offset = -plane.normal.dot(a);
        }
    }

    /// Whether the volume meets the oriented box, with the box's bounding
    /// sphere `radius` and extents. Each face first rejects the box's
    /// bounding sphere (`radius < distance`) and then its support along the
    /// face normal, so the answer is conservative near a face's edges.
    pub fn intersects_obb(&self, obb: &Obb, radius: f32) -> bool {
        for plane in &self.planes {
            let distance = plane.distance(obb.centre);
            if radius < distance {
                return false;
            }
            if distance > 0.0 {
                let turned = obb.rotate(plane.normal);
                let support = obb.half_extents.x * turned.x.abs()
                    + obb.half_extents.y * turned.y.abs()
                    + obb.half_extents.z * turned.z.abs();
                if support < distance {
                    return false;
                }
            }
        }
        true
    }

    /// Whether the volume meets the world-aligned box `centre`, `extents`,
    /// with the same bounding-sphere and per-face support rejections as
    /// [`ConvexVolume::intersects_obb`].
    pub fn intersects_aabb(&self, centre: Vec3, radius: f32, half_extents: Vec3) -> bool {
        for plane in &self.planes {
            let distance = plane.distance(centre);
            if distance > radius {
                return false;
            }
            if distance > 0.0 {
                let support = half_extents.x * plane.normal.x.abs()
                    + half_extents.y * plane.normal.y.abs()
                    + half_extents.z * plane.normal.z.abs();
                if support < distance {
                    return false;
                }
            }
        }
        true
    }

    /// How far the box lies outside the volume at its most exposed face,
    /// zero when it is inside or crosses a face. This is the largest
    /// face distance left after the box's support along that face is taken
    /// off.
    pub fn outside_distance(&self, obb: &Obb) -> f32 {
        let mut farthest = 0.0f32;
        for plane in &self.planes {
            let mut distance = plane.distance(obb.centre);
            if distance >= 0.0 {
                let turned = obb.rotate(plane.normal);
                let support = obb.half_extents.x * turned.x.abs()
                    + obb.half_extents.y * turned.y.abs()
                    + obb.half_extents.z * turned.z.abs();
                distance -= support;
                if distance >= 0.0 && farthest < distance {
                    farthest = distance;
                }
            }
        }
        farthest
    }

    /// Clips `polygon` by every face of this volume, in face order, and
    /// builds the pyramid from `apex` through the clipped polygon: one face
    /// per polygon edge, its normal and offset refreshed from its three
    /// vertices. Returns `None` when fewer than three points remain.
    ///
    /// The original's face-count bookkeeping is ambiguous in the
    /// decompilation (its plane loop and the count it stores differ by
    /// one); this builds the closed fan, one face per edge, which is what
    /// its vertex buffer is laid out for. See the report.
    pub fn clip_polygon(&self, polygon: &[Vec3], apex: Vec3) -> Option<ConvexVolume> {
        let mut points = polygon.to_vec();
        for plane in &self.planes {
            points = clip_polygon_by_plane(&points, plane.normal, plane.offset);
            if points.is_empty() {
                break;
            }
        }
        if points.len() < 3 {
            return None;
        }
        let count = points.len();
        let mut planes = Vec::with_capacity(count);
        for i in 0..count {
            let previous = points[(i + count - 1) % count];
            planes.push(Plane {
                normal: Vec3::ZERO,
                offset: 0.0,
                vertices: vec![previous, apex, points[i]],
            });
        }
        let mut volume = Self { planes };
        volume.refresh();
        Some(volume)
    }
}

/// Sutherland–Hodgman clipping of a polygon by one plane
/// `normal · x + offset = 0`. A vertex exactly on the plane is kept once;
/// the crossing point is added only when the two ends are strictly on
/// opposite sides, matching the original's branch order.
fn clip_polygon_by_plane(points: &[Vec3], normal: Vec3, offset: f32) -> Vec<Vec3> {
    let Some(&last) = points.last() else {
        return Vec::new();
    };
    let mut previous = last;
    let mut previous_distance = normal.dot(previous) + offset;
    let mut out = Vec::with_capacity(points.len() + 1);
    for &point in points {
        let distance = normal.dot(point) + offset;
        let crossing = (previous_distance < 0.0 && distance > 0.0)
            || (previous_distance > 0.0 && distance < 0.0);
        if crossing {
            let t = previous_distance / (previous_distance - distance);
            out.push(previous + (point - previous) * t);
        }
        if distance <= 0.0 {
            out.push(point);
        }
        previous = point;
        previous_distance = distance;
    }
    out
}

#[cfg(test)]
#[path = "intersect_test.rs"]
mod tests;
