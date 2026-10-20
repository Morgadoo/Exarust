//! The original's CPU particle emitter (`TParticleSystem`) and its template
//! records, as plain state and rules at the simulation's 83 Hz tick.
//!
//! An emitter keeps a ring of Verlet particles, ages them, integrates them,
//! spawns new ones from one or more sources and tracks the light and sound
//! it drives. The template record (`TParticleSysDescriptor`) holds the
//! per-second values; [`Descriptor::scaled`] scales it by the emitter's size
//! and [`Emitter::new`] derives the per-tick values exactly as the original
//! does. [`Emitter::quads`] builds the camera-facing quads the renderer
//! draws.
//!
//! Units are the original's: centimetres, seconds for the template and
//! ticks for the simulation. Rules: `docs/RENDERING.md` (Fires).

use bevy::prelude::{Mat3, Vec2, Vec3};

/// The simulation's tick rate.
pub const TICK_HZ: f32 = 83.0;
/// Gravity per tick² (cm): the environment's value, which the original
/// reads from the sim rather than from the template.
pub const GRAVITY: f32 = -980.665 / (TICK_HZ * TICK_HZ);

/// Behaviour flags of an emitter (the template's, copied to the emitter).
pub mod flags {
    /// Low two bits: 1 = random initial velocity from the template's spread.
    pub const VELOCITY_MASK: u32 = 3;
    /// Random initial velocity mode.
    pub const VELOCITY_RANDOM: u32 = 1;
    /// Start each particle's life shortened by up to a quarter.
    pub const LIFE_RANDOM: u32 = 4;
    /// One-shot: the emitter stops when the ring empties.
    pub const ONE_SHOT: u32 = 0x10;
    /// Stop the emitter once its tick deadline passes.
    pub const FINITE: u32 = 0x20;
    /// Keep the bounding box instead of updating it.
    pub const FIXED_BOUNDS: u32 = 0x40;
    /// Quad orientation from the camera's forward axis, not the emitter.
    pub const CAMERA_FORWARD: u32 = 0x80;
    /// Flat colour from the particle's age.
    pub const FLAT_COLOUR: u32 = 0x100;
    /// Colour from the key list without the ring slot's parity offset.
    pub const KEYS: u32 = 0x200;
    /// Colour from the key list, odd ring slots one key later.
    pub const KEYS_PARITY: u32 = 0x300;
    /// Per-particle UV variant 0..7.
    pub const UV_8: u32 = 0x400;
    /// Per-particle UV variant 0..31 (atlas cell and square symmetry).
    pub const UV_32: u32 = 0x800;
    /// Per-particle size from a random table.
    pub const RANDOM_SIZE: u32 = 0x2000;
    /// Fire ageing: distance cap and a faster decay.
    pub const FIRE_AGEING: u32 = 0x40000;
    /// Extra ageing for burst deaths, and an accumulator refund.
    pub const BURST: u32 = 0x80000;
    /// Sound volume follows the emitter's motion.
    pub const SOUND_MOTION: u32 = 0x2000_0000;
    /// Sound pitch takes the wide random walk.
    pub const SOUND_WIDE_PITCH: u32 = 0x4000_0000;
    /// Quad mode, the top nibble.
    pub const QUAD_MASK: u32 = 0xf000_0000;
    /// Velocity-aligned diamond (the flames' quads).
    pub const QUAD_DIAMOND: u32 = 0x4000_0000;
    /// Screen-aligned quad with the age-based size (the ember beds).
    pub const QUAD_STRETCHED: u32 = 0x1000_0000;
}

/// Random draws in the original's order. The integer stream is the
/// original's linear congruential sequence; the float stream is a second
/// instance of it, standing in for the original's global Mersenne twister
/// (whose seed is not reproducible). The number and order of draws, which
/// is what the rules depend on, are the same.
pub trait Draws {
    /// An integer in `[0, n)`, as `(seed · n) >> 32`.
    fn int(&mut self, n: u32) -> u32;
    /// A float in `[0, 1)`.
    fn unit(&mut self) -> f32;
}

/// The default streams: the original's integer LCG and a float LCG.
#[derive(Debug, Clone)]
pub struct Rng {
    ints: u32,
    floats: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self {
            ints: seed | 1,
            floats: seed.wrapping_mul(0x9E37_79B9) | 1,
        }
    }
}

impl Draws for Rng {
    fn int(&mut self, n: u32) -> u32 {
        self.ints = self.ints.wrapping_mul(0x0808_8405).wrapping_add(1);
        ((u64::from(self.ints) * u64::from(n)) >> 32) as u32
    }

    fn unit(&mut self) -> f32 {
        self.floats = self.floats.wrapping_mul(0x0808_8405).wrapping_add(1);
        (self.floats >> 8) as f32 * (1.0 / 16_777_216.0)
    }
}

/// A template's light: sRGB colour, base intensity, radius in cm and the
/// flicker step.
#[derive(Debug, Clone, Copy)]
pub struct LightDesc {
    pub colour: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub flicker: f32,
}

impl LightDesc {
    pub const NONE: Self = Self {
        colour: [0.0; 3],
        intensity: 0.0,
        radius: 0.0,
        flicker: 0.0,
    };

    /// Whether a template of this light builds one (a radius is required).
    pub fn is_some(&self) -> bool {
        self.radius > 0.0
    }
}

/// A colour key: RGBA bytes, the alpha the particle's opacity.
pub type Key = [u8; 4];

/// A colour key list: the count and its keys (K1 at index 0). The original
/// stores the count as the list's first byte and the keys after it.
#[derive(Debug, Clone, Copy)]
pub struct Keys {
    pub count: i32,
    pub entries: &'static [Key],
}

impl Keys {
    /// The colour at normalised age `age`; odd ring slots start one key
    /// later. Returns bytes, truncated and saturated as the original's
    /// colour lerp does.
    pub fn sample(&self, age: f32, odd: bool) -> Key {
        let f = self.count as f32 * age;
        let f = f.max(0.0);
        let k = f.trunc() as usize;
        let frac = f - k as f32;
        let first = k + 1 + usize::from(odd);
        let a = self.entry(first);
        let b = self.entry(first + 1);
        std::array::from_fn(|c| {
            let v = f32::from(a[c]) + (f32::from(b[c]) - f32::from(a[c])) * frac;
            v.clamp(0.0, 255.0) as u8
        })
    }

    fn entry(&self, i: usize) -> Key {
        self.entries
            .get(i.saturating_sub(1))
            .copied()
            .unwrap_or([0; 4])
    }
}

/// A template record (`TParticleSysDescriptor`) in the original's units.
#[derive(Debug, Clone, Copy)]
pub struct Descriptor {
    /// Particles per second.
    pub rate: i32,
    /// Lifetime, seconds.
    pub life: f32,
    /// Base half-size (cm).
    pub half_size: f32,
    /// Initial speed along the random velocity (cm/s).
    pub speed: f32,
    /// Velocity kept per second; at or below zero, all of it.
    pub retention: f32,
    /// Extra half-size at the end of life, as a multiple of the half-size.
    pub growth: f32,
    /// Stretch along the motion (at or below the epsilon, 1).
    pub stretch: f32,
    /// Spawn ring radius (cm).
    pub spawn_radius: f32,
    /// Horizontal velocity spread (cm/s, a thousandth per random unit).
    pub spread: f32,
    /// Gravity multiplier (negative rises).
    pub buoyancy: f32,
    /// Horizontal pull toward the node per tick.
    pub centring: f32,
    /// Fixed pool capacity; 0 derives it.
    pub pool: i32,
    /// Behaviour flags.
    pub flags: u32,
    /// Colour keys.
    pub keys: Keys,
    /// Light.
    pub light: LightDesc,
    /// Looping sound name.
    pub sound: Option<&'static str>,
    /// Sound base volume.
    pub volume: f32,
    /// Live count the light's density is normalised by; below 1 derives it.
    pub density: i32,
    /// Body dose per contacting particle.
    pub dose: f32,
}

impl Descriptor {
    /// The template scaled by an emitter's size, as the original scales it:
    /// half-size, speed, volume, rate, ring, buoyancy, the light by √size
    /// and the density normaliser.
    pub fn scaled(mut self, size: f32) -> Self {
        self.half_size *= size * 0.5 + 0.5;
        self.speed *= size * size;
        self.volume *= size;
        self.rate = (self.rate as f32 * size).round_ties_even() as i32;
        self.spawn_radius *= size * 0.5 + size * size;
        self.buoyancy *= size;
        self.light.intensity *= size.sqrt();
        self.light.radius *= size.sqrt();
        self.density =
            (self.density as f32 * (size * size * 0.9 + size * 0.1)).round_ties_even() as i32;
        self
    }
}

/// The per-tick values derived from a scaled template.
#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub flags: u32,
    /// Ticks between spawns: `(83 / rate) · 2/3`.
    pub interval: f32,
    /// Lifetime in ticks.
    pub start_life: i32,
    /// Initial speed per tick.
    pub speed: f32,
    pub half_size: f32,
    /// Growth over the life (half-size units × the half-size).
    pub growth: f32,
    /// The collision radius growth (1.5× in quad mode 4).
    pub collision_growth: f32,
    pub spawn_radius: f32,
    pub spread: f32,
    pub buoyancy: f32,
    pub centring: f32,
    pub stretch: f32,
    pub retention: f32,
    /// Ring capacity, excluding the spare slot.
    pub capacity: usize,
    /// Live count the density divides by.
    pub density_count: i32,
    pub keys: Keys,
    pub light: LightDesc,
}

impl Params {
    fn new(d: Descriptor, sources: usize) -> Self {
        let interval = TICK_HZ / d.rate as f32 * (2.0 / 3.0);
        let start_life = (TICK_HZ * d.life) as i32;
        let capacity = if d.pool < 1 {
            sources * ((d.rate as f32 * d.life * 1.5 + 0.501 + 1.0) as usize) + 2
        } else {
            d.pool as usize
        };
        let density_count = if d.density < 1 {
            if d.flags & flags::LIFE_RANDOM != 0 {
                sources as i32 * (d.rate as f32 * d.life * 0.875) as i32
            } else {
                (d.rate as f32 * d.life) as i32 * sources as i32
            }
        } else {
            d.density
        };
        let retention = if d.retention <= 0.0 {
            1.0
        } else {
            (d.retention.ln() / TICK_HZ).exp()
        };
        let stretch = if d.stretch <= 1.192_09e-7 {
            1.0
        } else {
            d.stretch
        };
        let growth = d.growth * d.half_size;
        let collision_growth = if d.flags & flags::QUAD_MASK == flags::QUAD_DIAMOND {
            growth * 1.5
        } else {
            growth
        };
        Self {
            flags: d.flags,
            interval,
            start_life,
            speed: d.speed / TICK_HZ,
            half_size: d.half_size,
            growth,
            collision_growth,
            spawn_radius: d.spawn_radius,
            spread: d.spread,
            buoyancy: d.buoyancy,
            centring: d.centring,
            stretch,
            retention,
            capacity,
            density_count,
            keys: d.keys,
            light: d.light,
        }
    }
}

/// The node an emitter hangs from: its world position and rotation.
#[derive(Debug, Clone, Copy)]
pub struct NodeFrame {
    pub position: Vec3,
    pub rotation: Mat3,
}

impl Default for NodeFrame {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Mat3::IDENTITY,
        }
    }
}

/// A spawn source: a frame on the emitter's ring with a local origin.
#[derive(Debug, Clone, Copy)]
pub struct Source {
    /// The frame's axes, in the emitter's space.
    pub x_axis: Vec3,
    pub direction: Vec3,
    pub y_axis: Vec3,
    /// The frame's origin, relative to the emitter's node.
    pub offset: Vec3,
    /// Spawns left; negative is unlimited.
    pub count: i32,
    /// The source's world origin last time it spawned.
    pub last: Vec3,
    /// The source's world velocity, cm per tick.
    pub velocity: Vec3,
}

impl Source {
    /// The default source: the identity frame at the node.
    pub fn at_node() -> Self {
        Self {
            x_axis: Vec3::X,
            direction: Vec3::NEG_Z,
            y_axis: Vec3::Y,
            offset: Vec3::ZERO,
            count: -1,
            last: Vec3::ZERO,
            velocity: Vec3::ZERO,
        }
    }

    /// A ring of `count` sources. The angles are drawn first, each at least
    /// π/count from every earlier one, then each source's radius (0.5–1 of
    /// `radius`), in the original's order.
    pub fn ring<R: Draws>(count: usize, radius: f32, rng: &mut R) -> Vec<Source> {
        let n = count.max(1);
        let mut angles = vec![0.0f32; n];
        for slot in 0..n {
            loop {
                let angle = std::f32::consts::TAU * rng.unit();
                let clash = angles[..slot]
                    .iter()
                    .any(|&other| (angle - other).abs() < std::f32::consts::PI / n as f32);
                if !clash {
                    angles[slot] = angle;
                    break;
                }
            }
        }
        let mut out = Vec::with_capacity(n);
        for angle in angles {
            let direction = Vec3::new(angle.cos(), 0.0, -angle.sin());
            let x_axis = direction.cross(Vec3::Z).normalize_or(Vec3::X);
            let y_axis = x_axis.cross(direction).normalize_or(Vec3::Y);
            let r = radius * (rng.unit() * 0.5 + 0.5);
            out.push(Source {
                x_axis,
                direction,
                y_axis,
                offset: direction * r,
                count: -1,
                last: Vec3::ZERO,
                velocity: Vec3::ZERO,
            });
        }
        out
    }
}

/// How a body moves an attached emitter.
#[derive(Debug, Clone, Copy)]
pub struct BodyMotion {
    /// The body velocity at the emitter, cm per tick.
    pub velocity: Vec3,
    /// Whether the body carries the particles by its full displacement
    /// (the original's state bit 0x800) rather than half its velocity.
    pub full_displacement: bool,
}

/// A particle slot of the emitter's ring.
#[derive(Debug, Clone, Copy, Default)]
struct Slot {
    p: Vec3,
    q: Vec3,
    life: i32,
    /// Atlas cell (bits 0–2 of the value after the mask) and symmetry.
    variant: u8,
    /// The random size of a template that asks for one (cm).
    size: f32,
}

/// The emitter's light walk and density.
#[derive(Debug, Clone, Copy)]
pub struct LightState {
    /// The template's base intensity.
    pub base: f32,
    /// The flicker step.
    pub step: f32,
    /// The soft-bounded walk and its velocity.
    pub walk: f32,
    pub speed: f32,
    /// Shadow-softness jitter, 0.87–1.15.
    pub jitter: f32,
    /// The softness before the jitter.
    pub softness: f32,
    /// Smoothed live count over the normaliser.
    pub density: f32,
    /// The highest intensity reached, times 1.1.
    pub peak: f32,
}

impl LightState {
    fn new(light: LightDesc) -> Self {
        Self {
            base: light.intensity,
            step: light.flicker,
            walk: 0.0,
            speed: 0.0,
            jitter: 1.0,
            softness: 2.0,
            density: 0.0,
            peak: 0.0,
        }
    }

    /// One tick of the flicker walk, then the density from the live count.
    fn tick<R: Draws>(&mut self, live: usize, normaliser: i32, rng: &mut R) {
        if 1.0 < self.walk || 1.0 < self.speed {
            self.speed -= self.step * rng.unit();
        } else if self.walk < -1.0 || self.speed < -1.0 {
            self.speed += self.step * rng.unit();
        } else {
            self.speed += (self.step + 1.0) * (rng.unit() - 0.5);
        }
        self.walk += self.speed * 0.3 * rng.unit();
        self.jitter += (rng.unit() - 0.5) * 0.125;
        self.jitter = if self.jitter >= 0.87 {
            self.jitter.min(1.15)
        } else {
            0.87
        };
        self.density = live as f32 / normaliser as f32 * 0.2 + self.density * 0.8;
    }

    /// The light's shadow softness this frame.
    pub fn softness(&self) -> f32 {
        self.softness * self.jitter
    }

    /// The light's intensity this frame, and whether it set a new peak.
    pub fn intensity(&mut self) -> (f32, bool) {
        let intensity = self.density * (self.walk * 0.05 + 1.0) * self.base;
        if intensity >= self.peak {
            self.peak = intensity * 1.1;
            (intensity, true)
        } else {
            (intensity, false)
        }
    }

    /// The emissive scale an emissive parent mesh gets.
    pub fn emissive(&self) -> f32 {
        self.density * (self.walk * 0.2 + 0.9) * 0.8
    }
}

/// The emitter's sound walk.
#[derive(Debug, Clone, Copy)]
pub struct SoundState {
    pub volume: f32,
    pub pitch: f32,
    /// The template's base volume.
    pub base: f32,
}

impl SoundState {
    fn new(volume: f32) -> Self {
        Self {
            volume: 1.0,
            pitch: 1.0,
            base: volume,
        }
    }

    /// One frame of the volume and pitch walks.
    fn frame<R: Draws>(&mut self, density: f32, emitter_flags: u32, rng: &mut R) {
        if emitter_flags & flags::SOUND_MOTION == 0 {
            self.volume += self.base * 0.05 * (rng.unit() - 0.5);
            self.volume = if self.base * 0.8 <= self.volume {
                self.volume.min(self.base * 1.2)
            } else {
                self.base * 0.8
            };
        }
        if emitter_flags & flags::ONE_SHOT != 0 {
            self.volume *= density;
        }
        if emitter_flags & flags::SOUND_WIDE_PITCH == 0 {
            self.pitch += (rng.unit() - 0.5) * 0.005;
            self.pitch = if self.pitch >= 0.85 {
                self.pitch.min(1.15)
            } else {
                0.85
            };
        } else {
            self.pitch += (rng.unit() - 0.5) * 0.05;
            self.pitch = if self.pitch >= 0.6 {
                self.pitch.min(1.4)
            } else {
                0.6
            };
        }
    }
}

/// The emitter's bounding box: centre, half extent (with the original's
/// 5 cm padding) and radius².
#[derive(Debug, Clone, Copy, Default)]
pub struct Bounds {
    pub centre: Vec3,
    pub extent: Vec3,
    pub radius_squared: f32,
}

/// What one tick did.
#[derive(Debug, Clone, Copy, Default)]
pub struct TickStats {
    /// Particles that died and were popped.
    pub died: usize,
    /// Particles spawned.
    pub spawned: usize,
    /// Live particles after the tick.
    pub live: usize,
}

/// The emitter state, at the original's tick.
#[derive(Debug, Clone)]
pub struct Emitter {
    pub params: Params,
    pub node: NodeFrame,
    /// The node's position last tick.
    pub last_node: Vec3,
    /// The template's size: the fire presets store their fire size here,
    /// and the ageing's distance cap and the renderer use it.
    pub size: f32,
    /// A body carrying the emitter, if any.
    pub body: Option<BodyMotion>,
    slots: Vec<Slot>,
    first: usize,
    next: usize,
    sources: Vec<Source>,
    accumulator: f32,
    pub light: LightState,
    pub sound: SoundState,
    pub bounds: Bounds,
    /// Drawn; a finished one-shot emitter is hidden.
    pub visible: bool,
    /// The emitter's own flags word (the original's `+0x220`).
    pub emitter_flags: u32,
    /// Ticks run, for finitely lived emitters.
    pub ticks: i32,
    /// The tick a finitely lived emitter stops at (relative to its start).
    pub deadline: i32,
    /// Body contacts counted since the last body pass.
    pub hits: u32,
    /// Flat-colour particles' opacity (the material's, 0–1).
    pub flat_opacity: f32,
    smoothed: Vec3,
    motion_squared: f32,
    motion_average: f32,
}

impl Emitter {
    /// An emitter for a scaled template and its sources.
    pub fn new(template: Descriptor, size: f32, sources: Vec<Source>) -> Self {
        let params = Params::new(template, sources.len().max(1));
        let slots = vec![Slot::default(); params.capacity + 1];
        Self {
            params,
            node: NodeFrame::default(),
            last_node: Vec3::ZERO,
            size,
            body: None,
            slots,
            first: 0,
            next: 0,
            sources,
            accumulator: 0.0,
            light: LightState::new(params.light),
            sound: SoundState::new(template.volume),
            bounds: Bounds::default(),
            visible: true,
            emitter_flags: 0x4600,
            ticks: 0,
            deadline: i32::MAX,
            hits: 0,
            flat_opacity: 1.0,
            smoothed: Vec3::ZERO,
            motion_squared: 0.0,
            motion_average: 0.0,
        }
    }

    /// The sources.
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }

    /// The live slots' index ranges, oldest first.
    pub fn ranges(&self) -> Vec<(usize, usize)> {
        let cap = self.params.capacity;
        if self.first == self.next {
            Vec::new()
        } else if self.first < self.next {
            vec![(self.first, self.next - 1)]
        } else if self.next == 0 {
            vec![(self.first, cap)]
        } else {
            vec![(self.first, cap), (0, self.next - 1)]
        }
    }

    /// Live particles.
    pub fn live(&self) -> usize {
        self.ranges()
            .iter()
            .map(|&(a, b)| self.slots[a..=b].iter().filter(|s| s.life > 0).count())
            .sum()
    }

    /// Particles alive with their slot index, in ring order.
    pub fn particles(&self) -> impl Iterator<Item = (usize, Vec3, Vec3, i32)> + '_ {
        self.ranges().into_iter().flat_map(move |(a, b)| {
            (a..=b).filter_map(move |i| {
                let s = &self.slots[i];
                (s.life > 0).then_some((i, s.p, s.q, s.life))
            })
        })
    }

    /// Whether a given slot is drawn (for the print of the ring).
    pub fn slot_visible(&self, i: usize) -> bool {
        self.slots.get(i).is_some_and(|s| s.life > 0)
    }

    /// One simulation tick.
    pub fn tick<R: Draws>(&mut self, node: NodeFrame, rng: &mut R) -> TickStats {
        self.node = node;
        self.ticks = self.ticks.saturating_add(1);
        let p = self.params;
        if p.light.is_some() {
            let live = self.live();
            self.light.tick(live, p.density_count, rng);
        }
        let mut died = 0;
        if p.start_life > 0 {
            if p.flags & flags::FIRE_AGEING != 0 {
                self.age(rng);
            } else {
                for (a, b) in self.ranges() {
                    for s in &mut self.slots[a..=b] {
                        if s.life > 0 {
                            s.life -= 1;
                        }
                    }
                }
            }
            // Pop the dead from the head, until the ring is empty.
            while self.slots[self.first].life < 1 && self.first != self.next {
                self.first += 1;
                if self.params.capacity < self.first {
                    self.first = 0;
                }
                died += 1;
                if died > self.params.capacity + 1 {
                    break;
                }
            }
            if p.flags & flags::BURST != 0 && died > 1 {
                self.accumulator -= (died.min(4) - 1) as f32 * p.interval;
            }
        }
        if p.flags & flags::FIRE_AGEING == 0 {
            self.last_node = self.node.position;
        }
        for (a, b) in self.ranges() {
            for i in a..=b {
                self.integrate(i);
            }
        }
        if p.flags & flags::FINITE != 0 && self.deadline < self.ticks {
            self.visible = false;
            if self.first == self.next {
                self.emitter_flags |= 0x1000;
                return TickStats {
                    died,
                    spawned: 0,
                    live: 0,
                };
            }
            return TickStats {
                died,
                spawned: 0,
                live: self.live(),
            };
        }
        if p.flags & flags::ONE_SHOT != 0
            && self.emitter_flags & 0x1200 == 0
            && self.first == self.next
        {
            self.emitter_flags |= 0x1000;
            self.visible = false;
            return TickStats {
                died,
                spawned: 0,
                live: 0,
            };
        }
        // Two ticks in three lose one of the accumulator.
        if 0.0 < self.accumulator && rng.int(3) != 0 {
            self.accumulator -= 1.0;
        }
        let mut spawned = 0;
        loop {
            if !self.visible || 1.0 <= self.accumulator {
                break;
            }
            for index in 0..self.sources.len() {
                if self.sources[index].count == 0 {
                    continue;
                }
                let world = self.node.position + self.node.rotation * self.sources[index].offset;
                let velocity = match self.body {
                    Some(body) => body.velocity,
                    None => world - self.sources[index].last,
                };
                self.sources[index].last = world;
                self.sources[index].velocity = velocity;
                if self.sources[index].count > 0 {
                    self.sources[index].count -= 1;
                }
                if self.spawn(index, rng) {
                    spawned += 1;
                }
            }
            self.accumulator += p.interval;
        }
        self.update_bounds();
        TickStats {
            died,
            spawned,
            live: self.live(),
        }
    }

    /// The fire's ageing: node motion, sound, a faster decay and a distance
    /// cap.
    fn age<R: Draws>(&mut self, rng: &mut R) {
        let p = self.params;
        let current = self.node.position;
        let body = self.body;
        let delta = match body {
            Some(b) => b.velocity,
            None => self.last_node - current,
        };
        self.smoothed += (delta - self.smoothed) * 0.1;
        self.motion_squared = self.smoothed.length_squared();
        self.motion_average = self.motion_squared * 0.1 + self.motion_average * 0.9;
        if p.flags & flags::SOUND_MOTION != 0 && p.light.is_some() {
            let f = self.motion_squared * 0.014_285_714;
            let f = f * f * 0.75;
            self.sound.volume = if f <= 1.0 { f.max(0.03) } else { 1.0 };
        }
        if let Some(b) = body {
            let offset = if b.full_displacement {
                current - self.last_node
            } else {
                b.velocity * 0.5
            };
            for (a, b) in self.ranges() {
                for s in &mut self.slots[a..=b] {
                    if s.life > 0 {
                        s.p += offset;
                        s.q += offset;
                    }
                }
            }
        }
        // Each live particle loses one extra life tick half the time.
        if p.flags & flags::BURST != 0 {
            for (a, b) in self.ranges() {
                for s in &mut self.slots[a..=b] {
                    if s.life > 0 && rng.int(2) == 0 {
                        s.life -= 1;
                    }
                }
            }
        }
        let radius = 140.0 * self.size;
        let ratio = if radius != 0.0 {
            p.start_life as f32 / radius
        } else {
            f32::INFINITY
        };
        for (a, b) in self.ranges() {
            for i in a..=b {
                if self.slots[i].life <= 0 {
                    continue;
                }
                let mut d = self.slots[i].p - current;
                d.y *= 0.7;
                let cap = ((radius - d.length()) * ratio).round_ties_even() as i32;
                if cap < self.slots[i].life {
                    self.slots[i].life = cap;
                } else {
                    self.slots[i].life -= 1;
                }
            }
        }
        self.last_node = current;
    }

    fn integrate(&mut self, i: usize) {
        let p = self.params;
        let node = self.node.position;
        let s = &mut self.slots[i];
        if s.life <= 0 {
            return;
        }
        let mut d = s.p - s.q;
        s.q = s.p;
        d.y += GRAVITY * p.buoyancy;
        s.p += d * p.retention;
        s.p.x += (node.x - s.p.x) * p.centring;
        s.p.z += (node.z - s.p.z) * p.centring;
    }

    /// Spawns one particle from a source.
    fn spawn<R: Draws>(&mut self, source: usize, rng: &mut R) -> bool {
        let p = self.params;
        let slot = self.next;
        let mut life = if p.flags & flags::LIFE_RANDOM != 0 {
            p.start_life - rng.int((p.start_life >> 2).max(0) as u32) as i32
        } else if p.start_life < 1 {
            100
        } else {
            p.start_life
        };
        if p.flags & flags::ONE_SHOT != 0 {
            life = life.min(p.start_life);
        }
        let variant = if p.flags & flags::UV_8 != 0 {
            rng.int(8) as u8
        } else if p.flags & flags::UV_32 != 0 {
            rng.int(32) as u8
        } else {
            0
        };
        let src = self.sources[source];
        let mut at = self.node.position + self.node.rotation * src.offset;
        if 0.0 < p.spawn_radius {
            let angle = std::f32::consts::TAU * rng.unit();
            let u = rng.unit();
            let r = (1.0 - u * u * u * u) * p.spawn_radius;
            at += Vec3::new(angle.cos(), 0.0, -angle.sin()) * r;
        }
        let direction = if p.flags & flags::VELOCITY_MASK == flags::VELOCITY_RANDOM {
            let x = p.spread * 0.001 * (rng.int(2001) as f32 - 1000.0);
            let y = rng.int(1001) as f32 * 0.001;
            let z = p.spread * 0.001 * (rng.int(2001) as f32 - 1000.0);
            self.node.rotation * (src.x_axis * x + src.y_axis * y + src.direction * z)
        } else {
            Vec3::Y
        };
        let mut prev = at - direction * p.retention;
        at += direction * p.retention;
        prev -= src.velocity * 0.3;
        let random_size = self.slots[slot].size;
        self.slots[slot] = Slot {
            p: at,
            q: prev,
            life,
            variant,
            size: random_size,
        };
        self.next += 1;
        if self.params.capacity < self.next {
            self.next = 0;
            self.emitter_flags &= !0x200;
            if p.flags & flags::ONE_SHOT != 0 {
                self.visible = false;
            }
        }
        if p.start_life == 0 && self.first == self.next {
            self.visible = false;
        }
        true
    }

    /// The bounding box of the live particles, padded by 5 cm.
    fn update_bounds(&mut self) {
        if self.params.flags & flags::FIXED_BOUNDS != 0 {
            return;
        }
        if self.first == self.next {
            self.bounds.extent = Vec3::ZERO;
            self.bounds.radius_squared = 0.0;
            return;
        }
        let latest = if self.next == 0 {
            self.params.capacity
        } else {
            self.next - 1
        };
        let mut min = self.slots[latest].p;
        let mut max = min;
        for (a, b) in self.ranges() {
            for s in &self.slots[a..=b] {
                if s.life <= 0 {
                    continue;
                }
                min = min.min(s.p);
                max = max.max(s.p);
            }
        }
        self.bounds.centre = (min + max) * 0.5;
        self.bounds.extent = (max - self.bounds.centre) + Vec3::splat(5.0);
        self.bounds.radius_squared = self.bounds.extent.length_squared();
    }

    /// A body contact pass over a slot range: particles inside the body are
    /// pushed out, lose a life tick, and count toward the emitter's dose.
    pub fn contact(&mut self, body: &mut impl BodyContact, first: usize, last: usize) {
        let p = self.params;
        let last = last.min(self.slots.len().saturating_sub(1));
        for i in first..=last {
            if self.slots[i].life <= 0 {
                continue;
            }
            let s = self.slots[i];
            let age = (p.start_life - s.life) as f32;
            let radius = age * (p.collision_growth / p.start_life.max(1) as f32);
            let local = body.to_local(s.p);
            if local.length_squared() > (radius + body.radius()).powi(2) {
                continue;
            }
            if !body.inside(local, radius) {
                continue;
            }
            let out = body.push_out(local);
            let hit = body.to_world(out);
            let old = s.p;
            let slot = &mut self.slots[i];
            slot.p = hit;
            slot.q += (old - hit) * -0.98;
            slot.life -= 1;
            self.hits = self.hits.saturating_add(1);
        }
    }

    /// The body dose for the contacts counted since the last call.
    pub fn take_dose(&mut self, dose: f32) -> f32 {
        let n = self.hits as f32;
        self.hits = 0;
        n * dose
    }

    /// One frame of the sound walk.
    pub fn sound_frame<R: Draws>(&mut self, rng: &mut R) {
        self.sound.frame(self.light.density, self.params.flags, rng);
    }
}

/// A body the particles can collide with.
pub trait BodyContact {
    /// The body's radius (cm).
    fn radius(&self) -> f32;
    /// A world point in the body's local space.
    fn to_local(&self, point: Vec3) -> Vec3;
    /// A body-local point in world space.
    fn to_world(&self, point: Vec3) -> Vec3;
    /// Whether a body-local point is inside the body.
    fn inside(&mut self, point: Vec3, radius: f32) -> bool;
    /// The nearest surface point to a body-local point inside the body.
    fn push_out(&mut self, point: Vec3) -> Vec3;
}

/// What the renderer needs from a frame: the camera.
#[derive(Debug, Clone, Copy)]
pub struct View {
    pub eye: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
}

/// One particle's quad: four corners and the texture coordinates of each.
#[derive(Debug, Clone, Copy)]
pub struct Quad {
    pub corners: [Vec3; 4],
    pub uv: [Vec2; 4],
    pub colour: Key,
}

/// The eight symmetries of the unit square, as the corner UVs of the
/// particle atlas.
pub const SYMMETRIES: [[[f32; 2]; 4]; 8] = [
    [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
    [[1.0, 0.0], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
    [[1.0, 1.0], [0.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    [[0.0, 1.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]],
    [[1.0, 1.0], [1.0, 0.0], [0.0, 1.0], [0.0, 0.0]],
    [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]],
    [[1.0, 0.0], [1.0, 1.0], [0.0, 0.0], [0.0, 1.0]],
];

impl Emitter {
    /// The quads of the live particles, oldest first, interpolated between
    /// the last two ticks by `alpha`.
    pub fn quads(&self, alpha: f32, view: View) -> Vec<Quad> {
        let p = self.params;
        let centre = if self.bounds.radius_squared > 0.0 {
            self.bounds.centre
        } else {
            self.node.position
        };
        let to_emitter = if p.flags & flags::CAMERA_FORWARD != 0 {
            view.forward
        } else {
            (centre - view.eye).normalize_or(view.forward)
        };
        let mut out = Vec::new();
        for (index, position, previous, life) in self.particles() {
            let at = previous + (position - previous) * alpha;
            let age = 1.0 - (life as f32 - alpha) / p.start_life.max(1) as f32;
            let size = if p.flags & flags::RANDOM_SIZE != 0 {
                self.slots[index].size
            } else {
                p.half_size
            };
            let mode = p.flags & flags::QUAD_MASK;
            let corners = if mode == flags::QUAD_DIAMOND {
                diamond_quad(at, position - previous, size, age, p, to_emitter, view)
            } else if mode == flags::QUAD_STRETCHED {
                stretched_quad(at, size, age, p, to_emitter, view)
            } else {
                screen_quad(at, size, view)
            };
            let uv: [Vec2; 4] = if p.flags & flags::UV_8 != 0 {
                std::array::from_fn(|i| {
                    Vec2::from(SYMMETRIES[(self.slots[index].variant & 7) as usize][i])
                })
            } else if p.flags & flags::UV_32 != 0 {
                let v = self.slots[index].variant & 31;
                let cell = v >> 3;
                let sym = SYMMETRIES[(v & 7) as usize];
                let origin = [(cell & 1) as f32 * 0.5, ((cell >> 1) & 1) as f32 * 0.5];
                std::array::from_fn(|i| {
                    Vec2::new(origin[0] + sym[i][0] * 0.5, origin[1] + sym[i][1] * 0.5)
                })
            } else {
                std::array::from_fn(|i| Vec2::from(SYMMETRIES[0][i]))
            };
            let colour = if p.flags & flags::KEYS_PARITY == flags::KEYS_PARITY {
                p.keys.sample(age, index & 1 == 1)
            } else if p.flags & flags::KEYS != 0 {
                p.keys.sample(age, false)
            } else {
                let f = 1.0 - (life as f32 - alpha) / p.start_life.max(1) as f32;
                let a = ((1.0 - f * f) * 255.0).round_ties_even().clamp(0.0, 255.0) as u8;
                let o = (self.flat_opacity * 255.0)
                    .round_ties_even()
                    .clamp(0.0, 255.0) as u8;
                [a, a, a, o]
            };
            out.push(Quad {
                corners,
                uv,
                colour,
            });
        }
        out
    }
}

/// The velocity-aligned diamond of a flame particle.
fn diamond_quad(
    at: Vec3,
    delta: Vec3,
    size: f32,
    age: f32,
    p: Params,
    to_emitter: Vec3,
    view: View,
) -> [Vec3; 4] {
    let mut w = delta;
    w.y -= p.buoyancy;
    let speed = w.length();
    let (d, x_axis, y_axis, l) = if speed > 0.0 {
        let d = -w / speed;
        let across = d.cross(to_emitter);
        let l = across.length();
        if l > 1.0e-5 {
            let x = across / l;
            let y = across.cross(to_emitter) / l;
            (d, x, y, l)
        } else {
            (d, view.right, view.up, 0.0)
        }
    } else {
        // At rest along the view: the original divides by zero; keep a
        // finite quad of the base size instead.
        (Vec3::NEG_Y, view.right, view.up, 0.0)
    };
    let width = size + l * age * age * p.growth;
    let stretch = if speed > 0.0 {
        (2.0 - 2.0 / (speed + 1.0)) * p.stretch * d.dot(to_emitter).abs().acos()
    } else {
        0.0
    };
    let x = x_axis * width;
    let y = y_axis * ((1.0 + stretch) * width);
    [at - y, at - x, at + x, at + y]
}

/// The screen-aligned, age-stretched quad of an ember bed.
fn stretched_quad(
    at: Vec3,
    size: f32,
    age: f32,
    p: Params,
    to_emitter: Vec3,
    view: View,
) -> [Vec3; 4] {
    let x_axis = to_emitter.cross(view.up).normalize_or(view.right);
    let y_axis = x_axis.cross(to_emitter).normalize_or(view.up);
    let width = size + age * age * p.growth;
    let x = x_axis * width;
    let y = y_axis * ((1.0 + p.stretch) * width);
    [at - y, at - x, at + x, at + y]
}

/// A flat billboard of half-size `size`.
fn screen_quad(at: Vec3, size: f32, view: View) -> [Vec3; 4] {
    let x = view.right * size;
    let y = view.up * size;
    [at - y, at - x, at + x, at + y]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scripted stream: the numbers the test wants, in order.
    struct Scripted {
        ints: Vec<u32>,
        units: Vec<f32>,
    }

    impl Scripted {
        fn new(ints: &[u32], units: &[f32]) -> Self {
            Self {
                ints: ints.to_vec(),
                units: units.to_vec(),
            }
        }
    }

    impl Draws for Scripted {
        fn int(&mut self, n: u32) -> u32 {
            let v = if self.ints.is_empty() {
                0
            } else {
                self.ints.remove(0)
            };
            v % n.max(1)
        }

        fn unit(&mut self) -> f32 {
            if self.units.is_empty() {
                0.0
            } else {
                self.units.remove(0)
            }
        }
    }

    /// The small fire's template, the one a torch uses.
    const SMALL: Descriptor = Descriptor {
        rate: 250,
        life: 1.25,
        half_size: 10.0,
        speed: 0.0,
        retention: 0.0,
        growth: 2.0,
        stretch: 0.6,
        spawn_radius: 35.0,
        spread: 0.0,
        buoyancy: -1.25,
        centring: 0.0007,
        pool: 0,
        flags: 0x420C_0B01,
        keys: Keys {
            count: 9,
            entries: &[
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [32, 13, 3, 16],
                [64, 23, 5, 32],
                [192, 127, 67, 32],
                [255, 160, 94, 64],
                [170, 120, 60, 64],
                [90, 30, 7, 64],
                [48, 12, 3, 80],
                [0, 0, 0, 48],
                [0, 0, 0, 0],
            ],
        },
        light: LightDesc {
            colour: [1.0, 0.68, 0.43],
            intensity: 8.0,
            radius: 1500.0,
            flicker: 1.0,
        },
        sound: Some("FireS02"),
        volume: 0.0,
        density: 450,
        dose: 0.0,
    };

    fn torch() -> Emitter {
        Emitter::new(SMALL.scaled(0.2), 0.2, vec![Source::at_node()])
    }

    #[test]
    fn the_scaling_matches_the_small_template() {
        let d = SMALL.scaled(0.2);
        assert_eq!(d.rate, 50);
        assert!((d.half_size - 6.0).abs() < 1e-6);
        assert!((d.spawn_radius - 4.9).abs() < 1e-6);
        assert!((d.buoyancy - -0.25).abs() < 1e-6);
        assert!((d.light.intensity - 3.577_709).abs() < 1e-5);
        assert!((d.light.radius - 670.820_4).abs() < 1e-3);
        assert_eq!(d.density, 25);
    }

    #[test]
    fn the_derived_values_match_the_torch() {
        let e = torch();
        assert_eq!(e.params.start_life, 103);
        assert!((e.params.interval - 83.0 / 50.0 * (2.0 / 3.0)).abs() < 1e-6);
        assert_eq!(e.params.capacity, 97);
        assert_eq!(e.slots.len(), 98);
        assert_eq!(e.params.density_count, 25);
        assert_eq!(e.params.retention, 1.0);
        assert!((e.params.growth - 12.0).abs() < 1e-6);
        assert!((e.params.collision_growth - 18.0).abs() < 1e-6);
    }

    #[test]
    fn one_spawn_lands_on_the_ring_at_rest() {
        let mut e = torch();
        // Three light draws, then the ring's angle and radius.
        let mut rng = Scripted::new(&[], &[0.0, 0.0, 0.0, 0.5, 0.0]);
        e.tick(NodeFrame::default(), &mut rng);
        let (_, p, q, life) = e.particles().next().expect("one particle");
        assert_eq!(life, 103);
        assert!((p - q).length() < 1e-6);
        // θ = π, u = 0: r = 4.9.
        assert!((p.x + 4.9).abs() < 1e-4, "{p:?}");
        assert!(p.z.abs() < 1e-4);
    }

    #[test]
    fn the_ring_law_puts_particles_near_the_rim() {
        let r = |u: f32| 4.9 * (1.0 - u * u * u * u);
        assert!((r(0.0) - 4.9).abs() < 1e-6);
        assert!((r(0.5) - 4.9 * 0.9375).abs() < 1e-6);
        assert_eq!(r(1.0), 0.0);
    }

    #[test]
    fn the_interval_gives_the_template_rate() {
        let e = torch();
        let actual = (2.0 / 3.0) / e.params.interval;
        assert!((actual - 50.0 / 83.0).abs() < 1e-9);
    }

    #[test]
    fn integration_rises_and_pulls_toward_the_axis() {
        let mut e = torch();
        e.first = 0;
        e.next = 1;
        e.slots[0] = Slot {
            p: Vec3::new(3.0, 10.0, -2.0),
            q: Vec3::new(3.0, 10.0, -2.0),
            life: 100,
            ..Slot::default()
        };
        e.integrate(0);
        let s = e.slots[0];
        // Buoyancy −0.25 of gravity a tick²: 0.0356 cm/tick².
        assert!((s.p.y - (10.0 + 0.25 * 980.665 / (83.0 * 83.0))).abs() < 1e-6);
        assert!(s.p.x < 3.0 && s.p.z > -2.0);
        assert!((s.p.x - (3.0 - 3.0 * 0.0007)).abs() < 1e-6);
    }

    #[test]
    fn the_distance_cap_kills_a_particle_off_the_axis() {
        let mut e = torch();
        e.first = 0;
        e.next = 1;
        e.slots[0] = Slot {
            p: Vec3::new(0.0, 35.0, 0.0),
            q: Vec3::new(0.0, 35.0, 0.0),
            life: 103,
            ..Slot::default()
        };
        let mut rng = Scripted::new(&[1], &[]);
        e.age(&mut rng);
        // D = 24.5, cap = round((28 − 24.5) · 103/28) = round(12.875) = 13.
        assert_eq!(e.slots[0].life, 13);
    }

    #[test]
    fn the_extra_ageing_is_half_the_ticks() {
        let mut e = torch();
        e.first = 0;
        e.next = 1;
        e.slots[0] = Slot {
            p: Vec3::ZERO,
            q: Vec3::ZERO,
            life: 103,
            ..Slot::default()
        };
        // The extra-tick draw is 0: it loses one, then the normal one.
        let mut rng = Scripted::new(&[0], &[]);
        e.age(&mut rng);
        assert_eq!(e.slots[0].life, 101);
    }

    #[test]
    fn the_key_list_fades_in_and_out() {
        let k = SMALL.scaled(0.2).keys;
        assert_eq!(k.sample(0.0, false), [0, 0, 0, 0]);
        assert_eq!(k.sample(1.0, true)[3], 0);
        let c = k.sample(4.0 / 9.0, true);
        assert_eq!(c[3], 64);
        assert_eq!(c[0], 255);
    }

    #[test]
    fn the_colour_lerp_truncates() {
        let keys = Keys {
            count: 1,
            entries: &[[10, 20, 30, 40], [110, 220, 130, 240]],
        };
        assert_eq!(keys.sample(0.5, false), [60, 120, 80, 140]);
        let keys = Keys {
            count: 1,
            entries: &[[0, 0, 0, 0], [3, 3, 3, 3]],
        };
        // 1.89 truncates to 1.
        assert_eq!(keys.sample(0.63, false), [1, 1, 1, 1]);
    }

    #[test]
    fn the_light_walk_stays_bounded() {
        let mut e = torch();
        let mut rng = Rng::new(7);
        for _ in 0..10_000 {
            let live = e.live();
            e.light.tick(live, e.params.density_count, &mut rng);
            assert!(e.light.walk.abs() < 2.5);
            assert!((0.87..=1.15).contains(&e.light.jitter));
        }
    }

    #[test]
    fn the_light_density_is_eighty_twenty() {
        let mut e = torch();
        e.light.density = 1.0;
        let mut rng = Scripted::new(&[], &[0.0, 0.0, 0.0]);
        e.light.tick(0, 25, &mut rng);
        assert!((e.light.density - 0.8).abs() < 1e-6);
    }

    #[test]
    fn the_quad_is_stretched_along_the_motion() {
        let mut e = torch();
        e.first = 0;
        e.next = 1;
        e.slots[0] = Slot {
            p: Vec3::new(0.0, 3.0, 0.0),
            q: Vec3::new(0.0, 1.0, 0.0),
            life: 50,
            ..Slot::default()
        };
        let view = View {
            eye: Vec3::new(100.0, 0.0, 0.0),
            right: Vec3::Y,
            up: Vec3::Y,
            forward: Vec3::NEG_X,
        };
        let quads = e.quads(0.0, view);
        assert_eq!(quads.len(), 1);
        let c = quads[0].corners;
        assert!((c[3].y - c[0].y).abs() > (c[2].x - c[1].x).abs());
        assert!((c[3].y - 3.0).abs() > 6.0);
    }

    #[test]
    fn the_texture_variants_cover_four_cells_and_eight_symmetries() {
        let mut e = torch();
        e.params.flags = (e.params.flags | flags::UV_32) & !flags::UV_8;
        e.first = 0;
        e.next = 1;
        e.slots[0] = Slot {
            p: Vec3::ZERO,
            q: Vec3::new(0.0, -1.0, 0.0),
            life: 50,
            variant: 9,
            ..Slot::default()
        };
        let view = View {
            eye: Vec3::new(0.0, 0.0, 100.0),
            right: Vec3::X,
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
        };
        let uv = e.quads(0.0, view)[0].uv;
        // Variant 9: cell 1 (origin 0.5, 0), symmetry 1.
        assert_eq!(uv[0], Vec2::new(1.0, 0.0));
    }

    #[test]
    fn a_finite_emitter_hides_after_its_deadline() {
        let mut d = SMALL.scaled(0.2);
        d.flags |= flags::FINITE;
        let mut e = Emitter::new(d, 0.2, vec![Source::at_node()]);
        e.deadline = 5;
        let mut rng = Scripted::new(&[], &[]);
        for _ in 0..6 {
            e.tick(NodeFrame::default(), &mut rng);
        }
        assert!(!e.visible);
    }

    #[test]
    fn the_body_contact_pushes_a_particle_out_and_counts_the_dose() {
        struct Ball;
        impl BodyContact for Ball {
            fn radius(&self) -> f32 {
                20.0
            }
            fn to_local(&self, p: Vec3) -> Vec3 {
                p
            }
            fn to_world(&self, p: Vec3) -> Vec3 {
                p
            }
            fn inside(&mut self, p: Vec3, _r: f32) -> bool {
                p.length() < 20.0
            }
            fn push_out(&mut self, p: Vec3) -> Vec3 {
                p.normalize_or(Vec3::Y) * 20.0
            }
        }
        let mut e = torch();
        e.first = 0;
        e.next = 1;
        e.slots[0] = Slot {
            p: Vec3::new(1.0, 0.0, 0.0),
            q: Vec3::ZERO,
            life: 103,
            ..Slot::default()
        };
        e.contact(&mut Ball, 0, 0);
        assert!((e.slots[0].p.length() - 20.0).abs() < 1e-5);
        assert_eq!(e.slots[0].life, 102);
        assert!((e.take_dose(0.25) - 0.25).abs() < 1e-6);
        assert_eq!(e.hits, 0);
    }

    #[test]
    fn a_ring_keeps_its_sources_apart() {
        let mut rng = Scripted::new(&[], &[0.0, 0.4, 0.40, 0.8, 0.0, 0.25, 0.5, 0.99, 0.5, 0.5]);
        let sources = Source::ring(4, 40.0, &mut rng);
        assert_eq!(sources.len(), 4);
        for s in &sources {
            let r = s.offset.length();
            assert!((20.0..=40.0).contains(&r), "{r}");
        }
    }
}
