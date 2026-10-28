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
use exa_physics::grip::{GripKind, HeldGrip, mode_from_clip};
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
    pub body: Entity,
}

/// Starts the flip on every held item of the requested character.
pub fn flip_requests(
    mut requests: MessageReader<FlipWeaponRequest>,
    mut held: Query<&mut HeldGrip>,
) {
    for request in requests.read() {
        for mut grip in &mut held {
            if grip.holder_key != request.body {
                continue;
            }
            if let Some(links) = grip.links.as_mut() {
                links.begin_flip();
            }
        }
    }
}

/// Derives each held item's grip mode from its holder's clip, then steps an
/// active flip, turns the item's node when the flip lands and rebuilds the
/// weapon record.
#[allow(clippy::type_complexity)]
pub fn animate_grips(
    mut held: Query<(
        Entity,
        &mut HeldGrip,
        &mut Transform,
        Option<&mut Blade>,
        Option<&HeldRecord>,
    )>,
    bodies: Query<(Entity, &Ragdoll)>,
    owners: Query<(&Animator, &Condition)>,
    motions: Option<Res<Motions>>,
) {
    for (item, mut grip, mut transform, blade, record) in &mut held {
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
        let before = links.flipped;
        let attacking = *condition == Condition::ATTACKING;
        links.flip_step(item_body, attacking);
        if links.flipped != before {
            // Half a turn about the item's own y axis, as the original's
            // sign-bit flip of the x and z axes.
            transform.rotate_local_y(std::f32::consts::PI);
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
            .add_systems(FixedUpdate, animate_grips.before(GripSolveSet::Sync));
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
        }])
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
        Section { flags: 0x11, stats }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<FlipWeaponRequest>()
            .add_systems(Update, (flip_requests, animate_grips).chain());
        app
    }

    /// The clip's mode switches the two-hander's links, and a flip request
    /// runs the animation to its end, turning the item and rebuilding the
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
            soft_weld: false,
            shield: false,
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
        // Half a turn about the item's y axis.
        let turn = app.world().get::<Transform>(item_e).unwrap().rotation;
        assert!((turn.to_euler(EulerRot::YXZ).0.abs() - std::f32::consts::PI).abs() < 1e-5);
    }
}
