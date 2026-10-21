//! The company's arsenal: five shelves of five boxes, and which shelf an
//! item belongs on.
//!
//! An apparel item goes on the armour shelf when its layer type is 3
//! through 6, else on the clothing shelf; an item that is not apparel goes
//! on the weapons shelf when it is a weapon, the shields shelf when it is a
//! shield, and nowhere otherwise. Stowing adds an entry with its flags to
//! the shelf's first box. Releasing clears flag bits from the first entry,
//! box by box, that holds the item with any of them set. A new career
//! stows its starting kit with flags 0x600.
//!
//! Evidence: the retail arena's arsenal routines read from `Exanima.exe`
//! (`context/re/arena-match.md`).

/// Shelves in the arsenal, and boxes on a shelf.
pub const SHELVES: usize = 5;
pub const BOXES: usize = 5;
/// The weapons shelf.
pub const WEAPONS: usize = 0;
/// The shields shelf.
pub const SHIELDS: usize = 2;
/// The armour shelf: apparel of layer types 3 through 6.
pub const ARMOUR: usize = 3;
/// The clothing shelf: every other apparel.
pub const CLOTHING: usize = 4;
/// Flags a new career's starting kit is stowed with.
pub const STARTING_KIT: u32 = 0x600;

/// What the object database says about an item, as the arsenal asks it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemKind {
    /// Found in the database at all.
    pub known: bool,
    /// Its apparel record's layer type, when it is apparel.
    pub apparel_layer: Option<u8>,
    /// It has the weapon kind (0x4000).
    pub weapon: bool,
    /// It has the shield kind (0x8000).
    pub shield: bool,
}

/// The shelf an item goes on, if any.
pub fn shelf_of(kind: ItemKind) -> Option<usize> {
    if !kind.known {
        return None;
    }
    if let Some(layer) = kind.apparel_layer {
        return Some(if (3..=6).contains(&layer) {
            ARMOUR
        } else {
            CLOTHING
        });
    }
    if kind.weapon {
        Some(WEAPONS)
    } else if kind.shield {
        Some(SHIELDS)
    } else {
        None
    }
}

/// One entry in a box: the item, its flags, and the two words a new entry
/// gets (40 and 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArsenalEntry {
    pub item: u32,
    pub flags: u32,
    pub first: u16,
    pub second: u16,
}

/// The arsenal's shelves.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arsenal {
    pub shelves: [[Vec<ArsenalEntry>; BOXES]; SHELVES],
}

impl Arsenal {
    /// Stow an item on its shelf's first box. An item with no shelf is not
    /// stowed.
    pub fn stow(&mut self, item: u32, kind: ItemKind, flags: u32) -> bool {
        let Some(shelf) = shelf_of(kind) else {
            return false;
        };
        self.shelves[shelf][0].push(ArsenalEntry {
            item,
            flags,
            first: 0x28,
            second: 4,
        });
        true
    }

    /// Clear `mask` from the first entry, box by box on the item's shelf,
    /// that holds the item with any bit of `mask` set.
    pub fn release(&mut self, item: u32, kind: ItemKind, mask: u32) -> bool {
        let Some(shelf) = shelf_of(kind) else {
            return false;
        };
        for entries in &mut self.shelves[shelf] {
            if let Some(entry) = entries
                .iter_mut()
                .find(|entry| entry.item == item && entry.flags & mask != 0)
            {
                entry.flags &= !mask;
                return true;
            }
        }
        false
    }

    /// Empty every shelf, as a new career does.
    pub fn clear(&mut self) {
        for shelf in &mut self.shelves {
            for entries in shelf {
                entries.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apparel(layer: u8) -> ItemKind {
        ItemKind {
            known: true,
            apparel_layer: Some(layer),
            ..ItemKind::default()
        }
    }

    #[test]
    fn items_go_on_their_shelves() {
        assert_eq!(shelf_of(apparel(2)), Some(CLOTHING));
        assert_eq!(shelf_of(apparel(3)), Some(ARMOUR));
        assert_eq!(shelf_of(apparel(6)), Some(ARMOUR));
        assert_eq!(shelf_of(apparel(7)), Some(CLOTHING));
        let weapon = ItemKind {
            known: true,
            weapon: true,
            shield: true,
            ..ItemKind::default()
        };
        assert_eq!(shelf_of(weapon), Some(WEAPONS));
        let shield = ItemKind {
            weapon: false,
            ..weapon
        };
        assert_eq!(shelf_of(shield), Some(SHIELDS));
        let neither = ItemKind {
            known: true,
            ..ItemKind::default()
        };
        assert_eq!(shelf_of(neither), None);
        let unknown = ItemKind {
            known: false,
            ..weapon
        };
        assert_eq!(shelf_of(unknown), None);
        // Apparel wins over the weapon kind.
        let both = ItemKind {
            weapon: true,
            ..apparel(4)
        };
        assert_eq!(shelf_of(both), Some(ARMOUR));
    }

    #[test]
    fn stowing_and_releasing_flags() {
        let mut arsenal = Arsenal::default();
        let sword = ItemKind {
            known: true,
            weapon: true,
            ..ItemKind::default()
        };
        assert!(arsenal.stow(7, sword, STARTING_KIT));
        assert_eq!(
            arsenal.shelves[WEAPONS][0],
            vec![ArsenalEntry {
                item: 7,
                flags: 0x600,
                first: 40,
                second: 4
            }]
        );
        assert!(arsenal.release(7, sword, 0x200));
        assert_eq!(arsenal.shelves[WEAPONS][0][0].flags, 0x400);
        assert!(!arsenal.release(7, sword, 0x200));
        assert!(arsenal.release(7, sword, 0x600));
        assert_eq!(arsenal.shelves[WEAPONS][0][0].flags, 0);
        // A later box is searched when the first holds no match.
        arsenal.shelves[WEAPONS][3].push(ArsenalEntry {
            item: 7,
            flags: 0x200,
            first: 40,
            second: 4,
        });
        assert!(arsenal.release(7, sword, 0x200));
        assert_eq!(arsenal.shelves[WEAPONS][3][0].flags, 0);
        let rock = ItemKind {
            known: true,
            ..ItemKind::default()
        };
        assert!(!arsenal.stow(9, rock, 0x600));
        arsenal.clear();
        assert!(arsenal.shelves.iter().flatten().all(Vec::is_empty));
    }
}
