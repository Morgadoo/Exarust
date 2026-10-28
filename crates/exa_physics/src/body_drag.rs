//! The hand's grab: a link from a point on one part to a point in the
//! world, pulling the part across the ground (the original's horizontal
//! drag link, which code adds to a body for the player's hand).
//!
//! The grabbed point is kept as weights on the part's particles, from the
//! point in the part's frame where the grab took it. In each pass of a limp
//! body's iterations that the grab runs in, after the part's own links, the
//! gap to the anchor is taken across the ground only (in cm, its height
//! zeroed). Under 1.19209e-7 cm² it does nothing, and a link with a slack
//! does nothing while its 3-D gap is within that slack. Otherwise the pull is
//! twice the strength times the gap, the gap counted at most 3·√mobility cm
//! and never more than 5 cm. When the point's motion since the tick began heads
//! toward the anchor (its share along the gap's direction is over
//! 1.19209e-7), that share is taken off the pull. The pull moves each
//! particle by its weight, taken by the part alone (no share with another).
//! A grab with a last pass runs in the passes up to it (counted from 0 each
//! tick), and in every pass otherwise. A living body's grabs are left alone:
//! the hand never takes hold of one.
//!
//! Evidence: the original's limp-body link iteration (the horizontal drag
//! branch and the pass limit) and the link it adds for the hand (the point
//! in the part's frame, the world anchor, zero slack), read in the
//! instructions; `context/re/body-links.md` §6.4 has the notes.

use bevy::prelude::*;

use super::{CM, Ragdoll, weights};

/// The squared gap (cm²) and the approaching share (cm) at or under which
/// the original's link code does nothing: its single-precision epsilon.
const EPSILON: f32 = 1.192_09e-7;
/// The pull's length cap (cm) is this times the root of the part's
/// mobility, and never more than [`MAX_STEP`].
const STEP_PER_ROOT_MOBILITY: f32 = 3.0;
const MAX_STEP: f32 = 5.0;
/// The pull is this many times the strength times the gap.
const PULL: f32 = 2.0;

/// A hand's hold on a body: a point on a part pulled toward an anchor in the
/// world.
#[derive(Debug, Clone, PartialEq)]
pub struct Grab {
    pub part: usize,
    /// The grabbed point, as weights on the part's particles.
    on_part: [f32; 4],
    /// Where the point is pulled to (m).
    pub anchor: Vec3,
    pub strength: f32,
    /// The grab runs in the tick's passes up to this one (from 0), or in
    /// every pass.
    pub last_pass: Option<u32>,
    /// The 3-D gap (m) the point may drift from the anchor before the link
    /// pulls, as the original's slack. Zero for the hand's own drag.
    pub slack: f32,
}

impl Ragdoll {
    /// Takes hold of `part` at the world point `at` (m), pulled toward `at`
    /// until the anchor moves, with `strength` and, when given, only up to
    /// `last_pass`. The grab is the body's last. Returns whether the part
    /// exists.
    pub fn grab(&mut self, part: usize, at: Vec3, strength: f32, last_pass: Option<u32>) -> bool {
        self.grab_with_slack(part, at, strength, last_pass, 0.0)
    }

    /// Takes hold as [`Ragdoll::grab`] does, with `slack` (m): the point
    /// drifts that far from the anchor before the link pulls.
    pub fn grab_with_slack(
        &mut self,
        part: usize,
        at: Vec3,
        strength: f32,
        last_pass: Option<u32>,
        slack: f32,
    ) -> bool {
        let Some(p) = self.parts.get(part) else {
            return false;
        };
        let local = p.transform().inverse().transform_point3(at);
        let on_part = weights(p, local);
        self.grabs.push(Grab {
            part,
            on_part,
            anchor: at,
            strength,
            last_pass,
            slack,
        });
        true
    }

    /// The body's grabs, the last added last.
    pub fn grabs(&self) -> &[Grab] {
        &self.grabs
    }

    /// Moves the last grab's anchor to `anchor` (m).
    pub fn pull_grab_to(&mut self, anchor: Vec3) {
        if let Some(g) = self.grabs.last_mut() {
            g.anchor = anchor;
        }
    }

    /// Puts the last grab's anchor where its point is, so it pulls nothing.
    pub fn hold_grab_still(&mut self) {
        if let Some(at) = self.grab_point()
            && let Some(g) = self.grabs.last_mut()
        {
            g.anchor = at;
        }
    }

    /// Where the last grab's point is now (m).
    pub fn grab_point(&self) -> Option<Vec3> {
        let g = self.grabs.last()?;
        self.parts.get(g.part).map(|p| p.point(&g.on_part))
    }

    /// Lets go of the last grab. Returns whether there was one.
    pub fn release_grab(&mut self) -> bool {
        self.grabs.pop().is_some()
    }

    /// Turns every part about the world point `centre` by `rotation`, and
    /// places the body there, as the original's held item cursor writes its
    /// node's matrix: a held item may be turned without being let go.
    pub fn turn_about(&mut self, centre: Vec3, rotation: bevy::math::Mat3) {
        let around = bevy::math::Affine3A::from_translation(centre)
            * bevy::math::Affine3A::from_mat3(rotation)
            * bevy::math::Affine3A::from_translation(-centre);
        let worlds: Vec<bevy::math::Affine3A> =
            self.parts.iter().map(|p| around * p.transform()).collect();
        self.place_at(&worlds);
    }

    /// Part `i`'s grabs in this pass of a limp body's iterations.
    pub(super) fn pull_grabs(&mut self, i: usize) {
        let pass = self.pass;
        for g in &self.grabs {
            if g.part != i || g.last_pass.is_some_and(|last| pass > last) {
                continue;
            }
            let part = &mut self.parts[i];
            let now = part.point(&g.on_part);
            let moved = (now - part.point_at_start(&g.on_part)) * CM;
            if let Some(pull) = drag_pull(
                (g.anchor - now) * CM,
                moved,
                part.inverse_mass,
                g.strength,
                g.slack * CM,
            ) {
                part.push(&g.on_part, pull / CM);
            }
        }
    }
}

/// The horizontal drag's correction (cm) of a point `gap` (cm, anchor minus
/// point) from its anchor, whose point moved `moved` (cm) since the tick
/// began, on a part of `mobility`, with `strength`; `None` when the gap
/// across the ground is under the epsilon.
pub fn drag_pull(gap: Vec3, moved: Vec3, mobility: f32, strength: f32, slack: f32) -> Option<Vec3> {
    if gap.length() <= slack {
        return None;
    }
    let mut d = Vec3::new(gap.x, 0.0, gap.z);
    let len2 = d.x * d.x + d.y * d.y + d.z * d.z;
    if len2 < EPSILON {
        return None;
    }
    let len = len2.sqrt();
    let unit = d * (1.0 / len);
    let mut cap = mobility.sqrt() * STEP_PER_ROOT_MOBILITY;
    if cap > MAX_STEP {
        cap = MAX_STEP;
    }
    if cap < len {
        d *= cap / len * strength * PULL;
    } else {
        d *= strength * PULL;
    }
    let toward = moved.x * unit.x + moved.y * unit.y + moved.z * unit.z;
    if toward > EPSILON {
        d += unit * toward * -1.0;
    }
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::RigidPart;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    /// The pull: twice the strength times the gap within the cap, the cap's
    /// length beyond it, across the ground only.
    #[test]
    fn the_pull_is_twice_the_strength_within_the_cap() {
        // Mobility 1: the cap is 3 cm. A 2 cm gap at strength 1 pulls 4 cm.
        let p = drag_pull(Vec3::new(2.0, 9.0, 0.0), Vec3::ZERO, 1.0, 1.0, 0.0).unwrap();
        assert!(close(p, Vec3::new(4.0, 0.0, 0.0)), "{p}");
        // 10 cm away: 3/10 of it, twice: 6 cm.
        let p = drag_pull(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, 1.0, 1.0, 0.0).unwrap();
        assert!(close(p, Vec3::new(0.0, 0.0, 6.0)), "{p}");
        // Mobility 4 would cap at 6 cm: never over 5. Strength 0.5: 5 cm.
        let p = drag_pull(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO, 4.0, 0.5, 0.0).unwrap();
        assert!(close(p, Vec3::new(5.0, 0.0, 0.0)), "{p}");
        // Mobility 0.08 caps at 3·√0.08 = 0.8485 cm.
        let p = drag_pull(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO, 0.08, 1.0, 0.0).unwrap();
        assert!((p.x - 2.0 * 3.0 * 0.08f32.sqrt()).abs() < 1e-5, "{p}");
        // A gap straight up, or under the epsilon, pulls nothing.
        assert_eq!(drag_pull(Vec3::Y * 50.0, Vec3::ZERO, 1.0, 1.0, 0.0), None);
        assert_eq!(drag_pull(Vec3::X * 3e-4, Vec3::ZERO, 1.0, 1.0, 0.0), None);
        assert!(drag_pull(Vec3::X * 4e-4, Vec3::ZERO, 1.0, 1.0, 0.0).is_some());
        // A link with 0.3 cm of slack does nothing within 3 mm of the
        // anchor, even across the ground, and pulls past it.
        assert_eq!(
            drag_pull(Vec3::new(0.2, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0, 0.3),
            None
        );
        assert!(drag_pull(Vec3::new(0.4, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0, 0.3).is_some());
        // The slack is 3-D: 0.25 cm up is also inside it.
        assert_eq!(
            drag_pull(Vec3::new(0.0, 0.25, 0.0), Vec3::ZERO, 1.0, 1.0, 0.3),
            None
        );
    }

    /// Motion already heading toward the anchor is taken off the pull; motion
    /// away from it is not.
    #[test]
    fn approaching_motion_is_taken_off() {
        let p = drag_pull(
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(1.5, 2.0, 0.5),
            1.0,
            1.0,
            0.0,
        )
        .unwrap();
        assert!(close(p, Vec3::new(4.5, 0.0, 0.0)), "{p}");
        let p = drag_pull(
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(-1.5, 0.0, 0.0),
            1.0,
            1.0,
            0.0,
        )
        .unwrap();
        assert!(close(p, Vec3::new(6.0, 0.0, 0.0)), "{p}");
    }

    fn crate_body() -> Ragdoll {
        Ragdoll::rigid(&[RigidPart {
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: bevy::math::Affine3A::IDENTITY,
            rest: [
                Vec3::new(-0.2, 0.0, -0.2),
                Vec3::new(0.2, 0.0, -0.2),
                Vec3::new(0.0, 0.0, 0.2),
                Vec3::new(0.0, 0.4, 0.0),
            ],
            point_masses: [4.0; 4],
            parent_world: bevy::math::Affine3A::IDENTITY,
            mobility: 1.0,
            spheres: vec![(Vec3::new(0.0, 0.2, 0.0), 0.2)],
            origin: Vec3::ZERO,
            links: Vec::new(),
            mesh: None,
        }])
    }

    /// A grab keeps its point on the part, and a pass moves the part by the
    /// pull; a grab with a last pass stops after it.
    #[test]
    fn a_grab_pulls_its_part_in_its_passes() {
        let mut body = crate_body();
        let at = Vec3::new(0.0, 0.2, 0.0);
        assert!(body.grab(0, at, 1.0, Some(1)));
        assert!(!body.grab(3, at, 1.0, None));
        assert!(close(body.grab_point().unwrap(), at));
        body.pull_grab_to(at + Vec3::new(0.5, 1.0, 0.0));
        let before = body.grab_point().unwrap();
        for pass in 0..3 {
            body.pass = pass;
            body.pull_grabs(0);
        }
        // Two passes, none in the third. The first pulls 6 cm along x (the
        // 3 cm cap, twice); a push moves the point by Σw² of it. The second
        // pulls 6 cm less the first's motion toward the anchor.
        let w2: f32 = body.grabs[0].on_part.iter().map(|w| w * w).sum();
        let first = 6.0 * w2;
        let want = (first + (6.0 - first) * w2) / CM;
        let moved = body.grab_point().unwrap() - before;
        assert!((moved.x - want).abs() < 1e-5, "{moved} vs {want}");
        assert!(moved.y.abs() < 1e-6 && moved.z.abs() < 1e-6, "{moved}");
        body.hold_grab_still();
        assert!(close(body.grabs[0].anchor, body.grab_point().unwrap()));
        assert!(body.release_grab());
        assert!(!body.release_grab());
        assert_eq!(body.grab_point(), None);
    }

    /// Turning a held body about a world point rotates its rest points
    /// around it, and leaves its centre where it was.
    #[test]
    fn a_held_body_turns_about_a_point() {
        let mut body = crate_body();
        // The part's rest points are around the origin; turn a quarter
        // about Y at the origin: (-0.2, 0, -0.2) goes to (-0.2, 0, 0.2).
        let yaw = bevy::math::Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2);
        body.turn_about(Vec3::ZERO, yaw);
        let at = body.parts[0].transform().translation;
        assert!(at.length() < 1e-6, "{at}");
        // A rest point at (-0.2, 0, -0.2) is now at (-0.2, 0, 0.2).
        let p = body.parts[0].positions[0];
        assert!((p - Vec3::new(-0.2, 0.0, 0.2)).length() < 1e-5, "{p}");
        // Turning about (1, 0, 0) moves the part at the origin to
        // (1, 0, 1): the point a quarter turn away round the centre.
        let mut body = crate_body();
        body.turn_about(Vec3::new(1.0, 0.0, 0.0), yaw);
        let at = body.parts[0].transform().translation;
        assert!(
            (Vec3::from(at) - Vec3::new(1.0, 0.0, 1.0)).length() < 1e-6,
            "{at}"
        );
    }
}
