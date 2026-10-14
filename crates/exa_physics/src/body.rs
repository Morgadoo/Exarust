//! The physical body: each body part is a tetrahedron of four particles kept
//! rigid by its edges, carrying sphere colliders, and joined to its parent by
//! the skeleton's links (a point on each part, allowed to drift apart by a
//! slack). Particles move by Verlet integration at the original's 83 Hz,
//! then constraints and contacts are solved by moving positions.
//!
//! This stage simulates limp bodies (the dead and the knocked out); muscles
//! driving the living body towards its animation come next. Evidence:
//! `docs/formats/RSQ.md` (Physical body).

use bevy::math::{Affine3A, Mat3};
use bevy::prelude::*;

use crate::Physics;

/// Simulation rate of the body.
pub const BODY_HZ: f32 = 83.0;
const GRAVITY: f32 = -9.806_65;
/// Velocity kept per second, by speed: fast, middling, slow (characters).
const RETENTION: [f32; 3] = [0.2556, 0.15, 0.03];
/// Speeds (m/s) separating the retention bands.
const RETENTION_SPEEDS: [f32; 2] = [0.4, 0.2];
const LINK_ITERATIONS: usize = 20;
const CONTACT_ITERATIONS: usize = 8;
/// Ticks run per frame at most.
const MAX_TICKS: u32 = 4;
/// Relative inverse mass per part (bone − 1): pelvis, legs, torso, head,
/// arms, hands.
const INVERSE_MASS: [f32; 18] = [
    0.45, 0.7, 1.0, 2.8, 0.7, 1.0, 2.8, 0.45, 0.5, 1.0, 1.0, 1.2, 1.5, 1.0, 1.2, 1.5, 2.5, 2.5,
];
/// Friction coefficient of body parts against the world.
const FRICTION: f32 = 1.0;
const MIN_EXTENT: f32 = 0.06;

/// Regular tetrahedron, centred on the origin.
const TEMPLATE: [Vec3; 4] = [
    Vec3::new(std::f32::consts::SQRT_2, -1.0, 0.0),
    Vec3::new(-std::f32::consts::SQRT_2, -1.0, 0.0),
    Vec3::new(0.0, 1.0, std::f32::consts::SQRT_2),
    Vec3::new(0.0, 1.0, -std::f32::consts::SQRT_2),
];
/// The template, laid out over the part's box, is enlarged by this.
const TEMPLATE_SCALE: f32 = 1.25;

/// A rigid part of a simulated body.
#[derive(Debug, Clone)]
pub struct Part {
    pub bone: usize,
    pub entity: Entity,
    /// Entity whose world transform the part's local transform is relative
    /// to: its parent part's, or the body root's.
    pub parent: Option<usize>,
    /// Rest positions of the particles in the part's space.
    pub rest: [Vec3; 4],
    pub positions: [Vec3; 4],
    pub previous: [Vec3; 4],
    pub inverse_mass: f32,
    /// Collision spheres in the part's space: centre, radius.
    pub spheres: Vec<(Vec3, f32)>,
    rest_inverse: Mat3,
}

/// Barycentric weights of a point given in a part's space.
fn weights(part: &Part, local: Vec3) -> [f32; 4] {
    let w = part.rest_inverse * (local - part.rest[0]);
    [1.0 - w.x - w.y - w.z, w.x, w.y, w.z]
}

impl Part {
    fn new(
        bone: usize,
        entity: Entity,
        parent: Option<usize>,
        centre: Vec3,
        half: Vec3,
        world: Affine3A,
    ) -> Self {
        let largest = half.max_element();
        let half = half.max(Vec3::splat(MIN_EXTENT.max(0.2 * largest)));
        let rest = TEMPLATE.map(|t| centre + t * half * TEMPLATE_SCALE);
        let edges = Mat3::from_cols(rest[1] - rest[0], rest[2] - rest[0], rest[3] - rest[0]);
        let positions = rest.map(|r| world.transform_point3(r));
        // Spheres along the longest axis of the box.
        let axis = if half.x >= half.y && half.x >= half.z {
            Vec3::X
        } else if half.y >= half.z {
            Vec3::Y
        } else {
            Vec3::Z
        };
        let long = half.dot(axis);
        let thick = (half - axis * long).max_element().max(MIN_EXTENT * 0.5);
        let count = ((long / thick).round() as usize).clamp(1, 4);
        let span = (long - thick).max(0.0);
        let spheres = (0..count)
            .map(|k| {
                let t = if count == 1 {
                    0.0
                } else {
                    k as f32 / (count - 1) as f32 * 2.0 - 1.0
                };
                (centre + axis * span * t, thick)
            })
            .collect();
        Self {
            bone,
            entity,
            parent,
            rest,
            positions,
            previous: positions,
            inverse_mass: INVERSE_MASS
                .get(bone.wrapping_sub(1))
                .copied()
                .unwrap_or(1.0),
            spheres,
            rest_inverse: edges.inverse(),
        }
    }

    fn point(&self, w: &[f32; 4]) -> Vec3 {
        (0..4).map(|i| self.positions[i] * w[i]).sum()
    }

    /// Moves the part so that the point with weights `w` moves by `delta`.
    fn displace(&mut self, w: &[f32; 4], delta: Vec3) {
        let norm: f32 = w.iter().map(|x| x * x).sum();
        if norm <= 1e-9 {
            return;
        }
        for (p, wi) in self.positions.iter_mut().zip(w) {
            *p += delta * (wi / norm);
        }
    }

    /// The part's rigid world transform from its particles.
    pub fn transform(&self) -> Affine3A {
        let edges = Mat3::from_cols(
            self.positions[1] - self.positions[0],
            self.positions[2] - self.positions[0],
            self.positions[3] - self.positions[0],
        );
        let m = edges * self.rest_inverse;
        // Nearest rotation by Gram–Schmidt (the edges keep it near rigid).
        let x = m.x_axis.normalize_or(Vec3::X);
        let y = (m.y_axis - x * x.dot(m.y_axis)).normalize_or(Vec3::Y);
        let z = x.cross(y);
        let r = Mat3::from_cols(x, y, z);
        let rest_centre: Vec3 = self.rest.iter().copied().sum::<Vec3>() * 0.25;
        let centre: Vec3 = self.positions.iter().copied().sum::<Vec3>() * 0.25;
        Affine3A::from_mat3_translation(r, centre - r * rest_centre)
    }
}

/// Asks for the body under this entity (a body model root) to go limp:
/// its parts become a [`Ragdoll`] from their current pose.
#[derive(Component, Debug, Clone, Copy)]
pub struct GoLimp {
    /// Initial velocity of the whole body, m/s.
    pub velocity: Vec3,
    /// Frames to wait first (a body spawned this frame is not posed yet).
    pub delay: u32,
}

impl Default for GoLimp {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            delay: 3,
        }
    }
}

/// A link between a part and its parent part.
#[derive(Debug, Clone)]
pub struct Link {
    pub part: usize,
    pub parent: usize,
    on_part: [f32; 4],
    on_parent: [f32; 4],
    pub slack: f32,
}

/// A limp simulated body.
#[derive(Component, Debug, Clone)]
pub struct Ragdoll {
    pub parts: Vec<Part>,
    pub links: Vec<Link>,
    accumulator: f32,
}

/// What building a body needs for each part.
#[derive(Debug, Clone)]
pub struct PartSetup {
    pub bone: usize,
    pub entity: Entity,
    /// Index of the parent part, if any.
    pub parent: Option<usize>,
    /// The part's current world transform.
    pub world: Affine3A,
    /// Box of the part's mesh in its space.
    pub centre: Vec3,
    pub half: Vec3,
    /// Links to the parent: point on this part, point on the parent, slack.
    pub links: Vec<(Vec3, Vec3, f32)>,
}

impl Ragdoll {
    pub fn new(setup: &[PartSetup]) -> Self {
        let parts: Vec<Part> = setup
            .iter()
            .map(|s| Part::new(s.bone, s.entity, s.parent, s.centre, s.half, s.world))
            .collect();
        let mut links = Vec::new();
        for (i, s) in setup.iter().enumerate() {
            let Some(parent) = s.parent else { continue };
            for &(on_part, on_parent, slack) in &s.links {
                links.push(Link {
                    part: i,
                    parent,
                    on_part: weights(&parts[i], on_part),
                    on_parent: weights(&parts[parent], on_parent),
                    slack,
                });
            }
        }
        Self {
            parts,
            links,
            accumulator: 0.0,
        }
    }

    /// Gives every particle a velocity (m/s) by placing its previous
    /// position behind it.
    pub fn set_velocity(&mut self, velocity: Vec3) {
        let back = velocity / BODY_HZ;
        for p in &mut self.parts {
            for i in 0..4 {
                p.previous[i] = p.positions[i] - back;
            }
        }
    }

    /// One tick of the simulation. `contact` pushes a sphere (centre,
    /// radius) out of the world, returning the correction.
    pub fn tick(&mut self, contact: &dyn Fn(Vec3, f32) -> Option<Vec3>) {
        let dt = 1.0 / BODY_HZ;
        let gravity = Vec3::Y * GRAVITY * dt * dt;
        let retention = RETENTION.map(|r| r.powf(dt));
        for p in &mut self.parts {
            let moved = (0..4)
                .map(|i| (p.positions[i] - p.previous[i]).length())
                .fold(0.0, f32::max)
                / dt;
            let keep = if moved > RETENTION_SPEEDS[0] {
                retention[0]
            } else if moved > RETENTION_SPEEDS[1] {
                retention[1]
            } else {
                retention[2]
            };
            for i in 0..4 {
                let v = p.positions[i] - p.previous[i];
                p.previous[i] = p.positions[i];
                p.positions[i] += v * keep + gravity;
            }
        }
        for _ in 0..LINK_ITERATIONS {
            for p in &mut self.parts {
                keep_rigid(p);
            }
            for l in &self.links {
                let a = self.parts[l.part].point(&l.on_part);
                let b = self.parts[l.parent].point(&l.on_parent);
                let d = b - a;
                let len = d.length();
                if len <= l.slack || len <= 1e-6 {
                    continue;
                }
                let correction = d * ((len - l.slack) / len);
                let (ia, ib) = (
                    self.parts[l.part].inverse_mass,
                    self.parts[l.parent].inverse_mass,
                );
                let total = ia + ib;
                self.parts[l.part].displace(&l.on_part, correction * (ia / total));
                self.parts[l.parent].displace(&l.on_parent, -correction * (ib / total));
            }
        }
        for _ in 0..CONTACT_ITERATIONS {
            for p in &mut self.parts {
                let frame = p.transform();
                for k in 0..p.spheres.len() {
                    let (centre, radius) = p.spheres[k];
                    let world = frame.transform_point3(centre);
                    if let Some(push) = contact(world, radius) {
                        let w = weights(p, centre);
                        p.displace(&w, push);
                        // Friction: slip under the limit (larger on flat
                        // ground) stops; beyond it, the limit is taken off.
                        let n = push.normalize_or_zero();
                        let limit = FRICTION * (n.y * n.y * 0.99 + 0.01) * push.length();
                        for ((x, prev), wi) in p.positions.iter().zip(&mut p.previous).zip(w) {
                            let slide = *x - *prev;
                            let tangent = slide - n * slide.dot(n);
                            let len = tangent.length();
                            let cancel = if len <= limit {
                                tangent
                            } else {
                                tangent * (limit / len)
                            };
                            *prev += cancel * wi.max(0.0);
                        }
                    }
                }
            }
        }
    }
}

/// Restores a part's six edge lengths.
fn keep_rigid(p: &mut Part) {
    const EDGES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    for (a, b) in EDGES {
        let rest = (p.rest[b] - p.rest[a]).length();
        let d = p.positions[b] - p.positions[a];
        let len = d.length();
        if len <= 1e-6 {
            continue;
        }
        let c = d * ((len - rest) / len) * 0.5;
        p.positions[a] += c;
        p.positions[b] -= c;
    }
}

/// Steps every ragdoll at the body rate and writes the parts' transforms.
pub fn step_ragdolls(
    time: Res<Time>,
    physics: Res<Physics>,
    mut bodies: Query<(&mut Ragdoll, &GlobalTransform)>,
    mut transforms: Query<&mut Transform>,
) {
    let world = &physics.0;
    let contact = |centre: Vec3, radius: f32| -> Option<Vec3> {
        let (_, projection) = world.project_point(
            centre,
            radius * 2.0,
            false,
            rapier3d::prelude::QueryFilter::only_fixed(),
        )?;
        let to_centre = centre - projection.point;
        let distance = to_centre.length();
        if projection.is_inside {
            Some(-to_centre.normalize_or(Vec3::Y) * (distance + radius))
        } else if distance < radius {
            Some(to_centre.normalize_or(Vec3::Y) * (radius - distance))
        } else {
            None
        }
    };
    for (mut body, root) in &mut bodies {
        body.accumulator += time.delta_secs();
        let mut ticks = 0;
        while body.accumulator >= 1.0 / BODY_HZ && ticks < MAX_TICKS {
            body.tick(&contact);
            body.accumulator -= 1.0 / BODY_HZ;
            ticks += 1;
        }
        body.accumulator = body.accumulator.min(1.0 / BODY_HZ);
        // Local transforms from the parts' world transforms.
        let worlds: Vec<Affine3A> = body.parts.iter().map(Part::transform).collect();
        let root_inverse = root.affine().inverse();
        for (i, p) in body.parts.iter().enumerate() {
            let parent = match p.parent {
                Some(j) => worlds[j].inverse(),
                None => root_inverse,
            };
            if let Ok(mut t) = transforms.get_mut(p.entity) {
                *t = Transform::from_matrix((parent * worlds[i]).into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_parts(slack: f32) -> Ragdoll {
        let e = Entity::PLACEHOLDER;
        let at = |y: f32| Affine3A::from_translation(Vec3::new(0.0, y, 0.0));
        Ragdoll::new(&[
            PartSetup {
                bone: 1,
                entity: e,
                parent: None,
                world: at(1.0),
                centre: Vec3::ZERO,
                half: Vec3::splat(0.1),
                links: vec![],
            },
            PartSetup {
                bone: 2,
                entity: e,
                parent: Some(0),
                world: at(0.6),
                centre: Vec3::ZERO,
                half: Vec3::new(0.05, 0.2, 0.05),
                links: vec![(Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.0, -0.2, 0.0), slack)],
            },
        ])
    }

    #[test]
    fn parts_fall_and_rest_on_the_ground() {
        let mut body = two_parts(0.0);
        let ground = |c: Vec3, r: f32| (c.y < r).then(|| Vec3::Y * (r - c.y));
        for _ in 0..400 {
            body.tick(&ground);
        }
        for p in &body.parts {
            let lowest = p.positions.iter().map(|v| v.y).fold(f32::MAX, f32::min);
            assert!(lowest > -0.15, "fell through: {lowest}");
            let t = p.transform();
            assert!(
                t.translation.y < 0.3,
                "still in the air: {}",
                t.translation.y
            );
        }
    }

    #[test]
    fn links_hold_parts_together_and_edges_keep_shape() {
        let mut body = two_parts(0.0);
        let none = |_: Vec3, _: f32| None;
        for _ in 0..50 {
            body.tick(&none);
        }
        let l = &body.links[0];
        let a = body.parts[1].point(&l.on_part);
        let b = body.parts[0].point(&l.on_parent);
        assert!(a.distance(b) < 0.01, "link stretched {}", a.distance(b));
        let p = &body.parts[1];
        let rest = (p.rest[1] - p.rest[0]).length();
        let now = (p.positions[1] - p.positions[0]).length();
        assert!((rest - now).abs() < 0.01);
    }
}
