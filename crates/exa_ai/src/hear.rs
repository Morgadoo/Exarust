//! What an idle character does about a noise, before its own activity.
//!
//! A disturbance above 0.3 sends it to that point. Otherwise an alert of
//! 0.2 or more makes it face a known target that is more than 45° off, or
//! look around once the gap since the last turn has elapsed. That gap is
//! between 0.7 s and 3 s.
//!
//! Evidence: the idle check and the alert look. The retail disturbance
//! sample is a constant 0, so a quiet cell never leaves the idle.

use bevy::math::Vec3;

/// A noise at least this strong is investigated.
pub const DISTURBANCE: f32 = 0.3;
/// An alert at least this high takes the look.
pub const ALERT: f32 = 0.2;
/// A known target further off than this is faced, in radians.
pub const LOOK_OFF: f32 = 45.0 * std::f32::consts::PI / 180.0;
/// Shortest and longest pause between look-around turns, in seconds.
pub const LOOK_SOON: f32 = 0.7;
pub const LOOK_LATE: f32 = 3.0;

/// Where the idle's attention goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Attention {
    /// Its own activity.
    Idle,
    /// Walk to the loud point.
    Investigate(Vec3),
    /// Turn to the known target.
    FaceTarget,
    /// Pick a new look-around heading.
    LookAround,
}

/// Disturbance first, then the alert look, then the idle.
pub fn attention(
    disturbance: f32,
    point: Vec3,
    alert: f32,
    target_off: Option<f32>,
    waited_s: f32,
    gap_s: f32,
) -> Attention {
    if disturbance.is_finite() && disturbance > DISTURBANCE && point.is_finite() {
        return Attention::Investigate(point);
    }
    if !alert.is_finite() || alert < ALERT {
        return Attention::Idle;
    }
    if target_off.is_some_and(|off| off.is_finite() && off.abs() > LOOK_OFF) {
        return Attention::FaceTarget;
    }
    let gap = if gap_s.is_finite() {
        gap_s.clamp(LOOK_SOON, LOOK_LATE)
    } else {
        LOOK_LATE
    };
    if waited_s.is_finite() && waited_s >= gap {
        Attention::LookAround
    } else {
        Attention::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_noise_is_walked_to_and_an_alert_looks() {
        let point = Vec3::new(30.0, 0.0, 40.0);
        assert_eq!(
            attention(0.3, point, 0.0, Some(1.0), 3.0, 0.7),
            Attention::Idle
        );
        assert_eq!(
            attention(0.31, point, 0.0, None, 0.0, 0.7),
            Attention::Investigate(point)
        );
        assert_eq!(
            attention(0.0, point, 0.2, Some(LOOK_OFF + 0.01), 0.0, 0.7),
            Attention::FaceTarget
        );
        assert_eq!(
            attention(0.0, point, 0.2, Some(0.1), 0.69, 0.7),
            Attention::Idle
        );
        assert_eq!(
            attention(0.0, point, 0.2, Some(0.1), 0.7, 0.7),
            Attention::LookAround
        );
        assert_eq!(
            attention(0.0, point, 0.19, Some(1.0), 3.0, 0.7),
            Attention::Idle
        );
    }
}
