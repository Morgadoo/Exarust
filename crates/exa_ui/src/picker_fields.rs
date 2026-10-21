//! The character creator's picker fields: the base field with its marker,
//! and the physique field that maps a point to the body-shape weights.
//!
//! A picker keeps a normalised point. Its marker is drawn at the point
//! times the field's size less one. The base hit test always misses but
//! reports the field's centre. Keyboard or stick input moves the point by
//! the stick vector in hundredths, clamps it to 0…1 (an unordered value
//! becomes 0) and runs the change callback. The physique field paints a
//! diamond in the field's grey. A point above the middle maps to the
//! first two weights, a point below to the last two; the fourth is the
//! product of the two coordinates.
//!
//! Evidence: the picker base's draw, hit test and input, the physique
//! picker's draw, pick, weight and release routines (`docs/UI.md`).

/// The body shape array's four positions, as float offsets.
pub const SHAPE_STRIDE: usize = 8;
/// The field must have more than this many entries to take a point.
pub const SHAPE_MIN: i32 = 3;
/// The marker's flag bit.
pub const MARKER_FLAG: u32 = 0x40;
/// A change must move a coordinate by at least this much (the original's
/// float epsilon 1.19e-7).
pub const CHANGE_EPSILON: f32 = f32::from_bits(0x3400_0000);

/// One picker field's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Picker {
    /// The normalised point, 0…1 in each axis.
    pub point: [f32; 2],
    /// The marker's flag word.
    pub flags: u32,
    /// The target control's id, or −1 for a free field.
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

/// The point after the clamp the original applies: above one becomes one,
/// below zero (or unordered) becomes zero.
fn clamp01(v: f32) -> f32 {
    if v >= 0.0 {
        if v > 1.0 { 1.0 } else { v }
    } else {
        0.0
    }
}

impl Picker {
    /// The marker's place inside a field `size` pixels across.
    pub fn marker_at(&self, size: (i32, i32)) -> (i32, i32) {
        let x = f64::from(size.0.wrapping_sub(1)) * f64::from(self.point[0]);
        let y = f64::from(size.1.wrapping_sub(1)) * f64::from(self.point[1]);
        (x.round_ties_even() as i32, y.round_ties_even() as i32)
    }

    /// The base hit test's answer: it misses, reporting the centre.
    pub fn base_hit(size: (i32, i32)) -> ((i32, i32), i32) {
        // The half is computed with the original's signed divide.
        ((size.0.wrapping_sub(0) / 2, size.1.wrapping_sub(0) / 2), -1)
    }

    /// The keyboard/stick input. `stick` is the axis pair; a mask with bit
    /// `0x20` picks the alternative pair. Returns whether it changed.
    pub fn input(&mut self, mask: u32, stick: ([f32; 2], [f32; 2])) -> bool {
        if mask == 0x100 {
            self.flags |= MARKER_FLAG;
            return false;
        }
        if mask == 0x200 {
            self.flags &= !MARKER_FLAG;
            return false;
        }
        if mask & 0x30 == 0 {
            return false;
        }
        let axis = if mask & 0x20 == 0 { stick.0 } else { stick.1 };
        self.point[0] = clamp01(self.point[0] + axis[0] * 0.01);
        self.point[1] = clamp01(self.point[1] + axis[1] * 0.01);
        true
    }

    /// The drag pick: maps the local point to 0…1 and only writes and runs
    /// the callback when either coordinate moved by at least the epsilon.
    /// Returns whether it changed.
    pub fn drag(&mut self, local: (i32, i32), size: (i32, i32)) -> bool {
        let mut x = local.0 as f32 / (size.0) as f32;
        x = clamp01(x);
        let mut y = 1.0 - local.1 as f32 / (size.1) as f32;
        y = clamp01(y);
        let dx = (self.point[0] - x).abs();
        let dy = ((self.point[1] - 1.0) + y).abs();
        if CHANGE_EPSILON <= dx || CHANGE_EPSILON <= dy {
            self.point = [x, 1.0 - y];
            return true;
        }
        false
    }
}

/// The physique field's body-shape weights from its point.
///
/// The first coordinate above the middle sets the third weight from the
/// distance past it and the second coordinate's vertical share; otherwise
/// the first two weights split the two variables. The fourth weight is
/// the product.
pub fn body_shape(point: [f32; 2]) -> [f32; 4] {
    let mut x = point[0];
    let y = 1.0 - point[1];
    let (first, second, third);
    if x > 0.5 {
        let f = x * 2.0 - 1.0;
        x = 1.0 - f * 0.5;
        third = (1.0 - y * 0.25) * f;
        first = 0.0;
        second = 0.0;
    } else {
        x *= 2.0;
        first = (1.0 - y) * (1.0 - x);
        second = (1.0 - x) * y;
        third = 0.0;
    }
    [first, third, x * y, second]
}

/// The physique field's diamond: its four vertices in the field's grey.
pub fn physique_diamond(size: (i32, i32)) -> [[f32; 2]; 4] {
    let w = f64::from(size.0) + 0.5;
    let h = f64::from(size.1) + 0.5;
    [
        [(w * 0.5) as f32, 0.0],
        [(w * 0.5) as f32, h as f32],
        [0.0, (h * 0.5) as f32],
        [w as f32, (h * 0.5) as f32],
    ]
}

/// The physique field's diamond colour.
pub const DIAMOND_COLOUR: [u8; 4] = [0xb4, 0xb4, 0xb4, 0x28];

/// What the physique field's release did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysiqueRelease {
    /// The generator's dirty flag.
    pub dirty: bool,
    /// The release picked at the point.
    pub picked: bool,
    /// The handler's answer is the pick's answer with 5 set.
    pub answer: i32,
}

/// The release applies the shape, marks the generator and picks again.
pub fn physique_release() -> PhysiqueRelease {
    PhysiqueRelease {
        dirty: true,
        picked: true,
        answer: 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_marker_follows_the_point() {
        let p = Picker {
            point: [0.5, 0.5],
            ..Default::default()
        };
        assert_eq!(p.marker_at((101, 81)), (50, 40));
        assert_eq!(Picker::base_hit((101, 81)), ((50, 40), -1));
    }

    #[test]
    fn the_input_moves_and_clamps() {
        let mut p = Picker::default();
        assert!(p.input(0x10, ([10.0, -5.0], [0.0, 0.0])));
        assert_eq!(p.point, [0.6, 0.45]);
        p.point = [0.99, 0.01];
        assert!(p.input(0x20, ([0.0, 0.0], [10.0, -10.0])));
        assert_eq!(p.point, [1.0, 0.0]);
        assert!(!p.input(0x100, ([1.0, 1.0], [1.0, 1.0])));
        assert_eq!(p.flags, MARKER_FLAG);
        assert!(!p.input(0x200, ([0.0, 0.0], [0.0, 0.0])));
        assert_eq!(p.flags, 0);
        // An unordered move becomes zero.
        p.point = [f32::NAN, f32::NAN];
        assert!(p.input(0x10, ([0.0, 0.0], [0.0, 0.0])));
        assert_eq!(p.point, [0.0, 0.0]);
    }

    #[test]
    fn the_drag_only_writes_on_a_real_change() {
        let mut p = Picker {
            point: [0.75, 0.75],
            ..Default::default()
        };
        // Local (75, 25) of 100 by 100: x = 0.75, y = 1 − 0.25 = 0.75, so
        // the stored point becomes (0.75, 0.25) — a real move.
        assert!(p.drag((75, 25), (100, 100)));
        assert_eq!(p.point, [0.75, 0.25]);
        assert!(!p.drag((75, 25), (100, 100)));
    }

    #[test]
    fn the_physique_point_maps_to_four_weights() {
        // Left half: x = 0.25 → 0.5, y = 0.25.
        assert_eq!(body_shape([0.25, 0.25]), [0.125, 0.0, 0.375, 0.375]);
        // Right half: x = 0.75 → f = 0.5, x = 0.75, y = 0.25.
        assert_eq!(body_shape([0.75, 0.25]), [0.0, 0.40625, 0.5625, 0.0]);
        let d = physique_diamond((100, 80));
        assert_eq!(d[0], [50.25, 0.0]);
        assert_eq!(d[2], [0.0, 40.25]);
        assert_eq!(DIAMOND_COLOUR, [0xb4, 0xb4, 0xb4, 0x28]);
        assert_eq!(physique_release().answer, 5);
    }
}
