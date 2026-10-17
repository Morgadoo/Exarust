//! Footsteps of moving characters, at a rate following their speed.

use bevy::audio::Volume;
use bevy::prelude::*;
use exa_assets::Game;
use exa_physics::CharacterBody;

use crate::{SoundBank, SoundSet};

/// Steps at a walk, and heavier ones running.
pub(crate) const STEPS: SoundSet<'static> = SoundSet::new("stpmed", 6);
pub(crate) const HEAVY_STEPS: SoundSet<'static> = SoundSet::new("stphvy", 6);

#[derive(Component, Default)]
pub(crate) struct StepClock(f32);

pub(crate) fn footsteps(
    mut commands: Commands,
    time: Res<Time>,
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
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
        let name = bank.pick_from(if speed > 2.5 { HEAVY_STEPS } else { STEPS });
        if let Some(h) = bank.get(&game, &mut sources, &name) {
            commands.spawn((
                AudioPlayer(h),
                PlaybackSettings::DESPAWN
                    .with_spatial(true)
                    .with_volume(Volume::Linear(0.6)),
                Transform::from_translation(gt.translation()),
            ));
        }
    }
}
