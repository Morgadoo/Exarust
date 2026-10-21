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

/// A light an arc adds to the world: a voxel light or the point light of a
/// level device.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glow {
    /// Colour, sRGB.
    pub colour: [f32; 3],
    /// Base intensity.
    pub intensity: f32,
    /// Radius, cm.
    pub radius: f32,
    /// A level device's point light: its radius (cm), softness and
    /// multiplier; `None` for a voxel light.
    pub point: Option<PointGlow>,
}

/// A level device's point light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointGlow {
    pub radius: f32,
    pub softness: f32,
    pub multiplier: f32,
}

impl Glow {
    /// The electrified weapon's and the shock's voxel light.
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

/// How long an arc lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifetime {
    /// Until something releases it.
    Permanent,
    /// A number of ticks, fixed.
    Ticks(u32),
    /// A number of ticks, `base + rand(spread)` drawn when the arc is made
    /// (the original's weapon and spark presets).
    RandomTicks { base: u32, spread: u32 },
    /// Milliseconds from a record.
    Millis(u32),
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

/// A fully specified arc.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcSpec {
    pub preset: Preset,
    /// The arc's flag word (the record's, or the preset's).
    pub flags: u32,
    pub tendrils: u32,
    pub segments: u32,
    /// The segment size (cm): the unit of the arc's length and width.
    pub size: f32,
    /// A length multiplier applied to the size (cm).
    pub scale: f32,
    pub lifetime: Lifetime,
    /// The sound to loop.
    pub sound: Option<String>,
    /// The zig-zag's amplitude, a fraction of the size.
    pub jitter: f32,
    /// The distance extra tendrils start apart (cm), used by bursts.
    pub burst_radius: f32,
    /// The light the arc adds.
    pub glow: Option<Glow>,
    /// The arc's start offset from its node (cm).
    pub offset: Vec3,
    /// The arc's direction from its start (unit).
    pub direction: Vec3,
    /// Its length (cm).
    pub length: f32,
    /// One-shot arcs hide when their lifetime runs out.
    pub one_shot: bool,
}

impl ArcSpec {
    /// An electrified weapon's arc: 20 tendrils of 5 segments, size 2 cm
    /// (three-quarters after the preset), 10–19 ticks, the electricity03
    /// hum and a blue voxel light (0.6, 0.6, 1) of 16 at 180 cm.
    pub fn weapon() -> Self {
        Self {
            preset: Preset::Weapon,
            flags: 0,
            tendrils: 20,
            segments: 5,
            size: 2.0 * 0.75,
            scale: 1.0,
            lifetime: Lifetime::RandomTicks {
                base: 10,
                spread: 10,
            },
            sound: Some("electricity03".to_string()),
            jitter: 0.075,
            burst_radius: 24.0,
            glow: Some(Glow::WEAPON),
            offset: Vec3::new(0.0, 53.0, 0.0),
            direction: Vec3::NEG_Z,
            length: 2.0 * 0.75 * 4.0 * 5.0,
            one_shot: true,
        }
    }

    /// A spark burst: 20 tendrils of 5 segments at 3 cm, with six extra
    /// tendrils and the smaller blue light (0.5, 0.5, 1) of 8 at 100 cm.
    pub fn spark_burst() -> Self {
        Self {
            preset: Preset::SparkBurst,
            flags: 0,
            tendrils: 20 + 6,
            segments: 5,
            size: 3.0,
            scale: 1.0,
            lifetime: Lifetime::RandomTicks {
                base: 10,
                spread: 10,
            },
            sound: None,
            jitter: 0.05,
            burst_radius: 8.0,
            glow: Some(Glow::SPARK),
            offset: Vec3::ZERO,
            direction: Vec3::NEG_Z,
            length: 3.0 * 4.0 * 5.0,
            one_shot: true,
        }
    }

    /// A level device's arc from its object record: `duration` in
    /// thousandths sets the size, the length in cm sets the segment count
    /// (`length / (4 · size)`, at least ten), and `branches` the tendrils
    /// (ten when zero). Its point light is colour (0.6, 0.65, 1), multiplier
    /// 2, softness 5, radius `80 · size`, with the 16-at-180 cm voxel light.
    pub fn device(duration: u16, length: u16, branches: i16, direction: Vec3) -> Self {
        let size = f32::from(duration) * 0.001;
        let segments = (f32::from(length) / (size * 4.0)).round_ties_even() as i32;
        let segments = if segments < 2 { 10 } else { segments as u32 };
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
            scale: 1.0,
            lifetime: Lifetime::Permanent,
            sound: Some("electricity01".to_string()),
            jitter: 0.5,
            burst_radius: 0.0,
            glow: Some(Glow {
                colour: [0.6, 0.65, 1.0],
                intensity: 16.0,
                radius: 180.0,
                point: Some(PointGlow {
                    radius: size * 80.0,
                    softness: 5.0,
                    multiplier: 2.0,
                }),
            }),
            offset: Vec3::ZERO,
            direction: direction.normalize_or(Vec3::NEG_Z),
            length: f32::from(length),
            one_shot: false,
        }
    }

    /// The 84-byte arc record's fields.
    pub fn from_record(record: &[u8]) -> Result<Self, ArcError> {
        let r = ArcRecord::parse(record)?;
        let spec = Self {
            preset: Preset::Record,
            flags: r.flags,
            tendrils: r.tendrils.max(1),
            segments: r.segments.max(2),
            size: r.size,
            scale: r.scale,
            lifetime: if r.flags & 1 != 0 {
                Lifetime::Millis(r.lifetime)
            } else {
                Lifetime::Permanent
            },
            sound: r.sound,
            jitter: r.jitter,
            burst_radius: r.burst_radius,
            glow: match r.flags & 0x300 {
                0x200 => Some(Glow::WEAPON),
                0x300 => Some(Glow::WEAPON),
                _ => None,
            },
            offset: r.offset,
            direction: r.direction.normalize_or(Vec3::NEG_Z),
            length: r.length,
            one_shot: r.flags & 1 != 0,
        };
        Ok(spec)
    }

    /// The arc's width (cm): twice its size, times the record's scale.
    pub fn width(&self) -> f32 {
        self.size * 2.0 * self.scale
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
    pub lifetime: u32,
    pub size: f32,
    pub scale: f32,
    /// A reserved word the original copies to the emitter.
    pub reserved: u32,
    /// The direction's jitter scale.
    pub jitter: f32,
    /// The burst radius.
    pub burst_radius: f32,
    /// A 15-character sound name.
    pub sound: Option<String>,
    pub offset: Vec3,
    pub direction: Vec3,
    /// The length (cm).
    pub length: f32,
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
            Some(String::from_utf8_lossy(&bytes[..end]).into_owned())
        };
        Ok(Self {
            flags: u(0),
            tendrils: u(4),
            segments: u(8),
            lifetime: u(0x0C),
            size: f(0x10),
            scale: f(0x14),
            reserved: u(0x18),
            jitter: f(0x1C),
            burst_radius: f(0x20),
            sound,
            offset: v(0x38),
            direction: v(0x44),
            length: f(0x50),
        })
    }
}

/// The charge an electrified item carries; a hit adds a step and the last
/// step before 255 releases the effect.
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
    /// A hit's step: 0x80 for a heavy part, else 0x10.
    pub const HEAVY: u8 = 0x80;
    pub const NORMAL: u8 = 0x10;

    /// Adds a hit's step; the step that would cross 255 fills the charge and
    /// spends it instead.
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
    /// The node's state word; bit 0 means it is finished.
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
/// is none or the old one finished; with a dead part, release a live arc.
pub fn electrified_action(has_item: bool, part_live: bool, arc: Option<ArcNode>) -> ArcAction {
    if !has_item {
        return ArcAction::None;
    }
    if part_live {
        match arc {
            Some(node) if !node.finished => ArcAction::None,
            _ => ArcAction::Create,
        }
    } else {
        match arc {
            Some(node) if !node.finished => ArcAction::Release,
            _ => ArcAction::None,
        }
    }
}

/// An electrified weapon's arc effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElectrifiedEffect {
    /// The node's kind; the high byte must be 0x21 to be the arc.
    pub kind: u32,
    /// The node's visibility byte.
    pub visible: u8,
    /// The node's state.
    pub state: i32,
    /// Whether the node still carries its frame callbacks.
    pub callbacks: bool,
}

/// Releases the electrified weapon's arc: the first 0x21-kind node hides,
/// moves to state 1 and loses its callbacks; the callbacks clear whether or
/// not a node matched.
pub fn clear_electrified_effect(nodes: &mut [ElectrifiedEffect]) -> bool {
    let mut matched = false;
    for node in nodes.iter_mut() {
        if node.kind & 0xff00_0000 == 0x2100_0000 {
            node.visible = 0;
            node.state = 1;
            matched = true;
            break;
        }
    }
    for node in nodes.iter_mut() {
        node.callbacks = false;
    }
    matched
}

/// What the arc check does with the node the equipment check found.
pub fn maintain_arc(effect: &mut ElectrifiedEffect, has_owner: bool) -> bool {
    if has_owner {
        effect.callbacks = true;
    }
    effect.kind & 0xff00_0000 == 0x2100_0000 && effect.state & 1 != 0
}

/// One arc: its preset, its jittered tendrils and its age.
#[derive(Debug, Clone)]
pub struct Arc {
    pub spec: ArcSpec,
    pub start: Vec3,
    pub end: Vec3,
    pub age: u32,
    /// Ticks it lives; 0 is permanent.
    pub lifetime: u32,
    tendrils: Vec<Vec<Vec3>>,
    /// A per-arc texture scroll offset.
    pub tex_offset: Vec2,
}

impl Arc {
    /// Builds an arc between two points. The interior points are jittered
    /// once from the spec's `jitter` (labelled a model of the original's
    /// per-tick tendril motion: the tendril simulation itself is outside
    /// this component's evidence).
    pub fn new(spec: ArcSpec, start: Vec3, end: Vec3) -> Self {
        // A record from an untrusted file cannot ask for an unbounded mesh.
        let n = spec.tendrils.clamp(1, 1024) as usize;
        let s = spec.segments.clamp(2, 4096) as usize;
        // A local, deterministic stream stands in for the original's global
        // one; the arc's jitter is a model of the tendril tick either way.
        let mut rng = LocalRng::new(start.x.to_bits() ^ end.z.to_bits() ^ 0x9E37_79B9);
        let lifetime = match spec.lifetime {
            Lifetime::Permanent => 0,
            Lifetime::Ticks(t) => t.max(1),
            Lifetime::RandomTicks { base, spread } => base + rng.int(spread.max(1)),
            Lifetime::Millis(ms) => (ms as f32 * ARC_TICK_HZ / 1000.0)
                .round_ties_even()
                .max(1.0) as u32,
        };
        let direction = (end - start).normalize_or(spec.direction);
        let (_, right, up) = orthonormal(direction);
        let width = spec.width();
        let mut tendrils = Vec::with_capacity(n);
        for t in 0..n {
            let mut points = Vec::with_capacity(s);
            let spread = if t < 20 { 0.0 } else { spec.burst_radius };
            let mut from = start;
            if spread > 0.0 {
                from += right * ((rng.unit() - 0.5) * spread) + up * ((rng.unit() - 0.5) * spread);
            }
            for k in 0..s {
                let f = k as f32 / (s - 1) as f32;
                let base = from.lerp(end, f);
                let bulge = (std::f32::consts::PI * f).sin();
                let amplitude = spec.jitter * width * bulge;
                let offset = if k == 0 || k + 1 == s || amplitude == 0.0 {
                    Vec3::ZERO
                } else {
                    right * ((rng.unit() - 0.5) * 2.0 * amplitude)
                        + up * ((rng.unit() - 0.5) * 2.0 * amplitude)
                };
                points.push(base + offset);
            }
            tendrils.push(points);
        }
        Self {
            spec,
            start,
            end,
            age: 0,
            lifetime,
            tendrils,
            tex_offset: Vec2::ZERO,
        }
    }

    /// The spec's tendrils, for a renderer or a test.
    pub fn tendrils(&self) -> &[Vec<Vec3>] {
        &self.tendrils
    }

    /// One tick: ages the arc and scrolls its texture. False once it is
    /// finished (a permanent arc never is).
    pub fn tick(&mut self) -> bool {
        self.age = self.age.saturating_add(1);
        self.tex_offset.x -= 0.005;
        if self.lifetime == 0 {
            true
        } else {
            self.age < self.lifetime
        }
    }

    /// The arc's ribbon: two triangles per segment, camera-facing, with the
    /// segment fraction as the V coordinate.
    pub fn ribbon(&self, view: Vec3) -> ArcMesh {
        let mut mesh = ArcMesh::default();
        for points in &self.tendrils {
            for k in 0..points.len() - 1 {
                let a = points[k];
                let b = points[k + 1];
                let along = (b - a).normalize_or(self.spec.direction);
                let side = along.cross(view).normalize_or(Vec3::X) * self.spec.width() * 0.5;
                let v0 = k as f32 / (points.len() - 1) as f32;
                let v1 = (k + 1) as f32 / (points.len() - 1) as f32;
                let base = mesh.positions.len() as u32;
                mesh.positions
                    .extend_from_slice(&[a - side, a + side, b - side, b + side]);
                mesh.uvs.extend_from_slice(&[
                    Vec2::new(0.0, v0),
                    Vec2::new(1.0, v0),
                    Vec2::new(0.0, v1),
                    Vec2::new(1.0, v1),
                ]);
                mesh.colours.extend_from_slice(&[
                    [1.0, 1.0, 1.0, 1.0],
                    [1.0, 1.0, 1.0, 1.0],
                    [1.0, 1.0, 1.0, 1.0],
                    [1.0, 1.0, 1.0, 1.0],
                ]);
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

/// The simulation rate arcs time their milliseconds against.
const ARC_TICK_HZ: f32 = 83.0;

/// The arc's local random stream (the original's integer LCG).
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
        let mut nodes = [
            ElectrifiedEffect {
                kind: 0x2200_0001,
                visible: 1,
                state: 0,
                callbacks: true,
            },
            ElectrifiedEffect {
                kind: 0x2100_0001,
                visible: 1,
                state: 0,
                callbacks: true,
            },
        ];
        assert!(clear_electrified_effect(&mut nodes));
        assert_eq!(nodes[0].kind, 0x2200_0001);
        assert_eq!(nodes[0].visible, 1);
        assert!(!nodes[0].callbacks);
        assert_eq!(nodes[1].visible, 0);
        assert_eq!(nodes[1].state, 1);
        assert!(!nodes[1].callbacks);
    }

    #[test]
    fn the_weapon_preset() {
        let s = ArcSpec::weapon();
        assert_eq!(s.tendrils, 20);
        assert_eq!(s.segments, 5);
        assert!((s.size - 1.5).abs() < 1e-6);
        assert!((s.width() - 3.0).abs() < 1e-6);
        assert_eq!(s.sound.as_deref(), Some("electricity03"));
        assert_eq!(s.glow, Some(Glow::WEAPON));
        assert_eq!(s.offset, Vec3::new(0.0, 53.0, 0.0));
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
        assert!((s.size - 3.0).abs() < 1e-6);
        assert_eq!(s.glow, Some(Glow::SPARK));
        assert!(s.one_shot);
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
        let point = s.glow.unwrap().point.unwrap();
        assert!((point.radius - 80.0).abs() < 1e-6);
        assert_eq!(point.softness, 5.0);
        assert_eq!(point.multiplier, 2.0);
        // A length too short for two segments falls back to ten.
        let s = ArcSpec::device(1000, 2, 0, Vec3::NEG_Z);
        assert_eq!(s.segments, 10);
    }

    #[test]
    fn the_record_parses_and_short_data_is_an_error() {
        assert_eq!(ArcRecord::parse(&[0; 10]), Err(ArcError::Short));
        let mut data = [0u8; 84];
        data[0..4].copy_from_slice(&0x201u32.to_le_bytes());
        data[4..8].copy_from_slice(&7u32.to_le_bytes());
        data[8..12].copy_from_slice(&9u32.to_le_bytes());
        data[0x0C..0x10].copy_from_slice(&250u32.to_le_bytes());
        data[0x10..0x14].copy_from_slice(&1.0f32.to_le_bytes());
        data[0x14..0x18].copy_from_slice(&0.5f32.to_le_bytes());
        data[0x1C..0x20].copy_from_slice(&0.25f32.to_le_bytes());
        data[0x20..0x24].copy_from_slice(&12.0f32.to_le_bytes());
        data[0x24..0x2B].copy_from_slice(b"elechit");
        data[0x38..0x3C].copy_from_slice(&1.0f32.to_le_bytes());
        data[0x44..0x48].copy_from_slice(&0.0f32.to_le_bytes());
        data[0x48..0x4C].copy_from_slice(&1.0f32.to_le_bytes());
        data[0x50..0x54].copy_from_slice(&90.0f32.to_le_bytes());
        let r = ArcRecord::parse(&data).expect("the record");
        assert_eq!(r.flags, 0x201);
        assert_eq!(r.tendrils, 7);
        assert_eq!(r.segments, 9);
        assert_eq!(r.lifetime, 250);
        assert_eq!(r.sound.as_deref(), Some("elechit"));
        assert_eq!(r.offset, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(r.direction, Vec3::new(0.0, 1.0, 0.0));
        assert!((r.length - 90.0).abs() < 1e-6);
        let s = ArcSpec::from_record(&data).expect("the spec");
        assert_eq!(s.tendrils, 7);
        assert_eq!(s.segments, 9);
        assert!((s.size - 1.0).abs() < 1e-6);
        assert!((s.scale - 0.5).abs() < 1e-6);
        assert!((s.width() - 1.0).abs() < 1e-6);
        assert_eq!(s.lifetime, Lifetime::Millis(250));
        assert!(s.one_shot);
        assert!(s.glow.is_some()); // flags 0x200 adds the voxel light
    }

    #[test]
    fn the_charge_spends_at_the_limit() {
        let mut charge = Charge { value: 0x00 };
        assert_eq!(charge.hit(false), ChargeOutcome::Charged);
        assert_eq!(charge.value, 0x10);
        charge.value = 0xef;
        // 0xef + 0x10 would cross 255: it fills and spends instead.
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
    }

    #[test]
    fn an_arc_has_the_presets_segments_and_ends() {
        let spec = ArcSpec::weapon();
        let arc = Arc::new(spec, Vec3::ZERO, Vec3::new(0.0, 0.0, -30.0));
        assert_eq!(arc.tendrils.len(), 20);
        for points in arc.tendrils() {
            assert_eq!(points.len(), 5);
            assert!((points[0] - Vec3::ZERO).length() < 1e-5);
            assert!((points[4] - Vec3::new(0.0, 0.0, -30.0)).length() < 1e-5);
        }
        assert!((10..20).contains(&arc.lifetime), "{}", arc.lifetime);
    }

    #[test]
    fn an_arc_ribbon_has_a_quad_per_segment() {
        let spec = ArcSpec::weapon();
        let arc = Arc::new(spec, Vec3::ZERO, Vec3::new(0.0, 0.0, -30.0));
        let mesh = arc.ribbon(Vec3::X);
        let quads = 20 * 4 * 4;
        assert_eq!(mesh.positions.len(), quads);
        assert_eq!(mesh.indices.len(), quads / 4 * 6);
        // The ribbon's half-width is the arc's width, two sizes.
        assert!(((mesh.positions[1] - mesh.positions[0]).length() - 3.0).abs() < 1e-4);
    }

    #[test]
    fn an_arc_expires_after_its_ticks() {
        let mut spec = ArcSpec::weapon();
        spec.lifetime = Lifetime::Ticks(3);
        let mut arc = Arc::new(spec, Vec3::ZERO, Vec3::NEG_Z);
        assert!(arc.tick());
        assert!(arc.tick());
        assert!(!arc.tick());
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
