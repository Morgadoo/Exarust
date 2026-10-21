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

// The original compares floats so that a NaN takes the other branch; the
// negated compares here keep that.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::prelude::*;

/// The original's smallest length² a vector must exceed to count (just
/// under 2⁻²³).
const EPSILON: f32 = f32::from_bits(0x33ff_ffd7);

/// The unarmed push along the cursor: 10·(1 − 2|clock − 0.5|) cm a tick,
/// while the clock is under 0.9.
const FIST_PUSH: f32 = 10.0;
const FIST_UNTIL: f64 = 0.9;
/// Edge alignment: the velocity's part across the weapon's axis is scaled
/// to 20 cm when over 1 cm²; the push share is min(5·(clock − 0.5), 0.5),
/// × clock for a greatsword.
const EDGE_CAP: f32 = 1.0;
const EDGE_SCALE: f32 = 20.0;
const EDGE_SHARE: f32 = 5.0;
const EDGE_MOST: f32 = 0.5;
/// The overhead's pull, from a clock of 0.4: gain (0.3 − 3·(0.7 − clock)²)
/// · 4/(0.25·|c| + 1), in double.
const OVERHEAD_FROM: f64 = 0.4;
const OVERHEAD_BASE: f64 = 0.3;
const OVERHEAD_SPREAD: f64 = 3.0;
const OVERHEAD_CENTRE: f64 = 0.7;
const OVERHEAD_GAIN: f32 = 4.0;
const OVERHEAD_REACH: f32 = 0.25;
/// The side swing's homing, for a clock in 0.51…0.99: gain
/// (0.5 − (2.1 − 2.8·clock)²), ×2 unless a greatsword, in double.
const HOMING_FROM: f64 = 0.51;
const HOMING_UNTIL: f64 = 0.99;
const HOMING_BASE: f64 = 2.1;
const HOMING_SLOPE: f64 = 2.8;
const HOMING_TOP: f64 = 0.5;
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
/// the ground), 10·(1 − 2|clock − 0.5|) cm a tick; none from a clock of
/// 0.9 (the compare is in double and a NaN clock pushes nothing). The
/// caller runs it for an unarmed striker's hand that is striking.
pub fn fist_push(progress: f32, cursor: Vec3) -> Vec3 {
    if !(f64::from(progress) < FIST_UNTIL) {
        return Vec3::ZERO;
    }
    cursor * ((1.0 - 2.0 * (progress - 0.5).abs()) * FIST_PUSH)
}

/// The edge alignment's inputs: the slot's tracked velocity and the
/// weapon's axis, the blade's two push points and its guide point in the
/// world, and the flipped-edge flag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edge {
    pub velocity: Vec3,
    pub axis: Vec3,
    /// The point the push is applied at; the opposite push goes to
    /// `other`.
    pub point: Vec3,
    pub guide: Vec3,
    pub flipped: bool,
    pub greatsword: bool,
}

/// The edge alignment's push `(at point, at the other point)` at
/// `progress`, or `None` when nothing is pushed. The caller runs it past a
/// clock of 0.5 for the primary armed slot, not in a thrust.
///
/// The velocity less its part along the weapon's axis, `c`, must exceed 1
/// cm² and is scaled to 20 cm; the push is `c − (point − guide)` (the
/// guide less the point, less `c`, for a flipped edge) × min(5·(clock −
/// 0.5), 0.5), × the clock for a greatsword; the other point gets the
/// reverse.
pub fn edge_push(progress: f32, edge: &Edge) -> Option<(Vec3, Vec3)> {
    if !(progress > 0.5) {
        return None;
    }
    let (v, u) = (edge.velocity, edge.axis);
    let along = (v.x * u.x + v.z * u.z) + v.y * u.y;
    let mut c = v - u * along;
    let len2 = c.z * c.z + (c.y * c.y + c.x * c.x);
    if !(len2 > EDGE_CAP) {
        return None;
    }
    c *= EDGE_SCALE / len2.sqrt();
    let out = if edge.flipped {
        (edge.guide - edge.point) - c
    } else {
        c - (edge.point - edge.guide)
    };
    let value = (progress - 0.5) * EDGE_SHARE;
    let mut share = if value < EDGE_MOST { value } else { EDGE_MOST };
    if edge.greatsword {
        share *= progress;
    }
    let push = out * share;
    Some((push, -push))
}

/// The overhead's pull toward the vertical plane through the heading and
/// the striker's own chest point `chest`, at `progress` past 0.4. `tip` and `tip_before`
/// are the weapon tip now and a tick ago; the pull is the chest less the tip,
/// less its part along `heading`, flattened to the ground, scaled by
/// (0.3 − 3(0.7 − clock)²)·4/(0.25|c| + 1) in double and, once the tip
/// already moves along it by `moved`, to 1 − moved/|c|² of that (nothing
/// past the pull's own length²).
pub fn overhead_pull(
    progress: f32,
    chest: Vec3,
    tip: Vec3,
    tip_before: Vec3,
    heading: Vec3,
) -> Option<Vec3> {
    if !(f64::from(progress) > OVERHEAD_FROM) {
        return None;
    }
    let delta = chest - tip;
    let along = (delta.x * heading.x + delta.z * heading.z) + delta.y * heading.y;
    let mut c = delta - heading * along;
    c.y = 0.0;
    let len2 = c.z * c.z + (c.y * c.y + c.x * c.x);
    if !(len2 > EPSILON) {
        return None;
    }
    let reach = OVERHEAD_GAIN / (len2.sqrt() * OVERHEAD_REACH + 1.0);
    let off = OVERHEAD_CENTRE - f64::from(progress);
    let gain = ((OVERHEAD_BASE - off * off * OVERHEAD_SPREAD) * f64::from(reach)) as f32;
    c *= gain;
    let len2 = c.z * c.z + (c.y * c.y + c.x * c.x);
    let moved = tip - tip_before;
    let dot = c.z * moved.z + (c.y * moved.y + c.x * moved.x);
    if dot < EPSILON {
        Some(c)
    } else if len2 > dot {
        Some(c * (1.0 - dot / len2))
    } else {
        None
    }
}

/// The side swing's vertical homing, at `progress` in 0.51…0.99: the tip
/// is pulled down to the target's chest height when it is above it, and up
/// to chest − 40 cm (100 crouched over 0.5) when it is below that, by the
/// gap × (0.5 − (2.1 − 2.8p)²) (×2 unless a greatsword), in double. Once
/// the tip already moves that way by `tip_y − tip_y_before` the push is
/// reduced to the part not yet covered (none past the push itself). It is
/// the push's vertical part, cm.
pub fn vertical_homing(
    progress: f32,
    tip_y: f32,
    tip_y_before: f32,
    target_chest_y: f32,
    crouch: f32,
    greatsword: bool,
) -> Option<f32> {
    let p = f64::from(progress);
    if !(p > HOMING_FROM && p < HOMING_UNTIL) {
        return None;
    }
    let mut gap = if target_chest_y < tip_y {
        target_chest_y - tip_y
    } else {
        let drop = if crouch > 0.5 {
            HOMING_CROUCHED
        } else {
            HOMING_LOW
        };
        let low = target_chest_y - drop;
        if low > tip_y { low - tip_y } else { 0.0 }
    };
    if !(gap.abs() > EPSILON) {
        return None;
    }
    let off = HOMING_BASE - p * HOMING_SLOPE;
    let shape = (HOMING_TOP - off * off) as f32;
    gap = if greatsword {
        gap * shape
    } else {
        gap * shape * HOMING_SCALE
    };
    let along = (tip_y - tip_y_before) * gap;
    if along < EPSILON {
        Some(gap)
    } else if along < gap {
        Some((1.0 - along / gap) * gap)
    } else {
        None
    }
}

/// The break-tangle counter after one tick: outside a parry, it falls by
/// one (floored at 0), then each held item's origin in the body's frame
/// adds 2 while it is inside the box y −5…60, z < 0, |x| < 20; over 60
/// the motion 0x1C starts and the counter clears. A NaN height passes the
/// box's y test, as the original's compares do.
pub fn break_tangle(counter: i32, parrying: bool, hands: &[Vec3]) -> (i32, bool) {
    if parrying {
        return (counter, false);
    }
    let mut c = (counter - 1).max(0);
    for hand in hands {
        let inside = !(hand.y < -5.0) && !(hand.y > 60.0) && hand.z < 0.0 && hand.x.abs() < 20.0;
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
/// then clears it on the second item itself and on the grandparent (the
/// second's parent keeps it); mode 2/3 clears it on every one.
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
            masks.0[2] &= !TWO_HAND_GROUP;
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

/// The clinch clear: the shield's and every held item's grip link flags are
/// set to 0x10_0000 (disabled; the word is assigned, not or-ed), the
/// hands' and forearms' class bit 8 is cleared, the countered motion is
/// reset, and ending a parry (unless the condition keeps the clip still)
/// restarts the stance with blend 0x28.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clinch {
    pub shield_link: u32,
    pub item_links: [u32; 2],
    pub hand_classes: [u32; 4],
    pub countered: i32,
    pub parrying: bool,
    /// The condition holds the clip clock still (shocked).
    pub clip_stands_still: bool,
    pub stance_blend: Option<u32>,
}

pub fn clear_clinch(mut clinch: Clinch) -> Clinch {
    clinch.shield_link = 0x10_0000;
    clinch.item_links.fill(0x10_0000);
    for class in &mut clinch.hand_classes {
        *class &= !8;
    }
    if clinch.countered >= 0 {
        clinch.countered = -1;
    }
    if clinch.parrying {
        clinch.parrying = false;
        if !clinch.clip_stands_still {
            clinch.stance_blend = Some(0x28);
        }
    }
    clinch
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    /// The fist pushes 10 cm at the middle and 0 at the ends, and nothing
    /// from a clock of 0.9 (or a NaN one).
    #[test]
    fn the_fist_pushes_along_the_cursor_until_nine_tenths() {
        assert_eq!(fist_push(0.0, Vec3::Z), Vec3::ZERO);
        assert!(close(fist_push(0.5, Vec3::Z).length(), 10.0));
        assert!(close(fist_push(0.7, Vec3::X).x, 6.0));
        assert_eq!(fist_push(0.91, Vec3::Z), Vec3::ZERO);
        assert_eq!(fist_push(f32::NAN, Vec3::Z), Vec3::ZERO);
        // The f32 nearest 0.9 is under the double 0.9: still pushing.
        assert!(close(fist_push(0.9, Vec3::Z).z, 2.0));
    }

    fn edge(velocity: Vec3, flipped: bool, greatsword: bool) -> Edge {
        Edge {
            velocity,
            axis: Vec3::Z,
            point: Vec3::new(0.0, 0.0, 10.0),
            guide: Vec3::new(0.0, 0.0, 30.0),
            flipped,
            greatsword,
        }
    }

    /// The velocity across the axis is capped to 20 cm; the push is that
    /// less the point's offset from the guide, shared 0.5 at most, the
    /// other point getting the reverse; a flipped edge turns the offset
    /// and the cap round.
    #[test]
    fn the_edge_push_is_the_velocity_across_the_axis() {
        // Velocity 100 along x, 5 along the axis: 100 across.
        let v = Vec3::new(100.0, 0.0, 5.0);
        let (a, b) = edge_push(1.0, &edge(v, false, false)).unwrap();
        // c = (20, 0, 0); point − guide = (0, 0, −20); c − that = (20, 0, 20).
        assert!(close(a.x, 10.0) && close(a.z, 10.0), "{a}");
        assert_eq!(b, -a);
        let (a, _) = edge_push(1.0, &edge(v, true, false)).unwrap();
        // guide − point = (0, 0, 20); less c = (−20, 0, 20).
        assert!(close(a.x, -10.0) && close(a.z, 10.0), "{a}");
        // A greatsword scales by the clock too.
        let (g, _) = edge_push(0.8, &edge(v, false, true)).unwrap();
        let (n, _) = edge_push(0.8, &edge(v, false, false)).unwrap();
        assert!(close(g.x, n.x * 0.8));
        // At 0.55 the share is 5·0.05 = 0.25, under the 0.5 cap.
        let (m, _) = edge_push(0.55, &edge(v, false, false)).unwrap();
        assert!(close(m.x, 20.0 * 0.25), "{m}");
    }

    /// No push before the clock passes 0.5, with 1 cm² or less across the
    /// axis, or with a NaN.
    #[test]
    fn the_edge_push_needs_a_clock_and_a_velocity() {
        let v = Vec3::new(100.0, 0.0, 0.0);
        assert!(edge_push(0.5, &edge(v, false, false)).is_none());
        assert!(edge_push(f32::NAN, &edge(v, false, false)).is_none());
        assert!(edge_push(0.9, &edge(Vec3::X, false, false)).is_none());
        assert!(edge_push(0.9, &edge(Vec3::new(0.0, 0.0, 50.0), false, false)).is_none());
        assert!(edge_push(0.9, &edge(Vec3::splat(f32::NAN), false, false)).is_none());
    }

    /// The overhead pulls the tip toward the chest's plane: at a clock of
    /// 0.7 with a chest 10 cm away across the heading, gain
    /// 0.3·4/(0.25·10 + 1) = 0.342857; a tip already moving that way is
    /// pulled less, and one moving faster not at all.
    #[test]
    fn the_overhead_pull_spares_a_tip_that_is_already_moving() {
        let chest = Vec3::new(10.0, 50.0, 0.0);
        let tip = Vec3::new(0.0, 80.0, 0.0);
        let pull = overhead_pull(0.7, chest, tip, tip, Vec3::Z).unwrap();
        assert!(
            close(pull.x, 10.0 * 0.342_857_14) && pull.y == 0.0,
            "{pull}"
        );
        // The tip moved +1 along x since: dot 3.43 of |c|² 11.76.
        let moving = tip - Vec3::X;
        let eased = overhead_pull(0.7, chest, tip, moving, Vec3::Z).unwrap();
        assert!(
            close(eased.x, pull.x * (1.0 - 3.428_571 / 11.755_1)),
            "{eased}"
        );
        // Moved far along it already: nothing.
        assert!(overhead_pull(0.7, chest, tip, tip - Vec3::X * 10.0, Vec3::Z).is_none());
        // Moving away: the whole pull.
        let away = overhead_pull(0.7, chest, tip, tip + Vec3::X, Vec3::Z).unwrap();
        assert_eq!(away, pull);
    }

    /// The chest less the tip, not the tip less the chest, is pulled; the
    /// heading's part is removed; before 0.4 and on top of the plane it
    /// does nothing.
    #[test]
    fn the_overhead_pull_goes_across_the_heading_toward_the_chest() {
        let tip = Vec3::new(0.0, 80.0, 5.0);
        let chest = Vec3::new(-10.0, 50.0, 25.0);
        let pull = overhead_pull(0.7, chest, tip, tip, Vec3::Z).unwrap();
        assert!(pull.x < 0.0 && pull.z == 0.0 && pull.y == 0.0);
        assert!(overhead_pull(0.3, chest, tip, tip, Vec3::Z).is_none());
        assert!(overhead_pull(f32::NAN, chest, tip, tip, Vec3::Z).is_none());
        let on_plane = Vec3::new(0.0, 50.0, 25.0);
        assert!(overhead_pull(0.7, on_plane, tip, tip, Vec3::Z).is_none());
    }

    /// The side swing's homing: a tip 30 cm above the chest falls by the
    /// gap times 2·(0.5 − (2.1 − 2.8p)²); below the crouch line it rises;
    /// between them it does not move; outside 0.51…0.99 nothing.
    #[test]
    fn the_homing_falls_to_the_chest_and_rises_to_the_low_line() {
        // p = 0.75: (2.1 − 2.1)² = 0, gain 0.5·2 = 1.
        let down = vertical_homing(0.75, 130.0, 130.0, 100.0, 0.0, false).unwrap();
        assert!(close(down, -30.0), "{down}");
        // A greatsword does not double the gain.
        let great = vertical_homing(0.75, 130.0, 130.0, 100.0, 0.0, true).unwrap();
        assert!(close(great, -15.0), "{great}");
        // Below chest − 40 = 60: a tip at 40 rises 20; crouched the line is 0.
        let up = vertical_homing(0.75, 40.0, 40.0, 100.0, 0.0, false).unwrap();
        assert!(close(up, 20.0), "{up}");
        assert!(vertical_homing(0.75, 40.0, 40.0, 100.0, 1.0, false).is_none());
        assert!(vertical_homing(0.75, 80.0, 80.0, 100.0, 0.0, false).is_none());
        assert!(vertical_homing(0.5, 130.0, 130.0, 100.0, 0.0, false).is_none());
        assert!(vertical_homing(0.99, 130.0, 130.0, 100.0, 0.0, false).is_none());
        assert!(vertical_homing(f32::NAN, 130.0, 130.0, 100.0, 0.0, false).is_none());
        assert!(vertical_homing(0.75, f32::NAN, 130.0, 100.0, 0.0, false).is_none());
    }

    /// A tip already falling toward the chest is pushed less (1 − v·g/g of
    /// the gap) and one falling faster than the gap not at all.
    #[test]
    fn the_homing_spares_a_tip_that_is_already_falling() {
        // Gap −30, the tip falling 10 cm a tick: the product 300 is not
        // below the gap, so no push.
        assert!(vertical_homing(0.75, 130.0, 140.0, 100.0, 0.0, false).is_none());
        // Rising away: along negative, the whole gap.
        let away = vertical_homing(0.75, 130.0, 120.0, 100.0, 0.0, false).unwrap();
        assert!(close(away, -30.0));
        // A gap of +20 with the tip rising 0.5: the product 10 is under 20,
        // so (1 − 10/20)·20 is pushed.
        let eased = vertical_homing(0.75, 40.0, 39.5, 100.0, 0.0, false).unwrap();
        assert!(close(eased, 10.0), "{eased}");
    }

    /// Mode 0 sets the bit on all five masks; mode 1 sets the first four
    /// and clears the second item and the grandparent (not its parent);
    /// 2/3 clear all; a class without 0x30 forces mode 2; one item does
    /// nothing.
    #[test]
    fn the_two_hand_group_is_the_originals() {
        let all = TwoHandMasks([0; 5]);
        let set = two_hand_group(2, 0, 0x30, all);
        assert_eq!(set, TwoHandMasks([TWO_HAND_GROUP; 5]));
        let plus = two_hand_group(2, 1, 0x30, all);
        assert_eq!(
            plus,
            TwoHandMasks([TWO_HAND_GROUP, TWO_HAND_GROUP, 0, TWO_HAND_GROUP, 0])
        );
        let cleared = two_hand_group(2, 2, 0x30, set);
        assert_eq!(cleared, TwoHandMasks([0; 5]));
        assert_eq!(two_hand_group(2, 3, 0x30, set), TwoHandMasks([0; 5]));
        assert_eq!(two_hand_group(2, 0, 0, set), TwoHandMasks([0; 5]));
        assert_eq!(two_hand_group(1, 0, 0x30, all), all);
    }

    /// The tangle counts a hand in the box by two, falls by one outside it,
    /// and starts over 60 when not parrying.
    #[test]
    fn the_tangle_break_is_the_originals() {
        let behind = Vec3::new(0.0, 20.0, -10.0);
        assert_eq!(break_tangle(0, false, &[behind]), (2, false));
        assert_eq!(break_tangle(58, false, &[behind]), (59, false));
        assert_eq!(break_tangle(59, false, &[behind]), (60, false));
        assert_eq!(break_tangle(60, false, &[behind]), (0, true));
        assert_eq!(
            break_tangle(10, false, &[Vec3::new(30.0, 20.0, -10.0)]),
            (9, false)
        );
        assert_eq!(break_tangle(0, false, &[Vec3::new(30.0, 20.0, -10.0)]).0, 0);
        assert_eq!(break_tangle(60, true, &[behind]), (60, false));
        // The box edges: y −5 and 60 are in, z 0 and |x| 20 are out.
        assert_eq!(break_tangle(0, false, &[Vec3::new(0.0, 60.0, -1.0)]).0, 2);
        assert_eq!(break_tangle(0, false, &[Vec3::new(0.0, -5.0, -1.0)]).0, 2);
        assert_eq!(break_tangle(0, false, &[Vec3::new(0.0, 61.0, -1.0)]).0, 0);
        assert_eq!(break_tangle(0, false, &[Vec3::new(0.0, 20.0, 0.0)]).0, 0);
        assert_eq!(break_tangle(0, false, &[Vec3::new(20.0, 20.0, -1.0)]).0, 0);
        // A NaN height passes the y test.
        assert_eq!(
            break_tangle(0, false, &[Vec3::new(0.0, f32::NAN, -1.0)]).0,
            2
        );
    }

    /// The clinch clear assigns the links, clears the hand bits, resets
    /// the motion and restarts the stance unless the clip stands still.
    #[test]
    fn the_clinch_clear_is_the_originals() {
        let before = Clinch {
            shield_link: 0,
            item_links: [8, 0],
            hand_classes: [8; 4],
            countered: 3,
            parrying: true,
            clip_stands_still: false,
            stance_blend: None,
        };
        let c = clear_clinch(before);
        assert_eq!(c.shield_link, 0x10_0000);
        assert_eq!(c.item_links, [0x10_0000, 0x10_0000]);
        assert_eq!(c.hand_classes, [0; 4]);
        assert_eq!(c.countered, -1);
        assert!(!c.parrying);
        assert_eq!(c.stance_blend, Some(0x28));
        let shocked = clear_clinch(Clinch {
            clip_stands_still: true,
            ..before
        });
        assert!(!shocked.parrying);
        assert_eq!(shocked.stance_blend, None);
        let idle = clear_clinch(Clinch {
            parrying: false,
            countered: -1,
            ..before
        });
        assert_eq!(idle.stance_blend, None);
        assert_eq!(idle.countered, -1);
    }
}
