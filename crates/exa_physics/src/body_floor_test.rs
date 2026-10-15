//! A shoved, muscle-driven body must not sink through a level floor.
//! (Included from body.rs: `#[cfg(test)] #[path = "body_floor_test.rs"] mod floor_test;`)
use super::*;
use bevy::time::TimeUpdateStrategy;
use rapier3d::prelude::PhysicsWorld;
use std::time::Duration;

/// A 40 m floor whose surface is at `at`, built by `trimesh_collider` as
/// level geometry is.
pub(super) fn floor(at: Vec3) -> Physics {
    let mesh = Plane3d::default().mesh().size(40.0, 40.0).build();
    let collider = crate::trimesh_collider(&mesh, &GlobalTransform::from_translation(at)).unwrap();
    let mut world = PhysicsWorld::new();
    world.insert_collider(collider, None);
    world.step();
    Physics(world)
}

/// An app stepping bodies with the game's `step_ragdolls`, `frame` seconds
/// per update.
pub(super) fn app(physics: Physics, frame: f32) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            frame,
        )))
        .insert_resource(physics)
        .add_message::<Shove>()
        .add_systems(Update, step_ragdolls);
    app
}

/// Height of the lowest sphere centre, and its bone.
pub(super) fn lowest(body: &Ragdoll) -> (f32, usize) {
    let mut low = (f32::MAX, 0);
    for p in &body.parts {
        let t = p.transform();
        for &(c, _) in &p.spheres {
            let y = t.transform_point3(c).y;
            if y < low.0 {
                low = (y, p.bone);
            }
        }
    }
    low
}

/// A small standing humanoid (pelvis, thighs, shins, torso, arms) with
/// its feet at `at`; returns its parts and their standing world pose.
pub(super) fn humanoid(at: Vec3) -> (Vec<PartSetup>, Vec<Affine3A>) {
    // bone, parent, centre (relative to the feet), half extents, joint
    let parts = [
        (
            1,
            None,
            Vec3::new(0.0, 1.1, 0.0),
            Vec3::new(0.15, 0.1, 0.1),
            Vec3::ZERO,
        ),
        (
            2,
            Some(0),
            Vec3::new(0.1, 0.75, 0.0),
            Vec3::new(0.07, 0.25, 0.07),
            Vec3::new(0.1, 1.0, 0.0),
        ),
        (
            3,
            Some(1),
            Vec3::new(0.1, 0.25, 0.0),
            Vec3::new(0.05, 0.25, 0.05),
            Vec3::new(0.1, 0.5, 0.0),
        ),
        (
            5,
            Some(0),
            Vec3::new(-0.1, 0.75, 0.0),
            Vec3::new(0.07, 0.25, 0.07),
            Vec3::new(-0.1, 1.0, 0.0),
        ),
        (
            6,
            Some(3),
            Vec3::new(-0.1, 0.25, 0.0),
            Vec3::new(0.05, 0.25, 0.05),
            Vec3::new(-0.1, 0.5, 0.0),
        ),
        (
            8,
            Some(0),
            Vec3::new(0.0, 1.45, 0.0),
            Vec3::new(0.18, 0.25, 0.1),
            Vec3::new(0.0, 1.2, 0.0),
        ),
        (
            13,
            Some(5),
            Vec3::new(0.3, 1.35, 0.0),
            Vec3::new(0.05, 0.3, 0.05),
            Vec3::new(0.3, 1.65, 0.0),
        ),
        (
            16,
            Some(5),
            Vec3::new(-0.3, 1.35, 0.0),
            Vec3::new(0.05, 0.3, 0.05),
            Vec3::new(-0.3, 1.65, 0.0),
        ),
    ];
    let pose: Vec<Affine3A> = parts
        .iter()
        .map(|p| Affine3A::from_translation(at + p.2))
        .collect();
    let setup = parts
        .iter()
        .enumerate()
        .map(|(i, &(bone, parent, centre, half, joint))| PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent,
            world: pose[i],
            centre: Vec3::ZERO,
            half,
            // Two points across each joint, in each part's own space.
            links: parent.map_or(vec![], |j| {
                [0.04, -0.04]
                    .map(|x| {
                        (
                            joint + Vec3::X * x - centre,
                            joint + Vec3::X * x - parts[j].2,
                            0.0,
                        )
                    })
                    .to_vec()
            }),
        })
        .collect();
    (setup, pose)
}

/// The `--shove` scenario end to end: stand, take a dazing blow to the
/// torso, lose balance and lie on a level floor where exanima01 starts the
/// player (90 m from the world's origin).
#[test]
fn a_shoved_body_does_not_sink_through_a_trimesh_floor() {
    let at = Vec3::new(81.0, 0.0, 39.2);
    // Four body ticks per frame (a 20 fps frame, the most a frame runs).
    let frame = 4.0 / BODY_HZ;
    let mut app = app(floor(at), frame);
    let (setup, pose) = humanoid(at);
    let body = app
        .world_mut()
        .spawn(Ragdoll::new(&setup).with_drive())
        .id();
    let mut low = (f32::MAX, 0, 0);
    for f in 0..400 {
        if f == 30 {
            // `--shove`: a 4 m/s blow to the torso that dazes three times.
            for velocity in [Vec3::NEG_Z * 4.0, Vec3::ZERO, Vec3::ZERO] {
                app.world_mut().write_message(Shove {
                    body,
                    bone: 8,
                    velocity,
                    daze: 0.5,
                });
            }
        }
        let mut ragdoll = app.world_mut().get_mut::<Ragdoll>(body).unwrap();
        if f == 50 {
            // Balance lost, as `keep_balance` decides after the stagger.
            ragdoll.fall();
        }
        ragdoll.aim(Affine3A::IDENTITY, pose.clone(), frame);
        app.update();
        let (y, bone) = lowest(app.world().get::<Ragdoll>(body).unwrap());
        if y < low.0 {
            low = (y, bone, f);
        }
    }
    let ragdoll = app.world().get::<Ragdoll>(body).unwrap();
    let pelvis = Vec3::from(ragdoll.part_of(1).unwrap().transform().translation);
    // Half the spheres' radius (6 cm and less): a centre at the surface is
    // a sphere half sunk.
    assert!(
        low.0 >= 0.03,
        "bone {} sphere centre at {:.3} m from the floor at frame {}",
        low.1,
        low.0,
        low.2
    );
    assert!(
        pelvis.distance(at) < 10.0,
        "the body ended {:.1} m away",
        pelvis.distance(at)
    );
}

/// (a) A sphere whose centre passes below the floor within one tick (as a
/// fast limb does) is taken back to where it first touched, not out of the
/// far side.
#[test]
fn a_sphere_crossing_the_floor_in_one_tick_is_pushed_back_up() {
    let mut app = app(floor(Vec3::ZERO), 1.0 / BODY_HZ);
    // A limp 12 cm ball 1 cm above the surface, falling at 9 m/s: its
    // centre would end the tick 4 cm under it.
    let ball = PartSetup {
        bone: 11,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(Vec3::new(0.0, 0.07, 0.0)),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.06),
        links: vec![],
    };
    let mut ragdoll = Ragdoll::new(&[ball]);
    ragdoll.set_velocity(Vec3::NEG_Y * 9.0);
    let body = app.world_mut().spawn(ragdoll).id();
    for f in 0..120 {
        app.update();
        let (y, _) = lowest(app.world().get::<Ragdoll>(body).unwrap());
        assert!(
            y > 0.0,
            "frame {f}: the ball's centre is {:.3} m below the floor",
            -y
        );
    }
    let (y, _) = lowest(app.world().get::<Ragdoll>(body).unwrap());
    assert!(
        (y - 0.06).abs() < 0.002,
        "the ball rests with its centre at {y:.4} m"
    );
}

/// Faces push from their front only: a centre that starts behind a face
/// (placed there) is not pushed by it, as in the original.
#[test]
fn a_centre_behind_a_face_is_not_pushed() {
    let mut app = app(floor(Vec3::ZERO), 1.0 / BODY_HZ);
    let ball = PartSetup {
        bone: 11,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(Vec3::new(0.0, -0.01, 0.0)),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.06),
        links: vec![],
    };
    let body = app.world_mut().spawn(Ragdoll::new(&[ball])).id();
    for _ in 0..40 {
        app.update();
    }
    let (y, _) = lowest(app.world().get::<Ragdoll>(body).unwrap());
    assert!(
        y < -0.2,
        "the ball was pushed back through the face: {y:.3}"
    );
}

/// (g) Re-aiming a fallen body far from the world's origin must not hand
/// its muscles goal speeds of tens of metres per tick: no floor, no blow,
/// only the switch to a pose measured from its own pelvis.
#[test]
fn a_fallen_body_far_from_the_origin_is_not_flung() {
    let at = Vec3::new(60.0, 0.0, 40.0);
    let (setup, pose) = humanoid(at);
    let mut body = Ragdoll::new(&setup).with_drive();
    let none: &[[Vec3; 3]] = &[];
    let frame = 2.0 / BODY_HZ;
    body.aim(Affine3A::IDENTITY, pose.clone(), frame);
    // Lying on its back: the body turned a quarter turn from the pose.
    let lie = Affine3A::from_translation(at + Vec3::Y * 0.2)
        * Affine3A::from_rotation_x(-std::f32::consts::FRAC_PI_2)
        * Affine3A::from_translation(-(at + Vec3::Y * 1.1));
    for p in &mut body.parts {
        for i in 0..4 {
            p.positions[i] = lie.transform_point3(p.positions[i]);
            p.previous[i] = p.positions[i];
        }
    }
    body.fall();
    let mut fastest = 0.0f32;
    for _ in 0..20 {
        body.aim(Affine3A::IDENTITY, pose.clone(), frame);
        for _ in 0..2 {
            body.tick(none);
            for p in &body.parts {
                for i in 0..4 {
                    fastest = fastest.max((p.positions[i] - p.previous[i]).length() * BODY_HZ);
                }
            }
        }
    }
    assert!(
        fastest < 5.0,
        "a resting fallen body reached {fastest:.1} m/s"
    );
}

/// A standing body far from the world's origin turning on the spot: each
/// part's target turns about the part (the aim's step, its application and
/// the drive's `before` must agree), so the muscles never ask for speeds of
/// metres per tick.
#[test]
fn a_turning_pose_far_from_the_origin_is_followed_gently() {
    let at = Vec3::new(81.0, 0.0, 39.2);
    let (setup, pose) = humanoid(at);
    let mut body = Ragdoll::new(&setup).with_drive();
    let none: &[[Vec3; 3]] = &[];
    let frame = 1.0 / 60.0;
    let turn = |a: f32| {
        Affine3A::from_translation(at)
            * Affine3A::from_rotation_y(a)
            * Affine3A::from_translation(-at)
    };
    let mut clock = 0.0;
    let mut fastest = 0.0f32;
    for f in 0..240 {
        // 2 rad/s about the body's own vertical.
        let yaw = f as f32 * frame * 2.0;
        body.aim(
            Affine3A::IDENTITY,
            pose.iter().map(|t| turn(yaw) * *t).collect(),
            frame,
        );
        clock += frame;
        while clock >= 1.0 / BODY_HZ {
            clock -= 1.0 / BODY_HZ;
            body.tick(none);
            for p in &body.parts {
                for i in 0..4 {
                    fastest = fastest.max((p.positions[i] - p.previous[i]).length() * BODY_HZ);
                }
            }
        }
    }
    assert!(fastest < 3.0, "a particle reached {fastest:.1} m/s");
}

/// A fallen body's muscles are inside it: they cannot slow its fall. With
/// the goal velocity of a fallen body's targets set to zero, its damping
/// (2·√gain, up to 0.29 per tick at full energy) acts against the world, and
/// an undazed fallen body falls about nine times slower than a limp one.
#[test]
fn a_fallen_body_falls_as_fast_as_a_limp_one() {
    let (setup, pose) = humanoid(Vec3::new(0.0, 10.0, 0.0));
    let none: &[[Vec3; 3]] = &[];
    let mut living = Ragdoll::new(&setup).with_drive();
    living.aim(Affine3A::IDENTITY, pose.clone(), 1.0 / BODY_HZ);
    living.fall();
    let mut limp = Ragdoll::new(&setup);
    // Half a second.
    for _ in 0..41 {
        living.aim(Affine3A::IDENTITY, pose.clone(), 1.0 / BODY_HZ);
        living.tick(none);
        limp.tick(none);
    }
    let (_, fallen) = living.centre_of_mass();
    let (_, free) = limp.centre_of_mass();
    assert!(
        fallen.y < 0.8 * free.y,
        "the fallen body falls at {:.2} m/s, a limp one at {:.2} m/s",
        -fallen.y,
        -free.y
    );
}

/// Momentum and angular momentum (about the centre of mass) of a body's
/// particles, per tick.
fn momenta(body: &Ragdoll) -> (Vec3, Vec3) {
    let (centre, _) = body.centre_of_mass();
    let (mut linear, mut angular) = (Vec3::ZERO, Vec3::ZERO);
    for p in &body.parts {
        let m = 0.25 / p.inverse_mass;
        for k in 0..4 {
            let v = p.positions[k] - p.previous[k];
            linear += v * m;
            angular += (p.positions[k] - centre).cross(v) * m;
        }
    }
    (linear, angular)
}

/// A fallen body's muscles change its pose only: one drive pass towards a
/// pose bent about the pelvis leaves the moving body's momentum and angular
/// momentum as they were. Pulling the limbs alone towards a pose measured
/// from the pelvis turned the whole body by a third of its own spin a tick.
#[test]
fn a_fallen_bodys_muscles_neither_push_nor_turn_it() {
    let (setup, pose) = humanoid(Vec3::new(5.0, 0.0, 3.0));
    let mut body = Ragdoll::new(&setup).with_drive();
    body.aim(Affine3A::IDENTITY, pose.clone(), 1.0 / BODY_HZ);
    body.fall();
    for (i, p) in body.parts.iter_mut().enumerate() {
        for k in 0..4 {
            p.previous[k] = p.positions[k] - Vec3::new(0.003 * i as f32, -0.002, 0.001 * k as f32);
        }
    }
    let bend = Affine3A::from_rotation_translation(Quat::from_rotation_z(0.7), Vec3::X * 0.1);
    body.aim(
        Affine3A::IDENTITY,
        pose.iter().map(|t| bend * *t).collect(),
        1.0 / BODY_HZ,
    );
    let (linear, angular) = momenta(&body);
    let Ragdoll { parts, drive, .. } = &mut body;
    drive_parts(parts, drive.as_mut().unwrap());
    let (pushed, turned) = momenta(&body);
    assert!(
        (pushed - linear).length() < 1e-5,
        "the muscles pushed the body by {}",
        pushed - linear
    );
    assert!(
        (turned - angular).length() < 1e-5 * angular.length().max(1.0),
        "the muscles turned the body by {} (its spin {angular})",
        turned - angular
    );
}

/// A fallen body lying on its back whose pose kicks its legs and swings its
/// arms, as the fall and unconscious clips do, stays where it lies: with the
/// muscles' net turn left in, it rolled and crawled 1–2 m in six seconds.
#[test]
fn a_fallen_body_kicking_its_legs_stays_where_it_lies() {
    let at = Vec3::new(20.0, 0.0, -10.0);
    let frame = 2.0 / BODY_HZ;
    let mut app = app(floor(at), frame);
    let (setup, pose) = humanoid(at);
    let mut ragdoll = Ragdoll::new(&setup).with_drive();
    ragdoll.aim(Affine3A::IDENTITY, pose.clone(), frame);
    let lie = Affine3A::from_translation(at + Vec3::Y * 0.15)
        * Affine3A::from_rotation_x(-std::f32::consts::FRAC_PI_2)
        * Affine3A::from_translation(-(at + Vec3::Y * 1.1));
    for p in &mut ragdoll.parts {
        for i in 0..4 {
            p.positions[i] = lie.transform_point3(p.positions[i]);
            p.previous[i] = p.positions[i];
        }
    }
    ragdoll.fall();
    let body = app.world_mut().spawn(ragdoll).id();
    let about = |c: Vec3, r: Quat| {
        Affine3A::from_translation(c) * Affine3A::from_quat(r) * Affine3A::from_translation(-c)
    };
    let (hip, shoulder) = (at + Vec3::Y * 1.0, at + Vec3::Y * 1.65);
    let start = app.world().get::<Ragdoll>(body).unwrap().centre_of_mass().0;
    let mut furthest = 0.0f32;
    // Six seconds, a 1.2 s cycle.
    for f in 0..250 {
        let phase = std::f32::consts::TAU * f as f32 * frame / 1.2;
        let kick = 0.45 * (1.0 - phase.cos());
        let swing = 1.2 * phase.sin();
        let kicking: Vec<Affine3A> = setup
            .iter()
            .zip(&pose)
            .map(|(s, t)| match s.bone {
                2 | 3 => about(hip, Quat::from_rotation_x(-kick)) * *t,
                5 | 6 => about(hip, Quat::from_rotation_x(-0.6 * kick)) * *t,
                13 => about(shoulder, Quat::from_rotation_x(swing)) * *t,
                16 => about(shoulder, Quat::from_rotation_x(-swing)) * *t,
                _ => *t,
            })
            .collect();
        let mut ragdoll = app.world_mut().get_mut::<Ragdoll>(body).unwrap();
        ragdoll.aim(Affine3A::IDENTITY, kicking, frame);
        app.update();
        let centre = app.world().get::<Ragdoll>(body).unwrap().centre_of_mass().0;
        furthest = furthest.max((centre - start).xz().length());
    }
    assert!(
        furthest < 0.3,
        "the kicking body travelled {furthest:.2} m from where it lay"
    );
}
