//! The global tooltip window and its base class: the delayed show, the
//! auto-hide timer, the fade, and where the box goes next to the pointer.
//!
//! A shown tooltip fades in over 256 clock units, stays up for 2500 more
//! and fades out over 256. Showing one while another fades out reverses the
//! fade instead of restarting it. The box is its text's width plus four
//! units, two units taller than a line, kept on the screen, and is drawn
//! with a translucent background whose alpha is 153 at full strength. The
//! skill tooltip is 50 units wide and shows its title at 2, 5 units and its
//! body in a box from 2, 6 units.
//!
//! Evidence: the tooltip base's show/hide, the tooltip window's show,
//! animate and hide routines and the skill tooltip's show and draw
//! (`docs/UI.md`, tooltips).

/// One GUI unit in pixels, the original's fixed design unit.
pub const UNIT: i32 = 4;
/// The fade's length, clock units.
pub const FADE_MS: u32 = 256;
/// How long a tooltip stays up after its fade-in, clock units.
pub const HIDE_AFTER_MS: u32 = 2500;
/// The tooltip background's alpha at full strength.
pub const BACKGROUND_ALPHA: f32 = 153.0;
/// The tooltip's width beyond its text, in units.
pub const WIDTH_PAD_UNITS: i32 = 4;
/// The tooltip's height beyond its font's line, in units.
pub const HEIGHT_PAD_UNITS: i32 = 2;
/// The right margin a clamped tooltip keeps, pixels.
pub const RIGHT_MARGIN: i32 = 2;
/// The distance the tooltip keeps from the pointer below it, pixels.
pub const BELOW_POINTER: i32 = 32;

/// The base tooltip's show/hide state (`TAbstractToolTip`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ToolTipState {
    /// 0 idle, 1 fading in, 2 fading out.
    pub state: u8,
    /// The clock the fade ends at (the effect's start, in reverse).
    pub fade_at: u32,
    /// The clock the tooltip hides by itself at.
    pub hide_at: u32,
    /// The tooltip is up (fading out included).
    pub showing: bool,
    /// The last alpha drawn, 0…1.
    pub alpha: f32,
}

impl ToolTipState {
    /// The base show: start the fade and the auto-hide timer. A tooltip
    /// already fading out reverses its fade.
    pub fn show(&mut self, now: u32) {
        self.fade_at = if self.state == 2 {
            now.wrapping_mul(2)
                .wrapping_add(FADE_MS)
                .wrapping_sub(self.fade_at)
        } else {
            now.wrapping_add(FADE_MS)
        };
        self.state = 1;
        self.hide_at = self.fade_at.wrapping_add(HIDE_AFTER_MS);
        self.showing = true;
    }

    /// The base hide: begin a 256-unit fade-out. Returns whether the fade
    /// began (a second hide while fading does nothing).
    pub fn hide(&mut self, now: u32) -> bool {
        if !self.showing {
            return false;
        }
        self.showing = false;
        self.state = 2;
        self.fade_at = now.wrapping_add(FADE_MS);
        true
    }

    /// One animation step: the alpha to draw and whether the form should be
    /// taken down. A hidden, idle tooltip asks to hide every step.
    pub fn animate(&mut self, now: u32) -> ToolTipFrame {
        match self.state {
            1 => {
                if now < self.fade_at {
                    let left = u64::from(self.fade_at) - u64::from(now);
                    self.alpha = (1.0 - left as f64 / f64::from(FADE_MS)) as f32;
                    ToolTipFrame {
                        alpha: self.alpha,
                        close: false,
                    }
                } else {
                    self.alpha = 1.0;
                    self.state = 0;
                    ToolTipFrame {
                        alpha: 1.0,
                        close: false,
                    }
                }
            }
            2 => {
                if self.fade_at <= now {
                    self.alpha = 0.0;
                    self.state = 0;
                    ToolTipFrame {
                        alpha: 0.0,
                        close: true,
                    }
                } else {
                    let left = u64::from(self.fade_at) - u64::from(now);
                    self.alpha = (left as f64 / f64::from(FADE_MS)) as f32;
                    ToolTipFrame {
                        alpha: self.alpha,
                        close: false,
                    }
                }
            }
            _ => ToolTipFrame {
                alpha: self.alpha,
                close: !self.showing,
            },
        }
    }
}

/// One tooltip animation step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolTipFrame {
    pub alpha: f32,
    pub close: bool,
}

/// The background alpha at a fade level: level × 153, rounded half to even.
pub fn background_alpha(level: f32) -> u8 {
    (level * BACKGROUND_ALPHA).round_ties_even() as u8
}

/// Where the tooltip box sits, in window pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolTipLayout {
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
    /// The child clip's width: the box's width less one.
    pub clip_width: i32,
}

/// Sizes and places the tooltip from its text width, its font's line
/// height, the pointer and the screen.
///
/// Across: the box is centred on the pointer, but a pointer within half a
/// box of the right edge right-aligns it two pixels in, and a pointer left
/// of the screen puts it at the edge. Down: below the pointer by 32 pixels
/// plus the box's height, or above it when that would leave the screen.
pub fn layout(
    text_width: i32,
    line_height: i32,
    pointer: (i32, i32),
    screen: (i32, i32),
) -> ToolTipLayout {
    let width = text_width.wrapping_add(UNIT * WIDTH_PAD_UNITS);
    let height = line_height.wrapping_add(UNIT * HEIGHT_PAD_UNITS);
    let half = width / 2;
    let (mx, my) = pointer;
    let x = if i64::from(screen.0) < i64::from(mx) + i64::from(half) {
        // The original's second test repeats the first, so the strip
        // branch is unreachable.
        if mx < 1 || i64::from(screen.0) < i64::from(mx) + i64::from(half) {
            screen.0.wrapping_sub(width).wrapping_sub(RIGHT_MARGIN)
        } else {
            mx
        }
    } else if mx < 1 {
        0
    } else {
        mx.wrapping_sub(half)
    };
    let below = f64::from(BELOW_POINTER) + f64::from(height);
    let y = if i64::from(screen.1) < i64::from(my) + below.round_ties_even() as i64 {
        my.wrapping_sub(height)
    } else {
        my.wrapping_add(height)
    };
    ToolTipLayout {
        width,
        height,
        x,
        y,
        clip_width: width.wrapping_sub(1),
    }
}

/// What the tooltip window's draw emits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolTipDraw {
    pub layout: ToolTipLayout,
    pub background_alpha: u8,
    /// The text's origin: two units in, one unit up from the bottom.
    pub text_at: [i32; 2],
}

/// The tooltip window's draw at a fade level.
pub fn draw(layout: ToolTipLayout, alpha: f32) -> ToolTipDraw {
    ToolTipDraw {
        layout,
        background_alpha: background_alpha(alpha),
        text_at: [UNIT * 2, layout.height.wrapping_sub(UNIT)],
    }
}

/// The skill tooltip's title and body origins, in units.
pub const SKILL_TITLE_UNITS: [i32; 2] = [2, 5];
pub const SKILL_BODY_UNITS: [i32; 2] = [2, 6];
/// The skill tooltip's width, in units, plus its odd-unit pixel.
pub const SKILL_WIDTH_UNITS: i32 = 50;
/// Its height in the normal and the compact layout, in units.
pub const SKILL_HEIGHT_UNITS: i32 = 18;
pub const SKILL_HEIGHT_COMPACT_UNITS: i32 = 24;
/// It sits nine units below its anchor.
pub const SKILL_GAP_UNITS: i32 = 9;

/// Where the skill tooltip sits and when it fades by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTipPlace {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub fade_until: u32,
}

/// Places the skill tooltip at a control's screen point.
pub fn skill_tip_place(
    anchor: (i32, i32),
    compact: bool,
    unit: i32,
    fade_at: u32,
) -> SkillTipPlace {
    let width = (unit & 1).wrapping_add(unit.wrapping_mul(SKILL_WIDTH_UNITS));
    let height = unit.wrapping_mul(if compact {
        SKILL_HEIGHT_COMPACT_UNITS
    } else {
        SKILL_HEIGHT_UNITS
    });
    let y = anchor.1.wrapping_add(unit.wrapping_mul(SKILL_GAP_UNITS));
    SkillTipPlace {
        x: anchor.0,
        y,
        width,
        height,
        fade_until: fade_at.wrapping_add(HIDE_AFTER_MS.wrapping_mul(3)),
    }
}

/// The skill tooltip's draw: its title, then its body box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTipDraw {
    pub title_at: [i32; 2],
    pub body: [i32; 4],
}

/// Draws the skill tooltip's title at two by five units and its body at
/// two by six units, as wide as the form less four units and as tall as
/// the form.
pub fn skill_tip_draw(place: SkillTipPlace, unit: i32) -> SkillTipDraw {
    SkillTipDraw {
        title_at: [
            unit.wrapping_mul(SKILL_TITLE_UNITS[0]),
            unit.wrapping_mul(SKILL_TITLE_UNITS[1]),
        ],
        body: [
            unit.wrapping_mul(SKILL_BODY_UNITS[0]),
            unit.wrapping_mul(SKILL_BODY_UNITS[1]),
            place.width.wrapping_sub(unit.wrapping_mul(4)),
            place.height,
        ],
    }
}

/// The skill tooltip's table cell: the original indexes a 32-byte row by
/// four-byte steps, i.e. two cells per 64-bit table entry. The mapping of
/// odd columns is not established, so only even column indices are read.
pub fn skill_tip_cell(row: u32, column: u32) -> usize {
    (row as usize) * 4 + (column as usize) / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_show_and_hide_fade() {
        let mut t = ToolTipState::default();
        t.show(1000);
        assert_eq!((t.state, t.fade_at, t.hide_at), (1, 1256, 3756));
        assert!(t.showing);
        assert_eq!(background_alpha(t.animate(1128).alpha), 76);
        assert_eq!(background_alpha(t.animate(1256).alpha), 153);
        assert_eq!(t.state, 0);
        // Fading out from idle starts at now + 256.
        assert!(t.hide(2000));
        assert_eq!((t.state, t.fade_at), (2, 2256));
        assert_eq!(background_alpha(t.animate(2128).alpha), 76);
        assert!(t.animate(2256).close);
        assert_eq!(t.state, 0);
    }

    #[test]
    fn showing_while_fading_out_reverses_the_fade() {
        let mut t = ToolTipState::default();
        t.show(100);
        t.animate(356);
        t.hide(400);
        // Reversal: (400·2 + 256) − 656 = 400.
        t.show(500);
        assert_eq!(t.fade_at, 600);
        assert_eq!(t.state, 1);
    }

    #[test]
    fn the_tooltip_is_kept_on_the_screen() {
        // 40 wide text, 12 high line: 56 by 20.
        let l = layout(40, 12, (200, 100), (640, 480));
        assert_eq!((l.width, l.height), (56, 20));
        assert_eq!((l.x, l.y), (172, 120));
        assert_eq!(l.clip_width, 55);
        // Near the right edge it is right-aligned two pixels in.
        let l = layout(40, 12, (620, 100), (640, 480));
        assert_eq!(l.x, 582);
        // Near the bottom it goes above the pointer.
        let l = layout(40, 12, (200, 460), (640, 480));
        assert_eq!(l.y, 440);
        // A pointer left of the screen sits at zero.
        assert_eq!(layout(40, 12, (0, 10), (640, 480)).x, 0);
    }

    #[test]
    fn the_draw_pads_and_fades() {
        let l = layout(40, 12, (200, 100), (640, 480));
        let d = draw(l, 0.5);
        assert_eq!(d.background_alpha, 76);
        assert_eq!(d.text_at, [8, 16]);
        assert_eq!(background_alpha(1.0), 153);
        assert_eq!(background_alpha(0.0), 0);
    }

    #[test]
    fn the_skill_tooltip_is_placed_and_drawn() {
        let p = skill_tip_place((100, 50), false, 4, 1000);
        assert_eq!((p.x, p.y, p.width, p.height), (100, 86, 200, 72));
        assert_eq!(p.fade_until, 1000 + 7500);
        let d = skill_tip_draw(p, 4);
        assert_eq!(d.title_at, [8, 20]);
        assert_eq!(d.body, [8, 24, 184, 72]);
        // The odd unit adds one pixel to the width.
        assert_eq!(skill_tip_place((0, 0), true, 5, 0).width, 251);
        assert_eq!(skill_tip_place((0, 0), true, 4, 0).height, 96);
        assert_eq!(skill_tip_cell(2, 6), 11);
    }
}
