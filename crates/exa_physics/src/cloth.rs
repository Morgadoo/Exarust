//! A cloth step: Verlet particles, a wind nudge, pins and edge constraints.
//!
//! Each free particle keeps `0.98977 × 0.5^(1/83)` of its velocity, dragged
//! 5% toward last tick's mean, then gains gravity. Gravity is the body's
//! acceleration, in centimetres per tick. The world's wind ([`Wind`]) above
//! 0.001 moves a particle by `direction · ((0.25 + noise) · strength)`. A
//! pin moves toward its rest point
//! by its weight. A link `(a, b)` with `d = b − a` moves `a` by `f·d` and
//! `b` by `−f·d`, `f = 0.5 − rsqrt(|d|²)·h`, where `h` is half the rest
//! distance and `rsqrt` the processor's approximate reciprocal square root:
//! the full correction to the rest length. The links come from a walk out
//! from the pins over the mesh ([`walk_links`]). The shipped
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
//! sum's squared length exceeds 400, for a record that names a rustle
//! sound. The mesh vertices are the particle positions. These are the built-in record's numbers; a cloth's own record
//! ([`crate::cloth_record`]) sets its drag, retention, gravity scale,
//! padding, slide and passes, and its particle mode (the hair's) makes all
//! its passes in the step and takes bodies particle by particle
//! ([`crate::cloth_body::touch_particles`]).
//!
//! Evidence: the cloth step notes (retention, drag, wind, pins, edges, sleep,
//! rustle, plane pushes), the cloth's integrate slot, its post-body push
//! pass and its end-of-tick slot (REA assembly); the bodies are
//! [`crate::cloth_body`]'s and the level's meshes [`crate::cloth_level`]'s,
//! after the step.

use bevy::math::Vec3;
use bevy::render::render_resource::Face;

use crate::body::BODY_HZ;

/// How much of the velocity a particle of the built-in record's cloth
/// keeps a tick: fast retention times the per-second retention, taken to
/// one tick ([`crate::cloth_record::ClothRecord::retention`]).
pub fn retention() -> f32 {
    crate::cloth_record::ClothRecord::LEVEL.retention()
}

/// Drag toward the cloth's mean velocity (the built-in record's).
pub const MEAN_DRAG: f32 = 0.05;
/// Wind at or below this does not move the cloth (compared in double).
pub const WIND_FLOOR: f64 = 0.001;
/// Collision padding, centimetres (the built-in record's).
pub const PADDING_CM: f32 = 2.0;
/// The share of a cloth contact handed back to the body: 0.015 for a body
/// of the cloth-colliding class, 0.01 when the cloth's own class flag is
/// set, else 0.002. The game does not hand it back yet
/// ([`crate::cloth_body`]).
pub const FRICTION: [f32; 3] = [0.002, 0.01, 0.015];
/// Sleep countdown does not count itself up past this (a body may hand it
/// a higher one).
pub const SLEEP_CAP: i32 = 32;

/// A hold without a pin to measure from keeps this radius (cm).
pub const GRAB_FREE_RADIUS: f32 = 5.0;

/// The pin search's starting distance (the engine's large float).
const GRAB_NO_DISTANCE2: f32 = f32::from_bits(0x7f7f_c99e);

/// The share of the way to its target a hold's smoothed point adds to its
/// motion each tick, and the share of that motion it then moves.
pub const GRAB_PULL: f32 = 0.1;
pub const GRAB_FOLLOW: f32 = 0.3;

/// The countdown the hand's drag wakes the cloth to.
pub const GRAB_WAKE: i32 = 32;

/// The engine's small float (`0x33ffffd7`).
const MOTION_FLOOR2: f32 = f32::from_bits(0x33ff_ffd7);

/// A hand's hold on the cloth (the original's grab record): the point at
/// `weights` over the particles `corners`, the hand's `target` for it (cm,
/// the cloth's frame), its `radius` from its `pin` (the nearest pinned
/// particle, measured in the mesh when the hold was made), and its smoothed
/// point now and a tick ago, which the constraint passes put the point on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrabRecord {
    pub corners: [usize; 3],
    pub weights: [f32; 3],
    pub target: Vec3,
    pub radius: f32,
    pub smoothed: Vec3,
    pub last: Vec3,
    pub pin: Option<usize>,
}
/// Rustle starts when the squared sum of the velocities exceeds this.
pub const RUSTLE_SPEED_SQ: f32 = 400.0;
/// The body's gravitational acceleration, metres per second squared.
const GRAVITY_MPS2: f32 = -9.806_65;

/// Gravity for one tick, in centimetres, scaled by the cloth's gravity scale.
pub fn gravity_cm(scale: f32) -> Vec3 {
    Vec3::new(0.0, GRAVITY_MPS2 * 100.0 * scale / (BODY_HZ * BODY_HZ), 0.0)
}

/// The wind direction as the game stores it before its first step,
/// `(0.6, 0, 0.3)`; each step normalises it in place.
pub const WIND_DIRECTION: Vec3 = Vec3::new(0.6, 0.0, 0.3);

/// One cloth particle. Positions are centimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    pub position: Vec3,
    pub previous: Vec3,
}

/// A link between two particles and half its rest distance, in
/// centimetres (the original stores the distance times 0.5, which its edge
/// pass multiplies by the reciprocal square root of the squared length).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub half_rest: f32,
}

impl Edge {
    /// The link from `a` to `b` at rest where `points` puts them: half the
    /// distance, summed `(x² + z²) + y²` before the root, times 0.5.
    pub fn at_rest(points: &[Vec3], a: usize, b: usize) -> Self {
        Self {
            a,
            b,
            half_rest: crate::engine_math::length3(points[a] - points[b]) * 0.5,
        }
    }
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
/// multiplied by 0.005 (in double) before the fractional lookup.
pub const NOISE_CELLS: usize = 16;

/// The world's wind as every cloth reads it: the strength and direction a
/// simulation step's hook sets, the frame the noise tile lies in, and the
/// tile of 256 noise samples the hook walks each step, with the engine
/// generator the walk draws from (its own stream here). One for the whole
/// simulation, as the original keeps it in globals.
#[derive(Debug, Clone, PartialEq)]
pub struct Wind {
    /// The scene's wind times 0.3: how far a cloth is blown, cm a tick.
    pub strength: f32,
    /// The wind's direction, normalised in place each step by the
    /// approximate reciprocal square root.
    pub direction: Vec3,
    /// The rows of the noise tile's frame: across, up and along the wind
    /// (its origin is the world's).
    pub frame: [Vec3; 3],
    /// The noise samples, row by row.
    pub tile: [f32; NOISE_CELLS * NOISE_CELLS],
    /// The generator the walk draws from ([`next_random`]).
    pub roll: u32,
}

impl Default for Wind {
    fn default() -> Self {
        Self::CALM
    }
}

impl Wind {
    /// The wind before the first step: no strength, the stored direction,
    /// an empty frame and a flat tile.
    pub const CALM: Self = Self {
        strength: 0.0,
        direction: WIND_DIRECTION,
        frame: [Vec3::ZERO; 3],
        tile: [0.0; NOISE_CELLS * NOISE_CELLS],
        roll: 0,
    };

    /// One simulation step's wind hook: the strength is the scene's wind
    /// times 0.3 (in double); every sample walks by a draw `r`, within
    /// −1…1 by `(r − 0.5)·s`, above 1 down by `s·0.5·r` and below −1 up by
    /// as much, with `s = strength·0.2·4` (in double); then the direction
    /// is normalised and the tile's frame rebuilt from it. Every sample
    /// draws, also with no wind.
    pub fn advance(&mut self, scene_wind: f32) {
        self.strength = (f64::from(scene_wind) * 0.3) as f32;
        let stride = (f64::from(self.strength) * 0.2 * 4.0) as f32;
        for sample in &mut self.tile {
            let value = *sample;
            // A NaN sample is neither above 1 nor below −1 (`comiss`), so
            // it takes the in-range walk.
            *sample = if value > 1.0 {
                let r = next_random(&mut self.roll);
                value - stride * 0.5 * r
            } else if value < -1.0 {
                let r = next_random(&mut self.roll);
                stride * 0.5 * r + value
            } else {
                let r = next_random(&mut self.roll);
                (r - 0.5) * stride + value
            };
        }
        self.direction = crate::engine_math::normalize_approx(self.direction);
        self.frame = wind_frame(self.direction);
    }

    /// A world point (cm) in the tile's frame: `(p − 0)` dotted with each
    /// row, summed `(x + y) + z`.
    pub fn in_frame(&self, point: Vec3) -> Vec3 {
        let row = |r: Vec3| (point.x * r.x + point.y * r.y) + point.z * r.z;
        Vec3::new(row(self.frame[0]), row(self.frame[1]), row(self.frame[2]))
    }

    /// The noise at a point of the tile's frame (its first two
    /// coordinates, cm), bilinear between the four samples about it and
    /// wrapping at the tile's edge, summed in the original's order.
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        let (ix, fx) = tile_cell(x);
        let (iy, fy) = tile_cell(y);
        // [Exarust: a cell index the rounding takes to 16 reads past the
        // tile in the original (into the frame after it); here 0.]
        let at = |i: i64| -> f32 {
            usize::try_from(i)
                .ok()
                .and_then(|i| self.tile.get(i))
                .copied()
                .unwrap_or(0.0)
        };
        let gx = 1.0 - fx;
        let gy = 1.0 - fy;
        let mut out;
        if ix < 15 {
            let cell = iy * 16 + ix;
            out = gy * (gx * at(cell));
            out += gy * (at(cell + 1) * fx);
            if iy == 15 {
                out += (gx * at(ix)) * fy;
                out += (at(ix + 1) * fx) * fy;
            } else {
                out += (gx * at(cell + 16)) * fy;
                out += (at(cell + 17) * fx) * fy;
            }
        } else {
            let row = iy * 16;
            out = gy * (gx * at(row + 15));
            out += gy * (at(row) * fx);
            if iy < 15 {
                out += (gx * at(row + 16 + 15)) * fy;
                out += (at(row + 16) * fx) * fy;
            } else {
                out += (gx * at(ix)) * fy;
                out += (at(0) * fx) * fy;
            }
        }
        out
    }
}

/// The tile's frame from the wind's direction: the third row the
/// direction, the second the world's up less its part along it, the first
/// their cross product, each of the two normalised by the approximate
/// reciprocal square root without a zero guard.
fn wind_frame(direction: Vec3) -> [Vec3; 3] {
    let unmasked = |v: Vec3| {
        let squared = (v.x * v.x + v.z * v.z) + (v.y * v.y + 0.0);
        v * crate::engine_math::reciprocal_sqrt(squared)
    };
    let up = unmasked(crate::engine_math::remove_component(Vec3::Y, direction));
    let across = unmasked(up.cross(direction));
    [across, up, direction]
}

/// One coordinate's cell and fraction: the fraction of `cm·0.005` (in
/// double, then single), lifted by one when negative, times 16; the cell is
/// its truncation and the fraction what is left.
fn tile_cell(cm: f32) -> (i64, f32) {
    let mut unit = exa_formats::text::fraction((f64::from(cm) * 0.005) as f32);
    if unit < 0.0 {
        unit += 1.0;
    }
    let scaled = unit * 16.0;
    // `cvttss2si` to 64 bits, of which the low word is kept.
    let cell = scaled as i64 as i32;
    (i64::from(cell), scaled - cell as f32)
}

/// The cloth a step advances.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cloth {
    pub particles: Vec<Particle>,
    pub edges: Vec<Edge>,
    pub pins: Vec<Pin>,
    /// Mean velocity from the previous tick, centimetres per tick.
    pub mean_velocity: Vec3,
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
    /// Equal and opposite sum of this tick's collision pushes, for the body.
    pub reaction: Vec3,
    /// The shipped cloth draws both faces.
    pub double_sided: bool,
    /// Distance along the mesh from the nearest pin, in centimetres.
    /// Unreachable vertices stay infinite. With no pins, vertex 0 is the source.
    pub distances: Vec<f32>,
    /// Whether the rustle loop plays this tick, and at what volume and
    /// pitch (the integrate slot's rustle).
    pub rustle: bool,
    pub rustle_volume: f32,
    pub rustle_pitch: f32,
    /// The rustle's energy tail, which decays by 0.9 each awake tick.
    pub rustle_tail: f32,
    /// The hands' holds on the cloth, oldest first ([`GrabRecord`]).
    pub grabs: Vec<GrabRecord>,
    /// Each particle's place in the mesh the cloth was built from (cm, its
    /// frame): where a grab's distance to the nearest pin is measured. Empty
    /// for a cloth built from bare particles, which measures from where its
    /// particles are.
    pub rest: Vec<Vec3>,
    /// The parameter record the cloth was set up from
    /// ([`crate::cloth_record`]): its drag, retention, gravity scale,
    /// padding, slide, pass count and constraint mode.
    pub record: crate::cloth_record::ClothRecord,
    /// The bodies a particle-mode cloth takes this tick, in the sector's
    /// order ([`crate::cloth_body::touch_particles`]); a triangle-mode cloth
    /// takes [`Cloth::bodies`] instead.
    pub particle_bodies: Vec<crate::cloth_body::ParticleBody>,
}

impl Cloth {
    pub fn new(particles: Vec<Particle>) -> Self {
        Self {
            particles,
            edges: Vec::new(),
            pins: Vec::new(),
            mean_velocity: Vec3::ZERO,
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
            reaction: Vec3::ZERO,
            double_sided: true,
            distances: Vec::new(),
            rustle: false,
            rustle_volume: 0.0,
            rustle_pitch: 0.0,
            rustle_tail: 0.0,
            grabs: Vec::new(),
            rest: Vec::new(),
            record: crate::cloth_record::ClothRecord::LEVEL,
            particle_bodies: Vec::new(),
        }
    }

    /// Adds a hold on the cloth (the original's grab record, made when the
    /// hand takes the cloth): the point at `weights` over the particles
    /// `corners` (a triangle's), pulled toward `target` (cm, the cloth's
    /// frame). Its radius is the distance from the point's place in the
    /// mesh (`((r₀·w₀) + r₁·w₁) + r₂·w₂`) to the nearest pin's (the first
    /// of equals; squared distances `(dx² + dy²) + dz²`): its square root
    /// when over the engine's small float, 0 when not (or unordered), and
    /// [`GRAB_FREE_RADIUS`] with no pin. Its smoothed point starts at the
    /// target. Returns the record's index; a corner the cloth does not have
    /// adds nothing.
    pub fn add_grab(&mut self, corners: [usize; 3], weights: [f32; 3], target: Vec3) -> Option<usize> {
        let count = self.particles.len();
        if corners.iter().any(|&c| c >= count) {
            return None;
        }
        let rest = |i: usize| {
            self.rest
                .get(i)
                .copied()
                .unwrap_or(self.particles[i].position)
        };
        let point = rest(corners[0]) * weights[0]
            + rest(corners[1]) * weights[1]
            + rest(corners[2]) * weights[2];
        let mut best = GRAB_NO_DISTANCE2;
        let mut pin = None;
        for p in &self.pins {
            if p.index >= count {
                continue;
            }
            let d = point - rest(p.index);
            let d2 = (d.x * d.x + d.y * d.y) + d.z * d.z;
            if d2 < best {
                best = d2;
                pin = Some(p.index);
            }
        }
        let radius = match pin {
            None => GRAB_FREE_RADIUS,
            Some(_) if best > MOTION_FLOOR2 => best.sqrt(),
            Some(_) => 0.0,
        };
        self.grabs.push(GrabRecord {
            corners,
            weights,
            target,
            radius,
            smoothed: target,
            last: target,
            pin,
        });
        Some(self.grabs.len() - 1)
    }

    /// The hand's drag (the original's drag handler, on the cloth's side):
    /// the newest hold takes `at` as its target (cm, the cloth's frame) and
    /// the cloth is woken to [`GRAB_WAKE`]. The hand that took vertex
    /// `index` without a hold yet makes one there (all its weight on the
    /// vertex) first [Exarust's hand picks a vertex, not a point on a
    /// triangle]. A vertex the cloth does not have is ignored.
    pub fn grab(&mut self, index: usize, at: Vec3) {
        if index >= self.particles.len() {
            return;
        }
        if self.grabs.is_empty() && self.add_grab([index; 3], [1.0, 0.0, 0.0], at).is_none() {
            return;
        }
        if let Some(last) = self.grabs.last_mut() {
            last.target = at;
        }
        self.wake_to(GRAB_WAKE);
    }

    /// The hand lets go: the newest hold is dropped.
    pub fn release_grab(&mut self) {
        self.grabs.pop();
    }

    /// Raises the countdown to `count` when it is higher (the cloth's wake
    /// slot), never below the owner's.
    pub fn wake_to(&mut self, count: i32) {
        if self.sleep < count {
            self.sleep = count;
        }
    }

    /// Build a cloth from a triangle mesh. Each vertex is a particle, and
    /// the links are the walk out from the pins ([`walk_links`]) over the
    /// mesh's corner fans in the original's incidence order
    /// ([`incidence`]), each at half its rest distance. Pinned vertices are
    /// held at their current point. The draw is double sided.
    pub fn from_mesh(vertices: &[Vec3], triangles: &[[usize; 3]], pinned: &[usize]) -> Self {
        let kept: Vec<[usize; 3]> = triangles
            .iter()
            .copied()
            .filter(|tri| tri.iter().all(|&index| index < vertices.len()))
            .collect();
        let (starts, fans) = incidence(&kept, vertices.len());
        let links = walk_links(vertices, &kept, &starts, &fans, pinned);
        Self::from_mesh_linked(vertices, &kept, pinned, &links)
    }

    /// Build a cloth from a triangle mesh with the links already walked
    /// (each pair at half its rest distance where `vertices` puts it); a
    /// link naming a missing vertex is left out.
    pub fn from_mesh_linked(
        vertices: &[Vec3],
        triangles: &[[usize; 3]],
        pinned: &[usize],
        links: &[(usize, usize)],
    ) -> Self {
        let particles = vertices
            .iter()
            .copied()
            .map(|position| Particle {
                position,
                previous: position,
            })
            .collect();
        let mut cloth = Self::new(particles);
        for tri in triangles {
            if tri.iter().any(|index| *index >= vertices.len()) {
                continue;
            }
            cloth.triangles.push(ClothTri {
                a: tri[0],
                b: tri[1],
                c: tri[2],
            });
        }
        cloth.edges = links
            .iter()
            .filter(|&&(a, b)| a < vertices.len() && b < vertices.len())
            .map(|&(a, b)| Edge::at_rest(vertices, a, b))
            .collect();
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
        cloth.rest = vertices.to_vec();
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
    // The original steps the simulated object and then runs its level
    // contact, so a caller that gathers the level's meshes after the step
    // ([`step_before_level`] and [`finish_step`]) sees the tick's moves.
    let Some((before, awake)) = step_before_level(cloth, gravity) else {
        return;
    };
    crate::cloth_level::touch_level(cloth);
    finish_step(cloth, &before, awake);
}

/// One tick up to, but not including, the level's meshes: everything
/// [`step`] does before its level contact, returning the particles'
/// positions where the body stage began and whether the cloth was awake
/// by then, which [`finish_step`] needs after the level contact. None when
/// the tick changes nothing (non-finite input or no particles), as
/// [`step`] then skips its level contact too.
pub fn step_before_level(cloth: &mut Cloth, gravity: Vec3) -> Option<(Vec<Vec3>, bool)> {
    step_before_level_in(cloth, gravity, None)
}

/// The world's wind as one cloth meets it: the wind, and the map from the
/// cloth's own frame to the world's (both in centimetres), whose inverse
/// turns the wind's direction into the cloth's frame. The original's
/// particles are in the world already.
#[derive(Debug, Clone, Copy)]
pub struct WindAt<'a> {
    pub wind: &'a Wind,
    pub to_world: bevy::math::Affine3A,
}

/// [`step_before_level`] in the world's wind: after the integrate and the
/// countdown, while the wind is above [`WIND_FLOOR`] each particle moves
/// along the wind by `(0.25 + n)·strength`, with `n` the tile's noise where
/// the particle was a tick ago. The three parts are [`integrate`],
/// [`touch_listed_bodies`] and [`after_bodies`]; the world's live stage runs
/// the body stage between them inside the bodies' solver instead.
pub fn step_before_level_in(
    cloth: &mut Cloth,
    gravity: Vec3,
    wind: Option<WindAt>,
) -> Option<(Vec<Vec3>, bool)> {
    let Integrated { before, mut awake } = integrate(cloth, gravity, wind)?;
    touch_listed_bodies(cloth, &mut awake);
    after_bodies(cloth, awake);
    Some((before, awake))
}

/// What the cloth's own step leaves for its body stage: the particles where
/// the body stage begins (the reaction is measured from them) and whether
/// the cloth is awake by then.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Integrated {
    pub before: Vec<Vec3>,
    pub awake: bool,
}

/// The cloth's own step, the original's integrate slot (the sector's object
/// step, before its body contacts): integrate, countdown, wind, rustle,
/// the step's constraint passes and the hand's grab; then the body stage's
/// buffers are cleared. None when the tick changes nothing (non-finite
/// input or no particles).
pub fn integrate(cloth: &mut Cloth, gravity: Vec3, wind: Option<WindAt>) -> Option<Integrated> {
    if !gravity.is_finite() || !cloth.mean_velocity.is_finite() {
        return None;
    }
    let count = cloth.particles.len();
    if count == 0 {
        return None;
    }
    // The sum of the particles' velocities (their last tick's travel),
    // before drag, gravity and wind: the original's sleep, rustle and mean
    // all read this sum.
    let mut sum = Vec3::ZERO;
    let awake = cloth.sleep > 0;
    if awake {
        let mean = cloth.mean_velocity;
        let keep = cloth.record.retention();
        let drag = cloth.record.mean_drag;
        let gravity = cloth.record.gravity(gravity);
        for particle in &mut cloth.particles {
            if !particle.position.is_finite() || !particle.previous.is_finite() {
                continue;
            }
            let velocity = particle.position - particle.previous;
            sum += velocity;
            // `((v + (v̄ − v)·drag)·keep + g) + x`, the integrate loop's
            // order.
            let retained = (velocity + (mean - velocity) * drag) * keep;
            let next = (retained + gravity) + particle.position;
            particle.previous = particle.position;
            particle.position = next;
        }
    }
    // The step itself runs only for an awake cloth; a sleeping one keeps
    // its countdown (never below its owner's).
    cloth.mean_velocity = sum * (1.0 / count as f32);
    if awake {
        settle_sleep(cloth, sum.length_squared());
    } else {
        cloth.sleep = cloth.sleep.max(cloth.owner_sleep);
    }
    if awake
        && let Some(at) = wind
        && f64::from(at.wind.strength) > WIND_FLOOR
    {
        let direction = at.to_world.inverse().transform_vector3(at.wind.direction);
        for particle in &mut cloth.particles {
            let place = at
                .wind
                .in_frame(at.to_world.transform_point3(particle.previous));
            let push = (0.25 + at.wind.sample(place.x, place.y)) * at.wind.strength;
            particle.position += direction * push;
        }
    }
    // The rustle is the integrate slot's, which only an awake cloth runs.
    if awake {
        settle_rustle(cloth, sum);
    }
    if awake {
        // The holds' smoothed points move toward their targets, then the
        // triangle mode makes one constraint pass here, the particle mode
        // all.
        smooth_grabs(cloth);
        for _ in 0..cloth.record.step_passes() {
            apply_constraints(cloth, &[]);
        }
    }
    // Whether the sector's object flags have the cloth awake this tick: set
    // at its start, and by a body that raises its countdown.
    let before: Vec<Vec3> = cloth
        .particles
        .iter()
        .map(|particle| particle.position)
        .collect();
    cloth.body_tests = 0;
    cloth.pushed_by.clear();
    cloth.pushes.resize(count, Vec3::ZERO);
    Some(Integrated { before, awake })
}

/// The body stage over the bodies handed to the cloth beforehand
/// ([`Cloth::bodies`], [`Cloth::particle_bodies`]), with no body side: each
/// body's spheres push the cloth's triangles into the push buffer (or the
/// particle mode's particles directly), and a body that pushed trades
/// countdowns with it, setting `awake` when the cloth's countdown rose. The
/// live stage, which also pushes the bodies back, is the world's.
pub fn touch_listed_bodies(cloth: &mut Cloth, awake: &mut bool) {
    let count = cloth.particles.len();
    cloth.pushes.resize(count, Vec3::ZERO);
    let particle_mode = cloth.record.particle_mode();
    if particle_mode && !cloth.particle_bodies.is_empty() {
        // The particle mode pushes the particles straight off each body's
        // spheres, against the box about the particles after the step.
        let cloth_box = bounds(&cloth.particles);
        let bodies = std::mem::take(&mut cloth.particle_bodies);
        for (index, body) in bodies.iter().enumerate() {
            if crate::cloth_body::touch_particles(&mut cloth.particles, cloth_box, body) {
                cloth.pushed_by.push(index);
                if cloth.sleep < body.countdown {
                    cloth.sleep = body.countdown;
                    *awake = true;
                }
            }
        }
        cloth.particle_bodies = bodies;
    }
    if !particle_mode && !cloth.bodies.is_empty() {
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
                *awake |= woke;
            }
        }
        cloth.bodies = bodies;
    }
}

/// The cloth's pass after its body stage (the original's after-collisions
/// slot): an awake triangle-mode cloth applies its push buffer, then the
/// remaining constraint passes hold the pushed vertices on their planes;
/// then the level planes push it (they never wake it).
pub fn after_bodies(cloth: &mut Cloth, awake: bool) {
    // The awake cloth's pass after the body stage: the pushes, then the
    // remaining constraint passes holding the pushed vertices on their
    // planes.
    // (The particle mode's post-body pass returns at once.)
    if awake && !cloth.record.particle_mode() {
        let planes = crate::cloth_body::apply_pushes(cloth);
        for _ in 0..cloth.record.post_body_passes() {
            apply_constraints(cloth, &planes);
        }
    }
    // A level plane pushes like the level's meshes and, like them, does not
    // wake the cloth.
    for index in 0..cloth.planes.len() {
        let plane = cloth.planes[index];
        let padding = cloth.record.padding_cm;
        for particle in &mut cloth.particles {
            push_plane(particle, plane, padding);
        }
    }
}

/// The end of the tick, after the level's meshes have pushed the cloth
/// ([`step_before_level`]): an awake cloth whose countdown is under 2 falls
/// asleep where it is, a sleeping one stays where it is, and the reaction
/// is the opposite of the particles' moves since `before` (the level's
/// pushes included, as the original reads its reaction after its
/// contacts). `awake` is the cloth's state at the body stage.
pub fn finish_step(cloth: &mut Cloth, before: &[Vec3], awake: bool) {
    let mut awake = awake;
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
        cloth.reaction -= particle.position - *old;
    }
}

/// The draw update's tangent rebuild for a normal-mapped first part (the
/// original's, after the normals are rebuilt): each face's tangent from its
/// corners' places `positions` and texture coordinates `uvs`,
/// `t = (Δv₂·e₁ − Δv₁·e₂) · (1 / (Δu₁·Δv₂ − Δu₂·Δv₁))` with `e₁ = p₁ − p₀`,
/// `e₂ = p₂ − p₀` and `Δ` the coordinates' differences from corner 0 (an
/// exact reciprocal, no floor: a degenerate face gives infinities as the
/// original's), summed unweighted into its three vertices; then each
/// vertex's sum loses its part along its normal (`t + n·(−((tₓnₓ + t_yn_y)
/// + t_zn_z))`) and is scaled by the processor's approximate reciprocal
/// square root (zero stays zero), into the first three lanes of
/// `tangents`. The fourth lane (the handedness sign the mesh was loaded
/// with, Bevy's tangent sign) is left as it is: the original writes only
/// three. Faces naming a missing vertex, and vertices past any of the
/// arrays, are skipped.
pub fn rebuild_tangents(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    indices: &[u32],
    tangents: &mut [[f32; 4]],
) {
    let count = positions.len().min(uvs.len());
    let mut sums = vec![Vec3::ZERO; positions.len()];
    for face in indices.chunks_exact(3) {
        let [a, b, c] = [face[0] as usize, face[1] as usize, face[2] as usize];
        if a >= count || b >= count || c >= count {
            continue;
        }
        let p = |i: usize| Vec3::from_array(positions[i]);
        let e1 = p(b) - p(a);
        let e2 = p(c) - p(a);
        let (u0, u1, u2) = (uvs[a], uvs[b], uvs[c]);
        let du1 = u1[0] - u0[0];
        let dv1 = u1[1] - u0[1];
        let du2 = u2[0] - u0[0];
        let dv2 = u2[1] - u0[1];
        let r = 1.0 / (du1 * dv2 - du2 * dv1);
        let t = Vec3::new(
            (dv2 * e1.x - dv1 * e2.x) * r,
            (dv2 * e1.y - dv1 * e2.y) * r,
            (dv2 * e1.z - dv1 * e2.z) * r,
        );
        sums[a] += t;
        sums[b] += t;
        sums[c] += t;
    }
    for ((slot, t), n) in tangents.iter_mut().zip(&sums).zip(normals) {
        let n = Vec3::from_array(*n);
        let along = (t.x * n.x + t.y * n.y) + t.z * n.z;
        let across = *t + n * -along;
        let unit = crate::engine_math::normalize_approx(across);
        slot[0] = unit.x;
        slot[1] = unit.y;
        slot[2] = unit.z;
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

#[cfg(test)]
fn ordered(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

/// The corners of every vertex, vertex by vertex, in the order the
/// original's mesh incidence lists them when no smoothing split reorders
/// them: faces last first and each face's corners last first. Returns the
/// prefix bounds (`fans[starts[v]..starts[v + 1]]` are vertex `v`'s) and
/// the `(face, corner)` list. A corner naming a vertex past `count` is
/// left out.
pub fn incidence(faces: &[[usize; 3]], count: usize) -> (Vec<u32>, Vec<(u32, u8)>) {
    let mut starts = vec![0u32; count + 1];
    for face in faces {
        for &vertex in face {
            if vertex < count {
                starts[vertex + 1] += 1;
            }
        }
    }
    for vertex in 0..count {
        starts[vertex + 1] += starts[vertex];
    }
    let mut fill = starts.clone();
    let mut fans = vec![(0u32, 0u8); starts[count] as usize];
    for (index, face) in faces.iter().enumerate().rev() {
        for corner in (0..3).rev() {
            let vertex = face[corner];
            if vertex < count {
                fans[fill[vertex] as usize] = (index as u32, corner as u8);
                fill[vertex] += 1;
            }
        }
    }
    (starts, fans)
}

/// From a face corner, the corner whose vertex the walk calls the first
/// neighbour (the original's table `2, 0, 1`).
const FIRST_NEIGHBOUR: [usize; 3] = [2, 0, 1];
/// From a face corner, the other neighbour's corner (`1, 2, 0`).
const SECOND_NEIGHBOUR: [usize; 3] = [1, 2, 0];

/// A vertex the walk settles has this state; one still unmet has −1 and
/// one waiting has its slot in the open list.
const SETTLED: i32 = -2;

/// One neighbour of the vertex the walk settles: the vertex, and the
/// neighbours on either side of it around the settled vertex (−1 when the
/// fan has none there).
#[derive(Debug, Clone, Copy)]
struct Neighbour {
    vertex: usize,
    before: i64,
    after: i64,
}

impl Neighbour {
    /// Sets a side of `of`'s entry, adding the entry (with the other side
    /// −1) when the list does not have it yet.
    fn set(list: &mut Vec<Self>, of: usize, before: Option<usize>, after: Option<usize>) {
        if let Some(entry) = list.iter_mut().find(|entry| entry.vertex == of) {
            if let Some(before) = before {
                entry.before = before as i64;
            }
            if let Some(after) = after {
                entry.after = after as i64;
            }
        } else {
            list.push(Self {
                vertex: of,
                before: before.map_or(-1, |v| v as i64),
                after: after.map_or(-1, |v| v as i64),
            });
        }
    }
}

/// `(x² + y²) + z²` of `a − b`, the engine's squared distance.
fn squared_distance(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    (d.x * d.x + d.y * d.y) + d.z * d.z
}

/// The cloth's links as the original's set-up walks them out from the pins
/// (`pins`, in order; vertex 0 when there are none), over each vertex's
/// corner fan (`fans[starts[v]..starts[v + 1]]`, `(face, corner)` pairs
/// into `faces`). Returns them in the order the edge pass visits them: the
/// walk's links from each settled vertex to its unsettled neighbours, then
/// the neighbour pairs nearly in line through the settled vertex (the
/// cosine of their arms above 0.7 in double precision), then the other
/// pairs.
///
/// The walk takes the open vertex whose distance is below the first open
/// one's, the last such in the list (not the nearest: the original compares
/// each with the first entry only); the taken slot is filled from the end.
/// A taken vertex lists its fan's neighbours, leaving out the far end of a
/// face's longest edge from it (a quad's diagonal), and links to each
/// neighbour not yet settled, opening it at its distance along the link
/// (`(x² + z²) + y²` under the root) or lowering its distance. A pair of
/// neighbours is linked unless a settled vertex between them that the fan
/// does not list, or a face about the first that joins the second to a
/// settled vertex, shows the pair was linked from there. A vertex no pin
/// reaches gets no link.
pub fn walk_links(
    positions: &[Vec3],
    faces: &[[usize; 3]],
    starts: &[u32],
    fans: &[(u32, u8)],
    pins: &[usize],
) -> Vec<(usize, usize)> {
    let count = positions.len();
    let mut tree = Vec::new();
    let mut bent = Vec::new();
    let mut straight = Vec::new();
    if count == 0 {
        return tree;
    }
    let fan = |vertex: usize| -> &[(u32, u8)] {
        match (starts.get(vertex), starts.get(vertex + 1)) {
            (Some(&a), Some(&b)) => fans.get(a as usize..b as usize).unwrap_or_default(),
            _ => &[],
        }
    };
    // The two neighbours of a vertex in one of its corners.
    let sides = |(face, corner): (u32, u8)| -> Option<(usize, usize)> {
        let face = faces.get(face as usize)?;
        let corner = usize::from(corner).min(2);
        let first = face[FIRST_NEIGHBOUR[corner]];
        let second = face[SECOND_NEIGHBOUR[corner]];
        (first < count && second < count).then_some((first, second))
    };
    let mut state = vec![-1i32; count];
    let mut open: Vec<(usize, f32)> = Vec::new();
    for &pin in pins {
        if pin < count {
            state[pin] = open.len() as i32;
            open.push((pin, 0.0));
        }
    }
    if open.is_empty() {
        open.push((0, 0.0));
        state[0] = 0;
    }
    let mut live = open.len();
    let mut neighbours: Vec<Neighbour> = Vec::new();
    while live >= 1 {
        let mut best = 0;
        let first = open[0].1;
        for (slot, &(_, far)) in open.iter().enumerate().take(live).skip(1) {
            if first > far {
                best = slot;
            }
        }
        live -= 1;
        let (vertex, distance) = open[best];
        open[best] = open[live];
        state[open[best].0] = best as i32;
        let at = positions[vertex];
        neighbours.clear();
        for &corner in fan(vertex) {
            let Some((first, second)) = sides(corner) else {
                continue;
            };
            let to_first = squared_distance(at, positions[first]);
            let to_second = squared_distance(at, positions[second]);
            let across = squared_distance(positions[first], positions[second]);
            // The far end of the triangle's longest edge from the vertex.
            #[allow(clippy::neg_cmp_op_on_partial_ord)]
            let far = if !(across < to_second) {
                (across < to_first).then_some(first)
            } else if to_second < to_first {
                Some(first)
            } else {
                Some(second)
            };
            if far != Some(first) {
                Neighbour::set(&mut neighbours, first, None, Some(second));
            }
            if far != Some(second) {
                Neighbour::set(&mut neighbours, second, Some(first), None);
            }
        }
        let listed =
            |neighbours: &[Neighbour], v: i64| neighbours.iter().any(|n| n.vertex as i64 == v);
        let settled = |state: &[i32], v: i64| usize::try_from(v).is_ok_and(|v| state[v] < -1);
        // Whether a face about `from` joins `to` to a settled vertex.
        let joined = |state: &[i32], from: usize, to: usize| {
            fan(from).iter().any(|&corner| {
                sides(corner).is_some_and(|(first, second)| {
                    (first == to && state[second] < -1) || (second == to && state[first] < -1)
                })
            })
        };
        for i in 0..neighbours.len() {
            let here = neighbours[i];
            if state[here.vertex] > SETTLED {
                tree.push((vertex, here.vertex));
                let along = crate::engine_math::length3(at - positions[here.vertex]) + distance;
                let slot = state[here.vertex];
                if slot < 0 {
                    if live < open.len() {
                        open[live] = (here.vertex, along);
                    } else {
                        open.push((here.vertex, along));
                    }
                    state[here.vertex] = live as i32;
                    live += 1;
                } else if along < open[slot as usize].1 {
                    open[slot as usize].1 = along;
                }
            }
            for j in i + 1..neighbours.len() {
                let there = neighbours[j];
                if here.before >= 0
                    && here.before == there.after
                    && settled(&state, here.before)
                    && !listed(&neighbours, here.before)
                {
                    continue;
                }
                if here.after >= 0
                    && here.after == there.before
                    && settled(&state, here.after)
                    && !listed(&neighbours, here.after)
                {
                    continue;
                }
                if there.before == here.vertex as i64 && joined(&state, here.vertex, there.vertex) {
                    continue;
                }
                if there.after == here.vertex as i64 && joined(&state, here.vertex, there.vertex) {
                    continue;
                }
                let arm = at - positions[here.vertex];
                let other = positions[there.vertex] - at;
                let cosine = crate::engine_math::cosine(arm, other);
                if f64::from(cosine) > 0.7 {
                    straight.push((here.vertex, there.vertex));
                } else {
                    bent.push((here.vertex, there.vertex));
                }
            }
        }
        state[vertex] = SETTLED;
    }
    tree.extend(straight);
    tree.extend(bent);
    tree
}

/// The integrate slot's step of every hold, before the constraint passes:
/// the hold's target, kept within its radius of its pin's particle where
/// that is now (`pin + d·(radius/|d|)` when `|d|² > radius²`, else the
/// target as it is; a hold without a pin takes its target), then its
/// smoothed point moves on as a damped follower: the motion it made last
/// tick, plus [`GRAB_PULL`] of the way to that target, times
/// [`GRAB_FOLLOW`].
fn smooth_grabs(cloth: &mut Cloth) {
    for index in 0..cloth.grabs.len() {
        let grab = cloth.grabs[index];
        let toward = match grab.pin.and_then(|p| cloth.particles.get(p)) {
            None => grab.target,
            Some(pin) => {
                let d = grab.target - pin.position;
                let d2 = (d.x * d.x + d.y * d.y) + d.z * d.z;
                // Within the radius, or unordered: the target.
                if !(grab.radius * grab.radius < d2) {
                    grab.target
                } else {
                    pin.position + d * (grab.radius / d2.sqrt())
                }
            }
        };
        let mut motion = grab.smoothed - grab.last;
        let g = &mut cloth.grabs[index];
        g.last = g.smoothed;
        motion += (toward - g.smoothed) * GRAB_PULL;
        g.smoothed += motion * GRAB_FOLLOW;
    }
}

/// Each constraint pass's holds, after the pins: the hold's point on its
/// particles (`((p₀·w₀) + p₁·w₁) + p₂·w₂`) is moved onto its smoothed point,
/// each corner by the gap times its weight over the largest weight
/// (`MAXSS`).
fn spread_grabs(cloth: &mut Cloth) {
    let count = cloth.particles.len();
    for index in 0..cloth.grabs.len() {
        let grab = cloth.grabs[index];
        let [a, b, c] = grab.corners;
        if a >= count || b >= count || c >= count {
            continue;
        }
        let [w0, w1, w2] = grab.weights;
        let p = |i: usize| cloth.particles[i].position;
        let at = p(a) * w0 + p(b) * w1 + p(c) * w2;
        let gap = grab.smoothed - at;
        let scale = 1.0 / crate::engine_math::max(crate::engine_math::max(w0, w1), w2);
        cloth.particles[a].position += gap * (w0 * scale);
        cloth.particles[b].position += gap * (w1 * scale);
        cloth.particles[c].position += gap * (w2 * scale);
    }
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
    spread_grabs(cloth);
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

/// The integrate slot's rustle, for a cloth whose record names a rustle
/// sound: while the squared velocity sum is above 400 the loop plays at
/// `0.5·tail + 0.0001·|sum|` (the half in single, the sum in double) and the
/// pitch `min(0.5·volume + 0.5, 2)` (`minss`: a NaN takes 2); otherwise it
/// is stopped. Every awake tick the tail decays by 0.9 in double. (The
/// tail is what the body contact adds to the cloth's energy; Exarust's
/// body contact does not add it yet, so the loop plays from the sum
/// alone.)
fn settle_rustle(cloth: &mut Cloth, sum: Vec3) {
    if cloth.record.rustle.is_some() {
        let speed_sq = (sum.x * sum.x + sum.y * sum.y) + sum.z * sum.z;
        if speed_sq > RUSTLE_SPEED_SQ {
            let volume =
                (f64::from(cloth.rustle_tail * 0.5) + f64::from(speed_sq.sqrt()) * 0.0001) as f32;
            let pitch = volume * 0.5 + 0.5;
            cloth.rustle = true;
            cloth.rustle_volume = volume;
            cloth.rustle_pitch = if pitch < 2.0 { pitch } else { 2.0 };
        } else {
            cloth.rustle = false;
        }
    }
    cloth.rustle_tail = (f64::from(cloth.rustle_tail) * 0.9) as f32;
}

/// A cloth's rustle loop as the step leaves it, for the sound to follow:
/// playing at a place (world, metres) with a volume and pitch, or stopped.
/// Sent while it plays and once when it stops.
#[derive(bevy::prelude::Message, Debug, Clone, PartialEq)]
pub struct ClothRustled {
    pub cloth: bevy::prelude::Entity,
    /// The record's rustle sound set.
    pub sound: &'static str,
    pub position: Vec3,
    pub volume: f32,
    pub pitch: f32,
    pub playing: bool,
}

fn push_plane(particle: &mut Particle, plane: LevelPlane, padding: f32) -> bool {
    let normal = plane.normal.normalize_or_zero();
    if normal == Vec3::ZERO || !plane.point.is_finite() {
        return false;
    }
    let distance = (particle.position - plane.point).dot(normal);
    if distance >= padding {
        return false;
    }
    particle.position += normal * (padding - distance);
    true
}

/// The edge pass the original runs (its other, square-root-free form sits
/// behind a switch the game never sets): `d = b − a`, its squared length
/// summed `(x² + z²) + y²`, `f = 0.5 − rsqrt(|d|²)·h` with `h` the link's
/// half rest distance, then `a += f·d` and `b −= f·d`. The reciprocal
/// square root is the processor's approximation, as the original's. Two
/// particles on one point make `f` infinite and the move not a number, as
/// in the original.
fn correct_edge(particles: &mut [Particle], edge: Edge) {
    if edge.a >= particles.len() || edge.b >= particles.len() {
        return;
    }
    let a = particles[edge.a].position;
    let b = particles[edge.b].position;
    let d = b - a;
    let squared = (d.x * d.x + d.z * d.z) + d.y * d.y;
    let f = 0.5 - crate::engine_math::reciprocal_sqrt(squared) * edge.half_rest;
    let shift = d * f;
    particles[edge.a].position = a + shift;
    particles[edge.b].position = b - shift;
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

    /// The tick divides at the level's meshes: the step before them leaves
    /// the level alone, and the level contact runs between it and the
    /// finish.
    #[test]
    fn the_step_stops_before_the_level_and_finishes_after_it() {
        let mut cloth = Cloth::new(vec![still(Vec3::new(0.0, 1.0, 0.0))]);
        // Wound so the face points up: the level meets a particle's front
        // side only, with the particle 1 cm over it and the record's 2 cm
        // padding.
        cloth.level.push(LevelTri {
            a: Vec3::new(-10.0, 0.0, -10.0),
            b: Vec3::new(0.0, 0.0, 10.0),
            c: Vec3::new(10.0, 0.0, -10.0),
        });
        let (before, awake) =
            step_before_level(&mut cloth, gravity_cm(1.0)).expect("an awake cloth steps");
        assert_eq!(before.len(), 1);
        assert_eq!(cloth.level_tests, 0, "the step ran the level contact");
        let y = cloth.particles[0].position.y;
        assert!(crate::cloth_level::touch_level(&mut cloth));
        assert!(cloth.level_tests > 0);
        assert!(cloth.particles[0].position.y > y);
        finish_step(&mut cloth, &before, awake);
        assert!(cloth.reaction.y < 0.0);
    }

    #[test]
    fn a_cloth_vertex_falls_and_an_edge_pulls() {
        let g = gravity_cm(1.0);
        assert!(g.y < 0.0);
        let mut cloth = Cloth::new(vec![still(Vec3::new(0.0, 100.0, 0.0))]);
        step(&mut cloth, g);
        assert!((cloth.particles[0].position.y - (100.0 + g.y)).abs() < 1e-4);
        assert!((cloth.particles[0].previous.y - 100.0).abs() < 1e-6);

        // A link at its rest distance stays (up to the approximate root).
        let mut cloth = Cloth::new(vec![still(Vec3::ZERO), still(Vec3::new(20.0, 0.0, 0.0))]);
        cloth.edges.push(Edge {
            a: 0,
            b: 1,
            half_rest: 10.0,
        });
        step(&mut cloth, Vec3::ZERO);
        assert!(cloth.particles[0].position.abs_diff_eq(Vec3::ZERO, 1e-2));
        assert!(
            cloth.particles[1]
                .position
                .abs_diff_eq(Vec3::new(20.0, 0.0, 0.0), 1e-2)
        );
        // A link stretched to twice its rest is pulled all the way back.
        let mut cloth = Cloth::new(vec![still(Vec3::ZERO), still(Vec3::new(20.0, 0.0, 0.0))]);
        cloth.edges.push(Edge {
            a: 0,
            b: 1,
            half_rest: 5.0,
        });
        step(&mut cloth, Vec3::ZERO);
        let gap = cloth.particles[1].position - cloth.particles[0].position;
        assert!((gap.x - 10.0).abs() < 1e-2, "{gap}");
        assert!((cloth.particles[0].position.x - 5.0).abs() < 1e-2);

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

        // The rustle, worked from the integrate slot's assembly: a 30 cm
        // tick over 400 squared plays at `0.5·tail + 0.0001·30`, pitch
        // `0.5·v + 0.5`; the tail decays by 0.9 in double each awake tick.
        let mut cloth = Cloth::new(vec![Particle {
            position: Vec3::ZERO,
            previous: Vec3::new(-30.0, 0.0, 0.0),
        }]);
        cloth.rustle_tail = 0.2;
        step(&mut cloth, Vec3::ZERO);
        assert!(cloth.rustle);
        let volume = (f64::from(0.2f32 * 0.5) + f64::from(30.0f32) * 0.0001) as f32;
        assert_eq!(cloth.rustle_volume, volume);
        assert_eq!(cloth.rustle_pitch, volume * 0.5 + 0.5);
        assert_eq!(cloth.rustle_tail, (f64::from(0.2f32) * 0.9) as f32);
        let tail = cloth.rustle_tail;
        cloth.particles[0].previous = cloth.particles[0].position;
        step(&mut cloth, Vec3::ZERO);
        assert!(!cloth.rustle);
        assert_eq!(cloth.rustle_tail, (f64::from(tail) * 0.9) as f32);
        // A record without a rustle sound never rustles.
        let mut quiet = Cloth::new(vec![Particle {
            position: Vec3::ZERO,
            previous: Vec3::new(-30.0, 0.0, 0.0),
        }]);
        quiet.record.rustle = None;
        step(&mut quiet, Vec3::ZERO);
        assert!(!quiet.rustle);

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
    }

    /// The step's wind hook, worked from its assembly: the strength is the
    /// scene's wind times 0.3 in double, each sample draws once and walks
    /// by `(r − 0.5)·s` in range, `−s·0.5·r` above 1 and `+s·0.5·r` below
    /// −1 (a NaN takes the in-range walk), `s` the strength times 0.2 and
    /// 4 in double; then the direction is normalised and the frame built
    /// from it.
    #[test]
    fn the_wind_hook_walks_the_tile_and_turns_the_frame() {
        let mut wind = Wind::CALM;
        wind.tile[1] = 1.5;
        wind.tile[2] = -1.5;
        wind.tile[3] = f32::NAN;
        wind.advance(2.0);
        assert_eq!(wind.strength, (2.0f64 * 0.3) as f32);
        let stride = (f64::from(wind.strength) * 0.2 * 4.0) as f32;
        let mut roll = 0u32;
        let r: Vec<f32> = (0..4).map(|_| next_random(&mut roll)).collect();
        assert_eq!(r[0], (1.0f64 / 4_294_967_296.0) as f32);
        assert_eq!(wind.tile[0], (r[0] - 0.5) * stride + 0.0);
        assert_eq!(wind.tile[1], 1.5 - stride * 0.5 * r[1]);
        assert_eq!(wind.tile[2], stride * 0.5 * r[2] + -1.5);
        assert!(wind.tile[3].is_nan());
        // Every sample drew once.
        let mut check = 0u32;
        for _ in 0..NOISE_CELLS * NOISE_CELLS {
            next_random(&mut check);
        }
        assert_eq!(wind.roll, check);
        // The direction is normalised; the frame's last row is it, its up
        // row the world's up (scaled by the approximate root), its first
        // row across both.
        let d = crate::engine_math::normalize_approx(WIND_DIRECTION);
        assert_eq!(wind.direction, d);
        assert_eq!(wind.frame[2], d);
        assert_eq!(wind.frame[1].x, 0.0);
        assert_eq!(wind.frame[1].z, 0.0);
        assert!((wind.frame[1].y - 1.0).abs() < 1e-3);
        assert!(wind.frame[0].dot(d).abs() < 1e-3);
        assert!((wind.frame[0].length() - 1.0).abs() < 1e-3);
        // No wind still draws.
        let mut calm = Wind::CALM;
        calm.advance(0.0);
        assert_eq!(calm.strength, 0.0);
        assert_eq!(calm.roll, check);
    }

    /// The tile's bilinear sample, worked from its assembly: 2 m cells,
    /// wrapping at both edges.
    #[test]
    fn the_wind_tile_is_sampled_between_its_cells() {
        let mut wind = Wind::CALM;
        for (i, sample) in wind.tile.iter_mut().enumerate() {
            *sample = i as f32;
        }
        // On a cell corner: 250 cm is 1.25 tiles, cell 4; 50 cm cell 4.
        assert_eq!(wind.sample(250.0, 50.0), 68.0);
        // Inside a cell.
        let (fx, fy) = (0.25f32, 0.5f32);
        let at = wind.sample(fx * 12.5, 50.0 + fy * 12.5);
        let want =
            (1.0 - fy) * ((1.0 - fx) * 64.0 + fx * 65.0) + fy * ((1.0 - fx) * 80.0 + fx * 81.0);
        assert!((at - want).abs() < 1e-3, "{at} {want}");
        // Past the last column and row it wraps to the first.
        let at = wind.sample(-10.0, 790.0);
        let (fx, fy) = (0.2f32, 0.2f32);
        let want =
            (1.0 - fy) * ((1.0 - fx) * 255.0 + fx * 240.0) + fy * ((1.0 - fx) * 15.0 + fx * 0.0);
        assert!((at - want).abs() < 1e-2, "{at} {want}");
    }

    /// In the world's wind each particle moves along it by
    /// `(0.25 + noise)·strength`, the floor compared in double (a 0.001
    /// strength in single is above 0.001 in double, so it blows).
    #[test]
    fn the_wind_blows_a_cloth_along_it() {
        let mut wind = Wind::CALM;
        wind.strength = 1.0;
        wind.direction = Vec3::X;
        wind.frame = [Vec3::X, Vec3::Y, Vec3::Z];
        wind.tile = [1.0; NOISE_CELLS * NOISE_CELLS];
        let at = WindAt {
            wind: &wind,
            to_world: bevy::math::Affine3A::IDENTITY,
        };
        let mut blown = Cloth::new(vec![still(Vec3::ZERO)]);
        step_before_level_in(&mut blown, Vec3::ZERO, Some(at)).unwrap();
        assert_eq!(blown.particles[0].position, Vec3::new(1.25, 0.0, 0.0));
        let mut faint = wind.clone();
        faint.strength = 0.001;
        let mut cloth = Cloth::new(vec![still(Vec3::ZERO)]);
        let at = WindAt {
            wind: &faint,
            to_world: bevy::math::Affine3A::IDENTITY,
        };
        step_before_level_in(&mut cloth, Vec3::ZERO, Some(at)).unwrap();
        assert_eq!(cloth.particles[0].position.x, 1.25 * 0.001);
        faint.strength = 0.0009;
        let mut cloth = Cloth::new(vec![still(Vec3::ZERO)]);
        let at = WindAt {
            wind: &faint,
            to_world: bevy::math::Affine3A::IDENTITY,
        };
        step_before_level_in(&mut cloth, Vec3::ZERO, Some(at)).unwrap();
        assert_eq!(cloth.particles[0].position, Vec3::ZERO);
        // A sleeping cloth is not blown.
        let mut asleep = Cloth::new(vec![still(Vec3::ZERO)]);
        asleep.sleep = 0;
        let at = WindAt {
            wind: &wind,
            to_world: bevy::math::Affine3A::IDENTITY,
        };
        step_before_level_in(&mut asleep, Vec3::ZERO, Some(at)).unwrap();
        assert_eq!(asleep.particles[0].position, Vec3::ZERO);
    }

    /// The integrate loop adds `((v + (v̄ − v)·drag)·keep + g) + x`.
    #[test]
    fn the_integrate_adds_the_travel_and_gravity_before_the_place() {
        let x = Vec3::new(1000.1, 3.3, -77.7);
        let previous = x - Vec3::new(0.37, -0.11, 0.05);
        let v = x - previous;
        let mut cloth = Cloth::new(vec![Particle {
            position: x,
            previous,
        }]);
        let g = gravity_cm(1.0);
        step_before_level(&mut cloth, g).unwrap();
        let record = cloth.record;
        let retained = (v + (Vec3::ZERO - v) * record.mean_drag) * record.retention();
        let want = (retained + record.gravity(g)) + x;
        assert_eq!(cloth.particles[0].position, want);
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

    /// The hand's drag makes a hold on the vertex it took (all the weight
    /// on it), wakes the cloth to 32, and the hold's smoothed point follows
    /// the target as a damped follower, the vertex put on it by each
    /// constraint pass; letting go drops the hold and leaves the vertex to
    /// gravity. A vertex the cloth does not have is ignored.
    #[test]
    fn a_grabbed_vertex_follows_the_hand() {
        let mut cloth = Cloth::new(vec![still(Vec3::ZERO), still(Vec3::new(10.0, 0.0, 0.0))]);
        cloth.sleep = 0;
        let target = Vec3::new(0.0, 0.0, 50.0);
        cloth.grab(0, target);
        assert_eq!(cloth.sleep, GRAB_WAKE);
        assert_eq!(cloth.grabs.len(), 1);
        // Made at the drag's first target: the smoothed point is there.
        step(&mut cloth, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(cloth.particles[0].position, target);
        // A new target: the follower moves 0.3 of a tenth of the way, then
        // keeps its motion.
        let next = Vec3::new(0.0, 0.0, 60.0);
        cloth.grab(0, next);
        assert_eq!(cloth.grabs.len(), 1);
        step(&mut cloth, Vec3::ZERO);
        let first = target + ((next - target) * GRAB_PULL) * GRAB_FOLLOW;
        assert_eq!(cloth.grabs[0].smoothed, first);
        assert_eq!(cloth.particles[0].position, first);
        step(&mut cloth, Vec3::ZERO);
        let motion = (first - target) + (next - first) * GRAB_PULL;
        assert_eq!(cloth.grabs[0].smoothed, first + motion * GRAB_FOLLOW);
        cloth.release_grab();
        assert!(cloth.grabs.is_empty());
        let at = cloth.particles[0].position;
        step(&mut cloth, Vec3::new(0.0, 0.0, -1.0));
        assert!(cloth.particles[0].position.z < at.z);
        cloth.grab(9, target);
        assert!(cloth.grabs.is_empty());
    }

    /// The tangent rebuild: a face's u-gradient through the exact
    /// reciprocal, summed into its vertices, made perpendicular to each
    /// vertex's normal and normalised by the approximate root, with the
    /// fourth lane (the loaded handedness) kept.
    #[test]
    fn the_tangents_are_rebuilt_from_the_moved_faces() {
        let positions = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0], [10.0, 10.0, 5.0]];
        let uvs = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let tilted = Vec3::new(0.0, 0.6, 0.8);
        let normals = [[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], tilted.to_array(), [0.0, 0.0, 1.0]];
        let indices = [0, 1, 2];
        let mut tangents = [[9.0, 9.0, 9.0, -1.0], [0.0, 0.0, 0.0, 1.0], [0.0; 4], [7.0, 7.0, 7.0, 1.0]];
        rebuild_tangents(&positions, &normals, &uvs, &indices, &mut tangents);
        // Δu₁ = 1, Δv₂ = 1: t = e₁ = (10, 0, 0) at each corner.
        let unit = crate::engine_math::normalize_approx(Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(&tangents[0][..3], &unit.to_array());
        assert_eq!(tangents[0][3], -1.0);
        assert_eq!(tangents[1], [unit.x, unit.y, unit.z, 1.0]);
        // Already across the tilted normal: unchanged but for the root.
        assert_eq!(&tangents[2][..3], &unit.to_array());
        // A vertex no face reaches: its zero sum stays zero.
        assert_eq!(tangents[3], [0.0, 0.0, 0.0, 1.0]);
        // A sum with a part along the normal loses it.
        let uvs = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]];
        let normals = [tilted.to_array(); 4];
        let mut tangents = [[0.0; 4]; 4];
        rebuild_tangents(&positions, &normals, &uvs, &[0, 1, 2], &mut tangents);
        // Δu₂ = 1, Δv₁ = 1: r = −1, t = −(−1·e₂) = e₂ = (0, 10, 0).
        let t = Vec3::new(0.0, 10.0, 0.0);
        let along = (t.x * tilted.x + t.y * tilted.y) + t.z * tilted.z;
        let want = crate::engine_math::normalize_approx(t + tilted * -along);
        assert_eq!(&tangents[0][..3], &want.to_array());
        assert!(Vec3::from_slice(&tangents[0][..3]).dot(tilted).abs() < 1e-3);
    }

    /// A hold made on a triangle point measures its radius in the mesh to
    /// the nearest pin (the first of equals), 5 cm with no pin and 0 on the
    /// pin itself; the constraint pass moves the point onto the smoothed
    /// point, each corner by its weight over the largest; and a target
    /// beyond the radius of the pin's particle is pulled back onto that
    /// sphere.
    #[test]
    fn a_hold_spreads_over_its_triangle_within_its_pins_reach() {
        let corners = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(0.0, -30.0, 0.0),
        ];
        let mut cloth = Cloth::from_mesh_linked(&corners, &[[0, 1, 2]], &[], &[]);
        // No pin: 5 cm.
        let w = [0.5, 0.25, 0.25];
        cloth.add_grab([0, 1, 2], w, Vec3::ZERO).unwrap();
        assert_eq!(cloth.grabs[0].radius, GRAB_FREE_RADIUS);
        assert_eq!(cloth.grabs[0].pin, None);
        cloth.grabs.clear();
        // Pins at 3 (30 cm below 0) and 2: the point (2.5, 2.5, 0) is
        // √(2.5² + 7.5²) from 2's rest.
        cloth.pins = vec![
            Pin {
                index: 3,
                rest: corners[3],
                weight: 1.0,
            },
            Pin {
                index: 2,
                rest: corners[2],
                weight: 1.0,
            },
        ];
        cloth.add_grab([0, 1, 2], w, Vec3::ZERO).unwrap();
        let point = corners[0] * 0.5 + corners[1] * 0.25 + corners[2] * 0.25;
        let d = point - corners[2];
        assert_eq!(cloth.grabs[0].pin, Some(2));
        assert_eq!(cloth.grabs[0].radius, ((d.x * d.x + d.y * d.y) + d.z * d.z).sqrt());
        // On the pin itself: 0.
        cloth.grabs.clear();
        cloth.add_grab([2, 0, 1], [1.0, 0.0, 0.0], Vec3::ZERO).unwrap();
        assert_eq!(cloth.grabs[0].radius, 0.0);
        cloth.grabs.clear();
        // The spread: the point goes onto the smoothed point.
        cloth.pins.clear();
        cloth.add_grab([0, 1, 2], w, Vec3::new(2.5, 2.5, 4.0)).unwrap();
        spread_grabs(&mut cloth);
        let gap = Vec3::new(0.0, 0.0, 4.0);
        assert_eq!(cloth.particles[0].position, corners[0] + gap * (0.5 * 2.0));
        assert_eq!(cloth.particles[1].position, corners[1] + gap * (0.25 * 2.0));
        assert_eq!(cloth.particles[2].position, corners[2] + gap * (0.25 * 2.0));
        // Beyond the radius of the pin's particle: pulled onto the sphere.
        let mut cloth = Cloth::from_mesh_linked(&corners, &[[0, 1, 2]], &[0], &[]);
        cloth.add_grab([1, 1, 1], [1.0, 0.0, 0.0], corners[1]).unwrap();
        assert_eq!(cloth.grabs[0].radius, 10.0);
        cloth.grabs[0].target = Vec3::new(40.0, 0.0, 0.0);
        smooth_grabs(&mut cloth);
        let toward = Vec3::new(10.0, 0.0, 0.0);
        assert_eq!(
            cloth.grabs[0].smoothed,
            corners[1] + ((toward - corners[1]) * GRAB_PULL) * GRAB_FOLLOW
        );
        assert_eq!(cloth.grabs[0].last, corners[1]);
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

    /// The original's edge pass, worked from its assembly: `d = b − a`,
    /// `(x² + z²) + y²`, the approximate reciprocal root times the half
    /// rest, `f = 0.5 − that`, `a += f·d`, `b −= f·d`.
    #[test]
    fn the_edge_pass_is_the_full_correction_through_the_approximate_root() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 6.0, 15.0);
        let mut particles = vec![still(a), still(b)];
        correct_edge(
            &mut particles,
            Edge {
                a: 0,
                b: 1,
                half_rest: 2.5,
            },
        );
        // |d|² = (9 + 144) + 16 = 169: a 13 cm link with a 5 cm rest.
        let r = crate::engine_math::reciprocal_sqrt(169.0);
        assert!((r - 1.0 / 13.0).abs() < 1e-4);
        let f = 0.5 - r * 2.5;
        let d = b - a;
        assert_eq!(particles[0].position, a + d * f);
        assert_eq!(particles[1].position, b - d * f);
        // About 4 cm in from each end: 5 cm apart.
        let gap = particles[1].position - particles[0].position;
        assert!((gap.length() - 5.0).abs() < 2e-2, "{gap}");
        // The rest a link is built with is half the distance.
        let edge = Edge::at_rest(&[a, b], 0, 1);
        assert_eq!(edge.half_rest, 6.5);
    }

    /// The walk out from the pin over one quad, worked by hand from the
    /// original's assembly: the incidence lists faces last first; from the
    /// pin the diagonal is not a neighbour, the open-list pick keeps the
    /// first entry on a tie, and the two neighbour pairs at right angles
    /// (the quad's two diagonals) come after the four sides.
    #[test]
    fn the_links_are_walked_out_from_the_pins() {
        let vertices = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        let faces = [[0, 1, 2], [1, 3, 2]];
        let (starts, fans) = incidence(&faces, 4);
        assert_eq!(starts, vec![0, 1, 3, 5, 6]);
        assert_eq!(fans, vec![(0, 0), (1, 0), (0, 1), (1, 2), (0, 2), (1, 1)]);
        let links = walk_links(&vertices, &faces, &starts, &fans, &[0]);
        assert_eq!(links, vec![(0, 2), (0, 1), (2, 3), (1, 3), (2, 1), (3, 0)]);
        let cloth = Cloth::from_mesh(&vertices, &faces, &[0]);
        let pairs: Vec<_> = cloth.edges.iter().map(|e| (e.a, e.b)).collect();
        assert_eq!(pairs, links);
        assert_eq!(cloth.edges[0].half_rest, 0.5);
        // No pins: the walk starts at vertex 0 all the same.
        assert_eq!(walk_links(&vertices, &faces, &starts, &fans, &[]), links);
        // A vertex no pin reaches gets no link.
        let apart = [
            vertices[0],
            vertices[1],
            vertices[2],
            Vec3::new(9.0, 9.0, 0.0),
        ];
        let lone = [[0, 1, 2]];
        let (starts, fans) = incidence(&lone, 4);
        let links = walk_links(&apart, &lone, &starts, &fans, &[3]);
        assert!(links.is_empty(), "{links:?}");
    }

    /// Along a strip of two quads the pair through the middle vertex is
    /// nearly in line (cosine above 0.7) and is listed after every walk
    /// link and before the pairs at an angle.
    #[test]
    fn the_in_line_pairs_come_before_the_bent_ones() {
        let vertices = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(2.0, 1.0, 0.0),
        ];
        let faces = [[0, 1, 3], [1, 4, 3], [1, 2, 4], [2, 5, 4]];
        let cloth = Cloth::from_mesh(&vertices, &faces, &[0, 3]);
        let pairs: Vec<_> = cloth.edges.iter().map(|e| ordered(e.a, e.b)).collect();
        let straight = pairs
            .iter()
            .position(|&p| p == (0, 2))
            .expect("the pair through vertex 1");
        // Seven sides, then the in-line pairs, then the diagonals.
        assert_eq!(straight, 7, "{pairs:?}");
        assert!(pairs[..7].iter().all(|&(a, b)| {
            let d = vertices[a] - vertices[b];
            (d.length() - 1.0).abs() < 1e-6
        }));
        let last = pairs.last().copied().unwrap();
        let d = vertices[last.0] - vertices[last.1];
        assert!((d.length() - 2f32.sqrt()).abs() < 1e-6, "{pairs:?}");
    }
}
