//! A door: its record, the hinge or slide its links describe, and the
//! state machine of a hinged or sliding leaf.
//!
//! The original's door body is `TRBDDoor`. Its links say what kind of leaf
//! it is: a link with slider flag 4 makes it slide, and its travel link is
//! the first whose body point is 30–160 cm up; a hinge's pins (slack ≤ 30
//! cm) give its axis and its longest link is the swing stop, with radius r
//! and half swing 2·asin(slack / 2r). A slider's travel is its link's
//! fraction (1 shut, 0 open); a hinge's angle is the stop's heading, which
//! is at the closed end when the stop link is taut and reads `half_swing`
//! in the leaf's frame. A hinge's range is −2·half swing times the swing
//! sign (−1 when the stop's body point has x > 0). A slip reads its
//! operative word: lock, bar, motor, fast motor and frozen flags, the four
//! sound letters and the key, plus the stored mode, countdown and latch.
//!
//! Each tick begins with a signalled open (mode 3): below 2 it opens, else
//! the countdown drops. The handle's velocity is smoothed by half. The
//! hinge creak stops at a squared 0.015; otherwise the speed is its root,
//! the volume 0.075 times the root of the speed, and the pitch 0.022 times
//! the speed plus 0.7, at most 1. The stop knocks when its link is stretched
//! past its slack, faster than 0.01 and off cooldown (200 ms plus up to 99
//! more): volume 20 times the creak volume squared. A finished unlock
//! clears the latch on both leaves.
//!
//! A motorised or toggling leaf aims at `(1 − fraction) · range · sign +
//! closed`. A plain step is `|error / range| · 0.001 + 0.0003`, a motor's
//! 0.0003, a fast one's 0.005. A toggle arrives within its step and either
//! bars itself or ends; an open command arrives within 0.1 rad.
//!
//! A leaf is shut when its angle passes closed (a hinge) or its travel
//! passes 0.995 (a slider); shutting sets the pull count to 20, bars a
//! latching leaf and slams (hinge volume 0.5·min(0.1·|handle|², 1); slider
//! 0.75·(0.4·√energy)²) after a 200 ms cooldown. An open leaf is passable
//! when unlocked and past closed − 0.4 rad (hinge) or travel 0.8 (slider).
//! A shut leaf is passable when unlocked; a plain hinge holds it at closed
//! unless it has left the mode's hold, and leaves the shut state when it is
//! 0.05 rad short of closed (a paired leaf when the partner is not near
//! closed). The pull count rises by 1 to 20; while a hand holds an unbarred
//! leaf short of closed it falls by 4, and below 0 the leaf unlatches with
//! the `opn` sound. A frozen slider keeps its parked travel and is
//! passable. An unlock waits 1000 ms, and at the exact time the leaf is
//! still latched.
//!
//! `open` shoves a hinge 3 cm along its facing, or jumps a plain slider to
//! 0.6; `toggle` chooses by openness ((angle − closed)·−0.7, or 1 −
//! travel) against 0.7; a signal wakes a latched or motorised leaf for 120
//! ticks. The sounds carry their random share as spreads.
//!
//! In the solver, the link laws push a leaf: a hand or body moving the stop
//! changes its heading, and the slide link's fraction is what the step
//! reads and writes. The per-tick inputs ([`Tick`]) are those link states;
//! a hinged leaf's body measures them and applies the drive
//! (`crate::body::door_body`).
//!
//! Evidence: the hinged and sliding steps, the open shove, the toggle, the
//! close-and-lock, the signalled open and wait, the unlock, the record and
//! the link geometry.

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
    // The plain step's share is taken in single precision and then scaled
    // and offset in double precision before it is rounded back; the open
    // command's 0.1 rad slop is a double too. Their single-precision
    // spellings differ by one unit in the last place at the boundary.
    let step = if !motor.motorised {
        let share = if motor.range.is_finite() && motor.range != 0.0 {
            (error / motor.range).abs()
        } else {
            0.0
        };
        (f64::from(share) * MOTOR_CATCH_WIDE + MOTOR_STEP_WIDE) as f32
    } else if motor.fast {
        FAST_STEP
    } else {
        MOTOR_STEP
    };
    let arrived = match active {
        MotorMode::Toggle => error.abs() <= step,
        MotorMode::Open => f64::from(error.abs()) <= OPEN_SLOP_WIDE,
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

// The thresholds and factors the original keeps as doubles. Each comparison
// or product below widens the single-precision value first, so these are
// spelled as doubles rather than widened from the `f32` constants above
// (`0.05_f32 as f64` is not `0.05`).
const MOTOR_CATCH_WIDE: f64 = 0.001;
const MOTOR_STEP_WIDE: f64 = 0.0003;
const OPEN_SLOP_WIDE: f64 = 0.1;
const CREAK_STOP_WIDE: f64 = 0.015;
const CLOSED_SLOP_WIDE: f64 = 0.04;
const NEAR_WIDE: f64 = 0.1;
const SINGLE_LEAVE_WIDE: f64 = 0.05;
const PASSABLE_WIDE: f64 = 0.4;
const SLIDE_FLOOR_WIDE: f64 = 0.01;
const SLIDE_CLOSE_STEP_WIDE: f64 = 0.075;
const SLIDE_CEIL_WIDE: f64 = 0.99;
const SLIDE_SHUT_WIDE: f64 = 0.995;
const SLIDE_PASS_WIDE: f64 = 0.8;
const SLIDE_KNOCK_WIDE: f64 = 0.05;
const KNOCK_SPEED_WIDE: f64 = 0.01;

/// Whether a squared handle speed makes a creak: strictly above 0.015 as a
/// double. A non-finite value is quiet, as the original's unordered compare
/// takes the quiet branch.
fn creaks(energy: f32) -> bool {
    f64::from(energy) > CREAK_STOP_WIDE
}

/// The hinge creak's volume and pitch for a speed (the root of the squared
/// handle speed): `0.075·√speed` and `min(0.022·speed + 0.7, 1)`, both
/// formed in double precision and rounded to single.
fn hinge_creak(speed: f32) -> (f32, f32) {
    let volume = (f64::from(speed.sqrt()) * 0.075) as f32;
    let pitch = (f64::from(speed) * 0.022 + 0.7) as f32;
    (volume, if pitch > 1.0 { 1.0 } else { pitch })
}

/// The creak of one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Creak {
    pub smoothed: Vec3,
    pub playing: bool,
    pub volume: f32,
    pub pitch: f32,
}

/// Smooth the handle velocity and say whether the hinge creaks. The
/// smoothing moves halfway from the old value toward the new one
/// (`s + (v − s)·0.5`, the original's form).
pub fn creak(smoothed: Vec3, handle_velocity: Vec3) -> Creak {
    let smoothed = smoothed + (handle_velocity - smoothed) * 0.5;
    let energy = smoothed.length_squared();
    if !creaks(energy) {
        return Creak {
            smoothed,
            playing: false,
            volume: 0.0,
            pitch: 0.0,
        };
    }
    let (volume, pitch) = hinge_creak(energy.sqrt());
    Creak {
        smoothed,
        playing: true,
        volume,
        pitch,
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
    /// Waiting out a signalled open.
    Wait,
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
    /// An unlock is waiting out its second.
    pub unlock_pending: bool,
    pub unlock_at_ms: i32,
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
    pub knock_pitch: f32,
    pub slam_volume: f32,
    pub slam_pitch: f32,
    /// The signalled wait ended with an open this tick.
    pub opened: bool,
}

/// Smoothing while the door is free to travel.
pub const SLIDE_OPEN_SMOOTH: f32 = 0.01;
/// Smoothing while a motorised door opens.
pub const SLIDE_MOTOR_SMOOTH: f32 = 0.1;
/// Smoothing while a close command runs.
pub const SLIDE_CLOSE_SMOOTH: f32 = 0.05;
/// Smoothing while a shut door is held closed.
pub const SLIDE_HELD_SMOOTH: f32 = 0.5;
/// The pitch a sliding sound is built from when the leaf is still.
pub const SLIDE_STOP_PITCH: f32 = 0.7;
/// The volume basis a sliding sound is built from when the leaf is still.
pub const SLIDE_STOP_BASIS: f32 = 0.003;
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

/// The creak's volume, and the basis a shut or knock volume is built from:
/// `0.2·√energy` formed in double precision.
fn slide_basis(energy: f32) -> f32 {
    if !creaks(energy) {
        SLIDE_STOP_BASIS
    } else {
        (f64::from(energy.sqrt()) * 0.2) as f32
    }
}

/// The creak's pitch, and the pitch a shut or knock sound is built from:
/// `min(0.2·energy + 0.7, 1)` formed in double precision.
fn slide_pitch(energy: f32) -> f32 {
    if !creaks(energy) {
        SLIDE_STOP_PITCH
    } else {
        let pitch = (f64::from(energy) * 0.2 + 0.7) as f32;
        if pitch > 1.0 { 1.0 } else { pitch }
    }
}

/// The creak of a sliding door from its stored energy.
pub fn slide_creak(energy: f32) -> SlideTick {
    if !creaks(energy) {
        return SlideTick {
            playing: false,
            volume: 0.0,
            pitch: 0.0,
            passable: false,
            unlatched: false,
            slammed: false,
            knocked: false,
            knock_volume: 0.0,
            knock_pitch: 0.0,
            slam_volume: 0.0,
            slam_pitch: 0.0,
            opened: false,
        };
    }
    SlideTick {
        playing: true,
        volume: slide_basis(energy),
        pitch: slide_pitch(energy),
        passable: false,
        unlatched: false,
        slammed: false,
        knocked: false,
        knock_volume: 0.0,
        knock_pitch: 0.0,
        slam_volume: 0.0,
        slam_pitch: 0.0,
        opened: false,
    }
}

/// A command the tick wrote to the stored word: the mode code and the
/// countdown as they were written (the word keeps the countdown of the last
/// command, not the one the tick has since counted down).
type Written = Option<(i32, i32)>;

fn end_slide(slide: &mut Slide, mode: SlideMode, written: &mut Written) {
    slide.mode = mode;
    slide.countdown = 0;
    *written = Some((slide_mode_code(mode), 0));
}

/// The stored-word mode code of a sliding leaf's mode.
fn slide_mode_code(mode: SlideMode) -> i32 {
    match mode {
        SlideMode::Idle => MODE_IDLE,
        SlideMode::Shut => MODE_SHUT,
        SlideMode::Wait => MODE_WAIT,
        SlideMode::Open => MODE_OPEN,
        SlideMode::Close => MODE_TOGGLE,
    }
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

/// One tick with no clock: no unlock is ever due.
pub fn slide_step(slide: &mut Slide) -> SlideTick {
    slide_tick(slide, i32::MIN)
}

/// One tick, in the original's order. A frozen door keeps its parked
/// fraction and stays passable.
pub fn slide_tick(slide: &mut Slide, now_ms: i32) -> SlideTick {
    let mut written = None;
    slide_tick_written(slide, now_ms, &mut written)
}

/// [`slide_tick`], also reporting the last command written to the stored
/// word this tick.
fn slide_tick_written(slide: &mut Slide, now_ms: i32, written: &mut Written) -> SlideTick {
    // A signalled open waits, then opens, in the same fields the motor uses.
    // The wait runs in the shared part of the door's tick, before the
    // sliding rules and so before the frozen check: a frozen leaf still
    // takes the open command (its travel is put back to the parked value
    // just below).
    let mut opened = false;
    if slide.mode == SlideMode::Wait {
        if slide.countdown < 2 {
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
            *written = Some((MODE_OPEN, slide.countdown));
            opened = true;
        } else {
            slide.countdown -= 1;
        }
    }

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
            knock_pitch: 0.0,
            slam_volume: 0.0,
            slam_pitch: 0.0,
            opened,
        };
    }

    let sound = slide_creak(slide.energy);
    let basis = slide_basis(slide.energy);
    let stopped_pitch = slide_pitch(slide.energy);
    // The knock and the slam take 0.1 off the creak pitch in double
    // precision; the roll's 0.2 is added on top.
    let struck_pitch = (f64::from(stopped_pitch) - 0.1) as f32;
    let mut knocked = false;
    let mut knock_volume = 0.0;
    // The bottom test is a double compare: a travel of exactly 0.01 in
    // single precision (0.00999999977…) is still below 0.01.
    let below = f64::from(slide.fraction) < SLIDE_FLOOR_WIDE;
    if !below {
        slide.knocked = false;
    } else if !slide.knocked && f64::from(slide.energy) > SLIDE_KNOCK_WIDE {
        knocked = true;
        knock_volume = basis * 2.0 * basis * 2.0 * 0.5;
        slide.knocked = true;
    } else {
        slide.knocked = true;
    }

    // A finished unlock clears the saved latch.
    if slide.unlock_pending && slide.unlock_at_ms < now_ms {
        slide.latched = false;
        slide.unlock_pending = false;
    }

    let mut slammed = false;
    let mut slam_volume = 0.0;
    let mut unlatched = false;
    let passable = if !slide.closed {
        let passable = !slide.locked && f64::from(slide.fraction) >= SLIDE_PASS_WIDE;
        slide.smoothing = SLIDE_OPEN_SMOOTH;
        if slide.mode == SlideMode::Open {
            if slide.countdown < 2 {
                end_slide(slide, SlideMode::Idle, written);
            } else {
                slide.countdown -= 1;
            }
            if slide.motorised {
                slide.smoothing = SLIDE_MOTOR_SMOOTH;
                slide.fraction = (f64::from(slide.fraction) - SLIDE_FLOOR_WIDE) as f32;
                if f64::from(slide.fraction) < SLIDE_FLOOR_WIDE {
                    slide.fraction = 0.0;
                    end_slide(slide, SlideMode::Idle, written);
                }
            }
        }
        if slide.mode == SlideMode::Close {
            if slide.countdown < 2 {
                end_slide(slide, SlideMode::Idle, written);
            } else {
                slide.countdown -= 1;
            }
            slide.smoothing = SLIDE_CLOSE_SMOOTH;
            slide.fraction = (f64::from(slide.fraction) + SLIDE_CLOSE_STEP_WIDE) as f32;
            if f64::from(slide.fraction) > SLIDE_CEIL_WIDE {
                slide.fraction = 1.0;
                end_slide(slide, SlideMode::Shut, written);
                slide.latched = true;
            }
        }
        if f64::from(slide.fraction) >= SLIDE_SHUT_WIDE && slide.mode != SlideMode::Open {
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
        } else if f64::from(slide.fraction) < SLIDE_SHUT_WIDE {
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
        knock_pitch: struck_pitch,
        slam_volume,
        slam_pitch: struck_pitch,
        opened,
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
        near_below(angle, closed)
    } else {
        true
    }
}

/// `closed − 0.1 < angle`, the near test, in double precision as the
/// original forms it.
fn near_below(angle: f32, closed: f32) -> bool {
    f64::from(closed) - NEAR_WIDE < f64::from(angle)
}

/// `angle < closed − 0.05`, a single leaf's leave test, in double precision.
/// A leaf whose single-precision angle is the nearest to 0.05 short
/// (0.94999999 against a closed heading of 1) is already past it.
fn leaves_single(angle: f32, closed: f32) -> bool {
    f64::from(closed) - SINGLE_LEAVE_WIDE > f64::from(angle)
}

/// The shut check for one leaf. A single door still holds. A paired door
/// does not hold while its partner is away, and it leaves when its own
/// angle is short of closed.
pub fn pair_shut(door: &Pair) -> PairTick {
    if !door.paired {
        let leave = leaves_single(door.angle, door.closed) && !door.latched;
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
    /// The unlatch sound the open plays.
    pub sound: Option<Sound>,
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
            sound: None,
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
            sound: None,
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
        hinge_openness(leaf.angle, leaf.closed_angle)
    }
}

/// `(angle − closed)·−0.7`: the gap is taken in single precision and
/// scaled by the double −0.7 before it is rounded back.
fn hinge_openness(angle: f32, closed: f32) -> f32 {
    (f64::from(angle - closed) * -0.7) as f32
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

/// A door record's flags.
pub const LATCHES: u32 = 0x2;
pub const LOCKED: u32 = 0x4;
pub const NOT_CLICKABLE: u32 = 0x8;
pub const MOTORISED: u32 = 0x10;
pub const FAST: u32 = 0x40;
pub const INVERTED: u32 = 0x80;
pub const FROZEN: u32 = 0x100;
pub const NODE_MARK: u32 = 0x200;

/// What the object database says about a door.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Record {
    pub flags: u32,
    /// The id of the key that unlocks it.
    pub key: u32,
    /// Sound letters: `a` plus the letter in the sound's name.
    pub lock_sound: u8,
    pub slam_sound: u8,
    pub hinge_sound: u8,
    pub unlatch_sound: u8,
}

impl Record {
    pub fn has(&self, flag: u32) -> bool {
        self.flags & flag != 0
    }
}

/// The name of a door sound (`doorb_slm` and so on).
pub fn sound_name(letter: u8, suffix: &str) -> String {
    format!("door{}_{suffix}", letter.wrapping_add(b'a') as char)
}

/// One link of a mechanism's body: a point on the body, written as weights
/// over its four rest points, the point it is tied to, and the slack it may
/// move within (centimetres). A slider (flag 4) stores its travel as slack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Link {
    pub on_body: [f32; 4],
    pub on_parent: [f32; 4],
    pub slack: f32,
    pub flags: u32,
}

impl Link {
    /// A slider.
    pub const SLIDER: u32 = 4;

    /// The body-local point the link holds.
    pub fn point(&self, rest: &[Vec3; 4]) -> Vec3 {
        rest[0] * self.on_body[0]
            + rest[1] * self.on_body[1]
            + rest[2] * self.on_body[2]
            + rest[3] * self.on_body[3]
    }

    /// The point the link is tied to (unused by the swing rules).
    pub fn anchor(&self, rest: &[Vec3; 4]) -> Vec3 {
        rest[0] * self.on_parent[0]
            + rest[1] * self.on_parent[1]
            + rest[2] * self.on_parent[2]
            + rest[3] * self.on_parent[3]
    }
}

/// Links shorter than this (slack, cm) are hinge pins.
pub const PIN_SLACK: f32 = 30.0;
/// A sliding door's travel link reaches this high on the body, in cm.
pub const TRAVEL_MIN_CM: f32 = 30.0;
pub const TRAVEL_MAX_CM: f32 = 160.0;

/// The hinge or slide a door's links describe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub sliding: bool,
    /// The link whose state is the leaf's travel, in link order.
    pub travel_link: usize,
    /// −1 when the stop's body point lies on the positive x side.
    pub sign: f32,
    /// Distance from the hinge to the stop, in centimetres.
    pub radius: f32,
    /// Half the swing, in radians.
    pub half_swing: f32,
    /// The stop link's slack, in centimetres.
    pub stop_slack: f32,
    /// The hinge axis in the body's frame.
    pub axis: Vec3,
}

impl Shape {
    /// The angle a shut leaf reads.
    ///
    /// The closed heading is the heading of the stop's anchor (its tied
    /// point less the node's position) in the leaf's frame, plus the half
    /// swing. The frame is built from that same anchor: its up row is the
    /// negated hinge axis, its forward row is the anchor with its part
    /// along that row removed (normalised), and its side row is up × forward.
    /// A heading is measured in the frame's side–forward plane from the
    /// forward row (the arccosine of the forward share, negated when the
    /// side share is negative), so the anchor reads zero by construction and
    /// the closed heading is exactly the half swing. Whether a leaf at its
    /// file position reads closed is a property of the data (a stop link
    /// taut on the shut side), not of this rule.
    pub fn closed(&self) -> f32 {
        self.half_swing
    }

    /// The angle the leaf travels through, from shut to open.
    pub fn range(&self) -> f32 {
        -2.0 * self.half_swing * self.sign
    }
}

/// The half swing a link of this slack allows at this radius, in radians:
/// twice the arcsine of half the slack over the radius, with the engine's
/// fast arcsine (the same one the lever takes).
pub fn half_swing(slack: f32, radius: f32) -> f32 {
    crate::lever::engine_asin(slack * 0.5 / radius) * 2.0
}

/// A body's links as the door's hinge or slide: the direction of the last
/// qualifying hinge pin (straight up when none qualifies), the last swing
/// stop (its index, radius and slack) with the sign any stop set, or the
/// travel link of a slider. A hinge with no stop keeps a guessed half swing
/// of pi.
pub fn shape(rest: &[Vec3; 4], links: &[Link]) -> Shape {
    let sliding = links.iter().any(|l| l.flags & Link::SLIDER != 0);
    if sliding {
        // The stop's file position: the first link whose body point sits
        // between 30 and 160 cm up.
        let travel = links
            .iter()
            .position(|l| {
                let y = l.point(rest).y;
                TRAVEL_MIN_CM < y && y < TRAVEL_MAX_CM
            })
            .unwrap_or(0);
        return Shape {
            sliding: true,
            travel_link: travel,
            sign: 1.0,
            radius: 0.0,
            half_swing: 0.0,
            stop_slack: 0.0,
            axis: Vec3::X,
        };
    }

    // The axis starts straight up. Each qualifying pin simply replaces it
    // with its own direction, normalised: the first pin point farther than
    // 1 cm from the body's origin, then any later one that is not below it
    // (y ≥ 0). There is no hemisphere step; a later pin pointing the other
    // way flips the axis. (The original normalises with the fast reciprocal
    // square root, so its axis is unit only to about twelve bits.)
    let mut axis = Vec3::Y;
    let mut pinned = false;
    let mut stop = None;
    // The sign starts at 1 and any stop whose point has x > 0 sets it to −1;
    // a later stop does not set it back.
    let mut sign = 1.0;
    for (i, link) in links.iter().enumerate() {
        let point = link.point(rest);
        if link.slack <= PIN_SLACK {
            // The squared length is summed z, y, x, as the original does.
            let squared = (point.z * point.z + point.y * point.y) + point.x * point.x;
            if squared > 1.0 && (!pinned || point.y >= 0.0) {
                let length = point.length();
                axis = if length > 0.0 {
                    point / length
                } else {
                    Vec3::ZERO
                };
                pinned = true;
            }
        } else {
            if point.x > 0.0 {
                sign = -1.0;
            }
            // The last stop wins, as the original's loop overwrites.
            stop = Some((i, point, link.slack));
        }
    }
    match stop {
        Some((travel_link, point, stop_slack)) => {
            let radius = crate::lever::engine_length(point);
            Shape {
                sliding: false,
                travel_link,
                sign,
                radius,
                half_swing: half_swing(stop_slack, radius),
                stop_slack,
                axis,
            }
        }
        // A guess: the original reads a half swing it never set when a
        // hinge has no stop. No shipped hinge lacks one; pi keeps the leaf
        // free.
        None => Shape {
            sliding: false,
            travel_link: 0,
            sign,
            radius: 0.0,
            half_swing: std::f32::consts::PI,
            stop_slack: 0.0,
            axis,
        },
    }
}

/// The collision sound's two factors for a body of this mobility: the
/// squared inverse of `1 + min(2m, 1)`, and the mean of its root and itself.
pub fn sound_factors(mobility: f32) -> (f32, f32) {
    let x = (mobility * 2.0).min(1.0) + 1.0;
    (1.0 / (x * x), (x.sqrt() + x) * 0.5)
}

/// A leaf's motor mode, stored in the object's own word.
pub const MODE_IDLE: i32 = 0;
/// A slider whose close command arrived (the word then reads latched).
pub const MODE_SHUT: i32 = 1;
/// Waiting out a signalled open.
pub const MODE_WAIT: i32 = 3;
/// Driving open.
pub const MODE_OPEN: i32 = 4;
/// Driving to the stored fraction.
pub const MODE_TOGGLE: i32 = 8;
/// A shut leaf reads closed within this many radians of its closed heading.
pub const CLOSED_SLOP: f32 = 0.04;
/// A leaf is near closed within this many radians.
pub const NEAR_SLOP: f32 = 0.1;
/// An open leaf is passable once it is this far past closed, in radians.
pub const PASSABLE_GAP: f32 = 0.4;

/// A sound the original plays with a random share.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sound {
    pub volume: f32,
    /// Added to the volume, times a roll in [0, 1).
    pub volume_spread: f32,
    pub pitch: f32,
    pub pitch_spread: f32,
}

impl Sound {
    /// The volume and pitch for one roll in [0, 1).
    pub fn rolled(&self, roll: f32) -> (f32, f32) {
        (
            self.volume + self.volume_spread * roll,
            self.pitch + self.pitch_spread * roll,
        )
    }

    /// The volume and pitch with the original's draws: the pitch's share
    /// is drawn first and the volume's second, each only when it has one
    /// (a knock or a slam draws once, the unlatch and the lock sounds
    /// twice). Each level is `draw·spread + base`, formed in double
    /// precision and narrowed, as the original forms it. Returns `(volume,
    /// pitch)`.
    ///
    /// The bases and spreads are the original's double literals (0.1,
    /// 0.05, …) kept in single precision; they are widened by their
    /// shortest decimal spelling so that 0.1 widens to the double 0.1, not
    /// to the single 0.1's exact value.
    pub fn levels(&self, mut draw: impl FnMut() -> f64) -> (f32, f32) {
        let pitch = if self.pitch_spread != 0.0 {
            (draw() * wide(self.pitch_spread) + wide(self.pitch)) as f32
        } else {
            self.pitch
        };
        let volume = if self.volume_spread != 0.0 {
            (draw() * wide(self.volume_spread) + wide(self.volume)) as f32
        } else {
            self.volume
        };
        (volume, pitch)
    }
}

/// A single-precision literal widened to the double it was written as:
/// the shortest decimal that reads back as `x`, read as a double. A value
/// with no such spelling widens exactly.
fn wide(x: f32) -> f64 {
    x.to_string().parse().unwrap_or(f64::from(x))
}

/// An object word as a door leaves it: the mode in the low byte, the
/// countdown in the top byte, and the latch, which shares bit 0 with the
/// mode.
pub fn stored_word(mode: i32, countdown: i32, latched: bool) -> i32 {
    (mode & 0xFF) | (countdown << 24) | i32::from(latched)
}

/// A door body's state and its per-tick rules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Door {
    pub record: Record,
    pub shape: Shape,
    /// The object's stored word, kept as the original keeps it.
    pub saved: i32,
    pub locked: bool,
    pub frozen: bool,
    pub motorised: bool,
    pub fast: bool,
    pub latches: bool,
    pub latched: bool,
    /// The hinge's motor mode and countdown.
    pub mode: i32,
    pub countdown: i32,
    /// 1 aims shut, 0 aims open.
    pub fraction: f32,
    /// The leaf is shut.
    pub closed: bool,
    /// The leaf reported near closed last time it was open.
    pub near: bool,
    /// This leaf's own passage state.
    pub passable: bool,
    /// The passage state of the pair.
    pub both_passable: bool,
    /// The shut pull counter.
    pub pull: i32,
    /// A hand holds the leaf.
    pub held: bool,
    pub has_partner: bool,
    pub partner_near: bool,
    pub unlock_pending: bool,
    pub unlock_at_ms: i32,
    pub slam_cooldown_ms: u32,
    pub knock_cooldown_ms: u32,
    /// The heading read from the stop this tick.
    pub angle: f32,
    /// The sliding leaf's rule state.
    pub slide: Slide,
}

/// One tick of a door body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    pub now_ms: i32,
    /// The stop's heading in the leaf's frame, before the swing sign.
    pub heading: f32,
    /// Squared handle speed.
    pub energy: f32,
    /// The stop link's distance (hinge) or the travel fraction (slider).
    pub travel: f32,
    /// Squared handle speed of a shutting leaf, for the slam.
    pub handle_speed_sq: f32,
    /// A random value in 0..100, for the knock and slam cooldowns.
    pub roll: u32,
}

impl Tick {
    pub fn new(now_ms: i32, heading: f32, energy: f32, travel: f32) -> Self {
        Self {
            now_ms,
            heading,
            energy,
            travel,
            handle_speed_sq: 0.0,
            roll: 0,
        }
    }
}

/// What one door tick reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoorTick {
    pub playing: bool,
    pub volume: f32,
    pub pitch: f32,
    /// The angle the leaf is driven toward, when a command is running.
    pub drive: Option<f32>,
    /// The leaf went from open to shut.
    pub shut: bool,
    pub shut_sound: Option<Sound>,
    /// The leaf left the shut state by a pull.
    pub unlatched: bool,
    pub unlatch_sound: Option<Sound>,
    pub knock_sound: Option<Sound>,
    pub passable: bool,
    pub both_passable: bool,
    /// The travel written back to the sliding link.
    pub travel: f32,
    /// The unlatch sound of an open this tick (a finished signal wait).
    pub opened: Option<Sound>,
    /// The 3 cm shove of that open, in the leaf's frame (a hinge only).
    pub shove: Vec3,
}

impl Door {
    /// A door from its record, shape and stored word. `heading` is the
    /// stop's heading at the file position; `travel` the slide fraction.
    pub fn new(record: Record, shape: Shape, saved: i32, heading: f32, travel: f32) -> Self {
        let mut door = Door {
            record,
            shape,
            saved,
            locked: record.has(LOCKED),
            frozen: record.has(FROZEN),
            motorised: record.has(MOTORISED),
            fast: record.has(FAST),
            latches: record.has(LATCHES),
            latched: saved & 1 != 0,
            mode: saved & 0xF,
            // The countdown is the word's top byte, read unsigned.
            countdown: ((saved as u32) >> 24) as i32,
            fraction: 0.0,
            closed: false,
            near: false,
            passable: false,
            both_passable: false,
            pull: 0,
            held: false,
            has_partner: false,
            partner_near: false,
            unlock_pending: false,
            unlock_at_ms: 0,
            slam_cooldown_ms: 0,
            knock_cooldown_ms: 0,
            angle: 0.0,
            slide: Slide {
                mode: SlideMode::Idle,
                countdown: 0,
                frozen: record.has(FROZEN),
                motorised: record.has(MOTORISED),
                locked: record.has(LOCKED),
                latches: record.has(LATCHES),
                latched: saved & 1 != 0,
                closed: false,
                holding: false,
                knocked: false,
                count: 0,
                fraction: travel,
                parked: travel,
                smoothing: 0.0,
                energy: 0.0,
                unlock_pending: false,
                unlock_at_ms: 0,
            },
        };
        if shape.sliding {
            door.closed = f64::from(travel) >= SLIDE_SHUT_WIDE;
            if door.frozen {
                door.slide.parked = travel;
                door.slide.fraction = travel;
                door.slide.smoothing = 1.0;
                door.closed = true;
            }
            door.passable = if door.closed {
                !door.locked
            } else {
                !door.locked && f64::from(travel) >= SLIDE_PASS_WIDE
            };
            door.both_passable = door.passable;
            door.slide.closed = door.closed;
            door.slide.mode = match door.mode {
                MODE_WAIT => SlideMode::Wait,
                MODE_OPEN => SlideMode::Open,
                MODE_TOGGLE => SlideMode::Close,
                MODE_SHUT => SlideMode::Shut,
                _ => SlideMode::Idle,
            };
            door.slide.countdown = door.countdown;
        } else {
            door.angle = heading * shape.sign;
            // Shut within 0.04 rad of closed, a double compare.
            door.closed = f64::from(shape.closed()) - CLOSED_SLOP_WIDE < f64::from(door.angle);
            door.near = door.closed;
            if door.closed {
                door.pull = PULL_CAP;
            }
            door.passable = !door.locked && (door.closed || door.open_passable());
            door.both_passable = door.passable;
        }
        door.slide.count = door.pull;
        door
    }

    /// The stored word this door would leave behind. It is the word as the
    /// original keeps it: every command writes its mode into the low byte
    /// (so an even mode clears the latch bit) and its countdown into the top
    /// byte, a tick's countdown is not written back, a shut latching leaf
    /// and a close-and-lock set bit 0, and an unlock clears it.
    pub fn stored(&self) -> i32 {
        self.saved
    }

    /// Start a command: the mode and countdown, also written to the word's
    /// low and top bytes (the bytes between are kept).
    fn command(&mut self, mode: i32, countdown: i32) {
        self.mode = mode;
        self.countdown = countdown;
        self.saved = (self.saved & 0x00FF_FF00) | (mode & 0xFF) | (countdown << 24);
    }

    /// Start a sliding leaf's command, in its own fields and in the word.
    fn slide_command(&mut self, mode: SlideMode, countdown: i32) {
        self.slide.mode = mode;
        self.slide.countdown = countdown;
        self.command(slide_mode_code(mode), countdown);
    }

    /// An open hinge is passable past `closed − 0.4`, a double compare.
    fn open_passable(&self) -> bool {
        f64::from(self.shape.closed()) - PASSABLE_WIDE < f64::from(self.angle)
    }

    /// Whether the leaf is open enough to walk through: a slider by its
    /// travel, a hinge by its angle.
    pub fn openness(&self) -> f32 {
        if self.shape.sliding {
            1.0 - self.slide.fraction
        } else {
            hinge_openness(self.angle, self.shape.closed())
        }
    }

    /// Push the leaf open by 3 cm along its facing, or jump a slider to 0.6.
    /// The latch, the shut flag and the pull count clear; a motorised leaf
    /// gets its command.
    pub fn open(&mut self) -> Opened {
        self.latched = false;
        self.closed = false;
        self.pull = 0;
        if self.shape.sliding {
            self.slide.latched = false;
            self.slide.closed = false;
            self.slide.count = 0;
            if self.motorised {
                self.slide_command(SlideMode::Open, SLIDE_MOTOR_TICKS);
            } else {
                self.slide.fraction = SLIDE_JUMP;
                self.slide_command(SlideMode::Open, SLIDE_OPEN_TICKS);
            }
            Opened {
                sound: Some(self.unlatch_sound()),
                shove: Vec3::ZERO,
                woke: true,
            }
        } else {
            if self.motorised {
                self.fraction = 0.0;
                self.command(MODE_OPEN, HINGE_OPEN_TICKS);
            } else {
                self.command(MODE_IDLE, 0);
            }
            Opened {
                sound: Some(self.unlatch_sound()),
                shove: open_push(Vec3::Z),
                woke: false,
            }
        }
    }

    fn unlatch_sound(&self) -> Sound {
        Sound {
            volume: 0.1,
            volume_spread: 0.05,
            pitch: 0.8,
            pitch_spread: 0.3,
        }
    }

    fn unlock_sound(&self) -> Sound {
        Sound {
            volume: 0.15,
            volume_spread: 0.1,
            pitch: 0.8,
            pitch_spread: 0.2,
        }
    }

    fn lock_sound(&self) -> Sound {
        Sound {
            volume: 0.15,
            volume_spread: 0.1,
            pitch: 0.9,
            pitch_spread: 0.15,
        }
    }

    /// Toggle the leaf by how open it is. A latched leaf opens first.
    pub fn toggle(&mut self) -> Toggled {
        let mut shove = Vec3::ZERO;
        if self.latched {
            shove = self.open().shove;
        }
        let span = self.openness();
        if self.shape.sliding {
            if span <= TOGGLE_OPEN {
                self.slide_command(SlideMode::Open, TOGGLE_FAST);
                self.slide.closed = false;
                self.closed = false;
            } else {
                self.slide_command(SlideMode::Close, TOGGLE_FAST);
            }
        } else {
            self.fraction = if span > TOGGLE_OPEN { 1.0 } else { 0.0 };
            let ticks = if self.fast { TOGGLE_FAST } else { TOGGLE_SLOW };
            self.command(MODE_TOGGLE, ticks);
        }
        Toggled {
            shove,
            wake_ticks: TOGGLE_WAKE,
        }
    }

    /// Close and lock the leaf. Returns the lock sound when one plays.
    pub fn close_and_lock(&mut self) -> Option<Sound> {
        let sound = if !self.shape.sliding {
            if self.motorised {
                self.fraction = 1.0;
                let ticks = if self.fast { SHUT_FAST } else { SHUT_SLOW };
                self.command(MODE_TOGGLE, ticks);
            }
            None
        } else {
            let sound = if self.latched {
                None
            } else {
                Some(self.lock_sound())
            };
            if self.fast {
                self.slide_command(SlideMode::Close, SHUT_FAST);
            }
            sound
        };
        if self.closed {
            self.latched = true;
            self.slide.latched = true;
            self.saved |= 1;
        }
        sound
    }

    /// Unlock the leaf after the key fits: the unlock sound, the saved latch
    /// cleared and a 1000 ms wait on both leaves.
    pub fn unlock(&mut self, partner: Option<&mut Door>, now_ms: i32) -> Sound {
        self.saved &= !1;
        self.unlock_pending = true;
        self.unlock_at_ms = now_ms.saturating_add(UNLOCK_WAIT_MS);
        if let Some(partner) = partner {
            partner.unlock_pending = true;
            partner.unlock_at_ms = now_ms.saturating_add(UNLOCK_WAIT_MS);
        }
        self.unlock_sound()
    }

    /// A signal asks the leaf to open: a latched or motorised leaf wakes and
    /// waits 120 ticks; a plain leaf nothing.
    pub fn wake(&mut self) {
        if self.latched || self.motorised {
            if self.shape.sliding {
                self.slide_command(SlideMode::Wait, SIGNAL_WAIT);
            } else {
                self.command(MODE_WAIT, SIGNAL_WAIT);
            }
        }
    }

    /// One tick, in the original's order.
    pub fn step(&mut self, tick: Tick, partner: Option<&mut Door>) -> DoorTick {
        if self.shape.sliding {
            return self.step_slide(tick);
        }
        // A signalled open waits out its countdown, then opens.
        let mut opened = None;
        let mut shove = Vec3::ZERO;
        if self.mode == MODE_WAIT {
            if self.countdown < 2 {
                let open = self.open();
                opened = open.sound;
                shove = open.shove;
            } else {
                self.countdown -= 1;
            }
        }
        let mut done = self.step_hinge(tick, partner);
        done.opened = opened;
        done.shove = shove;
        done
    }

    fn step_hinge(&mut self, tick: Tick, mut partner: Option<&mut Door>) -> DoorTick {
        let partner_near = partner.as_ref().map(|p| p.near);
        let partner_passable = partner.as_ref().map(|p| p.passable).unwrap_or(false);
        self.has_partner = partner.is_some();
        self.partner_near = partner_near.unwrap_or(false);

        // The handle's speed. A creep at or below the threshold leaves the
        // squared energy in place; above it the field holds the speed.
        let energy = tick.energy;
        let playing = creaks(energy);
        let speed = if playing { energy.sqrt() } else { energy };
        let (volume, pitch) = if playing {
            hinge_creak(speed)
        } else {
            (SLIDE_STOP_BASIS, 0.0)
        };

        // The stop knocks when its link is stretched past its slack. Its
        // volume has no random share; the roll only lifts the pitch.
        let mut knock_sound = None;
        if tick.travel > self.shape.stop_slack
            && f64::from(speed) > KNOCK_SPEED_WIDE
            && self.knock_cooldown_ms < tick.now_ms as u32
        {
            knock_sound = Some(Sound {
                volume: volume * volume * 20.0,
                volume_spread: 0.0,
                pitch: (f64::from(speed) * 0.1 + 0.3) as f32,
                pitch_spread: 0.2,
            });
            self.knock_cooldown_ms = (tick.now_ms.max(0) as u32)
                .saturating_add(200)
                .saturating_add(tick.roll % 100);
        }

        // A finished unlock clears the latch on both leaves (the partner's
        // own wait is left running).
        if self.unlock_pending && self.unlock_at_ms < tick.now_ms {
            self.saved &= !1;
            self.latched = false;
            self.unlock_pending = false;
            if let Some(partner) = partner.as_deref_mut() {
                partner.latched = false;
            }
        }

        // The stop's heading is in the hinge frame. The swing sign maps it
        // onto the angle the shut test and the passage use. Dropping the
        // sign makes a negative-swing leaf read under its shut angle and
        // leave the shut state at rest.
        self.angle = tick.heading * self.shape.sign;

        // The passage follows the heading just stored, shut or open. The
        // room route reads it: a hinge more than 0.4 rad past closed is
        // routable, a shut one is not, and a locked door always is.
        let own = !self.locked && (self.shape.sliding || self.open_passable());
        self.set_passage(own, partner_passable);

        let motor_drive = self.drive();

        let mut shut = false;
        let mut shut_sound = None;
        let mut unlatch_sound = None;
        if !self.closed {
            if self.angle <= self.shape.closed() {
                self.near = near_below(self.angle, self.shape.closed());
            } else {
                self.closed = true;
                self.near = true;
                self.pull = PULL_CAP;
                if self.latches {
                    self.latched = true;
                    self.saved |= 1;
                }
                if self.slam_cooldown_ms < tick.now_ms as u32 {
                    let amp = (f64::from(tick.handle_speed_sq) * 0.1) as f32;
                    let amp = amp.min(1.0);
                    shut_sound = Some(Sound {
                        volume: amp * 0.5,
                        volume_spread: 0.0,
                        pitch: (f64::from(amp) * 0.3 + 0.4) as f32,
                        pitch_spread: 0.2,
                    });
                    self.slam_cooldown_ms = (tick.now_ms.max(0) as u32)
                        .saturating_add(200)
                        .saturating_add(tick.roll % 100);
                }
                shut = true;
            }
        } else {
            let drive = if self.has_partner && !self.partner_near {
                None
            } else if self.mode & MODE_TOGGLE == 0 {
                Some(self.shape.closed())
            } else {
                motor_drive
            };
            if !self.has_partner {
                if leaves_single(self.angle, self.shape.closed()) && !self.latched {
                    self.closed = false;
                }
            } else if !self.partner_near && self.angle < self.shape.closed() {
                self.closed = false;
            }
            if self.pull < PULL_CAP {
                self.pull += 1;
            }
            if self.held && !self.latched && self.angle < self.shape.closed() {
                self.pull -= PULL_STEP;
                if self.pull < 0 {
                    unlatch_sound = Some(self.unlatch_sound());
                    self.closed = false;
                    self.pull = 0;
                }
            }
            if let Some(partner) = partner.as_deref_mut() {
                partner.both_passable = self.both_passable;
            }
            return DoorTick {
                playing,
                volume,
                pitch,
                drive,
                shut,
                shut_sound,
                unlatched: unlatch_sound.is_some(),
                unlatch_sound,
                knock_sound,
                passable: self.passable,
                both_passable: self.both_passable,
                travel: tick.travel,
                opened: None,
                shove: Vec3::ZERO,
            };
        }
        if let Some(partner) = partner.as_mut() {
            partner.both_passable = self.both_passable;
        }

        DoorTick {
            playing,
            volume,
            pitch,
            drive: motor_drive,
            shut,
            shut_sound,
            unlatched: false,
            unlatch_sound,
            knock_sound,
            passable: self.passable,
            both_passable: self.both_passable,
            travel: tick.travel,
            opened: None,
            shove: Vec3::ZERO,
        }
    }

    /// Record this leaf's own passage and the pair's: the pair is passable
    /// only when both leaves are. The tick also writes the pair's value to
    /// the partner, as the original does.
    pub fn set_passage(&mut self, own: bool, partner_passable: bool) {
        self.passable = own;
        self.both_passable = own && (!self.has_partner || partner_passable);
    }

    /// The motor's step for modes 4 and 8, shared with the rule helper.
    /// Ending a command (its countdown, or an arrival that does not latch)
    /// writes mode 0 and countdown 0 to the word; a tick's countdown does
    /// not, and neither does a toggle's latch on arrival.
    fn drive(&mut self) -> Option<f32> {
        let mode = match self.mode {
            MODE_OPEN => MotorMode::Open,
            MODE_TOGGLE => MotorMode::Toggle,
            _ => return None,
        };
        let mut motor = Motor {
            mode,
            countdown: self.countdown,
            motorised: self.motorised,
            fast: self.fast,
            latches: self.latches,
            latched: self.latched,
            fraction: self.fraction,
            closed: self.shape.closed(),
            range: self.shape.range(),
            sign: self.shape.sign,
            angle: self.angle,
        };
        let drive = motor_step(&mut motor);
        self.latched = motor.latched;
        if motor.mode == MotorMode::Idle {
            self.command(MODE_IDLE, 0);
        } else {
            self.countdown = motor.countdown;
        }
        drive
    }

    /// The sliding leaf's tick, in the original's order.
    fn step_slide(&mut self, tick: Tick) -> DoorTick {
        self.slide.frozen = self.frozen;
        self.slide.locked = self.locked;
        self.slide.motorised = self.motorised;
        self.slide.latches = self.latches;
        self.slide.latched = self.latched;
        self.slide.closed = self.closed;
        self.slide.count = self.pull;
        self.slide.holding = self.held;
        self.slide.energy = tick.energy;
        self.slide.unlock_pending = self.unlock_pending;
        self.slide.unlock_at_ms = self.unlock_at_ms;
        let was_pending = self.unlock_pending;
        let mut written = None;
        let tick = slide_tick_written(&mut self.slide, tick.now_ms, &mut written);
        // The word in the order the tick touches it: a finished unlock
        // clears bit 0, the commands write their mode and countdown, a shut
        // latching leaf sets bit 0.
        if was_pending && !self.slide.unlock_pending {
            self.saved &= !1;
        }
        if let Some((mode, countdown)) = written {
            self.saved = (self.saved & 0x00FF_FF00) | (mode & 0xFF) | (countdown << 24);
        }
        if tick.slammed && self.latches {
            self.saved |= 1;
        }
        self.mode = slide_mode_code(self.slide.mode);
        self.countdown = self.slide.countdown;
        self.closed = self.slide.closed;
        self.latched = self.slide.latched;
        self.pull = self.slide.count;
        self.unlock_pending = self.slide.unlock_pending;
        self.unlock_at_ms = self.slide.unlock_at_ms;
        self.passable = tick.passable;
        self.both_passable = tick.passable;
        DoorTick {
            playing: tick.playing,
            volume: tick.volume,
            pitch: tick.pitch,
            drive: None,
            shut: tick.slammed,
            shut_sound: if tick.slammed {
                Some(Sound {
                    volume: tick.slam_volume,
                    volume_spread: 0.0,
                    pitch: tick.slam_pitch,
                    pitch_spread: 0.2,
                })
            } else {
                None
            },
            unlatched: tick.unlatched,
            // A pull that unlatches a slider plays `opn` with a wider pitch
            // roll (0.4) than the open command's (0.3).
            unlatch_sound: if tick.unlatched {
                Some(Sound {
                    volume: 0.1,
                    volume_spread: 0.05,
                    pitch: 0.8,
                    pitch_spread: 0.4,
                })
            } else {
                None
            },
            knock_sound: if tick.knocked {
                Some(Sound {
                    volume: tick.knock_volume,
                    volume_spread: 0.0,
                    pitch: tick.knock_pitch,
                    pitch_spread: 0.2,
                })
            } else {
                None
            },
            travel: self.slide.fraction,
            passable: tick.passable,
            both_passable: tick.passable,
            // A finished wait opens the leaf through the open command, with
            // its sound.
            opened: if tick.opened {
                Some(self.unlatch_sound())
            } else {
                None
            },
            shove: Vec3::ZERO,
        }
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
            unlock_pending: false,
            unlock_at_ms: 0,
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
        low.fraction = 0.02;
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
        let on_the_line = pair_shut(&pair(0.950_000_05));
        assert!(on_the_line.shut);
        assert!(on_the_line.hold);
        // The float nearest 0.95 (0.94999999) is past the double 0.05 line.
        let past_the_line = pair_shut(&pair(0.95));
        assert!(!past_the_line.shut);
        assert!(past_the_line.hold);

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

    fn rest() -> [Vec3; 4] {
        [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]
    }

    fn pin(x: f32, y: f32, z: f32) -> Link {
        // The rest tetra is the origin and the three axes, so the weights
        // [1, x, y, z] place the point at (x, y, z).
        Link {
            on_body: [1.0, x, y, z],
            on_parent: [1.0, 0.0, 0.0, 0.0],
            slack: 0.0,
            flags: 0,
        }
    }

    /// The shipped door's swing stop: a 164.8 cm radius, 134.04 cm of slack
    /// and its point 0.7 cm to the negative side.
    #[test]
    fn a_doors_swing_is_twice_the_arcsine_of_half_its_slack() {
        let stop = Link {
            on_body: [1.0, -164.8, 0.0, -0.7],
            on_parent: [1.0, 0.0, 0.0, 0.0],
            slack: 134.04,
            flags: 0,
        };
        let swing = shape(&rest(), &[pin(0.7, 242.9, -0.7), stop]);
        assert!(!swing.sliding);
        assert_eq!(swing.travel_link, 1);
        assert!((swing.radius - 164.8015).abs() < 1e-3, "{}", swing.radius);
        // The engine's fast arcsine: 0.83806, against the exact 0.83761.
        assert!(
            (swing.half_swing - 0.83806).abs() < 1e-4,
            "{}",
            swing.half_swing
        );
        assert!((swing.closed() - 0.83806).abs() < 1e-4);
        assert!((swing.range() + 1.67612).abs() < 1e-3);
        assert_eq!(swing.sign, 1.0);
        assert!((swing.axis.y - 1.0).abs() < 1e-3, "{:?}", swing.axis);

        let positive = Link {
            on_body: [1.0, 164.8, 0.0, 0.0],
            ..stop
        };
        let flipped = shape(&rest(), &[positive]);
        assert_eq!(flipped.sign, -1.0);
        assert!((flipped.range() - 1.6761).abs() < 1e-3);

        // A link with no stop keeps the half swing of pi.
        let open = shape(&rest(), &[pin(1.0, 0.0, 0.0)]);
        assert_eq!(open.half_swing, std::f32::consts::PI);
    }

    /// A slider's travel link is the first whose body point is 30 to 160 cm
    /// up.
    #[test]
    fn a_sliders_travel_link_is_the_one_at_door_height() {
        let low = Link {
            on_body: [1.0, 0.0, 20.0, 0.0],
            on_parent: [1.0, 0.0, 0.0, 0.0],
            slack: 4900.0,
            flags: Link::SLIDER,
        };
        let high = Link {
            on_body: [1.0, 0.0, 40.0, 0.0],
            ..low
        };
        let above = Link {
            on_body: [1.0, 0.0, 170.0, 0.0],
            ..low
        };
        let travel = shape(&rest(), &[low, high, above]);
        assert!(travel.sliding);
        assert_eq!(travel.travel_link, 1);
    }

    fn hinge_shape() -> Shape {
        Shape {
            sliding: false,
            travel_link: 0,
            sign: 1.0,
            radius: 100.0,
            half_swing: std::f32::consts::FRAC_PI_6,
            stop_slack: 50.0,
            axis: Vec3::X,
        }
    }

    fn door(record: Record, angle: f32) -> Door {
        Door::new(record, hinge_shape(), 0, angle, 0.0)
    }

    #[test]
    fn a_door_at_its_file_position_is_shut() {
        let shut = door(Record::default(), 0.52);
        assert!(shut.closed);
        assert_eq!(shut.pull, PULL_CAP);
        assert!(shut.passable);

        let ajar = door(Record::default(), 0.40);
        assert!(!ajar.closed);
        assert!(!ajar.near);
        assert_eq!(ajar.pull, 0);

        let barred = door(
            Record {
                flags: LOCKED,
                ..Record::default()
            },
            0.52,
        );
        assert!(!barred.passable);

        // The latch shares the mode word's low bit, as the original's does:
        // a latched door reads mode 5, not 4.
        let saved = stored_word(MODE_OPEN, 10, true);
        let resumed = Door::new(Record::default(), hinge_shape(), saved, 0.52, 0.0);
        assert_eq!(resumed.mode, 5);
        assert_eq!(resumed.countdown, 10);
        assert!(resumed.latched);
        assert_eq!(resumed.stored(), saved);
    }

    #[test]
    fn a_motor_door_steps_toward_its_aim_and_latches_on_arrival() {
        let record = Record {
            flags: MOTORISED,
            ..Record::default()
        };
        let saved = stored_word(MODE_OPEN, 10, false);
        let plain_open = Door::new(Record::default(), hinge_shape(), saved, 0.0, 0.0);
        let mut plain = plain_open;
        let tick = plain.step(Tick::new(0, 0.0, 0.0, 0.0), None);
        assert!((tick.drive.unwrap() + 0.0008).abs() < 1e-6);
        let mut opening = Door::new(record, hinge_shape(), saved, 0.0, 0.0);
        let tick = opening.step(Tick::new(0, 0.0, 0.0, 0.0), None);
        assert_eq!(opening.countdown, 9);
        // (1 - 0) * -1.0472 + 0.5236 = -0.5236 at the open end.
        let drive = tick.drive.expect("an open command drives");
        assert!((drive + 0.0003).abs() < 1e-6, "{drive}");

        let mut arriving = Door::new(record, hinge_shape(), saved, -0.5, 0.0);
        let tick = arriving.step(Tick::new(0, -0.5, 0.0, 0.0), None);
        assert!((tick.drive.unwrap() + 0.5236).abs() < 1e-4);
        assert_eq!(arriving.mode, MODE_IDLE);
        assert_eq!(arriving.countdown, 0);

        let fast = Record {
            flags: MOTORISED | FAST,
            ..Record::default()
        };
        let mut quick = Door::new(fast, hinge_shape(), saved, 0.0, 0.0);
        let tick = quick.step(Tick::new(0, 0.0, 0.0, 0.0), None);
        assert!((tick.drive.unwrap() + 0.005).abs() < 1e-6);

        let latches = Record {
            flags: LATCHES,
            ..Record::default()
        };
        let mut barring = Door::new(
            latches,
            hinge_shape(),
            stored_word(MODE_TOGGLE, 10, false),
            0.0,
            0.0,
        );
        barring.fraction = 0.0;
        // The aim is the open end; an angle within a step of it latches.
        let tick = barring.step(Tick::new(0, -0.5236, 0.0, 0.0), None);
        assert!(barring.latched);
        assert_eq!(barring.mode, MODE_TOGGLE);
        assert!((tick.drive.unwrap() + 0.5236).abs() < 1e-4);
    }

    #[test]
    fn a_shutting_door_slams_and_a_pull_unlatches_it() {
        let record = Record {
            flags: LATCHES,
            ..Record::default()
        };
        let mut shutting = door(record, 0.0);
        let mut tick = Tick::new(1, 0.6, 0.0, 0.0);
        tick.handle_speed_sq = 0.4;
        let done = shutting.step(tick, None);
        assert!(done.shut);
        assert!(shutting.closed);
        assert!(shutting.latched);
        assert_eq!(shutting.pull, PULL_CAP);
        assert_eq!(shutting.stored() & 1, 1);
        // min(0.04, 1) * 0.5 volume, 0.4 + 0.3 * 0.04 pitch.
        let slam = done.shut_sound.expect("the shut leaf slams");
        assert!((slam.volume - 0.02).abs() < 1e-6);
        assert!((slam.pitch - 0.412).abs() < 1e-6);
        // The cooldown holds the next slam back.
        let again = shutting.step(Tick::new(50, 0.6, 0.0, 0.0), None);
        assert!(again.shut_sound.is_none());

        let mut held = door(Record::default(), 0.52);
        held.held = true;
        let mut counts = [0; 6];
        for slot in &mut counts {
            let done = held.step(Tick::new(0, 0.52, 0.0, 0.0), None);
            assert!(!done.unlatched);
            *slot = held.pull;
        }
        assert_eq!(counts, [16, 13, 10, 7, 4, 1]);
        let done = held.step(Tick::new(0, 0.52, 0.0, 0.0), None);
        assert!(done.unlatched);
        assert_eq!(held.pull, 0);
        assert!(!held.closed);
        let sound = done.unlatch_sound.expect("the pull unlatches with a sound");
        assert!((sound.volume - 0.1).abs() < 1e-6);
        assert!((sound.pitch - 0.8).abs() < 1e-6);
    }

    #[test]
    fn an_open_toggle_unlock_and_wake_set_their_commands() {
        let mut plain = door(Record::default(), 0.52);
        let opened = plain.open();
        assert!(!plain.latched && !plain.closed);
        assert_eq!(plain.mode, MODE_IDLE);
        assert!(
            opened
                .shove
                .abs_diff_eq(Vec3::new(0.0, 0.0, OPEN_PUSH_CM), 1e-4)
        );
        assert!(opened.sound.is_some());

        let mut motor = door(
            Record {
                flags: MOTORISED,
                ..Record::default()
            },
            0.52,
        );
        motor.open();
        assert_eq!(motor.mode, MODE_OPEN);
        assert_eq!(motor.countdown, HINGE_OPEN_TICKS);
        assert!((motor.fraction).abs() < 1e-6);

        let mut slow = door(Record::default(), 0.0);
        let done = slow.toggle();
        assert!((slow.fraction).abs() < 1e-6);
        assert_eq!(slow.mode, MODE_TOGGLE);
        assert_eq!(slow.countdown, TOGGLE_SLOW);
        assert_eq!(done.wake_ticks, TOGGLE_WAKE);
        // (0 - 0.5236) * -0.7 = 0.3665, under seven tenths: it opens.
        let mut swung = door(Record::default(), -0.5764);
        swung.toggle();
        assert!((swung.fraction - 1.0).abs() < 1e-6);

        let mut latched = door(Record::default(), 0.52);
        latched.latched = true;
        let done = latched.toggle();
        assert!(!latched.latched);
        assert!(
            done.shove
                .abs_diff_eq(Vec3::new(0.0, 0.0, OPEN_PUSH_CM), 1e-4)
        );

        let mut unlockable = door(Record::default(), 0.52);
        let mut partner = door(Record::default(), 0.52);
        let sound = unlockable.unlock(Some(&mut partner), 500);
        assert!((sound.volume - 0.15).abs() < 1e-6);
        assert!(unlockable.unlock_pending && partner.unlock_pending);
        assert_eq!(unlockable.unlock_at_ms, UNLOCK_WAIT_MS + 500);
        assert_eq!(partner.unlock_at_ms, UNLOCK_WAIT_MS + 500);
        assert_eq!(unlockable.saved & 1, 0);

        let mut sleepy = door(Record::default(), 0.52);
        sleepy.wake();
        assert_eq!(sleepy.mode, MODE_IDLE);
        sleepy.motorised = true;
        sleepy.wake();
        assert_eq!(sleepy.mode, MODE_WAIT);
        assert_eq!(sleepy.countdown, SIGNAL_WAIT);
    }

    fn slide_shape() -> Shape {
        Shape {
            sliding: true,
            travel_link: 0,
            sign: 1.0,
            radius: 0.0,
            half_swing: 0.0,
            stop_slack: 0.0,
            axis: Vec3::X,
        }
    }

    #[test]
    fn a_sliding_leaf_waits_out_an_unlock_and_keeps_its_parked_fraction() {
        let mut shut = Door::new(Record::default(), slide_shape(), 1, 0.0, 1.0);
        assert!(shut.closed);
        assert!(shut.passable);
        shut.unlock(None, 0);
        assert!(shut.latched);
        let done = shut.step(Tick::new(UNLOCK_WAIT_MS, 0.0, 0.0, 1.0), None);
        assert!(done.passable);
        assert!(shut.latched);
        shut.step(Tick::new(UNLOCK_WAIT_MS + 1, 0.0, 0.0, 1.0), None);
        assert!(!shut.latched);

        let frozen = Record {
            flags: FROZEN,
            ..Record::default()
        };
        let mut parked = Door::new(frozen, slide_shape(), 0, 0.0, 0.4);
        assert!(parked.closed && parked.passable);
        let done = parked.step(Tick::new(0, 0.0, 0.0, 0.9), None);
        assert!((done.travel - 0.4).abs() < 1e-6);
        assert!((parked.slide.fraction - 0.4).abs() < 1e-6);
        assert!(done.passable);
    }

    #[test]
    fn a_heavy_door_speaks_softer() {
        let (squared, mean) = sound_factors(1.0);
        assert!((squared - 0.25).abs() < 1e-6);
        assert!((mean - 1.7071).abs() < 1e-4);
        let (squared, mean) = sound_factors(0.016);
        assert!((squared - 0.9390).abs() < 1e-4);
        assert!((mean - 1.0239).abs() < 1e-4);
        let (squared, mean) = sound_factors(0.0);
        assert!((squared - 1.0).abs() < 1e-6);
        assert!((mean - 1.0).abs() < 1e-6);
    }

    /// Only a command writes the word: a toggle stores mode 8 and its 100
    /// ticks, and the tick that counts the countdown down leaves it.
    #[test]
    fn a_command_writes_the_word_and_a_tick_does_not() {
        let shape = Shape {
            sliding: false,
            travel_link: 0,
            sign: 1.0,
            radius: 100.0,
            half_swing: std::f32::consts::FRAC_PI_6,
            stop_slack: 50.0,
            axis: Vec3::Y,
        };
        let mut door = Door::new(Record::default(), shape, 0, 0.52, 0.0);
        door.toggle();
        let word = MODE_TOGGLE | (TOGGLE_SLOW << 24);
        assert_eq!(door.stored(), word);
        door.step(Tick::new(0, 0.52, 0.0, 0.0), None);
        assert_eq!(door.countdown, TOGGLE_SLOW - 1);
        assert_eq!(door.stored(), word);
        assert_eq!(((door.stored() as u32) >> 24) as i32, TOGGLE_SLOW);
    }
}
