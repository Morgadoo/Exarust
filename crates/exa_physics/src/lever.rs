//! A lever: its record, the axle and swing its links describe, its creak,
//! the detent it steps toward, the signal when it arrives, and its timed
//! return.
//!
//! The original's lever body is `TRBRotator`. Links of at most 1 cm of
//! slack are axle pins (the first gives the axle) and the last longer link
//! is the arm: its radius r and half swing 2·asin(slack / 2r) put the two
//! detents at the ends of the swing, `detent0 = −half swing` and a step of
//! twice the half swing in the arm's own frame.
//!
//! The arm velocity is smoothed by half. The creak stops when the squared
//! speed is at most 0.015. Otherwise the volume is 0.2 times the square root
//! of that energy and the pitch is 0.2 times the energy plus 0.7, at most 1.
//!
//! Held, the target is the hand. Forced, it is the detent whose report is
//! off. Returning, it is the stored detent. Otherwise it is the nearest
//! (round to even). The drive moves 0.02 rad toward that target and holds
//! once the gap is within 0.02.
//!
//! Arriving more than 0.1 rad from the last detent reports the rest flag at
//! the first detent and its opposite at the second. A report of 0 signals 0
//! unless the lever is inverted, and stops a return. A report of 1 signals
//! 1, or −1 when the lever is forced and inverted, and arms a timed return:
//! the deadline is the countdown's time, the object word's bits 16 and up
//! hold the remaining time in hundredths of a second, and the `leverb_tck01`
//! loop fades out while the lever returns. A linked light fades out on 0
//! and in on 1 instead. A report of 0 with a light does not signal.
//!
//! A lever at its file position reads a detent end, so `settle` puts the
//! last report and the home there; a linked light starts full on the first
//! detent and off on the second.
//!
//! The solver feeds the step the arm's angle and its velocity: a hand or a
//! body pushing the arm moves it, and the step only nudges it by 0.02 rad.
//!
//! Evidence: the lever init, its step, the timed return and its own tick,
//! and the light finder.

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
    /// The clock the armed return fires at, in milliseconds.
    pub deadline_ms: i32,
    /// The counted-down time the object word holds, in milliseconds.
    pub remaining_ms: i32,
    /// The tick sound is looping and its volume.
    pub tick_playing: bool,
    pub tick_volume: f32,
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
        let steps = ((lever.angle - lever.detent0) / lever.span).round_ties_even();
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

/// A lever's operative record: what it signals and how it returns.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Record {
    /// The operative kind; 6 fades a linked light instead of signalling.
    pub kind: u8,
    /// The first detent reports 1.
    pub flags: u16,
    /// The object signalled first.
    pub target: u32,
    /// The object signalled second.
    pub second: u32,
    /// The auto-return time in units of 100 ms; 0 stays.
    pub time: u16,
}

/// The first detent reports 1.
pub const ON_AT_REST: u16 = 1;
/// The lever is held at the detent whose report is 0.
pub const FORCED: u16 = 2;
/// The signals are inverted.
pub const INVERTED: u16 = 8;
/// The second target is signalled too.
pub const SECOND: u16 = 0x10;
/// The kind that fades a linked light.
pub const LIGHT_KIND: u8 = 6;
/// Links at or under this slack are axle pins.
pub const AXLE_SLACK: f32 = 1.0;
/// The creak loop's sound name.
pub const CREAK_SOUND: &str = "leverb_mov";
/// The return's tick loop sound name.
pub const TICK_SOUND: &str = "leverb_tck01";
/// The tick loop starts at this volume and loses 0.01 a tick.
pub const TICK_VOLUME: f32 = 0.1;
pub const TICK_STEP: f32 = 0.01;

/// The axle and swing a lever's links describe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub radius: f32,
    pub half_swing: f32,
    /// The first detent's angle.
    pub detent0: f32,
    /// Radians from the first detent to the second.
    pub step: f32,
    pub axis: Vec3,
}

/// The half swing a lever link of this slack allows at this radius.
pub fn half_swing(slack: f32, radius: f32) -> f32 {
    2.0 * (slack * 0.5 / radius).asin()
}

/// A lever body's links as its axle and swing: links of at most 1 cm of
/// slack are pins (the first gives the axle), and the last longer link is
/// the arm. The two detents sit at the ends of the swing.
pub fn shape(rest: &[Vec3; 4], links: &[crate::door::Link]) -> Shape {
    let mut axis = Vec3::X;
    let mut pinned = false;
    let mut arm = None;
    for link in links {
        let point = link.point(rest);
        if link.slack <= AXLE_SLACK {
            if point.length_squared() > 1.0 && (!pinned || point.y >= 0.0) {
                let length = point.length();
                if length > 0.0 {
                    let candidate = point / length;
                    axis = if axis.dot(candidate) < 0.0 {
                        -candidate
                    } else {
                        candidate
                    };
                }
                pinned = true;
            }
        } else {
            arm = Some((point, link.slack));
        }
    }
    match arm {
        Some((point, slack)) => {
            let radius = point.length();
            let half = half_swing(slack, radius);
            Shape {
                radius,
                half_swing: half,
                detent0: -half,
                step: half * 2.0,
                axis,
            }
        }
        None => Shape {
            radius: 0.0,
            half_swing: std::f32::consts::PI,
            detent0: -std::f32::consts::PI,
            step: std::f32::consts::PI * 2.0,
            axis,
        },
    }
}

impl Lever {
    /// A lever at the file position's angle, settled on its nearest detent.
    pub fn new(shape: Shape, angle: f32) -> Self {
        let mut lever = Lever {
            smoothed: Vec3::ZERO,
            velocity: Vec3::ZERO,
            angle,
            detent0: shape.detent0,
            span: shape.step,
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
            deadline_ms: 0,
            remaining_ms: 0,
            tick_playing: false,
            tick_volume: 0.0,
            has_target: false,
            has_second: false,
        };
        lever.settle();
        lever
    }

    /// The detent index this angle is nearest: ties go to the lower, as the
    /// original's round-to-even does.
    pub fn detent_index(&self) -> i32 {
        if !self.span.is_finite() || self.span == 0.0 {
            return 0;
        }
        ((self.angle - self.detent0) / self.span).round_ties_even() as i32
    }

    /// The angle of a detent.
    pub fn detent_angle(&self, index: i32) -> f32 {
        self.detent0 + index as f32 * self.span
    }

    /// Settle on the nearest detent, as the file position does. Returns the
    /// detent index.
    pub fn settle(&mut self) -> i32 {
        let index = self.detent_index();
        let angle = self.detent_angle(index);
        self.last = angle;
        self.hand = angle;
        self.home = index;
        self.returning = false;
        index
    }

    /// Read the operative record: what the detents report and when the
    /// lever returns. A kind 6 lever fades a light found under its node.
    pub fn configure(&mut self, record: &Record, light_found: bool) {
        self.on_at_rest = record.flags & ON_AT_REST != 0;
        self.forced = record.flags & FORCED != 0;
        self.invert = record.flags & INVERTED != 0;
        self.second = record.flags & SECOND != 0;
        self.has_target = record.target != 0;
        self.has_second = record.second != 0;
        self.has_light = light_found && record.kind == LIGHT_KIND;
        self.return_ms = i32::from(record.time) * 100;
    }

    /// The linked light's intensity at load: full on the first detent, off
    /// on the second.
    pub fn initial_light(&self, index: i32) -> Option<f32> {
        if index == 0 { Some(1.0) } else { Some(0.0) }
    }

    /// Arm the timed return: it fires at `now_ms + ms` and the tick loop
    /// starts. The lever is not returning yet.
    pub fn arm(&mut self, now_ms: i32, ms: i32) {
        self.deadline_ms = now_ms.saturating_add(ms);
        self.remaining_ms = ms;
        self.returning = false;
        self.tick_playing = true;
        self.tick_volume = TICK_VOLUME;
    }

    /// Restore an arm across a load: the object word's bits 16 and up hold
    /// the remaining time in units of 100 ms.
    pub fn restore(&mut self, word: i32, now_ms: i32) {
        let stored = (word >> 16) * 100;
        if self.return_ms != 0 && stored != 0 {
            self.arm(now_ms, stored);
        }
    }

    /// The object word's return time: the remaining milliseconds in units
    /// of 100 ms, above the low word.
    pub fn return_word(&self) -> i32 {
        (self.remaining_ms / 100) << 16
    }

    /// The timed return's own tick: while the clock is short of the
    /// deadline it counts down, at the deadline it aims home (the detent
    /// whose report is 0) and returns true.
    pub fn timed_return(&mut self, now_ms: i32) -> bool {
        if now_ms < self.deadline_ms {
            self.remaining_ms = self.deadline_ms - now_ms;
            false
        } else {
            self.remaining_ms = 0;
            self.home = if self.on_at_rest { 1 } else { 0 };
            self.returning = true;
            true
        }
    }

    /// One tick with the arm's angle and its smoothed velocity, in the
    /// original's order: the return's tick fade happens before the report.
    pub fn step(&mut self, angle: f32, velocity: Vec3, now_ms: i32) -> LeverTick {
        let fading = self.returning && self.tick_playing;
        if fading {
            self.tick_volume -= TICK_STEP;
            if self.tick_volume < TICK_STEP {
                self.tick_volume = 0.0;
                self.tick_playing = false;
            }
        }
        self.angle = angle;
        self.velocity = velocity;
        let tick = lever_step(self);
        if tick.arm_return {
            self.arm(now_ms, self.return_ms);
        }
        tick
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
            deadline_ms: 0,
            remaining_ms: 0,
            tick_playing: false,
            tick_volume: 0.0,
            has_target: true,
            has_second: false,
        }
    }

    fn arm_link(x: f32, y: f32, z: f32, slack: f32) -> crate::door::Link {
        crate::door::Link {
            on_body: [1.0, x, y, z],
            on_parent: [1.0, 0.0, 0.0, 0.0],
            slack,
            flags: 0,
        }
    }

    fn rest() -> [Vec3; 4] {
        [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]
    }

    /// The shipped lever: axle pins 15 cm either side and a 33.81 cm arc at
    /// a 40 cm arm, so the detents sit at ±50 degrees.
    #[test]
    fn a_levers_detents_sit_at_the_ends_of_its_swing() {
        let links = [
            arm_link(15.0, 0.0, 0.0, 0.0),
            arm_link(-15.0, 0.0, 0.0, 0.0),
            arm_link(0.0, 40.0, 0.0, 33.81),
        ];
        let swing = shape(&rest(), &links);
        assert!((swing.radius - 40.0).abs() < 1e-5);
        assert!((swing.half_swing - 0.87266).abs() < 1e-4);
        assert!((swing.detent0 + 0.87266).abs() < 1e-4);
        assert!((swing.step - 1.74533).abs() < 1e-4);
        assert_eq!(swing.axis, Vec3::X);

        // A lever with no arm keeps the half swing of pi.
        let bare = shape(&rest(), &[arm_link(15.0, 0.0, 0.0, 0.0)]);
        assert_eq!(bare.half_swing, std::f32::consts::PI);
    }

    #[test]
    fn a_lever_settles_on_the_detent_it_is_nearest() {
        let swing = Shape {
            radius: 40.0,
            half_swing: 0.87266,
            detent0: -0.87266,
            step: 1.74533,
            axis: Vec3::X,
        };
        let on = Lever::new(swing, 0.87266);
        assert_eq!(on.detent_index(), 1);
        assert!((on.last - 0.87266).abs() < 1e-4);
        assert_eq!(on.initial_light(1), Some(0.0));
        let off = Lever::new(swing, -0.87266);
        assert_eq!(off.detent_index(), 0);
        assert_eq!(off.initial_light(0), Some(1.0));
        // Exactly half way rounds to the lower detent.
        let tied = Lever::new(swing, 0.0);
        assert_eq!(tied.detent_index(), 0);
        assert!((tied.last + 0.87266).abs() < 1e-5);

        let mut configured = off;
        let record = Record {
            flags: FORCED | INVERTED | SECOND,
            kind: LIGHT_KIND,
            target: 7,
            second: 8,
            time: 3,
        };
        configured.configure(&record, true);
        assert!(configured.forced && configured.invert && configured.second);
        assert!(configured.has_light && configured.has_target && configured.has_second);
        assert_eq!(configured.return_ms, 300);
        let no_light = Record { kind: 3, ..record };
        configured.configure(&no_light, true);
        assert!(!configured.has_light);
    }

    #[test]
    fn a_timed_return_counts_down_and_then_aims_home() {
        let swing = Shape {
            radius: 40.0,
            half_swing: 0.87266,
            detent0: -0.87266,
            step: 1.74533,
            axis: Vec3::X,
        };
        let mut lever = Lever::new(swing, 0.87266);
        lever.return_ms = 500;
        lever.arm(1000, 500);
        assert_eq!(lever.deadline_ms, 1500);
        assert_eq!(lever.remaining_ms, 500);
        assert_eq!(lever.return_word(), 5 << 16);
        assert!(lever.tick_playing);
        assert!((lever.tick_volume - TICK_VOLUME).abs() < 1e-6);

        assert!(!lever.timed_return(1200));
        assert_eq!(lever.remaining_ms, 300);
        assert!(!lever.timed_return(1499));
        assert_eq!(lever.remaining_ms, 1);
        assert!(lever.timed_return(1500));
        assert_eq!(lever.remaining_ms, 0);
        assert!(lever.returning);
        // Without the first detent reporting on, home is the first detent.
        assert_eq!(lever.home, 0);

        // The tick fades while returning and stops after ten ticks.
        let mut volume = lever.tick_volume;
        for _ in 0..9 {
            lever.step(0.0, Vec3::ZERO, 1500);
            volume -= TICK_STEP;
            assert!(
                (lever.tick_volume - volume).abs() < 1e-5,
                "{}",
                lever.tick_volume
            );
            assert!(lever.tick_playing);
        }
        lever.step(0.0, Vec3::ZERO, 1500);
        assert!(!lever.tick_playing);
        assert!((lever.tick_volume).abs() < 1e-6);

        let mut restored = Lever::new(swing, 0.87266);
        restored.return_ms = 500;
        restored.restore(5 << 16, 2000);
        assert_eq!(restored.remaining_ms, 500);
        assert_eq!(restored.deadline_ms, 2500);
    }

    #[test]
    fn a_returning_lever_steps_back_to_its_home_detent() {
        let swing = Shape {
            radius: 40.0,
            half_swing: 0.87266,
            detent0: -0.87266,
            step: 1.74533,
            axis: Vec3::X,
        };
        let mut lever = Lever::new(swing, 0.0);
        lever.home = 1;
        lever.returning = true;
        let tick = lever.step(0.0, Vec3::ZERO, 0);
        assert!((tick.target - 0.87266).abs() < 1e-4);
        assert!((tick.drive - 0.02).abs() < 1e-5);

        // Moving from the first detent onto the second asks for a return.
        let mut armed = Lever::new(swing, -0.87266);
        armed.return_ms = 250;
        let tick = armed.step(0.87266, Vec3::ZERO, 100);
        assert!(tick.arm_return);
        assert_eq!(armed.deadline_ms, 350);
        assert_eq!(armed.remaining_ms, 250);
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
