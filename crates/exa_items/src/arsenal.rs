//! The arena's arsenal: the company's shared stores as five item columns
//! beside an inventory, with a borrowed item marked checked out in its
//! column until it comes back.
//!
//! Each column is 236 pixels wide, laid 60 GUI units apart (240 pixels at
//! the default unit of 4). As many as fit after a 12-unit margin (and 61
//! units for the inventory while it is shown) are shown, at most five,
//! scrolled by two arrows. An item dropped into a column from elsewhere is
//! looked up in the arena's sets: one of the current set clears the
//! checked-out mark of its copy and the drop adds nothing; one owned by
//! another set goes back to that set. Escape closes the arsenal, a right
//! button release closes it, and the inventory key shows or hides the
//! inventory.
//!
//! The form has eight controls: the two scroll arrows, five template
//! buttons between them, and the inventory toggle last.
//!
//! Evidence: the arsenal view's constructor, layout routine, close, key,
//! scroll and drop handlers, and the column frame's close (`docs/UI.md`).

/// The columns.
pub const COLUMNS: usize = 5;
/// A column's width in pixels, and its height in the normal and the compact
/// layout.
pub const COLUMN_WIDTH: i32 = 236;
pub const COLUMN_HEIGHT: i32 = 628;
pub const COLUMN_HEIGHT_COMPACT: i32 = 548;
/// The spacing of the columns, the margin and the inventory's share, in GUI
/// units (times the unit).
pub const COLUMN_STEP: i32 = 60;
pub const MARGIN: i32 = 12;
pub const INVENTORY_SHARE: i32 = 61;
/// A checked-out item's mark in its column, and in a drag payload.
pub const CHECKED_OUT: u32 = 0x200;
/// The columns' green and yellow highlight masks.
pub const HIGHLIGHT_GREEN: u32 = 0x400;
pub const HIGHLIGHT_YELLOW: u32 = 0x200;
/// The key that closes the arsenal.
pub const ESCAPE: u32 = 0x1B;
/// The inventory key's default (the letter `I`).
pub const INVENTORY_KEY: u32 = 0x49;

/// Where the arsenal's parts go. `area_width` and `area_height` are the form
/// values the layout starts from once the base layout has run, `width` is
/// the form's own width, and `unit` the GUI unit (a zero unit is taken as 1;
/// the original would divide by zero).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// How many columns are shown, and the first one shown.
    pub visible: i32,
    pub scroll: i32,
    /// Whether each scroll arrow is enabled.
    pub left_arrow: bool,
    pub right_arrow: bool,
    /// Each column's shown state and x inside the frame.
    pub columns: [(bool, i32); COLUMNS],
    pub frame_width: i32,
    /// The frame's and the inventory's places.
    pub frame: (i32, i32),
    pub inventory: (i32, i32),
    /// The controls' row: half the room above the inventory, less 8 units.
    pub top: i32,
    /// The left edge the row is centred on.
    pub left: i32,
}

/// The arsenal's layout. `inventory_height` is the embedded inventory's.
pub fn layout(
    width: i32,
    area_height: i32,
    area_width: i32,
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
    let top = (area_height - inventory_height) / 2 - 8 * unit;
    let left = (area_width - (COLUMN_STEP * visible + 3 + share) * unit) / 2;
    Layout {
        visible,
        scroll,
        left_arrow,
        right_arrow,
        columns,
        frame_width: 3 * unit + COLUMN_STEP * unit * visible,
        frame: (share * unit + left, 12 * unit + top),
        inventory: (left - 4 * unit, 12 * unit + top),
        top,
        left,
    }
}

/// A scroll button's press (mask bit 2 set): `0x1002` scrolls left (down to
/// 0), `0x2002` right; the layout then clamps it. Without bit 2 the base
/// form handles the event and the scroll is unchanged.
pub fn scroll_by(mask: u32, scroll: i32) -> i32 {
    if mask & 2 == 0 {
        return scroll;
    }
    match mask & 0xF002 {
        0x1002 if scroll > 0 => scroll - 1,
        0x2002 => scroll + 1,
        _ => scroll,
    }
}

/// A key press: Escape closes, the inventory key flips the inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Close,
    /// Show the inventory when it is hidden, hide it when shown.
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

/// A mouse release closes the arsenal when both bit 1 and the `0x2000`
/// button bit are set (a right-button release); the event is then
/// handled. Anything else goes to the base form.
pub fn closes_on_release(mask: u32) -> bool {
    mask & 0x2002 == 0x2002
}

/// What a drag payload hook on a column does: a payload whose source list
/// sits in the arsenal's frame (it came from a column) passes untouched and
/// the hook says yes. Any other payload is marked [`CHECKED_OUT`], the
/// item's icon takes another reference, and the hook says no. Which event
/// calls the hook is not traced.
pub fn column_hook(source_in_frame: bool, flags: &mut u32, add_ref: &mut dyn FnMut()) -> bool {
    if source_in_frame {
        return true;
    }
    *flags |= CHECKED_OUT;
    add_ref();
    false
}

/// The embedded inventory's drop hook: it takes a payload only while the
/// payload is not marked [`CHECKED_OUT`].
pub fn inventory_accepts(flags: u32) -> bool {
    flags & CHECKED_OUT == 0
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
/// any; `marked` finds a checked-out copy of the item in a column (the
/// drop's own column is `target`); `roster_marked` says the arena's set
/// lists it as checked out.
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

/// The column frame's initial size and place: 972 wide ((236 + 4) × 4 +
/// 12), 644 tall (564 in the compact layout), at (264, 48).
pub const FRAME_WIDTH: i32 = (COLUMN_WIDTH + 4) * 4 + 12;
pub const FRAME_HEIGHT: i32 = 0x284;
pub const FRAME_HEIGHT_COMPACT: i32 = 0x234;
pub const FRAME_X: i32 = 0x108;
pub const FRAME_Y: i32 = 0x30;
/// The window tag the constructor sets.
pub const FORM_TAG: u32 = 0xce1c_2024;
/// The highest control index the constructor asks for: the form has eight
/// controls, the two arrows, five template buttons and the inventory
/// toggle.
pub const LAST_CONTROL: usize = 7;
/// The inventory is built at this y.
pub const INVENTORY_Y: i32 = 0x30;
/// The inventory pack's drop mask the constructor sets.
pub const PACK_DROP_MASK: u32 = 0x200;
/// The frame's fixed width share, in GUI units.
pub const FRAME_EDGE: i32 = 3;
/// The arrows' offset from the frame's ends and the template row spacing,
/// in GUI units.
pub const ARROW_WIDTH_UNITS: i32 = 6;
pub const TEMPLATE_STEP_UNITS: i32 = 30;

/// The column frame as the constructor sizes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArsenalForm {
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
    /// The last control index the form's table is loaded to.
    pub last_control: usize,
    /// The inventory starts shown.
    pub inventory_shown: bool,
    pub compact: bool,
}

/// The constructor's values for the compact-layout global.
pub fn arsenal_form(compact: bool) -> ArsenalForm {
    ArsenalForm {
        width: FRAME_WIDTH,
        height: if compact {
            FRAME_HEIGHT_COMPACT
        } else {
            FRAME_HEIGHT
        },
        x: FRAME_X,
        y: FRAME_Y,
        last_control: LAST_CONTROL,
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

/// The five columns as the constructor places them: 236 wide, 628 tall (548
/// compact), (236 + 4) apart, the first at zero. The layout later moves
/// them to `unit × 60` apart.
pub fn column_places(compact: bool) -> [ColumnPlace; COLUMNS] {
    std::array::from_fn(|i| ColumnPlace {
        x: (COLUMN_WIDTH + 4) * i as i32,
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

/// The controls' places the layout sets, as the 16-bit values the control
/// rows keep. Every control sits on the layout's `top` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArsenalControls {
    /// Control 0, the left arrow.
    pub left_arrow: [i16; 2],
    /// Control 1, the right arrow.
    pub right_arrow: [i16; 2],
    /// Controls 2 up to the last but one: control index and place.
    pub template: Vec<(usize, [i16; 2])>,
    /// The last control, the inventory toggle.
    pub toggle: [i16; 2],
}

/// The control places for a laid-out arsenal. `last_control` is the form's
/// last control index; the template runs from control 2 to the one before
/// it.
pub fn arsenal_controls(
    layout: &Layout,
    unit: i32,
    inventory_shown: bool,
    last_control: usize,
) -> ArsenalControls {
    let top = layout.top as i16;
    let left_x = layout.frame.0 - unit * 2 + if inventory_shown { 0 } else { unit * 5 };
    let span = layout.visible * COLUMN_STEP;
    let right_x = (span - ARROW_WIDTH_UNITS) * unit + layout.frame.0;
    let row_shift = (span + FRAME_EDGE) / 2 * unit - unit * 5;
    let template = (2..last_control)
        .map(|e| {
            let x = (e as i32 - 4) * unit * TEMPLATE_STEP_UNITS + layout.frame.0 + row_shift;
            (e, [x as i16, top])
        })
        .collect();
    ArsenalControls {
        left_arrow: [left_x as i16, top],
        right_arrow: [right_x as i16, top],
        template,
        toggle: [(layout.left - unit * ARROW_WIDTH_UNITS) as i16, top],
    }
}

/// What closing the arsenal does, in the original's order: the base form
/// closes first, then the controller is told, the columns are saved, and
/// the arena window is closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArsenalClose {
    /// The player controller (when there is one) is told the window shut.
    pub notify_controller: bool,
    /// The five columns are written into the set's store, only for a set
    /// that is chosen (not negative).
    pub save_columns: bool,
    /// With an arena present its mode becomes this and its window closes.
    pub arena_mode: Option<i32>,
}

/// The close for the chosen `set` (negative when none), whether a player
/// controller exists and whether the arena is present.
pub fn arsenal_close(set: i32, controller: bool, arena_present: bool) -> ArsenalClose {
    ArsenalClose {
        notify_controller: controller,
        save_columns: set >= 0,
        arena_mode: arena_present.then_some(ARENA_MODE_AFTER_CLOSE),
    }
}

/// The arena mode the close leaves behind.
pub const ARENA_MODE_AFTER_CLOSE: i32 = 3;

/// The column frame's close: it closes its parent, the arsenal.
pub fn cont_frame_close(parent: u64) -> u64 {
    parent
}

/// The inventory toggle's two control flag bits: shown sets them.
pub const TOGGLE_BITS: u32 = 0x60;

/// The toggle control's flags after the inventory is shown or hidden; the
/// arsenal then lays out again.
pub fn toggle_flags(flags: u32, shown: bool) -> u32 {
    if shown {
        flags | TOGGLE_BITS
    } else {
        flags & !TOGGLE_BITS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_columns_fit_the_window() {
        // 1280 wide at unit 4: (1280 - 48 - 244) / 240 = 4 columns.
        let l = layout(1280, 900, 1280, 590, 4, true, 3);
        assert_eq!(l.visible, 4);
        assert_eq!(l.scroll, 1);
        assert!(l.left_arrow);
        assert!(!l.right_arrow);
        assert_eq!(l.columns[0], (false, -240));
        assert_eq!(l.columns[1], (true, 0));
        assert_eq!(l.columns[4], (true, 720));
        assert_eq!(l.frame_width, 12 + 960);
        // top = (900 - 590) / 2 - 32 = 123; left = (1280 - 304 * 4) / 2 = 32.
        assert_eq!((l.top, l.left), (123, 32));
        assert_eq!(l.frame, (244 + 32, 48 + 123));
        assert_eq!(l.inventory, (16, 48 + 123));
        let wide = layout(2560, 900, 2560, 590, 4, false, 0);
        assert_eq!(wide.visible, 5);
        assert!(!wide.left_arrow && !wide.right_arrow);
        // A scroll past the end is pulled back; the right arrow ends.
        let back = layout(2560, 900, 2560, 590, 4, false, 3);
        assert_eq!(back.scroll, 0);
    }

    #[test]
    fn scrolling_keys_and_release() {
        assert_eq!(scroll_by(0x1002, 0), 0);
        assert_eq!(scroll_by(0x1002, 2), 1);
        assert_eq!(scroll_by(0x2002, 2), 3);
        assert_eq!(scroll_by(0x0002, 2), 2);
        assert_eq!(scroll_by(0x2000, 2), 2);
        assert_eq!(key(ESCAPE, 0, INVENTORY_KEY), Key::Close);
        assert_eq!(key(0x49, 0, INVENTORY_KEY), Key::ToggleInventory);
        assert_eq!(key(ESCAPE, 2, INVENTORY_KEY), Key::None);
        assert_eq!(key(0x41, 0, INVENTORY_KEY), Key::None);
        assert_eq!(store_slot(2, 3), 13);
        assert!(closes_on_release(0x2002));
        assert!(closes_on_release(0x2012));
        assert!(!closes_on_release(0x2000));
        assert!(!closes_on_release(0x0002));
    }

    #[test]
    fn borrowing_marks_the_item_and_keeps_it() {
        let mut flags = 0x10;
        let mut refs = 0;
        // A payload from outside the frame is marked and takes a reference.
        assert!(!column_hook(false, &mut flags, &mut || refs += 1));
        assert_eq!((flags, refs), (0x210, 1));
        // One from a column passes untouched.
        let mut flags = 0x10;
        assert!(column_hook(true, &mut flags, &mut || refs += 1));
        assert_eq!((flags, refs), (0x10, 1));
        // The inventory refuses what is checked out.
        assert!(inventory_accepts(0x10));
        assert!(!inventory_accepts(0x210));
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
        // The drop's own column is searched first.
        let d = drop_in(false, true, Some(1), 1, 2, |c| Some(c + 10), false);
        assert_eq!(d.cleared, Some((2, 12)));
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
        assert_eq!(f.last_control, 7);
        assert!(f.inventory_shown);
        let cols = column_places(false);
        assert_eq!(cols[0].x, 0);
        assert_eq!(cols[4].x, 960);
        assert_eq!((cols[0].width, cols[0].height), (236, 628));
        assert_eq!(column_places(true)[0].height, 548);
        assert_eq!((cols[0].green, cols[0].yellow), (0x400, 0x200));
    }

    #[test]
    fn the_layout_places_the_arrows_and_template_rows() {
        let l = layout(1280, 900, 1280, 590, 4, true, 3);
        let c = arsenal_controls(&l, 4, true, LAST_CONTROL);
        // Everything sits on the top row, not the frame's y.
        assert_eq!(c.left_arrow, [268, 123]);
        // (240 - 6) × 4 + 276.
        assert_eq!(c.right_arrow, [1212, 123]);
        // The row shift is 4 × ((240 + 3) / 2) - 20 = 464; controls 2..=6
        // are 120 apart from (2 - 4) × 120 + 276 + 464 = 500.
        let xs: Vec<_> = c.template.iter().map(|t| (t.0, t.1[0])).collect();
        assert_eq!(xs, [(2, 500), (3, 620), (4, 740), (5, 860), (6, 980)]);
        assert_eq!(c.toggle, [32 - 24, 123]);
        // The hidden inventory moves the left arrow five units right.
        let hidden = layout(1280, 900, 1280, 590, 4, false, 3);
        let c = arsenal_controls(&hidden, 4, false, LAST_CONTROL);
        assert_eq!(c.left_arrow[0] as i32, hidden.frame.0 - 8 + 20);
    }

    #[test]
    fn closing_and_toggling() {
        let c = arsenal_close(2, true, true);
        assert!(c.save_columns && c.notify_controller);
        assert_eq!(c.arena_mode, Some(3));
        // No set chosen: the columns are not saved.
        assert!(!arsenal_close(-1, true, true).save_columns);
        // No controller: nobody is told; no arena: no mode change.
        let c = arsenal_close(0, false, false);
        assert!(c.save_columns && !c.notify_controller);
        assert_eq!(c.arena_mode, None);
        assert_eq!(toggle_flags(0, true), 0x60);
        assert_eq!(toggle_flags(0x60, false), 0);
        assert_eq!(toggle_flags(0x1, true), 0x61);
        assert_eq!(cont_frame_close(9), 9);
    }
}
