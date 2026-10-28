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

/// The weld change death asks of every held item: a dead holder stiffens
/// its soft (parry) world weld to 1 (the original's death routine keeps the
/// weapon in the corpse's hand). The collapse routine's 10 cm slack for a
/// gobbler (race state word 0x800) is written to the link the held item's
/// parent part keeps at the weld's index, not to the item's own weld, and
/// is not reproduced.
pub fn weld_states(
    mut held: Query<&mut HeldGrip>,
    bodies: Query<(Entity, &Ragdoll)>,
    owners: Query<&Condition>,
) {
    for mut grip in &mut held {
        if grip.released {
            continue;
        }
        let Some((owner, _)) = bodies
            .iter()
            .find(|(_, b)| b.parts.first().is_some_and(|p| p.entity == grip.holder_key))
        else {
            continue;
        };
        let Ok(condition) = owners.get(owner) else {
            continue;
        };
        let Some(links) = grip.links.as_mut() else {
            continue;
        };
        if condition.is_dead() {
            links.weld_on_death();
        }
    }
}

/// Marks the weapons and torches a collapsing holder lets go of: the
/// collapse routine empties the hands, while a shield stays on the off arm
/// (its weld stiffens at death instead). A body that is dead without a
/// collapse keeps its items.
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
            .add_systems(FixedUpdate, weld_states.before(GripSolveSet::Sync))
            .add_systems(
                FixedUpdate,
                release_collapsed.before(exa_physics::grip_solve::GripSolveSet::Build),
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

    /// A dead holder keeps the item and stiffens its soft weld; a collapse
    /// leaves the weld alone.
    #[test]
    fn a_dead_holder_stiffens_the_soft_weld() {
        let mut app = App::new();
        app.add_systems(Update, weld_states);
        let owner = app.world_mut().spawn(Condition::COMBAT_IDLE).id();
        let item_e = app.world_mut().spawn_empty().id();
        let holder_body = holder(owner, Vec3::ZERO);
        let item_body = item(body_root(), Vec3::new(0.2, 0.0, 0.0));
        let key = holder_body.parts[0].entity;
        let bindings = exa_physics::grip::GripBindings {
            item_part: 0,
            hand_parts: [1, 1],
            arm_parts: [1, 1],
        };
        let shield = exa_physics::grip::shield_links(
            &item_body,
            &holder_body,
            bindings,
            1,
            Affine3A::IDENTITY,
            Vec3::ZERO,
        )
        .unwrap();
        let soft = shield.world_soft.unwrap();
        assert_eq!(shield.links[soft].strength, 0.05);
        let grip = HeldGrip {
            spec: spec(),
            scale: 1.0,
            size: 1.0,
            second_hand_factor: 1.0,
            flip: false,
            class: 0x10,
            soft_weld: true,
            kind: exa_physics::grip::HeldKind::Shield,
            holder_key: key,
            hands: [Some(Entity::from_raw_u32(2).unwrap()); 2],
            arms: [None, None],
            links: Some(shield),
            released: false,
        };
        app.world_mut().entity_mut(owner).insert(holder_body);
        app.world_mut().entity_mut(item_e).insert((item_body, grip));
        app.update();
        // Alive: untouched.
        let strength = |app: &App| {
            let links = app
                .world()
                .get::<HeldGrip>(item_e)
                .unwrap()
                .links
                .as_ref()
                .unwrap();
            links.links[soft].strength
        };
        assert_eq!(strength(&app), 0.05);
        app.world_mut().entity_mut(owner).insert(Condition::DEAD);
        app.update();
        assert_eq!(strength(&app), 1.0);
        assert!(
            app.world().get::<HeldGrip>(item_e).is_some(),
            "the corpse keeps the item"
        );
        // A collapsing body leaves the weld alone.
        app.world_mut()
            .entity_mut(owner)
            .insert(Condition::COLLAPSING);
        app.update();
        assert_eq!(strength(&app), 1.0);
    }

    /// The root entity the item body is built on.
    fn body_root() -> Entity {
        Entity::from_raw_u32(9).unwrap()
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
