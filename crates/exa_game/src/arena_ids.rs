//! The arena's item ids across a save and a load, and the fighter-rank
//! helper the board and the company share.
//!
//! Before saving, every item the arena owns is given its saved id, in this
//! order: the manager's record, the hub stash, the arsenal's five shelves
//! of five boxes, then each employee's record and own item list. A record
//! is prepared hands (all four), worn items, loadouts (each one's four
//! hands, then its worn items), and last the carried items.
//!
//! After a load the saved ids are mapped back, entry by entry through the
//! arsenal. An entry flagged `0x400` remaps the stash and the loadouts of
//! the character holding it; one flagged `0x200` remaps the hands and the
//! carried items. The arsenal entry itself then takes the new id.
//!
//! The `0x400` pass replaces the first stash item that matches and then
//! remaps the manager's loadouts, ending there. With no stash match it
//! walks the staff: the first employee whose own list holds the id has it
//! replaced and that employee's loadouts remapped, and the walk stops. A
//! loadout's first two hand slots are searched (the first match ends that
//! search), then its worn list (the first match ends that too).
//!
//! The `0x200` pass checks a record's first hand pair, its second hand pair
//! (each pair stops at its first match, and neither ends the pass), then its
//! carried items, where the first match ends the whole pass. The manager
//! comes first, then each employee. Loadouts are not touched.
//!
//! Evidence: the arena's save-id preparation and the two post-load remap
//! routines, read from `Exanima.exe` 0.9.5.

use exa_formats::arena_save::ItemEntry;
use exa_formats::characters::CharacterRecord;

use crate::arsenal::{Arsenal, ArsenalEntry, ItemKind};
use crate::career::SKILL_STEP;

/// The flag an arsenal entry carries when its id also lives in an
/// employee's record or own item list.
pub const EMPLOYEE_REF: u32 = 0x400;
/// The flag an arsenal entry carries when its id also lives in a
/// character's record.
pub const CHARACTER_REF: u32 = 0x200;

/// An employee as the save-id passes see it: the character record and the
/// employee's own list of twelve-byte item references.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EmployeeRefs {
    pub record: CharacterRecord,
    pub items: Vec<ItemEntry>,
}

/// The `0x400` flag a reclassified item enters the arsenal with.
pub const SAVED_ITEM: u32 = 0x400;

/// Puts an item on the shelf its object record names, unless the shelf
/// already holds it. The save's reclassification pass adds the item with
/// the saved-item flag; a shelf that already has any copy of the id is left
/// alone. Returns whether the item was added.
pub fn add_classified(arsenal: &mut Arsenal, item: u32, kind: ItemKind) -> bool {
    let Some(shelf) = crate::arsenal::shelf_of(kind) else {
        return false;
    };
    if arsenal.shelves[shelf]
        .iter()
        .any(|entries| entries.iter().any(|entry| entry.item == item))
    {
        return false;
    }
    arsenal.stow(item, kind, SAVED_ITEM)
}

/// Reclassifies a list of twelve-byte item records into the arsenal, as the
/// load's id pass does with the manager's own items: each record's first
/// word is looked up in the object database and added to its shelf. The
/// list is expected to have been deduplicated first.
pub fn reclassify_records(
    arsenal: &mut Arsenal,
    records: &[[u8; 12]],
    mut kind_of: impl FnMut(u32) -> ItemKind,
) -> usize {
    let mut added = 0;
    for record in records {
        let id = u32::from_le_bytes([record[0], record[1], record[2], record[3]]);
        if add_classified(arsenal, id, kind_of(id)) {
            added += 1;
        }
    }
    added
}

/// Prepare one character record: its hands, worn items, loadouts, then
/// its carried items.
pub fn prepare_character_items(
    character: &mut CharacterRecord,
    prepare_item: &mut impl FnMut(u32) -> u32,
) {
    prepare_character(character, prepare_item);
}

/// Flushes one scene character before a save: a reference with its top bit
/// set registers the character in the world character database under the
/// low 28 bits, and every item in the record takes its saved id. The
/// original also rewrites the ids of objects attached to the character;
/// those attachments are the caller's.
///
/// Returns the id to register, or `None` when the character was already
/// registered.
pub fn flush_scene_character(
    character: &mut CharacterRecord,
    reference: u32,
    mut prepare_item: impl FnMut(u32) -> u32,
) -> Option<u32> {
    let register = reference & 0x8000_0000 != 0;
    prepare_character(character, &mut prepare_item);
    register.then_some(reference & 0x0fff_ffff)
}

/// The fighter rank the board and the company read: the six skills' levels
/// (each experience word over 5000) summed, halved and capped at Master.
pub fn fighter_rank(skill_xp: &[u16; 6]) -> u32 {
    let sum = skill_xp.iter().take(6).fold(0u32, |sum, &word| {
        sum.saturating_add(u32::from(word) / u32::from(SKILL_STEP))
    });
    (sum / 2).min(5)
}

/// Give every arena item its saved id. `prepare_item` is the object
/// database's id preparation; it is called once per hand, worn item,
/// loadout item, carried item, stash item and arsenal entry, in the
/// original's order: manager, stash, arsenal, staff.
pub fn prepare_ids(
    arsenal: &mut Arsenal,
    stash: &mut [u32],
    manager: &mut CharacterRecord,
    employees: &mut [EmployeeRefs],
    mut prepare_item: impl FnMut(u32) -> u32,
) {
    prepare_character(manager, &mut prepare_item);
    for item in stash {
        *item = prepare_item(*item);
    }
    for shelf in &mut arsenal.shelves {
        for entries in shelf {
            for entry in entries {
                entry.item = prepare_item(entry.item);
            }
        }
    }
    for employee in employees {
        prepare_character(&mut employee.record, &mut prepare_item);
        for item in &mut employee.items {
            item.id = prepare_item(item.id);
        }
    }
}

/// Prepare one character record: its hands, worn items, loadouts, then
/// its carried items.
fn prepare_character(character: &mut CharacterRecord, prepare_item: &mut impl FnMut(u32) -> u32) {
    for hand in &mut character.hands {
        hand.id = prepare_item(hand.id);
    }
    for worn in &mut character.worn {
        worn.id = prepare_item(worn.id);
    }
    for loadout in &mut character.loadouts {
        for hand in &mut loadout.hands {
            hand.id = prepare_item(hand.id);
        }
        for worn in &mut loadout.worn {
            worn.id = prepare_item(worn.id);
        }
    }
    for item in &mut character.inventory {
        item.item = prepare_item(item.item);
    }
}

/// Map one id through the load's object table. An entry whose id did not
/// change is left alone (the original only remaps entries the loader
/// rewrote).
pub fn remap_arsenal_ids(
    arsenal: &mut Arsenal,
    stash: &mut [u32],
    manager: &mut CharacterRecord,
    employees: &mut [EmployeeRefs],
    mut map_item: impl FnMut(u32) -> u32,
) {
    for shelf in &mut arsenal.shelves {
        for entries in shelf {
            for entry in entries {
                remap_entry(entry, stash, manager, employees, &mut map_item);
            }
        }
    }
}

fn remap_entry(
    entry: &mut ArsenalEntry,
    stash: &mut [u32],
    manager: &mut CharacterRecord,
    employees: &mut [EmployeeRefs],
    map_item: &mut impl FnMut(u32) -> u32,
) {
    let old = entry.item;
    let new = map_item(old);
    if new == old {
        return;
    }
    if entry.flags & EMPLOYEE_REF != 0 {
        remap_employee_refs(old, new, stash, manager, employees);
    }
    if entry.flags & CHARACTER_REF != 0 {
        remap_character_refs(old, new, manager, employees);
    }
    entry.item = new;
}

/// The `0x400` pass: the first matching stash item is replaced and the
/// manager's loadouts remapped, ending the pass. With no stash match, the
/// first employee whose own list holds the id has it replaced and their
/// loadouts remapped.
pub fn remap_employee_refs(
    old: u32,
    new: u32,
    stash: &mut [u32],
    manager: &mut CharacterRecord,
    employees: &mut [EmployeeRefs],
) {
    if let Some(item) = stash.iter_mut().find(|item| **item == old) {
        *item = new;
        remap_loadouts(old, new, manager);
        return;
    }
    for employee in employees {
        if let Some(item) = employee.items.iter_mut().find(|item| item.id == old) {
            item.id = new;
            remap_loadouts(old, new, &mut employee.record);
            return;
        }
    }
}

/// Remap the first two hand slots and the worn items of every loadout.
/// A hand match ends that loadout's hand search; a worn match ends its
/// worn search.
pub fn remap_loadouts(old: u32, new: u32, character: &mut CharacterRecord) -> bool {
    let mut any = false;
    for loadout in &mut character.loadouts {
        for hand in loadout.hands.iter_mut().take(2) {
            if hand.id == old {
                hand.id = new;
                any = true;
                break;
            }
        }
        if let Some(worn) = loadout.worn.iter_mut().find(|item| item.id == old) {
            worn.id = new;
            any = true;
        }
    }
    any
}

/// The `0x200` pass: the manager's two hand pairs and carried items, then
/// each employee's in the same order. A carried match ends the pass.
pub fn remap_character_refs(
    old: u32,
    new: u32,
    manager: &mut CharacterRecord,
    employees: &mut [EmployeeRefs],
) {
    if remap_personal(old, new, manager) {
        return;
    }
    for employee in employees {
        if remap_personal(old, new, &mut employee.record) {
            return;
        }
    }
}

/// One record's `0x200` search: the first match in each hand pair is
/// replaced (neither stops the search), then the first matching carried
/// item is replaced and reported.
pub fn remap_personal(old: u32, new: u32, character: &mut CharacterRecord) -> bool {
    for pair in character.hands.chunks_mut(2) {
        if let Some(hand) = pair.iter_mut().find(|hand| hand.id == old) {
            hand.id = new;
        }
    }
    if let Some(item) = character.inventory.iter_mut().find(|item| item.item == old) {
        item.item = new;
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arsenal::{ARMOUR, CLOTHING, SHIELDS, WEAPONS};
    use exa_formats::characters::{ItemRef, Loadout};

    fn character(
        hands: [u32; 2],
        carried: &[u32],
        loadouts: &[([u32; 2], &[u32])],
    ) -> CharacterRecord {
        let mut record = CharacterRecord::new();
        record.hands[0] = ItemRef::new(hands[0], 0);
        record.hands[1] = ItemRef::new(hands[1], 0);
        record.inventory = carried
            .iter()
            .map(|&id| exa_formats::characters::Carried {
                item: id,
                flags: 0,
                extra: 0,
            })
            .collect();
        record.loadouts = loadouts
            .iter()
            .map(|&(hands, worn)| Loadout {
                word: 0,
                hands: [
                    ItemRef::new(hands[0], 0),
                    ItemRef::new(hands[1], 0),
                    ItemRef::NONE,
                    ItemRef::NONE,
                ],
                worn: worn.iter().map(|&id| ItemRef::new(id, 0)).collect(),
            })
            .collect();
        record
    }

    fn entry(id: u32, flags: u32) -> ArsenalEntry {
        ArsenalEntry {
            item: id,
            flags,
            first: 0,
            second: 0,
        }
    }

    #[test]
    fn the_rank_halves_six_skill_levels_and_caps_at_master() {
        assert_eq!(fighter_rank(&[0; 6]), 0);
        assert_eq!(fighter_rank(&[5000, 0, 0, 0, 0, 0]), 0);
        assert_eq!(fighter_rank(&[5000, 5000, 0, 0, 0, 0]), 1);
        // Ten levels halve to Master; more stay there.
        assert_eq!(fighter_rank(&[50_000, 50_000, 0, 0, 0, 0]), 5);
        assert_eq!(fighter_rank(&[65_535; 6]), 5);
        // Each word is floored to its level; only six are read.
        assert_eq!(fighter_rank(&[9_999, 9_999, 0, 0, 0, 0]), 1);
        assert_eq!(fighter_rank(&[5000, 5000, 5000, 5000, 5000, 5000]), 3);
    }

    #[test]
    fn preparation_walks_manager_stash_arsenal_and_staff() {
        let mut arsenal = Arsenal::default();
        arsenal.shelves[WEAPONS][0].push(entry(10, CHARACTER_REF));
        arsenal.shelves[ARMOUR][1].push(entry(20, EMPLOYEE_REF));
        let mut stash = vec![30];
        let mut manager = character([40, 0], &[50], &[([60, 0], &[70])]);
        manager.worn = vec![ItemRef::new(45, 0)];
        let mut staff = vec![EmployeeRefs {
            record: character([80, 0], &[90], &[]),
            items: vec![ItemEntry {
                id: 100,
                flags: 0,
                extra: 0,
            }],
        }];
        let mut seen = Vec::new();
        prepare_ids(&mut arsenal, &mut stash, &mut manager, &mut staff, |id| {
            seen.push(id);
            id + 1
        });
        // A record: four hands, worn items, each loadout's four hands and
        // its worn items, then the carried items.
        assert_eq!(
            seen,
            vec![
                40, 0, 0, 0, 45, 60, 0, 0, 0, 70, 50, 30, 10, 20, 80, 0, 0, 0, 90, 100
            ]
        );
        assert_eq!(arsenal.shelves[WEAPONS][0][0].item, 11);
        assert_eq!(arsenal.shelves[ARMOUR][1][0].item, 21);
        assert_eq!(stash, vec![31]);
        assert_eq!(manager.hands[0].id, 41);
        assert_eq!(manager.worn[0].id, 46);
        assert_eq!(manager.inventory[0].item, 51);
        assert_eq!(manager.loadouts[0].hands[0].id, 61);
        assert_eq!(manager.loadouts[0].worn[0].id, 71);
        assert_eq!(staff[0].items[0].id, 101);
    }

    #[test]
    fn the_character_pass_checks_both_hand_pairs_then_carried_items() {
        let mut manager = character([1, 2], &[3], &[([4, 0], &[5])]);
        manager.hands[2] = ItemRef::new(1, 0);
        remap_character_refs(1, 9, &mut manager, &mut []);
        // Each hand pair replaces its first match; neither ends the pass.
        assert_eq!(manager.hands[0].id, 9);
        assert_eq!(manager.hands[2].id, 9);
        manager.hands[1] = ItemRef::new(9, 0);
        manager.hands[0] = ItemRef::new(9, 0);
        remap_character_refs(9, 7, &mut manager, &mut []);
        // Only the first match of the pair goes.
        assert_eq!(manager.hands[0].id, 7);
        assert_eq!(manager.hands[1].id, 9);
        remap_character_refs(3, 8, &mut manager, &mut []);
        assert_eq!(manager.inventory[0].item, 8);
        // The loadouts are not part of this pass.
        remap_character_refs(4, 6, &mut manager, &mut []);
        remap_character_refs(5, 6, &mut manager, &mut []);
        assert_eq!(manager.loadouts[0].hands[0].id, 4);
        assert_eq!(manager.loadouts[0].worn[0].id, 5);
    }

    #[test]
    fn a_carried_match_ends_the_character_pass_before_the_staff() {
        let mut manager = character([0, 0], &[3], &[]);
        let mut staff = vec![EmployeeRefs {
            record: character([3, 0], &[3], &[]),
            items: vec![],
        }];
        remap_character_refs(3, 8, &mut manager, &mut staff);
        assert_eq!(manager.inventory[0].item, 8);
        assert_eq!(staff[0].record.hands[0].id, 3);
        assert_eq!(staff[0].record.inventory[0].item, 3);
        // Without a manager match the staff is searched in turn.
        remap_character_refs(3, 4, &mut manager, &mut staff);
        assert_eq!(staff[0].record.hands[0].id, 4);
        assert_eq!(staff[0].record.inventory[0].item, 4);
    }

    #[test]
    fn the_employee_pass_replaces_one_stash_item_and_the_managers_loadouts() {
        let mut stash = vec![1, 1];
        let mut manager = character([0, 0], &[], &[([1, 1], &[1, 1])]);
        let mut staff = vec![EmployeeRefs {
            record: character([0, 0], &[], &[([1, 0], &[])]),
            items: vec![ItemEntry {
                id: 1,
                flags: 0,
                extra: 0,
            }],
        }];
        remap_employee_refs(1, 5, &mut stash, &mut manager, &mut staff);
        // Only the first stash item goes, then the manager's loadouts (the
        // first hand match and the first worn match), and the pass ends
        // without reaching the staff.
        assert_eq!(stash, vec![5, 1]);
        assert_eq!(manager.loadouts[0].hands[0].id, 5);
        assert_eq!(manager.loadouts[0].hands[1].id, 1);
        assert_eq!(manager.loadouts[0].worn[0].id, 5);
        assert_eq!(manager.loadouts[0].worn[1].id, 1);
        assert_eq!(staff[0].items[0].id, 1);
        assert_eq!(staff[0].record.loadouts[0].hands[0].id, 1);
    }

    #[test]
    fn the_employee_pass_reaches_the_staff_without_a_stash_match() {
        let mut stash = vec![2];
        let mut manager = character([0, 0], &[], &[([1, 0], &[])]);
        let entry_with = |id| ItemEntry {
            id,
            flags: 0,
            extra: 0,
        };
        let mut staff = vec![
            EmployeeRefs {
                record: character([0, 0], &[], &[([1, 0], &[])]),
                items: vec![entry_with(3)],
            },
            EmployeeRefs {
                record: character([0, 0], &[], &[([1, 0], &[1])]),
                items: vec![entry_with(1), entry_with(1)],
            },
            EmployeeRefs {
                record: character([0, 0], &[], &[([1, 0], &[])]),
                items: vec![entry_with(1)],
            },
        ];
        remap_employee_refs(1, 5, &mut stash, &mut manager, &mut staff);
        // The manager is not touched; the second employee's first listed
        // match and their loadouts change; the walk then stops.
        assert_eq!(manager.loadouts[0].hands[0].id, 1);
        assert_eq!(staff[0].record.loadouts[0].hands[0].id, 1);
        assert_eq!(staff[1].items[0].id, 5);
        assert_eq!(staff[1].items[1].id, 1);
        assert_eq!(staff[1].record.loadouts[0].hands[0].id, 5);
        assert_eq!(staff[1].record.loadouts[0].worn[0].id, 5);
        assert_eq!(staff[2].items[0].id, 1);
        assert_eq!(staff[2].record.loadouts[0].hands[0].id, 1);
    }

    #[test]
    fn an_arsenal_remap_runs_the_flagged_pass_and_stores_the_new_id() {
        let mut arsenal = Arsenal::default();
        arsenal.shelves[WEAPONS][0].push(entry(10, CHARACTER_REF));
        arsenal.shelves[ARMOUR][0].push(entry(20, EMPLOYEE_REF));
        arsenal.shelves[WEAPONS][1].push(entry(30, 0));
        let mut stash = vec![20];
        let mut manager = character([10, 0], &[], &[]);
        let mut staff = vec![EmployeeRefs {
            record: character([20, 0], &[], &[]),
            items: vec![ItemEntry {
                id: 20,
                flags: 0,
                extra: 0,
            }],
        }];
        remap_arsenal_ids(&mut arsenal, &mut stash, &mut manager, &mut staff, |id| {
            id + 100
        });
        assert_eq!(arsenal.shelves[WEAPONS][0][0].item, 110);
        assert_eq!(arsenal.shelves[ARMOUR][0][0].item, 120);
        // Every entry takes the mapped id; the flags only choose which
        // references are remapped with it.
        assert_eq!(arsenal.shelves[WEAPONS][1][0].item, 130);
        assert_eq!(manager.hands[0].id, 110);
        assert_eq!(stash, vec![120]);
        // The stash match ended the employee pass before the staff.
        assert_eq!(staff[0].items[0].id, 20);
        // That pass leaves hands alone.
        assert_eq!(staff[0].record.hands[0].id, 20);
    }

    fn kind(weapon: bool, shield: bool, layer: Option<u8>) -> ItemKind {
        ItemKind {
            known: true,
            apparel_layer: layer,
            weapon,
            shield,
        }
    }

    #[test]
    fn reclassification_puts_an_item_on_its_shelf_once_with_the_saved_flag() {
        let mut arsenal = Arsenal::default();
        assert!(add_classified(&mut arsenal, 10, kind(true, false, None)));
        assert_eq!(arsenal.shelves[WEAPONS][0][0].item, 10);
        assert_eq!(arsenal.shelves[WEAPONS][0][0].flags, SAVED_ITEM);
        // A second copy on any box of the same shelf is refused.
        arsenal.shelves[WEAPONS][4].push(entry(10, 0));
        assert!(!add_classified(&mut arsenal, 10, kind(true, false, None)));
        // Apparel of layer 3 and 6 is armour; the other layers clothing.
        assert!(add_classified(
            &mut arsenal,
            11,
            kind(false, false, Some(3))
        ));
        assert!(add_classified(
            &mut arsenal,
            12,
            kind(false, false, Some(6))
        ));
        assert!(add_classified(
            &mut arsenal,
            13,
            kind(false, false, Some(2))
        ));
        assert_eq!(arsenal.shelves[ARMOUR][0][0].item, 11);
        assert_eq!(arsenal.shelves[ARMOUR][0][1].item, 12);
        assert_eq!(arsenal.shelves[CLOTHING][0][0].item, 13);
        assert!(add_classified(&mut arsenal, 14, kind(false, true, None)));
        assert_eq!(arsenal.shelves[SHIELDS][0][0].item, 14);
        // An unknown item has no shelf.
        assert!(!add_classified(
            &mut arsenal,
            15,
            ItemKind {
                known: false,
                ..ItemKind::default()
            }
        ));
    }

    #[test]
    fn the_id_pass_reclassifies_each_records_first_word() {
        let mut records = [[0u8; 12]; 3];
        for (slot, id) in records.iter_mut().zip([20u32, 21, 22]) {
            slot[..4].copy_from_slice(&id.to_le_bytes());
        }
        let mut arsenal = Arsenal::default();
        let added = reclassify_records(&mut arsenal, &records, |id| match id {
            20 => kind(true, false, None),
            21 => kind(false, true, None),
            _ => kind(false, false, Some(4)),
        });
        assert_eq!(added, 3);
        assert_eq!(arsenal.shelves[WEAPONS][0][0].item, 20);
        assert_eq!(arsenal.shelves[SHIELDS][0][0].item, 21);
        assert_eq!(arsenal.shelves[ARMOUR][0][0].item, 22);
        // Re-running with the same classification adds nothing.
        assert_eq!(
            reclassify_records(&mut arsenal, &records, |id| match id {
                20 => kind(true, false, None),
                21 => kind(false, true, None),
                _ => kind(false, false, Some(4)),
            }),
            0
        );
    }

    #[test]
    fn flushing_a_scene_character_registers_once_and_remaps_every_item() {
        let mut record = character([40, 0], &[50], &[([60, 0], &[70])]);
        record.worn = vec![ItemRef::new(45, 0)];
        let mut seen = Vec::new();
        let registered = flush_scene_character(&mut record, 0x8000_0000 | 0x1234, |id| {
            seen.push(id);
            id + 1
        });
        assert_eq!(registered, Some(0x1234));
        // The record's hands, worn items, loadout hands and worn items, then
        // its carried items.
        assert_eq!(seen, vec![40, 0, 0, 0, 45, 60, 0, 0, 0, 70, 50]);
        assert_eq!(record.hands[0].id, 41);
        assert_eq!(record.worn[0].id, 46);
        assert_eq!(record.inventory[0].item, 51);
        // A reference without the top bit does not register again.
        assert_eq!(flush_scene_character(&mut record, 0x1234, |id| id), None);
    }
}
