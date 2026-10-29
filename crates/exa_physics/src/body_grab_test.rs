//! The articulated tree's link-mobility blend, the link-share recompute and
//! the breakable grab, pinned to the original's numbers (its centimetres
//! worked by hand).

use bevy::math::Affine3A;

use super::*;
use crate::body::{Ragdoll, RigidPart};

fn rigid_part(at: Vec3, parent: Option<usize>, mobility: f32) -> RigidPart {
    RigidPart {
        entity: Entity::PLACEHOLDER,
        parent,
        world: Affine3A::from_translation(at),
        rest: [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z],
        point_masses: [4.0; 4],
        mobility,
        parent_world: Affine3A::IDENTITY,
        spheres: vec![(Vec3::ZERO, 0.1)],
        origin: at,
        links: Vec::new(),
        mesh: None,
        triangles: None,
    }
}

/// A chain root (1) → middle (0.5) → tip (0.25), and a second child of
/// the root (0.125).
fn chain() -> Ragdoll {
    Ragdoll::rigid(&[
        rigid_part(Vec3::ZERO, None, 1.0),
        rigid_part(Vec3::X, Some(0), 0.5),
        rigid_part(Vec3::X * 2.0, Some(1), 0.25),
        rigid_part(Vec3::Z, Some(0), 0.125),
    ])
}

#[test]
fn each_mask_pass_blends_a_child_with_its_parent() {
    let body = chain();
    let m: Vec<f32> = body.parts.iter().map(|p| p.link_mobility).collect();
    // The middle body's own pass: tip = 0.5·0.5 + 0.25·0.5 = 0.375. The
    // root's pass: tip again with the middle still at 0.5 (0.25 + 0.1875),
    // then middle = 1·0.5 + 0.5·0.5, the second child 0.5 + 0.0625.
    assert_eq!(m, vec![1.0, 0.75, 0.4375, 0.5625]);
    // The contact mobility is untouched.
    let contact: Vec<f32> = body.parts.iter().map(|p| p.inverse_mass).collect();
    assert_eq!(contact, vec![1.0, 0.5, 0.25, 0.125]);
}

#[test]
fn the_link_shares_are_taken_before_the_blend() {
    let mut body = chain();
    // The middle body's links to the root: 0.5/1.5 and 1/1.5.
    let middle = body.links.iter().find(|l| l.part == 1).unwrap();
    assert_eq!(middle.shares, (0.5_f32 / 1.5, 1.0_f32 / 1.5));
    // A recompute reads the blended mobilities: 0.75/1.75 and 1/1.75.
    body.recompute_link_shares();
    let middle = body.links.iter().find(|l| l.part == 1).unwrap();
    assert_eq!(middle.shares, (0.75_f32 / 1.75, 1.0_f32 / 1.75));
    let tip = body.links.iter().find(|l| l.part == 2).unwrap();
    assert_eq!(tip.shares, (0.4375_f32 / 1.1875, 0.75_f32 / 1.1875));
}

#[test]
fn a_disabled_link_keeps_its_shares_on_a_recompute() {
    let mut body = chain();
    let k = body.links.iter().position(|l| l.part == 1).unwrap();
    body.links[k].flags |= LINK_DISABLED;
    body.recompute_link_shares();
    assert_eq!(body.links[k].shares, (0.5_f32 / 1.5, 1.0_f32 / 1.5));
}

#[test]
fn a_soft_weld_splits_by_the_mobilities_as_they_stand() {
    // 0.1/(0.75 + 1) in double: each side its mobility times that.
    let share = (0.1_f64 / 1.75_f64) as f32;
    assert_eq!(weld_shares(0.75, 1.0), (0.75 * share, share));
}

#[test]
fn a_rigid_bodys_link_mobility_is_its_registered_mobility() {
    let body = Ragdoll::rigid(&[rigid_part(Vec3::ZERO, None, 0.5)]);
    // A rigid body's link mobility is its registered mobility.
    assert_eq!(body.link_mobility(0), Some(0.5));
    assert_eq!(body.link_mobility(1), None);
}

/// A weapon part at the origin and a struck part `gap` metres along x,
/// as two free bodies of one solver list.
fn pair(gap: f32) -> Ragdoll {
    Ragdoll::rigid(&[
        rigid_part(Vec3::ZERO, None, 1.0),
        rigid_part(Vec3::X * gap, None, 3.0),
        rigid_part(Vec3::Y * 5.0, None, 1.0),
    ])
}

fn grab(strain: f32, flags: u32, axis: Vec3) -> GrabPull {
    GrabPull {
        strain,
        flags,
        strength: 1.0,
        axis,
        on_weapon: [1.0, 0.0, 0.0, 0.0],
        on_struck: [1.0, 0.0, 0.0, 0.0],
        shares: grab_shares(1.0, 3.0),
    }
}

#[test]
fn the_grab_shares_follow_the_link_mobilities() {
    assert_eq!(grab_shares(1.0, 3.0), (0.25, -0.75));
}

#[test]
fn a_grab_along_its_axis_spends_its_strain_and_pulls_by_the_shares() {
    let mut body = pair(0.1);
    let mut g = grab(5.0, LINK_BODY | LINK_BREAKABLE, Vec3::X);
    let (w0, s0) = (body.parts[0].positions[0], body.parts[1].positions[0]);
    assert!(!body.pull_grab(0, 1, &mut g, false));
    // d = 10 cm along the axis: q = 10 + 1 = 11, the strain falls by
    // (11/10)·2 + 1 = 3.2.
    assert_eq!(g.strain, 5.0 - ((11.0_f32 / 10.0) * 2.0 + 1.0));
    // f = 1/(10·0.001 + 1) in double; the weapon takes a quarter of it,
    // the struck part three quarters back.
    let f = (1.0_f64 / (10.0_f64 * 0.001 + 1.0)) as f32;
    let gap = s0 - w0;
    assert_eq!(body.parts[0].positions[0], w0 + gap * (0.25 * f));
    assert_eq!(body.parts[1].positions[0], s0 + gap * (-0.75 * f));
    // The other particles carry no weight.
    assert_eq!(body.parts[0].positions[1], Vec3::X);
}

#[test]
fn a_grab_pushed_against_its_axis_loses_one_a_pass() {
    let mut body = pair(0.1);
    let mut g = grab(5.0, LINK_BODY | LINK_BREAKABLE, -Vec3::X);
    body.pull_grab(0, 1, &mut g, false);
    // q = −10 + 1 = −9: the strain falls by 1.
    assert_eq!(g.strain, 4.0);
    // A NaN q is not above 0 either.
    let mut g = grab(5.0, LINK_BODY | LINK_BREAKABLE, Vec3::NAN);
    body.pull_grab(0, 1, &mut g, false);
    assert_eq!(g.strain, 4.0);
}

#[test]
fn a_spent_grab_breaks_without_pulling() {
    let mut body = pair(0.1);
    let before = body.parts[0].positions;
    let mut g = grab(1.0, LINK_BODY | LINK_BREAKABLE, Vec3::X);
    assert!(body.pull_grab(0, 1, &mut g, false));
    assert_eq!(g.strain, 1.0 - 3.2);
    assert_ne!(g.flags & LINK_DISABLED, 0);
    assert_eq!(body.parts[0].positions, before);
    // A broken grab does nothing more.
    assert!(!body.pull_grab(0, 1, &mut g, false));
    // A NaN strain keeps pulling: only an ordered compare breaks.
    let mut g = grab(f32::NAN, LINK_BODY | LINK_BREAKABLE, Vec3::X);
    assert!(!body.pull_grab(0, 1, &mut g, false));
    assert_ne!(body.parts[0].positions, before);
}

#[test]
fn a_disabled_struck_body_breaks_the_grab_once_the_gap_counts() {
    let mut body = pair(0.1);
    let mut g = grab(5.0, LINK_BODY | LINK_BREAKABLE, Vec3::X);
    assert!(body.pull_grab(0, 1, &mut g, true));
    assert_eq!(g.strain, 5.0);
    // With the points together the grab is not even looked at.
    let mut body = pair(0.0);
    let mut g = grab(5.0, LINK_BODY | LINK_BREAKABLE, Vec3::X);
    assert!(!body.pull_grab(0, 1, &mut g, true));
    assert_eq!(g.flags & LINK_DISABLED, 0);
}

#[test]
fn a_reanchoring_grab_moves_its_struck_point_once_the_gap_counts() {
    let mut body = pair(0.0);
    let mut g = grab(5.0, LINK_BODY | LINK_BREAKABLE | LINK_REANCHOR, Vec3::X);
    body.pull_grab(0, 1, &mut g, false);
    // Under the epsilon: the flag stays.
    assert_ne!(g.flags & LINK_REANCHOR, 0);
    let mut body = pair(0.1);
    let before = body.parts[1].positions;
    body.pull_grab(0, 1, &mut g, false);
    assert_eq!(g.flags & LINK_REANCHOR, 0);
    assert_eq!(g.strain, 5.0);
    // The weapon point (the origin) on the struck part's particles,
    // which sit 10 cm along x: −0.1 on the x particle, 1.1 on the first.
    assert!((g.on_struck[0] - 1.1).abs() < 1e-6, "{:?}", g.on_struck);
    assert!((g.on_struck[1] + 0.1).abs() < 1e-6, "{:?}", g.on_struck);
    assert_eq!(body.parts[1].positions, before);
}

#[test]
fn a_broken_grab_drops_every_grab_to_its_part_and_the_grab_layers() {
    let mut body = pair(0.1);
    body.raise_grab_layer(1);
    body.raise_grab_layer(2);
    body.parts[0].class_word = 0xC0;
    let mut grabs = vec![
        (1, grab(1.0, LINK_BODY | LINK_BREAKABLE, Vec3::X)),
        (2, grab(1.0, LINK_BODY | LINK_BREAKABLE, Vec3::X)),
        (1, grab(1.0, LINK_BODY, Vec3::X)),
        (1, grab(1.0, LINK_BODY | LINK_BREAKABLE, -Vec3::X)),
    ];
    body.release_grabs(0, 1, &mut grabs);
    let left: Vec<(usize, u32)> = grabs.iter().map(|(s, g)| (*s, g.flags)).collect();
    assert_eq!(left, vec![(2, LINK_BODY | LINK_BREAKABLE), (1, LINK_BODY)]);
    assert_eq!(body.parts[0].class_word, 0x80);
    assert_eq!(body.parts[1].class_word, 0);
    assert_eq!(body.parts[2].class_word, GRAB_LAYER);
    // Nothing to drop: no layer changes.
    body.parts[1].class_word = GRAB_LAYER;
    let mut none = Vec::new();
    body.release_grabs(0, 1, &mut none);
    assert_eq!(body.parts[1].class_word, GRAB_LAYER);
}
