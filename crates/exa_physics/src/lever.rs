//! A lever: its creak, the detent it steps toward, and the signal when it arrives.
//!
//! The arm velocity is smoothed by half. The creak stops when the squared
//! speed is at most 0.015. Otherwise the volume is 0.2 times the square root
//! of that energy and the pitch is 0.2 times the energy plus 0.7, at most 1.
//!
//! Held, the target is the hand. Forced, it is the detent whose report is
//! off. Returning, it is the stored detent. Otherwise it is the nearest.
//! The drive moves 0.02 rad toward that target and holds once the gap is
//! within 0.02.
//!
//! Arriving more than 0.1 rad from the last detent reports the rest flag at
//! the first detent and its opposite at the second. A report of 0 signals 0
//! unless the lever is inverted, and stops a return. A report of 1 signals
//! 1, or −1 when the lever is forced and inverted, and arms a timed return.
//! A linked light fades out on 0 and in on 1. A report of 0 with a light
//! does not signal.
//!
//! Evidence: the lever step. The return's later tick and the arm nudge are
//! not in this slice.

use bevy::math::Vec3;

/// Squared arm speed at or below which the creak stops.
pub const CREAK_STOP: f32 = 0.015;
/// How far the drive moves toward the target in one tick, in radians.
pub const DRIVE_STEP: f32 = 0.02;
/// The drive holds once the gap is within this, in radians.
pub const HOLD: f32 = 0.02;
/// A newly reached detent is more than this far from the last, in radians.
pub const DETENT_GAP: f32 = 0.1;
/// A linked light fades out at this rate.
pub const LIGHT_OUT: f32 = 1.0;
/// A linked light fades in at this rate.
pub const LIGHT_IN: f32 = -1.0;

/// What a newly reached detent tells its targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Off,
    On,
    /// Forced and inverted.
    Inverted,
}

/// One lever tick's aim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lever {
    pub smoothed: Vec3,
    pub velocity: Vec3,
    pub angle: f32,
    /// The first detent.
    pub detent0: f32,
    /// Radians from the first detent to the second.
    pub span: f32,
    /// The detent angle last reported.
    pub last: f32,
    pub held: bool,
    pub hand: f32,
    /// Held on the detent whose report is off.
    pub forced: bool,
    /// The first detent reports on.
    pub on_at_rest: bool,
    pub invert: bool,
    /// The second target is signalled too.
    pub second: bool,
    pub returning: bool,
    /// Detent index a return aims at.
    pub home: i32,
    pub has_light: bool,
    /// Milliseconds before a return. Zero stays.
    pub return_ms: i32,
    pub has_target: bool,
    pub has_second: bool,
}

/// What one tick reports. The smoothed velocity, the last detent and the
/// return flag stay on [`Lever`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeverTick {
    pub playing: bool,
    pub volume: f32,
    pub pitch: f32,
    pub drive: f32,
    pub target: f32,
    pub signal: Option<Signal>,
    pub first: bool,
    pub second: bool,
    /// Fade rate of a linked light, when this tick changes it.
    pub light: Option<f32>,
    /// A timed return was asked for. The lever is not yet returning.
    pub arm_return: bool,
}

fn target_of(lever: &Lever) -> f32 {
    if lever.held {
        lever.hand
    } else if lever.forced {
        if lever.on_at_rest {
            lever.detent0 + lever.span
        } else {
            lever.detent0
        }
    } else if lever.returning {
        lever.detent0 + lever.home as f32 * lever.span
    } else if !lever.span.is_finite() || lever.span == 0.0 {
        lever.detent0
    } else {
        let steps = ((lever.angle - lever.detent0) / lever.span).round();
        steps * lever.span + lever.detent0
    }
}

/// Value at the target, and the detent angle to remember. Between the two
/// detents there is no report.
fn report(lever: &Lever, target: f32) -> (Option<i32>, Option<f32>) {
    let second = lever.detent0 + lever.span;
    if lever.detent0 + DETENT_GAP <= target {
        if target <= second - DETENT_GAP {
            (None, None)
        } else {
            let value = if lever.on_at_rest { 0 } else { 1 };
            (Some(value), Some(second))
        }
    } else {
        let value = if lever.on_at_rest { 1 } else { 0 };
        (Some(value), Some(lever.detent0))
    }
}

/// One tick. The angle is the arm's angle this tick. The drive is where
/// the arm is nudged.
pub fn lever_step(lever: &mut Lever) -> LeverTick {
    let smoothed = lever.smoothed * 0.5 + lever.velocity * 0.5;
    lever.smoothed = smoothed;
    let energy = smoothed.length_squared();
    let (playing, volume, pitch) = if !energy.is_finite() || energy <= CREAK_STOP {
        (false, 0.0, 0.0)
    } else {
        (true, energy.sqrt() * 0.2, (energy * 0.2 + 0.7).min(1.0))
    };

    let target = target_of(lever);
    let error = target - lever.angle;
    let mut signal = None;
    let mut first = false;
    let mut second = false;
    let mut light = None;
    let mut arm_return = false;
    let drive = if error.is_finite() && error.abs() <= HOLD {
        if (target - lever.last).abs() > DETENT_GAP {
            let (value, landed) = report(lever, target);
            if let Some(at) = landed {
                lever.last = at;
            }
            match value {
                Some(0) => {
                    if lever.has_light {
                        light = Some(LIGHT_OUT);
                    } else if !lever.invert {
                        first = lever.has_target;
                        second = lever.second && lever.has_second;
                        if first || second {
                            signal = Some(Signal::Off);
                        }
                    }
                    lever.returning = false;
                }
                Some(1) => {
                    if lever.has_light {
                        light = Some(LIGHT_IN);
                    }
                    let which = if lever.invert && lever.forced {
                        Signal::Inverted
                    } else {
                        Signal::On
                    };
                    first = lever.has_target;
                    second = lever.second && lever.has_second;
                    if first || second {
                        signal = Some(which);
                    }
                    if lever.return_ms != 0 {
                        arm_return = true;
                        lever.returning = false;
                    }
                }
                _ => {}
            }
        }
        target
    } else {
        let sign = if error > 0.0 {
            1.0
        } else if error < 0.0 {
            -1.0
        } else {
            0.0
        };
        lever.angle + sign * DRIVE_STEP
    };

    LeverTick {
        playing,
        volume,
        pitch,
        drive,
        target,
        signal,
        first,
        second,
        light,
        arm_return,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lever(angle: f32) -> Lever {
        Lever {
            smoothed: Vec3::ZERO,
            velocity: Vec3::ZERO,
            angle,
            detent0: 0.0,
            span: 1.0,
            last: 0.0,
            held: false,
            hand: 0.0,
            forced: false,
            on_at_rest: false,
            invert: false,
            second: false,
            returning: false,
            home: 0,
            has_light: false,
            return_ms: 0,
            has_target: true,
            has_second: false,
        }
    }

    #[test]
    fn a_lever_creaks_and_steps_toward_the_nearest_detent() {
        let mut moving = lever(0.2);
        moving.velocity = Vec3::X;
        let tick = lever_step(&mut moving);
        assert!(tick.playing);
        assert!((tick.volume - 0.1).abs() < 1e-5, "{}", tick.volume);
        assert!((tick.pitch - 0.75).abs() < 1e-5, "{}", tick.pitch);
        assert!((tick.drive - 0.18).abs() < 1e-5, "{}", tick.drive);
        assert!(tick.signal.is_none());
        assert!(moving.smoothed.abs_diff_eq(Vec3::new(0.5, 0.0, 0.0), 1e-5));

        let mut quiet = lever(0.0);
        assert!(!lever_step(&mut quiet).playing);

        let mut off = lever(0.01);
        off.last = 1.0;
        let tick = lever_step(&mut off);
        assert!((tick.drive - 0.0).abs() < 1e-5);
        assert_eq!(tick.signal, Some(Signal::Off));
        assert!(tick.first);
        assert!(!tick.second);
        assert!((off.last - 0.0).abs() < 1e-5);
        assert!(!off.returning);

        let mut on = lever(0.99);
        on.return_ms = 500;
        let tick = lever_step(&mut on);
        assert_eq!(tick.signal, Some(Signal::On));
        assert!((on.last - 1.0).abs() < 1e-5);
        assert!(tick.arm_return);
        assert!(!on.returning);

        let mut held = lever(0.0);
        held.held = true;
        held.hand = 0.0;
        held.last = 1.0;
        held.on_at_rest = true;
        held.forced = true;
        held.invert = true;
        held.return_ms = 100;
        let tick = lever_step(&mut held);
        assert_eq!(tick.signal, Some(Signal::Inverted));
        assert!(tick.arm_return);
        assert!(!held.returning);

        let mut lamp = lever(0.0);
        lamp.last = 1.0;
        lamp.has_light = true;
        let tick = lever_step(&mut lamp);
        assert_eq!(tick.light, Some(LIGHT_OUT));
        assert!(tick.signal.is_none());

        let mut lit = lever(0.99);
        lit.has_light = true;
        lit.has_target = true;
        let tick = lever_step(&mut lit);
        assert_eq!(tick.light, Some(LIGHT_IN));
        assert_eq!(tick.signal, Some(Signal::On));

        let mut mid = lever(0.5);
        mid.held = true;
        mid.hand = 0.5;
        let tick = lever_step(&mut mid);
        assert!((tick.drive - 0.5).abs() < 1e-5);
        assert!(tick.signal.is_none());
        assert!((mid.last - 0.0).abs() < 1e-5);

        let mut close = lever(0.0);
        close.last = DETENT_GAP;
        let tick = lever_step(&mut close);
        assert!(tick.signal.is_none());
        assert!((close.last - DETENT_GAP).abs() < 1e-5);

        let mut back = lever(0.5);
        back.returning = true;
        back.home = 1;
        let tick = lever_step(&mut back);
        assert!((tick.target - 1.0).abs() < 1e-5);
        assert!((tick.drive - 0.52).abs() < 1e-5, "{}", tick.drive);

        let mut pushed = lever(0.4);
        pushed.forced = true;
        let tick = lever_step(&mut pushed);
        assert!((tick.target - 0.0).abs() < 1e-5);
        assert!((tick.drive - 0.38).abs() < 1e-5, "{}", tick.drive);
    }
}
