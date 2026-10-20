//! The Settings and Controls pages' content and visible layout, from the
//! retail 1600×1080 screenshots: two tabs, the settings rows (info, slider,
//! choices and toggles), the control bindings and the footer buttons.
//!
//! The row content is exact; the layout is the screenshot's geometry scaled
//! linearly with the form, because the binary builds these rows from embedded
//! control templates whose records are not decoded yet.

/// A settings tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Settings,
    Controls,
}

/// A rectangular place in the form, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A choice's options, low to high.
pub const CHOICES: [&str; 5] = ["Off", "Low", "Medium", "High", "Very high"];

/// A settings row's control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingKind {
    /// A read-only value, e.g. the device or its memory.
    Info,
    /// A 0..1 slider.
    Slider(f32),
    /// An index into [`CHOICES`].
    Choice(u8),
    /// A checkbox.
    Toggle(bool),
}

/// One settings row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettingItem {
    pub label: &'static str,
    pub kind: SettingKind,
    /// The content row (0..13).
    pub row: u8,
    /// The column: 0 left, 1 right.
    pub column: u8,
}

/// The Settings page's rows, in display order.
pub const SETTINGS: [SettingItem; 16] = [
    SettingItem {
        label: "Video Device:",
        kind: SettingKind::Info,
        row: 0,
        column: 0,
    },
    SettingItem {
        label: "Video Memory:",
        kind: SettingKind::Info,
        row: 1,
        column: 0,
    },
    SettingItem {
        label: "Audio volume:",
        kind: SettingKind::Slider(0.5),
        row: 2,
        column: 0,
    },
    SettingItem {
        label: "Music",
        kind: SettingKind::Toggle(true),
        row: 2,
        column: 1,
    },
    SettingItem {
        label: "Super sampling:",
        kind: SettingKind::Choice(3),
        row: 3,
        column: 0,
    },
    SettingItem {
        label: "Shading quality:",
        kind: SettingKind::Choice(3),
        row: 4,
        column: 0,
    },
    SettingItem {
        label: "Texture detail:",
        kind: SettingKind::Choice(3),
        row: 5,
        column: 0,
    },
    SettingItem {
        label: "Texture quality:",
        kind: SettingKind::Choice(4),
        row: 6,
        column: 0,
    },
    SettingItem {
        label: "Shadow detail:",
        kind: SettingKind::Choice(3),
        row: 7,
        column: 0,
    },
    SettingItem {
        label: "Shadow quality:",
        kind: SettingKind::Choice(3),
        row: 8,
        column: 0,
    },
    SettingItem {
        label: "HW antialiasing:",
        kind: SettingKind::Choice(0),
        row: 9,
        column: 0,
    },
    SettingItem {
        label: "Vertical sync",
        kind: SettingKind::Toggle(true),
        row: 10,
        column: 0,
    },
    SettingItem {
        label: "Show framerate",
        kind: SettingKind::Toggle(false),
        row: 10,
        column: 1,
    },
    SettingItem {
        label: "Window mode",
        kind: SettingKind::Toggle(false),
        row: 11,
        column: 0,
    },
    SettingItem {
        label: "Show gameplay tips",
        kind: SettingKind::Toggle(true),
        row: 11,
        column: 1,
    },
    SettingItem {
        label: "Early Checkpoints",
        kind: SettingKind::Toggle(true),
        row: 12,
        column: 0,
    },
];

/// One control binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlItem {
    pub label: &'static str,
    pub key: &'static str,
}

/// The Controls page's bindings, in display order.
pub const CONTROLS: [ControlItem; 22] = [
    ControlItem {
        label: "Forward",
        key: "W",
    },
    ControlItem {
        label: "Back",
        key: "S",
    },
    ControlItem {
        label: "Left",
        key: "A",
    },
    ControlItem {
        label: "Right",
        key: "D",
    },
    ControlItem {
        label: "Run/Sprint",
        key: "Shift",
    },
    ControlItem {
        label: "Crouch",
        key: "X",
    },
    ControlItem {
        label: "Orbit Camera",
        key: "Mouse 3",
    },
    ControlItem {
        label: "Cursor Look",
        key: "V",
    },
    ControlItem {
        label: "Toggle Combat",
        key: "Tab",
    },
    ControlItem {
        label: "Alternate Attack",
        key: "Alt",
    },
    ControlItem {
        label: "Switch Equipped",
        key: "R",
    },
    ControlItem {
        label: "Flip weapon",
        key: "F",
    },
    ControlItem {
        label: "Interact",
        key: "Control",
    },
    ControlItem {
        label: "Inventory",
        key: "I",
    },
    ControlItem {
        label: "Skills",
        key: "K",
    },
    ControlItem {
        label: "Journal",
        key: "J",
    },
    ControlItem {
        label: "Talk",
        key: "T",
    },
    ControlItem {
        label: "Use Power",
        key: "",
    },
    ControlItem {
        label: "Power Bar",
        key: "",
    },
    ControlItem {
        label: "Previous Power",
        key: "",
    },
    ControlItem {
        label: "Next Power",
        key: "",
    },
    ControlItem {
        label: "Power Tree",
        key: "",
    },
];

/// The layout derived from the 1600×1080 screenshot's 630×800 form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsLayout {
    pub form: Rect,
    pub tabs: [Rect; 2],
    pub content: Rect,
    pub settings_rows: [i32; 13],
    pub control_rows: [i32; 14],
    pub label_x: i32,
    pub control_x: i32,
    pub value_x: i32,
    pub control_label_x: i32,
    pub control_key_x: i32,
    pub buttons: [Rect; 3],
}

const SETTINGS_ROW_OFFSETS: [i32; 13] = [
    130, 171, 229, 285, 323, 361, 399, 437, 475, 529, 574, 616, 658,
];
const CONTROL_ROW_OFFSETS: [i32; 14] = [
    130, 164, 198, 231, 265, 298, 332, 366, 399, 433, 466, 500, 534, 568,
];

/// The layout for a form at `form`. The screenshot's 630×800 form is the
/// reference; other sizes scale linearly.
pub fn layout(form: Rect) -> SettingsLayout {
    let sx = |x: i32| form.x + x * form.w / 630;
    let sy = |y: i32| form.y + y * form.h / 800;
    let sw = |w: i32| w * form.w / 630;
    let sh = |h: i32| h * form.h / 800;
    let rect = |x: i32, y: i32, w: i32, h: i32| Rect {
        x: sx(x),
        y: sy(y),
        w: sw(w),
        h: sh(h),
    };
    let mut settings_rows = [0; 13];
    for (row, offset) in settings_rows.iter_mut().zip(SETTINGS_ROW_OFFSETS) {
        *row = sy(offset);
    }
    let mut control_rows = [0; 14];
    for (row, offset) in control_rows.iter_mut().zip(CONTROL_ROW_OFFSETS) {
        *row = sy(offset);
    }
    SettingsLayout {
        form,
        tabs: [rect(20, 46, 280, 34), rect(308, 46, 280, 34)],
        content: rect(4, 92, 622, 600),
        settings_rows,
        control_rows,
        label_x: sx(40),
        control_x: sx(235),
        value_x: sx(460),
        control_label_x: sx(130),
        control_key_x: sx(420),
        buttons: [
            rect(60, 723, 140, 28),
            rect(225, 723, 145, 28),
            rect(390, 723, 150, 28),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_rows() {
        assert_eq!(SETTINGS.len(), 16);
        assert_eq!(SETTINGS[0].label, "Video Device:");
        assert_eq!(SETTINGS[2].kind, SettingKind::Slider(0.5));
        assert_eq!(
            (SETTINGS[3].label, SETTINGS[3].kind),
            ("Music", SettingKind::Toggle(true))
        );
        assert_eq!(SETTINGS[4].kind, SettingKind::Choice(3));
        assert_eq!(SETTINGS[7].kind, SettingKind::Choice(4));
        assert_eq!(SETTINGS[10].kind, SettingKind::Choice(0));
        assert_eq!(SETTINGS[11].kind, SettingKind::Toggle(true));
        assert_eq!(SETTINGS[14].row, 11);
        assert_eq!(SETTINGS[14].column, 1);
        assert_eq!(CHOICES[4], "Very high");
    }

    #[test]
    fn the_control_rows() {
        assert_eq!(CONTROLS.len(), 22);
        assert_eq!((CONTROLS[0].label, CONTROLS[0].key), ("Forward", "W"));
        assert_eq!(
            (CONTROLS[6].label, CONTROLS[6].key),
            ("Orbit Camera", "Mouse 3")
        );
        assert_eq!(
            (CONTROLS[12].label, CONTROLS[12].key),
            ("Interact", "Control")
        );
    }

    #[test]
    fn the_retail_layout() {
        let form = Rect {
            x: 500,
            y: 130,
            w: 630,
            h: 800,
        };
        let l = layout(form);
        assert_eq!(
            l.tabs[0],
            Rect {
                x: 520,
                y: 176,
                w: 280,
                h: 34
            }
        );
        assert_eq!(
            l.tabs[1],
            Rect {
                x: 808,
                y: 176,
                w: 280,
                h: 34
            }
        );
        assert_eq!(l.settings_rows[0], 260);
        assert_eq!(l.settings_rows[12], 788);
        assert_eq!(l.control_rows[0], 260);
        assert_eq!(l.control_rows[13], 698);
        assert_eq!(l.label_x, 540);
        assert_eq!(l.control_x, 735);
        assert_eq!(l.value_x, 960);
        assert_eq!(l.control_label_x, 630);
        assert_eq!(l.control_key_x, 920);
        assert_eq!(
            l.buttons[0],
            Rect {
                x: 560,
                y: 853,
                w: 140,
                h: 28
            }
        );
        assert_eq!(l.buttons[2].x, 890);
    }

    #[test]
    fn the_layout_scales_with_the_form() {
        let l = layout(Rect {
            x: 0,
            y: 0,
            w: 315,
            h: 400,
        });
        assert_eq!(
            l.tabs[0],
            Rect {
                x: 10,
                y: 23,
                w: 140,
                h: 17
            }
        );
        assert_eq!(l.settings_rows[0], 65);
        assert_eq!(l.buttons[1].y, 361);
    }
}
