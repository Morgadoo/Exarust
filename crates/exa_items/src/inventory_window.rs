//! The inventory window: a character's doll beside its pack, the two
//! linked so that a move between them is not counted against the pack.
//!
//! The window is 244 units wide and 644 tall (628 in the compact layout,
//! 590 inside the arena's arsenal), with an 8-unit border, 8 units from the
//! top of its parent. The doll fills the width inside the border and all
//! but 260 units of the height; the pack is 228 by 180 at the bottom of the
//! client area, with a capacity of 400 and items up to 50. What the
//! character wears counts against the pack's capacity. A drag out of the
//! pack is refused while the player's body swaps its weapon sets, and
//! closes the window while a mind-controlled character is shown. Changing
//! the player's pack sets the compass flag when it holds a usable item of
//! kind 4, sort 11.
//!
//! Evidence: the inventory window's constructor, bind, close and callbacks
//! (`context/re/item-gui-lane.md`). The window is not wired into the
//! running game; its item pane is `item_gui`'s.

use crate::possession::{HELD, Possessions, SECOND_SET};

/// The window's width, border and title.
pub const WIDTH: i32 = 244;
pub const BORDER: i32 = 8;
pub const TITLE: &str = "Inventory";
/// The window's distance from the top of its parent.
pub const TOP: i32 = 8;
/// The screen tag.
pub const TAG: u32 = 0xce1c_2010;
/// The window's height in the normal layout, the compact one, and inside
/// the arsenal.
pub const HEIGHT: i32 = 644;
pub const HEIGHT_COMPACT: i32 = 628;
pub const HEIGHT_EMBEDDED: i32 = 590;
/// The height the doll leaves to the pack and the tabs.
pub const BELOW_DOLL: i32 = 260;
/// The pack's size, capacity and largest item (the last is a game global
/// that starts at 50).
pub const PACK_WIDTH: i32 = 228;
pub const PACK_HEIGHT: i32 = 180;
pub const PACK_CAPACITY: u32 = 400;
pub const PACK_MAX_ITEM: u32 = 50;
/// The player's body state while it swaps weapon sets, during which
/// nothing is dragged out of a pack.
pub const SWAP_STATE: u32 = 0x1F;
/// The character view's word bit that, with the pack as the drag's source,
/// makes a mind-controlled doll close the window.
pub const MIND_CONTROL_BIT: u32 = 0x10000;
/// The usable section's kind and sort of the compass.
pub const COMPASS: [u8; 2] = [4, 11];
/// The mouse mask of a right-button release, which closes the window.
pub const RIGHT_RELEASE: u16 = 0x2002;
/// Window ids registered for an inventory carry this bit.
pub const INVENTORY_WINDOW: u32 = 0x2000_0000;
/// The slot a drag off the doll does not answer for.
pub const KEEP_SLOT: i32 = 8;
/// The value a bound doll model is given (its role is not traced), and the
/// word set beside it.
pub const MODEL_VALUE: f32 = 0.05;
pub const MODEL_WORD: u32 = 0x1000;
/// The encumbrance dots show the shown body's encumbrance (0 to 1) times
/// this, one dot a tenth (a single-precision product).
pub const DOTS_FACTOR: f32 = 10.0;

/// Which layout the window is built in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Normal,
    /// The global compact flag (what sets it is not traced).
    Compact,
    /// Inside the arena's arsenal.
    Embedded,
}

/// Where the window's parts sit, in GUI units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub width: i32,
    pub height: i32,
    /// The doll's size; it sits at the client origin.
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

/// The pack's y: it fills the bottom of a client area `client_height`
/// tall.
pub fn pack_y(client_height: i32) -> i32 {
    client_height.wrapping_sub(PACK_HEIGHT)
}

/// A word the form is given in a control table with its tabs (the tab
/// strip's, by the layout; its role is not traced): 98, 94 or 84.
pub fn tab_word(layout: Layout) -> u16 {
    match layout {
        Layout::Normal => 0x62,
        Layout::Compact => 0x5e,
        Layout::Embedded => 0x54,
    }
}

/// The doll's attribute word: the old word with its two top flag bits
/// (`0xc000`) cleared and `0x90` set.
pub fn doll_attributes(word: u32) -> u32 {
    (word & 0xffff_3fff) | 0x10 | 0x80
}

/// The two tab words, by the record's weapon-set flag: the first set is
/// out (`0x60`, 0) or the second (0, `0x60`).
pub fn tabs(second_set: bool) -> (i32, i32) {
    if second_set { (0, 0x60) } else { (0x60, 0) }
}

/// Whether the record has its second weapon set out.
pub fn second_tab(p: &Possessions) -> bool {
    p.flags & SECOND_SET != 0
}

/// Whether a drag may leave the pack, and whether the window closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPermission {
    Allowed,
    Refused,
    /// Close the window and refuse.
    Close,
}

/// The permission check the doll and the pack install. `doll_flag` is a
/// byte of the doll's record (its role is not traced); `source_is_pack`
/// that the drag starts in the pack; `view_word` the character view's flag
/// word; `controlled` the mind-control test of the shown character against
/// the player; `player_state` the player's body state.
pub fn permission(
    doll_flag: bool,
    source_is_pack: bool,
    view_word: u32,
    controlled: bool,
    player_state: u32,
) -> DragPermission {
    if doll_flag && source_is_pack && view_word & MIND_CONTROL_BIT != 0 && controlled {
        DragPermission::Close
    } else if player_state == SWAP_STATE {
        DragPermission::Refused
    } else {
        DragPermission::Allowed
    }
}

/// Whether a carried list holds a compass: any item whose usable section
/// starts with [`COMPASS`]. The worn list is not looked at.
pub fn carries_compass<F>(p: &Possessions, usable: F) -> bool
where
    F: Fn(u32) -> Option<[u8; 2]>,
{
    p.carried.iter().any(|c| usable(c.id) == Some(COMPASS))
}

/// The pack-change callback's two duties: save the pack into the
/// character's list when the pack's changed byte is set, and, for the
/// player's own pack, scan for the compass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackChanged {
    pub save_pack: bool,
    pub compass: bool,
}

pub fn pack_changed(pack_dirty: bool, is_player: bool) -> PackChanged {
    PackChanged {
        save_pack: pack_dirty,
        compass: is_player,
    }
}

/// What binding to a character adds to the pack's used capacity beyond
/// the fill: the bulk of every worn item.
pub fn worn_bulk<F>(p: &Possessions, bulk: F) -> u32
where
    F: Fn(u32) -> u32,
{
    p.worn.iter().map(|worn| bulk(worn.id)).sum()
}

/// The value the inventory's rating-dots record (class 0xac01, beside the
/// Encumbrance label) shows for a body's encumbrance: ten dots at 1.
pub fn encumbrance_dots(encumbrance: f32) -> f32 {
    encumbrance * DOTS_FACTOR
}

/// Removes the last entry of `id` that is marked held, the way a drop onto
/// the doll takes an item that was in a hand. Returns whether one left.
pub fn strip_last_held(p: &mut Possessions, id: u32) -> bool {
    match p
        .carried
        .iter()
        .rposition(|c| c.id == id && c.flags & HELD != 0)
    {
        Some(at) => {
            p.carried.remove(at);
            true
        }
        None => false,
    }
}

/// What a drop onto the doll or the pack does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DollDrop {
    /// Save the pack into the record first (its changed byte is set).
    pub save_pack: bool,
    /// Take the item's last held entry off the record (see
    /// [`strip_last_held`]) and refill the pack from the record.
    pub strip_held: bool,
    /// The record recounts the item (the call that stores the count is not
    /// traced).
    pub notify_count: bool,
}

/// The drop rule: nothing for a cancelled drag (`i32::MIN`) or while the
/// pointer is over the doll or the pack; otherwise the pack is saved when
/// changed, the held entry is taken off unless the drag came from the pack,
/// and the record recounts the item.
pub fn doll_drop(slot: i32, over_window: bool, pack_dirty: bool, from_pack: bool) -> DollDrop {
    if slot == i32::MIN || over_window {
        return DollDrop {
            save_pack: false,
            strip_held: false,
            notify_count: false,
        };
    }
    DollDrop {
        save_pack: pack_dirty,
        strip_held: !from_pack,
        notify_count: true,
    }
}

/// What the doll's drag-off callback does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropBack {
    /// The callback's answer. `None` for [`KEEP_SLOT`]: the original
    /// returns whatever its register held.
    pub reply: Option<bool>,
    /// The pack is asked to act on its last entry (the call is not
    /// traced).
    pub pack_last: bool,
}

/// The drag-off rule: any slot but [`KEEP_SLOT`] answers true and, when the
/// pack accepts the item (`can_add`), has the pack act on its last entry.
pub fn drop_back(slot: i32, can_add: bool) -> DropBack {
    if slot == KEEP_SLOT {
        DropBack {
            reply: None,
            pack_last: false,
        }
    } else {
        DropBack {
            reply: Some(true),
            pack_last: can_add,
        }
    }
}

/// What handing an item back through the shared list object does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GiveBack {
    Nothing,
    /// A pane is open and does not hold the item yet but can take it: it
    /// acts on its last entry (the call is not traced).
    PaneLast,
    /// No pane is open: append the record at the pack's give-back place
    /// ([`Possessions::give_back`]).
    Append,
}

/// The give-back rule. With a pane open, `pane_count` is its count for the
/// item; a positive one does nothing, otherwise it takes the item when
/// `can_add`. With no pane, a list to append to takes the record.
pub fn give_back(has_pane: bool, has_list: bool, pane_count: i32, can_add: bool) -> GiveBack {
    if has_pane {
        if pane_count <= 0 && can_add {
            GiveBack::PaneLast
        } else {
            GiveBack::Nothing
        }
    } else if has_list {
        GiveBack::Append
    } else {
        GiveBack::Nothing
    }
}

/// The child-id notification: a child closing with −1 sets the form's
/// result to −3.
pub fn child_result(id: i32) -> Option<i32> {
    (id == -1).then_some(-3)
}

/// What a right release does: run the installed callback, or close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnTo {
    /// There is no callback: close the window.
    Close,
    /// Run the installed callback.
    Callback,
}

/// The right-release rule of the doll and the window.
pub fn return_to(has_callback: bool) -> ReturnTo {
    if has_callback {
        ReturnTo::Callback
    } else {
        ReturnTo::Close
    }
}

/// Whether a mouse event is a right-button release (the window handles it
/// as [`return_to`] and answers handled; anything else goes to the base).
pub fn closes_on(mask: u16) -> bool {
    mask & RIGHT_RELEASE == RIGHT_RELEASE
}

/// Building the doll's model: with an old model, take it from the scene and
/// let the scene settle 14 ms; then generate the new one and attach it when
/// there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildDoll {
    pub detach_old: bool,
    pub sleep_ms: u32,
}

pub fn build_doll(has_old_model: bool) -> BuildDoll {
    BuildDoll {
        detach_old: has_old_model,
        sleep_ms: if has_old_model { 14 } else { 0 },
    }
}

/// The window id the game is told about when an inventory closes.
pub fn closed_window_id(record_id: u32) -> u32 {
    record_id | INVENTORY_WINDOW
}

/// The window position the original starts with (a sentinel that means
/// "never placed", `-987654321`).
pub const DEFAULT_POS: i32 = -987_654_321;

/// The bind plan for a character view: the model is built, linked to the
/// view and given its values; the pack is filled and the worn bulk added;
/// the tabs follow the weapon set; the encumbrance dots follow the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindPlan {
    pub build_model: bool,
    pub link_model: bool,
    pub fill_pack: bool,
    pub add_worn_bulk: bool,
    pub tabs: (i32, i32),
    pub dots: bool,
}

pub fn bind_plan(has_view: bool, has_model: bool, second_set: bool) -> BindPlan {
    BindPlan {
        build_model: has_view,
        link_model: has_view && has_model,
        fill_pack: has_view,
        add_worn_bulk: has_view,
        tabs: tabs(second_set),
        dots: has_view && has_model,
    }
}

/// What happens to the shared list object on close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListRelease {
    /// There is none.
    None,
    /// Others still hold it: its pane link is cleared and its count drops.
    Cleared,
    /// It was the last holder: the object is freed.
    Freed,
}

impl ListRelease {
    /// The release for a shared object's count: `None` when absent, a
    /// count above one clears it, one or less frees it.
    pub fn for_count(count: Option<i32>) -> Self {
        match count {
            None => ListRelease::None,
            Some(n) if n > 1 => ListRelease::Cleared,
            Some(_) => ListRelease::Freed,
        }
    }
}

/// What a close remembers and tells the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosePlan {
    /// The window id, when the game is told.
    pub notify_id: Option<u32>,
    /// The pack is saved into the record.
    pub save_pack: bool,
    pub list: ListRelease,
    /// The position to remember, for the player's own window.
    pub remembered: Option<(i32, i32)>,
    /// The base close runs and answers; otherwise the answer is true.
    pub base: bool,
}

/// The close plan. A window already closing does nothing and answers true.
/// Otherwise a bound character view tells the game (`record_id |
/// 0x2000_0000`) and saves a changed pack; the shared list object is
/// released by its count (`list_count`, `None` when absent); the player's
/// window keeps its place; then the base close runs.
pub fn close_plan(
    closing: bool,
    has_view: bool,
    pack_dirty: bool,
    record_id: u32,
    list_count: Option<i32>,
    is_player: bool,
    position: (i32, i32),
) -> ClosePlan {
    if closing {
        return ClosePlan {
            notify_id: None,
            save_pack: false,
            list: ListRelease::None,
            remembered: None,
            base: false,
        };
    }
    ClosePlan {
        notify_id: has_view.then(|| closed_window_id(record_id)),
        save_pack: has_view && pack_dirty,
        list: ListRelease::for_count(list_count),
        remembered: is_player.then_some(position),
        base: true,
    }
}

/// The doll's view alpha's factor: the bind sets the doll form's alpha to
/// the character view's scale times ten (the view's own scale field, a
/// tenth of the alpha).
pub const VIEW_FADE_FACTOR: f32 = 10.0;

/// The doll's view fade: the view's scale times [`VIEW_FADE_FACTOR`].
pub fn doll_fade(view_scale: f32) -> f32 {
    view_scale * VIEW_FADE_FACTOR
}

/// What binding a body to the open inventory does: the doll's model is
/// loaded (`model`), the two draw-order words take 0x60 and 0 (the body's
/// flag word `0x1000000` puts them the other way), and the doll's alpha
/// follows the view's scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BindView {
    /// A body was bound and its model is looked up.
    pub model: bool,
    /// The two draw-order words: (front, back).
    pub layers: (i32, i32),
    /// The doll form's alpha.
    pub fade: f32,
}

/// The bind for a body whose flag word has (or lacks) bit 24.
pub fn bind_view(has_body: bool, body_flag_0x1000000: bool, view_scale: f32) -> BindView {
    BindView {
        model: has_body,
        layers: if body_flag_0x1000000 {
            (0, 0x60)
        } else {
            (0x60, 0)
        },
        fade: doll_fade(view_scale),
    }
}

/// What closing the open inventory releases besides the form: the shared
/// list (by count), the pane (by count) and the character view's panel
/// redraw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseParts {
    /// The list's release: a count of 1 frees it, a larger one clears its
    /// entries, the rest decrements it.
    pub list: ListRelease,
    /// The pane's release, the same rule.
    pub pane: ListRelease,
    /// The character view's panel is redrawn from the body's pack.
    pub redraw_panel: bool,
}

/// The parts a close releases, by the list's and pane's counts (`None`
/// when the object is absent) and the view's panel flag.
pub fn close_parts(
    list_count: Option<i32>,
    pane_count: Option<i32>,
    redraw_flag: bool,
) -> CloseParts {
    CloseParts {
        list: ListRelease::for_count(list_count),
        pane: ListRelease::for_count(pane_count),
        redraw_panel: redraw_flag,
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
    fn binding_a_body_sets_the_dolls_layers_and_fade() {
        let bound = bind_view(true, false, 75.0);
        assert!(bound.model);
        assert_eq!(bound.layers, (0x60, 0));
        // The view's field is a tenth of the alpha the doll form gets.
        assert_eq!(bound.fade, 750.0);
        let flipped = bind_view(true, true, 75.0);
        assert_eq!(flipped.layers, (0, 0x60));
        assert_eq!(flipped.fade, 750.0);
        let none = bind_view(false, false, 10.0);
        assert!(!none.model);
        assert_eq!(none.layers, (0x60, 0));
        assert_eq!(none.fade, 100.0);
    }

    #[test]
    fn closing_releases_the_list_and_the_pane_by_their_counts() {
        let parts = close_parts(Some(3), Some(1), true);
        assert_eq!(parts.list, ListRelease::Cleared);
        assert_eq!(parts.pane, ListRelease::Freed);
        assert!(parts.redraw_panel);
        let parts = close_parts(None, Some(2), false);
        assert_eq!(parts.list, ListRelease::None);
        assert_eq!(parts.pane, ListRelease::Cleared);
        assert!(!parts.redraw_panel);
        assert_eq!(ListRelease::for_count(Some(1)), ListRelease::Freed);
        assert_eq!(ListRelease::for_count(None), ListRelease::None);
    }

    #[test]
    fn the_window_and_its_parts() {
        let g = geometry(Layout::Normal);
        assert_eq!((g.width, g.height), (244, 644));
        assert_eq!(g.doll, (228, 384));
        assert_eq!(g.pack, (228, 180));
        assert_eq!(geometry(Layout::Compact).doll.1, 368);
        assert_eq!(geometry(Layout::Embedded).height, 590);
        assert_eq!(geometry(Layout::Embedded).doll.1, 330);
        // The pack sits at the bottom of the client area, not 260 up.
        assert_eq!(pack_y(600), 420);
        assert_eq!(encumbrance_dots(0.25), 2.5);
        assert_eq!(encumbrance_dots(1.0), 10.0);
        assert_eq!(
            (
                tab_word(Layout::Normal),
                tab_word(Layout::Compact),
                tab_word(Layout::Embedded)
            ),
            (98, 94, 84)
        );
        assert_eq!(doll_attributes(0xc000_c000), 0xc000_0090);
    }

    #[test]
    fn the_drag_permission() {
        // A mind-controlled doll, dragging from the pack: close.
        assert_eq!(
            permission(true, true, 0x10000, true, 0),
            DragPermission::Close
        );
        // Without the bit, or the control, or the pack as source: the swap
        // state decides.
        assert_eq!(
            permission(true, true, 0, true, SWAP_STATE),
            DragPermission::Refused
        );
        assert_eq!(
            permission(true, false, 0x10000, true, 3),
            DragPermission::Allowed
        );
        assert_eq!(
            permission(false, true, 0x10000, true, 3),
            DragPermission::Allowed
        );
    }

    #[test]
    fn the_compass_and_the_worn_bulk() {
        let p = Possessions {
            carried: vec![item(115, 0), item(5, HELD), item(6, 0)],
            worn: vec![
                exa_formats::characters::ItemRef::new(7, 0),
                exa_formats::characters::ItemRef::new(8, 0),
            ],
            ..Default::default()
        };
        assert!(carries_compass(&p, |id| (id == 115).then_some([4, 11])));
        assert!(!carries_compass(&p, |id| (id == 115).then_some([4, 1])));
        // Only what is worn is added by the bind; the fill counts the rest.
        assert_eq!(worn_bulk(&p, |id| id * 10), 150);
        assert!(!second_tab(&p));
    }

    #[test]
    fn dropping_on_the_doll() {
        let mut p = Possessions {
            carried: vec![item(5, HELD), item(5, HELD), item(5, 0)],
            ..Default::default()
        };
        assert!(strip_last_held(&mut p, 5));
        assert_eq!(p.carried, vec![item(5, HELD), item(5, 0)]);
        assert!(strip_last_held(&mut p, 5));
        assert!(!strip_last_held(&mut p, 5));
        assert_eq!(p.carried, vec![item(5, 0)]);

        let d = doll_drop(3, false, true, false);
        assert!(d.save_pack && d.strip_held && d.notify_count);
        // A drag from the pack still saves and recounts, but strips nothing.
        let d = doll_drop(3, false, true, true);
        assert!(d.save_pack && !d.strip_held && d.notify_count);
        assert!(!doll_drop(3, false, false, false).save_pack);
        let none = doll_drop(i32::MIN, false, true, false);
        assert!(!none.save_pack && !none.strip_held && !none.notify_count);
        assert!(!doll_drop(3, true, true, false).notify_count);
    }

    #[test]
    fn the_drag_off_and_the_give_back() {
        assert_eq!(
            drop_back(3, true),
            DropBack {
                reply: Some(true),
                pack_last: true
            }
        );
        assert!(!drop_back(3, false).pack_last);
        assert_eq!(drop_back(KEEP_SLOT, true).reply, None);
        assert!(!drop_back(KEEP_SLOT, true).pack_last);

        assert_eq!(give_back(true, false, 0, true), GiveBack::PaneLast);
        assert_eq!(give_back(true, true, 2, true), GiveBack::Nothing);
        assert_eq!(give_back(true, true, 0, false), GiveBack::Nothing);
        assert_eq!(give_back(false, true, 0, false), GiveBack::Append);
        assert_eq!(give_back(false, false, 0, false), GiveBack::Nothing);
    }

    #[test]
    fn the_bind_and_the_tabs() {
        assert_eq!(tabs(false), (0x60, 0));
        assert_eq!(tabs(true), (0, 0x60));
        let b = bind_plan(true, true, true);
        assert!(b.build_model && b.link_model && b.fill_pack && b.add_worn_bulk && b.dots);
        assert_eq!(b.tabs, (0, 0x60));
        let b = bind_plan(true, false, false);
        assert!(b.fill_pack && !b.link_model && !b.dots);
        let b = bind_plan(false, false, false);
        assert!(!b.fill_pack && !b.build_model);
        assert_eq!(build_doll(true).sleep_ms, 14);
        assert!(build_doll(true).detach_old);
        assert_eq!(build_doll(false).sleep_ms, 0);
    }

    #[test]
    fn the_right_release_and_the_child() {
        assert!(closes_on(0x2002));
        assert!(!closes_on(0x2000));
        assert_eq!(return_to(false), ReturnTo::Close);
        assert_eq!(return_to(true), ReturnTo::Callback);
        assert_eq!(child_result(-1), Some(-3));
        assert_eq!(child_result(3), None);
        assert_eq!(closed_window_id(0x41), 0x2000_0041);
    }

    #[test]
    fn closing_and_the_remembered_place() {
        let c = close_plan(false, true, true, 7, Some(1), true, (10, 20));
        assert_eq!(c.notify_id, Some(0x2000_0007));
        assert!(c.save_pack && c.base);
        assert_eq!(c.list, ListRelease::Freed);
        assert_eq!(c.remembered, Some((10, 20)));
        // A pack that did not change is not saved; a shared list stays.
        let c = close_plan(false, true, false, 7, Some(2), false, (1, 2));
        assert!(!c.save_pack);
        assert_eq!(c.list, ListRelease::Cleared);
        assert_eq!(c.remembered, None);
        // No bound view: the game is not told.
        let c = close_plan(false, false, true, 7, None, false, (1, 2));
        assert_eq!(c.notify_id, None);
        assert!(!c.save_pack && c.base);
        assert_eq!(c.list, ListRelease::None);
        assert!(!close_plan(true, true, true, 7, Some(1), true, (0, 0)).base);

        assert_eq!(DEFAULT_POS, -987_654_321);
        assert!(!SavedPosition::default().placed());
        let mut p = SavedPosition::default();
        p.remember((12, 34));
        assert_eq!((p.x, p.y, p.placed()), (12, 34, true));
    }
}
