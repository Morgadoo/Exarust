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
}
