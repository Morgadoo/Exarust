//! Footsteps of moving characters, at a rate following their speed.
//!
//! Each step is a one-shot from a numbered set, so it goes through the
//! mixer's voice pool and source ranking.

use bevy::prelude::*;
use exa_assets::Game;
use exa_physics::CharacterBody;

use crate::environment::AudioEnvironment;
use crate::{SoundBank, SoundSet};

/// Steps at a walk, and heavier ones running.
pub(crate) const STEPS: SoundSet<'static> = SoundSet::new("stpmed", 6);
pub(crate) const HEAVY_STEPS: SoundSet<'static> = SoundSet::new("stphvy", 6);

#[derive(Component, Default)]
pub(crate) struct StepClock(f32);

#[allow(clippy::too_many_arguments)]
pub(crate) fn footsteps(
    mut commands: Commands,
    time: Res<Time>,
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut sets: Local<[Option<u32>; 2]>,
    mut walkers: Query<(
        Entity,
        &CharacterBody,
        &GlobalTransform,
        Option<&mut StepClock>,
    )>,
) {
    let Some(game) = game else { return };
    for (e, body, gt, clock) in &mut walkers {
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
