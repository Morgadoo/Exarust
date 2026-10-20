//! The item view (`TGUIItemView`): a list of named entries with icons,
//! laid out as a grid (names under icons, `GRID`), a vertical list
//! (`LIST`) or columns that fill top to bottom and scroll sideways (the
//! default). Icons are rendered on demand, one missing icon of the visible
//! ones per request. A press selects (or, with check marks, toggles a
//! mark), a second press renames, a double press activates; typed letters
//! jump to the next entry starting with them (case-blind, forgotten after
//! 600 ms). Rules: `docs/UI.md` (Item views).

use crate::form::ClientRect;
use crate::gui_font::{GuiFont, round_i32};

/// Layout and behaviour flags.
pub mod style {
    /// Names under icons in a grid.
    pub const GRID: u32 = 0x1000_0000;
    /// One entry per row.
    pub const LIST: u32 = 0x0800_0000;
    /// Icons only; the pointer's entry shows its name as a tooltip.
    pub const ICONS_ONLY: u32 = 0x4000_0000;
    /// The icon size is set by the owner, not 4 units.
    pub const FIXED_ICON: u32 = 0x2000_0000;
    /// A grid row is its pitch tall, with no names.
    pub const NO_NAMES: u32 = 0x0020_0000;
    /// Names start two units in, with no icon before them.
    pub const NO_ICON_SPACE: u32 = 0x0010_0000;
    /// Marked entries draw the marker icon (with no icon renderer).
    pub const MARKERS: u32 = 0x0040_0000;
    /// The whole row is hit and highlighted, not just icon and name.
    pub const FULL_ROW: u32 = 0x0100_0000;
    /// Presses toggle marks (both bits).
    pub const CHECKS: u32 = 0x00c0_0000;
    /// Entries may be renamed.
    pub const RENAME: u32 = 0x20;
    /// A press needs an entry under the pointer.
    pub const NEEDS_ENTRY: u32 = 0x40;
}

/// An entry's flags.
pub mod entry {
    /// Marked (checked).
    pub const MARKED: u32 = 0x1;
    /// Greyed; a press does not mark it.
    pub const DISABLED: u32 = 0x2;
}

/// Type-ahead forgets its letters after this long, in milliseconds.
pub const TYPE_AHEAD_MS: u32 = 600;
/// The ellipsis byte a cut name ends with.
pub const ELLIPSIS: u8 = 0x85;
/// The default icon size.
pub const DEFAULT_ICON: i32 = 16;
/// The large-atlas icon marked entries draw.
pub const DEFAULT_MARKER: i32 = 0x11;

/// An entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ViewEntry {
    pub name: Vec<u8>,
    /// Its icon's handle (0 until rendered).
    pub icon: i32,
    pub flags: u32,
    /// What it stands for.
    pub item: u32,
}

/// What the view told its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewEvent {
    /// The selection moved (or a mark changed).
    Selected(i32),
    /// The selected entry was activated.
    Activated(i32),
    /// An entry was renamed.
    Renamed(i32),
    /// Show this name as the pointer's tooltip.
    Tooltip(Vec<u8>),
}

/// What a press did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewReply {
    Form,
    Code(u32),
    /// A drag of entry `index` (0-based) begins, grabbed at `grab`.
    Drag {
        index: i32,
        grab: (i32, i32),
        kind: i32,
    },
}

/// An item view's state.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemView {
    pub entries: Vec<ViewEntry>,
    pub style: u32,
    /// The selected entry, 1-based.
    pub selected: i32,
    /// The entry under the pointer, 1-based.
    pub hover: i32,
    /// The last entry a check press toggled.
    pub checked: i32,
    pub icon: i32,
    /// A row's height and an entry's pitch across.
    pub row: i32,
    pub pitch: i32,
    /// Entries per grid row, rows per column, or rows in view.
    pub per_line: i32,
    /// Entries up to this one cannot be renamed.
    pub rename_from: i32,
    pub renaming: bool,
    /// The name before the rename.
    pub backup: Vec<u8>,
    pub drag_kind: i32,
    /// Type-ahead's letters and when the last came.
    pub typed: Vec<u8>,
    pub typed_at: u32,
    /// The icons' tint.
    pub tint: [u8; 4],
    pub marker: i32,
    /// The activation key is held.
    pub pressed: bool,
    /// The view takes the focus on a press (the result code says so).
    pub focusable: bool,
    pub client: ClientRect,
    pub content: (i32, i32),
    pub scroll: (f32, f32),
    pub step: (f32, f32),
    pub unit: i32,
}

impl ItemView {
    pub fn new(unit: i32, style: u32, client: ClientRect) -> Self {
        Self {
            entries: Vec::new(),
            style,
            selected: 0,
            hover: 0,
            checked: 0,
            icon: DEFAULT_ICON,
            row: DEFAULT_ICON + unit,
            pitch: 0,
            per_line: 0,
            rename_from: 0,
            renaming: false,
            backup: Vec::new(),
            drag_kind: 0,
            typed: Vec::new(),
            typed_at: 0,
            tint: [255; 4],
            marker: DEFAULT_MARKER,
            pressed: false,
            focusable: true,
            client,
            content: (client.width(), client.height()),
            scroll: (0.0, 0.0),
            step: (0.0, 0.0),
            unit,
        }
    }

    fn is(&self, bits: u32) -> bool {
        self.style & bits == bits
    }

    fn count(&self) -> i32 {
        self.entries.len() as i32
    }

    /// The unit changed: icons are 4 units (unless fixed), then the
    /// metrics follow.
    pub fn rescale(&mut self, unit: i32, font: &GuiFont) -> Vec<ViewEvent> {
        self.unit = unit;
        if !self.is(style::FIXED_ICON) {
            self.icon = unit << 2;
        }
        if self.is(style::GRID) {
            self.pitch = unit * 4 + self.icon;
        } else {
            self.row = unit + self.icon;
        }
        self.measure(font)
    }

    /// The pitches for the layout, then the layout: a grid's pitch is the
    /// icon plus 4 units and its row adds 6 units and a line (or is the
    /// pitch, without names); a list's pitch is the client width less two
    /// units; columns are the icon plus 2 units apart. A selection past
    /// the end moves to the last entry.
    pub fn measure(&mut self, font: &GuiFont) -> Vec<ViewEvent> {
        let mut events = Vec::new();
        let u = self.unit;
        if self.count() < self.selected {
            self.selected = self.count();
            events.push(ViewEvent::Selected(self.selected));
        }
        if self.is(style::GRID) {
            self.pitch = u * 4 + self.icon;
            self.row = if self.style & style::NO_NAMES == 0 {
                u * 6 + self.icon + font.line_height
            } else {
                self.pitch
            };
        } else if self.is(style::LIST) {
            self.pitch = self.client.width() - u * 2;
        } else {
            self.pitch = u * 2 + self.icon;
        }
        self.realign();
        events
    }

    /// Lays the entries out in the client: entries per line, the content
    /// extent and the scroll step.
    pub fn realign(&mut self) {
        let u = self.unit;
        let (w, h) = (self.client.width(), self.client.height());
        let n = self.count();
        if self.is(style::GRID) {
            self.per_line = ((i64::from(w) - i64::from(u << 1) + 2) / i64::from(self.pitch)) as i32;
            let lines = if self.per_line < 1 {
                0
            } else {
                (n + self.per_line) / self.per_line
            };
            self.content.1 = h + (lines - 1) * self.row;
            self.step.1 = if lines < 2 {
                0.0
            } else {
                (1.0 / f64::from(lines - 1)) as f32
            };
        } else if self.is(style::LIST) {
            self.per_line = ((i64::from(h) - i64::from(u << 1)) / i64::from(self.row)) as i32;
            self.content.1 = h + (n - 1) * self.row;
            self.step.1 = if n < 2 {
                0.0
            } else {
                (1.0 / f64::from(n - 1)) as f32
            };
            self.pitch = w - u * 2;
        } else {
            self.per_line = ((i64::from(h) - i64::from(u << 1)) / i64::from(self.row)) as i32;
            let columns = if self.per_line < 1 {
                0
            } else {
                (n + self.per_line) / self.per_line
            };
            self.content.0 = w + (columns - 1) * self.pitch;
            self.step.0 = if columns < 2 {
                0.0
            } else {
                (1.0 / f64::from(columns - 1)) as f32
            };
        }
    }

    /// The lines (grid), entries (list) in view, top to bottom.
    pub fn rows_in_view(&self) -> (i32, i32) {
        let h = self.client.height();
        let px = round_i32(f64::from((self.content.1 - h) as f32 * self.scroll.1));
        let first = px / self.row;
        let last = (h / self.row + first + 1).min(self.count() - 1);
        (first.max(0), last)
    }

    /// The columns in view, left to right (the default layout).
    pub fn columns_in_view(&self) -> (i32, i32) {
        let w = self.client.width();
        let px = round_i32(f64::from((self.content.0 - w) as f32 * self.scroll.0));
        let first = px / self.pitch;
        let last = (w / self.pitch + first + 1).min(self.count() - 1);
        (first.max(0), last)
    }

    /// The first visible entry still without an icon, to render next.
    pub fn next_icon(&self) -> Option<i32> {
        let (first, last) = if self.is(style::GRID) {
            self.rows_in_view()
        } else if self.is(style::LIST) {
            (0, 0)
        } else {
            self.columns_in_view()
        };
        let n = self.count();
        for line in first..=last {
            let (from, to) = if self.is(style::LIST) {
                self.rows_in_view()
            } else {
                let from = line * self.per_line;
                (from, (from + self.per_line - 1).min(n - 1))
            };
            for i in from..=to {
                if self.entries.get(i as usize).is_some_and(|e| e.icon == 0) {
                    return Some(i);
                }
            }
            if !self.is(style::LIST) && from + self.per_line >= n {
                return None;
            }
        }
        None
    }

    /// The 1-based entry under `x`, `y` (scrolled client coordinates), or
    /// -1. A column entry is hit within its icon and name, the name
    /// measured as its byte count times the font's gap.
    pub fn entry_at(&self, x: i32, y: i32, font: &GuiFont) -> i32 {
        let n = self.count();
        if n < 1 {
            return -1;
        }
        let u = self.unit;
        if self.is(style::GRID) {
            let col = x / self.pitch;
            let index = if col < self.per_line {
                (y / self.row) * self.per_line + col
            } else {
                n
            };
            return if index < n { index + 1 } else { -1 };
        }
        if self.is(style::LIST) {
            let index = ((i64::from(y) - i64::from(u)) / i64::from(self.row)) as i32 + 1;
            return if index <= n { index } else { -1 };
        }
        let r = ((i64::from(y) - i64::from(u)) / i64::from(self.row)) as i32;
        let index = if r < self.per_line {
            ((i64::from(x) - i64::from(u)) / i64::from(self.pitch)) as i32 * self.per_line + r
        } else {
            n
        };
        if index > n - 1 {
            return -1;
        }
        if self.is(style::FULL_ROW) {
            return index + 1;
        }
        let name = self.entries[index as usize].name.len() as i64;
        let reach = i64::from(index / self.per_line) * i64::from(self.pitch)
            + i64::from(self.icon)
            + name * i64::from(font.gap);
        if i64::from(x) - i64::from(u << 1) < reach {
            index + 1
        } else {
            -1
        }
    }

    /// The pointer moves over entry `index` (from `entry_at`): an icons-only
    /// view shows its name.
    pub fn hover_entry(&mut self, index: i32) -> Vec<ViewEvent> {
        if index < 1 {
            return Vec::new();
        }
        self.hover = index;
        if self.style & style::ICONS_ONLY != 0 {
            let name = self.entries.get(index as usize - 1).map(|e| e.name.clone());
            return name.map(ViewEvent::Tooltip).into_iter().collect();
        }
        Vec::new()
    }

    /// Empties the view: no rename, no selection (reported when there
    /// was one), no entries, both scrolls at the start.
    pub fn clear(&mut self) -> Vec<ViewEvent> {
        let mut events = Vec::new();
        self.renaming = false;
        if self.selected > 0 {
            self.selected = 0;
            events.push(ViewEvent::Selected(0));
        }
        self.entries.clear();
        self.scroll = (0.0, 0.0);
        events
    }

    /// The dragged entry's label at the pointer less the grab: with names,
    /// a box a pitch wide and a row less 2 tall (x0, y0, x1, y1), the icon
    /// at its corner and the name's pen a unit past the icon, centred by
    /// the font's text height; icons only, just the icon.
    pub fn drag_label(&self, pointer: (i32, i32), grab: (i32, i32), font: &GuiFont) -> DragLabel {
        let (x, y) = (pointer.0 - grab.0, pointer.1 - grab.1);
        if self.style & style::ICONS_ONLY != 0 {
            return DragLabel {
                frame: None,
                icon: (x, y),
                pen: None,
            };
        }
        let pen_y = ((font.ascent + self.row + 1) as u32 >> 1) as i32 + y;
        DragLabel {
            frame: Some([x, y, x + self.pitch - 1, y + self.row - 2]),
            icon: (x, y),
            pen: Some((self.unit + x + self.icon, pen_y)),
        }
    }

    /// The pointer moves with a press held: a rename takes it (3).
    pub fn pointer_move(&self) -> ViewReply {
        if self.renaming {
            ViewReply::Code(3)
        } else {
            ViewReply::Form
        }
    }

    /// Starts renaming the selected entry when the view allows it.
    pub fn begin_rename(&mut self) {
        if self.style & style::RENAME != 0 && self.rename_from < self.selected {
            self.renaming = true;
            self.backup = self.entries[self.selected as usize - 1].name.clone();
        }
    }

    /// Ends a rename, reporting a changed name.
    pub fn end_rename(&mut self) -> Vec<ViewEvent> {
        let mut events = Vec::new();
        if let Some(e) = self.entries.get(self.selected.max(1) as usize - 1)
            && e.name != self.backup
        {
            events.push(ViewEvent::Renamed(self.selected));
        }
        self.renaming = false;
        events
    }

    /// Toggles an entry's mark the way the original's press on the
    /// selected entry does: it inverts the mark bit and sets every other
    /// flag bit (so the entry also turns greyed).
    fn flip_all(flags: u32) -> u32 {
        !(flags & entry::MARKED) | (flags & !entry::MARKED)
    }

    /// A press on the hovered entry with `flags` (the table's pointer bits).
    pub fn press(&mut self, flags: u32, x: i32, events: &mut Vec<ViewEvent>) -> ViewReply {
        use crate::table_view::pointer;
        if self.hover < -1 {
            return ViewReply::Form;
        }
        if flags & pointer::DRAG != 0 {
            if self.drag_kind != 0 && self.hover > 0 {
                let grab = if self.style & style::ICONS_ONLY == 0 {
                    (
                        ((self.icon as u32) >> 1) as i32 - self.unit,
                        ((self.row as u32) >> 1) as i32,
                    )
                } else {
                    (
                        ((self.icon as u32) >> 1) as i32,
                        ((self.icon as u32) >> 1) as i32,
                    )
                };
                return ViewReply::Drag {
                    index: self.hover - 1,
                    grab,
                    kind: self.drag_kind,
                };
            }
            return ViewReply::Code(0);
        }
        let _ = x;
        if self.hover == self.selected {
            if flags & pointer::DOUBLE == pointer::DOUBLE {
                if self.selected > 0 {
                    events.push(ViewEvent::Activated(self.selected));
                }
                return ViewReply::Code(0x41);
            }
            if self.renaming {
                return ViewReply::Code(5);
            }
            if flags & pointer::PRESS == pointer::PRESS {
                if self.is(style::CHECKS) {
                    if self.hover > 0 {
                        let e = &mut self.entries[self.hover as usize - 1];
                        e.flags = Self::flip_all(e.flags);
                    }
                    return ViewReply::Code(3);
                }
                if self.selected > 0 {
                    self.begin_rename();
                    return ViewReply::Code(0x10);
                }
            }
            return ViewReply::Code(0);
        }
        if flags & pointer::PRESS != 0 {
            if self.renaming {
                events.extend(self.end_rename());
            }
            if self.style & style::NEEDS_ENTRY == 0 || self.hover > 0 {
                if self.is(style::CHECKS) {
                    if self.hover > 0 {
                        let e = &mut self.entries[self.hover as usize - 1];
                        if e.flags & entry::DISABLED == 0 {
                            e.flags ^= entry::MARKED;
                            self.checked = self.hover;
                            events.push(ViewEvent::Selected(self.selected));
                        }
                    }
                } else {
                    self.selected = self.hover;
                    events.push(ViewEvent::Selected(self.selected));
                }
            }
            return ViewReply::Code(if self.focusable { 0x11 } else { 1 });
        }
        ViewReply::Code(0)
    }

    /// Scrolls so the selected entry's line shows: a line at or above the
    /// first in view goes one above the top; one at or past the last full
    /// line becomes the second-to-last.
    pub fn scroll_to_selection(&mut self) {
        let h = self.client.height();
        let px = round_i32(f64::from((self.content.1 - h) as f32 * self.scroll.1));
        let first = px / self.row;
        let last = h / self.row + first - 1;
        let line = ((i64::from(self.selected) - 1) / i64::from(self.per_line.max(1))) as i32;
        if line < last {
            if line <= first {
                self.set_scroll_y((line - 1) as f32 * self.step.1);
            }
        } else {
            self.set_scroll_y((line - last + first + 1) as f32 * self.step.1);
        }
    }

    fn set_scroll_y(&mut self, f: f32) {
        self.scroll.1 = f.clamp(0.0, 1.0);
        if self.content.1 <= self.client.height() {
            self.scroll.1 = 0.0;
        }
    }

    fn set_scroll_x(&mut self, f: f32) {
        self.scroll.0 = f.clamp(0.0, 1.0);
    }

    /// A command word: the activation key, or a step (in a grid up and
    /// down move a line, left and right an entry; else up and down move an
    /// entry).
    pub fn command(&mut self, word: u32) -> Vec<ViewEvent> {
        let mut events = Vec::new();
        match word {
            0x100 => self.pressed = !self.pressed,
            0x200 => self.pressed = false,
            _ if word & 3 != 0 => {
                let mut next = self.selected;
                let dir = word & 0xf000;
                if self.is(style::GRID) {
                    match dir {
                        0x4000 => next -= self.per_line,
                        0x8000 => next += self.per_line,
                        0x2000 => next += 1,
                        0x1000 => next -= 1,
                        _ => {}
                    }
                } else if dir == 0x4000 {
                    next -= 1;
                } else if dir == 0x8000 {
                    next += 1;
                }
                next = if next < 1 { 1 } else { next.min(self.count()) };
                if next != self.selected {
                    self.selected = next;
                    self.hover = next;
                    self.scroll_to_selection();
                    events.push(ViewEvent::Selected(next));
                }
            }
            _ => {}
        }
        events
    }

    /// A key (or a typed byte at `now` ms), not renaming: Enter activates,
    /// Home/End and the arrows move (Left and Right by a line of entries
    /// outside the list, Up and Down by one), F2 renames, and typed bytes
    /// search. The view then scrolls to the selection. `None` hands the
    /// key on (Escape goes to the parent form).
    pub fn key(&mut self, key: u32, typed: bool, now: u32) -> Option<Vec<ViewEvent>> {
        if self.renaming {
            return match (typed, key) {
                (false, 0x0d) => Some(self.end_rename()),
                (false, 0x1b) => {
                    let i = self.selected.max(1) as usize - 1;
                    if let Some(e) = self.entries.get_mut(i) {
                        e.name = self.backup.clone();
                    }
                    self.renaming = false;
                    Some(Vec::new())
                }
                _ => Some(Vec::new()),
            };
        }
        let mut events = Vec::new();
        let mut target = None;
        let n = self.count();
        if typed {
            if u64::from(self.typed_at) + u64::from(TYPE_AHEAD_MS) < u64::from(now) {
                self.typed.clear();
            }
            self.typed_at = now;
            let b = key as u8;
            self.typed
                .push(if b.is_ascii_uppercase() { b + 0x20 } else { b });
            let from = (self.selected - 1).max(0);
            let matches = |e: &ViewEntry| prefix_matches(&self.typed, &e.name);
            target = (from..n)
                .find(|&i| matches(&self.entries[i as usize]))
                .or_else(|| (0..from).find(|&i| matches(&self.entries[i as usize])))
                .map(|i| i + 1);
        } else {
            match key {
                0x0d => {
                    if self.selected > 0 {
                        events.push(ViewEvent::Activated(self.selected));
                    }
                }
                0x1b => return None,
                0x23 => target = Some(n),
                0x24 => target = Some(1),
                0x25 => {
                    if !self.is(style::LIST) && self.per_line < self.selected {
                        target = Some(self.selected - self.per_line);
                    }
                }
                0x26 => target = Some(self.selected - 1),
                0x27 => {
                    if !self.is(style::LIST) {
                        target = Some(self.selected + self.per_line);
                    }
                }
                0x28 => target = Some(self.selected + 1),
                0x71 => self.begin_rename(),
                _ => {}
            }
        }
        if let Some(t) = target {
            let t = t.max(1).min(n);
            if t != self.selected {
                self.selected = t;
                events.push(ViewEvent::Selected(t));
            }
            let line = ((i64::from(self.selected) - 1) / i64::from(self.per_line.max(1))) as f32;
            if !self.is(style::LIST) {
                if self.is(style::GRID) {
                    self.set_scroll_y(line * self.step.1);
                } else {
                    self.set_scroll_x(line * self.step.0);
                }
            } else {
                let first = round_i32(f64::from(self.scroll.1 / self.step.1));
                if first < self.selected - 1 {
                    if self.per_line + first < self.selected {
                        self.set_scroll_y((self.selected - self.per_line) as f32 * self.step.1);
                    }
                } else {
                    self.set_scroll_y((self.selected - 1) as f32 * self.step.1);
                }
            }
        }
        Some(events)
    }
}

/// Whether `name` starts with the lower-cased `typed`, the name's capitals
/// read as small letters.
pub fn prefix_matches(typed: &[u8], name: &[u8]) -> bool {
    name.len() >= typed.len()
        && typed.iter().zip(name).all(|(&t, &c)| {
            let c = if c.is_ascii_uppercase() { c + 0x20 } else { c };
            t == c
        })
}

/// A name cut to `width` in `font`: the ellipsis's width plus each byte's
/// but the last; at the first that does not fit, the bytes before it and
/// an ellipsis.
pub fn fit_name(name: &[u8], width: i32, font: &GuiFont) -> Vec<u8> {
    let mut w = font.advance(ELLIPSIS);
    for i in 0..name.len().saturating_sub(1) {
        w += font.advance(name[i]);
        if width < w {
            let mut out = name[..i].to_vec();
            out.push(ELLIPSIS);
            return out;
        }
    }
    name.to_vec()
}

/// A grid name cut to the bytes that fit under an icon: one fewer than the
/// icon over the font's gap, with an ellipsis; otherwise centred.
pub fn grid_name(name: &[u8], icon: i32, font: &GuiFont) -> (Vec<u8>, i32) {
    let fits = icon / font.gap - 1;
    if fits < name.len() as i32 {
        let mut out = name[..fits.max(0) as usize].to_vec();
        out.push(ELLIPSIS);
        (out, 0)
    } else {
        let spare = icon as i64 - i64::from(font.gap) * name.len() as i64;
        (name.to_vec(), ((spare as u64) >> 1) as i32)
    }
}

/// A dragged entry's label: its box, its icon's corner and its name's
/// pen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragLabel {
    pub frame: Option<[i32; 4]>,
    pub icon: (i32, i32),
    pub pen: Option<(i32, i32)>,
}

/// One entry's draw: its name's pen (x, y; `None` without a name), its
/// icon's corner, and how it is highlighted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryDraw {
    pub index: i32,
    pub name: Option<(Vec<u8>, (i32, i32))>,
    pub icon: Option<(i32, (i32, i32))>,
    /// Drawn outlined (the pointer's entry, or the one being renamed).
    pub lit: bool,
    pub selected: bool,
    pub greyed: bool,
}

impl ItemView {
    /// The grid's draw: the selected entry's box (x0, y0, x1, y1), and for
    /// every entry its cut name under the icon and its icon.
    pub fn draw_grid(&self, font: &GuiFont) -> (Option<[i32; 4]>, Vec<EntryDraw>) {
        let u = self.unit;
        let n = self.count();
        let lines = if self.per_line < 1 {
            n
        } else {
            (n + self.per_line) / self.per_line
        };
        let mut selection = None;
        let mut out = Vec::new();
        for line in 0..lines {
            for col in 0..self.per_line.max(1) {
                let i = line * self.per_line + col;
                if i >= n {
                    break;
                }
                let (x, y) = (u * 2 + col * self.pitch, u * 2 + line * self.row);
                if self.selected - 1 == i {
                    selection = Some([
                        x - 1,
                        y - 1,
                        self.pitch + x - u * 2 + 1,
                        self.row + y - u * 2 + 1,
                    ]);
                }
                let e = &self.entries[i as usize];
                let name = (self.style & style::NO_NAMES == 0).then(|| {
                    let (text, shift) = grid_name(&e.name, self.icon, font);
                    let pen = (
                        u * 3 + col * self.pitch + shift,
                        u * 4 + self.icon + font.line_height + line * self.row,
                    );
                    (text, pen)
                });
                out.push(EntryDraw {
                    index: i,
                    name,
                    icon: Some((e.icon, (u * 3 + col * self.pitch, u * 3 + line * self.row))),
                    lit: self.hover - 1 == i,
                    selected: self.selected - 1 == i,
                    greyed: false,
                });
            }
        }
        (selection, out)
    }

    /// The selected entry's box in a list or columns (x0, y0, x1, y1): to
    /// the end of its name (capped at its pitch and two pixels), or the
    /// whole row.
    pub fn list_selection(&self, font: &GuiFont) -> Option<[i32; 4]> {
        let u = self.unit;
        let i = self.selected - 1;
        if i < 0 || i >= self.count() {
            return None;
        }
        let per = if self.is(style::LIST) {
            self.count()
        } else {
            self.per_line.max(1)
        };
        let (col, r) = (i / per, i % per);
        let (x, y) = (u + col * self.pitch, u + r * self.row);
        let x1 = if self.is(style::FULL_ROW) {
            self.pitch + x + 1
        } else {
            let w = u * 2 + self.icon + x + font.text_width(&self.entries[i as usize].name);
            w.min(x + self.pitch + 2)
        };
        Some([x - 1, y - 1, x1, y + self.row - 1])
    }

    /// The names and icons of the entries in view in a list or columns.
    pub fn draw_list(&self, font: &GuiFont) -> Vec<EntryDraw> {
        let u = self.unit;
        let list = self.is(style::LIST);
        let (columns, rows, y_px, x_px) = if list {
            let h = self.client.height();
            let px = round_i32(f64::from((self.content.1 - h) as f32 * self.scroll.1));
            ((0, 0), self.rows_in_view(), px, 0)
        } else {
            let w = self.client.width();
            let px = round_i32(f64::from((self.content.0 - w) as f32 * self.scroll.0));
            (self.columns_in_view(), (0, 0), 0, px)
        };
        let names = self.style & style::ICONS_ONLY == 0;
        let mut out = Vec::new();
        let mut pen_x = if self.style & style::NO_ICON_SPACE == 0 {
            x_px + u + self.icon
        } else {
            u * 2 + x_px
        };
        let room = self.pitch - pen_x + x_px + (u >> 1);
        let mut icon_x = x_px + u;
        for column in columns.0..=columns.1 {
            let (from, to) = if list {
                rows
            } else {
                let from = column * self.per_line;
                (from, (from + self.per_line - 1).min(self.count() - 1))
            };
            let mut pen_y = ((font.ascent + self.row + 2) as u32 >> 1) as i32 + y_px + u;
            let mut icon_y = u + y_px + (u >> 2) + 1;
            for i in from..=to {
                let Some(e) = self.entries.get(i as usize) else {
                    break;
                };
                let lit = self.hover - 1 == i || (self.renaming && self.selected - 1 == i);
                out.push(EntryDraw {
                    index: i,
                    name: names.then(|| (fit_name(&e.name, room, font), (pen_x, pen_y))),
                    icon: Some((e.icon, (icon_x, icon_y))),
                    lit,
                    selected: self.selected - 1 == i,
                    greyed: e.flags & entry::DISABLED != 0,
                });
                pen_y += self.row;
                icon_y += self.row;
            }
            pen_x += self.pitch;
            icon_x += self.pitch;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::RAYFORM_10;
    use crate::table_view::pointer;

    fn client(w: i32, h: i32) -> ClientRect {
        ClientRect {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        }
    }

    fn view(style: u32, n: usize) -> ItemView {
        let mut v = ItemView::new(5, style, client(200, 120));
        for i in 0..n {
            v.entries.push(ViewEntry {
                name: format!("Item{i}").into_bytes(),
                icon: 0,
                flags: 0,
                item: i as u32,
            });
        }
        v.rescale(5, &RAYFORM_10);
        v
    }

    #[test]
    fn grid_metrics() {
        let v = view(style::GRID, 7);
        // Icons of 4 units, a pitch of 4 more, rows of 6 more and a line.
        assert_eq!((v.icon, v.pitch, v.row), (20, 40, 70));
        // (200 − 10 + 2) / 40 = 4 a line; (7 + 4) / 4 = 2 lines.
        assert_eq!(v.per_line, 4);
        assert_eq!((v.content.1, v.step.1), (190, 1.0));
        assert_eq!(view(style::GRID | style::NO_NAMES, 1).row, 40);
    }

    #[test]
    fn list_and_column_metrics() {
        let l = view(style::LIST, 5);
        assert_eq!((l.icon, l.row, l.pitch, l.per_line), (20, 25, 190, 4));
        assert_eq!((l.content.1, l.step.1), (220, 0.25));
        let c = view(0, 9);
        assert_eq!((c.pitch, c.per_line), (30, 4));
        // (9 + 4) / 4 = 3 columns.
        assert_eq!((c.content.0, c.step.0), (260, 0.5));
    }

    #[test]
    fn hits_by_layout() {
        let g = view(style::GRID, 7);
        assert_eq!(
            (
                g.entry_at(10, 10, &RAYFORM_10),
                g.entry_at(45, 75, &RAYFORM_10)
            ),
            (1, 6)
        );
        assert_eq!(g.entry_at(170, 10, &RAYFORM_10), -1);
        assert_eq!(g.entry_at(130, 75, &RAYFORM_10), -1);
        let l = view(style::LIST, 5);
        assert_eq!(
            (
                l.entry_at(0, 5, &RAYFORM_10),
                l.entry_at(0, 30, &RAYFORM_10)
            ),
            (1, 2)
        );
        assert_eq!(l.entry_at(0, 200, &RAYFORM_10), -1);
        let c = view(0, 9);
        // Column 1, row 2: entry 7; within the icon and five bytes of name.
        assert_eq!(c.entry_at(40, 55, &RAYFORM_10), 7);
        assert_eq!(c.entry_at(40 + 100, 55, &RAYFORM_10), -1);
        let mut f = view(style::FULL_ROW, 9);
        f.realign();
        assert_eq!(f.entry_at(64, 55, &RAYFORM_10), 7);
    }

    #[test]
    fn icons_render_one_missing_at_a_time() {
        let mut g = view(style::GRID, 7);
        assert_eq!(g.next_icon(), Some(0));
        g.entries[0].icon = 3;
        g.entries[1].icon = 4;
        assert_eq!(g.next_icon(), Some(2));
        for e in &mut g.entries {
            e.icon = 1;
        }
        assert_eq!(g.next_icon(), None);
    }

    #[test]
    fn presses_select_rename_and_activate() {
        let mut v = view(style::LIST | style::RENAME, 5);
        let mut ev = Vec::new();
        v.hover = 2;
        assert_eq!(v.press(pointer::PRESS, 0, &mut ev), ViewReply::Code(0x11));
        assert_eq!((v.selected, ev.clone()), (2, vec![ViewEvent::Selected(2)]));
        assert_eq!(v.press(pointer::PRESS, 0, &mut ev), ViewReply::Code(0x10));
        assert!(v.renaming);
        v.entries[1].name = b"Blade".to_vec();
        assert_eq!(v.key(0x0d, false, 0), Some(vec![ViewEvent::Renamed(2)]));
        assert_eq!(v.press(pointer::DOUBLE, 0, &mut ev), ViewReply::Code(0x41));
        assert_eq!(ev.last(), Some(&ViewEvent::Activated(2)));
        // Escape while renaming puts the old name back.
        v.begin_rename();
        v.entries[1].name = b"X".to_vec();
        v.key(0x1b, false, 0);
        assert_eq!(v.entries[1].name, b"Blade");
    }

    #[test]
    fn check_presses_toggle_marks() {
        let mut v = view(style::LIST | style::CHECKS, 3);
        let mut ev = Vec::new();
        v.hover = 2;
        v.press(pointer::PRESS, 0, &mut ev);
        assert_eq!(
            (v.entries[1].flags, v.checked, v.selected),
            (entry::MARKED, 2, 0)
        );
        v.entries[2].flags = entry::DISABLED;
        v.hover = 3;
        v.press(pointer::PRESS, 0, &mut ev);
        assert_eq!(v.entries[2].flags, entry::DISABLED);
        // On the selected entry the original sets every other bit too.
        v.selected = 1;
        v.hover = 1;
        assert_eq!(v.press(pointer::PRESS, 0, &mut ev), ViewReply::Code(3));
        assert_eq!(v.entries[0].flags, u32::MAX);
        assert_eq!(ItemView::flip_all(entry::MARKED), u32::MAX - 1);
    }

    #[test]
    fn drags_grab_by_layout() {
        let mut v = view(style::LIST, 3);
        v.hover = 2;
        let mut ev = Vec::new();
        assert_eq!(v.press(pointer::DRAG, 0, &mut ev), ViewReply::Code(0));
        v.drag_kind = 4;
        assert_eq!(
            v.press(pointer::DRAG, 0, &mut ev),
            ViewReply::Drag {
                index: 1,
                grab: (5, 12),
                kind: 4
            }
        );
        v.style |= style::ICONS_ONLY;
        assert_eq!(
            v.press(pointer::DRAG, 0, &mut ev),
            ViewReply::Drag {
                index: 1,
                grab: (10, 10),
                kind: 4
            }
        );
    }

    #[test]
    fn keys_move_and_type_ahead() {
        let mut v = view(0, 9);
        v.selected = 1;
        assert_eq!(v.key(0x27, false, 0), Some(vec![ViewEvent::Selected(5)]));
        assert_eq!(v.key(0x26, false, 0), Some(vec![ViewEvent::Selected(4)]));
        v.key(0x23, false, 0);
        assert_eq!(v.selected, 9);
        assert_eq!(v.key(0x1b, false, 0), None);
        v.entries[2].name = b"Sword".to_vec();
        v.entries[6].name = b"shield".to_vec();
        v.selected = 1;
        // "s" finds Sword, "sh" (within 600 ms) finds shield.
        v.key(u32::from(b'S'), true, 1000);
        assert_eq!(v.selected, 3);
        v.key(u32::from(b'h'), true, 1500);
        assert_eq!(v.selected, 7);
        // Later, "s" starts over and wraps to Sword from the selection.
        v.key(u32::from(b's'), true, 9000);
        assert_eq!(v.selected, 7);
        // Past the last entry the search wraps to the first: Sword.
        v.selected = 8;
        v.key(u32::from(b's'), true, 9700);
        assert_eq!(v.selected, 3);
        // Within 600 ms the letters add up: "ss" matches nothing.
        v.key(u32::from(b's'), true, 9800);
        assert_eq!(v.selected, 3);
    }

    #[test]
    fn grid_commands_move_by_lines() {
        let mut g = view(style::GRID, 7);
        g.selected = 2;
        assert_eq!(g.command(0x8002), vec![ViewEvent::Selected(6)]);
        assert_eq!(g.command(0x1002), vec![ViewEvent::Selected(5)]);
        assert_eq!(g.command(0x4002), vec![ViewEvent::Selected(1)]);
        assert!(g.command(0x1001).is_empty());
    }

    #[test]
    fn clear_and_the_drag_label() {
        let mut v = view(style::LIST, 3);
        v.selected = 2;
        v.scroll = (0.5, 0.5);
        assert_eq!(v.clear(), vec![ViewEvent::Selected(0)]);
        assert_eq!((v.entries.len(), v.scroll), (0, (0.0, 0.0)));
        let v = view(style::LIST, 3);
        let label = v.drag_label((100, 100), (5, 12), &RAYFORM_10);
        assert_eq!(label.frame, Some([95, 88, 95 + 190 - 1, 88 + 23]));
        assert_eq!(label.icon, (95, 88));
        assert_eq!(label.pen, Some((5 + 95 + 20, 88 + (17 + 25 + 1) / 2)));
        assert_eq!(v.pointer_move(), ViewReply::Form);
    }

    #[test]
    fn names_are_cut_with_an_ellipsis() {
        let w = RAYFORM_10.advance(ELLIPSIS);
        assert_eq!(fit_name(b"abc", 1000, &RAYFORM_10), b"abc");
        // The last byte is not counted.
        let ab = RAYFORM_10.text_width(b"ab");
        assert_eq!(fit_name(b"abc", w + ab, &RAYFORM_10), b"abc");
        assert_eq!(fit_name(b"abc", w + ab - 1, &RAYFORM_10), b"a\x85");
        // Under a 20-pixel icon at gap 10: one byte fits.
        assert_eq!(grid_name(b"abc", 20, &RAYFORM_10), (b"a\x85".to_vec(), 0));
        assert_eq!(grid_name(b"a", 20, &RAYFORM_10), (b"a".to_vec(), 5));
    }

    #[test]
    fn the_grid_and_list_draws() {
        let mut g = view(style::GRID, 5);
        g.selected = 2;
        g.hover = 5;
        let (sel, entries) = g.draw_grid(&RAYFORM_10);
        assert_eq!(sel, Some([49, 9, 81, 71]));
        assert_eq!(entries[4].icon.map(|i| i.1), Some((15, 85)));
        assert!(entries[4].lit);
        assert_eq!(entries[0].name.as_ref().map(|n| n.1), Some((15, 60)));
        let mut l = view(style::LIST, 3);
        l.selected = 2;
        let s = l.list_selection(&RAYFORM_10).unwrap();
        assert_eq!(s[..2], [4, 29]);
        let d = l.draw_list(&RAYFORM_10);
        assert_eq!(d.len(), 3);
        assert_eq!(d[1].icon.map(|i| i.1), Some((5, 32)));
        assert_eq!(
            d[0].name.as_ref().map(|n| n.1),
            Some((25, 5 + (17 + 25 + 2) / 2))
        );
    }
}
