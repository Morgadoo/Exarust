//! The level's looping music or ambience.

use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use exa_assets::Game;

use crate::SoundBank;

/// The looping music or ambience to play (set by the level loader).
#[derive(Resource, Clone, Debug)]
pub struct LevelMusic(pub String);

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

#[derive(Component)]
pub(crate) struct MusicPlayer;

pub(crate) fn start_music(
    mut commands: Commands,
    game: Option<Res<Game>>,
    music: Option<Res<LevelMusic>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    playing: Query<Entity, With<MusicPlayer>>,
) {
    let (Some(game), Some(music)) = (game, music) else {
        return;
    };
    if !music.is_changed() {
        return;
    }
    for e in &playing {
        commands.entity(e).despawn();
    }
    if let Some(h) = bank.get(&game, &mut sources, &music.0) {
        commands.spawn((
            AudioPlayer(h),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::Linear(0.45),
                ..default()
            },
            MusicPlayer,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hub, the arena levels and the story levels each have their
    /// track; anything else plays the ambience.
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
