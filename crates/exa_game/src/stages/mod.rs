//! The character tick in its stages (`exa_physics::body::CharacterStage`):
//! the driver that ticks every living physical body in the original's
//! order, the player's last, each body's whole tick before the next one's,
//! and Exarust's work in each stage, one module a stage.
//!
//! Exarust's parts of the original's body tick, by stage:
//! - Readiness: the daze's stance and readiness (`exa_combat::stance`);
//! - Behaviour: the player's input (`player`, in `FixedPreUpdate`) and the
//!   duellist (`exa_ai`, once a frame);
//! - Posture: the condition and the collapse's dying tick
//!   (`exa_combat::condition::PostureTick`), falls (`balance`), and what
//!   the body does this tick ([`posture`]);
//! - Attack: the swing routine (`exa_combat::swing`);
//! - Movement: the walk's lowest ankle, the controller's step left out of
//!   the carry, the balance and the ground under the feet ([`movement`]);
//! - Sampler: the clip's clock, capture and sample, and the condition's
//!   clip-end writer ([`sampler`]);
//! - Carry: the targets on the character frame, the lift, the controller's
//!   follow ([`carry`]).
//!
//! The parts not in a module here still run stage by stage over every
//! character, ahead of the character ticks (or once a frame); each joins
//! its stage with the change that rewrites it.

pub mod carry;
pub mod movement;
pub mod posture;
pub mod sampler;

use bevy::prelude::*;
use exa_anim::{Animator, TickedClips};
use exa_physics::CharacterBody;
use exa_physics::body::{
    CharacterStage, CharacterTick, CharacterTicks, Ragdoll, TickedCharacter, run_character_ticks,
};

use crate::player::PlayerBody;

/// Runs the character ticks of living physical bodies: the driver in the
/// fixed step's [`CharacterTicks`], and Exarust's stages in the
/// [`CharacterTick`] schedule.
pub struct CharacterTickPlugin;

impl Plugin for CharacterTickPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<sampler::SampledPose>()
            .add_systems(FixedUpdate, tick_characters.in_set(CharacterTicks))
            .add_systems(
                CharacterTick,
                (
                    posture::take_bearing.in_set(CharacterStage::Posture),
                    movement::measure_ground.in_set(CharacterStage::Movement),
                    sampler::sample_clip.in_set(CharacterStage::Sampler),
                    carry::aim_body.in_set(CharacterStage::Carry),
                ),
            );
    }
}

/// Every living physical body (one with muscles whose clips its ticks
/// advance), ticked in the original's order ([`player_last`]), each with
/// its character: the controller above it, if any. Nothing ticks before the
/// motion set is loaded.
#[allow(clippy::type_complexity)]
fn tick_characters(
    world: &mut World,
    bodies: &mut QueryState<
        (Entity, &Ragdoll, Has<PlayerBody>),
        (With<Animator>, With<TickedClips>),
    >,
    mut order: Local<Vec<(bool, Entity)>>,
) {
    if !world.contains_resource::<exa_anim::Motions>() {
        return;
    }
    order.clear();
    order.extend(
        bodies
            .iter(world)
            .filter(|(_, body, _)| body.drive.is_some())
            .map(|(e, _, player)| (player, e)),
    );
    player_last(&mut order);
    let ticks: Vec<TickedCharacter> = order
        .iter()
        .map(|&(_, body)| TickedCharacter {
            body,
            character: character_of(world, body),
        })
        .collect();
    run_character_ticks(world, ticks);
}

/// The nearest ancestor of `body` with a character controller.
fn character_of(world: &World, body: Entity) -> Option<Entity> {
    let mut node = world.get::<ChildOf>(body).map(ChildOf::parent);
    while let Some(n) = node {
        if world.get::<CharacterBody>(n).is_some() {
            return Some(n);
        }
        node = world.get::<ChildOf>(n).map(ChildOf::parent);
    }
    None
}

/// Puts the player's body (flagged `true`) after every other, which keep
/// their order: the original ticks its characters from the last registered
/// to the first, the player.
fn player_last(order: &mut [(bool, Entity)]) {
    order.sort_by_key(|&(player, _)| player);
}

#[cfg(test)]
mod tests {
    use bevy::math::Affine3A;
    use exa_physics::body::{BodyPlugin, PartSetup};

    use super::*;

    /// Every character tick the stages ran, in order.
    #[derive(Resource, Default)]
    struct Ticked(Vec<TickedCharacter>);

    fn note(ticked: Res<TickedCharacter>, mut log: ResMut<Ticked>) {
        log.0.push(*ticked);
    }

    /// A one-part body, with muscles if `living`.
    fn body(living: bool) -> Ragdoll {
        let body = Ragdoll::new(&[PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(Vec3::Y),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
        }]);
        if living { body.with_drive() } else { body }
    }

    /// The driver ticks every living body whose clips its ticks advance,
    /// the player's last, each with the controller above it (through a
    /// plain node) or none; a limp body and one whose clips play by frames
    /// are not ticked, and nothing ticks before the motion set is loaded.
    #[test]
    fn living_bodies_tick_in_order_with_their_characters() {
        let mut app = App::new();
        app.add_plugins(BodyPlugin)
            .insert_resource(exa_physics::Physics::default())
            .init_resource::<Ticked>()
            .add_systems(FixedUpdate, tick_characters.in_set(CharacterTicks))
            .add_systems(CharacterTick, note.in_set(CharacterStage::Upkeep));
        let world = app.world_mut();
        let clips = || (Animator::new("Default", "Idle"), TickedClips);
        let player = world
            .spawn((body(true), clips(), PlayerBody::new(0.0)))
            .id();
        let walker = world.spawn(CharacterBody::default()).id();
        let node = world.spawn(ChildOf(walker)).id();
        let walking = world.spawn((body(true), clips(), ChildOf(node))).id();
        let standing = world.spawn((body(true), clips())).id();
        world.spawn((body(false), clips()));
        world.spawn((body(true), Animator::new("Default", "Idle")));

        world.run_schedule(FixedUpdate);
        assert!(world.resource::<Ticked>().0.is_empty(), "no motion set");
        world.insert_resource(crate::limp::tests::motions());
        world.run_schedule(FixedUpdate);
        let ticked = &world.resource::<Ticked>().0;
        assert_eq!(ticked.len(), 3);
        assert_eq!(
            ticked[2],
            TickedCharacter {
                body: player,
                character: None
            }
        );
        let others = [
            TickedCharacter {
                body: walking,
                character: Some(walker),
            },
            TickedCharacter {
                body: standing,
                character: None,
            },
        ];
        assert!(others.iter().all(|t| ticked[..2].contains(t)), "{ticked:?}");
    }

    #[test]
    fn the_player_is_ticked_last() {
        let e = |i: u32| Entity::from_raw_u32(i).unwrap();
        let mut order = vec![(false, e(3)), (true, e(1)), (false, e(7)), (false, e(2))];
        player_last(&mut order);
        let ticked: Vec<Entity> = order.iter().map(|&(_, x)| x).collect();
        assert_eq!(ticked, [e(3), e(7), e(2), e(1)]);
    }
}
