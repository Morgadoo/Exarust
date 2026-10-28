//! Audio from `Sound.rpk` (RIFF/WAVE PCM): the name-keyed buffer cache and
//! sound sets (`bank`), the original's virtual-voice mixer with 512 voices
//! ranked onto 64 sources (`environment`), music crossfades (`music`), area
//! ambience (`ambience`), the global EFX reverb's parameters (`reverb`),
//! weapon impacts and pain sounds on hits (`hits`), and footsteps for moving
//! characters (`footsteps`).
//!
//! The original mixes through OpenAL with EFX; Bevy's rodio backend has
//! neither EFX nor per-source auxiliary sends. The mixer computes the reverb
//! block and the send flag of every voice but the backend does not apply them.
//! The OpenAL distance model the original sets up (linear, clamped between
//! 4 m and 30 m) is applied by the playback layer (see the crate README).

mod ambience;
mod bank;
mod environment;
mod footsteps;
mod hits;
mod loops;
mod music;
mod playback;
mod reverb;
mod sounds;
pub mod voices;

use bevy::prelude::*;

pub use ambience::{AmbienceArea, LevelAmbience, ambience_for_level, bed_name, set_prefix};
pub use bank::{Lcg, SoundBank, SoundSet};
pub use environment::{
    AudioEnvironment, BusGains, Listener, SourceCommand, SourcePlay, SourceUpdate, UpdateInput,
    Voice, VoiceHandle, heard_gain, voice_flags,
};
pub use loops::{LoopSources, MAX_HELD};
pub use music::{
    LevelMusic, LevelMusicStart, LevelMusicVolume, MUSIC_VOLUME, MusicSetting, StoryMusic,
    music_for_level, story_music, story_music_changes,
};
pub use playback::{ANCHOR_HEIGHT_M, ListenerAnchor};
pub use reverb::{EaxReverb, LevelReverb, Reverb};
pub use sounds::{FOLEY_GOLEM, FOLEY_PLAYER, HEAL, HealSound, SWING, foley_set, hit_set};

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundBank>()
            .init_resource::<AudioEnvironment>()
            .init_resource::<playback::SourceEntities>()
            .init_resource::<MusicSetting>()
            .add_message::<sounds::HealSound>()
            .add_message::<exa_combat::impact::ImpactSounded>()
            .add_message::<exa_combat::impact::WeaponSounded>()
            .add_systems(
                Update,
                (
                    hits::hit_sounds,
                    hits::impact_sounds,
                    hits::weapon_sounds,
                    sounds::heal_sounds,
                    sounds::swing_sounds,
                    footsteps::footsteps,
                    music::start_music,
                    music::apply_music_setting,
                    ambience::set_level_ambience,
                    reverb::apply_level_reverb,
                    playback::advance_audio_environment,
                    playback::apply_source_commands,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use exa_assets::{GameData, PackKind, locate_install};

    use super::*;

    /// The installed `Sound.rpk` holds every variant of every set the
    /// sounds pick from, and no more: the sword impacts 01 to 06, grunts 01
    /// to 06 for each of the voices 01 to 09, the steps and the heavy steps
    /// 01 to 06 (skipped without `EXANIMA_DIR`).
    #[test]
    fn the_installed_sound_sets_have_every_variant() {
        let Some(dir) = locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        let sounds = game.pack(PackKind::Sound).unwrap();
        let voices: Vec<String> = (0..=hits::VOICES.count)
            .map(|v| format!("{}_grunts", hits::VOICES.name(v)))
            .collect();
        let (voices, past_voices) = voices.split_at(hits::VOICES.count);
        let mut sets = vec![
            hits::SWORD_IMPACTS,
            footsteps::STEPS,
            footsteps::HEAVY_STEPS,
        ];
        sets.extend(voices.iter().map(|v| SoundSet::new(v, hits::GRUNTS)));
        assert_eq!(sets.len(), 12);
        for set in sets {
            for i in 0..set.count {
                assert!(sounds.get(&set.name(i)).is_some(), "{}", set.name(i));
            }
            assert!(sounds.get(&set.name(set.count)).is_none(), "{set:?}");
        }
        let past = SoundSet::new(&past_voices[0], hits::GRUNTS).name(0);
        assert_eq!(past, "vhm10_grunts01");
        assert!(sounds.get(&past).is_none());
    }
}
