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
use exa_formats::weapon_stats::at;
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
    /// The item starts turned over (its own record hand's flip flag): its
    /// record is built from the flipped side and its grip at the far end of
    /// the flip animation.
    pub flip: bool,
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

/// The shield a character carries in the off hand and the block factors its
/// equip wrote on the holder ([`exa_combat::damage::shield_guard`]). The
/// factors are kept here, not put in the fighter's guard: the original
/// blocks only a blow that strikes the shield itself, and Exarust strikes
/// limbs, never a shield body, so a standing guard would turn every blow
/// on the holder into a block. When the shield leaves the world or its
/// holder lets it go, the marker is removed.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ShieldHold {
    /// The shield's item body (the entity carrying its [`HeldGrip`]).
    pub item: Entity,
    /// The guard the shield gives a blow that strikes it.
    pub guard: exa_combat::damage::ShieldGuard,
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
            Option<&exa_combat::Fighter>,
            Has<crate::HalfSword>,
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
    for (body, off, ragdoll, race, fighter, half_sword) in &mut bodies {
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
        let (size, scale, mut holder) = fighter.map_or((1.0, 1.0, Holder::AVERAGE), |f| {
            (f.stats.size, f.stats.scale, f.stats.holder())
        });
        holder.half_sword = half_sword;
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
        info!(
            "off hand: {} as a {:?}{}",
            off.model,
            off.kind,
            if off.flip { ", flipped" } else { "" }
        );
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
            flip: off.flip,
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
        // A shield's equip writes its block factors on the holder: the
        // record's parry factor, leak and cost with its own shield
        // techniques, and a full arm. They wait on the marker, which leaves
        // with the shield ([`drop_shield_holds`]); the fighter's guard is
        // not changed.
        if off.kind == OffKind::Shield
            && let Some(record) = off.shield
            && let Some(fighter) = fighter
        {
            let perks = fighter.perks;
            let guard = exa_combat::damage::shield_guard(
                &record,
                perks.has(exa_formats::characters::Perk::Mobility),
                perks.has(exa_formats::characters::Perk::Endure),
            );
            commands
                .entity(body)
                .insert(ShieldHold { item: root, guard });
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
                off.flip,
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
                    crate::grip_runtime::HeldItemSpec {
                        class: (section.flags & 0xFFFF) as u16,
                        start_byte: section.stats.get(at::START),
                        item_cm: top,
                        race_reach: race.weapons.unarmed_reach,
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
                g.placement(&race.grip_holder(size), off.flip)
            }),
        OffKind::Shield => {
            let _ = holder;
            Transform::from_rotation(Quat::from_rotation_z(SHIELD_TURN_DEGREES.to_radians()))
        }
    }
}

/// Removes a holder's [`ShieldHold`] when its shield is no longer held: its
/// item body left the world or its grip was released (dropped, or its
/// holder collapsed if a shield ever could).
pub fn drop_shield_holds(
    mut commands: Commands,
    holders: Query<(Entity, &ShieldHold)>,
    held: Query<&HeldGrip>,
) {
    for (owner, hold) in &holders {
        if held.get(hold.item).is_ok_and(|grip| !grip.released) {
            continue;
        }
        commands.entity(owner).remove::<ShieldHold>();
    }
}

/// Puts the equip and attach passes' result on a spawned character's model
/// entity and returns the [`Hands`](crate::held::Hands) its moveset follows.
///
/// The main hand's weapon and a second weapon (beside it) or a shield (from
/// either hand) pick the moveset. The main weapon's own flip flag gives
/// [`HeldFlipped`](crate::HeldFlipped); the last weapon's half-sword flag
/// gives [`HalfSword`](crate::HalfSword). The off-hand item the attach
/// pass hangs (a shield, else hand 1's torch, else a second weapon) becomes
/// an [`OffHeld`] with its own flip; the off-hand torch is not attached
/// (the equip pass leaves it on its unlit side, and Exarust's carried torch
/// always burns). The main weapon itself is the caller's `Wielded`.
pub fn insert_equipped(
    entity: &mut EntityCommands,
    equip: &[Option<crate::spawn::Equipped>; 2],
) -> crate::held::Hands {
    use crate::spawn::{Attach, attach_plan, half_sword, holds_flipped};
    let hands = hands_of(equip);
    let off = attach_plan(equip)
        .into_iter()
        .find(|p| p.kind != Attach::Weapon);
    if holds_flipped(equip).unwrap_or(false) {
        entity.insert(crate::HeldFlipped);
    }
    if half_sword(equip).unwrap_or(false) {
        entity.insert(crate::HalfSword);
    }
    if let Some(plan) = off
        && plan.kind != Attach::Torch
        && let Some(item) = equip[plan.hand].as_ref()
    {
        entity.insert(OffHeld {
            model: item.model.clone(),
            weapon: item.weapon,
            shield: item.shield_record,
            flip: plan.flipped,
            kind: if plan.kind == Attach::Shield {
                OffKind::Shield
            } else {
                OffKind::Weapon
            },
        });
    }
    hands
}

/// The [`Hands`](crate::held::Hands) the equip pass's result gives the
/// moveset: the main hand's weapon, a shield from either hand (the attach
/// pass's off-arm shield), else hand 1's weapon as the second beside a main
/// weapon.
pub fn hands_of(equip: &[Option<crate::spawn::Equipped>; 2]) -> crate::held::Hands {
    use crate::spawn::{Attach, EquipKind, attach_plan};
    let mut hands = crate::held::Hands::default();
    if let Some(item) = &equip[0]
        && item.kind == EquipKind::Weapon
        && let Some(weapon) = item.weapon
    {
        hands.main = Some(crate::held::HeldItem::weapon(weapon.flags));
    }
    let plan = attach_plan(equip);
    let off = plan.iter().find(|p| p.kind != Attach::Weapon);
    match off.map(|p| (p.kind, equip[p.hand].as_ref())) {
        Some((Attach::Shield, _)) => {
            hands.shield = Some(crate::held::ShieldStyle::Plain);
        }
        Some((Attach::SecondWeapon, Some(item))) if hands.main.is_some() => {
            hands.second = item.weapon.map(|w| crate::held::HeldItem::weapon(w.flags));
        }
        _ => {}
    }
    hands
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
        .add_systems(FixedUpdate, drop_shield_holds.after(GripSolveSet::Build));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn equipped(model: &str, kind: crate::spawn::EquipKind, flags: u32) -> crate::spawn::Equipped {
        crate::spawn::Equipped {
            model: model.into(),
            kind,
            weapon: (kind == crate::spawn::EquipKind::Weapon).then_some(
                exa_formats::objdb::Weapon {
                    flags: 0x11,
                    seeds: [0; 2],
                    quality: 0,
                    damage: 0,
                    dirt: 0,
                    materials: [(0, 0, 0, 0); 4],
                    selection: [0; 10],
                    stats: exa_formats::weapon_stats::StatBytes::default(),
                },
            ),
            shield_record: None,
            flags,
        }
    }

    /// The shipped levels' placed characters through the equip and attach
    /// passes: which off-hand items they hang and which hand flags they set
    /// (skipped without the game).
    #[test]
    fn shipped_placed_characters_hang_their_off_hands() {
        use crate::spawn::{
            Attach, HandEntry, attach_plan, equip_hands, half_sword, holds_flipped,
        };
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let resource = game.pack(PackKind::Resource).unwrap();
        let db = exa_formats::rdb::Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let items = exa_formats::objdb::items(&db);
        let (mut shields, mut seconds, mut torches, mut flipped, mut half) = (0, 0, 0, 0, 0);
        let mut levels = 0;
        for entry in resource.entries() {
            let name = entry.name.to_ascii_lowercase();
            if !name.ends_with(".rfc") {
                continue;
            }
            let Some(scene) = resource
                .get(&name)
                .and_then(|b| exa_formats::rfc::Scene::parse(b).ok())
            else {
                continue;
            };
            if scene.characters.is_none() {
                continue;
            }
            levels += 1;
            let before = shields + seconds;
            let locale = exa_formats::roles::locale_id(resource.get("locales.rdb").unwrap(), &name);
            for c in exa_world::placed_characters(&scene, &items, locale) {
                if c.dead {
                    continue;
                }
                let equip = equip_hands(&c.record, |hand, _| {
                    let item = c.hands.get(hand)?.as_ref()?;
                    Some(HandEntry {
                        model: item.model.clone(),
                        weapon: item.weapon,
                        shield: item.shield,
                        shield_record: item.shield_record,
                        torch: item.torch,
                    })
                });
                for plan in attach_plan(&equip) {
                    match plan.kind {
                        Attach::Shield => shields += 1,
                        Attach::SecondWeapon => seconds += 1,
                        Attach::Torch => torches += 1,
                        Attach::Weapon => {}
                    }
                }
                flipped += usize::from(holds_flipped(&equip) == Some(true));
                half += usize::from(half_sword(&equip) == Some(true));
            }
            if shields + seconds > before {
                eprintln!("{name}: {} off-hand items", shields + seconds - before);
            }
        }
        eprintln!(
            "levels {levels}: shields {shields}, second weapons {seconds}, off torches {torches}, flipped {flipped}, half-sword {half}"
        );
        assert_eq!(levels, 13);
        // No level places an off-hand torch or sets either hand flag; they
        // come from the arena's and the saves' records.
        assert_eq!(
            (shields, seconds, torches, flipped, half),
            (27, 12, 0, 0, 0)
        );
    }

    /// The equip result on a spawned model: the main weapon's flip, the
    /// last weapon's half-sword hold and the second weapon's own flip.
    #[test]
    fn the_equipped_hands_mark_the_model() {
        use crate::spawn::EquipKind;
        let mut world = World::new();
        let model = world.spawn_empty().id();
        let equip = [
            Some(equipped("sword", EquipKind::Weapon, 0x0200_0000)),
            Some(equipped("dagger", EquipKind::Weapon, 0x0100_0000)),
        ];
        let hands = {
            let mut commands = world.commands();
            insert_equipped(&mut commands.entity(model), &equip)
        };
        world.flush();
        assert_eq!(hands.main.unwrap().flags, 0x11);
        assert_eq!(hands.second.unwrap().flags, 0x11);
        assert!(world.get::<crate::HeldFlipped>(model).is_some());
        assert!(world.get::<crate::HalfSword>(model).is_some());
        let off = world.get::<OffHeld>(model).unwrap();
        assert_eq!(
            (off.model.as_str(), off.kind, off.flip),
            ("dagger", OffKind::Weapon, false)
        );
        // A shield in the main hand goes on the off arm; no weapon, so no
        // flip and no hold.
        let model = world.spawn_empty().id();
        let equip = [
            Some(equipped("buckler", EquipKind::Shield, 0x0300_0000)),
            None,
        ];
        let hands = {
            let mut commands = world.commands();
            insert_equipped(&mut commands.entity(model), &equip)
        };
        world.flush();
        assert_eq!(hands.shield, Some(crate::held::ShieldStyle::Plain));
        assert!(hands.main.is_none());
        assert!(world.get::<crate::HeldFlipped>(model).is_none());
        assert!(world.get::<crate::HalfSword>(model).is_none());
        assert_eq!(world.get::<OffHeld>(model).unwrap().kind, OffKind::Shield);
        // The off hand's torch is not hung, and a lone second weapon is
        // attached but does not make a two-weapon moveset.
        let model = world.spawn_empty().id();
        let equip = [None, Some(equipped("torch", EquipKind::Torch, 0))];
        let hands = {
            let mut commands = world.commands();
            insert_equipped(&mut commands.entity(model), &equip)
        };
        world.flush();
        assert_eq!(hands, crate::held::Hands::default());
        assert!(world.get::<OffHeld>(model).is_none());
        let equip = [None, Some(equipped("axe", EquipKind::Weapon, 0x0200_0000))];
        let lone = hands_of(&equip);
        assert!(lone.second.is_none() && lone.main.is_none());
    }

    /// A held shield keeps its marker and leaves the fighter's guard
    /// alone; letting it go removes the marker.
    #[test]
    fn a_dropped_shield_takes_its_hold_away() {
        use exa_combat::damage::{Guard, ShieldGuard};
        let mut app = App::new();
        app.add_systems(Update, drop_shield_holds);
        let shield = app.world_mut().spawn(held(HeldKind::Shield)).id();
        let guard = ShieldGuard {
            parry: 0.875,
            leak: 0.71,
            cost: 0.71,
            stamina: 1.0,
            ..ShieldGuard::default()
        };
        let owner = app
            .world_mut()
            .spawn((
                exa_combat::Fighter::default(),
                ShieldHold {
                    item: shield,
                    guard,
                },
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<ShieldHold>(owner).unwrap().guard, guard);
        assert_eq!(
            app.world().get::<exa_combat::Fighter>(owner).unwrap().guard,
            Guard::Open
        );
        app.world_mut()
            .get_mut::<HeldGrip>(shield)
            .unwrap()
            .released = true;
        app.update();
        assert!(app.world().get::<ShieldHold>(owner).is_none());
        // A shield whose item left the world goes the same way.
        let other = app.world_mut().spawn(held(HeldKind::Shield)).id();
        app.world_mut()
            .entity_mut(owner)
            .insert(ShieldHold { item: other, guard });
        app.world_mut().entity_mut(other).despawn();
        app.update();
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
            flip: false,
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
            flip: false,
            kind: OffKind::Shield,
        };
        assert_eq!(grip_hand(&shield), exa_combat::grip::GripHand::Left);
        let t = placement(&shield, &RaceWeaponry::default(), 1.0, Holder::AVERAGE);
        let want = Quat::from_rotation_z((-5.0f32).to_radians());
        assert!(t.rotation.abs_diff_eq(want, 1e-6));
    }

    #[test]
    fn a_second_weapon_starts_flipped_when_its_hand_asks_for_it() {
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
        let make = |flip| OffHeld {
            model: "ub05ssworda".into(),
            weapon: Some(sword),
            shield: None,
            flip,
            kind: OffKind::Weapon,
        };
        let race = RaceWeaponry::default();
        let plain = placement(&make(false), &race, 1.0, Holder::AVERAGE);
        let flipped = placement(&make(true), &race, 1.0, Holder::AVERAGE);
        // The flipped item takes the grip's half turn about y.
        let want = plain.rotation * Quat::from_rotation_y(std::f32::consts::PI);
        assert!(flipped.rotation.abs_diff_eq(want, 1e-6));
        assert_eq!(flipped.translation, plain.translation);
    }
}
