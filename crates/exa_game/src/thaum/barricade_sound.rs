//! The force barricade's sounds: the hum and the impacts.
//!
//! - **The sounds** (the creator, once a session): the `thmfrc_bi` set is
//!   the impact, the `thmfrc_bc01` buffer the hum.
//! - **The entry** ([`super::barricade::Barricade::add_to_world`]): when
//!   the hum's buffer loaded, a voice for it at the node: volume 0, pitch 1,
//!   looping, its start `length·0.8·r` into the buffer in double precision
//!   (`r` the process stream's next draw, kept on the wall as its
//!   `hum_roll`).
//! - **Each frame** ([`super::barricade::Barricade::frame`]): the voice
//!   follows the node, and the frame's hum words set its volume and pitch
//!   (standing 0.5 and 1; the rise's first and last quarters and the fall
//!   their curves; a wall told to end leaves them).
//! - **The impacts** ([`super::barricade::Barricade::shove`]): a limb in
//!   front at `0.3 + 0.2·r` volume and `0.6 + 0.4·r` pitch, a held weapon
//!   at `0.1 + 0.3·r` and `0.6 + 0.6·r`, at most every 400 ms, at the last
//!   struck sphere in front carried into the world. The tick sends them as
//!   [`BarricadeSounded`].
//! - **The end**: the destructor stops the hum.
//!
//! Evidence: the barricade's creator, entry, frame, body test and
//! destructor, read from `Exanima.exe`. [inf: the sphere's place is carried
//! into the world by the wall's frame; the original uses the node's matrix,
//! whose origin follows the rising wall's slid middle.]

use std::collections::HashMap;

use bevy::prelude::*;
use exa_audio::{VoiceHandle, voice_flags};

use super::blast_sound::BlastAudio;
use super::force::{BARRICADE_HUM, DOME_SOUND};
use super::live::LiveBarricade;
use super::torrent_sound::loop_offset;

/// Centimetres a metre.
const CM: f32 = 100.0;

/// An impact the wall's body test played: its place (cm, in the world),
/// volume and pitch.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct BarricadeSounded {
    pub at: Vec3,
    pub volume: f32,
    pub pitch: f32,
}

/// The loaded hum buffer and impact set (`None` inside when the buffer
/// failed to load), and each wall's hum by its entity.
#[derive(Default)]
pub(crate) struct BarricadeVoices {
    hum: Option<Option<u32>>,
    impact: Option<u32>,
    voices: HashMap<Entity, VoiceHandle>,
}

/// Each frame, after the walls' draw: the impacts the ticks sent play, a
/// new wall starts its hum, and each hum follows its wall's node at the
/// frame's volume and pitch; a wall that left the scene stops its hum.
pub(crate) fn sound_barricades(
    walls: Query<(Entity, &LiveBarricade)>,
    time: Res<Time<Fixed>>,
    mut impacts: MessageReader<BarricadeSounded>,
    mut audio: BlastAudio,
    mut state: Local<BarricadeVoices>,
) {
    let Some((game, bank, sources, env)) = audio.parts() else {
        impacts.clear();
        return;
    };
    let gone: Vec<Entity> = state
        .voices
        .keys()
        .copied()
        .filter(|&e| !walls.contains(e))
        .collect();
    for entity in gone {
        if let Some(handle) = state.voices.remove(&entity) {
            env.free_handle(bank, handle);
            info!("barricade {entity:?}: gone, its hum stopped");
        }
    }
    if !env.is_live() {
        impacts.clear();
        return;
    }
    for impact in impacts.read() {
        let set = *state
            .impact
            .get_or_insert_with(|| bank.load_set(game, sources, DOME_SOUND));
        let voice = env.play_set(bank, set, impact.at / CM, impact.volume, impact.pitch, 0);
        info!(
            "barricade: {DOME_SOUND} at {:?} cm, volume {:.3} pitch {:.3}, voice {voice:?}",
            impact.at, impact.volume, impact.pitch
        );
    }
    let alpha = time.overstep_fraction();
    for (entity, live) in &walls {
        // The frame's words, from a copy: the draw already ran the frame
        // on the wall itself.
        let mut wall = live.wall.clone();
        let Some(frame) = wall.frame(alpha) else {
            continue;
        };
        let node = wall.node_at / CM;
        let handle = match state.voices.get(&entity) {
            Some(&handle) => handle,
            None => {
                let Some(buffer) = *state
                    .hum
                    .get_or_insert_with(|| bank.load_buffer(game, sources, BARRICADE_HUM))
                else {
                    continue;
                };
                let handle = env.new_handle();
                state.voices.insert(entity, handle);
                if env.play_buffer_attached(buffer, handle, node, 0.0, 1.0, voice_flags::LOOPING)
                    && let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id))
                {
                    voice.offset = loop_offset(bank.buffer_secs(buffer), live.wall.hum_roll);
                    info!(
                        "barricade {entity:?}: {BARRICADE_HUM} loop at {:?} cm, start {:.3} s",
                        wall.node_at, voice.offset
                    );
                }
                handle
            }
        };
        if let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id)) {
            voice.position = node;
            if let Some((gain, pitch)) = frame.hum {
                voice.gain = gain;
                voice.pitch = pitch;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::force::{ForceKind, creator_spec};

    #[test]
    fn the_sounds_are_the_creators() {
        let sounds = creator_spec(ForceKind::Barricade).sounds;
        assert_eq!((sounds[0], sounds[1]), (DOME_SOUND, BARRICADE_HUM));
        assert_eq!((DOME_SOUND, BARRICADE_HUM), ("thmfrc_bi", "thmfrc_bc01"));
    }
}
