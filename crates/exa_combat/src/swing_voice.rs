//! The swing's voice: the whoosh a held weapon keeps while it swings, as
//! the original's swing routine and body tick drive it.
//!
//! The voice starts, silent, the first attack tick the clip is in the
//! swing's region ([`crate::swing::SwingState::in_region`]), at the
//! weapon's swing pitch (W26) × 0.4; the same start rustles the clothing
//! once its foley clock allows. Every attack tick after that sets the
//! voice's volume and pitch from the tracked speed
//! ([`crate::swing::swing_sound`]), the tracker of the second hand for a
//! two-weapon fighter's other-side attacks ([`voice_speed2`]). Leaving the
//! region lets the next entry start the voice again. Every body tick of a
//! body that is not attacking fades each held item's playing voice by 0.01
//! of volume and 0.0005 of pitch, and asks it to stop under 0.005
//! ([`fade_voice`]).
//!
//! Not wired: Exarust plays one whoosh when the strike window opens
//! (`exa_audio`); these are tested rules with the original's numbers.

use crate::attack::AttackMotion;

/// The voice's start pitch: W26 × 0.4, in double.
const START_PITCH: f64 = 0.4;
/// The fade a body tick takes off a held voice's volume and pitch, in
/// double, and the volume under which the voice is asked to stop.
const FADE_VOLUME: f64 = 0.01;
const FADE_PITCH: f64 = 0.0005;
const STOP_UNDER: f64 = 0.005;
/// The swing start's rustle: the foley volume × (0.02·r + 0.02), the pitch
/// 0.4·r + 0.8, and its clock's wait: 400 ms plus up to 99 more.
const RUSTLE_SHARE: f64 = 0.02;
const RUSTLE_PITCH_SPAN: f64 = 0.4;
const RUSTLE_PITCH_BASE: f64 = 0.8;
const RUSTLE_WAIT_MS: u32 = 400;
const RUSTLE_JITTER_MS: u64 = 100;
/// The weapon-class bit of a fighter striking with one hand at a time
/// (two weapons, fists, claws).
const ONE_HAND_STRIKES: u8 = 0x80;

/// The swing voice's state on a fighter: whether it has started in the
/// current pass through the swing's region.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SwingVoice {
    pub started: bool,
}

/// A voice start: its volume (silent) and pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceStart {
    pub volume: f32,
    pub pitch: f32,
}

impl SwingVoice {
    /// One attack tick's start test: outside the region the voice may start
    /// again (and nothing plays); inside it, the first tick starts it.
    /// Returns the start when the fighter has a voice to start
    /// (`has_voice`: its source and its swing set); the start counts
    /// either way, as the original marks it started without one.
    pub fn enter(&mut self, region: bool, has_voice: bool, swing_pitch: f32) -> Option<VoiceStart> {
        if !region {
            self.started = false;
            return None;
        }
        if self.started {
            return None;
        }
        self.started = true;
        has_voice.then(|| VoiceStart {
            volume: 0.0,
            pitch: (f64::from(swing_pitch) * START_PITCH) as f32,
        })
    }
}

/// The tracked speed² the voice follows: the second hand's tracker for a
/// fighter striking one hand at a time in an other-side attack (actions
/// 8 to 10), else the first's.
pub fn voice_speed2(trackers: [f32; 2], weapon_class: u8, motion: AttackMotion) -> f32 {
    if weapon_class & ONE_HAND_STRIKES != 0 && matches!(motion, AttackMotion::Other(_)) {
        trackers[1]
    } else {
        trackers[0]
    }
}

/// The rustle a swing start makes, when the foley clock allows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rustle {
    /// The rustle's volume, which the character keeps.
    pub volume: f32,
    /// The volume played: that volume × the sound gain.
    pub gain: f32,
    pub pitch: f32,
    /// The foley clock's next free time (ms).
    pub next_ms: u32,
}

/// The swing start's rustle: only while the foley clock (`next_ms`, the
/// unsigned compare, so a body without foley, all ones, never rustles) is
/// before `now_ms`. The volume is `foley_volume · (0.02·r_volume + 0.02)`
/// and the pitch `0.4·r_pitch + 0.8`, in double; the played gain the
/// single product volume × `sound_gain`. The engine's generator (`roll`)
/// then steps once and the clock waits 400 ms plus `(roll · 100) >> 32`.
pub fn swing_rustle(
    foley_volume: f32,
    sound_gain: f32,
    now_ms: u32,
    next_ms: u32,
    r_volume: f32,
    r_pitch: f32,
    roll: &mut u32,
) -> Option<Rustle> {
    if next_ms >= now_ms {
        return None;
    }
    let share = f64::from(r_volume) * RUSTLE_SHARE + RUSTLE_SHARE;
    let volume = (f64::from(foley_volume) * share) as f32;
    let pitch = (f64::from(r_pitch) * RUSTLE_PITCH_SPAN + RUSTLE_PITCH_BASE) as f32;
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    let jitter = ((u64::from(*roll) * RUSTLE_JITTER_MS) >> 32) as u32;
    Some(Rustle {
        volume,
        gain: volume * sound_gain,
        pitch,
        next_ms: jitter.wrapping_add(RUSTLE_WAIT_MS).wrapping_add(now_ms),
    })
}

/// One body tick's fade of a playing held voice: the volume less 0.01 and
/// the pitch less 0.0005, each in double and narrowed; returns the new
/// volume and pitch and whether the voice is asked to stop (volume under
/// 0.005; a NaN volume is not). The body tick fades only while the body
/// is not attacking.
pub fn fade_voice(volume: f32, pitch: f32) -> (f32, f32, bool) {
    let volume = (f64::from(volume) - FADE_VOLUME) as f32;
    let pitch = (f64::from(pitch) - FADE_PITCH) as f32;
    (volume, pitch, f64::from(volume) < STOP_UNDER)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The voice starts silent at 0.4 × W26 on the region's first tick,
    /// not again inside it, and again after leaving it; without a voice
    /// the start still counts.
    #[test]
    fn the_voice_starts_once_per_pass_through_the_region() {
        let mut voice = SwingVoice::default();
        assert_eq!(voice.enter(false, true, 1.5), None);
        let start = voice.enter(true, true, 1.5).unwrap();
        assert_eq!(start.volume, 0.0);
        assert!((start.pitch - 0.6).abs() < 1e-7);
        assert_eq!(voice.enter(true, true, 1.5), None);
        assert_eq!(voice.enter(false, true, 1.5), None);
        assert!(!voice.started);
        assert_eq!(voice.enter(true, false, 1.5), None);
        assert!(voice.started);
    }

    /// A one-hand striker's other-side attack follows the second tracker;
    /// any other attack, or a two-hander, the first.
    #[test]
    fn the_voice_follows_the_striking_hand() {
        let t = [4.0, 9.0];
        assert_eq!(voice_speed2(t, 0x80, AttackMotion::Other(1)), 9.0);
        assert_eq!(voice_speed2(t, 0x80, AttackMotion::Main(1)), 4.0);
        assert_eq!(voice_speed2(t, 0x10, AttackMotion::Other(1)), 4.0);
    }

    /// With the clock free the rustle at foley volume 2 and draws 0.5 has
    /// a volume of 2 × 0.03 = 0.06 and a pitch of 1.0, and waits 400 ms
    /// plus its jitter; with the clock not yet passed, or all ones, none.
    #[test]
    fn a_swing_start_rustles_when_its_clock_allows() {
        let mut roll = 1;
        let r = swing_rustle(2.0, 0.5, 1_000, 999, 0.5, 0.5, &mut roll).unwrap();
        assert!((r.volume - 0.06).abs() < 1e-7);
        assert!((r.gain - 0.03).abs() < 1e-7);
        assert!((r.pitch - 1.0).abs() < 1e-7);
        assert_eq!(roll, 0x0808_8406);
        let jitter = ((u64::from(roll) * 100) >> 32) as u32;
        assert_eq!(r.next_ms, 1_400 + jitter);
        let mut roll = 1;
        assert_eq!(
            swing_rustle(2.0, 1.0, 1_000, 1_000, 0.5, 0.5, &mut roll),
            None
        );
        assert_eq!(
            swing_rustle(2.0, 1.0, 1_000, u32::MAX, 0.5, 0.5, &mut roll),
            None
        );
        assert_eq!(roll, 1, "no rustle steps no generator");
    }

    /// A tick takes 0.01 off the volume and 0.0005 off the pitch; the voice
    /// stops once the volume is under 0.005, and a NaN volume never stops.
    #[test]
    fn a_held_voice_fades_and_stops() {
        let (v, p, stop) = fade_voice(0.5, 1.0);
        assert!((v - 0.49).abs() < 1e-7 && (p - 0.9995).abs() < 1e-7 && !stop);
        let (v, _, stop) = fade_voice(0.015, 1.0);
        assert!((v - 0.005).abs() < 1e-7);
        assert_eq!(stop, f64::from(v) < 0.005);
        assert!(fade_voice(0.0, 1.0).2);
        assert!(!fade_voice(f32::NAN, 1.0).2);
    }
}
