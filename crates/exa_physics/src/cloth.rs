//! A cloth step: Verlet particles, a wind nudge, pins and edge constraints.
//!
//! Each free particle keeps `0.98977 × 0.5^(1/83)` of its velocity, dragged
//! 5% toward last tick's mean, then gains gravity. Gravity is the body's
//! acceleration, in centimetres per tick. Wind above 0.001 moves a particle
//! by `direction · (noise + 0.25) · wind`. A pin moves toward its rest point
//! by its weight. An edge corrects by
//! `(L² / (|d|² + L²) − 0.5) · 0.8774` without a square root. One pass, which
//! is the shipped cloth's mode. A sphere pushes a vertex out to its radius
//! plus 2 cm, and a level plane does the same along its normal. The three
//! body frictions are 0.002, 0.01 and 0.015; which body uses which was not
//! read. Sleep counts down while the mean squared speed is under
//! √(vertices × 10⁻⁶), and never below the owner's count. Rustle plays when
//! the squared speed sum exceeds 400. The mesh vertices are the particle
//! positions.
//!
//! Evidence: the cloth step notes (retention, drag, wind, pins, edges, sleep,
//! rustle, sphere and plane pushes). Swept triangle tests are not in this step.

use bevy::math::Vec3;

use crate::body::BODY_HZ;

/// How much of the velocity a particle keeps. Fast retention times the
/// cloth's per-second retention, taken to one tick.
pub fn retention() -> f32 {
    0.98977 * 0.5_f32.powf(1.0 / BODY_HZ)
}

/// Drag toward the cloth's mean velocity.
pub const MEAN_DRAG: f32 = 0.05;
/// Edge correction stiffness.
pub const EDGE_STIFFNESS: f32 = 0.8774;
/// Wind below this does not move the cloth.
pub const WIND_FLOOR: f32 = 0.001;
/// Collision padding, centimetres.
pub const PADDING_CM: f32 = 2.0;
/// Friction on a cloth vertex, the three body factors. Which body uses which
/// was not read.
pub const FRICTION: [f32; 3] = [0.002, 0.01, 0.015];
/// Sleep countdown does not rise above this.
pub const SLEEP_CAP: i32 = 32;
/// Rustle starts when the squared sum of the velocities exceeds this.
pub const RUSTLE_SPEED_SQ: f32 = 400.0;
/// The body's gravitational acceleration, metres per second squared.
const GRAVITY_MPS2: f32 = -9.806_65;

/// Gravity for one tick, in centimetres, scaled by the cloth's gravity scale.
pub fn gravity_cm(scale: f32) -> Vec3 {
    Vec3::new(0.0, GRAVITY_MPS2 * 100.0 * scale / (BODY_HZ * BODY_HZ), 0.0)
}

/// The built-in wind direction, `normalise(0.6, 0, 0.3)`.
pub fn wind_direction() -> Vec3 {
    Vec3::new(0.6, 0.0, 0.3).normalize()
}

/// One cloth particle. Positions are centimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    pub position: Vec3,
    pub previous: Vec3,
}

/// An edge and its rest length, in centimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub length: f32,
}

/// A vertex pinned toward a rest point by `weight` (0 stays, 1 sits on it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pin {
    pub index: usize,
    pub rest: Vec3,
    pub weight: f32,
}

/// A body sphere the cloth must stay outside, in centimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodySphere {
    pub center: Vec3,
    pub radius: f32,
    pub friction: f32,
}

/// A level plane. The normal points out of the solid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelPlane {
    pub point: Vec3,
    pub normal: Vec3,
}

/// The cloth a step advances.
#[derive(Debug, Clone, PartialEq)]
pub struct Cloth {
    pub particles: Vec<Particle>,
    pub edges: Vec<Edge>,
    pub pins: Vec<Pin>,
    /// Mean velocity from the previous tick, centimetres per tick.
    pub mean_velocity: Vec3,
    pub wind: f32,
    pub wind_direction: Vec3,
    /// One noise sample per particle, already in −1 to 1.
    pub noise: Vec<f32>,
    /// Ticks of motion still owed. Zero is asleep.
    pub sleep: i32,
    /// The owner's countdown. This cloth never sleeps deeper than that.
    pub owner_sleep: i32,
    pub spheres: Vec<BodySphere>,
    pub planes: Vec<LevelPlane>,
    pub rustle: bool,
    pub rustle_volume: f32,
    pub rustle_pitch: f32,
    /// The rustle envelope, which decays by 0.9 each tick.
    pub rustle_tail: f32,
}

impl Cloth {
    pub fn new(particles: Vec<Particle>) -> Self {
        Self {
            particles,
            edges: Vec::new(),
            pins: Vec::new(),
            mean_velocity: Vec3::ZERO,
            wind: 0.0,
            wind_direction: wind_direction(),
            noise: Vec::new(),
            sleep: SLEEP_CAP,
            owner_sleep: 0,
            spheres: Vec::new(),
            planes: Vec::new(),
            rustle: false,
            rustle_volume: 0.0,
            rustle_pitch: 0.0,
            rustle_tail: 0.0,
        }
    }

    /// Vertex positions the mesh draws, in centimetres.
    pub fn vertices(&self) -> Vec<Vec3> {
        self.particles
            .iter()
            .map(|particle| particle.position)
            .collect()
    }
}

/// Advance one tick. Returns without changing non-finite input.
pub fn step(cloth: &mut Cloth, gravity: Vec3) {
    if !gravity.is_finite() || !cloth.mean_velocity.is_finite() {
        return;
    }
    let count = cloth.particles.len();
    if count == 0 {
        return;
    }
    let mut sum = Vec3::ZERO;
    let mut speed_sq = 0.0;
    let awake = cloth.sleep > 0;
    if awake {
        let mean = cloth.mean_velocity;
        let keep = retention();
        for (index, particle) in cloth.particles.iter_mut().enumerate() {
            if !particle.position.is_finite() || !particle.previous.is_finite() {
                continue;
            }
            let velocity = particle.position - particle.previous;
            let retained = (velocity + (mean - velocity) * MEAN_DRAG) * keep;
            let mut next = particle.position + retained + gravity;
            if cloth.wind > WIND_FLOOR {
                let noise = cloth.noise.get(index).copied().unwrap_or(0.0);
                if cloth.wind_direction.is_finite() && noise.is_finite() {
                    next += cloth.wind_direction * (noise + 0.25) * cloth.wind;
                }
            }
            let step = next - particle.position;
            sum += step;
            speed_sq += step.length_squared();
            particle.previous = particle.position;
            particle.position = next;
        }
    }
    cloth.mean_velocity = sum / count as f32;
    settle_sleep(cloth, speed_sq / count as f32);
    settle_rustle(cloth, sum);
    if awake {
        apply_constraints(cloth);
    }
    let mut hit = false;
    for index in 0..cloth.spheres.len() {
        let sphere = cloth.spheres[index];
        for particle in &mut cloth.particles {
            hit |= push_sphere(particle, sphere);
        }
    }
    for index in 0..cloth.planes.len() {
        let plane = cloth.planes[index];
        for particle in &mut cloth.particles {
            hit |= push_plane(particle, plane);
        }
    }
    if hit && cloth.sleep == 0 {
        cloth.sleep = 1.max(cloth.owner_sleep);
    } else if cloth.sleep == 0 {
        for particle in &mut cloth.particles {
            particle.previous = particle.position;
        }
    }
}

fn apply_constraints(cloth: &mut Cloth) {
    for index in 0..cloth.pins.len() {
        let pin = cloth.pins[index];
        let Some(particle) = cloth.particles.get_mut(pin.index) else {
            continue;
        };
        if !pin.rest.is_finite() || !pin.weight.is_finite() {
            continue;
        }
        let weight = pin.weight.clamp(0.0, 1.0);
        particle.position += (pin.rest - particle.position) * weight;
    }
    for index in 0..cloth.edges.len() {
        let edge = cloth.edges[index];
        correct_edge(&mut cloth.particles, edge);
    }
}

fn settle_sleep(cloth: &mut Cloth, mean_sq: f32) {
    let threshold = (cloth.particles.len() as f32 * 1e-6).sqrt();
    if mean_sq < threshold {
        cloth.sleep -= 1;
    } else if mean_sq > (2.0 * threshold).powi(2) {
        cloth.sleep += 1;
    }
    cloth.sleep = cloth.sleep.clamp(cloth.owner_sleep, SLEEP_CAP);
}

fn settle_rustle(cloth: &mut Cloth, sum: Vec3) {
    let speed_sq = sum.length_squared();
    if speed_sq > RUSTLE_SPEED_SQ {
        let speed = speed_sq.sqrt();
        cloth.rustle_volume = 0.5 * cloth.rustle_volume + 0.0001 * speed;
        cloth.rustle_pitch = (0.5 * cloth.rustle_volume + 0.5).min(2.0);
        cloth.rustle = true;
        cloth.rustle_tail = cloth.rustle_volume;
    } else {
        cloth.rustle = false;
        cloth.rustle_volume = 0.0;
        cloth.rustle_pitch = 0.0;
        cloth.rustle_tail *= 0.9;
    }
}

fn push_sphere(particle: &mut Particle, sphere: BodySphere) -> bool {
    if !sphere.center.is_finite() || !sphere.radius.is_finite() || sphere.radius < 0.0 {
        return false;
    }
    let reach = sphere.radius + PADDING_CM;
    let mut offset = particle.position - sphere.center;
    if offset.length_squared() <= f32::EPSILON {
        offset = Vec3::Y;
    }
    let distance = offset.length();
    if distance >= reach {
        return false;
    }
    let normal = offset / distance;
    let surface = sphere.center + normal * reach;
    let motion = surface - particle.previous;
    let along = normal * motion.dot(normal);
    let tangent = motion - along;
    let friction = sphere.friction.clamp(0.0, 1.0);
    particle.position = particle.previous + along + tangent * (1.0 - friction);
    true
}

fn push_plane(particle: &mut Particle, plane: LevelPlane) -> bool {
    let normal = plane.normal.normalize_or_zero();
    if normal == Vec3::ZERO || !plane.point.is_finite() {
        return false;
    }
    let distance = (particle.position - plane.point).dot(normal);
    if distance >= PADDING_CM {
        return false;
    }
    particle.position += normal * (PADDING_CM - distance);
    true
}

/// Reflect a noise sample into −1 to 1 after one random-walk step.
pub fn walk_noise(value: f32, random: f32, wind: f32) -> f32 {
    if !value.is_finite() || !random.is_finite() || !wind.is_finite() {
        return 0.0;
    }
    let mut sample = value + (random - 0.5) * 0.8 * wind;
    for _ in 0..8 {
        if sample > 1.0 {
            sample = 2.0 - sample;
        } else if sample < -1.0 {
            sample = -2.0 - sample;
        } else {
            break;
        }
    }
    sample.clamp(-1.0, 1.0)
}

fn correct_edge(particles: &mut [Particle], edge: Edge) {
    if edge.a == edge.b || edge.a >= particles.len() || edge.b >= particles.len() {
        return;
    }
    if !edge.length.is_finite() || edge.length < 0.0 {
        return;
    }
    let delta = particles[edge.a].position - particles[edge.b].position;
    let distance_sq = delta.length_squared();
    let length_sq = edge.length * edge.length;
    let denom = distance_sq + length_sq;
    if denom <= f32::EPSILON {
        return;
    }
    let scale = (length_sq / denom - 0.5) * EDGE_STIFFNESS;
    let shift = delta * scale;
    particles[edge.a].position += shift;
    particles[edge.b].position -= shift;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn still(at: Vec3) -> Particle {
        Particle {
            position: at,
            previous: at,
        }
    }

    #[test]
    fn a_cloth_vertex_falls_and_an_edge_pulls() {
        let g = gravity_cm(1.0);
        assert!(g.y < 0.0);
        let mut cloth = Cloth::new(vec![still(Vec3::new(0.0, 100.0, 0.0))]);
        step(&mut cloth, g);
        assert!((cloth.particles[0].position.y - (100.0 + g.y)).abs() < 1e-4);
        assert!((cloth.particles[0].previous.y - 100.0).abs() < 1e-6);

        let mut cloth = Cloth::new(vec![still(Vec3::ZERO), still(Vec3::new(20.0, 0.0, 0.0))]);
        cloth.edges.push(Edge {
            a: 0,
            b: 1,
            length: 20.0,
        });
        step(&mut cloth, Vec3::ZERO);
        assert!(cloth.particles[0].position.abs_diff_eq(Vec3::ZERO, 1e-4));
        assert!(
            cloth.particles[1]
                .position
                .abs_diff_eq(Vec3::new(20.0, 0.0, 0.0), 1e-4)
        );

        let mut cloth = Cloth::new(vec![still(Vec3::ZERO), still(Vec3::new(20.0, 0.0, 0.0))]);
        cloth.edges.push(Edge {
            a: 0,
            b: 1,
            length: 10.0,
        });
        step(&mut cloth, Vec3::ZERO);
        let gap = cloth.particles[1].position - cloth.particles[0].position;
        assert!(gap.x > 9.0 && gap.x < 20.0, "{gap}");
        assert!(cloth.particles[0].position.x > 0.0);

        let mut cloth = Cloth::new(vec![still(Vec3::ZERO)]);
        cloth.pins.push(Pin {
            index: 0,
            rest: Vec3::new(5.0, 0.0, 0.0),
            weight: 1.0,
        });
        step(&mut cloth, Vec3::ZERO);
        assert!(
            cloth.particles[0]
                .position
                .abs_diff_eq(Vec3::new(5.0, 0.0, 0.0), 1e-4)
        );

        let mut cloth = Cloth::new(vec![still(Vec3::ZERO)]);
        cloth.wind = 1.0;
        cloth.wind_direction = Vec3::X;
        cloth.noise = vec![0.0];
        step(&mut cloth, Vec3::ZERO);
        assert!((cloth.particles[0].position.x - 0.25).abs() < 1e-4);
        let mut calm = Cloth::new(vec![still(Vec3::ZERO)]);
        calm.wind = WIND_FLOOR;
        calm.wind_direction = Vec3::X;
        calm.noise = vec![0.0];
        step(&mut calm, Vec3::ZERO);
        assert!(calm.particles[0].position.abs_diff_eq(Vec3::ZERO, 1e-4));

        assert!((walk_noise(0.9, 1.0, 1.0) - 0.7).abs() < 1e-4);
        assert!(walk_noise(0.0, 0.5, 1.0).abs() < 1e-6);
        let dir = wind_direction();
        assert!((dir.length() - 1.0).abs() < 1e-5);
        assert!(dir.x > dir.z && dir.z > 0.0);
    }

    #[test]
    fn a_cloth_sleeps_rustles_and_clears_a_sphere_and_a_plane() {
        let mut cloth = Cloth::new(vec![still(Vec3::ZERO)]);
        cloth.sleep = 4;
        cloth.owner_sleep = 4;
        step(&mut cloth, Vec3::ZERO);
        assert_eq!(cloth.sleep, 4);
        cloth.owner_sleep = 0;
        cloth.sleep = SLEEP_CAP;
        for _ in 0..SLEEP_CAP {
            step(&mut cloth, Vec3::ZERO);
        }
        assert_eq!(cloth.sleep, 0);
        let parked = cloth.particles[0].position;
        step(&mut cloth, gravity_cm(1.0));
        assert_eq!(cloth.particles[0].position, parked);
        assert_eq!(cloth.vertices(), vec![parked]);

        cloth.spheres.push(BodySphere {
            center: parked,
            radius: 10.0,
            friction: 0.0,
        });
        step(&mut cloth, Vec3::ZERO);
        let cleared = cloth.particles[0].position - parked;
        assert!(
            (cleared.length() - (10.0 + PADDING_CM)).abs() < 1e-3,
            "{cleared}"
        );
        assert_eq!(cloth.sleep, 1);
        assert_eq!(cloth.vertices(), vec![cloth.particles[0].position]);

        let mut glancing = Cloth::new(vec![Particle {
            position: Vec3::new(6.0, 4.0, 0.0),
            previous: Vec3::new(10.0, 0.0, 0.0),
        }]);
        glancing.sleep = 0;
        glancing.spheres.push(BodySphere {
            center: Vec3::ZERO,
            radius: 8.0,
            friction: 0.0,
        });
        let mut rubbed = glancing.clone();
        rubbed.spheres[0].friction = FRICTION[2];
        step(&mut glancing, Vec3::ZERO);
        step(&mut rubbed, Vec3::ZERO);
        assert!(glancing.particles[0].position.y > rubbed.particles[0].position.y);
        assert!(rubbed.particles[0].position.length() > 8.0);

        let mut cloth = Cloth::new(vec![still(Vec3::new(0.0, -5.0, 0.0))]);
        cloth.planes.push(LevelPlane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        });
        step(&mut cloth, Vec3::ZERO);
        assert!((cloth.particles[0].position.y - PADDING_CM).abs() < 1e-3);

        let mut cloth = Cloth::new(vec![Particle {
            position: Vec3::ZERO,
            previous: Vec3::new(-30.0, 0.0, 0.0),
        }]);
        step(&mut cloth, Vec3::ZERO);
        assert!(cloth.rustle);
        assert!(cloth.rustle_volume > 0.0);
        assert!((0.5..=2.0).contains(&cloth.rustle_pitch));
        let tail = cloth.rustle_tail;
        cloth.particles[0].previous = cloth.particles[0].position;
        step(&mut cloth, Vec3::ZERO);
        assert!(!cloth.rustle);
        assert_eq!(cloth.rustle_volume, 0.0);
        assert!((cloth.rustle_tail - tail * 0.9).abs() < 1e-5);

        let mut cloth = Cloth::new(vec![still(Vec3::new(0.0, 100.0, 0.0))]);
        let rest = cloth.vertices();
        step(&mut cloth, gravity_cm(1.0));
        assert_ne!(cloth.vertices(), rest);
        assert_eq!(cloth.vertices(), vec![cloth.particles[0].position]);
    }
}
