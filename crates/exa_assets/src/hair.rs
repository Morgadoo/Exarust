//! Hair colour: the original's `THairColPicker`, its darkening by the feral
//! level and the colour and gloss the hair material takes.
//!
//! The picker interpolates five decoded hue anchors by the record's hue,
//! desaturates by the saturation, builds a specular grey and moves it toward
//! the diffuse by the value, encodes both and derives a second hue from the
//! specular for the shader's anisotropic highlight. All of it runs through
//! the engine's own fast log/exp and sRGB chain, as the skin tone does
//! ([`exa_assets::human_skin`]). Evidence: the picker and the hair material
//! setup, instruction by instruction.

use crate::human_skin::{hsv_to_rgb, srgb_decode, srgb_encode};
use exa_formats::palette::rgb_to_hsv;

/// The picker's hue anchors in stored sRGB bytes: dark red, blonde, reddish,
/// dark red and grey (the last beyond the three the interpolation reaches).
pub const HAIR_HUES: [[u8; 3]; 5] = [
    [164, 98, 48],
    [170, 130, 82],
    [152, 75, 54],
    [164, 98, 48],
    [128, 128, 128],
];

/// A hair colour as the material takes it: the diffuse and specular colours
/// in encoded sRGB, the gloss and the opacity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HairShading {
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    /// Glossiness of the hair material.
    pub gloss: f32,
    /// How solid the strands are.
    pub opacity: f32,
}

/// A record colour byte as the material pass reads it: `byte × 1/255` in
/// double, narrowed.
fn unit(b: u8) -> f32 {
    (f64::from(b) * (1.0 / 255.0)) as f32
}

/// The hair a record's three colour bytes make at `feral`, with the
/// material's specular level boosted by 1.3 unless `light_variant` (the
/// record's profession bit `0x10000`).
///
/// The colour bytes are the record's hue, saturation and value; the feral
/// level lifts the saturation and value toward 1 (by `0.6·f` of the way)
/// and then scales the saturation by `1 − 0.4·f`, all in double as the
/// original. The gloss is `0.9 − 0.4·f` and the opacity `(1 − 0.65·f)²`.
pub fn hair_shading(colour: [u8; 3], feral: f32, light_variant: bool) -> HairShading {
    let f = f64::from(feral);
    let lift = |b: u8| (1.0 - (1.0 - f * 0.6) * f64::from(1.0 - unit(b))) as f32;
    let h = unit(colour[0]);
    let s = ((1.0 - f * 0.4) * f64::from(lift(colour[1]))) as f32;
    let v = lift(colour[2]);
    let (diffuse, specular) = pick(h, s, v);
    let specular = if light_variant {
        specular
    } else {
        specular.map(|c| c * 1.3)
    };
    HairShading {
        diffuse,
        specular,
        gloss: (0.9 - f * 0.4) as f32,
        opacity: {
            let o = 1.0 - f * 0.65;
            (o * o) as f32
        },
    }
}

/// The creation preview's hair colour: the record's three colour bytes
/// taken as hue, saturation and value without the feral level, and the
/// specular boosted by 1.3. The preview writes only these two colours (and
/// always uses the `HarHPC1` texture set); the material's gloss and opacity
/// keep whatever the last in-game hair build left in the shared material.
pub fn preview_shading(colour: [u8; 3]) -> ([f32; 3], [f32; 3]) {
    let (diffuse, specular) = pick(unit(colour[0]), unit(colour[1]), unit(colour[2]));
    (diffuse, specular.map(|c| c * 1.3))
}

/// Whether a hair part takes the hair material: its
/// material opacity is over 0.5.
pub fn hair_material_part(opacity: f32) -> bool {
    opacity > 0.5
}

/// The engine's byte-to-colour decode: each byte times the single-precision
/// reciprocal of 255.
fn decode_bytes(c: [u8; 3]) -> [f32; 3] {
    c.map(|b| f32::from(b) * f32::from_bits(0x3b80_8081))
}

/// The picker: hue, saturation and value in 0…1 to diffuse and specular
/// colours in encoded sRGB. The mixed-precision steps (a single value
/// widened, worked in double and narrowed) follow the instructions.
pub fn pick(h: f32, s: f32, v: f32) -> ([f32; 3], [f32; 3]) {
    let hue = h * 3.0;
    let i = (hue.trunc() as usize).min(HAIR_HUES.len() - 2);
    let frac = hue - i as f32;
    let a = decode_bytes(HAIR_HUES[i]).map(srgb_decode);
    let b = decode_bytes(HAIR_HUES[i + 1]).map(srgb_decode);
    let anchor = mix(a, b, frac);
    let s1 = (f64::from(s) + 0.1) as f32;
    let anchor = desaturate(anchor, (f64::from(s1 * s1) * 0.1) as f32);
    let grey = (f64::from(0.75f32) * (f64::from(s1) * 0.3)) as f32;
    let scale = (f64::from(0.75f32) * ((f64::from(s1) + 0.1) * 0.6)) as f32;
    let mut spec = anchor.map(|c| c * scale);
    spec = mix(spec, [grey; 3], (f64::from(v) * 0.8 + 0.2) as f32);
    let lift = (f64::from(s1) * 0.4 + 0.3) as f32;
    let diffuse = spec.map(|c| srgb_encode(c * lift * 2.0));
    let specular = srgb_encode_set(spec);
    // The shader's highlight: the specular's hue at a fixed value, then
    // moved by the record's value and brightness.
    let mut hsv = rgb_to_hsv(specular);
    hsv[2] = (f64::from(s1) * 0.9 + 0.3) as f32;
    hsv[2] = (f64::from(hsv[2]) - f64::from(v) * 0.1) as f32;
    hsv[1] *= 1.5;
    hsv[2] = (f64::from(hsv[2]) * 0.8) as f32;
    (diffuse, srgb_encode_set(hsv_to_rgb(hsv)))
}

fn srgb_encode_set(c: [f32; 3]) -> [f32; 3] {
    c.map(srgb_encode)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// Moves a colour toward its luma by `t` (0 keeps it, 1 makes it grey).
pub fn desaturate(c: [f32; 3], t: f32) -> [f32; 3] {
    let luma = c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114;
    c.map(|x| x + (luma - x) * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The picker at grey, half saturation and a blonde: concrete encoded
    /// values pinned from the engine's own chain.
    #[test]
    fn the_picker_gives_the_encoded_colours() {
        let (d, s) = pick(0.5, 0.5, 0.5);
        assert_eq!(d.map(f32::to_bits), [0x3ece_a740, 0x3eb9_4b64, 0x3eaf_a89d]);
        assert_eq!(s.map(f32::to_bits), [0x3f50_ffee, 0x3f41_cd15, 0x3f3a_69b5]);
        let (d, s) = pick(0.0, 0.0, 0.0);
        assert_eq!(d.map(f32::to_bits), [0x3e20_a95d, 0x3dbf_7a98, 0x3d66_1a37]);
        assert_eq!(s.map(f32::to_bits), [0x3f18_282c, 0x3ecc_570f, 0x3e35_6bc5]);
    }

    /// A colour (record bytes 0, 13, 110 at feral 0) whose green channel
    /// single-precision steps would round differently: the original's bits,
    /// from an instruction-level emulation of the material pass and picker.
    #[test]
    fn the_picker_keeps_the_mixed_precision_steps() {
        let shading = hair_shading([0, 13, 110], 0.0, false);
        assert_eq!(
            shading.diffuse.map(f32::to_bits),
            [0x3e37_d4fe, 0x3e11_aa7d, 0x3e00_8f99]
        );
        assert_eq!(
            shading.specular.map(f32::to_bits),
            [0x3f46_6f8e, 0x3f28_8958, 0x3f18_abb4]
        );
    }

    #[test]
    fn the_preview_boosts_the_specular_and_shows_opaque_parts() {
        let (diffuse, specular) = preview_shading([40, 80, 40]);
        let (d, spec) = pick(unit(40), unit(80), unit(40));
        assert_eq!(diffuse, d);
        assert_eq!(specular, spec.map(|c| c * 1.3));
        assert!(hair_material_part(0.51));
        assert!(!hair_material_part(0.5));
    }

    /// The hue anchors interpolate: 0 is the dark red, 1 is the blonde.
    #[test]
    fn the_hue_anchors_are_the_five_stored_colours() {
        let first = HAIR_HUES[0].map(|b| srgb_decode(f32::from(b) / 255.0));
        let (d, _) = pick(0.0, 0.0, 0.0);
        // With no saturation and value the diffuse is the anchor brightened
        // by the value-free terms; the hue itself stays the anchor's.
        let hue_of = |c: [f32; 3]| rgb_to_hsv(c)[0];
        assert!((hue_of(d) - hue_of(first)).abs() < 0.02, "{d:?} {first:?}");
        assert_eq!(HAIR_HUES[4], [128, 128, 128]);
    }

    #[test]
    fn desaturation_moves_to_the_luma() {
        let c = [1.0, 0.0, 0.0];
        let half = desaturate(c, 0.5);
        assert_eq!(half, [0.6495, 0.1495, 0.1495]);
        assert_eq!(desaturate(c, 0.0), c);
        let grey = desaturate(c, 1.0);
        assert_eq!(grey, [0.299_000_02, 0.299, 0.299]);
    }

    /// Feral hair is lighter and more saturated and thins the strands;
    /// the light variant leaves the specular unboosted.
    #[test]
    fn the_material_follows_the_feral_level() {
        let calm = hair_shading([40, 80, 40], 0.0, false);
        let feral = hair_shading([40, 80, 40], 1.0, false);
        assert!(feral.specular[0] > calm.specular[0]);
        assert!((calm.gloss - 0.9).abs() < 1e-6);
        assert!((feral.gloss - 0.5).abs() < 1e-6);
        // Worked in double: 0.9 − 0.25·0.4 narrows to 0x3f4ccccd.
        assert_eq!(
            hair_shading([0; 3], 0.25, false).gloss.to_bits(),
            0x3f4c_cccd
        );
        assert!((calm.opacity - 1.0).abs() < 1e-6);
        assert!((feral.opacity - 0.1225).abs() < 1e-6);
        let light = hair_shading([40, 80, 40], 0.0, true);
        for i in 0..3 {
            assert!((light.specular[i] - calm.specular[i] / 1.3).abs() < 1e-5);
        }
        // At feral 0 the stored values are the colour itself.
        let plain = hair_shading([0, 128, 128], 0.0, false);
        assert!(plain.diffuse[1] > 0.0);
    }
}
