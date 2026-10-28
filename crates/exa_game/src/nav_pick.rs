//! The original's picking routines: a screen point's ray, the point where it
//! meets a plane, the nearest mesh along it, the ground pick and the kinesis
//! pick. The cursor's aim plane (`crate::player::cursor_ground`) uses them;
//! the world cursor's reach is the hand's (`crate::hand`), which picks the
//! level and the props through the physics world directly.
//!
//! The original finds the nearest hit by walking its scene's sectors and
//! their geometry nodes, rejecting nodes by class, layer and option bits and
//! by their bounds (the segment against the bounding sphere, then against
//! the box), and asking each mesh's own ray callback for the nearest
//! triangle within the distance found so far. Exarust has no CPU copy of a
//! level's triangles, so the mesh callback is a trait: [`PhysicsMesh`]
//! answers from the physics world's fixed colliders, Exarust's stand-in for
//! the level's meshes. The class, layer, bounds and nearest-first rules are
//! the original's. Exarust works in world space; the original moves the
//! segment into each sector's and each mesh's frame first.

use bevy::math::Vec3;
use exa_physics::Physics;
use exa_physics::intersect::{Obb, segment_obb_hit, segment_sphere};

use crate::thaum::engine::length;

/// The class mask a pickable node matches: `class & 0x89 == 0x89`.
pub const PICK_CLASS: u32 = 0x89;
/// A node with this class bit is left out of every pick.
pub const NO_PICK: u32 = 0x0400_0000;
/// The option bit a node needs for the kinesis pick.
pub const KINESIS_NODE: u32 = 0x20_0000;
/// The distance a mesh pick starts from: the original's largest-float
/// stand-in, 3.4e38.
pub const MESH_FAR: f32 = f32::from_bits(0x7f7f_c99e);
/// How far a kinesis pick looks (the original's 4000 cm).
pub const KINESIS_FAR: f32 = 40.0;
/// How far the ground pick looks (the original's 4000 cm).
pub const GROUND_FAR: f32 = 40.0;
/// The sphere a node the ground pick's accept callback refuses grows by
/// (the original's 10 cm).
pub const GROUND_RADIUS_REFUSED: f32 = 0.1;
/// The sphere a node the accept callback takes grows by (the original's
/// 5 m: the whole reach of a raised ground surface).
pub const GROUND_RADIUS_ACCEPTED: f32 = 5.0;

/// A ray: an origin and a direction. The direction may be a unit vector or
/// a segment vector; distances are in its units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self { origin, direction }
    }

    /// The point `distance` along the ray.
    pub fn at(self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    /// The same ray with a unit direction, as both mesh picks normalise the
    /// viewport's near-to-far vector: scaled by the reciprocal of the
    /// vector's length, with no guard for a zero length.
    pub fn unit(self) -> Self {
        Self {
            origin: self.origin,
            direction: self.direction * (1.0 / length(self.direction)),
        }
    }
}

/// The screen ray the original's point-to-ray routine builds: the near
/// point is the origin and the far point less the near point the
/// direction, unnormalised. Exarust's camera gives the same pair through
/// Bevy's viewport unprojection; this keeps the original's shape so a pick
/// can be tested without a camera.
pub fn screen_ray(near: Vec3, far: Vec3) -> Ray {
    Ray::new(near, far - near)
}

/// A plane `normal · point = offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub offset: f32,
}

impl Plane {
    pub fn new(normal: Vec3, offset: f32) -> Self {
        Self { normal, offset }
    }

    /// A plane with normal `up` through a point.
    pub fn through(up: Vec3, point: Vec3) -> Self {
        Self {
            normal: up,
            offset: up.dot(point),
        }
    }
}

/// Where a ray meets a plane, and how far along the ray that is. The
/// original divides without a parallel-ray guard; a ray parallel to the
/// plane therefore gives an infinite or NaN distance, and the caller
/// rejects it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneHit {
    pub point: Vec3,
    pub distance: f32,
}

impl PlaneHit {
    /// The distance in world units: the ray parameter times the direction's
    /// length. The original's screen-plane routine records this in its
    /// global for the GUI drag to read; Exarust returns it instead.
    pub fn world_distance(&self, ray: Ray) -> f32 {
        self.distance * length(ray.direction)
    }
}

/// The original's plane pick: `(offset − normal·origin) / (normal·dir)`. The
/// dots are summed in the original's order, `n.z·p.z + (n.y·p.y + n.x·p.x)`.
pub fn plane_hit(ray: Ray, plane: Plane) -> PlaneHit {
    let dot = |a: Vec3| plane.normal.z * a.z + (plane.normal.y * a.y + plane.normal.x * a.x);
    let distance = (plane.offset - dot(ray.origin)) / dot(ray.direction);
    PlaneHit {
        point: ray.at(distance),
        distance,
    }
}

/// What both mesh picks record when they meet nothing: the distance along
/// the ray to the ground plane `y = 0`, `origin.y / −direction.y` (no
/// guard for a level ray).
pub fn floor_distance(ray: Ray) -> f32 {
    ray.origin.y / -ray.direction.y
}

/// A mesh's own ray test: the nearest face the ray meets within `within`
/// (in the direction's units) and its distance.
pub trait RayMesh {
    fn nearest_hit(&self, ray: Ray, within: f32) -> Option<(usize, f32)>;
}

/// One node a pick walks.
#[derive(Clone, Copy)]
pub struct Candidate<'a> {
    /// The node's class word; the pick tests it with [`PICK_CLASS`] and
    /// [`NO_PICK`].
    pub class: u32,
    /// The layer word; a pick with `layer_mask` visits the node when they
    /// share a bit.
    pub layers: u32,
    /// The node's option word; the kinesis pick needs [`KINESIS_NODE`].
    pub flags: u32,
    /// The node's box; its centre is also the bounding sphere's.
    pub bounds: Obb,
    /// The bounding sphere's radius.
    pub radius: f32,
    pub mesh: &'a dyn RayMesh,
}

impl<'a> Candidate<'a> {
    /// A pickable node whose bounds hold the whole segment from `origin`
    /// along `segment`: Exarust's stand-in for a level answered by one
    /// callback (the physics world), which has no boxes of its own.
    pub fn enclosing(origin: Vec3, segment: Vec3, mesh: &'a dyn RayMesh) -> Self {
        let half = segment.length() * 0.5;
        Self {
            class: PICK_CLASS,
            layers: u32::MAX,
            flags: 0,
            bounds: Obb::new(
                origin + segment * 0.5,
                Vec3::splat(half),
                [Vec3::X, Vec3::Y, Vec3::Z],
            ),
            radius: half,
            mesh,
        }
    }

    fn pickable(&self, layer_mask: u32) -> bool {
        self.class & PICK_CLASS == PICK_CLASS
            && self.layers & layer_mask != 0
            && self.class & NO_PICK != NO_PICK
    }
}

/// What a mesh pick found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// The candidate's index in the slice passed to [`nearest_hit`].
    pub node: usize,
    /// The face number its callback reported.
    pub face: usize,
    /// The distance along the ray (in the direction's units).
    pub distance: f32,
    pub point: Vec3,
}

/// The nearest mesh a ray meets, as the original's pick walks its nodes.
/// The segment is the unit `ray` times `length` (the original's near-to-far
/// distance). Each node passes the class, layer and no-pick bits, then the
/// segment must meet its bounding sphere and then its box; the mesh is
/// asked for a face within the best distance so far (which starts at
/// [`MESH_FAR`]), and a hit shortens the segment to it. When nothing is hit
/// the original records [`floor_distance`] instead.
pub fn nearest_hit(
    ray: Ray,
    length: f32,
    layer_mask: u32,
    candidates: &[Candidate<'_>],
) -> Option<Hit> {
    let mut best = MESH_FAR;
    let mut segment = ray.direction * length;
    let mut found = None;
    for (index, node) in candidates.iter().enumerate() {
        if !node.pickable(layer_mask)
            || !segment_sphere(ray.origin, segment, node.bounds.centre, node.radius)
            || !segment_obb_hit(ray.origin, segment, &node.bounds)
        {
            continue;
        }
        if let Some((face, distance)) = node.mesh.nearest_hit(ray, best) {
            segment = ray.direction * distance;
            best = distance;
            found = Some((index, face, distance));
        }
    }
    found.map(|(node, face, distance)| Hit {
        node,
        face,
        distance,
        point: ray.at(distance),
    })
}

/// The kinesis pick: whether the unit `ray` meets a kinesis node within
/// [`KINESIS_FAR`], and the distance it reports. Each node passes the class,
/// layer and no-pick bits and needs [`KINESIS_NODE`]; its bounding sphere
/// grows by `height` for the segment test, and the mesh is asked from the
/// point where the ray crosses that sphere's top (`(centre.y + radius +
/// height − origin.y) / direction.y` along it) for what is left of the
/// distance; a hit there sets the distance to the crossing plus the mesh's
/// distance and shortens the segment. The original keeps no node; with no
/// hit the distance is [`floor_distance`].
///
/// [guess: the original's callback for this pick reports a count of hits
/// along a segment and writes the distance back; the nearest face stands in
/// for it]
pub fn kinesis_hit(
    ray: Ray,
    layer_mask: u32,
    height: f32,
    candidates: &[Candidate<'_>],
) -> (bool, f32) {
    let rise = 1.0 / ray.direction.y;
    let mut far = KINESIS_FAR;
    let mut segment = ray.direction * far;
    let mut found = false;
    for node in candidates {
        if !node.pickable(layer_mask) || node.flags & KINESIS_NODE == 0 {
            continue;
        }
        let radius = node.radius + height;
        if !segment_sphere(ray.origin, segment, node.bounds.centre, radius) {
            continue;
        }
        let top = (node.bounds.centre.y + radius - ray.origin.y) * rise;
        let start = ray.at(top);
        if let Some((_, distance)) = node
            .mesh
            .nearest_hit(Ray::new(start, ray.direction), far - top)
        {
            far = distance + top;
            segment = ray.direction * far;
            found = true;
        }
    }
    if found {
        (true, far)
    } else {
        (false, floor_distance(ray))
    }
}

/// The ground pick: the highest surface under the cursor, as the original's
/// point-to-ground routine walks its nodes. It is the nearest-mesh pick with
/// two differences: each node's bounding sphere grows by
/// [`GROUND_RADIUS_ACCEPTED`] when the caller's accept test takes it and by
/// [`GROUND_RADIUS_REFUSED`] when it does not; and the node's mesh is asked
/// from the point where the ray crosses the grown sphere's top, for the
/// stretch that is left of the [`GROUND_FAR`] reach. The first node whose
/// mesh answers keeps the pick; a later, nearer one still wins because the
/// segment shortens (the original keeps the least distance reported). The
/// distance is [`floor_distance`] when nothing is found.
///
/// The original's accept test is a per-node callback and its mesh test
/// counts the ground surfaces under a column; Exarust has neither, so the
/// accept test is the caller's closure and [`RayMesh`]'s nearest hit stands
/// in for the column. The crossing uses the engine's `1 / direction.y`,
/// held to zero when it is positive (the ray looks up): a level ray gives
/// an infinite crossing, as the original's division does.
pub fn ground_hit(
    ray: Ray,
    layer_mask: u32,
    accept: impl Fn(&Candidate<'_>) -> bool,
    candidates: &[Candidate<'_>],
) -> (Option<Hit>, f32) {
    let ratio = 1.0 / ray.direction.y;
    let ratio = if ratio > 0.0 { 0.0 } else { ratio };
    let mut far = GROUND_FAR;
    let mut segment = ray.direction * far;
    let mut found = None;
    for (index, node) in candidates.iter().enumerate() {
        if !node.pickable(layer_mask) {
            continue;
        }
        let grow = if accept(node) {
            GROUND_RADIUS_ACCEPTED
        } else {
            GROUND_RADIUS_REFUSED
        };
        let radius = node.radius + grow;
        if !segment_sphere(ray.origin, segment, node.bounds.centre, radius) {
            continue;
        }
        let crossing = (node.bounds.centre.y + radius - ray.origin.y) * ratio;
        // The original keeps the crossed point for a positive crossing and
        // for a NaN one (its unordered compare jumps to the same branch).
        let start = if crossing > 0.0 || crossing.is_nan() {
            ray.at(crossing)
        } else {
            ray.origin
        };
        if let Some((face, distance)) = node
            .mesh
            .nearest_hit(Ray::new(start, ray.direction), far - crossing)
        {
            far = distance + crossing;
            segment = ray.direction * far;
            found = Some((index, face, far));
        }
    }
    match found {
        Some((node, face, distance)) => (
            Some(Hit {
                node,
                face,
                distance,
                point: ray.at(distance),
            }),
            distance,
        ),
        None => (None, floor_distance(ray)),
    }
}

/// A mesh made of triangles: the callback a CPU mesh gives the pick. The
/// original's triangles come from its kd-tree; the test here is the
/// usual two-sided barycentric one and a face counts when it is nearer
/// than `within` (guess: the tree's own triangle test was not read),
/// returning the nearest face.
#[derive(Clone, Copy, Debug, Default)]
pub struct Triangles<'a>(pub &'a [[Vec3; 3]]);

impl RayMesh for Triangles<'_> {
    fn nearest_hit(&self, ray: Ray, within: f32) -> Option<(usize, f32)> {
        let mut best = within;
        let mut face = None;
        for (index, triangle) in self.0.iter().enumerate() {
            if let Some(distance) = ray_triangle(ray, triangle)
                && distance < best
            {
                best = distance;
                face = Some(index);
            }
        }
        face.map(|face| (face, best))
    }
}

/// The distance along a two-sided ray to a triangle, if it meets it ahead.
fn ray_triangle(ray: Ray, triangle: &[Vec3; 3]) -> Option<f32> {
    let edge1 = triangle[1] - triangle[0];
    let edge2 = triangle[2] - triangle[0];
    let p = ray.direction.cross(edge2);
    let determinant = edge1.dot(p);
    if determinant.abs() < 1e-9 {
        return None;
    }
    let inverse = 1.0 / determinant;
    let offset = ray.origin - triangle[0];
    let u = offset.dot(p) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = offset.cross(edge1);
    let v = ray.direction.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = edge2.dot(q) * inverse;
    (distance >= 0.0).then_some(distance)
}

/// A mesh answered by the physics world: the level's static geometry, its
/// fixed colliders. Exarust stands it in for the original's mesh
/// callbacks; `far` caps the cast besides the pick's own distance.
pub struct PhysicsMesh<'a> {
    pub physics: &'a Physics,
    pub far: f32,
}

impl RayMesh for PhysicsMesh<'_> {
    fn nearest_hit(&self, ray: Ray, within: f32) -> Option<(usize, f32)> {
        self.physics
            .fixed_hit(ray.origin, ray.direction, self.far.min(within))
            .map(|distance| (0, distance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ray(origin: Vec3, direction: Vec3) -> Ray {
        Ray::new(origin, direction)
    }

    /// A node of class 0x89 on `layers` bounded by a cube of half size
    /// `half` about `centre` (its sphere the cube's half diagonal).
    fn node<'a>(layers: u32, centre: Vec3, half: f32, mesh: &'a dyn RayMesh) -> Candidate<'a> {
        Candidate {
            class: PICK_CLASS,
            layers,
            flags: 0,
            bounds: Obb::new(centre, Vec3::splat(half), [Vec3::X, Vec3::Y, Vec3::Z]),
            radius: half * 3.0f32.sqrt(),
            mesh,
        }
    }

    fn triangle_at(z: f32) -> [[Vec3; 3]; 1] {
        [[
            Vec3::new(-1.0, -1.0, z),
            Vec3::new(1.0, -1.0, z),
            Vec3::new(0.0, 1.0, z),
        ]]
    }

    #[test]
    fn a_plane_pick_returns_the_point_and_its_distance() {
        let hit = plane_hit(
            ray(Vec3::new(1.0, 5.0, 2.0), Vec3::new(0.0, -1.0, 0.0)),
            Plane::new(Vec3::Y, 0.8),
        );
        assert!((hit.distance - 4.2).abs() < 1e-6);
        assert!((hit.point - Vec3::new(1.0, 0.8, 2.0)).length() < 1e-6);
        // A plane through a point, aimed up.
        let hit = plane_hit(
            ray(Vec3::ZERO, Vec3::Y * 2.0),
            Plane::through(Vec3::Y, Vec3::new(0.0, 3.0, 0.0)),
        );
        assert!((hit.distance - 1.5).abs() < 1e-6);
        assert!((hit.point.y - 3.0).abs() < 1e-6);
        // A ray parallel to the plane has no finite point.
        let hit = plane_hit(
            ray(Vec3::new(0.0, 1.0, 0.0), Vec3::X),
            Plane::new(Vec3::Y, 0.0),
        );
        assert!(!hit.distance.is_finite());
        // The world distance counts the direction's length: a 2 m direction
        // halves the parameter.
        let r = ray(Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, -2.0, 0.0));
        let hit = plane_hit(r, Plane::new(Vec3::Y, 0.0));
        assert_eq!(hit.distance, 1.0);
        assert_eq!(hit.world_distance(r), 2.0);
    }

    #[test]
    fn a_miss_records_the_ground_plane() {
        // 5 m up looking 45° down: the ground is 5·√2 along.
        let d = Vec3::new(0.0, -1.0, 1.0).normalize();
        assert_eq!(floor_distance(ray(Vec3::new(0.0, 5.0, 0.0), d)), 5.0 / -d.y);
        // A level ray has no finite distance.
        assert!(!floor_distance(ray(Vec3::Y, Vec3::X)).is_finite());
    }

    #[test]
    fn a_triangle_mesh_reports_its_nearest_face_within_the_distance() {
        let flat = [triangle_at(1.0)[0], triangle_at(2.0)[0]];
        let mesh = Triangles(&flat);
        let (face, distance) = mesh
            .nearest_hit(ray(Vec3::ZERO, Vec3::Z), MESH_FAR)
            .unwrap();
        assert_eq!(face, 0);
        assert!((distance - 1.0).abs() < 1e-6);
        // Nothing nearer than the distance so far.
        assert!(mesh.nearest_hit(ray(Vec3::ZERO, Vec3::Z), 0.5).is_none());
        // Missing the triangles, or looking away, is no hit.
        assert!(
            mesh.nearest_hit(ray(Vec3::new(2.5, 2.5, 0.0), Vec3::Z), MESH_FAR)
                .is_none()
        );
        assert!(
            mesh.nearest_hit(ray(Vec3::ZERO, -Vec3::Z), MESH_FAR)
                .is_none()
        );
    }

    #[test]
    fn the_pick_filters_by_class_layer_and_flag_and_keeps_the_nearest() {
        let near = triangle_at(1.0);
        let far = triangle_at(3.0);
        let near = Triangles(&near);
        let far = Triangles(&far);
        let r = ray(Vec3::ZERO, Vec3::Z);
        let mut candidates = [
            node(1, Vec3::new(0.0, 0.0, 3.0), 1.0, &far),
            node(2, Vec3::new(0.0, 0.0, 1.0), 1.0, &near),
            node(2, Vec3::new(0.0, 0.0, 1.0), 1.0, &near),
            node(2, Vec3::new(0.0, 0.0, 1.0), 1.0, &near),
        ];
        candidates[2].class = 0x09;
        candidates[3].class |= NO_PICK;
        // Layer 2 picks the near mesh (candidate 1); layer 1 the far one.
        let hit = nearest_hit(r, 10.0, 2, &candidates).unwrap();
        assert_eq!((hit.node, hit.face), (1, 0));
        assert!((hit.distance - 1.0).abs() < 1e-6);
        let hit = nearest_hit(r, 10.0, 1, &candidates).unwrap();
        assert_eq!(hit.node, 0);
        assert!((hit.distance - 3.0).abs() < 1e-6);
        // The class and no-pick filters leave nothing.
        assert!(nearest_hit(r, 10.0, u32::MAX, &candidates[2..]).is_none());
        // The no-pick bit is a class bit, not an option bit.
        candidates[3].class &= !NO_PICK;
        candidates[3].flags = NO_PICK;
        assert_eq!(
            nearest_hit(r, 10.0, u32::MAX, &candidates[3..])
                .unwrap()
                .node,
            0
        );
        // A segment too short to reach the bounds rejects the mesh (the
        // mesh itself is asked within the best distance, not the segment).
        assert!(nearest_hit(r, 0.1, 1, &candidates).is_none());
        // A bounding sphere off the ray rejects the mesh.
        let off = [node(2, Vec3::new(10.0, 0.0, 1.0), 1.0, &near)];
        assert!(nearest_hit(r, 10.0, 2, &off).is_none());
    }

    #[test]
    fn a_hit_shortens_the_segment_for_the_nodes_after_it() {
        // The near mesh first: the far node's sphere (radius 0.5 about
        // z = 3) is then beyond the 1 m segment, so it is not asked.
        let near = triangle_at(1.0);
        let near = Triangles(&near);
        struct Panics;
        impl RayMesh for Panics {
            fn nearest_hit(&self, _: Ray, _: f32) -> Option<(usize, f32)> {
                panic!("the shortened segment does not reach this node")
            }
        }
        let mut far = node(1, Vec3::new(0.0, 0.0, 3.0), 0.25, &Panics);
        far.radius = 0.5;
        let candidates = [node(1, Vec3::new(0.0, 0.0, 1.0), 1.0, &near), far];
        let hit = nearest_hit(ray(Vec3::ZERO, Vec3::Z), 10.0, 1, &candidates).unwrap();
        assert_eq!(hit.node, 0);
    }

    #[test]
    fn an_enclosing_node_holds_the_whole_segment() {
        let wall = triangle_at(7.0);
        let wall = Triangles(&wall);
        let r = ray(Vec3::ZERO, Vec3::Z);
        let level = Candidate::enclosing(r.origin, r.direction * 10.0, &wall);
        let hit = nearest_hit(r, 10.0, u32::MAX, &[level]).unwrap();
        assert!((hit.point - Vec3::new(0.0, 0.0, 7.0)).length() < 1e-6);
    }

    #[test]
    fn the_kinesis_pick_needs_the_node_flag_and_asks_from_the_lifted_top() {
        // Looking down from 10 m at a node about the origin, radius 1.
        let floor = [[
            Vec3::new(-5.0, 0.0, -5.0),
            Vec3::new(5.0, 0.0, -5.0),
            Vec3::new(0.0, 0.0, 5.0),
        ]];
        let mesh = Triangles(&floor);
        let r = ray(Vec3::new(0.0, 10.0, 0.0), -Vec3::Y);
        let mut target = node(u32::MAX, Vec3::ZERO, 0.5, &mesh);
        target.radius = 1.0;
        target.flags = KINESIS_NODE;
        // The top of the sphere grown by 2 m is 3 m up, 7 m along; the
        // mesh is met 3 m further: 10 m in all.
        assert_eq!(kinesis_hit(r, u32::MAX, 2.0, &[target]), (true, 10.0));
        // Without the flag the node is not picked, and the miss records
        // the ground plane: 10 m.
        target.flags = 0;
        assert_eq!(kinesis_hit(r, u32::MAX, 2.0, &[target]), (false, 10.0));
        // A layer the pick does not visit rejects it.
        target.flags = KINESIS_NODE;
        assert!(!kinesis_hit(r, 0, 2.0, &[target]).0);
        // Out past 40 m the segment never reaches the sphere.
        let high = ray(Vec3::new(0.0, 50.0, 0.0), -Vec3::Y);
        assert_eq!(kinesis_hit(high, u32::MAX, 2.0, &[target]), (false, 50.0));
        // The height grows the sphere: beside the node, 2 m off, only a
        // lift of 2 m reaches it (the mesh is the wide floor).
        let beside = ray(Vec3::new(2.0, 10.0, 0.0), -Vec3::Y);
        assert!(!kinesis_hit(beside, u32::MAX, 0.0, &[target]).0);
        assert!(kinesis_hit(beside, u32::MAX, 2.0, &[target]).0);
    }

    #[test]
    fn the_bounds_test_is_the_segment_against_the_sphere() {
        let near = triangle_at(1.0);
        let near = Triangles(&near);
        let r = ray(Vec3::ZERO, Vec3::Z);
        let mut behind = node(1, Vec3::new(0.0, 0.0, -1.0), 0.25, &near);
        // Behind the start: only within the radius of the start.
        behind.radius = 0.5;
        assert!(nearest_hit(r, 10.0, 1, &[behind]).is_none());
        behind.bounds.centre.z = -0.2;
        behind.bounds.half_extents = Vec3::splat(0.5);
        // Now both bounds hold the start; the mesh itself is met at 1 m.
        assert!(nearest_hit(r, 10.0, 1, &[behind]).is_some());
    }

    #[test]
    fn a_screen_ray_runs_from_near_to_far() {
        let near = Vec3::new(1.0, 2.0, 3.0);
        let far = Vec3::new(1.0, 2.0, 13.0);
        let r = screen_ray(near, far);
        assert_eq!(r.origin, near);
        assert_eq!(r.direction, Vec3::new(0.0, 0.0, 10.0));
        // The picks use the unit form; the length is summed (x² + z²) + y².
        let unit = r.unit();
        assert_eq!(unit.direction, Vec3::Z);
        let unit = screen_ray(Vec3::ZERO, Vec3::new(3.0, 4.0, 0.0)).unit();
        assert_eq!(unit.direction, Vec3::new(0.6, 0.8, 0.0));
        // A zero vector has no guard: the direction is NaN.
        assert!(
            screen_ray(Vec3::ZERO, Vec3::ZERO)
                .unit()
                .direction
                .x
                .is_nan()
        );
    }

    #[test]
    fn the_ground_pick_grows_the_sphere_and_asks_from_its_top() {
        // A floor about the origin, met straight down from 10 m.
        let floor = [[
            Vec3::new(-20.0, 0.0, -20.0),
            Vec3::new(20.0, 0.0, -20.0),
            Vec3::new(0.0, 0.0, 20.0),
        ]];
        let mesh = Triangles(&floor);
        let down = ray(Vec3::new(0.0, 10.0, 0.0), -Vec3::Y);
        let mut target = node(u32::MAX, Vec3::ZERO, 0.5, &mesh);
        target.radius = 0.5;
        // Taken: the sphere grows 5 m, its top is 5.5 m up (4.5 m along),
        // and the floor is 5.5 m further: 10 m in all, face 0.
        let (hit, distance) = ground_hit(down, u32::MAX, |_| true, &[target]);
        let hit = hit.unwrap();
        assert_eq!((hit.node, hit.face), (0, 0));
        assert_eq!(distance, 10.0);
        assert!((hit.point - Vec3::ZERO).length() < 1e-5);
        // Refused: the sphere only grows 10 cm, the top is 0.6 m up
        // (9.4 m along) and the floor 0.6 m further: still 10 m.
        let (hit, distance) = ground_hit(down, u32::MAX, |_| false, &[target]);
        assert_eq!(hit.unwrap().distance, 10.0);
        assert_eq!(distance, 10.0);
        // Beside the node by 3 m, only the taken 5 m sphere reaches the
        // floor; the refused 0.6 m sphere misses, and the miss records the
        // ground plane below the eye: 10 m again but no hit.
        let beside = ray(Vec3::new(3.0, 10.0, 0.0), -Vec3::Y);
        assert!(
            ground_hit(beside, u32::MAX, |_| true, &[target])
                .0
                .is_some()
        );
        let (none, distance) = ground_hit(beside, u32::MAX, |_| false, &[target]);
        assert!(none.is_none());
        assert_eq!(distance, 10.0);
        // The layer and class filters apply as in the nearest pick.
        assert!(ground_hit(down, 0, |_| true, &[target]).0.is_none());
        target.class = 0x09;
        assert!(ground_hit(down, u32::MAX, |_| true, &[target]).0.is_none());
        // A level ray gives an infinite crossing ratio, held to zero: the
        // mesh is asked from the eye itself.
        let level = ray(Vec3::new(0.0, 5.0, -2.0), Vec3::Z);
        assert!(ground_hit(level, u32::MAX, |_| true, &[target]).0.is_none());
        assert_eq!(floor_distance(level), f32::NEG_INFINITY);
    }

    #[test]
    fn a_ground_pick_keeps_the_last_node_that_answers_within_its_reach() {
        // Two floors along the same downward ray: a taken high one nearer
        // the eye and a refused low one. The second is asked with what is
        // left after the first, so both are met and the later one is the
        // pick (the original writes the node and face on every hit).
        let high = [[
            Vec3::new(-20.0, 6.0, -20.0),
            Vec3::new(20.0, 6.0, -20.0),
            Vec3::new(0.0, 6.0, 20.0),
        ]];
        let low = [[
            Vec3::new(-20.0, 0.0, -20.0),
            Vec3::new(20.0, 0.0, -20.0),
            Vec3::new(0.0, 0.0, 20.0),
        ]];
        let high = Triangles(&high);
        let low = Triangles(&low);
        let down = ray(Vec3::new(0.0, 10.0, 0.0), -Vec3::Y);
        let mut near = node(1, Vec3::ZERO, 0.5, &high);
        near.radius = 0.5;
        let mut far = node(1, Vec3::ZERO, 0.5, &low);
        far.radius = 0.5;
        let (hit, distance) = ground_hit(down, 1, |_| true, &[near, far]);
        let hit = hit.unwrap();
        assert_eq!(hit.node, 1);
        assert_eq!(distance, 10.0);
        assert!((hit.point.y).abs() < 1e-5);
    }
}
