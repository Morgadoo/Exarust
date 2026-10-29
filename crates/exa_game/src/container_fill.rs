//! The world-load fill of the loaded level's placed containers: a container
//! child naming a random table (the shipped chests' children are all such
//! "DropTable" records) is replaced by the row the table gives it, as the
//! original's world load does before any container is opened. Without it a
//! chest's popup shows the table records themselves, drawn with the table's
//! placeholder model.
//!
//! The draws are [`exa_items::randomise::Fill`]'s (quotas, count draws,
//! weighted draws, nested tables, the slot's item limit). The children keep
//! their places; a picked child takes the row's item with zero flags, an
//! empty slot id 0 with flags `0x100`, as the original writes them.
//! Exarust stand-ins: the original's pick instantiates a copy of the row's
//! item and applies the row's condition word, and its walk covers the
//! whole database in record order from the engine's shared generator;
//! here the child takes the row's item id itself, only the placed
//! containers are walked (in the level database's record order) and the
//! generator starts from [`FILL_SEED`].

use bevy::prelude::*;
use exa_formats::containers::Container;
use exa_formats::objdb::{ID_MASK, LOCAL};
use exa_items::catalogue::ItemCatalogue;
use exa_items::randomise::{Fill, TableSource};

/// The generator's start for a level's fill. [Exarust stand-in: the
/// original draws from the engine's shared generator, whose state at the
/// world load Exarust does not keep.]
pub const FILL_SEED: u32 = 0x0080_b71a;
/// The flags an empty slot is written with.
pub const EMPTY_SLOT: u32 = 0x100;

/// Fills the table children of `containers`, in order, from one shared
/// fill. Returns how many children were written.
pub fn fill_containers(containers: &mut [Container], source: &dyn TableSource, seed: u32) -> usize {
    let mut fill = Fill::default();
    for (c, container) in containers.iter().enumerate() {
        for (k, child) in container.children.iter().enumerate() {
            fill.add_slot(
                child.id,
                (c << 16) | k,
                i32::from(container.max_item),
                source,
            );
        }
    }
    if fill.draws.is_empty() {
        return 0;
    }
    let mut state = seed;
    fill.run(source, &mut state);
    let mut written = 0;
    for d in 0..fill.draws.len() {
        for (key, row) in fill.given(d) {
            let Some(child) = containers
                .get_mut(key >> 16)
                .and_then(|container| container.children.get_mut(key & 0xFFFF))
            else {
                continue;
            };
            match row {
                Some(row) => {
                    child.id = row.id;
                    child.flags = 0;
                }
                None => {
                    child.id = 0;
                    child.flags = EMPTY_SLOT;
                }
            }
            written += 1;
        }
    }
    written
}

/// Runs the fill once per loaded level, once the load is finished and the
/// level's records are in the catalogue: the placed containers, in the level database's record
/// order, are filled and written back into their sections.
pub(crate) fn fill_level_containers(
    level: Option<Res<exa_world::LevelLocalItems>>,
    catalogue: Option<Res<ItemCatalogue>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut objects: Query<(&exa_world::PlacedItem, &mut exa_world::ObjectSections)>,
    mut done: Local<bool>,
) {
    let (Some(level), Some(catalogue)) = (level, catalogue) else {
        return;
    };
    if level.is_changed() {
        *done = false;
    }
    // The placed objects stream in while the level loads; the fill waits
    // for the load to finish.
    if *done || loading.is_some() || catalogue.is_empty() || objects.is_empty() {
        return;
    }
    *done = true;
    let order = |tagged: u32| {
        (tagged & LOCAL != 0)
            .then(|| {
                level
                    .0
                    .iter()
                    .position(|item| item.id & ID_MASK == tagged & ID_MASK)
            })
            .flatten()
            .unwrap_or(usize::MAX)
    };
    let mut placed: Vec<(usize, u32, Container)> = objects
        .iter()
        .filter_map(|(item, sections)| {
            let container = sections.container.clone()?;
            Some((order(item.tagged_id), item.tagged_id, container))
        })
        .collect();
    placed.sort_by_key(|(at, id, _)| (*at, *id));
    let mut containers: Vec<Container> = placed.iter().map(|(_, _, c)| c.clone()).collect();
    let written = fill_containers(&mut containers, catalogue.as_ref(), FILL_SEED);
    for ((_, id, _), filled) in placed.iter().zip(containers) {
        for (item, mut sections) in &mut objects {
            if item.tagged_id == *id {
                sections.container = Some(filled.clone());
            }
        }
    }
    info!(
        "containers: {written} table children filled in {} placed containers",
        placed.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::containers::{Child, RandomTable, TableEntry};
    use std::collections::HashMap;

    struct Tables(HashMap<u32, RandomTable>, HashMap<u32, u8>);

    impl TableSource for Tables {
        fn table(&self, id: u32) -> Option<RandomTable> {
            self.0.get(&id).cloned()
        }
        fn bulk(&self, id: u32) -> Option<u8> {
            self.1.get(&id).copied()
        }
    }

    fn row(id: u32, weight: u8) -> TableEntry {
        TableEntry {
            id,
            weight,
            decay: 0,
            quota: 0,
            limit: 0,
            flags: 0,
            condition: 0,
            bulk: 0,
        }
    }

    fn chest(children: &[u32]) -> Container {
        Container {
            width: 60,
            height: 40,
            capacity: 400,
            max_item: 60,
            children: children
                .iter()
                .enumerate()
                .map(|(i, &id)| Child {
                    id,
                    flags: 0x2020202,
                    x: i as i16 * 100,
                    y: -50,
                })
                .collect(),
        }
    }

    /// A table child takes the row's item with zero flags and keeps its
    /// place; a plain item child is left; a table with only a zero row
    /// writes the empty slot.
    #[test]
    fn table_children_take_their_rows() {
        let source = Tables(
            HashMap::from([
                (
                    LOCAL | 0x21,
                    RandomTable {
                        min: 0,
                        max: 0,
                        entries: vec![row(0x3cf, 10)],
                    },
                ),
                (
                    0x99,
                    RandomTable {
                        min: 0,
                        max: 0,
                        entries: vec![row(0, 10)],
                    },
                ),
            ]),
            HashMap::new(),
        );
        let mut containers = vec![chest(&[LOCAL | 0x21, 7]), chest(&[0x99])];
        assert_eq!(fill_containers(&mut containers, &source, FILL_SEED), 2);
        let first = &containers[0].children;
        assert_eq!((first[0].id, first[0].flags), (0x3cf, 0));
        assert_eq!((first[0].x, first[0].y), (0, -50));
        assert_eq!((first[1].id, first[1].flags), (7, 0x2020202));
        let second = &containers[1].children[0];
        assert_eq!((second.id, second.flags), (0, EMPTY_SLOT));
        // Nothing to draw writes nothing.
        let mut plain = vec![chest(&[7])];
        assert_eq!(fill_containers(&mut plain, &source, FILL_SEED), 0);
    }

    /// exanima01's placed containers filled from the shipped tables: no
    /// child is left naming a table, and the chest of drawers' five
    /// children become items with models (skipped without the game).
    #[test]
    fn the_first_levels_containers_fill_with_items() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(game) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let Some(pack) = game.pack(exa_assets::PackKind::Resource) else {
            return;
        };
        let (Some(level), Some(db)) = (pack.get("exanima01.rfc"), pack.get("objdb.rdb")) else {
            return;
        };
        let scene = exa_formats::rfc::Scene::parse(level).unwrap();
        let db = exa_formats::rdb::Database::parse(db).unwrap();
        let mut catalogue = ItemCatalogue::of_database(&db);
        catalogue.set_level_items(&scene.local_items);
        let mut ids = Vec::new();
        let mut containers = Vec::new();
        for item in &scene.local_items {
            let placed = scene
                .placed_objects
                .iter()
                .any(|o| o.slot == 0 && o.is_local() && o.id & ID_MASK == item.id);
            if let Some(c) = item.container.clone().filter(|_| placed) {
                ids.push(item.id);
                containers.push(c);
            }
        }
        let written = fill_containers(&mut containers, &catalogue, FILL_SEED);
        let tables = containers
            .iter()
            .flat_map(|c| &c.children)
            .filter(|c| catalogue.table(c.id).is_some())
            .count();
        let drawers = &containers[ids.iter().position(|&id| id == 0x54).unwrap()];
        let models: Vec<Option<String>> = drawers
            .children
            .iter()
            .map(|c| catalogue.model(c.id).map(|m| m.model.clone()))
            .collect();
        // 292 table children are drawn; the chest of drawers' "Clothes"
        // tables give boots, a shirt, a belt and trousers.
        assert_eq!((written, tables), (292, 0));
        let names: Vec<&str> = models.iter().map(|m| m.as_deref().unwrap_or("")).collect();
        assert_eq!(
            names,
            ["Boots_B", "Boots_B", "Shirty_A", "Belt_A", "Trouser_A"]
        );
        assert!(drawers.children.iter().all(|c| c.flags == 0));
    }
}
