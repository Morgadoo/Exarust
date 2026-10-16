//! Blood stains on weapons, as the original draws them: a second pass over
//! a bloodied weapon's mesh that multiplies the frame by a gradient of
//! blood (`BldGr02D`, `Textures.rpk`: white at its first row, sparse red
//! speckles by a quarter of its height, mostly solid dark red from half
//! way on). The gradient runs down the weapon from its tip, and how far
//! depends on the blood it holds, so the stain creeps from the tip toward
//! the guard as the weapon draws blood. Rules: `docs/RENDERING.md` (Blood
//! on weapons).
//!
//! - **Coordinates**, from the mesh's position in the weapon's frame, in
//!   centimetres: u = 0.025·x − 0.5 (repeating across the blade every
//!   40 cm), v = 0.0125·y + o, with o = −(L/80)·(1 − √a·√(80/L)) for a
//!   blade whose tip stands L cm up its frame and holds blood a
//!   ([`stain_offset`]). So v = (y − L)/80 + √(a·L/80): the gradient's
//!   first row lies √(80·L·a) cm below the tip ([`stained_length`]), and
//!   below that v clamps to the white row (the gradient repeats across and
//!   clamps along its height).
//! - **Colour**: the texel, read as data (not sRGB), faded to white by the
//!   object's fade (no Exarust object fades: 0), written opaque and
//!   multiplied into the frame: white leaves the weapon as it is, the red
//!   darkens it.
//! - **Pass**: depth writes off, the depth nudged toward the camera so the
//!   overlay passes the depth test against its own weapon, back faces
//!   culled, no shadows. Drawn only while the weapon holds more than 0.01
//!   ([`stain_shows`]).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::pbr::{Material, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

pub struct StainPlugin;

impl Plugin for StainPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "stain.wesl");
        app.add_plugins(MaterialPlugin::<StainMaterial>::default());
    }
}

/// The blood gradient (`Textures.rpk`), loaded as data, repeating across
/// and clamped along its height.
pub const STAIN_GRADIENT: &str = "bldgr02d";
/// The blood a weapon must hold before its stain is drawn (compared in
/// double precision).
pub const STAIN_FROM: f64 = 0.01;
/// Centimetres of weapon the gradient's height spans.
pub const STAIN_SPAN: f32 = 80.0;
/// The gradient across the weapon: u = x·[0] + [1] (x in cm).
pub const ACROSS: [f32; 2] = [0.025, -0.5];
/// The gradient along the weapon per centimetre: 1/80.
pub const ALONG: f32 = 0.0125;
/// How far the overlay's depth is nudged toward the camera, in the depth
/// buffer's [0, 1] units: the original's 0.000 01 on its [−1, 1] depth.
pub const STAIN_DEPTH_BIAS: f32 = 0.000_005;

const CM_PER_METRE: f32 = 100.0;

/// Whether a weapon holding blood `amount` shows its stain: above 0.01.
pub fn stain_shows(amount: f32) -> bool {
    f64::from(amount) > STAIN_FROM
}

/// The gradient's offset o for a weapon whose tip stands `tip` metres up
/// its frame (L in cm) and that holds blood `amount`: o = −k1·(1 − √a·k2)
/// with k1 = L/80 and k2 = √(80/L).
pub fn stain_offset(amount: f32, tip: f32) -> f32 {
    let tip_cm = tip * CM_PER_METRE;
    // L·0.0125 in double precision, as the original computes it.
    let k1 = (f64::from(tip_cm) * (1.0 / f64::from(STAIN_SPAN))) as f32;
    let k2 = (STAIN_SPAN / tip_cm).sqrt();
    -k1 * (1.0 - amount.sqrt() * k2)
}

/// The gradient's coordinates at `point` (metres, in the weapon's frame)
/// for the offset `offset` ([`stain_offset`]).
pub fn stain_uv(point: Vec3, offset: f32) -> Vec2 {
    let p = point * CM_PER_METRE;
    Vec2::new(ACROSS[0] * p.x + ACROSS[1], ALONG * p.y + offset)
}

/// How far below the tip the gradient begins (metres), for a weapon whose
/// tip stands `tip` metres up its frame holding blood `amount`:
/// √(80·L·a) cm. Below it the stain is white (none).
pub fn stained_length(amount: f32, tip: f32) -> f32 {
    (STAIN_SPAN * tip * CM_PER_METRE * amount).sqrt() / CM_PER_METRE
}

/// The colour the frame is multiplied by: the gradient's texel `texel`
/// faded to white by `fade`.
pub fn stain_colour(texel: Vec3, fade: f32) -> Vec3 {
    texel.lerp(Vec3::ONE, fade)
}

/// The stain's parameters for the shader.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct StainParams {
    /// The gradient's coordinates from a point in cm: u = x·[0] + [1],
    /// v = y·[2] + [3] (the offset).
    pub coords: Vec4,
    /// The mesh's frame in the weapon's (identity for a model's own mesh;
    /// an assembled weapon's part placement).
    pub weapon_from_mesh: Mat4,
    /// The object's fade toward white (0: no Exarust object fades).
    pub fade: f32,
}

impl StainParams {
    /// The stain of a weapon whose tip stands `tip` metres up its frame and
    /// that holds blood `amount`, on a mesh placed `weapon_from_mesh` in the
    /// weapon's frame.
    pub fn new(amount: f32, tip: f32, weapon_from_mesh: Mat4) -> Self {
        Self {
            coords: Vec4::new(ACROSS[0], ACROSS[1], ALONG, stain_offset(amount, tip)),
            weapon_from_mesh,
            fade: 0.0,
        }
    }
}

/// The stain pass over one mesh of a bloodied weapon.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct StainMaterial {
    #[uniform(0)]
    pub params: StainParams,
    /// The blood gradient ([`STAIN_GRADIENT`]).
    #[texture(1)]
    #[sampler(2)]
    pub gradient: Handle<Image>,
}

fn stain_shader() -> ShaderRef {
    ShaderRef::Path(AssetPath::from_path_buf(embedded_path!("stain.wesl")).with_source("embedded"))
}

impl Material for StainMaterial {
    fn vertex_shader() -> ShaderRef {
        stain_shader()
    }

    fn fragment_shader() -> ShaderRef {
        stain_shader()
    }

    // Frame × colour; Bevy's multiply blend with an opaque fragment.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Multiply
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn enable_oit() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    /// The offset o = −(L/80)(1 − √a·√(80/L)): for the short sword's 60.05 cm
    /// tip, −0.750 625 clean, −0.0627 holding 0.25 (√(L/80) = 0.8664).
    #[test]
    fn the_offset_follows_the_blood() {
        let tip = 0.6005;
        assert!(close(stain_offset(0.0, tip), -0.750_625));
        assert!(close(stain_offset(0.25, tip), -0.750_625 + 0.5 * 0.866_386));
        assert!(close(stain_offset(1.0, tip), -0.750_625 + 0.866_386));
        // A blade of exactly 80 cm: o = √a − 1.
        for a in [0.0, 0.09, 0.64, 1.0] {
            assert!(close(stain_offset(a, 0.8), a.sqrt() - 1.0), "{a}");
        }
    }

    /// v = (y − L)/80 + √(a·L/80): at the tip √(a·L/80), and 0 (the white
    /// first row) √(80·L·a) cm below it, which grows with the square root
    /// of the blood: 34.7 cm of the short sword at 0.25, 69.3 cm at 1.
    #[test]
    fn the_stain_creeps_from_the_tip() {
        let tip = 0.6005;
        for a in [0.04, 0.25, 0.5, 1.0] {
            let o = stain_offset(a, tip);
            let at_tip = stain_uv(Vec3::new(0.0, tip, 0.0), o);
            assert!(close(at_tip.y, (a * 60.05 / 80.0).sqrt()), "{a}");
            let edge = tip - stained_length(a, tip);
            assert!(stain_uv(Vec3::new(0.0, edge, 0.0), o).y.abs() < 1e-5, "{a}");
            // Below the edge the gradient clamps to its white row.
            assert!(stain_uv(Vec3::new(0.0, edge - 0.01, 0.0), o).y < 0.0);
        }
        assert!(close(stained_length(0.25, tip), 0.346_554));
        assert!(close(stained_length(1.0, tip), 0.693_109));
        // Clean, the tip itself is at the white row.
        assert!(close(
            stain_uv(Vec3::new(0.0, tip, 0.0), stain_offset(0.0, tip)).y,
            0.0
        ));
        // Across the blade u repeats every 40 cm: the blade's centre line
        // sits at u = −0.5 (the gradient's middle column).
        let o = stain_offset(0.5, tip);
        assert!(close(stain_uv(Vec3::ZERO, o).x, -0.5));
        assert!(close(stain_uv(Vec3::new(0.02, 0.3, 0.0), o).x, -0.45));
        assert!(close(stain_uv(Vec3::new(0.4, 0.3, 0.0), o).x, 0.5));
    }

    /// Drawn above 0.01 only, compared in double precision.
    #[test]
    fn faint_stains_are_not_drawn() {
        assert!(!stain_shows(0.0));
        assert!(!stain_shows(0.009_999));
        // The single-precision 0.01 is 0.009 999 999 8: not drawn.
        assert!(!stain_shows(0.01));
        assert!(stain_shows(0.010_001));
        assert!(stain_shows(0.025));
    }

    /// The fade lerps the texel toward white; the parameters carry the
    /// original's coefficients.
    #[test]
    fn the_fade_whitens() {
        let red = Vec3::new(0.67, 0.22, 0.2);
        assert_eq!(stain_colour(red, 0.0), red);
        assert_eq!(stain_colour(red, 1.0), Vec3::ONE);
        assert!((stain_colour(red, 0.5) - Vec3::new(0.835, 0.61, 0.6)).length() < 1e-6);
        let p = StainParams::new(0.25, 0.6005, Mat4::IDENTITY);
        assert_eq!(p.coords.x, 0.025);
        assert_eq!(p.coords.y, -0.5);
        assert_eq!(p.coords.z, 0.0125);
        assert_eq!(p.coords.w, stain_offset(0.25, 0.6005));
        assert_eq!(p.fade, 0.0);
    }
}
