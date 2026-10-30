//! The grip's runtime: the mode its current clip asks for, the flip
//! animation, and the record rebuild a finished flip needs.
//!
//! The original's body tick derives the grip mode from the motion it plays
//! (a knocked-down body mode 0, Idle or Walk mode 2, Stab 3, and 1 for the
//! other clips) and runs the table entry's callback, which switches the two-hander's
//! second hand, arm and stab links or slides the polearm's main link. A
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
use exa_physics::grip::{GripKind, HeldGrip, HeldKind, mode_from_clip};
use exa_physics::grip_solve::GripSolveSet;

/// The weapon data a flip rebuilds its record from.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HeldRecord {
    pub section: Section,
    pub lengths: Lengths,
    pub holder: Holder,
}

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
    for (owner, body, current) in &mut holders {
        let Some(key) = body.parts.first().map(|p| p.entity) else {
            continue;
        };
        let mut count = 0;
        let mut first: Option<(HeldItemSpec, bool, Option<[Vec3; 4]>)> = None;
        for (_, grip, blade, spec, item_body) in &held {
            // The count is the hands' weapon slots: a shield or a torch
            // is held through its own slot.
            if grip.holder_key != key || grip.kind != HeldKind::Weapon {
                continue;
            }
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

/// Marks the weapons and torches a collapsing holder lets go of: the
/// collapse routine empties the hands, while a shield stays on the off arm.
/// A body that is dead without a collapse keeps its items.
pub fn release_collapsed(
    mut commands: Commands,
    held: Query<(Entity, &HeldGrip)>,
    bodies: Query<(Entity, &Ragdoll)>,
    owners: Query<&Condition>,
) {
    for (item, grip) in &held {
        if grip.released || grip.kind == HeldKind::Shield {
            continue;
        }
        let Some((owner, _)) = bodies
            .iter()
            .find(|(_, b)| b.parts.first().is_some_and(|p| p.entity == grip.holder_key))
        else {
            continue;
        };
        if owners.get(owner).is_ok_and(|c| *c == Condition::COLLAPSING) {
            commands.entity(item).insert(exa_physics::grip::GripRelease);
        }
    }
}

/// What a held item's world welds have already taken from its holder's
/// condition: the collapse's 10 cm soft-weld slack and death's stiffening
/// land once each, and a body that rises again keeps them (the original's
/// revive does not rebuild the welds).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HeldWeld {
    pub collapsed: bool,
    pub stiff: bool,
}

/// Applies the original's collapse and death changes to a held item's world
/// welds: a collapsing holder's item takes the 10 cm soft-weld slack (the
/// collapse routine's, which runs after it has let go of the weapons, so
/// what is left is the shield or torch that stays), and a dead holder's
/// item takes the stiff soft weld. The original's collapse gate also reads
/// the motion's fall class, which Exarust's collapse always is (it starts
/// Fall Backward) [guess on the motion class's meaning].
#[allow(clippy::type_complexity)]
pub fn adjust_welds(
    mut commands: Commands,
    mut held: Query<(
        Entity,
        &mut HeldGrip,
        Option<&mut HeldWeld>,
        Has<exa_physics::grip::GripRelease>,
    )>,
    owners: Query<(&Ragdoll, &Condition)>,
) {
    if held.is_empty() {
        return;
    }
    let keys: Vec<(Entity, Condition)> = owners
        .iter()
        .filter_map(|(b, c)| b.parts.first().map(|p| (p.entity, *c)))
        .collect();
    for (item, mut grip, weld, releasing) in &mut held {
        if grip.released || releasing || grip.links.is_none() {
            continue;
        }
        let Some(condition) = keys
            .iter()
            .find(|(key, _)| *key == grip.holder_key)
            .map(|(_, c)| *c)
        else {
            continue;
        };
        let mut state = weld.as_ref().map_or(HeldWeld::default(), |w| **w);
        let before = state;
        if let Some(links) = grip.links.as_mut() {
            if condition.is_collapsing() && !state.collapsed {
                links.slacken_soft_weld();
                state.collapsed = true;
            }
            if condition == Condition::DEAD && !state.stiff {
                links.stiffen_soft_weld();
                state.stiff = true;
            }
        }
        if state != before {
            match weld {
                Some(mut current) => {
                    *current = state;
                }
                None => {
                    commands.entity(item).insert(state);
                }
            }
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
    )>,
    bodies: Query<(Entity, &Ragdoll)>,
    owners: Query<(&Animator, &Condition)>,
    motions: Option<Res<Motions>>,
) {
    for (item, mut grip, blade, record) in &mut held {
        let Ok((_, item_body)) = bodies.get(item) else {
            continue;
        };
        let Some((owner, holder_body)) = bodies
            .iter()
            .find(|(_, b)| b.parts.first().is_some_and(|p| p.entity == grip.holder_key))
        else {
            continue;
        };
        let Ok((anim, condition)) = owners.get(owner) else {
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
        let mode = mode_from_clip(clip, clip, condition.has(Condition::DOWN.0));
        if kind != GripKind::Other && links.mode != mode {
            // The half-sword turn needs the weapon section's own thrust
            // class; Exarust has not read that byte, so it passes the
            // record's kind as the nearest value. [guess]
            let thrust_three = blade.as_deref().is_some_and(|b| b.record.kind() == 3);
            links.apply_mode(
                mode,
                kind,
                scale,
                thrust_three,
                false,
                item_body,
                holder_body,
            );
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

/// The grip runtime's systems: the flip request, then the mode and flip
/// step before the solver copies the links for this tick.
pub struct GripRuntimePlugin;

impl Plugin for GripRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FlipWeaponRequest>()
            .add_systems(FixedUpdate, flip_requests.before(GripSolveSet::Sync))
            .add_systems(FixedUpdate, animate_grips.before(GripSolveSet::Sync))
            .add_systems(FixedUpdate, swap_on_switch.before(GripSolveSet::Sync))
            .add_systems(FixedUpdate, refresh_holds.before(GripSolveSet::Sync))
            .add_systems(
                FixedUpdate,
                release_collapsed.before(exa_physics::grip_solve::GripSolveSet::Build),
            )
            .add_systems(
                FixedUpdate,
                adjust_welds
                    .after(release_collapsed)
                    .before(exa_physics::grip_solve::GripSolveSet::Build),
            )
            .add_systems(
                FixedUpdate,
                resolve_pending_grips.before(GripSolveSet::Sync),
            );
    }
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
    /// shield on its arm; a dead one keeps everything.
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
            };
            app.world_mut().entity_mut(e).insert((body, grip));
            e
        };
        let weapon = spawn(&mut app, exa_physics::grip::HeldKind::Weapon);
        let torch = spawn(&mut app, exa_physics::grip::HeldKind::Torch);
        let shield = spawn(&mut app, exa_physics::grip::HeldKind::Shield);
        app.update();
        assert!(
            app.world()
                .get::<exa_physics::grip::GripRelease>(weapon)
                .is_some()
        );
        assert!(
            app.world()
                .get::<exa_physics::grip::GripRelease>(torch)
                .is_some()
        );
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

    /// A collapse gives its held item's soft weld a 10 cm slack once, and
    /// death then makes it stiff; a weapon the collapse lets go of takes
    /// neither.
    #[test]
    fn a_collapse_slackens_and_death_stiffens_the_held_weld() {
        let mut app = App::new();
        app.add_systems(Update, adjust_welds);
        let owner = app.world_mut().spawn(Condition::COLLAPSING).id();
        let holder_body = holder(owner, Vec3::ZERO);
        let key = holder_body.parts[0].entity;
        app.world_mut().entity_mut(owner).insert(holder_body);
        let spawn = |app: &mut App, kind, links: exa_physics::grip::GripLinks| {
            let e = app.world_mut().spawn_empty().id();
            let body = item(Entity::from_raw_u32(30).unwrap(), Vec3::ZERO);
            let grip = HeldGrip {
                spec: spec(),
                scale: 1.0,
                size: 1.0,
                second_hand_factor: 1.0,
                flip: false,
                class: crate::offhand::SHIELD_CLASS,
                soft_weld: true,
                kind,
                holder_key: key,
                hands: [Some(Entity::from_raw_u32(2).unwrap()); 2],
                arms: [None, None],
                links: Some(links),
                released: false,
            };
            app.world_mut().entity_mut(e).insert((body, grip));
            e
        };
        let item_links = |app: &mut App| {
            let body = item(Entity::from_raw_u32(31).unwrap(), Vec3::ZERO);
            let hand = holder(Entity::from_raw_u32(32).unwrap(), Vec3::ZERO);
            let _ = app;
            exa_physics::grip::torch_links(
                &body,
                &hand,
                exa_physics::grip::GripBindings {
                    item_part: 0,
                    hand_parts: [0, 0],
                    arm_parts: [0, 0],
                },
                0,
                Affine3A::IDENTITY,
            )
            .unwrap()
        };
        let links = item_links(&mut app);
        let shield = spawn(&mut app, exa_physics::grip::HeldKind::Shield, links);
        app.update();
        let links = app
            .world()
            .get::<HeldGrip>(shield)
            .unwrap()
            .links
            .clone()
            .unwrap();
        let soft = links.world_soft.unwrap();
        assert_eq!(links.links[soft].slack, 0.10);
        assert_eq!(links.links[soft].strength, 0.05);
        assert!(app.world().get::<HeldWeld>(shield).unwrap().collapsed);
        // Death stiffens it; the tick after that changes nothing more.
        app.world_mut().entity_mut(owner).insert(Condition::DEAD);
        app.update();
        let links = app
            .world()
            .get::<HeldGrip>(shield)
            .unwrap()
            .links
            .clone()
            .unwrap();
        assert_eq!(links.links[soft].strength, 1.0);
        assert_eq!(links.links[soft].slack, 0.10);
        let state = *app.world().get::<HeldWeld>(shield).unwrap();
        assert!(state.stiff && state.collapsed);
        app.update();
        assert_eq!(*app.world().get::<HeldWeld>(shield).unwrap(), state);
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
}
