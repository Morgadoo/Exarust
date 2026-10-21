//! Measurements of a body for the debug tools: each joint's turn from its
//! rest, split into a swing of the part's bone axis and a twist about it;
//! each link's gap against its slack; where a living link muscle's anchor
//! is. Nothing here changes the body, and nothing calls it unless a debug
//! tool is on.
//! (Included from body.rs: `#[path = "body_inspect.rs"] pub mod inspect;`)

use bevy::prelude::*;

use super::{FootState, Link, PIVOT_SLACK, Ragdoll, Sleep};

/// The six edges of a part's tetrahedron, by particle.
pub const TETRA_EDGES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];

/// How far past its slack (m) a link may be and still count as within it:
/// the solver's softness leaves a pivot a fraction of a millimetre open.
pub const STRETCH_TOLERANCE: f32 = 0.001;
/// How far past its slack (m) a link is only slightly stretched.
pub const STRETCH_SLIGHT: f32 = 0.01;

/// A link's gap against its slack, as the overlay colours it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stretch {
    /// Within its slack (at most 1 mm past it).
    Within,
    /// Up to 1 cm past its slack.
    Slight,
    /// More than 1 cm past its slack.
    Over,
}

/// The band of a link `excess` metres past its slack (negative inside it).
pub fn stretch(excess: f32) -> Stretch {
    if excess.is_nan() || excess <= STRETCH_TOLERANCE {
        Stretch::Within
    } else if excess <= STRETCH_SLIGHT {
        Stretch::Slight
    } else {
        Stretch::Over
    }
}

/// A joint's turn from its rest, in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointAngles {
    /// The whole turn, 0 to π.
    pub total: f32,
    /// The turn that tilts the bone axis away from where the rest puts it,
    /// 0 to π.
    pub swing: f32,
    /// The turn about the bone axis, −π to π, right-handed about it.
    pub twist: f32,
    /// The unit axis of the swing in the part's space (perpendicular to the
    /// bone axis), zero without a swing.
    pub swing_axis: Vec3,
}

/// Splits `turn`, a part's rotation from its rest relative to its parent
/// (acting in the part's own space: posed = rest · turn), into a twist about
/// `axis` (the part's bone axis in its own space) followed by a swing that
/// tilts the axis: turn = swing · twist.
pub fn joint_angles(turn: Quat, axis: Vec3) -> JointAngles {
    let axis = axis.normalize_or(Vec3::Y);
    // The shorter way round.
    let q = if turn.w < 0.0 { -turn } else { turn };
    let q = q.normalize();
    let along = q.xyz().dot(axis);
    let twist = Quat::from_xyzw(axis.x * along, axis.y * along, axis.z * along, q.w);
    let twist = if twist.length_squared() < 1e-12 {
        // A half turn about an axis across the bone: no twist.
        Quat::IDENTITY
    } else {
        twist.normalize()
    };
    let swing = q * twist.inverse();
    let swing = if swing.w < 0.0 { -swing } else { swing };
    let swing_vector = swing.xyz();
    JointAngles {
        total: angle(q),
        swing: angle(swing),
        twist: 2.0 * along.atan2(q.w),
        swing_axis: swing_vector.normalize_or_zero(),
    }
}

/// The angle (rad, 0 to π) of a rotation, precise near zero.
fn angle(q: Quat) -> f32 {
    2.0 * q.xyz().length().atan2(q.w.abs())
}

/// The angle (rad, 0 to π) between two rotations.
pub fn angle_between(a: Quat, b: Quat) -> f32 {
    angle((a.inverse() * b).normalize())
}

/// The name of a bone of the human skeleton.
pub fn bone_name(bone: usize) -> &'static str {
    match bone {
        1 => "pelvis",
        2 => "R thigh",
        3 => "R shin",
        4 => "R foot",
        5 => "L thigh",
        6 => "L shin",
        7 => "L foot",
        8 => "belly",
        9 => "chest",
        10 => "head",
        11 => "R shoulder",
        12 => "R upper arm",
        13 => "R forearm",
        14 => "L shoulder",
        15 => "L upper arm",
        16 => "L forearm",
        17 => "R hand",
        18 => "L hand",
        _ => "?",
    }
}

impl Sleep {
    /// The state's name in the debug tools.
    pub fn name(self) -> &'static str {
        match self {
            Sleep::Awake => "awake",
            Sleep::Asleep => "asleep",
            Sleep::Deep => "deep",
        }
    }
}

/// A link's state where the last tick left it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkReport {
    /// Its index in [`Ragdoll::links`].
    pub index: usize,
    /// Its place among its part's links (the origin pivot last).
    pub rank: usize,
    /// The pivot at the part's origin.
    pub origin: bool,
    /// A pivot (no slack).
    pub pivot: bool,
    /// Its slack (m).
    pub slack: f32,
    /// How far apart its two points are (m).
    pub gap: f32,
}

impl LinkReport {
    /// How far past its slack the link is (m), negative inside it.
    pub fn excess(&self) -> f32 {
        self.gap - self.slack
    }

    /// Its band in the overlay.
    pub fn stretch(&self) -> Stretch {
        stretch(self.excess())
    }
}

/// A part's joint where the last tick left it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointReport {
    /// Its turn from its rest relative to its parent.
    pub angles: JointAngles,
    /// A living body's: how far (rad) its turn is from the one the clip
    /// asks of it, which its link muscles' anchors stand for.
    pub anchor_error: Option<f32>,
    /// A living body's: the clip's turn from the rest, split the same way.
    pub clip: Option<JointAngles>,
}

impl Ragdoll {
    /// Sets each part's rest turn relative to its parent (the model's rest
    /// pose), from which the debug tools measure its joint; identity when
    /// not set.
    pub fn with_rest_turns(mut self, turns: &[Quat]) -> Self {
        if turns.len() == self.parts.len() {
            for (p, t) in self.parts.iter_mut().zip(turns) {
                p.rest_turn = t.normalize();
            }
        }
        self
    }

    /// The part's rest turn relative to its parent.
    pub fn rest_turn(&self, part: usize) -> Quat {
        self.parts.get(part).map_or(Quat::IDENTITY, |p| p.rest_turn)
    }

    /// The part's turn relative to its parent where the last tick left
    /// them; none for a part without a parent.
    pub fn relative_turn(&self, part: usize) -> Option<Quat> {
        let p = self.parts.get(part)?;
        let parent = self.parts.get(p.parent?)?;
        let turn = |t: bevy::math::Affine3A| Quat::from_mat3a(&t.matrix3).normalize();
        Some((turn(parent.transform()).inverse() * turn(p.transform())).normalize())
    }

    /// The part's bone axis in its own space: from its origin (the joint)
    /// to the mean of its child parts' origins, or to its centre when it
    /// has none.
    pub fn bone_axis(&self, part: usize) -> Vec3 {
        let Some(p) = self.parts.get(part) else {
            return Vec3::Y;
        };
        let (sum, count) = self
            .parts
            .iter()
            .filter(|c| c.parent == Some(part))
            .fold((Vec3::ZERO, 0), |(s, n), c| (s + c.origin, n + 1));
        let toward = if count > 0 {
            sum / count as f32
        } else {
            p.rest.iter().copied().sum::<Vec3>() * 0.25
        };
        toward.normalize_or(Vec3::Y)
    }

    /// The part's joint where the last tick left it: its turn from its rest
    /// and, in a living body, how far that is from the clip's turn. None
    /// for a part without a parent.
    pub fn joint(&self, part: usize) -> Option<JointReport> {
        let now = self.relative_turn(part)?;
        let (rest, axis) = (self.rest_turn(part), self.bone_axis(part));
        let angles = joint_angles(rest.inverse() * now, axis);
        let wanted = self.drive.as_ref().and_then(|d| {
            let parent = self.parts[part].parent?;
            let (t, tp) = (d.targets.get(part)?, d.targets.get(parent)?);
            let (_, wanted, _) = (tp.inverse() * *t).to_scale_rotation_translation();
            Some(wanted)
        });
        Some(JointReport {
            angles,
            anchor_error: wanted.map(|w| angle_between(now, w)),
            clip: wanted.map(|w| joint_angles(rest.inverse() * w, axis)),
        })
    }

    /// The link at `index` where the last tick left it.
    pub fn link_report(&self, index: usize) -> Option<LinkReport> {
        let l = self.links.get(index)?;
        let first = self.links_of.get(l.part).map_or(index, |r| r.start);
        let (a, b) = self.link_ends(l);
        Some(LinkReport {
            index,
            rank: index - first,
            origin: l.origin,
            pivot: l.slack <= PIVOT_SLACK,
            slack: l.slack,
            gap: a.distance(b),
        })
    }

    /// The link of the part at `part` furthest past its slack.
    pub fn worst_link(&self, part: usize) -> Option<LinkReport> {
        let range = self.links_of.get(part)?.clone();
        range
            .filter_map(|k| self.link_report(k))
            .max_by(|a, b| a.excess().total_cmp(&b.excess()))
    }

    /// How much of its range the part's joint uses: the largest share of
    /// its slack any of its slack links is open (1 at the limit), and that
    /// link.
    pub fn range_used(&self, part: usize) -> Option<(f32, LinkReport)> {
        let range = self.links_of.get(part)?.clone();
        range
            .filter_map(|k| self.link_report(k))
            .filter(|l| !l.pivot && l.slack > 0.0)
            .map(|l| (l.gap / l.slack, l))
            .max_by(|a, b| a.0.total_cmp(&b.0))
    }

    /// The part's pivot at its origin.
    pub fn origin_pivot(&self, part: usize) -> Option<LinkReport> {
        let range = self.links_of.get(part)?.clone();
        range
            .filter(|&k| self.links[k].origin)
            .find_map(|k| self.link_report(k))
    }

    /// The link of the whole body furthest past its slack.
    pub fn worst_link_of_body(&self) -> Option<LinkReport> {
        (0..self.links.len())
            .filter_map(|k| self.link_report(k))
            .max_by(|a, b| a.excess().total_cmp(&b.excess()))
    }

    /// A link's two points (world) where the last tick left them: on its
    /// part and on its parent.
    fn link_ends(&self, l: &Link) -> (Vec3, Vec3) {
        (
            self.parts[l.part].point(&l.on_part),
            self.parts[l.parent].point(&l.on_parent),
        )
    }

    /// The four particles of a part as it is drawn, `alpha` of the way
    /// between its last two ticks.
    pub fn drawn_particles(&self, part: usize, alpha: f32) -> [Vec3; 4] {
        let p = &self.parts[part];
        let t = p.drawn(alpha);
        p.rest.map(|r| t.transform_point3(r))
    }

    /// A link's two points (world) as the parts are drawn: on its part and
    /// on its parent.
    pub fn drawn_link(&self, index: usize, alpha: f32) -> Option<(Vec3, Vec3)> {
        let l = self.links.get(index)?;
        let (part, parent) = (&self.parts[l.part], &self.parts[l.parent]);
        Some((
            part.drawn(alpha)
                .transform_point3(part.rest_point(&l.on_part)),
            parent
                .drawn(alpha)
                .transform_point3(parent.rest_point(&l.on_parent)),
        ))
    }

    /// Where a living link muscle's anchor is (world) as the parts are
    /// drawn: where the clip's joint turn puts the part's point.
    pub fn drawn_anchor(&self, index: usize, alpha: f32) -> Option<Vec3> {
        self.drive.as_ref()?;
        let l = self.links.get(index)?;
        let anchor = l.anchor.filter(|_| l.slack > PIVOT_SLACK)?;
        let parent = &self.parts[l.parent];
        Some(
            parent
                .drawn(alpha)
                .transform_point3(parent.rest_point(&anchor)),
        )
    }

    /// A living body's foot contact `leg` as its lower leg is drawn: the
    /// contact (world), its anchor and its state.
    pub fn drawn_foot(&self, leg: usize, alpha: f32) -> Option<(Vec3, Vec3, FootState)> {
        self.drive.as_ref()?;
        let leg = self.legs.get(leg)?;
        let shin = &self.parts[leg.shin];
        let at = shin
            .drawn(alpha)
            .transform_point3(shin.rest_point(&leg.contact));
        Some((at, leg.anchor, leg.state))
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_3, FRAC_PI_4, PI};

    use bevy::math::Affine3A;

    use super::*;
    use crate::body::PartSetup;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn the_rest_pose_is_no_turn() {
        for axis in [Vec3::Y, Vec3::NEG_X, Vec3::new(0.3, -0.2, 0.9)] {
            let a = joint_angles(Quat::IDENTITY, axis);
            assert_eq!((a.total, a.swing, a.twist), (0.0, 0.0, 0.0));
            assert_eq!(a.swing_axis, Vec3::ZERO);
        }
        // Measured from a rest that is not the identity.
        let rest = Quat::from_euler(EulerRot::YXZ, 0.4, -0.7, 1.1);
        let a = joint_angles(rest.inverse() * rest, Vec3::X);
        assert!(close(a.total, 0.0) && close(a.swing, 0.0) && close(a.twist, 0.0));
    }

    #[test]
    fn a_turn_about_the_bone_is_twist() {
        let axis = Vec3::new(1.0, 2.0, -2.0) / 3.0;
        for angle in [0.3, -1.2, FRAC_PI_2, 3.0] {
            let a = joint_angles(Quat::from_axis_angle(axis, angle), axis);
            assert!(close(a.twist, angle), "{angle}: {a:?}");
            assert!(close(a.swing, 0.0) && close(a.total, angle.abs()), "{a:?}");
        }
    }

    #[test]
    fn a_turn_across_the_bone_is_swing() {
        // An elbow along −X bending about Y, either way.
        for (angle, axis) in [(FRAC_PI_3, Vec3::Y), (-FRAC_PI_4, Vec3::Y), (2.5, Vec3::Z)] {
            let a = joint_angles(Quat::from_axis_angle(axis, angle), Vec3::NEG_X);
            assert!(close(a.swing, angle.abs()) && close(a.total, angle.abs()));
            assert!(close(a.twist, 0.0), "{a:?}");
            assert!(
                a.swing_axis.abs_diff_eq(axis * angle.signum(), 1e-4),
                "{a:?}"
            );
        }
        // A half turn across the bone.
        let a = joint_angles(Quat::from_rotation_z(PI), Vec3::Y);
        assert!(close(a.swing, PI) && close(a.twist, 0.0), "{a:?}");
    }

    #[test]
    fn swing_and_twist_split_a_turn() {
        let axis = Vec3::Y;
        for (swing, twist) in [(0.5, 0.8), (1.4, -0.6), (0.1, 2.9)] {
            let turn = Quat::from_rotation_x(swing) * Quat::from_rotation_y(twist);
            let a = joint_angles(turn, axis);
            assert!(close(a.swing, swing) && close(a.twist, twist), "{a:?}");
            assert!(a.swing_axis.abs_diff_eq(Vec3::X, 1e-4));
            let total = 2.0 * turn.w.abs().acos();
            assert!(close(a.total, total));
            assert!(a.total <= a.swing + a.twist.abs() + 1e-4);
        }
        // The sign of the quaternion does not matter.
        let turn = Quat::from_rotation_z(0.7) * Quat::from_rotation_y(-0.4);
        assert_eq!(joint_angles(turn, axis), joint_angles(-turn, axis));
    }

    #[test]
    fn angles_between_turns() {
        let a = Quat::from_rotation_y(0.3);
        assert!(close(angle_between(a, a), 0.0));
        assert!(close(angle_between(a, Quat::from_rotation_y(1.0)), 0.7));
        assert!(close(angle_between(Quat::IDENTITY, -Quat::IDENTITY), 0.0));
    }

    #[test]
    fn stretch_bands() {
        for (excess, band) in [
            (-0.05, Stretch::Within),
            (0.0, Stretch::Within),
            (0.001, Stretch::Within),
            (0.0011, Stretch::Slight),
            (0.01, Stretch::Slight),
            (0.0101, Stretch::Over),
            (0.5, Stretch::Over),
            (f32::NAN, Stretch::Within),
        ] {
            assert_eq!(stretch(excess), band, "{excess}");
        }
    }

    /// A pelvis and a thigh hanging under it, pinned at the hip, with a
    /// slack link 30 cm down the thigh, `slack` long.
    fn hip(slack: f32) -> Ragdoll {
        let e = Entity::PLACEHOLDER;
        let at = |y: f32| Affine3A::from_translation(Vec3::new(0.0, y, 0.0));
        Ragdoll::new(&[
            PartSetup {
                bone: 1,
                entity: e,
                parent: None,
                world: at(1.0),
                centre: Vec3::ZERO,
                half: Vec3::splat(0.1),
                origin: Vec3::ZERO,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            },
            PartSetup {
                bone: 2,
                entity: e,
                parent: Some(0),
                world: at(0.9),
                centre: Vec3::new(0.0, -0.2, 0.0),
                half: Vec3::new(0.06, 0.22, 0.06),
                origin: Vec3::new(0.0, -0.1, 0.0),
                links: vec![(Vec3::new(0.0, -0.3, 0.0), Vec3::new(0.0, -0.3, 0.0), slack)],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            },
        ])
    }

    #[test]
    fn a_body_reports_its_joints_and_links() {
        let mut body = hip(0.05);
        assert!(body.joint(0).is_none() && body.relative_turn(0).is_none());
        // Built as posed: no turn, every link closed.
        let j = body.joint(1).unwrap();
        assert!(close(j.angles.total, 0.0) && j.anchor_error.is_none() && j.clip.is_none());
        assert!(body.bone_axis(1).abs_diff_eq(Vec3::NEG_Y, 1e-4));
        assert!(body.bone_axis(0).abs_diff_eq(Vec3::NEG_Y, 1e-4));
        let links: Vec<LinkReport> = (0..2).map(|k| body.link_report(k).unwrap()).collect();
        assert_eq!(
            (links[0].rank, links[0].origin, links[0].pivot),
            (0, false, false)
        );
        assert_eq!(
            (links[1].rank, links[1].origin, links[1].pivot),
            (1, true, true)
        );
        assert!(
            links
                .iter()
                .all(|l| l.gap < 1e-5 && l.stretch() == Stretch::Within)
        );
        // Swing the thigh 60° about X at the hip.
        let hip = Vec3::new(0.0, 0.9, 0.0);
        let swing = Affine3A::from_translation(hip)
            * Affine3A::from_rotation_x(FRAC_PI_3)
            * Affine3A::from_translation(-hip);
        for x in &mut body.parts[1].positions {
            *x = swing.transform_point3(*x);
        }
        let j = body.joint(1).unwrap();
        assert!(close(j.angles.swing, FRAC_PI_3) && close(j.angles.twist, 0.0));
        assert!(j.angles.swing_axis.abs_diff_eq(Vec3::X, 1e-4));
        // The link 30 cm down swings 30 cm out of its 5 cm slack; the pivot
        // stays closed.
        let worst = body.worst_link(1).unwrap();
        assert_eq!(worst.index, 0);
        assert!(close(worst.gap, 0.3) && close(worst.excess(), 0.25));
        assert_eq!(worst.stretch(), Stretch::Over);
        assert!(body.link_report(1).unwrap().gap < 1e-5);
        assert_eq!(body.worst_link_of_body(), Some(worst));
        let (used, link) = body.range_used(1).unwrap();
        assert!(close(used, 6.0) && link.index == 0, "{used}");
        assert_eq!(body.origin_pivot(1).unwrap().index, 1);
        assert!(body.range_used(0).is_none() && body.origin_pivot(0).is_none());
        // Rest turns move where the joint is measured from.
        let body = body.with_rest_turns(&[Quat::IDENTITY, Quat::from_rotation_x(FRAC_PI_3)]);
        assert!(close(body.joint(1).unwrap().angles.total, 0.0));
    }

    #[test]
    fn a_living_body_reports_its_anchors() {
        let mut body = hip(0.05).with_drive();
        let rest = body.parts.iter().map(|p| p.transform()).collect::<Vec<_>>();
        assert!(body.drawn_anchor(0, 1.0).is_none());
        // The clip swings the thigh 90° about X; the body has not followed.
        let hip = Affine3A::from_translation(Vec3::new(0.0, 0.9, 0.0));
        let wanted = vec![rest[0], hip * Affine3A::from_rotation_x(FRAC_PI_2)];
        body.aim(Affine3A::IDENTITY, wanted);
        let j = body.joint(1).unwrap();
        assert!(close(j.anchor_error.unwrap(), FRAC_PI_2), "{j:?}");
        let clip = j.clip.unwrap();
        assert!(close(clip.swing, FRAC_PI_2) && clip.swing_axis.abs_diff_eq(Vec3::X, 1e-4));
        // The anchor is the link's point on the thigh, swung 90° about X
        // at the hip: 30 cm from it along −Z.
        let anchor = body.drawn_anchor(0, 1.0).unwrap();
        assert!(
            anchor.abs_diff_eq(Vec3::new(0.0, 0.9, -0.3), 1e-4),
            "{anchor}"
        );
        let (on_part, on_parent) = body.drawn_link(0, 1.0).unwrap();
        assert!(on_part.abs_diff_eq(Vec3::new(0.0, 0.6, 0.0), 1e-4));
        assert!(on_parent.abs_diff_eq(on_part, 1e-4));
        let particles = body.drawn_particles(1, 0.5);
        for (drawn, now) in particles.iter().zip(body.parts[1].positions) {
            assert!(drawn.abs_diff_eq(now, 1e-4));
        }
    }
}
