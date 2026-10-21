use super::*;
use crate::characters::{CHARACTERS, LevelCharacters};
use crate::races::{self, Natural, RaceBody, RaceStats};
use crate::weapon_stats::at;

fn race(byte: u8, health: f32) -> Race {
    Race {
        id: u32::from(byte) + 1,
        name: String::new(),
        flags: 0,
        look: 0,
        stem: String::new(),
        movesets: [String::new(), String::new()],
        health,
        natural: Natural::default(),
        body: RaceBody::FALLBACK,
        stats: RaceStats::FALLBACK,
        voice: crate::races::RaceVoice::default(),
    }
}

/// The human of `races.rdb` as far as the derivation reads it.
fn human() -> Race {
    let mut r = race(0, 1.0);
    r.flags = 0xF;
    r.body.pin_factor = 1.25;
    r.stats.perception = 1.0;
    r.stats.voice_volume = 1.0;
    r.stats.attack_voices = 4;
    r
}

fn record(byte: u8, height: f32, feral: f32) -> CharacterRecord {
    CharacterRecord {
        race: byte,
        height,
        feral,
        ..Default::default()
    }
}

const ALIVE: Spawn = Spawn {
    story: true,
    dead: false,
};

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

/// Height −1 (tall) to 1 (short): scale and size × 1.05 to 0.95, the
/// standing pelvis 120.75 to 98.25 cm, the leg reach 118.3 to 96.1 cm, the
/// gait clock × 0.909 to 1.111; out-of-range heights are kept within −1…1.
#[test]
fn height_scales_the_body() {
    let human = human();
    let short = CharacterStats::derive(&record(0, 1.0, 0.0), Some(&human), ALIVE);
    assert_eq!((short.scale, short.size), (0.95, 0.95));
    assert!(close(short.standing_height, 0.9825));
    assert!(close(short.leg_reach, 0.961));
    assert!(close(short.gait_clock, 1.0 / 0.9));
    let tall = CharacterStats::derive(&record(0, -1.0, 0.0), Some(&human), ALIVE);
    assert_eq!((tall.scale, tall.size), (1.05, 1.05));
    assert!(close(tall.standing_height, 1.2075));
    assert!(close(tall.leg_reach, 1.183));
    assert!(close(tall.gait_clock, 1.0 / 1.1));
    let average = CharacterStats::derive(&record(0, 0.0, 0.0), Some(&human), ALIVE);
    assert_eq!((average.scale, average.gait_clock), (1.0, 1.0));
    assert!(close(average.standing_height, 1.095));
    assert!(close(average.leg_reach, 1.072));
    let beyond = CharacterStats::derive(&record(0, 3.0, 0.0), Some(&human), ALIVE);
    assert_eq!((beyond.height, beyond.scale), (1.0, 0.95));
}

/// The race's scale is capped at 1.5 before the height: a tall harvester
/// (scale 1.8, size 1.8) has scale 1.575 and size 1.89.
#[test]
fn the_scale_is_capped_before_the_height() {
    let mut harvester = race(10, 4.0);
    harvester.body.scale = 1.8;
    harvester.body.size = 1.8;
    let s = CharacterStats::derive(&record(10, -1.0, 0.0), Some(&harvester), ALIVE);
    assert!(close(s.scale, 1.575));
    assert!(close(s.size, 1.89));
}

/// The feral level weakens the undead: at 1.0 the most energy is 0.9, the
/// support pins' factor 1.0, the head has no pose gain and a fifth of its
/// muscle, the walking arms 0.05, the perception 0.5, the step reach 56 cm;
/// at 0.5 they are 0.95, 1.1875, a half, 0.6, 0.125, 0.75 and 68 cm.
#[test]
fn feral_weakens_the_undead() {
    let human = human();
    let zombie = CharacterStats::derive(&record(0, 0.0, 1.0), Some(&human), ALIVE);
    assert_eq!(zombie.max_energy, 0.9);
    assert_eq!(zombie.pin_factor, 1.0);
    assert_eq!(zombie.head_gain, 0.0);
    assert!(close(zombie.head_factor, 0.2));
    assert_eq!(zombie.walking_arms, 0.05);
    assert_eq!(zombie.perception, 0.5);
    assert_eq!(zombie.attack_voice_step, 1);
    assert!(close(zombie.step_reach(0.0), 0.56));
    let half = CharacterStats::derive(&record(0, 0.0, 0.5), Some(&human), ALIVE);
    assert_eq!(half.max_energy, 0.95);
    assert_eq!(half.pin_factor, 1.1875);
    assert_eq!(half.head_gain, 0.5);
    assert!(close(half.head_factor, 0.6));
    assert!(close(half.walking_arms, 0.125));
    assert_eq!(half.perception, 0.75);
    assert!(close(half.step_reach(0.0), 0.68));
    let living = CharacterStats::derive(&record(0, 0.0, 0.0), Some(&human), ALIVE);
    assert_eq!(
        (living.max_energy, living.pin_factor, living.head_gain),
        (1.0, 1.25, 1.0)
    );
    assert_eq!((living.head_factor, living.walking_arms), (1.0, 0.2));
    assert_eq!((living.perception, living.attack_voice_step), (1.0, 3));
    assert!(close(living.step_reach(0.0), 0.8));
    // Feral levels beyond 1 are kept within 0…1.
    let beyond = CharacterStats::derive(&record(0, 0.0, 2.0), Some(&human), ALIVE);
    assert_eq!((beyond.feral, beyond.max_energy), (1.0, 0.9));
}

/// The step reach is 80 cm·√scale, 20 % shorter per unit of encumbrance:
/// an average human at half encumbrance reaches 72 cm, an ogre (scale 1.5)
/// unencumbered 98 cm.
#[test]
fn reach_follows_scale_and_encumbrance() {
    let human = CharacterStats::derive(&record(0, 0.0, 0.0), Some(&human()), ALIVE);
    assert!(close(human.step_reach(0.5), 0.72));
    let mut ogre = race(3, 6.0);
    ogre.body.scale = 1.5;
    let ogre = CharacterStats::derive(&record(3, 0.0, 0.0), Some(&ogre), ALIVE);
    assert!(close(ogre.step_reach(0.0), 0.8 * 1.5f32.sqrt()));
}

/// The init leaves the encumbrance at 0 for the armour pass to sum; the
/// stats keep it and whether Deflection is learnt.
#[test]
fn encumbrance_after_the_init() {
    let mut r = record(0, 0.0, 0.0);
    let plain = CharacterStats::derive(&r, Some(&human()), ALIVE);
    assert_eq!((plain.encumbrance, plain.deflection), (0.0, false));
    let worn = plain.with_encumbrance(0.5);
    assert_eq!(worn.encumbrance, 0.5);
    assert!(close(worn.step_reach(worn.encumbrance), 0.72));
    r.skills[crate::characters::ARMOUR] = crate::characters::Skill {
        points: 5000,
        techniques: [4, 0, 0, 0, 0, 0],
    };
    assert!(CharacterStats::derive(&r, Some(&human()), ALIVE).deflection);
}

/// The weapons a character holds are held by its size, scale and
/// encumbrance: a new human by the average holder, a tall one (height −1)
/// in armour of 0.5 by size and scale 1.05 at 0.5; the step reach is the
/// weapon record's, in metres.
#[test]
fn weapons_are_held_by_the_character() {
    let plain = CharacterStats::derive(&record(0, 0.0, 0.0), Some(&human()), ALIVE);
    assert_eq!(plain.holder(), Holder::AVERAGE);
    let tall =
        CharacterStats::derive(&record(0, -1.0, 0.0), Some(&human()), ALIVE).with_encumbrance(0.5);
    let h = tall.holder();
    assert!(close(h.size, 1.05) && close(h.scale, 1.05));
    assert_eq!(
        (h.encumbrance, h.weightless, h.half_sword),
        (0.5, 0.0, false)
    );
    assert_eq!(
        tall.step_reach(0.5),
        weapon_record::step_reach(tall.scale, tall.feral, 0.5) / 100.0
    );
    // The arena's 90 cm sword (weight 50, balance 80, striking 20 to 200)
    // in the tall armoured hand: reach 60 × 1.05 + 57.6 cm, recoil 0.67,
    // swing shaping 0.275, blend 8.
    let mut bytes = crate::weapon_stats::StatBytes::default();
    for (off, v) in [
        (at::WEIGHT, 50),
        (at::BALANCE, 80),
        (at::KIND, 2),
        (at::START, 20),
        (at::END, 200),
        (at::SLASH, 72),
    ] {
        bytes.set(off, v);
    }
    let sword = weapon_record::Section {
        flags: 0x11,
        stats: bytes,
    };
    let lengths = weapon_record::Lengths::one_part(90.0, 0.1, 1.0);
    let w = weapon_record::WeaponRecord::build(&sword, false, lengths, &h);
    assert!(close(w.reach, 120.6) && close(w.recoil, 0.67), "{w:?}");
    assert!(close(w.swing_shaping, 0.275));
    assert_eq!(w.blend, 8);
}

/// Maximum health is 0.25 times the race's factor, but for the race bytes
/// the init rules: the ogre 1.5 and Sir 1.0 whatever their factor, golems
/// 0.5 and the golem boss 1.0, and the ancient, the Fael, the ghouls, the
/// gobbler and the manipulator their own (the manipulator 0.375 against
/// its race's 0.25); the skeleton and the wraith keep their race's.
#[test]
fn maximum_health_by_race_byte() {
    let max = |byte: u8, factor: f32| {
        CharacterStats::derive(&record(byte, 0.0, 0.0), Some(&race(byte, factor)), ALIVE).max_health
    };
    assert_eq!(max(0, 1.0), 0.25);
    assert_eq!(max(1, 3.0), 0.75);
    assert_eq!(max(2, 9.0), 0.375);
    assert_eq!(max(3, 2.0), 1.5);
    assert_eq!(max(4, 7.0), 1.0);
    assert_eq!(max(5, 9.0), 0.75);
    assert_eq!(max(6, 9.0), 0.375);
    assert_eq!(max(7, 9.0), 0.5);
    assert_eq!(max(8, 9.0), 0.5);
    assert_eq!(max(9, 1.0), 0.375);
    assert_eq!(max(10, 4.0), 1.0);
    assert_eq!(max(14, 3.0), 0.75);
    assert_eq!(max(16, 9.0), 0.5);
    assert_eq!(max(17, 9.0), 1.0);
    assert_eq!(max(18, 9.0), 0.5);
    // Without its race the factor is the built-in record's 1.
    let unknown = CharacterStats::derive(&record(13, 0.0, 0.0), None, ALIVE);
    assert_eq!(unknown.max_health, 0.25);
}

/// Health starts at the saved pools' shares of the maximum (×1.5259022e-5
/// each); a living character whose record saved no red health spawns
/// full, a corpse does not.
#[test]
fn health_from_the_saved_pools() {
    let human = human();
    let mut r = record(0, 0.0, 0.0);
    r.pools = Pools {
        yellow: 0x8000,
        red: 0xFFFF,
        focus: 0,
    };
    let s = CharacterStats::derive(&r, Some(&human), ALIVE);
    assert!(close(s.yellow, 0.125_001_9), "{}", s.yellow);
    assert!(close(s.red, 0.25), "{}", s.red);
    r.pools = Pools {
        yellow: 0,
        red: 0,
        focus: 0,
    };
    let arena = Spawn {
        story: false,
        dead: false,
    };
    let s = CharacterStats::derive(&r, Some(&human), arena);
    assert!(close(s.yellow, 0.25) && close(s.red, 0.25));
    let corpse = Spawn {
        story: true,
        dead: true,
    };
    let s = CharacterStats::derive(&r, Some(&human), corpse);
    assert_eq!((s.yellow, s.red), (0.0, 0.0));
}

/// The rule bits: a living human that is not feral takes remedies and, in
/// the story, may rise (0x12; 0x2 in the arena); a feral one takes no
/// lasting wounds (0x8), fierce over half feral (0xC); skeletons 0x8; the
/// ogre 0x67; the Fael 0x7; a golem guard 0x140; the golem suit 0x170 in
/// the story; a golem wearing its kind's core drops 0x100.
#[test]
fn body_rule_bits() {
    let rules = |r: &Race, byte: u8, feral: f32, story: bool| {
        let spawn = Spawn { story, dead: false };
        CharacterStats::derive(&record(byte, 0.0, feral), Some(r), spawn)
            .rules
            .0
    };
    let human = human();
    assert_eq!(rules(&human, 0, 0.0, true), 0x12);
    assert_eq!(rules(&human, 0, 0.0, false), 0x2);
    assert_eq!(rules(&human, 0, 0.4, true), 0x8);
    assert_eq!(rules(&human, 0, 0.84, true), 0xC);
    assert_eq!(rules(&race(1, 1.0), 1, 0.0, true), 0x8);
    let mut ogre = race(3, 6.0);
    (ogre.flags, ogre.stats.flags2) = (0x27, 0x7);
    assert_eq!(rules(&ogre, 3, 0.0, true), 0x67);
    let mut fael = race(5, 3.0);
    (fael.flags, fael.stats.flags2) = (0x3, 0x3);
    assert_eq!(rules(&fael, 5, 0.0, true), 0x7);
    let mut guard = race(16, 2.0);
    guard.flags = 0x1030;
    assert_eq!(rules(&guard, 16, 0.0, true), 0x140);
    let mut suit = race(18, 2.0);
    suit.flags = 0x20B8;
    assert_eq!(rules(&suit, 18, 0.0, true), 0x170);
    assert_eq!(rules(&suit, 18, 0.0, false), 0x160);
    // A golem wearing its kind's core (item 500 for the `0x2000` kind, 431
    // for the `0x1000` kind) regenerates red health; another's does not.
    let cored = |r: &Race, byte: u8, core: u32| {
        let mut rec = record(byte, 0.0, 0.0);
        rec.worn = vec![ItemRef::new(7, 0), ItemRef::new(core, 0)];
        CharacterStats::derive(&rec, Some(r), ALIVE).rules.0
    };
    assert_eq!(cored(&suit, 18, 500), 0x70);
    assert_eq!(cored(&suit, 18, 431), 0x170);
    assert_eq!(cored(&guard, 16, 431), 0x40);
    let r = BodyRules(0x170);
    assert!(r.has(BodyRules::REGENERATES_RED) && r.has(BodyRules::NO_RED_REGENERATION));
    assert!(!r.has(BodyRules::TAKES_REMEDIES));
    // Red health regenerates with 0x20 and without 0x100: the ogre and the
    // cored suit, not the coreless suit, a guard or a human.
    let regenerates = |bits: u32| BodyRules(bits).regenerates_red();
    assert!(regenerates(0x67) && regenerates(0x70));
    assert!(!regenerates(0x170) && !regenerates(0x140) && !regenerates(0x12));
}

/// A race whose second flag word holds 0x8 (the harvester) walks on four
/// limbs: its stab is resolved as a swing. The ogre's 0x7 does not.
#[test]
fn quadrupeds() {
    let stats = |flags2: u32| {
        let mut r = race(10, 4.0);
        r.stats.flags2 = flags2;
        CharacterStats::derive(&record(10, 0.0, 0.0), Some(&r), ALIVE).quadruped
    };
    assert!(stats(0x8));
    assert!(!stats(0x7));
    assert!(!CharacterStats::default().quadruped);
}

/// The voice: the record's pitch kept within 0.9…1.07 (the hulking ghoul
/// 0.85), the volume the scale times the race's factor, the attack-voice
/// step the race's voices less one.
#[test]
fn voice() {
    let human = human();
    let pitch = |p: f32| {
        let mut r = record(0, 0.0, 0.0);
        r.voice_pitch = p;
        CharacterStats::derive(&r, Some(&human), ALIVE).voice_pitch
    };
    assert_eq!((pitch(0.8), pitch(1.0), pitch(1.2)), (0.9, 1.0, 1.07));
    let mut ghoul = race(7, 2.0);
    ghoul.stats.voice_volume = 2.0;
    let g = CharacterStats::derive(&record(7, 0.0, 0.0), Some(&ghoul), ALIVE);
    assert_eq!((g.voice_pitch, g.voice_volume), (0.85, 2.0));
    let mut fael = race(5, 3.0);
    fael.body.scale = 1.35;
    fael.stats.voice_volume = 1.5;
    let f = CharacterStats::derive(&record(5, 0.0, 0.0), Some(&fael), ALIVE);
    assert!(close(f.voice_volume, 2.025));
    let skeleton = CharacterStats::derive(&record(1, 0.0, 0.0), Some(&race(1, 1.0)), ALIVE);
    assert_eq!(skeleton.attack_voice_step, -1);
}

/// A new record of no known race: scale 1, the most energy 1, health
/// 0.25 full, the built-in record's factors (pins 1, no gait push).
#[test]
fn default_is_a_new_record_of_no_race() {
    let s = CharacterStats::default();
    assert_eq!((s.scale, s.size, s.max_energy), (1.0, 1.0, 1.0));
    assert!(close(s.yellow, 0.25) && close(s.red, 0.25));
    assert_eq!((s.max_health, s.pin_factor, s.clip_rate), (0.25, 1.0, 1.0));
    assert_eq!(s.sprint.thrust, 0.0);
    assert_eq!(s.rules, BodyRules(BodyRules::TAKES_REMEDIES));
}

/// Every placed character of the installed story levels (skipped without
/// the game), derived as the original spawns them.
#[test]
fn shipped_level_characters() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let races = races::parse(resource.get("races.rdb").unwrap()).unwrap();
    let mut placed = Vec::new();
    for e in resource.entries() {
        let Ok(chunks) = crate::chunk::rfc_chunks(resource.bytes(e)) else {
            continue;
        };
        for c in chunks.flatten().filter(|c| c.tag == CHARACTERS) {
            let level = LevelCharacters::parse(c.data).unwrap();
            for p in &level.placements {
                let r = level.record(p.id).unwrap().clone();
                placed.push((e.name.clone(), p.is_dead(), r));
            }
        }
    }
    let derived: Vec<(bool, CharacterRecord, CharacterStats)> = placed
        .into_iter()
        .map(|(name, dead, r)| {
            assert!(name.to_ascii_lowercase().starts_with("exanima"), "{name}");
            let spawn = Spawn { story: true, dead };
            let s = CharacterStats::derive(&r, races::of_character(&races, r.race), spawn);
            (dead, r, s)
        })
        .collect();
    let living: Vec<_> = derived.iter().filter(|(dead, ..)| !dead).collect();
    assert_eq!((derived.len(), living.len()), (477, 451));
    // Every living character spawns at full health (every one saved full
    // pools), its scale and size follow its height, its step reach its
    // scale and feral level.
    for (_, r, s) in &living {
        assert!(
            s.yellow == s.max_health && s.red == s.max_health,
            "{}",
            r.name
        );
        let race = races::of_character(&races, r.race).unwrap();
        let shrink = 1.0 - 0.05 * r.height;
        assert!(
            close(s.scale, race.body.scale.min(1.5) * shrink),
            "{}",
            r.name
        );
        assert!(close(s.size, race.body.size * shrink), "{}", r.name);
        let feral = 1.0 - 0.3 * s.feral;
        assert!(close(s.step_reach(0.0), 0.8 * s.scale.sqrt() * feral));
    }
    let humans: Vec<&CharacterStats> = living
        .iter()
        .filter(|(_, r, _)| r.race == 0)
        .map(|(_, _, s)| s)
        .collect();
    let tally = |f: &dyn Fn(&CharacterStats) -> f32| {
        let mut m = std::collections::BTreeMap::<i64, usize>::new();
        for s in &humans {
            *m.entry((f(s) * 10_000.0).round() as i64).or_default() += 1;
        }
        m.into_iter().collect::<Vec<_>>()
    };
    // The 97 living humans, all feral (0.398 ×1, 0.424 ×1, 0.5 ×27, 0.842
    // ×21, 1.0 ×47): the most energy, the support pins' factor, the head's
    // pose gain, the walking arms and the sprint thrust (per second).
    assert_eq!(
        tally(&|s| s.max_energy),
        [(9000, 47), (9158, 21), (9500, 27), (9576, 1), (9602, 1)]
    );
    assert_eq!(
        tally(&|s| s.pin_factor),
        [
            (10000, 47),
            (10729, 21),
            (11875, 27),
            (12050, 1),
            (12104, 1)
        ]
    );
    assert_eq!(
        tally(&|s| s.head_gain),
        [(0, 47), (1583, 21), (5000, 27), (5755, 1), (6018, 1)]
    );
    assert_eq!(
        tally(&|s| s.walking_arms),
        [(500, 47), (737, 21), (1250, 27), (1363, 1), (1403, 1)]
    );
    assert_eq!(
        tally(&|s| s.sprint.thrust / 100.0),
        [
            (35000, 47),
            (36934, 21),
            (41111, 27),
            (42034, 1),
            (42355, 1)
        ]
    );
    // Heights from −1 to 1: scales 0.95 (×3) to 1.05 (×4); step reaches
    // 54.6 to 70.4 cm.
    let scales = tally(&|s| s.scale);
    assert_eq!(
        (scales[0], scales[scales.len() - 1]),
        ((9500, 3), (10500, 4))
    );
    let reaches = tally(&|s| s.step_reach(0.0));
    assert_eq!(
        (reaches[0], reaches[reaches.len() - 1]),
        ((5458, 2), (7044, 1))
    );
    // Rule bits: no lasting wounds, fierce over half feral; none takes
    // remedies or may rise.
    let fierce = humans.iter().filter(|s| s.rules.0 == 0xC).count();
    let plain = humans.iter().filter(|s| s.rules.0 == 0x8).count();
    assert_eq!((fierce, plain), (68, 29));
    // The other races, none feral: count, maximum health and rule bits by
    // race byte (every placed golem wears its core: no 0x100).
    let mut others = std::collections::BTreeMap::<u8, (usize, f32, u32)>::new();
    for (_, r, s) in &living {
        if r.race != 0 {
            assert_eq!(s.max_energy, 1.0);
            let e = others.entry(r.race).or_insert((0, s.max_health, s.rules.0));
            assert_eq!((e.1, e.2), (s.max_health, s.rules.0), "{}", r.name);
            e.0 += 1;
        }
    }
    assert_eq!(
        others.into_iter().collect::<Vec<_>>(),
        [
            (1, (128, 0.25, 0x8)),
            (2, (23, 0.375, 0x8)),
            (4, (1, 1.0, 0x67)),
            (5, (6, 0.75, 0x7)),
            (6, (56, 0.375, 0x7)),
            (7, (41, 0.5, 0x7)),
            (8, (10, 0.5, 0x7)),
            (9, (19, 0.375, 0x7)),
            (10, (3, 1.0, 0x65)),
            (16, (46, 0.5, 0x40)),
            (17, (1, 1.0, 0x40)),
            (19, (10, 0.5, 0x40)),
            (20, (10, 0.5, 0x40)),
        ]
    );
    // Red health regenerates for four of them: Sir (race byte 4) and the
    // three harvesters (race byte 10); the cored golems' bits hold no 0x20.
    let mut red: Vec<u8> = living
        .iter()
        .filter(|(_, _, s)| s.rules.regenerates_red())
        .map(|(_, r, _)| r.race)
        .collect();
    red.sort_unstable();
    assert_eq!(red, [4, 10, 10, 10]);
}

/// The arena's actors (skipped without the game).
#[test]
fn shipped_arena_actors() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let races = races::parse(resource.get("races.rdb").unwrap()).unwrap();
    let actors = exa_rpk::Archive::parse(resource.get("actors.rpk").unwrap()).unwrap();
    let derived: Vec<(String, CharacterRecord, CharacterStats)> = actors
        .entries()
        .iter()
        .map(|e| {
            let r = CharacterRecord::parse(0, actors.bytes(e)).unwrap();
            let spawn = Spawn {
                story: false,
                dead: false,
            };
            let s = CharacterStats::derive(&r, races::of_character(&races, r.race), spawn);
            (e.name.clone(), r, s)
        })
        .collect();
    assert_eq!(derived.len(), 21);
    // Every opponent and the proctor saved no health, yet each spawns full
    // (a human's 0.25, the demon's 1.25); none is feral.
    for (name, r, s) in &derived {
        assert!(s.yellow == s.max_health && s.red == s.max_health, "{name}");
        assert_eq!((s.max_energy, s.feral), (1.0, 0.0), "{name}");
        if name.starts_with("opponent") || name == "proctor" {
            assert_eq!((r.pools.yellow, r.pools.red), (0, 0), "{name}");
        }
    }
    let actor = |name: &str| &derived.iter().find(|(n, ..)| n == name).unwrap().2;
    let demon = actor("demon a01");
    assert_eq!((demon.max_health, demon.scale), (1.25, 1.5));
    assert_eq!((demon.pin_factor, demon.rules.0), (1.3, 0x65));
    // Of the arena's actors only the demon regenerates red health.
    let red: Vec<&str> = derived
        .iter()
        .filter(|(_, _, s)| s.rules.regenerates_red())
        .map(|(n, ..)| n.as_str())
        .collect();
    assert_eq!(red, ["demon a01"]);
    // The tallest opponent (height −1) and the shortest (0.84).
    let tall = actor("opponent04");
    assert_eq!(tall.scale, 1.05);
    assert!(
        close(tall.step_reach(0.0), 0.819_756),
        "{}",
        tall.step_reach(0.0)
    );
    let short = actor("opponent05");
    assert!((short.scale - 0.9579).abs() < 1e-4, "{}", short.scale);
    // In the arena no living human may rise: each takes remedies only.
    assert!(
        derived
            .iter()
            .filter(|(_, r, _)| r.race == 0)
            .all(|(_, _, s)| s.rules.0 == BodyRules::TAKES_REMEDIES)
    );
}

/// The attack reach: √scale × (1 − 0.3 age, omitted when the
/// age is negative) × (1 − 0.2 encumbrance) × 80 cm.
#[test]
fn the_attack_reach() {
    let human = CharacterStats::default();
    assert!((human.attack_reach(0.0) - 80.0).abs() < 1e-6);
    assert!((human.attack_reach(0.5) - 68.0).abs() < 1e-6);
    assert!((human.attack_reach(-1.0) - 80.0).abs() < 1e-6);
    let armoured = CharacterStats {
        encumbrance: 0.5,
        ..CharacterStats::default()
    };
    assert!((armoured.attack_reach(0.0) - 72.0).abs() < 1e-6);
    let big = CharacterStats {
        scale: 4.0,
        ..CharacterStats::default()
    };
    assert!((big.attack_reach(0.0) - 160.0).abs() < 1e-6);
    let both = CharacterStats {
        scale: 2.25,
        encumbrance: 0.25,
        ..CharacterStats::default()
    };
    // 1.5 × 1 × 0.95 × 80.
    assert!((both.attack_reach(0.0) - 114.0).abs() < 1e-5);
    // A NaN age is not negative: the aged product is NaN.
    assert!(human.attack_reach(f32::NAN).is_nan());
}
