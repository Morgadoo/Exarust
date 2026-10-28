//! The GI emissive objects the force effects carry: the light each spreads
//! through the voxel volume, its box, and how its values follow the
//! effect's phase.
//!
//! Every emissive is a GI light object with a program of its own (`gi_dome`,
//! `gi_bubble`, `gi_cuboid_fn`, `gi_cuboid`, `gi_gradline`, `gi_multipoint`,
//! `gi_telebubble`), drawn into the volume with the volume's box matrix
//! (`bMat`); the faced ones sample the force texture as `Tex0` on unit 2.
//! `gain` below is the GI volume's light scale, which every colour takes.
//!
//! - **Barrier** ([`barrier_light`]): the extent `bExt` is the node's scale
//!   less 60 across and 40 up; the strength is `min(1.5·share, 1)` growing,
//!   1 under state 3, `max(1 − share, 0)` from 3; the colour
//!   `(0.14, 0.08, 1)` times strength, gain and 0.8.
//! - **Bubble** ([`bubble_light`]): the radius is the dome's plus 5; the
//!   strength `min(1.2·share, 1)` growing (the product in double
//!   precision), 1, or `max(1 − share, 0)`; the fade `1 − strength`; the
//!   colour `(0.1, 0.06, 1)` times `(2 − e)·e·gain`, `e = strength/2 + 1/2`.
//! - **Barricade** ([`barricade_light`]): the Barrier's extent; strength
//!   `min(1.5·share, 1)`, 1, or `max(1 − 1.5·share, 0)²`; colour
//!   `(0.14, 0.08, 1)` times strength, gain and 1.2.
//! - **Cuboid** ([`cuboid_light`]): the extent is `max(scale − 20, 10)` on
//!   each axis, the colour the emissive's own (`gi_cuboid` takes no
//!   texture position).
//! - **Strikes** ([`strikes_light`]): a gradient line between two points
//!   with attenuation 10000, the colour the emissive's times `1 − fade`,
//!   and a point light (its fourth value ten times its strength) or none.
//! - **Tendril** ([`tendril_light`]): one light per tendril point, its
//!   colour `(0.25, 0.25, 1)` times gain, the point's intensity and the
//!   emissive's scale; attenuation 16000; the program is built per point
//!   count.
//! - **Teleporter bubble** ([`teleporter_light`]): the colour `(0.75, 0.8,
//!   1)` times gain and the swell `s` (1 up to a charge of 0.8, then
//!   `1 − 5·(charge − 0.8)`, doubled for the colour); the pulse
//!   `(2 − charge)·charge·s`; the fade `max(1 − fade, 0)` under state 5;
//!   the animation phase `(ms mod p)/p` with `p = 500 − round(350·charge)`.
//!
//! - **Blast** ([`blast_light`]) and **torrent** ([`torrent_light`]):
//!   multi-point lights, one point a chunk or a gathered stream sample,
//!   coloured (0.16, 0.08, 1) and (0.12, 0.06, 1); their strengths follow
//!   the blast's charge or its chunks' life and the torrent's level.
//!
//! The boxes are the mesh node's axis-aligned bounds widened 10 cm and
//! rounded, plus 60 cm of falloff (80 for the cuboid); the tendril's, the
//! blast's and the torrent's are their points' bounds plus 120 cm.
//!
//! Nothing injects these lights into Exarust's voxel volume yet.
//!
//! Evidence: the seven emissive classes' program setup, bounds and draw,
//! read from `Exanima.exe`.

// NaN-sensitive compares are kept as the original makes them.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::prelude::*;

use super::force_dome::Phase;

/// The shader programs the force emissives use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Program {
    Dome,
    Bubble,
    CuboidFn,
    Cuboid,
    GradientLine,
    MultiPoint,
    TeleporterBubble,
}

impl Program {
    /// The program's name, as the renderer looks it up.
    pub fn name(self) -> &'static str {
        match self {
            Program::Dome => "gi_dome",
            Program::Bubble => "gi_bubble",
            Program::CuboidFn => "gi_cuboid_fn",
            Program::Cuboid => "gi_cuboid",
            Program::GradientLine => "gi_gradline",
            Program::MultiPoint => "gi_multipoint",
            Program::TeleporterBubble => "gi_telebubble",
        }
    }

    /// The uniforms the program binds, in order (the texture sampler first
    /// where it has one).
    pub fn uniforms(self) -> &'static [&'static str] {
        match self {
            Program::Dome | Program::CuboidFn => &["Tex0", "bMat", "bExt", "Colour", "TexPos"],
            Program::Bubble => &["Tex0", "bMat", "Radius", "Colour", "TexPos", "Fade"],
            Program::Cuboid => &["Tex0", "bMat", "bExt", "Colour"],
            Program::GradientLine => &["Line", "Grad", "Colour", "Atten", "PointLgt"],
            Program::MultiPoint => &["Light", "Atten"],
            Program::TeleporterBubble => &[
                "Tex0", "bMat", "Radius", "Colour", "TexPos", "Fade", "Pulse", "Anim",
            ],
        }
    }
}

/// The force dome's and the barricade's colour.
pub const DOME_COLOUR: Vec3 = Vec3::new(0.14, 0.08, 1.0);
/// The bubble's colour.
pub const BUBBLE_COLOUR: Vec3 = Vec3::new(0.1, 0.06, 1.0);
/// The tendril's colour.
pub const TENDRIL_COLOUR: Vec3 = Vec3::new(0.25, 0.25, 1.0);
/// The teleporter bubble's colour (the force texture's name is followed by
/// it in the original's data).
pub const TELEPORTER_COLOUR: Vec3 = Vec3::new(0.75, 0.8, 1.0);
/// The intensity scales: barrier 0.8, barricade 1.2.
pub const BARRIER_GAIN: f32 = 0.8;
pub const BARRICADE_GAIN: f32 = 1.2;
/// The bounds' margin over the mesh: 10 cm, then the falloff.
pub const BOUNDS_MARGIN_CM: f32 = 10.0;
pub const FALLOFF_CM: f32 = 60.0;
pub const CUBOID_FALLOFF_CM: f32 = 80.0;
pub const TENDRIL_FALLOFF_CM: f32 = 120.0;
/// The gradient line's attenuation, and the multi-point light's.
pub const LINE_ATTENUATION: f32 = 10_000.0;
pub const MULTIPOINT_ATTENUATION: f32 = 16_000.0;
/// The teleporter bubble's animation period, 500 ms less 350 per charge.
pub const TELE_PERIOD_MS: i32 = 500;
pub const TELE_PERIOD_SWING: f32 = 350.0;

/// `min(x, 1)`.
fn at_most_one(x: f32) -> f32 {
    if x >= 1.0 { 1.0 } else { x }
}

/// `max(x, 0)`, a NaN kept.
fn at_least_zero(x: f32) -> f32 {
    if x <= 0.0 { 0.0 } else { x }
}

/// The faced extent: the node's scale less 60 across and 40 up.
fn faced_extent(scale: Vec3) -> Vec3 {
    Vec3::new(scale.x - 60.0, scale.y - 40.0, scale.z)
}

/// A faced light's extent and colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacedLight {
    pub extent: Vec3,
    pub colour: Vec3,
}

/// The Barrier's light from its phase and shown share and its node's scale.
pub fn barrier_light(phase: Phase, share: f32, scale: Vec3, gain: f32) -> FacedLight {
    let strength = match phase {
        Phase::Growing => at_most_one(share * 1.5),
        p if p < Phase::Shrinking => 1.0,
        _ => at_least_zero(1.0 - share * 1.0),
    };
    FacedLight {
        extent: faced_extent(scale),
        colour: DOME_COLOUR * (strength * gain * BARRIER_GAIN),
    }
}

/// The barricade's light from its state word and shown share.
pub fn barricade_light(state: i32, share: f32, scale: Vec3, gain: f32) -> FacedLight {
    let strength = if state == 1 {
        at_most_one(share * 1.5)
    } else if (state as u32) < 3 {
        1.0
    } else {
        let s = at_least_zero(1.0 - share * 1.5);
        s * s
    };
    FacedLight {
        extent: faced_extent(scale),
        colour: DOME_COLOUR * (strength * gain * BARRICADE_GAIN),
    }
}

/// The Bubble's light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BubbleLight {
    pub radius: f32,
    pub fade: f32,
    pub colour: Vec3,
}

/// The Bubble's light from its phase, shown share and radius.
pub fn bubble_light(phase: Phase, share: f32, radius: f32, gain: f32) -> BubbleLight {
    let strength = match phase {
        Phase::Growing => at_most_one((f64::from(share) * 1.2) as f32),
        p if p < Phase::Shrinking => 1.0,
        _ => at_least_zero(1.0 - share * 1.0),
    };
    let e = strength * 0.5 + 0.5;
    BubbleLight {
        radius: radius + 5.0,
        fade: 1.0 - strength,
        colour: BUBBLE_COLOUR * ((2.0 - e) * e * gain * 1.0),
    }
}

/// The cuboid's extent and colour: `max(scale − 20, 10)` on each axis.
pub fn cuboid_light(scale: Vec3, colour: Vec3) -> FacedLight {
    let extent = |s: f32| {
        let e = s - 20.0;
        if e <= 10.0 { 10.0 } else { e }
    };
    FacedLight {
        extent: Vec3::new(extent(scale.x), extent(scale.y), extent(scale.z)),
        colour,
    }
}

/// The strikes' light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrikesLight {
    pub line: [Vec3; 2],
    pub gradient: Vec3,
    pub colour: Vec3,
    pub attenuation: f32,
    pub point: [f32; 4],
}

/// The strikes' light: the line's two ends and its gradient (already in the
/// volume's space), the emissive's colour, the effect's fade and its point
/// light (place and strength) when it has one.
pub fn strikes_light(
    line: [Vec3; 2],
    gradient: Vec3,
    colour: Vec3,
    fade: f32,
    point: Option<(Vec3, f32)>,
) -> StrikesLight {
    StrikesLight {
        line,
        gradient,
        colour: colour * (1.0 - fade),
        attenuation: LINE_ATTENUATION,
        point: point.map_or([0.0; 4], |(at, strength)| {
            [at.x, at.y, at.z, strength * 10.0]
        }),
    }
}

/// The tendril's lights: each point's place and colour, the colour
/// `(0.25, 0.25, 1)` times the gain, the point's intensity and the
/// emissive's scale.
pub fn tendril_light(points: &[(Vec3, f32)], scale: f32, gain: f32) -> Vec<(Vec3, Vec3)> {
    points
        .iter()
        .map(|&(at, intensity)| (at, TENDRIL_COLOUR * (gain * intensity * scale)))
        .collect()
}

/// The teleporter bubble's light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeleporterLight {
    pub fade: f32,
    pub pulse: f32,
    pub anim: f32,
    pub colour: Vec3,
}

/// The teleporter bubble's light from its state word, fade value, charge
/// and phase stamp, at the game clock `now_ms`.
pub fn teleporter_light(
    state: u32,
    fade: f32,
    charge: f32,
    stamp_ms: u32,
    now_ms: u32,
    gain: f32,
) -> TeleporterLight {
    let fade = if state < 5 {
        at_least_zero(1.0 - fade)
    } else {
        fade
    };
    let swell = if charge <= 0.8 {
        1.0
    } else {
        1.0 - (charge - 0.8) * 5.0
    };
    let pulse = (2.0 - charge) * charge * swell;
    let period = (TELE_PERIOD_MS - (charge * TELE_PERIOD_SWING).round_ties_even() as i32) as i64;
    let span = i64::from(now_ms) - i64::from(stamp_ms);
    let anim = if period == 0 {
        0.0
    } else {
        ((span % period) as f64 / period as f64) as f32
    };
    let colour_swell = if charge > 0.8 { swell * 2.0 } else { swell };
    TeleporterLight {
        fade,
        pulse,
        anim,
        colour: TELEPORTER_COLOUR * (gain * colour_swell),
    }
}

/// The axis-aligned world box of a mesh node: the node's place plus each
/// axis times its scale, widened 10 cm and rounded; then the light's centre
/// and half extents with the falloff added.
pub fn world_box(centre: Vec3, axes: [Vec3; 3], scale: Vec3, falloff: f32) -> (Vec3, Vec3) {
    let dot = axes[0].abs() * scale.x + axes[1].abs() * scale.y + axes[2].abs() * scale.z;
    let round = |v: Vec3| {
        Vec3::new(
            v.x.round_ties_even(),
            v.y.round_ties_even(),
            v.z.round_ties_even(),
        )
    };
    let min = round(centre - dot - Vec3::splat(BOUNDS_MARGIN_CM));
    let max = round(centre + dot + Vec3::splat(BOUNDS_MARGIN_CM));
    let middle = min + (max - min) * 0.5;
    let half = max - middle + Vec3::splat(falloff);
    (middle, half)
}

/// The tendril's, blast's and torrent's light box: the bounds of its
/// points, their middle, and the half extents plus 120 cm (the blast then
/// moves the middle by its node's place).
pub fn point_box(points: &[Vec3]) -> Option<(Vec3, Vec3)> {
    let first = *points.first()?;
    let mut min = first;
    let mut max = first;
    for point in points {
        for axis in 0..3 {
            let v = point[axis];
            if !(min[axis] <= v) {
                min[axis] = v;
            } else if max[axis] < v {
                max[axis] = v;
            }
        }
    }
    let centre = min + (max - min) * 0.5;
    let half = max - centre + Vec3::splat(TENDRIL_FALLOFF_CM);
    Some((centre, half))
}

/// The box a node-placed emissive light uses: the points' box middle
/// moved by the node's place (the original adds the node's translation
/// after the half extents are taken).
pub fn node_box(centre: Vec3, half: Vec3, node_at: Vec3) -> (Vec3, Vec3) {
    (centre + node_at, half)
}

/// The blast's and the torrent's light colours.
pub const BLAST_COLOUR: Vec3 = Vec3::new(0.16, 0.08, 1.0);
pub const TORRENT_COLOUR: Vec3 = Vec3::new(0.12, 0.06, 1.0);
/// The blast light's attenuation while charging and once its chunks have
/// spent 0.3 of their life; the torrent light's.
pub const BLAST_CHARGE_ATTENUATION: f32 = 5000.0;
pub const BLAST_SPENT_ATTENUATION: f32 = 1000.0;
pub const TORRENT_ATTENUATION: f32 = 2000.0;

/// A multi-point light: each point's place and colour, and the shared
/// attenuation.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiPointLight {
    pub points: Vec<(Vec3, Vec3)>,
    pub attenuation: f32,
}

/// The blast's light: one point a chunk (its life record's light point),
/// coloured [`BLAST_COLOUR`] times the gain, the strength and the setup's
/// light. While charging the strength is 1 from a charge of 0.3 and
/// `(0.6 − c)·c·11.11` below it (in double precision), the attenuation
/// 5000. Thrown, with `f = 1 − ticks/life` (in double precision), the
/// strength is 1 up to 0.666 and `max(4 − 4.5·f, 0)²` above, the
/// attenuation 1000 from 0.3 and `5000 − 13333·f` below; the attenuation
/// is the last chunk's.
pub fn blast_light(
    points: &[(Vec3, i32)],
    thrown: bool,
    charge: f32,
    life_ticks: i32,
    light: f32,
    gain: f32,
) -> MultiPointLight {
    let mut attenuation = BLAST_CHARGE_ATTENUATION;
    let points = points
        .iter()
        .map(|&(at, ticks)| {
            let strength = if !thrown {
                attenuation = BLAST_CHARGE_ATTENUATION;
                let c = f64::from(charge);
                if c < 0.3 {
                    ((0.6 - c) * c * 11.11) as f32
                } else {
                    1.0
                }
            } else {
                let f = (1.0 - f64::from(ticks) / f64::from(life_ticks)) as f32;
                attenuation = if f64::from(f) < 0.3 {
                    5000.0 - f * 13_333.0
                } else {
                    BLAST_SPENT_ATTENUATION
                };
                if f64::from(f) > 0.666 {
                    // MAXSS returns its second operand, 0, for a NaN.
                    let s = 4.0 - f * 4.5;
                    let s = if s > 0.0 { s } else { 0.0 };
                    s * s
                } else {
                    1.0
                }
            };
            (at, BLAST_COLOUR * ((gain * strength) * light))
        })
        .collect();
    MultiPointLight {
        points,
        attenuation,
    }
}

/// The torrent's light: one point a light point of its frame, coloured
/// [`TORRENT_COLOUR`] times the gain, the point's weight over 4 and
/// `(2 − l)·l·1.2` of the torrent's level `l` (the last in double
/// precision); attenuation 2000.
pub fn torrent_light(points: &[(Vec3, f32)], level: f32, gain: f32) -> MultiPointLight {
    let curve = (f64::from((2.0 - level) * level) * 1.2) as f32;
    MultiPointLight {
        points: points
            .iter()
            .map(|&(at, weight)| (at, TORRENT_COLOUR * ((gain * (weight * 0.25)) * curve)))
            .collect(),
        attenuation: TORRENT_ATTENUATION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_programs_and_their_uniforms_are_the_originals() {
        assert_eq!(Program::Dome.name(), "gi_dome");
        assert_eq!(Program::Bubble.uniforms()[2], "Radius");
        assert_eq!(
            Program::Cuboid.uniforms(),
            &["Tex0", "bMat", "bExt", "Colour"]
        );
        assert_eq!(
            Program::GradientLine.uniforms(),
            &["Line", "Grad", "Colour", "Atten", "PointLgt"]
        );
        assert_eq!(Program::TeleporterBubble.uniforms().len(), 8);
    }

    #[test]
    fn the_barrier_light_follows_grow_stand_and_fade() {
        let scale = Vec3::new(85.0, 85.0, 25.0);
        let g = barrier_light(Phase::Growing, 0.5, scale, 1.0);
        assert_eq!(g.extent, Vec3::new(25.0, 45.0, 25.0));
        assert_eq!(g.colour, DOME_COLOUR * (0.75 * 1.0 * 0.8));
        let g = barrier_light(Phase::Ending, 0.0, scale, 2.0);
        assert_eq!(g.colour, DOME_COLOUR * (1.0 * 2.0 * 0.8));
        let g = barrier_light(Phase::Shrinking, 0.25, scale, 1.0);
        assert_eq!(g.colour, DOME_COLOUR * (0.75 * 1.0 * 0.8));
    }

    #[test]
    fn the_bubble_light_takes_its_radius_plus_five_and_its_fade() {
        let g = bubble_light(Phase::Standing, 1.0, 100.0, 1.0);
        assert_eq!(g.radius, 105.0);
        assert_eq!(g.fade, 0.0);
        assert_eq!(g.colour, BUBBLE_COLOUR);
        let g = bubble_light(Phase::Growing, 0.5, 100.0, 1.0);
        let f = (0.5f64 * 1.2) as f32;
        let e = f * 0.5 + 0.5;
        assert_eq!(g.fade, 1.0 - f);
        assert_eq!(g.colour, BUBBLE_COLOUR * ((2.0 - e) * e));
    }

    #[test]
    fn the_barricade_light_falls_with_the_square() {
        let g = barricade_light(1, 0.5, Vec3::splat(100.0), 0.5);
        assert_eq!(g.colour, DOME_COLOUR * (0.75 * 0.5 * 1.2));
        // 1 − 0.75·1.5 is below zero: dark.
        let g = barricade_light(3, 0.75, Vec3::splat(100.0), 1.0);
        assert_eq!(g.colour, Vec3::ZERO);
    }

    #[test]
    fn the_cuboid_extent_keeps_ten_centimetres() {
        let g = cuboid_light(Vec3::new(25.0, 50.0, 31.0), Vec3::ONE);
        assert_eq!(g.extent, Vec3::new(10.0, 30.0, 11.0));
    }

    #[test]
    fn the_strikes_and_tendril_lights_scale_their_colours() {
        let s = strikes_light([Vec3::ZERO, Vec3::X], Vec3::Y, Vec3::ONE, 0.25, None);
        assert_eq!(s.colour, Vec3::splat(0.75));
        assert_eq!(s.point, [0.0; 4]);
        let s = strikes_light(
            [Vec3::ZERO; 2],
            Vec3::Y,
            Vec3::ONE,
            0.0,
            Some((Vec3::Z, 0.5)),
        );
        assert_eq!(s.point, [0.0, 0.0, 1.0, 5.0]);
        let t = tendril_light(&[(Vec3::ONE, 2.0)], 3.0, 0.5);
        assert_eq!(t[0].1, TENDRIL_COLOUR * 3.0);
    }

    #[test]
    fn the_teleporter_light_swells_and_quickens() {
        let t = teleporter_light(2, 0.25, 0.5, 0, 250, 1.0);
        assert_eq!(t.fade, 0.75);
        assert_eq!(t.pulse, 1.5 * 0.5);
        // 500 − 175 = 325 ms: 250/325.
        assert_eq!(t.anim, (250.0f64 / 325.0) as f32);
        assert_eq!(t.colour, TELEPORTER_COLOUR);
        let t = teleporter_light(5, -0.5, 0.9, 0, 0, 1.0);
        assert_eq!(t.fade, -0.5);
        let s = 1.0 - (0.9f32 - 0.8) * 5.0;
        assert_eq!(t.pulse, (2.0 - 0.9) * 0.9 * s);
        assert_eq!(t.colour, TELEPORTER_COLOUR * (s * 2.0));
    }

    #[test]
    fn a_world_box_is_the_mesh_box_plus_the_margins() {
        let (centre, half) = world_box(
            Vec3::new(100.0, 200.0, 300.0),
            [Vec3::X, Vec3::Y, Vec3::Z],
            Vec3::splat(50.0),
            FALLOFF_CM,
        );
        assert_eq!(centre, Vec3::new(100.0, 200.0, 300.0));
        assert_eq!(half, Vec3::splat(120.0));
        let turned = [Vec3::new(0.6, 0.8, 0.0), Vec3::new(-0.8, 0.6, 0.0), Vec3::Z];
        let (_, half) = world_box(Vec3::ZERO, turned, Vec3::splat(100.0), FALLOFF_CM);
        assert_eq!(half.x, 140.0 + 10.0 + 60.0);
        assert_eq!(half.z, 100.0 + 10.0 + 60.0);
    }

    #[test]
    fn an_emissive_box_moves_with_its_node() {
        let (centre, half) = node_box(Vec3::X, Vec3::splat(5.0), Vec3::new(0.0, 10.0, 0.0));
        assert_eq!(centre, Vec3::new(1.0, 10.0, 0.0));
        assert_eq!(half, Vec3::splat(5.0));
    }

    #[test]
    fn a_point_box_is_the_tendril_bounds_plus_120() {
        let (centre, half) =
            point_box(&[Vec3::new(0.0, 0.0, 0.0), Vec3::new(100.0, 20.0, 0.0)]).unwrap();
        assert_eq!(centre, Vec3::new(50.0, 10.0, 0.0));
        assert_eq!(half, Vec3::new(50.0 + 120.0, 10.0 + 120.0, 120.0));
    }
    #[test]
    fn the_blast_light_follows_its_charge_and_life() {
        let charging = blast_light(&[(Vec3::ZERO, 58)], false, 0.1, 58, 0.6, 1.0);
        assert_eq!(charging.attenuation, 5000.0);
        let s = ((0.6f64 - f64::from(0.1f32)) * f64::from(0.1f32) * 11.11) as f32;
        assert_eq!(charging.points[0].1, BLAST_COLOUR * (s * 0.6));
        // A third of its life left: f = 2/3 is above 0.666.
        let late = blast_light(&[(Vec3::ZERO, 20)], true, 0.0, 60, 0.6, 1.0);
        assert_eq!(late.attenuation, 1000.0);
        let f = (1.0 - 20.0f64 / 60.0) as f32;
        let s = 4.0 - f * 4.5;
        assert_eq!(late.points[0].1, BLAST_COLOUR * ((s * s) * 0.6));
        let fresh = blast_light(&[(Vec3::ZERO, 60)], true, 0.0, 60, 0.6, 1.0);
        assert_eq!(fresh.attenuation, 5000.0);
        assert_eq!(fresh.points[0].1, BLAST_COLOUR * 0.6);
    }

    #[test]
    fn the_torrent_light_scales_by_its_level() {
        let light = torrent_light(&[(Vec3::ZERO, 2.0)], 1.0, 1.0);
        assert_eq!(light.attenuation, 2000.0);
        assert_eq!(light.points[0].1, TORRENT_COLOUR * (0.5 * 1.2));
    }
}
