//! The original's light-alias weight: how strongly one light lights a sector
//! node, and which of the six shadow-cube faces it reaches it through.
//!
//! A light alias is the light's copy in one sector. When a sector node asks,
//! the alias runs either the plain reach test (an unranged light, node flag
//! `0x40000` clear) or the gated one (a ranged light, whose cell flood has
//! already decided that its sphere crosses the sector; the gate itself is
//! the caller's). Both return a weight and a face mask, and report whether
//! the weight passes the alias's cutoff.
//!
//! - **Plain reach** (`0x053400`): the component-wise product of the alias's
//!   colour and the node's colour, its Rec. 601 luma; above the cutoff the
//!   node is lit and the mask is one (the mask's one bit in the shadow map),
//!   else the mask is zero.
//! - **Gated reach** (`0x0530B0`): the light sphere must meet the node's
//!   sphere, and the distance from the light to the node's oriented box must
//!   be under the light's radius. The falloff distance is that box distance,
//!   or the sphere shell `(√d² − node radius)²` when the node lies outside
//!   the light sphere and the shell is the smaller of the two; the weight is
//!   `max(1 − distance · falloff, 0)⁴`. A spot light then shapes it: the
//!   distance from a point above the light to the node's box gives
//!   `t = (light y − node top) / √((light y − node top)² + d²)`, blended
//!   toward one by the light's flat factor, and multiplied by
//!   `max((cos(2·t·cone) − 1)·exponent, 0)`; `t` at or under 2⁻²³ zeroes
//!   the weight. The weight times the alias's intensity must pass the
//!   cutoff, then times the luma of the colour product. The face mask starts
//!   all six and clears a face when the node lies beyond it by more than the
//!   node's flags word read as a float, over that face's four plane tests.
//!
//! Rules: `docs/RENDERING.md` (Lights).

use bevy::math::Vec3;

/// The engine's "a distance this small is not a distance": 2⁻²³.
const TINY: f32 = 1.192_089_982_282_595_9e-7;

/// The luma weights, Rec. 601.
const LUMA_R: f32 = 0.298_999_99;
const LUMA_G: f32 = 0.587;
const LUMA_B: f32 = 0.114;

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

/// A spot's shaping constants, as the light node computes them from its
/// record's `(q, p)` pair: the cone exponent
/// `1/(acos(cos 2 · (p + 1)) − 1)`, the flat factor
/// `½·sin(q·π/2)` for `q ≥ 0` or `3 − 3/cos(q·π/2)` below it, and `p + 1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpotShape {
    /// The cone exponent (`0x248`).
    pub exponent: f32,
    /// The flat factor the vertical blend uses (`0x24C`).
    pub flat: f32,
    /// `p + 1`, the vertical exponent (`0x250`).
    pub cone: f32,
    /// The record's `p`, kept for the shape.
    pub p: f32,
    /// The record's `q`, kept for the shape.
    pub q: f32,
}

impl SpotShape {
    /// The shape a record's `(q, p)` pair gives, in single precision. The
    /// sine, cosine and arc cosine are the engine's fast pair.
    pub fn of(q: f32, p: f32) -> Self {
        use exa_physics::engine_math::{acos, sin_cos};
        let half_pi = std::f32::consts::FRAC_PI_2;
        let exponent = 1.0 / (acos(sin_cos(2.0).1 * (p + 1.0)) - 1.0);
        let flat = if q >= 0.0 {
            0.5 * sin_cos(q * half_pi).0
        } else {
            3.0 - 3.0 / sin_cos(q * half_pi).1
        };
        Self {
            exponent,
            flat,
            cone: p + 1.0,
            p,
            q,
        }
    }
}

/// One light alias's lighting values: the position (the light's own, moved
/// into the sector), radius, derived colour, intensity factor, cutoff,
/// falloff and spot shaping, plus the flag word (`0x300` spot, `0x10000`
/// shadow masks) and whether the alias has a mask accumulator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AliasLight {
    pub position: Vec3,
    pub radius: f32,
    /// The derived colour (`0x1D4`).
    pub colour: Vec3,
    /// The intensity factor (`0x1E0`).
    pub intensity: f32,
    /// The weight under which a node is unlit (`0x1E4`, 0.01).
    pub cutoff: f32,
    /// The inverse reach the falloff uses (`0x230`).
    pub falloff: f32,
    pub flags: u32,
    pub spot: Option<SpotShape>,
    /// The alias keeps a mask accumulator a lit node ORs its face mask into.
    pub has_mask: bool,
}

/// The node a light alias lights: its centre, half extents and frame axes,
/// its top and the three offsets the spot's top uses, its colour, flags and
/// the mask accumulated before this call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AliasNode {
    pub centre: Vec3,
    /// The node's bounding radius (`0x2C`).
    pub radius: f32,
    pub half: Vec3,
    /// The box's unit axes.
    pub axes: [Vec3; 3],
    /// The box's top (`0x24`).
    pub top: f32,
    /// The three `(offset, half extent)` pairs the top is lowered by.
    pub drops: [(f32, f32); 3],
    /// The node's colour (`0x1F8`).
    pub colour: Vec3,
    /// The node's flag word, whose low bits double as the face threshold
    /// when read as a float (the original's own comparison).
    pub flags: u32,
    /// The mask accumulated before this call (`0x10`).
    pub mask: u32,
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

/// The original's ordered `a > b`: false on either NaN (an unordered
/// compare fails every branch that needs it).
fn greater(a: f32, b: f32) -> bool {
    !a.is_nan() && !b.is_nan() && a > b
}

/// [`greater`]'s complement: `a <= b` or either NaN.
fn not_greater(a: f32, b: f32) -> bool {
    !greater(a, b)
}

/// The complement of a strict less-than: `a >= b` or either NaN.
fn not_less(a: f32, b: f32) -> bool {
    a.is_nan() || b.is_nan() || a >= b
}

/// The squared distance from a point to an oriented box: the point in the
/// box frame, each axis past its half extent squared and summed.
pub fn box_distance_squared(point: Vec3, centre: Vec3, axes: [Vec3; 3], half: Vec3) -> f32 {
    let d = point - centre;
    let local = [d.dot(axes[0]), d.dot(axes[1]), d.dot(axes[2])];
    let mut sum = 0.0;
    for (axis, extent) in local.into_iter().zip([half.x, half.y, half.z]) {
        let past = axis.abs() - extent;
        if past > 0.0 {
            sum += past * past;
        }
    }
    sum
}

/// The cube faces a node is reached through, all six unless a face's plane
/// test puts the node beyond it by more than the node's flags read as a
/// float. `d[i]` is the node's position less the light's along face normal
/// `i`.
pub fn face_mask(node: Vec3, light: Vec3, threshold: f32) -> u32 {
    let d: Vec<f32> = FACE_NORMALS
        .iter()
        .map(|n| node.dot(*n) - light.dot(*n))
        .collect();
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

/// The luma in the original's accumulation order, `z·0.114 + y·0.587 + x·0.299`.
fn luma(v: Vec3) -> f32 {
    v.z * LUMA_B + v.y * LUMA_G + v.x * LUMA_R
}

/// The plain reach test (`0x053400`): the luma of the colour product against
/// the cutoff. A lit node gets a mask of one and sets the alias's mask to
/// one.
pub fn reach_plain(light: &AliasLight, node: &AliasNode) -> AliasHit {
    let weight = luma(light.colour * node.colour);
    if weight <= light.cutoff {
        AliasHit {
            weight,
            mask: 0,
            lit: false,
        }
    } else {
        AliasHit {
            weight,
            mask: 1,
            lit: true,
        }
    }
}

/// The gated reach test (`0x0530B0`), with the cell flood's answer passed as
/// `gate`. See the module docs for the rule.
pub fn reach(light: &AliasLight, node: &AliasNode, gate: bool) -> AliasHit {
    if !gate {
        return AliasHit {
            weight: 0.0,
            mask: 0,
            lit: node.mask != 0,
        };
    }
    // The light and node spheres must overlap: an unordered radius compare
    // fails, as the original's does.
    let centre_delta = light.position - node.centre;
    let centre_sq = centre_delta.x * centre_delta.x
        + centre_delta.y * centre_delta.y
        + centre_delta.z * centre_delta.z;
    let reach_sum = light.radius + node.radius;
    if not_greater(reach_sum * reach_sum, centre_sq) {
        return AliasHit {
            weight: 0.0,
            mask: 0,
            lit: node.mask != 0,
        };
    }
    let box_sq = box_distance_squared(light.position, node.centre, node.axes, node.half);
    if not_less(box_sq, light.radius * light.radius) {
        return AliasHit {
            weight: 0.0,
            mask: 0,
            lit: node.mask != 0,
        };
    }
    // The falloff distance: the shell distance when the node is outside the
    // light sphere and that is the smaller value, else the box distance.
    let node_radius_sq = node.radius * node.radius;
    let mut distance_sq = box_sq;
    if node_radius_sq < centre_sq {
        let shell = centre_sq.sqrt() - node.radius;
        let shell_sq = shell * shell;
        if shell_sq >= box_sq {
            distance_sq = shell_sq;
        }
    }
    // The colour product's luma is taken before the distance, as the
    // original multiplies it in later.
    let colour_luma = luma(light.colour * node.colour);
    let mut distance = distance_sq;
    if distance > TINY {
        distance = distance.sqrt();
    }
    let mut weight = (1.0 - distance * light.falloff).max(0.0);
    weight *= weight;
    weight *= weight;
    if let Some(spot) = light.spot
        && light.flags & 0x300 == 0x200
    {
        let top = node.top - node.drops.iter().map(|(o, h)| o.abs() * h).sum::<f32>();
        let probe = Vec3::new(light.position.x, top, light.position.z);
        let d = box_distance_squared(probe, node.centre, node.axes, node.half);
        let dy = light.position.y - top;
        let t = dy / (dy * dy + d).sqrt();
        let t = t + (1.0 - t) * spot.flat;
        if t <= TINY {
            weight = 0.0;
        } else {
            // The original's fast cosine and arc cosine, exactly.
            use exa_physics::engine_math::{acos, sin_cos};
            let shaped = (acos(sin_cos(2.0).1 * t * spot.cone) - 1.0) * spot.exponent;
            weight *= shaped.max(0.0);
        }
    }
    let scaled = weight * light.intensity;
    if scaled <= light.cutoff {
        return AliasHit {
            weight: 0.0,
            mask: 0,
            lit: node.mask != 0,
        };
    }
    let weight = scaled * colour_luma;
    let lit = weight > light.cutoff;
    let mask = face_mask(node.centre, light.position, f32::from_bits(node.flags));
    AliasHit { weight, mask, lit }
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
            top: 10.0,
            drops: [(0.0, 0.0); 3],
            colour: Vec3::new(1.0, 1.0, 1.0),
            flags: 0,
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
    fn the_plain_reach_is_the_colour_products_luma_against_the_cutoff() {
        let light = light();
        let hit = reach_plain(&light, &node());
        assert_eq!(hit.weight, 1.0);
        assert_eq!((hit.mask, hit.lit), (1, true));
        // Half the colour is half the luma; under the cutoff is unlit.
        let mut dim = node();
        dim.colour = Vec3::splat(0.005);
        let hit = reach_plain(&light, &dim);
        assert!(hit.weight < 0.01);
        assert_eq!((hit.mask, hit.lit), (0, false));
        // The luma order shows on mixed colours.
        dim.colour = Vec3::new(1.0, 0.0, 0.0);
        assert_eq!(reach_plain(&light, &dim).weight, 0.298_999_99);
    }

    #[test]
    fn the_gated_reach_falls_off_and_merges_the_face_mask() {
        let light = light();
        let mut n = node();
        // 90 cm from the box's near face: distance 90, falloff 0.001 →
        // 0.91⁴.
        let hit = reach(&light, &n, true);
        let expected = 0.91f32 * 0.91 * 0.91 * 0.91;
        assert!((hit.weight - expected).abs() < 1e-6, "{}", hit.weight);
        assert!(hit.lit);
        // The node sits along +z: only the face pointing back at it stays
        // (the pair whose tests are all negative), the rest clear.
        assert_eq!(hit.mask, 1 << 5);
        // A closed gate or a light sphere that misses the node is unlit.
        assert!(!reach(&light, &n, false).lit);
        let mut far = light;
        far.radius = 1.0;
        assert!(!reach(&far, &n, true).lit);
        // A node inside the light's radius but past the box distance uses
        // the box distance; the sphere shell only when the node is outside.
        let mut inside = n;
        inside.centre = Vec3::new(0.0, 0.0, 500.0);
        let hit = reach(&light, &inside, true);
        assert!(hit.lit);
        // The mask accumulator ORs the mask and keeps its own bits.
        assert_eq!(merge_mask(0b10, 0b01, 0b100), 0b111);
        assert_eq!(merge_mask(0, 0x3F, 0), 0x3F);
        // The node's flags word doubles as the face threshold when read as
        // a float: an infinite float keeps every face.
        n.flags = 0x7F80_0000;
        assert_eq!(reach(&light, &n, true).mask, 0x3F);
    }

    #[test]
    fn a_spot_shapes_the_weight_and_zeroes_a_point_below_the_cone() {
        let mut light = light();
        light.flags = 0x200;
        light.spot = Some(SpotShape::of(1.0, 1.0));
        let hit = reach(&light, &node(), true);
        // The spot multiplies the falloff weight by its shaped term.
        assert!(hit.weight > 0.0 && hit.weight < 0.91f32.powi(4));
        // The top used by the probe is lowered by the three drops.
        let mut dropped = node();
        dropped.drops = [(50.0, 0.2), (0.0, 1.0), (0.0, 1.0)];
        let hit = reach(&light, &dropped, true);
        assert!(hit.weight >= 0.0);
    }
}
