//! The hub shop's live stock: the object database's shop entries and the
//! item instances the stock pass makes.
//!
//! [`crate::trade`]'s stock rules read a [`ShopWorld`]: every item record
//! with the OR of its section tags, the item a copy was made from, the
//! copies themselves and the variant seed. This module supplies that world
//! from the game's `objdb.rdb`, parsed once on the first shop click and kept
//! in [`LiveShopStock`], so a shelf's first stock is the object database's
//! own items.
//!
//! An item copy is the original's local instance: a new id with the
//! instance bits set and the source record remembered. The original also
//! rolls the copy's condition and variant into a new item body; Exarust has
//! no live item-body database yet, so the copy keeps its id and source only
//! and the variant draw's step is applied to the seed without a body to
//! write. That is the one approximation here.
//!
//! Evidence: the trade unit's stock update and the hub's first-stock call
//! (`context/re/arena-hub.md`); the record's flags and shop fields are
//! `exa_formats::trade_record`.

use std::collections::HashMap;

use bevy::prelude::Resource;
use exa_formats::rdb::Database;
use exa_formats::trade_record::{TradeRecord, section_flags};

use crate::hub_windows::HubItemNames;
use crate::trade::{INSTANCE, ShopEntry, ShopWorld, step};

/// The game's shop entries and the item instances made from them.
#[derive(Resource, Debug, Default, Clone)]
pub struct LiveShopStock {
    entries: Vec<ShopEntry>,
    parents: HashMap<u32, u32>,
    next: u32,
    loaded: bool,
}

impl LiveShopStock {
    /// Whether the object database was read (even if it did not parse, so a
    /// missing file is not retried every frame).
    #[must_use]
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// How many item records the object database holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the object database holds no item records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Read every record of an object database into shop entries. The
    /// records stay in the database's order (id order).
    pub fn load_from_database(&mut self, database: &Database<'_>) {
        self.entries.clear();
        self.parents.clear();
        self.next = 0;
        for record in &database.records {
            let payload = database.payload(record);
            self.entries.push(ShopEntry {
                id: record.id,
                flags: section_flags(payload),
                record: TradeRecord::parse(payload).ok(),
            });
        }
        self.loaded = true;
    }

    /// Read the game's object database once. A missing or unreadable one
    /// leaves the stock empty and marks the read done.
    pub fn load_from_game(&mut self, game: &exa_assets::Game) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let Some(bytes) = game
            .0
            .pack(exa_assets::PackKind::Resource)
            .and_then(|pack| pack.get("objdb.rdb"))
        else {
            bevy::log::warn!("shop stock: no objdb.rdb in the resource pack");
            return;
        };
        match Database::parse(bytes) {
            Ok(database) => {
                self.load_from_database(&database);
                bevy::log::info!("shop stock: {} item records", self.entries.len());
            }
            Err(error) => bevy::log::warn!("shop stock: objdb.rdb does not parse: {error}"),
        }
    }

    /// Remember the display names of the shelf's items: each record from the
    /// item catalogue, each instance under its source's name.
    pub fn fill_names(
        &self,
        catalogue: &exa_items::catalogue::ItemCatalogue,
        names: &mut HubItemNames,
    ) {
        for entry in &self.entries {
            if let Some(facts) = catalogue.card_facts(entry.id)
                && facts.titled
            {
                names.0.insert(entry.id, facts.name.clone());
            }
        }
        for (&instance, &base) in &self.parents {
            if let Some(name) = names.0.get(&base).cloned() {
                names.0.insert(instance, name);
            }
        }
    }

    /// The source record an instance was copied from.
    #[must_use]
    pub fn parent_of_instance(&self, instance: u32) -> Option<u32> {
        self.parents.get(&instance).copied()
    }

    /// A new copy of `id`: the next instance id, remembering its source.
    fn copy(&mut self, id: u32, seed: &mut u32) -> u32 {
        *seed = step(*seed);
        self.next = self.next.wrapping_add(1);
        let instance = INSTANCE | self.next;
        self.parents.insert(instance, id);
        instance
    }
}

impl ShopWorld for LiveShopStock {
    fn entries(&self) -> &[ShopEntry] {
        &self.entries
    }

    fn parent_of(&self, instance: u32) -> Option<u32> {
        self.parent_of_instance(instance)
    }

    fn release(&mut self, instance: u32) {
        self.parents.remove(&instance);
    }

    fn apparel_copy(&mut self, id: u32, seed: &mut u32) -> u32 {
        self.copy(id, seed)
    }

    fn weapon_copy(&mut self, id: u32, seed: &mut u32) -> u32 {
        self.copy(id, seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::trade_record::TradeModel;

    fn record(rarity: u8, listing: u32) -> TradeRecord {
        TradeRecord {
            listing: Some(listing),
            parent: 0,
            model: Some(TradeModel {
                named: true,
                flags: 0,
                rarity,
                size: [1, 1],
            }),
            rank: 0,
        }
    }

    fn world() -> LiveShopStock {
        LiveShopStock {
            entries: vec![
                ShopEntry {
                    id: 10,
                    flags: 0x1000,
                    record: Some(record(0, 0)),
                },
                ShopEntry {
                    id: 20,
                    flags: 0x4000,
                    record: Some(record(20, 0)),
                },
            ],
            ..LiveShopStock::default()
        }
    }

    /// Copies take instance ids, remember their source and the seed steps.
    #[test]
    fn a_copy_is_a_new_instance_of_its_source() {
        let mut stock = world();
        let mut seed = 7;
        let apparel = stock.apparel_copy(10, &mut seed);
        let weapon = stock.weapon_copy(20, &mut seed);
        assert_eq!(apparel, INSTANCE | 1);
        assert_eq!(weapon, INSTANCE | 2);
        assert_eq!(stock.parent_of(apparel), Some(10));
        assert_eq!(stock.parent_of(weapon), Some(20));
        assert_eq!(seed, step(step(7)));
        stock.release(apparel);
        assert_eq!(stock.parent_of(apparel), None);
        assert_eq!(stock.parent_of(weapon), Some(20));
    }

    /// A shelf of the two records fills, keeps only listed items and names
    /// its instances after their sources.
    #[test]
    fn a_first_stock_takes_the_records_and_names_its_copies() {
        let mut stock = world();
        let mut shelf = Vec::new();
        let report = crate::trade::first_stock(&mut shelf, [1, 2], &mut stock);
        assert_eq!(report.invalid, 0);
        assert!((18..=20).contains(&shelf.len()));
        assert!(
            shelf
                .iter()
                .all(|&id| id == 10 || id == 20 || stock.parent_of(id).is_some())
        );
        assert!(stock.len() == 2);
    }

    /// The shop entries read the payload's tags as the kinds word and keep
    /// the record's shop fields.
    #[test]
    fn a_database_load_takes_the_flags_and_shop_fields() {
        // A model section (0x00100000) with a name, flags, rarity 40 and
        // two size words, plus a name section (0x10000000).
        let mut model = b"Helm".to_vec();
        model.resize(0x28, 0);
        model[0x1D] = 40;
        model[0x20..0x24].copy_from_slice(&1u32.to_le_bytes());
        model[0x24..0x28].copy_from_slice(&1u32.to_le_bytes());
        let name = vec![0u8; 0x18];
        let mut payload = Vec::new();
        for (tag, body) in [(0x0010_0000u32, &model), (0x1000_0000u32, &name)] {
            payload.extend(tag.to_le_bytes());
            payload.extend((body.len() as u32 + 8).to_le_bytes());
            payload.extend_from_slice(body);
        }
        let mut file = Vec::new();
        file.extend_from_slice(&exa_formats::objdb::MAGIC.to_le_bytes());
        file.extend_from_slice(&16u32.to_le_bytes());
        file.extend_from_slice(&7u32.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        file.extend_from_slice(&payload);
        let database = Database::parse(&file).unwrap();
        let mut stock = LiveShopStock::default();
        stock.load_from_database(&database);
        assert!(stock.is_loaded());
        assert_eq!(stock.len(), 1);
        let entry = stock.entries()[0];
        assert_eq!(entry.id, 7);
        assert_eq!(entry.flags, 0x0010_0000 | 0x1000_0000);
        assert_eq!(entry.record.unwrap().model.unwrap().rarity, 40);
    }
}
