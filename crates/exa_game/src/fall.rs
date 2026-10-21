//! Falling to death and the kill plane, as the original runs them
//! (`exa_combat::fall` holds the rules).
//!
//! In its character tick's posture stage, after the condition is written
//! and the falls branch has started its clip, every living body that takes
//! the falls branch runs its fall check: the
//! scream (a heavy groan, at the body), being lost over nothing, then the
//! player's pit sound with the start of its fade to black
//! (`exa_ui::DeathFade`) and its death 4 s after it was lost, or another
//! character's death 3 s after: dead, no health left, and taken out of the
//! world with everything it carries. After the body solver, a dead body
//! whose root fell below −50 m loses its health and, unless it is the
//! player's, is taken out the same way. Rules: `docs/formats/RSQ.md`
//! (Falling to death).

use bevy::audio::Volume;
use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::SoundBank;
use exa_combat::condition::{Branch, DeathClock, PostureBranch, StateFlags, settle_death};
use exa_combat::fall::{FallDeath, FallReading, scream_voice};
use exa_combat::{Condition, Fighter, Roll};
use exa_physics::body::{
    BodySystems, BodyTicks, CharacterStage, CharacterTick, Ragdoll, TickedCharacter,
};
use exa_ui::DeathFade;

use crate::player::PlayerBody;

pub struct FallPlugin;

impl Plugin for FallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            CharacterTick,
            fall_to_death
                .in_set(CharacterStage::Posture)
                .after(crate::balance::take_posture),
        )
        .add_systems(FixedUpdate, fall_out_of_world.after(BodySystems));
    }
}

/// The pit sound the player hears 1 s after it is lost, on the interface:
/// at this gain, not placed in the world.
pub const PIT_SOUND: &str = "pitdeath01";
const PIT_GAIN: f32 = 0.1;
/// The human voices the scream picks from (`vhm01`–`vhm09`, six heavy
/// groans each) until characters carry their own voice sets.
const VOICES: usize = 9;
const GROANS: usize = 6;

/// What a fall takes sounds from (none in tests without the game).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Sounds<'w> {
    game: Option<Res<'w, Game>>,
    bank: Option<ResMut<'w, SoundBank>>,
    sources: Option<ResMut<'w, Assets<AudioSource>>>,
}

impl Sounds<'_> {
    /// Plays `name` once at `gain` and `pitch`: at `at` in the world, or on
    /// the interface.
    fn play(
        &mut self,
        commands: &mut Commands,
        name: &str,
        gain: f32,
        pitch: f32,
        at: Option<Vec3>,
    ) {
        let (Some(game), Some(bank), Some(sources)) = (
            self.game.as_ref(),
            self.bank.as_mut(),
            self.sources.as_mut(),
        ) else {
            return;
        };
        let Some(clip) = bank.get(game, sources, name) else {
            return;
        };
        let settings = PlaybackSettings::DESPAWN
            .with_volume(Volume::Linear(gain))
            .with_speed(pitch);
        match at {
            Some(at) => commands.spawn((
                AudioPlayer(clip),
                settings.with_spatial(true),
                Transform::from_translation(at),
            )),
            None => commands.spawn((AudioPlayer(clip), settings)),
        };
    }

    /// A heavy groan of a human voice.
    fn groan(&mut self) -> String {
        match self.bank.as_mut() {
            Some(bank) => format!(
                "vhm{:02}_groanh{:02}",
                1 + bank.pick(VOICES),
                1 + bank.pick(GROANS)
            ),
            None => "vhm01_groanh01".into(),
        }
    }
}

/// The ticked living body's fall check while it takes the falls branch, in
/// its posture stage, on the game clock.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn fall_to_death(
    ticked: Res<TickedCharacter>,
    mut commands: Commands,
    ticks: Res<BodyTicks>,
    time: Option<Res<Time<Virtual>>>,
    mut fade: Option<ResMut<DeathFade>>,
    mut sounds: Sounds,
    mut roll: Local<Roll>,
    mut bodies: Query<(
        Entity,
        &PostureBranch,
        &mut FallDeath,
        &mut Condition,
        &mut StateFlags,
        &mut Fighter,
        &mut Ragdoll,
        Option<&PlayerBody>,
        Option<&ChildOf>,
    )>,
) {
    let now = ticks.millis();
    if let Ok((
        entity,
        branch,
        mut fall,
        mut condition,
        mut flags,
        mut fighter,
        mut body,
        player,
        parent,
    )) = bodies.get_mut(ticked.body)
    {
        let Some(drive) = body.drive.as_ref() else {
            return;
        };
        if branch.0 != Branch::Falling {
            return;
        }
        let Some(pelvis) = body.parts.iter().find(|p| p.parent.is_none()) else {
            return;
        };
        let balance = drive.balance();
        let feet: Vec<Option<f32>> = body.legs().iter().map(|l| l.ground).collect();
        let reading = FallReading {
            sinking: balance.slow_velocity().y,
            pelvis: pelvis.transform().translation.y,
            ground: balance.ground(),
            feet: &feet,
        };
        let step = fall.check(&reading, condition.is_dead(), player.is_some(), now);
        if step.scream {
            let (gain, pitch) = scream_voice(roll.next(), roll.next());
            let name = sounds.groan();
            let stats = &fighter.stats;
            sounds.play(
                &mut commands,
                &name,
                gain * stats.voice_volume,
                pitch * stats.voice_pitch,
                balance.chest(),
            );
            info!(
                "{entity:?} screams as it falls, tick {} ({name}, gain {gain:.3}, pitch {pitch:.3})",
                ticks.0
            );
        }
        if step.lost {
            info!(
                "{entity:?} is lost, tick {}: no ground within 25 m under its pelvis at {:.2} m",
                ticks.0, reading.pelvis
            );
        }
        if step.pit {
            sounds.play(&mut commands, PIT_SOUND, PIT_GAIN, 1.0, None);
            if let (Some(fade), Some(time)) = (fade.as_mut(), time.as_ref()) {
                fade.started = Some(time.elapsed_secs_f64());
            }
            info!(
                "{entity:?} hears the pit, tick {}; the screen fades",
                ticks.0
            );
        }
        if step.dies {
            let now_ms = now as u32;
            if let Some(settled) = settle_death(
                *condition,
                *flags,
                fall.stage.death_stage(),
                fighter.health.yellow,
                fighter.first_flag_red,
                now_ms,
            ) {
                *condition = settled.condition;
                *flags = settled.flags;
                commands.entity(entity).insert(DeathClock {
                    wake_at_ms: settled.wake_at_ms,
                    died_at_ms: now_ms,
                });
            }
            fighter.health.yellow = 0.0;
            fighter.health.red = 0.0;
            body.set_class_word(0);
            body.clear_death_strikes();
            take_out(&mut commands, entity, parent);
            info!(
                "{entity:?} falls to its death, tick {}, {} ms after it was lost",
                ticks.0,
                now - fall.since
            );
        }
    }
}

/// The kill plane, after the body solver: a dead body whose root fell
/// below −50 m loses its health and, but for the player's, leaves the
/// world; once.
#[allow(clippy::type_complexity)]
fn fall_out_of_world(
    mut commands: Commands,
    mut bodies: Query<(
        Entity,
        &Ragdoll,
        &mut FallDeath,
        &Condition,
        &mut Fighter,
        Option<&PlayerBody>,
        Option<&ChildOf>,
    )>,
) {
    for (entity, body, mut fall, condition, mut fighter, player, parent) in &mut bodies {
        if !condition.is_dead() || !body.fell_out() || !fall.fall_out() {
            continue;
        }
        fighter.health.yellow = 0.0;
        fighter.health.red = 0.0;
        if player.is_none() {
            take_out(&mut commands, entity, parent);
        }
        info!("{entity:?} fell out of the world");
    }
}

/// Takes a character out of the world: its root (the body model's parent,
/// which holds its controller) and everything under it.
fn take_out(commands: &mut Commands, body: Entity, parent: Option<&ChildOf>) {
    commands
        .entity(parent.map_or(body, ChildOf::parent))
        .despawn();
}

#[cfg(test)]
#[path = "fall_test.rs"]
mod tests;
