//! Area ambience: a looping bed (`Env<area>01`) at 15 % of the area's
//! volume, plus a numbered set of random one-shots (`Amb<area>01` … `16`).
//!
//! The bed is an interface-bus voice, listener-relative, unattenuated and
//! looping, started at a random point in its first 0.8 s. The one-shots are
//! scheduled on the game clock: 15–45 s after an area is set, then 5–15 s
//! after a mono one and 15–45 s after a stereo one. A mono one is placed
//! 10–20 m from the listener anchor in a random horizontal direction, at
//! gain 0.02–0.05 for the first two variants and 0.03–0.10 for the rest; a
//! stereo one is listener-relative and unattenuated at gain 0.075–0.15. Both
//! vary their pitch by ±10 % around 0.9 (mono) or 0.8 (stereo).

use bevy::prelude::*;
use exa_assets::Game;

use crate::SoundBank;
use crate::environment::{AudioEnvironment, CM_PER_M, SILENCE, voice_flags};

/// The bed's gain as a fraction of the area volume.
pub const BED_GAIN: f64 = 0.15;
/// The bed starts somewhere in its first 0.8 s.
pub const BED_OFFSET_S: f64 = 0.8;
/// The longest name the original builds for a bed or a set: its names pass
/// through a 15-character short string.
pub const MAX_NAME_LEN: usize = 15;
/// The shortest delay before a mono one-shot after another one, ms.
pub const MONO_DELAY_MS: u32 = 5000;
/// The shortest delay before any other one-shot, ms.
pub const LONG_DELAY_MS: u32 = 15_000;
/// The spread of the first delay and the stereo delay, ms.
pub const DELAY_SPREAD_MS: u32 = 30_000;
/// The spread of a mono one-shot's delay, ms.
pub const MONO_SPREAD_MS: u32 = 10_000;
/// Distance from the listener anchor, in cm: 10 m to 20 m.
pub const ONE_SHOT_DISTANCE_MIN_CM: f32 = 1000.0;
/// Extra distance the draw can add, in cm.
pub const ONE_SHOT_DISTANCE_SPREAD_CM: f32 = 1000.0;

/// A numbered set's prefix for an area (`Amb<area>`), cut at 15 characters.
pub fn set_prefix(area: &str) -> String {
    short_string(format!("Amb{area}"))
}

/// The bed's file name for an area (`Env<area>01`), cut at 15 characters.
pub fn bed_name(area: &str) -> String {
    short_string(format!("Env{area}01"))
}

/// Cuts a name to the 15 characters a short string holds.
fn short_string(mut name: String) -> String {
    if name.len() > MAX_NAME_LEN {
        // The original's short string holds 15 bytes; a Rust string can only
        // be cut at a character boundary.
        let mut cut = MAX_NAME_LEN;
        while cut > 0 && !name.is_char_boundary(cut) {
            cut -= 1;
        }
        name.truncate(cut);
    }
    name
}

/// The ambience a level starts with. An outdoor arena gets the nature bed
/// `NtrA` at full volume when it is neither raining nor windy (the weather
/// and time-of-day callers are not in Exarust yet, so this assumes a still,
/// dry day; the night bed is the only one with a stated volume, 0.3). A
/// story level takes the area its name's bytes pick, at full volume: the
/// fourth level names `AmbUWBA`, whose bed and set a prefix then turns into
/// `EnvAmbUWBA01` and `AmbAmbUWBA`, files that do not ship — the original's
/// own naming bug, reproduced. Every other story level stops the ambience.
pub fn ambience_for_level(level: &str) -> Option<AmbienceArea> {
    let stem = level
        .strip_suffix(".rfc")
        .unwrap_or(level)
        .to_ascii_lowercase();
    if stem.starts_with("arenao") {
        Some(AmbienceArea {
            name: "NtrA".into(),
            volume: 1.0,
        })
    } else if stem.starts_with("exanima0") || stem.starts_with("exanimac") {
        crate::music::story_music(level.as_bytes())
            .ambience
            .map(|name| AmbienceArea {
                name: name.into(),
                volume: 1.0,
            })
    } else {
        None
    }
}

/// An area's ambience as the game mode chooses it.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct LevelAmbience(pub Option<AmbienceArea>);

/// An outdoor arena's ambience, as the original chooses it when the arena's
/// scene is set up: from the scene's rain, daylight and wind, and the
/// arena's own silent-day flag.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArenaAmbience {
    /// The area to play, or `None` to stop the ambience.
    pub area: Option<AmbienceArea>,
    /// The wind loop's own bed gain and pitch, which the original overrides
    /// after setting the area (they replace the bed's `0.15 × volume`).
    pub bed: Option<(f32, f32)>,
}

/// The rain amount above which the weather's own rain sound replaces the
/// ambience.
pub const RAIN_AMBIENCE_LIMIT: f64 = 0.01;
/// The daylight factor below which the outdoor arenas play the night bed.
pub const NIGHT_DAYLIGHT_LIMIT: f32 = 0.5;
/// The night bed's volume.
pub const NIGHT_VOLUME: f32 = 0.3;
/// The wind strength above which the outdoor arenas play the wind bed.
pub const WIND_LIMIT: f64 = 0.1;
/// The wind bed's gain as a fraction of the squared strength, and its pitch
/// base and rise.
pub const WIND_GAIN: f64 = 0.2;
pub const WIND_PITCH_BASE: f64 = 0.6;

/// The outdoor arenas' ambience from the scene's weather, in the original's
/// order: heavy rain stops it (the weather code plays the rain instead); a
/// daylight factor below 0.5 plays the night bed `NtrAN` at 0.3; a calm
/// arena plays the day bed `NtrA` at full volume; otherwise a wind above 0.1
/// plays `NtrW` at full volume with the bed's gain `wind² × 0.2` and pitch
/// `0.6 + 0.5 × wind`; a still day stops it. `calm` is the arena's entry in
/// the original's per-arena table; that table's index comes from the arena
/// progression, which is not ported, so nothing in the running game calls
/// this yet.
pub fn arena_ambience(daylight: f32, wind: f32, raining: f32, calm: bool) -> ArenaAmbience {
    // The original compares the single fields against double constants.
    if f64::from(raining) > RAIN_AMBIENCE_LIMIT {
        return ArenaAmbience::default();
    }
    if NIGHT_DAYLIGHT_LIMIT > daylight {
        return ArenaAmbience {
            area: Some(AmbienceArea {
                name: "NtrAN".into(),
                volume: NIGHT_VOLUME,
            }),
            bed: None,
        };
    }
    if calm {
        return ArenaAmbience {
            area: Some(AmbienceArea {
                name: "NtrA".into(),
                volume: 1.0,
            }),
            bed: None,
        };
    }
    if f64::from(wind) > WIND_LIMIT {
        let gain = ((wind * wind) as f64 * WIND_GAIN) as f32;
        let pitch = ((wind * 0.5) as f64 + WIND_PITCH_BASE) as f32;
        return ArenaAmbience {
            area: Some(AmbienceArea {
                name: "NtrW".into(),
                volume: 1.0,
            }),
            bed: Some((gain, pitch)),
        };
    }
    ArenaAmbience::default()
}

/// The area to play and the volume the game mode asks for.
#[derive(Clone, Debug, PartialEq)]
pub struct AmbienceArea {
    pub name: String,
    pub volume: f32,
}

/// The ambience state: the bed voice and buffer, the one-shot set, the area
/// volume and the next one-shot's due time on the game clock.
#[derive(Debug, Clone, Default)]
pub struct AmbienceState {
    /// Whether an area's ambience is installed, even one with no files.
    pub installed: bool,
    pub bed_voice: Option<u32>,
    pub bed_buffer: u32,
    pub set: u32,
    pub volume: f32,
    pub next_due_ms: u32,
}

impl AudioEnvironment {
    /// Sets the area's ambience from a loaded bed buffer and one-shot set.
    /// The bed voice is reused when it exists; the old bed buffer is released
    /// only when it changes, and the old set after the new one is loaded.
    pub fn apply_ambience(&mut self, bank: &mut SoundBank, bed_buffer: u32, set: u32, volume: f32) {
        self.ambience.volume = volume;
        self.ambience.installed = true;
        if self.ambience.bed_buffer != bed_buffer && self.ambience.bed_buffer != 0 {
            bank.release_buffer(self.ambience.bed_buffer);
        }
        self.ambience.bed_buffer = bed_buffer;
        if bed_buffer == 0 {
            if let Some(id) = self.ambience.bed_voice.take() {
                self.free_voice(bank, id);
            }
        } else {
            if self.ambience.bed_voice.is_none() {
                self.ambience.bed_voice = Some(self.allocate_or_dummy());
            }
            let offset = (f64::from(bank.random_unit()) * BED_OFFSET_S) as f32;
            if let Some(id) = self.ambience.bed_voice
                && let Some(voice) = self.voice_mut(id)
            {
                voice.buffer = bed_buffer;
                voice.gain = (f64::from(volume) * BED_GAIN) as f32;
                voice.pitch = 1.0;
                voice.flags |= crate::music::MUSIC_FLAGS;
                voice.offset = offset;
            }
        }
        if self.ambience.set != 0 {
            bank.release_set(self.ambience.set);
        }
        self.ambience.set = set;
    }

    /// Applies an arena's weather ambience: clears it when the choice has no
    /// area, otherwise sets the area and then gives the wind loop its own bed
    /// gain and pitch, as the arena setup overrides them after setting it.
    pub fn apply_arena_ambience(
        &mut self,
        bank: &mut SoundBank,
        choice: ArenaAmbience,
        bed_buffer: u32,
        set: u32,
    ) {
        let Some(area) = choice.area else {
            self.clear_ambience(bank);
            return;
        };
        self.apply_ambience(bank, bed_buffer, set, area.volume);
        if let Some((gain, pitch)) = choice.bed
            && let Some(id) = self.ambience.bed_voice
            && let Some(voice) = self.voice_mut(id)
        {
            voice.gain = gain;
            voice.pitch = pitch;
        }
    }

    /// Clears the area's ambience: the bed voice, its buffer and the set.
    pub fn clear_ambience(&mut self, bank: &mut SoundBank) {
        self.ambience.bed_voice = self.release_voice(bank, self.ambience.bed_voice);
        if self.ambience.bed_buffer != 0 {
            bank.release_buffer(self.ambience.bed_buffer);
            self.ambience.bed_buffer = 0;
        }
        if self.ambience.set != 0 {
            bank.release_set(self.ambience.set);
            self.ambience.set = 0;
        }
        self.ambience.installed = false;
    }

    /// The one-shot scheduler, run once an update while an area is installed
    /// and its next shot is due on the game clock.
    pub(crate) fn schedule_ambience(&mut self, bank: &mut SoundBank, game_ms: u32) {
        if self.ambience.next_due_ms >= game_ms {
            return;
        }
        let roll = bank.rng().below(DELAY_SPREAD_MS);
        self.ambience.next_due_ms = game_ms.wrapping_add(LONG_DELAY_MS).wrapping_add(roll);
        if self.ambience.set == SoundBank::EMPTY_SET || bank.set_count(self.ambience.set) == 0 {
            return;
        }
        let Some(buffer) = bank.pick_set_variant(self.ambience.set) else {
            return;
        };
        let Some(id) = self.allocate() else {
            return;
        };
        let default_flags = self.default_flags;
        if !bank.buffer_is_stereo(buffer) {
            // A random horizontal direction from the anchor.
            let (x, z) = one_shot_direction(bank.random_unit(), bank.random_unit());
            // The original draws this distance from the runtime library's
            // own random stream, not the engine generator; its generator and
            // seed are outside the evidence, so a stand-in stream serves.
            let distance_cm = self.rtl_random.unit_f64() * f64::from(ONE_SHOT_DISTANCE_SPREAD_CM);
            let distance_cm = (distance_cm + f64::from(ONE_SHOT_DISTANCE_MIN_CM)) as f32;
            // The original scales the direction in centimetres and adds the
            // anchor there; positions here are metres.
            let offset_cm = Vec3::new(x, 0.0, z) * distance_cm;
            let position = self.listener.anchor + offset_cm / CM_PER_M;
            let first_two = bank.set_last(self.ambience.set) < 2;
            // Every range is a unit draw scaled by double-precision constants.
            let gain = if first_two {
                scaled_draw(bank, 0.03, 0.02)
            } else {
                scaled_draw(bank, 0.07, 0.03)
            };
            let pitch = scaled_draw(bank, 0.2, 0.9);
            if let Some(voice) = self.voice_mut(id) {
                voice.flags = default_flags;
                voice.buffer = buffer;
                voice.position = position;
                voice.gain = gain;
                voice.pitch = pitch;
            }
            let roll = bank.rng().below(MONO_SPREAD_MS);
            self.ambience.next_due_ms = game_ms.wrapping_add(MONO_DELAY_MS).wrapping_add(roll);
        } else {
            let gain = scaled_draw(bank, 0.075, 0.075);
            let pitch = scaled_draw(bank, 0.2, 0.8);
            if let Some(voice) = self.voice_mut(id) {
                voice.flags = default_flags
                    | voice_flags::INTERFACE
                    | voice_flags::LISTENER_RELATIVE
                    | voice_flags::NO_ATTENUATION;
                voice.buffer = buffer;
                voice.position = Vec3::ZERO;
                voice.gain = gain;
                voice.pitch = pitch;
            }
            let roll = bank.rng().below(DELAY_SPREAD_MS);
            self.ambience.next_due_ms = game_ms.wrapping_add(LONG_DELAY_MS).wrapping_add(roll);
        }
    }
}

/// A unit draw from the engine generator times `spread` plus `base`, in
/// double precision and stored as a single.
fn scaled_draw(bank: &mut SoundBank, spread: f64, base: f64) -> f32 {
    (f64::from(bank.random_unit()) * spread + base) as f32
}

/// The horizontal direction of a mono one-shot from its two draws:
/// `(0.5 − x, 0.5 − z)`, normalised in single precision. When the squared
/// length is at or below the `f32` epsilon — or unordered, which the
/// original's compare treats the same — it sets only the first component to
/// 1 and keeps the second draw's value.
fn one_shot_direction(x_draw: f32, z_draw: f32) -> (f32, f32) {
    let mut x = 0.5 - x_draw;
    let mut z = 0.5 - z_draw;
    let length2 = z * z + x * x;
    if length2 > SILENCE {
        let scale = 1.0 / length2.sqrt();
        x *= scale;
        z *= scale;
    } else {
        x = 1.0;
    }
    (x, z)
}

/// Sets or clears the level's ambience when the resource changes (once the
/// game data is there to load it from).
pub(crate) fn set_level_ambience(
    mut env: ResMut<AudioEnvironment>,
    ambience: Option<Res<LevelAmbience>>,
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut pending: Local<bool>,
) {
    let Some(ambience) = ambience else {
        return;
    };
    if ambience.is_changed() {
        *pending = true;
    }
    if !*pending {
        return;
    }
    let Some(area) = &ambience.0 else {
        *pending = false;
        env.clear_ambience(&mut bank);
        return;
    };
    let Some(game) = game else {
        return;
    };
    *pending = false;
    let bed = bank
        .load_buffer(&game, &mut sources, &bed_name(&area.name))
        .unwrap_or(0);
    let set = bank.load_set(&game, &mut sources, &set_prefix(&area.name));
    env.apply_ambience(&mut bank, bed, set, area.volume);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::{UpdateInput, voice_flags};
    use crate::music::MUSIC_FLAGS;

    /// The names follow the original: `Env<area>01` and the set prefix
    /// `Amb<area>` (whose variants append 01 to 16).
    #[test]
    fn ambience_names_are_built_from_the_area() {
        assert_eq!(bed_name("NtrA"), "EnvNtrA01");
        assert_eq!(set_prefix("NtrA"), "AmbNtrA");
        assert_eq!(format!("{}01", set_prefix("NtrA")), "AmbNtrA01");
        // Both pass through a 15-character short string.
        assert_eq!(bed_name("ABCDEFGHIJKL"), "EnvABCDEFGHIJKL");
        assert_eq!(set_prefix("ABCDEFGHIJKLMN"), "AmbABCDEFGHIJKL");
    }

    /// Only the outdoor arenas get a known area; indoor levels start silent
    /// until their game mode chooses one.
    #[test]
    fn outdoor_levels_get_the_daytime_nature_bed() {
        assert_eq!(
            ambience_for_level("arenao1"),
            Some(AmbienceArea {
                name: "NtrA".into(),
                volume: 1.0
            })
        );
        assert_eq!(ambience_for_level("ArenaO2.rfc").unwrap().name, "NtrA");
        assert_eq!(ambience_for_level("arena"), None);
        assert_eq!(ambience_for_level("exanima01"), None);
        assert_eq!(ambience_for_level("charview"), None);
    }

    /// A story level takes the area its name's bytes pick at full volume: the
    /// fourth level names `AmbUWBA`, whose bed and set names the prefixes then
    /// build into files that do not ship; every other story level has none.
    #[test]
    fn the_fourth_story_level_names_its_ambience() {
        let four = ambience_for_level("exanima04").expect("the fourth level");
        assert_eq!(four.name, "AmbUWBA");
        assert_eq!(four.volume, 1.0);
        assert_eq!(bed_name(&four.name), "EnvAmbUWBA01");
        assert_eq!(set_prefix(&four.name), "AmbAmbUWBA");
        for level in [
            "exanima01",
            "exanima02",
            "exanima05",
            "exanima06",
            "exanima07",
            "exanima08",
            "exanimac1",
        ] {
            assert_eq!(ambience_for_level(level), None, "{level}");
        }
    }

    /// The bed plays at 0.15 × the area volume, pitch 1, on the interface
    /// bus, looping, at a start offset in the first 0.8 s.
    #[test]
    fn the_bed_is_a_quiet_interface_loop() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let bed = bank.insert_test_buffer(3.0, 3000, false);
        let mut probe = crate::bank::Lcg::default();
        let draw = probe.unit_f32();
        bank.rng().0 = 0;
        env.apply_ambience(&mut bank, bed, SoundBank::EMPTY_SET, 0.5);
        let id = env.ambience.bed_voice.expect("a bed voice");
        let voice = env.voice(id).unwrap();
        assert_eq!(voice.buffer, bed);
        assert_eq!(voice.gain, (f64::from(0.5f32) * 0.15) as f32);
        assert_eq!(voice.pitch, 1.0);
        assert_eq!(voice.flags, MUSIC_FLAGS | voice_flags::ALLOCATED);
        assert_eq!(voice.offset, (f64::from(draw) * 0.8) as f32);
        assert!(env.ambience.installed);
        assert_eq!(env.ambience.volume, 0.5);
    }

    /// Clearing releases the bed voice, buffer and set.
    #[test]
    fn clearing_ambience_lets_go_of_everything() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let bed = bank.insert_test_buffer(3.0, 3000, false);
        let set = bank.insert_test_set(vec![bed]);
        env.apply_ambience(&mut bank, bed, set, 1.0);
        env.clear_ambience(&mut bank);
        assert_eq!(env.ambience.bed_voice, None);
        assert_eq!(env.ambience.bed_buffer, 0);
        assert_eq!(env.ambience.set, 0);
        assert!(!env.ambience.installed);
        assert!(bank.buffer(bed).is_none());
    }

    /// A set with no files still counts as installed: the scheduler keeps
    /// moving its due time without starting a voice.
    #[test]
    fn an_empty_area_still_reschedules() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        env.apply_ambience(&mut bank, 0, SoundBank::EMPTY_SET, 1.0);
        let input = UpdateInput {
            game_ms: 1,
            ..UpdateInput::default()
        };
        let seen_before = env.ambience.next_due_ms;
        env.update(&mut bank, &input);
        assert_ne!(env.ambience.next_due_ms, seen_before);
        assert_eq!(env.ambience.next_due_ms, 1 + LONG_DELAY_MS);
        assert!(env.voice(0).is_none_or(|v| v.flags == 0));
    }

    /// A mono one-shot is placed 10–20 m from the anchor, gets one of the two
    /// quiet gains for the first variants, and reschedules 5–15 s later.
    #[test]
    fn mono_one_shots_are_placed_around_the_listener() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let bed = bank.insert_test_buffer(3.0, 3000, false);
        let one = bank.insert_test_buffer(1.0, 1000, false);
        let set = bank.insert_test_set(vec![one]);
        env.apply_ambience(&mut bank, bed, set, 1.0);
        env.listener.anchor = Vec3::new(10.0, 2.0, 0.0);
        // The generator starts from a known state for the placement draws.
        bank.rng().0 = 0;
        // The distance uses the separate stand-in stream, also from zero.
        env.rtl_random.0 = 0;
        let input = UpdateInput {
            game_ms: 1,
            ..UpdateInput::default()
        };
        env.update(&mut bank, &input);
        let voice = (0..crate::environment::VOICE_CAPACITY as u32)
            .filter_map(|id| env.voice(id))
            .find(|voice| voice.buffer == one)
            .expect("a one-shot voice");
        // Binding consumed the restart request and marked it active.
        assert_eq!(
            voice.flags,
            crate::environment::DEFAULT_ONE_SHOT_FLAGS & !voice_flags::RESTART
                | voice_flags::ACTIVE
        );
        // The first unit draw is 2^-32, so x ≈ 0.5, and z from the next
        // draw; the unit vector runs mostly along z.
        let offset = voice.position - Vec3::new(10.0, 2.0, 0.0);
        assert!((offset.length() - 10.0).abs() < 0.01, "{offset:?}");
        assert!(voice.gain >= 0.02 && voice.gain <= 0.05);
        assert!(voice.pitch >= 0.9 && voice.pitch <= 1.1);
        assert!(env.ambience.next_due_ms > MONO_DELAY_MS);
        assert!(env.ambience.next_due_ms <= MONO_DELAY_MS + MONO_SPREAD_MS + 1);
    }

    /// A stereo one-shot is listener-relative and unattenuated, at
    /// 0.075–0.15 gain and 0.8–1.0 pitch.
    #[test]
    fn stereo_one_shots_stay_on_the_listener() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let bed = bank.insert_test_buffer(3.0, 3000, false);
        let one = bank.insert_test_buffer(1.0, 1000, true);
        let set = bank.insert_test_set(vec![one]);
        env.apply_ambience(&mut bank, bed, set, 1.0);
        bank.rng().0 = 0;
        let input = UpdateInput {
            game_ms: 1,
            ..UpdateInput::default()
        };
        env.update(&mut bank, &input);
        let voice = (0..crate::environment::VOICE_CAPACITY as u32)
            .filter_map(|id| env.voice(id))
            .find(|voice| voice.buffer == one)
            .expect("a one-shot voice");
        assert_eq!(voice.position, Vec3::ZERO);
        assert_eq!(
            voice.flags
                & (voice_flags::INTERFACE
                    | voice_flags::LISTENER_RELATIVE
                    | voice_flags::NO_ATTENUATION),
            voice_flags::INTERFACE | voice_flags::LISTENER_RELATIVE | voice_flags::NO_ATTENUATION
        );
        assert!(voice.gain >= 0.075 && voice.gain <= 0.15);
        assert!(voice.pitch >= 0.8 && voice.pitch <= 1.0);
        assert!(env.ambience.next_due_ms > LONG_DELAY_MS);
    }

    /// The formats the buffer reports decide the one-shot kind.
    #[test]
    fn a_stereo_file_is_the_stereo_kind() {
        let mut bank = SoundBank::default();
        let mono = bank.insert_test_buffer(1.0, 1000, false);
        let stereo = bank.insert_test_buffer(1.0, 1000, true);
        assert!(!bank.buffer_is_stereo(mono));
        assert!(bank.buffer_is_stereo(stereo));
    }

    /// The direction is the two draws taken off 0.5, normalised in single
    /// precision. A zero vector (both draws exactly 0.5) falls back to the
    /// original's `x = 1` and keeps z; an unordered draw does the same.
    #[test]
    fn a_mono_direction_is_normalised_or_falls_back_to_x() {
        let (x, z) = one_shot_direction(0.0, 0.0);
        let unit = 1.0f32 / 2.0f32.sqrt();
        assert!((x - unit).abs() < 1e-7 && (z - unit).abs() < 1e-7);
        assert_eq!(one_shot_direction(0.5, 0.5), (1.0, 0.0));
        let (x, z) = one_shot_direction(f32::NAN, 0.25);
        assert_eq!(x, 1.0);
        assert_eq!(z, 0.25);
        // The squared length and its sum keep the original's order.
        let (x, z) = one_shot_direction(0.25, 0.75);
        assert!((x * x + z * z - 1.0).abs() < 1e-6);
        assert_eq!(short_string("abcdefghijklmnop".into()), "abcdefghijklmno");
        assert_eq!(short_string("éééééééé".into()), "ééééééé");
    }

    /// A quiet or unordered remembered volume is left alone by the music
    /// setting, as the original's unordered-safe compare does.
    #[test]
    fn a_quiet_or_unordered_volume_ignores_the_music_setting() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let buffer = bank.insert_test_buffer(1.0, 1000, false);
        env.music_play_now(&mut bank, "quiet", f32::NAN, |_, _| buffer);
        env.update(&mut bank, &crate::environment::UpdateInput::default());
        let a = env.music.a.expect("a voice");
        env.music_set_enabled(&mut bank, false, |_, _| 0);
        assert_ne!(env.voice(a).unwrap().flags, 0);
        assert!(!env.music.enabled);
    }

    /// The outdoor arena's choice, in the original's order: rain stops it,
    /// a dark scene plays the night bed at 0.3, a calm arena the day bed,
    /// a wind above 0.1 the wind bed at `wind² × 0.2` gain and
    /// `0.6 + 0.5 × wind` pitch, and a still day nothing.
    #[test]
    fn arena_ambience_follows_rain_daylight_and_wind() {
        let rainy = arena_ambience(1.0, 0.0, 0.02, false);
        assert_eq!(rainy, ArenaAmbience::default());
        let night = arena_ambience(0.4, 0.0, 0.0, false);
        assert_eq!(night.area.as_ref().map(|a| a.name.as_str()), Some("NtrAN"));
        assert_eq!(night.area.as_ref().map(|a| a.volume), Some(0.3));
        assert_eq!(night.bed, None);
        let calm = arena_ambience(0.9, 0.0, 0.0, true);
        assert_eq!(calm.area.as_ref().map(|a| a.name.as_str()), Some("NtrA"));
        assert_eq!(calm.area.as_ref().map(|a| a.volume), Some(1.0));
        let windy = arena_ambience(0.9, 0.5, 0.0, false);
        assert_eq!(windy.area.as_ref().map(|a| a.name.as_str()), Some("NtrW"));
        let (gain, pitch) = windy.bed.expect("the wind bed");
        assert!((gain - 0.05).abs() < 1e-7, "{gain}");
        assert!((pitch - 0.85).abs() < 1e-7, "{pitch}");
        assert_eq!(
            arena_ambience(0.9, 0.099, 0.0, false),
            ArenaAmbience::default()
        );
        // The single 0.1 is a hair above the double threshold 0.1, so it
        // takes the wind branch, as the original's single-to-double compare
        // does.
        assert_eq!(
            arena_ambience(0.9, 0.1, 0.0, false)
                .area
                .as_ref()
                .map(|a| a.name.as_str()),
            Some("NtrW")
        );
        // Unordered values take the not-greater branch: no rain, no night,
        // no wind.
        assert_eq!(
            arena_ambience(f32::NAN, f32::NAN, f32::NAN, false),
            ArenaAmbience::default()
        );
    }

    /// The wind choice's bed overrides the area's 0.15 × volume gain and
    /// pitch 1; a night choice keeps the plain bed.
    #[test]
    fn the_wind_bed_takes_its_own_gain_and_pitch() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let bed = bank.insert_test_buffer(3.0, 3000, false);
        let choice = arena_ambience(0.9, 0.5, 0.0, false);
        env.apply_arena_ambience(&mut bank, choice, bed, SoundBank::EMPTY_SET);
        let id = env.ambience.bed_voice.expect("a bed voice");
        assert!((env.voice(id).unwrap().gain - 0.05).abs() < 1e-7);
        assert!((env.voice(id).unwrap().pitch - 0.85).abs() < 1e-7);
        let night = arena_ambience(0.4, 0.0, 0.0, false);
        env.apply_arena_ambience(&mut bank, night, bed, SoundBank::EMPTY_SET);
        assert!((env.voice(id).unwrap().gain - 0.045).abs() < 1e-7);
        assert_eq!(env.voice(id).unwrap().pitch, 1.0);
        // A choice without an area clears the ambience.
        env.apply_arena_ambience(&mut bank, ArenaAmbience::default(), 0, 0);
        assert!(!env.ambience.installed);
        assert_eq!(env.ambience.bed_voice, None);
    }
}
