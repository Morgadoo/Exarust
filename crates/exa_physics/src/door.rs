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
//! A sliding door creaks from its stored energy. Past a squared 0.015 the
//! volume is 0.2 times the square root of the energy and the pitch is 0.2
//! times the energy plus 0.7, at most 1. A motorised open lowers the travel
//! fraction by 0.01 a tick, with smoothing 0.1, and stops once it is below
//! 0.01. A close raises it by 0.075, with smoothing 0.05, and latches at 1
//! once it passes 0.99. Shut and not held, the fraction stays 1 with
//! smoothing 0.5. Held below 0.995, the same pull of 4 unlatches on the
//! seventh tick from 20. A plain open jumps the fraction to 0.6 for 80
//! ticks. A frozen door keeps its stored fraction and stays passable. A
//! held hand point farther than 60 cm is pulled back to 60 cm.
//!
//! Evidence: the hinged door step, the open shove, and the sliding door
//! step. The partner door, the lock, and the slam's random pitch are not
//! in this slice.

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

/// Travel fraction 1 is shut and 0 is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlideMode {
    Idle,
    /// The close command has arrived.
    Shut,
    Open,
    Close,
}

/// One tick of a sliding door.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slide {
    pub mode: SlideMode,
    pub countdown: i32,
    pub frozen: bool,
    pub motorised: bool,
    pub locked: bool,
    /// Bars itself when it finishes shutting.
    pub latches: bool,
    pub latched: bool,
    pub closed: bool,
    pub holding: bool,
    /// The bottom knock has already fired until the fraction rises.
    pub knocked: bool,
    pub count: i32,
    pub fraction: f32,
    /// The fraction a frozen door keeps.
    pub parked: f32,
    pub smoothing: f32,
    /// Squared speed the creak reads.
    pub energy: f32,
}

/// What one sliding tick reports. The fraction and the smoothing stay on [`Slide`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlideTick {
    pub playing: bool,
    pub volume: f32,
    pub pitch: f32,
    pub passable: bool,
    pub unlatched: bool,
    pub slammed: bool,
    pub knocked: bool,
    pub knock_volume: f32,
    pub slam_volume: f32,
}

/// Creak stop, shared with a hinge.
const SLIDE_CREAK_PITCH: f32 = 0.2;
const SLIDE_CREAK_VOLUME: f32 = 0.2;
/// Smoothing while the door is free to travel.
pub const SLIDE_OPEN_SMOOTH: f32 = 0.01;
/// Smoothing while a motorised door opens.
pub const SLIDE_MOTOR_SMOOTH: f32 = 0.1;
/// Smoothing while a close command runs.
pub const SLIDE_CLOSE_SMOOTH: f32 = 0.05;
/// Smoothing while a shut door is held closed.
pub const SLIDE_HELD_SMOOTH: f32 = 0.5;
/// How far a motorised open moves the fraction in one tick.
pub const SLIDE_OPEN_STEP: f32 = 0.01;
/// How far a close command moves the fraction in one tick.
pub const SLIDE_CLOSE_STEP: f32 = 0.075;
/// A motorised open stops once the fraction is below this.
pub const SLIDE_OPEN_FLOOR: f32 = 0.01;
/// A close command latches once the fraction passes this.
pub const SLIDE_CLOSE_CEIL: f32 = 0.99;
/// At or above this, and not opening, the door is shut.
pub const SLIDE_SHUT: f32 = 0.995;
/// An open door can be walked through at or above this fraction.
pub const SLIDE_PASS: f32 = 0.8;
/// A plain open jumps here.
pub const SLIDE_JUMP: f32 = 0.6;
/// Ticks a plain open command lasts.
pub const SLIDE_OPEN_TICKS: i32 = 80;
/// Ticks a motorised open command lasts.
pub const SLIDE_MOTOR_TICKS: i32 = 300;
/// The bottom knock needs more stored energy than this.
pub const SLIDE_KNOCK_ENERGY: f32 = 0.05;
/// A held hand point is kept inside this many centimetres of the link.
pub const SLIDE_HAND_CM: f32 = 60.0;

fn slide_basis(energy: f32) -> f32 {
    if !energy.is_finite() || energy <= CREAK_STOP {
        0.003
    } else {
        energy.sqrt() * SLIDE_CREAK_VOLUME
    }
}

/// The creak of a sliding door from its stored energy.
pub fn slide_creak(energy: f32) -> SlideTick {
    if !energy.is_finite() || energy <= CREAK_STOP {
        return SlideTick {
            playing: false,
            volume: 0.0,
            pitch: 0.0,
            passable: false,
            unlatched: false,
            slammed: false,
            knocked: false,
            knock_volume: 0.0,
            slam_volume: 0.0,
        };
    }
    SlideTick {
        playing: true,
        volume: energy.sqrt() * SLIDE_CREAK_VOLUME,
        pitch: (energy * SLIDE_CREAK_PITCH + 0.7).min(1.0),
        passable: false,
        unlatched: false,
        slammed: false,
        knocked: false,
        knock_volume: 0.0,
        slam_volume: 0.0,
    }
}

fn end_slide(slide: &mut Slide, mode: SlideMode) {
    slide.mode = mode;
    slide.countdown = 0;
}

/// Pull a held hand point back to 60 cm. A point on that sphere stays.
/// A non-finite offset stays as it is.
pub fn slide_hand(offset: Vec3) -> Vec3 {
    let reach = offset.length_squared();
    if !reach.is_finite() || reach <= SLIDE_HAND_CM * SLIDE_HAND_CM {
        offset
    } else {
        offset * (SLIDE_HAND_CM / reach.sqrt())
    }
}

/// Start an open command. A plain door jumps to 0.6 for 80 ticks. A
/// motorised door crawls for 300 ticks.
pub fn slide_open(slide: &mut Slide) {
    slide.latched = false;
    slide.closed = false;
    slide.count = 0;
    slide.mode = SlideMode::Open;
    if slide.motorised {
        slide.countdown = SLIDE_MOTOR_TICKS;
    } else {
        slide.fraction = SLIDE_JUMP;
        slide.countdown = SLIDE_OPEN_TICKS;
    }
}

/// One tick. A frozen door keeps its parked fraction and stays passable.
pub fn slide_step(slide: &mut Slide) -> SlideTick {
    if slide.frozen {
        slide.fraction = slide.parked;
        return SlideTick {
            playing: false,
            volume: 0.0,
            pitch: 0.0,
            passable: true,
            unlatched: false,
            slammed: false,
            knocked: false,
            knock_volume: 0.0,
            slam_volume: 0.0,
        };
    }

    let sound = slide_creak(slide.energy);
    let basis = slide_basis(slide.energy);
    let mut knocked = false;
    let mut knock_volume = 0.0;
    let risen = slide.fraction >= SLIDE_OPEN_FLOOR;
    if risen {
        slide.knocked = false;
    } else if !slide.knocked && slide.energy.is_finite() && slide.energy > SLIDE_KNOCK_ENERGY {
        knocked = true;
        knock_volume = basis * 2.0 * basis * 2.0 * 0.5;
        slide.knocked = true;
    } else {
        slide.knocked = true;
    }

    let mut slammed = false;
    let mut slam_volume = 0.0;
    let mut unlatched = false;
    let passable = if !slide.closed {
        let passable = !slide.locked && slide.fraction >= SLIDE_PASS;
        slide.smoothing = SLIDE_OPEN_SMOOTH;
        if slide.mode == SlideMode::Open {
            if slide.countdown < 2 {
                end_slide(slide, SlideMode::Idle);
            } else {
                slide.countdown -= 1;
            }
            if slide.motorised {
                slide.smoothing = SLIDE_MOTOR_SMOOTH;
                slide.fraction -= SLIDE_OPEN_STEP;
                if slide.fraction < SLIDE_OPEN_FLOOR {
                    slide.fraction = 0.0;
                    end_slide(slide, SlideMode::Idle);
                }
            }
        }
        if slide.mode == SlideMode::Close {
            if slide.countdown < 2 {
                end_slide(slide, SlideMode::Idle);
            } else {
                slide.countdown -= 1;
            }
            slide.smoothing = SLIDE_CLOSE_SMOOTH;
            slide.fraction += SLIDE_CLOSE_STEP;
            if slide.fraction > SLIDE_CLOSE_CEIL {
                slide.fraction = 1.0;
                end_slide(slide, SlideMode::Shut);
                slide.latched = true;
            }
        }
        if slide.fraction >= SLIDE_SHUT && slide.mode != SlideMode::Open {
            slide.closed = true;
            slide.count = PULL_CAP;
            if slide.latches {
                slide.latched = true;
            }
            slammed = true;
            slam_volume = basis * 2.0 * basis * 2.0 * 0.75;
        }
        passable
    } else {
        if slide.count < PULL_CAP {
            slide.count += 1;
        }
        if !slide.holding || slide.latched {
            slide.fraction = 1.0;
            slide.smoothing = SLIDE_HELD_SMOOTH;
        } else if slide.fraction < SLIDE_SHUT {
            slide.count -= PULL_STEP;
            if slide.count < 0 {
                unlatched = true;
                slide.closed = false;
                slide.smoothing = SLIDE_OPEN_SMOOTH;
                slide.count = 0;
            }
        }
        !slide.locked
    };

    SlideTick {
        playing: sound.playing,
        volume: sound.volume,
        pitch: sound.pitch,
        passable,
        unlatched,
        slammed,
        knocked,
        knock_volume,
        slam_volume,
    }
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

    fn slide() -> Slide {
        Slide {
            mode: SlideMode::Idle,
            countdown: 0,
            frozen: false,
            motorised: false,
            locked: false,
            latches: false,
            latched: false,
            closed: false,
            holding: false,
            knocked: false,
            count: 0,
            fraction: 1.0,
            parked: 1.0,
            smoothing: 0.0,
            energy: 0.0,
        }
    }

    #[test]
    fn a_sliding_door_creaks_opens_and_the_seventh_pull_unlatches() {
        let mut moving = slide();
        moving.energy = 0.25;
        let tick = slide_step(&mut moving);
        assert!(tick.playing);
        assert!((tick.volume - 0.1).abs() < 1e-5, "{}", tick.volume);
        assert!((tick.pitch - 0.75).abs() < 1e-5, "{}", tick.pitch);

        let mut quiet = slide();
        quiet.energy = CREAK_STOP;
        assert!(!slide_step(&mut quiet).playing);

        let mut opening = slide();
        opening.motorised = true;
        opening.fraction = SLIDE_OPEN_FLOOR + 0.005;
        opening.countdown = 10;
        slide_open(&mut opening);
        assert_eq!(opening.countdown, SLIDE_MOTOR_TICKS);
        assert!((opening.fraction - (SLIDE_OPEN_FLOOR + 0.005)).abs() < 1e-6);
        let tick = slide_step(&mut opening);
        assert!((opening.fraction - 0.0).abs() < 1e-6);
        assert!((opening.smoothing - SLIDE_MOTOR_SMOOTH).abs() < 1e-6);
        assert_eq!(opening.mode, SlideMode::Idle);
        assert!(!tick.passable);

        let mut closing = slide();
        closing.mode = SlideMode::Close;
        closing.countdown = 10;
        closing.fraction = 0.95;
        closing.energy = 0.25;
        let tick = slide_step(&mut closing);
        assert!((closing.fraction - 1.0).abs() < 1e-6);
        assert!((closing.smoothing - SLIDE_CLOSE_SMOOTH).abs() < 1e-6);
        assert!(closing.latched);
        assert!(closing.closed);
        assert_eq!(closing.mode, SlideMode::Shut);
        assert!(tick.slammed);
        assert!(
            (tick.slam_volume - 0.03).abs() < 1e-5,
            "{}",
            tick.slam_volume
        );

        let mut shut = slide();
        shut.closed = true;
        shut.fraction = 0.4;
        let tick = slide_step(&mut shut);
        assert!((shut.fraction - 1.0).abs() < 1e-6);
        assert!((shut.smoothing - SLIDE_HELD_SMOOTH).abs() < 1e-6);
        assert!(tick.passable);

        let mut held = slide();
        held.closed = true;
        held.holding = true;
        held.count = PULL_CAP;
        held.fraction = 0.9;
        let mut seen = [0; 6];
        for slot in &mut seen {
            let tick = slide_step(&mut held);
            assert!(!tick.unlatched);
            *slot = held.count;
        }
        assert_eq!(seen, [16, 13, 10, 7, 4, 1]);
        let tick = slide_step(&mut held);
        assert!(tick.unlatched);
        assert!(!held.closed);
        assert_eq!(held.count, 0);
        assert!((held.smoothing - SLIDE_OPEN_SMOOTH).abs() < 1e-6);

        let mut plain = slide();
        plain.fraction = 1.0;
        slide_open(&mut plain);
        assert!((plain.fraction - SLIDE_JUMP).abs() < 1e-6);
        assert_eq!(plain.countdown, SLIDE_OPEN_TICKS);
        assert_eq!(plain.mode, SlideMode::Open);
        let before = plain.fraction;
        slide_step(&mut plain);
        assert!((plain.fraction - before).abs() < 1e-6);
        assert_eq!(plain.countdown, SLIDE_OPEN_TICKS - 1);

        let mut parked = slide();
        parked.frozen = true;
        parked.parked = 0.4;
        parked.fraction = 0.1;
        parked.energy = 0.25;
        let tick = slide_step(&mut parked);
        assert!((parked.fraction - 0.4).abs() < 1e-6);
        assert!(tick.passable);
        assert!(!tick.playing);

        let hand = slide_hand(Vec3::new(100.0, 0.0, 0.0));
        assert!(hand.abs_diff_eq(Vec3::new(SLIDE_HAND_CM, 0.0, 0.0), 1e-4));
        let kept = slide_hand(Vec3::new(SLIDE_HAND_CM, 0.0, 0.0));
        assert!(kept.abs_diff_eq(Vec3::new(SLIDE_HAND_CM, 0.0, 0.0), 1e-4));

        let mut low = slide();
        low.fraction = 0.0;
        low.energy = 0.01;
        assert!(!slide_step(&mut low).knocked);
        assert!(low.knocked);
        low.energy = 0.25;
        assert!(!slide_step(&mut low).knocked);
        low.fraction = SLIDE_OPEN_FLOOR;
        slide_step(&mut low);
        assert!(!low.knocked);
        low.fraction = 0.0;
        let tick = slide_step(&mut low);
        assert!(tick.knocked);
        assert!(
            (tick.knock_volume - 0.02).abs() < 1e-5,
            "{}",
            tick.knock_volume
        );

        let mut wide = slide();
        wide.fraction = SLIDE_PASS;
        assert!(slide_step(&mut wide).passable);
        wide.fraction = SLIDE_PASS - 0.01;
        wide.closed = false;
        assert!(!slide_step(&mut wide).passable);
    }
}
