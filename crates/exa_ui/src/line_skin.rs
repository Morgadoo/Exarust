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
