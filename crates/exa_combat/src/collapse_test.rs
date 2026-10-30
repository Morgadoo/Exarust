//! The collapse before death: its start, the dying tick's rules and the
//! death 250 ticks later.
use bevy::ecs::system::RunSystemOnce;
use bevy::math::Affine3A;
use bevy::prelude::MinimalPlugins;
use exa_physics::Physics;
use exa_physics::body::pairs::{FOOT_LAYER, LIVING_SHIN_LAYER};
use exa_physics::body::{
    BodyPlugin, FootState, PIN_GAIN, Pairs, PartSetup, SolverStages, step_bodies,
};

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

/// Fall Backward's blend: round((1.2 − E) × 300) in double precision, ties
/// to even: 60 at full energy, 90 at 0.9, 174 at 0.62, 345 at the daze's
/// floor of 0.05.
#[test]
fn fall_backward_blends_by_the_energy_left() {
    for (energy, blend) in [(1.0, 60), (0.9, 90), (0.62, 174), (0.5, 210), (0.05, 345)] {
        assert_eq!(collapse_blend(energy), blend, "energy {energy}");
    }
}

/// The collapse's groan: the second draw sets the volume and the first the
/// pitch, both 0.1-scaled sums in double. At 0 and 1 the pairs are
/// (0.15, 0.8) and (0.25, 0.9).
#[test]
fn the_collapse_asks_for_a_groan_at_its_own_draws() {
    assert_eq!(collapse_voice(0.0, 0.0), (0.15, 0.8));
    assert_eq!(collapse_voice(1.0, 1.0), (0.25, 0.9));
    // The volume comes from the second draw, the pitch from the first.
    let (volume, pitch) = collapse_voice(0.5, 0.0);
    assert_eq!((volume, pitch), (0.15, 0.85));
    // A draw of 0.3 worked in single would give 0.83000004; in double the
    // product and sum then narrow to 0.83.
    assert_eq!(collapse_voice(0.3, 0.0).1, 0.83);
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
    let chain = exa_physics::part::PartState::STRIKE_CHAIN;
    body.parts[2].skips_thin = true;
    body.parts[2].state.0 |= chain;
    assert!(start_collapse(
        &mut condition,
        &mut flags,
        &mut anim,
        Some(&mut body),
        1.0,
        None
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
    // The foot is off the thin-plate bit and can no longer strike.
    assert!(!body.parts[2].skips_thin);
    assert_eq!(body.parts[2].state.0 & chain, 0);
    assert_eq!(body.level_mask(2), 0);
    // A second lethal wound changes nothing.
    anim.time = 12.0;
    assert!(!start_collapse(
        &mut condition,
        &mut flags,
        &mut anim,
        None,
        0.05,
        None
    ));
    assert_eq!((anim.time, anim.blend), (12.0, 60.0));
    let mut dead = Condition::DEAD;
    assert!(!start_collapse(
        &mut dead, &mut flags, &mut anim, None, 1.0, None
    ));
    assert_eq!(dead, Condition::DEAD);
    // Killed while falling backwards: the clip goes on, held.
    let mut falling = Animator::new("Default", "Idle");
    falling.play_once(FALL_BACKWARD, "Unconcious 1");
    falling.time = 7.0;
    let mut c = Condition::NORMAL;
    assert!(start_collapse(
        &mut c,
        &mut flags,
        &mut falling,
        None,
        0.5,
        None
    ));
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

/// One dying tick from a tone that crosses 0.25. Low red clears may-rise.
/// The golem suit keeps it and waits 10000 ms. A lost fall clears it even
/// for that suit. The dormant state flag is left alone.
#[test]
fn a_collapsing_death_settles_may_rise() {
    use crate::condition::{DeathClock, StateFlags};
    use crate::fall::{FallDeath, FallStage};
    use exa_formats::character_stats::BodyRules;
    use exa_physics::body::BodyTicks;

    let run = |mut fighter: Fighter, stage| {
        fighter.stats.rules = BodyRules(BodyRules::MAY_RISE);
        let mut app = App::new();
        app.add_systems(Update, dying_tick);
        let mut body = Ragdoll::new(&[]).with_drive();
        body.drive.as_mut().unwrap().tone = 0.25;
        let id = app
            .world_mut()
            .spawn((
                fighter,
                Condition::COLLAPSING,
                StateFlags(StateFlags::DORMANT | StateFlags::WAKING),
                FallDeath {
                    stage,
                    ..FallDeath::default()
                },
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
        app.insert_resource(BodyTicks(0));
        app.update();
        let rules = app.world().get::<Fighter>(id).unwrap().stats.rules.0;
        let flags = app.world().get::<StateFlags>(id).unwrap().0;
        let clock = *app.world().get::<DeathClock>(id).unwrap();
        (rules, flags, clock)
    };

    let low = Fighter {
        health: crate::Health {
            red: 0.0,
            ..crate::Health::of_race(1.0)
        },
        ..Fighter::default()
    };
    let (rules, flags, clock) = run(low, FallStage::Steady);
    assert_eq!(rules & BodyRules::MAY_RISE, 0);
    assert_eq!(flags, StateFlags::DORMANT | StateFlags::WAKING);
    assert_eq!(clock.wake_at_ms, None);
    assert_eq!(clock.died_at_ms, 0);

    let suited = Fighter {
        health: crate::Health {
            red: 0.01,
            ..crate::Health::of_race(1.0)
        },
        first_flag_red: true,
        ..Fighter::default()
    };
    let (rules, flags, clock) = run(suited, FallStage::Steady);
    assert_eq!(rules & BodyRules::MAY_RISE, BodyRules::MAY_RISE);
    assert_eq!(flags, StateFlags::DORMANT | StateFlags::WAKING);
    assert_eq!(clock.wake_at_ms, Some(crate::condition::DEATH_WAKE_MS));

    let lost = Fighter {
        first_flag_red: true,
        health: crate::Health {
            red: 1.0,
            ..crate::Health::of_race(1.0)
        },
        ..Fighter::default()
    };
    let (rules, _, clock) = run(lost, FallStage::Lost);
    assert_eq!(rules & BodyRules::MAY_RISE, 0);
    assert_eq!(clock.wake_at_ms, None);
}

/// Death clears the strike-chain bit on each lower leg's first child (the
/// foot), and going limp takes the lower legs out of the living layer (0,
/// the feet 0x30). The class word is not written: nothing reads it. A part
/// that is not a lower leg's first child keeps its chain bit.
#[test]
fn a_collapsing_death_clears_the_lower_legs_strikes() {
    use exa_physics::body::{PartSetup, pairs};
    use exa_physics::part::PartState;

    let mut app = App::new();
    app.add_systems(Update, dying_tick);
    let part = |bone, parent, foot| PartSetup {
        bone,
        entity: Entity::PLACEHOLDER,
        parent,
        world: Affine3A::IDENTITY,
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot,
        pins: Vec::new(),
        spheres: None,
    };
    let mut body = Ragdoll::new(&[
        part(1, None, None),
        part(3, Some(0), Some(Vec3::new(0.0, -0.4, 0.0))),
        part(4, Some(1), None),
    ])
    .with_drive();
    let chain = PartState::STRIKE_CHAIN;
    for p in &mut body.parts {
        p.class_word = 0x11;
        p.state.0 |= chain;
    }
    body.drive.as_mut().unwrap().tone = 0.25;
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
    let body = app.world().get::<Ragdoll>(id).unwrap();
    // The foot child loses its chain, the leg and the root keep theirs.
    assert_eq!(body.parts[2].state.0 & chain, 0);
    assert_eq!(body.parts[1].state.0 & chain, chain);
    assert_eq!(body.parts[0].state.0 & chain, chain);
    // The class word is left alone; the go-limp layer writer runs instead.
    assert_eq!(body.parts[0].class_word, 0x11);
    assert_eq!(body.parts[1].pairing.layer, 0);
    assert_eq!(body.parts[2].pairing.layer, pairs::FOOT_LAYER);
    assert_eq!(pairs::FOOT_LAYER, 0x30);
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
    // The direction uses the hardware reciprocal square root, whose result
    // is implementation-defined, so this component is compared with a
    // tolerance rather than bit for bit.
    let expected = f32::from_bits(0x42b4_f9a3);
    assert!((rooted.target.y - expected).abs() <= expected * 1e-3);
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
        spheres: None,
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
        spheres: None,
    };
    let mut body = Ragdoll::new(&[pelvis, shin(3, 0.3), shin(5, -0.3)]).with_drive();
    // The character frame faces away, so its side axis runs from foot 1 to
    // foot 0, as the stagger's score reads it.
    body.drive
        .as_mut()
        .unwrap()
        .set_frame(Affine3A::from_quat(bevy::math::Quat::from_rotation_y(
            std::f32::consts::PI,
        )));
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

fn sword(id: u32, velocity: Vec3) -> HeldWeapon {
    HeldWeapon {
        id,
        own_mobility: 1.25,
        mobility: 0.2,
        friction: 0.1,
        striker: 0x40,
        velocity,
        part: WeaponPart {
            id: u64::from(id),
            owner: 9,
            root: 1,
            children: Vec::new(),
            object_type: 0,
            object_flags: 0,
            mark: 0,
            iterations: 0,
        },
        parent: None,
        links: 1,
        grip: -1,
        awake: 1,
        shape: None,
    }
}

/// The dying tick drops both held weapons. Friction is 0.75, mobility is
/// the item's own, the drag is that mobility times 0.175, the striker
/// flags clear, and the velocity stays. The hands, the class word and the
/// primary are empty. A second tick adds nothing.
#[test]
fn the_dying_tick_drops_the_held_weapons() {
    let mut app = App::new();
    app.add_systems(Update, dying_tick);
    let hands = Hands {
        count: 2,
        slots: [
            Some(sword(3, Vec3::X * 1.5)),
            Some(sword(4, Vec3::new(0.0, 0.0, -0.25))),
        ],
        class_word: 0x20,
        primary: 9,
        loose: Vec::new(),
    };
    let id = app
        .world_mut()
        .spawn((
            Fighter::default(),
            Condition::COLLAPSING,
            Collapse::default(),
            Ragdoll::new(&[]).with_drive(),
            Animator::new("Default", FALL_BACKWARD),
            GlobalTransform::default(),
            hands,
        ))
        .id();
    app.insert_resource(TickedCharacter {
        body: id,
        character: None,
    });
    app.update();
    let hands = app.world().get::<Hands>(id).unwrap();
    assert_eq!(hands.count, 0);
    assert_eq!(hands.class_word, 0);
    assert_eq!(hands.primary, 0);
    assert!(hands.slots.iter().all(Option::is_none));
    assert_eq!(hands.loose.len(), 2);
    assert_eq!(hands.loose[0].id, 3);
    assert_eq!(hands.loose[0].mobility.to_bits(), 0x3fa0_0000);
    assert_eq!(hands.loose[0].drag.to_bits(), 0x3e60_0000);
    assert_eq!(hands.loose[0].friction.to_bits(), 0x3f40_0000);
    assert_eq!(hands.loose[0].striker, 0);
    assert_eq!(hands.loose[0].velocity.x.to_bits(), 0x3fc0_0000);
    assert_eq!(hands.loose[1].id, 4);
    assert_eq!(hands.loose[1].velocity.z.to_bits(), 0xbe80_0000);
    app.update();
    assert_eq!(app.world().get::<Hands>(id).unwrap().loose.len(), 2);
    let mut bare = Hands {
        count: 0,
        class_word: 0x11,
        ..Hands::default()
    };
    let mut condition = Condition::NORMAL;
    let mut flags = StateFlags::default();
    let mut anim = Animator::new("Default", "Idle");
    assert!(start_collapse(
        &mut condition,
        &mut flags,
        &mut anim,
        None,
        1.0,
        Some(&mut bare)
    ));
    assert_eq!((bare.count, bare.class_word, bare.loose.len()), (0, 0, 0));
}

fn part(id: u64, mark: i32, iterations: i32, children: Vec<WeaponPart>) -> WeaponPart {
    WeaponPart {
        id,
        owner: 9,
        root: id.wrapping_sub(2),
        children,
        object_type: 0,
        object_flags: 0,
        mark,
        iterations,
    }
}

/// The dying tick makes the dropped weapon its own owner. A child and a
/// grandchild copy that owner and take the weapon as their root. The
/// parent list loses the weapon. A child of type 0x1089 with bit
/// 0x400000 takes mark -2. A childless weapon keeps a negative grip.
#[test]
fn the_dying_tick_returns_the_weapon_to_itself() {
    let mut child = part(7, 1, 11, vec![part(8, 8, 12, Vec::new())]);
    child.object_type = 0x1089;
    child.object_flags = 0x400001;
    let mut armed = sword(3, Vec3::ZERO);
    armed.part = part(3, 6, 99, vec![child]);
    armed.parent = Some(vec![5, 3, 8]);
    armed.grip = 4;
    armed.links = 6;
    armed.awake = 9;
    let mut other = sword(4, Vec3::ZERO);
    other.part.mark = 4;
    other.part.object_type = 0x89;
    other.part.object_flags = 0x400000;
    other.parent = Some(vec![9]);
    other.grip = -3;
    let mut app = App::new();
    app.add_systems(Update, dying_tick);
    let id = app
        .world_mut()
        .spawn((
            Fighter::default(),
            Condition::COLLAPSING,
            Collapse::default(),
            Ragdoll::new(&[]).with_drive(),
            Animator::new("Default", FALL_BACKWARD),
            GlobalTransform::default(),
            Hands {
                count: 2,
                slots: [Some(armed), Some(other)],
                class_word: 0,
                primary: 9,
                loose: Vec::new(),
            },
        ))
        .id();
    app.insert_resource(TickedCharacter {
        body: id,
        character: None,
    });
    app.update();
    let hands = app.world().get::<Hands>(id).unwrap();
    let first = &hands.loose[0];
    assert_eq!(first.part.owner, 3);
    assert_eq!(first.part.children[0].owner, 3);
    assert_eq!(first.part.children[0].children[0].owner, 3);
    assert_eq!(first.part.root, 3);
    assert_eq!(first.part.children[0].root, 3);
    assert_eq!(first.part.children[0].children[0].root, 3);
    assert_eq!(first.part.iterations, 4);
    assert_eq!(first.part.children[0].iterations, 11);
    assert_eq!(first.part.children[0].children[0].iterations, 12);
    assert_eq!(first.links, 0);
    assert_eq!(first.grip, -1);
    assert_eq!(first.awake, 0);
    assert_eq!(first.parent.as_deref(), Some(&[5, 8][..]));
    assert_eq!(first.part.mark, 6);
    assert_eq!(first.part.children[0].mark, -2);
    assert_eq!(first.part.children[0].children[0].mark, 8);
    let second = &hands.loose[1];
    assert_eq!(second.part.owner, 4);
    assert_eq!(second.part.root, 4);
    assert_eq!(second.part.iterations, 2);
    assert_eq!(second.grip, -3);
    assert_eq!(second.part.mark, 4);
    assert_eq!(second.parent.as_deref(), Some(&[9][..]));
    assert_eq!(second.links, 0);
    assert_eq!(second.awake, 0);
    app.update();
    let again = app.world().get::<Hands>(id).unwrap();
    assert_eq!(again.loose.len(), 2);
    assert_eq!(again.loose[0].part.owner, 3);
    assert_eq!(again.loose[0].parent.as_deref(), Some(&[5, 8][..]));
}

fn blade(at: Vec3) -> RigidPart {
    RigidPart {
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(at),
        rest: [
            Vec3::new(-0.2, 0.0, -0.2),
            Vec3::new(0.2, 0.0, -0.2),
            Vec3::new(0.0, 0.0, 0.2),
            Vec3::new(0.0, 0.4, 0.0),
        ],
        point_masses: [4.0; 4],
        parent_world: Affine3A::IDENTITY,
        mobility: 0.2,
        spheres: vec![(Vec3::new(0.0, 0.2, 0.0), 0.2)],
        origin: Vec3::ZERO,
        links: Vec::new(),
        mesh: None,
        triangles: None,
    }
}

fn free_bodies(app: &mut App) -> Vec<Entity> {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, (With<Ragdoll>, Without<Fighter>)>();
    query.iter(app.world()).collect()
}

/// The dying tick puts a solid dropped weapon into the solver. It is awake,
/// keeps mobility 1.25, friction 0.75, its sphere and its velocity. One
/// solver tick moves it. A flat shape spawns nothing, and a second dying
/// tick adds no body.
#[test]
fn the_dying_tick_steps_the_dropped_weapon() {
    let mut solid = sword(3, Vec3::X * 1.5);
    solid.shape = Some(blade(Vec3::Y));
    let mut flat = sword(4, Vec3::ZERO);
    flat.shape = Some(blade(Vec3::ZERO));
    if let Some(shape) = flat.shape.as_mut() {
        shape.rest[3] = Vec3::new(0.1, 0.0, 0.0);
    }
    let mut app = App::new();
    app.add_systems(Update, dying_tick);
    let id = app
        .world_mut()
        .spawn((
            Fighter::default(),
            Condition::COLLAPSING,
            Collapse::default(),
            Ragdoll::new(&[]).with_drive(),
            Animator::new("Default", FALL_BACKWARD),
            GlobalTransform::default(),
            Hands {
                count: 2,
                slots: [Some(solid), Some(flat)],
                class_word: 0,
                primary: 0,
                loose: Vec::new(),
            },
        ))
        .id();
    app.insert_resource(TickedCharacter {
        body: id,
        character: None,
    });
    app.update();
    let hands = app.world().get::<Hands>(id).unwrap();
    assert!(hands.loose[0].embodied);
    assert!(!hands.loose[1].embodied);
    let bodies = free_bodies(&mut app);
    assert_eq!(bodies.len(), 1);
    let weapon = app.world().get::<Ragdoll>(bodies[0]).unwrap();
    assert!(!weapon.asleep());
    assert_eq!(weapon.countdown(), 256);
    let part = &weapon.parts[0];
    assert_eq!(part.friction.to_bits(), 0x3f40_0000);
    assert_eq!(part.inverse_mass.to_bits(), 0x3fa0_0000);
    assert_eq!(*part.spheres, vec![(Vec3::new(0.0, 0.2, 0.0), 0.2)]);
    let back = 1.5 / 83.0;
    assert_eq!(part.previous[0].x, part.positions[0].x - back);
    assert_eq!(part.positions[3], Vec3::new(0.0, 1.4, 0.0));
    app.update();
    assert_eq!(free_bodies(&mut app).len(), 1);
    let mut weapon = app.world_mut().get_mut::<Ragdoll>(bodies[0]).unwrap();
    weapon.tick(&[] as &[[Vec3; 3]]);
    let p = weapon.parts[0].positions[0];
    assert_eq!(p.x.to_bits(), 0xbe3a_7bb7);
    assert_eq!(p.y.to_bits(), 0x3f7f_a2b5);
    assert_eq!(p.z.to_bits(), 0xbe4c_cccd);
}

fn sphere_centre(body: &Ragdoll) -> Vec3 {
    let part = &body.parts[0];
    let (centre, _) = part.spheres[0];
    part.transform().transform_point3(centre)
}

/// The dying tick's free weapon meets another body in the shipped solver.
/// The two spheres start on top of each other. One `step_bodies` records
/// the contact and pushes them apart. Stepping the weapon alone leaves it
/// at the bits the free-body test pins.
#[test]
fn the_dropped_weapon_meets_another_body() {
    let weapon_entity = Entity::from_raw_u32(30).unwrap();
    let other_entity = Entity::from_raw_u32(40).unwrap();
    let mut solid = sword(3, Vec3::X * 1.5);
    let mut shape = blade(Vec3::Y);
    shape.entity = weapon_entity;
    solid.shape = Some(shape);
    let mut other = blade(Vec3::Y);
    other.entity = other_entity;
    other.mobility = 1.25;
    let sleeper = Ragdoll::rigid(&[other]);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, BodyPlugin))
        .insert_resource(Physics::default())
        .add_systems(Update, dying_tick);
    let id = app
        .world_mut()
        .spawn((
            Fighter::default(),
            Condition::COLLAPSING,
            Collapse::default(),
            Ragdoll::new(&[]).with_drive(),
            Animator::new("Default", FALL_BACKWARD),
            GlobalTransform::default(),
            Hands {
                count: 1,
                slots: [Some(solid), None],
                class_word: 0,
                primary: 0,
                loose: Vec::new(),
            },
        ))
        .id();
    app.world_mut().spawn(sleeper);
    app.insert_resource(TickedCharacter {
        body: id,
        character: None,
    });
    app.update();
    let bodies = free_bodies(&mut app);
    assert_eq!(bodies.len(), 2);
    let before = {
        let world = app.world();
        let weapon = world
            .iter_entities()
            .find_map(|e| {
                let body = world.get::<Ragdoll>(e.id())?;
                (body.parts.first()?.entity == weapon_entity).then(|| sphere_centre(body))
            })
            .unwrap();
        let other = world
            .iter_entities()
            .find_map(|e| {
                let body = world.get::<Ragdoll>(e.id())?;
                (body.parts.first()?.entity == other_entity).then(|| sphere_centre(body))
            })
            .unwrap();
        weapon.distance(other)
    };
    assert!(before < 1e-5, "{before}");
    app.world_mut().run_system_once(step_bodies).unwrap();
    let touches = app
        .world()
        .resource::<SolverStages>()
        .get::<Pairs>()
        .unwrap()
        .touches();
    assert_eq!(touches.len(), 1);
    assert_eq!(touches[0].a.0, weapon_entity);
    assert_eq!(touches[0].b.0, other_entity);
    assert_eq!(touches[0].hits, 1);
    assert_eq!(touches[0].deepest.to_bits(), 0x3ec3_9cd9);
    let moved = {
        let world = app.world();
        let weapon = world
            .iter_entities()
            .find_map(|e| {
                let body = world.get::<Ragdoll>(e.id())?;
                (body.parts.first()?.entity == weapon_entity).then(|| sphere_centre(body))
            })
            .unwrap();
        let other = world
            .iter_entities()
            .find_map(|e| {
                let body = world.get::<Ragdoll>(e.id())?;
                (body.parts.first()?.entity == other_entity).then(|| sphere_centre(body))
            })
            .unwrap();
        weapon.distance(other)
    };
    assert!(moved > before);
    let alone = Vec3::new(
        f32::from_bits(0xbe3a_7bb7),
        f32::from_bits(0x3f7f_a2b5),
        f32::from_bits(0xbe4c_cccd),
    );
    let at = app
        .world()
        .iter_entities()
        .find_map(|entity| {
            let body = app.world().get::<Ragdoll>(entity.id())?;
            (body.parts.first()?.entity == weapon_entity).then(|| body.parts[0].positions[0])
        })
        .unwrap();
    assert_ne!(at, alone);
    assert_eq!(at.x.to_bits(), 0xbda3_ae15);
    assert_eq!(at.y.to_bits(), 0x3f7e_43be);
    assert_eq!(at.z.to_bits(), 0xbe49_2d0c);
}

/// A collapse's start asks for its groan once, from its own roll's first
/// two draws (the first the pitch, the second the volume); a later tick
/// asks nothing more, and a body added already dead asks nothing.
#[test]
fn a_starting_collapse_groans_once() {
    let mut app = App::new();
    app.add_message::<crate::pain::PainSounded>()
        .add_systems(Update, collapse_groans);
    let fighter = app
        .world_mut()
        .spawn((Condition::COLLAPSING, Collapse::default()))
        .id();
    app.world_mut()
        .spawn((Condition::DEAD, Collapse::default()));
    app.update();
    app.update();
    let messages = app
        .world()
        .resource::<bevy::ecs::message::Messages<crate::pain::PainSounded>>();
    let mut cursor = messages.get_cursor();
    let all: Vec<_> = cursor.read(messages).collect();
    assert_eq!(all.len(), 1);
    let mut roll = Roll::default();
    let (first, second) = (roll.next(), roll.next());
    // The second draw from the start state is 0x8088406·2⁻³².
    assert_eq!(second, (0x0808_8406_u32 as f64 * 2f64.powi(-32)) as f32);
    let (volume, pitch) = collapse_voice(first, second);
    assert_eq!(all[0].fighter, fighter);
    assert_eq!(
        all[0].sounds.voice,
        Some((2, crate::pain::Loudness { volume, pitch }))
    );
    assert_eq!(all[0].sounds.wound, None);
    assert!((volume - 0.153_138).abs() < 1e-5 && (pitch - 0.8).abs() < 1e-6);
}
