//! The technique selector: one skill's techniques, their descriptions, the
//! experience a technique costs and the confirmation before forgetting one.
//!
//! Evidence: the technique selector's routines in the retail image. A level
//! is 5 000 experience. Assigning a technique adds at most a level's worth
//! from the character's pool; with the weighted flag the gain is
//! `pool · 4 / (4 + Σ level)`, capped at 5 000, and the pool drops by
//! `gain · divisor / 4`. Forgetting returns `level · 5 000 − spent`, capped
//! by the unspent pool. The form is 332 × 112 pixels, lists eight
//! techniques and draws the selected one's name centred at 16 units and its
//! description at 20 units above the bottom. Words 9 and 10 mean "forget".

/// Experience in one level.
pub const LEVEL_XP: u16 = 5_000;
/// The form's width in pixels.
pub const FORM_WIDTH: i32 = 0x14c;
/// The form's height in pixels.
pub const FORM_HEIGHT: i32 = 0x70;
/// The rows (techniques) the form lists.
pub const ROWS: usize = 8;
/// The technique slots a skill tracks.
pub const SLOTS: usize = 6;
/// The assignment bytes: slots one through five.
pub const ASSIGNED: usize = 5;
/// The name's y in units.
pub const NAME_Y_UNITS: i64 = 16;
/// The description sits this many units above the bottom.
pub const DESCRIPTION_BOTTOM_UNITS: i64 = 20;
/// The confirmation text sits at 18 units and wraps over 20.
pub const CONFIRM_Y_UNITS: i64 = 18;
pub const CONFIRM_HEIGHT_UNITS: i64 = 20;
/// The most techniques a character may hold before extras are locked.
pub const TECHNIQUE_LIMIT: i32 = 15;
/// The two words that mean "forget".
pub const FORGET_WORDS: [i32; 2] = [9, 10];
/// The one row word the base handler keeps.
pub const BASE_WORD: i32 = 9;
/// The form's mode value that lights the renderer's 0x110 bits.
pub const MODE_SELECT: i32 = 1;
/// The confirmation state.
pub const STATE_CONFIRM: i32 = 2;
/// The text of the confirmation.
pub const CONFIRM_TEXT: &str = "Are you sure you want to forget this technique?";

/// One skill's technique training: six experience words and the five
/// assignment bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkillTraining {
    /// The experience of each slot; 5 000 is a level.
    pub xp: [u16; SLOTS],
    /// The technique id in each learned slot, 0 = none.
    pub assigned: [u8; ASSIGNED],
}

/// A character's training: eight skills and the unspent pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechniqueTraining {
    pub skills: [SkillTraining; ROWS],
    /// The unspent experience pool.
    pub pool: i32,
    /// Assignments truncate a slot to whole levels first.
    pub truncate: bool,
    /// The weighted gain.
    pub weighted: bool,
    /// Every learned row is shown as learned.
    pub all_learned: bool,
}

impl Default for TechniqueTraining {
    fn default() -> Self {
        Self {
            skills: [SkillTraining::default(); ROWS],
            pool: 0,
            truncate: false,
            weighted: false,
            all_learned: false,
        }
    }
}

impl TechniqueTraining {
    /// The sum of every skill's six experience words.
    pub fn spent(&self) -> i32 {
        self.skills
            .iter()
            .flat_map(|skill| skill.xp)
            .map(i32::from)
            .sum()
    }

    /// The unspent experience: the pool less the spent words.
    pub fn available(&self) -> i32 {
        self.pool - self.spent()
    }
}

/// The selector's state: the technique it highlights and the level slot it
/// edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TechniqueSelector {
    /// The technique id last picked; below 1 means none.
    pub index: i32,
    /// The level slot, one through five.
    pub slot: usize,
    /// The skill group (the form's `param_3` less one).
    pub group: usize,
    /// The shown state: 0 the list, 2 the confirmation.
    pub state: i32,
    /// The style's low word, a row code; 9 and 10 mean "forget".
    pub code: i32,
    /// The hidden flag.
    pub hidden: bool,
}

/// What picking a technique did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechniqueChange {
    pub training: TechniqueTraining,
    /// The experience added (or taken, when negative).
    pub moved: i32,
    /// The selector stores this index.
    pub index: i32,
    /// The character view needs a refresh.
    pub refresh: bool,
}

/// Assigns a technique (a non-negative id) or forgets one (a negative id).
/// The gain and the refund are the original's arithmetic.
pub fn assign_technique(
    training: TechniqueTraining,
    selector: TechniqueSelector,
    technique: i32,
) -> TechniqueChange {
    let mut training = training;
    let mut moved = 0;
    if technique == selector.index {
        return TechniqueChange {
            training,
            moved,
            index: selector.index,
            refresh: false,
        };
    }
    let slot = selector.slot.clamp(1, ASSIGNED) - 1;
    let group = selector.group.min(ROWS - 1);
    let available = training.available();
    let pool = training.pool;
    let truncate = training.truncate;
    let weighted = training.weighted;
    let skill = &mut training.skills[group];
    if technique < 0 {
        // The refund: the slot's level, less what it already holds, capped
        // by the unspent pool. The unspent pool counts every skill.
        let level = selector.slot as i32;
        let refund = level * i32::from(LEVEL_XP) - i32::from(skill.xp[slot]);
        let add = refund.min(available);
        skill.xp[slot] = skill.xp[slot].wrapping_add(add as u16);
        moved = add;
    } else {
        if truncate && u32::from(skill.xp[slot]) / u32::from(LEVEL_XP) < selector.slot as u32 {
            skill.xp[slot] -= skill.xp[slot] % LEVEL_XP;
        }
        skill.assigned[slot] = technique as u8;
        if pool > 0 {
            let add = if weighted {
                let divisor = 4 + skill
                    .xp
                    .iter()
                    .map(|&xp| i32::from(xp) / i32::from(LEVEL_XP))
                    .sum::<i32>();
                let gain = ((i64::from(pool) << 2) / i64::from(divisor)) as i32;
                let gain = gain.clamp(0, i32::from(LEVEL_XP));
                let cost = ((i64::from(gain) * i64::from(divisor)
                    + ((i64::from(gain) * i64::from(divisor)) >> 63 & 3))
                    >> 2) as i32;
                training.pool -= cost;
                gain
            } else {
                available.clamp(0, i32::from(LEVEL_XP))
            };
            if selector.index < 1 {
                skill.xp[slot] = skill.xp[slot].wrapping_add(add as u16);
                moved = add;
            }
        }
    }
    TechniqueChange {
        training,
        moved,
        index: technique,
        refresh: true,
    }
}

/// What a row of the refreshed tree is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A learned technique.
    Technique,
    /// A locked row.
    Locked,
    /// The row to learn next.
    Next { extra: bool },
    /// An empty row.
    Empty,
}

/// One refreshed row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechniqueRow {
    pub kind: RowKind,
    pub technique: u8,
    /// A learned technique above the skill's level.
    pub locked: bool,
    /// The skill's colour slot (the original's `skill + 2`).
    pub colour: i32,
}

/// One skill's refreshed rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillRows {
    pub rows: [TechniqueRow; SLOTS],
    /// The level as stored (experience / 5 000).
    pub level: i32,
    /// The skill holds more techniques than its level.
    pub more: bool,
}

/// The refreshed tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechniqueTree {
    pub skills: Vec<SkillRows>,
    /// Every assigned technique across the skills.
    pub total: i32,
}

impl TechniqueTree {
    /// The row states of one skill.
    pub fn skill(&self, index: usize) -> Option<&SkillRows> {
        self.skills.get(index)
    }
}

/// Rebuilds the technique rows of every skill, as the form does after an
/// assignment or a change of character.
pub fn refresh_tree(training: &TechniqueTraining) -> TechniqueTree {
    let mut skills = Vec::with_capacity(ROWS);
    let mut total = 0;
    for (skill, source) in training.skills.iter().enumerate() {
        let xp = source.xp[0];
        let level = i32::from(xp) / i32::from(LEVEL_XP);
        let mut rows = [TechniqueRow {
            kind: RowKind::Empty,
            technique: 0,
            locked: false,
            colour: skill as i32 + 2,
        }; SLOTS];
        let mut count = 0;
        for (slot, row) in rows.iter_mut().enumerate().take(ASSIGNED) {
            let technique = source.assigned[slot];
            if technique != 0 {
                count += 1;
                row.kind = RowKind::Technique;
                row.technique = technique;
                // The original's word 2 marks a technique above the learned
                // level.
                row.locked = level - 1 < slot as i32;
            }
        }
        total += count;
        let more = level < count;
        skills.push(SkillRows { rows, level, more });
    }
    let any_more = skills.iter().any(|skill| skill.more);
    for skill in &mut skills {
        if !training.all_learned {
            if total < TECHNIQUE_LIMIT {
                let mut first_empty = None;
                for slot in 0..=skill.level.min(SLOTS as i32 - 1) as usize {
                    if skill.rows[slot].kind == RowKind::Empty {
                        first_empty = Some(slot);
                        break;
                    }
                }
                if let Some(slot) = first_empty {
                    skill.rows[slot].kind = RowKind::Next { extra: any_more };
                }
                let from = (skill.level + 2).max(0) as usize;
                for slot in from.min(SLOTS)..SLOTS {
                    skill.rows[slot].kind = RowKind::Locked;
                }
            } else if skill.level < SLOTS as i32 {
                let from = (skill.level - 1).max(0) as usize;
                for slot in from..(SLOTS - 1) {
                    if skill.rows[slot].kind == RowKind::Empty {
                        skill.rows[slot].kind = RowKind::Locked;
                    }
                }
            }
        } else {
            let upto = (skill.level + 1).clamp(0, SLOTS as i32) as usize;
            for slot in 0..upto {
                skill.rows[slot].kind = RowKind::Locked;
            }
        }
    }
    TechniqueTree { skills, total }
}

/// Where the name and description go, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechniqueText {
    pub name_x: i32,
    pub name_y: i32,
    pub description_x: i32,
    pub description_y: i32,
    pub description_width: i32,
}

/// The text places for a name `name_width` pixels wide.
pub fn text_places(form_width: i32, form_height: i32, unit: i32, name_width: i32) -> TechniqueText {
    TechniqueText {
        name_x: (form_width - name_width) / 2,
        name_y: unit * NAME_Y_UNITS as i32,
        description_x: unit,
        description_y: form_height - unit * DESCRIPTION_BOTTOM_UNITS as i32,
        description_width: form_width - 2 * unit,
    }
}

/// What the key handler did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectorKey {
    /// The form returns this; 0x40 closes it.
    pub result: u32,
    /// The form requests its close.
    pub close: bool,
    /// The key went to the base handler.
    pub base: bool,
}

/// The key guard: with no index, no page, or a hidden word 9/10 the base
/// handles the key; otherwise the form closes with 0x40.
pub fn selector_key(index: i32, page: i32, code: i32, hidden: bool) -> SelectorKey {
    if index < 1 || page < 2 {
        return SelectorKey {
            result: 0,
            close: false,
            base: true,
        };
    }
    if code == BASE_WORD && !hidden {
        return SelectorKey {
            result: 0,
            close: false,
            base: true,
        };
    }
    SelectorKey {
        result: 0x40,
        close: true,
        base: false,
    }
}

/// Clamps the picked index to the eight rows and sets the mode's style bits.
pub fn clamp_index(raw: u32, mode: i32, style: u32) -> (i32, u32) {
    let mut index = (raw & 0xffff) as i32;
    if index > ROWS as i32 {
        index = 0;
    }
    let mut style = style;
    if mode == MODE_SELECT {
        if index < 1 {
            style &= !0x110;
        } else {
            style |= 0x110;
        }
    }
    (index, style)
}

/// The technique the close picks: the index, or "forget" for words 9 and
/// 10, or nothing when no row is picked.
pub fn close_pick(index: i32, word: i32) -> Option<i32> {
    if index < 1 {
        if FORGET_WORDS.contains(&word) {
            Some(-1)
        } else {
            None
        }
    } else {
        Some(index)
    }
}

/// The paint's steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorDraw {
    Base,
    Skin,
    Confirmation,
    Technique { row: usize },
}

/// The paint's order: the base, the skin at unit 2 and either the
/// confirmation or the picked technique's name and description.
pub fn draw_order(state: i32, picked: i32) -> Vec<SelectorDraw> {
    let mut draw = vec![SelectorDraw::Base, SelectorDraw::Skin];
    if state == STATE_CONFIRM {
        draw.push(SelectorDraw::Confirmation);
    } else if (0..ROWS as i32).contains(&picked) {
        draw.push(SelectorDraw::Technique {
            row: picked as usize,
        });
    }
    draw
}

#[cfg(test)]
mod tests {
    use super::*;

    fn training() -> TechniqueTraining {
        let mut t = TechniqueTraining {
            pool: 12_000,
            ..TechniqueTraining::default()
        };
        t.skills[0].xp = [15_000, 10_000, 0, 0, 0, 0];
        t.skills[0].assigned = [1, 2, 0, 0, 0];
        t.skills[1].xp = [10_000, 0, 0, 0, 0, 0];
        t
    }

    #[test]
    fn spent_and_available_count_the_words_and_the_pool() {
        let t = training();
        assert_eq!(t.spent(), 35_000);
        assert_eq!(t.available(), -23_000);
        let mut t = t;
        t.pool = 40_000;
        assert_eq!(t.available(), 5_000);
    }

    #[test]
    fn an_assignment_adds_at_most_a_level_from_the_pool() {
        let mut t = training();
        t.pool = 40_000;
        let selector = TechniqueSelector {
            index: -1,
            slot: 3,
            ..TechniqueSelector::default()
        };
        let change = assign_technique(t, selector, 7);
        assert_eq!(change.training.skills[0].assigned[2], 7);
        // The unspent pool is 15 000; a level is added.
        assert_eq!(change.moved, 5_000);
        assert_eq!(change.training.skills[0].xp[2], 5_000);
        assert_eq!(change.training.pool, 40_000);
        assert_eq!(change.index, 7);
        assert!(change.refresh);

        // A pool of 1 000 adds exactly that.
        let change = assign_technique(t, selector, 8);
        assert_eq!(change.training.skills[0].xp[2], 5_000);
        let mut t2 = t;
        t2.pool = 36_000;
        let change = assign_technique(t2, selector, 8);
        assert_eq!(change.moved, 1_000);
        assert_eq!(change.training.skills[0].xp[2], 1_000);
    }

    #[test]
    fn the_weighted_gain_divides_by_four_plus_the_levels() {
        let mut t = training();
        t.pool = 40_000;
        t.weighted = true;
        let selector = TechniqueSelector {
            index: -1,
            slot: 3,
            ..TechniqueSelector::default()
        };
        // Levels: 3 + 2 + 0 ... = 5, divisor 9. 40000*4/9 = 17777,
        // capped at 5000; the pool drops by 5000*9/4 = 11250.
        let change = assign_technique(t, selector, 7);
        assert_eq!(change.moved, 5_000);
        assert_eq!(change.training.pool, 28_750);
        assert_eq!(change.training.skills[0].xp[2], 5_000);
    }

    #[test]
    fn a_forget_returns_the_slot_less_what_it_holds() {
        let mut t = training();
        t.pool = 40_000;
        // Slot 4's refund is 20 000 - 15 000 = 5 000, capped by the
        // unspent 5 000 (the slot's own experience is 0).
        let selector = TechniqueSelector {
            index: 1,
            slot: 4,
            ..TechniqueSelector::default()
        };
        let change = assign_technique(t, selector, -1);
        assert_eq!(change.moved, 5_000);
        assert_eq!(change.training.skills[0].xp[3], 5_000);
        assert_eq!(change.index, -1);
        // A short unspent pool caps it.
        let mut t = change.training;
        t.pool = 41_000;
        let change = assign_technique(t, selector, -1);
        assert_eq!(change.moved, 1_000);
        assert_eq!(change.training.skills[0].xp[3], 6_000);
    }

    #[test]
    fn the_truncation_flag_rounds_a_slot_down_first() {
        let mut t = training();
        t.skills[0].xp[2] = 4_000;
        t.truncate = true;
        t.pool = 100_000;
        let selector = TechniqueSelector {
            index: -1,
            slot: 3,
            ..TechniqueSelector::default()
        };
        let change = assign_technique(t, selector, 9);
        // 4000 / 5000 < 3, so 4000 - 4000 % 5000 = 0; then a level.
        assert_eq!(change.training.skills[0].xp[2], 5_000);
    }

    #[test]
    fn the_same_technique_changes_nothing() {
        let t = training();
        let selector = TechniqueSelector {
            index: 4,
            slot: 1,
            ..TechniqueSelector::default()
        };
        let change = assign_technique(t, selector, 4);
        assert_eq!(change.training, t);
        assert_eq!(change.moved, 0);
        assert!(!change.refresh);
    }

    #[test]
    fn the_refresh_marks_learned_locked_and_next_rows() {
        let t = training();
        let tree = refresh_tree(&t);
        assert_eq!(tree.total, 2);
        let skill = tree.skill(0).unwrap();
        assert_eq!(skill.level, 3);
        assert_eq!(skill.rows[0].kind, RowKind::Technique);
        assert_eq!(skill.rows[1].kind, RowKind::Technique);
        // Row 2 is empty and becomes the next one to learn.
        assert_eq!(skill.rows[2].kind, RowKind::Next { extra: false });
        // Rows from level+2 (5) on are locked.
        assert_eq!(skill.rows[5].kind, RowKind::Locked);
        // The second skill has no assignment at all.
        let skill = tree.skill(1).unwrap();
        assert_eq!(skill.level, 2);
        assert_eq!(skill.rows[0].kind, RowKind::Next { extra: false });
        assert_eq!(skill.rows[4].kind, RowKind::Locked);
    }

    #[test]
    fn fifteen_techniques_lock_the_rows_from_the_learned_level() {
        let mut t = training();
        for skill in &mut t.skills {
            skill.xp = [25_000; SLOTS];
            skill.assigned = [1, 2, 3, 4, 5];
        }
        let tree = refresh_tree(&t);
        assert_eq!(tree.total, 40);
        let skill = tree.skill(0).unwrap();
        assert_eq!(skill.rows[0].kind, RowKind::Technique);
        assert_eq!(skill.rows[4].kind, RowKind::Technique);
        // Past the level (5), every row is already filled, so nothing is
        // emptied; row 5 stays empty.
        assert_eq!(skill.rows[5].kind, RowKind::Empty);
    }

    #[test]
    fn the_tree_text_and_paint_places_are_pinned() {
        let places = text_places(FORM_WIDTH, FORM_HEIGHT, 4, 100);
        assert_eq!(places.name_x, 116);
        assert_eq!(places.name_y, 64);
        assert_eq!(places.description_x, 4);
        assert_eq!(places.description_y, 32);
        assert_eq!(places.description_width, 324);
        assert_eq!(
            draw_order(0, 3),
            vec![
                SelectorDraw::Base,
                SelectorDraw::Skin,
                SelectorDraw::Technique { row: 3 }
            ]
        );
        assert_eq!(
            draw_order(2, 3),
            vec![
                SelectorDraw::Base,
                SelectorDraw::Skin,
                SelectorDraw::Confirmation
            ]
        );
        assert_eq!(draw_order(0, 9).len(), 2);
        assert_eq!(CONFIRM_TEXT.len(), 47);
    }

    #[test]
    fn keys_close_except_for_the_base_cases() {
        assert!(selector_key(0, 5, 1, false).base);
        assert!(selector_key(3, 1, 1, false).base);
        assert!(selector_key(3, 5, 9, false).base);
        assert!(selector_key(3, 5, 10, false).close);
        let closed = selector_key(3, 5, 3, false);
        assert_eq!(closed.result, 0x40);
        assert!(closed.close);
        assert!(!closed.base);
        // The hidden flag keeps word 9 out of the base.
        assert!(selector_key(3, 5, 9, true).close);
    }

    #[test]
    fn the_index_clamps_to_eight_and_lights_the_mode() {
        assert_eq!(clamp_index(3, 0, 0x35c), (3, 0x35c));
        assert_eq!(clamp_index(9, 0, 0x35c), (0, 0x35c));
        assert_eq!(clamp_index(8, 0, 0), (8, 0));
        assert_eq!(clamp_index(0, MODE_SELECT, 0x35c), (0, 0x24c));
        assert_eq!(clamp_index(2, MODE_SELECT, 0), (2, 0x110));
        assert_eq!(clamp_index(0x1_0002, 1, 0), (2, 0x110));
    }

    #[test]
    fn the_close_picks_a_technique_or_forgets() {
        assert_eq!(close_pick(3, 0), Some(3));
        assert_eq!(close_pick(0, 9), Some(-1));
        assert_eq!(close_pick(0, 10), Some(-1));
        assert_eq!(close_pick(0, 1), None);
        assert_eq!(close_pick(-1, 9), Some(-1));
    }
}
