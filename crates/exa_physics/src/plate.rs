//! A pressure plate or push button: its travel, its grind, the click when
//! it goes down, and the record it reads.
//!
//! The original's push surface is `TRBPushSurface`. Its mobility is 0.3
//! when the file's is at least 0.5, else 0.03; it reads its operative
//! record's two targets and flags (kind 2 or bit 8 inverts the signals, bit
//! 2 makes it one-shot, bit 0x10 signals the second target) and starts
//! from the object word's low byte.
//!
//! Each travel rises by 0.02 while the plate is pressed, at most 1, and
//! falls by 0.01 otherwise, at least 0. A one-shot stops being pressed once
//! a travel passes 1, and later travels in that tick fall. The mean is
//! written back to every travel.
//!
//! The grind speed is 0.9 times the change in the mean plus 0.1 times the
//! old speed. It stops at 0.001 or below. Otherwise the volume is the square
//! of 50 times the speed and the pitch is 20 times the speed plus 0.8, at
//! most 1.
//!
//! A mean above 0.7 presses a plate that was up and signals 1, or −1 when
//! inverted, with the click at 0.3 (pitch 0.9 plus 0.2 random). A mean
//! below 0.5 releases a plate that was down and signals 0, unless inverted.
//! A mean of 0.7 does not press. A mean of 0.5 does not release.
//!
//! The solver moves the travels through the slider links: a body standing
//! on the plate presses it by moving the link, and the ±0.02/0.01 ramp is
//! the plate's own spring.
//!
//! Evidence: the push-surface init and step.

/// How far a pressed travel rises in one tick.
pub const RISE: f32 = 0.02;
/// How far a free travel falls in one tick.
pub const FALL: f32 = 0.01;
/// A mean above this presses the plate.
pub const PRESS: f32 = 0.7;
/// A mean below this releases the plate.
pub const RELEASE: f32 = 0.5;
/// The grind stops at or below this speed.
pub const GRIND_STOP: f32 = 0.001;
/// Volume of the click when the plate goes down.
pub const CLICK: f32 = 0.3;

/// What a plate tells its targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Off,
    On,
    Inverted,
}

/// One plate tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plate {
    pub pushed: bool,
    /// Clears `pushed` once a travel passes 1.
    pub one_shot: bool,
    pub invert: bool,
    /// The second target is signalled too.
    pub second: bool,
    /// The plate has clicked down.
    pub down: bool,
    pub speed: f32,
    pub previous: f32,
    pub has_target: bool,
    pub has_second: bool,
    /// The object signalled when the plate goes down or up.
    pub target: u32,
    pub second_target: u32,
}

/// What one tick reports. The travels are written in the slice passed in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlateTick {
    pub mean: f32,
    pub playing: bool,
    pub volume: f32,
    pub pitch: f32,
    pub signal: Option<Signal>,
    pub first: bool,
    pub second: bool,
    pub click: Option<f32>,
}

/// One tick. An empty plate does nothing.
pub fn plate_step(plate: &mut Plate, travels: &mut [f32]) -> PlateTick {
    if travels.is_empty() {
        return PlateTick {
            mean: plate.previous,
            playing: false,
            volume: 0.0,
            pitch: 0.0,
            signal: None,
            first: false,
            second: false,
            click: None,
        };
    }

    let mut sum = 0.0;
    for travel in travels.iter_mut() {
        if plate.pushed {
            *travel += RISE;
            if *travel > 1.0 {
                *travel = 1.0;
                if plate.one_shot {
                    plate.pushed = false;
                }
            }
        } else {
            *travel -= FALL;
            if *travel < 0.0 {
                *travel = 0.0;
            }
        }
        sum += *travel;
    }
    let mean = sum / travels.len() as f32;
    let change = (plate.previous - mean).abs();
    plate.speed = change * 0.9 + plate.speed * 0.1;
    plate.previous = mean;

    let (playing, volume, pitch) = if !plate.speed.is_finite() || plate.speed <= GRIND_STOP {
        (false, 0.0, 0.0)
    } else {
        let scaled = plate.speed * 50.0;
        (true, scaled * scaled, (plate.speed * 20.0 + 0.8).min(1.0))
    };

    let mut signal = None;
    let mut first = false;
    let mut second = false;
    let mut click = None;
    if mean > PRESS {
        if !plate.down {
            plate.down = true;
            click = Some(CLICK);
            let which = if plate.invert {
                Signal::Inverted
            } else {
                Signal::On
            };
            first = plate.has_target;
            second = plate.second && plate.has_second;
            if first || second {
                signal = Some(which);
            }
        }
    } else if mean < RELEASE && plate.down {
        plate.down = false;
        if !plate.invert {
            first = plate.has_target;
            second = plate.second && plate.has_second;
            if first || second {
                signal = Some(Signal::Off);
            }
        }
    }

    for travel in travels.iter_mut() {
        *travel = mean;
    }

    PlateTick {
        mean,
        playing,
        volume,
        pitch,
        signal,
        first,
        second,
        click,
    }
}

/// A push surface's operative record.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Record {
    /// The operative kind; 2 makes the signals inverted.
    pub kind: u8,
    /// The flags: one-shot and the second target.
    pub flags: u16,
    pub target: u32,
    pub second: u32,
}

/// The surface stops taking pushes once it is all the way down.
pub const ONE_SHOT: u16 = 2;
/// The second target is signalled too.
pub const SECOND: u16 = 0x10;
/// Signals are inverted.
pub const INVERTED: u16 = 8;
/// The kind that inverts the signals.
pub const INVERTED_KIND: u8 = 2;
/// The grind loop's sound name.
pub const GRIND_SOUND: &str = "Stnb_mov";
/// The click's sound name.
pub const CLICK_SOUND: &str = "Stnb_fit";

/// A push surface's mobility after the file's value: 0.3 when the file's
/// is at least 0.5, else 0.03.
pub fn mobility(file: f32) -> f32 {
    if 0.5 <= file { 0.3 } else { 0.03 }
}

impl Plate {
    /// A plate from its operative record and the object word's low byte.
    pub fn new(record: &Record, saved: u8) -> Self {
        Plate {
            pushed: false,
            one_shot: record.flags & ONE_SHOT != 0,
            invert: record.kind == INVERTED_KIND || record.flags & INVERTED != 0,
            second: record.flags & SECOND != 0,
            down: saved == 1,
            speed: 0.0,
            previous: 0.0,
            has_target: record.target != 0,
            has_second: record.second != 0,
            target: record.target,
            second_target: record.second,
        }
    }

    /// The object word's low byte this plate leaves behind.
    pub fn saved(&self) -> u8 {
        u8::from(self.down)
    }

    /// The click when the plate goes down.
    pub fn click(&self) -> crate::door::Sound {
        crate::door::Sound {
            volume: CLICK,
            volume_spread: 0.0,
            pitch: 0.9,
            pitch_spread: 0.2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plate() -> Plate {
        Plate {
            pushed: false,
            one_shot: false,
            invert: false,
            second: false,
            down: false,
            speed: 0.0,
            previous: 0.0,
            has_target: true,
            has_second: false,
            target: 1,
            second_target: 0,
        }
    }

    #[test]
    fn a_plate_reads_its_targets_and_flags_from_its_record() {
        let record = Record {
            kind: INVERTED_KIND,
            flags: ONE_SHOT | SECOND,
            target: 21,
            second: 22,
        };
        let plate = Plate::new(&record, 1);
        assert!(plate.one_shot && plate.invert && plate.second);
        assert!(plate.down && plate.has_target && plate.has_second);
        assert_eq!((plate.target, plate.second_target), (21, 22));
        assert_eq!(plate.saved(), 1);
        assert_eq!(Plate::new(&record, 0).saved(), 0);
        let flipped = Record {
            kind: 1,
            flags: INVERTED,
            ..record
        };
        assert!(Plate::new(&flipped, 0).invert);
        assert!((mobility(0.5) - 0.3).abs() < 1e-6);
        assert!((mobility(0.49) - 0.03).abs() < 1e-6);
        let click = plate.click();
        assert!((click.volume - CLICK).abs() < 1e-6);
        assert!((click.pitch - 0.9).abs() < 1e-6);
        assert!((click.pitch_spread - 0.2).abs() < 1e-6);
    }

    #[test]
    fn a_plate_rises_while_pressed_and_clicks_above_seven_tenths() {
        let mut still = plate();
        let tick = plate_step(&mut still, &mut []);
        assert!(!tick.playing);
        assert!((still.speed - 0.0).abs() < 1e-6);

        let mut rising = plate();
        rising.pushed = true;
        let mut travel = [0.0];
        let tick = plate_step(&mut rising, &mut travel);
        assert!((tick.mean - 0.02).abs() < 1e-5, "{}", tick.mean);
        assert!((travel[0] - tick.mean).abs() < 1e-6);
        assert!(tick.playing);
        assert!((tick.volume - 0.81).abs() < 1e-3, "{}", tick.volume);
        assert!((tick.pitch - 1.0).abs() < 1e-5);
        assert!(tick.signal.is_none());
        assert!(!rising.down);

        let mut below = plate();
        below.pushed = true;
        let mut travel = [0.68];
        let tick = plate_step(&mut below, &mut travel);
        assert!(tick.mean <= PRESS, "{}", tick.mean);
        assert!(!below.down);
        assert!(tick.click.is_none());

        let mut above = plate();
        above.pushed = true;
        let mut travel = [0.69];
        let tick = plate_step(&mut above, &mut travel);
        assert!(tick.mean > PRESS, "{}", tick.mean);
        assert!(above.down);
        assert_eq!(tick.signal, Some(Signal::On));
        assert_eq!(tick.click, Some(CLICK));

        let mut flipped = plate();
        flipped.pushed = true;
        flipped.invert = true;
        flipped.second = true;
        flipped.has_second = true;
        let mut travel = [0.69];
        let tick = plate_step(&mut flipped, &mut travel);
        assert_eq!(tick.signal, Some(Signal::Inverted));
        assert!(tick.first);
        assert!(tick.second);

        let mut held = plate();
        held.down = true;
        let mut travel = [0.51];
        let tick = plate_step(&mut held, &mut travel);
        assert!((tick.mean - 0.5).abs() < 1e-5, "{}", tick.mean);
        assert!(held.down);
        assert!(tick.signal.is_none());

        let mut let_go = plate();
        let_go.down = true;
        let mut travel = [0.5];
        let tick = plate_step(&mut let_go, &mut travel);
        assert!(tick.mean < RELEASE, "{}", tick.mean);
        assert!(!let_go.down);
        assert_eq!(tick.signal, Some(Signal::Off));

        let mut quiet = plate();
        quiet.down = true;
        quiet.invert = true;
        let mut travel = [0.5];
        let tick = plate_step(&mut quiet, &mut travel);
        assert!(!quiet.down);
        assert!(tick.signal.is_none());

        let mut once = plate();
        once.pushed = true;
        once.one_shot = true;
        let mut travel = [0.99, 0.5];
        plate_step(&mut once, &mut travel);
        assert!(!once.pushed);
        assert!((travel[0] - travel[1]).abs() < 1e-6);
        assert!(once.down);
    }
}
