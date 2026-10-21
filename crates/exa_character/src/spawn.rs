//! What a record holds and where its spawn stands: the generator's equip
//! pass and the spawn's motion and height rules.
//!
//! The plans are plain data. The game uses the equip plan for the hands it
//! can draw; shields and the off-hand torch stay plans until the renderer
//! has a place for them, and the spawn heights are not wired.

use exa_formats::characters::{CharacterRecord, ItemRef};
use exa_formats::objdb::Weapon;

/// Where an item's model comes from: a factory recipe or an entry of the
/// object model library the generator keeps (not the `Characters` package,
/// which the apparel loader reads).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelSource {
    Factory(String),
    Library(String),
}

/// How an item is shown and held, as the item-model loader plans it.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemModelPlan {
    pub source: ModelSource,
    pub kind: EquipKind,
    /// The side of the generated skin textures, set only for an item with
    /// a body section: `0x400` when high-resolution skins are on and the
    /// pixel-format setting is 3 or more, else `0x200`.
    pub skin_side: Option<u32>,
}

/// The item-model loader's plan. The model is the item's model section
/// (a name starting `@` is a factory recipe, any other a library entry);
/// an item without one has no model. A weapon section makes it a weapon,
/// else a shield section a shield; an item with neither is not held. The
/// loader also writes the item's id through the model's nodes.
pub fn item_model_plan(
    item: &exa_formats::objdb::Item,
    pixel_format: i32,
    high_res_skins: bool,
) -> Option<ItemModelPlan> {
    if item.model.is_empty() {
        return None;
    }
    let source = match item.model.strip_prefix('@') {
        Some(factory) => ModelSource::Factory(factory.to_string()),
        None => ModelSource::Library(item.model.clone()),
    };
    let kind = if item.weapon.is_some() {
        EquipKind::Weapon
    } else if item.shield.is_some() {
        EquipKind::Shield
    } else {
        return None;
    };
    let skin_side = item
        .burden
        .is_some()
        .then_some(if high_res_skins && pixel_format >= 3 {
            0x400
        } else {
            0x200
        });
    Some(ItemModelPlan {
        source,
        kind,
        skin_side,
    })
}

/// Whether a material name is one the generator makes at run time: its
/// first character is `&`. Those are valid without a library entry.
pub fn is_runtime_material(name: &str) -> bool {
    name.starts_with('&')
}

/// What a hand holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Equipped {
    /// The model entry (or `@factory`).
    pub model: String,
    /// How the item is held.
    pub kind: EquipKind,
    /// The weapon section, for a weapon.
    pub weapon: Option<Weapon>,
    /// The record's flags for this hand.
    pub flags: u32,
}

/// The three ways the equip pass can take a hand's item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipKind {
    /// A weapon in the main or off hand.
    Weapon,
    /// A shield.
    Shield,
    /// The off hand's torch (an item with the torch section).
    Torch,
}

/// A hand's item as the equip pass resolves it (the level's own or the
/// game's database).
#[derive(Clone, Debug, PartialEq)]
pub struct HandEntry {
    pub model: String,
    pub weapon: Option<Weapon>,
    pub shield: bool,
    /// The item has the torch section.
    pub torch: bool,
}

/// The equip pass over a record's first two hands. `resolve` gets the hand
/// index and its item reference.
///
/// The off hand's item takes the torch path when it has the torch section;
/// otherwise a weapon section makes it a weapon, else a shield section a
/// shield, and an item with neither is not equipped. Both hands are always
/// visited. The record's flags for each hand are kept so the holder can
/// apply its bits.
pub fn equip_hands(
    record: &CharacterRecord,
    mut resolve: impl FnMut(usize, ItemRef) -> Option<HandEntry>,
) -> [Option<Equipped>; 2] {
    let mut out = [None, None];
    for (hand, slot) in out.iter_mut().enumerate() {
        let r = record.hands[hand];
        let Some(item) = r.is_some().then(|| resolve(hand, r)).flatten() else {
            continue;
        };
        let kind = if hand == 1 && item.torch {
            EquipKind::Torch
        } else if item.weapon.is_some() {
            EquipKind::Weapon
        } else if item.shield {
            EquipKind::Shield
        } else {
            continue;
        };
        *slot = Some(Equipped {
            model: item.model,
            kind,
            weapon: item.weapon,
            flags: r.flags,
        });
    }
    out
}

/// The motion a placed character starts in: the saved one when the
/// placement has it, an attack motion (5…12) reset to the Defence stance
/// 3, else 0x13, or 0x19 when the record's profession word has `0x1000`
/// (bit 4 of its race byte).
pub fn placed_motion(saved: Option<i32>, record: &CharacterRecord) -> i32 {
    match saved {
        Some(m) if (5..13).contains(&m) => 3,
        Some(m) => m,
        None if record.profession() & 0x1000 != 0 => 0x19,
        None => 0x13,
    }
}

/// The motion the player's spawn sets: 3 in mode 1, else 0x13.
pub fn player_motion(mode: i32) -> i32 {
    if mode == 1 { 3 } else { 0x13 }
}

/// The corpse's motion: the Defence stance 3.
pub const CORPSE_MOTION: i32 = 3;
/// The corpse's condition word: dead.
pub const CORPSE_CONDITION: u32 = 0xA00;

/// The root height a placed spawn without a saved pose places a body at:
/// the placement's height plus the standing pelvis, less 8 cm, or, for a
/// lying placement (its flag bit 1), the standing pelvis less 5 cm. A
/// saved pose places the body itself and takes its root height from it.
pub fn placed_root_height(placement_y: f32, standing: f32, lying: bool) -> f32 {
    if lying {
        standing - 5.0
    } else {
        (placement_y + standing) - 8.0
    }
}

/// The player's spawn height: the standing pelvis less 5 cm, or, with a
/// placement frame, the frame's height plus the pelvis less 6 cm.
pub fn player_root_height(standing: f32, frame_y: Option<f32>) -> f32 {
    match frame_y {
        Some(y) => (y + standing) - 6.0,
        None => standing - 5.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weapon(flags: u32) -> Weapon {
        Weapon {
            flags,
            seeds: [0; 2],
            quality: 0,
            damage: 0,
            dirt: 0,
            materials: [(0, 0, 0, 0); 4],
            selection: [0; 10],
            stats: exa_formats::weapon_stats::StatBytes::default(),
        }
    }

    fn entry(name: &str, weapon: Option<Weapon>, shield: bool) -> HandEntry {
        HandEntry {
            model: name.into(),
            weapon,
            shield,
            torch: false,
        }
    }

    fn item(id: u32, model: &str) -> exa_formats::objdb::Item {
        exa_formats::objdb::Item {
            id,
            name: "Sword".into(),
            model: model.into(),
            model_flags: 0,
            worn_model: String::new(),
            appearance: None,
            slot_layers: [0; exa_formats::objdb::SLOTS],
            slot_words: [0; exa_formats::objdb::SLOTS],
            occupancy: [0; exa_formats::objdb::OCCUPANCY_BONES],
            weapon: Some(weapon(0x11)),
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
            equip_lock: 0,
            torch: false,
        }
    }

    #[test]
    fn an_item_model_is_a_factory_or_a_library_entry() {
        let it = item(1, "@sword_a");
        let plan = item_model_plan(&it, 2, true).unwrap();
        assert_eq!(plan.source, ModelSource::Factory("sword_a".into()));
        assert_eq!(plan.kind, EquipKind::Weapon);
        // No body section: the skin side is left alone.
        assert_eq!(plan.skin_side, None);
        let mut it = item(2, "mace_a");
        it.burden = Some(exa_formats::objdb::Burden::default());
        assert_eq!(
            item_model_plan(&it, 5, true).unwrap().source,
            ModelSource::Library("mace_a".into())
        );
        assert_eq!(
            item_model_plan(&it, 5, true).unwrap().skin_side,
            Some(0x400)
        );
        assert_eq!(
            item_model_plan(&it, 5, false).unwrap().skin_side,
            Some(0x200)
        );
        assert_eq!(
            item_model_plan(&it, 2, true).unwrap().skin_side,
            Some(0x200)
        );
        // Neither weapon nor shield: no held model; nor without a model.
        it.weapon = None;
        assert!(item_model_plan(&it, 2, true).is_none());
        assert!(item_model_plan(&item(3, ""), 2, true).is_none());
    }

    #[test]
    fn run_time_materials_start_with_an_ampersand() {
        assert!(is_runtime_material("&doublet_ba1"));
        assert!(is_runtime_material("&short"));
        assert!(!is_runtime_material("harhps1"));
    }

    #[test]
    fn a_weapon_goes_to_the_main_hand_and_a_shield_to_the_off() {
        let mut record = CharacterRecord::default();
        record.hands[0] = ItemRef::new(1, 0);
        record.hands[1] = ItemRef::new(2, 0);
        let hands = equip_hands(&record, |_, r| {
            Some(match r.id {
                1 => entry("sword", Some(weapon(0x11)), false),
                _ => entry("shield", None, true),
            })
        });
        assert_eq!(hands[0].as_ref().unwrap().kind, EquipKind::Weapon);
        assert_eq!(hands[1].as_ref().unwrap().kind, EquipKind::Shield);
        // An item with both sections is a weapon.
        let both = equip_hands(&record, |_, _| {
            Some(entry("buckler", Some(weapon(0x11)), true))
        });
        assert_eq!(both[1].as_ref().unwrap().kind, EquipKind::Weapon);
    }

    #[test]
    fn the_off_hands_torch_takes_its_own_path() {
        let mut record = CharacterRecord::default();
        record.hands[0] = ItemRef::new(1, 0);
        record.hands[1] = ItemRef::new(2, 0x100);
        let hands = equip_hands(&record, |hand, _| {
            Some(if hand == 0 {
                HandEntry {
                    torch: true,
                    ..entry("torch", None, false)
                }
            } else {
                HandEntry {
                    torch: true,
                    ..entry("torch", Some(weapon(0x13)), false)
                }
            })
        });
        // The main hand's torch section is ignored; with no weapon or
        // shield it is not equipped.
        assert!(hands[0].is_none());
        assert_eq!(hands[1].as_ref().unwrap().kind, EquipKind::Torch);
        assert_eq!(hands[1].as_ref().unwrap().flags, 0x100);
        // An item with neither section is skipped, and the off hand is
        // still visited.
        record.hands[1] = ItemRef::new(3, 0);
        let hands = equip_hands(&record, |hand, _| {
            Some(if hand == 0 {
                entry("cup", None, false)
            } else {
                entry("shield", None, true)
            })
        });
        assert!(hands[0].is_none());
        assert_eq!(hands[1].as_ref().unwrap().kind, EquipKind::Shield);
    }

    #[test]
    fn an_empty_hand_leaves_no_entry() {
        let record = CharacterRecord::default();
        assert_eq!(equip_hands(&record, |_, _| None), [None, None]);
    }

    #[test]
    fn motions_are_the_spawns() {
        let mut record = CharacterRecord::default();
        assert_eq!(placed_motion(None, &record), 0x13);
        // Bit 0x1000 of the profession word is the race byte's bit 4.
        record.race = 0x10;
        assert_eq!(placed_motion(None, &record), 0x19);
        assert_eq!(placed_motion(Some(7), &record), 3);
        assert_eq!(placed_motion(Some(4), &record), 4);
        assert_eq!(placed_motion(Some(13), &record), 13);
        assert_eq!(placed_motion(Some(1), &record), 1);
        assert_eq!((player_motion(1), player_motion(0)), (3, 0x13));
        assert_eq!(CORPSE_MOTION, 3);
        assert_eq!(CORPSE_CONDITION, 0xA00);
    }

    #[test]
    fn the_spawn_heights_are_the_originals() {
        assert_eq!(placed_root_height(0.0, 101.5, false), 93.5);
        assert_eq!(placed_root_height(2.0, 101.5, false), 95.5);
        assert_eq!(placed_root_height(2.0, 101.5, true), 96.5);
        assert_eq!(player_root_height(101.5, None), 96.5);
        assert_eq!(player_root_height(101.5, Some(2.0)), 97.5);
    }
}
