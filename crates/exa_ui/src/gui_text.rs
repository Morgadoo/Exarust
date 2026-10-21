//! The GUI's text support routines: measuring a byte run with a font's
//! advance table, choosing a font (and its texture), emitting one line's
//! glyph quads (plain, centred, clipped to a width, or from the tail), the
//! wrapped block draw with its first-line skip and centring, the line count
//! and the width fit, the ellipsis truncation, and the menu font's rescale.
//!
//! The six bitmap fonts and their tables are in [`crate::gui_font`]. The
//! main menu draws its rows with a sprite sheet instead: a 4 × 7 sheet of
//! 112-pixel letter cells, whose record the original rescales from its
//! design values on every row-scale change ([`MenuFont`]). Glyph quad
//! coordinates are pixels with the top-left origin; a drawn line's `y` is
//! the baseline (the quad's bottom edge). Rules: `docs/UI.md` (Fonts,
//! Text boxes).

use crate::gui_font::{GuiFont, round_i32};

/// The glyph the original draws as an ellipsis.
pub const ELLIPSIS: u8 = 0x85;

/// A drawn glyph quad and the pen before it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextQuad {
    /// The byte's cell in a 16 × 16 atlas.
    pub byte: u8,
    /// The quad's pixel rectangle (left, top, width, height).
    pub rect: [i32; 4],
    /// The atlas cell's corner (u, v): the column and row over 16. The
    /// sample's far corner is this plus the font's sample step
    /// ([`font_uv`], the glyph quad over the texture's size), which the
    /// original adds per vertex; the quad's top edge samples the far row
    /// (the glyphs are stored upside down).
    pub cell: (f32, f32),
    /// The pen before the glyph.
    pub pen: i32,
    /// The advance the glyph takes.
    pub advance: i32,
}

/// The width of a byte run with the main font (0 for an empty run).
pub fn measure(font: &GuiFont, text: &[u8]) -> i32 {
    text.iter().map(|&b| font.advance(b)).sum()
}

/// The width of a byte run with an explicitly given font.
pub fn measure_in(font: &GuiFont, text: &[u8]) -> i32 {
    measure(font, text)
}

/// The font selection's result: whether the atlas texture changed and so
/// has to be bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FontBinding {
    /// The font's texture identity last bound; `None` before any.
    pub bound: Option<&'static str>,
}

impl FontBinding {
    /// Selects `font`: binds its texture when it differs from the last one
    /// bound, as the original's `RF_GUI_Main` font selector does.
    pub fn select(&mut self, font: &GuiFont) -> bool {
        if self.bound == Some(font.texture) {
            return false;
        }
        self.bound = Some(font.texture);
        true
    }
}

/// The atlas sample scale of a font: its glyph quad over the texture's
/// size, or `None` when the texture is missing or has no extent.
pub fn font_uv(font: &GuiFont, texture: (u16, u16)) -> Option<(f32, f32)> {
    if texture.0 == 0 || texture.1 == 0 {
        return None;
    }
    Some((
        font.glyph.0 as f32 / f32::from(texture.0),
        font.glyph.1 as f32 / f32::from(texture.1),
    ))
}

/// The atlas cell of one byte: its column and row over 16. The sample
/// spans this corner plus the font's sample step ([`font_uv`]).
pub fn glyph_cell(byte: u8) -> (f32, f32) {
    let row = f32::from(byte >> 4);
    let column = f32::from(byte) - row * 16.0;
    (column * 0.0625, row * 0.0625)
}

/// One line of glyph quads at `x`, baseline `y`, ending at `x + width`.
/// Returns the pen after the last glyph.
#[allow(clippy::too_many_arguments)]
fn line_quads(
    font: &GuiFont,
    text: &[u8],
    first: usize,
    last: usize,
    mut x: i32,
    y: i32,
    limit: Option<i32>,
    out: &mut Vec<TextQuad>,
) -> i32 {
    let (w, h) = font.glyph;
    for &byte in &text[first..last] {
        let pen = x;
        out.push(TextQuad {
            byte,
            rect: [pen, y - h, w, h],
            cell: glyph_cell(byte),
            pen,
            advance: font.advance(byte),
        });
        x += font.advance(byte);
        if limit.is_some_and(|limit| limit < x) {
            return x;
        }
    }
    x
}

/// Draws one line of text at `x`, baseline `y`, emitting every glyph quad.
/// The texture is expected to be bound already.
pub fn draw_line(font: &GuiFont, text: &[u8], x: i32, y: i32) -> Vec<TextQuad> {
    let mut out = Vec::with_capacity(text.len());
    line_quads(font, text, 0, text.len(), x, y, None, &mut out);
    out
}

/// The same quads without the original's begin/end batch calls.
pub fn draw_line_plain(font: &GuiFont, text: &[u8], x: i32, y: i32) -> Vec<TextQuad> {
    draw_line(font, text, x, y)
}

/// Draws one line centred in `width` (the line moved right by
/// `round((width − total)·0.5 + x)` when it is narrower) and stopped after
/// the first glyph whose pen passes the moved start plus `width`. Returns
/// the quads and the pen at their end.
pub fn draw_line_centred(
    font: &GuiFont,
    text: &[u8],
    x: i32,
    y: i32,
    width: i32,
) -> (Vec<TextQuad>, i32) {
    let total: i32 = measure(font, text);
    let start = if total < width {
        round_i32(f64::from((width - total) as f32 * 0.5 + x as f32))
    } else {
        x
    };
    let mut out = Vec::with_capacity(text.len());
    let pen = line_quads(
        font,
        text,
        0,
        text.len(),
        start,
        y,
        Some(start + width),
        &mut out,
    );
    (out, pen)
}

/// Draws the bytes `[first, last]` (both 1-based and inclusive) at `x`,
/// baseline `y`. Returns the pen at the end.
pub fn draw_span(font: &GuiFont, text: &[u8], first: i32, last: i32, x: i32, y: i32) -> i32 {
    if text.is_empty() || first < 1 || last < first {
        return x;
    }
    let first = (first as usize - 1).min(text.len());
    let last = (last as usize).min(text.len());
    let mut out = Vec::new();
    line_quads(font, text, first, last, x, y, None, &mut out)
}

/// Shortens `text` to `width` with an ellipsis. Starting from the
/// ellipsis' advance, the advances of the bytes before the last are added
/// one by one; when the sum passes `width` at the k-th byte (1-based), the
/// first k − 1 bytes and the ellipsis are returned. The last byte is never
/// counted, and a text whose sum stays within `width` is returned whole.
pub fn ellipsis_fit(font: &GuiFont, text: &[u8], width: i32) -> Vec<u8> {
    let mut used = font.advance(ELLIPSIS);
    for k in 1..text.len() {
        used += font.advance(text[k - 1]);
        if width < used {
            let mut out = text[..k - 1].to_vec();
            out.push(ELLIPSIS);
            return out;
        }
    }
    text.to_vec()
}

/// Draws a line cut to at most `round(width) / gap` glyphs, the last one
/// replaced by the ellipsis when the text is longer. The pen still moves
/// by the replaced byte's own advance, as the original's does.
pub fn draw_truncated(font: &GuiFont, text: &[u8], x: i32, y: i32, width: i32) -> Vec<TextQuad> {
    let mut out = Vec::new();
    if width < font.gap {
        return out;
    }
    let count = round_i32(f64::from(width)) / font.gap;
    let (marker, draw) = if text.len() as i32 <= count {
        (text.len() as i32 + 1, text.len())
    } else {
        (count, count.max(0) as usize)
    };
    let mut pen = x;
    for (i, &byte) in text.iter().take(draw).enumerate() {
        let byte = if i as i32 + 1 == marker {
            ELLIPSIS
        } else {
            byte
        };
        out.push(TextQuad {
            byte,
            rect: [pen, y - font.glyph.1, font.glyph.0, font.glyph.1],
            cell: glyph_cell(byte),
            pen,
            advance: font.advance(text[i]),
        });
        pen += font.advance(text[i]);
    }
    out
}

/// Draws the tail of a line that fits `width`, with an ellipsis standing in
/// for the first byte drawn when a head is cut off. The fit is measured
/// with `fit_font` (the original measures with the GUI level's main font)
/// and the tail is drawn and placed with `font`, starting where it would
/// sit had the whole line been drawn from `x`.
pub fn draw_tail(
    font: &GuiFont,
    fit_font: &GuiFont,
    text: &[u8],
    x: i32,
    y: i32,
    width: i32,
) -> Vec<TextQuad> {
    let mut out = Vec::new();
    let len = text.len();
    if len == 0 {
        return out;
    }
    let mut left = width;
    let mut first = 1usize;
    let mut index = len + 1;
    while index > 1 {
        let at = index - 1;
        left -= fit_font.advance(text[at - 1]);
        first = index;
        if left < 0 {
            break;
        }
        index = at;
        first = 1;
    }
    if first > len {
        return out;
    }
    let mut pen = x + (1..first).map(|i| font.advance(text[i - 1])).sum::<i32>();
    for i in first..=len {
        let byte = if i == first && first > 1 {
            ELLIPSIS
        } else {
            text[i - 1]
        };
        out.push(TextQuad {
            byte,
            rect: [pen, y - font.glyph.1, font.glyph.0, font.glyph.1],
            cell: glyph_cell(byte),
            pen,
            advance: font.advance(byte),
        });
        pen += font.advance(byte);
    }
    out
}

/// Scans the line that starts at 0-based `start` for `width`: the end
/// (exclusive) and the width left at the line's last break, which a centred
/// line is moved right by half of. A line feed is skipped by the caller.
fn scan_line(text: &[u8], start: usize, width: i32, font: &GuiFont) -> (usize, u32) {
    let len = text.len();
    let mut left = width;
    let mut rest = width as u32;
    let mut space = 0usize;
    let mut end = len;
    let mut k = start;
    while k < len {
        let c = text[k];
        if c == b'\r' {
            rest = left as u32;
            end = k + 1;
            break;
        }
        if c == b' ' {
            space = k + 1;
            rest = left as u32;
        }
        left -= font.advance(c);
        if left < 0 {
            end = if space < 1 { k + 1 } else { space };
            break;
        }
        if len - 1 == k {
            rest = left as u32;
        }
        k += 1;
    }
    (end, rest)
}

/// Scans the line that starts at 0-based `start` for `width` the way the
/// width fit does: the end (exclusive) and the pixels used, which count
/// every byte that still fits, the head of a word moved to the next line
/// included, and stop before a carriage return or the overflowing byte.
fn used_line(text: &[u8], start: usize, width: i32, font: &GuiFont) -> (usize, i32) {
    let len = text.len();
    let mut used = 0;
    let mut space = 0usize;
    let mut end = len;
    let mut k = start;
    while k < len {
        let c = text[k];
        if c == b'\r' {
            end = k + 1;
            break;
        }
        if c == b' ' {
            space = k + 1;
        }
        let next = used + font.advance(c);
        if width < next {
            end = if space < 1 { k + 1 } else { space };
            break;
        }
        used = next;
        k += 1;
    }
    (end, used)
}

/// The start of the line after one that ends at `end`: a line feed there
/// is skipped.
fn next_start(text: &[u8], end: usize) -> usize {
    if text.get(end) == Some(&b'\n') {
        end + 1
    } else {
        end
    }
}

/// The line count of a plain text at `width` (an empty text has none).
pub fn count_lines(font: &GuiFont, text: &[u8], width: i32) -> i32 {
    let mut count = 0;
    let mut start = 0usize;
    while start < text.len() {
        let (end, _) = scan_line(text, start, width, font);
        start = next_start(text, end);
        count += 1;
    }
    count
}

/// Narrows `width` to the text's average line width, then widens it by
/// the GUI `unit` until the text wraps into no more lines than it took at
/// the given width; `width` ends as the widest line of that last pass and
/// that pass's line count is returned. A final carriage return counts one
/// line more. A text under two bytes is one line and leaves the width
/// alone; one that takes a single line gets the width it used.
pub fn fit_width(font: &GuiFont, text: &[u8], width: &mut i32, unit: i32) -> i32 {
    if text.len() < 2 {
        return 1;
    }
    let mut lines = 0;
    let mut sum = 0i32;
    let mut start = 0usize;
    while start < text.len() {
        let (end, used) = used_line(text, start, *width, font);
        sum += used;
        start = next_start(text, end);
        lines += 1;
    }
    if text.last() == Some(&b'\r') {
        lines += 1;
    }
    if lines < 2 {
        *width = sum;
        return lines;
    }
    *width = sum / lines;
    loop {
        let mut widest = 0;
        let mut count = 0;
        let mut start = 0usize;
        while start < text.len() {
            let (end, used) = used_line(text, start, *width, font);
            widest = widest.max(used);
            start = next_start(text, end);
            count += 1;
        }
        *width += unit;
        if count <= lines {
            *width = widest;
            return count;
        }
    }
}

/// A wrapped block's draw: the glyphs in draw order, the line count reached
/// and the pen's end position on the last line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WrappedDraw {
    pub quads: Vec<TextQuad>,
    pub lines: i32,
    pub end: (i32, i32),
}

/// Draws a plain text wrapped at `width`, from the line top `y` down to
/// `y + height`: a line is drawn while its top is at most `y + height −
/// line_height`, and its glyphs hang down from that top (unlike a single
/// line, whose `y` is the baseline). The first `first_line` lines are drawn
/// at `y` without moving down. A centred line is moved right by half the
/// width left at its last break. Returns the lines scanned and the glyphs.
#[allow(clippy::too_many_arguments)]
pub fn draw_wrapped(
    font: &GuiFont,
    text: &[u8],
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    first_line: i32,
    centre: bool,
) -> WrappedDraw {
    let mut out = WrappedDraw {
        end: (x, y),
        ..Default::default()
    };
    let line_height = font.line_height;
    let limit = y + height;
    let mut pen_y = y;
    let mut start = 0usize;
    let mut line = 0i32;
    loop {
        if text.len() <= start || limit - line_height < pen_y {
            return out;
        }
        let (end, rest) = scan_line(text, start, width, font);
        let line_x = if centre { x + (rest >> 1) as i32 } else { x };
        let mut pen = line_x;
        if start <= end.saturating_sub(1) {
            for &byte in &text[start..end] {
                if byte != b'\n' {
                    out.quads.push(TextQuad {
                        byte,
                        rect: [pen, pen_y, font.glyph.0, font.glyph.1],
                        cell: glyph_cell(byte),
                        pen,
                        advance: font.advance(byte),
                    });
                    pen += font.advance(byte);
                }
            }
        }
        out.end = (pen, pen_y);
        start = end;
        if start < text.len() && text[start] == b'\n' {
            start += 1;
        }
        if first_line <= line {
            pen_y += line_height;
        }
        line += 1;
        out.lines = line;
    }
}

/// The float-position form of [`draw_wrapped`]: the original's draw with
/// pixel positions kept as floats and the block's bottom rounded.
#[allow(clippy::too_many_arguments)]
pub fn draw_wrapped_at(
    font: &GuiFont,
    text: &[u8],
    x: f32,
    y: f32,
    width: i32,
    height: i32,
    first_line: i32,
    centre: bool,
) -> WrappedDraw {
    let mut out = WrappedDraw {
        end: (round_i32(f64::from(x)), round_i32(f64::from(y))),
        ..Default::default()
    };
    let line_height = font.line_height;
    // The block's bottom is fixed at the start; the check is against the
    // current line's top, as the original keeps its first sum.
    let bottom = round_i32(f64::from(y + height as f32 - line_height as f32));
    let mut pen_y = y;
    let mut start = 0usize;
    let mut line = 0i32;
    loop {
        if text.len() <= start || (bottom as f32) < pen_y {
            return out;
        }
        let (end, rest) = scan_line(text, start, width, font);
        let line_x = if centre { x + (rest >> 1) as f32 } else { x };
        let mut pen = line_x;
        if start <= end.saturating_sub(1) {
            for &byte in &text[start..end] {
                if byte != b'\n' {
                    out.quads.push(TextQuad {
                        byte,
                        rect: [
                            round_i32(f64::from(pen)),
                            round_i32(f64::from(pen_y)),
                            font.glyph.0,
                            font.glyph.1,
                        ],
                        cell: glyph_cell(byte),
                        pen: round_i32(f64::from(pen)),
                        advance: font.advance(byte),
                    });
                    pen += font.advance(byte) as f32;
                }
            }
        }
        out.end = (round_i32(f64::from(pen)), round_i32(f64::from(pen_y)));
        start = end;
        if start < text.len() && text[start] == b'\n' {
            start += 1;
        }
        if first_line <= line {
            pen_y += line_height as f32;
        }
        line += 1;
        out.lines = line;
    }
}

/// Adds `gap` to every advance of a font record, wrapping to 16 bits (the
/// original's start-up step that folds the gap into the stored widths).
pub fn add_gap(advances: &mut [i16; 256], gap: i32) {
    for advance in advances.iter_mut() {
        *advance = advance.wrapping_add(gap as i16);
    }
}

/// The wraps-to-the-same-lines width rule over a menu font.
pub fn fit_menu_width(font: &MenuFont, text: &[u8], width: &mut i32, unit: i32) -> i32 {
    let as_gui = font.as_gui_font();
    fit_width(&as_gui, text, width, unit)
}

/// Adds every font's gap to its own advances (the original's start-up step
/// over its six bitmap fonts).
pub fn add_gap_to_fonts<const N: usize>(fonts: &mut [GuiFont; N]) {
    for font in fonts.iter_mut() {
        let gap = font.gap;
        add_gap(&mut font.advances, gap);
    }
}

/// The main menu's sprite font record: 112-pixel letter cells in a 4 × 7
/// sheet at its design scale, rescaled by half the row scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuFont {
    /// The atlas sample step; zero for the bitmap fonts.
    pub uv: (f32, f32),
    /// The drawn cell in pixels.
    pub glyph: (i32, i32),
    /// The height rows are centred by.
    pub ascent: i32,
    pub slant: i32,
    /// The advance of a space.
    pub gap: i32,
    pub line_height: i32,
    /// The cell's texture origin (a guess: the first glyph's inset).
    pub origin: (i32, i32),
    /// The subtraction the menu row width makes (see [`MenuFont::row_width`]).
    pub trim: i32,
    pub advances: [i16; 256],
}

/// The texture the menu font's record names (its file-name source).
pub const MENU_FONT_TEXTURE: &str = "titlefont.rfi";

/// The menu font's design record (112-pixel cells, 70-pixel rows at scale
/// one). Its letter advances are the sheet's recorded widths.
pub const MENU_FONT: MenuFont = MenuFont {
    uv: (0.0, 0.0),
    glyph: (112, 112),
    ascent: 98,
    slant: 4,
    gap: 36,
    line_height: 128,
    origin: (16, 32),
    trim: 6,
    advances: [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, -2, -4, 0, 0, 0, 0, -7, -1, 0, 0, 0, -3, 0, -3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        -3, -3, 0, 0, 0, 0, 0, 76, 58, 70, 64, 58, 56, 76, 80, 36, 36, 72, 60, 96, 80, 76, 60, 76,
        66, 54, 66, 76, 76, 96, 68, 64, 64, 56, 0, 0, 0, 0, 0, -1, 0, 0, 0, 0, -2, 0, 0, -6, -4,
        -1, -6, 4, 0, 1, 0, 0, -3, -2, -4, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
};

/// Whether a new scale differs by at least 0.001 from the last one (in
/// single precision, then compared as a double); a not-a-number scale
/// always rescales.
pub fn needs_rescale(last: f32, scale: f32) -> bool {
    let difference = (last - scale).abs();
    f64::from(difference).partial_cmp(&0.001) != Some(std::cmp::Ordering::Less)
}

impl MenuFont {
    /// The record at `scale`: the design record with every metric scaled
    /// and rounded half to even, the advances of `A` through `a` included
    /// (the original's conversions all round with the default mode).
    pub fn at_scale(scale: f32) -> Self {
        let mut font = MENU_FONT;
        let round = |value: i32| round_i32(f64::from(value as f32 * scale));
        font.glyph = (round(MENU_FONT.glyph.0), round(MENU_FONT.glyph.1));
        font.ascent = round(MENU_FONT.ascent);
        font.gap = round(MENU_FONT.gap);
        font.line_height = round(MENU_FONT.line_height);
        font.origin = (round(MENU_FONT.origin.0), round(MENU_FONT.origin.1));
        font.trim = round(MENU_FONT.trim);
        for (index, advance) in font.advances.iter_mut().enumerate() {
            // Only the letters the sheet records (A through a) are
            // rescaled, exactly as the original's loop does.
            if (0x41..=0x61).contains(&index) {
                *advance =
                    round_i32(f64::from(f32::from(MENU_FONT.advances[index]) * scale)) as i16;
            }
        }
        font
    }

    /// The advance of a byte in the menu record: a space takes the gap.
    pub fn advance(&self, byte: u8) -> i32 {
        if byte == b' ' {
            self.gap
        } else {
            i32::from(self.advances[byte as usize])
        }
    }

    /// A menu row's width: the advances of its bytes, a space counted as
    /// the gap, less the record's trim.
    pub fn row_width(&self, text: &[u8]) -> i32 {
        text.iter().map(|&b| self.advance(b)).sum::<i32>() - self.trim
    }

    /// A bitmap-font view of the record, for the generic line rules (the
    /// menu's advance of a space differs and is handled by `row_width`).
    fn as_gui_font(&self) -> GuiFont {
        GuiFont {
            texture: MENU_FONT_TEXTURE,
            glyph: self.glyph,
            ascent: self.ascent,
            slant: self.slant,
            gap: self.gap,
            line_height: self.line_height,
            advances: self.advances,
        }
    }

    /// The menu sprite line: 26 letters in a sheet of four columns and
    /// eight rows (the cell's column over four, its row 0.875 less an
    /// eighth per row; the far corner adds the record's sample step), a
    /// space taking the gap. `x` is moved left by
    /// the cell origin first, as the original does. Returns the quads and
    /// the pen after them.
    pub fn draw_line(&self, text: &[u8], x: i32, y: i32) -> (Vec<TextQuad>, i32) {
        let mut out = Vec::with_capacity(text.len());
        let mut pen = x - self.origin.0;
        for &byte in text {
            if byte != b' ' {
                // The letter index is unsigned: a byte below `A` wraps to a
                // huge index and samples far outside the sheet, as the
                // original's does.
                let index = u32::from(byte).wrapping_sub(0x41);
                let row = (index >> 2) as f32;
                let column = (index as i32) as f32 - row * 4.0;
                out.push(TextQuad {
                    byte,
                    rect: [pen, y - self.glyph.1, self.glyph.0, self.glyph.1],
                    cell: (column * 0.25, 0.875 - row * 0.125),
                    pen,
                    advance: i32::from(self.advances[byte as usize]),
                });
            }
            pen += if byte == b' ' {
                self.gap
            } else {
                i32::from(self.advances[byte as usize])
            };
        }
        (out, pen)
    }
}

/// The menu font scale from the row scale: half of it.
pub fn menu_font_scale(row_scale: f32) -> f32 {
    row_scale * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::RAYFORM_10;

    /// A font where every byte advances 10, a space 5, a carriage return
    /// and a line feed 0.
    fn tens() -> GuiFont {
        let mut advances = [10i16; 256];
        advances[b' ' as usize] = 5;
        advances[b'\r' as usize] = 0;
        advances[b'\n' as usize] = 0;
        GuiFont {
            texture: "test",
            glyph: (12, 16),
            ascent: 13,
            slant: 3,
            gap: 6,
            line_height: 16,
            advances,
        }
    }

    #[test]
    fn measuring_sums_the_advance_table() {
        let f = tens();
        assert_eq!(measure(&f, b""), 0);
        assert_eq!(measure(&f, b"ab"), 20);
        assert_eq!(measure(&f, b"a b"), 25);
        assert_eq!(measure_in(&f, b"abc"), 30);
        assert_eq!(measure(&RAYFORM_10, b"AB"), 21);
    }

    #[test]
    fn a_font_binds_once_per_texture() {
        let mut binding = FontBinding::default();
        assert!(binding.select(&RAYFORM_10));
        assert!(!binding.select(&RAYFORM_10));
        assert!(binding.select(&tens()));
    }

    #[test]
    fn a_font_samples_its_cell() {
        assert_eq!(
            font_uv(&RAYFORM_10, (1024, 1024)),
            Some((15.0 / 1024.0, 20.0 / 1024.0))
        );
        assert_eq!(font_uv(&RAYFORM_10, (0, 10)), None);
        assert_eq!(glyph_cell(b'A'), (0.0625, 0.25));
    }

    #[test]
    fn a_line_advances_and_flips_its_rows() {
        let f = tens();
        let quads = draw_line(&f, b"ab", 7, 30);
        assert_eq!(quads.len(), 2);
        assert_eq!(quads[0].rect, [7, 14, 12, 16]);
        assert_eq!(quads[1].rect, [17, 14, 12, 16]);
        assert_eq!(quads[0].cell, (0.0625, 0.375));
        assert_eq!(quads[1].pen, 17);
    }

    #[test]
    fn a_centred_line_is_clipped_at_its_width() {
        let f = tens();
        // 30 wide in 100: starts at 35; four glyphs would end at 75.
        let quads = draw_line(&f, b"abcdef", 0, 0);
        assert_eq!(quads.len(), 6);
        let mut out = Vec::new();
        let pen = line_quads(&f, b"abcdef", 0, 6, 35, 0, Some(35 + 30), &mut out);
        assert_eq!(pen, 75);
        assert_eq!(out.len(), 4);
        // The real centred draw of a 40-pixel line in 100 starts at 30.
        let (quads, pen) = draw_line_centred(&f, b"abcd", 0, 0, 100);
        assert_eq!((quads[0].rect[0], pen), (30, 70));
        // A line wider than its box starts at x and stops after the glyph
        // that passes x + width: 25 wide stops at the third glyph, 30.
        let (quads, pen) = draw_line_centred(&f, b"abcdef", 0, 0, 25);
        assert_eq!((quads.len(), pen), (3, 30));
    }

    #[test]
    fn a_span_draws_its_inclusive_range() {
        let f = tens();
        assert_eq!(draw_span(&f, b"abcde", 2, 4, 10, 0), 40);
        assert_eq!(draw_span(&f, b"abcde", 0, 2, 10, 0), 10);
        assert_eq!(draw_span(&f, b"abcde", 2, 1, 10, 0), 10);
    }

    #[test]
    fn the_ellipsis_replaces_the_tail_that_does_not_fit() {
        let mut f = tens();
        f.advances[ELLIPSIS as usize] = 10;
        assert_eq!(ellipsis_fit(&f, b"abcdef", 100), b"abcdef");
        // 10 for the ellipsis, then a, b, c reach 40 and d passes 45 at the
        // fourth byte: the first three bytes stay.
        assert_eq!(ellipsis_fit(&f, b"abcdef", 45), b"abc\x85");
        // The first byte already passes 5: only the ellipsis is left.
        assert_eq!(ellipsis_fit(&f, b"ab", 5), b"\x85");
        // The last byte is never counted: 10 + 10 + 10 is 30 for "abc".
        assert_eq!(ellipsis_fit(&f, b"abc", 30), b"abc");
    }

    #[test]
    fn a_truncated_line_ends_in_an_ellipsis() {
        let mut f = tens();
        f.gap = 10;
        f.advances[ELLIPSIS as usize] = 10;
        // 45 / 10 = 4 glyphs: three bytes and the ellipsis.
        let q = draw_truncated(&f, b"abcdef", 0, 20, 45);
        assert_eq!(q.len(), 4);
        assert_eq!(q.iter().map(|g| g.byte).collect::<Vec<_>>(), b"abc\x85");
        assert_eq!(q[3].rect[0], 30);
        // The ellipsis advances by the byte it replaces.
        f.advances[ELLIPSIS as usize] = 3;
        let q = draw_truncated(&f, b"abcdef", 0, 20, 45);
        assert_eq!(q[3].advance, 10);
        // A short text draws whole, and a narrow width draws nothing.
        assert_eq!(
            draw_truncated(&f, b"ab", 0, 20, 45)
                .iter()
                .map(|g| g.byte)
                .collect::<Vec<_>>(),
            b"ab"
        );
        assert!(draw_truncated(&f, b"abc", 0, 20, 5).is_empty());
    }

    #[test]
    fn a_tail_is_fitted_and_headed_by_an_ellipsis() {
        let mut f = tens();
        f.advances[ELLIPSIS as usize] = 10;
        // 25 pixels show the last two bytes and the ellipsis.
        let q = draw_tail(&f, &f, b"abcdef", 4, 20, 25);
        assert_eq!(q.iter().map(|g| g.byte).collect::<Vec<_>>(), b"\x85f");
        assert_eq!(q[0].pen, 4 + 40);
        // A generous width draws the whole line.
        let q = draw_tail(&f, &f, b"abcdef", 0, 20, 1000);
        assert_eq!(q.len(), 6);
        // The fit is measured with the second font: at 25, a font of
        // 30-pixel bytes keeps only the last byte, headed by the ellipsis.
        let mut wide = tens();
        wide.advances = [30; 256];
        wide.advances[b'f' as usize] = 20;
        let q = draw_tail(&f, &wide, b"abcdef", 0, 20, 25);
        assert_eq!(q.iter().map(|g| g.byte).collect::<Vec<_>>(), b"\x85");
        assert_eq!(q[0].pen, 50);
        let q = draw_tail(&f, &f, b"abcdef", 0, 20, 1000);
        assert_eq!(q.len(), 6);
        assert_eq!(q[0].byte, b'a');
    }

    #[test]
    fn the_line_count_wraps_on_spaces_and_carriage_returns() {
        let f = tens();
        assert_eq!(count_lines(&f, b"", 30), 0);
        assert_eq!(count_lines(&f, b"a", 30), 1);
        assert_eq!(count_lines(&f, b"ab cd ef", 30), 3);
        assert_eq!(count_lines(&f, b"ab\r\ncd", 100), 2);
        assert_eq!(count_lines(&f, b"abcdef", 30), 2);
    }

    #[test]
    fn the_width_fit_keeps_the_line_count() {
        let f = tens();
        // "abc def ghi" at 30 is three lines of 30; the average is 30.
        let mut width = 30;
        assert_eq!(fit_width(&f, b"abc def ghi", &mut width, 4), 3);
        assert_eq!(width, 30);
        let mut width = 100;
        assert_eq!(fit_width(&f, b"abc def", &mut width, 4), 1);
        assert_eq!(width, 65);
        let mut width = 12;
        assert_eq!(fit_width(&f, b"a", &mut width, 4), 1);
        assert_eq!(width, 12);
        // A final carriage return counts a third line: the average is 20,
        // widened by the unit until two lines remain at 32, so the last
        // pass's two lines are returned and the widest, 30, is kept.
        let mut width = 30;
        assert_eq!(fit_width(&f, b"abc def\r", &mut width, 4), 2);
        assert_eq!(width, 30);
        // The used width counts the bytes that fit, the space and the head
        // of the wrapped word included: "ab cdef" at 40 uses 35 (a, b, the
        // space and c) and then 40; at the average, 37, the first line still
        // uses 35 and the second 30, so two lines keep the widest, 35.
        let mut width = 40;
        assert_eq!(fit_width(&f, b"ab cdef", &mut width, 5), 2);
        assert_eq!(width, 35);
    }

    #[test]
    fn a_wrapped_block_stops_at_its_height_and_skips_lines() {
        let f = tens();
        let d = draw_wrapped(&f, b"ab cd ef", 0, 0, 30, 48, 0, false);
        // Three lines fit in 48; the third starts at y 32, below the last
        // full line (the original allows one pitch more).
        assert_eq!(d.lines, 3);
        assert_eq!(d.quads.first().map(|q| q.rect[0]), Some(0));
        // The glyphs hang down from the line's top.
        assert_eq!(d.quads[0].rect[1], 0);
        assert_eq!(d.quads.last().map(|q| q.rect[1]), Some(32));
        // Skipping one line draws the first two at the top; the bottom is
        // fixed at round(40 − 16) = 24, so the third line, at 16, is the
        // last.
        let d = draw_wrapped_at(&f, b"ab cd ef", 0.0, 0.0, 30, 40, 1, false);
        assert_eq!(d.quads[0].rect[1], 0);
        assert_eq!(d.quads[2].rect[1], 0);
        assert_eq!(d.quads.last().map(|q| q.rect[1]), Some(16));
        assert_eq!(d.lines, 3);
        // A many-line text stops at the fixed bottom: lines at 0 and 16.
        let long = b"a b c d e f g h i j k l m n o p q r s t";
        assert_eq!(
            draw_wrapped_at(&f, long, 0.0, 0.0, 10, 40, 0, false).lines,
            2
        );
        // Centring moves the line right by half its rest.
        let d = draw_wrapped(&f, b"ab", 0, 0, 30, 100, 0, true);
        assert_eq!(d.quads[0].rect[0], 5);
    }

    #[test]
    fn every_font_folds_its_own_gap() {
        let mut fonts = [RAYFORM_10; 2];
        add_gap_to_fonts(&mut fonts);
        assert_eq!(
            fonts[0].advance(b'A'),
            RAYFORM_10.advance(b'A') + RAYFORM_10.gap
        );
        assert_eq!(
            fonts[1].advance(b' '),
            RAYFORM_10.advance(b' ') + RAYFORM_10.gap
        );
    }

    #[test]
    fn a_gap_folds_into_every_advance() {
        let mut advances = [1i16; 256];
        add_gap(&mut advances, 6);
        assert_eq!(advances[0], 7);
        assert_eq!(advances[255], 7);
        let mut high = [i16::MAX; 256];
        add_gap(&mut high, 6);
        assert_eq!(high[0], i16::MIN + 5);
    }

    #[test]
    fn the_menu_font_rescales_from_its_design_record() {
        let half = MenuFont::at_scale(0.5);
        assert_eq!(half.glyph, (56, 56));
        assert_eq!(half.ascent, 49);
        assert_eq!(half.gap, 18);
        assert_eq!(half.line_height, 64);
        assert_eq!(half.origin, (8, 16));
        assert_eq!(half.trim, 3);
        assert_eq!(half.advance(b'A'), 38);
        assert_eq!(half.advance(b' '), 18);
        // Only the letter range is rescaled.
        assert_eq!(
            half.advances[b'!' as usize],
            MENU_FONT.advances[b'!' as usize]
        );
        let quarter = MenuFont::at_scale(0.25);
        assert_eq!(quarter.glyph, (28, 28));
        assert_eq!(quarter.advance(b'A'), 19);
        // 70 × 0.25 is 17.5, which rounds to the even 18.
        assert_eq!(quarter.advance(b'C'), 18);
        // A row width counts a space as the gap, then trims.
        assert_eq!(half.row_width(b"A B"), 38 + 18 + 29 - 3);
    }

    #[test]
    fn a_tiny_or_nan_scale_change_rescales() {
        assert!(!needs_rescale(0.5, 0.5009));
        assert!(!needs_rescale(0.5, 0.501));
        assert!(needs_rescale(0.5, 0.502));
        assert!(needs_rescale(f32::NAN, 0.5));
        assert!(needs_rescale(0.5, f32::NAN));
        assert!((menu_font_scale(1.0) - 0.5).abs() < 1e-7);
        assert!((menu_font_scale(0.375) - 0.1875).abs() < 1e-7);
    }

    #[test]
    fn the_menu_line_samples_the_letter_sheet() {
        let half = MenuFont::at_scale(0.5);
        let (quads, pen) = half.draw_line(b"AB C", 100, 60);
        assert_eq!(quads.len(), 3);
        assert_eq!(quads[0].rect, [92, 4, 56, 56]);
        assert_eq!(quads[0].cell, (0.0, 0.875));
        assert_eq!(quads[1].cell, (0.25, 0.875));
        // E is the fifth letter: column 0 of the second row.
        let (quads, _) = half.draw_line(b"E", 0, 0);
        assert_eq!(quads[0].cell, (0.0, 0.75));
        // The space takes the gap and draws nothing.
        assert_eq!(pen, 92 + 38 + 29 + 18 + 35);
    }
}
