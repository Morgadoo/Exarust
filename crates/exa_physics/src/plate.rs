//! A pressure plate: its travel, its grind, and the click when it goes down.
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
//! inverted. A mean below 0.5 releases a plate that was down and signals 0,
//! unless inverted. A mean of 0.7 does not press. A mean of 0.5 does not
//! release.
//!
//! Evidence: the pressure-plate step. A body standing on the plate is not
//! in this slice.

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
        }
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
