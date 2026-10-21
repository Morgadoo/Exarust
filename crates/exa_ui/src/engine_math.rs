//! The engine's numeric helpers, from the one shared implementation in
//! `exa_physics::engine_math`, with the UI's older call-site names.
//!
//! The conversions round half to even and flag NaN and out-of-range values
//! with the processor's integer indefinite; the sine and cosine are the
//! engine's fast polynomials.

pub use exa_physics::engine_math::*;

/// The 64-bit integer conversion under its UI call-site name.
pub fn round_i64(value: f32) -> i64 {
    round_to_i64(value)
}

/// The 32-bit integer conversion under its UI call-site name.
pub fn round_i32(value: f32) -> i32 {
    round_to_i32(value)
}
