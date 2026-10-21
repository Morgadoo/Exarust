//! The arsenal icon sheet, `arsenalico.rfi`, as plain data: its cells and
//! the runtime atlas they are copied into.
//!
//! The sheet is cut as 96-pixel square cells from one column, seven rows
//! deep, in the order [`SHEET_ROW`] gives: 384, 288, 192, 96, 0, 480 and
//! 576 pixels from the sheet's first row. Each cut is scaled to a cell of
//! eight GUI units and added to the runtime icon atlas under id 1, so the
//! first cut is handle 1 and the last handle 7. The atlas starts with one
//! cell (the cell side rounded up to a power of two) and, when it is full,
//! doubles its height if it is as wide as it is tall and its width
//! otherwise; the cells are then laid out again in row order. Seven icons
//! therefore end in an atlas two cells wide and four tall (64 by 128
//! pixels) with the last cell free. [`atlas_size`] gives the size after a
//! number of icons.
//!
//! The image list is attached to the five shelf buttons of the arsenal
//! form and to eight controls of the board window. Which handle each draw
//! picks is not read here: the original's choice is a draw callback, and
//! no rank-to-icon table is assumed. The texture is loaded elsewhere; this
//! module keeps no state and does no file I/O.

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
/// The runtime atlas's columns, rows and size in pixels once the seven
/// icons are in.
pub const ATLAS_COLUMNS: u32 = 2;
pub const ATLAS_ROWS: u32 = 4;
pub const ATLAS_WIDTH: u32 = ATLAS_COLUMNS * ICON_PIXELS;
pub const ATLAS_HEIGHT: u32 = ATLAS_ROWS * ICON_PIXELS;
/// The sheet row each icon is cut from, in handle order.
pub const SHEET_ROW: [u32; ICON_COUNT] = [4, 3, 2, 1, 0, 5, 6];

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

/// The atlas's columns and rows after `count` icons: one cell, then
/// doubling the rows when the grid is square and the columns otherwise
/// each time it is full.
pub fn atlas_size(count: u32) -> (u32, u32) {
    let (mut columns, mut rows) = (1u32, 1u32);
    while columns * rows < count {
        if columns < rows {
            columns <<= 1;
        } else {
            rows <<= 1;
        }
    }
    (columns, rows)
}

/// An icon handle's cell in the runtime atlas, in atlas pixels, with the
/// seven icons in: cells run along the rows of a two-column grid.
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
    let u0 = rect.x as f32 / ATLAS_WIDTH as f32;
    let v0 = rect.y as f32 / ATLAS_HEIGHT as f32;
    let du = ICON_PIXELS as f32 / ATLAS_WIDTH as f32;
    let dv = ICON_PIXELS as f32 / ATLAS_HEIGHT as f32;
    Some(UvRect {
        u: [u0, u0 + du],
        v: [v0, v0 + dv],
    })
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
    fn the_atlas_grows_one_side_at_a_time() {
        let grid: Vec<_> = (1..=9).map(atlas_size).collect();
        assert_eq!(
            grid,
            [
                (1, 1),
                (1, 2),
                (2, 2),
                (2, 2),
                (2, 4),
                (2, 4),
                (2, 4),
                (2, 4),
                (4, 4)
            ]
        );
        assert_eq!(atlas_size(ICON_COUNT as u32), (ATLAS_COLUMNS, ATLAS_ROWS));
        assert_eq!((ATLAS_WIDTH, ATLAS_HEIGHT), (64, 128));
    }

    #[test]
    fn seven_icons_fill_a_two_by_four_atlas_row_by_row() {
        let corner = |handle| cell_rect(handle).map(|r| (r.x, r.y));
        assert_eq!(corner(1), Some((0, 0)));
        assert_eq!(corner(2), Some((32, 0)));
        assert_eq!(corner(3), Some((0, 32)));
        assert_eq!(corner(4), Some((32, 32)));
        assert_eq!(corner(5), Some((0, 64)));
        assert_eq!(corner(6), Some((32, 64)));
        assert_eq!(corner(7), Some((0, 96)));
        assert_eq!(
            cell_rect(1),
            Some(Rect {
                x: 0,
                y: 0,
                w: 32,
                h: 32
            })
        );
        assert_eq!(ICON_UNITS, 8);
        assert_eq!(cell_rect(0), None);
        assert_eq!(cell_rect(8), None);
    }

    #[test]
    fn texture_coordinates_follow_the_cell_order() {
        assert_eq!(
            cell_uv(1),
            Some(UvRect {
                u: [0.0, 0.5],
                v: [0.0, 0.25]
            })
        );
        assert_eq!(
            cell_uv(2),
            Some(UvRect {
                u: [0.5, 1.0],
                v: [0.0, 0.25]
            })
        );
        assert_eq!(
            cell_uv(7),
            Some(UvRect {
                u: [0.0, 0.5],
                v: [0.75, 1.0]
            })
        );
        assert_eq!(cell_uv(8), None);
    }
}
