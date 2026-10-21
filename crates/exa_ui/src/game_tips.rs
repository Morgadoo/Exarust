//! The gameplay-tip banner shown during loading and prelude screens.
//!
//! The banner is a translucent box at the top of the screen, sized to its
//! text plus two units and 160 alpha, with the tip one unit in and four
//! units down. It takes no input. The tip is chosen among the eighteen
//! tips with the fewest shows (a random one of those on a tie), or a
//! contextual one from a second table; the `ShowGameTips` option turns the
//! banner off.
//!
//! Evidence: the tip banner's constructor, its draw and its no-input slot
//! and the tip chooser in the prelude unit (`docs/UI.md`, loading).

/// The number of gameplay tips.
pub const TIPS: usize = 18;
/// The first tip, the banner's default text.
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
    /// How often each tip has been shown.
    pub shown: [u32; TIPS],
    pub show_tips: bool,
}

impl Default for GameTips {
    fn default() -> Self {
        Self {
            text: FIRST_TIP.to_string(),
            shown: [0; TIPS],
            show_tips: true,
        }
    }
}

impl GameTips {
    /// The banner's constructor: the first tip's text.
    pub fn new() -> Self {
        Self::default()
    }

    /// The tip chooser. When tips are off or there are none, nothing is
    /// chosen. `contextual` selects the second table; `roll` picks among
    /// the least-shown tips (the original draws a random number).
    pub fn choose(
        &mut self,
        contextual: Option<&[&'static str]>,
        contextual_index: Option<usize>,
        roll: u32,
    ) -> Option<String> {
        if !self.show_tips {
            return None;
        }
        if let Some(table) = contextual {
            let index = contextual_index.unwrap_or(0);
            let text = table.get(index)?;
            self.text = (*text).to_string();
            return Some(self.text.clone());
        }
        let least = *self.shown.iter().min()?;
        let candidates: Vec<usize> = (0..TIPS).filter(|&i| self.shown[i] == least).collect();
        if candidates.is_empty() {
            return None;
        }
        let index = candidates[roll as usize % candidates.len()];
        self.shown[index] = self.shown[index].wrapping_add(1);
        self.text = if index == 0 {
            FIRST_TIP.to_string()
        } else {
            format!("{index}")
        };
        Some(self.text.clone())
    }

    /// The draw, in window units: the box and the text's origin.
    pub fn draw(&self, window_width: i32, unit: i32, text_width: i32) -> TipBanner {
        let width = text_width.wrapping_add(unit.wrapping_mul(BOX_PAD_UNITS));
        let x = ((i64::from(window_width) - i64::from(width)) >> 1) as i32;
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
        assert!(t.show_tips);
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
    fn the_tip_chooser_prefers_the_least_shown() {
        let mut t = GameTips::new();
        t.shown[3] = 5;
        t.shown[7] = 2;
        // The least shown is index 0…2, 4…6, 8…17; a roll picks within.
        let before = t.shown;
        let choice = t.choose(None, None, 0).unwrap();
        assert_eq!(choice, FIRST_TIP);
        assert_eq!(t.shown[0], 1);
        assert_eq!(t.shown[3], before[3]);
        t.show_tips = false;
        assert!(t.choose(None, None, 0).is_none());
        t.show_tips = true;
        let context = ["a", "b"];
        assert_eq!(t.choose(Some(&context), Some(1), 0).as_deref(), Some("b"));
    }
}
