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
//! inflated mass divides the way IEEE division does.

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
}
