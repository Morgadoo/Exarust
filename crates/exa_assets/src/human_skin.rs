//! The human body's skin as the original draws it. The body model's own skin
//! material (`skinhuman`) is never shown: at run time the original puts a
//! built-in skin material in its place, with textures generated for the
//! character from the race's own skin textures, and draws it with its
//! subsurface program. Ancients wear the same skin with a paler scatter
//! colour. Evidence and the open points: `docs/RENDERING.md` (Skin).
//!
//! - **Skin tone**: the creator's 3 × 3 tone grid, read from the record's
//!   two 16-bit coordinates, scales the race's stored colour texels
//!   (0.29–0.64 of them; the middle tone is 0.465). Exarust now carries the
//!   record's tone; a body without a skin look keeps the middle tone.
//! - **Textures**: the race's, named after its body model's stem:
//!   `<stem>difm` (colour), `<stem>nrmm0` (normal) and `<stem>spcm` (gloss),
//!   whatever the body's skin material names (the ancients' names a
//!   `missing.dds`).
//! - **Diffuse**: white, times a generated map: the race's skin texture
//!   multiplied by the character's skin tone, which darkens it to
//!   0.29–0.64. The original loads the race texture as data, so the product
//!   is taken on its stored (sRGB-encoded) values; it renders that map with
//!   the sRGB encoding off into an sRGB texture, so drawing the skin decodes
//!   the tinted colour once: the texture's own skin colour, sRGB
//!   (0.80, 0.65, 0.57), becomes linear (0.112, 0.076, 0.059) at the middle
//!   tone. [`tone_colour`] gives the multiplier for any tone; the shader
//!   takes it as the material's diffuse tint (sRGB values, decoded once).
//! - **Specular**: a generated packed map of full intensity, gloss 0.8 × the
//!   race's gloss texture and no metal, so the roughness is 1 − 0.8·gloss and
//!   F0 is 0.04.
//! - **Subsurface**: of the diffuse colour, a share sRGB (1.0, 0.8, 0.46)
//!   (ancients: (1.0, 0.63, 0.133)) scatters under the skin. That share is
//!   lit by a wrapped term on a blurred normal (mip 2), with the diffuse map
//!   blurred too; the rest takes plain N·L. Lit skin stays as bright as plain
//!   diffuse, with a red wrap into the shadow side.
//! - **Blood**: the human skin shows wound blood through the skin program's
//!   dirt mode 4 (`BloodProgram::Skin`); the ancient skin has none.

use bevy::prelude::*;
use exa_render::material::{BloodProgram, ExaMaterial, ExaParams, flags, linear_rgb};

use crate::model::{ModelLoader, is_body_model};
use crate::texture::ColorSpace;

/// The body model's skin material, which the built-in skin replaces.
pub const RACE_SKIN: &str = "skinhuman";
/// The middle skin tone, the centre of the original's 3 × 3 tone grid: what
/// the generated diffuse map multiplies the race's stored skin texels by.
pub const MIDDLE_SKIN_TONE: [f32; 3] = [0.46, 0.47, 0.47];
/// The middle tone's coordinates on the creator's grid (the record's two
/// 16-bit words).
pub const MIDDLE_TONE: [u16; 2] = [0x8000, 0x8000];
/// The creator's nine skin tones as hue, saturation and value, rows `v` = 0,
/// ½, 1 and columns `u` = 0, ½, 1: the original's start-up table, which it
/// turns into linear colours (HSV to RGB, then the sRGB decode).
const TONE_HSV: [[[f32; 3]; 3]; 3] = [
    [
        [0.082, 0.35, 0.35],
        [0.075, 0.28, 0.38],
        [0.055, 0.15, 0.38],
    ],
    [[0.08, 0.35, 0.28], [0.062, 0.3, 0.33], [0.055, 0.3, 0.38]],
    [
        [0.065, 0.35, 0.21],
        [0.055, 0.38, 0.28],
        [0.062, 0.38, 0.34],
    ],
];
/// The base skin texture's own colour, sRGB (204, 166, 146)/255 as stored;
/// the grid is divided by its linear value.
const TONE_REFERENCE: [f32; 3] = [0.8, 0.650_980_4, 0.572_549_05];
/// The tone multiplier's scale before the sRGB encode, and after it.
const TONE_SCALE: f32 = 0.6;
const TONE_GAIN: f32 = 1.4;

/// The engine's fast natural logarithm: the float's exponent times ln 2
/// plus a polynomial in its mantissa (operation order as the original's).
fn engine_ln(x: f32) -> f32 {
    let bits = x.to_bits();
    let exponent = (bits >> 23) as i32 as f32;
    // A non-positive argument gives −∞; NaN takes the positive offset.
    let offset = if x <= 0.0 {
        f32::NEG_INFINITY
    } else {
        f32::from_bits(0xc2b3_de55)
    };
    let m = f32::from_bits((bits & 0x007f_ffff) | 1.0f32.to_bits());
    let m2 = m * m;
    let ln2 = f32::from_bits(0x3f31_7218);
    let tail = ((m * f32::from_bits(0x3ccc_a7fc) - f32::from_bits(0x3e79_8f62)) * m2
        + (m - f32::from_bits(0x4011_9122)))
        * m2
        + exponent * ln2;
    (m * f32::from_bits(0x4059_7523) + offset) + tail
}

/// The engine's fast exponential: the argument scaled into a float's bit
/// pattern (rounded to nearest, clamped, negatives to zero), its exponent
/// times a polynomial in its mantissa.
fn engine_exp(x: f32) -> f32 {
    let y = x * f32::from_bits(0x4b38_aa3b) + f32::from_bits(0x4e7e_0000);
    let top = f32::from_bits(0x4eff_0000);
    let y = if y < top { y } else { top };
    // CVTPS2DQ: round half to even.
    let i = (y.round_ties_even() as i32).max(0) as u32;
    let scale = f32::from_bits(i & 0x7f80_0000);
    let m = f32::from_bits((i & 0x007f_ffff) | 1.0f32.to_bits());
    let p = (((m * f32::from_bits(0x3c5d_be77) + f32::from_bits(0xbb0b_4858)) * m
        + f32::from_bits(0x3e2a_9cbe))
        * m
        + f32::from_bits(0x3e9f_c08b))
        * m
        + f32::from_bits(0x3f02_8d05);
    scale * p
}

/// The engine's sRGB decode of one channel (double compares and divides,
/// the power through the fast logarithm and exponential).
fn srgb_decode(c: f32) -> f32 {
    let wide = f64::from(c);
    if wide > 0.04045 {
        let base = ((wide + 0.055) / 1.055) as f32;
        engine_exp(engine_ln(base) * 2.4)
    } else {
        (wide / 12.92) as f32
    }
}

/// The engine's linear-to-sRGB encode of one channel.
fn srgb_encode(c: f32) -> f32 {
    if f64::from(c) > 0.003_130_8 {
        let p = engine_exp(engine_ln(c) * 0.416_666_66);
        (f64::from(p) * 1.055 - 0.055) as f32
    } else {
        (f64::from(c) * 12.92) as f32
    }
}

/// Hue, saturation and value to RGB as the engine converts them (the sextant
/// is the floor of 6·hue).
fn hsv_to_rgb([h, s, v]: [f32; 3]) -> [f32; 3] {
    if s == 0.0 {
        return [v; 3];
    }
    let h6 = h * 6.0;
    let sextant = h6.floor() as i32;
    let f = h6 - sextant as f32;
    let p = (1.0 - s) * v;
    let q = (1.0 - s * f) * v;
    let t = (1.0 - (1.0 - f) * s) * v;
    match sextant {
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        5 => [v, p, q],
        _ => [v, t, p],
    }
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// The grid colour at row `row`, column `column`, in linear light.
fn tone_grid(row: usize, column: usize) -> [f32; 3] {
    hsv_to_rgb(TONE_HSV[row][column]).map(srgb_decode)
}

/// A coordinate's half of the grid and the fraction within it: twice the
/// coordinate, less one past the middle.
fn grid_half(coordinate: f32) -> (usize, f32) {
    let s = coordinate * 2.0;
    if s > 1.0 { (1, s - 1.0) } else { (0, s) }
}

/// The skin tone multiplier of the creator's grid at the record's two
/// 16-bit words, in sRGB values: the generated diffuse map multiplies the
/// race's stored texels by it. The grid is interpolated in linear light,
/// divided by the base texture's colour, scaled by 0.6, encoded and scaled
/// by 1.4, in the original's order and precision.
pub fn tone_colour(tone: [u16; 2]) -> [f32; 3] {
    // The column in double precision, the row in single, as the original.
    let u = (f64::from(tone[0]) * 1.525_902_189_669_642_2e-5) as f32;
    let v = f32::from(tone[1]) * 1.525_902_2e-5;
    let (column, s) = grid_half(u);
    let (row, t) = grid_half(v);
    let upper = mix(tone_grid(row, column), tone_grid(row, column + 1), s);
    let lower = mix(
        tone_grid(row + 1, column),
        tone_grid(row + 1, column + 1),
        s,
    );
    let grid = mix(upper, lower, t);
    let reference = TONE_REFERENCE.map(srgb_decode);
    [0, 1, 2].map(|i| srgb_encode(grid[i] / reference[i] * TONE_SCALE) * TONE_GAIN)
}
/// The share of the diffuse colour that scatters under a human's skin (sRGB).
pub const SCATTER: [f32; 3] = [1.0, 0.8, 0.46];
/// The share that scatters under an ancient's skin (sRGB).
pub const ANCIENT_SCATTER: [f32; 3] = [1.0, 0.63, 0.133];
/// The blur of the scattered light: the normal map's mip level it reads.
pub const SCATTER_BLUR: f32 = 2.0;
/// The generated specular map's gloss per unit of the race's gloss texture.
pub const GLOSS: f32 = 0.8;
/// Fresnel at normal incidence of a surface without metal.
pub const F0: f32 = 0.04;

/// The original's two built-in skins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkinKind {
    Human,
    Ancient,
}

impl SkinKind {
    /// The skin a race of body model stem `stem` wears.
    pub fn of_stem(stem: &str) -> Self {
        if stem.eq_ignore_ascii_case("ancient") {
            Self::Ancient
        } else {
            Self::Human
        }
    }

    /// The share of the diffuse colour that scatters under the skin (sRGB).
    pub fn scatter(self) -> [f32; 3] {
        match self {
            Self::Human => SCATTER,
            Self::Ancient => ANCIENT_SCATTER,
        }
    }

    /// Whether wound blood shows on the skin: the human skin has the
    /// original's blood dirt mode, the ancient skin none.
    pub fn shows_blood(self) -> bool {
        self == Self::Human
    }
}

/// Whether a part of `entry` drawn with material `name` wears the skin.
pub fn is_race_skin(entry: &str, name: &str) -> bool {
    is_body_model(entry) && name.eq_ignore_ascii_case(RACE_SKIN)
}

/// The race stem of a body model (`humanbase.rfc` → `human`).
fn race_stem(entry: &str) -> String {
    let lower = entry.to_ascii_lowercase();
    lower.strip_suffix("base.rfc").unwrap_or(&lower).to_string()
}

/// The race's skin textures the generated maps are made from: colour,
/// normal and gloss, the men's or the women's versions (`difm`/`diff`,
/// `nrmm0`/`nrmf0`, `spcm`/`spcf`) with the first physique's normal map.
pub fn race_textures(stem: &str, female: bool) -> [String; 3] {
    let suffixes = if female {
        ["diff", "nrmf0", "spcf"]
    } else {
        ["difm", "nrmm0", "spcm"]
    };
    suffixes.map(|suffix| format!("{stem}{suffix}"))
}

/// The built-in skin's shading parameters, without textures.
pub fn skin_params(kind: SkinKind) -> ExaParams {
    skin_params_of(kind, tone_colour(MIDDLE_TONE))
}

/// The built-in skin's shading parameters for a skin tone multiplier.
pub fn skin_params_of(kind: SkinKind, tone: [f32; 3]) -> ExaParams {
    ExaParams {
        diffuse: Vec3::from_array(tone).extend(1.0),
        specular: Vec3::splat(F0).extend(GLOSS),
        flags: flags::LIT | flags::PACKED_SPECULAR | flags::SUBSURFACE | flags::TINT_AS_SRGB,
        subsurface: linear_rgb(kind.scatter()).extend(SCATTER_BLUR),
        ..default()
    }
}

/// The built-in skin of the race whose body model is `entry`, with the
/// race's textures, at the middle tone.
pub fn skin_material(loader: &mut ModelLoader, entry: &str) -> ExaMaterial {
    skin_material_of(loader, entry, tone_colour(MIDDLE_TONE), false)
}

/// The built-in skin of the race whose body model is `entry` for a skin
/// tone multiplier ([`tone_colour`]), in the men's or women's texture set.
pub fn skin_material_of(
    loader: &mut ModelLoader,
    entry: &str,
    tone: [f32; 3],
    female: bool,
) -> ExaMaterial {
    let stem = race_stem(entry);
    let kind = SkinKind::of_stem(&stem);
    let mut mat = ExaMaterial {
        params: skin_params_of(kind, tone),
        environment: loader.environment_map(),
        ..default()
    };
    // All read as stored, as the original loads them: the tint applies to
    // the encoded colour texels and the shading decodes the product.
    let [colour, normal, gloss] = race_textures(&stem, female);
    mat.diffuse_map = loader.texture(&colour, ColorSpace::Linear).map(|t| t.0);
    mat.normal_map = loader.texture(&normal, ColorSpace::Linear).map(|t| t.0);
    mat.gloss_map = loader.texture(&gloss, ColorSpace::Linear).map(|t| t.0);
    // Wounds show on the human skin through the skin program's blood.
    if kind.shows_blood() && mat.diffuse_map.is_some() {
        let spatter = loader.spatter();
        mat = mat.with_blood(BloodProgram::Skin, spatter);
    }
    mat.with_map_flags()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_body_skin_is_replaced() {
        assert!(is_race_skin("humanbase.rfc", "SkinHuman"));
        assert!(is_race_skin("ancientbase.rfc", "skinhuman"));
        assert!(!is_race_skin("humanbase.rfc", "eyehuman"));
        assert!(!is_race_skin("torcha02", "skinhuman"));
    }

    #[test]
    fn the_tone_grid_gives_the_published_multipliers() {
        // The nine published values, rows v = 0, ½, 1 and columns
        // u = 0, ½, 1.
        let published = [
            [[0.49, 0.50, 0.46], [0.53, 0.56, 0.55], [0.53, 0.59, 0.64]],
            [[0.39, 0.40, 0.37], [0.46, 0.47, 0.47], [0.53, 0.53, 0.53]],
            [[0.29, 0.30, 0.29], [0.39, 0.37, 0.36], [0.48, 0.45, 0.43]],
        ];
        let coordinate = [0u16, 0x8000, 0xFFFF];
        for (v, row) in published.iter().enumerate() {
            for (u, want) in row.iter().enumerate() {
                let got = tone_colour([coordinate[u], coordinate[v]]);
                for c in 0..3 {
                    assert!(
                        (got[c] - want[c]).abs() < 0.006,
                        "u {u} v {v} c {c}: {} vs {}",
                        got[c],
                        want[c]
                    );
                }
            }
        }
        let middle = tone_colour(MIDDLE_TONE);
        for c in 0..3 {
            assert!(
                (middle[c] - MIDDLE_SKIN_TONE[c]).abs() < 0.006,
                "{middle:?}"
            );
        }
    }

    /// The engine's fast logarithm and exponential track the true ones to
    /// a few parts in ten thousand, which is what the skin tone carries.
    #[test]
    fn the_engine_power_is_close_to_the_true_one() {
        for x in [0.01f32, 0.1, 0.35, 0.6, 1.0, 1.7] {
            let ln = engine_ln(x);
            assert!((ln - x.ln()).abs() < 2e-3, "ln {x}: {ln}");
            let e = engine_exp(ln);
            assert!((e / x - 1.0).abs() < 3e-3, "exp {x}: {e}");
        }
        assert_eq!(engine_exp(-200.0), 0.0);
        assert!((srgb_decode(0.8) - 0.6038).abs() < 1e-3);
        assert!((srgb_encode(0.6038) - 0.8).abs() < 1e-3);
        assert_eq!(srgb_decode(0.04), (0.04f64 / 12.92) as f32);
    }

    /// The tone pipeline bit for bit, against an independent single
    /// precision emulation of the original's instructions.
    #[test]
    fn tones_match_the_original_bit_for_bit() {
        let bits = |t: [u16; 2]| tone_colour(t).map(f32::to_bits);
        assert_eq!(bits(MIDDLE_TONE), [0x3eeb_cf59, 0x3ef0_1524, 0x3eef_21e0]);
        assert_eq!(bits([0, 0]), [0x3efa_1b4f, 0x3f00_ac80, 0x3eeb_b95d]);
        assert_eq!(
            bits([0xFFFF, 0xFFFF]),
            [0x3ef2_f514, 0x3ee8_513c, 0x3edb_77d9]
        );
        assert_eq!(bits([0, 0xFFFF]), [0x3e96_0719, 0x3e97_31bd, 0x3e93_17f8]);
    }

    /// The middle grid entry, (0.062, 0.3, 0.33) in HSV: the first sextant,
    /// f = 0.372, so (V, (1 − (1 − f)·S)·V, (1 − S)·V).
    #[test]
    fn tones_come_from_the_hsv_table() {
        let rgb = hsv_to_rgb(TONE_HSV[1][1]);
        let want = [0.33, 0.267_828, 0.231];
        for c in 0..3 {
            assert!((rgb[c] - want[c]).abs() < 1e-6, "{rgb:?}");
        }
        assert_eq!(hsv_to_rgb([0.4, 0.0, 0.2]), [0.2; 3]);
        // The middle words fall just past the middle: the upper halves,
        // a hair from their start.
        assert_eq!(grid_half(0x8000 as f32 / 65535.0).0, 1);
        assert_eq!(grid_half(0.5), (0, 1.0));
    }

    #[test]
    fn the_race_names_its_skin_textures() {
        assert_eq!(race_stem("HumanBase.rfc"), "human");
        assert_eq!(
            race_textures(&race_stem("ancientbase.rfc"), false),
            ["ancientdifm", "ancientnrmm0", "ancientspcm"]
        );
        assert_eq!(
            race_textures(&race_stem("humanbase.rfc"), true),
            ["humandiff", "humannrmf0", "humanspcf"]
        );
        assert_eq!(SkinKind::of_stem("human"), SkinKind::Human);
        assert_eq!(SkinKind::of_stem("Ancient"), SkinKind::Ancient);
        // Only the human skin has the blood dirt mode.
        assert!(SkinKind::Human.shows_blood());
        assert!(!SkinKind::Ancient.shows_blood());
    }

    #[test]
    fn skin_is_darker_and_rougher_than_its_record() {
        let p = skin_params(SkinKind::Human);
        // The record's own diffuse, sRGB 0.784, is 0.578 in linear light.
        assert!(p.diffuse.x < 0.578 && p.diffuse.x > 0.29);
        // Roughness 1 − 0.8·gloss: never glossier than 0.2.
        assert!((p.specular.w - 0.8).abs() < 1e-6);
        assert!((p.specular.x - 0.04).abs() < 1e-6);
        assert_ne!(p.flags & flags::SUBSURFACE, 0);
        assert_ne!(p.flags & flags::TINT_AS_SRGB, 0);
    }

    #[test]
    fn the_tinted_texels_are_decoded_once() {
        // The race texture's own skin colour, as stored (sRGB), tinted by the
        // middle tone and read back as sRGB: a lighter, less saturated tan
        // than tinting the decoded colour and decoding it again.
        let stored = [0.8, 0.651, 0.573];
        let tan: Vec<f32> = stored
            .iter()
            .zip(MIDDLE_SKIN_TONE)
            .map(|(t, k)| exa_render::material::linear(t * k))
            .collect();
        assert!((tan[0] - 0.112).abs() < 0.002, "{tan:?}");
        assert!((tan[1] - 0.076).abs() < 0.002, "{tan:?}");
        assert!((tan[2] - 0.059).abs() < 0.002, "{tan:?}");
        let twice = exa_render::material::linear(exa_render::material::linear(0.8) * 0.46);
        assert!(twice < tan[0] * 0.6, "{twice}");
    }

    #[test]
    fn lit_skin_is_as_bright_as_plain_diffuse() {
        // Facing a light: plain N·L on the unscattered share, the wrapped
        // term (1.1 at N·L = 1, blur 2) on the scattered share.
        let sss = skin_params(SkinKind::Human).subsurface;
        let c = 0.5 + 0.05 * sss.w;
        let wrap = |b: f32| ((b + c).max(0.0).powi(2) * 0.5).min(0.5) + (b - 1.0 + c).max(0.0);
        let response = |b: f32, k: f32| b.max(0.0) * (1.0 - k) + wrap(b) * k;
        let lit = [sss.x, sss.y, sss.z].map(|k| response(1.0, k));
        for r in lit {
            assert!((1.0..=1.11).contains(&r), "{r}");
        }
        // Past the terminator only the red share still glows.
        let edge = [sss.x, sss.y, sss.z].map(|k| response(0.0, k));
        assert!((edge[0] - 0.18).abs() < 0.01 && edge[2] < 0.05, "{edge:?}");
    }
}
