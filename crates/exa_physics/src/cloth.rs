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
//! rustle, sphere and plane pushes, a sphere swept through a cloth triangle,
//! and a vertex swept through a level triangle).

use bevy::math::Vec3;
use bevy::render::render_resource::Face;

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

/// A body sphere swept from `from` to `to`, in centimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweptSphere {
    pub from: Vec3,
    pub to: Vec3,
    pub radius: f32,
    pub friction: f32,
}

/// Three particle indices of a cloth triangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClothTri {
    pub a: usize,
    pub b: usize,
    pub c: usize,
}

/// A level triangle, in centimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelTri {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}

/// The 16×16 wind noise tile. A cell is 2 m, so a centimetre coordinate is
/// multiplied by 0.005 before the fractional lookup.
pub const NOISE_CELLS: usize = 16;
const NOISE_SCALE: f32 = 0.005;

/// Scene wind times 0.3, the strength the cloth's noise tile walks with.
pub fn cloth_wind(environment: f32) -> f32 {
    environment * 0.3
}

/// One frame of the noise tile. Inside −1 to 1 a sample walks by
/// `(random − 0.5) · wind · 0.8`. Outside that, it is pushed back by half
/// that stride times the random value.
pub fn advance_tile(samples: &mut [f32; NOISE_CELLS * NOISE_CELLS], wind: f32, randoms: &[f32]) {
    let stride = wind * 0.8;
    for (index, sample) in samples.iter_mut().enumerate() {
        let random = randoms.get(index).copied().unwrap_or(0.5);
        if !sample.is_finite() || !random.is_finite() || !stride.is_finite() {
            *sample = 0.0;
            continue;
        }
        *sample = if *sample > 1.0 {
            *sample - stride * 0.5 * random
        } else if *sample < -1.0 {
            *sample + stride * 0.5 * random
        } else {
            *sample + (random - 0.5) * stride
        };
    }
}

/// Bilinear sample of the tile at a world position in centimetres.
pub fn sample_tile(samples: &[f32; NOISE_CELLS * NOISE_CELLS], x_cm: f32, z_cm: f32) -> f32 {
    let (ix, fx) = tile_cell(x_cm);
    let (iz, fz) = tile_cell(z_cm);
    let at = |x: usize, z: usize| samples[z * NOISE_CELLS + x];
    let x1 = (ix + 1) % NOISE_CELLS;
    let z1 = (iz + 1) % NOISE_CELLS;
    let i00 = at(ix, iz);
    let i10 = at(x1, iz);
    let i01 = at(ix, z1);
    let i11 = at(x1, z1);
    (1.0 - fz) * ((1.0 - fx) * i00 + fx * i10) + fz * ((1.0 - fx) * i01 + fx * i11)
}

fn tile_cell(cm: f32) -> (usize, f32) {
    let mut unit = (cm * NOISE_SCALE).fract();
    if unit < 0.0 {
        unit += 1.0;
    }
    let scaled = unit * NOISE_CELLS as f32;
    let index = (scaled.floor() as usize) % NOISE_CELLS;
    (index, scaled - index as f32)
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
    pub swept: Vec<SweptSphere>,
    pub triangles: Vec<ClothTri>,
    pub level: Vec<LevelTri>,
    /// Wind noise, when the cloth samples the tile instead of `noise`.
    pub tile: Option<[f32; NOISE_CELLS * NOISE_CELLS]>,
    /// Equal and opposite sum of this tick's collision pushes, for the body.
    pub reaction: Vec3,
    /// The shipped cloth draws both faces.
    pub double_sided: bool,
    /// Distance along the mesh from the nearest pin, in centimetres.
    /// Unreachable vertices stay infinite. With no pins, vertex 0 is the source.
    pub distances: Vec<f32>,
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
            swept: Vec::new(),
            triangles: Vec::new(),
            level: Vec::new(),
            tile: None,
            reaction: Vec3::ZERO,
            double_sided: true,
            distances: Vec::new(),
            rustle: false,
            rustle_volume: 0.0,
            rustle_pitch: 0.0,
            rustle_tail: 0.0,
        }
    }

    /// Build a cloth from a triangle mesh. Each vertex is a particle. Each
    /// triangle edge is a structural link, and the two vertices opposite a
    /// shared edge are a bend link. Pinned vertices are held at their
    /// current point. The draw is double sided.
    pub fn from_mesh(vertices: &[Vec3], triangles: &[[usize; 3]], pinned: &[usize]) -> Self {
        let particles = vertices
            .iter()
            .copied()
            .map(|position| Particle {
                position,
                previous: position,
            })
            .collect();
        let mut cloth = Self::new(particles);
        let mut seen = Vec::<(usize, usize)>::new();
        let mut shared: Vec<((usize, usize), Vec<usize>)> = Vec::new();
        for tri in triangles {
            let corners = [tri[0], tri[1], tri[2]];
            if corners.iter().any(|index| *index >= vertices.len()) {
                continue;
            }
            cloth.triangles.push(ClothTri {
                a: corners[0],
                b: corners[1],
                c: corners[2],
            });
            for (slot, opposite) in corners.iter().copied().enumerate() {
                let start = corners[(slot + 1) % 3];
                let end = corners[(slot + 2) % 3];
                let key = ordered(start, end);
                let length = vertices[start].distance(vertices[end]);
                if !seen.contains(&key) {
                    seen.push(key);
                    cloth.edges.push(Edge {
                        a: start,
                        b: end,
                        length,
                    });
                }
                if let Some((_, opposites)) = shared.iter_mut().find(|(edge, _)| *edge == key) {
                    opposites.push(opposite);
                } else {
                    shared.push((key, vec![opposite]));
                }
            }
        }
        for (_, opposites) in shared {
            if opposites.len() >= 2 {
                let (start, end) = (opposites[0], opposites[1]);
                let key = ordered(start, end);
                if !seen.contains(&key) && start < vertices.len() && end < vertices.len() {
                    seen.push(key);
                    cloth.edges.push(Edge {
                        a: start,
                        b: end,
                        length: vertices[start].distance(vertices[end]),
                    });
                }
            }
        }
        for index in pinned {
            if let Some(vertex) = vertices.get(*index) {
                cloth.pins.push(Pin {
                    index: *index,
                    rest: *vertex,
                    weight: 1.0,
                });
            }
        }
        cloth.distances = geodesic(vertices, triangles, pinned);
        cloth
    }

    /// The GPU cull face of each submitted pass. Front is culled first, then
    /// back, so both sides of the cloth are drawn.
    pub fn submitted_culls(&self) -> Vec<Face> {
        self.draw_passes()
            .into_iter()
            .map(|pass| match pass {
                DrawPass::CullFront => Face::Front,
                DrawPass::CullBack => Face::Back,
            })
            .collect()
    }

    /// The double-sided draw hook: cull front faces and draw, then cull back
    /// faces and draw. A single-sided cloth only draws its front.
    pub fn draw_passes(&self) -> Vec<DrawPass> {
        if self.double_sided {
            vec![DrawPass::CullFront, DrawPass::CullBack]
        } else {
            vec![DrawPass::CullBack]
        }
    }

    /// Triangle corners to draw. Both windings when the cloth is double sided.
    pub fn faces(&self) -> Vec<(usize, usize, usize)> {
        let mut faces = Vec::new();
        for tri in &self.triangles {
            faces.push((tri.a, tri.b, tri.c));
            if self.double_sided {
                faces.push((tri.a, tri.c, tri.b));
            }
        }
        faces
    }

    /// Vertex positions the mesh draws, in centimetres.
    pub fn vertices(&self) -> Vec<Vec3> {
        self.particles
            .iter()
            .map(|particle| particle.position)
            .collect()
    }
}

/// The smallest half extent cloth bounds keep (the original's 1e-4 cm).
pub const BOUNDS_FLOOR_CM: f32 = 1.0e-4;

/// The box about a cloth's particles, in centimetres: the midpoint of the
/// per-component minimum and maximum, and the box's half extents (each at
/// least [`BOUNDS_FLOOR_CM`]). An empty cloth gives the origin and the
/// floor.
pub fn bounds(particles: &[Particle]) -> (Vec3, Vec3) {
    let Some(first) = particles.first() else {
        return (Vec3::ZERO, Vec3::splat(BOUNDS_FLOOR_CM));
    };
    let mut min = first.position;
    let mut max = first.position;
    for particle in &particles[1..] {
        min = min.min(particle.position);
        max = max.max(particle.position);
    }
    let centre = (min + max) * 0.5;
    let half = (max - centre).max(Vec3::splat(BOUNDS_FLOOR_CM));
    (centre, half)
}

/// A particle or sphere as a swept test sees it: its position, radius and
/// the velocity that carries it (and its radius) over the tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweptProbe {
    pub position: Vec3,
    pub radius: f32,
    pub velocity: Vec3,
    pub radius_rate: f32,
}

/// The first particle in `from..from + count` that a sphere swept over the
/// tick meets, as the original's cloth query does.
///
/// The relative motion is the sphere's velocity less the particle's. The
/// pair is taken as touching when they already overlap (their distance no
/// more than the difference of their radii) and the relative radius rate
/// covers that distance; otherwise when the squared distance times the
/// relative speed squared is within the approach's squared dot product
/// (the closest approach reaches them). The original's second branch also
/// allows a tolerance the first already implies.
pub fn swept_hit(
    particles: &[SweptProbe],
    sphere: &SweptProbe,
    from: usize,
    count: usize,
) -> Option<usize> {
    let end = from.saturating_add(count).min(particles.len());
    for (index, particle) in particles.iter().enumerate().take(end).skip(from) {
        let d = particle.position - sphere.position;
        let distance_sq = d.x * d.x + d.y * d.y + d.z * d.z;
        let reach = particle.radius - sphere.radius;
        if distance_sq <= reach * reach {
            let relative = sphere.velocity - particle.velocity;
            let rate = sphere.radius_rate - particle.radius_rate;
            if distance_sq <= rate * rate {
                return Some(index);
            }
            let speed_sq =
                relative.x * relative.x + relative.y * relative.y + relative.z * relative.z;
            if 1.192_09e-7 <= speed_sq {
                let dot = relative.x * d.x + relative.y * d.y + relative.z * d.z;
                let left = (distance_sq - rate * rate) * speed_sq;
                if left <= dot * dot {
                    let slack = speed_sq * 0.5 - dot.abs();
                    if left - dot * dot <= slack * slack {
                        return Some(index);
                    }
                }
            }
        }
    }
    None
}

/// The engine's generator step, as the cloth's blood painter draws it:
/// `s = s · 0x08088405 + 1`, returned as `(s as double) · 2⁻³²` in a
/// float, so 0 up to just under 1.
pub fn next_random(roll: &mut u32) -> f32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    (f64::from(*roll) * 2.328_306_4e-10) as f32
}

/// Paints blood on a cloth's vertex colours (RGBA8, alpha 255 clean) where
/// a ray from `point` along `direction` passes near a vertex, in the
/// vertex space.
///
/// The point is moved half a centimetre back along the direction and the
/// direction normalised. A vertex counts when it lies within the squared
/// radius of the point (or of the ray, for a vertex ahead of the point:
/// its squared distance scaled by `1 − t/|d|`, t the vertex's distance
/// along the direction). The paint's share is `1 − distance²/radius²`; a
/// draw blends it with a random value for the stain's edge, and the vertex
/// alpha takes `(1 − a²·share)·255` where that is below the alpha it has.
/// The engine's round helper is a floor. Returns how many vertices changed.
pub fn paint_blood(
    vertices: &[Vec3],
    colours: &mut [[u8; 4]],
    point: Vec3,
    direction: Vec3,
    radius_squared: f32,
    roll: &mut u32,
) -> usize {
    let point = point + direction * -0.5;
    let length = direction.length();
    let direction = direction * (1.0 / length);
    let mut painted = 0;
    for (index, vertex) in vertices.iter().enumerate() {
        let Some(colour) = colours.get_mut(index) else {
            break;
        };
        let d = *vertex - point;
        let mut distance_sq = d.x * d.x + d.y * d.y + d.z * d.z;
        let along = d.x * direction.x + d.y * direction.y + d.z * direction.z;
        if 0.0 < along {
            distance_sq *= 1.0 - along / distance_sq.sqrt();
        }
        if distance_sq <= radius_squared {
            let share = 1.0 - distance_sq / radius_squared;
            let random = next_random(roll);
            let blend = (1.0 - share) * random + share;
            let value = (1.0 - blend * blend * share) * 255.0;
            if value < f32::from(colour[3]) {
                colour[3] = value.floor() as u8;
                painted += 1;
            }
        }
    }
    painted
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
        let wind = cloth.wind;
        let wind_direction = cloth.wind_direction;
        let tile = cloth.tile;
        let noise = cloth.noise.clone();
        for (index, particle) in cloth.particles.iter_mut().enumerate() {
            if !particle.position.is_finite() || !particle.previous.is_finite() {
                continue;
            }
            let velocity = particle.position - particle.previous;
            let retained = (velocity + (mean - velocity) * MEAN_DRAG) * keep;
            let mut next = particle.position + retained + gravity;
            if wind > WIND_FLOOR {
                let sample = if let Some(tile) = tile {
                    sample_tile(&tile, particle.position.x, particle.position.z)
                } else {
                    noise.get(index).copied().unwrap_or(0.0)
                };
                if wind_direction.is_finite() && sample.is_finite() {
                    next += wind_direction * (sample + 0.25) * wind;
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
    let before: Vec<Vec3> = cloth
        .particles
        .iter()
        .map(|particle| particle.position)
        .collect();
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
    for index in 0..cloth.swept.len() {
        let body = cloth.swept[index];
        for tri in 0..cloth.triangles.len() {
            let tri = cloth.triangles[tri];
            hit |= sweep_sphere_triangle(&mut cloth.particles, tri, body);
        }
    }
    for index in 0..cloth.level.len() {
        let tri = cloth.level[index];
        for particle in &mut cloth.particles {
            hit |= sweep_vertex(particle, tri);
        }
    }
    cloth.reaction = Vec3::ZERO;
    for (particle, old) in cloth.particles.iter().zip(before) {
        cloth.reaction -= particle.position - old;
    }
    if hit && cloth.sleep == 0 {
        cloth.sleep = 1.max(cloth.owner_sleep);
    } else if cloth.sleep == 0 {
        for particle in &mut cloth.particles {
            particle.previous = particle.position;
        }
    }
}

/// One pass of the double-sided cloth draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawPass {
    /// Cull the front, so the back of the cloth is drawn.
    CullFront,
    /// Cull the back, so the front of the cloth is drawn.
    CullBack,
}

/// Distance along triangle edges from the nearest pin. No pins means vertex 0.
fn geodesic(vertices: &[Vec3], triangles: &[[usize; 3]], pinned: &[usize]) -> Vec<f32> {
    let count = vertices.len();
    let mut distance = vec![f32::INFINITY; count];
    let sources: Vec<usize> = if pinned.is_empty() {
        vec![0]
    } else {
        pinned.to_vec()
    };
    for source in sources {
        if source < count {
            distance[source] = 0.0;
        }
    }
    let mut neighbors = vec![Vec::<(usize, f32)>::new(); count];
    for tri in triangles {
        let corners = [tri[0], tri[1], tri[2]];
        if corners.iter().any(|index| *index >= count) {
            continue;
        }
        for slot in 0..3 {
            let start = corners[slot];
            let end = corners[(slot + 1) % 3];
            let length = vertices[start].distance(vertices[end]);
            neighbors[start].push((end, length));
            neighbors[end].push((start, length));
        }
    }
    let mut settled = vec![false; count];
    for _ in 0..count {
        let mut next = None;
        let mut best = f32::INFINITY;
        for (index, &so_far) in distance.iter().enumerate() {
            if !settled[index] && so_far < best {
                best = so_far;
                next = Some(index);
            }
        }
        let Some(current) = next else {
            break;
        };
        settled[current] = true;
        for &(neighbor, length) in &neighbors[current] {
            let along = distance[current] + length;
            if along < distance[neighbor] {
                distance[neighbor] = along;
            }
        }
    }
    distance
}

fn ordered(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
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

/// Where a segment from `from` to `to` crosses a triangle, if it does.
fn segment_triangle(from: Vec3, to: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<(f32, Vec3, Vec3)> {
    let dir = to - from;
    if !dir.is_finite() || dir.length_squared() <= f32::EPSILON {
        return None;
    }
    let ab = b - a;
    let ac = c - a;
    let pvec = dir.cross(ac);
    let det = ab.dot(pvec);
    if !det.is_finite() || det.abs() < 1e-6 {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = from - a;
    let u = tvec.dot(pvec) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qvec = tvec.cross(ab);
    let v = dir.dot(qvec) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = ac.dot(qvec) * inv;
    if !(0.0..=1.0).contains(&t) {
        return None;
    }
    let mut normal = ab.cross(ac).normalize_or_zero();
    if normal == Vec3::ZERO {
        return None;
    }
    if normal.dot(dir) > 0.0 {
        normal = -normal;
    }
    Some((t, from + dir * t, normal))
}

fn sweep_vertex(particle: &mut Particle, tri: LevelTri) -> bool {
    let Some((_, hit, normal)) =
        segment_triangle(particle.previous, particle.position, tri.a, tri.b, tri.c)
    else {
        return false;
    };
    particle.position = hit + normal * PADDING_CM;
    true
}

fn sweep_sphere_triangle(particles: &mut [Particle], tri: ClothTri, body: SweptSphere) -> bool {
    if tri.a >= particles.len() || tri.b >= particles.len() || tri.c >= particles.len() {
        return false;
    }
    if !body.radius.is_finite() || body.radius < 0.0 {
        return false;
    }
    let (a, b, c) = (
        particles[tri.a].position,
        particles[tri.b].position,
        particles[tri.c].position,
    );
    let Some((_, hit, normal)) = segment_triangle(body.from, body.to, a, b, c) else {
        return false;
    };
    let sphere = BodySphere {
        center: hit + normal * body.radius,
        radius: body.radius,
        friction: body.friction,
    };
    let mut struck = false;
    for index in [tri.a, tri.b, tri.c] {
        struck |= push_sphere(&mut particles[index], sphere);
    }
    struck
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

/// The cloth's start fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClothStart {
    pub flags_or: u32,
    pub seed: i32,
    pub matrix: [f32; 16],
    pub step: f32,
    pub link_or: u32,
    pub linked_body: Option<u32>,
}

/// Seeds the cloth, installs the identity matrix and derives its step.
pub fn start_cloth(count: i32, seed: i32, body: Option<(u32, u32)>) -> ClothStart {
    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let (link_or, linked_body) = match body {
        Some((id, flags)) if (flags & 0x1000_0000) != 0 => (1, Some(id)),
        _ => (0, None),
    };
    ClothStart {
        flags_or: 0x1_0000,
        seed,
        matrix: IDENTITY,
        step: (f64::from(count) * 1e-6).sqrt() as f32,
        link_or,
        linked_body,
    }
}

/// The cloth frame swap's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClothFrame {
    pub copies: u32,
    pub rotated: bool,
    pub pending: i32,
    pub flags_or: u32,
    pub frame: Option<i32>,
}

/// Copies the pending buffer or rotates the three vertex buffers.
pub fn submit_cloth_frame(pending: i32, frame: i32) -> ClothFrame {
    if pending <= 1 {
        ClothFrame {
            copies: 2,
            rotated: false,
            pending: 0,
            flags_or: 1,
            frame: None,
        }
    } else {
        ClothFrame {
            copies: 0,
            rotated: true,
            pending,
            flags_or: 0,
            frame: Some(frame),
        }
    }
}

/// The cloth activation's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClothActivateInput {
    /// The mesh node's deform flags; 0x2000_0000 marks the mesh as
    /// dynamically deformed.
    pub mesh_flags: u32,
    /// The mesh node's first flags word (the activation sets 0x1_0000; its
    /// meaning is a guess).
    pub node_flags: u32,
    /// The mesh node's class flags (the activation sets 0x20_4030).
    pub class_flags: u32,
    /// The mesh node's skinning flags (the activation sets 0x6000).
    pub skin_flags: u32,
    pub has_handle: bool,
    pub is_root: bool,
    pub slow_global: bool,
}

/// The cloth activation's new words and calls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClothActivate {
    pub mesh_flags: u32,
    pub node_flags: u32,
    pub class_flags: u32,
    pub skin_flags: u32,
    pub phase: i32,
    pub scale: f32,
    pub handle_written: bool,
    pub origin_copied: bool,
    pub slow_or: u32,
}

/// Sets the cloth's activation bits, phase and scale.
pub fn activate_cloth(input: ClothActivateInput) -> ClothActivate {
    ClothActivate {
        mesh_flags: input.mesh_flags | 0x2000_0000,
        node_flags: input.node_flags | 0x1_0000,
        class_flags: input.class_flags | 0x20_4030,
        skin_flags: input.skin_flags | 0x6000,
        phase: 2,
        scale: 0.4,
        handle_written: input.has_handle,
        origin_copied: input.is_root,
        slow_or: if input.slow_global { 0x8000 } else { 0 },
    }
}

/// The cloth transform's call counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClothTransform {
    pub particle_calls: u32,
    pub origin_calls: u32,
    pub extent_calls: u32,
}

/// Transforms all three particle buffers and the object's origin and extent.
pub fn transform_cloth(count: i32, object_flags: u32) -> ClothTransform {
    ClothTransform {
        particle_calls: count.max(0) as u32 * 3,
        origin_calls: u32::from((object_flags & 0x20) != 0),
        extent_calls: 1,
    }
}

/// The cloth rebuild's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClothRebuild {
    pub origin_copied: bool,
    pub transformed: i32,
    pub submitted: bool,
}

/// Copies the origin, transforms every point and submits the buffer.
pub fn rebuild_cloth(
    is_root: bool,
    has_override: bool,
    override_matches: bool,
    count: i32,
) -> ClothRebuild {
    if !is_root {
        return ClothRebuild {
            origin_copied: false,
            transformed: 0,
            submitted: false,
        };
    }
    if has_override && !override_matches {
        return ClothRebuild {
            origin_copied: true,
            transformed: 0,
            submitted: false,
        };
    }
    ClothRebuild {
        origin_copied: true,
        transformed: count.max(0),
        submitted: true,
    }
}

/// The cloth sync's work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClothSync {
    pub applied: bool,
    pub transform_calls: u32,
    pub pose_calls: u32,
    pub scale: f32,
}

/// Transforms and poses the cloth unless the meta bit 0x10 is set.
pub fn sync_cloth(meta_flags: u32) -> ClothSync {
    if (meta_flags & 0x10) != 0 {
        ClothSync {
            applied: false,
            transform_calls: 0,
            pose_calls: 0,
            scale: 0.0,
        }
    } else {
        ClothSync {
            applied: true,
            transform_calls: 1,
            pose_calls: 1,
            scale: 1.0,
        }
    }
}

/// One particle for the pressure pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClothPressureParticle {
    pub normal: Vec3,
    pub d8: Vec3,
    pub e0: Vec3,
}

/// The pressure pass's inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct ClothPressureInput {
    pub gate: u32,
    pub coupled: i32,
    pub particles: Vec<ClothPressureParticle>,
}

/// The pressure pass's outputs.
#[derive(Debug, Clone, PartialEq)]
pub struct ClothPressure {
    pub skipped: bool,
    pub normals: Vec<Vec3>,
    pub ws: Vec<f32>,
    pub d8: Vec<Vec3>,
    pub coupled_calls: i32,
    pub zeroed: bool,
    pub flag_cleared: bool,
}

/// Normalizes the normals, projects each edge and couples the result.
pub fn apply_cloth_pressure(input: ClothPressureInput) -> ClothPressure {
    if (input.gate & 0xf00) == 0x100 {
        return ClothPressure {
            skipped: true,
            normals: Vec::new(),
            ws: Vec::new(),
            d8: Vec::new(),
            coupled_calls: 0,
            zeroed: false,
            flag_cleared: false,
        };
    }
    let count = input.particles.len();
    let mut normals = Vec::with_capacity(count);
    let mut ws = Vec::with_capacity(count);
    let mut d8s = Vec::with_capacity(count);
    for particle in &input.particles {
        let len2 = particle.normal.length_squared();
        if len2 < 1e-6 {
            normals.push(Vec3::ZERO);
            ws.push(-1.0);
            d8s.push(particle.d8);
            continue;
        }
        let len = len2.sqrt();
        let unit = particle.normal / len;
        let w = unit.dot(particle.d8) + len;
        let mut d8 = particle.d8;
        let mut local = particle.e0 - d8;
        local -= unit * local.dot(unit);
        let local2 = local.length_squared();
        if f64::from(local2) > 0.2 {
            let scale = -(0.1f64 / f64::from(local2)).sqrt() as f32;
            d8 += local * scale;
        } else {
            d8 -= local;
        }
        normals.push(unit + d8 * -len);
        ws.push(w);
        d8s.push(d8);
    }
    ClothPressure {
        skipped: false,
        normals,
        ws,
        d8: d8s,
        coupled_calls: if input.coupled >= 2 {
            input.coupled - 1
        } else {
            0
        },
        zeroed: true,
        flag_cleared: true,
    }
}

/// The cloth destroy's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClothDestroyInput {
    pub free_flag: bool,
    pub pointers: [bool; 4],
    pub is_self_root: bool,
    pub has_origin: bool,
    pub has_list: bool,
    pub list_count: i32,
}

/// The cloth destroy's calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClothDestroy {
    pub release_calls: u32,
    pub pointer_frees: u32,
    pub origin_freed: bool,
    pub field_calls: u32,
    pub list_frees: i32,
    pub list_free: bool,
    pub base_calls: u32,
    pub final_calls: u32,
}

/// Releases the cloth's pointers, fields and list.
pub fn destroy_cloth(input: ClothDestroyInput) -> ClothDestroy {
    let free = u32::from(input.free_flag);
    ClothDestroy {
        release_calls: free,
        pointer_frees: input.pointers.iter().filter(|present| **present).count() as u32,
        origin_freed: input.is_self_root && input.has_origin,
        field_calls: 3,
        list_frees: if input.has_list {
            input.list_count.max(0)
        } else {
            0
        },
        list_free: input.has_list,
        base_calls: 1,
        final_calls: free,
    }
}

/// One particle for the body-collision pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClothCollideParticle {
    pub d8: Vec3,
    pub e0: Vec3,
    pub hit: bool,
    pub accepted: bool,
    pub side: bool,
    pub resolved: bool,
}

/// The body-collision pass's inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct ClothCollideInput {
    pub same_body: bool,
    pub radius: f32,
    pub particles: Vec<ClothCollideParticle>,
}

/// The body-collision pass's outputs.
#[derive(Debug, Clone, PartialEq)]
pub struct ClothCollide {
    pub particles: Vec<Vec3>,
    pub callback_calls: u32,
    pub indices: Vec<i32>,
    pub sides: Vec<bool>,
}

/// Pushes the hitting particles out to the body's radius.
pub fn collide_cloth(input: ClothCollideInput) -> ClothCollide {
    if input.same_body {
        return ClothCollide {
            particles: input.particles.iter().map(|particle| particle.d8).collect(),
            callback_calls: 0,
            indices: Vec::new(),
            sides: Vec::new(),
        };
    }
    let mut particles = Vec::with_capacity(input.particles.len());
    let mut callback_calls = 0;
    let mut indices = Vec::new();
    let mut sides = Vec::new();
    for (index, particle) in input.particles.iter().enumerate() {
        if !particle.hit || !particle.accepted {
            particles.push(particle.d8);
            continue;
        }
        callback_calls += 1;
        indices.push(index as i32);
        sides.push(particle.side);
        if !particle.resolved {
            particles.push(particle.d8);
            continue;
        }
        let edge = particle.e0 - particle.d8;
        let len2 = edge.length_squared();
        let corrected = if len2 <= input.radius * input.radius {
            particle.e0
        } else {
            let len = len2.sqrt();
            particle.e0 + edge * ((len - input.radius) / len)
        };
        particles.push(corrected);
    }
    ClothCollide {
        particles,
        callback_calls,
        indices,
        sides,
    }
}

/// The cloth clone's copied fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClothClone {
    pub created: bool,
    /// The template cloth whose constraint data the clone shares (a new
    /// cloth is its own template).
    pub template: u64,
    /// A word of the base simulation object; nothing in the cloth reads it,
    /// so its meaning is unknown.
    pub base_word: u32,
    /// The parameter record's flags (bits 0xF00 the mode, 0x7000 collision
    /// with bodies, 4 the double-sided draw).
    pub param_flags: u32,
    /// The parameter record (the node's cloth chunk or the built-in default).
    pub params: u64,
}

/// Creates a new cloth and copies its four fields.
pub fn clone_cloth(template: u64, base_word: u32, param_flags: u32, params: u64) -> ClothClone {
    ClothClone {
        created: true,
        template,
        base_word,
        param_flags,
        params,
    }
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

    #[test]
    fn a_swept_sphere_and_a_vertex_meet_a_triangle() {
        let mut cloth = Cloth::new(vec![
            still(Vec3::new(2.0, 2.0, 0.0)),
            still(Vec3::new(12.0, 2.0, 0.0)),
            still(Vec3::new(2.0, 12.0, 0.0)),
        ]);
        cloth.sleep = 0;
        cloth.triangles.push(ClothTri { a: 0, b: 1, c: 2 });
        cloth.swept.push(SweptSphere {
            from: Vec3::new(2.0, 2.0, 20.0),
            to: Vec3::new(2.0, 2.0, -10.0),
            radius: 4.0,
            friction: 0.0,
        });
        step(&mut cloth, Vec3::ZERO);
        assert!(
            (cloth.particles[0].position.z + PADDING_CM).abs() < 1e-2,
            "{:?}",
            cloth.particles[0].position
        );
        assert!(
            cloth.particles[1]
                .position
                .abs_diff_eq(Vec3::new(12.0, 2.0, 0.0), 1e-3)
        );

        let mut missed = cloth.clone();
        missed.particles[0].position = Vec3::new(2.0, 2.0, 0.0);
        missed.particles[0].previous = missed.particles[0].position;
        missed.swept[0].from = Vec3::new(100.0, 100.0, 20.0);
        missed.swept[0].to = Vec3::new(100.0, 100.0, -10.0);
        missed.sleep = 0;
        step(&mut missed, Vec3::ZERO);
        assert!(
            missed.particles[0]
                .position
                .abs_diff_eq(Vec3::new(2.0, 2.0, 0.0), 1e-3)
        );

        let floor = LevelTri {
            a: Vec3::new(-1.0, -1.0, 0.0),
            b: Vec3::new(20.0, -1.0, 0.0),
            c: Vec3::new(-1.0, 20.0, 0.0),
        };
        let mut cloth = Cloth::new(vec![Particle {
            previous: Vec3::new(2.0, 2.0, 8.0),
            position: Vec3::new(2.0, 2.0, -8.0),
        }]);
        cloth.sleep = 0;
        cloth.level.push(floor);
        step(&mut cloth, Vec3::ZERO);
        assert!(
            (cloth.particles[0].position.z - PADDING_CM).abs() < 1e-3,
            "{:?}",
            cloth.particles[0].position
        );
        let mut beside = Cloth::new(vec![Particle {
            previous: Vec3::new(100.0, 2.0, 8.0),
            position: Vec3::new(100.0, 2.0, -8.0),
        }]);
        beside.sleep = 0;
        beside.level.push(floor);
        step(&mut beside, Vec3::ZERO);
        assert!((beside.particles[0].position.z + 8.0).abs() < 1e-3);
    }

    #[test]
    fn a_mesh_builds_edges_and_the_wind_tile_pushes_the_body() {
        let vertices = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        let cloth = Cloth::from_mesh(&vertices, &[[0, 1, 2], [1, 3, 2]], &[0]);
        assert_eq!(cloth.particles.len(), 4);
        assert_eq!(cloth.edges.len(), 6);
        assert!(
            cloth
                .edges
                .iter()
                .any(|edge| ordered(edge.a, edge.b) == (0, 3))
        );
        assert_eq!(cloth.pins.len(), 1);
        assert_eq!(cloth.pins[0].weight, 1.0);
        assert_eq!(cloth.faces().len(), 4);
        assert!(cloth.faces().contains(&(0, 2, 1)));

        let mut samples = [0.0; NOISE_CELLS * NOISE_CELLS];
        let randoms = [1.0; NOISE_CELLS * NOISE_CELLS];
        advance_tile(&mut samples, cloth_wind(1.0 / 0.3), &randoms);
        assert!((samples[0] - 0.4).abs() < 1e-4, "{}", samples[0]);
        samples[0] = 1.5;
        advance_tile(&mut samples, 1.0, &randoms);
        assert!((samples[0] - 1.1).abs() < 1e-4, "{}", samples[0]);
        let flat = [1.0; NOISE_CELLS * NOISE_CELLS];
        assert!((sample_tile(&flat, 0.0, 0.0) - 1.0).abs() < 1e-5);

        let mut blown = Cloth::new(vec![still(Vec3::ZERO)]);
        blown.wind = 1.0;
        blown.wind_direction = Vec3::X;
        blown.tile = Some(flat);
        step(&mut blown, Vec3::ZERO);
        assert!((blown.particles[0].position.x - 1.25).abs() < 1e-3);

        let mut cloth = Cloth::new(vec![still(Vec3::ZERO)]);
        cloth.sleep = 0;
        cloth.spheres.push(BodySphere {
            center: Vec3::ZERO,
            radius: 10.0,
            friction: 0.0,
        });
        step(&mut cloth, Vec3::ZERO);
        let push = cloth.particles[0].position;
        assert!(
            push.abs_diff_eq(-cloth.reaction, 1e-3),
            "{:?} vs {:?}",
            push,
            cloth.reaction
        );
    }

    #[test]
    fn pin_distance_follows_the_mesh_and_the_draw_uses_both_faces() {
        let vertices = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let cloth = Cloth::from_mesh(&vertices, &[[0, 1, 3], [1, 2, 3]], &[0]);
        assert!(cloth.distances[0].abs() < 1e-4);
        assert!((cloth.distances[1] - 1.0).abs() < 1e-4);
        assert!((cloth.distances[3] - 1.0).abs() < 1e-4);
        assert!(
            (cloth.distances[2] - 2.0).abs() < 1e-4,
            "along the edges, not the straight {}",
            cloth.distances[2]
        );
        assert_eq!(
            cloth.draw_passes(),
            vec![DrawPass::CullFront, DrawPass::CullBack]
        );
        assert_eq!(cloth.submitted_culls(), vec![Face::Front, Face::Back]);
        let mut single = cloth.clone();
        single.double_sided = false;
        assert_eq!(single.draw_passes(), vec![DrawPass::CullBack]);
        assert_eq!(single.submitted_culls(), vec![Face::Back]);
    }

    #[test]
    fn the_cloth_starts_and_swaps_its_frames() {
        let started = start_cloth(10000, 7, Some((9, 0x1000_0000)));
        assert_eq!(started.flags_or, 0x1_0000);
        assert_eq!(started.seed, 7);
        assert_eq!(started.step.to_bits(), 0.1f32.to_bits());
        assert_eq!(started.link_or, 1);
        assert_eq!(started.linked_body, Some(9));
        assert_eq!(started.matrix[0], 1.0);
        assert_eq!(started.matrix[5], 1.0);
        assert_eq!(started.matrix[10], 1.0);
        assert_eq!(started.matrix[15], 1.0);
        assert_eq!(started.matrix[1], 0.0);

        let plain = start_cloth(100, 3, Some((4, 0)));
        assert_eq!(plain.link_or, 0);
        assert_eq!(plain.linked_body, None);

        let pending = submit_cloth_frame(1, 9);
        assert_eq!(pending.copies, 2);
        assert_eq!(pending.pending, 0);
        assert_eq!(pending.flags_or, 1);
        assert!(!pending.rotated);
        assert_eq!(pending.frame, None);

        let rotating = submit_cloth_frame(2, 9);
        assert_eq!(rotating.copies, 0);
        assert!(rotating.rotated);
        assert_eq!(rotating.pending, 2);
        assert_eq!(rotating.flags_or, 0);
        assert_eq!(rotating.frame, Some(9));
    }

    #[test]
    fn the_cloth_activates_and_transforms() {
        let active = activate_cloth(ClothActivateInput {
            mesh_flags: 0,
            node_flags: 0,
            class_flags: 0,
            skin_flags: 0,
            has_handle: true,
            is_root: true,
            slow_global: true,
        });
        assert_eq!(active.mesh_flags, 0x2000_0000);
        assert_eq!(active.node_flags, 0x1_0000);
        assert_eq!(active.class_flags, 0x20_4030);
        assert_eq!(active.skin_flags, 0x6000);
        assert_eq!(active.phase, 2);
        assert_eq!(active.scale.to_bits(), 0.4f32.to_bits());
        assert!(active.handle_written);
        assert!(active.origin_copied);
        assert_eq!(active.slow_or, 0x8000);

        let quiet = activate_cloth(ClothActivateInput {
            mesh_flags: 1,
            node_flags: 2,
            class_flags: 4,
            skin_flags: 8,
            has_handle: false,
            is_root: false,
            slow_global: false,
        });
        assert_eq!(quiet.mesh_flags, 1 | 0x2000_0000);
        assert_eq!(quiet.slow_or, 0);
        assert!(!quiet.handle_written);

        let all = transform_cloth(3, 0x20);
        assert_eq!(all.particle_calls, 9);
        assert_eq!(all.origin_calls, 1);
        assert_eq!(all.extent_calls, 1);
        let plain = transform_cloth(3, 0);
        assert_eq!(plain.origin_calls, 0);
        assert_eq!(plain.extent_calls, 1);
    }

    #[test]
    fn the_cloth_rebuilds_only_from_its_root() {
        let rebuilt = rebuild_cloth(true, false, false, 3);
        assert!(rebuilt.origin_copied);
        assert_eq!(rebuilt.transformed, 3);
        assert!(rebuilt.submitted);

        let other = rebuild_cloth(false, false, false, 3);
        assert!(!other.origin_copied);
        assert_eq!(other.transformed, 0);
        assert!(!other.submitted);

        let mismatch = rebuild_cloth(true, true, false, 3);
        assert!(mismatch.origin_copied);
        assert_eq!(mismatch.transformed, 0);
        assert!(!mismatch.submitted);

        let matched = rebuild_cloth(true, true, true, 3);
        assert_eq!(matched.transformed, 3);
        assert!(matched.submitted);
    }

    #[test]
    fn the_cloth_sync_skips_on_the_meta_bit() {
        let ran = sync_cloth(0);
        assert!(ran.applied);
        assert_eq!((ran.transform_calls, ran.pose_calls), (1, 1));
        assert_eq!(ran.scale.to_bits(), 1.0f32.to_bits());

        let skipped = sync_cloth(0x10);
        assert!(!skipped.applied);
        assert_eq!((skipped.transform_calls, skipped.pose_calls), (0, 0));
    }

    #[test]
    fn the_cloth_pressure_normalizes_and_corrects() {
        let result = apply_cloth_pressure(ClothPressureInput {
            gate: 0,
            coupled: 3,
            particles: vec![
                ClothPressureParticle {
                    normal: Vec3::ZERO,
                    d8: Vec3::new(1.0, 1.0, 1.0),
                    e0: Vec3::new(2.0, 2.0, 2.0),
                },
                ClothPressureParticle {
                    normal: Vec3::new(3.0, 0.0, 0.0),
                    d8: Vec3::new(0.0, 2.0, 0.0),
                    e0: Vec3::new(0.0, 2.0, 5.0),
                },
            ],
        });
        assert!(!result.skipped);
        assert_eq!(result.normals[0], Vec3::ZERO);
        assert_eq!(result.ws[0], -1.0);
        assert_eq!(result.d8[0], Vec3::new(1.0, 1.0, 1.0));
        assert!((result.ws[1] - 3.0).abs() < 1e-5);
        assert!((result.normals[1].x - 1.0).abs() < 1e-5);
        assert!((result.normals[1].y + 6.0).abs() < 1e-5);
        assert!((result.normals[1].z - 0.948_683_3).abs() < 1e-5);
        assert!(result.d8[1].x.abs() < 1e-5);
        assert!((result.d8[1].y - 2.0).abs() < 1e-5);
        assert!((result.d8[1].z + 0.316_227_77).abs() < 1e-5);
        assert_eq!(result.coupled_calls, 2);
        assert!(result.zeroed);
        assert!(result.flag_cleared);

        let gated = apply_cloth_pressure(ClothPressureInput {
            gate: 0x100,
            coupled: 3,
            particles: vec![],
        });
        assert!(gated.skipped);
        assert_eq!(gated.coupled_calls, 0);
        assert!(!gated.zeroed);
    }

    #[test]
    fn the_cloth_destroys_its_owned_parts() {
        let freeing = destroy_cloth(ClothDestroyInput {
            free_flag: true,
            pointers: [true; 4],
            is_self_root: true,
            has_origin: true,
            has_list: true,
            list_count: 3,
        });
        assert_eq!(freeing.release_calls, 1);
        assert_eq!(freeing.pointer_frees, 4);
        assert!(freeing.origin_freed);
        assert_eq!(freeing.field_calls, 3);
        assert_eq!(freeing.list_frees, 3);
        assert!(freeing.list_free);
        assert_eq!(freeing.base_calls, 1);
        assert_eq!(freeing.final_calls, 1);

        let quiet = destroy_cloth(ClothDestroyInput {
            free_flag: false,
            pointers: [false; 4],
            is_self_root: false,
            has_origin: false,
            has_list: false,
            list_count: 0,
        });
        assert_eq!(quiet.release_calls, 0);
        assert_eq!(quiet.pointer_frees, 0);
        assert!(!quiet.origin_freed);
        assert_eq!(quiet.field_calls, 3);
        assert_eq!(quiet.list_frees, 0);
        assert!(!quiet.list_free);
        assert_eq!(quiet.base_calls, 1);
        assert_eq!(quiet.final_calls, 0);
    }

    #[test]
    fn the_cloth_collision_pushes_to_the_radius() {
        let particle = |d8, e0, hit, accepted, side, resolved| ClothCollideParticle {
            d8,
            e0,
            hit,
            accepted,
            side,
            resolved,
        };
        let pushed = collide_cloth(ClothCollideInput {
            same_body: false,
            radius: 1.0,
            particles: vec![
                particle(Vec3::new(0.0, 2.0, 0.0), Vec3::ZERO, true, true, true, true),
                particle(
                    Vec3::new(0.5, 0.0, 0.0),
                    Vec3::ZERO,
                    true,
                    true,
                    false,
                    true,
                ),
                particle(
                    Vec3::new(3.0, 3.0, 3.0),
                    Vec3::ZERO,
                    false,
                    true,
                    false,
                    true,
                ),
                particle(
                    Vec3::new(4.0, 4.0, 4.0),
                    Vec3::ZERO,
                    true,
                    false,
                    false,
                    true,
                ),
                particle(
                    Vec3::new(5.0, 5.0, 5.0),
                    Vec3::ZERO,
                    true,
                    true,
                    true,
                    false,
                ),
            ],
        });
        assert!((pushed.particles[0].y + 1.0).abs() < 1e-5);
        assert_eq!(pushed.particles[1], Vec3::ZERO);
        assert_eq!(pushed.particles[2], Vec3::new(3.0, 3.0, 3.0));
        assert_eq!(pushed.particles[3], Vec3::new(4.0, 4.0, 4.0));
        assert_eq!(pushed.particles[4], Vec3::new(5.0, 5.0, 5.0));
        assert_eq!(pushed.callback_calls, 3);
        assert_eq!(pushed.indices, vec![0, 1, 4]);
        assert_eq!(pushed.sides, vec![true, false, true]);

        let own = collide_cloth(ClothCollideInput {
            same_body: true,
            radius: 1.0,
            particles: vec![particle(
                Vec3::new(0.0, 2.0, 0.0),
                Vec3::ZERO,
                true,
                true,
                true,
                true,
            )],
        });
        assert_eq!(own.callback_calls, 0);
        assert_eq!(own.particles[0], Vec3::new(0.0, 2.0, 0.0));
    }

    #[test]
    fn the_cloth_clone_copies_its_fields() {
        let cloned = clone_cloth(0x1122, 0x33, 0x44, 0x55);
        assert!(cloned.created);
        assert_eq!(cloned.template, 0x1122);
        assert_eq!(cloned.base_word, 0x33);
        assert_eq!(cloned.param_flags, 0x44);
        assert_eq!(cloned.params, 0x55);
    }

    #[test]
    fn cloth_bounds_midpoint_the_extremes() {
        let cloth = Cloth::new(vec![
            still(Vec3::new(-10.0, 0.0, -20.0)),
            still(Vec3::new(10.0, 5.0, 20.0)),
        ]);
        let (centre, half) = bounds(&cloth.particles);
        assert_eq!(centre, Vec3::new(0.0, 2.5, 0.0));
        assert_eq!(half, Vec3::new(10.0, 2.5, 20.0));
        let flat = Cloth::new(vec![still(Vec3::new(3.0, 4.0, 5.0))]);
        let (centre, half) = bounds(&flat.particles);
        assert_eq!(centre, Vec3::new(3.0, 4.0, 5.0));
        assert_eq!(half, Vec3::splat(BOUNDS_FLOOR_CM));
        assert_eq!(bounds(&[]), (Vec3::ZERO, Vec3::splat(BOUNDS_FLOOR_CM)));
    }

    #[test]
    fn a_swept_sphere_finds_the_first_particle_it_meets() {
        // A large cloth vertex at the origin and a small sphere passing it
        // 0.4 cm off the line: the pair already overlaps the vertex's
        // envelope, and the sphere's growing radius covers the 0.4 cm
        // approach (the squared impact parameter, 0.16) when the relative
        // radius rate is 1, but not at 0.1.
        let particles = [SweptProbe {
            position: Vec3::ZERO,
            radius: 100.0,
            velocity: Vec3::ZERO,
            radius_rate: 0.0,
        }];
        let sphere = SweptProbe {
            position: Vec3::new(0.0, 0.4, 90.0),
            radius: 1.0,
            velocity: Vec3::new(0.0, 0.0, 10.0),
            radius_rate: 1.0,
        };
        assert_eq!(swept_hit(&particles, &sphere, 0, 1), Some(0));
        let thin = SweptProbe {
            radius_rate: 0.1,
            ..sphere
        };
        assert_eq!(swept_hit(&particles, &thin, 0, 1), None);
        // A sphere resting inside the envelope at the same point hits at
        // once; a far one does not.
        let inside = SweptProbe {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            radius_rate: 0.0,
            ..sphere
        };
        assert_eq!(swept_hit(&particles, &inside, 0, 1), Some(0));
        let far = SweptProbe {
            position: Vec3::new(200.0, 0.0, 0.0),
            ..sphere
        };
        assert_eq!(swept_hit(&particles, &far, 0, 1), None);
        // The index range starts at `from` and stops at `from + count`.
        let second = SweptProbe {
            position: Vec3::ZERO,
            radius: 100.0,
            velocity: Vec3::ZERO,
            radius_rate: 0.0,
        };
        let pair = [particles[0], second];
        assert_eq!(swept_hit(&pair, &inside, 0, 1), Some(0));
        assert_eq!(swept_hit(&pair, &inside, 1, 1), Some(1));
        assert_eq!(swept_hit(&pair, &inside, 2, 1), None);
    }

    #[test]
    fn blood_paints_the_vertices_the_ray_meets() {
        let vertices = [
            Vec3::ZERO,
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 3.0),
        ];
        let mut colours = [[255u8, 255, 255, 255]; 3];
        let mut roll = 1;
        // Radius² 1 at the origin along +z: the first vertex is clean.
        let painted = paint_blood(&vertices, &mut colours, Vec3::ZERO, Vec3::Z, 1.0, &mut roll);
        assert_eq!(painted, 1);
        assert_eq!(colours[0][3], 0);
        assert_eq!(colours[1][3], 255);
        assert_eq!(colours[2][3], 255);
        // A vertex ahead of the point on the ray, 0.5 cm off it: the ray's
        // distance leaves it within radius² 1.
        let mut off = [[255u8, 255, 255, 255]; 1];
        let mut roll = 1;
        assert_eq!(
            paint_blood(
                &[Vec3::new(0.5, 0.0, 2.0)],
                &mut off,
                Vec3::ZERO,
                Vec3::Z,
                1.0,
                &mut roll
            ),
            1
        );
        assert!(off[0][3] < 255);
        // The generator starts at 1: s = 1·0x08088405 + 1.
        let mut roll = 1;
        let first = next_random(&mut roll);
        assert_eq!(roll, 0x0808_8406);
        assert!((f64::from(first) - f64::from(0x0808_8406u32) * 2.328_306_4e-10).abs() < 1e-9);
        assert!((0.0..1.0).contains(&first));
    }
}
