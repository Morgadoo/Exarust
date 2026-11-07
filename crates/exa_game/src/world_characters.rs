//! The live world character database: the store (tag `0x4000_0000`) the
//! original keeps across levels for the characters that are not one
//! level's own: the player, and every follower that has taken a door with
//! the player.
//!
//! **What the original does.** A story start empties both character
//! databases and adds the new character to the world one under its first
//! free id (so 1). A save load imports the save's `#worldchrdb` into it,
//! and a save writes it back as that section. At a door, the transition
//! flushes the player and every follower ([`crate::follower_travel`]): a
//! character whose record carries the local database's tag (`0x8000_0000`)
//! keeps where it came from on its record (the level it leaves and its
//! local id, the record's origin sub-chunk), leaves the local database and
//! joins the world one under its first free id, which stamps the world tag
//! on its id; every flushed character's record then takes the level being
//! left as its level. Once the destination is in, the re-attach pass
//! stores the destination's id as each follower's level. The level
//! importer's character reader looks a placement whose reference has the
//! world bit up in this store and creates it only when the record's level
//! is the current one ([`exa_world::placed_characters`]).
//!
//! **What Exarust does.** [`WorldCharacters`] holds a
//! [`CharacterDb`] with the world tag. A fresh story (the default, and the
//! host's new-story start) holds the player's slot under id 1 (a reset
//! record; the save writes the live player there); a story load imports the
//! save's section ([`WorldCharacters::from_save`]). The follower flush
//! registers each travelling follower ([`WorldCharacters::register`]); the
//! host's re-attach stores the destination's id
//! ([`WorldCharacters::arrive`]); the story save exports the store with the
//! live player over id 1 ([`crate::save_live::story_player_world_with`]);
//! and the host's level load reads the store's records for the level's
//! world references. [Exarust's: there is no live local character
//! database, so the flush's removal from it has no counterpart (the
//! level's own records stay in its section); the scene characters are not
//! written into the save's level section, so a saved follower is kept in
//! the store but not placed again by a load.]
//!
//! Evidence (REA, Exanima.exe, checked against the assembly): the save
//! unit's character flush (the local-tag test, the origin words, the local
//! delete, the world add under id 0 and the level word), its re-attach pass
//! (the level word set to the new current level), the character
//! databases' lookup (the world bit routes to the world store, the low 28
//! bits name the entry) and add (id 0 takes the first free id and the
//! store's tag is stamped on the record), and the record writer's origin
//! sub-chunk (written when the level word is set).

use bevy::prelude::*;
use exa_formats::character_db::{CharacterDb, LOCAL_TAG, WORLD_TAG};
use exa_formats::characters::{CharacterRecord, ID_MASK};

use crate::save_live::PLAYER_RECORD_ID;

/// The live world character database.
#[derive(Resource, Debug)]
pub struct WorldCharacters {
    pub db: CharacterDb,
}

impl Default for WorldCharacters {
    fn default() -> Self {
        Self::new_story()
    }
}

impl WorldCharacters {
    /// A new story's store: emptied, with the new character under id 1
    /// (a reset record stands in until a save writes the live player).
    pub fn new_story() -> Self {
        let mut db = CharacterDb::new(WORLD_TAG);
        let _ = db.add(PLAYER_RECORD_ID, CharacterRecord::reset());
        Self { db }
    }

    /// A story load's store: the save's world character database as
    /// imported (a save without one gives an empty store).
    pub fn from_save(db: Option<CharacterDb>) -> Self {
        Self {
            db: db.unwrap_or_else(|| CharacterDb::new(WORLD_TAG)),
        }
    }

    /// Every record the store holds, in id order, each under its
    /// world-tagged id (entries that do not parse are left out).
    pub fn records(&mut self) -> Vec<CharacterRecord> {
        let ids: Vec<u32> = self.db.ids().collect();
        ids.into_iter()
            .filter_map(|id| self.db.get(id).map(|shared| shared.record().clone()))
            .collect()
    }

    /// The records other than the player's slot, for a save's export.
    pub fn others(&mut self) -> Vec<CharacterRecord> {
        self.records()
            .into_iter()
            .filter(|record| record.id & ID_MASK != PLAYER_RECORD_ID)
            .collect()
    }

    /// The flush's database part for one travelling character leaving
    /// level `level` (its id in `locales.rdb`): a record that is not the
    /// world store's (the local tag, or Exarust's untagged level ids)
    /// takes its origin (`level` and its old id with the local tag), joins
    /// the store under its first free id and carries that id with the
    /// world tag from now on; a record already in the store replaces its
    /// live copy. Either way the record's level becomes `level`. Returns
    /// the record's id.
    pub fn register(&mut self, record: &mut CharacterRecord, level: u32) -> u32 {
        if record.id & WORLD_TAG == 0 {
            let local = if record.id & LOCAL_TAG != 0 {
                record.id
            } else {
                // Exarust keeps a level's own ids untagged.
                (record.id & ID_MASK) | LOCAL_TAG
            };
            record.origin = Some([level, local]);
            record.locale = level;
            if let Some(shared) = self.db.add(0, record.clone()) {
                record.id = shared.id();
            }
        } else {
            record.locale = level;
            self.store(record);
        }
        record.id
    }

    /// The re-attach's database part: the follower's record (world id)
    /// takes level `level` (the destination's id) and the store keeps it.
    pub fn arrive(&mut self, record: &mut CharacterRecord, level: u32) {
        record.locale = level;
        if record.id & WORLD_TAG != 0 {
            self.store(record);
        }
    }

    /// Puts `record` as the live copy of its world id, adding the entry
    /// when the store lacks it.
    fn store(&mut self, record: &CharacterRecord) {
        let id = record.id & ID_MASK;
        match self.db.get(id) {
            Some(shared) => *shared.record() = record.clone(),
            None => {
                let _ = self.db.delete(id);
                let _ = self.db.add(id, record.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(id: u32, name: &str) -> CharacterRecord {
        let mut record = CharacterRecord::reset();
        record.id = id;
        record.name = name.into();
        record
    }

    #[test]
    fn a_new_story_holds_the_player_slot_so_a_follower_takes_two() {
        let mut world = WorldCharacters::new_story();
        let mut derrin = named(0x8000_0007, "Derrin");
        let id = world.register(&mut derrin, 3);
        assert_eq!(id, WORLD_TAG | 2);
        assert_eq!(derrin.id, WORLD_TAG | 2);
        assert_eq!(derrin.origin, Some([3, 0x8000_0007]));
        assert_eq!(derrin.locale, 3);
        let records = world.records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].name, "Derrin");
        assert_eq!(records[1].id, WORLD_TAG | 2);
        assert_eq!(world.others().len(), 1);
    }

    #[test]
    fn an_untagged_level_id_is_the_local_one() {
        let mut world = WorldCharacters::new_story();
        let mut searcher = named(12, "Searcher");
        world.register(&mut searcher, 5);
        assert_eq!(searcher.origin, Some([5, LOCAL_TAG | 12]));
        assert_eq!(searcher.id, WORLD_TAG | 2);
    }

    #[test]
    fn a_world_follower_keeps_its_id_and_takes_each_new_level() {
        let mut world = WorldCharacters::new_story();
        let mut derrin = named(7, "Derrin");
        world.register(&mut derrin, 3);
        world.arrive(&mut derrin, 4);
        assert_eq!(world.records()[1].locale, 4);
        // The next door: already the store's, it keeps its id and origin.
        derrin.name = "Derrin again".into();
        let id = world.register(&mut derrin, 4);
        assert_eq!(id, WORLD_TAG | 2);
        assert_eq!(derrin.origin, Some([3, LOCAL_TAG | 7]));
        let records = world.records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].name, "Derrin again");
    }

    #[test]
    fn the_store_survives_its_save_form() {
        let mut world = WorldCharacters::new_story();
        let mut derrin = named(7, "Derrin");
        world.register(&mut derrin, 3);
        world.db.flush().unwrap();
        let bytes = world.db.save();
        let mut loaded =
            WorldCharacters::from_save(Some(CharacterDb::load(WORLD_TAG, &bytes).unwrap()));
        let records = loaded.records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].origin, Some([3, LOCAL_TAG | 7]));
        assert_eq!(records[1].locale, 3);
        assert!(WorldCharacters::from_save(None).records().is_empty());
    }
}
