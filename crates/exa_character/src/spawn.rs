//! What a record holds and where its spawn stands: the generator's equip
//! pass (`0x1000efd80`) and the spawn's motion, stance and tallies
//! (`0x1000f0170`, `0x1000f0d00`, `0x1000f1220`).
//!
//! The plans are plain data. The game uses them for the hands it can draw;
//! shields, off-hand lights and saved poses stay plans until the renderer
//! has a place for them.

use exa_formats::characters::{CharacterRecord, ItemRef};
use exa_formats::objdb::Weapon;

/// Where an item's model comes from (`0x1000e66d0`): a factory recipe or a
/// `Characters.rpk` entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelSource {
    Factory(String),
    Characters(String),
}

/// How an item is shown and held, as the item-model loader plans it.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemModelPlan {
    pub source: ModelSource,
    pub kind: EquipKind,
    /// The side of the generated skin textures the item's materials use
    /// (`0x200` below pixel format 3, else `0x400`).
    pub skin_side: u32,
    /// Whether the item is a light that lights its holder.
    pub light: bool,
}

/// The item-model loader's plan (`0x1000e66d0`): a name starting `@` is a
/// factory recipe, any other a `Characters` entry (the original looks it up
/// in that package's library). The item's weapon, shield and light sections
/// pick its class, and the current pixel format picks the texture side.
pub fn item_model_plan(
    item: &exa_formats::objdb::Item,
    pixel_format: u32,
) -> Option<ItemModelPlan> {
    if item.model.is_empty() && item.worn_model.is_empty() {
        return None;
    }
    let name = if item.model.is_empty() {
        item.worn_model.clone()
    } else {
        item.model.clone()
    };
    let source = match name.strip_prefix('@') {
        Some(factory) => ModelSource::Factory(factory.to_string()),
        None => ModelSource::Characters(name.clone()),
    };
    let kind = if item.shield.is_some() {
        EquipKind::Shield
    } else if item.weapon.is_some() {
        EquipKind::Weapon
    } else if item.name.eq_ignore_ascii_case("torch") {
        EquipKind::Torch
    } else {
        EquipKind::Light
    };
    Some(ItemModelPlan {
        source,
        kind,
        skin_side: crate::factory::skin_side(pixel_format),
        light: item.name.eq_ignore_ascii_case("torch"),
    })
}

/// Whether a material name is one the generator makes at run time (`&` and
/// ten characters, `0x1000e9fb0`): those are valid without a library entry.
pub fn is_runtime_material(name: &str) -> bool {
    name.starts_with('&') && name.chars().count() == 12
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
    /// The record's flags for this hand (flipped/light).
    pub flags: u32,
}

/// The three ways the equip pass can take a hand's item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipKind {
    /// A weapon in the main or off hand.
    Weapon,
    /// A shield in the off hand.
    Shield,
    /// A light (the torch's own path is [`EquipKind::Torch`]).
    Light,
    /// The off hand's torch path.
    Torch,
}

/// A hand's item as the equip pass resolves it (the level's own or the
/// game's database).
#[derive(Clone, Debug, PartialEq)]
pub struct HandEntry {
    pub model: String,
    pub weapon: Option<Weapon>,
    pub shield: bool,
    /// The item's light section, if it has one.
    pub light: bool,
}

/// The equip pass (`0x1000efd80`) over a record's first two hands.
///
/// The off hand's torch path (`flags & 0x100`) takes the item before the
/// weapon/shield tests and ends the pass. A weapon goes to the main hand
/// (or the off hand when it is a second weapon), a shield and a light to the
/// off hand. The record's flags for each hand are kept so the holder can
/// apply the flipped and burning bits.
pub fn equip_hands(
    record: &CharacterRecord,
    mut resolve: impl FnMut(ItemRef) -> Option<HandEntry>,
) -> [Option<Equipped>; 2] {
    let mut out = [None, None];
    for (hand, slot) in out.iter_mut().enumerate() {
        let r = record.hands[hand];
        let item = r.is_some().then(|| resolve(r)).flatten();
        let Some(item) = item else {
            if hand == 1 {
                break;
            }
            continue;
        };
        let kind = if hand == 1 && r.flags & 0x100 != 0 {
            EquipKind::Torch
        } else if item.shield {
            EquipKind::Shield
        } else if item.weapon.is_some() {
            EquipKind::Weapon
        } else {
            let _ = item.light;
            EquipKind::Light
        };
        *slot = Some(Equipped {
            model: item.model,
            kind,
            weapon: item.weapon,
            flags: r.flags,
        });
        if hand == 1 || kind == EquipKind::Torch {
            break;
        }
    }
    out
}

/// The stance the spawn sets (`0x1000f0170`): 3 for the player/practice
/// spawn, 0x13 for the other spawns.
pub fn spawn_stance(player: bool) -> i32 {
    if player { 3 } else { 0x13 }
}

/// The motion a placed character starts in (`0x1000f0d00`): the saved one
/// when the placement has it, an attack motion (5…12) reset to the Defence
/// stance 3, else 0x13, or 0x19 for a feral/golem record whose word has
/// `0x1000`.
pub fn placed_motion(saved: Option<i32>, record: &CharacterRecord) -> i32 {
    match saved {
        Some(m) if (5..13).contains(&m) => 3,
        Some(m) => m,
        None if record.profession_rest & 0x1000 != 0 => 0x19,
        None => 0x13,
    }
}

/// A spawn's motion, stance, root height, condition and saved pose.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnPlan {
    pub stance: i32,
    pub motion: i32,
    pub root_height: f32,
    pub condition: Option<u32>,
    pub pose: Vec<exa_formats::morph::Frame>,
}

/// The placed-character spawn plan (`0x1000f0d00`).
pub fn placed_spawn(
    record: &CharacterRecord,
    standing: f32,
    saved_motion: Option<i32>,
    saved_pose: Option<&[exa_formats::morph::Frame]>,
    saved_y: f32,
    lying: bool,
) -> SpawnPlan {
    SpawnPlan {
        stance: 0x13,
        motion: placed_motion(saved_motion, record),
        root_height: placed_root_height(standing, saved_pose.is_some(), saved_y, lying),
        condition: None,
        pose: saved_pose.map(<[_]>::to_vec).unwrap_or_default(),
    }
}

/// The player/creator spawn plan (`0x1000f0170`): stance 3 and the standing
/// height less 5 cm, or the saved pose's height.
pub fn player_spawn(
    standing: f32,
    saved_pose: Option<&[exa_formats::morph::Frame]>,
    saved_y: f32,
) -> SpawnPlan {
    SpawnPlan {
        stance: 3,
        motion: 0x13,
        root_height: player_root_height(standing, saved_pose.is_some(), saved_y),
        condition: None,
        pose: saved_pose.map(<[_]>::to_vec).unwrap_or_default(),
    }
}

/// The corpse spawn plan (`0x1000f1220`): the Defence stance, dead
/// condition and no motion of its own.
pub fn corpse_spawn(
    standing: f32,
    saved_pose: Option<&[exa_formats::morph::Frame]>,
    saved_y: f32,
) -> SpawnPlan {
    SpawnPlan {
        stance: CORPSE_MOTION,
        motion: CORPSE_MOTION,
        root_height: placed_root_height(standing, saved_pose.is_some(), saved_y, true),
        condition: Some(CORPSE_CONDITION),
        pose: saved_pose.map(<[_]>::to_vec).unwrap_or_default(),
    }
}

/// The corpse's motion (`0x1000f1220`): the Defence stance 3.
pub const CORPSE_MOTION: i32 = 3;
/// The corpse's condition word (`0x1000f1220`): dead.
pub const CORPSE_CONDITION: u32 = 0xA00;

/// The root height a spawn places a body at (`0x1000f0d00`): the standing
/// pelvis less 8 cm over the level's feet point, unless the saved condition
/// has bit 2 (lying), which places it 5 cm under.
pub fn placed_root_height(standing: f32, saved_pose: bool, saved_y: f32, lying: bool) -> f32 {
    if saved_pose {
        saved_y + standing - 6.0
    } else if lying {
        standing - 5.0
    } else {
        standing - 8.0
    }
}

/// The player's spawn height (`0x1000f0170`): the standing pelvis less 5 cm,
/// or the saved pose's height plus the pelvis less 6 cm.
pub fn player_root_height(standing: f32, saved_pose: bool, saved_y: f32) -> f32 {
    if saved_pose {
        saved_y + standing - 6.0
    } else {
        standing - 5.0
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
            light: false,
        }
    }

    fn item(id: u32, model: &str) -> exa_formats::objdb::Item {
        exa_formats::objdb::Item {
            id,
            name: "Torch".into(),
            model: model.into(),
            model_flags: 0,
            worn_model: String::new(),
            appearance: None,
            slot_layers: [0; exa_formats::objdb::SLOTS],
            slot_words: [0; exa_formats::objdb::SLOTS],
            occupancy: [0; exa_formats::objdb::OCCUPANCY_BONES],
            weapon: None,
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
            equip_lock: 0,
        }
    }

    #[test]
    fn an_item_model_is_a_factory_or_a_characters_entry() {
        let it = item(1, "@sword_a");
        let plan = item_model_plan(&it, 2).unwrap();
        assert_eq!(plan.source, ModelSource::Factory("sword_a".into()));
        assert_eq!(plan.skin_side, 0x200);
        let it = item(2, "shirt_a");
        assert_eq!(
            item_model_plan(&it, 5).unwrap().source,
            ModelSource::Characters("shirt_a".into())
        );
        assert_eq!(item_model_plan(&it, 5).unwrap().skin_side, 0x400);
        assert_eq!(item_model_plan(&it, 5).unwrap().kind, EquipKind::Torch);
        assert!(item_model_plan(&item(3, ""), 2).is_none());
    }

    #[test]
    fn only_run_time_materials_skip_the_library() {
        assert!(is_runtime_material("&doublet_ba1"));
        assert!(!is_runtime_material("&short"));
        assert!(!is_runtime_material("harhps1"));
    }

    #[test]
    fn a_weapon_goes_to_the_main_hand_and_a_shield_to_the_off() {
        let mut record = CharacterRecord::default();
        record.hands[0] = ItemRef::new(1, 0);
        record.hands[1] = ItemRef::new(2, 0);
        let hands = equip_hands(&record, |r| {
            Some(match r.id {
                1 => entry("sword", Some(weapon(0x11)), false),
                _ => entry("shield", None, true),
            })
        });
        assert_eq!(hands[0].as_ref().unwrap().kind, EquipKind::Weapon);
        assert_eq!(hands[1].as_ref().unwrap().kind, EquipKind::Shield);
    }

    #[test]
    fn the_off_hands_torch_path_ends_the_pass() {
        let mut record = CharacterRecord::default();
        record.hands[0] = ItemRef::new(1, 0);
        record.hands[1] = ItemRef::new(2, 0x100);
        let hands = equip_hands(&record, |r| {
            Some(if r.id == 1 {
                entry("sword", Some(weapon(0x11)), false)
            } else {
                entry("torch", Some(weapon(0x13)), false)
            })
        });
        assert_eq!(hands[1].as_ref().unwrap().kind, EquipKind::Torch);
        assert_eq!(hands[1].as_ref().unwrap().flags, 0x100);
    }

    #[test]
    fn an_empty_hand_leaves_no_entry() {
        let record = CharacterRecord::default();
        assert_eq!(equip_hands(&record, |_| None), [None, None]);
    }

    #[test]
    fn stances_and_motions_are_the_spawns() {
        assert_eq!(spawn_stance(true), 3);
        assert_eq!(spawn_stance(false), 0x13);
        let mut record = CharacterRecord::default();
        assert_eq!(placed_motion(None, &record), 0x13);
        record.profession_rest = 0x1000;
        assert_eq!(placed_motion(None, &record), 0x19);
        assert_eq!(placed_motion(Some(7), &record), 3);
        assert_eq!(placed_motion(Some(4), &record), 4);
        assert_eq!(placed_motion(Some(13), &record), 13);
        assert_eq!(placed_motion(Some(1), &record), 1);
        assert_eq!(CORPSE_MOTION, 3);
        assert_eq!(CORPSE_CONDITION, 0xA00);
    }

    #[test]
    fn the_three_spawn_plans_pin_their_fields() {
        let record = CharacterRecord::default();
        let p = placed_spawn(&record, 101.5, Some(7), None, 0.0, false);
        assert_eq!((p.stance, p.motion), (0x13, 3));
        assert_eq!(p.root_height, 93.5);
        let p = player_spawn(101.5, None, 0.0);
        assert_eq!((p.stance, p.motion, p.root_height), (3, 0x13, 96.5));
        let c = corpse_spawn(101.5, None, 0.0);
        assert_eq!(
            (c.condition, c.motion, c.root_height),
            (Some(0xA00), 3, 96.5)
        );
    }

    #[test]
    fn the_spawn_heights_are_the_originals() {
        assert_eq!(placed_root_height(101.5, false, 0.0, false), 93.5);
        assert_eq!(placed_root_height(101.5, false, 0.0, true), 96.5);
        assert_eq!(placed_root_height(101.5, true, 2.0, false), 97.5);
        assert_eq!(player_root_height(101.5, false, 0.0), 96.5);
        assert_eq!(player_root_height(101.5, true, 2.0), 97.5);
    }
}
