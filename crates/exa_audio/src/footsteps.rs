//! Footsteps of moving characters, at a rate following their speed.
//!
//! Each step is a one-shot from a numbered set, so it goes through the
//! mixer's voice pool and source ranking.

use std::collections::HashMap;

use bevy::prelude::*;
use exa_assets::Game;
use exa_combat::Fighter;
use exa_physics::CharacterBody;
use exa_physics::body::{
    BodyTicks, FootCue, FootSounded, Footstep, anchor_rustle, gait_step, release_step,
    release_step_clock, rustle_clock,
};

use crate::environment::AudioEnvironment;
use crate::sounds::foley_set;
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
        let step = gait_step(plant.speed_cm, plant.volume, pitch_roll, gain_roll);
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

/// Each body's two foot sound clocks (ms): when its rustle and its
/// footstep may next play.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FootClocks {
    pub rustle: u32,
    pub step: u32,
}

/// Plays the foot sounds the balance's footwork cued
/// ([`exa_physics::body::FootSounded`]), as its sound pass does, the
/// draws, the play's variant pick and the clock's jitter all from the
/// engine's generator (the sound bank's), in the original's order:
///
/// - **Rustle** (a step started), while its clock is behind the game clock:
///   the volume's draw, the pitch's draw ([`anchor_rustle`] with the
///   character's foley base), the foley set played at the standing place at
///   the volume times the sound gain (1), then its clock 400 ms plus the
///   jitter ([`rustle_clock`]).
/// - **Footstep** (a step ended), while its clock is behind: the pitch's and
///   the gain's draws ([`release_step`] at the body's step loudness and the
///   character's footstep volume), the race's step set played at the body
///   root's place; the wet step ([`wet_release_step`]) would follow over
///   water, but Exarust's levels carry no water surface, so it never does;
///   then its clock 250 ms plus the jitter ([`release_step_clock`]).
///
/// [inf] The original plays the footstep on one of two alternating held
/// voices; it is a one-shot here. The walking step's own write of the step
/// clock is not reproduced (the walk does not run the footwork). A body
/// without a fighter (its armour and race sets) sounds nothing.
#[allow(clippy::too_many_arguments)]
pub(crate) fn foot_sounds(
    game: Option<Res<Game>>,
    mut cues: MessageReader<FootSounded>,
    fighters: Query<&Fighter>,
    ticks: Option<Res<BodyTicks>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut clocks: Local<HashMap<Entity, FootClocks>>,
    mut sets: Local<HashMap<&'static str, u32>>,
) {
    let Some(game) = game else {
        cues.clear();
        return;
    };
    let now = ticks.as_ref().map_or(0, |t| t.millis()) as u32;
    for sounded in cues.read() {
        let Ok(fighter) = fighters.get(sounded.body) else {
            continue;
        };
        let clock = clocks.entry(sounded.body).or_default();
        match sounded.cue {
            FootCue::StepStart {
                leg,
                gap_sq_cm,
                attacking,
            } => {
                if clock.rustle >= now {
                    continue;
                }
                let foley = exa_combat::sounds::foley(&fighter.armour, false, false, false);
                let g = bank.random_unit();
                let p = bank.random_unit();
                // The rustle's own clock is stepped after the play.
                let mut scratch = 0;
                let r = anchor_rustle(foley.base, gap_sq_cm, attacking, g, p, now, &mut scratch);
                if let Some(set) = foley_set(foley.kind) {
                    let id = *sets
                        .entry(set.prefix)
                        .or_insert_with(|| bank.load_set(&game, &mut sources, set.prefix));
                    env.play_set(
                        &mut bank,
                        id,
                        sounded.standing,
                        r.volume * SOUND_GAIN,
                        r.pitch,
                        0,
                    );
                    debug!(
                        "foot rustle: {:?} leg {leg} {} vol {:.4} pitch {:.3}",
                        sounded.body, set.prefix, r.volume, r.pitch
                    );
                }
                clock.rustle = rustle_clock(now, &mut bank.rng().0);
            }
            FootCue::StepEnd { leg, loudness, .. } => {
                if now <= clock.step {
                    continue;
                }
                let Some(name) = fighter.stats.footsteps.name() else {
                    continue;
                };
                let p = bank.random_unit();
                let g = bank.random_unit();
                let step = release_step(loudness, fighter.stats.footstep_volume, p, g);
                let id = *sets
                    .entry(name)
                    .or_insert_with(|| bank.load_set(&game, &mut sources, name));
                env.play_set(&mut bank, id, sounded.hip, step.gain, step.pitch, 0);
                debug!(
                    "foot step: {:?} leg {leg} {name} loudness {loudness:.3} gain {:.4} pitch {:.3}",
                    sounded.body, step.gain, step.pitch
                );
                clock.step = release_step_clock(now, &mut bank.rng().0);
            }
        }
    }
}

/// The character's sound gain, 1 from the constructor.
const SOUND_GAIN: f32 = 1.0;
