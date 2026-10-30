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
/// The eighteen rotating tips, in the original's table order.
pub const TIPS: [&str; 18] = [
    FIRST_TIP,
    "Your character is subject to physics, your inputs must work in harmony with them.",
    "Even when a parry is not successful, attempting to defend will reduce damage taken.",
    "Let go of LMB at any time to perform a feint or abandon your attack to defend instead.",
    "To successfully block or parry you must turn and position yourself to face the blow.",
    "NPCs play by the same exact rules, make human errors and have slow reaction times.",
    "To attack from your left, move the cursor to your character's left before pressing LMB.",
    "To deliver significant damage you must land clean hits with the correct part of your weapon.",
    "Use dashes to attack your opponents from unexpected angles and bypass their defences.",
    "Watch your opponent closely and land your attacks when they are most vulnerable.",
    "Overhead attacks can deliver devastating head blows and hit prone opponents.",
    "Slash damage is effective on weak and slicing blows but easily mitigated by armour.",
    "Piercing damage is better at finding potential gaps and weak spots in armour.",
    "Crushing damage is reliant on strong blows but typically most effective against armour.",
    "Weapons with a higher balance are quicker and more responsive.",
    "Impact mainly deals stamina damage but also affects all other damage types.",
    "Armour can be worn in layers for even better protection.",
    "Higher coverage armour has fewer exposed parts and weaknesses.",
];
/// The six indexed messages the same popup shows: the control hint, the
/// defeat and death notices and the three victory notices.
pub const HINTS: [&str; 6] = [
    "Press TAB to toggle combat mode or I to toggle the inventory. Double click bodies to loot them.",
    "You have been defeated.",
    "You have died.",
    "Congratulations! You have defeated all ten opponents.",
    "Congratulations! You have defeated all opponents in the novice arena!",
    "Congratulations! You have defeated all opponents in the expert arena!",
];
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
    /// left edge is half the difference of the window's width and the
    /// box's (a logical shift of the 64-bit difference; its low 32 bits
    /// are those of a signed halving, so a box wider than the window
    /// starts left of it).
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
    fn the_tables_are_the_original_seventeen_plus_one_and_six() {
        assert_eq!(TIPS.len(), 18);
        assert_eq!(HINTS.len(), 6);
        assert_eq!(TIPS[0], FIRST_TIP);
        assert_eq!(
            TIPS[17],
            "Higher coverage armour has fewer exposed parts and weaknesses."
        );
        assert!(HINTS[0].starts_with("Press TAB"));
        assert_eq!(HINTS[2], "You have died.");
        assert!(HINTS[5].contains("expert arena"));
    }

    #[test]
    fn a_box_wider_than_the_window_starts_left_of_it() {
        let b = GameTips::new().draw(100, 4, 200);
        assert_eq!(b.box_at[0], -54);
    }
}
