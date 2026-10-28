//! What the possession rules read from the game's object database: each
//! item's kind (weapon flags, shield, held light) and each container's
//! children, built once the game is loaded, and the systems that keep
//! characters' possessions: a requested weapon-set swap is carried out and
//! leaves the hands' plan on the body.

use crate::possession::{HandPlan, ItemKind, Possessions};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_assets::{Game, PackKind};
use exa_formats::characters::ID_MASK;
use exa_formats::containers::{CONTAINER, Container, LIGHT, section};
use exa_formats::rdb::Database;

const BODY: u32 = 0x1000;
const WEAPON: u32 = 0x4000;
const SHIELD: u32 = 0x8000;
/// The item's model section; its body starts eight bytes after the tag.
const MODEL: u32 = 0x0010_0000;
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
}

impl ItemCatalogue {
    /// Reads every record of an object database.
    pub fn of_database(db: &Database<'_>) -> Self {
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

    /// The item's ground model, when its record names one. An item without
    /// a model section has no icon, as the original's icon acquire returns
    /// 0 for it.
    pub fn model(&self, id: u32) -> Option<&ItemModel> {
        if id & !ID_MASK != 0 {
            return None;
        }
        self.models.get(&id)
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}

/// The model section's `+0x24` byte (body `+0x1C`): the bulk an item panel
/// counts. `None` for an item without a model section or with a short one.
fn model_bulk(payload: &[u8]) -> Option<u8> {
    section(payload, MODEL)
        .and_then(|body| body.get(MODEL_BULK))
        .copied()
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
    let Some(bytes) = game
        .0
        .pack(PackKind::Resource)
        .and_then(|p| p.get("objdb.rdb"))
    else {
        warn!("item catalogue: no objdb.rdb in the resource pack");
        return;
    };
    match Database::parse(bytes) {
        Ok(db) => {
            *catalogue = ItemCatalogue::of_database(&db);
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
