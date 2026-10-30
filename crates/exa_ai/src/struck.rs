//! A struck character's behaviour reacting to the blow, as the original's
//! damage routine calls it once per contact: a clash on a held weapon
//! (kind 1) or a block on a held shield (kind 2) first in its branch, a
//! wound on flesh (kind 0) after the damage call, its sounds and the
//! damage-taken notice. `exa_combat` writes the notice
//! ([`exa_combat::BlowNoticed`]) at those points and deals the behaviour
//! base's count in place ([`exa_combat::CombatScore`]); this module runs the
//! rest, in the original's order:
//!
//! 1. **The combatant base** ([`combatant::take_hit`]): unless the struck
//!    character is down or dead, a wound lowers its courage, and an
//!    attacker whose swing is not yet marked is marked and its practice
//!    takes the struck character's combat rating as its lesson when higher
//!    (a duellist's own practice here; any other attacker, the player's,
//!    through [`Taught`]).
//! 2. **The motile AI** ([`Combatant::react_to_hit`]): the attacker's
//!    contact gets its flags word 3 and, below the concern cap, its concern
//!    rises by the threat estimate ([`combatant::threat_from_nearby`] over
//!    the struck brain's nearby list) plus the kind's share.
//!
//! The base also ends a running Spirit Form or Control Mind of the struck
//! body when its state has the mind bit; Exarust's mind powers keep no such
//! bit on the body, so that call is not made (see the change's design).

use bevy::prelude::*;
use exa_combat::swing::SwingState;
use exa_combat::{Blade, BlowNoticed, Condition, stance};
use exa_physics::body::Ragdoll;

use crate::Duellist;
use crate::brain::Npc;
use crate::combatant::{self, Blow, Combatant, NearbyAim};

/// Centimetres per metre: the reaction measures in the original's units.
const CM: f32 = 100.0;

/// The hit reactions: after the combat's frame systems have dealt the
/// blows. The game applies [`Taught`] after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HitReactions;

/// A lesson the struck character's combat rating offers an attacker that
/// is not a duellist (the player): its practice takes it when higher
/// ([`combatant::raise_lesson`]).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct Taught {
    pub attacker: Entity,
    pub rating: f32,
}

/// A fighter's reach (cm), as the threat estimate reads the attacker's:
/// its longest armed held weapon's, else the unarmed reach.
fn reach_cm(fighter: Entity, blades: &Query<&Blade>) -> f32 {
    blades
        .iter()
        .filter(|b| b.wielder == fighter && b.record.armed())
        .map(|b| b.record.reach_metres())
        .reduce(f32::max)
        .unwrap_or(stance::UNARMED_REACH)
        * CM
}

/// Every blow noticed this frame, on a character with a combatant: the
/// base's courage and mark, then the AI's concern for the attacker.
#[allow(clippy::type_complexity)]
pub fn react_to_blows(
    mut blows: MessageReader<BlowNoticed>,
    mut taught: MessageWriter<Taught>,
    mut struck: Query<(&mut Combatant, Option<&mut Npc>)>,
    mut swings: Query<&mut SwingState>,
    mut duellists: Query<&mut Duellist>,
    bodies: Query<(&GlobalTransform, &Condition, Option<&Ragdoll>)>,
    blades: Query<&Blade>,
) {
    for blow in blows.read() {
        let Ok((mut combatant, mut npc)) = struck.get_mut(blow.target) else {
            continue;
        };
        let kind = blow.kind.code();
        let gate = Blow {
            kind,
            down_or_dead: blow.down,
            removed: false,
        };
        let confidence = npc.as_ref().map_or(0.0, |n| n.morale.confidence);
        let courage_before = npc.as_ref().map(|n| n.morale.courage);
        let mut swing = swings.get_mut(blow.attacker).ok();
        let lesson = combatant::take_hit(
            gate,
            npc.as_mut().map(|n| &mut n.morale.courage),
            confidence,
            swing.as_mut().map(|s| &mut s.flags),
        );
        if lesson {
            let rating = combatant.combat_rating;
            match duellists.get_mut(blow.attacker) {
                Ok(mut duellist) => {
                    combatant::raise_lesson(&mut duellist.mind.practice.lesson, rating);
                }
                Err(_) => {
                    taught.write(Taught {
                        attacker: blow.attacker,
                        rating,
                    });
                }
            }
        }
        // The threat estimate, asked for only below the concern cap: the
        // attacker's position and look against the struck brain's nearby
        // bodies' aim points [guess: the root stands in for the body's
        // anchor and the chest point for its aim point, as the duellist
        // takes them].
        let nearby: &[u64] = npc.as_ref().map_or(&[], |n| n.nearby.as_slice());
        let mut threat_used = None;
        let threat = || {
            let Ok((at, ..)) = bodies.get(blow.attacker) else {
                return 1.0;
            };
            let aims = nearby.iter().filter_map(|&id| {
                let (gt, condition, body) = bodies.get(Entity::try_from_bits(id)?).ok()?;
                Some(NearbyAim {
                    id,
                    aim_cm: stance::chest_point(gt, body) * CM,
                    dead: condition.0 == combatant::CONDITION_DEAD,
                })
            });
            let t = combatant::threat_from_nearby(
                blow.attacker.to_bits(),
                at.translation() * CM,
                at.rotation() * Vec3::Z,
                reach_cm(blow.attacker, &blades),
                aims,
            );
            threat_used = Some(t);
            t
        };
        let before = combatant.contact(blow.attacker).map(|c| c.concern);
        let concern = combatant.react_to_hit(blow.attacker, kind, threat);
        let courage_after = npc.as_ref().map(|n| n.morale.courage);
        info!(
            "hit reaction: {:?} struck by {:?} ({:?}{}): concern {:?} -> {:?} (threat {:?}), \
             courage {:?} -> {:?}, lesson {}",
            blow.target,
            blow.attacker,
            blow.kind,
            if blow.down { ", down" } else { "" },
            before,
            concern,
            threat_used,
            courage_before,
            courage_after,
            lesson
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_combat::BlowKind;

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<BlowNoticed>()
            .add_message::<Taught>()
            .add_systems(Update, react_to_blows);
        app
    }

    fn notice(app: &mut App, target: Entity, attacker: Entity, kind: BlowKind, down: bool) {
        app.world_mut().write_message(BlowNoticed {
            target,
            attacker,
            kind,
            down,
        });
        app.update();
    }

    /// The lessons offered so far, taken out.
    fn taught(app: &mut App) -> Vec<Taught> {
        let mut messages = app.world_mut().resource_mut::<Messages<Taught>>();
        messages.drain().collect()
    }

    /// A wound from the player: the brain's courage falls by the
    /// confidence factor, the player's swing is marked once and offered
    /// the brain's combat rating, and the player's contact gets the flags
    /// word 3 and the concern of the threat estimate. A body standing
    /// 100 cm along the player's look within its reach plus 100 cm takes
    /// up 0.01 of it: threat 0.99, rise (0.99 + 0.5)·(1 − 0) on a neutral
    /// contact.
    #[test]
    fn a_wound_lowers_courage_marks_the_striker_and_raises_concern() {
        let mut app = app();
        let player = app
            .world_mut()
            .spawn((
                GlobalTransform::IDENTITY,
                Condition::NORMAL,
                SwingState::default(),
            ))
            .id();
        let bystander = app
            .world_mut()
            .spawn((
                // Its chest stands over its root by the standing height:
                // put the root so the chest is 1 m ahead at the player's
                // height.
                GlobalTransform::from_translation(
                    Vec3::new(0.0, 0.0, 1.0) - Vec3::Y * stance::CHEST_HEIGHT,
                ),
                Condition::NORMAL,
            ))
            .id();
        let mut npc = Npc::new([0.0, 40.0, 0.0, 0.0]);
        npc.morale.courage = 0.7;
        npc.morale.confidence = 0.4;
        npc.nearby = vec![player.to_bits(), bystander.to_bits()];
        let mut combatant = Combatant {
            combat_rating: 12.0,
            ..Combatant::default()
        };
        combatant.contact_mut(player).flags = combatant::CONTACT_NOTICED;
        let brain = app
            .world_mut()
            .spawn((GlobalTransform::IDENTITY, Condition::NORMAL, npc, combatant))
            .id();
        notice(&mut app, brain, player, BlowKind::Wound, false);
        let npc = app.world().get::<Npc>(brain).unwrap();
        let courage = ((f64::from(1.0f32 + 0.4) * 0.025 + 0.95) * f64::from(0.7f32)) as f32;
        assert_eq!(npc.morale.courage, courage);
        let swing = app.world().get::<SwingState>(player).unwrap();
        assert_eq!(swing.flags, combatant::ATTACKER_MARK);
        assert_eq!(
            taught(&mut app),
            vec![Taught {
                attacker: player,
                rating: 12.0
            }]
        );
        let contact = app
            .world()
            .get::<Combatant>(brain)
            .unwrap()
            .contact(player)
            .unwrap()
            .clone();
        assert_eq!(contact.flags, combatant::CONTACT_CONCERN_FLAGS);
        // The bystander is 100 cm along the look (the chest stand-in lands
        // there): share 0.01. The player itself is skipped.
        let threat = 1.0f32 - 100.0 / 10_000.0;
        let rise = (f64::from(threat + 0.5) * (1.0 - 0.0) + 0.0) as f32;
        assert!((contact.concern - rise).abs() < 1e-5, "{}", contact.concern);
        // A clash next: no courage lost, no second lesson (the swing is
        // marked), the concern rises by (threat + 0.5 + 0.5).
        notice(&mut app, brain, player, BlowKind::Clash, false);
        assert_eq!(
            app.world().get::<Npc>(brain).unwrap().morale.courage,
            courage
        );
        assert!(taught(&mut app).is_empty());
        let after = app
            .world()
            .get::<Combatant>(brain)
            .unwrap()
            .contact(player)
            .unwrap()
            .concern;
        let expected = (f64::from(threat + 0.5 + 0.5) + f64::from(contact.concern)) as f32;
        assert!((after - expected).abs() < 1e-5, "{after}");
    }

    /// A struck duellist that is already down keeps its courage and marks
    /// nobody, but its concern for the attacker still rises; a duellist
    /// attacker takes the lesson in its own practice.
    #[test]
    fn a_down_body_still_worries_and_a_duellist_attacker_learns() {
        let mut app = app();
        let attacker = app
            .world_mut()
            .spawn((
                GlobalTransform::IDENTITY,
                Condition::NORMAL,
                SwingState::default(),
                Duellist::new(Entity::PLACEHOLDER),
            ))
            .id();
        let mut npc = Npc::new([0.0; 4]);
        npc.morale.courage = 0.5;
        let mut combatant = Combatant {
            combat_rating: 7.0,
            ..Combatant::default()
        };
        combatant.contact_mut(attacker).attitude = -100.0;
        let brain = app
            .world_mut()
            .spawn((GlobalTransform::IDENTITY, Condition::DOWN, npc, combatant))
            .id();
        notice(&mut app, brain, attacker, BlowKind::Wound, true);
        assert_eq!(app.world().get::<Npc>(brain).unwrap().morale.courage, 0.5);
        assert_eq!(app.world().get::<SwingState>(attacker).unwrap().flags, 0);
        // No nearby bodies: threat 1, rise (1 + 0.5)·(1 + 1) = 3.
        let concern = app
            .world()
            .get::<Combatant>(brain)
            .unwrap()
            .contact(attacker)
            .unwrap()
            .concern;
        assert_eq!(concern, 3.0);
        // Standing again, a block marks the duellist and teaches it 7.
        app.world_mut().entity_mut(brain).insert(Condition::NORMAL);
        notice(&mut app, brain, attacker, BlowKind::Block, false);
        let duellist = app.world().get::<Duellist>(attacker).unwrap();
        assert_eq!(duellist.mind.practice.lesson, 7.0);
        assert!(taught(&mut app).is_empty());
        // (1 + 0.5 + 0.25)·2 + 3.
        let concern = app
            .world()
            .get::<Combatant>(brain)
            .unwrap()
            .contact(attacker)
            .unwrap()
            .concern;
        assert_eq!(concern, 6.5);
    }
}
