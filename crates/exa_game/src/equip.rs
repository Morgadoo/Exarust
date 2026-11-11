//! What a character's record says it holds becomes the hands the moveset
//! and the player light read.
//!
//! The record keeps two weapon sets of two hands each and the view's apply
//! step trades them when a swap re-dresses the hands. Exarust carries the
//! plan of that step as [`PendingHands`]; this module turns the record's
//! out set into the [`Hands`] component once the swap has landed: the main
//! hand's weapon, a second weapon in the off hand, the off hand's shield,
//! and a torch in the off hand (an off-hand weapon that also has a light
//! section is the light, as the attach rule has it). A hand holding an item
//! with none of those sections is empty for the moveset.
//!
//! Evidence: the character view's attachment plan and apply step, read in
//! the instructions (`context/re/item-gui-lane.md`).

use bevy::prelude::*;
use exa_character::held::{Hands, HeldItem, ShieldStyle};
use exa_formats::characters::ID_MASK;
use exa_formats::encumbrance::{Skills, encumbrance, head_extra};
use exa_formats::objdb::Armour;
use exa_items::catalogue::{ItemCatalogue, PendingHands};
use exa_items::possession::{ItemKind, Possessions};

use crate::torch::CarriesTorch;

/// A character's encumbrance from what its record currently wears and
/// holds: the worn items' burdens, the shield of the first two hands (the
/// second wins, as [`exa_formats::encumbrance::held_shield`] picks it), and
/// the character's armour and shield techniques. An item the catalogue
/// does not know weighs nothing, as a missing record does in the original's
/// armour pass. Ids are looked up by their low 30 bits, as the spawn's sum
/// looks them up in the base database, so the two agree.
pub fn encumbrance_of(p: &Possessions, catalogue: &ItemCatalogue, skills: Skills) -> f32 {
    let worn: Vec<_> = p
        .worn
        .iter()
        .filter_map(|worn| catalogue.burden(worn.id & ID_MASK))
        .collect();
    let shield = [p.out[1], p.out[0]]
        .into_iter()
        .filter(|hand| hand.id != 0)
        .map(|hand| hand.id & ID_MASK)
        .filter(|&id| {
            catalogue
                .kind(id)
                .is_some_and(|kind| kind.shield && kind.weapon.is_none())
        })
        .find_map(|id| catalogue.shield_record(id));
    encumbrance(&worn, shield.as_ref(), skills)
}

/// Refreshes a body's encumbrance when its possessions change, as the
/// armour pass does after an equip, unequip or swap. Both copies are
/// written: the body's [`Physical::stats`](exa_physics::body::Physical)
/// (gait, turn and step reach) and its fighter's stats (footwork and the
/// movement stage). The body's live record supplies the techniques. A
/// body's first frame keeps the value its spawn summed from the level's
/// own item list.
#[allow(clippy::type_complexity)]
pub fn refresh_encumbrance(
    catalogue: Option<Res<ItemCatalogue>>,
    mut bodies: Query<
        (
            Ref<Possessions>,
            Option<&exa_character::record::CharacterData>,
            &mut exa_physics::body::Physical,
            Option<&mut exa_combat::Fighter>,
        ),
        Changed<Possessions>,
    >,
) {
    let Some(catalogue) = catalogue.as_deref() else {
        return;
    };
    for (possessions, data, mut physical, fighter) in &mut bodies {
        if possessions.is_added() {
            continue;
        }
        let skills = data.map_or_else(Skills::default, |data| Skills::of_record(&data.record));
        let value = encumbrance_of(&possessions, catalogue, skills);
        physical.stats.encumbrance = value;
        if let Some(mut fighter) = fighter {
            fighter.stats.encumbrance = value;
        }
    }
}

/// The armour pass's worn-item rules, run again when a body's possessions
/// change (an equip, an unequip or a swap) as the original runs the whole
/// pass on every equipment refresh; the spawn ran them first:
/// - the special kinds ([`exa_formats::encumbrance::WornSpecials`]): kind 2
///   grants the second wind, 3/4/6 register their item effects, 4 marks
///   the golem suit, 5 wards the mind, 7 quickens the focus;
/// - the item-effect list: the pass first drops every worn item's entry,
///   then registers the worn ones in wearing order; an item still worn
///   keeps the bytes its entry carries;
/// - the golem fix: a golem race's red regeneration is blocked unless it
///   wears its core item, an item without an apparel section
///   ([`exa_formats::character_stats::golem_rules`]).
///
/// A body's first frame keeps what its spawn set.
#[allow(clippy::type_complexity)]
pub fn refresh_worn_specials(
    mut commands: Commands,
    catalogue: Option<Res<ItemCatalogue>>,
    mut bodies: Query<
        (
            Entity,
            Ref<Possessions>,
            &mut exa_combat::Fighter,
            Option<&mut crate::item_effects::WornEffects>,
            Option<&mut crate::thaum::cast::PowerCaster>,
        ),
        Changed<Possessions>,
    >,
) {
    let Some(catalogue) = catalogue.as_deref() else {
        return;
    };
    for (entity, possessions, mut fighter, effects, caster) in &mut bodies {
        if possessions.is_added() {
            continue;
        }
        let worn: Vec<(u32, Option<exa_formats::objdb::Burden>)> = possessions
            .worn
            .iter()
            .map(|w| (w.id, catalogue.burden(w.id & ID_MASK)))
            .collect();
        let specials = exa_formats::encumbrance::WornSpecials::of_worn(
            worn.iter().filter_map(|&(id, b)| Some((id, b?))),
        );
        let rules = exa_formats::character_stats::golem_rules(
            fighter.stats.rules,
            fighter.stats.golem_core,
            worn.iter().filter(|(_, b)| b.is_none()).map(|&(id, _)| id),
        );
        if fighter.stats.rules != rules || fighter.second_wind.granted != specials.second_wind {
            fighter.stats.rules = rules;
            fighter.second_wind.granted = specials.second_wind;
        }
        let fresh = crate::item_effects::WornEffects::of_specials(&specials, rules.0);
        match (effects, fresh) {
            (Some(mut old), Some(mut new)) => {
                for item in &mut new.items {
                    if let Some(kept) = old.items.iter().find(|o| o.id == item.id) {
                        *item = *kept;
                    }
                }
                new.list.impacts = old.list.impacts;
                new.list.ticks = old.list.ticks;
                new.next_run_ms = old.next_run_ms;
                if *old != new {
                    *old = new;
                }
            }
            (Some(mut old), None) => {
                // Every worn item's entry is dropped; the list stays, empty.
                if !old.list.entries.is_empty() || !old.items.is_empty() {
                    old.list.entries.clear();
                    old.items.clear();
                }
            }
            (None, Some(new)) => {
                commands.entity(entity).insert(new);
            }
            (None, None) => {}
        }
        let suit = specials
            .effects
            .iter()
            .any(|item| item.special == exa_formats::objdb::Burden::GOLEM_SUIT);
        if suit {
            commands.entity(entity).insert(exa_combat::GolemSuit);
        } else {
            commands.entity(entity).remove::<exa_combat::GolemSuit>();
        }
        if specials.mind_ward {
            commands.entity(entity).insert(exa_ai::brain::MindWarded);
        } else {
            commands
                .entity(entity)
                .remove::<exa_ai::brain::MindWarded>();
        }
        if let Some(mut caster) = caster {
            let before = (caster.body.item_flags, caster.body.regen_bonus.to_bits());
            let mut next = caster.clone();
            next.apply_worn(&specials);
            if (next.body.item_flags, next.body.regen_bonus.to_bits()) != before {
                *caster = next;
            }
        }
        debug!(
            "armour pass on {entity:?}: rules {:#x}, second wind {}, ward {}, suit {suit}, \
             focus bonus {}",
            rules.0, specials.second_wind, specials.mind_ward, specials.focus_regen
        );
    }
}

/// The eleventh body slot: the armour pass reads a worn piece's display only
/// while walking that node. A body without it still takes the physique offset.
const HEAD_SLOT: usize = 10;

/// The head height the armour pass measured, onto each held weapon, and onto
/// an arm link that is already built. A two-hander or a polearm is the only
/// grip the pass moves. This runs before the grip builder, so a weapon whose
/// links do not exist yet is shifted when they are built. Without a character
/// record the physique is the average (0.5), which adds nothing.
pub fn apply_head_height(
    catalogue: Option<Res<ItemCatalogue>>,
    bodies: Query<(
        &exa_physics::body::Ragdoll,
        Option<&Possessions>,
        Option<&exa_character::record::CharacterData>,
    )>,
    mut held: Query<&mut exa_physics::grip::HeldGrip>,
) {
    let Some(catalogue) = catalogue.as_deref() else {
        return;
    };
    let armed = |grip: &exa_physics::grip::HeldGrip| {
        grip.kind == exa_physics::grip::HeldKind::Weapon
            && grip.class & exa_physics::grip::ARMED_GRIP != 0
    };
    let holders = exa_physics::body::first_by_key(
        held.iter()
            .filter(|grip| armed(grip))
            .map(|grip| grip.holder_key),
        bodies.iter().map(|found| (found, found.0)),
    );
    for mut grip in &mut held {
        if !armed(&grip) {
            continue;
        }
        let Some(&(body, possessions, data)) = holders.get(&grip.holder_key) else {
            continue;
        };
        let physique = data.map_or(0.5, |data| data.record.physique[0]);
        let worn: Vec<Armour> = if body.parts.iter().any(|part| part.bone == HEAD_SLOT) {
            possessions.map_or_else(Vec::new, |possessions| {
                possessions
                    .worn
                    .iter()
                    .filter_map(|worn| catalogue.burden(worn.id & ID_MASK))
                    .map(|burden| Armour {
                        display: burden.display,
                        head_height_mark: burden.head_height_mark,
                        ..Armour::default()
                    })
                    .collect()
            })
        } else {
            Vec::new()
        };
        let extra = head_extra(&worn, physique);
        if grip.head_extra.to_bits() == extra.to_bits() {
            continue;
        }
        let part = grip.spec.arm.map(|pair| pair.part);
        grip.head_extra = extra;
        if let (Some(links), Some(part)) = (grip.links.as_mut(), part) {
            exa_physics::grip::raise_arm(links, body, part, extra);
        }
    }
}

/// The hands a character's out set makes. `kind` gives an item's sections.
pub fn hands_of(p: &Possessions, kind: impl Fn(u32) -> Option<ItemKind>) -> Hands {
    let mut hands = Hands::default();
    let out = &p.out;
    if let Some(k) = (out[0].id != 0).then(|| kind(out[0].id)).flatten()
        && let Some(flags) = k.weapon
    {
        hands.main = Some(HeldItem {
            flags: (flags & 0xFFFF) as u16,
            light: k.light,
        });
    }
    if let Some(k) = (out[1].id != 0).then(|| kind(out[1].id)).flatten() {
        if let Some(flags) = k.weapon {
            if k.light {
                hands.off_light = true;
            } else {
                hands.second = Some(HeldItem {
                    flags: (flags & 0xFFFF) as u16,
                    light: false,
                });
            }
        } else if k.shield {
            hands.shield = Some(ShieldStyle::Plain);
        } else if k.light {
            hands.off_light = true;
        }
    }
    hands
}

/// The hands after a swap, keeping Exarust's carried torch: the torch is
/// outside the record, so the hand it is in keeps it whatever the record's
/// out set holds there.
pub fn keep_torch(mut next: Hands, before: &Hands) -> Hands {
    if let Some(main) = before.main
        && main.light
        && main.flags == crate::torch::TORCH_FLAGS
    {
        next.main = Some(main);
    }
    if before.off_light {
        next.off_light = true;
        next.second = None;
        next.shield = None;
    }
    next
}

/// Applies the plan a swap left on the body: the hands follow the record's
/// out set and the plan is consumed. The plan itself is the record side of
/// the work; the models are attached by the renderer's own draw of the
/// hands, so nothing is spawned here. A body carrying Exarust's torch keeps
/// it in its hand ([`keep_torch`]).
pub fn apply_pending_hands(
    mut commands: Commands,
    catalogue: Option<Res<ItemCatalogue>>,
    mut bodies: Query<(Entity, &Possessions, &mut Hands, Has<CarriesTorch>), With<PendingHands>>,
) {
    for (body, possessions, mut hands, torch) in &mut bodies {
        let mut next = hands_of(possessions, |id| {
            catalogue.as_ref().and_then(|c| c.kind(id))
        });
        if torch {
            next = keep_torch(next, &hands);
        }
        hands.set_if_neq(next);
        commands.entity(body).remove::<PendingHands>();
    }
}

/// The record's side of a collapse's drop: on the tick a collapse starts,
/// the holder's came-loose hook runs on its possessions
/// ([`Possessions::forget_loose`]): every out hand that does not hold a
/// shield loses its item and that item's held entry leaves the carried
/// list, and a stowed hand whose item is gone is cleared. Only a body that
/// held something in a non-shield hand drops anything (the hook runs once
/// per dropped item). An item the catalogue does not know is not a shield.
pub fn forget_collapsed_hands(
    catalogue: Option<Res<ItemCatalogue>>,
    mut bodies: Query<&mut Possessions, Added<exa_combat::collapse::Collapse>>,
) {
    let is_shield = |id: u32| {
        catalogue
            .as_ref()
            .and_then(|c| c.kind(id & ID_MASK))
            .is_some_and(|k| k.shield && k.weapon.is_none())
    };
    for mut possessions in &mut bodies {
        if !possessions
            .out
            .iter()
            .any(|hand| hand.id != 0 && !is_shield(hand.id))
        {
            continue;
        }
        possessions.forget_loose(is_shield);
        info!(
            "a collapse drops the held items from the record: out set {:?}",
            possessions.out.map(|h| h.id)
        );
    }
}

/// Applies the hand plans of swapped weapon sets, and runs the bodies' worn
/// item effects once a fixed tick.
pub struct EquipPlugin;

impl Plugin for EquipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            apply_pending_hands
                .after(exa_items::catalogue::swap_weapon_sets)
                .before(refresh_encumbrance),
        )
        .add_systems(Update, refresh_encumbrance)
        .add_systems(Update, refresh_worn_specials.after(refresh_encumbrance))
        .add_systems(Update, forget_collapsed_hands.before(refresh_encumbrance))
        .add_systems(
            FixedUpdate,
            apply_head_height.before(exa_physics::grip_solve::GripSolveSet::Build),
        )
        .add_systems(
            FixedUpdate,
            crate::item_effects::run_worn_effects
                .after(exa_combat::regen::HealthTick)
                .before(exa_physics::body::CharacterTicks),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_items::possession::Hand;

    fn item(weapon: Option<u32>, shield: bool, light: bool) -> ItemKind {
        ItemKind {
            weapon,
            shield,
            light,
            sight: false,
        }
    }

    fn kind_of(id: u32) -> Option<ItemKind> {
        Some(match id {
            5 => item(Some(0x521), false, false),
            6 => item(Some(0x11), false, false),
            7 => item(None, true, false),
            8 => item(None, false, true),
            9 => item(Some(0x10), false, true),
            _ => return None,
        })
    }

    #[test]
    fn the_out_set_makes_the_hands() {
        let p = Possessions {
            out: [Hand { id: 5, flags: 0 }, Hand { id: 6, flags: 0 }],
            ..Default::default()
        };
        let hands = hands_of(&p, kind_of);
        assert_eq!(hands.main.unwrap().flags, 0x521);
        assert_eq!(hands.second.unwrap().flags, 0x11);
        assert!(hands.shield.is_none() && !hands.off_light);
        // A shield in the off hand.
        let p = Possessions {
            out: [Hand { id: 5, flags: 0 }, Hand { id: 7, flags: 0 }],
            ..Default::default()
        };
        let hands = hands_of(&p, kind_of);
        assert_eq!(hands.shield, Some(ShieldStyle::Plain));
        assert!(hands.second.is_none());
        // A light in the off hand.
        let p = Possessions {
            out: [Hand { id: 0, flags: 0 }, Hand { id: 8, flags: 0 }],
            ..Default::default()
        };
        let hands = hands_of(&p, kind_of);
        assert!(hands.main.is_none() && hands.off_light);
        // A weapon that also has a light section is the off-hand light.
        let p = Possessions {
            out: [Hand { id: 5, flags: 0 }, Hand { id: 9, flags: 0 }],
            ..Default::default()
        };
        let hands = hands_of(&p, kind_of);
        assert!(hands.off_light && hands.second.is_none() && hands.shield.is_none());
        // An item with no section in the main hand leaves it empty.
        let p = Possessions {
            out: [Hand { id: 7, flags: 0 }, Hand { id: 0, flags: 0 }],
            ..Default::default()
        };
        assert!(hands_of(&p, kind_of).main.is_none());
    }

    /// A catalogue with one worn body (encumbrance byte `worn`) and one
    /// shield whose weight, block and bulk bytes are given.
    fn catalogue(worn: u8, shield: [u8; 3]) -> ItemCatalogue {
        let mut body = vec![0u8; 0x1B0];
        body[0x1A5] = worn;
        let mut shield_body = vec![0u8; 8];
        shield_body[4..7].copy_from_slice(&shield);
        let section = |tag: u32, body: &[u8]| {
            let mut out = Vec::new();
            out.extend_from_slice(&tag.to_le_bytes());
            out.extend_from_slice(&((8 + body.len()) as u32).to_le_bytes());
            out.extend_from_slice(body);
            out
        };
        let first = section(0x1000, &body);
        let second = section(0x8000, &shield_body);
        let mut file = Vec::new();
        file.extend_from_slice(&exa_formats::objdb::MAGIC.to_le_bytes());
        file.extend_from_slice(&48u32.to_le_bytes());
        for (id, offset, len) in [
            (10u32, 0u32, first.len() as u32),
            (20u32, first.len() as u32, second.len() as u32),
        ] {
            file.extend_from_slice(&id.to_le_bytes());
            file.extend_from_slice(&0u32.to_le_bytes());
            file.extend_from_slice(&offset.to_le_bytes());
            file.extend_from_slice(&len.to_le_bytes());
        }
        // The slot after the last entry, which the loader does not count.
        file.extend_from_slice(&[0; 16]);
        file.extend_from_slice(&first);
        file.extend_from_slice(&second);
        let db = exa_formats::rdb::Database::parse(&file).unwrap();
        ItemCatalogue::of_database(&db)
    }

    #[test]
    fn the_encumbrance_follows_the_worn_items_and_the_shield() {
        let catalogue = catalogue(10, [100, 0, 100]);
        let p = Possessions {
            out: [Hand::default(), Hand { id: 20, flags: 0 }],
            worn: vec![exa_formats::characters::ItemRef { id: 10, flags: 0 }],
            ..Default::default()
        };
        // 10 × 0.0025 plus the shield's (2 − 0) × 1 × 0.25.
        assert!((encumbrance_of(&p, &catalogue, Skills::default()) - 0.525).abs() < 1e-6);
        // An unknown worn item weighs nothing, and a weapon in the same
        // hand is not the shield.
        let p = Possessions {
            out: [Hand { id: 10, flags: 0 }, Hand::default()],
            worn: vec![exa_formats::characters::ItemRef { id: 99, flags: 0 }],
            ..Default::default()
        };
        assert_eq!(encumbrance_of(&p, &catalogue, Skills::default()), 0.0);

        // A level-local (tagged) id weighs as its base record, as the
        // spawn's sum looks it up.
        let p = Possessions {
            out: [
                Hand::default(),
                Hand {
                    id: 20 | 0x8000_0000,
                    flags: 0,
                },
            ],
            worn: vec![exa_formats::characters::ItemRef {
                id: 10 | 0x8000_0000,
                flags: 0,
            }],
            ..Default::default()
        };
        assert!((encumbrance_of(&p, &catalogue, Skills::default()) - 0.525).abs() < 1e-6);

        // The system leaves a new body's spawn value and writes a changed
        // body's into its physical stats.
        let mut app = App::new();
        app.insert_resource(catalogue).add_plugins(EquipPlugin);
        let mut spawned = exa_physics::body::Physical::default();
        spawned.stats.encumbrance = 0.25;
        let body = app
            .world_mut()
            .spawn((Possessions::default(), spawned))
            .id();
        app.update();
        let encumbrance = |app: &App| {
            app.world()
                .get::<exa_physics::body::Physical>(body)
                .unwrap()
                .stats
                .encumbrance
        };
        assert_eq!(encumbrance(&app), 0.25);
        {
            let mut entity = app.world_mut().entity_mut(body);
            let mut possessions = entity.get_mut::<Possessions>().unwrap();
            possessions.out[1] = Hand { id: 20, flags: 0 };
            possessions
                .worn
                .push(exa_formats::characters::ItemRef { id: 10, flags: 0 });
        }
        app.update();
        assert!((encumbrance(&app) - 0.525).abs() < 1e-6);
    }

    /// Putting on a worn item of special kind 2 grants the second wind,
    /// taking it off takes it away; a golem's core (an item without an
    /// apparel section) unblocks its red regeneration while it is worn.
    #[test]
    fn the_armour_pass_reruns_its_worn_rules_on_a_change() {
        let mut body = vec![0u8; 0x220];
        // The special kind byte of the item body.
        body[0x214] = exa_formats::objdb::Burden::SECOND_WIND;
        let mut file = Vec::new();
        let mut section = Vec::new();
        section.extend_from_slice(&0x1000u32.to_le_bytes());
        section.extend_from_slice(&((8 + body.len()) as u32).to_le_bytes());
        section.extend_from_slice(&body);
        file.extend_from_slice(&exa_formats::objdb::MAGIC.to_le_bytes());
        file.extend_from_slice(&32u32.to_le_bytes());
        for word in [10u32, 0, 0, section.len() as u32] {
            file.extend_from_slice(&word.to_le_bytes());
        }
        file.extend_from_slice(&[0; 16]);
        file.extend_from_slice(&section);
        let db = exa_formats::rdb::Database::parse(&file).unwrap();
        let catalogue = ItemCatalogue::of_database(&db);
        let mut app = App::new();
        app.insert_resource(catalogue).add_plugins(EquipPlugin);
        let mut fighter = exa_combat::Fighter::default();
        fighter.stats.golem_core = Some(500);
        fighter.stats.rules = exa_formats::character_stats::BodyRules(0x160);
        let e = app
            .world_mut()
            .spawn((Possessions::default(), fighter))
            .id();
        app.update();
        let state = |app: &App| {
            let f = app.world().get::<exa_combat::Fighter>(e).unwrap();
            (f.second_wind.granted, f.stats.rules.0)
        };
        // The spawn's values stand on the first frame.
        assert_eq!(state(&app), (false, 0x160));
        let wear = |app: &mut App, ids: &[u32]| {
            let mut entity = app.world_mut().entity_mut(e);
            let mut possessions = entity.get_mut::<Possessions>().unwrap();
            possessions.worn = ids
                .iter()
                .map(|&id| exa_formats::characters::ItemRef { id, flags: 0 })
                .collect();
        };
        wear(&mut app, &[10, 500]);
        app.update();
        assert_eq!(state(&app), (true, 0x60));
        wear(&mut app, &[]);
        app.update();
        assert_eq!(state(&app), (false, 0x160));
    }

    #[test]
    fn the_carried_torch_stays_in_its_hand() {
        let torch = HeldItem {
            flags: crate::torch::TORCH_FLAGS,
            light: true,
        };
        let swapped = Hands {
            main: Some(HeldItem::weapon(0x521)),
            shield: Some(ShieldStyle::Plain),
            ..Default::default()
        };
        // In the main hand: the record's weapon does not replace it.
        let before = Hands {
            main: Some(torch),
            ..Default::default()
        };
        let kept = keep_torch(swapped, &before);
        assert_eq!(kept.main, Some(torch));
        assert_eq!(kept.shield, Some(ShieldStyle::Plain));
        // In the off hand: it stays the off hand's light.
        let before = Hands {
            off_light: true,
            ..Default::default()
        };
        let kept = keep_torch(swapped, &before);
        assert!(kept.off_light && kept.shield.is_none());
        assert_eq!(kept.main.unwrap().flags, 0x521);
    }

    #[test]
    fn a_swap_moves_the_hands() {
        let mut app = App::new();
        app.init_resource::<ItemCatalogue>()
            .add_plugins(EquipPlugin);
        let body = app
            .world_mut()
            .spawn((
                Possessions {
                    out: [Hand { id: 6, flags: 0 }, Hand::default()],
                    ..Default::default()
                },
                Hands {
                    main: Some(HeldItem::weapon(0x521)),
                    ..Default::default()
                },
                PendingHands(exa_items::possession::HandPlan {
                    changed: [true, false],
                    take: [None, None],
                }),
            ))
            .id();
        app.update();
        // The catalogue is empty, so the record's item resolves to no
        // sections and the main hand is emptied; the plan is consumed.
        let hands = app.world().get::<Hands>(body).unwrap();
        assert!(hands.main.is_none());
        assert!(app.world().get::<PendingHands>(body).is_none());
    }

    /// A collapse drops the out set's items from the record and their held
    /// entries from the carried list; empty hands write nothing.
    #[test]
    fn a_collapse_forgets_the_dropped_items() {
        use exa_items::possession::Carried;
        let mut app = App::new();
        app.add_systems(Update, forget_collapsed_hands);
        let held = Carried {
            id: 5,
            flags: 0x100,
            ..Carried::default()
        };
        let armed = app
            .world_mut()
            .spawn(Possessions {
                out: [Hand { id: 5, flags: 0 }, Hand::default()],
                carried: vec![held],
                ..Possessions::default()
            })
            .id();
        let empty = app.world_mut().spawn(Possessions::default()).id();
        app.update();
        for e in [armed, empty] {
            app.world_mut()
                .entity_mut(e)
                .insert(exa_combat::collapse::Collapse::default());
        }
        app.update();
        let p = app.world().get::<Possessions>(armed).unwrap();
        assert_eq!(p.out[0].id, 0);
        assert!(p.carried.is_empty());
        assert_eq!(
            app.world().get::<Possessions>(empty).unwrap(),
            &Possessions::default()
        );
    }
}
