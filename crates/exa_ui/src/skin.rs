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
}
