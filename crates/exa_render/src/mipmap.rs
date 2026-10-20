//! Kaiser-windowed mipmaps, as the original builds them
//! (`RFP_Mipmaps_Kaiser`): a bank of 13 normalised kernels, one per level,
//! and a separable resample that reads the base level directly.
//!
//! For level `L` the table has `n = 8·2^(L−1)` taps, `h = n/2` and
//! `c = 2^−L`. Tap `i` uses `u = i − h + 0.5`, `t = u/h` and
//! `w = I0(4·√(1 − t²))/I0(4) · sinc(π·c·u)`, or 0 outside `|t| < 1`;
//! the weights are normalised to sum 1. The axis pass uses the integer
//! ratio `src/dst`, starts at `−((n − ratio)/2)`, and wraps or clamps each
//! tap. Channels are filtered interleaved (RGBA in the model).

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
        let weights = kaiser_table(level);
        let mut horizontal = vec![0.0f32; dst_width * height * 4];
        let mut row = vec![0.0f32; dst_width * 4];
        for y in 0..height {
            let start = y * width * 4;
            let end = start + width * 4;
            resample_axis(
                &base[start..end],
                width,
                dst_width,
                &weights,
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
            resample_axis(&column, height, dst_height, &weights, wrap, &mut output);
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
/// generated level, and the total level count (base included). Channels are
/// normalised by 255 and quantised back on output.
pub fn build_mip_chain_rgba8(
    base: &[u8],
    width: usize,
    height: usize,
    wrap: bool,
) -> (Vec<u8>, u32) {
    if width == 0 || height == 0 || base.len() < width * height * 4 {
        return (base.to_vec(), 1);
    }
    let floats: Vec<f32> = base.iter().map(|value| f32::from(*value) / 255.0).collect();
    let levels = build_mip_chain(&floats, width, height, wrap, KAISER_MAX_LEVEL);
    let mut out = base.to_vec();
    for level in &levels {
        out.extend(
            level
                .texels
                .iter()
                .map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8),
        );
    }
    (out, 1 + levels.len() as u32)
}

#[cfg(test)]
mod rgba_chain_tests {
    use super::*;

    #[test]
    fn an_rgba_chain_appends_its_levels() {
        let base = vec![51u8; 4 * 4 * 4];
        let (out, count) = build_mip_chain_rgba8(&base, 4, 4, true);
        assert_eq!(count, 2);
        assert_eq!(out.len(), 4 * 4 * 4 + 2 * 2 * 4);
        assert!(out.iter().all(|&value| value == 51));

        let (short, count) = build_mip_chain_rgba8(&[7; 4], 1, 1, true);
        assert_eq!(count, 1);
        assert_eq!(short, vec![7; 4]);
    }
}
