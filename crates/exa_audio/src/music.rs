//! The level's looping music or ambience.

use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use exa_assets::Game;

use crate::SoundBank;

/// The looping music or ambience to play (set by the level loader).
#[derive(Resource, Clone, Debug)]
pub struct LevelMusic(pub String);

/// Music for a level name, following the original's track naming.
pub fn music_for_level(level: &str) -> &'static str {
    let l = level.to_ascii_lowercase();
    if l.starts_with("arenahub") {
        "msc_hub01"
    } else if l.starts_with("arenao") {
        "msc_arenao"
    } else if l.starts_with("arena") {
        "msc_arena3"
    } else if l.starts_with("exanima0") || l.starts_with("exanimac") {
        "msc_uw01amb"
    } else {
        "ambntra01"
    }
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
        assert_eq!(music_for_level("exanimac"), "msc_uw01amb");
        assert_eq!(music_for_level("charview"), "ambntra01");
    }
}
