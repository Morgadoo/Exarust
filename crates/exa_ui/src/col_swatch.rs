//! The palette swatch popup (`TGUIColSwatch`): the 256 indexed palette
//! colours as 32 columns of 8 cells of three units each, the pointer's cell
//! and the selection framed, and a pick storing the index, showing it as two
//! hex digits and calling the owner back.
//!
//! The palette itself lives outside this component (the original asks its
//! voxel-GI palette helper), so it is a trait here: [`Palette`] maps an
//! index to its colour. Evidence: the popup's constructor, grid draw, frame
//! draw, hit test, select, apply, command and set-value slots.

/// The swatch block (its constructor's field): its border share in pixels.
pub const BLOCK: i32 = 8;
/// The grid's columns.
pub const COLUMNS: i32 = 32;
/// The grid's rows.
pub const ROWS: i32 = 8;
/// A cell is three units square.
pub const CELL_UNITS: i32 = 3;
/// The popup's style word.
pub const SWATCH_STYLE: u32 = 0x800;
/// The popup's state word bit: no frame.
pub const FRAMELESS: u32 = 0x10;

/// A source of the 256 indexed palette colours.
pub trait Palette {
    /// The colour of an index, red green blue alpha.
    fn colour(&self, index: u8) -> [u8; 4];
}

/// A palette of opaque greys, useful as a stand-in where a caller has no
/// game palette: index `i` is `(i, i, i, 255)`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GreyPalette;

impl Palette for GreyPalette {
    fn colour(&self, index: u8) -> [u8; 4] {
        [index, index, index, 255]
    }
}

/// The popup's geometry: the cell size, its size in pixels, the grid's
/// origin and the selection's frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwatchMetrics {
    pub block: i32,
    pub cell: i32,
    pub width: i32,
    pub height: i32,
    /// The grid's first cell's corner (a pixel in).
    pub origin: (i32, i32),
}

impl SwatchMetrics {
    /// The constructor's size: an eight-pixel border, 32 cells across and 8
    /// down, each three units.
    pub fn new(unit: i32) -> Self {
        let cell = CELL_UNITS * unit;
        Self {
            block: BLOCK,
            cell,
            width: BLOCK * 2 + COLUMNS * cell,
            height: BLOCK * 2 + ROWS * cell,
            origin: (1, 1),
        }
    }

    /// The cell's rectangle (x0, y0, x1, y1), the frame included.
    pub fn cell_rect(&self, index: u8) -> [i32; 4] {
        let column = i32::from(index) >> 3;
        let row = i32::from(index) & 7;
        let (x, y) = (
            self.origin.0 + column * self.cell,
            self.origin.1 + row * self.cell,
        );
        [x, y, x + self.cell, y + self.cell]
    }

    /// The selection's outer frame (white) and its inner frame (black), as
    /// the draw places them: the outer a pixel past the cell on each side,
    /// the inner the cell itself.
    pub fn selection_rects(&self, index: u8) -> ([i32; 4], [i32; 4]) {
        let [x0, y0, x1, y1] = self.cell_rect(index);
        ([x0, y0, x1 + 1, y1 + 1], [x0 + 1, y0 + 1, x1, y1])
    }
}

/// What a pick did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwatchPick {
    /// The index changed; the owner is called with it.
    Changed(u8),
    /// The index was already the one shown.
    Same,
}

/// The popup's form size and styles after its constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwatchForm {
    pub metrics: SwatchMetrics,
    /// The client's left inset (the constructor's 6, then 8).
    pub inset: i32,
    pub style: u32,
    pub state: u32,
}

impl SwatchForm {
    pub fn new(unit: i32) -> Self {
        Self {
            metrics: SwatchMetrics::new(unit),
            inset: BLOCK,
            style: SWATCH_STYLE,
            state: FRAMELESS,
        }
    }
}

/// The palette swatch popup.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourSwatch {
    /// The index shown, 0-based (0–255).
    pub index: u8,
    /// The external value the set-value message applies (0-based).
    pub value: u8,
    /// The cell under the pointer, or 255 for none.
    pub hover: Option<u8>,
    /// The label's two hex digits.
    pub text: String,
    /// The label's colour, from the palette.
    pub label: [u8; 4],
    pub metrics: SwatchMetrics,
    pub style: u32,
    pub state: u32,
    /// Whether a drag is in progress (the original's +0x70 not −1).
    pub dragging: bool,
    pub shown: bool,
}

impl ColourSwatch {
    /// A popup at `unit`: index 0, no hover, the label showing "00".
    pub fn new(unit: i32) -> Self {
        Self {
            index: 0,
            value: 0,
            hover: None,
            text: "00".to_string(),
            label: [0, 0, 0, 255],
            metrics: SwatchMetrics::new(unit),
            style: SWATCH_STYLE,
            state: FRAMELESS,
            dragging: false,
            shown: false,
        }
    }

    /// The cell at a point in the popup's coordinates: `(x − 1) / cell`
    /// held to 0–31 and `(y − 1) / cell` held to 0–7, combined as
    /// `row * 8 + column`. A point outside the grid still clamps, as the
    /// original's division and clamps do.
    pub fn hit(&self, x: i32, y: i32) -> u8 {
        let cell = self.metrics.cell;
        let column = ((x - 1) / cell).clamp(0, COLUMNS - 1);
        let row = ((y - 1) / cell).clamp(0, ROWS - 1);
        (column * 8 + row) as u8
    }

    /// The pointer moves: remembers the hovered cell.
    pub fn move_to(&mut self, x: i32, y: i32) -> u8 {
        let index = self.hit(x, y);
        self.hover = Some(index);
        index
    }

    /// The two hex digits of an index.
    pub fn index_text(index: u8) -> String {
        format!("{index:02X}")
    }

    /// Selects an index: the label's colour comes from the palette, its
    /// byte and text follow, and the owner is called. Selecting the index
    /// already shown does nothing.
    pub fn select(&mut self, index: u8, palette: &impl Palette) -> SwatchPick {
        if index == self.index {
            return SwatchPick::Same;
        }
        self.index = index;
        self.label = palette.colour(index);
        self.text = Self::index_text(index);
        SwatchPick::Changed(index)
    }

    /// The set-value slot: the index takes the caller's value masked to a
    /// byte, the label follows and the owner hears the new index.
    pub fn set_value(&mut self, value: u32, palette: &impl Palette) -> u8 {
        self.index = (value & 0xff) as u8;
        self.label = palette.colour(self.index);
        self.text = Self::index_text(self.index);
        self.index
    }

    /// A press at a point: with no drag in progress the cell under the
    /// pointer is selected (3). A press while dragging is left to the form.
    pub fn press(&mut self, x: i32, y: i32, palette: &impl Palette) -> Option<u8> {
        if self.dragging {
            return None;
        }
        let index = self.hit(x, y);
        self.select(index, palette);
        Some(3)
    }

    /// The apply message: with no button bit, a value different from the
    /// shown index is set first; the popup then closes (the owner's 3).
    pub fn apply(&mut self, buttons: u32, palette: &impl Palette) -> u8 {
        if buttons & 1 == 0 && self.value != self.index {
            self.set_value(u32::from(self.value), palette);
        }
        self.shown = false;
        self.index
    }

    /// A command word: 0x100 closes; 0x200 applies the value and closes;
    /// 0x1000 and 0x2000 move by a column (eight indices) within 0–248;
    /// 0x4000 and 0x8000 move by a row (one index) within the eight cells of
    /// a column. The direction words need a low bit set, as the original's
    /// words do.
    pub fn command(&mut self, word: u32, palette: &impl Palette) -> Option<u8> {
        match word {
            0x100 => {
                self.shown = false;
                None
            }
            0x200 => {
                let index = self.value;
                let picked = self.select(index, palette);
                self.shown = false;
                match picked {
                    SwatchPick::Changed(index) => Some(index),
                    SwatchPick::Same => Some(index),
                }
            }
            _ if word & 3 != 0 => {
                let mut index = i32::from(self.index);
                match word & 0xf000 {
                    0x1000 => {
                        if index > 7 {
                            index -= 8;
                        }
                    }
                    0x2000 => {
                        if index < 0xf8 {
                            index += 8;
                        }
                    }
                    0x4000 => {
                        if index & 7 != 0 {
                            index -= 1;
                        }
                    }
                    0x8000 => {
                        if index & 7 < 7 {
                            index += 1;
                        }
                    }
                    _ => {}
                }
                match self.select(index as u8, palette) {
                    SwatchPick::Changed(index) => Some(index),
                    SwatchPick::Same => None,
                }
            }
            _ => None,
        }
    }

    /// The cells to draw, in the original's order: thirty-two columns, each
    /// eight rows, at the grid's origin.
    pub fn cells(&self) -> Vec<(u8, [i32; 4])> {
        let mut out = Vec::with_capacity(256);
        for column in 0..COLUMNS {
            for row in 0..ROWS {
                let index = (column * 8 + row) as u8;
                out.push((index, self.metrics.cell_rect(index)));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> impl Palette {
        struct P;
        impl Palette for P {
            fn colour(&self, index: u8) -> [u8; 4] {
                [index, 255 - index, index / 2, 255]
            }
        }
        P
    }

    #[test]
    fn the_grid_is_thirty_two_columns_of_eight_cells() {
        let metrics = SwatchMetrics::new(4);
        assert_eq!(
            (metrics.cell, metrics.width, metrics.height),
            (12, 400, 112)
        );
        assert_eq!(metrics.cell_rect(0), [1, 1, 13, 13]);
        assert_eq!(metrics.cell_rect(1), [1, 13, 13, 25]);
        assert_eq!(metrics.cell_rect(7), [1, 85, 13, 97]);
        assert_eq!(metrics.cell_rect(8), [13, 1, 25, 13]);
        assert_eq!(metrics.cell_rect(255), [373, 85, 385, 97]);
        assert_eq!(
            metrics.selection_rects(9),
            ([13, 13, 26, 26], [14, 14, 25, 25])
        );
        let swatch = ColourSwatch::new(4);
        let cells = swatch.cells();
        assert_eq!(cells.len(), 256);
        assert_eq!(cells[0], (0, [1, 1, 13, 13]));
        assert_eq!(cells[255], (255, [373, 85, 385, 97]));
        let form = SwatchForm::new(4);
        assert_eq!((form.inset, form.style, form.state), (8, 0x800, 0x10));
    }

    #[test]
    fn a_point_hits_the_clamped_cell() {
        let swatch = ColourSwatch::new(4);
        // (1, 1) is index 0; a 12-pixel cell.
        assert_eq!(swatch.hit(1, 1), 0);
        assert_eq!(swatch.hit(5, 5), 0);
        assert_eq!(swatch.hit(13, 1), 8);
        assert_eq!(swatch.hit(1, 13), 1);
        assert_eq!(swatch.hit(13, 13), 9);
        // Clamps at both ends.
        assert_eq!(swatch.hit(-5, -5), 0);
        assert_eq!(swatch.hit(9999, 9999), ((31 * 8) + 7) as u8);
        let mut swatch = swatch;
        assert_eq!(swatch.move_to(20, 20), 9);
        assert_eq!(swatch.hover, Some(9));
    }

    #[test]
    fn a_pick_stores_the_index_and_shows_two_hex_digits() {
        let mut swatch = ColourSwatch::new(4);
        assert_eq!(swatch.text, "00");
        assert_eq!(swatch.select(0, &palette()), SwatchPick::Same);
        assert_eq!(swatch.select(0x2a, &palette()), SwatchPick::Changed(0x2a));
        assert_eq!(swatch.text, "2A");
        assert_eq!(swatch.label, [0x2a, 0xd5, 0x15, 255]);
        assert_eq!(
            swatch.press(1 + 12 * 8 + 3, 1 + 12 * 2 + 3, &palette()),
            Some(3)
        );
        assert_eq!(swatch.index, 66);
        assert_eq!(swatch.text, "42");
        assert_eq!(swatch.set_value(0x1ff, &palette()), 0xff);
        assert_eq!(swatch.text, "FF");
        assert_eq!(swatch.label[0], 0xff);
        // A press while dragging is left to the form.
        swatch.dragging = true;
        assert_eq!(swatch.press(1, 1, &palette()), None);
    }

    #[test]
    fn apply_and_the_command_words_move_and_close() {
        let mut swatch = ColourSwatch::new(4);
        swatch.shown = true;
        swatch.value = 5;
        assert_eq!(swatch.apply(0, &palette()), 5);
        assert!(!swatch.shown);
        assert_eq!(swatch.index, 5);
        // A held button does not apply the value.
        let mut swatch = ColourSwatch::new(4);
        swatch.value = 9;
        assert_eq!(swatch.apply(1, &palette()), 0);
        assert_eq!(swatch.index, 0);
        // One column is eight indices, one row one index.
        let mut swatch = ColourSwatch::new(4);
        swatch.index = 9;
        assert_eq!(swatch.command(0x2002, &palette()), Some(17));
        assert_eq!(swatch.command(0x1002, &palette()), Some(9));
        assert_eq!(swatch.command(0x8002, &palette()), Some(10));
        assert_eq!(swatch.command(0x4002, &palette()), Some(9));
        // At the top of a column up does nothing; at the bottom down does.
        swatch.index = 8;
        assert_eq!(swatch.command(0x4002, &palette()), None);
        swatch.index = 7;
        assert_eq!(swatch.command(0x8002, &palette()), None);
        swatch.index = 0;
        assert_eq!(swatch.command(0x1002, &palette()), None);
        swatch.index = 0xf8;
        assert_eq!(swatch.command(0x2002, &palette()), None);
        // 0x200 applies the held value and closes; 0x100 just closes.
        swatch.value = 0x12;
        assert_eq!(swatch.command(0x200, &palette()), Some(0x12));
        assert!(!swatch.shown);
        swatch.shown = true;
        assert_eq!(swatch.command(0x100, &palette()), None);
        assert!(!swatch.shown);
    }
}
