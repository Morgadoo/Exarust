//! The collapse before death: its start, the dying tick's rules and the
//! death 250 ticks later.
use bevy::math::Affine3A;
use exa_physics::body::pairs::{FOOT_LAYER, LIVING_SHIN_LAYER};
use exa_physics::body::{FootState, PIN_GAIN, PartSetup};

use super::*;

/// A living body of a pelvis, a lower leg with its foot contact and the
/// foot under it.
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

/// Fall Backward's blend: round((1.2 − E) × 300) in double precision, ties
/// to even: 60 at full energy, 90 at 0.9, 174 at 0.62, 345 at the daze's
/// floor of 0.05.
#[test]
fn fall_backward_blends_by_the_energy_left() {
    for (energy, blend) in [(1.0, 60), (0.9, 90), (0.62, 174), (0.5, 210), (0.05, 345)] {
        assert_eq!(collapse_blend(energy), blend, "energy {energy}");
    }
}

/// The tone falls 0.003 a tick (in double, stored as a float): from 1 the
/// body dies on the 250th tick (3.01 s at 83 Hz), at 0.24999 its tone set
/// to 0.01; on the 249th it is still at 0.2530.
#[test]
fn death_comes_at_a_tone_of_a_quarter() {
    let mut tone = 1.0;
    let mut ticks = 0;
    loop {
        let (next, dies) = lose_tone(tone);
        ticks += 1;
        if dies {
            assert_eq!(next, 0.01);
            break;
        }
        tone = next;
    }
    assert_eq!(ticks, 250);
    assert!((tone - 0.252_997_25).abs() < 1e-6, "{tone}");
}

/// The pins' gain falls 0.0003 a tick: a living body's 0.081 (the float
/// 0.08 under the double 0.08 gains 0.001 once) is 0.0063 after 249
/// ticks; under 0 it is set to 0.01.
#[test]
fn the_pins_fade() {
    let living = exa_physics::body::recover_pins(PIN_GAIN, false);
    assert_eq!(living, 0.081);
    assert_eq!(exa_physics::body::recover_pins(living, false), living);
    // Collapsing it does not recover.
    assert_eq!(exa_physics::body::recover_pins(0.05, true), 0.05);
    let mut gain = living;
    for _ in 0..249 {
        gain = fade_pins(gain);
    }
    assert!((gain - 0.006_300_02).abs() < 1e-7, "{gain}");
    assert_eq!(fade_pins(0.0002), 0.01);
    assert!((fade_pins(0.0003)).abs() < 1e-9);
}

/// Rebalance needs a size under 1.2 and a balance under 0.8, both compared
/// in double precision, so the floats 1.2 and 0.8 (each a hair over) do
/// not.
#[test]
fn small_bodies_rebalance_under_0_8() {
    assert!(rebalances(1.0, 0.79));
    assert!(!rebalances(1.0, 0.8));
    assert!(rebalances(1.19, 0.0));
    assert!(!rebalances(1.2, 0.0));
    assert!(!rebalances(1.5, 0.0));
}

/// Under balance 0.8 a collapsing body of size 1 starts Rebalance, looping,
/// with 20 frames of blend, linear; once it plays a further dying tick
/// leaves its clock; a steady body, or one of size 1.2, keeps Fall
/// Backward.
#[test]
fn rebalance_starts_once_with_20_frames() {
    let mut anim = Animator::new("Default", FALL_BACKWARD);
    anim.ease = Ease::Smooth;
    assert!(!throw_arms_wide(&mut anim, 1.0, 0.8));
    assert!(!throw_arms_wide(&mut anim, 1.2, 0.0));
    assert_eq!(anim.action, FALL_BACKWARD);
    assert!(throw_arms_wide(&mut anim, 1.0, 0.5));
    assert_eq!(
        (anim.action.as_str(), anim.looping, anim.blend, anim.ease),
        (REBALANCE, true, 20.0, Ease::Linear)
    );
    anim.time = 6.0;
    assert!(!throw_arms_wide(&mut anim, 1.0, 0.0));
    assert_eq!((anim.time, anim.blend), (6.0, 20.0));
}

/// The heading drifts by (draw − 0.5) × 0.5 on each axis and stays a unit
/// vector: middle draws leave it, a full x draw turns (0, 1) to (0.2425,
/// 0.9701), two empty draws to (−0.3162, 0.9487).
#[test]
fn the_heading_wanders() {
    let ahead = Vec2::Y;
    assert_eq!(jitter_heading(ahead, 0.5, 0.5), ahead);
    let right = jitter_heading(ahead, 1.0, 0.5);
    assert!((right - Vec2::new(0.242_535_6, 0.970_142_5)).length() < 1e-6);
    let back = jitter_heading(ahead, 0.0, 0.0);
    assert!((back - Vec2::new(-0.316_227_8, 0.948_683_3)).length() < 1e-6);
    let mut h = ahead;
    let mut roll = Roll::default();
    for _ in 0..250 {
        h = jitter_heading(h, roll.next(), roll.next());
        assert!((h.length() - 1.0).abs() < 1e-5);
    }
}

/// The start: collapsing, no longer waking, Fall Backward looping with 60
/// frames of blend at full energy, smooth, the body no longer on its
/// controller and its lower legs out of the living lower legs' layer (the
/// feet keep theirs); a fighter collapsing or dead already is left alone;
/// one playing Fall Backward keeps its clock and holds the pose.
#[test]
fn a_lethal_wound_starts_the_collapse() {
    let mut condition = Condition::COMBAT_IDLE;
    let mut flags = StateFlags(StateFlags::WAKING | StateFlags::DORMANT);
    let mut anim = Animator::new("Default", "Cmbt - Idle");
    let mut body = legs();
    assert_eq!(body.parts[1].pairing.layer, LIVING_SHIN_LAYER);
    assert!(start_collapse(
        &mut condition,
        &mut flags,
        &mut anim,
        Some(&mut body),
        1.0
    ));
    assert_eq!(condition, Condition::COLLAPSING);
    assert_eq!(flags.0, StateFlags::DORMANT);
    assert_eq!(
        (anim.action.as_str(), anim.looping, anim.blend, anim.ease),
        (FALL_BACKWARD, true, 60.0, Ease::Smooth)
    );
    assert!(!body.drive.as_ref().unwrap().supported);
    assert_eq!(body.parts[1].pairing.layer, 0);
    assert_eq!(body.parts[2].pairing.layer, FOOT_LAYER);
    // A second lethal wound changes nothing.
    anim.time = 12.0;
    assert!(!start_collapse(
        &mut condition,
        &mut flags,
        &mut anim,
        None,
        0.05
    ));
    assert_eq!((anim.time, anim.blend), (12.0, 60.0));
    let mut dead = Condition::DEAD;
    assert!(!start_collapse(&mut dead, &mut flags, &mut anim, None, 1.0));
    assert_eq!(dead, Condition::DEAD);
    // Killed while falling backwards: the clip goes on, held.
    let mut falling = Animator::new("Default", "Idle");
    falling.play_once(FALL_BACKWARD, "Unconcious 1");
    falling.time = 7.0;
    let mut c = Condition::NORMAL;
    assert!(start_collapse(&mut c, &mut flags, &mut falling, None, 0.5));
    assert_eq!(
        (falling.time, falling.looping, falling.then.as_deref()),
        (7.0, true, None)
    );
    assert_eq!(falling.ease, Ease::Smooth);
}

/// The dying tick on a collapsing fighter, a tick of its character at a
/// time (upkeep, posture, then the attack stage's health while it lives):
/// its tone falls 0.003 and its pins' gain 0.0003 (from 0.08), its heading
/// drifts, a steady body does not Rebalance, its yellow health regenerates
/// from 0; on the 250th tick it is dead, its yellow 0 again (the rest of
/// that tick does not run), its muscles gone, asked to go limp keeping its
/// motion, its collapse over.
#[test]
fn the_body_dies_250_ticks_after_the_blow() {
    use crate::condition::ticked_lives;
    use crate::regen::{keep_health, regenerate_red};
    let mut app = App::new();
    app.add_systems(
        Update,
        (regenerate_red, dying_tick, keep_health.run_if(ticked_lives)).chain(),
    );
    let killed = Fighter {
        health: crate::Health {
            yellow: 0.0,
            ..crate::Health::of_race(1.0)
        },
        ..Fighter::default()
    };
    let id = app
        .world_mut()
        .spawn((
            killed,
            Condition::COLLAPSING,
            Collapse::default(),
            Ragdoll::new(&[]).with_drive(),
            Animator::new("Default", FALL_BACKWARD),
            GlobalTransform::default(),
        ))
        .id();
    app.insert_resource(TickedCharacter {
        body: id,
        character: None,
    });
    for _ in 0..249 {
        app.update();
    }
    let body = app.world().get::<Ragdoll>(id).unwrap();
    let drive = body.drive.as_ref().unwrap();
    assert!((drive.tone - 0.252_997_25).abs() < 1e-6, "{}", drive.tone);
    assert!((drive.pin_gain - 0.005_300_01).abs() < 1e-7);
    let collapse = *app.world().get::<Collapse>(id).unwrap();
    assert_eq!(collapse.ticks, 249);
    let heading = collapse.heading.unwrap();
    assert!((heading.length() - 1.0).abs() < 1e-5 && heading != Vec2::Y);
    assert_eq!(
        app.world().get::<Animator>(id).unwrap().action,
        FALL_BACKWARD
    );
    assert_eq!(
        *app.world().get::<Condition>(id).unwrap(),
        Condition::COLLAPSING
    );
    let yellow = |app: &App| app.world().get::<Fighter>(id).unwrap().health.yellow;
    assert!(
        (yellow(&app) - 5.625_008e-4).abs() < 1e-9,
        "{}",
        yellow(&app)
    );
    app.update();
    assert_eq!(*app.world().get::<Condition>(id).unwrap(), Condition::DEAD);
    assert_eq!(yellow(&app), 0.0);
    assert!(app.world().get::<Ragdoll>(id).unwrap().drive.is_none());
    let limp = app.world().get::<GoLimp>(id).unwrap();
    assert_eq!((limp.velocity, limp.delay), (None, 0));
    assert!(app.world().get::<Collapse>(id).is_none());
    app.update();
    assert_eq!(yellow(&app), 0.0);
}

fn falling() -> CollapseStep {
    CollapseStep {
        stepping: false,
        reach: 80.0,
        balance: 1.0,
        scale_root: 1.0,
        tone: 1.0,
        chest: Vec3::new(0.0, 95.0, 0.0),
        velocity: Vec3::new(3.0, 0.0, 4.0),
    }
}

#[test]
fn a_collapse_starts_one_stagger_step() {
    let step = stagger_step(&falling()).unwrap();
    assert_eq!(step.length.to_bits(), 0x42a0_0000);
    assert_eq!(step.target.x.to_bits(), 0x42d7_faf0);
    assert_eq!(step.target.y.to_bits(), 0x42be_0000);
    assert_eq!(step.target.z.to_bits(), 0x430f_fca0);
    assert_eq!((step.landings, step.kind), (0, 0));
    let edge = stagger_step(&CollapseStep {
        reach: 0.01,
        balance: 0.4,
        ..falling()
    })
    .unwrap();
    assert_eq!(edge.length.to_bits(), 0x3b83_126f);
    let under = stagger_step(&CollapseStep {
        reach: 0.01,
        balance: f32::from_bits(0x3ecc_cccc),
        ..falling()
    })
    .unwrap();
    assert_eq!(under.length.to_bits(), 0x3b83_126e);
    let still = stagger_step(&CollapseStep {
        velocity: Vec3::ZERO,
        chest: Vec3::new(1.0, 2.0, 3.0),
        ..falling()
    })
    .unwrap();
    assert_eq!(still.target, Vec3::new(1.0, 2.0, 3.0));
    let rooted = stagger_step(&CollapseStep {
        balance: 0.8,
        scale_root: 2.0_f32.sqrt(),
        tone: 0.5,
        chest: Vec3::new(10.0, 0.0, -5.0),
        velocity: Vec3::new(0.0, 1.0, 0.0),
        ..falling()
    })
    .unwrap();
    assert_eq!(rooted.length.to_bits(), 0x4280_0000);
    assert_eq!(rooted.target.x.to_bits(), 0x4120_0000);
    assert_eq!(rooted.target.y.to_bits(), 0x42b4_f9a3);
    assert_eq!(rooted.target.z.to_bits(), 0xc0a0_0000);
    assert!(
        stagger_step(&CollapseStep {
            stepping: true,
            ..falling()
        })
        .is_none()
    );
}

fn two_feet() -> (Ragdoll, [Vec3; 2]) {
    let shin = |bone: usize, x: f32| PartSetup {
        bone,
        entity: Entity::from_raw_u32(bone as u32 + 1).unwrap(),
        parent: Some(0),
        world: Affine3A::from_translation(Vec3::new(x, 0.0, 0.0)),
        centre: Vec3::ZERO,
        half: Vec3::new(0.06, 0.2, 0.06),
        origin: Vec3::ZERO,
        links: vec![],
        foot: Some(Vec3::new(0.0, -0.2, 0.0)),
        pins: Vec::new(),
    };
    let pelvis = PartSetup {
        bone: 1,
        entity: Entity::from_raw_u32(1).unwrap(),
        parent: None,
        world: Affine3A::from_translation(Vec3::ZERO),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
    };
    let body = Ragdoll::new(&[pelvis, shin(3, 0.3), shin(5, -0.3)]).with_drive();
    let ankles = [Vec3::new(0.3, -0.2, 0.0), Vec3::new(-0.3, -0.2, 0.0)];
    (body, ankles)
}

#[test]
fn a_collapse_walks_the_stagger_it_starts() {
    let step = stagger_step(&falling()).unwrap();
    let (mut body, ankles) = two_feet();
    let spots = [Some(ankles[0]), Some(ankles[1])];
    let walked = body.walk_stagger(step.target, Vec3::ZERO, &spots, 1.0);
    assert_eq!(walked.foot, Some(0));
    assert!(!walked.ended);
    assert_eq!(walked.states, vec![FootState::ANCHORED, 0]);
    assert_eq!(walked.origin.x.to_bits(), 0x3f07_2b02);
    assert_eq!(walked.origin.y.to_bits(), 0);
    assert_eq!(walked.origin.z.to_bits(), 0x3f34_3958);
    assert_eq!(walked.anchors[0].x.to_bits(), 0x3f53_f7cf);
    assert_eq!(walked.anchors[0].y.to_bits(), 0xbe4c_cccc);
    assert_eq!(walked.anchors[0].z.to_bits(), 0x3f34_3958);
}

/// The dying tick walks the stagger from the chest it has already measured.
/// A chest that moves a metre along z, recorded three times so the fast
/// velocity is that move, anchors one foot. Without the walk the feet stay
/// unanchored.
#[test]
fn the_dying_tick_walks_the_stagger() {
    let (mut body, _) = two_feet();
    {
        let drive = body.drive.as_mut().unwrap();
        drive.balance_mut().record_chest(Vec3::ZERO);
        drive.balance_mut().record_chest(Vec3::Z);
        drive.balance_mut().record_chest(Vec3::Z);
    }
    let mut app = App::new();
    app.add_systems(Update, dying_tick);
    let id = app
        .world_mut()
        .spawn((
            Fighter::default(),
            Condition::COLLAPSING,
            Collapse::default(),
            body,
            Animator::new("Default", FALL_BACKWARD),
            GlobalTransform::default(),
        ))
        .id();
    app.insert_resource(TickedCharacter {
        body: id,
        character: None,
    });
    app.update();
    let ragdoll = app.world().get::<Ragdoll>(id).unwrap();
    let states: Vec<u32> = ragdoll.legs().iter().map(|leg| leg.state.0).collect();
    assert_eq!(states, vec![FootState::ANCHORED, 0]);
    assert!(ragdoll.drive.as_ref().unwrap().stepping());
    app.update();
    let ragdoll = app.world().get::<Ragdoll>(id).unwrap();
    let again: Vec<u32> = ragdoll.legs().iter().map(|leg| leg.state.0).collect();
    assert_eq!(again, vec![FootState::ANCHORED, 0]);
}
