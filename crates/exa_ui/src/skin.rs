//! The draw skin's opening metrics. The unit is the GUI scale. The three
//! colours are the same four bytes. The base fill and the call after it are
//! counted. Their bodies are not in.

/// The four colour bytes stored, then copied twice.
pub const SKIN_INK: [u8; 4] = [0xff, 0xfa, 0xe6, 100];

/// Metrics a skin stores when it is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawSkin {
    pub span_a: i32,
    pub span_b: i32,
    pub span_c: i32,
    pub span_d: i32,
    pub span_e: i32,
    pub pad_a: i32,
    pub pad_b: i32,
    pub pad_c: i32,
    pub pad_d: i32,
    pub inks: [[u8; 4]; 3],
    pub mark: i32,
    pub alloc_calls: u32,
    pub base_calls: u32,
    pub after_calls: u32,
}

/// Open a skin for `unit`. Argument 1 counts an allocation, and a failed
/// allocation returns nothing. Any other argument skips that count. A
/// non-zero argument counts the call after the fill.
pub fn open_skin(unit: i32, arg: i32, allocated: bool) -> Option<DrawSkin> {
    let mut alloc_calls = 0;
    let present = if arg == 1 {
        alloc_calls = 1;
        allocated
    } else {
        true
    };
    if !present {
        return None;
    }
    let doubled = unit.wrapping_shl(1);
    let mut skin = DrawSkin {
        span_a: doubled,
        span_b: unit,
        span_c: doubled,
        span_d: doubled,
        span_e: unit.wrapping_shl(2),
        pad_a: 8,
        pad_b: 4,
        pad_c: 8,
        pad_d: 8,
        inks: [SKIN_INK; 3],
        mark: 1,
        alloc_calls,
        base_calls: 1,
        after_calls: 0,
    };
    if arg != 0 {
        skin.after_calls = 1;
    }
    Some(skin)
}

/// Cap sample, then the same sample on the other end.
pub const SLIDER_CAP: [u32; 4] = [0x3e4c_cccd, 0x3f4c_cccd, 0x3e4c_cccd, 0x3f80_0000];

/// Track sample between the caps.
pub const SLIDER_TRACK: [u32; 4] = [0x3e80_0000, 0x3f4c_cccd, 0x3e80_0000, 0x3f80_0000];

/// Thumb sample.
pub const SLIDER_THUMB: [u32; 4] = [0x3ecc_cccd, 0x3f4c_cccd, 0x3ecc_cccd, 0x3f80_0000];

/// End-cap sample on a scroll bar: 0.7, 0.1, 0.7, 0.2.
pub const SCROLL_EDGE: [u32; 4] = [0x3f33_3333, 0x3dcc_cccd, 0x3f33_3333, 0x3e4c_cccd];

/// Bar sample between those caps: 0.3, 0.1, 0.3, 0.2.
pub const SCROLL_BAR: [u32; 4] = [0x3e99_999a, 0x3dcc_cccd, 0x3e99_999a, 0x3e4c_cccd];

/// Corner sample of a frame: 0.4, 0.3, 0.4, 0.45.
pub const FRAME_CORNER: [u32; 4] = [0x3ecc_cccd, 0x3e99_999a, 0x3ecc_cccd, 0x3ee6_6666];

/// Edge sample of a frame: 0.475, 0.3, 0.475, 0.35.
pub const FRAME_EDGE: [u32; 4] = [0x3ef3_3333, 0x3e99_999a, 0x3ef3_3333, 0x3eb3_3333];

const SLIDER_PRESSED: u32 = 0x10;
const SLIDER_SCROLL: u32 = 0x20_0000;
const SLIDER_MARK: u32 = 0x1_0000;
const SLIDER_MODE: u32 = 0x3000_0000;
const SCROLL_MODE: u32 = 0x1000_0000;
const FRAME_WIDE: u32 = 0x80_0000;
const FRAME_HOT: u32 = 0x60;
const ICON_MARK: u32 = 0x1_0000;
const ICON_NUDGE: u32 = 0x40;
const ICON_BAND: u32 = 0x60;
const ICON_STYLE: u32 = 0x0100_0000;
const ICON_TURN: u32 = 0x7000_0000;
const ICON_SCALE: u32 = 0x3f80_0000;
const FORM_SHADE: u32 = 0x40;
const FORM_BACK: u32 = 0x2_0000;
const FORM_HOLD: u32 = 0x4;
const FORM_LOCK: u32 = 0x800;
const FORM_PLAIN: u32 = 0x100;
/// Bit of the plain word. A set bit skips the bar after the right strip.
const FORM_STILL: u32 = 0x200;
const FORM_ONE: u32 = 0x3f80_0000;
const FORM_TITLE: u32 = 1;
const FORM_CLOSE: u32 = 2;
const FORM_BOX: u32 = 0x40_0000;
const FORM_STYLE: u32 = 0x30;
const FORM_STYLE_ON: u32 = 0x10;
const FORM_BAND: u32 = 0xc0;
const FORM_BAND_ON: u32 = 0x80;
const FORM_MARK: u32 = 0x40;
const FORM_TAB: u32 = 0x200;
const TAB_SCALE: u64 = 0x3fc3_3333_3333_3333;
/// The double 0.15 that slants the close-edge tint.
const CLOSE_SLANT: u64 = 0x3fc3_3333_3333_3333;

/// Corner sample after the child walk: 0, 0, 0, 0.3.
pub const FORM_CORNER: [u32; 4] = [0, 0, 0, 0x3e99_999a];

/// Sample used for the top corner when bit 2 is set: 0, 0.4, 0, 0.7.
pub const FORM_ALT: [u32; 4] = [0, 0x3ecc_cccd, 0, 0x3f33_3333];

/// Close square: 0, 0.8, 0, 1.
pub const FORM_CLOSE_INK: [u32; 4] = [0, 0x3f4c_cccd, 0, 0x3f80_0000];

/// Edge bar: 0.3, 0, 0.3, 0.1.
pub const FORM_EDGE: [u32; 4] = [0x3e99_999a, 0, 0x3e99_999a, 0x3dcc_cccd];

/// Cap beside a title: 0.7, 0, 0.7, 0.1.
pub const FORM_CAP: [u32; 4] = [0x3f33_3333, 0, 0x3f33_3333, 0x3dcc_cccd];

/// Wide top cap: 0.7, 0.1, 0.7, 0.2.
pub const WIDE_CAP: [u32; 4] = [0x3f33_3333, 0x3dcc_cccd, 0x3f33_3333, 0x3e4c_cccd];

/// Wide top bar: 0.3, 0.1, 0.3, 0.2.
pub const WIDE_BAR: [u32; 4] = [0x3e99_999a, 0x3dcc_cccd, 0x3e99_999a, 0x3e4c_cccd];

/// Tab after the held bar: 0.5, 0.8, 0.5, 0.9.
pub const HELD_TAB: [u32; 4] = [0x3f00_0000, 0x3f4c_cccd, 0x3f00_0000, 0x3f66_6666];

/// Quad after the lower tab: 0.4, 0.8, 0.4, 1.
pub const HELD_POST: [u32; 4] = [0x3ecc_cccd, 0x3f4c_cccd, 0x3ecc_cccd, 0x3f80_0000];

/// Cap under the held left strip: 0.2, 0.1, 0.2, 0.2.
pub const HELD_CAP: [u32; 4] = [0x3e4c_cccd, 0x3dcc_cccd, 0x3e4c_cccd, 0x3e4c_cccd];

/// Quad used when the edge bits are not both set: 0.15, 0.2, 0.15, 0.6.
pub const HELD_FLIP: [u32; 4] = [0x3e19_999a, 0x3e4c_cccd, 0x3e19_999a, 0x3f19_999a];

/// Right cap when mask bits 1 and 4 are set: 0.65, 0.7, 0.65, 0.9.
pub const HELD_END: [u32; 4] = [0x3f26_6666, 0x3f33_3333, 0x3f26_6666, 0x3f66_6666];

/// Quad past the right edge when those bits are not both set: 0.6, 0.2, 0.6, 0.6.
pub const HELD_OVER: [u32; 4] = [0x3f19_999a, 0x3e4c_cccd, 0x3f19_999a, 0x3f19_999a];

/// Right strip when the tab bit is set: 0.7, 0.5, 0.7, 0.7.
pub const HELD_RIGHT: [u32; 4] = [0x3f33_3333, 0x3f00_0000, 0x3f33_3333, 0x3f33_3333];

/// Middle of a span bar: 0.3, 0.1, 0.3, 0.2, then 0.5, 0.2, 0.5, 0.1.
pub const SPAN_BAR: [u32; 8] = [
    0x3e99_999a,
    0x3dcc_cccd,
    0x3e99_999a,
    0x3e4c_cccd,
    0x3f00_0000,
    0x3e4c_cccd,
    0x3f00_0000,
    0x3dcc_cccd,
];

/// Square on the line class: 0.4, 0.8, 0.4, 1, then 0.5, 1, 0.5, 0.8.
pub const SPAN_SQUARE: [u32; 8] = [
    0x3ecc_cccd,
    0x3f4c_cccd,
    0x3ecc_cccd,
    0x3f80_0000,
    0x3f00_0000,
    0x3f80_0000,
    0x3f00_0000,
    0x3f4c_cccd,
];

/// Thumb of the line rail: 0.5, 0.8, 0.5, 0.9, then 0.55, 0.9, 0.55, 0.8.
pub const LINE_RAIL: [u32; 8] = [
    0x3f00_0000,
    0x3f4c_cccd,
    0x3f00_0000,
    0x3f66_6666,
    0x3f0c_cccd,
    0x3f66_6666,
    0x3f0c_cccd,
    0x3f4c_cccd,
];

/// Chip of the line class: 0.75, 0.8, 0.75, 1, then 0.85, 1, 0.85, 0.8.
pub const LINE_CHIP: [u32; 8] = [
    0x3f40_0000,
    0x3f4c_cccd,
    0x3f40_0000,
    0x3f80_0000,
    0x3f59_999a,
    0x3f80_0000,
    0x3f59_999a,
    0x3f4c_cccd,
];

/// Second pass of the line chip: 0.85, 0.8, 0.85, 1, then 0.95, 1, 0.95, 0.8.
pub const LINE_CHIP_ALT: [u32; 8] = [
    0x3f59_999a,
    0x3f4c_cccd,
    0x3f59_999a,
    0x3f80_0000,
    0x3f73_3333,
    0x3f80_0000,
    0x3f73_3333,
    0x3f4c_cccd,
];

/// End cap of a capped span: 0.15, 0.2, 0.15, 0.4, then 0.3, 0.4, 0.3, 0.2.
pub const SPAN_END: [u32; 8] = [
    0x3e19_999a,
    0x3e4c_cccd,
    0x3e19_999a,
    0x3ecc_cccd,
    0x3e99_999a,
    0x3ecc_cccd,
    0x3e99_999a,
    0x3e4c_cccd,
];

/// Cap of a span bar: 0.7, 0.1, 0.7, 0.2, then 0.75, 0.2, 0.75, 0.1.
pub const SPAN_CAP: [u32; 8] = [
    0x3f33_3333,
    0x3dcc_cccd,
    0x3f33_3333,
    0x3e4c_cccd,
    0x3f40_0000,
    0x3e4c_cccd,
    0x3f40_0000,
    0x3dcc_cccd,
];

/// The double 0.2 used to place the bar after the right strip.
const BEAM_SCALE: u64 = 0x3fc9_9999_9999_999a;

/// The double 6.6 used to place an edge.
const EDGE_SCALE: u64 = 0x401a_6666_6666_6666;

/// A slider widget. `value_bits` are the bits of its 0..1 value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliderPlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub mode: u32,
    pub state: u32,
    pub value_bits: u32,
    /// Half-span the scroll quads add and subtract. The flat path does not use it.
    pub lip: i32,
}

/// Four corners of one slider quad, with the sample bits shared by them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliderQuad {
    pub sample: [u32; 4],
    pub corners: [(i32, i32); 4],
}

/// Quads of a flat slider. A scroll mode leaves these unset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatSlider {
    pub pad: i32,
    pub thumb: i32,
    pub marked: bool,
    pub quads: [SliderQuad; 4],
}

/// A scroll bar: the trough, then twelve quads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollBar {
    pub trough: [i32; 4],
    pub thumb: i32,
    pub thumb_far: i32,
    pub marked: bool,
    pub bar: u8,
    pub quads: [SliderQuad; 12],
}

/// The ink, the flat quads, or the scroll bar. A third mode has neither.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliderDraw {
    pub ink: u8,
    pub flat: Option<FlatSlider>,
    pub scroll: Option<ScrollBar>,
}

fn slider_ink(state: u32) -> u8 {
    if (state & SLIDER_PRESSED) == SLIDER_PRESSED {
        0
    } else if (state & SLIDER_SCROLL) == SLIDER_SCROLL {
        1
    } else {
        2
    }
}

fn quad(sample: [u32; 4], corners: [(i32, i32); 4]) -> SliderQuad {
    SliderQuad { sample, corners }
}

fn right_strip(unit: i32, width: i32, height: i32, slot: i32, down: i32) -> Vec<SliderQuad> {
    let outer = width.wrapping_sub(slot);
    let inner = outer.wrapping_sub(unit.wrapping_shl(1));
    let low = height.wrapping_sub(down);
    vec![quad(
        HELD_RIGHT,
        [(outer, down), (outer, low), (inner, low), (inner, down)],
    )]
}

fn trim_strip(width: i32, height: i32, slot: i32, near: i32, down: i32) -> Vec<SliderQuad> {
    let outer = width.wrapping_sub(slot);
    let inner = width.wrapping_sub(near);
    let top = height.wrapping_sub(down);
    vec![quad(
        WIDE_BAR,
        [(outer, top), (inner, top), (inner, down), (outer, down)],
    )]
}

fn low_strip(height: i32, slot: i32, band: i32, near: i32, down: i32) -> Vec<SliderQuad> {
    let top = height.wrapping_sub(near);
    let low = height.wrapping_sub(down);
    vec![quad(
        HELD_CAP,
        [(slot, top), (band, top), (band, low), (slot, low)],
    )]
}

fn sole_quad(height: i32, slot: i32, band: i32, near: i32, down: i32) -> Vec<SliderQuad> {
    let high = height.wrapping_sub(slot);
    let low = height.wrapping_sub(band);
    vec![quad(
        HELD_CAP,
        [(near, high), (near, low), (down, low), (down, high)],
    )]
}

fn floor_bar(width: i32, height: i32, slot: i32, near: i32, down: i32) -> Vec<SliderQuad> {
    let high = height.wrapping_sub(slot);
    let low = height.wrapping_sub(near);
    let right = width.wrapping_sub(down);
    vec![quad(
        WIDE_BAR,
        [(down, high), (down, low), (right, low), (right, high)],
    )]
}

fn rear_strip(
    width: i32,
    height: i32,
    slot: i32,
    near: i32,
    down: i32,
    step: i32,
) -> Vec<SliderQuad> {
    let outer = width.wrapping_sub(slot);
    let inner = outer.wrapping_sub(step);
    let low = height.wrapping_sub(down);
    let high = height.wrapping_sub(near);
    vec![quad(
        HELD_END,
        [(outer, low), (outer, high), (inner, high), (inner, low)],
    )]
}

fn lip_strip(
    width: i32,
    height: i32,
    slot: i32,
    band: i32,
    near: i32,
    down: i32,
) -> Vec<SliderQuad> {
    let outer = width.wrapping_sub(slot);
    let inner = width.wrapping_sub(band);
    let high = height.wrapping_sub(near);
    let low = height.wrapping_sub(down);
    vec![quad(
        HELD_CAP,
        [(outer, high), (inner, high), (inner, low), (outer, low)],
    )]
}

fn sill_quad(width: i32, height: i32, slot: i32, near: i32, down: i32) -> Vec<SliderQuad> {
    let outer = width.wrapping_sub(near);
    let inner = width.wrapping_sub(down);
    let high = height.wrapping_sub(slot);
    let low = height.wrapping_sub(near);
    vec![quad(
        HELD_CAP,
        [(outer, high), (outer, low), (inner, low), (inner, high)],
    )]
}

fn scroll_bar(unit: i32, place: SliderPlace) -> ScrollBar {
    let x = i32::from(place.x);
    let y = i32::from(place.y);
    let width = i32::from(place.w);
    let height = i32::from(place.h);
    let right = x.wrapping_add(width);
    let bottom = y.wrapping_add(height);
    let inset = i32::from(unit as i16);
    let lip = place.lip;
    let trough = [
        x.wrapping_add(inset).wrapping_add(1),
        y.wrapping_add(inset).wrapping_add(1),
        width.wrapping_sub(inset.wrapping_shl(1)).wrapping_sub(2),
        height.wrapping_sub(inset.wrapping_shl(1)).wrapping_sub(2),
    ];
    let spare = i64::from(width) - i64::from(inset.wrapping_shl(1)) - i64::from(unit);
    let along = (spare as f32) * f32::from_bits(place.value_bits);
    let thumb = x.wrapping_add(along.round_ties_even() as i32);
    let thumb_far = thumb.wrapping_add(unit.wrapping_mul(3));
    let left = x.wrapping_add(lip);
    let inner = right.wrapping_sub(lip);
    let above = y.wrapping_add(unit);
    let below = y.wrapping_sub(unit);
    let low = bottom.wrapping_sub(unit);
    let high = bottom.wrapping_add(unit);
    let near = thumb.wrapping_sub(unit);
    let far = thumb.wrapping_add(unit);
    let near_far = thumb_far.wrapping_sub(unit);
    let far_far = thumb_far.wrapping_add(unit);
    let over = y.wrapping_add(lip);
    let under = bottom.wrapping_sub(lip);
    ScrollBar {
        trough,
        thumb,
        thumb_far,
        marked: (place.state & SLIDER_MARK) == SLIDER_MARK,
        bar: u8::from((place.state & SLIDER_SCROLL) == SLIDER_SCROLL),
        quads: [
            quad(
                SCROLL_EDGE,
                [(left, above), (left, below), (x, below), (x, above)],
            ),
            quad(
                SCROLL_BAR,
                [(left, above), (left, below), (inner, below), (inner, above)],
            ),
            quad(
                SCROLL_EDGE,
                [
                    (inner, above),
                    (inner, below),
                    (right, below),
                    (right, above),
                ],
            ),
            quad(
                SCROLL_EDGE,
                [(left, low), (left, high), (x, high), (x, low)],
            ),
            quad(
                SCROLL_BAR,
                [(left, low), (left, high), (inner, high), (inner, low)],
            ),
            quad(
                SCROLL_EDGE,
                [(inner, low), (inner, high), (right, high), (right, low)],
            ),
            quad(
                SCROLL_BAR,
                [(far, under), (near, under), (near, over), (far, over)],
            ),
            quad(
                SCROLL_EDGE,
                [(far, over), (near, over), (near, y), (far, y)],
            ),
            quad(
                SCROLL_EDGE,
                [(far, under), (near, under), (near, bottom), (far, bottom)],
            ),
            quad(
                SCROLL_BAR,
                [
                    (near_far, under),
                    (far_far, under),
                    (far_far, over),
                    (near_far, over),
                ],
            ),
            quad(
                SCROLL_EDGE,
                [
                    (near_far, over),
                    (far_far, over),
                    (far_far, y),
                    (near_far, y),
                ],
            ),
            quad(
                SCROLL_EDGE,
                [
                    (near_far, under),
                    (far_far, under),
                    (far_far, bottom),
                    (near_far, bottom),
                ],
            ),
        ],
    }
}

/// Draw a flat slider or a scroll bar. Any other high mode selects the ink
/// and draws neither.
pub fn draw_flat_slider(unit: i32, place: SliderPlace) -> SliderDraw {
    let ink = slider_ink(place.state);
    let mode = place.mode & SLIDER_MODE;
    if mode == SCROLL_MODE {
        return SliderDraw {
            ink,
            flat: None,
            scroll: Some(scroll_bar(unit, place)),
        };
    }
    if mode != 0 {
        return SliderDraw {
            ink,
            flat: None,
            scroll: None,
        };
    }
    let x = i32::from(place.x);
    let y = i32::from(place.y);
    let width = i32::from(place.w);
    let right = x + width;
    let pad = unit.wrapping_shl(1);
    let thick = unit.wrapping_shl(2);
    let spare = i64::from(width) - i64::from(thick) - i64::from(pad);
    let along = (spare as f32) * f32::from_bits(place.value_bits);
    let thumb = unit.wrapping_add(along.round_ties_even() as i32);
    let top = y + thick;
    SliderDraw {
        ink,
        scroll: None,
        flat: Some(FlatSlider {
            pad,
            thumb,
            marked: (place.state & SLIDER_MARK) == SLIDER_MARK,
            quads: [
                quad(SLIDER_CAP, [(x, top), (x, y), (x + pad, y), (x + pad, top)]),
                quad(
                    SLIDER_CAP,
                    [
                        (right, top),
                        (right, y),
                        (right - pad, y),
                        (right - pad, top),
                    ],
                ),
                quad(
                    SLIDER_TRACK,
                    [
                        (x + pad, top),
                        (x + pad, y),
                        (right - pad, y),
                        (right - pad, top),
                    ],
                ),
                quad(
                    SLIDER_THUMB,
                    [
                        (x + thumb, top),
                        (x + thumb, y),
                        (x + thick + thumb, y),
                        (x + thick + thumb, top),
                    ],
                ),
            ],
        }),
    }
}

/// A frame rectangle. `mark` is the word that keeps an idle ink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramePlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub mode: u32,
    pub state: u32,
    pub mark: i16,
}

/// Corner squares, edge bars, and the side bars when the frame is tall enough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameDraw {
    pub ink: u8,
    pub quads: Vec<SliderQuad>,
}

/// Draw a frame. Four corners and two edge bars are always present. The two
/// side bars appear only when four times the unit is below the height.
pub fn draw_frame(unit: i32, place: FramePlace) -> FrameDraw {
    let ink = if (place.state & SLIDER_PRESSED) == SLIDER_PRESSED {
        0
    } else if place.mark == 0 && (place.mode & FRAME_WIDE) == 0 {
        2
    } else if (place.state & FRAME_HOT) == 0 {
        0
    } else {
        3
    };
    let x = i32::from(place.x);
    let y = i32::from(place.y);
    let right = x.wrapping_add(i32::from(place.w));
    let bottom = y.wrapping_add(i32::from(place.h));
    let span = unit.wrapping_mul(3);
    let mut quads = vec![
        quad(
            FRAME_CORNER,
            [
                (x, y),
                (x, y.wrapping_add(span)),
                (x.wrapping_add(span), y.wrapping_add(span)),
                (x.wrapping_add(span), y),
            ],
        ),
        quad(
            FRAME_CORNER,
            [
                (x, bottom),
                (x, bottom.wrapping_sub(span)),
                (x.wrapping_add(span), bottom.wrapping_sub(span)),
                (x.wrapping_add(span), bottom),
            ],
        ),
        quad(
            FRAME_CORNER,
            [
                (right, y),
                (right, y.wrapping_add(span)),
                (right.wrapping_sub(span), y.wrapping_add(span)),
                (right.wrapping_sub(span), y),
            ],
        ),
        quad(
            FRAME_CORNER,
            [
                (right, bottom),
                (right, bottom.wrapping_sub(span)),
                (right.wrapping_sub(span), bottom.wrapping_sub(span)),
                (right.wrapping_sub(span), bottom),
            ],
        ),
        quad(
            FRAME_EDGE,
            [
                (x.wrapping_add(span), y),
                (x.wrapping_add(span), y.wrapping_add(unit)),
                (right.wrapping_sub(span), y.wrapping_add(unit)),
                (right.wrapping_sub(span), y),
            ],
        ),
        quad(
            FRAME_EDGE,
            [
                (x.wrapping_add(span), bottom),
                (x.wrapping_add(span), bottom.wrapping_sub(unit)),
                (right.wrapping_sub(span), bottom.wrapping_sub(unit)),
                (right.wrapping_sub(span), bottom),
            ],
        ),
    ];
    if i64::from(unit).wrapping_shl(2) < i64::from(place.h) {
        quads.push(quad(
            FRAME_EDGE,
            [
                (x, bottom.wrapping_sub(span)),
                (x.wrapping_add(unit), bottom.wrapping_sub(span)),
                (x.wrapping_add(unit), y.wrapping_add(span)),
                (x, y.wrapping_add(span)),
            ],
        ));
        quads.push(quad(
            FRAME_EDGE,
            [
                (right, bottom.wrapping_sub(span)),
                (right.wrapping_sub(unit), bottom.wrapping_sub(span)),
                (right.wrapping_sub(unit), y.wrapping_add(span)),
                (right, y.wrapping_add(span)),
            ],
        ));
    }
    FrameDraw { ink, quads }
}

/// A widget that centres an icon or a caption. `own` means the widget carries
/// its own icon list. Otherwise the shared list is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconPlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub mode: u32,
    pub state: u32,
    pub id: u32,
    pub own: bool,
}

/// Where a centred icon is asked to draw. The glyph call itself is not in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CentredIcon {
    pub own: bool,
    pub marked: bool,
    /// The icon-list turn, 0 when the icon is drawn upright.
    pub turn: u8,
    pub ink: u8,
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub scale_bits: u32,
}

fn centre_spare(span: i16, icon: i32) -> i32 {
    let spare = i64::from(span) - i64::from(icon);
    (spare / 2) as i32
}

/// Place a centred icon. `icon` is the stored icon size. A marked state
/// passes the scale bits of 1, and bit 0x40 inside the band 0x60 adds one
/// to the height. Any other state picks an ink and does not pass that scale.
/// The style's top three bits turn the icon.
pub fn draw_centred_icon(icon: i32, place: IconPlace) -> CentredIcon {
    let x = i32::from(place.x).wrapping_add(centre_spare(place.w, icon));
    let y = i32::from(place.y).wrapping_add(centre_spare(place.h, icon));
    let turn = ((place.mode & ICON_TURN) >> 28) as u8;
    if (place.state & 0xf_0000) == ICON_MARK {
        let nudged = if (place.state & ICON_BAND) == ICON_NUDGE {
            y.wrapping_add(1)
        } else {
            y
        };
        return CentredIcon {
            own: place.own,
            marked: true,
            turn,
            ink: 0,
            id: place.id,
            x,
            y: nudged,
            scale_bits: ICON_SCALE,
        };
    }
    let ink = if (place.state & SLIDER_PRESSED) == SLIDER_PRESSED {
        0
    } else if (place.mode & ICON_STYLE) == 0 {
        if (place.state & ICON_NUDGE) == ICON_NUDGE {
            1
        } else {
            2
        }
    } else if (place.state & ICON_NUDGE) == ICON_NUDGE {
        3
    } else {
        4
    };
    CentredIcon {
        own: place.own,
        marked: false,
        turn,
        ink,
        id: place.id,
        x,
        y,
        scale_bits: 0,
    }
}

/// A form the dispatcher opens. The words are the sizes and the scroll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchPlace {
    pub flags_a: u32,
    pub flags_b: u32,
    pub flags_c: u32,
    pub flags_d: u32,
    pub title_width: i32,
    pub title_byte: u8,
    pub baseline: i32,
    pub titled: bool,
    pub close_mark: i32,
    pub width: i32,
    pub height: i32,
    pub at_x: i32,
    pub at_y: i32,
    pub view_x: i32,
    pub view_y: i32,
    pub span_x: i32,
    pub span_y: i32,
    pub limit_x: i32,
    pub limit_y: i32,
    pub along_x: u32,
    pub along_y: u32,
    pub count: i32,
    /// The form's x and y that the hold-bit test compares with the unit.
    pub edge_x: i32,
    pub edge_y: i32,
    /// The two ends whose difference is the room on the right.
    pub room_far_x: i32,
    pub room_near_x: i32,
    /// The two ends whose difference is the room on the bottom.
    pub room_far_y: i32,
    pub room_near_y: i32,
    /// Flags of the owner checked before the held bar.
    pub owner_flags: u32,
    /// The tab's own x and y, before the unit is added.
    pub grip_x: i32,
    pub grip_y: i32,
    /// The y the lower tab hangs from.
    pub seat_y: i32,
    /// The y the quad after the lower tab measures. Its low edge sits one unit below.
    pub post_y: i32,
    /// The close square's x and y.
    pub close_x: i32,
    pub close_y: i32,
}

/// One corner of the backdrop, with its sample bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackdropCorner {
    pub x: i32,
    pub y: i32,
    pub u: u32,
    pub v: u32,
}

/// A centred title. The glyph call is not in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TitleDraw {
    pub x: i32,
    pub baseline: i32,
    pub width: i32,
    pub tint: Option<u8>,
}

/// One close-edge tint: its sample, integer corners and colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseEdge {
    pub sample: [u32; 8],
    pub corners: [(i32, i32); 4],
    pub ink: [u8; 4],
}

/// The opening of a form: spans, an optional shade and backdrop, the scroll
/// shift, the shown children, the title, the corners, the edge bars, the
/// wide top, the left side, the bottom, the right side, the ink copy,
/// the square drawn when the close mark is -4, the hold-bit edge mask,
/// and the bar, the tab, the lower tab, the quad, the left strip, the cap,
/// and the ledge that follow it, and the sign-flipped quad used when the
/// edge bits are not both set, and the right cap drawn when mask bits 1
/// and 4 are set, and the narrow cap drawn when that tab bit is clear,
/// and the quad past the right edge when those bits are not both set,
/// including the same corners when the tab bit is clear, and the right
/// strip drawn when the tab bit is set, and the bar drawn after that
/// strip when mask bit 4 is clear and the plain word's bit 0x200 is
/// clear, and the two caps that finish that bar. A clear tab bit draws
/// a strip on the right, and a lower strip when mask bits 2 and 8 are
/// set, and the ledge that follows it. A clear hold bit on that same
/// band draws the wider frame, including that strip, the wider right
/// strip, the wider lower strip, and the wider ledge, and not that bar.
/// When those bits differ, a quad hangs below the left edge. Both arms
/// then draw a bar along the bottom. Mask bits 4 and 8 together, with
/// the tab bit, draw a strip on the lower right. A clear tab bit draws
/// the narrower strip. Both of those arms then draw a quad along the
/// bottom. A clear hold bit draws the wider bar and, with the tab bit,
/// the wider strip, or the narrower one when that bit is clear, and
/// the wider quad either way. When mask bits 4 and 8 differ, the tab
/// bit draws a quad past that corner, and a clear tab bit draws the
/// same corners. A clear hold bit draws none of either. On that band,
/// a close mark of -4 draws one square, and the band then counts one
/// call with argument 0. Every dispatch then passes the frame to one
/// call, restores the saved register, sets the stack to the frame, and
/// pops the frame. The helper then releases one string slot that
/// starts empty, and an empty slot is not freed. The bytes the jump
/// skips do not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormDispatch {
    pub span_a: i32,
    pub span_b: i32,
    pub span_c: i32,
    pub shadow: bool,
    pub shade: u8,
    pub backdrop: Option<[BackdropCorner; 4]>,
    pub shift_x: i32,
    pub shift_y: i32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub clip: [i32; 4],
    pub shifted: bool,
    pub pane_calls: u32,
    pub shown: Vec<i32>,
    pub title: Option<TitleDraw>,
    pub corners: Vec<SliderQuad>,
    pub edges: Vec<SliderQuad>,
    pub close_edges: Vec<CloseEdge>,
    pub wide_top: Vec<SliderQuad>,
    pub wide_side: Vec<SliderQuad>,
    pub wide_bottom: Vec<SliderQuad>,
    pub wide_right: Vec<SliderQuad>,
    pub wide_calls: u32,
    pub mark_calls: u32,
    pub late_close: Vec<SliderQuad>,
    pub edge_mask: u32,
    pub held_bar: Vec<SliderQuad>,
    pub held_tab: Vec<SliderQuad>,
    pub held_lower: Vec<SliderQuad>,
    pub held_post: Vec<SliderQuad>,
    pub held_left: Vec<SliderQuad>,
    pub held_cap: Vec<SliderQuad>,
    pub held_ledge: Vec<SliderQuad>,
    pub held_flip: Vec<SliderQuad>,
    pub held_end: Vec<SliderQuad>,
    pub held_plain: Vec<SliderQuad>,
    pub held_shelf: Vec<SliderQuad>,
    pub held_over: Vec<SliderQuad>,
    pub held_bare: Vec<SliderQuad>,
    pub held_right: Vec<SliderQuad>,
    pub held_beam: Vec<SliderQuad>,
    pub held_caps: Vec<SliderQuad>,
    pub held_trim: Vec<SliderQuad>,
    pub held_low: Vec<SliderQuad>,
    pub held_sole: Vec<SliderQuad>,
    pub held_skirt: Vec<SliderQuad>,
    pub held_floor: Vec<SliderQuad>,
    pub held_rear: Vec<SliderQuad>,
    pub held_lip: Vec<SliderQuad>,
    pub held_sill: Vec<SliderQuad>,
    pub held_drape: Vec<SliderQuad>,
    pub held_fold: Vec<SliderQuad>,
    pub band_close: Vec<SliderQuad>,
    pub band_calls: u32,
    pub frame_calls: u32,
    pub frame_restored: bool,
    pub frame_text: u64,
    pub frame_release: u32,
    pub frame_freed: bool,
    pub fault_taken: bool,
    /// Mask forced when the hold bit is clear. The held mask stays separate.
    pub open_mask: u32,
    pub open_calls: u32,
    pub open_tab: Vec<SliderQuad>,
    pub open_lower: Vec<SliderQuad>,
    pub open_post: Vec<SliderQuad>,
    pub open_left: Vec<SliderQuad>,
    pub open_cap: Vec<SliderQuad>,
    pub open_ledge: Vec<SliderQuad>,
    pub open_right: Vec<SliderQuad>,
    pub open_trim: Vec<SliderQuad>,
    pub open_low: Vec<SliderQuad>,
    pub open_sole: Vec<SliderQuad>,
    pub open_floor: Vec<SliderQuad>,
    pub open_rear: Vec<SliderQuad>,
    pub open_lip: Vec<SliderQuad>,
    pub open_sill: Vec<SliderQuad>,
}

fn edge_mask(unit: i32, place: &DispatchPlace) -> u32 {
    let wide = (place.flags_b & FORM_BOX) != 0;
    let band = (place.flags_d & FORM_BAND) == FORM_BAND_ON;
    let held = (place.flags_a & FORM_HOLD) != 0;
    if wide || !band || !held {
        return 0;
    }
    let mut mask = 0;
    if place.edge_y < unit {
        mask |= 1;
    }
    if place.edge_x < unit {
        mask |= 2;
    }
    let sum_x = i64::from(unit) + i64::from(place.edge_x) + i64::from(place.width);
    let room_x = i64::from(place.room_far_x) - i64::from(place.room_near_x);
    if sum_x > room_x {
        mask |= 4;
    }
    let sum_y = i64::from(unit) + i64::from(place.edge_y) + i64::from(place.height);
    let room_y = i64::from(place.room_far_y) - i64::from(place.room_near_y);
    if sum_y > room_y {
        mask |= 8;
    }
    mask
}

fn raised_tabs(
    unit: i32,
    place: &DispatchPlace,
) -> (Vec<SliderQuad>, Vec<SliderQuad>, Vec<SliderQuad>) {
    let extra = (f64::from(unit) * f64::from_bits(TAB_SCALE)).round_ties_even() as i32;
    let base = place.grip_x.wrapping_add(extra);
    let x0 = base.wrapping_add(unit);
    let x1 = base.wrapping_add(unit.wrapping_mul(3));
    let lift = unit.wrapping_mul(3);
    let upper = place.grip_y.wrapping_add(lift);
    let high = upper.wrapping_add(unit.wrapping_shl(1));
    let tab = vec![quad(
        HELD_TAB,
        [(x0, high), (x0, upper), (x1, upper), (x1, high)],
    )];
    let lower_at = place.seat_y.wrapping_sub(lift);
    let low = lower_at.wrapping_sub(unit.wrapping_shl(1));
    let lower = vec![quad(
        HELD_TAB,
        [(x0, low), (x0, lower_at), (x1, lower_at), (x1, low)],
    )];
    let post_low = place.post_y.wrapping_sub(unit);
    let post_high = place.post_y.wrapping_add(lift);
    let far = base.wrapping_add(unit.wrapping_shl(2));
    let post = vec![quad(
        HELD_POST,
        [
            (base, post_high),
            (base, post_low),
            (far, post_low),
            (far, post_high),
        ],
    )];
    (tab, lower, post)
}

fn edge_reach(unit: i32) -> i32 {
    (f64::from(unit) * f64::from_bits(EDGE_SCALE)).round_ties_even() as i32
}

fn round_even(span: i64, bits: u32) -> i32 {
    ((span as f32) * f32::from_bits(bits)).round_ties_even() as i32
}

/// Open a form. `shade` is the byte passed with a shade. `shown` is each
/// child's mark. A short list is not read past its end.
pub fn dispatch_form(unit: i32, shade: u8, place: DispatchPlace, shown: &[u8]) -> FormDispatch {
    let span_a = unit.wrapping_shl(1);
    let span_b = unit.wrapping_mul(3);
    let span_c = unit.wrapping_shl(2);
    let shadow = (place.flags_b & FORM_SHADE) == FORM_SHADE;
    let backdrop = if (place.flags_b & FORM_BACK) != 0 && (place.flags_a & FORM_HOLD) == 0 {
        Some([
            BackdropCorner {
                x: 0,
                y: 0,
                u: 0,
                v: 0,
            },
            BackdropCorner {
                x: 0,
                y: place.height,
                u: 0,
                v: FORM_ONE,
            },
            BackdropCorner {
                x: place.width,
                y: place.height,
                u: FORM_ONE,
                v: FORM_ONE,
            },
            BackdropCorner {
                x: place.width,
                y: 0,
                u: FORM_ONE,
                v: 0,
            },
        ])
    } else {
        None
    };
    let (shift_x, shift_y) = if (place.flags_b & FORM_LOCK) == FORM_LOCK {
        (0, 0)
    } else {
        let spare_x = i64::from(place.limit_x) - i64::from(place.span_x) + i64::from(place.view_x);
        let spare_y = i64::from(place.limit_y) - i64::from(place.span_y) + i64::from(place.view_y);
        (
            round_even(spare_x, place.along_x),
            round_even(spare_y, place.along_y),
        )
    };
    let origin_x = place.view_x.wrapping_sub(shift_x);
    let origin_y = place.view_y.wrapping_sub(shift_y);
    let mut visits = Vec::new();
    let mut index = 0;
    while index < place.count {
        if shown.get(index as usize).is_some_and(|byte| *byte != 0) {
            visits.push(index);
        }
        index += 1;
    }
    let measured = if (place.flags_b & FORM_TITLE) == FORM_TITLE {
        place.title_width
    } else {
        0
    };
    let title = if measured > 0 {
        let spare = i64::from(place.width) - i64::from(measured);
        Some(TitleDraw {
            x: (spare / 2) as i32,
            baseline: place.baseline,
            width: place.width,
            tint: if place.titled {
                Some(round_even(i64::from(place.title_byte), FORM_ONE) as u8)
            } else {
                None
            },
        })
    } else {
        None
    };
    let styled = (place.flags_d & FORM_STYLE) == FORM_STYLE_ON;
    let wide = (place.flags_b & FORM_BOX) != 0;
    let (corners, edges, wide_calls) = if styled && wide {
        (Vec::new(), Vec::new(), 1)
    } else if styled {
        let thick = unit.wrapping_mul(6);
        let bar = unit.wrapping_shl(2);
        let width = place.width;
        let height = place.height;
        let top = if (place.flags_b & FORM_CLOSE) == FORM_CLOSE {
            FORM_ALT
        } else {
            FORM_CORNER
        };
        let mut quads = vec![quad(
            FORM_CORNER,
            [(0, 0), (0, thick), (thick, thick), (thick, 0)],
        )];
        if (place.flags_b & FORM_CLOSE) == FORM_CLOSE && place.close_mark != -4 {
            quads.push(quad(
                FORM_CLOSE_INK,
                [
                    (width, 0),
                    (width.wrapping_sub(bar), 0),
                    (width.wrapping_sub(bar), bar),
                    (width, bar),
                ],
            ));
        }
        quads.push(quad(
            top,
            [
                (width, 0),
                (width.wrapping_sub(thick), 0),
                (width.wrapping_sub(thick), thick),
                (width, thick),
            ],
        ));
        quads.push(quad(
            FORM_CORNER,
            [
                (0, height),
                (thick, height),
                (thick, height.wrapping_sub(thick)),
                (0, height.wrapping_sub(thick)),
            ],
        ));
        quads.push(quad(
            FORM_CORNER,
            [
                (width, height),
                (width, height.wrapping_sub(thick)),
                (width.wrapping_sub(thick), height.wrapping_sub(thick)),
                (width.wrapping_sub(thick), height),
            ],
        ));
        let band = unit.wrapping_shl(1);
        let reach = edge_reach(unit);
        let mut bars = Vec::new();
        if measured > 0 {
            let spare = i64::from(width) - i64::from(measured) - i64::from(unit.wrapping_mul(5));
            let gap = (spare / 2) as i32;
            bars.push(quad(
                FORM_EDGE,
                [(reach, 0), (reach, band), (gap, band), (gap, 0)],
            ));
            bars.push(quad(
                FORM_EDGE,
                [
                    (width.wrapping_sub(gap), 0),
                    (width.wrapping_sub(gap), band),
                    (width.wrapping_sub(reach), band),
                    (width.wrapping_sub(reach), 0),
                ],
            ));
            bars.push(quad(
                FORM_CAP,
                [
                    (gap, 0),
                    (gap, band),
                    (gap.wrapping_add(band), band),
                    (gap.wrapping_add(band), 0),
                ],
            ));
            bars.push(quad(
                FORM_CAP,
                [
                    (width.wrapping_sub(gap), 0),
                    (width.wrapping_sub(gap), band),
                    (width.wrapping_sub(gap).wrapping_sub(band), band),
                    (width.wrapping_sub(gap).wrapping_sub(band), 0),
                ],
            ));
        } else {
            bars.push(quad(
                FORM_EDGE,
                [
                    (reach, 0),
                    (reach, band),
                    (width.wrapping_sub(reach), band),
                    (width.wrapping_sub(reach), 0),
                ],
            ));
        }
        bars.push(quad(
            FORM_EDGE,
            [
                (0, height.wrapping_sub(reach)),
                (band, height.wrapping_sub(reach)),
                (band, reach),
                (0, reach),
            ],
        ));
        bars.push(quad(
            FORM_EDGE,
            [
                (reach, height),
                (reach, height.wrapping_sub(band)),
                (width.wrapping_sub(reach), height.wrapping_sub(band)),
                (width.wrapping_sub(reach), height),
            ],
        ));
        bars.push(quad(
            FORM_EDGE,
            [
                (width, height.wrapping_sub(reach)),
                (width.wrapping_sub(band), height.wrapping_sub(reach)),
                (width.wrapping_sub(band), reach),
                (width, reach),
            ],
        ));
        (quads, bars, 0)
    } else {
        (Vec::new(), Vec::new(), 0)
    };
    let (wide_top, wide_side, wide_bottom, wide_right) =
        if wide && (place.flags_d & FORM_BAND) == FORM_BAND_ON {
            let band = unit.wrapping_shl(1);
            let width = place.width;
            let height = place.height;
            let down = unit;
            let up = unit.wrapping_neg();
            let top = vec![
                quad(WIDE_CAP, [(band, down), (band, up), (0, up), (0, down)]),
                quad(
                    WIDE_BAR,
                    [
                        (band, down),
                        (band, up),
                        (width.wrapping_sub(band), up),
                        (width.wrapping_sub(band), down),
                    ],
                ),
                quad(
                    WIDE_CAP,
                    [
                        (width.wrapping_sub(band), down),
                        (width.wrapping_sub(band), up),
                        (width, up),
                        (width, down),
                    ],
                ),
            ];
            let side = vec![
                quad(
                    WIDE_BAR,
                    [
                        (down, height.wrapping_sub(band)),
                        (up, height.wrapping_sub(band)),
                        (up, band),
                        (down, band),
                    ],
                ),
                quad(WIDE_CAP, [(down, band), (up, band), (up, 0), (down, 0)]),
                quad(
                    WIDE_CAP,
                    [
                        (down, height.wrapping_sub(band)),
                        (up, height.wrapping_sub(band)),
                        (up, height),
                        (down, height),
                    ],
                ),
            ];
            let low = height.wrapping_sub(unit);
            let high = height.wrapping_add(unit);
            let bottom = vec![
                quad(WIDE_CAP, [(band, low), (band, high), (0, high), (0, low)]),
                quad(
                    WIDE_BAR,
                    [
                        (band, low),
                        (band, high),
                        (width.wrapping_sub(band), high),
                        (width.wrapping_sub(band), low),
                    ],
                ),
                quad(
                    WIDE_CAP,
                    [
                        (width.wrapping_sub(band), low),
                        (width.wrapping_sub(band), high),
                        (width, high),
                        (width, low),
                    ],
                ),
            ];
            let near = width.wrapping_sub(unit);
            let far = width.wrapping_add(unit);
            let right = vec![
                quad(
                    WIDE_BAR,
                    [
                        (near, height.wrapping_sub(band)),
                        (far, height.wrapping_sub(band)),
                        (far, band),
                        (near, band),
                    ],
                ),
                quad(WIDE_CAP, [(near, band), (far, band), (far, 0), (near, 0)]),
                quad(
                    WIDE_CAP,
                    [
                        (near, height.wrapping_sub(band)),
                        (far, height.wrapping_sub(band)),
                        (far, height),
                        (near, height),
                    ],
                ),
            ];
            (top, side, bottom, right)
        } else {
            (Vec::new(), Vec::new(), Vec::new(), Vec::new())
        };
    let mark_calls = u32::from(
        wide && (place.flags_d & FORM_BAND) == FORM_BAND_ON
            && (place.flags_a & FORM_MARK) == FORM_MARK,
    );
    let late_close = if place.close_mark == -4 {
        let span = unit.wrapping_shl(2);
        let width = place.width;
        vec![quad(
            FORM_CLOSE_INK,
            [
                (width, 0),
                (width.wrapping_sub(span), 0),
                (width.wrapping_sub(span), span),
                (width, span),
            ],
        )]
    } else {
        Vec::new()
    };
    let edge_mask = edge_mask(unit, &place);
    let close_edges = if (place.flags_c & FORM_STILL) != 0 {
        let x0 = i64::from(place.close_x)
            + (f64::from(unit) * f64::from_bits(CLOSE_SLANT)).round_ties_even() as i64;
        let lift = if (edge_mask & 5) == 5 {
            unit.wrapping_mul(3)
        } else {
            unit.wrapping_shl(1)
        };
        let y0 = i64::from(place.close_y) + i64::from(lift);
        let u = i64::from(unit);
        let corners = match place.close_mark {
            -9 => Some([
                (x0 + u, y0 + 2 * u),
                (x0 + u, y0),
                (x0 + 3 * u, y0),
                (x0 + 3 * u, y0 + 2 * u),
            ]),
            -10 => Some([
                (x0 + u, y0 - 2 * u),
                (x0 + u, y0),
                (x0 + 3 * u, y0),
                (x0 + 3 * u, y0 - 2 * u),
            ]),
            -8 => Some([
                (x0, y0 + 4 * u),
                (x0, y0),
                (x0 + 4 * u, y0),
                (x0 + 4 * u, y0 + 4 * u),
            ]),
            _ => None,
        };
        corners
            .map(|corners| {
                vec![CloseEdge {
                    sample: LINE_RAIL,
                    corners: std::array::from_fn(|i| {
                        let (x, y) = corners[i];
                        (x as i32, y as i32)
                    }),
                    ink: [0xff, 0xcc, 0x99, 0xff],
                }]
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let reach = unit.wrapping_shl(2).wrapping_add(unit.wrapping_shl(1));
    let band = unit.wrapping_shl(1);
    let skip_bar = (edge_mask & 1) != 0
        && (place.owner_flags & 1) != 0
        && (place.owner_flags & 0x80_0000) == 0;
    let on_hold = (place.flags_b & FORM_BOX) == 0
        && (place.flags_d & FORM_BAND) == FORM_BAND_ON
        && (place.flags_a & FORM_HOLD) != 0;
    let held_bar = if on_hold && !skip_bar {
        let width = place.width;
        vec![quad(
            WIDE_BAR,
            [
                (reach, 0),
                (reach, band),
                (width.wrapping_sub(reach), band),
                (width.wrapping_sub(reach), 0),
            ],
        )]
    } else {
        Vec::new()
    };
    let held_tab = if on_hold && (place.flags_b & FORM_TAB) == FORM_TAB {
        let extra = (f64::from(unit) * f64::from_bits(TAB_SCALE)).round_ties_even() as i32;
        let base = place.grip_x.wrapping_add(extra);
        let lift = if (edge_mask & 0x5) == 0x5 {
            unit.wrapping_mul(3)
        } else {
            unit.wrapping_shl(1)
        };
        let anchor = place.grip_y.wrapping_add(lift);
        let x0 = base.wrapping_add(unit);
        let x1 = base.wrapping_add(unit.wrapping_mul(3));
        let y1 = anchor.wrapping_add(unit.wrapping_shl(1));
        vec![quad(
            HELD_TAB,
            [(x0, y1), (x0, anchor), (x1, anchor), (x1, y1)],
        )]
    } else {
        Vec::new()
    };
    let held_lower = if on_hold && (place.flags_b & FORM_TAB) == FORM_TAB {
        let extra = (f64::from(unit) * f64::from_bits(TAB_SCALE)).round_ties_even() as i32;
        let base = place.grip_x.wrapping_add(extra);
        let drop = if (edge_mask & 0xc) == 0xc {
            unit.wrapping_mul(3)
        } else {
            unit.wrapping_shl(1)
        };
        let anchor = place.seat_y.wrapping_sub(drop);
        let x0 = base.wrapping_add(unit);
        let x1 = base.wrapping_add(unit.wrapping_mul(3));
        let y0 = anchor.wrapping_sub(unit.wrapping_shl(1));
        vec![quad(
            HELD_TAB,
            [(x0, y0), (x0, anchor), (x1, anchor), (x1, y0)],
        )]
    } else {
        Vec::new()
    };
    let held_post = if on_hold && (place.flags_b & FORM_TAB) == FORM_TAB {
        let extra = (f64::from(unit) * f64::from_bits(TAB_SCALE)).round_ties_even() as i32;
        let base = place.grip_x.wrapping_add(extra);
        let low = place.post_y.wrapping_sub(unit);
        let high = place.post_y.wrapping_add(unit.wrapping_mul(3));
        let far = base.wrapping_add(unit.wrapping_shl(2));
        vec![quad(
            HELD_POST,
            [(base, high), (base, low), (far, low), (far, high)],
        )]
    } else {
        Vec::new()
    };
    let held_left = if on_hold {
        let reach = unit.wrapping_shl(2).wrapping_add(unit.wrapping_shl(1));
        let band = unit.wrapping_shl(1);
        let top = place.height.wrapping_sub(reach);
        vec![quad(
            WIDE_BAR,
            [(0, top), (band, top), (band, reach), (0, reach)],
        )]
    } else {
        Vec::new()
    };
    let held_cap = if on_hold && (edge_mask & 0x3) == 0x3 {
        let near = unit.wrapping_shl(1);
        let far = unit.wrapping_shl(2).wrapping_add(near);
        vec![quad(
            HELD_CAP,
            [(0, near), (near, near), (near, far), (0, far)],
        )]
    } else {
        Vec::new()
    };
    let skip_ledge = (place.owner_flags & 0x80_0000) == 0
        && ((place.flags_b & FORM_TITLE) != 0 || (place.owner_flags & 1) != 0);
    let held_ledge = if on_hold && (edge_mask & 0x3) == 0x3 && !skip_ledge {
        let near = unit.wrapping_shl(1);
        let far = unit.wrapping_shl(2).wrapping_add(near);
        vec![quad(
            HELD_CAP,
            [(near, 0), (near, near), (far, near), (far, 0)],
        )]
    } else {
        Vec::new()
    };
    let held_flip = if on_hold && (edge_mask & 0x3) != 0x3 {
        let near = unit.wrapping_shl(1);
        let far = unit.wrapping_shl(2).wrapping_add(near);
        let back = near.wrapping_neg();
        vec![quad(
            HELD_FLIP,
            [(back, back), (back, far), (far, far), (far, back)],
        )]
    } else {
        Vec::new()
    };
    let held_end = if on_hold && (edge_mask & 0x5) == 0x5 && (place.flags_b & FORM_TAB) == FORM_TAB
    {
        let near = unit.wrapping_shl(1);
        let far = unit.wrapping_shl(2).wrapping_add(near);
        let width = place.width;
        let inner = width.wrapping_sub(far);
        vec![quad(
            HELD_END,
            [(width, far), (width, near), (inner, near), (inner, far)],
        )]
    } else {
        Vec::new()
    };
    let held_plain =
        if on_hold && (edge_mask & 0x5) == 0x5 && (place.flags_b & FORM_TAB) != FORM_TAB {
            let near = unit.wrapping_shl(1);
            let far = unit.wrapping_shl(2).wrapping_add(near);
            let width = place.width;
            let inner = width.wrapping_sub(near);
            vec![quad(
                HELD_CAP,
                [(width, near), (inner, near), (inner, far), (width, far)],
            )]
        } else {
            Vec::new()
        };
    let held_shelf = if on_hold
        && (edge_mask & 0x5) == 0x5
        && (place.flags_b & FORM_TAB) != FORM_TAB
        && !skip_ledge
    {
        let near = unit.wrapping_shl(1);
        let far = unit.wrapping_shl(2).wrapping_add(near);
        let width = place.width;
        let outer = width.wrapping_sub(near);
        let inner = width.wrapping_sub(far);
        vec![quad(
            HELD_CAP,
            [(outer, 0), (outer, near), (inner, near), (inner, 0)],
        )]
    } else {
        Vec::new()
    };
    let held_over = if on_hold && (edge_mask & 0x5) != 0x5 && (place.flags_b & FORM_TAB) == FORM_TAB
    {
        let near = unit.wrapping_shl(1);
        let far = unit.wrapping_shl(2).wrapping_add(near);
        let width = place.width;
        let outer = width.wrapping_add(near);
        let inner = width.wrapping_sub(far);
        let back = near.wrapping_neg();
        vec![quad(
            HELD_OVER,
            [(outer, back), (outer, far), (inner, far), (inner, back)],
        )]
    } else {
        Vec::new()
    };
    let held_bare = if on_hold && (edge_mask & 0x5) != 0x5 && (place.flags_b & FORM_TAB) != FORM_TAB
    {
        let near = unit.wrapping_shl(1);
        let far = unit.wrapping_shl(2).wrapping_add(near);
        let width = place.width;
        let outer = width.wrapping_add(near);
        let inner = width.wrapping_sub(far);
        let back = near.wrapping_neg();
        vec![quad(
            HELD_FLIP,
            [(outer, back), (outer, far), (inner, far), (inner, back)],
        )]
    } else {
        Vec::new()
    };
    let held_right = if on_hold && (place.flags_b & FORM_TAB) == FORM_TAB {
        let slot = unit.wrapping_shl(1);
        let down = unit.wrapping_shl(2).wrapping_add(slot);
        right_strip(unit, place.width, place.height, slot, down)
    } else {
        Vec::new()
    };
    let on_open = (place.flags_b & FORM_BOX) == 0
        && (place.flags_d & FORM_BAND) == FORM_BAND_ON
        && (place.flags_a & FORM_HOLD) == 0;
    let open_mask = if on_open { 0xf } else { 0 };
    let open_calls = u32::from(on_open && (place.flags_b & FORM_TITLE) == 0);
    let (open_tab, open_lower, open_post) = if on_open && (place.flags_b & FORM_TAB) == FORM_TAB {
        raised_tabs(unit, &place)
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    };
    let (open_left, open_cap, open_ledge) = if on_open {
        let across = unit.wrapping_shl(2);
        let down = across.wrapping_shl(1);
        let slot = unit.wrapping_shl(1);
        let top = place.height.wrapping_sub(down);
        let left = vec![quad(
            WIDE_BAR,
            [(slot, top), (across, top), (across, down), (slot, down)],
        )];
        let cap = vec![quad(
            HELD_CAP,
            [
                (slot, across),
                (across, across),
                (across, down),
                (slot, down),
            ],
        )];
        let ledge = if skip_ledge {
            Vec::new()
        } else {
            vec![quad(
                HELD_CAP,
                [
                    (across, slot),
                    (across, across),
                    (down, across),
                    (down, slot),
                ],
            )]
        };
        (left, cap, ledge)
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    };
    let open_right = if on_open && (place.flags_b & FORM_TAB) == FORM_TAB {
        let slot = unit.wrapping_shl(2);
        let down = slot.wrapping_shl(1);
        right_strip(unit, place.width, place.height, slot, down)
    } else {
        Vec::new()
    };
    let on_band = (place.flags_b & FORM_BOX) == 0 && (place.flags_d & FORM_BAND) == FORM_BAND_ON;
    let live_mask = if on_hold {
        edge_mask
    } else if on_open {
        0xf
    } else {
        0
    };
    let draw_side = on_band
        && (place.flags_b & FORM_TAB) == FORM_TAB
        && (live_mask & 4) == 0
        && (place.flags_c & FORM_STILL) == 0;
    let (held_beam, held_caps) = if draw_side {
        let extra = (f64::from(unit) * f64::from_bits(BEAM_SCALE)).round_ties_even() as i32;
        let base = place.width.wrapping_add(extra);
        let down = unit.wrapping_shl(2);
        let left = base.wrapping_sub(unit);
        let right = base.wrapping_add(unit);
        let low = place.height.wrapping_sub(down);
        let slot = 0;
        let foot = place.height.wrapping_sub(slot);
        (
            vec![quad(
                WIDE_BAR,
                [(right, low), (left, low), (left, down), (right, down)],
            )],
            vec![
                quad(
                    WIDE_CAP,
                    [(right, down), (left, down), (left, slot), (right, slot)],
                ),
                quad(
                    WIDE_CAP,
                    [(right, low), (left, low), (left, foot), (right, foot)],
                ),
            ],
        )
    } else {
        (Vec::new(), Vec::new())
    };
    let held_trim = if on_hold && (place.flags_b & FORM_TAB) != FORM_TAB {
        let near = unit.wrapping_shl(1);
        let down = unit.wrapping_shl(2).wrapping_add(near);
        trim_strip(place.width, place.height, 0, near, down)
    } else {
        Vec::new()
    };
    let open_trim = if on_open && (place.flags_b & FORM_TAB) != FORM_TAB {
        let slot = unit.wrapping_shl(1);
        let near = unit.wrapping_shl(2);
        let down = near.wrapping_shl(1);
        trim_strip(place.width, place.height, slot, near, down)
    } else {
        Vec::new()
    };
    let held_low = if on_hold && (edge_mask & 0xa) == 0xa {
        let band = unit.wrapping_shl(1);
        let down = unit.wrapping_shl(2).wrapping_add(band);
        low_strip(place.height, 0, band, band, down)
    } else {
        Vec::new()
    };
    let open_low = if on_open {
        let slot = unit.wrapping_shl(1);
        let band = unit.wrapping_shl(2);
        let down = band.wrapping_shl(1);
        low_strip(place.height, slot, band, band, down)
    } else {
        Vec::new()
    };
    let held_sole = if on_hold && (edge_mask & 0xa) == 0xa {
        let band = unit.wrapping_shl(1);
        let down = unit.wrapping_shl(2).wrapping_add(band);
        sole_quad(place.height, 0, band, band, down)
    } else {
        Vec::new()
    };
    let open_sole = if on_open {
        let slot = unit.wrapping_shl(1);
        let band = unit.wrapping_shl(2);
        let down = band.wrapping_shl(1);
        sole_quad(place.height, slot, band, band, down)
    } else {
        Vec::new()
    };
    let held_skirt = if on_hold && (edge_mask & 0xa) != 0xa {
        let near = unit.wrapping_shl(1);
        let down = unit.wrapping_shl(2).wrapping_add(near);
        let back = near.wrapping_neg();
        let low = place.height.wrapping_sub(down);
        let high = place.height.wrapping_add(near);
        vec![quad(
            HELD_FLIP,
            [(back, high), (back, low), (down, low), (down, high)],
        )]
    } else {
        Vec::new()
    };
    let held_floor = if on_hold {
        let near = unit.wrapping_shl(1);
        let down = unit.wrapping_shl(2).wrapping_add(near);
        floor_bar(place.width, place.height, 0, near, down)
    } else {
        Vec::new()
    };
    let open_floor = if on_open {
        let slot = unit.wrapping_shl(1);
        let near = unit.wrapping_shl(2);
        let down = near.wrapping_shl(1);
        floor_bar(place.width, place.height, slot, near, down)
    } else {
        Vec::new()
    };
    let held_rear = if on_hold && (edge_mask & 0xc) == 0xc && (place.flags_b & FORM_TAB) == FORM_TAB
    {
        let near = unit.wrapping_shl(1);
        let down = unit.wrapping_shl(2).wrapping_add(near);
        let step = unit.wrapping_shl(2);
        rear_strip(place.width, place.height, 0, near, down, step)
    } else {
        Vec::new()
    };
    let open_rear = if on_open && (place.flags_b & FORM_TAB) == FORM_TAB {
        let slot = unit.wrapping_shl(1);
        let near = unit.wrapping_shl(2);
        let down = near.wrapping_shl(1);
        let step = unit.wrapping_shl(2);
        rear_strip(place.width, place.height, slot, near, down, step)
    } else {
        Vec::new()
    };
    let held_lip = if on_hold && (edge_mask & 0xc) == 0xc && (place.flags_b & FORM_TAB) != FORM_TAB
    {
        let band = unit.wrapping_shl(1);
        let down = unit.wrapping_shl(2).wrapping_add(band);
        lip_strip(place.width, place.height, 0, band, band, down)
    } else {
        Vec::new()
    };
    let open_lip = if on_open && (place.flags_b & FORM_TAB) != FORM_TAB {
        let slot = unit.wrapping_shl(1);
        let band = unit.wrapping_shl(2);
        let down = band.wrapping_shl(1);
        lip_strip(place.width, place.height, slot, band, band, down)
    } else {
        Vec::new()
    };
    let held_sill = if on_hold && (edge_mask & 0xc) == 0xc {
        let near = unit.wrapping_shl(1);
        let down = unit.wrapping_shl(2).wrapping_add(near);
        sill_quad(place.width, place.height, 0, near, down)
    } else {
        Vec::new()
    };
    let open_sill = if on_open {
        let slot = unit.wrapping_shl(1);
        let near = unit.wrapping_shl(2);
        let down = near.wrapping_shl(1);
        sill_quad(place.width, place.height, slot, near, down)
    } else {
        Vec::new()
    };
    let held_drape =
        if on_hold && (edge_mask & 0xc) != 0xc && (place.flags_b & FORM_TAB) == FORM_TAB {
            let near = unit.wrapping_shl(1);
            let down = unit.wrapping_shl(2).wrapping_add(near);
            let right = place.width.wrapping_add(near);
            let left = place.width.wrapping_sub(down);
            let high = place.height.wrapping_add(near);
            let low = place.height.wrapping_sub(down);
            vec![quad(
                HELD_OVER,
                [(right, high), (right, low), (left, low), (left, high)],
            )]
        } else {
            Vec::new()
        };
    let held_fold = if on_hold && (edge_mask & 0xc) != 0xc && (place.flags_b & FORM_TAB) != FORM_TAB
    {
        let near = unit.wrapping_shl(1);
        let down = unit.wrapping_shl(2).wrapping_add(near);
        let right = place.width.wrapping_add(near);
        let left = place.width.wrapping_sub(down);
        let high = place.height.wrapping_add(near);
        let low = place.height.wrapping_sub(down);
        vec![quad(
            HELD_FLIP,
            [(right, high), (right, low), (left, low), (left, high)],
        )]
    } else {
        Vec::new()
    };
    let on_band = on_hold || on_open;
    let band_calls = u32::from(on_band);
    let frame_calls = 1;
    let frame_restored = true;
    let frame_text = 0;
    let frame_release = 1;
    let frame_freed = false;
    let fault_taken = false;
    let band_close = if on_band && place.close_mark == -4 {
        let span = unit.wrapping_shl(2);
        let width = place.width;
        vec![quad(
            FORM_CLOSE_INK,
            [
                (width, 0),
                (width.wrapping_sub(span), 0),
                (width.wrapping_sub(span), span),
                (width, span),
            ],
        )]
    } else {
        Vec::new()
    };
    FormDispatch {
        span_a,
        span_b,
        span_c,
        shadow,
        shade: if shadow { shade } else { 0 },
        backdrop,
        shift_x,
        shift_y,
        origin_x,
        origin_y,
        clip: [
            place.at_x.wrapping_add(place.view_x),
            place.at_y.wrapping_add(place.view_y),
            place.span_x.wrapping_sub(place.view_x),
            place.span_y.wrapping_sub(place.view_y),
        ],
        shifted: (place.flags_c & FORM_PLAIN) == 0,
        pane_calls: 1,
        shown: visits,
        title,
        corners,
        edges,
        close_edges,
        wide_top,
        wide_side,
        wide_bottom,
        wide_right,
        wide_calls,
        mark_calls,
        late_close,
        edge_mask,
        held_bar,
        held_tab,
        held_lower,
        held_post,
        held_left,
        held_cap,
        held_ledge,
        held_flip,
        held_end,
        held_plain,
        held_shelf,
        held_over,
        held_bare,
        held_right,
        held_beam,
        held_caps,
        held_trim,
        held_low,
        held_sole,
        held_skirt,
        held_floor,
        held_rear,
        held_lip,
        held_sill,
        held_drape,
        held_fold,
        band_close,
        band_calls,
        frame_calls,
        frame_restored,
        frame_text,
        frame_release,
        frame_freed,
        fault_taken,
        open_mask,
        open_calls,
        open_tab,
        open_lower,
        open_post,
        open_left,
        open_cap,
        open_ledge,
        open_right,
        open_trim,
        open_low,
        open_sole,
        open_floor,
        open_rear,
        open_lip,
        open_sill,
    }
}

/// Shifts, scales, and the colour word applied to each span-bar corner.
/// An unset view is all zeros: the image does not store the shifts, the scales,
/// the flag, or the mark, and the colour word starts at 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpanView {
    pub shift_x: f32,
    pub shift_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub colour: u32,
    pub flag: u32,
    pub mark: u32,
    pub cursor: u32,
}

/// The image start. Shifts, scales, the flag and the mark read as 0, and the colour word is 0.
pub const SPAN_VIEW_UNSET: SpanView = SpanView {
    shift_x: 0.0,
    shift_y: 0.0,
    scale_x: 0.0,
    scale_y: 0.0,
    colour: 0,
    flag: 0,
    mark: 0,
    cursor: 0,
};

/// Three quads of a span bar. The other line-class methods are not in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpanBar {
    pub prep_calls: u32,
    pub binds: [[u32; 8]; 3],
    pub quads: [[(f32, f32); 4]; 3],
    pub placed: [[(f32, f32); 4]; 3],
    pub colour: u32,
    pub colour_writes: u32,
    pub marks: [u32; 12],
    pub mark_indexes: [u32; 12],
    pub mark_count: u32,
    pub cursor_after: u32,
}

fn place_corner(x: f32, y: f32, view: SpanView) -> (f32, f32) {
    (
        (x + view.shift_x) * view.scale_x,
        (view.shift_y - y) * view.scale_y,
    )
}

/// Draws the span bar. `band_word` is the stored count, doubled for the thickness.
pub fn draw_span_bar(
    unit: i32,
    origin_x: f32,
    origin_y: f32,
    span: f32,
    band_word: i32,
    view: SpanView,
) -> SpanBar {
    let unit_f = unit as f32;
    let thick = (i64::from(band_word) << 1) as f32;
    let top = origin_y;
    let bottom = origin_y + thick;
    let inset_l = origin_x + unit_f;
    let inset_r = origin_x + span - unit_f;
    let right = origin_x + span;
    let quads = [
        [
            (inset_l, top),
            (inset_l, bottom),
            (inset_r, bottom),
            (inset_r, top),
        ],
        [
            (inset_l, top),
            (inset_l, bottom),
            (origin_x, bottom),
            (origin_x, top),
        ],
        [
            (inset_r, top),
            (inset_r, bottom),
            (right, bottom),
            (right, top),
        ],
    ];
    let placed = quads.map(|quad| quad.map(|(x, y)| place_corner(x, y, view)));
    let marked = (view.flag & 4) > 0;
    let mut cursor = view.cursor;
    let mut marks = [0; 12];
    let mut mark_indexes = [0; 12];
    for slot in 0..12 {
        if marked {
            marks[slot] = view.mark;
            mark_indexes[slot] = cursor;
        }
        cursor = cursor.wrapping_add(1);
    }
    SpanBar {
        prep_calls: 1,
        binds: [SPAN_BAR, SPAN_CAP, SPAN_CAP],
        quads,
        placed,
        colour: view.colour,
        colour_writes: 12,
        marks,
        mark_indexes,
        mark_count: if marked { 12 } else { 0 },
        cursor_after: cursor,
    }
}

/// A square on the line class: one quad, four colour writes and its marks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpanSquare {
    pub mode_calls: u32,
    pub mode: i32,
    pub prep_calls: u32,
    pub begin_calls: u32,
    pub bind_calls: u32,
    pub bind: [u32; 8],
    pub quads: [(f32, f32); 4],
    pub colour: u32,
    pub colour_writes: u32,
    pub end_calls: u32,
    pub marks: [u32; 4],
    pub mark_indexes: [u32; 4],
    pub mark_count: u32,
    pub cursor_after: u32,
}

/// Draws the square after the span bar. `word` is the stored count; its
/// left shift gives the side, and the inset pulls the origin back. The
/// begin clears the cursor, so the four marks start at 0.
pub fn draw_span_square(origin_x: f32, origin_y: f32, word: i32, view: SpanView) -> SpanSquare {
    let side = (((word as u32) << 2) as i32) as i64;
    let inset = (((side - 2) as u64) >> 1) as i64;
    let back = inset as f32;
    let x = origin_x - back;
    let y = origin_y - back;
    let side = side as f32;
    let quads = [(x, y + side), (x, y), (x + side, y), (x + side, y + side)];
    let marked = (view.flag & 4) > 0;
    let mut marks = [0; 4];
    let mut mark_indexes = [0; 4];
    for slot in 0..4 {
        if marked {
            marks[slot] = view.mark;
            mark_indexes[slot] = slot as u32;
        }
    }
    SpanSquare {
        mode_calls: 1,
        mode: 2,
        prep_calls: 1,
        begin_calls: 1,
        bind_calls: 1,
        bind: SPAN_SQUARE,
        quads,
        colour: view.colour,
        colour_writes: 4,
        end_calls: 1,
        marks,
        mark_indexes,
        mark_count: if marked { 4 } else { 0 },
        cursor_after: 4,
    }
}

/// A four-unit chip on the line class: one quad, or two when bit 0x40 is set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineChip {
    pub mode_calls: u32,
    pub mode: i32,
    pub begin_calls: u32,
    pub prep_calls: u32,
    pub bind_calls: u32,
    pub samples: [[u32; 8]; 2],
    pub quads: [[(f32, f32); 4]; 2],
    pub draw_calls: u32,
    pub ink_index: u32,
    pub colour: [u8; 4],
    pub colour_writes: u32,
    pub end_calls: u32,
}

/// Draws the chip. Bit 0x10 selects the first ink, mode 0x10000 the second
/// with alpha 255, and any other word the third. Bit 0x40 repeats the quad
/// with the second sample.
pub fn draw_line_chip(unit: i32, x: i16, y: i16, flag: u32, inks: [[u8; 4]; 3]) -> LineChip {
    let side = unit.wrapping_shl(2) as f32;
    let x = f32::from(x);
    let y = f32::from(y);
    let corners = [(x, y + side), (x, y), (x + side, y), (x + side, y + side)];
    let (ink_index, mut colour) = if (flag & 0x10) > 0 {
        (0, inks[0])
    } else if (flag & 0xf_0000) == 0x1_0000 {
        (1, inks[1])
    } else {
        (2, inks[2])
    };
    if ink_index == 1 {
        colour[3] = 0xff;
    }
    let second = (flag & 0x40) > 0;
    LineChip {
        mode_calls: 1,
        mode: 2,
        begin_calls: 1,
        prep_calls: 1,
        bind_calls: 1 + u32::from(second),
        samples: [LINE_CHIP, if second { LINE_CHIP_ALT } else { [0; 8] }],
        quads: [corners, if second { corners } else { [(0.0, 0.0); 4] }],
        draw_calls: 1 + u32::from(second),
        ink_index,
        colour,
        colour_writes: 4 * (1 + u32::from(second)),
        end_calls: 1,
    }
}

/// One arm of a bevel: two fill inks, two border inks and one outline ink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BevelArm {
    pub fill: [[u8; 4]; 2],
    pub border: [[u8; 4]; 2],
    pub outline: [u8; 4],
}

/// The raised, pressed and idle arms of a bevel box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BevelInk {
    pub raised: BevelArm,
    pub pressed: BevelArm,
    pub idle: BevelArm,
}

/// The shipped line-class inks: only the pressed arm shades, black at alpha 0xE0.
pub const BEVEL_INK: BevelInk = BevelInk {
    raised: BevelArm {
        fill: [[0, 0, 0, 0], [0, 0, 0, 0]],
        border: [[0, 0, 0, 0], [0, 0, 0, 0]],
        outline: [0, 0, 0, 0],
    },
    pressed: BevelArm {
        fill: [[0, 0, 0, 0xe0], [0, 0, 0, 0xe0]],
        border: [[0, 0, 0, 0xe0], [0, 0, 0, 0xe0]],
        outline: [0, 0, 0, 0xe0],
    },
    idle: BevelArm {
        fill: [[0, 0, 0, 0], [0, 0, 0, 0]],
        border: [[0, 0, 0, 0], [0, 0, 0, 0]],
        outline: [0, 0, 0, 0],
    },
};

/// A bevel place. Bit 0x40 of the state selects raised, else bit 0x10 pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BevelPlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub state: u32,
}

/// Four corners with their inks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InkQuad {
    pub corners: [(f32, f32); 4],
    pub inks: [[u8; 4]; 4],
}

/// A segment with its end inks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InkLine {
    pub ends: [(f32, f32); 2],
    pub inks: [[u8; 4]; 2],
}

/// Two fill quads, six border lines and four outline lines.
#[derive(Debug, Clone, PartialEq)]
pub struct BevelBox {
    pub fill: [InkQuad; 2],
    pub border: [InkLine; 6],
    pub outline: [InkLine; 4],
    pub begins: u32,
    pub binds: u32,
    pub ends: u32,
}

fn mix_ink(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let mut out = [0; 4];
    for (slot, (from, to)) in out.iter_mut().zip(a.iter().zip(b.iter())) {
        let value = f32::from(*from) + (f32::from(*to) - f32::from(*from)) * t;
        *slot = value.round_ties_even().clamp(0.0, 255.0) as u8;
    }
    out
}

/// Draws the bevel box. The fill insets by two pixels, the border and the
/// outline by one, then each line moves in by half a pixel.
pub fn draw_bevel(place: BevelPlace, ink: &BevelInk) -> BevelBox {
    let arm = if (place.state & 0x40) > 0 {
        &ink.raised
    } else if (place.state & 0x10) > 0 {
        &ink.pressed
    } else {
        &ink.idle
    };
    let x = i64::from(place.x);
    let y = i64::from(place.y);
    let w = i64::from(place.w);
    let h = i64::from(place.h);

    let fx0 = (x + 2) as f32;
    let fy0 = (y + 2) as f32;
    let fx1 = (x + w - 2) as f32;
    let fy1 = (y + h - 2) as f32;
    let fmid = (fy0 - fy1) * 0.5 + fy1;
    let fill_a = arm.fill[0];
    let fill_b = arm.fill[1];
    let m60 = mix_ink(fill_a, fill_b, 0.6);
    let m40 = mix_ink(fill_a, fill_b, 0.4);
    let fill = [
        InkQuad {
            corners: [(fx1, fy0), (fx0, fy0), (fx0, fmid), (fx1, fmid)],
            inks: [fill_b, fill_b, m60, m60],
        },
        InkQuad {
            corners: [(fx1, fmid), (fx0, fmid), (fx0, fy1), (fx1, fy1)],
            inks: [fill_a, fill_a, m40, m40],
        },
    ];

    let bx0 = (x + 1) as f32 + 0.5;
    let by0 = (y + 1) as f32 + 0.5;
    let bx1 = (x + w - 1) as f32 - 0.5;
    let by1 = (y + h - 1) as f32 - 0.5;
    let bmid = (by0 - by1) * 0.5 + by1;
    let border_a = arm.border[0];
    let border_b = arm.border[1];
    let m65 = mix_ink(border_a, border_b, 0.65);
    let m35 = mix_ink(border_a, border_b, 0.35);
    let border = [
        InkLine {
            ends: [(bx1, by0), (bx0, by0)],
            inks: [border_b, border_b],
        },
        InkLine {
            ends: [(bx0, by0), (bx0, bmid + 0.5)],
            inks: [border_b, m65],
        },
        InkLine {
            ends: [(bx0, bmid + 0.5), (bx0, by1)],
            inks: [m35, border_a],
        },
        InkLine {
            ends: [(bx0, by1), (bx1, by1)],
            inks: [border_a, border_a],
        },
        InkLine {
            ends: [(bx1, by1), (bx1, bmid)],
            inks: [border_a, m35],
        },
        InkLine {
            ends: [(bx1, bmid), (bx1, by0)],
            inks: [m65, border_b],
        },
    ];

    let ox0 = x as f32 + 0.5;
    let oy0 = y as f32 + 0.5;
    let ox1 = (x + w) as f32 - 0.5;
    let oy1 = (y + h) as f32 - 0.5;
    let outline_ink = arm.outline;
    let outline = [
        InkLine {
            ends: [(ox1, oy0), (ox0, oy0)],
            inks: [outline_ink, outline_ink],
        },
        InkLine {
            ends: [(ox0, oy0), (ox0, oy1)],
            inks: [outline_ink, outline_ink],
        },
        InkLine {
            ends: [(ox0, oy1), (ox1, oy1)],
            inks: [outline_ink, outline_ink],
        },
        InkLine {
            ends: [(ox1, oy1), (ox1, oy0)],
            inks: [outline_ink, outline_ink],
        },
    ];

    BevelBox {
        fill,
        border,
        outline,
        begins: 3,
        binds: 12,
        ends: 3,
    }
}

/// The parent bevel's two boolean arms: the first wins, else the second, else idle.
pub fn draw_skin_bevel(place: BevelPlace, raised: bool, pressed: bool, ink: &BevelInk) -> BevelBox {
    let state = if raised {
        0x40
    } else if pressed {
        0x10
    } else {
        0
    };
    draw_bevel(BevelPlace { state, ..place }, ink)
}

/// Two end caps and the middle bar between their inner edges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CappedSpan {
    pub mode_calls: u32,
    pub mode: i32,
    pub begin_calls: u32,
    pub prep_calls: u32,
    pub ink_binds: u32,
    pub bind_calls: u32,
    /// Two cap binds, then the middle bind.
    pub binds: [[u32; 8]; 3],
    pub quads: [[(f32, f32); 4]; 3],
    pub placed: [[(f32, f32); 4]; 3],
    pub colour: u32,
    pub colour_writes: u32,
    pub end_calls: u32,
}

/// Draws the capped span. The retail method's second positional float is
/// dead, so it is not taken. `centre` is the vertical middle.
pub fn draw_capped_span(
    unit: i32,
    left: f32,
    right: f32,
    centre: f32,
    ink: u32,
    view: SpanView,
) -> CappedSpan {
    let doubled = unit.wrapping_shl(1) as f32;
    let quadrupled = unit.wrapping_shl(2) as f32;
    let back = -doubled;
    let top = centre + back;
    let bottom = quadrupled + top;
    let left_out = left + back;
    let left_in = left + quadrupled;
    let right_out = right - back;
    let right_in = right - quadrupled;
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
            (left_in, centre),
            (left_in, bottom),
            (right_in, bottom),
            (right_in, centre),
        ],
    ];
    let placed = quads.map(|quad| quad.map(|(x, y)| place_corner(x, y, view)));
    CappedSpan {
        mode_calls: 1,
        mode: 2,
        begin_calls: 1,
        prep_calls: 1,
        ink_binds: 1,
        bind_calls: 3,
        binds: [SPAN_END, SPAN_END, SPAN_BAR],
        quads,
        placed,
        colour: ink,
        colour_writes: 12,
        end_calls: 1,
    }
}

/// A label place. The style bits 0x7000 align horizontally and 0x700 vertically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelPlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub inset: i16,
    pub style: u32,
    pub state: u32,
}

/// The measured text and the font's line height and base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelMetrics {
    pub width: i32,
    pub line_height: i32,
    pub base: i32,
}

/// Where the label lands, its ink, and the renderer it calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineLabel {
    pub x: i32,
    pub y: i32,
    pub ink_index: Option<u32>,
    pub colour: Option<[u8; 4]>,
    pub coloured: bool,
    pub text_calls: u32,
}

/// Places the label. The style word aligns it, and the state word chooses
/// the ink and the one-pixel nudge.
/// Aligns a measured label from the style word.
fn align_label(place: LabelPlace, metrics: LabelMetrics, unit: i32) -> (i32, i32) {
    let px = i32::from(place.x);
    let py = i32::from(place.y);
    let pw = i32::from(place.w);
    let ph = i32::from(place.h);
    let inset = i32::from(place.inset);
    let x = match place.style & 0x7000 {
        0x1000 => px - metrics.width - unit,
        0x2000 => px + (pw - 1 - metrics.width) / 2,
        0x3000 => px + pw - metrics.width - unit,
        0x4000 => px + pw + unit,
        _ => px + inset + (unit + 1) / 2,
    };
    let y = match place.style & 0x700 {
        0x100 => py - unit / 2,
        0x200 => py + (ph + metrics.line_height + 2) / 2,
        0x300 => py + ph + metrics.base - 1,
        0x400 => py + ph,
        _ => py + (unit + 1) / 2,
    };
    (x, y)
}

pub fn place_line_label(
    place: LabelPlace,
    metrics: LabelMetrics,
    unit: i32,
    inks: [[u8; 4]; 3],
) -> LineLabel {
    let (x, mut y) = align_label(place, metrics, unit);
    let (ink_index, coloured, nudge) = if (place.state & 0x10) > 0 {
        (Some(0), false, 0)
    } else if (place.state & 0xf_0000) == 0x1_0000 {
        (None, true, i32::from((place.state & 0x40) > 0))
    } else if (place.state & 0x60) > 0 {
        (Some(2), false, 1)
    } else {
        (Some(1), false, 0)
    };
    y += nudge;
    LineLabel {
        x,
        y,
        ink_index,
        colour: ink_index.map(|index| inks[index as usize]),
        coloured,
        text_calls: 1,
    }
}

/// A parent-skin label: one call, or two for the shadow arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinLabel {
    pub x: i32,
    pub y: i32,
    pub calls: Vec<([u8; 4], (i32, i32))>,
    pub coloured: bool,
    pub text_calls: u32,
}

/// Places the parent skin's label. Bit 0x40 draws the two-call drop shadow.
pub fn place_skin_label(
    place: LabelPlace,
    metrics: LabelMetrics,
    unit: i32,
    inks: [[u8; 4]; 4],
) -> SkinLabel {
    let (x, y) = align_label(place, metrics, unit);
    if (place.state & 0x10) > 0 {
        SkinLabel {
            x,
            y,
            calls: vec![(inks[0], (x, y))],
            coloured: false,
            text_calls: 1,
        }
    } else if (place.state & 0xf_0000) == 0x1_0000 {
        let nudge = i32::from((place.state & 0x40) > 0);
        SkinLabel {
            x,
            y: y + nudge,
            calls: Vec::new(),
            coloured: true,
            text_calls: 1,
        }
    } else if (place.state & 0x40) > 0 {
        SkinLabel {
            x,
            y,
            calls: vec![(inks[1], (x - 1, y)), (inks[2], (x, y + 1))],
            coloured: false,
            text_calls: 2,
        }
    } else {
        SkinLabel {
            x,
            y,
            calls: vec![(inks[3], (x, y))],
            coloured: false,
            text_calls: 1,
        }
    }
}

/// A rail place: a track at the bottom and a thumb at the right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RailPlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub state: u32,
}

/// Three track quads and the thumb.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineRail {
    pub mode_calls: u32,
    pub mode: i32,
    pub begin_calls: u32,
    pub prep_calls: u32,
    pub bind_calls: u32,
    pub binds: [[u32; 8]; 4],
    pub quads: [[(f32, f32); 4]; 4],
    pub placed: [[(f32, f32); 4]; 4],
    pub track_ink: [u8; 4],
    pub thumb_ink: [u8; 4],
    pub colour_writes: u32,
    pub end_calls: u32,
}

/// Draws the rail. Bit 0x10 selects the first ink; mode 0x10000 gives the
/// thumb the third ink with alpha 255.
pub fn draw_line_rail(unit: i32, place: RailPlace, inks: [[u8; 4]; 3], view: SpanView) -> LineRail {
    let x = i64::from(place.x);
    let y = i64::from(place.y);
    let w = i64::from(place.w);
    let h = i64::from(place.h);
    let doubled = i64::from(unit.wrapping_shl(1));
    let quadrupled = i64::from(unit.wrapping_shl(2));
    let triple = i64::from(unit.wrapping_mul(3));
    let bottom = y + h - triple;
    let right = x + w;
    let left_out = x - doubled;
    let left_in = x + quadrupled;
    let right_out = right - doubled;
    let right_in = right - quadrupled;
    let top_thumb = y + doubled;
    let bottom_thumb = y + quadrupled;
    let thumb_right = right - i64::from(unit);
    let thumb_left = right - triple;
    let quads = [
        [
            (left_out as f32, bottom as f32),
            (left_out as f32, (bottom + quadrupled) as f32),
            (left_in as f32, (bottom + quadrupled) as f32),
            (left_in as f32, bottom as f32),
        ],
        [
            (right_out as f32, bottom as f32),
            (right_out as f32, (bottom + quadrupled) as f32),
            (right_in as f32, (bottom + quadrupled) as f32),
            (right_in as f32, bottom as f32),
        ],
        [
            (left_in as f32, (bottom + doubled) as f32),
            (left_in as f32, (bottom + quadrupled) as f32),
            (right_in as f32, (bottom + quadrupled) as f32),
            (right_in as f32, (bottom + doubled) as f32),
        ],
        [
            (thumb_right as f32, top_thumb as f32),
            (thumb_right as f32, bottom_thumb as f32),
            (thumb_left as f32, bottom_thumb as f32),
            (thumb_left as f32, top_thumb as f32),
        ],
    ];
    let placed = quads.map(|quad| quad.map(|(x, y)| place_corner(x, y, view)));
    let track_ink = if (place.state & 0x10) > 0 {
        inks[0]
    } else {
        inks[1]
    };
    let thumb_ink = if (place.state & 0xf_0000) == 0x1_0000 {
        let mut ink = inks[2];
        ink[3] = 0xff;
        ink
    } else {
        track_ink
    };
    LineRail {
        mode_calls: 1,
        mode: 2,
        begin_calls: 1,
        prep_calls: 1,
        bind_calls: 4,
        binds: [SPAN_END, SPAN_END, SPAN_BAR, LINE_RAIL],
        quads,
        placed,
        track_ink,
        thumb_ink,
        colour_writes: 16,
        end_calls: 1,
    }
}

/// The three row heights the line class scales: 80, 104 and 128.
pub const LINE_HEIGHTS: [f32; 3] = [80.0, 104.0, 128.0];

/// The scaled row heights and the parent's scale call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineScale {
    pub scale_calls: u32,
    pub heights: [u8; 3],
}
/// Scales the three heights. Each product rounds to nearest even and keeps
/// the low byte; an out-of-range product stores 0.
/// Rounds to nearest even and keeps the low byte; an out-of-range product is 0.
fn low_byte(value: f32) -> u8 {
    let value = value.round_ties_even();
    if value >= 2_147_483_648.0 || value < -2_147_483_648.0 || value.is_nan() {
        0
    } else {
        value as i32 as u8
    }
}

pub fn scale_line(scale_y: f32) -> LineScale {
    let mut heights = [0u8; 3];
    for (slot, base) in heights.iter_mut().zip(LINE_HEIGHTS) {
        *slot = low_byte(base * scale_y);
    }
    LineScale {
        scale_calls: 1,
        heights,
    }
}

/// The two ramp-alpha groups: fourteen entries, then forty-eight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinAlpha {
    pub first: [u8; 14],
    pub second: [u8; 48],
}

/// Scales the two stored alpha bytes into their ramp entries.
pub fn scale_skin_alpha(first: u8, second: u8, scale_x: f32, scale_y: f32) -> SkinAlpha {
    SkinAlpha {
        first: [low_byte(f32::from(first) * scale_x); 14],
        second: [low_byte(f32::from(second) * scale_y); 48],
    }
}

/// A place for the rail hit test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitPlace {
    pub flags: u32,
    pub control: u32,
    pub right: i32,
    pub left: i32,
    pub top: i32,
    pub bottom: i32,
}

/// Returns the original's hit code: 0, -3, -4, -8, -9 or -10. The meaning
/// of each code is not established.
pub fn hit_line(place: HitPlace, x: i32, y: i32, unit: i32, wide: bool) -> i32 {
    let span = if wide {
        unit.wrapping_mul(6)
    } else {
        unit.wrapping_shl(2)
    };
    if (place.flags & 0x30) == 0x10 && span >= y {
        if (place.control & 2) != 0 && place.right.wrapping_sub(span) <= x {
            return -4;
        }
        if (place.control & 1) != 0 {
            return -3;
        }
    } else if (place.control & 0x200) != 0 && x > place.left {
        if y > place.top {
            if y < place.bottom {
                return -8;
            }
            if place.bottom.wrapping_add(span) > y {
                return -10;
            }
        } else if place.top.wrapping_sub(span) < y {
            return -9;
        }
    }
    0
}

/// A line-shade place. The state band 0x10000 selects the special ink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineShadePlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub state: u32,
    pub inner_ink: [u8; 4],
    pub side_inset: i32,
}

/// The normal and special outer inks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineShadeInks {
    pub normal: [u8; 4],
    pub special: [u8; 4],
}

/// One shade quad. The inner rectangle binds no sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineShadeQuad {
    pub sample: Option<[u32; 8]>,
    pub corners: [(f32, f32); 4],
    pub placed: [(f32, f32); 4],
    pub ink: [u8; 4],
}

/// The inner rectangle's integer extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineShadeRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub right: i64,
    pub bottom: i64,
}

/// Six outer quads and the solid inner rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct LineShade {
    pub modes: [i32; 3],
    pub mode_calls: u32,
    pub prepare_calls: u32,
    pub outer_ink_bind_calls: u32,
    pub inner_ink_bind_calls: u32,
    pub sample_bind_calls: u32,
    pub begin_calls: u32,
    pub end_calls: u32,
    pub vertex_calls: u32,
    pub colour_writes: u32,
    pub batch_count: u32,
    pub outer: [LineShadeQuad; 6],
    pub inner: LineShadeQuad,
    pub inner_rect: LineShadeRect,
    pub outer_ink: [u8; 4],
    pub inner_ink: [u8; 4],
    pub special_ink_used: bool,
    pub place_inset_after: i16,
}

/// Draws the shade: six outer quads over a span, then a solid inner
/// rectangle inset by the unit.
pub fn draw_line_shade(
    unit: i32,
    place: LineShadePlace,
    inks: LineShadeInks,
    view: SpanView,
) -> LineShade {
    let x = i32::from(place.x);
    let y = i32::from(place.y);
    let w = i32::from(place.w);
    let h = i32::from(place.h);
    let half = ((unit as u32) >> 1) as i32;
    let right = x.wrapping_add(w);
    let top = y.wrapping_sub(half);
    let bottom = y.wrapping_add(h).wrapping_add(half);
    let side = place.side_inset;
    let left_inset = (i64::from(x) + i64::from(side)) as f32;
    let right_inset = (i64::from(right) - i64::from(side)) as f32;
    let left = x as f32;
    let right_f = right as f32;
    let top_high = (i64::from(top) + i64::from(unit)) as f32;
    let top_low = (i64::from(top) - i64::from(unit)) as f32;
    let bottom_low = (i64::from(bottom) - i64::from(unit)) as f32;
    let bottom_high = (i64::from(bottom) + i64::from(unit)) as f32;
    let raw: [[(f32, f32); 4]; 6] = [
        [
            (left_inset, top_high),
            (left_inset, top_low),
            (left, top_low),
            (left, top_high),
        ],
        [
            (left_inset, top_high),
            (left_inset, top_low),
            (right_inset, top_low),
            (right_inset, top_high),
        ],
        [
            (right_inset, top_high),
            (right_inset, top_low),
            (right_f, top_low),
            (right_f, top_high),
        ],
        [
            (left_inset, bottom_low),
            (left_inset, bottom_high),
            (left, bottom_high),
            (left, bottom_low),
        ],
        [
            (left_inset, bottom_low),
            (left_inset, bottom_high),
            (right_inset, bottom_high),
            (right_inset, bottom_low),
        ],
        [
            (right_inset, bottom_low),
            (right_inset, bottom_high),
            (right_f, bottom_high),
            (right_f, bottom_low),
        ],
    ];
    let binds = [SPAN_CAP, SPAN_BAR, SPAN_CAP, SPAN_CAP, SPAN_BAR, SPAN_CAP];
    let special = (place.state & 0xf_0000) == 0x1_0000;
    let outer_ink = if special {
        let mut ink = inks.special;
        ink[3] = 0xff;
        ink
    } else {
        inks.normal
    };
    let mut inner_ink = place.inner_ink;
    inner_ink[3] = 0xff;
    let inset = unit as i16;
    let inner_left = x.wrapping_add(i32::from(inset));
    let inner_top = y.wrapping_add(i32::from(inset));
    let inner_width = w.wrapping_sub(i32::from(inset).wrapping_shl(1));
    let inner_height = h.wrapping_sub(i32::from(inset).wrapping_shl(1));
    let inner_right = i64::from(inner_left) + i64::from(inner_width);
    let inner_bottom = i64::from(inner_top) + i64::from(inner_height);
    let inner_corners = [
        (inner_right as f32, inner_top as f32),
        (inner_left as f32, inner_top as f32),
        (inner_left as f32, inner_bottom as f32),
        (inner_right as f32, inner_bottom as f32),
    ];
    let mut outer = [LineShadeQuad {
        sample: None,
        corners: [(0.0, 0.0); 4],
        placed: [(0.0, 0.0); 4],
        ink: outer_ink,
    }; 6];
    for (slot, quad) in outer.iter_mut().enumerate() {
        *quad = LineShadeQuad {
            sample: Some(binds[slot]),
            corners: raw[slot],
            placed: raw[slot].map(|(px, py)| place_corner(px, py, view)),
            ink: outer_ink,
        };
    }
    LineShade {
        modes: [2, 1, 0x10],
        mode_calls: 3,
        prepare_calls: 1,
        outer_ink_bind_calls: 1,
        inner_ink_bind_calls: 1,
        sample_bind_calls: 6,
        begin_calls: 2,
        end_calls: 2,
        vertex_calls: 28,
        colour_writes: 28,
        batch_count: 2,
        outer,
        inner: LineShadeQuad {
            sample: None,
            corners: inner_corners,
            placed: inner_corners.map(|(px, py)| place_corner(px, py, view)),
            ink: inner_ink,
        },
        inner_rect: LineShadeRect {
            left: inner_left,
            top: inner_top,
            width: inner_width,
            height: inner_height,
            right: inner_right,
            bottom: inner_bottom,
        },
        outer_ink,
        inner_ink,
        special_ink_used: special,
        place_inset_after: inset,
    }
}

/// The two calls the line opening wrapper makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineOpen {
    pub parent_calls: u32,
    pub texture_calls: u32,
}

/// Opens the line: the parent's open, then the texture load.
pub fn open_line() -> LineOpen {
    LineOpen {
        parent_calls: 1,
        texture_calls: 1,
    }
}

/// The skin renderer's guarded preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinPrepared {
    pub calls: u32,
    pub prepared: bool,
}

/// Prepares the renderer. The guard skips the two setup calls when it is set.
pub fn prepare_skin_renderer(already: bool) -> SkinPrepared {
    SkinPrepared {
        calls: if already { 0 } else { 2 },
        prepared: true,
    }
}

/// A parent-skin shade place: one inset rectangle in the place colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShadePlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub inset: i16,
    pub ink: [u8; 4],
}

/// One inset rectangle and the calls around it.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinShade {
    pub prepare_calls: u32,
    pub modes: [i32; 2],
    pub mode_calls: u32,
    pub colour: [u8; 4],
    pub corners: [(f32, f32); 4],
    pub rect: LineShadeRect,
    pub vertex_calls: u32,
    pub colour_writes: u32,
    pub begin_calls: u32,
    pub end_calls: u32,
}

/// Draws the parent skin's inner shade.
pub fn draw_skin_shade(place: ShadePlace) -> SkinShade {
    let x = i32::from(place.x);
    let y = i32::from(place.y);
    let w = i32::from(place.w);
    let h = i32::from(place.h);
    let inset = i32::from(place.inset);
    let left = x.wrapping_add(inset);
    let top = y.wrapping_add(inset);
    let width = w.wrapping_sub(inset.wrapping_shl(1));
    let height = h.wrapping_sub(inset.wrapping_shl(1));
    let right = i64::from(left) + i64::from(width);
    let bottom = i64::from(top) + i64::from(height);
    let corners = [
        (right as f32, top as f32),
        (left as f32, top as f32),
        (left as f32, bottom as f32),
        (right as f32, bottom as f32),
    ];
    let mut colour = place.ink;
    colour[3] = 0xff;
    SkinShade {
        prepare_calls: 1,
        modes: [1, 0x10],
        mode_calls: 2,
        colour,
        corners,
        rect: LineShadeRect {
            left,
            top,
            width,
            height,
            right,
            bottom,
        },
        vertex_calls: 4,
        colour_writes: 4,
        begin_calls: 1,
        end_calls: 1,
    }
}

/// The parent skin's cross: eight vertices as four line segments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinCross {
    pub corners: [(f32, f32); 8],
    pub begin_calls: u32,
    pub end_calls: u32,
    pub vertex_calls: u32,
}

/// Draws the cross at a point.
pub fn draw_skin_cross(unit: i32, x: i32, y: i32) -> SkinCross {
    let half = unit.wrapping_add(1) / 2 + 1;
    let x = i64::from(x);
    let y = i64::from(y);
    let u = i64::from(unit);
    let h = i64::from(half);
    let corners = [
        ((x - h + 2) as f32, (y - u) as f32),
        ((x + h - 1) as f32, (y - u) as f32),
        ((x - h + 2) as f32, (y + u) as f32),
        ((x + h - 1) as f32, (y + u) as f32),
        ((x - u + 1) as f32, (y - h + 2) as f32),
        ((x - u + 1) as f32, (y + h - 1) as f32),
        ((x + u + 1) as f32, (y - h + 2) as f32),
        ((x + u + 1) as f32, (y + h - 1) as f32),
    ];
    SkinCross {
        corners,
        begin_calls: 1,
        end_calls: 1,
        vertex_calls: 8,
    }
}

/// The four metrics the unit and trim derive for control rectangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinMetrics {
    pub unit: i32,
    pub trim: i32,
}

impl SkinMetrics {
    /// Fifteen: the rail and close squares' side.
    pub const RAIL: i32 = 15;

    /// Seventeen: the close band's width.
    pub const CLOSE: i32 = 17;
}

/// The form fields the rectangle switch reads: the width and height, the
/// client inset, the client's top and right, and the form's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlPlace {
    pub x: i32,
    pub y: i32,
    pub size: i32,
    pub field_a: i32,
    pub field_b: i32,
    pub flags: u32,
}

fn close_band(place: ControlPlace, metrics: SkinMetrics, extra: i32) -> [i32; 4] {
    let left = place
        .x
        .wrapping_sub(SkinMetrics::CLOSE)
        .wrapping_sub(metrics.unit)
        .wrapping_sub(extra);
    let top = metrics.unit.wrapping_sub(1);
    let right = left.wrapping_add(SkinMetrics::CLOSE);
    let bottom = (24 - metrics.unit).wrapping_add(3);
    [left, top, right, bottom]
}

fn rail_square(left: i32, top: i32, rail: i32) -> [i32; 4] {
    [left, top, left.wrapping_add(rail), top.wrapping_add(rail)]
}

/// Places the rectangle a control code names. An unknown code returns nothing.
pub fn control_rect(place: ControlPlace, code: i32, metrics: SkinMetrics) -> Option<[i32; 4]> {
    let rail = SkinMetrics::RAIL;
    match code {
        -4 => Some(close_band(place, metrics, 0)),
        -6 => {
            let extra = if (place.flags & 2) != 0 { 3 } else { 0 };
            Some(close_band(place, metrics, extra))
        }
        -5 => {
            let extra = match place.flags & 0xa {
                0xa => 6,
                0 => 0,
                _ => 3,
            };
            Some(close_band(place, metrics, extra))
        }
        -9 => {
            let left = place.x.wrapping_sub(place.size).wrapping_sub(rail);
            Some(rail_square(left, place.field_a, rail))
        }
        -10 => {
            let left = place.x.wrapping_sub(place.size).wrapping_sub(rail);
            let top = place.y.wrapping_sub(place.size).wrapping_sub(rail);
            Some(rail_square(left, top, rail))
        }
        -13 => {
            let top = place.y.wrapping_sub(place.size).wrapping_sub(rail);
            Some(rail_square(place.size, top, rail))
        }
        -14 => {
            let top = place.y.wrapping_sub(place.size).wrapping_sub(rail);
            let left = place.field_b.wrapping_sub(rail);
            Some(rail_square(left, top, rail))
        }
        _ => None,
    }
}

/// The three texture slots the frames read, kept by their byte offsets.
pub const SKIN_FRAME_A: u32 = 0x9c;
pub const SKIN_FRAME_B: u32 = 0xb4;
pub const SKIN_FRAME_C: u32 = 0xb8;

/// A place for the two frame switches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinFramePlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub style: u32,
}

/// One textured frame: its rectangle and the two texture slots.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinFrameDraw {
    pub rect: [f32; 4],
    pub first: u32,
    pub second: u32,
}

/// Selects the parent skin's two frames from the style word.
pub fn skin_frames(place: SkinFramePlace) -> Vec<SkinFrameDraw> {
    let x = i64::from(place.x);
    let y = i64::from(place.y);
    let w = i64::from(place.w);
    let h = i64::from(place.h);
    let outer = [(x) as f32, (y) as f32, (x + w) as f32, (y + h) as f32];
    let inner = [
        (x + 1) as f32,
        (y + 1) as f32,
        (x + w - 1) as f32,
        (y + h - 1) as f32,
    ];
    let mut frames = Vec::new();
    match place.style & 0x30 {
        0x10 => frames.push(SkinFrameDraw {
            rect: outer,
            first: SKIN_FRAME_A,
            second: SKIN_FRAME_C,
        }),
        0x20 => frames.push(SkinFrameDraw {
            rect: outer,
            first: SKIN_FRAME_B,
            second: SKIN_FRAME_A,
        }),
        _ => {}
    }
    match place.style & 0xc0 {
        0x40 => frames.push(SkinFrameDraw {
            rect: inner,
            first: SKIN_FRAME_A,
            second: SKIN_FRAME_C,
        }),
        0x80 => frames.push(SkinFrameDraw {
            rect: inner,
            first: SKIN_FRAME_B,
            second: SKIN_FRAME_A,
        }),
        _ => {}
    }
    frames
}

/// The image draw's inputs: the place, the image word and its side, and the two words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinImagePlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub image: i32,
    pub size: i32,
    pub style: u32,
    pub flags: u32,
    pub default_image: bool,
}

/// The three colour bytes the ink binds read, plus the shared alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinImageInks {
    pub plain: [u8; 3],
    pub raised: [u8; 3],
    pub alpha: u8,
}

/// What the image draw selects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinImageDraw {
    pub x: i64,
    pub y: i64,
    pub image: i32,
    pub default_image: bool,
    pub frame: Option<u32>,
    pub scale: Option<f32>,
    pub ink: Option<u32>,
    pub colour: Option<[u8; 4]>,
}

/// Centres a control's image and selects its ink and frame.
pub fn draw_skin_image(place: SkinImagePlace, inks: SkinImageInks) -> SkinImageDraw {
    let size = i64::from(place.size);
    let half_x = (i64::from(place.w) - size) / 2;
    let half_y = (i64::from(place.h) - size) / 2;
    let nudge = i64::from((place.flags & 0x60) == 0x40);
    let x = i64::from(place.x) + half_x;
    let y = i64::from(place.y) + half_y + nudge;
    let frame = if (place.style & 0x7000_0000) > 0 {
        Some((place.style >> 28) & 7)
    } else {
        None
    };

    let mode_10000 = (place.flags & 0xf_0000) == 0x1_0000;
    let (ink, colour, scale) = if mode_10000 {
        (None, None, Some(1.0))
    } else if (place.style & 0x100_0000) != 0 {
        if (place.flags & 0x10) != 0 {
            (Some(0xd8), None, None)
        } else if (place.flags & 0x40) == 0x40 {
            (
                Some(0xf4),
                Some([inks.raised[0], inks.raised[1], inks.raised[2], inks.alpha]),
                None,
            )
        } else {
            (
                Some(0xe4),
                Some([inks.plain[0], inks.plain[1], inks.plain[2], inks.alpha]),
                None,
            )
        }
    } else if (place.flags & 0x11) != 0 {
        (Some(0xd8), None, None)
    } else if (place.flags & 0x40) == 0x40 {
        (Some(0xf4), None, None)
    } else {
        (Some(0xe4), None, None)
    };

    SkinImageDraw {
        x,
        y,
        image: place.image,
        default_image: place.default_image,
        frame,
        scale,
        ink,
        colour,
    }
}

/// The fields the parent hit test reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinHitPlace {
    pub x0: i32,
    pub y0: i32,
    pub size: i32,
    pub limit_right: i32,
    pub limit_top: i32,
    pub limit_bottom: i32,
    pub word_5c: u32,
    pub flags: u32,
}

/// Hits the parent skin. `rect_for` supplies each code's rectangle.
pub fn hit_skin(
    place: SkinHitPlace,
    x: i32,
    y: i32,
    unit: i32,
    rect_for: &dyn Fn(i32) -> Option<[i32; 4]>,
) -> i32 {
    let mut result = 0i32;
    if (place.flags & 0x4000) != 0 {
        if place.size >= x {
            result = 0x8000_0011u32 as i32;
        } else if x >= place.x0 - place.size - 1 {
            result = 0x8000_0012u32 as i32;
        }
    }
    if (place.flags & 0x8000) != 0 {
        if place.size >= y {
            result |= 0x8000_0014u32 as i32;
        } else if y >= place.y0 - place.size - 1 {
            result |= 0x8000_0018u32 as i32;
        }
    }
    if result != 0 {
        return result;
    }

    let probe = |code: i32| -> Option<i32> {
        rect_for(code).and_then(|r| {
            if x >= r[0] && x <= r[2] && y >= r[1] && y <= r[3] {
                Some(code)
            } else {
                None
            }
        })
    };

    if (place.flags & 1) != 0 {
        let threshold = if (place.flags & 0x80_0030) == 0x20 || (place.word_5c & 0x80) != 0 {
            place.limit_top
        } else {
            place.limit_top.wrapping_add(unit)
        };
        let probe_if = |flag: u32, code: i32| -> Option<i32> {
            if (place.flags & flag) != 0 {
                probe(code)
            } else {
                None
            }
        };
        let threshold_hit = threshold > y || y < place.limit_top;
        if threshold_hit {
            if let Some(code) = probe_if(2, -4) {
                return code;
            }
            if let Some(code) = probe_if(4, -5) {
                return code;
            }
            if let Some(code) = probe_if(8, -10) {
                return code;
            }
            return -3;
        }
    }
    if (place.flags & 0x200) != 0 && x >= place.limit_right {
        if let Some(code) = probe(-9) {
            return code;
        }
        if let Some(code) = probe(-10) {
            return code;
        }
        return -8;
    }
    if (place.flags & 0x100) != 0 && y >= place.limit_bottom {
        if let Some(code) = probe(-14) {
            return code;
        }
        if let Some(code) = probe(-13) {
            return code;
        }
        return -12;
    }
    0
}

/// The parent detail dispatch's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinDetailPlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub inset: i16,
    pub style: u32,
    pub flags: u32,
}

/// The inset rectangle and the optional detail helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinDetail {
    pub prepare_calls: u32,
    pub rect_calls: u32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub first: bool,
    pub second: bool,
    pub horizontal: Option<[i32; 4]>,
    pub horizontal_offset: i32,
    pub horizontal_mode: i32,
    pub vertical: Option<[i32; 4]>,
}

/// Draws the inset rectangle and dispatches the horizontal or vertical helper.
pub fn draw_skin_detail(place: SkinDetailPlace, unit: i32) -> SkinDetail {
    let i = i32::from(place.inset);
    let x = i32::from(place.x);
    let y = i32::from(place.y);
    let w = i32::from(place.w);
    let h = i32::from(place.h);
    let x1 = x.wrapping_add(i);
    let y1 = y.wrapping_add(i);
    let w1 = w.wrapping_sub(i.wrapping_mul(2));
    let h1 = h.wrapping_sub(i.wrapping_mul(2));
    let first = (place.flags & 0x10) == 0x10;
    let second = (place.flags & 0x40) == 0x40;

    let horizontal = if (place.style & 0x2000_0000) != 0 && !second {
        Some([x, y, w, h])
    } else {
        None
    };
    let vertical = if ((place.style & 0x4000_0000) != 0 && second) || (place.flags & 0x80) != 0 {
        Some([x1, y1, w1, h1])
    } else {
        None
    };

    SkinDetail {
        prepare_calls: 1,
        rect_calls: 1,
        x: x1,
        y: y1,
        w: w1,
        h: h1,
        first,
        second,
        horizontal,
        horizontal_offset: unit.wrapping_neg() / 2,
        horizontal_mode: 0x50,
        vertical,
    }
}

/// The parent rectangle dispatch's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinRectPlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub inset: i16,
    pub flags: u32,
}

/// The inset rectangle and the optional cornered overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinRect {
    pub rect: [i32; 4],
    pub first: bool,
    pub second: bool,
    pub rect_calls: u32,
    pub cornered_calls: u32,
}

/// Draws the inset rectangle and, on flag 0x40, the cornered overlay.
pub fn draw_skin_rect(place: SkinRectPlace) -> SkinRect {
    let i = i32::from(place.inset);
    let x = i32::from(place.x).wrapping_add(i);
    let y = i32::from(place.y).wrapping_add(i);
    let w = i32::from(place.w).wrapping_sub(i.wrapping_mul(2));
    let h = i32::from(place.h).wrapping_sub(i.wrapping_mul(2));
    let first = (place.flags & 0x10) == 0x10;
    let second = (place.flags & 0x40) == 0x40;
    SkinRect {
        rect: [x, y, w, h],
        first,
        second,
        rect_calls: 1,
        cornered_calls: u32::from(second),
    }
}

/// A four-band textured strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinStrip {
    pub vertices: [(f32, f32); 8],
    pub binds: [u32; 4],
    pub begin_calls: u32,
    pub bind_calls: u32,
    pub end_calls: u32,
}

/// Draws the strip between x = first/third and y = second/fourth.
pub fn draw_skin_strip(first: f32, second: f32, third: f32, fourth: f32) -> SkinStrip {
    let mid = (second - fourth) * 0.5 + fourth;
    SkinStrip {
        vertices: [
            (third, second),
            (first, second),
            (third, mid),
            (first, mid),
            (third, mid),
            (first, mid),
            (first, fourth),
            (third, fourth),
        ],
        binds: [0x12c, 0x134, 0x138, 0x130],
        begin_calls: 1,
        bind_calls: 4,
        end_calls: 1,
    }
}

/// The two slider styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinSliderMode {
    Plain,
    Filled,
}

/// The parent slider's inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinSliderPlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub inset: i16,
    pub thumb: i32,
    pub fraction: f32,
    pub style: u32,
    pub flags: u32,
}

/// The selected slider draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinSlider {
    pub mode: SkinSliderMode,
    pub rect: [i32; 4],
    pub thumb: Option<[i32; 4]>,
    pub thumb_first: bool,
    pub thumb_second: bool,
    pub bar: Option<[i32; 4]>,
    pub translate: i32,
    pub begin_calls: u32,
    pub bind_calls: u32,
    pub end_calls: u32,
}

/// Draws the parent slider in its plain or filled style.
pub fn draw_skin_slider(place: SkinSliderPlace, unit: i32) -> Option<SkinSlider> {
    let x = i32::from(place.x);
    let y = i32::from(place.y);
    let w = i32::from(place.w);
    let h = i32::from(place.h);
    let i = i32::from(place.inset);
    match place.style & 0x3000_0000 {
        0 => {
            let n = ((w - 2 * i - place.thumb) as f32 * place.fraction).round_ties_even() as i32;
            Some(SkinSlider {
                mode: SkinSliderMode::Plain,
                rect: [x + 2, y + 2, w - 4, h - 4],
                thumb: Some([x + i + n, y + i, place.thumb, h - 2 * i]),
                thumb_first: (place.flags & 0x10) == 0x10,
                thumb_second: false,
                bar: None,
                translate: n,
                begin_calls: 0,
                bind_calls: 0,
                end_calls: 0,
            })
        }
        0x1000_0000 => {
            let n = ((w - 2 * i - unit) as f32 * place.fraction).round_ties_even() as i32;
            Some(SkinSlider {
                mode: SkinSliderMode::Filled,
                rect: [x + i + 1, y + i + 1, w - 2 * i - 2, h - 2 * i - 2],
                thumb: None,
                thumb_first: false,
                thumb_second: false,
                bar: Some([x + i + n, y, unit, h]),
                translate: 0,
                begin_calls: 1,
                bind_calls: 1,
                end_calls: 1,
            })
        }
        _ => None,
    }
}

/// The four measure fields the setter writes.
pub fn skin_measures(first: i32, second: i32, third: i32, unit: i32) -> [i32; 4] {
    [first, second, third, unit.wrapping_mul(4).wrapping_add(4)]
}

/// The solid-colour fragment shader the skin prepares.
pub const SKIN_SOLID_SHADER: &str =
    "in vec4 vrCol;\r\nout vec4 FragColor;\r\nvoid main() {\r\nFragColor = vrCol;}";
/// The textured fragment shader.
pub const SKIN_TEXTURED_SHADER: &str = "in vec2 vrTex;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform float TexCol;\r\nuniform sampler2D Tex;\r\nvoid main() {\r\nvec4 tcol = texture(Tex, vrTex);\r\ntcol.rgb = tcol.rgb * TexCol + 1.0 - TexCol;\r\nFragColor.rgb = vrCol.rgb * tcol.rgb;FragColor.a = vrCol.a * tcol.a;}";
/// The rounded-rect fragment shader.
pub const SKIN_ROUNDED_SHADER: &str = "in vec2 vrTex;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform vec4 Scale;\r\nvoid main() {\r\nvec2 s = abs(0.5 - vrTex);\r\ns = (s - Scale.xy) * Scale.zw;\r\nfloat a = length(max(s,0.0));\r\na = max(1.0 - a, 0.0);\r\na = a * a;\r\na = a * a;\r\nFragColor.rgb = vrCol.rgb;FragColor.a = vrCol.a * a;}";

/// A uniform assignment or lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkinUniformValue {
    Float(f32),
    Int(i32),
}

/// One uniform operation: a value written, or `None` for a lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinUniform {
    pub name: &'static str,
    pub value: Option<SkinUniformValue>,
}

/// One prepared program with its uniforms.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinProgram {
    pub kind: u32,
    pub source: &'static str,
    pub uniforms: Vec<SkinUniform>,
}

/// The skin's three programs.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinPrograms {
    pub programs: Vec<SkinProgram>,
    pub uniform_calls: u32,
}

/// Builds the skin's three programs once.
pub fn prepare_skin_programs(already: bool) -> SkinPrograms {
    if already {
        return SkinPrograms {
            programs: Vec::new(),
            uniform_calls: 0,
        };
    }
    let mprj = SkinUniform {
        name: "MPrj",
        value: Some(SkinUniformValue::Float(2.0)),
    };
    let scale = SkinUniform {
        name: "Scale",
        value: None,
    };
    let tex = SkinUniform {
        name: "Tex",
        value: Some(SkinUniformValue::Int(0)),
    };
    let texcol = SkinUniform {
        name: "TexCol",
        value: None,
    };
    SkinPrograms {
        programs: vec![
            SkinProgram {
                kind: 5,
                source: SKIN_SOLID_SHADER,
                uniforms: vec![mprj],
            },
            SkinProgram {
                kind: 0x15,
                source: SKIN_TEXTURED_SHADER,
                uniforms: vec![mprj, scale],
            },
            SkinProgram {
                kind: 0x15,
                source: SKIN_ROUNDED_SHADER,
                uniforms: vec![mprj, tex, texcol],
            },
        ],
        uniform_calls: 6,
    }
}

/// The inset bevel's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinInsetBevelPlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub inset: i16,
    pub flags: u32,
}

/// The flag-0x40 arm: fill, border, outline and corner detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinInsetArm {
    pub inner: [i32; 4],
    pub inner_binds: [u32; 2],
    pub border: [i32; 4],
    pub border_binds: [u32; 2],
    pub outline: [i32; 4],
    pub outline_slot: u32,
    pub detail: [i32; 4],
    pub detail_mode: i32,
    pub detail_unit: i32,
}

/// The other arm: one virtual inset rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinInsetPlain {
    pub rect: [i32; 4],
    pub first: bool,
    pub second: bool,
    pub extra_height: i32,
}

/// The inset bevel dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinInsetBevel {
    pub prepare_calls: u32,
    pub arm: Option<SkinInsetArm>,
    pub plain: Option<SkinInsetPlain>,
}

/// Draws the inset bevel or the plain inset rectangle.
pub fn draw_inset_bevel(place: SkinInsetBevelPlace, unit: i32) -> SkinInsetBevel {
    let i = i32::from(place.inset);
    let x = i32::from(place.x);
    let y = i32::from(place.y);
    let w = i32::from(place.w);
    let h = i32::from(place.h);
    let left = x + i;
    let top = y + i;
    let width = w - 2 * i;
    let height = h - 2 * i;
    if (place.flags & 0x40) == 0x40 {
        SkinInsetBevel {
            prepare_calls: 1,
            arm: Some(SkinInsetArm {
                inner: [left + 2, top + 2, width - 4, height - 4],
                inner_binds: [0x74, 0x80],
                border: [left + 1, top + 1, width - 2, height - 2],
                border_binds: [0x7c, 0x88],
                outline: [left, top, width, height],
                outline_slot: 0x6c,
                detail: [left + 1, top + 1, width - 2, height - 2],
                detail_mode: 0xa0,
                detail_unit: unit,
            }),
            plain: None,
        }
    } else {
        SkinInsetBevel {
            prepare_calls: 1,
            arm: None,
            plain: Some(SkinInsetPlain {
                rect: [left, top, width, height],
                first: (place.flags & 0x10) == 0x10,
                second: false,
                extra_height: height,
            }),
        }
    }
}

/// Draws the second parent slider, whose track sits one pixel in from the left.
pub fn draw_skin_scroll(place: SkinSliderPlace, unit: i32) -> Option<SkinSlider> {
    let x = i32::from(place.x);
    let y = i32::from(place.y);
    let w = i32::from(place.w);
    let h = i32::from(place.h);
    let i = i32::from(place.inset);
    match place.style & 0x3000_0000 {
        0 => {
            let n = ((w - 2 * i - place.thumb) as f32 * place.fraction).round_ties_even() as i32;
            Some(SkinSlider {
                mode: SkinSliderMode::Plain,
                rect: [x + 1, y + 2, w - 2, h - 4],
                thumb: Some([x + i + n, y + 1 + i, place.thumb, h - 2 - 2 * i]),
                thumb_first: (place.flags & 0x10) == 0x10,
                thumb_second: false,
                bar: None,
                translate: n,
                begin_calls: 0,
                bind_calls: 0,
                end_calls: 0,
            })
        }
        0x1000_0000 => {
            let n = ((w - 2 * i - unit) as f32 * place.fraction).round_ties_even() as i32;
            Some(SkinSlider {
                mode: SkinSliderMode::Filled,
                rect: [x + i + 1, y + i + 1, w - 2 * i - 2, h - 2 * i - 2],
                thumb: None,
                thumb_first: false,
                thumb_second: false,
                bar: Some([x + i + n, y, unit, h]),
                translate: 0,
                begin_calls: 1,
                bind_calls: 1,
                end_calls: 1,
            })
        }
        _ => None,
    }
}

/// The scroll region's inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinRegionPlace {
    pub origin: [i32; 2],
    pub inset: [i32; 2],
    pub extent: [i32; 2],
    pub total: [i32; 2],
    pub fraction: [f32; 2],
    pub mode: u32,
    pub flags: u32,
    pub translated: bool,
    pub clip_ok: bool,
    pub prepared: bool,
    pub lines: Vec<[i32; 4]>,
    pub children: Vec<bool>,
}

/// The region's clip, scroll, replayed quads and detail.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinRegion {
    pub draw: bool,
    pub clip: [i32; 4],
    pub scroll: [i32; 2],
    pub translate: Option<[f32; 2]>,
    pub lines: Vec<[i32; 4]>,
    pub child_calls: u32,
    pub detail: Option<[i32; 4]>,
    pub detail_mode: i32,
    pub detail_offset: i32,
    pub begin_calls: u32,
    pub vtable_calls: u32,
    pub fallback_calls: u32,
    pub line_end_calls: u32,
    pub scissor_calls: u32,
}

/// Draws the parent skin's scroll region.
pub fn draw_skin_region(place: SkinRegionPlace, unit: i32) -> SkinRegion {
    let translate = place
        .translated
        .then(|| [-(place.origin[0] as f32), place.origin[1] as f32]);
    let early = SkinRegion {
        draw: false,
        clip: [0; 4],
        scroll: [0; 2],
        translate,
        lines: Vec::new(),
        child_calls: 0,
        detail: None,
        detail_mode: 0,
        detail_offset: 0,
        begin_calls: 1,
        vtable_calls: 1,
        fallback_calls: u32::from(!place.prepared),
        line_end_calls: 0,
        scissor_calls: 0,
    };
    if (place.mode & 1) != 0 {
        return early;
    }
    let clip = [
        place.origin[0] + place.inset[0],
        place.origin[1] + place.inset[1],
        place.extent[0] - place.inset[0],
        place.extent[1] - place.inset[1],
    ];
    if !place.clip_ok {
        return early;
    }
    let scroll = if (place.flags & 0x800) != 0 {
        [0, 0]
    } else {
        [
            ((place.total[0] - clip[2]) as f32 * place.fraction[0]).round_ties_even() as i32,
            ((place.total[1] - clip[3]) as f32 * place.fraction[1]).round_ties_even() as i32,
        ]
    };
    let (detail, detail_mode, detail_offset) = if (place.flags & 0x80) != 0 {
        (Some([scroll[0], scroll[1], clip[2], clip[3]]), 0x30, unit)
    } else if (place.flags & 0x10_0000) != 0 {
        (
            Some([scroll[0], scroll[1], clip[2], clip[3]]),
            0x50,
            unit.wrapping_mul(4),
        )
    } else {
        (None, 0, 0)
    };
    let child_calls = place.children.iter().filter(|visible| **visible).count() as u32;
    let line_end_calls = u32::from(!place.lines.is_empty());
    SkinRegion {
        draw: true,
        clip,
        scroll,
        translate,
        lines: place.lines.clone(),
        child_calls,
        detail,
        detail_mode,
        detail_offset,
        begin_calls: 2,
        vtable_calls: 2,
        fallback_calls: u32::from(!place.prepared),
        line_end_calls,
        scissor_calls: 2,
    }
}

/// A GUI colour: red, green, blue and alpha, in the order the original
/// stores them.
pub type SkinInk = [u8; 4];

/// The twelve shades one scheme colour expands into.
pub type SkinGroup = [SkinInk; 12];

/// The skin's five colour groups and the accent shades the scheme derives
/// from its accent colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinPalette {
    /// The form's own colours.
    pub form: SkinGroup,
    /// The controls' idle colours.
    pub control: SkinGroup,
    /// A copy of `form`, read by hot controls.
    pub form_hot: SkinGroup,
    /// The label inks.
    pub text: SkinGroup,
    /// The selection and highlight inks.
    pub highlight: SkinGroup,
    /// The accent at half strength.
    pub accent_dim: SkinInk,
    /// The accent at 1.9, then tuned by 0.8.
    pub accent_bright: SkinInk,
    /// The accent mixes at 0.4, 0.6, 0.5 and 0.9 towards the bright end.
    pub accent_mix: [SkinInk; 4],
    /// The two stored alpha bytes the ramp and the accent use.
    pub first_alpha: u8,
    pub second_alpha: u8,
    /// The accent's two derived alpha bytes.
    pub accent_alpha_a: u8,
    pub accent_alpha_b: u8,
}

impl SkinPalette {
    /// A palette whose twelve entries of every group hold `ink`. A freshly
    /// allocated skin is zeroed; the base skin fills its palette before a
    /// scheme is read, and that fill is outside this component (guess).
    pub fn filled(ink: SkinInk) -> Self {
        SkinPalette {
            form: [ink; 12],
            control: [ink; 12],
            form_hot: [ink; 12],
            text: [ink; 12],
            highlight: [ink; 12],
            accent_dim: ink,
            accent_bright: ink,
            accent_mix: [ink; 4],
            first_alpha: 0,
            second_alpha: 0,
            accent_alpha_a: 0,
            accent_alpha_b: 0,
        }
    }
}

/// A scheme record: four colours with the scale and tone each expands by,
/// the accent and the two alpha bytes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinScheme {
    pub groups: [(SkinInk, f32, f32); 4],
    pub accent: SkinInk,
    pub accent_scale: f32,
    pub first_alpha: u8,
    pub second_alpha: u8,
}

/// `MINSS`'s operand order: the second operand comes back when either is NaN.
fn min_ss(first: f32, second: f32) -> f32 {
    if first.is_nan() || second.is_nan() || second < first {
        second
    } else {
        first
    }
}

/// One colour channel scaled by `factor` and saturated to a byte, as the
/// original's packed conversion does. A result that does not fit the
/// conversion's integer range stores zero.
fn scaled_channel(channel: u8, factor: f32) -> u8 {
    let value = f32::from(channel) * factor;
    rounded_byte(value)
}

/// Rounds to nearest even and saturates to a byte; a non-finite or
/// out-of-range input stores zero, as the original's conversion does.
fn rounded_byte(value: f32) -> u8 {
    if !value.is_finite() || value <= -2_147_483_648.0 || value >= 2_147_483_648.0 {
        return 0;
    }
    let rounded = value.round_ties_even();
    if rounded <= 0.0 {
        0
    } else if rounded >= 255.0 {
        255
    } else {
        rounded as u8
    }
}

/// Scales a colour's red, green and blue; the alpha is kept.
fn scale_rgb(ink: &mut SkinInk, factor: f32) {
    for channel in &mut ink[..3] {
        *channel = scaled_channel(*channel, factor);
    }
}

/// Blends a colour's red, green and blue towards its luminance by `amount`;
/// the alpha is kept. The luminance is 0.299 red, 0.587 green, 0.114 blue,
/// summed as the original sums it.
fn tone_rgb(ink: &mut SkinInk, amount: f32) {
    let red = f32::from(ink[0]);
    let green = f32::from(ink[1]);
    let blue = f32::from(ink[2]);
    let luminance = red * 0.299 + (green * 0.587 + blue * 0.114);
    for channel in &mut ink[..3] {
        let value = f32::from(*channel);
        *channel = rounded_byte((luminance - value) * amount + value);
    }
}

/// Writes a colour's red, green and blue scaled by `factor`; the
/// destination's alpha is kept, as the original's three-byte stores do.
fn set_scaled_rgb(ink: &mut SkinInk, base: SkinInk, factor: f32) {
    for channel in 0..3 {
        ink[channel] = scaled_channel(base[channel], factor);
    }
}

/// Expands one scheme colour into the twelve shades of a group. `scale`
/// shrinks and grows the colour, `tone` pulls it towards its luminance.
/// Entries 0 to 3 and 5 to 7 keep the destination's alpha, as the
/// original's three-byte stores do.
pub fn expand_group(group: &mut SkinGroup, base: SkinInk, scale: f32, tone: f32) {
    let inverse = 1.0 / scale;
    group[4] = base;
    set_scaled_rgb(&mut group[5], base, scale);
    set_scaled_rgb(&mut group[6], base, scale * scale);
    let mut power = scale * scale * scale;
    set_scaled_rgb(&mut group[7], base, power);
    group[8] = base;
    tone_rgb(&mut group[8], min_ss(1.0, tone * 1.75));
    power *= scale;
    scale_rgb(&mut group[8], power);
    group[9] = base;
    tone_rgb(&mut group[9], min_ss(1.0, tone * 2.0));
    power *= scale;
    scale_rgb(&mut group[9], power);
    group[10] = base;
    tone_rgb(&mut group[10], min_ss(1.0, tone * 2.25));
    power *= scale;
    scale_rgb(&mut group[10], power);
    group[11] = base;
    tone_rgb(&mut group[11], min_ss(1.0, tone * 2.5));
    power *= scale;
    scale_rgb(&mut group[11], power);

    set_scaled_rgb(&mut group[3], base, inverse);
    let square = inverse * inverse;
    set_scaled_rgb(&mut group[2], base, square);
    let cube = square * inverse * 0.909_090_94;
    set_scaled_rgb(&mut group[1], base, cube);
    let fourth = cube * inverse * 0.909_090_94;
    set_scaled_rgb(&mut group[0], base, fourth);

    tone_rgb(&mut group[8], tone * 0.5);
    tone_rgb(&mut group[9], min_ss(1.0, tone));
    tone_rgb(&mut group[10], min_ss(1.0, tone * 1.25));
    tone_rgb(&mut group[11], min_ss(1.0, tone * 1.5));
}

/// Reads a scheme into the palette, as the skin's loader does. The first
/// group becomes both the form and the hot-form group, the next three
/// become the control, label and highlight groups, and the accent shades
/// are mixed from the accent and the first group's third shade.
pub fn load_scheme(palette: &mut SkinPalette, scheme: &SkinScheme) {
    for (group, (base, scale, tone)) in [
        &mut palette.form,
        &mut palette.control,
        &mut palette.text,
        &mut palette.highlight,
    ]
    .into_iter()
    .zip(scheme.groups)
    {
        expand_group(group, base, scale, tone);
    }
    palette.form_hot = palette.form;

    let mut dim = scheme.accent;
    scale_rgb(&mut dim, 0.5);
    palette.accent_dim = dim;
    let mut bright = scheme.accent;
    scale_rgb(&mut bright, 1.9);
    tone_rgb(&mut bright, 0.8);
    palette.accent_bright = bright;
    palette.accent_mix = [
        mix_ink(dim, bright, 0.4),
        mix_ink(dim, bright, 0.6),
        mix_ink(dim, bright, 0.5),
        mix_ink(dim, bright, 0.9),
    ];
    let amount = f32::from(scheme.second_alpha) * scheme.accent_scale * 0.003_921_569;
    palette.accent_mix[2] = mix_ink(palette.form[3], palette.accent_mix[2], amount);
    palette.accent_mix[3] = mix_ink(palette.form[3], palette.accent_mix[3], amount);
    palette.first_alpha = scheme.first_alpha;
    palette.second_alpha = scheme.second_alpha;
    let gate = f32::from(scheme.second_alpha) * scheme.accent_scale;
    palette.accent_alpha_a = (gate.round_ties_even() as i32) as u8;
    palette.accent_alpha_b = palette.accent_alpha_a;
}

/// Scales the palette's alpha bytes: the form group and the two accent
/// mixes take the first ramp, every other group takes the second. The
/// highlight group's last entry keeps its alpha, as the original leaves it.
pub fn scale_palette_alpha(palette: &mut SkinPalette, first_scale: f32, second_scale: f32) {
    let first = low_byte(f32::from(palette.first_alpha) * first_scale);
    let second = low_byte(f32::from(palette.second_alpha) * second_scale);
    for ink in &mut palette.form {
        ink[3] = first;
    }
    for group in [
        &mut palette.control,
        &mut palette.form_hot,
        &mut palette.text,
    ] {
        for ink in group {
            ink[3] = second;
        }
    }
    for ink in &mut palette.highlight[..11] {
        ink[3] = second;
    }
    palette.accent_mix[2][3] = first;
    palette.accent_mix[3][3] = first;
}

/// The skin's layout metrics: the border, padding, title and row sizes the
/// constructor stores, and the reach that follows the GUI unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinLayout {
    pub border: i32,
    pub pad: i32,
    pub title: i32,
    pub reach: i32,
    pub edge: i32,
    pub gap: i32,
    pub head: i32,
    pub row: i32,
}

/// Sets the layout metrics from the GUI unit: border 2, pad 2, title 24,
/// reach four units plus four.
pub fn set_skin_metrics(layout: &mut SkinLayout, unit: i32) {
    layout.border = 2;
    layout.pad = 2;
    layout.title = 24;
    layout.reach = unit.wrapping_mul(4).wrapping_add(4);
}

/// A skin's constructor: the default sizes, then the metrics from the unit.
pub fn skin_constructor(unit: i32) -> SkinLayout {
    let mut layout = SkinLayout {
        border: 2,
        pad: 2,
        title: 24,
        reach: 0,
        edge: 2,
        gap: 2,
        head: 24,
        row: 20,
    };
    set_skin_metrics(&mut layout, unit);
    layout
}

/// A rectangle in the skin's window pixels: left, top, right and bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuiRect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl GuiRect {
    /// A rectangle from its place and size.
    pub fn sized(x: f32, y: f32, w: f32, h: f32) -> Self {
        GuiRect {
            x0: x,
            y0: y,
            x1: x + w,
            y1: y + h,
        }
    }

    /// A rectangle from its corners.
    pub fn corners(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        GuiRect { x0, y0, x1, y1 }
    }
}

/// The program a batch's geometry is submitted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinMode {
    /// No program has been selected.
    None,
    /// Flat colours, through the solid program.
    Solid,
    /// A texture tinted by the `TexCol` uniform.
    Textured,
    /// Flat colours clipped by the `Scale` uniform's rounded rectangle.
    Rounded,
    /// The GUI subclass' own program.
    Skin,
}

/// How a batch's vertices are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiDraw {
    /// Two triangles for every four vertices.
    Quads,
    /// Consecutive pairs of vertices as lines.
    Lines,
    /// Every three vertices as a triangle.
    Triangles,
}

/// A vertex in window pixels: its place, texture coordinate and ink.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuiVertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    pub ink: SkinInk,
}

/// One submitted piece of geometry with the program state it needs.
#[derive(Debug, Clone, PartialEq)]
pub struct GuiPrimitive {
    pub mode: SkinMode,
    pub blend: u16,
    pub draw: GuiDraw,
    pub texture_shift: [f32; 2],
    pub rounded_scale: Option<[f32; 4]>,
    pub tex_col: Option<f32>,
    pub vertices: Vec<GuiVertex>,
}

/// The skin's immediate-mode batcher: the program, blend and ink state the
/// original keeps in globals, and the geometry gathered between a begin and
/// an end. Geometry stays in window pixels; the projection to clip space is
/// applied when the batch is handed to the renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinBatch {
    pub mode: SkinMode,
    /// The mode the subclass reports when the skin asks for it.
    pub subclass_mode: u32,
    pub blend: u16,
    pub ink: SkinInk,
    pub u: f32,
    pub v: f32,
    pub vertices: Vec<GuiVertex>,
    pub primitives: Vec<GuiPrimitive>,
    pub texture_shift: [f32; 2],
    pub rounded_scale: Option<[f32; 4]>,
    /// The textured program's `TexCol`, when one has been set.
    pub tex_col: Option<f32>,
    pub begin_calls: u32,
    pub end_calls: u32,
    pub mode_calls: u32,
    /// The texture binds the original's texture setter performs.
    pub texture_binds: u32,
}

impl Default for SkinBatch {
    fn default() -> Self {
        SkinBatch {
            mode: SkinMode::None,
            subclass_mode: 0,
            blend: 0,
            ink: [0; 4],
            u: 0.0,
            v: 0.0,
            vertices: Vec::new(),
            primitives: Vec::new(),
            texture_shift: [0.0; 2],
            rounded_scale: None,
            tex_col: None,
            begin_calls: 0,
            end_calls: 0,
            mode_calls: 0,
            texture_binds: 0,
        }
    }
}

impl SkinBatch {
    /// An empty batch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects the renderer for `mode`, as the skin's mode switch does. Mode
    /// 0x10 asks the subclass for its own mode. A mode the switch does not
    /// name is stored and binds nothing.
    pub fn select(&mut self, mode: u32, subclass: u32, textured_tint: bool) {
        self.mode_calls += 1;
        if mode == 0x10 {
            self.mode = SkinMode::Skin;
            self.subclass_mode = subclass;
            return;
        }
        let chosen = match mode {
            1 => Some((SkinMode::Solid, None)),
            2 => Some((
                SkinMode::Textured,
                Some(if textured_tint { 1.0 } else { 0.0 }),
            )),
            3 => Some((SkinMode::Rounded, None)),
            5 => Some((SkinMode::Textured, Some(0.0))),
            _ => None,
        };
        if let Some((chosen_mode, tex_col)) = chosen {
            self.mode = chosen_mode;
            if let Some(value) = tex_col {
                self.tex_col = Some(value);
            }
        }
    }

    /// Asks the subclass for its mode, as mode 0x10 does through its slot.
    pub fn select_subclass(&mut self, subclass: u32) {
        self.select(0x10, subclass, false);
    }

    /// Replaces the texture offset. The original stores it in 512ths.
    pub fn set_texture_shift(&mut self, x: f32, y: f32) {
        self.texture_shift = [x * 0.001_953_125, y * 0.001_953_125];
    }

    /// Adds to the texture offset, in 512ths.
    pub fn add_texture_shift(&mut self, x: f32, y: f32) {
        self.texture_shift[0] += x * 0.001_953_125;
        self.texture_shift[1] += y * 0.001_953_125;
    }

    /// Starts a run of vertices and clears the vertex list.
    pub fn begin(&mut self) {
        self.begin_calls += 1;
        self.vertices.clear();
    }

    /// Sets the current ink.
    pub fn set_ink(&mut self, ink: SkinInk) {
        self.ink = ink;
    }

    /// Sets the current ink from its bytes.
    pub fn set_rgba(&mut self, red: u8, green: u8, blue: u8, alpha: u8) {
        self.ink = [red, green, blue, alpha];
    }

    /// Sets the current texture coordinate.
    pub fn set_uv(&mut self, u: f32, v: f32) {
        self.u = u;
        self.v = v;
    }

    /// Appends a vertex at the current ink and texture coordinate.
    pub fn vertex(&mut self, x: f32, y: f32) {
        self.vertices.push(GuiVertex {
            x,
            y,
            u: self.u,
            v: self.v,
            ink: self.ink,
        });
    }

    /// Ends the run and submits it as quads.
    pub fn end_quads(&mut self) {
        self.end(GuiDraw::Quads);
    }

    /// Ends the run and submits it as lines.
    pub fn end_lines(&mut self) {
        self.end(GuiDraw::Lines);
    }

    /// Ends the run and submits it as triangles.
    pub fn end_triangles(&mut self) {
        self.end(GuiDraw::Triangles);
    }

    fn end(&mut self, draw: GuiDraw) {
        self.end_calls += 1;
        if self.vertices.is_empty() {
            return;
        }
        let vertices = std::mem::take(&mut self.vertices);
        self.primitives.push(GuiPrimitive {
            mode: self.mode,
            blend: self.blend,
            draw,
            texture_shift: self.texture_shift,
            rounded_scale: self.rounded_scale,
            tex_col: self.tex_col,
            vertices,
        });
    }
}

/// Sets the rounded program's `Scale` from a place and the radius the
/// caller adds on each side: the rect's share of the grown sides, then the
/// reciprocal of what is left of half the coordinate space.
pub fn rounded_scale(rect: GuiRect, radius: f32) -> [f32; 4] {
    let grow = radius * 2.0;
    let sx = (rect.x1 - rect.x0) / (grow + (rect.x1 - rect.x0)) * 0.5;
    let sy = (rect.y1 - rect.y0) / (grow + (rect.y1 - rect.y0)) * 0.5;
    [sx, sy, 1.0 / (0.5 - sx), 1.0 / (0.5 - sy)]
}

/// The original's line rectangle: the corners pull in half a pixel, then
/// the four sides are drawn as four line pairs.
pub fn outline_rect(batch: &mut SkinBatch, rect: GuiRect) {
    let x0 = rect.x0 + 0.5;
    let y0 = rect.y0 + 0.5;
    let x1 = rect.x1 - 0.5;
    let y1 = rect.y1 - 0.5;
    batch.begin();
    batch.vertex(x1, y0);
    batch.vertex(x0, y0);
    batch.vertex(x0, y0);
    batch.vertex(x0, y1);
    batch.vertex(x0, y1);
    batch.vertex(x1, y1);
    batch.vertex(x1, y1);
    batch.vertex(x1, y0);
    batch.end_lines();
}

/// A vertical gradient with four stops: the top ink, its 0.57 and 0.43
/// mixes towards the bottom ink, then the bottom ink.
pub fn fade_rect(batch: &mut SkinBatch, rect: GuiRect, bottom: SkinInk, top: SkinInk) {
    let high = mix_ink(bottom, top, 0.57);
    let low = mix_ink(bottom, top, 0.43);
    let mid = (rect.y0 - rect.y1) * 0.5 + rect.y1;
    batch.begin();
    batch.set_ink(top);
    batch.vertex(rect.x1, rect.y0);
    batch.vertex(rect.x0, rect.y0);
    batch.set_ink(high);
    batch.vertex(rect.x0, mid);
    batch.vertex(rect.x1, mid);
    batch.set_ink(low);
    batch.vertex(rect.x1, mid);
    batch.vertex(rect.x0, mid);
    batch.set_ink(bottom);
    batch.vertex(rect.x0, rect.y1);
    batch.vertex(rect.x1, rect.y1);
    batch.end_quads();
}

/// A vertical gradient whose two inks have their alphas scaled by `alpha`
/// first, with the 0.4 and 0.6 mixes between them.
pub fn faded_rect(batch: &mut SkinBatch, rect: GuiRect, bottom: SkinInk, top: SkinInk, alpha: f32) {
    let scaled = |ink: SkinInk| {
        let mut scaled = ink;
        scaled[3] = rounded_byte(f32::from(ink[3]) * alpha);
        scaled
    };
    let bottom = scaled(bottom);
    let top = scaled(top);
    let high = mix_ink(bottom, top, 0.6);
    let low = mix_ink(bottom, top, 0.4);
    let mid = (rect.y0 - rect.y1) * 0.5 + rect.y1;
    batch.begin();
    batch.set_ink(top);
    batch.vertex(rect.x1, rect.y0);
    batch.vertex(rect.x0, rect.y0);
    batch.set_ink(high);
    batch.vertex(rect.x0, mid);
    batch.vertex(rect.x1, mid);
    batch.set_ink(low);
    batch.vertex(rect.x1, mid);
    batch.vertex(rect.x0, mid);
    batch.set_ink(bottom);
    batch.vertex(rect.x0, rect.y1);
    batch.vertex(rect.x1, rect.y1);
    batch.end_quads();
}

/// A bar of vertical stripes. `inks` holds the count plus one: each stripe
/// runs from one colour to the next, and the last colour closes the bar.
/// The stripe boundaries are the count's fractions of the width, rounded to
/// nearest even.
pub fn stripe_bar(batch: &mut SkinBatch, rect: GuiRect, inks: &[SkinInk]) {
    let count = inks.len().saturating_sub(1);
    if count == 0 {
        return;
    }
    let step = ((rect.x1 - rect.x0) as f64 / count as f64) as f32;
    let mut ink = inks[0];
    ink[3] = 0xff;
    batch.begin();
    batch.set_ink(ink);
    let mut x = rect.x0;
    for (index, &next) in inks.iter().enumerate().skip(1) {
        batch.vertex(x, rect.y0);
        batch.vertex(x, rect.y1);
        batch.set_ink(with_alpha(next, 0xff));
        x = rect.x0 + (index as f32 * step).round_ties_even();
        batch.vertex(x, rect.y1);
        batch.vertex(x, rect.y0);
    }
    batch.end_quads();
}

/// The translucent band a listing draws over a selected row: an outline
/// whose middle is the highlight ink, then glow bands above and below.
/// Selecting the row widens the glow by the unit.
pub fn selection_band(
    batch: &mut SkinBatch,
    rect: GuiRect,
    ink: SkinInk,
    highlight: SkinInk,
    selected: bool,
    unit: i32,
) {
    let edge = [ink[0], ink[1], ink[2], 25];
    let low = [highlight[0], highlight[1], highlight[2], 200];
    let top = [ink[0], ink[1], ink[2], 100];
    let mid = [ink[0], ink[1], ink[2], 30];
    let clear = [ink[0], ink[1], ink[2], 0];
    let corner = -(unit as f32);
    let reach = if selected {
        -((unit * 2) as f32)
    } else {
        -((unit + 1) as f32)
    };
    let left = rect.x0;
    let right = rect.x1;
    let top_y = rect.y0;
    let bottom = rect.y1 - 1.0;
    let centre = (rect.x0 + rect.x1) * 0.5;
    let low_y = rect.y0 + 1.0;
    let high_y = low_y + (rect.y1 - rect.y0 - 2.0);
    batch.blend = 0x201;
    batch.begin();
    batch.set_ink(edge);
    batch.vertex(left, top_y);
    batch.set_ink(low);
    batch.vertex(centre, top_y);
    batch.vertex(centre, top_y);
    batch.set_ink(edge);
    batch.vertex(right, top_y);
    batch.vertex(right, bottom);
    batch.set_ink(low);
    batch.vertex(centre, bottom);
    batch.vertex(centre, bottom);
    batch.set_ink(edge);
    batch.vertex(left, bottom);
    batch.end_lines();
    batch.begin();
    batch.set_ink(clear);
    batch.vertex(centre, low_y - reach);
    batch.set_ink(top);
    batch.vertex(centre, low_y);
    batch.set_ink(mid);
    batch.vertex(left, low_y);
    batch.vertex(left, low_y);
    batch.set_ink(clear);
    batch.vertex(left, low_y - corner);
    batch.vertex(centre, low_y - reach);
    batch.vertex(centre, low_y - reach);
    batch.set_ink(mid);
    batch.vertex(right, low_y);
    batch.set_ink(top);
    batch.vertex(centre, low_y);
    batch.set_ink(clear);
    batch.vertex(centre, low_y - reach);
    batch.vertex(right, low_y - corner);
    batch.set_ink(mid);
    batch.vertex(right, low_y);
    batch.vertex(left, high_y);
    batch.set_ink(clear);
    batch.vertex(centre, high_y - reach);
    batch.vertex(left, high_y - corner);
    batch.vertex(centre, high_y - reach);
    batch.set_ink(mid);
    batch.vertex(left, high_y);
    batch.set_ink(top);
    batch.vertex(centre, high_y);
    batch.vertex(centre, high_y);
    batch.set_ink(mid);
    batch.vertex(right, high_y);
    batch.set_ink(clear);
    batch.vertex(centre, high_y - reach);
    batch.vertex(right, high_y - corner);
    batch.vertex(centre, high_y - reach);
    batch.set_ink(mid);
    batch.vertex(right, high_y);
    batch.end_triangles();
    batch.blend = 0x203;
}

/// Rounds to nearest even and keeps the low byte, as the original does for
/// an alpha word.
fn rounded_byte_wrap(value: f32) -> u8 {
    if !value.is_finite() || value <= -2_147_483_648.0 || value >= 2_147_483_648.0 {
        0
    } else {
        (value.round_ties_even() as i32) as u8
    }
}

/// Draws a rounded rectangle through the rounded program. `radius` is the
/// side the caller adds, multiplied by four units. `alpha` is the colour's
/// share of 255.
pub fn rounded_fill(batch: &mut SkinBatch, rect: GuiRect, unit: i32, radius: f32, alpha: f32) {
    let grow = unit as f32 * radius * 4.0;
    batch.select(3, 0, false);
    batch.set_rgba(0, 0, 0, rounded_byte_wrap(alpha * 255.0));
    batch.rounded_scale = Some(rounded_scale(rect, grow));
    batch.begin();
    batch.set_uv(0.0, 0.0);
    batch.vertex(rect.x0 - grow, rect.y0 - grow);
    batch.set_uv(0.0, 1.0);
    batch.vertex(rect.x0 - grow, rect.y1 + grow);
    batch.set_uv(1.0, 1.0);
    batch.vertex(rect.x1 + grow, rect.y1 + grow);
    batch.set_uv(1.0, 0.0);
    batch.vertex(rect.x1 + grow, rect.y0 - grow);
    batch.end_quads();
}

/// The integer-rectangle variant: the radius is an integer multiple of the
/// unit and the alpha is already a byte.
pub fn rounded_fill_fixed(batch: &mut SkinBatch, rect: GuiRect, unit: i32, radius: i32, alpha: u8) {
    let grow = (i64::from(unit) * i64::from(radius) * 4) as f32;
    batch.select(3, 0, false);
    batch.set_rgba(0, 0, 0, alpha);
    batch.rounded_scale = Some(rounded_scale(rect, grow));
    batch.begin();
    batch.set_uv(0.0, 0.0);
    batch.vertex(rect.x0 - grow, rect.y0 - grow);
    batch.set_uv(0.0, 1.0);
    batch.vertex(rect.x0 - grow, rect.y1 + grow);
    batch.set_uv(1.0, 1.0);
    batch.vertex(rect.x1 + grow, rect.y1 + grow);
    batch.set_uv(1.0, 0.0);
    batch.vertex(rect.x1 + grow, rect.y0 - grow);
    batch.end_quads();
}

/// The rounded program's textured border: four bands around the rectangle,
/// each sampling the corner texture coordinates the rounded shader blends.
pub fn rounded_textured_frame(
    batch: &mut SkinBatch,
    rect: GuiRect,
    unit: i32,
    radius: i32,
    alpha: u8,
) {
    let grow = (i64::from(unit) * i64::from(radius)) as f32;
    batch.select(3, 0, false);
    batch.set_rgba(0, 0, 0, alpha);
    let scale = rounded_scale(rect, grow);
    let sx = scale[0];
    let sy = scale[1];
    batch.rounded_scale = Some(scale);
    let x0 = rect.x0;
    let y0 = rect.y0;
    let x1 = rect.x1;
    let y1 = rect.y1;
    batch.begin();
    batch.set_uv(0.0, 0.0);
    batch.vertex(x0 - grow, y0 - grow);
    batch.set_uv(0.0, 0.5 - sy);
    batch.vertex(x0 - grow, y0);
    batch.set_uv(1.0, 0.5 - sy);
    batch.vertex(x1 + grow, y0);
    batch.set_uv(1.0, 0.0);
    batch.vertex(x1 + grow, y0 - grow);
    batch.set_uv(0.0, 0.5 - sy);
    batch.vertex(x0 - grow, y0);
    batch.set_uv(0.5 - sx, 0.5 - sy);
    batch.vertex(x0, y0);
    batch.set_uv(0.5 - sx, 0.5 + sy);
    batch.vertex(x0, y1);
    batch.set_uv(0.0, 0.5 + sy);
    batch.vertex(x0 - grow, y1);
    batch.set_uv(0.0, 0.5 + sy);
    batch.vertex(x0 - grow, y1);
    batch.set_uv(0.0, 1.0);
    batch.vertex(x0 - grow, y1 + grow);
    batch.set_uv(1.0, 1.0);
    batch.vertex(x1 + grow, y1 + grow);
    batch.set_uv(1.0, 0.5 + sy);
    batch.vertex(x1 + grow, y1);
    batch.set_uv(1.0, 0.5 - sy);
    batch.vertex(x1 + grow, y0);
    batch.set_uv(0.5 + sx, 0.5 - sy);
    batch.vertex(x1, y0);
    batch.set_uv(0.5 + sx, 0.5 + sy);
    batch.vertex(x1, y1);
    batch.set_uv(1.0, 0.5 + sy);
    batch.vertex(x1 + grow, y1);
    batch.end_quads();
}

/// Grows a rounded rectangle by the unit times `radius` and fills it.
pub fn rounded_grow(batch: &mut SkinBatch, rect: GuiRect, unit: i32, radius: f32, alpha: f32) {
    let grow = unit as f32 * radius;
    batch.select(3, 0, false);
    batch.set_rgba(0, 0, 0, rounded_byte_wrap(alpha * 255.0));
    batch.rounded_scale = Some(rounded_scale(rect, grow));
    batch.begin();
    batch.set_uv(0.0, 0.0);
    batch.vertex(rect.x0 - grow, rect.y0 - grow);
    batch.set_uv(0.0, 1.0);
    batch.vertex(rect.x0 - grow, rect.y1 + grow);
    batch.set_uv(1.0, 1.0);
    batch.vertex(rect.x1 + grow, rect.y1 + grow);
    batch.set_uv(1.0, 0.0);
    batch.vertex(rect.x1 + grow, rect.y0 - grow);
    batch.end_quads();
}

/// A four-sided line frame whose sides carry one ink each: top, left,
/// right and bottom, in that order.
pub fn edge_frame(batch: &mut SkinBatch, rect: GuiRect, inks: [SkinInk; 4]) {
    let x0 = rect.x0 + 0.5;
    let y0 = rect.y0 + 0.5;
    let x1 = rect.x1 - 0.5;
    let y1 = rect.y1 - 0.5;
    batch.begin();
    batch.set_ink(inks[0]);
    batch.vertex(x1 - 0.5, y0);
    batch.vertex(x0, y0);
    batch.set_ink(inks[1]);
    batch.vertex(x0, y0);
    batch.vertex(x0, y1);
    batch.set_ink(inks[3]);
    batch.vertex(x0, y1);
    batch.vertex(x1, y1);
    batch.set_ink(inks[2]);
    batch.vertex(x1, y1);
    batch.vertex(x1, y0 - 1.0);
    batch.end_lines();
}

/// Two line frames, the second one pixel inside the first.
pub fn double_frame(batch: &mut SkinBatch, rect: GuiRect, outer: SkinInk, inner: SkinInk) {
    let frame = |batch: &mut SkinBatch, rect: GuiRect, ink: SkinInk| {
        batch.set_ink(ink);
        batch.vertex(rect.x1 - 0.5, rect.y0 + 0.5);
        batch.vertex(rect.x0 + 0.5, rect.y0 + 0.5);
        batch.vertex(rect.x0 + 0.5, rect.y0 + 0.5);
        batch.vertex(rect.x0 + 0.5, rect.y1 - 0.5);
        batch.vertex(rect.x0 + 0.5, rect.y1 - 0.5);
        batch.vertex(rect.x1 - 0.5, rect.y1 - 0.5);
        batch.vertex(rect.x1 - 0.5, rect.y1 - 0.5);
        batch.vertex(rect.x1 - 0.5, rect.y0 - 0.5);
    };
    batch.begin();
    frame(batch, rect, outer);
    frame(
        batch,
        GuiRect {
            x0: rect.x0 + 1.0,
            y0: rect.y0 + 1.0,
            x1: rect.x1 - 1.0,
            y1: rect.y1 - 1.0,
        },
        inner,
    );
    batch.end_lines();
}

/// A line frame whose left and right halves fade between three inks: the
/// top and bottom inks and the 0.65 and 0.35 mixes towards the middle ink.
pub fn gradient_frame(
    batch: &mut SkinBatch,
    rect: GuiRect,
    top: SkinInk,
    middle: SkinInk,
    bottom: SkinInk,
) {
    let high = mix_ink(bottom, middle, 0.65);
    let low = mix_ink(bottom, middle, 0.35);
    let x0 = rect.x0 + 0.5;
    let y0 = rect.y0 + 0.5;
    let x1 = rect.x1 - 0.5;
    let y1 = rect.y1 - 0.5;
    let middle_y = (y0 - y1) * 0.5 + y1 + 0.5;
    batch.begin();
    batch.set_ink(top);
    batch.vertex(x0 + 1.0, y0);
    batch.vertex(x1, y0);
    batch.set_ink(middle);
    batch.vertex(x0, y0);
    batch.set_ink(high);
    batch.vertex(x0, middle_y);
    batch.set_ink(low);
    batch.vertex(x0, middle_y);
    batch.set_ink(bottom);
    batch.vertex(x0, y1);
    batch.vertex(x0, y1);
    batch.vertex(x1, y1);
    batch.vertex(x1, y1);
    batch.set_ink(low);
    batch.vertex(x1, middle_y);
    batch.set_ink(high);
    batch.vertex(x1, middle_y);
    batch.set_ink(middle);
    batch.vertex(x1, y0 - 1.0);
    batch.end_lines();
}

/// A solid rectangle in the current ink.
pub fn solid_rect(batch: &mut SkinBatch, rect: GuiRect) {
    batch.begin();
    batch.vertex(rect.x1, rect.y0);
    batch.vertex(rect.x0, rect.y0);
    batch.vertex(rect.x0, rect.y1);
    batch.vertex(rect.x1, rect.y1);
    batch.end_quads();
}

/// A rectangle with one ink on its top edge and another on its bottom.
pub fn pair_rect(batch: &mut SkinBatch, rect: GuiRect, bottom: SkinInk, top: SkinInk) {
    batch.begin();
    batch.set_ink(top);
    batch.vertex(rect.x1, rect.y0);
    batch.vertex(rect.x0, rect.y0);
    batch.set_ink(bottom);
    batch.vertex(rect.x0, rect.y1);
    batch.vertex(rect.x1, rect.y1);
    batch.end_quads();
}

/// A one-pixel line box whose top and right sides are one ink and whose
/// left and bottom sides are another.
pub fn bordered_rect(batch: &mut SkinBatch, rect: GuiRect, inner: SkinInk, outer: SkinInk) {
    let x0 = rect.x0 + 0.5;
    let y0 = rect.y0 + 0.5;
    let x1 = rect.x1 - 0.5;
    let y1 = rect.y1 - 0.5;
    batch.begin();
    batch.set_ink(outer);
    batch.vertex(x1, y0);
    batch.vertex(x0, y0);
    batch.vertex(x0, y0);
    batch.set_ink(inner);
    batch.vertex(x0, y1);
    batch.vertex(x0, y1);
    batch.vertex(x1, y1);
    batch.vertex(x1, y1);
    batch.set_ink(outer);
    batch.vertex(x1, y0);
    batch.end_lines();
}

/// A four-stop vertical band: `inks` runs from the top to the bottom.
pub fn four_stop_rect(batch: &mut SkinBatch, rect: GuiRect, inks: [SkinInk; 4]) {
    let middle = (rect.y0 - rect.y1) * 0.5 + rect.y1;
    batch.begin();
    batch.set_ink(inks[0]);
    batch.vertex(rect.x1, rect.y0);
    batch.vertex(rect.x0, rect.y0);
    batch.set_ink(inks[1]);
    batch.vertex(rect.x0, middle);
    batch.vertex(rect.x1, middle);
    batch.set_ink(inks[2]);
    batch.vertex(rect.x1, middle);
    batch.vertex(rect.x0, middle);
    batch.set_ink(inks[3]);
    batch.vertex(rect.x0, rect.y1);
    batch.vertex(rect.x1, rect.y1);
    batch.end_quads();
}

/// The two-ink band: the top ink from half a pixel in, the bottom ink below
/// the middle, inset half a pixel at each edge.
pub fn two_ink_band(batch: &mut SkinBatch, rect: GuiRect, top: SkinInk, bottom: SkinInk) {
    batch.begin();
    batch.set_ink(top);
    batch.vertex(rect.x1 - 0.5, rect.y0 + 0.5);
    batch.vertex(rect.x0, rect.y0 + 0.5);
    batch.set_ink(bottom);
    batch.vertex(rect.x0, rect.y1 - 0.5);
    batch.vertex(rect.x1, rect.y1 - 0.5);
    batch.end_quads();
}

/// A four-sided line frame with one ink per side: top, left, right, bottom.
pub fn four_ink_frame(batch: &mut SkinBatch, rect: GuiRect, inks: [SkinInk; 4]) {
    let x0 = rect.x0 + 0.5;
    let y0 = rect.y0 + 0.5;
    let x1 = rect.x1 - 0.5;
    let y1 = rect.y1 - 0.5;
    batch.begin();
    batch.set_ink(inks[0]);
    batch.vertex(x1 - 0.5, y0);
    batch.vertex(x0, y0);
    batch.set_ink(inks[1]);
    batch.vertex(x0, y0);
    batch.vertex(x0, y1);
    batch.set_ink(inks[3]);
    batch.vertex(x0, y1);
    batch.vertex(x1, y1);
    batch.set_ink(inks[2]);
    batch.vertex(x1, y1);
    batch.vertex(x1, y0 - 1.0);
    batch.end_lines();
}

/// A colour with a new alpha.
pub fn with_alpha(mut ink: SkinInk, alpha: u8) -> SkinInk {
    ink[3] = alpha;
    ink
}

/// The drop shadow ring: an outer edge at `alpha`, fading to clear at the
/// inset, with the inset clamped to half the height.
pub fn shadow_ring(batch: &mut SkinBatch, rect: GuiRect, inset: i32, alpha: u8) {
    let height = (rect.y1 - rect.y0) as i32;
    let mut inset = inset;
    if height / 2 < inset {
        inset = height / 2;
    }
    let inset = inset as f32;
    let dark = [0, 0, 0, alpha];
    let clear = [0, 0, 0, 0];
    ring(batch, rect, inset, dark, clear);
}

fn ring(batch: &mut SkinBatch, rect: GuiRect, inset: f32, outer: SkinInk, inner: SkinInk) {
    let x0 = rect.x0;
    let y0 = rect.y0;
    let x1 = rect.x1;
    let y1 = rect.y1;
    batch.begin();
    batch.set_ink(outer);
    batch.vertex(x0, y0);
    batch.vertex(x0, y1);
    batch.set_ink(inner);
    batch.vertex(x0 + inset, y1 - inset);
    batch.vertex(x0 + inset, y0 + inset);
    batch.vertex(x0 + inset, y0 + inset);
    batch.vertex(x1 - inset, y0 + inset);
    batch.set_ink(outer);
    batch.vertex(x1, y0);
    batch.vertex(x0, y0);
    batch.vertex(x1, y1);
    batch.vertex(x1, y0);
    batch.set_ink(inner);
    batch.vertex(x1 - inset, y0 + inset);
    batch.vertex(x1 - inset, y1 - inset);
    batch.vertex(x1 - inset, y1 - inset);
    batch.vertex(x0 + inset, y1 - inset);
    batch.set_ink(outer);
    batch.vertex(x0, y1);
    batch.vertex(x1, y1);
    batch.end_quads();
}

/// The fixed bevel ring: six-pixel bands with a 48-alpha outer edge.
pub fn bevel_ring(batch: &mut SkinBatch, rect: GuiRect) {
    ring(batch, rect, 6.0, [0, 0, 0, 0x30], [0, 0, 0, 0]);
}

/// The sunken ring of a control face: the bands alternate between the
/// highlight ink at alpha 96 and the same ink cleared, then an outline in
/// the next highlight ink.
pub fn sunken_ring(batch: &mut SkinBatch, rect: GuiRect, unit: i32, highlight: &SkinGroup) {
    let face = GuiRect {
        x0: rect.x0 + 1.0,
        y0: rect.y0 + 1.0,
        x1: rect.x1 - 1.0,
        y1: rect.y1 - 1.0,
    };
    let outer = with_alpha([highlight[8][0], highlight[8][1], highlight[8][2], 0], 96);
    let inner = [highlight[6][0], highlight[6][1], highlight[6][2], 0];
    ring(batch, face, unit as f32, outer, inner);
    batch.set_ink(highlight[7]);
    outline_rect(
        batch,
        GuiRect {
            x0: face.x0 + 1.0,
            y0: face.y0 + 1.0,
            x1: face.x1 - 1.0,
            y1: face.y1 - 1.0,
        },
    );
}

/// The vertical gradient with a clamped middle band: the top ink, the 0.65
/// and 0.5 mixes, the 0.35 mix and the bottom ink, with the two middle
/// boundaries held two units in from each end.
pub fn vertical_gradient(
    batch: &mut SkinBatch,
    rect: GuiRect,
    unit: i32,
    top: SkinInk,
    bottom: SkinInk,
) {
    let high = mix_ink(bottom, top, 0.65);
    let middle = mix_ink(bottom, top, 0.5);
    let low = mix_ink(bottom, top, 0.35);
    let span = 2.0 * unit as f32;
    let mid = (rect.y0 - rect.y1) * 0.5 + rect.y1;
    let upper = if mid > rect.y0 + span {
        rect.y0 + span
    } else {
        mid
    };
    let lower = if mid < rect.y1 - span {
        rect.y1 - span
    } else {
        mid
    };
    batch.begin();
    batch.set_ink(top);
    batch.vertex(rect.x1, rect.y0);
    batch.vertex(rect.x0, rect.y0);
    batch.set_ink(high);
    batch.vertex(rect.x0, upper);
    batch.vertex(rect.x1, upper);
    batch.set_ink(middle);
    batch.vertex(rect.x1, upper);
    batch.vertex(rect.x0, upper);
    batch.vertex(rect.x0, lower);
    batch.vertex(rect.x1, lower);
    batch.set_ink(low);
    batch.vertex(rect.x1, lower);
    batch.vertex(rect.x0, lower);
    batch.set_ink(bottom);
    batch.vertex(rect.x0, rect.y1);
    batch.vertex(rect.x1, rect.y1);
    batch.end_quads();
}

/// The line form of that gradient: the same stops drawn as two side rails
/// and a top and bottom edge.
pub fn vertical_gradient_frame(
    batch: &mut SkinBatch,
    rect: GuiRect,
    unit: i32,
    top: SkinInk,
    bottom: SkinInk,
) {
    let high = mix_ink(bottom, top, 0.65);
    let middle = mix_ink(bottom, top, 0.5);
    let low = mix_ink(bottom, top, 0.35);
    let span = 2.0 * unit as f32;
    let x0 = rect.x0 + 0.5;
    let y0 = rect.y0 + 0.5;
    let x1 = rect.x1 - 0.5;
    let y1 = rect.y1 - 0.5;
    let mid = (y0 - y1) * 0.5 + y1;
    let upper = if mid > y0 + span { y0 + span } else { mid };
    let lower = if mid < y1 - span { y1 - span } else { mid };
    batch.begin();
    batch.set_ink(top);
    batch.vertex(x1, y0);
    batch.vertex(x0, y0);
    batch.vertex(x0, y0);
    batch.set_ink(high);
    batch.vertex(x0, upper);
    batch.set_ink(middle);
    batch.vertex(x0, upper);
    batch.vertex(x0, lower);
    batch.set_ink(low);
    batch.vertex(x0, lower);
    batch.set_ink(bottom);
    batch.vertex(x0, y1);
    batch.vertex(x0, y1);
    batch.vertex(x1, y1);
    batch.vertex(x1, y1);
    batch.set_ink(low);
    batch.vertex(x1, lower);
    batch.set_ink(middle);
    batch.vertex(x1, lower);
    batch.vertex(x1, upper);
    batch.set_ink(high);
    batch.vertex(x1, upper);
    batch.set_ink(top);
    batch.vertex(x1, y0);
    batch.end_lines();
}

/// The two scroll-arrow strokes: six line pairs whose first four vertices
/// take the full ink and whose rest take two thirds of its alpha.
fn arrow_strokes(batch: &mut SkinBatch, points: [(f32, f32); 12], ink: SkinInk) {
    for (index, (x, y)) in points.into_iter().enumerate() {
        if index == 4 {
            batch.set_ink(with_alpha(ink, (ink[3] / 3) * 2));
        }
        batch.vertex(x, y);
    }
    batch.end_lines();
}

/// A horizontal scroll arrow: a stroke from the left point to the middle,
/// then its faded lower copy.
pub fn scroll_arrow_horizontal(batch: &mut SkinBatch, x: f32, y: f32, w: f32, ink: SkinInk) {
    let mid = ((w as i32 + 1) as u32 >> 1) as f32;
    batch.begin();
    batch.set_ink(ink);
    arrow_strokes(
        batch,
        [
            (x, y + mid),
            (x + w * 0.5 - 0.5, y + 1.0),
            (x + w, y + mid),
            (x + w * 0.5, y),
            (x, y + mid + 1.0),
            (x + w * 0.5 + 0.5, y + 1.0),
            (x + w, y + mid + 1.0),
            (x + w * 0.5 + 0.5, y + 2.0),
            (x, y + mid - 1.0),
            (x + w * 0.5 + 0.5, y - 1.0),
            (x + w, y + mid - 1.0),
            (x + w * 0.5 + 0.5, y),
        ],
        ink,
    );
}

/// A vertical scroll arrow, turned a quarter from the horizontal one.
pub fn scroll_arrow_vertical(batch: &mut SkinBatch, x: f32, y: f32, w: f32, ink: SkinInk) {
    let mid = ((w as i32 + 1) as u32 >> 1) as f32;
    batch.begin();
    batch.set_ink(ink);
    arrow_strokes(
        batch,
        [
            (x, y),
            (x + w * 0.5, y + mid),
            (x + w, y),
            (x + w * 0.5 - 0.5, y + mid + 0.5),
            (x, y + 1.0),
            (x + w * 0.5, y + mid + 1.0),
            (x + w, y + 1.0),
            (x + w * 0.5 - 0.5, y + mid + 0.5),
            (x, y - 1.0),
            (x + w * 0.5, y + mid - 1.0),
            (x + w, y - 1.0),
            (x + w * 0.5 - 0.5, y + mid - 0.5),
        ],
        ink,
    );
}

/// A control face's inks come from the palette's control group; a hot face
/// uses the copy of the form group.
pub fn draw_button_face(
    batch: &mut SkinBatch,
    rect: GuiRect,
    pressed: bool,
    hot: bool,
    palette: &SkinPalette,
) {
    let inner = GuiRect {
        x0: rect.x0 + 2.0,
        y0: rect.y0 + 2.0,
        x1: rect.x1 - 2.0,
        y1: rect.y1 - 2.0,
    };
    let border = GuiRect {
        x0: rect.x0 + 1.0,
        y0: rect.y0 + 1.0,
        x1: rect.x1 - 1.0,
        y1: rect.y1 - 1.0,
    };
    if pressed {
        pair_rect(batch, inner, palette.control[4], palette.control[1]);
        bordered_rect(batch, border, palette.control[6], palette.control[4]);
        batch.set_ink(palette.control[0]);
    } else if hot {
        pair_rect(batch, inner, palette.form_hot[1], palette.form_hot[6]);
        bordered_rect(batch, border, palette.form_hot[4], palette.form_hot[6]);
        batch.set_ink(palette.form_hot[0]);
    } else {
        pair_rect(batch, inner, palette.control[2], palette.control[4]);
        bordered_rect(batch, border, palette.control[5], palette.control[7]);
        batch.set_ink(palette.control[0]);
    }
    outline_rect(batch, rect);
}

/// The gradient form of the control face.
pub fn draw_gradient_button_face(
    batch: &mut SkinBatch,
    rect: GuiRect,
    pressed: bool,
    hot: bool,
    palette: &SkinPalette,
) {
    let inner = GuiRect {
        x0: rect.x0 + 2.0,
        y0: rect.y0 + 2.0,
        x1: rect.x1 - 2.0,
        y1: rect.y1 - 2.0,
    };
    let border = GuiRect {
        x0: rect.x0 + 1.0,
        y0: rect.y0 + 1.0,
        x1: rect.x1 - 1.0,
        y1: rect.y1 - 1.0,
    };
    if pressed {
        gradient_box(batch, inner, palette.control[4], palette.control[0]);
        bevel_rect(batch, border, palette.control[6], palette.control[4]);
        batch.set_ink(palette.control[0]);
    } else if hot {
        gradient_box(batch, inner, palette.form_hot[2], palette.form_hot[4]);
        bevel_rect(batch, border, palette.form_hot[5], palette.form_hot[4]);
        batch.set_ink(palette.form_hot[0]);
    } else {
        gradient_box(batch, inner, palette.control[2], palette.control[5]);
        bevel_rect(batch, border, palette.control[5], palette.control[6]);
        batch.set_ink(palette.control[0]);
    }
    outline_rect(batch, rect);
}

/// The raised or sunken box a control's mark sits in.
pub fn draw_button_box(
    batch: &mut SkinBatch,
    rect: GuiRect,
    pressed: bool,
    hot: bool,
    palette: &SkinPalette,
) {
    let inner = GuiRect {
        x0: rect.x0 + 2.0,
        y0: rect.y0 + 2.0,
        x1: rect.x1 - 2.0,
        y1: rect.y1 - 2.0,
    };
    let border = GuiRect {
        x0: rect.x0 + 1.0,
        y0: rect.y0 + 1.0,
        x1: rect.x1 - 1.0,
        y1: rect.y1 - 1.0,
    };
    if pressed {
        gradient_box(batch, inner, palette.control[4], palette.control[1]);
        bevel_rect(batch, border, palette.control[6], palette.control[4]);
        batch.set_ink(palette.control[0]);
    } else if hot {
        gradient_box(batch, inner, palette.form[3], palette.form[4]);
        bevel_rect(batch, border, palette.form[4], palette.form[5]);
        batch.set_ink(palette.form_hot[0]);
    } else {
        gradient_box(batch, inner, palette.control[2], palette.control[6]);
        bevel_rect(batch, border, palette.control[5], palette.control[7]);
        batch.set_ink(palette.control[0]);
    }
    outline_rect(batch, rect);
}

/// The scroll region's frame and the panel's inset bevel and shadow.
pub fn draw_panel(
    batch: &mut SkinBatch,
    rect: GuiRect,
    inset: i32,
    state: u32,
    unit: i32,
    palette: &SkinPalette,
) {
    let inset = inset as f32;
    let inner = GuiRect {
        x0: rect.x0 + inset,
        y0: rect.y0 + inset,
        x1: rect.x1 - inset,
        y1: rect.y1 - inset,
    };
    if (state & 0x40) == 0x40 {
        let box_rect = GuiRect {
            x0: inner.x0 + 2.0,
            y0: inner.y0 + 2.0,
            x1: inner.x1 - 2.0,
            y1: inner.y1 - 2.0,
        };
        pair_rect(batch, box_rect, palette.control[2], palette.control[5]);
        let border = GuiRect {
            x0: inner.x0 + 1.0,
            y0: inner.y0 + 1.0,
            x1: inner.x1 - 1.0,
            y1: inner.y1 - 1.0,
        };
        bordered_rect(batch, border, palette.control[4], palette.control[6]);
        batch.set_ink(palette.control[0]);
        outline_rect(batch, inner);
        shadow_ring(
            batch,
            GuiRect {
                x0: inner.x0 + 1.0,
                y0: inner.y0 + 1.0,
                x1: inner.x1 - 1.0,
                y1: inner.y1 - 1.0,
            },
            unit,
            0xa0,
        );
    } else {
        draw_frame_control(batch, inner, false, (state & 0x10) == 0x10, palette);
    }
}

/// The frame slot's draw: a plain, hot or pressed box around the rectangle.
pub fn draw_frame_control(
    batch: &mut SkinBatch,
    rect: GuiRect,
    pressed: bool,
    hot: bool,
    palette: &SkinPalette,
) {
    draw_button_face(batch, rect, pressed, hot, palette);
}

/// A control's background fill in its own colour, which arrives with its
/// alpha forced opaque.
pub fn draw_control_fill(batch: &mut SkinBatch, rect: GuiRect, ink: SkinInk) {
    batch.select(1, 0, false);
    batch.set_ink(with_alpha(ink, 0xff));
    solid_rect(batch, rect);
    batch.select(0x10, 0, false);
}

/// A control's frame and, when it is pressed, the fixed bevel ring inside.
pub fn draw_control_box(
    batch: &mut SkinBatch,
    rect: GuiRect,
    inset: f32,
    state: u32,
    palette: &SkinPalette,
) {
    let inner = GuiRect {
        x0: rect.x0 + inset,
        y0: rect.y0 + inset,
        x1: rect.x1 - inset,
        y1: rect.y1 - inset,
    };
    draw_frame_control(
        batch,
        inner,
        (state & 0x40) == 0x40,
        (state & 0x10) == 0x10,
        palette,
    );
    if (state & 0x40) == 0x40 {
        bevel_ring(batch, inner);
    }
}

/// The frame's style switch: each of the two style fields independently
/// selects a double frame, the second one inset by a pixel.
pub fn draw_style_frames(batch: &mut SkinBatch, rect: GuiRect, style: u32, palette: &SkinPalette) {
    match style & 0x30 {
        0x10 => double_frame(batch, rect, palette.form_hot[0], palette.control[7]),
        0x20 => double_frame(batch, rect, palette.control[6], palette.form_hot[0]),
        _ => {}
    }
    let inset = GuiRect {
        x0: rect.x0 + 1.0,
        y0: rect.y0 + 1.0,
        x1: rect.x1 - 1.0,
        y1: rect.y1 - 1.0,
    };
    match style & 0xc0 {
        0x40 => double_frame(batch, inset, palette.form_hot[0], palette.control[7]),
        0x80 => double_frame(batch, inset, palette.control[6], palette.form_hot[0]),
        _ => {}
    }
}

/// The mark box when it is not selected: a bevel just outside the rect.
pub fn draw_mark_plain(batch: &mut SkinBatch, rect: GuiRect, palette: &SkinPalette) {
    bevel_rect(
        batch,
        GuiRect {
            x0: rect.x0 - 1.0,
            y0: rect.y0 - 1.0,
            x1: rect.x1 + 1.0,
            y1: rect.y1 + 1.0,
        },
        palette.control[6],
        palette.control[5],
    );
    batch.set_ink(palette.control[0]);
    outline_rect(batch, rect);
}

/// The mark box when it is selected: two bevelled rings and a faded face.
pub fn draw_mark_pressed(batch: &mut SkinBatch, rect: GuiRect, palette: &SkinPalette) {
    bevel_rect(
        batch,
        GuiRect {
            x0: rect.x0 - 1.0,
            y0: rect.y0 - 1.0,
            x1: rect.x1 + 1.0,
            y1: rect.y1 + 1.0,
        },
        palette.control[6],
        palette.control[5],
    );
    bevel_rect(
        batch,
        GuiRect {
            x0: rect.x0 + 1.0,
            y0: rect.y0 + 1.0,
            x1: rect.x1 - 1.0,
            y1: rect.y1 - 1.0,
        },
        palette.control[7],
        palette.control[6],
    );
    batch.set_ink(palette.control[0]);
    outline_rect(batch, rect);
    fade_rect(
        batch,
        GuiRect {
            x0: rect.x0 + 1.0,
            y0: rect.y0 + 1.0,
            x1: rect.x1 - 1.0,
            y1: rect.y1 - 1.0,
        },
        palette.form[3],
        palette.form[2],
    );
}

/// Draws a mark at the rectangle its code names. A selected mark takes the
/// pressed box and the faded face; the active mark takes the plain box.
pub fn draw_mark(
    batch: &mut SkinBatch,
    rect: GuiRect,
    selected: bool,
    active: bool,
    palette: &SkinPalette,
) {
    if selected {
        draw_mark_pressed(batch, rect, palette);
    } else if active {
        draw_mark_plain(batch, rect, palette);
    }
}

/// The accent gradient bar: the dim accent, its two mixes and the bright
/// accent, from top to bottom.
pub fn draw_accent_bar(batch: &mut SkinBatch, rect: GuiRect, palette: &SkinPalette) {
    four_stop_rect(
        batch,
        rect,
        [
            palette.accent_dim,
            palette.accent_mix[0],
            palette.accent_mix[1],
            palette.accent_bright,
        ],
    );
}

/// Whether a mark's code is the active one, or the hot one while the form
/// is not hot.
pub fn mark_active(code: i32, active_code: i32, hot_flag: bool, hot_code: i32) -> bool {
    code == active_code || (code == hot_code && !hot_flag)
}

/// A mark's icon draw: the place it starts from, the icon, its ink and its
/// scale. Icons draw upward from the point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkIcon {
    pub x: f32,
    pub y: f32,
    pub id: u32,
    pub ink: Option<SkinInk>,
    pub scale: Option<f32>,
}

/// Places a mark's icon at the bottom-left of its rectangle. The selected
/// mark draws at scale 1; the others bind an ink first.
pub fn draw_mark_icon(
    rect: GuiRect,
    unit: i32,
    id: u32,
    selected: bool,
    palette: &SkinPalette,
) -> MarkIcon {
    let x = rect.x0 + (unit >> 1) as f32;
    let y = rect.y1;
    if selected {
        MarkIcon {
            x,
            y,
            id,
            ink: None,
            scale: Some(1.0),
        }
    } else {
        MarkIcon {
            x,
            y,
            id,
            ink: Some(palette.text[6]),
            scale: None,
        }
    }
}

/// The gradient form of a rectangle: top ink, its 0.6 mix towards the
/// bottom ink, the bottom ink itself, then its 0.4 mix back towards the top.
pub fn gradient_box(batch: &mut SkinBatch, rect: GuiRect, bottom: SkinInk, top: SkinInk) {
    let high = mix_ink(bottom, top, 0.6);
    let low = mix_ink(bottom, top, 0.4);
    let middle = (rect.y0 - rect.y1) * 0.5 + rect.y1;
    batch.begin();
    batch.set_ink(top);
    batch.vertex(rect.x1, rect.y0);
    batch.vertex(rect.x0, rect.y0);
    batch.set_ink(high);
    batch.vertex(rect.x0, middle);
    batch.vertex(rect.x1, middle);
    batch.set_ink(bottom);
    batch.vertex(rect.x1, middle);
    batch.vertex(rect.x0, middle);
    batch.set_ink(low);
    batch.vertex(rect.x0, rect.y1);
    batch.vertex(rect.x1, rect.y1);
    batch.end_quads();
}

/// The bevel form of a rectangle: two inks fading between each other across
/// the left and right sides, with the top edge at the top ink and the bottom
/// edge at the bottom ink.
pub fn bevel_rect(batch: &mut SkinBatch, rect: GuiRect, bottom: SkinInk, top: SkinInk) {
    let high = mix_ink(bottom, top, 0.65);
    let low = mix_ink(bottom, top, 0.35);
    let x0 = rect.x0 + 0.5;
    let y0 = rect.y0 + 0.5;
    let x1 = rect.x1 - 0.5;
    let y1 = rect.y1 - 0.5;
    let middle = (y0 - y1) * 0.5 + y1 + 0.5;
    batch.begin();
    batch.set_ink(top);
    batch.vertex(x1, y0);
    batch.vertex(x0, y0);
    batch.vertex(x0, y0);
    batch.set_ink(high);
    batch.vertex(x0, middle);
    batch.set_ink(low);
    batch.vertex(x0, middle);
    batch.set_ink(bottom);
    batch.vertex(x0, y1);
    batch.vertex(x0, y1);
    batch.vertex(x1, y1);
    batch.vertex(x1, y1);
    batch.set_ink(low);
    batch.vertex(x1, middle);
    batch.set_ink(high);
    batch.vertex(x1, middle);
    batch.set_ink(top);
    batch.vertex(x1, y0 - 1.0);
    batch.end_lines();
}

/// The bar's two forms: the line gradient when idle, the gradient boxes
/// when pressed or hot.
pub fn draw_bar_with_boxes(
    batch: &mut SkinBatch,
    rect: GuiRect,
    unit: i32,
    pressed: bool,
    palette: &SkinPalette,
) {
    let inner = GuiRect {
        x0: rect.x0 + 1.0,
        y0: rect.y0 + 1.0,
        x1: rect.x1 - 1.0,
        y1: rect.y1 - 1.0,
    };
    if pressed {
        gradient_box(
            batch,
            GuiRect {
                x0: rect.x0 + 2.0,
                y0: rect.y0 + 2.0,
                x1: rect.x1 - 2.0,
                y1: rect.y1 - 2.0,
            },
            palette.control[4],
            palette.control[0],
        );
        bevel_rect(batch, inner, palette.control[6], palette.control[4]);
    } else {
        vertical_gradient(batch, inner, unit, palette.control[5], palette.control[1]);
        vertical_gradient_frame(batch, inner, unit, palette.control[6], palette.control[4]);
    }
    batch.set_ink(palette.control[0]);
    outline_rect(batch, rect);
}

/// A slider's draw: the track and its thumb, or the striped fill and the
/// unit-wide bar at the value.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinControlSlider {
    pub shift: i32,
    pub frame: Option<GuiRect>,
    pub stripe: bool,
    pub translate: [f32; 2],
    pub begin: u32,
    pub end: u32,
}

/// Draws the slider at `fraction`. A scroll mode fills the track with the
/// striped record; the plain mode draws the thumb frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlSliderPlace {
    pub rect: GuiRect,
    pub inset: i32,
    pub thumb: i32,
    pub state: u32,
    pub style: u32,
    pub fraction: f32,
}

pub fn draw_control_slider(
    batch: &mut SkinBatch,
    place: ControlSliderPlace,
    stripes: &[SkinInk],
    palette: &SkinPalette,
) -> SkinControlSlider {
    let rect = place.rect;
    let thumb = place.thumb;
    let state = place.state;
    let fraction = place.fraction;
    let mode = place.style & 0x3000_0000;
    let inset = place.inset as f32;
    let inner = GuiRect {
        x0: rect.x0 + inset,
        y0: rect.y0 + inset,
        x1: rect.x1 - inset,
        y1: rect.y1 - inset,
    };
    if mode == 0 {
        faded_rect(
            batch,
            GuiRect {
                x0: rect.x0 + 2.0,
                y0: rect.y0 + 2.0,
                x1: rect.x1 - 2.0,
                y1: rect.y1 - 2.0,
            },
            palette.control[4],
            palette.form_hot[0],
            0.6,
        );
        let spare = (rect.x1 - rect.x0 - inset * 2.0 - thumb as f32) * fraction;
        let shift = spare.round_ties_even() as i32;
        batch.add_texture_shift(-(shift as f32), 0.0);
        let frame = GuiRect {
            x0: rect.x0 + inset + shift as f32,
            y0: rect.y0 + inset,
            x1: rect.x0 + inset + shift as f32 + thumb as f32,
            y1: rect.y1 - inset,
        };
        draw_frame_control(batch, frame, false, (state & 0x10) == 0x10, palette);
        batch.add_texture_shift(shift as f32, 0.0);
        SkinControlSlider {
            shift,
            frame: Some(frame),
            stripe: false,
            translate: batch.texture_shift,
            begin: 0,
            end: 0,
        }
    } else if mode == 0x1000_0000 {
        batch.select(1, 0, false);
        stripe_bar(
            batch,
            GuiRect {
                x0: inner.x0 + 1.0,
                y0: inner.y0 + 1.0,
                x1: inner.x1 - 1.0,
                y1: inner.y1 - 1.0,
            },
            stripes,
        );
        let spare = (rect.x1 - rect.x0 - inset * 2.0 - 4.0) * fraction;
        let shift = spare.round_ties_even() as i32;
        batch.set_ink(palette.text[5]);
        outline_rect(
            batch,
            GuiRect {
                x0: rect.x0 + inset + shift as f32,
                y0: rect.y0,
                x1: rect.x0 + inset + shift as f32 + 4.0,
                y1: rect.y1,
            },
        );
        batch.select(0x10, 0, false);
        SkinControlSlider {
            shift,
            frame: None,
            stripe: true,
            translate: batch.texture_shift,
            begin: 0,
            end: 0,
        }
    } else {
        SkinControlSlider {
            shift: 0,
            frame: None,
            stripe: false,
            translate: batch.texture_shift,
            begin: 0,
            end: 0,
        }
    }
}

/// The scroll bar's draw: a box with a thumb at the end, or a striped fill
/// with the unit bar. The step it returns is the unit rounded up to an odd
/// value.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinScrollBar {
    pub step: i32,
    pub shift: i32,
    pub box_rect: Option<GuiRect>,
    pub thumb: Option<GuiRect>,
    pub stripe: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlScrollPlace {
    pub rect: GuiRect,
    pub inset: i32,
    pub state: u32,
    pub style: u32,
    pub fraction: f32,
    pub unit: i32,
}

pub fn draw_control_scrollbar(
    batch: &mut SkinBatch,
    place: ControlScrollPlace,
    stripes: &[SkinInk],
    palette: &SkinPalette,
) -> SkinScrollBar {
    let rect = place.rect;
    let state = place.state;
    let fraction = place.fraction;
    let unit = place.unit;
    let mode = place.style & 0x3000_0000;
    let inset = place.inset as f32;
    if mode == 0 {
        let bar = ((unit * 4) | 1) as f32;
        let trough = GuiRect {
            x0: rect.x0 + 1.0,
            y0: rect.y0 + 1.0,
            x1: rect.x1 - 1.0,
            y1: rect.y1 - 1.0,
        };
        let thumb = GuiRect {
            x0: rect.x1 - bar,
            y0: rect.y0 + 2.0,
            x1: rect.x1 - 2.0,
            y1: rect.y1 - 2.0,
        };
        // The subclass' box slot is not in this component; the base skin's
        // raised box is its equivalent (guess).
        draw_button_box(batch, trough, false, false, palette);
        draw_frame_control(
            batch,
            thumb,
            (state & 0x40) == 0x40,
            (state & 0x10) == 0x10,
            palette,
        );
        SkinScrollBar {
            step: (unit + 1) - (unit & 1),
            shift: 0,
            box_rect: Some(trough),
            thumb: Some(thumb),
            stripe: false,
        }
    } else if mode == 0x2000_0000 {
        batch.select(1, 0, false);
        stripe_bar(
            batch,
            GuiRect {
                x0: rect.x0 + inset + 1.0,
                y0: rect.y0 + inset + 1.0,
                x1: rect.x1 - inset - 1.0,
                y1: rect.y1 - inset - 1.0,
            },
            stripes,
        );
        let spare = (rect.x1 - rect.x0 - inset * 2.0 - unit as f32) * fraction;
        let shift = spare.round_ties_even() as i32;
        batch.set_ink(palette.text[5]);
        outline_rect(
            batch,
            GuiRect {
                x0: rect.x0 + inset + shift as f32,
                y0: rect.y0,
                x1: rect.x0 + inset + shift as f32 + unit as f32,
                y1: rect.y1,
            },
        );
        batch.select(0x10, 0, false);
        SkinScrollBar {
            step: (unit + 1) - (unit & 1),
            shift,
            box_rect: None,
            thumb: None,
            stripe: true,
        }
    } else {
        SkinScrollBar {
            step: (unit + 1) - (unit & 1),
            shift: 0,
            box_rect: None,
            thumb: None,
            stripe: false,
        }
    }
}

/// A control's sunken or raised shell: its frame, optional shadow and the
/// sunken face.
pub fn draw_control_shell(
    batch: &mut SkinBatch,
    rect: GuiRect,
    inset: i32,
    style: u32,
    state: u32,
    unit: i32,
    palette: &SkinPalette,
) {
    let inset = inset as f32;
    let inner = GuiRect {
        x0: rect.x0 + inset,
        y0: rect.y0 + inset,
        x1: rect.x1 - inset,
        y1: rect.y1 - inset,
    };
    draw_frame_control(
        batch,
        inner,
        (state & 0x40) == 0x40,
        (state & 0x10) == 0x10,
        palette,
    );
    if (style & 0x2000_0000) != 0 && (state & 0x40) == 0 {
        shadow_ring(batch, rect, -unit / 2, 0x50);
    }
    if ((style & 0x4000_0000) != 0 && (state & 0x40) != 0) || (state & 0x80) != 0 {
        sunken_ring(batch, inner, unit, &palette.highlight);
    }
}

/// Fades a label in from its start clock: the first 300 ms scale its alpha,
/// and once the fade is complete the label draws normally and clears its
/// fade bit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FadedLabel {
    pub ink: SkinInk,
    pub y: i32,
    pub calls: u32,
    pub fade: f32,
    pub finished: bool,
    pub clear_bit: bool,
}

/// Draws a label fading in over a third of a second.
pub fn fade_label(start: u32, now: u32, rect_y: i32, ink: SkinInk) -> FadedLabel {
    let fade = now.wrapping_sub(start) as f32 * 0.003_333_333_5;
    if fade <= 1.0 {
        FadedLabel {
            ink: with_alpha(ink, rounded_byte(f32::from(ink[3]) * fade)),
            y: rect_y,
            calls: 1,
            fade,
            finished: false,
            clear_bit: false,
        }
    } else {
        FadedLabel {
            ink,
            y: 0,
            calls: 1,
            fade,
            finished: true,
            clear_bit: true,
        }
    }
}

/// A line box whose top and left sides take the first ink and whose bottom
/// and right sides take the second, with the last point half a pixel up.
pub fn two_ink_outline(
    batch: &mut SkinBatch,
    rect: GuiRect,
    top_left: SkinInk,
    bottom_right: SkinInk,
) {
    let x0 = rect.x0 + 0.5;
    let y0 = rect.y0 + 0.5;
    let x1 = rect.x1 - 0.5;
    let y1 = rect.y1 - 0.5;
    batch.begin();
    batch.set_ink(top_left);
    batch.vertex(x1 - 0.5, y0);
    batch.vertex(x0, y0);
    batch.vertex(x0, y0);
    batch.vertex(x0, y1);
    batch.set_ink(bottom_right);
    batch.vertex(x0, y1);
    batch.vertex(x1, y1);
    batch.vertex(x1, y1);
    batch.vertex(x1, y0 - 1.0);
    batch.end_lines();
}

/// The form fields a control-code rectangle reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormRectPlace {
    pub width: i32,
    pub height: i32,
    pub unit: i32,
    pub inset: i32,
    pub client_top: i32,
    pub right: i32,
    pub bottom: i32,
    pub flags: u32,
}

/// A form's frame rectangle for a close or rail code. Codes outside the
/// switch leave the rectangle untouched.
pub fn form_rect(place: FormRectPlace, code: i32, rect: &mut [i32; 4]) -> bool {
    let control = ControlPlace {
        x: place.width,
        y: place.height,
        size: place.inset,
        field_a: place.client_top,
        field_b: place.right,
        flags: place.flags,
    };
    match control_rect(
        control,
        code,
        SkinMetrics {
            unit: place.unit,
            trim: 0,
        },
    ) {
        Some(found) => {
            *rect = found;
            true
        }
        None => false,
    }
}

/// The fields a form's chrome reads. The client rectangle's four edges are
/// kept separately, as the original stores them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormChromePlace {
    pub rect: GuiRect,
    pub frame_style: u32,
    pub flags: u32,
    pub state: u32,
    pub margin: f32,
    pub bar: f32,
    pub client_left: f32,
    pub client_top: f32,
    pub client_right: f32,
    pub client_bottom: f32,
    /// The form's own fill colour, or the base palette's shade 3.
    pub fill: Option<SkinInk>,
    pub flat: bool,
}

/// The chrome's geometry: the gutter quads, the fill and the frames.
#[derive(Debug, Clone, PartialEq)]
pub struct FormChrome {
    pub gutters: Vec<[(f32, f32); 4]>,
    pub fill: Option<[(f32, f32); 4]>,
    pub fills: Vec<[(f32, f32); 4]>,
    pub frames: u32,
    pub begin_calls: u32,
    pub end_calls: u32,
    pub mode_calls: u32,
}

/// Draws a form's frame: the scrollbar gutters, the client fill and the
/// double frames its style word selects.
pub fn draw_form_chrome(
    batch: &mut SkinBatch,
    place: FormChromePlace,
    palette: &SkinPalette,
) -> FormChrome {
    let mut chrome = FormChrome {
        gutters: Vec::new(),
        fill: None,
        fills: Vec::new(),
        frames: 0,
        begin_calls: 0,
        end_calls: 0,
        mode_calls: 0,
    };
    let begin_before = batch.begin_calls;
    let end_before = batch.end_calls;
    let mode_before = batch.mode_calls;
    if (place.state & 1) == 1 {
        match place.frame_style & 0x30 {
            0x10 => {
                if (place.state & 0x40) == 0x40 {
                    batch.set_ink(palette.form[5]);
                    solid_rect(batch, place.rect);
                } else {
                    double_frame(batch, place.rect, palette.form[0], palette.form[5]);
                }
                chrome.frames += 1;
            }
            0x20 => {
                two_ink_outline(batch, place.rect, palette.form[0], palette.form[6]);
                chrome.frames += 1;
            }
            _ => {}
        }
    } else {
        if (place.flags & 0x2_0000) != 0 && (place.state & 4) == 0 {
            let m = place.margin;
            let bar = place.bar;
            let top = place.client_top;
            let left = place.client_left;
            let right = place.client_right;
            let bottom = place.client_bottom;
            let w = place.rect.x1;
            let h = place.rect.y1;
            batch.begin();
            if m + bar < top {
                batch.set_ink(palette.form[3]);
                batch.vertex(m, m);
                batch.vertex(m, top - bar);
                batch.vertex(w - m, top - bar);
                batch.vertex(w - m, m);
            }
            if m + bar < left {
                batch.vertex(m, top - bar);
                batch.vertex(m, h - m);
                batch.vertex(left - bar, h - m);
                batch.vertex(left - bar, top - bar);
            }
            if right < (w - m) - bar {
                batch.vertex(right + bar, top - bar);
                batch.vertex(right + bar, h - m);
                batch.vertex(w - m, h - m);
                batch.vertex(w - m, top - bar);
            }
            if bottom < (h - m) - bar {
                batch.vertex(left - bar, bottom + bar);
                batch.vertex(left - bar, h - m);
                batch.vertex(right + bar, h - m);
                batch.vertex(right + bar, bottom + bar);
            }
            batch.end_quads();
            if let Some(primitive) = batch.primitives.last() {
                chrome.gutters = primitive
                    .vertices
                    .chunks(4)
                    .map(|quad| {
                        [
                            (quad[0].x, quad[0].y),
                            (quad[1].x, quad[1].y),
                            (quad[2].x, quad[2].y),
                            (quad[3].x, quad[3].y),
                        ]
                    })
                    .collect();
            }
        }
        if (place.state & 0x20) != 0 && place.flat {
            batch.select(0x10, 0, false);
            chrome.mode_calls += 1;
            batch.set_ink(place.fill.unwrap_or(palette.form[3]));
            let corners = [
                (place.client_left, place.client_top),
                (place.client_left, place.client_bottom),
                (place.client_right, place.client_bottom),
                (place.client_right, place.client_top),
            ];
            batch.begin();
            for (x, y) in corners {
                batch.vertex(x, y);
            }
            batch.end_quads();
            chrome.fill = Some(corners);
            chrome.fills.push(corners);
        }
        match place.frame_style & 0x30 {
            0x10 => {
                if (place.state & 0x40) == 0x40 {
                    batch.set_ink(palette.form[5]);
                    solid_rect(
                        batch,
                        GuiRect {
                            x0: place.rect.x0 + 1.0,
                            y0: place.rect.y0 + 1.0,
                            x1: place.rect.x1 - 1.0,
                            y1: place.rect.y1 - 1.0,
                        },
                    );
                } else {
                    double_frame(batch, place.rect, palette.form[0], palette.form[5]);
                }
                chrome.frames += 1;
            }
            0x20 => {
                double_frame(batch, place.rect, palette.form[5], palette.form[0]);
                chrome.frames += 1;
            }
            _ => {}
        }
        let grown = GuiRect {
            x0: place.rect.x0 - 2.0,
            y0: place.rect.y0 - 2.0,
            x1: place.rect.x1 + 2.0,
            y1: place.rect.y1 + 2.0,
        };
        match place.frame_style & 0xc0 {
            0x40 => {
                double_frame(batch, grown, palette.form[0], palette.form[5]);
                chrome.frames += 1;
            }
            0x80 => {
                double_frame(batch, grown, palette.form[5], palette.form[0]);
                chrome.frames += 1;
            }
            _ => {}
        }
    }
    chrome.begin_calls = batch.begin_calls - begin_before;
    chrome.end_calls = batch.end_calls - end_before;
    chrome.mode_calls += batch.mode_calls - mode_before;
    chrome
}

/// A form's scrollbar: where its thumb starts and how long it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormScroll {
    pub position: i32,
    pub length: i32,
}

/// The form background's inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct FormBackgroundPlace {
    pub rect: GuiRect,
    pub inset: i32,
    pub client_top: i32,
    pub client_right: i32,
    pub client_bottom: i32,
    pub state: u32,
    pub flags: u32,
    pub frame_style: u32,
    pub margin: f32,
    pub bar: f32,
    pub vertical: Option<FormScroll>,
    pub horizontal: Option<FormScroll>,
    pub title_width: i32,
    pub title_x: i32,
    pub title_y: i32,
    pub title_style: u32,
    pub title_offset: i32,
    pub marks: Vec<(i32, u32, bool)>,
    pub active_code: i32,
    pub hot_code: i32,
    pub hot_flag: bool,
}

/// A form background's structure in draw order.
#[derive(Debug, Clone, PartialEq)]
pub struct FormBackground {
    pub rounded: bool,
    pub chrome: bool,
    pub title_bar: bool,
    pub title_band: bool,
    pub mark_rects: Vec<[i32; 4]>,
    pub mark_icons: Vec<MarkIcon>,
    pub scroll_frames: Vec<[i32; 4]>,
    pub scroll_shadows: Vec<[i32; 4]>,
    pub scroll_bars: Vec<[i32; 4]>,
    pub arrow_icons: Vec<MarkIcon>,
    pub title: Option<[[i32; 3]; 2]>,
    pub begin_calls: u32,
}

/// Draws a form's default background: the rounded shell, the chrome, the
/// title bar, the scrollbars with their arrows, and the title text blocks.
pub fn draw_form_background(
    batch: &mut SkinBatch,
    place: FormBackgroundPlace,
    unit: i32,
    palette: &SkinPalette,
) -> FormBackground {
    let mut drawn = FormBackground {
        rounded: false,
        chrome: false,
        title_bar: false,
        title_band: false,
        mark_rects: Vec::new(),
        mark_icons: Vec::new(),
        scroll_frames: Vec::new(),
        scroll_shadows: Vec::new(),
        scroll_bars: Vec::new(),
        arrow_icons: Vec::new(),
        title: None,
        begin_calls: 0,
    };
    let start = batch.primitives.len();
    if (place.state & 0x40) == 0x40 {
        rounded_textured_frame(
            batch,
            GuiRect {
                x0: 1.0,
                y0: 1.0,
                x1: place.rect.x1 - 1.0,
                y1: place.rect.y1 - 1.0,
            },
            unit,
            4,
            120,
        );
        drawn.rounded = true;
    }
    batch.select(0x10, 0, false);
    draw_form_chrome(
        batch,
        FormChromePlace {
            rect: place.rect,
            frame_style: place.frame_style,
            flags: place.flags,
            state: place.state,
            margin: place.margin,
            bar: place.bar,
            client_left: place.inset as f32,
            client_top: place.client_top as f32,
            client_right: place.client_right as f32,
            client_bottom: place.client_bottom as f32,
            fill: None,
            flat: false,
        },
        palette,
    );
    drawn.chrome = true;
    if (place.state & 1) == 1 {
        let width = place.rect.x1;
        let title = GuiRect::corners(2.0, 2.0, width - 2.0, 24.0);
        fade_rect(batch, title, palette.form[3], palette.form[2]);
        drawn.title_bar = true;
        if (place.state & 0x800030) == 0x20 && (place.frame_style & 0x80) == 0 {
            two_ink_band(
                batch,
                GuiRect::corners(2.0, 24.0, width - 2.0, 26.0),
                palette.form[1],
                palette.form[5],
            );
            drawn.title_band = true;
        }
        let mut rect = [0; 4];
        for (code, id, selected) in place.marks.clone() {
            if form_rect(
                FormRectPlace {
                    width: place.rect.x1 as i32,
                    height: place.rect.y1 as i32,
                    unit,
                    inset: place.inset,
                    client_top: place.client_top,
                    right: place.client_right,
                    bottom: place.client_bottom,
                    flags: place.state,
                },
                code,
                &mut rect,
            ) {
                drawn.mark_rects.push(rect);
                let hot = selected || (code == place.hot_code && place.hot_flag);
                let active = code == place.active_code;
                let shape = GuiRect::corners(
                    rect[0] as f32,
                    rect[1] as f32,
                    rect[2] as f32,
                    rect[3] as f32,
                );
                draw_mark(batch, shape, hot, active, palette);
                drawn.mark_icons.push(draw_mark_icon(
                    shape,
                    unit,
                    id,
                    mark_active(code, place.active_code, place.hot_flag, place.hot_code),
                    palette,
                ));
            }
        }
    }
    batch.select(2, 0, false);
    for (code, id, vertical) in [
        (-9, 4, true),
        (-6, 5, true),
        (-13, 2, false),
        (-14, 3, false),
    ] {
        let enabled = if vertical {
            (place.state & 0x200) != 0
        } else {
            (place.state & 0x100) != 0
        };
        if !enabled {
            continue;
        }
        let mut rect = [0; 4];
        if !form_rect(
            FormRectPlace {
                width: place.rect.x1 as i32,
                height: place.rect.y1 as i32,
                unit,
                inset: place.inset,
                client_top: place.client_top,
                right: place.client_right,
                bottom: place.client_bottom,
                flags: place.state,
            },
            code,
            &mut rect,
        ) {
            continue;
        }
        let active = mark_active(code, place.active_code, place.hot_flag, place.hot_code);
        drawn.arrow_icons.push(MarkIcon {
            x: (rect[0] + (unit >> 1)) as f32,
            y: (rect[1] + (unit >> 1)) as f32,
            id,
            ink: if active { None } else { Some(palette.text[5]) },
            scale: active.then_some(1.0),
        });
    }
    if (place.state & 0x200) != 0
        && let Some(scroll) = place.vertical
    {
        let mut rect = [0; 4];
        if form_rect(
            FormRectPlace {
                width: place.rect.x1 as i32,
                height: place.rect.y1 as i32,
                unit,
                inset: place.inset,
                client_top: place.client_top,
                right: place.client_right,
                bottom: place.client_bottom,
                flags: place.state,
            },
            -9,
            &mut rect,
        ) {
            let frame = GuiRect::corners(
                (place.rect.x1 as i32 - place.inset - 17) as f32,
                (place.client_top - 2) as f32,
                (place.rect.x1 as i32 - place.inset + 2) as f32,
                (place.rect.y1 as i32 - place.inset + 2) as f32,
            );
            double_frame(batch, frame, palette.form[5], palette.form[0]);
            let mut top = scroll.position - (place.client_top + 15);
            if top > 0 {
                drawn.scroll_shadows.push([
                    (place.rect.x1 as i32 - place.inset - 15),
                    place.client_top + 15,
                    15,
                    top,
                ]);
                shadow_ring(
                    batch,
                    GuiRect::sized(
                        (place.rect.x1 as i32 - place.inset - 15) as f32,
                        (place.client_top + 15) as f32,
                        15.0,
                        top as f32,
                    ),
                    unit,
                    0x30,
                );
            }
            top = (place.rect.y1 as i32 - place.client_top - scroll.position - scroll.length) - 15;
            if top > 0 {
                drawn.scroll_shadows.push([
                    (place.rect.x1 as i32 - place.inset - 15),
                    scroll.position + scroll.length,
                    15,
                    top,
                ]);
                shadow_ring(
                    batch,
                    GuiRect::sized(
                        (place.rect.x1 as i32 - place.inset - 15) as f32,
                        (scroll.position + scroll.length) as f32,
                        15.0,
                        top as f32,
                    ),
                    unit,
                    0x30,
                );
            }
            let selected = true;
            draw_frame_control(
                batch,
                GuiRect::sized(
                    rect[0] as f32,
                    rect[1] as f32,
                    rect[2] as f32,
                    rect[3] as f32,
                ),
                selected,
                false,
                palette,
            );
            batch.add_texture_shift(0.0, scroll.position as f32);
            draw_bar_with_boxes(
                batch,
                GuiRect::sized(
                    rect[0] as f32,
                    scroll.position as f32,
                    rect[2] as f32,
                    scroll.length as f32,
                ),
                unit,
                false,
                palette,
            );
            batch.add_texture_shift(0.0, -(scroll.position as f32));
            drawn
                .scroll_bars
                .push([rect[0], scroll.position, rect[2], scroll.length]);
            drawn.scroll_frames.push(rect);
        }
        if (place.state & 0x100) != 0
            && let Some(scroll) = place.horizontal
        {
            let mut rect = [0; 4];
            if form_rect(
                FormRectPlace {
                    width: place.rect.x1 as i32,
                    height: place.rect.y1 as i32,
                    unit,
                    inset: place.inset,
                    client_top: place.client_top,
                    right: place.client_right,
                    bottom: place.client_bottom,
                    flags: place.state,
                },
                -13,
                &mut rect,
            ) {
                draw_frame_control(
                    batch,
                    GuiRect::corners(
                        rect[0] as f32,
                        rect[1] as f32,
                        rect[2] as f32,
                        rect[3] as f32,
                    ),
                    false,
                    false,
                    palette,
                );
                batch.add_texture_shift(scroll.position as f32, 0.0);
                draw_bar_with_boxes(
                    batch,
                    GuiRect::corners(
                        scroll.position as f32,
                        rect[1] as f32,
                        (scroll.position + scroll.length) as f32,
                        rect[3] as f32,
                    ),
                    unit,
                    false,
                    palette,
                );
                batch.add_texture_shift(-(scroll.position as f32), 0.0);
                drawn
                    .scroll_bars
                    .push([scroll.position, rect[1], scroll.length, rect[3]]);
                let frame = GuiRect::corners(
                    (place.inset - 2) as f32,
                    (place.rect.y1 as i32 - place.inset - 17) as f32,
                    (place.client_right + 2) as f32,
                    (place.rect.y1 as i32 - place.inset + 2) as f32,
                );
                double_frame(batch, frame, palette.form[5], palette.form[0]);
                drawn.scroll_frames.push(rect);
            }
        }
    }
    if (place.state & 1) == 1 {
        let mut width = place.rect.x1 as i32 - (unit + place.inset) * 2;
        if (place.state & 2) == 2 {
            width -= 17;
        }
        if (place.state & 4) == 4 {
            width -= 20;
        }
        let x = if (place.title_style & 0x3000) == 0x3000 {
            (place.rect.x1 as i32 - place.title_width) / 2
        } else if place.title_x < 1 {
            place.inset + unit
        } else {
            width -= 17;
            place.inset + 0x15
        };
        let y = (place.client_top + place.title_offset + 24 + 1) >> 1;
        let first = [x - 1, y, width];
        let second = [x, y + 1, width];
        drawn.title = Some([first, second]);
    }
    drawn.begin_calls = (batch.primitives.len() - start) as u32;
    drawn
}

/// A form page's fields for the draw-order model.
#[derive(Debug, Clone, PartialEq)]
pub struct FormPagePlace {
    pub rect: GuiRect,
    pub inset: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub scale: [f32; 2],
    pub flags: u32,
    pub own_background: bool,
    pub parent_clip: [i32; 4],
    pub clip_ok: bool,
    pub lines: Vec<GuiRect>,
    pub controls: u32,
    pub children: Vec<bool>,
}

/// A form page's clip, origin, lines and draw-order counts.
#[derive(Debug, Clone, PartialEq)]
pub struct FormPage {
    pub background_calls: u32,
    pub clip: [i32; 4],
    pub visible: bool,
    pub origin: [i32; 2],
    pub scroll: [i32; 2],
    pub lines: Vec<[(f32, f32); 4]>,
    pub control_calls: u32,
    pub child_calls: u32,
    pub shadow: Option<GuiRect>,
}

/// Draws a form: its own background or the skin's, clips to the client,
/// applies the scroll offset, then draws the lines, the control records and
/// the visible children, and last the drop shadow.
pub fn draw_form_page(
    batch: &mut SkinBatch,
    place: FormPagePlace,
    unit: i32,
    palette: &SkinPalette,
) -> FormPage {
    batch.select(0x10, 0, false);
    let background_calls = u32::from(!place.own_background);
    let client = [
        place.rect.x0 as i32 + place.inset,
        place.rect.y0 as i32 + place.top,
        place.right - place.inset,
        place.bottom - place.top,
    ];
    let clip_x = client[0].max(place.parent_clip[0]);
    let clip_y = client[1].max(place.parent_clip[1]);
    let clip_right = (client[0] + client[2]).min(place.parent_clip[0] + place.parent_clip[2]);
    let clip_bottom = (client[1] + client[3]).min(place.parent_clip[1] + place.parent_clip[3]);
    let clip = [clip_x, clip_y, clip_right - clip_x, clip_bottom - clip_y];
    let overlap = clip[2] > 0 && clip[3] > 0;
    let (scroll, origin) = if (place.flags & 0x800) != 0 {
        ([0, 0], [place.rect.x0 as i32, place.rect.y0 as i32])
    } else {
        let x = ((place.scroll_x - (place.right - place.inset)) as f32 * place.scale[0])
            .round_ties_even() as i32;
        let y = ((place.scroll_y - (place.bottom - place.top)) as f32 * place.scale[1])
            .round_ties_even() as i32;
        ([x, y], [place.rect.x0 as i32 - x, place.rect.y0 as i32 - y])
    };
    if !overlap || !place.clip_ok {
        return FormPage {
            background_calls,
            clip,
            visible: false,
            origin,
            scroll,
            lines: Vec::new(),
            control_calls: 0,
            child_calls: 0,
            shadow: None,
        };
    }
    batch.select(0x10, 0, false);
    let mut lines = Vec::new();
    if !place.lines.is_empty() {
        batch.set_ink(palette.form[3]);
        batch.begin();
        for line in &place.lines {
            batch.vertex(line.x0, line.y0);
            batch.vertex(line.x0, line.y1);
            batch.vertex(line.x1, line.y1);
            batch.vertex(line.x1, line.y0);
        }
        batch.end_quads();
        lines = place
            .lines
            .iter()
            .map(|line| {
                [
                    (line.x0, line.y0),
                    (line.x0, line.y1),
                    (line.x1, line.y1),
                    (line.x1, line.y0),
                ]
            })
            .collect();
    }
    let child_calls = place.children.iter().filter(|visible| **visible).count() as u32;
    let shadow = if (place.flags & 0x80) != 0 {
        batch.select(1, 0, false);
        let rect = GuiRect::sized(
            scroll[0] as f32,
            scroll[1] as f32,
            (place.right - place.inset) as f32,
            (place.bottom - place.top) as f32,
        );
        shadow_ring(batch, rect, unit, 0x30);
        Some(rect)
    } else if (place.flags & 0x10_0000) != 0 {
        batch.select(1, 0, false);
        let rect = GuiRect::sized(
            scroll[0] as f32,
            scroll[1] as f32,
            (place.right - place.inset) as f32,
            (place.bottom - place.top) as f32,
        );
        shadow_ring(batch, rect, unit * 4, 0x50);
        Some(rect)
    } else {
        None
    };
    FormPage {
        background_calls,
        clip,
        visible: true,
        origin,
        scroll,
        lines,
        control_calls: place.controls,
        child_calls,
        shadow,
    }
}

/// A control label's measured placement. The text width is measured in the
/// control's font; the line height and base come from the same font.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlLabelPlace {
    pub rect: GuiRect,
    pub inset: i32,
    pub style: u32,
    pub state: u32,
    pub offset: i32,
    pub text_width: i32,
    pub line_height: i32,
    pub base: i32,
    pub text_len: usize,
    pub step: i32,
}

/// The control-label draw variants the skin carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelVariant {
    /// Bit 0x10 picks the focused ink, otherwise the plain one.
    Plain,
    /// The plain form, but bit 0x10000 draws text with its cursor.
    WithText,
    /// Bit 0x10 picks the selected ink, otherwise the caller's ink.
    Ink,
    /// Truncates the text to the width and draws it as a block.
    Block,
    /// The slot's full matrix: focus, colour, shadow or idle.
    Slot,
}

/// What a label draw asks the text renderer to do.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlLabelDraw {
    pub x: i32,
    pub y: i32,
    /// The ink of the single call, when there is one.
    pub ink: Option<SkinInk>,
    /// The two calls of the shadow arm, first then second.
    pub shadow: Option<[(i32, i32); 2]>,
    /// True for the coloured renderer.
    pub coloured: bool,
    /// The bytes of the text the block arm keeps.
    pub keep: usize,
    /// The cursor cross drawn after coloured text, and the blend it uses.
    pub cross: Option<SkinInk>,
    pub calls: u32,
    pub mode_calls: u32,
}

fn place_control_text(place: &ControlLabelPlace, unit: i32) -> (i32, i32) {
    let px = place.rect.x0 as i32;
    let py = place.rect.y0 as i32;
    let pw = (place.rect.x1 - place.rect.x0) as i32;
    let ph = (place.rect.y1 - place.rect.y0) as i32;
    let x = match place.style & 0x7000 {
        0x1000 => px - place.text_width - unit,
        0x3000 => px + (pw - 1 - place.text_width) / 2,
        0x4000 => px + pw - place.text_width - unit,
        0x5000 => px + pw + unit,
        _ => px + place.inset + (unit + 1) / 2,
    };
    let y = match place.style & 0x700 {
        0x100 => py - unit / 2,
        0x300 => py + (ph + place.line_height + 2) / 2,
        0x400 => py + ph + place.base - 1,
        0x500 => py + ph,
        _ => (unit + 1) / 2 + py,
    };
    (x, y)
}

/// Places and selects a control label: the measured x and y, then the ink
/// and the calls the variant makes.
pub fn draw_control_label(
    place: ControlLabelPlace,
    variant: LabelVariant,
    ink: SkinInk,
    palette: &SkinPalette,
    unit: i32,
) -> ControlLabelDraw {
    let (x, y) = place_control_text(&place, unit);
    let focused = (place.state & 0x10) == 0x10;
    let coloured = (place.state & 0xf_0000) == 0x1_0000;
    match variant {
        LabelVariant::Plain => ControlLabelDraw {
            x,
            y: y + place.offset,
            ink: Some(if focused {
                palette.text[2]
            } else {
                palette.text[5]
            }),
            shadow: None,
            coloured: false,
            keep: place.text_len,
            cross: None,
            calls: 1,
            mode_calls: 0,
        },
        LabelVariant::WithText => {
            if focused {
                ControlLabelDraw {
                    x,
                    y: y + place.offset,
                    ink: Some(palette.text[2]),
                    shadow: None,
                    coloured: false,
                    keep: place.text_len,
                    cross: None,
                    calls: 1,
                    mode_calls: 0,
                }
            } else if coloured {
                ControlLabelDraw {
                    x,
                    y: y + place.offset,
                    ink: Some(palette.text[6]),
                    shadow: None,
                    coloured: true,
                    keep: place.text_len,
                    cross: Some(palette.highlight[0]),
                    calls: 1,
                    mode_calls: 2,
                }
            } else {
                ControlLabelDraw {
                    x,
                    y: y + place.offset,
                    ink: Some(palette.text[5]),
                    shadow: None,
                    coloured: false,
                    keep: place.text_len,
                    cross: None,
                    calls: 1,
                    mode_calls: 0,
                }
            }
        }
        LabelVariant::Ink => ControlLabelDraw {
            x,
            y: y + place.offset,
            ink: Some(if focused { palette.text[3] } else { ink }),
            shadow: None,
            coloured: false,
            keep: place.text_len,
            cross: None,
            calls: 1,
            mode_calls: 0,
        },
        LabelVariant::Block => {
            let keep = if place.step > 0 {
                let fit = ((place.rect.x1 - place.rect.x0) as i32 - place.inset * 2) / place.step;
                place.text_len.min(fit.max(0) as usize)
            } else {
                place.text_len
            };
            ControlLabelDraw {
                x,
                y,
                ink: Some(if coloured {
                    with_alpha(
                        [
                            palette.highlight[7][0],
                            palette.highlight[7][1],
                            palette.highlight[7][2],
                            0,
                        ],
                        0xff,
                    )
                } else {
                    palette.text[5]
                }),
                shadow: None,
                coloured,
                keep,
                cross: None,
                calls: 1,
                mode_calls: 0,
            }
        }
        LabelVariant::Slot => {
            if focused {
                ControlLabelDraw {
                    x,
                    y,
                    ink: Some(palette.text[2]),
                    shadow: None,
                    coloured: false,
                    keep: place.text_len,
                    cross: None,
                    calls: 1,
                    mode_calls: 0,
                }
            } else if coloured {
                let nudge = i32::from((place.state & 0x40) == 0x40);
                ControlLabelDraw {
                    x,
                    y: y + nudge,
                    ink: Some(palette.text[6]),
                    shadow: None,
                    coloured: true,
                    keep: place.text_len,
                    cross: Some(palette.highlight[0]),
                    calls: 1,
                    mode_calls: 2,
                }
            } else if (place.state & 0x40) == 0x40 {
                ControlLabelDraw {
                    x,
                    y,
                    ink: None,
                    shadow: Some([(x - 1, y), (x, y + 1)]),
                    coloured: false,
                    keep: place.text_len,
                    cross: None,
                    calls: 2,
                    mode_calls: 0,
                }
            } else {
                ControlLabelDraw {
                    x,
                    y,
                    ink: Some(palette.text[7]),
                    shadow: None,
                    coloured: false,
                    keep: place.text_len,
                    cross: None,
                    calls: 1,
                    mode_calls: 0,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skin_stores_the_unit_and_the_three_colours() {
        let plain = open_skin(5, 0, false).expect("a passed object");
        assert_eq!(plain.span_a, 10);
        assert_eq!(plain.span_b, 5);
        assert_eq!(plain.span_c, 10);
        assert_eq!(plain.span_d, 10);
        assert_eq!(plain.span_e, 20);
        assert_eq!(
            [plain.pad_a, plain.pad_b, plain.pad_c, plain.pad_d],
            [8, 4, 8, 8]
        );
        assert_eq!(plain.inks, [SKIN_INK; 3]);
        assert_eq!(plain.mark, 1);
        assert_eq!(plain.alloc_calls, 0);
        assert_eq!(plain.base_calls, 1);
        assert_eq!(plain.after_calls, 0);

        let made = open_skin(5, 1, true).expect("an allocation");
        assert_eq!(made.span_a, 10);
        assert_eq!(made.alloc_calls, 1);
        assert_eq!(made.after_calls, 1);
        assert_eq!(made.inks[2], [0xff, 0xfa, 0xe6, 100]);

        assert!(open_skin(5, 1, false).is_none());

        let other = open_skin(5, 2, false).expect("a passed object");
        assert_eq!(other.alloc_calls, 0);
        assert_eq!(other.after_calls, 1);
        assert_eq!(other.span_b, 5);
    }

    #[test]
    fn a_flat_slider_places_the_thumb_from_the_unit() {
        let zero = draw_flat_slider(
            5,
            SliderPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 16,
                mode: 0,
                state: 0,
                lip: 0,
                value_bits: 0,
            },
        );
        let flat = zero.flat.expect("a flat slider");
        let ink = zero.ink;
        let pad = flat.pad;
        let thumb = flat.thumb;
        let marked = flat.marked;
        let quads = flat.quads;
        assert_eq!(ink, 2);
        assert_eq!(pad, 10);
        assert_eq!(thumb, 5);
        assert!(!marked);
        assert_eq!(quads[0].sample, SLIDER_CAP);
        assert_eq!(quads[0].corners, [(10, 40), (10, 20), (20, 20), (20, 40)]);
        assert_eq!(
            quads[1].corners,
            [(110, 40), (110, 20), (100, 20), (100, 40)]
        );
        assert_eq!(quads[2].sample, SLIDER_TRACK);
        assert_eq!(quads[2].corners, [(20, 40), (20, 20), (100, 20), (100, 40)]);
        assert_eq!(quads[3].sample, SLIDER_THUMB);
        assert_eq!(quads[3].corners, [(15, 40), (15, 20), (35, 20), (35, 40)]);

        let half = draw_flat_slider(
            5,
            SliderPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 16,
                mode: 0,
                state: 0,
                lip: 0,
                value_bits: 0x3f00_0000,
            },
        );
        assert_eq!(half.flat.expect("a flat slider").thumb, 40);

        let tie = draw_flat_slider(
            5,
            SliderPlace {
                x: 0,
                y: 0,
                w: 31,
                h: 16,
                mode: 0,
                state: 0,
                lip: 0,
                value_bits: 0x3f00_0000,
            },
        );
        assert_eq!(tie.flat.expect("a flat slider").thumb, 5);
        let next = draw_flat_slider(
            5,
            SliderPlace {
                x: 0,
                y: 0,
                w: 33,
                h: 16,
                mode: 0,
                state: 0,
                lip: 0,
                value_bits: 0x3f00_0000,
            },
        );
        assert_eq!(next.flat.expect("a flat slider").thumb, 7);

        let marked = draw_flat_slider(
            5,
            SliderPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 16,
                mode: 0,
                state: 0x20_0010 | 0x1_0000,
                lip: 0,
                value_bits: 0,
            },
        );
        assert_eq!(marked.ink, 0);
        assert!(marked.flat.expect("a flat slider").marked);

        let scroll_ink = draw_flat_slider(
            5,
            SliderPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 16,
                mode: 0,
                state: 0x20_0000,
                lip: 0,
                value_bits: 0,
            },
        );
        assert_eq!(scroll_ink.ink, 1);

        let scroll = draw_flat_slider(
            5,
            SliderPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 16,
                mode: 0x1000_0000,
                state: 0,
                lip: 0,
                value_bits: 0x3f80_0000,
            },
        );
        assert_eq!(scroll.ink, 2);
        assert!(scroll.flat.is_none());
        assert_eq!(scroll.scroll.expect("a scroll bar").thumb, 95);
    }

    #[test]
    fn a_scroll_bar_draws_twelve_quads() {
        let drawn = draw_flat_slider(
            5,
            SliderPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                mode: 0x1000_0000,
                state: 0x20_0000 | 0x1_0000,
                lip: 4,
                value_bits: 0,
            },
        );
        assert_eq!(drawn.ink, 1);
        assert!(drawn.flat.is_none());
        let bar = drawn.scroll.expect("a scroll bar");
        assert_eq!(bar.trough, [16, 26, 88, 68]);
        assert_eq!(bar.thumb, 10);
        assert_eq!(bar.thumb_far, 25);
        assert!(bar.marked);
        assert_eq!(bar.bar, 1);
        assert_eq!(bar.quads.len(), 12);
        assert_eq!(bar.quads[0].sample, SCROLL_EDGE);
        assert_eq!(
            bar.quads[0].corners,
            [(14, 25), (14, 15), (10, 15), (10, 25)]
        );
        assert_eq!(bar.quads[1].sample, SCROLL_BAR);
        assert_eq!(
            bar.quads[1].corners,
            [(14, 25), (14, 15), (106, 15), (106, 25)]
        );
        assert_eq!(
            bar.quads[2].corners,
            [(106, 25), (106, 15), (110, 15), (110, 25)]
        );
        assert_eq!(
            bar.quads[3].corners,
            [(14, 95), (14, 105), (10, 105), (10, 95)]
        );
        assert_eq!(bar.quads[4].sample, SCROLL_BAR);
        assert_eq!(
            bar.quads[4].corners,
            [(14, 95), (14, 105), (106, 105), (106, 95)]
        );
        assert_eq!(
            bar.quads[5].corners,
            [(106, 95), (106, 105), (110, 105), (110, 95)]
        );
        assert_eq!(bar.quads[6].sample, SCROLL_BAR);
        assert_eq!(bar.quads[6].corners, [(15, 96), (5, 96), (5, 24), (15, 24)]);
        assert_eq!(bar.quads[7].corners, [(15, 24), (5, 24), (5, 20), (15, 20)]);
        assert_eq!(
            bar.quads[8].corners,
            [(15, 96), (5, 96), (5, 100), (15, 100)]
        );
        assert_eq!(
            bar.quads[9].corners,
            [(20, 96), (30, 96), (30, 24), (20, 24)]
        );
        assert_eq!(
            bar.quads[10].corners,
            [(20, 24), (30, 24), (30, 20), (20, 20)]
        );
        assert_eq!(
            bar.quads[11].corners,
            [(20, 96), (30, 96), (30, 100), (20, 100)]
        );

        let half = draw_flat_slider(
            5,
            SliderPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                mode: 0x1000_0000,
                state: 0,
                lip: 4,
                value_bits: 0x3f00_0000,
            },
        );
        let half_bar = half.scroll.expect("a scroll bar");
        assert_eq!(half_bar.thumb, 52);
        assert_eq!(half_bar.bar, 0);
        assert!(!half_bar.marked);

        let tie = draw_flat_slider(
            5,
            SliderPlace {
                x: 10,
                y: 0,
                w: 16,
                h: 16,
                mode: 0x1000_0000,
                state: 0,
                lip: 0,
                value_bits: 0x3f00_0000,
            },
        );
        assert_eq!(tie.scroll.expect("a scroll bar").thumb, 10);
        let next = draw_flat_slider(
            5,
            SliderPlace {
                x: 10,
                y: 0,
                w: 18,
                h: 16,
                mode: 0x1000_0000,
                state: 0,
                lip: 0,
                value_bits: 0x3f00_0000,
            },
        );
        assert_eq!(next.scroll.expect("a scroll bar").thumb, 12);

        let other = draw_flat_slider(
            5,
            SliderPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                mode: 0x2000_0000,
                state: 0,
                lip: 4,
                value_bits: 0,
            },
        );
        assert!(other.flat.is_none());
        assert!(other.scroll.is_none());
    }

    #[test]
    fn a_frame_draws_corners_and_edge_bars() {
        let short = draw_frame(
            5,
            FramePlace {
                x: 10,
                y: 20,
                w: 100,
                h: 20,
                mode: 0,
                state: 0,
                mark: 0,
            },
        );
        assert_eq!(short.ink, 2);
        assert_eq!(short.quads.len(), 6);
        assert_eq!(short.quads[0].sample, FRAME_CORNER);
        assert_eq!(
            short.quads[0].corners,
            [(10, 20), (10, 35), (25, 35), (25, 20)]
        );
        assert_eq!(
            short.quads[1].corners,
            [(10, 40), (10, 25), (25, 25), (25, 40)]
        );
        assert_eq!(
            short.quads[2].corners,
            [(110, 20), (110, 35), (95, 35), (95, 20)]
        );
        assert_eq!(
            short.quads[3].corners,
            [(110, 40), (110, 25), (95, 25), (95, 40)]
        );
        assert_eq!(short.quads[4].sample, FRAME_EDGE);
        assert_eq!(
            short.quads[4].corners,
            [(25, 20), (25, 25), (95, 25), (95, 20)]
        );
        assert_eq!(
            short.quads[5].corners,
            [(25, 40), (25, 35), (95, 35), (95, 40)]
        );

        let tall = draw_frame(
            5,
            FramePlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                mode: 0,
                state: 0x20,
                mark: 1,
            },
        );
        assert_eq!(tall.ink, 3);
        assert_eq!(tall.quads.len(), 8);
        assert_eq!(
            tall.quads[6].corners,
            [(10, 85), (15, 85), (15, 35), (10, 35)]
        );
        assert_eq!(
            tall.quads[7].corners,
            [(110, 85), (105, 85), (105, 35), (110, 35)]
        );

        let pressed = draw_frame(
            5,
            FramePlace {
                x: 0,
                y: 0,
                w: 40,
                h: 20,
                mode: 0,
                state: 0x10,
                mark: 1,
            },
        );
        assert_eq!(pressed.ink, 0);
        let quiet = draw_frame(
            5,
            FramePlace {
                x: 0,
                y: 0,
                w: 40,
                h: 20,
                mode: 0x80_0000,
                state: 0,
                mark: 0,
            },
        );
        assert_eq!(quiet.ink, 0);
    }

    #[test]
    fn a_centred_icon_sits_on_half_the_spare() {
        let plain = draw_centred_icon(
            16,
            IconPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                mode: 0,
                state: 0,
                id: 7,
                own: false,
            },
        );
        assert!(!plain.own);
        assert!(!plain.marked);
        assert_eq!(plain.turn, 0);
        assert_eq!(plain.ink, 2);
        assert_eq!(plain.id, 7);
        assert_eq!((plain.x, plain.y), (52, 52));
        assert_eq!(plain.scale_bits, 0);

        let marked = draw_centred_icon(
            16,
            IconPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                mode: 0,
                state: 0x1_0000,
                id: 7,
                own: true,
            },
        );
        assert!(marked.own);
        assert!(marked.marked);
        assert_eq!((marked.x, marked.y), (52, 52));
        assert_eq!(marked.scale_bits, 0x3f80_0000);

        let nudged = draw_centred_icon(
            16,
            IconPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                mode: 0x4000_0000,
                state: 0x1_0040,
                id: 3,
                own: false,
            },
        );
        assert_eq!(nudged.turn, 4);
        assert_eq!((nudged.x, nudged.y), (52, 53));

        let both = draw_centred_icon(
            16,
            IconPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                mode: 0,
                state: 0x1_0060,
                id: 3,
                own: false,
            },
        );
        assert_eq!(both.y, 52);

        let tight = draw_centred_icon(
            16,
            IconPlace {
                x: 10,
                y: 20,
                w: 10,
                h: 8,
                mode: 0,
                state: 0,
                id: 1,
                own: false,
            },
        );
        assert_eq!((tight.x, tight.y), (7, 16));

        let pressed = draw_centred_icon(
            16,
            IconPlace {
                x: 0,
                y: 0,
                w: 16,
                h: 16,
                mode: 0x8000_0000,
                state: 0x10,
                id: 1,
                own: false,
            },
        );
        assert_eq!(pressed.ink, 0);
        let hot = draw_centred_icon(
            16,
            IconPlace {
                x: 0,
                y: 0,
                w: 16,
                h: 16,
                mode: 0,
                state: 0x40,
                id: 1,
                own: false,
            },
        );
        assert_eq!(hot.ink, 1);
        let dim_hot = draw_centred_icon(
            16,
            IconPlace {
                x: 0,
                y: 0,
                w: 16,
                h: 16,
                mode: 0x0100_0000,
                state: 0x40,
                id: 1,
                own: false,
            },
        );
        assert_eq!(dim_hot.ink, 3);
        let dim = draw_centred_icon(
            16,
            IconPlace {
                x: 0,
                y: 0,
                w: 16,
                h: 16,
                mode: 0x0100_0000,
                state: 0,
                id: 1,
                own: false,
            },
        );
        assert_eq!(dim.ink, 4);
    }

    fn place(
        flags_a: u32,
        flags_b: u32,
        flags_c: u32,
        along_x: u32,
        along_y: u32,
        count: i32,
    ) -> DispatchPlace {
        DispatchPlace {
            flags_a,
            flags_b,
            flags_c,
            flags_d: 0,
            title_width: 0,
            title_byte: 0,
            baseline: 0,
            titled: false,
            close_mark: 0,
            width: 100,
            height: 80,
            at_x: 3,
            at_y: 4,
            view_x: 10,
            view_y: 6,
            span_x: 30,
            span_y: 8,
            limit_x: 50,
            limit_y: 20,
            along_x,
            along_y,
            count,
            edge_x: 0,
            edge_y: 0,
            room_far_x: 0,
            room_near_x: 0,
            room_far_y: 0,
            room_near_y: 0,
            owner_flags: 0,
            grip_x: 0,
            grip_y: 0,
            seat_y: 0,
            post_y: 0,
            close_x: 0,
            close_y: 0,
        }
    }

    #[test]
    fn a_dispatch_stores_the_spans_and_the_shown_children() {
        let open = dispatch_form(
            5,
            9,
            place(0, 0x2_0040, 0, 0x3f80_0000, 0x3f00_0000, 4),
            &[0, 1, 0, 1],
        );
        assert_eq!((open.span_a, open.span_b, open.span_c), (10, 15, 20));
        assert!(open.shadow);
        assert_eq!(open.shade, 9);
        let back = open.backdrop.expect("a backdrop");
        assert_eq!((back[0].x, back[0].y, back[0].u, back[0].v), (0, 0, 0, 0));
        assert_eq!((back[1].x, back[1].y, back[1].v), (0, 80, 0x3f80_0000));
        assert_eq!((back[2].x, back[2].y), (100, 80));
        assert_eq!((back[3].u, back[3].v), (0x3f80_0000, 0));
        assert_eq!((open.shift_x, open.shift_y), (30, 9));
        assert_eq!((open.origin_x, open.origin_y), (-20, -3));
        assert_eq!(open.clip, [13, 10, 20, 2]);
        assert!(open.shifted);
        assert_eq!(open.pane_calls, 1);
        assert_eq!(open.shown, [1, 3]);

        let held = dispatch_form(5, 9, place(4, 0x2_0000, 0x100, 0, 0, 0), &[]);
        assert!(held.backdrop.is_none());
        assert!(!held.shadow);
        assert_eq!(held.shade, 0);
        assert!(!held.shifted);
        assert!(held.shown.is_empty());

        let locked = dispatch_form(5, 1, place(0, 0x800, 0, 0x3f80_0000, 0x3f80_0000, 1), &[0]);
        assert_eq!((locked.shift_x, locked.shift_y), (0, 0));
        assert_eq!((locked.origin_x, locked.origin_y), (10, 6));
        assert!(locked.shown.is_empty());

        let tie = dispatch_form(
            5,
            1,
            DispatchPlace {
                limit_x: 1,
                span_x: 0,
                view_x: 0,
                along_x: 0x3f00_0000,
                limit_y: 3,
                span_y: 0,
                view_y: 0,
                along_y: 0x3f00_0000,
                ..place(0, 0, 0, 0, 0, 0)
            },
            &[],
        );
        assert_eq!((tie.shift_x, tie.shift_y), (0, 2));
    }

    #[test]
    fn a_dispatch_centres_the_title_and_draws_the_corners() {
        let mut form = place(0, 0x11, 0, 0, 0, 0);
        form.flags_d = 0x10;
        form.title_width = 40;
        form.baseline = 12;
        form.titled = true;
        form.title_byte = 200;
        let drawn = dispatch_form(5, 0, form, &[]);
        let title = drawn.title.expect("a title");
        assert_eq!(title.x, 30);
        assert_eq!(title.baseline, 12);
        assert_eq!(title.width, 100);
        assert_eq!(title.tint, Some(200));
        assert_eq!(drawn.corners.len(), 4);
        assert_eq!(drawn.corners[0].sample, FORM_CORNER);
        assert_eq!(
            drawn.corners[0].corners,
            [(0, 0), (0, 30), (30, 30), (30, 0)]
        );
        assert_eq!(
            drawn.corners[1].corners,
            [(100, 0), (70, 0), (70, 30), (100, 30)]
        );
        assert_eq!(
            drawn.corners[2].corners,
            [(0, 80), (30, 80), (30, 50), (0, 50)]
        );
        assert_eq!(
            drawn.corners[3].corners,
            [(100, 80), (100, 50), (70, 50), (70, 80)]
        );
        assert_eq!(drawn.wide_calls, 0);

        form.flags_b = 0x13;
        form.close_mark = 0;
        let closed = dispatch_form(5, 0, form, &[]);
        assert_eq!(closed.corners.len(), 5);
        assert_eq!(closed.corners[1].sample, FORM_CLOSE_INK);
        assert_eq!(
            closed.corners[1].corners,
            [(100, 0), (80, 0), (80, 20), (100, 20)]
        );
        assert_eq!(closed.corners[2].sample, FORM_ALT);

        form.close_mark = -4;
        let bare = dispatch_form(5, 0, form, &[]);
        assert_eq!(bare.corners.len(), 4);
        assert_eq!(bare.corners[1].sample, FORM_ALT);

        form.flags_b = 0x40_0011;
        let wide = dispatch_form(5, 0, form, &[]);
        assert!(wide.corners.is_empty());
        assert_eq!(wide.wide_calls, 1);

        form.flags_b = 1;
        form.flags_d = 0;
        form.title_width = 16;
        form.width = 10;
        form.titled = false;
        let tight = dispatch_form(5, 0, form, &[]);
        let title = tight.title.expect("a title");
        assert_eq!(title.x, -3);
        assert!(title.tint.is_none());
        assert!(tight.corners.is_empty());

        form.title_width = 0;
        assert!(dispatch_form(5, 0, form, &[]).title.is_none());
    }

    #[test]
    fn a_dispatch_draws_the_edge_bars() {
        let mut form = place(0, 0, 0, 0, 0, 0);
        form.flags_d = 0x10;
        let plain = dispatch_form(5, 0, form, &[]);
        assert_eq!(plain.edges.len(), 4);
        assert_eq!(plain.edges[0].sample, FORM_EDGE);
        assert_eq!(
            plain.edges[0].corners,
            [(33, 0), (33, 10), (67, 10), (67, 0)]
        );
        assert_eq!(
            plain.edges[1].corners,
            [(0, 47), (10, 47), (10, 33), (0, 33)]
        );
        assert_eq!(
            plain.edges[2].corners,
            [(33, 80), (33, 70), (67, 70), (67, 80)]
        );
        assert_eq!(
            plain.edges[3].corners,
            [(100, 47), (90, 47), (90, 33), (100, 33)]
        );

        form.flags_b = 1;
        form.title_width = 40;
        let split = dispatch_form(5, 0, form, &[]);
        assert_eq!(split.edges.len(), 7);
        assert_eq!(
            split.edges[0].corners,
            [(33, 0), (33, 10), (17, 10), (17, 0)]
        );
        assert_eq!(
            split.edges[1].corners,
            [(83, 0), (83, 10), (67, 10), (67, 0)]
        );
        assert_eq!(split.edges[2].sample, FORM_CAP);
        assert_eq!(
            split.edges[2].corners,
            [(17, 0), (17, 10), (27, 10), (27, 0)]
        );
        assert_eq!(
            split.edges[3].corners,
            [(83, 0), (83, 10), (73, 10), (73, 0)]
        );

        form.flags_d = 0;
        assert!(dispatch_form(5, 0, form, &[]).edges.is_empty());
        form.flags_d = 0x10;
        form.flags_b = 0x40_0000;
        assert!(dispatch_form(5, 0, form, &[]).edges.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_wide_top() {
        let mut form = place(0, 0x40_0000, 0, 0, 0, 0);
        form.flags_d = 0x80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert!(drawn.corners.is_empty());
        assert!(drawn.edges.is_empty());
        assert_eq!(drawn.wide_calls, 0);
        assert_eq!(drawn.wide_top.len(), 3);
        assert_eq!(drawn.wide_top[0].sample, WIDE_CAP);
        assert_eq!(
            drawn.wide_top[0].corners,
            [(10, 5), (10, -5), (0, -5), (0, 5)]
        );
        assert_eq!(drawn.wide_top[1].sample, WIDE_BAR);
        assert_eq!(
            drawn.wide_top[1].corners,
            [(10, 5), (10, -5), (90, -5), (90, 5)]
        );
        assert_eq!(drawn.wide_top[2].sample, WIDE_CAP);
        assert_eq!(
            drawn.wide_top[2].corners,
            [(90, 5), (90, -5), (100, -5), (100, 5)]
        );

        form.flags_d = 0x90;
        let styled = dispatch_form(5, 0, form, &[]);
        assert_eq!(styled.wide_top.len(), 3);
        assert_eq!(styled.wide_calls, 1);
        assert!(styled.corners.is_empty());
        form.flags_d = 0x10;
        assert!(dispatch_form(5, 0, form, &[]).wide_top.is_empty());
        form.flags_b = 0;
        form.flags_d = 0x80;
        assert!(dispatch_form(5, 0, form, &[]).wide_top.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_wide_side() {
        let mut form = place(0, 0x40_0000, 0, 0, 0, 0);
        form.flags_d = 0x80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.wide_top.len(), 3);
        assert_eq!(drawn.wide_side.len(), 3);
        assert_eq!(drawn.wide_side[0].sample, WIDE_BAR);
        assert_eq!(
            drawn.wide_side[0].corners,
            [(5, 70), (-5, 70), (-5, 10), (5, 10)]
        );
        assert_eq!(drawn.wide_side[1].sample, WIDE_CAP);
        assert_eq!(
            drawn.wide_side[1].corners,
            [(5, 10), (-5, 10), (-5, 0), (5, 0)]
        );
        assert_eq!(
            drawn.wide_side[2].corners,
            [(5, 70), (-5, 70), (-5, 80), (5, 80)]
        );

        form.flags_d = 0x10;
        assert!(dispatch_form(5, 0, form, &[]).wide_side.is_empty());
        form.flags_b = 0;
        form.flags_d = 0x80;
        assert!(dispatch_form(5, 0, form, &[]).wide_side.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_wide_bottom() {
        let mut form = place(0, 0x40_0000, 0, 0, 0, 0);
        form.flags_d = 0x80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.wide_side.len(), 3);
        assert_eq!(drawn.wide_bottom.len(), 3);
        assert_eq!(drawn.wide_bottom[0].sample, WIDE_CAP);
        assert_eq!(
            drawn.wide_bottom[0].corners,
            [(10, 75), (10, 85), (0, 85), (0, 75)]
        );
        assert_eq!(drawn.wide_bottom[1].sample, WIDE_BAR);
        assert_eq!(
            drawn.wide_bottom[1].corners,
            [(10, 75), (10, 85), (90, 85), (90, 75)]
        );
        assert_eq!(drawn.wide_bottom[2].sample, WIDE_CAP);
        assert_eq!(
            drawn.wide_bottom[2].corners,
            [(90, 75), (90, 85), (100, 85), (100, 75)]
        );

        form.flags_d = 0x10;
        assert!(dispatch_form(5, 0, form, &[]).wide_bottom.is_empty());
        form.flags_b = 0;
        form.flags_d = 0x80;
        assert!(dispatch_form(5, 0, form, &[]).wide_bottom.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_wide_right() {
        let mut form = place(0, 0x40_0000, 0, 0, 0, 0);
        form.flags_d = 0x80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.wide_bottom.len(), 3);
        assert_eq!(drawn.wide_right.len(), 3);
        assert_eq!(drawn.wide_right[0].sample, WIDE_BAR);
        assert_eq!(
            drawn.wide_right[0].corners,
            [(95, 70), (105, 70), (105, 10), (95, 10)]
        );
        assert_eq!(drawn.wide_right[1].sample, WIDE_CAP);
        assert_eq!(
            drawn.wide_right[1].corners,
            [(95, 10), (105, 10), (105, 0), (95, 0)]
        );
        assert_eq!(
            drawn.wide_right[2].corners,
            [(95, 70), (105, 70), (105, 80), (95, 80)]
        );

        form.flags_d = 0x10;
        assert!(dispatch_form(5, 0, form, &[]).wide_right.is_empty());
        form.flags_b = 0;
        form.flags_d = 0x80;
        assert!(dispatch_form(5, 0, form, &[]).wide_right.is_empty());
    }

    #[test]
    fn a_dispatch_copies_the_ink_after_the_right_side() {
        let mut form = place(0x40, 0x40_0000, 0, 0, 0, 0);
        form.flags_d = 0x80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.wide_right.len(), 3);
        assert_eq!(drawn.mark_calls, 1);

        form.flags_a = 0;
        let clear = dispatch_form(5, 0, form, &[]);
        assert_eq!(clear.wide_right.len(), 3);
        assert_eq!(clear.mark_calls, 0);

        form.flags_a = 0x40;
        form.flags_d = 0x10;
        assert_eq!(dispatch_form(5, 0, form, &[]).mark_calls, 0);
        form.flags_b = 0;
        form.flags_d = 0x80;
        assert_eq!(dispatch_form(5, 0, form, &[]).mark_calls, 0);
    }

    #[test]
    fn a_dispatch_draws_the_late_close() {
        let mut form = place(0, 0, 0, 0, 0, 0);
        form.close_mark = -4;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.late_close.len(), 1);
        assert_eq!(drawn.late_close[0].sample, FORM_CLOSE_INK);
        assert_eq!(
            drawn.late_close[0].corners,
            [(100, 0), (80, 0), (80, 20), (100, 20)]
        );
        assert!(drawn.corners.is_empty());

        form.close_mark = 0;
        assert!(dispatch_form(5, 0, form, &[]).late_close.is_empty());
        form.close_mark = -3;
        assert!(dispatch_form(5, 0, form, &[]).late_close.is_empty());
    }

    #[test]
    fn a_dispatch_marks_the_held_edges() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        form.room_near_x = 0;
        form.room_far_y = 200;
        form.room_near_y = 0;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask, 7);
        assert!(drawn.wide_top.is_empty());

        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 200;
        form.room_far_y = 200;
        assert_eq!(dispatch_form(5, 0, form, &[]).edge_mask, 0);

        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 109;
        form.room_far_y = 89;
        assert_eq!(dispatch_form(5, 0, form, &[]).edge_mask, 3);

        form.flags_a = 0;
        assert_eq!(dispatch_form(5, 0, form, &[]).edge_mask, 0);
        form.flags_a = FORM_HOLD;
        form.flags_b = 0x40_0000;
        assert_eq!(dispatch_form(5, 0, form, &[]).edge_mask, 0);
        form.flags_b = 0;
        form.flags_d = 0x10;
        assert_eq!(dispatch_form(5, 0, form, &[]).edge_mask, 0);
    }

    #[test]
    fn a_dispatch_draws_the_held_bar() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_y = 5;
        form.edge_x = 5;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 1, 0);
        assert_eq!(drawn.held_bar.len(), 1);
        assert_eq!(drawn.held_bar[0].sample, WIDE_BAR);
        assert_eq!(
            drawn.held_bar[0].corners,
            [(30, 0), (30, 10), (70, 10), (70, 0)]
        );

        form.edge_y = 4;
        form.owner_flags = 1;
        let skipped = dispatch_form(5, 0, form, &[]);
        assert_eq!(skipped.edge_mask & 1, 1);
        assert!(skipped.held_bar.is_empty());

        form.owner_flags = 1 | 0x80_0000;
        assert_eq!(dispatch_form(5, 0, form, &[]).held_bar.len(), 1);
        form.owner_flags = 0;
        assert_eq!(dispatch_form(5, 0, form, &[]).held_bar.len(), 1);

        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_bar.is_empty());
        form.flags_a = FORM_HOLD;
        form.flags_b = 0x40_0000;
        assert!(dispatch_form(5, 0, form, &[]).held_bar.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_tab() {
        let mut form = place(FORM_HOLD, FORM_TAB, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.room_far_y = 400;
        form.grip_x = 10;
        form.grip_y = 20;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.held_bar.len(), 1);
        assert_eq!(drawn.held_tab.len(), 1);
        assert_eq!(drawn.held_tab[0].sample, HELD_TAB);
        assert_eq!(
            drawn.held_tab[0].corners,
            [(16, 40), (16, 30), (26, 30), (26, 40)]
        );

        form.edge_y = 4;
        form.edge_x = 4;
        form.room_far_x = 100;
        let raised = dispatch_form(5, 0, form, &[]);
        assert_eq!(raised.edge_mask & 0x5, 0x5);
        assert_eq!(
            raised.held_tab[0].corners,
            [(16, 45), (16, 35), (26, 35), (26, 45)]
        );

        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_tab.is_empty());
        form.flags_b = FORM_TAB;
        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_tab.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_lower_tab() {
        let mut form = place(FORM_HOLD, FORM_TAB, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.room_far_y = 400;
        form.grip_x = 10;
        form.seat_y = 80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.held_tab.len(), 1);
        assert_eq!(drawn.held_lower.len(), 1);
        assert_eq!(drawn.held_lower[0].sample, HELD_TAB);
        assert_eq!(
            drawn.held_lower[0].corners,
            [(16, 60), (16, 70), (26, 70), (26, 60)]
        );

        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        form.room_far_y = 80;
        let raised = dispatch_form(5, 0, form, &[]);
        assert_eq!(raised.edge_mask & 0xc, 0xc);
        assert_eq!(
            raised.held_lower[0].corners,
            [(16, 55), (16, 65), (26, 65), (26, 55)]
        );

        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_lower.is_empty());
        form.flags_b = FORM_TAB;
        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_lower.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_post_quad() {
        let mut form = place(FORM_HOLD, FORM_TAB, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.room_far_y = 400;
        form.grip_x = 10;
        form.post_y = 40;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.held_lower.len(), 1);
        assert_eq!(drawn.held_post.len(), 1);
        assert_eq!(drawn.held_post[0].sample, HELD_POST);
        assert_eq!(
            drawn.held_post[0].corners,
            [(11, 55), (11, 35), (31, 35), (31, 55)]
        );

        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_post.is_empty());
        form.flags_b = FORM_TAB;
        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_post.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_left() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert!(drawn.held_post.is_empty());
        assert_eq!(drawn.held_left.len(), 1);
        assert_eq!(drawn.held_left[0].sample, WIDE_BAR);
        assert_eq!(
            drawn.held_left[0].corners,
            [(0, 50), (10, 50), (10, 30), (0, 30)]
        );

        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_left.is_empty());
        form.flags_a = FORM_HOLD;
        form.flags_b = 0x40_0000;
        assert!(dispatch_form(5, 0, form, &[]).held_left.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_cap() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 400;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 0x3, 0x3);
        assert_eq!(drawn.held_left.len(), 1);
        assert_eq!(drawn.held_cap.len(), 1);
        assert_eq!(drawn.held_cap[0].sample, HELD_CAP);
        assert_eq!(
            drawn.held_cap[0].corners,
            [(0, 10), (10, 10), (10, 30), (0, 30)]
        );

        form.edge_x = 5;
        form.edge_y = 5;
        let level = dispatch_form(5, 0, form, &[]);
        assert_eq!(level.held_left.len(), 1);
        assert!(level.held_cap.is_empty());

        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_cap.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_ledge() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 400;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.held_cap.len(), 1);
        assert_eq!(drawn.held_ledge.len(), 1);
        assert_eq!(drawn.held_ledge[0].sample, HELD_CAP);
        assert_eq!(
            drawn.held_ledge[0].corners,
            [(10, 0), (10, 10), (30, 10), (30, 0)]
        );

        form.flags_b = FORM_TITLE;
        assert!(dispatch_form(5, 0, form, &[]).held_ledge.is_empty());
        form.owner_flags = 0x80_0000;
        assert_eq!(dispatch_form(5, 0, form, &[]).held_ledge.len(), 1);

        form.flags_b = 0;
        form.owner_flags = 1;
        assert!(dispatch_form(5, 0, form, &[]).held_ledge.is_empty());
        form.owner_flags = 1 | 0x80_0000;
        assert_eq!(dispatch_form(5, 0, form, &[]).held_ledge.len(), 1);

        form.owner_flags = 0;
        form.edge_x = 5;
        form.edge_y = 5;
        let level = dispatch_form(5, 0, form, &[]);
        assert_eq!(level.held_left.len(), 1);
        assert!(level.held_ledge.is_empty());

        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_ledge.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_open_frame() {
        let mut form = place(0, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.height = 100;
        form.grip_x = 10;
        form.grip_y = 20;
        form.seat_y = 80;
        form.post_y = 40;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask, 0);
        assert!(drawn.held_left.is_empty());
        assert_eq!(drawn.open_mask, 0xf);
        assert_eq!(drawn.open_calls, 1);
        assert!(drawn.open_tab.is_empty());
        assert_eq!(drawn.open_left[0].sample, WIDE_BAR);
        assert_eq!(
            drawn.open_left[0].corners,
            [(10, 60), (20, 60), (20, 40), (10, 40)]
        );
        assert_eq!(
            drawn.open_cap[0].corners,
            [(10, 20), (20, 20), (20, 40), (10, 40)]
        );
        assert_eq!(
            drawn.open_ledge[0].corners,
            [(20, 10), (20, 20), (40, 20), (40, 10)]
        );

        form.flags_b = FORM_TAB;
        let tabbed = dispatch_form(5, 0, form, &[]);
        assert_eq!(tabbed.open_calls, 1);
        assert!(tabbed.held_tab.is_empty());
        assert_eq!(
            tabbed.open_tab[0].corners,
            [(16, 45), (16, 35), (26, 35), (26, 45)]
        );
        assert_eq!(
            tabbed.open_lower[0].corners,
            [(16, 55), (16, 65), (26, 65), (26, 55)]
        );
        assert_eq!(
            tabbed.open_post[0].corners,
            [(11, 55), (11, 35), (31, 35), (31, 55)]
        );

        form.flags_b = FORM_TITLE;
        let titled = dispatch_form(5, 0, form, &[]);
        assert_eq!(titled.open_calls, 0);
        assert_eq!(titled.open_left.len(), 1);
        assert!(titled.open_ledge.is_empty());

        form.flags_b = 0x40_0000;
        let wide = dispatch_form(5, 0, form, &[]);
        assert_eq!(wide.open_mask, 0);
        assert_eq!(wide.open_calls, 0);
        assert!(wide.open_left.is_empty());

        form.flags_b = 0;
        form.flags_d = 0x10;
        assert_eq!(dispatch_form(5, 0, form, &[]).open_mask, 0);

        form.flags_a = FORM_HOLD;
        form.flags_d = 0x80;
        let held = dispatch_form(5, 0, form, &[]);
        assert_eq!(held.open_mask, 0);
        assert!(held.open_left.is_empty());
        assert_eq!(held.held_left.len(), 1);
    }

    #[test]
    fn a_dispatch_draws_the_sign_flip() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 0x3, 0);
        assert_eq!(drawn.held_left.len(), 1);
        assert!(drawn.held_cap.is_empty());
        assert_eq!(drawn.held_flip.len(), 1);
        assert_eq!(drawn.held_flip[0].sample, HELD_FLIP);
        assert_eq!(
            drawn.held_flip[0].corners,
            [(-10, -10), (-10, 30), (30, 30), (30, -10)]
        );

        form.edge_y = 4;
        let one = dispatch_form(5, 0, form, &[]);
        assert_eq!(one.edge_mask & 0x3, 1);
        assert_eq!(one.held_flip.len(), 1);

        form.edge_x = 4;
        form.room_far_x = 400;
        let both = dispatch_form(5, 0, form, &[]);
        assert_eq!(both.edge_mask & 0x3, 0x3);
        assert!(both.held_flip.is_empty());
        assert_eq!(both.held_cap.len(), 1);

        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_flip.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_end() {
        let mut form = place(FORM_HOLD, FORM_TAB, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 0x5, 0x5);
        assert_eq!(drawn.held_end.len(), 1);
        assert_eq!(drawn.held_end[0].sample, HELD_END);
        assert_eq!(
            drawn.held_end[0].corners,
            [(100, 30), (100, 10), (70, 10), (70, 30)]
        );

        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_end.is_empty());

        form.flags_b = FORM_TAB;
        form.edge_y = 5;
        assert!(dispatch_form(5, 0, form, &[]).held_end.is_empty());

        form.edge_y = 4;
        form.room_far_x = 400;
        assert!(dispatch_form(5, 0, form, &[]).held_end.is_empty());

        form.room_far_x = 100;
        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_end.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_plain_cap() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 0x5, 0x5);
        assert!(drawn.held_end.is_empty());
        assert_eq!(drawn.held_plain.len(), 1);
        assert_eq!(drawn.held_plain[0].sample, HELD_CAP);
        assert_eq!(
            drawn.held_plain[0].corners,
            [(100, 10), (90, 10), (90, 30), (100, 30)]
        );

        form.flags_b = FORM_TAB;
        let tabbed = dispatch_form(5, 0, form, &[]);
        assert!(tabbed.held_plain.is_empty());
        assert_eq!(tabbed.held_end.len(), 1);

        form.flags_b = 0;
        form.edge_y = 5;
        assert!(dispatch_form(5, 0, form, &[]).held_plain.is_empty());

        form.edge_y = 4;
        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_plain.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_shelf() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.held_plain.len(), 1);
        assert_eq!(drawn.held_shelf.len(), 1);
        assert_eq!(drawn.held_shelf[0].sample, HELD_CAP);
        assert_eq!(
            drawn.held_shelf[0].corners,
            [(90, 0), (90, 10), (70, 10), (70, 0)]
        );

        form.flags_b = FORM_TITLE;
        let titled = dispatch_form(5, 0, form, &[]);
        assert_eq!(titled.held_plain.len(), 1);
        assert!(titled.held_shelf.is_empty());
        form.owner_flags = 0x80_0000;
        assert_eq!(dispatch_form(5, 0, form, &[]).held_shelf.len(), 1);

        form.flags_b = 0;
        form.owner_flags = 1;
        assert!(dispatch_form(5, 0, form, &[]).held_shelf.is_empty());
        form.owner_flags = 1 | 0x80_0000;
        assert_eq!(dispatch_form(5, 0, form, &[]).held_shelf.len(), 1);

        form.owner_flags = 0;
        form.flags_b = FORM_TAB;
        assert!(dispatch_form(5, 0, form, &[]).held_shelf.is_empty());

        form.flags_b = 0;
        form.edge_y = 5;
        assert!(dispatch_form(5, 0, form, &[]).held_shelf.is_empty());

        form.edge_y = 4;
        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_shelf.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_over() {
        let mut form = place(FORM_HOLD, FORM_TAB, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 0x5, 0);
        assert!(drawn.held_end.is_empty());
        assert_eq!(drawn.held_over.len(), 1);
        assert_eq!(drawn.held_over[0].sample, HELD_OVER);
        assert_eq!(
            drawn.held_over[0].corners,
            [(110, -10), (110, 30), (70, 30), (70, -10)]
        );

        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_over.is_empty());

        form.flags_b = FORM_TAB;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        let clipped = dispatch_form(5, 0, form, &[]);
        assert_eq!(clipped.edge_mask & 0x5, 0x5);
        assert!(clipped.held_over.is_empty());
        assert_eq!(clipped.held_end.len(), 1);

        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_over.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_bare() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 0x5, 0);
        assert!(drawn.held_over.is_empty());
        assert_eq!(drawn.held_bare.len(), 1);
        assert_eq!(drawn.held_bare[0].sample, HELD_FLIP);
        assert_eq!(
            drawn.held_bare[0].corners,
            [(110, -10), (110, 30), (70, 30), (70, -10)]
        );

        form.flags_b = FORM_TAB;
        let tabbed = dispatch_form(5, 0, form, &[]);
        assert!(tabbed.held_bare.is_empty());
        assert_eq!(tabbed.held_over.len(), 1);

        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        let clipped = dispatch_form(5, 0, form, &[]);
        assert!(clipped.held_bare.is_empty());
        assert!(clipped.held_over.is_empty());
        assert_eq!(clipped.held_end.len(), 1);

        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_bare.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_right() {
        let mut form = place(FORM_HOLD, FORM_TAB, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.held_over.len(), 1);
        assert!(drawn.open_right.is_empty());
        assert_eq!(drawn.held_right.len(), 1);
        assert_eq!(drawn.held_right[0].sample, HELD_RIGHT);
        assert_eq!(
            drawn.held_right[0].corners,
            [(90, 30), (90, 50), (80, 50), (80, 30)]
        );

        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        let capped = dispatch_form(5, 0, form, &[]);
        assert_eq!(capped.held_end.len(), 1);
        assert_eq!(capped.held_right.len(), 1);
        assert_eq!(
            capped.held_right[0].corners,
            [(90, 30), (90, 50), (80, 50), (80, 30)]
        );

        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_right.is_empty());

        form.flags_b = FORM_TAB;
        form.flags_a = 0;
        form.height = 100;
        let open = dispatch_form(5, 0, form, &[]);
        assert!(open.held_right.is_empty());
        assert_eq!(open.open_right.len(), 1);
        assert_eq!(open.open_right[0].sample, HELD_RIGHT);
        assert_eq!(
            open.open_right[0].corners,
            [(80, 40), (80, 60), (70, 60), (70, 40)]
        );

        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).open_right.is_empty());

        form.flags_a = FORM_HOLD;
        form.flags_b = FORM_TAB | FORM_BOX;
        form.flags_d = 0x80;
        assert!(dispatch_form(5, 0, form, &[]).held_right.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_beam() {
        let mut form = place(FORM_HOLD, FORM_TAB, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 4, 0);
        assert_eq!(drawn.held_right.len(), 1);
        assert_eq!(drawn.held_beam.len(), 1);
        assert_eq!(drawn.held_beam[0].sample, WIDE_BAR);
        assert_eq!(
            drawn.held_beam[0].corners,
            [(106, 60), (96, 60), (96, 20), (106, 20)]
        );

        form.flags_c = FORM_STILL;
        let still = dispatch_form(5, 0, form, &[]);
        assert!(still.shifted);
        assert_eq!(still.held_right.len(), 1);
        assert!(still.held_beam.is_empty());

        form.flags_c = FORM_PLAIN;
        let plain = dispatch_form(5, 0, form, &[]);
        assert!(!plain.shifted);
        assert_eq!(plain.held_beam.len(), 1);

        form.flags_c = 0;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        let clipped = dispatch_form(5, 0, form, &[]);
        assert_eq!(clipped.edge_mask & 4, 4);
        assert!(clipped.held_beam.is_empty());
        assert_eq!(clipped.held_end.len(), 1);

        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_beam.is_empty());

        form.flags_b = FORM_TAB;
        form.flags_a = 0;
        let open = dispatch_form(5, 0, form, &[]);
        assert_eq!(open.open_mask, 0xf);
        assert!(open.held_beam.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_caps() {
        let mut form = place(FORM_HOLD, FORM_TAB, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.held_beam.len(), 1);
        assert_eq!(drawn.held_caps.len(), 2);
        assert_eq!(drawn.held_caps[0].sample, WIDE_CAP);
        assert_eq!(drawn.held_caps[1].sample, WIDE_CAP);
        assert_eq!(
            drawn.held_caps[0].corners,
            [(106, 20), (96, 20), (96, 0), (106, 0)]
        );
        assert_eq!(
            drawn.held_caps[1].corners,
            [(106, 60), (96, 60), (96, 80), (106, 80)]
        );

        form.flags_c = FORM_STILL;
        let still = dispatch_form(5, 0, form, &[]);
        assert_eq!(still.held_right.len(), 1);
        assert!(still.held_caps.is_empty());

        form.flags_c = 0;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        assert!(dispatch_form(5, 0, form, &[]).held_caps.is_empty());

        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_caps.is_empty());

        form.flags_b = FORM_TAB;
        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_caps.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_trim() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert!(drawn.held_beam.is_empty());
        assert!(drawn.open_trim.is_empty());
        assert_eq!(drawn.held_trim.len(), 1);
        assert_eq!(drawn.held_trim[0].sample, WIDE_BAR);
        assert_eq!(
            drawn.held_trim[0].corners,
            [(100, 50), (90, 50), (90, 30), (100, 30)]
        );

        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        let clipped = dispatch_form(5, 0, form, &[]);
        assert_eq!(clipped.held_plain.len(), 1);
        assert_eq!(clipped.held_trim.len(), 1);

        form.flags_b = FORM_TAB;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        assert!(dispatch_form(5, 0, form, &[]).held_trim.is_empty());

        form.flags_a = 0;
        form.flags_b = 0;
        form.height = 100;
        let open = dispatch_form(5, 0, form, &[]);
        assert!(open.held_trim.is_empty());
        assert_eq!(open.open_trim.len(), 1);
        assert_eq!(open.open_trim[0].sample, WIDE_BAR);
        assert_eq!(
            open.open_trim[0].corners,
            [(90, 60), (80, 60), (80, 40), (90, 40)]
        );

        form.flags_b = FORM_TAB;
        assert!(dispatch_form(5, 0, form, &[]).open_trim.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_low() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 400;
        form.room_far_y = 80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 0xa, 0xa);
        assert!(drawn.open_low.is_empty());
        assert_eq!(drawn.held_low.len(), 1);
        assert_eq!(drawn.held_low[0].sample, HELD_CAP);
        assert_eq!(
            drawn.held_low[0].corners,
            [(0, 70), (10, 70), (10, 50), (0, 50)]
        );

        form.flags_b = FORM_TAB;
        assert_eq!(dispatch_form(5, 0, form, &[]).held_low.len(), 1);

        form.flags_b = 0;
        form.room_far_y = 400;
        assert!(dispatch_form(5, 0, form, &[]).held_low.is_empty());

        form.room_far_y = 80;
        form.edge_x = 5;
        assert!(dispatch_form(5, 0, form, &[]).held_low.is_empty());

        form.flags_a = 0;
        form.edge_x = 4;
        form.height = 100;
        let open = dispatch_form(5, 0, form, &[]);
        assert!(open.held_low.is_empty());
        assert_eq!(open.open_low.len(), 1);
        assert_eq!(open.open_low[0].sample, HELD_CAP);
        assert_eq!(
            open.open_low[0].corners,
            [(10, 80), (20, 80), (20, 60), (10, 60)]
        );

        form.flags_d = 0x10;
        assert!(dispatch_form(5, 0, form, &[]).open_low.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_sole() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 400;
        form.room_far_y = 80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.held_low.len(), 1);
        assert!(drawn.open_sole.is_empty());
        assert_eq!(drawn.held_sole.len(), 1);
        assert_eq!(drawn.held_sole[0].sample, HELD_CAP);
        assert_eq!(
            drawn.held_sole[0].corners,
            [(10, 80), (10, 70), (30, 70), (30, 80)]
        );

        form.flags_b = FORM_TAB;
        assert_eq!(dispatch_form(5, 0, form, &[]).held_sole.len(), 1);

        form.flags_b = 0;
        form.room_far_y = 400;
        assert!(dispatch_form(5, 0, form, &[]).held_sole.is_empty());

        form.room_far_y = 80;
        form.edge_x = 5;
        assert!(dispatch_form(5, 0, form, &[]).held_sole.is_empty());

        form.flags_a = 0;
        form.edge_x = 4;
        form.height = 100;
        let open = dispatch_form(5, 0, form, &[]);
        assert!(open.held_sole.is_empty());
        assert_eq!(open.open_sole.len(), 1);
        assert_eq!(open.open_sole[0].sample, HELD_CAP);
        assert_eq!(
            open.open_sole[0].corners,
            [(20, 90), (20, 80), (40, 80), (40, 90)]
        );

        form.flags_d = 0x10;
        assert!(dispatch_form(5, 0, form, &[]).open_sole.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_skirt() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 400;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_ne!(drawn.edge_mask & 0xa, 0xa);
        assert!(drawn.held_sole.is_empty());
        assert_eq!(drawn.held_skirt.len(), 1);
        assert_eq!(drawn.held_skirt[0].sample, HELD_FLIP);
        assert_eq!(
            drawn.held_skirt[0].corners,
            [(-10, 90), (-10, 50), (30, 50), (30, 90)]
        );

        form.flags_b = FORM_TAB;
        assert_eq!(dispatch_form(5, 0, form, &[]).held_skirt.len(), 1);

        form.flags_b = 0;
        form.edge_x = 5;
        assert_eq!(dispatch_form(5, 0, form, &[]).held_skirt.len(), 1);

        form.edge_x = 4;
        form.room_far_y = 80;
        let both = dispatch_form(5, 0, form, &[]);
        assert_eq!(both.edge_mask & 0xa, 0xa);
        assert!(both.held_skirt.is_empty());
        assert_eq!(both.held_sole.len(), 1);

        form.room_far_y = 400;
        form.flags_a = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_skirt.is_empty());

        form.flags_a = FORM_HOLD;
        form.flags_d = 0x10;
        assert!(dispatch_form(5, 0, form, &[]).held_skirt.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_floor() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 5;
        form.edge_y = 5;
        form.room_far_x = 400;
        form.room_far_y = 400;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert!(drawn.open_floor.is_empty());
        assert_eq!(drawn.held_floor.len(), 1);
        assert_eq!(drawn.held_floor[0].sample, WIDE_BAR);
        assert_eq!(
            drawn.held_floor[0].corners,
            [(30, 80), (30, 70), (70, 70), (70, 80)]
        );

        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_y = 80;
        let both = dispatch_form(5, 0, form, &[]);
        assert_eq!(both.held_sole.len(), 1);
        assert_eq!(both.held_floor.len(), 1);

        form.flags_b = FORM_TAB;
        assert_eq!(dispatch_form(5, 0, form, &[]).held_floor.len(), 1);

        form.flags_a = 0;
        form.flags_b = 0;
        form.height = 100;
        form.room_far_y = 400;
        let open = dispatch_form(5, 0, form, &[]);
        assert!(open.held_floor.is_empty());
        assert_eq!(open.open_floor.len(), 1);
        assert_eq!(open.open_floor[0].sample, WIDE_BAR);
        assert_eq!(
            open.open_floor[0].corners,
            [(40, 90), (40, 80), (60, 80), (60, 90)]
        );

        form.flags_d = 0x10;
        assert!(dispatch_form(5, 0, form, &[]).open_floor.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_rear() {
        let mut form = place(FORM_HOLD, FORM_TAB, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        form.room_far_y = 80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 0xc, 0xc);
        assert_eq!(drawn.held_floor.len(), 1);
        assert!(drawn.open_rear.is_empty());
        assert_eq!(drawn.held_rear.len(), 1);
        assert_eq!(drawn.held_rear[0].sample, HELD_END);
        assert_eq!(
            drawn.held_rear[0].corners,
            [(100, 50), (100, 70), (80, 70), (80, 50)]
        );

        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_rear.is_empty());

        form.flags_b = FORM_TAB;
        form.room_far_x = 400;
        assert!(dispatch_form(5, 0, form, &[]).held_rear.is_empty());

        form.room_far_x = 100;
        form.room_far_y = 400;
        assert!(dispatch_form(5, 0, form, &[]).held_rear.is_empty());

        form.flags_a = 0;
        form.room_far_y = 80;
        form.height = 100;
        let open = dispatch_form(5, 0, form, &[]);
        assert!(open.held_rear.is_empty());
        assert_eq!(open.open_rear.len(), 1);
        assert_eq!(open.open_rear[0].sample, HELD_END);
        assert_eq!(
            open.open_rear[0].corners,
            [(90, 60), (90, 80), (70, 80), (70, 60)]
        );

        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).open_rear.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_lip() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        form.room_far_y = 80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 0xc, 0xc);
        assert!(drawn.held_rear.is_empty());
        assert!(drawn.open_lip.is_empty());
        assert_eq!(drawn.held_lip.len(), 1);
        assert_eq!(drawn.held_lip[0].sample, HELD_CAP);
        assert_eq!(
            drawn.held_lip[0].corners,
            [(100, 70), (90, 70), (90, 50), (100, 50)]
        );

        form.flags_b = FORM_TAB;
        let tabbed = dispatch_form(5, 0, form, &[]);
        assert!(tabbed.held_lip.is_empty());
        assert_eq!(tabbed.held_rear.len(), 1);

        form.flags_b = 0;
        form.room_far_x = 400;
        assert!(dispatch_form(5, 0, form, &[]).held_lip.is_empty());

        form.room_far_x = 100;
        form.flags_a = 0;
        form.height = 100;
        let open = dispatch_form(5, 0, form, &[]);
        assert!(open.held_lip.is_empty());
        assert_eq!(open.open_lip.len(), 1);
        assert_eq!(open.open_lip[0].sample, HELD_CAP);
        assert_eq!(
            open.open_lip[0].corners,
            [(90, 80), (80, 80), (80, 60), (90, 60)]
        );

        form.flags_b = FORM_TAB;
        assert!(dispatch_form(5, 0, form, &[]).open_lip.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_sill() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 100;
        form.room_far_y = 80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.edge_mask & 0xc, 0xc);
        assert_eq!(drawn.held_lip.len(), 1);
        assert!(drawn.held_rear.is_empty());
        assert!(drawn.open_sill.is_empty());
        assert_eq!(drawn.held_sill.len(), 1);
        assert_eq!(drawn.held_sill[0].sample, HELD_CAP);
        assert_eq!(
            drawn.held_sill[0].corners,
            [(90, 80), (90, 70), (70, 70), (70, 80)]
        );

        form.flags_b = FORM_TAB;
        let tabbed = dispatch_form(5, 0, form, &[]);
        assert_eq!(tabbed.held_rear.len(), 1);
        assert!(tabbed.held_lip.is_empty());
        assert_eq!(tabbed.held_sill.len(), 1);
        assert_eq!(
            tabbed.held_sill[0].corners,
            [(90, 80), (90, 70), (70, 70), (70, 80)]
        );

        form.flags_b = 0;
        form.room_far_x = 400;
        assert!(dispatch_form(5, 0, form, &[]).held_sill.is_empty());

        form.room_far_x = 100;
        form.room_far_y = 400;
        assert!(dispatch_form(5, 0, form, &[]).held_sill.is_empty());

        form.room_far_y = 80;
        form.flags_a = 0;
        form.height = 100;
        let open = dispatch_form(5, 0, form, &[]);
        assert!(open.held_sill.is_empty());
        assert_eq!(open.open_lip.len(), 1);
        assert_eq!(open.open_sill.len(), 1);
        assert_eq!(open.open_sill[0].sample, HELD_CAP);
        assert_eq!(
            open.open_sill[0].corners,
            [(80, 90), (80, 80), (60, 80), (60, 90)]
        );

        form.flags_b = FORM_TAB;
        let open_tab = dispatch_form(5, 0, form, &[]);
        assert!(open_tab.open_lip.is_empty());
        assert_eq!(open_tab.open_rear.len(), 1);
        assert_eq!(open_tab.open_sill.len(), 1);
        assert_eq!(
            open_tab.open_sill[0].corners,
            [(80, 90), (80, 80), (60, 80), (60, 90)]
        );

        form.flags_d = 0x10;
        assert!(dispatch_form(5, 0, form, &[]).open_sill.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_drape() {
        let mut form = place(FORM_HOLD, FORM_TAB, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 400;
        form.room_far_y = 80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_ne!(drawn.edge_mask & 0xc, 0xc);
        assert!(drawn.held_sill.is_empty());
        assert!(drawn.held_rear.is_empty());
        assert_eq!(drawn.held_drape.len(), 1);
        assert_eq!(drawn.held_drape[0].sample, HELD_OVER);
        assert_eq!(
            drawn.held_drape[0].corners,
            [(110, 90), (110, 50), (70, 50), (70, 90)]
        );

        form.room_far_x = 100;
        form.room_far_y = 400;
        let low = dispatch_form(5, 0, form, &[]);
        assert_ne!(low.edge_mask & 0xc, 0xc);
        assert!(low.held_sill.is_empty());
        assert_eq!(low.held_drape.len(), 1);

        form.room_far_y = 80;
        let both = dispatch_form(5, 0, form, &[]);
        assert_eq!(both.edge_mask & 0xc, 0xc);
        assert!(both.held_drape.is_empty());
        assert_eq!(both.held_sill.len(), 1);
        assert_eq!(both.held_rear.len(), 1);

        form.room_far_x = 400;
        form.flags_b = 0;
        assert!(dispatch_form(5, 0, form, &[]).held_drape.is_empty());

        form.flags_a = 0;
        form.flags_b = FORM_TAB;
        form.height = 100;
        let open = dispatch_form(5, 0, form, &[]);
        assert!(open.held_drape.is_empty());
        assert_eq!(open.open_sill.len(), 1);

        form.flags_d = 0x10;
        assert!(dispatch_form(5, 0, form, &[]).held_drape.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_held_fold() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 400;
        form.room_far_y = 80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_ne!(drawn.edge_mask & 0xc, 0xc);
        assert!(drawn.held_drape.is_empty());
        assert!(drawn.held_sill.is_empty());
        assert_eq!(drawn.held_fold.len(), 1);
        assert_eq!(drawn.held_fold[0].sample, HELD_FLIP);
        assert_eq!(
            drawn.held_fold[0].corners,
            [(110, 90), (110, 50), (70, 50), (70, 90)]
        );

        form.flags_b = FORM_TAB;
        let tabbed = dispatch_form(5, 0, form, &[]);
        assert!(tabbed.held_fold.is_empty());
        assert_eq!(tabbed.held_drape.len(), 1);
        assert_eq!(tabbed.held_drape[0].sample, HELD_OVER);

        form.flags_b = 0;
        form.room_far_x = 100;
        form.room_far_y = 80;
        let both = dispatch_form(5, 0, form, &[]);
        assert_eq!(both.edge_mask & 0xc, 0xc);
        assert!(both.held_fold.is_empty());
        assert_eq!(both.held_lip.len(), 1);
        assert_eq!(both.held_sill.len(), 1);

        form.room_far_x = 400;
        form.flags_a = 0;
        form.height = 100;
        let open = dispatch_form(5, 0, form, &[]);
        assert!(open.held_fold.is_empty());
        assert_eq!(open.open_sill.len(), 1);

        form.flags_d = 0x10;
        assert!(dispatch_form(5, 0, form, &[]).held_fold.is_empty());
    }

    #[test]
    fn a_dispatch_draws_the_band_close() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.close_mark = -4;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 400;
        form.room_far_y = 80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.held_fold.len(), 1);
        assert_eq!(drawn.late_close.len(), 1);
        assert_eq!(drawn.band_close.len(), 1);
        assert_eq!(drawn.band_close[0].sample, FORM_CLOSE_INK);
        assert_eq!(
            drawn.band_close[0].corners,
            [(100, 0), (80, 0), (80, 20), (100, 20)]
        );

        form.room_far_x = 100;
        form.flags_b = FORM_TAB;
        let both = dispatch_form(5, 0, form, &[]);
        assert_eq!(both.held_sill.len(), 1);
        assert_eq!(both.band_close.len(), 1);

        form.close_mark = 0;
        assert!(dispatch_form(5, 0, form, &[]).band_close.is_empty());
        form.close_mark = -3;
        assert!(dispatch_form(5, 0, form, &[]).band_close.is_empty());

        form.close_mark = -4;
        form.flags_d = 0x10;
        let plain = dispatch_form(5, 0, form, &[]);
        assert!(plain.band_close.is_empty());
        assert_eq!(plain.late_close.len(), 1);

        form.flags_d = 0x80;
        form.flags_b = FORM_BOX;
        let wide = dispatch_form(5, 0, form, &[]);
        assert!(wide.band_close.is_empty());
        assert_eq!(wide.late_close.len(), 1);

        form.flags_a = 0;
        form.flags_b = 0;
        form.height = 100;
        let open = dispatch_form(5, 0, form, &[]);
        assert!(open.held_fold.is_empty());
        assert_eq!(open.open_sill.len(), 1);
        assert_eq!(open.band_close.len(), 1);
        assert_eq!(
            open.band_close[0].corners,
            [(100, 0), (80, 0), (80, 20), (100, 20)]
        );
    }

    #[test]
    fn a_dispatch_counts_the_band_call() {
        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 400;
        form.room_far_y = 80;
        let drawn = dispatch_form(5, 0, form, &[]);
        assert_eq!(drawn.held_fold.len(), 1);
        assert!(drawn.band_close.is_empty());
        assert_eq!(drawn.band_calls, 1);

        form.close_mark = -4;
        let marked = dispatch_form(5, 0, form, &[]);
        assert_eq!(marked.band_close.len(), 1);
        assert_eq!(marked.band_calls, 1);

        form.close_mark = 0;
        form.flags_a = 0;
        form.height = 100;
        let open = dispatch_form(5, 0, form, &[]);
        assert_eq!(open.open_sill.len(), 1);
        assert_eq!(open.band_calls, 1);

        form.flags_a = FORM_HOLD;
        form.flags_d = 0x10;
        assert_eq!(dispatch_form(5, 0, form, &[]).band_calls, 0);

        form.flags_d = 0x80;
        form.flags_b = FORM_BOX;
        assert_eq!(dispatch_form(5, 0, form, &[]).band_calls, 0);
    }

    #[test]
    fn a_dispatch_restores_the_frame() {
        let plain = dispatch_form(5, 0, place(0, 0, 0, 0, 0, 0), &[]);
        assert_eq!(plain.band_calls, 0);
        assert_eq!(plain.frame_calls, 1);
        assert!(plain.frame_restored);

        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 400;
        form.room_far_y = 80;
        let band = dispatch_form(5, 0, form, &[]);
        assert_eq!(band.band_calls, 1);
        assert_eq!(band.held_fold.len(), 1);
        assert_eq!(band.frame_calls, 1);
        assert!(band.frame_restored);

        form.flags_d = 0x10;
        let other = dispatch_form(5, 0, form, &[]);
        assert_eq!(other.band_calls, 0);
        assert_eq!(other.frame_calls, 1);
        assert!(other.frame_restored);

        form.flags_d = 0x80;
        form.flags_b = FORM_BOX;
        let wide = dispatch_form(5, 0, form, &[]);
        assert_eq!(wide.band_calls, 0);
        assert_eq!(wide.frame_calls, 1);
        assert!(wide.frame_restored);
    }

    #[test]
    fn a_dispatch_releases_the_frame_text() {
        let plain = dispatch_form(5, 0, place(0, 0, 0, 0, 0, 0), &[]);
        assert!(plain.frame_restored);
        assert_eq!(plain.frame_text, 0);
        assert_eq!(plain.frame_release, 1);
        assert!(!plain.frame_freed);
        assert!(!plain.fault_taken);

        let mut form = place(FORM_HOLD, 0, 0, 0, 0, 0);
        form.flags_d = 0x80;
        form.edge_x = 4;
        form.edge_y = 4;
        form.room_far_x = 400;
        form.room_far_y = 80;
        let band = dispatch_form(5, 0, form, &[]);
        assert_eq!(band.held_fold.len(), 1);
        assert_eq!(band.frame_text, 0);
        assert_eq!(band.frame_release, 1);
        assert!(!band.frame_freed);
        assert!(!band.fault_taken);

        form.flags_d = 0x10;
        let other = dispatch_form(5, 0, form, &[]);
        assert_eq!(other.band_calls, 0);
        assert_eq!(other.frame_release, 1);
        assert!(!other.frame_freed);
        assert!(!other.fault_taken);

        form.flags_d = 0x80;
        form.flags_b = FORM_BOX;
        let wide = dispatch_form(5, 0, form, &[]);
        assert_eq!(wide.band_calls, 0);
        assert_eq!(wide.frame_text, 0);
        assert_eq!(wide.frame_release, 1);
        assert!(!wide.frame_freed);
        assert!(!wide.fault_taken);
    }

    #[test]
    fn a_span_bar_draws_three_quads() {
        let bar = draw_span_bar(5, 10.0, 20.0, 100.0, 4, SPAN_VIEW_UNSET);
        assert_eq!(bar.prep_calls, 1);
        assert_eq!(bar.binds, [SPAN_BAR, SPAN_CAP, SPAN_CAP]);
        let expect: [[(f32, f32); 4]; 3] = [
            [(15.0_f32, 20.0), (15.0, 28.0), (105.0, 28.0), (105.0, 20.0)],
            [(15.0, 20.0), (15.0, 28.0), (10.0, 28.0), (10.0, 20.0)],
            [(105.0, 20.0), (105.0, 28.0), (110.0, 28.0), (110.0, 20.0)],
        ];
        for (got, want) in bar.quads.iter().zip(expect) {
            for ((gx, gy), (wx, wy)) in got.iter().zip(want) {
                assert_eq!(gx.to_bits(), wx.to_bits());
                assert_eq!(gy.to_bits(), wy.to_bits());
            }
        }

        let neg = draw_span_bar(5, 10.0, 20.0, 100.0, -3, SPAN_VIEW_UNSET);
        let thick = neg.quads[0][1].1 - neg.quads[0][0].1;
        assert_eq!(thick.to_bits(), (-6.0_f32).to_bits());
        let left = neg.quads[1][1].1 - neg.quads[1][0].1;
        assert_eq!(left.to_bits(), (-6.0_f32).to_bits());
    }

    #[test]
    fn a_span_bar_places_the_view() {
        let view = SpanView {
            shift_x: 2.0,
            shift_y: 100.0,
            scale_x: 0.5,
            scale_y: 2.0,
            colour: 17,
            flag: 0,
            mark: 0,
            cursor: 0,
        };
        let bar = draw_span_bar(5, 10.0, 20.0, 100.0, 4, view);
        assert_eq!(bar.quads[0][0].0.to_bits(), 15.0_f32.to_bits());
        assert_eq!(bar.quads[0][0].1.to_bits(), 20.0_f32.to_bits());
        assert_eq!(bar.placed[0][0].0.to_bits(), 8.5_f32.to_bits());
        assert_eq!(bar.placed[0][0].1.to_bits(), 160.0_f32.to_bits());
        assert_eq!(bar.colour, 17);
        assert_eq!(bar.colour_writes, 12);

        let cold = draw_span_bar(5, 10.0, 20.0, 100.0, 4, SPAN_VIEW_UNSET);
        assert_eq!(cold.placed[0][0].0.to_bits(), 0.0_f32.to_bits());
        assert_eq!(cold.placed[0][0].1.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(cold.colour, 0);
        assert_eq!(cold.colour_writes, 12);
    }

    #[test]
    fn a_span_bar_stores_the_mark_when_bit_4_is_set() {
        let mut view = SPAN_VIEW_UNSET;
        view.flag = 4;
        view.mark = 9;
        view.cursor = 3;
        let bar = draw_span_bar(5, 10.0, 20.0, 100.0, 4, view);
        assert_eq!(bar.mark_count, 12);
        assert_eq!(bar.cursor_after, 15);
        for index in 0..12 {
            assert_eq!(bar.mark_indexes[index], 3 + index as u32);
            assert_eq!(bar.marks[index], 9);
        }

        view.flag = 5;
        let either = draw_span_bar(5, 10.0, 20.0, 100.0, 4, view);
        assert_eq!(either.mark_count, 12);
        assert_eq!(either.marks[0], 9);

        view.flag = 0x8000_0004;
        let high = draw_span_bar(5, 10.0, 20.0, 100.0, 4, view);
        assert_eq!(high.mark_count, 12);
        assert_eq!(high.mark_indexes[0], 3);

        view.flag = 0;
        let clear = draw_span_bar(5, 10.0, 20.0, 100.0, 4, view);
        assert_eq!(clear.mark_count, 0);
        assert_eq!(clear.cursor_after, 15);
        assert_eq!(clear.marks, [0; 12]);

        view.flag = 1;
        let other = draw_span_bar(5, 10.0, 20.0, 100.0, 4, view);
        assert_eq!(other.mark_count, 0);
        assert_eq!(other.cursor_after, 15);

        view.flag = 4;
        view.cursor = 0xffff_fffe;
        let wrapped = draw_span_bar(5, 10.0, 20.0, 100.0, 4, view);
        assert_eq!(wrapped.mark_indexes[0], 0xffff_fffe);
        assert_eq!(wrapped.mark_indexes[1], 0xffff_ffff);
        assert_eq!(wrapped.mark_indexes[2], 0);
        assert_eq!(wrapped.marks[2], 9);
        assert_eq!(wrapped.cursor_after, 10);
    }

    #[test]
    fn a_span_square_draws_one_quad() {
        let view = SpanView {
            colour: 17,
            flag: 4,
            mark: 9,
            cursor: 99,
            ..SPAN_VIEW_UNSET
        };
        let square = draw_span_square(10.0, 20.0, 4, view);
        assert_eq!(square.mode_calls, 1);
        assert_eq!(square.mode, 2);
        assert_eq!(square.prep_calls, 1);
        assert_eq!(square.begin_calls, 1);
        assert_eq!(square.bind_calls, 1);
        assert_eq!(square.bind, SPAN_SQUARE);
        assert_eq!(square.colour, 17);
        assert_eq!(square.colour_writes, 4);
        assert_eq!(square.end_calls, 1);
        let expect: [(f32, f32); 4] = [(3.0, 29.0), (3.0, 13.0), (19.0, 13.0), (19.0, 29.0)];
        for ((gx, gy), (wx, wy)) in square.quads.iter().zip(expect) {
            assert_eq!(gx.to_bits(), wx.to_bits());
            assert_eq!(gy.to_bits(), wy.to_bits());
        }
        assert_eq!(square.mark_count, 4);
        assert_eq!(square.mark_indexes, [0, 1, 2, 3]);
        assert_eq!(square.marks, [9; 4]);
        assert_eq!(square.cursor_after, 4);

        let six = draw_span_square(10.0, 20.0, 6, view);
        let expect: [(f32, f32); 4] = [(-1.0, 33.0), (-1.0, 9.0), (23.0, 9.0), (23.0, 33.0)];
        for ((gx, gy), (wx, wy)) in six.quads.iter().zip(expect) {
            assert_eq!(gx.to_bits(), wx.to_bits());
            assert_eq!(gy.to_bits(), wy.to_bits());
        }

        let minus = draw_span_square(10.0, 20.0, -1, view);
        assert_eq!(minus.quads[0].0.to_bits(), 0xdf00_0000);
        assert_eq!(minus.quads[0].1.to_bits(), 0xdf00_0000);

        let clear = draw_span_square(10.0, 20.0, 4, SpanView { flag: 0, ..view });
        assert_eq!(clear.mark_count, 0);
        assert_eq!(clear.marks, [0; 4]);
        assert_eq!(clear.mark_indexes, [0; 4]);
        assert_eq!(clear.cursor_after, 4);
    }

    #[test]
    fn a_line_chip_draws_four_units() {
        let inks = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];
        let chip = draw_line_chip(5, 10, 20, 0, inks);
        assert_eq!(chip.mode_calls, 1);
        assert_eq!(chip.mode, 2);
        assert_eq!(chip.begin_calls, 1);
        assert_eq!(chip.prep_calls, 1);
        assert_eq!(chip.bind_calls, 1);
        assert_eq!(chip.samples[0], LINE_CHIP);
        assert_eq!(chip.draw_calls, 1);
        assert_eq!(chip.ink_index, 2);
        assert_eq!(chip.colour, [9, 10, 11, 12]);
        assert_eq!(chip.colour_writes, 4);
        assert_eq!(chip.end_calls, 1);
        let expect: [(f32, f32); 4] = [(10.0, 40.0), (10.0, 20.0), (30.0, 20.0), (30.0, 40.0)];
        for ((gx, gy), (wx, wy)) in chip.quads[0].iter().zip(expect) {
            assert_eq!(gx.to_bits(), wx.to_bits());
            assert_eq!(gy.to_bits(), wy.to_bits());
        }
        assert_eq!(chip.quads[1], [(0.0, 0.0); 4]);

        let alt = draw_line_chip(5, 10, 20, 0x40, inks);
        assert_eq!(alt.bind_calls, 2);
        assert_eq!(alt.samples[0], LINE_CHIP);
        assert_eq!(alt.samples[1], LINE_CHIP_ALT);
        assert_eq!(alt.draw_calls, 2);
        assert_eq!(alt.colour_writes, 8);
        assert_eq!(alt.quads[1], alt.quads[0]);

        let first = draw_line_chip(5, 10, 20, 0x10, inks);
        assert_eq!(first.ink_index, 0);
        assert_eq!(first.colour, [1, 2, 3, 4]);

        let mode = draw_line_chip(5, 10, 20, 0x1_0000, inks);
        assert_eq!(mode.ink_index, 1);
        assert_eq!(mode.colour, [5, 6, 7, 255]);

        let both = draw_line_chip(5, 10, 20, 0x1_0040, inks);
        assert_eq!(both.ink_index, 1);
        assert_eq!(both.colour, [5, 6, 7, 255]);
        assert_eq!(both.draw_calls, 2);
    }

    fn bevel_arm(fill: [[u8; 4]; 2], border: [[u8; 4]; 2], outline: [u8; 4]) -> BevelArm {
        BevelArm {
            fill,
            border,
            outline,
        }
    }

    #[test]
    fn a_bevel_box_draws_two_fills_six_border_lines_and_four_outline_lines() {
        let ink = BevelInk {
            raised: bevel_arm([[200; 4], [201; 4]], [[210; 4], [211; 4]], [212; 4]),
            pressed: bevel_arm([[100; 4], [101; 4]], [[110; 4], [111; 4]], [112; 4]),
            idle: bevel_arm(
                [[0, 0, 0, 0], [100, 100, 100, 100]],
                [[10, 10, 10, 10], [110, 110, 110, 110]],
                [7, 7, 7, 7],
            ),
        };
        let bevel = draw_bevel(
            BevelPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                state: 0,
            },
            &ink,
        );
        assert_eq!(bevel.begins, 3);
        assert_eq!(bevel.binds, 12);
        assert_eq!(bevel.ends, 3);

        let fill0: [(f32, f32); 4] = [(108.0, 22.0), (12.0, 22.0), (12.0, 60.0), (108.0, 60.0)];
        let fill1: [(f32, f32); 4] = [(108.0, 60.0), (12.0, 60.0), (12.0, 98.0), (108.0, 98.0)];
        for ((gx, gy), (wx, wy)) in bevel.fill[0].corners.iter().zip(fill0) {
            assert_eq!(gx.to_bits(), wx.to_bits());
            assert_eq!(gy.to_bits(), wy.to_bits());
        }
        for ((gx, gy), (wx, wy)) in bevel.fill[1].corners.iter().zip(fill1) {
            assert_eq!(gx.to_bits(), wx.to_bits());
            assert_eq!(gy.to_bits(), wy.to_bits());
        }
        assert_eq!(bevel.fill[0].inks, [[100; 4], [100; 4], [60; 4], [60; 4]]);
        assert_eq!(
            bevel.fill[1].inks,
            [[0, 0, 0, 0], [0, 0, 0, 0], [40; 4], [40; 4]]
        );

        let border: [(f32, f32); 12] = [
            (108.5, 21.5),
            (11.5, 21.5),
            (11.5, 21.5),
            (11.5, 60.5),
            (11.5, 60.5),
            (11.5, 98.5),
            (11.5, 98.5),
            (108.5, 98.5),
            (108.5, 98.5),
            (108.5, 60.0),
            (108.5, 60.0),
            (108.5, 21.5),
        ];
        for (line, pair) in bevel.border.iter().zip(border.chunks(2)) {
            for ((gx, gy), (wx, wy)) in line.ends.iter().zip(pair) {
                assert_eq!(gx.to_bits(), wx.to_bits());
                assert_eq!(gy.to_bits(), wy.to_bits());
            }
        }
        let border_inks = [
            [110; 4], [110; 4], [110; 4], [75; 4], [45; 4], [10; 4], [10; 4], [10; 4], [10; 4],
            [45; 4], [75; 4], [110; 4],
        ];
        let got: Vec<[u8; 4]> = bevel.border.iter().flat_map(|line| line.inks).collect();
        assert_eq!(got, border_inks);

        let outline: [(f32, f32); 8] = [
            (109.5, 20.5),
            (10.5, 20.5),
            (10.5, 20.5),
            (10.5, 99.5),
            (10.5, 99.5),
            (109.5, 99.5),
            (109.5, 99.5),
            (109.5, 20.5),
        ];
        for (line, pair) in bevel.outline.iter().zip(outline.chunks(2)) {
            for ((gx, gy), (wx, wy)) in line.ends.iter().zip(pair) {
                assert_eq!(gx.to_bits(), wx.to_bits());
                assert_eq!(gy.to_bits(), wy.to_bits());
            }
            assert_eq!(line.inks, [[7; 4], [7; 4]]);
        }

        let raised = draw_bevel(
            BevelPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                state: 0x40,
            },
            &ink,
        );
        assert_eq!(raised.fill[0].inks[0], [201; 4]);

        let pressed = draw_bevel(
            BevelPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                state: 0x10,
            },
            &ink,
        );
        assert_eq!(pressed.fill[0].inks[0], [101; 4]);

        let both = draw_bevel(
            BevelPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                state: 0x50,
            },
            &ink,
        );
        assert_eq!(both.fill[0].inks[0], [201; 4]);

        let shipped = draw_bevel(
            BevelPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                state: 0,
            },
            &BEVEL_INK,
        );
        assert!(
            shipped
                .fill
                .iter()
                .flat_map(|quad| quad.inks)
                .all(|ink| ink == [0, 0, 0, 0])
        );
        let shaded = draw_bevel(
            BevelPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                state: 0x10,
            },
            &BEVEL_INK,
        );
        assert!(
            shaded
                .border
                .iter()
                .flat_map(|line| line.inks)
                .all(|ink| ink == [0, 0, 0, 0xe0])
        );
    }

    #[test]
    fn a_capped_span_draws_two_caps_and_a_middle() {
        let span = draw_capped_span(5, 10.0, 110.0, 28.0, 17, SPAN_VIEW_UNSET);
        assert_eq!(span.mode_calls, 1);
        assert_eq!(span.mode, 2);
        assert_eq!(span.begin_calls, 1);
        assert_eq!(span.prep_calls, 1);
        assert_eq!(span.ink_binds, 1);
        assert_eq!(span.bind_calls, 3);
        assert_eq!(span.binds, [SPAN_END, SPAN_END, SPAN_BAR]);
        assert_eq!(span.colour, 17);
        assert_eq!(span.colour_writes, 12);
        assert_eq!(span.end_calls, 1);
        let expect: [[(f32, f32); 4]; 3] = [
            [(0.0, 18.0), (0.0, 38.0), (30.0, 38.0), (30.0, 18.0)],
            [(120.0, 18.0), (120.0, 38.0), (90.0, 38.0), (90.0, 18.0)],
            [(30.0, 28.0), (30.0, 38.0), (90.0, 38.0), (90.0, 28.0)],
        ];
        for (quad, want) in span.quads.iter().zip(expect) {
            for ((gx, gy), (wx, wy)) in quad.iter().zip(want) {
                assert_eq!(gx.to_bits(), wx.to_bits());
                assert_eq!(gy.to_bits(), wy.to_bits());
            }
        }

        let view = SpanView {
            shift_x: 2.0,
            shift_y: 100.0,
            scale_x: 0.5,
            scale_y: 2.0,
            ..SPAN_VIEW_UNSET
        };
        let placed = draw_capped_span(5, 10.0, 110.0, 28.0, 17, view);
        assert_eq!(placed.quads[0][0].0.to_bits(), 0.0_f32.to_bits());
        assert_eq!(placed.quads[0][0].1.to_bits(), 18.0_f32.to_bits());
        assert_eq!(placed.placed[0][0].0.to_bits(), 1.0_f32.to_bits());
        assert_eq!(placed.placed[0][0].1.to_bits(), 164.0_f32.to_bits());

        let zero = draw_capped_span(5, 0.0, 0.0, 0.0, 0, SPAN_VIEW_UNSET);
        assert_eq!(zero.quads[0][0].1.to_bits(), (-10.0_f32).to_bits());
        assert_eq!(zero.quads[0][1].1.to_bits(), 10.0_f32.to_bits());
    }

    #[test]
    fn a_line_label_aligns_its_text() {
        let metrics = LabelMetrics {
            width: 40,
            line_height: 24,
            base: 18,
        };
        let inks = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];
        let place = |style: u32, state: u32, inset: i16| LabelPlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            inset,
            style,
            state,
        };

        let left = place_line_label(place(0x1000, 0, 0), metrics, 5, inks);
        assert_eq!((left.x, left.y), (-35, 23));
        let centred = place_line_label(place(0x2000, 0, 0), metrics, 5, inks);
        assert_eq!(centred.x, 39);
        let inside = place_line_label(place(0x3000, 0, 0), metrics, 5, inks);
        assert_eq!(inside.x, 65);
        let outside = place_line_label(place(0x4000, 0, 0), metrics, 5, inks);
        assert_eq!(outside.x, 115);
        let plain = place_line_label(place(0, 0, 0), metrics, 5, inks);
        assert_eq!((plain.x, plain.y), (13, 23));
        let inset = place_line_label(place(0, 0, 7), metrics, 5, inks);
        assert_eq!(inset.x, 20);

        let up = place_line_label(place(0x100, 0, 0), metrics, 5, inks);
        assert_eq!(up.y, 18);
        let middle = place_line_label(place(0x200, 0, 0), metrics, 5, inks);
        assert_eq!(middle.y, 73);
        let base = place_line_label(place(0x300, 0, 0), metrics, 5, inks);
        assert_eq!(base.y, 117);
        let bottom = place_line_label(place(0x400, 0, 0), metrics, 5, inks);
        assert_eq!(bottom.y, 100);

        let first = place_line_label(place(0, 0x10, 0), metrics, 5, inks);
        assert_eq!(first.ink_index, Some(0));
        assert_eq!(first.colour, Some([1, 2, 3, 4]));
        assert!(!first.coloured);
        assert_eq!(first.text_calls, 1);

        let coloured = place_line_label(place(0, 0x1_0040, 0), metrics, 5, inks);
        assert!(coloured.coloured);
        assert_eq!(coloured.ink_index, None);
        assert_eq!(coloured.colour, None);
        assert_eq!(coloured.y, 24);

        let third = place_line_label(place(0, 0x60, 0), metrics, 5, inks);
        assert_eq!(third.ink_index, Some(2));
        assert_eq!(third.colour, Some([9, 10, 11, 12]));
        assert_eq!(third.y, 24);

        let second = place_line_label(place(0, 0, 0), metrics, 5, inks);
        assert_eq!(second.ink_index, Some(1));
        assert_eq!(second.colour, Some([5, 6, 7, 8]));
    }

    #[test]
    fn a_line_rail_draws_a_track_and_a_thumb() {
        let inks = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];
        let rail = draw_line_rail(
            5,
            RailPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                state: 0,
            },
            inks,
            SPAN_VIEW_UNSET,
        );
        assert_eq!(rail.mode_calls, 1);
        assert_eq!(rail.mode, 2);
        assert_eq!(rail.begin_calls, 1);
        assert_eq!(rail.prep_calls, 1);
        assert_eq!(rail.bind_calls, 4);
        assert_eq!(rail.binds, [SPAN_END, SPAN_END, SPAN_BAR, LINE_RAIL]);
        assert_eq!(rail.colour_writes, 16);
        assert_eq!(rail.end_calls, 1);
        assert_eq!(rail.track_ink, [5, 6, 7, 8]);
        assert_eq!(rail.thumb_ink, [5, 6, 7, 8]);
        let expect: [[(f32, f32); 4]; 4] = [
            [(0.0, 85.0), (0.0, 105.0), (30.0, 105.0), (30.0, 85.0)],
            [(100.0, 85.0), (100.0, 105.0), (90.0, 105.0), (90.0, 85.0)],
            [(30.0, 95.0), (30.0, 105.0), (90.0, 105.0), (90.0, 95.0)],
            [(105.0, 30.0), (105.0, 40.0), (95.0, 40.0), (95.0, 30.0)],
        ];
        for (quad, want) in rail.quads.iter().zip(expect) {
            for ((gx, gy), (wx, wy)) in quad.iter().zip(want) {
                assert_eq!(gx.to_bits(), wx.to_bits());
                assert_eq!(gy.to_bits(), wy.to_bits());
            }
        }

        let first = draw_line_rail(
            5,
            RailPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                state: 0x10,
            },
            inks,
            SPAN_VIEW_UNSET,
        );
        assert_eq!(first.track_ink, [1, 2, 3, 4]);

        let mode = draw_line_rail(
            5,
            RailPlace {
                x: 10,
                y: 20,
                w: 100,
                h: 80,
                state: 0x1_0000,
            },
            inks,
            SPAN_VIEW_UNSET,
        );
        assert_eq!(mode.track_ink, [5, 6, 7, 8]);
        assert_eq!(mode.thumb_ink, [9, 10, 11, 255]);
    }

    #[test]
    fn a_line_scale_stores_three_heights() {
        let one = scale_line(1.0);
        assert_eq!(one.heights, [80, 104, 128]);
        assert_eq!(one.scale_calls, 1);

        let half = scale_line(0.5);
        assert_eq!(half.heights, [40, 52, 64]);

        let double = scale_line(2.0);
        assert_eq!(double.heights, [160, 208, 0]);
    }

    #[test]
    fn a_line_hit_returns_the_original_codes() {
        let place = HitPlace {
            flags: 0x10,
            control: 2,
            right: 100,
            left: 10,
            top: 20,
            bottom: 60,
        };
        assert_eq!(hit_line(place, 90, 10, 5, false), -4);
        assert_eq!(hit_line(place, 50, 10, 5, false), 0);
        assert_eq!(
            hit_line(
                HitPlace {
                    control: 1,
                    ..place
                },
                10,
                10,
                5,
                false
            ),
            -3
        );
        assert_eq!(hit_line(place, 90, 25, 5, true), -4);

        let strip = HitPlace {
            flags: 0,
            control: 0x200,
            right: 100,
            left: 10,
            top: 20,
            bottom: 60,
        };
        assert_eq!(hit_line(strip, 50, 30, 5, false), -8);
        assert_eq!(hit_line(strip, 50, 70, 5, false), -10);
        assert_eq!(hit_line(strip, 50, 90, 5, false), 0);
        assert_eq!(hit_line(strip, 50, 15, 5, false), -9);
        assert_eq!(hit_line(strip, 50, -5, 5, false), 0);
        assert_eq!(hit_line(strip, 5, 30, 5, false), 0);
        assert_eq!(hit_line(strip, 50, 30, 5, true), -8);
    }

    #[test]
    fn a_line_shade_draws_six_quads_and_an_inner_rect() {
        let place = LineShadePlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            state: 0,
            inner_ink: [1, 2, 3, 4],
            side_inset: 15,
        };
        let inks = LineShadeInks {
            normal: [0x99, 0xaa, 0xbb, 0xcc],
            special: [0xde, 0xad, 0xbe, 0xef],
        };
        let shade = draw_line_shade(5, place, inks, SPAN_VIEW_UNSET);
        assert_eq!(shade.modes, [2, 1, 0x10]);
        assert_eq!(shade.mode_calls, 3);
        assert_eq!(shade.prepare_calls, 1);
        assert_eq!(shade.outer_ink_bind_calls, 1);
        assert_eq!(shade.inner_ink_bind_calls, 1);
        assert_eq!(shade.sample_bind_calls, 6);
        assert_eq!(shade.begin_calls, 2);
        assert_eq!(shade.end_calls, 2);
        assert_eq!(shade.vertex_calls, 28);
        assert_eq!(shade.colour_writes, 28);
        assert_eq!(shade.batch_count, 2);
        assert_eq!(shade.place_inset_after, 5);
        assert!(!shade.special_ink_used);
        assert_eq!(shade.outer_ink, [0x99, 0xaa, 0xbb, 0xcc]);
        assert_eq!(shade.inner_ink, [1, 2, 3, 0xff]);

        let expect: [[(f32, f32); 4]; 6] = [
            [(25.0, 23.0), (25.0, 13.0), (10.0, 13.0), (10.0, 23.0)],
            [(25.0, 23.0), (25.0, 13.0), (95.0, 13.0), (95.0, 23.0)],
            [(95.0, 23.0), (95.0, 13.0), (110.0, 13.0), (110.0, 23.0)],
            [(25.0, 97.0), (25.0, 107.0), (10.0, 107.0), (10.0, 97.0)],
            [(25.0, 97.0), (25.0, 107.0), (95.0, 107.0), (95.0, 97.0)],
            [(95.0, 97.0), (95.0, 107.0), (110.0, 107.0), (110.0, 97.0)],
        ];
        let binds = [SPAN_CAP, SPAN_BAR, SPAN_CAP, SPAN_CAP, SPAN_BAR, SPAN_CAP];
        for (slot, quad) in shade.outer.iter().enumerate() {
            assert_eq!(quad.sample, Some(binds[slot]));
            assert_eq!(quad.ink, [0x99, 0xaa, 0xbb, 0xcc]);
            for ((gx, gy), (wx, wy)) in quad.corners.iter().zip(expect[slot]) {
                assert_eq!(gx.to_bits(), wx.to_bits());
                assert_eq!(gy.to_bits(), wy.to_bits());
            }
        }
        assert_eq!(shade.inner.sample, None);
        assert_eq!(shade.inner.ink, [1, 2, 3, 0xff]);
        let inner: [(f32, f32); 4] = [(105.0, 25.0), (15.0, 25.0), (15.0, 95.0), (105.0, 95.0)];
        for ((gx, gy), (wx, wy)) in shade.inner.corners.iter().zip(inner) {
            assert_eq!(gx.to_bits(), wx.to_bits());
            assert_eq!(gy.to_bits(), wy.to_bits());
        }
        assert_eq!(
            shade.inner_rect,
            LineShadeRect {
                left: 15,
                top: 25,
                width: 90,
                height: 70,
                right: 105,
                bottom: 95,
            }
        );

        let special = draw_line_shade(
            5,
            LineShadePlace {
                state: 0x1_0000,
                ..place
            },
            inks,
            SPAN_VIEW_UNSET,
        );
        assert!(special.special_ink_used);
        assert_eq!(special.outer_ink, [0xde, 0xad, 0xbe, 0xff]);

        let view = SpanView {
            shift_x: 2.0,
            shift_y: 100.0,
            scale_x: 0.5,
            scale_y: 2.0,
            ..SPAN_VIEW_UNSET
        };
        let placed = draw_line_shade(5, place, inks, view);
        assert_eq!(placed.outer[0].placed[0].0.to_bits(), 13.5_f32.to_bits());
        assert_eq!(placed.outer[0].placed[0].1.to_bits(), 154.0_f32.to_bits());
    }

    #[test]
    fn a_dispatch_tints_the_close_edges() {
        let mut form = place(0, 0, FORM_STILL, 0, 0, 0);
        form.close_x = 10;
        form.close_y = 20;
        form.close_mark = -9;
        let nine = dispatch_form(5, 0, form, &[]);
        assert_eq!(nine.close_edges.len(), 1);
        assert_eq!(nine.close_edges[0].sample, LINE_RAIL);
        assert_eq!(nine.close_edges[0].ink, [0xff, 0xcc, 0x99, 0xff]);
        assert_eq!(
            nine.close_edges[0].corners,
            [(16, 40), (16, 30), (26, 30), (26, 40)]
        );

        form.close_mark = -10;
        let ten = dispatch_form(5, 0, form, &[]);
        assert_eq!(
            ten.close_edges[0].corners,
            [(16, 20), (16, 30), (26, 30), (26, 20)]
        );

        form.close_mark = -8;
        let eight = dispatch_form(5, 0, form, &[]);
        assert_eq!(
            eight.close_edges[0].corners,
            [(11, 50), (11, 30), (31, 30), (31, 50)]
        );

        form.close_mark = -4;
        let none = dispatch_form(5, 0, form, &[]);
        assert!(none.close_edges.is_empty());
    }

    #[test]
    fn a_line_open_makes_both_calls() {
        let open = open_line();
        assert_eq!(open.parent_calls, 1);
        assert_eq!(open.texture_calls, 1);
    }

    #[test]
    fn a_skin_renderer_prepares_once() {
        let first = prepare_skin_renderer(false);
        assert_eq!(first.calls, 2);
        assert!(first.prepared);

        let again = prepare_skin_renderer(true);
        assert_eq!(again.calls, 0);
        assert!(again.prepared);
    }

    #[test]
    fn a_skin_alpha_scales_its_two_groups() {
        let unit = scale_skin_alpha(255, 255, 1.0, 1.0);
        assert_eq!(unit.first, [255; 14]);
        assert_eq!(unit.second, [255; 48]);

        let half = scale_skin_alpha(100, 200, 0.5, 0.5);
        assert_eq!(half.first, [50; 14]);
        assert_eq!(half.second, [100; 48]);

        let double = scale_skin_alpha(0, 200, 1.0, 2.0);
        assert_eq!(double.first, [0; 14]);
        assert_eq!(double.second, [144; 48]);
    }

    #[test]
    fn a_skin_label_draws_its_shadow_arm() {
        let metrics = LabelMetrics {
            width: 40,
            line_height: 24,
            base: 18,
        };
        let inks = [
            [1, 2, 3, 4],
            [5, 6, 7, 8],
            [9, 10, 11, 12],
            [13, 14, 15, 16],
        ];
        let place = |state: u32| LabelPlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            inset: 0,
            style: 0,
            state,
        };

        let shadow = place_skin_label(place(0x40), metrics, 5, inks);
        assert_eq!((shadow.x, shadow.y), (13, 23));
        assert_eq!(shadow.text_calls, 2);
        assert_eq!(
            shadow.calls,
            vec![([5, 6, 7, 8], (12, 23)), ([9, 10, 11, 12], (13, 24))]
        );
        assert!(!shadow.coloured);

        let first = place_skin_label(place(0x10), metrics, 5, inks);
        assert_eq!(first.calls, vec![([1, 2, 3, 4], (13, 23))]);
        assert_eq!(first.text_calls, 1);

        let plain = place_skin_label(place(0), metrics, 5, inks);
        assert_eq!(plain.calls, vec![([13, 14, 15, 16], (13, 23))]);
        assert_eq!(plain.text_calls, 1);

        let coloured = place_skin_label(place(0x1_0040), metrics, 5, inks);
        assert!(coloured.coloured);
        assert!(coloured.calls.is_empty());
        assert_eq!(coloured.y, 24);
        assert_eq!(coloured.text_calls, 1);
    }

    #[test]
    fn a_control_rect_switches_on_its_code() {
        let metrics = SkinMetrics { unit: 5, trim: 2 };
        assert_eq!(SkinMetrics::RAIL, 15);
        assert_eq!(SkinMetrics::CLOSE, 17);
        assert_eq!((metrics.unit, metrics.trim), (5, 2));

        let place = ControlPlace {
            x: 100,
            y: 50,
            size: 20,
            field_a: 7,
            field_b: 90,
            flags: 0xa,
        };

        assert_eq!(control_rect(place, -4, metrics), Some([78, 4, 95, 22]));
        assert_eq!(control_rect(place, -6, metrics), Some([75, 4, 92, 22]));
        assert_eq!(control_rect(place, -5, metrics), Some([72, 4, 89, 22]));
        assert_eq!(control_rect(place, -9, metrics), Some([65, 7, 80, 22]));
        assert_eq!(control_rect(place, -10, metrics), Some([65, 15, 80, 30]));
        assert_eq!(control_rect(place, -13, metrics), Some([20, 15, 35, 30]));
        assert_eq!(control_rect(place, -14, metrics), Some([75, 15, 90, 30]));
        assert_eq!(control_rect(place, -1, metrics), None);
        assert_eq!(control_rect(place, 4, metrics), None);
    }

    #[test]
    fn skin_frames_select_both_groups() {
        let place = |style: u32| SkinFramePlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            style,
        };

        let first = skin_frames(place(0x10));
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].rect, [10.0, 20.0, 110.0, 100.0]);
        assert_eq!((first[0].first, first[0].second), (0x9c, 0xb8));

        let swap = skin_frames(place(0x20));
        assert_eq!(swap[0].rect, [10.0, 20.0, 110.0, 100.0]);
        assert_eq!((swap[0].first, swap[0].second), (0xb4, 0x9c));

        let inner = skin_frames(place(0x40));
        assert_eq!(inner[0].rect, [11.0, 21.0, 109.0, 99.0]);
        assert_eq!((inner[0].first, inner[0].second), (0x9c, 0xb8));

        let inner_swap = skin_frames(place(0x80));
        assert_eq!(inner_swap[0].rect, [11.0, 21.0, 109.0, 99.0]);
        assert_eq!((inner_swap[0].first, inner_swap[0].second), (0xb4, 0x9c));

        let both = skin_frames(place(0x50));
        assert_eq!(both.len(), 2);
        assert_eq!(both[0].rect, [10.0, 20.0, 110.0, 100.0]);
        assert_eq!(both[1].rect, [11.0, 21.0, 109.0, 99.0]);

        assert!(skin_frames(place(0)).is_empty());
        assert!(skin_frames(place(0x30)).is_empty());
        assert!(skin_frames(place(0xc0)).is_empty());
    }

    #[test]
    fn a_skin_image_centres_and_selects_its_ink() {
        let inks = SkinImageInks {
            plain: [1, 2, 3],
            raised: [4, 5, 6],
            alpha: 255,
        };
        let place = |style: u32, flags: u32| SkinImagePlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            image: 42,
            size: 16,
            style,
            flags,
            default_image: false,
        };

        let centred = draw_skin_image(place(0, 0), inks);
        assert_eq!((centred.x, centred.y), (52, 52));
        assert_eq!(centred.ink, Some(0xe4));
        assert_eq!(centred.colour, None);
        assert_eq!(centred.frame, None);
        assert_eq!(centred.scale, None);

        let marked = draw_skin_image(place(0, 0x40), inks);
        assert_eq!((marked.x, marked.y), (52, 53));

        let scaled = draw_skin_image(place(0, 0x10000), inks);
        assert_eq!(scaled.ink, None);
        assert_eq!(scaled.scale, Some(1.0));

        let framed = draw_skin_image(place(0x3000_0000, 0x10000), inks);
        assert_eq!(framed.frame, Some(3));

        let texture = draw_skin_image(place(0x100_0000, 0x10), inks);
        assert_eq!(texture.ink, Some(0xd8));
        assert_eq!(texture.colour, None);

        let raised = draw_skin_image(place(0x100_0000, 0x40), inks);
        assert_eq!(raised.ink, Some(0xf4));
        assert_eq!(raised.colour, Some([4, 5, 6, 255]));

        let plain = draw_skin_image(place(0x100_0000, 0), inks);
        assert_eq!(plain.ink, Some(0xe4));
        assert_eq!(plain.colour, Some([1, 2, 3, 255]));

        let plain_slot = draw_skin_image(place(0, 0x40), inks);
        assert_eq!(plain_slot.ink, Some(0xf4));
        assert_eq!(plain_slot.colour, None);
    }

    #[test]
    fn a_skin_shade_draws_an_inset_rectangle() {
        let shade = draw_skin_shade(ShadePlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            inset: 5,
            ink: [1, 2, 3, 4],
        });
        assert_eq!(shade.prepare_calls, 1);
        assert_eq!(shade.modes, [1, 0x10]);
        assert_eq!(shade.mode_calls, 2);
        assert_eq!(shade.colour, [1, 2, 3, 255]);
        assert_eq!(shade.vertex_calls, 4);
        assert_eq!(shade.colour_writes, 4);
        assert_eq!(shade.begin_calls, 1);
        assert_eq!(shade.end_calls, 1);
        let want: [(f32, f32); 4] = [(105.0, 25.0), (15.0, 25.0), (15.0, 95.0), (105.0, 95.0)];
        for ((gx, gy), (wx, wy)) in shade.corners.iter().zip(want) {
            assert_eq!(gx.to_bits(), wx.to_bits());
            assert_eq!(gy.to_bits(), wy.to_bits());
        }
        assert_eq!(
            shade.rect,
            LineShadeRect {
                left: 15,
                top: 25,
                width: 90,
                height: 70,
                right: 105,
                bottom: 95,
            }
        );
    }

    #[test]
    fn a_skin_cross_draws_eight_corners() {
        let cross = draw_skin_cross(5, 10, 20);
        assert_eq!(cross.begin_calls, 1);
        assert_eq!(cross.end_calls, 1);
        assert_eq!(cross.vertex_calls, 8);
        let want: [(f32, f32); 8] = [
            (8.0, 15.0),
            (13.0, 15.0),
            (8.0, 25.0),
            (13.0, 25.0),
            (6.0, 18.0),
            (6.0, 23.0),
            (16.0, 18.0),
            (16.0, 23.0),
        ];
        for ((gx, gy), (wx, wy)) in cross.corners.iter().zip(want) {
            assert_eq!(gx.to_bits(), wx.to_bits());
            assert_eq!(gy.to_bits(), wy.to_bits());
        }
    }

    #[test]
    fn a_parent_bevel_takes_its_two_boolean_arms() {
        let ink = BevelInk {
            raised: bevel_arm([[200; 4], [201; 4]], [[210; 4], [211; 4]], [212; 4]),
            pressed: bevel_arm([[100; 4], [101; 4]], [[110; 4], [111; 4]], [112; 4]),
            idle: bevel_arm([[50; 4], [51; 4]], [[60; 4], [61; 4]], [62; 4]),
        };
        let place = BevelPlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            state: 0,
        };

        let raised = draw_skin_bevel(place, true, true, &ink);
        assert_eq!(raised.fill[0].inks[0], [201; 4]);
        let pressed = draw_skin_bevel(place, false, true, &ink);
        assert_eq!(pressed.fill[0].inks[0], [101; 4]);
        let idle = draw_skin_bevel(place, false, false, &ink);
        assert_eq!(idle.fill[0].inks[0], [51; 4]);
        assert_eq!((idle.begins, idle.binds, idle.ends), (3, 12, 3));
    }

    #[test]
    fn a_skin_hit_checks_edges_then_probes() {
        let place = |flags: u32| SkinHitPlace {
            x0: 100,
            y0: 100,
            size: 10,
            limit_right: 90,
            limit_top: 50,
            limit_bottom: 90,
            word_5c: 0,
            flags,
        };
        let none = |_: i32| None;

        let left = 0x8000_0011u32 as i32;
        let right = 0x8000_0012u32 as i32;
        let top = 0x8000_0014u32 as i32;
        let bottom = 0x8000_0018u32 as i32;

        assert_eq!(hit_skin(place(0x4000), 0, 0, 5, &none), left);
        assert_eq!(hit_skin(place(0x4000), 95, 0, 5, &none), right);
        assert_eq!(hit_skin(place(0x8000), 0, 95, 5, &none), bottom);
        assert_eq!(hit_skin(place(0x4000 | 0x8000), 0, 0, 5, &none), left | top);

        let four = |code: i32| (code == -4).then_some([0, 0, 200, 200]);
        assert_eq!(hit_skin(place(0x3), 10, 40, 5, &four), -4);
        assert_eq!(hit_skin(place(0x3), 10, 40, 5, &none), -3);
        let five = |code: i32| (code == -5).then_some([0, 0, 200, 200]);
        assert_eq!(hit_skin(place(0x5), 10, 40, 5, &five), -5);
        let ten = |code: i32| (code == -10).then_some([0, 0, 200, 200]);
        assert_eq!(hit_skin(place(0x9), 10, 40, 5, &ten), -10);
        assert_eq!(hit_skin(place(0x21), 10, 50, 5, &four), 0);

        let only_ten = |code: i32| (code == -10).then_some([0, 0, 200, 200]);
        assert_eq!(hit_skin(place(0x200), 90, 10, 5, &only_ten), -10);
        assert_eq!(hit_skin(place(0x200), 90, 10, 5, &none), -8);
        assert_eq!(hit_skin(place(0x100), 10, 90, 5, &none), -12);

        let edge = |code: i32| (code == -4).then_some([10, 20, 30, 40]);
        assert_eq!(hit_skin(place(0x3), 30, 40, 5, &edge), -4);
    }

    #[test]
    fn a_skin_detail_dispatches_its_two_helpers() {
        let place = |style: u32, flags: u32| SkinDetailPlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            inset: 5,
            style,
            flags,
        };

        let plain = draw_skin_detail(place(0, 0), 5);
        assert_eq!((plain.prepare_calls, plain.rect_calls), (1, 1));
        assert_eq!((plain.x, plain.y, plain.w, plain.h), (15, 25, 90, 70));
        assert!(!plain.first);
        assert!(!plain.second);
        assert_eq!(plain.horizontal, None);
        assert_eq!(plain.vertical, None);

        let horizontal = draw_skin_detail(place(0x2000_0000, 0), 5);
        assert_eq!(horizontal.horizontal, Some([10, 20, 100, 80]));
        assert_eq!(horizontal.horizontal_offset, -2);
        assert_eq!(horizontal.horizontal_mode, 0x50);
        assert_eq!(horizontal.vertical, None);
        assert_eq!(
            draw_skin_detail(place(0x2000_0000, 0x40), 5).horizontal,
            None
        );

        let vertical = draw_skin_detail(place(0x4000_0000, 0x40), 5);
        assert_eq!(vertical.vertical, Some([15, 25, 90, 70]));
        assert_eq!(draw_skin_detail(place(0x4000_0000, 0), 5).vertical, None);
        assert_eq!(
            draw_skin_detail(place(0, 0x80), 5).vertical,
            Some([15, 25, 90, 70])
        );
    }

    #[test]
    fn a_skin_rect_dispatches_its_overlay() {
        let place = |flags: u32| SkinRectPlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            inset: 5,
            flags,
        };

        let plain = draw_skin_rect(place(0));
        assert_eq!(plain.rect, [15, 25, 90, 70]);
        assert_eq!((plain.rect_calls, plain.cornered_calls), (1, 0));
        assert!(!plain.first);
        assert!(!plain.second);

        let first = draw_skin_rect(place(0x10));
        assert!(first.first);
        assert!(!first.second);
        assert_eq!(first.cornered_calls, 0);

        let second = draw_skin_rect(place(0x40));
        assert!(!second.first);
        assert!(second.second);
        assert_eq!(second.cornered_calls, 1);
        assert_eq!(second.rect, [15, 25, 90, 70]);
    }

    #[test]
    fn a_skin_strip_draws_four_bands() {
        let strip = draw_skin_strip(10.0, 20.0, 30.0, 40.0);
        let want: [(f32, f32); 8] = [
            (30.0, 20.0),
            (10.0, 20.0),
            (30.0, 30.0),
            (10.0, 30.0),
            (30.0, 30.0),
            (10.0, 30.0),
            (10.0, 40.0),
            (30.0, 40.0),
        ];
        for ((gx, gy), (wx, wy)) in strip.vertices.iter().zip(want) {
            assert_eq!(gx.to_bits(), wx.to_bits());
            assert_eq!(gy.to_bits(), wy.to_bits());
        }
        assert_eq!(strip.binds, [0x12c, 0x134, 0x138, 0x130]);
        assert_eq!(
            (strip.begin_calls, strip.bind_calls, strip.end_calls),
            (1, 4, 1)
        );
    }

    #[test]
    fn a_skin_slider_draws_both_styles() {
        let place = |style: u32, flags: u32| SkinSliderPlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            inset: 5,
            thumb: 10,
            fraction: 0.5,
            style,
            flags,
        };

        let plain = draw_skin_slider(place(0, 0), 5).expect("plain");
        assert_eq!(plain.mode, SkinSliderMode::Plain);
        assert_eq!(plain.rect, [12, 22, 96, 76]);
        assert_eq!(plain.thumb, Some([55, 25, 10, 70]));
        assert_eq!(plain.translate, 40);
        assert!(!plain.thumb_first);
        assert!(!plain.thumb_second);
        assert_eq!(plain.bar, None);

        let first = draw_skin_slider(place(0, 0x10), 5).expect("plain");
        assert!(first.thumb_first);

        let filled = draw_skin_slider(place(0x1000_0000, 0), 5).expect("filled");
        assert_eq!(filled.mode, SkinSliderMode::Filled);
        assert_eq!(filled.rect, [16, 26, 88, 68]);
        assert_eq!(filled.bar, Some([57, 20, 5, 80]));
        assert_eq!(filled.thumb, None);
        assert_eq!(
            (filled.begin_calls, filled.bind_calls, filled.end_calls),
            (1, 1, 1)
        );

        assert_eq!(draw_skin_slider(place(0x2000_0000, 0), 5), None);
    }

    #[test]
    fn skin_measures_copy_the_three_globals() {
        assert_eq!(skin_measures(2, 2, 24, 4), [2, 2, 24, 20]);
        assert_eq!(skin_measures(2, 2, 24, 5), [2, 2, 24, 24]);
    }

    #[test]
    fn the_skin_programs_build_once() {
        let again = prepare_skin_programs(true);
        assert!(again.programs.is_empty());
        assert_eq!(again.uniform_calls, 0);

        let built = prepare_skin_programs(false);
        assert_eq!(built.programs.len(), 3);
        assert_eq!(built.uniform_calls, 6);
        let kinds: Vec<u32> = built.programs.iter().map(|p| p.kind).collect();
        assert_eq!(kinds, vec![5, 0x15, 0x15]);

        assert_eq!(
            built.programs[0].uniforms,
            vec![SkinUniform {
                name: "MPrj",
                value: Some(SkinUniformValue::Float(2.0))
            }]
        );
        assert_eq!(
            built.programs[1].uniforms,
            vec![
                SkinUniform {
                    name: "MPrj",
                    value: Some(SkinUniformValue::Float(2.0))
                },
                SkinUniform {
                    name: "Scale",
                    value: None
                }
            ]
        );
        assert_eq!(
            built.programs[2].uniforms,
            vec![
                SkinUniform {
                    name: "MPrj",
                    value: Some(SkinUniformValue::Float(2.0))
                },
                SkinUniform {
                    name: "Tex",
                    value: Some(SkinUniformValue::Int(0))
                },
                SkinUniform {
                    name: "TexCol",
                    value: None
                }
            ]
        );

        assert!(built.programs[0].source.contains("FragColor = vrCol;"));
        assert!(built.programs[1].source.contains("uniform sampler2D Tex;"));
        assert!(built.programs[2].source.contains("uniform vec4 Scale;"));
    }

    #[test]
    fn an_inset_bevel_selects_its_arm() {
        let place = |flags: u32| SkinInsetBevelPlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            inset: 5,
            flags,
        };

        let arm = draw_inset_bevel(place(0x40), 5);
        assert_eq!(arm.prepare_calls, 1);
        assert!(arm.plain.is_none());
        let arm = arm.arm.expect("arm");
        assert_eq!(arm.inner, [17, 27, 86, 66]);
        assert_eq!(arm.inner_binds, [0x74, 0x80]);
        assert_eq!(arm.border, [16, 26, 88, 68]);
        assert_eq!(arm.border_binds, [0x7c, 0x88]);
        assert_eq!(arm.outline, [15, 25, 90, 70]);
        assert_eq!(arm.outline_slot, 0x6c);
        assert_eq!(arm.detail, [16, 26, 88, 68]);
        assert_eq!(arm.detail_mode, 0xa0);
        assert_eq!(arm.detail_unit, 5);

        let plain = draw_inset_bevel(place(0x10), 5);
        assert!(plain.arm.is_none());
        let plain = plain.plain.expect("plain");
        assert_eq!(plain.rect, [15, 25, 90, 70]);
        assert!(plain.first);
        assert!(!plain.second);
        assert_eq!(plain.extra_height, 70);
    }

    #[test]
    fn a_second_skin_slider_offsets_its_track() {
        let place = |style: u32, flags: u32| SkinSliderPlace {
            x: 10,
            y: 20,
            w: 100,
            h: 80,
            inset: 5,
            thumb: 10,
            fraction: 0.5,
            style,
            flags,
        };

        let plain = draw_skin_scroll(place(0, 0x10), 5).expect("plain");
        assert_eq!(plain.rect, [11, 22, 98, 76]);
        assert_eq!(plain.thumb, Some([55, 26, 10, 68]));
        assert_eq!(plain.translate, 40);
        assert!(plain.thumb_first);
        assert!(!plain.thumb_second);

        let filled = draw_skin_scroll(place(0x1000_0000, 0), 5).expect("filled");
        assert_eq!(filled.rect, [16, 26, 88, 68]);
        assert_eq!(filled.bar, Some([57, 20, 5, 80]));
        assert_eq!(
            (filled.begin_calls, filled.bind_calls, filled.end_calls),
            (1, 1, 1)
        );

        assert_eq!(draw_skin_scroll(place(0x2000_0000, 0), 5), None);
    }

    #[test]
    fn a_skin_region_clips_scrolls_and_replays() {
        let place = |mode: u32, flags: u32, clip_ok: bool| SkinRegionPlace {
            origin: [10, 20],
            inset: [5, 4],
            extent: [100, 80],
            total: [150, 120],
            fraction: [0.5, 0.25],
            mode,
            flags,
            translated: true,
            clip_ok,
            prepared: true,
            lines: vec![[1, 2, 3, 4], [5, 6, 7, 8]],
            children: vec![true, false, true],
        };

        let drawn = draw_skin_region(place(0, 0, true), 5);
        assert!(drawn.draw);
        assert_eq!(drawn.clip, [15, 24, 95, 76]);
        assert_eq!(drawn.scroll, [28, 11]);
        assert_eq!(drawn.translate, Some([-10.0, 20.0]));
        assert_eq!(drawn.lines.len(), 2);
        assert_eq!(drawn.child_calls, 2);
        assert_eq!(drawn.detail, None);
        assert_eq!(
            (
                drawn.begin_calls,
                drawn.vtable_calls,
                drawn.line_end_calls,
                drawn.scissor_calls
            ),
            (2, 2, 1, 2)
        );

        let skipped = draw_skin_region(place(1, 0, true), 5);
        assert!(!skipped.draw);
        assert_eq!(skipped.begin_calls, 1);
        assert_eq!(skipped.vtable_calls, 1);
        assert_eq!(skipped.clip, [0; 4]);

        let rejected = draw_skin_region(place(0, 0, false), 5);
        assert!(!rejected.draw);
        assert_eq!(rejected.begin_calls, 1);

        let fixed = draw_skin_region(place(0, 0x800, true), 5);
        assert_eq!(fixed.scroll, [0, 0]);

        let one = draw_skin_region(place(0, 0x80, true), 5);
        assert_eq!(one.detail, Some([28, 11, 95, 76]));
        assert_eq!((one.detail_mode, one.detail_offset), (0x30, 5));

        let two = draw_skin_region(place(0, 0x10_0000, true), 5);
        assert_eq!((two.detail_mode, two.detail_offset), (0x50, 20));

        let both = draw_skin_region(place(0, 0x10_0080, true), 5);
        assert_eq!((both.detail_mode, both.detail_offset), (0x30, 5));
    }

    #[test]
    fn a_scheme_colour_expands_into_twelve_shades() {
        let mut group = [[0u8, 0, 0, 100]; 12];
        expand_group(&mut group, [100, 150, 200, 100], 2.0, 0.5);
        assert_eq!(
            group,
            [
                [5, 8, 10, 100],
                [11, 17, 23, 100],
                [25, 38, 50, 100],
                [50, 75, 100, 100],
                [100, 150, 200, 100],
                [200, 255, 255, 100],
                [255, 255, 255, 100],
                [255, 255, 255, 100],
                [255, 255, 255, 100],
                [255, 255, 255, 100],
                [255, 255, 255, 100],
                [255, 255, 255, 100],
            ]
        );
    }

    #[test]
    fn a_scale_keeps_the_destination_alpha() {
        let mut group = [[9u8, 9, 9, 77]; 12];
        expand_group(&mut group, [40, 80, 120, 200], 1.5, 0.0);
        assert_eq!(group[0][3], 77);
        assert_eq!(group[5][3], 77);
        assert_eq!(group[7][3], 77);
        assert_eq!(group[4][3], 200);
        assert_eq!(group[8][3], 200);
        assert_eq!(group[11][3], 200);
        assert_eq!(group[4][..3], [40, 80, 120]);
    }

    #[test]
    fn the_accent_mixes_and_the_two_alpha_bytes() {
        let mut palette = SkinPalette::filled([9, 9, 9, 77]);
        let scheme = SkinScheme {
            groups: [
                ([100, 150, 200, 40], 2.0, 0.5),
                ([80, 90, 100, 50], 1.5, 0.25),
                ([200, 200, 200, 60], 1.2, 0.1),
                ([180, 60, 30, 70], 1.1, 0.3),
            ],
            accent: [200, 100, 50, 180],
            accent_scale: 1.5,
            first_alpha: 120,
            second_alpha: 200,
        };
        load_scheme(&mut palette, &scheme);
        assert_eq!(palette.accent_dim, [100, 50, 25, 180]);
        assert_eq!(palette.accent_bright, [210, 197, 178, 180]);
        assert_eq!(
            palette.accent_mix,
            [
                [144, 109, 86, 180],
                [166, 138, 117, 180],
                [174, 133, 102, 198],
                [225, 201, 174, 198],
            ]
        );
        assert_eq!(palette.form, palette.form_hot);
        assert_eq!(palette.form[3], [50, 75, 100, 77]);
        assert_eq!((palette.first_alpha, palette.second_alpha), (120, 200));
        assert_eq!((palette.accent_alpha_a, palette.accent_alpha_b), (44, 44));
    }

    #[test]
    fn the_palette_alpha_ramps_skip_the_last_highlight_entry() {
        let mut palette = SkinPalette::filled([10, 20, 30, 200]);
        palette.first_alpha = 100;
        palette.second_alpha = 160;
        scale_palette_alpha(&mut palette, 0.5, 0.25);
        assert_eq!(palette.form[0][3], 50);
        assert_eq!(palette.form[11][3], 50);
        assert_eq!(palette.control[3][3], 40);
        assert_eq!(palette.form_hot[0][3], 40);
        assert_eq!(palette.text[11][3], 40);
        assert_eq!(palette.highlight[10][3], 40);
        assert_eq!(palette.highlight[11][3], 200);
        assert_eq!(palette.accent_mix[2][3], 50);
        assert_eq!(palette.accent_mix[3][3], 50);
        assert_eq!(palette.accent_mix[0][3], 200);
    }

    #[test]
    fn a_skin_layout_follows_the_unit() {
        let layout = skin_constructor(5);
        assert_eq!(
            layout,
            SkinLayout {
                border: 2,
                pad: 2,
                title: 24,
                reach: 24,
                edge: 2,
                gap: 2,
                head: 24,
                row: 20,
            }
        );
        let wider = skin_constructor(12);
        assert_eq!(wider.reach, 52);
    }

    #[test]
    fn a_minimum_keeps_the_second_operand_on_nan() {
        assert_eq!(min_ss(f32::NAN, 0.5), 0.5);
        assert!(min_ss(0.5, f32::NAN).is_nan());
        assert_eq!(min_ss(2.0, 1.0), 1.0);
    }

    #[test]
    fn a_batch_selects_its_program_and_texture_offset() {
        let mut batch = SkinBatch::new();
        batch.select(5, 0, true);
        assert_eq!(batch.mode, SkinMode::Textured);
        assert_eq!(batch.tex_col, Some(0.0));
        batch.select(2, 0, true);
        assert_eq!(batch.tex_col, Some(1.0));
        batch.select(1, 0, false);
        assert_eq!(batch.mode, SkinMode::Solid);
        batch.select(3, 0, false);
        assert_eq!(batch.mode, SkinMode::Rounded);
        batch.select(0x10, 4, false);
        assert_eq!((batch.mode, batch.subclass_mode), (SkinMode::Skin, 4));
        batch.set_texture_shift(512.0, 256.0);
        assert_eq!(batch.texture_shift, [1.0, 0.5]);
        batch.add_texture_shift(-256.0, 512.0);
        assert_eq!(batch.texture_shift, [0.5, 1.5]);
        assert_eq!(batch.mode_calls, 5);
    }

    #[test]
    fn an_outline_draws_four_line_pairs_half_a_pixel_in() {
        let mut batch = SkinBatch::new();
        batch.set_ink([1, 2, 3, 4]);
        outline_rect(&mut batch, GuiRect::corners(10.0, 20.0, 110.0, 100.0));
        let primitive = &batch.primitives[0];
        assert_eq!(primitive.draw, GuiDraw::Lines);
        let points: Vec<(f32, f32)> = primitive.vertices.iter().map(|v| (v.x, v.y)).collect();
        assert_eq!(
            points,
            [
                (109.5, 20.5),
                (10.5, 20.5),
                (10.5, 20.5),
                (10.5, 99.5),
                (10.5, 99.5),
                (109.5, 99.5),
                (109.5, 99.5),
                (109.5, 20.5),
            ]
        );
        assert!(primitive.vertices.iter().all(|v| v.ink == [1, 2, 3, 4]));
    }

    #[test]
    fn a_fade_rect_runs_top_to_bottom_through_two_mixes() {
        let mut batch = SkinBatch::new();
        fade_rect(
            &mut batch,
            GuiRect::corners(10.0, 20.0, 110.0, 100.0),
            [255, 255, 255, 255],
            [0, 0, 0, 255],
        );
        let primitive = &batch.primitives[0];
        assert_eq!(primitive.draw, GuiDraw::Quads);
        let inks: Vec<SkinInk> = primitive.vertices.iter().map(|v| v.ink).collect();
        assert_eq!(inks[0], [0, 0, 0, 255]);
        assert_eq!(inks[2], [110, 110, 110, 255]);
        assert_eq!(inks[4], [145, 145, 145, 255]);
        assert_eq!(inks[6], [255, 255, 255, 255]);
        assert_eq!(primitive.vertices[2].y, 60.0);
    }

    #[test]
    fn a_faded_rect_scales_both_alphas_before_mixing() {
        let mut batch = SkinBatch::new();
        faded_rect(
            &mut batch,
            GuiRect::corners(0.0, 0.0, 10.0, 10.0),
            [10, 20, 30, 200],
            [40, 50, 60, 100],
            0.5,
        );
        let inks: Vec<SkinInk> = batch.primitives[0].vertices.iter().map(|v| v.ink).collect();
        assert_eq!(inks[0], [40, 50, 60, 50]);
        assert_eq!(inks[2], [28, 38, 48, 70]);
        assert_eq!(inks[4], [22, 32, 42, 80]);
        assert_eq!(inks[6], [10, 20, 30, 100]);
    }

    #[test]
    fn a_stripe_bar_walks_the_count_fractions() {
        let mut batch = SkinBatch::new();
        stripe_bar(
            &mut batch,
            GuiRect::corners(10.0, 20.0, 111.0, 40.0),
            &[[10, 20, 30, 0], [40, 50, 60, 0], [70, 80, 90, 0]],
        );
        let primitive = &batch.primitives[0];
        let points: Vec<(f32, f32)> = primitive.vertices.iter().map(|v| (v.x, v.y)).collect();
        assert_eq!(
            points,
            [
                (10.0, 20.0),
                (10.0, 40.0),
                (60.0, 40.0),
                (60.0, 20.0),
                (60.0, 20.0),
                (60.0, 40.0),
                (111.0, 40.0),
                (111.0, 20.0),
            ]
        );
        assert_eq!(primitive.vertices[0].ink, [10, 20, 30, 255]);
        assert_eq!(primitive.vertices[2].ink, [40, 50, 60, 255]);
        assert_eq!(primitive.vertices[4].ink, [40, 50, 60, 255]);
        assert_eq!(primitive.vertices[6].ink, [70, 80, 90, 255]);
    }

    #[test]
    fn a_selection_band_draws_its_outline_and_two_glows() {
        let mut batch = SkinBatch::new();
        selection_band(
            &mut batch,
            GuiRect::corners(10.0, 20.0, 110.0, 100.0),
            [10, 20, 30, 255],
            [200, 100, 50, 255],
            false,
            4,
        );
        assert_eq!(batch.blend, 0x203);
        assert_eq!(batch.primitives.len(), 2);
        assert_eq!(batch.primitives[0].draw, GuiDraw::Lines);
        let outline = &batch.primitives[0].vertices;
        assert_eq!((outline[0].x, outline[0].y), (10.0, 20.0));
        assert_eq!(outline[0].ink, [10, 20, 30, 25]);
        assert_eq!(outline[1].ink, [200, 100, 50, 200]);
        assert_eq!((outline[3].x, outline[3].y), (110.0, 20.0));
        let glow = &batch.primitives[1].vertices;
        assert_eq!(batch.primitives[1].draw, GuiDraw::Triangles);
        assert_eq!(glow.len(), 24);
        assert_eq!((glow[0].x, glow[0].y), (60.0, 26.0));
        assert_eq!(glow[0].ink, [10, 20, 30, 0]);
        assert_eq!((glow[1].x, glow[1].y), (60.0, 21.0));
        assert_eq!(glow[1].ink, [10, 20, 30, 100]);
        assert_eq!((glow[13].x, glow[13].y), (60.0, 104.0));
        assert_eq!((glow[14].x, glow[14].y), (10.0, 103.0));
    }

    #[test]
    fn a_rounded_fill_sets_its_scale_and_grows_by_four_radii() {
        let mut batch = SkinBatch::new();
        rounded_fill(
            &mut batch,
            GuiRect::corners(10.0, 20.0, 110.0, 100.0),
            5,
            1.0,
            1.0,
        );
        assert_eq!(batch.mode, SkinMode::Rounded);
        let scale = batch.rounded_scale.expect("a rounded scale");
        assert_eq!(scale[0], 100.0 / 140.0 * 0.5);
        assert_eq!(scale[2], 1.0 / (0.5 - scale[0]));
        assert_eq!(batch.primitives[0].vertices[0].x, -10.0);
        assert_eq!(batch.primitives[0].vertices[0].y, 0.0);
        assert_eq!(batch.primitives[0].vertices[2].x, 130.0);
        assert_eq!(batch.primitives[0].vertices[2].y, 120.0);
        assert_eq!(batch.primitives[0].vertices[0].ink, [0, 0, 0, 255]);
    }

    #[test]
    fn a_shadow_ring_clamps_its_inset_to_half_the_height() {
        let mut batch = SkinBatch::new();
        shadow_ring(
            &mut batch,
            GuiRect::corners(10.0, 20.0, 30.0, 36.0),
            10,
            0x30,
        );
        let primitive = &batch.primitives[0];
        assert_eq!(primitive.vertices.len(), 16);
        assert_eq!(
            (primitive.vertices[0].x, primitive.vertices[0].y),
            (10.0, 20.0)
        );
        assert_eq!(primitive.vertices[0].ink, [0, 0, 0, 0x30]);
        assert_eq!(
            (primitive.vertices[2].x, primitive.vertices[2].y),
            (18.0, 28.0)
        );
        assert_eq!(primitive.vertices[2].ink, [0, 0, 0, 0]);
    }

    #[test]
    fn a_double_frame_insets_its_second_loop_by_one() {
        let mut batch = SkinBatch::new();
        double_frame(
            &mut batch,
            GuiRect::corners(10.0, 20.0, 110.0, 100.0),
            [1, 1, 1, 255],
            [2, 2, 2, 255],
        );
        let primitive = &batch.primitives[0];
        assert_eq!(primitive.vertices.len(), 16);
        assert_eq!(primitive.vertices[0].ink, [1, 1, 1, 255]);
        assert_eq!(
            (primitive.vertices[0].x, primitive.vertices[0].y),
            (109.5, 20.5)
        );
        assert_eq!(
            (primitive.vertices[7].x, primitive.vertices[7].y),
            (109.5, 19.5)
        );
        assert_eq!(primitive.vertices[8].ink, [2, 2, 2, 255]);
        assert_eq!(
            (primitive.vertices[8].x, primitive.vertices[8].y),
            (108.5, 21.5)
        );
    }

    #[test]
    fn a_vertical_gradient_holds_its_mix_bands_two_units_in() {
        let mut batch = SkinBatch::new();
        vertical_gradient(
            &mut batch,
            GuiRect::corners(10.0, 20.0, 110.0, 100.0),
            4,
            [0, 0, 0, 255],
            [255, 255, 255, 255],
        );
        let primitive = &batch.primitives[0];
        assert_eq!(primitive.vertices.len(), 12);
        assert_eq!(
            (primitive.vertices[3].x, primitive.vertices[3].y),
            (110.0, 28.0)
        );
        assert_eq!(
            (primitive.vertices[6].x, primitive.vertices[6].y),
            (10.0, 92.0)
        );
        assert_eq!(primitive.vertices[0].ink, [0, 0, 0, 255]);
        assert_eq!(primitive.vertices[2].ink, [89, 89, 89, 255]);
        assert_eq!(primitive.vertices[4].ink, [128, 128, 128, 255]);
        assert_eq!(primitive.vertices[8].ink, [166, 166, 166, 255]);
        assert_eq!(primitive.vertices[10].ink, [255, 255, 255, 255]);
    }

    #[test]
    fn a_scroll_arrow_is_a_line_list_with_a_faded_second_half() {
        let mut batch = SkinBatch::new();
        scroll_arrow_horizontal(&mut batch, 10.0, 20.0, 9.0, [1, 2, 3, 200]);
        let primitive = &batch.primitives[0];
        assert_eq!(primitive.draw, GuiDraw::Lines);
        assert_eq!(primitive.vertices.len(), 12);
        assert_eq!(primitive.vertices[0].x, 10.0);
        assert_eq!(primitive.vertices[0].y, 25.0);
        assert_eq!(primitive.vertices[1].x, 14.0);
        assert_eq!(primitive.vertices[1].y, 21.0);
        assert_eq!(primitive.vertices[3].ink, [1, 2, 3, 200]);
        assert_eq!(primitive.vertices[4].ink, [1, 2, 3, 132]);
        assert_eq!(primitive.vertices[11].ink, [1, 2, 3, 132]);
    }

    fn palette_for_faces() -> SkinPalette {
        let mut palette = SkinPalette::filled([9, 9, 9, 255]);
        palette.form = [[10, 11, 12, 40]; 12];
        palette.form_hot = [[20, 21, 22, 50]; 12];
        palette.control = [[30, 31, 32, 60]; 12];
        palette.text = [[40, 41, 42, 70]; 12];
        palette.highlight = [[50, 51, 52, 80]; 12];
        palette
    }

    #[test]
    fn an_idle_button_face_draws_its_quad_border_and_outline() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        draw_button_face(
            &mut batch,
            GuiRect::corners(0.0, 0.0, 100.0, 24.0),
            false,
            false,
            &palette,
        );
        assert_eq!(batch.primitives.len(), 3);
        let quad = &batch.primitives[0].vertices;
        assert_eq!((quad[0].x, quad[0].y), (98.0, 2.0));
        assert_eq!(quad[0].ink, [30, 31, 32, 60]);
        assert_eq!(quad[2].ink, [30, 31, 32, 60]);
        let border = &batch.primitives[1].vertices;
        assert_eq!(border[0].ink, [30, 31, 32, 60]);
        assert_eq!(border[3].ink, [30, 31, 32, 60]);
        let outline = &batch.primitives[2].vertices;
        assert_eq!(outline[0].ink, [30, 31, 32, 60]);
    }

    #[test]
    fn a_hot_button_face_uses_the_form_copy() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        draw_button_face(
            &mut batch,
            GuiRect::corners(0.0, 0.0, 100.0, 24.0),
            false,
            true,
            &palette,
        );
        assert_eq!(batch.primitives[0].vertices[0].ink, [20, 21, 22, 50]);
        assert_eq!(batch.primitives[2].vertices[0].ink, [20, 21, 22, 50]);
    }

    #[test]
    fn a_pressed_button_face_uses_the_pressed_inks() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        draw_button_face(
            &mut batch,
            GuiRect::corners(0.0, 0.0, 100.0, 24.0),
            true,
            true,
            &palette,
        );
        assert_eq!(batch.primitives[0].vertices[0].ink, [30, 31, 32, 60]);
        assert_eq!(batch.primitives[0].vertices[2].ink, [30, 31, 32, 60]);
    }

    #[test]
    fn a_pressed_panel_draws_a_bevel_outline_and_inner_shadow() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        draw_panel(
            &mut batch,
            GuiRect::corners(0.0, 0.0, 100.0, 40.0),
            4,
            0x40,
            4,
            &palette,
        );
        assert_eq!(batch.primitives.len(), 4);
        assert_eq!(batch.primitives[0].vertices[0].ink, [30, 31, 32, 60]);
        assert_eq!(batch.primitives[2].vertices[0].ink, [30, 31, 32, 60]);
        assert_eq!(batch.primitives[2].vertices[0].x, 95.5);
        assert_eq!(batch.primitives[3].vertices.len(), 16);
        assert_eq!(batch.primitives[3].vertices[0].ink, [0, 0, 0, 0xa0]);
        assert_eq!(batch.primitives[3].vertices[0].x, 5.0);
    }

    #[test]
    fn a_plain_panel_draws_the_frame_slot() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        draw_panel(
            &mut batch,
            GuiRect::corners(0.0, 0.0, 100.0, 40.0),
            4,
            0,
            4,
            &palette,
        );
        assert_eq!(batch.primitives.len(), 3);
        assert_eq!(batch.primitives[0].vertices[0].x, 94.0);
        assert_eq!(batch.primitives[0].vertices[0].y, 6.0);
    }

    #[test]
    fn a_plain_slider_places_its_thumb_from_the_fraction() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        let drawn = draw_control_slider(
            &mut batch,
            ControlSliderPlace {
                rect: GuiRect::corners(0.0, 0.0, 100.0, 24.0),
                inset: 4,
                thumb: 16,
                state: 0,
                style: 0,
                fraction: 0.5,
            },
            &[[1, 2, 3, 4], [5, 6, 7, 8]],
            &palette,
        );
        assert_eq!(drawn.shift, 38);
        let frame = drawn.frame.expect("a thumb");
        assert_eq!(
            (frame.x0, frame.y0, frame.x1, frame.y1),
            (42.0, 4.0, 58.0, 20.0)
        );
        assert_eq!(batch.primitives.len(), 4);
        assert_eq!(batch.texture_shift, [0.0, 0.0]);
    }

    #[test]
    fn a_filled_slider_stripes_its_track_and_outlines_the_bar() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        let drawn = draw_control_slider(
            &mut batch,
            ControlSliderPlace {
                rect: GuiRect::corners(0.0, 0.0, 100.0, 24.0),
                inset: 4,
                thumb: 16,
                state: 0,
                style: 0x1000_0000,
                fraction: 0.5,
            },
            &[[1, 2, 3, 4], [5, 6, 7, 8]],
            &palette,
        );
        assert!(drawn.stripe);
        assert_eq!(drawn.shift, 44);
        let stripe = &batch.primitives[0].vertices;
        assert_eq!((stripe[0].x, stripe[0].y), (5.0, 5.0));
        assert_eq!((stripe[2].x, stripe[2].y), (95.0, 19.0));
        let outline = &batch.primitives[1].vertices;
        assert_eq!((outline[0].x, outline[0].y), (51.5, 0.5));
        assert_eq!((outline[2].x, outline[2].y), (48.5, 0.5));
        assert_eq!(outline[0].ink, [40, 41, 42, 70]);
    }

    #[test]
    fn a_plain_scrollbar_puts_its_thumb_at_the_right_and_steps_by_ten() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        let drawn = draw_control_scrollbar(
            &mut batch,
            ControlScrollPlace {
                rect: GuiRect::corners(0.0, 0.0, 100.0, 24.0),
                inset: 4,
                state: 0,
                style: 0,
                fraction: 0.5,
                unit: 4,
            },
            &[[1, 2, 3, 4], [5, 6, 7, 8]],
            &palette,
        );
        assert_eq!(drawn.step, 5);
        let thumb = drawn.thumb.expect("a thumb");
        assert_eq!(
            (thumb.x0, thumb.y0, thumb.x1, thumb.y1),
            (83.0, 2.0, 98.0, 22.0)
        );
        assert_eq!(
            drawn.box_rect.expect("a box"),
            GuiRect::corners(1.0, 1.0, 99.0, 23.0)
        );
    }

    #[test]
    fn a_scrollbar_fill_is_the_unit_bar_at_the_value() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        let drawn = draw_control_scrollbar(
            &mut batch,
            ControlScrollPlace {
                rect: GuiRect::corners(0.0, 0.0, 100.0, 24.0),
                inset: 4,
                state: 0,
                style: 0x2000_0000,
                fraction: 0.5,
                unit: 4,
            },
            &[[1, 2, 3, 4], [5, 6, 7, 8]],
            &palette,
        );
        assert_eq!(drawn.shift, 44);
        let outline = &batch.primitives[1].vertices;
        assert_eq!((outline[0].x, outline[0].y), (51.5, 0.5));
        assert_eq!((outline[2].x, outline[2].y), (48.5, 0.5));
    }

    #[test]
    fn a_mark_is_active_on_its_code_or_an_unhot_hot_code() {
        assert!(mark_active(-9, -9, false, -10));
        assert!(mark_active(-10, -9, false, -10));
        assert!(!mark_active(-10, -9, true, -10));
        assert!(!mark_active(-8, -9, false, -10));
    }

    #[test]
    fn a_selected_mark_icon_draws_at_scale_one_from_the_bottom_left() {
        let palette = palette_for_faces();
        let selected = draw_mark_icon(
            GuiRect::corners(10.0, 20.0, 30.0, 40.0),
            5,
            4,
            true,
            &palette,
        );
        assert_eq!((selected.x, selected.y), (12.0, 40.0));
        assert_eq!(selected.scale, Some(1.0));
        assert_eq!(selected.ink, None);
        let plain = draw_mark_icon(
            GuiRect::corners(10.0, 20.0, 30.0, 40.0),
            5,
            4,
            false,
            &palette,
        );
        assert_eq!(plain.scale, None);
        assert_eq!(plain.ink, Some([40, 41, 42, 70]));
    }

    #[test]
    fn the_style_switch_draws_both_double_frames() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        draw_style_frames(
            &mut batch,
            GuiRect::corners(0.0, 0.0, 100.0, 40.0),
            0x50,
            &palette,
        );
        assert_eq!(batch.primitives.len(), 2);
        assert_eq!(batch.primitives[0].vertices.len(), 16);
        assert_eq!(batch.primitives[1].vertices[0].x, 98.5);
        assert_eq!(batch.primitives[1].vertices[0].ink, [20, 21, 22, 50]);
        assert_eq!(batch.primitives[1].vertices[8].ink, [30, 31, 32, 60]);
    }

    #[test]
    fn a_label_fades_in_over_three_hundred_seconds() {
        let faded = fade_label(0, 150, 40, [1, 2, 3, 200]);
        assert_eq!(faded.ink, [1, 2, 3, 100]);
        assert!(!faded.finished);
        assert_eq!(faded.y, 40);
        let done = fade_label(0, 400, 40, [1, 2, 3, 200]);
        assert!(done.finished);
        assert!(done.clear_bit);
        assert_eq!(done.ink, [1, 2, 3, 200]);
    }

    fn form_rect_place() -> FormRectPlace {
        FormRectPlace {
            width: 200,
            height: 100,
            unit: 5,
            inset: 5,
            client_top: 24,
            right: 180,
            bottom: 90,
            flags: 0,
        }
    }

    #[test]
    fn a_form_rect_switches_on_its_code() {
        let mut rect = [0; 4];
        assert!(form_rect(form_rect_place(), -4, &mut rect));
        assert_eq!(rect, [178, 4, 195, 22]);
        assert!(form_rect(form_rect_place(), -6, &mut rect));
        assert_eq!(rect, [178, 4, 195, 22]);
        assert!(form_rect(
            FormRectPlace {
                flags: 2,
                ..form_rect_place()
            },
            -6,
            &mut rect
        ));
        assert_eq!(rect, [175, 4, 192, 22]);
        assert!(form_rect(
            FormRectPlace {
                flags: 0xa,
                ..form_rect_place()
            },
            -5,
            &mut rect
        ));
        assert_eq!(rect, [172, 4, 189, 22]);
        assert!(form_rect(form_rect_place(), -9, &mut rect));
        assert_eq!(rect, [180, 24, 195, 39]);
        assert!(form_rect(form_rect_place(), -13, &mut rect));
        assert_eq!(rect, [5, 80, 20, 95]);
        assert!(form_rect(form_rect_place(), -14, &mut rect));
        assert_eq!(rect, [165, 80, 180, 95]);
        assert!(form_rect(form_rect_place(), -10, &mut rect));
        assert_eq!(rect, [180, 80, 195, 95]);
        rect = [1, 2, 3, 4];
        assert!(!form_rect(form_rect_place(), -8, &mut rect));
        assert_eq!(rect, [1, 2, 3, 4]);
    }

    #[test]
    fn a_chrome_draws_the_scrollbar_gutters_and_the_client_fill() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        let place = FormChromePlace {
            rect: GuiRect::corners(0.0, 0.0, 200.0, 100.0),
            frame_style: 0,
            flags: 0x2_0000,
            state: 0x20,
            margin: 4.0,
            bar: 8.0,
            client_left: 20.0,
            client_top: 30.0,
            client_right: 180.0,
            client_bottom: 80.0,
            fill: Some([1, 2, 3, 4]),
            flat: true,
        };
        let chrome = draw_form_chrome(&mut batch, place, &palette);
        assert_eq!(chrome.gutters.len(), 4);
        assert_eq!(
            chrome.gutters[0],
            [(4.0, 4.0), (4.0, 22.0), (196.0, 22.0), (196.0, 4.0)]
        );
        assert_eq!(
            chrome.gutters[3],
            [(12.0, 88.0), (12.0, 96.0), (188.0, 96.0), (188.0, 88.0)]
        );
        assert_eq!(
            chrome.fill,
            Some([(20.0, 30.0), (20.0, 80.0), (180.0, 80.0), (180.0, 30.0)])
        );
        assert_eq!(batch.mode, SkinMode::Skin);
        assert_eq!(chrome.frames, 0);
        assert_eq!((chrome.begin_calls, chrome.end_calls), (2, 2));
    }

    #[test]
    fn a_flat_styled_chrome_frames_the_form() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        let chrome = draw_form_chrome(
            &mut batch,
            FormChromePlace {
                rect: GuiRect::corners(0.0, 0.0, 200.0, 100.0),
                frame_style: 0x10,
                flags: 0,
                state: 1,
                margin: 0.0,
                bar: 0.0,
                client_left: 0.0,
                client_top: 0.0,
                client_right: 0.0,
                client_bottom: 0.0,
                fill: None,
                flat: false,
            },
            &palette,
        );
        assert_eq!(chrome.frames, 1);
        assert_eq!(batch.primitives.len(), 1);
        assert_eq!(batch.primitives[0].vertices.len(), 16);
        assert_eq!(batch.primitives[0].vertices[0].ink, [10, 11, 12, 40]);
    }

    #[test]
    fn a_form_background_orders_the_round_shell_title_marks_and_text() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        let drawn = draw_form_background(
            &mut batch,
            FormBackgroundPlace {
                rect: GuiRect::corners(0.0, 0.0, 200.0, 100.0),
                inset: 5,
                client_top: 24,
                client_right: 180,
                client_bottom: 90,
                state: 0x40 | 1 | 2,
                flags: 0,
                frame_style: 0,
                margin: 4.0,
                bar: 8.0,
                vertical: None,
                horizontal: None,
                title_width: 0,
                title_x: 0,
                title_y: 0,
                title_style: 0,
                title_offset: 0,
                marks: vec![(-4, 1, false)],
                active_code: -4,
                hot_code: 0,
                hot_flag: false,
            },
            4,
            &palette,
        );
        assert!(drawn.rounded);
        assert!(drawn.chrome);
        assert!(drawn.title_bar);
        assert_eq!(drawn.mark_rects, vec![[179, 3, 196, 23]]);
        assert_eq!(drawn.mark_icons.len(), 1);
        assert_eq!(
            (drawn.mark_icons[0].x, drawn.mark_icons[0].y),
            (181.0, 23.0)
        );
        assert_eq!(drawn.mark_icons[0].scale, Some(1.0));
        assert_eq!(drawn.title, Some([[8, 24, 165], [9, 25, 165]]));
        assert_eq!(batch.primitives[0].vertices.len(), 16);
    }

    #[test]
    fn a_form_page_clips_scrolls_and_counts_its_children() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        let page = draw_form_page(
            &mut batch,
            FormPagePlace {
                rect: GuiRect::corners(10.0, 20.0, 210.0, 140.0),
                inset: 5,
                top: 24,
                right: 200,
                bottom: 130,
                scroll_x: 0,
                scroll_y: 0,
                scale: [1.0, 1.0],
                flags: 0,
                own_background: false,
                parent_clip: [0, 0, 1000, 1000],
                clip_ok: true,
                lines: vec![
                    GuiRect::sized(0.0, 0.0, 10.0, 10.0),
                    GuiRect::sized(0.0, 20.0, 10.0, 10.0),
                ],
                controls: 5,
                children: vec![true, false, true],
            },
            4,
            &palette,
        );
        assert!(page.visible);
        assert_eq!(page.background_calls, 1);
        assert_eq!(page.clip, [15, 44, 195, 106]);
        assert_eq!(page.scroll, [-195, -106]);
        assert_eq!(page.origin, [205, 126]);
        assert_eq!(page.lines.len(), 2);
        assert_eq!(page.control_calls, 5);
        assert_eq!(page.child_calls, 2);
        assert_eq!(page.shadow, None);

        let locked = draw_form_page(
            &mut batch,
            FormPagePlace {
                rect: GuiRect::corners(10.0, 20.0, 210.0, 140.0),
                inset: 5,
                top: 24,
                right: 200,
                bottom: 130,
                scroll_x: 0,
                scroll_y: 0,
                scale: [1.0, 1.0],
                flags: 0x800,
                own_background: true,
                parent_clip: [0, 0, 1000, 1000],
                clip_ok: true,
                lines: Vec::new(),
                controls: 0,
                children: Vec::new(),
            },
            4,
            &palette,
        );
        assert_eq!(locked.scroll, [0, 0]);
        assert_eq!(locked.origin, [10, 20]);
        assert_eq!(locked.background_calls, 0);

        let hidden = draw_form_page(
            &mut batch,
            FormPagePlace {
                rect: GuiRect::corners(10.0, 20.0, 210.0, 140.0),
                inset: 5,
                top: 24,
                right: 200,
                bottom: 130,
                scroll_x: 0,
                scroll_y: 0,
                scale: [1.0, 1.0],
                flags: 0,
                own_background: false,
                parent_clip: [0, 0, 1000, 1000],
                clip_ok: false,
                lines: Vec::new(),
                controls: 3,
                children: vec![true],
            },
            4,
            &palette,
        );
        assert!(!hidden.visible);
        assert_eq!(hidden.control_calls, 0);
        assert_eq!(hidden.child_calls, 0);

        let clipped = draw_form_page(
            &mut batch,
            FormPagePlace {
                rect: GuiRect::corners(10.0, 20.0, 210.0, 140.0),
                inset: 5,
                top: 24,
                right: 200,
                bottom: 130,
                scroll_x: 0,
                scroll_y: 0,
                scale: [1.0, 1.0],
                flags: 0,
                own_background: false,
                parent_clip: [0, 0, 50, 50],
                clip_ok: true,
                lines: Vec::new(),
                controls: 0,
                children: Vec::new(),
            },
            4,
            &palette,
        );
        assert_eq!(clipped.clip, [15, 44, 35, 6]);
    }

    #[test]
    fn a_form_page_draws_its_drop_shadow_last() {
        let mut batch = SkinBatch::new();
        let palette = palette_for_faces();
        let page = draw_form_page(
            &mut batch,
            FormPagePlace {
                rect: GuiRect::corners(10.0, 20.0, 210.0, 140.0),
                inset: 5,
                top: 24,
                right: 200,
                bottom: 130,
                scroll_x: 0,
                scroll_y: 0,
                scale: [1.0, 1.0],
                flags: 0x80,
                own_background: true,
                parent_clip: [0, 0, 1000, 1000],
                clip_ok: true,
                lines: Vec::new(),
                controls: 0,
                children: Vec::new(),
            },
            4,
            &palette,
        );
        let shadow = page.shadow.expect("a shadow");
        assert_eq!((shadow.x0, shadow.y0), (-195.0, -106.0));
        assert_eq!((shadow.x1, shadow.y1), (0.0, 0.0));
        let primitive = batch.primitives.last().expect("a shadow draw");
        assert_eq!(primitive.vertices.len(), 16);
        assert_eq!(primitive.vertices[0].ink, [0, 0, 0, 0x30]);
    }

    fn label_place(style: u32, state: u32) -> ControlLabelPlace {
        ControlLabelPlace {
            rect: GuiRect::sized(10.0, 20.0, 100.0, 80.0),
            inset: 2,
            style,
            state,
            offset: 0,
            text_width: 40,
            line_height: 24,
            base: 18,
            text_len: 20,
            step: 7,
        }
    }

    #[test]
    fn a_control_label_aligns_by_its_style_word() {
        let (x, y) = place_control_text(&label_place(0x1000, 0), 5);
        assert_eq!((x, y), (-35, 23));
        let (x, y) = place_control_text(&label_place(0x2000, 0), 5);
        assert_eq!((x, y), (15, 23));
        let (x, y) = place_control_text(&label_place(0x3000, 0), 5);
        assert_eq!((x, y), (39, 23));
        let (x, y) = place_control_text(&label_place(0x4000, 0), 5);
        assert_eq!((x, y), (65, 23));
        let (x, y) = place_control_text(&label_place(0x5000, 0), 5);
        assert_eq!((x, y), (115, 23));
        let (_, y) = place_control_text(&label_place(0x100, 0), 5);
        assert_eq!(y, 18);
        let (_, y) = place_control_text(&label_place(0x300, 0), 5);
        assert_eq!(y, 73);
        let (_, y) = place_control_text(&label_place(0x400, 0), 5);
        assert_eq!(y, 117);
        let (_, y) = place_control_text(&label_place(0x500, 0), 5);
        assert_eq!(y, 100);
    }

    #[test]
    fn a_control_label_picks_the_focused_or_the_plain_ink() {
        let palette = palette_for_faces();
        let plain = draw_control_label(label_place(0, 0), LabelVariant::Plain, [0; 4], &palette, 5);
        assert_eq!(plain.ink, Some(palette.text[5]));
        let focused = draw_control_label(
            label_place(0, 0x10),
            LabelVariant::Plain,
            [0; 4],
            &palette,
            5,
        );
        assert_eq!(focused.ink, Some(palette.text[2]));
        let ink = draw_control_label(
            label_place(0, 0),
            LabelVariant::Ink,
            [7, 8, 9, 10],
            &palette,
            5,
        );
        assert_eq!(ink.ink, Some([7, 8, 9, 10]));
    }

    #[test]
    fn a_block_label_keeps_only_the_bytes_that_fit() {
        let palette = palette_for_faces();
        let block = draw_control_label(label_place(0, 0), LabelVariant::Block, [0; 4], &palette, 5);
        assert_eq!(block.keep, 13);
        assert_eq!(block.ink, Some(palette.text[5]));
        let coloured = draw_control_label(
            label_place(0, 0x1_0000),
            LabelVariant::Block,
            [0; 4],
            &palette,
            5,
        );
        assert!(coloured.coloured);
        assert_eq!(coloured.ink.expect("an ink")[3], 255);
    }

    #[test]
    fn a_slot_label_with_a_shadow_makes_two_calls() {
        let palette = palette_for_faces();
        let shadow = draw_control_label(
            label_place(0, 0x40),
            LabelVariant::Slot,
            [0; 4],
            &palette,
            5,
        );
        assert_eq!(shadow.calls, 2);
        assert_eq!(shadow.shadow, Some([(14, 23), (15, 24)]));
        let coloured = draw_control_label(
            label_place(0, 0x1_0000),
            LabelVariant::Slot,
            [0; 4],
            &palette,
            5,
        );
        assert!(coloured.coloured);
        assert_eq!(coloured.cross, Some(palette.highlight[0]));
        assert_eq!(coloured.mode_calls, 2);
    }
}
