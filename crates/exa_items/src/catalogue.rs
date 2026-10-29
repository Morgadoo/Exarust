//! What the possession rules read from the game's object database: each
//! item's kind (weapon flags, shield, held light) and each container's
//! children, built once the game is loaded, and the systems that keep
//! characters' possessions: a requested weapon-set swap is carried out and
//! leaves the hands' plan on the body.

use crate::card::{ApparelStats, ShieldStats, WeaponStats};
use crate::desc_phrase::SentenceSource;
use crate::possession::{HandPlan, ItemKind, Possessions};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_assets::{Game, PackKind};
use exa_formats::characters::ID_MASK;
use exa_formats::containers::{CONTAINER, Container, LIGHT, USABLE, coins_of, section};
use exa_formats::objdb::{Burden, INHERIT};
use exa_formats::objstrings::Library;
use exa_formats::rdb::Database;
use exa_formats::weapon_record::ShieldRecord;

const BODY: u32 = 0x1000;
const WEAPON: u32 = 0x4000;
const SHIELD: u32 = 0x8000;
/// The item's model section; its body starts eight bytes after the tag.
const MODEL: u32 = 0x0010_0000;
/// The map record section that keeps a map out of the stats cards.
const MAP: u32 = 0x8000_0b03;
/// A weapon or shield section flag that keeps the item off its stats page.
const WITHHELD: u32 = 0x0100_0000;
/// The bulk byte from the model section's body: the section's `+0x24` less
/// its eight-byte header (`exa_formats::objdb` reads the same one).
const MODEL_BULK: usize = 0x1C;

/// The ground model an item icon renders, from the object database's model
/// section.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemModel {
    /// The `Objects`/`Objlib` entry, or `@` and a factory name.
    pub model: String,
    /// The model section's word: bit 0 dims a lit model by its condition,
    /// bit `0x2000` asks for the icon's extra effect.
    pub flags: u32,
    /// A factory weapon's seeds, when the model is assembled.
    pub weapon: Option<exa_formats::objdb::Weapon>,
    /// The body section's appearance block: the original's icon render
    /// paints an apparel model's run-time (`&`) materials from it, the same
    /// paint job an equipped item takes, before it draws the icon.
    pub appearance: Option<exa_formats::objdb::Appearance>,
}

/// Item kinds and container children of the game's object database, by
/// record id. Ids with a database tag (a level's own items, the world's)
/// are not in it.
#[derive(Resource, Debug, Default, Clone)]
pub struct ItemCatalogue {
    kinds: HashMap<u32, ItemKind>,
    contents: HashMap<u32, Vec<u32>>,
    /// The model section's `0x24` byte: an item panel counts it as the
    /// entry's bulk, and the equip conflict pass reads the same byte as an
    /// item's weight class.
    bulk: HashMap<u32, u8>,
    models: HashMap<u32, ItemModel>,
    /// The usable section's first two bytes (its kind and sort): what a use
    /// or the compass check reads. `None` for an item that is not usable.
    usable: HashMap<u32, [u8; 2]>,
    /// What the description card and the encumbrance sum read from the
    /// record. Kept apart from [`ItemKind`] so the possession rules stay
    /// small.
    cards: HashMap<u32, CardFacts>,
    /// The items' worn weight, keyed by record id.
    burdens: HashMap<u32, Burden>,
    /// The loaded level's own records (their ids tagged with the local
    /// tier bit): the model an icon renders and the bulk a panel counts.
    level_models: HashMap<u32, ItemModel>,
    level_bulk: HashMap<u32, u8>,
    /// The random tables container children draw from, by record id (the
    /// level's own under their local-tagged ids).
    tables: HashMap<u32, exa_formats::containers::RandomTable>,
    level_tables: HashMap<u32, exa_formats::containers::RandomTable>,
    /// A shield record's section bytes, for the encumbrance sum.
    shields: HashMap<u32, ShieldRecord>,
    /// The game's object string library, kept for the level's own cards.
    strings: Option<Library>,
    /// The loaded level's own records' card facts, under their local-tagged
    /// ids.
    level_cards: HashMap<u32, CardFacts>,
}

/// What one item's description card and weight rules read.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CardFacts {
    /// The display name from the model section's string id. Without a
    /// string library it is the record's own 16-byte name (an Exarust
    /// fallback; the original always has its library).
    pub name: String,
    /// Whether the name resolved: the card builder shows no card when the
    /// string resolver fails (a model section whose name id is 0, or an
    /// inherited id whose parent has none).
    pub titled: bool,
    /// The model's 16-byte name starts with `@` (a factory model): the
    /// weapon card adds its generated sentence only then.
    pub factory: bool,
    /// The description from the model section's string id.
    pub description: String,
    /// The model section's string id: a card is refused while it is zero.
    pub name_id: u32,
    /// The model section's flags word.
    pub model_flags: u32,
    /// The item's value in copper, expanded from the compact word (the
    /// model section's `+0x26`, body `+0x1e`).
    pub coins: u32,
    /// The condition the title tint reads, from the section of the page
    /// the card shows: a weapon's quality byte, an apparel body's
    /// appearance quality, else 0. The rank word ([`Self::skill`]) and the
    /// socket flag ([`Self::sockets`]) come from the same section (the
    /// shield page's rank is its section's `+0x08`).
    pub condition: u8,
    pub container: bool,
    pub map: bool,
    pub model: bool,
    /// A weapon's card fields.
    pub weapon: Option<WeaponStats>,
    /// An apparel item's card fields.
    pub apparel: Option<ApparelStats>,
    /// A shield's card fields.
    pub shield: Option<ShieldStats>,
    /// The body's armour type byte (1–6 makes an apparel card).
    pub armour: u8,
    /// The crystal-socket flag bit.
    pub sockets: bool,
    /// The weapon card's material row gate.
    pub material: bool,
    /// The rank word's low byte: the weapon's `+0x42` or the body's
    /// `+0x1ab`.
    pub skill: u8,
    /// What a character wearing the item adds to its encumbrance.
    pub burden: Option<Burden>,
    /// The shield's held factors, when it has a shield section.
    pub shield_record: Option<ShieldRecord>,
    /// The generated sentence the card adds to the description, and the
    /// section bytes it reads.
    pub sentence: Option<SentenceSource>,
}

impl ItemCatalogue {
    /// Reads every record of an object database, without its string
    /// library: display names fall back to the records' own names and
    /// descriptions are empty.
    pub fn of_database(db: &Database<'_>) -> Self {
        Self::of_database_with_strings(db, None)
    }

    /// Reads every record of an object database and resolves display names
    /// and descriptions through its object string library.
    pub fn of_database_with_strings(db: &Database<'_>, strings: Option<&Library>) -> Self {
        let mut out = Self::default();
        for item in exa_formats::objdb::items(db) {
            if let Some(table) = item.random_table.clone() {
                out.tables.insert(item.id, table);
            }
            if item.model.is_empty() {
                continue;
            }
            out.models.insert(
                item.id,
                ItemModel {
                    model: item.model,
                    flags: item.model_flags,
                    weapon: item.weapon,
                    appearance: item.appearance,
                },
            );
        }
        let payloads: HashMap<u32, &[u8]> =
            db.records.iter().map(|r| (r.id, db.payload(r))).collect();
        for r in &db.records {
            let payload = db.payload(r);
            let weapon = section(payload, WEAPON)
                .and_then(|b| b.get(..4))
                .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")));
            out.kinds.insert(
                r.id,
                ItemKind {
                    weapon,
                    shield: section(payload, SHIELD).is_some(),
                    light: section(payload, LIGHT).is_some(),
                    sight: section(payload, BODY).map(exa_formats::objdb::special_kind)
                        == Some(exa_formats::objdb::Burden::SEES_IN_THE_DARK),
                },
            );
            if let Some(c) = section(payload, CONTAINER).and_then(Container::parse) {
                out.contents
                    .insert(r.id, c.children.iter().map(|c| c.id).collect());
            }
            if let Some(bulk) = model_bulk(payload) {
                out.bulk.insert(r.id, bulk);
            }
            if let Some(usable) = usable_of(payload) {
                out.usable.insert(r.id, usable);
            }
            let facts = card_facts(payload, strings, |id| payloads.get(&id).copied());
            if let Some(burden) = facts.burden {
                out.burdens.insert(r.id, burden);
            }
            if let Some(shield) = facts.shield_record {
                out.shields.insert(r.id, shield);
            }
            out.cards.insert(r.id, facts);
        }
        out.strings = strings.cloned();
        out
    }

    /// The kind of the game's item `id`.
    pub fn kind(&self, id: u32) -> Option<ItemKind> {
        if id & !ID_MASK != 0 {
            return None;
        }
        self.kinds.get(&id).copied()
    }

    /// A container's children; `None` for anything that is not one.
    pub fn contents(&self, id: u32) -> Option<&[u32]> {
        if id & !ID_MASK != 0 {
            return None;
        }
        self.contents.get(&id).map(Vec::as_slice)
    }

    /// The item's bulk (the model section's `0x24` byte); `None` for an id
    /// that is not a record of the game's database.
    pub fn bulk(&self, id: u32) -> Option<u8> {
        if id & !ID_MASK != 0 {
            return self.level_bulk.get(&id).copied();
        }
        self.bulk.get(&id).copied()
    }

    /// The item's usable section's kind and sort (the compass is kind 4,
    /// sort 11); `None` when the item cannot be used.
    pub fn usable(&self, id: u32) -> Option<[u8; 2]> {
        if id & !ID_MASK != 0 {
            return None;
        }
        self.usable.get(&id).copied()
    }

    /// The item's ground model, when its record names one. An item without
    /// a model section has no icon, as the original's icon acquire returns
    /// 0 for it.
    pub fn model(&self, id: u32) -> Option<&ItemModel> {
        if id & !ID_MASK != 0 {
            return self.level_models.get(&id);
        }
        self.models.get(&id)
    }

    /// The description card's facts for `id`.
    pub fn card_facts(&self, id: u32) -> Option<&CardFacts> {
        if id & !ID_MASK != 0 {
            return self.level_cards.get(&id);
        }
        self.cards.get(&id)
    }

    /// What wearing the item adds to its wearer's encumbrance.
    pub fn burden(&self, id: u32) -> Option<Burden> {
        if id & !ID_MASK != 0 {
            return None;
        }
        self.burdens.get(&id).copied()
    }

    /// A held shield's weight, block and bulk.
    pub fn shield_record(&self, id: u32) -> Option<ShieldRecord> {
        if id & !ID_MASK != 0 {
            return None;
        }
        self.shields.get(&id).copied()
    }

    /// The item's usable section's first byte: the card's delay class.
    pub fn card_kind(&self, id: u32) -> Option<u8> {
        self.usable(id).map(|usable| usable[0])
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    /// Takes the loaded level's own records (the level's local database),
    /// replacing the previous level's: each record, under its local-tagged
    /// id, gets the model it names, or, without a model name, its
    /// template's (the header's parent word, a game record or another
    /// local one), and the bulk byte of its model section, else its
    /// template's. The original reads a local record in its own tier and
    /// inherits through the template; a placed chest's children are such
    /// records, so their icons render and their bulk counts. [guess: a
    /// local instance without its own model name takes its template's]
    pub fn set_level_items(&mut self, items: &[exa_formats::objdb::Item]) {
        use exa_formats::objdb::LOCAL;
        self.level_models.clear();
        self.level_bulk.clear();
        self.level_tables = items
            .iter()
            .filter_map(|item| {
                let table = item.random_table.clone()?;
                Some((LOCAL | (item.id & ID_MASK), table))
            })
            .collect();
        let by_id: HashMap<u32, &exa_formats::objdb::Item> =
            items.iter().map(|item| (item.id & ID_MASK, item)).collect();
        for item in items {
            let tagged = LOCAL | (item.id & ID_MASK);
            // Follow the template chain (bounded) to the first record
            // naming a model.
            let mut at = Some(item);
            let mut model = None;
            let mut bulk = None;
            // The nearest body section along the chain paints the icon
            // [guess: the original's section lookup inherits through the
            // template as its model lookup does].
            let mut appearance = None;
            for _ in 0..8 {
                let Some(record) = at else { break };
                if bulk.is_none() && !record.model.is_empty() {
                    bulk = Some(record.equip_lock);
                }
                appearance = appearance.or(record.appearance);
                if !record.model.is_empty() {
                    model = Some(ItemModel {
                        model: record.model.clone(),
                        flags: record.model_flags,
                        weapon: record.weapon,
                        appearance,
                    });
                    break;
                }
                let parent = record.parent;
                if parent == 0 {
                    break;
                }
                if parent & LOCAL != 0 {
                    at = by_id.get(&(parent & ID_MASK)).copied();
                } else {
                    model = self
                        .models
                        .get(&(parent & ID_MASK))
                        .cloned()
                        .map(|mut model| {
                            model.appearance = appearance.or(model.appearance);
                            model
                        });
                    bulk = bulk.or_else(|| self.bulk.get(&(parent & ID_MASK)).copied());
                    break;
                }
            }
            if let Some(model) = model {
                self.level_models.insert(tagged, model);
            }
            if let Some(bulk) = bulk {
                self.level_bulk.insert(tagged, bulk);
            }
        }
    }
}

impl ItemCatalogue {
    /// Builds the loaded level's own records' card facts (replacing the
    /// previous level's), under their local-tagged ids. The texts resolve
    /// as the original's resolver does for the local tier: 0 is no text;
    /// the inherit id reads the same field of the record the payload's
    /// parent word names, in the game's library (a game parent's resolved
    /// text stands for it [guess: equal unless that field is itself
    /// inherited]); another id with bit 31 set is the level's own library
    /// at the low 30 bits; any other id the game's library. Without the
    /// game's library the records keep their own 16-byte names.
    pub fn set_level_cards(&mut self, records: &[(u32, Vec<u8>)], local: Option<&Library>) {
        use exa_formats::objdb::LOCAL;
        self.level_cards.clear();
        let by_id: HashMap<u32, &[u8]> = records
            .iter()
            .map(|(id, payload)| (id & ID_MASK, payload.as_slice()))
            .collect();
        for (id, payload) in records {
            let mut facts = card_facts(payload, None, |_| None);
            if facts.model
                && let Some(shared) = self.strings.as_ref()
            {
                let text = |field: usize, pick: fn(&CardFacts) -> &String| {
                    let raw = section(payload, MODEL).map_or(0, |m| word(m, field));
                    let found = if raw == 0 {
                        return None;
                    } else if raw == INHERIT {
                        let parent = word(payload, 0x1C);
                        if parent & LOCAL != 0 {
                            let raw = by_id
                                .get(&(parent & ID_MASK))
                                .and_then(|p| section(p, MODEL))
                                .map_or(0, |m| word(m, field));
                            if raw == 0 {
                                return None;
                            }
                            shared.get(raw)
                        } else {
                            let facts = self.cards.get(&(parent & ID_MASK))?;
                            if !facts.titled && pick(facts).is_empty() {
                                return None;
                            }
                            Some(pick(facts).clone())
                        }
                    } else if raw & INHERIT != 0 {
                        local.and_then(|l| l.get(raw & ID_MASK))
                    } else {
                        shared.get(raw)
                    };
                    Some(found.unwrap_or_default())
                };
                let name = text(NAME_ID, |f| &f.name);
                facts.titled = name.is_some();
                facts.name = name.unwrap_or_default();
                facts.description = text(DESCRIPTION_ID, |f| &f.description).unwrap_or_default();
            }
            self.level_cards.insert(LOCAL | (id & ID_MASK), facts);
        }
    }
}

/// The model section's body offsets the display strings are named at.
const NAME_ID: usize = 0x14;
const DESCRIPTION_ID: usize = 0x18;
/// The model section's compact value word, from the section header
/// (`+0x26`), i.e. body `+0x1e`.
const COINS: usize = 0x1E;

fn word(body: &[u8], at: usize) -> u32 {
    body.get(at..at + 4)
        .map_or(0, |w| u32::from_le_bytes(w.try_into().expect("4 bytes")))
}

fn byte(body: &[u8], at: usize) -> u8 {
    body.get(at).copied().unwrap_or(0)
}

/// The record's display string named at `field` of its model section, by
/// the original's string resolver over the base tier: 0 resolves to
/// nothing; the inherit id (`0x80000000` exactly) reads the same field of
/// the record the payload's parent word names (`+0x1C`) and looks it up in
/// the shared library, one level only; another id with bit 31 set is the
/// tier's own library, which for the base tier is the shared one, at the
/// low 30 bits; any other id is the shared library's as it is. Once an id
/// is chosen the resolver succeeds even for an empty slot.
fn resolve_text<'a>(
    payload: &[u8],
    field: usize,
    strings: &Library,
    find: impl Fn(u32) -> Option<&'a [u8]>,
) -> Option<String> {
    let field_of = |payload: &[u8]| section(payload, MODEL).map_or(0, |m| word(m, field));
    let raw = field_of(payload);
    let text = if raw == 0 {
        return None;
    } else if raw == INHERIT {
        let parent = find(word(payload, 0x1C))?;
        let raw = field_of(parent);
        if raw == 0 {
            return None;
        }
        strings.get(raw)
    } else if raw & INHERIT == 0 {
        strings.get(raw)
    } else {
        strings.get(raw & ID_MASK)
    };
    Some(text.unwrap_or_default())
}

/// The card facts of a record payload. The offsets are the section bodies'
/// (the card builders read header-relative words, eight bytes earlier).
/// `find` gives a base record's payload by id, for inherited strings.
fn card_facts<'a>(
    payload: &[u8],
    strings: Option<&Library>,
    find: impl Fn(u32) -> Option<&'a [u8]>,
) -> CardFacts {
    let mut facts = CardFacts::default();
    let model = section(payload, MODEL);
    let body = section(payload, BODY);
    let weapon_section = section(payload, WEAPON);
    let shield_section = section(payload, SHIELD);
    facts.container = section(payload, CONTAINER).is_some();
    facts.map = section(payload, MAP).is_some();
    facts.model = model.is_some();
    if let Some(model) = model {
        facts.name_id = word(model, NAME_ID);
        facts.model_flags = word(model, 0x10);
        facts.factory = model.first() == Some(&b'@');
        facts.coins = model.get(COINS..COINS + 2).map_or(0, |w| {
            coins_of(u16::from_le_bytes(w.try_into().expect("2 bytes")))
        });
        if let Some(strings) = strings {
            let name = resolve_text(payload, NAME_ID, strings, &find);
            facts.titled = name.is_some();
            facts.name = name.unwrap_or_default();
            facts.description =
                resolve_text(payload, DESCRIPTION_ID, strings, &find).unwrap_or_default();
        } else {
            // No library loaded: the record's own 16-byte name, as the item
            // parser reads it. [Exarust fallback]
            let raw = &model[..model.len().min(16)];
            let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
            facts.name = raw[..end].iter().map(|&c| c as char).collect();
            facts.titled = true;
        }
    }
    if let Some(body) = body {
        facts.armour = byte(body, 0x1A8);
        facts.burden = Some(Burden::parse(body));
        if facts.armour != 0 {
            facts.apparel = Some(ApparelStats {
                weight: byte(body, 0x1A4),
                impact: byte(body, 0x1A0),
                slash: byte(body, 0x1A1),
                crush: byte(body, 0x1A2),
                pierce: byte(body, 0x1A3),
                encumbrance: byte(body, 0x1A5),
            });
        }
    }
    if let Some(weapon) = weapon_section {
        facts.material = byte(weapon, 0x3A) != 0;
        facts.weapon = Some(WeaponStats {
            flags: word(weapon, 0),
            weight: byte(weapon, 0x30),
            balance: byte(weapon, 0x31),
            grip: byte(weapon, 0x32),
            slash: byte(weapon, 0x37),
            crush: byte(weapon, 0x38),
            pierce: byte(weapon, 0x39),
            thrust: byte(weapon, 0x33),
        });
    }
    if let Some(shield) = shield_section {
        facts.shield = Some(ShieldStats {
            flags: word(shield, 0),
            weight: byte(shield, 4),
            impact: byte(shield, 5),
            encumbrance: byte(shield, 6),
        });
        facts.shield_record = ShieldRecord::from_section(shield);
    }
    facts.sentence = SentenceSource::choose(
        weapon_section.filter(|w| word(w, 0) & WITHHELD == 0),
        body,
        shield_section,
        facts.factory,
    );
    // The tint, rank and socket bytes are read by the builder of the page
    // the card shows, in the card builder's order: a weapon section that is
    // not withheld, else an apparel body of armour type 1 to 6, else a
    // shield section that is not withheld.
    if let Some(weapon) = weapon_section.filter(|w| word(w, 0) & WITHHELD == 0) {
        facts.condition = byte(weapon, 0x0C);
        facts.skill = byte(weapon, 0x42);
        facts.sockets = byte(weapon, 0x48) & 0x80 != 0;
    } else if let Some(body) = body.filter(|b| (1..7).contains(&byte(b, 0x1A8))) {
        facts.condition = byte(body, 0x1B0);
        facts.skill = byte(body, 0x1AB);
        facts.sockets = byte(body, 0x218) & 0x80 != 0;
    } else if let Some(shield) = shield_section.filter(|s| word(s, 0) & WITHHELD == 0) {
        facts.skill = byte(shield, 0x08);
    }
    facts
}

/// The model section's `+0x24` byte (body `+0x1C`): the bulk an item panel
/// counts. `None` for an item without a model section or with a short one.
fn model_bulk(payload: &[u8]) -> Option<u8> {
    section(payload, MODEL)
        .and_then(|body| body.get(MODEL_BULK))
        .copied()
}

/// The usable section's first two bytes: the item's use kind and sort, as
/// the compass check reads them. `None` for a short section.
fn usable_of(payload: &[u8]) -> Option<[u8; 2]> {
    section(payload, USABLE)
        .and_then(|body| body.get(..2))
        .map(|b| [b[0], b[1]])
}

/// Asks for a character's weapon sets to be swapped.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SwapRequested;

impl crate::randomise::TableSource for ItemCatalogue {
    /// The random table `id` names: a game record's, or the loaded level's
    /// own under its local-tagged id.
    fn table(&self, id: u32) -> Option<exa_formats::containers::RandomTable> {
        if id & !ID_MASK != 0 {
            self.level_tables.get(&id).cloned()
        } else {
            self.tables.get(&id).cloned()
        }
    }

    fn bulk(&self, id: u32) -> Option<u8> {
        ItemCatalogue::bulk(self, id)
    }
}

/// What the hands must take after a swap, for the code that attaches held
/// models. Nothing reattaches them yet.
#[derive(Component, Debug, Clone, Copy)]
pub struct PendingHands(pub HandPlan);

/// Builds the catalogue once the game data is there. The object database
/// is read once: a missing or unreadable one leaves the catalogue empty
/// rather than being retried every frame.
pub fn load_catalogue(
    game: Option<Res<Game>>,
    mut catalogue: ResMut<ItemCatalogue>,
    mut tried: Local<bool>,
) {
    if *tried {
        return;
    }
    let Some(game) = game else {
        return;
    };
    *tried = true;
    let Some(resource) = game.0.pack(PackKind::Resource) else {
        warn!("item catalogue: no resource pack");
        return;
    };
    let Some(bytes) = resource.get("objdb.rdb") else {
        warn!("item catalogue: no objdb.rdb in the resource pack");
        return;
    };
    match Database::parse(bytes) {
        Ok(db) => {
            // The display names and descriptions live in the object string
            // library; a missing or unreadable one leaves the records' own
            // names and empty descriptions.
            let strings = resource
                .get("objstrings.rdb")
                .and_then(|bytes| Library::parse(bytes).ok());
            *catalogue = ItemCatalogue::of_database_with_strings(&db, strings.as_ref());
            info!("item catalogue: {} records", catalogue.kinds.len());
        }
        Err(e) => warn!("item catalogue: objdb.rdb does not parse: {e}"),
    }
}

/// Carries out requested swaps: the sets trade places and the hands' plan
/// is left on the body.
pub fn swap_weapon_sets(
    mut commands: Commands,
    catalogue: Res<ItemCatalogue>,
    mut bodies: Query<(Entity, &mut Possessions), With<SwapRequested>>,
) {
    for (body, mut p) in &mut bodies {
        let plan = p.swap_sets(|id| catalogue.kind(id));
        let mut e = commands.entity(body);
        e.remove::<SwapRequested>();
        if let Some(plan) = plan {
            e.insert(PendingHands(plan));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::possession::{HELD, Hand};

    /// A level's own records take their own model or their template's,
    /// under the local-tagged id; untagged lookups are unchanged.
    #[test]
    fn level_records_resolve_their_model_and_bulk() {
        use exa_formats::objdb::{Item, LOCAL};
        let mut catalogue = ItemCatalogue::default();
        catalogue.models.insert(
            40,
            ItemModel {
                model: "Sword".into(),
                flags: 1,
                weapon: None,
                appearance: None,
            },
        );
        catalogue.bulk.insert(40, 12);
        let record = |id: u32, model: &str, parent: u32, bulk: u8| Item {
            model: model.into(),
            parent,
            equip_lock: bulk,
            ..Item::parse(id, &[]).expect("an empty item")
        };
        catalogue.set_level_items(&[
            record(1, "Apple", 0, 3),
            record(2, "", 40, 0),
            record(3, "", LOCAL | 1, 0),
            record(4, "", 0, 0),
        ]);
        assert_eq!(catalogue.model(LOCAL | 1).unwrap().model, "Apple");
        assert_eq!(catalogue.bulk(LOCAL | 1), Some(3));
        assert_eq!(catalogue.model(LOCAL | 2).unwrap().model, "Sword");
        assert_eq!(catalogue.model(LOCAL | 2).unwrap().flags, 1);
        assert_eq!(catalogue.bulk(LOCAL | 2), Some(12));
        assert_eq!(catalogue.model(LOCAL | 3).unwrap().model, "Apple");
        assert!(catalogue.model(LOCAL | 4).is_none());
        assert!(catalogue.model(1).is_none());
        assert_eq!(catalogue.model(40).unwrap().model, "Sword");
        // A new level replaces the old one's records.
        catalogue.set_level_items(&[]);
        assert!(catalogue.model(LOCAL | 1).is_none());
    }

    /// A record's body appearance rides with its model: a level instance
    /// keeps its own block, else takes its template's.
    #[test]
    fn level_records_carry_the_appearance_their_icon_paints() {
        use exa_formats::objdb::{Appearance, Item, LOCAL};
        let shirt = Appearance {
            quality: 200,
            damage: 10,
            dirt: 30,
            layers: Default::default(),
        };
        let worn = Appearance {
            damage: 120,
            ..shirt
        };
        let mut catalogue = ItemCatalogue::default();
        catalogue.models.insert(
            50,
            ItemModel {
                model: "Shirt".into(),
                flags: 0,
                weapon: None,
                appearance: Some(shirt),
            },
        );
        let record = |id: u32, parent: u32, appearance: Option<Appearance>| Item {
            parent,
            appearance,
            ..Item::parse(id, &[]).expect("an empty item")
        };
        catalogue.set_level_items(&[record(1, 50, None), record(2, 50, Some(worn))]);
        assert_eq!(catalogue.model(LOCAL | 1).unwrap().appearance, Some(shirt));
        assert_eq!(catalogue.model(LOCAL | 2).unwrap().appearance, Some(worn));
        assert_eq!(catalogue.model(50).unwrap().appearance, Some(shirt));
    }

    #[test]
    fn a_requested_swap_trades_the_sets() {
        let mut app = App::new();
        app.init_resource::<ItemCatalogue>()
            .add_systems(Update, swap_weapon_sets);
        let body = app
            .world_mut()
            .spawn((
                Possessions {
                    out: [Hand { id: 5, flags: 0 }, Hand::default()],
                    stowed: [Hand { id: 6, flags: 0 }, Hand::default()],
                    carried: vec![
                        crate::possession::Carried {
                            id: 5,
                            flags: HELD,
                            x: 0,
                            y: 0,
                        },
                        crate::possession::Carried {
                            id: 6,
                            flags: 0,
                            x: 0,
                            y: 0,
                        },
                    ],
                    ..Default::default()
                },
                SwapRequested,
            ))
            .id();
        app.update();
        let p = app.world().get::<Possessions>(body).unwrap();
        assert_eq!(p.out[0].id, 6);
        assert_eq!(p.carried[1].flags, HELD);
        assert!(app.world().get::<SwapRequested>(body).is_none());
        // The catalogue knows neither item: the hand changes, nothing to take.
        let plan = app.world().get::<PendingHands>(body).unwrap().0;
        assert_eq!(plan.changed, [true, false]);
        assert!(plan.take[0].is_none());
    }

    #[test]
    fn the_model_bulk_is_the_sections_0x24_byte() {
        // A model section: tag, size (8 + 0x1D), then a body whose byte
        // 0x1C is the bulk.
        let mut payload = Vec::new();
        let body_len = 0x1D;
        payload.extend_from_slice(&MODEL.to_le_bytes());
        payload.extend_from_slice(&(8 + body_len as u32).to_le_bytes());
        let mut body = vec![0u8; body_len];
        body[MODEL_BULK] = 7;
        payload.extend_from_slice(&body);
        assert_eq!(model_bulk(&payload), Some(7));
        // A truncated body has no byte.
        payload.truncate(8 + 0x1C);
        assert_eq!(model_bulk(&payload), None);
        // A record with no model section has none.
        assert_eq!(model_bulk(&[]), None);
    }

    #[test]
    fn the_usable_section_gives_its_kind_and_sort() {
        // The compass's usable section: kind 4, sort 11.
        let mut payload = Vec::new();
        payload.extend_from_slice(&USABLE.to_le_bytes());
        payload.extend_from_slice(&10u32.to_le_bytes());
        payload.extend_from_slice(&[4, 11]);
        assert_eq!(usable_of(&payload), Some([4, 11]));
        // Nothing usable, and a section shorter than two bytes.
        assert_eq!(usable_of(&[]), None);
        let mut short = Vec::new();
        short.extend_from_slice(&USABLE.to_le_bytes());
        short.extend_from_slice(&9u32.to_le_bytes());
        short.push(4);
        assert_eq!(usable_of(&short), None);
    }

    /// One `tag, size` section with `body`.
    fn section_bytes(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&((8 + body.len()) as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn the_card_facts_read_the_section_bytes() {
        let mut library = Library::default();
        let name = library.alloc();
        assert!(library.set(name, b"Rusty Sword"));
        let description = library.alloc();
        assert!(library.set(description, b"It is old."));

        // A model section: 16-byte name, flags, name and description ids,
        // then the value word at body +0x1e.
        let mut model = vec![0u8; 0x20];
        model[0] = b'M';
        model[0x10..0x14].copy_from_slice(&2u32.to_le_bytes());
        // The high bit names the record's own tier library; the base
        // catalogue resolves the same slot in the shared one.
        model[0x14..0x18].copy_from_slice(&(name | 0x8000_0000).to_le_bytes());
        model[0x18..0x1c].copy_from_slice(&description.to_le_bytes());
        // The compact value word: 0xC001 is one unit of 1440 copper.
        model[0x1e..0x20].copy_from_slice(&0xC001u16.to_le_bytes());

        // A weapon section: flags, quality, stat bytes and the socket word.
        let mut weapon = vec![0u8; 0x5a];
        weapon[0..4].copy_from_slice(&0x10u32.to_le_bytes());
        weapon[0x0c] = 200;
        weapon[0x30] = 40; // weight
        weapon[0x31] = 60; // balance
        weapon[0x32] = 130; // impact
        weapon[0x33] = 70; // thrust
        weapon[0x37] = 80; // slash
        weapon[0x38] = 20; // crush
        weapon[0x39] = 30; // pierce
        weapon[0x3a] = 1; // material row
        weapon[0x42] = 0x42; // rank
        weapon[0x48] = 0x80; // sockets

        let mut payload = section_bytes(MODEL, &model);
        payload.extend(section_bytes(WEAPON, &weapon));
        payload.extend(section_bytes(CONTAINER, &[]));
        let mut file = Vec::new();
        file.extend_from_slice(&exa_formats::objdb::MAGIC.to_le_bytes());
        file.extend_from_slice(&16u32.to_le_bytes());
        file.extend_from_slice(&7u32.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        file.extend_from_slice(&payload);
        let db = Database::parse(&file).unwrap();
        let cat = ItemCatalogue::of_database_with_strings(&db, Some(&library));
        let facts = cat.card_facts(7).unwrap();
        assert_eq!(facts.name, "Rusty Sword");
        assert_eq!(facts.description, "It is old.");
        assert_eq!((facts.name_id, facts.model_flags), (name | 0x8000_0000, 2));
        assert_eq!(facts.coins, 1440);
        assert_eq!((facts.condition, facts.skill), (200, 0x42));
        assert!(facts.container && facts.model && !facts.map && facts.sockets);
        assert!(facts.material);
        let w = facts.weapon.unwrap();
        assert_eq!((w.flags, w.weight, w.balance), (0x10, 40, 60));
        assert_eq!((w.grip, w.thrust), (130, 70));
        assert_eq!((w.slash, w.crush, w.pierce), (80, 20, 30));
    }

    /// An object database of `(id, payload)` records.
    fn database_bytes(records: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut file = Vec::new();
        file.extend_from_slice(&exa_formats::objdb::MAGIC.to_le_bytes());
        file.extend_from_slice(&(records.len() as u32 * 16).to_le_bytes());
        let mut offset = 0u32;
        for (id, payload) in records {
            file.extend_from_slice(&id.to_le_bytes());
            file.extend_from_slice(&0u32.to_le_bytes());
            file.extend_from_slice(&offset.to_le_bytes());
            file.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            offset += payload.len() as u32;
        }
        for (_, payload) in records {
            file.extend_from_slice(payload);
        }
        file
    }

    /// A record's header section (the first, so its parent word is the
    /// payload's `+0x1C`) naming `parent`.
    fn header(parent: u32) -> Vec<u8> {
        let mut body = vec![0u8; 0x18];
        body[0x14..0x18].copy_from_slice(&parent.to_le_bytes());
        section_bytes(0x1000_0000, &body)
    }

    /// A model section body with a name, flags and the two string ids.
    fn model_body(name: &[u8], name_id: u32, description_id: u32) -> Vec<u8> {
        let mut model = vec![0u8; 0x20];
        model[..name.len()].copy_from_slice(name);
        model[0x14..0x18].copy_from_slice(&name_id.to_le_bytes());
        model[0x18..0x1c].copy_from_slice(&description_id.to_le_bytes());
        model
    }

    /// The level's own records get cards under their local-tagged ids: a
    /// bit-31 id reads the level's library, a plain id the game's, the
    /// inherit id a game parent's text or a local parent's field in the
    /// game's library; a zero id has no name.
    #[test]
    fn the_level_records_have_cards_by_the_local_resolver() {
        use exa_formats::objdb::LOCAL;
        let mut shared = Library::default();
        let chest = shared.alloc();
        assert!(shared.set(chest, b"Chest"));
        let oak = shared.alloc();
        assert!(shared.set(oak, b"Oak, iron bound."));
        let mut game = header(0);
        game.extend(section_bytes(MODEL, &model_body(b"Chest_A", chest, oak)));
        let file = database_bytes(&[(5, game)]);
        let db = Database::parse(&file).unwrap();
        let mut cat = ItemCatalogue::of_database_with_strings(&db, Some(&shared));
        let mut local = Library::default();
        let drawers = local.alloc();
        assert!(local.set(drawers, b"Chest of drawers"));
        let mut own = header(0);
        own.extend(section_bytes(
            MODEL,
            &model_body(b"Drawers", drawers | INHERIT, oak),
        ));
        let mut copy = header(5);
        copy.extend(section_bytes(MODEL, &model_body(b"", INHERIT, INHERIT)));
        let mut of_local = header(LOCAL | 0x54);
        of_local.extend(section_bytes(MODEL, &model_body(b"", INHERIT, 0)));
        let mut blank = header(0);
        blank.extend(section_bytes(MODEL, &model_body(b"X", 0, 0)));
        cat.set_level_cards(
            &[(0x54, own), (0x55, copy), (0x56, of_local), (0x57, blank)],
            Some(&local),
        );
        let facts = cat.card_facts(LOCAL | 0x54).unwrap();
        assert_eq!(facts.name, "Chest of drawers");
        assert_eq!(facts.description, "Oak, iron bound.");
        assert!(facts.titled);
        let facts = cat.card_facts(LOCAL | 0x55).unwrap();
        assert_eq!((facts.name.as_str(), facts.titled), ("Chest", true));
        assert_eq!(facts.description, "Oak, iron bound.");
        // The local parent's name id has bit 31 set, but the inherit reads
        // it in the game's library, whose slot is empty: it still resolves.
        let facts = cat.card_facts(LOCAL | 0x56).unwrap();
        assert!(facts.titled);
        assert_eq!(facts.description, "");
        assert!(!cat.card_facts(LOCAL | 0x57).unwrap().titled);
        assert!(cat.card_facts(LOCAL | 0x58).is_none());
        // A new level replaces them.
        cat.set_level_cards(&[], None);
        assert!(cat.card_facts(LOCAL | 0x54).is_none());
    }

    #[test]
    fn the_card_strings_follow_the_resolver_and_the_page_picks_its_bytes() {
        let mut library = Library::default();
        let name = library.alloc();
        assert!(library.set(name, b"Mail Coat"));
        let description = library.alloc();
        assert!(library.set(description, b"Rings of iron."));

        // Record 1 names its strings; record 2 inherits both through its
        // parent word (payload +0x1C), and record 3 inherits from a
        // parent that is missing, so its name does not resolve.
        let mut first = header(0);
        first.extend(section_bytes(
            MODEL,
            &model_body(b"Coat", name, description),
        ));
        let mut second = header(1);
        second.extend(section_bytes(
            MODEL,
            &model_body(b"@coat", INHERIT, INHERIT),
        ));
        // Record 2 also has a withheld weapon section and an apparel body
        // of armour type 3: the apparel page's bytes are read.
        let mut weapon = vec![0u8; 0x50];
        weapon[0..4].copy_from_slice(&WITHHELD.to_le_bytes());
        weapon[0x0c] = 9;
        weapon[0x42] = 9;
        weapon[0x48] = 0x80;
        second.extend(section_bytes(WEAPON, &weapon));
        let mut body = vec![0u8; 0x220];
        body[0x1a8] = 3;
        body[0x1b0] = 77;
        body[0x1ab] = 0x21;
        second.extend(section_bytes(BODY, &body));
        let mut third = header(99);
        third.extend(section_bytes(MODEL, &model_body(b"Odd", INHERIT, 0)));

        let file = database_bytes(&[(1, first), (2, second), (3, third)]);
        let db = Database::parse(&file).unwrap();
        let cat = ItemCatalogue::of_database_with_strings(&db, Some(&library));
        let facts = cat.card_facts(2).unwrap();
        assert_eq!(facts.name, "Mail Coat");
        assert_eq!(facts.description, "Rings of iron.");
        assert!(facts.titled && facts.factory);
        assert_eq!((facts.condition, facts.skill), (77, 0x21));
        assert!(!facts.sockets);
        // No weapon page, so the apparel body gives the sentence.
        assert_eq!(
            facts.sentence,
            Some(SentenceSource::Apparel {
                seed: 0,
                variant: 0,
                quality: 77,
                damage: 0
            })
        );
        // A plain record with neither section has none.
        assert_eq!(cat.card_facts(1).unwrap().sentence, None);
        let facts = cat.card_facts(3).unwrap();
        assert!(!facts.titled);
        assert_eq!(facts.description, "");
        assert!(!cat.card_facts(1).unwrap().factory);
        // Without a library the record's own name stands in.
        let cat = ItemCatalogue::of_database(&db);
        let facts = cat.card_facts(3).unwrap();
        assert!(facts.titled);
        assert_eq!(facts.name, "Odd");
    }

    #[test]
    fn the_catalogue_reads_kinds_and_children() {
        let mut cat = ItemCatalogue::default();
        cat.kinds.insert(
            7,
            ItemKind {
                weapon: Some(0x13),
                light: true,
                ..Default::default()
            },
        );
        cat.contents.insert(9, vec![7]);
        assert_eq!(cat.kind(7).unwrap().grip(), 0x10);
        assert_eq!(cat.kind(0x8000_0007), None);
        assert_eq!(cat.contents(9), Some(&[7u32][..]));
        assert_eq!(cat.contents(7), None);
    }
}
