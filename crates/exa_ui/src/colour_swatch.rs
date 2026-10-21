//! The colour-swatch popup the emblem editor opens on a colour button.
//!
//! The popup is an eight-column grid of three-unit cells. The current
//! colour's cell is `(colour >> 3)` columns and `(colour % 8)` rows. With
//! the anchor bit `0x800` clear the grid is placed from the screen's
//! bottom-right; with it set, from the button's anchor — its x plus half
//! its width and its y plus its height — converted to screen coordinates.
//! The popup switches to the `0x6000` action set and keeps the parent's
//! owner pointer.
//!
//! Evidence: the colour-swatch routine of the object-editor unit
//! (`docs/UI.md`, the colour popup).

/// The grid's columns and cell size in units.
pub const COLUMNS: i32 = 8;
pub const CELL_UNITS: i32 = 3;
/// The anchor flag.
pub const ANCHOR: u32 = 0x800;
/// The action set the popup switches to.
pub const ACTION_SET: u32 = 0x6000;
/// The colour button's class word.
pub const COLOUR_BUTTON: u16 = 0x17;

/// A colour button's descriptor: its class, its current colour and the
/// rectangle the anchor is measured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourButton {
    pub class: u16,
    /// The current palette index.
    pub colour: u8,
    pub flags: u32,
    /// The button's rectangle: x, y, width, height (its anchor is
    /// `x + width/2`, `y + height`).
    pub rect: [i16; 4],
}

/// The popup's open values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swatch {
    /// The colour the popup starts on.
    pub current: u8,
    /// The colour value it writes back into.
    pub value: u64,
    /// The form it belongs to.
    pub parent: u64,
    /// The owner the form keeps.
    pub owner: u64,
    pub action_set: u32,
}

/// What opening the swatch does for a descriptor of the colour-button
/// class. Other classes open the popup with no colour grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwatchOpen {
    pub swatch: Option<Swatch>,
    pub prepare: bool,
    pub action_set: u32,
}

pub fn open_swatch(class: u16, colour: u8, value: u64, parent: u64, owner: u64) -> SwatchOpen {
    if class != COLOUR_BUTTON {
        return SwatchOpen {
            swatch: None,
            prepare: true,
            action_set: ACTION_SET,
        };
    }
    SwatchOpen {
        swatch: Some(Swatch {
            current: colour,
            value,
            parent,
            owner,
            action_set: ACTION_SET,
        }),
        prepare: true,
        action_set: ACTION_SET,
    }
}

/// The grid's cell centre in the local grid, before anchoring: column
/// `colour >> 3`, row `colour % 8`, both offset by half a cell.
pub fn cell_centre(colour: u8, unit: i32, form_x: i32) -> (i32, i32) {
    let cell = unit.wrapping_mul(CELL_UNITS);
    let half = cell >> 1;
    let column = i32::from(colour) >> 3;
    let row = i32::from(colour) % 8;
    (
        column
            .wrapping_mul(cell)
            .wrapping_add(half)
            .wrapping_add(form_x),
        row.wrapping_mul(cell)
            .wrapping_add(half)
            .wrapping_add(form_x),
    )
}

/// Where the popup goes: screen-sized from the bottom-right when the
/// anchor bit is clear, else from the button's screen anchor.
pub fn swatch_place(
    colour: u8,
    unit: i32,
    form_x: i32,
    flags: u32,
    screen: (i32, i32),
    button: ColourButton,
    anchor: (i32, i32),
) -> (i32, i32) {
    let (x, y) = cell_centre(colour, unit, form_x);
    if flags & ANCHOR == 0 {
        (screen.0.wrapping_sub(x), screen.1.wrapping_sub(y))
    } else {
        let _ = button;
        (anchor.0.wrapping_sub(x), anchor.1.wrapping_sub(y))
    }
}

/// The button's local anchor: x plus half its width, y plus its height.
pub fn button_anchor(rect: [i16; 4]) -> (i32, i32) {
    (
        i32::from(rect[0]).wrapping_add(i32::from(rect[2]) / 2),
        i32::from(rect[1]).wrapping_add(i32::from(rect[3])),
    )
}

/// The unit init and the finaliser. Exarust keeps no editable character
/// texture generator or editor tables, so the finaliser's frees have no
/// counterpart; the two hooks are modelled as empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorGlobals {
    pub texture_generator: Option<u64>,
    pub second_global: Option<u64>,
    pub tables: usize,
}

/// The finaliser releases everything.
pub fn finalise(mut globals: EditorGlobals) -> EditorGlobals {
    globals.texture_generator = None;
    globals.second_global = None;
    globals.tables = 0;
    globals
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button() -> ColourButton {
        ColourButton {
            class: COLOUR_BUTTON,
            colour: 12,
            flags: ANCHOR,
            rect: [10, 20, 30, 40],
        }
    }

    #[test]
    fn opening_a_colour_button_takes_its_colour() {
        let o = open_swatch(COLOUR_BUTTON, 12, 0x1234, 7, 8);
        let s = o.swatch.unwrap();
        assert_eq!(s.current, 12);
        assert_eq!(s.value, 0x1234);
        assert_eq!((s.parent, s.owner), (7, 8));
        assert_eq!(o.action_set, 0x6000);
        assert!(open_swatch(9, 1, 0, 0, 0).swatch.is_none());
    }

    #[test]
    fn the_grid_cell_is_eight_columns_of_three_units() {
        // Unit 4: cell 12, half 6. Colour 12: column 1, row 4.
        assert_eq!(cell_centre(12, 4, 100), (100 + 12 + 6, 100 + 48 + 6));
        // The original adds the form's x to both coordinates.
        assert_eq!(cell_centre(0, 4, 0), (6, 6));
        assert_eq!(cell_centre(7, 4, 0), (6, 90));
        assert_eq!(cell_centre(8, 4, 0), (18, 6));
    }

    #[test]
    fn the_place_anchors_to_the_screen_or_the_button() {
        let b = button();
        let p = swatch_place(12, 4, 0, 0, (640, 480), b, (0, 0));
        assert_eq!(p, (640 - 18, 480 - 54));
        assert_eq!(button_anchor(b.rect), (25, 60));
        let p = swatch_place(12, 4, 0, ANCHOR, (640, 480), b, button_anchor(b.rect));
        assert_eq!(p, (25 - 18, 60 - 54));
    }

    #[test]
    fn the_finaliser_clears_every_global() {
        let g = finalise(EditorGlobals {
            texture_generator: Some(1),
            second_global: Some(2),
            tables: 3,
        });
        assert_eq!(
            g,
            EditorGlobals {
                texture_generator: None,
                second_global: None,
                tables: 0
            }
        );
    }
}
