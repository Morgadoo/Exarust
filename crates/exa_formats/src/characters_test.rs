//! Tests of the character records and level placements.
//! (Included from characters.rs: `#[path = "characters_test.rs"] mod tests;`)

use super::*;

fn u32s(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn f32s(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn name(s: &str) -> Vec<u8> {
    let mut n = s.as_bytes().to_vec();
    n.resize(16, 0);
    n
}

/// Tag and size around a record's bytes.
fn frame(version: u8, body: &[u8]) -> Vec<u8> {
    let mut b = u32s(&[RECORD & 0xFFFF_FF00 | u32::from(version), body.len() as u32]);
    b.extend(body);
    b
}

/// The fields every version shares, up to the worn items.
fn front(voice: &str, profession: u32, worn: &[u32]) -> Vec<u8> {
    let mut b = name("Zombie03");
    b.extend(name("Smith"));
    b.extend(f32s(&[0.25, 0.75, -0.5, 0.3])); // physique, height, age
    b.extend([0u8; 12 + 4]); // skin, hair, face styles
    b.extend(name(voice));
    b.extend(f32s(&[1.05]));
    b.extend(u32s(&[profession, 0]));
    b.extend(u32s(&[0x17, 0x100, 0, 0, 0x22, 0, 0, 0]));
    b.extend((worn.len() as i32).to_le_bytes());
    for &w in worn {
        b.extend(u32s(&[w, 0x100]));
    }
    b
}

type SkillEntry<'a> = (u16, u16, &'a [u8]);

/// Feral level to inventory: feral 0.84, pools full / half / empty, combat
/// skill 25, undead voice 3, traits 0, −50, 100, −100, 25000 experience.
fn middle(skills: &[SkillEntry<'_>], inventory: &[[u32; 3]]) -> Vec<u8> {
    let mut b = f32s(&[0.84]);
    for v in [FULL_POOL, 0x8000, 0, 0x1234] {
        b.extend(v.to_le_bytes());
    }
    b.extend(f32s(&[25.0]));
    b.extend(u32s(&[3]));
    b.extend([100, 50, 200, 0]);
    b.extend(u32s(&[0xDEAD, 25000, 0]));
    b.extend((skills.len() as i32).to_le_bytes());
    for &(slot, points, techniques) in skills {
        b.extend(slot.to_le_bytes());
        b.extend(points.to_le_bytes());
        b.push(techniques.len() as u8);
        b.extend(techniques);
    }
    b.extend((inventory.len() as i32).to_le_bytes());
    for e in inventory {
        b.extend(u32s(e));
    }
    b
}

/// Roles, no relations and no saved state: what a version 7 record needs
/// after the inventory.
fn v7_rest(roles: &[(u32, u8, u8, &[u32])]) -> Vec<u8> {
    let mut blob = Vec::new();
    for &(id, flags, inherited, vars) in roles {
        blob.extend(u32s(&[id]));
        blob.extend(f32s(&[0.5]));
        blob.extend([flags, inherited, vars.len() as u8, 0]);
        blob.extend(u32s(vars));
    }
    let mut b = u32s(&[roles.len() as u32, blob.len() as u32]);
    b.extend(blob);
    b.extend(u32s(&[0, 0, 0])); // relations, level tables, saved state
    b
}

/// A version 7 record with the given skills and nothing else of note.
fn with_skills(skills: &[SkillEntry<'_>]) -> CharacterRecord {
    let mut body = front("vi01", 0, &[]);
    body.extend(middle(skills, &[]));
    body.extend(v7_rest(&[]));
    CharacterRecord::parse(1, &frame(7, &body)).unwrap()
}

#[test]
fn whole_version_7_record() {
    let mut body = front("vi03", 0x0000_0B01, &[0x8000_0010, 0x8000_00CE]);
    body.extend(middle(
        &[(0, 15000, &[1, 4, 2]), (2, 10000, &[5, 4]), (3, 5000, &[6])],
        &[[0x8000_0010, 0x100, 0x0004_0028], [0, 0, 0]],
    ));
    // Corpse (explicit) and its parent Undead with two variables.
    let mut blob = u32s(&[8]);
    blob.extend(f32s(&[0.0]));
    blob.extend([1, 0, 0, 0]);
    blob.extend(u32s(&[2]));
    blob.extend(f32s(&[0.5]));
    blob.extend([0, 1, 2, 0]);
    blob.extend(u32s(&[7, 9]));
    body.extend(u32s(&[2, blob.len() as u32]));
    body.extend(blob);
    // One relation toward character 5, and one level table without any.
    body.extend(u32s(&[1, 5, 0, 0x14]));
    body.extend([0u8; 0x14]);
    body.extend(u32s(&[1, 0x8000_0001, 0, 0]));
    // Saved state: a heal pool of 0.25 and a potion expiring at 1800.
    body.extend(u32s(&[
        2,
        SAVED_HEAL_POOL,
        0.25f32.to_bits(),
        SAVED_POTION,
        1800,
    ]));
    // An unread sub-chunk, then the powers: two known, one with a blob.
    body.extend(u32s(&[0x00CD_EC12, 8, 0, 1]));
    let mut powers = u32s(&[7, 0x0002_020B, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
    powers.extend(u32s(&[3, 0x0001_5FFF, 4, POWER_BLOB, 12, 0xAA, 0xBB]));
    body.extend(u32s(&[POWERS[0], powers.len() as u32]));
    body.extend(powers);
    let mut b = frame(7, &body);
    b.extend(u32s(&[0xDBDB, 4, 0])); // an actor's item database follows
    let r = CharacterRecord::parse(2, &b).unwrap();
    assert_eq!((r.id, r.version), (2, 7));
    assert_eq!((r.name.as_str(), r.surname.as_str()), ("Zombie03", "Smith"));
    assert_eq!((r.physique, r.height, r.age), ([0.25, 0.75], -0.5, 0.3));
    assert!(r.female);
    assert_eq!(r.race, 11);
    assert_eq!(r.profession, 0x0000_0B01);
    assert_eq!((r.voice.as_str(), r.voice_pitch), ("vi03", 1.05));
    assert_eq!(r.hands, [0x17, 0, 0x22, 0]);
    assert_eq!(r.hand_flags, [0x100, 0, 0, 0]);
    assert_eq!(r.worn, [0x8000_0010, 0x8000_00CE]);
    assert_eq!(r.feral, 0.84);
    assert_eq!(
        r.pools,
        Pools {
            yellow: 0xFFFF,
            red: 0x8000,
            focus: 0
        }
    );
    let [yellow, red, focus] = r.pools.shares();
    assert_eq!((yellow, focus), (1.0, 0.0));
    assert!((red - 0.500_007_6).abs() < 1e-6, "{red}");
    assert_eq!((r.combat_skill, r.undead_voice), (25.0, 3));
    assert_eq!(r.traits, [0.0, -50.0, 100.0, -100.0]);
    assert_eq!(r.experience, 25000);
    assert_eq!(r.skills[CLOSE_COMBAT].points, 15000);
    assert_eq!(r.skills[CLOSE_COMBAT].techniques, [1, 4, 2, 0, 0, 0]);
    assert_eq!(r.skills[ARMOUR].learnt(), [5, 4]);
    assert_eq!(r.skills[SHIELDS].learnt(), [6]);
    assert_eq!(r.skills[RANGED_COMBAT], Skill::default());
    assert_eq!(
        r.inventory,
        [
            Carried {
                item: 0x8000_0010,
                flags: 0x100,
                extra: 0x0004_0028
            },
            Carried {
                item: 0,
                flags: 0,
                extra: 0
            }
        ]
    );
    assert_eq!(
        r.roles,
        [
            Role {
                id: 8,
                explicit: true,
                inherited: 0,
                variables: vec![]
            },
            Role {
                id: 2,
                explicit: false,
                inherited: 1,
                variables: vec![7, 9]
            }
        ]
    );
    assert_eq!(r.relations, 1);
    assert_eq!((r.heal_pool(), r.potion_expiry()), (Some(0.25), Some(1800)));
    let powers = r.powers.unwrap();
    assert_eq!((powers.word, powers.slots[0]), (7, 0x0002_020B));
    assert_eq!(
        powers.known,
        [
            Power {
                id: 3,
                flags: 0x0001_5FFF
            },
            Power {
                id: 4,
                flags: POWER_BLOB
            }
        ]
    );
}

/// The perk table of the notes: Close Combat 1–4, Armour 5, Shields 1–6,
/// Insight 6, Concentration 1, 5, 6 and 8, each only once learnt (one
/// technique per 5000 points).
#[test]
fn learnt_techniques_give_perks() {
    let r = with_skills(&[
        (0, 15000, &[1, 4, 2, 3]),
        (2, 25000, &[1, 2, 4, 3, 5]),
        (3, 30000, &[1, 2, 3, 4, 5, 6]),
        (4, 30000, &[1, 2, 3, 4, 5, 6]),
        (5, 25000, &[1, 5, 6, 8, 2]),
    ]);
    let perks: Vec<&str> = r.perks().iter().map(Perk::name).collect();
    assert_eq!(
        perks,
        [
            "Remise",
            "Feint",
            "Riposte",
            "Security",
            "Counter",
            "Synchrony",
            "Ward",
            "Persist",
            "Endure",
            "Mobility",
            "Iteration",
            "Sustain",
            "Abstraction",
            "Clarity",
            "Volition"
        ]
    );
    // Fend is the fourth Close Combat technique: 15000 points learn three.
    assert!(!r.perks().has(Perk::Fend));
    assert!(with_skills(&[(0, 4999, &[1])]).perks().is_empty());
    assert!(with_skills(&[(0, 5000, &[3])]).perks().has(Perk::Fend));
    // Dual Wield, Draw and the Armour tiers are techniques, not perks.
    assert!(with_skills(&[(0, 40000, &[8, 5, 6, 7])]).perks().is_empty());
}

#[test]
fn manoeuvring_tier_is_the_highest_learnt() {
    let tier = |points, techniques: &[u8]| {
        let r = with_skills(&[(2, points, techniques)]);
        (r.manoeuvring_tier(), r.deflection())
    };
    assert_eq!(tier(10000, &[2, 1]), (2, false));
    assert_eq!(tier(25000, &[1, 2, 4, 3, 5]), (3, true));
    assert_eq!(tier(10000, &[4, 1]), (1, true));
    assert_eq!(tier(4999, &[3]), (0, false));
    assert_eq!(tier(5000, &[5]), (0, false));
    // 65535 points would be 13 levels; only the six stored count.
    assert_eq!(tier(65535, &[1, 0, 0, 0, 0, 3]), (3, false));
    assert_eq!(
        with_skills(&[(2, 65535, &[1])]).skills[ARMOUR]
            .learnt()
            .len(),
        6
    );
}

#[test]
fn skill_slots_past_the_six_are_skipped() {
    let r = with_skills(&[(6, 5000, &[1]), (0, 5000, &[2]), (9, 10000, &[1, 2])]);
    assert_eq!(r.skills[CLOSE_COMBAT].learnt(), [2]);
    assert!(r.skills[1..].iter().all(|s| *s == Skill::default()));
}

/// Version 6: roles as {id, word} pairs added in turn, relations skipped,
/// powers as 16-byte entries.
#[test]
fn legacy_version_6_record() {
    let mut body = front("vm02", 0x0000_0100, &[0x8000_0001]);
    body.extend(middle(&[(0, 10000, &[1, 2])], &[[0x8000_0002, 0x100, 0]]));
    body.extend(u32s(&[2, 9, 0, 1, 0])); // roles Skeleton, then Human
    body.extend(u32s(&[1, 4, 0, 4, 0])); // a relation table of 4 bytes
    body.extend(u32s(&[1, 0, 0, 1, 7, 0, 0])); // a level table
    body.extend(u32s(&[1, SAVED_POTION, 99]));
    let mut powers = u32s(&[1; 11]);
    powers.extend(u32s(&[1, 5, POWER_BLOB, 0xEE, 0xEE]));
    body.extend(u32s(&[POWERS[1], powers.len() as u32]));
    body.extend(powers);
    let r = CharacterRecord::parse(3, &frame(6, &body)).unwrap();
    assert_eq!((r.version, r.race, r.female), (6, 1, false));
    assert_eq!(r.worn, [0x8000_0001]);
    assert_eq!(r.feral, 0.84);
    assert_eq!(r.inventory.len(), 1);
    let ids: Vec<u32> = r.roles.iter().map(|x| x.id).collect();
    assert_eq!(ids, [1, 9]);
    assert!(r.roles.iter().all(|x| x.explicit && x.variables.is_empty()));
    assert_eq!(r.relations, 0);
    assert_eq!(r.potion_expiry(), Some(99));
    assert_eq!(
        r.perks().iter().collect::<Vec<_>>(),
        [Perk::Remise, Perk::Feint]
    );
    assert_eq!(
        r.powers.unwrap().known,
        [Power {
            id: 5,
            flags: POWER_BLOB
        }]
    );
}

#[test]
fn legacy_first_roles_are_replaced() {
    let roles = |stored: &[u32]| {
        let mut body = front("vi01", 0, &[]);
        body.extend(middle(&[], &[]));
        body.extend((stored.len() as i32).to_le_bytes());
        for &id in stored {
            body.extend(u32s(&[id, 0]));
        }
        body.extend(u32s(&[0, 0, 0]));
        let r = CharacterRecord::parse(1, &frame(6, &body)).unwrap();
        r.roles.iter().map(|x| x.id).collect::<Vec<_>>()
    };
    // Disposition → Corpse and Activity Wait; XA_Proctor → XA Undead Bandit
    // and Activity Wait; Skeleton - Wait → Skeleton and Activity Wait.
    assert_eq!(roles(&[13, 1]), [33, 8]);
    assert_eq!(roles(&[27]), [33, 26]);
    assert_eq!(roles(&[22]), [33, 9]);
    assert_eq!(roles(&[9, 13]), [13, 9]);
    assert!(roles(&[]).is_empty());
}

/// Version 5 skips relations as 16-byte entries.
#[test]
fn legacy_version_5_relations() {
    let mut body = front("vi01", 0, &[]);
    body.extend(middle(&[], &[]));
    body.extend(u32s(&[0])); // no roles
    body.extend(u32s(&[1, 0, 0, 0, 0])); // one 16-byte relation
    body.extend(u32s(&[1, 0, 0, 2])); // a level table of two
    body.extend([0u8; 32]);
    body.extend(u32s(&[1, SAVED_HEAL_POOL, 0.5f32.to_bits()]));
    let r = CharacterRecord::parse(1, &frame(5, &body)).unwrap();
    assert_eq!((r.version, r.heal_pool()), (5, Some(0.5)));
}

/// Both saved entries present: each reads its own value (the original's
/// load reads the last entry's for both; that is the save system's call).
#[test]
fn saved_entries_read_their_own_values() {
    let mut body = front("vi01", 0, &[]);
    body.extend(middle(&[], &[]));
    body.extend(u32s(&[0, 0, 0, 0]));
    body.extend(u32s(&[
        2,
        SAVED_HEAL_POOL,
        0.2f32.to_bits(),
        SAVED_POTION,
        1800,
    ]));
    let r = CharacterRecord::parse(1, &frame(7, &body)).unwrap();
    assert_eq!((r.heal_pool(), r.potion_expiry()), (Some(0.2), Some(1800)));
}

#[test]
fn voice_numbers() {
    let index = |voice: &str| {
        CharacterRecord {
            voice: voice.into(),
            ..Default::default()
        }
        .voice_index()
    };
    assert_eq!(index("vi01"), 0);
    assert_eq!(index("vi03"), 2);
    assert_eq!(index("vm09"), 8);
    assert_eq!(index("vi00"), 0);
    assert_eq!(index("vm12"), 0);
    assert_eq!(index("vi0"), 0);
    // A stored byte 0xE9 reads as 'é': 233 − 49.
    assert_eq!(index("vi0\u{e9}"), 184);
}

#[test]
fn malformed_records_are_errors() {
    let mut good = front("vi01", 0, &[]);
    good.extend(middle(&[], &[]));
    good.extend(v7_rest(&[]));
    assert!(CharacterRecord::parse(1, &frame(7, &good)).is_ok());
    let parse = |b: &[u8]| CharacterRecord::parse(1, b);
    assert!(matches!(
        parse(&frame(8, &good)),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        parse(&u32s(&[0x00AB_AC07, 0])),
        Err(Error::BadMagic { .. })
    ));
    let mut cut = frame(7, &good);
    cut.truncate(cut.len() - 1);
    assert!(matches!(parse(&cut), Err(Error::Truncated { .. })));
    for n in 0..good.len() {
        assert!(parse(&frame(7, &good[..n])).is_err(), "{n} bytes");
    }
    // Three stray bytes where a sub-chunk would start.
    let mut stray = good.clone();
    stray.extend([1, 2, 3]);
    assert!(parse(&frame(7, &stray)).is_err());
    // A sub-chunk running past the record.
    let mut over = good.clone();
    over.extend(u32s(&[0x00CD_EC12, 9, 0, 0]));
    assert!(parse(&frame(7, &over)).is_err());
    // Seven techniques in a slot of six.
    let mut seven = front("vi01", 0, &[]);
    seven.extend(middle(&[(0, 35000, &[1, 2, 3, 4, 5, 6, 7])], &[]));
    seven.extend(v7_rest(&[]));
    assert!(matches!(parse(&frame(7, &seven)), Err(Error::Malformed(_))));
    // A negative worn count.
    let mut negative = front("vi01", 0, &[]);
    let at = negative.len() - 4;
    negative[at..].copy_from_slice(&(-1i32).to_le_bytes());
    negative.extend(middle(&[], &[]));
    negative.extend(v7_rest(&[]));
    assert!(matches!(
        parse(&frame(7, &negative)),
        Err(Error::Malformed(_))
    ));
    // A role list whose blob holds more than its entries.
    let mut roles = front("vi01", 0, &[]);
    roles.extend(middle(&[], &[]));
    roles.extend(u32s(&[0, 4, 0, 0, 0, 0]));
    assert!(matches!(parse(&frame(7, &roles)), Err(Error::Malformed(_))));
    // A power blob shorter than its own size word.
    let mut blob = good.clone();
    let mut powers = u32s(&[0; 11]);
    powers.extend(u32s(&[1, 1, POWER_BLOB, 2]));
    blob.extend(u32s(&[POWERS[0], powers.len() as u32]));
    blob.extend(powers);
    assert!(matches!(parse(&frame(7, &blob)), Err(Error::Malformed(_))));
}

#[test]
fn placements_and_their_database() {
    let mut body = front("vi01", 0x0000_0001, &[0x8000_0010, 0x8000_00CE]);
    body.extend(middle(&[], &[]));
    body.extend(v7_rest(&[(8, 1, 0, &[])]));
    let rec = frame(7, &body);
    let mut db = u32s(&[0xDBAC_CD00, 16, 2, 0, 0, rec.len() as u32]);
    db.extend(&rec);
    let mut place = u32s(&[0x8000_0002]);
    for v in [2500.0f32, 0.0, -1000.0, 1.5] {
        place.extend(v.to_le_bytes());
    }
    place.extend((DEAD).to_le_bytes());
    let mut b = 1i32.to_le_bytes().to_vec();
    b.extend(u32s(&[CHARACTER_DB, db.len() as u32]));
    b.extend(&db);
    b.extend(u32s(&[PLACEMENT, place.len() as u32]));
    b.extend(&place);
    let c = LevelCharacters::parse(&b).unwrap();
    let p = &c.placements[0];
    assert_eq!(p.position, [25.0, 0.0, -10.0]);
    assert!(p.is_dead());
    let r = c.record(p.id).unwrap();
    assert_eq!(r.name, "Zombie03");
    assert_eq!(r.worn, [0x8000_0010, 0x8000_00CE]);
    assert_eq!(r.roles[0].id, 8);
}

/// Every level's character section, with each database entry's parse.
fn level_records(
    resource: &exa_rpk::MappedArchive,
) -> Vec<(String, LevelCharacters, Vec<Result<CharacterRecord>>)> {
    let mut out = Vec::new();
    for e in resource.entries() {
        let Ok(chunks) = crate::chunk::rfc_chunks(resource.bytes(e)) else {
            continue;
        };
        for c in chunks.flatten().filter(|c| c.tag == CHARACTERS) {
            let entries = LevelCharacters::read_database(c.data).unwrap();
            let level = LevelCharacters::parse(c.data).unwrap();
            out.push((
                e.name.clone(),
                level,
                entries.into_iter().map(|(_, r)| r).collect(),
            ));
        }
    }
    out
}

/// The level character databases of the installed game (skipped without
/// it): every record parses to its end, every placed character's record is
/// version 7 and every living one stores full pools; every living human is
/// feral.
#[test]
fn shipped_level_records() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let levels = level_records(resource);
    assert_eq!(levels.len(), 13);
    let mut versions = [0; 8];
    let (mut empty, mut garbage) = (0, Vec::new());
    for (name, _, records) in &levels {
        for r in records {
            match r {
                Ok(r) => versions[usize::from(r.version)] += 1,
                Err(Error::Truncated { len: 0, .. }) => empty += 1,
                Err(e) => garbage.push(format!("{name}: {e}")),
            }
        }
    }
    // 478 records of version 7 (none of the 24 older ones is placed), 26
    // deleted entries, and two stubs that are not records (in exanima07 and
    // exanima07_sw).
    assert_eq!(versions, [0, 0, 0, 0, 0, 4, 20, 478]);
    assert_eq!(empty, 26);
    assert_eq!(garbage.len(), 2, "{garbage:?}");
    assert!(garbage.iter().all(|g| g.starts_with("exanima07")));
    let (mut placed, mut living, mut humans) = (0, 0, 0);
    let mut feral = std::collections::BTreeMap::<u32, usize>::new();
    for (_, level, records) in &levels {
        assert_eq!(level.records.len(), records.iter().flatten().count());
        for p in &level.placements {
            let r = level.record(p.id).expect("every placement has its record");
            assert_eq!(r.version, RECORD_VERSION);
            placed += 1;
            if p.is_dead() {
                continue;
            }
            living += 1;
            assert!(r.pools.is_full(), "{} {:?}", r.name, r.pools);
            if r.race == 0 {
                humans += 1;
                *feral.entry((r.feral * 100.0).round() as u32).or_default() += 1;
            } else {
                assert_eq!(r.feral, 0.0, "{}", r.name);
            }
        }
    }
    assert_eq!((placed, living, humans), (477, 451, 97));
    assert_eq!(
        feral.into_iter().collect::<Vec<_>>(),
        [(40, 1), (42, 1), (50, 27), (84, 21), (100, 47)]
    );
    // Perks across every parsed record; the four thaumaturges of exanima08
    // and one of exanima67 keep powers.
    let all: Vec<&CharacterRecord> = levels.iter().flat_map(|l| l.2.iter().flatten()).collect();
    let count = |perk| all.iter().filter(|r| r.perks().has(perk)).count();
    assert_eq!(
        Perk::ALL.map(count),
        [
            358, 303, 251, 273, 147, 103, 4, 2, 41, 33, 85, 3, 2, 1, 3, 1
        ]
    );
    // Dual Wield (Close Combat 8) is learnt, mostly as a fifth technique.
    let dual_wield = all.iter().filter(|r| r.skills[CLOSE_COMBAT].knows(8));
    assert_eq!(dual_wield.count(), 37);
    assert_eq!(all.iter().filter(|r| r.powers.is_some()).count(), 5);
    assert!(all.iter().all(|r| r.relations == 0 && r.saved.is_empty()));
}

/// The arena's actors in `actors.rpk` (skipped without the game): 21
/// records, 14 of version 6.
#[test]
fn shipped_actors() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let actors = exa_rpk::Archive::parse(resource.get("actors.rpk").unwrap()).unwrap();
    let records: Vec<(String, CharacterRecord)> = actors
        .entries()
        .iter()
        .map(|e| {
            let r = CharacterRecord::parse(0, actors.bytes(e)).unwrap();
            (e.name.clone(), r)
        })
        .collect();
    assert_eq!(records.len(), 21);
    assert_eq!(records.iter().filter(|(_, r)| r.version == 6).count(), 14);
    let actor = |name: &str| &records.iter().find(|(n, _)| n == name).unwrap().1;
    let perks = |name: &str| {
        actor(name)
            .perks()
            .iter()
            .map(Perk::name)
            .collect::<Vec<_>>()
    };
    let o16 = actor("opponent16");
    assert_eq!(
        (o16.version, o16.combat_skill, o16.experience),
        (7, 100.0, 45000)
    );
    assert_eq!(
        perks("opponent16"),
        ["Remise", "Feint", "Fend", "Riposte", "Security"]
    );
    assert_eq!((o16.manoeuvring_tier(), o16.deflection()), (3, true));
    assert_eq!(
        perks("opponent13"),
        [
            "Remise",
            "Feint",
            "Fend",
            "Riposte",
            "Counter",
            "Synchrony",
            "Ward",
            "Persist",
            "Endure"
        ]
    );
    let o1 = actor("opponent01");
    assert_eq!((o1.version, o1.pools.yellow), (6, 0));
    assert!(o1.perks().is_empty());
    // The opponents (of both versions) and the proctor store empty pools;
    // the other actors full yellow and red.
    for (name, r) in &records {
        let stored = (r.pools.yellow, r.pools.red);
        if name.starts_with("opponent") || name == "proctor" {
            assert_eq!(stored, (0, 0), "{name}");
        } else {
            assert_eq!(stored, (FULL_POOL, FULL_POOL), "{name}");
        }
    }
    let demon = actor("demon a01");
    assert_eq!((demon.race, demon.combat_skill), (11, 100.0));
    assert_eq!(perks("demon a01"), ["Remise", "Feint"]);
    let ids = |name: &str| actor(name).roles.iter().map(|r| r.id).collect::<Vec<_>>();
    assert_eq!(ids("xac_derrin"), [1, 33, 20]);
}
