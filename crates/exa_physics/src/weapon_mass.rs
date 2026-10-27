//! A weapon part tree's mass redistribution.
//!
//! Two walks, parent before children. The first latches a saved mass of
//! zero from the live mass, replaces the live mass with 1.25 times that
//! saved mass, and stores its reciprocal. It counts the parts and sums
//! those reciprocals. The second writes both words with the reciprocal of
//! a half-even, half-by-inverse share of the caller's scale. A saved mass
//! that is already set is the one inflated.
//!
//! The arithmetic is single precision in the original's order. A zero
//! inflated mass divides the way IEEE division does. [`distribute_tree`]
//! is the same law over a flat part list, the form a held weapon's model
//! nodes take; [`WeaponMassNode`] is a convenient tree for callers that
//! already have one.

use bevy::prelude::Component;

/// Even share of the blend, and the weight of a part's own inverse.
const HALF: f32 = 0.5;
/// A latched mass is inflated by this before it is inverted.
const INFLATE: f32 = 1.25;

/// One node of the weapon's part tree while its masses are redistributed.
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponMassNode {
    /// The live mass word. The first pass may replace it with the saved
    /// mass times 1.25. The second pass replaces it with the distributed
    /// inverse.
    pub mass: f32,
    /// The inverse-mass word. The first pass stores the reciprocal of the
    /// inflated mass. The second pass stores the distributed inverse.
    pub inverse: f32,
    /// Saved mass. Zero is replaced by the live mass on the first pass
    /// and then left alone, including a later redistribution.
    pub saved: f32,
    /// Child parts, in the order the tree stores them.
    pub children: Vec<WeaponMassNode>,
}

impl WeaponMassNode {
    /// A leaf whose saved mass is still zero.
    pub fn leaf(mass: f32) -> Self {
        Self {
            mass,
            inverse: 0.0,
            saved: 0.0,
            children: Vec::new(),
        }
    }
}

/// The scratch the two passes share: how many parts, the sum of their
/// inflated inverses, and the caller's scale.
struct MassTally {
    count: i32,
    sum: f32,
    scale: f32,
}

/// Redistributes `root` and every part under it. `scale` multiplies each
/// part's blended share before the reciprocal is stored in both words.
pub fn distribute_weapon_mass(root: &mut WeaponMassNode, scale: f32) {
    let mut tally = MassTally {
        count: 0,
        sum: 0.0,
        scale,
    };
    inflate(root, &mut tally);
    rewrite(root, &tally);
}

/// Parent before children: latch, inflate, invert, then the same for each
/// child.
fn inflate(node: &mut WeaponMassNode, tally: &mut MassTally) {
    if node.saved == 0.0 {
        node.saved = node.mass;
    }
    node.mass = node.saved * INFLATE;
    node.inverse = 1.0 / node.mass;
    tally.count += 1;
    tally.sum += node.inverse;
    for child in &mut node.children {
        inflate(child, tally);
    }
}

/// Parent before children: both words become the distributed inverse.
fn rewrite(node: &mut WeaponMassNode, tally: &MassTally) {
    let count = tally.count as f32;
    let share = HALF / count + (node.inverse / tally.sum) * HALF;
    node.inverse = 1.0 / (share * tally.scale);
    node.mass = node.inverse;
    for child in &mut node.children {
        rewrite(child, tally);
    }
}

/// One part of a weapon in a flat list, the parents before their children.
/// This is [`WeaponMassNode`] without the tree: a held weapon built from a
/// model's rigid nodes keeps the node indices and their parent relations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponMassPart {
    /// The part's parent in the same list, when it has one.
    pub parent: Option<usize>,
    /// The live mass word: the initial mass, then 1.25 times the saved
    /// mass. [`distribute_tree`] rewrites it to the distributed inverse.
    pub mass: f32,
    /// The inverse-mass word: the reciprocal of the inflated mass, then the
    /// distributed inverse.
    pub inverse: f32,
    /// The saved mass (the original's separate latch, zero until the first
    /// pass).
    pub saved: f32,
}

impl WeaponMassPart {
    /// A part from its live mass, its saved mass still unlatched.
    pub fn new(parent: Option<usize>, mass: f32) -> Self {
        Self {
            parent,
            mass,
            inverse: 0.0,
            saved: 0.0,
        }
    }
}

/// A held weapon's parts and their total simulation mass.
#[derive(Debug, Clone, Default, PartialEq, Component)]
pub struct WeaponMass {
    /// The parts, in the model's node order, parents before children.
    pub parts: Vec<WeaponMassPart>,
    /// The weapon's total simulation mass (`weapon_record::total_mass`).
    pub total: f32,
}

impl WeaponMass {
    /// Distributes `parts` with the weapon's total mass as the scale, as the
    /// holding pass does for a held weapon's body tree.
    pub fn new(mut parts: Vec<WeaponMassPart>, total: f32) -> Self {
        distribute_tree(&mut parts, total);
        Self { parts, total }
    }
}

/// The parent-before-children order a tree walk visits: roots in list
/// order, each one's children in list order, depth first. A cycle in bad
/// input is cut after every part has been visited once.
fn tree_order(parts: &[WeaponMassPart]) -> Vec<usize> {
    let mut children = vec![Vec::new(); parts.len()];
    let mut roots = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        match part.parent.filter(|&p| p < parts.len() && p != i) {
            Some(p) => children[p].push(i),
            None => roots.push(i),
        }
    }
    let mut order = Vec::with_capacity(parts.len());
    let mut stack: Vec<usize> = roots.into_iter().rev().collect();
    let mut seen = vec![false; parts.len()];
    while let Some(i) = stack.pop() {
        if i >= parts.len() || seen[i] {
            continue;
        }
        seen[i] = true;
        order.push(i);
        for &child in children[i].iter().rev() {
            stack.push(child);
        }
    }
    // A part in a cycle and not reachable from a root still takes a turn,
    // in list order, as a fresh root would.
    for (i, &was_seen) in seen.iter().enumerate() {
        if !was_seen {
            order.push(i);
        }
    }
    order
}

/// Redistributes a flat weapon part list in the original's two passes,
/// depth first from each root in list order. The order is computed from
/// `parent`, so the caller need not sort the list.
pub fn distribute_tree(parts: &mut [WeaponMassPart], scale: f32) {
    let order = tree_order(parts);
    let mut count = 0i32;
    let mut sum = 0.0f32;
    for &i in &order {
        let part = &mut parts[i];
        if part.saved == 0.0 {
            part.saved = part.mass;
        }
        part.mass = part.saved * INFLATE;
        part.inverse = 1.0 / part.mass;
        count += 1;
        sum += part.inverse;
    }
    let count = count as f32;
    for &i in &order {
        let part = &mut parts[i];
        let share = HALF / count + (part.inverse / sum) * HALF;
        part.inverse = 1.0 / (share * scale);
        part.mass = part.inverse;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_part_keeps_its_mass_and_inverts_the_scale() {
        let mut light = WeaponMassNode::leaf(2.0);
        distribute_weapon_mass(&mut light, 1.0);
        assert_eq!(light.saved, 2.0);
        assert_eq!(light.mass, 1.0);
        assert_eq!(light.inverse, 1.0);

        let mut heavy = WeaponMassNode::leaf(7.0);
        distribute_weapon_mass(&mut heavy, 1.0);
        assert_eq!(heavy.saved, 7.0);
        assert_eq!(heavy.inverse, 1.0);

        let mut scaled = WeaponMassNode::leaf(2.0);
        distribute_weapon_mass(&mut scaled, 2.0);
        assert_eq!(scaled.inverse.to_bits(), 0x3f00_0000);
        assert_eq!(scaled.mass.to_bits(), scaled.inverse.to_bits());
    }

    #[test]
    fn a_tree_blends_half_evenly_and_half_by_inverse() {
        // Saved 8 on the second child, live mass 99: the live word is not
        // the one inflated. Parent mass 2, first child mass 1, scale 0.6.
        let mut root = WeaponMassNode {
            mass: 2.0,
            inverse: 0.0,
            saved: 0.0,
            children: vec![
                WeaponMassNode::leaf(1.0),
                WeaponMassNode {
                    mass: 99.0,
                    inverse: 0.0,
                    saved: 8.0,
                    children: Vec::new(),
                },
            ],
        };
        distribute_weapon_mass(&mut root, 0.6);
        assert_eq!(root.saved.to_bits(), 0x4000_0000);
        assert_eq!(root.mass.to_bits(), 0x40a6_6665);
        assert_eq!(root.inverse.to_bits(), 0x40a6_6665);
        assert_eq!(root.children[0].saved.to_bits(), 0x3f80_0000);
        assert_eq!(root.children[0].inverse.to_bits(), 0x4060_dd67);
        assert_eq!(root.children[1].saved.to_bits(), 0x4100_0000);
        assert_eq!(root.children[1].inverse.to_bits(), 0x4101_ffff);
        assert_eq!(
            root.children[1].mass.to_bits(),
            root.children[1].inverse.to_bits()
        );
    }

    #[test]
    fn a_second_pass_inflates_the_saved_mass_again() {
        let mut root = WeaponMassNode::leaf(2.0);
        root.children.push(WeaponMassNode::leaf(1.0));
        distribute_weapon_mass(&mut root, 0.6);
        let once = root.clone();
        distribute_weapon_mass(&mut root, 0.6);
        assert_eq!(root, once);
    }

    #[test]
    fn a_negative_zero_saved_mass_is_latched_from_the_live_mass() {
        let mut node = WeaponMassNode {
            mass: 2.0,
            inverse: 0.0,
            saved: -0.0,
            children: Vec::new(),
        };
        distribute_weapon_mass(&mut node, 2.0);
        assert_eq!(node.saved.to_bits(), 0x4000_0000);
        assert_eq!(node.inverse.to_bits(), 0x3f00_0000);
    }

    #[test]
    fn a_zero_mass_follows_the_division() {
        let mut node = WeaponMassNode::leaf(0.0);
        distribute_weapon_mass(&mut node, 1.0);
        assert_eq!(node.saved.to_bits(), 0);
        assert!(node.mass.is_nan());
        assert!(node.inverse.is_nan());
    }

    #[test]
    fn a_flat_tree_takes_the_same_walk_as_the_node_tree() {
        // A weapon's handle with two rigid children and a grandchild, in
        // the model's node order.
        let mut parts = vec![
            WeaponMassPart::new(None, 2.0),
            WeaponMassPart::new(Some(0), 1.0),
            WeaponMassPart::new(Some(0), 0.5),
            WeaponMassPart::new(Some(1), 0.25),
        ];
        distribute_tree(&mut parts, 0.6);
        let mut root = WeaponMassNode {
            mass: 2.0,
            inverse: 0.0,
            saved: 0.0,
            children: vec![
                WeaponMassNode {
                    mass: 1.0,
                    inverse: 0.0,
                    saved: 0.0,
                    children: vec![WeaponMassNode::leaf(0.25)],
                },
                WeaponMassNode::leaf(0.5),
            ],
        };
        distribute_weapon_mass(&mut root, 0.6);
        let node = |n: &WeaponMassNode| (n.mass.to_bits(), n.inverse.to_bits(), n.saved.to_bits());
        assert_eq!(
            (parts[0].mass.to_bits(), parts[0].inverse.to_bits()),
            (root.mass.to_bits(), root.inverse.to_bits())
        );
        assert_eq!(
            (
                parts[1].mass.to_bits(),
                parts[1].inverse.to_bits(),
                parts[1].saved.to_bits()
            ),
            node(&root.children[0])
        );
        assert_eq!(
            (
                parts[3].mass.to_bits(),
                parts[3].inverse.to_bits(),
                parts[3].saved.to_bits()
            ),
            node(&root.children[0].children[0])
        );
        assert_eq!(
            (
                parts[2].mass.to_bits(),
                parts[2].inverse.to_bits(),
                parts[2].saved.to_bits()
            ),
            node(&root.children[1])
        );
        // The first pass latches the saved mass and the second keeps it.
        assert_eq!(parts[0].saved.to_bits(), root.saved.to_bits());
    }

    #[test]
    fn a_flat_tree_visits_roots_in_list_order() {
        // Two roots, the second listed first in the call's parent list:
        // the walk still visits them root by root, then each one's
        // children, and every part gets a share of its own.
        let mut parts = vec![
            WeaponMassPart::new(None, 4.0),
            WeaponMassPart::new(None, 2.0),
            WeaponMassPart::new(Some(0), 1.0),
        ];
        distribute_tree(&mut parts, 1.0);
        assert_eq!(parts[0].mass, parts[0].inverse);
        assert_eq!(parts[1].mass, parts[1].inverse);
        assert_eq!(parts[2].mass, parts[2].inverse);
        // The heavier first root's inverse share is larger than the second
        // root's, and its child's is the smallest.
        assert!(parts[0].inverse > parts[1].inverse);
        assert!(parts[0].inverse > parts[2].inverse);
        // A second pass inflates the saved masses again but writes the same
        // words, exactly as the tree does.
        let once = parts.clone();
        distribute_tree(&mut parts, 1.0);
        assert_eq!(parts, once);
    }

    #[test]
    fn a_weapon_mass_table_keeps_the_total_and_works_on_the_parts() {
        let parts = vec![
            WeaponMassPart::new(None, 3.0),
            WeaponMassPart::new(Some(0), 1.0),
        ];
        let mass = WeaponMass::new(parts, 0.75);
        assert_eq!(mass.total, 0.75);
        assert_eq!(mass.parts.len(), 2);
        assert!(
            mass.parts
                .iter()
                .all(|p| p.mass.is_finite() && p.mass > 0.0)
        );
    }
}
