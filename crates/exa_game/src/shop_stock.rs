//! The hub shop's live stock: the object database's shop entries and the
//! item instances the stock pass makes.
//!
//! [`crate::trade`]'s stock rules read a [`ShopWorld`]: every item record
//! with the OR of its section tags, the item a copy was made from, the
//! copies themselves and the variant seed. This module supplies that world
//! from the game's `objdb.rdb`, parsed once on the first shop click and kept
//! in [`LiveShopStock`], so a shelf's first stock is the object database's
//! own items. The kinds word is the table's flags word, as the original's
//! stock update reads it from the table entry (the databases' writer stores
//! the OR of the record's section tags there).
//!
//! An item copy is the original's local instance. In the game the stock
//! update runs over [`LiveShopWorld`], which makes each copy in the live
//! object database ([`exa_assets::live_objects::LiveObjects`]) with its own
//! item body, as the original's two copy helpers do:
//!
//! - **Apparel**: the record is instanced (`instantiate_or_duplicate`); its
//!   body's condition byte takes a quality `2u − 1` from the shared seed,
//!   damage 0x40 and dirt 0x35 with their spreads
//!   ([`crate::trade::apparel_condition`]); the apparel palette is rolled for
//!   the body's wealth (condition plus quality), reseeding the shared seed
//!   from the clock; then the copy is refreshed from its template: the
//!   template's value word, its paint seed when the copy has none, its
//!   protection bytes, the layers' and variant's roll (reseeding again) and
//!   the condition pass that scales protection and value by the copy's
//!   quality, damage and variant.
//! - **Weapons**: only a record whose header word has the instance bit and
//!   whose model is a factory's (`@…`) is copied (any other is stocked as
//!   itself); the copy is graded at level 1 (dirt 0x35 and damage 0x40, both
//!   within a whole spread, [`exa_items::randomise::grade`]), then refreshed
//!   from its template: the value word, the template's weight, balance,
//!   impact, thrust, sides, adjust and skill bytes, the condition's roll,
//!   the factory's stat pass and the condition pass over stats and value.
//! - A copy dropped from the shelf is removed from its tier (a record with
//!   a random table is kept).
//!
//! [`LiveShopStock`] alone is still a [`ShopWorld`] (tests and a missing
//! database): its copies keep an id under the instance bits and their
//! source only, and step the seed once.

//! Evidence: the trade unit's stock update and the hub's first-stock call
//! (`context/re/arena-hub.md`); the record's flags and shop fields are
//! `exa_formats::trade_record`.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::Resource;
use exa_formats::factory::Factory;
use exa_formats::objdb::{Item, ObjectDb};
use exa_formats::rdb::Database;
use exa_formats::trade_record::TradeRecord;
use exa_items::palette::{ApparelBlock, Palette};
use exa_items::randomise::{GradeSource, grade_object, section_range};
use exa_items::refresh::{TEMPLATE_SEED, apparel_condition_pass, refresh_weapon};

use crate::hub_windows::HubItemNames;
use crate::trade::{INSTANCE, ShopEntry, ShopWorld, apparel_condition, apparel_quality, step};

/// The game's shop entries and the item instances made from them.
#[derive(Resource, Debug, Default, Clone)]
pub struct LiveShopStock {
    entries: Vec<ShopEntry>,
    parents: HashMap<u32, u32>,
    next: u32,
    loaded: bool,
    /// The shared seed a weapon copy's refresh left, handed back to the
    /// stock update.
    refresh_state: Option<u32>,
}

impl LiveShopStock {
    /// A stock world over entries already read (in id order), marked
    /// loaded so the game's database is not read over them.
    #[must_use]
    pub fn from_entries(entries: Vec<ShopEntry>) -> Self {
        Self {
            entries,
            loaded: true,
            ..Self::default()
        }
    }

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

    /// Read every record of an object database into shop entries: its id,
    /// the table's flags word (the kinds the stock update tests) and its
    /// shop fields. The records stay in the table's order (id order).
    pub fn load_from_database(&mut self, database: &Database<'_>) {
        self.entries.clear();
        self.parents.clear();
        self.next = 0;
        for record in &database.records {
            let payload = database.payload(record);
            self.entries.push(ShopEntry {
                id: record.id,
                flags: record.flags,
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

    /// Remembers a live copy's source, for the shelf's names.
    pub fn note_copy(&mut self, copy: u32, source: u32) {
        if copy != source {
            self.parents.insert(copy, source);
        }
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

/// The record sections the copies read.
const NAME: u32 = 0x1000_0000;
const MODEL: u32 = 0x0010_0000;
const BODY: u32 = 0x1000;
const WEAPON: u32 = 0x4000;
/// The model's compact value word.
const VALUE: usize = 0x1E;
/// The grade a shop's weapon copy is made to.
pub const SHOP_GRADE: i32 = 1;

/// The apparel palette the copies are rolled with. The original's is one
/// shared palette; it persists between rolls.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct ItemPalette(pub Palette);

/// The weapon factories the copies' stat pass has parsed, by name.
#[derive(Resource, Debug, Default, Clone)]
pub struct ShopFactories(pub HashMap<String, Option<Arc<Factory>>>);

impl ShopFactories {
    /// The factory pass over a weapon body for the factory `name` (the
    /// model name after its `@`), reading `Factories.rpk` once per name.
    /// False when no factory has that name.
    pub fn pass(&mut self, game: Option<&exa_assets::Game>, name: &str, body: &mut [u8]) -> bool {
        self.pass_in(game.map(|game| &*game.0), name, body)
    }

    /// [`Self::pass`] over the game's data itself.
    pub fn pass_in(
        &mut self,
        game: Option<&exa_assets::GameData>,
        name: &str,
        body: &mut [u8],
    ) -> bool {
        let key = name.to_ascii_lowercase();
        let factory = self
            .0
            .entry(key)
            .or_insert_with(|| {
                let pack = game?.pack(exa_assets::PackKind::Factories)?;
                let bytes = pack
                    .get(name)
                    .or_else(|| pack.get(&format!("{name}.fty")))?;
                Factory::parse(bytes).ok().map(Arc::new)
            })
            .clone();
        factory.is_some_and(|factory| exa_items::refresh::factory_pass(&factory, body))
    }
}

/// The shop's stock world over the live object database: the shelf's
/// entries and names from [`LiveShopStock`], the copies made in `db`.
pub struct LiveShopWorld<'a> {
    pub stock: &'a mut LiveShopStock,
    pub db: &'a mut ObjectDb,
    pub palette: &'a mut Palette,
    /// The engine's entropy word (the performance counter), read where the
    /// original reseeds its shared generator.
    pub entropy: &'a mut dyn FnMut() -> u32,
    /// The factory stat pass by factory name (after the `@`).
    pub factory: &'a mut dyn FnMut(&str, &mut [u8]) -> bool,
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4)
        .map_or(0, |w| u32::from_le_bytes(w.try_into().expect("4 bytes")))
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2)
        .map(|w| u16::from_le_bytes(w.try_into().expect("2 bytes")))
}

/// The template a copy names: its header's parent word.
pub(crate) fn template_of(payload: &[u8]) -> Option<u32> {
    section_range(payload, NAME).map(|r| u32_at(&payload[r], 0x14))
}

/// The section tags of a payload, in order, with their body ranges.
fn sections(payload: &[u8]) -> Vec<(u32, std::ops::Range<usize>)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 8 <= payload.len() {
        let tag = u32_at(payload, at);
        let size = u32_at(payload, at + 4) as usize;
        if size < 8 || at + size > payload.len() {
            break;
        }
        out.push((tag, at + 8..at + size));
        at += size;
    }
    out
}

/// The apparel refresh from the template: the copy's sections in order,
/// each one the template also has. The model takes the template's value
/// word; the body its paint seed when it has none, its protection bytes,
/// the roll of its layers and variant (from the clock; the shared seed is
/// left where the roll ends) and the condition pass, which scales the
/// value word read before it.
pub(crate) fn refresh_apparel_copy(
    payload: &mut [u8],
    template: &[u8],
    palette: &Palette,
    entropy: u32,
    seed: &mut u32,
) {
    let mut value_at: Option<usize> = None;
    for (tag, range) in sections(payload) {
        let Some(own) = section_range(template, tag) else {
            continue;
        };
        if tag == MODEL {
            if let (Some(word), true) = (u16_at(&template[own], VALUE), range.len() >= VALUE + 2) {
                payload[range.start + VALUE..range.start + VALUE + 2]
                    .copy_from_slice(&word.to_le_bytes());
                value_at = Some(range.start + VALUE);
            }
        } else if tag == BODY {
            let tb = &template[own];
            let body = &mut payload[range.clone()];
            let (Some(mut block), Some(tblock)) =
                (ApparelBlock::of_body(body), ApparelBlock::of_body(tb))
            else {
                continue;
            };
            if block.seed == TEMPLATE_SEED {
                block.seed = tblock.seed;
            }
            if body.len() < 0x1A6 || tb.len() < 0x1A6 {
                continue;
            }
            body[0x1A0..0x1A6].copy_from_slice(&tb[0x1A0..0x1A6]);
            *seed = block.roll(palette, entropy);
            block.write_to(body);
            let mut value = value_at.and_then(|at| u16_at(payload, at));
            let body = &mut payload[range];
            apparel_condition_pass(body, tb, value.as_mut());
            if let (Some(at), Some(value)) = (value_at, value) {
                payload[at..at + 2].copy_from_slice(&value.to_le_bytes());
            }
        }
    }
}

/// The weapon refresh from the template: the model takes the template's
/// value word; the weapon section the template's bytes and, for a factory
/// model, its roll, the factory's stats and the condition pass.
pub(crate) fn refresh_weapon_copy(
    payload: &mut [u8],
    template: &[u8],
    entropy: u32,
    seed: &mut u32,
    factory: &mut dyn FnMut(&str, &mut [u8]) -> bool,
) {
    let mut model: Option<(String, Option<usize>)> = None;
    for (tag, range) in sections(payload) {
        let Some(own) = section_range(template, tag) else {
            continue;
        };
        if tag == MODEL {
            let name_end = payload[range.clone()]
                .iter()
                .take(16)
                .position(|&b| b == 0)
                .unwrap_or(16.min(range.len()));
            let name =
                String::from_utf8_lossy(&payload[range.start..range.start + name_end]).into_owned();
            let mut value_at = None;
            if let (Some(word), true) = (u16_at(&template[own], VALUE), range.len() >= VALUE + 2) {
                payload[range.start + VALUE..range.start + VALUE + 2]
                    .copy_from_slice(&word.to_le_bytes());
                value_at = Some(range.start + VALUE);
            }
            model = Some((name, value_at));
        } else if tag == WEAPON {
            let (name, value_at) = model.clone().unwrap_or_default();
            let mut value = value_at.and_then(|at| u16_at(payload, at)).unwrap_or(0);
            let made = name.strip_prefix('@').map(str::to_string);
            let body = &mut payload[range];
            refresh_weapon(
                body,
                &template[own],
                &mut value,
                made.is_some(),
                entropy,
                seed,
                |body| {
                    if let Some(made) = &made {
                        factory(made, body);
                    }
                },
            );
            if let Some(at) = value_at {
                payload[at..at + 2].copy_from_slice(&value.to_le_bytes());
            }
        }
    }
}

impl LiveShopWorld<'_> {
    /// Without a base tier (no game data) the copies are the stock's own
    /// stand-ins. [Exarust stand-in]
    fn stand_in(&self) -> bool {
        self.db.base.is_empty()
    }

    fn payload(&self, id: u32) -> Option<Vec<u8>> {
        self.db.find(id).map(|r| r.payload().to_vec())
    }

    fn template_payload(&self, payload: &[u8]) -> Option<Vec<u8>> {
        template_of(payload).and_then(|t| self.payload(t))
    }
}

impl GradeSource for LiveShopWorld<'_> {
    fn read(&mut self, id: u32) -> Option<(Item, Vec<u8>)> {
        let record = self.db.find(id)?;
        let mut item = record.item().ok()?;
        item.id = id;
        Some((item, record.payload().to_vec()))
    }

    fn instantiate(&mut self, id: u32) -> u32 {
        self.db.instantiate_or_duplicate(id)
    }

    fn write(&mut self, id: u32, payload: &[u8]) {
        self.db.set_payload(id, payload);
    }

    /// The weapon refresh. The grade hook carries no seed, so the weapon
    /// copy parks the shared seed in the stock for the refresh's roll and
    /// takes back where it ends ([`ShopWorld::weapon_copy`]).
    fn refresh(&mut self, id: u32) {
        let Some(mut payload) = self.payload(id) else {
            return;
        };
        let Some(template) = self.template_payload(&payload) else {
            return;
        };
        let entropy = (self.entropy)();
        let mut state = self.stock.refresh_state.unwrap_or(entropy);
        refresh_weapon_copy(&mut payload, &template, entropy, &mut state, self.factory);
        self.db.set_payload(id, &payload);
        self.stock.refresh_state = Some(state);
    }
}

impl ShopWorld for LiveShopWorld<'_> {
    fn entries(&self) -> &[ShopEntry] {
        &self.stock.entries
    }

    /// A copy's source is its header's parent word, as the stock update
    /// reads it; a record the database lacks has none.
    fn parent_of(&self, instance: u32) -> Option<u32> {
        if self.stand_in() {
            return self.stock.parent_of_instance(instance);
        }
        self.db.find(instance)?;
        Some(
            self.db
                .section(instance, NAME)
                .map_or(0, |header| u32_at(header, 0x14)),
        )
    }

    fn release(&mut self, instance: u32) {
        self.stock.parents.remove(&instance);
        self.db.remove(instance);
    }

    fn apparel_copy(&mut self, id: u32, seed: &mut u32) -> u32 {
        if self.stand_in() {
            return self.stock.copy(id, seed);
        }
        let copy = self.db.instantiate_or_duplicate(id);
        self.stock.note_copy(copy, id);
        let Some(mut payload) = self.payload(copy) else {
            return copy;
        };
        let Some(body) = section_range(&payload, BODY) else {
            return copy;
        };
        // The quality from the shared seed, then the palette for the
        // body's wealth (which reseeds it from the clock), then damage and
        // dirt (written with the quality here: they draw nothing).
        apparel_condition(&mut payload[body.clone()], seed);
        let wealth = apparel_quality(&payload[body]);
        *seed = self.palette.roll(wealth, 1, (self.entropy)());
        if let Some(template) = self.template_payload(&payload) {
            let entropy = (self.entropy)();
            refresh_apparel_copy(&mut payload, &template, self.palette, entropy, seed);
        }
        self.db.set_payload(copy, &payload);
        copy
    }

    fn weapon_copy(&mut self, id: u32, seed: &mut u32) -> u32 {
        if self.stand_in() {
            return self.stock.copy(id, seed);
        }
        self.stock.refresh_state = Some(*seed);
        let copy = grade_object(self, id, SHOP_GRADE);
        if let Some(state) = self.stock.refresh_state.take() {
            *seed = state;
        }
        self.stock.note_copy(copy, id);
        copy
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

    /// A shelf of the two records fills with copies of them, each copy
    /// remembering its source.
    #[test]
    fn a_first_stock_takes_the_records_as_copies() {
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

    fn section(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut out = tag.to_le_bytes().to_vec();
        out.extend((body.len() as u32 + 8).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    /// A base record: a header with the instance bit, a model with a value
    /// word and either an apparel body or a weapon section.
    fn base_record(model_name: &str, value: u16, tail: (u32, Vec<u8>)) -> Vec<u8> {
        let mut header = b"Rec".to_vec();
        header.resize(0x18, 0);
        header[0x10] = 1;
        let mut model = model_name.as_bytes().to_vec();
        model.resize(0x28, 0);
        model[0x1E..0x20].copy_from_slice(&value.to_le_bytes());
        let mut payload = section(NAME, &header);
        payload.extend(section(MODEL, &model));
        payload.extend(section(tail.0, &tail.1));
        payload
    }

    fn live_db() -> ObjectDb {
        let mut body = vec![0u8; 0x214];
        body[0x1A0..0x1A6].copy_from_slice(&[40, 50, 60, 70, 80, 90]);
        body[0x1A8] = 1;
        body[0x1AA] = 0xFF;
        body[0x1B0] = 0xC0;
        let mut weapon = vec![0u8; 0x60];
        weapon[0x0C] = 0xFF;
        weapon[0x30..0x40].fill(20);
        let mut db = ObjectDb::default();
        db.base = exa_formats::objdb::Store::from_records([
            (
                10u32,
                0x0010_1000u32 | NAME,
                base_record("Coat", 100, (BODY, body)).as_slice(),
            ),
            (
                20,
                0x0010_4000 | NAME,
                base_record("@sword_x", 200, (WEAPON, weapon)).as_slice(),
            ),
            (
                30,
                0x0010_4000 | NAME,
                base_record("plain_axe", 50, (WEAPON, vec![0u8; 0x60])).as_slice(),
            ),
        ]);
        db
    }

    /// The live world copies an apparel record into the local tier: the
    /// copy names its source, takes the quality the shared seed gives
    /// before the palette reseeds it, and its body is the template's
    /// refreshed at that condition.
    #[test]
    fn a_live_apparel_copy_gets_its_own_body() {
        let mut stock = LiveShopStock::from_entries(Vec::new());
        let mut db = live_db();
        let mut palette = Palette::default();
        let mut entropy = || 0x1234_5678u32;
        let mut factory = |_: &str, _: &mut [u8]| false;
        let mut world = LiveShopWorld {
            stock: &mut stock,
            db: &mut db,
            palette: &mut palette,
            entropy: &mut entropy,
            factory: &mut factory,
        };
        let mut seed = 7;
        let copy = world.apparel_copy(10, &mut seed);
        assert_eq!(copy, exa_formats::objdb::LOCAL | 1);
        assert_eq!(world.parent_of(copy), Some(10));
        // The condition byte is the one the seed alone gives.
        let mut alone = db.section(10, BODY).unwrap().to_vec();
        let mut first = 7;
        apparel_condition(&mut alone, &mut first);
        let body = db.section(copy, BODY).unwrap();
        assert_eq!(body[0x1A9], alone[0x1A9]);
        // The template keeps its own body; the seed moved on.
        assert_eq!(db.section(10, BODY).unwrap()[0x1A9], 0);
        assert_ne!(seed, first);
        assert_eq!(stock.parent_of_instance(copy), Some(10));
    }

    /// A factory weapon with the instance bit is copied and graded: dirt
    /// and damage drawn within their spreads from 0x35 and 0x40, its
    /// factory pass asked by name; a weapon that is not a factory's is
    /// stocked as itself.
    #[test]
    fn a_live_weapon_copy_is_graded_and_refreshed() {
        let mut stock = LiveShopStock::from_entries(Vec::new());
        let mut db = live_db();
        let mut palette = Palette::default();
        let mut entropy = || 0x0BAD_F00Du32;
        let mut asked = Vec::new();
        let mut factory = |name: &str, _: &mut [u8]| {
            asked.push(name.to_string());
            false
        };
        let mut world = LiveShopWorld {
            stock: &mut stock,
            db: &mut db,
            palette: &mut palette,
            entropy: &mut entropy,
            factory: &mut factory,
        };
        let mut seed = 7;
        let copy = world.weapon_copy(20, &mut seed);
        assert_eq!(copy, exa_formats::objdb::LOCAL | 1);
        assert_eq!(world.weapon_copy(30, &mut seed), 30);
        let weapon = db.section(copy, WEAPON).unwrap();
        // The grade's damage and dirt were drawn within their spreads.
        assert_ne!((weapon[0x0D], weapon[0x0E]), (0, 0));
        assert_eq!(weapon[0x0C], 0xFF);
        assert_eq!(asked, ["sword_x"]);
    }

    /// The shop entries take the table's flags word as the kinds word and
    /// keep the record's shop fields.
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
        file.extend_from_slice(&32u32.to_le_bytes());
        file.extend_from_slice(&7u32.to_le_bytes());
        // The table's flags word: a weapon section (0x4000) the payload
        // does not hold, so the kinds come from the table, not the payload.
        file.extend_from_slice(&0x4000u32.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        // The slot after the last entry, which the loader does not count.
        file.extend_from_slice(&[0; 16]);
        file.extend_from_slice(&payload);
        let database = Database::parse(&file).unwrap();
        let mut stock = LiveShopStock::default();
        stock.load_from_database(&database);
        assert!(stock.is_loaded());
        assert_eq!(stock.len(), 1);
        let entry = stock.entries()[0];
        assert_eq!(entry.id, 7);
        assert_eq!(entry.flags, 0x4000);
        assert_eq!(entry.record.unwrap().model.unwrap().rarity, 40);
    }
}
