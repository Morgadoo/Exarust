//! The text box (`TTextBox`): a multi-line edit field on a form. It wraps
//! its text at its width less its margins (`text_layout`), counts its
//! lines and scrolls by whole lines, keeps a caret and a selection anchor,
//! and edits on keys and typed bytes: line breaks are `\r\n` and move and
//! delete as one, Home/End go to the wrapped line's ends, Up/Down/Page keep
//! the pen offset, Ctrl+Home/End jump to the text's ends, Ctrl+C/X/V use
//! the clipboard, Shift keeps the anchor. A drag selects and scrolls at
//! most every 180 ms; the scroll eases a fifth of the way per frame.
//! Rules: `docs/UI.md` (Text boxes).

use crate::form::ClientRect;
use crate::gui_font::{GuiFont, round_i32};
use crate::text_layout::{
    CaretQuad, LineView, Pictures, TextDraw, TextFrame, TextInks, caret_at, caret_quad, draw_text,
    expand_bindings, line_home_end, move_lines, text_height,
};

/// Backspace.
pub const KEY_BACK: u32 = 0x08;
/// Tab.
pub const KEY_TAB: u32 = 0x09;
/// Enter.
pub const KEY_RETURN: u32 = 0x0d;
/// Page Up.
pub const KEY_PRIOR: u32 = 0x21;
/// Page Down.
pub const KEY_NEXT: u32 = 0x22;
/// End.
pub const KEY_END: u32 = 0x23;
/// Home.
pub const KEY_HOME: u32 = 0x24;
/// Left.
pub const KEY_LEFT: u32 = 0x25;
/// Up.
pub const KEY_UP: u32 = 0x26;
/// Right.
pub const KEY_RIGHT: u32 = 0x27;
/// Down.
pub const KEY_DOWN: u32 = 0x28;
/// Delete.
pub const KEY_DELETE: u32 = 0x2e;
/// C (copy with Ctrl).
pub const KEY_C: u32 = 0x43;
/// V (paste with Ctrl).
pub const KEY_V: u32 = 0x56;
/// X (cut with Ctrl).
pub const KEY_X: u32 = 0x58;

/// A drag scrolls at most this often, in milliseconds.
pub const DRAG_SCROLL_MS: u32 = 180;
/// The scroll's ease: the share of the way it moves per frame.
pub const SCROLL_EASE: f32 = 0.2;
/// The scroll snaps to its target when this close.
pub const SCROLL_SNAP: f32 = 0.001;
/// An auto-width box wraps at this many units.
pub const AUTO_WIDTH_UNITS: i32 = 200;

/// The keyboard's modifiers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
}

/// The system clipboard as the box uses it.
pub trait Clipboard {
    /// Puts text on the clipboard.
    fn set(&mut self, text: &[u8]);
    /// The clipboard's text, when it has any.
    fn get(&mut self) -> Option<Vec<u8>>;
}

/// What an edit or a layout told the box's owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextBoxEvent {
    /// The text changed (or a key that may change it was handled).
    Changed,
    /// Tab was pressed (in a box that takes it).
    Tab,
    /// The box took the focus while nothing on its form had it.
    Focused,
    /// An auto-growing box changed its height.
    Resized(i32),
}

/// What a pointer event did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerReply {
    /// The form's own handling (frame, scroll bar) takes it.
    Form,
    /// A wheel event, passed to the parent form.
    Parent,
    /// Handled, with the original's result code.
    Code(u32),
}

/// The pointer flags the box reads.
pub mod pointer {
    /// A wheel event.
    pub const WHEEL: u32 = 0x2;
    /// A drag begins (the press has already placed the caret).
    pub const DRAG: u32 = 0x4000;
    /// A press that also asks for the code that opens a menu.
    pub const MENU: u32 = 0x2000;
}

/// The text, with the storage the original's string keeps: a shrink
/// leaves the old bytes past the end, and a grow takes them back (fresh
/// bytes are zero). This matters only where the original grows a string
/// by a length taken before a cut (Enter over a selection). Exarust's
/// guess is that the allocator resizes in place.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextBuffer {
    bytes: Vec<u8>,
    len: usize,
}

impl TextBuffer {
    pub fn from_bytes(text: &[u8]) -> Self {
        let mut bytes = text.to_vec();
        bytes.push(0);
        Self {
            bytes,
            len: text.len(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    pub fn len(&self) -> i32 {
        self.len as i32
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Sets the length, keeping the storage and writing the terminator.
    fn set_len(&mut self, len: i32) {
        let len = len.max(0) as usize;
        if self.bytes.len() < len + 1 {
            self.bytes.resize(len + 1, 0);
        }
        self.len = len;
        self.bytes[len] = 0;
    }

    /// The byte at 1-based `i` of the storage.
    fn get(&self, i: i32) -> u8 {
        if i < 1 {
            return 0;
        }
        self.bytes.get(i as usize - 1).copied().unwrap_or(0)
    }

    /// Writes the byte at 1-based `i` (nothing before the start).
    fn put(&mut self, i: i32, b: u8) {
        if i < 1 {
            return;
        }
        let i = i as usize - 1;
        if i >= self.bytes.len() {
            self.bytes.resize(i + 1, 0);
        }
        self.bytes[i] = b;
    }
}

/// A text box's state.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBox {
    pub text: TextBuffer,
    /// The selection's other end.
    pub anchor: i32,
    pub caret: i32,
    /// The wrapped line count.
    pub lines: i32,
    /// The line pitch: the spacing in units plus the font's line height.
    pub pitch: i32,
    /// Extra line spacing, in units.
    pub spacing: i32,
    /// The margin either side of the text, in pixels.
    pub margin: i32,
    /// The width the text wraps at.
    pub wrap_width: i32,
    /// The content height the scroll runs over.
    pub content_height: i32,
    /// The client rectangle the form gives it.
    pub client: ClientRect,
    /// The form's height, which an auto-growing box sets.
    pub height: i32,
    /// The scroll fraction drawn.
    pub scroll: f32,
    /// The scroll fraction it eases to.
    pub scroll_target: f32,
    /// One line's share of the scroll.
    pub scroll_step: f32,
    /// The scroll snaps to whole lines.
    pub snap: bool,
    /// The box takes no input.
    pub read_only: bool,
    /// The box uses the small font.
    pub small_font: bool,
    /// The box grows to fit its text.
    pub auto_grow: bool,
    /// The box wraps at 200 units rather than its width.
    pub auto_width: bool,
    /// The text's inline markup is read.
    pub markup: bool,
    /// The box has the focus (the caret is drawn).
    pub focused: bool,
    /// The focused child's id (0 for none), as the form keeps it.
    pub focus_id: u32,
    /// The owner handles Tab; without it Tab does nothing.
    pub on_tab: bool,
    /// A drag is selecting.
    pub dragging: bool,
    /// The earliest time a drag scrolls again.
    pub drag_scroll_at: u32,
    pub unit: i32,
    pub font: &'static GuiFont,
    pub main_font: &'static GuiFont,
    pub small: &'static GuiFont,
}

/// A text box's picture lookup: none.
fn no_pictures(_: &[u8]) -> Option<(u16, u16)> {
    None
}

impl TextBox {
    /// A new, empty box at `unit` with the level's fonts: it starts with
    /// `Rayform08R` until it is laid out, and snaps its scroll to lines.
    pub fn new(
        unit: i32,
        main_font: &'static GuiFont,
        small: &'static GuiFont,
        client: ClientRect,
    ) -> Self {
        Self {
            text: TextBuffer::from_bytes(b""),
            anchor: 0,
            caret: 0,
            lines: 0,
            pitch: 0,
            spacing: 0,
            margin: 0,
            wrap_width: 0,
            content_height: 0,
            client,
            height: client.bottom,
            scroll: 0.0,
            scroll_target: 0.0,
            scroll_step: 0.0,
            snap: true,
            read_only: false,
            small_font: false,
            auto_grow: false,
            auto_width: false,
            markup: false,
            focused: false,
            focus_id: 0,
            on_tab: false,
            dragging: false,
            drag_scroll_at: 0,
            unit,
            font: &crate::gui_font::RAYFORM_08,
            main_font,
            small,
        }
    }

    /// The client height the text shows in.
    pub fn view_height(&self) -> i32 {
        self.client.height()
    }

    /// The lines that fit, uncapped.
    pub fn fitting_lines(&self) -> i32 {
        if self.pitch == 0 {
            return 0;
        }
        self.view_height() / self.pitch
    }

    /// The text's left edge: a unit and the margin.
    pub fn text_x(&self) -> i32 {
        self.margin + self.unit
    }

    /// Replaces the text (expanding `[inptN]` key names when `bindings`
    /// is given), puts the caret and anchor at the start, lays out and
    /// scrolls to the top.
    pub fn set_text(
        &mut self,
        text: &[u8],
        bindings: Option<&dyn Fn(usize) -> Vec<u8>>,
    ) -> Vec<TextBoxEvent> {
        let text = match bindings {
            Some(name) => expand_bindings(text, name),
            None => text.to_vec(),
        };
        self.text = TextBuffer::from_bytes(&text);
        self.anchor = 0;
        self.caret = 0;
        let events = self.relayout(true);
        self.set_scroll(0.0);
        events
    }

    /// Counts the lines again. When the count changes the content height
    /// follows (the view plus the lines that do not fit), an auto-growing
    /// box takes that height (plus its top and left insets), and, with
    /// `realign`, the box lays itself out again.
    pub fn relayout(&mut self, realign: bool) -> Vec<TextBoxEvent> {
        let mut events = Vec::new();
        self.pitch = self.unit * self.spacing + self.font.line_height;
        let measured = text_height(
            self.text.as_bytes(),
            self.wrap_width,
            self.pitch,
            self.font,
            self.markup,
            self.unit,
            &no_pictures,
        );
        let lines = round_i32(f64::from(measured) / f64::from(self.pitch)).max(1);
        if lines != self.lines {
            self.lines = lines;
            let visible = (self.view_height() / self.pitch).min(self.lines);
            self.content_height = self.view_height() + (self.lines - visible) * self.pitch;
            if self.auto_grow {
                let old = self.height;
                self.height = self.content_height + self.client.top + self.client.left;
                if old != self.height {
                    events.push(TextBoxEvent::Resized(self.height));
                }
            }
            if realign {
                events.extend(self.realign());
            }
        }
        events
    }

    /// Lays the box out for its client rectangle: the wrap width (the
    /// client width less a unit and the margin each side, or 200 units),
    /// the font, the lines, the content height and the scroll step (one
    /// line of those that do not fit; none when all fit, which scrolls to
    /// the top).
    pub fn realign(&mut self) -> Vec<TextBoxEvent> {
        if self.auto_width {
            self.wrap_width = AUTO_WIDTH_UNITS * self.unit;
        } else {
            self.wrap_width = self.client.width() - (self.unit + self.margin) * 2;
        }
        self.font = if self.small_font {
            self.small
        } else {
            self.main_font
        };
        let events = self.relayout(false);
        let visible = (self.view_height() / self.pitch).min(self.lines);
        self.content_height = (self.lines - visible) * self.pitch + self.view_height();
        if visible < self.lines {
            self.scroll_step = (1.0 / f64::from(self.lines - visible)) as f32;
        } else {
            self.scroll_step = 0.0;
            self.set_scroll(0.0);
        }
        events
    }

    /// The form's scroll setter: clamped to 0..1, snapped to whole steps,
    /// and 0 when the content fits.
    fn form_scroll(&mut self, f: f32) {
        let f = f.clamp(0.0, 1.0);
        if f == self.scroll {
            return;
        }
        self.scroll = if self.snap {
            let q = f / self.scroll_step;
            let lines = if q.is_finite() && q.abs() < 9.2e18 {
                q.round_ties_even()
            } else {
                i64::MIN as f32
            };
            lines * self.scroll_step
        } else {
            f
        };
        if self.content_height <= self.view_height() {
            self.scroll = 0.0;
        }
    }

    /// Scrolls at once: the drawn and the target fraction both.
    pub fn set_scroll(&mut self, f: f32) {
        self.scroll = self.scroll_target;
        self.form_scroll(f);
        self.scroll_target = self.scroll;
    }

    /// Scrolls the target by `delta`, leaving the drawn fraction to ease
    /// after it (the wheel: one step up or down).
    pub fn scroll_by(&mut self, delta: f32) {
        let drawn = self.scroll;
        self.set_scroll(self.scroll_target + delta);
        self.scroll = drawn;
    }

    /// One frame of the scroll's ease; true while it is still moving.
    pub fn ease_scroll(&mut self) -> bool {
        if (self.scroll_target - self.scroll).abs() <= SCROLL_SNAP {
            self.scroll = self.scroll_target;
            false
        } else {
            self.scroll = self.scroll_target * SCROLL_EASE + self.scroll * (1.0 - SCROLL_EASE);
            true
        }
    }

    /// Keeps the margin's share of the unit when the unit changes from
    /// `old_unit`.
    pub fn rescale(&mut self, old_unit: i32, unit: i32) {
        if old_unit != 0 {
            self.margin = self.margin / old_unit * unit;
        }
        self.unit = unit;
    }

    fn view(&self) -> LineView {
        LineView {
            lines: self.lines,
            visible: self.fitting_lines(),
            scroll: self.scroll,
            step: self.scroll_step,
        }
    }

    /// Removes the selection, leaving the caret and anchor at its start.
    fn cut_selection(&mut self) {
        let len = self.text.len();
        let (lo, hi) = if self.anchor < self.caret {
            (self.anchor, self.caret)
        } else {
            (self.caret, self.anchor)
        };
        let span = hi - lo;
        for i in hi + 1..=len {
            let b = self.text.get(i);
            self.text.put(i - span, b);
        }
        self.text.set_len(len - span);
        self.caret = lo;
        self.anchor = lo;
    }

    /// The selection's bytes.
    fn selected(&self) -> Vec<u8> {
        let (lo, hi) = if self.anchor < self.caret {
            (self.anchor, self.caret)
        } else {
            (self.caret, self.anchor)
        };
        (lo + 1..=hi).map(|i| self.text.get(i)).collect()
    }

    /// Inserts `bytes` at the caret (cutting a selection first) and moves
    /// the caret and anchor past them.
    fn paste_bytes(&mut self, bytes: &[u8]) {
        if self.caret != self.anchor {
            self.cut_selection();
        }
        let n = bytes.len() as i32;
        let total = self.text.len() + n;
        self.text.set_len(total);
        let mut i = total;
        while self.caret + 2 <= i {
            let b = self.text.get(i - n);
            self.text.put(i, b);
            i -= 1;
        }
        for (k, &b) in bytes.iter().enumerate() {
            self.text.put(self.caret + 1 + k as i32, b);
        }
        self.caret += n;
        self.anchor = self.caret;
    }

    /// A key, or (`typed`) a byte typed: edits, caret moves and clipboard
    /// keys as the original's text box. A read-only box takes none.
    pub fn key(
        &mut self,
        key: u32,
        typed: bool,
        mods: Modifiers,
        clipboard: &mut dyn Clipboard,
    ) -> Vec<TextBoxEvent> {
        let mut events = Vec::new();
        if self.read_only {
            return events;
        }
        if mods.ctrl && !typed {
            match key {
                KEY_END => {
                    self.caret = self.text.len();
                    if !mods.shift {
                        self.anchor = self.caret;
                    }
                    self.set_scroll(1.0);
                }
                KEY_HOME => {
                    self.caret = 0;
                    if !mods.shift {
                        self.anchor = self.caret;
                    }
                    self.set_scroll(0.0);
                }
                KEY_C => clipboard.set(&self.selected()),
                KEY_V => {
                    if let Some(bytes) = clipboard.get() {
                        self.paste_bytes(&bytes);
                    }
                    events.push(TextBoxEvent::Changed);
                }
                KEY_X => {
                    clipboard.set(&self.selected());
                    self.cut_selection();
                    self.anchor = self.caret;
                    events.push(TextBoxEvent::Changed);
                }
                _ => {}
            }
            events.extend(self.relayout(true));
            return events;
        }
        let len = self.text.len();
        if typed {
            if self.caret != self.anchor {
                self.cut_selection();
            }
            let total = self.text.len() + 1;
            self.text.set_len(total);
            let mut i = total;
            while self.caret + 2 <= i {
                let b = self.text.get(i - 1);
                self.text.put(i, b);
                i -= 1;
            }
            self.text.put(self.caret + 1, key as u8);
            self.caret += 1;
            self.anchor = self.caret;
        } else {
            let fitting = self.fitting_lines();
            let width = self.wrap_width;
            match key {
                KEY_BACK => {
                    if self.caret != self.anchor {
                        self.cut_selection();
                    } else if self.caret > 0 {
                        let len = self.text.len();
                        if self.text.get(self.caret) == b'\n' {
                            let last = len - 2;
                            if self.caret <= last {
                                for i in (self.caret - 1).max(1)..=last {
                                    let b = self.text.get(i + 2);
                                    self.text.put(i, b);
                                }
                            }
                            self.caret = (self.caret - 2).max(0);
                            self.text.set_len(last);
                        } else {
                            let last = len - 1;
                            for i in self.caret..=last {
                                let b = self.text.get(i + 1);
                                self.text.put(i, b);
                            }
                            self.caret -= 1;
                            self.text.set_len(last);
                        }
                    }
                    self.anchor = self.caret;
                }
                KEY_TAB => {
                    if !self.on_tab {
                        return events;
                    }
                    events.push(TextBoxEvent::Tab);
                }
                KEY_RETURN => {
                    if self.caret != self.anchor {
                        self.cut_selection();
                    }
                    let total = len + 2;
                    self.text.set_len(total);
                    let mut i = total;
                    while self.caret + 2 <= i {
                        let b = self.text.get(i - 2);
                        self.text.put(i, b);
                        i -= 1;
                    }
                    self.text.put(self.caret + 1, b'\r');
                    self.text.put(self.caret + 2, b'\n');
                    self.caret += 2;
                    self.anchor = self.caret;
                }
                KEY_PRIOR | KEY_NEXT | KEY_UP | KEY_DOWN => {
                    let delta = match key {
                        KEY_PRIOR => -(fitting - 2),
                        KEY_NEXT => fitting - 2,
                        KEY_UP => -1,
                        _ => 1,
                    };
                    let moved = move_lines(
                        self.text.as_bytes(),
                        width,
                        self.caret,
                        delta,
                        self.view(),
                        self.font,
                    );
                    if let Some(f) = moved.scroll {
                        self.set_scroll(f);
                    }
                    self.caret = moved.caret;
                }
                KEY_END | KEY_HOME => {
                    self.caret = line_home_end(
                        self.text.as_bytes(),
                        width,
                        self.caret,
                        key == KEY_END,
                        self.font,
                    );
                }
                KEY_LEFT => {
                    if self.caret > 0 {
                        self.caret -= 1;
                        if self.text.get(self.caret + 1) == b'\n' {
                            self.caret -= 1;
                        }
                    }
                }
                KEY_RIGHT => {
                    if self.caret < len {
                        self.caret += 1;
                        if self.text.get(self.caret + 1) == b'\r' {
                            self.caret += 1;
                        }
                    }
                }
                KEY_DELETE => {
                    if self.caret != self.anchor {
                        self.cut_selection();
                    } else {
                        if len <= self.caret {
                            self.focused = true;
                            return events;
                        }
                        if self.text.get(self.caret + 2) == b'\n' {
                            let last = len - 2;
                            for i in self.caret + 1..=last {
                                let b = self.text.get(i + 2);
                                self.text.put(i, b);
                            }
                            self.text.set_len(last);
                        } else {
                            let last = len - 1;
                            for i in self.caret + 1..=last {
                                let b = self.text.get(i + 1);
                                self.text.put(i, b);
                            }
                            self.text.set_len(last);
                        }
                    }
                    self.anchor = self.caret;
                }
                _ => return events,
            }
        }
        if !mods.shift {
            self.anchor = self.caret;
        }
        events.push(TextBoxEvent::Changed);
        self.focused = true;
        events.extend(self.relayout(true));
        events
    }

    /// Focuses the child `id` (0 for none): the box reports `Focused` when
    /// it gains the focus with nothing focused before, and draws its caret
    /// while focused.
    pub fn set_focus(&mut self, id: u32) -> Vec<TextBoxEvent> {
        let mut events = Vec::new();
        if id != 0 && self.focus_id == 0 {
            events.push(TextBoxEvent::Focused);
        }
        if id != self.focus_id {
            self.focused = id != 0;
            self.focus_id = id;
        }
        events
    }

    /// The caret under the pointer at `x`, `y` in the box's scrolled client
    /// coordinates.
    fn caret_under(&self, x: i32, y: i32) -> i32 {
        caret_at(
            self.text.as_bytes(),
            x - self.unit - self.margin,
            y,
            self.wrap_width,
            self.font,
        )
    }

    /// A press at `x`, `y` (scrolled client coordinates) with `flags`:
    /// a wheel goes to the parent, a press on the form's frame to the
    /// form; a read-only box answers 0x10; a drag begins (0x15); else the
    /// caret moves under the pointer (the anchor too, without Shift) and
    /// the box takes the focus.
    pub fn press(
        &mut self,
        x: i32,
        y: i32,
        flags: u32,
        on_frame: bool,
        mods: Modifiers,
    ) -> PointerReply {
        if flags & pointer::WHEEL != 0 {
            return PointerReply::Parent;
        }
        if on_frame {
            return PointerReply::Form;
        }
        if self.read_only {
            return PointerReply::Code(0x10);
        }
        if flags & pointer::DRAG != 0 {
            self.dragging = true;
            return PointerReply::Code(0x15);
        }
        self.caret = self.caret_under(x, y);
        if !mods.shift {
            self.anchor = self.caret;
        }
        self.focused = true;
        PointerReply::Code(if flags & pointer::MENU != 0 { 0x30 } else { 0 })
    }

    /// The pointer moves to `x`, `y` at `now` (milliseconds): while
    /// dragging the caret follows it, and at most every 180 ms the scroll
    /// brings the pointer's line into view.
    pub fn drag_to(&mut self, x: i32, y: i32, now: u32) -> PointerReply {
        if !self.dragging {
            return PointerReply::Form;
        }
        self.caret = self.caret_under(x, y);
        if self.drag_scroll_at < now {
            let first = round_i32(f64::from(self.scroll / self.scroll_step));
            let line = (f64::from(y) / f64::from(self.pitch)) as i32;
            if line < first + 1 {
                self.set_scroll(line as f32 * self.scroll_step);
            } else {
                let fitting = self.fitting_lines();
                if first + fitting - 1 <= line {
                    self.set_scroll((line - fitting + 2) as f32 * self.scroll_step);
                }
            }
            self.drag_scroll_at = now.wrapping_add(DRAG_SCROLL_MS);
        }
        self.focused = true;
        PointerReply::Code(3)
    }

    /// The press ends: the drag stops.
    pub fn release(&mut self) -> PointerReply {
        self.dragging = false;
        PointerReply::Form
    }

    /// The box's frame tick: a drag keeps following the pointer (and
    /// scrolling) while it is held still.
    pub fn tick(&mut self, x: i32, y: i32, now: u32) {
        if self.dragging {
            self.drag_to(x, y, now);
        }
    }

    fn frame(&self) -> TextFrame {
        TextFrame {
            width: self.wrap_width,
            height: self.content_height,
            pitch: self.pitch,
            markup: self.markup,
        }
    }

    /// The text's draw: glyphs, selection and pictures in content
    /// coordinates (the form scrolls them by the scroll fraction).
    pub fn draw(&self, inks: &TextInks, picture: Pictures) -> TextDraw {
        draw_text(
            self.text.as_bytes(),
            self.text_x(),
            0,
            0,
            (self.anchor, self.caret),
            &self.frame(),
            self.font,
            inks,
            self.unit,
            picture,
        )
    }

    /// The caret's quad, drawn when the box has the focus and takes input.
    pub fn caret_draw(&self, inks: &TextInks, picture: Pictures) -> Option<CaretQuad> {
        if self.read_only || !self.focused {
            return None;
        }
        caret_quad(
            self.text.as_bytes(),
            self.caret,
            self.text_x(),
            0,
            0,
            &self.frame(),
            self.font,
            inks,
            self.unit,
            picture,
        )
    }
}

/// The height a text needs at `width` with markup, as the original sizes
/// tooltips and help panes: its lines at `pitch` (0 takes the font's line
/// height) and its pictures.
pub fn fit_height(
    text: &[u8],
    width: i32,
    pitch: i32,
    font: &GuiFont,
    unit: i32,
    picture: Pictures,
) -> i32 {
    let pitch = if pitch == 0 { font.line_height } else { pitch };
    text_height(text, width, pitch, font, true, unit, picture)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::{RAYFORM_06, RAYFORM_08, RAYFORM_10};

    /// A clipboard in memory.
    #[derive(Default)]
    struct Board(Option<Vec<u8>>);

    impl Clipboard for Board {
        fn set(&mut self, text: &[u8]) {
            self.0 = Some(text.to_vec());
        }
        fn get(&mut self) -> Option<Vec<u8>> {
            self.0.clone()
        }
    }

    fn client(w: i32, h: i32) -> ClientRect {
        ClientRect {
            left: 2,
            top: 2,
            right: 2 + w,
            bottom: 2 + h,
        }
    }

    /// A box at unit 5 with Rayform10R (line height 20), 200 × 60.
    fn boxed(text: &[u8]) -> TextBox {
        let mut b = TextBox::new(5, &RAYFORM_10, &RAYFORM_08, client(200, 60));
        b.realign();
        b.set_text(text, None);
        b
    }

    fn press(b: &mut TextBox, key: u32) -> Vec<TextBoxEvent> {
        b.key(key, false, Modifiers::default(), &mut Board::default())
    }

    fn type_bytes(b: &mut TextBox, bytes: &[u8]) {
        for &c in bytes {
            b.key(
                u32::from(c),
                true,
                Modifiers::default(),
                &mut Board::default(),
            );
        }
    }

    #[test]
    fn layout_follows_the_client_and_font() {
        let b = boxed(b"hello");
        assert_eq!(b.font.texture, "Rayform10R");
        // 200 less a unit and no margin each side.
        assert_eq!(b.wrap_width, 190);
        assert_eq!(
            (b.pitch, b.lines, b.content_height, b.scroll_step),
            (20, 1, 60, 0.0)
        );
        let mut b = TextBox::new(5, &RAYFORM_10, &RAYFORM_06, client(200, 60));
        b.small_font = true;
        b.margin = 4;
        b.realign();
        assert_eq!(
            (b.font.texture, b.wrap_width, b.pitch),
            ("Rayform06R", 182, 12)
        );
    }

    #[test]
    fn overflowing_lines_give_a_scroll_step() {
        let mut b = boxed(b"");
        b.set_text(b"a\r\nb\r\nc\r\nd\r\ne", None);
        b.realign();
        // Five lines, three fit: two lines of scroll.
        assert_eq!((b.lines, b.content_height, b.scroll_step), (5, 100, 0.5));
        b.spacing = 1;
        b.realign();
        assert_eq!((b.pitch, b.lines, b.content_height), (25, 5, 135));
    }

    #[test]
    fn typing_inserts_at_the_caret() {
        let mut b = boxed(b"");
        type_bytes(&mut b, b"abd");
        press(&mut b, KEY_LEFT);
        type_bytes(&mut b, b"c");
        assert_eq!(b.text.as_bytes(), b"abcd");
        assert_eq!((b.caret, b.anchor), (3, 3));
        let events = press(&mut b, KEY_BACK);
        assert_eq!(events, vec![TextBoxEvent::Changed]);
        assert_eq!((b.text.as_bytes(), b.caret), (b"abd".as_slice(), 2));
    }

    #[test]
    fn line_breaks_move_and_delete_as_one() {
        let mut b = boxed(b"");
        type_bytes(&mut b, b"ab");
        press(&mut b, KEY_RETURN);
        type_bytes(&mut b, b"cd");
        assert_eq!(b.text.as_bytes(), b"ab\r\ncd");
        assert_eq!(b.lines, 2);
        // Left over "\r\n" from before 'c' lands before '\r'.
        b.caret = 4;
        press(&mut b, KEY_LEFT);
        assert_eq!(b.caret, 2);
        // Right from before '\r' passes it but stops before '\n', as the
        // original does; Right again passes the line feed.
        press(&mut b, KEY_RIGHT);
        assert_eq!(b.caret, 3);
        press(&mut b, KEY_RIGHT);
        assert_eq!(b.caret, 4);
        // Backspace after "\r\n" removes both.
        press(&mut b, KEY_BACK);
        assert_eq!((b.text.as_bytes(), b.caret), (b"abcd".as_slice(), 2));
        // Delete before "\r\n" removes both.
        b.set_text(b"ab\r\ncd", None);
        b.caret = 2;
        b.anchor = 2;
        press(&mut b, KEY_DELETE);
        assert_eq!((b.text.as_bytes(), b.caret), (b"abcd".as_slice(), 2));
    }

    #[test]
    fn delete_at_the_end_changes_nothing() {
        let mut b = boxed(b"ab");
        b.caret = 2;
        b.anchor = 2;
        assert!(press(&mut b, KEY_DELETE).is_empty());
        assert_eq!(b.text.as_bytes(), b"ab");
    }

    #[test]
    fn shift_keeps_the_anchor_and_typing_replaces_the_selection() {
        let mut b = boxed(b"hello world");
        let shift = Modifiers {
            shift: true,
            ctrl: false,
        };
        for _ in 0..5 {
            b.key(KEY_RIGHT, false, shift, &mut Board::default());
        }
        assert_eq!((b.anchor, b.caret), (0, 5));
        type_bytes(&mut b, b"J");
        assert_eq!(b.text.as_bytes(), b"J world");
        assert_eq!((b.anchor, b.caret), (1, 1));
    }

    #[test]
    fn enter_over_a_selection_grows_by_the_old_length() {
        // The original sizes the text from its length before the cut, so
        // the bytes past the cut come back at the end.
        let mut b = boxed(b"abcdef");
        b.anchor = 1;
        b.caret = 3;
        press(&mut b, KEY_RETURN);
        assert_eq!(b.text.len(), 8);
        assert_eq!(&b.text.as_bytes()[..6], b"a\r\ndef");
    }

    #[test]
    fn the_clipboard_keys() {
        let ctrl = Modifiers {
            shift: false,
            ctrl: true,
        };
        let mut board = Board::default();
        let mut b = boxed(b"hello world");
        b.anchor = 6;
        b.caret = 11;
        b.key(KEY_C, false, ctrl, &mut board);
        assert_eq!(board.0.as_deref(), Some(b"world".as_slice()));
        let events = b.key(KEY_X, false, ctrl, &mut board);
        assert_eq!(events, vec![TextBoxEvent::Changed]);
        assert_eq!(
            (b.text.as_bytes(), b.caret, b.anchor),
            (b"hello ".as_slice(), 6, 6)
        );
        b.caret = 0;
        b.anchor = 0;
        b.key(KEY_V, false, ctrl, &mut board);
        assert_eq!((b.text.as_bytes(), b.caret), (b"worldhello ".as_slice(), 5));
        b.key(KEY_END, false, ctrl, &mut board);
        assert_eq!((b.caret, b.anchor), (11, 11));
        b.key(KEY_HOME, false, ctrl, &mut board);
        assert_eq!(b.caret, 0);
    }

    #[test]
    fn home_end_and_the_vertical_keys() {
        let mut b = boxed(b"");
        b.set_text(b"one\r\ntwo\r\nthree\r\nfour\r\nfive", None);
        b.realign();
        b.caret = 6;
        press(&mut b, KEY_END);
        assert_eq!(b.caret, 8);
        press(&mut b, KEY_HOME);
        assert_eq!(b.caret, 5);
        press(&mut b, KEY_DOWN);
        assert_eq!(b.caret, 10);
        // Down twice more reaches line 3, the last visible one: the scroll
        // moves a line.
        // Line 2 is the last visible one: the scroll moves a line.
        assert_eq!(b.scroll, 0.5);
        press(&mut b, KEY_DOWN);
        press(&mut b, KEY_DOWN);
        assert_eq!((b.caret, b.scroll), (23, 1.0));
        // Tab does nothing unless the owner takes it.
        assert!(press(&mut b, KEY_TAB).is_empty());
        b.on_tab = true;
        let events = press(&mut b, KEY_TAB);
        assert_eq!(events[0], TextBoxEvent::Tab);
    }

    #[test]
    fn a_read_only_box_takes_nothing() {
        let mut b = boxed(b"ab");
        b.read_only = true;
        assert!(press(&mut b, KEY_BACK).is_empty());
        assert_eq!(b.text.as_bytes(), b"ab");
        assert_eq!(
            b.press(0, 0, 0, false, Modifiers::default()),
            PointerReply::Code(0x10)
        );
    }

    #[test]
    fn the_pointer_places_the_caret_and_drags() {
        let mut b = boxed(b"hello");
        // Rayform10R: h 10, e 10; the text starts at the unit (5).
        let shift = Modifiers::default();
        assert_eq!(b.press(5 + 14, 3, 0, false, shift), PointerReply::Code(0));
        assert_eq!((b.caret, b.anchor, b.focused), (1, 1, true));
        assert_eq!(
            b.press(5 + 14, 3, pointer::MENU, false, shift),
            PointerReply::Code(0x30)
        );
        assert_eq!(
            b.press(0, 0, pointer::DRAG, false, shift),
            PointerReply::Code(0x15)
        );
        assert!(b.dragging);
        assert_eq!(b.drag_to(5 + 200, 3, 1000), PointerReply::Code(3));
        assert_eq!((b.anchor, b.caret), (1, 5));
        assert_eq!(b.drag_scroll_at, 1180);
        b.release();
        assert!(!b.dragging);
        assert_eq!(
            b.press(0, 0, pointer::WHEEL, false, shift),
            PointerReply::Parent
        );
        assert_eq!(b.press(0, 0, 0, true, shift), PointerReply::Form);
    }

    #[test]
    fn the_scroll_eases_and_snaps() {
        let mut b = boxed(b"");
        b.set_text(b"a\r\nb\r\nc\r\nd\r\ne", None);
        b.realign();
        b.scroll_by(0.5);
        assert_eq!((b.scroll, b.scroll_target), (0.0, 0.5));
        assert!(b.ease_scroll());
        assert!((b.scroll - 0.1).abs() < 1e-6);
        for _ in 0..80 {
            b.ease_scroll();
        }
        assert_eq!(b.scroll, 0.5);
        // A set snaps to whole lines.
        b.set_scroll(0.7);
        assert_eq!((b.scroll, b.scroll_target), (0.5, 0.5));
        b.set_scroll(0.8);
        assert_eq!(b.scroll, 1.0);
    }

    #[test]
    fn an_auto_growing_box_reports_its_height() {
        let mut b = TextBox::new(5, &RAYFORM_10, &RAYFORM_08, client(200, 20));
        b.auto_grow = true;
        b.realign();
        let events = b.set_text(b"a\r\nb\r\nc", None);
        // Three lines of 20 over a 20-pixel view, plus the 2-pixel insets.
        assert!(events.contains(&TextBoxEvent::Resized(64)));
        assert_eq!(b.height, 64);
    }

    #[test]
    fn key_names_expand_on_set() {
        let mut b = boxed(b"");
        let names = |_: usize| b"SPACE".to_vec();
        b.set_text(b"[inpt0] jumps", Some(&names));
        assert_eq!(b.text.as_bytes(), b"Space jumps");
    }

    #[test]
    fn focus_is_reported_once() {
        let mut b = boxed(b"");
        assert_eq!(b.set_focus(1), vec![TextBoxEvent::Focused]);
        assert!(b.focused);
        assert!(b.set_focus(1).is_empty());
        assert!(b.set_focus(0).is_empty());
        assert!(!b.focused);
    }

    #[test]
    fn the_margin_keeps_its_share_of_the_unit() {
        let mut b = boxed(b"");
        b.margin = 9;
        b.rescale(4, 8);
        assert_eq!((b.margin, b.unit), (16, 8));
    }

    #[test]
    fn the_draw_and_the_caret() {
        let inks = TextInks {
            text: [255, 250, 230, 255],
            tag_alpha: 255,
            selection: [80, 80, 80, 128],
            slant: 2,
        };
        let mut b = boxed(b"ab");
        b.anchor = 0;
        b.caret = 1;
        let d = b.draw(&inks, &no_pictures);
        // Rayform10R: 'a' advances 9.
        assert_eq!(
            d.glyphs.iter().map(|g| g.x).collect::<Vec<_>>(),
            vec![5, 14]
        );
        assert_eq!(d.selections.len(), 1);
        assert_eq!(b.caret_draw(&inks, &no_pictures), None);
        b.focused = true;
        // Half the 10-pixel gap back from the pen after 'a'.
        assert_eq!(b.caret_draw(&inks, &no_pictures).map(|c| c.x), Some(9));
    }

    #[test]
    fn a_fitted_height() {
        assert_eq!(
            fit_height(b"a\r\nb", 100, 0, &RAYFORM_10, 5, &no_pictures),
            40
        );
    }
}
