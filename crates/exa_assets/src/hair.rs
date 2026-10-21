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

/// The picker's hue anchors in stored sRGB bytes: dark red, blonde, reddish,
/// dark red and grey (the last beyond the three the interpolation reaches).
const HAIR_HUES: [[u8; 3]; 5] = [
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

/// The hair a record's three colour bytes make at `feral`, with the
/// material's specular level doubled unless `light_variant` (the record's
/// bit the generator's pass).
///
/// The colour bytes are the record's hue, saturation and value; the feral
/// level lifts the saturation toward 0.6 and the value toward 0.4 of where
/// they were, and its share thins the hair.
pub fn hair_shading(colour: [u8; 3], feral: f32, light_variant: bool) -> HairShading {
    let h = (f64::from(colour[0]) * 0.003_921_568_627_450_98) as f32;
    let s = (1.0 - feral * 0.4)
        * (1.0 - (1.0 - feral * 0.6) * (1.0 - f32::from(colour[1]) * 0.003_921_569));
    let v = 1.0 - (1.0 - feral * 0.6) * (1.0 - f32::from(colour[2]) * 0.003_921_569);
    let (diffuse, specular) = pick(h, s, v);
    let specular = if light_variant {
        specular
    } else {
        specular.map(|c| c * 1.3)
    };
    HairShading {
        diffuse,
        specular,
        gloss: 0.9 - feral * 0.4,
        opacity: {
            let f = 1.0 - feral * 0.65;
            f * f
        },
    }
}

/// The creation preview's hair colour: the record's three
/// colour bytes taken as hue, saturation and value without the feral
/// modulation, the specular level boosted by 1.3, gloss 0.9 and full
/// opacity. The preview's material is the same `_HumanHair` payload.
pub fn preview_shading(colour: [u8; 3]) -> HairShading {
    let h = f32::from(colour[0]) / 255.0;
    let s = f32::from(colour[1]) / 255.0;
    let v = f32::from(colour[2]) / 255.0;
    let (diffuse, specular) = pick(h, s, v);
    HairShading {
        diffuse,
        specular: specular.map(|c| c * 1.3),
        gloss: 0.9,
        opacity: 1.0,
    }
}

/// Whether a hair part takes the hair material: its
/// material opacity is over 0.5.
pub fn hair_material_part(opacity: f32) -> bool {
    opacity > 0.5
}

/// The picker: hue, saturation and value in 0…1 to diffuse and specular
/// colours in encoded sRGB.
pub fn pick(h: f32, s: f32, v: f32) -> ([f32; 3], [f32; 3]) {
    let hue = h * 3.0;
    let i = (hue.trunc() as usize).min(HAIR_HUES.len() - 2);
    let frac = hue - i as f32;
    let a = HAIR_HUES[i].map(|b| srgb_decode(f32::from(b) / 255.0));
    let b = HAIR_HUES[i + 1].map(|b| srgb_decode(f32::from(b) / 255.0));
    let anchor = mix(a, b, frac);
    let s1 = s + 0.1;
    let anchor = desaturate(anchor, ((s1 * s1) as f64 * 0.1) as f32);
    let grey = 0.75 * (s1 * 0.3);
    let mut spec = anchor.map(|c| c * (0.75 * ((s1 + 0.1) * 0.6)));
    spec = mix(spec, [grey; 3], v * 0.8 + 0.2);
    let diffuse = spec.map(|c| srgb_encode(c * (s1 * 0.4 + 0.3) * 2.0));
    let specular = srgb_encode_set(spec);
    // The shader's highlight: the specular's hue at a fixed value, then
    // moved by the record's value and brightness.
    let mut hsv = rgb_to_hsv(specular);
    hsv[2] = s1 * 0.9 + 0.3;
    hsv[2] -= v * 0.1;
    hsv[1] *= 1.5;
    hsv[2] *= 0.8;
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

/// RGB to hue, saturation and value as the engine's colour helper does.
pub fn rgb_to_hsv(c: [f32; 3]) -> [f32; 3] {
    let [r, g, b] = c;
    let mut imax = if g > r { 1 } else { 0 };
    if c[imax] < b {
        imax = 2;
    }
    let mut imin = if g < r { 1 } else { 0 };
    if c[imin] > b {
        imin = 2;
    }
    let v = c[imax];
    let floor = 0.001_960_784_4;
    if v < floor {
        return [0.0, 0.0, v];
    }
    let s = 1.0 - c[imin] / v;
    if s < floor {
        return [0.0, s, v];
    }
    let h = match imax {
        0 => {
            let h = (c[1] - c[2]) / (v - c[imin]) / 6.0;
            if h < 0.0 { h + 1.0 } else { h }
        }
        // The engine works the green sextant in double precision.
        1 => {
            ((f64::from(c[2]) - f64::from(c[0])) / f64::from(v - c[imin]) / 6.0 + 1.0 / 3.0) as f32
        }
        _ => (c[0] - c[1]) / (v - c[imin]) / 6.0 + 2.0 / 3.0,
    };
    [h, s, v]
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

    #[test]
    fn the_preview_keeps_the_specular_light_and_shows_opaque_parts() {
        let shown = preview_shading([40, 80, 40]);
        assert_eq!(shown.gloss, 0.9);
        assert_eq!(shown.opacity, 1.0);
        let (_, spec) = pick(40.0 / 255.0, 80.0 / 255.0, 40.0 / 255.0);
        for (shown, base) in shown.specular.iter().zip(spec) {
            assert!((shown - base * 1.3).abs() < 1e-6);
        }
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
    fn rgb_to_hsv_reads_the_sextants() {
        assert_eq!(rgb_to_hsv([1.0, 0.0, 0.0]), [0.0, 1.0, 1.0]);
        let green = rgb_to_hsv([0.0, 1.0, 0.0]);
        assert!((green[0] - 1.0 / 3.0).abs() < 1e-6);
        let blue = rgb_to_hsv([0.0, 0.0, 1.0]);
        assert!((blue[0] - 2.0 / 3.0).abs() < 1e-6);
        assert_eq!(rgb_to_hsv([0.5, 0.5, 0.5]), [0.0, 0.0, 0.5]);
        assert_eq!(rgb_to_hsv([0.0, 0.0, 0.0]), [0.0, 0.0, 0.0]);
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
