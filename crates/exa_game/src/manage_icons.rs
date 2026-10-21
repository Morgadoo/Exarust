//! The arsenal icon sheet, `arsenalico.rfi`, as plain data: its cells, the
//! runtime atlas they are copied into, and the icon each roster row picks.
//!
//! The sheet is cut as 96-pixel square cells from one column, seven rows
//! deep, in the order [`SHEET_ROW`] gives: 384, 288, 192, 96, 0, 480 and
//! 576 pixels from the sheet's first row. Each cut is added to the runtime
//! icon atlas under id 1, so the first cut is handle 1 and the last handle
//! 7. The atlas starts with one 32-pixel cell and doubles its shorter side
//! as it fills (height first), ending at 128 pixels square: four columns
//! by four rows, of which the first seven cells hold these icons.
//!
//! Handle `rank + 1` is the icon for rank 0 (Inept) through 5 (Master);
//! handle 7 is the fallback icon for the "-" label past Master. A roster
//! row chooses by rank only: a specialist and the health bar take none,
//! and no profession or category changes the choice.
//!
//! Guess: the seven rows are the six ranks and the "-" fallback, in that
//! handle order; the sheet's own pixels are not read here. The texture is
//! loaded elsewhere; this module keeps no state and does no file I/O.

/// The sheet's file name.
pub const SHEET_NAME: &str = "arsenalico.rfi";
/// A source cell's side in pixels.
pub const SHEET_CELL: u32 = 96;
/// Columns of the sheet read by the construction.
pub const SHEET_COLUMNS: u32 = 1;
/// Rows of the sheet read by the construction.
pub const SHEET_ROWS: u32 = 7;
/// The icons the construction adds.
pub const ICON_COUNT: usize = 7;
/// A GUI unit in pixels.
pub const UNIT_PIXELS: u32 = 4;
/// A runtime atlas cell's side in pixels (eight GUI units).
pub const ICON_PIXELS: u32 = 32;
/// An icon's side in GUI units.
pub const ICON_UNITS: u32 = ICON_PIXELS / UNIT_PIXELS;
/// The runtime atlas's columns, rows and side in pixels.
pub const ATLAS_COLUMNS: u32 = 4;
pub const ATLAS_ROWS: u32 = 4;
pub const ATLAS_PIXELS: u32 = 128;
/// The sheet row each icon is cut from, in handle order.
pub const SHEET_ROW: [u32; ICON_COUNT] = [4, 3, 2, 1, 0, 5, 6];
/// The highest rank's index: Master.
pub const MASTER_RANK: usize = 5;
/// The icon for a label with no rank.
pub const FALLBACK_HANDLE: u32 = ICON_COUNT as u32;

/// A pixel rectangle: its corner and its size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// A texture rectangle: the sampled share of the atlas from each corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvRect {
    pub u: [f32; 2],
    pub v: [f32; 2],
}

/// The sheet cell an icon handle is cut from, in sheet pixels.
pub fn sheet_cell(handle: u32) -> Option<Rect> {
    let row = *SHEET_ROW.get(handle.checked_sub(1)? as usize)?;
    Some(Rect {
        x: 0,
        y: row * SHEET_CELL,
        w: SHEET_CELL,
        h: SHEET_CELL,
    })
}

/// An icon handle's cell in the runtime atlas, in atlas pixels.
pub fn cell_rect(handle: u32) -> Option<Rect> {
    let index = handle.checked_sub(1)?;
    if index as usize >= ICON_COUNT {
        return None;
    }
    Some(Rect {
        x: index % ATLAS_COLUMNS * ICON_PIXELS,
        y: index / ATLAS_COLUMNS * ICON_PIXELS,
        w: ICON_PIXELS,
        h: ICON_PIXELS,
    })
}

/// An icon handle's texture coordinates in the runtime atlas.
pub fn cell_uv(handle: u32) -> Option<UvRect> {
    let rect = cell_rect(handle)?;
    let u0 = rect.x as f32 / ATLAS_PIXELS as f32;
    let v0 = rect.y as f32 / ATLAS_PIXELS as f32;
    let du = ICON_PIXELS as f32 / ATLAS_PIXELS as f32;
    let dv = ICON_PIXELS as f32 / ATLAS_PIXELS as f32;
    Some(UvRect {
        u: [u0, u0 + du],
        v: [v0, v0 + dv],
    })
}

/// The handle of a rank's icon: ranks up to Master are `rank + 1`; any
/// other rank (the "-" label) takes the fallback.
pub fn icon_for_rank(rank: usize) -> u32 {
    if rank <= MASTER_RANK {
        rank as u32 + 1
    } else {
        FALLBACK_HANDLE
    }
}

/// What a roster or arsenal row shows, as far as its icon goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowIcon {
    /// A fighter's rank index; ranks past Master take the fallback.
    Rank(usize),
    /// A merchant, trainer or physician: no icon.
    Specialist,
    /// The health column: no icon (the original paints a bar).
    Health,
}

/// The icon a row picks: a fighter's rank icon, and none for a specialist
/// or the health column.
pub fn icon_for_row(row: RowIcon) -> Option<u32> {
    match row {
        RowIcon::Rank(rank) => Some(icon_for_rank(rank)),
        RowIcon::Specialist | RowIcon::Health => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sheet_is_one_column_of_seven_cells() {
        assert_eq!(
            (SHEET_NAME, SHEET_CELL, SHEET_COLUMNS, SHEET_ROWS),
            ("arsenalico.rfi", 96, 1, 7)
        );
        assert_eq!(SHEET_ROW, [4, 3, 2, 1, 0, 5, 6]);
        let rect = |handle| sheet_cell(handle).map(|r| (r.x, r.y, r.w, r.h));
        assert_eq!(rect(1), Some((0, 384, 96, 96)));
        assert_eq!(rect(2), Some((0, 288, 96, 96)));
        assert_eq!(rect(3), Some((0, 192, 96, 96)));
        assert_eq!(rect(4), Some((0, 96, 96, 96)));
        assert_eq!(rect(5), Some((0, 0, 96, 96)));
        assert_eq!(rect(6), Some((0, 480, 96, 96)));
        assert_eq!(rect(7), Some((0, 576, 96, 96)));
        assert_eq!(rect(0), None);
        assert_eq!(rect(8), None);
    }

    #[test]
    fn seven_icons_fill_the_first_cells_of_a_four_by_four_atlas() {
        assert_eq!((ATLAS_COLUMNS, ATLAS_ROWS, ATLAS_PIXELS), (4, 4, 128));
        assert_eq!(ATLAS_COLUMNS * ICON_PIXELS, ATLAS_PIXELS);
        assert_eq!(ATLAS_ROWS * ICON_PIXELS, ATLAS_PIXELS);
        let corner = |handle| cell_rect(handle).map(|r| (r.x, r.y));
        assert_eq!(corner(1), Some((0, 0)));
        assert_eq!(corner(2), Some((32, 0)));
        assert_eq!(corner(3), Some((64, 0)));
        assert_eq!(corner(4), Some((96, 0)));
        assert_eq!(corner(5), Some((0, 32)));
        assert_eq!(corner(6), Some((32, 32)));
        assert_eq!(corner(7), Some((64, 32)));
        assert_eq!(
            cell_rect(1),
            Some(Rect {
                x: 0,
                y: 0,
                w: 32,
                h: 32
            })
        );
        assert_eq!(cell_rect(0), None);
        assert_eq!(cell_rect(8), None);
    }

    #[test]
    fn texture_coordinates_follow_the_cell_order() {
        assert_eq!(
            cell_uv(1),
            Some(UvRect {
                u: [0.0, 0.25],
                v: [0.0, 0.25]
            })
        );
        assert_eq!(
            cell_uv(4),
            Some(UvRect {
                u: [0.75, 1.0],
                v: [0.0, 0.25]
            })
        );
        assert_eq!(
            cell_uv(5),
            Some(UvRect {
                u: [0.0, 0.25],
                v: [0.25, 0.5]
            })
        );
        assert_eq!(
            cell_uv(7),
            Some(UvRect {
                u: [0.5, 0.75],
                v: [0.25, 0.5]
            })
        );
        assert_eq!(cell_uv(8), None);
    }

    #[test]
    fn a_row_chooses_its_rank_icon() {
        assert_eq!(ICON_UNITS, 8);
        assert_eq!(
            (0..=5).map(icon_for_rank).collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6]
        );
        assert_eq!(icon_for_rank(6), FALLBACK_HANDLE);
        assert_eq!(icon_for_rank(usize::MAX), FALLBACK_HANDLE);
        assert_eq!(icon_for_row(RowIcon::Rank(0)), Some(1));
        assert_eq!(icon_for_row(RowIcon::Rank(5)), Some(6));
        assert_eq!(icon_for_row(RowIcon::Rank(6)), Some(7));
        assert_eq!(icon_for_row(RowIcon::Specialist), None);
        assert_eq!(icon_for_row(RowIcon::Health), None);
    }
}
