//! The gameplay-tip banner shown during loading and prelude screens.
//!
//! The banner is a translucent box near the top of the screen, as wide as
//! its text plus two units and four units tall, drawn with 160 alpha, with
//! the text one unit in and four units down. It takes no input. Which tip
//! it shows (the least-shown rotating tip, the control hints, the
//! `ShowGameTips` option) is the prelude's: see `exa_game::prelude::Tips`.
//!
//! Evidence: the tip banner's constructor, draw and no-input slot, and the
//! prelude's tip chooser (`docs/UI.md`, loading).

/// The banner's default text, until the prelude sets one.
pub const FIRST_TIP: &str =
    "Augmenting strikes by swinging and moving your body can greatly increase damage.";
/// The banner's box alpha.
pub const BOX_ALPHA: u8 = 160;
/// The banner's height, in units.
pub const BOX_HEIGHT_UNITS: i32 = 4;
/// Its distance from the top, in units.
pub const BOX_TOP_UNITS: i32 = 8;
/// Its width beyond the text, in units.
pub const BOX_PAD_UNITS: i32 = 2;

/// The tip banner's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTips {
    /// The text shown.
    pub text: String,
}

impl Default for GameTips {
    fn default() -> Self {
        Self {
            text: FIRST_TIP.to_string(),
        }
    }
}

impl GameTips {
    /// The banner's constructor: the default text.
    pub fn new() -> Self {
        Self::default()
    }

    /// The draw, in window units: the box and the text's origin. The box's
    /// left edge is the difference of the window's width and the box's
    /// shifted as an unsigned 64-bit value, so a box wider than the window
    /// lands far to the right.
    pub fn draw(&self, window_width: i32, unit: i32, text_width: i32) -> TipBanner {
        let width = text_width.wrapping_add(unit.wrapping_mul(BOX_PAD_UNITS));
        let x = ((i64::from(window_width) - i64::from(width)) as u64 >> 1) as i32;
        let y = unit.wrapping_mul(BOX_TOP_UNITS);
        TipBanner {
            box_at: [x, y],
            box_size: [width, unit.wrapping_mul(BOX_HEIGHT_UNITS)],
            alpha: BOX_ALPHA,
            text_at: [x.wrapping_add(unit), y.wrapping_add(unit.wrapping_mul(4))],
        }
    }

    /// The banner does not take input.
    pub fn input(&self) -> u32 {
        0
    }
}

/// The banner's quads and text origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TipBanner {
    pub box_at: [i32; 2],
    pub box_size: [i32; 2],
    pub alpha: u8,
    pub text_at: [i32; 2],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_starts_with_the_first_tip() {
        let t = GameTips::new();
        assert_eq!(t.text, FIRST_TIP);
        assert_eq!(t.input(), 0);
    }

    #[test]
    fn the_draw_centres_the_box() {
        let t = GameTips::new();
        let b = t.draw(640, 4, 100);
        assert_eq!(b.box_size, [108, 16]);
        assert_eq!(b.box_at, [266, 32]);
        assert_eq!(b.text_at, [270, 48]);
        assert_eq!(b.alpha, 160);
    }

    #[test]
    fn a_box_wider_than_the_window_shifts_as_unsigned() {
        let b = GameTips::new().draw(100, 4, 200);
        // (100 − 208) as an unsigned 64-bit value, halved, keeping 32 bits.
        assert_eq!(b.box_at[0], (((100i64 - 208) as u64) >> 1) as i32);
        assert!(b.box_at[0] > 1_000_000);
    }
}
