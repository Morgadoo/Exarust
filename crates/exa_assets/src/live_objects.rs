//! The running game's object database: the original's three tiers in one
//! resource. The base tier is `objdb.rdb` with the shared `objstrings.rdb`,
//! read lazily the first time something asks; the local tier is the loaded
//! level's own database, replaced at each level load; the world tier holds
//! the objects promoted out of levels, filled by a save's load and exported
//! by its save. Every item system reads and writes records here, so a
//! kit's copy, a level's own record and a promoted record each keep their
//! own bytes under their own tagged id.
//!
//! Evidence: `docs/formats/OTHER.md` (Object database, The live tiers).

use bevy::prelude::*;
use exa_formats::objdb::{Item, ObjectDb, Store, Tier};
use exa_formats::objstrings::Library;
use exa_rpk::{GameData, PackKind};

/// How many in-place record writes the change log keeps; a cache further
/// behind than that rebuilds everything.
const LOG_LEN: usize = 256;

/// What a decoded cache must refresh since a revision it saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Changes {
    /// Nothing changed.
    None,
    /// Only these records' bytes changed (whole ids, any tier).
    Records(Vec<u32>),
    /// Records came or went (a tier swap, a copy, a promotion): every
    /// tagged record is to be read again.
    All,
}

/// The live object database and a log of its writes.
#[derive(Resource, Debug, Default)]
pub struct LiveObjects {
    db: ObjectDb,
    base_tried: bool,
    /// Bumped by every write, so the decoded caches built from the
    /// database (the item records the panels read, the item catalogue's
    /// cards) know when to refresh.
    revision: u64,
    /// The revision of the last write that may have added, moved or
    /// removed records.
    structure: u64,
    /// The in-place writes since then: the revision and the record's id.
    log: std::collections::VecDeque<(u64, u32)>,
}

impl LiveObjects {
    /// A live database over an already built one (tests, tools).
    pub fn of(db: ObjectDb) -> Self {
        Self {
            db,
            base_tried: true,
            revision: 1,
            structure: 1,
            log: Default::default(),
        }
    }

    /// Reads the base tier and the shared library from the game's resource
    /// pack the first time, as the original's lazy loader does on the base
    /// tier's first use (a missing `objstrings.rdb` leaves the library
    /// empty). Returns whether the base tier holds records.
    pub fn ensure_base(&mut self, game: &GameData) -> bool {
        if !self.base_tried {
            self.base_tried = true;
            let pack = game.pack(PackKind::Resource);
            if let Some(bytes) = pack.and_then(|pack| pack.get("objdb.rdb")) {
                let strings = pack
                    .and_then(|pack| pack.get("objstrings.rdb"))
                    .unwrap_or_default();
                match ObjectDb::parse(bytes, strings) {
                    Ok(parsed) => {
                        self.db.base = parsed.base;
                        self.db.strings = parsed.strings;
                        self.revision += 1;
                        info!(
                            "object database: base tier {} records, {} shared strings",
                            self.db.base.len(),
                            self.db.strings.len()
                        );
                    }
                    Err(error) => warn!("object database: objdb.rdb does not read: {error}"),
                }
            }
        }
        !self.db.base.is_empty()
    }

    /// The database, to read.
    pub fn db(&self) -> &ObjectDb {
        &self.db
    }

    /// The database, to write anything: counts as a change that may add or
    /// remove records.
    pub fn db_mut(&mut self) -> &mut ObjectDb {
        self.revision += 1;
        self.structure = self.revision;
        self.log.clear();
        &mut self.db
    }

    /// How many writes the database has seen.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// What changed after revision `seen`.
    pub fn changes_since(&self, seen: u64) -> Changes {
        if seen >= self.revision {
            return Changes::None;
        }
        if seen < self.structure {
            return Changes::All;
        }
        // Every write after the last structural one is an in-place write,
        // logged with consecutive revisions; an older one than the log's
        // first was trimmed.
        if self.log.front().is_none_or(|&(rev, _)| rev > seen + 1) {
            return Changes::All;
        }
        let mut ids: Vec<u32> = self
            .log
            .iter()
            .filter(|&&(rev, _)| rev > seen)
            .map(|&(_, id)| id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        Changes::Records(ids)
    }

    fn touched(&mut self, id: u32) {
        self.revision += 1;
        self.log.push_back((self.revision, id));
        while self.log.len() > LOG_LEN {
            self.log.pop_front();
        }
    }

    /// A kit's byte written into the record `id` names: apparel or weapon,
    /// dirt or wear. Returns whether the record and its section were there.
    pub fn write_condition(&mut self, id: u32, apparel: bool, dirt: bool, value: u8) -> bool {
        let written = self.db.write_condition_byte(id, apparel, dirt, value);
        if written {
            self.touched(id);
        }
        written
    }

    /// A whole payload stored as the record `id` names (a copy the item
    /// code rewrote), logged as an in-place write. Returns whether the
    /// record was there.
    pub fn set_payload(&mut self, id: u32, payload: &[u8]) -> bool {
        let written = self.db.set_payload(id, payload);
        if written {
            self.touched(id);
        }
        written
    }

    /// One byte of the record `id` names, at `offset` in the body of its
    /// first section with `tag` (a socket's charge, …), logged as an
    /// in-place write. Returns whether the record and its section were
    /// there.
    pub fn write_section_byte(&mut self, id: u32, tag: u32, offset: usize, value: u8) -> bool {
        let written = self.db.set_section_byte(id, tag, offset, value);
        if written {
            self.touched(id);
        }
        written
    }

    /// A container's children written back into the record `id` names.
    pub fn set_container_children(
        &mut self,
        id: u32,
        children: &[exa_formats::containers::Child],
    ) -> bool {
        let written = self.db.set_container_children(id, children);
        if written {
            self.touched(id);
        }
        written
    }

    /// The level's own database becomes the local tier, as the level
    /// reader's local-database chunk replaces it, and the whole tier is then
    /// migrated from its templates: every level import runs the importer's
    /// hook, which is the tier migration for a save's own section and, for
    /// the archive's file, the same migration before the container fill.
    pub fn load_level(&mut self, name: &str, records: Option<Store>, strings: Option<Library>) {
        let records = records.unwrap_or_default();
        let count = records.len();
        let last = records.entries().last().map_or(0, |entry| entry.id);
        let db = self.db_mut();
        db.load_local(records, strings.unwrap_or_default());
        let migrated = db.migrate_tier(Tier::Local);
        info!(
            "object database: level {name} local tier {count} records (ids 1..={last}, {migrated} migrated), next local id {:#x}",
            self.db.first_free_id(Tier::Local) | exa_formats::objdb::LOCAL
        );
    }

    /// A record decoded as an item, by its whole id (any tier).
    pub fn item(&self, id: u32) -> Option<Item> {
        let mut item = self.db.find(id)?.item().ok()?;
        item.id = id;
        Some(item)
    }

    /// Every record of the local and world tiers decoded as items, under
    /// their tagged ids.
    pub fn tagged_items(&self) -> Vec<Item> {
        [Tier::Local, Tier::World]
            .into_iter()
            .flat_map(|tier| {
                self.db.store(tier).records().filter_map(move |record| {
                    let mut item = record.item().ok()?;
                    item.id = record.id | tier.tag();
                    Some(item)
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::objdb::{LOCAL, WORLD};

    fn record(name: &str) -> Vec<u8> {
        let mut header = name.as_bytes().to_vec();
        header.resize(16, 0);
        header.extend([0; 8]);
        let mut out = 0x1000_0000u32.to_le_bytes().to_vec();
        out.extend((header.len() as u32 + 8).to_le_bytes());
        out.extend(header);
        out
    }

    #[test]
    fn a_level_load_replaces_only_the_local_tier_and_counts_as_a_write() {
        let mut db = ObjectDb::default();
        let (_, at) = db.world.insert(1);
        db.world.store(at, &record("World1"));
        let mut live = LiveObjects::of(db);
        let before = live.revision();
        let level = Store::from_records([(3u32, 0x1000_0000u32, record("Level3").as_slice())]);
        live.load_level("test", Some(level), None);
        assert_eq!(live.changes_since(before), Changes::All);
        assert_eq!(live.changes_since(live.revision()), Changes::None);
        assert_eq!(live.item(LOCAL | 3).unwrap().name, "Level3");
        assert_eq!(live.item(WORLD | 1).unwrap().name, "World1");
        let ids: Vec<u32> = live.tagged_items().iter().map(|item| item.id).collect();
        assert_eq!(ids, [LOCAL | 3, WORLD | 1]);
        // A byte written in place is logged by its id; a missing section
        // is no write.
        let seen = live.revision();
        assert!(!live.write_condition(LOCAL | 3, true, true, 1));
        assert_eq!(live.changes_since(seen), Changes::None);
        live.load_level("empty", None, None);
        assert!(live.item(LOCAL | 3).is_none());
        assert_eq!(live.db().first_free_id(Tier::Local), 1);
    }

    #[test]
    fn in_place_writes_are_logged_until_the_log_overflows() {
        let mut body = 0x1000u32.to_le_bytes().to_vec();
        body.extend((0x1C0u32 + 8).to_le_bytes());
        body.resize(8 + 0x1C0, 0);
        let mut db = ObjectDb::default();
        let (_, at) = db.base.insert(7);
        db.base.store(at, &body);
        let mut live = LiveObjects::of(db);
        let seen = live.revision();
        assert!(live.write_condition(7, true, true, 0x20));
        assert!(live.write_condition(7, true, false, 0x10));
        assert_eq!(live.changes_since(seen), Changes::Records(vec![7]));
        assert_eq!(
            live.db().stored_condition(7).unwrap().apparel_dirt,
            Some(0x20)
        );
        let later = live.revision();
        for _ in 0..LOG_LEN + 1 {
            live.write_condition(7, true, true, 1);
        }
        assert_eq!(live.changes_since(later), Changes::All);
        let recent = live.revision() - 2;
        assert_eq!(live.changes_since(recent), Changes::Records(vec![7]));
        live.db_mut();
        assert_eq!(live.changes_since(recent), Changes::All);
    }
}
