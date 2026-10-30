//! The original's light-alias weight: how strongly one light lights a sector
//! node, and which of the six shadow-cube faces it reaches it through.
//!
//! A light alias is the light's copy in one sector. When a sector node asks,
//! the alias runs either the plain reach test or the gated one. Both return
//! a weight and a face mask, and report whether the weight passes the
//! alias's cutoff.
//!
//! - **Plain reach**: the component-wise product of the alias's colour and
//!   the node's colour, its Rec. 601 luma; above the cutoff (an ordered
//!   compare) the node is lit and the mask is one, else zero.
//! - **Gated reach**: when the light's node flags carry `0x2000000`, the
//!   scene's cell flood must first say the light reaches the node's sector
//!   (the caller's answer). The light sphere must then meet the node's
//!   sphere, and the squared distance from the light to the node's oriented
//!   box must be under the light's squared radius. The falloff distance is
//!   that box distance, or the squared sphere shell `(√d² − node radius)²`
//!   when the node's centre lies outside its own radius of the light and
//!   the shell is the larger; the weight is `max(1 − d·falloff, 0)⁴`. A spot
//!   (alias flags `& 0x300 == 0x200`) then shapes it: from the box's lowest
//!   point `b` (the centre's height less each axis's vertical reach), with
//!   `dy = light y − b` and `d` the box distance of the light moved to that
//!   height, `t = dy/√(dy² + d)` is blended toward one by the spot's flat
//!   factor and the weight is multiplied by
//!   `max((exp(ln 2 · (t·cone)) − 1)·exponent, 0)` with the engine's fast
//!   logarithm and exponential; `t` at or under 2⁻²³ zeroes the weight. The
//!   weight times the alias's intensity must pass the cutoff; the result
//!   is then the (unscaled) weight times the luma of the colour product,
//!   lit when that passes the cutoff too. The face mask starts with all six
//!   faces and clears a face when the node lies beyond it by more than the
//!   node's bounding radius, over that face's four plane tests.
//!
//! Rules: `docs/RENDERING.md` (Lights).

use bevy::math::Vec3;
use exa_assets::vertex_cache::{fast_exp, fast_ln};
use exa_formats::colour::luma;
use exa_physics::intersect::{Obb, point_obb_distance_squared};

/// The engine's small-value threshold (2⁻²³, rounded as stored).
const TINY: f32 = f32::from_bits(0x33ff_ffd7);

/// The light node flag that makes the gated test ask the cell flood first.
pub const FLOOD_GATED: u32 = 0x200_0000;

/// The six face normals of the original's shadow-cube face test
/// (`0.7071` on two axes each, the diagonal pairs).
pub const FACE_NORMALS: [Vec3; 6] = [
    Vec3::new(0.707_106_77, 0.707_106_77, 0.0),
    Vec3::new(0.707_106_77, -0.707_106_77, 0.0),
    Vec3::new(0.707_106_77, 0.0, 0.707_106_77),
    Vec3::new(0.707_106_77, 0.0, -0.707_106_77),
    Vec3::new(0.0, 0.707_106_77, 0.707_106_77),
    Vec3::new(0.0, 0.707_106_77, -0.707_106_77),
];

/// Each face's four plane tests: the index into the six face distances and
/// the sign of the test.
pub const FACE_TESTS: [[(usize, f32); 4]; 6] = [
    [(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0)],
    [(0, -1.0), (1, -1.0), (2, -1.0), (3, -1.0)],
    [(0, -1.0), (1, 1.0), (4, -1.0), (5, -1.0)],
    [(0, 1.0), (1, -1.0), (4, 1.0), (5, 1.0)],
    [(2, -1.0), (3, 1.0), (4, -1.0), (5, 1.0)],
    [(2, 1.0), (3, -1.0), (4, 1.0), (5, -1.0)],
];

/// A spot's shaping constants, as the light's activation computes them from
/// its record's `(q, p)` pair: the cone exponent
/// `1/(exp(ln 2 · (p + 1)) − 1)` with the engine's fast logarithm and
/// exponential, the flat factor `sin(q·π·0.5)·0.5` for `q` not below zero
/// (a NaN too) or `3 − 3/cos(q·0.5·π)` below it (the angle and the factor in
/// double precision), and `p + 1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpotShape {
    /// The cone exponent.
    pub exponent: f32,
    /// The flat factor the vertical blend uses.
    pub flat: f32,
    /// `p + 1`, the vertical exponent.
    pub cone: f32,
}

impl SpotShape {
    /// The shape a record's `(q, p)` pair gives.
    pub fn of(q: f32, p: f32) -> Self {
        let cone = p + 1.0;
        let exponent = 1.0 / (fast_exp(fast_ln(2.0) * cone) - 1.0);
        let flat = if q < 0.0 {
            let angle = f64::from(q * 0.5) * std::f64::consts::PI;
            (-3.0 / angle.cos() + 3.0) as f32
        } else {
            let angle = f64::from(q) * std::f64::consts::PI * 0.5;
            (angle.sin() * 0.5) as f32
        };
        Self {
            exponent,
            flat,
            cone,
        }
    }
}

/// One light alias's lighting values: the position (the light's own, moved
/// into the sector), radius, derived colour, intensity factor, cutoff,
/// falloff and spot shaping, plus its node flag word (`0x300` spot) and
/// whether the alias has a mask accumulator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AliasLight {
    pub position: Vec3,
    pub radius: f32,
    /// The derived colour.
    pub colour: Vec3,
    /// The intensity factor.
    pub intensity: f32,
    /// The weight under which a node is unlit (0.01).
    pub cutoff: f32,
    /// The inverse reach the falloff uses.
    pub falloff: f32,
    pub flags: u32,
    pub spot: Option<SpotShape>,
    /// The alias keeps a mask accumulator a lit node ORs its face mask into.
    pub has_mask: bool,
}

/// The node a light alias lights: its oriented box (centre, half extents
/// and unit axes as rows), its bounding radius, its colour and the face
/// mask its bucket held before this call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AliasNode {
    pub centre: Vec3,
    /// The node's bounding radius (also the face test's threshold).
    pub radius: f32,
    pub half: Vec3,
    /// The box's unit axes.
    pub axes: [Vec3; 3],
    /// The node's colour.
    pub colour: Vec3,
    /// The face mask the node's bucket held before this call.
    pub mask: u32,
}

impl AliasNode {
    fn obb(&self) -> Obb {
        Obb::new(self.centre, self.half, self.axes)
    }

    /// The box's lowest point: the centre's height less each axis's
    /// vertical reach, subtracted in axis order.
    fn bottom(&self) -> f32 {
        self.centre.y
            - self.axes[0].y.abs() * self.half.x
            - self.axes[1].y.abs() * self.half.y
            - self.axes[2].y.abs() * self.half.z
    }
}

/// What one reach test reports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AliasHit {
    /// The weight.
    pub weight: f32,
    /// The face mask for this node (zero when unlit or not gated).
    pub mask: u32,
    /// The weight passed the cutoff.
    pub lit: bool,
}

/// The cube faces a node is reached through: all six unless one of a
/// face's four plane tests puts the node beyond it by more than
/// `threshold` (the node's bounding radius). `d[i]` is the node's position
/// less the light's along face normal `i`; a NaN test clears nothing.
pub fn face_mask(node: Vec3, light: Vec3, threshold: f32) -> u32 {
    let d = FACE_NORMALS.map(|n| node.dot(n) - light.dot(n));
    let mut mask = 0x3F;
    for (i, tests) in FACE_TESTS.iter().enumerate() {
        for &(index, sign) in tests {
            if -sign * d[index] > threshold {
                mask &= !(1 << i);
                break;
            }
        }
    }
    mask
}

/// The plain reach test: the luma of the colour product against the
/// cutoff (ordered: a NaN luma is unlit). A lit node gets a mask of one,
/// and so does the alias's accumulator.
pub fn reach_plain(light: &AliasLight, node: &AliasNode) -> AliasHit {
    let weight = luma((light.colour * node.colour).to_array());
    let lit = light.cutoff < weight;
    AliasHit {
        weight,
        mask: u32::from(lit),
        lit,
    }
}

/// The gated reach test, with the cell flood's answer passed as `gate`
/// (pass `true` when the light's flags lack [`FLOOD_GATED`]). An early miss
/// reports a zero weight and mask, and lit only when the node's bucket held
/// a mask before. See the module docs for the rule.
pub fn reach(light: &AliasLight, node: &AliasNode, gate: bool) -> AliasHit {
    let miss = AliasHit {
        weight: 0.0,
        mask: 0,
        lit: node.mask != 0,
    };
    if !gate {
        return miss;
    }
    // The spheres must overlap; an unordered compare misses.
    let delta = light.position - node.centre;
    let centre_sq = (delta.x * delta.x + delta.y * delta.y) + delta.z * delta.z;
    let reach_sum = light.radius + node.radius;
    if !(reach_sum * reach_sum > centre_sq) {
        return miss;
    }
    let obb = node.obb();
    let box_sq = point_obb_distance_squared(light.position, &obb);
    if !(box_sq < light.radius * light.radius) {
        return miss;
    }
    // The shell distance when the node's centre lies outside its own radius
    // and the shell is not the smaller (an unordered compare keeps it).
    let mut distance = box_sq;
    if centre_sq > node.radius * node.radius {
        let shell = centre_sq.sqrt() - node.radius;
        let shell_sq = shell * shell;
        if !(shell_sq < box_sq) {
            distance = shell_sq;
        }
    }
    let product = light.colour * node.colour;
    if distance > TINY {
        distance = distance.sqrt();
    }
    // MAXSS returns its second operand, zero, on a NaN.
    let mut weight = maxss(1.0 - distance * light.falloff, 0.0);
    weight *= weight;
    weight *= weight;
    if let Some(spot) = light.spot
        && light.flags & 0x300 == 0x200
    {
        let bottom = node.bottom();
        let probe = Vec3::new(light.position.x, bottom, light.position.z);
        let d = point_obb_distance_squared(probe, &obb);
        let dy = light.position.y - bottom;
        let mut t = dy / (dy * dy + d).sqrt();
        t += (1.0 - t) * spot.flat;
        if t > TINY {
            let x = t * spot.cone;
            let shaped = (fast_exp(fast_ln(2.0) * x) - 1.0) * spot.exponent;
            weight *= maxss(shaped, 0.0);
        } else {
            weight = 0.0;
        }
    }
    if !(weight * light.intensity > light.cutoff) {
        return miss;
    }
    let weight = luma(product.to_array()) * weight;
    let lit = light.cutoff < weight;
    let mask = face_mask(node.centre, light.position, node.radius);
    AliasHit { weight, mask, lit }
}

/// `MAXSS`: the first operand when it is larger, else the second.
fn maxss(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// The alias's mask accumulator after a lit node: the node's face mask ORs
/// in, keeping the bits that mask leaves as they were
/// (`mask | old | (~mask & previous)`).
pub fn merge_mask(accumulated: u32, mask: u32, previous: u32) -> u32 {
    mask | accumulated | (!mask & previous)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node() -> AliasNode {
        AliasNode {
            centre: Vec3::new(0.0, 0.0, 100.0),
            radius: 17.320_51,
            half: Vec3::new(10.0, 10.0, 10.0),
            axes: [Vec3::X, Vec3::Y, Vec3::Z],
            colour: Vec3::new(1.0, 1.0, 1.0),
            mask: 0,
        }
    }

    fn light() -> AliasLight {
        AliasLight {
            position: Vec3::ZERO,
            radius: 1000.0,
            colour: Vec3::new(1.0, 1.0, 1.0),
            intensity: 1.0,
            cutoff: 0.01,
            falloff: 0.001,
            flags: 0,
            spot: None,
            has_mask: true,
        }
    }

    #[test]
    fn the_face_tables_are_the_originals() {
        assert_eq!(FACE_NORMALS[0].x.to_bits(), 0x3f35_04f3);
        assert_eq!(FACE_TESTS[2], [(0, -1.0), (1, 1.0), (4, -1.0), (5, -1.0)]);
    }

    #[test]
    fn the_plain_reach_is_the_colour_products_luma_against_the_cutoff() {
        let light = light();
        let hit = reach_plain(&light, &node());
        assert_eq!(hit.weight, (0.299f32 + 0.587) + 0.114);
        assert_eq!((hit.mask, hit.lit), (1, true));
        let mut dim = node();
        dim.colour = Vec3::splat(0.005);
        let hit = reach_plain(&light, &dim);
        assert_eq!((hit.mask, hit.lit), (0, false));
        // A NaN luma is unlit (the ordered compare).
        dim.colour = Vec3::NAN;
        assert!(!reach_plain(&light, &dim).lit);
    }

    #[test]
    fn the_gated_reach_falls_off_and_masks_by_the_node_radius() {
        let light = light();
        let n = node();
        // 90 cm from the box's near face (the shell, 100 − 17.32, is the
        // smaller): distance 90, falloff 0.001 → 0.91⁴, times the luma.
        let hit = reach(&light, &n, true);
        let mut w = 1.0f32 - 90.0 * 0.001;
        w *= w;
        w *= w;
        assert_eq!(hit.weight, ((0.299f32 + 0.587) + 0.114) * w);
        assert!(hit.lit);
        // The node sits along +z: only the face pointing back at it stays.
        assert_eq!(hit.mask, 1 << 5);
        // A closed gate or a light sphere that misses is a miss.
        assert!(!reach(&light, &n, false).lit);
        let mut far = light;
        far.radius = 1.0;
        assert_eq!(reach(&far, &n, true).weight, 0.0);
        // A node whose radius covers the light keeps every face.
        let mut big = n;
        big.radius = 1000.0;
        assert_eq!(reach(&light, &big, true).mask, 0x3F);
        // The intensity gates the weight but does not scale the result.
        let mut bright = light;
        bright.intensity = 4.0;
        assert_eq!(reach(&bright, &n, true).weight, hit.weight);
        let mut faint = light;
        faint.intensity = 0.001;
        let missed = reach(&faint, &n, true);
        assert_eq!((missed.weight, missed.lit), (0.0, false));
        // The mask accumulator ORs the mask and keeps its own bits.
        assert_eq!(merge_mask(0b10, 0b01, 0b100), 0b111);
        assert_eq!(merge_mask(0, 0x3F, 0), 0x3F);
    }

    #[test]
    fn the_shell_distance_is_the_larger_of_the_two() {
        // A long thin box: the box distance (10) is below the shell
        // (√(100² + 0) − 50 = 50), so the shell is used.
        let light = light();
        let mut n = node();
        n.half = Vec3::new(10.0, 10.0, 90.0);
        n.radius = 50.0;
        let hit = reach(&light, &n, true);
        let mut w = 1.0f32 - 50.0 * 0.001;
        w *= w;
        w *= w;
        assert_eq!(hit.weight, ((0.299f32 + 0.587) + 0.114) * w);
    }

    #[test]
    fn a_spot_shapes_the_weight_with_the_fast_power_of_two() {
        let shape = SpotShape::of(1.0, 1.0);
        assert_eq!(shape.cone, 2.0);
        assert_eq!(shape.exponent, 1.0 / (fast_exp(fast_ln(2.0) * 2.0) - 1.0));
        assert_eq!(
            shape.flat,
            ((std::f64::consts::PI * 0.5).sin() * 0.5) as f32
        );
        let below = SpotShape::of(-0.5, 0.0);
        let angle = f64::from(-0.5f32 * 0.5) * std::f64::consts::PI;
        assert_eq!(below.flat, (-3.0 / angle.cos() + 3.0) as f32);
        // A NaN q takes the sine branch.
        assert!(SpotShape::of(f32::NAN, 0.0).flat.is_nan());
        // The light 10 above the box's bottom height, right over it.
        let mut light = light();
        light.flags = 0x200;
        light.position = Vec3::new(0.0, 0.0, 95.0);
        light.spot = Some(SpotShape::of(0.0, 1.0));
        let mut n = node();
        n.centre = Vec3::new(0.0, -20.0, 100.0);
        let hit = reach(&light, &n, true);
        // The bottom is −30: dy = 30, the probe at (0, −30, 95) is inside
        // the box's footprint (d = 0), so t = 1 and the term is
        // (2^2 − 1)·exponent with the fast functions.
        let spot = light.spot.unwrap();
        let term = (fast_exp(fast_ln(2.0) * (1.0 * spot.cone)) - 1.0) * spot.exponent;
        // The box distance is 10 (the light 10 above the box's top).
        let mut w = 1.0f32 - 10.0 * 0.001;
        w *= w;
        w *= w;
        w *= term;
        assert_eq!(hit.weight, ((0.299f32 + 0.587) + 0.114) * w);
        // A light below the box's bottom: t ≤ 0 zeroes the weight.
        light.position = Vec3::new(0.0, -60.0, 95.0);
        assert_eq!(reach(&light, &n, true).weight, 0.0);
    }
}
