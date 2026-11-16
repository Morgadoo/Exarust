//! A cloth's rustle: the voice its set-up allocates (and the set its
//! record names, `CltMov` for the level's cloths), which the cloth's step
//! retriggers on a fresh variant whenever the last one has ended while the
//! cloth moves fast, and follows with the cloth's centre, volume and pitch;
//! a slow step asks it to stop ([`exa_physics::cloth::ClothRustled`]).
//!
//! Evidence: the cloth's parameter-record set-up (a voice from the pool, or
//! the shared dummy when full, and the set by its base name) and its
//! integrate slot's rustle block (the retrigger when the voice's active,
//! restart and loop bits are clear, at pitch 0.5, then the place, gain and
//! pitch; the stop bit when slow), REA assembly.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use exa_assets::Game;
use exa_physics::cloth::ClothRustled;

use crate::environment::voice_flags;
use crate::{AudioEnvironment, SoundBank};

/// The bits of a voice that mean it still sounds: active, restart, loop.
const SOUNDING: u32 = voice_flags::ACTIVE | voice_flags::RESTART | voice_flags::LOOPING;

/// A cloth's rustle voice and its set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RustleVoice {
    pub voice: u32,
    pub set: u32,
}

/// Follows each cloth's rustle: the first message a cloth sends gives it
/// its voice and set; a playing step retriggers an ended voice and moves,
/// levels and pitches it; a stopping step sets its stop bit. A cloth that
/// is gone lets its voice go.
pub(crate) fn cloth_rustles(
    game: Option<Res<Game>>,
    mut rustles: MessageReader<ClothRustled>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    entities: Query<()>,
    mut voices: Local<HashMap<Entity, RustleVoice>>,
) {
    let Some(game) = game else {
        rustles.clear();
        return;
    };
    for rustle in rustles.read() {
        let held = *voices.entry(rustle.cloth).or_insert_with(|| RustleVoice {
            voice: env.allocate_or_dummy(),
            set: bank.load_set(&game, &mut sources, &rustle.sound.to_ascii_lowercase()),
        });
        follow(&mut env, &mut bank, held, rustle);
    }
    voices.retain(|&cloth, held| {
        let alive = entities.contains(cloth);
        if !alive {
            env.free_voice(&mut bank, held.voice);
        }
        alive
    });
}

/// One step's rustle on its voice.
pub fn follow(
    env: &mut AudioEnvironment,
    bank: &mut SoundBank,
    held: RustleVoice,
    rustle: &ClothRustled,
) {
    if !rustle.playing {
        if let Some(voice) = env.voice_mut(held.voice) {
            voice.flags |= voice_flags::STOP;
        }
        return;
    }
    let ended = env
        .voice(held.voice)
        .is_some_and(|voice| voice.flags & SOUNDING == 0);
    if ended {
        env.retrigger_set(
            bank,
            held.voice,
            held.set,
            rustle.position,
            rustle.volume,
            0.5,
        );
    }
    if let Some(voice) = env.voice_mut(held.voice) {
        voice.position = rustle.position;
        voice.gain = rustle.volume;
        voice.pitch = rustle.pitch;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rustle(playing: bool) -> ClothRustled {
        ClothRustled {
            cloth: Entity::PLACEHOLDER,
            sound: "CltMov",
            position: Vec3::new(1.0, 2.0, 3.0),
            volume: 0.004,
            pitch: 0.502,
            playing,
        }
    }

    /// A fast step retriggers an ended voice on a variant at pitch 0.5 and
    /// then sets its place, volume and pitch; while it sounds it is only
    /// followed; a slow step sets its stop bit.
    #[test]
    fn a_rustle_retriggers_follows_and_stops() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let set = bank.insert_test_set(vec![1, 2]);
        let held = RustleVoice {
            voice: env.allocate_or_dummy(),
            set,
        };
        follow(&mut env, &mut bank, held, &rustle(true));
        let voice = *env.voice(held.voice).unwrap();
        assert_ne!(voice.flags & SOUNDING, 0, "the rustle did not start");
        assert_ne!(voice.buffer, 0);
        assert_eq!(voice.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!((voice.gain, voice.pitch), (0.004, 0.502));
        // Still sounding: followed, not retriggered.
        let mut louder = rustle(true);
        louder.volume = 0.01;
        follow(&mut env, &mut bank, held, &louder);
        let after = *env.voice(held.voice).unwrap();
        assert_eq!(after.flags, voice.flags);
        assert_eq!(after.gain, 0.01);
        follow(&mut env, &mut bank, held, &rustle(false));
        assert_ne!(env.voice(held.voice).unwrap().flags & voice_flags::STOP, 0);
    }
}
