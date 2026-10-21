//! The colour window and the game's small skinned dialogs.
//!
//! The colour window is 402 by 282 with sixteen layout elements, a centred
//! x, the upper layer and the title "Colour". Its constructor installs five
//! gradient records (live, black to a colour: red, green, blue, red and
//! white) and hands them, with the hue table, to the six slider controls.
//! The confirmation dialogs are all 392 by 112 with a skin, a title and two
//! layout elements; their click handler closes with result `0x40` when
//! nothing is selected and otherwise runs the base handler. The arena's
//! status panel draws the company emblem across its middle, 16 pixels
//! down.
//!
//! Evidence: the colour picker's constructor and the four dialogs'
//! constructors and click handlers, with the constructor's data read from
//! the retail image (`docs/UI.md`). The layout-element records themselves
//! are not ported.

/// A gradient record the colour window's constructor installs: whether it
/// is live and the RGBA of the two ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gradient {
    pub live: i32,
    pub from: [u8; 4],
    pub to: [u8; 4],
}

/// The colour window's size, title and number of layout elements.
pub const COLOUR_WIDTH: i32 = 402;
pub const COLOUR_HEIGHT: i32 = 282;
pub const COLOUR_TITLE: &str = "Colour";
pub const COLOUR_ELEMENTS: usize = 16;
/// The colour window's style bits: centred in x.
pub const COLOUR_STYLE: u32 = 0x800;
/// The colour window's attribute bits: the upper layer.
pub const COLOUR_ATTRIBUTES: u32 = 0x1000;
/// The colour window's update word after construction.
pub const COLOUR_UPDATE: u32 = 0x2000;

const BLACK: [u8; 4] = [0, 0, 0, 255];

/// The five gradients the constructor installs, in the order of the
/// controls that take them (the first, second, third, fifth and sixth
/// slider; the fourth takes [`HUE_TABLE`]).
pub const GRADIENTS: [Gradient; 5] = [
    Gradient {
        live: 1,
        from: BLACK,
        to: [255, 0, 0, 255],
    },
    Gradient {
        live: 1,
        from: BLACK,
        to: [0, 255, 0, 255],
    },
    Gradient {
        live: 1,
        from: BLACK,
        to: [0, 0, 255, 255],
    },
    Gradient {
        live: 1,
        from: BLACK,
        to: [255, 0, 0, 255],
    },
    Gradient {
        live: 1,
        from: BLACK,
        to: [255, 255, 255, 255],
    },
];

/// The fourth slider's colour table: a count of 6 and the stops red,
/// yellow, green, cyan, blue, magenta and red again (the seven entries
/// before the next record).
pub const HUE_COUNT: u32 = 6;
pub const HUE_TABLE: [[u8; 4]; 7] = [
    [255, 0, 0, 255],
    [255, 255, 0, 255],
    [0, 255, 0, 255],
    [0, 255, 255, 255],
    [0, 0, 255, 255],
    [255, 0, 255, 255],
    [255, 0, 0, 255],
];

/// The colour window's constructor data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourWindow {
    pub width: i32,
    pub height: i32,
    /// The border word (the client's left edge): 1.
    pub border: i32,
    pub style: u32,
    pub attributes: u32,
    /// The slider that reads the hue table.
    pub hue_slider: usize,
}

/// The window's values: 402 by 282, border 1, centred in x, the upper
/// layer, and the fourth slider reading the hue table.
pub fn colour_window() -> ColourWindow {
    ColourWindow {
        width: COLOUR_WIDTH,
        height: COLOUR_HEIGHT,
        border: 1,
        style: COLOUR_STYLE,
        attributes: COLOUR_ATTRIBUTES,
        hue_slider: 3,
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
/// The dialogs' style bits: centred in both axes, plus the 0x3000 pair.
pub const DIALOG_STYLE: u32 = 0xb800;
/// The dialogs' attribute bits: the upper layer.
pub const DIALOG_ATTRIBUTES: u32 = 0x1000;

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

/// The status panel's emblem place: the middle of its client width, 16
/// pixels (four units of four) down, and 48 pixels tall.
pub fn status_emblem_place(client_width: i32, unit: i32) -> (i32, i32, i32) {
    (client_width / 2, unit * 4, 0x30)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colour_window_has_its_gradients() {
        let c = colour_window();
        assert_eq!((c.width, c.height, c.border), (402, 282, 1));
        assert_eq!((c.style, c.attributes), (0x800, 0x1000));
        assert_eq!(GRADIENTS[0].from, [0, 0, 0, 255]);
        assert_eq!(GRADIENTS[1].to, [0, 255, 0, 255]);
        assert_eq!(GRADIENTS[2].to, [0, 0, 255, 255]);
        assert_eq!(GRADIENTS[4].to, [255, 255, 255, 255]);
        assert_eq!(HUE_TABLE[1], [255, 255, 0, 255]);
        assert_eq!(HUE_TABLE[0], HUE_TABLE[6]);
        assert_eq!(c.hue_slider, 3);
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
