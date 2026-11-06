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

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_assets::Game;
use exa_physics::body::{
    BodyTicks, ImpactEvent, Pairs, Ragdoll, ScrapeEvent, SolverStages, Touch, impact_sound,
};
use exa_physics::prop::BodySounds;

use crate::{AudioEnvironment, SoundBank, VoiceHandle, voice_flags};
use bevy::audio::AudioSource;

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

/// The frame's side of the bodies' sounds: the sets it resolved by name
/// and each body's scrape loops. The simulation decides what sounds and
/// hands the frame the plays ([`exa_physics::sim_app::FrameCommands::send`]),
/// which resolves the names to the sound bank's sets.
#[derive(Resource, Default)]
pub struct FrameBodySounds {
    sets: HashMap<String, u32>,
    loops: HashMap<Entity, Vec<VoiceHandle>>,
}

/// The set named `name` on the frame (loaded on its first use, kept).
fn frame_set(world: &mut World, name: &str) -> u32 {
    if let Some(&id) = world
        .get_resource::<FrameBodySounds>()
        .and_then(|f| f.sets.get(name))
    {
        return id;
    }
    let Some(game) = world.get_resource::<Game>().cloned() else {
        return SoundBank::EMPTY_SET;
    };
    let id = world.resource_scope(|world, mut bank: Mut<SoundBank>| {
        let mut sources = world.resource_mut::<Assets<AudioSource>>();
        bank.load_set(&game, &mut sources, name)
    });
    world
        .get_resource_or_init::<FrameBodySounds>()
        .sets
        .insert(name.to_owned(), id);
    id
}

/// Plays `rung` (sets by name, points and sounds) on the frame, after
/// resolving every set the body's parts name, in order.
fn play_rung(world: &mut World, names: &[Option<String>], rung: &[(usize, Vec3, ImpactEvent)]) {
    let sets: Vec<u32> = names
        .iter()
        .map(|n| {
            n.as_deref()
                .map_or(SoundBank::EMPTY_SET, |n| frame_set(world, n))
        })
        .collect();
    world.resource_scope(|world, mut bank: Mut<SoundBank>| {
        let mut env = world.resource_mut::<AudioEnvironment>();
        for &(part, point, sound) in rung {
            let set = sets.get(part).copied().unwrap_or(SoundBank::EMPTY_SET);
            env.play_set(&mut bank, set, point, sound.volume, sound.pitch, 0);
        }
    });
}

/// Drives `body`'s scrape loops on the frame: each `(index, set name,
/// event)` as the post-step drives its source.
fn drive_scrapes(world: &mut World, body: Entity, scrapes: &[(usize, String, ScrapeEvent)]) {
    for (i, name, event) in scrapes {
        let mut state = world.get_resource_or_init::<FrameBodySounds>();
        let handles = state.loops.entry(body).or_default();
        if handles.len() <= *i {
            handles.resize_with(i + 1, || VoiceHandle::NONE);
        }
        let mut handle = handles[*i];
        let fresh = handle == VoiceHandle::NONE;
        if fresh && !event.playing {
            continue;
        }
        let set = frame_set(world, name);
        world.resource_scope(|world, mut bank: Mut<SoundBank>| {
            let mut env = world.resource_mut::<AudioEnvironment>();
            if fresh {
                handle = env.new_handle();
            }
            drive_scrape(&mut env, &mut bank, handle, set, event);
        });
        if fresh && let Some(mut state) = world.get_resource_mut::<FrameBodySounds>() {
            state.loops.entry(body).or_default()[*i] = handle;
        }
    }
}

/// Frees the loops of bodies that left the world, as their destructor does.
fn free_loops(world: &mut World, gone: &[Entity]) {
    let freed: Vec<VoiceHandle> = world
        .get_resource_mut::<FrameBodySounds>()
        .map(|mut s| {
            gone.iter()
                .filter_map(|e| s.loops.remove(e))
                .flatten()
                .collect()
        })
        .unwrap_or_default();
    world.resource_scope(|world, mut bank: Mut<SoundBank>| {
        let mut env = world.resource_mut::<AudioEnvironment>();
        for h in freed {
            env.free_handle(&mut bank, h);
        }
    });
}

/// Rings the plain bodies' impacts and runs their scrape sources, once per
/// body tick, as the original's post-step does; the frame plays what rang
/// and drives the loops. Whether a part's set has a variant is read from
/// the sound pack (the frame's bank is the frame's).
#[allow(clippy::too_many_arguments)]
pub(crate) fn body_sounds(
    game: Option<Res<Game>>,
    ticks: Option<Res<BodyTicks>>,
    stages: Option<Res<SolverStages>>,
    mut bodies: Query<(Entity, &mut Ragdoll, &BodySounds)>,
    mut frame: ResMut<exa_physics::sim_app::FrameCommands>,
    mut sizes: Local<HashMap<String, usize>>,
    mut roll: Local<u32>,
    mut sounding: Local<bevy::ecs::entity::EntityHashSet>,
) {
    let Some(game) = game else { return };
    let now = ticks.as_ref().map_or(0, |t| t.millis()) as u32;
    let touches = stages
        .as_ref()
        .and_then(|s| s.get::<Pairs>())
        .map_or(&[][..], Pairs::touches);
    let mut size = |name: &str| {
        *sizes
            .entry(name.to_owned())
            .or_insert_with(|| SoundBank::set_size(&game, name))
    };
    for (entity, mut body, names) in &mut bodies {
        if body.held_by.is_some() {
            continue;
        }
        let mut impacts = struck_parts(touches, entity);
        impacts.extend(body.level_impacts().iter().map(|i| (i.part, i.strength)));
        if !impacts.is_empty() {
            let sounded: Vec<bool> = names
                .impact
                .iter()
                .map(|n| n.as_deref().is_some_and(|n| size(n) > 0))
                .collect();
            let rung: Vec<(usize, Vec3, ImpactEvent)> = ring_impacts(
                &mut body,
                &impacts,
                |part| sounded.get(part).copied().unwrap_or(false),
                now,
                &mut roll,
            )
            .into_iter()
            .map(|(part, sound)| {
                let p = &body.parts[part];
                (
                    part,
                    p.transform().transform_point3(p.pairing.mesh.centre),
                    sound,
                )
            })
            .collect();
            let part_names = names.impact.clone();
            frame.send(move |world| play_rung(world, &part_names, &rung));
        }
        let events = body.scrape_events();
        if events.is_empty() {
            continue;
        }
        let scrapes: Vec<(usize, String, ScrapeEvent)> = events
            .iter()
            .enumerate()
            .filter_map(|(i, e)| Some((i, names.scrapes.get(i)?.clone(), *e)))
            .collect();
        sounding.insert(entity);
        frame.send(move |world| drive_scrapes(world, entity, &scrapes));
    }
    // A body that left the world frees its loops, as its destructor does.
    let gone: Vec<Entity> = sounding
        .iter()
        .copied()
        .filter(|e| !bodies.contains(*e))
        .collect();
    if !gone.is_empty() {
        for e in &gone {
            sounding.remove(e);
        }
        frame.send(move |world| free_loops(world, &gone));
    }
}

#[cfg(test)]
#[path = "bodies_test.rs"]
mod tests;
