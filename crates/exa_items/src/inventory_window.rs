//! The inventory window: a character's doll beside its pack, the two
//! linked so that a move between them is not counted against the pack.
//!
//! The window is 244 units wide and 644 tall (628 in the compact layout,
//! 590 inside the arena's arsenal), with an 8-unit border. The doll fills
//! the width inside the border and all but 260 units of the height; the
//! pack below it is 228 by 180 with a capacity of 400 and items up to 50.
//! What the character wears counts against the pack's capacity. A drag out
//! of the pack is refused while the body swaps its weapon sets, and while a
//! mind-controlled character is shown. Changing the player's pack sets the
//! compass flag when it holds a usable item of kind 4, sort 11. Evidence:
//! the inventory window notes (`context/re/item-gui-lane.md`).

use crate::possession::{HELD, Possessions};

/// The window's width, border and title.
pub const WIDTH: i32 = 244;
pub const BORDER: i32 = 8;
pub const TITLE: &str = "Inventory";
/// The window's height in the normal layout, the compact one, and inside
/// the arsenal.
pub const HEIGHT: i32 = 644;
pub const HEIGHT_COMPACT: i32 = 628;
pub const HEIGHT_EMBEDDED: i32 = 590;
/// The height the doll leaves to the pack and the tabs.
pub const BELOW_DOLL: i32 = 260;
/// The pack's size, capacity and largest item.
pub const PACK_WIDTH: i32 = 228;
pub const PACK_HEIGHT: i32 = 180;
pub const PACK_CAPACITY: u32 = 400;
pub const PACK_MAX_ITEM: u32 = 50;
/// The body state of a weapon-set swap, during which nothing is dragged out
/// of the pack.
pub const SWAP_STATE: u32 = 0x1F;
/// The usable section's kind and sort of the compass.
pub const COMPASS: [u8; 2] = [4, 11];
/// The mouse mask of a right-button release, which closes the window.
pub const RIGHT_RELEASE: u16 = 0x2002;
/// Window ids registered for an inventory carry this bit.
pub const INVENTORY_WINDOW: u32 = 0x2000_0000;
/// The doll's drop slot that does not hand the item back to the pack.
pub const KEEP_SLOT: i32 = 8;

/// Which layout the window is built in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Normal,
    /// The global compact flag. [what sets it is not traced]
    Compact,
    /// Inside the arena's arsenal.
    Embedded,
}

/// Where the window's parts sit, in GUI units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub width: i32,
    pub height: i32,
    pub doll: (i32, i32),
    pub pack: (i32, i32),
}

/// The window's geometry for a layout.
pub fn geometry(layout: Layout) -> Geometry {
    let height = match layout {
        Layout::Normal => HEIGHT,
        Layout::Compact => HEIGHT_COMPACT,
        Layout::Embedded => HEIGHT_EMBEDDED,
    };
    Geometry {
        width: WIDTH,
        height,
        doll: (WIDTH - 2 * BORDER, height - BELOW_DOLL),
        pack: (PACK_WIDTH, PACK_HEIGHT),
    }
}

/// Whether a drag may leave the pack. A mind-controlled character's pack
/// closes the window instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragOut {
    Allowed,
    Refused,
    /// Close the window and refuse.
    Close,
}

/// The pack's drag-out rule. `from_pack` says the drag starts in the pack;
/// `controlled` that the doll shows a character under the player's mind
/// control; `player_state` is the player's body state.
pub fn drag_out(doll_live: bool, from_pack: bool, controlled: bool, player_state: u32) -> DragOut {
    if doll_live && from_pack && controlled {
        DragOut::Close
    } else if player_state == SWAP_STATE {
        DragOut::Refused
    } else {
        DragOut::Allowed
    }
}

/// Whether a carried list holds a compass: any item whose usable section
/// starts with [`COMPASS`].
pub fn carries_compass<F>(p: &Possessions, usable: F) -> bool
where
    F: Fn(u32) -> Option<[u8; 2]>,
{
    p.carried.iter().any(|c| usable(c.id) == Some(COMPASS))
}

/// The pack-change callback's two duties: save the doll's pack when its
/// view is visible, and, for the player's own pack, update the compass
/// flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackChanged {
    pub save_pack: bool,
    pub compass: bool,
}

pub fn pack_changed(view_visible: bool, is_player: bool) -> PackChanged {
    PackChanged {
        save_pack: view_visible,
        compass: is_player,
    }
}

/// The pack's used capacity once bound to a character: the bulk of every
/// listed item, held ones too, plus that of everything it wears.
pub fn used_capacity<F>(p: &Possessions, bulk: F) -> u32
where
    F: Fn(u32) -> u32,
{
    let carried: u32 = p.carried.iter().map(|c| bulk(c.id)).sum();
    carried + p.worn.iter().map(|&id| bulk(id)).sum::<u32>()
}

/// Which tab the window opens on: the first set's unless the second is
/// out.
pub fn second_tab(p: &Possessions) -> bool {
    p.flags & crate::possession::SECOND_SET != 0
}

/// The doll's camera distance for a character of `height` (metres).
pub fn camera_distance(height: f32) -> f32 {
    height * 10.0
}

/// A drop onto the doll from anywhere but the pack takes the item's last
/// held entry off the list; the record is then told the item's count
/// changed (its role checks, not modelled). A cancelled drag, or one whose
/// cursor is still over the doll or the pack, does nothing. Returns whether
/// an entry left the list.
pub fn drop_on_doll(
    p: &mut Possessions,
    id: u32,
    from_pack: bool,
    cancelled: bool,
    over_window: bool,
) -> bool {
    if cancelled || over_window || from_pack {
        return false;
    }
    if let Some(at) = p
        .carried
        .iter()
        .rposition(|c| c.id == id && c.flags & HELD != 0)
    {
        p.carried.remove(at);
        return true;
    }
    false
}

/// The child-id notification: a child closing with −1 sets the form's
/// result to −3.
pub fn child_result(id: i32) -> Option<i32> {
    (id == -1).then_some(-3)
}

/// What the doll's return-to callback does when it has no callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnTo {
    /// There is no give-back callback: close the window.
    Close,
    /// Run the installed give-back callback.
    Callback,
}

pub fn return_to(has_callback: bool) -> ReturnTo {
    if has_callback {
        ReturnTo::Callback
    } else {
        ReturnTo::Close
    }
}

/// Building the doll's model: detach the old model, let the scene settle
/// 14 ms, generate the new model and attach it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildDoll {
    pub sleep_ms: u32,
    pub detach_old: bool,
    pub generate: bool,
    pub attach: bool,
}

pub fn build_doll(has_old_model: bool) -> BuildDoll {
    BuildDoll {
        sleep_ms: 14,
        detach_old: has_old_model,
        generate: true,
        attach: true,
    }
}

/// Whether an item dragged off the doll comes back to the pack: on any
/// slot but [`KEEP_SLOT`].
pub fn drop_back(slot: i32) -> bool {
    slot != KEEP_SLOT
}

/// The window id the game is told about when an inventory closes.
pub fn closed_window_id(record_id: u32) -> u32 {
    record_id | INVENTORY_WINDOW
}

/// Whether a mouse event closes the window: a right-button release.
pub fn closes_on(mask: u16) -> bool {
    mask & RIGHT_RELEASE == RIGHT_RELEASE
}

/// The window position the original starts with (a sentinel that means
/// "never placed").
pub const DEFAULT_POS: i32 = -987_654_321;
/// The window's flags.
pub const WINDOW_FLAGS: u32 = 0x3000;
/// The doll's dock style and flag words.
pub const DOLL_DOCK: i32 = 2;
pub const DOLL_EXTRA_FLAGS: u32 = 0x80;
/// The pack panel's dock style, flags and capacity flag.
pub const PACK_DOCK: i32 = 2;
pub const PACK_FLAGS: u32 = 0x80;
pub const PACK_EXTRA_FLAGS: u32 = 0x10_0010;
/// The doll's model key, its light and the pack's row capacity.
pub const PACK_MAX_ROWS: i32 = 50;

/// Where the doll and the pack sit in a window, in client units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowLayout {
    pub doll: [i32; 4],
    pub pack: [i32; 4],
    /// The tab word: the first set's 0x60 or the second's 0x60 at +0x154.
    pub first_tab: i32,
    pub second_tab: i32,
}

/// The doll fills the width inside the border and the height above the
/// 260 units the pack and tabs take; the pack sits at the bottom.
pub fn window_layout(layout: Layout, second_set: bool) -> WindowLayout {
    let g = geometry(layout);
    let (first_tab, second) = if second_set { (0, 0x60) } else { (0x60, 0) };
    WindowLayout {
        doll: [0, 0, g.doll.0, g.doll.1],
        pack: [0, g.height.wrapping_sub(BELOW_DOLL), g.pack.0, g.pack.1],
        first_tab,
        second_tab: second,
    }
}

/// What a close remembers and tells the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosePlan {
    /// The window id, when the game is told.
    pub notify_id: Option<u32>,
    /// The pack was saved (the doll was live).
    pub save_pack: bool,
    /// The position to remember, for the player's own window.
    pub remembered: Option<(i32, i32)>,
    pub base: bool,
}

/// The close plan: a live doll tells the game (id | 0x2000_0000) and saves
/// its pack; the player's window keeps its place for the session.
pub fn close_plan(
    closing: bool,
    doll_live: bool,
    record_id: u32,
    is_player: bool,
    position: (i32, i32),
) -> ClosePlan {
    if closing {
        return ClosePlan {
            notify_id: None,
            save_pack: false,
            remembered: None,
            base: false,
        };
    }
    ClosePlan {
        notify_id: doll_live.then_some(record_id | INVENTORY_WINDOW),
        save_pack: doll_live,
        remembered: is_player.then_some(position),
        base: true,
    }
}

/// The remembered window position: the sentinel while it was never saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedPosition {
    pub x: i32,
    pub y: i32,
}

impl Default for SavedPosition {
    fn default() -> Self {
        Self {
            x: DEFAULT_POS,
            y: DEFAULT_POS,
        }
    }
}

impl SavedPosition {
    /// Saves the player's window place.
    pub fn remember(&mut self, position: (i32, i32)) {
        self.x = position.0;
        self.y = position.1;
    }

    /// Whether a place has been saved.
    pub fn placed(&self) -> bool {
        self.x != DEFAULT_POS || self.y != DEFAULT_POS
    }
}

/// A drag out of the pack, with the mind-control close: the window closes
/// and the drag is refused. Everything else follows [`drag_out`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPermission {
    Allowed,
    Refused,
    /// Close, return false.
    Close,
}

/// The permission check the doll and the pack install: a mind-controlled
/// character shown in the doll closes the window when the drag starts in
/// the pack and the player's control word carries `0x10000`.
pub fn permission(
    doll_pending: bool,
    from_pack: bool,
    source_is_pack: bool,
    controlled: bool,
    player_state: u32,
) -> DragPermission {
    if doll_pending && source_is_pack && player_state & 0x10000 != 0 && controlled {
        DragPermission::Close
    } else if player_state == SWAP_STATE {
        DragPermission::Refused
    } else {
        let _ = from_pack;
        DragPermission::Allowed
    }
}

/// What the give-back list path adds or removes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GiveBack {
    /// The list is the pane's: the item is looked up and its last copy
    /// removed.
    RemovedFromPane(usize),
    /// No pane: the record is appended to the list.
    Appended,
}

/// The give-back entry (the doll's drop-back callback). With no panel,
/// the record `{item, flags, 40, 4}` is appended; with one, the item is
/// found and the panel's last entry removed when it holds it.
pub fn give_back(
    run: &mut dyn FnMut(u32) -> Option<usize>,
    id: u32,
    pane_len: usize,
    has_pane: bool,
) -> GiveBack {
    if !has_pane {
        return GiveBack::Appended;
    }
    if run(id).is_some() && pane_len > 0 {
        GiveBack::RemovedFromPane(pane_len - 1)
    } else {
        GiveBack::Appended
    }
}

/// What dropping onto the doll does (the long form: the slot, the drag's
/// source and whether the view is visible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DollDrop {
    /// Remove the last held entry of this id.
    pub strip_held: bool,
    /// Save the pack first (the doll's view is visible).
    pub save_pack: bool,
    /// Tell the character the item's count changed.
    pub notify_count: bool,
}

/// The doll's drop rule: a real slot that did not come from the pack and
/// whose cursor is over neither the doll nor the pack.
pub fn doll_drop(
    slot: i32,
    from_pack: bool,
    over_window: bool,
    view_visible: bool,
    panel_is_source: bool,
) -> DollDrop {
    if slot == i32::MIN || from_pack || over_window {
        return DollDrop {
            strip_held: false,
            save_pack: false,
            notify_count: false,
        };
    }
    DollDrop {
        strip_held: !panel_is_source,
        save_pack: view_visible,
        notify_count: true,
    }
}

/// The bind plan: shows the model, fills the pack, adds the worn bulk and
/// picks the tab by the character's set flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindPlan {
    pub create_model: bool,
    pub fill_pack: bool,
    pub add_worn_bulk: bool,
    pub second_set: bool,
    pub camera: bool,
}

pub fn bind_plan(record_present: bool, second_set: bool) -> BindPlan {
    BindPlan {
        create_model: record_present,
        fill_pack: record_present,
        add_worn_bulk: record_present,
        second_set,
        camera: record_present,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::possession::Carried;

    fn item(id: u32, flags: u32) -> Carried {
        Carried {
            id,
            flags,
            x: 0,
            y: 0,
        }
    }

    #[test]
    fn the_window_and_its_parts() {
        let g = geometry(Layout::Normal);
        assert_eq!((g.width, g.height), (244, 644));
        assert_eq!(g.doll, (228, 384));
        assert_eq!(g.pack, (228, 180));
        assert_eq!(geometry(Layout::Compact).doll.1, 368);
        assert_eq!(geometry(Layout::Embedded).height, 590);
        assert_eq!(camera_distance(1.8), 18.0);
    }

    #[test]
    fn dragging_out_of_the_pack() {
        assert_eq!(drag_out(true, true, true, 0), DragOut::Close);
        assert_eq!(drag_out(true, false, true, SWAP_STATE), DragOut::Refused);
        assert_eq!(drag_out(false, true, true, 3), DragOut::Allowed);
    }

    #[test]
    fn the_compass_and_the_capacity() {
        let p = Possessions {
            carried: vec![item(115, 0), item(5, HELD), item(6, 0)],
            worn: vec![7],
            ..Default::default()
        };
        assert!(carries_compass(&p, |id| (id == 115).then_some([4, 11])));
        assert!(!carries_compass(&p, |id| (id == 115).then_some([4, 1])));
        // The held sword still counts, and so does the worn item.
        assert_eq!(used_capacity(&p, |id| id), 115 + 5 + 6 + 7);
        assert!(!second_tab(&p));
    }

    #[test]
    fn dropping_on_the_doll_and_back() {
        let mut p = Possessions {
            carried: vec![item(5, HELD), item(5, HELD), item(5, 0)],
            ..Default::default()
        };
        assert!(!drop_on_doll(&mut p, 5, true, false, false));
        assert!(!drop_on_doll(&mut p, 5, false, true, false));
        assert!(drop_on_doll(&mut p, 5, false, false, false));
        assert_eq!(p.carried, vec![item(5, HELD), item(5, 0)]);
        assert!(drop_back(3));
        assert!(!drop_back(KEEP_SLOT));
        assert_eq!(closed_window_id(0x41), 0x2000_0041);
        assert!(closes_on(0x2002));
        assert!(!closes_on(0x2000));
    }

    #[test]
    fn the_window_layout_and_tabs() {
        let l = window_layout(Layout::Normal, false);
        assert_eq!(l.doll, [0, 0, 228, 384]);
        assert_eq!(l.pack, [0, 644 - 260, 228, 180]);
        assert_eq!((l.first_tab, l.second_tab), (0x60, 0));
        let l = window_layout(Layout::Embedded, true);
        assert_eq!(l.pack[1], 590 - 260);
        assert_eq!((l.first_tab, l.second_tab), (0, 0x60));
        assert_eq!(DEFAULT_POS, -987_654_321);
        assert!(!SavedPosition::default().placed());
        let mut p = SavedPosition::default();
        p.remember((12, 34));
        assert_eq!((p.x, p.y, p.placed()), (12, 34, true));
    }

    #[test]
    fn closing_and_permissions() {
        let c = close_plan(false, true, 7, true, (10, 20));
        assert_eq!(c.notify_id, Some(0x2000_0007));
        assert!(c.save_pack);
        assert_eq!(c.remembered, Some((10, 20)));
        assert!(c.base);
        let c = close_plan(false, false, 7, false, (1, 2));
        assert_eq!(c.notify_id, None);
        assert_eq!(c.remembered, None);
        assert!(!close_plan(true, true, 7, true, (0, 0)).base);
        assert_eq!(
            permission(true, true, true, true, 0x10000),
            DragPermission::Close
        );
        assert_eq!(
            permission(false, false, true, false, SWAP_STATE),
            DragPermission::Refused
        );
        assert_eq!(
            permission(true, false, false, true, 0),
            DragPermission::Allowed
        );
    }

    #[test]
    fn the_doll_drop_and_give_back() {
        let d = doll_drop(5, false, false, true, false);
        assert!(d.strip_held && d.save_pack && d.notify_count);
        assert!(!doll_drop(5, true, false, true, false).strip_held);
        assert!(!doll_drop(i32::MIN, false, false, true, false).notify_count);
        assert_eq!(
            give_back(&mut |_| Some(0), 5, 3, true),
            GiveBack::RemovedFromPane(2)
        );
        assert_eq!(give_back(&mut |_| None, 5, 3, true), GiveBack::Appended);
        assert_eq!(give_back(&mut |_| Some(0), 5, 0, false), GiveBack::Appended);
        let b = bind_plan(true, true);
        assert!(b.create_model && b.fill_pack && b.add_worn_bulk && b.camera);
        assert!(b.second_set);
        assert_eq!(child_result(-1), Some(-3));
        assert_eq!(child_result(3), None);
        assert_eq!(return_to(false), ReturnTo::Close);
        assert_eq!(return_to(true), ReturnTo::Callback);
        let d = build_doll(true);
        assert_eq!(d.sleep_ms, 14);
        assert!(d.detach_old && d.generate && d.attach);
        assert_eq!(
            pack_changed(true, false),
            PackChanged {
                save_pack: true,
                compass: false
            }
        );
        assert!(pack_changed(false, true).compass);
    }
}
