//! The dead who rise again: the AI manager's rise rule run live, the
//! original's revive of a corpse, and the undoing of Exarust's go-limp.
//!
//! - **Rule.** Every frame, each character that is in the scene, dead,
//!   whose may-rise rule is set without the bit that blocks red
//!   regeneration, and whose rise clock (written by death,
//!   [`DeathClock`]) the game clock has passed, rises when its red health
//!   widened does not compare under the double 0.01
//!   ([`exa_ai::manager::frame`]). With less red and no keep-rising mark
//!   (the race's first flag bit `0x80`) its may-rise rule clears.
//! - **Revive.** The rise runs the original's revive: the character is
//!   knocked out at once ([`exa_combat::condition::revive`]: down, the wake
//!   nibble, yellow = red × 0.5), its voice volume comes back as its body
//!   scale × the race's voice factor (`CharacterStats::voice_volume`, the
//!   single-precision product the original writes), its limbs are
//!   muscled again and the muscles turned on with the race's factors and
//!   the character's stats the body had ([`Ragdoll::revival_drive`], not
//!   the human default), empty hands are set up unarmed again (the hands'
//!   refresh), and an AI brain applies its roles again. The hands count as
//!   empty when both out-set hands of the record are
//!   ([`weapon_slots_empty`]): the collapse's drop cleared every hand but a
//!   shield's ([`exa_character::body_words::drop_collapsed_hands`],
//!   [`crate::equip::forget_collapsed_hands`]). The collapse record goes
//!   with the death clock, so a second collapse drops again.
//! - **Undo go-limp.** A body whose muscles are back while it still keeps
//!   what go-limp took ([`LimpKept`]) gets its clip player and its
//!   character's controller back, faces the way its chest lies over its
//!   pelvis (the revive's heading reset), and its controller is put under
//!   the pelvis. This also serves the story flow's revive of the player.
//!
//! Evidence: the original's AI manager frame (its rise branch: the rule
//! word masked by `0x110`, the condition `0xA00`, the global millisecond
//! clock strictly past the wake clock, the red pool against the double
//! 0.01, the race flag `0x80`), its revive routine and the glow fade it
//! calls after it (the skin material `go` faded to 1.0).
//!
//! Not reproduced: the glow fade (Exarust has no live skin-material fade:
//! it is logged), the combat-mode byte the revive clears on every body
//! (Exarust keeps no Tab combat mode on an AI body: its in-combat flag is
//! the behaviour's, which the duellist's own entry and exit drive), the
//! revive's per-limb node-turn copy, the two unknown byte
//! writes beside it, the loop sound sources (Bevy's) and the attack reach
//! refresh (Exarust computes the reach on demand). The player is left to
//! the game flow's own revive.

use bevy::prelude::*;
use exa_anim::TickedClips;
use exa_character::held::{Hands, HeldMoveset};
use exa_combat::collapse::Collapse;
use exa_combat::condition::{DeathClock, KnockOut, revive};
use exa_combat::voice::CharacterVoice;
use exa_combat::{Condition, Fighter, StateFlags};
use exa_formats::character_stats::BodyRules;
use exa_items::possession::Possessions;
use exa_physics::CharacterBody;
use exa_physics::body::{BodyTicks, OutOfScene, Ragdoll, reset_heading};

use crate::limp::LimpKept;
use crate::player::PlayerBody;

/// The glow material's parameter word the rise fades (the two characters
/// `go`), and the strength it fades it to.
pub const RISE_GLOW_MATERIAL: u16 = exa_ai::golem::GLOW_MATERIAL;
pub const RISE_GLOW: f32 = 1.0;

/// The rise rule's reading of a character: `dormant` it is out of the
/// scene; the may-rise rule is [`BodyRules::MAY_RISE`] without
/// [`BodyRules::NO_RED_REGENERATION`]; it is due once `now_ms` is strictly
/// past the rise clock (a disabled clock never is); its pool is the red
/// health; the race's first flag bit `0x80` keeps the rule.
pub fn rise_state(
    rules: BodyRules,
    condition: Condition,
    wake_at_ms: Option<u32>,
    now_ms: u32,
    red: f32,
    keeps_rising: bool,
    dormant: bool,
) -> exa_ai::manager::FrameState {
    exa_ai::manager::FrameState {
        dormant,
        may_rise: rules.0 & (BodyRules::MAY_RISE | BodyRules::NO_RED_REGENERATION)
            == BodyRules::MAY_RISE,
        dead: condition == Condition::DEAD,
        due: wake_at_ms.is_some_and(|at| now_ms > at),
        pool: red,
        keeps_rising,
    }
}

/// The revive's test for the unarmed set-up: both of the record's out-set
/// hands are empty. A collapse's drop has emptied every non-shield hand of
/// it, so a shield bearer is not set up unarmed. A body without a record
/// reads its hands, whose off hand counts a shield or a light too.
pub fn weapon_slots_empty(possessions: Option<&Possessions>, hands: Option<&Hands>) -> bool {
    match possessions {
        Some(p) => p.out.iter().all(|hand| hand.id == 0),
        None => hands.is_none_or(Hands::is_empty),
    }
}

/// The yaw (about +y, from +z) of a reset heading, none for a zero one.
pub fn heading_yaw(heading: Vec2) -> Option<f32> {
    (heading != Vec2::ZERO).then(|| heading.x.atan2(heading.y))
}

/// The AI manager's rise, once a frame over every dead body that kept what
/// go-limp took (not the player's): the rule, then the revive on a rise or
/// the may-rise rule cleared.
#[allow(clippy::type_complexity)]
pub fn rise_the_dead(
    mut commands: Commands,
    ticks: Option<Res<BodyTicks>>,
    mut bodies: Query<
        (
            Entity,
            &mut Condition,
            Option<&mut StateFlags>,
            Option<&mut KnockOut>,
            &mut Fighter,
            &mut Ragdoll,
            &DeathClock,
            Option<&mut CharacterVoice>,
            Option<&mut Hands>,
            Option<&mut exa_ai::brain::Npc>,
            Option<&exa_ai::roles::Roles>,
            Option<&Possessions>,
            Has<OutOfScene>,
        ),
        (With<LimpKept>, Without<PlayerBody>),
    >,
) {
    let now = ticks.map_or(0, |t| t.millis() as u32);
    for (
        entity,
        mut condition,
        flags,
        knock,
        mut fighter,
        mut body,
        clock,
        voice,
        hands,
        npc,
        roles,
        possessions,
        dormant,
    ) in &mut bodies
    {
        let state = rise_state(
            fighter.stats.rules,
            *condition,
            clock.wake_at_ms,
            now,
            fighter.health.red,
            fighter.first_flag_red,
            dormant,
        );
        match exa_ai::manager::frame(state) {
            exa_ai::manager::Frame::StopRising => {
                fighter.stats.rules.0 &= !BodyRules::MAY_RISE;
                info!(
                    "{entity:?} stays dead: red {:.4} under the rise's 0.01 at {now} ms",
                    fighter.health.red
                );
                continue;
            }
            exa_ai::manager::Frame::Rise => {}
            _ => continue,
        }
        let Some(drive) = body.revival_drive() else {
            debug!("{entity:?} is due to rise but its body never had muscles");
            continue;
        };
        let mut word = flags.as_deref().copied().unwrap_or_default();
        let empty = weapon_slots_empty(possessions, hands.as_deref());
        let red = fighter.health.red;
        let Some(revival) = revive(&mut condition, &mut word, red, empty, now) else {
            continue;
        };
        fighter.health.yellow = revival.yellow;
        match flags {
            Some(mut f) => *f = word,
            None => {
                commands.entity(entity).insert(word);
            }
        }
        match knock {
            Some(mut k) => *k = revival.knock,
            None => {
                commands.entity(entity).insert(revival.knock);
            }
        }
        let volume = fighter.stats.voice_volume;
        if let Some(mut voice) = voice {
            voice.volume = volume;
        }
        let (stance, scale) = (drive.race.stance, drive.stats.scale);
        body.revive(drive);
        // The unarmed set-up: the hands' refresh halves the fists' contact
        // mobility and gives them the weapon bit again.
        if revival.rearm
            && let Some(mut hands) = hands
        {
            hands.set_changed();
        }
        // An AI brain applies its roles again (the original calls its
        // controller's role hook only for an AI controller).
        if let (Some(mut npc), Some(roles)) = (npc, roles) {
            npc.roles_key = !exa_ai::brain::roles_key(roles);
        }
        // The collapse is over: a later one starts afresh (its drop and its
        // collapse-start writes run again).
        commands.entity(entity).remove::<(DeathClock, Collapse)>();
        info!(
            "{entity:?} rises at {now} ms ({} ms after its death): red {red:.3}, yellow {:.3}, \
             race stance {stance}, scale {scale}, voice volume {volume}, unarmed set-up {}",
            now.wrapping_sub(clock.died_at_ms),
            revival.yellow,
            revival.rearm
        );
        debug!(
            "{entity:?} rise: glow material {:#06x} faded to {RISE_GLOW} (not drawn: no live \
             skin-material fade)",
            RISE_GLOW_MATERIAL
        );
    }
}

/// Undoes go-limp on a body whose muscles came back (a rise, or the story
/// flow's revive): its clip player (with the held moveset its hands pick)
/// and its character's controller return, it faces the way its chest lies
/// over its pelvis, and the controller stands under its pelvis.
#[allow(clippy::type_complexity)]
pub fn restore_revived(
    mut commands: Commands,
    mut bodies: Query<(
        Entity,
        &LimpKept,
        &Ragdoll,
        Option<&HeldMoveset>,
        &mut Transform,
    )>,
    mut roots: Query<&mut Transform, Without<Ragdoll>>,
    parents: Query<&ChildOf>,
) {
    for (root, kept, body, held, mut transform) in &mut bodies {
        if body.drive.is_none() {
            continue;
        }
        let pelvis = body
            .parts
            .iter()
            .position(|p| p.bone == 1)
            .map(|i| Vec3::from(body.parts[i].transform().translation));
        let chest = body
            .support_pin_part()
            .map(|i| Vec3::from(body.parts[i].transform().translation));
        let parent_turn = parents
            .get(root)
            .ok()
            .and_then(|c| roots.get(c.parent()).ok())
            .map_or(Quat::IDENTITY, |t| t.rotation);
        if let (Some(pelvis), Some(chest)) = (pelvis, chest)
            && let Some(yaw) = heading_yaw(reset_heading(chest, pelvis))
        {
            transform.rotation = parent_turn.inverse() * Quat::from_rotation_y(yaw);
        }
        let mut entity = commands.entity(root);
        entity.remove::<LimpKept>().insert(TickedClips);
        if let Some(mut anim) = kept.animator.clone() {
            if let Some(held) = held {
                anim.config = held.config;
            }
            entity.insert(anim);
        }
        if let Some((character, controller)) = &kept.character {
            if let (Some(pelvis), Ok(mut at)) = (pelvis, roots.get_mut(*character)) {
                at.translation.x = pelvis.x;
                at.translation.z = pelvis.z;
            }
            let mut fresh = CharacterBody::default();
            fresh.radius = controller.radius;
            fresh.half_height = controller.half_height;
            commands.entity(*character).insert(fresh);
        }
        info!(
            "{root:?} revived: clips and controller back, pelvis at {:?}",
            pelvis.map(|p| p.to_array())
        );
    }
}

#[cfg(test)]
#[path = "rise_test.rs"]
mod tests;
