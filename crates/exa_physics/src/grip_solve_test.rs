//! Tests for the cross-body grip stage: a held item's body pulled to its
//! hand, a world weld pulling it alone, and a disabled link left alone.

use bevy::math::Affine3A;
use bevy::prelude::*;

use crate::body::{Ragdoll, RigidPart, SolverStage};
use crate::grip::{DISABLED, GripLink, GripLinks, PIVOT, RUNTIME};
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
        normal: Vec3::ZERO,
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
        motion: -1,
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

/// A plane link (the shield's parry link) pulls the item by its offset
/// less 0.8 of its part along the link's axis: an anchor 1 cm off along
/// the axis and 1 cm across it moves the point 0.2 cm and 1 cm.
#[test]
fn a_plane_link_keeps_a_fifth_of_its_pull_along_its_axis() {
    use crate::grip::PLANE;
    let item = Entity::from_raw_u32(1).unwrap();
    let hand = Entity::from_raw_u32(2).unwrap();
    let mut item_body = rigid(item, Vec3::new(0.0, 0.01, 0.0), 1.0);
    let mut holder = rigid(hand, Vec3::ZERO, 1.0);
    let start = item_body.point_at(0, &[1.0, 0.0, 0.0, 0.0]).unwrap();
    let mut plane = pivot([1.0, 0.0, 0.0, 0.0], [0.0; 4], (1.0, 0.0));
    plane.hand = None;
    plane.flags = PLANE;
    plane.anchor = Some(start + Vec3::new(0.01, 0.0, 0.01));
    plane.normal = Vec3::X;
    let mut stage = solver(item, hand, links(plane));
    stage.links(&mut [&mut item_body, &mut holder], 0);
    let moved = item_body.point_at(0, &[1.0, 0.0, 0.0, 0.0]).unwrap() - start;
    assert!(
        (moved - Vec3::new(0.002, 0.0, 0.01)).length() < 1e-4,
        "{moved}"
    );
}

/// A dropped item keeps every state bit it had except the strike flags the
/// attach set, takes the dropped weapon's friction and stays a free body,
/// the root of its own tree (link iterations 2 + its parts − 1), its top
/// part's contact mobility back at its base; its holder, still in the
/// world, hears that it came loose.
#[test]
fn a_dropped_item_clears_its_strike_flags() {
    use crate::grip::{GripRelease, GripSpec, HeldGrip, HeldKind};
    use crate::part::PartState;
    let item = Entity::from_raw_u32(20).unwrap();
    let holder_part = Entity::from_raw_u32(21).unwrap();
    let mut body = rigid(item, Vec3::ZERO, 1.0);
    body.set_link_iterations(15);
    body.parts[0].state = PartState(
        PartState::HELD | PartState::STRIKING | PartState::STRIKE_CHAIN | PartState::STRIKE_MARK,
    );
    // The arm setter's scaled contact mobility while it was held.
    let base = body.parts[0].inverse_mass;
    body.parts[0].pairing.mobility = base * 0.3;
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
        holder_key: holder_part,
        hands: [None, None],
        arms: [None, None],
        links: Some(links(pivot(
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            (1.0, 0.0),
        ))),
        released: false,
        head_extra: 0.0,
    };
    let torch = HeldGrip {
        kind: HeldKind::Torch,
        ..grip.clone()
    };
    let mut app = App::new();
    app.add_message::<crate::grip::CameLoose>()
        .add_systems(Update, crate::grip_solve::release_freed_grips);
    let holder = app
        .world_mut()
        .spawn(rigid(holder_part, Vec3::ZERO, 1.0))
        .id();
    let entity = app.world_mut().spawn((body, grip, GripRelease)).id();
    app.update();
    let loose: Vec<crate::grip::CameLoose> = app
        .world()
        .resource::<Messages<crate::grip::CameLoose>>()
        .iter_current_update_messages()
        .copied()
        .collect();
    assert_eq!(
        loose,
        vec![crate::grip::CameLoose {
            item: entity,
            holder
        }]
    );
    let body = app.world().get::<Ragdoll>(entity).unwrap();
    assert_eq!(body.link_iterations(), 2);
    let state = body.parts[0].state;
    assert!(state.has(PartState::HELD));
    assert!(!state.has(PartState::STRIKING));
    assert!(!state.has(PartState::STRIKE_CHAIN));
    assert!(!state.has(PartState::STRIKE_MARK));
    assert_eq!(body.parts[0].friction, 0.75);
    // The drop puts the contact mobility back at its base.
    assert_eq!(body.parts[0].pairing.mobility, base);
    assert!(body.held_by.is_none());
    assert!(app.world().get::<HeldGrip>(entity).is_none());
    assert!(app.world().get::<GripRelease>(entity).is_none());
    // A torch let go of tells its holder nothing.
    let torch_body = rigid(Entity::from_raw_u32(22).unwrap(), Vec3::ZERO, 1.0);
    let freed = app.world_mut().spawn((torch_body, torch, GripRelease)).id();
    app.update();
    assert!(app.world().get::<HeldGrip>(freed).is_none());
    assert_eq!(
        app.world()
            .resource::<Messages<crate::grip::CameLoose>>()
            .iter_current_update_messages()
            .count(),
        0
    );
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
        head_extra: 0.0,
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
        let friction = if kind == HeldKind::Torch { 0.0 } else { 0.5 };
        assert_eq!(body.parts[0].friction, friction, "{kind:?}");
        // A shield and a torch take their link mobility from their contact
        // mobility and lose the sole bit.
        if kind != HeldKind::Weapon {
            let part = &body.parts[0];
            assert_eq!(part.link_mobility, part.pairing.mobility, "{kind:?}");
            assert!(!part.state.has(PartState::SOLE), "{kind:?}");
        }
    }
}

/// A world weld (no pivot flag) takes the plain link law whether its holder
/// lives or not: `strength·d`, a `10·strength²` cm move at most past
/// `10·strength` cm, or `strength` times the excess past a slack. The
/// centroid weights make each move a pure translation, so the item's
/// displacement is the link's correction.
#[test]
fn a_world_weld_takes_the_plain_link_law() {
    let w = [1.0, 0.0, 0.0, 0.0];
    let item = Entity::from_raw_u32(1).unwrap();
    let hand = Entity::from_raw_u32(2).unwrap();
    let start = rigid(item, Vec3::ZERO, 1.0).point_at(0, &w).unwrap();
    let pass = |holder: &mut Ragdoll, strength: f32, slack: f32, gap: f32| {
        let mut item_body = rigid(item, Vec3::ZERO, 1.0);
        let mut weld = pivot(w, [0.0; 4], (1.0, 0.0));
        weld.hand = None;
        weld.flags = RUNTIME;
        weld.anchor = Some(start + Vec3::new(0.0, gap, 0.0));
        weld.strength = strength;
        weld.slack = slack;
        let mut stage = solver(item, hand, links(weld));
        stage.links(&mut [&mut item_body, holder], 0);
        item_body.point_at(0, &w).unwrap().y - start.y
    };
    for mut holder in [
        rigid(hand, Vec3::ZERO, 1.0),
        rigid(hand, Vec3::ZERO, 1.0).with_drive(),
    ] {
        // Strength 1: 5 cm closes in full (inside the 10 cm reach); 20 cm
        // moves by the 10·1² cm cap.
        assert!((pass(&mut holder, 1.0, 0.0, 0.05) - 0.05).abs() < 1e-6);
        assert!((pass(&mut holder, 1.0, 0.0, 0.2) - 0.1).abs() < 1e-6);
        // The soft weld's 0.05: 5% of a 1 mm gap (inside the 0.5 cm reach),
        // but only 10·0.05² = 0.025 cm of a 20 cm one.
        assert!((pass(&mut holder, 0.05, 0.0, 0.001) - 0.000_05).abs() < 1e-8);
        assert!((pass(&mut holder, 0.05, 0.0, 0.2) - 0.000_25).abs() < 1e-7);
        // A 10 cm slack: an 8 cm gap is inside; a 15 cm one takes back
        // 0.05 of its 5 cm excess.
        assert_eq!(pass(&mut holder, 0.05, 0.1, 0.08), 0.0);
        assert!((pass(&mut holder, 0.05, 0.1, 0.15) - 0.0025).abs() < 1e-7);
    }
}

/// A hand link is a pivot in the body solver's plain iteration whether the
/// holder lives or not (the original's held item never takes the living
/// part bit): a 10 cm gap closes by 1/(1 + 0.001·10²) = 1/1.1.
#[test]
fn a_living_holders_hand_link_keeps_the_plain_pivot_law() {
    let w = [1.0, 0.0, 0.0, 0.0];
    let item = Entity::from_raw_u32(1).unwrap();
    let hand = Entity::from_raw_u32(2).unwrap();
    for mut holder in [
        rigid(hand, Vec3::new(0.1, 0.0, 0.0), 1.0),
        rigid(hand, Vec3::new(0.1, 0.0, 0.0), 1.0).with_drive(),
    ] {
        let mut item_body = rigid(item, Vec3::ZERO, 1.0);
        let start = item_body.point_at(0, &w).unwrap();
        let mut stage = solver(item, hand, links(pivot(w, w, (1.0, 0.0))));
        stage.links(&mut [&mut item_body, &mut holder], 0);
        let point = item_body.point_at(0, &w).unwrap();
        assert!((point.x - (start.x + 0.1 / 1.1)).abs() < 1e-6, "{point}");
    }
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
        head_extra: 0.0,
    };
    let mut app = App::new();
    app.add_systems(Update, crate::grip_solve::build_grips);
    let holder_e = app.world_mut().spawn(holder).id();
    let item_e = app.world_mut().spawn((item_body, grip)).id();
    app.update();
    let grip = app.world().get::<HeldGrip>(item_e).unwrap();
    let links = grip.links.as_ref().expect("the links were not built");
    // The attach's origin pivot first, then the table's pivot and the
    // stiff world weld.
    assert_eq!(links.links.len(), 3);
    assert_eq!(links.links[0].flags, PIVOT | crate::grip::RUNTIME);
    assert_eq!(links.links[0].slack, 0.0);
    assert_eq!(links.main, 1);
    assert_eq!(links.world_soft, None);
    assert_eq!(links.world_hard, Some(2));
    assert_eq!(links.links[2].flags & DISABLED, DISABLED);
    assert!(links.links[0].enabled() && links.links[1].enabled());
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
        head_extra: 0.0,
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
        head_extra: 0.0,
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
    // The origin pivot to the forearm, the three face pivots, the soft
    // world weld.
    assert_eq!(links.links.len(), 5);
    assert_eq!(links.main, 1);
    assert_eq!(links.world_soft, Some(4));
    assert_eq!(links.world_hard, None);
    for link in &links.links[..4] {
        assert_eq!(link.flags, PIVOT | crate::grip::RUNTIME);
    }
    assert_eq!(links.links[4].flags, crate::grip::RUNTIME | DISABLED);
}

/// The re-place after a move: a body moved a metre leaves its held item
/// behind; the re-place puts the item back at the place its attach
/// measured under the hand, at rest, and an armed grip's enabled
/// second-hand link takes the span it now has as its slack and length.
#[test]
fn a_moved_holder_takes_its_held_item_along() {
    use crate::body::PartSetup;
    use crate::grip::{ENABLED, GripPoint, GripSpec, HeldGrip, HeldKind, HeldRest, ReplaceHeld};

    let owner = Entity::from_raw_u32(10).unwrap();
    let hand = Entity::from_raw_u32(11).unwrap();
    let other = Entity::from_raw_u32(12).unwrap();
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
        setup(18, other, Some(0), Vec3::new(-0.2, 0.0, 0.0)),
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
        class: 0x30,
        soft_weld: false,
        kind: HeldKind::Weapon,
        holder_key: owner,
        hands: [Some(hand), Some(other)],
        arms: [Some(hand), None],
        links: None,
        released: false,
        head_extra: 0.0,
    };
    let mut app = App::new();
    app.add_systems(
        Update,
        (
            crate::grip_solve::build_grips,
            crate::grip_solve::replace_held,
        )
            .chain(),
    );
    let holder_e = app.world_mut().spawn(holder).id();
    let item_e = app.world_mut().spawn((item_body, grip)).id();
    app.update();
    let rest = *app.world().get::<HeldRest>(item_e).expect("no rest kept");
    assert_eq!(rest.hand, 1);
    assert!(!rest.flipped);
    // A second-hand link, enabled, from the item's origin to the left hand.
    {
        let mut grip = app.world_mut().get_mut::<HeldGrip>(item_e).unwrap();
        let links = grip.links.as_mut().unwrap();
        let mut second = links.links[0].clone();
        second.hand = Some(2);
        second.on_item = [1.0, 0.0, 0.0, 0.0];
        second.on_hand = [1.0, 0.0, 0.0, 0.0];
        second.slack = 0.0;
        second.flags = ENABLED;
        links.second = Some(links.links.len());
        links.links.push(second);
    }
    let before = app.world().get::<Ragdoll>(item_e).unwrap().parts[0].transform();
    app.world_mut()
        .get_mut::<Ragdoll>(holder_e)
        .unwrap()
        .translate(Vec3::new(1.0, 0.0, 0.5));
    app.world_mut().entity_mut(holder_e).insert(ReplaceHeld);
    app.update();
    let body = app.world().get::<Ragdoll>(item_e).unwrap();
    let after = body.parts[0].transform();
    let moved = Vec3::from(after.translation - before.translation);
    assert!(
        (moved - Vec3::new(1.0, 0.0, 0.5)).length() < 1e-4,
        "{moved}"
    );
    assert_eq!(body.parts[0].positions, body.parts[0].previous);
    assert!(app.world().get::<ReplaceHeld>(holder_e).is_none());
    // The second-hand link spans item origin to the left hand now.
    let holder = app.world().get::<Ragdoll>(holder_e).unwrap();
    let grip = app.world().get::<HeldGrip>(item_e).unwrap();
    let links = grip.links.as_ref().unwrap();
    let link = &links.links[links.second.unwrap()];
    let a = body.point_at(0, &link.on_item).unwrap();
    let b = holder.point_at(2, &link.on_hand).unwrap();
    assert!((link.slack - (a - b).length()).abs() < 1e-5);
    assert_eq!(link.slack, link.length);
    assert!(link.slack > 0.1);
}

/// A flip since the attach turns the re-placed item half a turn about its
/// own Y axis.
#[test]
fn a_flipped_items_rest_turns_half_a_turn() {
    use crate::grip::HeldRest;
    let rest = HeldRest {
        hand: 0,
        item_in_hand: Affine3A::from_translation(Vec3::new(0.0, 0.1, 0.0)),
        flipped: false,
    };
    let hand = Affine3A::from_translation(Vec3::new(1.0, 0.0, 0.0));
    let plain = rest.placement(hand, false);
    let turned = rest.placement(hand, true);
    assert_eq!(plain.translation, turned.translation);
    let x = turned.transform_vector3(Vec3::X);
    assert!((x + Vec3::X).length() < 1e-6);
}

/// A held item hangs under its hand by the attach's origin pivot and the
/// grip's two points: turned about the line through the two points, it is
/// brought back, as three pivots hold a rigid body still. Without the
/// origin pivot a two-point grip leaves that turn free and the item swings
/// away from the palm.
#[test]
fn the_origin_pivot_holds_the_item_still_under_its_hand() {
    use crate::grip::{GripBindings, origin_pivot};
    let item_e = Entity::from_raw_u32(1).unwrap();
    let hand_e = Entity::from_raw_u32(2).unwrap();
    let item = rigid(item_e, Vec3::new(0.0, 0.0, 0.0), 1.0);
    let hand = rigid(hand_e, Vec3::new(0.0, 0.0, 0.0), 0.0);
    let bindings = GripBindings {
        item_part: 0,
        hand_parts: [0, 0],
        arm_parts: [0, 0],
    };
    let spec = crate::grip::GripSpec {
        hand: 0,
        links: vec![
            crate::grip::GripPoint {
                point: Vec3::new(0.0, 20.0, 0.0),
                slack: 0.0,
            },
            crate::grip::GripPoint {
                point: Vec3::new(15.0, 0.0, 0.0),
                slack: 0.0,
            },
        ],
        second_hand: None,
        arm: None,
        extra: None,
    };
    let solve = |with_pivot: bool| {
        let built = crate::grip::build(
            &item,
            &hand,
            bindings,
            &spec,
            1.0,
            1.0,
            Affine3A::IDENTITY,
            false,
            1.0,
        )
        .unwrap();
        let links = if with_pivot {
            built.with_origin_pivot(origin_pivot(&item, &hand, 0, 0, Affine3A::IDENTITY).unwrap())
        } else {
            built
        };
        // Turn the item 40 degrees about the line through its two grip
        // points, then let the grip stage run.
        let a = Vec3::new(0.0, 0.2, 0.0);
        let b = Vec3::new(0.15, 0.0, 0.0);
        let axis = (b - a).normalize();
        let turn = Affine3A::from_translation(a)
            * Affine3A::from_axis_angle(axis, 40f32.to_radians())
            * Affine3A::from_translation(-a);
        let mut turned = item.clone();
        turned.place_at(&[turn]);
        let mut solver = GripSolver {
            groups: vec![GripGroup {
                item: item_e,
                probe: hand_e,
                links,
            }],
        };
        let mut hand = hand.clone();
        for i in 0..20 {
            let mut bodies = [&mut turned, &mut hand];
            solver.links(&mut bodies, i);
        }
        let origin = turned.weights_at(0, Vec3::ZERO).unwrap();
        turned.point_at(0, &origin).unwrap().length() * 100.0
    };
    // The origin stays off by the turn without the pivot, and comes back
    // with it.
    assert!(solve(false) > 5.0, "the two points alone hold the turn");
    assert!(solve(true) < 0.5, "the origin pivot brings it back");
}

/// A linked grip of `holder_part`'s body on its item.
fn held_weapon(holder_part: Entity) -> crate::grip::HeldGrip {
    use crate::grip::{HeldGrip, HeldKind};
    HeldGrip {
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
        kind: HeldKind::Weapon,
        holder_key: holder_part,
        hands: [None, None],
        arms: [None, None],
        links: Some(links(pivot(
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            (1.0, 0.0),
        ))),
        released: false,
        head_extra: 0.0,
    }
}

/// A held item takes its holder's park mark: parked with a parked holder,
/// unparked once the holder is, and left alone while its holder is not
/// found or the grip is released.
#[test]
fn a_held_item_is_parked_and_unparked_with_its_holder() {
    use crate::grip::HeldGrip;
    let holder_part = Entity::from_raw_u32(30).unwrap();
    let item_part = Entity::from_raw_u32(31).unwrap();
    let mut app = App::new();
    app.add_systems(Update, crate::grip_solve::share_holder_park);
    let holder = app
        .world_mut()
        .spawn(rigid(holder_part, Vec3::ZERO, 1.0))
        .id();
    let item = app
        .world_mut()
        .spawn((rigid(item_part, Vec3::X, 1.0), held_weapon(holder_part)))
        .id();
    let parked = |app: &App, e| app.world().get::<Ragdoll>(e).unwrap().parked();
    app.update();
    assert!(!parked(&app, item));
    app.world_mut().get_mut::<Ragdoll>(holder).unwrap().park();
    app.update();
    assert!(parked(&app, item), "parked with its holder");
    app.world_mut().get_mut::<Ragdoll>(holder).unwrap().unpark();
    app.update();
    assert!(!parked(&app, item), "unparked with its holder");
    // A released grip keeps its own mark.
    app.world_mut().get_mut::<Ragdoll>(holder).unwrap().park();
    app.world_mut().get_mut::<HeldGrip>(item).unwrap().released = true;
    app.update();
    assert!(!parked(&app, item));
}

/// A held item shares its holder's sleep, as the original's body tree
/// shares its root's countdown: it takes the larger of the two countdowns
/// and wakes with an awake holder, falls asleep with a sleeping one, a
/// dormant holder's countdown carries it down in step, and an item woken
/// by a touch wakes its sleeping holder.
#[test]
fn a_held_item_sleeps_and_wakes_with_its_holder() {
    let holder_part = Entity::from_raw_u32(50).unwrap();
    let item_part = Entity::from_raw_u32(51).unwrap();
    let mut app = App::new();
    app.add_systems(Update, crate::grip_solve::share_holder_sleep);
    let mut awake_item = rigid(item_part, Vec3::X, 1.0);
    awake_item.wake();
    let holder = app
        .world_mut()
        .spawn(rigid(holder_part, Vec3::ZERO, 1.0))
        .id();
    let item = app
        .world_mut()
        .spawn((awake_item, held_weapon(holder_part)))
        .id();
    let get = |app: &App, e| {
        let b = app.world().get::<Ragdoll>(e).unwrap();
        (b.countdown(), b.asleep())
    };
    // The item, awake at 256, wakes its sleeping holder.
    app.update();
    assert_eq!(get(&app, holder), (crate::body::WAKE_COUNT, false));
    assert_eq!(get(&app, item), (crate::body::WAKE_COUNT, false));
    // Both count down; the item takes the larger countdown.
    app.world_mut().get_mut::<Ragdoll>(holder).unwrap().take_ticks(40);
    app.world_mut().get_mut::<Ragdoll>(item).unwrap().take_ticks(50);
    app.update();
    assert_eq!(get(&app, item), (crate::body::WAKE_COUNT - 40, false));
    assert_eq!(get(&app, holder).0, crate::body::WAKE_COUNT - 40);
    // Both asleep (as their post-steps leave them): they stay asleep.
    let swap_in = |app: &mut App, e, mut body: Ragdoll| {
        std::mem::swap(
            &mut *app.world_mut().get_mut::<Ragdoll>(e).unwrap(),
            &mut body,
        );
    };
    swap_in(&mut app, holder, rigid(holder_part, Vec3::ZERO, 1.0));
    swap_in(&mut app, item, rigid(item_part, Vec3::X, 1.0));
    app.update();
    assert_eq!(get(&app, item), (0, true));
    assert_eq!(get(&app, holder), (0, true));
    // A sleeping item of an awake holder wakes with it.
    app.world_mut().get_mut::<Ragdoll>(holder).unwrap().wake_to(100);
    app.update();
    assert_eq!(get(&app, item), (100, false));
    // Asleep together again, a touch on the item wakes its holder.
    swap_in(&mut app, holder, rigid(holder_part, Vec3::ZERO, 1.0));
    swap_in(&mut app, item, rigid(item_part, Vec3::X, 1.0));
    app.world_mut().get_mut::<Ragdoll>(item).unwrap().wake();
    app.update();
    assert_eq!(get(&app, holder), (crate::body::WAKE_COUNT, false));
    // A released grip keeps its own sleep.
    swap_in(&mut app, item, rigid(item_part, Vec3::X, 1.0));
    app.world_mut()
        .get_mut::<crate::grip::HeldGrip>(item)
        .unwrap()
        .released = true;
    app.update();
    assert_eq!(get(&app, item), (0, true));
}

/// The live step (the physics plugin's fixed step): a held item is parked
/// with its parked holder and unparked with it, and a sleeping held item
/// of an awake holder is awake after the step (it takes its holder's
/// countdown before the solver), as the original's tree wake and unpark
/// reach the bodies under the character's node.
#[test]
fn the_step_parks_wakes_and_unparks_a_held_item_with_its_holder() {
    let holder_part = Entity::from_raw_u32(40).unwrap();
    let item_part = Entity::from_raw_u32(41).unwrap();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::transform::TransformPlugin,
        crate::PhysicsPlugin,
    ))
    .init_resource::<Assets<Mesh>>();
    app.update();
    let mut awake = rigid(holder_part, Vec3::new(0.0, 5.0, 0.0), 1.0);
    awake.wake();
    let holder = app.world_mut().spawn(awake).id();
    // A rigid body is registered asleep (countdown 0).
    let item_body = rigid(item_part, Vec3::new(0.3, 5.0, 0.0), 1.0);
    assert!(item_body.asleep());
    let item = app
        .world_mut()
        .spawn((item_body, held_weapon(holder_part)))
        .id();
    let step = |app: &mut App| app.world_mut().run_schedule(FixedUpdate);
    let body = |app: &App, e| app.world().get::<Ragdoll>(e).unwrap().clone();
    step(&mut app);
    let woken = body(&app, item);
    assert!(!woken.asleep(), "the held item is awake");
    assert!(woken.countdown() > 1, "countdown {}", woken.countdown());
    app.world_mut().get_mut::<Ragdoll>(holder).unwrap().park();
    step(&mut app);
    assert!(body(&app, item).parked(), "parked with its holder");
    app.world_mut().get_mut::<Ragdoll>(holder).unwrap().unpark();
    step(&mut app);
    assert!(!body(&app, item).parked(), "unparked with its holder");
}

/// The attach and the drops write the item's own (top) body, as the
/// original's: a two-part weapon's child keeps its built friction while held
/// and after the drop, which clears the strike flags over the whole tree; a
/// dropped shield keeps its mark bit (its drop clears no strike flag) and a
/// dropped torch loses its strike flags on its own body.
#[test]
fn the_attach_and_the_drops_write_the_items_own_body() {
    use crate::grip::{GripRelease, GripSpec, HeldGrip, HeldKind};
    use crate::part::PartState;
    let item = Entity::from_raw_u32(30).unwrap();
    let child = Entity::from_raw_u32(31).unwrap();
    let holder_part = Entity::from_raw_u32(32).unwrap();
    let part = |entity, parent| RigidPart {
        entity,
        parent,
        world: Affine3A::IDENTITY,
        rest: [
            Vec3::new(-0.1, 0.0, -0.1),
            Vec3::new(0.1, 0.0, -0.1),
            Vec3::new(0.0, 0.0, 0.1),
            Vec3::new(0.0, 0.2, 0.0),
        ],
        point_masses: [4.0; 4],
        mobility: 1.0,
        parent_world: Affine3A::IDENTITY,
        spheres: vec![(Vec3::ZERO, 0.1)],
        origin: Vec3::ZERO,
        links: Vec::new(),
        mesh: None,
        triangles: None,
    };
    let grip = |kind| HeldGrip {
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
        kind,
        holder_key: holder_part,
        hands: [None, None],
        arms: [None, None],
        links: Some(links(pivot(
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            (1.0, 0.0),
        ))),
        released: false,
        head_extra: 0.0,
    };
    let mut app = App::new();
    app.add_message::<crate::grip::CameLoose>().add_systems(
        Update,
        (
            crate::grip_solve::mark_held,
            crate::grip_solve::release_freed_grips,
        )
            .chain(),
    );
    app.world_mut().spawn(rigid(holder_part, Vec3::ZERO, 1.0));
    let weapon = app
        .world_mut()
        .spawn((
            Ragdoll::rigid(&[part(item, None), part(child, Some(0))]),
            grip(HeldKind::Weapon),
        ))
        .id();
    let built = app.world().get::<Ragdoll>(weapon).unwrap().parts[1].friction;
    app.update();
    let body = app.world().get::<Ragdoll>(weapon).unwrap();
    assert_eq!(body.parts[0].friction, 0.5);
    assert_eq!(body.parts[1].friction, built);
    assert!(body.parts[1].state.has(PartState::STRIKING));
    app.world_mut().entity_mut(weapon).insert(GripRelease);
    app.update();
    let body = app.world().get::<Ragdoll>(weapon).unwrap();
    assert_eq!(body.parts[0].friction, 0.75);
    assert_eq!(body.parts[1].friction, built);
    for p in &body.parts {
        assert!(!p.state.has(PartState::STRIKING | PartState::STRIKE_MARK));
    }
    for (kind, kept) in [(HeldKind::Shield, true), (HeldKind::Torch, false)] {
        let held = app
            .world_mut()
            .spawn((
                rigid(Entity::from_raw_u32(40).unwrap(), Vec3::ZERO, 1.0),
                grip(kind),
            ))
            .id();
        app.update();
        app.world_mut().entity_mut(held).insert(GripRelease);
        app.update();
        let body = app.world().get::<Ragdoll>(held).unwrap();
        assert_eq!(
            body.parts[0].state.has(PartState::STRIKE_MARK),
            kept,
            "{kind:?}"
        );
        assert_eq!(body.parts[0].friction, 0.75, "{kind:?}");
    }
}
