//! A running throw: the body under the cursor it may take, what it reads of
//! that body each tick and the velocity it puts on it
//! (`docs/THAUMATURGY.md`, Force).
//!
//! **The pick.** The cursor's ray passes over the level and meets the
//! nearest body: a prop's stand-in or a character's sphere. It can be
//! thrown when it is a dead character, or a prop whose mobility is at least
//! 0.01 (compared in double), and nobody holds it. A throw a kinesis starts
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
//! [inf: the original holds the body's single node: its centre (+0x250),
//! weights (+0x220) and mobility (+0x54). Exarust takes the first part's
//! own origin and inverse mass, the prop's whole rigid body or the
//! character's root part. The item's 0x80000 refusal and its 0x200
//! permission have no Exarust counterpart and are not read. The caster's
//! aim point is the controller's; Exarust takes the cursor's ground point.]
//! [inf: the original lifts a handed throw at its cast and plays the pick
//! sound there; Exarust does both on the throw's first tick. A faded voice
//! that has already freed itself is left alone.]
//! Each tick it coasts after the shell drops it stamps the body with the
//! caster for 2 s ([`super::user_stamp`]).
//! The throw's shell and the special material are not wired.

use bevy::prelude::*;

use super::user_stamp::UserStamp;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank, voice_flags};
use exa_combat::Condition;
use exa_physics::Physics;
use exa_physics::body::Ragdoll;
use exa_physics::prop::{PropBody, StandIns};

use super::cast::{ActiveClass, PowerCaster, TICK_MS};
use super::kinesis_live::KinesisCursor;
use super::power::ENDING;
use super::power::force::{
    THROW_LIFT_SOUND, THROW_LIFT_VOLUME, THROW_PICK_SOUND, THROW_PICK_VOLUME, ThrowView,
    kinesis_spin, throw_lift_pitch, throw_pick_pitch,
};
use crate::hand::{Hand, Holding, character_hit};
use crate::player::{CursorGround, PlayerBody};

const CM: f32 = 100.0;
/// How far the cursor's ray looks for a body (m).
const PICK_RANGE: f32 = 40.0;
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

/// Each frame, the throwable body under the player's cursor.
#[allow(clippy::type_complexity)]
pub(super) fn pick_throw(
    cursor: Res<KinesisCursor>,
    physics: Option<Res<Physics>>,
    stand_ins: Option<Res<StandIns>>,
    hand: Option<Res<Hand>>,
    bodies: Query<(Entity, &Ragdoll, Option<&Condition>, Has<PropBody>)>,
    mut pick: ResMut<ThrowPick>,
) {
    let found = (|| {
        let (origin, dir) = cursor.0?;
        let (physics, stand_ins) = (physics.as_deref()?, stand_ins.as_deref()?);
        let not_prop = |h| stand_ins.owner_of(h).is_none();
        let prop = physics
            .fixed_pick_except(origin, dir, PICK_RANGE, &not_prop)
            .and_then(|(h, t)| stand_ins.owner_of(h).map(|(body, _)| (t, body)));
        let reach = prop.map_or(PICK_RANGE, |p| p.0);
        let characters = bodies
            .iter()
            .filter_map(|(e, r, c, is_prop)| (!is_prop).then_some((e, r, c.copied()?)));
        let (body, met) = match character_hit(origin, dir, reach, characters) {
            Some((_, crate::hand::Hit::Character { body, dead, .. })) => {
                (body, Met::Character { dead })
            }
            _ => {
                let (_, body) = prop?;
                let (_, ragdoll, ..) = bodies.get(body).ok()?;
                let mobility = ragdoll.parts.first()?.inverse_mass;
                (body, Met::Prop { mobility })
            }
        };
        let held = hand
            .as_deref()
            .is_some_and(|h| matches!(h.holding, Holding::Drag { body: b, .. } if b == body));
        throwable(met, held).then_some(body)
    })();
    pick.set_if_neq(ThrowPick(found));
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

/// A throw ended before its drop: its lift's voice fades.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThrowFade(pub u32);

/// What the throw sounds take from the game (none in tests without it).
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct ThrowAudio<'w> {
    game: Option<Res<'w, Game>>,
    bank: Option<ResMut<'w, SoundBank>>,
    sources: Option<ResMut<'w, Assets<AudioSource>>>,
    env: Option<ResMut<'w, AudioEnvironment>>,
}

/// The pick's and the lift's sound sets, loaded once.
#[derive(Default)]
pub(super) struct ThrowSets {
    pick: Option<u32>,
    lift: Option<u32>,
}

/// Plays `prefix`'s set at `at` (m): the roll first, then the variant.
fn play(
    audio: &mut ThrowAudio,
    set: &mut Option<u32>,
    prefix: &str,
    at: Vec3,
    volume: f32,
    pitch: fn(f32) -> f32,
) -> Option<u32> {
    let (Some(game), Some(bank), Some(sources), Some(env)) = (
        audio.game.as_deref(),
        audio.bank.as_deref_mut(),
        audio.sources.as_deref_mut(),
        audio.env.as_deref_mut(),
    ) else {
        return None;
    };
    let set = *set.get_or_insert_with(|| bank.load_set(game, sources, prefix));
    let pitch = pitch(bank.random_unit());
    env.play_set(bank, set, at, volume, pitch, 0)
}

/// Fades a lift's voice while it is still allocated.
pub fn fade_voice(env: &mut AudioEnvironment, voice: u32) {
    if let Some(v) = env.voice_mut(voice)
        && v.flags & voice_flags::ALLOCATED != 0
    {
        v.flags |= voice_flags::FADING;
    }
}

/// After each tick, the throws' sounds: the pick's at the body when the
/// cursor's throw started, the lift's, and the fades of throws that ended
/// before their drop.
pub(super) fn sound_throws(
    mut casters: Query<&mut PowerCaster>,
    bodies: Query<&Ragdoll>,
    mut faded: MessageReader<ThrowFade>,
    mut audio: ThrowAudio,
    mut sets: Local<ThrowSets>,
) {
    for &ThrowFade(voice) in faded.read() {
        if let Some(env) = audio.env.as_deref_mut() {
            fade_voice(env, voice);
        }
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
                play(
                    &mut audio,
                    &mut sets.pick,
                    THROW_PICK_SOUND,
                    at,
                    THROW_PICK_VOLUME,
                    throw_pick_pitch,
                );
            }
            if std::mem::take(&mut power.sound.lift) {
                power.sound.voice = play(
                    &mut audio,
                    &mut sets.lift,
                    THROW_LIFT_SOUND,
                    at,
                    THROW_LIFT_VOLUME,
                    throw_lift_pitch,
                );
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
