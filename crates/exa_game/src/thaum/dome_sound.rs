//! The force domes' sounds: the cast, the hum, the hits and the Bubble's
//! break.
//!
//! - **The sets** (the creators, once a session): the Barrier and the
//!   Bubble take `thmfrc_bi` as the dome's hit set, the `thmfrc_bc02`
//!   buffer as its hum, and the shared `thmfrc_ss` set, which the cast
//!   plays; the Bubble also loads `thmfrc_brf`, its break.
//! - **The cast** ([`crate::effects::force_dome::Dome::cast`]): `thmfrc_ss`
//!   at the caster's x and z, raised to the higher of its height plus 10
//!   and the ground plus 100, at the effect's volume (0.2 Barrier, 0.3
//!   Bubble) and pitch `0.6 + 0.4·r`. When the hum buffer loaded, the cast
//!   then takes a voice for it at the node's place: volume 0, pitch 1,
//!   looping, its start `0.8·r` of the buffer's length into it (`r` the
//!   effect stream's next draw, the product in double).
//! - **The hum**: each frame the dome's frame writes the voice's volume and
//!   pitch ([`crate::effects::force_dome::DomeFrame::hum`]) and moves it: to
//!   the Bubble's place, and 80 cm ahead of the Barrier's drawn centre along
//!   its facing. The dome's teardown frees the voice.
//! - **The hits** ([`crate::effects::force_dome::Dome::contact`],
//!   [`crate::effects::force_dome::Barrier::contact`]): `thmfrc_bi` at the
//!   contact's volume and pitch, at the Bubble's place, and at the Barrier's
//!   shell point carried into the world.
//! - **The break** ([`crate::effects::force_dome::Bubble::tick`]):
//!   `thmfrc_brf` at the Bubble's place, volume 0.3 and pitch `0.6 + 0.2·r`.
//!   The Barrier has no break sound.
//!
//! The effects queue their sounds as [`DomeCue`]s; [`sound_domes`] plays
//! them once a frame and keeps each dome's hum voice on a handle, freed
//! when the dome leaves the scene.
//!
//! Evidence: the Barrier's and the Bubble's creators, the dome's cast,
//! frame and destructor, the Bubble's tick and both keep-outs, read from
//! `Exanima.exe`. [inf: the Barrier's hit sound stands at the last pushed
//! part's shell point; the original's place is a local the decompiler
//! loses, carried through the node's frame.]

use std::collections::HashMap;

use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank, VoiceHandle, voice_flags};

use super::barrier_live::LiveBarrier;
use super::bubble_live::LiveBubble;
use super::force::{BARRIER_HUM, BUBBLE_BREAK, DOME_SOUND, PLACE_SOUND};
use crate::effects::force_dome::Sound;

/// Centimetres a metre.
const CM: f32 = 100.0;
/// The hum's start, as a share of its buffer's length (times the draw).
pub const HUM_START_SHARE: f64 = 0.8;

/// Which of the dome's sets a cue plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomeSet {
    /// `thmfrc_ss`, at the cast.
    Cast,
    /// `thmfrc_bi`, at a contact.
    Hit,
    /// `thmfrc_brf`, the Bubble's break.
    Break,
}

impl DomeSet {
    /// The set's name prefix.
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Cast => PLACE_SOUND,
            Self::Hit => DOME_SOUND,
            Self::Break => BUBBLE_BREAK,
        }
    }
}

/// A sound a dome asks for: its set, place (cm), volume and pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomeCue {
    pub set: DomeSet,
    pub sound: Sound,
}

/// The hum's place (cm) and levels, as the dome's last frame wrote them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomeHum {
    pub at: Vec3,
    pub volume: f32,
    pub pitch: f32,
}

/// A dome's sounds: the cues it queued since the last frame, its hum and
/// the draw its hum's start takes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DomeSounds {
    pub cues: Vec<DomeCue>,
    /// The hum's place and levels; `None` until the first frame.
    pub hum: Option<DomeHum>,
    /// The cast's draw for the hum's start.
    pub hum_roll: f64,
}

impl DomeSounds {
    /// The cast's sounds: its `thmfrc_ss` cue, and the hum's start draw.
    pub fn cast(sound: Sound, hum_roll: f64) -> Self {
        Self {
            cues: vec![DomeCue {
                set: DomeSet::Cast,
                sound,
            }],
            hum: None,
            hum_roll,
        }
    }

    /// Queues a cue.
    pub fn push(&mut self, set: DomeSet, sound: Sound) {
        self.cues.push(DomeCue { set, sound });
    }
}

/// The hum's start in seconds: `0.8·r` of the buffer's length, in double.
pub fn hum_offset(length_secs: f32, roll: f64) -> f32 {
    (f64::from(length_secs) * HUM_START_SHARE * roll) as f32
}

/// What the dome sounds take from the game (none in tests without it).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct DomeAudio<'w> {
    game: Option<Res<'w, Game>>,
    bank: Option<ResMut<'w, SoundBank>>,
    sources: Option<ResMut<'w, Assets<AudioSource>>>,
    env: Option<ResMut<'w, AudioEnvironment>>,
}

/// The dome sets, loaded once, and the hum buffer (`None` inside when it
/// failed to load).
#[derive(Default)]
pub(crate) struct DomeSets {
    cast: Option<u32>,
    hit: Option<u32>,
    broke: Option<u32>,
    hum: Option<Option<u32>>,
}

/// Each dome's hum voice, by the dome's entity.
#[derive(Default)]
pub(crate) struct DomeVoices(HashMap<Entity, VoiceHandle>);

/// One dome's frame of sound: its cues played, its hum started on the
/// first frame and moved and levelled on every frame.
#[allow(clippy::too_many_arguments)]
fn sound_dome(
    entity: Entity,
    sounds: &mut DomeSounds,
    game: &Game,
    bank: &mut SoundBank,
    sources: &mut Assets<AudioSource>,
    env: &mut AudioEnvironment,
    sets: &mut DomeSets,
    voices: &mut DomeVoices,
) {
    for cue in sounds.cues.drain(..) {
        let slot = match cue.set {
            DomeSet::Cast => &mut sets.cast,
            DomeSet::Hit => &mut sets.hit,
            DomeSet::Break => &mut sets.broke,
        };
        let set = *slot.get_or_insert_with(|| bank.load_set(game, sources, cue.set.prefix()));
        let voice = env.play_set(
            bank,
            set,
            cue.sound.at / CM,
            cue.sound.volume,
            cue.sound.pitch,
            0,
        );
        info!(
            "dome {entity:?}: {} at {:?} cm, volume {:.3} pitch {:.3}, voice {voice:?}",
            cue.set.prefix(),
            cue.sound.at,
            cue.sound.volume,
            cue.sound.pitch
        );
    }
    let Some(hum) = sounds.hum else {
        return;
    };
    let Some(buffer) = *sets
        .hum
        .get_or_insert_with(|| bank.load_buffer(game, sources, BARRIER_HUM))
    else {
        return;
    };
    let handle = match voices.0.get(&entity) {
        Some(&handle) => handle,
        None => {
            let handle = env.new_handle();
            voices.0.insert(entity, handle);
            if env.play_buffer_attached(buffer, handle, hum.at / CM, 0.0, 1.0, voice_flags::LOOPING)
                && let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id))
            {
                voice.offset = hum_offset(bank.buffer_secs(buffer), sounds.hum_roll);
                info!(
                    "dome {entity:?}: {BARRIER_HUM} loop at {:?} cm, start {:.3} s",
                    hum.at, voice.offset
                );
            }
            handle
        }
    };
    if let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id)) {
        voice.position = hum.at / CM;
        voice.gain = hum.volume;
        voice.pitch = hum.pitch;
    }
}

/// Each frame, after the domes' frames: their queued sounds and their hums,
/// and the hums of domes that have left the scene freed.
pub(crate) fn sound_domes(
    mut barriers: Query<(Entity, &mut LiveBarrier)>,
    mut bubbles: Query<(Entity, &mut LiveBubble)>,
    mut audio: DomeAudio,
    mut sets: Local<DomeSets>,
    mut voices: Local<DomeVoices>,
) {
    let (Some(game), Some(bank), Some(sources), Some(env)) = (
        audio.game.as_deref(),
        audio.bank.as_deref_mut(),
        audio.sources.as_deref_mut(),
        audio.env.as_deref_mut(),
    ) else {
        return;
    };
    let gone: Vec<Entity> = voices
        .0
        .keys()
        .copied()
        .filter(|&e| !barriers.contains(e) && !bubbles.contains(e))
        .collect();
    for entity in gone {
        if let Some(handle) = voices.0.remove(&entity) {
            env.free_handle(bank, handle);
            info!("dome {entity:?}: gone, its hum freed");
        }
    }
    if !env.is_live() {
        return;
    }
    for (entity, mut live) in &mut barriers {
        let sounds = &mut live.sounds;
        sound_dome(
            entity,
            sounds,
            game,
            bank,
            sources,
            env,
            &mut sets,
            &mut voices,
        );
    }
    for (entity, mut live) in &mut bubbles {
        let sounds = &mut live.sounds;
        sound_dome(
            entity,
            sounds,
            game,
            bank,
            sources,
            env,
            &mut sets,
            &mut voices,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sets_are_the_creators() {
        assert_eq!(DomeSet::Cast.prefix(), "thmfrc_ss");
        assert_eq!(DomeSet::Hit.prefix(), "thmfrc_bi");
        assert_eq!(DomeSet::Break.prefix(), "thmfrc_brf");
        assert_eq!(BARRIER_HUM, "thmfrc_bc02");
    }

    #[test]
    fn the_hum_starts_a_share_of_its_length_in() {
        // 0.8·r of the buffer, the product in double.
        assert_eq!(hum_offset(2.5, 0.5), 1.0);
        let r = 0.123_456_789_f64;
        assert_eq!(hum_offset(3.3, r), (f64::from(3.3f32) * 0.8 * r) as f32);
        let sounds = DomeSounds::cast(
            Sound {
                at: Vec3::ONE,
                volume: 0.2,
                pitch: 0.8,
            },
            r,
        );
        assert_eq!(sounds.cues.len(), 1);
        assert_eq!(sounds.cues[0].set, DomeSet::Cast);
        assert_eq!(sounds.hum, None);
    }
}
