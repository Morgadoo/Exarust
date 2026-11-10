//! The teleport bubble's two sounds.
//!
//! - **The spawn** (the journey's bubble creator): `teleportf01` and
//!   `teleportc01` are loaded once; the bubble takes a voice of its own
//!   for `teleportf01` at its place, volume 0.3, pitch 1 (no extra flags:
//!   a one-shot that stays bound to the bubble).
//! - **The shrink** (the bubble's step, state 5): while the voice still
//!   exists the step writes its volume, `(value + 1)·0.5` with the value
//!   running 0 to −1 ([`Bubble::sound_volume`]), so the sound jumps to 0.5
//!   when the shrink starts and falls to nothing with the bubble.
//! - **The close** (the bubble's step, the grow done without the hold):
//!   `teleportc01` at the bubble's place, volume 0.3, pitch 1, flags 0.
//! - **The end** (the bubble's end routine): the voice is freed.
//!
//! The bubble's step runs in the fixed step; this frame system follows
//! each bubble's state as the frame sees it and plays what the step's
//! transitions ask for: a bubble seen for the first time takes its voice,
//! one seen leaving the grow for the rest plays the close, and a bubble
//! gone frees its voice.
//!
//! Evidence: the bubble's creator, step and end routines, read from
//! `Exanima.exe`.

use std::collections::HashMap;

use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank, VoiceHandle};

use super::bubble::{Bubble, BubbleEffect, BubbleState};

/// The spawn sound, played on the bubble's own voice.
pub const SPAWN_SOUND: &str = "teleportf01";
/// The close sound, a one-shot when a plain bubble has grown.
pub const CLOSE_SOUND: &str = "teleportc01";
/// Both sounds' volume.
pub const VOLUME: f32 = 0.3;
/// Both sounds' pitch.
pub const PITCH: f32 = 1.0;

/// What the bubble's step asks of its sounds, from its state the frame
/// saw before (`None` for a bubble seen the first time) and its state now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BubbleSounds {
    /// Take the bubble's voice and start `teleportf01`.
    pub spawn: bool,
    /// Play `teleportc01` once.
    pub close: bool,
    /// The voice's volume this frame.
    pub volume: Option<f32>,
}

/// The sounds a bubble's state change asks for: the spawn when it is new,
/// the close when it has left the grow for the rest, and the shrink's
/// volume while shrinking.
pub fn bubble_sounds(before: Option<BubbleState>, bubble: &Bubble) -> BubbleSounds {
    let close =
        bubble.state == BubbleState::Idle && before.is_none_or(|state| state == BubbleState::Grow);
    BubbleSounds {
        spawn: before.is_none(),
        close,
        volume: (bubble.state == BubbleState::Shrink).then_some(bubble.sound_volume),
    }
}

/// The sound bank and environment the bubbles play on.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct BubbleAudio<'w> {
    game: Option<Res<'w, Game>>,
    bank: Option<ResMut<'w, SoundBank>>,
    sources: Option<ResMut<'w, Assets<AudioSource>>>,
    env: Option<ResMut<'w, AudioEnvironment>>,
}

/// The two buffers (loaded once; `None` inside when one failed) and each
/// bubble's voice and last seen state.
#[derive(Default)]
pub(crate) struct BubbleVoices {
    spawn: Option<Option<u32>>,
    close: Option<Option<u32>>,
    bubbles: HashMap<Entity, (VoiceHandle, BubbleState)>,
}

/// Each frame: the new bubbles' voices, the close sounds, the shrinking
/// voices' volume and the voices of bubbles that have gone.
pub(crate) fn sound_bubbles(
    bubbles: Query<(Entity, &BubbleEffect, &GlobalTransform)>,
    mut audio: BubbleAudio,
    mut state: Local<BubbleVoices>,
) {
    let (Some(game), Some(bank), Some(sources), Some(env)) = (
        audio.game.as_deref(),
        audio.bank.as_deref_mut(),
        audio.sources.as_deref_mut(),
        audio.env.as_deref_mut(),
    ) else {
        return;
    };
    let gone: Vec<Entity> = state
        .bubbles
        .keys()
        .copied()
        .filter(|&e| !bubbles.contains(e))
        .collect();
    for entity in gone {
        if let Some((handle, _)) = state.bubbles.remove(&entity) {
            env.free_handle(bank, handle);
            info!("bubble {entity:?}: gone, its {SPAWN_SOUND} voice freed");
        }
    }
    for (entity, bubble, at) in &bubbles {
        let at = at.translation();
        let before = state.bubbles.get(&entity).map(|&(_, state)| state);
        let sounds = bubble_sounds(before, &bubble.0);
        let handle = match state.bubbles.get(&entity) {
            Some(&(handle, _)) => handle,
            None => {
                let handle = env.new_handle();
                if let Some(buffer) = *state
                    .spawn
                    .get_or_insert_with(|| bank.load_buffer(game, sources, SPAWN_SOUND))
                {
                    let played = env.play_buffer_attached(buffer, handle, at, VOLUME, PITCH, 0);
                    info!("bubble {entity:?}: {SPAWN_SOUND} at {at:?}, voice {played}");
                }
                handle
            }
        };
        state.bubbles.insert(entity, (handle, bubble.0.state));
        if sounds.close
            && let Some(buffer) = *state
                .close
                .get_or_insert_with(|| bank.load_buffer(game, sources, CLOSE_SOUND))
        {
            let voice = env.play_buffer(buffer, at, VOLUME, PITCH, 0);
            info!("bubble {entity:?}: {CLOSE_SOUND} at {at:?}, voice {voice:?}");
        }
        if let Some(volume) = sounds.volume
            && let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id))
        {
            voice.gain = volume;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_bubble_spawns_closes_and_fades_its_voice() {
        let mut bubble = Bubble::new(0, false, [0.0, 0.0]);
        let first = bubble_sounds(None, &bubble);
        assert!(first.spawn && !first.close && first.volume.is_none());
        bubble.step(1_801);
        let closed = bubble_sounds(Some(BubbleState::Grow), &bubble);
        assert!(!closed.spawn && closed.close);
        // Seen resting again: no second close.
        assert!(!bubble_sounds(Some(BubbleState::Idle), &bubble).close);
        bubble.step(2_302);
        bubble.step(2_303);
        bubble.step(2_503);
        let shrinking = bubble_sounds(Some(BubbleState::Shrink), &bubble);
        // Half-way down: the value is −0.5, the volume (−0.5 + 1)·0.5.
        assert_eq!(shrinking.volume, Some(0.25));
    }

    #[test]
    fn a_holding_bubble_never_closes() {
        let mut bubble = Bubble::new(0, true, [0.0, 0.0]);
        bubble.step(1_801);
        assert_eq!(bubble.state, BubbleState::Hold);
        assert!(!bubble_sounds(Some(BubbleState::Grow), &bubble).close);
    }
}
