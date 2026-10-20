//! Pixel-format converters: the original's image formats and the routines
//! that turn one into another, byte for byte.
//!
//! A format id is a bit-packed word, the same word an `.rfi` header stores
//! (see [`crate::rfi::PixelFormat::from_word`]). Bits 6–12 count the colour
//! bits of a pixel and bits 0–5 the alpha bits; bit 18 marks a bit-packed
//! format, and bit 31 a block-compressed one. The original registers 14
//! eight-bit and 6 float formats, each with a fallback and a converter
//! ([`EIGHT_BIT_FORMATS`], [`FLOAT_FORMATS`]). A converter takes a source
//! image and a target id and either writes the target pixels or declines.
//! Most compare the target under the mask `0x800C_FFFF`, which drops the
//! data-kind bits, so `0x0111_C600` and `0x1111_C600` count as RGB8 there;
//! the two-channel converters compare it whole.
//!
//! The float helpers are the same routines the mipmapper picks per format:
//! bytes to floats by `× 1/255` (linear), squared (gamma 2), or centred
//! (`(b − 127.5)/127.5`, signed), and back with clamps and `√` (see
//! [`MipConversion`]).
//!
//! Rounding is the processor's default conversion, to nearest with ties to
//! even; a NaN or out-of-range value converts to the integer indefinite
//! value, whose low byte is 0. Bytes keep the low eight bits of the integer.
//!
//! Evidence: reverse engineering of the original's format registrations,
//! its converters and its mip builder (REA). Rules: `docs/formats/RFI.md`
//! (Pixel-format conversion).

use crate::{Error, Result};

/// The registered format ids.
pub mod id {
    pub const RGB8: u32 = 0x0100_C600;
    pub const LUM8: u32 = 0x0100_4200;
    pub const RGBA8: u32 = 0x0100_E608;
    pub const LUMA8: u32 = 0x0100_6208;
    pub const ALPHA8: u32 = 0x0100_2008;
    pub const LUM1: u32 = 0x0104_4040;
    pub const XYZ8: u32 = 0x1111_C600;
    pub const XY8: u32 = 0x1111_8400;
    pub const VALA8: u32 = 0x0111_6208;
    pub const XYZV8: u32 = 0x1111_E608;
    pub const VAL8: u32 = 0x0111_4200;
    pub const AB8: u32 = 0x0111_8400;
    pub const ABC8: u32 = 0x0111_C600;
    pub const ABCD8: u32 = 0x0111_E608;
    /// Two 8-bit channels; reached only as a fallback and a target.
    pub const PAIR8: u32 = 0x0100_8400;
    pub const RGBF: u32 = 0x0108_D800;
    pub const RGBAF: u32 = 0x0108_F820;
    pub const LF: u32 = 0x0108_4800;
    pub const AF: u32 = 0x0108_2020;
    pub const XYZF: u32 = 0x0109_D800;
    pub const VF: u32 = 0x0109_4800;
}

/// Compares a target without its data-kind bits.
pub const TARGET_MASK: u32 = 0x800C_FFFF;

/// Which converter reads a source format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// One 8-bit grey channel.
    Grey,
    /// One 8-bit alpha channel.
    Alpha,
    /// Grey and alpha.
    GreyAlpha,
    /// Two 8-bit channels (X, Y).
    Pair,
    /// One bit per pixel, lowest bit first.
    Bits,
    /// Three 8-bit channels.
    Rgb,
    /// Four 8-bit channels.
    Rgba,
    /// Three floats.
    Rgbf,
    /// Four floats.
    Rgbaf,
    /// One float, clamped inclusively.
    Scalarf,
    /// One float value, scaled without a clamp into RGB.
    Valuef,
}

/// A registered format: its id, the format it falls back to, its name and
/// the converter that reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    pub id: u32,
    pub fallback: u32,
    pub name: &'static str,
    pub source: Source,
}

const fn format(id: u32, fallback: u32, name: &'static str, source: Source) -> Format {
    Format {
        id,
        fallback,
        name,
        source,
    }
}

/// The eight-bit formats in the order the original registers them.
pub const EIGHT_BIT_FORMATS: [Format; 14] = [
    format(id::RGB8, id::RGB8, "RGB8", Source::Rgb),
    format(id::LUM8, id::RGB8, "LUM8", Source::Grey),
    format(id::RGBA8, id::RGBA8, "RGBA8", Source::Rgba),
    format(id::LUMA8, id::RGBA8, "LUMA8", Source::GreyAlpha),
    format(id::ALPHA8, id::RGB8, "ALPHA8", Source::Alpha),
    format(id::LUM1, id::LUM8, "LUM1", Source::Bits),
    format(id::XYZ8, id::RGB8, "XYZ8", Source::Rgb),
    format(id::XY8, id::PAIR8, "XY8", Source::Pair),
    format(id::VALA8, id::LUMA8, "VALA8", Source::GreyAlpha),
    format(id::XYZV8, id::RGBA8, "XYZV8", Source::Rgba),
    format(id::VAL8, id::LUM8, "VAL8", Source::Grey),
    format(id::AB8, id::PAIR8, "AB8", Source::Pair),
    format(id::ABC8, id::RGB8, "ABC8", Source::Rgb),
    format(id::ABCD8, id::RGBA8, "ABCD8", Source::Rgba),
];

/// The float formats in the order the original registers them.
pub const FLOAT_FORMATS: [Format; 6] = [
    format(id::RGBF, id::RGB8, "RGBf", Source::Rgbf),
    format(id::RGBAF, id::RGBA8, "RGBAf", Source::Rgbaf),
    format(id::LF, id::LUM8, "Lf", Source::Scalarf),
    format(id::AF, id::ALPHA8, "Af", Source::Scalarf),
    format(id::XYZF, id::XYZ8, "XYZf", Source::Rgbf),
    format(id::VF, id::VAL8, "Vf", Source::Valuef),
];

/// The registered format with this id.
pub fn lookup(format_id: u32) -> Option<&'static Format> {
    EIGHT_BIT_FORMATS
        .iter()
        .chain(FLOAT_FORMATS.iter())
        .find(|f| f.id == format_id)
}

/// Bits of one pixel: the colour bits plus the alpha bits.
pub fn bits_per_pixel(format_id: u32) -> u32 {
    ((format_id & 0x1FC0) >> 6) + (format_id & 0x3F)
}

/// Bytes of `pixels` pixels of an uncompressed format: whole bytes per
/// pixel, or the bits rounded up to a byte when the format is bit-packed.
/// `None` for block-compressed formats.
pub fn pixel_bytes(format_id: u32, pixels: usize) -> Option<usize> {
    if format_id & 0x8000_0000 != 0 {
        return None;
    }
    let bits = bits_per_pixel(format_id) as usize;
    Some(if format_id & 0x0004_0000 != 0 {
        (bits * pixels).div_ceil(8)
    } else {
        bits.div_ceil(8) * pixels
    })
}

/// Rounds to the nearest integer, ties to even, as the processor's default
/// float conversion does; NaN and values beyond `i64` give `i64::MIN`.
pub fn round_f32(value: f32) -> i64 {
    const LIMIT: f32 = 9.223_372e18;
    if value.is_nan() || !(-LIMIT..LIMIT).contains(&value) {
        i64::MIN
    } else {
        value.round_ties_even() as i64
    }
}

/// [`round_f32`] for a double.
pub fn round_f64(value: f64) -> i64 {
    const LIMIT: f64 = 9.223_372_036_854_775_808e18;
    if value.is_nan() || !(-LIMIT..LIMIT).contains(&value) {
        i64::MIN
    } else {
        value.round_ties_even() as i64
    }
}

/// The low byte of a rounded value.
fn byte_f32(value: f32) -> u8 {
    round_f32(value) as u8
}

/// `1/255` as the original stores it.
pub const BYTE_UNIT: f32 = f32::from_bits(0x3B80_8081);
/// `1/127.5` as the original stores it.
pub const SIGNED_UNIT: f32 = f32::from_bits(0x3C00_8081);
const SIGNED_CENTRE: f32 = 127.5;
/// The smallest `1 − x² − y²` that keeps a rebuilt Z above 0.
const Z_FLOOR: f32 = f32::from_bits(0x33FF_FFD7);
/// Above this (as raw bits) a gamma channel is full.
const GAMMA_FULL_BITS: u32 = 0x3F7F_7F6F;

/// Rec. 601 luma of an RGB byte triple, in double precision.
pub fn luma(r: u8, g: u8, b: u8) -> u8 {
    let sum = f64::from(g) * 0.587 + f64::from(r) * 0.299;
    round_f64(f64::from(b) * 0.114 + sum) as u8
}

/// The Z byte of a two-channel normal: X and Y centred on 127.5 and scaled
/// by `1/127.5`, `Z = √(1 − x² − y²)` mapped back to bytes, or 0 when the
/// remainder is not above the floor.
pub fn rebuild_z(x: u8, y: u8) -> u8 {
    let nx = (f32::from(x) - SIGNED_CENTRE) * SIGNED_UNIT;
    let ny = (f32::from(y) - SIGNED_CENTRE) * SIGNED_UNIT;
    let rest = (1.0 - nx * nx) - ny * ny;
    if rest > Z_FLOOR {
        byte_f32(rest.sqrt() * SIGNED_CENTRE + SIGNED_CENTRE)
    } else {
        0
    }
}

/// One bit per pixel, lowest bit first, to 0 or 255.
pub fn expand_bits(src: &[u8], pixels: usize, dst: &mut [u8]) {
    for (i, out) in dst.iter_mut().take(pixels).enumerate() {
        let shift = (i & 7) as u32;
        let bit = u32::from(src.get(i >> 3).copied().unwrap_or(0)) & (1 << shift);
        *out = ((bit << (8 - shift)) as u8).wrapping_sub((bit >> shift) as u8);
    }
}

/// Bytes to floats by `× 1/255`.
pub fn unit_floats(src: &[u8], dst: &mut [f32]) {
    for (out, &b) in dst.iter_mut().zip(src) {
        *out = f32::from(b) * BYTE_UNIT;
    }
}

/// Bytes to floats by `× 1/255`, squared (gamma 2 to linear).
pub fn gamma_floats(src: &[u8], dst: &mut [f32]) {
    for (out, &b) in dst.iter_mut().zip(src) {
        let v = f32::from(b) * BYTE_UNIT;
        *out = v * v;
    }
}

/// Bytes to floats centred on 127.5: `(b − 127.5) × 1/127.5`.
pub fn signed_floats(src: &[u8], dst: &mut [f32]) {
    for (out, &b) in dst.iter_mut().zip(src) {
        *out = (f32::from(b) - SIGNED_CENTRE) * SIGNED_UNIT;
    }
}

/// RGBA bytes to floats: colour squared, alpha linear.
pub fn gamma_colour_floats(src: &[u8], dst: &mut [f32]) {
    for (out, px) in dst
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(src.as_chunks::<4>().0.iter())
    {
        for c in 0..3 {
            let v = f32::from(px[c]) * BYTE_UNIT;
            out[c] = v * v;
        }
        out[3] = f32::from(px[3]) * BYTE_UNIT;
    }
}

/// Grey-alpha bytes to floats: grey squared, alpha linear.
pub fn gamma_pair_floats(src: &[u8], dst: &mut [f32]) {
    for (out, px) in dst
        .as_chunks_mut::<2>()
        .0
        .iter_mut()
        .zip(src.as_chunks::<2>().0.iter())
    {
        let v = f32::from(px[0]) * BYTE_UNIT;
        out[0] = v * v;
        out[1] = f32::from(px[1]) * BYTE_UNIT;
    }
}

/// A float in 0..1 to a byte: 0 unless above 0, 255 unless below 1, else
/// `v × 255` rounded. NaN rounds to 0.
pub fn unit_byte(v: f32) -> u8 {
    if !(v.is_nan() || v > 0.0) {
        0
    } else if !(v.is_nan() || v < 1.0) {
        255
    } else {
        byte_f32(v * 255.0)
    }
}

/// [`unit_byte`] with the bounds included: 0 only below 0, 255 only above 1.
pub fn unit_byte_inclusive(v: f32) -> u8 {
    if v < 0.0 {
        0
    } else if v > 1.0 {
        255
    } else {
        byte_f32(v * 255.0)
    }
}

/// A float in −1..1 to a byte: `v × 127.5 + 127.5`, 0 unless above −1,
/// 255 unless below 1.
pub fn signed_byte(v: f32) -> u8 {
    if !(v.is_nan() || v > -1.0) {
        0
    } else if !(v.is_nan() || v < 1.0) {
        255
    } else {
        byte_f32(v * SIGNED_CENTRE + SIGNED_CENTRE)
    }
}

/// A squared float back to a byte through `√`, as [`unit_byte`] bounds it.
pub fn gamma_colour_byte(v: f32) -> u8 {
    if !(v.is_nan() || v > 0.0) {
        0
    } else if !(v.is_nan() || v < 1.0) {
        255
    } else {
        byte_f32(v.sqrt() * 255.0)
    }
}

/// A squared float back to a byte through `√`, bounded on its raw bits: 0
/// when they read as a non-positive integer (zero, negatives, negative
/// NaNs), 255 from just under 1 up (and positive NaNs).
pub fn gamma_byte(v: f32) -> u8 {
    let bits = v.to_bits();
    if bits as i32 <= 0 {
        0
    } else if bits >= GAMMA_FULL_BITS {
        255
    } else {
        byte_f32(v.sqrt() * 255.0)
    }
}

/// Floats to bytes by [`unit_byte`].
pub fn unit_bytes(src: &[f32], dst: &mut [u8]) {
    for (out, &v) in dst.iter_mut().zip(src) {
        *out = unit_byte(v);
    }
}

/// Floats to bytes by [`gamma_byte`].
pub fn gamma_bytes(src: &[f32], dst: &mut [u8]) {
    for (out, &v) in dst.iter_mut().zip(src) {
        *out = gamma_byte(v);
    }
}

/// Floats to bytes by [`signed_byte`].
pub fn signed_bytes(src: &[f32], dst: &mut [u8]) {
    for (out, &v) in dst.iter_mut().zip(src) {
        *out = signed_byte(v);
    }
}

/// RGBA floats to bytes: colour through `√`, alpha linear.
pub fn gamma_colour_bytes(src: &[f32], dst: &mut [u8]) {
    for (out, px) in dst
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(src.as_chunks::<4>().0.iter())
    {
        for c in 0..3 {
            out[c] = gamma_colour_byte(px[c]);
        }
        out[3] = unit_byte(px[3]);
    }
}

/// Grey-alpha floats to bytes: grey through `√`, alpha linear.
pub fn gamma_pair_bytes(src: &[f32], dst: &mut [u8]) {
    for (out, px) in dst
        .as_chunks_mut::<2>()
        .0
        .iter_mut()
        .zip(src.as_chunks::<2>().0.iter())
    {
        out[0] = gamma_colour_byte(px[0]);
        out[1] = unit_byte(px[1]);
    }
}

/// How the mipmapper turns a format's bytes into the floats it filters and
/// back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MipConversion {
    /// `× 1/255` and [`unit_byte`].
    Linear,
    /// Every channel squared, back through [`gamma_byte`].
    Gamma,
    /// Colour squared and alpha linear, back through `√` and
    /// [`unit_byte`] (RGBA and grey-alpha formats).
    GammaColour,
    /// Centred on 127.5 and back through [`signed_byte`].
    Signed,
}

/// The conversion the mipmapper uses for a format word, or `None` when it
/// builds no chain for it (block-compressed and unknown layouts). Signed
/// data (bit 28) is centred; a word with bit 16 or 17 is linear; otherwise
/// colour is filtered squared, with alpha kept linear for RGBA and
/// grey-alpha words.
pub fn mip_conversion(word: u32) -> Option<MipConversion> {
    let mut conversion = if word & 0x1000_0000 != 0 {
        MipConversion::Signed
    } else if word & 0x0003_0000 == 0 {
        MipConversion::Gamma
    } else {
        MipConversion::Linear
    };
    let plain = word & 0x1003_0000 == 0;
    match word & TARGET_MASK {
        0x2008 | 0x4200 | 0x8400 | 0xC600 => {}
        0x6208 | 0xE608 if plain => conversion = MipConversion::GammaColour,
        0x6208 | 0xE608 => {}
        _ => return None,
    }
    Some(conversion)
}

/// Checks that `buf` holds `needed` bytes.
fn need(buf: &[u8], needed: usize) -> Result<()> {
    if buf.len() < needed {
        Err(Error::Truncated {
            offset: 0,
            needed,
            len: buf.len(),
        })
    } else {
        Ok(())
    }
}

fn read_f32(src: &[u8], i: usize) -> f32 {
    src.get(i * 4..i * 4 + 4)
        .map_or(0.0, |b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Converts `pixels` pixels of `src` (in format `source`) into `dst` as
/// format `target`. Returns `Ok(false)` when the source's converter does not
/// handle that target (nothing is written), and an error when the source is
/// not a registered format or a buffer is too short. Float pixels are
/// little-endian `f32`.
pub fn convert(
    source: u32,
    target: u32,
    src: &[u8],
    pixels: usize,
    dst: &mut [u8],
) -> Result<bool> {
    let format =
        lookup(source).ok_or_else(|| Error::Unsupported(format!("pixel format {source:#010x}")))?;
    let src_len = pixel_bytes(source, pixels).unwrap_or(0);
    need(src, src_len)?;
    let masked = target & TARGET_MASK;
    let n = pixels;
    match format.source {
        Source::Grey => match masked {
            0x2008 | 0x4200 => {
                need(dst, n)?;
                dst[..n].copy_from_slice(&src[..n]);
            }
            0x6208 => {
                need(dst, n * 2)?;
                for (out, &l) in dst.as_chunks_mut::<2>().0.iter_mut().zip(&src[..n]) {
                    out.copy_from_slice(&[l, 255]);
                }
            }
            0xC600 => {
                need(dst, n * 3)?;
                for (out, &l) in dst.as_chunks_mut::<3>().0.iter_mut().zip(&src[..n]) {
                    out.copy_from_slice(&[l, l, l]);
                }
            }
            0xE608 => {
                need(dst, n * 4)?;
                for (out, &l) in dst.as_chunks_mut::<4>().0.iter_mut().zip(&src[..n]) {
                    out.copy_from_slice(&[l, l, l, 255]);
                }
            }
            0x0008_4800 => {
                need(dst, n * 4)?;
                for (out, &l) in dst.as_chunks_mut::<4>().0.iter_mut().zip(&src[..n]) {
                    out.copy_from_slice(&(f32::from(l) * BYTE_UNIT).to_le_bytes());
                }
            }
            _ => return Ok(false),
        },
        Source::Alpha => match masked {
            0x2008 | 0x4200 => {
                need(dst, n)?;
                dst[..n].copy_from_slice(&src[..n]);
            }
            0xC600 => {
                need(dst, n * 3)?;
                for (out, &a) in dst.as_chunks_mut::<3>().0.iter_mut().zip(&src[..n]) {
                    out.copy_from_slice(&[a, a, a]);
                }
            }
            0xE608 => {
                need(dst, n * 4)?;
                for (out, &a) in dst.as_chunks_mut::<4>().0.iter_mut().zip(&src[..n]) {
                    out.copy_from_slice(&[255, 255, 255, a]);
                }
            }
            _ => return Ok(false),
        },
        Source::GreyAlpha => {
            let pairs = src[..n * 2].as_chunks::<2>().0.iter();
            match target {
                id::ALPHA8 => {
                    need(dst, n)?;
                    for (out, p) in dst.iter_mut().zip(pairs) {
                        *out = p[1];
                    }
                }
                id::RGB8 => {
                    need(dst, n * 3)?;
                    for (out, p) in dst.as_chunks_mut::<3>().0.iter_mut().zip(pairs) {
                        out.copy_from_slice(&[p[0], p[0], p[0]]);
                    }
                }
                id::RGBA8 => {
                    need(dst, n * 4)?;
                    for (out, p) in dst.as_chunks_mut::<4>().0.iter_mut().zip(pairs) {
                        out.copy_from_slice(&[p[0], p[0], p[0], p[1]]);
                    }
                }
                _ => return Ok(false),
            }
        }
        Source::Pair => {
            let pairs = src[..n * 2].as_chunks::<2>().0.iter();
            match target {
                id::ALPHA8 => {
                    need(dst, n)?;
                    for (out, p) in dst.iter_mut().zip(pairs) {
                        *out = p[1];
                    }
                }
                id::RGB8 => {
                    need(dst, n * 3)?;
                    for (out, p) in dst.as_chunks_mut::<3>().0.iter_mut().zip(pairs) {
                        out.copy_from_slice(&[p[0], p[1], 255]);
                    }
                }
                id::RGBA8 => {
                    need(dst, n * 4)?;
                    for (out, p) in dst.as_chunks_mut::<4>().0.iter_mut().zip(pairs) {
                        out.copy_from_slice(&[p[0], p[1], 255, 255]);
                    }
                }
                id::XYZ8 => {
                    need(dst, n * 3)?;
                    for (out, p) in dst.as_chunks_mut::<3>().0.iter_mut().zip(pairs) {
                        out.copy_from_slice(&[p[0], p[1], rebuild_z(p[0], p[1])]);
                    }
                }
                _ => return Ok(false),
            }
        }
        Source::Bits => match target {
            id::LUM8 | id::VAL8 => {
                need(dst, n)?;
                expand_bits(src, n, dst);
            }
            id::RGB8 => {
                need(dst, n * 3)?;
                expand_bits(src, n, dst);
                // Spread in place from the last pixel down, as the original
                // does.
                for i in (0..n).rev() {
                    let l = dst[i];
                    dst[i * 3..i * 3 + 3].copy_from_slice(&[l, l, l]);
                }
            }
            _ => return Ok(false),
        },
        Source::Rgb => {
            let px = src[..n * 3].as_chunks::<3>().0.iter();
            match masked {
                0x2008 | 0x4200 => {
                    need(dst, n)?;
                    for (out, p) in dst.iter_mut().zip(px) {
                        *out = luma(p[0], p[1], p[2]);
                    }
                }
                0x6208 => {
                    // The original's grey-alpha target gets an alpha of 1.
                    need(dst, n * 2)?;
                    for (out, p) in dst.as_chunks_mut::<2>().0.iter_mut().zip(px) {
                        out.copy_from_slice(&[luma(p[0], p[1], p[2]), 1]);
                    }
                }
                0x8400 => {
                    need(dst, n * 2)?;
                    for (out, p) in dst.as_chunks_mut::<2>().0.iter_mut().zip(px) {
                        out.copy_from_slice(&[p[0], p[1]]);
                    }
                }
                0xE608 => {
                    need(dst, n * 4)?;
                    for (out, p) in dst.as_chunks_mut::<4>().0.iter_mut().zip(px) {
                        out.copy_from_slice(&[p[0], p[1], p[2], 255]);
                    }
                }
                _ => return Ok(false),
            }
        }
        Source::Rgba => {
            let px = src[..n * 4].as_chunks::<4>().0.iter();
            let colour = target & 0x8F0C_FFFF;
            if colour == id::RGB8 || colour == id::XYZ8 {
                need(dst, n * 3)?;
                for (out, p) in dst.as_chunks_mut::<3>().0.iter_mut().zip(px) {
                    out.copy_from_slice(&p[..3]);
                }
            } else {
                match masked {
                    0x2008 => {
                        need(dst, n)?;
                        for (out, p) in dst.iter_mut().zip(px) {
                            *out = p[3];
                        }
                    }
                    0x4200 => {
                        need(dst, n)?;
                        for (out, p) in dst.iter_mut().zip(px) {
                            *out = luma(p[0], p[1], p[2]);
                        }
                    }
                    0x6208 => {
                        need(dst, n * 2)?;
                        for (out, p) in dst.as_chunks_mut::<2>().0.iter_mut().zip(px) {
                            out.copy_from_slice(&[luma(p[0], p[1], p[2]), p[3]]);
                        }
                    }
                    _ => return Ok(false),
                }
            }
        }
        Source::Rgbf | Source::Rgbaf => {
            let channels = if format.source == Source::Rgbf { 3 } else { 4 };
            let (unit, signed) = if channels == 3 {
                (id::RGB8, id::XYZ8)
            } else {
                (id::RGBA8, id::XYZV8)
            };
            let byte: fn(f32) -> u8 = if target == unit {
                unit_byte
            } else if target == signed {
                signed_byte
            } else {
                return Ok(false);
            };
            let count = n * channels;
            need(dst, count)?;
            for (i, out) in dst[..count].iter_mut().enumerate() {
                *out = byte(read_f32(src, i));
            }
        }
        Source::Scalarf => match masked {
            0x2008 | 0x4200 => {
                need(dst, n)?;
                for (i, out) in dst[..n].iter_mut().enumerate() {
                    *out = unit_byte_inclusive(read_f32(src, i));
                }
            }
            0xC600 => {
                need(dst, n * 3)?;
                for (i, out) in dst.as_chunks_mut::<3>().0.iter_mut().take(n).enumerate() {
                    out.fill(unit_byte_inclusive(read_f32(src, i)));
                }
            }
            _ => return Ok(false),
        },
        Source::Valuef => match masked {
            0x2008 | 0x4200 => {
                need(dst, n)?;
                for (i, out) in dst[..n].iter_mut().enumerate() {
                    *out = unit_byte(read_f32(src, i));
                }
            }
            0xC600 => {
                // The value range 0..1, scaled without a clamp.
                let (lo, hi) = (0.0f32, 1.0f32);
                let scale = 255.0 / (hi - lo);
                need(dst, n * 3)?;
                for (i, out) in dst.as_chunks_mut::<3>().0.iter_mut().take(n).enumerate() {
                    out.fill(byte_f32((read_f32(src, i) - lo) * scale));
                }
            }
            _ => return Ok(false),
        },
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registrations_and_sizes() {
        assert_eq!(EIGHT_BIT_FORMATS.len() + FLOAT_FORMATS.len(), 20);
        let lum1 = lookup(id::LUM1).unwrap();
        assert_eq!((lum1.name, lum1.fallback), ("LUM1", id::LUM8));
        assert_eq!(lookup(id::VF).unwrap().fallback, id::VAL8);
        assert_eq!(bits_per_pixel(id::RGB8), 24);
        assert_eq!(bits_per_pixel(id::RGBA8), 32);
        assert_eq!(bits_per_pixel(id::RGBAF), 128);
        assert_eq!(pixel_bytes(id::LUMA8, 5), Some(10));
        assert_eq!(pixel_bytes(id::LUM1, 9), Some(2));
        assert_eq!(pixel_bytes(id::RGBF, 2), Some(24));
        assert_eq!(pixel_bytes(0x813B_C600, 16), None);
    }

    #[test]
    fn luma_weighs_601_in_doubles() {
        // 2.99 + 117.4 + 3.42 = 123.81.
        assert_eq!(luma(10, 200, 30), 124);
        assert_eq!(luma(255, 255, 255), 255);
        assert_eq!(luma(0, 0, 1), 0);
        // 0.299 · 5 = 1.495 rounds down, 0.587 · 1 = 0.587 rounds up.
        assert_eq!(luma(5, 0, 0), 1);
        assert_eq!(luma(0, 1, 0), 1);
    }

    #[test]
    fn z_is_rebuilt_from_x_and_y() {
        assert_eq!(rebuild_z(128, 128), 255);
        assert_eq!(rebuild_z(200, 128), 232);
        // x past 1 leaves no room: 0, not 127.
        assert_eq!(rebuild_z(255, 128), 0);
        assert_eq!(rebuild_z(0, 0), 0);
    }

    #[test]
    fn bits_expand_lowest_first() {
        let mut out = [9u8; 10];
        expand_bits(&[0b0000_0101, 0b0000_0010], 10, &mut out);
        assert_eq!(out, [255, 0, 255, 0, 0, 0, 0, 0, 0, 255]);
    }

    #[test]
    fn float_bytes_round_to_even_and_clamp() {
        assert_eq!(unit_byte(0.5), 128);
        assert_eq!(unit_byte(0.0), 0);
        assert_eq!(unit_byte(1.0), 255);
        assert_eq!(unit_byte(f32::NAN), 0);
        assert_eq!(unit_byte(-3.0), 0);
        assert_eq!(unit_byte_inclusive(-0.0), 0);
        assert_eq!(unit_byte_inclusive(1.0), 255);
        assert_eq!(unit_byte_inclusive(2.0), 255);
        assert_eq!(signed_byte(-1.0), 0);
        assert_eq!(signed_byte(0.0), 128);
        // −0.5 · 127.5 + 127.5 = 63.75.
        assert_eq!(signed_byte(-0.5), 64);
        assert_eq!(signed_byte(1.0), 255);
        // √0.25 · 255 = 127.5, to even.
        assert_eq!(gamma_colour_byte(0.25), 128);
        assert_eq!(gamma_byte(0.25), 128);
        // The two square-root routes part on NaN and below zero.
        assert_eq!(gamma_colour_byte(f32::NAN), 0);
        assert_eq!(gamma_byte(f32::NAN), 255);
        assert_eq!(gamma_byte(-f32::NAN), 0);
        assert_eq!(gamma_byte(f32::from_bits(0x3F7F_7F6F)), 255);
        // √(0.998) · 255 = 254.74.
        assert_eq!(gamma_byte(f32::from_bits(0x3F7F_7F6E)), 255);
        assert_eq!(gamma_byte(0.01), 26);
    }

    #[test]
    fn byte_floats() {
        let mut out = [0.0f32; 3];
        unit_floats(&[0, 51, 255], &mut out);
        assert_eq!(out, [0.0, 51.0 * BYTE_UNIT, 255.0 * BYTE_UNIT]);
        assert_eq!(BYTE_UNIT, 0.003_921_569);
        gamma_floats(&[0, 51, 255], &mut out);
        assert_eq!(out[1], (51.0 * BYTE_UNIT) * (51.0 * BYTE_UNIT));
        signed_floats(&[0, 128, 255], &mut out);
        assert_eq!(out[1], 0.5 * SIGNED_UNIT);
        assert!((out[0] + 1.0).abs() < 1e-6 && (out[2] - 1.0).abs() < 1e-6);
        let mut quad = [0.0f32; 4];
        gamma_colour_floats(&[51, 51, 51, 51], &mut quad);
        assert_eq!(quad[3], 51.0 * BYTE_UNIT);
        assert_eq!(quad[0], quad[3] * quad[3]);
        let mut back = [0u8; 4];
        gamma_colour_bytes(&quad, &mut back);
        assert_eq!(back, [51; 4]);
    }

    fn run(source: u32, target: u32, src: &[u8], pixels: usize, out_len: usize) -> Option<Vec<u8>> {
        let mut dst = vec![0xAAu8; out_len];
        convert(source, target, src, pixels, &mut dst)
            .unwrap()
            .then_some(dst)
    }

    #[test]
    fn eight_bit_sources_to_their_targets() {
        assert_eq!(
            run(id::LUM8, id::RGBA8, &[7, 9], 2, 8).unwrap(),
            [7, 7, 7, 255, 9, 9, 9, 255]
        );
        assert_eq!(run(id::LUM8, id::LUMA8, &[7], 1, 2).unwrap(), [7, 255]);
        assert_eq!(run(id::LUM8, id::ALPHA8, &[7], 1, 1).unwrap(), [7]);
        let floats = run(id::LUM8, id::LF, &[51], 1, 4).unwrap();
        assert_eq!(floats, (51.0 * BYTE_UNIT).to_le_bytes());
        assert_eq!(
            run(id::ALPHA8, id::RGBA8, &[40], 1, 4).unwrap(),
            [255, 255, 255, 40]
        );
        assert_eq!(run(id::ALPHA8, id::RGB8, &[40], 1, 3).unwrap(), [40; 3]);
        assert_eq!(
            run(id::LUMA8, id::RGBA8, &[7, 40], 1, 4).unwrap(),
            [7, 7, 7, 40]
        );
        assert_eq!(run(id::LUMA8, id::ALPHA8, &[7, 40], 1, 1).unwrap(), [40]);
        // The grey-alpha converter compares targets whole: no LUM8.
        assert_eq!(run(id::LUMA8, id::LUM8, &[7, 40], 1, 1), None);
        assert_eq!(
            run(id::XY8, id::XYZ8, &[200, 128], 1, 3).unwrap(),
            [200, 128, 232]
        );
        assert_eq!(
            run(id::XY8, id::RGBA8, &[200, 128], 1, 4).unwrap(),
            [200, 128, 255, 255]
        );
        assert_eq!(
            run(id::LUM1, id::RGB8, &[0b10], 2, 6).unwrap(),
            [0, 0, 0, 255, 255, 255]
        );
        assert_eq!(run(id::LUM1, id::LUM8, &[0b10], 2, 2).unwrap(), [0, 255]);
        assert_eq!(run(id::LUM1, id::ALPHA8, &[0b10], 2, 2), None);
        assert_eq!(
            run(id::RGB8, id::LUM8, &[10, 200, 30], 1, 1).unwrap(),
            [124]
        );
        // The original's RGB to grey-alpha writes an alpha of 1.
        assert_eq!(
            run(id::RGB8, id::LUMA8, &[10, 200, 30], 1, 2).unwrap(),
            [124, 1]
        );
        assert_eq!(run(id::XYZ8, id::PAIR8, &[1, 2, 3], 1, 2).unwrap(), [1, 2]);
        assert_eq!(
            run(id::RGB8, id::RGBA8, &[1, 2, 3], 1, 4).unwrap(),
            [1, 2, 3, 255]
        );
        assert_eq!(run(id::RGB8, id::RGB8, &[1, 2, 3], 1, 3), None);
        assert_eq!(
            run(id::RGBA8, id::XYZ8, &[1, 2, 3, 4], 1, 3).unwrap(),
            [1, 2, 3]
        );
        assert_eq!(
            run(id::RGBA8, id::ABC8, &[1, 2, 3, 4], 1, 3).unwrap(),
            [1, 2, 3]
        );
        assert_eq!(
            run(id::RGBA8, id::ALPHA8, &[1, 2, 3, 4], 1, 1).unwrap(),
            [4]
        );
        assert_eq!(
            run(id::RGBA8, id::LUMA8, &[10, 200, 30, 4], 1, 2).unwrap(),
            [124, 4]
        );
    }

    fn floats(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn float_sources_to_bytes() {
        let rgb = floats(&[0.5, -0.5, 2.0]);
        assert_eq!(run(id::RGBF, id::RGB8, &rgb, 1, 3).unwrap(), [128, 0, 255]);
        assert_eq!(run(id::XYZF, id::XYZ8, &rgb, 1, 3).unwrap(), [191, 64, 255]);
        assert_eq!(run(id::RGBF, id::RGBA8, &rgb, 1, 4), None);
        let rgba = floats(&[0.0, 0.25, 1.0, 0.5]);
        assert_eq!(
            run(id::RGBAF, id::RGBA8, &rgba, 1, 4).unwrap(),
            [0, 64, 255, 128]
        );
        let v = floats(&[1.0, -0.0]);
        assert_eq!(run(id::LF, id::LUM8, &v, 2, 2).unwrap(), [255, 0]);
        assert_eq!(
            run(id::AF, id::RGB8, &v, 2, 6).unwrap(),
            [255, 255, 255, 0, 0, 0]
        );
        // Vf to RGB is not clamped: 1.5 · 255 = 382.5 → 382, low byte 126.
        let v = floats(&[1.5, -0.01]);
        assert_eq!(
            run(id::VF, id::RGB8, &v, 2, 6).unwrap(),
            [126, 126, 126, 253, 253, 253]
        );
        assert_eq!(run(id::VF, id::LUM8, &v, 2, 2).unwrap(), [255, 0]);
    }

    #[test]
    fn short_buffers_and_unknown_formats_are_errors() {
        let mut dst = [0u8; 2];
        assert!(convert(id::LUM8, id::RGBA8, &[1], 1, &mut dst).is_err());
        assert!(convert(id::LUM8, id::LUM8, &[], 1, &mut dst).is_err());
        assert!(convert(0x813B_C600, id::RGBA8, &[1], 1, &mut dst).is_err());
    }

    #[test]
    fn mip_conversions_follow_the_word() {
        assert_eq!(mip_conversion(id::RGB8), Some(MipConversion::Gamma));
        assert_eq!(mip_conversion(id::LUM8), Some(MipConversion::Gamma));
        assert_eq!(mip_conversion(id::ALPHA8), Some(MipConversion::Gamma));
        assert_eq!(mip_conversion(id::RGBA8), Some(MipConversion::GammaColour));
        assert_eq!(mip_conversion(id::LUMA8), Some(MipConversion::GammaColour));
        assert_eq!(mip_conversion(id::ABC8), Some(MipConversion::Linear));
        assert_eq!(mip_conversion(id::VAL8), Some(MipConversion::Linear));
        assert_eq!(mip_conversion(id::ABCD8), Some(MipConversion::Linear));
        assert_eq!(mip_conversion(id::XYZ8), Some(MipConversion::Signed));
        assert_eq!(mip_conversion(id::XYZV8), Some(MipConversion::Signed));
        assert_eq!(mip_conversion(0x813B_C600), None);
        assert_eq!(mip_conversion(id::LF), None);
    }
}
