//! The line skin's own methods.
//!
//! The game draws its interface with a thin-line subclass of the draw skin.
//! Most of that subclass already lives in [`crate::skin`]: its opening and
//! scale, the hit test, the slider and scroll bar, the frame, the chip, the
//! rail, the shade (its border and inner face), the four-unit square, the
//! capped and plain span bars, the label and the widget dispatcher
//! ([`crate::skin::dispatch_form`]). This module holds the methods that were
//! still missing: the five unit lengths and the once-only registration, the
//! line skin's own centred icon, the scroll refresh that places a bar
//! window's thumb, the progress strip a titled window and an open bar draw,
//! the integer span bar of the compact skill screen, and the maximised
//! window's frame.
//!
//! Quads follow [`crate::skin::SliderQuad`]: the first four texture words of
//! the sample and integer corners. Every term the original adds is a whole
//! number, so its float sums are exact while the corners stay below 2^24.

use crate::skin::{
    CappedSpan, IconPlace, SKIN_INK, SPAN_BAR, SPAN_END, SkinInk, SkinPalette, SkinScheme,
    SliderQuad, SpanView, WIDE_BAR, WIDE_CAP, load_scheme, place_span_corner,
};

/// The colour scheme the shipped line skin loads: four colours with their
/// scales and tones (black, black transparent, grey, orange-red), the
/// accent and the two alpha ramps. The original keeps it with the GUI's
/// data, not in a game file.
pub const LINE_SCHEME: SkinScheme = SkinScheme {
    groups: [
        ([0, 0, 0, 224], 1.3, 0.4),
        ([0, 0, 0, 0], 1.4, 0.4),
        ([170, 165, 160, 255], 1.15, 0.0),
        ([200, 90, 10, 196], 1.22, 0.2),
    ],
    accent: [16, 12, 40, 196],
    accent_scale: 0.0,
    first_alpha: 170,
    second_alpha: 125,
};

/// The palette an opened line skin holds: its constructor's ink, then the
/// scheme expanded into the five groups and the accent ramp.
pub fn line_palette() -> SkinPalette {
    let mut palette = SkinPalette::filled(SKIN_INK);
    load_scheme(&mut palette, &LINE_SCHEME);
    palette
}

/// The palette a form draws with: every form's draw hands its fade to the
/// skin as both the alpha and the fade, and the skin scales its alpha ramps
/// by them (the form group to the first alpha byte, 170, times the fade;
/// the control, hot-form, label and highlight groups to the second, 125,
/// times it). A shown form's fade is 1.
pub fn shown_palette(fade: f32) -> SkinPalette {
    let mut palette = line_palette();
    crate::skin::scale_palette_alpha(&mut palette, fade, fade);
    palette
}

/// The line skin's three control inks, the disabled, idle and on ones its
/// slider, frame and check box bind: the constructor's colour 255, 250, 230
/// with the alphas its fade hand-off writes, 80, 104 and 128 times the fade
/// (rounded half to even, low byte).
pub fn line_control_inks(fade: f32) -> [SkinInk; 3] {
    let [r, g, b, _] = SKIN_INK;
    [80.0, 104.0, 128.0].map(|alpha: f32| [r, g, b, round_low_word(alpha * fade) as u8])
}

/// The alpha the line skin's hovered thumb and box take: the fade times
/// 255, rounded half to even, low byte.
pub fn hover_alpha(fade: f32) -> u8 {
    round_low_word(fade * 255.0) as u8
}

/// A form's title ink pairs and text ink read from the line skin's palette.
pub fn line_inks() -> (SkinInk, SkinInk, SkinInk) {
    let palette = line_palette();
    (palette.form[1], palette.text[6], palette.text[4])
}

/// The five lengths the line skin derives from the GUI unit: 2, 1, 2, 2 and
/// 4 units. The parent constructor calls this setter through the object's
/// table, so an opened line skin keeps the fifth length (see
/// [`crate::skin::open_skin`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineMetrics {
    pub span_a: i32,
    pub span_b: i32,
    pub span_c: i32,
    pub span_d: i32,
    pub span_e: i32,
}

/// Sets the five lengths from the unit.
pub fn set_line_metrics(unit: i32) -> LineMetrics {
    LineMetrics {
        span_a: unit.wrapping_shl(1),
        span_b: unit,
        span_c: unit.wrapping_shl(1),
        span_d: unit.wrapping_shl(1),
        span_e: unit.wrapping_shl(2),
    }
}

/// Registers the line unit once and answers whether it is now registered.
pub fn register_line_skin(registered: &mut bool) -> bool {
    *registered = true;
    *registered
}

/// How the line icon's ink takes its alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineIconAlpha {
    /// The shade as stored.
    Stored,
    /// The shade's colour with alpha 255.
    Opaque,
    /// The shade's colour with its alpha raised by 0x40, wrapping in a byte.
    Raised,
}

/// The ink a line icon binds: a shade of the skin's label group, counted
/// from one as `docs/UI.md` counts them, and its alpha rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineIconInk {
    pub shade: u8,
    pub alpha: LineIconAlpha,
}

impl LineIconInk {
    /// The colour bound for this ink, given the label group's twelve shades.
    pub fn colour(self, shades: &[[u8; 4]; 12]) -> [u8; 4] {
        let mut colour = shades[usize::from(self.shade - 1)];
        match self.alpha {
            LineIconAlpha::Stored => {}
            LineIconAlpha::Opaque => colour[3] = 0xff,
            LineIconAlpha::Raised => colour[3] = colour[3].wrapping_add(0x40),
        }
        colour
    }
}

/// Where the line skin's centred icon draws and how. The glyph call itself
/// belongs to the icon manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineIcon {
    pub own: bool,
    pub marked: bool,
    /// Style bit 0x40000000 picks the icon manager's other draw, whose
    /// texture coordinates run right to left.
    pub mirrored: bool,
    /// `None` for a marked icon, which binds no ink.
    pub ink: Option<LineIconInk>,
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub scale_bits: u32,
}

fn centre_spare(span: i16, icon: i32) -> i32 {
    ((i64::from(span) - i64::from(icon)) / 2) as i32
}

/// The line skin's override of the centred icon. It places the icon as the
/// parent does (half the spare, toward zero, with a marked state's bit 0x40
/// one pixel lower) but chooses its own inks: state bit 0x10 the fourth
/// shade as stored; otherwise, with style bit 0x80000000 clear, the eighth
/// shade on state bit 0x40 and the fourth, both forced opaque; with it set,
/// the seventh shade with its alpha raised by 0x40 on state bit 0x40, else
/// the sixth as stored.
pub fn draw_line_icon(icon: i32, place: IconPlace) -> LineIcon {
    let x = i32::from(place.x).wrapping_add(centre_spare(place.w, icon));
    let y = i32::from(place.y).wrapping_add(centre_spare(place.h, icon));
    let mirrored = (place.mode & 0x4000_0000) != 0;
    if (place.state & 0xf_0000) == 0x1_0000 {
        let y = if (place.state & 0x60) == 0x40 {
            y.wrapping_add(1)
        } else {
            y
        };
        return LineIcon {
            own: place.own,
            marked: true,
            mirrored,
            ink: None,
            id: place.id,
            x,
            y,
            scale_bits: 0x3f80_0000,
        };
    }
    let hot = (place.state & 0x40) == 0x40;
    let ink = if (place.state & 0x10) == 0x10 {
        LineIconInk {
            shade: 4,
            alpha: LineIconAlpha::Stored,
        }
    } else if (place.mode & 0x8000_0000) == 0 {
        LineIconInk {
            shade: if hot { 8 } else { 4 },
            alpha: LineIconAlpha::Opaque,
        }
    } else if hot {
        LineIconInk {
            shade: 7,
            alpha: LineIconAlpha::Raised,
        }
    } else {
        LineIconInk {
            shade: 6,
            alpha: LineIconAlpha::Stored,
        }
    };
    LineIcon {
        own: place.own,
        marked: false,
        mirrored,
        ink: Some(ink),
        id: place.id,
        x,
        y,
        scale_bits: 0,
    }
}

/// Converts a float the way the original's 64-bit `CVTSS2SI` stored as a
/// 32-bit word does: round half to even, keep the low 32 bits, and read the
/// indefinite value (a NaN or a magnitude of 2^63 or more) as 0.
fn round_low_word(value: f32) -> i32 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || !(-9.223_372e18..9.223_372e18).contains(&rounded) {
        0
    } else {
        (rounded as i64) as i32
    }
}

/// The DPI border metric the client measure adds under a title (2).
pub const TITLE_GAP: i32 = 2;

/// The base form's client measure with the line skin's lengths (the
/// measure the show's client refresh runs): the left edge is the border; a
/// corner-framed form (style bits 0x30) keeps the skin's first length (two
/// units) as its frame span and a panel-framed one (0xc0) the second (one
/// unit) as its title span; the right edge is the width less the border,
/// less the skin's scroll length (two units) with the vertical bar; the
/// bottom the height less the border (the horizontal bar's case is not
/// taken here). A titled form (attribute bit 1) starts its client at the
/// skin's title length (two units) plus the DPI border metric when its
/// attribute word's 0x800030 bits are exactly 0x20, else plus the title
/// span; an untitled one at the border.
pub fn line_client(
    width: i32,
    height: i32,
    border: i32,
    style: u32,
    attributes: u32,
    unit: i32,
) -> crate::form::ClientRect {
    let span = if style & 0xc0 != 0 { unit } else { 0 };
    let right = if attributes & 0x200 == 0x200 {
        width - border - unit * 2
    } else {
        width - border
    };
    let top = if attributes & 1 == 1 {
        unit * 2
            + if attributes & 0x80_0030 == 0x20 {
                TITLE_GAP
            } else {
                span
            }
    } else {
        border
    };
    crate::form::ClientRect {
        left: border,
        top,
        right,
        bottom: height - border,
    }
}

/// What the scroll refresh reads from a bar window and its owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineScrollPlace {
    /// The style word whose bit 4 makes the window measure its owner.
    pub style: u32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub client_top: i32,
    pub client_bottom: i32,
    /// The owner's room: its right less its left, its bottom less its top.
    pub owner_left: i32,
    pub owner_top: i32,
    pub owner_right: i32,
    pub owner_bottom: i32,
    /// The vertical scroll fraction.
    pub fraction: f32,
}

/// The scroll rail's start, end, side and thumb, as the refresh stores them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineScroll {
    pub mask: u32,
    pub start: i32,
    pub end: i32,
    pub side: i32,
    pub thumb: i32,
}

/// Refreshes a bar window's scroll rail. Without style bit 4 the edge mask
/// is 0xf. Otherwise bit 1 marks a top above the unit, bit 4 a right edge
/// within a unit of the owner's room and bit 8 the same for the bottom; the
/// refresh never sets bit 2 (the dispatcher's own mask does). The start is
/// five units below the client top with bits 1 and 4, else four; the end
/// five or four units above the client bottom with bits 4 and 8; the side
/// is two units; the thumb slides from the start by the fraction of the
/// span less the side.
pub fn refresh_line_scroll(place: LineScrollPlace, unit: i32) -> LineScroll {
    let mask = if (place.style & 4) == 0 {
        0xf
    } else {
        let mut mask = u32::from(place.y < unit);
        let reach_y = i64::from(place.y) + i64::from(place.h) + i64::from(unit);
        if reach_y > i64::from(place.owner_bottom) - i64::from(place.owner_top) {
            mask |= 8;
        }
        let reach_x = i64::from(place.x) + i64::from(place.w) + i64::from(unit);
        if reach_x > i64::from(place.owner_right) - i64::from(place.owner_left) {
            mask |= 4;
        }
        mask
    };
    let start = if (mask & 5) == 5 {
        unit.wrapping_mul(5).wrapping_add(place.client_top)
    } else {
        unit.wrapping_shl(2).wrapping_add(place.client_top)
    };
    let end = if (mask & 0xc) == 0xc {
        place.client_bottom.wrapping_sub(unit.wrapping_mul(5))
    } else {
        place.client_bottom.wrapping_sub(unit.wrapping_shl(2))
    };
    let side = unit.wrapping_shl(1);
    // The span is a 32-bit difference; the side comes off in 64 bits.
    let span = i64::from(end.wrapping_sub(start)) - i64::from(side);
    let slide = round_low_word(span as f32 * place.fraction);
    LineScroll {
        mask,
        start,
        end,
        side,
        thumb: start.wrapping_add(slide),
    }
}

/// The first four texture words of the 0.2, 0.1, 0.2, 0.2 sample: the
/// progress strip's caps.
pub const LINE_STRIP_CAP: [u32; 4] = [0x3e4c_cccd, 0x3dcc_cccd, 0x3e4c_cccd, 0x3e4c_cccd];

/// The first four texture words of the 0.15, 0.2, 0.15, 0.5 sample: the
/// maximised window's corner arms.
pub const WINDOW_CORNER: [u32; 4] = [0x3e19_999a, 0x3e4c_cccd, 0x3e19_999a, 0x3f00_0000];

fn quad64(sample: [u32; 4], corners: [(i64, i64); 4]) -> SliderQuad {
    SliderQuad {
        sample,
        corners: corners.map(|(x, y)| (x as i32, y as i32)),
    }
}

/// The progress strip along a window's top: a cap at each end eight units
/// in, the track between them, and, for a thumb past zero, the track split
/// at the thumb and its mirror with a cap on the inner side of each. The
/// strip runs from the skin's first side length down to its third; both
/// callers draw it with 2 and 4 units.
pub fn draw_line_progress(
    unit: i32,
    first: i32,
    third: i32,
    width: i32,
    thumb: i32,
) -> Vec<SliderQuad> {
    let eight = i64::from(unit.wrapping_shl(3));
    let (f, t, w, k) = (
        i64::from(first),
        i64::from(third),
        i64::from(width),
        i64::from(thumb),
    );
    let mut quads = vec![
        quad64(LINE_STRIP_CAP, [(t, f), (t, t), (eight, t), (eight, f)]),
        quad64(
            LINE_STRIP_CAP,
            [(w - t, f), (w - t, t), (w - eight, t), (w - eight, f)],
        ),
    ];
    if thumb < 1 {
        quads.push(quad64(
            WIDE_BAR,
            [(eight, f), (eight, t), (w - eight, t), (w - eight, f)],
        ));
    } else {
        quads.push(quad64(WIDE_BAR, [(eight, f), (eight, t), (k, t), (k, f)]));
        quads.push(quad64(
            WIDE_BAR,
            [(w - eight, f), (w - eight, t), (w - k, t), (w - k, f)],
        ));
        quads.push(quad64(WIDE_CAP, [(k, f), (k, t), (k + f, t), (k + f, f)]));
        quads.push(quad64(
            WIDE_CAP,
            [(w - k, f), (w - k, t), (w - k - f, t), (w - k - f, f)],
        ));
    }
    quads
}

/// The compact skill screen's span bar: the capped span drawn from integer
/// arguments with the shared skin and no ink of its own. `y` is the top;
/// the caps run four units down from it and the middle starts two units
/// down. The corners pass through the view as the other span bars do.
pub fn draw_open_span(unit: i32, x: i32, y: i32, width: i32, view: SpanView) -> CappedSpan {
    let quadrupled = i64::from(unit.wrapping_shl(2));
    let back = i64::from(unit.wrapping_mul(-2));
    let x64 = i64::from(x);
    let y64 = i64::from(y);
    // The right edge is a 32-bit sum.
    let right = i64::from(width.wrapping_add(x));
    let top = y64 as f32;
    let bottom = (y64 + quadrupled) as f32;
    let middle = (y64 + i64::from(unit) * 2) as f32;
    let left_out = (x64 + back) as f32;
    let left_in = (x64 + quadrupled) as f32;
    let right_out = (right - back) as f32;
    let right_in = (right - quadrupled) as f32;
    let quads = [
        [
            (left_out, top),
            (left_out, bottom),
            (left_in, bottom),
            (left_in, top),
        ],
        [
            (right_out, top),
            (right_out, bottom),
            (right_in, bottom),
            (right_in, top),
        ],
        [
            (left_in, middle),
            (left_in, bottom),
            (right_in, bottom),
            (right_in, middle),
        ],
    ];
    let placed = quads.map(|quad| quad.map(|(px, py)| place_span_corner(px, py, view)));
    CappedSpan {
        mode_calls: 1,
        mode: 2,
        begin_calls: 1,
        prep_calls: 1,
        ink_binds: 0,
        bind_calls: 3,
        binds: [SPAN_END, SPAN_END, SPAN_BAR],
        quads,
        placed,
        colour: view.colour,
        colour_writes: 12,
        end_calls: 1,
    }
}

/// What the maximised window frame reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowFramePlace {
    /// The style word: bit 4 makes the frame bind its own ink and test the
    /// owner, bit 0x40 picks the hot ink.
    pub style: u32,
    /// The window's top within its owner.
    pub y: i32,
    pub owner_state: u32,
    pub width: i32,
    pub height: i32,
}

/// The ink the maximised frame binds before its quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFrameInk {
    Normal,
    Hot,
}

/// The maximised window's frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowFrame {
    /// `None` when the style has no bit 4 and the dispatcher's ink stays.
    pub ink: Option<WindowFrameInk>,
    pub quads: Vec<SliderQuad>,
}

/// Draws a maximised window's frame: the doubled top bar, the left and
/// right bars and the four corner arms around them, and the bottom bar. The
/// side lengths are 0 and two units with style bit 4 (which also binds the
/// ink and draws the outer top bar unless the window sits within a unit of
/// its owner's top while the owner is focused), else 0 and four units. The
/// arms reach two units outside and six in. The skin's side lengths go back
/// to two and four units afterwards.
pub fn draw_window_frame(unit: i32, place: WindowFramePlace) -> WindowFrame {
    let two = i64::from(unit) << 1;
    let six = (i64::from(unit) << 2) + two;
    let held = (place.style & 4) != 0;
    let third = if held { two } else { i64::from(unit) << 2 };
    let w = i64::from(place.width);
    let h = i64::from(place.height);
    let mut quads = Vec::with_capacity(9);
    let ink = if held {
        if place.y >= unit || (place.owner_state & 1) == 0 {
            quads.push(quad64(
                WIDE_BAR,
                [(six, 0), (six, two), (w - six, two), (w - six, 0)],
            ));
        }
        Some(if (place.style & 0x40) == 0 {
            WindowFrameInk::Normal
        } else {
            WindowFrameInk::Hot
        })
    } else {
        None
    };
    quads.push(quad64(
        WIDE_BAR,
        [(six, 0), (six, two), (w - six, two), (w - six, 0)],
    ));
    quads.push(quad64(
        WIDE_BAR,
        [(0, h - third), (two, h - third), (two, third), (0, third)],
    ));
    quads.push(quad64(
        WINDOW_CORNER,
        [(-two, -two), (-two, third), (six, third), (six, -two)],
    ));
    quads.push(quad64(
        WINDOW_CORNER,
        [
            (w + two, -two),
            (w + two, third),
            (w - six, third),
            (w - six, -two),
        ],
    ));
    quads.push(quad64(
        WIDE_BAR,
        [
            (w, h - third),
            (w - two, h - third),
            (w - two, third),
            (w, third),
        ],
    ));
    quads.push(quad64(
        WINDOW_CORNER,
        [
            (-two, h + two),
            (-two, h - third),
            (six, h - third),
            (six, h + two),
        ],
    ));
    quads.push(quad64(
        WIDE_BAR,
        [(six, h), (six, h - two), (w - six, h - two), (w - six, h)],
    ));
    quads.push(quad64(
        WINDOW_CORNER,
        [
            (w + two, h + two),
            (w + two, h - third),
            (w - six, h - third),
            (w - six, h + two),
        ],
    ));
    WindowFrame { ink, quads }
}

/// The form attribute, style and state bits the line skin's form draw
/// reads.
pub mod form_bits {
    /// The title bar.
    pub const TITLE: u32 = 1;
    /// The close mark.
    pub const CLOSE: u32 = 2;
    /// A drop shadow around the form.
    pub const SHADOW: u32 = 0x40;
    /// The vertical scroll bar.
    pub const SCROLL: u32 = 0x200;
    /// The backdrop: one quad over the whole form.
    pub const BACKDROP: u32 = 0x2_0000;
    /// The wide frame (the bars outside the form's edges).
    pub const WIDE: u32 = 0x40_0000;
    /// An attribute bit that keeps the title strip off a titled form, and
    /// on an owner lets a child draw its top joints and bar anyway.
    pub const OPEN_TOP: u32 = 0x80_0000;
    /// The state bit of a form whose owner is another form, not a task
    /// (the commit of the words sets it).
    pub const HELD: u32 = 4;
    /// The state bit that picks the hot ink.
    pub const HOT: u32 = 0x40;
    /// The frame style bits: 0x10 the corner frame, 0x80 the panel frame.
    pub const CORNER_STYLE: u32 = 0x30;
    pub const PANEL_STYLE: u32 = 0xc0;
}

/// What the line skin's form draw reads of one form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineFormPlace {
    pub width: i32,
    pub height: i32,
    /// The form's place in its owner's client.
    pub x: i32,
    pub y: i32,
    /// The attribute word ([`form_bits`]).
    pub attributes: u32,
    /// The frame style word.
    pub style: u32,
    /// The state word.
    pub state: u32,
    /// The owner form's attribute word.
    pub owner_attributes: u32,
    /// The owner's client width and height (the room a held panel tests
    /// its right and bottom edges against).
    pub owner_client: (i32, i32),
    /// The title's measured width (read only with the title bit).
    pub title_width: i32,
    /// The mark under the pointer (-4 the close mark, which then draws in
    /// the hover colour after the frame).
    pub hover_mark: i32,
    /// The vertical scroll bar's place, read for a panel with the scroll
    /// bar attribute (0x200).
    pub scroll: Option<LineFormScroll>,
}

/// What the panel frame's scroll-bar branch reads of a form: its client's
/// top, right and bottom edges (form-local), the thumb's place the line
/// skin's scroll refresh stores ([`refresh_line_scroll`]) and the form's
/// option word, whose bit 0x200 keeps the rail outside a free right edge
/// off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineFormScroll {
    pub client_top: i32,
    pub client_right: i32,
    pub client_bottom: i32,
    pub thumb: i32,
    pub options: u32,
}

/// The ink the frame quads take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineFormInk {
    /// The skin's idle control ink (255, 250, 230 at 104 × fade).
    Idle,
    /// The highlight group's eighth shade.
    Hot,
}

/// What the line skin's form draw emits, in its order: the shadow and the
/// backdrop (before the form's content and children), then the title and
/// the frame quads from the skin atlas (after them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineForm {
    /// The drop shadow: the rounded program's ring eight units out
    /// ([`crate::skin::rounded_textured_frame`] with 8), in black at the
    /// form group's fourth shade's alpha.
    pub shadow: bool,
    /// The backdrop quad: solid, in the form group's fourth shade, its
    /// corners (0, 0), (0, h), (w, h), (w, 0).
    pub backdrop: Option<[(i32, i32); 4]>,
    /// The title's x (centred in the form's width); its baseline is the
    /// main font's ascent and its ink the label group's seventh shade.
    pub title_x: Option<i32>,
    /// The ink of the panel frame's quads (the corner frame's are idle).
    pub ink: LineFormInk,
    /// The frame quads from the skin atlas, in drawing order.
    pub frame: Vec<SliderQuad>,
    /// The panel frame's edge mask: 1 top, 2 left, 4 right, 8 bottom for a
    /// held panel within a unit of its owner's client edge (all four for a
    /// free one).
    pub edges: u32,
    /// False when the form takes a frame branch that is not ported here
    /// (the wide frame, or a scroll bar with no place given); its frame
    /// quads are then missing.
    pub complete: bool,
    /// The frame quads drawn in the hover ink ([`hover_ink`]) instead of
    /// the frame's: the scroll bar's upper arrow under the mark -9, its
    /// lower arrow under -10 and its thumb under -8.
    pub hot: Vec<usize>,
    /// The quads the form draw adds after the frame, in the hover ink: the
    /// close mark under the mark -4.
    pub late: Vec<SliderQuad>,
}

/// The hover ink of the line skin's frame marks: 255, 204, 153 at the
/// GUI's alpha byte (the fade times 255).
pub fn hover_ink(fade: f32) -> SkinInk {
    [255, 204, 153, hover_alpha(fade)]
}

/// The line skin's form draw (the method the game's skin puts in the
/// form-draw slot): the shadow and the backdrop, the centred title, the
/// corner frame of a top-level form (style 0x10: ornamented six-unit
/// corners, the close mark, the edge bars two units thick with a gap for
/// the title, and the title strip under it) and the panel frame of a child
/// (style 0x80). A free panel's frame runs two units in from each edge
/// (the strip on top, bars down the sides and along the bottom, two joint
/// pieces at each corner); a held one (owned by a form) lies on its edges,
/// and each corner within a unit of the owner's client edges on both sides
/// closes with joints, any other with the curled corner piece two units
/// outside. Every term the original adds is a whole number, so the float
/// sums are exact.
pub fn draw_line_form(unit: i32, place: LineFormPlace) -> LineForm {
    use crate::skin::{
        FORM_ALT, FORM_CAP, FORM_CLOSE_INK, FORM_CORNER, FORM_EDGE, HELD_END, HELD_FLIP, HELD_OVER,
        HELD_POST, HELD_RIGHT, HELD_TAB, WIDE_CAP,
    };
    use form_bits::*;
    let held = place.state & HELD != 0;
    let backdrop = (place.attributes & BACKDROP != 0 && !held).then_some([
        (0, 0),
        (0, place.height),
        (place.width, place.height),
        (place.width, 0),
    ]);
    let titled = place.attributes & TITLE == TITLE;
    let measured = if titled { place.title_width } else { 0 };
    let mut form = LineForm {
        shadow: place.attributes & SHADOW == SHADOW,
        backdrop,
        title_x: (measured > 0)
            .then(|| ((i64::from(place.width) - i64::from(measured)) / 2) as i32),
        ink: LineFormInk::Idle,
        frame: Vec::new(),
        edges: 0,
        complete: true,
        hot: Vec::new(),
        late: Vec::new(),
    };
    let u = i64::from(unit);
    let (w, h) = (i64::from(place.width), i64::from(place.height));
    if place.style & CORNER_STYLE == 0x10 {
        if place.attributes & WIDE != 0 {
            form.complete = false;
        } else {
            let six = u * 6;
            let four = u << 2;
            let frame = &mut form.frame;
            frame.push(quad64(
                FORM_CORNER,
                [(0, 0), (0, six), (six, six), (six, 0)],
            ));
            let close = place.attributes & CLOSE == CLOSE;
            if close && place.hover_mark != -4 {
                frame.push(quad64(
                    FORM_CLOSE_INK,
                    [(w, 0), (w - four, 0), (w - four, four), (w, four)],
                ));
            }
            if close && place.hover_mark == -4 {
                form.late.push(quad64(
                    FORM_CLOSE_INK,
                    [(w, 0), (w - four, 0), (w - four, four), (w, four)],
                ));
            }
            let frame = &mut form.frame;
            frame.push(quad64(
                if close { FORM_ALT } else { FORM_CORNER },
                [(w, 0), (w - six, 0), (w - six, six), (w, six)],
            ));
            frame.push(quad64(
                FORM_CORNER,
                [(0, h), (six, h), (six, h - six), (0, h - six)],
            ));
            frame.push(quad64(
                FORM_CORNER,
                [(w, h), (w, h - six), (w - six, h - six), (w - six, h)],
            ));
            let two = u << 1;
            let reach = (f64::from(unit) * 6.6).round_ties_even() as i64;
            let mut thumb = measured;
            if measured < 1 {
                frame.push(quad64(
                    FORM_EDGE,
                    [(reach, 0), (reach, two), (w - reach, two), (w - reach, 0)],
                ));
            } else {
                let gap = (w - i64::from(measured) - u * 5) / 2;
                thumb = gap as i32;
                frame.push(quad64(
                    FORM_EDGE,
                    [(reach, 0), (reach, two), (gap, two), (gap, 0)],
                ));
                frame.push(quad64(
                    FORM_EDGE,
                    [
                        (w - gap, 0),
                        (w - gap, two),
                        (w - reach, two),
                        (w - reach, 0),
                    ],
                ));
                frame.push(quad64(
                    FORM_CAP,
                    [(gap, 0), (gap, two), (gap + two, two), (gap + two, 0)],
                ));
                frame.push(quad64(
                    FORM_CAP,
                    [
                        (w - gap, 0),
                        (w - gap, two),
                        (w - gap - two, two),
                        (w - gap - two, 0),
                    ],
                ));
            }
            frame.push(quad64(
                FORM_EDGE,
                [(0, h - reach), (two, h - reach), (two, reach), (0, reach)],
            ));
            frame.push(quad64(
                FORM_EDGE,
                [
                    (reach, h),
                    (reach, h - two),
                    (w - reach, h - two),
                    (w - reach, h),
                ],
            ));
            frame.push(quad64(
                FORM_EDGE,
                [
                    (w, h - reach),
                    (w - two, h - reach),
                    (w - two, reach),
                    (w, reach),
                ],
            ));
            if titled && place.attributes & OPEN_TOP == 0 {
                frame.extend(draw_line_progress(
                    unit,
                    unit.wrapping_shl(1),
                    unit.wrapping_shl(2),
                    place.width,
                    thumb,
                ));
            }
        }
    }
    if place.style & PANEL_STYLE != 0x80 {
        return form;
    }
    let scroll = if place.attributes & SCROLL != 0 {
        match place.scroll {
            Some(scroll) => Some(scroll),
            None => {
                form.complete = false;
                return form;
            }
        }
    } else {
        None
    };
    if place.attributes & WIDE != 0 {
        form.complete = false;
        return form;
    }
    // The free panel lies two units in, the held one on its edges.
    let (near, far, inner, outer) = if held {
        let mut edges = 0;
        if place.y < unit {
            edges |= 1;
        }
        if place.x < unit {
            edges |= 2;
        }
        if i64::from(place.owner_client.0) < u + i64::from(place.x) + w {
            edges |= 4;
        }
        if i64::from(place.owner_client.1) < u + i64::from(place.y) + h {
            edges |= 8;
        }
        form.edges = edges;
        (0, u << 1, u << 1, (u << 2) + (u << 1))
    } else {
        form.edges = 0xf;
        (u << 1, u << 2, u << 2, u << 3)
    };
    let edges = form.edges;
    let owner_open = place.owner_attributes & OPEN_TOP != 0;
    let owner_titled = place.owner_attributes & TITLE == TITLE;
    if held {
        if place.state & HOT != 0 {
            form.ink = LineFormInk::Hot;
        }
        if edges & 1 == 0 || !owner_titled || owner_open {
            form.frame.push(quad64(
                WIDE_BAR,
                [
                    (outer, near),
                    (outer, far),
                    (w - outer, far),
                    (w - outer, near),
                ],
            ));
        }
    } else if !titled {
        form.frame.extend(draw_line_progress(
            unit,
            unit.wrapping_shl(1),
            unit.wrapping_shl(2),
            place.width,
            0,
        ));
    }
    // The scroll bar: the two arrows a unit past the client's right edge
    // (and 0.15 of a unit, rounded), two or three units in from the
    // client's top and bottom, and the thumb at its stored place.
    if let Some(bar) = scroll {
        let x = i64::from(bar.client_right) + (f64::from(unit) * 0.15).round_ties_even() as i64;
        let up = i64::from(bar.client_top) + if edges & 5 == 5 { u * 3 } else { u * 2 };
        if place.hover_mark == -9 {
            form.hot.push(form.frame.len());
        }
        form.frame.push(quad64(
            HELD_TAB,
            [
                (u + x, u * 2 + up),
                (u + x, up),
                (u * 3 + x, up),
                (u * 3 + x, u * 2 + up),
            ],
        ));
        let down = i64::from(bar.client_bottom) - if edges & 0xc == 0xc { u * 3 } else { u * 2 };
        if place.hover_mark == -10 {
            form.hot.push(form.frame.len());
        }
        form.frame.push(quad64(
            HELD_TAB,
            [
                (u + x, down - u * 2),
                (u + x, down),
                (u * 3 + x, down),
                (u * 3 + x, down - u * 2),
            ],
        ));
        let thumb = i64::from(bar.thumb) - u;
        if place.hover_mark == -8 {
            form.hot.push(form.frame.len());
        }
        form.frame.push(quad64(
            HELD_POST,
            [
                (x, u * 4 + thumb),
                (x, thumb),
                (u * 4 + x, thumb),
                (u * 4 + x, u * 4 + thumb),
            ],
        ));
    }
    let top_pair = (!titled && !owner_titled) || owner_open;
    let frame = &mut form.frame;
    frame.push(quad64(
        WIDE_BAR,
        [
            (near, h - outer),
            (inner, h - outer),
            (inner, outer),
            (near, outer),
        ],
    ));
    if edges & 3 == 3 {
        frame.push(quad64(
            LINE_STRIP_CAP,
            [(near, inner), (far, inner), (far, outer), (near, outer)],
        ));
        if top_pair {
            frame.push(quad64(
                LINE_STRIP_CAP,
                [(inner, near), (inner, far), (outer, far), (outer, near)],
            ));
        }
    } else {
        frame.push(quad64(
            HELD_FLIP,
            [
                (-inner, -inner),
                (-inner, outer),
                (outer, outer),
                (outer, -inner),
            ],
        ));
    }
    if edges & 5 == 5 && scroll.is_some() {
        frame.push(quad64(
            HELD_END,
            [
                (w - near, outer),
                (w - near, inner),
                (w - u * 6, inner),
                (w - u * 6, outer),
            ],
        ));
    } else if edges & 5 == 5 {
        frame.push(quad64(
            LINE_STRIP_CAP,
            [
                (w - near, inner),
                (w - far, inner),
                (w - far, outer),
                (w - near, outer),
            ],
        ));
        if top_pair {
            frame.push(quad64(
                LINE_STRIP_CAP,
                [
                    (w - inner, near),
                    (w - inner, far),
                    (w - outer, far),
                    (w - outer, near),
                ],
            ));
        }
    } else {
        frame.push(quad64(
            if scroll.is_some() {
                HELD_OVER
            } else {
                HELD_FLIP
            },
            [
                (w + inner, -inner),
                (w + inner, outer),
                (w - outer, outer),
                (w - outer, -inner),
            ],
        ));
    }
    if let Some(bar) = scroll {
        // The right bar moves in beside the scroll bar; a right edge clear
        // of the owner's client gets the rail outside it, unless the
        // option word's bit 0x200 is set.
        frame.push(quad64(
            HELD_RIGHT,
            [
                (w - far, outer),
                (w - far, h - outer),
                (w - far - u * 2, h - outer),
                (w - far - u * 2, outer),
            ],
        ));
        if edges & 4 == 0 && bar.options & 0x200 == 0 {
            let four = u * 4;
            let rail = w + (f64::from(unit) * 0.2).round_ties_even() as i64;
            frame.push(quad64(
                WIDE_BAR,
                [
                    (u + rail, h - four),
                    (rail - u, h - four),
                    (rail - u, four),
                    (u + rail, four),
                ],
            ));
            frame.push(quad64(
                WIDE_CAP,
                [
                    (u + rail, four),
                    (rail - u, four),
                    (rail - u, near),
                    (u + rail, near),
                ],
            ));
            frame.push(quad64(
                WIDE_CAP,
                [
                    (u + rail, h - four),
                    (rail - u, h - four),
                    (rail - u, h - near),
                    (u + rail, h - near),
                ],
            ));
        }
    } else {
        frame.push(quad64(
            WIDE_BAR,
            [
                (w - near, h - outer),
                (w - inner, h - outer),
                (w - inner, outer),
                (w - near, outer),
            ],
        ));
    }
    if edges & 10 == 10 {
        frame.push(quad64(
            LINE_STRIP_CAP,
            [
                (near, h - inner),
                (far, h - inner),
                (far, h - outer),
                (near, h - outer),
            ],
        ));
        frame.push(quad64(
            LINE_STRIP_CAP,
            [
                (inner, h - near),
                (inner, h - far),
                (outer, h - far),
                (outer, h - near),
            ],
        ));
    } else {
        frame.push(quad64(
            HELD_FLIP,
            [
                (-inner, h + inner),
                (-inner, h - outer),
                (outer, h - outer),
                (outer, h + inner),
            ],
        ));
    }
    frame.push(quad64(
        WIDE_BAR,
        [
            (outer, h - near),
            (outer, h - inner),
            (w - outer, h - inner),
            (w - outer, h - near),
        ],
    ));
    if edges & 0xc == 0xc {
        if scroll.is_some() {
            frame.push(quad64(
                HELD_END,
                [
                    (w - near, h - outer),
                    (w - near, h - inner),
                    (w - near - u * 4, h - inner),
                    (w - near - u * 4, h - outer),
                ],
            ));
        } else {
            frame.push(quad64(
                LINE_STRIP_CAP,
                [
                    (w - near, h - inner),
                    (w - far, h - inner),
                    (w - far, h - outer),
                    (w - near, h - outer),
                ],
            ));
        }
        frame.push(quad64(
            LINE_STRIP_CAP,
            [
                (w - inner, h - near),
                (w - inner, h - far),
                (w - outer, h - far),
                (w - outer, h - near),
            ],
        ));
    } else {
        frame.push(quad64(
            if scroll.is_some() {
                HELD_OVER
            } else {
                HELD_FLIP
            },
            [
                (w + inner, h + inner),
                (w + inner, h - outer),
                (w - outer, h - outer),
                (w - outer, h + inner),
            ],
        ));
    }
    form
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::SPAN_VIEW_UNSET;

    #[test]
    fn the_line_unit_has_five_lengths_and_a_once_only_registration() {
        assert_eq!(
            set_line_metrics(5),
            LineMetrics {
                span_a: 10,
                span_b: 5,
                span_c: 10,
                span_d: 10,
                span_e: 20,
            }
        );
        let mut registered = false;
        assert!(register_line_skin(&mut registered));
        assert!(registered);
        assert!(register_line_skin(&mut registered));
    }

    fn icon_place(mode: u32, state: u32) -> IconPlace {
        IconPlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            mode,
            state,
            id: 7,
            own: false,
        }
    }

    #[test]
    fn the_line_icon_centres_like_the_parent_and_picks_its_own_inks() {
        let plain = draw_line_icon(16, icon_place(0, 0));
        assert_eq!((plain.x, plain.y), (52, 52));
        assert!(!plain.marked && !plain.mirrored);
        assert_eq!(
            plain.ink,
            Some(LineIconInk {
                shade: 4,
                alpha: LineIconAlpha::Opaque
            })
        );
        assert_eq!(plain.scale_bits, 0);

        let hot = draw_line_icon(16, icon_place(0, 0x40));
        assert_eq!(hot.ink.map(|ink| ink.shade), Some(8));
        // A hot unmarked icon is not nudged.
        assert_eq!(hot.y, 52);

        let focus = draw_line_icon(16, icon_place(0x8000_0000, 0x50));
        assert_eq!(
            focus.ink,
            Some(LineIconInk {
                shade: 4,
                alpha: LineIconAlpha::Stored
            })
        );

        let raised = draw_line_icon(16, icon_place(0x8000_0000, 0x40));
        assert_eq!(
            raised.ink,
            Some(LineIconInk {
                shade: 7,
                alpha: LineIconAlpha::Raised
            })
        );
        let dim = draw_line_icon(16, icon_place(0x8000_0000, 0));
        assert_eq!(dim.ink.map(|ink| ink.shade), Some(6));

        let marked = draw_line_icon(16, icon_place(0x4000_0000, 0x1_0040));
        assert!(marked.marked && marked.mirrored);
        assert_eq!(marked.ink, None);
        assert_eq!((marked.x, marked.y), (52, 53));
        assert_eq!(marked.scale_bits, 0x3f80_0000);
        // Bits 0x60 together do not nudge.
        assert_eq!(draw_line_icon(16, icon_place(0, 0x1_0060)).y, 52);
        // An odd negative spare halves toward zero.
        assert_eq!(draw_line_icon(105, icon_place(0, 0)).x, 8);
    }

    #[test]
    fn the_raised_ink_wraps_its_alpha_in_a_byte() {
        let mut shades = [[0u8; 4]; 12];
        shades[6] = [1, 2, 3, 0xd0];
        shades[3] = [4, 5, 6, 7];
        let raised = LineIconInk {
            shade: 7,
            alpha: LineIconAlpha::Raised,
        };
        assert_eq!(raised.colour(&shades), [1, 2, 3, 0x10]);
        let opaque = LineIconInk {
            shade: 4,
            alpha: LineIconAlpha::Opaque,
        };
        assert_eq!(opaque.colour(&shades), [4, 5, 6, 0xff]);
    }

    fn scroll_place() -> LineScrollPlace {
        LineScrollPlace {
            style: 4,
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            client_top: 30,
            client_bottom: 90,
            owner_left: 0,
            owner_top: 0,
            owner_right: 400,
            owner_bottom: 300,
            fraction: 0.5,
        }
    }

    #[test]
    fn the_scroll_refresh_places_the_thumb_from_the_owner_room() {
        // Room to spare: start 30 + 20, end 90 - 20, thumb 50 + (20 - 10) / 2.
        let scroll = refresh_line_scroll(scroll_place(), 5);
        assert_eq!(
            scroll,
            LineScroll {
                mask: 0,
                start: 50,
                end: 70,
                side: 10,
                thumb: 55,
            }
        );

        // Without bit 4 the mask is 0xf: start 30 + 25, end 90 - 25, and
        // the span 10 less the side is 0.
        let plain = refresh_line_scroll(
            LineScrollPlace {
                style: 0,
                ..scroll_place()
            },
            5,
        );
        assert_eq!((plain.mask, plain.start, plain.end), (0xf, 55, 65));
        assert_eq!(plain.thumb, 55);

        // A window at the owner's top left corner, touching its right and
        // bottom: bit 2 is never set by the refresh.
        let tight = refresh_line_scroll(
            LineScrollPlace {
                x: 2,
                y: 2,
                owner_right: 100,
                owner_bottom: 80,
                ..scroll_place()
            },
            5,
        );
        assert_eq!(tight.mask, 0xd);
        assert_eq!((tight.start, tight.end), (55, 65));

        // A NaN fraction reads as a zero slide.
        let lost = refresh_line_scroll(
            LineScrollPlace {
                fraction: f32::NAN,
                ..scroll_place()
            },
            5,
        );
        assert_eq!(lost.thumb, 50);
        // A slide rounds half to even: 10 * 0.25 is 2.5, which rounds to 2.
        let quarter = refresh_line_scroll(
            LineScrollPlace {
                fraction: 0.25,
                ..scroll_place()
            },
            5,
        );
        assert_eq!(quarter.thumb, 52);
    }

    #[test]
    fn a_progress_strip_draws_two_caps_a_track_and_the_thumb_caps() {
        let plain = draw_line_progress(5, 10, 20, 100, 0);
        assert_eq!(plain.len(), 3);
        assert_eq!(plain[0].sample, LINE_STRIP_CAP);
        assert_eq!(plain[0].corners, [(20, 10), (20, 20), (40, 20), (40, 10)]);
        assert_eq!(plain[1].corners, [(80, 10), (80, 20), (60, 20), (60, 10)]);
        assert_eq!(plain[2].sample, WIDE_BAR);
        assert_eq!(plain[2].corners, [(40, 10), (40, 20), (60, 20), (60, 10)]);

        let split = draw_line_progress(5, 10, 20, 100, 30);
        assert_eq!(split.len(), 6);
        assert_eq!(split[2].corners, [(40, 10), (40, 20), (30, 20), (30, 10)]);
        assert_eq!(split[3].corners, [(60, 10), (60, 20), (70, 20), (70, 10)]);
        assert_eq!(split[4].sample, WIDE_CAP);
        assert_eq!(split[4].corners, [(30, 10), (30, 20), (40, 20), (40, 10)]);
        assert_eq!(split[5].corners, [(70, 10), (70, 20), (60, 20), (60, 10)]);
    }

    #[test]
    fn the_open_span_draws_caps_and_a_middle_from_its_top() {
        let span = draw_open_span(5, 10, 20, 100, SPAN_VIEW_UNSET);
        assert_eq!(span.ink_binds, 0);
        assert_eq!(span.binds, [SPAN_END, SPAN_END, SPAN_BAR]);
        assert_eq!(
            span.quads[0],
            [(0.0, 20.0), (0.0, 40.0), (30.0, 40.0), (30.0, 20.0)]
        );
        assert_eq!(
            span.quads[1],
            [(120.0, 20.0), (120.0, 40.0), (90.0, 40.0), (90.0, 20.0)]
        );
        assert_eq!(
            span.quads[2],
            [(30.0, 30.0), (30.0, 40.0), (90.0, 40.0), (90.0, 30.0)]
        );
    }

    #[test]
    fn a_maximised_window_frames_its_edges() {
        let place = WindowFramePlace {
            style: 0,
            y: 0,
            owner_state: 0,
            width: 100,
            height: 80,
        };
        let plain = draw_window_frame(5, place);
        assert_eq!(plain.ink, None);
        assert_eq!(plain.quads.len(), 8);
        assert_eq!(
            plain.quads[0].corners,
            [(30, 0), (30, 10), (70, 10), (70, 0)]
        );
        // Without bit 4 the third side is four units.
        assert_eq!(
            plain.quads[1].corners,
            [(0, 60), (10, 60), (10, 20), (0, 20)]
        );
        assert_eq!(plain.quads[2].sample, WINDOW_CORNER);
        assert_eq!(
            plain.quads[2].corners,
            [(-10, -10), (-10, 20), (30, 20), (30, -10)]
        );
        assert_eq!(
            plain.quads[7].corners,
            [(110, 90), (110, 60), (70, 60), (70, 90)]
        );

        // With bit 4 the outer top bar is drawn too, unless the window
        // sits at its focused owner's top.
        let held = draw_window_frame(
            5,
            WindowFramePlace {
                style: 0x44,
                ..place
            },
        );
        assert_eq!(held.ink, Some(WindowFrameInk::Hot));
        assert_eq!(held.quads.len(), 9);
        assert_eq!(
            held.quads[2].corners,
            [(0, 70), (10, 70), (10, 10), (0, 10)]
        );
        let tucked = draw_window_frame(
            5,
            WindowFramePlace {
                style: 4,
                owner_state: 1,
                ..place
            },
        );
        assert_eq!(tucked.ink, Some(WindowFrameInk::Normal));
        assert_eq!(tucked.quads.len(), 8);
        let lower = draw_window_frame(
            5,
            WindowFramePlace {
                style: 4,
                owner_state: 0x80_0001,
                y: 5,
                ..place
            },
        );
        assert_eq!(lower.quads.len(), 9);
    }

    #[test]
    fn the_line_scheme_expands_to_the_shipped_inks() {
        let palette = line_palette();
        // The third group is the label group: its base is the grey text.
        assert_eq!(palette.text[4], [170, 165, 160, 255]);
        // The form group's base is the black shell at the first ramp's
        // alpha byte.
        assert_eq!(palette.form[4], [0, 0, 0, 224]);
        // The fourth group (highlight) keeps its own alpha for the accent.
        assert_eq!(palette.highlight[4], [191, 92, 20, 196]);
        // The accent's scale is zero, so both accent ends carry alpha 0.
        assert_eq!(palette.accent_dim[3], 0);
        assert_eq!(palette.accent_bright[3], 0);
        assert_eq!(palette.accent_dim[0], 8);
        let (shade, label, slot) = line_inks();
        assert_eq!(shade, palette.form[1]);
        assert_eq!(label, palette.text[6]);
        assert_ne!(slot, label);
    }
}

#[cfg(test)]
mod shown_tests {
    use super::*;

    #[test]
    fn a_shown_form_takes_the_alpha_ramps_and_the_control_inks() {
        let palette = shown_palette(1.0);
        assert_eq!(palette.form[3][3], 170);
        assert_eq!(palette.control[2][3], 125);
        assert_eq!(palette.highlight[7][3], 125);
        assert_eq!(
            line_control_inks(1.0),
            [
                [255, 250, 230, 80],
                [255, 250, 230, 104],
                [255, 250, 230, 128]
            ]
        );
        assert_eq!(line_control_inks(0.5)[1][3], 52);
        assert_eq!(hover_alpha(1.0), 255);
        assert_eq!(hover_alpha(0.5), 128);
    }

    fn place(width: i32, height: i32) -> LineFormPlace {
        LineFormPlace {
            width,
            height,
            x: 0,
            y: 0,
            attributes: 0x3_0022,
            style: 0,
            state: 0x1000,
            owner_attributes: 0,
            owner_client: (0, 0),
            title_width: 0,
            hover_mark: 0,
            scroll: None,
        }
    }

    #[test]
    fn a_held_scrolling_panel_draws_its_arrows_thumb_and_rail() {
        use crate::skin::{HELD_END, HELD_OVER, HELD_POST, HELD_RIGHT, HELD_TAB, WIDE_CAP};
        // The Settings form's Controls page: held at the owner's client's
        // top-left, its right edge clear of the owner's client.
        let page = LineFormPlace {
            attributes: 0x3_02a2,
            style: 0x88,
            state: 0x1004,
            owner_attributes: 0x3_1063,
            owner_client: (420, 520),
            scroll: Some(LineFormScroll {
                client_top: 2,
                client_right: 380,
                client_bottom: 414,
                thumb: 40,
                options: 0,
            }),
            ..place(400, 416)
        };
        let form = draw_line_form(4, page);
        assert!(form.complete);
        assert_eq!(form.edges, 3);
        let tabs: Vec<_> = form.frame.iter().filter(|q| q.sample == HELD_TAB).collect();
        assert_eq!(tabs.len(), 2);
        // x = 380 + round(0.6) = 381; up arrow 2 units under the top (the
        // top-right corner is not flush), down arrow 2 units over the bottom.
        assert_eq!(
            tabs[0].corners,
            [(385, 18), (385, 10), (393, 10), (393, 18)]
        );
        assert_eq!(
            tabs[1].corners,
            [(385, 398), (385, 406), (393, 406), (393, 398)]
        );
        let thumb = form.frame.iter().find(|q| q.sample == HELD_POST).unwrap();
        assert_eq!(thumb.corners, [(381, 52), (381, 36), (397, 36), (397, 52)]);
        // Held: near 0, far 2 units; the right bar sits 2 to 4 units in.
        let right = form.frame.iter().find(|q| q.sample == HELD_RIGHT).unwrap();
        assert_eq!(
            right.corners,
            [(392, 24), (392, 392), (384, 392), (384, 24)]
        );
        // The rail outside the free right edge: x = 400 + round(0.8) = 401.
        let caps = form.frame.iter().filter(|q| q.sample == WIDE_CAP).count();
        assert_eq!(caps, 2);
        // Neither right corner is flush: the scroll bar's corner pieces.
        assert_eq!(
            form.frame.iter().filter(|q| q.sample == HELD_OVER).count(),
            2
        );
        assert!(!form.frame.iter().any(|q| q.sample == HELD_END));
        // Without its place, the scroll branch is not drawn.
        let unplaced = draw_line_form(
            4,
            LineFormPlace {
                scroll: None,
                ..page
            },
        );
        assert!(!unplaced.complete);
    }

    #[test]
    fn a_free_panel_draws_its_backdrop_and_a_two_unit_frame() {
        use form_bits::*;
        let free = LineFormPlace {
            attributes: 0x13_0032,
            style: 0x80,
            owner_attributes: 0x3_0022,
            ..place(228, 180)
        };
        let form = draw_line_form(4, free);
        assert!(form.complete);
        assert!(!form.shadow);
        assert_eq!(form.edges, 0xf);
        assert_eq!(
            form.backdrop,
            Some([(0, 0), (0, 180), (228, 180), (228, 0)])
        );
        assert_eq!(form.ink, LineFormInk::Idle);
        // The strip's three quads, then eleven: two side bars, a bottom
        // bar and eight joint pieces.
        assert_eq!(form.frame.len(), 14);
        assert_eq!(form.frame[2].sample, WIDE_BAR);
        assert_eq!(
            form.frame[2].corners,
            [(32, 8), (32, 16), (196, 16), (196, 8)]
        );
        assert_eq!(
            form.frame[3].corners,
            [(8, 148), (16, 148), (16, 32), (8, 32)]
        );
        assert_eq!(form.frame[11].sample, WIDE_BAR);
        assert_eq!(
            form.frame[11].corners,
            [(32, 172), (32, 164), (196, 164), (196, 172)]
        );
        assert_eq!(
            form.frame[13].corners,
            [(212, 172), (212, 164), (196, 164), (196, 172)]
        );
        // A titled owner drops the top pair of joints; its open-top bit
        // keeps them.
        let titled = draw_line_form(
            4,
            LineFormPlace {
                owner_attributes: 0x3_0022 | TITLE,
                ..free
            },
        );
        assert_eq!(titled.frame.len(), 12);
        let open = draw_line_form(
            4,
            LineFormPlace {
                owner_attributes: 0x3_0022 | TITLE | OPEN_TOP,
                ..free
            },
        );
        assert_eq!(open.frame.len(), 14);
    }

    #[test]
    fn a_held_panel_lies_on_its_edges_and_curls_its_open_corners() {
        use crate::skin::HELD_FLIP;
        // The inventory's pack at unit 4: held, flush with the client's
        // left, right and bottom, under a titled owner.
        let pack = LineFormPlace {
            x: 0,
            y: 448,
            attributes: 0x11_0032,
            style: 0x80,
            state: 0x1004,
            owner_attributes: 0x3_0063,
            owner_client: (228, 628),
            ..place(228, 180)
        };
        let form = draw_line_form(4, pack);
        assert!(form.complete);
        assert_eq!(form.backdrop, None);
        assert_eq!(form.edges, 0xe);
        // The top bar (not flush with the top), the left bar, the curled
        // top corners, the right bar, the bottom joints and bar.
        assert_eq!(form.frame[0].sample, WIDE_BAR);
        assert_eq!(
            form.frame[0].corners,
            [(24, 0), (24, 8), (204, 8), (204, 0)]
        );
        assert_eq!(
            form.frame[1].corners,
            [(0, 156), (8, 156), (8, 24), (0, 24)]
        );
        assert_eq!(form.frame[2].sample, HELD_FLIP);
        assert_eq!(
            form.frame[2].corners,
            [(-8, -8), (-8, 24), (24, 24), (24, -8)]
        );
        assert_eq!(form.frame[3].sample, HELD_FLIP);
        assert_eq!(
            form.frame[3].corners,
            [(236, -8), (236, 24), (204, 24), (204, -8)]
        );
        assert_eq!(form.frame.len(), 10);
        assert!(form.frame[5..].iter().all(|quad| quad.sample != HELD_FLIP));
        // The character view: flush with the top, left and right under the
        // titled owner, so no top bar and no top joint pairs; its bottom
        // corners curl.
        let view = LineFormPlace {
            y: 0,
            attributes: 0x5_0092,
            ..pack
        };
        let view = draw_line_form(
            4,
            LineFormPlace {
                height: 384,
                ..view
            },
        );
        assert_eq!(view.edges, 7);
        assert_eq!(view.frame.len(), 7);
        assert_eq!(
            view.frame[0].corners,
            [(0, 360), (8, 360), (8, 24), (0, 24)]
        );
        assert_eq!(view.frame[4].sample, HELD_FLIP);
        assert_eq!(view.frame[6].sample, HELD_FLIP);
    }

    #[test]
    fn a_titled_top_level_form_draws_its_corner_frame_and_title_strip() {
        use crate::skin::{FORM_ALT, FORM_CAP, FORM_CLOSE_INK, FORM_CORNER, FORM_EDGE};
        // The inventory after its words are committed under the task.
        let inventory = LineFormPlace {
            attributes: 0x3_0063,
            style: 0x3010,
            title_width: 60,
            ..place(244, 644)
        };
        let form = draw_line_form(4, inventory);
        assert!(form.complete);
        assert!(form.shadow);
        assert!(form.backdrop.is_some());
        assert_eq!(form.title_x, Some(92));
        assert_eq!(form.frame[0].sample, FORM_CORNER);
        assert_eq!(form.frame[0].corners, [(0, 0), (0, 24), (24, 24), (24, 0)]);
        assert_eq!(form.frame[1].sample, FORM_CLOSE_INK);
        assert_eq!(
            form.frame[1].corners,
            [(244, 0), (228, 0), (228, 16), (244, 16)]
        );
        assert_eq!(form.frame[2].sample, FORM_ALT);
        // round(4 × 6.6) = 26; the gap is (244 − 60 − 20) / 2 = 82.
        assert_eq!(form.frame[5].sample, FORM_EDGE);
        assert_eq!(form.frame[5].corners, [(26, 0), (26, 8), (82, 8), (82, 0)]);
        assert_eq!(form.frame[7].sample, FORM_CAP);
        assert_eq!(form.frame[7].corners, [(82, 0), (82, 8), (90, 8), (90, 0)]);
        // Three side bars, then the strip split at the gap: two caps, two
        // tracks and the two inner caps.
        assert_eq!(form.frame.len(), 12 + 6);
        assert_eq!(form.frame[12].sample, LINE_STRIP_CAP);
        assert_eq!(
            form.frame[14].corners,
            [(32, 8), (32, 16), (82, 16), (82, 8)]
        );
        // A hovered close mark is left to the late draw; no title bit, no
        // title, no gap and no strip.
        let hovered = draw_line_form(
            4,
            LineFormPlace {
                hover_mark: -4,
                ..inventory
            },
        );
        assert_eq!(hovered.frame[1].sample, FORM_ALT);
        let plain = draw_line_form(
            4,
            LineFormPlace {
                attributes: 0x3_0062,
                ..inventory
            },
        );
        assert_eq!(plain.title_x, None);
        assert_eq!(plain.frame.len(), 9);
    }

    #[test]
    fn a_plain_form_draws_only_its_backdrop_and_scrolling_panels_are_flagged() {
        let outer = draw_line_form(
            4,
            LineFormPlace {
                style: 0x3000,
                ..place(244, 644)
            },
        );
        assert!(outer.complete);
        assert!(outer.frame.is_empty());
        assert!(outer.backdrop.is_some());
        let scrolling = draw_line_form(
            4,
            LineFormPlace {
                attributes: 0x3_0222,
                style: 0x80,
                ..place(100, 100)
            },
        );
        assert!(!scrolling.complete);
        assert!(scrolling.frame.is_empty());
    }
}
