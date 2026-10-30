//! The popup list of a combo box (`TDropDownList`): rows six units tall,
//! cut at sixteen with a vertical scrollbar, the row under the pointer
//! armed by a left press, the selection written back to the owner and the
//! list closed.
//!
//! The constructor, layout, draw, close, press, release, hit-test, select,
//! drag and command rules are the original's; the owner is a trait so the
//! list stays free of the form base class. The row metrics, row clamp and
//! step are the shared list helpers of [`crate::controls`]; the list's
//! placement under its owner is [`crate::controls::drop_down_place`].

use crate::controls::{ListMetrics, item_at, step_selection};
use crate::form::attribute;
use crate::gui_font::GuiFont;
use crate::skin::{GuiRect, SkinBatch, SkinPalette, double_frame, faded_rect, selection_band};

pub use crate::controls::ROW_UNITS;
/// The most rows shown before the list scrolls.
pub use crate::controls::VISIBLE_ROWS as MAX_ROWS;

/// The list's style word after its constructor (`0x98`; with the 0x7000
/// bits at 0x3000 the rows' text is centred).
pub const LIST_STYLE: u32 = 0x98;
/// The list's attribute word after its constructor (`0x4a1470`: among
/// others the kept-on-top layer, snapped scrolling and the client fill).
pub const LIST_ATTRIBUTES: u32 = 0x004a_1470;
/// The list's border after its constructor.
pub const LIST_BORDER: i32 = 3;
/// The attribute bit the layout sets when the list is cut: the vertical
/// scrollbar. The layout never clears it.
pub const SCROLL_BIT: u32 = attribute::VSCROLL;

/// What the layout wrote: the height, whether the list is cut, and for a
/// cut list its content height and vertical scroll step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropLayout {
    pub height: i32,
    pub scroll: bool,
    /// The whole list's height; written only for a cut list.
    pub content: Option<i32>,
    /// The vertical scroll step, `1 / (count − 16)` in double precision
    /// narrowed once; written only for a cut list.
    pub step: Option<f32>,
    /// The row's height.
    pub row: i32,
}

/// Lays a list of `count` rows out: a row is six units; up to sixteen rows
/// make the list that tall plus twice the border. Past sixteen the list is
/// sixteen rows tall plus twice the border, its content is every row and
/// its scroll step is one over the rows hidden.
pub fn layout(count: i32, unit: i32, border: i32) -> DropLayout {
    let row = unit * ROW_UNITS;
    if count > MAX_ROWS {
        DropLayout {
            height: border * 2 + (row << 4),
            scroll: true,
            content: Some(row * count),
            step: Some((1.0 / (i64::from(count) - i64::from(MAX_ROWS)) as f64) as f32),
            row,
        }
    } else {
        DropLayout {
            height: row * count + border * 2,
            scroll: false,
            content: None,
            step: None,
            row,
        }
    }
}

/// What a press did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPress {
    /// A left press over the current row armed it (the original's 5).
    Armed(i32),
    /// No row, or not the left button: the list asks to close (3).
    Close,
}

/// What a release did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropRelease {
    /// A row was armed: the selection was written back to the owner and
    /// the list asks to close (the original's 0x80, ending the capture).
    Chosen(i32),
    /// Nothing armed: the release goes to the form base class.
    Forward,
}

/// What a command word did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropCommand {
    /// 0x100 with a selection: written back to the owner; the list asks to
    /// close.
    Choose(i32),
    /// 0x100 without a selection, or 0x200: the list asks to close.
    Close,
    /// A direction moved the selection to this row (through the select
    /// slot).
    Moved(i32),
    /// The word did nothing.
    None,
}

/// The owner of a list, as the combo box's control record is.
pub trait DropDownOwner {
    /// The selection is written back: the list's own callback when one is
    /// set, else the owner record's 0x200 message with the row.
    fn row_chosen(&mut self, row: i32);
    /// The list closes: the owner record's state word loses its on and lit
    /// bits ([`crate::gui_controls::state::ON`] and `LIT`).
    fn drop_closed(&mut self);
}

/// One row's text draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowDraw {
    pub row: i32,
    /// The selected row's text goes through the skin's selected-text draw;
    /// the others are drawn in the skin's list-text ink.
    pub selected: bool,
    /// The text's pen (x, baseline y).
    pub pen: (i32, i32),
}

/// The selection's band: three skin draws over the selected row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionBand {
    /// The fill in the skin's selection pair, x0, y0, x1, y1.
    pub fill: [i32; 4],
    /// The frame in the skin's frame pair at half strength, x0, y0, x1, y1.
    pub frame: [i32; 4],
    /// The last call's x, y and the two lengths it takes.
    pub inner: [i32; 4],
}

/// The popup list.
#[derive(Debug, Clone, PartialEq)]
pub struct DropDownList {
    pub items: Vec<Vec<u8>>,
    /// The selected row, 1-based; 0 for none.
    pub selected: i32,
    /// The form's current control: the row the select slot last took.
    pub hover: i32,
    /// The form's pressed control: the row a left press armed.
    pub pressed: i32,
    /// The form's width.
    pub width: i32,
    pub border: i32,
    pub unit: i32,
    pub style: u32,
    pub attributes: u32,
    /// The list frees its item strings when it closes.
    pub owns_items: bool,
    /// The list has closed.
    pub closed: bool,
}

impl DropDownList {
    /// A list of `items`, `width` pixels wide, with the constructor's
    /// style, attributes and border.
    pub fn new(items: Vec<Vec<u8>>, width: i32, unit: i32) -> Self {
        Self {
            items,
            selected: 0,
            hover: 0,
            pressed: 0,
            width,
            border: LIST_BORDER,
            unit,
            style: LIST_STYLE,
            attributes: LIST_ATTRIBUTES,
            owns_items: false,
            closed: false,
        }
    }

    /// The number of rows.
    pub fn count(&self) -> i32 {
        self.items.len() as i32
    }

    fn metrics(&self) -> ListMetrics {
        ListMetrics {
            unit: self.unit,
            inset: self.border,
        }
    }

    /// The layout slot: [`layout`], and a cut list gains [`SCROLL_BIT`].
    pub fn lay_out(&mut self) -> DropLayout {
        let placed = layout(self.count(), self.unit, self.border);
        if placed.scroll {
            self.attributes |= SCROLL_BIT;
        }
        placed
    }

    /// The client's right edge the draw uses: the width less the border,
    /// less the skin's scrollbar width while [`SCROLL_BIT`] is set.
    pub fn client_right(&self, scrollbar_width: i32) -> i32 {
        if self.attributes & SCROLL_BIT != 0 {
            self.width - self.border - scrollbar_width
        } else {
            self.width - self.border
        }
    }

    /// The row at a point's y in the form's client coordinates:
    /// `(y − border) / row + 1` (truncated), held to the rows.
    pub fn row_at(&self, y: i32) -> i32 {
        let count = self.count();
        if count > 0 {
            return item_at(self.metrics(), y, self.border, count);
        }
        // An empty list: the clamp tests the count before 1.
        let pitch = i64::from(self.unit) * i64::from(ROW_UNITS);
        let row = (i64::from(y) - i64::from(self.border)) / pitch + 1;
        if row > i64::from(count) { count } else { 1 }
    }

    /// The hit-test slot: the form base class's code, unless it hit this
    /// list's client (code −1), which becomes the row at `client_y`.
    pub fn hit(&self, base_code: i32, hit_this_list: bool, client_y: i32) -> i32 {
        if hit_this_list && base_code == -1 {
            self.row_at(client_y)
        } else {
            base_code
        }
    }

    /// The select slot: a row of 1 or more becomes the current control and
    /// the selection (3). Anything else goes to the form base class, which
    /// stores it as the current control and answers `base`; the list adds
    /// 3 when it had a current row.
    pub fn select(&mut self, row: i32, base: u32) -> u32 {
        if row > 0 {
            self.hover = row;
            self.selected = row;
            return 3;
        }
        let had_row = self.hover > 0;
        self.hover = row;
        if had_row { base | 3 } else { base }
    }

    /// The press slot: a left press (bit 1) with a current row arms it;
    /// anything else asks the list to close.
    pub fn press(&mut self, word: u32) -> DropPress {
        if self.hover > 0 && word & 1 != 0 {
            self.pressed = self.hover;
            DropPress::Armed(self.hover)
        } else {
            DropPress::Close
        }
    }

    /// The release slot: with a row armed the selection is written back and
    /// the list asks to close; otherwise the release is the form base
    /// class's.
    pub fn release(&mut self, owner: &mut impl DropDownOwner) -> DropRelease {
        if self.pressed > 0 {
            owner.row_chosen(self.selected);
            DropRelease::Chosen(self.selected)
        } else {
            DropRelease::Forward
        }
    }

    /// The drag slot: while the pressed control is a row or none (0 or
    /// more) a drag does nothing and answers 1; a negative pressed code (a
    /// scrollbar or edge grip) goes to the form base class (`None`).
    pub fn drag_move(&self) -> Option<u32> {
        (self.pressed >= 0).then_some(1)
    }

    /// The command slot: 0x100 writes a selection back and asks to close;
    /// 0x200 asks to close; a word with a low bit set steps the selection
    /// up (0xf000 bits 0x4000) or down (0x8000), held to the rows, and a
    /// changed row goes through the select slot.
    pub fn command(&mut self, word: u32, owner: &mut impl DropDownOwner) -> DropCommand {
        if word == 0x100 {
            if self.selected > 0 {
                owner.row_chosen(self.selected);
                return DropCommand::Choose(self.selected);
            }
            return DropCommand::Close;
        }
        if word == 0x200 {
            return DropCommand::Close;
        }
        if word & 3 == 0 {
            return DropCommand::None;
        }
        let direction = match word & 0xf000 {
            0x4000 => -1,
            0x8000 => 1,
            _ => 0,
        };
        let count = self.count();
        let row = if count > 0 {
            step_selection(self.selected, direction, count)
        } else if self.selected + direction < 1 {
            // An empty list: the clamp tests 1 before the count.
            1
        } else {
            count
        };
        if row == self.selected {
            return DropCommand::None;
        }
        self.select(row, 0);
        DropCommand::Moved(row)
    }

    /// The close slot: the owner hears it, a list that owns its strings
    /// frees them, and the form base class closes the list.
    pub fn close(&mut self, owner: &mut impl DropDownOwner) {
        owner.drop_closed();
        if self.owns_items {
            self.items.clear();
        }
        self.closed = true;
    }

    /// The selection's band, when the selection is one of the rows. Rows
    /// run down from y 1; with `right` the client's right edge the fill
    /// spans 1…`right − border − 1` and the row's top to its bottom less
    /// two, the frame one pixel inside it, and the last call takes (2, row
    /// top + 1) with the lengths `right − border − 3` and `row − 3`.
    pub fn band(&self, scrollbar_width: i32) -> Option<SelectionBand> {
        if self.selected < 1 || self.selected > self.count() {
            return None;
        }
        let row = self.unit * ROW_UNITS;
        let top = 1 + (self.selected - 1) * row;
        let right = self.client_right(scrollbar_width) - self.border;
        Some(SelectionBand {
            fill: [1, top, right - 1, top + row - 2],
            frame: [2, top + 1, right - 2, top + row - 3],
            inner: [2, top + 1, right - 3, row - 3],
        })
    }

    /// Paints the selection's band through the skin, in the original's
    /// order: the double frame in the hot form group's seventh and first
    /// shades, a half-strength gradient from the form group's fifth shade
    /// at the bottom to its first at the top, and the skin's unselected
    /// selection band (the highlight outline and glows) over the inner
    /// rectangle. Nothing draws without a selected row.
    pub fn paint(&self, batch: &mut SkinBatch, palette: &SkinPalette, scrollbar_width: i32) {
        let Some(band) = self.band(scrollbar_width) else {
            return;
        };
        let rect = |r: [i32; 4]| GuiRect {
            x0: r[0] as f32,
            y0: r[1] as f32,
            x1: r[2] as f32,
            y1: r[3] as f32,
        };
        double_frame(
            batch,
            rect(band.fill),
            palette.form_hot[6],
            palette.form_hot[0],
        );
        faded_rect(
            batch,
            rect(band.frame),
            palette.form[4],
            palette.form[0],
            0.5,
        );
        // The last call takes a left, top, width and height.
        let [x, y, w, h] = band.inner;
        selection_band(
            batch,
            GuiRect {
                x0: x as f32,
                y0: y as f32,
                x1: (x + w) as f32,
                y1: (y + h) as f32,
            },
            &palette.highlight,
            false,
            self.unit,
        );
    }

    /// The rows' text, top to bottom. The pen sits a unit and a pixel in,
    /// or with the style's 0x7000 bits at 0x3000 is centred in the form's
    /// width plus a unit; the first baseline is `(row + ascent + 1) / 2 + 1`
    /// and each row is a row lower.
    pub fn rows(&self, font: &GuiFont) -> Vec<RowDraw> {
        let row = self.unit * ROW_UNITS;
        let centred = self.style & 0x7000 == 0x3000;
        let mut pen_y = ((i64::from(row) + i64::from(font.ascent) + 1) as u64 >> 1) as i32 + 1;
        let mut out = Vec::new();
        for (index, item) in self.items.iter().enumerate() {
            let pen_x = if centred {
                let spare = i64::from(self.width) - i64::from(font.text_width(item));
                ((spare as u64) >> 1) as i32 + self.unit
            } else {
                self.unit + 1
            };
            let number = index as i32 + 1;
            out.push(RowDraw {
                row: number,
                selected: number == self.selected,
                pen: (pen_x, pen_y),
            });
            pen_y += row;
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

    fn owner() -> Owner {
        Owner {
            chosen: Vec::new(),
            closed: 0,
        }
    }

    fn list(n: usize) -> DropDownList {
        DropDownList::new(
            (1..=n).map(|i| format!("Row {i}").into_bytes()).collect(),
            80,
            4,
        )
    }

    #[test]
    fn the_constructor_sets_the_lists_words() {
        let list = list(2);
        assert_eq!(
            (list.style, list.attributes, list.border),
            (0x98, 0x4a_1470, 3)
        );
        assert_eq!((list.selected, list.hover, list.pressed), (0, 0, 0));
        assert!(!list.owns_items && !list.closed);
    }

    #[test]
    fn rows_are_six_units_and_sixteen_cut_the_list() {
        // Unit 4, border 3: a row is 24.
        let short = layout(5, 4, 3);
        assert_eq!((short.row, short.height, short.scroll), (24, 126, false));
        assert_eq!((short.content, short.step), (None, None));
        let long = layout(20, 4, 3);
        // 16 × 24 + 6; content 20 × 24; step 1/4.
        assert_eq!((long.height, long.content), (390, Some(480)));
        assert!(long.scroll);
        assert_eq!(long.step, Some(0.25));
        let exactly = layout(16, 4, 3);
        assert!(!exactly.scroll);
        assert_eq!(exactly.height, 390);
        let seventeen = layout(17, 4, 3);
        assert_eq!(seventeen.step, Some(1.0));
        let empty = layout(0, 4, 3);
        assert_eq!((empty.height, empty.scroll), (6, false));
    }

    #[test]
    fn the_layout_sets_the_scroll_bit_and_never_clears_it() {
        let mut large = list(20);
        assert!(large.lay_out().scroll);
        assert_eq!(large.attributes & SCROLL_BIT, SCROLL_BIT);
        let mut small = list(3);
        small.attributes |= SCROLL_BIT;
        assert!(!small.lay_out().scroll);
        assert_eq!(small.attributes & SCROLL_BIT, SCROLL_BIT);
    }

    #[test]
    fn a_point_hits_the_row_it_is_in() {
        let list = list(5);
        // Row height 24, border 3: row 1 is y 3–26, row 2 27–50.
        assert_eq!(list.row_at(3), 1);
        assert_eq!(list.row_at(26), 1);
        assert_eq!(list.row_at(27), 2);
        assert_eq!(list.row_at(1000), 5);
        // Above the rows the division truncates to 0, then held to 1.
        assert_eq!(list.row_at(0), 1);
        assert_eq!(list.row_at(-100), 1);
        // An empty list answers its count, or 1 far enough above.
        let empty = DropDownList::new(Vec::new(), 80, 4);
        assert_eq!(empty.row_at(20), 0);
        assert_eq!(empty.row_at(-30), 1);
        // Only the base class's client code on this list becomes a row.
        assert_eq!(list.hit(-1, true, 30), 2);
        assert_eq!(list.hit(-1, false, 30), -1);
        assert_eq!(list.hit(-8, true, 30), -8);
    }

    #[test]
    fn select_takes_rows_and_hands_the_rest_to_the_base() {
        let mut list = list(4);
        assert_eq!(list.select(3, 0x40), 3);
        assert_eq!((list.selected, list.hover), (3, 3));
        // A current row: the base word gains 3; the selection stays.
        assert_eq!(list.select(0, 0x40), 0x43);
        assert_eq!((list.selected, list.hover), (3, 0));
        // No current row: the base word alone.
        assert_eq!(list.select(-1, 0x40), 0x40);
    }

    #[test]
    fn a_press_arms_the_current_row_and_the_release_writes_the_selection() {
        let mut list = list(5);
        let mut owner = owner();
        // No current row: the press closes.
        assert_eq!(list.press(1), DropPress::Close);
        list.select(2, 0);
        // Not the left button: closes.
        assert_eq!(list.press(2), DropPress::Close);
        assert_eq!(list.release(&mut owner), DropRelease::Forward);
        assert_eq!(list.press(0x1001), DropPress::Armed(2));
        assert_eq!(list.pressed, 2);
        assert_eq!(list.drag_move(), Some(1));
        assert_eq!(list.release(&mut owner), DropRelease::Chosen(2));
        assert_eq!(owner.chosen, vec![2]);
        // A scrollbar press (negative code) drags through the base class.
        list.pressed = -8;
        assert_eq!(list.drag_move(), None);
        assert_eq!(list.release(&mut owner), DropRelease::Forward);
    }

    #[test]
    fn a_command_chooses_closes_or_steps_the_selection() {
        let mut list = list(4);
        let mut owner = owner();
        // No selection: up clamps onto the first row.
        assert_eq!(list.command(0x4002, &mut owner), DropCommand::Moved(1));
        assert_eq!(list.command(0x8001, &mut owner), DropCommand::Moved(2));
        assert_eq!(list.command(0x8002, &mut owner), DropCommand::Moved(3));
        assert_eq!(list.command(0x4002, &mut owner), DropCommand::Moved(2));
        assert_eq!((list.selected, list.hover), (2, 2));
        // Neither low bit: nothing.
        assert_eq!(list.command(0x4000, &mut owner), DropCommand::None);
        list.select(1, 0);
        assert_eq!(list.command(0x4002, &mut owner), DropCommand::None);
        list.select(4, 0);
        assert_eq!(list.command(0x8002, &mut owner), DropCommand::None);
        assert_eq!(list.command(0x200, &mut owner), DropCommand::Close);
        assert_eq!(list.command(0x100, &mut owner), DropCommand::Choose(4));
        assert_eq!(owner.chosen, vec![4]);
        list.selected = 0;
        assert_eq!(list.command(0x100, &mut owner), DropCommand::Close);
        assert_eq!(owner.chosen, vec![4]);
    }

    #[test]
    fn closing_tells_the_owner_and_frees_owned_items_only() {
        let mut owner = owner();
        let mut list = list(3);
        list.close(&mut owner);
        assert_eq!(owner.closed, 1);
        assert!(list.closed);
        assert_eq!(list.count(), 3);
        let mut owned = self::list(3);
        owned.owns_items = true;
        owned.close(&mut owner);
        assert!(owned.items.is_empty());
        assert_eq!(owner.closed, 2);
    }

    #[test]
    fn the_band_paints_through_the_skin() {
        let mut palette = SkinPalette::filled([10, 20, 30, 255]);
        palette.form[4] = [1, 2, 3, 4];
        palette.form_hot[6] = [5, 6, 7, 8];
        let mut list = list(3);
        let mut batch = SkinBatch::new();
        list.paint(&mut batch, &palette, 15);
        assert!(batch.primitives.is_empty());
        list.selected = 2;
        let mut batch = SkinBatch::new();
        list.paint(&mut batch, &palette, 15);
        assert!(!batch.primitives.is_empty());
        // The double frame, the gradient and the band's outline and glows.
        assert_eq!(batch.primitives.len(), 4);
        // The band is the highlight group's unselected outline, over the
        // inner rectangle from (2, row top + 1).
        let outline = batch.vertices_of(&batch.primitives[2]);
        assert_eq!((outline[0].x, outline[0].y), (2.0, 26.0));
    }

    #[test]
    fn only_the_selection_draws_a_band() {
        let mut list = list(3);
        assert_eq!(list.band(15), None);
        list.select(2, 0);
        // Row 24; row 2 starts at y 25. Right edge 80 − 3 = 77, less the
        // border: 74.
        assert_eq!(
            list.band(15),
            Some(SelectionBand {
                fill: [1, 25, 73, 47],
                frame: [2, 26, 72, 46],
                inner: [2, 26, 71, 21],
            })
        );
        // With the scrollbar bit the skin's scrollbar width comes off too.
        list.attributes |= SCROLL_BIT;
        assert_eq!(list.band(15).map(|band| band.fill[2]), Some(58));
        list.selected = 4;
        assert_eq!(list.band(15), None);
    }

    #[test]
    fn rows_place_their_text_a_unit_in_and_a_row_apart() {
        let mut list = list(3);
        list.select(3, 0);
        let rows = list.rows(&RAYFORM_10);
        assert_eq!(rows.len(), 3);
        // (24 + 17 + 1) / 2 + 1 = 22, then a row (24) lower each.
        assert_eq!(rows[0].pen, (5, 22));
        assert_eq!(rows[1].pen, (5, 46));
        assert_eq!(rows[2].pen, (5, 70));
        assert!(!rows[0].selected && rows[2].selected);
        // Centred rows when the style says so.
        list.style = 0x3000;
        let rows = list.rows(&RAYFORM_10);
        let width = RAYFORM_10.text_width(b"Row 1");
        assert_eq!(rows[0].pen.0, ((80 - width) >> 1) + 4);
    }
}
