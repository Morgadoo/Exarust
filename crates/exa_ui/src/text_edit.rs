//! Single-line text editing (`RF_GUI_Text_Edit`): the visible range and its
//! horizontal scroll, the caret from a pointer's x, the key handler (typing,
//! Backspace, Delete, Left/Right, Home/End, Ctrl+C/X/V), the navigation-only
//! handler, the in-place selection deletion, the OS clipboard's text, the
//! caret's draw position, the selection's runs and the markup tags
//! (`[tcol]`, `[/tcol]`, `[scrp]`, `[algn]`, `[rimg]`).
//!
//! The edit state is four bytes, as the original's: the anchor and caret
//! (byte counts, so the caret before the first byte is 0), the visible
//! window's end and its first byte. The text itself is a byte vector;
//! limits are the original's 8-bit fields and the caller's maximum length.
//! Rules: `docs/UI.md` (Text boxes).

use crate::gui_font::GuiFont;
use crate::gui_text::{TextQuad, glyph_cell};
use crate::text_layout::parse_hex_colour;
use crate::text_layout::{CARET_GLYPH, SPAN_INK};

/// The edit's four state bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EditView {
    /// The selection's other end; equal to the caret when nothing is
    /// selected.
    pub anchor: u8,
    /// The caret: the number of bytes before it.
    pub caret: u8,
    /// The visible window's end: the last visible byte, 1-based. Only
    /// [`scroll_view`] moves it; the whole-run draw reads it as a count
    /// from the first visible byte instead.
    pub length: u8,
    /// The first visible byte (horizontal scroll).
    pub first: u8,
}

/// The keys the handler knows (the original's control-code bytes).
pub mod key {
    pub const BACKSPACE: u8 = 8;
    pub const END: u8 = 0x23;
    pub const HOME: u8 = 0x24;
    pub const LEFT: u8 = 0x25;
    pub const RIGHT: u8 = 0x27;
    pub const DELETE: u8 = 0x2e;
    pub const COPY: u8 = 0x43;
    pub const PASTE: u8 = 0x56;
    pub const CUT: u8 = 0x58;
}

/// The caret from a pointer's x (counted from the first visible byte):
/// walking the bytes after the first visible one up to the window's end,
/// the caret stops before the first byte whose half advance the pointer
/// has not reached, else lands on the window's end. The half advance is an
/// unsigned shift, so a negative advance always stops the walk. The caret
/// is then clamped to the text's length.
pub fn caret_from_x(font: &GuiFont, text: &[u8], view: &mut EditView, x: i32) -> u8 {
    let len = text.len().min(u8::MAX as usize) as u8;
    let end = view.length;
    let mut index = view.first;
    let mut caret = end;
    let mut left = x;
    if index < end {
        loop {
            let next = index + 1;
            let advance = text
                .get(next as usize - 1)
                .map_or(0, |&byte| font.advance(byte));
            caret = index;
            if left < ((advance as u32) >> 1) as i32 {
                break;
            }
            left -= advance;
            index = next;
            caret = end;
            if next >= end {
                break;
            }
        }
    }
    view.caret = caret.min(len);
    view.caret
}

/// Keeps the caret in view: the visible window's end and first byte are
/// adjusted as the original's four bytes describe.
pub fn scroll_view(font: &GuiFont, text: &[u8], view: &mut EditView, width: i32) {
    let len = text.len().min(u8::MAX as usize) as u8;
    if len < view.length {
        view.length = len;
    }
    if len < view.anchor {
        view.anchor = len;
    }
    if len < view.caret {
        view.caret = len;
    }
    let mut left = width;
    if view.first < view.caret {
        if view.caret < view.length {
            view.length = len;
            let mut index = view.first;
            while index < len {
                index += 1;
                left -= advance(font, text, index);
                if left < 1 {
                    view.length = index - 1;
                    break;
                }
            }
            while view.first != 0 && advance(font, text, view.first) <= left {
                left -= advance(font, text, view.first);
                view.first -= 1;
            }
        } else {
            view.length = if view.caret < len {
                view.caret + 1
            } else {
                len
            };
            view.first = 0;
            if view.length != 0 {
                let mut index = view.length + 1;
                loop {
                    index -= 1;
                    left -= advance(font, text, index);
                    if left < 1 {
                        view.first = index;
                        return;
                    }
                    if index <= 1 {
                        return;
                    }
                }
            }
        }
    } else {
        view.first = if view.caret == 0 { 0 } else { view.caret - 1 };
        view.length = len;
        let mut index = view.first;
        while index < len {
            index += 1;
            left -= advance(font, text, index);
            if left < 1 {
                view.length = index - 1;
                return;
            }
        }
    }
}

/// The advance of 1-based byte `index` (0 past either end).
fn advance(font: &GuiFont, text: &[u8], index: u8) -> i32 {
    if index == 0 {
        return 0;
    }
    text.get(index as usize - 1)
        .map_or(0, |&byte| font.advance(byte))
}

/// The selected range's ends, lower first; `None` when the ends meet.
pub fn selection(view: &EditView) -> Option<(usize, usize)> {
    let a = view.anchor as usize;
    let b = view.caret as usize;
    match a.cmp(&b) {
        std::cmp::Ordering::Less => Some((a, b)),
        std::cmp::Ordering::Greater => Some((b, a)),
        std::cmp::Ordering::Equal => None,
    }
}

/// Removes the selected bytes in place and puts both ends at the lower one.
/// Returns the number of bytes removed. The window is left to
/// [`scroll_view`].
pub fn delete_selection(text: &mut Vec<u8>, view: &mut EditView) -> usize {
    let Some((low, high)) = selection(view) else {
        return 0;
    };
    let high = high.min(text.len());
    let low = low.min(high);
    text.drain(low..high);
    view.anchor = low as u8;
    view.caret = low as u8;
    high - low
}

/// The bytes a copy takes: the selection, or the whole text when nothing
/// is selected (the original's copy hands the whole text over then).
pub fn selected_text(text: &[u8], view: &EditView) -> Vec<u8> {
    match selection(view) {
        Some((low, high)) => text[low.min(text.len())..high.min(text.len())].to_vec(),
        None => text.to_vec(),
    }
}

/// Removes the byte before the caret (Backspace): the selection when there
/// is one, else the byte at the caret's left; a caret at 0 removes
/// nothing. The anchor follows the caret.
pub fn backspace(text: &mut Vec<u8>, view: &mut EditView) {
    if selection(view).is_some() {
        delete_selection(text, view);
    } else if view.caret != 0 {
        let at = view.caret as usize - 1;
        if at < text.len() {
            text.remove(at);
        }
        view.caret -= 1;
    }
    view.anchor = view.caret;
}

/// Removes the byte at the caret (Delete): the selection when there is
/// one, else the byte to the caret's right; the caret at the end removes
/// nothing.
pub fn delete_forward(text: &mut Vec<u8>, view: &mut EditView) {
    if selection(view).is_some() {
        delete_selection(text, view);
    } else {
        let at = view.caret as usize;
        if at < text.len() {
            text.remove(at);
        }
    }
}

/// Inserts one byte at the caret, a selection deleted first. A text already
/// `max` long takes nothing and `false` is returned: the original's key
/// handler then returns before the anchor and the scroll.
pub fn insert_byte(text: &mut Vec<u8>, view: &mut EditView, byte: u8, max: i32) -> bool {
    if selection(view).is_some() {
        delete_selection(text, view);
    }
    if max <= text.len() as i32 {
        return false;
    }
    let at = (view.caret as usize).min(text.len());
    text.insert(at, byte);
    view.caret = view.caret.wrapping_add(1);
    view.anchor = view.caret;
    true
}

/// One key's handler: a byte to type (`insert` set) or one of the edit
/// keys; with Ctrl (and no byte to type), V pastes `clipboard` (`None` when
/// the system clipboard has no text), C copies and X cuts. A key below
/// Backspace, an unknown key and a byte refused at `max` change nothing
/// and do not scroll. Unless `shift` is held the anchor follows the caret;
/// `width` is the field's, for [`scroll_view`].
#[allow(clippy::too_many_arguments)]
pub fn apply_key(
    font: &GuiFont,
    text: &mut Vec<u8>,
    view: &mut EditView,
    byte: u8,
    insert: bool,
    ctrl: bool,
    shift: bool,
    max: i32,
    width: i32,
    clipboard: Option<&[u8]>,
) -> EditAction {
    if !ctrl || insert {
        if insert {
            if !insert_byte(text, view, byte, max) {
                return EditAction::None;
            }
        } else {
            match byte {
                key::BACKSPACE => backspace(text, view),
                key::END => view.caret = text.len().min(u8::MAX as usize) as u8,
                key::HOME => view.caret = 0,
                key::LEFT => {
                    if view.caret != 0 {
                        view.caret -= 1;
                    }
                }
                key::RIGHT => {
                    if (view.caret as usize) < text.len() {
                        view.caret += 1;
                    }
                }
                key::DELETE => delete_forward(text, view),
                _ => return EditAction::None,
            }
        }
        if !shift {
            view.anchor = view.caret;
        }
        scroll_view(font, text, view, width);
        return EditAction::None;
    }
    match byte {
        key::PASTE => {
            let pasted = paste(text, view, clipboard, max);
            scroll_view(font, text, view, width);
            if pasted {
                EditAction::Paste
            } else {
                EditAction::None
            }
        }
        key::COPY => EditAction::Copy(selected_text(text, view)),
        key::CUT => {
            let cut = selected_text(text, view);
            delete_selection(text, view);
            view.anchor = view.caret;
            scroll_view(font, text, view, width);
            EditAction::Cut(cut)
        }
        _ => EditAction::None,
    }
}

/// What a key asked the caller to do with the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditAction {
    None,
    Copy(Vec<u8>),
    Cut(Vec<u8>),
    Paste,
}

/// The navigation-only handler: Home, End, Left, Right and, with Ctrl (and
/// no byte to type), C to copy; nothing is edited. A key at or below 0x22
/// does nothing. `shift` keeps the anchor.
#[allow(clippy::too_many_arguments)]
pub fn navigate_key(
    font: &GuiFont,
    text: &[u8],
    view: &mut EditView,
    byte: u8,
    insert: bool,
    ctrl: bool,
    shift: bool,
    width: i32,
) -> Option<Vec<u8>> {
    if !ctrl || insert {
        if byte <= 0x22 {
            return None;
        }
        match byte {
            key::END => view.caret = text.len().min(u8::MAX as usize) as u8,
            key::HOME => view.caret = 0,
            key::LEFT => {
                if view.caret != 0 {
                    view.caret -= 1;
                }
            }
            key::RIGHT => {
                if (view.caret as usize) < text.len() {
                    view.caret += 1;
                }
            }
            _ => return None,
        }
        if !shift {
            view.anchor = view.caret;
        }
        scroll_view(font, text, view, width);
        None
    } else if byte == key::COPY {
        Some(selected_text(text, view))
    } else {
        None
    }
}

/// Pastes the clipboard's text at the caret, replacing a selection: the
/// caret moves past it (an 8-bit field, so it wraps), the text is cut to
/// `max` with the caret clamped there, and the anchor joins the caret.
/// `None` (no text on the clipboard) changes nothing and returns `false`.
pub fn paste(text: &mut Vec<u8>, view: &mut EditView, clipboard: Option<&[u8]>, max: i32) -> bool {
    let Some(clipboard) = clipboard else {
        return false;
    };
    if view.anchor != view.caret {
        delete_selection(text, view);
    }
    let at = (view.caret as usize).min(text.len());
    text.splice(at..at, clipboard.iter().copied());
    view.caret = view.caret.wrapping_add(clipboard.len() as u8);
    if max < text.len() as i32 {
        text.truncate(max.max(0) as usize);
        if max < i32::from(view.caret) {
            view.caret = max.max(0) as u8;
        }
    }
    view.anchor = view.caret;
    true
}

/// The caret's x in the field: the advances from the first visible byte to
/// the caret, less half the font's gap. The y is the line's baseline.
pub fn caret_position(font: &GuiFont, text: &[u8], view: &EditView, x: i32, y: i32) -> (i32, i32) {
    let mut pen = x;
    let start = view.first;
    if start < view.caret {
        for index in start..view.caret {
            pen += text
                .get(index as usize)
                .map_or(0, |&byte| font.advance(byte));
        }
    }
    (pen - (font.gap >> 1), y)
}

/// The ink a run of an edit field's draw is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditInk {
    /// The colour already bound (the plain draw sets none).
    Current,
    /// The skin's text ink of the field's state (the caller's index).
    Text(u32),
    /// The skin's selected-text ink.
    Selected,
    /// The skin's ink of the whole-run draw.
    Whole,
}

/// One run of glyphs in one ink.
#[derive(Debug, Clone, PartialEq)]
pub struct EditRun {
    pub ink: EditInk,
    pub quads: Vec<TextQuad>,
}

/// An edit field's draw: its runs in draw order, the selection rectangle
/// drawn between them, and the pen after the last run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditDraw {
    pub runs: Vec<EditRun>,
    /// (left, top, width, height) and the ink: the skin's selection colour
    /// with its alpha halved.
    pub selection: Option<([i32; 4], [u8; 4])>,
    pub end: i32,
}

impl EditDraw {
    /// Every glyph, in draw order.
    pub fn quads(&self) -> impl Iterator<Item = &TextQuad> {
        self.runs.iter().flat_map(|run| run.quads.iter())
    }
}

/// The 1-based inclusive span `[first, last]` from `x`, baseline `y`, into
/// a run; returns the pen at its end.
#[allow(clippy::too_many_arguments)]
fn span_run(
    font: &GuiFont,
    text: &[u8],
    first: usize,
    last: usize,
    x: i32,
    y: i32,
    ink: EditInk,
    out: &mut Vec<EditRun>,
) -> i32 {
    let mut pen = x;
    let mut quads = Vec::new();
    for index in first..=last {
        let Some(&byte) = text.get(index.wrapping_sub(1)) else {
            break;
        };
        quads.push(TextQuad {
            byte,
            rect: [pen, y - font.glyph.1, font.glyph.0, font.glyph.1],
            cell: glyph_cell(byte),
            pen,
            advance: font.advance(byte),
        });
        pen += font.advance(byte);
    }
    out.push(EditRun { ink, quads });
    pen
}

/// Draws an edit field's visible bytes from `x`, baseline `y`.
///
/// `whole` draws the window as a count of bytes from the first visible one
/// in the whole-run ink. Otherwise, without `show_selection` or without a
/// selection, the bytes from the first visible one to the window's end are
/// drawn in the colour already bound. With a selection, the bytes before
/// it (when it starts after the first visible byte) are drawn in the
/// state's text ink (`state`), then a rectangle one line tall, ending one
/// pixel below the baseline, runs under the selected bytes up to the
/// window's end in `selection_colour` at half its alpha, the selected bytes
/// are drawn in the selected ink, and the bytes after the selection (when
/// it ends before the text does) to the window's end in the text ink.
#[allow(clippy::too_many_arguments)]
pub fn draw_edit(
    font: &GuiFont,
    text: &[u8],
    view: &EditView,
    x: i32,
    y: i32,
    show_selection: bool,
    whole: bool,
    state: u32,
    selection_colour: [u8; 4],
) -> EditDraw {
    let mut out = EditDraw {
        end: x,
        ..Default::default()
    };
    let len = text.len();
    let start = view.first as usize + 1;
    let end = view.length as usize;
    if whole {
        let last = (end + start - 1).min(len);
        out.end = span_run(font, text, start, last, x, y, EditInk::Whole, &mut out.runs);
        return out;
    }
    let Some((low, high)) = selection(view).filter(|_| show_selection) else {
        out.end = span_run(
            font,
            text,
            start,
            end.min(len),
            x,
            y,
            EditInk::Current,
            &mut out.runs,
        );
        return out;
    };
    let mut pen = x;
    let mut first = start;
    if (view.first as usize) < low {
        pen = span_run(
            font,
            text,
            start,
            low.min(len),
            x,
            y,
            EditInk::Text(state),
            &mut out.runs,
        );
        first = low + 1;
    }
    let last = high.min(end);
    let right = pen
        + (first..=last)
            .filter_map(|index| text.get(index.wrapping_sub(1)))
            .map(|&byte| font.advance(byte))
            .sum::<i32>();
    let mut ink = selection_colour;
    ink[3] >>= 1;
    out.selection = Some((
        [pen, y - font.line_height + 1, right - pen, font.line_height],
        ink,
    ));
    let mut pen = span_run(
        font,
        text,
        first,
        last,
        pen,
        y,
        EditInk::Selected,
        &mut out.runs,
    );
    if high < len {
        pen = span_run(
            font,
            text,
            high + 1,
            end.min(len),
            pen,
            y,
            EditInk::Text(state),
            &mut out.runs,
        );
    }
    out.end = pen;
    out
}

/// A markup draw's state: the colour stack (eight, like the original), the
/// slant and the alignment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkupState {
    pub inks: [[u8; 4]; 8],
    pub depth: usize,
    /// The slant tag's amount (the main font's when `[scrp=i]`).
    pub slant: f32,
    /// 2 moves a line right by half its rest.
    pub align: i32,
}

impl MarkupState {
    pub fn new(base: [u8; 4]) -> Self {
        Self {
            inks: [base; 8],
            depth: 0,
            slant: 0.0,
            align: 0,
        }
    }

    pub fn ink(&self) -> [u8; 4] {
        self.inks[self.depth]
    }
}

/// An inline picture a `[rimg=NAME]` tag asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkupPicture {
    pub name: Vec<u8>,
    /// Its size over six, in units.
    pub size: (i32, i32),
}

/// Applies a markup tag (`scrp`/`algn`) to the draw's state, as the
/// original's tag routine does.
pub fn apply_markup_state(state: &mut MarkupState, tag: &[u8], main_slant: i32) {
    if tag.starts_with(b"scrp") {
        state.slant = if tag.get(5) == Some(&b'i') {
            main_slant as f32
        } else {
            0.0
        };
    } else if tag.starts_with(b"algn") {
        // A tag without a digit reads the terminating zero.
        state.align = i32::from(tag.get(5).copied().unwrap_or(0)) - 0x30;
    }
}

/// Applies a colour or picture tag to the draw's state. A colour tag pushes
/// the stack (`[tcol=RGB]` takes the skin's tag alpha, `[tcol=RGBA]` its
/// own) or pops it (`[/tcol]`); a picture returns its name and size for the
/// caller to load and draw.
pub fn apply_markup_tag(
    state: &mut MarkupState,
    tag: &[u8],
    tag_alpha: u8,
) -> Option<MarkupPicture> {
    if tag.first() == Some(&b'/') {
        if tag.get(1..5) == Some(b"tcol".as_slice()) {
            state.depth = state.depth.saturating_sub(1);
        }
    } else if tag.starts_with(b"tcol") {
        state.depth = (state.depth + 1).min(state.inks.len() - 1);
        let digits = tag.get(5..).unwrap_or(&[]);
        let ink = if tag.len() == 11 {
            let mut ink = parse_hex_colour(digits, 3);
            ink[3] = tag_alpha;
            ink
        } else {
            // The original scales the tag's own alpha by one and rounds,
            // so the alpha is kept as written, a zero alpha included.
            parse_hex_colour(digits, 4)
        };
        state.inks[state.depth] = ink;
    } else if tag.starts_with(b"rimg") {
        let name = tag.get(5..).unwrap_or(&[]).to_vec();
        return Some(MarkupPicture { name, size: (0, 0) });
    }
    None
}

/// The draw place of an inline picture: its texture size over six in
/// units, centred in the field's width (`x` is the field's left), hanging
/// down from the line's `y`.
pub fn picture_rect(text_width: i32, x: i32, y: i32, texture: (u16, u16), unit: i32) -> [i32; 4] {
    let w = i32::from(texture.0 / 6) * unit;
    let h = i32::from(texture.1 / 6) * unit;
    let left = x + (text_width - w) / 2;
    [left, y, w, h]
}

/// The line `y` after an inline picture: moved down by the picture's
/// height less the draw's line step, so the next line starts below it.
pub fn after_picture(y: i32, height: i32, line_step: i32) -> i32 {
    y + height - line_step
}

/// The pictures an edit's text refers to, in cache order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextureCache {
    entries: Vec<(Vec<u8>, usize)>,
}

impl TextureCache {
    /// The key buffer's size.
    const KEY: usize = 16;

    /// The handle for a name: the cached one, else a new one. The file name
    /// is the name followed by `.rfi`. The original keys its cache by a
    /// 16-byte buffer, so only a name's first 16 bytes tell names apart; it
    /// also passes the buffer through a system string routine first, which
    /// is not reproduced here.
    pub fn load(&mut self, name: &[u8]) -> usize {
        let name = &name[..name.len().min(Self::KEY)];
        if let Some((_, handle)) = self.entries.iter().find(|(n, _)| n == name) {
            return *handle;
        }
        let handle = self.entries.len() + 1;
        self.entries.push((name.to_vec(), handle));
        handle
    }

    /// The file name the loader is asked for.
    pub fn file_name(name: &[u8]) -> Vec<u8> {
        let mut out = name.to_vec();
        out.extend_from_slice(b".rfi");
        out
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The text colour a `{%` span pushes (shared with the paragraph layout).
pub const SPAN: [u8; 3] = SPAN_INK;

/// The caret's glyph.
pub const CARET: u8 = CARET_GLYPH;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::RAYFORM_08;

    fn tens() -> GuiFont {
        let mut font = RAYFORM_08;
        font.glyph = (12, 16);
        font.line_height = 16;
        font.gap = 6;
        font.advances = [10; 256];
        font.advances[b' ' as usize] = 5;
        font
    }

    fn view(anchor: u8, caret: u8, length: u8, first: u8) -> EditView {
        EditView {
            anchor,
            caret,
            length,
            first,
        }
    }

    #[test]
    fn the_caret_walks_the_visible_bytes_by_half_glyphs() {
        let f = tens();
        let text = b"abcdef";
        let mut v = view(0, 6, 6, 0);
        assert_eq!(caret_from_x(&f, text, &mut v, 0), 0);
        assert_eq!(caret_from_x(&f, text, &mut v, 4), 0);
        assert_eq!(caret_from_x(&f, text, &mut v, 5), 1);
        assert_eq!(caret_from_x(&f, text, &mut v, 25), 3);
        assert_eq!(caret_from_x(&f, text, &mut v, 1000), 6);
        // The walk reaches the window's last byte: with an end of 2, a
        // pointer at 12 is past half of a (5) but not of b (2 < 5).
        let mut v = view(0, 0, 2, 0);
        assert_eq!(caret_from_x(&f, text, &mut v, 12), 1);
    }

    #[test]
    fn a_selection_and_the_keys_edit_the_bytes() {
        let f = tens();
        let mut text = b"abcd".to_vec();
        let mut v = view(0, 0, 4, 0);
        // Right, right, then type X over the selection-free caret.
        apply_key(
            &f,
            &mut text,
            &mut v,
            key::RIGHT,
            false,
            false,
            false,
            9,
            80,
            None,
        );
        apply_key(
            &f,
            &mut text,
            &mut v,
            key::RIGHT,
            false,
            false,
            false,
            9,
            80,
            None,
        );
        assert_eq!((text.as_slice(), v.caret), (b"abcd".as_slice(), 2));
        apply_key(&f, &mut text, &mut v, b'X', true, false, false, 9, 80, None);
        assert_eq!(text, b"abXcd");
        assert_eq!((v.caret, v.anchor), (3, 3));
        // Backspace removes the byte before the caret.
        apply_key(
            &f,
            &mut text,
            &mut v,
            key::BACKSPACE,
            false,
            false,
            false,
            9,
            80,
            None,
        );
        assert_eq!(text, b"abcd");
        // Home and End.
        apply_key(
            &f,
            &mut text,
            &mut v,
            key::HOME,
            false,
            false,
            false,
            9,
            80,
            None,
        );
        assert_eq!(v.caret, 0);
        apply_key(
            &f,
            &mut text,
            &mut v,
            key::END,
            false,
            false,
            false,
            9,
            80,
            None,
        );
        assert_eq!(v.caret, 4);
    }

    #[test]
    fn a_shifted_move_keeps_the_anchor_and_delete_removes_the_range() {
        let f = tens();
        let mut text = b"abcdef".to_vec();
        let mut v = view(0, 0, 6, 0);
        apply_key(
            &f,
            &mut text,
            &mut v,
            key::RIGHT,
            false,
            false,
            true,
            9,
            80,
            None,
        );
        apply_key(
            &f,
            &mut text,
            &mut v,
            key::RIGHT,
            false,
            false,
            true,
            9,
            80,
            None,
        );
        assert_eq!((v.anchor, v.caret), (0, 2));
        assert_eq!(selected_text(&text, &v), b"ab");
        apply_key(
            &f,
            &mut text,
            &mut v,
            key::DELETE,
            false,
            false,
            false,
            9,
            80,
            None,
        );
        assert_eq!(text, b"cdef");
        assert_eq!((v.anchor, v.caret, v.length), (0, 0, 4));
    }

    #[test]
    fn typing_stops_at_the_length_limit() {
        let mut text = b"abc".to_vec();
        let mut v = view(3, 3, 3, 0);
        insert_byte(&mut text, &mut v, b'd', 3);
        assert_eq!(text, b"abc");
        insert_byte(&mut text, &mut v, b'd', 4);
        assert_eq!((text.as_slice(), v.caret), (b"abcd".as_slice(), 4));
    }

    #[test]
    fn copy_cut_and_paste_use_the_clipboard() {
        let f = tens();
        let mut text = b"hello".to_vec();
        let mut v = view(0, 2, 5, 0);
        let action = apply_key(
            &f,
            &mut text,
            &mut v,
            key::COPY,
            false,
            true,
            false,
            20,
            80,
            None,
        );
        assert_eq!(action, EditAction::Copy(b"he".to_vec()));
        let action = apply_key(
            &f,
            &mut text,
            &mut v,
            key::CUT,
            false,
            true,
            false,
            20,
            80,
            None,
        );
        assert_eq!(action, EditAction::Cut(b"he".to_vec()));
        assert_eq!(text, b"llo");
        let action = apply_key(
            &f,
            &mut text,
            &mut v,
            key::PASTE,
            false,
            true,
            false,
            20,
            80,
            Some(b"he".as_slice()),
        );
        assert_eq!(action, EditAction::Paste);
        assert_eq!(text, b"hello");
        assert_eq!((v.anchor, v.caret), (2, 2));
        // With nothing selected, a copy takes the whole text.
        let action = apply_key(
            &f,
            &mut text,
            &mut v,
            key::COPY,
            false,
            true,
            false,
            20,
            80,
            None,
        );
        assert_eq!(action, EditAction::Copy(b"hello".to_vec()));
        // An empty clipboard pastes nothing; a paste past the limit is cut
        // to it, the caret clamped there.
        assert!(!paste(&mut text, &mut v, None, 20));
        assert!(paste(&mut text, &mut v, Some(b"xyz".as_slice()), 6));
        assert_eq!(
            (text.as_slice(), v.caret, v.anchor),
            (b"hexyzl".as_slice(), 5, 5)
        );
        // A byte refused at the limit leaves the window unscrolled.
        let mut v = view(6, 6, 1, 3);
        assert_eq!(
            apply_key(&f, &mut text, &mut v, b'q', true, false, false, 6, 80, None),
            EditAction::None
        );
        assert_eq!((v.length, v.first), (1, 3));
    }

    #[test]
    fn the_navigation_handler_changes_no_bytes() {
        let f = tens();
        let text = b"abc".to_vec();
        let mut v = view(0, 1, 3, 0);
        assert_eq!(
            navigate_key(&f, &text, &mut v, key::RIGHT, false, false, false, 80),
            None
        );
        assert_eq!(v.caret, 2);
        v.anchor = 0;
        assert_eq!(
            navigate_key(&f, &text, &mut v, key::COPY, false, true, false, 80),
            Some(vec![b'a', b'b'])
        );
        assert_eq!(
            navigate_key(&f, &text, &mut v, 0x22, false, false, false, 80),
            None
        );
    }

    #[test]
    fn the_visible_window_scrolls_to_the_caret() {
        let f = tens();
        let text = b"abcdefghi";
        // A 30-pixel field shows three glyphs; the caret at 6 scrolls.
        let mut v = view(6, 6, 9, 0);
        scroll_view(&f, text, &mut v, 30);
        // The caret is before the window's end, so no scroll happens; the
        // window shrinks to the bytes that fit from its start.
        assert_eq!(v.first, 0);
        assert_eq!(v.length, 2);
        // The caret at 0 goes back to the start.
        v.caret = 0;
        scroll_view(&f, text, &mut v, 30);
        assert_eq!(v.first, 0);
        // A length past the text clamps, then the width trims the window.
        let mut v = view(0, 0, 200, 0);
        scroll_view(&f, text, &mut v, 30);
        assert_eq!(v.length, 2);
    }

    #[test]
    fn the_caret_sits_half_a_gap_before_its_byte() {
        let f = tens();
        let text = b"abcd";
        let v = view(0, 2, 4, 0);
        assert_eq!(caret_position(&f, text, &v, 10, 20), (27, 20));
        let v = view(0, 0, 4, 0);
        assert_eq!(caret_position(&f, text, &v, 10, 20), (7, 20));
    }

    #[test]
    fn the_selection_lies_behind_its_bytes() {
        let f = tens();
        let text = b"abcd";
        let v = view(3, 1, 4, 0);
        let d = draw_edit(&f, text, &v, 0, 20, true, false, 2, [1, 2, 3, 100]);
        // One line tall, ending a pixel below the baseline, at half alpha.
        let sel = d.selection.unwrap();
        assert_eq!(sel.0, [10, 20 - 16 + 1, 20, 16]);
        assert_eq!(sel.1, [1, 2, 3, 50]);
        let inks: Vec<EditInk> = d.runs.iter().map(|run| run.ink).collect();
        assert_eq!(
            inks,
            [EditInk::Text(2), EditInk::Selected, EditInk::Text(2)]
        );
        let bytes: Vec<u8> = d.quads().map(|q| q.byte).collect();
        assert_eq!(bytes, b"abcd");
        assert_eq!(d.end, 40);
        // Without the selection shown, the window is one run in the bound
        // colour.
        let d = draw_edit(&f, text, &v, 0, 20, false, false, 2, [1, 2, 3, 100]);
        assert!(d.selection.is_none());
        assert_eq!(d.runs.len(), 1);
        assert_eq!(d.runs[0].ink, EditInk::Current);
        assert_eq!(d.quads().count(), 4);
        // The whole-run draw reads the window's end as a count from the
        // first visible byte: two bytes from b.
        let v = view(0, 0, 2, 1);
        let d = draw_edit(&f, text, &v, 0, 20, true, true, 2, [1, 2, 3, 100]);
        let bytes: Vec<u8> = d.quads().map(|q| q.byte).collect();
        assert_eq!(bytes, b"bc");
        assert_eq!(d.runs[0].ink, EditInk::Whole);
    }

    #[test]
    fn the_markup_stack_and_tags() {
        let mut state = MarkupState::new([255, 255, 255, 255]);
        apply_markup_tag(&mut state, b"tcol=ff0000", 180);
        assert_eq!(state.ink(), [255, 0, 0, 180]);
        apply_markup_tag(&mut state, b"tcol=00ff0080", 180);
        assert_eq!(state.ink(), [0, 255, 0, 128]);
        apply_markup_tag(&mut state, b"/tcol", 180);
        assert_eq!(state.ink(), [255, 0, 0, 180]);
        // A written zero alpha is kept.
        apply_markup_tag(&mut state, b"tcol=00ff0000", 180);
        assert_eq!(state.ink(), [0, 255, 0, 0]);
        apply_markup_tag(&mut state, b"/tcol", 180);
        apply_markup_state(&mut state, b"scrp=i", 3);
        assert_eq!(state.slant, 3.0);
        apply_markup_state(&mut state, b"scrp=x", 3);
        assert_eq!(state.slant, 0.0);
        apply_markup_state(&mut state, b"algn=2", 3);
        assert_eq!(state.align, 2);
        let pic = apply_markup_tag(&mut state, b"rimg=MAP", 180).unwrap();
        assert_eq!(pic.name, b"MAP".to_vec());
        assert_eq!(picture_rect(100, 10, 20, (60, 120), 5), [35, 20, 50, 100]);
        assert_eq!(after_picture(20, 100, 16), 104);
    }

    #[test]
    fn the_texture_cache_keys_by_name_and_adds_the_extension() {
        let mut cache = TextureCache::default();
        assert_eq!(cache.load(b"MAP"), 1);
        assert_eq!(cache.load(b"MAP"), 1);
        assert_eq!(cache.load(b"ICON"), 2);
        assert_eq!(cache.len(), 2);
        assert_eq!(TextureCache::file_name(b"MAP"), b"MAP.rfi".to_vec());
        // Only the first 16 bytes key the cache.
        assert_eq!(cache.load(b"abcdefghijklmnopONE"), 3);
        assert_eq!(cache.load(b"abcdefghijklmnopTWO"), 3);
    }
}
