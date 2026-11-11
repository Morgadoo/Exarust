//! The force blast's and bolt's sounds: the charge loop and the release.
//!
//! - **The buffers** (the creators, once a session): both take the
//!   `thmfrc_cu01` buffer as the charge loop; the blast takes the
//!   `thmfrc_bb` set as its release, the bolt `thmfrc_blt`.
//! - **The start** ([`super::blast_live::start_blast`]'s caller): when the
//!   charge buffer loaded, a voice for it at the effect's node: volume 0,
//!   pitch 1, looping, its start `length·0.8·r` into the buffer (`r` the
//!   effect stream's next draw, [`charge_offset`]); and a handle for the
//!   release.
//! - **Each frame** ([`crate::effects::blast::Blast::frame`]): the charge
//!   voice follows the node at `0.4·min(charge·10/3, 1)` while the blast
//!   charges; once thrown it fades as `(0.9 − charge)·0.444` and is freed
//!   when the charge passes 0.9. The release voice follows the node at the
//!   setup's release volume.
//! - **The release** (the tick's throw): a variant of the release set on
//!   the release handle at the node, volume 0 until the frame levels it,
//!   pitch `0.9 + 0.2·r` ([`crate::effects::blast::BlastEvents::released`]).
//! - **The end**: the destructor frees both voices.
//!
//! The node's place is the blast's bounds centre. [inf: the original's
//! node takes the effect's bounds each frame; Exarust has no node.]
//!
//! Evidence: the blast's start, tick, frame and destructor, read from
//! `Exanima.exe`.

use std::collections::HashMap;

use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank, VoiceHandle, voice_flags};

use super::blast_live::LiveBlast;
use super::force::BLAST_SETUP;

/// Centimetres a metre.
const CM: f32 = 100.0;
/// The charge loop's start, as a share of its buffer's length.
pub const CHARGE_START_SHARE: f32 = 0.8;

/// The charge loop's start in seconds: the buffer's length times 0.8, then
/// the draw, in single precision as the start computes it.
pub fn charge_offset(length_secs: f32, roll: f32) -> f32 {
    length_secs * CHARGE_START_SHARE * roll
}

/// What a blast's sounds keep between its tick and the frame: the draw
/// the charge loop's start takes, and the release's pitch once thrown.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BlastSounds {
    pub charge_roll: f32,
    pub release_pitch: Option<f32>,
}

/// What the blast sounds take from the game (none in tests without it).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct BlastAudio<'w> {
    game: Option<Res<'w, Game>>,
    bank: Option<ResMut<'w, SoundBank>>,
    sources: Option<ResMut<'w, Assets<AudioSource>>>,
    env: Option<ResMut<'w, AudioEnvironment>>,
}

/// One blast's voices: the charge loop (freed once it fades out) and the
/// release, and whether the release has played.
#[derive(Debug, Clone, Copy)]
struct Voices {
    charge: Option<VoiceHandle>,
    release: VoiceHandle,
    released: bool,
}

/// The loaded buffers and sets (`None` inside when one failed to load),
/// and each blast's voices by its entity.
#[derive(Default)]
pub(crate) struct BlastVoices {
    charge: Option<Option<u32>>,
    releases: HashMap<&'static str, u32>,
    voices: HashMap<Entity, Voices>,
}

/// Each frame, after the blasts' ticks: a new blast starts its charge
/// loop, a thrown one plays its release once, and the voices follow the
/// frame's levels; a blast that left the scene frees its voices.
pub(crate) fn sound_blasts(
    blasts: Query<(Entity, &LiveBlast)>,
    time: Res<Time<Fixed>>,
    mut audio: BlastAudio,
    mut state: Local<BlastVoices>,
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
        .voices
        .keys()
        .copied()
        .filter(|&e| !blasts.contains(e))
        .collect();
    for entity in gone {
        if let Some(voices) = state.voices.remove(&entity) {
            if let Some(charge) = voices.charge {
                env.free_handle(bank, charge);
            }
            env.free_handle(bank, voices.release);
            info!("blast {entity:?}: gone, its voices freed");
        }
    }
    if !env.is_live() {
        return;
    }
    let alpha = time.overstep_fraction();
    for (entity, live) in &blasts {
        let at = live.blast.centre / CM;
        let voices = match state.voices.get(&entity).copied() {
            Some(voices) => voices,
            None => {
                let charge = state
                    .charge
                    .get_or_insert_with(|| {
                        bank.load_buffer(game, sources, BLAST_SETUP.charge_sound)
                    })
                    .map(|buffer| {
                        let handle = env.new_handle();
                        if env.play_buffer_attached(
                            buffer,
                            handle,
                            at,
                            0.0,
                            1.0,
                            voice_flags::LOOPING,
                        ) && let Some(voice) =
                            env.handle_voice(handle).and_then(|id| env.voice_mut(id))
                        {
                            voice.offset =
                                charge_offset(bank.buffer_secs(buffer), live.sounds.charge_roll);
                            info!(
                                "blast {entity:?}: {} loop at {:?} cm, start {:.3} s",
                                BLAST_SETUP.charge_sound, live.blast.centre, voice.offset
                            );
                        }
                        handle
                    });
                let voices = Voices {
                    charge,
                    release: env.new_handle(),
                    released: false,
                };
                state.voices.insert(entity, voices);
                voices
            }
        };
        let mut voices = voices;
        if let (false, Some(pitch)) = (voices.released, live.sounds.release_pitch) {
            voices.released = true;
            let name = live.blast.setup.release_sound;
            let set = *state
                .releases
                .entry(name)
                .or_insert_with(|| bank.load_set(game, sources, name));
            let played = env.play_set_attached(bank, set, voices.release, at, 0.0, pitch);
            info!(
                "blast {entity:?}: {name} at {:?} cm, pitch {pitch:.3}, played {played}",
                live.blast.centre
            );
        }
        let frame = live.blast.frame(alpha);
        if let Some(charge) = voices.charge {
            match frame.charge_volume {
                Some(volume) => {
                    if let Some(voice) = env.handle_voice(charge).and_then(|id| env.voice_mut(id)) {
                        voice.position = at;
                        voice.gain = volume;
                    }
                }
                None => {
                    env.free_handle(bank, charge);
                    voices.charge = None;
                    debug!("blast {entity:?}: the charge loop faded out");
                }
            }
        }
        if let Some(volume) = frame.release_volume
            && let Some(voice) = env
                .handle_voice(voices.release)
                .and_then(|id| env.voice_mut(id))
        {
            voice.position = at;
            voice.gain = volume;
        }
        state.voices.insert(entity, voices);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::force::BOLT_SETUP;

    #[test]
    fn the_sounds_are_the_creators() {
        assert_eq!(BLAST_SETUP.charge_sound, "thmfrc_cu01");
        assert_eq!(BLAST_SETUP.release_sound, "thmfrc_bb");
        assert_eq!(BOLT_SETUP.release_sound, "thmfrc_blt");
    }

    #[test]
    fn the_charge_loop_starts_a_share_of_its_length_in() {
        assert_eq!(charge_offset(2.5, 0.5), 2.5 * 0.8 * 0.5);
        assert_eq!(charge_offset(0.0, 0.9), 0.0);
    }
}
