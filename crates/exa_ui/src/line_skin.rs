//! The line skin: the game's flat GUI skin.
//!
//! The original draws every widget of its interface with this skin: frames and
//! window chrome, buttons, sliders and scroll bars, progress bars, dividers,
//! the centred icon of a control and its caption. It is a subclass of the
//! plain draw skin, so it reuses that skin's span, slider, frame and label
//! helpers and overrides the shapes it draws differently: thin drawn lines
//! instead of filled bevels.
//!
//! The skin's draw call is a batch of textured quads. Each quad names one of
//! the skin's eight-float samples (four texture coordinates, in order) and
//! carries its own colour. [`LineDraw`] gathers that batch, the mode and ink
//! changes around it, the clip region and the children the call walks. The
//! per-widget helpers write into one batch so a frame does not allocate a
//! vector per widget.

use crate::skin::{LINE_RAIL, SPAN_BAR, SPAN_CAP, SPAN_END, SkinInk};

/// The skin's alpha byte for an opaque force, 255.
pub const LINE_OPAQUE: u8 = 0xff;

/// The orange the original hard-codes for an active close edge and for the
/// late close square.
pub const LINE_CLOSE_INK: SkinInk = [0xff, 0xcc, 0x99, 0xff];

/// A sample used by the thin window edges: 0, 0, 0, 0.3 and 0.15, 0.3, 0.15, 0.
pub const LINE_EDGE: [u32; 8] = [
    0,
    0,
    0,
    0x3e99_999a,
    0x3e19_999a,
    0x3e99_999a,
    0x3e19_999a,
    0,
];

/// The alternate thin edge: 0, 0.4, 0, 0.7 and 0.15, 0.7, 0.15, 0.4.
pub const LINE_EDGE_ALT: [u32; 8] = [
    0,
    0x3ecc_cccd,
    0,
    0x3f33_3333,
    0x3e19_999a,
    0x3f33_3333,
    0x3e19_999a,
    0x3ecc_cccd,
];

/// The close square's face: 0, 0.8, 0, 1 and 0.1, 1, 0.1, 0.8.
pub const LINE_CLOSE: [u32; 8] = [
    0,
    0x3f4c_cccd,
    0,
    0x3f80_0000,
    0x3dcc_cccd,
    0x3f80_0000,
    0x3dcc_cccd,
    0x3f4c_cccd,
];

/// A title cap: 0.7, 0, 0.7, 0.1 and 0.75, 0.1, 0.75, 0.
pub const LINE_CAP: [u32; 8] = [
    0x3f33_3333,
    0,
    0x3f33_3333,
    0x3dcc_cccd,
    0x3f40_0000,
    0x3dcc_cccd,
    0x3f40_0000,
    0,
];

/// A track bar: 0.3, 0, 0.3, 0.1 and 0.5, 0.1, 0.5, 0.
pub const LINE_TRACK: [u32; 8] = [
    0x3e99_999a,
    0,
    0x3e99_999a,
    0x3dcc_cccd,
    0x3f00_0000,
    0x3dcc_cccd,
    0x3f00_0000,
    0,
];

/// The window rail: 0.2, 0.1, 0.2, 0.2 and 0.3, 0.2, 0.3, 0.1.
pub const LINE_BAR: [u32; 8] = [
    0x3e4c_cccd,
    0x3dcc_cccd,
    0x3e4c_cccd,
    0x3e4c_cccd,
    0x3e99_999a,
    0x3e4c_cccd,
    0x3e99_999a,
    0x3dcc_cccd,
];

/// The frame edge: 0.15, 0.2, 0.15, 0.6 and 0.35, 0.6, 0.35, 0.2.
pub const LINE_FRAME: [u32; 8] = [
    0x3e19_999a,
    0x3e4c_cccd,
    0x3e19_999a,
    0x3f19_999a,
    0x3eb3_3333,
    0x3f19_999a,
    0x3eb3_3333,
    0x3e4c_cccd,
];

/// The alternate frame edge: 0.6, 0.2, 0.6, 0.6 and 0.8, 0.6, 0.8, 0.2.
pub const LINE_FRAME_ALT: [u32; 8] = [
    0x3f19_999a,
    0x3e4c_cccd,
    0x3f19_999a,
    0x3f19_999a,
    0x3f4c_cccd,
    0x3f19_999a,
    0x3f4c_cccd,
    0x3e4c_cccd,
];

/// The end of a frame arm: 0.65, 0.7, 0.65, 0.9 and 0.75, 0.9, 0.75, 0.7.
pub const LINE_END: [u32; 8] = [
    0x3f26_6666,
    0x3f33_3333,
    0x3f26_6666,
    0x3f66_6666,
    0x3f40_0000,
    0x3f66_6666,
    0x3f40_0000,
    0x3f33_3333,
];

/// The right arm of a frame: 0.7, 0.5, 0.7, 0.7 and 0.75, 0.7, 0.75, 0.5.
pub const LINE_RIGHT: [u32; 8] = [
    0x3f33_3333,
    0x3f00_0000,
    0x3f33_3333,
    0x3f33_3333,
    0x3f40_0000,
    0x3f33_3333,
    0x3f40_0000,
    0x3f00_0000,
];

/// A frame corner: 0.15, 0.2, 0.15, 0.5 and 0.35, 0.5, 0.35, 0.2.
pub const LINE_CORNER: [u32; 8] = [
    0x3e19_999a,
    0x3e4c_cccd,
    0x3e19_999a,
    0x3f00_0000,
    0x3eb3_3333,
    0x3f00_0000,
    0x3eb3_3333,
    0x3e4c_cccd,
];

/// The open frame's face: 0.6, 0.2, 0.6, 0.6 and 0.8, 0.6, 0.8, 0.2.
pub const LINE_OPEN: [u32; 8] = LINE_FRAME_ALT;

/// A raised face: 0.75, 0.8, 0.75, 1 and 0.85, 1, 0.85, 0.8.
pub const LINE_FACE: [u32; 8] = [
    0x3f40_0000,
    0x3f4c_cccd,
    0x3f40_0000,
    0x3f80_0000,
    0x3f59_999a,
    0x3f80_0000,
    0x3f59_999a,
    0x3f4c_cccd,
];

/// A hot face: 0.85, 0.8, 0.85, 1 and 0.95, 1, 0.95, 0.8.
pub const LINE_FACE_HOT: [u32; 8] = [
    0x3f59_999a,
    0x3f4c_cccd,
    0x3f59_999a,
    0x3f80_0000,
    0x3f73_3333,
    0x3f80_0000,
    0x3f73_3333,
    0x3f4c_cccd,
];

/// A cap of the slider: 0.2, 0.8, 0.2, 1 and 0.25, 1, 0.25, 0.8.
pub const LINE_SLIDER_CAP: [u32; 8] = [
    0x3e4c_cccd,
    0x3f4c_cccd,
    0x3e4c_cccd,
    0x3f80_0000,
    0x3e80_0000,
    0x3f80_0000,
    0x3e80_0000,
    0x3f4c_cccd,
];

/// The middle of a slider: 0.25, 0.8, 0.25, 1 and 0.35, 1, 0.35, 0.8.
pub const LINE_SLIDER_MIDDLE: [u32; 8] = [
    0x3e80_0000,
    0x3f4c_cccd,
    0x3e80_0000,
    0x3f80_0000,
    0x3eb3_3333,
    0x3f80_0000,
    0x3eb3_3333,
    0x3f4c_cccd,
];

/// A slider thumb: 0.4, 0.8, 0.4, 1 and 0.5, 1, 0.5, 0.8.
pub const LINE_SLIDER_THUMB: [u32; 8] = [
    0x3ecc_cccd,
    0x3f4c_cccd,
    0x3ecc_cccd,
    0x3f80_0000,
    0x3f00_0000,
    0x3f80_0000,
    0x3f00_0000,
    0x3f4c_cccd,
];

/// The scroll bar's end: 0.7, 0.1, 0.7, 0.2 and 0.75, 0.2, 0.75, 0.1.
pub const LINE_SCROLL_EDGE: [u32; 8] = SPAN_CAP;

/// The scroll bar's rail: 0.3, 0.1, 0.3, 0.2 and 0.5, 0.2, 0.5, 0.1.
pub const LINE_SCROLL_BAR: [u32; 8] = SPAN_BAR;

/// The scroll bar's thumb: 0.5, 0.8, 0.5, 0.9 and 0.55, 0.9, 0.55, 0.8.
pub const LINE_SCROLL_THUMB: [u32; 8] = LINE_RAIL;

/// The five lengths a line skin derives from the GUI unit: 2, 1, 2, 2 and
/// 4 units.
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
    if !*registered {
        *registered = true;
    }
    *registered
}

/// The three side lengths the line skin keeps while it draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineSpans {
    pub first: i32,
    pub second: i32,
    pub third: i32,
}

/// A widget's owner: the rectangle the widget is laid out in and the owner's
/// state word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineOwner {
    pub left: i32,
    pub top: i32,
    pub bottom: i32,
    pub right: i32,
    pub state: u32,
}

/// The record a line widget draws from. The rectangle is the widget's own,
/// the client is its scrolled content, and the owner is the form it sits in.
#[derive(Debug, Clone, Copy)]
pub struct LineWidget<'a> {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// The two style words.
    pub style: u32,
    pub style_b: u32,
    /// The state word: focus, close, scroll and the fixed-size flags.
    pub state: u32,
    /// The extra flags: the clip word and the open scroll rail.
    pub flags: u32,
    /// The close mark, -4 for the square in the top right corner.
    pub close_mark: i32,
    /// The face colour of a rectangle slot.
    pub face: SkinInk,
    pub client_left: i32,
    pub client_top: i32,
    pub client_bottom: i32,
    pub client_right: i32,
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub scroll_scale_x: f32,
    pub scroll_scale_y: f32,
    /// The measured width and ascent of the caption, and its bytes.
    pub title: &'a [u8],
    pub title_width: i32,
    pub title_ascent: i32,
    pub title_tint: Option<SkinInk>,
    pub owner: Option<LineOwner>,
    /// The first byte's slide, as the scroll refresh computed it.
    pub thumb: i32,
    /// One byte per child; a zero byte skips that child.
    pub visible_children: &'a [u8],
}

impl LineWidget<'_> {
    fn owner(&self) -> LineOwner {
        self.owner.unwrap_or(LineOwner {
            left: 0,
            top: 0,
            bottom: 0,
            right: 0,
            state: 0,
        })
    }
}

/// The colours the line skin binds. The fields are named after what the
/// original paints with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineInks {
    /// The pressed state's ink.
    pub pressed: SkinInk,
    /// The normal ink.
    pub normal: SkinInk,
    /// The alternate ink of an unfocused line.
    pub alternate: SkinInk,
    /// The hot, focused ink of a window.
    pub hot: SkinInk,
    /// The backdrop behind a floating form.
    pub backdrop: SkinInk,
    /// The shadow's alpha.
    pub shadow_alpha: u8,
    /// A caption's ink.
    pub caption: SkinInk,
    /// A label's dim ink.
    pub label: SkinInk,
    /// A label's plain ink.
    pub label_plain: SkinInk,
    /// A label's hot ink.
    pub label_hot: SkinInk,
    /// A plain centred icon.
    pub icon: SkinInk,
    /// A hot centred icon.
    pub icon_hot: SkinInk,
    /// A focused centred icon.
    pub icon_focus: SkinInk,
    /// A raised centred icon, whose alpha is lifted when it is drawn.
    pub icon_raised: SkinInk,
}

impl LineInks {
    /// One ink for every field.
    pub fn uniform(ink: SkinInk) -> Self {
        LineInks {
            pressed: ink,
            normal: ink,
            alternate: ink,
            hot: ink,
            backdrop: ink,
            shadow_alpha: ink[3],
            caption: ink,
            label: ink,
            label_plain: ink,
            label_hot: ink,
            icon: ink,
            icon_hot: ink,
            icon_focus: ink,
            icon_raised: ink,
        }
    }
}

/// One quad of the batch, with the sample its four corners read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineQuad {
    pub sample: [u32; 8],
    pub corners: [(f32, f32); 4],
    pub ink: SkinInk,
}

/// One solid rectangle, drawn through the solid program rather than the
/// texture samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSolid {
    pub corners: [(f32, f32); 4],
    pub ink: SkinInk,
}

/// A caption the draw passes to the font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineTitle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub ink: SkinInk,
}

/// The soft plate under a floating form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineShadow {
    pub width: i32,
    pub height: i32,
    pub corner: i32,
    pub alpha: u8,
}

/// The batch one line widget's draw gathers.
#[derive(Debug, Clone, PartialEq)]
pub struct LineDraw {
    pub mode_calls: u32,
    pub mode: u32,
    pub texture_binds: u32,
    pub ink_binds: u32,
    pub begin_calls: u32,
    pub end_calls: u32,
    pub quads: Vec<LineQuad>,
    pub solids: Vec<LineSolid>,
    pub clip_calls: u32,
    pub clip_pushed: bool,
    pub clip: [f32; 4],
    pub scroll: [i32; 2],
    pub self_draw_calls: u32,
    pub children: Vec<usize>,
    pub title: Option<LineTitle>,
    pub backdrop: Option<LineQuad>,
    pub shadow: Option<LineShadow>,
    pub spans: LineSpans,
    pub release_calls: u32,
    /// The ink currently bound; the helpers that draw quads read it.
    pub ink: SkinInk,
}

impl Default for LineDraw {
    fn default() -> Self {
        Self::new()
    }
}

impl LineDraw {
    /// An empty batch with room for a window's quads.
    pub fn new() -> Self {
        LineDraw {
            mode_calls: 0,
            mode: 0,
            texture_binds: 0,
            ink_binds: 0,
            begin_calls: 0,
            end_calls: 0,
            quads: Vec::with_capacity(64),
            solids: Vec::with_capacity(4),
            clip_calls: 0,
            clip_pushed: false,
            clip: [0.0; 4],
            scroll: [0; 2],
            self_draw_calls: 0,
            children: Vec::new(),
            title: None,
            backdrop: None,
            shadow: None,
            spans: LineSpans::default(),
            release_calls: 0,
            ink: [0; 4],
        }
    }

    /// Empties the batch, keeping its buffers for the next frame.
    pub fn clear(&mut self) {
        self.mode_calls = 0;
        self.mode = 0;
        self.texture_binds = 0;
        self.ink_binds = 0;
        self.begin_calls = 0;
        self.end_calls = 0;
        self.quads.clear();
        self.solids.clear();
        self.clip_calls = 0;
        self.clip_pushed = false;
        self.clip = [0.0; 4];
        self.scroll = [0; 2];
        self.self_draw_calls = 0;
        self.children.clear();
        self.title = None;
        self.backdrop = None;
        self.shadow = None;
        self.spans = LineSpans::default();
        self.release_calls = 0;
        self.ink = [0; 4];
    }

    fn select(&mut self, mode: u32) {
        self.mode_calls += 1;
        self.mode = mode;
    }

    fn bind(&mut self, ink: SkinInk) {
        self.ink_binds += 1;
        self.ink = ink;
    }

    fn push(&mut self, sample: [u32; 8], corners: [(f32, f32); 4]) {
        self.quads.push(LineQuad {
            sample,
            corners,
            ink: self.ink,
        });
    }

    fn push_solid(&mut self, corners: [(f32, f32); 4]) {
        self.solids.push(LineSolid {
            corners,
            ink: self.ink,
        });
    }
}

/// Converts a float to an integer the way the original's `CVTSS2SI` does:
/// round to nearest even, and the indefinite value for a NaN or an
/// out-of-range number.
fn float_to_int(value: f32) -> i32 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&rounded) {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// Converts a double product the way the original's `CVTSD2SI` does.
fn double_to_int(value: f64) -> i32 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&rounded) {
        i32::MIN
    } else {
        rounded as i32
    }
}

fn f(value: i32) -> f32 {
    value as f32
}

/// The scroll mask of a line widget: bit 1 the top edge is above the unit,
/// bit 2 the left edge before it, bit 4 and bit 8 the widget reaches past
/// the owner's right or bottom edge by a unit.
fn scroll_mask(widget: &LineWidget, unit: i32) -> u32 {
    if (widget.style_b & 4) == 0 {
        return 0xf;
    }
    let owner = widget.owner();
    let mut mask = 0;
    if widget.y < unit {
        mask |= 1;
    }
    if widget.x < unit {
        mask |= 2;
    }
    let room_x = i64::from(owner.right) - i64::from(owner.left);
    if room_x < i64::from(unit) + i64::from(widget.x) + i64::from(widget.w) {
        mask |= 4;
    }
    let room_y = i64::from(owner.bottom) - i64::from(owner.top);
    if room_y < i64::from(unit) + i64::from(widget.y) + i64::from(widget.h) {
        mask |= 8;
    }
    mask
}

/// The scroll rail's start, end, side and thumb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineScroll {
    pub mask: u32,
    pub start: i32,
    pub end: i32,
    pub side: i32,
    pub thumb: i32,
}

/// Refreshes the scroll rail from the widget and its owner. The mask is the
/// same edge mask the window chrome uses; the thumb slides from the start to
/// the end less one side.
pub fn refresh_line_scroll(widget: &LineWidget, unit: i32) -> LineScroll {
    let mask = scroll_mask(widget, unit);
    let start = if (mask & 5) == 5 {
        unit.wrapping_mul(5).wrapping_add(widget.client_top)
    } else {
        unit.wrapping_shl(2).wrapping_add(widget.client_top)
    };
    let end = if (mask & 0xc) == 0xc {
        widget.client_bottom.wrapping_sub(unit.wrapping_mul(5))
    } else {
        widget.client_bottom.wrapping_sub(unit.wrapping_shl(2))
    };
    let side = unit.wrapping_shl(1);
    let span = i64::from(end) - i64::from(start) - i64::from(side);
    let thumb = start.wrapping_add(float_to_int(span as f32 * widget.scroll_scale_y));
    LineScroll {
        mask,
        start,
        end,
        side,
        thumb,
    }
}

/// The progress bar of a slider or track control: a cap at each end, the
/// track between them, and, when the thumb is past zero, a second pair of
/// track pieces with a cap on each side of the thumb.
pub fn draw_line_progress(draw: &mut LineDraw, widget: &LineWidget, unit: i32, thumb: i32) {
    let side = unit.wrapping_shl(3);
    let span = draw.spans;
    let width = f(widget.w);
    let low = f(span.first);
    let high = f(span.third);
    draw.push(
        LINE_BAR,
        [(high, low), (high, high), (f(side), high), (f(side), low)],
    );
    draw.push(
        LINE_BAR,
        [
            (width - high, low),
            (width - high, high),
            (width - f(side), high),
            (width - f(side), low),
        ],
    );
    if thumb <= 0 {
        draw.push(
            LINE_SCROLL_BAR,
            [
                (f(side), low),
                (f(side), high),
                (width - f(side), high),
                (width - f(side), low),
            ],
        );
    } else {
        let thumb_f = f(thumb);
        draw.push(
            LINE_SCROLL_BAR,
            [
                (f(side), low),
                (f(side), high),
                (thumb_f, high),
                (thumb_f, low),
            ],
        );
        draw.push(
            LINE_SCROLL_BAR,
            [
                (width - f(side), low),
                (width - f(side), high),
                (width - thumb_f, high),
                (width - thumb_f, low),
            ],
        );
        draw.push(
            LINE_SCROLL_EDGE,
            [
                (thumb_f, low),
                (thumb_f, high),
                (thumb_f + low, high),
                (thumb_f + low, low),
            ],
        );
        draw.push(
            LINE_SCROLL_EDGE,
            [
                (width - thumb_f, low),
                (width - thumb_f, high),
                (width - thumb_f - low, high),
                (width - thumb_f - low, low),
            ],
        );
    }
}

/// The six thin lines of a rule: a cap and a bar at the top, then the same
/// at the bottom, half a unit outside the widget.
pub fn draw_line_border(draw: &mut LineDraw, widget: &LineWidget, unit: i32) {
    let half = unit.wrapping_shr(1);
    let x = widget.x;
    let y = widget.y;
    let right = widget.x.wrapping_add(widget.w);
    let top = y.wrapping_sub(half);
    let bottom = y.wrapping_add(widget.h).wrapping_add(half);
    let inset = draw.spans.first;
    // The sample groups bound by the original around each line.
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (f(x + inset), f(top + unit)),
            (f(x + inset), f(top - unit)),
            (f(x), f(top - unit)),
            (f(x), f(top + unit)),
        ],
    );
    draw.push(
        LINE_SCROLL_BAR,
        [
            (f(x + inset), f(top + unit)),
            (f(x + inset), f(top - unit)),
            (f(right - inset), f(top - unit)),
            (f(right - inset), f(top + unit)),
        ],
    );
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (f(right - inset), f(top + unit)),
            (f(right - inset), f(top - unit)),
            (f(right), f(top - unit)),
            (f(right), f(top + unit)),
        ],
    );
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (f(x + inset), f(bottom - unit)),
            (f(x + inset), f(bottom + unit)),
            (f(x), f(bottom + unit)),
            (f(x), f(bottom - unit)),
        ],
    );
    draw.push(
        LINE_SCROLL_BAR,
        [
            (f(x + inset), f(bottom - unit)),
            (f(x + inset), f(bottom + unit)),
            (f(right - inset), f(bottom + unit)),
            (f(right - inset), f(bottom - unit)),
        ],
    );
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (f(right - inset), f(bottom - unit)),
            (f(right - inset), f(bottom + unit)),
            (f(right), f(bottom + unit)),
            (f(right), f(bottom - unit)),
        ],
    );
}

/// The four-unit marker square of a line control, centred on a point. It is
/// the line skin's replacement for the parent skin's cross.
pub fn draw_line_marker(draw: &mut LineDraw, unit: i32, x: i32, y: i32) {
    let side = unit.wrapping_shl(2);
    let back = ((i64::from(side) - 2) >> 1) as i32;
    let left = f(x.wrapping_sub(back));
    let top = f(y.wrapping_sub(back));
    let right = left + f(side);
    let bottom = top + f(side);
    draw.push(
        SPAN_END,
        [(left, bottom), (left, top), (right, top), (right, bottom)],
    );
}

/// The line skin's detail strip: a bar between two caps, two units tall.
pub fn draw_line_detail(draw: &mut LineDraw, x: i32, y: i32, width: i32, unit: i32) {
    let side = draw.spans.first;
    let low = f(y);
    let high = f(y.wrapping_add(unit.wrapping_shl(1)));
    let left = f(x);
    let right = f(x.wrapping_add(width));
    draw.push(
        LINE_SCROLL_BAR,
        [
            (left + f(side), low),
            (left + f(side), high),
            (right - f(side), high),
            (right - f(side), low),
        ],
    );
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (left + f(side), low),
            (left + f(side), high),
            (left, high),
            (left, low),
        ],
    );
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (right - f(side), low),
            (right - f(side), high),
            (right, high),
            (right, low),
        ],
    );
}

/// A bar from `x` to `x + width` at `y`: a cap, the middle and a cap. This is
/// the line skin's open progress track.
pub fn draw_line_bar(draw: &mut LineDraw, x: i32, y: i32, width: i32, unit: i32) {
    let left = f(x);
    let top = f(y);
    let right = f(x.wrapping_add(width));
    let bottom = f(y.wrapping_add(unit.wrapping_shl(2)));
    let side = unit.wrapping_shl(1);
    let inner_left = left + f(unit.wrapping_shl(2));
    let inner_right = right - f(unit.wrapping_shl(2));
    draw.push(
        SPAN_END,
        [
            (left - f(side), top),
            (left - f(side), bottom),
            (inner_left, bottom),
            (inner_left, top),
        ],
    );
    draw.push(
        SPAN_END,
        [
            (right + f(side), top),
            (right + f(side), bottom),
            (inner_right, bottom),
            (inner_right, top),
        ],
    );
    draw.push(
        SPAN_BAR,
        [
            (inner_left, top + f(side)),
            (inner_left, bottom),
            (inner_right, bottom),
            (inner_right, top + f(side)),
        ],
    );
}

/// A thin rule across `x..right` at `bottom`: two caps and a middle bar.
pub fn draw_line_rule(draw: &mut LineDraw, x: f32, right: f32, bottom: f32, unit: i32) {
    let side = f(unit.wrapping_shl(2));
    let edge = f(unit.wrapping_shl(1));
    let low = bottom - edge;
    let high = bottom + edge;
    let left_in = x + side;
    let right_in = right - side;
    draw.push(
        SPAN_END,
        [
            (x - edge, low),
            (x - edge, high),
            (left_in, high),
            (left_in, low),
        ],
    );
    draw.push(
        SPAN_END,
        [
            (right + edge, low),
            (right + edge, high),
            (right_in, high),
            (right_in, low),
        ],
    );
    draw.push(
        SPAN_BAR,
        [
            (left_in, bottom),
            (left_in, high),
            (right_in, high),
            (right_in, bottom),
        ],
    );
}

/// The vertical rail of a scrollable line window. The rail is a frame of
/// eight thin bars around the widget's edge plus the top bar, which the
/// original draws twice when the owner is focused and the top is in view.
pub fn draw_line_scroll(draw: &mut LineDraw, widget: &LineWidget, inks: &LineInks, unit: i32) {
    if (widget.style_b & 4) != 0 {
        draw.bind(if (widget.style_b & 0x40) == 0x40 {
            inks.hot
        } else {
            inks.normal
        });
    }
    let mask = scroll_mask(widget, unit);
    let side = unit.wrapping_shl(1);
    let six = unit.wrapping_mul(6);
    draw.spans.first = 0;
    draw.spans.third = if (widget.style_b & 4) != 0 {
        unit.wrapping_shl(1)
    } else {
        unit.wrapping_shl(2)
    };
    let width = widget.w;
    let height = widget.h;
    let first = f(six);
    if (mask & 1) == 0 || (widget.owner().state & 1) == 0 || (widget.owner().state & 0x80_0000) != 0
    {
        draw.push(
            LINE_SCROLL_BAR,
            [
                (first, f(draw.spans.first)),
                (first, f(side)),
                (f(width.wrapping_sub(six)), f(side)),
                (f(width.wrapping_sub(six)), f(draw.spans.first)),
            ],
        );
    }
    draw.push(
        LINE_SCROLL_BAR,
        [
            (first, f(draw.spans.first)),
            (first, f(side)),
            (f(width.wrapping_sub(six)), f(side)),
            (f(width.wrapping_sub(six)), f(draw.spans.first)),
        ],
    );
    draw.push(
        LINE_SCROLL_BAR,
        [
            (0.0, f(height.wrapping_sub(draw.spans.third))),
            (f(side), f(height.wrapping_sub(draw.spans.third))),
            (f(side), f(draw.spans.third)),
            (0.0, f(draw.spans.third)),
        ],
    );
    draw.push(
        LINE_CORNER,
        [
            (-f(side), -f(side)),
            (-f(side), f(draw.spans.third)),
            (first, f(draw.spans.third)),
            (first, -f(side)),
        ],
    );
    draw.push(
        LINE_CORNER,
        [
            (f(width) + f(side), -f(side)),
            (f(width) + f(side), f(draw.spans.third)),
            (f(width.wrapping_sub(six)), f(draw.spans.third)),
            (f(width.wrapping_sub(six)), -f(side)),
        ],
    );
    draw.push(
        LINE_SCROLL_BAR,
        [
            (
                f(width.wrapping_sub(draw.spans.first)),
                f(height.wrapping_sub(draw.spans.third)),
            ),
            (
                f(width.wrapping_sub(side)),
                f(height.wrapping_sub(draw.spans.third)),
            ),
            (f(width.wrapping_sub(side)), f(draw.spans.third)),
            (f(width), f(draw.spans.third)),
        ],
    );
    draw.push(
        LINE_CORNER,
        [
            (-f(side), f(height) + f(side)),
            (-f(side), f(height.wrapping_sub(draw.spans.third))),
            (first, f(height.wrapping_sub(draw.spans.third))),
            (first, f(height) + f(side)),
        ],
    );
    draw.push(
        LINE_SCROLL_BAR,
        [
            (first, f(height)),
            (first, f(height.wrapping_sub(side))),
            (f(width.wrapping_sub(six)), f(height.wrapping_sub(side))),
            (f(width.wrapping_sub(six)), f(height)),
        ],
    );
    draw.push(
        LINE_CORNER,
        [
            (f(width) + f(side), f(height) + f(side)),
            (f(width) + f(side), f(height.wrapping_sub(draw.spans.third))),
            (
                f(width.wrapping_sub(six)),
                f(height.wrapping_sub(draw.spans.third)),
            ),
            (f(width.wrapping_sub(six)), f(height) + f(side)),
        ],
    );
    draw.spans.first = unit.wrapping_shl(1);
    draw.spans.third = unit.wrapping_shl(2);
}

/// A line label's ink and place. The parent's label draw makes the call when
/// the state asks for it; this records which path the original takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineLabelDraw {
    pub ink: SkinInk,
    pub binds_ink: bool,
    pub x: i32,
    pub y: i32,
    pub draw_calls: u32,
}

/// Picks a line label's ink and place. A focused label binds the pressed
/// ink; the owner-drawn path passes its caption to the parent skin, which
/// binds its own ink; a plain state uses the plain ink, and a hot state the
/// hot ink one pixel lower.
pub fn draw_line_label(widget: &LineWidget, inks: &LineInks, x: i32, y: i32) -> LineLabelDraw {
    if (widget.state & 0x10) == 0x10 {
        return LineLabelDraw {
            ink: inks.label,
            binds_ink: true,
            x,
            y,
            draw_calls: 1,
        };
    }
    if (widget.state & 0xf_0000) == 0x1_0000 {
        return LineLabelDraw {
            ink: [0; 4],
            binds_ink: false,
            x,
            y,
            draw_calls: 1,
        };
    }
    if (widget.state & 0x60) == 0 {
        LineLabelDraw {
            ink: inks.label_plain,
            binds_ink: true,
            x,
            y,
            draw_calls: 1,
        }
    } else {
        LineLabelDraw {
            ink: inks.label_hot,
            binds_ink: true,
            x,
            y: y.wrapping_add(1),
            draw_calls: 1,
        }
    }
}

/// The rectangle slot of a line control: a thin double rule around the
/// widget, then a solid face inset by the unit.
pub fn draw_line_rect(draw: &mut LineDraw, widget: &LineWidget, inks: &LineInks, unit: i32) {
    if (widget.state & 0xf_0000) == 0x1_0000 {
        let mut ink = inks.hot;
        ink[3] = LINE_OPAQUE;
        draw.bind(ink);
    } else {
        draw.bind(inks.alternate);
    }
    draw.select(2);
    draw.texture_binds += 1;
    draw.begin_calls += 1;
    draw_line_border(draw, widget, unit);
    draw.end_calls += 1;
    draw.select(1);
    let inset = unit;
    let left = f(widget.x.wrapping_add(inset));
    let top = f(widget.y.wrapping_add(inset));
    let right = f(widget.x.wrapping_add(widget.w).wrapping_sub(inset));
    let bottom = f(widget.y.wrapping_add(widget.h).wrapping_sub(inset));
    let mut ink = widget.face;
    ink[3] = LINE_OPAQUE;
    draw.bind(ink);
    draw.push_solid([(left, top), (right, top), (right, bottom), (left, bottom)]);
    draw.select(0x10);
}

/// The thin chrome of a line window: six-unit corner squares, the title's
/// bars and caps, the side and bottom rails, and the progress bar when a
/// slider sits on the title.
fn draw_line_window(draw: &mut LineDraw, widget: &LineWidget, unit: i32, measured: i32) {
    let x = 0.0;
    let width = f(widget.w);
    let height = f(widget.h);
    let six = f(unit.wrapping_mul(6));
    let four = f(unit.wrapping_shl(2));
    draw.push(LINE_EDGE, [(x, 0.0), (x, six), (six, six), (six, 0.0)]);
    if (widget.state & 2) == 2 && widget.close_mark != -4 {
        draw.push(
            LINE_CLOSE,
            [
                (width, 0.0),
                (width - four, 0.0),
                (width - four, four),
                (width, four),
            ],
        );
    }
    let top_right = if (widget.state & 2) == 2 {
        LINE_EDGE_ALT
    } else {
        LINE_EDGE
    };
    draw.push(
        top_right,
        [
            (width, 0.0),
            (width - six, 0.0),
            (width - six, six),
            (width, six),
        ],
    );
    draw.push(
        LINE_EDGE,
        [
            (x, height),
            (x, height - six),
            (six, height - six),
            (six, height),
        ],
    );
    draw.push(
        LINE_EDGE,
        [
            (width, height),
            (width, height - six),
            (width - six, height - six),
            (width - six, height),
        ],
    );
    let bar = f(unit.wrapping_shl(1));
    let inset = f(double_to_int(f64::from(unit) * 6.6));
    let mut thumb = 0;
    if measured > 0 {
        // The original divides the whole difference, not the final term.
        let mid = ((i64::from(widget.w) - i64::from(measured) - i64::from(unit.wrapping_mul(5)))
            / 2) as i32;
        thumb = mid;
        draw.push(
            LINE_TRACK,
            [(inset, 0.0), (inset, bar), (f(mid), bar), (f(mid), 0.0)],
        );
        draw.push(
            LINE_TRACK,
            [
                (width - f(mid), 0.0),
                (width - f(mid), bar),
                (width - inset, bar),
                (width - inset, 0.0),
            ],
        );
        draw.push(
            LINE_CAP,
            [
                (f(mid), 0.0),
                (f(mid), bar),
                (f(mid) + bar, bar),
                (f(mid) + bar, 0.0),
            ],
        );
        draw.push(
            LINE_CAP,
            [
                (width - f(mid), 0.0),
                (width - f(mid), bar),
                (width - f(mid) - bar, bar),
                (width - f(mid) - bar, 0.0),
            ],
        );
    } else {
        draw.push(
            LINE_TRACK,
            [
                (inset, 0.0),
                (inset, bar),
                (width - inset, bar),
                (width - inset, 0.0),
            ],
        );
    }
    draw.push(
        LINE_TRACK,
        [
            (0.0, height - inset),
            (bar, height - inset),
            (bar, inset),
            (0.0, inset),
        ],
    );
    draw.push(
        LINE_TRACK,
        [
            (inset, height),
            (inset, height - bar),
            (width - inset, height - bar),
            (width - inset, height),
        ],
    );
    draw.push(
        LINE_TRACK,
        [
            (width, height - inset),
            (width - bar, height - inset),
            (width - bar, inset),
            (width, inset),
        ],
    );
    if (widget.state & 1) == 1 && (widget.state & 0x80_0000) == 0 {
        draw_line_progress(draw, widget, unit, thumb);
    }
}

/// The bar chrome of a line window: the frame, its orientation arms, the
/// close-edge strips and the scroll bar's ends.
fn draw_line_bars(draw: &mut LineDraw, widget: &LineWidget, inks: &LineInks, unit: i32) {
    let width = widget.w;
    let height = widget.h;
    if (widget.style_b & 4) == 0 {
        draw.spans.first = unit.wrapping_shl(1);
        draw.spans.third = unit.wrapping_shl(2);
        if (widget.state & 1) == 0 {
            draw_line_progress(draw, widget, unit, 0);
        }
        draw.spans.first = unit.wrapping_shl(1);
        draw.spans.third = unit.wrapping_shl(2);
        return;
    }
    let mask = scroll_mask(widget, unit);
    draw.spans.first = 0;
    draw.spans.third = unit.wrapping_shl(1);
    let side = unit.wrapping_shl(1);
    let six = unit.wrapping_mul(6);
    draw.bind(if (widget.style_b & 0x40) == 0x40 {
        inks.hot
    } else {
        inks.normal
    });
    if (mask & 1) == 0 || (widget.owner().state & 1) == 0 || (widget.owner().state & 0x80_0000) != 0
    {
        draw.push(
            LINE_SCROLL_BAR,
            [
                (f(six), 0.0),
                (f(six), f(side)),
                (f(width.wrapping_sub(six)), f(side)),
                (f(width.wrapping_sub(six)), 0.0),
            ],
        );
    }
    if (widget.state & 0x200) != 0 {
        let slant = double_to_int(f64::from(unit) * 0.15);
        let right = widget.client_right.wrapping_add(slant);
        let mut drop = if (mask & 5) == 5 {
            widget.client_top.wrapping_add(unit.wrapping_mul(3))
        } else {
            widget.client_top.wrapping_add(unit.wrapping_shl(1))
        };
        if widget.close_mark == -9 {
            draw.bind(LINE_CLOSE_INK);
        } else {
            draw.bind(inks.normal);
        }
        draw.push(
            LINE_SCROLL_THUMB,
            [
                (
                    f(unit.wrapping_add(right)),
                    f(unit.wrapping_shl(1).wrapping_add(drop)),
                ),
                (f(unit.wrapping_add(right)), f(drop)),
                (f(unit.wrapping_mul(3).wrapping_add(right)), f(drop)),
                (
                    f(unit.wrapping_mul(3).wrapping_add(right)),
                    f(unit.wrapping_shl(1).wrapping_add(drop)),
                ),
            ],
        );
        drop = if (mask & 0xc) == 0xc {
            widget.client_bottom.wrapping_sub(unit.wrapping_mul(3))
        } else {
            widget.client_bottom.wrapping_sub(unit.wrapping_shl(1))
        };
        if widget.close_mark == -10 {
            draw.bind(LINE_CLOSE_INK);
        } else {
            draw.bind(inks.normal);
        }
        draw.push(
            LINE_SCROLL_THUMB,
            [
                (
                    f(unit.wrapping_add(right)),
                    f(drop.wrapping_sub(unit.wrapping_shl(1))),
                ),
                (f(unit.wrapping_add(right)), f(drop)),
                (f(unit.wrapping_mul(3).wrapping_add(right)), f(drop)),
                (
                    f(unit.wrapping_mul(3).wrapping_add(right)),
                    f(drop.wrapping_sub(unit.wrapping_shl(1))),
                ),
            ],
        );
        let thumb = widget.thumb.wrapping_sub(unit);
        let left = widget.client_right.wrapping_add(slant);
        if widget.close_mark == -8 {
            draw.bind(LINE_CLOSE_INK);
        } else {
            draw.bind(inks.normal);
        }
        draw.push(
            LINE_CORNER,
            [
                (f(left), f(unit.wrapping_shl(2).wrapping_add(thumb))),
                (f(left), f(thumb)),
                (f(unit.wrapping_shl(2).wrapping_add(left)), f(thumb)),
                (
                    f(unit.wrapping_shl(2).wrapping_add(left)),
                    f(unit.wrapping_shl(2).wrapping_add(thumb)),
                ),
            ],
        );
        draw.bind(inks.normal);
    }
    draw.bind(if (widget.style_b & 0x44) == 0x44 {
        inks.hot
    } else {
        inks.normal
    });
    draw.push(
        LINE_SCROLL_BAR,
        [
            (0.0, f(height.wrapping_sub(six))),
            (f(side), f(height.wrapping_sub(six))),
            (f(side), f(six)),
            (0.0, f(six)),
        ],
    );
    if (mask & 3) == 3 {
        draw.push(
            LINE_BAR,
            [
                (0.0, f(side)),
                (f(side), f(side)),
                (f(side), f(six)),
                (0.0, f(six)),
            ],
        );
        if ((widget.state & 1) == 0 && (widget.owner().state & 1) == 0)
            || (widget.owner().state & 0x80_0000) != 0
        {
            draw.push(
                LINE_BAR,
                [
                    (f(side), 0.0),
                    (f(side), f(side)),
                    (f(six), f(side)),
                    (f(six), 0.0),
                ],
            );
        }
    } else {
        draw.push(
            LINE_FRAME,
            [
                (-f(side), -f(side)),
                (-f(side), f(six)),
                (f(six), f(six)),
                (f(six), -f(side)),
            ],
        );
    }
    if (mask & 5) == 5 {
        if (widget.state & 0x200) == 0 {
            draw.push(
                LINE_BAR,
                [
                    (f(width.wrapping_sub(side)), f(side)),
                    (f(width.wrapping_sub(unit.wrapping_shl(2))), f(side)),
                    (f(width.wrapping_sub(unit.wrapping_shl(2))), f(six)),
                    (f(width.wrapping_sub(side)), f(six)),
                ],
            );
            if ((widget.state & 1) == 0 && (widget.owner().state & 1) == 0)
                || (widget.owner().state & 0x80_0000) != 0
            {
                draw.push(
                    LINE_BAR,
                    [
                        (f(width.wrapping_sub(side)), 0.0),
                        (f(width.wrapping_sub(side)), f(side)),
                        (f(width.wrapping_sub(six)), f(side)),
                        (f(width.wrapping_sub(six)), 0.0),
                    ],
                );
            }
        } else {
            draw.push(
                LINE_END,
                [
                    (f(width.wrapping_sub(side)), f(six)),
                    (f(width.wrapping_sub(side)), f(side)),
                    (f(width.wrapping_sub(six)), f(side)),
                    (f(width.wrapping_sub(six)), f(six)),
                ],
            );
        }
    } else if (widget.state & 0x200) == 0 {
        draw.push(
            LINE_FRAME,
            [
                (f(width.wrapping_add(side)), -f(side)),
                (f(width.wrapping_add(side)), f(six)),
                (f(width.wrapping_sub(six)), f(six)),
                (f(width.wrapping_sub(six)), -f(side)),
            ],
        );
    } else {
        draw.push(
            LINE_FRAME_ALT,
            [
                (f(width.wrapping_add(side)), -f(side)),
                (f(width.wrapping_add(side)), f(six)),
                (f(width.wrapping_sub(six)), f(six)),
                (f(width.wrapping_sub(six)), -f(side)),
            ],
        );
    }
    if (widget.state & 0x200) == 0 {
        draw.push(
            LINE_SCROLL_BAR,
            [
                (f(width.wrapping_sub(side)), f(height.wrapping_sub(six))),
                (
                    f(width.wrapping_sub(unit.wrapping_shl(2))),
                    f(height.wrapping_sub(six)),
                ),
                (f(width.wrapping_sub(unit.wrapping_shl(2))), f(six)),
                (f(width.wrapping_sub(side)), f(six)),
            ],
        );
    } else {
        draw.push(
            LINE_RIGHT,
            [
                (f(width.wrapping_sub(unit.wrapping_shl(2))), f(six)),
                (
                    f(width.wrapping_sub(unit.wrapping_shl(2))),
                    f(height.wrapping_sub(six)),
                ),
                (f(width.wrapping_sub(six)), f(height.wrapping_sub(six))),
                (f(width.wrapping_sub(six)), f(six)),
            ],
        );
        if (mask & 4) == 0 && (widget.flags & 0x200) == 0 {
            let four = unit.wrapping_shl(2);
            let middle = width.wrapping_add(double_to_int(f64::from(unit) * 0.2));
            let low = f(unit.wrapping_add(middle));
            let high = f(middle.wrapping_sub(unit));
            draw.push(
                LINE_SCROLL_BAR,
                [
                    (low, f(height.wrapping_sub(four))),
                    (high, f(height.wrapping_sub(four))),
                    (high, f(four)),
                    (low, f(four)),
                ],
            );
            draw.push(
                LINE_SCROLL_EDGE,
                [
                    (low, f(four)),
                    (high, f(four)),
                    (high, f(unit.wrapping_shl(1))),
                    (low, f(unit.wrapping_shl(1))),
                ],
            );
            draw.push(
                LINE_SCROLL_EDGE,
                [
                    (low, f(height.wrapping_sub(four))),
                    (high, f(height.wrapping_sub(four))),
                    (high, f(height.wrapping_sub(unit.wrapping_shl(1)))),
                    (low, f(height.wrapping_sub(unit.wrapping_shl(1)))),
                ],
            );
        }
    }
    if (mask & 0xa) == 0xa {
        draw.push(
            LINE_BAR,
            [
                (0.0, f(height.wrapping_sub(side))),
                (f(side), f(height.wrapping_sub(side))),
                (f(side), f(height.wrapping_sub(six))),
                (0.0, f(height.wrapping_sub(six))),
            ],
        );
        draw.push(
            LINE_BAR,
            [
                (f(side), f(height)),
                (f(side), f(height.wrapping_sub(side))),
                (f(six), f(height.wrapping_sub(side))),
                (f(six), f(height)),
            ],
        );
    } else {
        draw.push(
            LINE_FRAME,
            [
                (-f(side), f(height.wrapping_add(side))),
                (-f(side), f(height.wrapping_sub(six))),
                (f(six), f(height.wrapping_sub(six))),
                (f(six), f(height.wrapping_add(side))),
            ],
        );
    }
    draw.push(
        LINE_SCROLL_BAR,
        [
            (f(six), f(height)),
            (f(six), f(height.wrapping_sub(side))),
            (f(width.wrapping_sub(six)), f(height.wrapping_sub(side))),
            (f(width.wrapping_sub(six)), f(height)),
        ],
    );
    if (mask & 0xc) == 0xc {
        if (widget.state & 0x200) == 0 {
            draw.push(
                LINE_BAR,
                [
                    (f(width.wrapping_sub(side)), f(height.wrapping_sub(side))),
                    (
                        f(width.wrapping_sub(unit.wrapping_shl(2))),
                        f(height.wrapping_sub(side)),
                    ),
                    (
                        f(width.wrapping_sub(unit.wrapping_shl(2))),
                        f(height.wrapping_sub(six)),
                    ),
                    (f(width.wrapping_sub(side)), f(height.wrapping_sub(six))),
                ],
            );
        } else {
            draw.push(
                LINE_END,
                [
                    (
                        f(width.wrapping_sub(unit.wrapping_shl(2))),
                        f(height.wrapping_sub(six)),
                    ),
                    (
                        f(width.wrapping_sub(unit.wrapping_shl(2))),
                        f(height.wrapping_sub(side)),
                    ),
                    (f(width.wrapping_sub(six)), f(height.wrapping_sub(side))),
                    (f(width.wrapping_sub(six)), f(height.wrapping_sub(six))),
                ],
            );
        }
        draw.push(
            LINE_BAR,
            [
                (f(width.wrapping_sub(side)), f(height)),
                (f(width.wrapping_sub(side)), f(height.wrapping_sub(side))),
                (f(width.wrapping_sub(six)), f(height.wrapping_sub(side))),
                (f(width.wrapping_sub(six)), f(height)),
            ],
        );
    } else if (widget.state & 0x200) == 0 {
        draw.push(
            LINE_FRAME,
            [
                (f(width.wrapping_add(side)), f(height.wrapping_add(side))),
                (f(width.wrapping_add(side)), f(height.wrapping_sub(six))),
                (f(width.wrapping_sub(six)), f(height.wrapping_sub(six))),
                (f(width.wrapping_sub(six)), f(height.wrapping_add(side))),
            ],
        );
    } else {
        draw.push(
            LINE_FRAME_ALT,
            [
                (f(width.wrapping_add(side)), f(height.wrapping_add(side))),
                (f(width.wrapping_add(side)), f(height.wrapping_sub(six))),
                (f(width.wrapping_sub(six)), f(height.wrapping_sub(six))),
                (f(width.wrapping_sub(six)), f(height.wrapping_add(side))),
            ],
        );
    }
    draw.spans.first = unit.wrapping_shl(1);
    draw.spans.third = unit.wrapping_shl(2);
}

/// The maximised window chrome: a double thin frame around the whole widget.
fn draw_line_maximized(draw: &mut LineDraw, widget: &LineWidget, inks: &LineInks, unit: i32) {
    draw.bind(if (widget.style_b & 0x40) == 0x40 {
        inks.hot
    } else {
        inks.normal
    });
    draw.spans.first = 0;
    draw.spans.third = unit.wrapping_shl(1);
    let width = f(widget.w);
    let height = f(widget.h);
    let u = f(unit);
    let two = f(unit.wrapping_shl(1));
    draw.push(LINE_SCROLL_EDGE, [(two, u), (two, -u), (0.0, -u), (0.0, u)]);
    draw.push(
        LINE_SCROLL_BAR,
        [(two, u), (two, -u), (width - two, -u), (width - two, u)],
    );
    draw.push(
        LINE_SCROLL_EDGE,
        [(width - two, u), (width - two, -u), (width, -u), (width, u)],
    );
    draw.push(
        LINE_SCROLL_BAR,
        [(u, height - two), (-u, height - two), (-u, two), (u, two)],
    );
    draw.push(LINE_SCROLL_EDGE, [(u, two), (-u, two), (-u, -u), (u, -u)]);
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (u, height - two),
            (-u, height - two),
            (-u, height + u),
            (u, height + u),
        ],
    );
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (width - u, height - two),
            (width + u, height - two),
            (width + u, two),
            (width - u, two),
        ],
    );
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (width - u, two),
            (width + u, two),
            (width + u, -u),
            (width - u, -u),
        ],
    );
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (width - u, height - two),
            (width + u, height - two),
            (width + u, height + u),
            (width - u, height + u),
        ],
    );
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (two, height - u),
            (two, height + u),
            (width - two, height + u),
            (width - two, height - u),
        ],
    );
    draw.push(
        LINE_SCROLL_BAR,
        [
            (two, height - u),
            (two, height + u),
            (width - two, height + u),
            (width - two, height - u),
        ],
    );
    draw.push(
        LINE_SCROLL_EDGE,
        [
            (two, height - two),
            (two, height + two),
            (width - two, height + two),
            (width - two, height - two),
        ],
    );
    draw.push(
        LINE_SCROLL_BAR,
        [
            (two, height - two),
            (two, height + two),
            (width - two, height + two),
            (width - two, height - two),
        ],
    );
    if (widget.style_b & 0x40) != 0 {
        draw.bind(inks.normal);
    }
}

/// Releases the draw's temporary text slot. An empty slot is not freed, but
/// the release is still attempted once.
pub fn release_line_text(draw: &mut LineDraw) {
    draw.release_calls += 1;
}

/// Draws one line widget into `draw`: the clip, the shadow and backdrop, the
/// caption, the window or bar chrome, the children and the close square.
///
/// The original also asks the widget to draw its own content and calls the
/// visible children in order. The batch records those calls as
/// [`LineDraw::self_draw_calls`] and [`LineDraw::children`]; the caller walks
/// its own children in the same order.
pub fn draw_line_widget(draw: &mut LineDraw, widget: &LineWidget, inks: &LineInks, unit: i32) {
    draw.spans.first = unit.wrapping_shl(1);
    draw.spans.second = unit.wrapping_mul(3);
    draw.spans.third = unit.wrapping_shl(2);

    if (widget.state & 0x40) == 0x40 {
        draw.shadow = Some(LineShadow {
            width: widget.w,
            height: widget.h,
            corner: 8,
            alpha: inks.shadow_alpha,
        });
    }

    if (widget.state & 0x2_0000) != 0 && (widget.style_b & 4) == 0 {
        draw.select(1);
        draw.bind(inks.backdrop);
        draw.begin_calls += 1;
        draw.texture_binds += 0;
        draw.backdrop = Some(LineQuad {
            sample: [0; 8],
            corners: [
                (0.0, 0.0),
                (0.0, f(widget.h)),
                (f(widget.w), f(widget.h)),
                (f(widget.w), 0.0),
            ],
            ink: inks.backdrop,
        });
        draw.end_calls += 1;
    }

    let (scroll_x, scroll_y) = if (widget.state & 0x800) == 0x800 {
        (0, 0)
    } else {
        let span_x = i64::from(widget.scroll_x)
            - (i64::from(widget.client_right) - i64::from(widget.client_left));
        let span_y = i64::from(widget.scroll_y)
            - (i64::from(widget.client_bottom) - i64::from(widget.client_top));
        (
            float_to_int(span_x as f32 * widget.scroll_scale_x),
            float_to_int(span_y as f32 * widget.scroll_scale_y),
        )
    };
    draw.scroll = [scroll_x, scroll_y];
    draw.clip = [
        f(widget.x.wrapping_add(widget.client_left)),
        f(widget.y.wrapping_add(widget.client_top)),
        f(widget.client_right.wrapping_sub(widget.client_left)),
        f(widget.client_bottom.wrapping_sub(widget.client_top)),
    ];
    draw.clip_calls += 1;
    draw.clip_pushed = true;

    draw.self_draw_calls += 1;
    for (index, visible) in widget.visible_children.iter().enumerate() {
        if *visible != 0 {
            draw.children.push(index);
        }
    }

    draw.select(2);
    let mut measured = 0;
    if (widget.state & 1) == 1 {
        measured = widget.title_width;
        if measured > 0 {
            let x = (i64::from(widget.w) - i64::from(measured)) / 2;
            draw.title = Some(LineTitle {
                x: x as i32,
                y: widget.title_ascent,
                width: widget.w,
                ink: widget.title_tint.unwrap_or(inks.caption),
            });
        }
    }

    draw.texture_binds += 1;
    draw.bind(inks.normal);
    draw.begin_calls += 1;
    if (widget.style & 0x30) == 0x10 {
        if (widget.state & 0x40_0000) != 0 {
            draw_line_scroll(draw, widget, inks, unit);
            draw.bind(inks.normal);
        } else {
            draw_line_window(draw, widget, unit, measured);
        }
    }
    if (widget.style & 0xc0) == 0x80 {
        if (widget.state & 0x40_0000) == 0 {
            draw_line_bars(draw, widget, inks, unit);
        } else {
            draw_line_maximized(draw, widget, inks, unit);
        }
    }
    draw.end_calls += 1;

    if widget.close_mark == -4 {
        let side = f(unit.wrapping_shl(2));
        draw.bind(LINE_CLOSE_INK);
        draw.begin_calls += 1;
        draw.push(
            LINE_CLOSE,
            [
                (f(widget.w), 0.0),
                (f(widget.w) - side, 0.0),
                (f(widget.w) - side, side),
                (f(widget.w), side),
            ],
        );
        draw.end_calls += 1;
    }
    draw.select(0);
    release_line_text(draw);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inks() -> LineInks {
        LineInks {
            pressed: [1, 2, 3, 4],
            normal: [5, 6, 7, 8],
            alternate: [9, 10, 11, 12],
            hot: [13, 14, 15, 16],
            backdrop: [17, 18, 19, 20],
            shadow_alpha: 21,
            caption: [22, 23, 24, 25],
            label: [26, 27, 28, 29],
            label_plain: [30, 31, 32, 33],
            label_hot: [34, 35, 36, 37],
            icon: [38, 39, 40, 41],
            icon_hot: [42, 43, 44, 45],
            icon_focus: [46, 47, 48, 49],
            icon_raised: [50, 51, 52, 53],
        }
    }

    fn widget<'a>() -> LineWidget<'a> {
        LineWidget {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            style: 0,
            style_b: 0,
            state: 0,
            flags: 0,
            close_mark: 0,
            face: [60, 61, 62, 63],
            client_left: 10,
            client_top: 20,
            client_bottom: 100,
            client_right: 110,
            scroll_x: 0,
            scroll_y: 0,
            scroll_scale_x: 0.0,
            scroll_scale_y: 0.0,
            title: b"",
            title_width: 0,
            title_ascent: 0,
            title_tint: None,
            owner: Some(LineOwner {
                left: 0,
                top: 0,
                right: 200,
                bottom: 200,
                state: 0,
            }),
            thumb: 0,
            visible_children: &[],
        }
    }

    fn corners(quad: &LineQuad) -> [(i32, i32); 4] {
        quad.corners.map(|(x, y)| (x as i32, y as i32))
    }

    #[test]
    fn the_float_conversions_round_to_even_and_keep_the_indefinite_value() {
        assert_eq!(float_to_int(0.5), 0);
        assert_eq!(float_to_int(1.5), 2);
        assert_eq!(float_to_int(2.5), 2);
        assert_eq!(float_to_int(-0.5), 0);
        assert_eq!(float_to_int(-1.5), -2);
        assert_eq!(float_to_int(f32::NAN), i32::MIN);
        assert_eq!(float_to_int(2_147_483_648.0), i32::MIN);
        assert_eq!(double_to_int(6.5), 6);
        assert_eq!(double_to_int(-6.5), -6);
        assert_eq!(double_to_int(f64::INFINITY), i32::MIN);
    }

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

    #[test]
    fn the_scroll_refresh_places_the_thumb_from_the_owner_room() {
        let mut place = widget();
        place.style_b = 4;
        place.scroll_scale_y = 0.5;
        let scroll = refresh_line_scroll(&place, 5);
        assert_eq!(scroll.mask, 0);
        assert_eq!(scroll.start, 40);
        assert_eq!(scroll.end, 80);
        assert_eq!(scroll.side, 10);
        assert_eq!(scroll.thumb, 55);

        let mut plain = widget();
        plain.scroll_scale_y = 0.5;
        let scroll = refresh_line_scroll(&plain, 5);
        assert_eq!(scroll.mask, 0xf);
        assert_eq!(scroll.start, 45);
        assert_eq!(scroll.end, 75);
        assert_eq!(scroll.thumb, 55);

        let mut tight = widget();
        tight.style_b = 4;
        tight.x = 2;
        tight.y = 2;
        tight.owner = Some(LineOwner {
            left: 0,
            top: 0,
            right: 100,
            bottom: 90,
            state: 0,
        });
        assert_eq!(refresh_line_scroll(&tight, 5).mask, 0x7);
    }

    #[test]
    fn a_progress_bar_draws_two_caps_a_track_and_the_thumb_caps() {
        let mut draw = LineDraw::new();
        draw.spans = LineSpans {
            first: 10,
            second: 15,
            third: 20,
        };
        draw.bind(inks().normal);
        draw_line_progress(&mut draw, &widget(), 5, 0);
        assert_eq!(draw.quads.len(), 3);
        assert_eq!(
            corners(&draw.quads[0]),
            [(20, 10), (20, 20), (40, 20), (40, 10)]
        );
        assert_eq!(
            corners(&draw.quads[1]),
            [(80, 10), (80, 20), (60, 20), (60, 10)]
        );
        assert_eq!(
            corners(&draw.quads[2]),
            [(40, 10), (40, 20), (60, 20), (60, 10)]
        );
        assert_eq!(draw.quads[2].sample, LINE_SCROLL_BAR);
        assert_eq!(draw.quads[0].ink, [5, 6, 7, 8]);

        let mut draw = LineDraw::new();
        draw.spans = LineSpans {
            first: 10,
            second: 15,
            third: 20,
        };
        draw.bind(inks().normal);
        draw_line_progress(&mut draw, &widget(), 5, 30);
        assert_eq!(draw.quads.len(), 6);
        assert_eq!(
            corners(&draw.quads[2]),
            [(40, 10), (40, 20), (30, 20), (30, 10)]
        );
        assert_eq!(
            corners(&draw.quads[3]),
            [(60, 10), (60, 20), (70, 20), (70, 10)]
        );
        assert_eq!(draw.quads[4].sample, LINE_SCROLL_EDGE);
        assert_eq!(
            corners(&draw.quads[4]),
            [(30, 10), (30, 20), (40, 20), (40, 10)]
        );
    }

    #[test]
    fn a_border_draws_two_lines_six_units_outside_the_middle() {
        let mut draw = LineDraw::new();
        draw.spans = LineSpans {
            first: 10,
            second: 15,
            third: 20,
        };
        draw.bind(inks().normal);
        draw_line_border(&mut draw, &widget(), 5);
        assert_eq!(draw.quads.len(), 6);
        assert_eq!(
            corners(&draw.quads[0]),
            [(20, 23), (20, 13), (10, 13), (10, 23)]
        );
        assert_eq!(draw.quads[1].sample, LINE_SCROLL_BAR);
        assert_eq!(
            corners(&draw.quads[1]),
            [(20, 23), (20, 13), (100, 13), (100, 23)]
        );
        assert_eq!(
            corners(&draw.quads[2]),
            [(100, 23), (100, 13), (110, 13), (110, 23)]
        );
        assert_eq!(
            corners(&draw.quads[3]),
            [(20, 97), (20, 107), (10, 107), (10, 97)]
        );
        assert_eq!(
            corners(&draw.quads[5]),
            [(100, 97), (100, 107), (110, 107), (110, 97)]
        );
    }

    #[test]
    fn a_marker_is_a_four_unit_square_centred_on_its_point() {
        let mut draw = LineDraw::new();
        draw.bind(inks().normal);
        draw_line_marker(&mut draw, 5, 10, 20);
        assert_eq!(draw.quads.len(), 1);
        assert_eq!(
            corners(&draw.quads[0]),
            [(1, 31), (1, 11), (21, 11), (21, 31)]
        );
        assert_eq!(draw.quads[0].sample, SPAN_END);
    }

    #[test]
    fn a_detail_strip_is_a_bar_between_two_caps() {
        let mut draw = LineDraw::new();
        draw.spans.first = 10;
        draw_line_detail(&mut draw, 10, 20, 100, 5);
        assert_eq!(draw.quads.len(), 3);
        assert_eq!(
            corners(&draw.quads[0]),
            [(20, 20), (20, 30), (100, 30), (100, 20)]
        );
        assert_eq!(
            corners(&draw.quads[1]),
            [(20, 20), (20, 30), (10, 30), (10, 20)]
        );
        assert_eq!(
            corners(&draw.quads[2]),
            [(100, 20), (100, 30), (110, 30), (110, 20)]
        );
    }

    #[test]
    fn the_two_open_bars_draw_caps_and_a_middle() {
        let mut draw = LineDraw::new();
        draw.bind(inks().normal);
        draw_line_bar(&mut draw, 10, 20, 100, 5);
        assert_eq!(draw.quads.len(), 3);
        assert_eq!(
            corners(&draw.quads[0]),
            [(0, 20), (0, 40), (30, 40), (30, 20)]
        );
        assert_eq!(
            corners(&draw.quads[1]),
            [(120, 20), (120, 40), (90, 40), (90, 20)]
        );
        assert_eq!(
            corners(&draw.quads[2]),
            [(30, 30), (30, 40), (90, 40), (90, 30)]
        );

        let mut draw = LineDraw::new();
        draw.bind(inks().normal);
        draw_line_rule(&mut draw, 10.0, 110.0, 100.0, 5);
        assert_eq!(draw.quads.len(), 3);
        assert_eq!(
            corners(&draw.quads[0]),
            [(0, 90), (0, 110), (30, 110), (30, 90)]
        );
        assert_eq!(
            corners(&draw.quads[1]),
            [(120, 90), (120, 110), (90, 110), (90, 90)]
        );
        assert_eq!(
            corners(&draw.quads[2]),
            [(30, 100), (30, 110), (90, 110), (90, 100)]
        );
    }

    #[test]
    fn the_scroll_rail_draws_its_top_bar_twice_when_the_top_is_in_view() {
        let mut place = widget();
        place.style_b = 4;
        let mut draw = LineDraw::new();
        draw_line_scroll(&mut draw, &place, &inks(), 5);
        assert_eq!(draw.quads.len(), 9);
        assert_eq!(
            corners(&draw.quads[0]),
            [(30, 0), (30, 10), (70, 10), (70, 0)]
        );
        assert_eq!(
            corners(&draw.quads[1]),
            [(30, 0), (30, 10), (70, 10), (70, 0)]
        );
        assert_eq!(
            corners(&draw.quads[2]),
            [(0, 70), (10, 70), (10, 10), (0, 10)]
        );
        assert_eq!(draw.quads[2].sample, LINE_SCROLL_BAR);
        assert_eq!(
            corners(&draw.quads[3]),
            [(-10, -10), (-10, 10), (30, 10), (30, -10)]
        );
        assert_eq!(draw.spans.first, 10);
        assert_eq!(draw.spans.third, 20);
    }

    #[test]
    fn the_line_label_picks_its_ink_and_place() {
        let mut place = widget();
        let label = draw_line_label(&place, &inks(), 3, 4);
        assert_eq!(label.ink, [30, 31, 32, 33]);
        assert!(label.binds_ink);
        assert_eq!((label.x, label.y), (3, 4));

        place.state = 0x10;
        assert_eq!(draw_line_label(&place, &inks(), 3, 4).ink, [26, 27, 28, 29]);

        place.state = 0x1_0000;
        let dim = draw_line_label(&place, &inks(), 3, 4);
        assert!(!dim.binds_ink);
        assert_eq!((dim.x, dim.y), (3, 4));

        place.state = 0x40;
        let hot = draw_line_label(&place, &inks(), 3, 4);
        assert_eq!(hot.ink, [34, 35, 36, 37]);
        assert_eq!(hot.y, 5);
    }

    #[test]
    fn a_rectangle_slot_draws_a_border_and_an_inset_face() {
        let mut place = widget();
        place.state = 0x1_0000;
        let mut draw = LineDraw::new();
        draw.spans = LineSpans {
            first: 10,
            second: 15,
            third: 20,
        };
        draw_line_rect(&mut draw, &place, &inks(), 5);
        assert_eq!(draw.quads.len(), 6);
        assert_eq!(draw.quads[0].ink, [13, 14, 15, 255]);
        assert_eq!(draw.solids.len(), 1);
        assert_eq!(
            draw.solids[0].corners,
            [(15.0, 25.0), (105.0, 25.0), (105.0, 95.0), (15.0, 95.0)]
        );
        assert_eq!(draw.solids[0].ink, [60, 61, 62, 255]);
        assert_eq!(draw.mode, 0x10);
    }

    #[test]
    fn an_empty_widget_draws_only_its_clip_and_its_mode() {
        let mut draw = LineDraw::new();
        draw_line_widget(&mut draw, &widget(), &inks(), 5);
        assert_eq!(draw.mode_calls, 2);
        assert_eq!(draw.mode, 0);
        assert_eq!(draw.texture_binds, 1);
        assert_eq!(draw.ink_binds, 1);
        assert_eq!(draw.begin_calls, 1);
        assert_eq!(draw.end_calls, 1);
        assert_eq!(draw.quads.len(), 0);
        assert_eq!(draw.clip, [20.0, 40.0, 100.0, 80.0]);
        assert!(draw.clip_pushed);
        assert_eq!(draw.self_draw_calls, 1);
        assert_eq!(draw.release_calls, 1);
    }

    #[test]
    fn a_window_titles_its_caption_and_closes_with_the_late_square() {
        let mut place = widget();
        place.style = 0x10;
        place.state = 0x2 | 0x1;
        place.close_mark = -4;
        place.title = b"title";
        place.title_width = 40;
        place.title_ascent = 18;
        let mut draw = LineDraw::new();
        draw_line_widget(&mut draw, &place, &inks(), 5);
        let title = draw.title.expect("a caption");
        assert_eq!((title.x, title.y, title.width), (30, 18, 100));
        assert_eq!(title.ink, [22, 23, 24, 25]);
        assert_eq!(draw.quads.len(), 18);
        assert_eq!(
            corners(&draw.quads[0]),
            [(0, 0), (0, 30), (30, 30), (30, 0)]
        );
        assert_eq!(
            corners(&draw.quads[1]),
            [(100, 0), (70, 0), (70, 30), (100, 30)]
        );
        assert_eq!(draw.quads[1].sample, LINE_EDGE_ALT);
        assert_eq!(
            corners(&draw.quads[4]),
            [(33, 0), (33, 10), (17, 10), (17, 0)]
        );
        assert_eq!(
            corners(&draw.quads[8]),
            [(0, 47), (10, 47), (10, 33), (0, 33)]
        );
        assert_eq!(
            corners(&draw.quads[17]),
            [(100, 0), (80, 0), (80, 20), (100, 20)]
        );
        assert_eq!(draw.quads[17].ink, LINE_CLOSE_INK);
    }

    #[test]
    fn a_bar_window_draws_its_close_strips_and_frame_arms() {
        let mut place = widget();
        place.style = 0x80;
        place.style_b = 0x44;
        place.state = 0x200;
        place.thumb = 30;
        let mut draw = LineDraw::new();
        draw_line_widget(&mut draw, &place, &inks(), 5);
        assert_eq!(draw.quads.len(), 14);
        assert_eq!(
            corners(&draw.quads[0]),
            [(30, 0), (30, 10), (70, 10), (70, 0)]
        );
        assert_eq!(
            corners(&draw.quads[1]),
            [(116, 40), (116, 30), (126, 30), (126, 40)]
        );
        assert_eq!(
            corners(&draw.quads[2]),
            [(116, 80), (116, 90), (126, 90), (126, 80)]
        );
        assert_eq!(
            corners(&draw.quads[3]),
            [(111, 45), (111, 25), (131, 25), (131, 45)]
        );
        assert_eq!(
            corners(&draw.quads[4]),
            [(0, 50), (10, 50), (10, 30), (0, 30)]
        );
        assert_eq!(
            corners(&draw.quads[5]),
            [(-10, -10), (-10, 30), (30, 30), (30, -10)]
        );
        assert_eq!(draw.quads[5].sample, LINE_FRAME);
        assert_eq!(
            corners(&draw.quads[8]),
            [(106, 60), (96, 60), (96, 20), (106, 20)]
        );
        assert_eq!(
            corners(&draw.quads[12]),
            [(30, 80), (30, 70), (70, 70), (70, 80)]
        );
    }

    #[test]
    fn a_plain_bar_draws_only_its_progress_track() {
        let mut place = widget();
        place.style = 0x80;
        let mut draw = LineDraw::new();
        draw_line_widget(&mut draw, &place, &inks(), 5);
        assert_eq!(draw.quads.len(), 3);
        assert_eq!(
            corners(&draw.quads[0]),
            [(20, 10), (20, 20), (40, 20), (40, 10)]
        );

        place.state = 1;
        let mut draw = LineDraw::new();
        draw_line_widget(&mut draw, &place, &inks(), 5);
        assert_eq!(draw.quads.len(), 0);
    }

    #[test]
    fn a_maximised_bar_draws_thirteen_thin_arms() {
        let mut place = widget();
        place.style = 0x80;
        place.state = 0x40_0000;
        let mut draw = LineDraw::new();
        draw_line_widget(&mut draw, &place, &inks(), 5);
        assert_eq!(draw.quads.len(), 13);
        assert_eq!(
            corners(&draw.quads[0]),
            [(10, 5), (10, -5), (0, -5), (0, 5)]
        );
        assert_eq!(
            corners(&draw.quads[12]),
            [(10, 70), (10, 90), (90, 90), (90, 70)]
        );
        assert_eq!(draw.spans.first, 0);
        assert_eq!(draw.spans.third, 10);
    }

    #[test]
    fn the_clip_scrolls_the_client_by_the_scaled_room() {
        let mut place = widget();
        place.scroll_x = 50;
        place.scroll_scale_x = 0.5;
        let mut draw = LineDraw::new();
        draw_line_widget(&mut draw, &place, &inks(), 5);
        assert_eq!(draw.scroll, [-25, 0]);
        assert_eq!(draw.clip, [20.0, 40.0, 100.0, 80.0]);

        place.state = 0x800;
        let mut draw = LineDraw::new();
        draw_line_widget(&mut draw, &place, &inks(), 5);
        assert_eq!(draw.scroll, [0, 0]);
        assert_eq!(draw.clip, [20.0, 40.0, 100.0, 80.0]);
    }

    #[test]
    fn the_widget_walks_only_visible_children() {
        let visible = [1u8, 0, 1];
        let mut place = widget();
        place.visible_children = &visible;
        let mut draw = LineDraw::new();
        draw_line_widget(&mut draw, &place, &inks(), 5);
        assert_eq!(draw.children, vec![0, 2]);
    }
}
