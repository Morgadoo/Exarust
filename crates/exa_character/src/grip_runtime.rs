//! The grip's runtime: the mode its current clip asks for, the flip
//! animation, and the record rebuild a finished flip needs.
//!
//! The original's body tick derives the grip mode from a motion when it
//! starts (a knocked-down body mode 0, Idle or Walk mode 2, Stab 3, and 1
//! for the other clips), and its locomotion routine sets it on every tick
//! it runs (1 running or in combat, else 2); a changed mode runs the table
//! entry's callback, which switches the two-hander's second hand, arm and
//! stab links or slides the polearm's main link
//! ([`exa_physics::grip::tick_mode`]). A
//! flip command starts the body's flip byte. Each tick the flip's progress
//! moves toward its other end unless an attack is away from the middle,
//! then the flip link's point turns, and the weapon record is rebuilt when
//! the flip lands.
//!
//! The grip records live on the item body ([`HeldGrip`]); this module is
//! the character side that reads the holder's animation and condition and
//! feeds them.

use bevy::prelude::*;
use exa_anim::{Animator, Motions};
use exa_combat::{Blade, Condition};
use exa_formats::weapon_record::{Holder, Lengths, Section, WeaponRecord};
use exa_physics::body::Ragdoll;
use exa_physics::grip::{GripKind, HeldGrip, HeldKind, tick_mode};
use exa_physics::grip_solve::GripSolveSet;

/// The weapon data a flip rebuilds its record from.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HeldRecord {
    pub section: Section,
    pub lengths: Lengths,
    pub holder: Holder,
}

/// A held item's ability type: the word its weapon section names (3 for
/// the burning blade), kept on the item body by the ability check. The
/// two-handed grip's callback reads it with its holder's lit-blade byte
/// ([`crate::held::BurningGrip`]).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeldAbility(pub u32);

/// The ability type of the burning blade.
pub const BURNING_ABILITY: u32 = 3;

/// A character asked to turn its held item over.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlipWeaponRequest {
    /// The character's entity (the one carrying its [`Ragdoll`]).
    pub body: Entity,
}

/// A held item whose grip waits for its holder's body: the character's
/// entity. The dresser may attach an item in the frame before the holder's
/// body exists; [`resolve_pending_grips`] fills the grip's holder key and
/// drops the marker once the body's first part does.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingGrip {
    pub holder: Entity,
}

/// Fills every pending grip's holder key from its character's body.
pub fn resolve_pending_grips(
    mut commands: Commands,
    bodies: Query<&Ragdoll>,
    mut held: Query<(Entity, &PendingGrip, &mut HeldGrip)>,
) {
    for (entity, pending, mut grip) in &mut held {
        let Some(key) = bodies
            .get(pending.holder)
            .ok()
            .and_then(|b| b.parts.first())
            .map(|p| p.entity)
        else {
            continue;
        };
        grip.holder_key = key;
        commands.entity(entity).remove::<PendingGrip>();
    }
}

/// A character's condition on the previous fixed tick, so the weapon
/// switch's swap fires once when its clip passes 60 %.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastCondition(pub Condition);

/// What the equip refresh measures from one held item: its weapon class,
/// the record's start byte, the item node's y extents (cm) and the race's
/// unarmed reach. [`exa_combat::grip::held_measures`] turns them into the
/// item's reach and aim height.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct HeldItemSpec {
    pub class: u16,
    pub start_byte: u8,
    pub item_cm: f32,
    pub race_reach: f32,
}

/// The character's held-gear refresh, as its equip and every swap
/// recompute it: the count of held items, the first striker's measures,
/// its aim point's weights on the item body and the parry gate. The aim
/// drive that reads the weights is outside this component; the reach the
/// AI reads is the equip pass's ([`exa_combat::grip::held_reach`]).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct HeldRefresh(pub exa_combat::grip::HoldRefresh);

/// Recomputes each character's held-gear refresh from its held items, in
/// the original's equip/swap refresh: the count of items, the striker in
/// the first held slot as the measured one, and the parry gate of that
/// item. Runs before the solver so a swap's new first item is measured the
/// same tick.
#[allow(clippy::type_complexity)]
pub fn refresh_holds(
    mut commands: Commands,
    held: Query<(
        Entity,
        &HeldGrip,
        Option<&Blade>,
        Option<&HeldItemSpec>,
        Option<&Ragdoll>,
    )>,
    mut holders: Query<(Entity, &Ragdoll, Option<&mut HeldRefresh>)>,
) {
    // Each holder's weapons in the held items' order: every body is a
    // holder here (the level's props too), so the items are bucketed by
    // holder once instead of walked once per body.
    let mut by_holder: bevy::ecs::entity::EntityHashMap<Vec<_>> = Default::default();
    for item in &held {
        // The count is the hands' weapon slots: a shield or a torch is held
        // through its own slot.
        if item.1.kind == HeldKind::Weapon {
            by_holder.entry(item.1.holder_key).or_default().push(item);
        }
    }
    for (owner, body, current) in &mut holders {
        let Some(key) = body.parts.first().map(|p| p.entity) else {
            continue;
        };
        let mut count = 0;
        let mut first: Option<(HeldItemSpec, bool, Option<[Vec3; 4]>)> = None;
        for &(_, grip, blade, spec, item_body) in by_holder.get(&key).into_iter().flatten() {
            count += 1;
            let Some(spec) = spec else {
                continue;
            };
            let is_first = blade.is_some_and(|b| b.slot == exa_combat::attack::Slot::First);
            if is_first || first.is_none() {
                let rest = item_body
                    .and_then(|b| b.parts.get(HeldGrip::ITEM_PART))
                    .map(|p| p.rest);
                first = Some((*spec, grip.soft_weld, rest));
            }
        }
        let refresh = HeldRefresh(match first {
            Some((spec, gate, rest)) => {
                let mut hold = exa_combat::grip::refresh_hold(
                    count,
                    gate,
                    spec.class,
                    spec.start_byte,
                    spec.item_cm,
                    spec.race_reach,
                );
                hold.aim_weights = rest
                    .zip(hold.measures)
                    .and_then(|(rest, m)| aim_weights_cm(&rest, m.aim_cm));
                hold
            }
            None => exa_combat::grip::refresh_hold(0, false, 0, 0, 0.0, 0.0),
        });
        if current.map(|c| *c) != Some(refresh) {
            commands.entity(owner).insert(refresh);
        }
    }
}

/// The aim point's weights on an item body's rest particles. Exarust keeps
/// a part's rest points in metres; the original's are centimetres, so the
/// points are scaled to centimetres first (the weights are the same up to
/// rounding). A flat rest tetrahedron, which the original's rigid-body
/// registration never makes, gives none instead of the division's
/// infinities [Exarust's].
fn aim_weights_cm(rest_m: &[Vec3; 4], aim_cm: f32) -> Option<[f32; 4]> {
    let rest_cm = rest_m.map(|p| (p * 100.0).to_array());
    let weights = exa_combat::grip::held_aim_weights(&rest_cm, aim_cm);
    weights.iter().all(|w| w.is_finite()).then_some(weights)
}

/// The other held slot.
fn other_slot(slot: exa_combat::attack::Slot) -> exa_combat::attack::Slot {
    match slot {
        exa_combat::attack::Slot::First => exa_combat::attack::Slot::Second,
        exa_combat::attack::Slot::Second => exa_combat::attack::Slot::First,
    }
}

/// Swaps a character's two held weapons when its switch clip reaches the
/// swap (60 % of the clip): the first and second held slots trade places
/// and its primary striker moves with the weapon. The items stay in their
/// hands; the slot that strikes, the attacker's primary and the holder's
/// [`crate::held::Hands`] (main and second weapon) change, so the moveset
/// is chosen again from the new main weapon. A holder with anything but two
/// weapons in the two slots keeps them.
#[allow(clippy::type_complexity)]
pub fn swap_on_switch(
    mut commands: Commands,
    mut holders: Query<(
        Entity,
        &Condition,
        &Ragdoll,
        Option<&mut LastCondition>,
        Option<&mut crate::held::Hands>,
    )>,
    held: Query<(Entity, &HeldGrip)>,
    mut blades: Query<&mut Blade>,
    mut attackers: Query<&mut exa_combat::attack::Attacker>,
) {
    for (owner, condition, body, last, hands) in &mut holders {
        let before = last.as_ref().map_or(*condition, |l| l.0);
        let due = exa_combat::condition::swap_due(before, *condition);
        if let Some(mut last) = last {
            last.0 = *condition;
        } else {
            commands.entity(owner).insert(LastCondition(*condition));
        }
        if !due {
            continue;
        }
        let Some(key) = body.parts.first().map(|p| p.entity) else {
            continue;
        };
        let items: Vec<Entity> = held
            .iter()
            .filter(|(_, grip)| grip.holder_key == key && grip.kind == HeldKind::Weapon)
            .map(|(entity, _)| entity)
            .collect();
        if items.len() != 2 {
            continue;
        }
        let (Ok(first), Ok(second)) = (blades.get(items[0]), blades.get(items[1])) else {
            continue;
        };
        if first.slot == second.slot {
            continue;
        }
        let swapped = [other_slot(first.slot), other_slot(second.slot)];
        for (entity, slot) in items.into_iter().zip(swapped) {
            if let Ok(mut blade) = blades.get_mut(entity) {
                blade.slot = slot;
            }
        }
        if let Ok(mut attacker) = attackers.get_mut(owner) {
            attacker.primary = attacker.primary.map(other_slot);
        }
        // The new first weapon picks the moveset: the original's equip
        // routines re-run the moveset choice on held slot 0 after every
        // change of the held list, so the hands trade their two weapons and
        // the moveset system chooses again from the new main hand.
        if let Some(mut hands) = hands
            && let (Some(main), Some(second)) = (hands.main, hands.second)
        {
            hands.main = Some(second);
            hands.second = Some(main);
        }
    }
}

/// Starts the flip on every held item of the requested character. A held
/// item names its holder by the holder body's first part, so the request's
/// body is looked up for it. The original's flip command refuses while the
/// character is switching or the item has no weapon section with a
/// flipped-side kind (`Section::can_flip`), and turns only the first held
/// item (a second weapon's blade, in [`exa_combat::attack::Slot::Second`],
/// stays as it is).
pub fn flip_requests(
    mut requests: MessageReader<FlipWeaponRequest>,
    bodies: Query<&Ragdoll>,
    mut held: Query<(&mut HeldGrip, Option<&HeldRecord>, Option<&Blade>)>,
    owners: Query<&Condition>,
) {
    for request in requests.read() {
        let Some(key) = bodies
            .get(request.body)
            .ok()
            .and_then(|b| b.parts.first())
            .map(|p| p.entity)
        else {
            continue;
        };
        if owners
            .get(request.body)
            .is_ok_and(|c| c.has(Condition::SWITCHING.0))
        {
            continue;
        }
        for (mut grip, record, blade) in &mut held {
            // Only the first held item turns: the command reads and the
            // flip step drives held slot 0 alone.
            if grip.holder_key != key
                || blade.is_some_and(|b| b.slot == exa_combat::attack::Slot::Second)
            {
                continue;
            }
            let can_flip = record.is_some_and(|r| r.section.can_flip());
            if let Some(links) = grip.links.as_mut() {
                links.begin_flip(can_flip);
            }
        }
    }
}

/// Marks the weapons and torches a collapsing holder lets go of. The
/// original's collapse drops every weapon in the held list and then the
/// torch it carries; a shield is not in that list (it hangs from the off
/// hand's own array), so it stays on the arm. A body that is dead without a
/// collapse keeps its items.
pub fn release_collapsed(
    mut commands: Commands,
    held: Query<(Entity, &HeldGrip)>,
    bodies: Query<(Entity, &Ragdoll)>,
    owners: Query<&Condition>,
) {
    let lets_go = |grip: &HeldGrip| !grip.released && grip.kind != HeldKind::Shield;
    let holders = exa_physics::body::first_by_key(
        held.iter()
            .filter(|(_, grip)| lets_go(grip))
            .map(|(_, grip)| grip.holder_key),
        bodies.iter(),
    );
    for (item, grip) in &held {
        if !lets_go(grip) {
            continue;
        }
        let Some(&owner) = holders.get(&grip.holder_key) else {
            continue;
        };
        if owners.get(owner).is_ok_and(|c| *c == Condition::COLLAPSING) {
            commands.entity(item).insert(exa_physics::grip::GripRelease);
        }
    }
}

/// Derives each held item's grip mode from its holder's clip, then steps an
/// active flip and rebuilds the weapon record when it lands. The item
/// itself turns with its body, which the flip link pulls round.
#[allow(clippy::type_complexity)]
pub fn animate_grips(
    mut held: Query<(
        Entity,
        &mut HeldGrip,
        Option<&mut Blade>,
        Option<&HeldRecord>,
        Option<&HeldAbility>,
    )>,
    bodies: Query<(Entity, &Ragdoll)>,
    owners: Query<(
        &Animator,
        &Condition,
        Option<&crate::held::BurningGrip>,
        Option<&exa_physics::body::locomotion::Locomotor>,
    )>,
    motions: Option<Res<Motions>>,
) {
    let holders = exa_physics::body::first_by_key(
        held.iter().map(|(_, grip, ..)| grip.holder_key),
        bodies.iter().map(|(entity, body)| ((entity, body), body)),
    );
    for (item, mut grip, blade, record, ability) in &mut held {
        let Ok((_, item_body)) = bodies.get(item) else {
            continue;
        };
        let Some(&(owner, holder_body)) = holders.get(&grip.holder_key) else {
            continue;
        };
        let Ok((anim, condition, burning_grip, locomotor)) = owners.get(owner) else {
            continue;
        };
        let clip = motions
            .as_ref()
            .and_then(|m| m.0.action_index(&anim.action))
            .map_or(0, |i| i as i32);
        let kind = grip.spec.kind();
        let scale = grip.scale;
        let Some(links) = grip.links.as_mut() else {
            continue;
        };
        // The mode a motion start derives, or the one the locomotion
        // routine wrote on this tick.
        let locomotion = locomotor.filter(|l| l.locomoted).map(|l| l.grip_mode);
        let mode = tick_mode(
            links.mode,
            links.motion,
            clip,
            condition.has(Condition::DOWN.0),
            locomotion,
        );
        links.motion = clip;
        if bevy::log::tracing::enabled!(target: "exa_physics::grip_trace", bevy::log::Level::DEBUG)
        {
            debug!(
                target: "exa_physics::grip_trace",
                "grip holder {owner} of {item}: clip {} ({clip:#x}), config {:?}, condition {:?}, locomotion {:?}",
                anim.action,
                anim.config,
                condition,
                locomotion
            );
        }
        if kind != GripKind::Other && links.mode != mode {
            debug!(
                "grip mode: {item:?} ({kind:?}) {} -> {mode} (clip {clip:#x}, {})",
                links.mode,
                if locomotion == Some(mode) {
                    "the locomotion's"
                } else {
                    "a motion start's"
                }
            );
            // The two-hander's callback reads the item's ability type and
            // its holder's lit-blade byte.
            let burning = ability.is_some_and(|a| a.0 == BURNING_ABILITY);
            let lit = burning_grip.is_some_and(|b| b.0);
            if burning && lit && mode == 2 {
                info!("grip: {item:?} burns in a lit hand: its two-handed rest takes mode 1");
            }
            links.apply_mode(mode, kind, scale, burning, lit, item_body, holder_body);
        }
        if links.relaxing {
            links.relax();
        }
        // The original's flip test reads the whole condition word.
        let attacking = *condition == Condition::ATTACKING;
        if links.flip_active {
            links.flip_step(item_body, attacking);
        }
        if links.record_dirty {
            links.record_dirty = false;
            if let (Some(mut blade), Some(record)) = (blade, record) {
                blade.record = WeaponRecord::build(
                    &record.section,
                    links.flipped,
                    record.lengths,
                    &record.holder,
                );
            }
        }
    }
}

/// A held item's contact mobility under its holder's arm setter: its base
/// times the setter's factor ([`exa_physics::body::HELD_CONTACT`], or
/// [`exa_physics::body::HELD_CONTACT_STRIKING`] in an attack or a parry),
/// a single-precision product as the original's. The holder's held list
/// takes it only when armed (its first held item's class has hands bits),
/// the shield always. `None` leaves the item as it is.
pub fn held_contact_mobility(
    base: f32,
    factor: Option<f32>,
    kind: HeldKind,
    armed: bool,
) -> Option<f32> {
    let factor = factor?;
    (kind == HeldKind::Shield || armed).then_some(base * factor)
}

/// The arm setter's tail: every tick it is called, the held items' contact
/// mobility is put at their base (the registered mobility, standing in for
/// the original's weapon-mass base) times the holder's factor, the shield's
/// whatever the holder holds, the other items' only when the holder is
/// armed ([`crate::body_words::armed`]). An item its holder let go of is
/// left alone. Runs after the character ticks wrote the factor, before the
/// solver reads the contact mobilities.
pub fn scale_held_contacts(
    held: Query<(Entity, &HeldGrip)>,
    mut bodies: Query<(Entity, &mut Ragdoll)>,
    hands: Query<&crate::held::Hands>,
) {
    if held.is_empty() {
        return;
    }
    let holders = exa_physics::body::first_by_key(
        held.iter().map(|(_, grip)| grip.holder_key),
        bodies.iter().map(|(entity, body)| {
            (
                (entity, body.drive.as_ref().and_then(|d| d.held_contact())),
                body,
            )
        }),
    );
    for (item, grip) in &held {
        if grip.released {
            continue;
        }
        let Some(&(owner, factor)) = holders.get(&grip.holder_key) else {
            continue;
        };
        let armed = crate::body_words::armed(hands.get(owner).ok());
        let Ok((_, mut body)) = bodies.get_mut(item) else {
            continue;
        };
        // The held body is the item's top part; its child bodies are not
        // on the holder's list.
        let Some(part) = body.parts.iter_mut().find(|p| p.parent.is_none()) else {
            continue;
        };
        if let Some(m) = held_contact_mobility(part.inverse_mass, factor, grip.kind, armed)
            && m != part.pairing.mobility
        {
            debug!(
                "held contact mobility: {item:?} ({:?}) of {owner:?}: {:.4} -> {m:.4} (base {:.4})",
                grip.kind, part.pairing.mobility, part.inverse_mass
            );
            part.pairing.mobility = m;
        }
    }
}

/// The grip runtime's systems: the flip request, then the mode and flip
/// step before the solver copies the links for this tick.
pub struct GripRuntimePlugin;

impl Plugin for GripRuntimePlugin {
    fn build(&self, app: &mut App) {
        bridge_grip_runtime_types(app);
        // After the AI's behaviour (`TickBehaviour`), as after the player's
        // input: a weapon switch or grip the AI asks this tick is taken now.
        use exa_physics::body::TickBehaviour;
        app.add_message::<FlipWeaponRequest>()
            .add_systems(
                FixedUpdate,
                (
                    flip_requests,
                    // The modes the character tick wrote (its motion starts
                    // and locomotion) apply before the bodies step.
                    animate_grips
                        .after(exa_physics::body::CharacterTicks)
                        .before(exa_physics::body::BodySystems),
                    scale_held_contacts
                        .after(exa_physics::body::CharacterTicks)
                        .before(exa_physics::body::BodySystems),
                    swap_on_switch,
                    refresh_holds,
                )
                    .after(TickBehaviour)
                    .before(GripSolveSet::Sync),
            )
            .add_systems(
                FixedUpdate,
                // A grip equipped before its holder's body was built takes
                // its holder key before the builder ties it to the hand: the
                // builder and the release that follows it would otherwise
                // see the item's links built for no known holder and drop
                // it on the first tick both bodies exist.
                (resolve_pending_grips, release_collapsed)
                    .chain()
                    .after(TickBehaviour)
                    .before(GripSolveSet::Build),
            );
    }
}

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_grip_runtime_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_message::<crate::grip_runtime::FlipWeaponRequest>()
        .bridge_copy::<crate::grip_runtime::HeldAbility>()
        .bridge_copy::<crate::grip_runtime::HeldItemSpec>()
        .bridge_copy::<crate::grip_runtime::HeldRecord>()
        .bridge_copy::<crate::grip_runtime::HeldRefresh>()
        .bridge_copy::<crate::grip_runtime::LastCondition>()
        .bridge_copy::<crate::grip_runtime::PendingGrip>()
        .bridge_copy::<crate::held::BurningGrip>()
        .bridge_copy::<exa_combat::Blade>()
        .bridge_copy::<exa_physics::grip::HeldGrip>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Affine3A;
    use exa_formats::weapon_stats::{StatBytes, at};
    use exa_physics::body::{PartSetup, RigidPart};
    use exa_physics::grip::{GripPair, GripPoint, GripSpec};

    fn holder(root: Entity, at: Vec3) -> Ragdoll {
        let setup = |bone, entity, parent, at| PartSetup {
            bone,
            entity,
            parent,
            world: Affine3A::from_translation(at),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        };
        Ragdoll::new(&[
            setup(1, root, None, at),
            setup(
                17,
                Entity::from_raw_u32(2).unwrap(),
                Some(0),
                at + Vec3::X * 0.2,
            ),
        ])
    }

    fn item(root: Entity, at: Vec3) -> Ragdoll {
        Ragdoll::rigid(&[RigidPart {
            entity: root,
            parent: None,
            world: Affine3A::from_translation(at),
            rest: [
                Vec3::new(-0.1, 0.0, -0.1),
                Vec3::new(0.1, 0.0, -0.1),
                Vec3::new(0.0, 0.0, 0.1),
                Vec3::new(0.0, 0.2, 0.0),
            ],
            point_masses: [4.0; 4],
            mobility: 1.0,
            parent_world: Affine3A::IDENTITY,
            spheres: Vec::new(),
            origin: Vec3::ZERO,
            links: Vec::new(),
            mesh: None,
            triangles: None,
        }])
    }

    /// A collapsing holder lets go of its weapons and torch but keeps the
    /// shield on its arm (the collapse's weapon list does not hold it); a
    /// dead one keeps everything.
    #[test]
    fn a_collapse_drops_the_weapon_and_keeps_the_shield() {
        let mut app = App::new();
        app.add_systems(Update, release_collapsed);
        let owner = app.world_mut().spawn(Condition::COLLAPSING).id();
        let holder_body = holder(owner, Vec3::ZERO);
        let key = holder_body.parts[0].entity;
        app.world_mut().entity_mut(owner).insert(holder_body);
        let spawn = |app: &mut App, kind| {
            let e = app.world_mut().spawn_empty().id();
            let body = item(Entity::from_raw_u32(30).unwrap(), Vec3::ZERO);
            let grip = HeldGrip {
                spec: spec(),
                scale: 1.0,
                size: 1.0,
                second_hand_factor: 1.0,
                flip: false,
                class: 0x10,
                soft_weld: false,
                kind,
                holder_key: key,
                hands: [Some(Entity::from_raw_u32(2).unwrap()); 2],
                arms: [None, None],
                links: None,
                released: false,
                head_extra: 0.0,
            };
            app.world_mut().entity_mut(e).insert((body, grip));
            e
        };
        let weapon = spawn(&mut app, exa_physics::grip::HeldKind::Weapon);
        let torch = spawn(&mut app, exa_physics::grip::HeldKind::Torch);
        let shield = spawn(&mut app, exa_physics::grip::HeldKind::Shield);
        app.update();
        for item in [weapon, torch] {
            assert!(
                app.world()
                    .get::<exa_physics::grip::GripRelease>(item)
                    .is_some()
            );
        }
        assert!(
            app.world()
                .get::<exa_physics::grip::GripRelease>(shield)
                .is_none()
        );
        // While alive nothing is marked; a dead body keeps its weapon too.
        app.world_mut()
            .entity_mut(owner)
            .insert(Condition::COMBAT_IDLE);
        app.world_mut()
            .entity_mut(weapon)
            .remove::<exa_physics::grip::GripRelease>();
        app.update();
        assert!(
            app.world()
                .get::<exa_physics::grip::GripRelease>(weapon)
                .is_none()
        );
        app.world_mut().entity_mut(owner).insert(Condition::DEAD);
        app.update();
        assert!(
            app.world()
                .get::<exa_physics::grip::GripRelease>(weapon)
                .is_none()
        );
    }

    /// The flip command refuses a switching character and a section with no
    /// flipped-side kind byte, and starts the flip otherwise.
    #[test]
    fn the_flip_command_gate_reads_the_section_flip_side() {
        use exa_physics::grip::HeldKind;
        let build = |app: &mut App, condition: Condition, can_flip: bool| {
            let owner = app.world_mut().spawn(condition).id();
            let body = holder(owner, Vec3::ZERO);
            let key = body.parts[0].entity;
            app.world_mut().entity_mut(owner).insert(body);
            let item_body = item(Entity::from_raw_u32(30).unwrap(), Vec3::ZERO);
            let item_to_hand = Affine3A::IDENTITY;
            let links = exa_physics::grip::build(
                &item_body,
                &holder(owner, Vec3::ZERO),
                exa_physics::grip::GripBindings {
                    item_part: 0,
                    hand_parts: [0, 0],
                    arm_parts: [0, 0],
                },
                &spec(),
                1.0,
                1.0,
                item_to_hand,
                false,
                1.0,
            )
            .unwrap();
            let mut stats = StatBytes::default();
            stats.set(at::SLASH, 80);
            if can_flip {
                stats.set(at::KIND + at::FLIP, 1);
            }
            let record = HeldRecord {
                section: Section { flags: 0x11, stats },
                lengths: Lengths::one_part(100.0, 0.1, 1.0),
                holder: Holder::AVERAGE,
            };
            let blade = Blade::new(
                owner,
                1.0,
                WeaponRecord::build(&record.section, false, record.lengths, &record.holder),
            );
            let app_item = app
                .world_mut()
                .spawn((
                    item_body,
                    HeldGrip {
                        spec: spec(),
                        scale: 1.0,
                        size: 1.0,
                        second_hand_factor: 1.0,
                        flip: false,
                        class: 0x20,
                        soft_weld: false,
                        kind: HeldKind::Weapon,
                        holder_key: key,
                        hands: [None, None],
                        arms: [None, None],
                        links: Some(links),
                        released: false,
                        head_extra: 0.0,
                    },
                    record,
                    blade,
                ))
                .id();
            app.world_mut()
                .write_message(FlipWeaponRequest { body: owner });
            app.update();
            let grip = app.world().get::<HeldGrip>(app_item).unwrap();
            grip.links.as_ref().unwrap().flip_active
        };
        let mut a = App::new();
        a.add_message::<FlipWeaponRequest>()
            .add_systems(Update, flip_requests);
        assert!(build(&mut a, Condition::COMBAT_IDLE, true));
        assert!(!build(&mut a, Condition::COMBAT_IDLE, false));
        assert!(!build(&mut a, Condition::SWITCHING, true));
    }

    fn spec() -> GripSpec {
        GripSpec {
            hand: 0,
            links: vec![
                GripPoint {
                    point: Vec3::new(0.0, 20.0, 0.0),
                    slack: 0.0,
                },
                GripPoint {
                    point: Vec3::new(15.0, 0.0, 0.0),
                    slack: 0.0,
                },
            ],
            second_hand: Some(GripPair {
                item: Vec3::new(0.0, -30.0, 0.0),
                part: Vec3::new(11.0, -3.0, 0.0),
            }),
            arm: Some(GripPair {
                item: Vec3::new(0.0, 20.0, 0.0),
                part: Vec3::new(-12.0, 3.0, 6.0),
            }),
            extra: Some(GripPair {
                item: Vec3::new(0.0, 25.0, 0.0),
                part: Vec3::new(8.0, -5.0, 0.0),
            }),
        }
    }

    fn section() -> Section {
        let mut stats = StatBytes::default();
        stats.set(at::SLASH, 80);
        stats.set(at::SLASH + at::FLIP, 30);
        // The weapon defines a flipped side (its kind byte), or the flip
        // command refuses it.
        stats.set(at::KIND + at::FLIP, 1);
        Section { flags: 0x11, stats }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<FlipWeaponRequest>()
            .add_systems(Update, (flip_requests, animate_grips).chain());
        app
    }

    /// The switch's swap trades the two held weapons' slots and the primary
    /// striker once, on the tick past 60 % of the clip.
    #[test]
    fn a_weapon_switch_swaps_the_two_held_slots_once() {
        use exa_combat::attack::{Attacker, Slot};
        let mut app = App::new();
        app.add_systems(Update, swap_on_switch);
        let mut attacker = Attacker::default();
        attacker.primary = Some(Slot::First);
        let owner = app
            .world_mut()
            .spawn((
                Condition::SWITCHED,
                LastCondition(Condition::SWITCHING),
                attacker,
                crate::held::Hands {
                    main: Some(crate::held::HeldItem::weapon(0x20)),
                    second: Some(crate::held::HeldItem::weapon(0x11)),
                    ..default()
                },
            ))
            .id();
        let body = holder(owner, Vec3::ZERO);
        let key = body.parts[0].entity;
        app.world_mut().entity_mut(owner).insert(body);
        let blade = |slot| {
            Blade::new(
                owner,
                1.0,
                WeaponRecord::build(
                    &section(),
                    false,
                    Lengths::one_part(100.0, 0.1, 1.0),
                    &Holder::AVERAGE,
                ),
            )
            .in_slot(slot)
        };
        let spawn = |app: &mut App, slot| {
            let grip = HeldGrip {
                spec: spec(),
                scale: 1.0,
                size: 1.0,
                second_hand_factor: 1.0,
                flip: false,
                class: 0x10,
                soft_weld: false,
                kind: exa_physics::grip::HeldKind::Weapon,
                holder_key: key,
                hands: [None, None],
                arms: [None, None],
                links: None,
                released: false,
                head_extra: 0.0,
            };
            app.world_mut().spawn((grip, blade(slot))).id()
        };
        let first = spawn(&mut app, Slot::First);
        let second = spawn(&mut app, Slot::Second);
        app.update();
        assert_eq!(app.world().get::<Blade>(first).unwrap().slot, Slot::Second);
        assert_eq!(app.world().get::<Blade>(second).unwrap().slot, Slot::First);
        assert_eq!(
            app.world().get::<Attacker>(owner).unwrap().primary,
            Some(Slot::Second)
        );
        // The hands trade their weapons, so the moveset choice now reads
        // the one-hander as the main weapon.
        let hands = *app.world().get::<crate::held::Hands>(owner).unwrap();
        assert_eq!(hands.main, Some(crate::held::HeldItem::weapon(0x11)));
        assert_eq!(hands.second, Some(crate::held::HeldItem::weapon(0x20)));
        assert_eq!(
            crate::held::choose(&hands, false).stance,
            crate::held::StanceClass::TwoWeapons
        );
        // The next tick is no swap: the slots stay.
        app.update();
        assert_eq!(app.world().get::<Blade>(first).unwrap().slot, Slot::Second);
        assert_eq!(
            app.world().get::<Attacker>(owner).unwrap().primary,
            Some(Slot::Second)
        );
    }

    /// The refresh measures the first held striker, counts every held item
    /// and takes the first item's parry gate; empty hands drop it.
    #[test]
    fn the_held_refresh_measures_the_first_striker() {
        use exa_combat::attack::Slot;
        use exa_combat::grip::{HoldRefresh, held_measures};
        let mut app = App::new();
        app.add_systems(Update, refresh_holds);
        let owner = app.world_mut().spawn_empty().id();
        let body = holder(owner, Vec3::ZERO);
        let key = body.parts[0].entity;
        app.world_mut().entity_mut(owner).insert(body);
        let spawn = |app: &mut App, slot: Slot, item: HeldItemSpec, soft: bool| {
            let grip = HeldGrip {
                spec: spec(),
                scale: 1.0,
                size: 1.0,
                second_hand_factor: 1.0,
                flip: false,
                class: u32::from(item.class),
                soft_weld: soft,
                kind: exa_physics::grip::HeldKind::Weapon,
                holder_key: key,
                hands: [None, None],
                arms: [None, None],
                links: None,
                released: false,
                head_extra: 0.0,
            };
            let blade = Blade::new(
                owner,
                1.0,
                WeaponRecord::build(
                    &section(),
                    false,
                    Lengths::one_part(100.0, 0.1, 1.0),
                    &Holder::AVERAGE,
                ),
            )
            .in_slot(slot);
            app.world_mut().spawn((grip, blade, item)).id()
        };
        let first = spawn(
            &mut app,
            Slot::First,
            HeldItemSpec {
                class: 0x20,
                start_byte: 128,
                item_cm: 90.0,
                race_reach: 50.0,
            },
            true,
        );
        app.update();
        let refresh = app.world().get::<HeldRefresh>(owner).unwrap().0;
        assert_eq!(
            refresh,
            HoldRefresh {
                held: true,
                may_parry: true,
                measures: Some(held_measures(0x20, 128, 90.0, 50.0)),
                aim_weights: None,
            }
        );
        // A second weapon counts, but the first slot keeps the measures.
        let second = spawn(
            &mut app,
            Slot::Second,
            HeldItemSpec {
                class: 0x10,
                start_byte: 5,
                item_cm: 97.1,
                race_reach: 50.0,
            },
            false,
        );
        app.update();
        let refresh = app.world().get::<HeldRefresh>(owner).unwrap().0;
        assert!(refresh.held && refresh.may_parry && refresh.measures.is_some());
        assert_eq!(refresh.measures.unwrap().reach, 175.2);
        // With the first weapon gone the second becomes the measured one.
        app.world_mut().entity_mut(first).despawn();
        app.update();
        let refresh = app.world().get::<HeldRefresh>(owner).unwrap().0;
        assert_eq!(refresh.measures, Some(held_measures(0x10, 5, 97.1, 50.0)));
        // An item with a body: the aim point 0.9 of its 97.1 cm up is
        // weighted on its rest particles and comes back from them.
        let item_body = holder(second, Vec3::ZERO);
        let rest = item_body.parts[0].rest;
        app.world_mut().entity_mut(second).insert(item_body);
        app.update();
        let refresh = app.world().get::<HeldRefresh>(owner).unwrap().0;
        let weights = refresh.aim_weights.expect("the item's body gives weights");
        let back: Vec3 = rest.iter().zip(weights).map(|(p, w)| *p * 100.0 * w).sum();
        let aim = exa_combat::grip::held_aim_height(97.1);
        assert!((back - Vec3::new(0.0, aim, 0.0)).length() < 1e-3);
        // Both gone: no hold.
        app.world_mut().entity_mut(second).despawn();
        app.update();
        let refresh = app.world().get::<HeldRefresh>(owner).unwrap().0;
        assert!(!refresh.held && refresh.measures.is_none());
    }

    /// A grip attached before its holder's body exists takes the holder's
    /// first part as its key once the body is there.
    #[test]
    fn a_pending_grip_waits_for_its_holders_body() {
        let mut app = App::new();
        app.add_systems(Update, resolve_pending_grips);
        let owner = app.world_mut().spawn_empty().id();
        let item_e = app.world_mut().spawn_empty().id();
        let grip = HeldGrip {
            spec: spec(),
            scale: 1.0,
            size: 1.0,
            second_hand_factor: 1.0,
            flip: false,
            class: 0x10,
            soft_weld: false,
            kind: exa_physics::grip::HeldKind::Weapon,
            holder_key: Entity::PLACEHOLDER,
            hands: [None, None],
            arms: [None, None],
            links: None,
            released: false,
            head_extra: 0.0,
        };
        app.world_mut()
            .entity_mut(item_e)
            .insert((grip.clone(), PendingGrip { holder: owner }));
        app.update();
        // No body yet: the key stays and the marker stays.
        assert!(app.world().get::<PendingGrip>(item_e).is_some());
        assert_eq!(
            app.world().get::<HeldGrip>(item_e).unwrap().holder_key,
            Entity::PLACEHOLDER
        );
        let body = holder(owner, Vec3::ZERO);
        let key = body.parts[0].entity;
        app.world_mut().entity_mut(owner).insert(body);
        app.update();
        assert!(app.world().get::<PendingGrip>(item_e).is_none());
        assert_eq!(app.world().get::<HeldGrip>(item_e).unwrap().holder_key, key);
    }

    /// A weapon equipped before its holder's body is built (the load builds
    /// a body only once it is dressed) is gripped by the hand once the body
    /// exists: on the first fixed tick with both bodies the pending holder
    /// key resolves before the grip builder runs, so the links tie the item
    /// to the hand part and the grip is not taken for an ownerless one and
    /// dropped.
    #[test]
    fn an_item_equipped_before_its_body_is_built_is_gripped_by_the_hand() {
        let mut app = App::new();
        app.init_resource::<exa_physics::body::SolverStages>()
            .add_plugins(GripRuntimePlugin);
        exa_physics::grip_solve::plugin(&mut app);
        app.finish();
        app.world_mut().run_schedule(Startup);
        let owner = app.world_mut().spawn(Condition::COMBAT_IDLE).id();
        let hand = Entity::from_raw_u32(2).unwrap();
        let item_e = app.world_mut().spawn_empty().id();
        let grip = HeldGrip {
            spec: GripSpec {
                hand: 0,
                links: vec![exa_physics::grip::GripPoint {
                    point: Vec3::new(0.0, 20.0, 0.0),
                    slack: 0.0,
                }],
                second_hand: None,
                arm: None,
                extra: None,
            },
            scale: 1.0,
            size: 1.0,
            second_hand_factor: 1.0,
            flip: false,
            class: 0x10,
            soft_weld: false,
            kind: exa_physics::grip::HeldKind::Weapon,
            holder_key: Entity::PLACEHOLDER,
            hands: [Some(hand), None],
            arms: [Some(hand), None],
            links: None,
            released: false,
            head_extra: 0.0,
        };
        // The equip: the grip waits for the holder's body.
        app.world_mut()
            .entity_mut(item_e)
            .insert((grip, PendingGrip { holder: owner }));
        app.world_mut().run_schedule(FixedUpdate);
        assert!(app.world().get::<PendingGrip>(item_e).is_some());
        // Both bodies are built in the same frame, before the next tick.
        let body = holder(owner, Vec3::ZERO);
        let key = body.parts[0].entity;
        app.world_mut().entity_mut(owner).insert(body);
        app.world_mut()
            .entity_mut(item_e)
            .insert(item(item_e, Vec3::new(0.2, 0.2, 0.0)));
        app.world_mut().run_schedule(FixedUpdate);
        let grip = app.world().get::<HeldGrip>(item_e).unwrap();
        assert_eq!(grip.holder_key, key);
        assert!(!grip.released, "the grip was dropped as ownerless");
        let links = grip
            .links
            .as_ref()
            .expect("the hand does not grip the item");
        assert!(links.links[links.main].enabled());
        let rest = app
            .world()
            .get::<exa_physics::grip::HeldRest>(item_e)
            .expect("the item has no place under its hand");
        assert_eq!(rest.hand, 1, "gripped by the hand part");
        let item_body = app.world().get::<Ragdoll>(item_e).unwrap();
        assert_eq!(item_body.held_by, Some(key));
    }

    /// The clip's mode switches the two-hander's links, and a flip request
    /// runs the animation to its end and rebuilds the
    /// weapon record from its flipped side.
    #[test]
    fn the_clip_switches_the_links_and_a_flip_turns_the_record() {
        let mut app = app();
        let owner = app
            .world_mut()
            .spawn((Animator::new("Default", "Walk"), Condition::COMBAT_IDLE))
            .id();
        let item_e = app.world_mut().spawn_empty().id();
        let holder_body = holder(owner, Vec3::ZERO);
        let item_body = item(item_e, Vec3::new(0.2, 0.0, 0.0));
        let bindings = exa_physics::grip::GripBindings {
            item_part: 0,
            hand_parts: [1, 1],
            arm_parts: [1, 1],
        };
        let item_to_hand =
            holder_body.parts[1].transform().inverse() * item_body.parts[0].transform();
        let links = exa_physics::grip::build(
            &item_body,
            &holder_body,
            bindings,
            &spec(),
            1.0,
            1.0,
            item_to_hand,
            false,
            1.0,
        )
        .unwrap();
        let grip = HeldGrip {
            spec: spec(),
            scale: 1.0,
            size: 1.0,
            second_hand_factor: 1.0,
            flip: false,
            class: 0x20,
            soft_weld: false,
            kind: exa_physics::grip::HeldKind::Weapon,
            holder_key: owner,
            hands: [
                Some(Entity::from_raw_u32(2).unwrap()),
                Some(Entity::from_raw_u32(2).unwrap()),
            ],
            arms: [None, None],
            links: Some(links),
            released: false,
            head_extra: 0.0,
        };
        let record = HeldRecord {
            section: section(),
            lengths: Lengths::one_part(100.0, 0.1, 1.0),
            holder: Holder::AVERAGE,
        };
        let blade = Blade::new(
            owner,
            1.0,
            WeaponRecord::build(
                &section(),
                false,
                Lengths::one_part(100.0, 0.1, 1.0),
                &Holder::AVERAGE,
            ),
        );
        app.world_mut().entity_mut(owner).insert(holder_body);
        app.world_mut().entity_mut(item_e).insert((
            item_body,
            grip,
            record,
            blade,
            Transform::IDENTITY,
        ));

        app.update();
        let links = app
            .world()
            .get::<HeldGrip>(item_e)
            .unwrap()
            .links
            .as_ref()
            .unwrap();
        // "Walk" (action 0) asks for mode 2: the arm link runs.
        assert_eq!(links.mode, 2);
        assert!(links.links[links.arm.unwrap()].enabled());

        app.world_mut()
            .write_message(FlipWeaponRequest { body: owner });
        for _ in 0..200 {
            app.update();
            if app
                .world()
                .get::<HeldGrip>(item_e)
                .unwrap()
                .links
                .as_ref()
                .unwrap()
                .flipped
            {
                break;
            }
        }
        let grip = app.world().get::<HeldGrip>(item_e).unwrap();
        let links = grip.links.as_ref().unwrap();
        assert!(links.flipped);
        assert!(!links.flip_active);
        assert_eq!(links.flip_progress, 1.0);
        let blade = app.world().get::<Blade>(item_e).unwrap();
        // The flipped side's slash 30: 0.5·(0.3)^0.7.
        assert!(
            (blade.record.slash - 0.215_255_81).abs() < 1e-6,
            "{}",
            blade.record.slash
        );
    }

    /// The held items' contact mobility: the base times the setter's
    /// factor, the shield's always, the other items' only when armed; no
    /// call leaves the item.
    #[test]
    fn the_held_contact_follows_the_arm_setter() {
        use exa_physics::body::{HELD_CONTACT, HELD_CONTACT_STRIKING};
        let f = held_contact_mobility;
        assert_eq!(
            f(0.3, Some(HELD_CONTACT), HeldKind::Weapon, true),
            Some(0.6)
        );
        assert_eq!(
            f(0.3, Some(HELD_CONTACT_STRIKING), HeldKind::Weapon, true),
            Some(0.3)
        );
        assert_eq!(f(0.3, Some(HELD_CONTACT), HeldKind::Torch, false), None);
        assert_eq!(
            f(0.3, Some(HELD_CONTACT), HeldKind::Shield, false),
            Some(0.6)
        );
        assert_eq!(f(0.3, None, HeldKind::Shield, true), None);
    }
}
