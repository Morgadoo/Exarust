//! Falling to death and the kill plane, as the original runs them
//! (`exa_combat::fall` holds the rules).
//!
//! In its character tick's posture stage, after the condition is written
//! and the falls branch has started its clip, every living body that takes
//! the falls branch runs its fall check: the
//! scream (the character's own heavy groan through its voice request, at the
//! body), being lost over nothing, then the
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
use exa_audio::{AudioEnvironment, SoundBank};
use exa_combat::condition::{Branch, DeathClock, PostureBranch, settle_death};
use exa_combat::fall::{FallDeath, FallReading, scream_voice};
use exa_combat::voice::{Answer, CharacterVoice};
use exa_combat::{Condition, Fighter, Roll};
use exa_physics::body::{
    BodySystems, BodyTicks, CharacterStage, CharacterTick, Ragdoll, TickedCharacter, pairs,
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

/// What a fall takes sounds from (none in tests without the game).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Sounds<'w> {
    game: Option<Res<'w, Game>>,
    bank: Option<ResMut<'w, SoundBank>>,
    sources: Option<ResMut<'w, Assets<AudioSource>>>,
    env: Option<ResMut<'w, AudioEnvironment>>,
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

    /// Loads (or finds) the numbered set `name`.
    fn load_set(&mut self, name: &str) -> u32 {
        let (Some(game), Some(bank), Some(sources)) = (
            self.game.as_ref(),
            self.bank.as_mut(),
            self.sources.as_mut(),
        ) else {
            return SoundBank::EMPTY_SET;
        };
        bank.load_set(game, sources, name)
    }

    /// Plays a loaded set at `at` with the voice request's gains.
    fn play_set(&mut self, set: u32, at: Option<Vec3>, gain: f32, pitch: f32) {
        let (Some(bank), Some(env), Some(at)) = (self.bank.as_mut(), self.env.as_mut(), at) else {
            return;
        };
        env.play_set(bank, set, at, gain, pitch, 0);
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
        &mut Fighter,
        &mut Ragdoll,
        Option<&PlayerBody>,
        Option<&ChildOf>,
        Option<&mut CharacterVoice>,
    )>,
    mut sets: Local<bevy::platform::collections::HashMap<String, u32>>,
) {
    let now = ticks.millis();
    if let Ok((
        entity,
        branch,
        mut fall,
        mut condition,
        mut fighter,
        mut body,
        player,
        parent,
        voice,
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
        if step.scream
            && let Some(mut voice) = voice
        {
            let (gain, pitch) = scream_voice(roll.next(), roll.next());
            let name = voice.groan_heavy.clone();
            let set = *sets
                .entry(name.clone())
                .or_insert_with(|| sounds.load_set(&name));
            let answer = voice.speak(2, set != SoundBank::EMPTY_SET, now as u32, gain, pitch);
            if let Answer::Played(spoken) = answer {
                sounds.play_set(set, balance.chest(), spoken.volume, spoken.pitch);
            }
            info!(
                "{entity:?} screams as it falls, tick {} ({name}, {answer:?})",
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
            fighter.health.yellow = 0.0;
            fighter.health.red = 0.0;
            if let Some(settled) = settle_death(
                *condition,
                fighter.stats.rules,
                fall.stage.death_stage(),
                fighter.health.red,
                fighter.first_flag_red,
                now_ms,
            ) {
                *condition = settled.condition;
                fighter.stats.rules = settled.rules;
                commands.entity(entity).insert(DeathClock {
                    wake_at_ms: settled.wake_at_ms,
                    died_at_ms: now_ms,
                });
            }
            // The death routine's lower-leg pass, then the class writer's
            // 0: the feet leave the living layer before the body is taken
            // out.
            body.clear_death_strikes();
            pairs::set_leg_layers(&mut body.parts, 0);
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
