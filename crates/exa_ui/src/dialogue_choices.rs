//! The reply/topic choice list shown during a conversation.
//!
//! The panel is 400 by 120 units, sits eight units above the bottom of its
//! parent, and animates its height when the list changes: the height eases
//! from its base to the target over 500 clock units. The hit test answers
//! the entry number (from 1) under the cursor, the band of an entry being
//! its wrapped line count times four units plus one. A left click picks the
//! hovered entry; the dialogue unit then marks it and holds it for one
//! second, during which the colours' alpha fades out and the entries are
//! cleared at the end. Rows are drawn as "N." and the wrapped text beside it.
//!
//! Evidence: the panel's constructor, hit test, input and draw routines
//! (`docs/UI.md`, dialogue).

use crate::form::{cvtss2si, cvtss2si64, ease};

/// The panel's size, in GUI units.
pub const PANEL_WIDTH: i32 = 400;
pub const PANEL_HEIGHT: i32 = 120;
/// Its distance above the parent's bottom, in units.
pub const BOTTOM_GAP_UNITS: i32 = 8;
/// The height animation's length, clock units.
pub const ANIM_MS: u32 = 500;
/// A picked entry stays highlighted this long, clock units.
pub const PICK_HOLD_MS: u32 = 1000;
/// The box's alpha before the global interface alpha scales it.
pub const BOX_ALPHA: f32 = 160.0;
/// The box's skin index.
pub const BOX_SKIN: i32 = 2;
/// What follows the entry's number.
pub const NUMBER_SUFFIX: &str = ".";

/// A choice entry as the panel shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub text: String,
    /// The wrapped line count.
    pub lines: i32,
}

/// The panel's constructor values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoicesForm {
    pub width: i32,
    pub height: i32,
    pub form_flags: u32,
    pub visible: bool,
    pub input: bool,
}

pub fn choices_form() -> ChoicesForm {
    ChoicesForm {
        width: PANEL_WIDTH,
        height: PANEL_HEIGHT,
        form_flags: 0x80800,
        visible: true,
        input: false,
    }
}

/// The palette colours (`0xAABBGGRR`-style words: the alpha is the top byte)
/// the draw reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoicePalette {
    /// The highlighted entry's colour; its alpha is raised by half again.
    pub highlight: u32,
    /// An ordinary entry's colour while no pick is held.
    pub normal: u32,
    /// An ordinary entry's colour while a pick is held and fading.
    pub normal_while_picked: u32,
}

/// The two text colours one frame draws with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceColours {
    pub highlight: u32,
    pub normal: u32,
}

/// The panel's state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChoicesView {
    pub entries: Vec<Entry>,
    /// The height the animation heads for.
    pub target: i32,
    /// The height the animation started from.
    pub base: i32,
    /// The height drawn.
    pub height: i32,
    /// The clock the height animation ends at.
    pub anim_end: u32,
    /// The panel's y in its parent.
    pub y: i32,
    /// The hit test's last answer: the entry number from 1 (0: none).
    pub hover: i32,
    /// The picked entry's number from 1 (0: none).
    pub picked: i32,
    /// The clock the pick's hold ends at.
    pub pick_end: u32,
}

/// What the input handler did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceInput {
    /// The entry the dialogue unit must select (0-based).
    pub select: Option<usize>,
    /// The answer to the caller.
    pub answer: i32,
}

impl ChoicesView {
    /// Replaces the entries and starts the height animation.
    pub fn set_entries(&mut self, entries: Vec<Entry>, target: i32, now: u32) {
        self.entries = entries;
        self.target = target;
        self.anim_end = now.wrapping_add(ANIM_MS);
    }

    /// The hit test. `base` is the form class's answer (-1: nothing), and
    /// `self_hit` whether the form hit was this panel and not a child.
    /// Below the first unit an entry's band is `lines·4 + 1` units tall;
    /// the answer is the entry's number from 1.
    pub fn hit(&self, base: i32, self_hit: bool, local_y: i32, unit: i32) -> i32 {
        if !self_hit || base != -1 || local_y <= unit {
            return base;
        }
        let mut y = unit;
        for (i, e) in self.entries.iter().enumerate() {
            y = y.wrapping_add(e.lines.wrapping_mul(4).wrapping_add(1).wrapping_mul(unit));
            if local_y < y {
                return i as i32 + 1;
            }
        }
        base
    }

    /// The input. With nothing hovered the form class answers (`base_answer`).
    /// A left click carrying `0x2000` selects the hovered entry and answers 3.
    pub fn input(&self, mask: u16, base_answer: i32) -> ChoiceInput {
        if self.hover < 1 {
            return ChoiceInput {
                select: None,
                answer: base_answer,
            };
        }
        if mask & 1 != 0 && mask & 0x2000 != 0 {
            return ChoiceInput {
                select: Some((self.hover - 1) as usize),
                answer: 3,
            };
        }
        ChoiceInput {
            select: None,
            answer: 0,
        }
    }

    /// What the dialogue unit's selection does to the panel: the picked
    /// number is the index plus one, held for one second.
    pub fn pick(&mut self, index: usize, now: u32) {
        self.picked = index as i32 + 1;
        self.pick_end = now.wrapping_add(PICK_HOLD_MS);
    }

    /// The height one frame draws; returns the panel's y once the height
    /// differs from its base. Past the animation's end the target is taken
    /// and becomes the base; before, the base moves toward it by the eased
    /// share (the base is kept).
    pub fn step_height(&mut self, now: u32, parent_height: i32, unit: i32) -> i32 {
        if self.target != self.base {
            if self.anim_end < now {
                self.height = self.target;
                self.base = self.target;
            } else {
                let left = i64::from(self.anim_end) - i64::from(now);
                let ramp = 1.0 - left as f64 * 0.002;
                let e = ease(ramp as f32);
                self.height = cvtss2si(
                    (i64::from(self.target) - i64::from(self.base)) as f32 * e + self.base as f32,
                );
            }
            self.y = parent_height
                .wrapping_sub(self.height)
                .wrapping_add(unit.wrapping_mul(-BOTTOM_GAP_UNITS));
        }
        self.y
    }

    /// The box's alpha under the global interface alpha.
    pub fn box_alpha(ui_alpha: f32) -> i32 {
        cvtss2si(BOX_ALPHA * ui_alpha)
    }

    /// The text colours of a frame. While a pick is held, both alphas are
    /// scaled by the eased remaining share, wrapping into a byte as the
    /// original's cast does; when the hold has run out the pick and the
    /// entry list are cleared.
    pub fn colours(&mut self, palette: &ChoicePalette, now: u32) -> ChoiceColours {
        let alpha = (palette.highlight >> 24) as u64;
        let mut highlight =
            (palette.highlight & 0x00ff_ffff) | ((((alpha * 3) >> 1) as u8 as u32) << 24);
        if self.picked < 1 {
            return ChoiceColours {
                highlight,
                normal: palette.normal,
            };
        }
        if self.pick_end <= now {
            self.picked = 0;
            self.entries.clear();
        }
        let left = i64::from(self.pick_end) - i64::from(now);
        let share = ease((left as f64 * 0.001) as f32);
        let scale = |colour: u32| {
            let a = cvtss2si64((colour >> 24) as f32 * share) as u8 as u32;
            (colour & 0x00ff_ffff) | (a << 24)
        };
        highlight = scale(highlight);
        ChoiceColours {
            highlight,
            normal: scale(palette.normal_while_picked),
        }
    }

    /// The draw's rows: the entry number at `(unit, y)` and the wrapped body
    /// beside it. The first row is always drawn; drawing stops once the
    /// space left under a row is less than one text line.
    pub fn rows(&self, width: i32, unit: i32, font: &FontMetrics, now: u32) -> Vec<ChoiceRow> {
        let mut out = Vec::new();
        if self.entries.is_empty() {
            return out;
        }
        let mut y = font.line_height.wrapping_add(unit);
        let mut remaining = self.height.wrapping_sub(y).wrapping_add(font.line_height);
        for (i, e) in self.entries.iter().enumerate() {
            let number = i as i32 + 1;
            let held = self.pick_end > now && self.picked == number;
            let highlighted = self.hover - 1 == i as i32 || held;
            out.push(ChoiceRow {
                number,
                prefix: format!("{number}{NUMBER_SUFFIX}"),
                text: e.text.clone(),
                prefix_at: [unit, y],
                body: [
                    unit.wrapping_mul(6),
                    y.wrapping_sub(font.text_offset),
                    width.wrapping_sub(unit.wrapping_mul(8)),
                    remaining,
                ],
                highlighted,
            });
            y = y
                .wrapping_add(font.line_height.wrapping_mul(e.lines))
                .wrapping_add(unit);
            remaining = self.height.wrapping_sub(y).wrapping_add(font.line_height);
            if font.line_height > remaining {
                break;
            }
        }
        out
    }
}

/// The two font values the draw reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontMetrics {
    /// The text line's height.
    pub line_height: i32,
    /// The distance the wrapped body's top sits above its first baseline.
    pub text_offset: i32,
}

/// One drawn choice row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceRow {
    pub number: i32,
    /// The entry's number and its suffix.
    pub prefix: String,
    pub text: String,
    pub prefix_at: [i32; 2],
    /// The wrapped body's box: x, y, width and the height left.
    pub body: [i32; 4],
    pub highlighted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> ChoicesView {
        ChoicesView {
            entries: vec![
                Entry {
                    text: "One".into(),
                    lines: 1,
                },
                Entry {
                    text: "Two".into(),
                    lines: 2,
                },
            ],
            target: 120,
            base: 120,
            height: 120,
            ..Default::default()
        }
    }

    const PALETTE: ChoicePalette = ChoicePalette {
        highlight: 0x4000_80ff,
        normal: 0xdc10_2030,
        normal_while_picked: 0x8000_0001,
    };

    #[test]
    fn the_hit_test_walks_the_entry_bands() {
        let v = view();
        assert_eq!(v.hit(-1, true, 2, 4), -1);
        // The first band runs from 4 to 4 + 20.
        assert_eq!(v.hit(-1, true, 5, 4), 1);
        assert_eq!(v.hit(-1, true, 23, 4), 1);
        // The second starts there and is 36 tall.
        assert_eq!(v.hit(-1, true, 24, 4), 2);
        assert_eq!(v.hit(-1, true, 59, 4), 2);
        assert_eq!(v.hit(-1, true, 60, 4), -1);
        // A child hit or the class's own answer wins.
        assert_eq!(v.hit(-1, false, 24, 4), -1);
        assert_eq!(v.hit(7, true, 24, 4), 7);
        let f = choices_form();
        assert_eq!((f.width, f.height, f.form_flags), (400, 120, 0x80800));
        assert!(f.visible && !f.input);
    }

    #[test]
    fn a_click_selects_the_hovered_entry() {
        let mut v = view();
        v.hover = 2;
        assert_eq!(
            v.input(0x2001, 9),
            ChoiceInput {
                select: Some(1),
                answer: 3
            }
        );
        assert_eq!(
            v.input(0x2000, 9),
            ChoiceInput {
                select: None,
                answer: 0
            }
        );
        v.hover = 0;
        assert_eq!(
            v.input(0x2001, 9),
            ChoiceInput {
                select: None,
                answer: 9
            }
        );
        // The dialogue unit's selection marks the pick.
        v.pick(1, 100);
        assert_eq!((v.picked, v.pick_end), (2, 1100));
    }

    #[test]
    fn the_height_eases_from_its_base_and_sits_above_the_bottom() {
        let mut v = view();
        v.base = 80;
        v.target = 120;
        v.anim_end = 1000;
        // At the end the ramp is 1 and the height is the target.
        assert_eq!(v.step_height(1000, 600, 4), 600 - 120 - 32);
        assert_eq!(v.height, 120);
        // 250 before the end the ramp is 0.5: the share is 0.5 and the
        // height 100. The base stays put while the animation runs.
        v.base = 80;
        v.target = 120;
        v.anim_end = 1500;
        assert_eq!(v.step_height(1250, 600, 4), 600 - 100 - 32);
        assert_eq!(v.base, 80);
        // Past the end the target is taken and becomes the base.
        v.anim_end = 1000;
        assert_eq!(v.step_height(1500, 600, 4), 600 - 120 - 32);
        assert_eq!(v.base, 120);
        // Level heights leave the panel where it is.
        assert_eq!(v.step_height(1600, 700, 4), 600 - 120 - 32);
        assert_eq!(ChoicesView::box_alpha(1.0), 160);
        assert_eq!(ChoicesView::box_alpha(0.5), 80);
    }

    #[test]
    fn colours_fade_with_the_pick_and_the_hold_clears_the_list() {
        let mut v = view();
        // No pick: the palette, the highlight alpha raised by half again.
        let c = v.colours(&PALETTE, 0);
        assert_eq!(c.highlight, 0x6000_80ff);
        assert_eq!(c.normal, 0xdc10_2030);
        // Wrapping into a byte: 0xdc * 3 >> 1 = 330, low byte 74.
        let wrap = ChoicePalette {
            highlight: 0xdc00_0000,
            ..PALETTE
        };
        assert_eq!(v.colours(&wrap, 0).highlight >> 24, 74);
        // Half the hold left: the share is ease(0.5) = 0.5.
        v.pick(0, 0);
        let c = v.colours(&PALETTE, 500);
        assert_eq!(c.highlight >> 24, 0x30);
        assert_eq!(c.normal, 0x4000_0001);
        assert_eq!(v.entries.len(), 2);
        // The hold over: the pick and the entries go, and the frame still
        // draws with the ease of the small negative share (0.032).
        let c = v.colours(&PALETTE, 1100);
        assert_eq!(v.picked, 0);
        assert!(v.entries.is_empty());
        assert_eq!(c.highlight >> 24, 3);
    }

    #[test]
    fn the_rows_are_numbered_and_stop_when_a_line_no_longer_fits() {
        let mut v = view();
        v.hover = 2;
        let font = FontMetrics {
            line_height: 20,
            text_offset: 12,
        };
        let rows = v.rows(400, 4, &font, 0);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].prefix, "1.");
        assert_eq!(rows[0].prefix_at, [4, 24]);
        // Remaining: (120 - 24) + 20.
        assert_eq!(rows[0].body, [24, 12, 368, 116]);
        assert!(!rows[0].highlighted && rows[1].highlighted);
        // The second starts a line and a unit further down.
        assert_eq!(rows[1].prefix_at, [4, 48]);
        assert_eq!(rows[1].body[3], 120 - 48 + 20);
        // A short box still draws the first row, then stops.
        v.height = 40;
        let rows = v.rows(400, 4, &font, 0);
        assert_eq!(rows.len(), 1);
        // A held pick highlights its row only until its end.
        v.height = 120;
        v.hover = 0;
        v.pick(0, 0);
        assert!(v.rows(400, 4, &font, 500)[0].highlighted);
        assert!(!v.rows(400, 4, &font, 1000)[0].highlighted);
    }
}
