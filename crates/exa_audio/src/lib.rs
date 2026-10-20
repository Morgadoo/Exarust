//! Audio from `Sound.rpk` (RIFF/WAVE PCM): music and ambience per level
//! (`music`), weapon impacts and pain sounds on hits (`hits`), footsteps for
//! moving characters (`footsteps`), each event choosing among a sound set's
//! numbered variants through the bank's one picker (`bank`). The original
//! mixes through OpenAL Soft with EFX reverb; reverb zones come later
//! (`docs/PLAN.md`, Phase 5).

mod bank;
mod footsteps;
mod hits;
mod music;

use bevy::prelude::*;

pub use bank::{SoundBank, SoundSet};
pub use music::{LevelMusic, StoryMusic, music_for_level, story_music, story_music_changes};

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundBank>().add_systems(
            Update,
            (hits::hit_sounds, footsteps::footsteps, music::start_music),
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
