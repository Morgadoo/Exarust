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
/// The nine grid colours in linear light, rows `v` = 0, ½, 1 and columns
/// `u` = 0, ½, 1. Recovered from the original's tone multipliers with
/// `grid = (reference / 0.6) · sRGB⁻¹(tone / 1.4)`; the multipliers are
/// known to two decimals, so the grid is good to about 0.001.
const TONE_GRID: [[[f32; 3]; 3]; 3] = [
    [
        [0.101_122, 0.066_601, 0.042_322],
        [0.119_097, 0.084_449, 0.061_387],
        [0.119_097, 0.094_311, 0.084_699],
    ],
    [
        [0.063_476, 0.042_180, 0.027_250],
        [0.088_754, 0.058_591, 0.044_239],
        [0.119_097, 0.075_217, 0.056_792],
    ],
    [
        [0.035_600, 0.023_982, 0.016_976],
        [0.063_476, 0.036_091, 0.025_810],
        [0.096_895, 0.053_580, 0.036_868],
    ],
];
/// The base skin texture's own colour in linear light: sRGB
/// (0.8, 0.651, 0.573). The tone grid is divided by it.
const TONE_REFERENCE: [f32; 3] = [0.603_827_34, 0.381_351_44, 0.287_936_84];
/// The tone multiplier's scale before the sRGB encode, and after it.
const TONE_DIVISOR: f32 = 0.6;
const TONE_GAIN: f32 = 1.4;

/// The linear-to-sRGB encode of one channel.
fn srgb_encode(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// One row of the grid at `u`: bilinear within the half it falls in, the
/// middle column exactly at 0.5.
fn row_at(row: &[[f32; 3]; 3], u: f32) -> [f32; 3] {
    if u <= 0.5 {
        mix(row[0], row[1], u * 2.0)
    } else {
        mix(row[1], row[2], (u - 0.5) * 2.0)
    }
}

/// The skin tone multiplier of the creator's grid at (`u`, `v`), in sRGB
/// values: the generated diffuse map multiplies the race's stored texels by
/// it. The grid is interpolated in linear light and the tone encoded once.
pub fn tone_colour(tone: [u16; 2]) -> [f32; 3] {
    let u = f32::from(tone[0]) / 65535.0;
    let v = f32::from(tone[1]) / 65535.0;
    let rows = [0, 1, 2].map(|i| row_at(&TONE_GRID[i], u));
    let grid = if v <= 0.5 {
        mix(rows[0], rows[1], v * 2.0)
    } else {
        mix(rows[1], rows[2], (v - 0.5) * 2.0)
    };
    [0, 1, 2].map(|i| {
        let linear = TONE_DIVISOR * grid[i] / TONE_REFERENCE[i];
        (TONE_GAIN * srgb_encode(linear.max(0.0))).min(1.0)
    })
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
    skin_params_of(kind, MIDDLE_SKIN_TONE)
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
    skin_material_of(loader, entry, MIDDLE_SKIN_TONE, false)
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
