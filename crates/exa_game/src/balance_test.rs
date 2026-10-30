//! The posture stage's falls, staggers and get-ups, pinned to the
//! original's numbers (`docs/formats/RSQ.md`, Falls and get-ups).
//! (Included from balance.rs: `#[path = "balance_test.rs"] mod tests;`)
use std::collections::HashMap;
use std::sync::Arc;

use bevy::ecs::system::RunSystemOnce;
use bevy::math::Affine3A;
use exa_anim::Capture;
use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
use exa_physics::body::PartSetup;

use super::*;

/// The unit lean `degrees` off the vertical, toward `toward` (across the
/// ground).
fn lean(degrees: f32, toward: Vec3) -> Vec3 {
    let (s, c) = degrees.to_radians().sin_cos();
    toward.normalize() * s + Vec3::Y * c
}

/// Leaning upright (lean height over 0.8, double), a still chest (slow
/// speed² at most 0.3) takes the Defence stance, a moving one Rebalance;
/// the float 0.8 is over the double 0.8 and the float 0.3 over the double
/// 0.3, as the original compares them.
#[test]
fn upright_bodies_steady_themselves() {
    let ahead = lean(30.0, Vec3::Z);
    assert_eq!(respond(ahead, Vec3::Z, 0.29, 0.5, 40.0), Response::Defence);
    assert_eq!(
        respond(ahead, Vec3::Z, 0.31, 0.5, 40.0),
        Response::Rebalance
    );
    assert_eq!(respond(ahead, Vec3::Z, 0.3, 0.5, 40.0), Response::Rebalance);
    // Upright wins even lost and far off: balance 0.05, 90 cm.
    assert_eq!(respond(ahead, Vec3::Z, 0.0, 0.05, 90.0), Response::Defence);
    let edge = Vec3::new(0.6, 0.8, 0.0);
    assert_eq!(respond(edge, Vec3::Z, 0.0, 0.5, 40.0), Response::Defence);
    let past = Vec3::new(0.6, 0.799_99, 0.0);
    assert!(matches!(
        respond(past, Vec3::Z, 0.0, 0.5, 40.0),
        Response::Fall(_)
    ));
    assert_eq!(Response::Defence.start(), ("Defence stance", 40));
    assert_eq!(Response::Rebalance.start(), ("Rebalance", 20));
}

/// Leaning more than 37° but well ahead or behind (|lean · forward| over
/// 0.4), slowly (speed² under 8), not lost (balance over 0.1) and under
/// 80 cm off, a body staggers: Get up leaning ahead, Get Up 2 leaning back,
/// both with a blend of 50; the float 0.4 and 0.1 pass as in the
/// original's double compares.
#[test]
fn moderate_leans_stagger() {
    let ahead = lean(45.0, Vec3::Z);
    let back = lean(45.0, -Vec3::Z);
    let stagger = |l| respond(l, Vec3::Z, 2.0, 0.45, 50.0);
    assert_eq!(stagger(ahead), Response::Stagger(Rise::FaceDown));
    assert_eq!(stagger(back), Response::Stagger(Rise::FaceUp));
    assert_eq!(
        Response::Stagger(Rise::FaceDown).start(),
        ("Get up", STAGGER_BLEND)
    );
    assert_eq!(Response::Stagger(Rise::FaceUp).start(), ("Get Up 2", 50));
    let at = Vec3::new(0.6, 0.6, 0.4);
    assert_eq!(
        respond(at, Vec3::Z, 7.99, 0.1, 79.99),
        Response::Stagger(Rise::FaceDown)
    );
    // The stagger keys: Get up's fourth (frame 55), Get Up 2's third (40).
    assert_eq!(
        (Rise::FaceDown.stagger_key(), Rise::FaceUp.stagger_key()),
        (3, 2)
    );
}

/// Sideways (|lean · forward| at most 0.4), fast (speed² 8 or more), lost
/// (balance at most 0.1) or 80 cm off, a body falls: forward when the lean
/// is ahead, else backward (and straight sideways), with no blend.
#[test]
fn far_fast_lost_or_sideways_leans_fall() {
    let side = lean(50.0, Vec3::X);
    assert_eq!(
        respond(side, Vec3::Z, 0.0, 0.45, 50.0),
        Response::Fall(false)
    );
    let at = Vec3::new(0.6, 0.6, 0.39);
    assert_eq!(respond(at, Vec3::Z, 2.0, 0.45, 50.0), Response::Fall(true));
    let ahead = lean(45.0, Vec3::Z);
    let back = lean(45.0, -Vec3::Z);
    assert_eq!(
        respond(ahead, Vec3::Z, 8.0, 0.45, 50.0),
        Response::Fall(true)
    );
    assert_eq!(
        respond(back, Vec3::Z, 8.0, 0.45, 50.0),
        Response::Fall(false)
    );
    assert_eq!(
        respond(ahead, Vec3::Z, 2.0, 0.099_99, 50.0),
        Response::Fall(true)
    );
    assert_eq!(
        respond(back, Vec3::Z, 2.0, 0.45, 80.0),
        Response::Fall(false)
    );
    assert_eq!(Response::Fall(true).start(), ("Fall Forward", 0));
    assert_eq!(Response::Fall(false).start(), ("Fall Backward", 0));
}

/// Standing in a plain clip under balance 0.8 (double), a body steadies
/// itself: the Defence stance with a still chest (fast speed² at most
/// 0.3), else Rebalance; at the float 0.8 nothing.
#[test]
fn unsteady_bodies_steady_themselves() {
    assert_eq!(steady(0.79, 0.29), Some(Response::Defence));
    assert_eq!(steady(0.6, 0.0), Some(Response::Defence));
    assert_eq!(steady(0.79, 0.31), Some(Response::Rebalance));
    assert_eq!(steady(0.8, 5.0), None);
    assert_eq!(steady(1.0, 0.0), None);
}

/// The get-up by the pelvis's front axis: with none playing face down at
/// a height of 0 or less (Get up), else Get Up 2; Get up holds on to a
/// height of 0.2, Get Up 2 down to −0.2.
#[test]
fn get_ups_pick_by_the_pelvis_with_hysteresis() {
    assert_eq!(pick_rise(0.0, None), Rise::FaceDown);
    assert_eq!(pick_rise(0.01, None), Rise::FaceUp);
    assert_eq!(pick_rise(-0.5, None), Rise::FaceDown);
    let down = Some(Rise::FaceDown);
    assert_eq!(pick_rise(0.2, down), Rise::FaceDown);
    assert_eq!(pick_rise(0.21, down), Rise::FaceUp);
    let up = Some(Rise::FaceUp);
    assert_eq!(pick_rise(-0.2, up), Rise::FaceDown);
    assert_eq!(pick_rise(-0.19, up), Rise::FaceUp);
    assert_eq!(pick_rise(f32::NAN, None), Rise::FaceDown);
    assert_eq!(Rise::of_action("Get Up 2"), Some(Rise::FaceUp));
    assert_eq!(Rise::of_action("Idle"), None);
}

/// A body lying still waits for its feet's support, at most 100 ticks;
/// a get-up already playing never waits.
#[test]
fn lying_bodies_wait_100_ticks_for_support() {
    assert!(waits_for_support(None, false, 99));
    assert!(!waits_for_support(None, false, 100));
    assert!(!waits_for_support(None, true, 0));
    assert!(!waits_for_support(Some(Rise::FaceUp), false, 0));
}

/// A get-up faces from the pelvis to the chest across the ground (Get up)
/// or the other way (Get Up 2), and turns a heading more than 18.2° off
/// (cosine under the double 0.95; the float 0.95 is under it).
#[test]
fn get_ups_face_the_way_they_rise() {
    let pelvis = Vec3::new(1.0, 0.15, 2.0);
    let chest = Vec3::new(1.0, 0.25, 2.6);
    assert_eq!(rise_heading(chest, pelvis, Rise::FaceDown), Some(Vec3::Z));
    assert_eq!(rise_heading(chest, pelvis, Rise::FaceUp), Some(-Vec3::Z));
    assert_eq!(rise_heading(pelvis + Vec3::Y, pelvis, Rise::FaceUp), None);
    let off = |deg: f32| Quat::from_rotation_y(deg.to_radians()) * Vec3::Z;
    assert!(turns_to_rise(off(18.3), Vec3::Z));
    assert!(!turns_to_rise(off(18.1), Vec3::Z));
    assert!(turns_to_rise(Vec3::new(0.0, 0.0, 0.95), Vec3::Z));
}

/// Under balance 0.8 a body turns toward its fall once its pelvis has
/// turned more than 45° from its clip's (cosine at most 0.7): from the
/// chest to the pelvis with the front axis up, from the pelvis to the
/// chest with it down, else along it; not when its heading is within 37°.
#[test]
fn falls_turn_the_heading() {
    let pelvis = Vec3::new(0.0, 0.3, 0.0);
    let chest = Vec3::new(0.0, 0.5, 0.5);
    let animated = Vec3::Z;
    let up = Vec3::new(0.0, 0.9, 0.436);
    assert_eq!(
        fall_heading(animated, up, pelvis, chest, Vec3::X),
        Some(-Vec3::Z)
    );
    let down = Vec3::new(0.0, -0.9, 0.436);
    assert_eq!(
        fall_heading(animated, down, pelvis, chest, Vec3::X),
        Some(Vec3::Z)
    );
    let side = Vec3::X;
    assert_eq!(
        fall_heading(animated, side, pelvis, chest, -Vec3::X),
        Some(Vec3::X)
    );
    // Within 45° of its clip's: the heading stays.
    let near = Quat::from_rotation_x(44f32.to_radians()) * Vec3::Z;
    assert!(animated.dot(near) > 0.7);
    assert_eq!(fall_heading(animated, near, pelvis, chest, -Vec3::Z), None);
    // Already within 37° of its fall: it stays.
    let close = Quat::from_rotation_y(36f32.to_radians()) * Vec3::X;
    assert_eq!(fall_heading(animated, side, pelvis, chest, close), None);
}

/// Getting up, energy gains 0.01 × (1 − progress) a tick beyond its
/// recovery: 0.01 at the start, 0.005 halfway, none at the end.
#[test]
fn a_get_up_restores_energy() {
    assert!((rising_energy(0.0) - 0.01).abs() < 1e-9);
    assert!((rising_energy(0.5) - 0.005).abs() < 1e-9);
    assert_eq!(rising_energy(1.0), 0.0);
    assert!((rising_energy(-0.25) - 0.0125).abs() < 1e-9);
}

/// The installed motion set holds the clips the posture stage starts, at
/// the original's action indices, as it plays them (skipped without
/// `EXANIMA_DIR`): the Defence stance (3), the falls (0x10, 0x11) and
/// Rebalance (0x21) looping, the get-ups (0x0E, 0x0F) played once with the
/// key times the stagger jumps in by (Get up 20, 40, 55, 70, 80; Get Up 2
/// 15, 40, 63, 82, 98, 110, 120); and the actions the condition's posture
/// decision tells apart.
#[test]
fn the_installed_posture_clips() {
    use exa_combat::condition::action;
    let Some(dir) = exa_assets::locate_install(None) else {
        return;
    };
    let data = exa_assets::GameData::open(&dir).unwrap();
    let game = exa_assets::Game(Arc::new(data));
    let motions = Motions::load(&game);
    let at = |name: &str| motions.0.action_index(name);
    let clip = |name: &str| {
        motions
            .0
            .clip("Default", exa_anim::MovesetConfig::NONE, name)
            .unwrap()
    };
    for (name, index, looping) in [
        (DEFENCE, 0x03, true),
        ("Get up", 0x0E, false),
        ("Get Up 2", 0x0F, false),
        (FALL_FORWARD, 0x10, true),
        (FALL_BACKWARD, 0x11, true),
        (REBALANCE, 0x21, true),
    ] {
        assert_eq!(at(name), Some(index), "{name}");
        assert_eq!(clip(name).mode & 1 == 1, looping, "{name}");
    }
    let times = |name: &str| -> Vec<f32> { clip(name).keys.iter().map(|k| k.time).collect() };
    assert_eq!(times("Get up"), [20.0, 40.0, 55.0, 70.0, 80.0]);
    assert_eq!(
        times("Get Up 2"),
        [15.0, 40.0, 63.0, 82.0, 98.0, 110.0, 120.0]
    );
    assert_eq!(at("Walk"), Some(action::WALK));
    assert_eq!(at("Run"), Some(action::RUN));
    assert_eq!(at("Cmbt - Idle"), Some(action::COMBAT_IDLE));
    assert_eq!(at("Attack Mid"), Some(*action::ATTACKS.start()));
    assert_eq!(at("Overhead Swing"), Some(*action::ATTACKS.end()));
    assert_eq!(at("Kick"), Some(action::KICK));
    assert_eq!(at("Switch Weapons"), Some(action::SWITCH_WEAPONS));
    assert_eq!(at("Force Blast"), Some(*action::POWERS.start()));
    assert_eq!(at("Disp Form"), Some(*action::POWERS.end()));
    assert_eq!(exa_combat::collapse::FALL_BACKWARD, FALL_BACKWARD);
    assert_eq!(exa_combat::collapse::REBALANCE, REBALANCE);
}

/// A motion set of the clips the posture stage starts: the stances, the
/// falls and Rebalance looping, the get-ups once, with the installed key
/// times of Get up and Get Up 2.
fn motions() -> Motions {
    let clips: [(&str, i32, &[f32]); 7] = [
        ("Idle", 3, &[20.0, 80.0]),
        ("Defence stance", 3, &[20.0, 40.0, 60.0, 80.0]),
        ("Rebalance", 3, &[20.0, 40.0]),
        ("Fall Forward", 3, &[10.0, 30.0]),
        ("Fall Backward", 3, &[10.0, 30.0]),
        ("Get up", 0, &[20.0, 40.0, 55.0, 70.0, 80.0]),
        ("Get Up 2", 0, &[15.0, 40.0, 63.0, 82.0, 98.0, 110.0, 120.0]),
    ];
    let set = MotionSet {
        bone_count: 1,
        actions: clips
            .iter()
            .map(|(n, _, _)| Action {
                name: (*n).into(),
                params: [0, 0],
            })
            .collect(),
        movesets: vec![Moveset {
            name: "Default".into(),
            parent: String::new(),
            clips: clips
                .iter()
                .map(|&(_, mode, times)| {
                    Some(Clip {
                        mode,
                        keys: times
                            .iter()
                            .map(|&time| Key {
                                time,
                                root: [0.0, 1.0, 0.0],
                                rotations: vec![[0.0, 0.0, 0.0, 1.0]],
                                flags: vec![0],
                            })
                            .collect(),
                    })
                })
                .collect(),
        }],
    };
    Motions::new(Arc::new(set), Arc::new(HashMap::new()), 0.0)
}

/// A living one-part body (a pelvis turned by `turn`, 15 cm up), its root
/// part's origin its chest.
fn pelvis(turn: Quat) -> Ragdoll {
    Ragdoll::new(&[PartSetup {
        bone: 1,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_rotation_translation(turn, Vec3::new(0.0, 0.15, 0.0)),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: None,
    }])
    .with_drive()
}

/// Each response starts its clip as the original's posture block does:
/// the Defence stance (40) and Rebalance (20, its wait 41) looping, a fall
/// looping with no blend and off the controller, a stagger's get-up played
/// once into the stance with a blend of 50 from the whole capture, jumped
/// in at its key (Get up at 55, Get Up 2 at 40), getting up and on the
/// controller; all eased fast. A clip already playing does not start
/// again, but is eased fast.
#[test]
fn responses_start_as_the_original() {
    let motions = motions();
    let run = |response| {
        let mut body = pelvis(Quat::IDENTITY);
        let mut anim = Animator::new("Default", "Idle");
        let mut condition = Condition::NORMAL;
        let started = begin(
            response,
            &mut body,
            &mut anim,
            &mut condition,
            &motions,
            "Idle",
        );
        (started, body, anim, condition)
    };
    let (started, body, anim, condition) = run(Response::Defence);
    assert!(started && anim.looping && anim.then.is_none());
    assert_eq!((anim.action.as_str(), anim.blend), ("Defence stance", 40.0));
    assert_eq!((anim.ease, condition), (Ease::Fast, Condition::NORMAL));
    assert!(body.drive.as_ref().unwrap().supported);
    let (_, body, anim, _) = run(Response::Rebalance);
    assert_eq!((anim.action.as_str(), anim.blend), ("Rebalance", 20.0));
    assert_eq!(body.drive.as_ref().unwrap().posture_wait, 41);
    let (_, body, anim, _) = run(Response::Fall(false));
    assert_eq!((anim.action.as_str(), anim.blend), ("Fall Backward", 0.0));
    assert!(anim.looping && anim.pending == Some(Capture::Plain));
    assert!(!body.drive.as_ref().unwrap().supported);
    let (started, body, anim, condition) = run(Response::Stagger(Rise::FaceDown));
    assert!(started && !anim.looping);
    assert_eq!(anim.then.as_deref(), Some("Idle"));
    assert_eq!((anim.action.as_str(), anim.blend), ("Get up", 50.0));
    assert_eq!((anim.time, anim.capture_key), (55.0, 3));
    assert_eq!(anim.pending, Some(Capture::Whole));
    assert_eq!((anim.ease, condition), (Ease::Fast, Condition::GETTING_UP));
    assert!(body.drive.as_ref().unwrap().supported);
    let (_, _, anim, _) = run(Response::Stagger(Rise::FaceUp));
    assert_eq!((anim.time, anim.capture_key), (40.0, 2));

    // Falling on: no new start, the fast ease again.
    let mut body = pelvis(Quat::IDENTITY);
    let mut anim = Animator::new("Default", "Fall Forward");
    anim.time = 12.0;
    let mut condition = Condition::NORMAL;
    let fall = Response::Fall(true);
    assert!(!begin(
        fall,
        &mut body,
        &mut anim,
        &mut condition,
        &motions,
        "Idle"
    ));
    assert_eq!(
        (anim.time, anim.ease, anim.pending),
        (12.0, Ease::Fast, None)
    );
}

/// A world ticking `body` as the character tick's posture stage would:
/// `branch` its branch this tick, `condition` its condition, playing
/// `action`.
fn posed(body: Ragdoll, action: &str, branch: Branch, condition: Condition) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(motions());
    let id = world
        .spawn((
            body,
            Animator::new("Default", action),
            condition,
            Stance::default(),
            PostureBranch(branch),
            StateFlags::NONE,
            Transform::default(),
        ))
        .id();
    world.insert_resource(TickedCharacter {
        body: id,
        character: None,
    });
    (world, id)
}

/// Off balance and upright, the posture stage starts the Defence stance
/// (blend 40, eased fast) and marks the body off its feet; standing again
/// the mark goes. Every posture stage takes a tick off the switch's wait.
#[test]
fn the_posture_stage_starts_and_marks() {
    let mut body = pelvis(Quat::IDENTITY);
    body.drive.as_mut().unwrap().posture_wait = 41;
    let (mut world, id) = posed(body, "Idle", Branch::Falling, Condition::NORMAL);
    world.run_system_once(take_posture).unwrap();
    let anim = world.get::<Animator>(id).unwrap();
    assert_eq!((anim.action.as_str(), anim.blend), ("Defence stance", 40.0));
    assert_eq!(anim.ease, Ease::Fast);
    assert!(world.get::<Fallen>(id).is_some());
    let wait = |w: &World| {
        w.get::<Ragdoll>(id)
            .unwrap()
            .drive
            .as_ref()
            .unwrap()
            .posture_wait
    };
    assert_eq!(wait(&world), 40);
    world.get_mut::<PostureBranch>(id).unwrap().0 = Branch::Standing;
    world.run_system_once(take_posture).unwrap();
    assert!(world.get::<Fallen>(id).is_none());
    assert_eq!(wait(&world), 39);
}

/// A Rebalance started off balance waits 41 ticks (27 unsteady): its own
/// posture stage leaves 40 and every later one takes one off, so the
/// posture switch (under 1) may start the stance on the 41st tick after it.
#[test]
fn rebalances_hold_the_switch_41_ticks() {
    assert_eq!((OFF_BALANCE_WAIT, UNSTEADY_WAIT), (41, 27));
    let motions = motions();
    let mut body = pelvis(Quat::IDENTITY);
    let mut anim = Animator::new("Default", "Idle");
    let mut condition = Condition::NORMAL;
    begin(
        Response::Rebalance,
        &mut body,
        &mut anim,
        &mut condition,
        &motions,
        "Idle",
    );
    let (mut world, id) = posed(body, "Rebalance", Branch::GettingUp, Condition::NORMAL);
    let wait = |w: &World| {
        w.get::<Ragdoll>(id)
            .unwrap()
            .drive
            .as_ref()
            .unwrap()
            .posture_wait
    };
    // The Rebalance's own posture stage.
    world.run_system_once(take_posture).unwrap();
    assert_eq!(wait(&world), 40);
    world.get_mut::<PostureBranch>(id).unwrap().0 = Branch::Standing;
    for _ in 0..39 {
        world.run_system_once(take_posture).unwrap();
    }
    assert_eq!(wait(&world), 1, "the 40th tick after still waits");
    world.run_system_once(take_posture).unwrap();
    assert_eq!(wait(&world), 0, "the 41st may switch");
}

/// Lying still with its pelvis's front axis up, a supported body gets up
/// with Get Up 2: getting up, on its controller, blending 20 frames from
/// its whole physical pose into the stance; face down it is Get up.
#[test]
fn lying_still_it_gets_up_by_its_pelvis() {
    let face_up = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    let mut body = pelvis(face_up);
    body.fall();
    let (mut world, id) = posed(body, "Fall Backward", Branch::LyingStill, Condition::DOWN);
    world.run_system_once(take_posture).unwrap();
    let anim = world.get::<Animator>(id).unwrap();
    assert_eq!((anim.action.as_str(), anim.blend), ("Get Up 2", 20.0));
    assert_eq!(anim.pending, Some(Capture::Whole));
    assert_eq!(anim.then.as_deref(), Some("Idle"));
    assert_eq!(*world.get::<Condition>(id).unwrap(), Condition::GETTING_UP);
    assert!(
        world
            .get::<Ragdoll>(id)
            .unwrap()
            .drive
            .as_ref()
            .unwrap()
            .supported
    );
    assert!(world.get::<Fallen>(id).is_some());

    let (mut world, id) = posed(
        pelvis(Quat::IDENTITY),
        "Fall Forward",
        Branch::LyingStill,
        Condition::DOWN,
    );
    world.run_system_once(take_posture).unwrap();
    assert_eq!(world.get::<Animator>(id).unwrap().action, "Get up");
}

/// Sets the clip clock of `id`'s animator.
fn set_time(world: &mut World, id: Entity, time: f32) {
    world.get_mut::<Animator>(id).unwrap().time = time;
}

/// While its get-up is under 40 %, the movement stage picks it again: a
/// body that rolled face up during Get up changes to Get Up 2 (blend 20,
/// whole capture) and rises again this tick; the same pick past its first
/// quarter keeps playing; at 40 % nothing is picked.
#[test]
fn a_body_that_rolls_over_changes_its_get_up() {
    use std::f32::consts::FRAC_PI_2;
    let (mut world, id) = posed(
        pelvis(Quat::from_rotation_x(-FRAC_PI_2)),
        "Get up",
        Branch::GettingUp,
        Condition::GETTING_UP,
    );
    // 30 % through Get up (last key 80): face up past 0.2, Get Up 2.
    set_time(&mut world, id, 24.0);
    world.run_system_once(pick_again).unwrap();
    let anim = world.get::<Animator>(id).unwrap();
    assert_eq!((anim.action.as_str(), anim.blend), ("Get Up 2", 20.0));
    assert_eq!(anim.pending, Some(Capture::Whole));
    let bearing = world
        .get::<Ragdoll>(id)
        .unwrap()
        .drive
        .as_ref()
        .unwrap()
        .bearing;
    assert!(bearing.rising && bearing.getting_up);
    // Get Up 2 at 30 % (last key 120), still face up: it plays on.
    set_time(&mut world, id, 56.0);
    world.get_mut::<Animator>(id).unwrap().blend = 20.0;
    world.run_system_once(pick_again).unwrap();
    assert_eq!(world.get::<Animator>(id).unwrap().time, 56.0);
    // Rolled face down (its front axis under −0.2): at 40 % too late,
    // just under it Get up.
    let (mut world, id) = posed(
        pelvis(Quat::from_rotation_x(FRAC_PI_2)),
        "Get Up 2",
        Branch::GettingUp,
        Condition::GETTING_UP,
    );
    set_time(&mut world, id, 48.0);
    world.run_system_once(pick_again).unwrap();
    assert_eq!(world.get::<Animator>(id).unwrap().action, "Get Up 2");
    set_time(&mut world, id, 47.9);
    world.run_system_once(pick_again).unwrap();
    assert_eq!(world.get::<Animator>(id).unwrap().action, "Get up");
}

/// Getting up (by last tick's condition), its energy gains the 0.005
/// recovery and 0.01 × (1 − progress) in one double sum: 0.51 halfway
/// through Get up. Standing, the next tick adds only the 0.005.
#[test]
fn getting_up_energy_rises_faster() {
    let mut body = pelvis(Quat::IDENTITY);
    body.drive.as_mut().unwrap().energy = 0.5;
    let (mut world, id) = posed(body, "Get up", Branch::GettingUp, Condition::GETTING_UP);
    world.get_mut::<Animator>(id).unwrap().time = 40.0;
    world.run_system_once(recover_rising).unwrap();
    let energy = world
        .get::<Ragdoll>(id)
        .unwrap()
        .drive
        .as_ref()
        .unwrap()
        .energy;
    let halfway = exa_physics::body::recover_energy(0.5, 1.0, true, 0.5);
    assert_eq!(energy, halfway);
    *world.get_mut::<Condition>(id).unwrap() = Condition::NORMAL;
    world.run_system_once(recover_rising).unwrap();
    let again = world
        .get::<Ragdoll>(id)
        .unwrap()
        .drive
        .as_ref()
        .unwrap()
        .energy;
    assert_eq!(
        again,
        exa_physics::body::recover_energy(halfway, 1.0, false, 0.5)
    );
}

/// After the recovery, a shock stagger replaces the energy with the most
/// minus 0.5 and starts Rebalance. The same clock as the next shock keeps
/// the flag; a clock past it clears the flag and does not restart Rebalance.
#[test]
fn a_shock_stagger_pins_energy_after_the_recovery() {
    let mut body = pelvis(Quat::IDENTITY);
    body.drive.as_mut().unwrap().energy = 1.0;
    let (mut world, id) = posed(body, "Idle", Branch::Standing, Condition::NORMAL);
    world.get_mut::<StateFlags>(id).unwrap().0 = StateFlags::SHOCK_STAGGER;
    world.entity_mut(id).insert(ShockReady(500));
    world.insert_resource(BodyTicks(0));
    world.init_resource::<ShockHeld>();
    world.run_system_once(recover_rising).unwrap();
    world.run_system_once(pin_shock_stagger).unwrap();
    // The daze block took the shock's branch: the sampler holds the daze pose.
    assert_eq!(world.resource::<ShockHeld>().0, Some(id));
    let energy = world
        .get::<Ragdoll>(id)
        .unwrap()
        .drive
        .as_ref()
        .unwrap()
        .energy;
    let anim = world.get::<Animator>(id).unwrap();
    assert_eq!(energy.to_bits(), 0x3f00_0000);
    assert_eq!(
        (anim.action.as_str(), anim.blend, anim.looping, anim.ease),
        ("Rebalance", 0.0, true, Ease::Linear)
    );
    assert_eq!(*world.get::<Stance>(id).unwrap(), Stance::Defence);
    assert_eq!(
        world.get::<StateFlags>(id).unwrap().0,
        StateFlags::SHOCK_STAGGER
    );
    // Twelve ticks are 144 ms: still before 500, and Rebalance is not restarted.
    let started = world.get::<Animator>(id).unwrap().time;
    world.insert_resource(BodyTicks(12));
    world.get_mut::<Animator>(id).unwrap().time = 4.0;
    world.run_system_once(pin_shock_stagger).unwrap();
    assert_eq!(world.get::<Animator>(id).unwrap().time, 4.0);
    assert_eq!(started, 0.0);
    world.insert_resource(BodyTicks(42));
    world.run_system_once(pin_shock_stagger).unwrap();
    assert_eq!(world.get::<StateFlags>(id).unwrap().0, 0);
    assert_eq!(world.get::<Animator>(id).unwrap().time, 4.0);
    // The tick that clears the flag still takes the daze pose; the next
    // one, with the flag clear, does not.
    assert_eq!(world.resource::<ShockHeld>().0, Some(id));
    world.run_system_once(pin_shock_stagger).unwrap();
    assert_eq!(world.resource::<ShockHeld>().0, None);
}

/// The bearing's parry flag is the one the previous tick's attack stage
/// left: the posture stage runs before this tick's parry check, so the arm
/// writes and the clip rate of this tick read last tick's value.
#[test]
fn the_bearing_reads_last_ticks_parry_flag() {
    let body = pelvis(Quat::IDENTITY);
    let (mut world, id) = posed(body, "Idle", Branch::Standing, Condition::NORMAL);
    let parrying = |w: &World| {
        w.get::<Ragdoll>(id)
            .unwrap()
            .drive
            .as_ref()
            .unwrap()
            .bearing
            .parrying
    };
    world
        .run_system_once(crate::stages::posture::take_bearing)
        .unwrap();
    assert!(!parrying(&world));
    let mut fighter = exa_combat::Fighter::default();
    fighter.parry.active = true;
    world.entity_mut(id).insert(fighter);
    world
        .run_system_once(crate::stages::posture::take_bearing)
        .unwrap();
    assert!(parrying(&world));
}

/// The shock-stagger bit reaches the pins even beside another flag. Exactly
/// that flag alone is what makes the stance factor 1.
#[test]
fn a_shock_stagger_bit_reaches_the_pins() {
    let body = pelvis(Quat::IDENTITY);
    let (mut world, id) = posed(body, "Idle", Branch::Standing, Condition::NORMAL);
    world.get_mut::<StateFlags>(id).unwrap().0 =
        StateFlags::SHOCK_STAGGER | StateFlags::TURNS_BLOWS;
    world
        .run_system_once(crate::stages::posture::take_bearing)
        .unwrap();
    let bearing = world
        .get::<Ragdoll>(id)
        .unwrap()
        .drive
        .as_ref()
        .unwrap()
        .bearing;
    assert!(bearing.shock_stagger);
    assert!(!bearing.full_stance);

    world.get_mut::<StateFlags>(id).unwrap().0 = StateFlags::SHOCK_STAGGER;
    world
        .run_system_once(crate::stages::posture::take_bearing)
        .unwrap();
    let bearing = world
        .get::<Ragdoll>(id)
        .unwrap()
        .drive
        .as_ref()
        .unwrap()
        .bearing;
    assert!(bearing.shock_stagger);
    assert!(bearing.full_stance);
}

/// A shock-staggered body on its feet holds the daze pose: its pelvis
/// (skin bone 1) keeps the Rebalance start's capture while it is up, and
/// takes a random turn half way from the Defence stance's first key once
/// its condition is down; with no capture there is no daze pose.
#[test]
fn a_shocked_body_holds_the_daze_pose() {
    let motions = motions();
    let body = pelvis(Quat::IDENTITY);
    let mut anim = Animator::new("Default", "Rebalance");
    let mut roll = exa_combat::Roll::default();
    let daze = |anim: &Animator, condition: Condition, roll: &mut exa_combat::Roll| {
        crate::stages::sampler::daze_pose(&motions, anim, &body, Some(&condition), roll)
    };
    assert!(daze(&anim, Condition::NORMAL, &mut roll).is_none());
    let turn = Quat::from_rotation_y(0.3);
    anim.capture = Some(exa_anim::ClipPose {
        rotations: vec![turn],
        root: Vec3::ZERO,
    });
    let up = daze(&anim, Condition::NORMAL, &mut roll).unwrap();
    assert_eq!(up, vec![turn]);
    let down = daze(&anim, Condition::DOWN, &mut roll).unwrap();
    assert_eq!(down.len(), 1);
    assert_ne!(down[0], turn);
    assert!((down[0].length() - 1.0).abs() < 1e-5);
}
