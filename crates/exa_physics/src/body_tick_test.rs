//! Bodies tick in the fixed step: the same ticks whatever the frame rate,
//! blows queued for the next tick, parts drawn between their last two ticks.
use super::floor_test::{Aim, app, floor, humanoid};
use super::*;

/// Every particle of every body after each tick, bodies in spawn order.
#[derive(Resource, Default)]
struct History(Vec<Vec<[Vec3; 4]>>);

#[derive(Component)]
struct Order(usize);

fn record(mut history: ResMut<History>, bodies: Query<(&Order, &Ragdoll)>) {
    let mut bodies: Vec<_> = bodies.iter().collect();
    bodies.sort_by_key(|(o, _)| o.0);
    history.0.push(
        bodies
            .iter()
            .flat_map(|(_, b)| b.parts.iter().map(|p| p.positions))
            .collect(),
    );
}

/// A living body walking and turning on a floor far from the world's
/// origin, its pose swaying, shoved before the first tick; and a limp body
/// dropped beside it, which lands and settles. Returns the particles after
/// each of the first `ticks` ticks at `fps` frames a second.
fn run_at(fps: f32, ticks: usize) -> Vec<Vec<[Vec3; 4]>> {
    let at = Vec3::new(81.0, 0.0, 39.2);
    let mut app = app(floor(at), 1.0 / fps);
    app.init_resource::<History>()
        .add_systems(FixedUpdate, record.after(BodySystems));
    let (setup, pose) = humanoid(at);
    let local: Vec<Affine3A> = pose
        .iter()
        .map(|t| Affine3A::from_translation(-at) * *t)
        .collect();
    let walk = move |tick: u64| {
        let t = tick as f32;
        let root = Affine3A::from_translation(at + Vec3::X * 0.014 * t)
            * Affine3A::from_rotation_y(0.01 * t);
        let sway = Affine3A::from_translation(Vec3::new(0.03 * (t * 0.2).sin(), 0.0, 0.0));
        (root, local.iter().map(|l| root * sway * *l).collect())
    };
    let living = app
        .world_mut()
        .spawn((
            Ragdoll::new(&setup).with_drive(),
            Aim(Box::new(walk)),
            Order(0),
        ))
        .id();
    let (limp, _) = humanoid(at + Vec3::new(0.0, 0.5, 2.0));
    app.world_mut().spawn((Ragdoll::new(&limp), Order(1)));
    app.world_mut().write_message(Shove {
        body: living,
        bone: 8,
        velocity: Vec3::new(1.0, 0.0, -1.5),
        daze: 0.3,
        step: None,
        sag: None,
    });
    while app.world().resource::<History>().0.len() < ticks {
        app.update();
    }
    let mut history = app.world_mut().remove_resource::<History>().unwrap().0;
    history.truncate(ticks);
    history
}

/// The same ticks at 30, 60, 144 and 240 frames a second: every particle of
/// a carried, driven, shoved body and of a limp one lands, after every
/// tick, exactly where it does at any other frame rate.
#[test]
fn particles_match_at_any_frame_rate() {
    // Three seconds: the limp body has landed.
    let ticks = 3 * 83;
    let reference = run_at(60.0, ticks);
    assert_ne!(reference[0], reference[ticks - 1], "nothing moved");
    for fps in [30.0, 144.0, 240.0] {
        let other = run_at(fps, ticks);
        let first = reference.iter().zip(&other).position(|(a, b)| a != b);
        assert_eq!(
            first, None,
            "at {fps} FPS the bodies part from 60 FPS at tick"
        );
    }
}

/// The world's solver runs each phase over every body before the next; with
/// no contacts between bodies, each body ends every tick exactly where
/// ticking it alone leaves it.
#[test]
fn the_world_ticks_each_body_as_it_ticks_alone() {
    let at = Vec3::new(5.0, 0.0, 3.0);
    let level = super::tests::floor_quad(0.0, 50.0);
    let (setup, pose) = humanoid(at);
    let (dropped, _) = humanoid(at + Vec3::new(1.0, 0.4, 0.0));
    let mut living = Ragdoll::new(&setup).with_drive();
    let mut limp = Ragdoll::new(&dropped);
    let (mut living_alone, mut limp_alone) = (living.clone(), limp.clone());
    living.shove(8, Vec3::X * 2.0, 0.4);
    living_alone.shove(8, Vec3::X * 2.0, 0.4);
    for _ in 0..120 {
        living.aim(Affine3A::IDENTITY, pose.clone());
        living_alone.aim(Affine3A::IDENTITY, pose.clone());
        solve(&mut [&mut living, &mut limp], level.as_slice());
        living_alone.tick(level.as_slice());
        limp_alone.tick(level.as_slice());
        for (world, alone) in [(&living, &living_alone), (&limp, &limp_alone)] {
            for (a, b) in world.parts.iter().zip(&alone.parts) {
                assert_eq!(a.positions, b.positions);
            }
        }
    }
}

/// A blow's sag is written on a living body as it stands (0.7 here, its
/// fade comes with the next step); a limp body has none to sag.
#[test]
fn a_blow_sets_the_sag() {
    let (setup, _) = humanoid(Vec3::ZERO);
    let mut living = Ragdoll::new(&setup).with_drive();
    living.sag_from_blow(0.7);
    assert_eq!(living.drive.as_ref().unwrap().sag, 0.7);
    let mut limp = Ragdoll::new(&setup);
    limp.sag_from_blow(0.7);
    assert!(limp.drive.is_none());
}

/// A blow sent in a frame that runs no tick waits in the queue and is
/// dealt by the next tick.
#[test]
fn a_shove_in_a_frame_without_a_tick_is_dealt_by_the_next_tick() {
    let frame = 1.0 / 240.0;
    let mut app = app(Physics::default(), frame);
    let (setup, pose) = humanoid(Vec3::ZERO);
    let body = app
        .world_mut()
        .spawn((Ragdoll::new(&setup).with_drive(), Aim::still(pose)))
        .id();
    let energy = |app: &App| {
        let body = app.world().get::<Ragdoll>(body).unwrap();
        body.drive.as_ref().unwrap().energy
    };
    let ticks = |app: &App| app.world().resource::<BodyTicks>().0;
    super::floor_test::run_ticks(&mut app, 2);
    // Up to a frame after which the next one runs no tick.
    loop {
        let time = app.world().resource::<Time<Fixed>>();
        if time.overstep().as_secs_f32() + frame < time.timestep().as_secs_f32() * 0.99 {
            break;
        }
        app.update();
    }
    let before = ticks(&app);
    app.world_mut().write_message(Shove {
        body,
        bone: 8,
        velocity: Vec3::X * 2.0,
        daze: 0.5,
        step: None,
        sag: None,
    });
    app.update();
    assert_eq!(ticks(&app), before, "the frame ran a tick");
    assert_eq!(app.world().resource::<ShoveQueue>().0.len(), 1);
    assert_eq!(energy(&app), 1.0, "dealt without a tick");
    while ticks(&app) == before {
        app.update();
    }
    assert!(app.world().resource::<ShoveQueue>().0.is_empty());
    // Halved by the daze, then one tick's recovery.
    assert_eq!(energy(&app), recover_energy(0.5, 1.0, false, 0.0));
}

/// A body model root that a frame system moves and turns every frame, as
/// the AI's and combat's turns and the teleport of a body getting up do.
#[derive(Component)]
struct Moved;

fn move_roots(mut roots: Query<&mut Transform, With<Moved>>) {
    for mut t in &mut roots {
        t.translation.x += 0.1;
        t.rotate_y(0.3);
    }
}

/// Parts are drawn between where the tick before last and the last tick
/// left them: a falling body's parts, each under its drawn parent, lie
/// that fraction of the way between their last two ticks, even when a
/// frame system moves and turns their body root after the ticks.
#[test]
fn parts_are_drawn_between_their_last_two_ticks() {
    let frame = 1.0 / 144.0;
    let mut app = app(Physics::default(), frame);
    app.add_systems(Update, move_roots);
    let world = app.world_mut();
    // A body model root away from the origin, the pelvis under it and the
    // thigh under the pelvis.
    let root = world
        .spawn((
            Transform::from_xyz(3.0, 0.0, -2.0).with_rotation(Quat::from_rotation_y(0.7)),
            Moved,
        ))
        .id();
    let pelvis = world.spawn((Transform::default(), ChildOf(root))).id();
    let thigh = world.spawn((Transform::default(), ChildOf(pelvis))).id();
    let (mut setup, _) = humanoid(Vec3::new(3.0, 2.0, -2.0));
    setup.truncate(2);
    setup[0].entity = pelvis;
    setup[1].entity = thigh;
    let mut body = Ragdoll::new(&setup);
    body.set_velocity(Vec3::new(1.0, 0.5, 0.0));
    let body = world.spawn(body).id();
    let mut between = 0;
    for _ in 0..60 {
        app.update();
        let world = app.world();
        let alpha = world.resource::<Time<Fixed>>().overstep_fraction();
        let ragdoll = world.get::<Ragdoll>(body).unwrap();
        // The drawn world transform, from the local transforms up.
        let world_of = |e: Entity| {
            let mut m = Affine3A::IDENTITY;
            let mut node = Some(e);
            while let Some(n) = node {
                m = world.get::<Transform>(n).unwrap().compute_affine() * m;
                node = world.get::<ChildOf>(n).map(ChildOf::parent);
            }
            m
        };
        for (p, e) in ragdoll.parts.iter().zip([pelvis, thigh]) {
            let drawn = world_of(e);
            let (from, to) = (p.shown.0.translation, p.shown.1.translation);
            let expected = from.lerp(to, alpha);
            assert!(
                drawn.translation.abs_diff_eq(expected, 1e-4),
                "bone {} drawn at {} for {expected}",
                p.bone,
                drawn.translation
            );
            if from != to && alpha > 0.1 && alpha < 0.9 {
                // Strictly between the two ticks.
                let along =
                    (drawn.translation - from).dot(to - from) / (to - from).length_squared();
                assert!(along > 0.05 && along < 0.95, "{along}");
                between += 1;
            }
        }
    }
    assert!(between > 10, "only {between} draws fell between ticks");
}
