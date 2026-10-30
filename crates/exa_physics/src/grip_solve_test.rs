//! Tests for the cross-body grip stage: a held item's body pulled to its
//! hand, a world weld pulling it alone, and a disabled link left alone.

use bevy::math::Affine3A;
use bevy::prelude::*;

use crate::body::{Ragdoll, RigidPart, SolverStage};
use crate::grip::{DISABLED, GripLink, GripLinks, PIVOT};
use crate::grip_solve::{GripGroup, GripSolver};

fn rigid(entity: Entity, at: Vec3, mobility: f32) -> Ragdoll {
    Ragdoll::rigid(&[RigidPart {
        entity,
        parent: None,
        world: Affine3A::from_translation(at),
        rest: [
            Vec3::new(-0.1, 0.0, -0.1),
            Vec3::new(0.1, 0.0, -0.1),
            Vec3::new(0.0, 0.0, 0.1),
            Vec3::new(0.0, 0.2, 0.0),
        ],
        point_masses: [4.0; 4],
        mobility,
        parent_world: Affine3A::IDENTITY,
        spheres: vec![(Vec3::ZERO, 0.1)],
        origin: Vec3::ZERO,
        links: Vec::new(),
        mesh: None,
        triangles: None,
    }])
}

fn pivot(on_item: [f32; 4], on_hand: [f32; 4], shares: (f32, f32)) -> GripLink {
    GripLink {
        item: 0,
        on_item,
        hand: Some(0),
        on_hand,
        slack: 0.0,
        length: 0.0,
        strength: 1.0,
        smoothing: 0.001,
        flags: PIVOT,
        shares,
        anchor: None,
    }
}

fn links(link: GripLink) -> GripLinks {
    GripLinks {
        links: vec![link],
        main: 0,
        second: None,
        arm: None,
        extra: None,
        flip: None,
        flip_point: Vec3::ZERO,
        flip_progress: 0.0,
        flipped: false,
        flip_active: false,
        mode: -1,
        relaxing: false,
        record_dirty: false,
        world_soft: None,
        world_hard: None,
    }
}

fn solver(item: Entity, probe: Entity, links: GripLinks) -> GripSolver {
    GripSolver {
        groups: vec![GripGroup { item, probe, links }],
    }
}

/// A pivot between a 1 cm gap pulls the item by its 2/3 share and the hand
/// by the 1/3 share, both softened by 1/(1 + 0.001·|d|²) with d in cm.
#[test]
fn a_grip_pivot_pulls_both_bodies_by_their_shares() {
    let item = Entity::from_raw_u32(1).unwrap();
    let hand = Entity::from_raw_u32(2).unwrap();
    let mut item_body = rigid(item, Vec3::new(0.0, 0.01, 0.0), 2.0);
    let mut holder = rigid(hand, Vec3::ZERO, 1.0);
    let item_before = item_body.parts[0].positions;
    let hand_before = holder.parts[0].positions;
    let mut stage = solver(
        item,
        hand,
        links(pivot(
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            (2.0 / 3.0, 1.0 / 3.0),
        )),
    );
    stage.links(&mut [&mut item_body, &mut holder], 0);
    // d = 1 cm: the soften is 1/(1.001).
    let softened = 1.0 / 1.001;
    let item_moved = item_body.parts[0].positions[0] - item_before[0];
    let hand_moved = holder.parts[0].positions[0] - hand_before[0];
    assert!(
        (item_moved.y + 0.01 * softened * 2.0 / 3.0).abs() < 1e-7,
        "{item_moved}"
    );
    assert!(
        (hand_moved.y - 0.01 * softened / 3.0).abs() < 1e-7,
        "{hand_moved}"
    );
    // The two points meet after the softened shares: the gap closes by the
    // soften alone.
    let item_point = item_body.point_at(0, &[1.0, 0.0, 0.0, 0.0]).unwrap();
    let hand_point = holder.point_at(0, &[1.0, 0.0, 0.0, 0.0]).unwrap();
    assert!((hand_point - item_point).length() < 0.001);
}

/// A disabled link moves nothing; a world link pulls the item alone to its
/// stored anchor.
#[test]
fn a_disabled_link_is_skipped_and_a_world_link_moves_the_item_alone() {
    let item = Entity::from_raw_u32(1).unwrap();
    let hand = Entity::from_raw_u32(2).unwrap();
    let mut item_body = rigid(item, Vec3::new(0.0, 0.01, 0.0), 1.0);
    let mut holder = rigid(hand, Vec3::ZERO, 1.0);
    let before = holder.parts[0].positions;
    let mut disabled = pivot([1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], (0.5, 0.5));
    disabled.flags |= DISABLED;
    let mut stage = solver(item, hand, links(disabled));
    stage.links(&mut [&mut item_body, &mut holder], 0);
    assert_eq!(holder.parts[0].positions, before);

    let mut item_body = rigid(item, Vec3::new(0.0, 0.01, 0.0), 1.0);
    let anchor = item_body.point_at(0, &[1.0, 0.0, 0.0, 0.0]).unwrap() + Vec3::new(0.0, 0.01, 0.0);
    let mut world = pivot([1.0, 0.0, 0.0, 0.0], [0.0; 4], (1.0, 0.0));
    world.hand = None;
    world.anchor = Some(anchor);
    let mut stage = solver(item, hand, links(world));
    let hand_before = holder.parts[0].positions;
    stage.links(&mut [&mut item_body, &mut holder], 0);
    assert_eq!(holder.parts[0].positions, hand_before);
    let point = item_body.point_at(0, &[1.0, 0.0, 0.0, 0.0]).unwrap();
    assert!((point - anchor).length() < 0.001);
}

/// A dropped item keeps every state bit it had except the strike flags the
/// attach set, takes the dropped weapon's friction and stays a free body.
#[test]
fn a_dropped_item_clears_its_strike_flags() {
    use crate::grip::{GripRelease, GripSpec, HeldGrip, HeldKind};
    use crate::part::PartState;
    let item = Entity::from_raw_u32(20).unwrap();
    let mut body = rigid(item, Vec3::ZERO, 1.0);
    body.parts[0].state = PartState(
        PartState::HELD | PartState::STRIKING | PartState::STRIKE_CHAIN | PartState::STRIKE_MARK,
    );
    let grip = HeldGrip {
        spec: GripSpec {
            hand: 0,
            links: Vec::new(),
            second_hand: None,
            arm: None,
            extra: None,
        },
        scale: 1.0,
        size: 1.0,
        second_hand_factor: 1.0,
        flip: false,
        class: 0x10,
        soft_weld: false,
        kind: HeldKind::Weapon,
        holder_key: item,
        hands: [None, None],
        arms: [None, None],
        links: Some(links(pivot(
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            (1.0, 0.0),
        ))),
        released: false,
    };
    let mut app = App::new();
    app.add_systems(Update, crate::grip_solve::release_freed_grips);
    let entity = app.world_mut().spawn((body, grip, GripRelease)).id();
    app.update();
    let body = app.world().get::<Ragdoll>(entity).unwrap();
    let state = body.parts[0].state;
    assert!(state.has(PartState::HELD));
    assert!(!state.has(PartState::STRIKING));
    assert!(!state.has(PartState::STRIKE_CHAIN));
    assert!(!state.has(PartState::STRIKE_MARK));
    assert_eq!(body.parts[0].friction, 0.75);
    assert!(body.held_by.is_none());
    assert!(app.world().get::<HeldGrip>(entity).is_none());
    assert!(app.world().get::<GripRelease>(entity).is_none());
}

/// A torch and a shield take the held and mark bits (`0x20002`) when they
/// are held; a weapon takes the weapon bit and the mark bit (`0x20001`).
#[test]
fn a_held_torch_takes_the_held_and_mark_bits() {
    use crate::grip::{HeldGrip, HeldKind};
    use crate::part::PartState;
    let item = Entity::from_raw_u32(20).unwrap();
    let grip = |kind| HeldGrip {
        spec: crate::grip::GripSpec {
            hand: 0,
            links: Vec::new(),
            second_hand: None,
            arm: None,
            extra: None,
        },
        scale: 1.0,
        size: 1.0,
        second_hand_factor: 1.0,
        flip: false,
        class: 0,
        soft_weld: true,
        kind,
        holder_key: item,
        hands: [None, None],
        arms: [None, None],
        links: Some(links(pivot(
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            (1.0, 0.0),
        ))),
        released: false,
    };
    for (kind, word) in [
        (HeldKind::Torch, PartState::HELD | PartState::STRIKE_MARK),
        (HeldKind::Shield, PartState::HELD | PartState::STRIKE_MARK),
        (
            HeldKind::Weapon,
            PartState::STRIKING | PartState::STRIKE_MARK,
        ),
    ] {
        let mut app = App::new();
        app.add_systems(Update, crate::grip_solve::mark_held);
        let entity = app
            .world_mut()
            .spawn((rigid(item, Vec3::ZERO, 1.0), grip(kind)))
            .id();
        app.update();
        let body = app.world().get::<Ragdoll>(entity).unwrap();
        assert_eq!(body.parts[0].state.0 & word, word, "{kind:?}");
        assert!(body.held_by.is_some());
    }
}

/// A world weld in a living holder is a limit link: it moves the item one
/// strength centimetre a pass, or `strength·10/|d|` centimetres past 10 cm.
/// The centroid weights make each move a pure translation, so the item's
/// displacement is the link's correction.
#[test]
fn a_living_world_weld_steps_by_its_strength() {
    let w = [1.0, 0.0, 0.0, 0.0];
    let item = Entity::from_raw_u32(1).unwrap();
    let hand = Entity::from_raw_u32(2).unwrap();
    let mut item_body = rigid(item, Vec3::ZERO, 1.0);
    let mut holder = rigid(hand, Vec3::ZERO, 1.0).with_drive();
    let start = item_body.point_at(0, &w).unwrap();
    let mut weld = pivot(w, [0.0; 4], (1.0, 0.0));
    weld.hand = None;
    weld.anchor = Some(start + Vec3::new(0.0, 0.01, 0.0));
    weld.strength = 0.05;
    let mut stage = solver(item, hand, links(weld.clone()));
    stage.links(&mut [&mut item_body, &mut holder], 0);
    // 1 cm: 0.05 cm of the gap, 0.0005 m.
    let point = item_body.point_at(0, &w).unwrap();
    assert!((point.y - (start.y + 0.0005)).abs() < 1e-7, "{point}");

    // 20 cm: strength·10/20 = 0.025 cm, 0.00025 m.
    let mut item_body = rigid(item, Vec3::ZERO, 1.0);
    weld.anchor = Some(start + Vec3::new(0.0, 0.2, 0.0));
    let mut stage = solver(item, hand, links(weld));
    stage.links(&mut [&mut item_body, &mut holder], 0);
    let point = item_body.point_at(0, &w).unwrap();
    assert!((point.y - (start.y + 0.00025)).abs() < 1e-7, "{point}");

    // A dead holder (no muscles) takes the plain world law: 1 cm at
    // strength 1, 10·1²/20 = 0.5 cm at 20 cm.
    let mut holder = rigid(hand, Vec3::ZERO, 1.0);
    let mut item_body = rigid(item, Vec3::ZERO, 1.0);
    let mut weld = pivot(w, [0.0; 4], (1.0, 0.0));
    weld.hand = None;
    weld.anchor = Some(start + Vec3::new(0.0, 0.01, 0.0));
    let mut stage = solver(item, hand, links(weld.clone()));
    stage.links(&mut [&mut item_body, &mut holder], 0);
    let point = item_body.point_at(0, &w).unwrap();
    assert!((point.y - (start.y + 0.01)).abs() < 1e-7, "{point}");
    let mut item_body = rigid(item, Vec3::ZERO, 1.0);
    weld.anchor = Some(start + Vec3::new(0.0, 0.2, 0.0));
    let mut stage = solver(item, hand, links(weld));
    stage.links(&mut [&mut item_body, &mut holder], 0);
    let point = item_body.point_at(0, &w).unwrap();
    assert!((point.y - (start.y + 0.005)).abs() < 1e-6, "{point}");
}

/// A living holder's hand link is a living pivot: the gap closes by
/// 1/(1 + 0.001·|d|_cm), not the squared softening of the limp law.
#[test]
fn a_living_hand_link_softens_by_its_gap_in_centimetres() {
    let w = [1.0, 0.0, 0.0, 0.0];
    let item = Entity::from_raw_u32(1).unwrap();
    let hand = Entity::from_raw_u32(2).unwrap();
    let mut item_body = rigid(item, Vec3::ZERO, 1.0);
    let mut holder = rigid(hand, Vec3::new(0.1, 0.0, 0.0), 1.0).with_drive();
    let start = item_body.point_at(0, &w).unwrap();
    let mut stage = solver(item, hand, links(pivot(w, w, (1.0, 0.0))));
    stage.links(&mut [&mut item_body, &mut holder], 0);
    // 10 cm: 1/1.01 of the gap, 0.09901 m. The limp law would give 1/1.1.
    let point = item_body.point_at(0, &w).unwrap();
    assert!((point.x - (start.x + 0.1 / 1.01)).abs() < 1e-6, "{point}");
}

/// The builder turns a held item and its character body into the grip's
/// links: the table's one main pivot, then the stiff disabled origin weld
/// (an ordinary item has no soft weld).
#[test]
fn a_held_items_links_are_built_from_its_bodies() {
    use crate::body::PartSetup;
    use crate::grip::{GripPoint, GripSpec, HeldGrip};

    let owner = Entity::from_raw_u32(10).unwrap();
    let hand = Entity::from_raw_u32(11).unwrap();
    let item = Entity::from_raw_u32(20).unwrap();
    let setup = |bone, entity, parent, at| PartSetup {
        bone,
        entity,
        parent,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: None,
    };
    let holder = Ragdoll::new(&[
        setup(1, owner, None, Vec3::ZERO),
        setup(17, hand, Some(0), Vec3::new(0.2, 0.0, 0.0)),
    ])
    .with_drive();
    let item_body = rigid(item, Vec3::new(0.2, 0.4, 0.0), 1.0);
    let grip = HeldGrip {
        spec: GripSpec {
            hand: 0,
            links: vec![GripPoint {
                point: Vec3::new(0.0, 20.0, 0.0),
                slack: 0.0,
            }],
            second_hand: None,
            arm: None,
            extra: None,
        },
        scale: 1.0,
        size: 1.0,
        second_hand_factor: 1.0,
        flip: false,
        class: 0x10,
        soft_weld: false,
        kind: crate::grip::HeldKind::Weapon,
        holder_key: owner,
        hands: [Some(hand), None],
        arms: [Some(hand), None],
        links: None,
        released: false,
    };
    let mut app = App::new();
    app.add_systems(Update, crate::grip_solve::build_grips);
    let holder_e = app.world_mut().spawn(holder).id();
    let item_e = app.world_mut().spawn((item_body, grip)).id();
    app.update();
    let grip = app.world().get::<HeldGrip>(item_e).unwrap();
    let links = grip.links.as_ref().expect("the links were not built");
    assert_eq!(links.links.len(), 2);
    assert_eq!(links.main, 0);
    assert_eq!(links.world_soft, None);
    assert_eq!(links.world_hard, Some(1));
    assert_eq!(links.links[1].flags & DISABLED, DISABLED);
    assert!(links.links[0].enabled());
    assert!(holder_e != Entity::PLACEHOLDER);
}

/// Inside a slack link the limp law is a damper on the smoothed length:
/// a 2 cm gap against a 5 cm length takes back 3 cm over the gap, and the
/// length follows a thousandth of the way to the gap.
#[test]
fn a_slack_grip_link_damps_its_length() {
    let item = Entity::from_raw_u32(1).unwrap();
    let hand = Entity::from_raw_u32(2).unwrap();
    let mut item_body = rigid(item, Vec3::ZERO, 1.0);
    let mut holder = rigid(hand, Vec3::new(0.02, 0.0, 0.0), 1.0);
    let mut link = pivot([1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], (0.5, 0.5));
    link.slack = 0.05;
    link.length = 0.05;
    link.smoothing = 0.001;
    let mut stage = solver(item, hand, links(link));
    stage.links(&mut [&mut item_body, &mut holder], 0);
    let link = &stage.groups[0].links.links[0];
    assert!(
        link.length < 0.0201 && link.length > 0.0199,
        "{}",
        link.length
    );
    let item_point = item_body.point_at(0, &[1.0, 0.0, 0.0, 0.0]).unwrap();
    assert!((item_point.x + 0.115).abs() < 1e-5, "{item_point}");
}

/// The cross-body stage keeps a held item at its hand over a run of solver
/// ticks: the item's grip point stays within a few centimetres of the
/// hand's while both fall.
#[test]
fn a_held_item_stays_at_its_hand_over_ticks() {
    use crate::body::{PartSetup, SolverStage, solve_with};
    use crate::grip::{GripPoint, GripSpec};
    use bevy::math::Affine3A;

    let owner = Entity::from_raw_u32(10).unwrap();
    let hand = Entity::from_raw_u32(11).unwrap();
    let item = Entity::from_raw_u32(20).unwrap();
    let setup = |bone, entity, parent, at| PartSetup {
        bone,
        entity,
        parent,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: None,
    };
    let mut holder = Ragdoll::new(&[
        setup(1, owner, None, Vec3::ZERO),
        setup(17, hand, Some(0), Vec3::new(0.3, 0.0, 0.0)),
    ]);
    let mut item_body = rigid(item, Vec3::new(0.3, 0.0, 0.0), 1.0);
    let bindings = crate::grip::GripBindings {
        item_part: 0,
        hand_parts: [1, 1],
        arm_parts: [1, 1],
    };
    let spec = GripSpec {
        hand: 0,
        links: vec![
            GripPoint {
                point: Vec3::new(0.0, 20.0, 0.0),
                slack: 0.0,
            },
            GripPoint {
                point: Vec3::new(5.0, 0.0, 0.0),
                slack: 0.0,
            },
        ],
        second_hand: None,
        arm: None,
        extra: None,
    };
    let item_to_hand = holder.parts[1].transform().inverse() * item_body.parts[0].transform();
    let links = crate::grip::build(
        &item_body,
        &holder,
        bindings,
        &spec,
        1.0,
        1.0,
        item_to_hand,
        false,
        1.0,
    )
    .unwrap();
    let check = links.links[0].clone();
    let stage = solver(item, hand, links);
    let mut stages: Vec<Box<dyn SolverStage>> = vec![Box::new(stage)];
    let ground: &[[Vec3; 3]] = &[];
    for _ in 0..50 {
        solve_with(&mut [&mut item_body, &mut holder], ground, &mut stages);
        let a = item_body.point_at(check.item, &check.on_item).unwrap();
        let b = holder
            .point_at(check.hand.unwrap(), &check.on_hand)
            .unwrap();
        assert!((a.distance(b)) < 0.05, "{a} {b}");
    }
}

/// The stage works on a copy of the links; after the tick a slack link's
/// smoothed length is carried back to the item's grip, so the damper keeps
/// its state from tick to tick.
#[test]
fn a_grip_links_smoothed_length_carries_over_ticks() {
    use crate::body::SolverStages;
    use crate::grip::{GripSpec, HeldGrip};

    let hand = Entity::from_raw_u32(2).unwrap();
    let mut link = pivot([1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], (0.5, 0.5));
    link.slack = 0.05;
    link.length = 0.05;
    let grip = HeldGrip {
        spec: GripSpec {
            hand: 0,
            links: Vec::new(),
            second_hand: None,
            arm: None,
            extra: None,
        },
        scale: 1.0,
        size: 1.0,
        second_hand_factor: 1.0,
        flip: false,
        class: 0x10,
        soft_weld: false,
        kind: crate::grip::HeldKind::Weapon,
        holder_key: hand,
        hands: [Some(hand), None],
        arms: [None, None],
        links: Some(links(link.clone())),
        released: false,
    };
    let mut app = App::new();
    let item_e = app.world_mut().spawn(grip).id();
    let mut solved = link;
    solved.length = 0.0321;
    let mut stages = SolverStages::default();
    stages.add(solver(item_e, hand, links(solved)));
    app.insert_resource(stages)
        .add_systems(Update, crate::grip_solve::store_grip_lengths);
    app.update();
    let grip = app.world().get::<HeldGrip>(item_e).unwrap();
    assert_eq!(grip.links.as_ref().unwrap().links[0].length, 0.0321);
}

/// A shield item takes the shield builder's three face pivots and its soft
/// world weld, not a weapon's table links.
#[test]
fn a_shield_item_builds_its_face_pivots() {
    use crate::body::PartSetup;
    use crate::grip::{GripSpec, HeldGrip};

    let owner = Entity::from_raw_u32(10).unwrap();
    let hand = Entity::from_raw_u32(11).unwrap();
    let item = Entity::from_raw_u32(20).unwrap();
    let setup = |bone, entity, parent, at| PartSetup {
        bone,
        entity,
        parent,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: None,
    };
    let holder = Ragdoll::new(&[
        setup(1, owner, None, Vec3::ZERO),
        setup(17, hand, Some(0), Vec3::new(0.2, 0.0, 0.0)),
    ]);
    let item_body = rigid(item, Vec3::new(0.2, 0.2, 0.0), 1.0);
    let grip = HeldGrip {
        spec: GripSpec {
            hand: 0,
            links: Vec::new(),
            second_hand: None,
            arm: None,
            extra: None,
        },
        scale: 1.0,
        size: 1.0,
        second_hand_factor: 1.0,
        flip: false,
        class: 0x10,
        soft_weld: false,
        kind: crate::grip::HeldKind::Shield,
        holder_key: owner,
        hands: [Some(hand), Some(hand)],
        arms: [Some(hand), Some(hand)],
        links: None,
        released: false,
    };
    let mut app = App::new();
    app.add_systems(Update, crate::grip_solve::build_grips);
    app.world_mut().spawn(holder);
    let item_e = app.world_mut().spawn((item_body, grip)).id();
    app.update();
    let grip = app.world().get::<HeldGrip>(item_e).unwrap();
    let links = grip
        .links
        .as_ref()
        .expect("the shield links were not built");
    assert_eq!(links.links.len(), 4);
    assert_eq!(links.world_soft, Some(3));
    assert_eq!(links.world_hard, None);
    for link in &links.links[..3] {
        assert_eq!(link.flags, PIVOT | crate::grip::RUNTIME);
    }
    assert_eq!(links.links[3].flags, crate::grip::RUNTIME | DISABLED);
}
