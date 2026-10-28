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
use exa_formats::encumbrance::{Skills, encumbrance};
use exa_items::catalogue::{ItemCatalogue, PendingHands};
use exa_items::possession::{ItemKind, Possessions};

use crate::torch::CarriesTorch;

/// A character's encumbrance from what its record currently wears and
/// holds: the worn items' burdens, the off hand's shield, and the
/// character's armour and shield techniques. An item the catalogue does
/// not know weighs nothing, as a missing record does in the original's
/// armour pass.
pub fn encumbrance_of(p: &Possessions, catalogue: &ItemCatalogue, skills: Skills) -> f32 {
    let worn: Vec<_> = p
        .worn
        .iter()
        .filter_map(|worn| catalogue.burden(worn.id))
        .collect();
    let shield = [p.out[1], p.out[0]]
        .into_iter()
        .filter(|hand| hand.id != 0)
        .filter(|hand| {
            catalogue
                .kind(hand.id)
                .is_some_and(|kind| kind.shield && kind.weapon.is_none())
        })
        .find_map(|hand| catalogue.shield_record(hand.id));
    encumbrance(&worn, shield.as_ref(), skills)
}

/// Refreshes a body's encumbrance when its possessions change, as the
/// armour pass does after an equip, unequip or swap. The statistics are
/// the body's [`Physical::stats`](exa_physics::body::Physical); the body's
/// live record supplies the techniques.
pub fn refresh_encumbrance(
    catalogue: Option<Res<ItemCatalogue>>,
    mut bodies: Query<
        (
            &Possessions,
            Option<&exa_character::record::CharacterData>,
            &mut exa_physics::body::Physical,
        ),
        Changed<Possessions>,
    >,
) {
    let Some(catalogue) = catalogue.as_deref() else {
        return;
    };
    for (possessions, data, mut physical) in &mut bodies {
        let skills = data.map_or_else(Skills::default, |data| Skills::of_record(&data.record));
        let value = encumbrance_of(possessions, catalogue, skills);
        physical.stats.encumbrance = value;
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

/// Applies the hand plans of swapped weapon sets.
pub struct EquipPlugin;

impl Plugin for EquipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            apply_pending_hands
                .after(exa_items::catalogue::swap_weapon_sets)
                .before(refresh_encumbrance),
        )
        .add_systems(Update, refresh_encumbrance);
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
        file.extend_from_slice(&32u32.to_le_bytes());
        for (id, offset, len) in [
            (10u32, 0u32, first.len() as u32),
            (20u32, first.len() as u32, second.len() as u32),
        ] {
            file.extend_from_slice(&id.to_le_bytes());
            file.extend_from_slice(&0u32.to_le_bytes());
            file.extend_from_slice(&offset.to_le_bytes());
            file.extend_from_slice(&len.to_le_bytes());
        }
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

        // The system writes the value into the body's physical stats.
        let mut app = App::new();
        app.insert_resource(catalogue).add_plugins(EquipPlugin);
        let body = app
            .world_mut()
            .spawn((
                Possessions {
                    out: [Hand::default(), Hand { id: 20, flags: 0 }],
                    worn: vec![exa_formats::characters::ItemRef { id: 10, flags: 0 }],
                    ..Default::default()
                },
                exa_physics::body::Physical::default(),
            ))
            .id();
        app.update();
        let physical = app
            .world()
            .get::<exa_physics::body::Physical>(body)
            .unwrap();
        assert!((physical.stats.encumbrance - 0.525).abs() < 1e-6);
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
}
