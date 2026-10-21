//! Wrapped text, as the original lays it out for its text boxes, help
//! pages and tooltips: lines break at a carriage return, after the last
//! space that fits, or after the byte that overflows a line with no space;
//! an optional inline markup sets colour (`[tcol=RRGGBB]` … `[/tcol]`),
//! italics (`[scrp=i]`), centring (`[algn=2]`) and pictures
//! (`[rimg=NAME]`), and `{%` … `%}` draws a span in italic blue. Caret
//! moves, the pointer's byte and the caret's position follow the same
//! breaks. Text is bytes (Latin-1); positions inside a text are 1-based
//! like the original's strings, and a caret is the count of bytes before
//! it. Rules: `docs/UI.md` (Text boxes).

use crate::gui_font::{GuiFont, round_i32};

/// The byte at 1-based `i`; 0 past the end (the terminator), and before
/// the start.
fn at(text: &[u8], i: i32) -> u8 {
    if i < 1 {
        return 0;
    }
    text.get(i as usize - 1).copied().unwrap_or(0)
}

/// The 1-based index of the `]` that closes a tag opened at `open`, or of
/// the text's second-to-last byte, where the search stops (so a `]` in the
/// last byte is not seen, as in the original). Exarust stops a tag opened
/// in a text's last two bytes at its end, where the original reads on
/// past it.
fn tag_close(text: &[u8], open: i32) -> i32 {
    let len = text.len() as i32;
    let mut m = open + 1;
    loop {
        if at(text, m) == b']' || m == len - 1 || m > len {
            return m.min(len + 1);
        }
        m += 1;
    }
}

/// A tag's content: the bytes between its brackets.
fn tag_content(text: &[u8], open: i32, close: i32) -> &[u8] {
    let from = open as usize;
    let to = ((close - 1).max(open) as usize).min(text.len());
    &text[from.min(to)..to]
}

/// A picture's size by name, for `[rimg=NAME]`.
pub type Pictures<'a> = &'a dyn Fn(&[u8]) -> Option<(u16, u16)>;

/// Where one line ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineScan {
    /// The 1-based index after the break: past the carriage return or the
    /// space that ends the line, past the overflowing byte of a line
    /// without a space, or one past the text.
    pub next: i32,
    /// The width left at the line's last break point (its carriage return
    /// or space, or its end), which a centred line is moved right by half
    /// of; `None` when the line overflowed before any.
    pub rest: Option<i32>,
}

/// Scans the line that starts at 1-based `start` for `width` pixels.
/// With `markup`, a `[` opens a tag that takes no width and is handed to
/// `tag` (its content).
pub fn scan_line(
    text: &[u8],
    start: i32,
    width: i32,
    font: &GuiFont,
    markup: bool,
    tag: &mut dyn FnMut(&[u8]),
) -> LineScan {
    let len = text.len() as i32;
    let mut remaining = width;
    let mut space = 0;
    let mut rest = None;
    let mut j = start;
    let mut next;
    loop {
        next = len + 1;
        if j > len {
            break;
        }
        let c = at(text, j);
        if c == b'\r' {
            next = j + 1;
            rest = Some(remaining);
            break;
        }
        if c == b' ' {
            next = j + 1;
            rest = Some(remaining);
            remaining -= font.advance(c);
            if remaining < 0 {
                break;
            }
            j += 1;
            space = next;
            continue;
        }
        if c == b'[' && markup {
            let close = tag_close(text, j);
            tag(tag_content(text, j, close));
            j = close + 1;
            continue;
        }
        next = space;
        remaining -= font.advance(c);
        if remaining < 0 {
            if next < 1 {
                next = j + 1;
            }
            break;
        }
        j += 1;
    }
    if j > len {
        rest = Some(remaining);
    }
    LineScan { next, rest }
}

/// The scan without markup.
fn plain(text: &[u8], start: i32, width: i32, font: &GuiFont) -> i32 {
    scan_line(text, start, width, font, false, &mut |_| {}).next
}

/// Where the next line starts after a break at `next`: past a line feed.
fn after(text: &[u8], next: i32) -> i32 {
    if at(text, next) == b'\n' {
        next + 1
    } else {
        next
    }
}

/// A line's last index for the caret: one past the text on the last line,
/// else one before the break.
fn line_end(text: &[u8], next: i32) -> i32 {
    let len = text.len() as i32;
    if next == len + 1 { len + 1 } else { next - 1 }
}

/// How many lines a text takes at `width` (markup counts as text): one for
/// a text under two bytes, else one per line, and one more when the
/// second-to-last byte is a carriage return.
pub fn line_count(text: &[u8], width: i32, font: &GuiFont) -> i32 {
    let len = text.len() as i32;
    if len < 2 {
        return 1;
    }
    let mut count = 0;
    let mut start = 1;
    while start <= len {
        start = after(text, plain(text, start, width, font));
        count += 1;
    }
    if at(text, len - 1) == b'\r' {
        count += 1;
    }
    count
}

/// A text's height at `width`: a `pitch` per line, a picture's height
/// (its size over 6, in units) less a pitch for each `[rimg=NAME]`, and a
/// pitch more after a final line feed. `picture` gives a picture's size.
pub fn text_height(
    text: &[u8],
    width: i32,
    pitch: i32,
    font: &GuiFont,
    markup: bool,
    unit: i32,
    picture: Pictures,
) -> i32 {
    let len = text.len() as i32;
    let mut y = 0;
    if len < 1 {
        return y;
    }
    let mut start = 1;
    while start <= len {
        let next = scan_line(text, start, width, font, markup, &mut |_| {}).next;
        let mut k = start;
        while k < next {
            if at(text, k) == b'[' && markup {
                let close = tag_close(text, k);
                let content = tag_content(text, k, close);
                if content.starts_with(b"rimg") {
                    let (_, h) = picture(content.get(5..).unwrap_or(&[])).unwrap_or((0, 0));
                    y = y + i32::from(h / 6) * unit - pitch;
                }
                k = close + 1;
            } else {
                k += 1;
            }
        }
        start = after(text, next);
        y += pitch;
    }
    if at(text, len) == b'\n' {
        y += pitch;
    }
    y
}

/// The caret at the start (`to_end` false) or the end of the line that
/// holds the byte after `caret`; the end stops before the line's break.
/// The caret is kept when no line holds it.
pub fn line_home_end(text: &[u8], width: i32, caret: i32, to_end: bool, font: &GuiFont) -> i32 {
    let len = text.len() as i32;
    let mut start = 1;
    while start <= len {
        let next = plain(text, start, width, font);
        let end = line_end(text, next);
        if start <= caret + 2 && caret < end {
            return if !to_end {
                start - 1
            } else if next == len + 1 {
                len
            } else {
                next - 2
            };
        }
        start = after(text, next);
    }
    caret
}

/// The pen offset of the byte at 1-based `target` on the line from
/// `start` to `next`: the advances before it (line feeds take none), or
/// the line's whole width when `target` is one past the text; 0 when the
/// line does not hold it.
fn pen_before(text: &[u8], start: i32, next: i32, target: i32, font: &GuiFont) -> i32 {
    let mut pen = 0;
    let mut k = start;
    while k < next {
        if at(text, k) != b'\n' {
            if k == target {
                return pen;
            }
            pen += font.advance(at(text, k));
        }
        k += 1;
    }
    if text.len() as i32 + 1 == target {
        pen
    } else {
        0
    }
}

/// The caret on the line from `start` to `next` nearest the pen offset
/// `pen`: before the first byte whose start reaches it, or before the one
/// ahead when that is nearer; before the line's last byte when none does.
fn caret_at_pen(text: &[u8], start: i32, next: i32, pen: i32, font: &GuiFont) -> i32 {
    let end = line_end(text, next);
    if end < start {
        return start - 1;
    }
    let mut acc = 0;
    let mut last = 0;
    let mut best = start - 1;
    let mut k = start;
    loop {
        if at(text, k) != b'\n' {
            if pen <= acc {
                return if 2 * (acc - pen) <= last {
                    k - 1
                } else {
                    k - 2
                };
            }
            last = font.advance(at(text, k));
            acc += last;
            best = k - 1;
        }
        if k >= end {
            return best;
        }
        k += 1;
    }
}

/// A scroll a caret move asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineMove {
    /// The new caret.
    pub caret: i32,
    /// The scroll fraction to set, when the target line left the view.
    pub scroll: Option<f32>,
}

/// The view a caret move keeps the caret in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineView {
    /// The box's line count.
    pub lines: i32,
    /// The lines that fit in the box.
    pub visible: i32,
    /// The scroll fraction.
    pub scroll: f32,
    /// One line's share of the scroll.
    pub step: f32,
}

/// The caret `delta` lines up (negative) or down from `caret`, at the
/// same pen offset (nearest byte), with the scroll that keeps the target
/// line in view: it becomes the first line when at or above the first
/// visible one, or the second-to-last when at or past the last visible
/// one.
pub fn move_lines(
    text: &[u8],
    width: i32,
    caret: i32,
    delta: i32,
    view: LineView,
    font: &GuiFont,
) -> LineMove {
    let len = text.len() as i32;
    let unchanged = LineMove {
        caret,
        scroll: None,
    };
    if len < 1 {
        return unchanged;
    }
    let target = caret + 1;
    let mut found = None;
    let mut index = 0;
    let mut start = 1;
    while start <= len + 1 {
        let next = plain(text, start, width, font);
        let end = line_end(text, next);
        if start - 1 <= target && target <= end {
            found = Some((index, pen_before(text, start, next, target, font)));
            break;
        }
        let following = after(text, next);
        if following <= start {
            break;
        }
        start = following;
        index += 1;
    }
    let Some((index, pen)) = found else {
        return unchanged;
    };
    let line = (index + delta).clamp(0, (view.lines - 1).max(0));
    let first = round_i32(f64::from(view.scroll / view.step));
    let scroll = if line < first + 1 {
        Some(line as f32 * view.step)
    } else if first + view.visible - 1 <= line {
        Some((line - view.visible + 2) as f32 * view.step)
    } else {
        None
    };
    let mut index = 0;
    let mut start = 1;
    while start <= len + 1 {
        let next = plain(text, start, width, font);
        if index == line {
            return LineMove {
                caret: caret_at_pen(text, start, next, pen, font),
                scroll,
            };
        }
        let following = after(text, next);
        if following <= start {
            break;
        }
        start = following;
        index += 1;
    }
    LineMove { caret, scroll }
}

/// The caret under the pointer at `x`, `y` (pixels from the text's
/// origin; the line is `y` over the font's line height, whatever the
/// box's spacing): before the first byte whose middle is right of `x`, else
/// at the line's end (before its break). Past the last line it is the
/// text's end; above the first, 0.
pub fn caret_at(text: &[u8], x: i32, y: i32, width: i32, font: &GuiFont) -> i32 {
    let len = text.len() as i32;
    let x = x.max(0);
    let line = (f64::from(y) / f64::from(font.line_height)) as i32;
    let mut index = 0;
    let mut start = 1;
    while start <= len {
        let next = plain(text, start, width, font);
        if index == line {
            let mut pen: i64 = 0;
            let mut k = start;
            while k < next {
                if at(text, k) != b'\n' {
                    let advance = font.advance(at(text, k));
                    if i64::from(x) < i64::from(advance as u32 >> 1) + pen {
                        return k - 1;
                    }
                    pen += i64::from(advance);
                }
                k += 1;
            }
            return if next == len + 1 { next - 1 } else { next - 2 };
        }
        start = after(text, next);
        index += 1;
    }
    if index <= line { len } else { 0 }
}

/// Title case for a key's name: the first letter of each word upper, the
/// rest lower, underscores turned to spaces (any byte but a letter starts
/// a new word).
pub fn title_case(name: &mut [u8]) {
    let mut word = true;
    for b in name.iter_mut() {
        match *b {
            b'a'..=b'z' => {
                if word {
                    *b -= 0x20;
                    word = false;
                }
            }
            b'A'..=b'Z' => {
                if word {
                    word = false;
                } else {
                    *b += 0x20;
                }
            }
            _ => {
                word = true;
                if *b == b'_' {
                    *b = b' ';
                }
            }
        }
    }
}

/// Replaces each `[inptN]` (one or two digits) with the title-cased name
/// of the key bound to control `N`, which `name` gives.
pub fn expand_bindings(text: &[u8], name: &dyn Fn(usize) -> Vec<u8>) -> Vec<u8> {
    let mut out = text.to_vec();
    let mut i: i32 = 1;
    while i <= out.len() as i32 {
        if at(&out, i) == b'[' && out.get(i as usize..i as usize + 4) == Some(b"inpt".as_slice()) {
            let one = at(&out, i + 5);
            let index = if at(&out, i + 6) == b']' {
                i32::from(one) - 0x30
            } else {
                (i32::from(one) - 0x30) * 10 + i32::from(at(&out, i + 6)) - 0x30
            };
            let mut key = name(index.max(0) as usize);
            title_case(&mut key);
            let close = tag_close(&out, i);
            let tag = (close + 1 - i).max(0) as usize;
            let from = i as usize - 1;
            let to = (from + tag).min(out.len());
            out.splice(from..to, key.iter().copied());
            i += key.len() as i32;
            continue;
        }
        i += 1;
    }
    out
}

/// The 3- or 4-byte colour a tag spells in hex: pairs of hex digits up to
/// six (or eight), stopping at the first other byte; bytes not reached
/// stay 0.
pub fn parse_hex_colour(digits: &[u8], bytes: usize) -> [u8; 4] {
    let mut out = [0u8; 4];
    let mut value: u8 = 0;
    let mut k = 0;
    for (i, &d) in digits.iter().take(bytes * 2).enumerate() {
        let nibble = match d {
            b'0'..=b'9' => d - b'0',
            b'a'..=b'f' => d - b'a' + 10,
            b'A'..=b'F' => d - b'A' + 10,
            _ => break,
        };
        value = value.wrapping_mul(16).wrapping_add(nibble);
        if (k * 2) < i {
            out[k] = value;
            value = 0;
            k += 1;
        }
    }
    out
}

/// The skin colours and fonts a text draw uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextInks {
    /// The text colour, the colour stack's base.
    pub text: [u8; 4],
    /// The alpha a 6-digit `[tcol=…]` takes.
    pub tag_alpha: u8,
    /// The selection colour (its alpha is halved when drawn).
    pub selection: [u8; 4],
    /// The slant `[scrp=i]` and `{%` spans take (the main font's).
    pub slant: i32,
}

/// The colour a `{%` span pushes: (180, 200, 240) at the text's alpha.
pub const SPAN_INK: [u8; 3] = [0xb4, 0xc8, 0xf0];

/// The colour stack: eight entries, as the original's.
const INK_DEPTH: usize = 8;

/// A text's frame: the wrap width, the height drawn into and the line
/// pitch (0 takes the font's line height).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextFrame {
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
    pub markup: bool,
}

/// One glyph quad: its byte, top-left corner, how far its top edge leans
/// right and its colour. The quad is the font's glyph size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphQuad {
    pub byte: u8,
    pub x: i32,
    pub y: i32,
    pub slant: f32,
    pub ink: [u8; 4],
}

impl GlyphQuad {
    /// The glyph's atlas coordinate: the low nibble is the column and the
    /// high nibble the row of the 16×16 sheet, each a sixteenth apart.
    pub fn uv(&self) -> (f32, f32) {
        (
            f32::from(self.byte & 0x0f) * 0.0625,
            f32::from(self.byte >> 4) * 0.0625,
        )
    }

    /// The glyph's four corners at `font`'s glyph size, clockwise from the
    /// top-left. The top edge leans right by `slant`; the bottom edge does
    /// not.
    pub fn corners(&self, font: &GuiFont) -> [[f32; 2]; 4] {
        let (w, h) = font.glyph;
        let (x, y) = (self.x as f32, self.y as f32);
        let (w, h) = (w as f32, h as f32);
        [
            [x + self.slant, y],
            [x + w + self.slant, y],
            [x + w, y + h],
            [x, y + h],
        ]
    }
}

/// An inline picture: its name and rectangle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPicture {
    pub name: Vec<u8>,
    pub rect: [i32; 4],
}

/// What a text draw emits, in order within each list.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextDraw {
    pub glyphs: Vec<GlyphQuad>,
    /// Selection rectangles (x, y, w, h) and their colour.
    pub selections: Vec<([i32; 4], [u8; 4])>,
    pub pictures: Vec<TextPicture>,
}

/// The draw's running state: pen, alignment, slant and colour stack.
struct Pen {
    y: i32,
    align: i32,
    slant: f32,
    inks: [[u8; 4]; INK_DEPTH],
    ink: usize,
}

impl Pen {
    fn new(y: i32, base: [u8; 4]) -> Self {
        Self {
            y,
            align: 0,
            slant: 0.0,
            inks: [base; INK_DEPTH],
            ink: 0,
        }
    }

    fn push(&mut self) {
        self.ink = (self.ink + 1).min(INK_DEPTH - 1);
    }

    fn pop(&mut self) {
        self.ink = self.ink.saturating_sub(1);
    }

    /// A tag met while breaking a line: italics and alignment.
    fn scan_tag(&mut self, content: &[u8], slant: i32) {
        if content.starts_with(b"scrp") {
            self.slant = if content.get(5) == Some(&b'i') {
                slant as f32
            } else {
                0.0
            };
        } else if content.starts_with(b"algn") {
            self.align = i32::from(content.get(5).copied().unwrap_or(0)) - 0x30;
        }
    }
}

/// A tag met while drawing: colours and pictures. A picture is centred in
/// the frame from `x0`, is its size over 6 in units, and moves the pen
/// down by its height less a pitch.
#[allow(clippy::too_many_arguments)]
fn draw_tag(
    pen: &mut Pen,
    content: &[u8],
    x0: i32,
    frame: &TextFrame,
    pitch: i32,
    unit: i32,
    inks: &TextInks,
    picture: Pictures,
    out: &mut TextDraw,
) {
    if content.first() == Some(&b'/') {
        if content.get(1..5) == Some(b"tcol".as_slice()) {
            pen.pop();
        }
    } else if content.starts_with(b"tcol") {
        pen.push();
        let digits = content.get(5..).unwrap_or(&[]);
        let ink = if content.len() == 11 {
            let mut ink = parse_hex_colour(digits, 3);
            ink[3] = inks.tag_alpha;
            ink
        } else {
            let mut ink = parse_hex_colour(digits, 4);
            ink[3] = round_i32(f64::from(f32::from(ink[3]))) as u8;
            ink
        };
        pen.inks[pen.ink] = ink;
    } else if content.starts_with(b"rimg") {
        let name = content.get(5..).unwrap_or(&[]).to_vec();
        let (w, h) = picture(&name).unwrap_or((0, 0));
        let w = i32::from(w / 6) * unit;
        let h = i32::from(h / 6) * unit;
        let x = x0 + (frame.width - w) / 2;
        out.pictures.push(TextPicture {
            name,
            rect: [x, pen.y, w, h],
        });
        pen.y = pen.y + h - pitch;
    }
}

/// The selection's ends, lower first; (-1, -1) when empty.
fn selection_span(selection: (i32, i32)) -> (i32, i32) {
    let (a, b) = selection;
    match a.cmp(&b) {
        std::cmp::Ordering::Less => (a, b),
        std::cmp::Ordering::Greater => (b, a),
        std::cmp::Ordering::Equal => (-1, -1),
    }
}

/// Draws a text into `frame` from `x0`, `y0`, skipping lines before
/// `first_line`: each line's glyphs at the font's advances (a centred line
/// moved right by half its rest), the selection `(anchor, caret)` behind
/// them, and pictures where tags name them. The draw stops at the line
/// that would start below the frame's last pitch.
#[allow(clippy::too_many_arguments)]
pub fn draw_text(
    text: &[u8],
    x0: i32,
    y0: i32,
    first_line: i32,
    selection: (i32, i32),
    frame: &TextFrame,
    font: &GuiFont,
    inks: &TextInks,
    unit: i32,
    picture: Pictures,
) -> TextDraw {
    let mut out = TextDraw::default();
    let len = text.len() as i32;
    let pitch = if frame.pitch == 0 {
        font.line_height
    } else {
        frame.pitch
    };
    let (lo, hi) = selection_span(selection);
    let selection_ink = [
        inks.selection[0],
        inks.selection[1],
        inks.selection[2],
        inks.selection[3] >> 1,
    ];
    let mut pen = Pen::new(y0, inks.text);
    let mut rest = 0;
    let mut selecting = false;
    let mut line = 0;
    let mut start = 1;
    loop {
        if len < start || (frame.height + y0) - pitch < pen.y {
            return out;
        }
        let scan = scan_line(text, start, frame.width, font, frame.markup, &mut |t| {
            pen.scan_tag(t, inks.slant)
        });
        if let Some(r) = scan.rest {
            rest = r;
        }
        let next = scan.next;
        if lo + 1 < next && start <= hi {
            selecting = true;
        }
        if first_line <= line {
            if selecting {
                if hi + 1 < next {
                    selecting = false;
                }
                let mut from = x0 as f32;
                let mut to = 0.0f32;
                let mut x = x0;
                let mut k = start;
                while k < next {
                    let c = at(text, k);
                    if c == b'\n' || c == b'\r' {
                        k += 1;
                        continue;
                    }
                    if c == b'[' && frame.markup {
                        k = tag_close(text, k) + 1;
                    }
                    if lo + 1 == k {
                        from = x as f32;
                    } else if hi + 1 == k {
                        to = x as f32;
                        break;
                    }
                    x += font.advance(at(text, k));
                    k += 1;
                }
                if to == 0.0 {
                    to = x as f32;
                }
                let left = round_i32(f64::from(from));
                let width = round_i32(f64::from(to - from));
                out.selections
                    .push(([left, pen.y + 1, width, pitch], selection_ink));
            }
            let mut x = if pen.align == 2 { x0 + rest / 2 } else { x0 };
            let mut k = start;
            while k < next {
                let c = at(text, k);
                if c == b'\n' {
                    k += 1;
                } else if c == b'[' && frame.markup {
                    let close = tag_close(text, k);
                    draw_tag(
                        &mut pen,
                        tag_content(text, k, close),
                        x0,
                        frame,
                        pitch,
                        unit,
                        inks,
                        picture,
                        &mut out,
                    );
                    k = close + 1;
                } else {
                    if c == b'{' && at(text, k + 1) == b'%' {
                        pen.slant = inks.slant as f32;
                        pen.push();
                        pen.inks[pen.ink] = [SPAN_INK[0], SPAN_INK[1], SPAN_INK[2], inks.text[3]];
                    }
                    out.glyphs.push(GlyphQuad {
                        byte: c,
                        x,
                        y: pen.y,
                        slant: pen.slant,
                        ink: pen.inks[pen.ink],
                    });
                    x += font.advance(c);
                    if c == b'}' && at(text, k - 1) == b'%' {
                        pen.pop();
                        pen.slant = 0.0;
                    }
                    k += 1;
                }
            }
        }
        let next = after(text, next);
        if first_line <= line {
            pen.y += pitch;
        }
        line += 1;
        start = next;
    }
}

/// The caret: a `|` glyph quad at its top-left corner, leaning by the
/// slant in force there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretQuad {
    pub x: i32,
    pub y: i32,
    pub slant: f32,
}

/// The byte whose glyph draws the caret.
pub const CARET_GLYPH: u8 = b'|';

/// Where the caret is drawn: on the line that holds it, half the font's
/// gap left of the pen before the byte after it. Pictures move the pen as
/// in the text draw.
#[allow(clippy::too_many_arguments)]
pub fn caret_quad(
    text: &[u8],
    caret: i32,
    x0: i32,
    y0: i32,
    first_line: i32,
    frame: &TextFrame,
    font: &GuiFont,
    inks: &TextInks,
    unit: i32,
    picture: Pictures,
) -> Option<CaretQuad> {
    let len = text.len() as i32;
    let pitch = if frame.pitch == 0 {
        font.line_height
    } else {
        frame.pitch
    };
    let target = caret + 1;
    let mut pen = Pen::new(y0, inks.text);
    let mut scratch = TextDraw::default();
    let mut rest = 0;
    let mut line = 0;
    let mut start = 1;
    loop {
        if len + 1 < start || (y0 + frame.height) - pitch < pen.y {
            return None;
        }
        let scan = scan_line(text, start, frame.width, font, frame.markup, &mut |t| {
            pen.scan_tag(t, inks.slant)
        });
        if let Some(r) = scan.rest {
            rest = r;
        }
        let next = scan.next;
        if first_line <= line {
            let mut x = if pen.align == 2 { rest / 2 + x0 } else { x0 };
            x -= (font.gap as u32 >> 1) as i32;
            let mut k = start;
            while k < next {
                let c = at(text, k);
                if c == b'\n' {
                    k += 1;
                } else if c == b'[' && frame.markup {
                    let close = tag_close(text, k);
                    draw_tag(
                        &mut pen,
                        tag_content(text, k, close),
                        x0,
                        frame,
                        pitch,
                        unit,
                        inks,
                        picture,
                        &mut scratch,
                    );
                    k = close + 1;
                } else {
                    if c == b'{' && at(text, k + 1) == b'%' {
                        pen.slant = inks.slant as f32;
                        pen.push();
                    }
                    if k == target {
                        break;
                    }
                    x += font.advance(c);
                    if c == b'}' && at(text, k - 1) == b'%' {
                        pen.pop();
                        pen.slant = 0.0;
                    }
                    k += 1;
                }
            }
            if k == target {
                return Some(CaretQuad {
                    x,
                    y: pen.y,
                    slant: pen.slant,
                });
            }
        }
        if len < start {
            return None;
        }
        let next = after(text, next);
        if first_line <= line {
            pen.y += pitch;
        }
        line += 1;
        start = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::{FONTBASE_24, RAYFORM_06};

    /// A font where every byte advances 10, a space 5.
    fn tens() -> GuiFont {
        let mut advances = [10i16; 256];
        advances[b' ' as usize] = 5;
        advances[b'\r' as usize] = 0;
        advances[b'\n' as usize] = 0;
        GuiFont {
            texture: "test",
            glyph: (12, 16),
            ascent: 12,
            slant: 3,
            gap: 6,
            line_height: 16,
            advances,
        }
    }

    fn none(_: &[u8]) -> Option<(u16, u16)> {
        None
    }

    #[test]
    fn lines_break_after_the_last_space_that_fits() {
        let f = tens();
        // "ab cd": a, b (20), space (25), c (35) over 30: the line ends
        // after the space.
        let s = scan_line(b"ab cd", 1, 30, &f, false, &mut |_| {});
        assert_eq!(
            s,
            LineScan {
                next: 4,
                rest: Some(10)
            }
        );
        // No space: the overflowing byte stays on the line.
        let s = scan_line(b"abcdef", 1, 30, &f, false, &mut |_| {});
        assert_eq!(s.next, 5);
        assert_eq!(s.rest, None);
        // A carriage return ends the line, and the scan of the rest runs to
        // one past the text with its width left.
        let s = scan_line(b"ab\r\ncd", 1, 100, &f, false, &mut |_| {});
        assert_eq!(
            s,
            LineScan {
                next: 4,
                rest: Some(80)
            }
        );
        let s = scan_line(b"ab\r\ncd", 5, 100, &f, false, &mut |_| {});
        assert_eq!(
            s,
            LineScan {
                next: 7,
                rest: Some(80)
            }
        );
    }

    #[test]
    fn markup_takes_no_width() {
        let f = tens();
        let mut tags = Vec::new();
        let s = scan_line(b"[tcol=ff0000]abc", 1, 25, &f, true, &mut |t| {
            tags.push(t.to_vec())
        });
        assert_eq!(s.next, 17);
        assert_eq!(tags, vec![b"tcol=ff0000".to_vec()]);
        // Without markup the bracket is text.
        assert_eq!(
            scan_line(b"[tcol=ff0000]abc", 1, 25, &f, false, &mut |_| {}).next,
            4
        );
    }

    #[test]
    fn the_line_count() {
        let f = tens();
        assert_eq!(line_count(b"", 30, &f), 1);
        assert_eq!(line_count(b"a", 30, &f), 1);
        assert_eq!(line_count(b"ab cd ef", 30, &f), 3);
        assert_eq!(line_count(b"ab\r\ncd", 100, &f), 2);
        // A trailing line break opens a line.
        assert_eq!(line_count(b"ab\r\n", 100, &f), 2);
    }

    #[test]
    fn the_text_height() {
        let f = tens();
        assert_eq!(text_height(b"", 30, 16, &f, true, 5, &none), 0);
        assert_eq!(text_height(b"ab cd ef", 30, 16, &f, true, 5, &none), 48);
        assert_eq!(text_height(b"ab\r\n", 100, 16, &f, true, 5, &none), 32);
        // A 60 × 120 picture at unit 5 is 100 tall: it adds 100 less a pitch.
        let pic = |n: &[u8]| (n == b"MAP").then_some((60u16, 120u16));
        assert_eq!(text_height(b"[rimg=MAP]x", 100, 16, &f, true, 5, &pic), 100);
        // A tag closed by the text's last byte loses its last letter, as in
        // the original: this one names "MA".
        assert_eq!(text_height(b"[rimg=MAP]", 100, 16, &f, true, 5, &pic), 0);
    }

    #[test]
    fn home_and_end_of_the_caret_line() {
        let f = tens();
        let t = b"ab cd ef";
        // Line 2 is "cd " (bytes 4..6).
        assert_eq!(line_home_end(t, 30, 4, false, &f), 3);
        assert_eq!(line_home_end(t, 30, 4, true, &f), 5);
        // The last line's end is the text's end.
        assert_eq!(line_home_end(t, 30, 7, true, &f), 8);
        assert_eq!(line_home_end(b"", 30, 0, true, &f), 0);
    }

    #[test]
    fn up_and_down_keep_the_pen() {
        let f = tens();
        let t = b"abc def ghi";
        let view = LineView {
            lines: 3,
            visible: 3,
            scroll: 0.0,
            step: 0.0,
        };
        // From before 'e' (caret 5, pen 10 on "def ") down to before 'h'.
        let m = move_lines(t, 40, 5, 1, view, &f);
        assert_eq!(m.caret, 9);
        // Up from before 'h' to before 'e'.
        assert_eq!(move_lines(t, 40, 9, -1, view, &f).caret, 5);
        // Past the last line clamps to it.
        assert_eq!(move_lines(t, 40, 9, 5, view, &f).caret, 9);
    }

    #[test]
    fn a_move_scrolls_the_target_line_into_view() {
        let f = tens();
        let t = b"abc def ghi jkl mno";
        let view = LineView {
            lines: 5,
            visible: 2,
            scroll: 0.0,
            step: 1.0 / 3.0,
        };
        // Down from line 0 to line 1: the last visible one, so it becomes
        // the second-to-last (line 1 → scroll 1 step).
        let m = move_lines(t, 40, 1, 1, view, &f);
        assert_eq!(m.scroll, Some(1.0 / 3.0));
        // Up to line 0 at scroll 0 sets the scroll to 0.
        assert_eq!(move_lines(t, 40, 5, -1, view, &f).scroll, Some(0.0));
    }

    #[test]
    fn the_pointer_picks_the_nearer_side_of_a_byte() {
        let f = tens();
        let t = b"abc def";
        assert_eq!(caret_at(t, 0, 0, 40, &f), 0);
        assert_eq!(caret_at(t, 4, 0, 40, &f), 0);
        assert_eq!(caret_at(t, 5, 0, 40, &f), 1);
        // Past the line's end: before its break (the space).
        assert_eq!(caret_at(t, 200, 0, 40, &f), 3);
        // Line 2, and past the last line.
        assert_eq!(caret_at(t, 12, 16, 40, &f), 5);
        assert_eq!(caret_at(t, 0, 99, 40, &f), 7);
        assert_eq!(caret_at(t, 0, -40, 40, &f), 0);
    }

    #[test]
    fn key_names_expand_in_title_case() {
        let mut n = b"LEFT_SHIFT".to_vec();
        title_case(&mut n);
        assert_eq!(n, b"Left Shift");
        let names = |i: usize| match i {
            3 => b"SPACE".to_vec(),
            12 => b"mouse_wheel".to_vec(),
            _ => Vec::new(),
        };
        assert_eq!(
            expand_bindings(b"Press [inpt3] now", &names),
            b"Press Space now"
        );
        assert_eq!(expand_bindings(b"[inpt12]!", &names), b"Mouse Wheel!");
    }

    #[test]
    fn hex_colours_stop_at_the_first_other_byte() {
        assert_eq!(parse_hex_colour(b"ff8000]", 3), [0xff, 0x80, 0x00, 0]);
        assert_eq!(parse_hex_colour(b"11223344", 4), [0x11, 0x22, 0x33, 0x44]);
        assert_eq!(parse_hex_colour(b"fff]", 4), [0xff, 0, 0, 0]);
    }

    fn inks() -> TextInks {
        TextInks {
            text: [255, 250, 230, 200],
            tag_alpha: 180,
            selection: [40, 60, 80, 202],
            slant: 3,
        }
    }

    fn frame(width: i32) -> TextFrame {
        TextFrame {
            width,
            height: 10_000,
            pitch: 0,
            markup: true,
        }
    }

    #[test]
    fn glyphs_follow_the_advances_and_colours() {
        let f = tens();
        let d = draw_text(
            b"a[tcol=ff0000]b[/tcol]c",
            7,
            0,
            0,
            (0, 0),
            &frame(100),
            &f,
            &inks(),
            5,
            &none,
        );
        let xs: Vec<_> = d.glyphs.iter().map(|g| (g.byte, g.x, g.ink)).collect();
        assert_eq!(
            xs,
            vec![
                (b'a', 7, [255, 250, 230, 200]),
                (b'b', 17, [255, 0, 0, 180]),
                (b'c', 27, [255, 250, 230, 200]),
            ]
        );
    }

    #[test]
    fn a_span_leans_and_turns_blue() {
        let f = tens();
        let d = draw_text(
            b"{%x%}y",
            0,
            0,
            0,
            (0, 0),
            &frame(100),
            &f,
            &inks(),
            5,
            &none,
        );
        let g: Vec<_> = d.glyphs.iter().map(|g| (g.byte, g.slant, g.ink)).collect();
        let blue = [180, 200, 240, 200];
        assert_eq!(g[0], (b'{', 3.0, blue));
        assert_eq!(g[2], (b'x', 3.0, blue));
        // The closing brace still leans; the byte after it does not.
        assert_eq!(g[4], (b'}', 3.0, blue));
        assert_eq!(g[5], (b'y', 0.0, [255, 250, 230, 200]));
    }

    #[test]
    fn glyph_atlas_coordinates_and_slanted_corners() {
        let f = tens();
        let d = draw_text(b"A", 0, 0, 0, (0, 0), &frame(100), &f, &inks(), 5, &none);
        let mut g = d.glyphs[0];
        // 'A' is 0x41: column 1, row 4 of the 16×16 sheet.
        assert_eq!(g.uv(), (0.0625, 0.25));
        // A slanted glyph (a `[scrp=i]` or `{%` span) leans its top edge
        // right; the bottom does not.
        g.slant = 3.0;
        assert_eq!(
            g.corners(&f),
            [[3.0, 0.0], [15.0, 0.0], [12.0, 16.0], [0.0, 16.0]]
        );
    }

    #[test]
    fn a_centred_line_moves_by_half_its_rest() {
        let f = tens();
        let d = draw_text(
            b"[algn=2]ab",
            0,
            0,
            0,
            (0, 0),
            &frame(100),
            &f,
            &inks(),
            5,
            &none,
        );
        assert_eq!(d.glyphs[0].x, 40);
    }

    #[test]
    fn the_selection_lies_behind_its_bytes() {
        let f = tens();
        // Bytes 1..3 of "abcd" selected (anchor 3, caret 1).
        let d = draw_text(b"abcd", 0, 0, 0, (3, 1), &frame(100), &f, &inks(), 5, &none);
        assert_eq!(d.selections, vec![([10, 1, 20, 16], [40, 60, 80, 101])]);
        // Across a break: the first line from the anchor to its end.
        let d = draw_text(b"ab cd", 0, 0, 0, (1, 4), &frame(30), &f, &inks(), 5, &none);
        assert_eq!(d.selections[0].0, [10, 1, 15, 16]);
        assert_eq!(d.selections[1].0, [0, 17, 10, 16]);
    }

    #[test]
    fn a_picture_is_centred_and_pushes_the_text_down() {
        let f = tens();
        let pic = |_: &[u8]| Some((60u16, 60u16));
        let d = draw_text(
            b"[rimg=X]\r\nz",
            0,
            0,
            0,
            (0, 0),
            &frame(100),
            &f,
            &inks(),
            5,
            &pic,
        );
        assert_eq!(d.pictures[0].rect, [25, 0, 50, 50]);
        assert_eq!(d.glyphs.last().map(|g| g.y), Some(50));
    }

    #[test]
    fn the_draw_stops_below_the_frame() {
        let f = tens();
        let tight = TextFrame {
            width: 30,
            height: 40,
            pitch: 0,
            markup: false,
        };
        let d = draw_text(
            b"ab cd ef gh",
            0,
            0,
            0,
            (0, 0),
            &tight,
            &f,
            &inks(),
            5,
            &none,
        );
        assert_eq!(d.glyphs.iter().map(|g| g.y).max(), Some(16));
        // First lines skipped draw nothing and take no height.
        let d = draw_text(
            b"ab cd ef",
            0,
            0,
            1,
            (0, 0),
            &frame(30),
            &f,
            &inks(),
            5,
            &none,
        );
        assert_eq!((d.glyphs[0].byte, d.glyphs[0].y), (b'c', 0));
    }

    #[test]
    fn the_caret_backs_off_half_the_gap() {
        let f = tens();
        let c = caret_quad(b"abc", 2, 4, 0, 0, &frame(100), &f, &inks(), 5, &none);
        assert_eq!(
            c,
            Some(CaretQuad {
                x: 21,
                y: 0,
                slant: 0.0
            })
        );
        let c = caret_quad(b"", 0, 4, 0, 0, &frame(100), &f, &inks(), 5, &none);
        assert_eq!(c.map(|c| c.x), Some(1));
        // At the end of a wrapped line it sits after the line's space.
        let c = caret_quad(b"ab cd", 3, 0, 0, 0, &frame(30), &f, &inks(), 5, &none);
        assert_eq!(c.map(|c| (c.x, c.y)), Some((22, 0)));
        // After a final line feed, on the empty line below.
        let c = caret_quad(b"ab\r\n", 4, 0, 0, 0, &frame(100), &f, &inks(), 5, &none);
        assert_eq!(c.map(|c| (c.x, c.y)), Some((-3, 16)));
    }

    #[test]
    fn real_fonts_wrap_by_their_tables() {
        // "Hello world" in fontbase24r: H 32 e 23 l 13 l 13 o 25 = 106,
        // space 21: a 120-pixel line breaks after the space.
        assert_eq!(
            scan_line(b"Hello world", 1, 120, &FONTBASE_24, false, &mut |_| {}).next,
            7
        );
        assert_eq!(line_count(b"Hello world", 120, &FONTBASE_24), 2);
        assert_eq!(line_count(b"Hello world", 300, &RAYFORM_06), 1);
    }
}
