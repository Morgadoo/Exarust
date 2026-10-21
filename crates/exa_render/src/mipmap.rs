//! The original's image resampling unit (`RFP_Mipmaps_Kaiser`): its Kaiser
//! bank for mip chains, its windowed kernel for resampling to an arbitrary
//! size, its separable axis passes, blits and its sharpen kernel.
//!
//! **Mip chains** ([`build_mip_chain_rgba8_between`]) use a bank of 13
//! normalised kernels, one per level. Level `L` has `n = 8·2^(L−1)` taps,
//! `h = n/2` and `c = 2^−L`. Tap `i` uses `u = i − h + 0.5`, `t = u/h` and
//! `w = I0(4·√(1 − t²))/I0(4) · sinc(π·c·u)`, or 0 outside `|t| < 1`; the
//! weights are normalised to sum one. Each level is filtered directly from
//! the stored base level, horizontally and then vertically, with its own
//! table. The axis pass takes `ratio = ⌊src/dst⌋` and starts at
//! `−((n − ratio) ≫ 1)`, wrapping or clamping every tap.
//!
//! **Resampling to an arbitrary size** ([`resample_rgba8_to_size`]) uses a
//! different, phase-shifted window: for every output texel the taps are
//! centred on `i·src/dst` and reach two output texels each way, with
//! `t = min(|tap|·scale, 2)/2`, `scale = min(dst/src, 1)` and
//! `w = (1 − t)³·(1 + 3t − 3t² − 13.6t³)`, normalised. The same window
//! kernel also scales icons, cursors, terrain inputs and GUI images in the
//! original.
//!
//! Channels are filtered interleaved, as f32, in the space the format word
//! picks: linear, colour squared, or centred for signed data
//! ([`exa_formats::pixel::mip_conversion`]). A mip chain keeps going until
//! a 1×1 level has been filtered (a level whose sides sum to two is the
//! last); dimensions shift right with a floor of one, so non-power-of-two
//! sides are allowed.
//!
//! Evidence: reverse engineering of the original's unit (the kernel
//! builders, the bank, the axis passes and the chain loop; REA). Rules:
//! `docs/formats/RFI.md` (Generated mip levels).

use exa_formats::pixel::{self, MipConversion};

/// The Kaiser window's beta.
pub const KAISER_BETA: f64 = 4.0;
/// The tap base of level 1.
pub const KAISER_BASE_TAPS: usize = 8;
/// The largest table level the original builds.
pub const KAISER_MAX_LEVEL: u32 = 13;
/// The arbitrary-size window's radius, in output texels.
pub const WINDOW_RADIUS: f32 = 2.0;
/// The smallest distance the window clamp accepts.
pub const WINDOW_EPSILON: f32 = 1e-6;

/// The modified Bessel function of the first kind, order zero, by its
/// series: `Σ ((x/2)^m / m!)²`, stopped when a squared term is no larger
/// than `sum · 2^−53` (the original's `1e-16`).
pub fn bessel_i0(x: f64) -> f64 {
    let half = x * 0.5;
    // The original keeps the linear term and squares it for the sum.
    let mut term = 1.0f64;
    let mut sum = 1.0f64;
    let mut m = 1.0f64;
    loop {
        term *= half / m;
        let squared = term * term;
        sum += squared;
        if squared <= sum * 1e-16 {
            return sum;
        }
        m += 1.0;
    }
}

/// `sin(x)/x`, with the series `1 − x²/6 + x⁴/120` under `1e-4`. The
/// quadratic coefficient is the original's f32-rounded `−1/6`.
pub fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-4 {
        const MINUS_SIXTH: f64 = -0.166_666_671_633_720_4;
        (x * x * 1.0 * 0.008_333_333_333_333_333 + MINUS_SIXTH) * x * x + 1.0
    } else {
        x.sin() / x
    }
}

/// The 13 tables, built once for every texture that needs them. The
/// original builds them at start-up; a `OnceLock` is Exarust's stand-in.
fn kaiser_bank() -> &'static [Vec<f32>] {
    static BANK: std::sync::OnceLock<Vec<Vec<f32>>> = std::sync::OnceLock::new();
    BANK.get_or_init(|| (1..=KAISER_MAX_LEVEL).map(kaiser_table).collect())
}

/// The normalised Kaiser table of a mip level (1 through 13). Level 1 has
/// eight taps; each later level doubles them and halves the sinc's scale.
pub fn kaiser_table(level: u32) -> Vec<f32> {
    let level = level.clamp(1, KAISER_MAX_LEVEL);
    let taps = KAISER_BASE_TAPS << (level - 1);
    // The original computes the half width, the tap position and the sum in
    // f32, and the Bessel and sinc arguments in f64.
    let half = (taps as f32) * 0.5;
    let half64 = f64::from(half);
    // The original's bank computes `2^−L` as `exp(−L·ln 2)`.
    let c = (-(level as f64) * 0.693_147_180_559_945_3).exp();
    let mut weights = vec![0.0f32; taps];
    let mut sum = 0.0f32;
    for (index, weight) in weights.iter_mut().enumerate() {
        let u = ((index as f32 - half) + 0.5) as f64;
        let t = u / half64;
        let radius = 1.0 - t * t;
        if radius >= 0.0 {
            *weight = (sinc(std::f64::consts::PI * c * u)
                * (bessel_i0(KAISER_BETA * radius.sqrt()) / bessel_i0(KAISER_BETA)))
                as f32;
            sum += *weight;
        }
    }
    let inverse = 1.0 / sum;
    for weight in &mut weights {
        *weight *= inverse;
    }
    weights
}

/// The first source tap of one output pixel and the integer ratio.
pub fn mip_axis_start(taps: usize, ratio: usize) -> i64 {
    -((taps as i64 - ratio as i64) >> 1)
}

/// Resamples one axis of an interleaved RGBA f32 image: `src_extent` source
/// texels to `dst_extent` with `weights` taps. Each tap wraps modulo the
/// source extent or clamps to its ends.
pub fn resample_axis(
    src: &[f32],
    src_extent: usize,
    dst_extent: usize,
    weights: &[f32],
    wrap: bool,
    out: &mut [f32],
) {
    resample_axis_channels(src, 4, src_extent, dst_extent, weights, wrap, out);
}

/// [`resample_axis`] for an image of `channels` interleaved f32 channels.
/// The mip chains filter the channels the format has (one for a grey image,
/// two for a normal, three for RGB, four for RGBA); Exarust's decoded base
/// is always RGBA8, so its chains pass four.
pub fn resample_axis_channels(
    src: &[f32],
    channels: usize,
    src_extent: usize,
    dst_extent: usize,
    weights: &[f32],
    wrap: bool,
    out: &mut [f32],
) {
    if src_extent == 0 || dst_extent == 0 || weights.is_empty() || channels == 0 {
        return;
    }
    let ratio = (src_extent / dst_extent).max(1);
    let start = mip_axis_start(weights.len(), ratio);
    for x in 0..dst_extent {
        let s0 = start + x as i64 * ratio as i64;
        for channel in 0..channels {
            let mut acc = 0.0f32;
            for (k, &weight) in weights.iter().enumerate() {
                let mut source = s0 + k as i64;
                source = if wrap {
                    source.rem_euclid(src_extent as i64)
                } else {
                    source.clamp(0, src_extent as i64 - 1)
                };
                acc += weight
                    * src
                        .get(source as usize * channels + channel)
                        .copied()
                        .unwrap_or(0.0);
            }
            if let Some(slot) = out.get_mut(x * channels + channel) {
                *slot = acc;
            }
        }
    }
}

/// The original's window at `t` in `0..=1`: `(1 − t)³·(1 + 3t − 3t² −
/// 13.6t³)`, with the arithmetic in f32 in the instruction order.
pub fn window_weight(t: f32) -> f32 {
    let one_minus = 1.0 - t;
    let six_t = 6.0 * t;
    let six_t2 = six_t * t;
    let mut value = (six_t + one_minus) * one_minus;
    value += six_t2;
    value *= one_minus;
    value += -2.1 * six_t2 * t;
    let cube = one_minus * one_minus * one_minus;
    value * cube
}

/// One window evaluation for a tap `offset` output texels from the centre:
/// `t = min(|offset|·scale, radius)/2` with `maxss`/`minss` semantics (a
/// NaN distance takes the low bound).
pub fn window_tap(offset: f32, scale: f32, radius: f32) -> f32 {
    let distance = offset.abs() * scale;
    let clamped = if distance.is_nan() {
        WINDOW_EPSILON
    } else {
        distance.max(WINDOW_EPSILON).min(radius)
    };
    window_weight(clamped * 0.5)
}

/// The taps of the phase-shifted window `scale` output texels wide, with
/// `offset` in output texels, normalised. Returns the first tap's signed
/// index (relative to the window centre) and its weights.
pub fn window_kernel(scale: f32, offset: f32, radius: f32) -> (i64, Vec<f32>) {
    if scale <= 0.0 || !scale.is_finite() {
        return (0, Vec::new());
    }
    let half = (radius / scale).ceil() as i64;
    let mut weights = Vec::with_capacity((2 * half + 1) as usize);
    let mut sum = 0.0f32;
    for k in -half..=half {
        let weight = window_tap(k as f32 + offset, scale, radius);
        weights.push(weight);
        sum += weight;
    }
    let inverse = 1.0 / sum;
    for weight in &mut weights {
        *weight *= inverse;
    }
    (-half, weights)
}

/// The original's sharpen kernel: `2·ceil(radius)+1` taps, a centre of one
/// plus the negative side taps, and side tap `i` `−amount/(radius·(radius−i))`.
/// Used by the icon and cursor passes.
pub fn sharpen_kernel(amount: f32, radius: f32) -> Vec<f32> {
    let half = radius.ceil() as i64;
    let mut weights = vec![0.0f32; (2 * half + 1) as usize];
    let centre = half as usize;
    weights[centre] = 1.0;
    for i in 0..half {
        let weight = (radius / half as f32) * (-(amount / radius) / (half as f32 - i as f32));
        weights[i as usize] = weight;
        weights[(2 * half - i) as usize] = weight;
        weights[centre] -= weight * 2.0;
    }
    weights
}

/// Normalises a kernel to sum one, dividing each weight by the sum in f32
/// (the terrain material's kernel normaliser).
pub fn normalise_kernel(weights: &mut [f32]) {
    let Some(&first) = weights.first() else {
        return;
    };
    let mut sum = first;
    for &weight in &weights[1..] {
        sum += weight;
    }
    for weight in weights {
        *weight /= sum;
    }
}

/// The bytes of one texel of an uncompressed format word, as the
/// original's bit count rounds it up to a byte.
pub fn bytes_per_pixel(format_word: u32) -> usize {
    bits_per_pixel(format_word).div_ceil(8) as usize
}

/// Copies whole rows of `dst_width` texels from `src`, the first source row
/// at `first_row` and the first source byte at `byte_column` (the
/// original's sub-image copy). Returns whether the rectangles fit.
/// The first level `k` (1 through 13) at which a destination size scaled by
/// `2^k` reaches the source size on both axes, as the original's
/// power-of-two resample looks for it. `None` when the sizes are not a
/// power-of-two ratio.
pub fn power_of_two_level(
    src_width: usize,
    src_height: usize,
    dst_width: usize,
    dst_height: usize,
) -> Option<u32> {
    for level in 1..=KAISER_MAX_LEVEL {
        if dst_width.checked_shl(level)? == src_width
            && dst_height.checked_shl(level)? == src_height
        {
            return Some(level);
        }
    }
    None
}

/// Places a byte image on a new, zero-filled canvas of `canvas_width` ×
/// `canvas_height` at the signed offset `(x, y)`, clipping the part that
/// falls outside, as the original's canvas copy does for the icon atlas.
/// Returns `None` when the input is too short for its size or the sizes
/// overflow.
#[allow(clippy::too_many_arguments)]
pub fn place_on_canvas(
    src: &[u8],
    src_width: usize,
    src_height: usize,
    format_word: u32,
    canvas_width: usize,
    canvas_height: usize,
    x: i32,
    y: i32,
) -> Option<Vec<u8>> {
    let bpp = bytes_per_pixel(format_word);
    let source_bytes = src_width.checked_mul(src_height)?.checked_mul(bpp)?;
    let canvas_bytes = canvas_width.checked_mul(canvas_height)?.checked_mul(bpp)?;
    if src.len() < source_bytes {
        return None;
    }
    let mut canvas = vec![0u8; canvas_bytes];
    let source_x = x.saturating_neg().max(0) as usize;
    let dest_x = x.max(0) as usize;
    let source_y = y.saturating_neg().max(0) as usize;
    let dest_y = y.max(0) as usize;
    let columns = src_width
        .saturating_sub(source_x)
        .min(canvas_width.saturating_sub(dest_x));
    let rows = src_height
        .saturating_sub(source_y)
        .min(canvas_height.saturating_sub(dest_y));
    let stride = columns * bpp;
    for row in 0..rows {
        let from = ((source_y + row) * src_width + source_x) * bpp;
        let to = ((dest_y + row) * canvas_width + dest_x) * bpp;
        canvas[to..to + stride].copy_from_slice(&src[from..from + stride]);
    }
    Some(canvas)
}

/// Normalises the first three floats of every RGBA texel to unit length, as
/// the original does to a filtered level of an image whose flag `0x4000000`
/// asks for it (normal maps). A zero-length vector is left alone; the
/// original divides it to NaN.
pub fn renormalise_texels(texels: &mut [f32]) {
    for texel in texels.as_chunks_mut::<4>().0 {
        let length = (texel[0] * texel[0] + texel[1] * texel[1] + texel[2] * texel[2]).sqrt();
        if length > 0.0 {
            texel[0] /= length;
            texel[1] /= length;
            texel[2] /= length;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn blit_rows(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    src: &[u8],
    src_width: usize,
    format_word: u32,
    byte_column: usize,
    first_row: usize,
) -> bool {
    let bpp = bytes_per_pixel(format_word);
    let stride = dst_width.saturating_mul(bpp);
    let Some(bytes) = stride.checked_mul(dst_height) else {
        return false;
    };
    if dst.len() < bytes {
        return false;
    }
    for row in 0..dst_height {
        let source = byte_column + src_width.saturating_mul(row + first_row) * bpp;
        let target = row * stride;
        let (Some(input), Some(output)) = (
            src.get(source..source.saturating_add(stride)),
            dst.get_mut(target..target.saturating_add(stride)),
        ) else {
            return false;
        };
        output.copy_from_slice(input);
    }
    true
}

/// Copies `src_width` texels of every source row into `dst` at
/// `byte_column` and `first_row` (the original's paste into a canvas).
/// Returns whether the rectangles fit.
#[allow(clippy::too_many_arguments)]
pub fn blit_into(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    src: &[u8],
    src_width: usize,
    src_height: usize,
    format_word: u32,
    byte_column: usize,
    first_row: usize,
) -> bool {
    let bpp = bytes_per_pixel(format_word);
    let stride = src_width.saturating_mul(bpp);
    if dst.len() < dst_width.saturating_mul(dst_height).saturating_mul(bpp)
        || src.len() < stride.saturating_mul(src_height)
    {
        return false;
    }
    for row in 0..src_height {
        let target = byte_column + dst_width.saturating_mul(row + first_row) * bpp;
        let (Some(input), Some(output)) = (
            src.get(row * stride..(row + 1) * stride),
            dst.get_mut(target..target.saturating_add(stride)),
        ) else {
            return false;
        };
        output.copy_from_slice(input);
    }
    true
}

/// The bits of one texel: the colour bits at 6..12 plus the alpha bits.
pub fn bits_per_pixel(format_word: u32) -> u32 {
    ((format_word & 0x1FC0) >> 6) + (format_word & 0x3F)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bessel_and_sinc_series_match_their_values() {
        assert!((bessel_i0(0.0) - 1.0).abs() < 1e-12);
        assert!((bessel_i0(1.0) - 1.266_065_877_752_008_4).abs() < 1e-12);
        assert!((sinc(0.0) - 1.0).abs() < 1e-12);
        assert!((sinc(1e-6) - 0.999_999_999_999_833_3).abs() < 1e-12);
        assert!((sinc(std::f64::consts::PI) - 3.898_171_5e-17).abs() < 1e-18);
    }

    #[test]
    fn the_kaiser_table_is_normalised_and_symmetric() {
        let table = kaiser_table(1);
        assert_eq!(table.len(), 8);
        let sum: f32 = table.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        for index in 0..4 {
            assert!((table[index] - table[7 - index]).abs() < 1e-6);
        }
        assert!(table[3] > table[0]);
        // The level-2 table doubles its taps and halves its scale.
        assert_eq!(kaiser_table(2).len(), 16);
        assert_eq!(kaiser_table(13).len(), 8 << 12);
    }

    #[test]
    fn the_window_has_the_original_values() {
        assert_eq!(window_weight(0.0), 1.0);
        assert_eq!(window_weight(0.5), 0.006_250_009);
        assert_eq!(window_weight(1.0), 0.0);
        assert_eq!(window_weight(1.5), 5.893_749_7);
        // The window clamp turns a NaN distance into the low bound.
        assert_eq!(window_tap(0.0, 1.0, 2.0), window_weight(0.5 * 1e-6));
        // A symmetric phase with scale one has five taps.
        let (start, weights) = window_kernel(1.0, 0.5, 2.0);
        assert_eq!(start, -2);
        assert_eq!(weights.len(), 5);
        let sum: f32 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        // With the half-texel phase the taps mirror about the centre between
        // taps one and two.
        assert!((weights[0] - weights[3]).abs() < 1e-6);
        assert!((weights[1] - weights[2]).abs() < 1e-6);
    }

    #[test]
    fn the_sharpen_kernel_is_one_plus_its_negative_lobes() {
        assert_eq!(
            sharpen_kernel(0.5, 2.0),
            [-0.125, -0.25, 1.75, -0.25, -0.125]
        );
        assert_eq!(sharpen_kernel(0.0, 1.0), [0.0, 1.0, 0.0]);
        let sum: f32 = sharpen_kernel(0.5, 3.0).iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_kernel_normaliser_divides_by_its_sum() {
        let mut kernel = [1.0, 2.0, 3.0, 4.0];
        normalise_kernel(&mut kernel);
        assert!((kernel.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!((kernel[3] - 0.4).abs() < 1e-6);
    }

    #[test]
    fn bytes_per_pixel_follows_the_format_word() {
        use exa_formats::pixel::id;
        assert_eq!(bytes_per_pixel(id::LUM8), 1);
        assert_eq!(bytes_per_pixel(id::RGB8), 3);
        assert_eq!(bytes_per_pixel(id::RGBA8), 4);
        assert_eq!(bytes_per_pixel(id::XY8), 2);
        assert_eq!(bytes_per_pixel(id::LUM1), 1);
        assert_eq!(bytes_per_pixel(id::RGBAF), 16);
    }

    #[test]
    fn a_canvas_places_the_image_and_clips_it() {
        use exa_formats::pixel::id;
        let src: Vec<u8> = [1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4].to_vec();
        let canvas = place_on_canvas(&src, 2, 2, id::RGB8, 4, 3, 1, 1).unwrap();
        assert_eq!(canvas.len(), 4 * 3 * 3);
        let at = |x: usize, y: usize| canvas[(y * 4 + x) * 3];
        assert_eq!((at(0, 0), at(1, 1), at(2, 2)), (0, 1, 4));
        // A negative offset crops the left column and the top row.
        let canvas = place_on_canvas(&src, 2, 2, id::RGB8, 2, 2, -1, -1).unwrap();
        assert_eq!(canvas, [4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        // A canvas past the source is zero there.
        let canvas = place_on_canvas(&src, 2, 2, id::RGB8, 4, 4, 2, 2).unwrap();
        assert_eq!(&canvas[(2 * 4 + 2) * 3..(2 * 4 + 3) * 3], &[1, 1, 1]);
        assert!(place_on_canvas(&[0; 4], 2, 2, id::RGB8, 4, 4, 0, 0).is_none());
    }

    #[test]
    fn power_of_two_resampling_finds_its_level() {
        assert_eq!(power_of_two_level(8, 8, 4, 4), Some(1));
        assert_eq!(power_of_two_level(8, 8, 2, 2), Some(2));
        assert_eq!(power_of_two_level(32, 16, 4, 2), Some(3));
        assert_eq!(power_of_two_level(8, 4, 2, 2), None);
        assert_eq!(power_of_two_level(8, 8, 3, 3), None);
    }

    #[test]
    fn sub_image_copies_move_whole_rows() {
        use exa_formats::pixel::id;
        // A 2x2 RGB source (3 bytes a texel): rows 10 11 / 20 21.
        let src: Vec<u8> = [10, 10, 10, 11, 11, 11, 20, 20, 20, 21, 21, 21].to_vec();
        let mut dst = [0u8; 2 * 2 * 3];
        assert!(blit_rows(&mut dst, 2, 2, &src, 2, id::RGB8, 0, 0));
        assert_eq!(dst, src[..]);
        // Pasting a 1x2 block at byte column 3, row 1.
        let strip: Vec<u8> = [10, 10, 10, 20, 20, 20].to_vec();
        let mut canvas = [0u8; 3 * 3 * 3];
        assert!(blit_into(&mut canvas, 3, 3, &strip, 1, 2, id::RGB8, 3, 1));
        assert_eq!(
            canvas,
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 10, 10, 0, 0, 0, 0, 0, 0, 20, 20, 20, 0, 0,
                0
            ]
        );
        // A rectangle past the end is refused, not written out of bounds.
        assert!(!blit_rows(&mut dst, 2, 2, &src, 2, id::RGB8, 1, 0));
    }

    #[test]
    fn the_axis_pass_averages_and_clamps() {
        let src = [1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0];
        let mut out = [0.0f32; 4];
        resample_axis(&src, 2, 1, &[0.5, 0.5], false, &mut out);
        assert_eq!(out, [0.5, 0.5, 0.0, 1.0]);

        let mut clamped = [0.0f32; 8];
        resample_axis(&src, 2, 2, &[0.25; 4], false, &mut clamped);
        assert!((clamped[0] - 0.5).abs() < 1e-6);
        assert!((clamped[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn wrapping_uses_the_other_end() {
        let src = [
            1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0,
        ];
        let mut out = [0.0f32; 8];
        resample_axis(&src, 4, 2, &[0.25; 4], true, &mut out);
        for channel in 0..3 {
            assert!((out[channel] - 0.25).abs() < 1e-6);
            assert!((out[4 + channel] - 0.25).abs() < 1e-6);
        }
        assert_eq!(out[3], 1.0);
        assert_eq!(out[7], 1.0);
    }
}

/// One generated level: its size and interleaved RGBA floats.
#[derive(Debug, Clone, PartialEq)]
pub struct MipLevel {
    pub width: usize,
    pub height: usize,
    pub texels: Vec<f32>,
}

/// Builds the mip chain from the base level: levels `1..=max_levels` while
/// each level follows the previous one (a level whose two sides sum to two
/// is the last). Each level is resampled directly from the base with its own
/// Kaiser table, first horizontally then vertically.
pub fn build_mip_chain(
    base: &[f32],
    width: usize,
    height: usize,
    wrap: bool,
    max_levels: u32,
) -> Vec<MipLevel> {
    let mut levels = Vec::new();
    if width == 0 || height == 0 || base.len() < width * height * 4 {
        return levels;
    }
    for level in 1..=max_levels.min(KAISER_MAX_LEVEL) {
        let dst_width = (width >> level).max(1);
        let dst_height = (height >> level).max(1);
        let weights = &kaiser_bank()[level as usize - 1];
        let mut horizontal = vec![0.0f32; dst_width * height * 4];
        let mut row = vec![0.0f32; dst_width * 4];
        for y in 0..height {
            let start = y * width * 4;
            let end = start + width * 4;
            resample_axis(&base[start..end], width, dst_width, weights, wrap, &mut row);
            horizontal[y * dst_width * 4..(y + 1) * dst_width * 4].copy_from_slice(&row);
        }
        let mut texels = vec![0.0f32; dst_width * dst_height * 4];
        let mut column = vec![0.0f32; height * 4];
        let mut output = vec![0.0f32; dst_height * 4];
        for x in 0..dst_width {
            for y in 0..height {
                let source = (y * dst_width + x) * 4;
                column[y * 4..y * 4 + 4].copy_from_slice(&horizontal[source..source + 4]);
            }
            resample_axis(&column, height, dst_height, weights, wrap, &mut output);
            for y in 0..dst_height {
                let destination = (y * dst_width + x) * 4;
                texels[destination..destination + 4].copy_from_slice(&output[y * 4..y * 4 + 4]);
            }
        }
        levels.push(MipLevel {
            width: dst_width,
            height: dst_height,
            texels,
        });
        if dst_width + dst_height <= 2 {
            break;
        }
    }
    levels
}

#[cfg(test)]
mod chain_tests {
    use super::*;

    #[test]
    fn a_uniform_image_stays_uniform() {
        let base = vec![0.4f32; 8 * 8 * 4];
        let levels = build_mip_chain(&base, 8, 8, false, 3);
        // 4x4, 2x2 and 1x1: a level whose sides sum to two is the last.
        assert_eq!(levels.len(), 3);
        assert_eq!((levels[0].width, levels[0].height), (4, 4));
        assert_eq!((levels[1].width, levels[1].height), (2, 2));
        assert_eq!((levels[2].width, levels[2].height), (1, 1));
        for level in levels {
            for value in level.texels {
                assert!((value - 0.4).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn a_non_power_of_two_side_still_gets_a_chain() {
        // Every level shifts right with a floor of one; the chain exists and
        // ends on 1x1 even for a 3x3 base.
        let base = vec![0.25f32; 3 * 3 * 4];
        let levels = build_mip_chain(&base, 3, 3, true, 8);
        assert_eq!(levels.len(), 1);
        assert_eq!((levels[0].width, levels[0].height), (1, 1));
        // 6x2: 3x1 then 1x1.
        let base = vec![0.5f32; 6 * 2 * 4];
        let levels = build_mip_chain(&base, 6, 2, true, 8);
        assert_eq!(levels.len(), 2);
        assert_eq!((levels[0].width, levels[0].height), (3, 1));
        assert_eq!((levels[1].width, levels[1].height), (1, 1));
    }
}

/// Builds the chain for an RGBA8 base: the base bytes followed by each
/// generated level, and the total level count (base included). The base is
/// turned into floats by `conversion` (linear, squared or centred, as the
/// original picks per format), each level is filtered from those floats and
/// turned back into bytes the same way. The horizontal pass is split across
/// rows for large images.
pub fn build_mip_chain_rgba8(
    base: &[u8],
    width: usize,
    height: usize,
    wrap: bool,
    conversion: MipConversion,
) -> (Vec<u8>, u32) {
    build_mip_chain_rgba8_from(base, width, height, 0, wrap, conversion)
}

/// Builds an RGBA8 chain starting at `first_level`: the texels are the level
/// `first_level` of the Kaiser chain and the levels above it, packed as a
/// texture's base level onwards, and the count is how many levels that is.
/// A low `TextureDetail` uploads a later level as the base and halves the
/// stored resolution.
pub fn build_mip_chain_rgba8_from(
    base: &[u8],
    width: usize,
    height: usize,
    first_level: u32,
    wrap: bool,
    conversion: MipConversion,
) -> (Vec<u8>, u32) {
    build_mip_chain_rgba8_between(
        base,
        width,
        height,
        first_level,
        KAISER_MAX_LEVEL,
        wrap,
        conversion,
    )
}

/// Builds an RGBA8 chain of the levels `first_level..=last_level`. The first
/// level is uploaded as the texture's base; `last_level` bounds the chain
/// (the texture's mip filter, or the first level itself when mipmapping is
/// off). The chain also stops after a level whose sides sum to two.
pub fn build_mip_chain_rgba8_between(
    base: &[u8],
    width: usize,
    height: usize,
    first_level: u32,
    last_level: u32,
    wrap: bool,
    conversion: MipConversion,
) -> (Vec<u8>, u32) {
    build_mip_chain_rgba8_between_flagged(
        base,
        width,
        height,
        first_level,
        last_level,
        wrap,
        conversion,
        false,
    )
}

/// [`build_mip_chain_rgba8_between`] with the image's normal-map flag: every
/// filtered level has each texel's first three channels normalised before it
/// is converted back to bytes (flag `0x4000000`).
#[allow(clippy::too_many_arguments)]
pub fn build_mip_chain_rgba8_between_flagged(
    base: &[u8],
    width: usize,
    height: usize,
    first_level: u32,
    last_level: u32,
    wrap: bool,
    conversion: MipConversion,
    renormalise: bool,
) -> (Vec<u8>, u32) {
    if width == 0 || height == 0 || base.len() < width * height * 4 {
        return (base.to_vec(), 1);
    }
    let (chain, count) = build_chain_from_floats(
        base,
        width,
        height,
        first_level,
        last_level,
        wrap,
        conversion,
        renormalise,
    );
    if count == 0 {
        // A chain that stops before `first_level` (a tiny image): the caller
        // has nothing to upload. The retail data never asks for this.
        return (Vec::new(), 0);
    }
    (chain, count)
}

/// The chain levels from `first_level` up, without the levels before it.
#[allow(clippy::too_many_arguments)]
fn build_chain_from_floats(
    base: &[u8],
    width: usize,
    height: usize,
    first_level: u32,
    last_level: u32,
    wrap: bool,
    conversion: MipConversion,
    renormalise: bool,
) -> (Vec<u8>, u32) {
    let first = first_level.max(1);
    let mut floats = vec![0.0f32; width * height * 4];
    let base_bytes = &base[..width * height * 4];
    match conversion {
        MipConversion::Linear => pixel::unit_floats(base_bytes, &mut floats),
        MipConversion::Gamma => pixel::gamma_floats(base_bytes, &mut floats),
        MipConversion::GammaColour => pixel::gamma_colour_floats(base_bytes, &mut floats),
        MipConversion::Signed => pixel::signed_floats(base_bytes, &mut floats),
    }
    let floats = floats.as_slice();
    // The chain's levels, base bytes included when `first_level` is 0.
    let mut out: Vec<u8> = if first_level == 0 {
        base.to_vec()
    } else {
        Vec::new()
    };
    let mut count = if first_level == 0 { 1u32 } else { 0u32 };
    let bank = kaiser_bank();
    for level in first..=last_level.min(KAISER_MAX_LEVEL) {
        let dst_width = (width >> level).max(1);
        let dst_height = (height >> level).max(1);
        let weights = &bank[level as usize - 1];
        let mut horizontal = vec![0.0f32; dst_width * height * 4];
        let threads = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1)
            .min((height / 8).max(1))
            .min(8);
        if threads > 1 && height * dst_width >= 8_192 {
            std::thread::scope(|scope| {
                let mut rest = horizontal.as_mut_slice();
                let per = height.div_ceil(threads);
                let mut start_row = 0;
                while start_row < height {
                    let rows = per.min(height - start_row);
                    let (chunk, tail) = rest.split_at_mut(rows * dst_width * 4);
                    rest = tail;
                    let weights_ref: &[f32] = weights;
                    scope.spawn(move || {
                        let mut row = vec![0.0f32; dst_width * 4];
                        for row_index in 0..rows {
                            let y = start_row + row_index;
                            resample_axis(
                                &floats[y * width * 4..(y + 1) * width * 4],
                                width,
                                dst_width,
                                weights_ref,
                                wrap,
                                &mut row,
                            );
                            chunk[row_index * dst_width * 4..(row_index + 1) * dst_width * 4]
                                .copy_from_slice(&row);
                        }
                    });
                    start_row += rows;
                }
            });
        } else {
            let mut row = vec![0.0f32; dst_width * 4];
            for y in 0..height {
                resample_axis(
                    &floats[y * width * 4..(y + 1) * width * 4],
                    width,
                    dst_width,
                    weights,
                    wrap,
                    &mut row,
                );
                horizontal[y * dst_width * 4..(y + 1) * dst_width * 4].copy_from_slice(&row);
            }
        }

        let mut texels = vec![0.0f32; dst_width * dst_height * 4];
        let vertical_threads = if threads > 1 && dst_width >= 64 {
            threads
        } else {
            1
        };
        if vertical_threads > 1 {
            let columns: Vec<(usize, Vec<f32>)> = std::thread::scope(|scope| {
                let per = dst_width.div_ceil(vertical_threads);
                let mut handles = Vec::new();
                for column_start in (0..dst_width).step_by(per) {
                    let column_end = (column_start + per).min(dst_width);
                    let horizontal_ref = &horizontal;
                    let weights_ref: &[f32] = weights;
                    handles.push(scope.spawn(move || {
                        let mut column = vec![0.0f32; height * 4];
                        let mut output = vec![0.0f32; dst_height * 4];
                        let mut results = Vec::with_capacity(column_end - column_start);
                        for x in column_start..column_end {
                            for y in 0..height {
                                let source = (y * dst_width + x) * 4;
                                column[y * 4..y * 4 + 4]
                                    .copy_from_slice(&horizontal_ref[source..source + 4]);
                            }
                            resample_axis(
                                &column,
                                height,
                                dst_height,
                                weights_ref,
                                wrap,
                                &mut output,
                            );
                            results.push((x, output.clone()));
                        }
                        results
                    }));
                }
                handles
                    .into_iter()
                    .flat_map(|handle| handle.join().expect("mip column"))
                    .collect()
            });
            for (x, data) in columns {
                for y in 0..dst_height {
                    let destination = (y * dst_width + x) * 4;
                    texels[destination..destination + 4].copy_from_slice(&data[y * 4..y * 4 + 4]);
                }
            }
        } else {
            let mut column = vec![0.0f32; height * 4];
            let mut output = vec![0.0f32; dst_height * 4];
            for x in 0..dst_width {
                for y in 0..height {
                    let source = (y * dst_width + x) * 4;
                    column[y * 4..y * 4 + 4].copy_from_slice(&horizontal[source..source + 4]);
                }
                resample_axis(&column, height, dst_height, weights, wrap, &mut output);
                for y in 0..dst_height {
                    let destination = (y * dst_width + x) * 4;
                    texels[destination..destination + 4].copy_from_slice(&output[y * 4..y * 4 + 4]);
                }
            }
        }
        let mut level = vec![0u8; texels.len()];
        if renormalise {
            renormalise_texels(&mut texels);
        }
        match conversion {
            MipConversion::Linear => pixel::unit_bytes(&texels, &mut level),
            MipConversion::Gamma => pixel::gamma_bytes(&texels, &mut level),
            MipConversion::GammaColour => pixel::gamma_colour_bytes(&texels, &mut level),
            MipConversion::Signed => pixel::signed_bytes(&texels, &mut level),
        }
        out.extend(level);
        count += 1;
        if dst_width + dst_height <= 2 {
            break;
        }
    }
    (out, count)
}

/// Resamples an RGBA8 image to a new size with the windowed kernel, in the
/// space the format word picks. The result is the converted destination
/// level. This is the original's resample-to-size path (icons, cursors,
/// terrain and GUI images), not the mip chain.
#[allow(clippy::too_many_arguments)]
pub fn resample_rgba8_to_size(
    base: &[u8],
    width: usize,
    height: usize,
    dst_width: usize,
    dst_height: usize,
    wrap: bool,
    conversion: MipConversion,
    renormalise: bool,
) -> Vec<u8> {
    if width == 0 || height == 0 || dst_width == 0 || dst_height == 0 {
        return Vec::new();
    }
    let usable = width.saturating_mul(height).saturating_mul(4);
    if base.len() < usable {
        return Vec::new();
    }
    let mut floats = vec![0.0f32; usable];
    let base_bytes = &base[..usable];
    match conversion {
        MipConversion::Linear => pixel::unit_floats(base_bytes, &mut floats),
        MipConversion::Gamma => pixel::gamma_floats(base_bytes, &mut floats),
        MipConversion::GammaColour => pixel::gamma_colour_floats(base_bytes, &mut floats),
        MipConversion::Signed => pixel::signed_floats(base_bytes, &mut floats),
    }
    let mut horizontal = vec![0.0f32; dst_width * height * 4];
    let mut row = vec![0.0f32; dst_width * 4];
    for y in 0..height {
        resample_axis_phased(
            &floats[y * width * 4..(y + 1) * width * 4],
            width,
            dst_width,
            wrap,
            &mut row,
        );
        horizontal[y * dst_width * 4..(y + 1) * dst_width * 4].copy_from_slice(&row);
    }
    let mut texels = vec![0.0f32; dst_width * dst_height * 4];
    let mut column = vec![0.0f32; height * 4];
    let mut output = vec![0.0f32; dst_height * 4];
    for x in 0..dst_width {
        for y in 0..height {
            let source = (y * dst_width + x) * 4;
            column[y * 4..y * 4 + 4].copy_from_slice(&horizontal[source..source + 4]);
        }
        resample_axis_phased(&column, height, dst_height, wrap, &mut output);
        for y in 0..dst_height {
            let destination = (y * dst_width + x) * 4;
            texels[destination..destination + 4].copy_from_slice(&output[y * 4..y * 4 + 4]);
        }
    }
    let mut out = vec![0u8; texels.len()];
    if renormalise {
        renormalise_texels(&mut texels);
    }
    match conversion {
        MipConversion::Linear => pixel::unit_bytes(&texels, &mut out),
        MipConversion::Gamma => pixel::gamma_bytes(&texels, &mut out),
        MipConversion::GammaColour => pixel::gamma_colour_bytes(&texels, &mut out),
        MipConversion::Signed => pixel::signed_bytes(&texels, &mut out),
    }
    out
}

/// One axis pass with the phase-shifted window: output texel `i` is centred
/// on source position `i·src/dst` (the fractional part goes into the
/// window's phase), and its taps span two output texels each way.
pub fn resample_axis_phased(
    src: &[f32],
    src_extent: usize,
    dst_extent: usize,
    wrap: bool,
    out: &mut [f32],
) {
    if src_extent == 0 || dst_extent == 0 || out.len() < dst_extent * 4 {
        return;
    }
    let ratio = src_extent as f64 / dst_extent as f64;
    let ratio_f = ratio as f32;
    let scale = ((dst_extent as f64 / src_extent as f64) as f32).min(1.0);
    // The original's base offset: half an output texel, or half a source
    // texel when the pass magnifies.
    let base = if ratio_f >= 1.0 { 0.5 } else { ratio_f * 0.5 };
    let integer_ratio = (src_extent / dst_extent).max(1) as i64;
    for x in 0..dst_extent {
        let position = x as f32 * ratio_f;
        let whole = position.trunc();
        let fraction = position - whole;
        let offset = base - fraction;
        let (_, weights) = window_kernel(scale, offset, WINDOW_RADIUS);
        let first = whole as i64 - ((weights.len() as i64 - integer_ratio) / 2);
        for channel in 0..4 {
            let mut acc = 0.0f32;
            for (k, &weight) in weights.iter().enumerate() {
                let mut source = first + k as i64;
                source = if wrap {
                    source.rem_euclid(src_extent as i64)
                } else {
                    source.clamp(0, src_extent as i64 - 1)
                };
                acc += weight
                    * src
                        .get(source as usize * 4 + channel)
                        .copied()
                        .unwrap_or(0.0);
            }
            out[x * 4 + channel] = acc;
        }
    }
}

#[cfg(test)]
mod rgba_chain_tests {
    use super::*;

    #[test]
    fn an_rgba_chain_appends_its_levels() {
        let base = vec![51u8; 4 * 4 * 4];
        for conversion in [
            MipConversion::Linear,
            MipConversion::Gamma,
            MipConversion::GammaColour,
            MipConversion::Signed,
        ] {
            // 2x2 and 1x1, after the stored base.
            let (out, count) = build_mip_chain_rgba8(&base, 4, 4, true, conversion);
            assert_eq!(count, 3);
            assert_eq!(out.len(), 4 * 4 * 4 + 2 * 2 * 4 + 4);
            assert!(out.iter().all(|&value| value == 51), "{conversion:?}");
        }

        // A 1x1 image still gets its single generated level.
        let (short, count) = build_mip_chain_rgba8(&[7; 4], 1, 1, true, MipConversion::Linear);
        assert_eq!(count, 2);
        assert_eq!(short, vec![7; 8]);
    }

    /// A later first level is uploaded as the base and halves the chain, as
    /// the texture detail setting does; each level is still filtered from
    /// the stored base.
    #[test]
    fn a_chain_can_start_at_a_later_level() {
        let base = vec![51u8; 8 * 8 * 4];
        // Level 1: 4x4; level 2: 2x2; level 3: 1x1.
        let (out, count) = build_mip_chain_rgba8_from(&base, 8, 8, 1, true, MipConversion::Linear);
        assert_eq!(count, 3);
        assert_eq!(out.len(), 4 * 4 * 4 + 2 * 2 * 4 + 4);
        assert!(out.iter().all(|&value| value == 51));

        let (out, count) = build_mip_chain_rgba8_from(&base, 8, 8, 2, true, MipConversion::Linear);
        assert_eq!(count, 2);
        assert_eq!(out.len(), 2 * 2 * 4 + 4);
        assert!(out.iter().all(|&value| value == 51));

        // A first level past the chain's own 1x1 still makes a 1x1 level:
        // the sides floor at one and the original filters from there.
        let (out, count) = build_mip_chain_rgba8_from(&base, 8, 8, 4, true, MipConversion::Linear);
        assert_eq!(count, 1);
        assert_eq!(out.len(), 4);
        // Past the last table there is nothing to upload.
        let (out, count) = build_mip_chain_rgba8_from(&base, 8, 8, 14, true, MipConversion::Linear);
        assert_eq!(count, 0);
        assert!(out.is_empty());
        // Which image stores which level is a format question.
        assert_eq!(exa_formats::rfi::level_dims(8, 8, 2), (2, 2));
    }

    /// Without mipmapping, a skipped first level is uploaded alone: the
    /// original generates levels `s..s`, not a chain.
    #[test]
    fn a_chain_can_stop_at_its_first_level() {
        let base = vec![51u8; 8 * 8 * 4];
        let (out, count) =
            build_mip_chain_rgba8_between(&base, 8, 8, 2, 2, true, MipConversion::Linear);
        assert_eq!(count, 1);
        assert_eq!(out.len(), 2 * 2 * 4);
        let (out, count) =
            build_mip_chain_rgba8_between(&base, 8, 8, 1, 1, true, MipConversion::Linear);
        assert_eq!(count, 1);
        assert_eq!(out.len(), 4 * 4 * 4);
    }

    /// A 4 × 4 checker of 0 and 255 halves to mid-grey: 128 when filtered
    /// linearly, √½ · 255 = 180 when squared first. Alpha stays linear.
    #[test]
    fn colour_is_filtered_squared() {
        let mut base = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                let v = if (x + y) % 2 == 0 { 255 } else { 0 };
                base.extend_from_slice(&[v, v, v, v]);
            }
        }
        let level = |conversion| {
            let (out, _) = build_mip_chain_rgba8(&base, 4, 4, true, conversion);
            out[64..68].to_vec()
        };
        let linear = level(MipConversion::Linear);
        let gamma = level(MipConversion::GammaColour);
        assert!(
            linear.iter().all(|&v| (127..=128).contains(&v)),
            "{linear:?}"
        );
        assert!((179..=181).contains(&gamma[0]), "{gamma:?}");
        assert_eq!(gamma[3], linear[3]);
        assert!((179..=181).contains(&level(MipConversion::Gamma)[3]));
    }

    /// The normal-map flag normalises every filtered level's texels before
    /// the bytes are written: a signed (1, 1, −1) filters to unit
    /// (0.577, 0.577, −0.577), bytes 201, 201, 54, instead of clamping.
    #[test]
    fn a_normal_map_level_is_renormalised() {
        let base = [255u8, 255, 0, 255].repeat(4 * 4);
        let (out, _) = build_mip_chain_rgba8_between_flagged(
            &base,
            4,
            4,
            0,
            1,
            true,
            MipConversion::Signed,
            true,
        );
        assert_eq!(&out[64..68], &[201, 201, 54, 255]);
        let (plain, _) =
            build_mip_chain_rgba8_between(&base, 4, 4, 0, 1, true, MipConversion::Signed);
        assert_eq!(&plain[64..68], &[255, 255, 0, 255]);
    }

    /// A non-power-of-two image gets a chain too; the level dimensions shift
    /// right with a floor of one.
    #[test]
    fn a_non_power_of_two_image_is_mipped() {
        let base = vec![90u8; 6 * 3 * 4];
        let (out, count) = build_mip_chain_rgba8(&base, 6, 3, true, MipConversion::Linear);
        // 3x1, then 1x1.
        assert_eq!(count, 3);
        assert_eq!(out.len(), 6 * 3 * 4 + 3 * 4 + 4);
        assert!(out.iter().all(|&value| value == 90));
    }

    /// A constant image resampled to another size keeps its value, whatever
    /// the phase of the window.
    #[test]
    fn resampling_to_a_new_size_keeps_a_constant() {
        let base = vec![200u8; 8 * 8 * 4];
        let out = resample_rgba8_to_size(&base, 8, 8, 5, 3, true, MipConversion::Linear, false);
        assert_eq!(out.len(), 5 * 3 * 4);
        assert!(
            out.iter().all(|&value| (199..=200).contains(&value)),
            "{out:?}"
        );
        let out = resample_rgba8_to_size(&base, 8, 8, 13, 11, false, MipConversion::Linear, false);
        assert_eq!(out.len(), 13 * 11 * 4);
        assert!(out.iter().all(|&value| (199..=200).contains(&value)));
    }

    /// The phased axis pass reproduces the window: a single bright texel in
    /// a 4-wide row resampled to 2 has its energy in the taps the window
    /// puts around each output centre.
    #[test]
    fn the_phased_pass_uses_the_window() {
        let src = [
            255.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];
        let mut out = [0.0f32; 8];
        resample_axis_phased(&src, 4, 2, false, &mut out);
        // Both outputs are symmetric around the bright texel: the first
        // keeps exactly half of its energy, the second only the window's
        // negative lobe.
        assert!((out[0] - 127.5).abs() < 1e-3, "{out:?}");
        assert!((out[4] + 12.657_432).abs() < 1e-3, "{out:?}");
    }
}
