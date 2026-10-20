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

const WEAPON: u32 = 0x4000;
const SHIELD: u32 = 0x8000;

/// Item kinds and container children of the game's object database, by
/// record id. Ids with a database tag (a level's own items, the world's)
/// are not in it.
#[derive(Resource, Debug, Default, Clone)]
pub struct ItemCatalogue {
    kinds: HashMap<u32, ItemKind>,
    contents: HashMap<u32, Vec<u32>>,
}

impl ItemCatalogue {
    /// Reads every record of an object database.
    pub fn of_database(db: &Database<'_>) -> Self {
        let mut out = Self::default();
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
                },
            );
            if let Some(c) = section(payload, CONTAINER).and_then(Container::parse) {
                out.contents
                    .insert(r.id, c.children.iter().map(|c| c.id).collect());
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

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}

/// Asks for a character's weapon sets to be swapped.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SwapRequested;

/// What the hands must take after a swap, for the code that attaches held
/// models. Nothing reattaches them yet.
#[derive(Component, Debug, Clone, Copy)]
pub struct PendingHands(pub HandPlan);

/// Builds the catalogue once the game data is there.
pub fn load_catalogue(game: Option<Res<Game>>, mut catalogue: ResMut<ItemCatalogue>) {
    if !catalogue.is_empty() {
        return;
    }
    let Some(game) = game else {
        return;
    };
    let Some(bytes) = game
        .0
        .pack(PackKind::Resource)
        .and_then(|p| p.get("objdb.rdb"))
    else {
        return;
    };
    if let Ok(db) = Database::parse(bytes) {
        *catalogue = ItemCatalogue::of_database(&db);
        info!("item catalogue: {} records", catalogue.kinds.len());
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
