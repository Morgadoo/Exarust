//! The engine's colour helpers: sRGB↔linear in both directions, RGB→HSV,
//! Rec. 601 luma and desaturation, for float colours and for 8-bit ones.
//!
//! The arithmetic is the original's instruction order: the luma weights
//! accumulate red, then green, then blue; the byte path rounds with
//! `CVTSS2SI` (to even) and stores the low byte; the seams of the HSV
//! conversion pick the greatest and least channel with the original's tie
//! comparisons. `docs/RENDERING.md` (Colour) describes where the engine uses
//! each.

/// The sRGB→linear knee.
pub const SRGB_KNEE: f32 = 0.04045;
/// The linear→sRGB knee.
pub const LINEAR_KNEE: f32 = 0.003_130_8;
/// Rec. 601 red weight.
pub const LUMA_R: f32 = 0.299;
/// Rec. 601 green weight.
pub const LUMA_G: f32 = 0.587;
/// Rec. 601 blue weight.
pub const LUMA_B: f32 = 0.114;
/// The smallest value the HSV conversion treats as colour.
const HSV_FLOOR: f32 = 0.001_960_784_4;

/// One sRGB-encoded channel as linear light: the exact piecewise curve.
pub fn srgb_to_linear(c: f32) -> f32 {
    if c > SRGB_KNEE {
        ((c + 0.055) / 1.055).powf(2.4)
    } else {
        c / 12.92
    }
}

/// An sRGB colour as linear light.
pub fn srgb_to_linear_rgb(c: [f32; 3]) -> [f32; 3] {
    c.map(srgb_to_linear)
}

/// One linear channel as sRGB: the inverse piecewise curve.
pub fn linear_to_srgb(c: f32) -> f32 {
    if c > LINEAR_KNEE {
        c.powf(1.0 / 2.4) * 1.055 - 0.055
    } else {
        c * 12.92
    }
}

/// A linear colour as sRGB.
pub fn linear_to_srgb_rgb(c: [f32; 3]) -> [f32; 3] {
    c.map(linear_to_srgb)
}

/// Rec. 601 luma in the original's accumulation order:
/// `r·0.299 + g·0.587 + b·0.114`.
pub fn luma(c: [f32; 3]) -> f32 {
    c[0] * LUMA_R + c[1] * LUMA_G + c[2] * LUMA_B
}

/// Moves each channel toward the colour's luma by `t` (0 keeps it, 1 makes
/// it grey), as the engine's float desaturation does.
pub fn desaturate(c: [f32; 3], t: f32) -> [f32; 3] {
    let grey = luma(c);
    c.map(|x| (grey - x) * t + x)
}

/// [`desaturate`] on an 8-bit colour: each channel is widened, the same
/// float expression runs, and the result rounds to even and keeps its low
/// byte, which is what the engine's store does (a value outside 0..255
/// wraps rather than clamping).
pub fn desaturate_bytes(c: [u8; 3], t: f32) -> [u8; 3] {
    let f = c.map(f32::from);
    let grey = luma(f);
    f.map(|x| crate::pixel::round_f32((grey - x) * t + x) as u8)
}

/// RGB→HSV as the engine's helper: hue in turns, saturation and value. A
/// value or saturation below [`HSV_FLOOR`] gives a grey.
pub fn rgb_to_hsv(c: [f32; 3]) -> [f32; 3] {
    let [r, g, b] = c;
    // The original's tie rules: a channel only wins by being strictly
    // greater, so a tie keeps the lower index; for the least channel the
    // same test runs against blue.
    let mut imax = usize::from(g > r);
    if c[imax] < b {
        imax = 2;
    }
    let mut imin = usize::from(g < r);
    if b < c[imin] {
        imin = 2;
    }
    let value = c[imax];
    if value < HSV_FLOOR {
        return [0.0, 0.0, value];
    }
    let saturation = 1.0 - c[imin] / value;
    if saturation < HSV_FLOOR {
        return [0.0, saturation, value];
    }
    let inv = 1.0 / (c[imax] - c[imin]);
    let t = c.map(|x| (x - c[imin]) * inv);
    let hue = match imax {
        0 => {
            let h = (t[1] - t[2]) / 6.0;
            if h < 0.0 { h + 1.0 } else { h }
        }
        1 => (f64::from((t[2] - t[0]) / 6.0) + 1.0 / 3.0) as f32,
        _ => (f64::from((t[0] - t[1]) / 6.0) + 2.0 / 3.0) as f32,
    };
    [hue, saturation, value]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_srgb_curves_are_inverse_on_the_knee_sides() {
        // The knee and both of its neighbours.
        assert_eq!(srgb_to_linear(0.04045), 0.04045 / 12.92);
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-6);
        assert!((srgb_to_linear(0.5) - 0.214_041_14).abs() < 1e-6);
        assert_eq!(linear_to_srgb(0.0031308), 0.0031308 * 12.92);
        assert!((linear_to_srgb(0.214_041_14) - 0.5).abs() < 1e-5);
        assert!((linear_to_srgb(1.0) - 1.0).abs() < 1e-6);
        // A colour round-trips.
        let c = [0.1, 0.5, 0.9];
        let back = linear_to_srgb_rgb(srgb_to_linear_rgb(c));
        for i in 0..3 {
            assert!((back[i] - c[i]).abs() < 1e-5);
        }
    }

    #[test]
    fn luma_uses_rec_601_in_the_original_order() {
        assert_eq!(luma([0.0, 0.0, 0.0]), 0.0);
        assert!((luma([1.0, 1.0, 1.0]) - 1.0).abs() < 1e-6);
        // 0.299 + 2×0.114 = 0.527 for pure red and blue.
        assert!((luma([1.0, 0.0, 1.0]) - 0.413).abs() < 1e-6);
        let c = [0.2, 0.6, 0.1];
        let manual = c[2] * LUMA_B + c[1] * LUMA_G + c[0] * LUMA_R;
        // The public order is red first; the sum differs only in the last
        // bit, which the engine's own order fixes.
        assert!((luma(c) - manual).abs() <= f32::EPSILON);
    }

    #[test]
    fn desaturation_moves_to_grey_and_the_byte_path_rounds() {
        let c = [0.9, 0.2, 0.1];
        let grey = luma(c);
        assert_eq!(desaturate(c, 0.0), c);
        for (x, g) in desaturate(c, 1.0).into_iter().zip([grey; 3]) {
            assert!((x - g).abs() < 1e-6);
        }
        // 255, 0, 0 has luma 76.245; halfway is 166/38/38 after rounding.
        assert_eq!(desaturate_bytes([255, 0, 0], 0.5), [166, 38, 38]);
        // Full desaturation rounds 76.245 to 76.
        assert_eq!(desaturate_bytes([255, 0, 0], 1.0), [76, 76, 76]);
        // The store keeps the low byte, so an out-of-range result wraps.
        assert_eq!(desaturate_bytes([255, 255, 255], 4.0), [255, 255, 255]);
    }

    #[test]
    fn rgb_to_hsv_reads_the_sextants_and_the_grey_floor() {
        assert_eq!(rgb_to_hsv([0.0, 0.0, 0.0]), [0.0, 0.0, 0.0]);
        assert_eq!(rgb_to_hsv([0.5, 0.5, 0.5]), [0.0, 0.0, 0.5]);
        let red = rgb_to_hsv([1.0, 0.0, 0.0]);
        assert_eq!(red, [0.0, 1.0, 1.0]);
        let green = rgb_to_hsv([0.0, 1.0, 0.0]);
        assert!((green[0] - 1.0 / 3.0).abs() < 1e-7);
        let blue = rgb_to_hsv([0.0, 0.0, 1.0]);
        assert!((blue[0] - 2.0 / 3.0).abs() < 1e-7);
        // A half-saturated orange: hue 1/12.
        let orange = rgb_to_hsv([1.0, 0.5, 0.0]);
        assert!((orange[0] - 1.0 / 12.0).abs() < 1e-6);
        assert_eq!(orange[1], 1.0);
        assert_eq!(orange[2], 1.0);
        // A tie between red and green keeps red as the greatest.
        assert_eq!(rgb_to_hsv([0.8, 0.8, 0.0])[0], 1.0 / 6.0);
    }
}
