//! The archive pass: what the importer's hook runs over a level read from
//! the archive (a fresh visit, not a save's own section) before anything
//! reads its records. Its first step is the tier migration; then the
//! world-load walk goes through the level's own records in table order:
//!
//! - a record with an item body whose armour type is not zero rolls the
//!   shared item palette for its palette byte's wealth plus signed variance
//!   (an apparel palette, kind 1) and is refreshed from its template: the
//!   template's value word, its paint seed when the copy has none, its
//!   protection bytes, the roll of the paint layers still to be coloured,
//!   the armour variant still to be rolled and the condition within its
//!   spreads, and the condition pass;
//! - else a weapon record is refreshed: the template's weapon bytes and,
//!   for a factory model, its condition roll, the factory's stats and the
//!   condition pass;
//! - containers naming random tables are filled after the walk
//!   ([`crate::container_fill`], which runs once the level has loaded).
//!
//! So a level's own apparel (what its characters wear and what lies in it)
//! takes the colours, variants and wear the original gives it on a new
//! visit. The records the pass changes are written back into the parsed
//! level, so the placed characters are dressed from them, and the local
//! tier the live database takes is this one (already migrated).
//!
//! [Exarust stand-in: the original reseeds its shared generator from the
//! clock at every palette, layer and weapon roll, so each new game rolls
//! anew; Exarust's lockstep needs the same level to load the same way, so
//! the entropy words are a fixed sequence from [`ARCHIVE_SEED`].]
//!
//! Evidence: the archive hook the save unit installs for an archive level,
//! the world-load walk and its apparel and weapon walkers, the palette roll
//! (`context/re/item-gui-lane.md`, Item copies and Random tables).

use exa_assets::{GameData, PackKind};
use exa_formats::containers::{Container, RandomTable};
use exa_formats::objdb::{ObjectDb, Store};
use exa_formats::rfc::Scene;
use exa_items::palette::Palette;
use exa_items::randomise::{Fill, TableSource, WorldSource, section_range, world_load};

use crate::shop_stock::{ShopFactories, refresh_apparel_copy, refresh_weapon_copy, template_of};

/// The first entropy word of a level's archive pass. [Exarust stand-in for
/// the clock the original reads at each roll.]
pub const ARCHIVE_SEED: u32 = 0x00A7_C41E;

const BODY: u32 = 0x1000;
const WEAPON: u32 = 0x4000;

/// The walk's view of the level's local store: the base tier for the
/// templates, the shared palette, the entropy words and the factories.
struct ArchiveWorld<'a> {
    store: &'a mut Store,
    base: &'a ObjectDb,
    game: Option<&'a GameData>,
    palette: Palette,
    clock: u32,
    /// The shared seed the last roll left.
    seed: u32,
    factories: ShopFactories,
    refreshed: usize,
}

impl ArchiveWorld<'_> {
    /// The next entropy word. [Exarust stand-in: a fixed sequence.]
    fn entropy(&mut self) -> u32 {
        self.clock = self.clock.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.clock
    }

    fn payload(&self, id: u32) -> Option<Vec<u8>> {
        self.store.find(id).map(|r| r.payload().to_vec())
    }
}

/// Section tags of a payload, in order.
fn tags(payload: &[u8]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(head) = payload.get(at..at + 8) {
        let tag = u32::from_le_bytes(head[..4].try_into().expect("4 bytes"));
        let size = u32::from_le_bytes(head[4..].try_into().expect("4 bytes")) as usize;
        if size < 8 || at + size > payload.len() {
            break;
        }
        out.push(tag);
        at += size;
    }
    out
}

impl WorldSource for ArchiveWorld<'_> {
    fn ids(&self) -> Vec<u32> {
        self.store.entries().iter().map(|e| e.id).collect()
    }

    fn sections(&self, id: u32) -> Vec<u32> {
        self.payload(id).map(|p| tags(&p)).unwrap_or_default()
    }

    fn body_byte(&self, id: u32, offset: usize) -> Option<u8> {
        let payload = self.store.find(id)?.payload();
        let body = section_range(payload, BODY)?;
        payload[body].get(offset).copied()
    }

    /// The fill of the level's containers runs once the level has loaded
    /// ([`crate::container_fill`]), so the walk lists none.
    fn container(&self, _id: u32) -> Option<Container> {
        None
    }

    fn refresh(&mut self, id: u32) {
        let Some(mut payload) = self.payload(id) else {
            return;
        };
        let Some(template) = template_of(&payload)
            .filter(|&t| t != 0)
            .and_then(|t| self.base.find(t))
            .map(|r| r.payload().to_vec())
        else {
            return;
        };
        let has = |tag| section_range(&payload, tag).is_some();
        let entropy = self.entropy();
        let mut seed = self.seed;
        if has(BODY) {
            refresh_apparel_copy(&mut payload, &template, &self.palette, entropy, &mut seed);
        } else if has(WEAPON) {
            let (game, factories) = (self.game, &mut self.factories);
            let mut factory = |name: &str, body: &mut [u8]| factories.pass_in(game, name, body);
            refresh_weapon_copy(&mut payload, &template, entropy, &mut seed, &mut factory);
        } else {
            return;
        }
        self.seed = seed;
        if let Some(at) = self.store.find_index(id) {
            self.store.store(at, &payload);
            self.refreshed += 1;
        }
    }

    /// An apparel palette (kind 1) for the record's wealth, as the walk
    /// asks before each apparel refresh.
    fn randomise_palette(&mut self, wealth: f32) {
        let entropy = self.entropy();
        self.seed = self.palette.roll(wealth, 1, entropy);
    }
}

/// The walk lists no containers, so it reads no tables.
struct NoTables;

impl TableSource for NoTables {
    fn table(&self, _id: u32) -> Option<RandomTable> {
        None
    }
    fn bulk(&self, _id: u32) -> Option<u8> {
        None
    }
}

/// Migrates and walks a level's local store over the base tier `base`;
/// returns how many records the walk refreshed.
pub fn run(store: &mut Store, base: &ObjectDb, game: Option<&GameData>) -> usize {
    base.migrate_store(store);
    let mut world = ArchiveWorld {
        store,
        base,
        game,
        palette: Palette::default(),
        clock: ARCHIVE_SEED,
        seed: 0,
        factories: ShopFactories::default(),
        refreshed: 0,
    };
    world_load(&mut world, &NoTables, &mut Fill::default());
    world.refreshed
}

/// The importer's hook for a level from the archive
/// ([`exa_world::set_archive_pass`]): the pass over the level's local store,
/// then every record it changed written back into the parsed level's
/// records and items. False (nothing done) without a local store or the
/// object database.
pub fn archive_pass(game: &GameData, scene: &mut Scene) -> bool {
    let Some(pack) = game.pack(PackKind::Resource) else {
        return false;
    };
    let Some(objdb) = pack.get("objdb.rdb") else {
        return false;
    };
    let strings = pack.get("objstrings.rdb").unwrap_or_default();
    let Ok(base) = ObjectDb::parse(objdb, strings) else {
        return false;
    };
    let Some(store) = scene.local_database.as_mut() else {
        return false;
    };
    let before: Vec<Vec<u8>> = store.records().map(|r| r.payload().to_vec()).collect();
    let refreshed = run(store, &base, Some(game));
    let mut changed = 0;
    for (at, old) in before.iter().enumerate() {
        let Some(record) = store.view(at) else {
            continue;
        };
        if record.payload() == old.as_slice() {
            continue;
        }
        changed += 1;
        let id = record.id;
        if let Some(slot) = scene.local_records.iter_mut().find(|(rid, _)| *rid == id) {
            slot.1 = record.payload().to_vec();
        }
        if let Ok(item) = record.item()
            && let Some(slot) = scene.local_items.iter_mut().find(|i| i.id == id)
        {
            *slot = item;
        }
    }
    bevy::log::info!(
        "archive pass: {refreshed} records refreshed, {changed} of {} changed",
        before.len()
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend((body.len() as u32 + 8).to_le_bytes());
        v.extend_from_slice(body);
        v
    }

    /// An apparel body with one cloth layer still to be coloured (mode 1,
    /// the palette's) and its variant still to be rolled.
    fn apparel_body(armour: u8) -> Vec<u8> {
        let mut body = vec![0u8; 0x224];
        body[0x1A8] = armour;
        body[0x1A9] = 2; // wealth 2
        body[0x1AA] = 0xFF;
        body[0x1B0] = 200;
        let layer = 0x1B4;
        body[layer] = 1; // flags
        body[layer + 1] = 1; // mask
        body[layer + 2] = 3; // set
        body[layer + 4] = 1; // cloth
        body[layer + 0xB] = 1;
        body
    }

    fn record(parent: u32, body: Vec<u8>) -> Vec<u8> {
        let mut header = b"Tunic".to_vec();
        header.resize(16, 0);
        header.extend(0u32.to_le_bytes());
        header.extend(parent.to_le_bytes());
        let mut payload = section(0x1000_0000, &header);
        payload.extend(section(BODY, &body));
        payload
    }

    fn body(store: &Store, id: u32) -> Vec<u8> {
        let p = store.find(id).unwrap().payload().to_vec();
        let r = section_range(&p, BODY).unwrap();
        p[r].to_vec()
    }

    #[test]
    fn the_walk_colours_and_rolls_a_level_apparel_record() {
        let template = record(0, apparel_body(1));
        let plain_template = record(0, apparel_body(0));
        let mut base = ObjectDb::default();
        base.base = Store::from_records([
            (7, 0x1000_1000, template.as_slice()),
            (8, 0x1000_1000, plain_template.as_slice()),
        ]);
        let copy = record(7, apparel_body(1));
        // The migration takes the armour type from the template, so a copy
        // of a template without one is not walked.
        let plain = record(8, apparel_body(1));
        let mut store = Store::from_records([
            (1, 0x1000_1000, copy.as_slice()),
            (2, 0x1000_1000, plain.as_slice()),
        ]);
        assert_eq!(run(&mut store, &base, None), 1);
        let rolled = body(&store, 1);
        // The layer is coloured from the palette and fixed (mode 5); the
        // variant is rolled.
        assert_eq!(rolled[0x1B4 + 0xB], 5);
        assert_ne!(rolled[0x1AA], 0xFF);
        // The same level rolls the same way.
        let mut again = Store::from_records([(1, 0x1000_1000, copy.as_slice())]);
        run(&mut again, &base, None);
        assert_eq!(body(&again, 1), rolled);
        // A body without an armour type is left to the migration alone.
        let plain = body(&store, 2);
        assert_eq!((plain[0x1A8], plain[0x1B4 + 0xB]), (0, 1));
    }

    /// Over the shipped levels: every apparel record the walk reaches is
    /// left with no layer still to colour and no variant still to roll,
    /// and the same level walks the same way twice. Prints the counts
    /// (`cargo test -p exa_game shipped_levels_walk -- --nocapture`).
    #[test]
    fn the_shipped_levels_walk_the_same_way_twice() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(game) = GameData::open(&dir) else {
            return;
        };
        let resource = game.pack(PackKind::Resource).expect("a resource pack");
        for level in ["exanima01.rfc", "exanima02.rfc", "arenahub.rfc"] {
            let parse = || Scene::parse(resource.get(level).expect("the level")).unwrap();
            let (mut a, mut b) = (parse(), parse());
            let raw = a.local_records.clone();
            assert!(archive_pass(&game, &mut a));
            assert!(archive_pass(&game, &mut b));
            assert_eq!(a.local_records, b.local_records, "{level}");
            assert_eq!(a.local_items, b.local_items, "{level}");
            let (mut apparel, mut unrolled, mut unvaried, mut changed) = (0, 0, 0, 0);
            for ((_, before), (_, after)) in raw.iter().zip(&a.local_records) {
                changed += usize::from(before != after);
                let Some(r) = section_range(after, BODY) else {
                    continue;
                };
                let Some(block) = exa_items::palette::ApparelBlock::of_body(&after[r]) else {
                    continue;
                };
                if block.armour == 0 {
                    continue;
                }
                apparel += 1;
                unrolled += usize::from(
                    block
                        .layers
                        .iter()
                        .any(|l| l.mask != 0 && l.set != 0 && l.mode != 5),
                );
                unvaried += usize::from(block.variant == 0xFF);
            }
            println!(
                "{level}: {} records, {changed} changed, {apparel} apparel, {unrolled} with layers to colour, {unvaried} with a variant to roll",
                raw.len()
            );
            assert_eq!((unrolled, unvaried), (0, 0), "{level}");
            // The placed characters whose worn pieces the pass changed.
            let global = exa_formats::objdb::items(
                &exa_formats::rdb::Database::parse(resource.get("objdb.rdb").unwrap()).unwrap(),
            );
            let before = Scene::parse(resource.get(level).unwrap()).unwrap();
            let old = exa_world::placed_characters(&before, &global, None, &[]);
            let new = exa_world::placed_characters(&a, &global, None, &[]);
            for (o, n) in old.iter().zip(&new) {
                let differ = o
                    .worn
                    .iter()
                    .zip(&n.worn)
                    .filter(|(x, y)| x.appearance != y.appearance)
                    .count();
                let recoloured = o
                    .worn
                    .iter()
                    .zip(&n.worn)
                    .filter(|(x, y)| {
                        let colours = |w: &exa_world::Worn| {
                            w.appearance
                                .map(|a| a.layers.map(|l| (l.material, l.colour)))
                        };
                        colours(x) != colours(y)
                    })
                    .count();
                if differ > 0 {
                    println!(
                        "  {} at {:.1},{:.1},{:.1} facing {:.0}: {differ} of {} worn pieces changed, {recoloured} recoloured",
                        n.name,
                        n.position.x,
                        n.position.y,
                        n.position.z,
                        n.yaw.to_degrees(),
                        n.worn.len()
                    );
                }
            }
        }
    }
}
