//! The off hand's held item as its own body: a shield on the off arm or a
//! second weapon in the second held slot.
//!
//! The original's attach pass hangs at most one off-hand item: a shield
//! (from either hand, hand 1's overriding hand 0's), else hand 1's torch,
//! else a second weapon. The dresser attaches the main hand's weapon; this
//! module attaches the other one, for the bodies the spawn's equip pass
//! resolved. A shield is its own body with a back pivot to its forearm,
//! two face pivots to the left hand and a soft world weld ([`exa_physics::grip::shield_links`]); a
//! second weapon is the grip table's off-hand entry (5) on the left hand.
//!
//! The shield's model sits where the original's own placement puts it: the
//! hand's rest offset in the forearm's frame, turned -5° about z. Exarust
//! parents it under the hand's bone node turned the same way (the hand's
//! own rest turn is not undone), which is a guess at the last degree.
//!
//! Evidence: the shield attach and link builder, and the grip table.

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use exa_assets::{Game, ModelCache, ModelLoader, PackKind, model};
use exa_combat::attack::Slot;
use exa_combat::grip::{self, Grip};
use exa_formats::weapon_record::{Holder, Lengths, Section, ShieldRecord};
use exa_physics::body::Ragdoll;
use exa_physics::grip::{HeldGrip, HeldKind};
use exa_physics::grip_solve::GripSolveSet;

use crate::grip_runtime::{HeldRecord, PendingGrip};
use crate::{LEFT_HAND, RIGHT_HAND, RaceWeaponry, grip_bone, held_item_bodies};

/// What the off hand holds: a shield on the off arm, or a weapon in the
/// second held slot.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct OffHeld {
    /// An `Objects.rpk` model, or `@` and a factory name.
    pub model: String,
    /// The item's weapon section, for a second weapon.
    pub weapon: Option<exa_formats::objdb::Weapon>,
    /// The item's shield section, for a shield.
    pub shield: Option<ShieldRecord>,
    pub kind: OffKind,
}

/// Which off-hand item an [`OffHeld`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffKind {
    /// A weapon in the second held slot.
    Weapon,
    /// A shield on the off arm.
    Shield,
}

/// The shield's own turn on the hand, degrees about z (its placement
/// routine's only rotation).
pub const SHIELD_TURN_DEGREES: f32 = -5.0;
/// The shield's hand class (the low 16 bits of a shield item's class word):
/// the off-hand grip entry.
pub const SHIELD_CLASS: u32 = 0x10;

/// Marks an [`OffHeld`] already attached.
#[derive(Component)]
struct OffAttached;

/// The shield item a character carries in the off hand, whose equip gave it
/// a [`exa_combat::damage::Guard::Shield`]. The shield's own body exists
/// while it is held; when it leaves the world or its holder lets it go, the
/// guard goes back to open and the marker is removed.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShieldHold {
    /// The shield's item body (the entity carrying its [`HeldGrip`]).
    pub item: Entity,
}

/// The grip an off-hand weapon takes: the table's second slot.
fn off_weapon_grip(class: u32) -> Option<&'static Grip> {
    grip::grip(class & 0xFFFF, Slot::Second)
}

/// Attaches each [`OffHeld`] once its hand bones and model exist: its
/// model nodes, its own bodies, the grip links to the hands, and a second
/// weapon's striking blade. The holder's body may come later: the grip
/// waits for it ([`PendingGrip`]).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn attach_off_hands(
    mut commands: Commands,
    game: Option<Res<Game>>,
    mut bodies: Query<
        (
            Entity,
            &OffHeld,
            Option<&Ragdoll>,
            Option<&RaceWeaponry>,
            Option<&mut exa_combat::Fighter>,
        ),
        Without<OffAttached>,
    >,
    children: Query<&Children>,
    parts: Query<&exa_anim::BodyPart>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    let Some(game) = game else { return };
    for (body, off, ragdoll, race, mut fighter) in &mut bodies {
        let by_bone: std::collections::HashMap<usize, Entity> = children
            .iter_descendants(body)
            .filter_map(|e| parts.get(e).ok().map(|p| (p.bone, e)))
            .collect();
        let Some(&hand) = by_bone.get(&grip_bone(grip_hand(off))) else {
            continue; // the body's parts are not attached yet
        };
        let mut loader = ModelLoader {
            game: &game.0,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: model::bc_supported(support.as_deref()),
        };
        let (size, scale, holder) = fighter.as_deref().map_or((1.0, 1.0, Holder::AVERAGE), |f| {
            (f.stats.size, f.stats.scale, f.stats.holder())
        });
        let race = race.copied().unwrap_or_default();
        let Some(weapon) = model_of(&mut loader, &off.model) else {
            warn!("{} not found: no off-hand item", off.model);
            commands.entity(body).insert(OffAttached);
            continue;
        };
        let transform = placement(off, &race, size, holder);
        let (root, nodes) = exa_world::spawn_model_nodes(&mut commands, &weapon, Some(transform));
        commands.entity(root).insert(ChildOf(hand));
        commands.entity(body).insert(OffAttached);
        let item_bodies = held_item_bodies(&weapon, root, &nodes);
        if item_bodies.is_empty() {
            continue;
        }
        let names = |bone: usize| by_bone.get(&bone).copied();
        let arms = [
            names(crate::FOREARM_BONES[0]),
            names(crate::FOREARM_BONES[1]),
        ];
        let holder_key = ragdoll
            .and_then(|r| r.parts.first())
            .map(|p| p.entity)
            .unwrap_or(Entity::PLACEHOLDER);
        let (kind, class, second_hand_factor) = match off.kind {
            OffKind::Shield => (HeldKind::Shield, SHIELD_CLASS, 1.0),
            OffKind::Weapon => (
                HeldKind::Weapon,
                off.weapon.as_ref().map_or(0, |w| w.flags),
                1.0,
            ),
        };
        let spec = match off.kind {
            OffKind::Weapon => off
                .weapon
                .as_ref()
                .and_then(|w| off_weapon_grip(w.flags).map(|g| g.spec()))
                .unwrap_or(exa_physics::grip::GripSpec {
                    hand: 1,
                    links: Vec::new(),
                    second_hand: None,
                    arm: None,
                    extra: None,
                }),
            OffKind::Shield => exa_physics::grip::GripSpec {
                hand: 1,
                links: Vec::new(),
                second_hand: None,
                arm: None,
                extra: None,
            },
        };
        let grip = HeldGrip {
            spec,
            scale,
            size,
            second_hand_factor,
            flip: false,
            class: class & 0xFFFF,
            soft_weld: true,
            kind,
            holder_key,
            hands: [names(RIGHT_HAND), names(LEFT_HAND)],
            arms,
            links: None,
            released: false,
        };
        commands
            .entity(root)
            .insert((exa_physics::prop::PendingProp(item_bodies), grip));
        if ragdoll.is_none() {
            commands.entity(root).insert(PendingGrip { holder: body });
        }
        // A shield's equip gives its holder the shield guard: the record's
        // parry factor, leak and cost with its own shield techniques, and a
        // full arm. The guard leaves with the shield ([`drop_shield_guards`]).
        if off.kind == OffKind::Shield
            && let Some(record) = off.shield
            && let Some(fighter) = fighter.as_mut()
        {
            let perks = fighter.perks;
            fighter.guard = exa_combat::damage::Guard::Shield(exa_combat::damage::shield_guard(
                &record,
                perks.has(exa_formats::characters::Perk::Mobility),
                perks.has(exa_formats::characters::Perk::Endure),
            ));
            commands.entity(body).insert(ShieldHold { item: root });
        }
        // A second weapon strikes: its own blade beside the main hand's.
        if off.kind == OffKind::Weapon
            && let Some(section) = off.weapon.as_ref().map(|w| Section {
                flags: w.flags,
                stats: w.stats,
            })
        {
            // The model's own chain gives a plain weapon's lengths; a
            // factory weapon takes its assembled top (cm).
            let top = crate::weapon_length(&weapon, &meshes) * 100.0;
            let record = crate::held_record(
                &game.0,
                &off.model,
                || top,
                section.flags,
                section.stats,
                &holder,
                &race,
            );
            let lengths = Lengths {
                length: record.length,
                sweet_spot: record.sweet_spot,
            };
            commands
                .entity(root)
                .insert((
                    exa_combat::Blade::new(body, record.length / 100.0, record)
                        .in_slot(Slot::Second),
                    HeldRecord {
                        section,
                        lengths,
                        holder,
                    },
                ))
                .insert(exa_combat::impact::ImpactBook::default());
        }
    }
}

/// The hand the off-hand item is held in: both the shield and the second
/// weapon's grip entry name the left (the second held slot).
fn grip_hand(off: &OffHeld) -> exa_combat::grip::GripHand {
    match off.kind {
        OffKind::Weapon => off
            .weapon
            .as_ref()
            .and_then(|w| off_weapon_grip(w.flags))
            .map_or(exa_combat::grip::GripHand::Left, |g| g.hand),
        OffKind::Shield => exa_combat::grip::GripHand::Left,
    }
}

/// The item model: an object library entry, or a factory recipe.
fn model_of(loader: &mut ModelLoader, name: &str) -> Option<std::sync::Arc<model::Model>> {
    match name.strip_prefix('@') {
        Some(factory) => loader.factory_weapon(factory, None),
        None => loader.model(PackKind::Objects, name),
    }
}

/// Where the off-hand item's node sits under its hand: the second held
/// slot's grip for a weapon, and for a shield the hand's own place turned
/// [`SHIELD_TURN_DEGREES`] about z.
fn placement(off: &OffHeld, race: &RaceWeaponry, size: f32, holder: Holder) -> Transform {
    match off.kind {
        OffKind::Weapon => off
            .weapon
            .as_ref()
            .and_then(|w| off_weapon_grip(w.flags))
            .map_or(Transform::IDENTITY, |g| {
                g.placement(&race.grip_holder(size), false)
            }),
        OffKind::Shield => {
            let _ = holder;
            Transform::from_rotation(Quat::from_rotation_z(SHIELD_TURN_DEGREES.to_radians()))
        }
    }
}

/// Puts a holder's guard back to open when the shield its equip gave it is
/// no longer held: its item body left the world or its grip was released
/// (dropped, or its holder collapsed if a shield ever could). The marker
/// goes with it.
pub fn drop_shield_guards(
    mut commands: Commands,
    mut holders: Query<(Entity, &ShieldHold, &mut exa_combat::Fighter)>,
    held: Query<&HeldGrip>,
) {
    for (owner, hold, mut fighter) in &mut holders {
        if held.get(hold.item).is_ok_and(|grip| !grip.released) {
            continue;
        }
        if matches!(fighter.guard, exa_combat::damage::Guard::Shield(_)) {
            fighter.guard = exa_combat::damage::Guard::Open;
        }
        commands.entity(owner).remove::<ShieldHold>();
    }
}

/// Adds the off-hand attach pass: after the dresser has hung the main
/// hand's weapon and the hands are known.
pub struct OffHandPlugin;

impl Plugin for OffHandPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            attach_off_hands
                .run_if(resource_exists::<Game>)
                .after(super::equip_outfits),
        )
        .add_systems(FixedUpdate, drop_shield_guards.after(GripSolveSet::Build));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dropped_shield_puts_the_guard_back_to_open() {
        use exa_combat::damage::{Guard, ShieldGuard};
        let mut app = App::new();
        app.add_systems(Update, drop_shield_guards);
        let shield = app.world_mut().spawn_empty().id();
        let armed = Guard::Shield(ShieldGuard {
            parry: 0.875,
            leak: 0.71,
            cost: 0.71,
            stamina: 1.0,
            ..ShieldGuard::default()
        });
        let owner = app
            .world_mut()
            .spawn((
                exa_combat::Fighter {
                    guard: armed,
                    ..exa_combat::Fighter::default()
                },
                ShieldHold { item: shield },
            ))
            .id();
        // A held shield (its grip exists and is not released) keeps the
        // guard.
        app.world_mut()
            .entity_mut(shield)
            .insert(held(HeldKind::Shield));
        app.update();
        assert_eq!(
            app.world().get::<exa_combat::Fighter>(owner).unwrap().guard,
            armed
        );
        assert!(app.world().get::<ShieldHold>(owner).is_some());
        // Letting it go, or losing the item, opens the guard and drops the
        // marker.
        app.world_mut()
            .entity_mut(shield)
            .insert(held(HeldKind::Shield));
        app.world_mut()
            .get_mut::<HeldGrip>(shield)
            .unwrap()
            .released = true;
        app.update();
        assert_eq!(
            app.world().get::<exa_combat::Fighter>(owner).unwrap().guard,
            Guard::Open
        );
        assert!(app.world().get::<ShieldHold>(owner).is_none());
    }

    /// A minimal held item: the grip record with no links yet.
    fn held(kind: HeldKind) -> HeldGrip {
        HeldGrip {
            spec: exa_physics::grip::GripSpec {
                hand: 1,
                links: Vec::new(),
                second_hand: None,
                arm: None,
                extra: None,
            },
            scale: 1.0,
            size: 1.0,
            second_hand_factor: 1.0,
            flip: false,
            class: SHIELD_CLASS,
            soft_weld: true,
            kind,
            holder_key: Entity::PLACEHOLDER,
            hands: [None, None],
            arms: [None, None],
            links: None,
            released: false,
        }
    }

    #[test]
    fn a_second_weapon_takes_the_off_hand_grip_and_a_shield_its_own_class() {
        let sword = exa_formats::objdb::Weapon {
            flags: 0x11,
            seeds: [0; 2],
            quality: 0,
            damage: 0,
            dirt: 0,
            materials: [(0, 0, 0, 0); 4],
            selection: [0; 10],
            stats: exa_formats::weapon_stats::StatBytes::default(),
        };
        let off = OffHeld {
            model: "ub05ssworda".into(),
            weapon: Some(sword),
            shield: None,
            kind: OffKind::Weapon,
        };
        let g = off_weapon_grip(0x11).unwrap();
        assert_eq!(g.hand, exa_combat::grip::GripHand::Left);
        assert_eq!(grip_hand(&off), exa_combat::grip::GripHand::Left);
        // The shield's hand class is the off-hand entry too, and its turn is
        // the original's -5°.
        assert_eq!(SHIELD_CLASS, 0x10);
        assert_eq!(SHIELD_TURN_DEGREES, -5.0);
        let shield = OffHeld {
            model: "shield_a".into(),
            weapon: None,
            shield: Some(ShieldRecord {
                weight: 0.6,
                block: 0.58,
                bulk: 0.3,
            }),
            kind: OffKind::Shield,
        };
        assert_eq!(grip_hand(&shield), exa_combat::grip::GripHand::Left);
        let t = placement(&shield, &RaceWeaponry::default(), 1.0, Holder::AVERAGE);
        let want = Quat::from_rotation_z((-5.0f32).to_radians());
        assert!(t.rotation.abs_diff_eq(want, 1e-6));
    }
}
