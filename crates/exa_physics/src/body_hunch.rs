//! The thorax hunch: a pinned part's local forward axis loses a smoothed
//! scalar on its height, then the basis is rebuilt. The scalar rises while
//! the body is on its feet and an attack clip takes more off it.
//!
//! Lengths are the original's. The original rebuilds the basis with the
//! processor's reciprocal square root; this normalises exactly, as the
//! torso twist does.

use bevy::prelude::*;

/// On its feet the hunch rises by this much a tick (double); off its feet
/// it falls by the same, and never below 0.
const STEP: f64 = 0.01;
/// An attack clip (actions 5–12) takes this much more off (double).
const ATTACK: f64 = 0.02;
/// The hunch never rises past this (double). The stored cap is the float 0.3.
const CAP: f64 = 0.3;

/// One tick of the hunch from the bearing the posture stage left: off its
/// feet it falls 0.01 (double) and stops at 0; otherwise it rises 0.01 and
/// stops at 0.3. A NaN stays. −0 is not below 0.
pub fn step_hunch(hunch: f32, off_feet: bool) -> f32 {
    if off_feet {
        let next = (f64::from(hunch) - STEP) as f32;
        if next < 0.0 { 0.0 } else { next }
    } else {
        let next = (f64::from(hunch) + STEP) as f32;
        if f64::from(next) > CAP { 0.3 } else { next }
    }
}

/// What an attack clip takes off the hunch (double), floored at 0. A NaN
/// stays. −0 is not below 0.
pub fn ease_hunch(hunch: f32) -> f32 {
    let next = (f64::from(hunch) - ATTACK) as f32;
    if next < 0.0 { 0.0 } else { next }
}

/// The hunch runs when the race's weapon recovery, widened to double, is
/// over 0.1. The float 0.1 widens to just over that double, so it passes;
/// the float just under it does not. A NaN does not.
pub fn hunch_applies(recovery: f32) -> bool {
    f64::from(recovery) > 0.1
}

/// The local turn after the hunch: the forward axis loses `hunch` on its
/// height, then the basis is rebuilt (that forward, the up axis with the
/// forward taken out, then their cross). A zero axis stays zero.
pub fn hunch_turn(turn: Quat, hunch: f32) -> Quat {
    let m = Mat3::from_quat(turn);
    let mut forward = m.z_axis;
    forward.y -= hunch;
    forward = forward.normalize_or_zero();
    let mut up = m.y_axis;
    up -= forward * up.dot(forward);
    up = up.normalize_or_zero();
    let side = up.cross(forward).normalize_or_zero();
    Quat::from_mat3(&Mat3::from_cols(side, up, forward))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On its feet the hunch rises 0.01 a tick and stops at 0.3. Off its
    /// feet it falls 0.01 and stops at 0. A NaN stays either way.
    #[test]
    fn the_hunch_rises_on_its_feet_and_falls_off_them() {
        let step = (0.01f64) as f32;
        assert_eq!(step_hunch(0.0, false).to_bits(), step.to_bits());
        assert_eq!(step_hunch(0.3, false).to_bits(), 0.3f32.to_bits());
        assert_eq!(step_hunch(0.0, true), 0.0);
        let down = step_hunch(0.3, true);
        assert!((f64::from(down) - (f64::from(0.3f32) - 0.01)).abs() < 1e-6);
        assert!(down > 0.0);
        assert!(step_hunch(f32::NAN, false).is_nan());
        assert!(step_hunch(f32::NAN, true).is_nan());
        assert_eq!(step_hunch(-0.0, true).to_bits(), 0.0f32.to_bits());
    }

    /// An attack takes 0.02 off, floored at 0. At the 0.3 cap the rise is
    /// swallowed first, so the attack then leaves 0.28. A NaN stays.
    #[test]
    fn an_attack_takes_two_hundredths_off() {
        let eased = ease_hunch(step_hunch(0.2, false));
        let expect = (f64::from(step_hunch(0.2, false)) - 0.02) as f32;
        assert_eq!(eased.to_bits(), expect.to_bits());
        assert_eq!(
            ease_hunch(step_hunch(0.3, false)).to_bits(),
            ease_hunch(0.3).to_bits()
        );
        assert!((ease_hunch(0.3) - 0.28).abs() < 1e-6);
        assert_eq!(ease_hunch(0.0), 0.0);
        assert!(ease_hunch(f32::NAN).is_nan());
    }

    /// Recovery over 0.1, the float widened to double. The float 0.1 passes;
    /// the float just under it, and a NaN, do not.
    #[test]
    fn recovery_over_a_tenth_lets_the_hunch_run() {
        assert!(hunch_applies(0.1));
        assert!(!hunch_applies(f32::from_bits(0x3dcc_cccc)));
        assert!(!hunch_applies(0.05));
        assert!(hunch_applies(1.0));
        assert!(!hunch_applies(f32::NAN));
    }

    /// Identity pitched by 0.3 drops its forward axis and stays orthonormal.
    /// A hunch of 0 leaves the turn.
    #[test]
    fn the_forward_axis_drops_and_the_basis_stays_square() {
        let pitched = hunch_turn(Quat::IDENTITY, 0.3);
        let m = Mat3::from_quat(pitched);
        assert!(m.z_axis.y < -0.25, "{}", m.z_axis);
        assert!(m.z_axis.z > 0.9);
        assert!((m.x_axis.length() - 1.0).abs() < 1e-5);
        assert!((m.y_axis.length() - 1.0).abs() < 1e-5);
        assert!((m.z_axis.length() - 1.0).abs() < 1e-5);
        assert!(m.x_axis.dot(m.y_axis).abs() < 1e-5);
        assert!(m.y_axis.dot(m.z_axis).abs() < 1e-5);
        assert!(m.z_axis.cross(m.x_axis).dot(m.y_axis) > 0.9);
        assert!(hunch_turn(Quat::IDENTITY, 0.0).abs_diff_eq(Quat::IDENTITY, 1e-6));
    }
}
