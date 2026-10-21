//! Rigid bodies of models (props, items) in the body solver, as the
//! original registers a model node's rigid body: its four particles at the
//! node's matrix times its stored rest points, at rest; gravity on each
//! particle by its share of the centre of mass; the stored collision
//! spheres against the level, with the bodies' friction 0.75; the velocity
//! kept by the default environment and the sleep countdown of any limp
//! body. A body whose parent node is a rigid body is an articulated child:
//! its stored links (points on it and on its parent, slack, strength,
//! smoothing) and a pivot from its origin to where its node sits in its
//! parent join it to the parent body, each correction shared by the two
//! mobilities. A registered body is asleep (countdown 0) until something
//! wakes it. Evidence: `docs/formats/RFC.md` (Rigid body),
//! `docs/formats/README.md`.

use bevy::math::{Affine3A, Mat3};
use bevy::prelude::*;

use super::{
    LIMP_PIVOT_SLACK, Link, Pairing, Part, Ragdoll, Sleep, pairs, setup_smoothing, tree_orders,
};

/// Friction of a rigid body against the level: its constructor's, whatever
/// the file stores.
pub const RIGID_FRICTION: f32 = 0.75;
/// A root part below this height (m; the original's −5000 cm) has fallen
/// out of the world.
pub const FALL_OUT_HEIGHT: f32 = -50.0;
/// Rest tetrahedra flatter than this (m³ of the edge determinant) are
/// rejected.
const MIN_REST_VOLUME: f32 = 1e-12;

/// One rigid body of a model, as registration needs it (metres, the node's
/// frame).
#[derive(Debug, Clone, PartialEq)]
pub struct RigidPart {
    /// The node the body moves.
    pub entity: Entity,
    /// The body of the node's parent, for an articulated child (an index
    /// into the list the body is built from; parents come first).
    pub parent: Option<usize>,
    /// The node's world transform: the particles are placed at it.
    pub world: Affine3A,
    pub rest: [Vec3; 4],
    pub point_masses: [f32; 4],
    /// The stored mobility over its material's divisor.
    pub mobility: f32,
    /// Collision spheres: centre, radius.
    pub spheres: Vec<(Vec3, f32)>,
    /// Where the node sits in its parent node's frame: an articulated
    /// child pivots there on its parent.
    pub origin: Vec3,
    /// An articulated child's stored links to its parent body.
    pub links: Vec<RigidLink>,
}

/// A stored link of an articulated child to its parent body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigidLink {
    /// The point on the child, in its frame.
    pub on_body: Vec3,
    /// The point on the parent, in the parent's frame, before the child's
    /// origin is added.
    pub on_parent: Vec3,
    pub slack: f32,
    pub strength: f32,
    pub smoothing: f32,
}

impl RigidPart {
    /// Whether its rest points span a tetrahedron (the solver needs its
    /// inverse).
    pub fn is_solid(&self) -> bool {
        let r = self.rest;
        let edges = Mat3::from_cols(r[1] - r[0], r[2] - r[0], r[3] - r[0]);
        edges.determinant().abs() > MIN_REST_VOLUME && edges.is_finite()
    }
}

/// Each particle's share of gravity: 4/mᵢ scaled so the shares sum to 4,
/// from the stored point masses mᵢ (4 over the centre of mass's weight on
/// each particle), so gravity acts at the centre of mass. Equal shares when
/// the masses give none.
pub fn gravity_shares(point_masses: [f32; 4]) -> [f32; 4] {
    let inverse = point_masses.map(|m| 4.0 / m);
    let sum: f32 = inverse.iter().sum();
    let shares = inverse.map(|w| w * 4.0 / sum);
    if shares.iter().all(|w| w.is_finite()) {
        shares
    } else {
        [1.0; 4]
    }
}

/// The shares of a correction two linked bodies take by their mobilities m
/// and m_parent: m/(m + m_parent) and m_parent/(m + m_parent).
fn mobility_shares(m: f32, m_parent: f32) -> (f32, f32) {
    let sum = m + m_parent;
    if sum > 0.0 && sum.is_finite() {
        (m / sum, m_parent / sum)
    } else {
        (0.5, 0.5)
    }
}

impl Part {
    /// A rigid body's part: its stored rest points placed at its node's
    /// matrix, at rest.
    fn rigid(s: &RigidPart, bone: usize) -> Self {
        let rest = s.rest;
        let edges = Mat3::from_cols(rest[1] - rest[0], rest[2] - rest[0], rest[3] - rest[0]);
        let rest_inverse = edges.inverse();
        let own = rest_inverse * -rest[0];
        let positions = rest.map(|r| s.world.transform_point3(r));
        Self {
            bone,
            entity: s.entity,
            parent: s.parent,
            rest,
            positions,
            previous: positions,
            inverse_mass: s.mobility,
            gravity: gravity_shares(s.point_masses),
            spheres: s.spheres.clone(),
            pairing: Pairing::new(&rest, &rest_inverse, &s.spheres, s.mobility),
            rest_inverse,
            friction: RIGID_FRICTION,
            start: positions,
            moved: [Vec3::ZERO; 4],
            own_origin: [1.0 - own.x - own.y - own.z, own.x, own.y, own.z],
            settled: Affine3A::IDENTITY,
            shown: (Affine3A::IDENTITY, Affine3A::IDENTITY),
            contacts: Vec::new(),
            strain: 0.0,
            leg_limit: None,
            origin: s.origin,
            foot: None,
            factor: 0.0,
            saturation: 0.0,
            relax: 0.0,
            relax_cap: 0.0,
            rest_turn: Quat::IDENTITY,
        }
        .settle()
    }
}

impl Ragdoll {
    /// The rigid bodies of a model node and its articulated children, as
    /// the original registers them: each placed at its node's matrix, at
    /// rest; each child joined to its parent by its stored links, then a
    /// pivot from its origin to where its node sits in the parent (slack 0,
    /// strength 1), every correction shared by the two mobilities. The body
    /// is registered asleep (countdown 0); it meets the level, and other
    /// bodies' parts through the pairs by its mobility and the group masks
    /// of its tree (a child never meets its parent). A part's `bone` is its
    /// index. Parts must be solid ([`RigidPart::is_solid`]).
    pub fn rigid(setup: &[RigidPart]) -> Self {
        let mut parts: Vec<Part> = setup
            .iter()
            .enumerate()
            .map(|(i, s)| Part::rigid(s, i))
            .collect();
        pairs::mask_groups(&mut parts);
        let mut links = Vec::new();
        let mut links_of = Vec::with_capacity(setup.len());
        for (i, s) in setup.iter().enumerate() {
            let first = links.len();
            if let Some(parent) = s.parent.filter(|&j| j < i) {
                let shares = mobility_shares(parts[i].inverse_mass, parts[parent].inverse_mass);
                let pivot = RigidLink {
                    on_body: Vec3::ZERO,
                    on_parent: Vec3::ZERO,
                    slack: 0.0,
                    strength: 1.0,
                    smoothing: setup_smoothing(Vec3::ZERO),
                };
                for (k, l) in s.links.iter().chain([&pivot]).enumerate() {
                    links.push(Link {
                        part: i,
                        parent,
                        on_part: super::weights(&parts[i], l.on_body),
                        on_parent: super::weights(&parts[parent], l.on_parent + s.origin),
                        slack: l.slack.max(0.0),
                        strength: l.strength,
                        smoothing: l.smoothing,
                        length: 0.0,
                        shares,
                        origin: k == s.links.len(),
                        anchor: None,
                        kept: None,
                        history: (Vec3::ZERO, Vec3::ZERO),
                    });
                }
            }
            links_of.push(first..links.len());
        }
        // Each slack link's smoothed length starts at its placed distance.
        for l in &mut links {
            if l.slack > LIMP_PIVOT_SLACK {
                l.length = parts[l.parent]
                    .point(&l.on_parent)
                    .distance(parts[l.part].point(&l.on_part));
            }
        }
        let (outward, inward) = tree_orders(&parts);
        Self {
            parts,
            links,
            links_of,
            outward,
            inward,
            chest: Vec::new(),
            legs: Vec::new(),
            sole: 0.0,
            drive: None,
            ticks: 0,
            countdown: 0,
            settle: 0,
            sleep: Sleep::Deep,
            simulated: false,
            fell_out: false,
        }
    }
}

#[cfg(test)]
#[path = "body_rigid_test.rs"]
mod tests;
