//! Music: one main track (A), one incoming or overlay track (B), and the
//! state machine that crossfades between them.
//!
//! Both tracks are ordinary voices on the interface bus, so their gains go
//! through the mixer's bus and source rules. All fades last 3000 ms on the
//! real-time clock. A crossfade ramps B up linearly while A falls as
//! `(1 − t)²`; an overlay fade-in does the same but A ends paused; a fade-out
//! restores A linearly while B falls. Starting a fade during the opposite one
//! mirrors its progress so the gains do not jump.
//!
//! Observed quirks kept: the outgoing track's volume stays A's nominal volume
//! after a crossfade (the original writes the promoted volume to a global
//! nothing reads), the incoming buffer is neither released nor forgotten when
//! an overlay ends, stopping A forgets its buffer without releasing it and
//! leaves the crossfader running, and an outgoing A without a source is not
//! freed when a crossfade ends.

use bevy::prelude::*;
use exa_assets::Game;

use crate::SoundBank;
use crate::environment::{AudioEnvironment, SourceCommand, voice_flags};

/// The level track's volume.
pub const MUSIC_VOLUME: f32 = 0.45;
/// Every fade lasts three seconds.
pub const FADE_MS: u32 = 3000;
/// One millisecond as a fraction of a fade, the constant the original uses.
pub const INV_FADE_MS: f64 = 0.0003333333333333333;
/// A music or ambience voice: interface bus, listener-relative, unattenuated,
/// looping, restart.
pub const MUSIC_FLAGS: u32 = voice_flags::INTERFACE
    | voice_flags::LISTENER_RELATIVE
    | voice_flags::NO_ATTENUATION
    | voice_flags::LOOPING
    | voice_flags::RESTART;
/// Nothing playing.
pub const IDLE: u32 = 0;
/// B fades in over A.
pub const OVERLAY_IN: u32 = 0x10;
/// B is at full volume and A is paused.
pub const OVERLAY_HELD: u32 = 0x11;
/// B fades out and A comes back.
pub const OVERLAY_OUT: u32 = 0x12;
/// B is the new track and A fades out.
pub const CROSSFADE: u32 = 0x20;
/// The smallest nominal volume that music on/off acts on.
pub const MIN_VOLUME: f32 = 0.01;

/// The level's music or ambience track (set by the level loader).
#[derive(Resource, Clone, Debug)]
pub struct LevelMusic(pub String);

/// The volume the level's track plays at, when the level's mode sets one
/// (an arena match plays each track at its own volume). Without it the
/// track plays at [`MUSIC_VOLUME`].
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct LevelMusicVolume(pub f32);

/// Where the level's track starts, in seconds, when its mode picks a start
/// (the arena hub starts its track at one of four places).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct LevelMusicStart(pub f32);

/// Music for a level name, following the original's track naming. A story
/// level (`exanima0…`, `exanimac…`) plays what [`story_music`] picks.
pub fn music_for_level(level: &str) -> &'static str {
    let l = level.to_ascii_lowercase();
    if l.starts_with("arenahub") {
        "msc_hub01"
    } else if l.starts_with("arenao") {
        "msc_arenao"
    } else if l.starts_with("arena") {
        "msc_arena3"
    } else if l.starts_with("exanima0") || l.starts_with("exanimac") {
        story_music(level.as_bytes()).key
    } else {
        "ambntra01"
    }
}

/// What the story's level hook plays for a level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoryMusic {
    /// The track as the original names it, and the sound bank's key.
    pub track: &'static str,
    pub key: &'static str,
    pub volume: f32,
    /// The ambience looped with it at full volume; `None` stops the loop.
    pub ambience: Option<&'static str>,
    /// The word the level's indoor preset sets after its reset (which
    /// leaves 2.0): 1.8, 1.4 or 2.0. What the word drives is not traced.
    pub preset: f32,
}

const fn story(
    track: &'static str,
    key: &'static str,
    volume_bits: u32,
    ambience: Option<&'static str>,
    preset_bits: u32,
) -> StoryMusic {
    StoryMusic {
        track,
        key,
        volume: f32::from_bits(volume_bits),
        ambience,
        preset: f32::from_bits(preset_bits),
    }
}

const PRESET_A: u32 = 0x3fe6_6666;
const PRESET_B: u32 = 0x3fb3_3333;
const PRESET_C: u32 = 0x4000_0000;

/// The story level hook's choice, from bytes of the 16-byte level name
/// (missing bytes read as 0), tested in this order: byte 11 `w`; byte 7
/// `6` with byte 8 `7`; byte 8 `4`, `6`, `5`, `7`, `8`, `2`; byte 7 `c`;
/// else the first level's track. Only byte 8 `4` loops an ambience.
pub fn story_music(name: &[u8]) -> StoryMusic {
    let at = |i: usize| name.get(i).copied().unwrap_or(0);
    if at(11) == b'w' {
        story(
            "Msc_SharpNatural",
            "msc_sharpnatural",
            0x3dcc_cccd,
            None,
            PRESET_A,
        )
    } else if at(7) == b'6' && at(8) == b'7' {
        story("Msc_GardensA", "msc_gardensa", 0x3e99_999a, None, PRESET_A)
    } else {
        match at(8) {
            b'4' => story(
                "Msc_UWB2_Amb",
                "msc_uwb2_amb",
                0x3e19_999a,
                Some("AmbUWBA"),
                PRESET_B,
            ),
            b'6' => story("Msc_UWB5_Amb", "msc_uwb5_amb", 0x3e80_0000, None, PRESET_C),
            b'5' => story("Msc_UWB4_Amb", "msc_uwb4_amb", 0x3e4c_cccd, None, PRESET_A),
            b'7' => story(
                "Msc_CountfourA",
                "msc_countfoura",
                0x3dcc_cccd,
                None,
                PRESET_C,
            ),
            b'8' => story("Msc_UWB6_Amb", "msc_uwb6_amb", 0x3dcc_cccd, None, PRESET_C),
            b'2' => story("Msc_UW01AmbB", "msc_uw01ambb", 0x3e19_999a, None, PRESET_A),
            _ if at(7) == b'c' => {
                story("Msc_CryptAmb", "msc_cryptamb", 0x3e80_0000, None, PRESET_A)
            }
            _ => story("Msc_UW01Amb", "msc_uw01amb", 0x3dcc_cccd, None, PRESET_A),
        }
    }
}

/// Whether entering a level changes the music: the story's check picks the
/// track as [`story_music`] does, except that byte 8 `7` names
/// `Msc_Countfour`, and compares it with the one playing on their first 15
/// characters.
pub fn story_music_changes(name: &[u8], playing: &str) -> bool {
    let picked = story_music(name).track;
    let track = if picked == "Msc_CountfourA" {
        "Msc_Countfour"
    } else {
        picked
    };
    let short = |s: &str| s.bytes().take(15).collect::<Vec<_>>();
    short(track) != short(playing)
}

/// The crossfader's state: two voices with their buffers and nominal volumes.
#[derive(Debug, Clone)]
pub struct MusicState {
    /// The "Music" setting.
    pub enabled: bool,
    /// The track the level or caller asked for.
    pub name: String,
    /// One of the state constants.
    pub state: u32,
    /// Real time in milliseconds the current fade started.
    pub started_ms: u32,
    /// The main track's voice.
    pub a: Option<u32>,
    /// The main track's buffer reference.
    pub a_buffer: u32,
    /// The main track's nominal volume.
    pub a_volume: f32,
    /// The incoming or overlay track's voice.
    pub b: Option<u32>,
    /// The incoming track's buffer reference.
    pub b_buffer: u32,
    /// The incoming track's nominal volume.
    pub b_volume: f32,
}

impl Default for MusicState {
    fn default() -> Self {
        Self {
            // The original ships with music enabled.
            enabled: true,
            name: String::new(),
            state: IDLE,
            started_ms: 0,
            a: None,
            a_buffer: 0,
            a_volume: 0.0,
            b: None,
            b_buffer: 0,
            b_volume: 0.0,
        }
    }
}

/// The fade's fraction at `now`, from the state's start: `(now − start)/3000`
/// in single precision after a double-precision division.
fn fraction(now_ms: u32, started_ms: u32) -> f32 {
    let elapsed = u64::from(now_ms).wrapping_sub(u64::from(started_ms)) as i64;
    (elapsed as f64 * INV_FADE_MS) as f32
}

/// The start time that makes an interrupted fade continue at the mirrored
/// point: `now − round((1 − t)·3000)`, the product in single precision.
fn mirrored_start(now_ms: u32, started_ms: u32) -> u32 {
    let fraction = fraction(now_ms, started_ms);
    let remaining = ((1.0f32 - fraction) * FADE_MS as f32).round_ties_even() as i64;
    // The original subtracts as 64-bit values and keeps the low 32 bits, so
    // a fade already past its end (remaining negative) pushes the start
    // forward.
    (i64::from(now_ms) - remaining) as u32
}

impl AudioEnvironment {
    /// The requested track's name.
    pub fn music_name(&self) -> &str {
        &self.music.name
    }

    /// Plays a track on A now at `volume`, keeping A's voice and swapping its
    /// buffer. The name and volume are remembered first; while music is off
    /// nothing is loaded. The old buffer is released before the new one is
    /// asked for.
    pub fn music_play_now(
        &mut self,
        bank: &mut SoundBank,
        name: &str,
        volume: f32,
        load: impl FnOnce(&mut SoundBank, &str) -> u32,
    ) {
        self.music.a_volume = volume;
        self.music.name.clear();
        self.music.name.push_str(name);
        if !self.music.enabled {
            return;
        }
        if self.music.a_buffer != 0 {
            bank.release_buffer(self.music.a_buffer);
            self.music.a_buffer = 0;
        }
        let buffer = load(bank, name);
        self.music.a_buffer = buffer;
        if buffer == 0 {
            return;
        }
        if self.music.a.is_none() {
            self.music.a = self.allocate();
        }
        if let Some(id) = self.music.a
            && let Some(voice) = self.voice_mut(id)
        {
            voice.buffer = buffer;
            voice.gain = volume;
            voice.pitch = 1.0;
            voice.flags |= MUSIC_FLAGS;
            voice.offset = 0.0;
        }
    }

    /// Stops the main track: frees A's voice and forgets its buffer without
    /// releasing it (the original leaks that reference), and forgets the name
    /// when asked. The crossfader's state is left as it is.
    pub fn music_stop(&mut self, bank: &mut SoundBank, forget_name: bool) {
        if let Some(id) = self.music.a.take() {
            self.free_voice(bank, id);
        }
        self.music.a_buffer = 0;
        if forget_name {
            self.music.name.clear();
        }
    }

    /// Crossfades to a track, forcing music on. B starts at gain 0 and
    /// becomes A when the fade ends. The name is remembered as A's, the
    /// volume as B's.
    pub fn music_crossfade(
        &mut self,
        bank: &mut SoundBank,
        name: &str,
        volume: f32,
        now_ms: u32,
        load: impl FnOnce(&mut SoundBank, &str) -> u32,
    ) {
        self.music.enabled = true;
        let buffer = load(bank, name);
        self.music.b_buffer = buffer;
        self.music.b_volume = volume;
        self.music.name.clear();
        self.music.name.push_str(name);
        if buffer == 0 {
            return;
        }
        self.music.state = CROSSFADE;
        self.music.started_ms = now_ms;
        if self.music.b.is_none() {
            self.music.b = self.allocate();
        }
        if let Some(id) = self.music.b
            && let Some(voice) = self.voice_mut(id)
        {
            voice.buffer = buffer;
            voice.gain = 0.0;
            voice.pitch = 1.0;
            voice.flags |= MUSIC_FLAGS;
        }
    }

    /// Starts an overlay fade-in: B enters at a random point in the first 80 %
    /// of its length and rises while A ducks. Interrupting a fade-out mirrors
    /// its progress; during a fade-in or while held nothing happens.
    pub fn music_overlay(
        &mut self,
        bank: &mut SoundBank,
        name: &str,
        volume: f32,
        now_ms: u32,
        load: impl FnOnce(&mut SoundBank, &str) -> u32,
    ) {
        if self.music.state == OVERLAY_OUT {
            self.music.state = OVERLAY_IN;
            self.music.started_ms = mirrored_start(now_ms, self.music.started_ms);
            return;
        }
        if self.music.state & OVERLAY_IN != 0 {
            return;
        }
        let buffer = load(bank, name);
        self.music.b_buffer = buffer;
        self.music.b_volume = volume;
        if buffer == 0 {
            return;
        }
        self.music.state = OVERLAY_IN;
        self.music.started_ms = now_ms;
        let length = bank.buffer_secs(buffer);
        if self.music.b.is_none() {
            self.music.b = self.allocate();
        }
        // Length × 0.8 × a unit draw, in double precision.
        let offset = (f64::from(length) * 0.8 * f64::from(bank.random_unit())) as f32;
        if let Some(id) = self.music.b
            && let Some(voice) = self.voice_mut(id)
        {
            voice.buffer = buffer;
            voice.offset = offset;
            voice.gain = 0.0;
            voice.pitch = 1.0;
            voice.flags |= MUSIC_FLAGS;
        }
    }

    /// Starts the overlay's fade-out: A resumes, B falls as `(1 − t)²`. From
    /// a fade-in the progress is mirrored; a second call during the fade-out
    /// mirrors it again.
    pub fn music_overlay_fade_out(&mut self, now_ms: u32) {
        if self.music.state == OVERLAY_HELD {
            self.music.state = OVERLAY_OUT;
            self.music.started_ms = now_ms;
            if let Some(id) = self.music.a
                && let Some(voice) = self.voice_mut(id)
            {
                voice.flags = (voice.flags & !voice_flags::PAUSED) | voice_flags::RESTART;
            }
        } else if self.music.state & OVERLAY_IN != 0 {
            self.music.state = OVERLAY_OUT;
            self.music.started_ms = mirrored_start(now_ms, self.music.started_ms);
        }
    }

    /// Sets A's start offset in seconds.
    pub fn music_set_start_offset(&mut self, offset: f32) {
        if let Some(id) = self.music.a
            && let Some(voice) = self.voice_mut(id)
        {
            voice.offset = offset;
        }
    }

    /// The "Music" setting. Turning it off stops A (keeping its name);
    /// turning it on plays the remembered track by name at the remembered
    /// volume. Either acts only when that volume is above 0.01.
    pub fn music_set_enabled(
        &mut self,
        bank: &mut SoundBank,
        enabled: bool,
        load: impl FnOnce(&mut SoundBank, &str) -> u32,
    ) {
        if enabled == self.music.enabled {
            return;
        }
        self.music.enabled = enabled;
        if MIN_VOLUME >= self.music.a_volume {
            return;
        }
        if enabled {
            let name = self.music.name.clone();
            let volume = self.music.a_volume;
            self.music_play_now(bank, &name, volume, load);
        } else {
            self.music_stop(bank, false);
        }
    }

    /// The crossfader's per-frame step.
    pub(crate) fn advance_music(&mut self, bank: &mut SoundBank, now_ms: u32) {
        let a_volume = self.music.a_volume;
        let b_volume = self.music.b_volume;
        let mut a = self.music.a;
        let mut b = self.music.b;
        let mut a_buffer = self.music.a_buffer;
        let mut b_buffer = self.music.b_buffer;
        let mut state = self.music.state;
        let started = self.music.started_ms;
        match state {
            CROSSFADE => {
                let t = fraction(now_ms, started);
                if u64::from(now_ms) < u64::from(started) + u64::from(FADE_MS) {
                    if let Some(id) = b
                        && let Some(voice) = self.voice_mut(id)
                    {
                        voice.gain = t * b_volume;
                    }
                    if let Some(id) = a
                        && let Some(voice) = self.voice_mut(id)
                    {
                        let inv = 1.0 - t;
                        voice.gain = inv * inv * a_volume;
                    }
                } else {
                    // The outgoing track is stopped and freed only if it was
                    // bound to a source; its buffer is then released.
                    if let Some(id) = a {
                        let bound = self.voice(id).is_some_and(|voice| voice.source >= 0);
                        if bound {
                            self.stop_bound_source(id);
                            self.free_voice(bank, id);
                        }
                    }
                    if a_buffer != 0 {
                        bank.release_buffer(a_buffer);
                    }
                    state = IDLE;
                    if let Some(id) = b
                        && let Some(voice) = self.voice_mut(id)
                    {
                        voice.gain = b_volume;
                    }
                    a = b;
                    // The original writes the promoted volume to a global
                    // nothing reads, so A's nominal volume keeps the outgoing
                    // track's value. Reproduced.
                    a_buffer = b_buffer;
                    b = None;
                    b_buffer = 0;
                }
            }
            OVERLAY_IN => {
                // At the crossing t is 1 and A is paused.
                let finished = u64::from(now_ms) >= u64::from(started) + u64::from(FADE_MS);
                let t = if finished {
                    1.0
                } else {
                    fraction(now_ms, started)
                };
                if finished {
                    state = OVERLAY_HELD;
                    if let Some(id) = a
                        && let Some(voice) = self.voice(id)
                        && voice.source >= 0
                    {
                        self.commands.push(SourceCommand::Pause {
                            slot: voice.source as u8,
                        });
                    }
                }
                if let Some(id) = b
                    && let Some(voice) = self.voice_mut(id)
                {
                    voice.gain = t * b_volume;
                }
                if let Some(id) = a
                    && let Some(voice) = self.voice_mut(id)
                {
                    let inv = 1.0 - t;
                    voice.gain = inv * inv * a_volume;
                }
            }
            OVERLAY_OUT => {
                if u64::from(now_ms) < u64::from(started) + u64::from(FADE_MS) {
                    let t = fraction(now_ms, started);
                    if let Some(id) = b
                        && let Some(voice) = self.voice_mut(id)
                    {
                        let inv = 1.0 - t;
                        voice.gain = inv * inv * b_volume;
                    }
                    if let Some(id) = a
                        && let Some(voice) = self.voice_mut(id)
                    {
                        voice.gain = t * a_volume;
                    }
                } else {
                    state = IDLE;
                    if let Some(id) = b.take() {
                        self.free_voice(bank, id);
                    }
                    if let Some(id) = a
                        && let Some(voice) = self.voice_mut(id)
                    {
                        voice.gain = a_volume;
                    }
                }
            }
            _ => {}
        }
        self.music.state = state;
        self.music.started_ms = started;
        self.music.a = a;
        self.music.b = b;
        self.music.a_buffer = a_buffer;
        self.music.b_buffer = b_buffer;
    }
}

/// Plays the level's track when the level sets it. A level change goes
/// through the original's transition screen, which stops A and forgets its
/// name, after which the level plays its track at once (no crossfade).
pub(crate) fn start_music(
    mut env: ResMut<AudioEnvironment>,
    music: Option<Res<LevelMusic>>,
    (volume, start): (Option<Res<LevelMusicVolume>>, Option<Res<LevelMusicStart>>),
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut pending: Local<bool>,
) {
    let Some(music) = music else {
        return;
    };
    if music.is_changed() {
        *pending = true;
    }
    let Some(game) = game else {
        return;
    };
    if !*pending {
        return;
    }
    *pending = false;
    env.music_stop(&mut bank, true);
    if music.0.is_empty() {
        return;
    }
    let volume = volume.map_or(MUSIC_VOLUME, |volume| volume.0);
    env.music_play_now(&mut bank, &music.0, volume, |bank, name| {
        bank.load_buffer(&game, &mut sources, name).unwrap_or(0)
    });
    if let Some(start) = start {
        env.music_set_start_offset(start.0);
    }
}

/// Applies the music setting the options screen stores when it changes. Not
/// wired to a screen yet; public so one can drive it.
pub(crate) fn apply_music_setting(
    mut env: ResMut<AudioEnvironment>,
    mut bank: ResMut<SoundBank>,
    setting: Option<Res<MusicSetting>>,
    game: Option<Res<Game>>,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    let (Some(setting), Some(game)) = (setting, game) else {
        return;
    };
    if !setting.is_changed() {
        return;
    }
    env.music_set_enabled(&mut bank, setting.enabled, |bank, name| {
        bank.load_buffer(&game, &mut sources, name).unwrap_or(0)
    });
}

/// The "Music" setting the options screen would write.
#[derive(Resource, Clone, Copy, Debug)]
pub struct MusicSetting {
    pub enabled: bool,
}

impl Default for MusicSetting {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::voice_flags;

    /// A loader that hands out `buffer` whatever the name.
    fn track(buffer: u32) -> impl FnOnce(&mut SoundBank, &str) -> u32 {
        move |_, _| buffer
    }

    fn env_with_track(length_secs: f32) -> (AudioEnvironment, SoundBank, u32) {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let buffer = bank.insert_test_buffer(length_secs, (length_secs * 1000.0) as i32, false);
        env.music_play_now(&mut bank, "msc_test", 1.0, track(buffer));
        // One update binds A to a source, as a running frame would.
        env.update(&mut bank, &crate::environment::UpdateInput::default());
        (env, bank, buffer)
    }

    #[test]
    fn levels_pick_their_music() {
        assert_eq!(music_for_level("ArenaHub"), "msc_hub01");
        assert_eq!(music_for_level("arenao1"), "msc_arenao");
        assert_eq!(music_for_level("arena"), "msc_arena3");
        assert_eq!(music_for_level("exanima01"), "msc_uw01amb");
        assert_eq!(music_for_level("exanimac"), "msc_cryptamb");
        assert_eq!(music_for_level("exanima02"), "msc_uw01ambb");
        assert_eq!(music_for_level("charview"), "ambntra01");
    }

    /// A crossfade starts B silent and ramps it linearly: at half the fade B
    /// is at half its volume and A at a quarter of its (ducked as (1−t)²).
    /// At the end B becomes A at full volume and the old A is freed.
    #[test]
    fn a_crossfade_promotes_the_new_track() {
        let (mut env, mut bank, old) = env_with_track(4.0);
        let new = bank.insert_test_buffer(4.0, 4000, false);
        env.music_crossfade(&mut bank, "other", 0.5, 0, track(new));
        let b = env.music.b.expect("a second voice");
        assert_eq!(env.voice(b).unwrap().gain, 0.0);
        let a = env.music.a.expect("the old voice");
        env.advance_music(&mut bank, 1500);
        assert!((env.voice(b).unwrap().gain - 0.25).abs() < 1e-6);
        assert!((env.voice(a).unwrap().gain - 0.25).abs() < 1e-6);
        env.advance_music(&mut bank, 3000);
        assert_eq!(env.music.state, IDLE);
        assert_eq!(env.music.a, Some(b));
        assert!(env.music.b.is_none());
        assert_eq!(env.music.a_buffer, new);
        assert_eq!(env.music.b_buffer, 0);
        assert!((env.voice(b).unwrap().gain - 0.5).abs() < 1e-6);
        assert_eq!(env.voice(a).unwrap().flags, 0);
        // The old buffer reference was released; the new one is held.
        assert!(bank.buffer(old).is_none());
        assert!(bank.buffer(new).is_some());
        // A keeps the outgoing volume in the original; the next fade uses it.
        assert_eq!(env.music.a_volume, 1.0);
        assert_eq!(env.music.name, "other");
    }

    /// An outgoing track that has no source when the crossfade ends is not
    /// freed: its voice stays allocated, as in the original.
    #[test]
    fn an_unbound_outgoing_track_is_left_allocated() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let old = bank.insert_test_buffer(4.0, 4000, false);
        env.music_play_now(&mut bank, "msc_test", 1.0, track(old));
        let a = env.music.a.expect("a voice");
        let new = bank.insert_test_buffer(4.0, 4000, false);
        env.music_crossfade(&mut bank, "other", 0.5, 0, track(new));
        env.advance_music(&mut bank, 3000);
        assert_ne!(env.voice(a).unwrap().flags, 0);
        assert_ne!(env.music.a, Some(a));
    }

    /// The overlay fade-in starts B from a random point in its first 80 %,
    /// ducks A as (1−t)², and ends with A paused and B full.
    #[test]
    fn an_overlay_fades_in_and_pauses_the_main_track() {
        let (mut env, mut bank, _) = env_with_track(4.0);
        let overlay = bank.insert_test_buffer(4.0, 4000, false);
        // A fresh generator's first unit draw, scaled in double precision.
        let mut probe = crate::bank::Lcg::default();
        let draw = probe.unit_f32();
        bank.rng().0 = 0;
        env.music_overlay(&mut bank, "msc_sprtrealm", 0.5, 1000, track(overlay));
        let b = env.music.b.expect("an overlay voice");
        assert_eq!(
            env.voice(b).unwrap().offset,
            (4.0f64 * 0.8 * f64::from(draw)) as f32
        );
        assert_eq!(env.music.state, OVERLAY_IN);
        env.advance_music(&mut bank, 2500);
        assert!((env.voice(b).unwrap().gain - 0.25).abs() < 1e-6);
        assert!((env.voice(env.music.a.unwrap()).unwrap().gain - 0.25).abs() < 1e-6);
        env.advance_music(&mut bank, 4000);
        assert_eq!(env.music.state, OVERLAY_HELD);
        assert!((env.voice(b).unwrap().gain - 0.5).abs() < 1e-6);
        assert_eq!(env.voice(env.music.a.unwrap()).unwrap().gain, 0.0);
        // The crossing pauses the main track's source.
        assert!(
            env.commands()
                .iter()
                .any(|command| matches!(command, SourceCommand::Pause { .. }))
        );
        // A second overlay while held loads nothing.
        let mut asked = false;
        env.music_overlay(&mut bank, "other", 1.0, 4100, |_, _| {
            asked = true;
            0
        });
        assert!(!asked);
    }

    /// A fade-out resumes A and falls B as (1−t)²; at the end B is freed and
    /// A back at full volume. A's resume is a restart request, which the
    /// backend turns into a resume of the paused source.
    #[test]
    fn an_overlay_fades_out_and_frees_the_second_track() {
        let (mut env, mut bank, _) = env_with_track(4.0);
        let overlay = bank.insert_test_buffer(4.0, 4000, false);
        env.music_overlay(&mut bank, "msc_sprtrealm", 1.0, 0, track(overlay));
        env.advance_music(&mut bank, 3000);
        let b = env.music.b.expect("an overlay voice");
        assert_eq!(env.music.state, OVERLAY_HELD);
        env.music_overlay_fade_out(5000);
        assert_eq!(env.music.state, OVERLAY_OUT);
        let a = env.music.a.expect("the main voice");
        assert_eq!(
            env.voice(a).unwrap().flags & voice_flags::RESTART,
            voice_flags::RESTART
        );
        env.advance_music(&mut bank, 6500);
        assert!((env.voice(b).unwrap().gain - 0.25).abs() < 1e-6);
        assert!((env.voice(a).unwrap().gain - 0.5).abs() < 1e-6);
        env.advance_music(&mut bank, 8000);
        assert_eq!(env.music.state, IDLE);
        assert!(env.music.b.is_none());
        assert_eq!(env.voice(b).unwrap().flags, 0);
        assert_eq!(env.voice(a).unwrap().gain, 1.0);
        // B's buffer is kept, neither released nor forgotten.
        assert_eq!(env.music.b_buffer, overlay);
    }

    /// Calling a fade-in during a fade-out mirrors the progress: 500 ms into
    /// a 3000 ms fade-out, the fade-in starts as if 2500 ms had passed.
    #[test]
    fn an_interrupted_fade_mirrors_its_progress() {
        assert_eq!(mirrored_start(2000, 1000), 0);
        // now 2500, 1500 ms in: remaining 1500, start = 2500 − 1500 = 1000.
        assert_eq!(mirrored_start(2500, 1000), 1000);
        // 500 ms in: remaining 2500, start = 1500 − 2500 wraps.
        assert_eq!(mirrored_start(1500, 1000), 1500u32.wrapping_sub(2500));
        // Past the end the remaining time is negative and the start moves
        // ahead of now.
        assert_eq!(mirrored_start(5000, 1000), 6000);
        // A second fade-out call during a fade-out mirrors again.
        let mut env = AudioEnvironment::new();
        env.music.state = OVERLAY_OUT;
        env.music.started_ms = 1000;
        env.music_overlay_fade_out(2000);
        assert_eq!(env.music.started_ms, 0);
    }

    /// Music off stops A, forgets its buffer without releasing it and keeps
    /// the name; on plays the remembered name again at the remembered
    /// volume. A volume of 0.01 is not above the threshold.
    #[test]
    fn music_switches_off_and_on() {
        let (mut env, mut bank, buffer) = env_with_track(4.0);
        let a = env.music.a.expect("a voice");
        env.music_set_enabled(&mut bank, false, track(0));
        assert_eq!(env.voice(a).unwrap().flags, 0);
        assert_eq!(env.music.name, "msc_test");
        assert_eq!(env.music.a_buffer, 0);
        // The reference was leaked, not released.
        assert!(bank.buffer(buffer).is_some());
        let mut asked = String::new();
        env.music_set_enabled(&mut bank, true, |_, name: &str| {
            asked = name.to_owned();
            buffer
        });
        assert_eq!(asked, "msc_test");
        let again = env.music.a.expect("a voice");
        assert_eq!(env.voice(again).unwrap().buffer, buffer);
        assert_eq!(env.voice(again).unwrap().gain, 1.0);
        // A quiet remembered track is left alone.
        let mut quiet = AudioEnvironment::new();
        let mut quiet_bank = SoundBank::default();
        let buffer = quiet_bank.insert_test_buffer(1.0, 1000, false);
        quiet.music_play_now(&mut quiet_bank, "quiet", 0.01, track(buffer));
        quiet.music_set_enabled(&mut quiet_bank, false, track(0));
        assert!(quiet.music.a.is_some());
    }

    /// Stopping A leaves a running crossfade alone, so B still takes over
    /// even with music switched off.
    #[test]
    fn stopping_leaves_the_crossfade_running() {
        let (mut env, mut bank, _) = env_with_track(4.0);
        let new = bank.insert_test_buffer(4.0, 4000, false);
        env.music_crossfade(&mut bank, "other", 0.5, 0, track(new));
        let b = env.music.b.expect("a second voice");
        env.music_set_enabled(&mut bank, false, track(0));
        assert_eq!(env.music.state, CROSSFADE);
        env.advance_music(&mut bank, 3000);
        assert_eq!(env.music.a, Some(b));
        assert_eq!(env.music.a_buffer, new);
    }

    /// Starting a track while music is off only remembers the name and
    /// volume and loads nothing; a crossfade forces music on.
    #[test]
    fn a_track_is_only_loaded_while_music_is_on() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let buffer = bank.insert_test_buffer(1.0, 1000, false);
        env.music.enabled = false;
        let mut asked = false;
        env.music_play_now(&mut bank, "quiet", 0.3, |_, _| {
            asked = true;
            buffer
        });
        assert!(!asked);
        assert_eq!(env.music.name, "quiet");
        assert_eq!(env.music.a_volume, 0.3);
        assert_eq!(env.music.a_buffer, 0);
        assert!(env.music.a.is_none());
        env.music_crossfade(&mut bank, "loud", 0.2, 0, track(buffer));
        assert!(env.music.enabled);
        assert_eq!(env.music.state, CROSSFADE);
    }

    /// Starting a track now reuses A's voice, releases the old buffer, swaps
    /// in the new one and restarts it at its volume.
    #[test]
    fn playing_now_swaps_the_track_on_the_same_voice() {
        let (mut env, mut bank, old) = env_with_track(4.0);
        let a = env.music.a.expect("a voice");
        let new = bank.insert_test_buffer(2.0, 2000, false);
        env.music_play_now(&mut bank, "new", 0.3, track(new));
        assert_eq!(env.music.a, Some(a));
        assert_eq!(env.music.a_buffer, new);
        let voice = env.voice(a).unwrap();
        assert_eq!(voice.buffer, new);
        assert_eq!(voice.gain, 0.3);
        assert_eq!(voice.offset, 0.0);
        assert_ne!(voice.flags & voice_flags::RESTART, 0);
        assert!(bank.buffer(old).is_none());
        assert_eq!(env.music.name, "new");
    }

    /// The start offset is A's, until it is consumed.
    #[test]
    fn the_start_offset_is_set_on_the_main_track() {
        let (mut env, _, _) = env_with_track(4.0);
        env.music_set_start_offset(41.0);
        let a = env.music.a.expect("a voice");
        assert_eq!(env.voice(a).unwrap().offset, 41.0);
    }

    /// Byte 8 picks most levels' tracks, volumes and presets; byte 11 `w`
    /// and `67` come first, byte 7 `c` last.
    #[test]
    fn story_levels_pick_track_volume_and_ambience() {
        let four = story_music(b"exanima04");
        assert_eq!(
            (four.track, four.ambience),
            ("Msc_UWB2_Amb", Some("AmbUWBA"))
        );
        assert_eq!(
            (four.volume.to_bits(), four.preset.to_bits()),
            (0x3e19_999a, 0x3fb3_3333)
        );
        let six = story_music(b"exanima06");
        assert_eq!(
            (six.track, six.volume, six.preset),
            ("Msc_UWB5_Amb", 0.25, 2.0)
        );
        assert_eq!(story_music(b"exanima05").track, "Msc_UWB4_Amb");
        assert_eq!(story_music(b"exanima07").track, "Msc_CountfourA");
        assert_eq!(story_music(b"exanima08").key, "msc_uwb6_amb");
        assert_eq!(story_music(b"exanima67").track, "Msc_GardensA");
        assert_eq!(story_music(b"exanima01__w").track, "Msc_SharpNatural");
        let crypt = story_music(b"exanimac1");
        assert_eq!(
            (crypt.track, crypt.preset.to_bits()),
            ("Msc_CryptAmb", 0x3fe6_6666)
        );
        let first = story_music(b"exanima01");
        assert_eq!(
            (first.track, first.volume.to_bits()),
            ("Msc_UW01Amb", 0x3dcc_cccd)
        );
        assert!(story_music_changes(b"exanima07", "Msc_UW01Amb"));
        assert!(!story_music_changes(b"exanima07", "Msc_Countfour"));
        assert!(!story_music_changes(b"exanima01__w", "Msc_SharpNaturalX"));
    }
}
