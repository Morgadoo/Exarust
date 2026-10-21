//! The Settings and Controls forms, rebuilt from the binary's 0x68-byte
//! control records (`TSGSettingDlg`'s tab, Settings and Controls arrays;
//! `docs/UI.md`, Forms). Positions are design pixels at GUI unit 4; y
//! anchors are on the 4-px grid and are scaled by the live unit.
//!
//! The option rows' behaviour is here too: the row ids, what each slider
//! step or checkbox writes ([`on_slider`], [`on_toggle`]), what the rows show
//! when the page opens ([`populate`]) and the labels from
//! [`exa_formats::hw_config`]. The form's four buttons and two tabs are
//! [`form_action`].

use exa_formats::hw_config::{self, Options, Scale};

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
    /// The original's action code for the row.
    pub action: u16,
    /// The row's slot in the 23-entry binding table, when it has one.
    pub slot: Option<usize>,
    /// The template's key text (the shipped table's key may differ).
    pub key: &'static str,
    /// The row's y in design pixels.
    pub y: i32,
    /// Whether the row has a key cell (Power Tree does not).
    pub keyed: bool,
}

const fn bind(label: &'static str, action: u16, slot: usize, key: &'static str, y: i32) -> Binding {
    Binding {
        label,
        action,
        slot: Some(slot),
        key,
        y,
        keyed: true,
    }
}

/// The Controls bindings (22 label rows, 21 key cells).
pub const BINDINGS: [Binding; 22] = [
    bind("Forward", 1, 1, "W", 8),
    bind("Back", 2, 2, "S", 36),
    bind("Left", 3, 3, "A", 64),
    bind("Right", 4, 4, "D", 92),
    bind("Run/Sprint", 5, 6, "Shift", 120),
    bind("Crouch", 6, 5, "X", 148),
    bind("Orbit Camera", 7, 8, "", 180),
    bind("Cursor Look", 8, 9, "SPACE", 208),
    bind("Toggle Combat", 0xA, 12, "TAB", 240),
    bind("Alternate Attack", 0xE, 7, "Alt", 268),
    bind("Switch Equipped", 0xF, 14, "R", 296),
    bind("Flip weapon", 0x17, 15, "F", 324),
    bind("Interact", 9, 10, "Ctrl", 352),
    bind("Inventory", 0xB, 11, "I", 384),
    bind("Skills", 0xC, 13, "K", 412),
    bind("Journal", 0xD, 16, "J", 440),
    bind("Talk", 0x10, 17, "T", 468),
    bind("Use Power", 0x15, 18, "", 504),
    bind("Power Bar", 0x11, 19, "", 532),
    bind("Previous Power", 0x13, 22, "", 560),
    bind("Next Power", 0x14, 21, "", 588),
    Binding {
        label: "Power Tree",
        action: 0,
        slot: None,
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

/// An editable row of the Settings page.
///
/// The ids are the tab dialog's control records: the two readouts are 1 and
/// 2, then the sliders and checkboxes in the order the page shows them (id
/// 10 is unused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Volume,
    Music,
    Supersample,
    ShadingQuality,
    TextureDetail,
    TextureQuality,
    ShadowDetail,
    ShadowQuality,
    Multisample,
    VerticalSync,
    ShowFps,
    WindowMode,
    ShowTips,
    Checkpoints,
}

impl Setting {
    /// The row's control id.
    pub fn id(self) -> u16 {
        match self {
            Setting::Volume => 3,
            Setting::Music => 4,
            Setting::Supersample => 5,
            Setting::ShadingQuality => 6,
            Setting::TextureDetail => 7,
            Setting::TextureQuality => 8,
            Setting::ShadowDetail => 9,
            Setting::ShadowQuality => 11,
            Setting::Multisample => 12,
            Setting::VerticalSync => 13,
            Setting::ShowFps => 14,
            Setting::WindowMode => 15,
            Setting::ShowTips => 16,
            Setting::Checkpoints => 17,
        }
    }

    /// The row with control id `id`, if it is editable.
    pub fn from_id(id: u16) -> Option<Self> {
        Some(match id {
            3 => Setting::Volume,
            4 => Setting::Music,
            5 => Setting::Supersample,
            6 => Setting::ShadingQuality,
            7 => Setting::TextureDetail,
            8 => Setting::TextureQuality,
            9 => Setting::ShadowDetail,
            11 => Setting::ShadowQuality,
            12 => Setting::Multisample,
            13 => Setting::VerticalSync,
            14 => Setting::ShowFps,
            15 => Setting::WindowMode,
            16 => Setting::ShowTips,
            17 => Setting::Checkpoints,
            _ => return None,
        })
    }

    /// Whether the row is a checkbox rather than a slider.
    pub fn is_checkbox(self) -> bool {
        matches!(
            self,
            Setting::Music
                | Setting::VerticalSync
                | Setting::ShowFps
                | Setting::WindowMode
                | Setting::ShowTips
                | Setting::Checkpoints
        )
    }
}

/// What a row shows when the Settings page opens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowState {
    /// The slider's position, 0 to 1. Checkboxes and a slider the original
    /// leaves at its record's value have none.
    pub fraction: Option<f32>,
    /// Whether a checkbox is checked.
    pub checked: bool,
    /// Whether the row takes input. Hardware anti-aliasing is disabled while
    /// super-sampling is off.
    pub enabled: bool,
    /// The slider's label, with the original's leading space.
    pub text: Option<&'static str>,
}

impl RowState {
    const fn slider(fraction: f32, text: Option<&'static str>) -> Self {
        Self {
            fraction: Some(fraction),
            checked: false,
            enabled: true,
            text,
        }
    }

    const fn check(checked: bool) -> Self {
        Self {
            fraction: None,
            checked,
            enabled: true,
            text: None,
        }
    }
}

/// The rows as the page opens, in row order.
///
/// Volume always shows the default's 2/3 position, not the live volume: the
/// original's opening handler divides the default by the maximum. Hardware
/// anti-aliasing is enabled only while super-sampling is on; when it is off,
/// the slider shows its level-0 position.
pub fn populate(options: &Options) -> [(Setting, RowState); 14] {
    let row = |setting: Setting| match setting {
        Setting::Volume => RowState::slider(Options::default().volume_fraction(), None),
        Setting::Music => RowState::check(options.music),
        Setting::Supersample => {
            RowState::slider(options.supersample_fraction(), options.supersample_text())
        }
        Setting::ShadingQuality => {
            RowState::slider(options.shading_fraction(), options.shading_text())
        }
        Setting::TextureDetail => RowState::slider(
            options.texture_detail_fraction(),
            options.texture_detail_text(),
        ),
        Setting::TextureQuality => RowState::slider(
            options.texture_quality_fraction(),
            options.texture_quality_text(),
        ),
        Setting::ShadowDetail => RowState::slider(
            options.shadow_detail_fraction(),
            options.shadow_detail_text(),
        ),
        Setting::ShadowQuality => RowState::slider(
            options.shadow_quality_fraction(),
            options.shadow_quality_text(),
        ),
        Setting::Multisample if options.supersample > 0 => RowState {
            fraction: None,
            checked: false,
            enabled: true,
            text: None,
        },
        Setting::Multisample => RowState {
            fraction: Some(options.multisample_fraction()),
            checked: false,
            enabled: false,
            text: options.multisample_text(),
        },
        Setting::VerticalSync => RowState::check(options.vertical_sync),
        Setting::ShowFps => RowState::check(options.show_fps),
        Setting::WindowMode => RowState::check(options.window_mode),
        Setting::ShowTips => RowState::check(options.show_tips),
        Setting::Checkpoints => RowState::check(options.checkpoints),
    };
    [
        Setting::Volume,
        Setting::Music,
        Setting::Supersample,
        Setting::ShadingQuality,
        Setting::TextureDetail,
        Setting::TextureQuality,
        Setting::ShadowDetail,
        Setting::ShadowQuality,
        Setting::Multisample,
        Setting::VerticalSync,
        Setting::ShowFps,
        Setting::WindowMode,
        Setting::ShowTips,
        Setting::Checkpoints,
    ]
    .map(|setting| (setting, row(setting)))
}

/// Where a control's change goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// The running game changes at once (volume, aim speed, music, vertical
    /// sync, the frame rate, the tips, checkpoints).
    Live,
    /// Only the screen's draft changes; Apply pushes it.
    Draft,
    /// The change is remembered but not written to the settings file (the
    /// Music checkbox mutes the session; the original writes the startup
    /// value).
    SessionOnly,
    /// The row is disabled and ignores input.
    Ignored,
}

/// A slider step: writes its value into `options` and says where it goes.
///
/// The steps are the original's: volume × 1.5; super-sampling `round(3·f)`,
/// one more above 1; shading `round(3·f) + 3`; texture detail `round(f) + 2`;
/// texture quality `round(3·f) + 2`; shadow detail `round(f) + 4`; shadow
/// quality `round(3·f) + 1`; hardware anti-aliasing `1 << round(3·f)`.
/// Rounding is to even, as the processor's conversion does.
///
/// The super-sampling slider also enables or disables the hardware
/// anti-aliasing row; it does not change that row's value. The volume and
/// aim-speed sliders change the live options; the renderer's quality sliders
/// change the draft.
pub fn on_slider(options: &mut Options, setting: Setting, fraction: f32) -> Effect {
    match setting {
        Setting::Volume => {
            options.volume = hw_config::volume_from_fraction(fraction);
            Effect::Live
        }
        Setting::Supersample => {
            options.supersample = hw_config::supersample_from_fraction(fraction);
            Effect::Draft
        }
        Setting::ShadingQuality => {
            options.shading_quality = hw_config::shading_from_fraction(fraction);
            Effect::Draft
        }
        Setting::TextureDetail => {
            options.texture_detail = hw_config::texture_detail_from_fraction(fraction);
            Effect::Draft
        }
        Setting::TextureQuality => {
            options.texture_quality = hw_config::texture_quality_from_fraction(fraction);
            Effect::Draft
        }
        Setting::ShadowDetail => {
            options.shadow_detail = hw_config::shadow_detail_from_fraction(fraction);
            Effect::Draft
        }
        Setting::ShadowQuality => {
            options.shadow_quality = hw_config::shadow_quality_from_fraction(fraction);
            Effect::Draft
        }
        Setting::Multisample => {
            if options.supersample <= 0 {
                return Effect::Ignored;
            }
            options.multisample = hw_config::multisample_from_fraction(fraction);
            Effect::Draft
        }
        Setting::Music
        | Setting::VerticalSync
        | Setting::ShowFps
        | Setting::WindowMode
        | Setting::ShowTips
        | Setting::Checkpoints => Effect::Ignored,
    }
}

/// A checkbox click: writes it into `options` and says where it goes.
///
/// The Music checkbox only mutes the running session; the original's setting
/// writes the startup music value and reopens unchecked from it.
pub fn on_toggle(options: &mut Options, setting: Setting, checked: bool) -> Effect {
    match setting {
        Setting::Music => Effect::SessionOnly,
        Setting::VerticalSync => {
            options.vertical_sync = checked;
            Effect::Live
        }
        Setting::ShowFps => {
            options.show_fps = checked;
            Effect::Live
        }
        Setting::WindowMode => {
            options.window_mode = checked;
            Effect::Draft
        }
        Setting::ShowTips => {
            options.show_tips = checked;
            Effect::Live
        }
        Setting::Checkpoints => {
            options.checkpoints = checked;
            Effect::Live
        }
        _ => Effect::Ignored,
    }
}

/// The aim-speed row's number (`round(f × 99) + 1`) and the fraction it
/// writes: the slider changes the live aim speed.
pub fn set_aim_speed(options: &mut Options, fraction: f32) -> i32 {
    options.aim_speed = fraction;
    options.aim_speed_text()
}

/// The text of a scale the controller-backed dropdown would show, for the
/// pages that reuse the renderer's labels.
pub fn scale_text(scale: Scale, value: i32) -> Option<&'static str> {
    scale.text(value)
}

/// A Settings or Controls form action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormAction {
    /// The SETTINGS tab (page 0).
    SettingsTab,
    /// The CONTROLS tab (page 1).
    ControlsTab,
    /// The Defaults button: run the benchmark and take its quality.
    Defaults,
    /// Apply: store the settings and close.
    Apply,
    /// Cancel: push the options the screen opened with and close.
    Cancel,
}

/// The action of the tab/button at `index` in [`TABS`]. The two rule records
/// and the Keyboard/Controller tabs are not form actions.
pub fn form_action(index: usize) -> Option<FormAction> {
    Some(match index {
        0 => FormAction::SettingsTab,
        1 => FormAction::ControlsTab,
        6 => FormAction::Defaults,
        7 => FormAction::Apply,
        8 => FormAction::Cancel,
        _ => return None,
    })
}

/// The page an action shows, for the two tabs.
pub fn action_page(action: FormAction) -> Option<u8> {
    match action {
        FormAction::SettingsTab => Some(0),
        FormAction::ControlsTab => Some(1),
        _ => None,
    }
}

#[cfg(test)]
mod screen_tests {
    use super::*;

    #[test]
    fn the_row_ids() {
        assert_eq!(Setting::Volume.id(), 3);
        assert_eq!(Setting::ShadowQuality.id(), 11);
        assert_eq!(Setting::Checkpoints.id(), 17);
        assert_eq!(Setting::from_id(10), None);
        assert_eq!(Setting::from_id(11), Some(Setting::ShadowQuality));
        assert!(Setting::Music.is_checkbox());
        assert!(!Setting::Volume.is_checkbox());
    }

    #[test]
    fn the_page_opens_on_the_original_values() {
        let live = Options {
            supersample: 3,
            shading_quality: 5,
            texture_detail: 2,
            texture_quality: 5,
            shadow_detail: 5,
            shadow_quality: 2,
            multisample: 8,
            vertical_sync: false,
            show_fps: true,
            window_mode: false,
            show_tips: false,
            checkpoints: false,
            ..Options::default()
        };
        let draft = Options::opened_from(&live);
        let rows = populate(&draft);
        assert_eq!(
            rows[0],
            (Setting::Volume, RowState::slider(2.0 / 3.0, None))
        );
        assert_eq!(rows[1], (Setting::Music, RowState::check(true)));
        assert_eq!(rows[2].1.text, Some(" Medium"));
        assert_eq!(rows[3].1.text, Some(" High"));
        assert_eq!(rows[4].1.text, Some(" Low"));
        assert_eq!(rows[5].1.text, Some(" Very high"));
        assert_eq!(rows[6].1.text, Some(" High"));
        assert_eq!(rows[7].1.text, Some(" Low"));
        // Super-sampling is on, so the anti-aliasing row is enabled and keeps
        // its record's position: it has no opening value.
        assert_eq!(
            rows[8],
            (
                Setting::Multisample,
                RowState {
                    fraction: None,
                    checked: false,
                    enabled: true,
                    text: None
                }
            )
        );
        assert_eq!(rows[9], (Setting::VerticalSync, RowState::check(true)));
        assert_eq!(rows[10], (Setting::ShowFps, RowState::check(true)));
        assert_eq!(rows[11], (Setting::WindowMode, RowState::check(false)));
        assert_eq!(rows[12], (Setting::ShowTips, RowState::check(true)));
        assert_eq!(rows[13], (Setting::Checkpoints, RowState::check(true)));
    }

    #[test]
    fn the_anti_aliasing_row_is_disabled_without_super_sampling() {
        let options = Options::default();
        let rows = populate(&options);
        assert!(!rows[8].1.enabled);
        assert_eq!(rows[8].1.fraction, Some(0.0));
        assert_eq!(rows[8].1.text, Some(" Off"));
    }

    #[test]
    fn a_slider_writes_its_step_and_says_where_it_goes() {
        let mut o = Options::default();
        assert_eq!(on_slider(&mut o, Setting::Volume, 0.5), Effect::Live);
        assert_eq!(o.volume, 0.75);

        assert_eq!(
            on_slider(&mut o, Setting::Supersample, 2.0 / 3.0),
            Effect::Draft
        );
        assert_eq!(o.supersample, 3);
        assert_eq!(
            on_slider(&mut o, Setting::ShadingQuality, 1.0 / 3.0),
            Effect::Draft
        );
        assert_eq!(o.shading_quality, 4);
        assert_eq!(
            on_slider(&mut o, Setting::TextureDetail, 1.0),
            Effect::Draft
        );
        assert_eq!(o.texture_detail, 3);
        assert_eq!(
            on_slider(&mut o, Setting::TextureQuality, 2.0 / 3.0),
            Effect::Draft
        );
        assert_eq!(o.texture_quality, 4);
        assert_eq!(on_slider(&mut o, Setting::ShadowDetail, 1.0), Effect::Draft);
        assert_eq!(o.shadow_detail, 5);
        assert_eq!(
            on_slider(&mut o, Setting::ShadowQuality, 2.0 / 3.0),
            Effect::Draft
        );
        assert_eq!(o.shadow_quality, 3);

        // The anti-aliasing row ignores input while super-sampling is off.
        let mut off = Options::default();
        assert_eq!(
            on_slider(&mut off, Setting::Multisample, 1.0),
            Effect::Ignored
        );
        assert_eq!(off.multisample, 0);
        on_slider(&mut off, Setting::Supersample, 1.0 / 3.0);
        assert_eq!(
            on_slider(&mut off, Setting::Multisample, 1.0),
            Effect::Draft
        );
        assert_eq!(off.multisample, 8);
    }

    #[test]
    fn a_checkbox_writes_its_flag_and_says_where_it_goes() {
        let mut o = Options::default();
        assert_eq!(
            on_toggle(&mut o, Setting::VerticalSync, false),
            Effect::Live
        );
        assert!(!o.vertical_sync);
        assert_eq!(on_toggle(&mut o, Setting::ShowFps, true), Effect::Live);
        assert!(o.show_fps);
        assert_eq!(on_toggle(&mut o, Setting::WindowMode, false), Effect::Draft);
        assert!(!o.window_mode);
        assert_eq!(on_toggle(&mut o, Setting::ShowTips, false), Effect::Live);
        assert!(!o.show_tips);
        assert_eq!(on_toggle(&mut o, Setting::Checkpoints, false), Effect::Live);
        assert!(!o.checkpoints);
        // Music only mutes the session: the option keeps the startup value.
        assert_eq!(
            on_toggle(&mut o, Setting::Music, false),
            Effect::SessionOnly
        );
        assert!(o.music);
        assert_eq!(on_toggle(&mut o, Setting::Volume, true), Effect::Ignored);
    }

    #[test]
    fn the_aim_speed_row() {
        let mut o = Options::default();
        assert_eq!(set_aim_speed(&mut o, 0.29), 30);
        assert_eq!(o.aim_speed, 0.29);
        assert_eq!(set_aim_speed(&mut o, 0.0), 1);
        assert_eq!(set_aim_speed(&mut o, 1.0), 100);
    }

    #[test]
    fn the_controls_rows_use_their_slots_once() {
        let mut slots: Vec<usize> = BINDINGS.iter().filter_map(|b| b.slot).collect();
        slots.sort_unstable();
        slots.dedup();
        // The page has no row for the primary mouse button (slot 0) nor for
        // the quick-power slot; every other slot appears once.
        assert_eq!(slots, (1..20).chain(21..23).collect::<Vec<usize>>());
        assert_eq!(BINDINGS[0].slot, Some(1)); // Forward
        assert_eq!(BINDINGS[6].slot, Some(8)); // Orbit Camera
        assert_eq!(BINDINGS[21].slot, None); // Power Tree
    }

    #[test]
    fn the_form_actions() {
        assert_eq!(form_action(0), Some(FormAction::SettingsTab));
        assert_eq!(form_action(1), Some(FormAction::ControlsTab));
        assert_eq!(form_action(2), None);
        assert_eq!(form_action(3), None);
        assert_eq!(form_action(6), Some(FormAction::Defaults));
        assert_eq!(form_action(7), Some(FormAction::Apply));
        assert_eq!(form_action(8), Some(FormAction::Cancel));
        assert_eq!(form_action(9), None);
        assert_eq!(action_page(FormAction::SettingsTab), Some(0));
        assert_eq!(action_page(FormAction::ControlsTab), Some(1));
        assert_eq!(action_page(FormAction::Apply), None);
    }
}
