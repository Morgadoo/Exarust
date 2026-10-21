//! Tests of the live record's rules.

use super::*;
use exa_formats::objdb::{Burden, OCCUPANCY_BONES};

/// The Armour skill's slot.
const ARMOUR_SLOT: usize = exa_formats::characters::ARMOUR;
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
        slot_words: [0; exa_formats::objdb::SLOTS],
        occupancy: table,
        weapon: None,
        armour: None,
        shield: None,
        door: None,
        operative: None,
        burden: Some(Burden::default()),
        equip_lock: 0,
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
    assert_eq!(blank.record.pools, exa_formats::characters::Pools::NEW);
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
    assert_eq!(data.equip_conflicts(&trousers, find), Some(refs(&[8, 700])));
    assert_eq!(data.record.worn, refs(&[144, 701]));
    // Nothing left to take off.
    assert_eq!(data.equip_conflicts(&trousers, find), Some(Vec::new()));
    // A worn item whose record cannot be read stays on.
    data.record.worn = refs(&[999]);
    assert_eq!(data.equip_conflicts(&trousers, find), Some(Vec::new()));
    assert_eq!(data.record.worn, refs(&[999]));
}

/// An overlapping worn item whose lock byte is over 50 refuses the equip:
/// nothing comes off, not even the other overlapping items. At 50, or on
/// an item that does not overlap, the lock does nothing.
#[test]
fn a_locked_overlapping_item_refuses_the_equip() {
    let trousers = item(35, 0x03);
    let braies = item(UNDERWEAR_ITEM, 0x03);
    let mut locked = item(700, 0x03);
    locked.equip_lock = 51;
    let mut shirt = item(144, 0x03_0000);
    shirt.equip_lock = 200;
    let items = [&trousers, &braies, &locked, &shirt];
    let find = |id: u32| items.into_iter().find(|i| i.id == id);
    let mut data = CharacterData::new();
    data.record.worn = refs(&[8, 700, 144]);
    assert_eq!(data.equip_conflicts(&trousers, find), None);
    assert_eq!(data.record.worn, refs(&[8, 700, 144]));

    let mut at_limit = item(700, 0x03);
    at_limit.equip_lock = 50;
    let items = [&trousers, &braies, &at_limit, &shirt];
    let find = |id: u32| items.into_iter().find(|i| i.id == id);
    assert_eq!(data.equip_conflicts(&trousers, find), Some(refs(&[8, 700])));
    assert_eq!(data.record.worn, refs(&[144]));
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
    // The copy that is not equipped is the one taken.
    assert!(data.remove_item(7));
    assert_eq!(
        data.record
            .inventory
            .iter()
            .map(|e| (e.item, e.flags))
            .collect::<Vec<_>>(),
        [(7, EQUIPPED), (8, 0)]
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

    // Among unequipped (or among equipped) copies the last one goes.
    let mut last = CharacterData::new();
    last.record.inventory = vec![
        entry(7, EQUIPPED),
        entry(7, EQUIPPED | 1),
        entry(7, 2),
        entry(7, 3),
    ];
    assert!(last.remove_item(7));
    let flags = |d: &CharacterData| {
        d.record
            .inventory
            .iter()
            .map(|e| e.flags)
            .collect::<Vec<_>>()
    };
    assert_eq!(flags(&last), [EQUIPPED, EQUIPPED | 1, 2]);
    last.record.inventory.truncate(2);
    assert!(last.remove_item(7));
    assert_eq!(flags(&last), [EQUIPPED]);

    // The worn loop moves on after a removal, so a copy right after a
    // removed one survives; it also reads the list's old tail, so a copy at
    // the end can take the last kept item with it.
    let mut pair = CharacterData::new();
    pair.record.worn = refs(&[8, 8, 9]);
    assert!(pair.remove_item(8));
    assert_eq!(pair.record.worn, refs(&[8, 9]));
    let mut around = CharacterData::new();
    around.record.worn = refs(&[8, 9, 8]);
    assert!(around.remove_item(8));
    assert!(around.record.worn.is_empty());

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
    assert!(data.strip(true).is_empty());
    assert!(data.record.hands.iter().all(|h| h.id == 0));
    assert_eq!(data.record.hands[0].flags, 7);
    assert!(data.record.worn.is_empty());
    assert_eq!(data.record.inventory.len(), 1);
    // Without the hands, they stay.
    let mut only_worn = CharacterData::new();
    only_worn.record.hands[0] = ItemRef::new(1, 0);
    only_worn.record.worn = refs(&[8]);
    only_worn.record.worn.push(ItemRef::new(0x8000_0005, 2));
    assert_eq!(only_worn.strip(false), [ItemRef::new(0x8000_0005, 2)]);
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
    // Every id goes through the remap (the game's leaves the ids it does
    // not know as they are), in the game's order.
    let mut seen = Vec::new();
    let mut next = 0x4000_0100;
    data.remap_items(|id| {
        seen.push(id);
        next += 1;
        next
    });
    assert_eq!(seen.len(), 4 + 1 + 4 + 1 + 2);
    assert_eq!(seen[3], 9);
    assert_eq!(
        data.record.hands.map(|h| h.id),
        [0x4000_0101, 0x4000_0102, 0x4000_0103, 0x4000_0104]
    );
    assert_eq!(data.record.worn[0].id, 0x4000_0105);
    // The loadouts come before the inventory; each loadout hand is
    // remapped on its own, even the four copies of one item.
    assert_eq!(data.record.loadouts[0].hands[0].id, 0x4000_0106);
    assert_eq!(data.record.loadouts[0].hands[3].id, 0x4000_0109);
    assert_eq!(data.record.loadouts[0].worn[0].id, 0x4000_010A);
    assert_eq!(data.record.inventory[0].item, 0x4000_010B);
    assert_eq!(data.record.inventory[1].item, 0x4000_010C);

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

    // A skill with no technique chosen re-bases to −1 level, in 16 bits.
    let mut empty = CharacterData::new();
    assert_eq!(empty.unlearn_level(2, 1), None);
    assert_eq!(empty.record.skills[1].points, 0u16.wrapping_sub(5000));
}

/// Unlearning a technique also unlearns the techniques of the skill that
/// need it: Manoeuvring in Armour I takes II and III with it (15000 points
/// off), Remise takes Dual Wield, and Synchrony takes Ward but not Counter.
#[test]
fn unlearning_a_technique_takes_its_dependents() {
    let mut data = CharacterData::new();
    data.record.skills[ARMOUR_SLOT] = Skill {
        points: 25000,
        techniques: [1, 4, 2, 3, 5, 0],
    };
    assert_eq!(data.unlearn_level(3, 1), Some(1));
    assert_eq!(
        data.record.skills[ARMOUR_SLOT].techniques,
        [4, 5, 0, 0, 0, 0]
    );
    assert_eq!(data.record.skills[ARMOUR_SLOT].points, 10000);

    data.record.skills[0] = Skill {
        points: 15000,
        techniques: [1, 8, 2, 0, 0, 0],
    };
    assert_eq!(data.unlearn_level(1, 1), Some(1));
    assert_eq!(data.record.skills[0].techniques, [2, 0, 0, 0, 0, 0]);
    assert_eq!(data.record.skills[0].points, 5000);

    data.record.skills[3] = Skill {
        points: 20000,
        techniques: [1, 2, 3, 6, 0, 0],
    };
    assert_eq!(data.unlearn_level(4, 2), Some(2));
    assert_eq!(data.record.skills[3].techniques, [1, 6, 0, 0, 0, 0]);
    assert_eq!(data.record.skills[3].points, 10000);

    // The scan resumes at the second slot after a removal: removing II
    // (the third slot) takes the first III with it, the second III moves
    // into the first slot, and the resumed scan skips it, so it stays.
    let mut skipped = CharacterData::new();
    skipped.record.skills[ARMOUR_SLOT] = Skill {
        points: 25000,
        techniques: [3, 3, 2, 4, 5, 0],
    };
    assert_eq!(skipped.unlearn_level(3, 3), Some(2));
    assert_eq!(
        skipped.record.skills[ARMOUR_SLOT].techniques,
        [3, 4, 5, 0, 0, 0]
    );
    assert_eq!(skipped.record.skills[ARMOUR_SLOT].points, 15000);
}

/// The prerequisite lookup by technique id: Close Combat's Dual Wield
/// lists Remise and Riposte, Armour's second and third tiers the one before,
/// an empty id reads the previous skill's eighth entry and an id past the
/// table has none.
#[test]
fn a_techniques_prerequisites_come_from_the_table() {
    assert_eq!(technique_prerequisites(0, 8), [1, 4]);
    assert_eq!(technique_prerequisites(2, 2), [1, 0]);
    assert_eq!(technique_prerequisites(2, 3), [2, 0]);
    assert_eq!(technique_prerequisites(4, 8), [0, 0]);
    assert_eq!(technique_prerequisites(1, 0), [1, 4]);
    assert_eq!(technique_prerequisites(0, 0), [0, 0]);
    assert_eq!(technique_prerequisites(9, 1), [0, 0]);
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
    // The load: shares times the maximum in double precision, the focus
    // share on its own; both saved entries report the last entry's value,
    // as the original's loop does. 32768 × 2 / 65535 = 1.0000153; 16384 × 2
    // / 65535 = 0.50000763; 49151 / 65535 = 0.74999619.
    let mut health = Health {
        max: 2.0,
        body_value: 7.0,
        ..Health::default()
    };
    data.restore_health(&mut health);
    assert_eq!(health.yellow, 1.000_015_3);
    assert_eq!(health.red, 0.500_007_6);
    assert_eq!(health.focus, 0.749_996_2);
    assert_eq!(health.potion, Some(1800));
    assert_eq!(health.heal_pool, f32::from_bits(1800));
    // The body's own value is not the record's to load.
    assert_eq!(health.body_value, 7.0);

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
    // Without an entry the body keeps its heal pool and potion.
    let mut body = Health {
        heal_pool: 0.5,
        potion: Some(3),
        ..Health::default()
    };
    dead.restore_health(&mut body);
    assert_eq!((body.heal_pool, body.potion), (0.5, Some(3)));
    // NaN red and a zero maximum save as 0: the conversion's indefinite
    // value is cut to 16 bits.
    dead.save_health(Health {
        yellow: 1.0,
        red: f32::NAN,
        max: 0.0,
        ..Health::default()
    });
    assert_eq!((dead.record.pools.yellow, dead.record.pools.red), (0, 0));
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
    assert_eq!(items.len(), 1423, "objdb items");
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
    // The refusal's lock byte: 121 shipped items carry one over 50
    // (chests, drawers and tables 200 or 255, the ogre's club 100), and of
    // the apparel the golem suit and the ogre armour and greaves, at 100.
    let over_limit = |item: &&Item| item.equip_lock > EQUIP_LOCK_LIMIT;
    assert_eq!(items.iter().filter(over_limit).count(), 121);
    let locked_apparel: Vec<(u32, u8)> = apparel
        .iter()
        .copied()
        .filter(over_limit)
        .map(|item| (item.id, item.equip_lock))
        .collect();
    assert_eq!(
        locked_apparel,
        [498, 503, 505, 508, 526, 528, 532].map(|id| (id, 100))
    );
    // Whatever overlaps a worn golem suit cannot go on over it.
    let golem = find(498).expect("the golem suit");
    let over = apparel
        .iter()
        .find(|item| item.id != golem.id && overlaps(item, golem))
        .expect("an item overlapping the golem suit");
    let mut suited = CharacterData::new();
    suited.record.worn = refs(&[golem.id]);
    assert_eq!(suited.equip_conflicts(over, find), None);
    assert_eq!(suited.record.worn, refs(&[golem.id]));

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
    assert_eq!(removed, Some(refs(&[UNDERWEAR_ITEM])));
    assert_eq!(data.record.worn, refs(&[shirt.id]));
    assert_eq!(data.record.worn[0].id, shirt.id);
}
