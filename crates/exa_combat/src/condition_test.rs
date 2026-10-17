//! The condition word: its values, the original's masks, the posture
//! decision's order and the writers between ticks.
use std::collections::HashMap;
use std::sync::Arc;

use exa_anim::Animator;
use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};

use super::*;
use crate::Fighter;

/// Every value the original writes.
const ALL: [Condition; 15] = [
    Condition::NORMAL,
    Condition::WALKING,
    Condition::ATTACKING,
    Condition::COMBAT_IDLE,
    Condition::CASTING,
    Condition::SHOCKED,
    Condition::WEAK,
    Condition::DOWN,
    Condition::GETTING_UP,
    Condition::DORMANT,
    Condition::COLLAPSING,
    Condition::DEAD,
    Condition::KICKING,
    Condition::SWITCHING,
    Condition::SWITCHED,
];

/// The values each mask holds, of all the original writes.
fn held(test: impl Fn(Condition) -> bool) -> Vec<u32> {
    ALL.iter()
        .copied()
        .filter(|&c| test(c))
        .map(|c| c.0)
        .collect()
}

/// The masks as the original tests them: 0xAC0 holds down, getting up,
/// dormant, collapsing, dead and shocked; 0xA00 the same but shocked;
/// 0x200 down, getting up, dormant and dead (not collapsing); 0x800
/// collapsing and dead; 0x10203 walking, attacking, down, getting up,
/// dormant, dead and both weapon switches.
#[test]
fn the_masks_hold_the_originals_values() {
    assert_eq!(
        held(Condition::helpless),
        [0xC0, 0x200, 0x300, 0x600, 0x800, 0xA00]
    );
    assert_eq!(
        held(Condition::fallen_or_dying),
        [0x200, 0x300, 0x600, 0x800, 0xA00]
    );
    assert_eq!(held(Condition::off_feet), [0x200, 0x300, 0x600, 0xA00]);
    assert_eq!(held(Condition::dying), [0x800, 0xA00]);
    assert_eq!(
        held(Condition::busy),
        [0x1, 0x2, 0x200, 0x300, 0x600, 0xA00, 0x10000, 0x10004]
    );
    assert_eq!(
        held(Condition::blocks_daze),
        [0x2, 0xC0, 0x200, 0x300, 0x600, 0x800, 0xA00]
    );
}

/// A standing body at full tone playing `motion`.
fn standing(motion: usize) -> Posture {
    Posture {
        motion: Some(motion),
        progress: 0.0,
        balance: 1.0,
        exertion: 0.0,
        tone: 1.0,
        parrying: false,
        shock_over: false,
    }
}

/// Standing at full tone the clip decides: the walk and the run walking,
/// an attack (0x05–0x0C) attacking, the combat idle combat idle, the kick
/// kicking, a power (0x22–0x28) casting, a weapon switch switching until
/// 60 % of it and switched after, anything else normal (the backpedal,
/// Rebalance, Idle, the Defence stance, Cower).
#[test]
fn standing_the_clip_decides() {
    let cases = [
        (0x00, 0x1),
        (0x01, 0x1),
        (0x02, 0x0),
        (0x03, 0x0),
        (0x04, 0x8),
        (0x05, 0x2),
        (0x0C, 0x2),
        (0x0D, 0x8000),
        (0x13, 0x0),
        (0x21, 0x0),
        (0x22, 0x20),
        (0x28, 0x20),
        (0x29, 0x0),
    ];
    for (motion, want) in cases {
        let (c, branch) = posture(Condition::NORMAL, &standing(motion));
        assert_eq!(c.0, want, "motion {motion:#x}");
        let attack = (0x05..=0x0C).contains(&motion);
        assert_eq!(
            branch,
            if attack {
                Branch::Attack
            } else {
                Branch::Standing
            }
        );
    }
    let switch = |progress| {
        posture(
            Condition::NORMAL,
            &Posture {
                progress,
                ..standing(0x1F)
            },
        )
        .0
    };
    // Compared in double precision: the float 0.6 is past the swap.
    assert_eq!(switch(0.599), Condition::SWITCHING);
    assert_eq!(switch(0.6), Condition::SWITCHED);
    // An action the motion set lacks is normal.
    let none = Posture {
        motion: None,
        ..standing(0)
    };
    assert_eq!(posture(Condition::WALKING, &none).0, Condition::NORMAL);
    // Parrying is the combat idle's whatever plays, but the kick wins.
    let parry = |motion| {
        posture(
            Condition::NORMAL,
            &Posture {
                parrying: true,
                ..standing(motion)
            },
        )
        .0
    };
    assert_eq!(parry(0x14), Condition::COMBAT_IDLE);
    assert_eq!(parry(0x0D), Condition::KICKING);
}

/// Under a tone of 0.85 a standing body is weak, whatever it plays, but an
/// attack is tested first: attacking at any tone.
#[test]
fn weak_under_a_tone_of_0_85() {
    let at = |motion, tone| {
        posture(
            Condition::NORMAL,
            &Posture {
                tone,
                ..standing(motion)
            },
        )
    };
    assert_eq!(at(0x04, 0.84), (Condition::WEAK, Branch::Weak));
    assert_eq!(at(0x00, 0.5), (Condition::WEAK, Branch::Weak));
    assert_eq!(at(0x04, 0.85).0, Condition::COMBAT_IDLE);
    assert_eq!(at(0x05, 0.1), (Condition::ATTACKING, Branch::Attack));
}

/// Off balance (under 0.6, compared in double precision: the float 0.6 is
/// a hair over it and stands) the clip no longer counts: the body keeps
/// its condition while it falls above 0.3 and is down under it (the float
/// 0.3 is not under), lying still (summed limb motion under 25 cm) or
/// still moving.
#[test]
fn off_balance_the_body_falls_then_lies_down() {
    let at = |balance: f32, exertion: f32| Posture {
        balance,
        exertion,
        ..standing(0x04)
    };
    assert_eq!(
        posture(Condition::COMBAT_IDLE, &at(0.6, 0.0)),
        (Condition::COMBAT_IDLE, Branch::Standing)
    );
    assert_eq!(
        posture(Condition::COMBAT_IDLE, &at(0.599, 0.0)),
        (Condition::COMBAT_IDLE, Branch::Falling)
    );
    assert_eq!(
        posture(Condition::WALKING, &at(0.3, 0.0)),
        (Condition::WALKING, Branch::Falling)
    );
    assert_eq!(
        posture(Condition::NORMAL, &at(0.299, 24.9)),
        (Condition::DOWN, Branch::LyingStill)
    );
    assert_eq!(
        posture(Condition::NORMAL, &at(0.299, 25.0)),
        (Condition::DOWN, Branch::Falling)
    );
    assert_eq!(
        posture(Condition::DOWN, &at(0.0, 0.0)),
        (Condition::DOWN, Branch::LyingStill)
    );
    // An attack playing off balance is not an attack's tick.
    let attack = Posture {
        balance: 0.5,
        ..standing(0x05)
    };
    assert_eq!(
        posture(Condition::ATTACKING, &attack),
        (Condition::ATTACKING, Branch::Falling)
    );
}

/// Getting up stays getting up at any balance until its clip ends; then
/// the next tick decides afresh.
#[test]
fn getting_up_lasts_until_the_clip_ends() {
    for balance in [0.0, 0.29, 0.5, 1.0] {
        let p = Posture {
            balance,
            ..standing(0x0F)
        };
        assert_eq!(
            posture(Condition::GETTING_UP, &p),
            (Condition::GETTING_UP, Branch::GettingUp)
        );
    }
    let mut c = Condition::GETTING_UP;
    c.clip_ended();
    assert_eq!(c, Condition::NORMAL);
    assert_eq!(posture(c, &standing(0x04)).0, Condition::COMBAT_IDLE);
    // The end of any other clip leaves the condition alone.
    let mut c = Condition::ATTACKING;
    c.clip_ended();
    assert_eq!(c, Condition::ATTACKING);
}

/// Collapsing and dead bodies skip the decision (the dying tick, or
/// nothing, runs instead); a shocked golem waits out its timer.
#[test]
fn collapsing_dead_and_shocked_skip_the_decision() {
    let off = Posture {
        balance: 0.0,
        ..standing(0x05)
    };
    assert_eq!(
        posture(Condition::COLLAPSING, &off),
        (Condition::COLLAPSING, Branch::Collapse)
    );
    assert_eq!(
        posture(Condition::DEAD, &standing(0x04)),
        (Condition::DEAD, Branch::Dead)
    );
    assert_eq!(
        posture(Condition::SHOCKED, &standing(0x04)),
        (Condition::SHOCKED, Branch::Shocked)
    );
    let over = Posture {
        shock_over: true,
        ..standing(0x04)
    };
    assert_eq!(
        posture(Condition::SHOCKED, &over),
        (Condition::NORMAL, Branch::Shocked)
    );
}

/// The daze makes a fighter normal unless it is helpless or attacking; the
/// get-up, death and the writers' values.
#[test]
fn the_writers() {
    for c in ALL {
        let mut dazed = c;
        dazed.daze();
        let kept = c.blocks_daze();
        assert_eq!(dazed, if kept { c } else { Condition::NORMAL }, "{c:?}");
    }
    let mut c = Condition::DOWN;
    c.get_up();
    assert_eq!(c.0, 0x300);
    c.die();
    assert_eq!(c.0, 0xA00);
}

/// A motion set of the actions the tests play: Walk, Cmbt - Idle and
/// Attack Mid, one key each.
fn motions() -> Motions {
    let names = [
        "Walk",
        "Run",
        "Backpedal",
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

/// The posture stage on a living fighter, a fixed tick at a time: it walks
/// (1), stands in the combat idle (8), attacks (2), and dazed mid-attack
/// it keeps attacking (the daze leaves an attacker alone); in the Defence
/// stance it is normal; a dead fighter, or one without muscles, is left
/// alone.
#[test]
fn the_posture_stage_writes_once_a_tick() {
    let mut app = App::new();
    app.insert_resource(motions())
        .add_systems(Update, track_condition);
    let body = Ragdoll::new(&[]).with_drive();
    let id = app
        .world_mut()
        .spawn((Fighter::default(), Animator::new("Default", "Walk"), body))
        .id();
    let condition = |app: &App| *app.world().get::<Condition>(id).unwrap();
    let play = |app: &mut App, action: &str| {
        app.world_mut().get_mut::<Animator>(id).unwrap().action = action.into();
        app.update();
    };
    app.update();
    assert_eq!(condition(&app), Condition::WALKING);
    play(&mut app, "Cmbt - Idle");
    assert_eq!(condition(&app), Condition::COMBAT_IDLE);
    play(&mut app, "Attack Mid");
    assert_eq!(condition(&app), Condition::ATTACKING);
    // Dazed while attacking: the daze leaves it alone, the swing goes on.
    app.world_mut()
        .get_mut::<Ragdoll>(id)
        .unwrap()
        .drive
        .as_mut()
        .unwrap()
        .energy = 0.5;
    app.update();
    assert_eq!(condition(&app), Condition::ATTACKING);
    play(&mut app, "Defence stance");
    assert_eq!(condition(&app), Condition::NORMAL);
    // A dead fighter keeps its word; one without muscles is left alone.
    *app.world_mut().get_mut::<Condition>(id).unwrap() = Condition::DEAD;
    play(&mut app, "Walk");
    assert_eq!(condition(&app), Condition::DEAD);
    app.world_mut().get_mut::<Ragdoll>(id).unwrap().go_limp();
    *app.world_mut().get_mut::<Condition>(id).unwrap() = Condition::COMBAT_IDLE;
    app.update();
    assert_eq!(condition(&app), Condition::COMBAT_IDLE);
}
