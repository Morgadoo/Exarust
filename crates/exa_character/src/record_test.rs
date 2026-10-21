//! Tests of the live record's rules.

use super::*;
use exa_formats::objdb::{Burden, OCCUPANCY_BONES};
use exa_formats::rdb::Database;

/// An item of `id` occupying the pelvis with `occupancy`, optionally
/// covering the groin.
fn item(id: u32, occupancy: u64) -> Item {
    let mut table = [0; OCCUPANCY_BONES];
    table[0] = occupancy;
    Item {
        id,
        name: format!("item {id}"),
        model: String::new(),
        model_flags: 0,
        worn_model: format!("worn{id}"),
        appearance: None,
        slot_layers: [0; exa_formats::objdb::SLOTS],
        occupancy: table,
        weapon: None,
        armour: None,
        shield: None,
        door: None,
        operative: None,
        burden: Some(Burden::default()),
    }
}

fn refs(ids: &[u32]) -> Vec<ItemRef> {
    ids.iter().map(|&id| ItemRef::new(id, 0)).collect()
}

#[test]
fn a_live_record_starts_from_the_creator_defaults() {
    let data = CharacterData::created();
    assert_eq!(data.refs, 1);
    assert_eq!(data.record, CharacterRecord::reset());
    assert_eq!(data.record.worn, refs(&[UNDERWEAR_ITEM]));
    let blank = CharacterData::new();
    assert!(blank.record.worn.is_empty());
    assert_eq!(blank.record.pools, exa_formats::characters::Pools::FULL);
}

#[test]
fn the_underwear_goes_on_a_bare_human_only() {
    let trousers = item(35, 0x03);
    let shirt = item(144, 0x03_0000);
    let braies = item(UNDERWEAR_ITEM, 0x03);
    let find = |id: u32| {
        [&trousers, &shirt, &braies]
            .into_iter()
            .find(|i| i.id == id)
    };

    let mut bare = CharacterData::new();
    assert!(bare.ensure_underwear(find));
    assert_eq!(bare.record.worn, refs(&[UNDERWEAR_ITEM]));
    // Idempotent: the braies cover the groin.
    assert!(!bare.ensure_underwear(find));

    let mut dressed = CharacterData::new();
    dressed.record.worn = refs(&[35]);
    assert!(!dressed.ensure_underwear(find));
    assert_eq!(dressed.record.worn, refs(&[35]));

    // A shirt leaves the groin bare.
    let mut shirt_only = CharacterData::new();
    shirt_only.record.worn = refs(&[144]);
    assert!(shirt_only.ensure_underwear(find));

    // Only humans.
    let mut ghoul = CharacterData::new();
    ghoul.record.race = 6;
    assert!(!ghoul.ensure_underwear(find));
    assert!(ghoul.record.worn.is_empty());

    // A worn item no item record names does not cover anything.
    let mut unknown = CharacterData::new();
    unknown.record.worn = refs(&[999]);
    assert!(unknown.ensure_underwear(find));
}

#[test]
fn overlapping_items_come_off_on_an_equip() {
    let trousers = item(35, 0x03);
    let shirt = item(144, 0x03_0000);
    let braies = item(UNDERWEAR_ITEM, 0x03);
    let hose = item(700, 0x03);
    let mail_legs = item(701, 0x0300);
    let items = [&trousers, &shirt, &braies, &hose, &mail_legs];
    let find = |id: u32| items.into_iter().find(|i| i.id == id);

    let mut data = CharacterData::new();
    data.record.worn = refs(&[8, 144, 700, 701]);
    // Trousers overlap the braies and the hose on the pelvis's innermost
    // layer, not the shirt (torso) nor the mail legs (layer 1).
    assert_eq!(data.equip_conflicts(&trousers, find), refs(&[8, 700]));
    assert_eq!(data.record.worn, refs(&[144, 701]));
    // Nothing left to take off.
    assert!(data.equip_conflicts(&trousers, find).is_empty());
    // A worn item whose record cannot be read stays on.
    data.record.worn = refs(&[999]);
    assert!(data.equip_conflicts(&trousers, find).is_empty());
    assert_eq!(data.record.worn, refs(&[999]));
}

#[test]
fn the_equipped_mark_follows_the_hands() {
    let mut data = CharacterData::new();
    data.record.inventory = vec![
        Carried {
            item: 7,
            flags: 0,
            ..Carried::default()
        },
        Carried {
            item: 7,
            flags: EQUIPPED,
            ..Carried::default()
        },
    ];
    data.unmark_equipped(7, 1);
    assert_eq!(data.record.inventory[1].flags, 0);
    data.mark_equipped(7, 5);
    assert!(
        data.record
            .inventory
            .iter()
            .all(|e| e.flags & EQUIPPED != 0)
    );
    // The last flagged copy goes first.
    let at = data.take_equipped(7);
    assert_eq!(at, Some(1));
    assert_eq!(data.record.inventory.len(), 1);
    assert_eq!(data.take_equipped(7), Some(0));
    assert_eq!(data.take_equipped(7), None);

    // Hands put their items into the inventory, marked when in the first
    // pair; the same item in both hands is listed twice.
    let mut held = CharacterData::new();
    held.record.hands[0] = ItemRef::new(11, 1);
    held.record.hands[1] = ItemRef::new(11, 1);
    held.record.hands[2] = ItemRef::new(12, 0);
    held.stow_hands();
    assert_eq!(
        held.record
            .inventory
            .iter()
            .map(|e| (e.item, e.flags))
            .collect::<Vec<_>>(),
        [(11, EQUIPPED), (11, EQUIPPED), (12, 0)]
    );
    // A second call adds nothing.
    held.stow_hands();
    assert_eq!(held.record.inventory.len(), 3);
    // Unstowing clears the first pair's marks and slots only.
    held.unstow_hands();
    assert_eq!(held.record.hands[0].id, 0);
    assert_eq!(held.record.hands[1].id, 0);
    assert_eq!(held.record.hands[2].id, 12);
    assert_eq!(held.record.hands[0].flags, 1);
    assert_eq!(
        held.record
            .inventory
            .iter()
            .map(|e| (e.item, e.flags))
            .collect::<Vec<_>>(),
        [(11, 0), (11, 0), (12, 0)]
    );
}

#[test]
fn a_hand_item_held_more_often_than_owned_loses_its_extra_reference() {
    let mut data = CharacterData::new();
    data.record.hands[0] = ItemRef::new(7, 0);
    data.record.hands[1] = ItemRef::new(7, 0);
    data.record.inventory.push(Carried {
        item: 7,
        flags: EQUIPPED,
        ..Carried::default()
    });
    // One copy owned: the second hand goes, the flags stay.
    data.trim_hands(7, 1);
    assert_eq!(data.record.hands[0].id, 7);
    assert_eq!(data.record.hands[1].id, 0);
    assert_eq!(data.record.hands[1].flags, 0);

    // The surplus pass adds the inventory copies to what may stay.
    let mut owned = CharacterData::new();
    owned.record.hands[0] = ItemRef::new(7, 0);
    owned.record.hands[1] = ItemRef::new(7, 0);
    owned.record.hands[2] = ItemRef::new(7, 0);
    owned.record.loadouts.push(Loadout {
        word: 0,
        hands: [ItemRef::new(7, 0); 4],
        worn: refs(&[7]),
    });
    // No copy owned: nothing may be held.
    owned.drop_surplus_refs(7, 0);
    // The first pair keeps one reference and the second pair loses its
    // only one.
    assert_eq!(owned.record.hands[0].id, 7);
    assert_eq!(owned.record.hands[1].id, 0);
    assert_eq!(owned.record.hands[2].id, 0);
    assert!(owned.record.loadouts[0].worn.is_empty());
}

#[test]
fn removing_an_item_takes_one_copy_and_its_spare_references() {
    let entry = |item, flags| Carried {
        item,
        flags,
        ..Carried::default()
    };
    let mut data = CharacterData::new();
    data.record.inventory = vec![entry(7, 0), entry(7, EQUIPPED), entry(8, 0)];
    data.record.hands[0] = ItemRef::new(7, 0);
    data.record.hands[1] = ItemRef::new(7, 0);
    // The equipped copy is the one taken.
    assert!(data.remove_item(7));
    assert_eq!(
        data.record
            .inventory
            .iter()
            .map(|e| (e.item, e.flags))
            .collect::<Vec<_>>(),
        [(7, 0), (8, 0)]
    );
    // One copy is left, so one hand reference stays.
    assert_eq!(data.record.hands[0].id, 7);
    assert_eq!(data.record.hands[1].id, 0);

    // Not in the inventory: a worn copy comes off.
    let mut worn = CharacterData::new();
    worn.record.worn = refs(&[8, 9]);
    assert!(worn.remove_item(8));
    assert_eq!(worn.record.worn, refs(&[9]));
    assert!(!worn.remove_item(8));

    // Everywhere: nothing of the id is left.
    let mut all = CharacterData::new();
    all.record.inventory = vec![entry(7, 0), entry(7, 0)];
    all.record.worn = refs(&[7, 9]);
    all.record.hands[0] = ItemRef::new(7, 3);
    all.record.hands[2] = ItemRef::new(7, 0);
    all.staged = Some(vec![entry(7, 0), entry(9, 0)]);
    all.record.loadouts.push(Loadout {
        word: 0,
        hands: [
            ItemRef::new(7, 0),
            ItemRef::NONE,
            ItemRef::NONE,
            ItemRef::NONE,
        ],
        worn: refs(&[7]),
    });
    all.remove_everywhere(7);
    assert_eq!(all.record.inventory.len(), 0);
    assert_eq!(all.record.worn, refs(&[9]));
    assert!(all.record.hands.iter().all(|h| h.id != 7));
    assert_eq!(all.record.hands[0], ItemRef::NONE);
    assert_eq!(all.staged.as_ref().unwrap().len(), 1);
    assert!(all.record.loadouts[0].worn.is_empty());
    assert!(all.record.loadouts[0].hands.iter().all(|h| h.id != 7));

    // Clearing empties every item list, flags and all.
    all.record.hands[1] = ItemRef::new(9, 5);
    all.clear_items();
    // Only the ids go; the flags beside them stay.
    assert!(all.record.hands.iter().all(|h| h.id == 0));
    assert_eq!(all.record.hands[1].flags, 5);
    assert!(all.record.worn.is_empty() && all.record.inventory.is_empty());
}

#[test]
fn a_stripped_record_keeps_its_flags_and_its_inventory() {
    let mut data = CharacterData::new();
    data.record.hands = [
        ItemRef::new(1, 7),
        ItemRef::new(2, 0),
        ItemRef::new(3, 0),
        ItemRef::new(4, 0),
    ];
    data.record.worn = refs(&[8]);
    data.record.inventory.push(Carried::default());
    data.strip(true);
    assert!(data.record.hands.iter().all(|h| h.id == 0));
    assert_eq!(data.record.hands[0].flags, 7);
    assert!(data.record.worn.is_empty());
    assert_eq!(data.record.inventory.len(), 1);
    // Without the hands, they stay.
    let mut only_worn = CharacterData::new();
    only_worn.record.hands[0] = ItemRef::new(1, 0);
    only_worn.record.worn = refs(&[8]);
    only_worn.strip(false);
    assert_eq!(only_worn.record.hands[0].id, 1);
    assert!(only_worn.record.worn.is_empty());
}

#[test]
fn only_database_references_are_released() {
    let mut data = CharacterData::new();
    data.record.hands[0] = ItemRef::new(0x8000_0001, 0);
    data.record.hands[1] = ItemRef::new(5, 0);
    data.record.worn = refs(&[0x4000_0002, 6]);
    data.record.inventory = vec![
        Carried {
            item: 0x8000_0003,
            flags: 0,
            ..Carried::default()
        },
        Carried {
            item: 0x4000_0004,
            flags: 0,
            ..Carried::default()
        },
        Carried {
            item: 7,
            flags: 0,
            ..Carried::default()
        },
    ];
    assert_eq!(
        data.shared_item_refs()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [0x8000_0001, 0x4000_0002, 0x8000_0003, 0x4000_0004]
    );
}

#[test]
fn items_remap_as_a_record_moves_between_databases() {
    let mut data = CharacterData::new();
    data.record.hands = [
        ItemRef::new(0x8000_0001, 0),
        ItemRef::new(0x8000_0002, 0),
        ItemRef::new(0x4000_0003, 0),
        ItemRef::new(9, 0),
    ];
    data.record.worn = refs(&[0x8000_0004]);
    data.record.inventory = vec![
        Carried {
            item: 0x8000_0001,
            flags: 0,
            ..Carried::default()
        },
        Carried {
            item: 0x8000_0004,
            flags: 0,
            ..Carried::default()
        },
    ];
    data.record.loadouts.push(Loadout {
        word: 0,
        hands: [ItemRef::new(0x8000_0009, 0); 4],
        worn: refs(&[0x8000_0008]),
    });
    let mut next = 0x4000_0100;
    data.remap_items(|id| {
        next += 1;
        let _ = id;
        next
    });
    assert_eq!(
        data.record.hands.map(|h| h.id),
        [0x4000_0101, 0x4000_0102, 0x4000_0103, 9]
    );
    assert_eq!(data.record.worn[0].id, 0x4000_0104);
    // The loadouts come before the inventory; each loadout hand is
    // remapped on its own, even the four copies of one item.
    assert_eq!(data.record.loadouts[0].hands[0].id, 0x4000_0105);
    assert_eq!(data.record.loadouts[0].hands[3].id, 0x4000_0108);
    assert_eq!(data.record.loadouts[0].worn[0].id, 0x4000_0109);
    assert_eq!(data.record.inventory[0].item, 0x4000_010A);
    assert_eq!(data.record.inventory[1].item, 0x4000_010B);

    // The transfer variant keeps one copy of a hand item.
    let mut moved = CharacterData::new();
    moved.record.hands[0] = ItemRef::new(0x8000_0001, 0);
    moved.record.hands[1] = ItemRef::new(0x8000_0002, 0);
    moved.record.inventory = vec![
        Carried {
            item: 0x8000_0001,
            flags: 0,
            ..Carried::default()
        },
        Carried {
            item: 0x8000_0002,
            flags: 0,
            ..Carried::default()
        },
        Carried {
            item: 0x8000_0003,
            flags: 0,
            ..Carried::default()
        },
    ];
    let mut next = 0x4000_0200;
    moved.remap_hands_and_inventory(|_| {
        next += 1;
        next
    });
    assert_eq!(moved.record.hands[0].id, 0x4000_0201);
    assert_eq!(moved.record.hands[1].id, 0x4000_0202);
    assert_eq!(moved.record.inventory[0].item, 0x4000_0201);
    assert_eq!(moved.record.inventory[1].item, 0x4000_0202);
    // The first two inventory entries take their hand's new id; only the
    // third is copied, so it takes the third new id.
    assert_eq!(moved.record.inventory[2].item, 0x4000_0203);
}

#[test]
fn skill_levels_are_points_over_five_thousand() {
    let mut data = CharacterData::new();
    data.record.skills[0] = Skill {
        points: 15000,
        techniques: [1, 2, 3, 0, 0, 0],
    };
    data.record.skills[2] = Skill {
        points: 5000,
        techniques: [5, 0, 0, 0, 0, 0],
    };
    assert_eq!(data.total_levels(), 4);
    // A technique at the slot the points fund next (here the fourth level of
    // Close Combat) would make the record inconsistent.
    assert!(data.can_spend_level());
    assert_eq!(data.surplus_chosen_level(), None);

    // A technique sitting in an unfunded slot: no room.
    let mut inconsistent = CharacterData::new();
    inconsistent.record.skills[0] = Skill {
        points: 5000,
        techniques: [1, 2, 0, 0, 0, 0],
    };
    assert!(!inconsistent.can_spend_level());
    assert_eq!(inconsistent.surplus_chosen_level(), Some((1, 2)));

    // The total cap of 15 levels.
    let mut full = CharacterData::new();
    for (slot, skill) in full.record.skills.iter_mut().enumerate() {
        *skill = Skill {
            points: if slot < 3 { 25000 } else { 0 },
            techniques: if slot < 3 { [1, 2, 3, 4, 5, 0] } else { [0; 6] },
        };
    }
    assert_eq!(full.total_levels(), 15);
    assert!(!full.can_spend_level());
    // Four levels and four techniques: consistent again, below the cap.
    full.record.skills[0] = Skill {
        points: 20000,
        techniques: [1, 2, 3, 4, 0, 0],
    };
    assert_eq!(full.total_levels(), 14);
    assert!(full.can_spend_level());
}

#[test]
fn unlearning_a_level_rebases_the_skill() {
    let mut data = CharacterData::new();
    data.record.skills[0] = Skill {
        points: 20000,
        techniques: [1, 2, 3, 4, 0, 0],
    };
    // The second level's technique is removed; later ones move down.
    assert_eq!(data.unlearn_level(1, 2), Some(2));
    assert_eq!(data.record.skills[0].techniques, [1, 3, 4, 0, 0, 0]);
    assert_eq!(data.record.skills[0].points, 15000);
    // A level past the chosen ones only clears the slot and re-bases.
    assert_eq!(data.unlearn_level(1, 5), None);
    assert_eq!(data.record.skills[0].points, 10000);
    // Slots are 1-based; 0 and past the six do nothing.
    assert_eq!(data.unlearn_level(0, 1), None);
    assert_eq!(data.unlearn_level(7, 1), None);

    // Unlearning a technique id removes it from every skill.
    let mut both = CharacterData::new();
    both.record.skills[0] = Skill {
        points: 10000,
        techniques: [4, 1, 0, 0, 0, 0],
    };
    both.record.skills[2] = Skill {
        points: 5000,
        techniques: [4, 0, 0, 0, 0, 0],
    };
    both.unlearn_technique(4);
    assert_eq!(both.record.skills[0].techniques, [1, 0, 0, 0, 0, 0]);
    assert_eq!(both.record.skills[0].points, 5000);
    assert_eq!(both.record.skills[2].techniques, [0; 6]);
    assert_eq!(both.record.skills[2].points, 0);
}

#[test]
fn a_loadout_applied_to_a_bare_human_puts_the_underwear_on() {
    let braies = item(UNDERWEAR_ITEM, 0x03);
    let shirt = item(144, 0x03_0000);
    let find = |id: u32| [&braies, &shirt].into_iter().find(|i| i.id == id);
    let loadout = Loadout {
        word: 1,
        hands: [
            ItemRef::new(17, 0),
            ItemRef::NONE,
            ItemRef::NONE,
            ItemRef::NONE,
        ],
        worn: refs(&[144]),
    };
    let mut data = CharacterData::new();
    data.apply_loadout(&loadout, find);
    assert_eq!(data.record.hands[0].id, 17);
    assert_eq!(data.record.worn, refs(&[144, UNDERWEAR_ITEM]));
    // The record reads back as a loadout.
    assert_eq!(data.loadout().hands, data.record.hands);
    assert_eq!(data.loadout().worn, data.record.worn);
}

#[test]
fn a_record_keeps_its_reference_count_and_its_scope() {
    let mut data = CharacterData::of_record(CharacterRecord {
        id: 5,
        ..CharacterRecord::new()
    });
    data.retain();
    assert_eq!(data.release(), None);
    assert_eq!(data.release(), None, "a record from no database");
    assert_eq!(data.refs, 0);

    let mut world = CharacterData::of_record(CharacterRecord {
        id: 0x4000_0007,
        ..CharacterRecord::new()
    });
    assert_eq!(world.release(), Some(RecordScope::World(7)));
    let mut local = CharacterData::of_record(CharacterRecord {
        id: 0x8000_0008,
        ..CharacterRecord::new()
    });
    assert_eq!(local.release(), Some(RecordScope::Local(8)));
}

#[test]
fn worn_items_append_and_come_off_one_copy() {
    let mut data = CharacterData::new();
    data.add_worn(ItemRef::new(8, 0));
    data.add_worn(ItemRef::new(144, 1));
    data.add_worn(ItemRef::new(8, 0));
    // The first copy only.
    assert!(data.remove_worn(8));
    assert_eq!(data.record.worn, [ItemRef::new(144, 1), ItemRef::new(8, 0)]);
    assert!(data.remove_worn(8));
    assert!(!data.remove_worn(8));
    // The picked-up underwear is never carried; anything else is.
    data.pick_up(ItemRef::new(UNDERWEAR_ITEM, 0));
    assert!(data.record.inventory.is_empty());
    data.pick_up(ItemRef::new(35, 2));
    assert_eq!(
        data.record.inventory,
        [Carried {
            item: 35,
            flags: 2,
            ..Carried::default()
        }]
    );
    // A unique pickup adds one copy only.
    assert!(!data.add_carried_unique(35, 2));
    assert!(data.add_carried_unique(36, 0));
    assert_eq!(data.record.inventory.len(), 2);
}

#[test]
fn the_save_writes_health_as_shares_and_the_load_reads_the_last_entry() {
    let mut data = CharacterData::new();
    data.save_health(Health {
        yellow: 0.5,
        red: 0.25,
        max: 1.0,
        focus: 0.75,
        heal_pool: 0.125,
        potion: Some(1800),
        body_value: 2.5,
    });
    assert_eq!(data.record.pools.yellow, 32768);
    assert_eq!(data.record.pools.red, 16384);
    assert_eq!(data.record.pools.focus, 49151);
    assert_eq!(data.record.seed, 0xFFFF);
    assert_eq!(
        data.record.saved,
        [
            exa_formats::characters::SavedState {
                tag: exa_formats::characters::SAVED_HEAL_POOL,
                value: 0.125f32.to_bits()
            },
            exa_formats::characters::SavedState {
                tag: exa_formats::characters::SAVED_POTION,
                value: 1800
            },
        ]
    );
    // The load: shares times the maximum, the focus share on its own; both
    // saved entries report the last entry's value, as the original's loop
    // does.
    let health = data.restore_health(2.0);
    assert!((health.yellow - 1.0).abs() < 1e-4);
    assert!((health.red - 0.5).abs() < 1e-4);
    assert!((health.focus - 0.749_98).abs() < 1e-4);
    assert_eq!(health.potion, Some(1800));
    assert_eq!(health.heal_pool, f32::from_bits(1800));
    assert_eq!(health.body_value, 2.5);

    // Red that is not positive saves as 0; an empty heal pool writes no
    // entry.
    let mut dead = CharacterData::new();
    dead.save_health(Health {
        yellow: 1.0,
        red: -1.0,
        max: 1.0,
        focus: 0.0,
        ..Health::default()
    });
    assert_eq!(dead.record.pools.red, 0);
    assert!(dead.record.saved.is_empty());
    assert_eq!(dead.restore_health(1.0).heal_pool, 0.0);
}

/// The shipped object database's apparel, through the two rules
/// (skipped without the game): the underwear goes on a bare human and the
/// items that cover the groin keep it off; every apparel item conflicts
/// with itself (its occupancy overlaps) and the trousers displace the
/// braies but not the shirt.
#[test]
fn shipped_apparel_follows_the_underwear_and_conflict_rules() {
    let Some(dir) = exa_assets::locate_install(None) else {
        return;
    };
    let game = exa_assets::GameData::open(&dir).unwrap();
    let items = game
        .pack(exa_assets::PackKind::Resource)
        .and_then(|p| p.get("objdb.rdb"))
        .and_then(|bytes| Database::parse(bytes).ok())
        .map(|db| exa_formats::objdb::items(&db))
        .unwrap_or_default();
    assert!(items.len() > 1400);
    let find = |id: u32| items.iter().find(|item| item.id == id);

    // Every apparel item overlaps itself on some bone.
    let apparel: Vec<_> = items
        .iter()
        .filter(|item| !item.worn_model.is_empty() && item.occupancy.iter().any(|o| *o != 0))
        .collect();
    assert!(apparel.len() > 300);
    for item in &apparel {
        assert!(overlaps(item, item), "{} ({})", item.name, item.id);
    }

    // A human in the braies and a shirt: the braies stay; the trousers take
    // them off (and not the shirt).
    let braies = find(UNDERWEAR_ITEM).expect("the underwear");
    assert!(braies.covers_groin());
    let shirt = items
        .iter()
        .find(|item| item.name == "Shirt_B")
        .or_else(|| {
            items
                .iter()
                .find(|item| item.worn_model.eq_ignore_ascii_case("shirt_b"))
        })
        .expect("a shirt");
    let trousers = items
        .iter()
        .find(|item| item.worn_model.eq_ignore_ascii_case("trouser_a"))
        .or_else(|| {
            items
                .iter()
                .find(|item| item.worn_model.eq_ignore_ascii_case("trousers_c"))
        })
        .expect("trousers");
    assert!(!shirt.covers_groin());
    assert!(trousers.covers_groin());

    let mut data = CharacterData::new();
    data.record.worn = refs(&[UNDERWEAR_ITEM, shirt.id]);
    assert!(!data.ensure_underwear(find));
    assert_eq!(data.record.worn.len(), 2);
    let removed = data.equip_conflicts(trousers, find);
    assert_eq!(removed, refs(&[UNDERWEAR_ITEM]));
    assert_eq!(data.record.worn, refs(&[shirt.id]));
    assert_eq!(data.record.worn[0].id, shirt.id);
}
