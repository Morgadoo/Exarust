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
