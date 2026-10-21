//! The technique selector: the eight techniques one skill offers, their
//! names and descriptions, picking one into a skill slot (with the
//! experience it moves) and the confirmation before forgetting one.
//!
//! Evidence: the technique selector's routines in the retail image, read
//! from the instructions, with the two character-data helpers they call
//! (the technique in training and the forget). The character record holds
//! six skills; each has one 16-bit experience word and five technique
//! bytes (slots 1…5). A level is 5 000 experience. The record's holder
//! keeps the experience pool and two flags: "truncate" rounds a slot's
//! skill down to whole levels before an assignment, "weighted" makes an
//! assignment draw from the pool with `pool · 4 / (4 + Σ levels)` and
//! drop the pool by `gain · divisor / 4`.
//!
//! The form is 332 × 112 pixels. The picked technique's name is centred
//! at 16 units, its description boxed from 18 units down to 20 units
//! above the bottom, the confirmation boxed at 12 units, 20 units high.

/// Experience in one level.
pub const LEVEL_XP: u16 = 5_000;
/// The form's width in pixels.
pub const FORM_WIDTH: i32 = 332;
/// The form's height in pixels.
pub const FORM_HEIGHT: i32 = 112;
/// The techniques a skill offers (the form's rows).
pub const ROWS: usize = 8;
/// The skills a character record holds.
pub const SKILLS: usize = 6;
/// The technique slots of one skill.
pub const SLOTS: usize = 5;
/// The technique-removal pass looks at the first four slots only.
pub const DEPENDANT_SLOTS: usize = 4;
/// The name's y in units.
pub const NAME_Y_UNITS: i32 = 16;
/// The description's y in units.
pub const DESCRIPTION_Y_UNITS: i32 = 18;
/// The description box ends this many units above the form's bottom.
pub const DESCRIPTION_BOTTOM_UNITS: i32 = 20;
/// The confirmation's y and box height in units.
pub const CONFIRM_Y_UNITS: i32 = 12;
pub const CONFIRM_HEIGHT_UNITS: i32 = 20;
/// The most techniques a tree holds before the next slots stop opening.
pub const TECHNIQUE_LIMIT: i32 = 15;
/// The two hovered row codes that mean "forget".
pub const FORGET_WORDS: [i32; 2] = [9, 10];
/// The row code the base handler keeps.
pub const BASE_WORD: i32 = 9;
/// The form's mode while it lists techniques.
pub const MODE_SELECT: i32 = 1;
/// The form's mode while it asks to confirm a forget.
pub const STATE_CONFIRM: i32 = 2;
/// The text of the confirmation.
pub const CONFIRM_TEXT: &str = "Are you sure you want to forget this technique?";
/// The renderer style bits a picked row lights in the select mode.
pub const PICKED_STYLE: u32 = 0x110;

/// One skill of the character record: its experience (`points`) and its
/// technique bytes (0 = empty). The rules here read the first five; the
/// record's sixth byte is never touched.
pub use exa_formats::characters::Skill as SkillRecord;

/// A skill's level: experience over 5 000.
pub fn skill_level(record: &SkillRecord) -> u16 {
    record.points / LEVEL_XP
}

/// The skill screen's training state: the record's six skills, the
/// screen's experience pool and its two flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TechniqueTraining {
    pub skills: [SkillRecord; SKILLS],
    /// The experience pool.
    pub pool: i32,
    /// Assignments round the slot's skill down to whole levels first.
    pub truncate: bool,
    /// Assignments draw from the pool by the weighted rule.
    pub weighted: bool,
}

/// The pairs of technique ids a technique depends on, by skill (1…6) and
/// technique id (0…8): the character data unit's table, read as one run
/// of pairs from skill 1's entry 0 (so technique 8 of a skill is the next
/// skill's entry 0). Forgetting a technique also forgets every later one
/// that names it.
pub const PREREQUISITES: [[i32; 2]; 49] = {
    let mut table = [[0; 2]; 49];
    table[8] = [1, 4];
    table[18] = [1, 0];
    table[19] = [2, 0];
    table[26] = [1, 0];
    table[27] = [2, 0];
    table[43] = [2, 0];
    table[44] = [3, 0];
    table
};

/// The pair a technique depends on: skill `skill` (1…6), technique `id`.
pub fn prerequisites(skill: usize, id: u8) -> [i32; 2] {
    PREREQUISITES
        .get((skill.saturating_sub(1)) * ROWS + usize::from(id))
        .copied()
        .unwrap_or([0, 0])
}

impl TechniqueTraining {
    /// The six skills' experience, summed as 32-bit words.
    pub fn spent(&self) -> i32 {
        self.skills
            .iter()
            .map(|skill| i32::from(skill.points))
            .sum()
    }

    /// The experience not yet in a skill: the pool less the six words.
    pub fn available(&self) -> i32 {
        self.pool.wrapping_sub(self.spent())
    }

    /// The record of skill `skill` (1…6).
    pub fn skill(&self, skill: u16) -> Option<&SkillRecord> {
        self.skills.get(usize::from(skill).checked_sub(1)?)
    }

    fn skill_mut(&mut self, skill: u16) -> Option<&mut SkillRecord> {
        self.skills.get_mut(usize::from(skill).checked_sub(1)?)
    }

    /// The technique in training: the first skill whose slot one past its
    /// level holds a technique, as (skill 1…6, slot 1…5). A skill at level
    /// five or more has none.
    pub fn in_training(&self) -> Option<(u16, u16)> {
        self.skills.iter().enumerate().find_map(|(index, skill)| {
            let level = usize::from(skill_level(skill));
            (level < SLOTS && skill.techniques[level] != 0)
                .then_some((index as u16 + 1, level as u16 + 1))
        })
    }

    /// Forgets the technique in slot `slot` (1…5) of skill `skill` (1…6).
    ///
    /// When later slots are filled the technique is taken out and the
    /// later ones close up; every technique in the first four slots that
    /// names the forgotten one (or names one forgotten with it) goes too,
    /// and the skill loses a level for each technique gone. When the slot
    /// is the last filled one (or past it), it is cleared and the skill's
    /// experience becomes one level less than the techniques it held.
    /// An empty slot is matched against technique 0's pair, as the
    /// original does.
    pub fn forget(&mut self, skill: u16, slot: u16) {
        if skill == 0 || slot == 0 {
            return;
        }
        let Some(record) = self.skill_mut(skill) else {
            return;
        };
        let held = record.techniques[..SLOTS]
            .iter()
            .position(|&id| id == 0)
            .unwrap_or(SLOTS);
        let index = usize::from(slot) - 1;
        if usize::from(slot) < held {
            let removed = record.techniques[index];
            close_up(&mut record.techniques, index);
            let mut count: i32 = 1;
            forget_dependants(record, usize::from(skill), removed, &mut count, 0);
            record.points = record
                .points
                .wrapping_add((count as i16).wrapping_mul(-5_000) as u16);
        } else {
            if let Some(byte) = record.techniques[..SLOTS].get_mut(index) {
                *byte = 0;
            }
            record.points = ((held as i16) - 1).wrapping_mul(5_000) as u16;
        }
    }
}

/// Moves the slots after `index` down by one and clears the last.
fn close_up(techniques: &mut [u8], index: usize) {
    for slot in index..SLOTS - 1 {
        techniques[slot] = techniques[slot + 1];
    }
    techniques[SLOTS - 1] = 0;
}

/// The removal pass: scan the first four slots; a technique whose pair
/// names `removed` is taken out (counted), its own dependants follow, and
/// the scan starts over. The scan stops at the first empty slot after
/// testing it.
fn forget_dependants(
    record: &mut SkillRecord,
    skill: usize,
    removed: u8,
    count: &mut i32,
    depth: u32,
) {
    // The pass recurses once per technique taken out; a record holds five,
    // so this bound is never reached by the original's own data.
    if depth > 32 {
        return;
    }
    let mut slot = 0;
    while slot < DEPENDANT_SLOTS {
        let id = record.techniques[slot];
        let pair = prerequisites(skill, id);
        if pair.contains(&i32::from(removed)) {
            close_up(&mut record.techniques, slot);
            *count += 1;
            forget_dependants(record, skill, id, count, depth + 1);
            slot = 0;
        }
        if record.techniques[slot] == 0 {
            break;
        }
        slot += 1;
    }
}

/// The skill screen's slot the selector edits, and the technique last
/// picked into it there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TechniqueSelector {
    /// The skill, 1…6.
    pub skill: u16,
    /// The slot, 1…5.
    pub slot: u16,
    /// The technique last picked from this slot; below 1 means none.
    pub last: i32,
}

/// What a pick did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechniqueChange {
    /// The pick changed the record (and the tree is refreshed).
    pub changed: bool,
    /// The record is the player's: the body is refreshed too.
    pub refresh_body: bool,
}

/// Puts technique `technique` (1…8) into the selector's slot, or forgets
/// the slot's technique (a negative `technique`). Picking the same
/// technique again does nothing.
///
/// A forget refunds into the technique in training when the pool is
/// positive and the rule is not weighted: the unspent experience, at most
/// what lifts that skill to the training slot's level.
///
/// An assignment first rounds the skill down to whole levels when the
/// truncate flag is set and the skill is below the slot; an empty slot
/// takes the place of the technique in training (that one is forgotten).
/// With a positive pool the skill gains the unspent experience clamped to
/// 0…5 000, or by the weighted rule `pool · 4 / (4 + Σ levels)` capped at
/// 5 000; only a slot nothing was picked into before gains, but the
/// weighted rule always draws `gain · divisor / 4` from the pool.
pub fn assign_technique(
    training: &mut TechniqueTraining,
    selector: &mut TechniqueSelector,
    technique: i32,
    is_player: bool,
) -> TechniqueChange {
    if technique == selector.last {
        return TechniqueChange {
            changed: false,
            refresh_body: false,
        };
    }
    let (skill, slot) = (selector.skill, selector.slot);
    if technique < 0 {
        training.forget(skill, slot);
        if training.pool > 0
            && !training.weighted
            && let Some((target, level)) = training.in_training()
        {
            let available = training.available();
            if let Some(record) = training.skill_mut(target) {
                let room = i32::from(level) * i32::from(LEVEL_XP) - i32::from(record.points);
                let add = if room < available { room } else { available };
                record.points = record.points.wrapping_add(add as u16);
            }
        }
    } else if (1..=SLOTS as u16).contains(&slot) && training.skill(skill).is_some() {
        let index = usize::from(slot) - 1;
        let truncate = training.truncate;
        if let Some(record) = training.skill_mut(skill)
            && truncate
            && skill_level(record) < slot
        {
            record.points -= record.points % LEVEL_XP;
        }
        let empty = training
            .skill(skill)
            .is_some_and(|r| r.techniques[index] == 0);
        if empty && let Some((target, at)) = training.in_training() {
            training.forget(target, at);
        }
        let pool = training.pool;
        let available = training.available();
        let divisor = 4 + training
            .skills
            .iter()
            .map(|skill| i32::from(skill_level(skill)))
            .sum::<i32>();
        let weighted = training.weighted;
        let mut drawn = 0i64;
        if let Some(record) = training.skill_mut(skill) {
            record.techniques[index] = technique as u8;
            if pool > 0 {
                let gain = if weighted {
                    let gain = ((i64::from(pool) << 2) / i64::from(divisor)) as i32;
                    let gain = gain.min(i32::from(LEVEL_XP));
                    drawn = i64::from(gain) * i64::from(divisor) / 4;
                    gain
                } else {
                    available.clamp(0, i32::from(LEVEL_XP))
                };
                if selector.last < 1 {
                    record.points = record.points.wrapping_add(gain as u16);
                }
            }
        }
        if weighted && pool > 0 {
            training.pool = (i64::from(training.pool) - drawn) as i32;
        }
    }
    selector.last = technique;
    TechniqueChange {
        changed: true,
        refresh_body: is_player,
    }
}

/// A technique node's state on the skill tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// A learned technique (its slot is within the level).
    Learned = 0,
    /// A slot that cannot be opened yet.
    Locked = 1,
    /// A technique above the level (in training or waiting).
    Assigned = 2,
    /// An empty slot (also the next slot while a skill holds more
    /// techniques than its level).
    Empty = 3,
    /// The next slot to open.
    Next = 4,
}

/// One technique node of the refreshed tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechniqueNode {
    pub technique: u8,
    /// The skill, 1…6 (the node's colour and name group).
    pub skill: u16,
    pub state: NodeState,
}

/// One skill's refreshed column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillColumn {
    /// The header's fill: experience times 0.00004 (a double), so 25 000
    /// fills it.
    pub progress: f32,
    pub nodes: [TechniqueNode; SLOTS],
}

/// The refreshed tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TechniqueTree {
    pub skills: Vec<SkillColumn>,
    /// The techniques held across the skills.
    pub total: i32,
}

/// Rebuilds the skill tree's technique nodes after a pick or a change of
/// character. `all_learned` is the screen's flag that shows every slot up
/// to one past the level as locked.
///
/// Every filled slot is learned within the level and assigned above it,
/// every empty one empty. Then, unless `all_learned`: with fewer than 15
/// techniques in the tree, each skill's first empty slot among the first
/// `level + 1` becomes the next one (left empty while any skill holds
/// more techniques than its level) and the slots from `level + 2` lock;
/// with 15 or more, the empty slots from `level + 1` lock on a skill below
/// level five. With `all_learned` the first `level + 1` slots lock.
pub fn refresh_tree(training: &TechniqueTraining, all_learned: bool) -> TechniqueTree {
    let mut total = 0;
    let mut any_more = false;
    let mut skills: Vec<SkillColumn> = training
        .skills
        .iter()
        .enumerate()
        .map(|(index, record)| {
            let level = i32::from(skill_level(record));
            let mut count = 0;
            let nodes = std::array::from_fn(|slot| {
                let technique = record.techniques[slot];
                let state = if technique == 0 {
                    NodeState::Empty
                } else {
                    count += 1;
                    if level - 1 < slot as i32 {
                        NodeState::Assigned
                    } else {
                        NodeState::Learned
                    }
                };
                TechniqueNode {
                    technique,
                    skill: index as u16 + 1,
                    state,
                }
            });
            total += count;
            if level < count {
                any_more = true;
            }
            SkillColumn {
                progress: (f64::from(record.points) * 4e-5) as f32,
                nodes,
            }
        })
        .collect();
    for (column, record) in skills.iter_mut().zip(&training.skills) {
        let level = usize::from(skill_level(record));
        if all_learned {
            for node in column.nodes.iter_mut().take(level + 1) {
                node.state = NodeState::Locked;
            }
        } else if total < TECHNIQUE_LIMIT {
            let next = if any_more {
                NodeState::Empty
            } else {
                NodeState::Next
            };
            if let Some(node) = column
                .nodes
                .iter_mut()
                .take(level + 1)
                .find(|node| node.technique == 0)
            {
                node.state = next;
            }
            for node in column.nodes.iter_mut().skip(level + 1) {
                node.state = NodeState::Locked;
            }
        } else if level < SLOTS {
            for node in column.nodes.iter_mut().skip(level) {
                if node.state == NodeState::Empty {
                    node.state = NodeState::Locked;
                }
            }
        }
    }
    TechniqueTree { skills, total }
}

/// One row control of the selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectorRow {
    /// The control's id, one through eight.
    pub id: i32,
    /// The row's style word: 0x10 for a technique the skill's table marks
    /// unavailable.
    pub style: u32,
}

/// The selector's form and its eight row controls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorLayout {
    pub width: i32,
    pub height: i32,
    pub flags: u32,
    pub style: u32,
    /// The skill index the form names (the constructor's group less one).
    pub skill_index: i32,
    pub rows: [SelectorRow; ROWS],
}

/// The constructor's layout for skill group `group` (1…6): a 332 × 112
/// form with flags 0x21060 and style 0x10, eight row controls whose ids
/// are 1…8, a row the skill's technique table marks unavailable carrying
/// style 0x10.
pub fn layout(group: i32, available: [bool; ROWS]) -> SelectorLayout {
    SelectorLayout {
        width: FORM_WIDTH,
        height: FORM_HEIGHT,
        flags: 0x21060,
        style: 0x10,
        skill_index: group - 1,
        rows: std::array::from_fn(|row| SelectorRow {
            id: row as i32 + 1,
            style: if available[row] { 0 } else { 0x10 },
        }),
    }
}

/// The forget entry: the confirmation control's code sets the form's mode
/// to two and three of its controls' style words to 0x110, 0 and 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgetMode {
    pub mode: i32,
    pub style: u32,
    pub second: u32,
    pub third: u32,
}

/// Enters the confirmation for forgetting a technique.
pub fn enter_forget() -> ForgetMode {
    ForgetMode {
        mode: STATE_CONFIRM,
        style: PICKED_STYLE,
        second: 0,
        third: 0,
    }
}

/// Where the name, the description and the confirmation go, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechniqueText {
    pub name_x: i32,
    pub name_y: i32,
    pub box_x: i32,
    pub box_width: i32,
    pub description_y: i32,
    pub description_height: i32,
    pub confirm_y: i32,
    pub confirm_height: i32,
}

/// The text places for a name `name_width` pixels wide. The name's x is
/// `(width − name) >> 1` (an arithmetic halving, so an odd negative rounds
/// down).
pub fn text_places(form_width: i32, form_height: i32, unit: i32, name_width: i32) -> TechniqueText {
    TechniqueText {
        name_x: ((i64::from(form_width) - i64::from(name_width)) >> 1) as i32,
        name_y: unit << 4,
        box_x: unit,
        box_width: form_width - 2 * unit,
        description_y: unit * DESCRIPTION_Y_UNITS,
        description_height: form_height - unit * DESCRIPTION_BOTTOM_UNITS,
        confirm_y: unit * CONFIRM_Y_UNITS,
        confirm_height: unit * CONFIRM_HEIGHT_UNITS,
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

/// The key guard: with a technique picked in the confirmation mode the
/// base handles the key; otherwise the base keeps it only while row code 9
/// is hovered and the form's lock byte is clear, and any other key closes
/// the form with 0x40. The mode is compared unsigned.
pub fn selector_key(picked: i32, mode: i32, code: i32, locked: bool) -> SelectorKey {
    let base = (picked >= 1 && mode as u32 > 1) || (code & 0xffff == BASE_WORD && !locked);
    if base {
        SelectorKey {
            result: 0,
            close: false,
            base: true,
        }
    } else {
        SelectorKey {
            result: 0x40,
            close: true,
            base: false,
        }
    }
}

/// The hover rule: the picked technique is the hovered control's low word
/// (above eight it is none); in the select mode a pick lights the
/// renderer's 0x110 bits and no pick clears them.
pub fn clamp_index(raw: u32, mode: i32, style: u32) -> (i32, u32) {
    let mut index = raw & 0xffff;
    if index > ROWS as u32 {
        index = 0;
    }
    let mut style = style;
    if mode == MODE_SELECT {
        if index < 1 {
            style &= !PICKED_STYLE;
        } else {
            style |= PICKED_STYLE;
        }
    }
    (index as i32, style)
}

/// The technique the close picks: the picked one, or "forget" (−1) while
/// row code 9 or 10 is hovered with no pick, or nothing. The close also
/// clears the owner's 0x60 style bits.
pub fn close_pick(picked: i32, code: i32) -> Option<i32> {
    if picked < 1 {
        FORGET_WORDS.contains(&(code & 0xffff)).then_some(-1)
    } else {
        Some(picked)
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

/// The paint's order: the base, the skin at style 2 and either the
/// confirmation or the hovered technique's name and description (`row` is
/// the hovered control's low word less one).
pub fn draw_order(mode: i32, row: i32) -> Vec<SelectorDraw> {
    let mut draw = vec![SelectorDraw::Base, SelectorDraw::Skin];
    if mode == STATE_CONFIRM {
        draw.push(SelectorDraw::Confirmation);
    } else if (0..ROWS as i32).contains(&row) {
        draw.push(SelectorDraw::Technique { row: row as usize });
    }
    draw
}

#[cfg(test)]
mod tests {
    use super::*;

    fn training() -> TechniqueTraining {
        let mut t = TechniqueTraining {
            pool: 40_000,
            ..TechniqueTraining::default()
        };
        t.skills[0] = SkillRecord {
            points: 15_000,
            techniques: [1, 2, 0, 0, 0, 0],
        };
        t.skills[1] = SkillRecord {
            points: 10_000,
            techniques: [0, 0, 0, 0, 0, 0],
        };
        t
    }

    #[test]
    fn the_pool_less_the_six_skills_is_unspent() {
        let t = training();
        assert_eq!(t.spent(), 25_000);
        assert_eq!(t.available(), 15_000);
    }

    #[test]
    fn the_technique_in_training_is_one_past_the_level() {
        let mut t = training();
        assert_eq!(t.in_training(), None);
        // Skill 1 is level 3; a technique in slot 4 is in training.
        t.skills[0].techniques[3] = 5;
        assert_eq!(t.in_training(), Some((1, 4)));
        // A level-five skill has none.
        t.skills[0].points = 25_000;
        assert_eq!(t.in_training(), None);
    }

    #[test]
    fn an_assignment_adds_at_most_a_level_from_the_unspent_pool() {
        let mut t = training();
        let mut selector = TechniqueSelector {
            skill: 1,
            slot: 3,
            last: -1,
        };
        let change = assign_technique(&mut t, &mut selector, 7, false);
        assert!(change.changed);
        assert_eq!(t.skills[0].techniques[2], 7);
        // 15 000 unspent: a level is added.
        assert_eq!(t.skills[0].points, 20_000);
        assert_eq!(t.pool, 40_000);
        assert_eq!(selector.last, 7);
        // A second pick from the same slot gains nothing.
        assert!(assign_technique(&mut t, &mut selector, 6, false).changed);
        assert_eq!(t.skills[0].techniques[2], 6);
        assert_eq!(t.skills[0].points, 20_000);
        // The same technique again changes nothing.
        assert!(!assign_technique(&mut t, &mut selector, 6, false).changed);
        // A short pool adds what is left.
        let mut t = training();
        t.pool = 26_000;
        let mut selector = TechniqueSelector {
            skill: 2,
            slot: 1,
            last: 0,
        };
        assign_technique(&mut t, &mut selector, 3, true);
        assert_eq!(t.skills[1].points, 11_000);
    }

    #[test]
    fn the_weighted_gain_divides_by_four_plus_every_level() {
        let mut t = training();
        t.weighted = true;
        let mut selector = TechniqueSelector {
            skill: 2,
            slot: 1,
            last: -1,
        };
        // Levels 3 + 2: divisor 9. 40 000 · 4 / 9 = 17 777, capped at
        // 5 000; the pool drops by 5 000 · 9 / 4 = 11 250.
        assign_technique(&mut t, &mut selector, 4, false);
        assert_eq!(t.skills[1].points, 15_000);
        assert_eq!(t.pool, 28_750);
        // A small pool: 100 · 4 / 10 = 40 (levels 3 + 3 now), drawing
        // 40 · 10 / 4 = 100; a slot picked before still draws.
        t.pool = 100;
        assign_technique(&mut t, &mut selector, 5, false);
        assert_eq!(t.skills[1].points, 15_000);
        assert_eq!(t.pool, 0);
    }

    #[test]
    fn the_truncate_flag_rounds_the_skill_down_first() {
        let mut t = training();
        t.skills[1].points = 9_000;
        t.truncate = true;
        let mut selector = TechniqueSelector {
            skill: 2,
            slot: 3,
            last: 1,
        };
        assign_technique(&mut t, &mut selector, 2, false);
        // 9 000 / 5 000 = 1 < 3: 9 000 − 4 000; a picked slot gains nothing.
        assert_eq!(t.skills[1].points, 5_000);
    }

    #[test]
    fn an_empty_slot_replaces_the_technique_in_training() {
        let mut t = training();
        t.skills[0].techniques = [1, 2, 3, 5, 0, 0];
        let mut selector = TechniqueSelector {
            skill: 2,
            slot: 1,
            last: 1,
        };
        assign_technique(&mut t, &mut selector, 3, false);
        // Skill 1's slot 4 was the last filled: cleared, and the skill
        // is one level under the four it held.
        assert_eq!(t.skills[0].techniques, [1, 2, 3, 0, 0, 0]);
        assert_eq!(t.skills[0].points, 15_000);
        // A gap counts only the slots before it: 5 000.
        let mut t = training();
        t.skills[0].techniques = [1, 2, 0, 5, 0, 0];
        assign_technique(&mut t, &mut selector, 4, false);
        assert_eq!(t.skills[0].techniques, [1, 2, 0, 0, 0, 0]);
        assert_eq!(t.skills[0].points, 5_000);
        assert_eq!(t.skills[1].techniques[0], 4);
    }

    #[test]
    fn forgetting_closes_up_and_takes_the_dependants() {
        // Skill 3: technique 2 names 1, technique 3 names 2.
        let mut t = TechniqueTraining::default();
        t.skills[2] = SkillRecord {
            points: 20_000,
            techniques: [1, 2, 3, 4, 0, 0],
        };
        t.forget(3, 1);
        // 1 goes, then 2 (names 1), then 3 (names 2): three techniques
        // gone, 20 000 − 15 000.
        assert_eq!(t.skills[2].techniques, [4, 0, 0, 0, 0, 0]);
        assert_eq!(t.skills[2].points, 5_000);
        // The last filled slot clears and the skill is a level under the
        // count it held.
        let mut t = TechniqueTraining::default();
        t.skills[0] = SkillRecord {
            points: 12_000,
            techniques: [1, 2, 0, 0, 0, 0],
        };
        t.forget(1, 2);
        assert_eq!(t.skills[0].techniques, [1, 0, 0, 0, 0, 0]);
        assert_eq!(t.skills[0].points, 5_000);
        // Past the filled slots: one held, so level 0; an empty record
        // goes to −1 level, wrapped.
        t.forget(1, 4);
        assert_eq!(t.skills[0].points, 0);
        let mut empty = TechniqueTraining::default();
        empty.forget(1, 1);
        assert_eq!(empty.skills[0].points, 60_536);
    }

    #[test]
    fn a_forget_refunds_into_the_technique_in_training() {
        let mut t = training();
        t.skills[0].techniques = [1, 2, 3, 6, 0, 0];
        t.skills[1].techniques[2] = 4;
        let mut selector = TechniqueSelector {
            skill: 1,
            slot: 1,
            last: 1,
        };
        assign_technique(&mut t, &mut selector, -1, false);
        // Skill 1 loses technique 1 and a level: 10 000. Skill 1 (level
        // 2) then has a technique in slot 3: it is in training and takes
        // the unspent 20 000, capped at 3 · 5 000 − 10 000.
        assert_eq!(t.skills[0].techniques, [2, 3, 6, 0, 0, 0]);
        assert_eq!(t.skills[0].points, 15_000);
        assert_eq!(selector.last, -1);
        // A weighted rule refunds nothing.
        let mut t = training();
        t.weighted = true;
        t.skills[0].techniques = [1, 2, 3, 6, 0, 0];
        let mut selector = TechniqueSelector {
            skill: 1,
            slot: 1,
            last: 1,
        };
        assign_technique(&mut t, &mut selector, -1, false);
        assert_eq!(t.skills[0].points, 10_000);
    }

    #[test]
    fn the_refresh_marks_learned_assigned_next_and_locked_nodes() {
        let t = training();
        let tree = refresh_tree(&t, false);
        assert_eq!(tree.total, 2);
        let skill = &tree.skills[0];
        assert!((skill.progress - 0.6).abs() < 1e-6);
        assert_eq!(skill.nodes[0].state, NodeState::Learned);
        assert_eq!(skill.nodes[0].skill, 1);
        assert_eq!(skill.nodes[1].state, NodeState::Learned);
        // Level 3: the first empty slot of the first four is next; the
        // fifth (from level + 2) locks.
        assert_eq!(skill.nodes[2].state, NodeState::Next);
        assert_eq!(skill.nodes[3].state, NodeState::Empty);
        assert_eq!(skill.nodes[4].state, NodeState::Locked);
        let skill = &tree.skills[1];
        assert_eq!(skill.nodes[0].state, NodeState::Next);
        assert_eq!(skill.nodes[3].state, NodeState::Locked);
        assert_eq!(tree.skills[2].nodes[0].state, NodeState::Next);
        assert_eq!(tree.skills[2].nodes[1].state, NodeState::Locked);
    }

    #[test]
    fn more_techniques_than_the_level_keep_the_next_slot_empty() {
        let mut t = training();
        // Skill 2 at level 2 holds three: one is assigned above the level.
        t.skills[1].techniques = [1, 2, 3, 0, 0, 0];
        let tree = refresh_tree(&t, false);
        assert_eq!(tree.skills[1].nodes[2].state, NodeState::Assigned);
        assert_eq!(tree.skills[0].nodes[2].state, NodeState::Empty);
        assert_eq!(tree.skills[0].nodes[4].state, NodeState::Locked);
    }

    #[test]
    fn fifteen_techniques_lock_the_empty_slots_from_the_level() {
        let mut t = TechniqueTraining::default();
        for record in &mut t.skills {
            *record = SkillRecord {
                points: 10_000,
                techniques: [1, 2, 3, 0, 0, 0],
            };
        }
        let tree = refresh_tree(&t, false);
        assert_eq!(tree.total, 18);
        let skill = &tree.skills[0];
        assert_eq!(skill.nodes[2].state, NodeState::Assigned);
        assert_eq!(skill.nodes[3].state, NodeState::Locked);
        assert_eq!(skill.nodes[4].state, NodeState::Locked);
        // With every slot shown learned, the first level + 1 lock.
        let tree = refresh_tree(&t, true);
        assert_eq!(tree.skills[0].nodes[2].state, NodeState::Locked);
        assert_eq!(tree.skills[0].nodes[3].state, NodeState::Empty);
    }

    #[test]
    fn the_text_places_are_pinned() {
        let places = text_places(FORM_WIDTH, FORM_HEIGHT, 4, 100);
        assert_eq!(places.name_x, 116);
        assert_eq!(places.name_y, 64);
        assert_eq!(places.box_x, 4);
        assert_eq!(places.box_width, 324);
        assert_eq!(places.description_y, 72);
        assert_eq!(places.description_height, 32);
        assert_eq!(places.confirm_y, 48);
        assert_eq!(places.confirm_height, 80);
        // A name wider than the form halves downward.
        assert_eq!(text_places(10, 0, 4, 13).name_x, -2);
        assert_eq!(CONFIRM_TEXT.len(), 47);
    }

    #[test]
    fn the_paint_draws_the_confirmation_or_the_hovered_technique() {
        assert_eq!(
            draw_order(1, 3),
            vec![
                SelectorDraw::Base,
                SelectorDraw::Skin,
                SelectorDraw::Technique { row: 3 }
            ]
        );
        assert_eq!(
            draw_order(STATE_CONFIRM, 3),
            vec![
                SelectorDraw::Base,
                SelectorDraw::Skin,
                SelectorDraw::Confirmation
            ]
        );
        assert_eq!(draw_order(1, 8).len(), 2);
        assert_eq!(draw_order(1, -1).len(), 2);
    }

    #[test]
    fn the_layout_and_the_forget_mode_are_pinned() {
        let mut available = [true; ROWS];
        available[2] = false;
        let layout = layout(2, available);
        assert_eq!((layout.width, layout.height), (332, 112));
        assert_eq!(layout.flags, 0x21060);
        assert_eq!(layout.skill_index, 1);
        assert_eq!(layout.rows[0].id, 1);
        assert_eq!(layout.rows[0].style, 0);
        assert_eq!(layout.rows[2].style, 0x10);
        assert_eq!(layout.rows[7].id, 8);
        assert_eq!(
            enter_forget(),
            ForgetMode {
                mode: 2,
                style: 0x110,
                second: 0,
                third: 0
            }
        );
    }

    #[test]
    fn keys_go_to_the_base_only_in_its_two_cases() {
        // A pick in the confirmation mode.
        assert!(selector_key(3, 2, 1, false).base);
        // Row code 9 hovered with the lock byte clear.
        assert!(selector_key(0, 1, 9, false).base);
        assert!(selector_key(3, 1, 0x1_0009, false).base);
        // Everything else closes with 0x40.
        let closed = selector_key(3, 1, 3, false);
        assert_eq!(closed.result, 0x40);
        assert!(closed.close && !closed.base);
        assert!(selector_key(0, 2, 10, false).close);
        assert!(selector_key(0, 1, 9, true).close);
        // The mode compare is unsigned: a negative mode counts as large.
        assert!(selector_key(1, -1, 0, false).base);
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

    #[test]
    fn the_prerequisite_pairs_follow_the_table_run() {
        assert_eq!(prerequisites(1, 8), [1, 4]);
        assert_eq!(prerequisites(2, 0), [1, 4]);
        assert_eq!(prerequisites(3, 2), [1, 0]);
        assert_eq!(prerequisites(6, 4), [3, 0]);
        assert_eq!(prerequisites(6, 8), [0, 0]);
        assert_eq!(prerequisites(1, 1), [0, 0]);
    }
}
