//! Single-line text editing (`RF_GUI_Text_Edit`): the visible range and its
//! horizontal scroll, the caret from a pointer's x, the key handler (typing,
//! Backspace, Delete, Left/Right, Home/End, Ctrl+C/X/V), the navigation-only
//! handler, the in-place selection deletion, the OS clipboard's text, the
//! caret's draw position, the selection's runs and the markup tags
//! (`[tcol]`, `[/tcol]`, `[scrp]`, `[algn]`, `[rimg]`).
//!
//! The edit state is four bytes, as the original's: the anchor and caret
//! (byte counts, so the caret before the first byte is 0), the shown text
//! length and the first visible byte. The text itself is a byte vector;
//! limits are the original's 8-bit fields and the caller's maximum length.
//! Rules: `docs/UI.md` (Text boxes).

use crate::gui_font::{GuiFont, round_i32};
use crate::gui_text::{TextQuad, glyph_uv};
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
    /// The text's shown length, in bytes.
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

/// The caret from a pointer's x, walking the visible bytes while the
/// pointer is past half a glyph (the original's `x` counts from the first
/// visible byte). The caret is clamped to the text's length.
pub fn caret_from_x(font: &GuiFont, text: &[u8], view: &mut EditView, x: i32) -> u8 {
    let len = text.len().min(u8::MAX as usize) as u8;
    let mut first = view.first.min(len);
    let mut caret = view.length;
    let mut left = x;
    if first < view.length {
        loop {
            let next = first + 1;
            let advance = font.advance(text[next as usize - 1]);
            caret = first;
            if left < advance >> 1 {
                break;
            }
            left -= advance;
            first = next;
            caret = view.length;
            if next + 1 >= view.length {
                break;
            }
        }
    }
    let caret = if caret < 1 { 0 } else { caret }.min(len);
    view.caret = caret;
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

/// Removes the selected bytes in place and puts the caret at their start.
/// Returns the number of bytes removed.
pub fn delete_selection(text: &mut Vec<u8>, view: &mut EditView) -> usize {
    let Some((low, high)) = selection(view) else {
        return 0;
    };
    let high = high.min(text.len());
    let low = low.min(high);
    text.drain(low..high);
    view.anchor = low as u8;
    view.caret = low as u8;
    view.length = text.len().min(u8::MAX as usize) as u8;
    high - low
}

/// The bytes the selection covers, for the clipboard; empty when nothing
/// is selected.
pub fn selected_text(text: &[u8], view: &EditView) -> Vec<u8> {
    match selection(view) {
        Some((low, high)) => text[low.min(text.len())..high.min(text.len())].to_vec(),
        None => Vec::new(),
    }
}

/// Removes the byte before the caret (Backspace): the selection when there
/// is one, else the byte at the caret's left; a caret at 0 removes
/// nothing.
pub fn backspace(text: &mut Vec<u8>, view: &mut EditView) {
    if selection(view).is_some() {
        delete_selection(text, view);
    } else if view.caret != 0 {
        let at = (view.caret as usize - 1).min(text.len().saturating_sub(1));
        if at < text.len() {
            text.remove(at);
        }
        view.caret -= 1;
        view.anchor = view.caret;
    }
    view.length = text.len().min(u8::MAX as usize) as u8;
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
    view.length = text.len().min(u8::MAX as usize) as u8;
}

/// Inserts one byte at the caret (replacing a selection), unless the text
/// is at `max` long. Sets the caret after the byte.
pub fn insert_byte(text: &mut Vec<u8>, view: &mut EditView, byte: u8, max: i32) {
    if selection(view).is_some() {
        delete_selection(text, view);
    }
    if max <= text.len() as i32 {
        return;
    }
    let at = (view.caret as usize).min(text.len());
    text.insert(at, byte);
    view.caret += 1;
    view.anchor = view.caret;
    view.length = text.len().min(u8::MAX as usize) as u8;
}

/// One key's handler: a byte to type (`insert` set) or one of the edit
/// keys; with Ctrl, C, X and V copy, cut and paste. `shift` keeps the
/// anchor, `max` limits the text's length and `width` is the field's.
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
    clipboard: &[u8],
) -> EditAction {
    let mut action = EditAction::None;
    if !ctrl || insert {
        if insert {
            insert_byte(text, view, byte, max);
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
                    if (view.caret as usize) < text.len().min(u8::MAX as usize) {
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
        view.length = text.len().min(u8::MAX as usize) as u8;
        scroll_view(font, text, view, width);
    } else {
        match byte {
            key::PASTE => {
                if paste(text, view, clipboard, max) {
                    scroll_view(font, text, view, width);
                    action = EditAction::Paste;
                }
            }
            key::COPY => {
                action = EditAction::Copy(selected_text(text, view));
            }
            key::CUT => {
                action = EditAction::Cut(selected_text(text, view));
                delete_selection(text, view);
                view.length = text.len().min(u8::MAX as usize) as u8;
                scroll_view(font, text, view, width);
            }
            _ => {}
        }
    }
    action
}

/// What a key asked the caller to do with the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditAction {
    None,
    Copy(Vec<u8>),
    Cut(Vec<u8>),
    Paste,
}

/// The navigation-only handler: Home, End, Left, Right and Ctrl+C, with no
/// edit. `shift` keeps the anchor.
pub fn navigate_key(
    font: &GuiFont,
    text: &[u8],
    view: &mut EditView,
    byte: u8,
    ctrl: bool,
    shift: bool,
    width: i32,
) -> Option<Vec<u8>> {
    if !ctrl {
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
                if (view.caret as usize) < text.len().min(u8::MAX as usize) {
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

/// Pastes the clipboard's text at the caret, replacing a selection, and
/// clamps the result to `max`. Returns whether the clipboard was read (the
/// caller knows whether it was available).
pub fn paste(text: &mut Vec<u8>, view: &mut EditView, clipboard: &[u8], max: i32) -> bool {
    let old_len = text.len();
    if view.anchor != view.caret {
        delete_selection(text, view);
    }
    let at = (view.caret as usize).min(text.len().min(old_len));
    let added = clipboard.len();
    text.splice(at..at, clipboard.iter().copied());
    view.caret = view.caret.saturating_add(added.min(u8::MAX as usize) as u8);
    if text.len() as i32 > max {
        text.truncate(max.max(0) as usize);
        if max < i32::from(view.caret) {
            view.caret = max.max(0) as u8;
        }
    }
    view.anchor = view.caret;
    view.length = text.len().min(u8::MAX as usize) as u8;
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

/// A selection rectangle and the glyph runs of an edit field's draw.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditDraw {
    pub quads: Vec<TextQuad>,
    /// (left, top, width, height) and the ink.
    pub selection: Option<([i32; 4], [u8; 4])>,
    pub end: i32,
}

/// Draws an edit field's visible run from `x`, baseline `y`, with the
/// selection behind it. Selection ink is the caller's (the skin's at half
/// alpha). `plain` draws the whole visible run without a selection (the
/// original's single-run path).
#[allow(clippy::too_many_arguments)]
pub fn draw_edit(
    font: &GuiFont,
    text: &[u8],
    view: &EditView,
    x: i32,
    y: i32,
    plain: bool,
    selection_ink: [u8; 4],
) -> EditDraw {
    let mut out = EditDraw {
        end: x,
        ..Default::default()
    };
    let len = text.len().min(u8::MAX as usize);
    let first = (view.first as usize).min(len);
    if plain {
        let last = (view.length as usize + first).min(len);
        let mut pen = x;
        let mut index = first;
        while index < last {
            let byte = text[index];
            out.quads.push(TextQuad {
                byte,
                rect: [pen, y - font.glyph.1, font.glyph.0, font.glyph.1],
                uv: glyph_uv(byte),
                pen,
                advance: font.advance(byte),
            });
            pen += font.advance(byte);
            index += 1;
        }
        out.end = pen;
        return out;
    }
    let span = selection(view).map(|(lo, hi)| (lo.min(len), hi.min(len)));
    let mut pen = x;
    let mut index = first;
    let mut selection_left = x;
    while index < len && index < (view.length as usize + first) {
        if let Some((lo, _)) = span
            && index == lo
        {
            selection_left = if lo < first { x } else { pen };
        }
        let byte = text[index];
        out.quads.push(TextQuad {
            byte,
            rect: [pen, y - font.glyph.1, font.glyph.0, font.glyph.1],
            uv: glyph_uv(byte),
            pen,
            advance: font.advance(byte),
        });
        pen += font.advance(byte);
        index += 1;
        if let Some((_, hi)) = span
            && index == hi
        {
            out.selection = Some((
                [
                    selection_left,
                    y - font.glyph.1 + 1,
                    pen - selection_left,
                    font.line_height,
                ],
                selection_ink,
            ));
        }
        if index >= (view.length as usize + first) {
            break;
        }
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
        state.align = i32::from(tag.get(5).copied().unwrap_or(b'0')) - 0x30;
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
        let mut ink = if tag.len() == 11 {
            let mut ink = parse_hex_colour(digits, 3);
            ink[3] = tag_alpha;
            ink
        } else {
            let mut ink = parse_hex_colour(digits, 4);
            ink[3] = round_i32(f64::from(f32::from(ink[3]))) as u8;
            ink
        };
        if tag.len() != 11 && ink[3] == 0 {
            ink[3] = tag_alpha;
        }
        state.inks[state.depth] = ink;
    } else if tag.starts_with(b"rimg") {
        let name = tag.get(5..).unwrap_or(&[]).to_vec();
        return Some(MarkupPicture { name, size: (0, 0) });
    }
    None
}

/// The draw place of an inline picture: its texture size over six in
/// units, centred in the field's width, its baseline at the pen.
pub fn picture_rect(text_width: i32, x: i32, y: i32, texture: (u16, u16), unit: i32) -> [i32; 4] {
    let w = i32::from(texture.0 / 6) * unit;
    let h = i32::from(texture.1 / 6) * unit;
    let left = x + (text_width - w) / 2;
    [left, y, w, h]
}

/// The pictures an edit's text refers to, in cache order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextureCache {
    entries: Vec<(Vec<u8>, usize)>,
}

impl TextureCache {
    /// The handle for a name: the cached one, else a new one. The file name
    /// is the name followed by `.rfi`.
    pub fn load(&mut self, name: &[u8]) -> usize {
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
            b"",
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
            b"",
        );
        assert_eq!((text.as_slice(), v.caret), (b"abcd".as_slice(), 2));
        apply_key(&f, &mut text, &mut v, b'X', true, false, false, 9, 80, b"");
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
            b"",
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
            b"",
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
            b"",
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
            b"",
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
            b"",
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
            b"",
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
            b"",
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
            b"",
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
            b"he",
        );
        assert_eq!(action, EditAction::Paste);
        assert_eq!(text, b"hello");
        assert_eq!((v.anchor, v.caret), (2, 2));
    }

    #[test]
    fn the_navigation_handler_changes_no_bytes() {
        let f = tens();
        let text = b"abc".to_vec();
        let mut v = view(0, 1, 3, 0);
        assert_eq!(
            navigate_key(&f, &text, &mut v, key::RIGHT, false, false, 80),
            None
        );
        assert_eq!(v.caret, 2);
        v.anchor = 0;
        assert_eq!(
            navigate_key(&f, &text, &mut v, key::COPY, true, false, 80),
            Some(vec![b'a', b'b'])
        );
        assert_eq!(
            navigate_key(&f, &text, &mut v, 0x22, false, false, 80),
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
        let d = draw_edit(&f, text, &v, 0, 20, false, [1, 2, 3, 100]);
        let sel = d.selection.unwrap();
        assert_eq!(sel.0, [10, 20 - 16 + 1, 20, 16]);
        assert_eq!(sel.1, [1, 2, 3, 100]);
        // The plain path draws the visible run without a selection.
        let d = draw_edit(&f, text, &v, 0, 20, true, [1, 2, 3, 100]);
        assert!(d.selection.is_none());
        assert_eq!(d.quads.len(), 4);
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
        apply_markup_state(&mut state, b"scrp=i", 3);
        assert_eq!(state.slant, 3.0);
        apply_markup_state(&mut state, b"scrp=x", 3);
        assert_eq!(state.slant, 0.0);
        apply_markup_state(&mut state, b"algn=2", 3);
        assert_eq!(state.align, 2);
        let pic = apply_markup_tag(&mut state, b"rimg=MAP", 180).unwrap();
        assert_eq!(pic.name, b"MAP".to_vec());
        assert_eq!(picture_rect(100, 10, 20, (60, 120), 5), [35, 20, 50, 100]);
    }

    #[test]
    fn the_texture_cache_keys_by_name_and_adds_the_extension() {
        let mut cache = TextureCache::default();
        assert_eq!(cache.load(b"MAP"), 1);
        assert_eq!(cache.load(b"MAP"), 1);
        assert_eq!(cache.load(b"ICON"), 2);
        assert_eq!(cache.len(), 2);
        assert_eq!(TextureCache::file_name(b"MAP"), b"MAP.rfi".to_vec());
    }
}
