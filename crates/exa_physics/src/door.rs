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
//! A paired leaf is not held shut while its partner is more than 0.1 rad
//! short of closed, and it leaves the shut state when its own angle is
//! short of closed. A single door leaves the shut state only once it is
//! more than 0.05 rad short, and it is still held. A paired door adds the
//! pass-through bits to its layer.
//!
//! An unlock clears the saved latch and waits 1000 ms. At that exact time
//! the door is still latched. One millisecond later both leaves open. Which
//! key fits the door is not in this slice.
//!
//! Close-and-lock on a motorised hinge sets the fraction to 1 and drives
//! toward shut for 300 ticks, or 100 when fast. A sliding leaf plays the
//! lock sound when it is not latched, and a fast one drives toward shut for
//! 100 ticks. A leaf that is already shut becomes latched.
//!
//! A signalled open plays the unlock sound once for the doors under a node.
//! A latched or motorised door waits 120 ticks. Below 2 the wait ends by
//! opening: the latch, the shut flag and the pull count clear. A plain
//! hinge is shoved 3 cm and goes idle. A motorised hinge aims open for 100
//! ticks. A plain slider jumps to 0.6 for 80 ticks. A motorised slider
//! drives open for 300 ticks.
//!
//! Evidence: the hinged door step, the open shove, the sliding door step,
//! the partner check, and the unlock wait. The slam's random pitch is not
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

/// A single shut door leaves once it is this far short of closed, in radians.
pub const SINGLE_LEAVE: f32 = 0.05;
/// A leaf counts as near closed within this many radians.
pub const NEAR_CLOSED: f32 = 0.1;
/// Layer bits a paired leaf sets so the two leaves pass through each other.
pub const PAIR_LAYER: u32 = 0x3000;

/// One shut leaf, and whether its partner is near closed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pair {
    pub paired: bool,
    pub partner_near: bool,
    pub angle: f32,
    pub closed: f32,
    pub latched: bool,
}

/// Whether the leaf stays shut, and whether it is driven toward closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairTick {
    pub shut: bool,
    pub hold: bool,
}

/// Near closed: within 0.1 rad while at or short of it, and always once past it.
/// A non-finite angle is not near.
pub fn near_shut(angle: f32, closed: f32) -> bool {
    if !angle.is_finite() || !closed.is_finite() {
        return false;
    }
    if angle <= closed {
        closed - NEAR_CLOSED < angle
    } else {
        true
    }
}

/// The shut check for one leaf. A single door still holds. A paired door
/// does not hold while its partner is away, and it leaves when its own
/// angle is short of closed.
pub fn pair_shut(door: &Pair) -> PairTick {
    if !door.paired {
        let leave = door.angle < door.closed - SINGLE_LEAVE && !door.latched;
        PairTick {
            shut: !leave,
            hold: true,
        }
    } else if !door.partner_near {
        let short = door.angle < door.closed;
        PairTick {
            shut: !short,
            hold: false,
        }
    } else {
        PairTick {
            shut: true,
            hold: true,
        }
    }
}

/// Add the pass-through bits when this leaf has a partner.
pub fn pair_layer(layer: u32, paired: bool) -> u32 {
    if paired { layer | PAIR_LAYER } else { layer }
}

/// How long a door stays latched after an unlock, in milliseconds.
pub const UNLOCK_WAIT_MS: i32 = 1000;

/// A door's latch and the wait before it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Latch {
    pub latched: bool,
    /// The bit stored with the node.
    pub saved: bool,
    pub pending: bool,
    pub ready_at_ms: i32,
}

/// Clear the saved latch and start the wait. The door stays latched.
/// A partner gets the same wait and keeps its own saved bit.
pub fn begin_unlock(door: &mut Latch, partner: Option<&mut Latch>, now_ms: i32) {
    door.saved = false;
    door.pending = true;
    door.ready_at_ms = now_ms.saturating_add(UNLOCK_WAIT_MS);
    if let Some(partner) = partner {
        partner.pending = true;
        partner.ready_at_ms = now_ms.saturating_add(UNLOCK_WAIT_MS);
    }
}

/// Open the latch once the clock is strictly past the wait. The partner's
/// latch opens in the same tick. At the exact wait, nothing changes.
pub fn unlock_due(door: &mut Latch, partner: Option<&mut Latch>, now_ms: i32) -> bool {
    if !door.pending || door.ready_at_ms >= now_ms {
        return false;
    }
    door.saved = false;
    door.latched = false;
    door.pending = false;
    if let Some(partner) = partner {
        partner.latched = false;
    }
    true
}

/// How a close-and-lock drives the leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutDrive {
    Idle,
    /// Aim at the shut fraction.
    TowardShut,
}

/// One leaf under a close-and-lock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShutLeaf {
    pub sliding: bool,
    pub motorised: bool,
    pub fast: bool,
    pub latched: bool,
    pub closed: bool,
    pub fraction: f32,
    pub drive: ShutDrive,
    pub countdown: i32,
}

/// Ticks a slow motorised hinge spends driving shut.
pub const SHUT_SLOW: i32 = 300;
/// Ticks a fast leaf spends driving shut.
pub const SHUT_FAST: i32 = 100;

/// Close and lock one leaf. Returns whether the lock sound plays.
pub fn close_and_lock(leaf: &mut ShutLeaf) -> bool {
    let mut sound = false;
    if !leaf.sliding {
        if leaf.motorised {
            leaf.fraction = 1.0;
            leaf.drive = ShutDrive::TowardShut;
            leaf.countdown = if leaf.fast { SHUT_FAST } else { SHUT_SLOW };
        }
    } else {
        if !leaf.latched {
            sound = true;
        }
        if leaf.fast {
            leaf.drive = ShutDrive::TowardShut;
            leaf.countdown = SHUT_FAST;
        }
    }
    if leaf.closed {
        leaf.latched = true;
    }
    sound
}

/// Ticks a signalled open waits before it opens.
pub const SIGNAL_WAIT: i32 = 120;

/// Whether a signalled open is waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalDrive {
    Idle,
    /// Waiting to open.
    Pending,
    /// Driving open.
    Open,
    /// A hinge aims at its fraction. A slider closes.
    Toggle,
}

/// One door under a signalled open.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalLeaf {
    pub sliding: bool,
    pub latched: bool,
    pub motorised: bool,
    pub closed: bool,
    pub count: i32,
    pub fraction: f32,
    pub angle: f32,
    pub closed_angle: f32,
    pub fast: bool,
    pub drive: SignalDrive,
    pub countdown: i32,
}

/// What the open command did. The shove is in the door's local frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opened {
    pub shove: Vec3,
    pub woke: bool,
}

/// Ticks a motorised hinge spends driving open.
pub const HINGE_OPEN_TICKS: i32 = 100;

/// Arm the doors under a node. The unlock sound plays once when there is
/// at least one door. Only a latched or motorised door waits.
pub fn signal_open(leaves: &mut [SignalLeaf]) -> bool {
    if leaves.is_empty() {
        return false;
    }
    for leaf in leaves.iter_mut() {
        if leaf.latched || leaf.motorised {
            leaf.drive = SignalDrive::Pending;
            leaf.countdown = SIGNAL_WAIT;
        }
    }
    true
}

/// The open a finished wait calls. The latch, the shut flag and the pull
/// count clear. A hinge is shoved 3 cm along local Z. A slider wakes.
pub fn begin_open(leaf: &mut SignalLeaf) -> Opened {
    leaf.latched = false;
    leaf.closed = false;
    leaf.count = 0;
    if !leaf.sliding {
        if leaf.motorised {
            leaf.fraction = 0.0;
            leaf.drive = SignalDrive::Open;
            leaf.countdown = HINGE_OPEN_TICKS;
        } else {
            leaf.drive = SignalDrive::Idle;
            leaf.countdown = 0;
        }
        Opened {
            shove: open_push(Vec3::Z),
            woke: false,
        }
    } else {
        leaf.drive = SignalDrive::Open;
        if leaf.motorised {
            leaf.countdown = SLIDE_MOTOR_TICKS;
        } else {
            leaf.fraction = SLIDE_JUMP;
            leaf.countdown = SLIDE_OPEN_TICKS;
        }
        Opened {
            shove: Vec3::ZERO,
            woke: true,
        }
    }
}

/// How open a door looks to a toggle. A slider is 1 minus its travel. A
/// hinge is `(angle − closed) · −0.7`.
pub const TOGGLE_OPEN: f32 = 0.7;
/// Scale from a hinge's angle gap to that openness.
pub const TOGGLE_HINGE: f32 = -0.7;
/// Ticks a slow hinge toggle runs.
pub const TOGGLE_SLOW: i32 = 100;
/// Ticks a fast hinge toggle runs, and every slider toggle.
pub const TOGGLE_FAST: i32 = 60;
/// Ticks a toggle wakes the door for.
pub const TOGGLE_WAKE: i32 = 256;

/// What a toggle did, after any latched open.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Toggled {
    pub shove: Vec3,
    pub wake_ticks: i32,
}

pub fn open_span(leaf: &SignalLeaf) -> f32 {
    if leaf.sliding {
        1.0 - leaf.fraction
    } else {
        (leaf.angle - leaf.closed_angle) * TOGGLE_HINGE
    }
}

/// Toggle one door. A latched door is opened first, then the command is chosen
/// from how open it is and the door is woken.
pub fn toggle(leaf: &mut SignalLeaf) -> Toggled {
    let mut shove = Vec3::ZERO;
    if leaf.latched {
        shove = begin_open(leaf).shove;
    }
    let span = open_span(leaf);
    if leaf.sliding {
        if span > TOGGLE_OPEN {
            leaf.drive = SignalDrive::Toggle;
            leaf.countdown = TOGGLE_FAST;
        } else {
            leaf.drive = SignalDrive::Open;
            leaf.countdown = TOGGLE_FAST;
            leaf.closed = false;
        }
    } else {
        leaf.fraction = if span > TOGGLE_OPEN { 1.0 } else { 0.0 };
        leaf.drive = SignalDrive::Toggle;
        leaf.countdown = if leaf.fast { TOGGLE_FAST } else { TOGGLE_SLOW };
    }
    Toggled {
        shove,
        wake_ticks: TOGGLE_WAKE,
    }
}

/// One tick of a waiting open. Below 2 the wait ends by opening.
pub fn signal_wait(leaf: &mut SignalLeaf) -> Option<Opened> {
    if leaf.drive != SignalDrive::Pending {
        return None;
    }
    if leaf.countdown < 2 {
        Some(begin_open(leaf))
    } else {
        leaf.countdown -= 1;
        None
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

    fn pair(angle: f32) -> Pair {
        Pair {
            paired: false,
            partner_near: false,
            angle,
            closed: 1.0,
            latched: false,
        }
    }

    #[test]
    fn a_partner_is_not_held_while_the_other_leaf_is_open() {
        let on_the_line = pair_shut(&pair(1.0 - SINGLE_LEAVE));
        assert!(on_the_line.shut);
        assert!(on_the_line.hold);

        let mut single = pair(1.0 - SINGLE_LEAVE - 0.01);
        let left = pair_shut(&single);
        assert!(!left.shut);
        assert!(left.hold);
        single.latched = true;
        let barred = pair_shut(&single);
        assert!(barred.shut);
        assert!(barred.hold);

        let mut open_partner = pair(1.0 - 0.01);
        open_partner.paired = true;
        let released = pair_shut(&open_partner);
        assert!(!released.shut);
        assert!(!released.hold);
        open_partner.angle = 1.0;
        let level = pair_shut(&open_partner);
        assert!(level.shut);
        assert!(!level.hold);

        let mut near = pair(1.0 - 0.2);
        near.paired = true;
        near.partner_near = true;
        let held = pair_shut(&near);
        assert!(held.shut);
        assert!(held.hold);

        assert!(!near_shut(1.0 - NEAR_CLOSED, 1.0));
        assert!(near_shut(1.0 - NEAR_CLOSED + 0.01, 1.0));
        assert!(near_shut(1.0 + 0.2, 1.0));
        assert!(!near_shut(f32::NAN, 1.0));

        assert_eq!(pair_layer(0, true), PAIR_LAYER);
        assert_eq!(pair_layer(PAIR_LAYER, true), PAIR_LAYER);
        assert_eq!(pair_layer(1, false), 1);
    }

    fn latch() -> Latch {
        Latch {
            latched: true,
            saved: true,
            pending: false,
            ready_at_ms: 0,
        }
    }

    #[test]
    fn an_unlock_waits_a_second_then_opens_both_leaves() {
        let mut door = latch();
        let mut partner = latch();
        begin_unlock(&mut door, Some(&mut partner), 0);
        assert!(!door.saved);
        assert!(door.latched);
        assert!(door.pending);
        assert_eq!(door.ready_at_ms, UNLOCK_WAIT_MS);
        assert!(partner.saved);
        assert!(partner.latched);
        assert!(partner.pending);
        assert_eq!(partner.ready_at_ms, UNLOCK_WAIT_MS);

        assert!(!unlock_due(&mut door, Some(&mut partner), UNLOCK_WAIT_MS));
        assert!(door.latched);
        assert!(partner.latched);

        assert!(unlock_due(
            &mut door,
            Some(&mut partner),
            UNLOCK_WAIT_MS + 1
        ));
        assert!(!door.latched);
        assert!(!door.pending);
        assert!(!door.saved);
        assert!(!partner.latched);
        assert!(partner.pending);
    }

    fn shut_leaf() -> ShutLeaf {
        ShutLeaf {
            sliding: false,
            motorised: false,
            fast: false,
            latched: false,
            closed: false,
            fraction: 0.0,
            drive: ShutDrive::Idle,
            countdown: 0,
        }
    }

    #[test]
    fn a_close_and_lock_drives_a_motor_shut_and_bars_a_shut_leaf() {
        let mut hinge = shut_leaf();
        hinge.motorised = true;
        hinge.fraction = 0.2;
        assert!(!close_and_lock(&mut hinge));
        assert!((hinge.fraction - 1.0).abs() < 1e-6);
        assert_eq!(hinge.drive, ShutDrive::TowardShut);
        assert_eq!(hinge.countdown, SHUT_SLOW);
        assert!(!hinge.latched);

        let mut fast = shut_leaf();
        fast.motorised = true;
        fast.fast = true;
        close_and_lock(&mut fast);
        assert_eq!(fast.countdown, SHUT_FAST);

        let mut plain = shut_leaf();
        plain.closed = true;
        assert!(!close_and_lock(&mut plain));
        assert_eq!(plain.drive, ShutDrive::Idle);
        assert!(plain.latched);

        let mut slide = shut_leaf();
        slide.sliding = true;
        assert!(close_and_lock(&mut slide));
        assert_eq!(slide.drive, ShutDrive::Idle);
        assert!(!slide.latched);

        let mut quick = shut_leaf();
        quick.sliding = true;
        quick.fast = true;
        quick.closed = true;
        assert!(close_and_lock(&mut quick));
        assert_eq!(quick.drive, ShutDrive::TowardShut);
        assert_eq!(quick.countdown, SHUT_FAST);
        assert!(quick.latched);

        let mut barred = shut_leaf();
        barred.sliding = true;
        barred.latched = true;
        assert!(!close_and_lock(&mut barred));
        assert!(barred.latched);
    }

    fn signal_leaf() -> SignalLeaf {
        SignalLeaf {
            sliding: false,
            latched: false,
            motorised: false,
            closed: true,
            count: PULL_CAP,
            fraction: 1.0,
            angle: 0.0,
            closed_angle: 0.0,
            fast: false,
            drive: SignalDrive::Idle,
            countdown: 0,
        }
    }

    fn wait_out(leaf: &mut SignalLeaf) -> Opened {
        signal_open(std::slice::from_mut(leaf));
        let mut opened = None;
        for _ in 0..SIGNAL_WAIT {
            opened = signal_wait(leaf);
            if opened.is_some() {
                break;
            }
        }
        opened.expect("the wait opens")
    }

    #[test]
    fn a_signalled_open_waits_120_ticks_on_a_latched_door() {
        assert!(!signal_open(&mut []));

        let mut plain = signal_leaf();
        assert!(signal_open(std::slice::from_mut(&mut plain)));
        assert_eq!(plain.drive, SignalDrive::Idle);
        assert!(signal_wait(&mut plain).is_none());

        let mut latched = signal_leaf();
        latched.latched = true;
        let mut motor = signal_leaf();
        motor.motorised = true;
        let mut leaves = [plain, latched, motor];
        assert!(signal_open(&mut leaves));
        assert_eq!(leaves[0].drive, SignalDrive::Idle);
        assert_eq!(leaves[1].countdown, SIGNAL_WAIT);
        assert_eq!(leaves[1].drive, SignalDrive::Pending);
        assert_eq!(leaves[2].countdown, SIGNAL_WAIT);

        let mut waiting = leaves[1];
        assert!(signal_wait(&mut waiting).is_none());
        assert_eq!(waiting.countdown, SIGNAL_WAIT - 1);
        assert_eq!(waiting.drive, SignalDrive::Pending);

        let mut ticks = 0;
        let mut armed = signal_leaf();
        armed.latched = true;
        signal_open(std::slice::from_mut(&mut armed));
        loop {
            ticks += 1;
            if signal_wait(&mut armed).is_some() {
                break;
            }
            assert!(ticks < SIGNAL_WAIT + 2);
        }
        assert_eq!(ticks, SIGNAL_WAIT);
        assert_eq!(armed.drive, SignalDrive::Idle);
        assert_eq!(armed.countdown, 0);
        assert!(!armed.latched);
        assert!(!armed.closed);
        assert_eq!(armed.count, 0);

        let mut hinge = signal_leaf();
        hinge.latched = true;
        hinge.motorised = true;
        hinge.fraction = 0.4;
        let opened = wait_out(&mut hinge);
        assert!((hinge.fraction - 0.0).abs() < 1e-6);
        assert_eq!(hinge.drive, SignalDrive::Open);
        assert_eq!(hinge.countdown, HINGE_OPEN_TICKS);
        assert!(
            opened
                .shove
                .abs_diff_eq(Vec3::new(0.0, 0.0, OPEN_PUSH_CM), 1e-4)
        );
        assert!(!opened.woke);
        assert!(!hinge.latched);

        let mut slider = signal_leaf();
        slider.sliding = true;
        slider.latched = true;
        slider.fraction = 1.0;
        let opened = wait_out(&mut slider);
        assert!((slider.fraction - SLIDE_JUMP).abs() < 1e-6);
        assert_eq!(slider.countdown, SLIDE_OPEN_TICKS);
        assert_eq!(slider.drive, SignalDrive::Open);
        assert!(opened.shove.abs_diff_eq(Vec3::ZERO, 1e-6));
        assert!(opened.woke);

        let mut motor_slide = signal_leaf();
        motor_slide.sliding = true;
        motor_slide.motorised = true;
        motor_slide.latched = true;
        motor_slide.fraction = 0.4;
        let opened = wait_out(&mut motor_slide);
        assert!((motor_slide.fraction - 0.4).abs() < 1e-6);
        assert_eq!(motor_slide.countdown, SLIDE_MOTOR_TICKS);
        assert!(opened.woke);
        assert!(opened.shove.abs_diff_eq(Vec3::ZERO, 1e-6));
    }

    #[test]
    fn a_toggle_opens_a_shut_hinge_and_closes_an_open_slider() {
        let mut shut = signal_leaf();
        shut.closed_angle = 1.0;
        shut.angle = 1.0;
        shut.fraction = 0.5;
        let done = toggle(&mut shut);
        assert!((shut.fraction - 0.0).abs() < 1e-6);
        assert_eq!(shut.drive, SignalDrive::Toggle);
        assert_eq!(shut.countdown, TOGGLE_SLOW);
        assert_eq!(done.wake_ticks, TOGGLE_WAKE);
        assert!(done.shove.abs_diff_eq(Vec3::ZERO, 1e-6));

        let mut on_the_line = signal_leaf();
        on_the_line.closed_angle = 1.0;
        on_the_line.angle = 0.0;
        let done = toggle(&mut on_the_line);
        assert!((on_the_line.fraction - 0.0).abs() < 1e-6);
        assert!(done.shove.abs_diff_eq(Vec3::ZERO, 1e-6));

        let mut swung = signal_leaf();
        swung.closed_angle = 1.0;
        swung.angle = 1.0 - 1.1;
        toggle(&mut swung);
        assert!((swung.fraction - 1.0).abs() < 1e-6);
        assert_eq!(swung.countdown, TOGGLE_SLOW);

        let mut fast = signal_leaf();
        fast.fast = true;
        toggle(&mut fast);
        assert_eq!(fast.countdown, TOGGLE_FAST);
        assert!((fast.fraction - 0.0).abs() < 1e-6);

        let mut open_slide = signal_leaf();
        open_slide.sliding = true;
        open_slide.fraction = 0.0;
        open_slide.closed = true;
        toggle(&mut open_slide);
        assert_eq!(open_slide.drive, SignalDrive::Toggle);
        assert_eq!(open_slide.countdown, TOGGLE_FAST);
        assert!(open_slide.closed);

        let mut shut_slide = signal_leaf();
        shut_slide.sliding = true;
        shut_slide.fraction = 0.3;
        shut_slide.closed = true;
        toggle(&mut shut_slide);
        assert_eq!(shut_slide.drive, SignalDrive::Open);
        assert_eq!(shut_slide.countdown, TOGGLE_FAST);
        assert!(!shut_slide.closed);
        assert!((shut_slide.fraction - 0.3).abs() < 1e-6);

        let mut barred = signal_leaf();
        barred.latched = true;
        barred.fraction = 1.0;
        barred.angle = 0.0;
        barred.closed_angle = 0.0;
        let done = toggle(&mut barred);
        assert!(!barred.latched);
        assert!(!barred.closed);
        assert_eq!(barred.count, 0);
        assert!((barred.fraction - 0.0).abs() < 1e-6);
        assert_eq!(barred.drive, SignalDrive::Toggle);
        assert_eq!(barred.countdown, TOGGLE_SLOW);
        assert!(
            done.shove
                .abs_diff_eq(Vec3::new(0.0, 0.0, OPEN_PUSH_CM), 1e-4)
        );

        let mut barred_slide = signal_leaf();
        barred_slide.sliding = true;
        barred_slide.latched = true;
        barred_slide.fraction = 1.0;
        let done = toggle(&mut barred_slide);
        assert!(!barred_slide.latched);
        assert!((barred_slide.fraction - SLIDE_JUMP).abs() < 1e-6);
        assert_eq!(barred_slide.drive, SignalDrive::Open);
        assert_eq!(barred_slide.countdown, TOGGLE_FAST);
        assert!(done.shove.abs_diff_eq(Vec3::ZERO, 1e-6));
        assert_eq!(done.wake_ticks, TOGGLE_WAKE);
    }
}
