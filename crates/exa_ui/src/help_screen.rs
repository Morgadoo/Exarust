//! The arena's help screens: a one- or two-pane tip window with Continue
//! and "Don't show again".
//!
//! The "don't show" answer goes into the same settings mask as the tip
//! popups' ([`crate::help_system::TipStore`]) and is OR-ed in only when a
//! control's `0x60` state bits are set; the store is then written. The
//! window's height follows its text: 23 units plus the measured text
//! height. The two-pane variant steps its page with the page keys, clamped
//! to its pages, and scrolls its text box by one scroll step with the
//! other two.
//!
//! Evidence: the help screen's settings save, resize, close, destroy and
//! right-click handlers and the two-pane key handler (`docs/UI.md`, arena
//! tutorial pages).

use crate::help_system::TipStore;

/// The height a help window adds to its measured text, in units.
pub const HELP_PAD_UNITS: i32 = 23;
/// The state bits that make the "don't show again" answer count.
pub const DONT_SHOW_STATE: u32 = 0x60;
/// The mode the arena is returned to when a help screen is destroyed while
/// the arena is up and not shutting down.
pub const ARENA_HELP_CLOSED_MODE: i32 = -1;

/// Remembers a page's dismissal when its control asked for it and returns
/// whether the store is to be written.
pub fn remember_dismissal(store: &mut TipStore, control_state: u32, page_bit: u32) -> bool {
    if control_state & DONT_SHOW_STATE != 0 {
        store.mark(page_bit);
        true
    } else {
        false
    }
}

/// The width the text is measured at: the window's width less twice the
/// text box's inset, the window's border and two units.
pub fn help_text_width(window_width: i32, text_inset: i32, border: i32, unit: i32) -> i32 {
    let inset = (i64::from(text_inset) + i64::from(border) + i64::from(unit.wrapping_shl(1))) << 1;
    (i64::from(window_width) - inset) as i32
}

/// The window's height for a measured text height.
pub fn help_height(text_height: i32, unit: i32) -> i32 {
    unit.wrapping_mul(HELP_PAD_UNITS).wrapping_add(text_height)
}

/// Whether a resize changes the window: only a window that has its text
/// box, and only when its height differs from the padded text height.
pub fn help_resizes(has_text: bool, window_height: i32, text_height: i32, unit: i32) -> bool {
    has_text && help_height(text_height, unit) != window_height
}

/// The mode a destroyed help screen sets, when the arena is up and its
/// closing flag is clear.
pub fn help_destroy_mode(arena_up: bool, arena_closing: bool) -> Option<i32> {
    (arena_up && !arena_closing).then_some(ARENA_HELP_CLOSED_MODE)
}

/// A help window's click answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelpClick {
    /// The window closes.
    pub close: bool,
    /// The handler runs the base handler and returns its answer.
    pub base: bool,
    /// The answer when it is not the base's.
    pub result: i32,
}

/// The click handler. A mask with bit 2 clear goes to the base handler; with
/// it set, a release carrying `0x2000` closes the window (answer 1) and any
/// other is not handled (answer 0).
pub fn help_click(mask: u16) -> HelpClick {
    if mask & 2 == 0 {
        HelpClick {
            close: false,
            base: true,
            result: 0,
        }
    } else if mask & 0x2000 == 0x2000 {
        HelpClick {
            close: true,
            base: false,
            result: 1,
        }
    } else {
        HelpClick {
            close: false,
            base: false,
            result: 0,
        }
    }
}

/// The two-pane help screen's second pane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoPane {
    /// The page shown.
    pub page: i32,
    /// The last page.
    pub last_page: i32,
    /// The text box's scroll step (a fraction of its content).
    pub scroll_step: f32,
}

/// One two-pane key event's effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoPaneKey {
    pub page: i32,
    /// The page changed, so its callback runs.
    pub changed: bool,
    /// The text box scrolls by this fraction (its target, not at once).
    pub scroll: Option<f32>,
    /// The event asked to close the window.
    pub close: bool,
    /// The handler's return value: the new page for a page event, the
    /// incoming mask otherwise.
    pub result: u32,
}

/// The two-pane key handler. With bit 1 set, `0x4001` steps the page back
/// and `0x8001` forward, clamped to one through the last page (a last page
/// below 1 wins over the lower clamp). With it clear, `0x4002` scrolls the
/// text up one step, `0x8002` down one, and a plain `0x200` closes.
pub fn two_pane_key(mask: u32, pane: TwoPane) -> TwoPaneKey {
    let mut out = TwoPaneKey {
        page: pane.page,
        changed: false,
        scroll: None,
        close: false,
        result: mask,
    };
    if mask & 1 == 0 {
        if mask & 0xf002 != 0 {
            if mask & 0xf002 == 0x4002 {
                out.scroll = Some(-pane.scroll_step);
            } else if mask & 0xf002 == 0x8002 {
                out.scroll = Some(pane.scroll_step);
            }
        } else if mask == 0x200 {
            out.close = true;
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
    out.result = page as u32;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dont_show_answer_is_stored_only_with_the_state_bits() {
        let mut store = TipStore::default();
        assert!(!remember_dismissal(&mut store, 0x10, 4));
        assert_eq!(store.shown, 0);
        assert!(remember_dismissal(&mut store, 0x60, 4));
        assert!(!store.should_show(4));
        assert!(store.should_show(8));
        assert!(remember_dismissal(&mut store, 0x20, 8));
        assert_eq!(store.shown, 12);
    }

    #[test]
    fn the_help_window_follows_its_text() {
        // 23 units of four plus the text.
        assert_eq!(help_height(100, 4), 192);
        assert!(help_resizes(true, 100, 100, 4));
        assert!(!help_resizes(true, 192, 100, 4));
        assert!(!help_resizes(false, 100, 100, 4));
        // 400 wide, inset 6, border 2, two units of four: 400 - 2 * 16.
        assert_eq!(help_text_width(400, 6, 2, 4), 368);
    }

    #[test]
    fn a_click_closes_on_a_release_with_the_bit() {
        assert!(help_click(0x2002).close);
        assert_eq!(help_click(0x2002).result, 1);
        let other = help_click(0x0002);
        assert!(!other.close && !other.base && other.result == 0);
        assert!(help_click(0x2000).base);
    }

    #[test]
    fn a_destroyed_screen_returns_the_arena_only_while_it_is_up() {
        assert_eq!(help_destroy_mode(true, false), Some(-1));
        assert_eq!(help_destroy_mode(true, true), None);
        assert_eq!(help_destroy_mode(false, false), None);
    }

    #[test]
    fn the_two_pane_keys_scroll_and_page() {
        let pane = TwoPane {
            page: 2,
            last_page: 4,
            scroll_step: 0.25,
        };
        assert_eq!(two_pane_key(0x4002, pane).scroll, Some(-0.25));
        assert_eq!(two_pane_key(0x8002, pane).scroll, Some(0.25));
        assert_eq!(two_pane_key(0x4002, pane).result, 0x4002);
        assert!(two_pane_key(0x200, pane).close);
        assert_eq!(two_pane_key(0x2, pane).scroll, None);
        let r = two_pane_key(0x4001, pane);
        assert_eq!((r.page, r.changed, r.result), (1, true, 1));
        let r = two_pane_key(0x4001, TwoPane { page: 1, ..pane });
        assert_eq!((r.page, r.changed), (1, false));
        let r = two_pane_key(0x8001, pane);
        assert_eq!((r.page, r.changed), (3, true));
        let r = two_pane_key(0x8001, TwoPane { page: 4, ..pane });
        assert_eq!((r.page, r.changed), (4, false));
        // A last page below the first wins.
        let r = two_pane_key(
            0x8001,
            TwoPane {
                page: 0,
                last_page: 0,
                ..pane
            },
        );
        assert_eq!(r.page, 0);
    }
}
