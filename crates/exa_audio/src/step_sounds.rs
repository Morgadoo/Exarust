//! Sounds a fixed step decides on and the frame plays. The sound bank, the
//! audio environment and the clips are the frame's: a step hands the frame
//! a play by name (`exa_physics::sim_app::FrameCommands::send`), which these
//! helpers resolve on the frame's world, in the order the step queued them.
//! A step that needs to know whether a set has a variant reads it from the
//! sound pack ([`crate::SoundBank::set_size`]).

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_assets::Game;

use crate::{AudioEnvironment, SoundBank};

/// The sets the frame loaded once by name for the steps' plays.
#[derive(Resource, Default)]
pub struct StepSoundSets(HashMap<String, u32>);

/// The set `name` on the frame: loaded on every call (`once` false, a
/// fresh reference each time, as a play that loads its set does), or once
/// and kept.
fn set_named(world: &mut World, name: &str, once: bool) -> u32 {
    if once
        && let Some(&id) = world
            .get_resource::<StepSoundSets>()
            .and_then(|s| s.0.get(name))
    {
        return id;
    }
    let Some(game) = world.get_resource::<Game>().cloned() else {
        return SoundBank::EMPTY_SET;
    };
    if !world.contains_resource::<SoundBank>() || !world.contains_resource::<Assets<AudioSource>>()
    {
        return SoundBank::EMPTY_SET;
    }
    let id = world.resource_scope(|world, mut bank: Mut<SoundBank>| {
        let mut sources = world.resource_mut::<Assets<AudioSource>>();
        bank.load_set(&game, &mut sources, name)
    });
    if once {
        world
            .get_resource_or_init::<StepSoundSets>()
            .0
            .insert(name.to_owned(), id);
    }
    id
}

/// Plays the set `name` at `at` with `gain` and `pitch` through the audio
/// environment (`once`: the set is loaded once and kept).
pub fn play_set_named(world: &mut World, name: &str, once: bool, at: Vec3, gain: f32, pitch: f32) {
    let set = set_named(world, name, once);
    if !world.contains_resource::<AudioEnvironment>() || !world.contains_resource::<SoundBank>() {
        return;
    }
    world.resource_scope(|world, mut bank: Mut<SoundBank>| {
        world
            .resource_mut::<AudioEnvironment>()
            .play_set(&mut bank, set, at, gain, pitch, 0);
    });
}

/// Plays the clip `name` once at `gain` and `pitch`: at `at` in the world,
/// or on the interface.
pub fn play_clip_named(world: &mut World, name: &str, at: Option<Vec3>, gain: f32, pitch: f32) {
    let Some(game) = world.get_resource::<Game>().cloned() else {
        return;
    };
    if !world.contains_resource::<SoundBank>() || !world.contains_resource::<Assets<AudioSource>>()
    {
        return;
    }
    let clip = world.resource_scope(|world, mut bank: Mut<SoundBank>| {
        let mut sources = world.resource_mut::<Assets<AudioSource>>();
        bank.get(&game, &mut sources, name)
    });
    let Some(clip) = clip else { return };
    let settings = PlaybackSettings::DESPAWN
        .with_volume(Volume::Linear(gain))
        .with_speed(pitch);
    match at {
        Some(at) => world.spawn((
            AudioPlayer(clip),
            settings.with_spatial(true),
            Transform::from_translation(at),
        )),
        None => world.spawn((AudioPlayer(clip), settings)),
    };
}
