//! The arena's help screens: a one- or two-pane tip window with Continue
//! and "Don't show again".
//!
//! The "don't show" bit is kept in the settings store and OR-ed in only
//! when a control's `0x60` state is set. The window's height follows its
//! text: 23 units plus the measured text height. The two-pane variant
//! scrolls its second pane with the page keys, clamps the page to its
//! contents, and can pin that pane on top.
//!
//! Evidence: the help screen's settings save, resize, close, destroy and
//! right-click handlers and the two-pane key handler (`docs/UI.md`, arena
//! tutorial pages).

/// The height a help window adds to its measured text, in pixels.
pub const HELP_PAD: i32 = 23;
/// The state bits that make the "don't show again" answer count.
pub const DONT_SHOW_STATE: u32 = 0x60;
/// The arena window mode set when the help screen closes.
pub const ARENA_HELP_CLOSED_MODE: i32 = 3;

/// The settings bytes remembering which help pages were dismissed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HelpDismissed {
    pub mask: u32,
}

impl HelpDismissed {
    /// ORs a page's bit in when its control asked to be remembered and
    /// returns whether the store was written.
    pub fn close(&mut self, control_state: u32, page_bit: u32) -> bool {
        if control_state & DONT_SHOW_STATE != 0 {
            self.mask |= page_bit;
            true
        } else {
            false
        }
    }

    /// Whether a page was dismissed.
    pub fn dismissed(&self, page_bit: u32) -> bool {
        self.mask & page_bit != 0
    }
}

/// The window's height for a measured text height.
pub fn help_height(text_height: i32) -> i32 {
    HELP_PAD.wrapping_add(text_height)
}

/// Whether a resize changes the window (its height differs from the
/// padded text height).
pub fn help_resizes(window_height: i32, text_height: i32) -> bool {
    help_height(text_height) != window_height
}

/// A right-click closes a help screen: a release carrying `0x2000`.
pub fn help_closes(mask: u16) -> bool {
    mask & 0x2002 == 0x2002
}

/// The two-pane help screen's second pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoPane {
    /// The pane is pinned on top (bit 31 of its state word).
    pub topmost: bool,
    /// The first line shown.
    pub page: i32,
    /// The last page.
    pub last_page: i32,
}

/// One two-pane key event's effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoPaneKey {
    pub topmost: bool,
    pub page: i32,
    /// The page changed, so its scroll callback runs.
    pub changed: bool,
    /// The event asked to close.
    pub close: bool,
}

/// The two-pane key handler. A press toggles the pane (`0x4002`), restores
/// it (`0x8002`), or steps the page (`0x4001` down, `0x8001` up) within
/// one through the last page. A plain `0x200` asks to close.
pub fn two_pane_key(mask: u32, pane: TwoPane) -> TwoPaneKey {
    let mut out = TwoPaneKey {
        topmost: pane.topmost,
        page: pane.page,
        changed: false,
        close: false,
    };
    if mask & 1 == 0 {
        if mask & 0xf002 == 0 {
            if mask == 0x200 {
                out.close = true;
            }
        } else if mask & 0xf002 == 0x4002 {
            out.topmost = !pane.topmost;
        } else if mask & 0xf002 == 0x8002 {
            out.topmost = false;
        }
        return out;
    }
    let mut page = pane.page;
    if mask & 0xf001 == 0x4001 {
        page = page.wrapping_sub(1);
    } else if mask & 0xf001 == 0x8001 {
        page = page.wrapping_add(1);
    }
    if page < 1 {
        page = 1;
    } else if pane.last_page < page {
        page = pane.last_page;
    }
    out.changed = page != pane.page;
    out.page = page;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dont_show_answer_is_stored_only_with_the_state_bits() {
        let mut d = HelpDismissed::default();
        assert!(!d.close(0x10, 4));
        assert_eq!(d.mask, 0);
        assert!(d.close(0x60, 4));
        assert_eq!(d.mask, 4);
        assert!(d.dismissed(4));
        assert!(!d.dismissed(8));
        assert!(d.close(0x20, 8));
        assert_eq!(d.mask, 12);
    }

    #[test]
    fn the_help_window_follows_its_text() {
        assert_eq!(help_height(100), 123);
        assert!(help_resizes(100, 100));
        assert!(!help_resizes(123, 100));
        assert!(help_closes(0x2002));
        assert!(!help_closes(0x2000));
        assert!(!help_closes(0x0001));
    }

    #[test]
    fn the_two_pane_keys_toggle_and_page() {
        let pane = TwoPane {
            topmost: false,
            page: 2,
            last_page: 4,
        };
        let r = two_pane_key(0x4002, pane);
        assert!(r.topmost);
        let r = two_pane_key(
            0x8002,
            TwoPane {
                topmost: true,
                ..pane
            },
        );
        assert!(!r.topmost);
        let r = two_pane_key(0x200, pane);
        assert!(r.close);
        let r = two_pane_key(0x4001, pane);
        assert_eq!((r.page, r.changed), (1, true));
        let r = two_pane_key(0x4001, TwoPane { page: 1, ..pane });
        assert_eq!((r.page, r.changed), (1, false));
        let r = two_pane_key(0x8001, pane);
        assert_eq!((r.page, r.changed), (3, true));
        let r = two_pane_key(0x8001, TwoPane { page: 4, ..pane });
        assert_eq!((r.page, r.changed), (4, false));
    }
}
