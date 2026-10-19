//! A cloth step: Verlet particles, a wind nudge, pins and edge constraints.
//!
//! Each free particle keeps `0.98977 × 0.5^(1/83)` of its velocity, dragged
//! 5% toward last tick's mean, then gains gravity. Gravity is the body's
//! acceleration, in centimetres per tick. Wind above 0.001 moves a particle
//! by `direction · (noise + 0.25) · wind`. A pin moves toward its rest point
//! by its weight. An edge corrects by
//! `(L² / (|d|² + L²) − 0.5) · 0.8774` without a square root. One pass, which
//! is the shipped cloth's mode. Sleep, rustle, collisions and the mesh are
//! not in this step.
//!
//! Evidence: the cloth step notes (retention, drag, wind, pins, edge correction).

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
        }
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
        sum += next - particle.position;
        particle.previous = particle.position;
        particle.position = next;
    }
    cloth.mean_velocity = sum / count as f32;
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
}
