//! The arena's item ids across a save and a load, and the fighter-rank
//! helper the board and the company share.
//!
//! Before saving, every item the arena owns is given its saved id: the
//! arsenal's five shelves of five boxes, the hub stash, the manager's
//! record and each employee's record and own item list. After a load the
//! saved ids are mapped back. An arsenal entry flagged `0x400` remaps the
//! employees' records and own lists; one flagged `0x200` remaps the
//! characters' hands, carried items and loadouts. The arsenal entry itself
//! then takes the new id.
//!
//! A character's loadouts are remapped first. Each loadout's first two hand
//! slots are checked (a match on the first ends the slots' search) and then
//! its worn list, where a match ends that loadout. The manager's loadouts
//! are the end of the first pass: the staff is reached only when he has
//! none. The employees' pass walks their own item lists first, and on a
//! match remaps that employee's loadouts and stops walking the staff.
//!
//! The personal pass checks the character's two own hand slots, then his
//! carried items, then his loadouts; the same order repeats per employee
//! and a match ends the whole pass.
//!
//! Evidence: the arena's save-id preparation and the two post-load remap
//! routines, read from `Exanima.exe` 0.9.5.

use exa_formats::arena_save::ItemEntry;
use exa_formats::characters::{CharacterRecord, ItemRef};

use crate::arsenal::{Arsenal, ArsenalEntry};
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

/// The fighter rank the board and the company read: the six skills' levels
/// (each experience word over 5000) summed, halved and capped at Master.
pub fn fighter_rank(skill_xp: &[u16; 6]) -> u32 {
    let sum = skill_xp.iter().take(6).fold(0u32, |sum, &word| {
        sum.saturating_add(u32::from(word) / u32::from(SKILL_STEP))
    });
    (sum / 2).min(5)
}

/// Give every arena item its saved id. `prepare_item` is the object
/// database's id preparation; it is called once per arsenal entry, once per
/// stash item and once per character item reference, in the original's
/// order: arsenal, stash, manager, staff.
pub fn prepare_ids(
    arsenal: &mut Arsenal,
    stash: &mut [u32],
    manager: &mut CharacterRecord,
    employees: &mut [EmployeeRefs],
    mut prepare_item: impl FnMut(u32) -> u32,
) {
    for shelf in &mut arsenal.shelves {
        for entries in shelf {
            for entry in entries {
                entry.item = prepare_item(entry.item);
            }
        }
    }
    for item in stash {
        *item = prepare_item(*item);
    }
    prepare_character(manager, &mut prepare_item);
    for employee in employees {
        prepare_character(&mut employee.record, &mut prepare_item);
        for item in &mut employee.items {
            item.id = prepare_item(item.id);
        }
    }
}

/// Prepare one character record: its hands, carried items and loadouts.
fn prepare_character(character: &mut CharacterRecord, prepare_item: &mut impl FnMut(u32) -> u32) {
    for hand in &mut character.hands {
        hand.id = prepare_item(hand.id);
    }
    for item in &mut character.inventory {
        item.item = prepare_item(item.item);
    }
    for loadout in &mut character.loadouts {
        for hand in &mut loadout.hands {
            hand.id = prepare_item(hand.id);
        }
        for worn in &mut loadout.worn {
            worn.id = prepare_item(worn.id);
        }
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

/// The `0x400` pass: the stash and the manager's loadouts, then each
/// employee's own list and, on a match, that employee's loadouts.
pub fn remap_employee_refs(
    old: u32,
    new: u32,
    stash: &mut [u32],
    manager: &mut CharacterRecord,
    employees: &mut [EmployeeRefs],
) {
    for item in stash.iter_mut() {
        if *item == old {
            *item = new;
        }
    }
    remap_loadouts(old, new, manager);
    if !manager.loadouts.is_empty() {
        return;
    }
    for employee in employees {
        let mut found = false;
        if let Some(item) = employee.items.iter_mut().find(|item| item.id == old) {
            item.id = new;
            found = true;
        }
        if found {
            remap_loadouts(old, new, &mut employee.record);
            break;
        }
    }
}

/// Remap the first two hand slots and the worn items of every loadout.
/// A hand match ends that loadout's hand search; a worn match ends the
/// loadout.
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

/// The `0x200` pass: the manager's own hands and carried items, his
/// loadouts, then each employee's in the same order. A match ends the
/// pass.
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

/// The two own hand slots, the carried items, then the loadouts.
pub fn remap_personal(old: u32, new: u32, character: &mut CharacterRecord) -> bool {
    for hand in character.hands.iter_mut().take(2) {
        if hand.id == old {
            hand.id = new;
            return true;
        }
    }
    if let Some(item) = character.inventory.iter_mut().find(|item| item.item == old) {
        item.item = new;
        return true;
    }
    remap_loadouts(old, new, character)
}

/// The two own hand slots remapped in place, without stopping at the first
/// match (the record pass's helper, kept for callers that hold both slots).
pub fn remap_hand_slots(hands: &mut [ItemRef], old: u32, new: u32) -> bool {
    let mut any = false;
    for hand in hands.iter_mut().take(2) {
        if hand.id == old {
            hand.id = new;
            any = true;
        }
    }
    any
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arsenal::{ARMOUR, WEAPONS};
    use exa_formats::characters::Loadout;

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
    fn preparation_walks_arsenal_stash_manager_and_staff() {
        let mut arsenal = Arsenal::default();
        arsenal.shelves[WEAPONS][0].push(entry(10, CHARACTER_REF));
        arsenal.shelves[ARMOUR][1].push(entry(20, EMPLOYEE_REF));
        let mut stash = vec![30];
        let mut manager = character([40, 0], &[50], &[([60, 0], &[70])]);
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
        assert_eq!(
            seen,
            vec![
                10, 20, 30, 40, 0, 0, 0, 50, 60, 0, 0, 0, 70, 80, 0, 0, 0, 90, 100
            ]
        );
        assert_eq!(arsenal.shelves[WEAPONS][0][0].item, 11);
        assert_eq!(arsenal.shelves[ARMOUR][1][0].item, 21);
        assert_eq!(stash, vec![31]);
        assert_eq!(manager.hands[0].id, 41);
        assert_eq!(manager.inventory[0].item, 51);
        assert_eq!(manager.loadouts[0].hands[0].id, 61);
        assert_eq!(manager.loadouts[0].worn[0].id, 71);
        assert_eq!(staff[0].items[0].id, 101);
    }

    #[test]
    fn the_character_pass_checks_hands_carried_and_loadouts_in_order() {
        let mut manager = character([1, 2], &[3], &[([4, 0], &[5])]);
        remap_character_refs(1, 9, &mut manager, &mut []);
        assert_eq!(manager.hands[0].id, 9);
        // A hand match ended the pass; the carried item is untouched.
        assert_eq!(manager.inventory[0].item, 3);
        remap_character_refs(3, 8, &mut manager, &mut []);
        assert_eq!(manager.inventory[0].item, 8);
        remap_character_refs(4, 7, &mut manager, &mut []);
        assert_eq!(manager.loadouts[0].hands[0].id, 7);
        remap_character_refs(5, 6, &mut manager, &mut []);
        assert_eq!(manager.loadouts[0].worn[0].id, 6);
    }

    #[test]
    fn the_employee_pass_reaches_staff_only_without_manager_loadouts() {
        let mut stash = vec![1];
        let mut manager = character([0, 0], &[], &[]);
        let mut staff = vec![
            EmployeeRefs {
                record: character([7, 0], &[], &[]),
                items: vec![ItemEntry {
                    id: 2,
                    flags: 0,
                    extra: 0,
                }],
            },
            EmployeeRefs {
                record: character([7, 0], &[], &[]),
                items: vec![ItemEntry {
                    id: 1,
                    flags: 0,
                    extra: 0,
                }],
            },
        ];
        remap_employee_refs(1, 5, &mut stash, &mut manager, &mut staff);
        assert_eq!(stash, vec![5]);
        assert_eq!(staff[0].items[0].id, 2);
        assert_eq!(staff[1].items[0].id, 5);
        // The first employee had no match; the second's own list matches,
        // its loadouts are remapped, and the pass stops there.
        assert_eq!(staff[0].record.hands[0].id, 7);
        assert_eq!(staff[1].record.hands[0].id, 7);
    }

    #[test]
    fn a_manager_with_loadouts_ends_the_employee_pass() {
        let mut stash = vec![1];
        let mut manager = character([0, 0], &[], &[([1, 0], &[])]);
        let mut staff = vec![EmployeeRefs {
            record: character([1, 0], &[], &[]),
            items: vec![ItemEntry {
                id: 1,
                flags: 0,
                extra: 0,
            }],
        }];
        remap_employee_refs(1, 5, &mut stash, &mut manager, &mut staff);
        assert_eq!(manager.loadouts[0].hands[0].id, 5);
        assert_eq!(staff[0].record.hands[0].id, 1);
        assert_eq!(staff[0].items[0].id, 1);
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
        assert_eq!(staff[0].items[0].id, 120);
        // The character pass stopped at the manager's own hand match.
        assert_eq!(staff[0].record.hands[0].id, 20);
    }
}
