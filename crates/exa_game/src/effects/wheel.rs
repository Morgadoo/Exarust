//! The domain wheel of the thaumaturgy screen: a 280 px form showing the
//! six domains on a ring, with the one under the mouse highlighted and a
//! click switching the power tree to another known domain.
//!
//! Icon 1 sits at the top of a 76 px ring and the others follow it
//! clockwise, a sixth of a turn apart. An icon takes a hit within 20.8 px
//! of its place; the hit test rounds the icon's place to whole pixels and
//! compares squared whole-pixel distances against 433. Hovering keeps the
//! highlight animation's start within the last 200 ms; the label of the
//! last highlighted icon fades in or out over those 200 ms below its icon.
//! Opening the wheel eases its zoom value from where it was over 300 ms
//! with `(3 − 2x)·x²`. A confirm needs the 0x2000 modifier, a known domain,
//! a different previous choice and a free button; it plays a sound at 0.25.
//!
//! The wheel reads two clocks, as the original does: the game clock (which
//! pauses) for the open ease and the button's hold, and the real clock for
//! the hover, the label and the choice's flash.
//!
//! The wheel adds no light. Its drawing (the background program, six ring
//! segments, the icons, their frames and the label's text) is not
//! reproduced; the places, sizes and alphas it draws with are.

use bevy::prelude::*;

/// The form's size in client pixels, its left margin and its form flags.
pub const SIZE_PX: i32 = 280;
pub const MARGIN_PX: i32 = 16;
pub const FORM_FLAGS: u32 = 0x0008_0000;
/// The constructor's default choice and zoom value.
pub const DEFAULT_CHOICE: i32 = 2;
pub const DEFAULT_FADE: f32 = 0.5;

/// The GUI scale the ring's radii are multiplied by. The original reads a
/// global whose shipped value is one.
pub const GUI_SCALE: f32 = 1.0;
/// The icon unit (32 px): the hit radius and the icons' sizes are built
/// from it.
pub const UNIT_PX: i32 = 32;
/// The text unit (4 px): the label's drop, padding and height are built
/// from it.
pub const TEXT_UNIT_PX: i32 = 4;
/// The radius icons sit on, before the GUI scale.
pub const RING_PX: f32 = 76.0;
/// The outer ring's radius, before the GUI scale.
pub const OUTER_RING_PX: f32 = 140.0;
/// The label drops ten text units below its icon, is four text units high
/// and padded by two on each side of its text.
pub const LABEL_DROP_UNITS: i32 = 10;
pub const LABEL_HEIGHT_UNITS: i32 = 4;
pub const LABEL_PAD_UNITS: i32 = 2;
/// A label's frame is drawn at six tenths of its fade.
pub const LABEL_FADE: f64 = 0.6;

/// The six sectors.
pub const SECTORS: u32 = 6;

/// The hit radius of an icon: 32 px × 0.65. The original squares it in
/// double precision and rounds it: 432.64 becomes 433.
pub const HIT_RADIUS_SCALE: f64 = 0.65;
pub const HIT_RADIUS_SQUARED: i64 = 433;

/// The highlight animation's length, and its rate per millisecond.
pub const HOVER_MS: u32 = 200;
pub const HOVER_PER_MS: f64 = 0.005;
/// The open animation's length.
pub const SHOW_MS: u32 = 300;
/// A confirmed choice holds its button for 256 ms (the original reads the
/// hold from a global whose shipped value is 256).
pub const CONFIRM_MS: u32 = 256;
/// The modifier a confirm needs.
pub const CONFIRM_MODIFIER: u32 = 0x2000;
/// The confirm sound's volume and pitch.
pub const CONFIRM_VOLUME: f32 = 0.25;
pub const CONFIRM_PITCH: f32 = 1.0;

/// The six sector labels, in order.
pub const LABELS: [&str; SECTORS as usize] =
    ["Mind", "Force", "Body", "Energy", "Light", "Displacement"];
/// The domain each sector selects, in the label table's order.
pub const DOMAINS: [Domain; SECTORS as usize] = [
    Domain::Mind,
    Domain::Force,
    Domain::Body,
    Domain::Energy,
    Domain::Light,
    Domain::Displacement,
];

/// The domains the wheel can switch the tree to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Mind,
    Force,
    Body,
    Energy,
    Light,
    Displacement,
}

/// A label's frame and its text's place, in client pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Where the text starts.
    pub text_x: f32,
    pub text_y: f32,
}

/// The wheel's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wheel {
    /// The icon under the mouse.
    pub hot: i32,
    /// The highlighted icon.
    pub highlighted: i32,
    /// The last icon highlighted, whose label is drawn.
    pub previous: i32,
    /// The chosen icon.
    pub chosen: i32,
    /// The choice before it, fading out.
    pub before: i32,
    /// The highlight animation's start, real clock.
    pub hover_stamp: u32,
    /// The zoom value the open animation eases to, where it eases from and
    /// the animation's deadline (game clock).
    pub fade: f32,
    pub fade_start: f32,
    pub fade_deadline: u32,
    /// The wheel's overall show, 0..1.
    pub shown: f32,
    /// The button's hold deadline (game clock).
    pub confirm_deadline: u32,
    /// The icon the button carries.
    pub confirm_choice: i32,
    /// When the last choice was made, real clock.
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
            fade_start: 0.0,
            fade_deadline: 0,
            shown: 1.0,
            confirm_deadline: 0,
            confirm_choice: 0,
            chosen_at: 0,
        }
    }
}

/// The ring's radius in whole pixels: 76 px × the GUI scale, rounded.
fn ring_px() -> i32 {
    (RING_PX * GUI_SCALE).round_ties_even() as i32
}

impl Wheel {
    /// The icon's angle, radians, as the hit test and the drawing take it:
    /// `(index + 2)·2π/6` in single precision. Icon 1 is at half a turn.
    pub fn sector_angle(index: i32) -> f32 {
        (index + 2) as f32 * std::f32::consts::TAU * (1.0 / 6.0)
    }

    /// The icon's place: `centre.x − r·sin a`, `r·cos a + centre.y`, each
    /// rounded to whole pixels. The centre is half the form's size.
    pub fn icon_at(centre: (i32, i32), index: i32) -> (i32, i32) {
        let (sin, cos) = Self::sector_angle(index).sin_cos();
        let r = ring_px() as f32;
        let x = centre.0 as f32 - r * sin;
        let y = r * cos + centre.1 as f32;
        (x.round_ties_even() as i32, y.round_ties_even() as i32)
    }

    /// The sector under `mouse`, or zero.
    pub fn hit_test(centre: (i32, i32), mouse: (i32, i32)) -> i32 {
        for index in 1..=SECTORS as i32 {
            let (x, y) = Self::icon_at(centre, index);
            let dx = i64::from(x) - i64::from(mouse.0);
            let dy = i64::from(y) - i64::from(mouse.1);
            if dy * dy + dx * dx < HIT_RADIUS_SQUARED {
                return index;
            }
        }
        0
    }

    /// Open the wheel: the zoom eases from its current value and the
    /// deadline is 300 ms away on the game clock.
    pub fn open(&mut self, game_now: u32) {
        self.fade_start = self.fade;
        self.fade_deadline = game_now.wrapping_add(SHOW_MS);
    }

    /// The eased zoom value: past the deadline the target itself, before it
    /// `(fade − start)·(1 − e(x)) + start` with `x` the time left over
    /// 300 ms and `e(x) = (3 − 2x)·x²`.
    pub fn fade(&self, game_now: u32) -> f32 {
        if self.fade_deadline < game_now {
            return self.fade;
        }
        let left = (super::span_ms(self.fade_deadline, game_now) as f64
            * (1.0 / f64::from(SHOW_MS))) as f32;
        let eased = (3.0 - 2.0 * left) * (left * left);
        (self.fade - self.fade_start) * (1.0 - eased) + self.fade_start
    }

    /// The circular radius factor `(2 − shown)·shown` from the wheel's
    /// overall show.
    pub fn swell(&self) -> f32 {
        (2.0 - self.shown) * self.shown
    }

    /// The ring an icon is drawn on: 76 px × the GUI scale × the swell.
    pub fn icon_radius(&self) -> f32 {
        GUI_SCALE * RING_PX * self.swell()
    }

    /// The outer ring's radius: 140 px × the GUI scale × the swell.
    pub fn outer_radius(&self) -> f32 {
        GUI_SCALE * OUTER_RING_PX * self.swell()
    }

    /// A label's frame for sector `index` whose text is `text_width` pixels
    /// wide. The label's angle is taken in double precision, `(index + 2)·π/3`,
    /// and its place is not rounded: it drops ten text units below its
    /// icon, its frame is padded by two text units around the text's half
    /// width and is four text units high, and its text is centred on the
    /// icon.
    pub fn label_at(centre: (i32, i32), index: i32, text_width: u32) -> LabelBox {
        let r = ring_px() as f32;
        let angle = ((f64::from(index) + 2.0) * std::f64::consts::PI * (1.0 / 3.0)) as f32;
        let (sin, cos) = angle.sin_cos();
        let x = centre.0 as f32 + -r * sin;
        let y = (TEXT_UNIT_PX * LABEL_DROP_UNITS) as f32 + (centre.1 as f32 + r * cos);
        let height = (TEXT_UNIT_PX * LABEL_HEIGHT_UNITS) as f32;
        let half = (TEXT_UNIT_PX * LABEL_PAD_UNITS) as f32 + (text_width >> 1) as f32;
        LabelBox {
            x: x - half,
            y: y - height,
            width: half * 2.0,
            height,
            text_x: x - (text_width >> 1) as f32,
            text_y: y,
        }
    }

    /// A hover over `index`. The highlight's animation start is kept within
    /// the last 200 ms as the original does: when the move goes from one
    /// icon straight to another the stamp is now; otherwise a running
    /// animation is mirrored so its remaining time is its elapsed time.
    ///
    /// Returns false only when there is nothing to highlight at all; the
    /// hot icon is stored either way.
    pub fn hover(&mut self, index: i32, real_now: u32) -> bool {
        if index <= 0 && self.hot <= 0 {
            self.hot = index;
            return false;
        }
        if index == self.highlighted {
            self.hot = index;
            return true;
        }
        if index > 0 && self.previous > 0 && self.previous != index {
            self.hover_stamp = real_now;
        } else {
            let elapsed = real_now.wrapping_sub(self.hover_stamp);
            if elapsed < HOVER_MS {
                self.hover_stamp = real_now.wrapping_sub(HOVER_MS - elapsed);
            } else {
                self.hover_stamp = real_now;
            }
        }
        self.highlighted = index;
        if index > 0 {
            self.previous = index;
        }
        self.hot = index;
        true
    }

    /// The fade the last highlighted icon's label is drawn with, or none
    /// when no label is drawn. While the highlight animates (a signed
    /// elapsed time under 200 ms) the fade rises at 0.005 a millisecond, or
    /// falls from one when nothing is highlighted; after it, a highlighted
    /// icon's label stays at one and an unhighlighted one is dropped.
    pub fn label_fade(&mut self, real_now: u32) -> Option<f32> {
        if self.previous <= 0 {
            return None;
        }
        let elapsed = real_now.wrapping_sub(self.hover_stamp) as i32;
        if elapsed < HOVER_MS as i32 {
            let rise = (f64::from(elapsed) * HOVER_PER_MS) as f32;
            return Some(if self.highlighted < 1 {
                1.0 - rise
            } else {
                rise
            });
        }
        if self.highlighted < 1 {
            self.previous = 0;
            return None;
        }
        Some(1.0)
    }

    /// The label frame's alpha for a label fade: six tenths of it.
    pub fn label_alpha(fade: f32) -> f32 {
        (f64::from(fade) * LABEL_FADE) as f32
    }

    /// The choice's flash: how far the last choice is into its 200 ms,
    /// clamped to one, at which the previous choice is dropped. With it,
    /// the alphas (0..255) the previous and the new choice's frames are
    /// drawn with.
    pub fn choice_flash(&mut self, real_now: u32) -> (f32, i32, i32) {
        let mut flash = (super::span_ms(real_now, self.chosen_at) as f64 * HOVER_PER_MS) as f32;
        if 1.0 < flash {
            flash = 1.0;
            self.before = 0;
        }
        let before = ((1.0 - flash * 0.75) * 255.0).round_ties_even() as i32 & 0xff;
        let chosen = ((flash * 0.75 + 0.25) * 255.0 * self.swell()).round_ties_even() as i32 & 0xff;
        (flash, before, chosen)
    }

    /// Confirm the icon under the mouse. A press needs a hot icon, the
    /// modifier, a choice different from the current one, a known domain
    /// and a free button. A confirmed choice holds the button 256 ms on the
    /// game clock, shifts the choices, stamps the real clock and plays the
    /// confirm sound. The button's flag is modelled as that hold (guess:
    /// its clearing code is outside this group).
    pub fn confirm(
        &mut self,
        modifiers: u32,
        known: &[bool; SECTORS as usize],
        game_now: u32,
        real_now: u32,
    ) -> bool {
        let Some(slot) = usize::try_from(self.hot - 1)
            .ok()
            .filter(|slot| *slot < SECTORS as usize)
        else {
            return false;
        };
        if modifiers & CONFIRM_MODIFIER == 0
            || self.hot == self.chosen
            || !known[slot]
            || self.confirm_deadline > game_now
        {
            return false;
        }
        self.confirm_deadline = game_now.wrapping_add(CONFIRM_MS);
        self.confirm_choice = self.hot;
        self.before = self.chosen;
        self.chosen = self.hot;
        self.chosen_at = real_now;
        true
    }
}

/// The wheel component a form would carry.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct DomainWheel(pub Wheel);

#[cfg(test)]
mod tests {
    use super::*;

    const CENTRE: (i32, i32) = (100, 100);

    #[test]
    fn the_icons_run_clockwise_from_the_top() {
        assert_eq!(Wheel::icon_at(CENTRE, 1), (100, 24));
        assert_eq!(Wheel::icon_at(CENTRE, 2), (166, 62));
        assert_eq!(Wheel::icon_at(CENTRE, 3), (166, 138));
        assert_eq!(Wheel::icon_at(CENTRE, 4), (100, 176));
        assert_eq!(Wheel::icon_at(CENTRE, 5), (34, 138));
        assert_eq!(Wheel::icon_at(CENTRE, 6), (34, 62));
        assert!((Wheel::sector_angle(1) - std::f32::consts::PI).abs() < 1e-6);
        assert_eq!(HIT_RADIUS_SQUARED, {
            let r = f64::from(UNIT_PX) * HIT_RADIUS_SCALE;
            (r * r).round_ties_even() as i64
        });
    }

    #[test]
    fn a_hit_is_within_208_pixels_of_an_icon() {
        assert_eq!(Wheel::hit_test(CENTRE, (100, 24)), 1);
        // 20² = 400 < 433; 21² = 441 is not.
        assert_eq!(Wheel::hit_test(CENTRE, (120, 24)), 1);
        assert_eq!(Wheel::hit_test(CENTRE, (121, 24)), 0);
        // 12² + 17² = 433 is not under 433; 12² + 16² = 400 is.
        assert_eq!(Wheel::hit_test(CENTRE, (112, 41)), 0);
        assert_eq!(Wheel::hit_test(CENTRE, (112, 40)), 1);
        assert_eq!(Wheel::hit_test(CENTRE, (100, 100)), 0);
        assert_eq!(Wheel::hit_test(CENTRE, (166, 62)), 2);
        assert_eq!(Wheel::hit_test(CENTRE, (34, 62)), 6);
    }

    #[test]
    fn a_hover_restarts_the_highlight_for_a_new_icon() {
        let mut wheel = Wheel::default();
        assert!(!wheel.hover(-1, 1_000));
        assert_eq!(wheel.hot, -1);
        assert!(wheel.hover(3, 1_000));
        assert_eq!((wheel.hot, wheel.highlighted, wheel.previous), (3, 3, 3));
        // A first highlight long after the last starts now.
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
        assert_eq!((wheel.hot, wheel.highlighted, wheel.previous), (0, 0, 5));
        assert_eq!(wheel.hover_stamp, 1_250 - 150);
    }

    #[test]
    fn a_label_fades_in_over_two_hundred_milliseconds_and_out_again() {
        let mut wheel = Wheel::default();
        assert_eq!(wheel.label_fade(0), None);
        wheel.hover(2, 1_000);
        assert_eq!(wheel.label_fade(1_000), Some(0.0));
        assert_eq!(wheel.label_fade(1_100), Some(0.5));
        assert_eq!(wheel.label_fade(1_199), Some((199.0 * HOVER_PER_MS) as f32));
        assert_eq!(wheel.label_fade(1_200), Some(1.0));
        // Off the wheel long after the highlight finished: a fresh fade-out.
        wheel.hover(0, 1_250);
        assert_eq!(wheel.label_fade(1_250), Some(1.0));
        assert_eq!(wheel.label_fade(1_350), Some(0.5));
        // Once the fade-out is over the label is dropped.
        assert_eq!(wheel.label_fade(1_450), None);
        assert_eq!(wheel.previous, 0);
        // Off the wheel 50 ms into a fade-in: the fade-out starts 150 ms in.
        wheel.hover(4, 1_500);
        wheel.hover(0, 1_550);
        assert_eq!(wheel.hover_stamp, 1_400);
        assert_eq!(wheel.label_fade(1_550), Some(0.25));
        assert_eq!(wheel.label_fade(1_600), None);
        assert_eq!(Wheel::label_alpha(0.5), 0.3);
        // A stamp ahead of the clock is a negative span: a falling label
        // above one.
        let mut ahead = Wheel {
            previous: 1,
            hover_stamp: 1_010,
            ..Wheel::default()
        };
        assert_eq!(ahead.label_fade(1_000), Some(1.05));
    }

    #[test]
    fn a_label_drops_below_its_icon() {
        // Icon 1 is at (100, 24); its label is 40 px lower, unrounded, its
        // text 20 px wide: half width 8 + 10, height 16.
        let label = Wheel::label_at(CENTRE, 1, 20);
        assert!((label.x - 82.0).abs() < 1e-4, "{label:?}");
        assert!((label.y - 48.0).abs() < 1e-4, "{label:?}");
        assert_eq!((label.width, label.height), (36.0, 16.0));
        assert!((label.text_x - 90.0).abs() < 1e-4);
        assert!((label.text_y - 64.0).abs() < 1e-4);
        // An odd width drops its last pixel from the half.
        assert_eq!(Wheel::label_at(CENTRE, 1, 21).width, 36.0);
        assert_eq!(LABELS[2], "Body");
        assert_eq!(DOMAINS[5], Domain::Displacement);
    }

    #[test]
    fn a_open_animation_eases_over_three_hundred_milliseconds() {
        let mut wheel = Wheel::default();
        assert_eq!(wheel.fade_start, 0.0);
        wheel.open(1_000);
        assert_eq!(wheel.fade_deadline, 1_300);
        assert_eq!(wheel.fade_start, DEFAULT_FADE);
        assert_eq!(wheel.fade(1_000), DEFAULT_FADE);
        wheel.fade = 1.0;
        wheel.fade_start = 0.5;
        // Halfway: e(0.5) = 0.5, so halfway between.
        assert_eq!(wheel.fade(1_150), 0.75);
        // At the deadline the time left is zero and the target is reached.
        assert_eq!(wheel.fade(1_300), 1.0);
        assert_eq!(wheel.fade(1_301), 1.0);
        // At the start the ease is one: still at the start.
        assert_eq!(wheel.fade(1_000), 0.5);
        // The radii follow the wheel's overall show.
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
        assert!(!wheel.confirm(0, &known, 1_000, 50));
        // An unknown domain is refused.
        let nothing = [false; SECTORS as usize];
        assert!(!wheel.confirm(CONFIRM_MODIFIER, &nothing, 1_000, 50));
        assert!(wheel.confirm(CONFIRM_MODIFIER, &known, 1_000, 50));
        assert_eq!((wheel.chosen, wheel.before), (2, 1));
        assert_eq!(wheel.confirm_deadline, 1_256);
        assert_eq!(wheel.confirm_choice, 2);
        // The choice is stamped on the real clock.
        assert_eq!(wheel.chosen_at, 50);
        // The button is held for 256 ms; another icon may then be chosen.
        wheel.hot = 3;
        known[2] = true;
        assert!(!wheel.confirm(CONFIRM_MODIFIER, &known, 1_255, 60));
        assert!(wheel.confirm(CONFIRM_MODIFIER, &known, 1_256, 60));
        assert_eq!((wheel.chosen, wheel.before), (3, 2));
        // Choosing the current domain again does nothing.
        assert!(!wheel.confirm(CONFIRM_MODIFIER, &known, 2_000, 70));
        // Nor does a hot icon off the ring.
        let mut out = Wheel {
            hot: 99,
            ..Wheel::default()
        };
        assert!(!out.confirm(CONFIRM_MODIFIER, &[true; SECTORS as usize], 0, 0));
        assert_eq!((CONFIRM_VOLUME, CONFIRM_PITCH), (0.25, 1.0));
    }

    #[test]
    fn a_choice_flashes_over_two_hundred_milliseconds() {
        let mut wheel = Wheel {
            chosen: 3,
            before: 2,
            chosen_at: 1_000,
            ..Wheel::default()
        };
        assert_eq!(wheel.choice_flash(1_000), (0.0, 255, 64));
        assert_eq!(wheel.choice_flash(1_100), (0.5, 159, 159));
        assert_eq!(wheel.before, 2);
        assert_eq!(wheel.choice_flash(1_201), (1.0, 64, 255));
        assert_eq!(wheel.before, 0);
    }

    #[test]
    fn the_form_has_the_original_size_and_flags() {
        assert_eq!((SIZE_PX, MARGIN_PX, FORM_FLAGS), (280, 16, 0x0008_0000));
        let wheel = Wheel::default();
        assert_eq!((wheel.chosen, wheel.fade, wheel.shown), (2, 0.5, 1.0));
    }
}
