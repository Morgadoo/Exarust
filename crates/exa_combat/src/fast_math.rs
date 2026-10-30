//! The engine's fast natural logarithm and exponential, which the damage
//! model raises its crush and pierce shares with (x^y = exp(y·ln x)).
//!
//! The functions are the one shared implementation in
//! [`exa_physics::engine_math`]: the logarithm reads the exponent bits and a
//! polynomial of the mantissa; zero and negative arguments take −∞ as their
//! offset (so ln 0 = −∞), a non-number keeps the finite offset. The
//! exponential scales its argument into float bits, rounds them to the
//! nearest integer (ties to even), clamps them at 0 below and 0x4EFF0000
//! (as a float) above, and multiplies the exponent bits by a polynomial of
//! the mantissa: exp(−∞) = 0.

pub use exa_physics::engine_math::{exp as fast_exp, ln as fast_ln};

/// x^y as the damage model takes it: exp(y·ln x), both fast.
pub fn fast_pow(x: f32, y: f32) -> f32 {
    fast_exp(fast_ln(x) * y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_approximations_are_close() {
        assert!((fast_ln(2.0) - std::f32::consts::LN_2).abs() < 1e-4);
        assert!((fast_ln(1.0)).abs() < 1e-3);
        assert!((fast_exp(1.0) - std::f32::consts::E).abs() < 1e-3);
        assert!((fast_pow(4.0, 0.5) - 2.0).abs() < 1e-3);
    }

    #[test]
    fn the_edges_are_the_engines() {
        // Pinned to the polynomials' exact results, not the true functions.
        assert_eq!(fast_ln(1.0).to_bits(), 0x3780_0000);
        assert_eq!(fast_exp(0.0).to_bits(), 0x3F80_0016);
        assert_eq!(fast_ln(0.0), f32::NEG_INFINITY);
        assert_eq!(fast_ln(-1.0), f32::NEG_INFINITY);
        assert_eq!(fast_exp(f32::NEG_INFINITY), 0.0);
        assert_eq!(fast_exp(1000.0), f32::INFINITY);
    }
}
