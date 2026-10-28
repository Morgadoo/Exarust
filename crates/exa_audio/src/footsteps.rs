//! Footsteps of moving characters, at a rate following their speed.
//!
//! Each step is a one-shot from a numbered set, so it goes through the
//! mixer's voice pool and source ranking.

use std::collections::HashMap;

use bevy::prelude::*;
use exa_assets::Game;
use exa_physics::CharacterBody;
use exa_physics::body::{Footstep, gait_step};

use crate::environment::AudioEnvironment;
use crate::{SoundBank, SoundSet};

/// Steps at a walk, and heavier ones running.
pub(crate) const STEPS: SoundSet<'static> = SoundSet::new("stpmed", 6);
pub(crate) const HEAVY_STEPS: SoundSet<'static> = SoundSet::new("stphvy", 6);

#[derive(Component, Default)]
pub(crate) struct StepClock(f32);

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn footsteps(
    mut commands: Commands,
    time: Res<Time>,
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut sets: Local<[Option<u32>; 2]>,
    mut races: Local<HashMap<&'static str, u32>>,
    mut plants: MessageReader<Footstep>,
    mut walkers: Query<(
        Entity,
        &CharacterBody,
        &GlobalTransform,
        Has<exa_physics::body::Ragdoll>,
        Option<&mut StepClock>,
    )>,
) {
    let Some(game) = game else {
        // The sounds a physical body's gait planted are still drained.
        plants.clear();
        return;
    };
    // A physical body's feet sound where its sampler planted them; the
    // gain and pitch are the original's rule with two engine draws.
    for plant in plants.read() {
        let Some(name) = plant.set else { continue };
        let set = match races.get(name) {
            Some(&set) => set,
            None => {
                let set = bank.load_set(&game, &mut sources, name);
                races.insert(name, set);
                set
            }
        };
        // The original draws the pitch's share, then the gain's.
        let pitch_roll = bank.random_unit();
        let gain_roll = bank.random_unit();
        let step = gait_step(plant.speed_cm, pitch_roll, gain_roll);
        env.play_set(&mut bank, set, plant.position, step.gain, step.pitch, 0);
    }
    for (e, body, gt, ragdoll, clock) in &mut walkers {
        if ragdoll {
            continue;
        }
        let Some(mut clock) = clock else {
            commands.entity(e).insert(StepClock::default());
            continue;
        };
        let speed = body.desired_velocity.length();
        if speed < 0.2 || !body.grounded {
            clock.0 = 0.0;
            continue;
        }
        clock.0 -= time.delta_secs();
        if clock.0 > 0.0 {
            continue;
        }
        // About two steps per metre.
        clock.0 = (0.75 / speed).clamp(0.25, 0.7);
        let heavy = speed > 2.5;
        let index = usize::from(heavy);
        let set = match sets[index] {
            Some(set) => set,
            None => {
                let descriptor = if heavy { HEAVY_STEPS } else { STEPS };
                let set = bank.load_set(&game, &mut sources, descriptor.prefix);
                sets[index] = Some(set);
                set
            }
        };
        env.play_set(&mut bank, set, gt.translation(), 0.6, 1.0, 0);
    }
}
