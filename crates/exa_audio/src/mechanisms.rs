//! The mechanisms' sounds (`exa_physics::mechanism_sound`): the doors',
//! levers' and push surfaces' restarted voices, their one-shots and the
//! lever's return tick loop, played as the original's steps play them.
//!
//! - **Voices.** One per body. A playing request finds the body's voice
//!   (allocating one the first time, or the shared dummy when the pool is
//!   full) and, when that voice is not playing, restarts it on a fresh
//!   variant of the set at the request's point and levels; a playing voice
//!   only takes the volume and pitch. A quiet request sets the volume to
//!   0, asks the voice to stop and frees it.
//! - **One-shots** draw the pitch's share first, then the volume's
//!   ([`exa_physics::door::Sound::levels`]), from the engine generator, and
//!   play a variant of the set; a hinged door's knock plays the body's own
//!   collision set when it has a variant.
//! - **The tick loop** starts `leverb_tck01` (the file, not a set) on a
//!   fresh voice at volume 0.1 and pitch 1, looping, follows the fade's
//!   volume and is freed at its end. [deviation] A start while a loop is
//!   held frees the old one first; the original overwrites its pointer,
//!   which its report order never lets happen.
//!
//! Every restart and one-shot is logged with its set, volume and pitch.
//! The voices of bodies that left the world are freed.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_assets::Game;
use exa_physics::mechanism_sound::{MechanismSounded, TICK_LOOP, TickCue};
use exa_physics::prop::BodySounds;

use crate::{AudioEnvironment, SoundBank, voice_flags};

/// The tick loop's volume and pitch at its start.
pub const TICK_LOOP_VOLUME: f32 = 0.1;
pub const TICK_LOOP_PITCH: f32 = 1.0;

/// What a voice request did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum VoiceStep {
    /// The voice was not playing and restarted on a variant.
    Restarted,
    /// The playing voice took the levels.
    Levelled,
    /// The voice was stopped and freed.
    Stopped,
    /// A quiet request with no voice held.
    Nothing,
}

/// One tick of a body's restarted voice, held in `held`.
pub(crate) fn drive_voice(
    env: &mut AudioEnvironment,
    bank: &mut SoundBank,
    held: &mut Option<u32>,
    set: u32,
    position: Vec3,
    levels: Option<(f32, f32)>,
) -> VoiceStep {
    let Some((volume, pitch)) = levels else {
        let Some(id) = held.take() else {
            return VoiceStep::Nothing;
        };
        if let Some(voice) = env.voice_mut(id) {
            voice.gain = 0.0;
            voice.flags |= voice_flags::STOP;
        }
        env.free_voice(bank, id);
        return VoiceStep::Stopped;
    };
    let id = *held.get_or_insert_with(|| env.allocate_or_dummy());
    let playing = env
        .voice(id)
        .is_some_and(|v| v.flags & voice_flags::ACTIVE != 0);
    if playing {
        if let Some(voice) = env.voice_mut(id) {
            voice.gain = volume;
            voice.pitch = pitch;
        }
        VoiceStep::Levelled
    } else {
        env.retrigger_set(bank, id, set, position, volume, pitch);
        VoiceStep::Restarted
    }
}

/// One cue of the lever's tick loop, held in `held`. `buffer` is the
/// loop's file, when the packs hold it.
pub(crate) fn drive_tick_loop(
    env: &mut AudioEnvironment,
    bank: &mut SoundBank,
    held: &mut Option<u32>,
    buffer: Option<u32>,
    position: Vec3,
    cue: TickCue,
) {
    match cue {
        TickCue::Start => {
            if let Some(old) = held.take() {
                env.free_voice(bank, old);
            }
            let Some(buffer) = buffer else {
                return;
            };
            let id = env.allocate_or_dummy();
            env.retrigger_buffer(id, buffer, position, TICK_LOOP_VOLUME, TICK_LOOP_PITCH);
            if let Some(voice) = env.voice_mut(id) {
                voice.flags |= voice_flags::LOOPING;
            }
            *held = Some(id);
        }
        TickCue::Gain(gain) => {
            if let Some(voice) = held.and_then(|id| env.voice_mut(id)) {
                voice.gain = gain;
            }
        }
        TickCue::Free => {
            if let Some(id) = held.take() {
                if let Some(voice) = env.voice_mut(id) {
                    voice.gain = 0.0;
                }
                env.free_voice(bank, id);
            }
        }
    }
}

/// The voices and tick loops the mechanisms hold, by body.
#[derive(Default)]
pub(crate) struct Held {
    voices: HashMap<Entity, Option<u32>>,
    loops: HashMap<Entity, Option<u32>>,
}

/// Plays the mechanisms' requests in the order their steps sent them.
#[allow(clippy::too_many_arguments)]
pub(crate) fn mechanism_sounds(
    game: Option<Res<Game>>,
    mut requests: MessageReader<MechanismSounded>,
    bodies: Query<&BodySounds>,
    alive: Query<()>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut sets: Local<HashMap<String, u32>>,
    mut tick_buffer: Local<Option<Option<u32>>>,
    mut held: Local<Held>,
) {
    let Some(game) = game else {
        requests.clear();
        return;
    };
    let mut resolve = |name: &str, bank: &mut SoundBank, sources: &mut Assets<AudioSource>| {
        *sets
            .entry(name.to_owned())
            .or_insert_with(|| bank.load_set(&game, sources, name))
    };
    for request in requests.read() {
        match request {
            MechanismSounded::Shot {
                set,
                position,
                sound,
                own_set_of,
            } => {
                let own = own_set_of
                    .and_then(|e| bodies.get(e).ok())
                    .and_then(|s| s.impact.first().cloned().flatten())
                    .map(|name| (resolve(&name, &mut bank, &mut sources), name))
                    .filter(|(id, _)| bank.set_count(*id) > 0);
                let (id, name) =
                    own.unwrap_or_else(|| (resolve(set, &mut bank, &mut sources), set.clone()));
                let (volume, pitch) = sound.levels(|| bank.rng().unit_f64());
                env.play_set(&mut bank, id, *position, volume, pitch, 0);
                info!(
                    "mechanism sound: {name} volume {volume:.4} pitch {pitch:.4} at ({:.2}, {:.2}, {:.2}){}",
                    position.x,
                    position.y,
                    position.z,
                    if bank.set_count(id) == 0 {
                        " (no variant)"
                    } else {
                        ""
                    }
                );
            }
            MechanismSounded::Voice {
                source,
                set,
                position,
                levels,
            } => {
                let slot = held.voices.entry(*source).or_default();
                if levels.is_none() && slot.is_none() {
                    continue;
                }
                let id = resolve(set, &mut bank, &mut sources);
                let step = drive_voice(&mut env, &mut bank, slot, id, *position, *levels);
                match (step, levels) {
                    (VoiceStep::Restarted, Some((volume, pitch))) => info!(
                        "mechanism voice: {set} restarts volume {volume:.4} pitch {pitch:.4} at ({:.2}, {:.2}, {:.2})",
                        position.x, position.y, position.z
                    ),
                    (VoiceStep::Levelled, Some((volume, pitch))) => {
                        debug!("mechanism voice: {set} volume {volume:.4} pitch {pitch:.4}")
                    }
                    (VoiceStep::Stopped, _) => info!("mechanism voice: {set} stops"),
                    _ => {}
                }
            }
            MechanismSounded::TickLoop {
                source,
                position,
                cue,
            } => {
                let buffer = *tick_buffer
                    .get_or_insert_with(|| bank.load_buffer(&game, &mut sources, TICK_LOOP));
                let slot = held.loops.entry(*source).or_default();
                drive_tick_loop(&mut env, &mut bank, slot, buffer, *position, *cue);
                match cue {
                    TickCue::Start => info!(
                        "mechanism loop: {TICK_LOOP} starts volume {TICK_LOOP_VOLUME} pitch {TICK_LOOP_PITCH} at ({:.2}, {:.2}, {:.2}){}",
                        position.x,
                        position.y,
                        position.z,
                        if buffer.is_none() { " (missing)" } else { "" }
                    ),
                    TickCue::Gain(gain) => debug!("mechanism loop: {TICK_LOOP} volume {gain:.4}"),
                    TickCue::Free => info!("mechanism loop: {TICK_LOOP} freed"),
                }
            }
        }
    }
    // A body that left the world frees what it held.
    let Held { voices, loops } = &mut *held;
    for map in [voices, loops] {
        map.retain(|entity, slot| {
            let keep = alive.contains(*entity);
            if !keep && let Some(id) = slot.take() {
                env.free_voice(&mut bank, id);
            }
            keep
        });
    }
}

#[cfg(test)]
#[path = "mechanisms_test.rs"]
mod tests;
