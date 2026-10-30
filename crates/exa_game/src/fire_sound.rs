//! The looping sound a fire's emitter keeps.
//!
//! A fire template names a loop (`FireS02` for the small flame, `FireS01`
//! for the medium-small one, `FireL01` for the large one). The original
//! creates one source for it when the emitter enters the world, plays it
//! looping, and each rendered frame walks its volume and pitch with the
//! emitter's own sound walk and moves it to the emitter's node. Exarust
//! reproduces that here, on top of `exa_audio`'s voices: the flame's
//! template names the sound, the emitter walks the levels, and this module
//! keeps one looping voice per flame on a handle of its own, freeing it (and
//! its buffer reference) when the flame goes away.
//!
//! A template that names no sound (or a single-letter name, which the
//! original treats as none) plays nothing. A sound that fails to load is
//! not asked for again for that flame (the original instead recreates the
//! source when a late buffer arrives).

use std::collections::HashMap;

use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank, VoiceHandle, voice_flags};
use exa_world::flame::Flame;

pub struct FireSoundPlugin;

impl Plugin for FireSoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FireVoices>()
            .add_systems(Update, fire_loops);
    }
}

/// The looping voice a flame holds.
#[derive(Debug, Clone, Copy)]
struct FireVoice {
    handle: VoiceHandle,
    /// The bank buffer the name resolved to; `None` once a load failed.
    buffer: Option<u32>,
}

/// Each flame's voice, by the flame's entity. Kept apart from the flame so
/// a despawned flame's voice can still be found and freed.
#[derive(Resource, Default)]
struct FireVoices(HashMap<Entity, FireVoice>);

/// The levels a fire's loop plays at: the emitter's walked volume, and its
/// pitch. The original writes both into the source every frame.
pub fn loop_levels(volume: f32, pitch: f32) -> (f32, f32) {
    (volume.max(0.0), pitch)
}

/// Starts and walks each flame's loop, and frees the loops of flames that
/// are gone. A flame with no sound, no game data or no audio device is
/// skipped; the loop follows the flame's node and levels once started, and
/// is started again if the voice pool took it away.
fn fire_loops(
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut env: ResMut<AudioEnvironment>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut voices: ResMut<FireVoices>,
    flames: Query<(Entity, &Flame, &GlobalTransform)>,
) {
    // Free the voices of flames that were despawned (a finished one-shot
    // fire, a level change).
    if !voices.0.is_empty() {
        let gone: Vec<Entity> = voices
            .0
            .keys()
            .copied()
            .filter(|&e| !flames.contains(e))
            .collect();
        for entity in gone {
            if let Some(voice) = voices.0.remove(&entity) {
                env.free_handle(&mut bank, voice.handle);
                if let Some(buffer) = voice.buffer {
                    bank.release_buffer(buffer);
                }
            }
        }
    }
    let Some(game) = game else { return };
    if !env.is_live() {
        return;
    }
    for (entity, flame, at) in &flames {
        let Some(name) = flame.emitter().sound_name() else {
            continue;
        };
        let voice = match voices.0.get(&entity) {
            Some(voice) => *voice,
            None => {
                let voice = FireVoice {
                    handle: env.new_handle(),
                    buffer: bank.load_buffer(&game, &mut sources, name),
                };
                voices.0.insert(entity, voice);
                voice
            }
        };
        let Some(buffer) = voice.buffer else {
            continue;
        };
        let (volume, pitch) =
            loop_levels(flame.emitter().sound.volume, flame.emitter().sound.pitch);
        // The sound's world position, in metres (the audio layer's units).
        let position = at.translation();
        match env.handle_voice(voice.handle) {
            Some(id) => {
                if let Some(v) = env.voice_mut(id) {
                    v.position = position;
                    v.gain = volume;
                    v.pitch = pitch;
                }
            }
            None => {
                env.play_buffer_attached(
                    buffer,
                    voice.handle,
                    position,
                    volume,
                    pitch,
                    voice_flags::LOOPING,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_loop_plays_at_the_walked_levels() {
        // The emitter's walk keeps the volume within the template's bounds
        // and the pitch within 0.85–1.15; the loop passes both through, and
        // a negative volume is clamped away.
        assert_eq!(loop_levels(0.5, 0.9), (0.5, 0.9));
        assert_eq!(loop_levels(-0.2, 1.1), (0.0, 1.1));
    }

    #[test]
    fn a_flame_keeps_one_voice_and_frees_it_when_gone() {
        // One looping voice per handle, however many frames re-level it:
        // replaying only when the handle is unbound keeps the pool from
        // filling with copies.
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let handle = env.new_handle();
        for frame in 0..10 {
            match env.handle_voice(handle) {
                Some(id) => env.voice_mut(id).unwrap().gain = frame as f32 * 0.1,
                None => {
                    assert!(env.play_buffer_attached(
                        1,
                        handle,
                        Vec3::ZERO,
                        1.0,
                        1.0,
                        voice_flags::LOOPING
                    ));
                }
            }
        }
        assert_eq!(env.high_water(), 1);
        env.free_handle(&mut bank, handle);
        assert_eq!(env.handle_voice(handle), None);
    }
}
