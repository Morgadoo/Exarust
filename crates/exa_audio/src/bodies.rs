//! The plain rigid bodies' own sounds: a prop's collision sound when a
//! contact strikes it, and the looping scrape and roll sources its stored
//! sound slots follow.
//!
//! - **Impacts.** Each tick, the impacts the pair stage dealt a body's
//!   parts ([`exa_physics::body::Touch::struck`]), then its level impacts
//!   ([`Ragdoll::level_impacts`]), in the original's order (body–body
//!   detection runs before body–level detection), go through the plain
//!   body's collision call ([`impact_sound`]) with the part's material
//!   collision sound set ([`BodySounds::impact`]). A sound plays as a
//!   one-shot at the part's mesh centre in the world, where the original's
//!   body keeps its centre. A part whose set has no shipped variant counts
//!   as having no sound: the original never gives such a body its impact
//!   class, which leaves it as silent as the parked clock does here.
//! - **Scrapes.** Each tick's scrape events ([`Ragdoll::scrape_events`])
//!   drive one attached loop per source: over the speed floor a source
//!   whose loop is not playing starts it at unit volume and pitch, and the
//!   loop then takes the event's volume, pitch and position; under the
//!   floor a playing loop is asked to stop.
//!
//! Character bodies (their limbs' impacts are `exa_combat`'s) and held
//! items (their striker and non-striker branches are too) carry no
//! [`BodySounds`] or are skipped. [inf] The original steps one generator
//! for every random draw; the draw here comes from this system's own
//! generator, as each of Exarust's sound paths keeps its own.

use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use exa_assets::Game;
use exa_physics::body::{
    BodyTicks, ImpactEvent, Pairs, Ragdoll, ScrapeEvent, SolverStages, Touch, impact_sound,
};
use exa_physics::prop::BodySounds;

use crate::{AudioEnvironment, SoundBank, VoiceHandle, voice_flags};

/// The impacts the pair stage dealt `body`'s parts this tick, in pair
/// order: each touch's sphere side first, then its touched side.
pub(crate) fn struck_parts(touches: &[Touch], body: Entity) -> Vec<(usize, f32)> {
    let mut struck = Vec::new();
    for touch in touches {
        for (side, impact) in [touch.a, touch.b].into_iter().zip(touch.struck) {
            if side.0 == body
                && let Some(impact) = impact
            {
                struck.push((side.1, impact));
            }
        }
    }
    struck
}

/// Rings each impact on `body` in turn (part index and strength), each
/// with its part's set: `sounded(part)` says whether the part has a
/// collision sound. Returns the sounds to play, with the part each is on.
pub(crate) fn ring_impacts(
    body: &mut Ragdoll,
    impacts: &[(usize, f32)],
    sounded: impl Fn(usize) -> bool,
    now_ms: u32,
    roll: &mut u32,
) -> Vec<(usize, ImpactEvent)> {
    let mut rung = Vec::new();
    for &(part, strength) in impacts {
        let Some(p) = body.parts.get_mut(part) else {
            continue;
        };
        if let Some(sound) = impact_sound(p, sounded(part), strength, now_ms, roll) {
            rung.push((part, sound));
        }
    }
    rung
}

/// Whether a scrape loop's voice plays: bound, started or asked to start,
/// and not asked to stop. [inf] The original tests its source's playing
/// bit; Exarust's voice is playing once started and until its stop.
fn loop_playing(env: &AudioEnvironment, handle: VoiceHandle) -> bool {
    env.handle_voice(handle)
        .and_then(|id| env.voice(id))
        .is_some_and(|v| {
            v.flags & (voice_flags::ACTIVE | voice_flags::RESTART) != 0
                && v.flags & voice_flags::STOP == 0
        })
}

/// One tick of a scrape source's loop, as the original's post-step drives
/// it: over the floor a silent loop starts at unit volume and pitch on the
/// source's set, and the playing loop takes the event's volume, pitch and
/// point; under the floor a playing loop is told to stop.
pub(crate) fn drive_scrape(
    env: &mut AudioEnvironment,
    bank: &mut SoundBank,
    handle: VoiceHandle,
    set: u32,
    event: &ScrapeEvent,
) {
    if event.playing {
        if !loop_playing(env, handle)
            && !env.play_loop_attached(bank, set, handle, event.point, 1.0, 1.0)
        {
            return;
        }
        if let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id)) {
            voice.gain = event.volume;
            voice.pitch = event.pitch;
            voice.position = event.point;
        }
    } else if loop_playing(env, handle)
        && let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id))
    {
        voice.flags |= voice_flags::STOP;
    }
}

/// Plays the plain bodies' impact sounds and drives their scrape loops,
/// once per body tick.
#[allow(clippy::too_many_arguments)]
pub(crate) fn body_sounds(
    game: Option<Res<Game>>,
    ticks: Option<Res<BodyTicks>>,
    stages: Option<Res<SolverStages>>,
    mut bodies: Query<(Entity, &mut Ragdoll, &BodySounds)>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut sets: Local<HashMap<String, u32>>,
    mut roll: Local<u32>,
    mut loops: Local<HashMap<Entity, Vec<VoiceHandle>>>,
) {
    let Some(game) = game else { return };
    let now = ticks.as_ref().map_or(0, |t| t.millis()) as u32;
    let touches = stages
        .as_ref()
        .and_then(|s| s.get::<Pairs>())
        .map_or(&[][..], Pairs::touches);
    let mut resolve = |name: &str, bank: &mut SoundBank| {
        *sets
            .entry(name.to_owned())
            .or_insert_with(|| bank.load_set(&game, &mut sources, name))
    };
    let mut seen = HashSet::new();
    for (entity, mut body, names) in &mut bodies {
        seen.insert(entity);
        if body.held_by.is_some() {
            continue;
        }
        let mut impacts = struck_parts(touches, entity);
        impacts.extend(body.level_impacts().iter().map(|i| (i.part, i.strength)));
        if !impacts.is_empty() {
            let part_sets: Vec<u32> = names
                .impact
                .iter()
                .map(|n| {
                    n.as_deref()
                        .map_or(SoundBank::EMPTY_SET, |n| resolve(n, &mut bank))
                })
                .collect();
            let set_of = |part: usize| part_sets.get(part).copied().unwrap_or(SoundBank::EMPTY_SET);
            let sounded = |part: usize| bank.set_count(set_of(part)) > 0;
            for (part, sound) in ring_impacts(&mut body, &impacts, sounded, now, &mut roll) {
                let p = &body.parts[part];
                let point = p.transform().transform_point3(p.pairing.mesh.centre);
                env.play_set(&mut bank, set_of(part), point, sound.volume, sound.pitch, 0);
            }
        }
        let events = body.scrape_events();
        if events.is_empty() {
            continue;
        }
        let handles = loops.entry(entity).or_default();
        for (i, event) in events.iter().enumerate() {
            let Some(name) = names.scrapes.get(i) else {
                continue;
            };
            if handles.len() <= i {
                handles.resize_with(i + 1, || VoiceHandle::NONE);
            }
            if handles[i] == VoiceHandle::NONE {
                if !event.playing {
                    continue;
                }
                handles[i] = env.new_handle();
            }
            let set = resolve(name, &mut bank);
            drive_scrape(&mut env, &mut bank, handles[i], set, event);
        }
    }
    // A body that left the world frees its loops, as its destructor does.
    loops.retain(|entity, handles| {
        let keep = seen.contains(entity);
        if !keep {
            for &h in handles.iter() {
                env.free_handle(&mut bank, h);
            }
        }
        keep
    });
}

#[cfg(test)]
#[path = "bodies_test.rs"]
mod tests;
