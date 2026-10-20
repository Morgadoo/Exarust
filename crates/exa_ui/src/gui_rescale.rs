//! What the GUI's main unit (`RF_GUI_Main`) does when the GUI unit
//! changes, and two of its builders: every form scales its place, size
//! and controls from the old unit to the new, keeping its anchors (right
//! or bottom edges keep their scaled gaps, stretched edges both, centred
//! forms stay centred, a sized client keeps its frame); the common glyph
//! sheet fills the large icon atlas; and the message box lays out its text
//! and buttons. Rules: `docs/UI.md` (Forms).

use crate::form::ClientRect;

/// Anchor bits of a form's or control's place word.
pub mod anchor {
    /// Both horizontal edges keep their gaps (the width stretches).
    pub const STRETCH_X: u32 = 0x3_0000;
    /// The right edge keeps its gap.
    pub const RIGHT: u32 = 0x2_0000;
    /// Centred across the parent.
    pub const CENTRE_X: u32 = 0x800;
    /// Both vertical edges keep their gaps.
    pub const STRETCH_Y: u32 = 0xc_0000;
    /// The bottom edge keeps its gap.
    pub const BOTTOM: u32 = 0x8_0000;
    /// Centred down the parent.
    pub const CENTRE_Y: u32 = 0x8000;
}

/// A control record's place, in the record's 16-bit fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordPlace {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    /// The record's style word (its anchor bits).
    pub style: u32,
    /// The record's form keeps bottom-anchored controls at the bottom.
    pub form_bottom: bool,
}

/// A form's geometry as the rescale reads and writes it.
#[derive(Debug, Clone, PartialEq)]
pub struct FormPlace {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// The border in pixels, and the design border it is scaled from
    /// (0: taken from the border the first time).
    pub border: i32,
    pub design_border: i32,
    /// The title bar's height.
    pub title: i32,
    pub client: ClientRect,
    pub content_height: i32,
    /// Two more lengths the form scales (its minimum size).
    pub minimum: (i32, i32),
    /// The anchor bits.
    pub anchors: u32,
    /// The size is scaled as is (not its client).
    pub keep_size: bool,
    /// The height is the title bar's.
    pub title_only: bool,
    /// The form only holds others: it scales its minimum and its children.
    pub container: bool,
    pub children: Vec<FormPlace>,
    pub records: Vec<RecordPlace>,
}

/// Scales `v` from `old` to `unit` as the original: divided by the old
/// unit, then times the new.
fn s(v: i32, old: i32, unit: i32) -> i32 {
    if old == 0 { v } else { v / old * unit }
}

/// The 16-bit version, wrapping as the record's fields do.
fn s16(v: i16, old: i32, unit: i32) -> i16 {
    if old == 0 {
        return v;
    }
    ((i32::from(v) / old) as i16).wrapping_mul(unit as i16)
}

/// An odd 16-bit size made even by one more (`limit`: only under it).
fn even16(v: i16, limit: Option<i32>) -> i16 {
    let small = limit.is_none_or(|l| i32::from(v) < l);
    if v & 1 == 1 && small {
        v.wrapping_add(1)
    } else {
        v
    }
}

/// Half of a difference through the original's unsigned shift, kept to
/// 16 bits.
fn half16(d: i64) -> i16 {
    ((d as u64) >> 1) as u16 as i16
}

/// Rescales `form` from `old` to `unit`: `parent` is the size of the
/// parent's client before the change, `parent_now` the parent's client
/// size after it, `base` the skin's base measure (a title-only form's
/// height past its title), `realign` the form's own client layout (run
/// after each size change, as the original's).
pub fn rescale_form(
    form: &mut FormPlace,
    old: i32,
    unit: i32,
    parent: (i32, i32),
    parent_now: (i32, i32),
    base: i32,
    realign: &mut dyn FnMut(&mut FormPlace),
) {
    if form.design_border == 0 {
        form.design_border = form.border;
    }
    if form.container {
        realign(form);
        form.minimum = (s(form.minimum.0, old, unit), s(form.minimum.1, old, unit));
        for child in &mut form.children {
            rescale_form(child, old, unit, parent, parent_now, base, realign);
        }
        return;
    }
    let (cw, ch) = (form.client.width(), form.client.height());
    form.border = if form.design_border < 4 {
        form.design_border
    } else {
        crate::gui_font::round_i32(f64::from(unit) * f64::from(form.design_border) * 0.25)
    };
    let gap_x = s(parent.0 - form.w - form.x, old, unit);
    if form.anchors & anchor::STRETCH_X == anchor::STRETCH_X {
        form.x = s(form.x, old, unit);
        form.w = (parent_now.0 - gap_x) - form.x;
    } else {
        if form.keep_size {
            form.w = s(form.w, old, unit);
        } else {
            let target = s(cw, old, unit);
            realign(form);
            form.w = (form.w - form.client.width()) + target;
        }
        if form.anchors & anchor::STRETCH_X == anchor::RIGHT {
            form.x = (parent_now.0 - gap_x) - form.w;
        } else if form.anchors & anchor::CENTRE_X == anchor::CENTRE_X {
            form.x = ((i64::from(parent_now.0) - i64::from(form.w)) / 2) as i32;
        } else {
            form.x = s(form.x, old, unit);
        }
    }
    form.content_height = s(form.content_height, old, unit);
    let gap_y = s(parent.1 - form.h - form.y, old, unit);
    if form.title_only {
        form.y = s(form.y, old, unit);
        form.h = form.title + base;
    } else if form.anchors & anchor::STRETCH_Y == anchor::STRETCH_Y {
        form.y = s(form.y, old, unit);
        form.h = (parent_now.1 - gap_y) - form.y;
    } else {
        if form.keep_size {
            form.h = s(form.h, old, unit);
        } else {
            let target = s(ch, old, unit);
            realign(form);
            form.h = (form.h - form.client.height()) + target;
        }
        if form.anchors & anchor::STRETCH_Y == anchor::BOTTOM {
            form.y = (parent_now.1 - gap_y) - form.h;
        } else if form.anchors & anchor::CENTRE_Y == anchor::CENTRE_Y {
            form.y = ((i64::from(parent_now.1) - i64::from(form.h)) / 2) as i32;
        } else {
            form.y = s(form.y, old, unit);
        }
    }
    realign(form);
    form.minimum = (s(form.minimum.0, old, unit), s(form.minimum.1, old, unit));
    let now = (form.client.width(), form.client.height());
    for child in &mut form.children {
        rescale_form(child, old, unit, (cw, ch), now, base, realign);
    }
    let (new_w, view) = (form.client.width() as i16, form.client.height() as i16);
    for r in &mut form.records {
        rescale_record(r, old, unit, (cw, ch), (new_w, view));
    }
}

/// Rescales one control record inside a client that was `old_client`
/// and is now `now` (width, height).
pub fn rescale_record(
    r: &mut RecordPlace,
    old: i32,
    unit: i32,
    old_client: (i32, i32),
    now: (i16, i16),
) {
    let u = unit as i16;
    let gap =
        (((old_client.0 - i32::from(r.w) - i32::from(r.x)) / old.max(1)) as i16).wrapping_mul(u);
    if r.style & anchor::STRETCH_X == anchor::STRETCH_X {
        r.x = s16(r.x, old, unit);
        r.w = now.0.wrapping_sub(gap).wrapping_sub(r.x);
    } else {
        r.w = even16(s16(r.w, old, unit), Some(unit * 16));
        if r.style & anchor::CENTRE_X == anchor::CENTRE_X {
            r.x = ((i32::from(now.0) - i32::from(r.w)) / 2) as i16;
        } else if r.style & anchor::STRETCH_X == anchor::RIGHT {
            r.x = now.0.wrapping_sub(gap).wrapping_sub(r.w);
        } else {
            r.x = s16(r.x, old, unit);
        }
    }
    if r.style & anchor::STRETCH_Y == anchor::BOTTOM && r.form_bottom {
        let old_h = r.h;
        r.h = even16(s16(r.h, old, unit), None);
        let gap = (((old_client.1 - i32::from(old_h) - i32::from(r.y)) / old.max(1)) as i16)
            .wrapping_mul(u);
        r.y = now.1.wrapping_sub(gap).wrapping_sub(r.h);
    } else {
        if old * 8 < i32::from(r.h) {
            r.y = r.y.wrapping_add(old as i16);
        } else {
            r.y =
                r.y.wrapping_add(half16(i64::from(r.h) - i64::from(old << 2)));
        }
        if old < i32::from(r.h) {
            r.h = even16(s16(r.h, old, unit), None);
        }
        r.y = s16(r.y, old, unit);
        if unit * 8 < i32::from(r.h) {
            r.y = r.y.wrapping_sub(u);
        } else {
            r.y =
                r.y.wrapping_sub(half16(i64::from(r.h) - i64::from(unit << 2)));
        }
    }
}

/// The common glyph sheet the large atlas is filled from.
pub const GLYPH_SHEET: &str = "GlphCommon";
/// The sheet's icons: 64-pixel squares at these corners, in the order the
/// atlas takes them (handle = position + 1).
pub const SHEET_ICONS: [(i32, i32); 19] = [
    (0, 0),
    (64, 0),
    (128, 0),
    (192, 64),
    (128, 64),
    (0, 128),
    (64, 64),
    (0, 64),
    (192, 192),
    (64, 128),
    (128, 128),
    (192, 128),
    (0, 192),
    (64, 192),
    (128, 192),
    (192, 0),
    (0, 256),
    (64, 256),
    (192, 256),
];
/// The side of a sheet icon.
pub const SHEET_ICON: u16 = 64;

/// The message box's layout: its size and where its label and buttons go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageBoxLayout {
    /// The form's width and height.
    pub size: (i32, i32),
    /// Its top in design pixels: the screen top over the unit (truncated),
    /// times 4, the design unit.
    pub y: i32,
    /// The label's place (x, y, w): 12, 4 and the width less 24.
    pub label: (i32, i32, i32),
    /// The buttons' places (x, y, w) on the 4-pixel grid; "Ok" then "Cancel".
    pub buttons: Vec<(&'static str, i32, i32, i32)>,
}

/// Message box flags.
pub mod message_box {
    pub const OK: u32 = 0x1;
    pub const CANCEL: u32 = 0x2;
    /// Placed a quarter down the screen rather than centred.
    pub const HIGH: u32 = 0x10;
}

/// Lays a message box out for a text `text_width` wide in `Rayform08R`:
/// the width is the text plus 64, at least 320; the buttons, 80 wide and
/// 96 apart, are centred under it at grid row 13; the form is the width
/// plus twice the skin's `chrome` and 104 tall, centred (or a quarter
/// down) in a screen `screen_height` tall, its top turned into design
/// pixels (over the unit, times 4).
pub fn message_box_layout(
    text_width: i32,
    flags: u32,
    screen_height: i32,
    chrome: i32,
    unit: i32,
) -> MessageBoxLayout {
    let width = (text_width + 0x40).max(0x140);
    let mut names = Vec::new();
    if flags & message_box::OK != 0 {
        names.push("Ok");
    }
    if flags & message_box::CANCEL != 0 {
        names.push("Cancel");
    }
    let n = names.len() as i64;
    let start = ((i64::from(width) + (n * 0x18 - 4) * -4) as u64 >> 1) as u16 as i16;
    let buttons = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            (
                *name,
                i32::from((i as i16).wrapping_mul(0x60).wrapping_add(start)),
                0xd,
                0x50,
            )
        })
        .collect();
    let h = 0x68;
    let y = if flags & message_box::HIGH == 0 {
        (((i64::from(screen_height) - i64::from(h)) as u64 >> 1) as i32) - ((h as u32 >> 1) as i32)
    } else {
        ((i64::from(screen_height) - i64::from(h)) as u64 >> 2) as i32
    };
    MessageBoxLayout {
        size: (chrome * 2 + width, h),
        y: ((i64::from(y) / i64::from(unit.max(1))) << 2) as i32,
        label: (0xc, 4, width - 0xc * 2),
        buttons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plain realign: the client is the form less its border.
    fn inset(f: &mut FormPlace) {
        f.client = ClientRect {
            left: f.border,
            top: f.border,
            right: f.w - f.border,
            bottom: f.h - f.border,
        };
    }

    fn form(x: i32, y: i32, w: i32, h: i32, anchors: u32) -> FormPlace {
        let mut f = FormPlace {
            x,
            y,
            w,
            h,
            border: 2,
            design_border: 0,
            title: 0,
            client: ClientRect::default_like(),
            content_height: h,
            minimum: (40, 20),
            anchors,
            keep_size: false,
            title_only: false,
            container: false,
            children: Vec::new(),
            records: Vec::new(),
        };
        inset(&mut f);
        f
    }

    impl ClientRect {
        fn default_like() -> Self {
            ClientRect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            }
        }
    }

    #[test]
    fn a_plain_form_scales_its_client_and_keeps_its_frame() {
        let mut f = form(40, 20, 104, 64, 0);
        rescale_form(&mut f, 4, 8, (800, 600), (800, 600), 46, &mut inset);
        // The client (100 × 60) doubles; the 2-pixel border stays.
        assert_eq!((f.x, f.y, f.w, f.h), (80, 40, 204, 124));
        assert_eq!((f.client.width(), f.client.height()), (200, 120));
        assert_eq!((f.minimum, f.content_height), ((80, 40), 128));
    }

    #[test]
    fn anchored_forms_keep_their_gaps_or_centre() {
        // Right and bottom: 800 − 104 − 680 = 16 to the right, 600 − 64 −
        // 520 = 16 below, both doubled.
        let mut f = form(680, 520, 104, 64, anchor::RIGHT | anchor::BOTTOM);
        rescale_form(&mut f, 4, 8, (800, 600), (800, 600), 46, &mut inset);
        assert_eq!((f.x, f.y), (800 - 32 - 204, 600 - 32 - 124));
        let mut c = form(0, 0, 104, 64, anchor::CENTRE_X | anchor::CENTRE_Y);
        rescale_form(&mut c, 4, 8, (800, 600), (800, 600), 46, &mut inset);
        assert_eq!((c.x, c.y), ((800 - 204) / 2, (600 - 124) / 2));
        let mut s = form(8, 8, 784, 64, anchor::STRETCH_X);
        rescale_form(&mut s, 4, 8, (800, 600), (800, 600), 46, &mut inset);
        assert_eq!((s.x, s.w), (16, 800 - 16 - 16));
        // A title-only form is its title bar and the base.
        let mut t = form(8, 8, 100, 30, 0);
        t.title_only = true;
        t.title = 12;
        rescale_form(&mut t, 4, 8, (800, 600), (800, 600), 46, &mut inset);
        assert_eq!((t.y, t.h), (16, 58));
    }

    #[test]
    fn a_large_border_scales_with_the_unit() {
        let mut f = form(0, 0, 104, 64, 0);
        f.border = 8;
        rescale_form(&mut f, 4, 6, (800, 600), (800, 600), 46, &mut inset);
        assert_eq!((f.design_border, f.border), (8, 12));
    }

    #[test]
    fn records_scale_on_their_grid() {
        // A 41 × 20 button at 10, 10 in a 100 × 60 client, unit 4 → 8.
        let mut r = RecordPlace {
            x: 10,
            y: 10,
            w: 41,
            h: 20,
            style: 0,
            form_bottom: false,
        };
        rescale_record(&mut r, 4, 8, (100, 60), (200, 120));
        // x: 10 / 4 = 2, × 8 = 16; the width 41 / 4 × 8 = 80 is even; the
        // height 20 → 40; y: 10 + (20 − 16) / 2 = 12, / 4 × 8 = 24, less
        // (40 − 32) / 2 = 20.
        assert_eq!((r.x, r.y, r.w, r.h), (16, 20, 80, 40));
        // An odd width under 16 units grows by one.
        let mut o = RecordPlace { w: 7, ..r };
        o.x = 0;
        rescale_record(&mut o, 4, 5, (100, 60), (125, 75));
        assert_eq!(o.w, 6);
        let mut odd = RecordPlace {
            w: 12,
            x: 0,
            y: 0,
            h: 4,
            style: 0,
            form_bottom: false,
        };
        rescale_record(&mut odd, 4, 3, (100, 60), (75, 45));
        assert_eq!(odd.w, 10);
        // Right-anchored keeps its scaled gap; stretched keeps both.
        let mut right = RecordPlace {
            x: 50,
            y: 0,
            w: 40,
            h: 4,
            style: anchor::RIGHT,
            form_bottom: false,
        };
        rescale_record(&mut right, 4, 8, (100, 60), (200, 120));
        assert_eq!((right.w, right.x), (80, 200 - 16 - 80));
        let mut wide = RecordPlace {
            x: 10,
            y: 0,
            w: 80,
            h: 4,
            style: anchor::STRETCH_X,
            form_bottom: false,
        };
        rescale_record(&mut wide, 4, 8, (100, 60), (200, 120));
        assert_eq!((wide.x, wide.w), (16, 200 - 16 - 16));
        // Bottom-anchored in a form that keeps them: its gap below doubles.
        let mut low = RecordPlace {
            x: 0,
            y: 30,
            w: 8,
            h: 10,
            style: anchor::BOTTOM,
            form_bottom: true,
        };
        rescale_record(&mut low, 4, 8, (100, 60), (200, 120));
        assert_eq!((low.h, low.y), (16, 120 - 40 - 16));
    }

    #[test]
    fn the_glyph_sheet_order() {
        assert_eq!(SHEET_ICONS.len(), 19);
        // Handle 17, the item view's default marker, is the square at (0, 256).
        assert_eq!(SHEET_ICONS[16], (0, 256));
        assert_eq!(
            (SHEET_ICONS[0], SHEET_ICONS[3], SHEET_ICONS[18]),
            ((0, 0), (192, 64), (192, 256))
        );
    }

    #[test]
    fn the_message_box() {
        let m = message_box_layout(100, message_box::OK | message_box::CANCEL, 768, 16, 4);
        // 320 wide; two buttons: (320 − (48 − 4)·4) / 2 = 72, then 168.
        assert_eq!(m.size, (352, 104));
        assert_eq!(m.label, (12, 4, 296));
        assert_eq!(m.buttons, vec![("Ok", 72, 13, 80), ("Cancel", 168, 13, 80)]);
        // (768 − 104) / 2 − 52 = 280, over the unit times 4 = 280.
        assert_eq!(m.y, 280);
        let one = message_box_layout(400, message_box::OK | message_box::HIGH, 768, 16, 5);
        assert_eq!(one.size.0, 16 * 2 + 464);
        assert_eq!(one.buttons, vec![("Ok", (464 - 80) / 2, 13, 80)]);
        assert_eq!(one.y, (768 - 104) / 4 / 5 * 4);
    }
}
