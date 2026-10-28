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

use crate::part::PartState;
use crate::sector::Bounds;

use super::{
    LIMP_PIVOT_SLACK, Link, Pairing, Part, Ragdoll, Sleep, WorldLink, pairs, setup_smoothing,
    tree_orders, world_link,
};

/// The gravity shares a built or stored body's point masses give, as the
/// original's setup works them out ([`crate::shape::setup_gravity`]).
pub use crate::shape::setup_gravity as gravity_shares;

/// Friction of a rigid body against the level: its constructor's, whatever
/// the file stores.
pub const RIGID_FRICTION: f32 = 0.75;
/// A root part below this height (m; the original's −5000 cm) has fallen
/// out of the world.
pub const FALL_OUT_HEIGHT: f32 = -50.0;
/// Rest tetrahedra flatter than this (m³ of the edge determinant) are
/// rejected.
const MIN_REST_VOLUME: f32 = 1e-12;

/// The smoothing the link conversion gives every slider link (flag 4),
/// whatever the file stores: the share of the way a slider's point is drawn
/// to where its last fraction put it.
const SLIDER_SMOOTHING: f32 = 0.01;

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
    /// The world transform of the node's parent, where a root body's world
    /// links anchor; identity for a free body or a child.
    pub parent_world: Affine3A,
    /// Collision spheres: centre, radius.
    pub spheres: Vec<(Vec3, f32)>,
    /// Where the node sits in its parent node's frame: an articulated
    /// child pivots there on its parent.
    pub origin: Vec3,
    /// An articulated child's stored links to its parent body.
    pub links: Vec<RigidLink>,
    /// The node's mesh box and bounding radius in its frame, which the
    /// pair stage buckets and meets the body by (the original copies the
    /// node's bounds); without one the box around its spheres stands in.
    pub mesh: Option<Bounds>,
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
    /// The file's flags (a root's world links keep them; a child keeps only
    /// the soft-weld sense).
    pub flags: u32,
    /// A slider's axis (m), its length the travel; zero for other links.
    pub axis: Vec3,
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
            triangles: None,
            pairing: {
                let mut pairing = Pairing::new(&rest, &rest_inverse, &s.spheres, s.mobility);
                if let Some(mesh) = s.mesh {
                    pairing.mesh = mesh;
                }
                pairing
            },
            rest_inverse,
            no_level: false,
            friction: RIGID_FRICTION,
            start: positions,
            moved: [Vec3::ZERO; 4],
            smoothed: [Vec3::ZERO; 4],
            own_origin: [1.0 - own.x - own.y - own.z, own.x, own.y, own.z],
            settled: Affine3A::IDENTITY,
            shown: (Affine3A::IDENTITY, Affine3A::IDENTITY),
            drawn_particles: ([Vec3::ZERO; 4], [Vec3::ZERO; 4]),
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
            // A prop is not a living limb: no limb family.
            state: PartState(0),
            static_only: false,
            class_word: 0,
            state_word: 0,
            flags2: 0,
            sound_ready_ms: 0,
            last_impact: 0.0,
            reacts: true,
            level_hits: 0,
            level_point: Vec3::ZERO,
            level_push: Vec3::ZERO,
            level_travel: Vec3::ZERO,
            level_travel2: 0.0,
            placed_offset: [Vec3::ZERO; 4],
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
        // A root body with stored links hangs from its node: each link's
        // parent point (in the parent node's frame, plus the node's own
        // translation) becomes a world anchor, and unless the first link is
        // a slider an origin pivot holds the part's origin at the node.
        let mut world = Vec::new();
        for (i, s) in setup.iter().enumerate() {
            if s.parent.is_some() || s.links.is_empty() {
                continue;
            }
            parts[i].no_level = true;
            for l in &s.links {
                let on_part = super::weights(&parts[i], l.on_body);
                let point = parts[i].point(&on_part);
                let anchor = s.parent_world.transform_point3(l.on_parent + s.origin);
                let axis = s.parent_world.transform_vector3(l.axis);
                let slider = l.flags & world_link::SLIDER != 0;
                let length = if slider && l.slack > 0.0 {
                    (axis.dot(point - anchor) / (l.slack * l.slack)).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                world.push(WorldLink {
                    part: i,
                    on_part,
                    anchor,
                    slack: l.slack.max(0.0),
                    strength: l.strength,
                    // The link conversion gives every slider link, the
                    // root's to the world too, a 0.01 smoothing.
                    smoothing: if slider {
                        SLIDER_SMOOTHING
                    } else {
                        l.smoothing
                    },
                    flags: l.flags,
                    axis,
                    length,
                    origin: false,
                    last_pass: None,
                });
            }
            if s.links[0].flags & world_link::SLIDER == 0 {
                world.push(WorldLink {
                    part: i,
                    on_part: parts[i].own_origin,
                    anchor: s.parent_world.transform_point3(s.origin),
                    slack: 0.0,
                    strength: 1.0,
                    smoothing: setup_smoothing(Vec3::ZERO),
                    flags: 0,
                    axis: Vec3::ZERO,
                    length: 0.0,
                    origin: true,
                    last_pass: None,
                });
            }
        }
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
                    flags: 0,
                    axis: Vec3::ZERO,
                };
                for (k, l) in s.links.iter().chain([&pivot]).enumerate() {
                    links.push(Link {
                        part: i,
                        parent,
                        on_part: super::weights(&parts[i], l.on_body),
                        on_parent: super::weights(&parts[parent], l.on_parent + s.origin),
                        slack: l.slack.max(0.0),
                        strength: l.strength,
                        flags: (l.flags & super::LINK_SOFT_WELD) | super::LINK_BODY,
                        softness: super::PIVOT_SOFTENING,
                        iteration_limit: None,
                        // The link conversion gives every slider link a
                        // 0.01 smoothing; the other links keep the stored one.
                        smoothing: if l.flags & world_link::SLIDER != 0 {
                            SLIDER_SMOOTHING
                        } else {
                            l.smoothing
                        },
                        length: 0.0,
                        shares,
                        mobility: (parts[i].inverse_mass, parts[parent].inverse_mass),
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
            world,
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
            grabs: Vec::new(),
            pieces: Vec::new(),
            triangles: Vec::new(),
            pass: 0,
            held_flags: [0; 2],
            held_count: 0,
            held_by: None,
            level_impacts: Vec::new(),
            limp_frame: None,
            limp_clip_rate: 1.0,
            limp_height: 0.0,
            limp_scale: 1.0,
            scrapes: Vec::new(),
            scrape_events: Vec::new(),
        }
    }
}

#[cfg(test)]
#[path = "body_rigid_test.rs"]
mod tests;
