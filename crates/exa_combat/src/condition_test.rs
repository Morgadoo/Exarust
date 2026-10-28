//! The condition word: its values, the original's masks, the posture
//! decision's order and the writers between ticks.
use std::collections::HashMap;
use std::sync::Arc;

use bevy::math::Affine3A;
use exa_anim::Animator;
use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
use exa_physics::body::PartSetup;
use exa_physics::body::pairs::{FOOT_LAYER, LIVING_SHIN_LAYER};

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
    // The swap fires once, on the tick that leaves the switching word.
    assert!(swap_due(Condition::SWITCHING, Condition::SWITCHED));
    assert!(!swap_due(Condition::SWITCHED, Condition::SWITCHED));
    assert!(!swap_due(Condition::NORMAL, Condition::SWITCHED));
    assert!(!swap_due(Condition::SWITCHING, Condition::SWITCHING));
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

/// The posture stage on a living fighter, a tick of its character at a
/// time: it walks
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
    app.insert_resource(TickedCharacter {
        body: id,
        character: None,
    });
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

/// A living body of a pelvis, a lower leg with its foot contact and the
/// foot under it. Registration puts the shin in the living layer and the
/// foot in the feet's.
fn legs() -> Ragdoll {
    let part = |bone, parent, y: f32, half: Vec3, foot| PartSetup {
        bone,
        entity: Entity::from_raw_u32(bone as u32 + 1).unwrap(),
        parent,
        world: Affine3A::from_translation(Vec3::new(0.0, y, 0.0)),
        centre: Vec3::ZERO,
        half,
        origin: Vec3::ZERO,
        links: vec![],
        foot,
        pins: Vec::new(),
        spheres: None,
    };
    Ragdoll::new(&[
        part(1, None, 1.0, Vec3::splat(0.1), None),
        part(
            3,
            Some(0),
            0.5,
            Vec3::new(0.06, 0.2, 0.06),
            Some(Vec3::new(0.0, -0.3, 0.0)),
        ),
        part(4, Some(1), 0.05, Vec3::new(0.06, 0.05, 0.12), None),
    ])
    .with_drive()
}

/// Weak, a kick, a walk and normal restore the living lower-leg layer when
/// they replace a condition above 0xFF. An attack that replaces one puts
/// the shins in 0x10. A condition already at most 0xFF, combat idle, a
/// power, a weapon switch, down, getting up and a shock write nothing, and
/// an off-balance body that keeps its word does not restore.
#[test]
fn a_high_condition_restores_the_lower_leg_layer() {
    let living = Some(LIVING_SHIN_LAYER);
    assert_eq!(
        leg_layer_for(Condition::DOWN, Condition::WEAK, Branch::Weak),
        living
    );
    assert_eq!(
        leg_layer_for(Condition::WEAK, Condition::WEAK, Branch::Weak),
        living
    );
    assert_eq!(
        leg_layer_for(Condition::KICKING, Condition::KICKING, Branch::Standing),
        living
    );
    assert_eq!(
        leg_layer_for(Condition::DOWN, Condition::WALKING, Branch::Standing),
        living
    );
    assert_eq!(
        leg_layer_for(Condition::GETTING_UP, Condition::NORMAL, Branch::Standing),
        living
    );
    assert_eq!(
        leg_layer_for(Condition::NORMAL, Condition::NORMAL, Branch::Standing),
        None
    );
    assert_eq!(
        leg_layer_for(Condition::WALKING, Condition::WALKING, Branch::Standing),
        None
    );
    assert_eq!(
        leg_layer_for(Condition::DOWN, Condition::COMBAT_IDLE, Branch::Standing),
        None
    );
    assert_eq!(
        leg_layer_for(Condition::DOWN, Condition::CASTING, Branch::Standing),
        None
    );
    assert_eq!(
        leg_layer_for(Condition::DOWN, Condition::SWITCHING, Branch::Standing),
        None
    );
    assert_eq!(
        leg_layer_for(Condition::DOWN, Condition::WEAK, Branch::Falling),
        None
    );
    assert_eq!(
        leg_layer_for(Condition::KICKING, Condition::KICKING, Branch::Falling),
        None
    );
    assert_eq!(
        leg_layer_for(Condition::DOWN, Condition::DOWN, Branch::Falling),
        None
    );
    assert_eq!(
        leg_layer_for(
            Condition::GETTING_UP,
            Condition::GETTING_UP,
            Branch::GettingUp
        ),
        None
    );
    assert_eq!(
        leg_layer_for(Condition::DOWN, Condition::ATTACKING, Branch::Attack),
        Some(ATTACK_SHIN_LAYER)
    );
    assert_eq!(
        leg_layer_for(Condition::NORMAL, Condition::ATTACKING, Branch::Attack),
        None
    );
    assert_eq!(
        leg_layer_for(Condition::SHOCKED, Condition::NORMAL, Branch::Shocked),
        None
    );
}

/// The posture stage on a body with legs: down into a stance puts the
/// living layer back (feet in it plus the feet's); a walk that was already
/// walking leaves a cleared shin; down into an attack puts the shins in
/// 0x10 and the feet stay in the feet's; off balance the attack is not
/// taken and the layer stays; weak from down restores, but a daze that
/// cleared weak first does not.
#[test]
fn the_posture_stage_restores_the_lower_legs() {
    let mut app = App::new();
    app.insert_resource(motions())
        .add_systems(Update, track_condition);
    let id = app
        .world_mut()
        .spawn((
            Fighter::default(),
            Animator::new("Default", "Defence stance"),
            legs(),
        ))
        .id();
    app.insert_resource(TickedCharacter {
        body: id,
        character: None,
    });
    let shin = |app: &App| {
        app.world().get::<Ragdoll>(id).unwrap().parts[1]
            .pairing
            .layer
    };
    let foot = |app: &App| {
        app.world().get::<Ragdoll>(id).unwrap().parts[2]
            .pairing
            .layer
    };
    let condition = |app: &App| *app.world().get::<Condition>(id).unwrap();
    let prepare = |app: &mut App, word: Condition, action: &str, layer: u32, foot_layer: u32| {
        let world = app.world_mut();
        *world.get_mut::<Condition>(id).unwrap() = word;
        world.get_mut::<Animator>(id).unwrap().action = action.into();
        let mut rag = world.get_mut::<Ragdoll>(id).unwrap();
        rag.parts[1].pairing.layer = layer;
        rag.parts[2].pairing.layer = foot_layer;
    };

    prepare(&mut app, Condition::DOWN, "Defence stance", 0, 0);
    app.update();
    assert_eq!(condition(&app), Condition::NORMAL);
    assert_eq!(shin(&app), LIVING_SHIN_LAYER);
    assert_eq!(foot(&app), LIVING_SHIN_LAYER | FOOT_LAYER);

    prepare(&mut app, Condition::WALKING, "Walk", 0, FOOT_LAYER);
    app.update();
    assert_eq!(condition(&app), Condition::WALKING);
    assert_eq!(shin(&app), 0);

    prepare(&mut app, Condition::DOWN, "Attack Mid", 0, FOOT_LAYER);
    app.update();
    assert_eq!(condition(&app), Condition::ATTACKING);
    assert_eq!(shin(&app), ATTACK_SHIN_LAYER);
    assert_eq!(foot(&app), FOOT_LAYER);

    {
        let world = app.world_mut();
        *world.get_mut::<Condition>(id).unwrap() = Condition::DOWN;
        world.get_mut::<Ragdoll>(id).unwrap().parts[1].pairing.layer = 0;
        world
            .get_mut::<Ragdoll>(id)
            .unwrap()
            .drive
            .as_mut()
            .unwrap()
            .balance_mut()
            .value = 0.4;
    }
    app.update();
    assert_eq!(condition(&app), Condition::DOWN);
    assert_eq!(shin(&app), 0);

    {
        let mut rag = app.world_mut().get_mut::<Ragdoll>(id).unwrap();
        let drive = rag.drive.as_mut().unwrap();
        drive.balance_mut().value = 1.0;
        drive.tone = 0.5;
    }
    prepare(&mut app, Condition::DOWN, "Defence stance", 0, 0);
    app.update();
    assert_eq!(condition(&app), Condition::WEAK);
    assert_eq!(shin(&app), LIVING_SHIN_LAYER);
    assert_eq!(foot(&app), LIVING_SHIN_LAYER | FOOT_LAYER);

    app.world_mut()
        .get_mut::<Ragdoll>(id)
        .unwrap()
        .drive
        .as_mut()
        .unwrap()
        .energy = 0.5;
    prepare(&mut app, Condition::WEAK, "Defence stance", 0, FOOT_LAYER);
    app.update();
    assert_eq!(condition(&app), Condition::WEAK);
    assert_eq!(shin(&app), 0);
}
