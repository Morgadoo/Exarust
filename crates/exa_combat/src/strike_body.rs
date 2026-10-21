//! The body side of the original's swing routine: the aim nudges that turn
//! the blade into the swing (the fist's cursor push, the edge alignment,
//! the overhead's plane and the side swing's vertical homing), the
//! two-hand group's class bit, the tangle break and the clinch clear.
//!
//! The swing clock, the strike window and the velocity trackers are in
//! [`crate::swing`]; the held weapon's own handling forces are in
//! [`crate::weapon`]. The nudges here return the push (cm) at the striker's
//! point; the caller applies it with its weights. They are reproduced but
//! not wired: Exarust holds weapons as bone-attached meshes, so there is no
//! striker body to push yet. Rules and constants: reverse engineering of
//! the swing routine.

use bevy::prelude::*;

/// The unarmed push along the cursor: 10·(1 − 2|clock − 0.5|) cm a tick.
const FIST_PUSH: f32 = 10.0;
/// Edge alignment: a velocity × axis over 1 cm² is scaled to 20 cm; the
/// push share is min(5·(clock − 0.5), 0.5), × clock for a greatsword.
const EDGE_CAP: f32 = 1.0;
const EDGE_SCALE: f32 = 20.0;
const EDGE_SHARE: f32 = 5.0;
const EDGE_MOST: f32 = 0.5;
/// The overhead's pull: gain (0.3 − 3·(0.7 − clock)²)·4/(0.25·|c| + 1).
const OVERHEAD_BASE: f32 = 0.3;
const OVERHEAD_SPREAD: f32 = 3.0;
const OVERHEAD_CENTRE: f32 = 0.7;
const OVERHEAD_GAIN: f32 = 4.0;
const OVERHEAD_REACH: f32 = 0.25;
/// The side swing's homing: gain (0.5 − (2.1 − 2.8·clock)²)·2.
const HOMING_BASE: f32 = 2.1;
const HOMING_SLOPE: f32 = 2.8;
const HOMING_TOP: f32 = 0.5;
const HOMING_SCALE: f32 = 2.0;
/// The crouched target's chest falls 40 cm, 100 cm crouched over 0.5.
const HOMING_LOW: f32 = 40.0;
const HOMING_CROUCHED: f32 = 100.0;
/// The break tangle counter gains 2 for a hand behind the pelvis and starts
/// the motion over 60; the hand is in a box y −5…60, z < 0, |x| < 20.
const TANGLE_ADD: i32 = 2;
const TANGLE_OVER: i32 = 60;
/// The two-hand group's class bit.
pub const TWO_HAND_GROUP: u32 = 0x4000_0000;

/// The fist's aim push: along the unit cursor direction `cursor` (across
/// the ground), 10·(1 − 2|clock − 0.5|) cm a tick.
pub fn fist_push(progress: f32, cursor: Vec3) -> Vec3 {
    cursor * (FIST_PUSH * (1.0 - 2.0 * (progress - 0.5).abs()))
}

/// The edge alignment's push at `progress`: `velocity × axis` (the slot's
/// velocity and the blade's axis), scaled to 20 cm when over 1 cm², times
/// min(5·(clock − 0.5), 0.5), times the clock for a greatsword.
pub fn edge_push(progress: f32, velocity: Vec3, axis: Vec3, greatsword: bool) -> Vec3 {
    let mut c = velocity.cross(axis);
    let len2 = c.length_squared();
    if len2 > EDGE_CAP {
        c *= EDGE_SCALE / len2.sqrt();
    }
    let share = EDGE_SHARE * (progress - 0.5);
    let share = if share > EDGE_MOST { EDGE_MOST } else { share };
    let share = if share < 0.0 { 0.0 } else { share };
    c * (if greatsword { share * progress } else { share })
}

/// The overhead's pull toward the vertical plane through the chest along
/// `heading`, at `progress`: `c` is the tip less the chest across the
/// ground; the pull is `c·gain` where gain = (0.3 − 3(0.7 − p)²)·4 /
/// (0.25|c| + 1), never past `c` (no overshoot).
pub fn overhead_pull(progress: f32, tip: Vec3, chest: Vec3, heading: Vec3) -> Vec3 {
    let delta = (tip - chest).with_y(0.0);
    let across = delta.dot(heading);
    let c = delta - heading * across;
    let len = c.length();
    if len <= 0.0 {
        return Vec3::ZERO;
    }
    let gain = (OVERHEAD_BASE - OVERHEAD_SPREAD * (OVERHEAD_CENTRE - progress).powi(2))
        * OVERHEAD_GAIN
        / (OVERHEAD_REACH * len + 1.0);
    let scale = gain.min(1.0);
    c * scale
}

/// The side swing's vertical homing: the tip is pulled down to the target's
/// chest height when it is above it, and up to chest − 40 cm (100 crouched
/// over 0.5) when it is below, by (0.5 − (2.1 − 2.8p)²)·2, never past the
/// height.
pub fn vertical_homing(
    progress: f32,
    tip_y: f32,
    target_chest_y: f32,
    crouch: f32,
    _greatsword: bool,
) -> f32 {
    let low = target_chest_y
        - if crouch > 0.5 {
            HOMING_CROUCHED
        } else {
            HOMING_LOW
        };
    let delta = if tip_y <= target_chest_y {
        if low <= tip_y { 0.0 } else { low - tip_y }
    } else {
        target_chest_y - tip_y
    };
    if delta == 0.0 {
        return 0.0;
    }
    let gain = HOMING_TOP - (HOMING_BASE - HOMING_SLOPE * progress).powi(2);
    let gain = gain * HOMING_SCALE;
    let step = delta * gain;
    if delta > 0.0 {
        step.min(delta)
    } else {
        step.max(delta)
    }
}

/// The break-tangle counter after one tick: outside a parry, it falls by
/// one (floored at 0), then each held item's origin in the body's frame
/// adds 2 while it is inside the box y −5…60, z < 0, |x| < 20; over 60
/// the motion 0x1C starts and the counter clears.
pub fn break_tangle(counter: i32, parrying: bool, hands: &[Vec3]) -> (i32, bool) {
    if parrying {
        return (counter, false);
    }
    let mut c = (counter - 1).max(0);
    for hand in hands {
        let inside = (-5.0..=60.0).contains(&hand.y) && hand.z < 0.0 && hand.x.abs() < 20.0;
        if inside {
            c += TANGLE_ADD;
            if c > TANGLE_OVER {
                return (0, true);
            }
        }
    }
    (c, false)
}

/// The class masks a two-hand group touches: the first held item, its
/// parent, the second, its parent and the second's grandparent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TwoHandMasks(pub [u32; 5]);

/// The two-hand group bit: with two held items, mode 2 is forced when the
/// weapon class lacks 0x30. Mode 0 sets the bit on both items, their
/// parents and the second's grandparent; mode 1 on the items and parents,
/// clearing it on the second and the grandparent; mode 2/3 clears it on
/// every one.
pub fn two_hand_group(
    held_count: usize,
    mode: i32,
    weapon_class: u32,
    mut masks: TwoHandMasks,
) -> TwoHandMasks {
    if held_count < 2 {
        return masks;
    }
    let mode = if weapon_class & 0x30 == 0 { 2 } else { mode };
    match mode {
        0 => {
            for m in &mut masks.0[..4] {
                *m |= TWO_HAND_GROUP;
            }
            masks.0[4] |= TWO_HAND_GROUP;
        }
        1 => {
            for m in &mut masks.0[..4] {
                *m |= TWO_HAND_GROUP;
            }
            masks.0[3] &= !TWO_HAND_GROUP;
            masks.0[4] &= !TWO_HAND_GROUP;
        }
        _ => {
            for m in &mut masks.0 {
                *m &= !TWO_HAND_GROUP;
            }
        }
    }
    masks
}

/// The clinch clear: the shield's and every held item's grip link is
/// disabled (flag 0x100000), the hands' and forearms' class bit 8 is
/// cleared, the countered motion is reset, and ending a parry (unless the
/// condition keeps the clip still) restarts the stance with blend 0x28.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clinch {
    pub shield_link: u32,
    pub item_links: [u32; 2],
    pub hand_classes: [u32; 4],
    pub countered: i32,
    pub parrying: bool,
    pub stance_blend: Option<u32>,
}

pub fn clear_clinch(mut clinch: Clinch) -> Clinch {
    clinch.shield_link |= 0x10_0000;
    for link in &mut clinch.item_links {
        *link |= 0x10_0000;
    }
    for class in &mut clinch.hand_classes {
        *class &= !8;
    }
    if clinch.countered >= 0 {
        clinch.countered = -1;
    }
    if clinch.parrying {
        clinch.parrying = false;
        clinch.stance_blend = Some(0x28);
    }
    clinch
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    /// The fist pushes 10 cm at both ends and 0 at the middle; the edge
    /// push is capped at 20 and shares 0.5 at the ends; the overhead does
    /// not overshoot; the homing falls to the chest and rises to the low
    /// line.
    #[test]
    fn the_aim_nudges_are_the_originals() {
        assert_eq!(fist_push(0.0, Vec3::Z), Vec3::ZERO);
        assert!(close(fist_push(0.5, Vec3::Z).length(), 10.0));
        let axis = Vec3::Z;
        let ed = edge_push(1.0, Vec3::X * 100.0, axis, false);
        assert!(close(ed.length(), 0.5 * 20.0));
        assert!(close(edge_push(0.4, Vec3::X, axis, false).length(), 0.0));
        let tip = Vec3::new(10.0, 0.0, 0.0);
        let chest = Vec3::ZERO;
        let heading = Vec3::Z;
        let pull = overhead_pull(0.7, tip, chest, heading);
        assert!(close(pull.length(), 10.0 * 0.3 * 4.0 / (0.25 * 10.0 + 1.0)));
        let down = vertical_homing(0.6, 50.0, 20.0, 0.0, false);
        assert!((-30.0..=0.0).contains(&down));
        let up = vertical_homing(0.6, -100.0, 20.0, 0.0, false);
        assert!((0.0..=80.0).contains(&up));
    }

    /// Two hands: mode 0 sets the bit on all five masks, 1 clears the
    /// second's and its grandparent's, 2/3 clear all; a class without 0x30
    /// forces mode 2; one item does nothing.
    #[test]
    fn the_two_hand_group_is_the_originals() {
        let all = TwoHandMasks([0; 5]);
        let set = two_hand_group(2, 0, 0x30, all);
        assert_eq!(set, TwoHandMasks([TWO_HAND_GROUP; 5]));
        let plus = two_hand_group(2, 1, 0x30, set);
        assert_eq!(plus.0[3], 0);
        assert_eq!(plus.0[4], 0);
        let cleared = two_hand_group(2, 2, 0x30, set);
        assert_eq!(cleared, TwoHandMasks([0; 5]));
        assert_eq!(two_hand_group(2, 0, 0, set), TwoHandMasks([0; 5]));
        assert_eq!(two_hand_group(1, 0, 0x30, all), all);
    }

    /// The tangle counts a hand in the box by two, falls by one outside it,
    /// and starts over 60 when not parrying.
    #[test]
    fn the_tangle_break_is_the_originals() {
        let behind = Vec3::new(0.0, 20.0, -10.0);
        assert_eq!(break_tangle(0, false, &[behind]), (2, false));
        assert_eq!(break_tangle(60, false, &[behind]), (0, true));
        assert_eq!(break_tangle(58, false, &[behind]), (59, false));
        assert_eq!(break_tangle(60, false, &[behind]), (0, true));
        assert_eq!(
            break_tangle(10, false, &[Vec3::new(30.0, 20.0, -10.0)]),
            (9, false)
        );
        assert_eq!(break_tangle(60, true, &[behind]), (60, false));
    }

    /// The clinch clear disables the links, clears the hand bits, resets
    /// the motion and restarts the stance.
    #[test]
    fn the_clinch_clear_is_the_originals() {
        let c = clear_clinch(Clinch {
            shield_link: 0,
            item_links: [8, 0],
            hand_classes: [8; 4],
            countered: 3,
            parrying: true,
            stance_blend: None,
        });
        assert_eq!(c.shield_link, 0x10_0000);
        assert_eq!(c.item_links, [0x10_0008, 0x10_0000]);
        assert_eq!(c.hand_classes, [0; 4]);
        assert_eq!(c.countered, -1);
        assert!(!c.parrying);
        assert_eq!(c.stance_blend, Some(0x28));
    }
}
