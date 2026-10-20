//! Kaiser-windowed mipmaps, as the original builds them
//! (`RFP_Mipmaps_Kaiser`): a bank of 13 normalised kernels, one per level,
//! and a separable resample that reads the base level directly.
//!
//! For level `L` the table has `n = 8·2^(L−1)` taps, `h = n/2` and
//! `c = 2^−L`. Tap `i` uses `u = i − h + 0.5`, `t = u/h` and
//! `w = I0(4·√(1 − t²))/I0(4) · sinc(π·c·u)`, or 0 outside `|t| < 1`;
//! the weights are normalised to sum 1. The axis pass uses the integer
//! ratio `src/dst`, starts at `−((n − ratio)/2)`, and wraps or clamps each
//! tap. Channels are filtered interleaved (RGBA in the model), as floats in
//! the space the format word picks: linear, colour squared, or centred for
//! signed data ([`exa_formats::pixel::mip_conversion`]).

use exa_formats::pixel::{self, MipConversion};

/// The Kaiser window's beta.
pub const KAISER_BETA: f64 = 4.0;
/// The tap base of level 1.
pub const KAISER_BASE_TAPS: usize = 8;
/// The largest table level the original builds.
pub const KAISER_MAX_LEVEL: u32 = 13;

/// The modified Bessel function of the first kind, order zero, by its
/// series: `Σ ((x/2)^m / m!)²`, stopped when a term is under 1e-16 of the
/// sum.
pub fn bessel_i0(x: f64) -> f64 {
    let half = x * 0.5;
    let mut term = 1.0f64;
    let mut sum = 1.0f64;
    let mut m = 1.0f64;
    loop {
        term *= (half / m) * (half / m);
        sum += term;
        if term < sum * 1e-16 {
            return sum;
        }
        m += 1.0;
    }
}

/// `sin(x)/x`, with the series `1 − x²/6 + x⁴/120` under 1e-4.
pub fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-4 {
        1.0 - x * x / 6.0 + x.powi(4) / 120.0
    } else {
        x.sin() / x
    }
}

/// The 13 tables, built once for every texture that needs them.
fn kaiser_bank() -> &'static [Vec<f32>] {
    static BANK: std::sync::OnceLock<Vec<Vec<f32>>> = std::sync::OnceLock::new();
    BANK.get_or_init(|| (1..=KAISER_MAX_LEVEL).map(kaiser_table).collect())
}

/// The normalised Kaiser table of a mip level (1 through 13).
pub fn kaiser_table(level: u32) -> Vec<f32> {
    let level = level.clamp(1, KAISER_MAX_LEVEL);
    let taps = KAISER_BASE_TAPS << (level - 1);
    let half = taps as f64 * 0.5;
    let c = 2f64.powi(-(level as i32));
    let mut weights = vec![0.0f64; taps];
    let mut sum = 0.0f64;
    for (index, weight) in weights.iter_mut().enumerate() {
        let u = index as f64 - half + 0.5;
        let t = u / half;
        let radius = 1.0 - t * t;
        if radius >= 0.0 {
            *weight = bessel_i0(KAISER_BETA * radius.sqrt()) / bessel_i0(KAISER_BETA)
                * sinc(std::f64::consts::PI * c * u);
            sum += *weight;
        }
    }
    weights.iter().map(|value| (value / sum) as f32).collect()
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
    if src_extent == 0 || dst_extent == 0 || weights.is_empty() {
        return;
    }
    let ratio = (src_extent / dst_extent).max(1);
    let start = mip_axis_start(weights.len(), ratio);
    for x in 0..dst_extent {
        let s0 = start + x as i64 * ratio as i64;
        let mut acc = [0.0f32; 4];
        for (k, &weight) in weights.iter().enumerate() {
            let mut source = s0 + k as i64;
            source = if wrap {
                source.rem_euclid(src_extent as i64)
            } else {
                source.clamp(0, src_extent as i64 - 1)
            };
            let base = source as usize * 4;
            for (channel, value) in acc.iter_mut().enumerate() {
                *value += weight * src.get(base + channel).copied().unwrap_or(0.0);
            }
        }
        let base = x * 4;
        if let Some(slot) = out.get_mut(base..base + 4) {
            slot.copy_from_slice(&acc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bessel_and_sinc_series_match_their_values() {
        assert!((bessel_i0(0.0) - 1.0).abs() < 1e-12);
        assert!((bessel_i0(1.0) - 1.266_065_877_752_008_4).abs() < 1e-12);
        assert!((sinc(0.0) - 1.0).abs() < 1e-12);
        assert!(sinc(std::f64::consts::PI).abs() < 1e-3);
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
/// both dimensions stay above zero, the width plus height stays at least
/// 3, and the dimensions divide exactly (the power-of-two path). Each level
/// is resampled directly from the base with its own Kaiser table, first
/// horizontally then vertically.
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
    for level in 1..=max_levels {
        let dst_width = width >> level;
        let dst_height = height >> level;
        if dst_width == 0 || dst_height == 0 || dst_width + dst_height < 3 {
            break;
        }
        if !width.is_multiple_of(dst_width) || !height.is_multiple_of(dst_height) {
            break;
        }
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
        assert_eq!(levels.len(), 2);
        assert_eq!((levels[0].width, levels[0].height), (4, 4));
        assert_eq!((levels[1].width, levels[1].height), (2, 2));
        for level in levels {
            for value in level.texels {
                assert!((value - 0.4).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn a_small_or_npot_image_has_no_chain() {
        assert!(build_mip_chain(&[0.0; 4], 1, 1, false, 3).is_empty());
        assert!(build_mip_chain(&[0.0; 3 * 3 * 4], 3, 3, false, 3).is_empty());
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
/// `first_level` of the Kaiser chain and the levels below it, packed as a
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
/// off). The chain also stops where the dimensions allow.
pub fn build_mip_chain_rgba8_between(
    base: &[u8],
    width: usize,
    height: usize,
    first_level: u32,
    last_level: u32,
    wrap: bool,
    conversion: MipConversion,
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
    );
    if count == 0 {
        // A chain that stops before `first_level` (a tiny image): the caller
        // has nothing to upload. The retail data never asks for this.
        return (Vec::new(), 0);
    }
    (chain, count)
}

/// The chain levels from `first_level` up, without the levels before it.
fn build_chain_from_floats(
    base: &[u8],
    width: usize,
    height: usize,
    first_level: u32,
    last_level: u32,
    wrap: bool,
    conversion: MipConversion,
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
    for level in 1..=last_level.min(KAISER_MAX_LEVEL) {
        let dst_width = width >> level;
        let dst_height = height >> level;
        if dst_width == 0 || dst_height == 0 || dst_width + dst_height < 3 {
            break;
        }
        if !width.is_multiple_of(dst_width) || !height.is_multiple_of(dst_height) {
            break;
        }
        if level < first {
            continue;
        }
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
        match conversion {
            MipConversion::Linear => pixel::unit_bytes(&texels, &mut level),
            MipConversion::Gamma => pixel::gamma_bytes(&texels, &mut level),
            MipConversion::GammaColour => pixel::gamma_colour_bytes(&texels, &mut level),
            MipConversion::Signed => pixel::signed_bytes(&texels, &mut level),
        }
        out.extend(level);
        count += 1;
    }
    (out, count)
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
            let (out, count) = build_mip_chain_rgba8(&base, 4, 4, true, conversion);
            assert_eq!(count, 2);
            assert_eq!(out.len(), 4 * 4 * 4 + 2 * 2 * 4);
            assert!(out.iter().all(|&value| value == 51), "{conversion:?}");
        }

        let (short, count) = build_mip_chain_rgba8(&[7; 4], 1, 1, true, MipConversion::Linear);
        assert_eq!(count, 1);
        assert_eq!(short, vec![7; 4]);
    }

    /// A later first level is uploaded as the base and halves the chain, as
    /// the texture detail setting does; each level is still filtered from
    /// the stored base.
    #[test]
    fn a_chain_can_start_at_a_later_level() {
        let base = vec![51u8; 8 * 8 * 4];
        // Level 1: 4x4; level 2: 2x2; level 3 would be 1x1, which the chain
        // stops before (the sides must sum to 3 or more).
        let (out, count) = build_mip_chain_rgba8_from(&base, 8, 8, 1, true, MipConversion::Linear);
        assert_eq!(count, 2);
        assert_eq!(out.len(), 4 * 4 * 4 + 2 * 2 * 4);
        assert!(out.iter().all(|&value| value == 51));

        let (out, count) = build_mip_chain_rgba8_from(&base, 8, 8, 2, true, MipConversion::Linear);
        assert_eq!(count, 1);
        assert_eq!(out.len(), 2 * 2 * 4);
        assert!(out.iter().all(|&value| value == 51));

        // Past the end of the chain there is nothing to upload.
        let (out, count) = build_mip_chain_rgba8_from(&base, 8, 8, 3, true, MipConversion::Linear);
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
}
