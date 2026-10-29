//! Held items struck as bodies: a striking weapon that meets a held shield
//! or a held weapon in the pair stage, as the original's striking part
//! classifies its contacts.
//!
//! - **The contact.** The pair stage hands each pair of an active striker
//!   (its strike-chain bit, the original's 0x10000, set while its owner's
//!   strike window is open) and a hittable part its hit event
//!   ([`exa_physics::body::Pairs::hit_events`]); the classifier runs on both
//!   sides ([`exa_physics::part::classify`]). It needs the striking part's
//!   window and chain bits, a struck body of another character, and the
//!   point on the striking side ([`exa_physics::part::faces`]).
//! - **The kind.** A held shield (held bit 2) is a block, refused once the
//!   swing's shield bit (0x200) is set. A held weapon (bit 1) is a clash,
//!   once a swing (0x100) unless its holder parries, or counters this very
//!   motion with a weapon other than its primary. A limb is the blade
//!   sweep's wound ([`crate::Hit`] with [`crate::Struck::Limb`]), not
//!   resolved here.
//! - **One pending contact.** The striking part keeps one contact a tick
//!   for the damage routine; a clash or a block replaces one recorded
//!   earlier in the tick, so the last accepted wins. It waits as
//!   [`crate::Attacking::held_pending`] until the damage routine deals it
//!   ([`crate::Hit`] with [`crate::Struck::Shield`] or
//!   [`crate::Struck::Weapon`]), and a block on a striker under the large
//!   size also keeps the sweep off its holder for the rest of the swing (its
//!   window shuts in the damage routine).
//! - **The block's effect** is the damage routine's kind-3 branch
//!   ([`crate::damage::block`]) on the holder's [`ShieldHold`]: the arm's
//!   stamina, the leak into yellow, the daze, the sag and lock of a
//!   parrying holder, the striker's recoil and the shield's sound clock.
//!
//! [guess] A clash's or block's shove lands on the struck item's hand (the
//! shield's left hand; a weapon's grip hand by its held slot): the
//! original sets the holder's sag and daze, not a part's push.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_physics::body::{Pairs, Ragdoll, SolverStages};
use exa_physics::grip::{HeldGrip, HeldKind};
use exa_physics::part::{Contact, ContactKind, PartState, classify, faces};

use crate::attack::{Attacker, Slot};
use crate::damage::ShieldGuard;
use crate::swing::{LARGE_STRIKER, SwingState};
use crate::{Attacking, Blade, Fighter, Hit, Motions, PowerFactors, Struck};

/// The hand bone a shield hangs from (the left hand): the bone a blow on
/// the shield is told against, and the limb the classifier never wounds on
/// its holder.
pub const SHIELD_HAND_BONE: usize = 18;
/// The right hand's bone, the first held slot's.
pub const MAIN_HAND_BONE: usize = 17;

/// The shield a character carries in the off hand and the block factors its
/// equip wrote on the holder ([`crate::damage::shield_guard`]), with the
/// arm's stamina the blocks spend and the body tick refills. The original
/// blocks only a blow that strikes the shield itself ([`strike_held`]);
/// when the shield leaves the world or its holder lets it go, the marker is
/// removed.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ShieldHold {
    /// The shield's item body (the entity carrying its [`HeldGrip`]).
    pub item: Entity,
    /// The guard the shield gives a blow that strikes it.
    pub guard: ShieldGuard,
}

/// The hand bone a held item's blow is told against: the shield's left
/// hand, or a weapon's hand by its held slot.
pub fn struck_bone(struck: Struck, slot: Option<Slot>) -> usize {
    match (struck, slot) {
        (Struck::Shield { .. }, _) | (_, Some(Slot::Second)) => SHIELD_HAND_BONE,
        _ => MAIN_HAND_BONE,
    }
}

/// A held body's place in the pair stage's contacts: its root entity, its
/// holder's body key, whether it is a shield, and the blade it is.
#[derive(Clone, Copy)]
struct HeldBody {
    root: Entity,
    holder: Entity,
    shield: bool,
    slot: Option<Slot>,
}

/// A contact the striking part accepted this tick.
struct Accepted {
    striker: Entity,
    target: Entity,
    point: Vec3,
    struck: Struck,
    bone: usize,
}

/// The original's striking part on the pair stage's hit events: each side
/// that is a held weapon of an attacking character classifies the other;
/// a held shield gives a block, a held weapon a clash. The last accepted
/// contact of each attacker this tick is sent as its [`Hit`], unless one
/// still waits for the damage routine. A clash's once-a-swing bit is set
/// at once, as the classifier does.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn strike_held(
    stages: Option<Res<SolverStages>>,
    motions: Option<Res<Motions>>,
    held: Query<(Entity, &Ragdoll, Option<&HeldGrip>, Option<&Blade>)>,
    blades: Query<(&Blade, &GlobalTransform)>,
    mut attackers: Query<(
        &mut Attacking,
        &mut SwingState,
        Option<&exa_anim::Animator>,
        Option<&PowerFactors>,
    )>,
    fighters: Query<(Entity, &Ragdoll, &Fighter, Option<&Attacker>)>,
    mut hits: MessageWriter<Hit>,
) {
    let Some(pairs) = stages.as_deref().and_then(|s| s.get::<Pairs>()) else {
        return;
    };
    let mut events = pairs.hit_events().peekable();
    if events.peek().is_none() {
        return;
    }
    // The held bodies and the characters by body key, gathered only on a
    // tick with hit events.
    let mut items: HashMap<Entity, HeldBody> = HashMap::default();
    for (root, body, grip, blade) in &held {
        let (Some(first), Some(holder)) = (body.parts.first(), body.held_by) else {
            continue;
        };
        items.insert(
            first.entity,
            HeldBody {
                root,
                holder,
                shield: grip.is_some_and(|g| g.kind == HeldKind::Shield),
                slot: blade.map(|b| b.slot),
            },
        );
    }
    let mut owners: HashMap<Entity, Entity> = HashMap::default();
    for (fighter, body, ..) in &fighters {
        if let Some(first) = body.parts.first() {
            owners.insert(first.entity, fighter);
        }
    }
    let mut accepted: HashMap<Entity, Accepted> = HashMap::default();
    let debug = bevy::log::tracing::enabled!(bevy::log::Level::DEBUG);
    for event in events {
        if debug {
            let what = |key: &Entity| match (items.get(key), owners.get(key)) {
                (Some(h), _) if h.shield => "a held shield",
                (Some(_), _) => "a held weapon",
                (None, Some(_)) => "a limb",
                _ => "a body",
            };
            debug!(
                "pair hit event: {} {:?} part {} and {} {:?} part {} at {}",
                what(&event.a.0),
                event.a.0,
                event.a.1,
                what(&event.b.0),
                event.b.0,
                event.b.1,
                event.point
            );
        }
        for (own, other) in [(event.a, event.b), (event.b, event.a)] {
            // Only a held weapon strikes here; a fist's and a limb's wounds
            // are the blade sweep's.
            let Some(striker) = items.get(&own.0).copied() else {
                continue;
            };
            let Some(struck) = items.get(&other.0).copied() else {
                continue;
            };
            let (Some(&attacker), Some(&victim)) =
                (owners.get(&striker.holder), owners.get(&struck.holder))
            else {
                continue;
            };
            let (Ok((_, own_body, ..)), Ok((_, other_body, ..))) =
                (held.get(striker.root), held.get(struck.root))
            else {
                continue;
            };
            let (Some(own_part), Some(other_part)) =
                (own_body.parts.get(own.1), other_body.parts.get(other.1))
            else {
                continue;
            };
            let Ok((attack, swing, ..)) = attackers.get(attacker) else {
                continue;
            };
            let Ok((_, _, defender, defender_attacker)) = fighters.get(victim) else {
                continue;
            };
            let local_y = own_part
                .transform()
                .inverse()
                .transform_point3(event.point)
                .y;
            let parent_strikes = own_part
                .parent
                .and_then(|p| own_body.parts.get(p))
                .is_some_and(|p| p.state.has(PartState::STRIKING));
            let contact = Contact {
                own: own_part.state,
                other: other_part.state,
                same_tree: striker.holder == struck.holder,
                faces: faces(local_y, attack.weapon_class & 0x30 == 0, parent_strikes),
                own_motion: attack.motion.index() as i32,
                own_flags: swing.flags,
                own_pending: 0,
                other_parrying: defender.parry.active,
                other_motion: defender.parry.counter_motion.map_or(-1, |m| m as i32),
                other_is_primary: struck.slot.is_some()
                    && defender_attacker.and_then(|a| a.primary) == struck.slot,
                other_guarded: false,
            };
            let Some(classified) = classify(&contact) else {
                debug!(
                    "held contact refused: own {:#x}, other {:#x}, same tree {}, faces {} \
                     (y {local_y:.3} m), swing flags {:#x}",
                    contact.own.bits(),
                    contact.other.bits(),
                    contact.same_tree,
                    contact.faces,
                    contact.own_flags
                );
                continue;
            };
            let kind = match classified.kind {
                ContactKind::Block if struck.shield => Struck::Shield { item: struck.root },
                ContactKind::Clash => Struck::Weapon {
                    primary: contact.other_is_primary,
                },
                _ => continue,
            };
            if let Ok((_, mut swing, ..)) = attackers.get_mut(attacker)
                && swing.flags != classified.flags
            {
                swing.flags = classified.flags;
            }
            debug!(
                "held contact: {attacker:?}'s weapon met {victim:?}'s {} ({kind:?}), y {local_y:.3} m",
                if struck.shield { "shield" } else { "weapon" }
            );
            accepted.insert(
                attacker,
                Accepted {
                    striker: striker.root,
                    target: victim,
                    point: event.point,
                    struck: kind,
                    bone: struck_bone(kind, struck.slot),
                },
            );
        }
    }
    for (attacker, contact) in accepted {
        let Ok((blade, gt)) = blades.get(contact.striker) else {
            continue;
        };
        let Ok((mut attack, _, anim, powers)) = attackers.get_mut(attacker) else {
            continue;
        };
        if attack.held_pending {
            continue;
        }
        attack.held_pending = true;
        let wielder = fighters.get(attacker).ok().map(|f| f.2);
        // A block on a striker under the large size shuts its window in
        // the damage routine: the sweep leaves the holder alone meanwhile.
        if matches!(contact.struck, Struck::Shield { .. })
            && wielder.is_some_and(|f| f64::from(f.stats.size) < LARGE_STRIKER)
        {
            attack.struck.insert(contact.target);
        }
        let progress = match (motions.as_deref(), anim) {
            (Some(motions), Some(anim)) => motions
                .clip(anim, &anim.action)
                .map_or(0.0, |c| anim.clip_progress(c)),
            _ => 0.0,
        };
        let strike =
            crate::blade_strike(blade, gt, contact.point, &attack, progress, wielder, powers);
        info!(
            "{attacker:?}'s weapon struck {:?}'s {} at {:.0} cm, {:.1} cm/tick",
            contact.target,
            match contact.struck {
                Struck::Shield { .. } => "shield: a block",
                _ => "weapon: a clash",
            },
            strike.along,
            strike.speed
        );
        hits.write(Hit {
            attacker,
            striker: contact.striker,
            target: contact.target,
            bone: contact.bone,
            point: contact.point,
            velocity: blade.tip_velocity,
            weapon: blade.record,
            strike,
            struck: contact.struck,
        });
    }
}

/// One tick of a held shield's arm on its holder's [`ShieldHold`]
/// ([`crate::regen::refill_shield`]).
pub fn refill_hold(hold: &mut ShieldHold) {
    let mut guard = crate::damage::Guard::Shield(hold.guard);
    crate::regen::refill_shield(&mut guard);
    if let crate::damage::Guard::Shield(shield) = guard {
        hold.guard = shield;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blow_on_a_held_item_lands_on_its_hand() {
        let item = Entity::PLACEHOLDER;
        assert_eq!(struck_bone(Struck::Shield { item }, None), 18);
        assert_eq!(
            struck_bone(Struck::Weapon { primary: true }, Some(Slot::First)),
            17
        );
        assert_eq!(
            struck_bone(Struck::Weapon { primary: false }, Some(Slot::Second)),
            18
        );
    }

    #[test]
    fn a_held_shields_arm_refills() {
        let mut hold = ShieldHold {
            item: Entity::PLACEHOLDER,
            guard: ShieldGuard {
                stamina: 0.5,
                rate: 0.0,
                blocked_at: 10,
                ..ShieldGuard::default()
            },
        };
        refill_hold(&mut hold);
        // The rate rises by 2e-6 in double, then the stamina by the rate.
        assert_eq!(hold.guard.rate, 2e-6_f64 as f32);
        assert_eq!(hold.guard.stamina, 0.5 + 2e-6_f64 as f32);
        assert_eq!(hold.guard.blocked_at, 10);
    }
}
