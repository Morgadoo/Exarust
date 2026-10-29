//! The character's voice lines: a grunt, a groan and a scream, each with
//! the spacing the original keeps (`TMotilePart`'s voice request). The
//! body tick, the damage routine and the collapse ask for them; the voice
//! itself (three samples, a gain and a pitch) is the character's.
//!
//! Evidence: the part class's voice request. The character keeps three
//! samples, the time its voice is next free, two volume gains (applied one
//! after the other) and a pitch gain. The samples are not played by this
//! crate; the rules are here as plain data so the callers share them.

/// Which voice line a character makes, by the waits the request keeps.
/// The names are the callers' (the pain voice's grunt and groan); the sets
/// played are the voice's slots 1, 2 and 0 ([`CharacterVoice::set`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    /// Line 1: the short groan set (`_GroanS`), a 400 ms wait.
    Grunt,
    /// Line 2: the heavy groan set (`_GroanH`), a 500 ms wait.
    Groan,
    /// Line 3: the grunt set (`_GruntS`), at most once 300 ms after the
    /// last line, leaving no wait.
    Scream,
}

/// A played line: its volume and pitch after the character's gains, and
/// the game clock the next one may play.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spoken {
    pub volume: f32,
    pub pitch: f32,
    pub next_ms: u32,
}

/// One request for a line: which, whether its slot has a sound, when, and
/// the caller's volume and pitch before the character's gains.
#[derive(Debug, Clone, Copy)]
pub struct Ask {
    /// The line: 1 grunt, 2 groan, 3 scream; anything else is silent.
    pub line: i32,
    /// The line's slot holds a sound.
    pub has_sample: bool,
    pub now_ms: u32,
    pub volume: f32,
    pub pitch: f32,
    /// The character's two volume gains, applied in this order
    /// (`(volume · first) · second`: single-precision products are not
    /// associative), and its pitch gain.
    pub volume_gain: f32,
    pub volume_gain2: f32,
    pub pitch_gain: f32,
}

/// What one request did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Answer {
    /// It played, at these values.
    Played(Spoken),
    /// The slot has no sample, so nothing played and the wait is unchanged.
    Silent,
    /// The line was still on its wait: nothing played.
    Refused,
}

/// The voice's clock: the game time from which the next line may play.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Voice {
    pub last_ms: u32,
}

impl Voice {
    /// The wait each line leaves: 400 ms for a grunt, 500 for a groan, and
    /// a scream may follow the last line after 300 ms.
    pub const GRUNT_WAIT: u32 = 400;
    pub const GROAN_WAIT: u32 = 500;
    pub const SCREAM_WAIT: u32 = 300;

    /// Whether a line may play now (`now >= last` for a grunt or groan, and
    /// `now >= last + 300` for a scream, the sum taken without wrapping: a
    /// clock behind `last` refuses).
    pub fn ready(&self, line: Line, now_ms: u32) -> bool {
        match line {
            Line::Grunt | Line::Groan => now_ms >= self.last_ms,
            Line::Scream => {
                u64::from(self.last_ms) + u64::from(Self::SCREAM_WAIT) <= u64::from(now_ms)
            }
        }
    }

    /// One request. A request with a line at or below zero does nothing and
    /// is accepted (the original returns success).
    pub fn request(&mut self, ask: &Ask) -> Answer {
        if ask.line <= 0 {
            return Answer::Silent;
        }
        let line = match ask.line {
            1 => Line::Grunt,
            2 => Line::Groan,
            3 => Line::Scream,
            _ => return Answer::Silent,
        };
        if !ask.has_sample {
            return Answer::Silent;
        }
        if !self.ready(line, ask.now_ms) {
            return Answer::Refused;
        }
        let out = Spoken {
            volume: ask.volume * ask.volume_gain * ask.volume_gain2,
            pitch: ask.pitch * ask.pitch_gain,
            next_ms: match line {
                Line::Grunt => ask.now_ms.wrapping_add(Self::GRUNT_WAIT),
                Line::Groan => ask.now_ms.wrapping_add(Self::GROAN_WAIT),
                Line::Scream => ask.now_ms,
            },
        };
        self.last_ms = out.next_ms;
        Answer::Played(out)
    }
}

/// A character's voice: the four sound sets its init builds (the living
/// voice's from the race's first template, a feral one's undead voice from
/// the second), its two volume gains (the voice volume and the sound gain,
/// 1 from the constructor) and pitch gain, and the clock of its lines.
#[derive(bevy::prelude::Component, Debug, Clone, Default, PartialEq)]
pub struct CharacterVoice {
    pub grunt: String,
    pub groan_short: String,
    pub groan_heavy: String,
    pub scream: String,
    /// The wound sound set ([`crate::pain::wound_set`]), if any.
    pub wound: Option<String>,
    pub volume: f32,
    pub gain: f32,
    pub pitch: f32,
    pub clock: Voice,
}

impl CharacterVoice {
    /// The set line `line` plays: 1 the short groan, 2 the heavy groan, 3
    /// the grunt (the voice entry's slots 1, 2 and 0); none otherwise.
    pub fn set(&self, line: i32) -> Option<&str> {
        match line {
            1 => Some(&self.groan_short),
            2 => Some(&self.groan_heavy),
            3 => Some(&self.grunt),
            _ => None,
        }
    }

    /// One request of line `line` at `now_ms` with the caller's volume and
    /// pitch, `has_sample` saying whether its set holds a sound.
    pub fn speak(
        &mut self,
        line: i32,
        has_sample: bool,
        now_ms: u32,
        volume: f32,
        pitch: f32,
    ) -> Answer {
        self.clock.request(&Ask {
            line,
            has_sample,
            now_ms,
            volume,
            pitch,
            volume_gain: self.volume,
            volume_gain2: self.gain,
            pitch_gain: self.pitch,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A request at the default gains (1) and the given values.
    fn ask(line: i32, has_sample: bool, now_ms: u32, volume: f32, pitch: f32) -> Ask {
        Ask {
            line,
            has_sample,
            now_ms,
            volume,
            pitch,
            volume_gain: 1.0,
            volume_gain2: 1.0,
            pitch_gain: 1.0,
        }
    }

    /// Line 1 plays the short groan, 2 the heavy, 3 the grunt; the gains
    /// apply volume first, then the sound gain.
    #[test]
    fn the_lines_play_their_slots() {
        let mut voice = CharacterVoice {
            grunt: "vhm01_GruntS".into(),
            groan_short: "vhm01_GroanS".into(),
            groan_heavy: "vhm01_GroanH".into(),
            volume: 1.5,
            gain: 1.0,
            pitch: 0.9,
            ..CharacterVoice::default()
        };
        assert_eq!(voice.set(1), Some("vhm01_GroanS"));
        assert_eq!(voice.set(2), Some("vhm01_GroanH"));
        assert_eq!(voice.set(3), Some("vhm01_GruntS"));
        assert_eq!(voice.set(4), None);
        let Answer::Played(s) = voice.speak(1, true, 100, 0.4, 1.0) else {
            panic!("refused");
        };
        assert_eq!((s.volume, s.pitch, s.next_ms), (0.4 * 1.5, 0.9, 500));
        assert_eq!(voice.speak(2, true, 499, 0.4, 1.0), Answer::Refused);
    }

    #[test]
    fn the_voice_waits() {
        let mut voice = Voice::default();
        // A grunt at 1000: volume 0.4 · 1.5 · 0.5, pitch 0.8 · 2, wait 400.
        assert_eq!(
            voice.request(&Ask {
                volume_gain: 1.5,
                volume_gain2: 0.5,
                pitch_gain: 2.0,
                ..ask(1, true, 1000, 0.4, 0.8)
            }),
            Answer::Played(Spoken {
                volume: 0.4 * 1.5 * 0.5,
                pitch: 0.8 * 2.0,
                next_ms: 1400,
            })
        );
        assert_eq!(
            voice.request(&Ask {
                volume_gain: 1.5,
                volume_gain2: 0.5,
                pitch_gain: 2.0,
                ..ask(1, true, 1399, 0.4, 0.8)
            }),
            Answer::Refused
        );
        assert_eq!(
            voice.request(&ask(2, true, 1400, 0.2, 0.9)),
            Answer::Played(Spoken {
                volume: 0.2,
                pitch: 0.9,
                next_ms: 1900,
            })
        );
        // A scream may follow the last line after 300 ms.
        assert_eq!(
            voice.request(&ask(3, true, 2199, 0.5, 1.0)),
            Answer::Refused
        );
        let scream = voice.request(&ask(3, true, 2200, 0.5, 1.0));
        assert!(matches!(scream, Answer::Played(_)));
        // And then 300 ms after itself.
        assert_eq!(
            voice.request(&ask(3, true, 2499, 0.5, 1.0)),
            Answer::Refused
        );
        assert!(matches!(
            voice.request(&ask(3, true, 2500, 0.5, 1.0)),
            Answer::Played(_)
        ));
    }

    #[test]
    fn the_volume_gains_apply_in_order() {
        // (v · a) · b, not v · (a · b): the two differ in the last bit.
        let (v, a, b) = (0.7_f32, 1.1_f32, 0.9_f32);
        assert_ne!((v * a) * b, v * (a * b));
        let mut voice = Voice::default();
        let Answer::Played(spoken) = voice.request(&Ask {
            volume_gain: a,
            volume_gain2: b,
            ..ask(1, true, 0, v, 1.0)
        }) else {
            panic!("a grunt on a free voice plays");
        };
        assert_eq!(spoken.volume, (v * a) * b);
    }

    #[test]
    fn a_scream_does_not_wrap_before_the_clock() {
        // The wait is added to the last time without wrapping: a clock
        // behind the last line refuses, and so does one just short of it.
        let mut voice = Voice { last_ms: 5000 };
        assert_eq!(voice.request(&ask(3, true, 100, 0.5, 1.0)), Answer::Refused);
        assert_eq!(
            voice.request(&ask(3, true, 5299, 0.5, 1.0)),
            Answer::Refused
        );
        assert!(matches!(
            voice.request(&ask(3, true, 5300, 0.5, 1.0)),
            Answer::Played(_)
        ));
        // A grunt compares without a wait: refused only before `last`.
        let mut voice = Voice { last_ms: 5000 };
        assert_eq!(
            voice.request(&ask(1, true, 4999, 0.5, 1.0)),
            Answer::Refused
        );
    }

    #[test]
    fn a_missing_sample_or_line_is_silent() {
        let mut voice = Voice::default();
        assert_eq!(
            voice.request(&ask(1, false, 5000, 1.0, 1.0)),
            Answer::Silent
        );
        assert_eq!(voice.last_ms, 0);
        assert_eq!(voice.request(&ask(0, true, 5000, 1.0, 1.0)), Answer::Silent);
        assert_eq!(
            voice.request(&ask(-1, true, 5000, 1.0, 1.0)),
            Answer::Silent
        );
        assert_eq!(voice.request(&ask(9, true, 5000, 1.0, 1.0)), Answer::Silent);
    }
}
