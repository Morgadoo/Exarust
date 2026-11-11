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
//! An item that leaves the company (sold at the shop or to the merchant)
//! leaves the arsenal and the holders' lists. A unique item (either of the
//! id's top two bits set) goes from every box, from the manager's and every
//! employee's record and their item lists. Any other item loses one copy:
//! scanning the shelf's boxes from the last and each box from its end, the
//! first copy flagged 0x200, else the first copy found. Each holder in turn
//! (the manager, then the staff) may keep as many copies as were left: the
//! copies past that, from the end of his list, close up, the list keeping
//! its length (the original closes up a copy of the list's count, so the
//! last entry stays where it was), and the allowance drops by those removed;
//! a holder left with fewer than two copies has the item taken from each of
//! his loadouts past what he keeps.
//!
//! The arena's shop and the company merchant trade through the arsenal:
//! the trade window's two arena hooks give up each sold item as above and
//! stow each bought item, made the world's own copy, on its shelf with no
//! flags, so a purchase never joins the manager's pack.
//!
//! Evidence: the retail arena's arsenal routines read from `Exanima.exe`
//! (`context/re/arena-match.md`).

use exa_character::record::CharacterData;
use exa_formats::characters::Loadout;

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
    /// Its body's armour type byte (the arena's apparel class), when the
    /// record has a body section and so is apparel.
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

impl ItemKind {
    /// The kind the object database's record gives, as the save's load pass
    /// classifies an item: an item with a body is apparel, its armour type
    /// byte choosing between the armour shelf (types 3 through 6) and the
    /// clothing shelf; an item without a body is a weapon when the record
    /// has a weapon section and a shield when it has a shield section. An
    /// item with neither body, weapon nor shield section is known but goes
    /// on no shelf. The section checks are on the section's presence, not on
    /// whether its contents parse, which is what the original tests.
    pub fn of_item(item: &exa_formats::objdb::Item) -> Self {
        Self {
            known: true,
            apparel_layer: item.has_body.then_some(item.armour_type),
            weapon: item.has_weapon,
            shield: item.has_shield,
        }
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

/// The id bits of a unique item.
pub const UNIQUE: u32 = 0xc000_0000;
/// The flag a copy the arsenal gives up first carries.
pub const GIVEN_FIRST: u32 = 0x200;

/// A holder of company items: the manager or an employee, the item list
/// the arsenal keeps for him (the manager's stash, an employee's kit) and
/// his six loadouts.
pub struct Holder<'a> {
    pub items: &'a mut Vec<u32>,
    pub loadouts: &'a mut [Loadout],
    /// What his record itself carries, holds and wears, when it is kept
    /// apart from the loadouts (a specialist's own items).
    pub own: Option<OwnItems<'a>>,
}

/// The items a holder's record itself carries, holds and wears.
pub struct OwnItems<'a> {
    pub carried: &'a mut Vec<u32>,
    pub hands: &'a mut [u32; 2],
    pub worn: &'a mut Vec<(u32, u32)>,
}

impl Holder<'_> {
    /// Forgets `item` everywhere on the holder's record: what it carries,
    /// holds and wears, and every loadout.
    fn remove_everywhere(&mut self, item: u32) {
        if let Some(own) = &mut self.own {
            own.carried.retain(|held| *held != item);
            for hand in own.hands.iter_mut() {
                if *hand == item {
                    *hand = 0;
                }
            }
            own.worn.retain(|(worn, _)| *worn != item);
        }
        for loadout in self.loadouts.iter_mut() {
            CharacterData::loadout_forget_item(loadout, item);
        }
    }
}

/// Remove every entry of `item` from a list, from its end.
fn drop_all<T>(entries: &mut Vec<T>, matches: impl Fn(&T) -> bool) {
    let mut index = entries.len();
    while index > 0 {
        index -= 1;
        if matches(&entries[index]) {
            entries.remove(index);
        }
    }
}

/// A holder keeps at most `allowance` copies of `item`; the list keeps its
/// length, and the allowance drops by the copies removed.
fn strip_holder(holder: &mut Holder, item: u32, allowance: &mut i32) {
    let items = &mut *holder.items;
    let mut length = items.len();
    let mut left = *allowance;
    let mut seen = 0i32;
    let mut removed = 0i32;
    for index in (0..items.len()).rev() {
        if items[index] != item {
            continue;
        }
        seen += 1;
        left -= 1;
        if left < 0 {
            length -= 1;
            for at in index..length {
                items[at] = items[at + 1];
            }
            removed += 1;
        }
    }
    if seen <= 0 {
        return;
    }
    *allowance -= removed;
    let kept = seen - removed;
    if kept < 2 {
        for loadout in holder.loadouts.iter_mut() {
            CharacterData::loadout_remove_item(loadout, item, kept as usize);
        }
    }
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

    /// Clear `mask` from the first entry of each box on the item's shelf
    /// that holds the item with any bit of `mask` set; whether any did. The
    /// original's walk goes on to the next box after a hit, so every box
    /// gives back one lend.
    pub fn release(&mut self, item: u32, kind: ItemKind, mask: u32) -> bool {
        let Some(shelf) = shelf_of(kind) else {
            return false;
        };
        let mut released = false;
        for entries in &mut self.shelves[shelf] {
            if let Some(entry) = entries
                .iter_mut()
                .find(|entry| entry.item == item && entry.flags & mask != 0)
            {
                entry.flags &= !mask;
                released = true;
            }
        }
        released
    }

    /// An item leaves the company. `holders` are the manager, then the
    /// staff in order. An item that is not unique and has no shelf is left
    /// alone (the original indexes outside the shelves).
    pub fn give_up(&mut self, item: u32, kind: ItemKind, holders: &mut [Holder]) {
        if item & UNIQUE != 0 {
            for shelf in &mut self.shelves {
                for entries in shelf {
                    drop_all(entries, |entry| entry.item == item);
                }
            }
            for holder in holders.iter_mut() {
                holder.remove_everywhere(item);
                drop_all(&mut *holder.items, |held| *held == item);
            }
            return;
        }
        let Some(shelf) = shelf_of(kind) else {
            return;
        };
        let mut copies = 0i32;
        let mut flagged: Option<(usize, usize)> = None;
        let mut plain: Option<(usize, usize)> = None;
        for box_index in (0..BOXES).rev() {
            let entries = &self.shelves[shelf][box_index];
            for index in (0..entries.len()).rev() {
                if entries[index].item != item {
                    continue;
                }
                copies += 1;
                if flagged.is_some() {
                    continue;
                }
                if entries[index].flags & GIVEN_FIRST != 0 {
                    flagged = Some((box_index, index));
                } else if plain.is_none() {
                    plain = Some((box_index, index));
                }
            }
        }
        if let Some((box_index, index)) = flagged.or(plain) {
            self.shelves[shelf][box_index].remove(index);
        }
        let mut allowance = copies - 1;
        for holder in holders.iter_mut() {
            strip_holder(holder, item, &mut allowance);
        }
    }

    /// A confirmed trade at the arena's shop or the company merchant, as
    /// the trade window's two arena hooks run it. The window settles the
    /// sell basket first: each sold item leaves the company
    /// ([`Arsenal::give_up`] over `holders`, the manager then the staff).
    /// Then each bought item, instead of joining the manager's pack, is made
    /// the world's own copy (`promote`) and stowed with no flags on its
    /// shelf's first box. Returns each bought item's id after the copy and
    /// the shelf it went on (none for an item no shelf takes, which the
    /// original also drops).
    pub fn trade(
        &mut self,
        sold: &[u32],
        bought: &[u32],
        kind: impl Fn(u32) -> ItemKind,
        mut promote: impl FnMut(u32) -> u32,
        holders: &mut [Holder],
    ) -> Vec<(u32, Option<usize>)> {
        for &item in sold {
            self.give_up(item, kind(item), holders);
        }
        bought
            .iter()
            .map(|&item| {
                let copy = promote(item);
                let item_kind = kind(copy);
                let shelf = self.stow(copy, item_kind, 0).then(|| shelf_of(item_kind));
                (copy, shelf.flatten())
            })
            .collect()
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

/// The shelf facts of the game's object database, by item id, for the
/// arena's live stows and releases ([`ItemKind::of_item`] for every record;
/// an id the database does not hold goes on no shelf).
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct ShelfKinds(pub std::collections::HashMap<u32, ItemKind>);

impl ShelfKinds {
    /// The kind of `item`: unknown when the database does not hold it.
    pub fn kind(&self, item: u32) -> ItemKind {
        self.0.get(&item).copied().unwrap_or_default()
    }

    /// The kinds of every item record of the game's `objdb.rdb`.
    pub fn of_game(game: &exa_assets::Game) -> Self {
        let items = game
            .0
            .pack(exa_assets::PackKind::Resource)
            .and_then(|pack| pack.get("objdb.rdb"))
            .and_then(|bytes| exa_formats::rdb::Database::parse(bytes).ok())
            .map(|db| exa_formats::objdb::items(&db))
            .unwrap_or_default();
        Self(crate::arena_ids::kinds_from_items(&items))
    }
}

/// Reads the shelf facts once the game is open.
pub fn load_shelf_kinds(
    game: Option<bevy::prelude::Res<exa_assets::Game>>,
    mut kinds: bevy::prelude::ResMut<ShelfKinds>,
    mut done: bevy::prelude::Local<bool>,
) {
    if *done {
        return;
    }
    let Some(game) = game else {
        return;
    };
    *done = true;
    *kinds = ShelfKinds::of_game(&game);
    bevy::log::info!("arsenal: shelf facts for {} items", kinds.0.len());
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
        // A hit does not end the walk: each box gives back its first lend.
        arsenal.shelves[WEAPONS][0][0].flags = 0x200;
        arsenal.shelves[WEAPONS][3][0].flags = 0x200;
        arsenal.shelves[WEAPONS][3].push(ArsenalEntry {
            item: 7,
            flags: 0x200,
            first: 40,
            second: 4,
        });
        assert!(arsenal.release(7, sword, 0x200));
        assert_eq!(arsenal.shelves[WEAPONS][0][0].flags, 0);
        assert_eq!(arsenal.shelves[WEAPONS][3][0].flags, 0);
        assert_eq!(arsenal.shelves[WEAPONS][3][1].flags, 0x200);
        let rock = ItemKind {
            known: true,
            ..ItemKind::default()
        };
        assert!(!arsenal.stow(9, rock, 0x600));
        arsenal.clear();
        assert!(arsenal.shelves.iter().flatten().all(Vec::is_empty));
    }

    fn entry(item: u32, flags: u32) -> ArsenalEntry {
        ArsenalEntry {
            item,
            flags,
            first: 0x28,
            second: 4,
        }
    }

    fn wearing(item: u32, copies: usize) -> Vec<Loadout> {
        use exa_formats::characters::ItemRef;
        let worn = ItemRef { id: item, flags: 0 };
        vec![
            Loadout {
                worn: vec![worn; copies],
                ..Loadout::default()
            };
            2
        ]
    }

    fn holder<'a>(items: &'a mut Vec<u32>, loadouts: &'a mut [Loadout]) -> Holder<'a> {
        Holder {
            items,
            loadouts,
            own: None,
        }
    }

    #[test]
    fn a_sold_copy_leaves_the_flagged_entry_first() {
        let sword = ItemKind {
            known: true,
            weapon: true,
            ..ItemKind::default()
        };
        let mut arsenal = Arsenal::default();
        arsenal.shelves[WEAPONS][1] = vec![entry(7, 0x200), entry(7, 0x400)];
        arsenal.shelves[WEAPONS][3] = vec![entry(7, 0x400), entry(9, 0)];
        let mut manager_items = vec![7, 3];
        let mut manager = wearing(7, 1);
        arsenal.give_up(7, sword, &mut [holder(&mut manager_items, &mut manager)]);
        // Box 3 is searched first but holds no flagged copy; box 1 does.
        assert_eq!(arsenal.shelves[WEAPONS][1], vec![entry(7, 0x400)]);
        assert_eq!(arsenal.shelves[WEAPONS][3].len(), 2);
        // Two copies are left, so the manager keeps his; holding fewer
        // than two, each loadout still takes off its worn copy.
        assert_eq!(manager_items, vec![7, 3]);
        assert!(manager.iter().all(|l| l.worn.is_empty()));
    }

    #[test]
    fn holders_lose_the_copies_past_what_is_left() {
        let sword = ItemKind {
            known: true,
            weapon: true,
            ..ItemKind::default()
        };
        let mut arsenal = Arsenal::default();
        arsenal.shelves[WEAPONS][0] = vec![entry(7, 0x400)];
        let mut first_items = vec![7, 5, 7];
        let mut first = wearing(7, 2);
        let mut second_items = vec![7];
        let mut second = wearing(7, 1);
        arsenal.give_up(
            7,
            sword,
            &mut [
                holder(&mut first_items, &mut first),
                holder(&mut second_items, &mut second),
            ],
        );
        assert!(arsenal.shelves[WEAPONS][0].is_empty());
        // Nothing is left: both copies close up (the last one needs no
        // move); the list keeps three entries.
        assert_eq!(first_items, vec![5, 5, 7]);
        assert!(first.iter().all(|l| l.worn.len() == 1));
        // The allowance fell to −2, so the second holder loses his copy.
        assert_eq!(second_items, vec![7]);
        assert!(second.iter().all(|l| l.worn.is_empty()));
    }

    #[test]
    fn a_unique_item_leaves_everything() {
        let unique = 0x8000_0005;
        let mut arsenal = Arsenal::default();
        arsenal.shelves[ARMOUR][2] = vec![entry(unique, 0), entry(4, 0), entry(unique, 0x400)];
        arsenal.shelves[WEAPONS][0] = vec![entry(unique, 0)];
        let mut items = vec![unique, 4, unique];
        let mut record = wearing(unique, 2);
        arsenal.give_up(
            unique,
            ItemKind::default(),
            &mut [holder(&mut items, &mut record)],
        );
        assert_eq!(arsenal.shelves[ARMOUR][2], vec![entry(4, 0)]);
        assert!(arsenal.shelves[WEAPONS][0].is_empty());
        assert_eq!(items, vec![4]);
        assert!(record.iter().all(|l| l.worn.is_empty()));
        // An ordinary item with no shelf is left alone.
        let mut kept = vec![9];
        arsenal.give_up(
            9,
            ItemKind::default(),
            &mut [holder(&mut kept, &mut record)],
        );
        assert_eq!(kept, vec![9]);
    }

    #[test]
    fn a_trade_gives_up_the_sold_and_shelves_the_bought() {
        let sword = ItemKind {
            known: true,
            weapon: true,
            ..ItemKind::default()
        };
        let kind = |item: u32| match item & 0xff {
            1..=3 => sword,
            4 => apparel(4),
            _ => ItemKind::default(),
        };
        let mut arsenal = Arsenal::default();
        let unique = 0x4000_0001;
        arsenal.shelves[WEAPONS][2] = vec![entry(unique, 0)];
        let mut stash = vec![unique, 9];
        let mut loadouts = wearing(unique, 1);
        let mut carried = vec![unique];
        let mut hands = [unique, 0];
        let mut worn = vec![(unique, 0)];
        let mut kit = vec![unique];
        let mut employee_loadouts = wearing(unique, 1);
        let mut holders = [
            holder(&mut stash, &mut loadouts),
            Holder {
                items: &mut kit,
                loadouts: &mut employee_loadouts,
                own: Some(OwnItems {
                    carried: &mut carried,
                    hands: &mut hands,
                    worn: &mut worn,
                }),
            },
        ];
        // The bought local copy comes back as a world id.
        let promote = |item: u32| {
            if item & 0xc000_0000 == 0x8000_0000 {
                (item & 0x3fff_ffff) | 0x4000_0000
            } else {
                item
            }
        };
        let went = arsenal.trade(
            &[unique],
            &[0x8000_0003, 0x4000_0004, 0x4000_0005],
            kind,
            promote,
            &mut holders,
        );
        assert_eq!(
            went,
            [
                (0x4000_0003, Some(WEAPONS)),
                (0x4000_0004, Some(ARMOUR)),
                (0x4000_0005, None),
            ]
        );
        assert_eq!(arsenal.shelves[WEAPONS][2], vec![]);
        assert_eq!(arsenal.shelves[WEAPONS][0], vec![entry(0x4000_0003, 0)]);
        assert_eq!(arsenal.shelves[ARMOUR][0], vec![entry(0x4000_0004, 0)]);
        assert_eq!(stash, vec![9]);
        assert!(loadouts.iter().all(|l| l.worn.is_empty()));
        assert!(kit.is_empty() && carried.is_empty() && worn.is_empty());
        assert_eq!(hands, [0, 0]);
        assert!(employee_loadouts.iter().all(|l| l.worn.is_empty()));
    }
}
