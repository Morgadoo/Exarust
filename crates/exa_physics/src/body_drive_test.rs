//! The pose drive: the original's law and gains, each pinned to its numbers
//! (`docs/formats/RSQ.md`, Physical body).
//! (Included from body.rs: `#[cfg(test)] #[path = "body_drive_test.rs"] mod drive_test;`)
use super::floor_test::{humanoid, standing_at};
use super::tests::floor_quad;
use super::*;

/// The tests' standing humanoid with its feet at the origin.
fn standing() -> (Ragdoll, Vec<Affine3A>) {
    standing_at(Vec3::ZERO)
}

/// The yaw (about the vertical) and the tilt (from the vertical) of a part.
fn turn_of(p: &Part) -> (f32, f32) {
    let m = p.transform().matrix3;
    let forward = Vec3::from(m.z_axis);
    let up = Vec3::from(m.y_axis);
    (forward.x.atan2(forward.z), up.angle_between(Vec3::Y))
}

/// Turns the particles of `p` by `r` about its own origin.
fn turn_part(p: &mut Part, r: Quat) {
    let origin = p.point(&p.own_origin);
    for x in &mut p.positions {
        *x = origin + r * (*x - origin);
    }
    p.previous = p.positions;
}

fn close(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

/// K = 19200/83² = 2.7870, and the gains G = share·K·t²·T·E⁴·(0.5T +
/// 0.5)·S² of an alert human: idle (T = 0.5) 0.0314 on the pelvis, the
/// spine and the feet and 0.0105 on the head; walking (T = 0.7) 0.0497;
/// running (T = 1) 0.0836. None on the thighs, shins, arms and hands. A
/// dazed body (E = 0.5) keeps a sixteenth, a half-supported one a quarter.
#[test]
fn pose_gains_are_the_originals() {
    assert!(close(DRIVE_SCALE, 2.7870, 1e-4), "{DRIVE_SCALE}");
    assert!(close(TRUNK_GAIN * DRIVE_SCALE, 0.0836, 5e-5));
    let mut d = Drive::default();
    for (stance, trunk, head) in [
        (0.5, 0.0314, 0.0105),
        (0.7, 0.0497, 0.0166),
        (1.0, 0.0836, 0.0279),
    ] {
        d.stance = stance;
        for bone in [1, 4, 7, 8, 9] {
            assert!(
                close(d.pose_gain(bone), trunk, 5e-5),
                "s {stance} bone {bone}"
            );
        }
        assert!(close(d.pose_gain(10), head, 5e-5), "s {stance} head");
        for bone in [2, 3, 5, 6, 11, 12, 13, 14, 15, 16, 17, 18] {
            assert_eq!(d.pose_gain(bone), 0.0, "bone {bone}");
        }
    }
    d.stance = 0.5;
    let alert = d.pose_gain(1);
    d.energy = 0.5;
    assert!(close(d.pose_gain(1), alert / 16.0, 1e-7));
    d.energy = 1.0;
    d.support = 0.5;
    assert!(close(d.pose_gain(1), alert / 4.0, 1e-7));
}

/// The push G·o·(1 − c), c = o·v/(s|o|²), in cm a tick, at s = 0.5: still,
/// the whole of G·o; closing at half of s|o| a tick, half of it; at s|o| or
/// faster, none; moving away, more (twice at s|o|); moving across, all of
/// it. The vertical is cut to a tenth, and an offset under 1.19e-7 cm² is
/// left alone.
#[test]
fn the_push_takes_off_what_the_particle_closes() {
    let (s, g) = (0.5, 0.0314);
    let o = Vec3::new(1.0, 0.0, 0.0);
    let push = |v: Vec3| pose_push(o, v, s, g);
    let near = |a: Option<Vec3>, b: Vec3| a.is_some_and(|a| a.abs_diff_eq(b, 1e-7));
    assert!(near(push(Vec3::ZERO), o * g));
    assert!(near(push(Vec3::X * 0.25), o * g * 0.5));
    assert_eq!(push(Vec3::X * 0.5), None);
    assert_eq!(push(Vec3::X * 3.0), None);
    assert!(near(push(Vec3::X * -0.5), o * g * 2.0));
    assert!(near(push(Vec3::Z * 2.0), o * g));
    let up = pose_push(Vec3::new(0.0, 2.0, 1.0), Vec3::ZERO, s, g);
    assert!(near(up, Vec3::new(0.0, 0.2, 1.0) * g));
    assert_eq!(pose_push(Vec3::splat(1.9e-4), Vec3::ZERO, s, g), None);
    assert!(pose_push(Vec3::splat(2.1e-4), Vec3::ZERO, s, g).is_some());
}

/// The pose drive has no position target: a body moved and moving as a
/// whole, every part as the pose turns it, is not pushed at all.
#[test]
fn a_rigid_translation_gives_no_push() {
    let (setup, pose) = humanoid(Vec3::new(3.0, 0.0, -2.0));
    let mut body = Ragdoll::new(&setup).with_drive();
    body.aim(Affine3A::IDENTITY, pose);
    for p in &mut body.parts {
        for k in 0..4 {
            p.positions[k] += Vec3::new(0.05, -0.03, 0.02);
            p.moved[k] = Vec3::new(0.016, 0.004, 0.0);
        }
    }
    let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
    let Ragdoll { parts, drive, .. } = &mut body;
    let drive = drive.as_ref().unwrap();
    assert!(drive.pose_gain(1) > 0.03);
    drive_parts(parts, drive);
    for (p, b) in body.parts.iter().zip(&before) {
        for (x, was) in p.positions.iter().zip(b) {
            assert!(x.abs_diff_eq(*was, 1e-6), "bone {}: {x} from {was}", p.bone);
        }
    }
}

/// A part turned off its pose is turned back about its own origin, which
/// stays where it is: at rest, a yaw closes by G (0.0314 idle) of itself in
/// a tick; a tilt, whose offsets are mostly vertical, by less.
#[test]
fn a_turned_part_is_turned_back_about_its_origin() {
    for (r, yawed) in [
        (Quat::from_rotation_y(0.3), true),
        (Quat::from_rotation_x(0.3), false),
    ] {
        let (setup, pose) = humanoid(Vec3::ZERO);
        let mut body = Ragdoll::new(&setup).with_drive();
        body.aim(Affine3A::IDENTITY, pose);
        let i = body.parts.iter().position(|p| p.bone == 8).unwrap();
        turn_part(&mut body.parts[i], r);
        let origin = body.parts[i].point(&body.parts[i].own_origin);
        let (yaw, tilt) = turn_of(&body.parts[i]);
        let Ragdoll { parts, drive, .. } = &mut body;
        let g = drive.as_ref().unwrap().pose_gain(8);
        drive_parts(parts, drive.as_ref().unwrap());
        // A yaw's offsets are level, and the pushes add up to nothing at
        // the origin; a tilt's vertical share is cut.
        let moved = parts[i].point(&parts[i].own_origin) - origin;
        assert!(moved.length() < if yawed { 1e-6 } else { 2e-3 }, "{moved}");
        keep_rigid(&mut parts[i]);
        let p = &body.parts[i];
        let (yaw_after, tilt_after) = turn_of(p);
        if yawed {
            assert!(close(yaw, 0.3, 1e-4));
            assert!(
                close(yaw_after, 0.3 * (1.0 - g), 0.002),
                "yaw {yaw_after}, not {}",
                0.3 * (1.0 - g)
            );
        } else {
            assert!(close(tilt, 0.3, 1e-4));
            assert!(
                tilt_after < tilt && tilt_after > 0.3 * (1.0 - g),
                "tilt {tilt_after}"
            );
        }
    }
}

/// The damper reads the particle's world velocity, the body's motion
/// included: a part turned a little while it moves at 1.6 cm a tick (a
/// walk) is not pushed where its offset points along the motion (c ≥ 1)
/// and is pushed harder than G·o where it points against it.
#[test]
fn moving_parts_are_pushed_against_their_motion_only() {
    let (setup, pose) = humanoid(Vec3::ZERO);
    let mut body = Ragdoll::new(&setup).with_drive();
    body.aim(Affine3A::IDENTITY, pose);
    let i = body.parts.iter().position(|p| p.bone == 8).unwrap();
    turn_part(&mut body.parts[i], Quat::from_rotation_y(0.05));
    body.parts[i].moved = [Vec3::new(0.016, 0.0, 0.0); 4];
    let before = body.parts[i].positions;
    let Ragdoll { parts, drive, .. } = &mut body;
    let drive = drive.as_ref().unwrap();
    let g = drive.pose_gain(8);
    let (_, turn, _) = drive.targets[i].to_scale_rotation_translation();
    let p = &parts[i];
    let origin = p.point(&p.own_origin);
    let offsets: Vec<Vec3> = (0..4)
        .map(|k| origin + turn * p.rest[k] - p.positions[k])
        .collect();
    drive_parts(parts, drive);
    let (mut along, mut against) = (0, 0);
    for k in 0..4 {
        let push = body.parts[i].positions[k] - before[k];
        let o = offsets[k];
        // The yaw's offsets are level; s = 0.5 idle.
        let c = o.x * 0.016 / (0.5 * o.length_squared());
        if c >= 1.0 {
            assert_eq!(push, Vec3::ZERO, "particle {k}");
            along += 1;
        } else if c < -0.5 {
            assert!(push.length() > g * o.length() * 1.5, "particle {k}: {push}");
            against += 1;
        }
    }
    assert!(along > 0 && against > 0, "{along} along, {against} against");
}

/// Without support there is no pose drive: a fallen body (support 0) and
/// one whose feet have no ground are left to their link muscles. A fall
/// keeps the body's tone, and standing again does not change it.
#[test]
fn no_support_no_pose_drive_and_a_fall_keeps_the_tone() {
    let none: &[[Vec3; 3]] = &[];
    for case in 0..2 {
        let (mut body, pose) = standing();
        let turned: Vec<Affine3A> = pose
            .iter()
            .map(|t| *t * Affine3A::from_rotation_y(0.4))
            .collect();
        body.drive.as_mut().unwrap().tone = 0.9;
        if case == 0 {
            // Lying on its back on the floor: its feet are out of reach.
            let lie = Affine3A::from_translation(Vec3::Y * 0.15)
                * Affine3A::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                * Affine3A::from_translation(Vec3::NEG_Y * 1.1);
            for p in &mut body.parts {
                p.positions = p.positions.map(|x| lie.transform_point3(x));
                p.previous = p.positions;
            }
            body.fall();
            body.measure_balance(floor_quad(0.0, 5.0).as_slice());
            assert!(!body.drive.as_ref().unwrap().balance().has_support());
        } else {
            // Carried by its controller, it keeps its balance.
            body.measure_balance(none);
            assert_eq!(body.drive.as_ref().unwrap().balance().value, 1.0);
            assert!(!body.drive.as_ref().unwrap().balance().has_support());
        }
        body.aim(Affine3A::IDENTITY, turned);
        assert_eq!(
            body.drive.as_ref().unwrap().pose_gain(1),
            0.0,
            "case {case}"
        );
        let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
        let Ragdoll { parts, drive, .. } = &mut body;
        drive_parts(parts, drive.as_ref().unwrap());
        for (p, b) in body.parts.iter().zip(&before) {
            assert_eq!(p.positions, *b, "case {case}, bone {}", p.bone);
        }
        assert_eq!(body.drive.as_ref().unwrap().tone, 0.9);
        body.stand();
        assert_eq!(body.drive.as_ref().unwrap().tone, 0.9);
    }
}

/// Energy recovers 0.005 a tick, up to a normal human's most, 1.
#[test]
fn energy_recovers_to_its_cap() {
    let (setup, _) = humanoid(Vec3::ZERO);
    let mut body = Ragdoll::new(&setup).with_drive();
    body.drive.as_mut().unwrap().energy = 0.99;
    let none: &[[Vec3; 3]] = &[];
    body.tick(none);
    assert!(close(body.drive.as_ref().unwrap().energy, 0.995, 1e-6));
    body.tick(none);
    body.tick(none);
    assert_eq!(body.drive.as_ref().unwrap().energy, MAX_ENERGY);
}

/// A standing body shoved at the chest at 1.5 m/s sways and recovers: its
/// torso is turned off its pose and turns back, it keeps its balance (never
/// under 0.6), and it comes to rest standing.
#[test]
fn a_standing_body_recovers_from_a_chest_shove() {
    let (mut body, pose) = standing();
    let floor = floor_quad(0.0, 5.0);
    let torso = body.parts.iter().position(|p| p.bone == 8).unwrap();
    let tick = |body: &mut Ragdoll| {
        body.measure_balance(floor.as_slice());
        body.aim(Affine3A::IDENTITY, pose.clone());
        body.tick(floor.as_slice());
    };
    for _ in 0..40 {
        tick(&mut body);
    }
    let rest = turn_of(&body.parts[torso]).1;
    body.shove(8, Vec3::Z * 1.5, 0.0);
    let (mut lowest, mut swayed) = (1.0f32, 0.0f32);
    for _ in 0..250 {
        tick(&mut body);
        lowest = lowest.min(body.drive.as_ref().unwrap().balance().value);
        swayed = swayed.max(turn_of(&body.parts[torso]).1 - rest);
    }
    let tilt = turn_of(&body.parts[torso]).1 - rest;
    let (_, velocity) = body.centre_of_mass();
    assert!(
        swayed.to_degrees() > 2.5,
        "the torso swayed {:.1}°",
        swayed.to_degrees()
    );
    assert!(lowest >= 0.6, "its balance fell to {lowest}");
    assert!(
        tilt.to_degrees().abs() < 1.0,
        "the torso is {:.1}° off",
        tilt.to_degrees()
    );
    assert!(velocity.length() < 0.01, "still moving at {velocity}");
}

/// Exarust's own until the support pins: a standing body's pelvis is held
/// over where its frame puts the pose's pelvis, across the ground only. A
/// pelvis off its target and moving is pulled back and braked across the
/// ground, never up or down: its height is the lift's. While the body
/// moves its frame rides on its pelvis, and it is only braked.
#[test]
fn the_stance_hold_pulls_the_pelvis_across_the_ground_only() {
    for moving in [false, true] {
        let (mut body, pose) = standing();
        body.aim(Affine3A::IDENTITY, pose);
        body.drive.as_mut().unwrap().bearing.moving = moving;
        let i = body.parts.iter().position(|p| p.bone == 1).unwrap();
        let p = &mut body.parts[i];
        for k in 0..4 {
            p.positions[k] += Vec3::new(0.05, 0.04, -0.03);
            p.previous[k] = p.positions[k] - Vec3::new(0.002, -0.003, 0.001);
        }
        let (before, was) = (p.positions, p.previous);
        let Ragdoll { parts, drive, .. } = &mut body;
        hold_on_frame(parts, drive.as_ref().unwrap());
        let p = &body.parts[i];
        for k in 0..4 {
            let pulled = p.positions[k] - before[k];
            let braked = p.previous[k] - was[k];
            if moving {
                assert_eq!(pulled, Vec3::ZERO);
            } else {
                assert!(pulled.x < 0.0 && pulled.z > 0.0, "{pulled}");
            }
            assert_eq!((pulled.y, braked.y), (0.0, 0.0));
            // Its previous place follows its motion: its velocity shrinks.
            assert!(braked.x > 0.0 && braked.z > 0.0, "{braked}");
        }
    }
}

/// The weight with which a body's targets give way to the pose it holds:
/// dazed (energy more than 0.05 under its most), 1 − (1 − S)²·(1 − s)·
/// (E/Emax)⁴, so a dazed body standing on its feet holds its pose in full
/// and one lying (E = 0.5) 0.9375; alert, (1 − S)·s, nothing without a
/// blow sag. The sag fades 0.01 a tick, and is gone once the body falls.
#[test]
fn a_dazed_body_holds_its_pose() {
    let mut d = Drive {
        energy: 0.5,
        ..Drive::default()
    };
    assert!(d.dazed() && d.holds_pose());
    assert_eq!(d.hold_weight(), 1.0);
    d.support = 0.0;
    assert!(close(d.hold_weight(), 0.9375, 1e-6));
    d.support = 0.5;
    d.sag = 0.5;
    assert!(close(d.hold_weight(), 1.0 - 0.25 * 0.5 * 0.0625, 1e-6));
    // The threshold is the double 0.95: 0.95f is under it, the next float
    // up is not.
    d.energy = 0.95;
    assert!(d.dazed());
    d.energy = 0.95f32.next_up();
    assert!(!d.dazed());
    d.energy = 0.951;
    assert!(!d.dazed());
    assert!(close(d.hold_weight(), 0.25, 1e-6));
    d.sag = 0.0;
    assert_eq!(d.hold_weight(), 0.0);
    assert!(d.holds_pose());
    d.support = 1.0;
    assert!(!d.holds_pose());
    // The sag fades with the ticks, and goes with a fall.
    let (mut body, targets) = standing();
    let floor = floor_quad(0.0, 5.0);
    body.aim(Affine3A::IDENTITY, targets);
    body.drive.as_mut().unwrap().sag = 0.5;
    body.tick(floor.as_slice());
    assert!(close(body.drive.as_ref().unwrap().sag, 0.49, 1e-6));
    body.fall();
    body.tick(floor.as_slice());
    assert_eq!(body.drive.as_ref().unwrap().sag, 0.0);
}
