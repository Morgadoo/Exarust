//! A running throw: the body under the cursor it may take, what it reads of
//! that body each tick and the velocity it puts on it
//! (`docs/THAUMATURGY.md`, Force).
//!
//! **The pick.** The navigation unit's ground pick from the cursor
//! (`super::cast_pick`), its accept test taking the nodes with an object,
//! meets the nearest node: the level, a prop's stand-in or a character's
//! limb. A body met can be thrown when it is a dead character, or a prop
//! whose mobility is at least 0.01 (compared in double), and nobody holds
//! it; the level met throws nothing. A throw a kinesis starts
//! takes the body the kinesis held instead, and its lift aims at the body's
//! centre and spends nothing a tick: the kinesis's start lifts it without
//! storing the cast's cost.
//!
//! **The sounds.** A throw from the cursor plays `thmfrc_knsst` at the body
//! at volume 0.3 and pitch `0.8 + 0.4·roll`; every lift plays
//! `thmfrc_throw` there at 0.6 and `1 + 0.2·roll` (each roll in double,
//! drawn before the variant), a one-shot that frees itself. A throw that
//! ends before its drop fades that voice.
//!
//! **The tick.** Before each body tick the throw reads its body's centre
//! (cm), its mobility as the mass and the caster's aim point: the cursor's
//! point for the player, else the caster's root. A body gone ends the
//! throw. After the tick its velocity moves the body's first part, each
//! particle by its origin weight, adding a fifth of the part's own motion
//! backwards while it is steered, and the body wakes.
//!
//! Evidence: the original's throw from the cursor pick, its pick filter,
//! its start from a kinesis and its tick, read in the instructions.
//! [inf: the original holds the body's single node: its centre, its
//! particle weights and its mobility. Exarust takes the first part's own
//! origin, origin weights and inverse mass, the prop's whole rigid body or
//! the character's root part. The item node's two refusal flags have no
//! Exarust counterpart and are not read. The caster's aim point is the
//! controller's; Exarust takes the cursor's ground point.]
//! [inf: the original lifts a handed throw at its cast and plays the pick
//! sound there; Exarust does both on the throw's first tick. A faded voice
//! that has already freed itself is left alone.]
//! Each tick it coasts after the shell drops it stamps the body with the
//! caster for 2 s ([`super::user_stamp`]).
//! The throw's shell and the held environment are `super::hold_live`'s.

use bevy::prelude::*;

use super::user_stamp::UserStamp;
use bevy::audio::AudioSource;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank, voice_flags};
use exa_combat::Condition;
use exa_physics::body::Ragdoll;
use exa_physics::prop::PropBody;

use super::cast::{ActiveClass, PowerCaster, TICK_MS};
use super::power::ENDING;
use super::power::force::{
    THROW_LIFT_SOUND, THROW_LIFT_VOLUME, THROW_PICK_SOUND, THROW_PICK_VOLUME, ThrowView,
    kinesis_spin, throw_lift_pitch, throw_pick_pitch,
};
use crate::hand::{Hand, Holding};
use crate::player::{CursorGround, PlayerBody};

const CM: f32 = 100.0;
/// The least mobility a thrown prop has.
const LEAST_MOBILITY: f64 = 0.01;

/// The throwable body under the player's cursor.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ThrowPick(pub Option<Entity>);

/// What the cursor's ray meets first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Met {
    Prop { mobility: f32 },
    Character { dead: bool },
}

/// Whether the body met can be thrown; `held` when someone holds it.
pub fn throwable(met: Met, held: bool) -> bool {
    let kind = match met {
        Met::Character { dead } => dead,
        Met::Prop { mobility } => f64::from(mobility) >= LEAST_MOBILITY,
    };
    kind && !held
}

/// Each frame, the throwable body under the player's cursor: the ground
/// pick from the cursor ([`super::cast_pick::CastPicker::ground`]), its
/// accept test taking the nodes with an object, and the node it meets
/// taken when it is a body the throw can take ([`throwable`]).
#[allow(clippy::type_complexity)]
pub(super) fn pick_throw(
    picker: super::cast_pick::CastPicker,
    hand: Option<Res<Hand>>,
    bodies: Query<(&Ragdoll, Option<&Condition>, Has<PropBody>)>,
    mut pick: ResMut<ThrowPick>,
) {
    let found = (|| {
        let hit = picker.ground(|object| object)?;
        trace!("throw pick: {hit:?}");
        let super::cast_pick::Owner::Body(body) = hit.owner else {
            return None;
        };
        let (ragdoll, condition, is_prop) = bodies.get(body).ok()?;
        let met = if is_prop {
            Met::Prop {
                mobility: ragdoll.parts.first()?.inverse_mass,
            }
        } else {
            Met::Character {
                dead: condition?.is_dead(),
            }
        };
        let held = hand
            .as_deref()
            .is_some_and(|h| matches!(h.holding, Holding::Drag { body: b, .. } if b == body));
        throwable(met, held).then_some(body)
    })();
    if pick.set_if_neq(ThrowPick(found)) {
        debug!("throw pick: {found:?}");
    }
}

/// What a throw reads of `body` (and of its caster) before a tick.
pub fn throw_view(body: &Ragdoll, caster_point: Vec3) -> Option<ThrowView> {
    let part = body.parts.first()?;
    Some(ThrowView {
        target_at: (body.origin_at(0)? * CM).to_array(),
        mass: part.inverse_mass,
        caster_point: caster_point.to_array(),
        tick_ms: TICK_MS,
    })
}

/// Before each body tick, each throw reads its body.
pub(super) fn sense_throws(
    mut casters: Query<(Entity, &mut PowerCaster, Has<PlayerBody>)>,
    bodies: Query<&Ragdoll>,
    ground: Option<Res<CursorGround>>,
) {
    for (entity, mut caster, player) in &mut casters {
        let root = bodies
            .get(entity)
            .ok()
            .and_then(|r| r.parts.first())
            .map_or(Vec3::ZERO, |p| Vec3::from(p.transform().translation) * CM);
        let point = match ground.as_deref().and_then(|g| g.0) {
            Some(at) if player => at * CM,
            _ => root,
        };
        for entry in caster.active.iter_mut() {
            let ActiveClass::Throw(power) = &mut entry.class else {
                continue;
            };
            let Some(bits) = power.body else { continue };
            let view = Entity::try_from_bits(bits)
                .and_then(|e| bodies.get(e).ok())
                .and_then(|body| throw_view(body, point));
            if view.is_none() {
                power.state = ENDING;
            }
            entry.throw_view = view;
        }
    }
}

/// Moves `body` by a throw tick's velocity (cm a tick); a steered one adds
/// [`kinesis_spin`] of the first part's own motion.
pub fn push_body(body: &mut Ragdoll, velocity: Vec3, steered: bool) {
    let mut v = velocity;
    if steered {
        let moved = body.origin_moved(0).unwrap_or(Vec3::ZERO) * CM;
        v += Vec3::from(kinesis_spin(moved.to_array()));
    }
    body.push_origin(0, v / CM);
    body.wake();
}

/// A throw ended before its drop: its lift's voice fades (the key the step
/// gave the lift's voice).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThrowFade(pub u32);

/// The frame's side of the thaum sounds the step decides (a throw's pick
/// and lift, a kinesis pick): their sound sets by name, loaded once, and
/// the lifts' voices by the key the step gave them (the step decides the
/// sounds; the frame's sound bank plays them).
#[derive(Resource, Default)]
pub struct ThrowSounds {
    sets: bevy::platform::collections::HashMap<&'static str, u32>,
    voices: bevy::platform::collections::HashMap<u32, u32>,
}

/// Plays `prefix`'s set at `at` (m) on the frame: the set loaded on its
/// first use, the bank's roll for the pitch first, then the variant; the
/// voice is kept under `key` when one is given.
pub(super) fn play_on_frame(
    world: &mut World,
    prefix: &'static str,
    volume: f32,
    pitch: fn(f32) -> f32,
    at: Vec3,
    key: Option<u32>,
) {
    let Some(game) = world.get_resource::<Game>().cloned() else {
        return;
    };
    if !world.contains_resource::<SoundBank>()
        || !world.contains_resource::<Assets<AudioSource>>()
        || !world.contains_resource::<AudioEnvironment>()
    {
        return;
    }
    let mut state = world.remove_resource::<ThrowSounds>().unwrap_or_default();
    world.resource_scope(|world, mut bank: Mut<SoundBank>| {
        let set = *state.sets.entry(prefix).or_insert_with(|| {
            let mut sources = world.resource_mut::<Assets<AudioSource>>();
            bank.load_set(&game, &mut sources, prefix)
        });
        let pitch = pitch(bank.random_unit());
        let voice = world
            .resource_mut::<AudioEnvironment>()
            .play_set(&mut bank, set, at, volume, pitch, 0);
        if let (Some(key), Some(voice)) = (key, voice) {
            state.voices.insert(key, voice);
        }
    });
    world.insert_resource(state);
}

/// Fades a lift's voice while it is still allocated.
pub fn fade_voice(env: &mut AudioEnvironment, voice: u32) {
    if let Some(v) = env.voice_mut(voice)
        && v.flags & voice_flags::ALLOCATED != 0
    {
        v.flags |= voice_flags::FADING;
    }
}

/// Fades the lift whose voice the step keyed `key`, on the frame.
fn fade_on_frame(world: &mut World, key: u32) {
    let voice = world
        .get_resource_mut::<ThrowSounds>()
        .and_then(|mut s| s.voices.remove(&key));
    if let (Some(voice), Some(mut env)) = (voice, world.get_resource_mut::<AudioEnvironment>()) {
        fade_voice(&mut env, voice);
    }
}

/// After each tick, the throws' sounds: the pick's at the body when the
/// cursor's throw started, the lift's, and the fades of throws that ended
/// before their drop. The step decides them; the frame plays them, a
/// lift's voice kept under a key the step hands out (`ThrowFade` names it).
pub(super) fn sound_throws(
    mut casters: Query<&mut PowerCaster>,
    bodies: Query<&Ragdoll>,
    mut faded: MessageReader<ThrowFade>,
    mut frame: ResMut<exa_physics::sim_app::FrameCommands>,
    mut keys: Local<u32>,
) {
    for &ThrowFade(key) in faded.read() {
        frame.send(move |world| fade_on_frame(world, key));
    }
    for mut caster in &mut casters {
        for entry in caster.active.iter_mut() {
            let ActiveClass::Throw(power) = &mut entry.class else {
                continue;
            };
            if !power.sound.pick && !power.sound.lift {
                continue;
            }
            let at = power
                .body
                .and_then(Entity::try_from_bits)
                .and_then(|e| bodies.get(e).ok())
                .and_then(|b| b.origin_at(0));
            let Some(at) = at else { continue };
            if std::mem::take(&mut power.sound.pick) {
                frame.send(move |world| {
                    play_on_frame(
                        world,
                        THROW_PICK_SOUND,
                        THROW_PICK_VOLUME,
                        throw_pick_pitch,
                        at,
                        None,
                    );
                });
            }
            if std::mem::take(&mut power.sound.lift) {
                *keys = keys.wrapping_add(1);
                let key = *keys;
                power.sound.voice = Some(key);
                frame.send(move |world| {
                    play_on_frame(
                        world,
                        THROW_LIFT_SOUND,
                        THROW_LIFT_VOLUME,
                        throw_lift_pitch,
                        at,
                        Some(key),
                    );
                });
            }
        }
    }
}

/// After each tick, each throw's velocity moves its body.
pub(super) fn push_throws(
    mut commands: Commands,
    mut casters: Query<(Entity, &mut PowerCaster)>,
    mut bodies: Query<&mut Ragdoll>,
) {
    for (user, mut caster) in &mut casters {
        for entry in caster.active.iter_mut() {
            let ActiveClass::Throw(power) = &mut entry.class else {
                continue;
            };
            if let Some(until_ms) = power.held_until.take()
                && let Some(body) = power.body.and_then(Entity::try_from_bits)
                && bodies.contains(body)
            {
                commands.entity(body).insert(UserStamp { user, until_ms });
            }
            let Some((velocity, steered)) = power.push.take() else {
                continue;
            };
            let Some(target) = power.body.and_then(Entity::try_from_bits) else {
                continue;
            };
            if let Ok(mut body) = bodies.get_mut(target) {
                push_body(&mut body, Vec3::from(velocity), steered);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fade marks a voice still allocated, and leaves a freed one free.
    #[test]
    fn a_fade_marks_only_an_allocated_voice() {
        let mut env = AudioEnvironment::new();
        let voice = env.allocate().unwrap();
        fade_voice(&mut env, voice);
        assert_ne!(env.voice(voice).unwrap().flags & voice_flags::FADING, 0);
        let free = env.allocate().unwrap() + 1;
        fade_voice(&mut env, free);
        assert_eq!(env.voice(free).unwrap().flags, 0);
    }

    #[test]
    fn only_dead_characters_and_mobile_props_unheld_are_thrown() {
        assert!(throwable(Met::Character { dead: true }, false));
        assert!(!throwable(Met::Character { dead: false }, false));
        assert!(throwable(Met::Prop { mobility: 0.0101 }, false));
        // The single 0.01 widens to just under the double 0.01.
        assert!(!throwable(Met::Prop { mobility: 0.01 }, false));
        assert!(!throwable(Met::Prop { mobility: 1.0 }, true));
    }
}
