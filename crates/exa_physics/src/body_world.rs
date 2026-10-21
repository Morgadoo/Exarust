//! Links from a body part to a point in the world: the plain pin, the
//! damped spring, the slider and the hand's horizontal drag that the
//! original's limp-body link iteration gives a link whose parent is a scene
//! node instead of another body.
//!
//! Such a link's parent side is a point in the parent node's frame, put in
//! the world by that node's matrix each pass. The self side is a point on
//! the part, kept as weights over its particles. Each link pulls only its
//! part (no share with a second body), by its weights unclamped, in the
//! part's own iteration after its body links. A link with a pass limit runs
//! only up to that pass. A slider keeps its travel parameter and a breakable
//! link its strain in [`WorldLink::state`].
//!
//! The laws are the original's, in centimetres: the plain link takes the
//! whole gap times the strength (a zero-slack link beyond 10·strength cm
//! takes 10·strength cm and its strength squared), with a slack it takes the
//! excess over the slack; the damped spring takes a tenth of the strength
//! times the gap less the motion already heading there less 0.4 of the
//! motion; the slider pulls the point 1 % of the way (its smoothing) toward
//! its last position on the axis, clamps the parameter to the travel and
//! applies the correction twice; the horizontal drag pulls across the ground
//! at twice the strength, the step capped at min(3·√mobility, 5) cm, less
//! the approach. Flag 0x800 removes 80 % of the gap's component along the
//! axis first. A disabled link (0x100000) does nothing.
//!
//! The original's only code-made world link is the hand's drag (this crate
//! keeps that one in [`crate::body::Grab`]); the plain, spring and slider
//! links are what a body hung from the world (a lamp, a chain, a door) gets
//! from its stored links. Evidence: `context/re/body-links.md` §6.4 and
//! `context/re/body-tick-order.md` §5.3.

use bevy::math::Affine3A;
use bevy::prelude::*;

use super::{CM, GENERAL_EPSILON, LINK_DISABLED, LINK_ITERATION_LIMIT, Part, Ragdoll, weights};

/// The link slides along its axis.
pub const WORLD_SLIDER: u32 = 0x4;
/// The link is a damped spring.
pub const WORLD_SOFT_WELD: u32 = 0x8;
/// The link pulls across the ground only.
pub const WORLD_HORIZONTAL: u32 = 0x200;
/// The link first removes the gap's component along its axis.
pub const WORLD_PLANE: u32 = 0x800;

/// The horizontal drag's step cap per root of the mobility, and its most.
const DRAG_PER_ROOT_MOBILITY: f32 = 3.0;
const DRAG_MAX_STEP: f32 = 5.0;
const DRAG_STRENGTH: f32 = 2.0;
/// A zero-slack world link's longest step, per unit of strength (cm), and
/// the squared length beyond which a pull takes the strength squared.
const PLAIN_MAX_STEP: f32 = 10.0;
const PLAIN_CAP_SQUARED: f32 = 100.0;
/// A plane link's removal of the gap along its axis.
const PLANE_REMOVAL: f32 = 0.8;

/// A link from a point on a part to a point in a scene node's frame.
#[derive(Debug, Clone)]
pub struct WorldLink {
    /// The part the link pulls.
    pub part: usize,
    /// The point on the part, as weights over its particles.
    on_part: [f32; 4],
    /// The anchor point in the parent node's frame (m).
    pub on_parent: Vec3,
    /// The parent node's world matrix this tick.
    pub frame: Affine3A,
    /// A plain link's slack (m) or a slider's travel squared (m²).
    pub slack: f32,
    /// A slider's pull toward its last position (0.01).
    pub smoothing: f32,
    pub strength: f32,
    /// A plain pin's softening per cm² (the original's 0.001; it is not
    /// read by the plain link, which keeps it for record parity).
    pub softness: f32,
    /// A slider's travel parameter or a breakable link's strain.
    pub state: f32,
    /// The link's kind and flags ([`WORLD_SLIDER`], [`WORLD_SOFT_WELD`],
    /// [`WORLD_HORIZONTAL`], [`WORLD_PLANE`], [`LINK_ITERATION_LIMIT`],
    /// [`LINK_DISABLED`]).
    pub flags: u32,
    /// A slider's axis in the parent node's frame; its length is the travel
    /// (so the stored slack is its squared length, as the original's slider
    /// divides by it).
    pub axis: Vec3,
    /// The link runs only in the tick's passes up to this one (from 0).
    pub last_pass: Option<u32>,
}

impl WorldLink {
    /// A plain link from the point `local` on `part` to `on_parent` in the
    /// parent node's frame, which `frame` puts in the world.
    pub fn plain(
        part: usize,
        part_frame: &Part,
        local: Vec3,
        on_parent: Vec3,
        frame: Affine3A,
    ) -> Self {
        Self {
            part,
            on_part: weights(part_frame, local),
            on_parent,
            frame,
            slack: 0.0,
            smoothing: 0.0,
            strength: 1.0,
            softness: super::PIVOT_SOFTENING,
            state: 0.0,
            flags: 0,
            axis: Vec3::ZERO,
            last_pass: None,
        }
    }
}

impl Ragdoll {
    /// Adds a world link. Returns its index, or None when its part does not
    /// exist.
    pub fn add_world_link(&mut self, link: WorldLink) -> Option<usize> {
        (link.part < self.parts.len()).then(|| {
            self.world_links.push(link);
            self.world_links.len() - 1
        })
    }

    /// The body's world links, in the order they were added.
    pub fn world_links(&self) -> &[WorldLink] {
        &self.world_links
    }

    /// The body's world links, to move a frame or an anchor.
    pub fn world_links_mut(&mut self) -> &mut [WorldLink] {
        &mut self.world_links
    }

    /// Puts the parent node of world link `index` at `frame` (m), as the
    /// scene does before a tick.
    pub fn set_world_frame(&mut self, index: usize, frame: Affine3A) {
        if let Some(link) = self.world_links.get_mut(index) {
            link.frame = frame;
        }
    }

    /// One pass of part `i`'s world links, after its body links (the
    /// original's link order: the stored links, the origin pivot, then the
    /// links code adds).
    pub(super) fn pull_world_links(&mut self, i: usize) {
        let pass = self.pass;
        let parts = &mut self.parts;
        let Some(part) = parts.get_mut(i) else {
            return;
        };
        for link in &mut self.world_links {
            if link.part != i
                || link.flags & LINK_DISABLED != 0
                || link
                    .last_pass
                    .is_some_and(|last| link.flags & LINK_ITERATION_LIMIT != 0 && pass > last)
            {
                continue;
            }
            let now = part.point(&link.on_part);
            let anchor = link.frame.transform_point3(link.on_parent);
            let moved = (now - part.point_at_start(&link.on_part)) * CM;
            let gap = (anchor - now) * CM;
            if let Some(correction) = world_correction(link, gap, moved, part.inverse_mass) {
                part.push(&link.on_part, correction / CM);
                // The slider's correction is applied once more, as the
                // original's branch falls through to the common apply.
                if link.flags & WORLD_SLIDER != 0 {
                    part.push(&link.on_part, correction / CM);
                }
            }
        }
    }
}

/// A world link's correction (cm) for the gap `gap` (cm, anchor minus the
/// part's point), the point's motion `moved` (cm) this tick, and the part's
/// mobility.
fn world_correction(
    link: &mut WorldLink,
    mut gap: Vec3,
    moved: Vec3,
    mobility: f32,
) -> Option<Vec3> {
    if link.flags & WORLD_HORIZONTAL != 0 {
        gap.y = 0.0;
        let len2 = gap.x * gap.x + gap.z * gap.z;
        if len2 < GENERAL_EPSILON {
            return None;
        }
        let len = len2.sqrt();
        let unit = gap * (1.0 / len);
        let mut cap = mobility.sqrt() * DRAG_PER_ROOT_MOBILITY;
        if cap > DRAG_MAX_STEP {
            cap = DRAG_MAX_STEP;
        }
        let mut pull = if cap < len {
            gap * (cap / len * DRAG_STRENGTH * link.strength)
        } else {
            gap * (DRAG_STRENGTH * link.strength)
        };
        let toward = moved.dot(unit);
        if toward > GENERAL_EPSILON {
            pull -= unit * toward;
        }
        return Some(pull);
    }
    let len2 = gap.length_squared();
    if link.flags & WORLD_SOFT_WELD != 0 {
        if len2 < GENERAL_EPSILON {
            return None;
        }
        let len = len2.sqrt();
        let mut pull = gap * (0.1 * link.strength);
        // The original normalises the scaled spring, not the raw gap.
        let unit = pull * (1.0 / len);
        let toward = moved.dot(unit);
        if toward > GENERAL_EPSILON {
            pull -= unit * toward;
        }
        pull -= moved * 0.4;
        return Some(pull);
    }
    if link.flags & WORLD_SLIDER != 0 {
        let anchor = link.frame.transform_point3(link.on_parent);
        let axis = link.frame.transform_vector3(link.axis);
        let p = anchor - gap / CM;
        let target = anchor + axis * link.state;
        let q = p + (target - p) * link.smoothing;
        let t = if link.slack != 0.0 {
            (axis.dot(q - anchor) / link.slack).clamp(0.0, 1.0)
        } else {
            // The original divides by the slider's stored slack; a zero
            // travel cannot parameterise an axis, so it stays at its start.
            0.0
        };
        link.state = t;
        let slid = anchor + axis * t;
        return Some(((slid - q) + (target - p) * link.smoothing) * CM);
    }
    let mut gap = gap;
    if link.flags & WORLD_PLANE != 0 {
        let along = gap.dot(link.axis);
        gap -= link.axis * (along * PLANE_REMOVAL);
    }
    let len2 = gap.length_squared();
    let slack = link.slack * CM;
    if slack * slack < len2 {
        if slack <= GENERAL_EPSILON {
            let longest = PLAIN_MAX_STEP * link.strength;
            if len2 <= longest * longest {
                Some(gap * link.strength)
            } else {
                Some(gap * ((PLAIN_CAP_SQUARED / len2).sqrt() * link.strength * link.strength))
            }
        } else {
            let len = len2.sqrt();
            link.state = len;
            Some(gap * ((len - slack) / len * link.strength))
        }
    } else {
        link.state = 0.0;
        None
    }
}

#[cfg(test)]
#[path = "body_world_test.rs"]
mod tests;
