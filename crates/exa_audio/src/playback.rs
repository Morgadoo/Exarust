//! Applies the mixer's source commands to Bevy audio: one entity per real
//! source slot, reusing the slot entities as the mixer rebinds them.
//!
//! The original drives OpenAL directly; Bevy's rodio backend has no source
//! handles, so each `Play` spawns an entity at the slot (the previous one is
//! despawned) and each refresh updates its sink. A finished one-shot shows as
//! an empty sink and goes back to the mixer as a stopped source. Distance
//! attenuation is still Bevy's (the original's OpenAL distance model is not
//! in the decompilation pack); the mixer's ranking only decides which voices
//! get a source.

use std::time::Duration;

use bevy::audio::{AudioSink, AudioSinkPlayback, PlaybackMode, PlaybackSettings, Volume};
use bevy::prelude::*;

use crate::SoundBank;
use crate::environment::{AudioEnvironment, SOURCE_CAPACITY, SourceCommand, UpdateInput};

/// The mixer's source slot on a playback entity.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct SourceSlot(pub u8);

/// The entity playing each source slot.
#[derive(Resource, Debug)]
pub(crate) struct SourceEntities([Option<Entity>; SOURCE_CAPACITY]);

impl Default for SourceEntities {
    fn default() -> Self {
        Self(std::array::from_fn(|_| None))
    }
}

/// Which source slots have finished playing.
fn finished_sources(query: &Query<(&SourceSlot, &AudioSink)>) -> [bool; SOURCE_CAPACITY] {
    let mut finished = [false; SOURCE_CAPACITY];
    for (slot, sink) in query {
        if let Some(entry) = finished.get_mut(slot.0 as usize) {
            *entry = sink.empty();
        }
    }
    finished
}

/// Feeds the real and game clocks, the listener, the window focus and the
/// finished sources into the mixer.
pub(crate) fn advance_audio_environment(
    real: Res<Time<Real>>,
    game: Res<Time<Virtual>>,
    windows: Query<&Window>,
    listener: Query<&GlobalTransform, With<SpatialListener>>,
    sinks: Query<(&SourceSlot, &AudioSink)>,
    mut env: ResMut<AudioEnvironment>,
    mut bank: ResMut<SoundBank>,
) {
    if let Ok(transform) = listener.single() {
        env.listener.anchor = transform.translation();
        env.listener.forward = transform.forward().as_vec3();
        env.listener.up = transform.up().as_vec3();
    }
    let focused = windows.is_empty() || windows.iter().any(|window| window.focused);
    let input = UpdateInput {
        now_ms: real.elapsed().as_millis() as u32,
        game_ms: game.elapsed().as_millis() as u32,
        paused: game.is_paused(),
        focused,
        source_stopped: finished_sources(&sinks),
    };
    env.update(&mut bank, &input);
}

/// Applies the mixer's commands to the playback entities.
pub(crate) fn apply_source_commands(
    mut commands: Commands,
    env: Res<AudioEnvironment>,
    bank: Res<SoundBank>,
    mut entities: ResMut<SourceEntities>,
    mut playing: Query<(&SourceSlot, &mut AudioSink, &mut Transform)>,
) {
    for command in env.commands() {
        match *command {
            SourceCommand::Stop { slot } => {
                if let Some(entity) = entities.0.get_mut(slot as usize).and_then(Option::take) {
                    commands.entity(entity).despawn();
                }
            }
            SourceCommand::Pause { slot } => {
                if let Some(entity) = entities.0.get(slot as usize).copied().flatten()
                    && let Ok((_, sink, _)) = playing.get(entity)
                {
                    sink.pause();
                }
            }
            SourceCommand::Resume { slot } => {
                if let Some(entity) = entities.0.get(slot as usize).copied().flatten()
                    && let Ok((_, sink, _)) = playing.get(entity)
                {
                    sink.play();
                }
            }
            SourceCommand::Refresh(update) => {
                let Some(entity) = entities.0.get(update.slot as usize).copied().flatten() else {
                    continue;
                };
                let Ok((_, mut sink, mut transform)) = playing.get_mut(entity) else {
                    continue;
                };
                sink.set_volume(Volume::Linear(update.gain));
                sink.set_speed(update.pitch);
                transform.translation = update.position;
            }
            SourceCommand::Play(play) => {
                let slot = play.slot as usize;
                if let Some(entity) = entities.0.get_mut(slot).and_then(Option::take) {
                    commands.entity(entity).despawn();
                }
                let Some(handle) = bank.buffer_handle(play.buffer) else {
                    continue;
                };
                let settings = PlaybackSettings {
                    mode: if play.looping {
                        PlaybackMode::Loop
                    } else {
                        PlaybackMode::Once
                    },
                    volume: Volume::Linear(play.gain),
                    speed: play.pitch,
                    paused: play.paused,
                    spatial: !play.relative,
                    start_position: (play.offset > 0.0)
                        .then(|| Duration::from_secs_f32(play.offset)),
                    ..default()
                };
                let entity = commands
                    .spawn((
                        AudioPlayer(handle),
                        settings,
                        Transform::from_translation(play.position),
                        SourceSlot(play.slot),
                    ))
                    .id();
                if let Some(entry) = entities.0.get_mut(slot) {
                    *entry = Some(entity);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::SourcePlay;

    /// A play command spawns one entity on its slot with the source's gain,
    /// pitch, looping and position; a stop despawns it.
    #[test]
    fn play_and_stop_spawn_and_despawn_a_slot() {
        let mut app = App::new();
        app.init_resource::<AudioEnvironment>()
            .init_resource::<SoundBank>()
            .init_resource::<SourceEntities>()
            .add_systems(Update, apply_source_commands);
        let mut bank = app.world_mut().resource_mut::<SoundBank>();
        let buffer = bank.insert_test_buffer(1.0, 1000, false);
        // Give the buffer an asset to resolve to.
        app.init_resource::<Assets<AudioSource>>();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: std::sync::Arc::from(vec![0u8; 4]),
            });
        app.world_mut()
            .resource_mut::<SoundBank>()
            .set_test_buffer_handle(buffer, handle);
        {
            let mut env = app.world_mut().resource_mut::<AudioEnvironment>();
            env.push_test_command(SourceCommand::Play(SourcePlay {
                slot: 2,
                buffer,
                gain: 0.5,
                pitch: 1.25,
                offset: 0.0,
                looping: true,
                paused: false,
                position: Vec3::new(3.0, 4.0, 5.0),
                min_gain: None,
                relative: false,
                reverb: true,
            }));
        }
        app.update();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<SourceSlot>>();
        assert_eq!(query.iter(world).count(), 1);
        // A stop removes it.
        app.world_mut()
            .resource_mut::<AudioEnvironment>()
            .clear_test_commands();
        app.world_mut()
            .resource_mut::<AudioEnvironment>()
            .push_test_command(SourceCommand::Stop { slot: 2 });
        app.update();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<SourceSlot>>();
        assert_eq!(query.iter(world).count(), 0);
    }
}
