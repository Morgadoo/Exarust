//! The world-load fill of the loaded level's placed containers: a container
//! child naming a random table (the shipped chests' children are all such
//! "DropTable" records) is replaced by the row the table gives it, as the
//! original's world load does before any container is opened. Without it a
//! chest's popup shows the table records themselves, drawn with the table's
//! placeholder model.
//!
//! The draws are [`exa_items::randomise::Fill`]'s (quotas, count draws,
//! weighted draws, nested tables, the slot's item limit). The children keep
//! their places; an empty slot gets id 0 with flags `0x100`, and a picked
//! child the row's item as the original's container instantiate makes it
//! ([`ContainerPicks`]): the item goes through the object database's
//! dispatcher (an instance-on-use record is copied into the local tier),
//! then the row's condition word is applied: an item body takes the palette
//! roll on the row's wealth, its count byte's signed fraction, damage and
//! dirt with their spreads and the apparel walker from its template; a
//! tagged weapon copy damage and dirt with their spreads and the weapon
//! walker; the child takes the result's id with zero flags.
//! Exarust stand-ins: the original's walk covers the whole database in
//! record order from the engine's shared generator; here only the placed
//! containers are walked (in the level database's record order), the
//! generator starts from [`FILL_SEED`] and the walkers' clock words are a
//! fixed sequence from it, as the archive pass's are.

use bevy::prelude::*;
use exa_assets::GameData;
use exa_formats::containers::{Container, RowCondition, TableEntry};
use exa_formats::objdb::{ID_MASK, LOCAL, ObjectDb};
use exa_items::catalogue::ItemCatalogue;
use exa_items::palette::Palette;
use exa_items::randomise::{Fill, PickStore, TableSource, apply_row, section_range};

use crate::shop_stock::{ShopFactories, refresh_apparel_copy, refresh_weapon_copy, template_of};

const BODY: u32 = 0x1000;
const WEAPON: u32 = 0x4000;
/// The id tags of the world and the local tier.
const TAGGED: u32 = 0xC000_0000;

/// The generator's start for a level's fill. [Exarust stand-in: the
/// original draws from the engine's shared generator, whose state at the
/// world load Exarust does not keep.]
pub const FILL_SEED: u32 = 0x0080_b71a;
/// The flags an empty slot is written with.
pub const EMPTY_SLOT: u32 = 0x100;

/// The original's container instantiate over the live object database:
/// the picked row's item through the dispatcher, then the row's condition
/// word and the walker of what the copy is.
pub struct ContainerPicks<'a> {
    pub db: &'a mut ObjectDb,
    pub game: Option<&'a GameData>,
    pub palette: Palette,
    /// The walkers' clock word. [Exarust stand-in: a fixed sequence.]
    pub clock: u32,
    /// The shared seed the last roll left.
    pub seed: u32,
    pub factories: ShopFactories,
}

impl<'a> ContainerPicks<'a> {
    /// Picks into `db`, the walkers' clock starting from `clock`.
    pub fn new(db: &'a mut ObjectDb, game: Option<&'a GameData>, clock: u32) -> Self {
        Self {
            db,
            game,
            palette: Palette::default(),
            clock,
            seed: 0,
            factories: ShopFactories::default(),
        }
    }

    /// The next clock word. [Exarust stand-in: a fixed sequence.]
    fn entropy(&mut self) -> u32 {
        self.clock = self.clock.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.clock
    }
}

impl PickStore for ContainerPicks<'_> {
    fn pick(&mut self, row: u32, condition: u32) -> Option<u32> {
        let copy = self.db.instantiate_or_duplicate(row);
        let Some(mut payload) = self.db.find(copy).map(|r| r.payload().to_vec()) else {
            return Some(copy);
        };
        let template = template_of(&payload)
            .filter(|&t| t != 0)
            .and_then(|t| self.db.find(t))
            .map(|r| r.payload().to_vec());
        if section_range(&payload, BODY).is_some() {
            // The palette roll on the row's wealth comes first; the body
            // is written whether or not the dispatcher made a copy.
            let wealth = RowCondition::of(condition).wealth;
            let entropy = self.entropy();
            self.seed = self.palette.roll(wealth, 1, entropy);
            apply_row(&mut payload, condition);
            if let Some(template) = &template {
                let entropy = self.entropy();
                let mut seed = self.seed;
                refresh_apparel_copy(&mut payload, template, &self.palette, entropy, &mut seed);
                self.seed = seed;
            }
        } else if copy & TAGGED != 0 && section_range(&payload, WEAPON).is_some() {
            apply_row(&mut payload, condition);
            if let Some(template) = &template {
                let entropy = self.entropy();
                let mut seed = self.seed;
                let (game, factories) = (self.game, &mut self.factories);
                let mut factory = |name: &str, body: &mut [u8]| factories.pass_in(game, name, body);
                refresh_weapon_copy(&mut payload, template, entropy, &mut seed, &mut factory);
                self.seed = seed;
            }
        } else {
            return Some(copy);
        }
        self.db.set_payload(copy, &payload);
        Some(copy)
    }
}

/// Fills the table children of `containers`, in order, from one shared
/// fill, a picked child taking the row's item as the row's own id.
/// Returns how many children were written.
pub fn fill_containers(containers: &mut [Container], source: &dyn TableSource, seed: u32) -> usize {
    fill_containers_with(containers, source, seed, &mut |row: TableEntry| row.id)
}

/// [`fill_containers`] with each picked row made by `pick` (the
/// container instantiate), in the order the draws give them.
pub fn fill_containers_with(
    containers: &mut [Container],
    source: &dyn TableSource,
    seed: u32,
    pick: &mut dyn FnMut(TableEntry) -> u32,
) -> usize {
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
                    child.id = pick(row);
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
/// order, are filled and written back into their sections and into their
/// records in the live local tier. A level read from a save's own section
/// is not filled: the original's import of a saved section runs only the
/// tier migration, and the containers keep what the save stored.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fill_level_containers(
    level: Option<Res<exa_world::LevelLocalItems>>,
    catalogue: Option<Res<ItemCatalogue>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    source: Option<Res<exa_world::LevelSource>>,
    game: Option<Res<exa_assets::Game>>,
    mut live: Option<ResMut<exa_assets::live_objects::LiveObjects>>,
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
    if source.as_ref().is_some_and(|source| source.from_save) {
        info!("containers: the level came from the save; its containers are not filled");
        return;
    }
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
    // Each picked row goes through the container instantiate over the live
    // database; without one (no game data) the child takes the row's own
    // id. [Exarust guard]
    let written = match live.as_mut().filter(|live| !live.db().base.is_empty()) {
        Some(live) => {
            let game = game.as_deref().map(|game| &*game.0);
            let mut picks = ContainerPicks::new(live.db_mut(), game, FILL_SEED);
            fill_containers_with(&mut containers, catalogue.as_ref(), FILL_SEED, &mut |row| {
                picks.pick(row.id, row.condition).unwrap_or(row.id)
            })
        }
        None => fill_containers(&mut containers, catalogue.as_ref(), FILL_SEED),
    };
    for ((_, id, _), filled) in placed.iter().zip(containers) {
        for (item, mut sections) in &mut objects {
            if item.tagged_id == *id {
                sections.container = Some(filled.clone());
            }
        }
        // The fill writes the level's own record, which a save then
        // carries in its level section.
        if id & exa_formats::objdb::TAG_MASK == LOCAL
            && let Some(live) = live.as_mut()
        {
            live.set_container_children(*id, &filled.children);
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

    fn section(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend((body.len() as u32 + 8).to_le_bytes());
        v.extend_from_slice(body);
        v
    }

    /// A header body: the name, the word (bit 0: instanced on use) and the
    /// parent.
    fn header(name: &str, word: u32) -> Vec<u8> {
        let mut b = name.as_bytes().to_vec();
        b.resize(16, 0);
        b.extend(word.to_le_bytes());
        b.extend(0u32.to_le_bytes());
        b
    }

    /// A random table body with one row of `id`, weight 10 and the
    /// condition word.
    fn table(id: u32, condition: u32) -> Vec<u8> {
        let mut b = vec![0u8; 0x10];
        b[0xC..0x10].copy_from_slice(&1u32.to_le_bytes());
        let mut row = vec![0u8; 0x10];
        row[0..4].copy_from_slice(&id.to_le_bytes());
        row[4] = 10;
        row[10..14].copy_from_slice(&condition.to_le_bytes());
        b.extend(row);
        b
    }

    /// The level's fill runs the container instantiate over the live
    /// database: a picked apparel record that is instanced on use becomes a
    /// local copy whose body takes the row's condition word (damage and dirt
    /// as shares of 255, their spreads in the packed byte, the count byte's
    /// signed fraction from the wealth); a picked weapon copy takes the
    /// damage and dirt and their spreads in its own bytes; the children name
    /// the copies with zero flags.
    #[test]
    fn a_picked_row_is_instanced_with_its_condition() {
        const NAME: u32 = 0x1000_0000;
        const MODEL: u32 = 0x10_0000;
        const TABLE: u32 = exa_formats::objdb::RANDOM_TABLE;
        let mut model = b"Shirt".to_vec();
        model.resize(0x28, 0);
        model[0x1C] = 5;
        let apparel = [
            section(NAME, &header("Shirt", 1)),
            section(MODEL, &model),
            section(BODY, &[0u8; 0x224]),
        ]
        .concat();
        let mut sword = b"Sword".to_vec();
        sword.resize(0x28, 0);
        sword[0x1C] = 5;
        let weapon = [
            section(NAME, &header("Sword", 1)),
            section(MODEL, &sword),
            section(WEAPON, &[0u8; 0x60]),
        ]
        .concat();
        // Wealth 1 (4 quarters), damage 0.5 (bits 6–8 = 2) spread 0.25
        // (bits 9–10 = 1), dirt 0.75 (bits 11–13 = 3) spread 0.5 (bits
        // 14–15 = 2).
        let condition = 4 | 2 << 6 | 1 << 9 | 3 << 11 | 2 << 14;
        let records = [
            (7, NAME | MODEL | BODY, apparel),
            (9, NAME | MODEL | WEAPON, weapon),
            (
                20,
                NAME | TABLE,
                [
                    section(NAME, &header("DropTable", 0)),
                    section(TABLE, &table(7, condition)),
                ]
                .concat(),
            ),
            (
                21,
                NAME | TABLE,
                [
                    section(NAME, &header("DropTable", 0)),
                    section(TABLE, &table(9, condition)),
                ]
                .concat(),
            ),
        ];
        let mut file = exa_formats::objdb::MAGIC.to_le_bytes().to_vec();
        file.extend((((records.len() + 1) * 16) as u32).to_le_bytes());
        let mut offset = 0u32;
        for (id, flags, payload) in &records {
            for w in [*id, *flags, offset, payload.len() as u32] {
                file.extend(w.to_le_bytes());
            }
            offset += payload.len() as u32;
        }
        file.extend([0; 16]);
        for (_, _, payload) in &records {
            file.extend(payload);
        }
        let catalogue =
            ItemCatalogue::of_database(&exa_formats::rdb::Database::parse(&file).unwrap());
        let mut live = exa_assets::live_objects::LiveObjects::default();
        live.db_mut().base = exa_formats::objdb::Store::parse(&file).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(catalogue)
            .insert_resource(live)
            .insert_resource(exa_world::LevelLocalItems(Vec::new()))
            .add_systems(Update, fill_level_containers);
        app.world_mut().spawn((
            exa_world::PlacedItem {
                id: 50,
                tagged_id: LOCAL | 50,
                condition: 0,
            },
            exa_world::ObjectSections {
                placement: 0,
                usable: None,
                use_script: None,
                container: Some(chest(&[20, 21])),
            },
        ));
        app.update();
        let children = {
            let mut q = app.world_mut().query::<&exa_world::ObjectSections>();
            let sections = q.single(app.world()).unwrap();
            sections.container.clone().unwrap().children
        };
        assert_eq!(
            children.iter().map(|c| (c.id, c.flags)).collect::<Vec<_>>(),
            vec![(LOCAL | 1, 0), (LOCAL | 2, 0)]
        );
        let live = app
            .world()
            .resource::<exa_assets::live_objects::LiveObjects>();
        let db = live.db();
        let body = db.find(LOCAL | 1).unwrap().section(BODY).unwrap().to_vec();
        // Damage 0.5 and dirt 0.75 of 255 (128 and 191), then the apparel
        // walker rolls each within its spread.
        let spreads = exa_items::instance::Spreads(body[0x1B3]);
        assert_eq!((spreads.damage(), spreads.dirt()), (0.25, 0.5));
        assert!((128 - 64..=128 + 64).contains(&i32::from(body[0x1B1])));
        assert!((191 - 128..=255).contains(&i32::from(body[0x1B2])));
        // The count byte's signed fraction: wealth 1 less the count 0.
        assert_eq!(body[0x1A9], exa_items::randomise::adjust_paint_seed(0, 1.0));
        let weapon = db
            .find(LOCAL | 2)
            .unwrap()
            .section(WEAPON)
            .unwrap()
            .to_vec();
        assert_eq!((weapon[0xD], weapon[0xE]), (128, 191));
        let spreads = exa_items::instance::Spreads(weapon[0xF]);
        assert_eq!((spreads.damage(), spreads.dirt()), (0.25, 0.5));
        // The weapon's base record keeps its bytes.
        assert_eq!(db.base.find(9).unwrap().section(WEAPON).unwrap()[0xD], 0);
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
        // Through the container instantiate over the shipped database the
        // same draws give the same items: every picked child is a record
        // of the database (a local copy or the base record itself) naming
        // the same model once the local tier is in the catalogue.
        let mut objects = ObjectDb::parse(
            pack.get("objdb.rdb").unwrap(),
            pack.get("objstrings.rdb").unwrap_or_default(),
        )
        .unwrap();
        if let Some(records) = scene.local_database.clone() {
            objects.load_local(records, scene.local_strings.clone().unwrap_or_default());
        }
        let level_records = objects.local.len();
        let mut picked = Vec::new();
        for item in &scene.local_items {
            let placed = scene
                .placed_objects
                .iter()
                .any(|o| o.slot == 0 && o.is_local() && o.id & ID_MASK == item.id);
            if let Some(c) = item.container.clone().filter(|_| placed) {
                picked.push(c);
            }
        }
        let mut picks = ContainerPicks::new(&mut objects, Some(&game), FILL_SEED);
        let mut made = Vec::new();
        let mut rows = Vec::new();
        let again = fill_containers_with(&mut picked, &catalogue, FILL_SEED, &mut |row| {
            let id = picks.pick(row.id, row.condition).unwrap_or(row.id);
            made.push(id);
            rows.push(row.id);
            id
        });
        // exanima01's tables pick 283 rows: one names a level record, which
        // the dispatcher duplicates in the local tier; 101 picks are local
        // copies and the other 182 base records that are not instanced on use.
        assert_eq!(rows.len(), 283);
        assert_eq!(rows.iter().filter(|&&r| r & LOCAL != 0).count(), 1);
        assert_eq!(made.iter().filter(|&&m| m & LOCAL != 0).count(), 101);
        assert_eq!(again, 292);
        assert!(objects.local.len() > level_records);
        let local: Vec<exa_formats::objdb::Item> = objects
            .local
            .records()
            .filter_map(|r| {
                let mut item = r.item().ok()?;
                item.id = r.id;
                Some(item)
            })
            .collect();
        catalogue.set_tier_items(LOCAL, &local);
        let drawers = &picked[ids.iter().position(|&id| id == 0x54).unwrap()];
        let again: Vec<String> = drawers
            .children
            .iter()
            .map(|c| {
                catalogue
                    .model(c.id)
                    .map_or(String::new(), |m| m.model.clone())
            })
            .collect();
        assert_eq!(again, names);
        // Every pick is a record of the database.
        assert!(!made.is_empty());
        for id in made {
            assert!(objects.find(id).is_some(), "{id:#x}");
        }
    }
}
