//! The engine's numeric helpers the forms lean on, with the processor's
//! own conversions: a float rounded to an integer the way `CVTSS2SI` does
//! (ties to even; a NaN or a value out of range gives the "integer
//! indefinite", the most negative word) and the engine's single-precision
//! sine and cosine.

/// `CVTSS2SI` to a 64-bit word: round half to even; NaN or out of range is
/// `i64::MIN`.
pub fn round_i64(value: f32) -> i64 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || rounded >= 9.223_372e18 || rounded < -9.223_372e18 {
        i64::MIN
    } else {
        rounded as i64
    }
}

/// `CVTPS2DQ` of one lane: round half to even; NaN or out of range is
/// `i32::MIN`.
pub fn round_i32(value: f32) -> i32 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || rounded >= 2_147_483_648.0 || rounded < -2_147_483_648.0 {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// The processor's approximate reciprocal square root (`RSQRTSS`).
pub fn reciprocal_sqrt(value: f32) -> f32 {
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

/// The engine's sine and cosine of `x`: the angle reduced by the nearest
/// even multiple of π/4 (one Cody–Waite step), then the single-precision
/// polynomials of the reduced angle, each octant picking and signing them.
/// Returns `(sin, cos)`.
pub fn sin_cos(x: f32) -> (f32, f32) {
    let sign_sin = x.to_bits() & 0x8000_0000;
    let x = f32::from_bits(x.to_bits() & 0x7fff_ffff);
    let j = round_i32(x * 1.273_239_5).wrapping_add(1) & !1;
    let y = j as f32;
    let first = j & 2 == 0;
    let sign_sin = sign_sin ^ (((j & 4) as u32) << 29);
    let sign_cos = ((!(j.wrapping_sub(2)) & 4) as u32) << 29;
    let x = x - y * 0.785_398_2;
    let z = x * x;
    let mut c = 2.443_315_7e-5 * z + -1.388_731_6e-3;
    c = c * z + 4.166_664_6e-2;
    c *= z;
    c *= z;
    c -= 0.5 * z;
    c += 1.0;
    let mut s = -1.951_529_6e-4 * z + 8.332_161e-3;
    s = s * z + -1.666_665_5e-1;
    s *= z;
    s *= x;
    s += x;
    let (sin, cos) = if first { (s, c) } else { (c, s) };
    (
        f32::from_bits(sin.to_bits() ^ sign_sin),
        f32::from_bits(cos.to_bits() ^ sign_cos),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_round_half_to_even_and_flag_the_rest() {
        assert_eq!(round_i64(2.5), 2);
        assert_eq!(round_i64(3.5), 4);
        assert_eq!(round_i64(-2.5), -2);
        assert_eq!(round_i64(f32::NAN), i64::MIN);
        assert_eq!(round_i64(f32::INFINITY), i64::MIN);
        assert_eq!(round_i32(1e10), i32::MIN);
        assert_eq!(round_i32(-0.5), 0);
    }

    #[test]
    fn the_engine_sine_follows_the_library_closely() {
        for i in -40..40 {
            let x = i as f32 * 0.37;
            let (s, c) = sin_cos(x);
            assert!((s - x.sin()).abs() < 1e-5, "{x}");
            assert!((c - x.cos()).abs() < 1e-5, "{x}");
        }
        assert_eq!(sin_cos(0.0), (0.0, 1.0));
        let (s, c) = sin_cos(std::f32::consts::PI);
        assert!(s.abs() < 1e-6 && (c + 1.0).abs() < 1e-6);
    }
}
