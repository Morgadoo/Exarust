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
//! A toggle or an open command aims at `(1 − fraction) · range · sign + closed`.
//! A plain step is `|error| / |range| · 0.001 + 0.0003`. A motorised door
//! steps 0.0003, and a fast one 0.005. The drive moves that far toward the
//! aim. A toggle arrives within its step; an open command arrives within
//! 0.1 rad. A countdown below 2 ends the command after this tick's drive.
//! A door that bars itself latches on arrival and keeps the command.
//!
//! Evidence: the hinged door step and the open shove. The slider and the
//! partner door are not in this slice.

use bevy::math::Vec3;

/// Squared handle speed at or below which the creak stops.
pub const CREAK_STOP: f32 = 0.015;
/// The pull count a door gains when it shuts, and its cap.
pub const PULL_CAP: i32 = 20;
/// How much one held tick takes off the pull count.
pub const PULL_STEP: i32 = 4;
/// What a motor command is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorMode {
    Idle,
    /// Held open.
    Open,
    /// Moving to the open or closed fraction.
    Toggle,
}

/// One motor tick's aim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motor {
    pub mode: MotorMode,
    pub countdown: i32,
    pub motorised: bool,
    pub fast: bool,
    /// Bars itself when a toggle arrives.
    pub latches: bool,
    pub latched: bool,
    /// 1 is shut, 0 is open.
    pub fraction: f32,
    pub closed: f32,
    pub range: f32,
    pub sign: f32,
    pub angle: f32,
}

/// The angle to drive toward this tick, when a command is running.
pub fn motor_step(motor: &mut Motor) -> Option<f32> {
    let active = motor.mode;
    if active == MotorMode::Idle {
        return None;
    }
    if motor.countdown < 2 {
        motor.mode = MotorMode::Idle;
    } else {
        motor.countdown -= 1;
    }
    let target = (1.0 - motor.fraction) * motor.range * motor.sign + motor.closed;
    let error = target - motor.angle;
    let step = if !motor.motorised {
        let share = if motor.range.is_finite() && motor.range != 0.0 {
            (error / motor.range).abs()
        } else {
            0.0
        };
        share * MOTOR_CATCH + MOTOR_STEP
    } else if motor.fast {
        FAST_STEP
    } else {
        MOTOR_STEP
    };
    let arrived = match active {
        MotorMode::Toggle => error.abs() <= step,
        MotorMode::Open => error.abs() <= OPEN_SLOP,
        MotorMode::Idle => return None,
    };
    if arrived {
        if active == MotorMode::Toggle && motor.latches {
            motor.latched = true;
        } else {
            motor.mode = MotorMode::Idle;
        }
        Some(target)
    } else {
        let sign = if error > 0.0 {
            1.0
        } else if error < 0.0 {
            -1.0
        } else {
            0.0
        };
        Some(motor.angle + sign * step)
    }
}
/// The shove that starts a door opening, in centimetres.
pub const OPEN_PUSH_CM: f32 = 3.0;
/// A plain door's smallest motor step, in radians.
pub const MOTOR_STEP: f32 = 0.0003;
/// Added to the plain step, times the absolute error over the range.
pub const MOTOR_CATCH: f32 = 0.001;
/// A fast motor's step, in radians.
pub const FAST_STEP: f32 = 0.005;
/// An open command has arrived when the error is within this, in radians.
pub const OPEN_SLOP: f32 = 0.1;

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

    fn motor(mode: MotorMode) -> Motor {
        Motor {
            mode,
            countdown: 10,
            motorised: false,
            fast: false,
            latches: false,
            latched: false,
            fraction: 0.0,
            closed: 0.0,
            range: 1.0,
            sign: 1.0,
            angle: 0.0,
        }
    }

    #[test]
    fn a_motor_steps_toward_its_aim() {
        let mut plain = motor(MotorMode::Toggle);
        let drive = motor_step(&mut plain).unwrap();
        assert!((drive - 0.0013).abs() < 1e-6, "{drive}");
        assert_eq!(plain.mode, MotorMode::Toggle);
        assert_eq!(plain.countdown, 9);

        let mut fast = motor(MotorMode::Toggle);
        fast.motorised = true;
        fast.fast = true;
        let drive = motor_step(&mut fast).unwrap();
        assert!((drive - FAST_STEP).abs() < 1e-6);

        let mut open = motor(MotorMode::Open);
        open.angle = 0.95;
        let drive = motor_step(&mut open).unwrap();
        assert!((drive - 1.0).abs() < 1e-6);
        assert_eq!(open.mode, MotorMode::Idle);

        let mut ending = motor(MotorMode::Toggle);
        ending.countdown = 1;
        assert!(motor_step(&mut ending).is_some());
        assert_eq!(ending.mode, MotorMode::Idle);

        let mut barred = motor(MotorMode::Toggle);
        barred.angle = 1.0;
        barred.latches = true;
        let drive = motor_step(&mut barred).unwrap();
        assert!((drive - 1.0).abs() < 1e-6);
        assert!(barred.latched);
        assert_eq!(barred.mode, MotorMode::Toggle);
    }
}
