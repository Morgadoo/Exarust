//! What the possession rules read from the game's object database: each
//! item's kind (weapon flags, shield, held light) and each container's
//! children, built once the game is loaded, and the systems that keep
//! characters' possessions: a requested weapon-set swap is carried out and
//! leaves the hands' plan on the body.

use crate::card::{ApparelStats, ShieldStats, WeaponStats};
use crate::possession::{HandPlan, ItemKind, Possessions};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_assets::{Game, PackKind};
use exa_formats::characters::ID_MASK;
use exa_formats::containers::{CONTAINER, Container, LIGHT, USABLE, coins_of, section};
use exa_formats::objdb::Burden;
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
    /// A shield record's section bytes, for the encumbrance sum.
    shields: HashMap<u32, ShieldRecord>,
}

/// What one item's description card and weight rules read.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CardFacts {
    /// The display name from the model section's string id. Falls back to
    /// the record's own 16-byte name when no string library is loaded.
    pub name: String,
    /// The description from the model section's string id.
    pub description: String,
    /// The model section's string id: a card is refused while it is zero.
    pub name_id: u32,
    /// The model section's flags word.
    pub model_flags: u32,
    /// The item's value in copper, expanded from the compact word (the
    /// model section's `+0x26`, body `+0x1e`).
    pub coins: u32,
    /// The condition the title tint reads: a weapon's quality byte, an
    /// apparel copy's appearance quality, else 0.
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
            if item.model.is_empty() {
                continue;
            }
            out.models.insert(
                item.id,
                ItemModel {
                    model: item.model,
                    flags: item.model_flags,
                    weapon: item.weapon,
                },
            );
        }
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
            let facts = card_facts(payload, strings);
            if let Some(burden) = facts.burden {
                out.burdens.insert(r.id, burden);
            }
            if let Some(shield) = facts.shield_record {
                out.shields.insert(r.id, shield);
            }
            out.cards.insert(r.id, facts);
        }
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
            return None;
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
            return None;
        }
        self.models.get(&id)
    }

    /// The description card's facts for `id`.
    pub fn card_facts(&self, id: u32) -> Option<&CardFacts> {
        if id & !ID_MASK != 0 {
            return None;
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

/// The card facts of a record payload. The offsets are the section bodies'
/// (the card builders read header-relative words, eight bytes earlier).
fn card_facts(payload: &[u8], strings: Option<&Library>) -> CardFacts {
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
        facts.coins = model.get(COINS..COINS + 2).map_or(0, |w| {
            coins_of(u16::from_le_bytes(w.try_into().expect("2 bytes")))
        });
        let description_id = word(model, DESCRIPTION_ID);
        if let Some(strings) = strings {
            // A string id with bit 31 set names the record's own tier
            // library; the base catalogue's shared library holds the same
            // slot numbers for the base tier.
            let text = |id: u32| strings.get(id & ID_MASK).unwrap_or_default();
            facts.name = text(facts.name_id);
            facts.description = text(description_id);
        }
        if facts.name.is_empty() {
            // The record's own 16-byte name, as the item parser reads it.
            let raw = &model[..model.len().min(16)];
            let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
            facts.name = raw[..end].iter().map(|&c| c as char).collect();
        }
    }
    if let Some(body) = body {
        facts.armour = byte(body, 0x1A8);
        facts.condition = byte(body, 0x1B0);
        facts.skill = byte(body, 0x1AB);
        facts.sockets = byte(body, 0x218) & 0x80 != 0;
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
        facts.condition = byte(weapon, 0x0C);
        facts.skill = byte(weapon, 0x42);
        facts.sockets = byte(weapon, 0x48) & 0x80 != 0;
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
