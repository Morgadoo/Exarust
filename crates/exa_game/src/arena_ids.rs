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

/// The classification lookup the load's id pass uses over an object
/// database's item records: [`ItemKind::of_item`] for every record, and an
/// id the database does not hold is unknown and goes on no shelf.
pub fn kinds_from_items(
    items: &[exa_formats::objdb::Item],
) -> std::collections::HashMap<u32, ItemKind> {
    items
        .iter()
        .map(|item| (item.id, ItemKind::of_item(item)))
        .collect()
}

/// What flushing one scene character produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneCharacterFlush {
    /// The id to register in the world character database (the reference's
    /// low 28 bits), or `None` when the reference's top bit is clear.
    pub register: Option<u32>,
    /// The whole reference the original stores on the record, or `None`
    /// when its top bit is clear.
    pub record_reference: Option<u32>,
    /// The first hand pair's (old, new) ids: the original rewrites the ids
    /// of the objects attached to the character through these, which is
    /// the caller's.
    pub hands: [(u32, u32); 2],
    /// The attached scene nodes that took a hand's new id.
    pub attached_nodes: usize,
}

/// Flushes one scene character before a save, in the original's order:
/// a reference with its top bit set registers the character under the low
/// 28 bits and stores the whole reference on the record; then every worn
/// item takes its saved id, then the first hand pair, then the character's
/// attached scene-object nodes are rewritten from the first pair (every
/// node whose object id equals a saved hand id takes the new one), then the
/// second pair, then each carried item. A carried item whose id is one of
/// the hands' old ids takes that hand's new id (the first pair is looked up
/// before the second) instead of a fresh call. The loadouts are not
/// touched.
pub fn flush_scene_character(
    character: &mut CharacterRecord,
    reference: u32,
    attached_nodes: &mut [u32],
    mut prepare_item: impl FnMut(u32) -> u32,
) -> SceneCharacterFlush {
    let registered = reference & 0x8000_0000 != 0;
    let register = registered.then_some(reference & 0x0fff_ffff);
    for worn in &mut character.worn {
        worn.id = prepare_item(worn.id);
    }
    let mut remap_pair = |first: usize| {
        std::array::from_fn::<(u32, u32), 2, _>(|offset| {
            let hand = &mut character.hands[first + offset];
            let old = hand.id;
            hand.id = prepare_item(old);
            (old, hand.id)
        })
    };
    let hands = remap_pair(0);
    let mut attached_nodes_updated = 0;
    for node in attached_nodes.iter_mut() {
        if let Some(&(_, new)) = hands.iter().find(|(old, _)| *old == *node) {
            *node = new;
            attached_nodes_updated += 1;
        }
    }
    let second = remap_pair(2);
    for item in &mut character.inventory {
        let reused = hands
            .iter()
            .chain(second.iter())
            .find(|(old, _)| *old == item.item);
        item.item = match reused {
            Some(&(_, new)) => new,
            None => prepare_item(item.item),
        };
    }
    SceneCharacterFlush {
        register,
        record_reference: registered.then_some(reference),
        hands,
        attached_nodes: attached_nodes_updated,
    }
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

/// The item word a regenerated list entry carries after the id and flags:
/// the low half `0x28`, the high half 4, as the original's list append
/// stores them (the arsenal's own entries carry the same pair).
pub const LIST_ENTRY_EXTRA: u32 = 0x28 | (4 << 16);

/// A worn id the list regeneration steps over. The original skips a worn
/// entry whose id word is exactly 8; what that value marks is not traced
/// [guess: a placeholder slot value].
pub const SKIPPED_WORN_ID: u32 = 8;

/// The employee class whose own list is emptied, not regenerated, after a
/// load (the class byte of a world character hired from the list).
pub const CLASS_WITHOUT_LIST: u32 = 4;

/// Adds an id to a twelve-byte list unless an entry already carries it, as
/// the original's unique append does: the new entry is `(id, flags, 0x28 |
/// 4 << 16)` at the end. Returns whether it was added.
pub fn add_unique_entry(list: &mut Vec<ItemEntry>, id: u32, flags: u32) -> bool {
    if list.iter().any(|entry| entry.id == id) {
        return false;
    }
    list.push(ItemEntry {
        id,
        flags,
        extra: LIST_ENTRY_EXTRA,
    });
    true
}

/// Merges a record's loadout items into a list, as the load's list
/// regeneration does: for each loadout in order, its first two hand ids
/// (a zero id is empty), then its worn ids other than
/// [`SKIPPED_WORN_ID`], each through [`add_unique_entry`] with flags 0.
/// The list is not cleared first.
pub fn merge_loadout_items(record: &CharacterRecord, list: &mut Vec<ItemEntry>) {
    for loadout in &record.loadouts {
        for hand in loadout.hands.iter().take(2) {
            if hand.id != 0 {
                add_unique_entry(list, hand.id, 0);
            }
        }
        for worn in &loadout.worn {
            if worn.id != SKIPPED_WORN_ID {
                add_unique_entry(list, worn.id, 0);
            }
        }
    }
}

/// An employee as the load's list regeneration sees it: the profession
/// word (its low byte is the class) and the record with its own list.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StaffLists {
    pub class: u32,
    pub refs: EmployeeRefs,
}

fn dedup_entries(list: &mut Vec<ItemEntry>) {
    let mut raw: Vec<[u8; 12]> = list
        .iter()
        .map(|entry| {
            let mut bytes = [0u8; 12];
            bytes[0..4].copy_from_slice(&entry.id.to_le_bytes());
            bytes[4..8].copy_from_slice(&entry.flags.to_le_bytes());
            bytes[8..12].copy_from_slice(&entry.extra.to_le_bytes());
            bytes
        })
        .collect();
    exa_formats::arena_save::remove_duplicate_item_ids(&mut raw);
    let word =
        |b: &[u8; 12], at: usize| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
    *list = raw
        .iter()
        .map(|b| ItemEntry {
            id: word(b, 0),
            flags: word(b, 4),
            extra: word(b, 8),
        })
        .collect();
}

fn dedup_arsenal_box(entries: &mut Vec<ArsenalEntry>) {
    let mut list: Vec<ItemEntry> = entries
        .iter()
        .map(|entry| ItemEntry {
            id: entry.item,
            flags: entry.flags,
            extra: u32::from(entry.first) | (u32::from(entry.second) << 16),
        })
        .collect();
    dedup_entries(&mut list);
    *entries = list
        .iter()
        .map(|entry| ArsenalEntry {
            item: entry.id,
            flags: entry.flags,
            first: entry.extra as u16,
            second: (entry.extra >> 16) as u16,
        })
        .collect();
}

/// The load's list regeneration over the arena's company, in the
/// original's order:
///
/// 1. each employee in staff order: a [`CLASS_WITHOUT_LIST`] employee's
///    own list is emptied; any other has its record's loadout items merged
///    into it ([`merge_loadout_items`]), its duplicates removed
///    ([`exa_formats::arena_save::remove_duplicate_item_ids`]) and every
///    entry put on its arsenal shelf with the saved-item flag
///    ([`add_classified`]);
/// 2. the manager's own list (the company stash's twelve-byte list
///    [guess: the field is the manager record's list beside the stash])
///    the same way from the manager's record;
/// 3. every box of every shelf (shelf by shelf, box by box) has its
///    duplicates removed.
///
/// `kind_of` is the object database's classification of an id; an id the
/// database does not know is not shelved. Returns how many entries were
/// shelved.
pub fn regenerate_lists(
    manager: &CharacterRecord,
    manager_items: &mut Vec<ItemEntry>,
    staff: &mut [StaffLists],
    arsenal: &mut Arsenal,
    mut kind_of: impl FnMut(u32) -> ItemKind,
) -> usize {
    let mut shelved = 0;
    let mut shelve = |list: &[ItemEntry], arsenal: &mut Arsenal| {
        for entry in list {
            if add_classified(arsenal, entry.id, kind_of(entry.id)) {
                shelved += 1;
            }
        }
    };
    for employee in staff.iter_mut() {
        if employee.class & 0xFF == CLASS_WITHOUT_LIST {
            employee.refs.items.clear();
            continue;
        }
        merge_loadout_items(&employee.refs.record, &mut employee.refs.items);
        dedup_entries(&mut employee.refs.items);
        shelve(&employee.refs.items, arsenal);
    }
    merge_loadout_items(manager, manager_items);
    dedup_entries(manager_items);
    shelve(manager_items, arsenal);
    for shelf in arsenal.shelves.iter_mut() {
        for entries in shelf.iter_mut() {
            dedup_arsenal_box(entries);
        }
    }
    shelved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arsenal::{ARMOUR, CLOTHING, SHIELDS, WEAPONS, shelf_of};
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
    fn flushing_a_scene_character_follows_the_original_order() {
        let mut record = character([40, 41], &[50, 41, 52], &[([60, 0], &[70])]);
        record.hands[2] = ItemRef::new(42, 0);
        record.hands[3] = ItemRef::new(52, 0);
        record.worn = vec![ItemRef::new(45, 0)];
        let mut seen = Vec::new();
        let mut nodes = vec![40, 41, 99];
        let flushed = flush_scene_character(&mut record, 0x8000_0000 | 0x1234, &mut nodes, |id| {
            seen.push(id);
            id + 100
        });
        assert_eq!(flushed.register, Some(0x1234));
        assert_eq!(flushed.record_reference, Some(0x8000_0000 | 0x1234));
        assert_eq!(flushed.hands, [(40, 140), (41, 141)]);
        // The attached nodes with the first pair's saved ids take their new
        // ids; the third node is left alone.
        assert_eq!(nodes, vec![140, 141, 99]);
        assert_eq!(flushed.attached_nodes, 2);
        // Worn, the first hand pair, the second pair, then the carried
        // items; carried 41 and 52 reuse the hands' new ids, and the
        // loadouts are left alone.
        assert_eq!(seen, vec![45, 40, 41, 42, 52, 50]);
        assert_eq!(record.worn[0].id, 145);
        let carried: Vec<u32> = record.inventory.iter().map(|item| item.item).collect();
        assert_eq!(carried, vec![150, 141, 152]);
        assert_eq!(record.loadouts[0].hands[0].id, 60);
        assert_eq!(record.loadouts[0].worn[0].id, 70);
        // A reference without the top bit does not register and stores no
        // reference.
        let mut nodes = Vec::new();
        let unregistered = flush_scene_character(&mut record, 0x1234, &mut nodes, |id| id);
        assert_eq!(unregistered.register, None);
        assert_eq!(unregistered.record_reference, None);
        assert_eq!(unregistered.attached_nodes, 0);
    }

    #[test]
    fn regeneration_merges_loadouts_skips_eight_and_shelves_with_the_saved_flag() {
        let weapon = ItemKind {
            known: true,
            apparel_layer: None,
            weapon: true,
            shield: false,
        };
        let unknown = ItemKind::default();
        let manager = character([0, 0], &[], &[([11, 0], &[8, 12, 11])]);
        let mut manager_items = vec![ItemEntry {
            id: 12,
            flags: 3,
            extra: 0,
        }];
        let mut staff = vec![
            StaffLists {
                class: 0x104,
                refs: EmployeeRefs {
                    record: character([0, 0], &[], &[([21, 22], &[])]),
                    items: vec![ItemEntry {
                        id: 99,
                        flags: 0,
                        extra: 0,
                    }],
                },
            },
            StaffLists {
                class: 16,
                refs: EmployeeRefs {
                    record: character([0, 0], &[], &[([31, 0], &[32])]),
                    items: Vec::new(),
                },
            },
        ];
        let mut arsenal = Arsenal::default();
        arsenal.shelves[WEAPONS][2].push(entry(32, 0));
        let shelved = regenerate_lists(
            &manager,
            &mut manager_items,
            &mut staff,
            &mut arsenal,
            |id| {
                if id == 12 { unknown } else { weapon }
            },
        );
        // Class 4 empties its own list and shelves nothing.
        assert!(staff[0].refs.items.is_empty());
        // The second employee gets 31 and 32 appended with the list word.
        assert_eq!(
            staff[1].refs.items,
            vec![
                ItemEntry {
                    id: 31,
                    flags: 0,
                    extra: 0x0004_0028
                },
                ItemEntry {
                    id: 32,
                    flags: 0,
                    extra: 0x0004_0028
                },
            ]
        );
        // The manager keeps 12 first, adds 11 once; worn id 8 is skipped.
        assert_eq!(
            manager_items.iter().map(|e| e.id).collect::<Vec<_>>(),
            [12, 11]
        );
        // 31 and 11 are shelved; 32 is already on the shelf, 12 is unknown.
        assert_eq!(shelved, 2);
        let first_box: Vec<(u32, u32)> = arsenal.shelves[WEAPONS][0]
            .iter()
            .map(|e| (e.item, e.flags))
            .collect();
        assert_eq!(first_box, [(31, SAVED_ITEM), (11, SAVED_ITEM)]);
    }

    #[test]
    fn regeneration_dedups_special_ids_in_every_arsenal_box() {
        let mut arsenal = Arsenal::default();
        let special = 0x8000_0005;
        arsenal.shelves[ARMOUR][3] = vec![
            entry(special, 1),
            entry(7, 0),
            entry(special, 2),
            entry(7, 0),
        ];
        regenerate_lists(
            &CharacterRecord::new(),
            &mut Vec::new(),
            &mut [],
            &mut arsenal,
            |_| ItemKind::default(),
        );
        // The special id's later copy goes; a plain id's repeat stays.
        assert_eq!(
            arsenal.shelves[ARMOUR][3]
                .iter()
                .map(|e| (e.item, e.flags))
                .collect::<Vec<_>>(),
            [(special, 1), (7, 0), (7, 0)]
        );
    }

    #[test]
    fn a_unique_append_ignores_an_id_already_listed() {
        let mut list = Vec::new();
        assert!(add_unique_entry(&mut list, 5, 0));
        assert!(!add_unique_entry(&mut list, 5, 9));
        assert_eq!(list.len(), 1);
    }

    /// An object record payload with the given sections. The body is long
    /// enough to hold the armour type byte; weapon and shield sections are
    /// short enough that their own parsers fail, which is the point: the
    /// classifier tests the section's presence, not its contents.
    fn object_payload(body: Option<u8>, weapon: bool, shield: bool) -> Vec<u8> {
        let mut out = Vec::new();
        let mut push = |tag: u32, data: &[u8]| {
            out.extend_from_slice(&tag.to_le_bytes());
            out.extend_from_slice(&((data.len() + 8) as u32).to_le_bytes());
            out.extend_from_slice(data);
        };
        if let Some(armour_type) = body {
            let mut data = vec![0u8; 0x1AA];
            data[0x1A8] = armour_type;
            push(0x1000, &data);
        }
        if weapon {
            push(0x4000, &[0u8; 4]);
        }
        if shield {
            push(0x8000, &[0u8; 4]);
        }
        out
    }

    fn object_item(
        id: u32,
        body: Option<u8>,
        weapon: bool,
        shield: bool,
    ) -> exa_formats::objdb::Item {
        exa_formats::objdb::Item::parse(id, &object_payload(body, weapon, shield)).unwrap()
    }

    #[test]
    fn the_object_record_classifies_like_the_load_pass() {
        // A body is apparel: armour types 3 through 6 go on the armour
        // shelf, every other type on the clothing shelf, and the body wins
        // over a weapon section.
        assert_eq!(
            shelf_of(ItemKind::of_item(&object_item(1, Some(3), false, false))),
            Some(ARMOUR)
        );
        assert_eq!(
            shelf_of(ItemKind::of_item(&object_item(2, Some(6), true, false))),
            Some(ARMOUR)
        );
        assert_eq!(
            shelf_of(ItemKind::of_item(&object_item(3, Some(2), false, false))),
            Some(CLOTHING)
        );
        assert_eq!(
            shelf_of(ItemKind::of_item(&object_item(4, Some(7), false, true))),
            Some(CLOTHING)
        );
        // Without a body the weapon section is checked first, then the
        // shield section; neither leaves the item unclassified.
        assert_eq!(
            shelf_of(ItemKind::of_item(&object_item(5, None, true, true))),
            Some(WEAPONS)
        );
        assert_eq!(
            shelf_of(ItemKind::of_item(&object_item(6, None, false, true))),
            Some(SHIELDS)
        );
        assert_eq!(
            shelf_of(ItemKind::of_item(&object_item(7, None, false, false))),
            None
        );
    }

    #[test]
    fn the_item_lookup_shelves_known_records_and_skips_unknown_ids() {
        let items = [
            object_item(1, Some(3), false, false),
            object_item(2, None, true, false),
        ];
        let kinds = kinds_from_items(&items);
        let mut arsenal = Arsenal::default();
        let records: Vec<[u8; 12]> = [1u32, 2, 99]
            .iter()
            .map(|&id| {
                let mut record = [0u8; 12];
                record[..4].copy_from_slice(&id.to_le_bytes());
                record
            })
            .collect();
        let added = reclassify_records(&mut arsenal, &records, |id| {
            kinds.get(&id).copied().unwrap_or_default()
        });
        assert_eq!(added, 2);
        assert_eq!(arsenal.shelves[ARMOUR][0][0].item, 1);
        assert_eq!(arsenal.shelves[WEAPONS][0][0].item, 2);
        assert_eq!(arsenal.shelves[SHIELDS][0].len(), 0);
    }
}
