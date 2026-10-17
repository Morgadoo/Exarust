//! The fall check and the kill plane in the fixed step: who runs them, the
//! timelines on the game clock and what dying out of the world takes
//! (`docs/formats/RSQ.md`, Falling to death).
use bevy::ecs::system::RunSystemOnce;
use bevy::math::Affine3A;
use exa_combat::fall::FallStage;
use exa_physics::body::{Bearing, PartSetup, RigidPart};

use super::*;

const NONE: &[[Vec3; 3]] = &[];

fn part(bone: usize, parent: Option<usize>, origin: Vec3, world: Vec3) -> PartSetup {
    let leg = matches!(bone, 2 | 3 | 5 | 6);
    PartSetup {
        bone,
        entity: Entity::PLACEHOLDER,
        parent,
        world: Affine3A::from_translation(world),
        centre: if leg {
            Vec3::new(0.0, -0.22, 0.0)
        } else {
            Vec3::ZERO
        },
        half: if leg {
            Vec3::new(0.07, 0.22, 0.07)
        } else {
            Vec3::new(0.15, 0.1, 0.1)
        },
        origin,
        links: vec![],
        foot: matches!(bone, 3 | 6).then(|| Vec3::new(0.0016, -0.4897, 0.024)),
        pins: if bone == 9 {
            vec![
                Vec3::new(0.104, 0.0965, 0.0125),
                Vec3::new(-0.104, 0.0965, 0.0125),
            ]
        } else {
            vec![]
        },
    }
}

/// A living biped (pelvis 1.03 m up, two legs with foot contacts, a thorax
/// with the chest pins), off its feet, that has sunk 8 cm a tick for 20
/// ticks over nothing: its chest's slow velocity is past 5 cm a tick and no
/// ground is within reach under it.
fn falling_biped() -> Ragdoll {
    let hip = |x: f32| Vec3::new(x, -0.085, 0.035);
    let knee = Vec3::new(0.0, -0.432, 0.0);
    let setup = [
        part(1, None, Vec3::ZERO, Vec3::new(0.0, 1.03, 0.0)),
        part(2, Some(0), hip(-0.09), Vec3::new(-0.09, 0.945, 0.035)),
        part(3, Some(1), knee, Vec3::new(-0.09, 0.513, 0.035)),
        part(5, Some(0), hip(0.09), Vec3::new(0.09, 0.945, 0.035)),
        part(6, Some(3), knee, Vec3::new(0.09, 0.513, 0.035)),
        part(
            9,
            Some(0),
            Vec3::new(0.0, 0.25, 0.0),
            Vec3::new(0.0, 1.28, 0.0),
        ),
    ];
    let mut body = Ragdoll::new(&setup).with_drive();
    body.fall();
    body.set_bearing(Bearing {
        off_balance: true,
        carried: false,
        ..Bearing::default()
    });
    for _ in 0..20 {
        for p in &mut body.parts {
            for x in &mut p.positions {
                x.y -= 0.08;
            }
        }
        body.measure_balance(NONE);
    }
    let b = body.drive.as_ref().unwrap().balance();
    assert!(b.slow_velocity().y < -0.05, "{}", b.slow_velocity().y);
    assert_eq!(b.ground(), Some(-100.0));
    body
}

/// A world with a falling character in `branch` (the player's when
/// `player`), under its root; returns the world, the root and the body.
fn falling(branch: Branch, player: bool) -> (World, Entity, Entity) {
    let mut world = World::new();
    world.init_resource::<BodyTicks>();
    world.init_resource::<Time<Virtual>>();
    world.init_resource::<DeathFade>();
    let root = world.spawn(Transform::default()).id();
    let body = world
        .spawn((
            Fighter::default(),
            falling_biped(),
            PostureBranch(branch),
            ChildOf(root),
        ))
        .id();
    if player {
        world.entity_mut(body).insert(PlayerBody::new(0.0));
    }
    (world, root, body)
}

/// Runs the fall check at body tick `tick` (12 ms each).
fn check_at(world: &mut World, tick: u64) {
    world.resource_mut::<BodyTicks>().0 = tick;
    world.run_system_once(fall_to_death).unwrap();
}

fn stage(world: &World, body: Entity) -> FallStage {
    world.get::<FallDeath>(body).unwrap().stage
}

/// Another character, falling in the falls branch over nothing, is lost
/// on the first check and dies on the 251st tick after (3,012 ms): dead,
/// no health left, and its root taken out with everything under it.
#[test]
fn a_character_dies_3_seconds_after_it_is_lost() {
    let (mut world, root, body) = falling(Branch::Falling, false);
    check_at(&mut world, 100);
    assert_eq!(stage(&world, body), FallStage::Lost);
    check_at(&mut world, 350);
    assert_eq!(stage(&world, body), FallStage::Lost);
    assert_eq!(*world.get::<Condition>(body).unwrap(), Condition::NORMAL);
    check_at(&mut world, 351);
    assert!(world.get_entity(root).is_err() && world.get_entity(body).is_err());
}

/// The player hears the pit and its screen starts to fade on the 84th tick
/// after it is lost, and dies on the 334th (4,008 ms), taken out too.
#[test]
fn the_player_fades_then_dies_4_seconds_after() {
    let (mut world, root, body) = falling(Branch::Falling, true);
    check_at(&mut world, 0);
    check_at(&mut world, 83);
    assert_eq!(world.resource::<DeathFade>().started, None);
    check_at(&mut world, 84);
    assert_eq!(stage(&world, body), FallStage::Fading);
    assert_eq!(world.resource::<DeathFade>().started, Some(0.0));
    check_at(&mut world, 333);
    let fighter = world.get::<Fighter>(body).unwrap();
    assert!(fighter.health.yellow > 0.0);
    check_at(&mut world, 334);
    assert!(world.get_entity(root).is_err());
}

/// The check runs only in the falls branch, and only on a living body.
#[test]
fn only_the_falls_branch_checks() {
    let (mut world, _, body) = falling(Branch::LyingStill, false);
    check_at(&mut world, 0);
    assert_eq!(stage(&world, body), FallStage::Steady);
    let (mut world, _, body) = falling(Branch::Falling, false);
    world.get_mut::<Ragdoll>(body).unwrap().go_limp();
    check_at(&mut world, 0);
    assert_eq!(stage(&world, body), FallStage::Steady);
}

/// A body (a crate's tetrahedron) dropped over nothing until its root is
/// below −50 m.
fn fallen_out() -> Ragdoll {
    let mut body = Ragdoll::rigid(&[RigidPart {
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::IDENTITY,
        rest: [
            Vec3::new(-0.2, 0.0, -0.2),
            Vec3::new(0.2, 0.0, -0.2),
            Vec3::new(0.0, 0.0, 0.2),
            Vec3::new(0.0, 0.4, 0.0),
        ],
        point_masses: [4.0; 4],
        mobility: 1.0,
        spheres: vec![(Vec3::new(0.0, 0.2, 0.0), 0.2)],
        origin: Vec3::ZERO,
        links: Vec::new(),
    }]);
    body.wake();
    for _ in 0..2000 {
        body.tick(NONE);
        if body.fell_out() {
            return body;
        }
    }
    panic!("never fell out");
}

/// The kill plane takes a dead character's body that fell below −50 m
/// once: no health left and, but for the player's, out of the world. It
/// leaves the living (the original checks only bodies on the plain step)
/// and a body already lost to its fall.
#[test]
fn the_kill_plane() {
    let spawn = |world: &mut World, condition: Condition, fall: FallDeath, player: bool| {
        let root = world.spawn(Transform::default()).id();
        let body = world
            .spawn((
                Fighter::default(),
                fallen_out(),
                condition,
                fall,
                ChildOf(root),
            ))
            .id();
        if player {
            world.entity_mut(body).insert(PlayerBody::new(0.0));
        }
        (root, body)
    };
    let mut world = World::new();
    let corpse = spawn(&mut world, Condition::DEAD, FallDeath::default(), false);
    let player = spawn(&mut world, Condition::DEAD, FallDeath::default(), true);
    let living = spawn(&mut world, Condition::NORMAL, FallDeath::default(), false);
    let lost = FallDeath {
        stage: FallStage::Lost,
        out: true,
        ..FallDeath::default()
    };
    let gone = spawn(&mut world, Condition::DEAD, lost, false);
    world.run_system_once(fall_out_of_world).unwrap();
    assert!(world.get_entity(corpse.0).is_err());
    let health = world.get::<Fighter>(player.1).unwrap().health;
    assert_eq!((health.yellow, health.red), (0.0, 0.0));
    assert!(world.get::<FallDeath>(player.1).unwrap().out);
    for (root, body) in [living, gone] {
        assert!(world.get_entity(root).is_ok());
        assert!(world.get::<Fighter>(body).unwrap().health.red > 0.0);
    }
    // Once out, the player's body is not taken again.
    world.get_mut::<Fighter>(player.1).unwrap().health.red = 0.1;
    world.run_system_once(fall_out_of_world).unwrap();
    assert_eq!(world.get::<Fighter>(player.1).unwrap().health.red, 0.1);
}

/// The installed `Sound.rpk` holds the pit sound and the nine human voices'
/// six heavy groans the scream picks from (skipped without `EXANIMA_DIR`).
#[test]
fn the_installed_fall_sounds() {
    let Some(dir) = exa_assets::locate_install(None) else {
        return;
    };
    let data = exa_assets::GameData::open(&dir).unwrap();
    let sounds = data.pack(exa_assets::PackKind::Sound).unwrap();
    assert!(sounds.get(PIT_SOUND).is_some());
    for voice in 1..=VOICES {
        for groan in 1..=GROANS {
            let name = format!("vhm{voice:02}_groanh{groan:02}");
            assert!(sounds.get(&name).is_some(), "{name}");
        }
    }
    // No tenth voice, no seventh groan.
    assert!(sounds.get("vhm10_groanh01").is_none());
    assert!(sounds.get("vhm01_groanh07").is_none());
}
