//! NPC behaviour. For now a duellist: once the fight starts it enters
//! combat in its guard, faces the target, closes to striking distance and
//! attacks at intervals with varied strikes, guarding while the target is
//! far and standing in its combat idle near it. NPCs act through the same
//! pieces as the player (body velocity, animator, stance, attacks); the
//! duellist holds every swing through (it never releases one).
//!
//! A level's living characters carry a brain ([`brain`]): their roles
//! choose their idle activity and rate the player, and a human-framed one
//! whose rating turns hostile engages as the duellist does.

use bevy::prelude::*;
use exa_anim::{Animator, Motions};
use exa_combat::attack::{self, Attacker, Busy};
use exa_combat::stance::{self, Stance};
use exa_combat::swing::{self, SwingState};
use exa_combat::{AttackStart, Attacking, Blade, Condition, Fighter};
use exa_physics::CharacterBody;
use exa_physics::body::{BodyTicks, Fallen, MAX_ENERGY, Ragdoll};

use crate::combatant::{Attitude, Combatant, Observed, Observer, atan2_xz, perceive};

pub mod activity;
pub mod brain;
pub mod combatant;
pub mod fight;
pub mod golem;
pub mod hear;
pub mod idle;
pub mod manager;
pub mod order;
pub mod path;
pub mod personality;
pub mod place;
pub mod roles;
pub mod search;
pub mod sense;

pub use order::{Order, follow};
pub use sense::{Decision, Mind, decide, noticeability, tick, view_weight};

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (brain::apply_new_roles, brain::rate_the_player).chain(),
                duellist,
            )
                .chain(),
        );
    }
}

/// A fighter (on its body model root, whose parent carries the
/// `CharacterBody`) that duels `target` (a world-space entity, e.g. the
/// player's physics root).
#[derive(Component, Debug, Clone)]
#[require(Stance, Combatant)]
pub struct Duellist {
    pub target: Entity,
    pub cooldown: f32,
    pub next_strike: usize,
    /// The attack choice's generator.
    pub roll: u32,
    /// The fight has started: until then the duellist stands idle.
    pub engaged: bool,
    /// It stands its ground: it faces its target and takes its stance, but
    /// neither walks nor strikes (a debug aid for still comparisons).
    pub holds_ground: bool,
    /// It has entered combat (taken its guard).
    in_combat: bool,
    /// The footwork it is carrying out.
    pub footwork: place::Footwork,
    /// Ticks the footwork is held before the next change. The original's
    /// combat state biases a step change by a switching cost; the hold is
    /// this interim duellist's version of it (a guess).
    pub hold_ticks: u32,
}

impl Duellist {
    /// A duellist of `target`, waiting for the fight to start, its first
    /// strike 2 s after it comes within striking distance.
    pub fn new(target: Entity) -> Self {
        Self {
            target,
            cooldown: 2.0,
            next_strike: 0,
            roll: 0,
            engaged: false,
            holds_ground: false,
            in_combat: false,
            footwork: place::Footwork::Hold,
            hold_ticks: 0,
        }
    }
}

/// The attack arc a fighter publishes, as the position score reads it.
#[allow(clippy::too_many_arguments)]
fn published_blow(
    fighter: Entity,
    blades: &Query<&Blade>,
    anim: &Animator,
    phase: f32,
    heading: f32,
    facing: f32,
    attacking: bool,
    stunned: bool,
) -> place::Blow {
    let armed = blades
        .iter()
        .any(|b| b.wielder == fighter && b.record.armed());
    let motion = if attacking { anim.action.as_str() } else { "" };
    let action = match motion {
        "Attack Mid" | "Attack Down" | "Attack Up" => 5,
        "Attack Combo 1" | "Attack Combo 2" | "Attack Combo 3" => 8,
        "Stab" => 11,
        "Overhead Swing" => 12,
        _ => 0,
    };
    let arc = place::publish_arc(place::Swing {
        action,
        phase,
        heading,
        facing,
        armed,
        held: armed,
        recovering: stunned,
        hand: None,
        idle_clock: 0,
        idle_stamp: 0,
    });
    place::Blow {
        kind: arc.kind,
        arc_start: arc.start,
        arc_end: arc.end,
        phase,
        skill: 0.0,
        committed: attacking && phase > 0.3,
        sweeping: attacking && !stunned,
        // The observer's grip link on the opponent; a duellist has none.
        forced_range: false,
        origin_in_arc: false,
        angle: 0.0,
    }
}

/// How close a strike is thrown (m). Closing stops earlier, on the
/// position score's plateau.
const STRIKE_RANGE: f32 = 1.15;
const STRIKE_INTERVAL: f32 = 2.2;
/// Exarust's interim footwork clip (the original's AI steps in its stance).
const COMBAT_WALK: &str = "Combat Walk";

/// A fighter's reach (m), as the original's equip refresh sets it: its
/// longest held weapon's record reach (60 cm × its size plus the weapon's
/// sweet spot), else (its fists) its race's unarmed reach (the human's
/// 50 cm).
fn reach_of(fighter: Entity, blades: &Query<&Blade>) -> f32 {
    blades
        .iter()
        .filter(|b| b.wielder == fighter && b.record.armed())
        .map(|b| b.record.reach_metres())
        .reduce(f32::max)
        .unwrap_or(stance::UNARMED_REACH)
}

/// A fighter's weapon class as its strikers make it: a held weapon's hands
/// bits, else the fists' (the claws' for a clawed race's).
fn weapon_class_of(fighter: Entity, blades: &Query<&Blade>) -> u8 {
    let mine = || blades.iter().filter(|b| b.wielder == fighter);
    if let Some(weapon) = mine().find(|b| b.record.armed()) {
        return (weapon.record.flags & 0x30) as u8;
    }
    if mine().any(|b| b.record.flags & 0x40 != 0) {
        0xC0
    } else {
        0x80
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn duellist(
    mut commands: Commands,
    time: Res<Time>,
    ticks: Option<Res<BodyTicks>>,
    targets: Query<&GlobalTransform>,
    motions: Option<Res<Motions>>,
    bodies: Query<
        (
            &Condition,
            &GlobalTransform,
            Option<&Ragdoll>,
            &Fighter,
            Option<&Animator>,
        ),
        Without<Duellist>,
    >,
    children: Query<&Children>,
    blades: Query<&Blade>,
    mut minds: Query<&mut Mind>,
    mut npcs: Query<(
        Entity,
        &mut Duellist,
        &mut Stance,
        &mut Transform,
        &mut Animator,
        (&mut Attacker, &mut SwingState),
        &ChildOf,
        Option<&Attacking>,
        Has<Fallen>,
        &mut Combatant,
        &Condition,
        &GlobalTransform,
        Option<&Ragdoll>,
        &Fighter,
    )>,
    mut roots: Query<(&mut CharacterBody, &GlobalTransform)>,
) {
    let now = ticks.map_or(0, |t| t.millis());
    for (
        entity,
        mut ai,
        mut stance,
        mut t,
        mut anim,
        (mut attacker, mut swinging),
        parent,
        attacking,
        fallen,
        mut combatant,
        condition,
        gt,
        ragdoll,
        fighter,
    ) in &mut npcs
    {
        let Ok((mut body, root_gt)) = roots.get_mut(parent.parent()) else {
            continue;
        };
        body.desired_velocity = Vec3::ZERO;
        // Down, getting up, collapsing or dead (or Exarust's fall), it does
        // nothing.
        if condition.fallen_or_dying() || fallen {
            continue;
        }
        if !ai.engaged
            && let Ok(mut mind) = minds.get_mut(entity)
        {
            if let Ok(target) = targets.get(ai.target) {
                let to = target.translation() - root_gt.translation();
                let flat = Vec3::new(to.x, 0.0, to.z);
                let vertical_cm = to.y.abs() * 100.0;
                mind.distance_cm = if vertical_cm < 150.0 {
                    flat.length() * 100.0
                } else {
                    to.length() * 100.0
                };
            }
            tick(&mut mind);
            if decide(&mind) == Decision::Engage {
                ai.engaged = true;
            }
        }
        if !ai.engaged {
            continue;
        }
        let Ok(target) = targets.get(ai.target) else {
            continue;
        };
        if !ai.in_combat {
            // Entering combat: the guard, from the idle it stood in, and
            // the published reach. The combat rating comes from the
            // experience system, which is not wired here.
            ai.in_combat = true;
            combatant.in_combat = true;
            combatant.clear_combat_step(gt.translation() * 100.0, t.rotation * Vec3::Z, 0.0);
            combatant.refresh_fighting(0.0, reach_of(entity, &blades) * 100.0);
            stance::set_stance(&mut stance, &mut anim, Stance::Defence);
            info!("duellist {entity:?} enters combat: {:?}", *stance);
            continue;
        }
        // The target's body: the fighter at or under the target entity.
        let target_body = std::iter::once(ai.target)
            .chain(children.iter_descendants(ai.target))
            .find_map(|e| bodies.get(e).ok());
        // Its stance by the distance between the chests: the guard at four
        // reaches or more, the combat idle within.
        if let Some((_, tgt, tgt_body, _, _)) = target_body {
            let d = stance::chest_point(gt, ragdoll).distance(stance::chest_point(tgt, tgt_body));
            let reach = reach_of(entity, &blades);
            let chosen = stance::duellist_stance(d, reach);
            if *stance != chosen {
                info!("duellist {entity:?} stance: {chosen:?} at {d:.2} m (reach {reach:.2} m)");
                *stance = chosen;
            }
        }
        // Its stance starts with 40 frames of blend without a primary
        // striker, else twice the primary's stance blend, eased smoothly.
        let primary_blend = attacker
            .primary_record(entity, blades.iter())
            .map(|r| r.blend);
        // Leave a collapsing or dead target alone.
        if target_body.is_some_and(|(c, ..)| c.dying()) {
            if anim.looping {
                stance::take_ai_stance(&mut anim, *stance, primary_blend);
            }
            continue;
        }
        let to = target.translation() - root_gt.translation();
        let flat = Vec3::new(to.x, 0.0, to.z);
        let dist = flat.length();
        let facing = t.rotation * Vec3::Z;
        let heading = facing.x.atan2(facing.z);
        let order = follow(
            &[root_gt.translation(), target.translation()],
            root_gt.translation(),
            heading,
            0,
        );
        if dist > 0.05 {
            t.rotation = Quat::from_rotation_y(order.heading);
        }

        // One perception pass on the target, then the position terms.
        let target_forward = target.rotation() * Vec3::Z;
        let target_condition = target_body.map(|(c, ..)| *c).unwrap_or(Condition::NORMAL);
        let target_fighter = target_body.map(|(_, _, _, f, _)| f);
        let target_anim = target_body.and_then(|(_, _, _, _, a)| a);
        let phase = target_anim
            .zip(motions.as_deref())
            .and_then(|(a, m)| m.clip(a, &a.action).map(|c| a.clip_progress(c)))
            .unwrap_or(0.0);
        let observed = Observed {
            entity: ai.target,
            position_cm: target.translation() * 100.0,
            look: target_forward,
            forward: target_forward,
            condition: target_condition.0,
            invisible: false,
            hidden: false,
            mask: None,
            asleep: false,
            removed: false,
            phase,
            in_combat: target_condition != Condition::NORMAL,
            stepping: false,
            poised: false,
            speed: 0.0,
        };
        let observer = Observer {
            entity,
            position_cm: gt.translation() * 100.0,
            look: facing,
            forward: facing,
            asleep: false,
            perception: fighter.stats.perception,
            calm: 0.0,
            aggression: 0.0,
            mind_aggression: 0.0,
            resist_mind: false,
            target: Some(ai.target),
        };
        // The role database is not wired into the duellist: its stored
        // attitude toward the target stands in for the raw one, and the
        // target's own contacts (none for the player) carry no trust.
        let index = combatant.contact_index(ai.target);
        let raw = Attitude {
            value: combatant.contacts[index].attitude,
            tolerance: combatant.contacts[index].tolerance * 100.0,
        };
        let sight = perceive(
            &observer,
            &observed,
            &[],
            &mut combatant.contacts,
            index,
            raw,
            &|_| 0.0,
            &[],
            now,
        );
        combatant.target = Some(ai.target);
        if let Some(floor) = sight.alert_floor
            && combatant.alert < floor
        {
            combatant.alert = floor;
        }
        let contact = &mut combatant.contacts[index];
        if target_condition.is_dead() && contact.died_ms.is_none() {
            contact.died_ms = Some(now);
        }

        let target_angle = {
            let to_me = gt.translation() - target.translation();
            atan2_xz(to_me.x, to_me.z)
        };
        let mut blow = published_blow(
            ai.target,
            &blades,
            target_anim.unwrap_or(&anim),
            phase,
            atan2_xz(target_forward.x, target_forward.z),
            atan2_xz(target_forward.x, target_forward.z),
            target_condition == Condition::ATTACKING,
            false,
        );
        blow.angle = target_angle;
        blow.origin_in_arc = place::between(blow.arc_end, blow.arc_start, target_angle);
        if blow.kind == 0 {
            blow.sweeping = false;
        }
        let dir_to_me = -flat.normalize_or_zero();
        let spot = place::Spot {
            other_reach_cm: reach_of(ai.target, &blades) * 100.0,
            my_reach_cm: reach_of(entity, &blades) * 100.0,
            other_size: target_fighter.map_or(1.0, |f| f.stats.size),
            my_step_cm: fighter.stats.step_reach(fighter.stats.encumbrance) * 100.0,
            other_large: target_fighter.is_some_and(|f| f.stats.rules.has(1)),
            melee: false,
            fight: true,
            feral: fighter.stats.feral,
            dull: fighter.stats.rules.has(4),
            blow,
            wide: false,
            observer_ahead: Vec3::new(order.heading.sin(), 0.0, order.heading.cos()).dot(dir_to_me),
            other_ahead: target_forward.dot(dir_to_me),
            in_attack: *condition == Condition::ATTACKING,
            presses: false,
            other_skill: 0.0,
        };
        if ai.holds_ground {
            ai.footwork = place::Footwork::Hold;
        } else if ai.hold_ticks > 0 {
            ai.hold_ticks -= 1;
        } else {
            let chosen = place::choose_footwork(dist * 100.0, spot);
            if chosen != ai.footwork {
                ai.footwork = chosen;
                // A queued step resets the switching weight to four, or to
                // eight for a retreat (the original's two reset values).
                ai.hold_ticks = if chosen == place::Footwork::Retreat {
                    8
                } else {
                    4
                };
            }
        }

        ai.cooldown -= time.delta_secs();
        if attacking.is_some() || !anim.looping {
            continue;
        }
        match ai.footwork {
            place::Footwork::Approach if !ai.holds_ground => {
                body.desired_velocity = order.velocity;
                anim.play_loop(COMBAT_WALK);
                continue;
            }
            place::Footwork::Retreat if !ai.holds_ground => {
                body.desired_velocity = -order.velocity * 0.8;
                anim.play_loop("Backpedal");
                continue;
            }
            place::Footwork::CircleLeft | place::Footwork::CircleRight if !ai.holds_ground => {
                let dir = flat.normalize_or_zero();
                let sign = if ai.footwork == place::Footwork::CircleLeft {
                    1.0
                } else {
                    -1.0
                };
                body.desired_velocity =
                    Vec3::new(dir.z, 0.0, -dir.x) * sign * order.velocity.length() * 0.6;
                anim.play_loop(COMBAT_WALK);
                continue;
            }
            _ => {}
        }
        // Standing: its stance starts at once, as the original's AI starts
        // it whenever it is free.
        stance::take_ai_stance(&mut anim, *stance, primary_blend);
        // Its request goes through the original's executor: enough energy
        // and the cooldown since its last attack's last tick (it never
        // releases a swing, so neither the overhead after a release nor
        // Feint).
        let (energy, most) = ragdoll
            .and_then(|r| r.drive.as_ref())
            .map_or((MAX_ENERGY, MAX_ENERGY), |d| (d.energy, d.max_energy()));
        let busy = Busy {
            now_ms: now,
            last_tick_ms: attacker.last_tick_ms,
            cooldown_ms: attacker.cooldown_ms(entity, blades.iter()),
            playing: None,
            released: swinging.has(swing::flags::RELEASED),
            stall_until_ms: swinging.end_ms,
            renew_ms: swinging.renew_ms,
            side_started_ms: attacker.side_started_ms,
            feint: fighter.feint(),
        };
        let ready = attack::has_energy(energy, most)
            && attack::execute(attack::Request::default(), busy).is_some();
        if dist <= STRIKE_RANGE && !ai.holds_ground && ai.cooldown <= 0.0 && ready {
            let reach_cm = reach_of(entity, &blades) * 100.0;
            let class = weapon_class_of(entity, &blades);
            let choice = fight::choose_attack(
                &fight::Situation {
                    unarmed: class == 0x80,
                    open_hands: class & 0x30 == 0,
                    distance_cm: dist * 100.0,
                    reach_cm,
                    energy,
                    energy_max: most,
                    ..fight::Situation::default()
                },
                &mut ai.roll,
            );
            let Some(choice) = choice else {
                continue;
            };
            let strike = choice.motion;
            ai.next_strike += 1;
            ai.cooldown = STRIKE_INTERVAL;
            exa_combat::start_attack(
                &mut commands,
                entity,
                &mut anim,
                &mut attacker,
                &mut swinging,
                blades.iter(),
                AttackStart {
                    remise: fighter.remise(),
                    ..AttackStart::of(strike, weapon_class_of(entity, &blades), now)
                },
                stance.action(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unarmed duellist (reach 0.5 m: it guards from 2 m) on a root at
    /// the origin, its target `distance` m away along +Z.
    fn duel(distance: f32, holds_ground: bool) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>().add_systems(Update, duellist);
        let target = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, distance)),
                Fighter::default(),
            ))
            .id();
        let root = app
            .world_mut()
            .spawn((CharacterBody::default(), GlobalTransform::IDENTITY))
            .id();
        let mut ai = Duellist::new(target);
        ai.holds_ground = holds_ground;
        let foe = app
            .world_mut()
            .spawn((
                ai,
                Transform::default(),
                GlobalTransform::IDENTITY,
                Animator::new("Default", "Idle"),
                Fighter::default(),
                ChildOf(root),
            ))
            .id();
        (app, foe, root)
    }

    fn state(app: &App, foe: Entity) -> (Stance, String) {
        let w = app.world();
        (
            *w.get::<Stance>(foe).unwrap(),
            w.get::<Animator>(foe).unwrap().action.clone(),
        )
    }

    fn engage(app: &mut App, foe: Entity) {
        app.world_mut().get_mut::<Duellist>(foe).unwrap().engaged = true;
    }

    /// The perception pass fills a contact for the target: seen, its
    /// distance and its closeness.
    #[test]
    fn the_duellist_keeps_a_contact_with_its_target() {
        let (mut app, foe, _) = duel(2.0, false);
        engage(&mut app, foe);
        app.update();
        app.update();
        let target = app.world().get::<Duellist>(foe).unwrap().target;
        let combatant = app.world().get::<Combatant>(foe).unwrap();
        assert_eq!(combatant.target, Some(target));
        let contact = combatant.contact(target).unwrap();
        assert!(contact.tracked());
        assert_ne!(contact.flags & combatant::CONTACT_NOTICED, 0);
        assert!((contact.distance_cm - 200.0).abs() < 0.1);
        assert!((contact.closeness - (1.0 - 200.0 / 1200.0)).abs() < 1e-5);
        // Facing each other 2 m apart: the observer's view weight is 1.
        assert!((contact.view - 1.0).abs() < 1e-5, "{}", contact.view);
    }

    /// A chosen footwork is held for its timer before the next decision.
    #[test]
    fn the_footwork_is_held_between_decisions() {
        let (mut app, foe, _) = duel(2.0, false);
        engage(&mut app, foe);
        app.update();
        app.world_mut().get_mut::<Duellist>(foe).unwrap().hold_ticks = 2;
        app.update();
        assert_ne!(state(&app, foe).1, "Combat Walk");
        app.update();
        assert_ne!(state(&app, foe).1, "Combat Walk");
        app.update();
        assert_eq!(state(&app, foe).1, "Combat Walk");
    }

    /// On the strike plateau (1.20 m, inside reach 50 cm plus a step of
    /// 80 cm) it stands. At 2 m it is past that plateau and walks in.
    #[test]
    fn a_duellist_stands_on_the_strike_plateau() {
        let (mut app, foe, root) = duel(1.20, false);
        engage(&mut app, foe);
        app.update();
        app.update();
        assert_ne!(state(&app, foe).1, "Combat Walk");
        assert_eq!(
            app.world()
                .get::<CharacterBody>(root)
                .unwrap()
                .desired_velocity,
            Vec3::ZERO
        );
        let (mut app, foe, root) = duel(2.0, false);
        engage(&mut app, foe);
        app.update();
        app.update();
        assert_eq!(state(&app, foe).1, "Combat Walk");
        assert!(
            app.world()
                .get::<CharacterBody>(root)
                .unwrap()
                .desired_velocity
                .length()
                > 0.0
        );
    }

    /// It stands idle until the fight starts, then takes the guard; far
    /// from its target (8 m, beyond four reaches) it keeps the guard and
    /// walks in with the interim footwork.
    #[test]
    fn the_duellist_enters_combat_in_its_guard() {
        let (mut app, foe, root) = duel(8.0, false);
        app.update();
        assert_eq!(state(&app, foe), (Stance::Idle, "Idle".into()));
        engage(&mut app, foe);
        app.update();
        assert_eq!(state(&app, foe), (Stance::Defence, "Defence stance".into()));
        let combatant = app.world().get::<Combatant>(foe).unwrap();
        assert!(combatant.in_combat);
        assert_eq!(combatant.reach_cm, 50.0);
        app.update();
        assert_eq!(state(&app, foe), (Stance::Defence, "Combat Walk".into()));
        let v = app
            .world()
            .get::<CharacterBody>(root)
            .unwrap()
            .desired_velocity;
        let order = follow(&[Vec3::ZERO, Vec3::new(0.0, 0.0, 8.0)], Vec3::ZERO, 0.0, 0);
        assert!(order.velocity.length() > 1.0, "{}", order.velocity);
        assert!(
            (v - order.velocity).length() < 1e-4,
            "{v} vs {}",
            order.velocity
        );
    }

    /// A contact it has grown hostile to is a fight: the same guard the
    /// engaged duellist takes, reached from the perception tick.
    #[test]
    fn a_hostile_contact_makes_the_duellist_fight() {
        let (mut app, foe, _) = duel(8.0, false);
        let mut mind = Mind::new(-100.0);
        for _ in 0..20_000 {
            tick(&mut mind);
        }
        assert_eq!(decide(&mind), Decision::Engage);
        app.world_mut().entity_mut(foe).insert(mind);
        app.update();
        assert_eq!(state(&app, foe).0, Stance::Defence);
    }

    /// Standing its ground it shows its stance: the guard at 8 m, the
    /// combat idle at 1.5 m (under four reaches), and it never strikes.
    #[test]
    fn a_still_duellist_shows_its_stance_by_distance() {
        let (mut app, foe, root) = duel(8.0, true);
        engage(&mut app, foe);
        app.update();
        app.update();
        assert_eq!(state(&app, foe), (Stance::Defence, "Defence stance".into()));
        let (mut near, foe2, root2) = duel(1.0, true);
        engage(&mut near, foe2);
        near.world_mut().get_mut::<Duellist>(foe2).unwrap().cooldown = 0.0;
        for _ in 0..3 {
            near.update();
        }
        assert_eq!(
            state(&near, foe2),
            (Stance::CombatIdle, "Cmbt - Idle".into())
        );
        assert!(near.world().get::<Attacking>(foe2).is_none());
        for (app, root) in [(&app, root), (&near, root2)] {
            let v = app
                .world()
                .get::<CharacterBody>(root)
                .unwrap()
                .desired_velocity;
            assert_eq!(v, Vec3::ZERO);
        }
    }

    /// Within striking distance it strikes once its cooldown runs out, the
    /// attack ending in its stance.
    #[test]
    fn the_duellist_strikes_into_its_stance() {
        let (mut app, foe, _) = duel(1.0, false);
        engage(&mut app, foe);
        app.update();
        app.update();
        assert!(app.world().get::<Attacking>(foe).is_none());
        app.world_mut().get_mut::<Duellist>(foe).unwrap().cooldown = 0.0;
        app.update();
        let anim = app.world().get::<Animator>(foe).unwrap();
        let mut roll = 0;
        let choice = fight::choose_attack(
            &fight::Situation {
                unarmed: true,
                distance_cm: 100.0,
                reach_cm: stance::UNARMED_REACH * 100.0,
                ..fight::Situation::default()
            },
            &mut roll,
        )
        .unwrap();
        assert_eq!(anim.action, choice.motion.action());
        assert_eq!(anim.then.as_deref(), Some("Cmbt - Idle"));
        assert!(app.world().get::<Attacking>(foe).is_some());
    }

    /// Its strikes go through the original's executor: none under 80 % of
    /// its most energy (0.79), and with 0.8 none until 400 ms (no primary
    /// striker yet) after its last attack tick on the game clock.
    #[test]
    fn the_duellist_waits_for_its_energy_and_cooldown() {
        let (mut app, foe, _) = duel(1.0, false);
        app.insert_resource(BodyTicks(1000)); // 12 000 ms
        engage(&mut app, foe);
        app.update();
        app.update();
        let part = exa_physics::body::PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: bevy::math::Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.08),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
        };
        let mut ragdoll = Ragdoll::new(&[part]).with_drive();
        ragdoll.drive.as_mut().unwrap().energy = 0.79;
        app.world_mut().entity_mut(foe).insert(ragdoll);
        app.world_mut().get_mut::<Duellist>(foe).unwrap().cooldown = 0.0;
        app.update();
        assert!(app.world().get::<Attacking>(foe).is_none());
        let mut e = app.world_mut().entity_mut(foe);
        e.get_mut::<Ragdoll>()
            .unwrap()
            .drive
            .as_mut()
            .unwrap()
            .energy = 0.8;
        e.get_mut::<Attacker>().unwrap().last_tick_ms = Some(11_700);
        app.update();
        assert!(app.world().get::<Attacking>(foe).is_none());
        app.world_mut().resource_mut::<BodyTicks>().0 = 1009; // 12 108 ms
        app.update();
        assert!(app.world().get::<Attacking>(foe).is_some());
    }
}
