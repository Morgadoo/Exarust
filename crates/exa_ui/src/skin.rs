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
const ICON_WIDE: u32 = 0x4000_0000;
const ICON_DIM: u32 = 0x8000_0000;
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
    pub wide: bool,
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
pub fn draw_centred_icon(icon: i32, place: IconPlace) -> CentredIcon {
    let x = i32::from(place.x).wrapping_add(centre_spare(place.w, icon));
    let y = i32::from(place.y).wrapping_add(centre_spare(place.h, icon));
    let wide = (place.mode & ICON_WIDE) != 0;
    if (place.state & ICON_MARK) == ICON_MARK {
        let nudged = if (place.state & ICON_BAND) == ICON_NUDGE {
            y.wrapping_add(1)
        } else {
            y
        };
        return CentredIcon {
            own: place.own,
            marked: true,
            wide,
            ink: 0,
            id: place.id,
            x,
            y: nudged,
            scale_bits: ICON_SCALE,
        };
    }
    let ink = if (place.state & SLIDER_PRESSED) == SLIDER_PRESSED {
        0
    } else if (place.mode & ICON_DIM) == 0 {
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
        wide,
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
/// the wider quad either way. The helper body and the other arm are
/// not in.
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
        assert!(!plain.wide);
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
        assert!(nudged.wide);
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
                mode: 0x8000_0000,
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
                mode: 0x8000_0000,
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
}
