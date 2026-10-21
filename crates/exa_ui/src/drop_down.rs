//! The popup list of a combo box (`TDropDownList`): rows six units tall,
//! cut at sixteen with a scrollbar, the pointer's row thrown in, a click
//! writing the choice back to the owner and closing the list.
//!
//! The layout, draw, hit test, press, release, select and command rules are
//! the original's; the owner is a trait so the list stays free of the form
//! base class. Evidence: the list's constructor, layout, draw, close,
//! press, release, hit test, select and command slots.

use crate::gui_font::GuiFont;

/// A row's height in units.
pub const ROW_UNITS: i32 = 6;
/// The most rows shown before the list scrolls.
pub const MAX_ROWS: i32 = 16;
/// The list's state word after its constructor: no frame, a page and no
/// scroll of its own (the popup form is drawn flat).
pub const LIST_STATE: u32 = 0x98;
/// The popup's style word: no title, no frame.
pub const LIST_STYLE: u32 = 3;
/// The scrollbar state bit the list gains when it is cut.
pub const SCROLL_BIT: u32 = 0x200;

/// The list's geometry: its height, whether it scrolls, the content's
/// height and the scroll step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropLayout {
    pub height: i32,
    pub scroll: bool,
    /// The whole list's height, the height when it does not scroll.
    pub content: i32,
    pub step: f32,
    /// The row's height.
    pub row: i32,
}

/// Lays a list of `count` rows out: a row is six units; up to sixteen rows
/// make the list that tall plus twice the border. Past sixteen the list is
/// sixteen rows tall, scrolls, and its scroll step is one over the rows
/// hidden.
pub fn layout(count: i32, unit: i32, border: i32) -> DropLayout {
    let row = ROW_UNITS * unit;
    let content = row * count.max(0);
    if count < MAX_ROWS + 1 {
        DropLayout {
            height: content + border * 2,
            scroll: false,
            content: content + border * 2,
            step: 0.0,
            row,
        }
    } else {
        DropLayout {
            height: row * MAX_ROWS + border * 2,
            scroll: true,
            content,
            step: if count > MAX_ROWS {
                (1.0 / f64::from(count - MAX_ROWS)) as f32
            } else {
                0.0
            },
            row,
        }
    }
}

/// What a press did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPress {
    /// The row under the pointer became the pending choice.
    Pending(i32),
    /// Nothing was under the pointer: close the list.
    Close,
    /// The press is not the list's.
    Ignored,
    /// The press became a held drag.
    Hold(i32),
}

/// What a release did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropRelease {
    /// The pending row was chosen: tell the owner and close.
    Chosen(i32),
    /// No pending choice: the release goes on to the parent form.
    Forward,
}

/// What a command word did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropCommand {
    /// A row was chosen (the command word 0x100): the list closes.
    Choose(i32),
    /// The list closes (word 0x200).
    Close,
    /// The selection moved; the list stays open.
    Moved(i32),
    /// The word did nothing.
    None,
}

/// The owner of a list, as the combo box is: the choice written back and
/// the close notification.
pub trait DropDownOwner {
    fn row_chosen(&mut self, row: i32);
    fn drop_closed(&mut self);
}

/// A row's draw: its 1-based row and its band (the original's selection
/// band rectangle x0, y0, x1, y1), if it is the pointer's or the
/// selection's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowDraw {
    pub row: i32,
    /// The band drawn for the row, if any.
    pub band: Option<[i32; 4]>,
    /// The text's pen (x, baseline y).
    pub pen: (i32, i32),
}

/// The popup list.
#[derive(Debug, Clone, PartialEq)]
pub struct DropDownList {
    pub items: Vec<Vec<u8>>,
    /// The selected item, 1-based; 0 for none.
    pub selected: i32,
    /// The item under the pointer, 1-based; 0 for none.
    pub hover: i32,
    /// The row a press armed; −1 when none.
    pub pending: i32,
    pub width: i32,
    /// The border the list is inset by (twice the GUI pad).
    pub border: i32,
    pub unit: i32,
    /// Where the list sits, in its parent's coordinates.
    pub left: i32,
    pub top: i32,
    /// The state word (starts at [`LIST_STATE`], gains [`SCROLL_BIT`]).
    pub state: u32,
    pub style: u32,
    /// The open scroll's fraction.
    pub scroll: f32,
    pub shown: bool,
}

impl DropDownList {
    /// A closed list of `items` at `left`, `top`, `width` units wide.
    pub fn new(items: Vec<Vec<u8>>, width: i32, unit: i32, border: i32) -> Self {
        Self {
            items,
            selected: 0,
            hover: 0,
            pending: 0,
            width,
            border,
            unit,
            left: 0,
            top: 0,
            state: LIST_STATE,
            style: LIST_STYLE,
            scroll: 0.0,
            shown: false,
        }
    }

    /// The number of rows.
    pub fn count(&self) -> i32 {
        self.items.len() as i32
    }

    /// The current geometry.
    pub fn layout(&self) -> DropLayout {
        layout(self.count(), self.unit, self.border)
    }

    /// Opens the list at `left`, `top` (the combo box's placement): the
    /// height follows the rows, the scroll bit is set past sixteen, and
    /// the scroll step follows the hidden rows.
    pub fn open(&mut self, left: i32, top: i32) -> DropLayout {
        self.left = left;
        self.top = top;
        let layout = self.layout();
        if layout.scroll {
            self.state |= SCROLL_BIT;
        } else {
            self.state &= !SCROLL_BIT;
        }
        self.scroll = 0.0;
        self.pending = 0;
        self.shown = true;
        layout
    }

    /// Where the row starts, in the list's coordinates.
    fn row_start(&self, row: i32, layout: DropLayout) -> i32 {
        self.border + (row - 1) * layout.row
    }

    /// The row at a point in the list's coordinates: `(y − border) / row +
    /// 1`, clamped to 1 and to the row count. A list without rows has no
    /// row.
    pub fn row_at(&self, x: i32, y: i32) -> i32 {
        if self.count() < 1 {
            return 0;
        }
        if x < 0 || x > self.width || y < self.border {
            return 0;
        }
        let row = (i64::from(y) - i64::from(self.border)) / i64::from(ROW_UNITS * self.unit) + 1;
        row.clamp(1, i64::from(self.count())) as i32
    }

    /// The pointer's row: the hover highlight. Returns the row.
    pub fn move_to(&mut self, x: i32, y: i32) -> i32 {
        self.hover = if self.row_at(x, y) > 0 {
            self.row_at(x, y)
        } else {
            0
        };
        self.hover
    }

    /// The select slot: a row 1 or more becomes the selection and the
    /// pointer's row; a row under 1 goes to the parent (and, when the
    /// list already had a selection, asks for a redraw). The original
    /// returns 3 on a selection and the parent's word otherwise.
    pub fn select(&mut self, row: i32) -> Option<i32> {
        if row < 1 {
            return None;
        }
        self.selected = row;
        self.hover = row;
        Some(3)
    }

    /// A press word: a left press over a row arms it (the original's 5);
    /// with no row under the pointer the list closes (3).
    pub fn press(&mut self, x: i32, y: i32, buttons: u32) -> DropPress {
        let row = self.row_at(x, y);
        if row < 1 || buttons & 1 == 0 {
            return DropPress::Close;
        }
        self.pending = row;
        if buttons & 0x4000 != 0 {
            DropPress::Hold(row)
        } else {
            DropPress::Pending(row)
        }
    }

    /// A release: an armed row is the choice, written back to the owner
    /// (the original's 0x200 message with the row) and the list closes.
    pub fn release(&mut self, owner: &mut impl DropDownOwner) -> DropRelease {
        if self.pending < 1 {
            return DropRelease::Forward;
        }
        let row = self.pending;
        self.pending = 0;
        owner.row_chosen(row);
        owner.drop_closed();
        self.shown = false;
        DropRelease::Chosen(row)
    }

    /// A command word: 0x100 chooses the pointer's row and closes; 0x200
    /// closes; a direction moves the selection by one row (up 0x4000, down
    /// 0x8000) held to 1 and the last row.
    pub fn command(&mut self, word: u32, owner: &mut impl DropDownOwner) -> DropCommand {
        match word {
            0x100 => {
                if self.hover > 0 {
                    let row = self.hover;
                    owner.row_chosen(row);
                }
                owner.drop_closed();
                self.shown = false;
                DropCommand::Choose(self.hover)
            }
            0x200 => {
                owner.drop_closed();
                self.shown = false;
                DropCommand::Close
            }
            _ if word & 3 != 0 => {
                let mut row = self.hover;
                match word & 0xf000 {
                    0x4000 => row -= 1,
                    0x8000 => row += 1,
                    _ => {}
                }
                row = row.clamp(1, self.count().max(1));
                if row != self.hover {
                    self.hover = row;
                    self.selected = row;
                    DropCommand::Moved(row)
                } else {
                    DropCommand::None
                }
            }
            _ => DropCommand::None,
        }
    }

    /// The close slot: the list and its item strings are freed and the
    /// owner hears the close.
    pub fn close(&mut self, owner: &mut impl DropDownOwner) {
        owner.drop_closed();
        self.items.clear();
        self.pending = 0;
        self.shown = false;
    }

    /// The focus-without-control slot: a pending choice does not let the
    /// focus go (1); otherwise the parent handles it.
    pub fn focus_without_control(&self) -> Option<u32> {
        (self.pending >= 0).then_some(1)
    }

    /// The rows to draw, top to bottom: the selection band (drawn wider
    /// when the pointer's row is also the selection) and the text's pen.
    /// The band runs from x 1 to the client's right edge less one and from
    /// the row's top to its bottom less two; the text sits a unit and a
    /// pixel in, at `(row height + font height + 1) / 2 + 1` from the row's
    /// top, or centred when the style's 0x7000 bits are 0x3000.
    pub fn rows(&self, font: &GuiFont) -> Vec<RowDraw> {
        let layout = self.layout();
        let mut out = Vec::new();
        for row in 1..=self.count() {
            let y = self.row_start(row, layout);
            let band = if row == self.hover || row == self.selected {
                Some([1, y, self.width - 1, y + layout.row - 2])
            } else {
                None
            };
            let pen_x = if self.style & 0x7000 == 0x3000 {
                let width = font.text_width(&self.items[row as usize - 1]);
                ((i64::from(self.width) - i64::from(width)) >> 1) as i32 + self.unit
            } else {
                self.unit + 1
            };
            let pen_y = ((layout.row + font.ascent + 1) as u32 >> 1) as i32 + y + 1;
            out.push(RowDraw {
                row,
                band,
                pen: (pen_x, pen_y),
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::RAYFORM_10;

    struct Owner {
        chosen: Vec<i32>,
        closed: u32,
    }

    impl DropDownOwner for Owner {
        fn row_chosen(&mut self, row: i32) {
            self.chosen.push(row);
        }
        fn drop_closed(&mut self) {
            self.closed += 1;
        }
    }

    fn list(n: usize) -> DropDownList {
        DropDownList::new(
            (1..=n).map(|i| format!("Row {i}").into_bytes()).collect(),
            80,
            4,
            8,
        )
    }

    #[test]
    fn rows_are_six_units_and_sixteen_cut_the_list() {
        let short = layout(5, 4, 8);
        assert_eq!((short.row, short.height, short.scroll), (24, 136, false));
        assert_eq!(short.content, 136);
        let long = layout(20, 4, 8);
        assert_eq!((long.height, long.content), (400, 480));
        assert!(long.scroll);
        assert_eq!(long.step, 0.25);
        let exactly = layout(16, 4, 8);
        assert!(!exactly.scroll);
        assert_eq!(exactly.height, 400);
        assert_eq!(exactly.step, 0.0);
        let empty = layout(0, 4, 8);
        assert_eq!((empty.height, empty.content, empty.scroll), (16, 16, false));
    }

    #[test]
    fn opening_sets_the_scroll_bit_and_the_height() {
        let mut large = list(20);
        let placed = large.open(10, 20);
        assert!(placed.scroll);
        assert_eq!(large.state & SCROLL_BIT, SCROLL_BIT);
        assert_eq!((large.left, large.top), (10, 20));
        assert!(large.shown);
        let mut small = list(3);
        small.state |= SCROLL_BIT;
        small.open(0, 0);
        assert_eq!(small.state & SCROLL_BIT, 0);
    }

    #[test]
    fn a_point_hits_the_row_it_is_in() {
        let list = list(5);
        // Row height 24, border 8: row 1 is y 8–31, row 2 32–55.
        assert_eq!(list.row_at(1, 0), 0);
        assert_eq!(list.row_at(1, 8), 1);
        assert_eq!(list.row_at(1, 31), 1);
        assert_eq!(list.row_at(1, 32), 2);
        assert_eq!(list.row_at(1, 1000), 5);
        assert_eq!(list.row_at(1, -1), 0);
        assert_eq!(list.row_at(-1, 20), 0);
        assert_eq!(list.row_at(81, 20), 0);
        assert_eq!(DropDownList::new(Vec::new(), 80, 4, 8).row_at(1, 20), 0);
    }

    #[test]
    fn a_press_arms_a_row_and_the_release_writes_it_back() {
        let mut list = list(5);
        list.open(0, 0);
        let mut owner = Owner {
            chosen: Vec::new(),
            closed: 0,
        };
        assert_eq!(list.press(10, 40, 1), DropPress::Pending(2));
        assert_eq!(list.pending, 2);
        assert_eq!(list.release(&mut owner), DropRelease::Chosen(2));
        assert_eq!(owner.chosen, vec![2]);
        assert_eq!(owner.closed, 1);
        assert!(!list.shown);
        // Nothing armed: the release goes to the parent.
        assert_eq!(list.release(&mut owner), DropRelease::Forward);
        // A press off the rows closes without arming.
        assert_eq!(list.press(10, 2, 1), DropPress::Close);
        assert_eq!(list.press(10, 40, 0), DropPress::Close);
        assert_eq!(list.press(10, 40, 0x4001), DropPress::Hold(2));
    }

    #[test]
    fn a_command_chooses_closes_or_moves() {
        let mut list = list(4);
        list.open(0, 0);
        let mut owner = Owner {
            chosen: Vec::new(),
            closed: 0,
        };
        // Up from no row clamps onto the first and selects it.
        assert_eq!(list.command(0x4002, &mut owner), DropCommand::Moved(1));
        assert_eq!(list.command(0x8002, &mut owner), DropCommand::Moved(2));
        assert_eq!(list.command(0x8002, &mut owner), DropCommand::Moved(3));
        assert_eq!(list.command(0x4002, &mut owner), DropCommand::Moved(2));
        assert_eq!(list.command(0x4002, &mut owner), DropCommand::Moved(1));
        assert_eq!(list.command(0x4002, &mut owner), DropCommand::None);
        assert_eq!(list.command(0x200, &mut owner), DropCommand::Close);
        assert_eq!(owner.closed, 1);
        list.hover = 3;
        assert_eq!(list.command(0x100, &mut owner), DropCommand::Choose(3));
        assert_eq!(owner.chosen, vec![3]);
        assert_eq!(list.focus_without_control(), Some(1));
        list.pending = -1;
        assert_eq!(list.focus_without_control(), None);
        assert_eq!(list.select(3), Some(3));
        assert_eq!((list.selected, list.hover), (3, 3));
        assert_eq!(list.select(0), None);
    }

    #[test]
    fn rows_draw_a_band_on_the_pointer_and_the_selection() {
        let mut list = list(3);
        list.open(0, 0);
        list.hover = 2;
        list.selected = 3;
        let rows = list.rows(&RAYFORM_10);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].band, None);
        assert_eq!(rows[1].band, Some([1, 32, 79, 54]));
        assert_eq!(rows[2].band, Some([1, 56, 79, 78]));
        // Text pen: a unit and a pixel in, the row's middle by the font
        // height (ascent 17 at the 10-point font, row 24: 22).
        assert_eq!(rows[0].pen, (5, 30));
        // Centred rows when the style says so.
        list.style = 0x3000;
        let rows = list.rows(&RAYFORM_10);
        let width = RAYFORM_10.text_width(b"Row 1");
        assert_eq!(rows[0].pen.0, ((80 - width) >> 1) + 4);
    }
}
