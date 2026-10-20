//! The table view (`TGUITableView`): rows of text cells under a header
//! of titled columns, rows 6 units tall below a 6-unit header. A press
//! selects a row (disabled rows cannot be), a second press on the selected
//! row edits an editable cell in place or opens a column's choice list, a
//! double press activates it, and a press on the line after the last row
//! of a table that grows adds a row and edits it. Down skips disabled
//! rows, Up does not. Clearing the first cell of the last row deletes the
//! row. The table never sorts: rows stay in the order they were added.
//! Rules: `docs/UI.md` (Tables).

use crate::form::ClientRect;
use crate::gui_font::{GuiFont, round_i32};

/// Column flags.
pub mod column {
    /// The title is centred.
    pub const CENTRED: u32 = 0x1;
    /// The cells are edited in place.
    pub const EDITABLE: u32 = 0x20;
    /// A press opens the column's choice list.
    pub const CHOICE: u32 = 0x100;
}

/// A row flag: the row is greyed and cannot be selected.
pub const ROW_DISABLED: u32 = 0x2;

/// Pointer flags the table reads.
pub mod pointer {
    /// The event comes from the gamepad's stick (with a direction bit).
    pub const STICK: u32 = 0x2;
    /// A press that leaves an open edit unless it lands on the edited
    /// column (what else the bit means is not known).
    pub const STRICT: u32 = 0x1000;
    /// A press.
    pub const PRESS: u32 = 0x2000;
    /// A drag begins.
    pub const DRAG: u32 = 0x4000;
    /// A double press.
    pub const DOUBLE: u32 = 0x8000;
}

/// Command words the table's command handler reads.
pub mod command {
    /// The activation key goes down.
    pub const ACTIVATE_DOWN: u32 = 0x100;
    /// The activation key goes up.
    pub const ACTIVATE_UP: u32 = 0x200;
    /// A step's source bits (1 the keys, 2 the stick); its direction is
    /// in `& 0xf000`.
    pub const STEP: u32 = 0x3;
    pub const UP: u32 = 0x4000;
    pub const DOWN: u32 = 0x8000;
}

/// A column: its title, width, flags, header icon and choices.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableColumn {
    pub title: Vec<u8>,
    pub width: i32,
    pub flags: u32,
    /// A large-atlas icon over the title (0 for none).
    pub icon: i32,
    /// The choice list's entries.
    pub choices: Vec<Vec<u8>>,
    /// The owner draws this column's cells itself.
    pub custom: bool,
}

/// A row: its flags and one text per column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableRow {
    pub flags: u32,
    pub cells: Vec<Vec<u8>>,
}

/// What the table told its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableEvent {
    /// The selection moved.
    Selected(i32),
    /// The selected row was activated (double press or key).
    Activated(i32),
    /// The stick pushed right on the selected row.
    Context(i32),
    /// An edit changed a cell (row, column).
    Edited(i32, i32),
    /// A choice was picked for a cell (column, 0-based choice).
    Chosen(i32, i32),
}

/// What a press did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableReply {
    /// The form's own handling takes it.
    Form,
    /// Handled, with the original's result code.
    Code(u32),
    /// A drag of row `row` (0-based) begins, with the grab offset.
    Drag {
        row: i32,
        grab: (i32, i32),
        kind: i32,
    },
    /// The choice list of the selected row's cell opens at `at`, `width`
    /// wide.
    Choices {
        column: i32,
        at: (i32, i32),
        width: i32,
    },
}

/// A table view's state.
#[derive(Debug, Clone, PartialEq)]
pub struct TableView {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
    /// The selected row, 1-based (0 for none).
    pub selected: i32,
    /// The row under the pointer, 1-based (0 or less for none).
    pub hover: i32,
    pub row_height: i32,
    /// The column being edited (or that opened its choices).
    pub edit_column: i32,
    /// A cell edit is open.
    pub editing: bool,
    /// The edit's text, caret and anchor.
    pub edit: Vec<u8>,
    pub edit_caret: i32,
    pub edit_anchor: i32,
    /// Rows may be dragged, as this kind (0: they may not).
    pub drag_kind: i32,
    /// Command bits that activate the selected row.
    pub activate_keys: u32,
    /// A press on the line after the last row adds a row.
    pub grows: bool,
    /// The activation key is held.
    pub pressed: bool,
    /// The table takes the focus on a press (the result code says so).
    pub focusable: bool,
    pub client: ClientRect,
    pub content_height: i32,
    pub scroll: f32,
    pub scroll_step: f32,
    pub unit: i32,
}

impl TableView {
    pub fn new(unit: i32, client: ClientRect) -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            selected: 0,
            hover: 0,
            row_height: 6 * unit,
            edit_column: 0,
            editing: false,
            edit: Vec::new(),
            edit_caret: 0,
            edit_anchor: 0,
            drag_kind: 0,
            activate_keys: 0,
            grows: false,
            pressed: false,
            focusable: true,
            client,
            content_height: client.height(),
            scroll: 0.0,
            scroll_step: 0.0,
            unit,
        }
    }

    /// The row count.
    pub fn row_count(&self) -> i32 {
        self.rows.len() as i32
    }

    /// Lays the table out: rows six units tall, the content the view plus
    /// all rows but one, one row's share of the scroll (none under two
    /// rows).
    pub fn realign(&mut self) {
        self.row_height = 6 * self.unit;
        let rows = self.row_count();
        self.content_height = self.client.height() + (rows - 1) * self.row_height;
        self.scroll_step = if rows < 2 {
            0.0
        } else {
            (1.0 / f64::from(rows - 1)) as f32
        };
    }

    /// Keeps each column's share of the unit when it changes from
    /// `old_unit`.
    pub fn rescale(&mut self, old_unit: i32, unit: i32) {
        if old_unit != 0 {
            for c in &mut self.columns {
                c.width = c.width / old_unit * unit;
            }
        }
        self.unit = unit;
    }

    /// Adds an empty row at the end.
    pub fn add_row(&mut self) {
        let cells = vec![Vec::new(); self.columns.len()];
        self.rows.push(TableRow { flags: 0, cells });
    }

    /// Removes the row at 0-based `index`; a selection past the end moves
    /// to the new last row (the pointer's row with it when they matched).
    pub fn remove_row(&mut self, index: i32) {
        if index < 0 || index >= self.row_count() {
            return;
        }
        self.rows.remove(index as usize);
        let rows = self.row_count();
        if rows < self.selected {
            if self.hover == self.selected {
                self.hover = rows;
            }
            self.selected = rows;
        }
    }

    /// The first row drawn below the header: the scroll's share of all
    /// rows but one, rounded.
    pub fn first_visible(&self) -> i32 {
        round_i32(f64::from((self.row_count() - 1) as f32 * self.scroll))
    }

    /// The 1-based row under `y` (scrolled client coordinates), or -1:
    /// none in the header, above the first drawn row or past the last,
    /// except the line after the last in a table that grows.
    pub fn row_at(&self, y: i32) -> i32 {
        let y = y + self.unit;
        if y < 6 * self.unit {
            return -1;
        }
        let row = ((i64::from(y) - i64::from(self.unit)) / i64::from(self.row_height)) as i32;
        let rows = self.row_count();
        if rows < row || row <= self.first_visible() {
            if self.grows && rows + 1 == row {
                return row;
            }
            return -1;
        }
        row
    }

    /// The pointer moves over row `row` (from `row_at`).
    pub fn hover_row(&mut self, row: i32) {
        self.hover = row;
    }

    /// The column under `x` (client coordinates): the first whose right
    /// edge reaches it.
    pub fn column_at(&self, x: i32) -> i32 {
        let mut column = 0;
        let mut right = self.columns.first().map_or(0, |c| c.width);
        while right < x {
            column += 1;
            match self.columns.get(column as usize) {
                Some(c) => right += c.width,
                None => break,
            }
        }
        column
    }

    /// The left edge of `column`'s cells: a unit in from the widths before.
    pub fn column_x(&self, column: i32) -> i32 {
        self.unit
            + self
                .columns
                .iter()
                .take(column.max(0) as usize)
                .map(|c| c.width)
                .sum::<i32>()
    }

    fn cell(&self, row: i32, column: i32) -> Option<&Vec<u8>> {
        self.rows
            .get(row as usize - 1)
            .and_then(|r| r.cells.get(column as usize))
    }

    /// Opens an edit of the selected row's edit column, when editable: the
    /// whole text selected.
    pub fn begin_edit(&mut self) {
        let editable = self
            .columns
            .get(self.edit_column as usize)
            .is_some_and(|c| c.flags & column::EDITABLE != 0);
        if !editable || self.selected < 1 {
            return;
        }
        self.editing = true;
        self.edit = self
            .cell(self.selected, self.edit_column)
            .cloned()
            .unwrap_or_default();
        self.edit_anchor = 0;
        self.edit_caret = self.edit.len() as i32;
    }

    /// Closes the edit: an emptied first cell of the last row deletes the
    /// row, else a changed text is stored and reported.
    pub fn commit_edit(&mut self) -> Vec<TableEvent> {
        self.editing = false;
        let rows = self.row_count();
        if self.edit_column == 0 && self.edit.is_empty() && self.selected == rows {
            self.remove_row(rows - 1);
            return Vec::new();
        }
        let (row, col) = (self.selected, self.edit_column);
        let changed = self.cell(row, col).is_some_and(|c| *c != self.edit);
        if changed {
            if let Some(cell) = self
                .rows
                .get_mut(row as usize - 1)
                .and_then(|r| r.cells.get_mut(col as usize))
            {
                *cell = self.edit.clone();
            }
            return vec![TableEvent::Edited(row, col)];
        }
        Vec::new()
    }

    /// The edit's caret under `x` (client coordinates): bytes from the
    /// column's left edge, before the first byte whose middle is past it.
    pub fn edit_caret_at(&self, x: i32, font: &GuiFont) -> i32 {
        let mut x = x - self.column_x(self.edit_column);
        let mut caret = self.edit.len() as i32;
        for (k, &b) in self.edit.iter().enumerate() {
            let advance = font.advance(b);
            if x < (advance as u32 >> 1) as i32 {
                caret = k as i32;
                break;
            }
            x -= advance;
        }
        caret.clamp(0, self.edit.len() as i32)
    }

    /// A press on the hovered row at `x` (client coordinates) with
    /// `flags`, as the original's table: see the module notes. A press
    /// with no row under it closes an open edit and goes to the form; the
    /// choice list is modal in the original (code 3 once it closes).
    pub fn press(
        &mut self,
        x: i32,
        flags: u32,
        font: &GuiFont,
        events: &mut Vec<TableEvent>,
    ) -> TableReply {
        if self.hover < 1 {
            if self.editing {
                events.extend(self.commit_edit());
            }
            return TableReply::Form;
        }
        if flags & pointer::DRAG != 0 {
            if self.drag_kind != 0 && self.hover > 0 {
                return TableReply::Drag {
                    row: self.hover - 1,
                    grab: (x - self.unit, (self.row_height as u32 >> 1) as i32),
                    kind: self.drag_kind,
                };
            }
            return TableReply::Code(0x10);
        }
        if self.hover == self.selected {
            if flags & pointer::DOUBLE == pointer::DOUBLE {
                if self.selected > 0 {
                    events.push(TableEvent::Activated(self.selected));
                }
                return TableReply::Code(0x41);
            }
            let column = self.column_at(x);
            if !self.editing {
                if flags & pointer::PRESS == pointer::PRESS {
                    if column >= self.columns.len() as i32 {
                        return TableReply::Code(0);
                    }
                    if self.selected > 0 {
                        self.edit_column = column;
                        let c = &self.columns[column as usize];
                        if flags & pointer::STICK == pointer::STICK {
                            events.push(TableEvent::Context(self.selected));
                        } else if c.flags & column::EDITABLE == 0 {
                            if c.flags & column::CHOICE != 0 {
                                return TableReply::Choices {
                                    column,
                                    at: (self.column_x(column), self.unit),
                                    width: c.width,
                                };
                            }
                        } else {
                            self.begin_edit();
                            return TableReply::Code(0x10);
                        }
                    }
                }
                return TableReply::Code(0x10);
            }
            if column == self.edit_column || flags & pointer::STRICT != pointer::STRICT {
                self.edit_caret = self.edit_caret_at(x, font);
                self.edit_anchor = self.edit_caret;
                return TableReply::Code(5);
            }
            events.extend(self.commit_edit());
            return TableReply::Code(3);
        }
        if flags & pointer::PRESS != 0 {
            if self.editing {
                events.extend(self.commit_edit());
                return TableReply::Code(0x10);
            }
            let code = if self.focusable { 0x11 } else { 1 };
            if self.row_count() < self.hover {
                self.selected = self.hover;
                self.edit_column = 0;
                self.add_row();
                self.begin_edit();
            } else if self.rows[self.hover as usize - 1].flags & ROW_DISABLED == 0 {
                self.selected = self.hover;
                events.push(TableEvent::Selected(self.selected));
            }
            return TableReply::Code(code);
        }
        TableReply::Code(0x10)
    }

    /// A picked choice (1-based) for the edit column's cell.
    pub fn choose(&mut self, choice: i32) -> Vec<TableEvent> {
        let col = self.edit_column;
        let text = self
            .columns
            .get(col as usize)
            .and_then(|c| c.choices.get((choice - 1).max(0) as usize))
            .cloned()
            .unwrap_or_default();
        if let Some(cell) = self
            .rows
            .get_mut(self.selected.max(1) as usize - 1)
            .and_then(|r| r.cells.get_mut(col as usize))
        {
            *cell = text;
        }
        vec![TableEvent::Chosen(col, choice - 1)]
    }

    /// A key while editing: Enter commits, Escape abandons, Delete on an
    /// empty edit deletes the row; others go to the edit field (`None`:
    /// not the table's, as when no edit is open).
    pub fn edit_key(&mut self, key: u32, typed: bool) -> Option<Vec<TableEvent>> {
        if !self.editing {
            return None;
        }
        if !typed && key > 0x0c {
            match key {
                0x0d => return Some(self.commit_edit()),
                0x1b => {
                    self.editing = false;
                    return Some(Vec::new());
                }
                0x2e if self.edit.is_empty() => {
                    self.remove_row(self.selected - 1);
                    self.editing = false;
                    return Some(Vec::new());
                }
                _ => {}
            }
        }
        Some(Vec::new())
    }

    /// A command word: the activation key's press and release, the
    /// activation mask, or a step up or down (Down passes disabled rows,
    /// staying put when only disabled rows follow; Up does not).
    pub fn command(&mut self, word: u32) -> Vec<TableEvent> {
        let mut events = Vec::new();
        match word {
            command::ACTIVATE_DOWN => {
                if !self.pressed {
                    self.pressed = true;
                } else if self.selected < 1 {
                    self.pressed = false;
                } else {
                    events.push(TableEvent::Activated(self.selected));
                }
            }
            command::ACTIVATE_UP => self.pressed = false,
            _ => {
                let mask = self.activate_keys;
                if mask != 0 && word & mask == mask {
                    if self.selected > 0 {
                        events.push(TableEvent::Activated(self.selected));
                    }
                } else if word & command::STEP != 0 {
                    let rows = self.row_count();
                    let mut next = self.selected;
                    if word & 0xf000 == command::UP {
                        next -= 1;
                    } else if word & 0xf000 == command::DOWN {
                        loop {
                            next += 1;
                            if rows < next || self.rows[next as usize - 1].flags & ROW_DISABLED == 0
                            {
                                break;
                            }
                        }
                        if rows < next {
                            next = self.selected;
                        }
                    }
                    next = next.max(1).min(rows);
                    if next != self.selected {
                        self.selected = next;
                        self.hover = next;
                        events.push(TableEvent::Selected(next));
                    }
                }
            }
        }
        events
    }
}

/// Where a table's parts are drawn.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TableDraw {
    /// Header boxes (x0, x1) from the top to 6 units down; the last runs
    /// to the client's width.
    pub headers: Vec<(i32, i32)>,
    /// The selected row's highlight (x, y, w, h) and whether it is filled
    /// (the pointer is on it and no edit is open).
    pub selection: Option<([i32; 4], bool)>,
    /// The scroll offset the rows and selection are drawn at.
    pub scroll_offset: i32,
    /// Cell texts: row, column, pen (x, y: the glyphs' left and bottom
    /// edge) and whether greyed.
    pub cells: Vec<(i32, i32, (i32, i32), bool)>,
    /// The cell the edit field draws in place of its text.
    pub edit_at: Option<(i32, i32)>,
    /// Owner-drawn cells: row, column, x, y and width.
    pub custom: Vec<(i32, i32, i32, i32, i32)>,
    /// Titles: column and pen (x, y).
    pub titles: Vec<(i32, (i32, i32))>,
    /// Header icons: handle and position.
    pub icons: Vec<(i32, (i32, i32))>,
}

impl TableView {
    /// The table's draw at its scroll: headers, the selected row, cells
    /// from the first drawn row (owner-drawn columns from the first row),
    /// titles (centred ones by their width in `font`) and header icons
    /// centred in the `icon_cell`-pixel cells of the large atlas.
    pub fn draw(&self, font: &GuiFont, icon_cell: i32) -> TableDraw {
        let u = self.unit;
        let width = self.client.width();
        let mut out = TableDraw::default();
        let n = self.columns.len();
        let mut x0 = 0;
        for (i, c) in self.columns.iter().enumerate() {
            let x1 = if i + 1 == n { width } else { c.width + x0 };
            out.headers.push((x0, x1));
            x0 = x1;
        }
        out.scroll_offset = -round_i32(f64::from(
            (self.content_height - self.client.height()) as f32 * self.scroll,
        ));
        let first = self.first_visible();
        if first < self.selected {
            let y = 6 * u * self.selected;
            let filled = self.hover == self.selected && !self.editing;
            out.selection = Some(([0, y, width, self.row_height - 2], filled));
        }
        let mut left = u;
        for (i, c) in self.columns.iter().enumerate() {
            let right = if i + 1 == n { width } else { c.width + left };
            if c.custom {
                for r in 0..self.row_count() {
                    out.custom
                        .push((r + 1, i as i32, left, 11 * u + 6 * u * r, right - left));
                }
            }
            left = right;
        }
        let mut x = 0;
        for (i, c) in self.columns.iter().enumerate() {
            if !c.custom {
                for r in first..self.row_count() {
                    let y = 6 * u * r + 11 * u;
                    let is_edit =
                        self.editing && self.selected == r + 1 && i as i32 == self.edit_column;
                    if is_edit {
                        out.edit_at = Some((x + u, y));
                    } else {
                        let grey = self.rows[r as usize].flags & ROW_DISABLED != 0;
                        out.cells.push((r + 1, i as i32, (x + u, y), grey));
                    }
                }
            }
            x += c.width;
        }
        let mut x = 0;
        for (i, c) in self.columns.iter().enumerate() {
            let pen = if c.flags & column::CENTRED == 0 {
                u + x
            } else {
                u + (c.width - font.text_width(&c.title)) / 2 + x
            };
            out.titles.push((i as i32, (pen, 5 * u)));
            if c.icon > 0 {
                out.icons
                    .push((c.icon, (x + (c.width - icon_cell) / 2, u >> 1)));
            }
            x += c.width;
        }
        out
    }

    /// The dragged row's label: its box (x, y, w, h) at the pointer less
    /// the grab, and the text's pen, its first cell drawn in `font`.
    pub fn drag_label(
        &self,
        row: i32,
        pointer: (i32, i32),
        grab: (i32, i32),
        font: &GuiFont,
    ) -> ([i32; 4], (i32, i32)) {
        let text = self
            .rows
            .get(row as usize)
            .and_then(|r| r.cells.first())
            .cloned()
            .unwrap_or_default();
        let w = font.text_width(&text);
        let x = pointer.0 - grab.0;
        let y = pointer.1 - grab.1;
        let pen_y = ((self.row_height + font.ascent + 1) as u32 >> 1) as i32 + y;
        ([x, y, w - 1, self.row_height - 2], (x, pen_y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::RAYFORM_10;

    fn table(rows: usize) -> TableView {
        let mut t = TableView::new(
            5,
            ClientRect {
                left: 2,
                top: 2,
                right: 302,
                bottom: 202,
            },
        );
        t.columns = vec![
            TableColumn {
                title: b"Name".to_vec(),
                width: 100,
                flags: column::EDITABLE,
                ..Default::default()
            },
            TableColumn {
                title: b"Kind".to_vec(),
                width: 80,
                flags: column::CHOICE | column::CENTRED,
                choices: vec![b"Sword".to_vec(), b"Axe".to_vec()],
                ..Default::default()
            },
            TableColumn {
                title: b"Note".to_vec(),
                width: 60,
                ..Default::default()
            },
        ];
        for i in 0..rows {
            t.add_row();
            t.rows[i].cells[0] = format!("r{i}").into_bytes();
        }
        t.realign();
        t
    }

    #[test]
    fn rows_are_six_units_under_a_six_unit_header() {
        let t = table(5);
        assert_eq!(
            (t.row_height, t.content_height, t.scroll_step),
            (30, 320, 0.25)
        );
        assert_eq!(table(1).scroll_step, 0.0);
        // The header and row 0's band are no row; row 1 starts at 30.
        assert_eq!(t.row_at(10), -1);
        assert_eq!(t.row_at(29), -1);
        assert_eq!((t.row_at(30), t.row_at(59), t.row_at(60)), (1, 1, 2));
        assert_eq!(t.row_at(5 * 30 + 1), 5);
        assert_eq!(t.row_at(6 * 30 + 1), -1);
        let mut g = table(5);
        g.grows = true;
        assert_eq!(g.row_at(6 * 30 + 1), 6);
    }

    #[test]
    fn rows_behind_the_header_are_not_hit() {
        let mut t = table(5);
        t.scroll = 0.5;
        assert_eq!(t.first_visible(), 2);
        assert_eq!((t.row_at(2 * 30 + 1), t.row_at(3 * 30 + 1)), (-1, 3));
    }

    #[test]
    fn columns_by_their_right_edges() {
        let t = table(1);
        assert_eq!(
            [0, 100, 101, 180, 181, 400].map(|x| t.column_at(x)),
            [0, 0, 1, 1, 2, 3]
        );
        assert_eq!((t.column_x(0), t.column_x(2)), (5, 185));
    }

    #[test]
    fn a_press_selects_and_a_second_edits() {
        let mut t = table(3);
        let mut ev = Vec::new();
        t.hover = 2;
        assert_eq!(
            t.press(10, pointer::PRESS, &RAYFORM_10, &mut ev),
            TableReply::Code(0x11)
        );
        assert_eq!((t.selected, ev.clone()), (2, vec![TableEvent::Selected(2)]));
        // The second press on the editable column opens an edit of all of
        // the cell.
        assert_eq!(
            t.press(10, pointer::PRESS, &RAYFORM_10, &mut ev),
            TableReply::Code(0x10)
        );
        assert!(t.editing);
        assert_eq!(
            (t.edit.as_slice(), t.edit_anchor, t.edit_caret),
            (b"r1".as_slice(), 0, 2)
        );
        t.edit = b"blade".to_vec();
        ev.clear();
        assert_eq!(
            t.edit_key(0x0d, false),
            Some(vec![TableEvent::Edited(2, 0)])
        );
        assert_eq!(t.rows[1].cells[0], b"blade");
        // On the choice column it opens the list under the cell.
        let r = t.press(150, pointer::PRESS, &RAYFORM_10, &mut ev);
        assert_eq!(
            r,
            TableReply::Choices {
                column: 1,
                at: (105, 5),
                width: 80
            }
        );
        assert_eq!(t.choose(2), vec![TableEvent::Chosen(1, 1)]);
        assert_eq!(t.rows[1].cells[1], b"Axe");
        // A double press activates.
        assert_eq!(
            t.press(10, pointer::DOUBLE, &RAYFORM_10, &mut ev),
            TableReply::Code(0x41)
        );
        assert!(ev.contains(&TableEvent::Activated(2)));
    }

    #[test]
    fn a_disabled_row_is_not_selected() {
        let mut t = table(3);
        t.rows[1].flags = ROW_DISABLED;
        t.hover = 2;
        let mut ev = Vec::new();
        t.press(10, pointer::PRESS, &RAYFORM_10, &mut ev);
        assert_eq!(t.selected, 0);
        assert!(ev.is_empty());
    }

    #[test]
    fn a_growing_table_adds_and_drops_its_last_row() {
        let mut t = table(2);
        t.grows = true;
        t.hover = 3;
        let mut ev = Vec::new();
        t.press(10, pointer::PRESS, &RAYFORM_10, &mut ev);
        assert_eq!((t.row_count(), t.selected, t.editing), (3, 3, true));
        // Committing it empty deletes it again.
        assert!(t.commit_edit().is_empty());
        assert_eq!((t.row_count(), t.selected), (2, 2));
    }

    #[test]
    fn escape_and_delete_while_editing() {
        let mut t = table(3);
        t.selected = 3;
        t.begin_edit();
        assert_eq!(t.edit_key(0x1b, false), Some(Vec::new()));
        assert!(!t.editing);
        t.selected = 2;
        t.begin_edit();
        t.edit.clear();
        t.edit_key(0x2e, false);
        assert_eq!(t.row_count(), 2);
        assert_eq!(t.edit_key(0x0d, false), None);
    }

    #[test]
    fn down_passes_disabled_rows_and_up_does_not() {
        let mut t = table(4);
        t.rows[1].flags = ROW_DISABLED;
        t.selected = 1;
        assert_eq!(
            t.command(command::STEP | command::DOWN),
            vec![TableEvent::Selected(3)]
        );
        assert_eq!(
            t.command(command::STEP | command::UP),
            vec![TableEvent::Selected(2)]
        );
        t.rows[3].flags = ROW_DISABLED;
        t.selected = 3;
        assert!(t.command(command::STEP | command::DOWN).is_empty());
        t.activate_keys = 0x10;
        assert_eq!(t.command(0x10), vec![TableEvent::Activated(3)]);
        // The activation key needs a press and a second press.
        assert!(t.command(command::ACTIVATE_DOWN).is_empty());
        assert_eq!(
            t.command(command::ACTIVATE_DOWN),
            vec![TableEvent::Activated(3)]
        );
    }

    #[test]
    fn removing_rows_keeps_the_selection_in_range() {
        let mut t = table(3);
        t.selected = 3;
        t.hover = 3;
        t.remove_row(2);
        assert_eq!((t.selected, t.hover), (2, 2));
        t.remove_row(9);
        assert_eq!(t.row_count(), 2);
    }

    #[test]
    fn a_drag_takes_the_pointer_row() {
        let mut t = table(3);
        t.hover = 2;
        let mut ev = Vec::new();
        assert_eq!(
            t.press(40, pointer::DRAG, &RAYFORM_10, &mut ev),
            TableReply::Code(0x10)
        );
        t.drag_kind = 7;
        assert_eq!(
            t.press(40, pointer::DRAG, &RAYFORM_10, &mut ev),
            TableReply::Drag {
                row: 1,
                grab: (35, 15),
                kind: 7
            }
        );
        let (rect, pen) = t.drag_label(1, (100, 100), (35, 15), &RAYFORM_10);
        assert_eq!(rect[..2], [65, 85]);
        assert_eq!(rect[3], 28);
        // Centred by the font's text height (17 in Rayform10R).
        assert_eq!(pen, (65, 85 + (30 + 17 + 1) / 2));
    }

    #[test]
    fn the_draw_lays_out_cells_and_titles() {
        let mut t = table(3);
        t.selected = 2;
        t.hover = 2;
        t.columns[2].icon = 4;
        let d = t.draw(&RAYFORM_10, 28);
        assert_eq!(d.headers, vec![(0, 100), (100, 180), (180, 300)]);
        assert_eq!(d.selection, Some(([0, 60, 300, 28], true)));
        // Row 1 at 11 units, the second column a unit in from 100.
        assert!(d.cells.contains(&(1, 0, (5, 55), false)));
        assert!(d.cells.contains(&(3, 1, (105, 115), false)));
        let kind_width = RAYFORM_10.text_width(b"Kind");
        assert_eq!(d.titles[1], (1, (5 + (80 - kind_width) / 2 + 100, 25)));
        assert_eq!(d.titles[0], (0, (5, 25)));
        assert_eq!(d.icons, vec![(4, (196, 2))]);
    }
}
