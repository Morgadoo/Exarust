//! The original's image resampling unit (`RFP_Mipmaps_Kaiser`): its Kaiser
//! and uniform banks for mip chains, its windowed kernel for resampling to
//! an arbitrary size, its separable axis passes, blits and its sharpen
//! kernel.
//!
//! **Mip chains** ([`build_mip_chain_rgba8_with`]) filter colour images with
//! a bank of 13 normalised Kaiser kernels, one per level. Level `L` has
//! `n = 8·2^(L−1)` taps, `h = n/2` and `c = 2^−L`. Tap `i` uses
//! `u = i − h + 0.5`, `t = u/h` and
//! `w = I0(4·√(1 − t²))/I0(4) · sinc(π·c·u)`, or 0 outside `|t| ≤ 1`; the
//! weights are summed in f64 and each divided by that sum. Data images
//! (format bit 16, not compressed) use the uniform bank instead: `2^L` taps
//! of `2^−L`. Each level is filtered directly from the stored base level,
//! horizontally and then vertically, with its own table. The axis pass
//! takes `ratio = ⌊src/dst⌋` and starts at `−((n − ratio) ≫ 1)`, wrapping
//! or clamping every tap by the image's own flags (`0x10000` across,
//! `0x20000` along the height).
//!
//! **Resampling to an arbitrary size** ([`resample_rgba8_to_size`]) takes
//! the chain's level when the sizes are a power-of-two ratio, and otherwise
//! a phase-shifted window: for every output texel the taps are centred on
//! `i·src/dst` and reach two output texels each way, with
//! `t = min(max(|tap|·scale, 1e-6), 2)/2`, `scale = min(dst/src, 1)` and
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
//! builders, both banks, the axis passes, the chain loop and the texture
//! loader's call into it; REA). Rules: `docs/formats/RFI.md` (Generated mip
//! levels).

use exa_formats::pixel::{self, MipConversion};

/// The Kaiser window's beta.
pub const KAISER_BETA: f64 = 4.0;
/// The tap base of level 1.
pub const KAISER_BASE_TAPS: usize = 8;
/// The uniform bank's tap base of level 1.
pub const BOX_BASE_TAPS: usize = 2;
/// The largest table level the original builds.
pub const KAISER_MAX_LEVEL: u32 = 13;
/// The arbitrary-size window's radius, in output texels.
pub const WINDOW_RADIUS: f32 = 2.0;
/// The smallest distance the window clamp accepts.
pub const WINDOW_EPSILON: f32 = 1e-6;

/// The modified Bessel function of the first kind, order zero, by its
/// series: `Σ ((x/2)^m / m!)²`, in f64. The loop goes on while
/// `sum · 1e-16 < term²` (an ordered compare: a NaN stops it), the sum
/// already holding the newest term.
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
        // Ordered: continue only while the term is still larger.
        if (sum * 1e-16).partial_cmp(&squared) != Some(std::cmp::Ordering::Less) {
            return sum;
        }
        m += 1.0;
    }
}

/// `sin(x)/x`, with the series `1 − x²/6 + x⁴/120` under `1e-4` (a NaN
/// takes the sine). The quadratic coefficient is the original's
/// f32-rounded `−1/6`, and `x²` is formed once.
pub fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-4 {
        const MINUS_SIXTH: f64 = -0.166_666_671_633_720_4;
        let x2 = x * x;
        (x2 * 1.0 * 0.008_333_333_333_333_333 + MINUS_SIXTH) * x2 + 1.0
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

/// The 13 uniform tables, built with the Kaiser bank at start-up.
fn box_bank() -> &'static [Vec<f32>] {
    static BANK: std::sync::OnceLock<Vec<Vec<f32>>> = std::sync::OnceLock::new();
    BANK.get_or_init(|| (1..=KAISER_MAX_LEVEL).map(box_table).collect())
}

/// The uniform table of a mip level (1 through 13): `2^L` taps of
/// `1/2^L` each, a plain box over the texels the level halves.
pub fn box_table(level: u32) -> Vec<f32> {
    let level = level.clamp(1, KAISER_MAX_LEVEL);
    let taps = BOX_BASE_TAPS << (level - 1);
    vec![1.0 / taps as f32; taps]
}

/// Which bank a generated chain filters with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MipKernel {
    /// The windowed-sinc bank: colour images.
    Kaiser,
    /// The uniform bank: data images.
    Box,
}

impl MipKernel {
    /// The bank the original's chain picks from an image's format word: the
    /// uniform one when bit 16 (data) is set and bit 31 (compressed) is not
    /// (`VAL8`, `ABC8`, `ABCD8`, the signed `0x1111` words and the float
    /// data words), the Kaiser bank otherwise.
    pub fn for_format(format_word: u32) -> Self {
        if format_word & 0x8001_0000 == 0x0001_0000 {
            MipKernel::Box
        } else {
            MipKernel::Kaiser
        }
    }

    fn table(self, level: u32) -> &'static [f32] {
        let bank = match self {
            MipKernel::Kaiser => kaiser_bank(),
            MipKernel::Box => box_bank(),
        };
        &bank[level.clamp(1, KAISER_MAX_LEVEL) as usize - 1]
    }
}

/// The normalised Kaiser table of a mip level (1 through 13). Level 1 has
/// eight taps; each later level doubles them and halves the sinc's scale.
pub fn kaiser_table(level: u32) -> Vec<f32> {
    let level = level.clamp(1, KAISER_MAX_LEVEL);
    let taps = KAISER_BASE_TAPS << (level - 1);
    // The original computes the half width and the tap position in f32,
    // the window, the sinc and the weights' sum in f64, and stores each
    // weight as f32.
    let half = (taps as f32) * 0.5;
    let half64 = f64::from(half);
    // The original's bank computes `2^−L` as `exp(L·(−ln 2))`.
    let c = (level as f64 * -0.693_147_180_559_945_3).exp();
    let mut weights = vec![0.0f32; taps];
    let mut sum = 0.0f64;
    for (index, weight) in weights.iter_mut().enumerate() {
        let u = ((index as f32 - half) + 0.5) as f64;
        let t = u / half64;
        let radius = 1.0 - t * t;
        // An ordered compare: a negative (or NaN) radius leaves a zero tap.
        if radius >= 0.0 {
            let window = bessel_i0(radius.sqrt() * KAISER_BETA) / bessel_i0(KAISER_BETA);
            *weight = (sinc(u * std::f64::consts::PI * c) * window) as f32;
            sum += f64::from(*weight);
        }
    }
    // Each stored weight is widened and divided by the f64 sum.
    for weight in &mut weights {
        *weight = (f64::from(*weight) / sum) as f32;
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

/// The engine's approximate reciprocal square root (the processor's
/// estimate, about 12 bits).
fn reciprocal_sqrt(value: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: SSE is part of the x86_64 baseline.
        unsafe {
            use std::arch::x86_64::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
            _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(value)))
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1.0 / value.sqrt()
    }
}

/// The engine's in-place vector normalisation: the squares summed x, z then
/// y, scaled by the approximate reciprocal square root; a zero vector stays
/// zero.
fn normalise_vector(v: &mut [f32]) {
    let sum = (v[0] * v[0] + v[2] * v[2]) + v[1] * v[1];
    let scale = if sum == 0.0 {
        0.0
    } else {
        reciprocal_sqrt(sum)
    };
    v[0] *= scale;
    v[1] *= scale;
    v[2] *= scale;
}

/// Normalises the first three floats of every RGBA texel, as the original
/// does to a filtered level of a three- or four-channel image whose flag
/// `0x4000000` asks for it (normal maps): the engine's vector normaliser,
/// so a zero vector stays zero. Alpha is left alone.
pub fn renormalise_texels(texels: &mut [f32]) {
    for texel in texels.as_chunks_mut::<4>().0 {
        normalise_vector(texel);
    }
}

/// Whether the original renormalises an image's filtered levels: the image
/// flag `0x4000000` on a three- or four-channel word (`0xC600`, `0xE608`).
/// One- and two-channel images are never touched.
pub fn renormalises(format_word: u32, image_flags: u32) -> bool {
    image_flags & 0x0400_0000 != 0 && matches!(format_word & pixel::TARGET_MASK, 0xC600 | 0xE608)
}

/// Copies `dst_height` whole rows of `dst_width` texels out of `src`, row
/// `y` and texel `x` of the source first (the original's sub-image copy).
/// Returns whether the rectangles fit.
#[allow(clippy::too_many_arguments)]
pub fn blit_rows(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    src: &[u8],
    src_width: usize,
    format_word: u32,
    x: usize,
    y: usize,
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
        let source = src_width
            .saturating_mul(row + y)
            .saturating_add(x)
            .saturating_mul(bpp);
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

/// Pastes every row of `src` into `dst` at texel `x` of row `y` (the
/// original's paste into a canvas). Returns whether the rectangles fit.
#[allow(clippy::too_many_arguments)]
pub fn blit_into(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    src: &[u8],
    src_width: usize,
    src_height: usize,
    format_word: u32,
    x: usize,
    y: usize,
) -> bool {
    let bpp = bytes_per_pixel(format_word);
    let stride = src_width.saturating_mul(bpp);
    if dst.len() < dst_width.saturating_mul(dst_height).saturating_mul(bpp)
        || src.len() < stride.saturating_mul(src_height)
    {
        return false;
    }
    for row in 0..src_height {
        let target = dst_width
            .saturating_mul(row + y)
            .saturating_add(x)
            .saturating_mul(bpp);
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
        // The series' ordered compare stops on a NaN instead of looping.
        assert!(bessel_i0(f64::NAN).is_nan());
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
        // Pasting a 1x2 block at texel 1, row 1.
        let strip: Vec<u8> = [10, 10, 10, 20, 20, 20].to_vec();
        let mut canvas = [0u8; 3 * 3 * 3];
        assert!(blit_into(&mut canvas, 3, 3, &strip, 1, 2, id::RGB8, 1, 1));
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
/// off). The chain also stops after a level whose sides sum to two. This
/// form filters with the Kaiser bank and wraps on both axes; an image's own
/// rules come from [`ChainOptions::for_image`].
pub fn build_mip_chain_rgba8_between(
    base: &[u8],
    width: usize,
    height: usize,
    first_level: u32,
    last_level: u32,
    wrap: bool,
    conversion: MipConversion,
) -> (Vec<u8>, u32) {
    build_mip_chain_rgba8_with(
        base,
        width,
        height,
        &ChainOptions {
            first_level,
            last_level,
            wrap: [wrap; 2],
            conversion,
            kernel: MipKernel::Kaiser,
            renormalise: false,
        },
    )
}

/// How the original's chain driver filters one image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChainOptions {
    /// The first generated level, uploaded as the texture's base (0 keeps
    /// the stored base in front of level 1).
    pub first_level: u32,
    /// The last level the chain may reach.
    pub last_level: u32,
    /// Wrap (`true`) or clamp each tap, horizontally then vertically.
    pub wrap: [bool; 2],
    /// The filtering space.
    pub conversion: MipConversion,
    /// The bank the levels are filtered with.
    pub kernel: MipKernel,
    /// Normalise every filtered texel's first three channels.
    pub renormalise: bool,
}

impl ChainOptions {
    /// The options the original's chain driver reads from an image: the
    /// format word picks the filtering space and the bank, and the image's
    /// own flags (its header word) the wrap on each axis (`0x10000` across,
    /// `0x20000` along the height, clamping without them) and the
    /// renormalisation (`0x4000000`, three- and four-channel words only).
    pub fn for_image(
        format_word: u32,
        image_flags: u32,
        first_level: u32,
        last_level: u32,
    ) -> Self {
        ChainOptions {
            first_level,
            last_level,
            wrap: [
                image_flags & 0x0001_0000 != 0,
                image_flags & 0x0002_0000 != 0,
            ],
            conversion: pixel::mip_conversion(format_word).unwrap_or(MipConversion::Linear),
            kernel: MipKernel::for_format(format_word),
            renormalise: renormalises(format_word, image_flags),
        }
    }
}

/// Builds an RGBA8 chain with the given options: the levels from
/// `first_level` up to `last_level` or the level whose sides sum to two,
/// each filtered from the stored base, packed largest first, and the count.
/// With `first_level` 0 the stored base comes first. A chain that starts
/// past the last table (a tiny image skipped too far) has nothing to
/// upload: no bytes and a count of 0.
pub fn build_mip_chain_rgba8_with(
    base: &[u8],
    width: usize,
    height: usize,
    options: &ChainOptions,
) -> (Vec<u8>, u32) {
    if width == 0 || height == 0 || base.len() < width * height * 4 {
        return (base.to_vec(), 1);
    }
    build_chain_from_floats(base, width, height, options)
}

/// Converts RGBA8 bytes to floats in the filtering space.
fn to_floats(bytes: &[u8], conversion: MipConversion, floats: &mut [f32]) {
    match conversion {
        MipConversion::Linear => pixel::unit_floats(bytes, floats),
        MipConversion::Gamma => pixel::gamma_floats(bytes, floats),
        MipConversion::GammaColour => pixel::gamma_colour_floats(bytes, floats),
        MipConversion::Signed => pixel::signed_floats(bytes, floats),
    }
}

/// Converts filtered floats back to RGBA8 bytes.
fn to_bytes(floats: &[f32], conversion: MipConversion, bytes: &mut [u8]) {
    match conversion {
        MipConversion::Linear => pixel::unit_bytes(floats, bytes),
        MipConversion::Gamma => pixel::gamma_bytes(floats, bytes),
        MipConversion::GammaColour => pixel::gamma_colour_bytes(floats, bytes),
        MipConversion::Signed => pixel::signed_bytes(floats, bytes),
    }
}

/// The chain levels from `first_level` up, without the levels before it.
fn build_chain_from_floats(
    base: &[u8],
    width: usize,
    height: usize,
    options: &ChainOptions,
) -> (Vec<u8>, u32) {
    let first_level = options.first_level;
    let first = first_level.max(1);
    let mut floats = vec![0.0f32; width * height * 4];
    to_floats(&base[..width * height * 4], options.conversion, &mut floats);
    let floats = floats.as_slice();
    // The chain's levels, base bytes included when `first_level` is 0.
    let mut out: Vec<u8> = if first_level == 0 {
        base.to_vec()
    } else {
        Vec::new()
    };
    let mut count = if first_level == 0 { 1u32 } else { 0u32 };
    let [wrap_x, wrap_y] = options.wrap;
    for level in first..=options.last_level.min(KAISER_MAX_LEVEL) {
        let dst_width = (width >> level).max(1);
        let dst_height = (height >> level).max(1);
        let weights = options.kernel.table(level);
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
                    scope.spawn(move || {
                        let mut row = vec![0.0f32; dst_width * 4];
                        for row_index in 0..rows {
                            let y = start_row + row_index;
                            resample_axis(
                                &floats[y * width * 4..(y + 1) * width * 4],
                                width,
                                dst_width,
                                weights,
                                wrap_x,
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
                    wrap_x,
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
                                weights,
                                wrap_y,
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
                resample_axis(&column, height, dst_height, weights, wrap_y, &mut output);
                for y in 0..dst_height {
                    let destination = (y * dst_width + x) * 4;
                    texels[destination..destination + 4].copy_from_slice(&output[y * 4..y * 4 + 4]);
                }
            }
        }
        if options.renormalise {
            renormalise_texels(&mut texels);
        }
        let mut level = vec![0u8; texels.len()];
        to_bytes(&texels, options.conversion, &mut level);
        out.extend(level);
        count += 1;
        if dst_width + dst_height <= 2 {
            break;
        }
    }
    (out, count)
}

/// The filtering space of the resample-to-size path: signed data
/// centred; data words (bit 16 or 17), and any image the resample
/// magnifies (fewer source texels than destination texels), linear; other
/// words squared. RGBA8 and LUMA8 colour words (no data or signed bit)
/// keep the squared colour with linear alpha whatever the sizes, as the
/// original's per-format override comes after the magnification test.
pub fn resample_conversion(format_word: u32, src_texels: u64, dst_texels: u64) -> MipConversion {
    let plain = format_word & 0x1003_0000 == 0;
    match format_word & pixel::TARGET_MASK {
        0x6208 | 0xE608 if plain => return MipConversion::GammaColour,
        _ => {}
    }
    if format_word & 0x1000_0000 != 0 {
        MipConversion::Signed
    } else if src_texels < dst_texels || format_word & 0x0003_0000 != 0 {
        MipConversion::Linear
    } else {
        MipConversion::Gamma
    }
}

/// Resamples an RGBA8 image to a new size, as the original's
/// resample-to-size path does (icons, cursors, terrain inputs and GUI
/// images). When the destination is the source shifted down by `k` (1 to
/// 13) on both axes, the result is level `k` of the image's mip chain
/// ([`power_of_two_level`], [`ChainOptions::for_image`]); otherwise both
/// axes go through the phase-shifted window, in the space
/// [`resample_conversion`] picks, wrapping or clamping by the image flags'
/// `0x10000` and `0x20000`, and renormalised under `0x4000000`.
#[allow(clippy::too_many_arguments)]
pub fn resample_rgba8_to_size(
    base: &[u8],
    width: usize,
    height: usize,
    dst_width: usize,
    dst_height: usize,
    format_word: u32,
    image_flags: u32,
) -> Vec<u8> {
    if width == 0 || height == 0 || dst_width == 0 || dst_height == 0 {
        return Vec::new();
    }
    let usable = width.saturating_mul(height).saturating_mul(4);
    if base.len() < usable {
        return Vec::new();
    }
    if let Some(level) = power_of_two_level(width, height, dst_width, dst_height) {
        let options = ChainOptions::for_image(format_word, image_flags, level, level);
        return build_mip_chain_rgba8_with(base, width, height, &options).0;
    }
    let conversion = resample_conversion(
        format_word,
        (width * height) as u64,
        (dst_width as u64).saturating_mul(dst_height as u64),
    );
    let wrap_x = image_flags & 0x0001_0000 != 0;
    let wrap_y = image_flags & 0x0002_0000 != 0;
    let mut floats = vec![0.0f32; usable];
    to_floats(&base[..usable], conversion, &mut floats);
    let mut horizontal = vec![0.0f32; dst_width * height * 4];
    let mut row = vec![0.0f32; dst_width * 4];
    for y in 0..height {
        resample_axis_phased(
            &floats[y * width * 4..(y + 1) * width * 4],
            width,
            dst_width,
            wrap_x,
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
        resample_axis_phased(&column, height, dst_height, wrap_y, &mut output);
        for y in 0..dst_height {
            let destination = (y * dst_width + x) * 4;
            texels[destination..destination + 4].copy_from_slice(&output[y * 4..y * 4 + 4]);
        }
    }
    if renormalises(format_word, image_flags) {
        renormalise_texels(&mut texels);
    }
    let mut out = vec![0u8; texels.len()];
    to_bytes(&texels, conversion, &mut out);
    out
}

/// Sharpens an RGBA8 image in place of its own size, as the original's image
/// sharpen does for icons and the menu cursor: the [`sharpen_kernel`] of
/// `amount` and `radius` run as the engine's separable axis filter at one
/// source texel per output texel (first tap [`mip_axis_start`]), across
/// every row and then down every column, in the filtering space
/// [`resample_conversion`] picks for equal texel counts, wrapping or
/// clamping by the image flags' `0x10000` and `0x20000`. Returns an empty
/// image when the input is shorter than its size.
pub fn sharpen_rgba8(
    base: &[u8],
    width: usize,
    height: usize,
    amount: f32,
    radius: f32,
    format_word: u32,
    image_flags: u32,
) -> Vec<u8> {
    let Some(usable) = width.checked_mul(height).and_then(|n| n.checked_mul(4)) else {
        return Vec::new();
    };
    if usable == 0 || base.len() < usable {
        return Vec::new();
    }
    let weights = sharpen_kernel(amount, radius);
    let texels = (width * height) as u64;
    let conversion = resample_conversion(format_word, texels, texels);
    let wrap_x = image_flags & 0x0001_0000 != 0;
    let wrap_y = image_flags & 0x0002_0000 != 0;
    let mut floats = vec![0.0f32; usable];
    to_floats(&base[..usable], conversion, &mut floats);
    let mut horizontal = vec![0.0f32; usable];
    for y in 0..height {
        let row = y * width * 4..(y + 1) * width * 4;
        resample_axis(
            &floats[row.clone()],
            width,
            width,
            &weights,
            wrap_x,
            &mut horizontal[row],
        );
    }
    let mut column = vec![0.0f32; height * 4];
    let mut output = vec![0.0f32; height * 4];
    for x in 0..width {
        for y in 0..height {
            let source = (y * width + x) * 4;
            column[y * 4..y * 4 + 4].copy_from_slice(&horizontal[source..source + 4]);
        }
        resample_axis(&column, height, height, &weights, wrap_y, &mut output);
        for y in 0..height {
            let destination = (y * width + x) * 4;
            floats[destination..destination + 4].copy_from_slice(&output[y * 4..y * 4 + 4]);
        }
    }
    let mut out = vec![0u8; usable];
    to_bytes(&floats, conversion, &mut out);
    out
}

/// One axis pass with the phase-shifted window: output texel `i` is centred
/// on source position `i·ratio` (`ratio = src/dst`, divided in f64 and
/// rounded to f32; the truncated whole part picks the taps, the fraction
/// goes into the window's phase), and its taps span two output texels each
/// way. The first tap is the whole part less `(taps − ⌊src/dst⌋)/2`
/// (truncated; the integer ratio is 0 when the pass magnifies).
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
    let ratio = (src_extent as f64 / dst_extent as f64) as f32;
    let scale = ((dst_extent as f64 / src_extent as f64) as f32).min(1.0);
    // The original's base offset: half an output texel, or half a source
    // texel when the pass magnifies.
    let base = if ratio >= 1.0 { 0.5 } else { ratio * 0.5 };
    let integer_ratio = (src_extent / dst_extent) as i64;
    for x in 0..dst_extent {
        let position = x as f32 * ratio;
        let whole = position as i64;
        let fraction = position - whole as f32;
        let offset = base - fraction;
        let (_, weights) = window_kernel(scale, offset, WINDOW_RADIUS);
        let first = whole - ((weights.len() as i64 - integer_ratio) / 2);
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
        use exa_formats::pixel::id;
        let base = [255u8, 255, 0, 255].repeat(4 * 4);
        let options = ChainOptions::for_image(id::XYZV8, 0x0403_0000, 0, 1);
        assert!(options.renormalise);
        assert_eq!(options.conversion, MipConversion::Signed);
        let (out, _) = build_mip_chain_rgba8_with(&base, 4, 4, &options);
        assert_eq!(&out[64..68], &[201, 201, 54, 255]);
        let (plain, _) =
            build_mip_chain_rgba8_between(&base, 4, 4, 0, 1, true, MipConversion::Signed);
        assert_eq!(&plain[64..68], &[255, 255, 0, 255]);
        // A two-channel word is never renormalised, whatever its flag.
        assert!(!renormalises(id::XY8, 0x0400_0000));
        assert!(renormalises(id::XYZ8, 0x0400_0000));
        assert!(!renormalises(id::XYZ8, 0));
    }

    /// A zero vector stays zero under the engine's normaliser (its mask
    /// turns the infinite reciprocal into 0), and alpha is never touched.
    #[test]
    fn a_zero_vector_stays_zero_when_renormalised() {
        let mut texels = [0.0, 0.0, 0.0, 0.5, 3.0, 0.0, 4.0, 1.0];
        renormalise_texels(&mut texels);
        assert_eq!(&texels[..4], &[0.0, 0.0, 0.0, 0.5]);
        // (3, 0, 4) has length 5; the reciprocal estimate is within 2^-11.
        assert!((texels[4] - 0.6).abs() < 1e-3, "{texels:?}");
        assert!((texels[6] - 0.8).abs() < 1e-3, "{texels:?}");
        assert_eq!(texels[7], 1.0);
    }

    /// Data words filter with the uniform bank: a 0, 0, 255, 255 row halves
    /// to exactly 0 and 255, where the Kaiser bank of a colour word blurs
    /// the step across both outputs.
    #[test]
    fn data_words_filter_with_the_box_bank() {
        use exa_formats::pixel::id;
        let mut base = Vec::new();
        for _ in 0..2 {
            for v in [0u8, 0, 255, 255] {
                base.extend_from_slice(&[v, v, v, 255]);
            }
        }
        assert_eq!(MipKernel::for_format(id::ABCD8), MipKernel::Box);
        assert_eq!(MipKernel::for_format(id::XYZV8), MipKernel::Box);
        assert_eq!(MipKernel::for_format(id::RGBA8), MipKernel::Kaiser);
        assert_eq!(MipKernel::for_format(0x817B_E608), MipKernel::Kaiser);
        let data = ChainOptions::for_image(id::ABCD8, 0, 1, 1);
        let (out, count) = build_mip_chain_rgba8_with(&base, 4, 2, &data);
        assert_eq!(count, 1);
        assert_eq!(out, [0, 0, 0, 255, 255, 255, 255, 255]);
        let colour = ChainOptions {
            kernel: MipKernel::Kaiser,
            ..data
        };
        let (blurred, _) = build_mip_chain_rgba8_with(&base, 4, 2, &colour);
        assert!(blurred[0] > 0 && blurred[4] < 255, "{blurred:?}");
        assert_eq!(box_table(1), [0.5, 0.5]);
        assert_eq!(box_table(3), [0.125; 8]);
    }

    /// The image's own flags pick wrap or clamp on each axis: with clamping
    /// a bright last column cannot bleed into the first output texel.
    #[test]
    fn the_image_flags_pick_wrap_or_clamp_per_axis() {
        use exa_formats::pixel::id;
        let mut base = Vec::new();
        for _ in 0..8 {
            for x in 0..8 {
                let v = if x == 7 { 255 } else { 0 };
                base.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let clamped = ChainOptions::for_image(id::RGBA8, 0, 1, 1);
        assert_eq!(clamped.wrap, [false, false]);
        let across = ChainOptions::for_image(id::RGBA8, 0x0001_0000, 1, 1);
        assert_eq!(across.wrap, [true, false]);
        assert_eq!(
            ChainOptions::for_image(id::RGBA8, 0x0002_0000, 1, 1).wrap,
            [false, true]
        );
        let (clamp_out, _) = build_mip_chain_rgba8_with(&base, 8, 8, &clamped);
        let (wrap_out, _) = build_mip_chain_rgba8_with(&base, 8, 8, &across);
        assert!(wrap_out[0] > clamp_out[0], "{wrap_out:?} {clamp_out:?}");
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
        use exa_formats::pixel::id;
        let base = vec![200u8; 8 * 8 * 4];
        let out = resample_rgba8_to_size(&base, 8, 8, 5, 3, id::ABCD8, 0x0003_0000);
        assert_eq!(out.len(), 5 * 3 * 4);
        assert!(
            out.iter().all(|&value| (199..=200).contains(&value)),
            "{out:?}"
        );
        let out = resample_rgba8_to_size(&base, 8, 8, 13, 11, id::ABCD8, 0);
        assert_eq!(out.len(), 13 * 11 * 4);
        assert!(out.iter().all(|&value| (199..=200).contains(&value)));
    }

    /// A destination that is the source shifted down is that level of the
    /// image's own chain, not the windowed resample.
    #[test]
    fn a_power_of_two_resample_is_a_chain_level() {
        use exa_formats::pixel::id;
        let base: Vec<u8> = (0..8 * 8 * 4).map(|i| (i * 7 % 251) as u8).collect();
        let resampled = resample_rgba8_to_size(&base, 8, 8, 2, 2, id::RGBA8, 0x0003_0000);
        let options = ChainOptions::for_image(id::RGBA8, 0x0003_0000, 2, 2);
        let (level, count) = build_mip_chain_rgba8_with(&base, 8, 8, &options);
        assert_eq!(count, 1);
        assert_eq!(resampled, level);
    }

    /// The resample's filtering space: signed centred, data and magnified
    /// images linear, the rest squared, with the RGBA8 and LUMA8 colour
    /// words squared whatever the sizes.
    #[test]
    fn the_resample_picks_its_space_by_word_and_size() {
        use exa_formats::pixel::id;
        assert_eq!(resample_conversion(id::XYZ8, 4, 1), MipConversion::Signed);
        assert_eq!(resample_conversion(id::ABC8, 4, 1), MipConversion::Linear);
        assert_eq!(resample_conversion(id::RGB8, 4, 1), MipConversion::Gamma);
        assert_eq!(resample_conversion(id::RGB8, 1, 4), MipConversion::Linear);
        assert_eq!(resample_conversion(id::LUM8, 1, 4), MipConversion::Linear);
        assert_eq!(
            resample_conversion(id::RGBA8, 1, 4),
            MipConversion::GammaColour
        );
        assert_eq!(
            resample_conversion(id::LUMA8, 4, 1),
            MipConversion::GammaColour
        );
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

    /// The image sharpen runs the kernel along each row and then each
    /// column at the image's own size: alpha (linear in an RGBA8 image)
    /// 100, 100, 200, 200 with amount 0.1 and radius 1 (taps −0.1, 1.2,
    /// −0.1) becomes 100, 90, 210, 200, the clamped ends unchanged, and a
    /// single row's column pass leaves it as it is. A flat image stays flat.
    #[test]
    fn the_image_sharpen_steepens_an_edge() {
        use exa_formats::pixel::id;
        let mut row = Vec::new();
        for alpha in [100u8, 100, 200, 200] {
            row.extend([0, 0, 0, alpha]);
        }
        let out = sharpen_rgba8(&row, 4, 1, 0.1, 1.0, id::RGBA8, 0);
        let alphas: Vec<u8> = out.chunks(4).map(|px| px[3]).collect();
        assert_eq!(alphas, [100, 90, 210, 200]);

        let flat = vec![128u8; 3 * 3 * 4];
        assert_eq!(sharpen_rgba8(&flat, 3, 3, 0.01, 1.0, id::RGBA8, 0), flat);
        // Wrapping rows: the left end now sees the right one.
        let wrapped = sharpen_rgba8(&row, 4, 1, 0.1, 1.0, id::RGBA8, 0x0001_0000);
        assert_eq!(wrapped[3], 90);
        assert!(sharpen_rgba8(&row, 5, 1, 0.1, 1.0, id::RGBA8, 0).is_empty());
    }
}
