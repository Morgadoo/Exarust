//! Applies the mixer's source commands to Bevy audio: one entity per real
//! source slot, reusing the slot entities as the mixer rebinds them.
//!
//! The original drives OpenAL directly; Bevy's rodio backend has no source
//! handles, so each `Play` spawns an entity at the slot (the previous one is
//! despawned) and each refresh updates its sink. A finished one-shot shows as
//! an empty sink and goes back to the mixer as a stopped source.
//!
//! Two OpenAL behaviours the original relies on are reproduced here. The
//! distance model is the one the original sets up for every source (linear,
//! clamped between 4 m and 30 m, [`heard_gain`]); rodio's own inverse-square
//! falloff is scaled out of the way so it only pans. And a restart of a source
//! that is still playing or paused keeps its buffer, because OpenAL refuses a
//! new buffer then: a paused source resumes (the music overlay's way back to
//! the main track), a playing one starts its old buffer again.

use std::time::Duration;

use bevy::audio::{
    AudioSink, AudioSinkPlayback, PlaybackMode, PlaybackSettings, SpatialAudioSink, SpatialScale,
    Volume,
};
use bevy::prelude::*;

use crate::SoundBank;
use crate::environment::{
    AudioEnvironment, SOURCE_CAPACITY, SourceCommand, SourcePlay, UpdateInput, heard_gain,
};

/// The scale spatial positions are given: rodio's falloff (1/d², held at 1
/// near the listener) stays at 1 within 100 m, so only its panning acts.
const PANNING_ONLY_SCALE: f32 = 0.01;
/// The listener anchor's height above the anchoring entity: the original
/// anchors on the player's body plus 50 cm up.
pub const ANCHOR_HEIGHT_M: f32 = 0.5;

/// The entity the listener is anchored on, 50 cm above it (the player).
/// Without one the anchor is the spatial listener's own position.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ListenerAnchor;

/// The mixer's source slot on a playback entity.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct SourceSlot(pub u8);

/// What one source slot is playing: its entity and the buffer it was given.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SlotPlayback {
    entity: Entity,
    buffer: u32,
}

/// The playback on each source slot.
#[derive(Resource, Debug)]
pub(crate) struct SourceEntities([Option<SlotPlayback>; SOURCE_CAPACITY]);

impl Default for SourceEntities {
    fn default() -> Self {
        Self([None; SOURCE_CAPACITY])
    }
}

type Sinks = AnyOf<(&'static AudioSink, &'static SpatialAudioSink)>;
/// Both sink kinds, optional, so a playback entity whose sink Bevy has not
/// created yet still matches (and counts as a source that holds its buffer).
type SinksMut = (
    Option<&'static mut AudioSink>,
    Option<&'static mut SpatialAudioSink>,
);

/// The sink of a playback entity, plain or spatial.
fn sink<'a>(
    sinks: (Option<&'a AudioSink>, Option<&'a SpatialAudioSink>),
) -> Option<&'a dyn AudioSinkPlayback> {
    match sinks {
        (Some(sink), _) => Some(sink),
        (_, Some(sink)) => Some(sink),
        _ => None,
    }
}

/// Which source slots have finished playing.
fn finished_sources(query: &Query<(&SourceSlot, Sinks)>) -> [bool; SOURCE_CAPACITY] {
    let mut finished = [false; SOURCE_CAPACITY];
    for (slot, sinks) in query {
        if let Some(entry) = finished.get_mut(slot.0 as usize) {
            *entry = sink(sinks).is_some_and(|sink| sink.empty());
        }
    }
    finished
}

/// Feeds the real and game clocks, the listener, the window focus and the
/// finished sources into the mixer.
#[allow(clippy::too_many_arguments)]
pub(crate) fn advance_audio_environment(
    real: Res<Time<Real>>,
    game: Res<Time<Virtual>>,
    windows: Query<&Window>,
    listener: Query<&GlobalTransform, With<SpatialListener>>,
    anchor: Query<&GlobalTransform, With<ListenerAnchor>>,
    sinks: Query<(&SourceSlot, Sinks)>,
    mut env: ResMut<AudioEnvironment>,
    mut bank: ResMut<SoundBank>,
) {
    if let Ok(transform) = listener.single() {
        env.listener.anchor = transform.translation();
        env.listener.forward = transform.forward().as_vec3();
        env.listener.up = transform.up().as_vec3();
    }
    if let Ok(transform) = anchor.single() {
        env.listener.anchor = transform.translation() + Vec3::Y * ANCHOR_HEIGHT_M;
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
    mut playing: Query<(&mut Transform, SinksMut), With<SourceSlot>>,
) {
    let listener = env.listener.position();
    for command in env.commands() {
        match *command {
            SourceCommand::Stop { slot } => {
                if let Some(playback) = entities.0.get_mut(slot as usize).and_then(Option::take) {
                    commands.entity(playback.entity).despawn();
                }
            }
            SourceCommand::Pause { slot } | SourceCommand::Resume { slot } => {
                let Some(playback) = entities.0.get(slot as usize).copied().flatten() else {
                    continue;
                };
                let Ok((_, (plain, spatial))) = playing.get(playback.entity) else {
                    continue;
                };
                if let Some(sink) = sink((plain, spatial)) {
                    if matches!(command, SourceCommand::Pause { .. }) {
                        sink.pause();
                    } else {
                        sink.play();
                    }
                }
            }
            SourceCommand::Refresh(update) => {
                let Some(playback) = entities.0.get(update.slot as usize).copied().flatten() else {
                    continue;
                };
                let Ok((mut transform, sinks)) = playing.get_mut(playback.entity) else {
                    continue;
                };
                let volume = Volume::Linear(heard_gain(
                    update.gain,
                    update.min_gain,
                    update.position,
                    listener,
                ));
                match sinks {
                    (Some(mut sink), _) => {
                        sink.set_volume(volume);
                        sink.set_speed(update.pitch);
                    }
                    (_, Some(mut sink)) => {
                        sink.set_volume(volume);
                        sink.set_speed(update.pitch);
                    }
                    _ => {}
                }
                transform.translation = update.position;
            }
            SourceCommand::Play(play) => {
                let slot = play.slot as usize;
                let held = entities.0.get(slot).copied().flatten();
                let mut buffer = play.buffer;
                if play.restart
                    && let Some(held) = held
                {
                    // The source still has its buffer unless it has run out.
                    let state = playing
                        .get(held.entity)
                        .ok()
                        .map(|(_, sinks)| sink(sinks).map(|sink| (sink.empty(), sink.is_paused())));
                    match state {
                        Some(Some((false, true))) => {
                            if let Ok((_, sinks)) = playing.get(held.entity)
                                && let Some(sink) = sink(sinks)
                            {
                                sink.play();
                            }
                            continue;
                        }
                        Some(Some((true, _))) | None => {}
                        Some(_) => buffer = held.buffer,
                    }
                }
                if let Some(held) = entities.0.get_mut(slot).and_then(Option::take) {
                    commands.entity(held.entity).despawn();
                }
                let Some(handle) = bank.buffer_handle(buffer) else {
                    continue;
                };
                let entity = commands
                    .spawn((
                        AudioPlayer(handle),
                        settings(&play, listener),
                        Transform::from_translation(play.position),
                        SourceSlot(play.slot),
                    ))
                    .id();
                if let Some(entry) = entities.0.get_mut(slot) {
                    *entry = Some(SlotPlayback { entity, buffer });
                }
            }
        }
    }
}

/// The playback settings a bound source starts with.
fn settings(play: &SourcePlay, listener: Vec3) -> PlaybackSettings {
    PlaybackSettings {
        mode: if play.looping {
            PlaybackMode::Loop
        } else {
            PlaybackMode::Once
        },
        volume: Volume::Linear(heard_gain(
            play.gain,
            play.min_gain,
            play.position,
            listener,
        )),
        speed: play.pitch,
        paused: play.paused,
        spatial: !play.relative,
        spatial_scale: Some(SpatialScale::new(PANNING_ONLY_SCALE)),
        start_position: (play.offset > 0.0).then(|| Duration::from_secs_f32(play.offset)),
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(slot: u8, buffer: u32) -> SourcePlay {
        SourcePlay {
            slot,
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
            restart: false,
        }
    }

    fn app_with_buffer() -> (App, u32) {
        let mut app = App::new();
        app.init_resource::<AudioEnvironment>()
            .init_resource::<SoundBank>()
            .init_resource::<SourceEntities>()
            .init_resource::<Assets<AudioSource>>()
            .add_systems(Update, apply_source_commands);
        let handle = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: std::sync::Arc::from(vec![0u8; 4]),
            });
        let mut bank = app.world_mut().resource_mut::<SoundBank>();
        let buffer = bank.insert_test_buffer(1.0, 1000, false);
        bank.set_test_buffer_handle(buffer, handle);
        (app, buffer)
    }

    fn send(app: &mut App, command: SourceCommand) {
        let mut env = app.world_mut().resource_mut::<AudioEnvironment>();
        env.clear_test_commands();
        env.push_test_command(command);
        app.update();
    }

    fn slot_entities(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<SourceSlot>>();
        query.iter(world).count()
    }

    /// A play command spawns one entity on its slot with the source's gain
    /// after distance, pitch, looping and a panning-only spatial scale; a
    /// stop despawns it.
    #[test]
    fn play_and_stop_spawn_and_despawn_a_slot() {
        let (mut app, buffer) = app_with_buffer();
        send(&mut app, SourceCommand::Play(play(2, buffer)));
        assert_eq!(slot_entities(&mut app), 1);
        let world = app.world_mut();
        let mut query = world.query::<&PlaybackSettings>();
        let settings = query.single(world).unwrap();
        // The default listener sits at the origin; the source is √50 ≈ 7.07 m
        // away, so OpenAL's slope gives 1 − (707 − 400)/2600 of the gain.
        let distance = Vec3::new(3.0, 4.0, 5.0).length() * 100.0;
        let expected = 0.5 * (1.0 - (distance - 400.0) / 2600.0);
        assert!((settings.volume.to_linear() - expected).abs() < 1e-6);
        assert_eq!(settings.speed, 1.25);
        assert!(matches!(settings.mode, PlaybackMode::Loop));
        assert!(settings.spatial);
        assert!(settings.spatial_scale.is_some());
        send(&mut app, SourceCommand::Stop { slot: 2 });
        assert_eq!(slot_entities(&mut app), 0);
    }

    /// A restart of a slot whose source is still there keeps the buffer it
    /// was given (OpenAL refuses a new one): the replacement entity plays the
    /// old buffer. A restart of an empty slot takes the new buffer.
    #[test]
    fn a_restart_keeps_a_live_sources_buffer() {
        let (mut app, buffer) = app_with_buffer();
        send(&mut app, SourceCommand::Play(play(1, buffer)));
        let other = app
            .world_mut()
            .resource_mut::<SoundBank>()
            .insert_test_buffer(1.0, 1000, false);
        let handle = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: std::sync::Arc::from(vec![1u8; 4]),
            });
        app.world_mut()
            .resource_mut::<SoundBank>()
            .set_test_buffer_handle(other, handle);
        let restart = SourcePlay {
            restart: true,
            ..play(1, other)
        };
        send(&mut app, SourceCommand::Play(restart));
        let held = app.world().resource::<SourceEntities>().0[1].unwrap();
        assert_eq!(held.buffer, buffer);
        send(&mut app, SourceCommand::Stop { slot: 1 });
        send(&mut app, SourceCommand::Play(restart));
        let held = app.world().resource::<SourceEntities>().0[1].unwrap();
        assert_eq!(held.buffer, other);
    }
}
