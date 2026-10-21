//! The character's voice lines: a grunt, a groan and a scream, each with
//! the spacing the original keeps (`TMotilePart`'s voice request). The
//! body tick, the damage routine and the collapse ask for them; the voice
//! itself (three samples, a gain and a pitch) is the character's.
//!
//! Evidence: the part class notes (`+0xf30` voices, `+0xf4c` the last
//! request's time, `+0xf38`/`+0xf58` the volume gains and `+0xf3c` the
//! pitch gain). The samples are not played by this crate; the rules are
//! here as plain data so the callers share them.

/// Which voice line a character makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    /// The grunt: slot 1, the shortest wait.
    Grunt,
    /// The groan: slot 2.
    Groan,
    /// The scream: slot 3, at most once 300 ms after the last line.
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
    /// The character's volume and pitch gains (`+0xf58`, `+0xf3c`).
    pub volume_gain: f32,
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

/// The voice's clock: the game time of the last line played (`+0xf4c`).
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
    /// `now >= last + 300` for a scream).
    pub fn ready(&self, line: Line, now_ms: u32) -> bool {
        match line {
            Line::Grunt | Line::Groan => now_ms >= self.last_ms,
            Line::Scream => now_ms.wrapping_sub(self.last_ms) >= Self::SCREAM_WAIT,
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
            volume: ask.volume * ask.volume_gain,
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
            pitch_gain: 1.0,
        }
    }

    #[test]
    fn the_voice_waits() {
        let mut voice = Voice::default();
        // A grunt at 1000: volume 0.4 · 1.5, pitch 0.8 · 2, wait 400.
        assert_eq!(
            voice.request(&Ask {
                volume_gain: 1.5,
                pitch_gain: 2.0,
                ..ask(1, true, 1000, 0.4, 0.8)
            }),
            Answer::Played(Spoken {
                volume: 0.4 * 1.5,
                pitch: 0.8 * 2.0,
                next_ms: 1400,
            })
        );
        assert_eq!(
            voice.request(&Ask {
                volume_gain: 1.5,
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
