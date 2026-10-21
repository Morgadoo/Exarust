//! The palette swatch popup (`TGUIColSwatch`): the 256 dye palette colours
//! as 32 columns of 8 cells, three units each, with the selection framed;
//! hovering a cell selects it at once, a pick stores the index, shows it on
//! the opener's label (its colour and its index in hex) and calls the owner
//! back, and a cancel restores the index the popup opened with.
//!
//! The cell colours are the palette's lifted ramp as bytes
//! ([`exa_formats::palette::lifted_bytes`]), the same the factory's area
//! colours use. Evidence: the popup's constructor, cell and frame draws,
//! hit test, hover, press, close, command, select and set-value slots, and
//! the engine's hex formatter, read from their assembly.

use exa_formats::palette::lifted_bytes;

/// The popup's border after its constructor (it first writes 6, then 8).
pub const BORDER: i32 = 8;
/// The popup's fixed size: 384 × 96 pixels inside the border on each side.
pub const WIDTH: i32 = BORDER * 2 + 0x180;
pub const HEIGHT: i32 = BORDER * 2 + 0x60;
/// The grid's columns and rows; index = column · 8 + row.
pub const COLUMNS: i32 = 32;
pub const ROWS: i32 = 8;
/// A cell is three units square.
pub const CELL_UNITS: i32 = 3;
/// The grid's offset inside the client.
pub const GRID_ORIGIN: i32 = 1;
/// The popup's form words after its constructor: style bit 0x10, the
/// attribute word and the option word.
pub const SWATCH_STYLE_BITS: u32 = 0x10;
pub const SWATCH_ATTRIBUTES: u32 = 0x42_1060;
pub const SWATCH_OPTIONS: u32 = 0x800;

/// The answer a slot gives when it changed the selection.
pub const CHANGED: u32 = 3;

/// The engine's hex formatter at the swatch's two digits: 0 is `"0"` (the
/// formatter's zero case skips the padding), one digit is padded with a
/// leading 0, upper-case digits.
pub fn index_text(index: u8) -> String {
    if index == 0 {
        "0".to_string()
    } else {
        format!("{index:02X}")
    }
}

/// A cell's colour: the palette's lifted ramp as bytes, alpha 255.
pub fn cell_colour(index: u8) -> [u8; 4] {
    let [r, g, b] = lifted_bytes(index);
    [r, g, b, 255]
}

/// One drawn cell: its index, colour and corners (left, top, right,
/// bottom). Cells are a pixel in from the grid origin on the left and top
/// and end at the next cell's start, so neighbours keep a pixel apart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwatchCell {
    pub index: u8,
    pub colour: [u8; 4],
    pub rect: [f32; 4],
}

/// The cells in the original's order, thirty-two columns of eight rows, at
/// `unit` pixels per unit.
pub fn cells(unit: i32) -> Vec<SwatchCell> {
    let cell = (CELL_UNITS * unit) as f32;
    let origin = GRID_ORIGIN as f32;
    let mut out = Vec::with_capacity(256);
    let mut x = 0.0f32;
    for column in 0..COLUMNS {
        let left = origin + x;
        x += cell;
        let mut y = 0.0f32;
        for row in 0..ROWS {
            let top = origin + y;
            y += cell;
            let index = (column * ROWS + row) as u8;
            out.push(SwatchCell {
                index,
                colour: cell_colour(index),
                rect: [left, top, x, y],
            });
        }
    }
    out
}

/// The selection's two frames (left, top, right, bottom), each in one of
/// the skin's frame colours: the outer from the cell's grid corner to a
/// pixel past its size, the inner a pixel in on the left and top.
pub fn selection_frames(index: i32, unit: i32) -> ([i32; 4], [i32; 4]) {
    let cell = CELL_UNITS * unit;
    let x = (index >> 3) * unit * 3;
    let y = (index % 8) * unit * 3;
    (
        [x, y, x + cell + 1, y + cell + 1],
        [x + 1, y + 1, x + cell, y + cell],
    )
}

/// The cell under a client point: `(x − 1)/cell` held to 0–31 and
/// `(y − 1)/cell` held to 0–7 (truncating divisions).
pub fn cell_at(x: i32, y: i32, unit: i32) -> u8 {
    let cell = CELL_UNITS * unit;
    let column = ((x - GRID_ORIGIN) / cell).clamp(0, COLUMNS - 1);
    let row = ((y - GRID_ORIGIN) / cell).clamp(0, ROWS - 1);
    (column * ROWS + row) as u8
}

/// The label the opener shows the pick on: its colour (alpha byte holds
/// the index) and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct SwatchLabel {
    pub colour: [u8; 4],
    pub text: String,
}

/// Who heard a selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heard {
    /// The popup's owner callback, with the index.
    Owner(i32),
    /// The label's own callback (no owner callback was set).
    Label,
    /// Nobody.
    Nobody,
}

/// The palette swatch popup.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourSwatch {
    /// The selected index.
    pub index: i32,
    /// The index the popup opened with, restored on a cancel.
    pub original: i32,
    /// The opener's label, when there is one.
    pub label: Option<SwatchLabel>,
    /// Whether the owner and the label have callbacks.
    pub owner_callback: bool,
    pub label_callback: bool,
    /// The form's hovered control code (0 for none).
    pub hovered: i32,
    /// Whether the form is closing (its state byte 2): hovering then does
    /// nothing.
    pub closing: bool,
    /// Whether a press is captured (the form's capture is not −1).
    pub captured: bool,
    pub shown: bool,
}

impl ColourSwatch {
    /// A popup: index 0, nothing hovered.
    pub fn new() -> Self {
        Self {
            index: 0,
            original: 0,
            label: None,
            owner_callback: false,
            label_callback: false,
            hovered: 0,
            closing: false,
            captured: false,
            shown: true,
        }
    }

    fn tell(&mut self) -> Heard {
        let index = self.index;
        if let Some(label) = self.label.as_mut() {
            let [r, g, b, _] = cell_colour(index as u8);
            label.colour = [r, g, b, index as u8];
            label.text = index_text(index as u8);
        }
        if self.owner_callback {
            Heard::Owner(self.index)
        } else if self.label.is_some() && self.label_callback {
            Heard::Label
        } else {
            Heard::Nobody
        }
    }

    /// The select slot: a new index is stored, shown on the label and
    /// told; the same index does nothing (`None`).
    pub fn select(&mut self, index: i32) -> Option<Heard> {
        if index == self.index {
            return None;
        }
        self.index = index;
        Some(self.tell())
    }

    /// The set-value slot: the value's low byte becomes the index, shown
    /// and told even when unchanged.
    pub fn set_value(&mut self, value: u32) -> Heard {
        self.index = (value & 0xff) as i32;
        self.tell()
    }

    /// The hit test over the grid: with no control under the point (the
    /// base's −1) the cell's control code, its index plus 1.
    pub fn hit(&self, base_code: i32, x: i32, y: i32, unit: i32) -> i32 {
        if base_code != -1 {
            return base_code;
        }
        i32::from(cell_at(x, y, unit)) + 1
    }

    /// The hover slot with the control code under the pointer: a cell
    /// (code above 0) is selected at once (3 when it changed, else 0) and
    /// becomes the hovered code; off the grid the base answers, with 3 when
    /// a cell was hovered. A closing popup answers 0.
    pub fn hover(&mut self, code: i32, base_answer: u32) -> u32 {
        if self.closing {
            return 0;
        }
        if code > 0 {
            let answer = if self.select(code - 1).is_some() {
                CHANGED
            } else {
                0
            };
            self.hovered = code;
            return answer;
        }
        if self.hovered > 0 {
            base_answer | CHANGED
        } else {
            base_answer
        }
    }

    /// The press slot at a client point: unless a press is captured the
    /// cell under it is selected (3 when it changed, else 0); a captured
    /// press is the form's.
    pub fn press(&mut self, x: i32, y: i32, unit: i32, base_answer: u32) -> u32 {
        if self.captured {
            return base_answer;
        }
        if self.select(i32::from(cell_at(x, y, unit))).is_some() {
            CHANGED
        } else {
            0
        }
    }

    /// The close slot: without the keep bit (bit 0 of the flags) a changed
    /// index goes back to the original; the popup closes and answers 3.
    pub fn close(&mut self, flags: u32) -> u32 {
        if flags & 1 == 0 && self.index != self.original {
            self.set_value(self.original as u32);
        }
        self.shown = false;
        CHANGED
    }

    /// A command word: 0x200 (cancel) restores the original index and
    /// closes; 0x100 closes keeping the pick; a word with a low bit set
    /// steps a column (0x1000 back, 0x2000 on, by eight within 0–255) or a
    /// row (0x4000 up, 0x8000 down, within the column), answering 3 when
    /// the selection changed. Anything else answers 1.
    pub fn command(&mut self, word: u32) -> u32 {
        match word {
            0x200 => {
                if self.index != self.original {
                    self.set_value(self.original as u32);
                }
                self.shown = false;
                1
            }
            0x100 => {
                self.shown = false;
                1
            }
            _ if word & 3 != 0 => {
                let mut index = self.index;
                match word & 0xf000 {
                    0x1000 if index > 7 => index -= 8,
                    0x2000 if index < 0xf8 => index += 8,
                    0x4000 if index & 7 > 0 => index -= 1,
                    0x8000 if index & 7 < 7 => index += 1,
                    _ => {}
                }
                if self.select(index).is_some() {
                    CHANGED
                } else {
                    1
                }
            }
            _ => 1,
        }
    }
}

impl Default for ColourSwatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_is_thirty_two_columns_of_eight_cells() {
        assert_eq!((WIDTH, HEIGHT), (400, 112));
        let cells = cells(4);
        assert_eq!(cells.len(), 256);
        assert_eq!(cells[0].rect, [1.0, 1.0, 12.0, 12.0]);
        assert_eq!(cells[1].rect, [1.0, 13.0, 12.0, 24.0]);
        assert_eq!(cells[8].rect, [13.0, 1.0, 24.0, 12.0]);
        assert_eq!(cells[255].rect, [373.0, 85.0, 384.0, 96.0]);
        assert_eq!(cells[9].colour, cell_colour(9));
        assert_eq!(cell_colour(0), [255, 255, 255, 255]);
        assert_eq!(selection_frames(9, 4), ([12, 12, 25, 25], [13, 13, 24, 24]));
    }

    #[test]
    fn a_point_hits_the_clamped_cell_as_a_control_code() {
        assert_eq!(cell_at(1, 1, 4), 0);
        assert_eq!(cell_at(13, 1, 4), 8);
        assert_eq!(cell_at(1, 13, 4), 1);
        assert_eq!(cell_at(-5, -5, 4), 0);
        assert_eq!(cell_at(9999, 9999, 4), 255);
        let swatch = ColourSwatch::new();
        assert_eq!(swatch.hit(-1, 13, 13, 4), 10);
        assert_eq!(swatch.hit(4, 13, 13, 4), 4);
    }

    #[test]
    fn the_index_text_is_the_engine_hex() {
        assert_eq!(index_text(0), "0");
        assert_eq!(index_text(5), "05");
        assert_eq!(index_text(0x2a), "2A");
        assert_eq!(index_text(0xff), "FF");
    }

    #[test]
    fn hovering_and_pressing_select_and_tell() {
        let mut swatch = ColourSwatch::new();
        swatch.label = Some(SwatchLabel {
            colour: [0; 4],
            text: String::new(),
        });
        swatch.owner_callback = true;
        assert_eq!(swatch.hover(0x2a + 1, 7), CHANGED);
        assert_eq!(swatch.index, 0x2a);
        let label = swatch.label.clone().unwrap();
        assert_eq!(label.text, "2A");
        assert_eq!(label.colour[3], 0x2a);
        assert_eq!(&label.colour[..3], &cell_colour(0x2a)[..3]);
        // The same cell again: no change; off the grid the base and 3.
        assert_eq!(swatch.hover(0x2a + 1, 7), 0);
        assert_eq!(swatch.hover(0, 4), 4 | CHANGED);
        swatch.closing = true;
        assert_eq!(swatch.hover(1, 4), 0);
        swatch.closing = false;
        assert_eq!(swatch.press(1, 1, 4, 9), CHANGED);
        assert_eq!(swatch.press(1, 1, 4, 9), 0);
        swatch.captured = true;
        assert_eq!(swatch.press(13, 13, 4, 9), 9);
        assert_eq!(swatch.set_value(0x1ff), Heard::Owner(0xff));
        swatch.owner_callback = false;
        swatch.label_callback = true;
        assert_eq!(swatch.select(3), Some(Heard::Label));
        assert_eq!(swatch.select(3), None);
    }

    #[test]
    fn cancel_restores_and_the_command_words_step() {
        let mut swatch = ColourSwatch::new();
        swatch.original = 5;
        swatch.index = 9;
        // The keep bit closes with the pick.
        assert_eq!(swatch.close(1), CHANGED);
        assert_eq!((swatch.index, swatch.shown), (9, false));
        assert_eq!(swatch.close(0), CHANGED);
        assert_eq!(swatch.index, 5);
        swatch.index = 9;
        assert_eq!(swatch.command(0x2002), CHANGED);
        assert_eq!(swatch.index, 17);
        assert_eq!(swatch.command(0x1002), CHANGED);
        assert_eq!(swatch.command(0x8002), CHANGED);
        assert_eq!(swatch.command(0x4002), CHANGED);
        assert_eq!(swatch.index, 9);
        swatch.index = 8;
        assert_eq!(swatch.command(0x4002), 1);
        swatch.index = 7;
        assert_eq!(swatch.command(0x8002), 1);
        swatch.index = 0xf8;
        assert_eq!(swatch.command(0x2002), 1);
        assert_eq!(swatch.command(0x2000), 1);
        assert_eq!(swatch.command(0x200), 1);
        assert_eq!((swatch.index, swatch.shown), (5, false));
        swatch.shown = true;
        assert_eq!(swatch.command(0x100), 1);
        assert!(!swatch.shown);
    }
}
