//! The original's picking routines: a screen point's ray, the point where it
//! meets a plane, the nearest mesh along it and the kinesis pick. The
//! cursor's aim plane (`crate::player::cursor_ground`) and the world
//! cursor's reach use them.
//!
//! The original finds the nearest hit by walking its scene's sectors and
//! their geometry nodes, rejecting nodes by class and layer and by their
//! bounding sphere, and asking each mesh's own ray callback for the nearest
//! triangle. Exarust has no CPU copy of a level's triangles, so the mesh
//! callback is a trait: [`PhysicsMesh`] answers from the physics world,
//! which is Exarust's stand-in for the level's meshes. The class, layer and
//! bounding rules are the original's.

use bevy::math::Vec3;
use exa_physics::Physics;

/// The class mask a pickable node matches: `class & 0x89 == 0x89`.
pub const PICK_CLASS: u32 = 0x89;
/// A node with this class bit is left out of the pick.
pub const NO_PICK: u32 = 0x0400_0000;
/// The farthest a mesh pick looks (the original starts from the largest
/// single-precision float).
pub const MESH_FAR: f32 = 3.4e38;
/// The sphere radius a ground pick uses for a small node, and for one its
/// callback calls large.
pub const GROUND_SMALL_RADIUS: f32 = 0.1;
pub const GROUND_LARGE_RADIUS: f32 = 5.0;
/// The distance a kinesis pick starts from (world units).
pub const KINESIS_FAR: f32 = 4000.0;

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

/// The original's plane pick: `(offset − normal·origin) / (normal·dir)`.
pub fn plane_hit(ray: Ray, plane: Plane) -> PlaneHit {
    let distance = (plane.offset - plane.normal.dot(ray.origin)) / plane.normal.dot(ray.direction);
    PlaneHit {
        point: ray.at(distance),
        distance,
    }
}

/// A mesh's own ray test: the nearest face and the distance along the ray.
pub trait RayMesh {
    fn nearest_hit(&self, ray: Ray) -> Option<(usize, f32)>;
}

/// One node a pick walks.
#[derive(Clone, Copy)]
pub struct Candidate<'a> {
    /// The node's class word; the pick tests it with [`PICK_CLASS`].
    pub class: u32,
    /// The layer word; a pick with `layer_mask` visits the node when they
    /// share a bit.
    pub layers: u32,
    /// The node's option word; [`NO_PICK`] leaves it out.
    pub flags: u32,
    /// The node's bounding sphere.
    pub centre: Vec3,
    pub radius: f32,
    pub mesh: &'a dyn RayMesh,
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

/// The nearest mesh a ray meets, as the original's pick walks its nodes:
/// class and layer first, then the bounding sphere, then the mesh, keeping
/// the nearest. The original starts from the largest float and keeps a hit
/// only when it is strictly nearer.
pub fn nearest_hit(ray: Ray, layer_mask: u32, candidates: &[Candidate<'_>]) -> Option<Hit> {
    let mut best = MESH_FAR;
    let mut found = None;
    for (index, node) in candidates.iter().enumerate() {
        if node.class & PICK_CLASS != PICK_CLASS
            || node.layers & layer_mask == 0
            || node.flags & NO_PICK != 0
        {
            continue;
        }
        if !ray_meets_sphere(ray, node.centre, node.radius) {
            continue;
        }
        if let Some((face, distance)) = node.mesh.nearest_hit(ray)
            && distance < best
        {
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

/// The nearest node a kinesis pick meets, with the node's box lifted by
/// `height` for the test and the node flag the original requires
/// (`0x200000`): returns the candidate index and distance. This is the
/// variant whose mesh callback only reports whether it was hit.
pub fn kinesis_hit(
    ray: Ray,
    layer_mask: u32,
    height: f32,
    candidates: &[Candidate<'_>],
) -> Option<(usize, f32)> {
    let mut best = MESH_FAR;
    let mut found = None;
    for (index, node) in candidates.iter().enumerate() {
        if node.class & PICK_CLASS != PICK_CLASS
            || node.layers & layer_mask == 0
            || node.flags & NO_PICK != 0
            || node.flags & KINESIS_NODE == 0
        {
            continue;
        }
        let centre = node.centre + Vec3::Y * height;
        if !ray_meets_sphere(ray, centre, node.radius) {
            continue;
        }
        if let Some((_, distance)) = node.mesh.nearest_hit(ray)
            && distance < best
        {
            best = distance;
            found = Some((index, distance));
        }
    }
    found
}

/// The node flag a kinesis pick requires (the original tests `0x200000` in
/// the node's option word).
pub const KINESIS_NODE: u32 = 0x20_0000;

/// The original's node bounding test: the ray from `origin` along
/// `direction` comes within `radius` of the sphere's centre. It keeps the
/// processor's arithmetic (no square root) and its branches.
pub fn ray_meets_sphere(ray: Ray, centre: Vec3, radius: f32) -> bool {
    let offset = centre - ray.origin;
    let dot = ray.direction.dot(offset);
    let offset_squared = offset.length_squared();
    let radius_squared = radius * radius;
    if dot <= 0.0 {
        offset_squared <= radius_squared
    } else if ray.direction.length_squared() <= dot {
        (offset - ray.direction).length_squared() <= radius_squared
    } else {
        offset_squared - dot * dot / ray.direction.length_squared() <= radius_squared
    }
}

/// A mesh made of triangles: the callback a CPU mesh gives the pick. The
/// original's triangles come from its kd-tree; the test here is the
/// usual two-sided barycentric one (guess: the tree's own triangle test
/// was not read), returning the nearest face.
#[derive(Clone, Copy, Debug, Default)]
pub struct Triangles<'a>(pub &'a [[Vec3; 3]]);

impl RayMesh for Triangles<'_> {
    fn nearest_hit(&self, ray: Ray) -> Option<(usize, f32)> {
        let mut best = f32::INFINITY;
        let mut face = 0;
        for (index, triangle) in self.0.iter().enumerate() {
            if let Some(distance) = ray_triangle(ray, triangle)
                && distance < best
            {
                best = distance;
                face = index;
            }
        }
        best.is_finite().then_some((face, best))
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

/// A mesh answered by the physics world: the level's static geometry and
/// the bodies in it. Exarust stands it in for the original's mesh callbacks.
pub struct PhysicsMesh<'a> {
    pub physics: &'a Physics,
    pub far: f32,
}

impl RayMesh for PhysicsMesh<'_> {
    fn nearest_hit(&self, ray: Ray) -> Option<(usize, f32)> {
        self.physics
            .fixed_hit(ray.origin, ray.direction, self.far)
            .map(|distance| (0, distance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ray(origin: Vec3, direction: Vec3) -> Ray {
        Ray::new(origin, direction)
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
    }

    #[test]
    fn a_triangle_mesh_reports_its_nearest_face() {
        let triangles = [
            [[
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(0.0, 1.0, 1.0),
            ]],
            [[
                Vec3::new(0.0, 0.0, 2.0),
                Vec3::new(2.0, 0.0, 2.0),
                Vec3::new(0.0, 2.0, 2.0),
            ]],
        ];
        let flat = [triangles[0][0], triangles[1][0]];
        let mesh = Triangles(&flat);
        let (face, distance) = mesh
            .nearest_hit(ray(Vec3::new(0.1, 0.1, 0.0), Vec3::Z))
            .unwrap();
        assert_eq!(face, 0);
        assert!((distance - 1.0).abs() < 1e-6);
        // Missing the triangles, or looking away, is no hit.
        assert!(
            mesh.nearest_hit(ray(Vec3::new(2.5, 2.5, 0.0), Vec3::Z))
                .is_none()
        );
        assert!(
            mesh.nearest_hit(ray(Vec3::new(0.1, 0.1, 0.0), -Vec3::Z))
                .is_none()
        );
    }

    #[test]
    fn the_pick_filters_by_class_layer_and_flag_and_keeps_the_nearest() {
        let near = [[
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ]];
        let far = [[
            Vec3::new(-1.0, -1.0, 3.0),
            Vec3::new(1.0, -1.0, 3.0),
            Vec3::new(0.0, 1.0, 3.0),
        ]];
        let near = Triangles(&near);
        let far = Triangles(&far);
        let ray = ray(Vec3::new(0.0, 0.0, 0.0), Vec3::Z);
        let candidates = [
            Candidate {
                class: 0x89,
                layers: 1,
                flags: 0,
                centre: Vec3::new(0.0, 0.0, 1.0),
                radius: 5.0,
                mesh: &far,
            },
            Candidate {
                class: 0x89,
                layers: 2,
                flags: 0,
                centre: Vec3::new(0.0, 0.0, 3.0),
                radius: 5.0,
                mesh: &near,
            },
            Candidate {
                class: 0x09,
                layers: 2,
                flags: 0,
                centre: Vec3::new(0.0, 0.0, 1.0),
                radius: 5.0,
                mesh: &near,
            },
            Candidate {
                class: 0x89,
                layers: 2,
                flags: NO_PICK,
                centre: Vec3::new(0.0, 0.0, 1.0),
                radius: 5.0,
                mesh: &near,
            },
        ];
        // Layer 2 picks the near mesh (candidate 1); layer 1 the far one.
        let hit = nearest_hit(ray, 2, &candidates).unwrap();
        assert_eq!((hit.node, hit.face), (1, 0));
        assert!((hit.distance - 1.0).abs() < 1e-6);
        let hit = nearest_hit(ray, 1, &candidates).unwrap();
        assert_eq!(hit.node, 0);
        assert!((hit.distance - 3.0).abs() < 1e-6);
        // The class and flag filters leave nothing.
        let only_skipped: Vec<Candidate<'_>> = candidates
            .iter()
            .map(|c| Candidate {
                class: 0x09,
                layers: u32::MAX,
                flags: 0,
                centre: c.centre,
                radius: c.radius,
                mesh: c.mesh,
            })
            .collect();
        assert!(nearest_hit(ray, u32::MAX, &only_skipped).is_none());
        // A bounding sphere off the ray rejects the mesh.
        let off = [Candidate {
            class: 0x89,
            layers: 2,
            flags: 0,
            centre: Vec3::new(10.0, 0.0, 1.0),
            radius: 1.0,
            mesh: &near,
        }];
        assert!(nearest_hit(ray, 2, &off).is_none());
    }

    #[test]
    fn the_kinesis_pick_needs_the_node_flag_and_the_height() {
        let triangle = [[
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ]];
        let mesh = Triangles(&triangle);
        let ray = ray(Vec3::ZERO, Vec3::Z);
        let mut node = Candidate {
            class: 0x89,
            layers: u32::MAX,
            flags: KINESIS_NODE,
            centre: Vec3::new(0.0, 0.0, 1.0),
            radius: 5.0,
            mesh: &mesh,
        };
        assert_eq!(kinesis_hit(ray, u32::MAX, 0.0, &[node]), Some((0, 1.0)));
        // Without the flag the node is not picked.
        node.flags = 0;
        assert!(kinesis_hit(ray, u32::MAX, 0.0, &[node]).is_none());
        // A layer the pick does not visit rejects it.
        node.flags = KINESIS_NODE;
        assert!(kinesis_hit(ray, 0, 0.0, &[node]).is_none());
        // The height lifts the box out of the ray.
        assert!(kinesis_hit(ray, u32::MAX, 10.0, &[node]).is_none());
    }

    #[test]
    fn a_ray_meets_a_sphere_by_its_bounding_test() {
        let r = ray(Vec3::ZERO, Vec3::Z);
        assert!(ray_meets_sphere(r, Vec3::new(0.0, 0.0, 1.0), 0.5));
        // Behind the ray's start: only within the radius of the start.
        assert!(!ray_meets_sphere(r, Vec3::new(0.0, 0.0, -1.0), 0.5));
        assert!(ray_meets_sphere(r, Vec3::new(0.0, 0.0, -0.2), 0.5));
        // Farther from the line than the radius.
        assert!(!ray_meets_sphere(r, Vec3::new(1.0, 0.0, 5.0), 0.5));
    }
}
