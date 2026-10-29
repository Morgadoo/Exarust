//! Taking damage as the original's damage call does: its order, the rule
//! bits and state flags it reads, the vital argument and the story's red
//! loss, pinned to worked numbers (single-precision values of the
//! original's double-precision arithmetic).
use super::*;

const HUMAN: BodyRules = BodyRules(BodyRules::TAKES_REMEDIES);
const FERAL: BodyRules = BodyRules(BodyRules::FIERCE | BodyRules::NO_LASTING_WOUNDS);

fn health(yellow: f32, red: f32) -> Health {
    Health {
        yellow,
        red,
        max: 0.25,
        ..Health::default()
    }
}

fn vital(yellow: f32, red: f32) -> Damage {
    Damage {
        yellow,
        red,
        vital: true,
    }
}

/// `h` after taking `damage` as a human out of the story, with what it
/// left.
fn took(mut h: Health, damage: Damage) -> (Taken, Health) {
    let taken = h.take(damage, HUMAN, StateFlags::NONE, false);
    (taken, h)
}

#[test]
fn health_and_death() {
    let mut h = Health::of_race(1.0);
    assert_eq!(h.max, 0.25);
    let light = Outcome {
        red: 0.1,
        yellow: 0.5,
        ..Outcome::default()
    };
    let taken = h.take(Damage::blow(&light), HUMAN, StateFlags::NONE, false);
    assert_eq!(taken, Taken::Hurt);
    assert!(h.yellow < 0.25 && h.red < 0.25);
    let heavy = Outcome {
        red: 2.0,
        yellow: 5.0,
        ..Outcome::default()
    };
    assert!(Damage::blow(&heavy).vital);
    let taken = h.take(Damage::blow(&heavy), HUMAN, StateFlags::NONE, false);
    assert_eq!(taken, Taken::Killed);
    assert_eq!(h.yellow, 0.0);
}

/// Yellow is cut first, then scaled by the share of red left: from full
/// health Y 1 and R 0.5 leave red 0.15 and yellow (0.25 − 0.1) × 0.6 =
/// 0.09 (scaling first would leave 0.05). The rate starts its ramp again.
#[test]
fn the_red_share_scales_the_cut_yellow() {
    let full = Health {
        rate: crate::regen::RATE_CAP,
        ..health(0.25, 0.25)
    };
    let (taken, h) = took(full, vital(1.0, 0.5));
    assert_eq!(taken, Taken::Hurt);
    assert_eq!((h.yellow, h.red, h.rate), (0.09, 0.15, 0.0));
}

/// With no lasting wounds (rule bit 0x8: feral humans, skeletons, the
/// ancients) red is untouched and yellow takes a fifth of the red amount
/// as well: Y 0.5 and R 0.3 leave a feral human yellow 0.14 and red 0.25,
/// a human yellow 0.152 and red 0.19.
#[test]
fn no_lasting_wounds_take_red_damage_from_yellow() {
    let mut feral = health(0.25, 0.25);
    let taken = feral.take(vital(0.5, 0.3), FERAL, StateFlags::NONE, false);
    assert_eq!(taken, Taken::Hurt);
    assert_eq!((feral.yellow, feral.red), (0.14, 0.25));
    let mut skeleton = health(0.25, 0.25);
    let rules = BodyRules(BodyRules::NO_LASTING_WOUNDS);
    skeleton.take(vital(0.5, 0.3), rules, StateFlags::NONE, false);
    assert_eq!(skeleton, feral);
    let (_, human) = took(health(0.25, 0.25), vital(0.5, 0.3));
    assert_eq!((human.yellow, human.red), (0.152, 0.19));
}

/// A potion halves the yellow amount's share (a twentieth): the same
/// damage leaves a human yellow 0.171 (0.17099999 in single precision)
/// and red 0.19.
#[test]
fn a_potion_halves_the_yellow_damage() {
    let mut h = health(0.25, 0.25);
    let potion = StateFlags(StateFlags::POTION);
    assert_eq!(h.take(vital(0.5, 0.3), HUMAN, potion, false), Taken::Hurt);
    assert_eq!((h.yellow, h.red), (0.170_999_99, 0.19));
}

/// Damage that leaves no red empties yellow (and kills when vital); red
/// may go below 0.
#[test]
fn no_red_left_empties_yellow() {
    let (taken, h) = took(health(0.25, 0.25), vital(0.0, 2.0));
    assert_eq!(taken, Taken::Killed);
    assert_eq!((h.yellow, h.red), (0.0, -0.15));
}

/// The lines: yellow under 0.05 (in single precision) weakens; under
/// 0.01, compared in double precision, vital damage kills. A yellow of
/// exactly 0.01 in single precision (0.0099999998) is under 0.01 in double
/// and dies; 0.05 is only hurt, the value below it weakens.
#[test]
fn the_lines_at_0_05_and_0_01() {
    let nothing = vital(0.0, 0.0);
    let (taken, h) = took(health(0.01, 0.25), nothing);
    assert_eq!((taken, h.yellow), (Taken::Killed, 0.0));
    let (taken, h) = took(health(0.010_000_001, 0.25), nothing);
    assert_eq!((taken, h.yellow), (Taken::Weakened, 0.010_000_001));
    let (taken, h) = took(health(0.05, 0.25), nothing);
    assert_eq!((taken, h.yellow), (Taken::Hurt, 0.05));
    let under = f32::from_bits(0.05f32.to_bits() - 1);
    assert_eq!(under, 0.049_999_997);
    let (taken, h) = took(health(under, 0.25), nothing);
    assert_eq!((taken, h.yellow), (Taken::Weakened, under));
    assert!(failing(0.01) && failing(0.0) && !failing(0.010_000_001));
}

/// The subtractions are in double precision: from full health Y 2 leaves
/// yellow at 0.05 in single precision (0.25 − 0.2 rounds up to it), only
/// hurt, where single-precision arithmetic would leave 0.049999997 and
/// weaken.
#[test]
fn the_subtractions_are_in_double_precision() {
    let (taken, h) = took(health(0.25, 0.25), vital(2.0, 0.0));
    assert_eq!((taken, h.yellow), (Taken::Hurt, 0.05));
    assert_eq!(0.25f32 - 2.0 * 0.1f32, 0.049_999_997);
}

/// Non-vital damage never kills: under 0.01 yellow is 0, the fighter lives
/// and weakens, and red keeps what it had even in the story.
#[test]
fn non_vital_damage_never_kills() {
    let mut h = health(0.05, 0.2);
    let bump = Damage {
        yellow: 0.45,
        red: 0.0,
        vital: false,
    };
    assert_eq!(h.take(bump, HUMAN, StateFlags::NONE, true), Taken::Weakened);
    assert_eq!((h.yellow, h.red), (0.0, 0.2));
}

/// In the story a killing blow also takes a tenth of the maximum from red:
/// 0.2 → 0.175 of 0.25; out of it red stays 0.2.
#[test]
fn a_killing_blow_in_the_story_costs_red() {
    for (story, red) in [(true, 0.175), (false, 0.2)] {
        let mut h = health(0.05, 0.2);
        let taken = h.take(vital(0.45, 0.0), HUMAN, StateFlags::NONE, story);
        assert_eq!((taken, h.yellow, h.red), (Taken::Killed, 0.0, red));
    }
}

/// Every fighter keeps a condition word, normal when it spawns, and empty
/// state flags.
#[test]
fn a_fighter_keeps_a_condition() {
    let mut app = App::new();
    let id = app.world_mut().spawn(Fighter::default()).id();
    assert_eq!(app.world().get::<Condition>(id), Some(&Condition::NORMAL));
    assert_eq!(app.world().get::<StateFlags>(id), Some(&StateFlags::NONE));
}

/// The damage call over every living character of the installed story
/// levels (skipped without `EXANIMA_DIR`): the same blow (Y 0.5, R 0.3)
/// leaves red untouched on exactly those with no lasting wounds, 248 of
/// the 451: the 97 feral humans, 128 skeletons and 23 ancients; it takes
/// 0.06 from the red of the 203 others.
#[test]
fn shipped_characters_take_damage_by_their_rule_bits() {
    use exa_formats::character_stats::Spawn;
    use exa_formats::characters::{CHARACTERS, LevelCharacters};
    use exa_formats::races;
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let races = races::parse(resource.get("races.rdb").unwrap()).unwrap();
    let (mut kept, mut cut) = (std::collections::BTreeMap::new(), 0);
    for e in resource.entries() {
        let Ok(chunks) = exa_formats::chunk::rfc_chunks(resource.bytes(e)) else {
            continue;
        };
        for c in chunks.flatten().filter(|c| c.tag == CHARACTERS) {
            let level = LevelCharacters::parse(c.data).unwrap();
            for p in level.placements.iter().filter(|p| !p.is_dead()) {
                let r = level.record(p.id).unwrap();
                let race = races::of_character(&races, r.race);
                let spawn = Spawn {
                    story: true,
                    dead: false,
                };
                let stats = CharacterStats::derive(r, race, spawn);
                let mut h = Fighter::of_stats(race, stats).health;
                let red = h.red;
                h.take(vital(0.5, 0.3), stats.rules, StateFlags::NONE, true);
                let lasting = !stats.rules.has(BodyRules::NO_LASTING_WOUNDS);
                if h.red == red {
                    assert!(!lasting, "{}", r.name);
                    *kept.entry((r.race, r.feral > 0.0)).or_insert(0) += 1;
                } else {
                    assert!(lasting, "{}", r.name);
                    assert!((red - h.red - 0.06).abs() < 1e-6, "{}", r.name);
                    cut += 1;
                }
            }
        }
    }
    // By race byte (0 human, 1 skeleton, 2 ancient) and whether feral.
    let kept: Vec<_> = kept.into_iter().collect();
    assert_eq!(kept, [((0, true), 97), ((1, false), 128), ((2, false), 23)]);
    assert_eq!(cut, 203);
}

#[test]
fn a_wound_scores_three_for_the_striker_and_a_parried_guard_one() {
    let mut striker = CombatScore::default();
    striker.credit_wound();
    striker.credit_wound();
    assert_eq!(striker.0, 6);
    let mut parrier = CombatScore::default();
    parrier.note_guard(ScoreEvent::Clashed, true);
    parrier.note_guard(ScoreEvent::Blocked, true);
    // Not parrying: the notice adds nothing; a wound notice on the struck
    // body adds nothing to it either.
    parrier.note_guard(ScoreEvent::Blocked, false);
    parrier.note_guard(ScoreEvent::Wounded, true);
    assert_eq!(parrier.0, 2);
    assert_eq!(parrier.take(), 2);
    assert_eq!(parrier.0, 0);
}
