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

/// A living body's pose for the tests' character tick, by tick number: its
/// root's world transform and the targets it is aimed at.
#[derive(Component)]
pub(super) struct Aim(pub Box<dyn Fn(u64) -> (Affine3A, Vec<Affine3A>) + Send + Sync>);

impl Aim {
    /// The same pose every tick, the root at the origin.
    pub(super) fn still(pose: Vec<Affine3A>) -> Self {
        Self(Box::new(move |_| (Affine3A::IDENTITY, pose.clone())))
    }
}

/// The tests' character tick: every body with an [`Aim`] is aimed at its
/// pose for this tick.
fn aim_bodies(clock: Res<BodyTicks>, mut bodies: Query<(&mut Ragdoll, &Aim)>) {
    for (mut body, aim) in &mut bodies {
        let (root, targets) = (aim.0)(clock.0);
        body.aim(root, targets);
    }
}

/// An app ticking bodies as the game does, at the game's fixed rate,
/// `frame` seconds per update; bodies with an [`Aim`] are aimed every tick.
pub(super) fn app(physics: Physics, frame: f32) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, BodyPlugin))
        .insert_resource(Time::<Fixed>::from_hz(crate::STEP_HZ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            frame,
        )))
        .insert_resource(physics)
        .add_systems(FixedUpdate, aim_bodies.in_set(CharacterTicks));
    app
}

/// Runs `app` until its bodies have ticked `ticks` more times.
pub(super) fn run_ticks(app: &mut App, ticks: u64) {
    let end = app.world().resource::<BodyTicks>().0 + ticks;
    while app.world().resource::<BodyTicks>().0 < end {
        app.update();
    }
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
/// its feet at `at`, every joint a hinge about its lateral axis turned by a
/// link muscle; returns its parts and their standing world pose.
pub(super) fn humanoid(at: Vec3) -> (Vec<PartSetup>, Vec<Affine3A>) {
    let (mut setup, pose) = hinged(at);
    for s in &mut setup {
        if s.parent.is_some() {
            // 12 cm along the part, with as much slack (about 60° of swing).
            let along = s.centre.normalize_or(Vec3::NEG_Y) * 0.12;
            s.links[1] = (along, along, 0.12);
        }
    }
    (setup, pose)
}

/// The humanoid standing with its feet at `at`, its balance measured: each
/// lower leg stands on a contact at its foot, 50 cm under the knee, and the
/// torso carries two shoulder pins, the chest. Its ankles stand on the
/// floor (no sole), where its feet are anchored, and the lift reads its
/// legs 3.3 cm longer than hip to ankle, so that its target (its legs plus
/// 5 cm) puts the balance point of the lift, 1.7 cm over it, where its legs
/// are straight, its pelvis 1.1 m over its feet.
pub(super) fn standing_at(at: Vec3) -> (Ragdoll, Vec<Affine3A>) {
    let (mut setup, pose) = humanoid(at);
    for s in &mut setup {
        match s.bone {
            3 | 6 => s.foot = Some(Vec3::new(0.0, -0.5, 0.0)),
            8 => s.pins = vec![Vec3::new(0.15, 0.4, 0.0), Vec3::new(-0.15, 0.4, 0.0)],
            _ => {}
        }
    }
    let mut body = Ragdoll::new(&setup).with_drive();
    body.sole = STANDING_SOLE;
    for leg in &mut body.legs {
        leg.offset = STANDING_SOLE;
        leg.length = STANDING_LEG;
    }
    (body, pose)
}

/// The standing humanoid's sole height (m): its ankles stand on the floor.
pub(super) const STANDING_SOLE: f32 = 0.0;
/// The leg length (m) its lift reads: 1.1 − (0.05 + 1/60).
pub(super) const STANDING_LEG: f32 = 1.0 + 1.0 / 30.0;

/// The same humanoid with every joint a hinge and nothing more: limp, it
/// has no joint left free to swing (a limp joint is not damped inside its
/// slack yet).
pub(super) fn hinged(at: Vec3) -> (Vec<PartSetup>, Vec<Affine3A>) {
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
    // Each part's origin is its joint (the pelvis's its centre), as in the
    // body models.
    let origin = |i: usize| match parts[i].1 {
        Some(_) => parts[i].4,
        None => parts[i].2,
    };
    let pose: Vec<Affine3A> = (0..parts.len())
        .map(|i| Affine3A::from_translation(at + origin(i)))
        .collect();
    let setup = parts
        .iter()
        .enumerate()
        .map(|(i, &(bone, parent, centre, half, _))| PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent,
            world: pose[i],
            centre: centre - origin(i),
            half,
            origin: parent.map_or(Vec3::ZERO, |j| origin(i) - origin(j)),
            // Two points beside the joint on its lateral axis: with the
            // origin pivot, a hinge.
            links: parent.map_or(vec![], |_| {
                [0.04, -0.04]
                    .map(|x| (Vec3::X * x, Vec3::X * x, 0.0))
                    .to_vec()
            }),
            foot: None,
            pins: Vec::new(),
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
        .spawn((Ragdoll::new(&setup).with_drive(), Aim::still(pose)))
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
        if f == 50 {
            // Balance lost, as `keep_balance` decides after the stagger.
            let mut ragdoll = app.world_mut().get_mut::<Ragdoll>(body).unwrap();
            ragdoll.fall();
        }
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
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
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
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
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

/// (g) A fallen body lying far from the world's origin while its pose
/// keeps being aimed is not flung: no floor, no blow, only its link
/// muscles (it has no support, so no pose drive).
#[test]
fn a_fallen_body_far_from_the_origin_is_not_flung() {
    let at = Vec3::new(60.0, 0.0, 40.0);
    let (setup, pose) = humanoid(at);
    let mut body = Ragdoll::new(&setup).with_drive();
    let none: &[[Vec3; 3]] = &[];
    body.aim(Affine3A::IDENTITY, pose.clone());
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
    for _ in 0..40 {
        body.aim(Affine3A::IDENTITY, pose.clone());
        body.tick(none);
        for p in &body.parts {
            for i in 0..4 {
                fastest = fastest.max((p.positions[i] - p.previous[i]).length() * BODY_HZ);
            }
        }
    }
    assert!(
        fastest < 5.0,
        "a resting fallen body reached {fastest:.1} m/s"
    );
}

/// A standing body far from the world's origin turning on the spot: each
/// tick's targets turn about the body, and the pose drive turns each part
/// about its own origin, so the muscles never ask for speeds of metres per
/// tick.
#[test]
fn a_turning_pose_far_from_the_origin_is_followed_gently() {
    let at = Vec3::new(81.0, 0.0, 39.2);
    let (mut body, pose) = standing_at(at);
    let floor = super::tests::floor_quad(0.0, 100.0);
    let turn = |a: f32| {
        Affine3A::from_translation(at)
            * Affine3A::from_rotation_y(a)
            * Affine3A::from_translation(-at)
    };
    let mut fastest = 0.0f32;
    // Four seconds at 2 rad/s about the body's own vertical.
    for tick in 0..332 {
        let yaw = tick as f32 / BODY_HZ * 2.0;
        body.measure_balance(floor.as_slice());
        body.aim(
            Affine3A::IDENTITY,
            pose.iter().map(|t| turn(yaw) * *t).collect(),
        );
        body.tick(floor.as_slice());
        for p in &body.parts {
            for i in 0..4 {
                fastest = fastest.max((p.positions[i] - p.previous[i]).length() * BODY_HZ);
            }
        }
    }
    assert!(fastest < 3.0, "a particle reached {fastest:.1} m/s");
}

/// A fallen body's muscles are inside it: they cannot slow its fall. It
/// has no support, so no pose drive, and its link muscles pull its parts
/// against each other. (A drive damping the world velocity of a fallen
/// body made an undazed one fall about nine times slower than a limp one.)
#[test]
fn a_fallen_body_falls_as_fast_as_a_limp_one() {
    let (setup, pose) = humanoid(Vec3::new(0.0, 10.0, 0.0));
    let none: &[[Vec3; 3]] = &[];
    let mut living = Ragdoll::new(&setup).with_drive();
    living.aim(Affine3A::IDENTITY, pose.clone());
    living.fall();
    let mut limp = Ragdoll::new(&setup);
    // Half a second.
    for _ in 0..41 {
        living.aim(Affine3A::IDENTITY, pose.clone());
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

/// A fallen body's pose drive does nothing: one drive pass toward a pose
/// bent about the pelvis leaves the moving body's momentum and angular
/// momentum as they were. (Pulling its limbs toward a pose measured from
/// the pelvis once turned the whole body by a third of its own spin a
/// tick.)
#[test]
fn a_fallen_bodys_muscles_neither_push_nor_turn_it() {
    let (setup, pose) = humanoid(Vec3::new(5.0, 0.0, 3.0));
    let mut body = Ragdoll::new(&setup).with_drive();
    body.aim(Affine3A::IDENTITY, pose.clone());
    body.fall();
    for (i, p) in body.parts.iter_mut().enumerate() {
        for k in 0..4 {
            p.previous[k] = p.positions[k] - Vec3::new(0.003 * i as f32, -0.002, 0.001 * k as f32);
        }
    }
    let bend = Affine3A::from_rotation_translation(Quat::from_rotation_z(0.7), Vec3::X * 0.1);
    body.aim(Affine3A::IDENTITY, pose.iter().map(|t| bend * *t).collect());
    let (linear, angular) = momenta(&body);
    let Ragdoll { parts, drive, .. } = &mut body;
    drive_parts(parts, drive.as_ref().unwrap());
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
/// arms, as the fall and unconscious clips do, kicks them with its link
/// muscles alone and stays where it lies. (A pose drive pulling a fallen
/// body's parts toward a pose measured from its pelvis once rolled and
/// crawled it 1–2 m in six seconds.)
#[test]
fn a_fallen_body_kicking_its_legs_stays_where_it_lies() {
    let at = Vec3::new(20.0, 0.0, -10.0);
    let frame = 2.0 / BODY_HZ;
    let mut app = app(floor(at), frame);
    let (setup, pose) = humanoid(at);
    let mut ragdoll = Ragdoll::new(&setup).with_drive();
    ragdoll.aim(Affine3A::IDENTITY, pose.clone());
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
    let about = |c: Vec3, r: Quat| {
        Affine3A::from_translation(c) * Affine3A::from_quat(r) * Affine3A::from_translation(-c)
    };
    let (hip, shoulder) = (at + Vec3::Y * 1.0, at + Vec3::Y * 1.65);
    let bones: Vec<usize> = setup.iter().map(|s| s.bone).collect();
    // A 1.2 s cycle.
    let kicking = move |tick: u64| {
        let phase = std::f32::consts::TAU * tick as f32 / BODY_HZ / 1.2;
        let kick = 0.45 * (1.0 - phase.cos());
        let swing = 1.2 * phase.sin();
        let targets = bones
            .iter()
            .zip(&pose)
            .map(|(bone, t)| match bone {
                2 | 3 => about(hip, Quat::from_rotation_x(-kick)) * *t,
                5 | 6 => about(hip, Quat::from_rotation_x(-0.6 * kick)) * *t,
                13 => about(shoulder, Quat::from_rotation_x(swing)) * *t,
                16 => about(shoulder, Quat::from_rotation_x(-swing)) * *t,
                _ => *t,
            })
            .collect();
        (Affine3A::IDENTITY, targets)
    };
    let body = app
        .world_mut()
        .spawn((ragdoll, Aim(Box::new(kicking))))
        .id();
    let start = app.world().get::<Ragdoll>(body).unwrap().centre_of_mass().0;
    let mut furthest = 0.0f32;
    // The right hip's angle (its thigh in the pelvis, about the lateral
    // axis): the link muscles kick the legs.
    let (mut low, mut high) = (f32::MAX, f32::MIN);
    // Six seconds.
    for _ in 0..250 {
        app.update();
        let ragdoll = app.world().get::<Ragdoll>(body).unwrap();
        let centre = ragdoll.centre_of_mass().0;
        furthest = furthest.max((centre - start).xz().length());
        let pelvis = ragdoll.part_of(1).unwrap().transform();
        let thigh = ragdoll.part_of(2).unwrap().transform();
        let down = pelvis.matrix3.inverse() * (thigh.matrix3 * Vec3::NEG_Y);
        let hip = down.z.atan2(-down.y).to_degrees();
        (low, high) = (low.min(hip), high.max(hip));
    }
    assert!(
        furthest < 0.3,
        "the kicking body travelled {furthest:.2} m from where it lay"
    );
    assert!(high - low > 20.0, "the legs kicked only {:.1}°", high - low);
}
