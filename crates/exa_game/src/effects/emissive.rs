//! The GI emissive objects the force effects carry: the light each spreads
//! through the voxel volume, its box, and how its strength follows the
//! effect's phase.
//!
//! Every emissive is a GI light object with a shader program of its own
//! (`gi_dome`, `gi_bubble`, `gi_cuboid_fn`, `gi_cuboid`, `gi_gradline`,
//! `gi_multipoint`, `gi_telebubble`), a colour, a texture position and a
//! world box. The box is the mesh's axis-aligned bounds, rounded to whole
//! centimetres and widened by 10 cm, then by 60 cm of falloff (80 for the
//! cuboid, 120 for the tendril's multi-point light; the teleporter bubble
//! 60).
//!
//! - **Barrier** ([`glow`], [`Kind::Barrier`]): growing, `ease(share·1.5)`;
//!   standing, 1; shrinking, `ease(1 − share)`. Its colour is
//!   (0.14, 0.08, 1) times that, the node's fade and 0.8.
//! - **Bubble**: growing, `ease(share·1.2)`; standing, 1; shrinking,
//!   `ease(1 − share)`; colour (0.1, 0.06, 1) times `(2 − f)·f`, the fade
//!   and 1, with alpha `1 − f`.
//! - **Barricade**: growing, `ease(share·1.5)`; standing, 1; shrinking,
//!   `ease(1 − share·1.5)²`; colour (0.14, 0.08, 1) times that, the fade
//!   and 1.2.
//! - **Teleporter bubble**: the texture's own colour, a pulse
//!   `(2 − charge)·charge·s` with `s = 1` under a charge of 0.8 and
//!   `s = 2·(1 − (charge − 0.8)·5)` above, a texture period of
//!   `500 − round(350·charge)` ms and a fade `ease(1 − alpha)` under state 5,
//!   or the alpha itself from 5.
//! - **Strikes**: a gradient line between two points with attenuation
//!   10000 and a point light, its colour `1 − fade` times the node's.
//! - **Tendril**: a multi-point light along the tendril's points to 16000 cm
//!   of attenuation, each point's strength scaled by the node's fade and
//!   its own intensity.
//!
//! Evidence: the seven emissive classes' program setup, world bounds and
//! draw, read from `Exanima.exe`.

use bevy::prelude::*;

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

    /// The uniform names the program binds, in order.
    pub fn uniforms(self) -> &'static [&'static str] {
        match self {
            Program::Dome => &["Pos", "Radius", "Colour", "TexPos"],
            Program::Bubble => &["Pos", "Radius", "Colour", "TexPos", "ViewPos"],
            Program::CuboidFn => &["Pos", "Radius", "Colour", "TexPos"],
            Program::Cuboid => &["Pos", "Radius", "Colour"],
            Program::GradientLine => &["Line", "Grad", "Colour", "Atten", "PointLgt"],
            Program::MultiPoint => &["Light", "Atten"],
            Program::TeleporterBubble => &[
                "bMat", "Radius", "Colour", "TexPos", "Fade", "Pulse", "Anim",
            ],
        }
    }

    /// The texture the light samples, if any: the two faced lights use the
    /// force effect texture's second layer.
    pub fn texture(self) -> Option<&'static str> {
        match self {
            Program::Dome | Program::Bubble | Program::CuboidFn | Program::Cuboid => {
                Some("ForceEfctA2E.rfi")
            }
            Program::TeleporterBubble => Some("ForceEfctA2E.rfi"),
            _ => None,
        }
    }
}

/// The colour of the force dome and the barricade: (0.14, 0.08, 1).
pub const DOME_COLOUR: [f32; 3] = [0.14, 0.08, 1.0];
/// The bubble's colour: (0.1, 0.06, 1).
pub const BUBBLE_COLOUR: [f32; 3] = [0.1, 0.06, 1.0];
/// The intensity scales: barrier 0.8, barricade 1.2, bubble 1, strikes 1.
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
/// The teleporter bubble's texture period, 500 to 150 ms.
pub const TELE_PERIOD_MAX: f32 = 500.0;
pub const TELE_PERIOD_SWING: f32 = 350.0;

/// What the emissive shows this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glow {
    pub colour: [f32; 3],
    /// The colour's strength (alpha for the bubble's own draw).
    pub strength: f32,
    /// The bubble's alpha, `1 − f`.
    pub alpha: f32,
}

/// The phase rule the light follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Barrier,
    Bubble,
    Barricade,
    Cuboid,
    Strikes,
    Tendril,
    TeleporterBubble,
}

/// The clamp the engine's ease helper applies: 0 at or below 0, 1 at or
/// above 1.
#[allow(clippy::manual_clamp)] // -0.0 stays +0.0, as the original's compare.
fn clamp01(x: f32) -> f32 {
    if x <= 0.0 {
        0.0
    } else if x >= 1.0 {
        1.0
    } else {
        x
    }
}

/// The light's strength by phase: state 1 grows, 2 ends, 3 shrinks, 4 gone.
/// `share` is the effect's shown share; `fade` the node's own fade.
pub fn glow(kind: Kind, state: i32, share: f32, fade: f32, at_ms: u32, since_ms: u32) -> Glow {
    match kind {
        Kind::Barrier => {
            let f = if state == 1 {
                clamp01(share * 1.5)
            } else if state < 3 {
                1.0
            } else {
                clamp01(1.0 - share)
            };
            let amount = f * fade * BARRIER_GAIN;
            Glow {
                colour: [DOME_COLOUR[0] * amount, DOME_COLOUR[1] * amount, amount],
                strength: f,
                alpha: 1.0,
            }
        }
        Kind::Bubble => {
            let f = if state == 1 {
                clamp01(share * 1.2)
            } else if state < 3 {
                1.0
            } else {
                clamp01(1.0 - share)
            };
            // The shader takes `f/2 + 0.5` through the half parabola.
            let eased = f * 0.5 + 0.5;
            let amount = (2.0 - eased) * eased * fade;
            Glow {
                colour: [BUBBLE_COLOUR[0] * amount, BUBBLE_COLOUR[1] * amount, amount],
                strength: f,
                alpha: 1.0 - f,
            }
        }
        Kind::Barricade => {
            let f = if state == 1 {
                clamp01(share * 1.5)
            } else if state < 3 {
                1.0
            } else {
                let e = clamp01(1.0 - share * 1.5);
                e * e
            };
            let amount = f * fade * BARRICADE_GAIN;
            Glow {
                colour: [DOME_COLOUR[0] * amount, DOME_COLOUR[1] * amount, amount],
                strength: f,
                alpha: 1.0,
            }
        }
        Kind::Cuboid => Glow {
            colour: [fade, fade, fade],
            strength: fade,
            alpha: 1.0,
        },
        Kind::Strikes => Glow {
            colour: [fade, fade, fade],
            strength: 1.0 - fade,
            alpha: 1.0,
        },
        Kind::Tendril => Glow {
            colour: [fade, fade, fade],
            strength: fade,
            alpha: 1.0,
        },
        Kind::TeleporterBubble => {
            let alpha = share;
            let f = if state < 5 {
                clamp01(1.0 - alpha)
            } else {
                alpha
            };
            // The texture's colour is the only colour the light has; the
            // original uploads its texture row colour (guess: white).
            let _ = (at_ms, since_ms);
            Glow {
                colour: [1.0, 1.0, 1.0],
                strength: tele_pulse(alpha),
                alpha: f,
            }
        }
    }
}

/// The teleporter bubble's pulse: `(2 − charge)·charge·s`, `s` 1 under a
/// charge of 0.8 and `2·(1 − (charge − 0.8)·5)` above.
pub fn tele_pulse(charge: f32) -> f32 {
    let swell = if 0.8 < charge {
        2.0 * (1.0 - (charge - 0.8) * 5.0)
    } else {
        1.0
    };
    (2.0 - charge) * charge * swell
}

/// The teleporter bubble's texture phase: `(now − since) mod period / period`,
/// the period falling from 500 to 150 ms as the charge passes 0.8.
pub fn texture_phase(charge: f32, now_ms: u32, since_ms: u32) -> f32 {
    let period = TELE_PERIOD_MAX - (charge * TELE_PERIOD_SWING).round_ties_even();
    if period == 0.0 {
        return 0.0;
    }
    (now_ms.wrapping_sub(since_ms) as f32 % period) / period
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

/// The tendril light's box: the bounds of its points, their middle, and the
/// half extents plus 120 cm.
pub fn point_box(points: &[Vec3]) -> Option<(Vec3, Vec3)> {
    let first = *points.first()?;
    let mut min = first;
    let mut max = first;
    for point in points {
        min = min.min(*point);
        max = max.max(*point);
    }
    let centre = min + (max - min) * 0.5;
    let half = max - centre + Vec3::splat(TENDRIL_FALLOFF_CM);
    Some((centre, half))
}

/// A tendril light point's strength: the node's fade times the point's own
/// intensity times the node's scale.
pub fn tendril_strength(fade: f32, intensity: f32, scale: f32) -> f32 {
    fade * intensity * scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_programs_and_their_uniforms_are_the_originals() {
        assert_eq!(Program::Dome.name(), "gi_dome");
        assert_eq!(Program::Bubble.name(), "gi_bubble");
        assert_eq!(Program::CuboidFn.name(), "gi_cuboid_fn");
        assert_eq!(
            Program::GradientLine.uniforms(),
            &["Line", "Grad", "Colour", "Atten", "PointLgt"]
        );
        assert_eq!(Program::MultiPoint.uniforms(), &["Light", "Atten"]);
        assert_eq!(Program::Dome.texture(), Some("ForceEfctA2E.rfi"));
        assert_eq!(Program::MultiPoint.texture(), None);
    }

    #[test]
    fn the_barrier_glow_follows_grow_stand_and_fade() {
        // Growing at half: ease(0.75) = 0.75.
        let g = glow(Kind::Barrier, 1, 0.5, 1.0, 0, 0);
        assert!((g.strength - 0.75).abs() < 1e-6);
        assert!((g.colour[0] - 0.14 * 0.75 * 0.8).abs() < 1e-6);
        // Standing: full.
        let g = glow(Kind::Barrier, 0, 1.0, 1.0, 0, 0);
        assert!((g.colour[1] - 0.08 * 0.8).abs() < 1e-6);
        // Shrinking at 0.25: ease(0.75).
        let g = glow(Kind::Barrier, 3, 0.25, 1.0, 0, 0);
        assert!((g.strength - 0.75).abs() < 1e-6);
    }

    #[test]
    fn the_bubble_pulses_and_reports_its_alpha() {
        let g = glow(Kind::Bubble, 0, 1.0, 1.0, 0, 0);
        assert!((g.colour[0] - 0.1).abs() < 1e-6);
        assert_eq!(g.alpha, 0.0);
        let g = glow(Kind::Bubble, 1, 0.5, 1.0, 0, 0);
        // f = ease(0.6) = 0.6; the half parabola at 0.8 is 0.96.
        assert!((g.colour[2] - 0.96).abs() < 1e-6);
        assert!((g.alpha - 0.4).abs() < 1e-6);
    }

    #[test]
    fn the_barricade_shrinks_with_the_square() {
        let g = glow(Kind::Barricade, 1, 0.5, 0.5, 0, 0);
        assert!((g.strength - 0.75).abs() < 1e-6);
        assert!((g.colour[2] - 0.75 * 0.5 * 1.2).abs() < 1e-6);
        // 1 − 0.75·1.5 = −0.125 clamps to 0.
        let g = glow(Kind::Barricade, 3, 0.75, 1.0, 0, 0);
        assert_eq!(g.colour[2], 0.0);
    }

    #[test]
    fn the_teleporter_bubble_changes_its_texture_period() {
        assert_eq!(TELE_PERIOD_MAX, 500.0);
        assert!((texture_phase(0.0, 250, 0) - 0.5).abs() < 1e-6);
        // At a charge of 1 the period is 150 ms.
        let period = TELE_PERIOD_MAX - TELE_PERIOD_SWING;
        assert!((texture_phase(1.0, period as u32, 0) - 0.0).abs() < 1e-5);
        // The swell: at 0.9, s = 2·(1 − 0.5) = 1, pulse = 1.1·0.9.
        let g = glow(Kind::TeleporterBubble, 0, 0.9, 1.0, 0, 0);
        assert!((g.strength - 1.1 * 0.9).abs() < 1e-5);
        assert!((tele_pulse(0.8) - 1.2 * 0.8).abs() < 1e-5);
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
        // A turned frame widens the axis-aligned box.
        let turned = [Vec3::new(0.6, 0.8, 0.0), Vec3::new(-0.8, 0.6, 0.0), Vec3::Z];
        let (_, half) = world_box(Vec3::ZERO, turned, Vec3::splat(100.0), FALLOFF_CM);
        assert_eq!(half.x, 140.0 + 10.0 + 60.0);
        assert_eq!(half.y, 140.0 + 10.0 + 60.0);
        assert_eq!(half.z, 100.0 + 10.0 + 60.0);
    }

    #[test]
    fn a_point_box_is_the_tendril_bounds_plus_120() {
        let (centre, half) =
            point_box(&[Vec3::new(0.0, 0.0, 0.0), Vec3::new(100.0, 20.0, 0.0)]).unwrap();
        assert_eq!(centre, Vec3::new(50.0, 10.0, 0.0));
        assert_eq!(half, Vec3::new(50.0 + 120.0, 10.0 + 120.0, 120.0));
        assert!((tendril_strength(0.5, 2.0, 3.0) - 3.0).abs() < 1e-6);
    }
}
