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
    assert_eq!((r.voice.as_str(), r.voice_pitch), ("vi03", 1.05));
    assert_eq!(r.hands.map(|h| h.id), [0x17, 0, 0x22, 0]);
    assert_eq!(r.worn_ids().collect::<Vec<_>>(), [0x8000_0010, 0x8000_00CE]);
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
                weight: 0.0,
                explicit: true,
                inherited: 0,
                variables: vec![]
            },
            Role {
                id: 2,
                weight: 0.5,
                explicit: false,
                inherited: 1,
                variables: vec![7, 9]
            }
        ]
    );
    assert_eq!(r.relation_count(), 1);
    assert_eq!((r.heal_pool(), r.potion_expiry()), (Some(0.25), Some(1800)));
    let powers = r.powers.unwrap();
    assert_eq!((powers.word, powers.slots[0]), (7, 0x0002_020B));
    assert_eq!(
        powers.known,
        [
            Power {
                id: 3,
                flags: 0x0001_5FFF,
                blob: None
            },
            Power {
                id: 4,
                flags: POWER_BLOB,
                blob: Some(vec![0xAA, 0, 0, 0, 0xBB, 0, 0, 0])
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
    // The body's technique word: every bit but Fend's 0x4.
    assert_eq!(r.perks().body_word(), 0x3ff1_100b);
    assert_eq!(
        with_skills(&[(5, 25000, &[1, 5, 6, 8, 2])])
            .perks()
            .body_word(),
        0x0071_0000
    );
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
    assert_eq!(r.worn_ids().collect::<Vec<_>>(), [0x8000_0001]);
    assert_eq!(r.feral, 0.84);
    assert_eq!(r.inventory.len(), 1);
    let ids: Vec<u32> = r.roles.iter().map(|x| x.id).collect();
    assert_eq!(ids, [1, 9]);
    assert!(r.roles.iter().all(|x| x.explicit && x.variables.is_empty()));
    assert_eq!(r.relation_count(), 0);
    assert_eq!(r.potion_expiry(), Some(99));
    assert_eq!(
        r.perks().iter().collect::<Vec<_>>(),
        [Perk::Remise, Perk::Feint]
    );
    assert_eq!(
        r.powers.unwrap().known,
        [Power {
            id: 5,
            flags: POWER_BLOB,
            blob: None
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
    let index = voice_number_of;
    assert_eq!(index(b"vi01"), 0);
    assert_eq!(index(b"vi03"), 2);
    assert_eq!(index(b"vm09"), 8);
    assert_eq!(index(b"vi00"), 0);
    assert_eq!(index(b"vm12"), 0);
    assert_eq!(index(b"vi0"), 0);
    // A stored byte 0xE9 is one character: 233 − 49.
    assert_eq!(index(b"vi0\xe9"), 184);
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
    assert_eq!(r.worn_ids().collect::<Vec<_>>(), [0x8000_0010, 0x8000_00CE]);
    assert_eq!(r.roles[0].id, 8);
}

/// A placement section holding one placement whose sub-chunks are `subs`.
fn placement_section(subs: &[u8]) -> Vec<u8> {
    let mut place = u32s(&[0x8000_0002]);
    for v in [2500.0f32, 0.0, -1000.0, 1.5] {
        place.extend(v.to_le_bytes());
    }
    place.extend(DEAD.to_le_bytes());
    place.extend(subs);
    let mut b = 1i32.to_le_bytes().to_vec();
    b.extend(u32s(&[PLACEMENT, place.len() as u32]));
    b.extend(&place);
    b
}

/// A 3×4 row frame translated by `t` (cm).
fn frame_at(t: [f32; 3]) -> Vec<u8> {
    f32s(&[
        1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, t[0], t[1], t[2],
    ])
}

/// A saved bone-tree record (56 bytes) and its children.
fn pose_node(t: [f32; 3], word: u32, children: i32, children_bytes: &[u8]) -> Vec<u8> {
    let mut b = frame_at(t);
    b.extend(u32s(&[word, children as u32]));
    b.extend(children_bytes);
    b
}

/// The bone tree keeps its records, words and child counts, and wins over
/// the legacy flat chunk whatever order the two are stored in; the flat
/// chunk alone is read as world frames.
#[test]
fn placements_read_their_saved_poses() {
    let child = pose_node([4.0, 5.0, 6.0], 8, 0, &[]);
    let tree = pose_node([1.0, 2.0, 3.0], 7, 1, &child);
    let mut flat = frame_at([9.0, 8.0, 7.0]);
    flat.extend(frame_at([10.0, 11.0, 12.0]));
    let mut subs = Vec::new();
    subs.extend(u32s(&[LEGACY_POSE, flat.len() as u32]));
    subs.extend(&flat);
    subs.extend(u32s(&[POSE, tree.len() as u32]));
    subs.extend(&tree);
    let c = LevelCharacters::parse(&placement_section(&subs)).unwrap();
    let SavedPose::Tree(records) = &c.placements[0].pose else {
        panic!("a bone tree: {:?}", c.placements[0].pose);
    };
    assert_eq!(records.len(), 2);
    assert_eq!((records[0].word, records[0].children), (7, 1));
    assert_eq!((records[1].word, records[1].children), (8, 0));
    assert_eq!(records[0].frame[3], [0.01, 0.02, 0.03]);
    assert_eq!(records[1].frame[3], [0.04, 0.05, 0.06]);

    let mut subs = Vec::new();
    subs.extend(u32s(&[LEGACY_POSE, flat.len() as u32]));
    subs.extend(&flat);
    let c = LevelCharacters::parse(&placement_section(&subs)).unwrap();
    let SavedPose::Flat(frames) = &c.placements[0].pose else {
        panic!("flat frames: {:?}", c.placements[0].pose);
    };
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0][3], [0.09, 0.08, 0.07]);
    assert_eq!(frames[1][3], [0.1, 0.11, 0.12]);
}

/// Malformed pose nodes are errors, never panics.
#[test]
fn malformed_saved_poses_are_errors() {
    for children in [-1, 65] {
        let tree = pose_node([0.0; 3], 1, children, &[]);
        let mut subs = Vec::new();
        subs.extend(u32s(&[POSE, tree.len() as u32]));
        subs.extend(&tree);
        assert!(matches!(
            LevelCharacters::parse(&placement_section(&subs)),
            Err(Error::Malformed(_))
        ));
    }
    // A tree that ends before a child.
    let tree = pose_node([0.0; 3], 1, 1, &[]);
    let mut subs = Vec::new();
    subs.extend(u32s(&[POSE, tree.len() as u32]));
    subs.extend(&tree);
    assert!(LevelCharacters::parse(&placement_section(&subs)).is_err());
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
    assert!(
        all.iter()
            .all(|r| r.relation_count() == 0 && r.saved.is_empty())
    );
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
    // The same sub-chunks are absent from the actors.
    assert!(
        records
            .iter()
            .all(|(_, r)| r.relation_count() == 0 && r.saved.is_empty())
    );
    assert!(
        records
            .iter()
            .all(|(_, r)| r.loadouts.is_empty() && r.reserved.is_none())
    );
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

/// A fresh record takes the default constructor's pools and pitch; the
/// creator's reset adds its identity defaults and the underwear.
#[test]
fn a_new_record_starts_blank_and_a_reset_one_takes_the_creators_defaults() {
    let new = CharacterRecord::new();
    // The constructor fills yellow and red only: focus stays 0.
    assert_eq!(new.pools, Pools::NEW);
    assert_eq!(
        (new.pools.yellow, new.pools.red, new.pools.focus),
        (FULL_POOL, FULL_POOL, 0)
    );
    assert_eq!(new.voice_pitch, 1.0);
    assert_eq!(new.physique, [0.0, 0.0]);
    assert!(new.worn.is_empty());
    assert!(new.name.is_empty() && new.hands.iter().all(|h| !h.is_some()));
    assert_eq!(&new.to_bytes().unwrap()[..4], &RECORD.to_le_bytes());

    let fresh = CharacterRecord::reset();
    assert_eq!(fresh.physique, [DEFAULT_PHYSIQUE; 2]);
    assert_eq!(fresh.skin, DEFAULT_SKIN);
    assert_eq!(fresh.hair_colour, DEFAULT_HAIR_COLOUR);
    assert_eq!((fresh.race, fresh.female), (0, false));
    assert_eq!(fresh.voice_number, 0);
    assert_eq!(fresh.voice, "vi01");
    assert_eq!(fresh.pools, Pools::NEW);
    assert_eq!(fresh.worn, [ItemRef::new(UNDERWEAR_ITEM, 0)]);
    // The reset keeps the name: the creator sets it, not the defaults.
    assert!(fresh.name.is_empty());
}

/// A record with every part, written and read back.
#[test]
fn written_records_read_back_the_same() {
    let mut r = CharacterRecord::reset();
    r.id = 9;
    r.name = "Zombie03".into();
    r.surname = "Smith".into();
    r.physique = [0.25, 0.75];
    r.height = -0.5;
    r.age = 0.3;
    r.skin = [0x1234, 0x5678];
    r.hair_style = 3;
    r.hair_colour = [1, 2, 3, 4];
    r.face = [5, 6];
    r.voice_number = 4;
    r.voice = voice_name(4);
    r.voice_pitch = 1.05;
    r.female = true;
    r.race = 11;
    r.locale = 2;
    r.hands = [
        ItemRef::new(0x17, 0x1),
        ItemRef::new(0x8000_0002, 0),
        ItemRef::NONE,
        ItemRef::new(0x22, 0x100),
    ];
    r.worn = vec![ItemRef::new(8, 0), ItemRef::new(0x8000_0003, 0x100)];
    r.feral = 0.84;
    r.pools = Pools {
        yellow: 0x8000,
        red: 0x4000,
        focus: 0x1000,
    };
    r.seed = 0x1234;
    r.combat_skill = 25.0;
    r.undead_voice = 3;
    r.traits = [0.0, -50.0, 100.0, -100.0];
    r.search_seed = 0xDEAD;
    r.experience = 25000;
    r.extra_word = 0;
    r.skills[CLOSE_COMBAT] = Skill {
        points: 15000,
        techniques: [1, 4, 2, 0, 0, 0],
    };
    r.inventory = vec![
        Carried {
            item: 0x8000_0010,
            flags: 0x100,
            extra: NEW_ENTRY_WORD,
        },
        Carried::default(),
    ];
    r.roles = vec![
        Role {
            id: 8,
            weight: 0.0,
            explicit: true,
            inherited: 0,
            variables: Vec::new(),
        },
        Role {
            id: 2,
            weight: 0.5,
            explicit: false,
            inherited: 1,
            variables: vec![7, 9],
        },
    ];
    r.relations = Relations {
        entries: vec![(5, 0), (6, 0x14)],
        blob: vec![1, 2, 3],
    };
    r.level_relations = vec![RelationTable {
        words: [0x8000_0001, 0],
        relations: Relations::default(),
    }];
    r.saved = vec![
        SavedState {
            tag: SAVED_HEAL_POOL,
            value: 0.25f32.to_bits(),
        },
        SavedState {
            tag: SAVED_POTION,
            value: 1800,
        },
    ];
    r.loadouts = vec![Loadout {
        word: 3,
        hands: [
            ItemRef::new(0x17, 0),
            ItemRef::new(0x18, 0),
            ItemRef::NONE,
            ItemRef::NONE,
        ],
        worn: vec![ItemRef::new(0x8000_0004, 0)],
    }];
    r.powers = Some(Powers {
        word: 7,
        slots: [0x0B; POWER_SLOTS],
        known: vec![
            Power {
                id: 3,
                flags: 0x15FFF,
                blob: None,
            },
            Power {
                id: 4,
                flags: POWER_BLOB,
                blob: Some(vec![0xAA, 0xBB]),
            },
        ],
    });
    r.reserved = Some([1, 2]);
    let bytes = r.to_bytes().unwrap();
    let read = CharacterRecord::parse(9, &bytes).unwrap();
    assert_eq!(read, r);
    // Roles keep their stored weight; traits round.
    assert_eq!(read.roles[1].weight, 0.5);
    assert_eq!(read.traits, [0.0, -50.0, 100.0, -100.0]);
    // Sub-chunks are only written when present: a reset record is the
    // fixed part and no sub-chunk at all.
    let plain = CharacterRecord::reset().to_bytes().unwrap();
    assert_eq!(plain.len(), 8 + 200);
    let size = u32::from_le_bytes(plain[4..8].try_into().unwrap());
    assert_eq!(size as usize + 8, plain.len());
}

/// Every version 7 record of the installed game writes back byte for byte
/// (skipped without the game): levels and the arena's actors.
#[test]
fn shipped_records_write_back_byte_for_byte() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let mut checked = 0;
    let mut reworded = 0;
    let mut bad = Vec::new();
    let mut check = |name: &str, payload: &[u8]| {
        let Ok(r) = CharacterRecord::parse(0, payload) else {
            return;
        };
        if r.version != RECORD_VERSION {
            return;
        }
        let size = u32::from_le_bytes(payload[4..8].try_into().unwrap()) as usize;
        let written = r.to_bytes().unwrap();
        let stored = &payload[..8 + size];
        checked += 1;
        // The game writes the voice field as `vi` and the record's voice
        // number, so the two records whose stored name is not that form
        // (`HM04`) come back with the canonical name.
        let voice = 8 + 64..8 + 80;
        if written[voice.clone()] != stored[voice.clone()] {
            reworded += 1;
            assert_eq!(&stored[voice.clone()], b"HM04\0\0\0\0\0\0\0\0\0\0\0\0");
        }
        let mut comparable = written.clone();
        comparable[voice.clone()].copy_from_slice(&stored[voice.clone()]);
        if comparable != stored {
            let at = comparable.iter().zip(stored).position(|(a, b)| a != b);
            bad.push(format!(
                "{name} {}: {} bytes written, {} stored, first difference at {at:?}",
                r.name,
                written.len(),
                8 + size
            ));
        }
    };
    for e in resource.entries() {
        let Ok(chunks) = crate::chunk::rfc_chunks(resource.bytes(e)) else {
            continue;
        };
        for c in chunks.flatten().filter(|c| c.tag == CHARACTERS) {
            let Ok((_, sections)) = sections(c.data) else {
                continue;
            };
            for (tag, body) in sections {
                if tag != CHARACTER_DB {
                    continue;
                }
                let Ok(db) = Database::parse(body) else {
                    continue;
                };
                for rec in &db.records {
                    check(&e.name, db.payload(rec));
                }
            }
        }
    }
    let actors = exa_rpk::Archive::parse(resource.get("actors.rpk").unwrap()).unwrap();
    for e in actors.entries() {
        check(&e.name, actors.bytes(e));
    }
    assert_eq!(checked, 485, "{checked} records");
    assert_eq!(reworded, 2, "records with a non-canonical voice name");
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The legacy reader compares the first role id before it skips the list,
/// so with no roles it reads the first relation word: a relation table of
/// 13 entries turns into the replacement roles of 13 (Disposition). A role
/// stored twice is added once.
#[test]
fn legacy_roles_peek_past_an_empty_list_and_add_each_role_once() {
    let mut body = front("vi01", 0, &[]);
    body.extend(middle(&[], &[]));
    body.extend(u32s(&[0])); // no roles
    body.extend(u32s(&[13])); // a relation table of 13 entries
    body.extend([0u8; 13 * 8]);
    body.extend(u32s(&[0, 0, 0])); // its empty blob, no level tables, nothing saved
    let r = CharacterRecord::parse(1, &frame(6, &body)).unwrap();
    assert_eq!(r.roles.iter().map(|x| x.id).collect::<Vec<_>>(), [33, 8]);

    let mut twice = front("vi01", 0, &[]);
    twice.extend(middle(&[], &[]));
    twice.extend(u32s(&[3, 9, 0, 2, 0, 9, 0]));
    twice.extend(u32s(&[0, 0, 0]));
    let r = CharacterRecord::parse(1, &frame(6, &twice)).unwrap();
    assert_eq!(r.roles.iter().map(|x| x.id).collect::<Vec<_>>(), [2, 9]);
}

/// The trait bytes as the game's conversion makes them: nearest integer,
/// ties to even, then + 100 in a byte; NaN, infinities and values past
/// the 64-bit range convert to the indefinite integer, whose low byte is
/// 0, so they store 100.
#[test]
fn traits_convert_as_the_game_does() {
    assert_eq!(round_even_i64(2.5), 2);
    assert_eq!(round_even_i64(3.5), 4);
    assert_eq!(round_even_i64(-2.5), -2);
    assert_eq!(round_even_i64(-0.4), 0);
    assert_eq!(round_even_i64(f32::NAN), i64::MIN);
    assert_eq!(round_even_i64(f32::INFINITY), i64::MIN);
    assert_eq!(round_even_i64(1e20), i64::MIN);
    assert_eq!(round_even_i64(-9.223_372e18), i64::MIN);
    let mut r = CharacterRecord::new();
    let at = |r: &CharacterRecord| {
        let bytes = r.to_bytes().unwrap();
        // tag, size, the fixed fields up to the traits (no worn items).
        let start = 8 + 16 + 16 + 8 + 4 + 4 + 4 + 4 + 4 + 4 + 16 + 4 + 4 + 4 + 32 + 4;
        let traits = start + 4 + 8 + 4 + 4;
        bytes[traits..traits + 4].to_vec()
    };
    r.traits = [2.5, -100.5, 300.0, f32::NAN];
    // 102, 0 (−100 rounds to even), 400 & 0xFF, and 100.
    assert_eq!(at(&r), [102, 0, 144, 100]);
    r.traits = [f32::INFINITY, 1e20, -3.5, 0.5];
    assert_eq!(at(&r), [100, 100, 96, 100]);
}

/// The voice name the writer builds from the number: the game's formatter
/// pads to two digits, writes 0 as `00` and a negative sum as `0`.
#[test]
fn voice_names_from_their_number() {
    assert_eq!(voice_name(0), "vi01");
    assert_eq!(voice_name(8), "vi09");
    assert_eq!(voice_name(98), "vi99");
    assert_eq!(voice_name(99), "vi100");
    // −1 + 1 = 0, and i32::MAX + 1 wraps negative.
    assert_eq!(voice_name(u32::MAX), "vi00");
    assert_eq!(voice_name(0x7FFF_FFFF), "vi0");
    // The reader only takes a name whose third character is `0`: `vi10`
    // reads back as voice 0.
    assert_eq!(voice_number_of(voice_name(9).as_bytes()), 0);
}

/// Names hold one character per byte: a Latin-1 character writes back as
/// its byte, not as UTF-8.
#[test]
fn names_write_one_byte_per_character() {
    let mut r = CharacterRecord::new();
    r.name = "\u{d6}laf".into();
    r.surname = "abcdefghijklmnopqrs".into();
    let bytes = r.to_bytes().unwrap();
    assert_eq!(&bytes[8..13], &[0xD6, b'l', b'a', b'f', 0]);
    let read = CharacterRecord::parse(0, &bytes).unwrap();
    assert_eq!(read.name, "\u{d6}laf");
    assert_eq!(read.surname, "abcdefghijklmnop");
}

/// The writer's conditions for the sub-chunks: powers only with an entry,
/// the reserved pair only when its first word is set, a power's blob
/// flag exactly when it has a blob (one of no bytes keeps it).
#[test]
fn sub_chunks_are_written_as_the_game_decides() {
    let fixed = CharacterRecord::new().to_bytes().unwrap().len();
    let mut r = CharacterRecord::new();
    r.powers = Some(Powers {
        word: 1,
        slots: [0; POWER_SLOTS],
        known: Vec::new(),
    });
    r.reserved = Some([0, 5]);
    assert_eq!(r.to_bytes().unwrap().len(), fixed);
    r.reserved = Some([5, 0]);
    r.powers.as_mut().unwrap().known = vec![
        Power {
            id: 2,
            flags: POWER_BLOB,
            blob: None,
        },
        Power {
            id: 3,
            flags: 0,
            blob: Some(Vec::new()),
        },
    ];
    let read = CharacterRecord::parse(0, &r.to_bytes().unwrap()).unwrap();
    assert_eq!(read.reserved, Some([5, 0]));
    let known = read.powers.unwrap().known;
    assert_eq!((known[0].flags, known[0].blob.as_ref()), (0, None));
    assert_eq!(
        (known[1].flags, known[1].blob.as_deref()),
        (POWER_BLOB, Some(&[][..]))
    );
}

/// The load and the spawn compute the pools in double precision:
/// 0.25 × 257/65535 is 0.000980392156…, nearest f32 0.00098039221; the
/// single-precision product (0.25 × 257 × 1.5259022e-5) gives the f32 below
/// it. The unit is the double nearest 1/65535 (`0x3EF0001000100010`).
#[test]
fn pools_load_in_double_precision() {
    assert_eq!(POOL_UNIT.to_bits(), 0x3EF0_0010_0010_0010);
    let loaded = Pools::restore(257, 0.25);
    assert_eq!(loaded, 0.000_980_392_2);
    assert_ne!(loaded, 0.25 * 257.0 * 1.525_902_2e-5_f32);
    assert_eq!(Pools::restore(FULL_POOL, 0.375), 0.375);
    assert_eq!(Pools::restore_focus(0x8000), 0.500_007_6);
    // The save: nearest even, cut to 16 bits; NaN stores 0.
    assert_eq!(Pools::save(32767.5), 32768);
    assert_eq!(Pools::save(65535.0), FULL_POOL);
    assert_eq!(Pools::save(f32::NAN), 0);
    assert_eq!(Pools::save(65536.0), 0);
}

/// The arena's technique check looks at the five written slots only and
/// needs the points of the slot's level.
#[test]
fn the_records_technique_check() {
    let skill = Skill {
        points: 10000,
        techniques: [4, 1, 2, 0, 0, 3],
    };
    assert!(skill.has_technique(4) && skill.has_technique(1));
    assert!(!skill.has_technique(2)); // the third level needs 15000
    assert!(!skill.has_technique(3)); // the sixth slot is not read
    let rich = Skill {
        points: 30000,
        ..skill
    };
    assert!(rich.has_technique(2) && !rich.has_technique(3));
    // 0 matches the first empty slot, the fourth level: 20000 points.
    assert!(rich.has_technique(0) && !skill.has_technique(0));
}

/// A copy goes through the writer: the voice name is rebuilt and the id
/// cleared.
#[test]
fn a_duplicate_is_written_and_read_back() {
    let mut r = CharacterRecord::reset();
    r.id = 0x8000_0007;
    r.voice = "HM04".into();
    r.voice_number = 3;
    r.skills[ARMOUR].points = 5000; // no technique: not written
    let copy = r.duplicate().unwrap();
    assert_eq!((copy.id, copy.voice.as_str()), (0, "vi04"));
    assert_eq!(copy.skills[ARMOUR], Skill::default());
    assert_eq!(copy.worn, r.worn);
}

/// The creator's reset of a record in use keeps what it does not set and
/// appends another underwear.
#[test]
fn the_creators_reset_keeps_the_other_fields() {
    let mut r = CharacterRecord::new();
    r.name = "Kept".into();
    r.pools.focus = 0x1234;
    r.set_profession(0x0001_0301);
    r.worn.push(ItemRef::new(9, 0));
    r.apply_creator_defaults();
    assert_eq!((r.name.as_str(), r.pools.focus), ("Kept", 0x1234));
    assert_eq!((r.profession(), r.female, r.race), (0, false, 0));
    assert_eq!(
        r.worn,
        [ItemRef::new(9, 0), ItemRef::new(UNDERWEAR_ITEM, 0)]
    );
}

/// The save load clears the race byte from the profession word, keeping sex
/// and the kept bits.
#[test]
fn the_load_reset_clears_only_the_race_byte() {
    let mut r = CharacterRecord::new();
    r.set_profession(0x8001_0301);
    assert_eq!(
        (r.race, r.female, r.profession_rest),
        (3, true, 0x8001_0000)
    );
    r.reset_loaded_identity();
    assert_eq!(r.race, 0);
    assert!(r.female);
    assert_eq!(r.profession(), 0x8001_0001);
}

/// The tree's technique check scans the funded slots and lets 0 match an
/// empty one; the perks' check refuses 0.
#[test]
fn the_trees_technique_check_scans_funded_slots() {
    let skill = Skill {
        points: 10_000,
        techniques: [0, 2, 7, 0, 0, 0],
    };
    assert!(skill.holds_funded(0));
    assert!(skill.holds_funded(2));
    assert!(!skill.holds_funded(7));
    assert!(!skill.knows(0));
    assert!(skill.knows(2));
}

/// A save's placement carries the motion state and the AI home beside the
/// pose; a level placement carries neither.
#[test]
fn a_placement_reads_its_motion_state_and_ai_home() {
    let mut b = u32s(&[7]);
    b.extend(f32s(&[100.0, 200.0, 300.0, 0.5]));
    b.extend(u32s(&[0]));
    b.extend(u32s(&[MOTION, 12]));
    b.extend(u32s(&[3]));
    b.extend(f32s(&[1.5]));
    b.extend(u32s(&[0]));
    b.extend(u32s(&[AI_HOME, 20]));
    b.extend(f32s(&[100.0, 200.0, 300.0, 0.25]));
    b.extend(u32s(&[0]));
    let p = read_placement(&b).unwrap();
    assert_eq!(
        p.motion,
        Some(MotionState {
            state: 3,
            value: 1.5
        })
    );
    assert_eq!(
        p.ai_home,
        Some(AiHome {
            position: [1.0, 2.0, 3.0],
            heading: 0.25
        })
    );
    // The same placement without the two chunks has neither.
    let mut plain = u32s(&[7]);
    plain.extend(f32s(&[100.0, 200.0, 300.0, 0.5]));
    plain.extend(u32s(&[0]));
    let p = read_placement(&plain).unwrap();
    assert_eq!((p.motion, p.ai_home), (None, None));
}

/// A section entry read whole: the record and the object database and
/// string library the sub-chunks after it carry.
#[test]
fn an_entry_reads_its_record_and_its_own_database() {
    let record = CharacterRecord::new();
    let mut bytes = record.to_bytes().unwrap();
    let mut db = crate::objdb::Store::from_records([(1, 0, &[0u8; 4][..])]);
    db.set_flags(0, crate::objdb::MAGIC);
    let mut library = crate::objstrings::Library::default();
    library.add(b"Blade");
    for (tag, body) in [
        (RECORD_STRINGS, library.write()),
        (0x1234, vec![9, 9, 9]),
        (ACTOR_ITEMS, db.write()),
    ] {
        bytes.extend(tag.to_le_bytes());
        bytes.extend((body.len() as u32).to_le_bytes());
        bytes.extend(body);
    }
    let (read, own) = read_entry(0, &bytes).unwrap();
    assert_eq!(read, record.duplicate().unwrap());
    assert_eq!(own.strings.get(1).as_deref(), Some("Blade"));
    assert_eq!(own.base.len(), 1);
    assert!(own.base.find(1).is_some());
    // An entry that ends with the record has an empty database.
    let (read, own) = read_entry(0, &record.to_bytes().unwrap()).unwrap();
    assert_eq!(read, record.duplicate().unwrap());
    assert!(own.base.is_empty() && own.strings.is_empty());
    // A sub-chunk that runs past the entry is an error, not a panic.
    let mut short = record.to_bytes().unwrap();
    short.extend(ACTOR_ITEMS.to_le_bytes());
    short.extend(100u32.to_le_bytes());
    assert!(read_entry(0, &short).is_err());
    assert!(read_entry(0, &[0; 3]).is_err());
}

/// The shipped entries read whole: every actor's record and its own item
/// database (skipped without the game).
#[test]
fn shipped_entries_read_their_own_databases() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let actors = exa_rpk::Archive::parse(resource.get("actors.rpk").unwrap()).unwrap();
    let mut with_items = 0;
    let mut items = 0;
    for e in actors.entries() {
        let b = actors.bytes(e);
        let (_, own) = read_entry(0, b).unwrap_or_else(|err| panic!("{}: {err}", e.name));
        assert!(
            own.strings.is_empty(),
            "{}: a shipped entry has no library",
            e.name
        );
        if !own.base.is_empty() {
            with_items += 1;
            items += own.base.len();
        }
    }
    assert_eq!(actors.entries().len(), 21);
    assert!(with_items >= 19, "{with_items}");
    // The databases hold the items actor_items reads.
    let listed: usize = actors
        .entries()
        .iter()
        .map(|e| actor_items(actors.bytes(e)).map_or(0, |i| i.len()))
        .sum();
    assert_eq!(items, listed);
}

/// A section's database read entry by entry: each record with the object
/// database its entry carries, and the errors of the entries that are not
/// records.
#[test]
fn a_sections_entries_carry_their_own_databases() {
    let mut body = front("vi01", 0, &[0x8000_0010]);
    body.extend(middle(&[], &[]));
    body.extend(v7_rest(&[]));
    let mut rec = frame(7, &body);
    let mut db = crate::objdb::Store::from_records([(1, 0, &[0u8; 4][..])]);
    db.set_flags(0, crate::objdb::MAGIC);
    let items = db.write();
    rec.extend(ACTOR_ITEMS.to_le_bytes());
    rec.extend((items.len() as u32).to_le_bytes());
    rec.extend(&items);
    // Two table entries: the record at the table's end, and an empty slot
    // whose entry is not a record.
    let mut rdb = u32s(&[0xDBAC_CD00, 32, 3, 0, 0, rec.len() as u32, 0, 0, 0, 0]);
    rdb.extend(&rec);
    let mut b = 0i32.to_le_bytes().to_vec();
    b.extend(u32s(&[CHARACTER_DB, rdb.len() as u32]));
    b.extend(&rdb);
    let entries = LevelCharacters::read_entries(&b).unwrap();
    assert_eq!(entries.len(), 2);
    let (id, entry) = &entries[0];
    assert_eq!(*id, 3);
    let (record, own) = entry.as_ref().unwrap();
    assert_eq!(record.worn_ids().collect::<Vec<_>>(), [0x8000_0010]);
    assert!(own.base.find(1).is_some());
    assert!(entries[1].1.is_err());
    // The free function reads the database alone, without a section.
    let entries = read_character_entries(&rdb).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].1.as_ref().unwrap().1.base.len(), 1);
}

/// Every shipped level section reads entry by entry, and the entry-owned
/// databases hold items for the records that name local ones (skipped
/// without the game).
#[test]
fn shipped_sections_read_their_entry_databases() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let mut sections = 0;
    let mut entries = 0;
    for e in resource.entries() {
        if !(e.name.starts_with("exanima") && e.name.ends_with(".rfc")) {
            continue;
        }
        let Some(bytes) = resource.get(&e.name) else {
            continue;
        };
        for c in crate::chunk::rfc_chunks(bytes).unwrap().flatten() {
            if c.tag != CHARACTERS {
                continue;
            }
            sections += 1;
            for (id, entry) in LevelCharacters::read_entries(c.data).unwrap() {
                match entry {
                    Ok((record, own)) => {
                        entries += 1;
                        assert_eq!(record.id, id);
                        // No shipped level entry carries its own item
                        // database: the items live in the level's own.
                        assert!(own.base.is_empty(), "{}", record.name);
                        assert!(own.strings.is_empty(), "{}", record.name);
                    }
                    Err(_) => {}
                }
            }
        }
    }
    assert_eq!(sections, 13);
    assert!(entries > 480, "{entries}");
}
