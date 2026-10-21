//! The colour window and the game's small skinned dialogs.
//!
//! The colour window is 402 by 282 with sixteen controls and six colour
//! strips: black, red, green, blue, red and white, plus the sixth slider's
//! own six-entry colour table. The confirmation dialogs are all 392 by 112
//! with a skin, a title and two buttons; their click handler closes with
//! result `0x40` when nothing is selected and otherwise runs the base
//! handler. The arena's status panel draws the company emblem across its
//! middle, four units down.
//!
//! Evidence: the colour picker's constructor and the four dialogs'
//! constructors and click handlers (`docs/UI.md`).

/// A colour strip: whether it is live and its RGBA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourRow {
    pub live: i32,
    pub rgba: [u8; 4],
}

/// The colour window's size, title and number of layout controls.
pub const COLOUR_WIDTH: i32 = 402;
pub const COLOUR_HEIGHT: i32 = 282;
pub const COLOUR_TITLE: &str = "Colour";
pub const COLOUR_CONTROLS: usize = 16;
/// The colour window's form flags.
pub const COLOUR_FLAGS: u32 = 0x1000;
pub const COLOUR_EXTRA_FLAGS: u32 = 0x800;

/// The six colour strips the constructor installs.
pub const COLOUR_ROWS: [ColourRow; 6] = [
    ColourRow {
        live: 1,
        rgba: [0, 0, 0, 255],
    },
    ColourRow {
        live: 1,
        rgba: [255, 0, 0, 255],
    },
    ColourRow {
        live: 1,
        rgba: [0, 255, 0, 255],
    },
    ColourRow {
        live: 1,
        rgba: [0, 0, 255, 255],
    },
    ColourRow {
        live: 1,
        rgba: [255, 0, 0, 255],
    },
    ColourRow {
        live: 1,
        rgba: [255, 255, 255, 255],
    },
];

/// The sixth slider's shared colour table (the bytes of the original's
/// table; the first six entries are the ones the dump covers completely).
pub const COLOUR_TABLE: [[u8; 4]; 6] = [
    [255, 0, 0, 255],
    [255, 255, 255, 0],
    [255, 0, 255, 0],
    [255, 0, 255, 255],
    [255, 0, 0, 255],
    [255, 255, 255, 0],
];

/// The colour window's constructor data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourPicker {
    pub width: i32,
    pub height: i32,
    pub digit: i32,
    pub flags: u32,
    pub form_flags: u32,
    /// The control the fourth strip's shared table feeds.
    pub shared_slot: usize,
}

/// The window's values: 402 by 282, digits 1, flags `0x1000` and `0x800`,
/// and the sixth control reads the shared table.
pub fn colour_picker() -> ColourPicker {
    ColourPicker {
        width: COLOUR_WIDTH,
        height: COLOUR_HEIGHT,
        digit: 1,
        flags: COLOUR_FLAGS,
        form_flags: COLOUR_EXTRA_FLAGS,
        shared_slot: 3,
    }
}

/// A skinned confirmation dialog's fixed data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dialog {
    pub title: &'static str,
    pub width: i32,
    pub height: i32,
    pub layout_elements: usize,
}

/// The dialog size every confirmation uses.
pub const DIALOG_WIDTH: i32 = 0x188;
pub const DIALOG_HEIGHT: i32 = 0x70;
/// The dialogs' form flags.
pub const DIALOG_FLAGS: u32 = 0xb800;
pub const DIALOG_FORM_FLAGS: u32 = 0x1000;

/// The "Reset to portal" dialog.
pub const RESET_TO_PORTAL: Dialog = Dialog {
    title: "Reset to portal",
    width: DIALOG_WIDTH,
    height: DIALOG_HEIGHT,
    layout_elements: 2,
};
/// The character list's "Delete Character" dialog.
pub const DELETE_CHARACTER: Dialog = Dialog {
    title: "Delete Character",
    width: DIALOG_WIDTH,
    height: DIALOG_HEIGHT,
    layout_elements: 2,
};
/// The arena's "Dismiss" dialog.
pub const DISMISS: Dialog = Dialog {
    title: "Dismiss",
    width: DIALOG_WIDTH,
    height: DIALOG_HEIGHT,
    layout_elements: 2,
};
/// The arena's generic yes/no dialog.
pub const CONFIRM: Dialog = Dialog {
    title: "",
    width: DIALOG_WIDTH,
    height: DIALOG_HEIGHT,
    layout_elements: 2,
};

/// A dialog click's answer: the base handler's result, or the close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogClick {
    pub close: bool,
    pub result: i32,
}

/// The four confirmation dialogs' click rule.
pub fn dialog_click(selection: i32, base: impl FnOnce() -> i32) -> DialogClick {
    if selection < 1 {
        DialogClick {
            close: true,
            result: 0x40,
        }
    } else {
        DialogClick {
            close: false,
            result: base(),
        }
    }
}

/// The status panel's emblem place: the panel's width across, four units
/// down, and 48 pixels tall.
pub fn status_emblem_place(client_width: i32, unit: i32) -> (i32, i32, i32) {
    (client_width / 2, unit * 4, 0x30)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colour_window_has_its_six_strips() {
        let c = colour_picker();
        assert_eq!((c.width, c.height), (402, 282));
        assert_eq!((c.digit, c.flags, c.form_flags), (1, 0x1000, 0x800));
        assert_eq!(COLOUR_ROWS[0].rgba, [0, 0, 0, 255]);
        assert_eq!(COLOUR_ROWS[1].rgba, [255, 0, 0, 255]);
        assert_eq!(COLOUR_ROWS[2].rgba, [0, 255, 0, 255]);
        assert_eq!(COLOUR_ROWS[3].rgba, [0, 0, 255, 255]);
        assert_eq!(COLOUR_ROWS[5].rgba, [255, 255, 255, 255]);
        assert_eq!(COLOUR_TABLE[3], [255, 0, 255, 255]);
    }

    #[test]
    fn the_dialogs_share_their_size() {
        for d in [RESET_TO_PORTAL, DELETE_CHARACTER, DISMISS, CONFIRM] {
            assert_eq!((d.width, d.height), (0x188, 0x70));
            assert_eq!(d.layout_elements, 2);
        }
        assert_eq!(RESET_TO_PORTAL.title, "Reset to portal");
        assert_eq!(DELETE_CHARACTER.title, "Delete Character");
        assert_eq!(DISMISS.title, "Dismiss");
    }

    #[test]
    fn a_click_without_a_selection_closes() {
        let c = dialog_click(-1, || 5);
        assert_eq!(
            c,
            DialogClick {
                close: true,
                result: 0x40
            }
        );
        let c = dialog_click(0, || 5);
        assert!(c.close);
        let c = dialog_click(1, || 5);
        assert_eq!(
            c,
            DialogClick {
                close: false,
                result: 5
            }
        );
    }

    #[test]
    fn the_status_emblem_sits_across_the_panel() {
        assert_eq!(status_emblem_place(400, 4), (200, 16, 48));
    }
}
