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
pub const BED_GAIN: f32 = 0.15;
/// The bed starts somewhere in its first 0.8 s.
pub const BED_OFFSET_S: f32 = 0.8;
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

/// A numbered set's prefix for an area (`Amb<area>`).
pub fn set_prefix(area: &str) -> String {
    format!("Amb{area}")
}

/// The bed's file name for an area (`Env<area>01`).
pub fn bed_name(area: &str) -> String {
    format!("Env{area}01")
}

/// The ambience a level starts with. Only the outdoor arenas' daylight case
/// is known: the nature bed `NtrA` when it is neither raining nor windy (the
/// weather and time-of-day callers are not in Exarust yet, so this assumes a
/// still, dry day). Guess: the bed at full volume; the night bed is the only
/// one with a stated volume, 0.3.
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
    } else {
        None
    }
}

/// An area's ambience as the game mode chooses it.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct LevelAmbience(pub Option<AmbienceArea>);

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
                self.ambience.bed_voice = self.allocate();
            }
            let offset = bank.random_unit() * BED_OFFSET_S;
            if let Some(id) = self.ambience.bed_voice
                && let Some(voice) = self.voice_mut(id)
            {
                voice.buffer = bed_buffer;
                voice.gain = volume * BED_GAIN;
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

    /// Clears the area's ambience: the bed voice, its buffer and the set.
    pub fn clear_ambience(&mut self, bank: &mut SoundBank) {
        if let Some(id) = self.ambience.bed_voice.take() {
            self.free_voice(bank, id);
        }
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
            let mut x = 0.5 - bank.random_unit();
            let mut z = 0.5 - bank.random_unit();
            let length2 = z * z + x * x;
            if length2 <= SILENCE {
                x = 1.0;
            } else {
                let scale = 1.0 / length2.sqrt();
                x *= scale;
                z *= scale;
            }
            // The original draws this distance from the RTL `Random` stream,
            // not the engine generator. Its seed is process-global and not
            // observable here; the stream starts at zero as a guess.
            let distance_cm = self.fpc_rng.unit_f64() * f64::from(ONE_SHOT_DISTANCE_SPREAD_CM);
            let distance_cm = (distance_cm + f64::from(ONE_SHOT_DISTANCE_MIN_CM)) as f32;
            let position = self.listener.anchor + Vec3::new(x, 0.0, z) * (distance_cm / CM_PER_M);
            let first_two = bank.set_last(self.ambience.set) < 2;
            let gain = if first_two {
                bank.random_unit() * 0.03 + 0.02
            } else {
                bank.random_unit() * 0.07 + 0.03
            };
            let pitch = bank.random_unit() * 0.2 + 0.9;
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
            let gain = bank.random_unit() * 0.075 + 0.075;
            let pitch = bank.random_unit() * 0.2 + 0.8;
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

/// Sets or clears the level's ambience when the resource changes.
pub(crate) fn set_level_ambience(
    mut env: ResMut<AudioEnvironment>,
    ambience: Option<Res<LevelAmbience>>,
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    let Some(ambience) = ambience else {
        return;
    };
    if !ambience.is_changed() {
        return;
    }
    let Some(area) = &ambience.0 else {
        env.clear_ambience(&mut bank);
        return;
    };
    let Some(game) = game else {
        return;
    };
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
        assert_eq!(voice.gain, 0.075);
        assert_eq!(voice.pitch, 1.0);
        assert_eq!(voice.flags, MUSIC_FLAGS | voice_flags::ALLOCATED);
        assert!((voice.offset - 0.8 * draw).abs() < 1e-6);
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
        // Distance uses the separate FPC stream, also from zero.
        env.fpc_rng.0 = 0;
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
}
