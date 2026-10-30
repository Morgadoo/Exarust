//! The electricity effects' rules (the original's `RFE_Effects_Electricity`
//! by address): the arc's colours and shader, its 84-byte parameter record,
//! the weapon, spark-burst and level-device presets, the charge that spends
//! an electrified item, the arc node's lifecycle, and the ribbon geometry
//! its segments produce.
//!
//! An arc is `tendrils` polylines of `segments` points between two ends,
//! jittered inside them and drawn as a camera-facing ribbon. The original's
//! per-tick tendril integration (`TTendrilEmitter`) is a separate component
//! outside this file's evidence; here the jitter is laid out once from the
//! record's jitter value and the ribbon is rebuilt from it, which reproduces
//! the presets, timing, segment count, width and glow exactly.

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use std::fmt;

pub struct ElectricityPlugin;

impl Plugin for ElectricityPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "arc.wesl");
        app.add_plugins(MaterialPlugin::<ArcMaterial>::default());
    }
}

/// The arc's electric colour (`EleCol`), sRGB.
pub const ELE_COLOUR: [f32; 3] = [0.65, 0.4, 1.0];
/// The arc's edge colour (`EdgCol`), sRGB.
pub const EDG_COLOUR: [f32; 3] = [0.15, 0.2, 0.4];
/// The spark's stored colour's green channel (53).
pub const SPARK_GREEN: f32 = 53.0;

/// The spark colour: `(0, 53, 0)`.
pub fn spark_colour() -> [f32; 3] {
    [0.0, SPARK_GREEN, 0.0]
}

/// Whether a part with this kind word and sub-kind byte is a live arc part:
/// kind 1 with a byte below 0xfe, or kind 0x11 with a byte below 0x7f.
pub fn is_live_part(kind: u16, sub: u8) -> bool {
    let kind = kind & 0x1f;
    (kind == 1 && sub < 0xfe) || (kind == 0x11 && sub < 0x7f)
}

/// Whether a part is an arc-spark part: kind 4 with a byte below 0xfe.
pub fn is_spark_part(kind: u16, sub: u8) -> bool {
    kind & 0x1f == 4 && sub < 0xfe
}

/// The random brightness scale in `[0.8, 1.0)` from a roll in `[0, 1)`.
pub fn random_brightness(roll: f64) -> f32 {
    (0.8 + roll * 0.2) as f32
}

/// A light an arc adds to the world: a voxel light (the weapon's and the
/// spark's) or the point light of a level device.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glow {
    /// Colour, sRGB.
    pub colour: [f32; 3],
    /// Base intensity.
    pub intensity: f32,
    /// Radius, cm.
    pub radius: f32,
    /// A level device's point light: its softness and multiplier; `None`
    /// for a voxel light.
    pub point: Option<PointGlow>,
}

/// A level device's point light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointGlow {
    pub softness: f32,
    pub multiplier: f32,
}

impl Glow {
    /// The electrified weapon's (and a record's, flag 0x200) voxel light.
    pub const WEAPON: Self = Self {
        colour: [0.6, 0.6, 1.0],
        intensity: 16.0,
        radius: 180.0,
        point: None,
    };
    /// The spark burst's voxel light.
    pub const SPARK: Self = Self {
        colour: [0.5, 0.5, 1.0],
        intensity: 8.0,
        radius: 100.0,
        point: None,
    };
}

/// How long an arc's tendrils live, in simulation ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifetime {
    /// Until something releases it (a level device).
    Permanent,
    /// A number of ticks, fixed.
    Ticks(u32),
    /// `base + Random(spread)` ticks, drawn when the arc is made (the
    /// run-time library's random integer).
    RandomTicks { base: u32, spread: u32 },
}

/// The arc's presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// An electrified weapon: 20 tendrils of 5 segments.
    Weapon,
    /// A spark burst: 20 tendrils of 5 segments, plus six more.
    SparkBurst,
    /// A level device, with its own point light.
    Device,
    /// An arc from a record.
    Record,
}

/// The emitter's default jitter (the tendril setup's).
const DEFAULT_JITTER: f32 = 0.1;

/// A fully specified arc.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcSpec {
    pub preset: Preset,
    /// The arc's flag word (the record's, or the preset's).
    pub flags: u32,
    pub tendrils: u32,
    pub segments: u32,
    /// The arc's size (cm): its width is twice it.
    pub size: f32,
    /// A segment's length (cm): four sizes, then the preset's or the
    /// record's scale.
    pub segment_length: f32,
    pub lifetime: Lifetime,
    /// The sound to loop.
    pub sound: Option<String>,
    /// The tendrils' jitter (the weapon's 0.075, the spark's 0.05, 0.1 by
    /// default).
    pub jitter: f32,
    /// A second tendril value a device sets to 0.5 and a record may set
    /// (its bytes 0x1C); read by the tendril tick, outside this evidence.
    pub sway: Option<f32>,
    /// The tendrils' reach (cm): the weapon's 24, the spark's 8, half a
    /// device's size. The extra tendrils of a burst start this far apart.
    pub reach: f32,
    /// A scale a device sets to its size × 0.005 and a record carries
    /// (its bytes 0x50); read by the tendril tick, outside this evidence.
    pub drift: f32,
    /// The light the arc adds.
    pub glow: Option<Glow>,
    /// A record's flag 0x300: the arc registers an electric flow with the
    /// voxel GI instead of a light.
    pub gi_flow: bool,
    /// The arc's start offset from its node (cm).
    pub offset: Vec3,
    /// The arc's direction from its start (unit): +X unless a record or a
    /// device sets it.
    pub direction: Vec3,
    /// Its length (cm): the segments laid end to end, or a device's.
    pub length: f32,
    /// A sustained arc renews its tendrils when they expire, until it is
    /// released; the others finish with their tendrils.
    pub sustained: bool,
}

impl ArcSpec {
    /// An electrified weapon's arc: 20 tendrils of 5 segments, size 2 cm
    /// (segments of three-quarters of 8 cm, width 4 cm), sustained, 10–19
    /// ticks a renewal, the electricity03 hum, jitter 0.075, reach 24 cm
    /// and a blue voxel light (0.6, 0.6, 1) of 16 at 180 cm, 53 cm up its
    /// node.
    pub fn weapon() -> Self {
        let size = 2.0;
        let segment_length = size * 4.0 * 0.75;
        Self {
            preset: Preset::Weapon,
            flags: 0,
            tendrils: 20,
            segments: 5,
            size,
            segment_length,
            lifetime: Lifetime::RandomTicks {
                base: 10,
                spread: 10,
            },
            sound: Some("electricity03".to_string()),
            jitter: 0.075,
            sway: None,
            reach: 24.0,
            drift: 0.0,
            glow: Some(Glow::WEAPON),
            gi_flow: false,
            offset: Vec3::new(0.0, 53.0, 0.0),
            direction: Vec3::X,
            length: segment_length * 5.0,
            sustained: true,
        }
    }

    /// A spark burst: 20 tendrils of 5 segments at size 3 (segments of
    /// twice 12 cm, width 6 cm) with six extra tendrils, one-shot over
    /// 10–19 ticks, jitter 0.05, reach 8 cm and the smaller blue light
    /// (0.5, 0.5, 1) of 8 at 100 cm.
    pub fn spark_burst() -> Self {
        let size = 3.0;
        let segment_length = size * 4.0 * 2.0;
        Self {
            preset: Preset::SparkBurst,
            flags: 0,
            tendrils: 20 + 6,
            segments: 5,
            size,
            segment_length,
            lifetime: Lifetime::RandomTicks {
                base: 10,
                spread: 10,
            },
            sound: None,
            jitter: 0.05,
            sway: None,
            reach: 8.0,
            drift: 0.0,
            glow: Some(Glow::SPARK),
            gi_flow: false,
            offset: Vec3::ZERO,
            direction: Vec3::X,
            length: segment_length * 5.0,
            sustained: false,
        }
    }

    /// A level device's arc from its object record: `duration` in
    /// thousandths sets the size, the length in cm sets the segment count
    /// (`round(length / (4 · size))`, ten below two), and `branches` the
    /// tendrils (ten when zero). It is permanent, with the electricity01
    /// hum and a point light (0.6, 0.65, 1) of half the size at 80 sizes,
    /// multiplier 2, softness 5; no voxel light.
    pub fn device(duration: u16, length: u16, branches: i16, direction: Vec3) -> Self {
        let size = f32::from(duration) * 0.001;
        let length = f32::from(length);
        let segments = (length / (size * 4.0)).round_ties_even();
        let segments = if segments.is_finite() && segments >= 2.0 {
            (segments as u32).min(4096)
        } else {
            10
        };
        let branches = if branches == 0 {
            10
        } else {
            branches.max(1) as u32
        };
        Self {
            preset: Preset::Device,
            flags: 0,
            tendrils: branches,
            segments,
            size,
            segment_length: size * 4.0,
            lifetime: Lifetime::Permanent,
            sound: Some("electricity01".to_string()),
            jitter: DEFAULT_JITTER,
            sway: Some(0.5),
            reach: size * 0.5,
            drift: size * 0.005,
            glow: Some(Glow {
                colour: [0.6, 0.65, 1.0],
                intensity: size * 0.5,
                radius: size * 80.0,
                point: Some(PointGlow {
                    softness: 5.0,
                    multiplier: 2.0,
                }),
            }),
            gi_flow: false,
            offset: Vec3::ZERO,
            direction: direction.normalize_or(Vec3::X),
            length,
            sustained: false,
        }
    }

    /// The 84-byte arc record's fields. Flag 1 makes the arc sustained,
    /// renewing its tendrils every `n + Random(n)` ticks; without it the
    /// arc is one-shot over `n` ticks, its first segment along the record's
    /// direction.
    pub fn from_record(record: &[u8]) -> Result<Self, ArcError> {
        let r = ArcRecord::parse(record)?;
        const EPSILON: f32 = 1.192_092_9e-7;
        let mut segment_length = r.size * 4.0;
        if r.scale > EPSILON {
            segment_length *= r.scale;
        }
        let tendrils = r.tendrils.clamp(1, 1024);
        let segments = r.segments.clamp(2, 4096);
        let sustained = r.flags & 1 != 0;
        let spec = Self {
            preset: Preset::Record,
            flags: r.flags,
            tendrils,
            segments,
            size: r.size,
            segment_length,
            lifetime: if sustained {
                Lifetime::RandomTicks {
                    base: r.lifetime,
                    spread: r.lifetime,
                }
            } else {
                Lifetime::Ticks(r.lifetime)
            },
            sound: r.sound,
            jitter: if r.jitter > EPSILON {
                r.jitter
            } else {
                DEFAULT_JITTER
            },
            sway: (r.sway > EPSILON).then_some(r.sway),
            reach: r.reach,
            drift: r.drift,
            glow: (r.flags & 0x300 == 0x200).then_some(Glow::WEAPON),
            gi_flow: r.flags & 0x300 == 0x300,
            offset: r.offset,
            direction: r.direction.normalize_or(Vec3::X),
            length: segment_length * segments as f32,
            sustained,
        };
        Ok(spec)
    }

    /// The arc's width (cm): twice its size.
    pub fn width(&self) -> f32 {
        self.size * 2.0
    }
}

/// Errors from an arc's data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcError {
    /// The record is shorter than its 84 bytes.
    Short,
}

impl fmt::Display for ArcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Short => write!(f, "the arc record is shorter than 84 bytes"),
        }
    }
}

impl std::error::Error for ArcError {}

/// The 84-byte parameter record of an arc.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcRecord {
    pub flags: u32,
    pub tendrils: u32,
    pub segments: u32,
    /// The tendrils' lifetime, ticks.
    pub lifetime: u32,
    pub size: f32,
    /// The segment length's scale (used above the epsilon).
    pub scale: f32,
    /// A word the original copies to the emitter unread here.
    pub reserved: u32,
    /// The tendrils' second value (used above the epsilon).
    pub sway: f32,
    /// The tendrils' reach (cm).
    pub reach: f32,
    /// A 15-character sound name; none when its first byte is zero.
    pub sound: Option<String>,
    /// The tendrils' jitter (used above the epsilon).
    pub jitter: f32,
    pub offset: Vec3,
    pub direction: Vec3,
    /// The tendrils' drift scale.
    pub drift: f32,
}

impl ArcRecord {
    /// Parses the record; short data is an error, never a panic.
    pub fn parse(data: &[u8]) -> Result<Self, ArcError> {
        if data.len() < 84 {
            return Err(ArcError::Short);
        }
        let u =
            |at: usize| u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
        let f =
            |at: usize| f32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
        let v = |at: usize| Vec3::new(f(at), f(at + 4), f(at + 8));
        let sound = {
            let bytes = &data[0x24..0x33];
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            (end > 0).then(|| String::from_utf8_lossy(&bytes[..end]).into_owned())
        };
        Ok(Self {
            flags: u(0),
            tendrils: u(4),
            segments: u(8),
            lifetime: u(0x0C),
            size: f(0x10),
            scale: f(0x14),
            reserved: u(0x18),
            sway: f(0x1C),
            reach: f(0x20),
            sound,
            jitter: f(0x34),
            offset: v(0x38),
            direction: v(0x44),
            drift: f(0x50),
        })
    }
}

/// The charge an electrified item carries; a hit adds a step and the step
/// that would reach 255 releases the effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Charge {
    pub value: u8,
}

/// What a hit did to the charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeOutcome {
    /// The charge rose.
    Charged,
    /// The charge reached its limit: the effect is released.
    Spent,
}

impl Charge {
    /// A hit's step for a part whose kind has bit 0x10 (kind 0x11).
    pub const HEAVY: u8 = 0x80;
    /// A hit's step otherwise (kind 1).
    pub const NORMAL: u8 = 10;

    /// Adds a hit's step; the step that would reach 255 fills the charge and
    /// spends it instead. `heavy` is the part kind's bit 0x10.
    pub fn hit(&mut self, heavy: bool) -> ChargeOutcome {
        let step = if heavy { Self::HEAVY } else { Self::NORMAL };
        if i32::from(self.value) < 0xff - i32::from(step) {
            self.value += step;
            ChargeOutcome::Charged
        } else {
            self.value = 0xff;
            ChargeOutcome::Spent
        }
    }
}

/// An arc node's presence and state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArcNode {
    /// The node's state word's bit 0: it is finishing.
    pub finished: bool,
}

/// What an equipment check should do with an electrified item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcAction {
    /// Nothing.
    None,
    /// Build the weapon's arc.
    Create,
    /// Release the existing arc.
    Release,
}

/// The electrified-item check: with a live part, create an arc when there
/// is none or the old one is finishing; with a dead part, release a live
/// arc.
pub fn electrified_action(has_item: bool, part_live: bool, arc: Option<ArcNode>) -> ArcAction {
    if !has_item {
        return ArcAction::None;
    }
    let live = arc.is_some_and(|node| !node.finished);
    match (part_live, live) {
        (true, false) => ArcAction::Create,
        (false, true) => ArcAction::Release,
        _ => ArcAction::None,
    }
}

/// A child node of an electrified item, as the arc checks see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElectrifiedEffect {
    /// The node's emitter kind; the high byte is 0x21 for an arc.
    pub kind: u32,
    /// The arc renews its tendrils.
    pub sustained: bool,
    /// The arc's state word; bit 0 means it is finishing.
    pub state: i32,
}

impl ElectrifiedEffect {
    fn is_arc(&self) -> bool {
        self.kind & 0xff00_0000 == 0x2100_0000
    }
}

/// The item that owns an electrified arc: whether it carries the hit and
/// release callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArcOwner {
    pub callbacks: bool,
}

/// Releases the electrified item's arc: its first arc child stops renewing
/// and starts finishing; the owner's callbacks clear whether or not a child
/// matched.
pub fn clear_electrified_effect(children: &mut [ElectrifiedEffect], owner: &mut ArcOwner) -> bool {
    let matched = children.iter_mut().find(|c| c.is_arc()).map(|arc| {
        arc.sustained = false;
        arc.state = 1;
    });
    owner.callbacks = false;
    matched.is_some()
}

/// The arc check of an item the equipment check found live: it hooks the
/// owner's callbacks (when there is an owner) and says whether to build a
/// new weapon arc: when the item has no arc child, or its first one is
/// finishing.
pub fn maintain_arc(children: &[ElectrifiedEffect], owner: &mut ArcOwner, has_owner: bool) -> bool {
    if has_owner {
        owner.callbacks = true;
    }
    children
        .iter()
        .find(|c| c.is_arc())
        .is_none_or(|arc| arc.state & 1 != 0)
}

/// One arc: its preset, its jittered tendrils and its age.
#[derive(Debug, Clone)]
pub struct Arc {
    pub spec: ArcSpec,
    pub start: Vec3,
    pub end: Vec3,
    pub age: u32,
    /// Ticks the tendrils live; 0 is permanent.
    pub lifetime: u32,
    tendrils: Vec<Vec<Vec3>>,
    /// The texture offset the shader adds (`TexOfs`): the original passes
    /// `(0, scroll)`, the scroll kept by the tendril tick.
    pub tex_offset: Vec2,
    rng: LocalRng,
}

impl Arc {
    /// Builds an arc between two points. The interior points are jittered
    /// from the spec's `jitter` (a model of the original's per-tick tendril
    /// motion: the tendril simulation itself is outside this component's
    /// evidence).
    pub fn new(spec: ArcSpec, start: Vec3, end: Vec3) -> Self {
        let rng = LocalRng::new(start.x.to_bits() ^ end.z.to_bits() ^ 0x9E37_79B9);
        let mut arc = Self {
            spec,
            start,
            end,
            age: 0,
            lifetime: 0,
            tendrils: Vec::new(),
            tex_offset: Vec2::ZERO,
            rng,
        };
        arc.renew();
        arc
    }

    /// Draws the tendrils' lifetime and lays them out.
    fn renew(&mut self) {
        let spec = &self.spec;
        // A record from an untrusted file cannot ask for an unbounded mesh.
        let n = spec.tendrils.clamp(1, 1024) as usize;
        let s = spec.segments.clamp(2, 4096) as usize;
        let rng = &mut self.rng;
        self.lifetime = match spec.lifetime {
            Lifetime::Permanent => 0,
            Lifetime::Ticks(t) => t.max(1),
            Lifetime::RandomTicks { base, spread } => base.saturating_add(rng.int(spread)).max(1),
        };
        let (start, end) = (self.start, self.end);
        let direction = (end - start).normalize_or(spec.direction);
        let (_, right, up) = orthonormal(direction);
        self.tendrils.clear();
        for t in 0..n {
            let mut points = Vec::with_capacity(s);
            let spread = if t < 20 { 0.0 } else { spec.reach };
            let mut from = start;
            if spread > 0.0 {
                from += right * ((rng.unit() - 0.5) * spread) + up * ((rng.unit() - 0.5) * spread);
            }
            for k in 0..s {
                let f = k as f32 / (s - 1) as f32;
                let base = from.lerp(end, f);
                let bulge = (std::f32::consts::PI * f).sin();
                // [guess] The jitter is a per-tick step in the original's
                // tendril; over a tendril's life it grows to a few segment
                // lengths. Lay it out at that scale.
                let amplitude = spec.jitter * spec.segment_length * 3.0 * bulge;
                let offset = if k == 0 || k + 1 == s || amplitude == 0.0 {
                    Vec3::ZERO
                } else {
                    right * ((rng.unit() - 0.5) * 2.0 * amplitude)
                        + up * ((rng.unit() - 0.5) * 2.0 * amplitude)
                };
                points.push(base + offset);
            }
            self.tendrils.push(points);
        }
    }

    /// The spec's tendrils, for a renderer or a test.
    pub fn tendrils(&self) -> &[Vec<Vec3>] {
        &self.tendrils
    }

    /// Stops a sustained arc renewing: it finishes with its tendrils.
    pub fn release(&mut self) {
        self.spec.sustained = false;
        if self.lifetime == 0 {
            self.lifetime = self.age.max(1);
        }
    }

    /// One tick: ages the arc and scrolls its texture; a sustained arc
    /// renews its tendrils when they expire. False once it is finished (a
    /// permanent arc never is).
    pub fn tick(&mut self) -> bool {
        self.age = self.age.saturating_add(1);
        // The texture scroll the tendril emitter keeps: a step of 0.015 a
        // tick added in double precision, and past 2 it drops back by one.
        // The emitter's own tick is modelled in the game's arc emitter; this
        // is the shader's offset for one arc.
        self.tex_offset.y = (f64::from(self.tex_offset.y) + 0.015) as f32;
        if 2.0 < self.tex_offset.y {
            self.tex_offset.y -= 1.0;
        }
        if self.lifetime == 0 || self.age < self.lifetime {
            return true;
        }
        if self.spec.sustained {
            self.age = 0;
            self.renew();
            true
        } else {
            false
        }
    }

    /// The arc's ribbon: two triangles per segment, camera-facing. U runs
    /// along the tendril and V across it, the shader's ridge peaking at the
    /// ribbon's middle.
    pub fn ribbon(&self, view: Vec3) -> ArcMesh {
        let mut mesh = ArcMesh::default();
        let half = self.spec.width() * 0.5;
        for points in &self.tendrils {
            let last = (points.len() - 1) as f32;
            for k in 0..points.len() - 1 {
                let a = points[k];
                let b = points[k + 1];
                let along = (b - a).normalize_or(self.spec.direction);
                let side = along.cross(view).normalize_or(Vec3::Y) * half;
                let u0 = k as f32 / last;
                let u1 = (k + 1) as f32 / last;
                let base = mesh.positions.len() as u32;
                mesh.positions
                    .extend_from_slice(&[a - side, a + side, b - side, b + side]);
                mesh.uvs.extend_from_slice(&[
                    Vec2::new(u0, 0.0),
                    Vec2::new(u0, 1.0),
                    Vec2::new(u1, 0.0),
                    Vec2::new(u1, 1.0),
                ]);
                mesh.colours.extend_from_slice(&[[1.0; 4]; 4]);
                mesh.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base + 2,
                    base + 1,
                    base + 3,
                ]);
            }
        }
        mesh
    }
}

/// An arc's ribbon mesh data.
#[derive(Debug, Clone, Default)]
pub struct ArcMesh {
    pub positions: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub colours: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

/// The arc's local random stream: the engine's integer LCG, standing in for
/// the run-time library's process-wide random integers (whose seed is not
/// reproducible).
#[derive(Debug, Clone)]
struct LocalRng {
    state: u32,
}

impl LocalRng {
    fn new(seed: u32) -> Self {
        Self { state: seed | 1 }
    }

    fn int(&mut self, n: u32) -> u32 {
        self.state = self.state.wrapping_mul(0x0808_8405).wrapping_add(1);
        ((u64::from(self.state) * u64::from(n)) >> 32) as u32
    }

    fn unit(&mut self) -> f32 {
        self.int(1 << 24) as f32 * (1.0 / 16_777_216.0)
    }
}

/// A right-handed basis around `direction`.
fn orthonormal(direction: Vec3) -> (Vec3, Vec3, Vec3) {
    let right = if direction.y.abs() < 0.99 {
        direction.cross(Vec3::Y).normalize_or(Vec3::X)
    } else {
        direction.cross(Vec3::Z).normalize_or(Vec3::X)
    };
    let up = right.cross(direction).normalize_or(Vec3::Y);
    (direction, right, up)
}

/// The arc's shader material: `_ElecMat01`, unlit and additive, its
/// emissive texture times the electric and edge colours.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ArcMaterial {
    /// The scrolling arc texture (`EmiTex`).
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    /// `EleCol`.
    #[uniform(2)]
    pub ele_colour: Vec4,
    /// `EdgCol`.
    #[uniform(3)]
    pub edg_colour: Vec4,
    /// `TexOfs`, added to the UV before the quarter-scale lookup.
    #[uniform(4)]
    pub tex_offset: Vec4,
}

impl ArcMaterial {
    /// The material with the original's two colours.
    pub fn new(texture: Handle<Image>) -> Self {
        Self {
            texture,
            ele_colour: Vec4::new(ELE_COLOUR[0], ELE_COLOUR[1], ELE_COLOUR[2], 1.0),
            edg_colour: Vec4::new(EDG_COLOUR[0], EDG_COLOUR[1], EDG_COLOUR[2], 1.0),
            tex_offset: Vec4::ZERO,
        }
    }
}

impl Material for ArcMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("arc.wesl")).with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode {
        // Blend word 0x161: source alpha, destination one.
        AlphaMode::Add
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // The material is two-sided.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spark_colour_is_green() {
        assert_eq!(spark_colour(), [0.0, 53.0, 0.0]);
    }

    #[test]
    fn the_part_kinds() {
        assert!(is_live_part(1, 0xfd));
        assert!(!is_live_part(1, 0xfe));
        assert!(is_live_part(0x11, 0x7e));
        assert!(!is_live_part(0x11, 0x7f));
        assert!(is_spark_part(4, 0));
        assert!(!is_spark_part(5, 0));
    }

    #[test]
    fn the_random_brightness_spans_its_range() {
        assert_eq!(random_brightness(0.0), 0.8);
        assert!((random_brightness(0.5) - 0.9).abs() < 1e-6);
    }

    #[test]
    fn the_electrified_effect_clears() {
        let mut children = [
            ElectrifiedEffect {
                kind: 0x2200_0001,
                sustained: true,
                state: 0,
            },
            ElectrifiedEffect {
                kind: 0x2100_0001,
                sustained: true,
                state: 0,
            },
        ];
        let mut owner = ArcOwner { callbacks: true };
        assert!(clear_electrified_effect(&mut children, &mut owner));
        assert!(children[0].sustained);
        assert_eq!(children[0].state, 0);
        assert!(!children[1].sustained);
        assert_eq!(children[1].state, 1);
        assert!(!owner.callbacks);
        // No arc child: only the callbacks clear.
        let mut owner = ArcOwner { callbacks: true };
        assert!(!clear_electrified_effect(&mut children[..1], &mut owner));
        assert!(!owner.callbacks);
    }

    #[test]
    fn the_arc_check_builds_when_there_is_no_live_arc() {
        let mut owner = ArcOwner::default();
        assert!(maintain_arc(&[], &mut owner, true));
        assert!(owner.callbacks);
        let live = ElectrifiedEffect {
            kind: 0x2100_0000,
            sustained: true,
            state: 0,
        };
        assert!(!maintain_arc(&[live], &mut owner, false));
        let finishing = ElectrifiedEffect { state: 1, ..live };
        assert!(maintain_arc(&[finishing], &mut owner, false));
    }

    #[test]
    fn the_weapon_preset() {
        let s = ArcSpec::weapon();
        assert_eq!(s.tendrils, 20);
        assert_eq!(s.segments, 5);
        assert_eq!(s.size, 2.0);
        // The three-quarters scale is on the segments, not the width.
        assert_eq!(s.segment_length, 6.0);
        assert_eq!(s.width(), 4.0);
        assert_eq!(s.length, 30.0);
        assert!(s.sustained);
        assert_eq!(s.sound.as_deref(), Some("electricity03"));
        assert_eq!(s.glow, Some(Glow::WEAPON));
        assert_eq!(s.offset, Vec3::new(0.0, 53.0, 0.0));
        assert_eq!(s.jitter, 0.075);
        assert_eq!(s.reach, 24.0);
        assert_eq!(
            s.lifetime,
            Lifetime::RandomTicks {
                base: 10,
                spread: 10
            }
        );
    }

    #[test]
    fn the_spark_burst_preset() {
        let s = ArcSpec::spark_burst();
        assert_eq!(s.tendrils, 26);
        assert_eq!(s.segments, 5);
        assert_eq!(s.size, 3.0);
        assert_eq!(s.width(), 6.0);
        // 4 · 3 cm, doubled after the extra tendrils.
        assert_eq!(s.segment_length, 24.0);
        assert_eq!(s.reach, 8.0);
        assert_eq!(s.glow, Some(Glow::SPARK));
        assert!(!s.sustained);
    }

    #[test]
    fn the_device_preset() {
        // Duration 1000 → size 1; length 200 → 50 segments; branches 0 →
        // ten.
        let s = ArcSpec::device(1000, 200, 0, Vec3::NEG_Z);
        assert!((s.size - 1.0).abs() < 1e-6);
        assert_eq!(s.segments, 50);
        assert_eq!(s.tendrils, 10);
        assert_eq!(s.sound.as_deref(), Some("electricity01"));
        assert_eq!(s.sway, Some(0.5));
        assert_eq!(s.jitter, 0.1);
        assert!((s.reach - 0.5).abs() < 1e-6);
        assert!((s.drift - 0.005).abs() < 1e-9);
        let glow = s.glow.unwrap();
        assert!((glow.radius - 80.0).abs() < 1e-6);
        assert!((glow.intensity - 0.5).abs() < 1e-6);
        assert_eq!(glow.colour, [0.6, 0.65, 1.0]);
        let point = glow.point.unwrap();
        assert_eq!(point.softness, 5.0);
        assert_eq!(point.multiplier, 2.0);
        // A length too short for two segments falls back to ten.
        let s = ArcSpec::device(1000, 2, 0, Vec3::NEG_Z);
        assert_eq!(s.segments, 10);
        // A zero duration cannot divide its way into a huge mesh.
        let s = ArcSpec::device(0, 100, 0, Vec3::NEG_Z);
        assert_eq!(s.segments, 10);
    }

    fn record(flags: u32, scale: f32) -> [u8; 84] {
        let mut data = [0u8; 84];
        data[0..4].copy_from_slice(&flags.to_le_bytes());
        data[4..8].copy_from_slice(&7u32.to_le_bytes());
        data[8..12].copy_from_slice(&9u32.to_le_bytes());
        data[0x0C..0x10].copy_from_slice(&25u32.to_le_bytes());
        data[0x10..0x14].copy_from_slice(&1.0f32.to_le_bytes());
        data[0x14..0x18].copy_from_slice(&scale.to_le_bytes());
        data[0x1C..0x20].copy_from_slice(&0.25f32.to_le_bytes());
        data[0x20..0x24].copy_from_slice(&12.0f32.to_le_bytes());
        data[0x24..0x2B].copy_from_slice(b"elechit");
        data[0x34..0x38].copy_from_slice(&0.2f32.to_le_bytes());
        data[0x38..0x3C].copy_from_slice(&1.0f32.to_le_bytes());
        data[0x44..0x48].copy_from_slice(&0.0f32.to_le_bytes());
        data[0x48..0x4C].copy_from_slice(&1.0f32.to_le_bytes());
        data[0x50..0x54].copy_from_slice(&0.01f32.to_le_bytes());
        data
    }

    #[test]
    fn the_record_parses_and_short_data_is_an_error() {
        assert_eq!(ArcRecord::parse(&[0; 10]), Err(ArcError::Short));
        let data = record(0x201, 0.5);
        let r = ArcRecord::parse(&data).expect("the record");
        assert_eq!(r.flags, 0x201);
        assert_eq!(r.tendrils, 7);
        assert_eq!(r.segments, 9);
        assert_eq!(r.lifetime, 25);
        assert_eq!(r.sound.as_deref(), Some("elechit"));
        assert_eq!(r.jitter, 0.2);
        assert_eq!(r.sway, 0.25);
        assert_eq!(r.reach, 12.0);
        assert_eq!(r.offset, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(r.direction, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(r.drift, 0.01);
        let s = ArcSpec::from_record(&data).expect("the spec");
        assert_eq!(s.tendrils, 7);
        assert_eq!(s.segments, 9);
        assert_eq!(s.size, 1.0);
        // The scale is on the segments: 4 · 0.5; the width stays 2.
        assert_eq!(s.segment_length, 2.0);
        assert_eq!(s.width(), 2.0);
        // Flag 1: sustained, renewing every 25 + Random(25) ticks.
        assert!(s.sustained);
        assert_eq!(
            s.lifetime,
            Lifetime::RandomTicks {
                base: 25,
                spread: 25
            }
        );
        assert_eq!(s.glow, Some(Glow::WEAPON));
        assert!(!s.gi_flow);
        // Without flag 1 it is one-shot; flag 0x300 is a GI flow, no light;
        // a zero jitter keeps the default and an empty name no sound.
        let mut data = record(0x300, 0.0);
        data[0x34..0x38].copy_from_slice(&0.0f32.to_le_bytes());
        data[0x24] = 0;
        let s = ArcSpec::from_record(&data).expect("the spec");
        assert!(!s.sustained);
        assert_eq!(s.lifetime, Lifetime::Ticks(25));
        assert_eq!(s.segment_length, 4.0);
        assert_eq!(s.glow, None);
        assert!(s.gi_flow);
        assert_eq!(s.jitter, 0.1);
        assert_eq!(s.sound, None);
    }

    #[test]
    fn the_charge_spends_at_the_limit() {
        let mut charge = Charge { value: 0x00 };
        assert_eq!(charge.hit(false), ChargeOutcome::Charged);
        assert_eq!(charge.value, 10);
        // Below 255 − 10 the step is added: 244 → 254.
        charge.value = 0xf4;
        assert_eq!(charge.hit(false), ChargeOutcome::Charged);
        assert_eq!(charge.value, 0xfe);
        // At 245 or more it fills and spends instead.
        charge.value = 0xf5;
        assert_eq!(charge.hit(false), ChargeOutcome::Spent);
        assert_eq!(charge.value, 0xff);
        let mut heavy = Charge { value: 0x7e };
        assert_eq!(heavy.hit(true), ChargeOutcome::Charged);
        assert_eq!(heavy.value, 0xfe);
        assert_eq!(heavy.hit(true), ChargeOutcome::Spent);
    }

    #[test]
    fn the_electrified_check() {
        assert_eq!(electrified_action(false, true, None), ArcAction::None);
        assert_eq!(electrified_action(true, true, None), ArcAction::Create);
        assert_eq!(
            electrified_action(true, true, Some(ArcNode { finished: true })),
            ArcAction::Create
        );
        assert_eq!(
            electrified_action(true, true, Some(ArcNode { finished: false })),
            ArcAction::None
        );
        assert_eq!(
            electrified_action(true, false, Some(ArcNode { finished: false })),
            ArcAction::Release
        );
        assert_eq!(electrified_action(true, false, None), ArcAction::None);
    }

    #[test]
    fn an_arc_has_the_presets_segments_and_ends() {
        let spec = ArcSpec::weapon();
        let arc = Arc::new(spec, Vec3::ZERO, Vec3::new(30.0, 0.0, 0.0));
        assert_eq!(arc.tendrils.len(), 20);
        for points in arc.tendrils() {
            assert_eq!(points.len(), 5);
            assert!((points[0] - Vec3::ZERO).length() < 1e-5);
            assert!((points[4] - Vec3::new(30.0, 0.0, 0.0)).length() < 1e-5);
        }
        assert!((10..20).contains(&arc.lifetime), "{}", arc.lifetime);
    }

    #[test]
    fn an_arc_ribbon_has_a_quad_per_segment_and_v_across() {
        let spec = ArcSpec::weapon();
        let arc = Arc::new(spec, Vec3::ZERO, Vec3::new(30.0, 0.0, 0.0));
        let mesh = arc.ribbon(Vec3::Z);
        let quads = 20 * 4 * 4;
        assert_eq!(mesh.positions.len(), quads);
        assert_eq!(mesh.indices.len(), quads / 4 * 6);
        // The ribbon is the arc's width across, two sizes.
        assert!(((mesh.positions[1] - mesh.positions[0]).length() - 4.0).abs() < 1e-4);
        // V is the across coordinate, U the along one.
        assert_eq!(mesh.uvs[0], Vec2::new(0.0, 0.0));
        assert_eq!(mesh.uvs[1], Vec2::new(0.0, 1.0));
        assert_eq!(mesh.uvs[2].y, 0.0);
        assert!(mesh.uvs[2].x > 0.0);
    }

    #[test]
    fn the_arc_scrolls_its_texture_by_a_tick_and_wraps() {
        let mut arc = Arc::new(ArcSpec::weapon(), Vec3::ZERO, Vec3::X * 30.0);
        assert_eq!(arc.tex_offset, Vec2::ZERO);
        arc.tick();
        assert!((arc.tex_offset.y - 0.015).abs() < 1e-6);
        // 134 ticks of 0.015 pass 2 (2.01), which drops back by one.
        for _ in 0..133 {
            arc.tick();
        }
        assert!(
            (arc.tex_offset.y - 1.01).abs() < 1e-4,
            "{}",
            arc.tex_offset.y
        );
        assert_eq!(arc.tex_offset.x, 0.0);
    }

    #[test]
    fn a_one_shot_arc_expires_after_its_ticks() {
        let mut spec = ArcSpec::spark_burst();
        spec.lifetime = Lifetime::Ticks(3);
        let mut arc = Arc::new(spec, Vec3::ZERO, Vec3::X);
        assert!(arc.tick());
        assert!(arc.tick());
        assert!(!arc.tick());
    }

    #[test]
    fn a_sustained_arc_renews_until_released() {
        let mut spec = ArcSpec::weapon();
        spec.lifetime = Lifetime::Ticks(3);
        let mut arc = Arc::new(spec, Vec3::ZERO, Vec3::X * 30.0);
        for _ in 0..100 {
            assert!(arc.tick());
        }
        arc.release();
        let mut ticks = 0;
        while arc.tick() {
            ticks += 1;
            assert!(ticks < 10);
        }
    }

    #[test]
    fn a_permament_arc_never_expires() {
        let spec = ArcSpec::device(1000, 100, 0, Vec3::NEG_Z);
        let mut arc = Arc::new(spec, Vec3::ZERO, Vec3::NEG_Z);
        for _ in 0..10_000 {
            assert!(arc.tick());
        }
    }
}
