//! Checks against the installed game's object databases (skipped without
//! `EXANIMA_DIR`): `objdb.rdb` with `objstrings.rdb`, every level's local
//! database and string library, and every arena actor's database.

use super::*;
use crate::objstrings::Library;
use std::collections::BTreeMap;

/// Every `(name, database, library)` of the levels in `Resource.rpk`; the
/// library is empty when a level has none.
fn level_databases(resource: &exa_rpk::MappedArchive) -> Vec<(String, Vec<u8>, Vec<u8>)> {
    let mut out = Vec::new();
    for e in resource.entries() {
        let Ok(chunks) = crate::chunk::rfc_chunks(resource.bytes(e)) else {
            continue;
        };
        for c in chunks.flatten() {
            if c.tag != crate::rfc::LOCAL_DATABASE {
                continue;
            }
            let (mut db, mut strings) = (None, Vec::new());
            let mut pos = 0;
            while pos + 8 <= c.data.len() {
                let tag = crate::read_u32(c.data, pos).unwrap();
                let size = crate::read_u32(c.data, pos + 4).unwrap() as usize;
                let body = &c.data[pos + 8..pos + 8 + size];
                match tag {
                    0xDBDB => db = Some(body.to_vec()),
                    0xDBCD => strings = body.to_vec(),
                    _ => {}
                }
                pos += 8 + size;
            }
            if let Some(db) = db {
                out.push((e.name.clone(), db, strings));
            }
        }
    }
    out
}

/// Container bodies hold a count at `0x14` and 12-byte entries from
/// `0x18`.
fn check_containers(name: &str, store: &Store) -> usize {
    let mut n = 0;
    for r in store.records() {
        if let Some(b) = r.section(CONTAINER) {
            let count = u32_at(b, 0x14) as usize;
            assert_eq!(b.len(), 0x18 + 12 * count, "{name}: record {}", r.id);
            n += 1;
        }
    }
    n
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

#[test]
fn shipped_object_database() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let file = resource.get("objdb.rdb").unwrap();
    let db = ObjectDb::parse(file, resource.get("objstrings.rdb").unwrap()).unwrap();
    assert_eq!(db.base.len(), 1423);
    assert_eq!(db.strings.len(), 1124);
    // The original's writer would save it as it was read.
    assert_eq!(db.write(Tier::Base), file);
    assert_eq!(db.name(2).as_deref(), Some("Leather Gloves"));
    assert_eq!(
        db.description(2).as_deref(),
        Some("A pair of thick leather gauntlets.")
    );
    assert_eq!(db.name(31).as_deref(), Some("Torch"));
    assert_eq!(db.description(31).as_deref(), Some("A simple torch."));
    // Record 8 (the braies) has no ground model; 425 does.
    assert!(db.find(8).unwrap().model().is_none());
    assert!(db.find(8).unwrap().has_body());
    assert_eq!(db.find(425).unwrap().model().unwrap().name, "Braies_A");
    let mut tags: BTreeMap<u32, usize> = BTreeMap::new();
    let mut flags_differ = Vec::new();
    let mut unresolved = 0;
    for r in db.base.records() {
        // Every record starts with its header, so the original's direct
        // reads at payload 0x18 and 0x1C are its words.
        let h = r.header().unwrap();
        assert_eq!((r.header_word(), r.parent_word()), (h.flags, h.parent));
        let mut or = 0;
        for s in r.sections() {
            *tags.entry(s.tag).or_default() += 1;
            or |= s.tag;
        }
        if or != r.flags {
            flags_differ.push(h.name.clone());
        }
        if let Some(m) = r.model() {
            for raw in [m.name_id, m.description_id] {
                if raw != 0 && raw != INHERIT && db.strings.bytes(raw & ID_MASK).is_none() {
                    unresolved += 1;
                }
            }
        }
    }
    assert_eq!(flags_differ, ["Map"]);
    assert_eq!(unresolved, 0);
    let expected: [(u32, usize); 11] = [
        (0x1000_0000, 1423),
        (0x0010_0000, 1403),
        (0x1000, 388),
        (0x4000, 208),
        (0x2_0000, 137),
        (0x1_0000, 49),
        (0x4_0000, 47),
        (0x200, 24),
        (0x8000, 22),
        (0x100, 1),
        (0x8000_0B03, 1),
    ];
    assert_eq!(tags.len(), expected.len());
    for (tag, count) in expected {
        assert_eq!(tags.get(&tag).copied(), Some(count), "tag {tag:#x}");
    }
    assert_eq!(check_containers("objdb", &db.base), 137);
    let tables: Vec<_> = db
        .base
        .records()
        .filter_map(|r| r.section(RANDOM_TABLE))
        .map(|b| crate::containers::RandomTable::parse(b).unwrap())
        .collect();
    assert_eq!(tables.len(), 24);
    // The first row of every table is the empty row, id 0.
    assert!(
        tables
            .iter()
            .all(|t| t.entries.first().is_some_and(|e| e.id == 0))
    );
}

#[test]
fn shipped_level_databases() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let mut base = ObjectDb::parse(
        resource.get("objdb.rdb").unwrap(),
        resource.get("objstrings.rdb").unwrap(),
    )
    .unwrap();
    let levels = level_databases(resource);
    assert_eq!(levels.len(), 22);
    let (mut records, mut own, mut containers) = (0, 0, 0);
    for (name, file, strings) in &levels {
        let store = Store::parse(file).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(&store.write(), file, "{name}");
        containers += check_containers(name, &store);
        records += store.len();
        base.local = store;
        base.local_strings = Library::parse(strings).unwrap();
        // Every own string a level record names is in the level's library,
        // and every name and description resolves without panicking.
        for r in base.local.records() {
            if let Some(m) = r.model() {
                for raw in [m.name_id, m.description_id] {
                    if raw > INHERIT {
                        own += 1;
                        assert!(
                            base.local_strings.bytes(raw & ID_MASK).is_some(),
                            "{name}: record {} string {raw:#x}",
                            r.id
                        );
                    }
                }
            }
            base.name(LOCAL | r.id);
            base.description(LOCAL | r.id);
        }
        // Migrating, instancing and promoting the whole level stays total.
        let mut copy = base.clone();
        assert_eq!(copy.migrate_tier(Tier::Local), copy.local.len());
        copy.set_remaps(true);
        copy.set_link_remaps(true);
        let ids: Vec<u32> = copy.local.records().map(|r| r.id | LOCAL).collect();
        for id in ids {
            copy.promote(id);
        }
        assert_eq!(copy.local.records().count(), 0, "{name}");
        copy.apply_link_remaps();
        let written = copy.write(Tier::World);
        assert_eq!(Store::parse(&written).unwrap().write(), written);
    }
    assert_eq!(records, 3812);
    assert_eq!(own, 119);
    assert_eq!(containers, 1283);
}

#[test]
fn shipped_actor_databases() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let actors = exa_rpk::Archive::parse(resource.get("actors.rpk").unwrap()).unwrap();
    let (mut databases, mut older) = (0, 0);
    for e in actors.entries() {
        let b = actors.bytes(e);
        let size = u32_at(b, 4) as usize;
        let rest = &b[8 + size..];
        if rest.is_empty() {
            continue;
        }
        assert_eq!(u32_at(rest, 0), 0xDBDB, "{}", e.name);
        let len = u32_at(rest, 4) as usize;
        let file = &rest[8..8 + len];
        let store = Store::parse(file).unwrap_or_else(|err| panic!("{}: {err}", e.name));
        // No actor database has an operative section, so the upgrade of
        // the older layout changes nothing but the magic it is saved with.
        let mut expected = file.to_vec();
        expected[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        assert_eq!(store.write(), expected, "{}", e.name);
        if u32_at(file, 0) == 0xDBAC_0D08 {
            older += 1;
        }
        databases += 1;
    }
    assert_eq!((databases, older), (20, 17));
}

/// The transition markers the levels' own object databases carry: 54
/// volumes whose flags' low byte is 0x40, each naming the level it connects
/// back to (the record's name) and the entrance. `exanima01`'s single
/// marker leads back from `exanima02` (locale id 3) at entrance 0.
#[test]
fn shipped_level_transition_markers() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let levels = level_databases(resource);
    let mut markers = Vec::new();
    for (name, file, _) in &levels {
        let db = Database::parse(file).unwrap();
        for item in items(&db) {
            if let Some(volume) = item.volume {
                if let Some(words) = volume.marker() {
                    markers.push((name.clone(), item.name, words, volume.flags, volume.extents));
                }
            }
        }
    }
    assert_eq!(markers.len(), 54);
    let first = &markers[0];
    assert_eq!(first.0, "exanima01.rfc");
    assert_eq!(first.1, "Exanima02");
    assert_eq!(first.2, [0, 3]);
    assert_eq!(first.3, 0x1040);
    assert_eq!(first.4, [50, 50, 50]);
    // Different entrances into the same level are separate markers.
    assert!(
        markers
            .iter()
            .any(|m| m.0 == "exanima05_sw.rfc" && m.1 == "CrossRoads_2" && m.2 == [2, 7])
    );
    // A marker whose flags word is the shorter 0x40 form (an older level's
    // record).
    assert!(
        markers
            .iter()
            .any(|m| m.0 == "exanima08.rfc" && m.3 == 0x40)
    );
}
