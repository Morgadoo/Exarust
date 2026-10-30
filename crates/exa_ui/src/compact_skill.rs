//! The compact skill screen: character creation's short skill tree, its
//! layout, its pointer hits, its key navigation, its wheel and its paint.
//!
//! Evidence: the compact skill screen's routines in the retail image, read
//! from the instructions. The form is 240 × 800 at (400, 50). Rows are
//! `spacing` units apart from a `margin` (4 units, plus 6 with a title
//! longer than one byte, plus 5 for the compact flag on a narrow
//! interface); the spacing is 30 units on a four-skill tree, else 22 on a
//! wide interface and 24 on a narrow one. A row's techniques sit eleven
//! units a column from two units in; a stored hover word is `row << 8 |
//! column` (both 1-based). A skill's experience draws one pip per 5 000,
//! the remainder as a narrower last pip.

use crate::engine_math::round_i64;

/// One column is this many units wide.
pub const COLUMN_UNITS: i32 = 11;
/// The title band is this many units wide.
pub const TITLE_UNITS: i32 = 54;
/// The base margin in units.
pub const MARGIN_UNITS: i32 = 4;
/// A title longer than one byte adds this to the margin.
pub const TITLE_MARGIN_UNITS: i32 = 6;
/// The compact flag adds this on a narrow interface.
pub const COMPACT_MARGIN_UNITS: i32 = 5;
/// Four skills use this row spacing.
pub const SPACING_FOUR: i32 = 30;
/// Otherwise a wide interface uses this.
pub const SPACING_WIDE: i32 = 22;
/// And a narrow one this.
pub const SPACING_NARROW: i32 = 24;
/// A row's technique line sits this many units below the row's margin (a
/// double).
pub const NODE_Y_UNITS: f64 = 16.6;
/// A hit is strictly within five units.
pub const HIT_UNITS: i64 = 5;
/// A grid cell's box is fourteen units tall.
pub const CELL_HEIGHT_UNITS: i32 = 14;
/// A grid cell (and a pip) is twelve units wide.
pub const CELL_WIDTH_UNITS: i32 = 12;
/// One level of a skill.
pub const LEVEL: u16 = 5_000;
/// The share of each experience point in a level (a double).
pub const LEVEL_SHARE: f64 = 0.0002;
/// A pip is two units tall.
pub const PIP_HEIGHT_UNITS: i32 = 2;
/// A technique node sits 4.6 units below the row's line (a double).
pub const NODE_DROP_UNITS: f64 = 4.6;
/// The pip texture's u range and v range on the interface sheet.
pub const PIP_U: [f32; 2] = [0.571_428_6, 0.785_714_3];
pub const PIP_V: [f32; 2] = [0.7, 0.8];
/// The line colour (alpha 0x78), the pips' (alpha 0x80) and the hover
/// highlight's (alpha 0xff), as RGBA words.
pub const LINE_COLOUR: u32 = 0x78f0_f8ff;
pub const PIP_COLOUR: u32 = 0x8049_c9ff;
pub const HIGHLIGHT_COLOUR: u32 = 0xff99_ccff;

/// The form's constructor metrics and the layout pass's margin and
/// spacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactMetrics {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub flags: u32,
    pub style: u32,
    pub margin: i32,
    pub spacing: i32,
}

/// The metrics of a tree of `categories` rows whose title is `title_len`
/// bytes long.
pub fn metrics(categories: usize, title_len: usize, compact: bool, wide: bool) -> CompactMetrics {
    let mut margin = MARGIN_UNITS;
    if title_len > 1 {
        margin += TITLE_MARGIN_UNITS;
    }
    if compact && !wide {
        margin += COMPACT_MARGIN_UNITS;
    }
    let spacing = if categories == 4 {
        SPACING_FOUR
    } else if wide {
        SPACING_WIDE
    } else {
        SPACING_NARROW
    };
    CompactMetrics {
        x: 400,
        y: 50,
        width: 240,
        height: 800,
        flags: 0x30022,
        style: 0x900,
        margin,
        spacing,
    }
}

/// A cell's centre (row and column 1-based): `((column − 1)·11 + 6)` units
/// across, and `16` units plus `margin + spacing·(row − 1)` units down,
/// summed as floats.
pub fn cell_centre(metrics: CompactMetrics, unit: i32, row: i32, column: i32) -> (f32, f32) {
    let unit = i64::from(unit);
    let x = ((i64::from(column) - 1) * 11 + 6) * unit;
    let y = (i64::from(metrics.margin) + i64::from(metrics.spacing) * (i64::from(row) - 1)) * unit;
    (x as f32, (unit << 4) as f32 + y as f32)
}

/// A row's technique line (row 1-based): `unit · (spacing·(row − 1) +
/// (margin + 16.6))` in doubles, rounded half to even.
pub fn row_line(metrics: CompactMetrics, unit: i32, row: i32) -> i32 {
    let rows = (i64::from(metrics.spacing) * (i64::from(row) - 1)) as f64;
    (f64::from(unit) * (rows + (f64::from(metrics.margin) + NODE_Y_UNITS))).round_ties_even() as i32
}

/// The cell position a stored hover word decodes to: the column's left in
/// pixels and the row's technique line. `None` for a word without a row or
/// column, or past the tree's rows.
pub fn decode_cell(
    metrics: CompactMetrics,
    unit: i32,
    rows: usize,
    word: u32,
) -> Option<(i32, i32)> {
    let row = ((word & 0xff00) >> 8) as i32;
    if row < 1 || row as i64 > rows as i64 {
        return None;
    }
    let column = (word & 7) as i32;
    if column < 1 {
        return None;
    }
    let x = ((column - 1) * COLUMN_UNITS + 2) * unit;
    Some((x, row_line(metrics, unit, row)))
}

/// Tests a pointer against the tree's cells. A row whose centre line plus
/// six units is not above the point is scanned, its five cells in order,
/// and its first cell strictly within five units is taken; a later row
/// overrides an earlier one. The result is `row << 8 | column` (both
/// 1-based), or 0.
pub fn hit_cells(metrics: CompactMetrics, unit: i32, rows: usize, point: (i32, i32)) -> i32 {
    let limit = (i64::from(unit) * HIT_UNITS * i64::from(unit) * HIT_UNITS) as f32;
    let mut hit = 0;
    for row in 1..=rows as i32 {
        let y = (row - 1) * unit * metrics.spacing + (metrics.margin + 16) * unit;
        if i64::from(unit) * 6 + i64::from(y) < i64::from(point.1) {
            continue;
        }
        for column in 0..5 {
            let x = unit * 11 * column + unit * 6;
            let dx = i64::from(point.0) - i64::from(x);
            let dy = i64::from(point.1) - i64::from(y);
            if ((dy * dy + dx * dx) as f32) < limit {
                hit = (row << 8 | column) + 1;
                break;
            }
        }
    }
    hit
}

/// The extra header cell: `50 + 28` units across, `9` units plus
/// `spacing·(row − 1)` units down.
pub fn header_cell(unit: i32, spacing: i32, row: i32) -> (f32, f32) {
    let unit = i64::from(unit);
    (
        (unit * 50) as f32 + (unit * 28) as f32,
        (unit * 9) as f32 + ((i64::from(row) - 1) * unit * i64::from(spacing)) as f32,
    )
}

/// The pip run a skill's experience draws: how many pips and the last
/// pip's share (1 for a whole level).
pub fn pips(experience: i32) -> (i32, f32) {
    let mut count = experience / i32::from(LEVEL);
    let share = if i64::from(count) * 5_000 < i64::from(experience) {
        let rest = i64::from(experience) - i64::from(count) * 5_000;
        count += 1;
        (rest as f64 * LEVEL_SHARE) as f32
    } else {
        1.0
    };
    (count, share)
}

/// One pip's quad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pip {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// The texture's u at the right edge (the left is [`PIP_U`]`[0]`).
    pub u_right: f32,
}

/// Lays out a skill's pips from `x`, `y`: twelve units wide, each
/// overlapping the last by a unit. A last pip with a share under 0.99
/// (compared in double) takes `share · 0.8 + 0.1` of the width (rounded
/// half to even) and of the texture.
pub fn lay_out_pips(x: i32, y: i32, unit: i32, experience: i32) -> Vec<Pip> {
    let (count, mut share) = pips(experience);
    let mut width = unit * CELL_WIDTH_UNITS;
    let height = unit << 1;
    let mut x = x;
    let mut out = Vec::new();
    for i in 0..count {
        let mut u_right = PIP_U[1];
        if i64::from(count) - 1 == i64::from(i) && f64::from(share) < 0.99 {
            share = (f64::from(share) * 0.8 + 0.1) as f32;
            width = round_i64(width as f32 * share) as i32;
            u_right = (PIP_U[1] - PIP_U[0]) * share + PIP_U[0];
        }
        out.push(Pip {
            x,
            y,
            width,
            height,
            u_right,
        });
        x = x + width - unit;
    }
    out
}

/// A technique node's place in its row: two units in, eleven units a
/// column, and `round(unit · 4.6)` below the row's line.
pub fn node_place(x: i32, y: i32, unit: i32, column: i32) -> (i32, i32) {
    let drop = (f64::from(unit) * NODE_DROP_UNITS).round_ties_even() as i32;
    (unit * 11 * column + unit * 2 + x, y + drop)
}

/// Whether the hover word picks this skill (0-based) and column (0-based).
pub fn node_hovered(selected: u32, skill: usize, column: i32) -> bool {
    i64::from(selected >> 8 & 0xff) == skill as i64 + 1
        && i64::from(selected & 0xff) == i64::from(column) + 1
}

/// The node slots a skill's row draws, and whether the last is the one in
/// training: every learned level, plus one when the slot at the level is
/// filled, or while the screen's open-slot flag is set below level five.
pub fn technique_slots(points: u16, techniques: &[u8], open_slot: bool) -> (i32, bool) {
    let level = usize::from(points / LEVEL);
    if techniques.get(level).copied().unwrap_or(0) != 0 {
        (level as i32 + 1, true)
    } else if open_slot && level < 5 {
        (level as i32 + 1, false)
    } else {
        (level as i32, false)
    }
}

/// The slots the wheel counts in a row: the level, plus one below level
/// five when the slot at the level is filled or the open-slot flag is set.
pub fn wheel_slots(points: u16, techniques: &[u8], open_slot: bool) -> i32 {
    let level = i32::from(points / LEVEL);
    if level < 5 && (techniques.get(level as usize).copied().unwrap_or(0) != 0 || open_slot) {
        level + 1
    } else {
        level
    }
}

/// One technique node the paint draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeDraw {
    pub x: i32,
    pub y: i32,
    /// The technique's icon, or `None` for the empty-slot icon.
    pub technique: Option<u8>,
    /// The colour's alpha byte (the hover highlight's when hovered).
    pub alpha: u8,
    pub hovered: bool,
}

/// The alpha byte of an RGBA colour scaled by the screen's alpha.
pub fn scaled_alpha(colour: u32, alpha: f32) -> u8 {
    round_i64((colour >> 24) as f32 * alpha) as u8
}

#[allow(clippy::too_many_arguments)]
/// A skill's technique nodes on the line at `x`, `y`: filled slots draw
/// their technique (the one in training at half the line colour's alpha),
/// the first empty slot draws the empty icon and ends the row; the hovered
/// node draws in the highlight colour.
pub fn technique_nodes(
    points: u16,
    techniques: &[u8],
    open_slot: bool,
    skill: usize,
    selected: u32,
    x: i32,
    y: i32,
    unit: i32,
    alpha: f32,
) -> Vec<NodeDraw> {
    let (count, training) = technique_slots(points, techniques, open_slot);
    let mut nodes = Vec::new();
    if count == 0 {
        return nodes;
    }
    let mut line = scaled_alpha(LINE_COLOUR, alpha);
    let highlight = scaled_alpha(HIGHLIGHT_COLOUR, alpha);
    for column in 0..count {
        let (nx, ny) = node_place(x, y, unit, column);
        let hovered = node_hovered(selected, skill, column);
        let technique = techniques.get(column as usize).copied().unwrap_or(0);
        if technique == 0 {
            nodes.push(NodeDraw {
                x: nx,
                y: ny,
                technique: None,
                alpha: if hovered { highlight } else { line },
                hovered,
            });
            break;
        }
        if column == count - 1 && training {
            line >>= 1;
        }
        nodes.push(NodeDraw {
            x: nx,
            y: ny,
            technique: Some(technique),
            alpha: if hovered { highlight } else { line },
            hovered,
        });
    }
    nodes
}

/// The screen state the key handler reads and writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompactSkillState {
    /// The hovered word.
    pub selected: i32,
    /// The committed pick.
    pub choice: i32,
    /// The pending pick.
    pub pending: i32,
    /// The fade kind (1 after a pick, 2 after the hint key).
    pub fade: u8,
    /// The fade's deadline.
    pub deadline: u32,
    /// The form's style word; bit 0x40 marks a pick shown.
    pub style: u32,
    /// The eight slots already taken (0 = free).
    pub used: [u8; 8],
}

/// The six 16-bit fields a compact skill window copies from its owner (six
/// consecutive eight-byte slots of the owner's record), or zeroes when the
/// owner asks for that; the window refreshes its layout after either.
pub fn attach_owner(owner: [i16; 6], zero: bool) -> [i16; 6] {
    if zero { [0; 6] } else { owner }
}

/// The base skill screen's parts the key handler calls.
pub trait CompactBase {
    /// The base's answer to the confirm key on the hovered word: 1 makes it
    /// pending, 2 commits it, anything else does nothing.
    fn confirm(&mut self, hovered: i32) -> i32;
    /// Fills the eight used slots for skill row `row` (the hover word's
    /// bits 8…10).
    fn prepare(&mut self, row: i32, used: &mut [u8; 8]);
    /// The base's cancel (the confirm key while a pick is shown).
    fn cancel(&mut self);
    /// The form's hover setter: the hovered word becomes `value`.
    fn select(&mut self, state: &mut CompactSkillState, value: i32) {
        state.selected = value;
    }
}

/// One key: 0x100 confirm, 0x200 the hint key; with bit 0 set, the
/// previous (0x1001) and next (0x2001) keys.
///
/// With nothing picked only the confirm key acts: a pending answer makes
/// the hovered word pending and hovers 1; a commit makes it the choice,
/// lets the base fill the used slots and hovers the first free one (0 if
/// none); then the pick fades in (kind 1, 300 ms) and lights style 0x40.
/// With a pick shown, the confirm key cancels and the hint key fades out
/// (kind 2, 200 ms, unless a fade runs) and clears 0x40, each hovering the
/// pending pick or else the choice. The arrows step to the previous or
/// next free slot (1…8), or, while a pick is pending, by one (forward only
/// from 1).
pub fn handle_key(state: &mut CompactSkillState, key: u32, now: u32, base: &mut impl CompactBase) {
    if state.choice == 0 && state.pending == 0 {
        if key == 0x100 {
            match base.confirm(state.selected) {
                1 => {
                    state.pending = state.selected;
                    base.select(state, 1);
                }
                2 => {
                    state.choice = state.selected;
                    base.prepare((state.selected >> 8) & 7, &mut state.used);
                    let first = state
                        .used
                        .iter()
                        .position(|&slot| slot == 0)
                        .map_or(0, |slot| slot as i32 + 1);
                    base.select(state, first);
                }
                _ => return,
            }
            state.fade = 1;
            state.deadline = now.wrapping_add(300);
            state.style |= 0x40;
        }
        return;
    }
    let shown = |state: &CompactSkillState| {
        if state.pending == 0 {
            state.choice
        } else {
            state.pending
        }
    };
    if key & 1 == 0 {
        if key & 0x100 != 0 {
            base.cancel();
            let value = shown(state);
            base.select(state, value);
        } else if key & 0x200 != 0 {
            if state.fade == 0 {
                state.fade = 2;
                state.deadline = now.wrapping_add(200);
            }
            state.style &= !0x40;
            let value = shown(state);
            base.select(state, value);
        }
        return;
    }
    let hovered = state.selected;
    let mut next = hovered;
    if state.pending == 0 {
        match key & 0xf001 {
            0x1001 if hovered >= 2 => {
                if let Some(slot) = (1..hovered)
                    .rev()
                    .find(|&slot| state.used[slot as usize - 1] == 0)
                {
                    next = slot;
                }
            }
            0x2001 if hovered < 8 => {
                if let Some(slot) =
                    (hovered.max(0) + 1..=8).find(|&slot| state.used[slot as usize - 1] == 0)
                {
                    next = slot;
                }
            }
            _ => {}
        }
    } else {
        match key & 0xf001 {
            0x1001 => next = hovered - 1,
            0x2001 if hovered < 2 => next = hovered + 1,
            _ => {}
        }
    }
    if next > 0 && next != hovered {
        base.select(state, next);
    }
}

/// One row the wheel walks: the skill's order id and its record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactRow {
    /// The skill this row shows (the screen's order table).
    pub skill: i32,
    pub points: u16,
    pub techniques: [u8; 6],
}

/// Scrolls the selection by the wheel's `amount` (x then y) from the
/// pointer's `x`, `y`, which it moves to the picked cell. A mostly
/// horizontal wheel (the y magnitude below the x one) takes the row whose
/// line is nearest the pointer and moves a column that way; a vertical
/// one takes the row above (wheel up) or below (wheel down) the pointer,
/// and keeps the pointer's column. Rows without slots are skipped. Returns
/// the hover word `(skill + 1) << 8 | column`, or `None`.
pub fn wheel(
    metrics: CompactMetrics,
    unit: i32,
    rows: &[CompactRow],
    open_slot: bool,
    x: &mut i32,
    y: &mut i32,
    amount: [f32; 2],
) -> Option<u32> {
    // An unordered compare: a NaN magnitude counts as vertical.
    let vertical = f32::from_bits(amount[1].to_bits() & 0x7fff_ffff)
        .partial_cmp(&f32::from_bits(amount[0].to_bits() & 0x7fff_ffff))
        != Some(std::cmp::Ordering::Less);
    let mut line =
        (f64::from(unit) * (f64::from(metrics.margin) + NODE_Y_UNITS)).round_ties_even() as i32;
    let mut best: i32 = -100_000;
    let mut chosen: Option<usize> = None;
    let step = unit * metrics.spacing;
    for (index, row) in rows.iter().enumerate() {
        if wheel_slots(row.points, &row.techniques, open_slot) < 1 {
            line = line.wrapping_add(step);
            continue;
        }
        let pointer = i64::from(*y);
        if !vertical {
            let distance = (i64::from(line) - pointer).abs();
            let previous = (i64::from(best) - pointer).abs();
            if distance > previous {
                break;
            }
            best = line;
        } else if amount[1] > 0.0 {
            if pointer + i64::from(unit) < i64::from(line) {
                chosen = Some(index);
                break;
            }
        } else if pointer - i64::from(unit) < i64::from(line) {
            break;
        }
        chosen = Some(index);
        line = line.wrapping_add(step);
    }
    let chosen = chosen?;
    let row = rows[chosen];
    let slots = wheel_slots(row.points, &row.techniques, open_slot);
    let mut column = ((i64::from(*x) - i64::from(unit * 2)) / (i64::from(unit) * 11)) as i32;
    if column < 0 {
        column = 0;
    } else if slots <= column {
        column = slots - 1;
    }
    let mut px = unit * 11 * column + unit * 2;
    if !vertical {
        if amount[0].partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) {
            if i64::from(column) < i64::from(slots) - 1 && *x > unit {
                column += 1;
                px += unit * 11;
            }
        } else if column > 0 {
            column -= 1;
            px -= unit * 11;
        }
    }
    *x = px;
    *y = row_line(metrics, unit, chosen as i32 + 1);
    Some((((i64::from(row.skill) + 1) << 8) | (i64::from(column) + 1)) as u32)
}

/// One step of the paint, with the numbers it uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompactDraw {
    /// The form's skin, style 2.
    Skin,
    /// A category's name centred in the title band.
    CategoryLabel { row: usize, x: f32, y: f32 },
    /// The form's title (a title longer than one byte).
    Title { x: f32, y: f32 },
    /// A row's line: from `x` at `y`, `length` long.
    Line {
        row: usize,
        x: i32,
        y: i32,
        length: i32,
    },
    /// One of the compact flag's five header cells: two stacked quads of
    /// 12 × 2 units from `y`.
    HeaderCell { x: i32, y: i32 },
    /// A grid cell, 12 × 14 units.
    Grid {
        row: usize,
        column: usize,
        x: i32,
        y: i32,
    },
    /// A skill's pips from x 0.
    Pips { row: usize, y: i32 },
    /// The compact flag's own pips (the screen's spare experience).
    CompactPips { y: i32 },
    /// A skill's technique nodes on its line.
    Nodes { row: usize, y: i32 },
    /// The base's hover ring for the hovered word, when positive.
    Hover { word: i32 },
    /// The committed pick's overlay.
    Choice,
    /// The pending pick's overlay (with no choice).
    Pending,
}

/// The paint's order with a tree of `rows` categories: the category names
/// (`widths` measured), the title, the lines, the compact header cells,
/// the grid, the pips, the nodes, then the base's hover ring and the
/// choice or the pending overlay.
#[allow(clippy::too_many_arguments)]
pub fn draw_order(
    metrics: CompactMetrics,
    unit: i32,
    rows: usize,
    widths: &[i32],
    title: Option<i32>,
    compact: bool,
    wide: bool,
    state: &CompactSkillState,
    hover: i32,
) -> Vec<CompactDraw> {
    let band = unit * TITLE_UNITS;
    let step = unit * metrics.spacing;
    let centred = |width: i32| ((i64::from(band) - i64::from(width)) / 2) as i32 + unit;
    let mut draw = vec![CompactDraw::Skin];
    let mut y = (metrics.margin + 7) * unit;
    for row in 0..rows {
        let width = widths.get(row).copied().unwrap_or(0);
        draw.push(CompactDraw::CategoryLabel {
            row,
            x: centred(width) as f32,
            y: y as f32,
        });
        y += step;
    }
    if let Some(width) = title {
        draw.push(CompactDraw::Title {
            x: centred(width) as f32,
            y: (unit << 2) as f32,
        });
    }
    let mut y = unit * metrics.margin;
    for row in 0..rows {
        draw.push(CompactDraw::Line {
            row,
            x: unit,
            y,
            length: band,
        });
        y += step;
    }
    let cell = unit * CELL_WIDTH_UNITS;
    if compact {
        let y = if wide { unit * 5 } else { unit * 6 };
        let mut x = 0;
        for _ in 0..5 {
            draw.push(CompactDraw::HeaderCell { x, y });
            x = x + cell - unit;
        }
    }
    let mut y = (metrics.margin + 8) * unit;
    for row in 0..rows {
        let mut x = 0;
        for column in 0..5 {
            draw.push(CompactDraw::Grid { row, column, x, y });
            x = x + cell - unit;
        }
        y += step;
    }
    let mut y = (metrics.margin + 9) * unit;
    for row in 0..rows {
        draw.push(CompactDraw::Pips { row, y });
        y += step;
    }
    if compact {
        draw.push(CompactDraw::CompactPips {
            y: if wide { unit * 6 } else { unit * 7 },
        });
    }
    let mut y = (metrics.margin + 8) * unit;
    for row in 0..rows {
        draw.push(CompactDraw::Nodes { row, y });
        y += step;
    }
    if hover > 0 {
        draw.push(CompactDraw::Hover { word: hover });
    }
    if state.choice != 0 {
        draw.push(CompactDraw::Choice);
    } else if state.pending != 0 {
        draw.push(CompactDraw::Pending);
    }
    draw
}

#[cfg(test)]
mod tests {
    use super::*;

    fn narrow() -> CompactMetrics {
        metrics(6, 0, false, false)
    }

    #[test]
    fn the_metrics_pick_the_margin_and_spacing() {
        let m = metrics(4, "Close Combat".len(), false, false);
        assert_eq!((m.x, m.y, m.width, m.height), (400, 50, 240, 800));
        assert_eq!((m.flags, m.style), (0x30022, 0x900));
        assert_eq!((m.margin, m.spacing), (10, 30));
        assert_eq!(metrics(6, 1, true, false).margin, 9);
        assert_eq!(metrics(6, 1, true, true).margin, 4);
        assert_eq!(metrics(6, 0, false, true).spacing, 22);
        assert_eq!(metrics(6, 0, false, false).spacing, 24);
    }

    #[test]
    fn cells_centre_decode_and_header_are_pinned() {
        let m = narrow();
        assert_eq!(cell_centre(m, 4, 1, 1), (24.0, 80.0));
        assert_eq!(cell_centre(m, 4, 2, 3), (112.0, 176.0));
        // 4 · (24 + (4 + 16.6)) = 178.4 → 178.
        assert_eq!(decode_cell(m, 4, 6, 0x0203), Some((96, 178)));
        assert_eq!(decode_cell(m, 4, 6, 0x0101), Some((8, 82)));
        assert_eq!(decode_cell(m, 4, 6, 0x0700), None);
        assert_eq!(decode_cell(m, 4, 6, 0x0201 + 0x500), None);
        assert_eq!(decode_cell(m, 4, 6, 0x0008), None);
        // Unit 5: 5 · 20.6 = 103 exactly.
        assert_eq!(row_line(m, 5, 1), 103);
        assert_eq!(header_cell(4, 24, 2), (312.0, 132.0));
    }

    #[test]
    fn a_hit_takes_the_first_column_and_the_last_row() {
        let m = narrow();
        // Row 1's line is at (4 + 16) · 4 = 80; cell 1 at x 24.
        assert_eq!(hit_cells(m, 4, 6, (24, 80)), 0x101);
        assert_eq!(hit_cells(m, 4, 6, (68, 80)), 0x102);
        // Strictly within five units: 20 px off is out.
        assert_eq!(hit_cells(m, 4, 6, (44, 80)), 0);
        assert_eq!(hit_cells(m, 4, 6, (43, 80)), 0x101);
        // A point below a row's band skips it; rows 2… still scan.
        assert_eq!(hit_cells(m, 4, 6, (24, 176)), 0x201);
        // With a spacing under five units two rows can both hold the point:
        // the later wins.
        let tight = CompactMetrics { spacing: 2, ..m };
        assert_eq!(hit_cells(tight, 4, 2, (24, 84)), 0x201);
    }

    #[test]
    fn pips_narrow_the_last_by_its_share() {
        assert_eq!(pips(0), (0, 1.0));
        assert_eq!(pips(5_000), (1, 1.0));
        assert_eq!(pips(12_500), (3, 0.5));
        let pips = lay_out_pips(0, 10, 4, 12_500);
        assert_eq!(pips.len(), 3);
        assert_eq!(
            pips[0],
            Pip {
                x: 0,
                y: 10,
                width: 48,
                height: 8,
                u_right: PIP_U[1]
            }
        );
        assert_eq!(pips[1].x, 44);
        // 0.5 · 0.8 + 0.1 = 0.5: 24 px and half the texture.
        assert_eq!(pips[2].x, 88);
        assert_eq!(pips[2].width, 24);
        assert!((pips[2].u_right - 0.678_571_4).abs() < 1e-6);
        // A share of 0.99 or more is a whole pip.
        let full = lay_out_pips(0, 0, 4, 9_950);
        assert_eq!(full[1].width, 48);
        let short = lay_out_pips(0, 0, 4, 9_949);
        assert!(short[1].width < 48);
    }

    #[test]
    fn nodes_stop_at_the_first_empty_slot_and_dim_the_one_in_training() {
        // Level 1 with slot 2 filled: two nodes, the second in training.
        let nodes = technique_nodes(5_000, &[3, 4, 0, 0, 0, 0], false, 0, 0, 0, 100, 4, 1.0);
        assert_eq!(nodes.len(), 2);
        assert_eq!((nodes[0].x, nodes[0].y), (8, 118));
        assert_eq!(nodes[0].alpha, 0x78);
        assert_eq!(nodes[1].x, 52);
        assert_eq!(nodes[1].alpha, 0x3c);
        // The open-slot flag shows an empty node after the level.
        let nodes = technique_nodes(5_000, &[3, 0, 0, 0, 0, 0], true, 1, 0x0202, 0, 100, 4, 1.0);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[1].technique, None);
        assert!(nodes[1].hovered);
        assert_eq!(nodes[1].alpha, 0xff);
        // An empty learned slot ends the row.
        let nodes = technique_nodes(10_000, &[0, 5, 0, 0, 0, 0], false, 0, 0, 0, 0, 4, 1.0);
        assert_eq!(nodes.len(), 1);
        assert_eq!(
            technique_slots(25_000, &[1, 2, 3, 4, 5, 0], true),
            (5, false)
        );
        assert_eq!(wheel_slots(25_000, &[1, 2, 3, 4, 5, 6], false), 5);
        assert_eq!(scaled_alpha(LINE_COLOUR, 0.5), 60);
    }

    struct Base {
        answer: i32,
        used: [u8; 8],
        cancelled: usize,
    }

    impl CompactBase for Base {
        fn confirm(&mut self, _hovered: i32) -> i32 {
            self.answer
        }
        fn prepare(&mut self, _row: i32, used: &mut [u8; 8]) {
            *used = self.used;
        }
        fn cancel(&mut self) {
            self.cancelled += 1;
        }
    }

    #[test]
    fn the_confirm_key_makes_a_pick_pending_or_commits_it() {
        let mut base = Base {
            answer: 1,
            used: [0; 8],
            cancelled: 0,
        };
        let mut state = CompactSkillState {
            selected: 0x203,
            ..CompactSkillState::default()
        };
        handle_key(&mut state, 0x100, 1_000, &mut base);
        assert_eq!(state.pending, 0x203);
        assert_eq!(state.choice, 0);
        assert_eq!(state.selected, 1);
        assert_eq!((state.fade, state.deadline, state.style), (1, 1_300, 0x40));
        let mut state = CompactSkillState {
            selected: 0x203,
            ..CompactSkillState::default()
        };
        base.answer = 2;
        base.used = [1, 1, 0, 1, 0, 0, 0, 0];
        handle_key(&mut state, 0x100, 0, &mut base);
        assert_eq!(state.choice, 0x203);
        assert_eq!(state.selected, 3);
        // Any other answer changes nothing.
        let mut state = CompactSkillState::default();
        base.answer = 0;
        handle_key(&mut state, 0x100, 0, &mut base);
        assert_eq!(state, CompactSkillState::default());
        // Other keys do nothing while nothing is picked.
        handle_key(&mut state, 0x2001, 0, &mut base);
        assert_eq!(state, CompactSkillState::default());
    }

    #[test]
    fn with_a_pick_shown_the_keys_cancel_hint_and_step() {
        let mut base = Base {
            answer: 0,
            used: [0; 8],
            cancelled: 0,
        };
        let mut state = CompactSkillState {
            selected: 4,
            choice: 0x203,
            style: 0x40,
            used: [1, 0, 1, 0, 1, 1, 0, 1],
            ..CompactSkillState::default()
        };
        // The arrows skip the used slots.
        handle_key(&mut state, 0x1001, 0, &mut base);
        assert_eq!(state.selected, 2);
        handle_key(&mut state, 0x1001, 0, &mut base);
        assert_eq!(state.selected, 2);
        handle_key(&mut state, 0x2001, 0, &mut base);
        assert_eq!(state.selected, 4);
        handle_key(&mut state, 0x2001, 0, &mut base);
        assert_eq!(state.selected, 7);
        // The hint key fades out once and hovers the choice.
        handle_key(&mut state, 0x200, 500, &mut base);
        assert_eq!((state.fade, state.deadline, state.style), (2, 700, 0));
        assert_eq!(state.selected, 0x203);
        state.deadline = 0;
        handle_key(&mut state, 0x200, 900, &mut base);
        assert_eq!(state.deadline, 0);
        // The confirm key cancels and hovers the pending pick first.
        state.pending = 0x105;
        handle_key(&mut state, 0x100, 0, &mut base);
        assert_eq!(base.cancelled, 1);
        assert_eq!(state.selected, 0x105);
        // While pending the arrows step by one; forward only from 1.
        state.selected = 1;
        handle_key(&mut state, 0x2001, 0, &mut base);
        assert_eq!(state.selected, 2);
        handle_key(&mut state, 0x2001, 0, &mut base);
        assert_eq!(state.selected, 2);
        handle_key(&mut state, 0x1001, 0, &mut base);
        assert_eq!(state.selected, 1);
        handle_key(&mut state, 0x1001, 0, &mut base);
        assert_eq!(state.selected, 1);
    }

    fn rows() -> Vec<CompactRow> {
        vec![
            CompactRow {
                skill: 0,
                points: 10_000,
                techniques: [1, 2, 3, 0, 0, 0],
            },
            CompactRow {
                skill: 1,
                points: 0,
                techniques: [0; 6],
            },
            CompactRow {
                skill: 2,
                points: 5_000,
                techniques: [1, 0, 0, 0, 0, 0],
            },
        ]
    }

    #[test]
    fn a_horizontal_wheel_takes_the_nearest_row_and_moves_a_column() {
        let m = narrow();
        // Lines: row 1 at 82, row 3 at 4 · (48 + 20.6) = 274.4 → 274.
        // Row 3 has one slot: the column is held to it.
        let (mut x, mut y) = (60, 260);
        let word = wheel(m, 4, &rows(), false, &mut x, &mut y, [1.0, 0.0]);
        assert_eq!(word, Some(0x301));
        assert_eq!((x, y), (8, 274));
        // Row 1 has three slots: one column on from column 1, then back.
        let (mut x, mut y) = (60, 90);
        assert_eq!(
            wheel(m, 4, &rows(), false, &mut x, &mut y, [1.0, 0.0]),
            Some(0x103)
        );
        assert_eq!((x, y), (96, 82));
        let (mut x, mut y) = (60, 90);
        assert_eq!(
            wheel(m, 4, &rows(), false, &mut x, &mut y, [-1.0, 0.0]),
            Some(0x101)
        );
        assert_eq!(x, 8);
    }

    #[test]
    fn a_vertical_wheel_takes_the_row_above_or_below() {
        let m = narrow();
        // Down from row 1's line: the first row below the pointer.
        let (mut x, mut y) = (60, 82);
        assert_eq!(
            wheel(m, 4, &rows(), false, &mut x, &mut y, [0.0, 1.0]),
            Some(0x301)
        );
        assert_eq!((x, y), (8, 274));
        // Up from row 3's line: the last row above it.
        let (mut x, mut y) = (60, 274);
        assert_eq!(
            wheel(m, 4, &rows(), false, &mut x, &mut y, [0.0, -1.0]),
            Some(0x102)
        );
        // Up from above every row: nothing.
        let (mut x, mut y) = (60, 10);
        assert_eq!(
            wheel(m, 4, &rows(), false, &mut x, &mut y, [0.0, -1.0]),
            None
        );
        assert_eq!((x, y), (60, 10));
    }

    #[test]
    fn the_paint_draws_names_lines_cells_pips_nodes_and_overlays() {
        let m = narrow();
        let state = CompactSkillState {
            pending: 3,
            ..CompactSkillState::default()
        };
        let draw = draw_order(m, 4, 2, &[100, 80], Some(60), true, false, &state, 0x101);
        assert_eq!(draw[0], CompactDraw::Skin);
        assert_eq!(
            draw[1],
            CompactDraw::CategoryLabel {
                row: 0,
                x: 62.0,
                y: 44.0
            }
        );
        assert_eq!(
            draw[2],
            CompactDraw::CategoryLabel {
                row: 1,
                x: 72.0,
                y: 140.0
            }
        );
        assert_eq!(draw[3], CompactDraw::Title { x: 82.0, y: 16.0 });
        assert_eq!(
            draw[4],
            CompactDraw::Line {
                row: 0,
                x: 4,
                y: 16,
                length: 216
            }
        );
        assert_eq!(
            draw[5],
            CompactDraw::Line {
                row: 1,
                x: 4,
                y: 112,
                length: 216
            }
        );
        assert_eq!(draw[6], CompactDraw::HeaderCell { x: 0, y: 24 });
        assert_eq!(draw[7], CompactDraw::HeaderCell { x: 44, y: 24 });
        assert_eq!(
            draw[11],
            CompactDraw::Grid {
                row: 0,
                column: 0,
                x: 0,
                y: 48
            }
        );
        assert_eq!(
            draw[12],
            CompactDraw::Grid {
                row: 0,
                column: 1,
                x: 44,
                y: 48
            }
        );
        assert_eq!(draw[21], CompactDraw::Pips { row: 0, y: 52 });
        assert_eq!(draw[23], CompactDraw::CompactPips { y: 28 });
        assert_eq!(draw[24], CompactDraw::Nodes { row: 0, y: 48 });
        assert_eq!(draw[26], CompactDraw::Hover { word: 0x101 });
        assert_eq!(draw[27], CompactDraw::Pending);
        assert_eq!(draw.len(), 28);
        let chosen = CompactSkillState {
            choice: 1,
            pending: 3,
            ..CompactSkillState::default()
        };
        let draw = draw_order(m, 4, 1, &[0], None, false, false, &chosen, 0);
        assert_eq!(draw.last(), Some(&CompactDraw::Choice));
    }

    #[test]
    fn a_compact_window_takes_six_owner_fields_or_zeroes() {
        assert_eq!(attach_owner([1, 2, 3, 4, 5, 6], false), [1, 2, 3, 4, 5, 6]);
        assert_eq!(attach_owner([1, 2, 3, 4, 5, 6], true), [0; 6]);
    }
}
