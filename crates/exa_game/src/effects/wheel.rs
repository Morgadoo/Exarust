//! The domain wheel of the thaumaturgy screen: a 280 px form showing the
//! six domains on a ring, with the one under the mouse highlighted and a
//! click switching the power tree to another known domain.
//!
//! The ring's radius is 76 px. An icon takes a hit within 20.8 px of its
//! place, and the hit test rounds both the icon's place and the mouse to
//! whole pixels. Hovering keeps the highlight animation's start within the
//! last 200 ms; opening the wheel snapshots the shown value and animates it
//! over 300 ms. A confirm needs the 0x2000 modifier, a known domain, a
//! different previous choice and a free button, and plays a sound.
//!
//! The wheel adds no light. Its drawing (six ring segments, the highlighted
//! icon's frame, its label and the wheel's textures) is not reproduced.

use bevy::prelude::*;

/// The form's size in client pixels, its left margin and its form flags.
pub const SIZE_PX: i32 = 280;
pub const MARGIN_PX: i32 = 16;
pub const FORM_FLAGS: u32 = 0x0008_0000;
/// The constructor's default choice, its highlighted fade and its button
/// kind.
pub const DEFAULT_CHOICE: i32 = 2;
pub const DEFAULT_FADE: f32 = 0.5;
pub const BUTTON: i32 = 2;

/// The wheel's unit: icons and labels are built from it.
pub const UNIT_PX: f32 = 32.0;
/// The radius icons sit on, and the label's drop below its icon.
pub const RING_PX: f32 = 76.0;
pub const LABEL_DROP_PX: f32 = 40.0;
pub const LABEL_HALF_PX: f32 = 16.0;
/// The label's small frame is 2 units wide and 4 high.
pub const LABEL_WIDTH_UNITS: i32 = 4;
pub const LABEL_TOP_PX: i32 = 16;
/// A label is drawn at six tenths of its fade.
pub const LABEL_FADE: f32 = 0.6;

/// The six sectors.
pub const SECTORS: u32 = 6;
/// The angle step between icons: a sixth of a turn.
pub const SECTOR_ANGLE: f64 = std::f64::consts::TAU / 6.0;

/// The hit radius of an icon: 32 px × 0.65, squared and rounded.
pub const HIT_RADIUS_PX: f32 = UNIT_PX * 0.65;
pub const HIT_RADIUS_SQUARED: i32 = 433;

/// The highlight animation's length.
pub const HOVER_MS: u32 = 200;
/// The open animation's length.
pub const SHOW_MS: u32 = 300;
/// A confirmed choice holds its button for 256 ms.
pub const CONFIRM_MS: u32 = 256;
/// The modifier a confirm needs.
pub const CONFIRM_MODIFIER: u32 = 0x2000;

/// The six sector labels, in order. The third slot's text was not in the
/// recovered strings (the others are Mind, Force, Energy, Light and
/// Displacement).
pub const LABELS: [&str; SECTORS as usize] =
    ["Mind", "Force", "", "Energy", "Light", "Displacement"];
/// The domain each sector selects, in the original's id order: Mind,
/// Force, (unknown), Energy, Light, Displacement.
pub const DOMAINS: [Domain; SECTORS as usize] = [
    Domain::Mind,
    Domain::Force,
    Domain::Unknown,
    Domain::Energy,
    Domain::Light,
    Domain::Displacement,
];

/// The domains the wheel can switch the tree to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Mind,
    Force,
    Energy,
    Light,
    Displacement,
    /// The third sector's text was not recovered.
    Unknown,
}

/// The hover state of the wheel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wheel {
    /// The icon under the mouse (`+0x6c`).
    pub hot: i32,
    /// The highlighted icon (`+0x194`).
    pub highlighted: i32,
    /// The icon highlighted before it (`+0x198`).
    pub previous: i32,
    /// The chosen icon (`+0x174`).
    pub chosen: i32,
    /// The choice before it (`+0x178`).
    pub before: i32,
    /// The highlight animation's start stamp.
    pub hover_stamp: u32,
    /// The highlighted icon's fade (`+0x158`), its start (`+0x188`) and the
    /// animation's deadline (`+0x184`).
    pub fade: f32,
    pub fade_start: f32,
    pub fade_deadline: u32,
    /// The wheel's overall show (`+0x180`).
    pub shown: f32,
    /// The button's hold deadline (`+0x230`).
    pub confirm_deadline: u32,
    /// The button's chosen icon (`+0x1c0`).
    pub confirm_choice: i32,
    /// The frame clock of the last choice (`+0x17c`).
    pub chosen_at: u32,
}

impl Default for Wheel {
    fn default() -> Self {
        Self {
            hot: 0,
            highlighted: 0,
            previous: 0,
            chosen: DEFAULT_CHOICE,
            before: 0,
            hover_stamp: 0,
            fade: DEFAULT_FADE,
            fade_start: DEFAULT_FADE,
            fade_deadline: 0,
            shown: 1.0,
            confirm_deadline: 0,
            confirm_choice: 0,
            chosen_at: 0,
        }
    }
}

impl Wheel {
    /// The icon's angle: sector `index` (1..6) sits at `(index + 2)·60°`.
    pub fn sector_angle(index: i32) -> f64 {
        f64::from(index + 2) * SECTOR_ANGLE
    }

    /// The icon's place from the form's centre, in client pixels. The
    /// original rounds the icon's place to whole pixels.
    pub fn icon_at(centre: Vec2, index: i32) -> (i32, i32) {
        let angle = Self::sector_angle(index);
        let x = centre.x - RING_PX * angle.cos() as f32;
        let y = centre.y + RING_PX * angle.sin() as f32;
        (x.round_ties_even() as i32, y.round_ties_even() as i32)
    }

    /// The sector under `mouse`, or zero. The distance is compared as whole
    /// pixels against 433 (20.8², rounded).
    pub fn hit_test(centre: Vec2, mouse: (i32, i32)) -> i32 {
        for index in 1..=SECTORS as i32 {
            let (x, y) = Self::icon_at(centre, index);
            let dx = i64::from(x) - i64::from(mouse.0);
            let dy = i64::from(y) - i64::from(mouse.1);
            if dy * dy + dx * dx < i64::from(HIT_RADIUS_SQUARED) {
                return index;
            }
        }
        0
    }

    /// Open the wheel: the highlighted fade's start is its current value
    /// and the deadline is 300 ms away.
    pub fn open(&mut self, now: u32) {
        self.fade_start = self.fade;
        self.fade_deadline = now.wrapping_add(SHOW_MS);
    }

    /// The eased highlighted fade. A smoothstep stands in for the GUI's
    /// ease, whose two scale constants were not in the pack (guess).
    pub fn fade(&self, now: u32) -> f32 {
        if self.fade_deadline < now {
            return self.fade;
        }
        let left = self.fade_deadline.wrapping_sub(now) as f32 / SHOW_MS as f32;
        let eased = left * left * (3.0 - 2.0 * left);
        (self.fade - self.fade_start) * (1.0 - eased) + self.fade_start
    }

    /// The circular radius factor `(2 − shown)·shown` from the wheel's
    /// overall show.
    pub fn swell(&self) -> f32 {
        (2.0 - self.shown) * self.shown
    }

    /// The ring an icon sits on: 76 px × the swell.
    pub fn icon_radius(&self) -> f32 {
        RING_PX * self.swell()
    }

    /// The outer ring's radius: 140 px × the swell.
    pub fn outer_radius(&self) -> f32 {
        140.0 * self.swell()
    }

    /// A label's place from the form's centre, and its width from the
    /// measured text: the label drops 40 px and is centred on the icon.
    pub fn label_at(centre: Vec2, index: i32, text_width: i32) -> (i32, i32) {
        let (x, y) = Self::icon_at(centre, index);
        let width = LABEL_WIDTH_UNITS * UNIT_PX as i32 + text_width / 2;
        (x - width, y + LABEL_DROP_PX as i32 - LABEL_TOP_PX)
    }

    /// A hover over `index`. The highlight's animation start is kept within
    /// the last 200 ms as the original does: when the move restarts the
    /// highlight the stamp is now; otherwise a running animation is mirrored
    /// so its remaining time is its elapsed time.
    ///
    /// Returns false only when there is nothing to highlight at all.
    pub fn hover(&mut self, index: i32, now: u32) -> bool {
        if index <= 0 && self.hot <= 0 {
            return false;
        }
        if index == self.highlighted {
            self.hot = index;
            return true;
        }
        if index > 0 && self.previous > 0 && self.previous != index {
            self.hover_stamp = now;
        } else {
            let elapsed = now.wrapping_sub(self.hover_stamp);
            if elapsed < HOVER_MS {
                self.hover_stamp = now.wrapping_sub(HOVER_MS.wrapping_sub(elapsed));
            } else {
                self.hover_stamp = now;
            }
        }
        self.highlighted = index;
        if index > 0 {
            self.previous = index;
        }
        self.hot = index;
        true
    }

    /// Whether the highlight is still animating and how far it got: the
    /// label's fade, one at rest and falling with a running animation.
    pub fn highlight(&self, now: u32) -> Option<f32> {
        let elapsed = now.wrapping_sub(self.hover_stamp);
        if elapsed < HOVER_MS {
            if self.highlighted < 1 {
                return Some(1.0 - elapsed as f32 * 0.005);
            }
            return Some(1.0);
        }
        if self.highlighted < 1 {
            None
        } else {
            Some(1.0)
        }
    }

    /// Confirm the icon under the mouse. A press needs the modifier, a
    /// different previous choice, a known domain and a free button; a
    /// confirmed choice stamps the button 256 ms ahead, shifts the choices
    /// and plays a sound.
    pub fn confirm(&mut self, modifiers: u32, known: &[bool; SECTORS as usize], now: u32) -> bool {
        if self.hot <= 0
            || modifiers & CONFIRM_MODIFIER == 0
            || self.hot == self.chosen
            || !known[(self.hot - 1) as usize]
            || self.confirm_deadline > now
        {
            return false;
        }
        self.confirm_deadline = now.wrapping_add(CONFIRM_MS);
        self.confirm_choice = self.hot;
        self.before = self.chosen;
        self.chosen = self.hot;
        self.chosen_at = now;
        true
    }
}

/// The wheel component a form would carry.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct DomainWheel(pub Wheel);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icons_sit_on_a_sixty_degree_ring() {
        let (x, y) = Wheel::icon_at(Vec2::new(100.0, 100.0), 1);
        assert_eq!((x, y), (176, 100));
        let (x, y) = Wheel::icon_at(Vec2::new(100.0, 100.0), 2);
        assert_eq!((x, y), (138, 34));
        let (x, y) = Wheel::icon_at(Vec2::new(100.0, 100.0), 3);
        assert_eq!((x, y), (62, 34));
        assert_eq!(Wheel::icon_at(Vec2::new(100.0, 100.0), 4), (24, 100));
        assert_eq!(Wheel::sector_angle(1).to_degrees(), 180.0);
        assert!((Wheel::sector_angle(6).to_degrees() - 480.0).abs() < 1e-4);
        assert_eq!(HIT_RADIUS_SQUARED, 433);
        assert!((HIT_RADIUS_PX - 20.8).abs() < 1e-5);
    }

    #[test]
    fn a_hit_is_within_208_pixels_of_an_icon() {
        let centre = Vec2::new(100.0, 100.0);
        assert_eq!(Wheel::hit_test(centre, (176, 100)), 1);
        assert_eq!(Wheel::hit_test(centre, (176, 120)), 1);
        assert_eq!(Wheel::hit_test(centre, (176, 121)), 0);
        assert_eq!(Wheel::hit_test(centre, (100, 100)), 0);
        assert_eq!(Wheel::hit_test(centre, (138, 34)), 2);
        assert_eq!(Wheel::hit_test(centre, (62, 34)), 3);
    }

    #[test]
    fn a_hover_restarts_the_highlight_for_a_new_icon() {
        let mut wheel = Wheel::default();
        assert!(!wheel.hover(0, 1_000));
        assert!(wheel.hover(3, 1_000));
        assert_eq!((wheel.hot, wheel.highlighted, wheel.previous), (3, 3, 3));
        assert_eq!(wheel.hover_stamp, 1_000);
        // The same icon again changes nothing.
        assert!(wheel.hover(3, 1_100));
        assert_eq!(wheel.hover_stamp, 1_000);
        // A new icon with a previous one restarts at now.
        assert!(wheel.hover(5, 1_200));
        assert_eq!(wheel.hover_stamp, 1_200);
        assert_eq!(wheel.previous, 5);
        // Moving off the wheel keeps the previous icon and mirrors a running
        // animation: elapsed 50 ms becomes 150 ms, so the stamp moves back.
        assert!(wheel.hover(0, 1_250));
        assert_eq!(wheel.hot, 0);
        assert_eq!(wheel.highlighted, 0);
        assert_eq!(wheel.previous, 5);
        assert_eq!(wheel.hover_stamp, 1_250 - 150);
        assert_eq!(wheel.highlight(1_250), Some(1.0 - 150.0 * 0.005));
        assert_eq!(wheel.highlight(1_350), None);
    }

    #[test]
    fn a_running_highlight_lasts_two_hundred_milliseconds() {
        let mut wheel = Wheel::default();
        wheel.hover(2, 1_000);
        assert_eq!(wheel.highlight(1_000), Some(1.0));
        assert_eq!(wheel.highlight(1_100), Some(1.0));
        assert_eq!(wheel.highlight(1_200), Some(1.0));
        wheel.hover(0, 1_250);
        assert_eq!(wheel.highlight(1_300), Some(0.75));
        assert!(wheel.highlight(1_450).is_none());
        // A long gap since the last hover starts a fresh animation.
        wheel.hover(4, 2_000);
        assert_eq!(wheel.hover_stamp, 2_000);
    }

    #[test]
    fn a_open_animation_runs_over_three_hundred_milliseconds() {
        let mut wheel = Wheel::default();
        wheel.open(1_000);
        assert_eq!(wheel.fade_deadline, 1_300);
        assert_eq!(wheel.fade(1_000), DEFAULT_FADE);
        wheel.fade = 1.0;
        wheel.open(2_000);
        assert_eq!(wheel.fade_start, 1.0);
        assert_eq!(wheel.fade(2_300), 1.0);
        wheel.fade_start = 0.5;
        wheel.fade_deadline = 2_300;
        assert_eq!(wheel.fade(2_150), 0.75);
        assert_eq!(wheel.fade(2_301), 1.0);
        // The radii follow the wheel's overall show, not the fade.
        assert_eq!(wheel.swell(), 1.0);
        assert_eq!(wheel.icon_radius(), 76.0);
        assert_eq!(wheel.outer_radius(), 140.0);
        wheel.shown = 0.5;
        assert_eq!(wheel.swell(), 0.75);
        assert_eq!(wheel.icon_radius(), 57.0);
        assert_eq!(wheel.outer_radius(), 105.0);
    }

    #[test]
    fn a_confirm_needs_a_known_domain_for_the_hovered_icon() {
        let mut wheel = Wheel {
            hot: 2,
            chosen: 1,
            ..Wheel::default()
        };
        let mut known = [false; SECTORS as usize];
        known[1] = true;
        // The modifier is required.
        assert!(!wheel.confirm(0, &known, 1_000));
        // An unknown domain is refused.
        let nothing: [bool; SECTORS as usize] = [false; SECTORS as usize];
        assert!(!wheel.confirm(CONFIRM_MODIFIER, &nothing, 1_000));
        assert!(wheel.confirm(CONFIRM_MODIFIER, &known, 1_000));
        assert_eq!(wheel.chosen, 2);
        assert_eq!(wheel.before, 1);
        assert_eq!(wheel.confirm_deadline, 1_256);
        assert_eq!(wheel.confirm_choice, 2);
        assert_eq!(wheel.chosen_at, 1_000);
        // The button is held for 256 ms; another icon may then be chosen.
        wheel.hot = 3;
        known[2] = true;
        assert!(!wheel.confirm(CONFIRM_MODIFIER, &known, 1_255));
        assert!(wheel.confirm(CONFIRM_MODIFIER, &known, 1_256));
        assert_eq!(wheel.chosen, 3);
        assert_eq!(wheel.before, 2);
        // Choosing the current domain again does nothing.
        wheel.hot = 3;
        assert!(!wheel.confirm(CONFIRM_MODIFIER, &known, 2_000));
    }

    #[test]
    fn a_label_drops_below_its_icon() {
        let (x, y) = Wheel::label_at(Vec2::new(100.0, 100.0), 1, 20);
        assert_eq!((x, y), (176 - 138, 100 + 24));
        assert_eq!(LABEL_DROP_PX, 40.0);
        assert_eq!(LABELS[1], "Force");
        assert_eq!(DOMAINS[5], Domain::Displacement);
        assert_eq!(LABELS[0], "Mind");
        assert_eq!(SIZE_PX, 280);
        assert_eq!(MARGIN_PX, 16);
        assert_eq!(FORM_FLAGS, 0x0008_0000);
        assert_eq!(BUTTON, 2);
        assert_eq!(CONFIRM_MODIFIER, 0x2000);
        assert_eq!(DEFAULT_FADE, 0.5);
    }
}
