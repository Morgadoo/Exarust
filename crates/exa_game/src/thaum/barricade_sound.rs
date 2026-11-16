//! The force barricade's sounds, the hum and the impacts, and the one-shot
//! sets the powers play from the simulation step ([`ForceSounded`]: the
//! barricade's impacts, Force Strikes' end, Shock's charge and release,
//! the displacements' start and come-back, the cast failures).
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
//!   [`ForceSounded`].
//! - **Force Strikes' end** ([`crate::thaum::power::force::Strikes`]): the
//!   `thmfrc_brf` set at the caster's place, volume 0.3 and pitch
//!   `0.6 + 0.4·r` (`r` the engine generator's draw, the sum in double).
//! - **The end**: the destructor stops the hum.
//! - **A following voice** ([`ForceSounded::follow`]): Vanish and the Long
//!   Door take a voice at their start that follows the body's place every
//!   control tick; the come-back plays on it. Here the voice is kept by
//!   the body's entity and follows the body's standing point each frame
//!   until the body leaves.
//!
//! Evidence: the barricade's creator, entry, frame, body test and
//! destructor, read from `Exanima.exe`. [inf: the sphere's place is carried
//! into the world by the wall's frame; the original uses the node's matrix,
//! whose origin follows the rising wall's slid middle.]

use std::collections::HashMap;

use bevy::prelude::*;
use exa_audio::{VoiceHandle, voice_flags};

use super::blast_sound::BlastAudio;
use super::force::BARRICADE_HUM;
use super::live::LiveBarricade;
use super::torrent_sound::loop_offset;

/// Centimetres a metre.
const CM: f32 = 100.0;

/// A one-shot of a set a power played in the simulation step: the set,
/// its place (cm, in the world), volume and pitch, and the body whose
/// following voice it plays on (none for a free one-shot).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct ForceSounded {
    pub set: &'static str,
    pub at: Vec3,
    pub volume: f32,
    pub pitch: f32,
    pub follow: Option<Entity>,
}

impl ForceSounded {
    /// A free one-shot of `set` at `at` (cm).
    pub fn at(set: &'static str, at: Vec3, volume: f32, pitch: f32) -> Self {
        Self {
            set,
            at,
            volume,
            pitch,
            follow: None,
        }
    }
}

/// The loaded hum buffer (`None` inside when it failed to load) and sets,
/// and each wall's hum by its entity.
#[derive(Default)]
pub(crate) struct BarricadeVoices {
    hum: Option<Option<u32>>,
    sets: HashMap<&'static str, u32>,
    voices: HashMap<Entity, VoiceHandle>,
    /// The bodies' following voices.
    follows: HashMap<Entity, VoiceHandle>,
}

/// Each frame, after the walls' draw: the sets the steps sent play, a
/// new wall starts its hum, and each hum follows its wall's node at the
/// frame's volume and pitch; a wall that left the scene stops its hum.
pub(crate) fn sound_barricades(
    walls: Query<(Entity, &LiveBarricade)>,
    bodies: Query<&exa_physics::body::Ragdoll>,
    time: Res<Time<Fixed>>,
    mut impacts: MessageReader<ForceSounded>,
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
    let left: Vec<Entity> = state
        .follows
        .keys()
        .copied()
        .filter(|&e| !bodies.contains(e))
        .collect();
    for entity in left {
        if let Some(handle) = state.follows.remove(&entity) {
            env.free_handle(bank, handle);
        }
    }
    if !env.is_live() {
        impacts.clear();
        return;
    }
    for impact in impacts.read() {
        let set = *state
            .sets
            .entry(impact.set)
            .or_insert_with(|| bank.load_set(game, sources, impact.set));
        let voice = match impact.follow {
            Some(body) => {
                let handle = *state
                    .follows
                    .entry(body)
                    .or_insert_with(|| env.new_handle());
                env.play_set_attached(
                    bank,
                    set,
                    handle,
                    impact.at / CM,
                    impact.volume,
                    impact.pitch,
                );
                env.handle_voice(handle)
            }
            None => env.play_set(bank, set, impact.at / CM, impact.volume, impact.pitch, 0),
        };
        info!(
            "force sound: {} at {:?} cm, volume {:.3} pitch {:.3}, voice {voice:?}, follows {:?}",
            impact.set, impact.at, impact.volume, impact.pitch, impact.follow
        );
    }
    // The following voices keep to their bodies' places.
    for (&body, &handle) in &state.follows {
        if let Ok(doll) = bodies.get(body)
            && let Some(at) = standing_point(doll)
            && let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id))
        {
            voice.position = at;
        }
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

/// A body's place for its following voice (m): its root across the floor
/// at its lowest part's height. [inf: the original's place is the body's
/// frame origin, which stands on its anchor.]
pub(crate) fn standing_point(doll: &exa_physics::body::Ragdoll) -> Option<Vec3> {
    let root = Vec3::from(doll.parts.first()?.transform().translation);
    let lowest = doll
        .parts
        .iter()
        .map(|part| part.transform().translation.y)
        .fold(f32::INFINITY, f32::min);
    let ground = if lowest.is_finite() { lowest } else { root.y };
    Some(Vec3::new(root.x, ground, root.z))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::force::{DOME_SOUND, ForceKind, creator_spec};

    #[test]
    fn the_sounds_are_the_creators() {
        let sounds = creator_spec(ForceKind::Barricade).sounds;
        assert_eq!((sounds[0], sounds[1]), (DOME_SOUND, BARRICADE_HUM));
        assert_eq!((DOME_SOUND, BARRICADE_HUM), ("thmfrc_bi", "thmfrc_bc01"));
    }
}
