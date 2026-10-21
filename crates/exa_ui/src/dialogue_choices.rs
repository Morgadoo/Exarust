//! The reply/topic choice list shown during a conversation.
//!
//! The panel is 400 by 120 units, sits above the bottom of its parent by
//! eight units, and animates its height when the list changes: the target
//! height eases in over 500 clock units. Each entry starts one unit down
//! and takes its wrapped line count times four units plus one. A click on
//! an entry picks it and makes the panel's dialogue the player's current
//! one. The hovered entry, and the picked one for one second, draw in the
//! highlight colour, whose alpha fades out with the pick.
//!
//! Evidence: the panel's constructor, hit test, input and draw routines
//! (`docs/UI.md`, dialogue).

/// The panel's size, in GUI units.
pub const PANEL_WIDTH: i32 = 400;
pub const PANEL_HEIGHT: i32 = 120;
/// Its distance above the parent's bottom, in units.
pub const BOTTOM_GAP_UNITS: i32 = 8;
/// The animation's length, clock units.
pub const ANIM_MS: u32 = 500;
/// The picked entry stays highlighted this long, clock units.
pub const PICK_HOLD_MS: u32 = 1000;
/// The box's alpha.
pub const BOX_ALPHA: f32 = 160.0;
/// The box's blend/skin index.
pub const BOX_SKIN: i32 = 2;

/// A polynomial ease, `3x² − 2x³`, clamped in effect by the caller's range.
/// [guess: the original's 36-byte helper is not in the pack; its callers
/// pass a 0…1 ramp]
pub fn ease(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}

/// A choice entry as the panel shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub text: String,
    /// The wrapped line count.
    pub lines: i32,
}

/// The panel's state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChoicesView {
    pub entries: Vec<Entry>,
    /// The target height.
    pub target: i32,
    /// The height drawn.
    pub shown: i32,
    /// The clock the height animation started at.
    pub anim_start: u32,
    /// The highlighted entry, from 1 (0: none).
    pub hover: i32,
    /// The picked entry, from 1 (0: none).
    pub picked: i32,
    /// The clock the pick's fade started at (0: no pick).
    pub pick_at: u32,
}

/// What a point inside the panel means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceHit {
    /// The entry's 1-based number; `None` for the panel itself.
    pub entry: Option<i32>,
}

impl ChoicesView {
    /// Replaces the entries and starts the height animation.
    pub fn set_entries(&mut self, entries: Vec<Entry>, target: i32, now: u32) {
        self.entries = entries;
        self.target = target;
        self.anim_start = now;
    }

    /// The hit test: the panel's own rectangle, then an entry whose band
    /// starts one unit down and is `lines·4 + 1` units tall.
    pub fn hit(&self, local: (i32, i32), base: Option<i32>, unit: i32) -> i32 {
        if let Some(entry) = base {
            return entry;
        }
        if local.1 <= unit {
            return -1;
        }
        let mut y = unit;
        for (i, e) in self.entries.iter().enumerate() {
            y = y.wrapping_add(e.lines.wrapping_mul(4).wrapping_add(1).wrapping_mul(unit));
            if local.1 < y {
                return i as i32 + 2;
            }
        }
        -1
    }

    /// The input: a left click carrying `0x2000` picks the hovered entry.
    /// Returns the picked index (if any) and the handler's answer.
    pub fn input(&mut self, hover: i32, mask: u16, now: u32) -> ChoiceInput {
        if hover < 1 {
            return ChoiceInput {
                picked: None,
                answer: 0,
            };
        }
        if mask & 1 != 0 && mask & 0x2000 != 0 {
            let index = (hover - 1) as usize;
            self.picked = hover;
            self.pick_at = now.wrapping_add(PICK_HOLD_MS);
            return ChoiceInput {
                picked: Some(index),
                answer: 3,
            };
        }
        ChoiceInput {
            picked: None,
            answer: 0,
        }
    }

    /// The height one frame draws and the box's y.
    pub fn step_height(&mut self, now: u32, parent_client_bottom: i32, unit: i32) -> i32 {
        if self.target != self.shown {
            if self.anim_start < now {
                self.shown = self.target;
            } else {
                let left = u64::from(self.anim_start) - u64::from(now);
                let ramp = 1.0 - left as f64 * 0.002;
                let e = ease(ramp as f32);
                self.shown = ((i64::from(self.target) - i64::from(self.shown)) as f32 * e
                    + self.shown as f32)
                    .round_ties_even() as i32;
            }
        }
        parent_client_bottom
            .wrapping_sub(self.shown)
            .wrapping_sub(unit.wrapping_mul(BOTTOM_GAP_UNITS))
    }

    /// The colour fade of the picked entry, 0…1, and the highlight alpha
    /// and base alpha after it.
    pub fn pick_fade(&mut self, now: u32) -> (f32, u8, u8, u8, u8) {
        let base = [255u8, 255, 255, 220];
        let highlight = [255u8, 224, 160, 255];
        if self.pick_at > 0 {
            if self.pick_at <= now {
                self.pick_at = 0;
                self.picked = 0;
            }
            let left = u64::from(self.pick_at) - u64::from(now);
            let t = ease((left as f64 * 0.001) as f32);
            return (
                t,
                (f32::from(highlight[3]) * t).round_ties_even() as u8,
                (f32::from(base[3]) * t).round_ties_even() as u8,
                highlight[1],
                highlight[2],
            );
        }
        (1.0, highlight[3], base[3], highlight[1], highlight[2])
    }

    /// The draw's text rows and the "n. " prefixes.
    pub fn rows(
        &self,
        height: i32,
        parent_client_bottom: i32,
        unit: i32,
        line_height: i32,
    ) -> Vec<ChoiceRow> {
        let top = parent_client_bottom - height - unit * BOTTOM_GAP_UNITS;
        let mut y = top + unit;
        let mut remaining = parent_client_bottom - y;
        let mut out = Vec::new();
        for (i, e) in self.entries.iter().enumerate() {
            if line_height > remaining {
                break;
            }
            let picked = self.picked == i as i32 + 1;
            let hovered = self.hover == i as i32 + 1;
            out.push(ChoiceRow {
                number: i as i32 + 1,
                prefix: format!("{}. ", i + 1),
                text: e.text.clone(),
                index_at: [unit, y],
                body: [unit * 6, y - line_height, PANEL_WIDTH - unit * 8, remaining],
                highlighted: hovered || picked,
                picked,
            });
            y = y
                .wrapping_add(line_height.wrapping_mul(e.lines))
                .wrapping_add(unit);
            remaining = parent_client_bottom - y;
        }
        out
    }
}

/// One drawn choice row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceRow {
    pub number: i32,
    /// The "n. " the original prepends to the number's text.
    pub prefix: String,
    pub text: String,
    pub index_at: [i32; 2],
    pub body: [i32; 4],
    pub highlighted: bool,
    pub picked: bool,
}

/// What the input handler did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceInput {
    pub picked: Option<usize>,
    pub answer: i32,
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
            shown: 120,
            ..Default::default()
        }
    }

    #[test]
    fn the_hit_test_walks_the_entry_bands() {
        let v = view();
        assert_eq!(v.hit((10, 2), None, 4), -1);
        assert_eq!(v.hit((10, 8), None, 4), 2);
        // The second band starts at 4 + 20 and is 36 tall.
        assert_eq!(v.hit((10, 24), None, 4), 3);
        assert_eq!(v.hit((10, 61), None, 4), -1);
        assert_eq!(v.hit((10, 8), Some(7), 4), 7);
    }

    #[test]
    fn a_click_picks_the_hovered_entry() {
        let mut v = view();
        assert_eq!(
            v.input(2, 0x2001, 100),
            ChoiceInput {
                picked: Some(1),
                answer: 3
            }
        );
        assert_eq!(v.picked, 2);
        assert_eq!(v.pick_at, 1100);
        assert_eq!(
            v.input(2, 0x2000, 100),
            ChoiceInput {
                picked: None,
                answer: 0
            }
        );
        assert!(v.input(0, 0x2001, 100).picked.is_none());
    }

    #[test]
    fn the_height_eases_and_sits_above_the_bottom() {
        let mut v = view();
        v.shown = 80;
        v.target = 120;
        v.anim_start = 1000;
        // At the start the ramp is 1 and the height is the target.
        assert_eq!(v.step_height(1000, 600, 4), 448);
        v.shown = 80;
        v.anim_start = 1500;
        // 250 earlier the ramp is 0.5: the height is 100.
        assert_eq!(v.step_height(1250, 600, 4), 468);
        v.shown = 80;
        v.anim_start = 1000;
        // Past the start the target is taken at once.
        assert_eq!(v.step_height(1500, 600, 4), 448);
    }

    #[test]
    fn the_rows_are_prefixed_and_stop_at_the_box() {
        let v = view();
        let rows = v.rows(120, 600, 4, 20);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].prefix, "1. ");
        assert_eq!(rows[0].index_at, [4, 452]);
        assert_eq!(rows[0].body, [24, 432, 368, 148]);
        assert_eq!(rows[1].index_at, [4, 476]);
        assert!(v.rows(40, 600, 4, 20).len() <= 2);
    }
}
