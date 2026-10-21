//! The engine's fast natural logarithm and exponential, which the damage
//! model raises its crush and pierce shares with (x^y = exp(y·ln x)).
//!
//! The logarithm reads the exponent bits and a polynomial of the mantissa;
//! zero and negative arguments take −∞ as their offset (so ln 0 = −∞), a
//! non-number keeps the finite offset. The exponential scales its argument
//! into float bits, rounds them to the nearest integer (ties to even),
//! clamps them at 0 below and 0x4EFF0000 (as a float) above, and multiplies
//! the exponent bits by a polynomial of the mantissa: exp(−∞) = 0.

/// The logarithm's constants, as float bits.
const LN2: u32 = 0x3F31_7218;
const LN_OFFSET: u32 = 0xC2B3_DE55;
const LN_C1: u32 = 0x4059_7523;
const LN_C2: u32 = 0x4011_9122;
const LN_C3: u32 = 0x3CCC_A7FC;
const LN_C4: u32 = 0x3E79_8F62;
/// The exponential's constants, as float bits.
const EXP_SCALE: u32 = 0x4B38_AA3B;
const EXP_BIAS: u32 = 0x4E7E_0000;
const EXP_MAX: u32 = 0x4EFF_0000;
const EXP_C0: u32 = 0x3F02_8D05;
const EXP_C1: u32 = 0x3E9F_C08B;
const EXP_C2: u32 = 0x3E2A_9CBE;
const EXP_C3: u32 = 0xBB0B_4858;
const EXP_C4: u32 = 0x3C5D_BE77;

const MANTISSA: u32 = 0x007F_FFFF;
const EXPONENT: u32 = 0x7F80_0000;
const ONE: u32 = 0x3F80_0000;

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// The engine's fast natural logarithm.
pub fn fast_ln(x: f32) -> f32 {
    let bits = x.to_bits();
    let exponent = (bits >> 23) as i32 as f32;
    // Not (x ≤ 0): a positive argument or a non-number.
    let offset = if x <= 0.0 {
        f32::NEG_INFINITY
    } else {
        f(LN_OFFSET)
    };
    let m = f((bits & MANTISSA) | ONE);
    let m2 = m * m;
    let mut t = m * f(LN_C3) - f(LN_C4);
    t *= m2;
    t += m - f(LN_C2);
    t *= m2;
    t += exponent * f(LN2);
    (m * f(LN_C1) + offset) + t
}

/// The engine's fast exponential.
pub fn fast_exp(x: f32) -> f32 {
    let v = x * f(EXP_SCALE) + f(EXP_BIAS);
    // Below the top, else the top (a non-number too).
    let v = if v < f(EXP_MAX) { v } else { f(EXP_MAX) };
    // The conversion's out-of-range value is negative, so 0.
    let i = if v.is_nan() {
        0
    } else {
        (v.round_ties_even() as i32).max(0) as u32
    };
    let m = f((i & MANTISSA) | ONE);
    let p = (((m * f(EXP_C4) + f(EXP_C3)) * m + f(EXP_C2)) * m + f(EXP_C1)) * m + f(EXP_C0);
    f(i & EXPONENT) * p
}

/// x^y as the damage model takes it: exp(y·ln x), both fast.
pub fn fast_pow(x: f32, y: f32) -> f32 {
    fast_exp(fast_ln(x) * y)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Close to the true functions, not equal: ln 2 and e¹ within 1e-4.
    #[test]
    fn near_the_true_functions() {
        assert!((fast_ln(2.0) - std::f32::consts::LN_2).abs() < 1e-4);
        assert!((fast_ln(1.0)).abs() < 1e-4);
        assert!((fast_exp(1.0) - std::f32::consts::E).abs() < 1e-3);
        assert!((fast_pow(0.5, 2.0) - 0.25).abs() < 1e-4);
    }

    /// Zero has a logarithm of −∞ and its powers are 0; a huge argument
    /// saturates the exponential's bits at the exponent mask: +∞.
    #[test]
    fn the_edges() {
        assert_eq!(fast_ln(0.0), f32::NEG_INFINITY);
        assert_eq!(fast_ln(-1.0), f32::NEG_INFINITY);
        assert_eq!(fast_exp(f32::NEG_INFINITY), 0.0);
        assert_eq!(fast_pow(0.0, 1.5), 0.0);
        assert_eq!(fast_exp(1000.0), f32::INFINITY);
    }
}
