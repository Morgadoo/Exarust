//! The Settings and Controls forms, rebuilt from the binary's 0x68-byte
//! control records (`TSGSettingDlg`'s tab, Settings and Controls arrays;
//! `docs/UI.md`, Forms). Positions are design pixels at GUI unit 4; y
//! anchors are on the 4-px grid and are scaled by the live unit.

/// The design dialog size at unit 4.
pub const DESIGN: (i32, i32) = (434, 536);
/// The Settings list's client height at unit 4.
pub const SETTINGS_HEIGHT: i32 = 408;
/// The Controls list's client height at unit 4.
pub const CONTROLS_HEIGHT: i32 = 416;
/// The Controls list's content height at unit 4.
pub const CONTROLS_CONTENT: i32 = 676;

/// A rectangle in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A control record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    pub class: u16,
    pub id: u16,
    /// Pixels, or an anchor code 0x8000..0x800a.
    pub x: i32,
    /// 4-px grid units, or an anchor code.
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub text: Option<&'static str>,
    pub style: u32,
    pub state: u32,
}

#[allow(clippy::too_many_arguments)]
const fn rec(
    class: u16,
    id: u16,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    text: Option<&'static str>,
    style: u32,
    state: u32,
) -> Record {
    Record {
        class,
        id,
        x,
        y,
        w,
        h,
        text,
        style,
        state,
    }
}

/// The default skin sizes `(w, h)` by class.
pub fn skin(class: u16) -> (i32, i32) {
    match class {
        0x01 => (80, 24),
        0x02 => (16, 16),
        0x03 => (28, 28),
        0x04 => (0, 16),
        0x05 => (0, 20),
        0x13 => (0, 3),
        0x10 => (60, 20),
        0x11 | 0x12 => (80, 24),
        _ => (0, 0),
    }
}

/// The y-anchor delta in grid units.
pub fn y_delta(code: i32) -> Option<(i32, bool)> {
    Some(match code {
        0x8000 => (0, false),
        0x8002 => (13, false),
        0x8003 => (12, false),
        0x8004 => (11, false),
        0x8005 => (10, false),
        0x8006 => (9, false),
        0x8007 => (8, false),
        0x8008 => (7, false),
        0x8009 => (6, false),
        0x800a => (5, false),
        _ => return None,
    })
}

/// The x gap after the previous record's right edge.
pub fn x_gap(code: i32) -> Option<i32> {
    Some(match code {
        0x8000 => 0,
        0x8002 => 72,
        0x8003 => 56,
        0x8004 => 32,
        0x8005 => 20,
        0x8006 => 16,
        0x8007 => 12,
        0x8008 => 8,
        0x8009 => 4,
        0x800a => 0,
        _ => return None,
    })
}

/// Resolves records to rectangles in design pixels, adding the child offset.
/// The first row's absolute y is in grid units; anchors add their delta to
/// the previous row, negated when style bit 0x80000 is set.
pub fn place(records: &[Record], y_offset: i32) -> Vec<Rect> {
    let mut out = Vec::with_capacity(records.len());
    let mut prev = Rect::default();
    let mut prev_y = 0;
    for (i, r) in records.iter().enumerate() {
        let (yu, negative) = if let Some((delta, neg)) = y_delta(r.y) {
            (prev_y + delta, neg)
        } else {
            (r.y, false)
        };
        let y = if negative { prev.y - yu * 4 } else { yu * 4 } + y_offset;
        let x = if i == 0 {
            if r.x >= 0x8000 { 4 } else { r.x }
        } else if r.x >= 0x8000 {
            if r.x == 0x8000 {
                prev.x
            } else if r.x == 0x8001 {
                if prev.y == y { prev.x + prev.w + 4 } else { 0 }
            } else {
                prev.x + prev.w + x_gap(r.x).unwrap_or(0)
            }
        } else {
            r.x
        };
        let (dw, dh) = skin(r.class);
        let w = if r.w == 0 { dw } else { r.w };
        let h = if r.h == 0 { dh } else { r.h };
        let w = (w as f32 * 0.25).round_ties_even() as i32 * 4;
        let h = (h as f32 * 0.25).round_ties_even() as i32 * 4;
        let rect = Rect { x, y, w, h };
        out.push(rect);
        prev = rect;
        prev_y = yu;
    }
    out
}

/// The tab/button array (9 records; Cancel may be omitted).
pub const TABS: [Record; 9] = [
    rec(1, 0, 12, 5, 192, 32, Some("SETTINGS"), 0x0040_3320, 0x60),
    rec(1, 0, 212, 0x8000, 192, 32, Some("CONTROLS"), 0x0040_3320, 0),
    rec(
        1,
        0,
        12,
        0x8006,
        192,
        0,
        Some("Keyboard"),
        0x0400_3320,
        0x170,
    ),
    rec(
        1,
        0,
        212,
        0x8000,
        192,
        0,
        Some("Controller"),
        0x0040_3300,
        0x110,
    ),
    rec(0x13, 0, 16, 0x8008, 400, 0, None, 0, 0),
    rec(0x13, 0, 16, 116, 400, 0, None, 0, 0),
    rec(1, 0, 40, 0x8009, 100, 0, Some("Defaults"), 0x2040_3300, 0),
    rec(1, 0, 0x8002, 0x8000, 104, 0, Some("Apply"), 0x2040_3300, 0),
    rec(1, 0, 0x8002, 0x8000, 100, 0, Some("Cancel"), 0x2040_3300, 0),
];

/// The Settings row array (27 records).
pub const SETTINGS: [Record; 27] = [
    rec(5, 0, 16, 0, 60, 0, Some("Video Device: "), 0, 0),
    rec(5, 1, 168, 0x8000, 248, 0, Some("Unknown"), 0, 0x2000_2380),
    rec(5, 0, 16, 0x8008, 60, 0, Some("Video Memory: "), 0, 0),
    rec(5, 2, 168, 0x8000, 248, 0, None, 0, 0x2380),
    rec(0x13, 0, 16, 0x8009, 400, 0, None, 0, 0),
    rec(5, 0, 16, 0x8009, 60, 0, Some("Audio volume: "), 0, 0),
    rec(3, 3, 168, 0x8000, 140, 0, None, 0, 0),
    rec(2, 4, 320, 0x8000, 56, 0, Some("Music"), 0, 0),
    rec(5, 0, 16, 0x8006, 60, 0, Some("Super sampling: "), 0, 0),
    rec(4, 5, 168, 0x8000, 140, 0, None, 0, 0),
    rec(5, 0, 16, 0x8007, 60, 0, Some("Shading quality: "), 0, 0),
    rec(4, 6, 168, 0x8000, 140, 0, None, 0, 0),
    rec(5, 0, 16, 0x8007, 60, 0, Some("Texture detail: "), 0, 0),
    rec(4, 7, 168, 0x8000, 140, 0, None, 0, 0),
    rec(5, 0, 16, 0x8008, 60, 0, Some("Texture quality: "), 0, 0),
    rec(4, 8, 168, 0x8000, 140, 0, None, 0, 0),
    rec(5, 0, 16, 0x8007, 60, 0, Some("Shadow detail: "), 0, 0),
    rec(4, 9, 168, 0x8000, 140, 0, None, 0, 0),
    rec(5, 0, 16, 0x8008, 60, 0, Some("Shadow quality: "), 0, 0),
    rec(4, 11, 168, 0x8000, 140, 0, None, 0, 0),
    rec(5, 0, 16, 0x8007, 60, 0, Some("HW antialiasing: "), 0, 0),
    rec(4, 12, 168, 0x8000, 140, 0, None, 0, 0),
    rec(2, 13, 72, 0x8007, 80, 0, Some("Vertical sync "), 0, 0),
    rec(2, 14, 228, 0x8000, 80, 0, Some("Show framerate"), 0, 0),
    rec(2, 15, 72, 0x8008, 80, 0, Some("Window mode"), 0, 0),
    rec(2, 16, 228, 0x8000, 80, 0, Some("Show gameplay tips"), 0, 0),
    rec(2, 17, 148, 0x8008, 100, 0, Some("Early Checkpoints"), 0, 0),
];

/// One Controls binding: a label row and its key cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    pub label: &'static str,
    pub action: u16,
    pub key: &'static str,
    /// The row's y in design pixels.
    pub y: i32,
    /// Whether the row has a key cell (Power Tree does not).
    pub keyed: bool,
}

const fn bind(label: &'static str, action: u16, key: &'static str, y: i32) -> Binding {
    Binding {
        label,
        action,
        key,
        y,
        keyed: true,
    }
}

/// The Controls bindings (22 label rows, 21 key cells).
pub const BINDINGS: [Binding; 22] = [
    bind("Forward", 1, "W", 8),
    bind("Back", 2, "S", 36),
    bind("Left", 3, "A", 64),
    bind("Right", 4, "D", 92),
    bind("Run/Sprint", 5, "Shift", 120),
    bind("Crouch", 6, "X", 148),
    bind("Orbit Camera", 7, "", 180),
    bind("Cursor Look", 8, "SPACE", 208),
    bind("Toggle Combat", 0xA, "TAB", 240),
    bind("Alternate Attack", 0xE, "Alt", 268),
    bind("Switch Equipped", 0xF, "R", 296),
    bind("Flip weapon", 0x17, "F", 324),
    bind("Interact", 9, "Ctrl", 352),
    bind("Inventory", 0xB, "I", 384),
    bind("Skills", 0xC, "K", 412),
    bind("Journal", 0xD, "J", 440),
    bind("Talk", 0x10, "T", 468),
    bind("Use Power", 0x15, "", 504),
    bind("Power Bar", 0x11, "", 532),
    bind("Previous Power", 0x13, "", 560),
    bind("Next Power", 0x14, "", 588),
    Binding {
        label: "Power Tree",
        action: 0,
        key: "",
        y: 616,
        keyed: false,
    },
];

/// The complete form layout at a GUI unit (4, 5, 6, 8 or 12).
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub form: Rect,
    pub tabs: Vec<Rect>,
    pub settings: Vec<Rect>,
    pub bindings: Vec<Rect>,
    pub separators: [Rect; 2],
    pub buttons: [Rect; 3],
    pub settings_viewport: Rect,
    pub controls_viewport: Rect,
    pub controls_content: i32,
}

/// Builds the form at `unit`, scaling the unit-4 design by `unit/4`.
pub fn layout(unit: i32) -> Layout {
    let scale = unit as f32 / 4.0;
    let s = |v: i32| (v as f32 * scale) as i32;
    let form = Rect {
        x: 0,
        y: 0,
        w: s(DESIGN.0),
        h: s(DESIGN.1),
    };
    let tabs = place(&TABS, 0);
    let settings = place(&SETTINGS, 64);
    let mut bindings = Vec::with_capacity(BINDINGS.len() * 2);
    for b in BINDINGS {
        bindings.push(Rect {
            x: s(80),
            y: s(56 + b.y),
            w: s(68),
            h: s(20),
        });
        if b.keyed {
            bindings.push(Rect {
                x: s(232),
                y: s(56 + b.y),
                w: s(100),
                h: s(20),
            });
        }
    }
    let scale_rect = |r: Rect| Rect {
        x: s(r.x),
        y: s(r.y),
        w: s(r.w),
        h: s(r.h),
    };
    let tabs: Vec<Rect> = tabs.into_iter().map(scale_rect).collect();
    let settings: Vec<Rect> = settings.into_iter().map(scale_rect).collect();
    Layout {
        form,
        separators: [tabs[4], tabs[5]],
        buttons: [tabs[6], tabs[7], tabs[8]],
        tabs: tabs[..4].to_vec(),
        settings,
        bindings,
        settings_viewport: Rect {
            x: 0,
            y: s(64),
            w: s(426),
            h: s(SETTINGS_HEIGHT),
        },
        controls_viewport: Rect {
            x: 0,
            y: s(56),
            w: s(426),
            h: s(CONTROLS_HEIGHT),
        },
        controls_content: s(CONTROLS_CONTENT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skin_sizes_and_anchors() {
        assert_eq!(skin(1), (80, 24));
        assert_eq!(skin(2), (16, 16));
        assert_eq!(skin(3), (28, 28));
        assert_eq!(skin(4), (0, 16));
        assert_eq!(skin(5), (0, 20));
        assert_eq!(skin(0x13), (0, 3));
        assert_eq!(y_delta(0x8000), Some((0, false)));
        assert_eq!(y_delta(0x8006), Some((9, false)));
        assert_eq!(y_delta(0x8008), Some((7, false)));
        assert_eq!(x_gap(0x8002), Some(72));
        assert_eq!(x_gap(0x800a), Some(0));
    }

    #[test]
    fn the_records_place_at_unit_four() {
        let l = layout(4);
        assert_eq!((l.form.w, l.form.h), (434, 536));
        assert_eq!(
            l.tabs[0],
            Rect {
                x: 12,
                y: 20,
                w: 192,
                h: 32
            }
        );
        assert_eq!(
            l.tabs[1],
            Rect {
                x: 212,
                y: 20,
                w: 192,
                h: 32
            }
        );
        assert_eq!(
            l.settings[0],
            Rect {
                x: 16,
                y: 64,
                w: 60,
                h: 20
            }
        );
        assert_eq!(
            l.settings[1],
            Rect {
                x: 168,
                y: 64,
                w: 248,
                h: 20
            }
        );
        assert_eq!(
            l.settings[3],
            Rect {
                x: 168,
                y: 92,
                w: 248,
                h: 20
            }
        );
        assert_eq!(
            l.settings[7],
            Rect {
                x: 320,
                y: 140,
                w: 56,
                h: 16
            }
        );
        assert_eq!(l.settings[26].y, 448);
        assert_eq!(l.settings[26].y + 20, 468);
        assert_eq!(
            l.separators[0],
            Rect {
                x: 16,
                y: 84,
                w: 400,
                h: 4
            }
        );
        assert_eq!(
            l.buttons[0],
            Rect {
                x: 40,
                y: 488,
                w: 100,
                h: 24
            }
        );
        assert_eq!(
            l.buttons[2],
            Rect {
                x: 388,
                y: 488,
                w: 100,
                h: 24
            }
        );
    }

    #[test]
    fn the_bindings_and_scroll() {
        let l = layout(4);
        assert_eq!(
            l.bindings[0],
            Rect {
                x: 80,
                y: 64,
                w: 68,
                h: 20
            }
        );
        assert_eq!(
            l.bindings[1],
            Rect {
                x: 232,
                y: 64,
                w: 100,
                h: 20
            }
        );
        assert_eq!(l.bindings.last().unwrap().y, 56 + 616);
        assert_eq!(l.controls_content, 676);
        assert_eq!(l.controls_viewport.h, 416);
    }

    #[test]
    fn the_layout_scales_with_the_unit() {
        let l = layout(6);
        assert_eq!((l.form.w, l.form.h), (651, 804));
        assert_eq!(
            l.tabs[0],
            Rect {
                x: 18,
                y: 30,
                w: 288,
                h: 48
            }
        );
        assert_eq!(
            l.settings[0],
            Rect {
                x: 24,
                y: 96,
                w: 90,
                h: 30
            }
        );
        assert_eq!(l.bindings[0].y, 96);
    }
}

/// A switched page: what is visible and the child's rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSwitch {
    pub page: u8,
    pub settings_visible: bool,
    pub controls_visible: bool,
    pub child: Rect,
}

/// The page switch: page 0 shows the Settings child at
/// (0,64,426,408); page 1 shows the Controls child at a y of 56 (widescreen)
/// or 96, height 472−y. The unit-4 values scale by `unit/4`.
pub fn switch_page(page: u8, widescreen: bool, unit: i32) -> Option<PageSwitch> {
    let s = |v: i32| v * unit / 4;
    match page {
        0 => Some(PageSwitch {
            page,
            settings_visible: true,
            controls_visible: false,
            child: Rect {
                x: 0,
                y: s(64),
                w: s(426),
                h: s(SETTINGS_HEIGHT),
            },
        }),
        1 => {
            let y = if widescreen { 56 } else { 96 };
            Some(PageSwitch {
                page,
                settings_visible: false,
                controls_visible: true,
                child: Rect {
                    x: 0,
                    y: s(y),
                    w: s(426),
                    h: s(472 - y),
                },
            })
        }
        _ => None,
    }
}

/// The Controls list's scrollbar at fraction `f`: the 412-px client of the
/// 416-px child over 676-px content, a 15-px rail and a minimum 9-px thumb.
/// Returns `(thumb_height, thumb_y)` scaled by `unit/4`.
pub fn controls_scroll(f: f32, unit: i32) -> (i32, i32) {
    let viewport = 412;
    let content = CONTROLS_CONTENT;
    let rail = 15;
    let track = viewport - 2 * rail;
    let mut thumb = (track as f64 * viewport as f64 / content as f64) as i32;
    thumb = thumb.clamp(9, track);
    let y = 17 + ((track - thumb) as f32 * f.clamp(0.0, 1.0)) as i32;
    (thumb * unit / 4, y * unit / 4)
}

#[cfg(test)]
mod page_tests {
    use super::*;

    #[test]
    fn the_page_switch() {
        let settings = switch_page(0, true, 4).unwrap();
        assert!(settings.settings_visible && !settings.controls_visible);
        assert_eq!(
            settings.child,
            Rect {
                x: 0,
                y: 64,
                w: 426,
                h: 408
            }
        );
        let controls = switch_page(1, true, 4).unwrap();
        assert!(!controls.settings_visible && controls.controls_visible);
        assert_eq!(
            controls.child,
            Rect {
                x: 0,
                y: 56,
                w: 426,
                h: 416
            }
        );
        let tall = switch_page(1, false, 4).unwrap();
        assert_eq!(
            tall.child,
            Rect {
                x: 0,
                y: 96,
                w: 426,
                h: 376
            }
        );
        assert!(switch_page(2, true, 4).is_none());
    }

    #[test]
    fn the_controls_scroll() {
        assert_eq!(controls_scroll(0.0, 4), (232, 17));
        assert_eq!(controls_scroll(1.0, 4), (232, 167));
        assert_eq!(controls_scroll(0.5, 6), (348, 138));
    }
}
