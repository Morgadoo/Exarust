//! The compact skill screen: character creation's short skill tree, its
//! layout, its pointer hits, its key navigation and its drawing order.
//!
//! Evidence: the compact skill screen's routines in the retail image. One
//! row per skill, eleven units apart; a row sits `margin` units from the
//! last plus 16.6 units; a row's height is the row spacing (30 units on a
//! four-skill tree, else 22 when the interface is wide, else 24). The
//! margin is 4 units, plus 6 when a category title is shown, plus 5 for the
//! compact flag on a narrow interface. A stored skill word is experience;
//! five thousand is a level and the pips past the last level show the
//! remainder as a dimmed pip.

use crate::selection::quarter_sine;

/// One column is this many units wide (the original's `0xb`).
pub const COLUMN_UNITS: i32 = 11;
/// The title band is this many units wide (`0x36`).
pub const TITLE_UNITS: i32 = 54;
/// The base margin in units.
pub const MARGIN_UNITS: i32 = 4;
/// A shown category title adds this to the margin.
pub const TITLE_MARGIN_UNITS: i32 = 6;
/// The compact flag adds this on a narrow interface.
pub const COMPACT_MARGIN_UNITS: i32 = 5;
/// Four skills use this row spacing.
pub const SPACING_FOUR: i32 = 30;
/// Otherwise a wide interface uses this.
pub const SPACING_WIDE: i32 = 22;
/// And a narrow one this.
pub const SPACING_NARROW: i32 = 24;
/// The first node's vertical offset inside a row, in units (`16.6`).
pub const NODE_Y_UNITS: f64 = 16.6;
/// A hit is within five units.
pub const HIT_UNITS: i32 = 5;
/// A cell's box is fourteen units tall.
pub const CELL_HEIGHT_UNITS: i32 = 14;
/// A cell is twelve units wide.
pub const CELL_WIDTH_UNITS: i32 = 12;
/// One level of a skill.
pub const LEVEL: u16 = 5_000;
/// The share of each experience point in a level (0.0002).
pub const LEVEL_SHARE: f64 = 0.0002;
/// A pip's width in units.
pub const PIP_UNITS: f32 = 12.0;

/// The form's constructor metrics: where it sits and how its rows are
/// spaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactMetrics {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub margin: i32,
    pub spacing: i32,
}

/// The metrics a tree of `categories` rows with `title` shown gets.
pub fn metrics(categories: usize, title: &str, compact: bool, wide: bool) -> CompactMetrics {
    let mut margin = MARGIN_UNITS;
    if title.chars().count() > 1 {
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
        margin,
        spacing,
    }
}

/// Where a 1-based row/column cell sits: its centre in pixels.
pub fn cell_centre(
    metrics: CompactMetrics,
    unit: i32,
    row: i32,
    column: i32,
) -> Option<(f32, f32)> {
    if row < 1 || column < 1 {
        return None;
    }
    let x = ((column - 1) * COLUMN_UNITS + 6) as f32 * unit as f32;
    let y = (i64::from(metrics.margin) + i64::from(metrics.spacing) * i64::from(row - 1) + 16)
        * i64::from(unit);
    Some((x, y as f32))
}

/// The cell position the screen's stored word decodes to: the row's column
/// start in pixels and its centre line. `None` for a word without a row or
/// column, or past the tree's rows.
pub fn decode_cell(
    metrics: CompactMetrics,
    unit: i32,
    rows: usize,
    word: i32,
) -> Option<(i32, i32)> {
    let row = (word >> 8) & 0xff;
    if row == 0 || row as usize > rows {
        return None;
    }
    let column = word & 7;
    if column == 0 {
        return None;
    }
    let x = ((column - 1) * COLUMN_UNITS + 2) * unit;
    let y = (f64::from(unit)
        * (f64::from((row - 1) * metrics.spacing) + f64::from(metrics.margin) + NODE_Y_UNITS))
        .round_ties_even() as i32;
    Some((x, y))
}

/// Tests a pointer against the tree's cells. The rows are walked in order;
/// the first row whose top band contains the point is scanned, its five
/// cells in order, and the first cell within five units wins. The result
/// is the stored word `(row << 8 | column) + 1`, or 0.
pub fn hit_cells(metrics: CompactMetrics, unit: i32, rows: usize, point: (i32, i32)) -> i32 {
    let unit = i64::from(unit);
    let limit = unit * i64::from(HIT_UNITS);
    for row in 1..=rows {
        let y =
            ((row as i64 - 1) * i64::from(metrics.spacing) + i64::from(metrics.margin + 16)) * unit;
        if i64::from(point.1) > y + unit * 6 {
            continue;
        }
        for column in 0..5i64 {
            let x = (column * 11 + 6) * unit;
            let dx = i64::from(point.0) - x;
            let dy = i64::from(point.1) - y;
            if dx * dx + dy * dy < limit * limit {
                return ((row as i32) << 8 | (column as i32 + 1)) + 1;
            }
        }
    }
    0
}

/// Where a cell's box starts (left and top) and how wide it is: the pips
/// and technique nodes use this.
pub fn cell_box(metrics: CompactMetrics, unit: i32, row: i32, column: i32) -> (i32, i32) {
    let x = ((column - 1) * COLUMN_UNITS + 2) * unit;
    let y = (f64::from(unit)
        * (f64::from((row - 1) * metrics.spacing) + f64::from(metrics.margin) + NODE_Y_UNITS))
        .round_ties_even() as i32;
    (x, y)
}

/// The pip run a skill's experience draws: how many pips and the partial
/// last pip's share.
pub fn pips(experience: u16) -> (i32, f32) {
    let mut count = i32::from(experience) / i32::from(LEVEL);
    let partial = if i64::from(count) * i64::from(LEVEL) < i64::from(experience) {
        count += 1;
        (f64::from(experience) - f64::from((count - 1) * i32::from(LEVEL))) as f32
            * LEVEL_SHARE as f32
    } else {
        1.0
    };
    (count, partial)
}

/// One pip's quad: its width, its left edge and its texture block's
/// coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pip {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    /// The first and the last vertex's texture coordinates.
    pub uv_from: [f32; 2],
    pub uv_to: [f32; 2],
    /// Whether this is the partial last pip.
    pub partial: bool,
}

/// Lays out a skill's pips from `x` and `y`. A partial last pip narrows by
/// `share = share × 0.8 + 0.1` and slides its texture toward the lit end.
pub fn lay_out_pips(x: i32, y: i32, unit: i32, experience: u16) -> Vec<Pip> {
    let (count, mut share) = pips(experience);
    let mut pips = Vec::new();
    let mut at = x as f32;
    for i in 0..count {
        let mut width = PIP_UNITS * unit as f32;
        let (from, to, partial) = if i == count - 1 && share < 0.99 {
            share = share * 0.8 + 0.1;
            width = (PIP_UNITS * unit as f32 * share).round_ties_even();
            ([0.571_428_6, 0.6], [0.785_714_3, 0.6], true)
        } else {
            ([0.571_428_6, 0.7], [0.785_714_3, 0.7], false)
        };
        pips.push(Pip {
            x: at,
            y: y as f32,
            width,
            uv_from: from,
            uv_to: to,
            partial,
        });
        at = at + width - unit as f32;
    }
    pips
}

/// The extra header cell's position: `78` units across and `9` units plus
/// one row spacing down (the original's vmt-slot helper).
pub fn header_cell(unit: i32, spacing: i32, row: i32) -> (f32, f32) {
    (
        (78 * unit) as f32,
        ((9 + spacing * (row - 1)) * unit) as f32,
    )
}

/// A technique node's place in its row: two units in, eleven units a column,
/// and 4.6 units below the row's top.
pub fn node_place(x: i32, y: i32, unit: i32, column: i32) -> (f32, f32) {
    (
        (x + 2 * unit + 11 * unit * column) as f32,
        (y as f64 + f64::from(unit) * 4.6).round_ties_even() as f32,
    )
}

/// Whether the screen's hover word picks this row and column (both
/// 1-based).
pub fn node_hovered(selected: i32, row: usize, column: i32) -> bool {
    selected >> 8 & 0xff == row as i32 + 1 && selected & 0xff == column + 1
}

/// The levels a skill's technique row shows: every learned level plus, when
/// the skill's level slot is filled or the "extra" flag is set, one more.
/// Returns the number of slots to draw and whether the last one is the
/// half-lit extra.
pub fn technique_slots(levels: u16, slot_filled: bool, extra: bool) -> (i32, bool) {
    let level = i32::from(levels) / i32::from(LEVEL);
    if slot_filled {
        (level + 1, true)
    } else if extra && level < 5 {
        (level + 1, false)
    } else {
        (level, false)
    }
}

/// A key answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactKeyAction {
    /// The screen's stored hover word after the key.
    pub selected: i32,
    /// The committed pick (0x180).
    pub choice: i32,
    /// The pending pick (0x184).
    pub pending: i32,
    /// The fade word (0x159).
    pub fade: u8,
    /// The fade's deadline.
    pub deadline: i32,
    /// The style word's 0x40 bit.
    pub style_bit: bool,
    /// Inform the form's owner that its layout changed.
    pub invalidate: bool,
    /// The base handler's pick (1 or 2), if it ran.
    pub base: Option<i32>,
}

/// The screen state the key handler changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompactSkillState {
    pub selected: i32,
    pub choice: i32,
    pub pending: i32,
    pub fade: u8,
    pub deadline: i32,
    pub style: u32,
    /// The slots already taken, in order.
    pub used: [u8; 8],
}

/// The base handler's answer on the confirm key: 1 keeps the pending pick,
/// 2 commits the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseConfirm {
    Pending,
    Commit,
    Nothing,
}

/// The base handler's two answers.
pub trait CompactBase {
    fn confirm(&mut self, selected: i32, index: i32) -> BaseConfirm;
    fn prepare(&mut self, row: i32, used: &mut [u8; 8]);
    fn action(&mut self);
}

/// One key. `key` is the screen's key word; the low bit means a pointer
/// event, 0x100 the confirm/back key, 0x200 the hint key, 0x1001 and 0x2001
/// the previous/next keys.
pub fn handle_key(
    state: &mut CompactSkillState,
    key: u32,
    now: i32,
    base: &mut impl CompactBase,
) -> CompactKeyAction {
    let mut out = CompactKeyAction {
        selected: state.selected,
        choice: state.choice,
        pending: state.pending,
        fade: state.fade,
        deadline: state.deadline,
        style_bit: state.style & 0x40 != 0,
        invalidate: false,
        base: None,
    };
    if state.choice == 0 && state.pending == 0 {
        if key == 0x100 {
            let answer = base.confirm(state.selected, state.selected);
            match answer {
                BaseConfirm::Pending => {
                    state.pending = state.selected;
                    base.action();
                    state.choice = 1;
                }
                BaseConfirm::Commit => {
                    state.choice = state.selected;
                    base.prepare((state.selected >> 8) & 7, &mut state.used);
                    let mut first = 0;
                    for i in 0..8 {
                        if state.used[i] == 0 {
                            first = i as i32 + 1;
                            break;
                        }
                    }
                    state.choice = first;
                    base.action();
                }
                BaseConfirm::Nothing => return out,
            }
            out.base = Some(match answer {
                BaseConfirm::Pending => 1,
                _ => 2,
            });
            out.invalidate = true;
            state.fade = 1;
            state.deadline = now + 300;
            state.style |= 0x40;
        }
        out.selected = state.selected;
        out.choice = state.choice;
        out.pending = state.pending;
        out.fade = state.fade;
        out.deadline = state.deadline;
        out.style_bit = state.style & 0x40 != 0;
        return out;
    }
    if key & 1 == 0 {
        if key & 0x100 != 0 {
            base.action();
            state.choice = if state.pending > 0 {
                state.pending
            } else {
                state.choice
            };
        } else if key & 0x200 != 0 {
            out.invalidate = true;
            if state.fade == 0 {
                state.fade = 2;
                state.deadline = now + 200;
            }
            state.style &= !0x40;
            state.choice = if state.pending > 0 {
                state.pending
            } else {
                state.choice
            };
        }
    } else {
        let mut next = state.selected;
        if state.pending == 0 {
            match key & 0xf001 {
                0x1001 => {
                    if state.selected >= 2 {
                        let mut at = state.selected - 1;
                        loop {
                            let zero = state.used.get((at - 1) as usize).copied().unwrap_or(0) == 0;
                            if zero {
                                next = at;
                                break;
                            }
                            at -= 1;
                            if at < 1 {
                                break;
                            }
                        }
                    }
                }
                0x2001 if state.selected < 8 => {
                    let mut at = state.selected;
                    loop {
                        if state.used.get(at as usize).copied().unwrap_or(1) == 0 {
                            next = at + 1;
                            break;
                        }
                        if at >= 7 {
                            break;
                        }
                        at += 1;
                    }
                }
                _ => {}
            }
        } else {
            match key & 0xf001 {
                0x1001 => next = state.selected - 1,
                0x2001 if state.selected < 2 => next = state.selected + 1,
                _ => {}
            }
        }
        if next > 0 && next != state.selected {
            state.selected = next;
            base.action();
        }
    }
    out.selected = state.selected;
    out.choice = state.choice;
    out.pending = state.pending;
    out.fade = state.fade;
    out.deadline = state.deadline;
    out.style_bit = state.style & 0x40 != 0;
    out
}

/// One row of the compact tree: the skill it names and its experience.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactRow {
    pub skill: u32,
    pub levels: u16,
}

/// The wheel's answer: the new first visible column (`x`), the selected
/// row's centre (`y`) and the stored word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WheelAnswer {
    pub x: i32,
    pub y: i32,
    pub word: u32,
}

/// Scrolls the tree. A horizontal wheel picks the row nearest the pointer
/// and one column further that way; a vertical wheel moves the row by one
/// (it stops at the rows either side of the pointer). `amount` is the
/// pointer's delta: x then y.
pub fn wheel(
    metrics: CompactMetrics,
    unit: i32,
    tree: &[CompactRow],
    extra: u16,
    x: &mut i32,
    y: &mut i32,
    amount: &mut [f32; 2],
) -> Option<WheelAnswer> {
    let horizontal = (amount[1].to_bits() & 0x7fff_ffff) < (amount[0].to_bits() & 0x7fff_ffff);
    let mut start =
        (f64::from(unit) * (f64::from(metrics.margin) + NODE_Y_UNITS)).round_ties_even() as i32;
    let mut chosen = -1i32;
    let mut best = -100_000i32;
    for row in 0..tree.len() + 1 {
        let levels = levels_at(tree, extra, row)?;
        let (slots, _) = technique_slots(levels, false, false);
        if slots == 0 {
            start = start.wrapping_add(metrics.spacing * unit);
            continue;
        }
        if horizontal {
            let distance = start.wrapping_sub(*y).abs();
            let previous = best.wrapping_sub(*y).abs();
            if chosen >= 0 && previous < distance {
                break;
            }
            best = start;
            chosen = row as i32;
        } else if amount[1] <= 0.0 {
            if y.wrapping_sub(unit) < start {
                break;
            }
            chosen = row as i32;
        } else {
            if y.wrapping_add(unit) < start {
                break;
            }
            chosen = row as i32;
        }
        start = start.wrapping_add(metrics.spacing * unit);
    }
    if chosen < 0 {
        return None;
    }
    let row = chosen as usize;
    let levels = levels_at(tree, extra, row)?;
    let (slots, _) = technique_slots(levels, false, false);
    let mut column = x.wrapping_sub(2 * unit) / (COLUMN_UNITS * unit);
    if column < 0 {
        column = 0;
    } else if slots <= column {
        column = slots - 1;
    }
    let mut px = COLUMN_UNITS * unit * column + 2 * unit;
    if horizontal {
        if amount[0] >= 0.0 {
            if column < slots - 1 && *x > unit {
                column += 1;
                px += COLUMN_UNITS * unit;
            }
        } else if column > 0 {
            column -= 1;
            px -= COLUMN_UNITS * unit;
        }
    }
    *x = px;
    *y = (f64::from(unit)
        * (f64::from(metrics.spacing * chosen) + f64::from(metrics.margin) + NODE_Y_UNITS))
        .round_ties_even() as i32;
    let skill = if row < tree.len() { tree[row].skill } else { 0 };
    Some(WheelAnswer {
        x: *x,
        y: *y,
        word: ((skill.wrapping_add(1)) << 8) | (column as u32 + 1),
    })
}

fn levels_at(tree: &[CompactRow], extra: u16, row: usize) -> Option<u16> {
    if row < tree.len() {
        Some(tree[row].levels)
    } else if row == tree.len() {
        Some(extra)
    } else {
        None
    }
}

/// One step of the drawing order, with the numbers it uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompactDraw {
    /// The form's skin at unit 2.
    SkinBack,
    /// A category title at its centred x and its y.
    CategoryLabel { row: usize, x: f32, y: f32 },
    /// The extra title line (when longer than one character).
    Title { x: f32, y: f32 },
    /// A row's separator line: x, y, length.
    Separator { x: f32, y: f32, length: f32 },
    /// A separator block above the extra row.
    ExtraBlock { x: f32, y: f32 },
    /// A cell's grid box.
    Grid {
        row: usize,
        column: usize,
        x: f32,
        y: f32,
    },
    /// A skill's pips row.
    Pips { row: usize, y: f32 },
    /// A skill's technique nodes row.
    Nodes { row: usize, y: f32 },
    /// The bottom title, when the form has one.
    BottomTitle,
    /// The pending or selected overlay.
    Overlay,
}

/// The paint's order with a tree of `rows` categories.
#[allow(clippy::too_many_arguments)]
pub fn draw_order(
    metrics: CompactMetrics,
    unit: i32,
    rows: usize,
    widths: &[i32],
    title_width: i32,
    extra: bool,
    wide: bool,
    bottom_title: bool,
    overlay: bool,
) -> Vec<CompactDraw> {
    let mut draw = vec![CompactDraw::SkinBack];
    let mut y = (metrics.margin + 7) * unit;
    for row in 0..rows {
        let width = widths.get(row).copied().unwrap_or(0);
        draw.push(CompactDraw::CategoryLabel {
            row,
            x: (unit + (TITLE_UNITS * unit - width) / 2) as f32,
            y: y as f32,
        });
        y += metrics.spacing * unit;
    }
    if title_width > 1 {
        draw.push(CompactDraw::Title {
            x: (unit + (TITLE_UNITS * unit - title_width) / 2) as f32,
            y: 4.0 * unit as f32,
        });
    }
    let mut y = (metrics.margin + 8) * unit;
    for _ in 0..rows {
        draw.push(CompactDraw::Separator {
            x: unit as f32,
            y: y as f32,
            length: (TITLE_UNITS * unit) as f32,
        });
        y += metrics.spacing * unit;
    }
    if extra {
        let first = if wide { 5 * unit } else { 6 * unit };
        let mut x = 0;
        for _ in 0..5 {
            draw.push(CompactDraw::ExtraBlock {
                x: x as f32,
                y: first as f32,
            });
            x += 12 * unit - unit;
        }
    }
    let mut y = (metrics.margin + 8) * unit;
    for row in 0..rows {
        for column in 0..5 {
            draw.push(CompactDraw::Grid {
                row,
                column,
                x: (column as f32) * 11.0 * unit as f32,
                y: y as f32,
            });
        }
        y += metrics.spacing * unit;
    }
    let mut y = (metrics.margin + 9) * unit;
    for row in 0..rows {
        draw.push(CompactDraw::Pips { row, y: y as f32 });
        y += metrics.spacing * unit;
    }
    let mut y = (metrics.margin + 8) * unit;
    for row in 0..rows {
        draw.push(CompactDraw::Nodes { row, y: y as f32 });
        y += metrics.spacing * unit;
    }
    if bottom_title {
        draw.push(CompactDraw::BottomTitle);
    }
    if overlay {
        draw.push(CompactDraw::Overlay);
    }
    draw
}

/// The base skill screen's own drawing uses this ease for a selection's
/// ring, kept here so the compact screen's overlay shares it (guess: the
/// base's draw uses the same quarter sine).
pub fn overlay_ease(share: f32) -> f32 {
    quarter_sine(share)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_metrics_pick_the_margin_and_spacing() {
        assert_eq!(
            metrics(4, "Close Combat", false, false),
            CompactMetrics {
                x: 400,
                y: 50,
                width: 240,
                height: 800,
                margin: 10,
                spacing: 30,
            }
        );
        assert_eq!(metrics(6, "", false, true).margin, 4);
        assert_eq!(metrics(6, "", true, false).margin, 9);
        assert_eq!(metrics(6, "", true, true).margin, 4);
        assert_eq!(metrics(6, "", false, true).spacing, 22);
        assert_eq!(metrics(6, "", false, false).spacing, 24);
        assert_eq!(metrics(4, "", true, false).spacing, 30);
    }

    #[test]
    fn cells_sit_eleven_units_apart() {
        let metrics = metrics(4, "x", false, false);
        assert_eq!(cell_centre(metrics, 4, 1, 1), Some((24.0, 80.0)));
        assert_eq!(cell_centre(metrics, 4, 1, 2), Some((68.0, 80.0)));
        assert_eq!(cell_centre(metrics, 4, 2, 1), Some((24.0, 200.0)));
        assert_eq!(cell_centre(metrics, 4, 0, 1), None);
        assert_eq!(cell_centre(metrics, 4, 1, 0), None);
    }

    #[test]
    fn a_word_decodes_to_its_cell() {
        let metrics = metrics(4, "x", false, false);
        // Row 1, column 1.
        assert_eq!(decode_cell(metrics, 4, 6, 0x0101), Some((8, 82)));
        // Row 2, column 3.
        assert_eq!(decode_cell(metrics, 4, 6, 0x0203), Some((96, 202)));
        assert_eq!(decode_cell(metrics, 4, 6, 0x0001), None);
        assert_eq!(decode_cell(metrics, 4, 6, 0x0100), None);
        assert_eq!(decode_cell(metrics, 4, 4, 0x0501), None);
        assert_eq!(cell_box(metrics, 4, 1, 1), (8, 82));
        assert_eq!(cell_box(metrics, 4, 2, 3), (96, 202));
    }

    #[test]
    fn a_hit_walks_the_rows_and_cells() {
        let metrics = metrics(4, "x", false, false);
        // Row 1's centre of column 1 is (24, 80); five units is 20 px.
        assert_eq!(hit_cells(metrics, 4, 4, (24, 80)), (0x0101) + 1);
        assert_eq!(hit_cells(metrics, 4, 4, (68, 80)), (0x0102) + 1);
        // The limit is strict: five units exactly is outside.
        assert_eq!(hit_cells(metrics, 4, 4, (24, 80 + 19)), (0x0101) + 1);
        assert_eq!(hit_cells(metrics, 4, 4, (24, 80 + 20)), 0);
        // Row 2 begins 30 units lower.
        assert_eq!(hit_cells(metrics, 4, 4, (24, 200)), (0x0201) + 1);
        // A point past every row's band is no hit.
        assert_eq!(hit_cells(metrics, 4, 4, (24, 1000)), 0);
    }

    #[test]
    fn a_skill_experience_makes_levels_and_a_partial_pip() {
        assert_eq!(pips(0), (0, 1.0));
        let (count, partial) = pips(4_999);
        assert_eq!(count, 1);
        assert!((partial - 0.999_8).abs() < 1e-6);
        assert_eq!(pips(5_000), (1, 1.0));
        let (count, partial) = pips(5_001);
        assert_eq!(count, 2);
        assert!((partial - 0.000_2).abs() < 1e-6);
        assert_eq!(pips(15_000), (3, 1.0));
        let pips = lay_out_pips(10, 20, 4, 5_001);
        assert_eq!(pips.len(), 2);
        assert_eq!(pips[0].x, 10.0);
        assert_eq!(pips[0].width, 48.0);
        assert!(!pips[0].partial);
        // The last pip: share 0.0002·1 = 0.0002, then ×0.8+0.1 = 0.10016.
        assert!(pips[1].partial);
        assert_eq!(pips[1].width, 5.0);
        // The next pip starts at the previous width less one unit.
        let three = lay_out_pips(0, 0, 4, 15_000);
        assert_eq!(three[1].x, 44.0);
        assert_eq!(three[2].x, 88.0);
    }

    #[test]
    fn technique_slots_show_a_learned_level_and_its_extra() {
        assert_eq!(technique_slots(0, false, false), (0, false));
        assert_eq!(technique_slots(5_000, false, false), (1, false));
        assert_eq!(technique_slots(10_000, true, false), (3, true));
        assert_eq!(technique_slots(10_000, false, true), (3, false));
        assert_eq!(technique_slots(25_000, false, true), (5, false));
        assert_eq!(technique_slots(30_000, false, true), (6, false));
    }

    struct Base {
        answer: BaseConfirm,
        actions: i32,
        prepared: Option<i32>,
    }
    impl CompactBase for Base {
        fn confirm(&mut self, _selected: i32, _index: i32) -> BaseConfirm {
            self.answer
        }
        fn prepare(&mut self, row: i32, used: &mut [u8; 8]) {
            self.prepared = Some(row);
            used[2] = 1;
        }
        fn action(&mut self) {
            self.actions += 1;
        }
    }

    #[test]
    fn the_confirm_key_pends_or_commits_a_pick() {
        let mut base = Base {
            answer: BaseConfirm::Pending,
            actions: 0,
            prepared: None,
        };
        let mut state = CompactSkillState {
            selected: 0x0203,
            ..CompactSkillState::default()
        };
        let out = handle_key(&mut state, 0x100, 1_000, &mut base);
        assert_eq!(out.base, Some(1));
        assert_eq!(state.pending, 0x0203);
        assert_eq!(state.choice, 1);
        assert!(out.style_bit);
        assert_eq!(state.fade, 1);
        assert_eq!(state.deadline, 1_300);
        assert!(out.invalidate);

        let mut base = Base {
            answer: BaseConfirm::Commit,
            actions: 0,
            prepared: None,
        };
        let mut state = CompactSkillState {
            selected: 0x0203,
            ..CompactSkillState::default()
        };
        let out = handle_key(&mut state, 0x100, 1_000, &mut base);
        assert_eq!(out.base, Some(2));
        assert_eq!(base.prepared, Some(2));
        // The first free used slot is 1.
        assert_eq!(state.choice, 1);

        let mut base = Base {
            answer: BaseConfirm::Nothing,
            actions: 0,
            prepared: None,
        };
        let mut state = CompactSkillState::default();
        let out = handle_key(&mut state, 0x100, 1_000, &mut base);
        assert_eq!(out.base, None);
        assert_eq!(state.fade, 0);
    }

    #[test]
    fn the_hint_key_hides_and_the_confirm_restores() {
        let mut base = Base {
            answer: BaseConfirm::Nothing,
            actions: 0,
            prepared: None,
        };
        let mut state = CompactSkillState {
            selected: 2,
            choice: 4,
            pending: 0,
            style: 0x40,
            ..CompactSkillState::default()
        };
        let out = handle_key(&mut state, 0x200, 5_000, &mut base);
        assert!(out.invalidate);
        assert_eq!(state.fade, 2);
        assert_eq!(state.deadline, 5_200);
        assert_eq!(state.style & 0x40, 0);
        assert_eq!(state.choice, 4);
        // A second hint does not restamp its deadline.
        handle_key(&mut state, 0x200, 5_100, &mut base);
        assert_eq!(state.deadline, 5_200);
        // The confirm action takes the pending pick if there is one.
        state.pending = 6;
        handle_key(&mut state, 0x100, 6_000, &mut base);
        assert_eq!(state.choice, 6);
        assert_eq!(base.actions, 1);
    }

    #[test]
    fn the_arrow_keys_skip_used_slots_and_stop() {
        let mut base = Base {
            answer: BaseConfirm::Nothing,
            actions: 0,
            prepared: None,
        };
        // Backward from 4: slot 3 is the first free one below.
        let mut state = CompactSkillState {
            selected: 4,
            choice: 1,
            used: [1, 1, 0, 0, 0, 0, 0, 0],
            ..CompactSkillState::default()
        };
        handle_key(&mut state, 0x1001, 0, &mut base);
        assert_eq!(state.selected, 3);
        // Forward from 4: slot 5 is not free, slot 6 is.
        let mut state = CompactSkillState {
            selected: 4,
            choice: 1,
            used: [1, 1, 1, 1, 1, 0, 0, 0],
            ..CompactSkillState::default()
        };
        handle_key(&mut state, 0x2001, 0, &mut base);
        assert_eq!(state.selected, 6);
        // At 8 forward stops.
        let mut state = CompactSkillState {
            selected: 8,
            choice: 1,
            ..CompactSkillState::default()
        };
        handle_key(&mut state, 0x2001, 0, &mut base);
        assert_eq!(state.selected, 8);
        // With a pending pick the arrows move one and stop at 1 and 2.
        let mut state = CompactSkillState {
            selected: 1,
            pending: 5,
            ..CompactSkillState::default()
        };
        handle_key(&mut state, 0x1001, 0, &mut base);
        assert_eq!(state.selected, 1);
        handle_key(&mut state, 0x2001, 0, &mut base);
        assert_eq!(state.selected, 2);
        handle_key(&mut state, 0x2001, 0, &mut base);
        assert_eq!(state.selected, 2);
    }

    #[test]
    fn the_wheel_scrolls_a_row_and_picks_a_column() {
        let metrics = metrics(4, "x", false, false);
        let tree = [
            CompactRow {
                skill: 11,
                levels: 10_000,
            },
            CompactRow {
                skill: 12,
                levels: 10_000,
            },
            CompactRow {
                skill: 13,
                levels: 5_000,
            },
            CompactRow {
                skill: 14,
                levels: 5_000,
            },
        ];
        // A vertical wheel picks the row around the pointer: row 2 at 202.
        let mut x = 8;
        let mut y = 202;
        let mut amount = [0.0, 1.0];
        let answer = wheel(metrics, 4, &tree, 0, &mut x, &mut y, &mut amount).unwrap();
        assert_eq!(answer.word, 0x0d01);
        assert_eq!(answer.y, 202);
        // A row above the first start is not chosen.
        let mut x = 8;
        let mut y = 10;
        let mut amount = [0.0, 1.0];
        assert!(wheel(metrics, 4, &tree, 0, &mut x, &mut y, &mut amount).is_none());
        // A horizontal wheel picks the nearest row and shifts one column.
        let mut x = 8;
        let mut y = 82;
        let mut amount = [1.0, 0.0];
        let answer = wheel(metrics, 4, &tree, 0, &mut x, &mut y, &mut amount).unwrap();
        assert_eq!(answer.word, 0x0c02);
        assert_eq!(answer.x, 52);
        assert_eq!(answer.y, 82);
        // The column clamps to the row's learned slots (two).
        let mut x = 200;
        let mut y = 82;
        let mut amount = [1.0, 0.0];
        let answer = wheel(metrics, 4, &tree, 0, &mut x, &mut y, &mut amount).unwrap();
        assert_eq!(answer.word, 0x0c02);
        assert_eq!(answer.x, 52);
        // No rows give no answer.
        let mut amount = [0.0, 1.0];
        assert!(wheel(metrics, 4, &[], 0, &mut x, &mut y, &mut amount).is_none());
    }

    #[test]
    fn the_header_and_nodes_sit_where_the_slots_place_them() {
        assert_eq!(header_cell(4, 30, 1), (312.0, 36.0));
        assert_eq!(header_cell(4, 30, 3), (312.0, 276.0));
        assert_eq!(node_place(8, 82, 4, 0), (16.0, 100.0));
        assert_eq!(node_place(8, 82, 4, 2), (104.0, 100.0));
        assert!(node_hovered(0x0203, 1, 2));
        assert!(!node_hovered(0x0203, 1, 1));
        assert!(!node_hovered(0, 0, 0));
    }

    #[test]
    fn the_draw_order_puts_the_pips_and_nodes_under_the_title() {
        let metrics = metrics(4, "x", false, false);
        let widths = [10, 10, 10, 10];
        let draw = draw_order(metrics, 4, 4, &widths, 20, true, false, true, true);
        assert_eq!(draw[0], CompactDraw::SkinBack);
        assert_eq!(
            draw[1],
            CompactDraw::CategoryLabel {
                row: 0,
                x: 107.0,
                y: 44.0
            }
        );
        assert_eq!(draw[5], CompactDraw::Title { x: 102.0, y: 16.0 });
        assert_eq!(
            draw[6],
            CompactDraw::Separator {
                x: 4.0,
                y: 48.0,
                length: 216.0
            }
        );
        assert_eq!(draw[10], CompactDraw::ExtraBlock { x: 0.0, y: 24.0 });
        assert!(draw.contains(&CompactDraw::Pips { row: 3, y: 412.0 }));
        assert!(draw.contains(&CompactDraw::Nodes { row: 3, y: 408.0 }));
        assert_eq!(draw.last(), Some(&CompactDraw::Overlay));
        let plain = draw_order(metrics, 4, 4, &widths, 1, false, false, false, false);
        assert_eq!(plain.last(), Some(&CompactDraw::Nodes { row: 3, y: 408.0 }));
        // A wide interface puts the extra blocks five units down.
        let wide = draw_order(metrics, 4, 4, &widths, 20, true, true, false, false);
        assert!(wide.contains(&CompactDraw::ExtraBlock { x: 0.0, y: 20.0 }));
    }
}
