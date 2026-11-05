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
//! world with everything it carries. The player's body leaves the scene
//! but stays the session's player, dead, so the session's hook can open
//! the end menu. After the body solver, a dead body whose root fell below
//! −50 m loses its health and, unless it is the player's, is taken out the
//! same way. Rules: `docs/formats/RSQ.md` (Falling to death).
//!
//! The fade runs on the game clock the body ticks keep (12 ms a tick), the
//! one timeline the original's frame clock and its simulation clock share:
//! the fade completes, and the game pauses, only after the tick that kills
//! the player has run. Leaving the session, the story's end-menu exit and
//! the story's respawn reset it ([`reset_death_fade`]), which gives the
//! game time back when the fade had paused it.

use bevy::audio::Volume;
use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank};
use exa_combat::condition::{Branch, DeathClock, PostureBranch, settle_death};
use exa_combat::fall::{FallDeath, FallReading, scream_voice};
use exa_combat::voice::{Answer, CharacterVoice};
use exa_combat::{Condition, Fighter, Roll};
use exa_physics::body::{
    BodySystems, BodyTicks, CharacterStage, CharacterTick, GAME_MS_PER_TICK, OutOfScene, Ragdoll,
    TickedCharacter, pairs,
};
use exa_ui::DeathFade;

use crate::player::PlayerBody;

pub struct FallPlugin;

impl Plugin for FallPlugin {
    fn build(&self, app: &mut App) {
        bridge_fall_types(app);
        app.add_systems(
            CharacterTick,
            fall_to_death
                .in_set(CharacterStage::Posture)
                .after(crate::balance::take_posture),
        )
        .add_systems(FixedUpdate, fall_out_of_world.after(BodySystems))
        .add_systems(Update, drive_death_fade);
    }
}

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_fall_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_copy::<exa_combat::voice::CharacterVoice>()
        .bridge_lend_resource::<exa_ui::fade::DeathFade>();
}

/// The death fade's world-bus gain and whether the game pauses, from how many
/// milliseconds of game time have passed since it started.
///
/// The share of the three seconds is taken in single precision. The bus gain
/// is one minus that share, held at 0 once the share reaches 1. The game
/// pauses only once the share is past 1, so the moment it completes does not
/// pause yet.
pub(crate) fn death_fade_hold(elapsed_ms: f64) -> (f32, bool) {
    let t = (elapsed_ms.max(0.0) * (1.0 / exa_ui::fade::FADE_MS)) as f32;
    let pause = t > 1.0;
    (1.0 - t.min(1.0), pause)
}

/// The game clock between the body ticks (ms): the last tick's own clock
/// (the counter has moved on past it) plus the share of the next tick the
/// fixed step has already gathered, `overstep` (0 to 1). The next tick runs
/// once this reaches its own clock, as the original's simulation ticks
/// catch up with its frame clock.
pub(crate) fn game_clock_ms(ticks: u64, overstep: f64) -> f64 {
    let step = GAME_MS_PER_TICK as f64;
    (ticks as f64 - 1.0).max(0.0) * step + overstep.clamp(0.0, 1.0) * step
}

/// While the death fade runs, the world bus follows it down, and the game
/// pauses once the fade has passed its three seconds (the original's first
/// state turning into its second, once).
fn drive_death_fade(
    mut fade: ResMut<DeathFade>,
    ticks: Option<Res<BodyTicks>>,
    fixed: Option<Res<Time<Fixed>>>,
    mut time: ResMut<Time<Virtual>>,
    mut audio: Option<ResMut<AudioEnvironment>>,
) {
    let overstep = fixed.as_ref().map_or(0.0, |f| f.overstep_fraction_f64());
    let now = game_clock_ms(ticks.map_or(0, |t| t.0), overstep);
    if fade.now != now {
        fade.now = now;
    }
    let Some(elapsed) = fade.elapsed() else {
        return;
    };
    let (gain, pause) = death_fade_hold(elapsed);
    if let Some(audio) = audio.as_mut() {
        audio.buses.world_fade = gain;
    }
    if pause && !fade.paused {
        fade.paused = true;
        time.pause();
        info!("the death fade is complete: the game pauses");
    }
}

/// The original's fade reset: a fade that had paused the game gives the
/// game time back, and the fade ends (its black layer goes). The world bus
/// is the transition's to restore.
pub fn reset_death_fade(fade: &mut DeathFade, time: Option<&mut Time<Virtual>>) {
    if fade.paused
        && let Some(time) = time
    {
        time.unpause();
    }
    if fade.started.is_some() || fade.paused {
        info!("the death fade is reset");
    }
    fade.started = None;
    fade.paused = false;
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
    mut combatants: Query<&mut exa_ai::combatant::Combatant>,
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
            // Line 2 is the voice entry's heavy groan (the original's voice
            // request of kind 2).
            let name = voice.set(2).unwrap_or_default().to_string();
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
            // The fade starts at this tick's own clock: the original
            // stamps its frame clock, which the tick has just reached.
            if let Some(fade) = fade.as_mut() {
                fade.started = Some(now as f64);
                fade.now = now as f64;
                fade.paused = false;
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
            // out. The player's behaviour reset first tells every other
            // character to forget them.
            body.clear_death_strikes();
            pairs::set_leg_layers(&mut body.parts, 0);
            if player.is_some() {
                let root = parent.map(ChildOf::parent);
                for mut combatant in &mut combatants {
                    combatant.forget(entity);
                    if let Some(root) = root {
                        combatant.forget(root);
                    }
                }
            }
            if player.is_some() {
                leave_scene(&mut commands, entity, parent);
            } else {
                take_out(&mut commands, entity, parent);
            }
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

/// Takes the player's fallen body out of the scene: the original removes
/// its node from the scene but the session keeps it as its player, so the
/// body stays, dead, out of the solver and the character ticks and hidden
/// with its root.
fn leave_scene(commands: &mut Commands, body: Entity, parent: Option<&ChildOf>) {
    commands
        .entity(body)
        .insert((OutOfScene, Visibility::Hidden));
    if let Some(parent) = parent {
        commands.entity(parent.parent()).insert(Visibility::Hidden);
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
