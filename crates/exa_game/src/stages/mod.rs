//! The character tick in its stages (`exa_physics::body::CharacterStage`):
//! the driver that ticks every living physical body in the original's
//! order, the player's last, each body's whole tick before the next one's,
//! and Exarust's work in each stage, one module a stage.
//!
//! Exarust's parts of the original's body tick, by stage:
//! - Upkeep: red regeneration (`exa_combat::regen`), the thorax hunch's
//!   rise ([`sampler::rise_thorax_hunch`]);
//! - Readiness: the daze's stance and readiness (`exa_combat::stance`, over
//!   every character before the ticks), a get-up's extra energy
//!   (`balance`);
//! - Behaviour: the player's input (`player` and `steering`, in
//!   `FixedPreUpdate`) and the AI (`exa_ai::BrainTick`: the brains, the
//!   duellists' combat state and the walks, every body tick before the
//!   character controllers); the
//!   player's writes its locomotion request;
//! - Posture: the condition (`exa_combat::condition`), the collapse's dying
//!   tick (`exa_combat::collapse`), the falls, staggers and get-ups
//!   (`balance`), the fall check (`fall`), and what the body does this tick
//!   ([`posture`]); a body that dies here skips the stages after it;
//! - Attack: the thorax hunch's ease on an attack clip, the swing routine
//!   (`exa_combat::swing`), then the health
//!   bookkeeping (`exa_combat::regen`);
//! - Movement: a get-up picked again and the turn toward a fall
//!   (`balance`); the movement decision, the body turn and the alignment
//!   from the locomotion request, and the capsule's velocity; the walk's
//!   lowest ankle, the controller's step left out of the carry, the
//!   balance and the ground under the feet ([`movement`]);
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
use exa_combat::condition::ticked_lives;
use exa_physics::CharacterBody;
use exa_physics::body::{
    CharacterStage, CharacterTick, CharacterTicks, OutOfScene, Ragdoll, TickedCharacter,
    run_character_ticks,
};

use crate::player::PlayerBody;

/// Runs the character ticks of living physical bodies: the driver in the
/// fixed step's [`CharacterTicks`], and Exarust's stages in the
/// [`CharacterTick`] schedule.
pub struct CharacterTickPlugin;

impl Plugin for CharacterTickPlugin {
    fn build(&self, app: &mut App) {
        use CharacterStage::*;
        app.init_resource::<sampler::SampledPose>()
            .add_message::<exa_physics::body::Footstep>()
            .add_message::<exa_physics::body::FootSounded>()
            .add_systems(FixedUpdate, tick_characters.in_set(CharacterTicks))
            // A body that dies in its posture stage leaves its tick there.
            .configure_sets(
                CharacterTick,
                (Attack, Movement, Sampler, Carry).run_if(ticked_lives),
            )
            .add_systems(
                CharacterTick,
                (
                    posture::take_bearing
                        .in_set(Posture)
                        .after(crate::balance::take_posture)
                        .after(crate::fall::fall_to_death),
                    (
                        movement::decide_movement,
                        movement::measure_ground,
                        movement::gait_footsteps,
                        movement::footwork_sounds,
                    )
                        .chain()
                        .in_set(Movement),
                    note_strike_chests
                        .in_set(Attack)
                        .before(exa_combat::swing::run_swings),
                    push_striking_fists
                        .in_set(Attack)
                        .after(exa_combat::swing::run_swings),
                    sampler::rise_thorax_hunch.in_set(Upkeep),
                    sampler::sample_clip.in_set(Sampler),
                    carry::aim_body.in_set(Carry),
                ),
            );
        bridge_character_tick_types(app);
    }
}

/// The data the character tick's systems touch (the driver's own query
/// too: it takes the whole world), for the simulation's bridge.
fn bridge_character_tick_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_copy::<Animator>()
        .bridge_copy::<TickedClips>()
        .bridge_copy::<exa_anim::BodyPart>()
        .bridge_copy::<exa_anim::Stature>()
        .bridge_copy_opaque_resource::<exa_anim::Motions>()
        .bridge_copy::<PlayerBody>()
        .bridge_copy::<OutOfScene>()
        .bridge_copy_drawn::<CharacterBody>()
        .bridge_copy::<exa_ai::brain::Npc>()
        .bridge_copy::<exa_ai::combatant::Combatant>()
        .bridge_copy::<exa_ai::Duellist>()
        .bridge_copy::<exa_combat::CombatTarget>()
        .bridge_copy::<exa_combat::Attacking>()
        .bridge_copy::<exa_combat::attack::Attacker>()
        .bridge_copy::<exa_combat::Condition>()
        .bridge_copy::<exa_combat::condition::PostureBranch>()
        .bridge_copy::<exa_combat::condition::StateFlags>()
        .bridge_copy::<exa_combat::Fighter>()
        .bridge_copy::<exa_combat::stance::Stance>()
        .bridge_copy::<exa_combat::swing::Swing>()
        .bridge_copy::<exa_combat::swing::SwingState>()
        .bridge_copy::<crate::balance::ShockHeld>()
        .bridge_copy::<crate::golem_body::StatueRest>()
        .bridge_copy::<exa_physics::body::InView>()
        .bridge_copy::<crate::item_effects::WornEffects>()
        .bridge_move_resource::<sampler::SampledPose>()
        .bridge_message::<exa_physics::body::Footstep>()
        .bridge_message::<exa_physics::body::FootSounded>();
}

/// The ticked body's engagement target (the original's combat target on
/// the character): the player's auto-target, otherwise the brain's combat
/// target, otherwise a brainless duellist's foe while it is in combat with
/// it. It is kept on the body as its [`exa_combat::CombatTarget`], which the
/// attack starters' aim and the parry check read, and the swing's homing
/// reads its chest height (the target body's, in centimetres). No target
/// removes the chest mark, and a side swing then does not home.
#[allow(clippy::type_complexity)]
fn note_strike_chests(
    ticked: Res<exa_physics::body::TickedCharacter>,
    players: Query<&crate::player::PlayerBody>,
    npcs: Query<&exa_ai::brain::Npc>,
    duellists: Query<&exa_ai::Duellist>,
    bodies: Query<&Ragdoll>,
    mut targets: Query<Option<&mut exa_combat::CombatTarget>, With<exa_combat::Fighter>>,
    mut commands: Commands,
) {
    let brain = ticked.character.unwrap_or(ticked.body);
    let foe = match players.get(ticked.body) {
        Ok(player) => player.foe(),
        Err(_) => match npcs.get(brain) {
            Ok(npc) => npc.pass.target.and_then(Entity::try_from_bits),
            Err(_) => duellists
                .get(ticked.body)
                .ok()
                .and_then(exa_ai::Duellist::combat_target),
        },
    };
    match targets.get_mut(ticked.body) {
        Ok(Some(mut target)) => {
            if target.0 != foe {
                target.0 = foe;
            }
        }
        Ok(None) => {
            commands
                .entity(ticked.body)
                .insert(exa_combat::CombatTarget(foe));
        }
        Err(_) => {}
    }
    let chest = foe
        .and_then(|foe| bodies.get(foe).ok())
        .and_then(|body| body.chest_point());
    match chest {
        Some(chest) => {
            commands
                .entity(ticked.body)
                .insert(exa_combat::strike_body::StrikeChest {
                    y_cm: chest.y * 100.0,
                });
        }
        None => {
            commands
                .entity(ticked.body)
                .remove::<exa_combat::strike_body::StrikeChest>();
        }
    }
}

/// While the character's weapon class word is 0, each striker whose window
/// is open is pushed along the cursor (the player's aim, otherwise the
/// body's yaw), 10 cm at the middle of the clip down to nothing from nine
/// tenths ([`exa_combat::strike_body::striking_fist`]). A striker is one of
/// the swing's held slots; its window is the swing's for that slot, and the
/// push moves the slot's hand (the right for the first, the left for the
/// second). The push is in centimetres; the part moves in metres, by its
/// origin weights.
type StrikingFist<'a> = (
    &'a exa_combat::Attacking,
    &'a exa_combat::swing::SwingState,
    Mut<'a, Ragdoll>,
    &'a Animator,
    Option<&'a PlayerBody>,
    Option<&'a exa_physics::body::locomotion::Locomotor>,
);

/// The hands the two held slots strike with: the skeleton's right and left.
const SLOT_HANDS: [usize; 2] = [17, 18];

fn push_striking_fists(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<exa_anim::Motions>>,
    mut bodies: Query<StrikingFist<'_>>,
) {
    let Some(motions) = motions else {
        return;
    };
    // The ticked character's own swing only: the stage runs once a
    // character.
    if let Ok((attack, swing, mut body, anim, player, loco)) = bodies.get_mut(ticked.body) {
        let open: Vec<usize> = (0..2).filter(|&slot| swing.open[slot]).collect();
        if open.is_empty() {
            return;
        }
        let Some(clip) = motions.clip(anim, &anim.action) else {
            return;
        };
        let progress = anim.clip_progress(clip);
        let Some(yaw) = player.map(|p| p.aim).or(loco.map(|l| l.yaw)) else {
            return;
        };
        let cursor = exa_physics::body::locomotion::forward(yaw);
        let mut pushed = 0;
        for &slot in &open {
            let Some(push) =
                exa_combat::strike_body::striking_fist(attack.weapon_class, progress, true, cursor)
            else {
                continue;
            };
            if let Some(part) = body.parts.iter().position(|p| p.bone == SLOT_HANDS[slot]) {
                body.push_origin(part, push / 100.0);
                pushed += 1;
            }
        }
        debug!(
            "fist nudge: class {:#x} at {progress:.2}, {} striker(s) open, {pushed} pushed",
            attack.weapon_class,
            open.len()
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
        (With<Animator>, With<TickedClips>, Without<OutOfScene>),
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
    // In entity order before the player goes last, whichever world's
    // tables the query walked.
    order.sort_unstable_by_key(|&(_, e)| e);
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
    #[derive(Resource, Default, Debug)]
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
            spheres: None,
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

    /// A body whose collapse ends in its posture stage (its tone reaches a
    /// quarter there) is dead and limp at once and runs none of the stages
    /// after it (attack, movement, sampler, carry); the other body runs
    /// them all.
    #[test]
    fn a_body_killed_in_its_posture_stage_leaves_its_tick() {
        use exa_combat::collapse::{Collapse, CollapsePlugin};
        use exa_combat::{Condition, Fighter};
        let mut app = App::new();
        app.add_plugins((BodyPlugin, CharacterTickPlugin, CollapsePlugin))
            .insert_resource(exa_physics::Physics::default())
            .insert_resource(crate::limp::tests::motions())
            .init_resource::<Ticked>()
            .add_systems(
                CharacterTick,
                (
                    note.in_set(CharacterStage::Attack),
                    note.in_set(CharacterStage::Carry),
                ),
            );
        let world = app.world_mut();
        let clips = || (Animator::new("Default", "Idle"), TickedClips);
        let mut dying = body(true);
        dying.drive.as_mut().unwrap().tone = 0.252;
        let dying = world
            .spawn((
                dying,
                clips(),
                Fighter::default(),
                Condition::COLLAPSING,
                Collapse::default(),
                GlobalTransform::default(),
            ))
            .id();
        let standing = world.spawn((body(true), clips())).id();
        world.run_schedule(FixedUpdate);
        assert_eq!(*world.get::<Condition>(dying).unwrap(), Condition::DEAD);
        assert!(world.get::<Ragdoll>(dying).unwrap().drive.is_none());
        let ticked: Vec<Entity> = world
            .resource::<Ticked>()
            .0
            .iter()
            .map(|t| t.body)
            .collect();
        assert_eq!(ticked, [standing, standing]);
    }

    /// The posture-stage test's app, its fixed step run in one world or
    /// moved to the simulation's world behind the bridge.
    fn dying_app(bridge: Option<exa_physics::sim_app::SimMode>) -> (App, Entity, Entity) {
        use bevy::ecs::schedule::ScheduleLabel;
        use exa_combat::collapse::{Collapse, CollapsePlugin};
        use exa_combat::{Condition, Fighter};
        use exa_physics::sim_bridge::{SimBridgeAppExt, SimBridgePlugin};
        let mut app = App::new();
        app.add_plugins((BodyPlugin, CharacterTickPlugin, CollapsePlugin))
            .insert_resource(exa_physics::Physics::default())
            .insert_resource(crate::limp::tests::motions())
            .init_resource::<Ticked>()
            .add_systems(
                CharacterTick,
                (
                    note.in_set(CharacterStage::Attack),
                    note.in_set(CharacterStage::Carry),
                ),
            );
        if let Some(mode) = bridge {
            app.add_plugins((
                exa_physics::sim_app::SimShellPlugin { mode },
                SimBridgePlugin {
                    extra: vec![CharacterTick.intern()],
                },
            ))
            .bridge_lend_resource::<Ticked>();
        }
        let world = app.world_mut();
        let clips = || (Animator::new("Default", "Idle"), TickedClips);
        let mut dying = body(true);
        dying.drive.as_mut().unwrap().tone = 0.252;
        let dying = world
            .spawn((
                dying,
                clips(),
                Fighter::default(),
                Condition::COLLAPSING,
                Collapse::default(),
                GlobalTransform::default(),
            ))
            .id();
        let standing = world.spawn((body(true), clips())).id();
        (app, dying, standing)
    }

    /// The posture-stage death, through `FixedMain`, in one world and with
    /// the fixed step and the character tick moved behind the bridge
    /// (inline and on the simulation's thread): the same death, the same
    /// stages ticked, and every type the moved systems touch is bridged.
    #[test]
    fn the_character_tick_runs_alike_behind_the_bridge() {
        use exa_combat::Condition;
        use exa_physics::sim_app::SimMode;
        use exa_physics::sim_bridge::FrameBridge;
        let run = |bridge: Option<SimMode>| {
            let (mut app, dying, standing) = dying_app(bridge);
            for _ in 0..3 {
                app.world_mut().run_schedule(bevy::app::FixedMain);
            }
            let world = app.world();
            if bridge.is_some() {
                let coverage = world.resource::<FrameBridge>().coverage.clone().unwrap();
                assert!(coverage.missing.is_empty(), "{:#?}", coverage.missing);
                assert!(coverage.schedules.iter().any(|s| s == "CharacterTick"));
            }
            let ticked: Vec<bool> = world
                .resource::<Ticked>()
                .0
                .iter()
                .map(|t| t.body == standing)
                .collect();
            (
                *world.get::<Condition>(dying).unwrap(),
                world.get::<Ragdoll>(dying).unwrap().drive.is_none(),
                ticked,
                world.resource::<exa_physics::body::BodyTicks>().0,
            )
        };
        let alone = run(None);
        assert_eq!(alone.0, Condition::DEAD);
        assert!(alone.1);
        assert_eq!(alone.3, 3);
        for mode in [SimMode::Inline, SimMode::Thread] {
            assert_eq!(alone, run(Some(mode)), "{mode:?}");
        }
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
