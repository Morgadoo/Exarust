//! A hinged door: its creak, the pull that unlatches it, and the shove that
//! opens it.
//!
//! The handle velocity is smoothed by half. The creak stops when the squared
//! speed is at most 0.015. Otherwise the speed is the square root of that,
//! the volume is 0.075 times the square root of the speed, and the pitch is
//! 0.022 times the speed plus 0.7, at most 1.
//!
//! A shut door's pull count rises by 1 up to 20, then, while it is held and
//! not latched and its angle is still short of closed, falls by 4. Below 0
//! it unlatches and the count returns to 0. From 20, the seventh pull does
//! that. Opening shoves the handle 3 cm along the door's facing.
//!
//! Evidence: the hinged door step and the open shove. The motor, the slider
//! and the partner door are not in this slice.

use bevy::math::Vec3;

/// Squared handle speed at or below which the creak stops.
pub const CREAK_STOP: f32 = 0.015;
/// The pull count a door gains when it shuts, and its cap.
pub const PULL_CAP: i32 = 20;
/// How much one held tick takes off the pull count.
pub const PULL_STEP: i32 = 4;
/// The shove that starts a door opening, in centimetres.
pub const OPEN_PUSH_CM: f32 = 3.0;

/// The creak of one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Creak {
    pub smoothed: Vec3,
    pub playing: bool,
    pub volume: f32,
    pub pitch: f32,
}

/// Smooth the handle velocity and say whether the hinge creaks.
pub fn creak(smoothed: Vec3, handle_velocity: Vec3) -> Creak {
    let smoothed = smoothed * 0.5 + handle_velocity * 0.5;
    let energy = smoothed.length_squared();
    if !energy.is_finite() || energy <= CREAK_STOP {
        return Creak {
            smoothed,
            playing: false,
            volume: 0.0,
            pitch: 0.0,
        };
    }
    let speed = energy.sqrt();
    Creak {
        smoothed,
        playing: true,
        volume: 0.075 * speed.sqrt(),
        pitch: (0.022 * speed + 0.7).min(1.0),
    }
}

/// The shut door's pull count and whether it is barred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pull {
    pub count: i32,
    pub latched: bool,
}

/// One tick of a shut door. Returns whether the unlatch sound plays.
pub fn pull_closed(pull: &mut Pull, holding: bool, angle: f32, closed: f32) -> bool {
    if pull.count < PULL_CAP {
        pull.count += 1;
    }
    if holding && !pull.latched && angle < closed {
        pull.count -= PULL_STEP;
        if pull.count < 0 {
            pull.count = 0;
            return true;
        }
    }
    false
}

/// The 3 cm shove along the door's facing. A zero facing shoves nothing.
pub fn open_push(facing: Vec3) -> Vec3 {
    let length = facing.length();
    if !length.is_finite() || length == 0.0 {
        Vec3::ZERO
    } else {
        facing * (OPEN_PUSH_CM / length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moving_hinge_creaks_and_the_seventh_pull_unlatches() {
        let still = creak(Vec3::ZERO, Vec3::ZERO);
        assert!(!still.playing);

        let moving = creak(Vec3::ZERO, Vec3::X);
        assert!(moving.playing);
        let speed = 0.5_f32;
        assert!((moving.volume - 0.075 * speed.sqrt()).abs() < 1e-5);
        assert!((moving.pitch - 0.711).abs() < 1e-5);

        let mut pull = Pull {
            count: PULL_CAP,
            latched: false,
        };
        let mut seen = [0; 6];
        for slot in &mut seen {
            assert!(!pull_closed(&mut pull, true, 0.0, 1.0));
            *slot = pull.count;
        }
        assert_eq!(seen, [16, 13, 10, 7, 4, 1]);
        assert!(pull_closed(&mut pull, true, 0.0, 1.0));
        assert_eq!(pull.count, 0);

        let mut barred = Pull {
            count: PULL_CAP,
            latched: true,
        };
        assert!(!pull_closed(&mut barred, true, 0.0, 1.0));
        assert_eq!(barred.count, PULL_CAP);

        assert!(open_push(Vec3::Z).abs_diff_eq(Vec3::new(0.0, 0.0, OPEN_PUSH_CM), 1e-4));
    }
}
