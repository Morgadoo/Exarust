//! Joints of the physical body: the pivot at every part's origin, where the
//! skeleton's links meet the parent, the parts' shares of a correction,
//! leg limits, smoothing, the pivot and limit laws and the solving order.
//! (Included from body.rs: `#[cfg(test)] #[path = "body_joint_test.rs"] mod joint_test;`)
use super::floor_test::{Aim, app, floor, humanoid, standing_at};
use super::*;

const NONE: &[[Vec3; 3]] = &[];

/// A part's setup with a box of `half` around `centre` in its space.
fn part(
    bone: usize,
    parent: Option<usize>,
    world: Affine3A,
    centre: Vec3,
    half: Vec3,
) -> PartSetup {
    PartSetup {
        bone,
        entity: Entity::PLACEHOLDER,
        parent,
        world,
        centre,
        half,
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
    }
}

fn rest_points(body: &Ragdoll, l: &Link) -> (Vec3, Vec3) {
    body.link_points(l)
}

fn links_of(body: &Ragdoll, i: usize) -> &[Link] {
    body.links_of(i)
}

/// A pelvis, a femur with two stored links, a lower leg with foot contacts
/// (a pivot, a 12.7 cm limit) and a hand with three, its rest pose turned a
/// quarter about z under the pelvis.
fn leg_and_hand() -> Ragdoll {
    let cm = |x: f32, y: f32, z: f32| Vec3::new(x, y, z) * 0.01;
    let hip = cm(-9.07, -8.46, 3.5);
    let knee = cm(-0.3, -43.19, -1.09);
    let wrist = cm(-20.0, 10.0, 0.0);
    let pelvis = Affine3A::from_translation(Vec3::Y);
    let femur = pelvis * Affine3A::from_translation(hip);
    let shin = femur * Affine3A::from_translation(knee);
    let hand = pelvis
        * Affine3A::from_translation(wrist)
        * Affine3A::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let long = Vec3::new(0.06, 0.22, 0.06);
    let mut setup = vec![
        part(1, None, pelvis, Vec3::ZERO, Vec3::new(0.15, 0.1, 0.1)),
        part(2, Some(0), femur, cm(0.0, -22.0, 0.0), long),
        part(3, Some(1), shin, cm(0.0, -22.0, 0.0), long),
        part(
            17,
            Some(0),
            hand,
            cm(-8.0, 0.0, 0.0),
            Vec3::new(0.08, 0.03, 0.05),
        ),
    ];
    setup[1].origin = hip;
    setup[1].links = vec![
        (cm(0.0, -12.49, 0.0), cm(2.65, -9.88, 7.16), 0.14879),
        (cm(8.32, 0.0, 0.0), cm(8.04, -2.15, 0.0), 0.03029),
    ];
    setup[2].origin = knee;
    setup[2].links = vec![
        (cm(8.32, 0.0, 0.0), cm(8.32, 0.21, 0.26), 0.0),
        (cm(0.0, -12.49, -2.08), cm(0.44, -2.31, -12.44), 0.12657),
    ];
    setup[2].foot = Some(cm(0.16, -48.97, 2.4));
    setup[3].origin = wrist;
    setup[3].links = vec![
        (cm(0.0, 0.0, 8.32), cm(-1.6, 2.74, 7.7), 0.08323),
        (cm(-10.4, 1.57, -0.08), cm(-10.28, -1.48, -1.7), 0.07368),
        (cm(0.0, 0.0, -8.32), cm(1.6, -2.74, -7.7), 0.08323),
    ];
    Ragdoll::new(&setup)
}

/// A pelvis and a femur with its two stored links, both inside their slack
/// as built (8.07 and 2.17 cm apart).
fn pelvis_and_femur() -> Ragdoll {
    let cm = |x: f32, y: f32, z: f32| Vec3::new(x, y, z) * 0.01;
    let hip = cm(-9.07, -8.46, 3.5);
    let pelvis = Affine3A::from_translation(Vec3::Y);
    let mut setup = vec![
        part(1, None, pelvis, Vec3::ZERO, Vec3::new(0.15, 0.1, 0.1)),
        part(
            2,
            Some(0),
            pelvis * Affine3A::from_translation(hip),
            cm(0.0, -22.0, 0.0),
            Vec3::new(0.06, 0.22, 0.06),
        ),
    ];
    setup[1].origin = hip;
    setup[1].links = vec![
        (cm(0.0, -12.49, 0.0), cm(2.65, -9.88, 7.16), 0.14879),
        (cm(8.32, 0.0, 0.0), cm(8.04, -2.15, 0.0), 0.03029),
    ];
    Ragdoll::new(&setup)
}

/// Every part with a parent takes the skeleton's links and then a pivot from
/// its origin to its place in the parent; the parent's link points are the
/// stored ones plus that place, never turned by the part's rest rotation.
#[test]
fn every_part_is_pinned_at_its_origin() {
    let body = leg_and_hand();
    assert!(links_of(&body, 0).is_empty(), "the pelvis has no parent");
    let counts: Vec<usize> = (1..4).map(|i| links_of(&body, i).len()).collect();
    assert_eq!(counts, [3, 3, 4], "femur, lower leg, hand");
    for (i, origin) in [
        (1, Vec3::new(-0.0907, -0.0846, 0.035)),
        (2, Vec3::new(-0.003, -0.4319, -0.0109)),
    ] {
        let pivot = links_of(&body, i).last().unwrap();
        assert!(pivot.origin && pivot.slack == 0.0 && pivot.strength == 1.0);
        let (on_part, on_parent) = rest_points(&body, pivot);
        assert!(on_part.length() < 1e-5, "{on_part}");
        assert!(on_parent.abs_diff_eq(origin, 1e-5), "{on_parent}");
    }
    // The hand's rest pose is turned a quarter, which adds nothing.
    let wrist = Vec3::new(-0.2, 0.1, 0.0);
    let (on_part, on_parent) = rest_points(&body, &links_of(&body, 3)[0]);
    assert!(on_part.abs_diff_eq(Vec3::new(0.0, 0.0, 0.0832), 1e-5));
    assert!(
        on_parent.abs_diff_eq(Vec3::new(-0.016, 0.0274, 0.077) + wrist, 1e-5),
        "{on_parent}"
    );
    assert!(links_of(&body, 3).iter().filter(|l| l.origin).count() == 1);
}

/// A correction is shared by the parts' mobilities, m/(m + m_parent): the
/// knee moves the lower leg 0.588 and the femur 0.412 of it, the hip the
/// femur 0.609 and the pelvis 0.391.
#[test]
fn shares_follow_the_parts_mobility() {
    let body = leg_and_hand();
    let shares = |i: usize| links_of(&body, i)[0].shares;
    let close =
        |(a, b): (f32, f32), (x, y): (f32, f32)| (a - x).abs() < 1e-3 && (b - y).abs() < 1e-3;
    assert!(close(shares(2), (0.588, 0.412)), "{:?}", shares(2));
    assert!(close(shares(1), (0.609, 0.391)), "{:?}", shares(1));
    for l in &body.links {
        assert!((l.shares.0 + l.shares.1 - 1.0).abs() < 1e-6);
    }
}

/// A lower leg's link with over 10 cm of slack limits the leg at a fifth of
/// its strength while the body lives, its femur's first long link is the
/// femur's limit, and going limp restores full strength. Each link keeps the
/// original's smoothing for its point's distance from the part's origin.
#[test]
fn leg_limits_and_smoothing() {
    let mut body = leg_and_hand().with_drive();
    let shin_limit = body.parts[2].leg_limit.unwrap();
    let femur_limit = body.parts[1].leg_limit.unwrap();
    assert_eq!(shin_limit, body.links_of[2].start + 1);
    assert_eq!(femur_limit, body.links_of[1].start);
    assert_eq!(body.links[shin_limit].strength, 0.2);
    assert_eq!(body.links[femur_limit].strength, 1.0);
    assert!(body.parts[0].leg_limit.is_none() && body.parts[3].leg_limit.is_none());
    let smoothing: Vec<f32> = links_of(&body, 1).iter().map(|l| l.smoothing).collect();
    // 12.49 cm, 8.32 cm, the origin.
    for (got, want) in smoothing.iter().zip([0.755, 0.914, 0.0]) {
        assert!((got - want).abs() < 1e-3, "{smoothing:?}");
    }
    // 10.52 cm from the hand's origin.
    assert!((links_of(&body, 3)[1].smoothing - 0.836).abs() < 1e-3);
    // Near the origin the formula passes 1 and is capped.
    assert_eq!(setup_smoothing(Vec3::new(0.01, 0.0, 0.0)), 0.99);
    assert_eq!(setup_smoothing(Vec3::new(0.003, 0.0, 0.0)), 0.0);
    body.go_limp();
    assert!(body.drive.is_none());
    assert_eq!(body.links[shin_limit].strength, 1.0);
}

/// The original's laws. A living pivot closes d/(1 + 0.001|d|) of its gap
/// (cm) and a living slack link takes back its excess, cut to 80 cm; a limp
/// pivot closes strength/(1 + 0.001|d|²), a limp slack link strength times
/// its excess.
#[test]
fn pivots_and_limits_correct_as_the_original() {
    let body = leg_and_hand();
    let (mut pivot, mut limit) = (
        links_of(&body, 2).last().unwrap().clone(),
        links_of(&body, 2)[1].clone(),
    );
    let gap = Vec3::new(0.06, 0.0, 0.08); // 10 cm
    let close = |a: Option<Vec3>, b: Vec3| a.is_some_and(|a| a.abs_diff_eq(b, 1e-6));
    assert!(close(pivot.living(gap), gap / 1.01));
    assert!(close(pivot.limp(gap), gap / 1.1));
    assert_eq!(pivot.living(Vec3::splat(1e-7)), None);
    // 12.657 cm of slack (strength 0.2): nothing inside it, the excess
    // beyond (full in a living body, by strength in a limp one).
    assert_eq!(limit.living(gap), None);
    let far = Vec3::new(0.0, -0.2, 0.0);
    assert!(close(
        limit.living(far),
        Vec3::new(0.0, -0.2 + 0.12657, 0.0)
    ));
    assert!(close(
        limit.limp(far),
        Vec3::new(0.0, (-0.2 + 0.12657) * 0.2, 0.0)
    ));
    let torn = Vec3::new(3.0, 0.0, 0.0);
    assert!(close(limit.living(torn), Vec3::X * MAX_CORRECTION));
    // A pivot torn far apart is corrected in full.
    assert!(close(pivot.living(torn), torn / (1.0 + 0.3)));
}

/// A limp body is solved parents first, depth first; a living one children
/// first.
#[test]
fn bodies_are_solved_in_their_tree_order() {
    let at = Affine3A::IDENTITY;
    let half = Vec3::splat(0.05);
    // 0 ─ 1 ─ 2, 0 ─ 3 ─ 4, given out of order.
    let setup = vec![
        part(1, None, at, Vec3::ZERO, half),
        part(8, Some(0), at, Vec3::ZERO, half),
        part(9, Some(1), at, Vec3::ZERO, half),
        part(2, Some(0), at, Vec3::ZERO, half),
        part(3, Some(3), at, Vec3::ZERO, half),
    ];
    let body = Ragdoll::new(&setup);
    assert_eq!(body.outward, [0, 1, 2, 3, 4]);
    assert_eq!(body.inward, [2, 1, 4, 3, 0]);
}

/// Distance between each hip (the femurs' origins) and its socket on the
/// pelvis.
fn hip_gaps(body: &Ragdoll) -> f32 {
    let pelvis = body.part_of(1).unwrap().transform();
    [
        (2, Vec3::new(0.1, -0.1, 0.0)),
        (5, Vec3::new(-0.1, -0.1, 0.0)),
    ]
    .iter()
    .map(|&(bone, socket)| {
        let femur = Vec3::from(body.part_of(bone).unwrap().transform().translation);
        femur.distance(pelvis.transform_point3(socket))
    })
    .fold(0.0, f32::max)
}

/// The hips stay in their sockets: a 4 m/s blow to the torso of a body
/// standing on its feet leaves each femur's origin within 2 mm of its
/// place on the pelvis after every tick, as it does when the body lies limp.
#[test]
fn the_hips_stay_in_their_sockets_under_a_shove() {
    let at = Vec3::new(30.0, 0.0, -12.0);
    let mut app = app(floor(at), 1.0 / BODY_HZ);
    let (setup, _) = humanoid(at);
    let (standing, pose) = standing_at(at);
    let body = app.world_mut().spawn((standing, Aim::still(pose))).id();
    let mut limp = Ragdoll::new(&setup);
    let mut worst = (0.0f32, 0.0f32);
    for f in 0..250 {
        if f == 20 {
            app.world_mut().write_message(Shove {
                body,
                bone: 8,
                velocity: Vec3::NEG_Z * 4.0,
                daze: 0.5,
                step: None,
                sag: None,
            });
            limp.shove(8, Vec3::NEG_Z * 4.0, 0.0);
        }
        app.update();
        limp.tick(NONE);
        let living = app.world().get::<Ragdoll>(body).unwrap();
        worst = (worst.0.max(hip_gaps(living)), worst.1.max(hip_gaps(&limp)));
    }
    assert!(
        worst.0 < 0.002,
        "a living hip left its socket by {:.1} mm",
        worst.0 * 1000.0
    );
    assert!(
        worst.1 < 0.002,
        "a limp hip left its socket by {:.1} mm",
        worst.1 * 1000.0
    );
}

/// The knee is a hinge: its origin pivot and a pivot on its lateral axis let
/// a lower leg struck sideways and forward turn about that axis only. A limp
/// knee is damped inside its range, so it is struck three times as hard and
/// bends less.
#[test]
fn the_knee_bends_about_its_lateral_axis() {
    let femur = Affine3A::from_translation(Vec3::new(0.0, 1.0, 0.0));
    let knee = Vec3::new(0.0, -0.43, 0.0);
    let long = Vec3::new(0.06, 0.22, 0.06);
    let mut setup = vec![
        part(2, None, femur, Vec3::new(0.0, -0.22, 0.0), long),
        part(
            3,
            Some(0),
            femur * Affine3A::from_translation(knee),
            Vec3::new(0.0, -0.22, 0.0),
            long,
        ),
    ];
    setup[1].origin = knee;
    setup[1].links = vec![
        (Vec3::new(0.083, 0.0, 0.0), Vec3::new(0.083, 0.0, 0.0), 0.0),
        (
            Vec3::new(0.0, -0.125, 0.0),
            Vec3::new(0.0, -0.125, 0.0),
            0.12,
        ),
    ];
    // How hard the lower leg is struck, how far it must bend (°) and how
    // near the knee's plane it must stay.
    for (living, blow, least, most) in [(false, 3.0, 5.0, 2.5), (true, 1.0, 10.0, 5.0)] {
        let mut body = Ragdoll::new(&setup);
        if living {
            body = body.with_drive();
            body.fall();
        }
        body.shove(3, Vec3::new(2.0, 0.0, 3.0) * blow, 0.0);
        let mut bent = 0.0f32;
        let mut off = 0.0f32;
        for _ in 0..83 {
            body.tick(NONE);
            let f = body.parts[0].transform();
            let s = body.parts[1].transform();
            // The lower leg's axis in the femur's frame.
            let axis = f.matrix3.inverse() * (s.matrix3 * Vec3::NEG_Y);
            off = off.max(axis.x.abs().asin().to_degrees());
            bent = bent.max(axis.z.atan2(-axis.y).abs().to_degrees());
        }
        assert!(
            bent > least,
            "the knee barely bent: {bent:.1}° (living {living})"
        );
        assert!(
            off < most,
            "the lower leg left the knee's plane by {off:.1}° (living {living})"
        );
    }
}

/// Inside its slack a limp link is a damper on its length. Its smoothed
/// length L follows the gap, L ← s·L + (1 − s)·|d|, and each pass takes the
/// points back to the L they had. When each pass closes its whole
/// correction, the length and L meet within the tick, and of the change
/// inertia made only (1 − s)/(2 − s) survives: 0.20 for the femur's point
/// 12.49 cm from its origin (s = 0.755). Beyond the slack the link is a
/// hard radius, and L is the slack.
#[test]
fn limp_slack_links_damp_through_their_smoothed_length() {
    let body = leg_and_hand();
    let mut link = links_of(&body, 1)[0].clone();
    // The posed distance, set when the body was built.
    let (a, b) = (
        body.parts[1].point(&link.on_part),
        body.parts[0].point(&link.on_parent),
    );
    assert!(
        (link.length - a.distance(b)).abs() < 1e-6,
        "{}",
        link.length
    );
    let s = link.smoothing;
    assert!((s - 0.755).abs() < 1e-3);
    // At its smoothed length the link does nothing.
    link.length = 0.1;
    assert_eq!(link.limp(Vec3::Y * 0.1), None);
    // Inertia stretched it by 1 cm this tick: 20 passes, each closing what
    // the link asks for.
    let mut d = Vec3::Y * 0.11;
    for _ in 0..LINK_ITERATIONS {
        if let Some(c) = link.limp(d) {
            d -= c;
        }
    }
    let kept = (d.length() - 0.1) / 0.01;
    assert!(
        (kept - (1.0 - s) / (2.0 - s)).abs() < 1e-3 && (kept - 0.197).abs() < 2e-3,
        "kept {kept:.4} of the stretch"
    );
    assert!((link.length - d.length()).abs() < 1e-5);
    // Beyond its 14.88 cm of slack: the excess at full strength.
    let c = link.limp(Vec3::Y * 0.2).unwrap();
    assert!(c.abs_diff_eq(Vec3::Y * (0.2 - link.slack), 1e-6), "{c}");
    assert_eq!(link.length, link.slack);
}

/// Turns the femur and lower leg of `body` by `turn` about the hip, at rest.
fn bend_the_hip(body: &mut Ragdoll, turn: Quat) {
    let hip: Vec3 = body.parts[1].transform().translation.into();
    let about = Affine3A::from_translation(hip)
        * Affine3A::from_quat(turn)
        * Affine3A::from_translation(-hip);
    for p in &mut body.parts[1..2] {
        for k in 0..4 {
            p.positions[k] = about.transform_point3(p.positions[k]);
            p.previous[k] = p.positions[k];
        }
    }
}

/// The femur's rotation relative to the pelvis.
fn hip_angle(body: &Ragdoll) -> Quat {
    let pelvis = body.parts[0].transform();
    let femur = body.parts[1].transform();
    Quat::from_mat3a(&(pelvis.matrix3.inverse() * femur.matrix3))
}

/// Going limp starts each slack link's smoothed length where the link is,
/// within its slack, as the original's death does: a joint bent while the
/// body lived and at rest inside its range then stays where it is. Left at
/// the lengths of the pose it was built in, the hip springs back.
#[test]
fn a_joint_gone_limp_at_rest_stays_put() {
    for reset in [true, false] {
        let mut body = pelvis_and_femur().with_drive();
        bend_the_hip(&mut body, Quat::from_rotation_x(0.35));
        let bent = hip_angle(&body);
        if reset {
            body.go_limp();
            for (k, l) in body.links.iter().enumerate() {
                let (a, b) = (
                    body.parts[l.part].point(&l.on_part),
                    body.parts[l.parent].point(&l.on_parent),
                );
                let want = if l.slack > 0.0 {
                    a.distance(b).min(l.slack)
                } else {
                    0.0
                };
                assert!((l.length - want).abs() < 1e-6, "link {k}: {}", l.length);
            }
        } else {
            body.drive = None;
        }
        for _ in 0..200 {
            body.tick(NONE);
        }
        let moved = hip_angle(&body).angle_between(bent).to_degrees();
        if reset {
            assert!(moved < 0.1, "the hip moved {moved:.2}° at rest");
        } else {
            assert!(moved > 5.0, "the hip moved only {moved:.2}°");
        }
    }
    // A link pulled past its slack starts at the slack.
    let mut body = pelvis_and_femur().with_drive();
    let k = body.links_of[1].start + 1;
    for p in &mut body.parts[1..2] {
        for x in p.positions.iter_mut().chain(p.previous.iter_mut()) {
            *x += Vec3::X * 0.1;
        }
    }
    body.go_limp();
    assert_eq!(body.links[k].length, body.links[k].slack);
}

/// A limp leg swinging inside its range is damped by its links: its swing,
/// and the stretch of its long link, keep 0.8 a tick and under 0.2 after
/// ten ticks (the points' weights spread each pass's correction, so the
/// body keeps more than the 0.20 of a link closing in full), where a leg
/// held by its hip alone keeps 0.86.
#[test]
fn a_limp_swing_inside_its_range_is_damped() {
    // The hip's turn and its long link's stretch in each tick after a swing
    // of 0.03 rad per tick.
    let swing = |body: &mut Ragdoll| {
        let hip: Vec3 = body.parts[1].transform().translation.into();
        let back = Affine3A::from_translation(hip)
            * Affine3A::from_rotation_x(-0.03)
            * Affine3A::from_translation(-hip);
        for p in &mut body.parts[1..2] {
            for k in 0..4 {
                p.previous[k] = back.transform_point3(p.positions[k]);
            }
        }
        let k = body.links_of[1].start;
        let length = |body: &Ragdoll, previous: bool| {
            let l = &body.links[k];
            let at = |p: &Part, w: &[f32; 4]| {
                (0..4)
                    .map(|i| if previous { p.previous[i] } else { p.positions[i] } * w[i])
                    .sum::<Vec3>()
            };
            at(&body.parts[0], &l.on_parent).distance(at(&body.parts[1], &l.on_part))
        };
        let stretch0 = length(body, false) - length(body, true);
        let mut out = Vec::new();
        for _ in 0..10 {
            let before = hip_angle(body);
            body.tick(NONE);
            out.push((
                hip_angle(body).angle_between(before) / 0.03,
                (length(body, false) - length(body, true)) / stretch0,
            ));
        }
        out
    };
    let damped = swing(&mut pelvis_and_femur());
    let mut free = pelvis_and_femur();
    for k in free.links_of[1].clone() {
        if !free.links[k].origin {
            free.links[k].strength = 0.0;
        }
    }
    let free = swing(&mut free);
    let (first, tenth) = (damped[0], damped[9]);
    assert!(
        first.0 < 0.85 && first.1 < 0.85,
        "first tick kept {first:?}"
    );
    assert!(tenth.0 < 0.2 && tenth.1 < 0.2, "tenth tick kept {tenth:?}");
    assert!(
        free[0].0 > 0.95 && free[9].0 > 0.8,
        "a free hip kept {free:?}"
    );
}
