//! Sounds of hits: a weapon impact and a pain grunt at the contact point.

use bevy::audio::Volume;
use bevy::prelude::*;
use exa_assets::Game;
use exa_combat::Hit;

use crate::{SoundBank, SoundSet};

/// Sword impacts.
pub(crate) const SWORD_IMPACTS: SoundSet<'static> = SoundSet::new("wh_sworda", 6);
/// Human voices, each with its grunts.
pub(crate) const VOICES: SoundSet<'static> = SoundSet::new("vhm", 9);
pub(crate) const GRUNTS: usize = 6;

pub(crate) fn hit_sounds(
    mut commands: Commands,
    game: Option<Res<Game>>,
    mut hits: MessageReader<Hit>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    let Some(game) = game else { return };
    for hit in hits.read() {
        let impact = bank.pick_from(SWORD_IMPACTS);
        let voice = format!("{}_grunts", bank.pick_from(VOICES));
        let grunt = bank.pick_from(SoundSet::new(&voice, GRUNTS));
        for (name, volume) in [(impact, 1.0), (grunt, 0.8)] {
            if let Some(h) = bank.get(&game, &mut sources, &name) {
                commands.spawn((
                    AudioPlayer(h),
                    PlaybackSettings::DESPAWN
                        .with_spatial(true)
                        .with_volume(Volume::Linear(volume)),
                    Transform::from_translation(hit.point),
                ));
            }
        }
    }
}
