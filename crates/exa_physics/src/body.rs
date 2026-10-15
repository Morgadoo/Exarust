//! The physical body: each body part is a tetrahedron of four particles kept
//! rigid by its edges, carrying sphere colliders, and joined to its parent by
//! the skeleton's links (a point on each part, allowed to drift apart by a
//! slack). Particles move by Verlet integration at the original's 83 Hz,
//! then constraints and contacts are solved by moving positions.
//!
//! A limp body (the dead, the knocked out) is only that. A living body adds
//! muscles: a [`Drive`] pulls every particle towards where the animated pose
//! puts it, with damping, and compensates gravity by muscle tone. Blows shove
//! parts and daze the body, which weakens its muscles until it recovers.
//! Evidence: `docs/formats/RSQ.md` (Physical body).

use bevy::math::{Affine3A, Mat3};
use bevy::prelude::*;

use crate::Physics;
use crate::contact::{CONTACT_SLOP, Level, LevelContact, rub, sphere_hit, swept_hit};

/// Simulation rate of the body.
pub const BODY_HZ: f32 = 83.0;
const GRAVITY: f32 = -9.806_65;
/// Velocity kept per second, by speed: fast, middling, slow (characters).
const RETENTION: [f32; 3] = [0.2556, 0.15, 0.03];
/// Speeds (m/s) separating the retention bands.
const RETENTION_SPEEDS: [f32; 2] = [0.4, 0.2];
const LINK_ITERATIONS: usize = 20;
const FRICTION_PASSES: usize = 8;
/// Ticks run per frame at most.
const MAX_TICKS: u32 = 4;
/// The fastest a particle may move (m/s): well above any blow or fall.
const MAX_SPEED: f32 = 25.0;
/// A limp body whose particles all stay slower than this (m/s) ...
const SLEEP_SPEED: f32 = 0.01;
/// ... for this many ticks (2 s) falls asleep. Its contacts are with fixed
/// geometry only, so it stays exactly as it is until shoved.
const SLEEP_TICKS: u32 = 166;
/// Relative inverse mass per part (bone − 1): pelvis, legs, torso, head,
/// arms, hands.
const INVERSE_MASS: [f32; 18] = [
    0.45, 0.7, 1.0, 2.8, 0.7, 1.0, 2.8, 0.45, 0.5, 1.0, 1.0, 1.2, 1.5, 1.0, 1.2, 1.5, 2.5, 2.5,
];
/// Muscle tone of a fallen body.
const FALLEN_TONE: f32 = 0.5;
/// Share of the gap to the animated pose closed per tick at full strength.
const DRIVE: f32 = 0.084;
/// The weaker drive of the hands.
const HAND_DRIVE: f32 = 0.07;
/// The pelvis carries the body: its drive is this much stronger.
const SUPPORT: f32 = 2.0;
/// Energy regained per tick after a daze, and its floor.
const ENERGY_RECOVERY: f32 = 0.005;
const MIN_ENERGY: f32 = 0.05;
/// Most of its energy a single blow takes.
const MAX_DAZE: f32 = 0.9;
/// Friction of a limb against the level.
const LIMB_FRICTION: f32 = 2.5;
/// Friction of a foot (bones 4 and 7).
const FOOT_FRICTION: f32 = 0.5;
const MIN_EXTENT: f32 = 0.06;
/// Level contacts a part keeps between ticks.
const MAX_CONTACTS: usize = 4;
/// Squared travel (m²) of a sphere centre over a tick above which the sphere
/// is swept from where the last tick left it.
const SWEEP_TRAVEL2: f32 = 1e-5;
/// Pushes shorter than this (squared, m²) are ignored.
const MIN_PUSH2: f32 = 1e-9;
/// Depth (m) under a stored contact plane that is corrected.
const MIN_DEPTH: f32 = 1e-7;
/// Push length (m) at detection, and depth (m) in the plane pass, summed
/// before a part's edges are restored.
const PUSH_STRAIN: f32 = 0.001;
const DEPTH_STRAIN: f32 = 0.01;
/// A level push longer than this (m) is reported, not clamped.
const EXCESSIVE_PUSH: f32 = 1.6;

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
    /// Friction against the level.
    pub friction: f32,
    /// Particles at the start of the tick (where the last tick left them).
    start: [Vec3; 4],
    /// The part's transform where the last tick left it.
    settled: Affine3A,
    /// Level contacts found last tick, enforced in this tick's iterations.
    pub contacts: Vec<LevelContact>,
    /// Pushes and depths summed since the edges were last restored (or
    /// the tick began).
    strain: f32,
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
            friction: if matches!(bone, 4 | 7) {
                FOOT_FRICTION
            } else {
                LIMB_FRICTION
            },
            start: positions,
            settled: Affine3A::IDENTITY,
            contacts: Vec::new(),
            strain: 0.0,
        }
        .settle()
    }

    /// Records where the tick leaves the part.
    fn settle(mut self) -> Self {
        self.settled = self.transform();
        self
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

    /// Weights of the world point `x` over the current particles.
    fn weights_now(&self, x: Vec3) -> Option<[f32; 4]> {
        let edges = Mat3::from_cols(
            self.positions[1] - self.positions[0],
            self.positions[2] - self.positions[0],
            self.positions[3] - self.positions[0],
        );
        if edges.determinant().abs() < 1e-12 {
            return None;
        }
        let w = edges.inverse() * (x - self.positions[0]);
        Some([1.0 - w.x - w.y - w.z, w.x, w.y, w.z])
    }

    /// How far the point with weights `w` moved this tick.
    fn slip(&self, w: &[f32; 4]) -> Vec3 {
        (0..4)
            .map(|i| (self.positions[i] - self.start[i]) * w[i])
            .sum()
    }

    /// A level hit on sphere `centre` (part space): the closest point `q` on
    /// the triangle and the push `delta` that takes the sphere off it. The
    /// part is pushed at once and the contact kept for the next tick.
    fn hit(&mut self, centre: Vec3, q: Vec3, delta: Vec3) -> bool {
        let len2 = delta.length_squared();
        if !(len2 >= MIN_PUSH2 && len2.is_finite()) {
            return false;
        }
        if len2 > EXCESSIVE_PUSH * EXCESSIVE_PUSH {
            warn_once!("excessive level contact push: {:.2} m", len2.sqrt());
        }
        // The deepest point of the sphere, or its centre when that lies far
        // outside the tetrahedron.
        let w = self
            .weights_now(q - delta)
            .filter(|w| w.iter().all(|x| x.abs() <= 1.0))
            .unwrap_or_else(|| weights(self, centre));
        let normal = delta / len2.sqrt();
        self.offer(LevelContact {
            weights: w,
            point: q,
            normal,
            plane: normal.dot(q) - CONTACT_SLOP,
        });
        // Spread so that the heaviest particle takes the whole push.
        let clamped = w.map(|x| x.max(0.0));
        let top = clamped.iter().copied().fold(0.0, f32::max);
        if top > 0.0 {
            for (x, c) in self.positions.iter_mut().zip(clamped) {
                *x += delta * (c / top);
            }
        }
        self.strain += len2.sqrt();
        if self.strain > PUSH_STRAIN {
            keep_rigid(self);
        }
        true
    }

    /// Keeps a new contact, or, with four kept, the four spread widest over
    /// the part.
    fn offer(&mut self, contact: LevelContact) {
        if self.contacts.len() < MAX_CONTACTS {
            self.contacts.push(contact);
            return;
        }
        let mut all = [contact.weights; MAX_CONTACTS + 1];
        for (w, c) in all.iter_mut().zip(&self.contacts) {
            *w = c.weights;
        }
        let gap =
            |a: usize, b: usize| -> f32 { (0..4).map(|k| (all[a][k] - all[b][k]).powi(2)).sum() };
        let mut pair = (0, 1, f32::MAX);
        for a in 0..all.len() {
            for b in a + 1..all.len() {
                let d = gap(a, b);
                if d < pair.2 {
                    pair = (a, b, d);
                }
            }
        }
        // Of the closest pair, the one nearer to the rest goes.
        let nearest = |i: usize, partner: usize| {
            (0..all.len())
                .filter(|&j| j != i && j != partner)
                .map(|j| gap(i, j))
                .fold(f32::MAX, f32::min)
        };
        let (a, b, _) = pair;
        let out = if nearest(a, b) < nearest(b, a) { a } else { b };
        if out < MAX_CONTACTS {
            self.contacts.swap_remove(out);
            self.contacts.push(contact);
        }
    }

    /// Enforces the stored contacts once: a point under its plane is lifted
    /// onto it, and its slip this tick is cancelled below the friction
    /// limit and damped above it.
    fn hold_contacts(&mut self) {
        let mut depth = 0.0;
        for k in 0..self.contacts.len() {
            let c = self.contacts[k];
            let mut correction = rub(self.slip(&c.weights), c.normal, self.friction);
            let below = c.plane - c.normal.dot(self.point(&c.weights));
            if below > MIN_DEPTH {
                correction += c.normal * below;
                depth += below;
            }
            for (x, w) in self.positions.iter_mut().zip(c.weights) {
                *x += correction * w.max(0.0);
            }
            self.strain += depth;
            if self.strain > DEPTH_STRAIN {
                keep_rigid(self);
            }
        }
    }

    /// Friction only, from the contacts found this tick.
    fn rub_contacts(&mut self) {
        for k in 0..self.contacts.len() {
            let c = self.contacts[k];
            let correction = rub(self.slip(&c.weights), c.normal, self.friction);
            for (x, w) in self.positions.iter_mut().zip(c.weights) {
                *x += correction * w;
            }
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

/// On a living body model root that lost its balance: it lies on the ground
/// and is not steered until it has got up.
#[derive(Component, Debug, Clone, Default)]
pub struct Fallen {
    /// Seconds since the fall.
    pub time: f32,
    /// The get-up clip is playing.
    pub getting_up: bool,
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

/// Makes the body under this entity (a body model root) physical: its parts
/// become a [`Ragdoll`] with muscles following its animation.
#[derive(Component, Debug, Clone, Copy)]
pub struct Physical {
    /// Frames to wait first (a body spawned this frame is not posed yet).
    pub delay: u32,
}

impl Default for Physical {
    fn default() -> Self {
        Self { delay: 3 }
    }
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

/// A simulated body: limp, or driven by its muscles when it has a
/// [`Drive`].
#[derive(Component, Debug, Clone)]
pub struct Ragdoll {
    pub parts: Vec<Part>,
    pub links: Vec<Link>,
    pub drive: Option<Drive>,
    /// Ticks run in the last frame.
    pub ticks: u32,
    accumulator: f32,
    /// Ticks a limp body has lain still.
    still: u32,
    /// A limp body at rest is not simulated until something shoves it.
    pub asleep: bool,
}

/// Muscles of a living body: the animated pose it follows.
#[derive(Debug, Clone)]
pub struct Drive {
    /// Animated world transform of each part, as last aimed.
    aimed: Vec<Affine3A>,
    /// The same, advanced tick by tick towards the next aim.
    targets: Vec<Affine3A>,
    /// How far each target moves per tick.
    target_steps: Vec<Affine3A>,
    /// The body root's world transform when last aimed.
    root: Option<Affine3A>,
    /// Muscle tone, 0..=1: drive strength and gravity compensation.
    pub tone: f32,
    /// Daze: 1 when alert; blows lower it and it recovers over time. Muscle
    /// strength follows its fourth power.
    pub energy: f32,
    /// Standing: the controller carries the body and tone holds it up.
    /// A fallen body only holds its limbs relative to its pelvis.
    pub supported: bool,
}

impl Default for Drive {
    fn default() -> Self {
        Self {
            aimed: Vec::new(),
            targets: Vec::new(),
            target_steps: Vec::new(),
            root: None,
            tone: 1.0,
            energy: 1.0,
            supported: true,
        }
    }
}

/// A blow to a body part: a change of velocity (m/s) and how much it dazes.
#[derive(Message, Debug, Clone, Copy)]
pub struct Shove {
    /// The body model root.
    pub body: Entity,
    pub bone: usize,
    pub velocity: Vec3,
    pub daze: f32,
}

/// The systems stepping bodies; animation targets are set before them.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BodySystems;

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
            drive: None,
            ticks: 0,
            accumulator: 0.0,
            still: 0,
            asleep: false,
        }
    }

    /// Gives the body muscles following its animation.
    pub fn with_drive(mut self) -> Self {
        self.drive = Some(Drive::default());
        self.wake();
        self
    }

    /// Simulates the body again.
    pub fn wake(&mut self) {
        if self.asleep {
            // A sleeping body kept no contacts; it starts afresh where it lies.
            for p in &mut self.parts {
                p.contacts.clear();
                p.settled = p.transform();
            }
        }
        self.asleep = false;
        self.still = 0;
    }

    /// The fastest particle's speed in the last tick (m/s).
    fn fastest(&self) -> f32 {
        self.parts
            .iter()
            .flat_map(|p| (0..4).map(move |i| (p.positions[i] - p.previous[i]).length()))
            .fold(0.0, f32::max)
            * BODY_HZ
    }

    /// Sets the animated pose to follow: the root's world transform and each
    /// part's animated world transform, `frame` seconds after the last. The
    /// body is carried along with the root's own motion, so the muscles
    /// only work against the pose and outside forces.
    pub fn aim(&mut self, root: Affine3A, targets: Vec<Affine3A>, frame: f32) {
        let pelvis = self.parts.iter().position(|p| p.bone == 1);
        let pelvis_world = pelvis.map(|i| self.parts[i].transform());
        let Some(drive) = self.drive.as_mut() else {
            return;
        };
        // A fallen body follows the pose relative to its own pelvis.
        let targets = match (drive.supported, pelvis, pelvis_world) {
            (false, Some(i), Some(world)) => {
                // The pelvis's place and turn only: its particles deform a
                // little, and a scale carried into the targets would grow
                // the body away from the world's origin.
                let (_, turn, place) = world.to_scale_rotation_translation();
                let align = Affine3A::from_rotation_translation(turn, place) * targets[i].inverse();
                targets.iter().map(|t| align * *t).collect()
            }
            _ => targets,
        };
        if !drive.supported {
            drive.root = None;
        }
        if let Some(previous) = drive.root {
            let carry = root * previous.inverse();
            for p in &mut self.parts {
                for x in p.positions.iter_mut().chain(p.previous.iter_mut()) {
                    *x = carry.transform_point3(*x);
                }
                // Where the last tick left it moves too, or the carry would
                // read as a sweep; its contacts travel with it.
                p.settled = carry * p.settled;
                for c in &mut p.contacts {
                    c.point = carry.transform_point3(c.point);
                    c.normal = carry.transform_vector3(c.normal).normalize_or(c.normal);
                    c.plane = c.normal.dot(c.point) - CONTACT_SLOP;
                }
            }
            for t in &mut drive.aimed {
                *t = carry * *t;
            }
        }
        let ticks = (frame * BODY_HZ).max(1.0);
        drive.target_steps = if drive.aimed.len() == targets.len() {
            targets
                .iter()
                .zip(&drive.aimed)
                .map(|(now, before)| {
                    // In the part's own frame: a turn about the part, not
                    // about the world's origin.
                    let step = before.inverse() * *now;
                    // A share of the frame's motion, per tick.
                    let (scale, rotation, translation) = step.to_scale_rotation_translation();
                    // The short way round.
                    let rotation = if rotation.w < 0.0 {
                        -rotation
                    } else {
                        rotation
                    };
                    let (axis, angle) = rotation.to_axis_angle();
                    Affine3A::from_scale_rotation_translation(
                        // Each tick takes its share of the scale too.
                        scale.powf(1.0 / ticks),
                        Quat::from_axis_angle(axis, angle / ticks),
                        translation / ticks,
                    )
                })
                .collect()
        } else {
            vec![Affine3A::IDENTITY; targets.len()]
        };
        drive.targets.clone_from(&targets);
        drive.aimed = targets;
        drive.root = Some(root);
    }

    /// Loses the controller's support: the body falls with slack muscles.
    pub fn fall(&mut self) {
        if let Some(d) = self.drive.as_mut() {
            d.supported = false;
            d.tone = FALLEN_TONE;
            // The pose is measured from the pelvis from now on: a new start,
            // not motion.
            d.root = None;
            d.aimed.clear();
        }
    }

    /// Stands again on the controller, from wherever the body lies.
    pub fn stand(&mut self) {
        if let Some(d) = self.drive.as_mut() {
            d.supported = true;
            d.tone = 1.0;
            d.root = None;
            d.aimed.clear();
        }
    }

    /// The part of `bone`.
    pub fn part_of(&self, bone: usize) -> Option<&Part> {
        self.parts.iter().find(|p| p.bone == bone)
    }

    /// Centre of mass and its velocity (m/s).
    pub fn centre_of_mass(&self) -> (Vec3, Vec3) {
        let (mut mass, mut centre, mut velocity) = (0.0, Vec3::ZERO, Vec3::ZERO);
        for p in &self.parts {
            let m = 1.0 / p.inverse_mass;
            let here: Vec3 = p.positions.iter().copied().sum::<Vec3>() * 0.25;
            let before: Vec3 = p.previous.iter().copied().sum::<Vec3>() * 0.25;
            mass += m;
            centre += here * m;
            velocity += (here - before) * m;
        }
        if mass <= 0.0 {
            return (Vec3::ZERO, Vec3::ZERO);
        }
        (centre / mass, velocity / mass * BODY_HZ)
    }

    /// Applies a blow to the part of `bone`.
    pub fn shove(&mut self, bone: usize, velocity: Vec3, daze: f32) {
        self.wake();
        let step = velocity / BODY_HZ;
        if let Some(p) = self.parts.iter_mut().find(|p| p.bone == bone) {
            for prev in &mut p.previous {
                *prev -= step;
            }
        }
        if let Some(d) = self.drive.as_mut() {
            d.energy = (d.energy * (1.0 - daze.clamp(0.0, MAX_DAZE))).max(MIN_ENERGY);
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

    /// One tick of the simulation against the static `level`, in the
    /// original's order: integrate and drive; 20 iterations of links, each
    /// part's stored level contacts and its edges; level detection with an
    /// immediate push, keeping the contacts for the next tick; 8 friction
    /// passes on them.
    pub fn tick<L: Level + ?Sized>(&mut self, level: &L) {
        self.integrate();
        if let Some(drive) = self.drive.as_mut() {
            drive_parts(&mut self.parts, drive);
        }
        for _ in 0..LINK_ITERATIONS {
            self.iterate();
        }
        for p in &mut self.parts {
            p.contacts.clear();
        }
        self.detect(level);
        for _ in 0..FRICTION_PASSES {
            for p in &mut self.parts {
                if p.friction > 0.0 {
                    p.rub_contacts();
                }
            }
        }
        for p in &mut self.parts {
            p.settled = p.transform();
        }
    }

    fn integrate(&mut self) {
        let dt = 1.0 / BODY_HZ;
        // Muscle tone holds the body up against gravity.
        let support = self
            .drive
            .as_ref()
            .filter(|d| d.supported)
            .map_or(0.0, |d| d.tone * d.energy);
        let gravity = Vec3::Y * GRAVITY * dt * dt * (1.0 - support);
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
            p.start = p.positions;
            p.strain = 0.0;
            for i in 0..4 {
                // A particle never carries more than MAX_SPEED (Exarust's
                // guard; the original has none).
                let v = (p.positions[i] - p.previous[i]).clamp_length_max(MAX_SPEED / BODY_HZ);
                p.previous[i] = p.positions[i];
                p.positions[i] += v * keep + gravity;
            }
        }
    }

    /// One constraint iteration over the body tree. A living body is solved
    /// from its hands and feet in to its pelvis, a limp one from the pelvis
    /// out; each part takes its links to its parent, then its stored level
    /// contacts, then its edges.
    fn iterate(&mut self) {
        let n = self.parts.len();
        let living = self.drive.is_some();
        for k in 0..n {
            let i = if living { n - 1 - k } else { k };
            for l in self.links.iter().filter(|l| l.part == i) {
                let a = self.parts[l.part].point(&l.on_part);
                let b = self.parts[l.parent].point(&l.on_parent);
                let d = b - a;
                let len = d.length();
                if len <= l.slack || len <= 1e-6 {
                    continue;
                }
                let correction = d * ((len - l.slack) / len);
                // Each point's share by its generalised inverse mass (the
                // part's, times the squared weights of the point): the two
                // parts' momenta change by equal and opposite amounts, even
                // when a point lies outside its tetrahedron.
                let ia =
                    self.parts[l.part].inverse_mass * l.on_part.iter().map(|w| w * w).sum::<f32>();
                let ib = self.parts[l.parent].inverse_mass
                    * l.on_parent.iter().map(|w| w * w).sum::<f32>();
                let total = ia + ib;
                if total <= 0.0 {
                    continue;
                }
                self.parts[l.part].displace(&l.on_part, correction * (ia / total));
                self.parts[l.parent].displace(&l.on_parent, -correction * (ib / total));
            }
            let p = &mut self.parts[i];
            p.hold_contacts();
            keep_rigid(p);
        }
    }

    /// Tests every sphere against the level: swept from where the last tick
    /// left it when it moved far enough, else where it is.
    fn detect<L: Level + ?Sized>(&mut self, level: &L) {
        let mut pieces = Vec::new();
        let mut triangles = Vec::new();
        for p in &mut self.parts {
            let before = p.settled;
            let (mut lo, mut hi) = (Vec3::MAX, Vec3::MIN);
            let now = p.transform();
            for &(c, r) in &p.spheres {
                for x in [before.transform_point3(c), now.transform_point3(c)] {
                    lo = lo.min(x - Vec3::splat(2.0 * r));
                    hi = hi.max(x + Vec3::splat(2.0 * r));
                }
            }
            pieces.clear();
            level.pieces(lo, hi, &mut pieces);
            for &piece in &pieces {
                for k in 0..p.spheres.len() {
                    let (centre, r) = p.spheres[k];
                    let start = before.transform_point3(centre);
                    let mut end = p.transform().transform_point3(centre);
                    let swept = start.distance_squared(end) > SWEEP_TRAVEL2;
                    // Room for the pushes of earlier triangles.
                    let reach = Vec3::splat(2.0 * r);
                    let (lo, hi) = if swept {
                        (start.min(end) - reach, start.max(end) + reach)
                    } else {
                        (end - reach, end + reach)
                    };
                    triangles.clear();
                    level.triangles(piece, lo, hi, &mut triangles);
                    for t in &triangles {
                        let hit = if swept {
                            swept_hit(t, start, end, r)
                        } else {
                            sphere_hit(t, end, r)
                        };
                        if let Some((q, delta)) = hit
                            && p.hit(centre, q, delta)
                        {
                            end = p.transform().transform_point3(centre);
                        }
                    }
                }
            }
        }
    }
}

/// The rigid motion of a body's particles: centre of mass, velocity and
/// spin (per tick), and the inverse of the inertia about the centre.
struct Bulk {
    centre: Vec3,
    mass: f32,
    velocity: Vec3,
    spin: Vec3,
    inverse_inertia: Mat3,
}

impl Bulk {
    fn of(parts: &[Part]) -> Option<Self> {
        let mass: f32 = parts.iter().map(|p| 1.0 / p.inverse_mass).sum();
        if mass <= 0.0 {
            return None;
        }
        // Each particle carries a quarter of its part's mass.
        let (mut centre, mut velocity) = (Vec3::ZERO, Vec3::ZERO);
        for p in parts {
            let m = 0.25 / p.inverse_mass;
            for k in 0..4 {
                centre += p.positions[k] * m;
                velocity += (p.positions[k] - p.previous[k]) * m;
            }
        }
        let (centre, velocity) = (centre / mass, velocity / mass);
        let (mut inertia, mut momentum) = (Mat3::ZERO, Vec3::ZERO);
        for p in parts {
            let m = 0.25 / p.inverse_mass;
            for k in 0..4 {
                let r = p.positions[k] - centre;
                let outer = Mat3::from_cols(r * r.x, r * r.y, r * r.z);
                inertia += (Mat3::IDENTITY * r.length_squared() - outer) * m;
                momentum += r.cross(p.positions[k] - p.previous[k]) * m;
            }
        }
        let inverse_inertia = inertia.inverse();
        if !inverse_inertia.is_finite() {
            return None;
        }
        Some(Self {
            centre,
            mass,
            velocity,
            spin: inverse_inertia * momentum,
            inverse_inertia,
        })
    }

    /// Velocity (per tick) of the body's rigid motion at `x`.
    fn velocity_at(&self, x: Vec3) -> Vec3 {
        self.velocity + self.spin.cross(x - self.centre)
    }

    /// The rigid motion (a shift and a small turn about the centre) that the
    /// changes `moves` of the particles at `parts` add up to.
    fn net(&self, parts: &[Part], moves: impl Fn(usize, usize) -> Vec3) -> (Vec3, Vec3) {
        let (mut shift, mut turn) = (Vec3::ZERO, Vec3::ZERO);
        for (i, p) in parts.iter().enumerate() {
            let m = 0.25 / p.inverse_mass;
            for k in 0..4 {
                let d = moves(i, k);
                shift += d * m;
                turn += (p.positions[k] - self.centre).cross(d) * m;
            }
        }
        (shift / self.mass, self.inverse_inertia * turn)
    }
}

/// Pulls every particle towards its place in the animated pose, damping its
/// velocity relative to the pose's own motion (a PD muscle per particle).
fn drive_parts(parts: &mut [Part], drive: &mut Drive) {
    if drive.targets.len() != parts.len() {
        return;
    }
    let strength = drive.tone * drive.tone * drive.energy.powi(4);
    // A standing body's targets are where its controller carries it. A
    // fallen body's follow its own pelvis, so they are only its pose: its
    // muscles work inside the body. Its limbs are damped relative to the
    // body's rigid motion, its bulk velocity and its spin (feeding the
    // targets' motion back flings the body, damping against the world holds
    // its fall back), and the net shift and turn of the whole body that the
    // pulls and the damping add up to are taken back out: a pose measured
    // from the pelvis, pulled on the limbs alone, turns the whole body, and
    // a turning body rolls and crawls over the floor.
    let bulk = if drive.supported {
        None
    } else {
        Bulk::of(parts)
    };
    let mut pulls = vec![[Vec3::ZERO; 4]; parts.len()];
    let mut brakes = vec![[Vec3::ZERO; 4]; parts.len()];
    for (i, p) in parts.iter().enumerate() {
        let base = match p.bone {
            1 if drive.supported => DRIVE * SUPPORT,
            // A fallen body's pelvis is where the pose is measured from.
            1 => 0.0,
            17 | 18 => HAND_DRIVE,
            _ => DRIVE,
        };
        let gain = (base * strength).min(1.0);
        let damping = (2.0 * gain.sqrt()).min(1.0);
        // The target advances through the tick.
        let before = drive.targets[i];
        drive.targets[i] *= drive.target_steps[i];
        let target = drive.targets[i];
        for k in 0..4 {
            let goal = target.transform_point3(p.rest[k]);
            let goal_velocity = match &bulk {
                Some(b) => b.velocity_at(p.positions[k]),
                None => goal - before.transform_point3(p.rest[k]),
            };
            let velocity = p.positions[k] - p.previous[k];
            pulls[i][k] = (goal - p.positions[k]) * gain;
            brakes[i][k] = (velocity - goal_velocity) * damping;
        }
    }
    let ((shift, turn), (slowed, unturned)) = match &bulk {
        Some(b) => (
            b.net(parts, |i, k| pulls[i][k]),
            b.net(parts, |i, k| brakes[i][k]),
        ),
        None => Default::default(),
    };
    let centre = bulk.as_ref().map_or(Vec3::ZERO, |b| b.centre);
    for (i, p) in parts.iter_mut().enumerate() {
        for k in 0..4 {
            let r = p.positions[k] - centre;
            p.positions[k] += pulls[i][k] - (shift + turn.cross(r));
            p.previous[k] += brakes[i][k] - (slowed + unturned.cross(r));
        }
    }
    drive.energy = (drive.energy + ENERGY_RECOVERY).min(1.0);
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
    // As in the original, restoring the edges starts the strain afresh.
    p.strain = 0.0;
}

/// World transform of `entity` from the local transforms up its ancestry
/// (current this frame, unlike the propagated global transforms).
pub fn world_of(
    entity: Entity,
    parents: &Query<&ChildOf>,
    transforms: &Query<&mut Transform>,
) -> Affine3A {
    let mut world = Affine3A::IDENTITY;
    let mut node = Some(entity);
    while let Some(e) = node {
        if let Ok(t) = transforms.get(e) {
            world = t.compute_affine() * world;
        }
        node = parents.get(e).ok().map(ChildOf::parent);
    }
    world
}

/// Steps every ragdoll at the body rate and writes the parts' transforms.
pub fn step_ragdolls(
    time: Res<Time>,
    physics: Res<Physics>,
    mut shoves: MessageReader<Shove>,
    mut bodies: Query<(Entity, &mut Ragdoll)>,
    parents: Query<&ChildOf>,
    mut transforms: Query<&mut Transform>,
) {
    for s in shoves.read() {
        if let Ok((_, mut body)) = bodies.get_mut(s.body) {
            body.shove(s.bone, s.velocity, s.daze);
        }
    }
    let level = &physics.0;
    for (_, mut body) in &mut bodies {
        if body.asleep {
            body.accumulator = 0.0;
            body.ticks = 0;
            continue;
        }
        body.accumulator += time.delta_secs();
        let mut ticks = 0;
        while body.accumulator >= 1.0 / BODY_HZ && ticks < MAX_TICKS {
            body.tick(level);
            body.accumulator -= 1.0 / BODY_HZ;
            ticks += 1;
        }
        body.accumulator = body.accumulator.min(1.0 / BODY_HZ);
        body.ticks = ticks;
        if body.drive.is_none() && ticks > 0 {
            body.still = if body.fastest() < SLEEP_SPEED {
                body.still + ticks
            } else {
                0
            };
            body.asleep = body.still >= SLEEP_TICKS;
        }
        // Local transforms from the parts' world transforms.
        let worlds: Vec<Affine3A> = body.parts.iter().map(Part::transform).collect();
        for (i, p) in body.parts.iter().enumerate() {
            let parent = match p.parent {
                Some(j) => worlds[j].inverse(),
                None => match parents.get(p.entity) {
                    Ok(up) => world_of(up.parent(), &parents, &transforms).inverse(),
                    Err(_) => Affine3A::IDENTITY,
                },
            };
            if let Ok(mut t) = transforms.get_mut(p.entity) {
                *t = Transform::from_matrix((parent * worlds[i]).into());
            }
        }
    }
}

#[cfg(test)]
#[path = "body_floor_test.rs"]
mod floor_test;

#[cfg(test)]
#[path = "body_contact_test.rs"]
mod contact_test;

#[cfg(test)]
mod tests {
    use super::*;

    /// A square floor at height `y`, facing up.
    pub(super) fn floor_quad(y: f32, half: f32) -> Vec<[Vec3; 3]> {
        let at = |x: f32, z: f32| Vec3::new(x * half, y, z * half);
        vec![
            [at(-1.0, -1.0), at(-1.0, 1.0), at(1.0, 1.0)],
            [at(-1.0, -1.0), at(1.0, 1.0), at(1.0, -1.0)],
        ]
    }

    const NONE: &[[Vec3; 3]] = &[];

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
    fn resting_limp_bodies_sleep_until_shoved() {
        let mut body = two_parts(0.0);
        body.still = SLEEP_TICKS;
        body.asleep = true;
        body.shove(1, Vec3::X, 0.0);
        assert!(!body.asleep);
        assert_eq!(body.still, 0);
    }

    #[test]
    fn parts_fall_and_rest_on_the_ground() {
        let mut body = two_parts(0.0);
        let ground = floor_quad(0.0, 5.0);
        for _ in 0..400 {
            body.tick(ground.as_slice());
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
        for _ in 0..50 {
            body.tick(NONE);
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

    /// A link whose points lie unevenly inside their tetrahedra (the
    /// parent's outside it) moves the two parts by equal and opposite
    /// momenta, so the constraints alone never set a body moving.
    #[test]
    fn links_keep_the_bodys_momentum() {
        let mut body = two_parts(0.0);
        let l = &body.links[0];
        let spread = |w: &[f32; 4]| w.iter().map(|x| x * x).sum::<f32>();
        assert!((spread(&l.on_part) - spread(&l.on_parent)).abs() > 0.2);
        // Pull the lower part off its joint, twisted.
        let twist = Affine3A::from_rotation_translation(
            Quat::from_rotation_z(0.4),
            Vec3::new(0.15, -0.05, 0.1),
        );
        for x in &mut body.parts[1].positions {
            *x = twist.transform_point3(*x);
        }
        let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
        for _ in 0..LINK_ITERATIONS {
            body.iterate();
        }
        // Each particle carries its part's mass.
        let momentum: Vec3 = body
            .parts
            .iter()
            .zip(&before)
            .flat_map(|(p, b)| (0..4).map(move |i| (p.positions[i] - b[i]) / p.inverse_mass))
            .sum();
        assert!(
            momentum.length() < 1e-4,
            "constraints moved the body by {momentum}"
        );
    }

    #[test]
    fn driven_body_follows_its_pose() {
        let mut body = two_parts(0.0).with_drive();
        let targets = |z: f32| {
            let r = Quat::from_rotation_y(z * 10.0);
            vec![
                Affine3A::from_rotation_translation(r, Vec3::new(0.0, 1.0, z)),
                Affine3A::from_rotation_translation(r, Vec3::new(0.0, 0.6, z)),
            ]
        };
        for f in 0..300 {
            let z = (f % 40) as f32 * 0.02;
            body.aim(Affine3A::IDENTITY, targets(z), 1.0 / 60.0);
            for _ in 0..if f % 3 == 0 { 2 } else { 1 } {
                body.tick(NONE);
            }
            let c = body.parts[0].transform().translation;
            assert!(c.is_finite() && c.length() < 3.0, "frame {f}: {c}");
        }
    }

    #[test]
    fn a_shove_staggers_and_the_body_recovers() {
        let mut body = two_parts(0.0).with_drive();
        let pose = vec![
            Affine3A::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            Affine3A::from_translation(Vec3::new(0.0, 0.6, 0.0)),
        ];
        let step = |body: &mut Ragdoll, ticks: usize| {
            for _ in 0..ticks {
                body.aim(Affine3A::IDENTITY, pose.clone(), 1.0 / BODY_HZ);
                body.tick(NONE);
            }
            body.parts[0].transform().translation.x
        };
        step(&mut body, 20);
        body.shove(1, Vec3::X * 3.0, 0.7);
        let pushed = step(&mut body, 10);
        assert!(pushed > 0.05, "barely moved: {pushed}");
        let back = step(&mut body, 400);
        assert!(back.abs() < 0.02, "did not recover: {back}");
        assert_eq!(body.drive.as_ref().unwrap().energy, 1.0);
    }
}
