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
//! The arithmetic follows the original's instructions: the values it works
//! out in double precision are worked out in double here, its rounding
//! conversions round half to even and its truncating ones truncate, and its
//! vector lengths add the components in its order.
//!
//! Units are the original's: centimetres, seconds for the template and
//! ticks for the simulation. Rules: `docs/RENDERING.md` (Fires).

use bevy::prelude::{Mat3, Vec2, Vec3};

/// The simulation's tick rate.
pub const TICK_HZ: f32 = 83.0;
/// Gravity per tick² (cm): the environment's (0, −980.665, 0) times the
/// reciprocal of the squared tick rate, as the original builds it.
pub const GRAVITY: f32 = -980.665 * (1.0 / (TICK_HZ * TICK_HZ));

/// Behaviour flags of an emitter (the template's, copied to the emitter).
pub mod flags {
    /// Low two bits: 1 = random initial velocity from the template's spread.
    pub const VELOCITY_MASK: u32 = 3;
    /// Random initial velocity mode.
    pub const VELOCITY_RANDOM: u32 = 1;
    /// Start each particle's life shortened by up to a quarter.
    pub const LIFE_RANDOM: u32 = 4;
    /// One-shot: the emitter stops when its ring wraps and finishes when
    /// the ring empties.
    pub const ONE_SHOT: u32 = 0x10;
    /// Stop the emitter once its deadline passes.
    pub const FINITE: u32 = 0x20;
    /// Keep the bounding box instead of updating it.
    pub const FIXED_BOUNDS: u32 = 0x40;
    /// Spawn velocity and smoothed motion from the camera's forward axis.
    pub const CAMERA_FORWARD: u32 = 0x80;
    /// The screen and stretched quads take the camera's axes through the
    /// node's own frame, so they billboard with the node rather than the
    /// camera (the flat-colour beds ask for it).
    pub const NODE_AXES: u32 = 0x40;
    /// The screen and stretched quads keep a tenth of their offset along the
    /// node's up axis, flattening them onto the node's horizontal plane
    /// (the medium-small bed asks for it).
    pub const VIEW_PLANE: u32 = 0x1000;
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
    pub const SOUND_MOTION: u32 = 0x0200_0000;
    /// Sound pitch takes the wide random walk.
    pub const SOUND_WIDE_PITCH: u32 = 0x0400_0000;
    /// Quad mode, the top nibble.
    pub const QUAD_MASK: u32 = 0xf000_0000;
    /// Velocity-aligned diamond (the flames' quads).
    pub const QUAD_DIAMOND: u32 = 0x4000_0000;
    /// Screen-aligned quad with the age-based size (the ember beds).
    pub const QUAD_STRETCHED: u32 = 0x1000_0000;
}

/// The emitter's own state bits (the original's run-time flag word).
pub mod state {
    /// The value an emitter starts with.
    pub const INITIAL: u32 = 0x4600;
    /// Cleared when the ring wraps for the first time.
    pub const FIRST_PASS: u32 = 0x200;
    /// The emitter has finished; the original removes its node.
    pub const FINISHED: u32 = 0x1000;
}

/// Random draws in the original's order. The integer stream is the
/// engine's linear congruential sequence (`seed · 0x08088405 + 1`, a draw
/// in `[0, n)` being `(seed · n) >> 32`); the unit stream is the run-time
/// library's Mersenne twister scaled by 2⁻³², in double precision. Both are
/// process-wide in the original and seeded from the clock, so only the
/// generators, the number and the order of draws are reproduced.
pub trait Draws {
    /// An integer in `[0, n)` from the linear congruential stream.
    fn int(&mut self, n: u32) -> u32;
    /// A double in `[0, 1)` from the Mersenne twister.
    fn unit(&mut self) -> f64;
}

/// The run-time library's MT19937 generator, standard seeding.
#[derive(Debug, Clone)]
pub struct Mt19937 {
    state: Box<[u32; 624]>,
    index: usize,
}

impl Mt19937 {
    pub fn new(seed: u32) -> Self {
        let mut state = Box::new([0u32; 624]);
        state[0] = seed;
        for i in 1..624 {
            state[i] = 1_812_433_253u32
                .wrapping_mul(state[i - 1] ^ (state[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        Self { state, index: 624 }
    }

    fn twist(&mut self) {
        for i in 0..624 {
            let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % 624] & 0x7fff_ffff);
            let mut next = self.state[(i + 397) % 624] ^ (y >> 1);
            if y & 1 != 0 {
                next ^= 0x9908_b0df;
            }
            self.state[i] = next;
        }
        self.index = 0;
    }

    /// The next tempered 32-bit output.
    pub fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }
}

/// The default streams: the engine's integer LCG and the run-time
/// library's twister.
#[derive(Debug, Clone)]
pub struct Rng {
    ints: u32,
    floats: Mt19937,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self {
            ints: seed,
            floats: Mt19937::new(seed.wrapping_mul(0x9E37_79B9)),
        }
    }
}

impl Draws for Rng {
    fn int(&mut self, n: u32) -> u32 {
        self.ints = self.ints.wrapping_mul(0x0808_8405).wrapping_add(1);
        ((u64::from(self.ints) * u64::from(n)) >> 32) as u32
    }

    fn unit(&mut self) -> f64 {
        f64::from(self.floats.next_u32()) * (1.0 / 4_294_967_296.0)
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

/// A colour key list: the count `n` and its `n + 2` keys (K1 at index 0).
/// The original stores the count as the list's first byte and the keys
/// after it.
#[derive(Debug, Clone, Copy)]
pub struct Keys {
    pub count: i32,
    pub entries: &'static [Key],
}

impl Keys {
    /// The colour at key position `f` (the count times the normalised age);
    /// odd ring slots start one key later. Each channel is lerped in single
    /// precision and rounded half to even, saturated to a byte.
    pub fn sample(&self, f: f32, odd: bool) -> Key {
        let k = f as i32;
        let frac = f - k as f32;
        let first = k + 1 + i32::from(odd);
        let a = self.entry(first);
        let b = self.entry(first + 1);
        std::array::from_fn(|c| {
            let a = f32::from(a[c]);
            let v = a + (f32::from(b[c]) - a) * frac;
            v.round_ties_even().clamp(0.0, 255.0) as u8
        })
    }

    fn entry(&self, i: i32) -> Key {
        usize::try_from(i - 1)
            .ok()
            .and_then(|i| self.entries.get(i))
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
    /// and the density normaliser (the integers rounded half to even).
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
    /// The reciprocal of the lifetime, the age's scale when drawing.
    pub inverse_life: f32,
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
    /// Velocity kept per tick.
    pub retention: f32,
    /// A one-shot template's ring deadline: the tick its first particle's
    /// life plus the ring's span reaches (rounded half to even), or
    /// `i32::MAX` for other templates. The original stores it for the game
    /// to read; the tick itself does not.
    pub deadline: i32,
    /// Ring capacity, excluding the spare slot.
    pub capacity: usize,
    /// Live count the density divides by.
    pub density_count: i32,
    pub keys: Keys,
    pub light: LightDesc,
}

impl Params {
    fn new(d: Descriptor, sources: usize) -> Self {
        // 83 / rate in double, stored, then times 2/3 in double.
        let interval = (f64::from(TICK_HZ) / f64::from(d.rate)) as f32;
        let interval = (f64::from(interval) * (2.0 / 3.0)) as f32;
        let start_life = (TICK_HZ * d.life) as i32;
        let capacity = if d.pool < 1 {
            let per_source = (f64::from(d.rate as f32 * d.life * 1.5) + 0.501 + 1.0) as i64;
            (sources as i64 * per_source + 2).max(1) as usize
        } else {
            d.pool as usize
        };
        // A record from an untrusted file cannot ask for an unbounded pool.
        let capacity = capacity.min(1 << 20);
        // A one-shot emitter also records when it finishes: the ring's whole
        // span plus its first particle's life and a half, rounded half to
        // even (the original's conversion of a single).
        let deadline = if d.flags & flags::ONE_SHOT != 0 {
            let ticks = start_life as f32 + capacity as f32 * interval + 0.5;
            ticks.round_ties_even() as i32
        } else {
            i32::MAX
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
        let retention = if d.retention > 0.0 {
            (f64::from(d.retention).ln() / f64::from(TICK_HZ)).exp() as f32
        } else {
            1.0
        };
        let stretch = if d.stretch > 1.192_092_9e-7 {
            d.stretch
        } else {
            1.0
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
            inverse_life: 1.0 / start_life as f32,
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
            deadline,
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

/// A spawn source: a frame (three axes and an origin) in the emitter
/// node's space. A random spawn velocity is `x·axes[0] + y·axes[1] +
/// z·axes[2]`, `y` being the template's upward component.
#[derive(Debug, Clone, Copy)]
pub struct Source {
    /// The frame's axes, in the node's space.
    pub axes: [Vec3; 3],
    /// The frame's origin, relative to the node.
    pub offset: Vec3,
    /// A pull back along the second axis applied to each new particle; the
    /// fire presets leave it at zero.
    pub kick: f32,
    /// Spawns left; negative is unlimited.
    pub count: i32,
    /// The source's world origin last tick.
    pub last: Vec3,
    /// The source's world velocity, cm per tick.
    pub velocity: Vec3,
    /// The camera-forward spawn's smoothed velocity.
    pub smoothed: Vec3,
}

impl Source {
    /// The default source: the identity frame at the node.
    pub fn at_node() -> Self {
        Self {
            axes: [Vec3::X, Vec3::Y, Vec3::Z],
            offset: Vec3::ZERO,
            kick: 0.0,
            count: -1,
            last: Vec3::ZERO,
            velocity: Vec3::ZERO,
            smoothed: Vec3::ZERO,
        }
    }

    /// A ring of `count` sources. The angles are drawn first, each at least
    /// π/count from every earlier one, then each source's radius (0.5–1 of
    /// `radius`), in the original's order. A source's second axis points
    /// out from the ring's centre, its first is that axis crossed with +Z
    /// and its third the first crossed with the second.
    pub fn ring<R: Draws>(count: usize, radius: f32, rng: &mut R) -> Vec<Source> {
        let n = count.max(1);
        let tau = std::f32::consts::TAU;
        let min_gap = std::f32::consts::PI / n as f32;
        let mut angles = vec![0.0f32; n];
        for slot in 0..n {
            loop {
                let angle = (f64::from(tau) * rng.unit()) as f32;
                let clash = angles[..slot]
                    .iter()
                    .any(|&other| (angle - other).abs() < min_gap);
                if !clash {
                    angles[slot] = angle;
                    break;
                }
            }
        }
        let mut out = Vec::with_capacity(n);
        for angle in angles {
            let (sin, cos) = angle.sin_cos();
            let out_dir = Vec3::new(cos, 0.0, -sin);
            let r = (f64::from(radius) * (rng.unit() * 0.5 + 0.5)) as f32;
            let second = out_dir.normalize_or(Vec3::X);
            let first = second.cross(Vec3::Z).normalize_or(Vec3::Y);
            let third = first.cross(second);
            out.push(Source {
                axes: [first, second, third],
                offset: out_dir * r,
                ..Source::at_node()
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
    /// Atlas cell (bits 3–4) and square symmetry (bits 0–2).
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
    /// A full light update is due next frame (the emitter starts asking for
    /// one).
    pub full: bool,
}

/// The light a frame asks for.
#[derive(Debug, Clone, Copy, Default)]
pub struct LightFrame {
    /// The intensity the light has this frame.
    pub intensity: f32,
    /// Its shadow softness.
    pub softness: f32,
    /// The emissive scale an emissive parent mesh gets.
    pub emissive: f32,
    /// The light needs a full update this frame (its first, a one-shot
    /// emitter's, or a brighter one than ever before).
    pub full_update: bool,
    /// The light has reached its template's intensity.
    pub at_base: bool,
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
            full: true,
        }
    }

    /// One tick of the flicker walk, then the density from the live count
    /// (the original works these out in double precision).
    fn tick<R: Draws>(&mut self, live: usize, normaliser: i32, rng: &mut R) {
        let speed = f64::from(self.speed);
        self.speed = if 1.0 < self.walk || 1.0 < self.speed {
            (speed - f64::from(self.step) * rng.unit()) as f32
        } else if self.walk < -1.0 || self.speed < -1.0 {
            (f64::from(self.step) * rng.unit() + speed) as f32
        } else {
            (f64::from(self.step + 1.0) * (rng.unit() - 0.5) + speed) as f32
        };
        self.walk = (f64::from(self.speed) * 0.3 * rng.unit() + f64::from(self.walk)) as f32;
        self.jitter = ((rng.unit() - 0.5) * 0.125 + f64::from(self.jitter)) as f32;
        if f64::from(self.jitter) < 0.87 {
            self.jitter = 0.87;
        } else if f64::from(self.jitter) > 1.15 {
            self.jitter = 1.15;
        }
        let share = (live as f64 / f64::from(normaliser)) as f32;
        self.density = (f64::from(share) * 0.2 + f64::from(self.density) * 0.8) as f32;
    }

    /// The light's shadow softness this frame.
    pub fn softness(&self) -> f32 {
        self.softness * self.jitter
    }

    /// One frame of the light: the intensity, the softness, the emissive
    /// scale of an emissive parent and whether a full update is due. The
    /// original works the two scales out in double precision, the intensity
    /// as the density times the flicker times the template's intensity.
    pub fn frame(&mut self, template_flags: u32) -> LightFrame {
        let intensity = (f64::from(self.density)
            * ((f64::from(self.walk) * 0.05 + 1.0) * f64::from(self.base)))
            as f32;
        let brighter = intensity > self.peak;
        let full_update = self.full || template_flags & flags::ONE_SHOT != 0 || brighter;
        if full_update && brighter {
            self.peak = (f64::from(intensity) * 1.1) as f32;
        }
        // The light's "not yet at rest" bit clears once it reaches the
        // template's intensity (the first frame is the only one that has it
        // set, since a one-shot template and a new peak ask for the full
        // update again).
        let at_base = self.base <= intensity;
        if at_base {
            self.full = false;
        }
        let emissive = (f64::from(self.density) * (f64::from(self.walk) * 0.2 + 0.9) * 0.8) as f32;
        LightFrame {
            intensity,
            softness: self.softness(),
            emissive,
            full_update,
            at_base,
        }
    }
}

/// One node of the scene a fire's emissive scale reaches: the node's own
/// scale and its first sub-object's material scale.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EmissiveNode {
    /// The node's emissive scale.
    pub node: f32,
    /// The first sub-object's material emissive scale.
    pub material: f32,
    /// A light node (its kind's low bits are 9).
    pub light: bool,
}

/// The emitter's emissive scale reaches its node's parent and every light
/// child of it, as the original's per-frame pass sets them: the parent
/// node's own scale and the first sub-object's material scale, then the same
/// pair on each light child. Returns the value set.
pub fn apply_emissive(value: f32, parent: &mut EmissiveNode, children: &mut [EmissiveNode]) -> f32 {
    parent.node = value;
    parent.material = value;
    for child in children {
        if child.light {
            child.node = value;
            child.material = value;
        }
    }
    value
}

/// A node an emitter hangs under: the scene flags the attach sets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AttachedNode {
    /// The node's flag word: it has a child.
    pub flags: u32,
    /// The node's render word.
    pub render: u32,
    /// The first sub-object's material flag word.
    pub material_flags: u32,
    /// Whether the node already had a non-zero frame when the emitter
    /// attached.
    pub framed: bool,
}

/// The child bit an emitter's node gains.
pub const NODE_HAS_CHILD: u32 = 0x1_0000;
/// The render bit a node with an emitter child gains.
pub const NODE_DRAWS_EMITTER: u32 = 0x20;
/// The material flag a sub-object drawn by an emitter gains.
pub const MATERIAL_EMITTER: u32 = 0x20;

/// Attaches an emitter to its node: the node gains the child bit, the
/// emitter is the node's child, the node and the emitter's first sub-object
/// draw through the emitter, and the emitter takes the node's frame. The
/// original also links the scene node here; the Bevy hierarchy is that link
/// in Exarust.
pub fn attach_node(node: &mut AttachedNode) {
    node.flags |= NODE_HAS_CHILD;
    node.render |= NODE_DRAWS_EMITTER;
    node.material_flags |= MATERIAL_EMITTER;
    node.framed = true;
}

/// The emitter's sound walk.
#[derive(Debug, Clone, Copy)]
pub struct SoundState {
    pub volume: f32,
    pub pitch: f32,
    /// The template's base volume.
    pub base: f32,
    /// Whether the template names a sound (the original only walks a sound
    /// it could create).
    pub attached: bool,
}

impl SoundState {
    fn new(volume: f32, attached: bool) -> Self {
        Self {
            volume: 1.0,
            pitch: 1.0,
            base: volume,
            attached,
        }
    }

    /// One frame of the volume and pitch walks.
    fn frame<R: Draws>(&mut self, density: f32, template_flags: u32, rng: &mut R) {
        if !self.attached {
            return;
        }
        if template_flags & flags::SOUND_MOTION == 0 {
            self.volume =
                (f64::from(self.base) * 0.05 * (rng.unit() - 0.5) + f64::from(self.volume)) as f32;
            // The bounds are worked out and compared in double precision; a
            // NaN volume fails both unordered compares and is kept.
            let low = f64::from(self.base) * 0.8;
            let high = f64::from(self.base) * 1.2;
            let volume = f64::from(self.volume);
            if volume < low {
                self.volume = low as f32;
            } else if high < volume {
                self.volume = high as f32;
            }
        }
        if template_flags & flags::ONE_SHOT != 0 {
            self.volume *= density;
        }
        let (step, low, high) = if template_flags & flags::SOUND_WIDE_PITCH == 0 {
            (0.005, 0.85, 1.15)
        } else {
            (0.05, 0.6, 1.4)
        };
        self.pitch = ((rng.unit() - 0.5) * step + f64::from(self.pitch)) as f32;
        // The bounds compare in double precision.
        let pitch = f64::from(self.pitch);
        if pitch < low {
            self.pitch = low as f32;
        } else if high < pitch {
            self.pitch = high as f32;
        }
    }
}

/// The emitter's bounding box: centre, half extent (with the original's
/// 5 cm padding) and radius.
#[derive(Debug, Clone, Copy, Default)]
pub struct Bounds {
    pub centre: Vec3,
    pub extent: Vec3,
    pub radius: f32,
}

/// What one tick did.
#[derive(Debug, Clone, Copy, Default)]
pub struct TickStats {
    /// The original's count of the head pops (it counts one more when the
    /// pops empty the ring onto a dead slot).
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
    /// and the ageing's distance cap uses it.
    pub size: f32,
    /// The emitter's detail value, which the renderer's level of detail
    /// scales by: the fire presets set it to twice a bed's size and, for
    /// some flames, twice the fire's.
    pub detail: f32,
    /// A body carrying the emitter, if any.
    pub body: Option<BodyMotion>,
    /// The frame of the body carrying the emitter: its sources and particles
    /// ride this instead of the node (the original's body-attached emitters).
    pub body_frame: Option<NodeFrame>,
    /// The looping sound a template names (a single-letter name is no
    /// sound, as the original checks the string's length).
    pub sound_name: Option<&'static str>,
    slots: Vec<Slot>,
    first: usize,
    next: usize,
    sources: Vec<Source>,
    accumulator: f32,
    pub light: LightState,
    pub sound: SoundState,
    pub bounds: Bounds,
    /// Still emitting. A one-shot emitter stops when its ring wraps, a
    /// finite one when its deadline passes.
    pub active: bool,
    /// The emitter's own state word ([`state`]).
    pub emitter_flags: u32,
    /// Ticks run, for finitely lived emitters.
    pub ticks: i32,
    /// The tick a finitely lived emitter stops at (the original keeps a
    /// millisecond clock; no preset sets the finite flag).
    pub deadline: i32,
    /// Body contacts counted since the last body pass.
    pub hits: u32,
    /// Flat-colour particles' opacity (the material's, 0–1).
    pub flat_opacity: f32,
    entered: bool,
    smoothed: Vec3,
    motion_squared: f32,
    motion_average: f32,
}

impl Emitter {
    /// An emitter for a scaled template and its sources.
    pub fn new(template: Descriptor, size: f32, sources: Vec<Source>) -> Self {
        let mut rng = Rng::new((template.flags ^ template.rate as u32) | 1);
        Self::with_draws(template, size, sources, &mut rng)
    }

    /// An emitter whose random size table draws from `rng`, as the original
    /// fills it when the emitter enters the world.
    pub fn with_draws<R: Draws>(
        template: Descriptor,
        size: f32,
        sources: Vec<Source>,
        rng: &mut R,
    ) -> Self {
        let sources = if sources.is_empty() {
            vec![Source::at_node()]
        } else {
            sources
        };
        let params = Params::new(template, sources.len());
        let mut slots = vec![Slot::default(); params.capacity + 1];
        if params.flags & flags::RANDOM_SIZE != 0 {
            for slot in &mut slots {
                slot.size = (f64::from(params.half_size) * (rng.unit() * 0.5 + 0.5)) as f32;
            }
        }
        let sound = template.sound.is_some_and(|name| name.len() > 1);
        let mut emitter_flags = state::INITIAL;
        if sound {
            emitter_flags |= template.flags & flags::SOUND_MOTION;
        }
        Self {
            params,
            node: NodeFrame::default(),
            last_node: Vec3::ZERO,
            size,
            detail: size,
            body: None,
            body_frame: None,
            sound_name: template.sound,
            slots,
            first: 0,
            next: 0,
            sources,
            accumulator: 0.0,
            light: LightState::new(params.light),
            sound: SoundState::new(template.volume, sound),
            bounds: Bounds::default(),
            active: true,
            emitter_flags,
            ticks: 0,
            deadline: i32::MAX,
            hits: 0,
            flat_opacity: 1.0,
            entered: false,
            smoothed: Vec3::ZERO,
            motion_squared: 0.0,
            motion_average: 0.0,
        }
    }

    /// The sources.
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }

    /// Whether the emitter has finished: a one-shot ring that emptied, or a
    /// finite emitter past its deadline with no particle left.
    pub fn finished(&self) -> bool {
        self.emitter_flags & state::FINISHED != 0
    }

    /// The emitter enters the world at its node: the motion it measures and
    /// its sources' last positions start from here (the original's
    /// insertion into the simulation).
    pub fn enter(&mut self, node: NodeFrame) {
        self.node = node;
        self.last_node = node.position;
        self.bounds.centre = node.position;
        self.smoothed = Vec3::ZERO;
        self.motion_squared = 0.0;
        self.motion_average = 0.0;
        for source in &mut self.sources {
            source.last = node.position + node.rotation * source.offset;
        }
        self.entered = true;
    }

    /// Hangs the emitter on a body: its sources and particles ride the
    /// body's frame from now on, and the fire's ageing (the distance cap)
    /// turns on. The original takes the frame from the body's matrix each
    /// tick; call this again when the body moves.
    pub fn attach_body(&mut self, frame: NodeFrame) {
        self.body_frame = Some(frame);
        self.params.flags |= flags::FIRE_AGEING;
        // The sources' last world positions start on the body, so the first
        // spawn does not inherit the jump from the node to the body.
        for source in &mut self.sources {
            source.last = frame.position + frame.rotation * source.offset;
        }
    }

    /// The live slots' index ranges, oldest first, each inclusive.
    pub fn ranges(&self) -> [Option<(usize, usize)>; 2] {
        let cap = self.params.capacity;
        if self.first == self.next {
            [None, None]
        } else if self.first < self.next {
            [Some((self.first, self.next - 1)), None]
        } else if self.next == 0 {
            [Some((self.first, cap)), None]
        } else {
            [Some((self.first, cap)), Some((0, self.next - 1))]
        }
    }

    fn indices(&self) -> impl Iterator<Item = usize> + use<> {
        self.ranges().into_iter().flatten().flat_map(|(a, b)| a..=b)
    }

    /// Live particles.
    pub fn live(&self) -> usize {
        self.indices().filter(|&i| self.slots[i].life > 0).count()
    }

    /// Particles alive with their slot index, in ring order.
    pub fn particles(&self) -> impl Iterator<Item = (usize, Vec3, Vec3, i32)> + '_ {
        self.indices().filter_map(move |i| {
            let s = &self.slots[i];
            (s.life > 0).then_some((i, s.p, s.q, s.life))
        })
    }

    /// Whether a given slot is drawn (for the print of the ring).
    pub fn slot_visible(&self, i: usize) -> bool {
        self.slots.get(i).is_some_and(|s| s.life > 0)
    }

    /// One simulation tick.
    pub fn tick<R: Draws>(&mut self, node: NodeFrame, rng: &mut R) -> TickStats {
        if !self.entered {
            self.enter(node);
        }
        // A body-attached emitter rides its body's frame, not the node's.
        let node = self.body_frame.unwrap_or(node);
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
                for i in self.indices() {
                    let s = &mut self.slots[i];
                    if s.life > 0 {
                        s.life -= 1;
                    }
                }
            }
            // Pop the dead from the head. The original counts a dead slot
            // before it checks for the empty ring.
            while self.slots[self.first].life < 1 {
                died += 1;
                if self.first == self.next || died > self.params.capacity + 1 {
                    break;
                }
                self.first += 1;
                if self.params.capacity < self.first {
                    self.first = 0;
                }
            }
        }
        if p.flags & flags::BURST != 0 && died > 1 {
            self.accumulator -= (died.min(4) - 1) as f32 * p.interval;
        }
        if p.flags & flags::FIRE_AGEING == 0 {
            self.last_node = self.node.position;
        }
        for i in self.indices() {
            self.integrate(i);
        }
        if p.flags & flags::FINITE != 0 {
            if self.deadline < self.ticks {
                self.active = false;
                if self.first == self.next {
                    self.emitter_flags |= state::FINISHED;
                    return TickStats {
                        died,
                        spawned: 0,
                        live: 0,
                    };
                }
            }
        } else if p.flags & flags::ONE_SHOT != 0
            && self.first == self.next
            && self.emitter_flags & (state::FINISHED | state::FIRST_PASS) == 0
        {
            self.emitter_flags |= state::FINISHED;
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
        // NaN stops the loop, as the original's unordered compare does.
        while self.active && self.accumulator < 1.0 {
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
                let slot = self.spawn(index, rng);
                spawned += 1;
                // The pull is along the source's own second axis, untransformed.
                let src = self.sources[index];
                self.slots[slot].q += src.axes[1] * -src.kick;
                if p.start_life == 0 && self.next == self.params.capacity {
                    self.active = false;
                    break;
                }
            }
            self.accumulator += p.interval;
        }
        if p.flags & flags::FIXED_BOUNDS == 0 {
            self.update_bounds();
        }
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
        let radius = 140.0 * self.size;
        // The original eases the node's motion in by 7.5% a tick.
        self.smoothed += (delta - self.smoothed) * 0.075;
        let s = self.smoothed;
        self.motion_squared = s.z * s.z + (s.y * s.y + s.x * s.x);
        self.motion_average =
            (f64::from(self.motion_average) * 0.9 + f64::from(self.motion_squared) * 0.1) as f32;
        if self.emitter_flags & flags::SOUND_MOTION != 0 && self.sound.attached {
            let f = self.motion_squared * 0.014_285_714;
            let f = f * f * 0.75;
            self.sound.volume = if 1.0 < f {
                1.0
            } else if f64::from(f) < 0.03 {
                0.03
            } else {
                f
            };
        }
        if let Some(b) = body {
            let offset = if b.full_displacement {
                current - self.last_node
            } else {
                b.velocity * 0.5
            };
            for i in self.indices() {
                let s = &mut self.slots[i];
                if s.life > 0 {
                    s.p += offset;
                    s.q += offset;
                }
            }
        }
        self.last_node = current;
        // Each live particle loses one extra life tick half the time.
        if p.flags & flags::BURST != 0 {
            for i in self.indices() {
                if self.slots[i].life > 0 && rng.int(2) == 0 {
                    self.slots[i].life -= 1;
                }
            }
        }
        let ratio = p.start_life as f32 / radius;
        for i in self.indices() {
            let s = &mut self.slots[i];
            if s.life <= 0 {
                continue;
            }
            let mut d = s.p - current;
            d.y = (f64::from(d.y) * 0.7) as f32;
            // The squares add as the original adds them: z², then y², then x².
            let distance = (d.z * d.z + (d.y * d.y + d.x * d.x)).sqrt();
            let cap = ((radius - distance) * ratio).round_ties_even();
            // The conversion of a NaN or out-of-range value gives the
            // integer indefinite.
            let cap = if cap.is_finite() && cap.abs() < 2.147_483_6e9 {
                cap as i32
            } else {
                i32::MIN
            };
            if cap < s.life {
                s.life = cap;
            } else {
                s.life -= 1;
            }
        }
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

    /// Spawns one particle from a source into the ring's next slot and
    /// returns that slot.
    fn spawn<R: Draws>(&mut self, source: usize, rng: &mut R) -> usize {
        let p = self.params;
        let slot = self.next;
        let mut life = if p.flags & flags::LIFE_RANDOM != 0 {
            p.start_life - rng.int((p.start_life as u32) >> 2) as i32
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
            self.slots[slot].variant
        };
        let src = self.sources[source];
        let rotation = self.node.rotation;
        let mut at = self.node.position + rotation * src.offset;
        if 0.0 < p.spawn_radius {
            let angle = (f64::from(std::f32::consts::TAU) * rng.unit()) as f32;
            let (sin, cos) = angle.sin_cos();
            let u = rng.unit();
            let u2 = u * u;
            let r = (1.0 - u2 * u2) as f32 * p.spawn_radius;
            at += Vec3::new(cos, 0.0, -sin) * r;
        }
        let mut direction = if p.flags & flags::VELOCITY_MASK == flags::VELOCITY_RANDOM {
            let spread = f64::from(p.spread) * 0.001;
            let x = (spread * (f64::from(rng.int(2001)) - 1000.0)) as f32;
            let y = (f64::from(rng.int(1001)) * 0.001) as f32;
            let z = (spread * (f64::from(rng.int(2001)) - 1000.0)) as f32;
            // The source's axes go through the node's frame first (each one
            // a matrix row product), then the three weighted axes add in
            // x, y, z order.
            let a0 = rotation * src.axes[0];
            let a1 = rotation * src.axes[1];
            let a2 = rotation * src.axes[2];
            a0 * x + a1 * y + a2 * z
        } else {
            Vec3::Y
        };
        let prev;
        if p.flags & flags::CAMERA_FORWARD == 0 {
            prev = at + direction * -p.speed - src.velocity * 0.3;
            at += direction * p.speed;
        } else {
            direction *= p.speed;
            direction += src.velocity * 0.15;
            direction = direction + (src.smoothed - direction) * 0.5;
            self.sources[source].smoothed = direction;
            prev = at + direction * -1.2;
            at += direction * 0.8;
        }
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
            self.emitter_flags &= !state::FIRST_PASS;
            if p.flags & flags::ONE_SHOT != 0 {
                self.active = false;
            }
            self.next = 0;
        }
        slot
    }

    /// The bounding box of the live particles, padded by 5 cm.
    fn update_bounds(&mut self) {
        if self.first == self.next {
            self.bounds.extent = Vec3::ZERO;
            self.bounds.radius = 0.0;
            return;
        }
        let latest = if self.next == 0 {
            self.params.capacity
        } else {
            self.next - 1
        };
        let mut min = self.slots[latest].p;
        let mut max = min;
        for i in self.indices() {
            let s = &self.slots[i];
            if s.life <= 0 {
                continue;
            }
            for c in 0..3 {
                if min[c] <= s.p[c] {
                    if max[c] < s.p[c] {
                        max[c] = s.p[c];
                    }
                } else {
                    min[c] = s.p[c];
                }
            }
        }
        self.bounds.centre = min + (max - min) * 0.5;
        let e = (max - self.bounds.centre) + Vec3::splat(5.0);
        self.bounds.extent = e;
        let squared = e.z * e.z + (e.y * e.y + e.x * e.x);
        // The original keeps the squared value unless it is positive (so a
        // NaN stays NaN).
        self.bounds.radius = if 0.0 < squared {
            squared.sqrt()
        } else {
            squared
        };
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

    /// The looping sound the template names, if any.
    pub fn sound_name(&self) -> Option<&'static str> {
        self.sound_name
    }

    /// One frame of the light and sound walks, at the original's per-frame
    /// point: the calls run once a rendered frame, not once a tick.
    pub fn frame<R: Draws>(&mut self, rng: &mut R) -> LightFrame {
        let frame = self.light.frame(self.params.flags);
        self.sound.frame(self.light.density, self.params.flags, rng);
        frame
    }

    /// The material's level-of-detail scalar for a camera at `eye` (cm):
    /// the emitter's detail value times 1000 over the *squared* distance to
    /// its node, the squared distance held at one or more. The original's
    /// distance helper returns the squared length (`(x² + y²) + z²`, no
    /// square root); a NaN distance fails the compare and is kept.
    pub fn lod_scale(&self, eye: Vec3) -> f32 {
        let d = eye - self.node.position;
        let mut squared = d.x * d.x + d.y * d.y + d.z * d.z;
        if squared < 1.0 {
            squared = 1.0;
        }
        self.detail * 1000.0 / squared
    }

    /// The four corners of the fixed screen quad a material that draws one
    /// asks for: each corner is 0.3 of the camera's back axis (the camera
    /// row the original negates for its forward axis, so pointing at the
    /// viewer) displaced half a unit along its right or up axis, normalised,
    /// in the original's corner order (−up, +right, −right, +up).
    pub fn camera_quad(view: View) -> [Vec3; 4] {
        let forward = -view.forward * 0.3;
        [
            (forward - view.up * 0.5).normalize_or(Vec3::NEG_Y),
            (forward + view.right * 0.5).normalize_or(Vec3::X),
            (forward - view.right * 0.5).normalize_or(Vec3::NEG_X),
            (forward + view.up * 0.5).normalize_or(Vec3::Y),
        ]
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

/// The particle unit's 128-entry UV lookup, built once at start-up: four
/// rows (the atlas cells, origins `(0, 0)`, `(½, 0)`, `(0, ½)`, `(½, ½)`)
/// of 32 entries (the eight [`SYMMETRIES`], four corners each, halved). The
/// entry for row `r`, symmetry `s` and corner `c` is at `r · 32 + s · 4 + c`
/// and equals `SYMMETRIES[s][c] · ½ + origin(r)`, which is what
/// [`Emitter::quads`] computes per particle for the 32-variant atlas flag.
pub fn particle_uv_table() -> [[f32; 2]; 128] {
    const ROW_ORIGINS: [[f32; 2]; 4] = [[0.0, 0.0], [0.5, 0.0], [0.0, 0.5], [0.5, 0.5]];
    let mut out = [[0.0f32; 2]; 128];
    for (row, origin) in ROW_ORIGINS.iter().enumerate() {
        for (entry, corner) in SYMMETRIES.iter().flatten().enumerate() {
            out[row * 32 + entry] = [corner[0] * 0.5 + origin[0], corner[1] * 0.5 + origin[1]];
        }
    }
    out
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
        let mut out = Vec::with_capacity(self.live());
        self.quads_into(alpha, view, &mut out);
        out
    }

    /// One camera axis as the quad builders use it: taken through the node's
    /// frame when the template asks for the node's axes (flag 0x40), else
    /// the camera's own axis; flattened onto the node's up plane when the
    /// view-plane bit is set; returned in world space with the axis' up
    /// component (the stretched quad's vertical scale reads it).
    fn quad_axis(&self, flags: u32, world: Vec3) -> (Vec3, f32) {
        let rotation = self.node.rotation;
        let mut axis = if flags & flags::NODE_AXES != 0 {
            rotation * world
        } else {
            world
        };
        if flags & flags::VIEW_PLANE != 0 {
            // The original projects onto the node's up axis and normalises.
            let up = rotation * Vec3::Y;
            axis += up * (axis.dot(up) * -0.9);
            axis = axis.normalize_or(Vec3::Y);
        }
        (axis, axis.y)
    }

    /// [`Emitter::quads`] into a buffer the caller keeps.
    pub fn quads_into(&self, alpha: f32, view: View, out: &mut Vec<Quad>) {
        out.clear();
        let p = self.params;
        let mode = p.flags & flags::QUAD_MASK;
        // The diamonds face the line from the eye to the particles' centre;
        // the beds' quads face the camera's up axis around that line.
        let to_centre = self.bounds.centre - view.eye;
        let facing = if mode == flags::QUAD_DIAMOND {
            if p.flags & flags::CAMERA_FORWARD != 0 {
                view.forward
            } else {
                to_centre.normalize_or(view.forward)
            }
        } else {
            to_centre
        };
        // The screen and stretched quads take the camera's axes through the
        // node's frame unless the template asks for the node's own axes, and
        // a template with the view-plane bit keeps a tenth of each axis'
        // offset along the node's up direction.
        let axis = |world: Vec3| self.quad_axis(p.flags, world);
        let x_base = to_centre.cross(view.up).normalize_or(view.right);
        let y_base = x_base.cross(to_centre).normalize_or(view.up);
        for (index, position, previous, life) in self.particles() {
            let delta = position - previous;
            let at = previous + delta * alpha;
            let age = 1.0 - (life as f32 - alpha) * p.inverse_life;
            let corners = if mode == flags::QUAD_DIAMOND {
                let size = if p.flags & flags::RANDOM_SIZE != 0 {
                    self.slots[index].size
                } else {
                    p.half_size
                };
                diamond_quad(at, delta, size, age, p, facing)
            } else if mode == flags::QUAD_STRETCHED {
                let (x, _) = axis(x_base);
                let (y, y_local) = axis(y_base);
                stretched_quad(at, age, p, x, y, y_local)
            } else {
                let (x, _) = axis(view.right);
                let (y, _) = axis(view.up);
                screen_quad(at, p.half_size, x, y)
            };
            let (uv, colour) = self.look(index, alpha, life);
            out.push(Quad {
                corners,
                uv,
                colour,
            });
        }
    }

    /// A particle's atlas UVs and colour at `alpha`, as the original's
    /// texture-variant and colour builders produce them.
    fn look(&self, index: usize, alpha: f32, life: i32) -> ([Vec2; 4], Key) {
        let p = self.params;
        let age = 1.0 - (life as f32 - alpha) * p.inverse_life;
        let variant = self.slots[index].variant;
        let uv: [Vec2; 4] = if p.flags & flags::UV_8 != 0 {
            std::array::from_fn(|i| Vec2::from(SYMMETRIES[usize::from(variant & 7)][i]))
        } else if p.flags & flags::UV_32 != 0 {
            let v = variant & 31;
            let cell = v >> 3;
            let sym = SYMMETRIES[usize::from(v & 7)];
            let origin = [f32::from(cell & 1) * 0.5, f32::from((cell >> 1) & 1) * 0.5];
            std::array::from_fn(|i| {
                Vec2::new(origin[0] + sym[i][0] * 0.5, origin[1] + sym[i][1] * 0.5)
            })
        } else {
            std::array::from_fn(|i| Vec2::from(SYMMETRIES[0][i]))
        };
        let colour = if p.flags & flags::KEYS_PARITY == flags::KEYS_PARITY {
            p.keys.sample(p.keys.count as f32 * age, index & 1 == 1)
        } else if p.flags & flags::KEYS != 0 {
            p.keys.sample(p.keys.count as f32 * age, false)
        } else {
            let f = 1.0 - (life as f32 - alpha) / p.start_life as f32;
            let a = ((1.0 - f * f) * 255.0).round_ties_even() as i32 as u8;
            let o = (self.flat_opacity * 255.0).round_ties_even() as i32 as u8;
            [a, a, a, o]
        };
        (uv, colour)
    }

    /// The fixed quads a material that draws them asks for: each live
    /// particle copies the same four corner offsets
    /// ([`Emitter::camera_quad`], the original's fixed template vertices)
    /// with its own atlas variant and colour. No Exarust material sets the
    /// flag, so nothing calls this yet.
    pub fn quads_fixed(&self, alpha: f32, view: View) -> Vec<Quad> {
        let corners = Emitter::camera_quad(view);
        self.particles()
            .map(|(index, _, _, life)| {
                let (uv, colour) = self.look(index, alpha, life);
                Quad {
                    corners,
                    uv,
                    colour,
                }
            })
            .collect()
    }
}

/// The original's fast arc cosine of a non-negative argument:
/// `(π/2 − 0.2075·x + 0.0535·x²)·√(1 − x)`.
fn fast_acos(x: f32) -> f32 {
    const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
    const LINEAR: f32 = f32::from_bits(0x3e54_7ae1);
    const SQUARE: f32 = f32::from_bits(0x3d5b_22d1);
    if x < 0.0 {
        let poly = x * x * SQUARE + (x * LINEAR + HALF_PI);
        (std::f64::consts::PI - f64::from(poly * (1.0 + x).sqrt())) as f32
    } else {
        let poly = x * x * SQUARE + (HALF_PI - x * LINEAR);
        poly * (1.0 - x).sqrt()
    }
}

/// A vector's length, its squares added as the original adds them.
fn length(v: Vec3) -> f32 {
    ((v.x * v.x + v.z * v.z) + v.y * v.y).sqrt()
}

/// The velocity-aligned diamond of a flame particle: tail, two sides, head.
fn diamond_quad(at: Vec3, delta: Vec3, size: f32, age: f32, p: Params, facing: Vec3) -> [Vec3; 4] {
    let mut w = delta;
    w.y -= p.buoyancy;
    let speed = length(w);
    let d = w * (-1.0 / speed);
    let fall = 2.0 - 2.0 / (speed + 1.0);
    let across = d.cross(facing);
    let l = length(across);
    // Along the view (or at rest with no buoyancy) the original divides by
    // zero; keep a finite quad of the base size there instead.
    if l.is_nan() || l <= 1.0e-6 || !speed.is_finite() || speed == 0.0 {
        let side = Vec3::X * size;
        let up = Vec3::Y * size;
        return [at - up, at + side, at - side, at + up];
    }
    let k = size / l + age * age * p.growth;
    let along = across.cross(facing);
    let x = across * k;
    let dot = (d.x * facing.x + d.y * facing.y) + d.z * facing.z;
    let stretch = fall * p.stretch * fast_acos(dot.abs());
    let reach = (stretch + 1.0) * k;
    [at + along * -reach, at + x, at - x, at + along * reach]
}

/// The screen-aligned, age-stretched quad of an ember bed. Its long axis
/// grows with how upright the camera's up axis is around the view line. The
/// long axis keeps its offset along the node's up direction (`y_local`), the
/// component the original reads from the node's local frame.
fn stretched_quad(
    at: Vec3,
    age: f32,
    p: Params,
    x_axis: Vec3,
    y_axis: Vec3,
    y_local: f32,
) -> [Vec3; 4] {
    let width = age * age * p.growth + p.half_size;
    let x = x_axis * width;
    let y = y_axis * ((y_local + 1.0) * width);
    [at - y, at + x, at - x, at + y]
}

/// A flat billboard of half-size `size` along the two axes.
fn screen_quad(at: Vec3, size: f32, right: Vec3, up: Vec3) -> [Vec3; 4] {
    let x = right * size;
    let y = up * size;
    [at - y, at + x, at - x, at + y]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scripted stream: the numbers the test wants, in order.
    struct Scripted {
        ints: Vec<u32>,
        units: Vec<f64>,
    }

    impl Scripted {
        fn new(ints: &[u32], units: &[f64]) -> Self {
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

        fn unit(&mut self) -> f64 {
            if self.units.is_empty() {
                0.0
            } else {
                self.units.remove(0)
            }
        }
    }

    /// The small fire's template, the one a torch uses (its raw keys).
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
        let mut e = Emitter::new(SMALL.scaled(0.2), 0.2, vec![Source::at_node()]);
        e.enter(NodeFrame::default());
        e
    }

    #[test]
    fn the_mersenne_twister_matches_its_reference_outputs() {
        let mut mt = Mt19937::new(5489);
        assert_eq!(mt.next_u32(), 3_499_211_612);
        assert_eq!(mt.next_u32(), 581_869_302);
        assert_eq!(mt.next_u32(), 3_890_346_734);
    }

    #[test]
    fn the_integer_stream_is_the_engines_lcg() {
        let mut rng = Rng::new(0);
        // seed 0 → 1: (1 · 3) >> 32 = 0; then 1 · 0x08088405 + 1.
        assert_eq!(rng.int(3), 0);
        assert_eq!(rng.ints, 1);
        rng.int(3);
        assert_eq!(rng.ints, 0x0808_8406);
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
        // 450 · (0.9 · 0.04 + 0.02) = 25.2 → 25.
        assert_eq!(d.density, 25);
    }

    #[test]
    fn the_derived_values_match_the_torch() {
        let e = torch();
        // trunc(83 · 1.25) = trunc(103.75).
        assert_eq!(e.params.start_life, 103);
        assert!((e.params.interval - 83.0 / 50.0 * (2.0 / 3.0)).abs() < 1e-6);
        // trunc(50 · 1.25 · 1.5 + 1.501) + 2 = trunc(95.251) + 2.
        assert_eq!(e.params.capacity, 97);
        assert_eq!(e.slots.len(), 98);
        assert_eq!(e.params.density_count, 25);
        assert_eq!(e.params.retention, 1.0);
        assert_eq!(e.params.speed, 0.0);
        assert!((e.params.growth - 12.0).abs() < 1e-6);
        assert!((e.params.collision_growth - 18.0).abs() < 1e-6);
        // The sound's motion bit follows the template into the state word.
        assert_eq!(e.emitter_flags, state::INITIAL | flags::SOUND_MOTION);
    }

    #[test]
    fn a_retention_is_taken_per_tick_in_double_precision() {
        let mut d = SMALL;
        d.retention = 0.5;
        let e = Emitter::new(d, 1.0, vec![Source::at_node()]);
        assert_eq!(e.params.retention, (0.5f64.ln() / 83.0).exp() as f32);
    }

    #[test]
    fn one_spawn_lands_on_the_ring_at_rest() {
        let mut e = torch();
        // Three light draws, then the atlas variant, the ring's angle and
        // radius, and the three velocity integers (the torch's speed is
        // zero, so they move nothing).
        let mut rng = Scripted::new(&[1, 9, 1000, 500], &[0.0, 0.0, 0.0, 0.5, 0.0]);
        e.tick(NodeFrame::default(), &mut rng);
        let (_, p, q, life) = e.particles().next().expect("one particle");
        assert_eq!(life, 103);
        // At rest: the template speed is zero and the node did not move.
        assert_eq!(p, q);
        // θ = π, u = 0: r = 4.9.
        assert!((p.x + 4.9).abs() < 1e-4, "{p:?}");
        assert!(p.z.abs() < 1e-4);
        assert!(p.y.abs() < 1e-6);
    }

    #[test]
    fn a_spawn_inherits_three_tenths_of_the_source_motion() {
        let mut e = torch();
        e.params.spawn_radius = 0.0;
        e.tick(NodeFrame::default(), &mut Scripted::new(&[], &[]));
        let moved = NodeFrame {
            position: Vec3::new(10.0, 0.0, 0.0),
            rotation: Mat3::IDENTITY,
        };
        let before = e.next;
        e.accumulator = 0.0;
        e.tick(moved, &mut Scripted::new(&[], &[]));
        let s = e.slots[before];
        // The source moved 10 cm this tick: q lags by 3 cm.
        assert!((s.p.x - 10.0).abs() < 1e-5);
        assert!((s.q.x - 7.0).abs() < 1e-5);
    }

    #[test]
    fn the_first_tick_does_not_fling_a_particle_from_the_origin() {
        let mut e = Emitter::new(SMALL.scaled(0.2), 0.2, vec![Source::at_node()]);
        e.params.spawn_radius = 0.0;
        let far = NodeFrame {
            position: Vec3::new(5000.0, 300.0, -2000.0),
            rotation: Mat3::IDENTITY,
        };
        e.tick(far, &mut Scripted::new(&[], &[]));
        let (_, p, q, _) = e.particles().next().expect("a particle");
        assert_eq!(p, q);
    }

    #[test]
    fn the_ring_law_puts_particles_near_the_rim() {
        let r = |u: f64| (1.0 - u * u * u * u) as f32 * 4.9;
        assert!((r(0.0) - 4.9).abs() < 1e-6);
        assert!((r(0.5) - 4.9 * 0.9375).abs() < 1e-6);
        assert_eq!(r(1.0), 0.0);
    }

    #[test]
    fn the_interval_gives_the_template_rate() {
        let e = torch();
        let actual = (2.0 / 3.0) / e.params.interval;
        assert!((actual - 50.0 / 83.0).abs() < 1e-6);
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
        assert!((s.p.x - (3.0 - 3.0 * 0.0007)).abs() < 1e-6);
        assert!((s.p.z - (-2.0 + 2.0 * 0.0007)).abs() < 1e-6);
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
        // The extra-tick draw is 1: no extra tick.
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
    fn an_emptying_ring_counts_one_more_pop() {
        let mut e = torch();
        e.params.flags &= !flags::FIRE_AGEING;
        e.first = 0;
        e.next = 2;
        e.slots[0].life = 1;
        e.slots[1].life = 1;
        e.slots[2].life = 0;
        let stats = e.tick(NodeFrame::default(), &mut Scripted::new(&[0], &[]));
        // Two pops, and the dead slot the empty ring stops on.
        assert_eq!(stats.died, 3);
    }

    #[test]
    fn the_key_list_fades_in_and_out() {
        let k = SMALL.scaled(0.2).keys;
        assert_eq!(k.sample(0.0, false), [0, 0, 0, 0]);
        assert_eq!(k.sample(9.0, true)[3], 0);
        let c = k.sample(4.0, true);
        assert_eq!(c[3], 64);
        assert_eq!(c[0], 255);
    }

    #[test]
    fn the_colour_lerp_rounds_half_to_even() {
        let keys = Keys {
            count: 1,
            entries: &[[10, 20, 30, 40], [110, 220, 130, 240]],
        };
        assert_eq!(keys.sample(0.5, false), [60, 120, 80, 140]);
        let keys = Keys {
            count: 1,
            entries: &[[0, 0, 0, 0], [3, 3, 3, 3]],
        };
        // 1.89 rounds to 2; 0.5 and 2.5 round to even.
        assert_eq!(keys.sample(0.63, false), [2, 2, 2, 2]);
        let keys = Keys {
            count: 1,
            entries: &[[0, 4, 0, 0], [1, 5, 0, 0]],
        };
        assert_eq!(keys.sample(0.5, false)[..2], [0, 4]);
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
        e.light.density = 0.0;
        e.light.tick(25, 25, &mut rng);
        assert!((e.light.density - 0.2).abs() < 1e-6);
    }

    #[test]
    fn the_light_peak_needs_a_strictly_brighter_frame() {
        let mut light = LightState::new(SMALL.light);
        // The emitter starts with a full update due.
        let frame = light.frame(0);
        assert!(frame.full_update);
        light.density = 1.0;
        let frame = light.frame(0);
        assert!(frame.full_update);
        assert!((light.peak - frame.intensity * 1.1).abs() < 1e-6);
        // A dimmer frame walks the light without a full update.
        let frame = light.frame(0);
        assert!(!frame.full_update);
        // A one-shot emitter asks for one every frame.
        assert!(light.frame(crate::particles::flags::ONE_SHOT).full_update);
    }

    #[test]
    fn the_emissive_scale_reaches_the_parent_and_its_lights() {
        let parent = EmissiveNode {
            node: 1.0,
            material: 1.0,
            light: false,
        };
        let mut parent = parent;
        let mut children = [
            EmissiveNode {
                node: 1.0,
                material: 1.0,
                light: true,
            },
            EmissiveNode {
                node: 1.0,
                material: 1.0,
                light: false,
            },
        ];
        assert_eq!(apply_emissive(0.4, &mut parent, &mut children), 0.4);
        assert_eq!((parent.node, parent.material), (0.4, 0.4));
        // The light child takes it; the other child keeps its own.
        assert_eq!((children[0].node, children[0].material), (0.4, 0.4));
        assert_eq!((children[1].node, children[1].material), (1.0, 1.0));
    }

    #[test]
    fn attaching_an_emitter_flags_its_node_and_material() {
        let mut node = AttachedNode::default();
        attach_node(&mut node);
        assert_eq!(node.flags & NODE_HAS_CHILD, NODE_HAS_CHILD);
        assert_eq!(node.render & NODE_DRAWS_EMITTER, NODE_DRAWS_EMITTER);
        assert_eq!(node.material_flags & MATERIAL_EMITTER, MATERIAL_EMITTER);
        assert!(node.framed);
    }

    #[test]
    fn the_quad_axes_take_the_nodes_frame_unless_it_asks_for_the_cameras() {
        let mut e = torch();
        e.first = 0;
        e.next = 1;
        e.slots[0] = Slot {
            p: Vec3::ZERO,
            q: Vec3::ZERO,
            life: 50,
            ..Slot::default()
        };
        // The node is turned a quarter turn about Z: its x is the world y.
        e.node.rotation = Mat3::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let view = View {
            eye: Vec3::new(0.0, 0.0, 100.0),
            right: Vec3::X,
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
        };
        // The torch's diamond ignores the camera axes; use the screen quad.
        let mut flat = SMALL.scaled(0.2);
        flat.flags = (flat.flags & !flags::QUAD_MASK) | flags::QUAD_STRETCHED;
        let mut e2 = Emitter::new(flat, 0.2, vec![Source::at_node()]);
        e2.first = 0;
        e2.next = 1;
        e2.slots[0] = Slot {
            p: Vec3::ZERO,
            q: Vec3::ZERO,
            life: 50,
            ..Slot::default()
        };
        e2.node.rotation = e.node.rotation;
        e2.bounds.centre = Vec3::ZERO;
        let c = e2.quads(0.0, view)[0].corners;
        // Without the node-axes bit the quad keeps the camera's up.
        assert!((c[3] - c[0]).y.abs() > (c[3] - c[0]).x.abs());
        // With it the quad turns with the node: its up runs along the world
        // -x.
        e2.params.flags |= flags::NODE_AXES;
        e2.params.flags &= !flags::VIEW_PLANE;
        let c = e2.quads(0.0, view)[0].corners;
        assert!((c[3] - c[0]).x.abs() > (c[3] - c[0]).y.abs());
    }

    #[test]
    fn the_view_plane_bit_flattens_a_quad_onto_the_nodes_up_plane() {
        // A camera axis a fifth up: 0.6 x, 0.8 y, 0. Keeping a tenth of the
        // up offset along the node's y gives (0.6, 0.08), normalised.
        let mut flat = SMALL.scaled(0.2);
        flat.flags = (flat.flags & !flags::QUAD_MASK) | flags::QUAD_STRETCHED;
        flat.flags |= flags::NODE_AXES | flags::VIEW_PLANE;
        let mut e = Emitter::new(flat, 0.2, vec![Source::at_node()]);
        e.first = 0;
        e.next = 1;
        e.slots[0] = Slot {
            p: Vec3::ZERO,
            q: Vec3::ZERO,
            life: 50,
            ..Slot::default()
        };
        e.bounds.centre = Vec3::new(0.0, 0.0, 100.0);
        let view = View {
            eye: Vec3::ZERO,
            right: Vec3::new(0.6, 0.8, 0.0),
            up: Vec3::new(0.6, 0.8, 0.0),
            forward: Vec3::NEG_Z,
        };
        let (axis, local) = e.quad_axis(e.params.flags, Vec3::new(0.6, 0.8, 0.0));
        let want = Vec3::new(0.6, 0.08, 0.0).normalize();
        assert!((axis - want).length() < 1e-6, "{axis:?}");
        assert!((local - want.y).abs() < 1e-6);
        // A vertical axis keeps only its tenth of the up component.
        let (axis, _) = e.quad_axis(e.params.flags, Vec3::Y);
        assert!((axis - Vec3::Y).length() < 1e-6);
        // The quad built from it is finite.
        assert!(e.quads(0.0, view)[0].corners.iter().all(|c| c.is_finite()));
    }

    #[test]
    fn a_body_frame_carries_the_emitter() {
        let mut e = torch();
        e.params.spawn_radius = 0.0;
        let frame = NodeFrame {
            position: Vec3::new(100.0, 0.0, 0.0),
            rotation: Mat3::from_rotation_z(std::f32::consts::FRAC_PI_2),
        };
        e.attach_body(frame);
        assert_eq!(e.params.flags & flags::FIRE_AGEING, flags::FIRE_AGEING);
        e.tick(NodeFrame::default(), &mut Scripted::new(&[1], &[0.5]));
        let (_, p, q, _) = e.particles().next().expect("a particle");
        // The spawn sits on the body, not on the node at the origin.
        assert!((p.x - 100.0).abs() < 1e-4, "{p:?}");
        assert!((q.x - 100.0).abs() < 1e-4);
    }

    #[test]
    fn the_fixed_quad_is_the_same_for_every_particle() {
        let mut e = torch();
        e.first = 0;
        e.next = 2;
        e.slots[0] = Slot {
            p: Vec3::new(1.0, 2.0, 3.0),
            q: Vec3::ZERO,
            life: 50,
            variant: 3,
            ..Slot::default()
        };
        e.slots[1] = Slot {
            p: Vec3::new(-4.0, 5.0, 6.0),
            q: Vec3::ZERO,
            life: 40,
            ..Slot::default()
        };
        let view = View {
            eye: Vec3::new(0.0, 0.0, 100.0),
            right: Vec3::X,
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
        };
        let quads = e.quads_fixed(0.0, view);
        assert_eq!(quads.len(), 2);
        assert_eq!(quads[0].corners, quads[1].corners);
        assert_eq!(quads[0].corners, Emitter::camera_quad(view));
    }

    #[test]
    fn the_light_scales_work_in_double_precision() {
        let mut light = LightState::new(SMALL.light);
        light.density = 0.7;
        light.walk = 0.3;
        // The exact double-precision result, narrowed once.
        let want = (0.7f64 * ((0.3f64 * 0.05 + 1.0) * f64::from(SMALL.light.intensity))) as f32;
        let frame = light.frame(0);
        assert_eq!(frame.intensity, want);
        // 0.7 · (0.3 · 0.2 + 0.9) · 0.8 = 0.5376.
        assert!((frame.emissive - 0.5376).abs() < 1e-6, "{}", frame.emissive);
        // The softness follows the jitter.
        light.jitter = 1.05;
        assert!((light.frame(0).softness - 2.1).abs() < 1e-6);
        // The light reaches its template intensity at full density.
        light.density = 1.0;
        assert!(light.frame(0).at_base);
    }

    #[test]
    fn a_template_names_its_looping_sound() {
        assert_eq!(torch().sound_name(), Some("FireS02"));
        let mut d = SMALL.scaled(0.2);
        d.sound = None;
        assert_eq!(
            Emitter::new(d, 0.2, vec![Source::at_node()]).sound_name(),
            None
        );
    }

    #[test]
    fn the_sound_walk_stays_within_its_template() {
        let mut sound = SoundState::new(0.5, true);
        sound.volume = 0.1;
        let mut rng = Scripted::new(&[], &[0.0, 0.0]);
        sound.frame(1.0, 0, &mut rng);
        // The volume fell to the low bound; the pitch walks a little.
        assert_eq!(sound.volume, 0.4);
        assert_eq!(sound.pitch, 0.9975);
        sound.volume = 1.0;
        let mut rng = Scripted::new(&[], &[1.0, 1.0]);
        sound.frame(1.0, 0, &mut rng);
        assert_eq!(sound.volume, 0.6);
        // The pitch walks by a hundredth of the template's step.
        sound.pitch = 0.851;
        let mut rng = Scripted::new(&[], &[0.0, 0.0]);
        sound.frame(1.0, 0, &mut rng);
        assert_eq!(sound.pitch, 0.85);
        sound.pitch = 1.149;
        let mut rng = Scripted::new(&[], &[1.0, 1.0]);
        sound.frame(1.0, 0, &mut rng);
        assert_eq!(sound.pitch, 1.15);
        // A wide-pitch template spans 0.6 to 1.4.
        let mut wide = SoundState::new(1.0, true);
        wide.pitch = 0.601;
        let mut rng = Scripted::new(&[], &[0.0, 0.0]);
        wide.frame(1.0, crate::particles::flags::SOUND_WIDE_PITCH, &mut rng);
        assert_eq!(wide.pitch, 0.6);
        // A one-shot emitter scales its volume by the light's density.
        let mut one = SoundState::new(1.0, true);
        one.volume = 1.0;
        let mut rng = Scripted::new(&[], &[0.5, 0.5]);
        one.frame(0.5, crate::particles::flags::ONE_SHOT, &mut rng);
        assert!((one.volume - 0.5).abs() < 1e-6, "{}", one.volume);
        // A NaN volume fails both unordered compares and is kept.
        let mut lost = SoundState::new(0.5, true);
        lost.volume = f32::NAN;
        let mut rng = Scripted::new(&[], &[0.5, 0.5]);
        lost.frame(1.0, 0, &mut rng);
        assert!(lost.volume.is_nan());
        // The bound itself is the double-precision product, narrowed once.
        let mut odd = SoundState::new(0.7, true);
        odd.volume = 10.0;
        let mut rng = Scripted::new(&[], &[0.5, 0.5]);
        odd.frame(1.0, 0, &mut rng);
        assert_eq!(odd.volume, (f64::from(0.7f32) * 1.2) as f32);
        // A template with no sound does not walk at all.
        let mut silent = SoundState::new(0.5, false);
        let mut rng = Scripted::new(&[], &[0.0, 0.0]);
        silent.frame(1.0, 0, &mut rng);
        assert_eq!((silent.volume, silent.pitch), (1.0, 1.0));
    }

    #[test]
    fn a_one_shot_deadline_spans_the_ring_and_the_life() {
        let mut d = SMALL.scaled(0.2);
        d.flags |= crate::particles::flags::ONE_SHOT;
        let e = Emitter::new(d, 0.2, vec![Source::at_node()]);
        // 103 + 97 · 1.10667 + 0.5 = 210.85 → 211 (the conversion rounds
        // half to even).
        let want = (103.0f32 + 97.0 * e.params.interval + 0.5).round_ties_even() as i32;
        assert_eq!(e.params.deadline, want);
        // A template that is not one-shot has no ring deadline.
        assert_eq!(torch().params.deadline, i32::MAX);
    }

    #[test]
    fn the_node_motion_is_eased_in_by_seven_and_a_half_percent() {
        let mut e = torch();
        e.slots[0] = Slot::default();
        e.first = 0;
        e.next = 0;
        e.last_node = Vec3::new(-10.0, 0.0, 0.0);
        e.age(&mut Scripted::new(&[1], &[]));
        // The node moved 10 cm back: the eased delta is a 7.5% step.
        assert!((e.smoothed.x + 0.75).abs() < 1e-6, "{:?}", e.smoothed);
    }

    #[test]
    fn the_bounds_radius_adds_the_squares_from_z() {
        let mut e = torch();
        e.first = 0;
        e.next = 3;
        e.slots[0] = Slot {
            p: Vec3::new(1.0, 2.0, 3.0),
            life: 10,
            ..Slot::default()
        };
        e.slots[1] = Slot {
            p: Vec3::new(-4.0, 5.0, -6.0),
            life: 10,
            ..Slot::default()
        };
        e.slots[2] = Slot::default();
        e.update_bounds();
        let extent = e.bounds.extent;
        // Extent = max − midpoint + 5 = half the span + 5.
        // The last slot (dead) seeds the box, as the original seeds it.
        assert_eq!(extent, Vec3::new(7.5, 7.5, 9.5));
        let squared = extent.z * extent.z + (extent.y * extent.y + extent.x * extent.x);
        assert_eq!(e.bounds.radius, squared.sqrt());
    }

    #[test]
    fn the_flat_quad_fades_on_the_age_square() {
        // (1 − f²) · 255 with f = 1 − life/start_life = 1 − 52/103.
        let mut e = torch();
        e.first = 0;
        e.next = 1;
        e.slots[0] = Slot {
            p: Vec3::ZERO,
            q: Vec3::new(0.0, -1.0, 0.0),
            life: 52,
            ..Slot::default()
        };
        e.params.flags &= !crate::particles::flags::KEYS;
        e.params.flags &= !crate::particles::flags::KEYS_PARITY;
        let view = View {
            eye: Vec3::new(0.0, 0.0, 100.0),
            right: Vec3::X,
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
        };
        let colour = e.quads(0.0, view)[0].colour;
        // 0.75483 · 255 = 192.48, rounded half to even.
        assert_eq!(colour[..3], [192, 192, 192]);
    }

    #[test]
    fn the_level_of_detail_is_the_detail_over_the_squared_distance() {
        let mut e = torch();
        e.size = 0.2;
        e.detail = 0.4;
        e.node.position = Vec3::new(100.0, 0.0, 0.0);
        // The detail, not the size, over the squared distance:
        // 0.4 · 1000 / 100² = 0.04.
        assert!((e.lod_scale(Vec3::ZERO) - 0.04).abs() < 1e-7);
        // A camera 10 cm away: 400 / 100 = 4.
        assert!((e.lod_scale(Vec3::new(110.0, 0.0, 0.0)) - 4.0).abs() < 1e-6);
        // A squared distance under one is held at one: 400.
        assert_eq!(e.lod_scale(Vec3::new(100.5, 0.0, 0.0)), 400.0);
        // A NaN eye fails the compare and stays NaN.
        assert!(e.lod_scale(Vec3::splat(f32::NAN)).is_nan());
    }

    #[test]
    fn the_fixed_camera_quad_has_the_originals_corner_order() {
        let view = View {
            eye: Vec3::ZERO,
            right: Vec3::X,
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
        };
        let q = Emitter::camera_quad(view);
        // Each corner is 0.3 along the camera's back axis (+Z here, towards
        // the viewer), half along right or up, then normalised: 0.3/0.5831
        // = 0.5145 and 0.5/0.5831 = 0.8575.
        let want = [0.514_495_7, 0.857_492_9];
        assert!((q[0].y + want[1]).abs() < 1e-5 && (q[0].z - want[0]).abs() < 1e-5);
        assert!((q[1].x - want[1]).abs() < 1e-5 && (q[1].z - want[0]).abs() < 1e-5);
        assert!((q[2].x + want[1]).abs() < 1e-5 && (q[2].z - want[0]).abs() < 1e-5);
        assert!((q[3].y - want[1]).abs() < 1e-5 && (q[3].z - want[0]).abs() < 1e-5);
        for corner in q {
            assert!((corner.length() - 1.0).abs() < 1e-5);
        }
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
        e.bounds.centre = Vec3::new(0.0, 3.0, 0.0);
        let view = View {
            eye: Vec3::new(100.0, 3.0, 0.0),
            right: Vec3::NEG_Z,
            up: Vec3::Y,
            forward: Vec3::NEG_X,
        };
        let quads = e.quads(0.0, view);
        assert_eq!(quads.len(), 1);
        let c = quads[0].corners;
        // At α = 0 the centre is the previous position (0, 1, 0). Side-on:
        // ℓ = 1, the stretch is (2 − 2/3.25) · 0.6 · acos(0) ≈ 1.31.
        let w = 2.0 + 0.25f32;
        let t = 1.0 - 50.0 / 103.0;
        let width = 6.0 + t * t * 12.0;
        let stretch = (2.0 - 2.0 / (w + 1.0)) * 0.6 * fast_acos(0.0);
        assert!(((c[1] - c[2]).length() - 2.0 * width).abs() < 1e-3);
        assert!(((c[3] - c[0]).length() - 2.0 * (1.0 + stretch) * width).abs() < 1e-3);
        // The head is up, along the motion and the buoyancy lean.
        assert!(c[3].y > c[0].y);
    }

    #[test]
    fn the_fast_arc_cosine_is_the_originals() {
        assert!((fast_acos(0.0) - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert_eq!(fast_acos(1.0), 0.0);
        // Close to the true value, but not it: at 0.5,
        // √0.5 · (π/2 − 0.10375 + 0.013375) = 1.04682 against 1.04720.
        let x = 0.5f32;
        assert!((fast_acos(x) - 1.046_82).abs() < 1e-4, "{}", fast_acos(x));
        assert_ne!(fast_acos(x), x.acos());
    }

    #[test]
    fn the_texture_variants_cover_four_cells_and_eight_symmetries() {
        let mut e = torch();
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
        assert_eq!(uv[3], Vec2::new(0.5, 0.5));
    }

    #[test]
    fn a_finite_emitter_stops_after_its_deadline() {
        let mut d = SMALL.scaled(0.2);
        d.flags |= flags::FINITE;
        let mut e = Emitter::new(d, 0.2, vec![Source::at_node()]);
        e.deadline = 5;
        let mut rng = Scripted::new(&[], &[]);
        for _ in 0..6 {
            e.tick(NodeFrame::default(), &mut rng);
        }
        assert!(!e.active);
    }

    #[test]
    fn a_one_shot_emitter_stops_at_the_wrap_and_finishes_empty() {
        let mut d = SMALL.scaled(0.2);
        d.flags |= flags::ONE_SHOT;
        let mut e = Emitter::new(d, 0.2, vec![Source::at_node()]);
        let mut rng = Rng::new(3);
        let mut ticks = 0;
        while !e.finished() && ticks < 2000 {
            e.tick(NodeFrame::default(), &mut rng);
            ticks += 1;
        }
        assert!(e.finished(), "{ticks}");
        assert!(!e.active);
        assert_eq!(e.live(), 0);
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
    fn the_random_size_table_is_filled_in_slot_order() {
        let mut d = SMALL.scaled(0.2);
        d.flags |= flags::RANDOM_SIZE;
        d.pool = 3;
        let mut rng = Scripted::new(&[], &[0.0, 0.5, 1.0, 0.25]);
        let e = Emitter::with_draws(d, 0.2, vec![Source::at_node()], &mut rng);
        // Four slots, each half-size × (0.5 + 0.5 u).
        assert!((e.slots[0].size - 6.0 * 0.5).abs() < 1e-6);
        assert!((e.slots[1].size - 6.0 * 0.75).abs() < 1e-6);
        assert!((e.slots[2].size - 6.0 * 1.0).abs() < 1e-6);
        assert!((e.slots[3].size - 6.0 * 0.625).abs() < 1e-6);
    }

    #[test]
    fn a_ring_keeps_its_sources_apart() {
        let mut rng = Scripted::new(&[], &[0.0, 0.4, 0.40, 0.8, 0.0, 0.25, 0.5, 0.99, 0.5, 0.5]);
        let sources = Source::ring(4, 40.0, &mut rng);
        assert_eq!(sources.len(), 4);
        for s in &sources {
            let r = s.offset.length();
            assert!((20.0..=40.0).contains(&r), "{r}");
            // The second axis points out of the ring.
            assert!((s.axes[1] - s.offset / r).length() < 1e-5);
        }
    }

    #[test]
    fn a_ring_source_sends_the_upward_draw_outward() {
        let mut rng = Scripted::new(&[], &[0.0, 0.5]);
        let sources = Source::ring(1, 10.0, &mut rng);
        let mut d = SMALL;
        d.speed = 83.0;
        d.spawn_radius = 0.0;
        let mut e = Emitter::new(d, 1.0, sources);
        e.enter(NodeFrame::default());
        // The velocity integers: x = 0, y = 1000 (1.0), z = 0.
        e.spawn(0, &mut Scripted::new(&[1000, 1000, 1000], &[]));
        let s = e.slots[0];
        // θ = 0: the source sits on +X and its y axis points out along +X.
        assert!((s.p - s.q - Vec3::new(2.0, 0.0, 0.0)).length() < 1e-5);
    }
}

#[cfg(test)]
mod uv_table_tests {
    use super::*;

    /// The lookup's rows: eight symmetries of four corners, halved, at four
    /// row origins.
    #[test]
    fn the_particle_uv_table_has_its_original_corners() {
        let table = particle_uv_table();
        // First symmetry: (0,0) (1,0) (0,1) (1,1), halved.
        assert_eq!(table[..4], [[0.0, 0.0], [0.5, 0.0], [0.0, 0.5], [0.5, 0.5]]);
        // Fifth symmetry (index 4): (0,1) (0,0) (1,1) (1,0).
        assert_eq!(
            table[16..20],
            [[0.0, 0.5], [0.0, 0.0], [0.5, 0.5], [0.5, 0.0]]
        );
        // Last symmetry: (1,0) (1,1) (0,0) (0,1).
        assert_eq!(
            table[28..32],
            [[0.5, 0.0], [0.5, 0.5], [0.0, 0.0], [0.0, 0.5]]
        );
        // Row 1 shifts by (½, 0), row 2 by (0, ½), row 3 by both.
        assert_eq!(table[32], [0.5, 0.0]);
        assert_eq!(table[64], [0.0, 0.5]);
        assert_eq!(table[3 * 32 + 3], [1.0, 1.0]);
        // Every entry stays in the unit square.
        assert!(
            table
                .iter()
                .all(|p| (0.0..=1.0).contains(&p[0]) && (0.0..=1.0).contains(&p[1]))
        );
    }

    /// The table is the precomputed form of the per-particle atlas variant
    /// the quad builder works out: cell `v >> 3`, symmetry `v & 7`.
    #[test]
    fn the_table_matches_the_variant_arithmetic() {
        let table = particle_uv_table();
        for variant in 0u8..32 {
            let cell = variant >> 3;
            let sym = SYMMETRIES[usize::from(variant & 7)];
            let origin = [f32::from(cell & 1) * 0.5, f32::from((cell >> 1) & 1) * 0.5];
            for (corner, uv) in sym.iter().enumerate() {
                let want = [origin[0] + uv[0] * 0.5, origin[1] + uv[1] * 0.5];
                let row = usize::from(cell);
                let got = table[row * 32 + usize::from(variant & 7) * 4 + corner];
                assert_eq!(got, want, "variant {variant} corner {corner}");
            }
        }
    }
}
