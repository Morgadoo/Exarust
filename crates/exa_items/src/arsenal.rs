//! The arena's arsenal: the company's shared stores as five item columns
//! beside an inventory, with a borrowed item marked checked out in its
//! column until it comes back.
//!
//! Each column is 236 units wide; as many as fit after a 12-unit margin
//! (and 61 units for the inventory when it is shown) are shown, 60 units
//! apart, at most five, scrolled by the arrow buttons. Dragging an item out
//! of the columns leaves it in its column marked `0x200` and holds another
//! reference to its icon. An item dropped into a column from elsewhere is
//! looked up in the arena's sets: one of the current set clears the
//! checked-out mark of its copy and the drop adds nothing; one owned by
//! another set goes back to that set. Escape closes the arsenal and the
//! inventory key shows or hides the inventory. Evidence: the arsenal notes
//! (`context/re/item-gui-lane.md`).

/// The columns.
pub const COLUMNS: usize = 5;
/// A column's width, in GUI units, and its height in the normal and the
/// compact layout.
pub const COLUMN_WIDTH: i32 = 236;
pub const COLUMN_HEIGHT: i32 = 628;
pub const COLUMN_HEIGHT_COMPACT: i32 = 548;
/// The spacing of the columns, the margin and the inventory's share, in GUI
/// units (times the unit).
pub const COLUMN_STEP: i32 = 60;
pub const MARGIN: i32 = 12;
pub const INVENTORY_SHARE: i32 = 61;
/// A checked-out item's mark in its column.
pub const CHECKED_OUT: u32 = 0x200;
/// The columns' green and yellow highlight masks.
pub const HIGHLIGHT_GREEN: u32 = 0x400;
pub const HIGHLIGHT_YELLOW: u32 = 0x200;
/// The key that closes the arsenal.
pub const ESCAPE: u32 = 0x1B;

/// Where the arsenal's parts go for a window of `width` by `height` pixels
/// at GUI unit `unit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// How many columns are shown, and the first one shown.
    pub visible: i32,
    pub scroll: i32,
    pub left_arrow: bool,
    pub right_arrow: bool,
    /// Each column's shown state and x inside the frame.
    pub columns: [(bool, i32); COLUMNS],
    pub frame_width: i32,
    pub frame: (i32, i32),
    pub inventory: (i32, i32),
}

/// The arsenal's layout. `inventory_height` is the embedded inventory's.
pub fn layout(
    width: i32,
    height: i32,
    client_width: i32,
    inventory_height: i32,
    unit: i32,
    inventory_shown: bool,
    scroll: i32,
) -> Layout {
    let unit = unit.max(1);
    let share = if inventory_shown { INVENTORY_SHARE } else { 0 };
    let room = width - MARGIN * unit - share * unit;
    let visible = (room / (unit * COLUMN_STEP)).min(COLUMNS as i32);
    let scroll = scroll.min(COLUMNS as i32 - visible);
    let left_arrow = scroll >= 1;
    let right_arrow = scroll + visible < COLUMNS as i32;
    let columns = std::array::from_fn(|i| {
        let i = i as i32;
        (
            i >= scroll && i < scroll + visible,
            unit * COLUMN_STEP * (i - scroll),
        )
    });
    let top = (height - inventory_height) / 2 - 8 * unit;
    let left = (client_width - (COLUMN_STEP * visible + 3 + share) * unit) / 2;
    Layout {
        visible,
        scroll,
        left_arrow,
        right_arrow,
        columns,
        frame_width: 3 * unit + COLUMN_STEP * unit * visible,
        frame: (share * unit + left, 12 * unit + top),
        inventory: (left - 4 * unit, 12 * unit + top),
    }
}

/// A scroll button's press: `0x1002` scrolls left (down to 0), `0x2002`
/// right; the layout then clamps it.
pub fn scroll_by(mask: u32, scroll: i32) -> i32 {
    match mask & 0xF002 {
        0x1002 if scroll > 0 => scroll - 1,
        0x2002 => scroll + 1,
        _ => scroll,
    }
}

/// A key press: Escape closes, the inventory key toggles the inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Close,
    ToggleInventory,
    None,
}

/// What a key does; releases (`flags` bit 2) do nothing.
pub fn key(code: u32, flags: u32, inventory_key: u32) -> Key {
    if flags & 2 != 0 {
        Key::None
    } else if code == ESCAPE {
        Key::Close
    } else if code == inventory_key {
        Key::ToggleInventory
    } else {
        Key::None
    }
}

/// A drag leaving a column: to anywhere outside the columns' frame the item
/// stays, marked [`CHECKED_OUT`], and its icon takes another reference;
/// inside the frame it moves as usual. Returns whether the column lets it
/// go.
pub fn drag_out(flags: &mut u32, into_frame: bool, add_ref: &mut dyn FnMut()) -> bool {
    if into_frame {
        return true;
    }
    *flags |= CHECKED_OUT;
    add_ref();
    false
}

/// What dropping an item into a column from elsewhere does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DropIn {
    /// Tell the source the drag ended (the item has an owning set).
    pub notify_source: bool,
    /// The checked-out copy whose mark was cleared: the drop's column first,
    /// then the columns in order.
    pub cleared: Option<(usize, usize)>,
    /// Return the item to its owning set.
    pub return_to_owner: bool,
    /// The drop adds nothing: its icon reference is dropped.
    pub consumed: bool,
}

/// The column drop rule. `owner` is the set the arena gives the item, if
/// any; `marked` finds a checked-out copy of the item in a column;
/// `roster_marked` says the arena's set lists it as checked out.
pub fn drop_in<F>(
    from_column: bool,
    has_source: bool,
    owner: Option<u32>,
    current: u32,
    target: usize,
    mut marked: F,
    roster_marked: bool,
) -> DropIn
where
    F: FnMut(usize) -> Option<usize>,
{
    let mut out = DropIn::default();
    if from_column {
        return out;
    }
    let Some(owner) = owner else {
        out.consumed = true;
        return out;
    };
    out.notify_source = has_source;
    if owner == current {
        let found = marked(target)
            .map(|e| (target, e))
            .or_else(|| (0..COLUMNS).find_map(|c| marked(c).map(|e| (c, e))));
        if let Some(at) = found {
            out.cleared = Some(at);
            out.consumed = true;
            return out;
        }
    }
    if roster_marked {
        out.consumed = true;
    } else if owner != current {
        out.return_to_owner = true;
        out.consumed = true;
    }
    out
}

/// The arena store slot of a set's column: five per set.
pub fn store_slot(set: u32, column: usize) -> usize {
    set as usize * COLUMNS + column
}

/// The arsenal window's size, place and tags.
pub const FORM_TAG: u32 = 0xce1c_2024;
pub const FORM_WIDTH: i32 = 0x3cc;
pub const FORM_HEIGHT: i32 = 0x284;
pub const FORM_HEIGHT_COMPACT: i32 = 0x234;
pub const FORM_X: i32 = 0x108;
pub const FORM_Y: i32 = 0x30;
/// The number of form controls the constructor builds.
pub const FORM_CONTROLS: usize = 7;
/// The inventory's extra flag and dock.
pub const INVENTORY_FLAGS: u32 = 0x10;
pub const INVENTORY_DOCK: i32 = 0x30;
/// The pack panel's drop mask.
pub const PACK_DROP_MASK: u32 = 0x200;
/// The frame's fixed width share.
pub const FRAME_EDGE: i32 = 3;
/// The template arrow controls' widths and the control row spacing.
pub const ARROW_WIDTH_UNITS: i32 = 6;
pub const TEMPLATE_STEP_UNITS: i32 = 30;

/// The arsenal's constructor data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArsenalForm {
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
    pub form_controls: usize,
    pub inventory_shown: bool,
    pub compact: bool,
}

/// The constructor's values: 972 wide, 644 (or 564 compact) tall at
/// 264, 48, with seven form controls and the inventory shown.
pub fn arsenal_form(compact: bool) -> ArsenalForm {
    ArsenalForm {
        width: FORM_WIDTH,
        height: if compact {
            FORM_HEIGHT_COMPACT
        } else {
            FORM_HEIGHT
        },
        x: FORM_X,
        y: FORM_Y,
        form_controls: FORM_CONTROLS,
        inventory_shown: true,
        compact,
    }
}

/// One column control's place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnPlace {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub green: u32,
    pub yellow: u32,
}

/// The five columns as the constructor places them: 236 by 628 (548
/// compact), 60 units apart, the first at zero.
pub fn column_places(compact: bool, unit: i32) -> [ColumnPlace; COLUMNS] {
    std::array::from_fn(|i| ColumnPlace {
        x: unit.wrapping_mul(COLUMN_STEP).wrapping_mul(i as i32),
        y: 0,
        width: COLUMN_WIDTH,
        height: if compact {
            COLUMN_HEIGHT_COMPACT
        } else {
            COLUMN_HEIGHT
        },
        green: HIGHLIGHT_GREEN,
        yellow: HIGHLIGHT_YELLOW,
    })
}

/// The arrows' and template controls' places the layout routine sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArsenalControls {
    /// The left arrow's control point.
    pub left_arrow: [i32; 2],
    /// The right arrow's control point.
    pub right_arrow: [i32; 2],
    /// The template rows: extra controls between the arrows.
    pub template: Vec<[i32; 2]>,
}

/// The control points for a laid-out arsenal.
pub fn arsenal_controls(
    layout: &Layout,
    client_width: i32,
    unit: i32,
    form_controls: usize,
) -> ArsenalControls {
    let _ = client_width;
    let top = layout.frame.1;
    // The frame's left is the inventory's left plus four units; the
    // inventory's share shows in the distance between them.
    let inventory_shown = layout.frame.0.wrapping_sub(layout.inventory.0) != unit.wrapping_mul(4);
    let left_x = layout
        .frame
        .0
        .wrapping_sub(unit.wrapping_mul(2))
        .wrapping_add(if inventory_shown {
            0
        } else {
            unit.wrapping_mul(5)
        });
    let visible_span = layout.visible.wrapping_mul(COLUMN_STEP);
    let right_x = visible_span
        .wrapping_sub(ARROW_WIDTH_UNITS)
        .wrapping_mul(unit)
        .wrapping_add(layout.frame.0);
    let row_shift = unit
        .wrapping_mul(visible_span.wrapping_add(FRAME_EDGE) / 2)
        .wrapping_sub(unit.wrapping_mul(5));
    let mut template = Vec::new();
    if form_controls > 2 {
        for i in 1..form_controls - 1 {
            let x = (i as i32 - 3)
                .wrapping_mul(unit.wrapping_mul(TEMPLATE_STEP_UNITS))
                .wrapping_add(layout.frame.0)
                .wrapping_add(row_shift);
            template.push([x, top]);
        }
    }
    ArsenalControls {
        left_arrow: [left_x, top],
        right_arrow: [right_x, top],
        template,
    }
}

/// What closing the arsenal does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArsenalClose {
    /// The five columns are written into the set's save words.
    pub save_columns: bool,
    /// The player controller is told.
    pub notify: bool,
    /// The arena window is closed and the mode set to 3.
    pub arena_mode: Option<i32>,
}

pub fn arsenal_close(set: i32, arena_present: bool) -> ArsenalClose {
    let _ = set;
    ArsenalClose {
        save_columns: true,
        notify: true,
        arena_mode: arena_present.then_some(3),
    }
}

/// The plain frame's only method: close its parent arsenal.
pub fn cont_frame_close(parent: u64) -> u64 {
    parent
}

/// The toggle's control flags: the inventory shown sets `0x60`.
pub fn toggle_flags(flags: u32, shown: bool) -> u32 {
    if shown { flags | 0x60 } else { flags & !0x60 }
}

/// A drag out of a column: only the arsenal's own inventory lets an item
/// go without the checked-out mark.
pub fn column_drag_out(
    source_is_inventory: bool,
    flags: &mut u32,
    add_ref: &mut dyn FnMut(),
) -> bool {
    if source_is_inventory {
        return true;
    }
    *flags |= CHECKED_OUT;
    add_ref();
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_columns_fit_the_window() {
        // 1280 wide at unit 4: (1280 − 48 − 244) / 240 = 4 columns.
        let l = layout(1280, 900, 1280, 590, 4, true, 3);
        assert_eq!(l.visible, 4);
        assert_eq!(l.scroll, 1);
        assert!(l.left_arrow);
        assert!(!l.right_arrow);
        assert_eq!(l.columns[0], (false, -240));
        assert_eq!(l.columns[1], (true, 0));
        assert_eq!(l.columns[4], (true, 720));
        assert_eq!(l.frame_width, 12 + 960);
        // (1280 − (240 + 3 + 61)·4) / 2 = 32.
        assert_eq!(l.frame, (244 + 32, 48 + 155 - 32));
        assert_eq!(l.inventory, (16, 48 + 123));
        let wide = layout(2560, 900, 2560, 590, 4, false, 0);
        assert_eq!(wide.visible, 5);
        assert!(!wide.left_arrow && !wide.right_arrow);
    }

    #[test]
    fn scrolling_and_keys() {
        assert_eq!(scroll_by(0x1002, 0), 0);
        assert_eq!(scroll_by(0x1002, 2), 1);
        assert_eq!(scroll_by(0x2002, 2), 3);
        assert_eq!(scroll_by(0x0002, 2), 2);
        assert_eq!(key(ESCAPE, 0, 0x49), Key::Close);
        assert_eq!(key(0x49, 0, 0x49), Key::ToggleInventory);
        assert_eq!(key(ESCAPE, 2, 0x49), Key::None);
        assert_eq!(store_slot(2, 3), 13);
    }

    #[test]
    fn borrowing_marks_the_item_and_keeps_it() {
        let mut flags = 0x10;
        let mut refs = 0;
        assert!(!drag_out(&mut flags, false, &mut || refs += 1));
        assert_eq!((flags, refs), (0x210, 1));
        assert!(drag_out(&mut flags, true, &mut || refs += 1));
        assert_eq!(refs, 1);
    }

    #[test]
    fn returning_an_item() {
        // From a column: nothing more.
        assert_eq!(
            drop_in(true, true, Some(1), 1, 0, |_| Some(0), false),
            DropIn::default()
        );
        // No owning set: consumed.
        assert!(drop_in(false, true, None, 1, 0, |_| None, false).consumed);
        // Back to its own set: the marked copy in column 3 is cleared.
        let d = drop_in(false, true, Some(1), 1, 0, |c| (c == 3).then_some(7), false);
        assert_eq!(d.cleared, Some((3, 7)));
        assert!(d.consumed && d.notify_source);
        // Another set's item goes back to it.
        let d = drop_in(false, false, Some(2), 1, 0, |_| None, false);
        assert!(d.return_to_owner && d.consumed && !d.notify_source);
        // The arena still lists it as checked out: consumed, not returned.
        let d = drop_in(false, true, Some(2), 1, 0, |_| None, true);
        assert!(d.consumed && !d.return_to_owner);
        // The current set's item without a marked copy simply lands.
        let d = drop_in(false, true, Some(1), 1, 0, |_| None, false);
        assert!(!d.consumed && !d.return_to_owner);
    }

    #[test]
    fn the_constructor_values_and_column_places() {
        let f = arsenal_form(false);
        assert_eq!((f.width, f.height, f.x, f.y), (972, 644, 264, 48));
        assert_eq!(arsenal_form(true).height, 564);
        assert_eq!(f.form_controls, 7);
        assert!(f.inventory_shown);
        let cols = column_places(false, 4);
        assert_eq!(cols[0].x, 0);
        assert_eq!(cols[4].x, 960);
        assert_eq!((cols[0].width, cols[0].height), (236, 628));
        assert_eq!(column_places(true, 4)[0].height, 548);
        assert_eq!((cols[0].green, cols[0].yellow), (0x400, 0x200));
    }

    #[test]
    fn the_layout_places_the_arrows_and_template_rows() {
        let l = layout(1280, 900, 1280, 590, 4, true, 3);
        let c = arsenal_controls(&l, 1280, 4, FORM_CONTROLS);
        assert_eq!(c.left_arrow, [268, l.frame.1]);
        assert_eq!(c.right_arrow, [1212, l.frame.1]);
        assert_eq!(c.template.len(), 5);
        assert_eq!(c.template[0], [500, l.frame.1]);
        // The hidden inventory moves the left arrow five units right.
        let hidden = layout(1280, 900, 1280, 590, 4, false, 3);
        let c = arsenal_controls(&hidden, 1280, 4, FORM_CONTROLS);
        assert_eq!(c.left_arrow[0], hidden.frame.0 - 8 + 20);
    }

    #[test]
    fn closing_and_toggling() {
        let c = arsenal_close(2, true);
        assert!(c.save_columns && c.notify);
        assert_eq!(c.arena_mode, Some(3));
        assert_eq!(arsenal_close(0, false).arena_mode, None);
        assert_eq!(toggle_flags(0, true), 0x60);
        assert_eq!(toggle_flags(0x60, false), 0);
        let mut flags = 0;
        let mut refs = 0;
        assert!(!column_drag_out(false, &mut flags, &mut || refs += 1));
        assert_eq!((flags, refs), (CHECKED_OUT, 1));
        assert!(column_drag_out(true, &mut flags, &mut || refs += 1));
        assert_eq!(refs, 1);
        assert_eq!(cont_frame_close(9), 9);
    }
}
