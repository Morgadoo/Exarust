//! A synthetic hinged leaf (not game data): 1 m from its hinge to its stop,
//! 2 m tall, its stop link 80 cm of slack, so a half swing of
//! 2·asin(0.4) ≈ 0.82 rad. At its file position the stop is taut on the
//! shut side.
use super::*;
use crate::body::{RigidLink, RigidPart};
use crate::door::{LATCHES, Record};

const NONE: &[[Vec3; 3]] = &[];
const SLACK: f32 = 0.8;

/// The shut stop's tied point: the stop's body point (−1, 0, 0) turned by
/// the half swing toward +z.
fn tied() -> Vec3 {
    let alpha = door::half_swing(SLACK * CM, 100.0);
    Vec3::new(-alpha.cos(), 0.0, alpha.sin())
}

/// The leaf's rigid body at `at`, turned `yaw` about the vertical.
fn leaf(at: Vec3, yaw: f32) -> RigidPart {
    let link = |on_body: Vec3, on_parent: Vec3, slack: f32| RigidLink {
        on_body,
        on_parent,
        slack,
        strength: 1.0,
        smoothing: 0.2,
        flags: 0,
        axis: Vec3::ZERO,
    };
    let turn = Quat::from_rotation_y(yaw);
    let mut spheres = Vec::new();
    for i in 0..10 {
        for j in 0..20 {
            spheres.push((
                Vec3::new(-0.05 - i as f32 * 0.1, 0.05 + j as f32 * 0.1, 0.0),
                0.06,
            ));
        }
    }
    RigidPart {
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_rotation_translation(turn, at),
        rest: [
            Vec3::new(0.0, 0.0, -0.05),
            Vec3::new(0.0, 0.0, 0.05),
            Vec3::new(-1.0, 1.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ],
        point_masses: [4.0; 4],
        // The parent node's frame: the leaf's own turn is its node's.
        parent_world: Affine3A::IDENTITY,
        mobility: 0.016,
        spheres,
        origin: at,
        links: vec![
            link(Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, 2.0, 0.0), 0.0),
            link(Vec3::new(-1.0, 0.0, 0.0), turn * tied(), SLACK),
        ],
        mesh: None,
    }
}

fn pending(flags: u32, saved: i32) -> PendingDoor {
    PendingDoor {
        record: Record {
            flags,
            ..Record::default()
        },
        saved,
        handle_height: 1.0,
    }
}

/// A leaf and its glue at `at`, turned `yaw`.
fn door_at(at: Vec3, yaw: f32, flags: u32, saved: i32) -> (Ragdoll, DoorBody) {
    let mut body = Ragdoll::rigid(&[leaf(at, yaw)]);
    let glue = DoorBody::new(&body, &pending(flags, saved), Affine3A::IDENTITY, at)
        .expect("a hinged leaf");
    body.parts[0].inverse_mass = HINGED_MOBILITY;
    body.parts[0].pairing.mobility = HINGED_MOBILITY;
    (body, glue)
}

/// A tick of the solver and then of the door, as the game runs them.
fn tick(body: &mut Ragdoll, glue: &mut DoorBody, n: i32) {
    body.tick(NONE);
    if !body.asleep() {
        glue.tick(body, n * 12, None);
    }
}

/// The leaf reads its shape, frame and shut angle from its links, at any
/// turn of its node.
#[test]
fn a_leaf_at_its_file_position_reads_shut() {
    for yaw in [0.0, 1.0, -2.5] {
        let (body, glue) = door_at(Vec3::new(3.0, 0.0, -2.0), yaw, 0, 0);
        let shape = glue.door.shape;
        assert!(!shape.sliding);
        assert_eq!(shape.sign, 1.0);
        assert!((shape.radius - 100.0).abs() < 0.01, "{}", shape.radius);
        assert!(
            (glue.door.angle - shape.closed()).abs() < 1e-3,
            "{yaw}: {}",
            glue.door.angle
        );
        assert!(glue.door.closed && glue.door.passable);
        assert!((glue.heading(&body) - shape.closed()).abs() < 1e-3);
    }
}

/// The drive's point at the shut angle is where the stop already is, and
/// the point at the open angle is the stop swung through the full range.
#[test]
fn the_drive_point_follows_the_angle() {
    let (body, glue) = door_at(Vec3::ZERO, 0.4, 0, 0);
    let stop = body.parts[0].point(&body.world[glue.stop].on_part);
    let shut = glue.stop_at(&body, glue.door.shape.closed());
    assert!(stop.distance(shut) < 1e-3, "{stop} {shut}");
    let open = glue.stop_at(&body, glue.door.shape.closed() + glue.door.shape.range());
    // Swung through twice the half swing about the hinge, 1 m out.
    let chord = 2.0 * glue.door.shape.half_swing.sin();
    assert!((open.distance(shut) - chord).abs() < 1e-3);
}

/// A shut leaf woken and left alone stays shut: over 1000 ticks the drive
/// holds its angle at the shut angle and nothing drifts.
#[test]
fn a_woken_leaf_left_alone_stays_shut() {
    let (mut body, mut glue) = door_at(Vec3::new(1.0, 0.0, 1.0), 0.7, 0, 0);
    body.wake();
    let start = body.parts[0].positions;
    for n in 0..1000 {
        if n % 200 == 0 {
            body.wake();
        }
        tick(&mut body, &mut glue, n);
        let a = glue.door.angle;
        assert!(a.is_finite());
        assert!(
            (a - glue.door.shape.closed()).abs() < door::CLOSED_SLOP,
            "tick {n}: {a}"
        );
        assert!(glue.door.closed);
    }
    for (now, was) in body.parts[0].positions.iter().zip(start) {
        assert!(now.distance(was) < 0.01, "{now} {was}");
    }
}

/// The anchor of a pull toward the open side: 1 m from the leaf's origin
/// at the grip's height, toward a point swung `open` past shut.
fn pull_anchor(body: &Ragdoll, glue: &DoorBody, open: f32, height: f32) -> Vec3 {
    let origin = glue.origin(body);
    let toward = glue.stop_at(body, glue.door.shape.closed() + open) - origin;
    let flat = Vec3::new(toward.x, 0.0, toward.z).normalize();
    origin + Vec3::Y * height + flat * HAND_REACH
}

/// The hand's link pulls a shut leaf open: the pull count runs out, the
/// leaf leaves the shut state with its unlatch sound and swings toward the
/// cursor's side; letting go removes the link.
#[test]
fn the_hand_pulls_a_shut_leaf_open() {
    let (mut body, mut glue) = door_at(Vec3::ZERO, 0.0, 0, 0);
    let height = 1.0;
    // The grip: 1 m from the origin toward the hit on the leaf.
    let at = pull_anchor(&body, &glue, 0.0, height);
    glue.take_hold(&mut body, at);
    let link = glue.hand_link(&body).cloned().unwrap();
    assert_eq!(link.flags, world_link_iteration_limit());
    assert_eq!(link.last_pass, Some(HAND_LAST_PASS));
    assert!((link.slack - 0.01).abs() < 1e-7 && (link.strength - 0.003).abs() < 1e-9);
    assert!(glue.door.held && !body.asleep());
    let mut opened_at = None;
    for n in 0..600 {
        let anchor = pull_anchor(&body, &glue, -1.2, height);
        glue.pull_to(&mut body, Some(anchor));
        tick(&mut body, &mut glue, n);
        if opened_at.is_none() && !glue.door.closed {
            opened_at = Some(n);
        }
        assert!(glue.door.angle.is_finite());
    }
    let opened = opened_at.expect("the leaf leaves the shut state");
    assert!(opened < 60, "unlatched after {opened} ticks");
    let shut = glue.door.shape.closed();
    assert!(
        glue.door.angle < shut - 0.5,
        "angle {} shut {shut}",
        glue.door.angle
    );
    // It never swings past its stop.
    assert!(glue.door.angle > shut + glue.door.shape.range() - 0.05);
    let links = body.world.len();
    glue.let_go(&mut body);
    assert_eq!(body.world.len(), links - 1);
    assert!(!glue.door.held && glue.hand_link(&body).is_none());
}

fn world_link_iteration_limit() -> u32 {
    crate::body::world_link::ITERATION_LIMIT
}

/// A suppressed frame disables the link and keeps its anchor.
#[test]
fn a_suppressed_pull_disables_the_link() {
    let (mut body, mut glue) = door_at(Vec3::ZERO, 0.0, 0, 0);
    let at = pull_anchor(&body, &glue, 0.0, 1.0);
    glue.take_hold(&mut body, at);
    glue.pull_to(&mut body, None);
    let link = glue.hand_link(&body).unwrap();
    assert!(link.flags & LINK_DISABLED != 0 && link.anchor == at);
    glue.pull_to(&mut body, Some(at + Vec3::X));
    let link = glue.hand_link(&body).unwrap();
    assert!(link.flags & LINK_DISABLED == 0 && link.anchor == at + Vec3::X);
}

/// A latched leaf (its stored word's bit 0) is held shut by its drive
/// however the hand pulls: the held count only falls while unlatched.
#[test]
fn a_latched_leaf_stays_shut_under_a_pull() {
    let (mut body, mut glue) = door_at(Vec3::ZERO, 0.0, LATCHES, 1);
    assert!(glue.latched());
    let at = pull_anchor(&body, &glue, 0.0, 1.0);
    glue.take_hold(&mut body, at);
    for n in 0..300 {
        let anchor = pull_anchor(&body, &glue, -1.2, 1.0);
        glue.pull_to(&mut body, Some(anchor));
        tick(&mut body, &mut glue, n);
        assert!(glue.door.closed, "tick {n}");
    }
}

/// An open leaf is free: it is not driven, and a woken one left alone
/// stays where it is.
#[test]
fn an_open_leaf_rests_where_it_is() {
    let (mut body, mut glue) = door_at(Vec3::ZERO, 0.0, 0, 0);
    let at = pull_anchor(&body, &glue, 0.0, 1.0);
    glue.take_hold(&mut body, at);
    for n in 0..300 {
        let anchor = pull_anchor(&body, &glue, -0.8, 1.0);
        glue.pull_to(&mut body, Some(anchor));
        tick(&mut body, &mut glue, n);
    }
    glue.let_go(&mut body);
    // Let it come to rest, then watch it.
    for n in 300..1300 {
        tick(&mut body, &mut glue, n);
    }
    let angle = glue.door.angle;
    assert!(!glue.door.closed);
    for n in 1300..1600 {
        body.wake();
        tick(&mut body, &mut glue, n);
    }
    assert!(
        (glue.door.angle - angle).abs() < 0.05,
        "{angle} {}",
        glue.door.angle
    );
}

/// In the running game: two leaves under one node register as a double
/// door (each the other's partner, pair layer bits `0x3000`, mobility
/// 0.05), a lone leaf has none, and the leaves sleep shut until a pull.
#[test]
fn registered_leaves_pair_and_sleep_shut() {
    use crate::Physics;
    use crate::body::BodyPlugin;
    use crate::prop::{PendingProp, PropPlugin};
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, BodyPlugin, PropPlugin))
        .insert_resource(Time::<Fixed>::from_hz(crate::STEP_HZ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 30.0,
        )))
        .insert_resource(Physics::default());
    let model = app
        .world_mut()
        .spawn((Transform::default(), GlobalTransform::default()))
        .id();
    let lone = app
        .world_mut()
        .spawn((Transform::default(), GlobalTransform::default()))
        .id();
    let mut leaves = Vec::new();
    for (parent, x) in [(model, 0.0), (model, 3.0), (lone, 8.0)] {
        let at = Vec3::new(x, 0.0, 0.0);
        let node = app
            .world_mut()
            .spawn((
                Transform::from_translation(at),
                GlobalTransform::from_translation(at),
                ChildOf(parent),
            ))
            .id();
        let mut part = leaf(at, 0.0);
        part.entity = node;
        app.world_mut()
            .entity_mut(node)
            .insert((PendingProp(vec![part]), pending(0, 0)));
        leaves.push(node);
    }
    for _ in 0..30 {
        app.update();
    }
    let glue = |e: Entity| app.world().get::<DoorBody>(e).cloned().unwrap();
    let body = |e: Entity| app.world().get::<Ragdoll>(e).unwrap();
    assert_eq!(glue(leaves[0]).partner, Some(leaves[1]));
    assert_eq!(glue(leaves[1]).partner, Some(leaves[0]));
    assert_eq!(glue(leaves[2]).partner, None);
    for (i, &e) in leaves.iter().enumerate() {
        let b = body(e);
        assert_eq!(b.parts[0].pairing.mobility, HINGED_MOBILITY);
        assert_eq!(b.parts[0].pairing.layer & 0x3000 != 0, i < 2);
        assert!(b.asleep());
        assert!(glue(e).door.closed);
        assert!(app.world().get::<PendingDoor>(e).is_none());
    }
}
