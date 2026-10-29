//! A cloth step: Verlet particles, a wind nudge, pins and edge constraints.
//!
//! Each free particle keeps `0.98977 × 0.5^(1/83)` of its velocity, dragged
//! 5% toward last tick's mean, then gains gravity. Gravity is the body's
//! acceleration, in centimetres per tick. Wind above 0.001 moves a particle
//! by `direction · (noise + 0.25) · wind`. A pin moves toward its rest point
//! by its weight. An edge corrects by
//! `(L² / (|d|² + L²) − 0.5) · 0.8774` without a square root. The shipped
//! cloth's mode makes one such pass in the step and, while awake, three
//! more after the body stage ([`crate::cloth_body`]), which pushes the
//! cloth's triangles off the body spheres the sector hands it and holds
//! the pushed vertices on their push planes in those passes. A level plane
//! pushes a vertex out to 2 cm along its normal. The three body frictions
//! are 0.002, 0.01 and 0.015 (the body's share of a contact). Sleep counts
//! down while the squared length of the particles' summed velocity (their
//! travel over the last tick, before drag, gravity and wind) is under
//! √(vertices × 10⁻⁶), counts up (capped at 32) over twice that squared,
//! and never stays below the owner's count; a body that pushes the cloth
//! hands it its own countdown when that is higher, and the tick's end puts
//! an awake cloth whose count is under 2 to sleep. That sum over the vertex
//! count is the mean the drag pulls toward. Rustle plays when the same
//! sum's squared length exceeds 400. The mesh vertices are the particle
//! positions.
//!
//! Evidence: the cloth step notes (retention, drag, wind, pins, edges, sleep,
//! rustle, plane pushes), the cloth's integrate slot, its post-body push
//! pass and its end-of-tick slot (REA assembly); the bodies are
//! [`crate::cloth_body`]'s and the level's meshes [`crate::cloth_level`]'s,
//! after the step.

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
/// The share of a cloth contact handed back to the body: 0.015 for a body
/// of the cloth-colliding class, 0.01 when the cloth's own class flag is
/// set, else 0.002. The game does not hand it back yet
/// ([`crate::cloth_body`]).
pub const FRICTION: [f32; 3] = [0.002, 0.01, 0.015];
/// Sleep countdown does not count itself up past this (a body may hand it
/// a higher one).
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

/// The face-flag bits that mark a cloth face as pinned (the top nibble of
/// the mesh's per-face flags, sub-chunk `0x3DC3`).
pub const PINNED_FACES: u32 = 0xF000_0000;
/// Of those, the bits that pin firmly; a pinned face with neither pins
/// loosely.
pub const FIRM_PIN_FACES: u32 = 0xC000_0000;
/// The weight of a firm pin: the vertex sits on its rest point.
pub const FIRM_PIN: f32 = 1.0;
/// The weight of a loose pin.
pub const LOOSE_PIN: f32 = 0.1;

/// The cloth's pin list, as the original builds it when its controller
/// starts: a position element (a particle) is pinned when every face about
/// it carries a bit of [`PINNED_FACES`], firmly ([`FIRM_PIN`]) when each of
/// them also carries a bit of [`FIRM_PIN_FACES`] and loosely
/// ([`LOOSE_PIN`]) otherwise; one face without the bits leaves it free. An
/// element no face touches is pinned firmly, as the original's empty walk
/// leaves it. `faces` are triangles over the elements, `flags` their face
/// flags in the same order (a face past the end has none). Returns the
/// pinned elements in order with their weights.
///
/// Every shipped cloth (the four banners and the wraith's robe) marks its
/// hem faces with `0x80000000`, so its hem is held firmly and the rest
/// hangs free.
pub fn face_flag_pins(elements: usize, faces: &[[u32; 3]], flags: &[u32]) -> Vec<(usize, f32)> {
    let mut free = vec![false; elements];
    let mut loose = vec![false; elements];
    for (index, face) in faces.iter().enumerate() {
        let flag = flags.get(index).copied().unwrap_or(0);
        for &corner in face {
            let Some(element) = usize::try_from(corner).ok().filter(|&e| e < elements) else {
                continue;
            };
            if flag & PINNED_FACES == 0 {
                free[element] = true;
            } else if flag & FIRM_PIN_FACES == 0 {
                loose[element] = true;
            }
        }
    }
    (0..elements)
        .filter(|&element| !free[element])
        .map(|element| {
            let weight = if loose[element] { LOOSE_PIN } else { FIRM_PIN };
            (element, weight)
        })
        .collect()
}

/// A level plane. The normal points out of the solid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelPlane {
    pub point: Vec3,
    pub normal: Vec3,
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
    /// The bodies the sector hands the cloth this tick, in its order
    /// ([`crate::cloth_body`]).
    pub bodies: Vec<crate::cloth_body::ClothBody>,
    /// The body stage's push buffer, one per particle (cm, the cloth's
    /// frame); the awake cloth applies and clears it after the stage.
    pub pushes: Vec<Vec3>,
    /// How many triangle records the body stage's swept query looked at
    /// last tick (a debug count; no rule reads it).
    pub body_tests: usize,
    /// The bodies (indices into `bodies`) that pushed the cloth last tick
    /// (a debug list; no rule reads it).
    pub pushed_by: Vec<usize>,
    pub planes: Vec<LevelPlane>,
    pub triangles: Vec<ClothTri>,
    pub level: Vec<LevelTri>,
    /// The level meshes `level` came from, in order: each one's box and
    /// where its triangles end ([`crate::cloth_level::touch_level`]). Empty
    /// means one mesh holding the whole list.
    pub level_pieces: Vec<crate::cloth_level::LevelPiece>,
    /// How many particle-triangle tests the last level stage made (a debug
    /// count; no rule reads it).
    pub level_tests: usize,
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
    /// The hand's grab: the vertex it holds and the point it is pulled to
    /// (centimetres, the cloth's own frame).
    pub grabbed: Option<(usize, Vec3)>,
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
            bodies: Vec::new(),
            pushes: Vec::new(),
            body_tests: 0,
            pushed_by: Vec::new(),
            planes: Vec::new(),
            triangles: Vec::new(),
            level: Vec::new(),
            level_pieces: Vec::new(),
            level_tests: 0,
            tile: None,
            reaction: Vec3::ZERO,
            double_sided: true,
            distances: Vec::new(),
            rustle: false,
            rustle_volume: 0.0,
            rustle_pitch: 0.0,
            rustle_tail: 0.0,
            grabbed: None,
        }
    }

    /// The hand grabs vertex `index` and pulls it to `at` (centimetres, the
    /// cloth's own frame). The item's own drag rules decide the point.
    pub fn grab(&mut self, index: usize, at: Vec3) {
        if index < self.particles.len() {
            self.grabbed = Some((index, at));
            self.sleep = self.sleep.max(2).max(self.owner_sleep);
        }
    }

    /// The hand lets go.
    pub fn release_grab(&mut self) {
        self.grabbed = None;
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
///
/// The extremes follow the vector min and max instructions, which hand back
/// the particle when either side is not a number, and the floor is applied
/// the same way (a half extent that is not above it becomes the floor).
pub fn bounds(particles: &[Particle]) -> (Vec3, Vec3) {
    let Some(first) = particles.first() else {
        return (Vec3::ZERO, Vec3::splat(BOUNDS_FLOOR_CM));
    };
    let mut min = first.position;
    let mut max = first.position;
    for particle in &particles[1..] {
        min = Vec3::from_array(std::array::from_fn(|i| {
            if min[i] < particle.position[i] {
                min[i]
            } else {
                particle.position[i]
            }
        }));
        max = Vec3::from_array(std::array::from_fn(|i| {
            if max[i] > particle.position[i] {
                max[i]
            } else {
                particle.position[i]
            }
        }));
    }
    let centre = (min + max) * 0.5;
    let spread = max - centre;
    let half = Vec3::from_array(std::array::from_fn(|i| {
        if spread[i] > BOUNDS_FLOOR_CM {
            spread[i]
        } else {
            BOUNDS_FLOOR_CM
        }
    }));
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

/// The first particle in `from..end` that a sphere swept over the tick
/// meets, as the original's cloth query does; `None` when none is met.
///
/// The relative motion is the sphere's velocity less the particle's. A
/// particle is only looked at when the squared distance between the two is
/// within the squared difference of their radii. It is a hit at once when
/// the squared distance is within the squared relative radius rate.
/// Otherwise, when the relative speed squared reaches the engine's float
/// epsilon, the closest approach decides: with `dot` the relative motion
/// along the separation, `left = (distance² − rate²) · speed²` must not
/// exceed `dot²`, and `left − dot²` must be within `slack · |slack|` for
/// `slack = speed²/2 − |dot|` (the slack goes negative, and so demands a
/// deeper miss, once the approach lies outside the tick's motion). Every
/// compare is the original's, so a value that is not a number skips the
/// "above" tests and passes the "not above" ones.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn swept_hit(
    particles: &[SweptProbe],
    sphere: &SweptProbe,
    from: usize,
    end: usize,
) -> Option<usize> {
    let end = end.min(particles.len());
    for (index, particle) in particles.iter().enumerate().take(end).skip(from) {
        let d = particle.position - sphere.position;
        let distance_sq = d.x * d.x + d.y * d.y + d.z * d.z;
        let reach = particle.radius - sphere.radius;
        if distance_sq > reach * reach {
            continue;
        }
        let relative = sphere.velocity - particle.velocity;
        let rate = sphere.radius_rate - particle.radius_rate;
        let rate_sq = rate * rate;
        if !(distance_sq > rate_sq) {
            return Some(index);
        }
        let diff = distance_sq - rate_sq;
        let speed_sq = relative.x * relative.x + relative.y * relative.y + relative.z * relative.z;
        if speed_sq < SPEED_FLOOR || speed_sq.is_nan() {
            continue;
        }
        let dot = relative.x * d.x + relative.y * d.y + relative.z * d.z;
        let left = diff * speed_sq;
        if left > dot * dot {
            continue;
        }
        let slack = speed_sq * 0.5 - dot.abs();
        if !(left - dot * dot > slack.abs() * slack) {
            return Some(index);
        }
    }
    None
}

/// The relative speed squared below which the swept query does not look at
/// the closest approach (the engine's float epsilon constant).
const SPEED_FLOOR: f32 = f32::from_bits(0x33ff_ffd7);

/// The engine's generator step, as the cloth's blood painter draws it:
/// `s = s · 0x08088405 + 1`, returned as `(s as double) · 2⁻³²` in a
/// float, so 0 up to just under 1.
pub fn next_random(roll: &mut u32) -> f32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    (f64::from(*roll) * (1.0 / 4_294_967_296.0)) as f32
}

/// Paints blood on a cloth's vertex colours (RGBA8, alpha 255 clean) where
/// a ray from `point` along `direction` passes near a vertex, in the
/// vertex space.
///
/// The point is moved half a centimetre back along the direction and the
/// direction normalised (its length summed the engine's way: x², z², then
/// y²). A vertex counts unless the squared radius is below its squared
/// distance from the point (or from the ray, for a vertex ahead of the
/// point: the squared distance scaled by `1 − t/d`, with `t` the vertex's
/// distance along the direction and `d` its distance). The paint's share is
/// `1 − distance²/radius²`; a draw blends it with a random value for the
/// stain's edge, and the vertex alpha takes `(1 − blend²·share)·255` where
/// that is below the alpha it has, rounded to nearest, ties to even, as the
/// hardware conversion does. Returns how many vertices changed.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn paint_blood(
    vertices: &[Vec3],
    colours: &mut [[u8; 4]],
    point: Vec3,
    direction: Vec3,
    radius_squared: f32,
    roll: &mut u32,
) -> usize {
    let point = point + direction * -0.5;
    let length = ((direction.x * direction.x + direction.z * direction.z)
        + direction.y * direction.y)
        .sqrt();
    let direction = direction * (1.0 / length);
    let mut painted = 0;
    for (index, vertex) in vertices.iter().enumerate() {
        let Some(colour) = colours.get_mut(index) else {
            break;
        };
        let d = *vertex - point;
        let mut distance_sq = d.y * d.y + d.x * d.x + d.z * d.z;
        let along = d.y * direction.y + d.x * direction.x + d.z * direction.z;
        if 0.0 < along {
            distance_sq *= 1.0 - along / distance_sq.sqrt();
        }
        if radius_squared < distance_sq {
            continue;
        }
        let share = 1.0 - distance_sq / radius_squared;
        let random = next_random(roll);
        let blend = (1.0 - share) * random + share;
        let value = (1.0 - blend * blend * share) * 255.0;
        if value < f32::from(colour[3]) {
            // The low byte of the 64-bit conversion, which is what the
            // original stores.
            colour[3] = (value.round_ties_even() as i64) as u8;
            painted += 1;
        }
    }
    painted
}

/// The cloth's blend after its owner moved, from three particle buffers.
///
/// For each particle, with `d = base − earlier` (how far the target moved
/// between two ticks), writes
/// `(d + (target − d)·k)·weight + offset + base` into `out`, and returns
/// the sum of every `d`. `target` and `k` are cloth fields in the original
/// (the target vector and the blend share), `weight` and `offset` come from
/// the caller. This is not a pin: the step's pins pull toward fixed rest
/// points; this pass carries the target's own displacement onto the
/// particles.
pub fn pull_toward(
    out: &mut [Vec3],
    base: &[Vec3],
    earlier: &[Vec3],
    target: Vec3,
    offset: Vec3,
    k: f32,
    weight: f32,
) -> Vec3 {
    let mut sum = Vec3::ZERO;
    for ((out, base), earlier) in out.iter_mut().zip(base).zip(earlier) {
        let d = *base - *earlier;
        sum += d;
        *out = (d + (target - d) * k) * weight + offset + *base;
    }
    sum
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
    // The sum of the particles' velocities (their last tick's travel),
    // before drag, gravity and wind: the original's sleep, rustle and mean
    // all read this sum.
    let mut sum = Vec3::ZERO;
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
            sum += velocity;
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
            particle.previous = particle.position;
            particle.position = next;
        }
    }
    // The step itself runs only for an awake cloth; a sleeping one keeps
    // its countdown (never below its owner's) and lets the rustle fade.
    cloth.mean_velocity = sum * (1.0 / count as f32);
    if awake {
        settle_sleep(cloth, sum.length_squared());
    } else {
        cloth.sleep = cloth.sleep.max(cloth.owner_sleep);
    }
    settle_rustle(cloth, sum);
    if awake {
        apply_constraints(cloth, &[]);
    }
    // Whether the sector's object flags have the cloth awake this tick: set
    // at its start, and by a body that raises its countdown.
    let mut awake = awake;
    // The hand's grabbed vertex sits on its target after the tick's links,
    // and wakes the cloth. [Exarust's own wake: the original's grab call
    // writes no countdown; 2 is the least that outlasts the tick's end.]
    if let Some((index, target)) = cloth.grabbed
        && target.is_finite()
        && index < count
    {
        let particle = &mut cloth.particles[index];
        particle.previous = target;
        particle.position = target;
        cloth.sleep = cloth.sleep.max(2).max(cloth.owner_sleep);
        awake = true;
    }
    let before: Vec<Vec3> = cloth
        .particles
        .iter()
        .map(|particle| particle.position)
        .collect();
    // The body stage: each body's spheres push the cloth's triangles into
    // the push buffer, and a body that pushed trades countdowns with it.
    cloth.body_tests = 0;
    cloth.pushed_by.clear();
    cloth.pushes.resize(count, Vec3::ZERO);
    if !cloth.bodies.is_empty() {
        let bounds = crate::cloth_body::triangle_bounds(cloth);
        let bodies = std::mem::take(&mut cloth.bodies);
        for (index, body) in bodies.iter().enumerate() {
            let touch = crate::cloth_body::touch_body(
                &cloth.particles,
                &cloth.triangles,
                &bounds,
                body,
                &mut cloth.pushes,
            );
            cloth.body_tests += touch.tests;
            if touch.pushed {
                cloth.pushed_by.push(index);
                let (woke, _owed) =
                    crate::cloth_body::trade_countdowns(&mut cloth.sleep, body.countdown);
                awake |= woke;
            }
        }
        cloth.bodies = bodies;
    }
    // The awake cloth's pass after the body stage: the pushes, then the
    // remaining constraint passes holding the pushed vertices on their
    // planes.
    if awake {
        let planes = crate::cloth_body::apply_pushes(cloth);
        for _ in 0..crate::cloth_body::PUSH_PASSES {
            apply_constraints(cloth, &planes);
        }
    }
    // A level plane pushes like the level's meshes and, like them, does not
    // wake the cloth.
    for index in 0..cloth.planes.len() {
        let plane = cloth.planes[index];
        for particle in &mut cloth.particles {
            push_plane(particle, plane);
        }
    }
    // The level's meshes last, after the step, as the sector calls them.
    crate::cloth_level::touch_level(cloth);
    // The end of the sector's tick: an awake cloth whose countdown is under
    // 2 falls asleep where it is; a sleeping one stays where it is.
    if awake && cloth.sleep < 2 {
        cloth.sleep = 0;
        awake = false;
    }
    if !awake {
        for particle in &mut cloth.particles {
            particle.previous = particle.position;
        }
    }
    cloth.reaction = Vec3::ZERO;
    for (particle, old) in cloth.particles.iter().zip(before) {
        cloth.reaction -= particle.position - old;
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

/// One constraint pass: pins, then edges, then (after the body push) the
/// pushed vertices put back on their push planes.
fn apply_constraints(cloth: &mut Cloth, planes: &[Option<(Vec3, f32)>]) {
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
    crate::cloth_body::hold_planes(cloth, planes);
}

/// The countdown from the squared length of the velocity sum: under
/// `√(vertices × 10⁻⁶)` it counts down, over twice that squared it counts
/// up (a NaN does neither), and only a count that went up is capped at
/// [`SLEEP_CAP`] (a countdown a moving body handed over stays until the
/// cloth slows). It is never left below the owner's. A count may go under
/// zero: the tick's end puts a count under 2 to sleep.
fn settle_sleep(cloth: &mut Cloth, sum_sq: f32) {
    let threshold = (cloth.particles.len() as f32 * 1e-6).sqrt();
    if sum_sq < threshold {
        cloth.sleep -= 1;
    } else if sum_sq > (2.0 * threshold) * (2.0 * threshold) {
        cloth.sleep += 1;
        if cloth.sleep > SLEEP_CAP {
            cloth.sleep = SLEEP_CAP;
        }
    }
    if cloth.sleep < cloth.owner_sleep {
        cloth.sleep = cloth.owner_sleep;
    }
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
    fn a_cloth_sleeps_rustles_and_clears_a_plane() {
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
    fn a_falling_particle_is_swept_back_off_the_floor() {
        let floor = LevelTri {
            a: Vec3::new(-1.0, -1.0, 0.0),
            b: Vec3::new(20.0, -1.0, 0.0),
            c: Vec3::new(-1.0, 20.0, 0.0),
        };
        // The level's meshes come after the step: a particle 1 cm over the
        // floor, falling 7 cm a tick, is swept back to 2 cm over it and
        // slides 0.1 cm short (1.9 cm), and the push wakes nothing.
        let mut cloth = Cloth::new(vec![Particle {
            previous: Vec3::new(2.0, 2.0, 8.0),
            position: Vec3::new(2.0, 2.0, 1.0),
        }]);
        cloth.level.push(floor);
        step(&mut cloth, Vec3::ZERO);
        assert!(
            (cloth.particles[0].position.z - 1.9).abs() < 1e-3,
            "{:?}",
            cloth.particles[0].position
        );
        let mut beside = Cloth::new(vec![Particle {
            previous: Vec3::new(100.0, 2.0, 8.0),
            position: Vec3::new(100.0, 2.0, 1.0),
        }]);
        beside.level.push(floor);
        step(&mut beside, Vec3::ZERO);
        assert!(beside.particles[0].position.z < 0.0);
    }

    #[test]
    fn a_mesh_builds_edges_and_the_wind_tile_blows() {
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
        // A large cloth vertex at the origin (radius 10) and a unit sphere
        // moving 2 cm along z with its radius growing by 1 over the tick.
        let particles = [SweptProbe {
            position: Vec3::ZERO,
            radius: 10.0,
            velocity: Vec3::ZERO,
            radius_rate: 0.0,
        }];
        let sphere = SweptProbe {
            position: Vec3::new(0.0, 1.0, -0.5),
            radius: 1.0,
            velocity: Vec3::new(0.0, 0.0, 2.0),
            radius_rate: 1.0,
        };
        // The separation is (0, -1, 0.5): squared 1.25, within the 9 cm
        // reach. The rate squared is 1, so the first test misses. The speed
        // squared is 4, the dot 1 and the left side (1.25 - 1) * 4 = 1 is
        // exactly dot squared; the slack is 2 - 1 = 1 and the remainder 0
        // is within 1 * 1.
        assert_eq!(swept_hit(&particles, &sphere, 0, 1), Some(0));
        // 1.5 cm farther back along the line the closest approach (1 cm) is
        // the same, but it lies a whole tick behind the start: the dot is 4, the
        // slack 2 - 4 = -2 and the remainder 0 is not within -2 * 2.
        let behind = SweptProbe {
            position: Vec3::new(0.0, 1.0, -2.0),
            ..sphere
        };
        assert_eq!(swept_hit(&particles, &behind, 0, 1), None);
        // Three cm off the line the sphere's growth (1 cm) cannot reach: the
        // left side (9.25 - 1) * 4 = 33 exceeds the dot squared, 1.
        let wide = SweptProbe {
            position: Vec3::new(0.0, 3.0, -0.5),
            ..sphere
        };
        assert_eq!(swept_hit(&particles, &wide, 0, 1), None);
        // A sphere resting inside the particle is a hit at once (its squared
        // distance is within the rate squared, here 0).
        let inside = SweptProbe {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            radius_rate: 0.0,
            ..sphere
        };
        assert_eq!(swept_hit(&particles, &inside, 0, 1), Some(0));
        // One far outside the 9 cm reach is not looked at.
        let far = SweptProbe {
            position: Vec3::new(200.0, 0.0, 0.0),
            ..sphere
        };
        assert_eq!(swept_hit(&particles, &far, 0, 1), None);
        // The index range runs from `from` up to `end`, which the particle
        // count caps.
        let pair = [particles[0], particles[0]];
        assert_eq!(swept_hit(&pair, &inside, 0, 2), Some(0));
        assert_eq!(swept_hit(&pair, &inside, 1, 2), Some(1));
        assert_eq!(swept_hit(&pair, &inside, 2, 3), None);
        // A position that is not a number passes the "above" tests the way
        // the original's compares do: here it is a hit.
        let lost = SweptProbe {
            position: Vec3::NAN,
            ..sphere
        };
        assert_eq!(swept_hit(&particles, &lost, 0, 1), Some(0));
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
        assert_eq!(first, (f64::from(0x0808_8406u32) / 4_294_967_296.0) as f32);
        assert!((0.0..1.0).contains(&first));
    }

    #[test]
    fn the_owner_move_blends_toward_the_target() {
        let base = [Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)];
        let earlier = [Vec3::new(0.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 0.0)];
        let mut out = [Vec3::splat(-1.0); 2];
        // d = (1,0,0) and (0,0,6); target 0, k 0.5, weight 0.5, offset 1:
        // ((d - d/2) * 0.5) + 1 + base.
        let sum = pull_toward(
            &mut out,
            &base,
            &earlier,
            Vec3::ZERO,
            Vec3::splat(1.0),
            0.5,
            0.5,
        );
        assert_eq!(sum, Vec3::new(1.0, 0.0, 6.0));
        assert_eq!(out, [Vec3::new(2.25, 3.0, 4.0), Vec3::new(5.0, 6.0, 8.5)]);
        // The inputs are left as they were.
        assert_eq!(base[0], Vec3::new(1.0, 2.0, 3.0));
    }

    /// A grabbed vertex is put on the hand's target after the tick's
    /// links and wakes the cloth; letting go leaves it to gravity.
    #[test]
    fn a_grabbed_vertex_follows_the_hand() {
        let mut cloth = Cloth::new(vec![still(Vec3::ZERO), still(Vec3::new(10.0, 0.0, 0.0))]);
        cloth.sleep = 0;
        let target = Vec3::new(0.0, 0.0, 50.0);
        cloth.grab(0, target);
        assert!(cloth.sleep > 0);
        step(&mut cloth, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(cloth.particles[0].position, target);
        cloth.release_grab();
        let at = cloth.particles[0].position;
        step(&mut cloth, Vec3::new(0.0, 0.0, -1.0));
        assert!(cloth.particles[0].position.z < at.z);
        // A grab on a vertex the cloth does not have is ignored.
        cloth.grab(9, target);
        assert!(cloth.grabbed.is_none());
    }

    /// The countdown reads the summed velocity, not the tick's step: a
    /// particle held on a plane under gravity has no velocity left after
    /// the push, so it counts down and sleeps, though gravity moves it
    /// every tick.
    #[test]
    fn a_cloth_held_against_gravity_sleeps() {
        let mut cloth = Cloth::new(vec![still(Vec3::new(0.0, 2.0, 0.0))]);
        cloth.planes.push(LevelPlane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        });
        let gravity = gravity_cm(1.0);
        step(&mut cloth, gravity);
        assert_eq!(cloth.sleep, SLEEP_CAP - 1);
        for _ in 0..SLEEP_CAP {
            step(&mut cloth, gravity);
        }
        assert_eq!(cloth.sleep, 0);
        assert!((cloth.particles[0].position.y - PADDING_CM).abs() < 1e-4);
        // A velocity sum over twice the threshold, squared, counts up: one
        // particle (threshold 0.001) moving 0.05 cm a tick (0.0025 cm²).
        let mut moving = Cloth::new(vec![Particle {
            position: Vec3::new(0.05, 0.0, 0.0),
            previous: Vec3::ZERO,
        }]);
        moving.sleep = 10;
        step(&mut moving, Vec3::ZERO);
        assert_eq!(moving.sleep, 11);
        assert!(
            moving
                .mean_velocity
                .abs_diff_eq(Vec3::new(0.05, 0.0, 0.0), 1e-7)
        );
    }

    /// The pin list reads the face flags: an element is pinned only when
    /// every face about it is marked, firmly when each mark has a firm bit,
    /// and an element no face touches is pinned firmly.
    #[test]
    fn face_flags_pin_the_hem_and_free_the_rest() {
        // Two triangles over 0-1-2 and 1-3-2, and element 4 on no face.
        let faces = [[0, 1, 2], [1, 3, 2]];
        assert_eq!(
            face_flag_pins(5, &faces, &[0x8000_0000, 0]),
            vec![(0, FIRM_PIN), (4, FIRM_PIN)]
        );
        assert_eq!(
            face_flag_pins(5, &faces, &[0x8000_0000, 0x1000_0000]),
            vec![
                (0, FIRM_PIN),
                (1, LOOSE_PIN),
                (2, LOOSE_PIN),
                (3, LOOSE_PIN),
                (4, FIRM_PIN)
            ]
        );
        // No flags at all: every element on a face hangs free.
        assert_eq!(face_flag_pins(4, &faces, &[]), Vec::new());
        // A corner past the elements is ignored.
        assert_eq!(
            face_flag_pins(2, &[[0, 1, 9]], &[0x4000_0000]),
            vec![(0, FIRM_PIN), (1, FIRM_PIN)]
        );
    }

    /// A banner held firmly by its hem and hanging free against a sleeping
    /// body's spheres settles on them and falls asleep: a sleeping body
    /// pushes but never keeps the cloth awake, and with only the hem pinned
    /// there is a rest the pushes hold. (Every vertex held at 0.1 toward a
    /// rest pose these spheres cut into, the old guess, never slept: the
    /// pins pulled the cloth back in each pass and the pushes threw it out,
    /// with a velocity sum² near 8.)
    #[test]
    fn a_hem_pinned_banner_against_a_sleeping_sphere_sleeps() {
        let mut cloth = hanging_banner();
        let standing = |centre: Vec3, radius: f32| crate::cloth_body::BodySweep {
            from: centre,
            to: centre,
            radius: radius + PADDING_CM,
            moving: false,
        };
        cloth.bodies = vec![crate::cloth_body::ClothBody {
            spheres: vec![
                standing(Vec3::new(5.0, -30.0, -3.0), 9.0),
                standing(Vec3::new(20.0, -40.0, 2.0), 12.0),
            ],
            countdown: 0,
            part: 0,
        }];
        let gravity = gravity_cm(1.0);
        let mut slept = None;
        for tick in 0..3000 {
            step(&mut cloth, gravity);
            if cloth.sleep == 0 {
                slept = Some(tick);
                break;
            }
        }
        assert!(slept.is_some(), "the banner never slept: {}", cloth.sleep);
        // The hem stayed near where it was put (the links after the pins
        // pull it a little each pass).
        for pin in &cloth.pins {
            let off = cloth.particles[pin.index].position.distance(pin.rest);
            assert!(off < 1.0, "the hem moved {off} cm");
        }
    }

    /// A 5 × 5 banner of 10 cm squares hanging in the x–y plane below
    /// y = 0, its top row of faces marked as pinned.
    fn hanging_banner() -> Cloth {
        let side = 5;
        let mut vertices = Vec::new();
        for row in 0..side {
            for column in 0..side {
                vertices.push(Vec3::new(column as f32 * 10.0, -(row as f32) * 10.0, 0.0));
            }
        }
        let mut faces = Vec::new();
        let mut flags = Vec::new();
        for row in 0..side - 1 {
            for column in 0..side - 1 {
                let at = (row * side + column) as u32;
                let s = side as u32;
                let mark = if row == 0 { 0x8000_0000 } else { 0 };
                faces.push([at, at + s, at + 1]);
                faces.push([at + 1, at + s, at + s + 1]);
                flags.extend([mark, mark]);
            }
        }
        let pins = face_flag_pins(vertices.len(), &faces, &flags);
        assert_eq!(pins.len(), side, "only the top row is pinned");
        let triangles: Vec<[usize; 3]> = faces
            .iter()
            .map(|f| [f[0] as usize, f[1] as usize, f[2] as usize])
            .collect();
        let pinned: Vec<usize> = pins.iter().map(|(i, _)| *i).collect();
        let mut cloth = Cloth::from_mesh(&vertices, &triangles, &pinned);
        for (pin, (_, weight)) in cloth.pins.iter_mut().zip(&pins) {
            pin.weight = *weight;
        }
        cloth
    }
}
