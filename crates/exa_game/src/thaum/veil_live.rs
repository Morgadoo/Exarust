//! The live constant displacement: what the power's start puts on the
//! caster's running body and what follows from it.
//!
//! - **The start** (the class's creator, then its start routine): the
//!   caster's physics nodes gain collision bit `0x4000`
//!   ([`super::displacement::DISPLACED_BIT`]); the body's hit veto is set
//!   to the power's phasing ([`exa_combat::veto::PhaseVeto`], a fresh
//!   streak), with the restore hook on the body; the tendril image hangs
//!   on the body (the body wisps, [`super::cast`]); and the
//!   `thmdsp_cns01` hum starts silent at pitch 1, looping, `(length·0.8)·r`
//!   seconds into its buffer (in double; `r` the engine generator's draw
//!   after the image's).
//! - **Each control tick** the hum moves to the body and rises 0.002 while
//!   it is under 0.12 ([`super::power::displacement::ConstantDisplacement`]);
//!   the power's end marks it fading and freed when silent (flags `0x300`),
//!   closes the shell and ends the tendril image (its strands frozen, its
//!   fade back to 0 and its fade-out started), so the wisps trail away.
//! - **A blow the veto lets through** ([`exa_combat::veto::BlowPhased`]):
//!   `thmdsp_phs` plays at the struck body at volume 0.6 and the drawn
//!   pitch; the striker and its descendants gain collision bits `0xC000`,
//!   so its parts and the displaced body's (both members of the `0x4000`
//!   channel, the striker exclusive) skip each other; a one-strand tendril
//!   hangs from the striker at the contact, lit at once and spending 50 of
//!   its budget a node ([`super::wisps::blow_wisps`]); and the striker is
//!   listed with the clock.
//! - **The restore hook** (every body step): a listed striker whose 500 ms
//!   have passed (`stamp + 500 < now`) leaves the list, the rest closing up
//!   behind it, ends the last tendril hanging from it and loses bits
//!   `0xC000`.
//!
//! The power never takes the veto, the displaced bit or the restore hook
//! away: they stay on the body after it ends ([`exa_combat::veto`]).
//!
//! In Exarust a held weapon is its own body, so its parts are the
//! striker's node and descendants; a fist's blade has no body of its own,
//! and the attacker's part nearest the contact stands for it [Exarust].
//! The original's list of phased strikers is shared by every displaced
//! body; each [`LiveVeil`] keeps its own. The body's root part's origin
//! stands for the places the hum and the phasing sound play at. The
//! phasing callback draws from the engine generator shared by the whole
//! game; Exarust's combat keeps its own sequence of it.
//!
//! Evidence: the constant displacement's creator and start, its control
//! handler, the phasing callback, its delayed tendril, the restore hook
//! and the collision bit walks, read from `Exanima.exe`.

use std::collections::HashMap;

use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank, VoiceHandle, voice_flags};
use exa_combat::veto::BlowPhased;
use exa_physics::body::{BodyTicks, Ragdoll};

use super::cast::{ActiveClass, PowerCaster};
use super::displacement::{
    CONSTANT_HUM, DISPLACED_BIT, PHASE_MS, PHASE_SOUND, PHASE_VOLUME, PHASED_BITS,
};
use super::live::LiveWisps;
use super::wisps::blow_wisps;

/// Centimetres a metre.
const CM: f32 = 100.0;
/// The hum's start, as a share of its buffer's length (times the draw).
pub const HUM_START_SHARE: f64 = 0.8;
/// The voice flags the power's end adds to its hum: freed when it ends,
/// fading out.
pub const HUM_END_FLAGS: u32 = voice_flags::AUTO_FREE | voice_flags::FADING;

/// A striker a phased blow let through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phased {
    /// The striker's body.
    pub body: Entity,
    /// Its part, when the striker is one part of a bigger body (a fist);
    /// `None`: every part of `body`.
    pub part: Option<usize>,
    /// The body clock at the pass (ms).
    pub at: u64,
}

/// A constantly displaced body's live state, on the caster.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct LiveVeil {
    /// The strikers let through, oldest first.
    pub phased: Vec<Phased>,
    /// The hum's start draw (the engine generator's).
    pub hum_roll: f32,
    /// The bodies whose displaced bit is set.
    pub displaced: bool,
}

/// A tendril a phased blow hung from its striker.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlowTendril {
    pub striker: Entity,
    pub part: Option<usize>,
    /// The order it was hung in (the last one is ended first).
    pub serial: u64,
}

/// The phasing sound a pass asks for: at the struck body (m), at its pitch.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct VeilSounded {
    pub at: Vec3,
    pub pitch: f32,
}

/// The hum's start in seconds: the buffer's length times 0.8, then the
/// draw, in double.
pub fn hum_offset(length_secs: f32, roll: f32) -> f32 {
    (f64::from(length_secs) * HUM_START_SHARE * f64::from(roll)) as f32
}

/// The restore hook's walk: the strikers whose 500 ms are over at `now`
/// leave the list in its order.
pub fn take_restored(list: &mut Vec<Phased>, now: u64) -> Vec<Phased> {
    let mut back = Vec::new();
    let mut i = 0;
    while i < list.len() {
        if list[i].at + PHASE_MS < now {
            back.push(list.remove(i));
        } else {
            i += 1;
        }
    }
    back
}

/// Adds or takes away `bits` on a body's parts (one, or all).
fn set_bits(body: &mut Ragdoll, part: Option<usize>, bits: u32, on: bool) {
    let parts: Vec<usize> = match part {
        Some(k) => vec![k],
        None => (0..body.parts.len()).collect(),
    };
    for k in parts {
        if let Some(layer) = body.layer(k) {
            body.set_layer(k, if on { layer | bits } else { layer & !bits });
        }
    }
}

/// The part of `body` whose origin is nearest `point` (m).
fn nearest_part(body: &Ragdoll, point: Vec3) -> Option<usize> {
    body.parts
        .iter()
        .enumerate()
        .map(|(k, p)| {
            (
                k,
                Vec3::from(p.transform().translation).distance_squared(point),
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(k, _)| k)
}

/// The start's collision bit: every physics node of a newly displaced body
/// gains `0x4000`.
pub(crate) fn displace_bodies(mut veils: Query<(Entity, &mut LiveVeil, &mut Ragdoll)>) {
    for (entity, mut veil, mut body) in &mut veils {
        if veil.displaced || body.parts.is_empty() {
            continue;
        }
        veil.displaced = true;
        set_bits(&mut body, None, DISPLACED_BIT, true);
        info!(
            "constant displacement: {entity:?}'s {} parts gain collision bit {DISPLACED_BIT:#x}",
            body.parts.len()
        );
    }
}

/// The restore hook, every body step: listed strikers whose 500 ms are
/// over end their last tendril and get their collisions back.
pub(crate) fn restore_phased(
    mut veils: Query<&mut LiveVeil>,
    mut bodies: Query<&mut Ragdoll>,
    mut tendrils: Query<(&BlowTendril, &mut LiveWisps)>,
    ticks: Option<Res<BodyTicks>>,
) {
    let now = ticks.as_deref().map_or(0, BodyTicks::millis);
    for mut veil in &mut veils {
        if veil.phased.is_empty() {
            continue;
        }
        for back in take_restored(&mut veil.phased, now) {
            // The last tendril hanging from the striker ends.
            if let Some((_, mut wisps)) = tendrils
                .iter_mut()
                .filter(|(t, w)| t.striker == back.body && t.part == back.part && !w.wisps.ended())
                .max_by_key(|(t, _)| t.serial)
            {
                wisps.wisps.end();
            }
            if let Ok(mut body) = bodies.get_mut(back.body) {
                set_bits(&mut body, back.part, PHASED_BITS, false);
            }
            info!(
                "constant displacement: striker {:?} (part {:?}) gets its collisions back at {now} ms",
                back.body, back.part
            );
        }
    }
}

/// The phasing callback's pass: the sound, the striker's bits, its
/// tendril and its place on the list.
#[allow(clippy::too_many_arguments)]
pub(crate) fn phase_blows(
    mut commands: Commands,
    mut blows: MessageReader<BlowPhased>,
    mut veils: Query<&mut LiveVeil>,
    mut bodies: Query<&mut Ragdoll>,
    mut sounds: MessageWriter<VeilSounded>,
    ticks: Option<Res<BodyTicks>>,
    mut serial: Local<u64>,
) {
    let now = ticks.as_deref().map_or(0, BodyTicks::millis);
    for blow in blows.read() {
        let at = bodies
            .get(blow.target)
            .ok()
            .and_then(|b| b.parts.first())
            .map_or(blow.point, |p| Vec3::from(p.transform().translation));
        sounds.write(VeilSounded {
            at,
            pitch: blow.pitch,
        });
        // The striker: a weapon's own body, or the attacker's part nearest
        // the contact for a fist's blade.
        let (body, part) = if bodies.contains(blow.striker) {
            (blow.striker, None)
        } else {
            let part = bodies
                .get(blow.attacker)
                .ok()
                .and_then(|b| nearest_part(b, blow.point));
            (blow.attacker, part)
        };
        let Ok(mut striker) = bodies.get_mut(body) else {
            continue;
        };
        set_bits(&mut striker, part, PHASED_BITS, true);
        let slot = part.unwrap_or(0);
        let local = striker
            .parts
            .get(slot)
            .map(|p| p.transform().inverse().transform_point3(blow.point) * CM);
        if let Some(local) = local {
            let place = |s: &super::wisps::Strand| {
                striker.parts.get(slot).map_or(s.local, |p| {
                    p.transform().transform_point3(s.local / CM) * CM
                })
            };
            let wisps = blow_wisps(local, 0, place);
            *serial += 1;
            commands.spawn((
                Name::new("_PhaseTendril"),
                LiveWisps {
                    wisps,
                    body,
                    parts: vec![slot],
                },
                BlowTendril {
                    striker: body,
                    part,
                    serial: *serial,
                },
            ));
        }
        if let Ok(mut veil) = veils.get_mut(blow.target) {
            veil.phased.push(Phased {
                body,
                part,
                at: now,
            });
        }
        info!(
            "constant displacement: a blow on {:?} by {:?} phases through (striker {body:?} part {part:?}, pitch {:.3}) at {now} ms",
            blow.target, blow.striker, blow.pitch
        );
    }
}

/// What the veil's sounds take from the game (none in tests without it).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct VeilAudio<'w> {
    game: Option<Res<'w, Game>>,
    bank: Option<ResMut<'w, SoundBank>>,
    sources: Option<ResMut<'w, Assets<AudioSource>>>,
    env: Option<ResMut<'w, AudioEnvironment>>,
}

/// The veil's loaded sounds and each humming body's voice.
#[derive(Default)]
pub(crate) struct VeilVoices {
    phase: Option<u32>,
    hum: Option<Option<u32>>,
    voices: HashMap<Entity, VoiceHandle>,
}

/// Each frame: the phasing sounds the step asked for, and each displaced
/// body's hum, started with its power, following its body at the power's
/// volume, and left to fade out and free itself when the power ends.
pub(crate) fn sound_veils(
    casters: Query<(Entity, &LiveVeil, &PowerCaster, Option<&Ragdoll>)>,
    mut cues: MessageReader<VeilSounded>,
    mut audio: VeilAudio,
    mut state: Local<VeilVoices>,
) {
    let (Some(game), Some(bank), Some(sources), Some(env)) = (
        audio.game.as_deref(),
        audio.bank.as_deref_mut(),
        audio.sources.as_deref_mut(),
        audio.env.as_deref_mut(),
    ) else {
        cues.clear();
        return;
    };
    let live = env.is_live();
    for cue in cues.read() {
        if !live {
            continue;
        }
        let set = *state
            .phase
            .get_or_insert_with(|| bank.load_set(game, sources, PHASE_SOUND));
        let voice = env.play_set(bank, set, cue.at, PHASE_VOLUME, cue.pitch, 0);
        info!(
            "constant displacement: {PHASE_SOUND} at {:?} m, pitch {:.3}, voice {voice:?}",
            cue.at, cue.pitch
        );
    }
    // A hum whose power has ended (or whose body is gone) fades and frees
    // itself.
    let running: HashMap<Entity, (f32, Vec3, f32)> = casters
        .iter()
        .filter_map(|(entity, veil, caster, body)| {
            let volume = caster.active.iter().find_map(|a| match &a.class {
                ActiveClass::Constant(p) => Some(p.hum_volume),
                _ => None,
            })?;
            let at = body
                .and_then(|b| b.parts.first())
                .map_or(Vec3::ZERO, |p| Vec3::from(p.transform().translation));
            Some((entity, (volume, at, veil.hum_roll)))
        })
        .collect();
    let ended: Vec<Entity> = state
        .voices
        .keys()
        .copied()
        .filter(|e| !running.contains_key(e))
        .collect();
    for entity in ended {
        if let Some(handle) = state.voices.remove(&entity)
            && let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id))
        {
            voice.flags |= HUM_END_FLAGS;
            info!("constant displacement {entity:?}: its hum fades out");
        }
    }
    if !live {
        return;
    }
    for (entity, (volume, at, roll)) in running {
        let handle = match state.voices.get(&entity) {
            Some(&handle) => handle,
            None => {
                let Some(buffer) = *state
                    .hum
                    .get_or_insert_with(|| bank.load_buffer(game, sources, CONSTANT_HUM))
                else {
                    continue;
                };
                let handle = env.new_handle();
                state.voices.insert(entity, handle);
                if env.play_buffer_attached(buffer, handle, at, 0.0, 1.0, voice_flags::LOOPING)
                    && let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id))
                {
                    voice.offset = hum_offset(bank.buffer_secs(buffer), roll);
                    info!(
                        "constant displacement {entity:?}: {CONSTANT_HUM} loop at {at:?} m, start {:.3} s",
                        voice.offset
                    );
                }
                handle
            }
        };
        if let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id)) {
            voice.position = at;
            voice.gain = volume;
        }
    }
}

/// The fixed-step data the veil touches, for the simulation's bridge.
pub(crate) fn bridge_veil_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_copy::<LiveVeil>()
        .bridge_copy::<BlowTendril>()
        .bridge_message::<VeilSounded>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hum_starts_a_share_of_its_length_in() {
        // The length times 0.8 first, then the draw, in double.
        let r = 0.123_456_78_f32;
        assert_eq!(
            hum_offset(3.3, r),
            (f64::from(3.3f32) * 0.8 * f64::from(r)) as f32
        );
        assert_eq!(CONSTANT_HUM, "thmdsp_cns01");
        assert_eq!(HUM_END_FLAGS, 0x300);
    }

    #[test]
    fn the_restore_takes_the_strikers_past_500_ms_in_order() {
        let e = |i: u32| Entity::from_raw_u32(i).unwrap();
        let mut list = vec![
            Phased {
                body: e(1),
                part: None,
                at: 100,
            },
            Phased {
                body: e(2),
                part: None,
                at: 400,
            },
            Phased {
                body: e(3),
                part: Some(2),
                at: 101,
            },
        ];
        // 600 is not past 100 + 500.
        assert!(take_restored(&mut list, 600).is_empty());
        let back = take_restored(&mut list, 602);
        assert_eq!(
            back.iter().map(|p| p.body).collect::<Vec<_>>(),
            [e(1), e(3)]
        );
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].body, e(2));
    }
}
