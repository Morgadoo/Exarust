//! The live database's tier swaps and the save unit's promotion passes
//! over a character's items: a level load replacing the local tier, a save
//! load replacing the world tier, the story's pre-save and level-change
//! promotion of the player's carried items, the arena's promotion of a
//! whole record, and a container's children written back into its record.
//! Evidence: `docs/formats/OTHER.md` (Object database, The live tiers).

use super::record::{section_range, set_u32};
use super::{CONTAINER, ObjectDb, Store, Tier};
use crate::characters::CharacterRecord;
use crate::containers::Child;
use crate::objstrings::Library;

/// The hand ids a carried-items promotion changed: each pair is the old
/// id and the id it became, for the out pair (hands 0 and 1) and the
/// stowed pair (hands 2 and 3). The original rewrites the held bodies' item
/// ids through the first pair.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HandPromotion {
    pub out: [(u32, u32); 2],
    pub stowed: [(u32, u32); 2],
}

impl HandPromotion {
    /// The id a held body's item becomes: the first out hand whose old id
    /// it is gives its new id, anything else stays.
    pub fn held(&self, id: u32) -> u32 {
        self.out
            .iter()
            .find(|(old, _)| *old == id)
            .map_or(id, |&(_, new)| new)
    }
}

impl ObjectDb {
    /// Replaces the local tier with a level's own database, as the level
    /// reader's local-database chunk does: the `0xDBDB` records become the
    /// tier as stored (table, flags and blob), the `0xDBCD` library the
    /// tier's own. A level without a database leaves an empty tier and an
    /// empty library. The world tier, the base tier and the remap tables
    /// are untouched.
    pub fn load_local(&mut self, records: Store, strings: Library) {
        self.local = records;
        self.local_strings = strings;
    }

    /// Replaces the world tier with a save's `#worldobjdb` export (its
    /// first chunk the records, its second the world tier's own library),
    /// then migrates every record from its template, as the save load
    /// does. `None` (no section in the save) clears the tier and its
    /// library instead. Returns how many records the migration visited (a
    /// record whose template does not resolve is visited and left alone).
    pub fn import_world(&mut self, export: Option<(Store, Library)>) -> usize {
        match export {
            Some((records, strings)) => {
                self.world = records;
                self.world_strings = strings;
                self.migrate_tier(Tier::World)
            }
            None => {
                self.clear_world();
                0
            }
        }
    }

    /// Empties the world tier and its library, as a new game and a save
    /// without a world database do.
    pub fn clear_world(&mut self) {
        self.world.clear();
        self.world_strings.clear();
    }

    /// The world tier as a save exports it: the tier's writer and its own
    /// library's writer, the payloads of the `#worldobjdb` section's two
    /// chunks. `None` when the tier holds no record (the save then writes
    /// no section).
    pub fn export_world(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        (!self.world.is_empty()).then(|| (self.world.write(), self.world_strings.write()))
    }

    /// The story's promotion of a character's carried items, run before a
    /// save and before a level change: the worn list, the out pair (hands
    /// 0 and 1), the stowed pair (hands 2 and 3), then the pack, where an
    /// entry that was a hand's old id takes that hand's new id (the out
    /// pair checked first) and any other entry is promoted itself. Local
    /// objects move to the world tier; other ids come back as they are
    /// ([`Self::promote`]).
    pub fn promote_carried(&mut self, record: &mut CharacterRecord) -> HandPromotion {
        for worn in &mut record.worn {
            worn.id = self.promote(worn.id);
        }
        let mut hands = HandPromotion::default();
        for slot in 0..2 {
            let old = record.hands[slot].id;
            record.hands[slot].id = self.promote(old);
            hands.out[slot] = (old, record.hands[slot].id);
        }
        for slot in 0..2 {
            let old = record.hands[2 + slot].id;
            record.hands[2 + slot].id = self.promote(old);
            hands.stowed[slot] = (old, record.hands[2 + slot].id);
        }
        for carried in &mut record.inventory {
            let id = carried.item;
            carried.item = if let Some(&(_, new)) = hands.out.iter().find(|(old, _)| *old == id) {
                new
            } else if let Some(&(_, new)) = hands.stowed.iter().find(|(old, _)| *old == id) {
                new
            } else {
                self.promote(id)
            };
        }
        hands
    }

    /// The arena's promotion of a whole record, as its save runs it for
    /// the manager and every employee: the four hands, the worn list, each
    /// loadout's four hands and worn list, then every pack entry promoted
    /// on its own (no hand matching). The save turns the id remap table on
    /// around it, so an id promoted twice finds its world id there.
    pub fn promote_record(&mut self, record: &mut CharacterRecord) {
        for hand in &mut record.hands {
            hand.id = self.promote(hand.id);
        }
        for worn in &mut record.worn {
            worn.id = self.promote(worn.id);
        }
        for loadout in &mut record.loadouts {
            for hand in &mut loadout.hands {
                hand.id = self.promote(hand.id);
            }
            for worn in &mut loadout.worn {
                worn.id = self.promote(worn.id);
            }
        }
        for carried in &mut record.inventory {
            carried.item = self.promote(carried.item);
        }
    }

    /// Writes a container's children back into the container section of
    /// the record `id` names: the count at body `0x14` and the 12-byte
    /// entries from `0x18` (id, flags, x and y). The section is resized to
    /// hold them and its size word written [Exarust's: the popup's write
    /// path is not traced; the section's other bytes stay]. A missing
    /// record or section is left alone.
    pub fn set_container_children(&mut self, id: u32, children: &[Child]) -> bool {
        let tier = Tier::of(id);
        let Some(at) = self.store(tier).find_index(Tier::index(id)) else {
            return false;
        };
        let Some(mut payload) = self
            .store(tier)
            .view(at)
            .map(|record| record.payload().to_vec())
        else {
            return false;
        };
        let Some(range) = section_range(&payload, CONTAINER) else {
            return false;
        };
        if range.len() < 0x18 {
            return false;
        }
        let body_len = 0x18 + children.len() * crate::containers::CHILD_LEN;
        let Some(start) = super::resize_section(&mut payload, CONTAINER, body_len as u32) else {
            return false;
        };
        set_u32(&mut payload, start + 4, body_len as u32 + 8);
        let body = start + 8;
        set_u32(&mut payload, body + 0x14, children.len() as u32);
        for (i, child) in children.iter().enumerate() {
            let at = body + 0x18 + i * crate::containers::CHILD_LEN;
            set_u32(&mut payload, at, child.id);
            set_u32(&mut payload, at + 4, child.flags);
            payload[at + 8..at + 10].copy_from_slice(&child.x.to_le_bytes());
            payload[at + 10..at + 12].copy_from_slice(&child.y.to_le_bytes());
        }
        self.store_mut(tier).store(at, &payload);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::record::test_util::*;
    use super::*;
    use crate::characters::{Carried, ItemRef, Loadout};
    use crate::objdb::{ID_MASK, LOCAL, MODEL, NAME, WORLD};

    fn put(store: &mut Store, id: u32, flags: u32, p: &[u8]) {
        let (_, at) = store.insert(id);
        store.set_flags(at, flags);
        store.store(at, p);
    }

    fn item(name: &str) -> Vec<u8> {
        payload(&[(NAME, header(name, 0, 0)), (MODEL, model(name, 0, 0, 0))])
    }

    /// A live database whose local tier holds records 1–6.
    fn live() -> ObjectDb {
        let mut db = ObjectDb::default();
        for id in 1..=6 {
            put(&mut db.local, id, NAME | MODEL, &item(&format!("Item{id}")));
        }
        db
    }

    fn name(db: &ObjectDb, id: u32) -> String {
        db.find(id).unwrap().header().unwrap().name
    }

    #[test]
    fn a_level_load_replaces_the_local_tier_and_leaves_the_world() {
        let mut db = live();
        put(&mut db.world, 1, NAME, &item("Kept"));
        let mut level = Store::default();
        put(&mut level, 9, NAME | MODEL, &item("Level9"));
        let mut strings = Library::default();
        strings.add(b"level text");
        db.load_local(level, strings);
        assert_eq!(db.local.len(), 1);
        assert_eq!(name(&db, LOCAL | 9), "Level9");
        assert!(db.find(LOCAL | 1).is_none());
        assert_eq!(db.local_strings.bytes(1), Some(&b"level text"[..]));
        assert_eq!(name(&db, WORLD | 1), "Kept");
        // A new local copy now takes the first id the level leaves free,
        // not the level's own record 1's.
        assert_eq!(db.first_free_id(Tier::Local), 1);
        db.load_local(Store::default(), Library::default());
        assert!(db.local.is_empty());
    }

    #[test]
    fn the_carried_items_go_to_the_world_with_hand_repeats_following() {
        let mut db = live();
        let mut record = CharacterRecord {
            hands: [
                ItemRef::new(LOCAL | 1, 0x100_0000),
                ItemRef::new(7, 0),
                ItemRef::new(LOCAL | 2, 0),
                ItemRef::NONE,
            ],
            worn: vec![ItemRef::new(LOCAL | 3, 0x10), ItemRef::new(8, 0)],
            inventory: vec![
                Carried {
                    item: LOCAL | 1,
                    ..Carried::default()
                },
                Carried {
                    item: LOCAL | 2,
                    ..Carried::default()
                },
                Carried {
                    item: LOCAL | 4,
                    ..Carried::default()
                },
                Carried {
                    item: 9,
                    ..Carried::default()
                },
            ],
            ..CharacterRecord::default()
        };
        let hands = db.promote_carried(&mut record);
        // The worn list first, then the out pair, the stowed pair and the
        // pack: world ids in that order.
        assert_eq!(record.worn[0], ItemRef::new(WORLD | 1, 0x10));
        assert_eq!(record.worn[1].id, 8);
        assert_eq!(record.hands[0], ItemRef::new(WORLD | 2, 0x100_0000));
        assert_eq!(record.hands[1].id, 7);
        assert_eq!(record.hands[2].id, WORLD | 3);
        assert_eq!(record.hands[3].id, 0);
        let pack: Vec<u32> = record.inventory.iter().map(|c| c.item).collect();
        assert_eq!(pack, [WORLD | 2, WORLD | 3, WORLD | 4, 9]);
        assert_eq!(hands.out[0], (LOCAL | 1, WORLD | 2));
        assert_eq!(hands.held(LOCAL | 1), WORLD | 2);
        assert_eq!(hands.held(LOCAL | 2), LOCAL | 2);
        assert_eq!(hands.stowed[0], (LOCAL | 2, WORLD | 3));
        // The records moved with their bytes; what stayed local is the rest.
        assert_eq!(name(&db, WORLD | 2), "Item1");
        assert_eq!(name(&db, WORLD | 4), "Item4");
        let left: Vec<u32> = db.local.entries().iter().map(|e| e.id).collect();
        assert_eq!(left, [5, 6]);
        // Already-world ids come back unchanged on a second pass.
        let again = db.promote_carried(&mut record.clone());
        assert_eq!(again.out[0], (WORLD | 2, WORLD | 2));
    }

    #[test]
    fn the_arena_promotes_every_list_of_a_record() {
        let mut db = live();
        let mut record = CharacterRecord {
            hands: [
                ItemRef::new(LOCAL | 1, 0),
                ItemRef::NONE,
                ItemRef::NONE,
                ItemRef::NONE,
            ],
            worn: vec![ItemRef::new(LOCAL | 2, 0)],
            loadouts: vec![Loadout {
                word: 0,
                hands: [
                    ItemRef::new(LOCAL | 3, 0),
                    ItemRef::NONE,
                    ItemRef::NONE,
                    ItemRef::NONE,
                ],
                worn: vec![ItemRef::new(LOCAL | 4, 0)],
            }],
            inventory: vec![Carried {
                item: LOCAL | 1,
                ..Carried::default()
            }],
            ..CharacterRecord::default()
        };
        let mut unmapped = record.clone();
        db.clone().promote_record(&mut unmapped);
        // Without the id remap table the pack's repeat of the promoted hand
        // is gone from the local tier and promotes to nothing.
        assert_eq!(unmapped.inventory[0].item, 0);
        // The arena's save turns the table on first, so the repeat finds
        // the hand's world id.
        db.set_remaps(true);
        db.promote_record(&mut record);
        assert_eq!(record.hands[0].id, WORLD | 1);
        assert_eq!(record.worn[0].id, WORLD | 2);
        assert_eq!(record.loadouts[0].hands[0].id, WORLD | 3);
        assert_eq!(record.loadouts[0].worn[0].id, WORLD | 4);
        assert_eq!(record.inventory[0].item, WORLD | 1);
    }

    #[test]
    fn a_save_exports_the_world_tier_and_a_load_brings_it_back() {
        let mut db = live();
        assert_eq!(db.export_world(), None);
        let mut record = CharacterRecord {
            worn: vec![ItemRef::new(LOCAL | 5, 0)],
            ..CharacterRecord::default()
        };
        db.promote_carried(&mut record);
        db.world_strings.add(b"own");
        let (objects, strings) = db.export_world().unwrap();
        let mut other = ObjectDb::default();
        let migrated = other.import_world(Some((
            Store::parse(&objects).unwrap(),
            Library::parse(&strings).unwrap(),
        )));
        // Every record is visited; this one names no template here, so its
        // bytes stay.
        assert_eq!(migrated, 1);
        assert_eq!(name(&other, record.worn[0].id), "Item5");
        assert_eq!(other.world_strings.bytes(1), Some(&b"own"[..]));
        other.import_world(None);
        assert!(other.world.is_empty() && other.world_strings.is_empty());
        assert_eq!(record.worn[0].id & ID_MASK, 1);
    }

    #[test]
    fn a_containers_children_are_written_into_its_record() {
        let mut db = ObjectDb::default();
        let mut body = vec![0u8; 0x18 + 12];
        body[0] = 4;
        body[0x14] = 1;
        body[0x18] = 0x33;
        let p = payload(&[(NAME, header("Chest", 0, 0)), (CONTAINER, body)]);
        put(&mut db.local, 2, NAME | CONTAINER, &p);
        let children = [
            Child {
                id: LOCAL | 7,
                flags: 1,
                x: -3,
                y: 40,
            },
            Child {
                id: 12,
                flags: 0,
                x: 5,
                y: 6,
            },
        ];
        assert!(db.set_container_children(LOCAL | 2, &children));
        let item = db.find(LOCAL | 2).unwrap().item().unwrap();
        let container = item.container.unwrap();
        assert_eq!(container.width, 4);
        assert_eq!(container.children, children);
        assert_eq!(db.find(LOCAL | 2).unwrap().header().unwrap().name, "Chest");
        assert!(db.set_container_children(LOCAL | 2, &[]));
        let empty = db.find(LOCAL | 2).unwrap().item().unwrap();
        assert!(empty.container.unwrap().children.is_empty());
        assert!(!db.set_container_children(LOCAL | 3, &[]));
        assert!(!db.set_container_children(3, &[]));
    }
}
