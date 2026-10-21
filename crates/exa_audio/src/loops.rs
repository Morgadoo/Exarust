//! The looping sources a character keeps: one per held item and two for its
//! own body (its voice and its armour/clothing foley). The original's part
//! allocates them lazily and frees them on death; the handles here are the
//! same idea on top of [`AudioEnvironment`]'s voices.
//!
//! Evidence: the part's source creator and destructor (one source per held
//! body and a pair for the body) and the foley's use of the body pair.

use bevy::prelude::*;

use crate::{AudioEnvironment, SoundBank, VoiceHandle};

/// The most held items a character keeps a source for (the two hands).
pub const MAX_HELD: usize = 2;

/// A character's looping sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopSources {
    /// One per held item, in hand order.
    pub held: [VoiceHandle; MAX_HELD],
    /// The character's voice loop and its body's foley loop.
    pub voice: VoiceHandle,
    pub foley: VoiceHandle,
}

impl Default for LoopSources {
    fn default() -> Self {
        Self {
            held: [VoiceHandle::NONE; MAX_HELD],
            voice: VoiceHandle::NONE,
            foley: VoiceHandle::NONE,
        }
    }
}

impl LoopSources {
    /// Allocates the missing handles: one per held item (`held` items) and
    /// the body's pair.
    pub fn reserve(&mut self, env: &mut AudioEnvironment, held: usize) {
        for slot in self.held.iter_mut().take(held.min(MAX_HELD)) {
            if *slot == VoiceHandle::NONE {
                *slot = env.new_handle();
            }
        }
        if self.voice == VoiceHandle::NONE {
            self.voice = env.new_handle();
            self.foley = env.new_handle();
        }
    }

    /// Frees every bound source and clears the handles.
    pub fn free(&mut self, env: &mut AudioEnvironment, bank: &mut SoundBank) {
        for slot in &mut self.held {
            if *slot != VoiceHandle::NONE {
                env.free_handle(bank, *slot);
                *slot = VoiceHandle::NONE;
            }
        }
        if self.voice != VoiceHandle::NONE {
            env.free_handle(bank, self.voice);
            env.free_handle(bank, self.foley);
            self.voice = VoiceHandle::NONE;
            self.foley = VoiceHandle::NONE;
        }
    }

    /// Plays a held item's loop on its source, or nothing when the source
    /// is not reserved.
    #[allow(clippy::too_many_arguments)]
    pub fn play_held(
        &mut self,
        env: &mut AudioEnvironment,
        bank: &mut SoundBank,
        index: usize,
        set: u32,
        position: Vec3,
        gain: f32,
        pitch: f32,
    ) -> bool {
        let Some(&handle) = self.held.get(index) else {
            return false;
        };
        handle != VoiceHandle::NONE
            && env.play_loop_attached(bank, set, handle, position, gain, pitch)
    }

    /// Plays the body's voice loop or its foley loop.
    #[allow(clippy::too_many_arguments)]
    pub fn play_body(
        &mut self,
        env: &mut AudioEnvironment,
        bank: &mut SoundBank,
        foley: bool,
        set: u32,
        position: Vec3,
        gain: f32,
        pitch: f32,
    ) -> bool {
        let handle = if foley { self.foley } else { self.voice };
        handle != VoiceHandle::NONE
            && env.play_loop_attached(bank, set, handle, position, gain, pitch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sources_are_made_lazily_and_freed() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let mut sources = LoopSources::default();
        sources.reserve(&mut env, 2);
        assert!(sources.held.iter().all(|h| *h != VoiceHandle::NONE));
        assert!(sources.voice != VoiceHandle::NONE);
        assert!(sources.foley != VoiceHandle::NONE);
        // A second reserve keeps the handles.
        let before = sources;
        sources.reserve(&mut env, 1);
        assert_eq!(sources, before);
        // A looping play binds the held source and loops.
        let set = bank.insert_test_set(vec![1, 2]);
        assert!(sources.play_held(&mut env, &mut bank, 0, set, Vec3::ZERO, 0.5, 1.0));
        assert!(env.handle_is_bound(sources.held[0]));
        let voice = env.handle_voice(sources.held[0]).unwrap();
        assert_ne!(
            env.voice(voice).unwrap().flags & crate::voice_flags::LOOPING,
            0
        );
        // Freeing clears every handle.
        sources.free(&mut env, &mut bank);
        assert_eq!(sources, LoopSources::default());
        assert!(!env.handle_is_bound(before.held[0]));
    }
}
