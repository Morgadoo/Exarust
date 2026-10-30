use super::*;
use crate::characters::{
    self, ARMOUR, CHARACTERS, ID_MASK, ItemRef, LOCAL_ITEM, LevelCharacters, SHIELDS, Skill,
};
use crate::objdb;
use crate::rdb::Database;

/// A shield of bulk `bulk`.
fn shield(bulk: f32) -> ShieldRecord {
    ShieldRecord {
        weight: 0.0,
        block: 0.0,
        bulk,
    }
}

/// A worn item of `byte` encumbrance covering `bones` (1..18), of the
/// articulated set `set` (0: none).
fn piece(byte: u8, bones: &[usize], set: u32) -> Burden {
    Burden {
        encumbrance: byte,
        bones: bones.iter().fold(0, |m, b| m | 1 << (b - 1)),
        set,
        ..Burden::default()
    }
}

fn plain(byte: u8) -> Burden {
    piece(byte, &[9], 0)
}

const NONE: Skills = Skills {
    manoeuvring: 0,
    mobility: false,
};

/// A layer covering `bones` (1..18) with the display coverage `display`,
/// carrying the head mark when `mark` is set.
fn helmet(display: u8, bones: &[usize], mark: bool) -> objdb::Armour {
    objdb::Armour {
        coverage: std::array::from_fn(|i| if bones.contains(&(i + 1)) { 50 } else { 0 }),
        display,
        head_height_mark: mark,
        ..objdb::Armour::default()
    }
}

/// The extra body height the armour pass takes from a layer over the
/// eleventh bone: the thickest display coverage, plus (physique − ½) × 8.
#[test]
fn the_head_extra_is_the_thickest_layer_and_the_physique() {
    // 60 % over the eleventh bone, 40 % over a leg: 60.
    let worn = [helmet(60, &[11], true), helmet(40, &[9], true)];
    assert_eq!(head_extra(&worn, 0.5), 60.0);
    // A physique of 1 (the default average is ½) adds 8 × ½.
    assert_eq!(head_extra(&worn, 1.0), 64.0);
    // A thinner layer over the same bone never replaces the thickest.
    assert_eq!(
        head_extra(&[helmet(30, &[11], true), helmet(60, &[11], true)], 0.5),
        60.0
    );
    // Without the body's head mark the layer counts not at all.
    assert_eq!(head_extra(&[helmet(60, &[11], false)], 0.5), 0.0);
    // A layer over another bone alone counts not at all.
    assert_eq!(head_extra(&[helmet(60, &[3, 10], true)], 0.5), 0.0);
    // Nothing worn: the physique offset alone, negative below ½.
    assert_eq!(head_extra(&[], 0.25), -2.0);
}

/// The duellist's outfit: a leather jacket (10), trousers and boots (0)
/// and a full helm (60) weigh 70 × 0.0025.
#[test]
fn bytes_add_up() {
    let outfit = [plain(10), plain(0), plain(0), plain(60)];
    assert_eq!(encumbrance(&outfit, None, NONE), 0.175);
    assert_eq!(encumbrance(&[], None, NONE), 0.0);
}

/// A cuirass (thorax and lumbar) and a helm of one set touch: each takes
/// 0.025 off; boots of the set touch neither; a piece of another set
/// touches as well, and pieces of no set never do.
#[test]
fn articulated_pieces_touching() {
    let cuirass = piece(80, &[8, 9], 445);
    let helm = piece(40, &[10], 445);
    let boots = piece(20, &[4, 7], 445);
    assert!((encumbrance(&[cuirass, helm, boots], None, NONE) - 0.3).abs() < 1e-6);
    let other_set = piece(40, &[10], 798);
    assert!((encumbrance(&[cuirass, other_set], None, NONE) - 0.25).abs() < 1e-6);
    let loose = piece(40, &[10], 0);
    assert!((encumbrance(&[cuirass, loose], None, NONE) - 0.3).abs() < 1e-6);
}

/// The original's neighbour table: the left upper arm (bone 15) reaches the
/// right shoulder (11), so a left vambrace and a right pauldron touch,
/// while the left pauldron (14) and the right upper arm (12) do not.
#[test]
fn the_left_arm_reaches_the_right_shoulder() {
    assert_eq!(neighbours(1 << 14), 1 << 10 | 1 << 14 | 1 << 15);
    let left_arm = piece(20, &[15], 1);
    let right_shoulder = piece(20, &[11], 1);
    assert!((encumbrance(&[left_arm, right_shoulder], None, NONE) - 0.05).abs() < 1e-6);
    let left_shoulder = piece(20, &[14], 1);
    let right_arm = piece(20, &[12], 1);
    assert!((encumbrance(&[left_shoulder, right_arm], None, NONE) - 0.1).abs() < 1e-6);
}

/// Each Manoeuvring tier takes 0.3 off; the float left of 0.3 − 0.3
/// (1.2e-8) is under 0.001 and weighs nothing.
#[test]
fn manoeuvring_takes_off_a_third() {
    let tier = |manoeuvring| Skills {
        manoeuvring,
        mobility: false,
    };
    assert!((encumbrance(&[plain(200)], None, tier(1)) - 0.2).abs() < 1e-6);
    assert_eq!(encumbrance(&[plain(120)], None, tier(1)), 0.0);
    assert_eq!(encumbrance(&[plain(120), plain(120)], None, tier(3)), 0.0);
}

/// A round shield (bulk 0.6) adds 0.3; with Mobility nothing; a buckler
/// (bulk 0.08) with Mobility nothing either; powered gauntlets halve the
/// round shield's share. The shield adds after the armour is floored.
#[test]
fn a_shield_adds_its_bulk() {
    let mobile = Skills {
        manoeuvring: 0,
        mobility: true,
    };
    assert!((encumbrance(&[], Some(&shield(0.6)), NONE) - 0.3).abs() < 1e-6);
    assert_eq!(encumbrance(&[], Some(&shield(0.6)), mobile), 0.0);
    assert_eq!(encumbrance(&[], Some(&shield(0.08)), mobile), 0.0);
    let gauntlets = |powered| Burden {
        special: Burden::LIGHT_SHIELD,
        powered,
        ..plain(20)
    };
    assert!((encumbrance(&[gauntlets(true)], Some(&shield(0.6)), NONE) - 0.2).abs() < 1e-6);
    assert!((encumbrance(&[gauntlets(false)], Some(&shield(0.6)), NONE) - 0.35).abs() < 1e-6);
    let tier2 = Skills {
        manoeuvring: 2,
        mobility: false,
    };
    assert!((encumbrance(&[plain(80)], Some(&shield(0.6)), tier2) - 0.3).abs() < 1e-6);
}

/// An unpowered golem suit weighs its wearer down to the most there is; a
/// powered one only by its byte. Three plates of 140 are capped at 1.
#[test]
fn the_golem_suit_and_the_cap() {
    let suit = |powered| Burden {
        special: Burden::GOLEM_SUIT,
        powered,
        ..plain(40)
    };
    assert_eq!(encumbrance(&[suit(false)], None, NONE), 1.0);
    assert!((encumbrance(&[suit(true)], None, NONE) - 0.1).abs() < 1e-6);
    let tier3 = Skills {
        manoeuvring: 3,
        mobility: false,
    };
    assert_eq!(encumbrance(&[suit(false)], None, tier3), 1.0);
    assert_eq!(encumbrance(&[plain(140); 3], None, NONE), 1.0);
}

/// What encumbrance slows, at encumbrance 0.5: the run blend tops at 0.85
/// (0.8 sprinting), the sprint clock runs at 0.95, the body turns at 0.7,
/// a standing body aligns to 0.5, long steps are 1.35 and 1.45 step
/// reaches and the get-up clock runs at 1.25.
#[test]
fn what_encumbrance_slows() {
    assert_eq!(run_cap(0.5, false), 0.85);
    assert_eq!(run_cap(0.5, true), 0.8);
    assert_eq!(run_cap(0.0, false), 1.0);
    assert_eq!(sprint_clock(0.5), 0.95);
    assert_eq!(turn_rate(0.5), 0.7);
    assert_eq!(standing_alignment(0.5), 0.5);
    assert_eq!(long_step(0.5), 1.35);
    assert_eq!(footwork_long_step(0.5), 1.45);
    // 80 cm at no encumbrance is 120 cm; at 0.5, 108 cm. A reach already
    // shortened to 72 cm lengthens to 97.2 cm.
    assert_eq!(long_step_length(80.0, 0.0).to_bits(), 0x42f0_0000);
    assert_eq!(long_step_length(80.0, 0.5).to_bits(), 0x42d8_0000);
    assert_eq!(long_step_length(72.0, 0.5).to_bits(), 0x42c2_6666);
    assert!(long_step_length(80.0, f32::NAN).is_nan());
    assert_eq!(get_up_rate(0.5), 1.25);
}

/// An item record with a body of `byte` encumbrance, or a shield or a
/// weapon section.
fn item(id: u32, section: u32, body: Vec<u8>) -> objdb::Item {
    let mut p = section.to_le_bytes().to_vec();
    p.extend((body.len() as u32 + 8).to_le_bytes());
    p.extend(body);
    objdb::Item::parse(id, &p).unwrap()
}

fn apparel(id: u32, byte: u8) -> objdb::Item {
    let mut body = vec![0u8; 0x224];
    body[0x150 + 4 * 8..0x150 + 4 * 9].copy_from_slice(&50u32.to_le_bytes());
    body[0x1A5] = byte;
    item(id, 0x1000, body)
}

/// A record wearing a local mail shirt (40) and a missing item, holding a
/// sword and a round shield, with Armour and Shields skills.
#[test]
fn a_records_encumbrance() {
    let items = [
        apparel(3, 40),
        item(7, 0x8000, vec![0, 0, 0, 0, 50, 58, 60, 0]),
        item(9, 0x4000, vec![0; 0x5A]),
    ];
    let find = |id: u32| items.iter().find(|i| i.id == id & ID_MASK);
    let mut r = characters::CharacterRecord {
        worn: vec![
            ItemRef::new(LOCAL_ITEM | 3, 0),
            ItemRef::new(LOCAL_ITEM | 99, 0),
        ],
        hands: [
            ItemRef::new(LOCAL_ITEM | 9, 0),
            ItemRef::new(LOCAL_ITEM | 7, 0),
            ItemRef::NONE,
            ItemRef::NONE,
        ],
        ..Default::default()
    };
    assert_eq!(held_shield(&r, find).map(|s| s.bulk), Some(0.6));
    assert!((of_record(&r, find) - 0.4).abs() < 1e-6);
    // Manoeuvring I takes the shirt off; Mobility the shield.
    r.skills[ARMOUR] = Skill {
        points: 5000,
        techniques: [1, 0, 0, 0, 0, 0],
    };
    assert!((of_record(&r, find) - 0.3).abs() < 1e-6);
    r.skills[SHIELDS] = Skill {
        points: 5000,
        techniques: [6, 0, 0, 0, 0, 0],
    };
    assert_eq!(of_record(&r, find), 0.0);
    // A weapon in the off hand is no shield.
    r.hands = [
        ItemRef::new(LOCAL_ITEM | 7, 0),
        ItemRef::new(LOCAL_ITEM | 9, 0),
        ItemRef::NONE,
        ItemRef::NONE,
    ];
    assert_eq!(held_shield(&r, find).map(|s| s.bulk), Some(0.6));
    r.hands = [
        ItemRef::new(LOCAL_ITEM | 9, 0),
        ItemRef::NONE,
        ItemRef::new(LOCAL_ITEM | 7, 0),
        ItemRef::NONE,
    ];
    assert_eq!(held_shield(&r, find), None);
}

/// Every level's placed characters with their records and the level's own
/// items.
fn levels(resource: &exa_rpk::MappedArchive) -> Vec<(String, LevelCharacters, Vec<objdb::Item>)> {
    let mut out = Vec::new();
    for e in resource.entries() {
        let Ok(chunks) = crate::chunk::rfc_chunks(resource.bytes(e)) else {
            continue;
        };
        let (mut level, mut local) = (None, Vec::new());
        for c in chunks.flatten() {
            if c.tag == CHARACTERS {
                level = Some(LevelCharacters::parse(c.data).unwrap());
            } else if c.tag == crate::rfc::LOCAL_DATABASE {
                let mut pos = 0;
                while pos + 8 <= c.data.len() {
                    let tag = crate::read_u32(c.data, pos).unwrap();
                    let size = crate::read_u32(c.data, pos + 4).unwrap() as usize;
                    if tag == 0xDBDB {
                        let db = Database::parse(&c.data[pos + 8..pos + 8 + size]).unwrap();
                        local = objdb::items(&db);
                    }
                    pos += 8 + size;
                }
            }
        }
        if let Some(level) = level {
            out.push((e.name.clone(), level, local));
        }
    }
    out
}

fn tally(values: impl Iterator<Item = f32>) -> Vec<(i64, usize)> {
    let mut m = std::collections::BTreeMap::<i64, usize>::new();
    for v in values {
        *m.entry((v * 1000.0).round() as i64).or_default() += 1;
    }
    m.into_iter().collect()
}

/// The encumbrance every living character of the installed story levels
/// spawns with (skipped without the game).
#[test]
fn shipped_level_encumbrance() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let global = objdb::items(&Database::parse(resource.get("objdb.rdb").unwrap()).unwrap());
    let mut living = Vec::new();
    for (_, level, local) in levels(resource) {
        for p in level.placements.iter().filter(|p| !p.is_dead()) {
            let r = level.record(p.id).unwrap();
            let find = |id: u32| {
                let items = if id & LOCAL_ITEM != 0 {
                    &local
                } else {
                    &global
                };
                items.iter().find(|i| i.id == id & ID_MASK)
            };
            let bulk = held_shield(r, find).map(|s| s.bulk);
            living.push((r.name.clone(), of_record(r, find), bulk));
        }
    }
    // 386 of the 451 weigh nothing, 27 hold a shield; the most encumbered
    // are Elestan (0.47 of armour and a round shield's 0.3) and Demian.
    assert_eq!(living.len(), 451);
    let weights = tally(living.iter().map(|l| l.1));
    assert_eq!(weights[0], (0, 386));
    assert_eq!(weights.len(), 31);
    assert_eq!(living.iter().filter(|l| l.2.is_some()).count(), 27);
    let mut heaviest = living.clone();
    heaviest.sort_by(|a, b| b.1.total_cmp(&a.1));
    assert_eq!(
        (heaviest[0].0.as_str(), heaviest[0].2),
        ("Elestan", Some(0.6))
    );
    assert!((heaviest[0].1 - 0.77).abs() < 1e-6);
    assert_eq!(heaviest[1].0, "Demian");
    assert!((heaviest[1].1 - 0.67).abs() < 1e-6);
}

/// The arena's actors, each wearing its own database's items (skipped
/// without the game).
#[test]
fn shipped_actor_encumbrance() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let global = objdb::items(&Database::parse(resource.get("objdb.rdb").unwrap()).unwrap());
    let actors = exa_rpk::Archive::parse(resource.get("actors.rpk").unwrap()).unwrap();
    let mut weights = std::collections::BTreeMap::new();
    for e in actors.entries() {
        let b = actors.bytes(e);
        let r = characters::CharacterRecord::parse(0, b).unwrap();
        let own = characters::actor_items(b).unwrap();
        let find = |id: u32| {
            let items = if id & LOCAL_ITEM != 0 { &own } else { &global };
            items.iter().find(|i| i.id == id & ID_MASK)
        };
        let bulk = held_shield(&r, find).map(|s| s.bulk);
        weights.insert(e.name.clone(), (of_record(&r, find), bulk));
    }
    assert_eq!(weights.len(), 21);
    // Opponent 7 wears 0.175 and holds a round shield (0.3); opponent 13
    // takes its armour off with Manoeuvring II and keeps its buckler's
    // 0.07; opponent 15 is the heaviest without a shield; the demon, the
    // villager and Derrin weigh nothing.
    let near = |name: &str, enc: f32, shield: Option<f32>| {
        let (e, s) = weights[name];
        assert!((e - enc).abs() < 1e-6 && s == shield, "{name}: {e} {s:?}");
    };
    near("opponent03", 0.065, Some(0.08));
    near("opponent05", 0.225, Some(0.3));
    near("opponent07", 0.475, Some(0.6));
    near("opponent10", 0.425, Some(0.6));
    near("opponent13", 0.07, Some(0.14));
    near("opponent15", 0.34, None);
    near("opponent16", 0.06, None);
    near("proctor", 0.195, None);
    near("arenashpkp", 0.025, None);
    for name in [
        "demon a01",
        "villager",
        "xac_derrin",
        "opponent01",
        "opponent08",
    ] {
        near(name, 0.0, None);
    }
}

/// Outfits given by model, as the player's and the duellist's are: the
/// first painted item worn as each model (skipped without the game).
#[test]
fn shipped_outfits() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let global = objdb::items(&Database::parse(resource.get("objdb.rdb").unwrap()).unwrap());
    let outfit = |models: &[&str]| {
        let worn: Vec<Burden> = models
            .iter()
            .filter_map(|m| objdb::worn_as(&global, m)?.burden)
            .collect();
        encumbrance(&worn, None, NONE)
    };
    assert_eq!(
        outfit(&["leathjac_a", "trouser_a", "boots_b", "helm_b"]),
        0.175
    );
    assert_eq!(outfit(&["braies_a"]), 0.0);
    // A tourney plate harness, helm and leggings (120, 80, 80) of one set,
    // each touching another: 0.7 less 3 × 0.025.
    let plate = outfit(&["fptdhrnsa", "fptdhelma", "fptdlegsa"]);
    assert!((plate - 0.625).abs() < 1e-6, "{plate}");
}

/// A worn item of special kind 2 grants its wearer a second wind; nothing
/// else does, and a record grants it through the item its worn id finds.
#[test]
fn a_second_wind_talisman_grants_it() {
    let talisman = Burden {
        special: Burden::SECOND_WIND,
        ..plain(5)
    };
    assert!(second_wind(&[talisman]));
    assert!(!second_wind(&[plain(5)]));
    assert!(!second_wind(&[]));
    let record = characters::CharacterRecord {
        worn: vec![ItemRef {
            id: 7,
            ..Default::default()
        }],
        ..Default::default()
    };
    // An item body of special kind 2, parsed as the object database gives
    // it (only the 0x1000 section, with the special byte).
    let item = |special: u8| {
        let mut body = vec![0u8; 0x218];
        body[0x214] = special;
        let mut payload = 0x1000u32.to_le_bytes().to_vec();
        payload.extend((body.len() as u32 + 8).to_le_bytes());
        payload.extend_from_slice(&body);
        objdb::Item::parse(7, &payload).unwrap()
    };
    let found = item(Burden::SECOND_WIND);
    assert!(second_wind_of_record(&record, |id| (id == 7).then_some(&found)));
    let plain_item = item(0);
    assert!(!second_wind_of_record(&record, |_| Some(&plain_item)));
    assert!(!second_wind_of_record(&record, |_| None));
}

/// The armour pass's special kinds: 2 the second wind, 3 healing, 4 golem
/// suits, 5 the mind ward, 6 the gauntlets, 7 the focus charm (a set
/// value, the same for two charms); 0 and 1 do nothing here.
#[test]
fn the_special_kinds_dispatch_in_wearing_order() {
    let worn = |special: u8, power_flags: u8, charge: u8| Burden {
        special,
        power_flags,
        charge,
        ..plain(5)
    };
    let specials = WornSpecials::of_worn([
        (10, worn(Burden::SEES_IN_THE_DARK, 0, 0)),
        (11, worn(Burden::HEALING, 2, 7)),
        (12, worn(Burden::GOLEM_SUIT, 1, 0)),
        (13, worn(Burden::LIGHT_SHIELD, 1, 0xff)),
        (14, worn(Burden::HEALING, 0x22, 0)),
        (15, worn(Burden::FOCUS_CHARM, 0, 0)),
        (16, worn(Burden::FOCUS_CHARM, 0, 0)),
    ]);
    assert!(!specials.second_wind);
    assert!(!specials.mind_ward);
    let item = |special, id, power_flags, charge| EffectItem {
        special,
        id,
        power_flags,
        charge,
    };
    assert_eq!(
        specials.effects,
        vec![
            item(Burden::HEALING, 11, 2, 7),
            item(Burden::GOLEM_SUIT, 12, 1, 0),
            item(Burden::LIGHT_SHIELD, 13, 1, 0xff),
            item(Burden::HEALING, 14, 0x22, 0),
        ]
    );
    assert_eq!(specials.focus_regen.to_bits(), 0x38d2_8eb4);
    assert!((specials.focus_regen - 1.004_02e-4).abs() < 1e-9);

    let ward = WornSpecials::of_worn([
        (1, worn(Burden::MIND_WARD, 0, 0)),
        (2, worn(Burden::SECOND_WIND, 0, 0)),
    ]);
    assert!(ward.mind_ward && ward.second_wind);
    assert_eq!(ward.focus_regen, 0.0);
    assert_eq!(WornSpecials::of_worn([]), WornSpecials::default());
}

/// A record's specials come from the items its worn ids find, with their
/// crystal bytes as the body stores them.
#[test]
fn a_record_resolves_its_specials() {
    let record = characters::CharacterRecord {
        worn: vec![
            ItemRef {
                id: 7,
                ..Default::default()
            },
            ItemRef {
                id: 8,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let mut body = vec![0u8; 0x21a];
    body[0x214] = Burden::HEALING;
    body[0x218] = 0x2;
    body[0x219] = 0x40;
    let mut payload = 0x1000u32.to_le_bytes().to_vec();
    payload.extend((body.len() as u32 + 8).to_le_bytes());
    payload.extend_from_slice(&body);
    let mask = objdb::Item::parse(7, &payload).unwrap();
    let specials = WornSpecials::of_record(&record, |id| (id == 7).then_some(&mask));
    assert_eq!(
        specials.effects,
        vec![EffectItem {
            special: Burden::HEALING,
            id: 7,
            power_flags: 2,
            charge: 0x40,
        }]
    );
    assert_eq!(
        WornSpecials::of_record(&record, |_| None),
        WornSpecials::default()
    );
}

/// The shipped object database carries every special kind the armour pass
/// dispatches (2 through 7); a kind-3 healing item stores the crystal
/// bytes its tick reads.
#[test]
fn shipped_special_kinds() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let global = objdb::items(&Database::parse(resource.get("objdb.rdb").unwrap()).unwrap());
    let kinds: Vec<(u32, Burden)> = global
        .iter()
        .filter_map(|item| Some((item.id, item.burden?)))
        .filter(|(_, b)| b.special > 1)
        .collect();
    for kind in 2..=7 {
        assert!(
            kinds.iter().any(|(_, b)| b.special == kind),
            "no item of special kind {kind}"
        );
    }
    let specials = WornSpecials::of_worn(kinds.iter().copied());
    assert!(specials.second_wind && specials.mind_ward);
    assert_eq!(specials.focus_regen, FOCUS_CHARM_REGEN);
    assert!(
        specials
            .effects
            .iter()
            .any(|e| e.special == Burden::HEALING)
    );
}

/// The shipped database's layers over the eleventh bone: 128, of which 85
/// carry the body's head mark and 43 do not; the display coverage runs 0 to
/// 10 percent. A full helm (`helm_b`, 8 %) adds 8 cm at an average
/// physique. Skipped without the game.
#[test]
fn shipped_head_marks() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let global = objdb::items(&Database::parse(resource.get("objdb.rdb").unwrap()).unwrap());
    let mut marked = 0;
    let mut unmarked = 0;
    let mut most = 0u8;
    for item in &global {
        let Some(a) = item.armour else { continue };
        if a.coverage[10] == 0 {
            continue;
        }
        most = most.max(a.display);
        if a.head_height_mark {
            marked += 1;
        } else {
            unmarked += 1;
        }
    }
    assert_eq!((marked, unmarked), (85, 43));
    assert_eq!(most, 10);
    // The heaviest helm's display is 10; at the average physique of ½ the
    // extra height is that many centimetres.
    let helms: Vec<objdb::Armour> = global
        .iter()
        .filter_map(|i| i.armour)
        .filter(|a| a.coverage[10] != 0 && a.head_height_mark)
        .collect();
    assert_eq!(head_extra(&helms, 0.5), 10.0);
    // A tenth of a physique above the average adds 0.8 cm.
    assert!((head_extra(&helms, 0.6) - 10.8).abs() < 1e-6);
}
