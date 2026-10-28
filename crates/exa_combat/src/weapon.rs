//! The held weapon's handling forces, as the original's body tick runs them
//! at the start of every attack (the routine that keeps the weapon in the
//! swing's plane): the blade's bend and lift curves, and the two pushes they
//! give the weapon body and its forearm, along the swing's guide direction.
//! The weighting of a push at a point is the original's too: a push into the
//! point acts in full, one partly into it in part, and one away from it not
//! at all. Rules and constants: reverse engineering of the held-weapon
//! handling routine and the push-at-point helper.
//!
//! A plain weapon model now has a solver body, but nothing applies these
//! forces to it yet: the step is tested with its own numbers. Lengths are the original's centimetres.

use bevy::math::Affine3A;
use bevy::prelude::*;

/// The guide point's offset from the body's frame, in its space (cm).
const GUIDE_OFFSET: Vec3 = Vec3::new(-20.0, 10.0, 0.0);
/// The blade's lift curve: at or under 0.1 none; otherwise
/// (p − 0.1)·1.65, then (3 − 2t)t², floored at 0.
const LIFT_FROM: f64 = 0.1;
const LIFT_SLOPE: f64 = 1.65;
/// The blade's bend curve: at or under 0.4 none; otherwise
/// (p − 0.4), (3 − 2t)t·(1 − p)²·4, floored at 0.
const BEND_FROM: f64 = 0.4;
const BEND_SHARE: f64 = 4.0;
/// The weapon's push is the guide's share × the lift × p × 6, plus past
/// 0.4 the bend × (W7 + 0.8) × 48; the forearm's is × p² × lift, and its
/// bend term × (1 − p)² × 12.
const WEAPON_LIFT: f64 = 6.0;
const WEAPON_BEND: f64 = 48.0;
const FOREARM_BEND: f64 = 12.0;
const BALANCE_OFFSET: f64 = 0.8;
/// Under this share of the clip no push is made.
const HANDLING_FROM: f64 = 0.1;

/// A value floored at 0 as the original does it: only a value below 0 is
/// replaced, so a NaN stays.
fn floored(x: f32) -> f32 {
    if x < 0.0 { 0.0 } else { x }
}

/// The blade's lift factor at `progress`: 0 up to 0.1 (and for a NaN),
/// else (3 − 2t)·t² with t = (p − 0.1)·1.65, floored at 0.
pub fn blade_lift(progress: f32) -> f32 {
    if f64::from(progress) <= LIFT_FROM || progress.is_nan() {
        return 0.0;
    }
    let t = ((f64::from(progress) - LIFT_FROM) * LIFT_SLOPE) as f32;
    floored((3.0 - 2.0 * t) * (t * t))
}

/// The blade's bend factor at `progress`: 0 up to 0.4 (and for a NaN),
/// else (3 − 2t)t·(1 − p)²·4 with t = p − 0.4, floored at 0.
pub fn blade_bend(progress: f32) -> f32 {
    if f64::from(progress) <= BEND_FROM || progress.is_nan() {
        return 0.0;
    }
    let t = (f64::from(progress) - BEND_FROM) as f32;
    let left = 1.0 - progress;
    floored((3.0 - 2.0 * t) * t * (left * left) * BEND_SHARE as f32)
}

/// The two pushes the held-weapon handling block makes at `progress` of an
/// attack's clip: the weapon's at `weapon_point` and the forearm's at
/// `hand_point` (world, cm), along the guide direction from the frame's
/// offset to `guide_point`; `balance` is the weapon's W7. None before 0.1.
/// The caller makes them only while a weapon is held and the body is not
/// staggered.
///
/// Each push is its point's offset from the guide's origin without its part
/// along the guide, scaled (the weapon's by lift·p·6, the forearm's by
/// p²·lift), plus, once the clip is past 0.4 (the single-precision 0.4),
/// the guide direction scaled by the bend and the balance (the weapon's
/// (W7 + 0.8)·48, the forearm's (W7 + 0.8)·(1 − p)²·12, each rounded to
/// single before the bend multiplies it).
pub fn handling_pushes(
    progress: f32,
    balance: f32,
    frame: Affine3A,
    guide_point: Vec3,
    weapon_point: Vec3,
    hand_point: Vec3,
) -> Option<[Vec3; 2]> {
    if f64::from(progress) <= HANDLING_FROM {
        return None;
    }
    let origin = frame.transform_point3(GUIDE_OFFSET);
    let dir = (guide_point - origin).normalize_or_zero();
    let lift = blade_lift(progress);
    let bend = blade_bend(progress);
    let p = progress;
    let side_to = |point: Vec3| {
        let d = origin - point;
        let dot = d.x * dir.x + d.y * dir.y + d.z * dir.z;
        d + dir * -dot
    };
    let bending = p > BEND_FROM as f32;
    let mut weapon_push = side_to(weapon_point) * (lift * p * WEAPON_LIFT as f32);
    if bending {
        let scale = ((f64::from(balance) + BALANCE_OFFSET) * WEAPON_BEND) as f32;
        weapon_push += dir * (bend * scale);
    }
    let mut forearm_push = side_to(hand_point) * (p * p * lift);
    if bending {
        let left = 1.0 - p;
        let scale = ((f64::from(balance) + BALANCE_OFFSET)
            * f64::from(left * left * FOREARM_BEND as f32)) as f32;
        forearm_push += dir * (bend * scale);
    }
    Some([weapon_push, forearm_push])
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_physics::body::push_fraction;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    /// The lift is 0 up to 0.1, 0.731808 at 0.5, (3 − 2·1.65)t² at 1; the
    /// bend is 0 up to 0.4, 0.28 at 0.5 and 0.3328 at 0.6.
    #[test]
    fn the_blade_curves_are_the_originals() {
        assert_eq!(blade_lift(0.0), 0.0);
        assert_eq!(blade_lift(0.09), 0.0);
        assert!(blade_lift(0.1) >= 0.0);
        assert!(close(blade_lift(0.5), 0.731_808));
        assert_eq!(blade_bend(0.39), 0.0);
        assert!(blade_bend(0.4) >= 0.0);
        assert!(close(blade_bend(0.5), 0.28));
        assert!(close(blade_bend(0.6), 0.332_8));
        assert!(close(blade_lift(1.0), 0.066_156_685));
    }

    /// A push against the point's travel this tick (or one it barely moves
    /// along) acts in full, one it moves along partly, and one it outruns
    /// not at all.
    #[test]
    fn the_push_weighting_is_the_originals() {
        let push = Vec3::X;
        assert_eq!(push_fraction(push, Vec3::new(-1.0, 0.0, 0.0)), 1.0);
        assert_eq!(push_fraction(push, Vec3::new(0.0, 0.0, 0.0)), 1.0);
        assert!(close(push_fraction(push, Vec3::new(0.1, 0.0, 0.0)), 0.6));
        assert_eq!(push_fraction(push, Vec3::new(0.3, 0.0, 0.0)), 0.0);
        assert_eq!(push_fraction(Vec3::ZERO, Vec3::X), 0.0);
        // A NaN weight moves nothing, as the original's compare skips it.
        assert_eq!(push_fraction(push, Vec3::new(f32::NAN, 0.0, 0.0)), 0.0);
    }

    /// The handling's guide, worked by hand: a guide up +z from the
    /// frame's offset, a weapon point (3, 4, 5) from it and a hand point
    /// half that, at half the clip with a balance of 0.5. Lift 0.731808,
    /// bend 0.28; the weapon's side push (−3, −4, 0)·2.195424 plus
    /// 0.28·62.4 along the guide, the forearm's (−1.5, −2, 0)·0.182952 plus
    /// 0.28·3.9 along it; none before 0.1, and the frame's translation
    /// moves the guide's origin.
    #[test]
    fn the_handling_pushes_along_the_guide() {
        let frame = Affine3A::IDENTITY;
        let origin = GUIDE_OFFSET;
        let guide = origin + Vec3::Z * 100.0;
        let weapon = origin + Vec3::new(3.0, 4.0, 5.0);
        let hand = origin + Vec3::new(1.5, 2.0, 2.5);
        let [w, f] = handling_pushes(0.5, 0.5, frame, guide, weapon, hand).unwrap();
        assert!(close(blade_lift(0.5), 0.731_808));
        assert!(close(blade_bend(0.5), 0.28));
        assert!((w - Vec3::new(-6.586_272, -8.781_696, 17.472)).length() < 1e-3);
        assert!((f - Vec3::new(-0.274_428, -0.365_904, 1.092)).length() < 1e-3);
        assert!(handling_pushes(0.05, 0.5, frame, guide, weapon, hand).is_none());
        // Moving the frame moves the origin, so the same points push
        // differently.
        let moved = Affine3A::from_translation(Vec3::X);
        let [w2, _] = handling_pushes(0.5, 0.5, moved, guide, weapon, hand).unwrap();
        assert!((w2 - w).length() > 0.1);
    }
}
