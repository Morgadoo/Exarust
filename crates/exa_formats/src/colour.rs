//! The engine's luma and desaturation helpers, for float colours and for
//! 8-bit ones.
//!
//! The arithmetic is the original's instruction order: the luma weights
//! accumulate red, then green, then blue; the byte path converts with
//! `CVTSI2SS`, rounds with `CVTSS2SI` (to even) and stores the low byte.
//! The sRGB curves (with the engine's own logarithm and exponential) are
//! `exa_assets::human_skin`'s, and the HSV conversion is
//! [`crate::palette::rgb_to_hsv`].

/// Rec. 601 red weight.
pub const LUMA_R: f32 = 0.299;
/// Rec. 601 green weight.
pub const LUMA_G: f32 = 0.587;
/// Rec. 601 blue weight.
pub const LUMA_B: f32 = 0.114;

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

#[cfg(test)]
mod tests {
    use super::*;

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
        // The store keeps the low byte, so an out-of-range result wraps:
        // red is -102.51, which rounds to -103 and stores as 153; green and
        // blue are 152.49, which round to 152.
        assert_eq!(desaturate_bytes([255, 0, 0], 2.0), [153, 152, 152]);
    }
}
