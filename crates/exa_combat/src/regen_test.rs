//! Health regeneration: the ramped rate, its gates, red regeneration, the
//! heal pool and the clamps, pinned to the original's numbers.
use std::collections::HashMap;
use std::sync::Arc;

use exa_formats::character_stats::CharacterStats;
use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};

use super::*;
use crate::condition::StateFlags;
use crate::damage::Outcome;
use crate::fighter::{Damage, Taken};

fn health(yellow: f32, red: f32, max: f32) -> Health {
    Health {
        yellow,
        red,
        max,
        ..Health::default()
    }
}

/// `h` after `n` ticks, never held.
fn healed(mut h: Health, n: usize) -> Health {
    for _ in 0..n {
        h.regenerate(false);
    }
    h
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-7
}

/// The cap is 0.01/83 (1.2048193e-4) a tick; from 0 the rate rises by
/// 0.0003 of it a tick and reaches it on the 3,334th tick, 40.2 s.
#[test]
fn the_rate_reaches_its_cap_in_40_seconds() {
    assert_eq!(RATE_CAP, 1.204_819_3e-4);
    let h = health(0.0, 0.0, 0.25);
    assert_eq!(healed(h, 1).rate, 3.614_457_7e-8);
    assert_eq!(healed(h, 2).rate, 7.228_915_4e-8);
    assert!(healed(h, 3333).rate < RATE_CAP);
    assert_eq!(healed(h, 3334).rate, RATE_CAP);
    assert_eq!(healed(h, 5000).rate, RATE_CAP);
}

/// The retail trace (`master-body.md` §4.3): the player out of the combat
/// idle at yellow 0.085 with red 0.203 regained 0.086, 0.090, 0.100, 0.110
/// and 0.121 at 5.0, 9.6, 16.2, 21.1 and 25.3 s, 0.202 at 45.8 s; the rule
/// gives 0.08641, 0.09018, 0.09974, 0.10998, 0.12093 and 0.20089 (the
/// trace's start is known to about a tenth of a second).
#[test]
fn the_retail_trace_fits() {
    let start = health(0.085, 0.203, 0.25);
    let fit = [
        (5.0, 0.086, 0.086_405_73),
        (9.6, 0.090, 0.090_178_72),
        (16.2, 0.100, 0.099_741_02),
        (21.1, 0.110, 0.109_979_205),
        (25.3, 0.121, 0.120_925_6),
        (45.8, 0.202, 0.200_887_96),
    ];
    for (seconds, retail, rule) in fit {
        let ticks = (seconds * 83.0_f64).round() as usize;
        let y = healed(start, ticks).yellow;
        assert!(close(y, rule), "{seconds} s ({ticks} ticks): {y}");
        assert!((y - retail).abs() < 0.0012, "{seconds} s: {y} vs {retail}");
    }
    // Then it stops at red.
    assert_eq!(healed(start, 6000).yellow, 0.203);
}

/// At the cap a human with full red heals 0.005 a second: the whole bar in
/// 50 s.
#[test]
fn a_human_heals_0_005_a_second_at_the_cap() {
    let h = Health {
        rate: RATE_CAP,
        ..health(0.0, 0.25, 0.25)
    };
    assert!(close(healed(h, 83).yellow, 0.004_999_996));
    let mut slow = health(0.1, 0.25, 0.25);
    slow = healed(slow, 40 * 83);
    assert!(close(slow.yellow, 0.199_630_68), "{}", slow.yellow);
    assert_eq!(healed(slow, 20 * 83).yellow, 0.25);
}

/// Held (attacking or in the combat idle), the rate is 0 and yellow does
/// not rise; the clamps still run.
#[test]
fn held_it_does_not_heal() {
    let mut h = Health {
        rate: RATE_CAP,
        ..health(0.1, 0.2, 0.25)
    };
    h.regenerate(true);
    assert_eq!((h.rate, h.yellow), (0.0, 0.1));
    let mut over = health(0.3, 0.27, 0.25);
    over.regenerate(true);
    assert_eq!((over.yellow, over.red), (0.27, 0.25));
    // Released, the ramp starts from 0.
    h.regenerate(false);
    assert_eq!(h.rate, 3.614_457_7e-8);
}

/// The gate: attacking (2), or the combat idle playing whatever the
/// condition (weak 0x100 included); the Defence stance (0) and walking
/// heal.
#[test]
fn attacking_or_the_combat_idle_hold_it() {
    let idle = Some(action::COMBAT_IDLE);
    assert!(holds_regeneration(Condition::ATTACKING, Some(5)));
    assert!(holds_regeneration(Condition::COMBAT_IDLE, idle));
    assert!(holds_regeneration(Condition::WEAK, idle));
    assert!(!holds_regeneration(Condition::NORMAL, Some(3)));
    assert!(!holds_regeneration(Condition::WALKING, Some(0)));
    assert!(!holds_regeneration(Condition::COLLAPSING, None));
}

/// Every blow sets the rate back to 0, whatever it deals.
#[test]
fn a_blow_restarts_the_ramp() {
    let mut h = Health {
        rate: RATE_CAP,
        ..health(0.2, 0.25, 0.25)
    };
    let nothing = Damage::blow(&Outcome::default());
    let taken = h.take(nothing, BodyRules::default(), StateFlags::NONE, false);
    assert_eq!((taken, h.rate), (Taken::Hurt, 0.0));
}

/// Yellow stays within red and red within the maximum, in that order (so
/// yellow follows a red brought down to the maximum a tick later); full
/// health stays full; yellow never goes under 0; nothing heals at a red of
/// 1.19209e-7.
#[test]
fn the_clamps() {
    let full = Health {
        rate: RATE_CAP,
        ..Health::of_race(1.0)
    };
    assert_eq!(healed(full, 10), full);
    let over = healed(health(0.3, 0.3, 0.25), 1);
    assert_eq!((over.yellow, over.red), (0.3, 0.25));
    let over = healed(over, 1);
    assert_eq!((over.yellow, over.red), (0.25, 0.25));
    let under = healed(health(-0.01, 0.2, 0.25), 1);
    assert_eq!(under.yellow, 0.0);
    // The floor is the image's float just under FLT_EPSILON; a red just
    // above it heals.
    assert_eq!(RED_FLOOR.to_bits(), 0x33ff_ffd7);
    let none = healed(health(0.0, RED_FLOOR, 0.25), 10);
    assert_eq!(none.yellow, 0.0);
    let some = healed(health(0.0, 1.2e-7, 0.25), 10);
    assert!(some.yellow > 0.0, "{some:?}");
}

/// Red regenerates 1.25e-5 a tick (0.00104 a second) up to the maximum for
/// a body whose rules hold 0x20 without 0x100: an ogre's red from 0.1 is
/// 0.10104 after 83 ticks and 0.1125 after 1,000; a golem suit without its
/// core and a human regain none.
#[test]
fn red_regenerates_by_the_rule_bits() {
    let ticks = |rules: u32, red: f32, n: usize| {
        let mut h = health(0.0, red, 1.5);
        for _ in 0..n {
            h.regenerate_red(BodyRules(rules));
        }
        h.red
    };
    assert!(close(ticks(0x67, 0.1, 83), 0.101_037_67));
    assert!(close(ticks(0x67, 0.1, 1000), 0.112_502_076));
    assert_eq!(ticks(0x70, 1.4999, 83), 1.5);
    assert_eq!(ticks(0x170, 0.1, 83), 0.1);
    assert_eq!(ticks(0x12, 0.1, 83), 0.1);
}

/// The heal pool moves 1.25e-5 a tick to red: a bandage's 0.2 drains in
/// 15,999 ticks (193 s); red from 0.1 is full (0.25) after 11,999 of them
/// while the pool drains on.
#[test]
fn the_heal_pool_drains_into_red() {
    let mut h = Health {
        heal_pool: 0.2,
        ..health(0.0, 0.1, 0.25)
    };
    h.regenerate(true);
    assert_eq!((h.red, h.heal_pool), (0.100_012_5, 0.199_987_5));
    let mut n = 1;
    let mut full = None;
    while h.heal_pool > 0.0 {
        h.regenerate(true);
        n += 1;
        if full.is_none() && h.red == 0.25 {
            full = Some(n);
        }
    }
    assert_eq!((n, full, h.red), (15_999, Some(11_999), 0.25));
}

/// A motion set of the clips the system test plays, Cmbt - Idle at the
/// original's action index 4.
fn motions() -> Motions {
    let names = [
        "Walk",
        "Run",
        "Idle",
        "Defence stance",
        "Cmbt - Idle",
        "Attack Mid",
    ];
    let clip = || Clip {
        mode: 1,
        keys: vec![Key {
            time: 10.0,
            root: [0.0; 3],
            rotations: vec![],
            flags: vec![],
        }],
    };
    let set = MotionSet {
        bone_count: 0,
        actions: names
            .iter()
            .map(|n| Action {
                name: (*n).into(),
                params: [0, 0],
            })
            .collect(),
        movesets: vec![Moveset {
            name: "Default".into(),
            parent: String::new(),
            clips: names.iter().map(|_| Some(clip())).collect(),
        }],
    };
    Motions::new(Arc::new(set), Arc::new(HashMap::new()), 0.0)
}

/// The health tick on a wounded fighter: it heals in the Defence stance,
/// not in Cmbt - Idle nor while attacking (the rate back at 0), and not
/// once dead; a dead body with red regeneration still regains red.
#[test]
fn the_health_tick() {
    let mut app = App::new();
    app.insert_resource(motions())
        .add_systems(Update, regenerate_health);
    let stats = CharacterStats {
        rules: BodyRules(0x67),
        ..CharacterStats::default()
    };
    let fighter = Fighter {
        health: health(0.1, 0.2, 0.25),
        ..Fighter::of_stats(None, stats)
    };
    let id = app
        .world_mut()
        .spawn((fighter, Animator::new("Default", "Defence stance")))
        .id();
    let get = |app: &App| app.world().get::<Fighter>(id).unwrap().health;
    let ticks = |app: &mut App, n: usize| {
        for _ in 0..n {
            app.update();
        }
    };
    ticks(&mut app, 100);
    let h = get(&app);
    assert!(h.yellow > 0.1 && h.rate > 0.0, "{h:?}");
    assert!(close(h.red, 0.201_250_2), "{h:?}");
    app.world_mut().get_mut::<Animator>(id).unwrap().action = "Cmbt - Idle".into();
    ticks(&mut app, 10);
    let idle = get(&app);
    assert_eq!((idle.rate, idle.yellow), (0.0, h.yellow));
    app.world_mut().get_mut::<Animator>(id).unwrap().action = "Defence stance".into();
    *app.world_mut().get_mut::<Condition>(id).unwrap() = Condition::ATTACKING;
    ticks(&mut app, 10);
    assert_eq!(get(&app).yellow, h.yellow);
    *app.world_mut().get_mut::<Condition>(id).unwrap() = Condition::DEAD;
    let red = get(&app).red;
    ticks(&mut app, 10);
    let dead = get(&app);
    assert_eq!(dead.yellow, h.yellow);
    assert!(dead.red > red, "{dead:?}");
}
