//! The "Body" loot window: another character's pack above what it wears
//! and holds.
//!
//! The window is 244 wide and 440 tall at x 400, with two 228-wide
//! panels: the pack 180 tall at the bottom of the client area, and the
//! body's equipment above it, ending four units before the pack starts.
//! A drag off the equipment panel drops the held weapon when that is what
//! was taken. Otherwise it takes the item off the character's record; when
//! the body then takes the item as its core (it was the golem's crystal) the
//! character is marked, a sound plays and the character's balance may be
//! reworked. An apparel item then has its textures and mesh rebuilt. The
//! equipment panel accepts only a core crystal the body takes
//! (`possession::takes_core`). Dropping Power Crystal AL (500) onto the body
//! clears the mark, runs the body's
//! "Activate" dialogue and sets a flag byte on the character's record
//! before the item is added.
//!
//! Evidence: the Body window's constructor, accept filter, drop, drag-off,
//! bind, close and right-click routines (`context/re/item-gui-lane.md`).

use crate::possession::takes_core;

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
/// The mark on the character's state that the crystal clears and a
/// removal sets.
pub const MARK_FLAG: u32 = 0x100;
/// The sound a removal plays (its record id).
pub const UNEQUIP_SOUND: u32 = 0x6f67;
/// The item classification that means apparel.
pub const APPAREL_KIND: u32 = 0x1000;
/// The dialogue the crystal runs on the body.
pub const ACTIVATE_DIALOGUE: &str = "Activate";

/// Where the two panels sit in a window whose client area is
/// `client_height` units tall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub pack: [i32; 4],
    pub equipment: [i32; 4],
}

/// The pack fills the bottom of the client area; the equipment panel above
/// it ends four units before the pack starts.
pub fn layout(client_height: i32) -> Layout {
    let pack_y = client_height.wrapping_sub(PACK_HEIGHT);
    Layout {
        pack: [0, pack_y, PACK_WIDTH, PACK_HEIGHT],
        equipment: [0, 0, PACK_WIDTH, pack_y.wrapping_sub(4)],
    }
}

/// What dragging an item off the equipment panel does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragOff {
    /// A cancelled drag (the original's `i32::MIN` slot).
    Nothing,
    /// The dropped item was the held weapon: drop it, clear a field of the
    /// character's record and tell the scene.
    HeldWeapon,
    /// Take it off the record and ask the view to remove it.
    Removed {
        /// The body takes the item as its core once it is off (it was the
        /// body's core crystal): mark the character, play the sound.
        marked: bool,
        /// Removed, and the record's state byte is clear: the character's
        /// balance is reworked.
        rebalance: bool,
        /// The item is apparel: rebuild its textures and mesh, whether or
        /// not it was the core.
        rebuild: bool,
    },
}

/// The drag-off decision. `held_weapon` is the item id of the held weapon
/// record (None when nothing is held); `removed` is the core test
/// ([`takes_core`]) run after the item left the record, true when the item
/// was the body's core crystal; `state_byte` the record's byte the
/// rebalance tests.
pub fn drag_off(
    slot: i32,
    id: u32,
    held_weapon: Option<u32>,
    removed: bool,
    state_byte: bool,
    apparel: bool,
) -> DragOff {
    if slot == i32::MIN {
        return DragOff::Nothing;
    }
    if held_weapon == Some(id) {
        return DragOff::HeldWeapon;
    }
    DragOff::Removed {
        marked: removed,
        rebalance: removed && !state_byte,
        rebuild: apparel,
    }
}

/// What dropping an item onto the equipment panel does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipDrop {
    /// Clear the character's mark ([`MARK_FLAG`]) and the field word after
    /// the mark.
    pub clear_mark: bool,
    pub clear_field: bool,
    /// Run this dialogue on the body (the actor is a game global, not
    /// traced).
    pub dialogue: Option<&'static str>,
    /// Set the record's activation byte.
    pub activated: bool,
    /// The item is added to the character's record either way.
    pub add: bool,
}

/// The drop for an item id: Power Crystal AL clears the mark and the field,
/// runs the "Activate" dialogue, sets the activation byte and then adds the
/// item; any other item is only added.
pub fn equip_drop(id: u32) -> EquipDrop {
    if id == POWER_CRYSTAL_AL {
        EquipDrop {
            clear_mark: true,
            clear_field: true,
            dialogue: Some(ACTIVATE_DIALOGUE),
            activated: true,
            add: true,
        }
    } else {
        EquipDrop {
            clear_mark: false,
            clear_field: false,
            dialogue: None,
            activated: false,
            add: true,
        }
    }
}

/// The equipment panel's accept filter: the body takes the dragged item as
/// its core crystal ([`takes_core`]: a body whose race flags name the AL or
/// A crystal, wearing none of the five crystals). Nothing else is dropped
/// onto a body's equipment.
pub fn accepts(race_flags: u32, id: u32, worn: &[u32]) -> bool {
    takes_core(race_flags, id, worn)
}

/// The close: the game is told the record's id with the loot-window bit
/// and the pack saves its contents when it was changed; then the base
/// close runs. A window already closing does none of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Close {
    pub notify_id: Option<u32>,
    pub save_pack: bool,
    pub base: bool,
}

pub fn close(closing: bool, record_id: u32, pack_dirty: bool) -> Close {
    if closing {
        Close {
            notify_id: None,
            save_pack: false,
            base: false,
        }
    } else {
        Close {
            notify_id: Some(record_id | LOOT_WINDOW),
            save_pack: pack_dirty,
            base: true,
        }
    }
}

/// A right release closes the window (answered handled); anything else
/// goes to the base handler.
pub fn closes_on(mask: u16) -> bool {
    mask & RIGHT_RELEASE == RIGHT_RELEASE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::possession::TAKES_CORE_AL;

    #[test]
    fn the_panels_stack_in_the_client_area() {
        // A 380-high client area: the pack is its last 180 units and the
        // equipment ends four units above it.
        let l = layout(380);
        assert_eq!(l.pack, [0, 200, 228, 180]);
        assert_eq!(l.equipment, [0, 0, 228, 196]);
        assert_eq!(TITLE, "Body");
        assert_eq!((WIDTH, HEIGHT, FORM_X), (244, 440, 400));
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
            DragOff::Removed {
                marked: true,
                rebalance: true,
                rebuild: true
            }
        );
        // The state byte set: no rebalance. Not removed: no mark, no
        // rebalance, but apparel is still rebuilt.
        assert_eq!(
            drag_off(1, 5, None, true, true, false),
            DragOff::Removed {
                marked: true,
                rebalance: false,
                rebuild: false
            }
        );
        assert_eq!(
            drag_off(1, 5, None, false, false, true),
            DragOff::Removed {
                marked: false,
                rebalance: false,
                rebuild: true
            }
        );
    }

    #[test]
    fn the_crystal_activates_the_body() {
        assert_eq!(
            equip_drop(POWER_CRYSTAL_AL),
            EquipDrop {
                clear_mark: true,
                clear_field: true,
                dialogue: Some(ACTIVATE_DIALOGUE),
                activated: true,
                add: true
            }
        );
        assert_eq!(
            equip_drop(499),
            EquipDrop {
                clear_mark: false,
                clear_field: false,
                dialogue: None,
                activated: false,
                add: true
            }
        );
        // Only a golem body without a crystal takes the AL crystal.
        assert!(accepts(TAKES_CORE_AL, POWER_CRYSTAL_AL, &[7]));
        assert!(!accepts(
            TAKES_CORE_AL,
            POWER_CRYSTAL_AL,
            &[POWER_CRYSTAL_AL]
        ));
        assert!(!accepts(TAKES_CORE_AL, 7, &[]));
        assert!(!accepts(0, POWER_CRYSTAL_AL, &[]));
    }

    #[test]
    fn closing_tells_the_game_and_saves_a_changed_pack() {
        assert_eq!(close(false, 0x41, true).notify_id, Some(0x2000_0041));
        assert!(close(false, 1, true).save_pack);
        assert!(!close(false, 1, false).save_pack);
        assert!(close(false, 1, false).base);
        assert_eq!(close(true, 1, true).notify_id, None);
        assert!(!close(true, 1, true).base);
        assert!(closes_on(0x2002) && !closes_on(0x2000));
    }
}
