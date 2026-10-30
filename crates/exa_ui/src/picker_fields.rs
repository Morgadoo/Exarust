//! The character creator's picker fields: the base field with its marker,
//! its keys, and the physique field that maps a point to body-shape weights.
//!
//! A picker keeps a normalised point. Its marker is drawn at the point
//! times the field's size less one, rounded to even in single precision. The
//! base hit test always misses but reports the field's centre. A key
//! moves the point by the stick vector in hundredths (the sum is worked in
//! double), holds it to 0…1 (an unordered value stays unordered) and runs the
//! change callback; the key handler always reports it handled. The pointer
//! pick is [`crate::pickers::pick_point`]. The physique field draws a cross
//! of two lines in the field's grey, and a point at or below the middle
//! maps to the first, third and fourth weights, a point above it to the
//! second and third.
//!
//! Evidence: the picker base's marker draw, hit test, key handler and pick,
//! and the physique picker's constructor, draw, weight, press, release and
//! change routines (`docs/UI.md`).

use crate::pickers::{PickPoint, pick_point};

/// The marker's flag bit.
pub const MARKER_FLAG: u32 = 0x40;
/// A body-shape array holds more than this many entries before it takes
/// weights.
pub const SHAPE_MIN: usize = 3;
/// The side of a picker field before the dialog lays it out, in pixels.
pub const FIELD_SIDE: i32 = 160;
/// The cross's colour (red, green, blue, alpha).
pub const CROSS_COLOUR: [u8; 4] = [0xb4, 0xb4, 0xb4, 0x28];
/// The key code that toggles the marker.
pub const KEY_TOGGLE_MARKER: u32 = 0x100;
/// The key code that hides the marker.
pub const KEY_HIDE_MARKER: u32 = 0x200;
/// The key bits that move the point; bit `0x20` picks the second stick.
pub const KEY_MOVE_BITS: u32 = 0x30;
/// What the picker's key handler and the pick answer.
pub const HANDLED: i32 = 1;
/// What a pointer pick answers, whether or not the point moved.
pub const PICK_ANSWER: i32 = 3;

/// One picker field's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Picker {
    /// The normalised point, 0…1 in each axis.
    pub point: [f32; 2],
    /// The marker's flag word.
    pub flags: u32,
    /// The target control's id, or −1 for a free field (the marker then
    /// draws in the free colour, otherwise in the bound colour).
    pub marker: i32,
}

impl Default for Picker {
    fn default() -> Self {
        Self {
            point: [0.5; 2],
            flags: 0,
            marker: -1,
        }
    }
}

/// Which of the skin's two marker colours the marker draws in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerInk {
    /// A free field (marker −1): the skin's free-marker colour with its
    /// alpha byte replaced by 0xff.
    Free,
    /// A bound field: the skin's bound-marker colour as it is.
    Bound,
}

/// The alpha the free marker's colour takes.
pub const FREE_MARKER_ALPHA: u8 = 0xff;

/// What a key did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickerKey {
    /// Always [`HANDLED`].
    pub answer: i32,
    /// Whether the change callback runs (a move key).
    pub callback: bool,
}

/// Below zero becomes zero, above one becomes one; an unordered value
/// passes both tests and stays unordered, as does −0.
fn hold(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

impl Picker {
    /// The marker's place inside a field of client `size`: the size less
    /// one, in 64-bit integers, times the point in single precision, then
    /// rounded to even.
    pub fn marker_at(&self, size: (i32, i32)) -> (i32, i32) {
        let at = |extent: i32, p: f32| {
            let span = (i64::from(extent) - 1) as f32;
            (span * p).round_ties_even() as i64 as i32
        };
        (at(size.0, self.point[0]), at(size.1, self.point[1]))
    }

    /// The marker's colour: free for marker −1, bound otherwise. The
    /// draw then hands the skin's marker routine [`Self::marker_at`].
    pub fn marker_ink(&self) -> MarkerInk {
        if self.marker == -1 {
            MarkerInk::Free
        } else {
            MarkerInk::Bound
        }
    }

    /// The base hit test's answer: it misses (−1), reporting the centre with
    /// the signed halving toward zero.
    pub fn base_hit(size: (i32, i32)) -> ((i32, i32), i32) {
        ((size.0 / 2, size.1 / 2), -1)
    }

    /// The key handler. The toggle code flips the marker's bit and the hide
    /// code clears it; a code with a move bit adds hundredths of the first
    /// stick (or the second, with bit `0x20`) to the point and runs the
    /// callback. Every code answers [`HANDLED`].
    pub fn input(&mut self, code: u32, sticks: ([f32; 2], [f32; 2])) -> PickerKey {
        let mut callback = false;
        if code == KEY_TOGGLE_MARKER {
            self.flags ^= MARKER_FLAG;
        } else if code == KEY_HIDE_MARKER {
            self.flags &= !MARKER_FLAG;
        } else if code & KEY_MOVE_BITS != 0 {
            let stick = if code & 0x20 != 0 { sticks.1 } else { sticks.0 };
            for (point, step) in self.point.iter_mut().zip(stick) {
                // The product and the sum are worked in double.
                let moved = f64::from(step) * 0.01 + f64::from(*point);
                *point = hold(moved as f32);
            }
            callback = true;
        }
        PickerKey {
            answer: HANDLED,
            callback,
        }
    }

    /// The pointer pick at `local` client coordinates; answers whether the
    /// point moved (and so the callback runs). The pick itself answers
    /// [`PICK_ANSWER`] either way.
    pub fn drag(&mut self, local: (i32, i32), size: (i32, i32)) -> bool {
        let mut p = PickPoint {
            u: self.point[0],
            v: self.point[1],
        };
        let moved = pick_point(&mut p, local.0, local.1, size);
        self.point = [p.u, p.v];
        moved
    }
}

/// The physique picker's constructor (`TPhysiquePicker`): it runs the tone
/// picker's constructor (so the same 160 × 160 field, form words and middle
/// point, [`crate::pickers::TonePicker::new`]), then takes the character
/// generator's body-shape array as the array its changes write
/// ([`apply_body_shape`]).
#[derive(Debug, Clone, PartialEq)]
pub struct PhysiquePicker {
    pub base: crate::pickers::TonePicker,
    /// The field's marker and keys.
    pub field: Picker,
    /// The picker writes the generator's body-shape array.
    pub writes_generator_shape: bool,
}

impl PhysiquePicker {
    pub fn new() -> Self {
        let base = crate::pickers::TonePicker::new();
        Self {
            field: Picker {
                point: [base.point.u, base.point.v],
                ..Picker::default()
            },
            base,
            writes_generator_shape: true,
        }
    }
}

impl Default for PhysiquePicker {
    fn default() -> Self {
        Self::new()
    }
}

/// Writes the physique point's four weights into the body-shape array, one
/// per entry's first float, when it holds more than [`SHAPE_MIN`] entries.
/// Answers whether it wrote.
pub fn apply_body_shape(point: [f32; 2], shape: &mut [[f32; 8]]) -> bool {
    if shape.len() <= SHAPE_MIN {
        return false;
    }
    let weights = body_shape(point);
    for (entry, w) in shape.iter_mut().zip(weights) {
        entry[0] = w;
    }
    true
}

/// The physique field's four weights from its point, in array order.
///
/// With `u` the first coordinate and `v = 1 − y`: a `u` above the middle
/// takes the right half (`f = 2u − 1`: the second weight `(1 − v/4)·f`, the
/// third `(1 − f/2)·v`, the others 0); otherwise (`s = 2u`) the first is
/// `(1 − v)(1 − s)`, the third `s·v`, the fourth `(1 − s)·v` and the second
/// 0. The compare is ordered, so a not-a-number `u` takes the left half.
pub fn body_shape(point: [f32; 2]) -> [f32; 4] {
    let u = point[0];
    let v = 1.0 - point[1];
    if u > 0.5 {
        let f = u * 2.0 - 1.0;
        let third = (1.0 - f * 0.5) * v;
        let second = (1.0 - v * 0.25) * f;
        [0.0, second, third, 0.0]
    } else {
        let s = u * 2.0;
        let third = s * v;
        let first = (1.0 - v) * (1.0 - s);
        let fourth = (1.0 - s) * v;
        [first, 0.0, third, fourth]
    }
}

/// The physique field's cross: two lines through the middle of a field of
/// client `size`, worked in single precision with half a pixel added.
/// Returns `[from, to]` for the vertical line then the horizontal one.
pub fn physique_cross(size: (i32, i32)) -> [[[f32; 2]; 2]; 2] {
    let w = (i64::from(size.0) as f32) + 0.5;
    let h = (i64::from(size.1) as f32) + 0.5;
    [
        [[w * 0.5, 0.0], [w * 0.5, h]],
        [[0.0, h * 0.5], [w, h * 0.5]],
    ]
}

/// What a press on the physique field did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysiquePress {
    /// The character generator re-rolls from its owner's data and is marked
    /// dirty.
    pub generator_dirty: bool,
    /// The press picks at the event's point.
    pub picked: bool,
    /// The answer to the event.
    pub answer: i32,
}

/// The press: a free field (marker −1) hands its owner to the generator,
/// marks it dirty, picks at the point and answers the pick's answer with
/// bits 1 and 4 set (7); a bound field answers the base's answer.
pub fn physique_press(marker: i32, has_owner: bool, base_answer: i32) -> PhysiquePress {
    if marker != -1 {
        return PhysiquePress {
            generator_dirty: false,
            picked: false,
            answer: base_answer,
        };
    }
    PhysiquePress {
        generator_dirty: has_owner,
        picked: true,
        answer: PICK_ANSWER | 5,
    }
}

/// What a release on the physique field did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysiqueRelease {
    /// The generator's dirty flag is cleared (the field has an owner).
    pub generator_clean: bool,
    /// The change callback runs (no owner, a callback).
    pub callback: bool,
}

/// The release: with an owner it clears the generator's dirty flag,
/// otherwise it runs the change callback if there is one.
pub fn physique_release(has_owner: bool, has_callback: bool) -> PhysiqueRelease {
    PhysiqueRelease {
        generator_clean: has_owner,
        callback: !has_owner && has_callback,
    }
}

/// What the physique field's change handler did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysiqueChange {
    /// The generator is marked dirty (a callback and an owner).
    pub generator_dirty: bool,
    /// The callback runs.
    pub callback: bool,
}

/// The change handler writes the weights (see [`apply_body_shape`]) and then,
/// with a callback, marks the generator dirty if there is an owner and runs
/// it.
pub fn physique_change(has_owner: bool, has_callback: bool) -> PhysiqueChange {
    PhysiqueChange {
        generator_dirty: has_callback && has_owner,
        callback: has_callback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_not_a_number_point_takes_the_left_half() {
        // 0.5 < NaN is false in the original's ordered compare, so the
        // left branch's weights are taken (with `s = NaN`).
        let weights = body_shape([f32::NAN, 0.5]);
        assert!(weights[0].is_nan() && weights[2].is_nan() && weights[3].is_nan());
        assert_eq!(weights[1], 0.0);
        // The right half's second weight would be NaN too, so pin the
        // branch by a finite u just above the middle.
        let right = body_shape([0.75, 0.5]);
        assert_eq!(right[0], 0.0);
        assert!(right[1] > 0.0);
    }

    #[test]
    fn the_marker_follows_the_point() {
        let p = Picker::default();
        // (101 − 1) · 0.5 and (81 − 1) · 0.5.
        assert_eq!(p.marker_at((101, 81)), (50, 40));
        // Halves round to even: 4 · 0.5 = 2, 5 · 0.5 = 2.5 → 2, 7 · 0.5 = 3.5 → 4.
        assert_eq!(p.marker_at((5, 6)), (2, 2));
        assert_eq!(p.marker_at((8, 9)), (4, 4));
        // The centre truncates toward zero, also below zero.
        assert_eq!(Picker::base_hit((101, 81)), ((50, 40), -1));
        assert_eq!(Picker::base_hit((-3, 0)), ((-1, 0), -1));
    }

    #[test]
    fn the_marker_ink_and_the_physique_constructor() {
        assert_eq!(Picker::default().marker_ink(), MarkerInk::Free);
        let bound = Picker {
            marker: 3,
            ..Picker::default()
        };
        assert_eq!(bound.marker_ink(), MarkerInk::Bound);
        assert_eq!(FREE_MARKER_ALPHA, 0xff);
        let p = PhysiquePicker::new();
        assert_eq!(p.base.size, (160, 160));
        assert_eq!(p.field.point, [0.5, 0.5]);
        assert!(p.writes_generator_shape);
    }

    #[test]
    fn the_keys_toggle_move_and_hold() {
        let mut p = Picker::default();
        // Toggle flips, hide clears, every code answers handled.
        let k = p.input(KEY_TOGGLE_MARKER, ([0.0; 2], [0.0; 2]));
        assert_eq!(
            (k.answer, k.callback, p.flags),
            (HANDLED, false, MARKER_FLAG)
        );
        p.input(KEY_TOGGLE_MARKER, ([0.0; 2], [0.0; 2]));
        assert_eq!(p.flags, 0);
        p.flags = MARKER_FLAG;
        let k = p.input(KEY_HIDE_MARKER, ([0.0; 2], [0.0; 2]));
        assert_eq!((k.answer, p.flags), (HANDLED, 0));
        // An unrelated code is handled and does nothing.
        let k = p.input(0x4, ([9.0; 2], [9.0; 2]));
        assert_eq!((k.answer, k.callback, p.point), (HANDLED, false, [0.5; 2]));
        // The first stick moves by hundredths, worked in double: 0.5 + 0.1.
        let k = p.input(0x10, ([10.0, -5.0], [0.0, 0.0]));
        assert!(k.callback);
        assert_eq!(p.point[0], (0.5f64 + 10.0 * 0.01) as f32);
        assert_eq!(p.point[1], (0.5f64 + -5.0 * 0.01) as f32);
        // Bit 0x20 reads the second stick, and the result is held to 0…1.
        p.point = [0.99, 0.01];
        p.input(0x20, ([0.0, 0.0], [10.0, -10.0]));
        assert_eq!(p.point, [1.0, 0.0]);
        // An unordered point stays unordered; the callback still runs.
        p.point = [f32::NAN, 0.5];
        let k = p.input(0x10, ([1.0, 1.0], [0.0, 0.0]));
        assert!(k.callback && p.point[0].is_nan());
    }

    #[test]
    fn the_pick_matches_the_shared_base_pick() {
        let mut p = Picker {
            point: [0.75, 0.75],
            ..Default::default()
        };
        // Local (75, 25) of 100 by 100: u = 0.75, 1 − 0.25 = 0.75, so the
        // stored point becomes (0.75, 0.25): a real move.
        assert!(p.drag((75, 25), (100, 100)));
        assert_eq!(p.point, [0.75, 0.25]);
        assert!(!p.drag((75, 25), (100, 100)));
    }

    #[test]
    fn the_physique_point_maps_to_four_weights() {
        // Left half: s = 0.5, v = 0.75.
        assert_eq!(body_shape([0.25, 0.25]), [0.125, 0.0, 0.375, 0.375]);
        // Right half: f = 0.5, v = 0.75.
        assert_eq!(body_shape([0.75, 0.25]), [0.0, 0.40625, 0.5625, 0.0]);
        // The middle gives the same weights either way; an unordered point
        // takes the left half (the ordered compare is false).
        assert_eq!(body_shape([0.5, 0.0]), [0.0, 0.0, 1.0, 0.0]);
        let nan = body_shape([f32::NAN, 0.0]);
        assert!(nan[0].is_nan() && nan[2].is_nan() && nan[3].is_nan());
        assert_eq!(nan[1], 0.0);
    }

    #[test]
    fn the_weights_need_four_entries() {
        let mut few = [[9.0; 8]; 3];
        assert!(!apply_body_shape([0.25, 0.25], &mut few));
        assert_eq!(few[0][0], 9.0);
        let mut shape = [[9.0; 8]; 5];
        assert!(apply_body_shape([0.25, 0.25], &mut shape));
        assert_eq!(
            shape.map(|e| e[0]),
            [0.125, 0.0, 0.375, 0.375, 9.0],
            "four weights; the fifth entry is untouched"
        );
        assert_eq!(shape[0][1], 9.0, "only the first float of an entry");
    }

    #[test]
    fn the_cross_is_two_lines_through_the_middle() {
        let c = physique_cross((100, 80));
        assert_eq!(c[0], [[50.25, 0.0], [50.25, 80.5]]);
        assert_eq!(c[1], [[0.0, 40.25], [100.5, 40.25]]);
        assert_eq!(CROSS_COLOUR, [0xb4, 0xb4, 0xb4, 0x28]);
    }

    #[test]
    fn the_press_release_and_change_follow_the_owner() {
        // A free field with an owner picks and answers 3 | 5 = 7.
        let p = physique_press(-1, true, 0x20);
        assert_eq!((p.generator_dirty, p.picked, p.answer), (true, true, 7));
        // A bound field answers the base's answer and does nothing else.
        let p = physique_press(4, true, 0x20);
        assert_eq!(
            (p.generator_dirty, p.picked, p.answer),
            (false, false, 0x20)
        );
        let r = physique_release(true, true);
        assert!(r.generator_clean && !r.callback);
        let r = physique_release(false, true);
        assert!(!r.generator_clean && r.callback);
        assert!(!physique_release(false, false).callback);
        let c = physique_change(true, true);
        assert!(c.generator_dirty && c.callback);
        let c = physique_change(true, false);
        assert!(!c.generator_dirty && !c.callback);
        assert_eq!(FIELD_SIDE, 160);
    }
}
