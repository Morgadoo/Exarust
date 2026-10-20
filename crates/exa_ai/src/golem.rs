//! The golem's dormancy and homing helpers (split out of the original's
//! `RFE_Effects_Electricity` unit by address).

/// The per-limb dormancy fade-in scale (0.05).
pub const DORMANCY_IN_SCALE: f64 = 0.05;
/// The per-limb dormancy fade-out scale (0.02).
pub const DORMANCY_OUT_SCALE: f64 = 0.02;

/// The dormancy fade-in: the stored value times 0.05, in double.
pub fn fade_in_scale(source: f32) -> f32 {
    (source as f64 * DORMANCY_IN_SCALE) as f32
}

/// The dormancy fade-out: the stored glow times 0.02, in double.
pub fn fade_out_scale(value: f32) -> f32 {
    (value as f64 * DORMANCY_OUT_SCALE) as f32
}

/// The homeward delay: the distance plus π plus a roll centred on 0.5.
pub fn home_delay(distance: f32, roll: f64) -> f32 {
    (distance as f64 + std::f64::consts::PI + roll - 0.5) as f32
}

/// The homeward reach: 200 plus the stored offset.
pub fn home_reach(offset: f32) -> f32 {
    200.0 + offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dormancy_fades() {
        assert!((fade_in_scale(20.0) - 1.0).abs() < 1e-6);
        assert!((fade_out_scale(50.0) - 1.0).abs() < 1e-6);
        assert_eq!(DORMANCY_IN_SCALE, 0.05);
        assert_eq!(DORMANCY_OUT_SCALE, 0.02);
    }

    #[test]
    fn the_homeward_delay_and_reach() {
        assert!((home_delay(10.0, 0.5) - (10.0 + std::f32::consts::PI)).abs() < 1e-5);
        assert!((home_delay(10.0, 1.0) - (10.5 + std::f32::consts::PI)).abs() < 1e-5);
        assert_eq!(home_reach(5.0), 205.0);
    }
}
