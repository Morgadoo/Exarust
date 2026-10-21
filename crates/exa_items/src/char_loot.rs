//! The "Body" loot window: another character's pack above what it wears
//! and holds.
//!
//! The window is 244 wide and 440 tall, with two 228-wide panels: the pack
//! 180 tall at the bottom, and the body's equipment above it, four units
//! shorter than the pack's top. A drag off the equipment panel drops the
//! held weapon when that is what was taken, otherwise unequips the item,
//! marks the character and rebuilds its apparel textures and mesh when the
//! item is apparel. Power Crystal AL (500) dropped onto the body clears a
//! flag and runs the body's "Activate" dialogue with the player as actor
//! before equipping.
//!
//! Evidence: the Body window's constructor, accept filter, drop, drag-off,
//! bind, close and right-click routines (`context/re/item-gui-lane.md`).

/// The window's title and size, and the panel sizes.
pub const TITLE: &str = "Body";
pub const WIDTH: i32 = 244;
pub const HEIGHT: i32 = 440;
pub const BORDER: i32 = 2;
pub const PACK_WIDTH: i32 = 228;
pub const PACK_HEIGHT: i32 = 180;
/// The window's default x position.
pub const FORM_X: i32 = 400;
/// The screen tag.
pub const TAG: u32 = 0xce1c_2040;
/// The window id bit the game is told about on close.
pub const LOOT_WINDOW: u32 = 0x2000_0000;
/// A right-button release.
pub const RIGHT_RELEASE: u16 = 0x2002;
/// Power Crystal AL's item id.
pub const POWER_CRYSTAL_AL: u32 = 500;
/// The body flag the crystal clears.
pub const ACTIVATED_FLAG: u32 = 0x100;
/// The sound the unequip plays (its record id).
pub const UNEQUIP_SOUND: u32 = 0x6f67;
/// The item classification that means apparel.
pub const APPAREL_KIND: u32 = 0x1000;

/// Where the two panels sit in a window of `height` units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub pack: [i32; 4],
    pub equipment: [i32; 4],
}

/// The pack fills the bottom of the window; the equipment panel above it
/// ends four units before the pack starts.
pub fn layout(height: i32) -> Layout {
    let pack_y = height.wrapping_sub(PACK_HEIGHT);
    Layout {
        pack: [0, pack_y, PACK_WIDTH, PACK_HEIGHT],
        equipment: [0, 0, PACK_WIDTH, pack_y.wrapping_sub(4)],
    }
}

/// What dragging an item off the equipment panel does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragOff {
    /// A cancelled drag (the original's `-0x80000000` slot).
    Nothing,
    /// The dropped item was the held weapon: drop it and clear the AI's
    /// activity.
    HeldWeapon,
    /// Unequip it, mark the character, play the sound and maybe rebalance.
    Unequipped {
        /// The view callback removed the item.
        changed: bool,
        /// The character was not feral, so its balance runs.
        rebalance: bool,
        /// The item is apparel: rebuild its textures and mesh.
        rebuild: bool,
    },
}

/// The drag-off decision. `held_weapon` is the item id the body's held
/// weapon record carries (None when nothing is held); `removed` is the
/// view callback's answer; `feral` the AI's byte.
pub fn drag_off(
    slot: i32,
    id: u32,
    held_weapon: Option<u32>,
    removed: bool,
    feral: bool,
    apparel: bool,
) -> DragOff {
    if slot == i32::MIN {
        return DragOff::Nothing;
    }
    if held_weapon == Some(id) {
        return DragOff::HeldWeapon;
    }
    DragOff::Unequipped {
        changed: removed,
        rebalance: !feral,
        rebuild: apparel && removed,
    }
}

/// What dropping an item onto the equipment panel does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipDrop {
    /// The Power Crystal AL cleared the flag and runs "Activate".
    pub activate: bool,
    pub equip: bool,
}

/// Whether the crystal's "Activate" dialogue runs.
pub fn equip_drop(id: u32) -> EquipDrop {
    EquipDrop {
        activate: id == POWER_CRYSTAL_AL,
        equip: true,
    }
}

/// The accept filter: whether the body can wear the item.
pub fn accepts(can_wear: bool) -> bool {
    can_wear
}

/// The window's bind: fills the pack from the character's carried list
/// and the equipment panel from its worn and held items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bind {
    pub pack_filled: bool,
    pub equipment_filled: bool,
    pub drag_callback: bool,
}

pub fn bind(record_present: bool) -> Bind {
    Bind {
        pack_filled: record_present,
        equipment_filled: record_present,
        drag_callback: record_present,
    }
}

/// The close: the game is told the window id, the panel saves its
/// contents, and the base close runs. A window already closing does none
/// of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Close {
    pub notify_id: Option<u32>,
    pub save_panel: bool,
    pub base: bool,
}

pub fn close(closing: bool, record_id: u32) -> Close {
    if closing {
        Close {
            notify_id: None,
            save_panel: false,
            base: false,
        }
    } else {
        Close {
            notify_id: Some(record_id | LOOT_WINDOW),
            save_panel: true,
            base: true,
        }
    }
}

/// A right release closes the window.
pub fn closes_on(mask: u16) -> bool {
    mask & RIGHT_RELEASE == RIGHT_RELEASE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panels_stack() {
        let l = layout(HEIGHT);
        assert_eq!(l.pack, [0, 260, 228, 180]);
        assert_eq!(l.equipment, [0, 0, 228, 256]);
        assert_eq!(TITLE, "Body");
        assert_eq!((WIDTH, FORM_X), (244, 400));
    }

    #[test]
    fn the_drag_off_rules() {
        assert_eq!(
            drag_off(i32::MIN, 5, None, false, false, false),
            DragOff::Nothing
        );
        assert_eq!(
            drag_off(1, 5, Some(5), true, false, true),
            DragOff::HeldWeapon
        );
        assert_eq!(
            drag_off(1, 5, Some(9), true, false, true),
            DragOff::Unequipped {
                changed: true,
                rebalance: true,
                rebuild: true
            }
        );
        assert_eq!(
            drag_off(1, 5, None, false, true, true),
            DragOff::Unequipped {
                changed: false,
                rebalance: false,
                rebuild: false
            }
        );
    }

    #[test]
    fn the_crystal_activates_the_body() {
        assert_eq!(
            equip_drop(POWER_CRYSTAL_AL),
            EquipDrop {
                activate: true,
                equip: true
            }
        );
        assert!(!equip_drop(499).activate);
        assert!(accepts(true));
        assert!(!accepts(false));
    }

    #[test]
    fn closing_tells_the_game_and_saves() {
        assert_eq!(close(false, 0x41).notify_id, Some(0x2000_0041));
        assert!(close(false, 1).save_panel);
        assert_eq!(close(true, 1).notify_id, None);
        assert!(!close(true, 1).base);
        assert!(closes_on(0x2002) && !closes_on(0x2000));
        let b = bind(true);
        assert!(b.pack_filled && b.equipment_filled && b.drag_callback);
    }
}
