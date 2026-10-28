//! The practice and arena fighters' kits across a match load.
//!
//! A saved fighter record names its items through the character's **own**
//! item database: an id whose top two bits are `10` is resolved by the
//! character-data helpers, first into a temporary database ([`prepare_kit`])
//! and then into the running scene's items ([`equip_kit`]). Both passes walk
//! the record in the same order — the first hand pair, the second hand pair,
//! the worn items, the carried items — and every pass leaves non-own ids
//! alone.
//!
//! Two quirks of the original are reproduced: the preparation pass records
//! the *second* hand pair's resolved ids from the *first* pair (the values
//! it wrote are dead, but a carried item that names the second pair's saved
//! id takes the first pair's resolved id), and the equip pass writes a hand
//! pair's new id into every attached scene node and the hand's own node that
//! still holds the saved id, without stopping at the first.
//!
//! Evidence: the two fighter-kit passes and the practice/arena preparation,
//! read from the instructions (`docs/formats/SAVES.md`, Practice).

use exa_formats::characters::CharacterRecord;

/// Whether an id names an item of the character's own database: the top two
/// bits are `10`.
pub fn own_item(id: u32) -> bool {
    id & 0xc000_0000 == 0x8000_0000
}

/// A hand pair's saved and resolved ids, in hand order.
pub type HandPair = [(u32, u32); 2];

/// What [`prepare_kit`] resolved and reused.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KitPreparation {
    /// The first hand pair's (saved, resolved) ids.
    pub hands: HandPair,
    /// The second hand pair's (saved, resolved) ids as saved in the record;
    /// the replacement the original stores along with them is the first
    /// pair's, see [`KitPreparation::carried_replacement`].
    pub second_hands: HandPair,
    /// The worn items that were own references and were resolved.
    pub worn_resolved: usize,
    /// The carried items that were own references and were resolved.
    pub carried_resolved: usize,
    /// The carried items that were handed a resolved hand id instead of
    /// being resolved themselves.
    pub carried_reused: usize,
}

impl KitPreparation {
    /// The id a carried item takes when it names the saved id of hand
    /// `index` (0 and 1 the first pair, 2 and 3 the second). The second
    /// pair's entries return the **first** pair's resolved id, as the
    /// original stores them.
    pub fn carried_replacement(&self, index: usize) -> Option<u32> {
        match index {
            0 => Some(self.hands[0].1),
            1 => Some(self.hands[1].1),
            2 => Some(self.hands[0].1),
            3 => Some(self.hands[1].1),
            _ => None,
        }
    }
}

/// The preparation pass: resolve every own item reference into the
/// character's temporary item database, in the original's order.
///
/// The first hand pair is resolved and its (saved, resolved) ids kept; then
/// the second pair (whose kept replacements repeat the first pair's); then
/// each worn item; then each carried item, which first searches the four
/// saved hand ids and takes the corresponding replacement when it finds one
/// (only own ids are searched) and is otherwise resolved.
pub fn prepare_kit(
    record: &mut CharacterRecord,
    mut resolve: impl FnMut(u32) -> u32,
) -> KitPreparation {
    let mut out = KitPreparation::default();
    for hand in 0..2 {
        let saved = record.hands[hand].id;
        out.hands[hand] = (
            saved,
            if own_item(saved) {
                let resolved = resolve(saved);
                record.hands[hand].id = resolved;
                resolved
            } else {
                saved
            },
        );
    }
    for hand in 0..2 {
        let slot = 2 + hand;
        let saved = record.hands[slot].id;
        out.second_hands[hand] = (
            saved,
            if own_item(saved) {
                let resolved = resolve(saved);
                record.hands[slot].id = resolved;
                resolved
            } else {
                saved
            },
        );
    }
    for worn in &mut record.worn {
        if own_item(worn.id) {
            worn.id = resolve(worn.id);
            out.worn_resolved += 1;
        }
    }
    for carried in &mut record.inventory {
        if !own_item(carried.item) {
            continue;
        }
        let saved = carried.item;
        let saved_hands = [
            out.hands[0].0,
            out.hands[1].0,
            out.second_hands[0].0,
            out.second_hands[1].0,
        ];
        if let Some(index) = saved_hands.iter().position(|&id| id == saved)
            && let Some(replacement) = out.carried_replacement(index)
        {
            carried.item = replacement;
            out.carried_reused += 1;
            continue;
        }
        carried.item = resolve(carried.item);
        out.carried_resolved += 1;
    }
    out
}

/// What [`equip_kit`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KitEquip {
    /// Each hand's saved and running id, in hand order.
    pub hands: [(u32, u32); 4],
    /// The attached scene nodes that took a new hand id.
    pub nodes_updated: usize,
    /// Whether the character's own hand node took a new id.
    pub hand_node_updated: bool,
    /// The worn items that were own references and were instantiated.
    pub worn_instantiated: usize,
    /// The carried items that were given a running hand id.
    pub carried_reused: usize,
    /// The carried items that were own references and were instantiated.
    pub carried_instantiated: usize,
    /// The temporary database and string library were freed and the item GUI
    /// refreshed (the original's rebuild flag).
    pub temp_freed: bool,
}

/// The equip pass: instantiate every own item into the running scene.
///
/// The saved hand ids are read first (the search keys for the carried
/// items). The first pair is instantiated and every attached scene node and
/// the character's hand node whose item id equals a saved hand id takes the
/// new id; the second pair is instantiated without touching nodes; the worn
/// items are instantiated; each carried item that is an own reference takes
/// the running id of the hand whose saved id it equals (first pair first,
/// then the second pair) or is instantiated. The temporary database and
/// string library are then freed and the rebuild flag is set for the next
/// preparation pass.
pub fn equip_kit(
    record: &mut CharacterRecord,
    attached_nodes: &mut [u32],
    hand_node: Option<&mut u32>,
    mut instantiate: impl FnMut(u32) -> u32,
) -> KitEquip {
    let mut out = KitEquip::default();
    let mut hand_node = hand_node;
    let saved: [u32; 4] = std::array::from_fn(|slot| record.hands[slot].id);
    for (hand, &saved_id) in saved.iter().enumerate().take(2) {
        out.hands[hand] = (saved_id, saved_id);
        if own_item(saved_id) {
            let new = instantiate(saved_id);
            record.hands[hand].id = new;
            out.hands[hand] = (saved_id, new);
            for node in attached_nodes.iter_mut() {
                if *node == saved_id {
                    *node = new;
                    out.nodes_updated += 1;
                }
            }
            if let Some(node) = hand_node.as_deref_mut()
                && *node == saved_id
            {
                *node = new;
                out.hand_node_updated = true;
            }
        }
    }
    for hand in 0..2 {
        let slot = 2 + hand;
        let saved_id = saved[slot];
        out.hands[slot] = (saved_id, saved_id);
        if own_item(saved_id) {
            let new = instantiate(saved_id);
            record.hands[slot].id = new;
            out.hands[slot] = (saved_id, new);
        }
    }
    for worn in &mut record.worn {
        if own_item(worn.id) {
            worn.id = instantiate(worn.id);
            out.worn_instantiated += 1;
        }
    }
    for carried in &mut record.inventory {
        if !own_item(carried.item) {
            continue;
        }
        let saved_id = carried.item;
        if let Some(index) = saved.iter().position(|&id| id == saved_id) {
            carried.item = out.hands[index].1;
            out.carried_reused += 1;
            continue;
        }
        carried.item = instantiate(carried.item);
        out.carried_instantiated += 1;
    }
    out.temp_freed = true;
    out
}

/// One step of a practice/arena combatant's scene preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatantSceneStep {
    /// The GUI's current form is closed (no argument, the original's zero).
    CloseForm,
    /// The item GUI's panels are reset.
    ResetItemGui,
    /// The prepared level's locale is put in place and its id looked up.
    Locale,
    /// The portal node goes to −1.
    ClearPortal,
    /// The kd-tree reservation.
    ReserveTrees,
    /// The GUI reset (task list, tips, game functions, item GUI, the word).
    ResetGui,
    /// The scene-content helpers' reset.
    ResetContent,
    /// The archive's `<locale>.rfc` is opened and read.
    OpenArchive,
    /// The level is imported with the saved locale for its terrain probe.
    Import,
    /// A successful import lets the arena task run its post-load hook when
    /// its flag bit 0 is clear.
    ArenaHook,
}

/// The combatant-scene preparation: the steps in order, and whether the
/// arena hook runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatantScene {
    pub steps: Vec<CombatantSceneStep>,
    pub arena_hook: bool,
}

/// The practice/arena scene preparation. The archive is opened **after** the
/// GUI and the trees are reset; a failed import skips the arena hook. The
/// hook runs only when an arena task exists and its flags' bit 0 is clear.
pub fn combatant_scene(
    archive_found: bool,
    imported: bool,
    arena_present: bool,
    arena_flags: u32,
) -> CombatantScene {
    let mut steps = vec![
        CombatantSceneStep::CloseForm,
        CombatantSceneStep::ResetItemGui,
        CombatantSceneStep::Locale,
        CombatantSceneStep::ClearPortal,
        CombatantSceneStep::ReserveTrees,
        CombatantSceneStep::ResetGui,
        CombatantSceneStep::ResetContent,
    ];
    let arena_hook = imported && arena_present && arena_flags & 1 == 0;
    if archive_found {
        steps.push(CombatantSceneStep::OpenArchive);
        steps.push(CombatantSceneStep::Import);
        if arena_hook {
            steps.push(CombatantSceneStep::ArenaHook);
        }
    }
    CombatantScene { steps, arena_hook }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::characters::{Carried, ItemRef};

    const OWN: u32 = 0x8000_0000;

    fn record(hands: [u32; 4], worn: &[u32], carried: &[u32]) -> CharacterRecord {
        let mut record = CharacterRecord::new();
        for (slot, id) in hands.into_iter().enumerate() {
            record.hands[slot] = ItemRef::new(id, 0);
        }
        record.worn = worn.iter().map(|&id| ItemRef::new(id, 0)).collect();
        record.inventory = carried
            .iter()
            .map(|&item| Carried {
                item,
                flags: 0,
                extra: 0,
            })
            .collect();
        record
    }

    fn carried_ids(record: &CharacterRecord) -> Vec<u32> {
        record.inventory.iter().map(|item| item.item).collect()
    }

    #[test]
    fn only_top_bits_ten_are_own_items() {
        assert!(own_item(OWN));
        assert!(own_item(OWN | 0x1234));
        assert!(!own_item(0));
        assert!(!own_item(0x4000_0001));
        assert!(!own_item(0xc000_0001));
    }

    #[test]
    fn preparation_resolves_hands_worn_and_carried_in_order() {
        let mut record = record([OWN | 1, 7, OWN | 2, 9], &[OWN | 3, 8], &[OWN | 4, 6]);
        let mut seen = Vec::new();
        let prep = prepare_kit(&mut record, |id| {
            seen.push(id);
            id + 100
        });
        // Hands, then worn, then carried; non-own ids are not resolved.
        assert_eq!(seen, vec![OWN | 1, OWN | 2, OWN | 3, OWN | 4]);
        assert_eq!(prep.hands, [(OWN | 1, (OWN | 1) + 100), (7, 7)]);
        assert_eq!(prep.second_hands, [(OWN | 2, (OWN | 2) + 100), (9, 9)]);
        assert_eq!(prep.worn_resolved, 1);
        assert_eq!(prep.carried_resolved, 1);
        assert_eq!(record.hands[0].id, (OWN | 1) + 100);
        assert_eq!(record.hands[1].id, 7);
        assert_eq!(record.worn[0].id, (OWN | 3) + 100);
        assert_eq!(record.worn[1].id, 8);
        assert_eq!(carried_ids(&record), vec![(OWN | 4) + 100, 6]);
    }

    #[test]
    fn a_carried_item_naming_a_hand_takes_the_hands_second_pairs_included() {
        let mut record = record(
            [OWN | 1, OWN | 2, OWN | 3, OWN | 4],
            &[],
            &[OWN | 1, OWN | 2, OWN | 3, OWN | 4, OWN | 5],
        );
        let prep = prepare_kit(&mut record, |id| id + 1000);
        assert_eq!(prep.carried_reused, 4);
        // A carried item that names the second pair takes the *first*
        // pair's resolved id (the original's stored values).
        assert_eq!(
            carried_ids(&record),
            vec![
                (OWN | 1) + 1000,
                (OWN | 2) + 1000,
                (OWN | 1) + 1000,
                (OWN | 2) + 1000,
                (OWN | 5) + 1000,
            ]
        );
        assert_eq!(prep.carried_resolved, 1);
    }

    #[test]
    fn equipping_resolves_and_hands_scene_nodes_the_running_ids() {
        let mut record = record(
            [OWN | 1, OWN | 2, 7, OWN | 4],
            &[OWN | 3],
            &[OWN | 1, 7, OWN | 4, 9],
        );
        let mut nodes = vec![OWN | 1, OWN | 2];
        let mut hand_node = OWN | 1;
        let mut seen = Vec::new();
        let equip = equip_kit(&mut record, &mut nodes, Some(&mut hand_node), |id| {
            seen.push(id);
            id + 10
        });
        // First pair, second pair, worn, then the carried item that is not
        // a hand and is not the second pair's saved id search.
        assert_eq!(seen, vec![OWN | 1, OWN | 2, OWN | 4, OWN | 3]);
        assert_eq!(equip.hands[0], (OWN | 1, (OWN | 1) + 10));
        assert_eq!(equip.hands[1], (OWN | 2, (OWN | 2) + 10));
        assert_eq!(equip.hands[2], (7, 7));
        assert_eq!(equip.hands[3], (OWN | 4, (OWN | 4) + 10));
        // Each attached node takes the running id of the hand whose saved id
        // it holds, and the character's own hand node does too.
        assert_eq!(nodes, vec![(OWN | 1) + 10, (OWN | 2) + 10]);
        assert_eq!(equip.nodes_updated, 2);
        assert_eq!(hand_node, (OWN | 1) + 10);
        assert!(equip.hand_node_updated);
        assert_eq!(record.worn[0].id, (OWN | 3) + 10);
        assert_eq!(equip.worn_instantiated, 1);
        // Carried: own 1 takes hand 0's running id, 7 stays, own 4 takes
        // hand 3's running id (the hand ids are searched before
        // instantiating), 9 stays.
        assert_eq!(
            carried_ids(&record),
            vec![(OWN | 1) + 10, 7, (OWN | 4) + 10, 9]
        );
        assert_eq!(equip.carried_reused, 2);
        assert_eq!(equip.carried_instantiated, 0);
        assert!(equip.temp_freed);
    }

    #[test]
    fn a_carried_item_that_is_not_a_hand_is_instantiated() {
        let mut record = record([OWN | 1, 0, 0, 0], &[], &[OWN | 5]);
        let equip = equip_kit(&mut record, &mut [], None, |id| id + 1);
        assert_eq!(equip.carried_instantiated, 1);
        assert_eq!(carried_ids(&record), vec![(OWN | 5) + 1]);
    }

    #[test]
    fn the_combatant_scene_runs_the_resets_before_the_archive() {
        let scene = combatant_scene(true, true, true, 0);
        assert_eq!(
            scene.steps,
            vec![
                CombatantSceneStep::CloseForm,
                CombatantSceneStep::ResetItemGui,
                CombatantSceneStep::Locale,
                CombatantSceneStep::ClearPortal,
                CombatantSceneStep::ReserveTrees,
                CombatantSceneStep::ResetGui,
                CombatantSceneStep::ResetContent,
                CombatantSceneStep::OpenArchive,
                CombatantSceneStep::Import,
                CombatantSceneStep::ArenaHook,
            ]
        );
        assert!(scene.arena_hook);
        // The hook needs bit 0 clear; a failed import or a missing archive
        // skips it.
        assert!(!combatant_scene(true, true, true, 1).arena_hook);
        assert!(!combatant_scene(true, true, false, 0).arena_hook);
        let failed = combatant_scene(true, false, true, 0);
        assert!(!failed.arena_hook);
        assert_eq!(failed.steps.last(), Some(&CombatantSceneStep::Import));
        let missing = combatant_scene(false, false, true, 0);
        assert_eq!(
            missing.steps.last(),
            Some(&CombatantSceneStep::ResetContent)
        );
    }
}
