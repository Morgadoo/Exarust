//! The combatant's head tracking, as the original runs it each tick: two
//! relaxed angles, a yaw and a pitch, chasing the look-at point through
//! the engine's cheap polynomial atan2. The yaw moves at most π/100 a tick
//! and is then held within ±π/4 of the body's facing; the pitch moves the
//! same step and its target is floored at −0.6. The result is the unit look
//! direction, the point 250 cm along it and the flat (y = 0) unit
//! direction.
//!
//! Aim inside 50 cm reuses the previous direction; between 50 and 100 cm
//! the vertical component is replaced by `(10000 − d²)·1e-4` before the
//! direction is normalized.

use bevy::math::Vec3;
use exa_physics::body::locomotion::{asin_approx, heading_of};

/// The most the yaw and pitch move in one tick, π/100 (1.8°), as the
/// original compares it: the double π/100 against a single-precision
/// error.
pub const HEAD_STEP_RADIANS: f64 = std::f64::consts::PI / 100.0;
/// The step it adds to an angle: the double π/100 rounded to a float.
pub const HEAD_STEP: f32 = 0.031_415_928;
/// The squared xz radius under which the previous direction is reused.
pub const HEAD_NEAR_SQUARED: f32 = 2500.0;
/// The squared xz radius at which the mid-band blend ends.
pub const HEAD_FAR_SQUARED: f32 = 10_000.0;
/// The look-ahead distance of the aim point.
pub const HEAD_AHEAD: f32 = 250.0;
/// The lowest pitch the target may ask for: the float nearest −0.6.
pub const HEAD_PITCH_FLOOR: f32 = f32::from_bits(0xbf19_999a);
/// The yaw limit about the body's facing.
pub const HEAD_YAW_LIMIT: f32 = std::f32::consts::FRAC_PI_4;
/// The eye in the character frame (cm): 20 to the side and 10 up, the
/// fixed offset the tracker transforms by the frame.
pub const EYE_IN_FRAME_CM: [f32; 3] = [-20.0, 10.0, 0.0];

/// The two relaxed angles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadTrack {
    pub yaw: f32,
    pub pitch: f32,
}

/// The tracked look: the unit direction, the aim point and the flat unit
/// direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadLook {
    pub dir: [f32; 3],
    pub point: [f32; 3],
    pub flat: [f32; 3],
}

/// The engine's yaw of a point's x and z, its planar-angle routine: zero
/// when the horizontal length squared is under 1e-8, otherwise the
/// engine's arc cosine of z over that length (π at or below −1, zero above
/// 1), negated for a negative x (`exa_physics`'s heading, which the free
/// camera's planar yaw also matches).
pub fn head_atan2_xz(x: f32, z: f32) -> f32 {
    heading_of(Vec3::new(x, 0.0, z))
}

/// Wraps an angle into ±π as the original does: the float widened and
/// compared with the double π, then the float full turn added or taken.
fn wrap(angle: f32) -> f32 {
    if f64::from(angle) < -std::f64::consts::PI {
        angle + std::f32::consts::TAU
    } else if f64::from(angle) > std::f64::consts::PI {
        angle - std::f32::consts::TAU
    } else {
        angle
    }
}

fn step(current: f32, want: f32) -> f32 {
    let error = wrap(want - current);
    // The original widens the single-precision error and compares it with
    // the double π/100.
    if f64::from(error) > HEAD_STEP_RADIANS {
        wrap(current + HEAD_STEP)
    } else if f64::from(error) < -HEAD_STEP_RADIANS {
        wrap(current - HEAD_STEP)
    } else {
        want
    }
}

/// Tracks the head one tick: `eye` is the world eye point, `body_forward`
/// the body's flat facing and `look_at` the world look-at point; the
/// previous unit direction is used for a look-at within 50 cm.
pub fn track_head(
    head: &mut HeadTrack,
    previous_dir: [f32; 3],
    eye: [f32; 3],
    body_forward: [f32; 3],
    look_at: [f32; 3],
) -> HeadLook {
    track_head_inner(
        head,
        previous_dir,
        eye,
        body_forward,
        look_at,
        HEAD_AHEAD,
        Some(HEAD_PITCH_FLOOR),
    )
}

/// The far head-tracking variant: a 150 cm look-ahead and
/// no pitch floor.
pub fn track_head_far(
    head: &mut HeadTrack,
    previous_dir: [f32; 3],
    eye: [f32; 3],
    body_forward: [f32; 3],
    look_at: [f32; 3],
) -> HeadLook {
    track_head_inner(head, previous_dir, eye, body_forward, look_at, 150.0, None)
}

fn track_head_inner(
    head: &mut HeadTrack,
    previous_dir: [f32; 3],
    eye: [f32; 3],
    body_forward: [f32; 3],
    look_at: [f32; 3],
    ahead: f32,
    pitch_floor: Option<f32>,
) -> HeadLook {
    let mut d = [
        look_at[0] - eye[0],
        look_at[1] - eye[1],
        look_at[2] - eye[2],
    ];
    let distance_squared = d[0] * d[0] + d[2] * d[2];
    let dir_in = if distance_squared < HEAD_NEAR_SQUARED {
        previous_dir
    } else {
        if distance_squared < HEAD_FAR_SQUARED {
            d[1] = (HEAD_FAR_SQUARED - distance_squared) * 1e-4;
        }
        let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        [d[0] / length, d[1] / length, d[2] / length]
    };

    let mut yaw = step(head.yaw, head_atan2_xz(dir_in[0], dir_in[2]));
    let body_yaw = head_atan2_xz(body_forward[0], body_forward[2]);

    // The target is the negated arc sine of the height, raised to the
    // floor by an ordered compare in double precision (a NaN target stays
    // NaN).
    let target = -asin_approx(dir_in[1]);
    let want_pitch = match pitch_floor {
        Some(floor) if f64::from(target) < -0.6 => floor,
        _ => target,
    };
    let pitch = step(head.pitch, want_pitch);

    // The yaw is held within ±45° of the body once, after the pitch step.
    // The bound is compared in double precision and the clamped angle is
    // the body yaw plus the float quarter turn.
    let offset = wrap(yaw - body_yaw);
    if f64::from(offset) > std::f64::consts::FRAC_PI_4 {
        yaw = wrap(body_yaw + HEAD_YAW_LIMIT);
    } else if f64::from(offset) < -std::f64::consts::FRAC_PI_4 {
        yaw = wrap(body_yaw - HEAD_YAW_LIMIT);
    }

    *head = HeadTrack { yaw, pitch };
    let dir = [
        pitch.cos() * yaw.sin(),
        -pitch.sin(),
        pitch.cos() * yaw.cos(),
    ];
    // The flat direction is scaled by the reciprocal of its length, not
    // divided by it.
    let flat_scale = 1.0 / (dir[0] * dir[0] + dir[2] * dir[2]).sqrt();
    HeadLook {
        dir,
        point: [
            eye[0] + ahead * dir[0],
            eye[1] + ahead * dir[1],
            eye[2] + ahead * dir[2],
        ],
        flat: [dir[0] * flat_scale, 0.0, dir[2] * flat_scale],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_step_is_the_double_pi_over_a_hundred() {
        assert_eq!(HEAD_STEP_RADIANS, std::f64::consts::PI / 100.0);
        // The float the original adds, and its pitch floor.
        assert_eq!(HEAD_STEP.to_bits(), 0x3d00_adfd);
        assert_eq!(HEAD_PITCH_FLOOR.to_bits(), 0xbf19_999a);
        // A float under the double step is inside the band, so the head
        // takes the wanted angle exactly; a larger error moves one step.
        let under = f32::from_bits(HEAD_STEP.to_bits() - 1);
        assert_eq!(step(0.0, under), under);
        assert_eq!(step(0.0, HEAD_STEP), HEAD_STEP);
        assert_eq!(step(0.0, 0.1), HEAD_STEP);
        assert_eq!(step(0.0, -0.1), -HEAD_STEP);
    }

    #[test]
    fn the_wrap_compares_with_the_double_pi() {
        // The float pi is above the double pi, so the original wraps it.
        let pi = std::f32::consts::PI;
        assert_eq!(wrap(pi), pi - std::f32::consts::TAU);
        assert_eq!(wrap(-pi), -pi + std::f32::consts::TAU);
        let under = f32::from_bits(pi.to_bits() - 1);
        assert_eq!(wrap(under), under);
    }

    #[test]
    fn the_pitch_floor_holds_at_the_floored_value() {
        // A point above the eye asks for a pitch of about −0.785, below the
        // floor. Held there, the head stays on the floored value.
        let mut head = HeadTrack {
            yaw: 0.0,
            pitch: HEAD_PITCH_FLOOR,
        };
        track_head(
            &mut head,
            [0.0, 0.0, 1.0],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [0.0, 1000.0, 1000.0],
        );
        assert_eq!(head.pitch, HEAD_PITCH_FLOOR);
        // From level it takes one step down.
        head.pitch = 0.0;
        let _ = track_head(
            &mut head,
            [0.0, 0.0, 1.0],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [0.0, 1000.0, 1000.0],
        );
        assert_eq!(head.pitch, -HEAD_STEP);
    }

    #[test]
    fn a_distant_target_is_tracked_at_once() {
        let mut head = HeadTrack {
            yaw: 0.0,
            pitch: 0.0,
        };
        let look = track_head(
            &mut head,
            [0.0, 0.0, 1.0],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [100.0, 0.0, 1000.0],
        );
        assert!((head.yaw - HEAD_STEP).abs() < 1e-6);
        assert!(head.pitch.abs() < 1e-6);
        let ahead = 250.0 * HEAD_STEP.sin();
        assert!((look.point[0] - ahead).abs() < 1e-3);
        assert!((look.point[2] - 250.0 * HEAD_STEP.cos()).abs() < 1e-3);
        assert!((look.flat[0] - HEAD_STEP.sin()).abs() < 1e-5);
    }

    #[test]
    fn the_atan2_is_the_engines_planar_angle() {
        assert_eq!(head_atan2_xz(0.0, 1.0), 0.0);
        assert_eq!(head_atan2_xz(0.0, -1.0), std::f32::consts::PI);
        // A small offset ahead is a small angle (within the arc cosine's
        // 4e-4), mirrored for a negative x.
        let side = head_atan2_xz(10.0, 1000.0);
        assert!((side - 0.01).abs() < 5e-4, "{side}");
        assert_eq!(head_atan2_xz(-10.0, 1000.0), -side);
        // x = z = 1: the upper form of the arc cosine, in single precision.
        let c = 1.0f32 / 2.0f32.sqrt();
        let upper =
            (c * c * 0.0535 + (std::f32::consts::FRAC_PI_2 - c * 0.2075)) * (1.0 - c).sqrt();
        assert_eq!(head_atan2_xz(1.0, 1.0), upper);
        // Behind: π less the same, in double precision.
        let behind = (std::f64::consts::PI - f64::from(upper)) as f32;
        assert_eq!(head_atan2_xz(1.0, -1.0), behind);
        // Under 1e-8 of horizontal length squared: zero.
        assert_eq!(head_atan2_xz(1e-5, 0.0), 0.0);
    }

    #[test]
    fn looking_up_takes_the_engines_arc_sine() {
        // Height 0.6 of a unit direction: asin 0.6435 (the polynomial is
        // within 1e-3); starting there, the far variant (no floor) holds it
        // exactly.
        let want = -asin_approx(0.6);
        assert!((want + 0.6435).abs() < 1e-3, "{want}");
        let mut head = HeadTrack {
            yaw: 0.0,
            pitch: want,
        };
        track_head_far(
            &mut head,
            [0.0, 0.0, 1.0],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [0.0, 600.0, 800.0],
        );
        assert_eq!(head.pitch, want);
    }

    #[test]
    fn the_yaw_moves_at_most_one_step() {
        let mut head = HeadTrack {
            yaw: 0.0,
            pitch: 0.0,
        };
        let look = track_head(
            &mut head,
            [0.0, 0.0, 1.0],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [100.0, 0.0, 100.0],
        );
        assert!((head.yaw - HEAD_STEP).abs() < 1e-6);
        assert_eq!(look.flat[1], 0.0);
    }

    #[test]
    fn a_close_target_keeps_the_previous_direction() {
        let mut head = HeadTrack {
            yaw: 0.0,
            pitch: 0.0,
        };
        let look = track_head(
            &mut head,
            [0.0, 0.0, 1.0],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [10.0, 0.0, 10.0],
        );
        assert_eq!(head.yaw, 0.0);
        assert_eq!(look.point, [0.0, 0.0, 250.0]);
    }

    #[test]
    fn the_mid_band_flattens_the_aim() {
        let mut head = HeadTrack {
            yaw: 0.0,
            pitch: 0.0,
        };
        track_head(
            &mut head,
            [0.0, 0.0, 1.0],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 80.0],
        );
        assert!(head.pitch < 0.0);
        assert!((head.pitch + 0.004_47).abs() < 1e-4);
    }

    #[test]
    fn the_body_holds_the_yaw_within_forty_five_degrees() {
        let mut head = HeadTrack {
            yaw: 1.0,
            pitch: 0.0,
        };
        let look = track_head(
            &mut head,
            [0.0, 0.0, 1.0],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [1000.0, 0.0, 0.0],
        );
        assert!((head.yaw - HEAD_YAW_LIMIT).abs() < 1e-6);
        assert!(look.flat[0] > 0.7);
    }

    #[test]
    fn the_far_variant_looks_a_hundred_and_fifty_centimetres() {
        let mut head = HeadTrack {
            yaw: 0.0,
            pitch: 0.0,
        };
        let look = track_head_far(
            &mut head,
            [0.0, 0.0, 1.0],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1000.0],
        );
        assert_eq!(look.point, [0.0, 0.0, 150.0]);

        let mut up = HeadTrack {
            yaw: 0.0,
            pitch: -1.02,
        };
        let look = track_head_far(
            &mut up,
            [0.0, 0.0, 1.0],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [100.0, 1000.0, 0.0],
        );
        assert!(up.pitch < -1.0);
        assert!(look.point[1] > 0.0);
    }
}
