//! The character frame: the original's rules, each pinned to its numbers
//! (`docs/formats/RSQ.md`, Physical body).
//! (Included from body_frame.rs: `#[path = "body_frame_test.rs"] mod tests;`)
use super::super::floor_test::standing_at;
use super::super::tests::floor_quad;
use super::super::{Bearing, Drive};
use super::*;

fn drive(body: &Ragdoll) -> &Drive {
    body.drive.as_ref().unwrap()
}

fn cm(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z) * 0.01
}

/// The pelvis's origin (m).
fn pelvis(body: &Ragdoll) -> Vec3 {
    Vec3::from(body.part_of(1).unwrap().transform().translation)
}

/// One character tick and one solver tick of a body standing in `pose`
/// on `level`, its support set to `support` once measured.
fn stand(body: &mut Ragdoll, pose: &[Affine3A], level: &[[Vec3; 3]], support: Option<f32>) {
    body.measure_balance(level);
    if let Some(s) = support {
        body.drive.as_mut().unwrap().support = s;
    }
    body.aim(Affine3A::IDENTITY, pose.to_vec());
    body.tick(level);
}

/// The frame follows the pelvis by what the body does: a get-up within
/// 10 cm while its clip is in its first quarter, then not at all; down (off
/// balance under 0.3) within 5 cm; moving onto the pelvis itself; without
/// support every tick; standing on its feet within 5 cm while its support
/// is under the double 0.95 (a float 0.95 is under it), else not at all,
/// its height kept at the pelvis's either way.
#[test]
fn the_frame_follows_by_the_originals_rules() {
    let standing = Bearing::default();
    let at = |b: &Bearing, balance: f32, support: Option<f32>| {
        follow(b, balance, support.is_some(), support.unwrap_or(0.0))
    };
    let rising = Bearing {
        off_balance: true,
        getting_up: true,
        rising: true,
        ..standing
    };
    assert_eq!(at(&rising, 0.1, Some(0.0)), (Follow::Within(10.0), false));
    let risen = Bearing {
        rising: false,
        ..rising
    };
    assert_eq!(at(&risen, 0.9, Some(0.9)), (Follow::Stays, false));
    let fallen = Bearing {
        off_balance: true,
        ..standing
    };
    assert_eq!(at(&fallen, 0.29, None), (Follow::Within(5.0), false));
    // Falling, not yet down: as it stands.
    assert_eq!(at(&fallen, 0.3, None), (Follow::Within(0.0), false));
    assert_eq!(at(&fallen, 0.5, Some(0.5)), (Follow::Within(5.0), true));
    let moving = Bearing {
        moving: true,
        ..standing
    };
    assert_eq!(at(&moving, 1.0, Some(1.0)), (Follow::Pelvis, false));
    assert_eq!(at(&moving, 1.0, None), (Follow::Pelvis, false));
    assert_eq!(at(&standing, 1.0, None), (Follow::Within(0.0), false));
    assert_eq!(at(&standing, 1.0, Some(0.9)), (Follow::Within(5.0), true));
    assert_eq!(at(&standing, 1.0, Some(0.95)), (Follow::Within(5.0), true));
    let over = f32::from_bits(0.95f32.to_bits() + 1);
    assert_eq!(at(&standing, 1.0, Some(over)), (Follow::Stays, true));
    assert_eq!(at(&standing, 1.0, Some(1.0)), (Follow::Stays, true));
}

/// The origin goes to the pelvis less the pose's root offset across the
/// ground, at the pelvis's height, once that is more than the reach away,
/// measured in all three axes in cm: placed at once the first time; not at
/// 5 cm, at 5.01; a standing body's height is brought to the pelvis's
/// first, a lying one's counts; a moving body's goes onto the pelvis
/// itself, its offset left out.
#[test]
fn the_origin_moves_once_past_its_reach() {
    let pelvis = cm(100.0, 90.0, -40.0);
    let offset = cm(3.0, 100.7, 4.0);
    let wanted = cm(97.0, 90.0, -44.0);
    let near = |a: Vec3, b: Vec3| a.abs_diff_eq(b, 1e-6);
    assert!(near(
        anchored(None, pelvis, offset, Follow::Stays, false),
        wanted
    ));
    let five = wanted + cm(3.0, 0.0, 4.0);
    let past = wanted + cm(3.0, 0.0, 4.01);
    let within = Follow::Within(5.0);
    assert!(near(
        anchored(Some(five), pelvis, offset, within, false),
        five
    ));
    assert!(near(
        anchored(Some(past), pelvis, offset, within, false),
        wanted
    ));
    // 4 cm across and 6 cm under: a standing body follows its height, a
    // lying one is past its reach.
    let under = wanted + cm(4.0, -6.0, 0.0);
    assert!(near(
        anchored(Some(under), pelvis, offset, within, true),
        wanted + cm(4.0, 0.0, 0.0)
    ));
    assert!(near(
        anchored(Some(under), pelvis, offset, within, false),
        wanted
    ));
    assert!(near(
        anchored(Some(under), pelvis, offset, Follow::Within(10.0), false),
        under
    ));
    let still = wanted + cm(0.001, 0.0, 0.0);
    assert!(near(
        anchored(Some(still), pelvis, offset, Follow::Within(0.0), false),
        wanted
    ));
    assert!(near(
        anchored(Some(past), pelvis, offset, Follow::Stays, false),
        past
    ));
    assert!(near(
        anchored(Some(past), pelvis, offset, Follow::Pelvis, false),
        pelvis
    ));
}

/// Where a standing body is pushed across the floor at 3 m/s: whether its
/// frame stayed put, how far its pelvis went at most and where it ends
/// (cm, across the ground), where its frame ends, and how far its pelvis
/// ends from where its frame puts the pose's.
fn pushed(support: f32) -> (bool, f32, f32, f32, f32) {
    let floor = floor_quad(0.0, 5.0);
    let (mut body, pose) = standing_at(Vec3::ZERO);
    for _ in 0..60 {
        stand(&mut body, &pose, &floor, Some(support));
    }
    let start = pelvis(&body);
    let anchor = drive(&body).anchor().unwrap();
    body.set_velocity(Vec3::Z * 3.0);
    let (mut far, mut stayed) = (0.0f32, true);
    for _ in 0..300 {
        stand(&mut body, &pose, &floor, Some(support));
        let now = drive(&body).anchor().unwrap();
        stayed &= now.xz() == anchor.xz();
        far = far.max((pelvis(&body) - start).with_y(0.0).length());
    }
    let (_, velocity) = body.centre_of_mass();
    assert!(velocity.length() < 0.005, "still moving at {velocity}");
    let moved = drive(&body).anchor().unwrap() - anchor;
    let place = Vec3::from(drive(&body).targets()[0].translation) - pelvis(&body);
    (
        stayed,
        far * 100.0,
        (pelvis(&body) - start).with_y(0.0).length() * 100.0,
        moved.with_y(0.0).length() * 100.0,
        place.with_y(0.0).length() * 100.0,
    )
}

/// A steady standing body (support 1) keeps its stance spot: pushed 9 cm
/// across the floor, its frame stays put and it is held back onto it. An
/// unsteady one (support 0.95) stays where it is pushed: pushed 14 cm, its
/// frame follows its pelvis once it is 5 cm off, so it is drawn back no
/// more than 5 cm, and it comes to rest about where its frame now puts it.
#[test]
fn a_body_shoved_standing_stays_where_it_was_pushed() {
    let (stayed, far, end, moved, _) = pushed(1.0);
    assert!(stayed && moved == 0.0, "the frame moved {moved} cm");
    assert!(far > 8.0, "pushed {far} cm");
    assert!(end < 1.0, "it rests {end} cm off its spot");
    let (stayed, far, end, moved, gap) = pushed(0.95);
    assert!(!stayed);
    assert!(
        far > 12.0 && end > far - 5.0,
        "pushed {far} cm, rests {end}"
    );
    assert!((moved - end).abs() < 1.0, "frame {moved} cm, pelvis {end}");
    assert!(gap < 0.5, "{gap} cm from where its frame puts it");
}

/// The frame rides with the carry, and the carry leaves out what the
/// controller moved to follow the frame: a standing body whose root moves
/// 30 cm across the ground under it is carried with it, frame and all,
/// unless the controller says it followed.
#[test]
fn the_carry_leaves_the_follow_out() {
    let floor = floor_quad(0.0, 5.0);
    for followed in [false, true] {
        let (mut body, pose) = standing_at(Vec3::ZERO);
        for _ in 0..30 {
            stand(&mut body, &pose, &floor, None);
        }
        let (start, anchor) = (pelvis(&body), drive(&body).anchor().unwrap());
        let step = cm(30.0, 0.0, -10.0);
        let root = Affine3A::from_translation(step);
        if followed {
            body.followed(step);
        }
        body.measure_balance(floor.as_slice());
        body.aim(root, pose.iter().map(|t| root * *t).collect());
        let (moved, frame) = (
            pelvis(&body) - start,
            drive(&body).anchor().unwrap() - anchor,
        );
        let want = if followed { Vec3::ZERO } else { step };
        assert!(moved.abs_diff_eq(want, 1e-5), "{followed}: pelvis {moved}");
        assert!(
            frame.with_y(0.0).abs_diff_eq(want, 1e-5),
            "{followed}: frame {frame}"
        );
        // The pose is put on the frame, under the body either way.
        let place = Vec3::from(drive(&body).targets()[0].translation) - pelvis(&body);
        assert!(place.with_y(0.0).length() < 0.005, "{place}");
    }
}

/// A get-up re-anchors the frame to the pelvis: in the clip's first
/// quarter within 10 cm, so a pose whose pelvis lies 30 cm off the frame's
/// origin is put on the body at once; then not at all, wherever the pelvis
/// goes.
#[test]
fn a_get_up_re_anchors_the_frame() {
    let floor = floor_quad(0.0, 5.0);
    let (mut body, pose) = standing_at(Vec3::ZERO);
    for _ in 0..30 {
        stand(&mut body, &pose, &floor, None);
    }
    let anchor = drive(&body).anchor().unwrap();
    body.fall();
    let mut bearing = Bearing {
        off_balance: true,
        getting_up: true,
        rising: true,
        ..Bearing::default()
    };
    body.stand();
    body.set_bearing(bearing);
    // The get-up's pose: its pelvis 30 cm behind the root.
    let lying = Affine3A::from_translation(cm(0.0, 0.0, -30.0));
    let rise: Vec<Affine3A> = pose.iter().map(|t| lying * *t).collect();
    body.aim(Affine3A::IDENTITY, rise.clone());
    let moved = drive(&body).anchor().unwrap() - anchor;
    assert!(
        moved.xz().abs_diff_eq(cm(0.0, 0.0, 30.0).xz(), 1e-3),
        "{moved}"
    );
    let place = Vec3::from(drive(&body).targets()[0].translation) - pelvis(&body);
    assert!(place.with_y(0.0).length() < 1e-3, "{place}");
    // 8 cm off: still within its reach; 12 cm: past it.
    let anchored = drive(&body).anchor().unwrap();
    let nudge = |body: &mut Ragdoll, by: Vec3| {
        for p in &mut body.parts {
            for k in 0..4 {
                p.positions[k] += by;
                p.previous[k] += by;
            }
        }
    };
    nudge(&mut body, cm(8.0, 0.0, 0.0));
    body.aim(Affine3A::IDENTITY, rise.clone());
    assert_eq!(drive(&body).anchor().unwrap().xz(), anchored.xz());
    nudge(&mut body, cm(4.0, 0.0, 0.0));
    body.aim(Affine3A::IDENTITY, rise.clone());
    let followed = drive(&body).anchor().unwrap() - anchored;
    assert!(followed.xz().abs_diff_eq(cm(12.0, 0.0, 0.0).xz(), 1e-3));
    // Past the first quarter it stays.
    bearing.rising = false;
    body.set_bearing(bearing);
    let held = drive(&body).anchor().unwrap();
    nudge(&mut body, cm(0.0, 0.0, 20.0));
    body.aim(Affine3A::IDENTITY, rise);
    assert_eq!(drive(&body).anchor().unwrap(), held);
}

/// Exarust's own carry is exact for a frame whose root has not moved: a
/// steady standing body 90 m from the world's origin, turned as a
/// character is, keeps its frame where it is tick after tick (a frame
/// carried through the root's inverse would creep by float rounding,
/// about 0.01 mm a tick there).
#[test]
fn a_steady_frame_far_from_the_origin_stays_put() {
    let at = Vec3::new(81.0, 0.0, 39.2);
    let floor = floor_quad(0.0, 100.0);
    let (mut body, pose) = standing_at(at);
    let root = Affine3A::from_translation(at) * Affine3A::from_rotation_y(0.7);
    let turn = root * Affine3A::from_translation(-at);
    for p in &mut body.parts {
        for k in 0..4 {
            p.positions[k] = turn.transform_point3(p.positions[k]);
            p.previous[k] = p.positions[k];
        }
    }
    let pose: Vec<Affine3A> = pose.iter().map(|t| turn * *t).collect();
    let tick = |body: &mut Ragdoll| {
        body.measure_balance(floor.as_slice());
        body.drive.as_mut().unwrap().support = 1.0;
        body.aim(root, pose.clone());
        body.tick(floor.as_slice());
    };
    for _ in 0..60 {
        tick(&mut body);
    }
    let anchor = drive(&body).anchor().unwrap();
    let mut far = 0.0f32;
    for _ in 0..600 {
        tick(&mut body);
        far = far.max((drive(&body).anchor().unwrap() - anchor).xz().length());
    }
    assert_eq!(far, 0.0, "the frame crept {} mm", far * 1000.0);
}

/// A collapsing body's frame follows its pelvis exactly every tick (the
/// original re-anchors it with no reach), whatever its balance and support.
#[test]
fn a_collapsing_frame_follows_the_pelvis_exactly() {
    let collapsing = Bearing {
        collapsing: true,
        ..Bearing::default()
    };
    for (balance, support) in [(1.0, Some(1.0)), (0.29, None), (0.7, Some(0.5))] {
        assert_eq!(
            follow(
                &collapsing,
                balance,
                support.is_some(),
                support.unwrap_or(0.0)
            ),
            (Follow::Within(0.0), false)
        );
    }
}
