//! The player behaviour's tick before its input: the contact list, the
//! late-swing check on the strongest engaged enemy, and the practice it
//! pays. Rules: `docs/PLAYER.md` (The behaviour's aim and contact rules).
//!
//! Each behaviour tick, as the original's does before it polls the input:
//!
//! - **Contacts.** The player's contact list is rebuilt over every other
//!   character within 12 m ([`player_behaviour::contacts`]).
//! - **Late-swing check.** While the player attacks and its swing is past
//!   0.6 of its clock, once a swing, the strongest engaged enemy in front
//!   among the contacts ([`player_behaviour::strongest_engaged`]: within
//!   the player's reach plus 70 cm, in front of its heading, its own
//!   concern for the player over 2) gives the practice its fighting
//!   estimate as the lesson when larger, and marks the swing found or none
//!   ([`player_behaviour::late_swing_check`]).
//! - **Practice.** Then, unless the player is collapsing or dead, one
//!   practice tick ([`exa_ai::combat::practise`]): a lesson above 0.005
//!   feeds the gain, and a gain over 10 pays four per whole point to the
//!   record's practice count.
//!
//! The player has no record in Exarust, so the paid practice is kept on
//! [`PlayerPractice::paid`] and logged; nothing converts it yet.

use bevy::prelude::*;
use exa_anim::{Animator, Motions};
use exa_character::Wielded;
use exa_character::record::CharacterData;
use exa_combat::stance;
use exa_combat::swing::SwingState;
use exa_combat::{Blade, Condition, Fighter};
use exa_physics::body::locomotion::{Locomotion, Locomotor, forward};

use crate::player::PlayerBody;
use crate::player_behaviour::{self, Contact, NearFighter};

/// What the player's behaviour keeps between ticks for its contacts and
/// its practice.
#[derive(Component, Debug, Default, Clone)]
pub struct PlayerPractice {
    /// The lesson, the gain and the fatigue.
    pub practice: exa_ai::combat::Practice,
    /// The record's practice count this behaviour paid (Exarust's own
    /// store: the player has no record).
    pub paid: i64,
    /// The contact list of the last tick, at most twelve.
    pub contacts: Vec<Contact>,
}

/// Centimetres a metre: the behaviour measures in the original's units.
const CM: f32 = 100.0;

/// The body's reach (cm), as the behaviour publishes it: its longest armed
/// held item's, else the unarmed reach.
fn reach_cm(fighter: Entity, blades: &Query<&Blade>) -> f32 {
    blades
        .iter()
        .filter(|b| b.wielder == fighter && b.record.armed())
        .map(|b| b.record.reach_metres())
        .reduce(f32::max)
        .unwrap_or(stance::UNARMED_REACH)
        * CM
}

/// A character's fighting estimate, the threat the late check keeps the
/// largest of: from its record's skills and combat skill (none without a
/// record), its held weapon's stat bytes times its reach, and its scale
/// ([`crate::experience::fighting_estimate`]).
fn estimate(data: Option<&CharacterData>, weapon: Option<&Wielded>, reach: f32, scale: f32) -> f32 {
    let skills = data.map(|d| d.record.skills).unwrap_or_default();
    let combat_skill = data.map_or(0.0, |d| d.record.combat_skill);
    let weapon = weapon
        .and_then(|w| w.weapon.as_ref())
        .map(|w| (&w.stats, reach));
    crate::experience::fighting_estimate(&skills, combat_skill, weapon, scale)
}

/// The player behaviour's contact list, late-swing check and practice
/// tick, each behaviour tick before the attack request.
#[allow(clippy::type_complexity)]
pub(crate) fn tick_player_practice(
    motions: Option<Res<Motions>>,
    blades: Query<&Blade>,
    mut players: Query<(
        Entity,
        Option<&ChildOf>,
        &PlayerBody,
        &mut PlayerPractice,
        &GlobalTransform,
        (&Locomotion, &Locomotor),
        Option<&Condition>,
        Option<&mut SwingState>,
        Option<&Animator>,
    )>,
    others: Query<
        (
            Entity,
            &GlobalTransform,
            &Fighter,
            Option<&Condition>,
            Option<&exa_ai::combatant::Combatant>,
            Option<&CharacterData>,
            Option<&Wielded>,
        ),
        Without<PlayerBody>,
    >,
) {
    for (me, root, pb, mut kept, gt, (req, body), condition, swing, anim) in &mut players {
        let condition = condition.copied().unwrap_or_default();
        let at = gt.translation() * CM;
        let facing = forward(body.yaw);
        let cursor = forward(pb.aim);
        kept.contacts = player_behaviour::contacts(
            at,
            facing,
            cursor,
            others
                .iter()
                .map(|(e, o, ..)| (e.to_bits(), o.translation() * CM)),
        );
        if let Some(mut swing) = swing {
            let clock = anim
                .zip(motions.as_deref())
                .and_then(|(a, m)| m.clip(a, &a.action).map(|c| a.clip_progress(c)))
                .unwrap_or(0.0);
            let contacts = &kept.contacts;
            let mut lesson = kept.practice.lesson;
            let flags = player_behaviour::late_swing_check(
                condition == Condition::ATTACKING,
                swing.flags,
                clock,
                &mut lesson,
                || {
                    let near: Vec<NearFighter> = contacts
                        .iter()
                        .filter_map(|c| others.get(Entity::from_bits(c.body)).ok())
                        .map(|(e, o, f, cond, combatant, data, weapon)| {
                            let concern = combatant.and_then(|c| {
                                c.contacts
                                    .iter()
                                    .find(|k| {
                                        k.character == me
                                            || root.is_some_and(|r| k.character == r.parent())
                                    })
                                    .map(|k| k.concern)
                            });
                            let reach = reach_cm(e, &blades);
                            NearFighter {
                                at: o.translation() * CM,
                                threat: estimate(data, weapon, reach, f.stats.scale),
                                player_score: concern,
                                skipped: cond.is_some_and(|c| c.fallen_or_dying()),
                            }
                        })
                        .collect();
                    player_behaviour::strongest_engaged(
                        at,
                        forward(req.heading),
                        reach_cm(me, &blades),
                        &near,
                    )
                },
            );
            if flags != swing.flags {
                let found = flags & exa_combat::swing::flags::ENGAGED_FOE != 0;
                debug!(
                    "late swing at {clock:.2}: {} engaged enemy in front, lesson {:.3} -> {lesson:.3}",
                    if found { "an" } else { "no" },
                    kept.practice.lesson,
                );
                swing.flags = flags;
            }
            kept.practice.lesson = lesson;
        }
        if condition.dying() {
            continue;
        }
        let paid = exa_ai::combat::practise(&mut kept.practice);
        if paid != 0 {
            kept.paid += i64::from(paid);
            info!(
                "practice: +{paid} to the practice count ({} in all; lesson {:.3}, fatigue {:.4})",
                kept.paid, kept.practice.lesson, kept.practice.fatigue
            );
        }
    }
}

/// The lessons the AI's hit reactions offer the player: a blow that marks
/// the player's swing gives the struck character's combat rating as the
/// lesson when it is larger ([`exa_ai::struck::Taught`]).
pub(crate) fn take_lessons(
    mut taught: MessageReader<exa_ai::struck::Taught>,
    mut players: Query<&mut PlayerPractice>,
) {
    for t in taught.read() {
        if let Ok(mut kept) = players.get_mut(t.attacker) {
            let before = kept.practice.lesson;
            exa_ai::combatant::raise_lesson(&mut kept.practice.lesson, t.rating);
            debug!(
                "the struck teach the player: lesson {before:.3} -> {:.3}",
                kept.practice.lesson
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A struck character's rating raises the player's lesson only when it
    /// is larger.
    #[test]
    fn a_struck_character_teaches_the_player_a_larger_lesson() {
        let mut app = App::new();
        app.add_message::<exa_ai::struck::Taught>()
            .add_systems(Update, take_lessons);
        let mut kept = PlayerPractice::default();
        kept.practice.lesson = 2.0;
        let player = app.world_mut().spawn(kept).id();
        for rating in [1.0, 3.5] {
            app.world_mut().write_message(exa_ai::struck::Taught {
                attacker: player,
                rating,
            });
        }
        app.update();
        let kept = app.world().get::<PlayerPractice>(player).unwrap();
        assert_eq!(kept.practice.lesson, 3.5);
    }

    /// A fighter without a record or weapon is estimated at 2 (no skill
    /// levels, no combat skill), quartered at a human's scale: 0.5.
    #[test]
    fn an_unrecorded_fighter_teaches_half_a_point() {
        assert_eq!(estimate(None, None, 50.0, 1.0), 0.5);
        // A large body adds 2 instead.
        assert_eq!(estimate(None, None, 50.0, 1.5), 4.0);
    }

    /// One engaged swing's lesson of 0.5 decays by 0.995 a tick while it
    /// feeds the gain by a tenth of it: a single lesson never pays, as the
    /// gain stays under 10.
    #[test]
    fn one_lesson_alone_never_pays() {
        let mut p = exa_ai::combat::Practice {
            lesson: 0.5,
            ..Default::default()
        };
        let mut paid = 0;
        for _ in 0..4000 {
            paid += exa_ai::combat::practise(&mut p);
        }
        assert_eq!(paid, 0);
        assert!(p.gain < 10.0 && p.gain > 9.0, "{p:?}");
    }
}
