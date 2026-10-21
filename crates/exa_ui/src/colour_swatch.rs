//! Opening the colour-swatch popup from a colour button.
//!
//! The popup (`col_swatch`) is an eight-row grid of three-unit cells. The
//! current colour's cell is `(colour >> 3)` columns and `(colour % 8)` rows,
//! so the popup is placed to put that cell under the point it opens at: with
//! the anchor bit `0x800` clear the pointer's position, with it set the
//! button's anchor (its x plus half its width, its y plus its height, taken
//! to screen coordinates). A button of the colour class also switches the
//! popup to the `0x6000` action set; any other class only creates it.
//!
//! Evidence: the colour-swatch opening routine of the object-editor unit
//! (`docs/UI.md`, the colour popup).

/// A cell's size in units.
pub const CELL_UNITS: i32 = 3;
/// The anchor flag.
pub const ANCHOR: u32 = 0x800;
/// The action set the popup switches to.
pub const ACTION_SET: u32 = 0x6000;
/// The colour button's class word.
pub const COLOUR_BUTTON: u16 = 0x17;

/// The popup's open values for a colour button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swatch {
    /// The colour the popup starts on.
    pub current: u8,
    /// The colour value it writes back into.
    pub value: u64,
    /// The button it opened from.
    pub button: u64,
    pub action_set: u32,
}

/// What opening the swatch does for a descriptor of `class`. Any class
/// creates the popup; only the colour-button class fills it in.
pub fn open_swatch(class: u16, colour: u8, value: u64, button: u64) -> Option<Swatch> {
    (class == COLOUR_BUTTON).then_some(Swatch {
        current: colour,
        value,
        button,
        action_set: ACTION_SET,
    })
}

/// The grid's cell centre in the popup: column `colour >> 3`, row
/// `colour % 8`, both offset by half a cell and by the popup's border.
pub fn cell_centre(colour: u8, unit: i32, border: i32) -> (i32, i32) {
    let cell = unit.wrapping_mul(CELL_UNITS);
    let half = ((cell as u32) >> 1) as i32;
    let column = i32::from(colour) >> 3;
    let row = i32::from(colour) % 8;
    (
        column
            .wrapping_mul(cell)
            .wrapping_add(half)
            .wrapping_add(border),
        row.wrapping_mul(cell)
            .wrapping_add(half)
            .wrapping_add(border),
    )
}

/// Where the popup goes: the point it opens at (the pointer when the
/// anchor bit is clear, the button's screen anchor when set) less the
/// current cell's centre.
pub fn swatch_place(
    colour: u8,
    unit: i32,
    border: i32,
    flags: u32,
    pointer: (i32, i32),
    anchor: (i32, i32),
) -> (i32, i32) {
    let (x, y) = cell_centre(colour, unit, border);
    let at = if flags & ANCHOR == 0 { pointer } else { anchor };
    (at.0.wrapping_sub(x), at.1.wrapping_sub(y))
}

/// The button's local anchor: x plus half its width, y plus its height.
pub fn button_anchor(rect: [i16; 4]) -> (i32, i32) {
    (
        i32::from(rect[0]).wrapping_add(i32::from(rect[2]) / 2),
        i32::from(rect[1]).wrapping_add(i32::from(rect[3])),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_a_colour_button_takes_its_colour() {
        let s = open_swatch(COLOUR_BUTTON, 12, 0x1234, 7).unwrap();
        assert_eq!(s.current, 12);
        assert_eq!(s.value, 0x1234);
        assert_eq!(s.button, 7);
        assert_eq!(s.action_set, 0x6000);
        assert!(open_swatch(9, 1, 0, 0).is_none());
    }

    #[test]
    fn the_grid_cell_is_eight_rows_of_three_units() {
        // Unit 4: cell 12, half 6. Colour 12: column 1, row 4.
        assert_eq!(cell_centre(12, 4, 100), (100 + 12 + 6, 100 + 48 + 6));
        assert_eq!(cell_centre(0, 4, 0), (6, 6));
        assert_eq!(cell_centre(7, 4, 0), (6, 90));
        assert_eq!(cell_centre(8, 4, 0), (18, 6));
    }

    #[test]
    fn the_place_anchors_to_the_pointer_or_the_button() {
        let rect = [10, 20, 30, 40];
        let p = swatch_place(12, 4, 0, 0, (640, 480), (0, 0));
        assert_eq!(p, (640 - 18, 480 - 54));
        assert_eq!(button_anchor(rect), (25, 60));
        let p = swatch_place(12, 4, 0, ANCHOR, (640, 480), button_anchor(rect));
        assert_eq!(p, (25 - 18, 60 - 54));
        // The half width truncates toward zero.
        assert_eq!(button_anchor([10, 20, -3, 40]), (9, 60));
    }
}
