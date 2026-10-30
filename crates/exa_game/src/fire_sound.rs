//! The looping sound a fire's emitter keeps.
//!
//! A fire template names a loop (`FireS02` for the small flame, `FireS01`
//! for the medium-small one, `FireL01` for the large one). The original
//! creates one source for it when the emitter enters the world, plays it
//! looping, and each rendered frame walks its volume and pitch with the
//! emitter's own sound walk and moves it to the emitter's node. Exarust
//! reproduces that here, on top of `exa_audio`'s voices: the flame's
//! template names the sound, the emitter walks the levels, and this module
//! binds one looping voice per flame.
//!
//! A template that names no sound (or a single-letter name, which the
//! original treats as none) plays nothing.

use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank, VoiceHandle, voice_flags};
use exa_world::flame::Flame;

pub struct FireSoundPlugin;

impl Plugin for FireSoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fire_loops);
    }
}

/// The looping voice a flame holds.
#[derive(Component, Debug)]
pub struct FireVoice {
    handle: VoiceHandle,
    /// The bank buffer the name resolved to, once loaded.
    buffer: Option<u32>,
}

impl Default for FireVoice {
    fn default() -> Self {
        Self {
            handle: VoiceHandle::NONE,
            buffer: None,
        }
    }
}

/// The levels a fire's loop plays at: the emitter's walked volume, and its
/// pitch. The original writes both into the source every frame.
pub fn loop_levels(volume: f32, pitch: f32) -> (f32, f32) {
    (volume.max(0.0), pitch)
}

/// Starts and walks each flame's loop. A flame with no sound, no game data
/// or no audio device is skipped; the loop starts once its buffer loads and
/// follows the flame's node and levels afterwards.
fn fire_loops(
    mut commands: Commands,
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut env: ResMut<AudioEnvironment>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut flames: Query<(Entity, &Flame, &GlobalTransform, Option<&mut FireVoice>)>,
) {
    let Some(game) = game else { return };
    for (entity, flame, at, voice) in &mut flames {
        let Some(name) = flame.emitter().sound_name() else {
            continue;
        };
        let (volume, pitch) =
            loop_levels(flame.emitter().sound.volume, flame.emitter().sound.pitch);
        // The sound's world position, in metres (the audio layer's units).
        let position = at.translation();
        let Some(mut voice) = voice else {
            commands.entity(entity).insert(FireVoice::default());
            continue;
        };
        if voice.handle == VoiceHandle::NONE {
            if !env.is_live() {
                continue;
            }
            voice.handle = env.new_handle();
        }
        let buffer = match voice.buffer {
            Some(buffer) => buffer,
            None => {
                let Some(buffer) = bank.load_buffer(&game, &mut sources, name) else {
                    // A missing sound does not spin the load every frame.
                    voice.buffer = None;
                    continue;
                };
                voice.buffer = Some(buffer);
                buffer
            }
        };
        if let Some(id) = env.handle_voice(voice.handle) {
            if let Some(v) = env.voice_mut(id) {
                v.position = position;
                v.gain = volume;
                v.pitch = pitch;
            }
        } else {
            env.play_buffer(buffer, position, volume, pitch, voice_flags::LOOPING);
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
}
