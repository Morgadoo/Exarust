//! Strikes as bodies: a striking weapon or fist that meets a held shield,
//! a held weapon, a fist or a limb in the pair stage, as the original's
//! striking part classifies its contacts (the part's hit-event slot).
//!
//! - **The contact.** The pair stage hands each pair of an active striker
//!   (its strike-chain bit, the original's 0x10000, set while its owner's
//!   strike window is open) and a hittable part its hit event
//!   ([`exa_physics::body::Pairs::hit_events`]); the classifier runs on both
//!   sides ([`exa_physics::part::classify`]). It needs the striking part's
//!   window and chain bits, a struck body of another character, and the
//!   point on the striking side ([`exa_physics::part::faces`]).
//! - **The strikers.** A held weapon's body, or an unarmed fighter's own
//!   hand: the unarmed set-up gives both hands the weapon bit, and the
//!   swing's window opening gives the motion's hand the strike-chain bit
//!   ([`crate::impact::sync_fist_striking`]), as the original's swing
//!   routine writes it on its strikers. A fist's frame is its hand's, so
//!   every contact faces the striking side (the character wields nothing).
//! - **The kind.** A held shield or torch (held bit 2) is a block, refused
//!   once the swing's shield bit (0x200) is set. A held weapon (bit 1) is a clash,
//!   once a swing (0x100) unless its holder parries, or counters this very
//!   motion with a weapon other than its primary. A muscled limb of
//!   another character is a wound ([`crate::Hit`] with
//!   [`crate::Struck::Limb`]), once a target a swing, unless a clash or
//!   block is pending or it is the hand holding the victim's shield: a held
//!   weapon's body and a fist wound only where they touch. A striking fist
//!   is struck as a held weapon (its hand's weapon bit): a clash. A held
//!   model with no body, and a fist on no body part, keep the blade sweep.
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
//!   A torch has no block factors of its own: its holder blocks with the
//!   factors its last shield wrote, none for Exarust ([`guard_for`]).
//! - **The rings.** The first block of a swing rings the struck item's own
//!   material ([`ShieldRung`], [`shield_ring`]).
//!
//! [guess] A clash's or block's shove lands on the struck item's hand (the
//! shield's left hand; a weapon's grip hand by its held slot): the
//! original sets the holder's sag and daze, not a part's push.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_physics::body::pairs::HitEvent;
use exa_physics::body::{Pairs, Ragdoll, SolverStages};
use exa_physics::grip::{HeldGrip, HeldKind};
use exa_physics::part::{Contact, ContactKind, PartState, classify, faces};

use crate::attack::{Attacker, Slot};
use crate::damage::{ShieldGuard, Strike};
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

/// The block factors a holder meets a blow on its held `item` with: its
/// shield's ([`ShieldHold`]) when that item is the shield, else (a torch)
/// the bare arm: no parry factor, leak or cost, and a full arm. The
/// original's block reads the holder's shield words whatever was struck;
/// they stay as the last shield equip wrote them and are zero on a holder
/// that never held one. [deviation: Exarust drops them with the shield]
pub fn guard_for(hold: Option<&ShieldHold>, item: Entity) -> ShieldGuard {
    match hold {
        Some(hold) if hold.item == item => hold.guard,
        _ => ShieldGuard {
            parry: 0.0,
            leak: 0.0,
            cost: 0.0,
            stamina: 1.0,
            rate: 0.0,
            blocked_at: crate::regen::ShieldStamina::STAMP_FULL,
            parrying: false,
        },
    }
}

/// The first block of a swing rings the struck shield's or torch's own
/// material (the collision sound set its body registered with) at the
/// blow: the item, the contact point, and the ring's volume and pitch
/// ([`shield_ring`]).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct ShieldRung {
    /// The struck item's body (its root part rings).
    pub item: Entity,
    pub point: Vec3,
    pub volume: f32,
    pub pitch: f32,
}

/// The volume and pitch a block's ring plays at from the blow's hit sound
/// ([`crate::damage::Outcome::volume`] and its pitch): the volume times
/// 1.5 in single precision, the pitch widened and times the double 1.8.
pub fn shield_ring(volume: f32, pitch: f32) -> (f32, f32) {
    (volume * 1.5, (f64::from(pitch) * 1.8) as f32)
}

/// A held body's place in the pair stage's contacts: its root entity, its
/// holder's body key, whether it blocks (a shield or a torch) and whether
/// it is a shield, and the blade it is.
#[derive(Clone, Copy)]
struct HeldBody {
    root: Entity,
    holder: Entity,
    blocks: bool,
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
    /// The contact in the striking part's frame (m): where along a held
    /// weapon it lies is read from it, placed on the striker's frame.
    local: Vec3,
    /// The striking part is the wielder's own hand (a fist or a claw): the
    /// striker's frame is that hand's, so the contact is read where it is.
    fist: bool,
    /// The striking part's index in the held item's body (0 for a fist).
    part: usize,
}

/// A fist carried by a part of its wielder's body: the hand part strikes
/// as the original's unarmed striker does (its weapon bit from the unarmed
/// set-up, its strike-chain bit while its window is open,
/// [`crate::impact::sync_fist_striking`]).
#[derive(Clone, Copy)]
struct FistPart {
    /// The fist's striker entity (its [`Blade`]).
    blade: Entity,
    wielder: Entity,
    slot: Slot,
}

/// The striking side of a hit event: a held body of a holder, or a
/// fighter's own hand.
#[derive(Clone, Copy)]
enum StrikingSide {
    Held(HeldBody),
    Fist(FistPart),
}

/// The struck side of a hit event: a held body (a shield, torch or weapon)
/// or a part of a fighter's body, with the fist it carries if it is a
/// striking hand.
#[derive(Clone, Copy)]
enum StruckSide {
    Held(HeldBody),
    Limb {
        victim: Entity,
        fist: Option<FistPart>,
    },
}

/// The bodies and characters [`strike_events`] reads and writes.
#[derive(bevy::ecs::system::SystemParam)]
#[allow(clippy::type_complexity)]
pub struct StrikeParams<'w, 's> {
    motions: Option<Res<'w, Motions>>,
    held: Query<
        'w,
        's,
        (
            Entity,
            &'static Ragdoll,
            Option<&'static HeldGrip>,
            Option<&'static Blade>,
        ),
    >,
    blades: Query<'w, 's, (&'static Blade, Option<&'static GlobalTransform>)>,
    fists: Query<'w, 's, (Entity, &'static Blade, &'static ChildOf), Without<Ragdoll>>,
    attackers: Query<
        'w,
        's,
        (
            &'static mut Attacking,
            &'static mut SwingState,
            Option<&'static exa_anim::Animator>,
            Option<&'static PowerFactors>,
        ),
    >,
    fighters: Query<
        'w,
        's,
        (
            Entity,
            &'static Ragdoll,
            &'static Fighter,
            Option<&'static Attacker>,
            Option<&'static crate::Condition>,
            Has<ShieldHold>,
        ),
    >,
    hits: MessageWriter<'w, Hit>,
}

/// The original's striking part on the pair stage's hit events (the part's
/// hit-event slot): each side that is an active striker, a held weapon of
/// an attacking character or an attacking fighter's own fist, classifies
/// the other; a held shield or torch gives a block, a held weapon or a
/// striking fist a clash, a muscled limb a wound. The last accepted
/// contact of each attacker this tick is sent as its [`Hit`], unless one
/// still waits for the damage routine. A clash's once-a-swing bit is set
/// at once, as the classifier does.
pub fn strike_held(stages: Option<Res<SolverStages>>, mut params: StrikeParams) {
    let Some(pairs) = stages.as_deref().and_then(|s| s.get::<Pairs>()) else {
        return;
    };
    let events: Vec<HitEvent> = pairs.hit_events().collect();
    if events.is_empty() {
        return;
    }
    strike_events(&events, &mut params);
}

/// [`strike_held`] on the given hit events.
pub fn strike_events(events: &[HitEvent], p: &mut StrikeParams) {
    // The held bodies, the fists and the characters by body key, gathered
    // only on a tick with hit events.
    let mut items: HashMap<Entity, HeldBody> = HashMap::default();
    for (root, body, grip, blade) in &p.held {
        let (Some(first), Some(holder)) = (body.parts.first(), body.held_by) else {
            continue;
        };
        items.insert(
            first.entity,
            HeldBody {
                root,
                holder,
                blocks: grip.is_some_and(|g| g.kind != HeldKind::Weapon),
                shield: grip.is_some_and(|g| g.kind == HeldKind::Shield),
                slot: blade.map(|b| b.slot),
            },
        );
    }
    let mut owners: HashMap<Entity, Entity> = HashMap::default();
    for (fighter, body, ..) in &p.fighters {
        if let Some(first) = body.parts.first() {
            owners.insert(first.entity, fighter);
        }
    }
    let mut fists: HashMap<(Entity, usize), FistPart> = HashMap::default();
    for (blade_entity, blade, parent) in &p.fists {
        if blade.record.armed() {
            continue;
        }
        let Ok((_, body, ..)) = p.held.get(blade.wielder) else {
            continue;
        };
        let (Some(first), Some(hand)) = (
            body.parts.first(),
            body.parts.iter().position(|q| q.entity == parent.parent()),
        ) else {
            continue;
        };
        fists.insert(
            (first.entity, hand),
            FistPart {
                blade: blade_entity,
                wielder: blade.wielder,
                slot: blade.slot,
            },
        );
    }
    let mut accepted: HashMap<Entity, Accepted> = HashMap::default();
    let debug = bevy::log::tracing::enabled!(bevy::log::Level::DEBUG);
    for event in events {
        if debug {
            let what = |side: &(Entity, usize)| match (items.get(&side.0), owners.get(&side.0)) {
                (Some(h), _) if h.shield => "a held shield",
                (Some(h), _) if h.blocks => "a held torch",
                (Some(_), _) => "a held weapon",
                (None, Some(_)) if fists.contains_key(side) => "a fist",
                (None, Some(_)) => "a limb",
                _ => "a body",
            };
            debug!(
                "pair hit event: {} {:?} part {} and {} {:?} part {} at {}",
                what(&event.a),
                event.a.0,
                event.a.1,
                what(&event.b),
                event.b.0,
                event.b.1,
                event.point
            );
        }
        for (own, other) in [(event.a, event.b), (event.b, event.a)] {
            // A held weapon's body, or a fighter's fist on its own body.
            let striking = match (items.get(&own.0), fists.get(&own)) {
                (Some(&held), _) => StrikingSide::Held(held),
                (None, Some(&fist)) => StrikingSide::Fist(fist),
                _ => continue,
            };
            let struck = match items.get(&other.0) {
                Some(&held) => StruckSide::Held(held),
                None => match owners.get(&other.0) {
                    Some(&victim) => StruckSide::Limb {
                        victim,
                        fist: fists.get(&other).copied(),
                    },
                    None => continue,
                },
            };
            let Some((attacker, found)) =
                side_contact(striking, own.1, struck, other.1, event.point, &owners, p)
            else {
                continue;
            };
            if found.struck == Struck::Limb {
                // A wound never replaces a contact already pending this
                // tick; a clash or a block replaces a pending wound.
                accepted.entry(attacker).or_insert(found);
            } else {
                accepted.insert(attacker, found);
            }
        }
    }
    for (attacker, contact) in accepted {
        let Ok((blade, scene)) = p.blades.get(contact.striker) else {
            continue;
        };
        // The striker's frame where this tick left it ([`crate::track_blades`]).
        let Some(frame) = blade.frame_or(scene) else {
            continue;
        };
        let Ok((mut attack, _, anim, powers)) = p.attackers.get_mut(attacker) else {
            continue;
        };
        if attack.held_pending {
            continue;
        }
        if contact.struck == Struck::Limb {
            // A wound: once a target a swing, as the blade sweep keeps it
            // (the wound shuts the striker's window in the damage routine).
            if !attack.struck.insert(contact.target) {
                continue;
            }
        } else {
            attack.held_pending = true;
        }
        let wielder = p.fighters.get(attacker).ok().map(|f| f.2);
        // A block on a striker under the large size shuts its window in
        // the damage routine: the sweep leaves the holder alone meanwhile.
        if matches!(contact.struck, Struck::Shield { .. })
            && wielder.is_some_and(|f| f64::from(f.stats.size) < LARGE_STRIKER)
        {
            attack.struck.insert(contact.target);
        }
        let progress = match (p.motions.as_deref(), anim) {
            (Some(motions), Some(anim)) => motions
                .clip(anim, &anim.action)
                .map_or(0.0, |c| anim.clip_progress(c)),
            _ => 0.0,
        };
        // Where on the striker: a held weapon's contact as its striking
        // part holds it, placed on the blade's frame; a fist's where the
        // hand met it (the fist's frame is the hand's).
        let on_blade = if contact.fist {
            contact.point
        } else {
            frame.transform_point3(contact.local)
        };
        let strike = Strike {
            part: contact.part,
            ..crate::blade_strike(blade, &frame, on_blade, &attack, progress, wielder, powers)
        };
        let by = if contact.fist { "fist" } else { "weapon" };
        if contact.struck == Struck::Limb {
            debug!(
                "{attacker:?}'s {by} body met {:?}'s bone {} at {:.0} cm, {:.1} cm/tick: a wound \
                 (y {:.1} cm in the striking part)",
                contact.target,
                contact.bone,
                strike.along,
                strike.speed,
                contact.local.y * 100.0
            );
        } else {
            info!(
                "{attacker:?}'s {by} struck {:?}'s {} at {:.0} cm, {:.1} cm/tick",
                contact.target,
                match contact.struck {
                    Struck::Shield { .. } => "shield: a block",
                    _ => "weapon: a clash",
                },
                strike.along,
                strike.speed
            );
        }
        p.hits.write(Hit {
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

/// One side of a hit event as the striking part classifies it
/// ([`classify`]): `striking`'s part `own` against `struck`'s part
/// `other`, met at `point`. The striker must be its attacking character's
/// active striker, the struck side another character's, and the point on
/// the striking side ([`faces`]; always for a character that wields
/// nothing). A held shield or torch is a block; a held weapon, or a fist
/// (its hand carries the weapon bit), a clash; a muscled limb a wound,
/// unless a hit waits, it is an engaged arm, or it is the hand holding the
/// victim's shield. There is no speed gate: the damage routine weighs the
/// contact's speed. A dead victim takes no wound, as the sweep skips it
/// [Exarust's: the original's dead body leaves the living-limb family,
/// which was not traced]. A clash writes the swing's once-a-swing bit at
/// once.
#[allow(clippy::too_many_arguments)]
fn side_contact(
    striking: StrikingSide,
    own: usize,
    struck: StruckSide,
    other: usize,
    point: Vec3,
    owners: &HashMap<Entity, Entity>,
    p: &mut StrikeParams,
) -> Option<(Entity, Accepted)> {
    let (attacker, own_root, striker, fist) = match striking {
        StrikingSide::Held(h) => (*owners.get(&h.holder)?, h.root, h.root, false),
        StrikingSide::Fist(f) => (f.wielder, f.wielder, f.blade, true),
    };
    let (victim, other_root, item, slot) = match struck {
        StruckSide::Held(h) => (*owners.get(&h.holder)?, h.root, Some(h.root), h.slot),
        StruckSide::Limb { victim, fist } => {
            (victim, victim, fist.map(|f| f.blade), fist.map(|f| f.slot))
        }
    };
    let (_, own_body, ..) = p.held.get(own_root).ok()?;
    let (_, other_body, ..) = p.held.get(other_root).ok()?;
    let own_part = own_body.parts.get(own)?;
    let other_part = other_body.parts.get(other)?;
    // The striking part's record, up its chain ([`crate::record_chain`]):
    // a held item's part finds its top part's (the item's Blade); a fist is
    // its own hand's. Without one the damage routine reports that it cannot
    // retrieve the weapon's values and deals nothing.
    let parents: Vec<Option<usize>> = own_body.parts.iter().map(|q| q.parent).collect();
    if !fist
        && (crate::record_chain::HeldChain {
            parents: &parents,
            hand_record: false,
        })
        .record_of(own)
        .is_none()
    {
        debug!("held contact on part {own} of {own_root:?}: unable to retrieve weapon values");
        return None;
    }
    // The struck item's part: a held item's own part, a fist none of its
    // own (its hand's frame is the fist Blade's).
    let struck_part = match struck {
        StruckSide::Held(_) => other,
        StruckSide::Limb { .. } => 0,
    };
    let (_, _, defender, defender_attacker, condition, shielded) = p.fighters.get(victim).ok()?;
    let (attack, swing, ..) = p.attackers.get(attacker).ok()?;
    let local = own_part.transform().inverse().transform_point3(point);
    let local_y = local.y;
    let parent_strikes = own_part
        .parent
        .and_then(|i| own_body.parts.get(i))
        .is_some_and(|q| q.state.has(PartState::STRIKING));
    let limb = matches!(struck, StruckSide::Limb { .. });
    let contact = Contact {
        own: own_part.state,
        other: other_part.state,
        same_tree: attacker == victim,
        faces: faces(local_y, attack.weapon_class & 0x30 == 0, parent_strikes),
        own_motion: attack.motion.index() as i32,
        own_flags: swing.flags,
        own_pending: i32::from(attack.held_pending),
        other_parrying: defender.parry.active,
        other_motion: defender.parry.counter_motion.map_or(-1, |m| m as i32),
        other_is_primary: slot.is_some() && defender_attacker.and_then(|a| a.primary) == slot,
        other_guarded: limb && shielded && other_part.bone == SHIELD_HAND_BONE,
    };
    let Some(classified) = classify(&contact) else {
        if !limb {
            debug!(
                "held contact refused: own {:#x}, other {:#x}, same tree {}, faces {} \
                 (y {local_y:.3} m), swing flags {:#x}",
                contact.own.bits(),
                contact.other.bits(),
                contact.same_tree,
                contact.faces,
                contact.own_flags
            );
        }
        return None;
    };
    let kind = match (classified.kind, struck) {
        (ContactKind::Block, StruckSide::Held(h)) if h.blocks => Struck::Shield { item: h.root },
        (ContactKind::Clash, _) => Struck::Weapon {
            primary: contact.other_is_primary,
            item: item?,
            part: struck_part,
        },
        (ContactKind::Wound, StruckSide::Limb { .. })
            if !condition.is_some_and(|c| c.is_dead()) =>
        {
            Struck::Limb
        }
        _ => return None,
    };
    if kind != Struck::Limb {
        if let Ok((_, mut swing, ..)) = p.attackers.get_mut(attacker)
            && swing.flags != classified.flags
        {
            swing.flags = classified.flags;
        }
        debug!(
            "held contact: {attacker:?}'s {} met {victim:?}'s {} ({kind:?}), y {local_y:.3} m",
            if fist { "fist" } else { "weapon" },
            match struck {
                StruckSide::Held(h) if h.shield => "shield",
                StruckSide::Held(h) if h.blocks => "torch",
                StruckSide::Held(_) => "weapon",
                StruckSide::Limb { .. } => "fist",
            }
        );
    }
    let bone = match kind {
        Struck::Limb => other_part.bone,
        _ => struck_bone(kind, slot),
    };
    Some((
        attacker,
        Accepted {
            striker,
            target: victim,
            point,
            struck: kind,
            bone,
            local,
            fist,
            part: if fist { 0 } else { own },
        },
    ))
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

/// Debug aid: with `EXA_HELD_TRACE` set, every 12 body ticks each held
/// weapon whose holder is attacking logs where its first part is, its
/// part state, layer and group words, whether it sleeps, and the nearest
/// part of another fighter's body (distance between part origins, and that
/// part's layer), to see why a swing meets nothing (`held-trace`).
#[allow(clippy::type_complexity)]
pub fn trace_held_weapons(
    mut tick: Local<u32>,
    held: Query<(Entity, &Ragdoll, &HeldGrip, Option<&Blade>)>,
    fighters: Query<(Entity, &Ragdoll, Has<Attacking>), With<Fighter>>,
) {
    if std::env::var_os("EXA_HELD_TRACE").is_none() {
        return;
    }
    *tick = tick.wrapping_add(1);
    if !(*tick).is_multiple_of(12) {
        return;
    }
    for (item, body, grip, blade) in &held {
        if grip.kind != HeldKind::Weapon {
            continue;
        }
        let Some((holder, holder_body, _)) = fighters.iter().find(|(_, b, attacking)| {
            *attacking && b.parts.first().map(|p| p.entity) == Some(grip.holder_key)
        }) else {
            continue;
        };
        let Some(first) = body.parts.first() else {
            continue;
        };
        let at = Vec3::from(first.transform().translation);
        // The blade as the sweep draws it: from the item's origin along its
        // y axis by the blade's length.
        let length = blade.map_or(0.0, |b| b.length);
        let tip = first.transform().transform_point3(Vec3::Y * length);
        let mut nearest: Option<(Entity, f32, u32, Vec3)> = None;
        for (other, other_body, _) in &fighters {
            if other == holder {
                continue;
            }
            let root = other_body
                .parts
                .first()
                .map_or(Vec3::ZERO, |p| Vec3::from(p.transform().translation));
            for q in &other_body.parts {
                let c = Vec3::from(q.transform().translation);
                let d = crate::closest_on_segment(at, tip, c).distance(c) * 100.0;
                if nearest.is_none_or(|n| d < n.1) {
                    nearest = Some((other, d, q.pairing.layer, root));
                }
            }
        }
        // The holder's frame forward against the way to that foe's root
        // and the blade's way, as yaws (degrees) across the ground.
        let yaw = |v: Vec3| v.x.atan2(v.z).to_degrees().round();
        let forward = holder_body
            .drive
            .as_ref()
            .and_then(|d| d.frame())
            .map(|f| Vec3::from(f.matrix3.z_axis));
        let facing = forward.map(yaw);
        let to_foe = nearest.map(|n| yaw(n.3 - at));
        let blade_way = yaw(tip - at);
        info!(
            "held-trace {holder:?} item {item:?} at {at:.2} tip {tip:.2} facing {facing:?} \
             to_foe {to_foe:?} blade {blade_way} state {:#x} layer {:#x} group {:#x} asleep {} \
             nearest {:?}",
            first.state.bits(),
            first.pairing.layer,
            first.pairing.group,
            body.asleep(),
            nearest.map(|(e, d, l, r)| (e, d.round(), format!("{l:#x} root {r:.2}")))
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Events handed to [`strike_events`] in a test.
    #[derive(Resource, Default)]
    struct Events(Vec<HitEvent>);

    fn strike(events: Res<Events>, mut p: StrikeParams) {
        strike_events(&events.0, &mut p);
    }

    /// A one-part body of `bone` at `at`, its part's entity `part` and
    /// state `state`.
    fn one_part(part: Entity, bone: usize, at: Vec3, state: u32) -> Ragdoll {
        let mut body = Ragdoll::new(&[exa_physics::body::PartSetup {
            bone,
            entity: part,
            parent: None,
            world: bevy::math::Affine3A::from_translation(at),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.05),
            spheres: None,
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
        }]);
        body.parts[0].state = PartState(state);
        body
    }

    const LIMB: u32 = PartState::LIMB | PartState::MUSCLED;

    fn fist_record() -> exa_formats::weapon_record::WeaponRecord {
        use exa_formats::weapon_record::{Holder, Lengths, Section, WeaponRecord};
        WeaponRecord::build(
            &Section::FISTS,
            false,
            Lengths::hand(0.0, 10.0),
            &Holder::AVERAGE,
        )
    }

    /// An unarmed puncher whose right hand carries `hand` and a struck
    /// fighter whose chest (or hand, `struck_bone`) carries `struck`, met in
    /// one hit event; the hits it sends and the puncher's and struck
    /// fighter's fists.
    fn punch(hand: u32, struck_bone: usize, struck: u32) -> (Vec<Hit>, Entity, Entity) {
        let mut app = App::new();
        app.add_message::<Hit>()
            .init_resource::<Events>()
            .add_systems(Update, strike);
        let point = Vec3::new(0.0, 1.4, 0.5);
        let spawn_fighter = |app: &mut App, bone: usize, state: u32, at: Vec3| {
            let part = app.world_mut().spawn_empty().id();
            let fighter = app
                .world_mut()
                .spawn((one_part(part, bone, at, state), Fighter::default()))
                .id();
            let fist = app
                .world_mut()
                .spawn((
                    ChildOf(part),
                    GlobalTransform::from_translation(at),
                    Blade::new(fighter, 0.1, fist_record()),
                ))
                .id();
            (fighter, part, fist)
        };
        let (puncher, hand_part, own_fist) =
            spawn_fighter(&mut app, MAIN_HAND_BONE, hand, point - Vec3::Z * 0.05);
        app.world_mut().entity_mut(puncher).insert((
            Attacking::new(crate::attack::AttackMotion::Main(0), 0x80),
            SwingState::default(),
        ));
        let (victim, victim_part, victim_fist) =
            spawn_fighter(&mut app, struck_bone, struck, point + Vec3::Z * 0.05);
        app.world_mut().resource_mut::<Events>().0.push(HitEvent {
            a: (hand_part, 0),
            b: (victim_part, 0),
            point,
            travel: Vec3::Z,
            push: Vec3::ZERO,
        });
        let _ = victim;
        app.update();
        let hits = app
            .world()
            .resource::<Messages<Hit>>()
            .iter_current_update_messages()
            .cloned()
            .collect();
        (hits, own_fist, victim_fist)
    }

    /// A striking fist (the weapon bit and the strike-chain bit on the
    /// hand) that meets a muscled limb of another fighter wounds it there:
    /// the hand itself is the striker, as the original's unarmed striker.
    #[test]
    fn a_striking_fist_wounds_the_limb_it_meets() {
        let (hits, fist, _) = punch(LIMB | PartState::STRIKE_FLAGS, 9, LIMB);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].striker, fist);
        assert_eq!(hits[0].struck, Struck::Limb);
        assert_eq!(hits[0].bone, 9);
    }

    /// A hand whose window is shut (no strike-chain bit) strikes nothing,
    /// and an engaged arm (class bit 8) is not wounded.
    #[test]
    fn a_fist_strikes_only_while_its_window_is_open() {
        let (hits, ..) = punch(LIMB | PartState::STRIKING, 9, LIMB);
        assert!(hits.is_empty());
        let (hits, ..) = punch(LIMB | PartState::STRIKE_FLAGS, 9, LIMB | PartState::ARM);
        assert!(hits.is_empty());
    }

    /// A fist that meets the other's fist (its hand carries the weapon
    /// bit) clashes with it: the struck item is that fist.
    #[test]
    fn a_fist_on_a_fist_is_a_clash() {
        let (hits, _, struck_fist) = punch(
            LIMB | PartState::STRIKE_FLAGS,
            MAIN_HAND_BONE,
            LIMB | PartState::STRIKING,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].struck,
            Struck::Weapon {
                primary: false,
                item: struck_fist,
                part: 0,
            }
        );
        assert_eq!(hits[0].bone, MAIN_HAND_BONE);
    }

    #[test]
    fn a_blow_on_a_held_item_lands_on_its_hand() {
        let item = Entity::PLACEHOLDER;
        assert_eq!(struck_bone(Struck::Shield { item }, None), 18);
        assert_eq!(
            struck_bone(
                Struck::Weapon {
                    primary: true,
                    item,
                    part: 0,
                },
                Some(Slot::First)
            ),
            17
        );
        assert_eq!(
            struck_bone(
                Struck::Weapon {
                    primary: false,
                    item,
                    part: 0,
                },
                Some(Slot::Second)
            ),
            18
        );
    }

    #[test]
    fn a_block_rings_at_one_and_a_half_volume_and_1_8_pitch() {
        let (volume, pitch) = shield_ring(0.5, 0.75);
        assert_eq!(volume, 0.75);
        assert_eq!(pitch, (0.75f64 * 1.8) as f32);
        // The pitch's product is the double's, then narrowed.
        let (_, pitch) = shield_ring(0.1, 0.1);
        assert_eq!(pitch, (f64::from(0.1f32) * 1.8) as f32);
    }

    #[test]
    fn a_torch_blocks_with_the_bare_arm() {
        let shield = Entity::from_raw_u32(1).unwrap();
        let torch = Entity::from_raw_u32(2).unwrap();
        let hold = ShieldHold {
            item: shield,
            guard: ShieldGuard {
                parry: 1.0,
                leak: 0.5,
                cost: 0.5,
                stamina: 0.25,
                ..ShieldGuard::default()
            },
        };
        assert_eq!(guard_for(Some(&hold), shield), hold.guard);
        let bare = guard_for(Some(&hold), torch);
        assert_eq!(guard_for(None, torch), bare);
        assert_eq!((bare.parry, bare.leak, bare.cost), (0.0, 0.0, 0.0));
        assert_eq!(bare.stamina, 1.0);
        // A blow on it costs the arm nothing and leaks no yellow; a
        // parrying holder sags 0.2 of the impact and is dazed 0.1 of it.
        let holder = crate::damage::Holding {
            synchrony: false,
            persist: false,
            counter: false,
            countering: false,
            clock: 0.5,
            potion: false,
        };
        let b = crate::damage::block(2.0, 0.6, &bare, &holder);
        assert_eq!((b.stamina, b.yellow, b.daze), (1.0, 0.6, 0.0));
        let parrying = ShieldGuard {
            parrying: true,
            ..bare
        };
        let b = crate::damage::block(2.0, 0.6, &parrying, &holder);
        assert_eq!(b.stamina, 1.0);
        assert_eq!(b.sag, Some(((0.7 - 0.5f64) * 2.0) as f32));
        assert_eq!(b.daze, ((0.3 - 0.2f64) * 2.0) as f32);
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
