//! The force torrent's sounds: the charge loop and the stream loop.
//!
//! - **The buffers** (the creator, once a session): the `thmfrc_cu01`
//!   buffer is the charge loop, `thmfrc_trntdr` the stream loop.
//! - **The start** ([`super::torrent_live::new_torrent`]'s caller): when a
//!   buffer loaded, a voice for it at the effect's node: volume 0, pitch 1,
//!   looping, its start `length·0.8·r` into the buffer in double precision
//!   (`r` the process stream's next draw, [`loop_offset`]); the stream
//!   loop's pitch is then 6. The charge loop draws first.
//! - **Each tick** ([`crate::effects::torrent::Torrent::tick_paying`]): the
//!   charge voice follows the caster's hand at `0.4·(2 − c)·c` while the
//!   torrent charges, then falls 0.01 a tick and is freed below 0.02; the
//!   stream loop eases 6 % toward volume 0.5 and pitch 0.6 while it runs
//!   and keeps 0.96 of both while it ends; each stream point a surface
//!   meets raises its volume and pitch ([`crate::thaum::torrent_hit`]).
//!   The tick keeps those words in [`crate::effects::torrent::TorrentSounds`].
//! - **Each frame**: the stream loop follows the node.
//! - **The end**: the destructor frees both voices.
//!
//! The voices take the tick's words each frame (the original's tick writes
//! them on the voices). The node's place is the torrent's bounds centre.
//! [inf: the original's node takes the effect's bounds each frame; Exarust
//! has no node.]
//!
//! Evidence: the torrent's start, tick, frame and destructor, read from
//! `Exanima.exe`.

use std::collections::HashMap;

use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank, VoiceHandle, voice_flags};

use super::blast_sound::BlastAudio;
use super::force::TORRENT_SOUND;
use super::torrent_live::LiveTorrent;
use crate::effects::torrent::LOOP_PITCH;

/// Centimetres a metre.
const CM: f32 = 100.0;
/// The charge loop's buffer (the blast's).
pub const CHARGE_SOUND: &str = "thmfrc_cu01";
/// A loop's start, as a share of its buffer's length.
pub const LOOP_START_SHARE: f64 = 0.8;

/// A loop's start in seconds: the buffer's length times 0.8, then the
/// draw, in double precision, narrowed once.
pub fn loop_offset(length_secs: f32, roll: f64) -> f32 {
    (f64::from(length_secs) * LOOP_START_SHARE * roll) as f32
}

/// The draws the two loops' starts take, as the torrent's start makes
/// them: the charge loop's first.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TorrentRolls {
    pub charge: f64,
    pub stream: f64,
}

/// One torrent's voices: the charge loop (freed once it fades out) and the
/// stream loop.
#[derive(Debug, Clone, Copy)]
struct Voices {
    charge: Option<VoiceHandle>,
    stream: Option<VoiceHandle>,
}

/// The loaded buffers (`None` inside when one failed to load), and each
/// torrent's voices by its entity.
#[derive(Default)]
pub(crate) struct TorrentVoices {
    charge: Option<Option<u32>>,
    stream: Option<Option<u32>>,
    voices: HashMap<Entity, Voices>,
}

/// Starts a looping voice of `buffer` at `at` (m): volume 0, `pitch`, its
/// start [`loop_offset`] of the draw into the buffer.
fn start_loop(
    env: &mut AudioEnvironment,
    bank: &SoundBank,
    buffer: u32,
    at: Vec3,
    pitch: f32,
    roll: f64,
) -> VoiceHandle {
    let handle = env.new_handle();
    if env.play_buffer_attached(buffer, handle, at, 0.0, pitch, voice_flags::LOOPING)
        && let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id))
    {
        voice.offset = loop_offset(bank.buffer_secs(buffer), roll);
    }
    handle
}

/// Each frame, after the torrents' ticks: a new torrent starts its two
/// loops, the voices take the tick's volumes and pitch and follow the hand
/// and the node, the charge loop is freed once the tick stops it; a torrent
/// that left the scene frees its voices.
pub(crate) fn sound_torrents(
    torrents: Query<(Entity, &LiveTorrent)>,
    mut audio: BlastAudio,
    mut state: Local<TorrentVoices>,
) {
    let Some((game, bank, sources, env)) = audio.parts() else {
        return;
    };
    let gone: Vec<Entity> = state
        .voices
        .keys()
        .copied()
        .filter(|&e| !torrents.contains(e))
        .collect();
    for entity in gone {
        if let Some(voices) = state.voices.remove(&entity) {
            for handle in [voices.charge, voices.stream].into_iter().flatten() {
                env.finish_handle(bank, handle);
            }
            info!("torrent {entity:?}: gone, its voices freed");
        }
    }
    if !env.is_live() {
        return;
    }
    for (entity, live) in &torrents {
        let sounds = &live.torrent.sounds;
        let node = live.torrent.centre / CM;
        let hand = live.torrent.frame.origin / CM;
        let mut voices = match state.voices.get(&entity).copied() {
            Some(voices) => voices,
            None => {
                let charge = load(&mut state.charge, game, bank, sources, CHARGE_SOUND)
                    .filter(|_| sounds.charge_playing)
                    .map(|buffer| start_loop(env, bank, buffer, node, 1.0, live.rolls.charge));
                let stream = load(&mut state.stream, game, bank, sources, TORRENT_SOUND)
                    .map(|b| start_loop(env, bank, b, node, LOOP_PITCH, live.rolls.stream));
                info!(
                    "torrent {entity:?}: {CHARGE_SOUND} {} and {TORRENT_SOUND} {} loops at {:?} cm",
                    charge.is_some(),
                    stream.is_some(),
                    live.torrent.centre
                );
                Voices { charge, stream }
            }
        };
        if let Some(charge) = voices.charge {
            if sounds.charge_playing {
                if let Some(voice) = env.handle_voice(charge).and_then(|id| env.voice_mut(id)) {
                    voice.position = hand;
                    voice.gain = sounds.charge_volume;
                }
            } else {
                env.free_handle(bank, charge);
                voices.charge = None;
                debug!("torrent {entity:?}: the charge loop faded out");
            }
        }
        if let Some(stream) = voices.stream
            && let Some(voice) = env.handle_voice(stream).and_then(|id| env.voice_mut(id))
        {
            voice.position = node;
            voice.gain = sounds.loop_volume;
            voice.pitch = sounds.loop_pitch;
        }
        state.voices.insert(entity, voices);
    }
}

/// A buffer loaded once a session (`None` when it failed).
fn load(
    slot: &mut Option<Option<u32>>,
    game: &Game,
    bank: &mut SoundBank,
    sources: &mut Assets<AudioSource>,
    name: &str,
) -> Option<u32> {
    *slot.get_or_insert_with(|| bank.load_buffer(game, sources, name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::force::{ForceKind, creator_spec};

    #[test]
    fn the_loops_are_the_creators() {
        assert_eq!(
            creator_spec(ForceKind::Torrent).sounds,
            [CHARGE_SOUND, TORRENT_SOUND]
        );
        assert_eq!(LOOP_PITCH, 6.0);
    }

    #[test]
    fn a_loop_starts_a_share_of_its_length_in_double_precision() {
        assert_eq!(loop_offset(2.5, 0.5), (2.5f64 * 0.8 * 0.5) as f32);
        // In single precision 0.8 rounds differently: the double product is
        // the original's.
        let roll = 0.123_456_789_f64;
        assert_eq!(loop_offset(3.7, roll), (3.7f32 as f64 * 0.8 * roll) as f32);
        assert_eq!(loop_offset(0.0, 0.9), 0.0);
    }
}
