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
    for r in db.base.records() {
        if let Some(folder) = r.folder() {
            // The first entry is the folder's own header: id 0.
            assert!(folder.entries.first().is_some_and(|e| e.id == 0));
        }
    }
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
