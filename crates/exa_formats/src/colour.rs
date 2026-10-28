//! The engine's luma and desaturation helpers, for float colours and for
//! 8-bit ones, and its packed RGBA8 store: the channel conversions, the
//! scalar and per-channel scales and the two-colour blends.
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

/// A colour component in byte units to a stored byte the original's way:
/// `CVTPS2DQ` (to nearest, ties to even, the integer indefinite for NaN and
/// out-of-range values), then `PACKSSDW` and `PACKUSWB`, i.e. a saturating
/// store to 0..=255. A NaN, either infinity or a negative value stores 0.
pub fn saturate_byte(value: f32) -> u8 {
    let rounded = crate::text::round_to_i32(value);
    let signed = rounded.clamp(i32::from(i16::MIN), i32::from(i16::MAX));
    signed.clamp(0, 255) as u8
}

/// A 0..=1 colour component to a stored byte: the component ×255 in single
/// precision, then [`saturate_byte`].
pub fn channel_byte(value: f32) -> u8 {
    saturate_byte(value * 255.0)
}

/// The three colour bytes of a packed RGBA8 word, red in the low byte.
pub fn rgb_bytes_of(packed: u32) -> [u8; 3] {
    [packed as u8, (packed >> 8) as u8, (packed >> 16) as u8]
}

/// A packed RGBA8 word's three colour channels as floats (× 1/255), the
/// alpha byte ignored.
pub fn unpack_rgb(packed: u32) -> [f32; 3] {
    let bytes = rgb_bytes_of(packed);
    bytes.map(|byte| f32::from(byte) * crate::pixel::BYTE_UNIT)
}

/// A packed RGBA8 word as four floats (× 1/255), red first.
pub fn unpack_rgba(packed: u32) -> [f32; 4] {
    let rgb = unpack_rgb(packed);
    [
        rgb[0],
        rgb[1],
        rgb[2],
        f32::from((packed >> 24) as u8) * crate::pixel::BYTE_UNIT,
    ]
}

/// Three 0..=1 floats packed into the low three bytes, red first. The
/// original writes only those three bytes, so the alpha byte is zero here.
pub fn pack_rgb(colour: [f32; 3]) -> u32 {
    let [r, g, b] = colour;
    u32::from(channel_byte(r))
        | (u32::from(channel_byte(g)) << 8)
        | (u32::from(channel_byte(b)) << 16)
}

/// Four 0..=1 floats packed into an RGBA8 word, red in the low byte.
pub fn pack_rgba(colour: [f32; 4]) -> u32 {
    pack_rgb([colour[0], colour[1], colour[2]]) | (u32::from(channel_byte(colour[3])) << 24)
}

/// [`pack_rgba`] with the alpha byte forced to 255.
pub fn pack_rgba_opaque(colour: [f32; 4]) -> u32 {
    pack_rgba(colour) | 0xFF00_0000
}

/// A packed word's four bytes as floats in *byte units* (no division by
/// 255), the form the scale and blend routines work in.
fn bytes_as_units(packed: u32) -> [f32; 4] {
    [
        f32::from(packed as u8),
        f32::from((packed >> 8) as u8),
        f32::from((packed >> 16) as u8),
        f32::from((packed >> 24) as u8),
    ]
}

/// Three byte-unit floats stored into the low three bytes.
fn pack_units_rgb(colour: [f32; 3]) -> u32 {
    u32::from(saturate_byte(colour[0]))
        | (u32::from(saturate_byte(colour[1])) << 8)
        | (u32::from(saturate_byte(colour[2])) << 16)
}

/// Four byte-unit floats stored into an RGBA8 word.
fn pack_units_rgba(colour: [f32; 4]) -> u32 {
    pack_units_rgb([colour[0], colour[1], colour[2]]) | (u32::from(saturate_byte(colour[3])) << 24)
}

/// A packed colour's three channels scaled by one float and stored again;
/// the alpha is not stored.
pub fn scale_rgb(packed: u32, scale: f32) -> u32 {
    let colour = bytes_as_units(packed);
    pack_units_rgb([colour[0] * scale, colour[1] * scale, colour[2] * scale])
}

/// A packed colour scaled by one float on every channel, alpha included.
pub fn scale_rgba(packed: u32, scale: f32) -> u32 {
    let colour = bytes_as_units(packed);
    pack_units_rgba(colour.map(|channel| channel * scale))
}

/// A packed colour's four channels scaled by four floats, one each.
pub fn scale_rgba_channels(packed: u32, scale: [f32; 4]) -> u32 {
    let colour = bytes_as_units(packed);
    pack_units_rgba([
        colour[0] * scale[0],
        colour[1] * scale[1],
        colour[2] * scale[2],
        colour[3] * scale[3],
    ])
}

/// Two packed colours blended in byte units: `a + (b − a)·t` on each
/// channel, stored saturated. `t` of 0 keeps `a`, 1 gives `b`; a NaN `t`
/// stores zero.
pub fn lerp_rgba(a: u32, b: u32, t: f32) -> u32 {
    let a = bytes_as_units(a);
    let b = bytes_as_units(b);
    let mut out = [0.0f32; 4];
    for (slot, (a, b)) in out.iter_mut().zip(a.iter().zip(b.iter())) {
        *slot = (b - a) * t + a;
    }
    pack_units_rgba(out)
}

/// [`lerp_rgba`] on the three colour channels only; the alpha byte is zero.
pub fn lerp_rgb(a: u32, b: u32, t: f32) -> u32 {
    let a = bytes_as_units(a);
    let b = bytes_as_units(b);
    let mut out = [0.0f32; 3];
    for (slot, (a, b)) in out.iter_mut().zip(a.iter().zip(b.iter())) {
        *slot = (b - a) * t + a;
    }
    pack_units_rgb(out)
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

    #[test]
    fn channel_stores_round_to_even_and_saturate() {
        assert_eq!(channel_byte(0.0), 0);
        assert_eq!(channel_byte(1.0), 255);
        // 0.5 × 255 = 127.5, ties to even: 128.
        assert_eq!(channel_byte(0.5), 128);
        // Ties go to the even byte on both sides.
        assert_eq!(saturate_byte(128.5), 128);
        assert_eq!(saturate_byte(127.5), 128);
        // 1/255 is a byte exactly after the multiply.
        assert_eq!(channel_byte(1.0 / 255.0), 1);
        // Saturation and the integer indefinite.
        assert_eq!(saturate_byte(-1.0), 0);
        assert_eq!(saturate_byte(255.4), 255);
        assert_eq!(saturate_byte(1.0e30), 0);
        assert_eq!(saturate_byte(f32::NAN), 0);
        assert_eq!(saturate_byte(f32::INFINITY), 0);
        assert_eq!(saturate_byte(f32::NEG_INFINITY), 0);
        assert_eq!(channel_byte(0.25), 64);
    }

    #[test]
    fn packed_words_unpack_red_first() {
        assert_eq!(rgb_bytes_of(0x00_80_40_20), [0x20, 0x40, 0x80]);
        let rgb = unpack_rgb(0x00_80_40_20);
        assert_eq!(rgb[0], 32.0 * crate::pixel::BYTE_UNIT);
        assert_eq!(rgb[2], 128.0 * crate::pixel::BYTE_UNIT);
        assert_eq!(unpack_rgba(0xFF_00_00_00), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(unpack_rgba(0x00_00_00_00), [0.0; 4]);
    }

    #[test]
    fn packing_writes_three_or_four_saturated_bytes() {
        assert_eq!(pack_rgb([1.0, 0.0, 0.5]), 0x00_80_00_FF);
        assert_eq!(pack_rgba([0.0, 0.0, 0.0, 1.0]), 0xFF00_0000);
        assert_eq!(pack_rgba_opaque([0.0, 0.0, 0.0, 0.0]), 0xFF00_0000);
        assert_eq!(pack_rgba([1.0, 0.5, 0.0, 0.5]), 0x8000_80FF);
        // Out-of-range channels saturate, NaN stores zero.
        assert_eq!(pack_rgb([2.0, -1.0, f32::NAN]), 0x00_00_00_FF);
    }

    #[test]
    fn scaling_and_blending_work_in_byte_units() {
        let colour = 0x00_10_20_30;
        assert_eq!(scale_rgb(colour, 2.0), 0x00_20_40_60);
        assert_eq!(scale_rgb(colour, 0.0), 0);
        assert_eq!(scale_rgb(colour, 300.0), 0x00FF_FFFF);
        assert_eq!(scale_rgba(0x40_10_20_30, 2.0), 0x80_20_40_60);
        // One scale per channel, alpha included.
        assert_eq!(
            scale_rgba_channels(0x40_10_20_30, [1.0, 2.0, 0.0, 0.5]),
            0x20_00_40_30
        );
        // A blend of black and white at half is 128 on every channel,
        // because 127.5 ties to even.
        assert_eq!(lerp_rgba(0x0000_0000, 0xFFFF_FFFF, 0.5), 0x8080_8080);
        assert_eq!(lerp_rgba(0x1122_3344, 0xFFFF_FFFF, 0.0), 0x1122_3344);
        assert_eq!(lerp_rgba(0x1122_3344, 0xFFFF_FFFF, 1.0), 0xFFFF_FFFF);
        assert_eq!(lerp_rgba(0x1122_3344, 0xFFFF_FFFF, f32::NAN), 0);
        // The RGB blend leaves the alpha byte unwritten.
        assert_eq!(lerp_rgb(0x0000_0000, 0xFFFF_FFFF, 0.5), 0x00_80_80_80);
    }
}
