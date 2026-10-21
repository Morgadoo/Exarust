//! The lift reads each foot's own ground. The controller's floor does not
//! raise the pelvis, on level ground or on the stairs.
//! (Included from body_lift.rs: `#[path = "body_climb_test.rs"] mod climb_tests;`)
use bevy::math::Affine3A;
use bevy::prelude::*;

use super::super::floor_test::{STANDING_LEG, STANDING_SOLE, standing_at};
use super::super::locomotion::LegMode;
use super::super::tests::floor_quad;
use super::super::{BODY_HZ, Bearing, Ragdoll};

/// The game's walking speed (m/s), and its capsule's radius and float over
/// the ground while it walks (m).
const WALK: f32 = 1.4;
const CAPSULE_RADIUS: f32 = 0.28;
const FLOAT: f32 = 0.02;
/// The staircase: four 18 cm steps 30 cm deep, the first riser at z = 0,
/// up to a landing.
const STEPS: usize = 4;
const RISE: f32 = 0.18;
const TREAD: f32 = 0.30;
/// Half the staircase's width, and how far its floor and landing run (m).
const HALF_WIDTH: f32 = 2.0;
const RUN: f32 = 5.0;
/// How far (rad) each leg of the walker is swung from upright, one forward,
/// one back.
const STRIDE: f32 = 0.4;

/// The level: a floor at 0 up to the first riser, then the steps (each
/// riser facing the walker) and the landing.
fn staircase() -> Vec<[Vec3; 3]> {
    let w = HALF_WIDTH;
    let mut tris = Vec::new();
    let mut top = |y: f32, z0: f32, z1: f32| {
        let at = |x: f32, z: f32| Vec3::new(x, y, z);
        tris.push([at(-w, z0), at(-w, z1), at(w, z1)]);
        tris.push([at(-w, z0), at(w, z1), at(w, z0)]);
    };
    top(0.0, -RUN, 0.0);
    for k in 1..=STEPS {
        let z0 = (k - 1) as f32 * TREAD;
        let z1 = if k == STEPS { RUN } else { k as f32 * TREAD };
        top(k as f32 * RISE, z0, z1);
    }
    for k in 1..=STEPS {
        let z = (k - 1) as f32 * TREAD;
        let (y0, y1) = ((k - 1) as f32 * RISE, k as f32 * RISE);
        let at = |x: f32, y: f32| Vec3::new(x, y, z);
        tris.push([at(-w, y0), at(-w, y1), at(w, y1)]);
        tris.push([at(-w, y0), at(w, y1), at(w, y0)]);
    }
    tris
}

/// The staircase's height at `z`.
fn height(z: f32) -> f32 {
    if z < 0.0 {
        0.0
    } else {
        ((z / TREAD).floor() as usize + 1).min(STEPS) as f32 * RISE
    }
}

/// Where the game's capsule, its axis at `z`, stands on the staircase: on
/// the step under its axis, or riding up the edge of a step ahead within
/// its radius (its hemisphere slides up the edge as the controller steps
/// up), plus its float.
fn capsule_floor(z: f32) -> f32 {
    let r = CAPSULE_RADIUS;
    let mut floor = height(z);
    for k in 1..=STEPS {
        let d = (k - 1) as f32 * TREAD - z;
        if d > 0.0 && d < r {
            floor = floor.max(k as f32 * RISE - (r - (r * r - d * d).sqrt()));
        }
    }
    floor + FLOAT
}

/// A walking body's bearing, its controller on the ground or not, its
/// lowest ankle `gait_floor` over its floor (its feet do not touch, as a
/// walking body's do not without its gait's plants).
fn walking(carried: bool, gait_floor: f32) -> Bearing {
    Bearing {
        moving: true,
        legs: LegMode::Walk,
        striding: true,
        walking: true,
        carried,
        gait_floor: Some(gait_floor),
        ..Bearing::default()
    }
}

/// The standing humanoid's pose (its feet at the origin) mid-stride: its
/// right leg swung `stride` rad forward about its hip, its left as far
/// back; and how high its ankles then are.
fn striding(stride: f32) -> (Vec<Affine3A>, f32) {
    let (body, mut pose) = standing_at(Vec3::ZERO);
    for (i, p) in body.parts.iter().enumerate() {
        let (hip, turn) = match p.bone {
            2 | 3 => (Vec3::new(0.1, 1.0, 0.0), -stride),
            5 | 6 => (Vec3::new(-0.1, 1.0, 0.0), stride),
            _ => continue,
        };
        let swing = Affine3A::from_translation(hip)
            * Affine3A::from_rotation_x(turn)
            * Affine3A::from_translation(-hip);
        pose[i] = swing * pose[i];
    }
    (pose, 1.0 - stride.cos())
}

/// One character tick and one solver tick of `body` in `bearing`, its root
/// at `root` and its pose `pose` (in the root's space), on `level`.
fn step(
    body: &mut Ragdoll,
    bearing: Bearing,
    root: Affine3A,
    pose: &[Affine3A],
    level: &[[Vec3; 3]],
) {
    body.set_bearing(bearing);
    body.measure_balance(level);
    body.aim(root, pose.iter().map(|p| root * *p).collect());
    body.tick(level);
}

/// The lift's target (m) this tick.
fn target(body: &Ragdoll) -> f32 {
    body.drive.as_ref().unwrap().lift_target().unwrap()
}

/// The lowest balance of `body` so far, with this tick's.
fn lowest_balance(body: &Ragdoll, so_far: f32) -> f32 {
    so_far.min(body.drive.as_ref().unwrap().balance().value)
}

/// Every one of `targets` is `want`.
fn all_at(targets: &[f32], want: f32) {
    for (tick, t) in targets.iter().enumerate() {
        assert!((t - want).abs() < 1e-5, "tick {tick}: {t}, not {want}");
    }
}

/// On level ground the target is the original's at a 0, 2 or 5 cm float,
/// walking or standing: walking, the ground plus the clip root's height
/// over its lowest ankle plus its sole (1.1 m + the sole here); standing,
/// capped at its ground + sole + legs, + 5 cm. A controller 12 cm up,
/// grounded or in the air, does not add to it.
#[test]
fn level_ground_keeps_the_originals_target() {
    let floor = floor_quad(0.0, RUN);
    let targets = |float: f32, bearing: Bearing| {
        let (mut body, pose) = standing_at(Vec3::ZERO);
        let mut targets = Vec::new();
        for tick in 0..200 {
            let z = if bearing.moving {
                tick as f32 * WALK / BODY_HZ
            } else {
                0.0
            };
            let root = Affine3A::from_translation(Vec3::new(0.0, float, z));
            step(&mut body, bearing, root, &pose, &floor);
            targets.push(target(&body));
        }
        targets
    };
    let standing = Bearing::default();
    let walks = 1.1 + STANDING_SOLE;
    let stands = STANDING_LEG + STANDING_SOLE + 0.05;
    for float in [0.0, 0.02, 0.05] {
        all_at(&targets(float, walking(true, 0.0)), walks);
        all_at(&targets(float, standing), stands);
    }
    all_at(&targets(0.12, walking(true, 0.0)), walks);
    all_at(&targets(0.12, standing), stands);
    all_at(&targets(0.12, walking(false, 0.0)), walks);
}

/// A fallen body does not ride its controller: just fallen, its feet still
/// on the floor and its grounded controller perched 12 cm up, its target is
/// the original's, the same every tick as with its controller on the floor.
#[test]
fn a_fallen_body_keeps_the_originals_target() {
    let floor = floor_quad(0.0, RUN);
    let fallen = Bearing {
        off_balance: true,
        ..Bearing::default()
    };
    let targets = |float: f32| {
        let (mut body, pose) = standing_at(Vec3::ZERO);
        body.fall();
        let root = Affine3A::from_translation(Vec3::new(0.0, float, 0.0));
        (0..20)
            .map(|_| {
                step(&mut body, fallen, root, &pose, &floor);
                target(&body)
            })
            .collect::<Vec<_>>()
    };
    let on_floor = targets(0.0);
    assert!(on_floor[0] > 0.5, "no target: {}", on_floor[0]);
    for (tick, (perched, original)) in targets(0.12).iter().zip(&on_floor).enumerate() {
        assert!(
            (perched - original).abs() < 1e-4,
            "tick {tick}: {perched}, not {original}"
        );
    }
}

/// How a walk up the staircase went: the lowest balance on the way, and at
/// the end the pelvis's height and its lowest foot contact's.
struct Climb {
    balance: f32,
    pelvis: f32,
    feet: f32,
}

/// A body walking at the game's speed from 1.2 m before the first riser
/// onto the landing, carried by a capsule that steps up the stairs ahead of
/// its feet, its legs a stride apart; it stops on the third step for 3 s,
/// standing straight, then walks on 1 m onto the landing. Its controller is
/// on the ground or not.
fn climb(carried: bool) -> Climb {
    let level = staircase();
    let start = -1.2;
    let stop = 2.5 * TREAD;
    let end = (STEPS - 1) as f32 * TREAD + 1.0;
    let (mut body, _) = standing_at(Vec3::new(0.0, 0.0, start));
    let (_, upright) = standing_at(Vec3::ZERO);
    let (stride, ankles) = striding(STRIDE);
    let at = |z: f32| Affine3A::from_translation(Vec3::new(0.0, capsule_floor(z), z));
    let standing = Bearing {
        carried,
        ..Bearing::default()
    };
    for _ in 0..100 {
        step(&mut body, standing, at(start), &upright, &level);
    }
    let mut balance = 1.0f32;
    let mut z = start;
    let mut stopped = false;
    while z < end {
        if z >= stop && !stopped {
            stopped = true;
            for _ in 0..(3.0 * BODY_HZ) as usize {
                step(&mut body, standing, at(z), &upright, &level);
                balance = lowest_balance(&body, balance);
            }
        }
        z += WALK / BODY_HZ;
        step(&mut body, walking(carried, ankles), at(z), &stride, &level);
        balance = lowest_balance(&body, balance);
    }
    let pelvis = body.part_of(1).unwrap().transform().translation.y;
    let feet = body
        .legs()
        .iter()
        .map(|l| body.parts[l.shin].point(&l.contact).y)
        .fold(f32::MAX, f32::min);
    Climb {
        balance,
        pelvis,
        feet,
    }
}

/// A body walking up four 18 cm steps keeps its balance at or above 0.9
/// and ends with its feet on the landing. The controller's floor does not
/// raise the pelvis: carried or not, the two pelvis heights agree.
#[test]
fn a_walker_climbs_four_18_cm_steps() {
    let landing = STEPS as f32 * RISE;
    let up = climb(true);
    assert!(up.balance >= 0.9, "lost its balance: {}", up.balance);
    assert!(up.feet > landing - 0.02, "feet at {}", up.feet);
    let caught = climb(false);
    assert!(
        caught.feet > landing - 0.02,
        "feet left the landing: {}",
        caught.feet
    );
    assert!(
        (up.pelvis - caught.pelvis).abs() < 0.02,
        "the controller floor raised the pelvis: {} vs {}",
        up.pelvis,
        caught.pelvis
    );
    assert!(
        caught.balance >= 0.9,
        "lost its balance: {}",
        caught.balance
    );
}
