//! The transition screen between game states: the fade to black, the
//! switch callback once black, the hold, and the fade back in.
//!
//! The fade takes 600 ms, or 800/300/2000 by the mode's flags, while the
//! music fades with it. Once black the screen runs the switch callback,
//! holds three frames and then the mode's 300, 500 or 1000 ms, and fades
//! back in over 600 ms before it closes. State 2 is the "TO BE CONTINUED"
//! hold, whose overlay rises to 0.6 over six seconds and whose string is
//! drawn centred.
//!
//! Evidence: the transition screen's run and draw routines; the flag
//! meanings are read from the branches and labelled as guesses where the
//! caller is not in the pack.

/// The fade-in lengths, in clock units.
pub const FADE_MS: f64 = 600.0;
pub const FADE_HOLD_1000_MS: f64 = 2000.0;
pub const FADE_HOLD_300_MS: f64 = 300.0;
pub const FADE_SKIP_MS: f64 = 800.0;
/// The "to be continued" hold before its callback, clock units.
pub const CONTINUED_HOLD_MS: u32 = 6000;
/// The overlay's cap during that hold.
pub const CONTINUED_LEVEL: f32 = 0.6;
/// The wait before the delayed switch, clock units.
pub const LONG_WAIT_MS: u32 = 10_000;
/// The dark overlay the transition drives.
pub const OVERLAY_ALPHA: f32 = 255.0;
/// The transition screen's string, as the original's bytes run.
pub const TRANSITION_TEXT: &str = "TO BE CONTINUED";

/// The mode flags the transition is opened with. The names come from the
/// branches; the caller's argument table is not in the pack [guess].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransitionFlags {
    /// Fades the music and ends the audio environment at black.
    pub fade_audio: bool,
    /// Skips the switch callback and starts the hold at once.
    pub skip_callback: bool,
    /// Holds 2000 ms in instead of 500, and 1000 ms after the switch.
    pub hold_1000: bool,
    /// Fades over 300 ms, holds 300 ms and clears the flag when held.
    pub hold_300: bool,
    /// Waits ten seconds before switching and does not call back.
    pub long_wait: bool,
}

/// The transition's phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Phase {
    #[default]
    FadingOut,
    Holding,
    Continued,
    SwitchNow,
    /// Two frames after the switch.
    HoldFrame,
    /// One frame after that.
    HoldFrame2,
    InHold,
    FadingIn,
    Done,
}

/// The transition screen's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionScreen {
    pub phase: Phase,
    /// The clock the current phase started at.
    pub start: u32,
    pub flags: TransitionFlags,
    /// The screen's alpha byte.
    pub alpha: u8,
    /// The "TO BE CONTINUED" text overlay's alpha byte.
    pub overlay: u8,
    /// The fade level the overlay uses, 0…1.
    pub level: f32,
    /// The music's level while fading.
    pub music: f32,
    /// The music's level before the fade started.
    pub music_saved: Option<f32>,
    /// The lowest overlay the black has reached (its complement is mixed
    /// in when fading back in).
    pub dim: f32,
    /// The frame counter that the screen consumes once.
    pub countdown: i32,
    /// The switch callback ran.
    pub callback: bool,
    /// The form should close.
    pub close: bool,
}

/// One step's draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionStep {
    pub alpha: u8,
    pub close: bool,
    pub callback: bool,
    /// The draw shows the "TO BE CONTINUED" text.
    pub continued: bool,
}

impl TransitionScreen {
    pub fn new(now: u32, flags: TransitionFlags, music: f32) -> Self {
        Self {
            phase: Phase::FadingOut,
            start: now,
            flags,
            alpha: 0,
            overlay: 0,
            level: 0.0,
            music,
            music_saved: None,
            dim: 0.0,
            countdown: 2,
            callback: false,
            close: false,
        }
    }

    /// Enters the "TO BE CONTINUED" hold at `now`.
    pub fn begin_continued(&mut self, now: u32) {
        self.phase = Phase::Continued;
        self.start = now;
    }

    /// The fade's rate, per clock unit.
    fn rate(&self) -> f64 {
        if self.flags.skip_callback {
            0.00125
        } else if self.flags.hold_300 {
            0.0033333333333333335
        } else if self.flags.hold_1000 {
            0.0005
        } else {
            0.0016666666666666668
        }
    }

    /// One frame of the state machine.
    pub fn step(&mut self, now: u32) -> TransitionStep {
        let elapsed = u64::from(now.wrapping_sub(self.start));
        match self.phase {
            Phase::FadingOut => {
                let rate = self.rate();
                let mut t = (elapsed as f64 * rate) as f32;
                if self.flags.fade_audio && self.music_saved.is_none() {
                    self.music_saved = Some(self.music);
                }
                if let Some(saved) = self.music_saved {
                    if self.flags.fade_audio {
                        self.music = (1.0 - t) * saved;
                    }
                }
                if t > 1.0 {
                    if self.flags.fade_audio {
                        self.music = 0.0;
                        self.dim = 0.0;
                    }
                    t = 1.0;
                    if !self.flags.skip_callback {
                        if !self.flags.long_wait {
                            self.phase = Phase::Holding;
                            self.callback = true;
                            self.start = now;
                        } else if elapsed > u64::from(LONG_WAIT_MS) {
                            self.phase = Phase::Holding;
                            self.start = now;
                        }
                    } else {
                        self.phase = Phase::Holding;
                        self.start = now;
                        self.overlay = 0;
                    }
                }
                self.level = t;
                self.alpha = (t * OVERLAY_ALPHA).round_ties_even() as u8;
            }
            Phase::Holding => {
                self.phase = Phase::SwitchNow;
                self.start = now;
                self.level = 1.0;
            }
            Phase::Continued => {
                if elapsed > u64::from(CONTINUED_HOLD_MS) {
                    self.flags.skip_callback = false;
                    self.phase = Phase::SwitchNow;
                    self.callback = true;
                } else {
                    let t = (elapsed as f64 * 0.0005) as f32;
                    let t = if t > CONTINUED_LEVEL {
                        CONTINUED_LEVEL
                    } else {
                        t
                    };
                    self.overlay = (t * OVERLAY_ALPHA).round_ties_even() as u8;
                }
                self.level = 1.0;
            }
            Phase::SwitchNow => {
                self.start = now;
                self.phase = Phase::HoldFrame;
                self.level = 1.0;
            }
            Phase::HoldFrame => {
                self.start = now;
                self.phase = Phase::HoldFrame2;
                self.level = 1.0;
                if self.countdown > 0 {
                    self.countdown -= 1;
                }
            }
            Phase::HoldFrame2 => {
                let hold: u32 = if self.flags.hold_1000 {
                    1000
                } else if self.flags.hold_300 {
                    300
                } else {
                    500
                };
                self.level = 1.0;
                if elapsed > u64::from(hold) {
                    self.flags.hold_1000 = false;
                    self.flags.hold_300 = false;
                    self.start = now;
                    self.phase = Phase::InHold;
                }
            }
            Phase::InHold => {
                self.level = 1.0;
            }
            Phase::FadingIn | Phase::Done => {
                let t = 1.0 - (elapsed as f64 * 0.0016666666666666668) as f32;
                if self.dim < 1.0 {
                    self.dim = 1.0 - t;
                }
                if t <= 0.0 {
                    self.dim = 1.0;
                    self.level = 0.0;
                    self.phase = Phase::Done;
                    self.close = true;
                    self.alpha = 0;
                } else {
                    self.level = t;
                    self.alpha = (t * OVERLAY_ALPHA).round_ties_even() as u8;
                }
            }
        }
        TransitionStep {
            alpha: self.alpha,
            close: self.close,
            callback: std::mem::take(&mut self.callback),
            continued: self.phase == Phase::Continued,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags() -> TransitionFlags {
        TransitionFlags::default()
    }

    #[test]
    fn the_fade_out_takes_six_hundred() {
        let mut t = TransitionScreen::new(
            1000,
            TransitionFlags {
                fade_audio: true,
                ..flags()
            },
            0.8,
        );
        // 300 of 600: half.
        let s = t.step(1300);
        assert_eq!(s.alpha, 128);
        assert_eq!(t.music, 0.4);
        let s = t.step(1601);
        assert_eq!(s.alpha, 255);
        assert!(s.callback);
        assert_eq!(t.phase, Phase::Holding);
    }

    #[test]
    fn the_mode_flags_pick_their_fades() {
        let mut t = TransitionScreen::new(
            0,
            TransitionFlags {
                hold_300: true,
                ..flags()
            },
            1.0,
        );
        let s = t.step(150);
        assert_eq!(s.alpha, 128);
        let mut t = TransitionScreen::new(
            0,
            TransitionFlags {
                hold_1000: true,
                ..flags()
            },
            1.0,
        );
        let s = t.step(1000);
        assert_eq!(s.alpha, 128);
        let mut t = TransitionScreen::new(
            0,
            TransitionFlags {
                skip_callback: true,
                ..flags()
            },
            1.0,
        );
        let s = t.step(400);
        assert_eq!(s.alpha, 128);
        // At black the callback is skipped and the alpha is cleared.
        let s = t.step(801);
        assert!(!s.callback);
        assert_eq!(t.overlay, 0);
        assert_eq!(s.alpha, 255);
        assert_eq!(t.phase, Phase::Holding);
    }

    #[test]
    fn the_hold_and_fade_back_run_in_order() {
        let mut t = TransitionScreen::new(0, flags(), 1.0);
        t.step(601);
        assert_eq!(t.phase, Phase::Holding);
        t.step(601);
        assert_eq!(t.phase, Phase::SwitchNow);
        t.step(601);
        assert_eq!(t.phase, Phase::HoldFrame);
        assert_eq!(t.countdown, 2);
        t.step(601);
        assert_eq!(t.phase, Phase::HoldFrame2);
        assert_eq!(t.countdown, 1);
        // The 500 ms hold keeps the screen black.
        t.step(1000);
        assert_eq!(t.phase, Phase::HoldFrame2);
        t.step(1102);
        assert_eq!(t.phase, Phase::InHold);
        // The state machine has no branch for InHold: it stays until the
        // caller moves it to FadingIn.
        t.phase = Phase::FadingIn;
        t.start = 1100;
        let s = t.step(1400);
        assert_eq!(s.alpha, 128);
        assert!(!s.close);
        let s = t.step(1700);
        assert!(s.close);
        assert_eq!(s.alpha, 0);
        assert_eq!(t.phase, Phase::Done);
    }

    #[test]
    fn the_to_be_continued_overlay_rises_to_six_tenths() {
        let mut t = TransitionScreen::new(0, flags(), 1.0);
        t.begin_continued(1000);
        let s = t.step(2000);
        assert!(s.continued);
        assert_eq!(t.overlay, 128);
        let s = t.step(3000);
        // (2000 × 0.0005) = 1, capped at 0.6 → 153.
        assert_eq!(t.overlay, 153);
        let s = t.step(7001);
        assert!(!s.continued);
        assert!(s.callback);
    }
}
