//! The item panels of a character's loot window and the moves between a body
//! and the player.
//!
//! A placed character's record keeps everything it carries, wears and holds:
//! the carried list with its saved cell positions, the worn items and the four
//! hand slots. [`Possessions`] is that record as the running game keeps it,
//! and this module turns it into the two panels of the original's "Body"
//! window: the pack loads the carried list as a character list (its own
//! positions, in sixteenths of a GUI unit, [`Container::load_list`]) and the
//! equipment panel loads the worn items and the two items of the set in the
//! hands with the worn/scatter pass ([`exa_game::item_container::worn_layout`],
//! seeded with the panel's own [`DOLL_SEED`]).
//!
//! Taking an item moves it between the two sides' [`Possessions`]: a worn
//! item leaves the worn list, a hand item clears its hand slot and the held
//! mark of a matching carried entry, and a carried item leaves the carried
//! list. The destination pack appends it at the drop's cell. The original
//! also re-dresses the body when an apparel item comes off; Exarust's
//! corpse bodies keep their built outfit, so the mesh is not re-dressed yet
//! (a documented deviation).
//!
//! The windows' panel geometry, the accept test and the drop are the item
//! container's reviewed rules; the live mouse path is
//! [`exa_game::panel_live`].

use bevy::prelude::*;
use exa_formats::characters::ItemRef;
use exa_items::catalogue::ItemCatalogue;
use exa_items::possession::{Carried, HELD, Hand, Possessions};

use crate::item_container::{
    Cell, Container, DOLL_SEED, Entry, ListChild, to_list_pos, worn_layout,
};
use crate::item_panel_draw::PanelIcons;

/// Which of a record's two item sides an entry came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    /// The carried list (the pack panel).
    Pack,
    /// The worn items and the two items of the set in the hands (the
    /// equipment panel).
    Equipment,
}

/// The pack's entries as the record saves them: the item, its flags and its
/// cell in sixteenths of a GUI unit (the record's third inventory word).
pub fn pack_children(possessions: &Possessions) -> Vec<ListChild> {
    possessions
        .carried
        .iter()
        .map(|carried| ListChild {
            id: carried.id,
            flags: carried.flags,
            x: carried.x,
            y: carried.y,
        })
        .collect()
}

/// What the equipment panel lists, in the original's order: every worn item
/// in wearing order, then the two hand slots of the set in the hands. Empty
/// hand slots are left out; a worn item keeps the flags its record stores.
pub fn equipment_items(possessions: &Possessions) -> Vec<(u32, u32)> {
    possessions
        .worn
        .iter()
        .map(|worn| (worn.id, worn.flags))
        .chain(
            possessions
                .out
                .iter()
                .filter(|hand| hand.id != 0)
                .map(|hand| (hand.id, hand.flags)),
        )
        .collect()
}

/// The bulk a panel counts for an item: the model section's `0x24` byte,
/// which the game's item database holds (`equip_lock` in `exa_formats`). A
/// level-local id has no catalogue record yet, so it counts 0; the original
/// promotes such a record into the world database when the level loads.
fn item_bulk(id: u32, catalogue: Option<&ItemCatalogue>) -> i32 {
    catalogue
        .and_then(|catalogue| catalogue.bulk(id))
        .map_or(0, i32::from)
}

/// Fills a pack panel from a character's carried list. The item resolver
/// gives every entry an icon slot (a reserved one when the item renderer has
/// not filled the cache) and its bulk; [`Container::load_list`] then places
/// and clamps the entries. Returns the number of entries loaded.
pub fn bind_pack(
    container: &mut Container,
    possessions: &Possessions,
    icons: &mut PanelIcons,
    catalogue: Option<&ItemCatalogue>,
) -> usize {
    let children = pack_children(possessions);
    // Reserve every icon before the load, so the half-extent lookup can
    // borrow the cache while the resolver hands out the slots. A reserved
    // slot has the panel icon's halves; a rendered one keeps its own.
    let mut slots = children
        .iter()
        .map(|child| icons.reserve(child.id).0)
        .collect::<Vec<_>>()
        .into_iter();
    container.load_list(
        &children,
        |id, _| Some((slots.next().unwrap_or(0), item_bulk(id, catalogue))),
        |slot| icons.half(slot),
    )
}

/// Fills a body's equipment panel with the worn/scatter pass: every worn
/// item and every item in a hand, placed at the panel's random spot and
/// pushed apart over the original's four rounds. `client` is the panel's
/// client area and `icon_size` its icon's side, as the painter draws it.
/// Returns the number of entries loaded.
pub fn bind_equipment(
    container: &mut Container,
    possessions: &Possessions,
    icons: &mut PanelIcons,
    client: (i32, i32),
) -> usize {
    let items = equipment_items(possessions);
    let mut state = DOLL_SEED;
    container.entries = worn_layout(
        &items,
        &mut |id| Some(icons.reserve(id)),
        client,
        container.icon_size,
        &mut state,
    );
    container.used = 0;
    container.loaded = true;
    container.entries.len()
}

/// Takes `id` off a character's possessions from one of its two sides and
/// returns the flags the item carried. The equipment side takes the first
/// worn copy, else the hand slot holding it (the off hand first); a hand
/// item's held mark also leaves the carried list (the original's
/// take-equipped rule). The pack side takes the last carried copy, as the
/// original's remove does.
pub fn take(possessions: &mut Possessions, kind: PanelKind, id: u32) -> Option<u32> {
    match kind {
        PanelKind::Equipment => {
            if let Some(at) = possessions.worn.iter().position(|worn| worn.id == id) {
                return Some(possessions.worn.remove(at).flags);
            }
            if let Some(hand) = possessions.out.iter_mut().rev().find(|hand| hand.id == id) {
                let flags = hand.flags;
                *hand = Hand::default();
                // The item in a hand is still listed in the pack, marked
                // held; the original's take-equipped removes that copy.
                if let Some(at) = possessions
                    .carried
                    .iter()
                    .rposition(|carried| carried.id == id && carried.flags & HELD != 0)
                {
                    possessions.carried.remove(at);
                }
                return Some(flags & !HELD);
            }
            None
        }
        PanelKind::Pack => {
            let at = possessions
                .carried
                .iter()
                .rposition(|carried| carried.id == id)?;
            Some(possessions.carried.remove(at).flags & !HELD)
        }
    }
}

/// Gives an item to a character's carried list at the cell a drop landed in,
/// converting the cell to the record's sixteenths of a GUI unit.
pub fn give(
    possessions: &mut Possessions,
    id: u32,
    flags: u32,
    cell: Cell,
    unit: i32,
    vscale: f32,
) {
    let (x, y) = to_list_pos(cell, unit, vscale);
    possessions.carried.push(Carried { id, flags, x, y });
}

/// Moves an item from one character's side to another character's pack, as
/// a drop on the destination's pack panel does. Returns the flags the item
/// left with, or `None` when the source does not hold it.
pub fn transfer(
    from: &mut Possessions,
    kind: PanelKind,
    id: u32,
    to: &mut Possessions,
    cell: Cell,
    unit: i32,
    vscale: f32,
) -> Option<u32> {
    let flags = take(from, kind, id)?;
    give(to, id, flags, cell, unit, vscale);
    Some(flags)
}

/// The equipment panel's entries for a record when there is no icon cache
/// to reserve slots from (a level-load preview or a test). It draws the same
/// random numbers as [`bind_equipment`] for the same record: the slot is the
/// item id and the half extents a square icon's.
pub fn equipment_layout(
    possessions: &Possessions,
    client: (i32, i32),
    icon_size: i32,
) -> Vec<Entry> {
    let items = equipment_items(possessions);
    let mut state = DOLL_SEED;
    worn_layout(
        &items,
        &mut |id| Some((id as usize, (icon_size / 2, icon_size / 2))),
        client,
        icon_size,
        &mut state,
    )
}

/// Gives every character that has a live record but no possessions the
/// record's own, once. The level spawn already does this for placed
/// characters and corpses; the pass covers a record that reaches the world
/// by another path (a saved body or a spectator's).
pub fn ensure_possessions(
    mut commands: Commands,
    characters: Query<(Entity, &exa_character::record::CharacterData), Without<Possessions>>,
) {
    for (entity, data) in &characters {
        commands
            .entity(entity)
            .insert(Possessions::of_record(&data.record));
    }
}

/// The build-at-load pass.
pub struct BodyInventoryPlugin;

impl Plugin for BodyInventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, ensure_possessions);
    }
}

/// The item a record's worn list holds, for a test or a caller that wants
/// the pair rather than the id.
pub fn worn_item(possessions: &Possessions, id: u32) -> Option<ItemRef> {
    possessions.worn.iter().copied().find(|worn| worn.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::characters::{Carried as RecordCarried, CharacterRecord};
    use exa_items::possession::{HELD, LIT, SECOND_SET};

    fn record() -> CharacterRecord {
        CharacterRecord {
            hands: [
                ItemRef::new(0x11, 0),
                ItemRef::new(0x12, LIT),
                ItemRef::new(0x13, 0),
                ItemRef::new(0x14, 0),
            ],
            worn: vec![ItemRef::new(0xa0, 0), ItemRef::new(0xa1, 4)],
            inventory: vec![
                RecordCarried {
                    item: 0x30,
                    flags: HELD,
                    extra: 0x0004_0028,
                },
                RecordCarried {
                    item: 0x31,
                    flags: 0,
                    extra: 0x0010_0008,
                },
            ],
            profession_rest: SECOND_SET,
            ..CharacterRecord::default()
        }
    }

    /// A pack load with a resolver that pretends the renderer filled every
    /// slot: the slot is the item id and the half extents a forty-pixel
    /// icon.
    fn bind_pack_for_test(container: &mut Container, possessions: &Possessions) {
        let children = pack_children(possessions);
        container.load_list(
            &children,
            |id, _| Some((id as usize, 0)),
            |_| Some((20, 20)),
        );
    }

    #[test]
    fn a_records_items_are_the_packs_children_and_the_equipment_list() {
        let p = Possessions::of_record(&record());
        let children = pack_children(&p);
        assert_eq!(
            children,
            vec![
                ListChild {
                    id: 0x30,
                    flags: HELD,
                    x: 0x28,
                    y: 4
                },
                ListChild {
                    id: 0x31,
                    flags: 0,
                    x: 8,
                    y: 0x10
                },
            ]
        );
        // Worn first, then only the hand slots with an item: the stowed set
        // is not on the body's doll.
        assert_eq!(
            equipment_items(&p),
            vec![(0xa0, 0), (0xa1, 4), (0x11, 0), (0x12, LIT)]
        );
        assert_eq!(worn_item(&p, 0xa1).unwrap().flags, 4);
    }

    #[test]
    fn the_pack_panel_loads_the_saved_places() {
        let p = Possessions::of_record(&record());
        let mut container = Container::form(4);
        bind_pack_for_test(&mut container, &p);
        assert_eq!(container.entries.len(), 2);
        assert_eq!(container.entries[0].id, 0x30);
        assert_eq!(container.entries[0].flags, HELD);
        // 0x28/16 × 4 = 10, 4/16 × 4 = 1.
        assert_eq!(container.entries[0].cell, Cell::new(10, 1));
        assert_eq!(container.entries[1].cell, Cell::new(2, 4));
        assert_eq!(container.entries[1].id, 0x31);
    }

    #[test]
    fn the_equipment_panel_lists_worn_and_held_items_with_their_flags() {
        let p = Possessions::of_record(&record());
        let entries = equipment_layout(&p, (228, 252), 160);
        let items: Vec<(u32, u32)> = entries.iter().map(|e| (e.id, e.flags)).collect();
        assert_eq!(items, equipment_items(&p));
        // Every centre stays inside the panel.
        for entry in &entries {
            let (cx, cy) = (entry.cell.x + 80, entry.cell.y + 80);
            assert!((20..=208).contains(&cx), "{entry:?}");
            assert!((20..=232).contains(&cy), "{entry:?}");
        }
    }

    #[test]
    fn taking_off_the_equipment_removes_the_worn_item_or_the_hand() {
        let mut p = Possessions::of_record(&record());
        assert_eq!(take(&mut p, PanelKind::Equipment, 0xa1), Some(4));
        assert_eq!(equipment_items(&p), vec![(0xa0, 0), (0x11, 0), (0x12, LIT)]);
        assert_eq!(take(&mut p, PanelKind::Equipment, 0xa1), None);
        // The off hand goes first and its held carried copy with it.
        p.carried.push(Carried {
            id: 0x12,
            flags: HELD,
            x: 0,
            y: 0,
        });
        assert_eq!(take(&mut p, PanelKind::Equipment, 0x12), Some(LIT));
        assert_eq!(p.out[1], Hand::default());
        assert_eq!(p.out[0].id, 0x11);
        assert!(p.carried.iter().all(|carried| carried.id != 0x12));
        // The pack takes the last copy and clears the held mark.
        assert_eq!(take(&mut p, PanelKind::Pack, 0x31), Some(0));
        assert!(p.carried.iter().all(|carried| carried.id != 0x31));
        assert_eq!(p.carried.len(), 1);
    }

    #[test]
    fn a_transfer_moves_an_item_between_two_characters() {
        let mut body = Possessions::of_record(&record());
        let mut player = Possessions::default();
        let flags = transfer(
            &mut body,
            PanelKind::Equipment,
            0xa0,
            &mut player,
            Cell::new(40, 60),
            4,
            1.0,
        );
        assert_eq!(flags, Some(0));
        assert_eq!(body.worn, vec![ItemRef::new(0xa1, 4)]);
        assert_eq!(
            player.carried,
            vec![Carried {
                id: 0xa0,
                flags: 0,
                x: 160,
                y: 240
            }]
        );
        // The pack's held copy leaves without its held mark when it goes to
        // the player's list.
        let flags = transfer(
            &mut body,
            PanelKind::Pack,
            0x30,
            &mut player,
            Cell::new(1, 2),
            4,
            1.0,
        );
        assert_eq!(flags, Some(0));
        assert_eq!(player.carried[1].id, 0x30);
        assert_eq!(player.carried[1].flags, 0);
        // An item the body does not hold moves nothing.
        assert_eq!(
            transfer(
                &mut body,
                PanelKind::Pack,
                0x99,
                &mut player,
                Cell::new(0, 0),
                4,
                1.0
            ),
            None
        );
    }

    #[test]
    fn the_load_pass_gives_a_recorded_character_its_possessions() {
        let mut app = App::new();
        app.add_plugins(BodyInventoryPlugin);
        let data = exa_character::record::CharacterData::of_record(record());
        let entity = app.world_mut().spawn(data).id();
        app.update();
        let p = app.world().get::<Possessions>(entity).unwrap();
        assert_eq!(p.out[0].id, 0x11);
        assert_eq!(p.worn.len(), 2);
    }

    /// The installed `exanima01`'s dead bodies (with `EXANIMA_DIR`): a
    /// corpse's loot window lists exactly the items its record carries, at
    /// the record's places. The check is skipped without an installation.
    #[test]
    fn a_corpses_loot_window_lists_its_records_items() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(game) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let Some(pack) = game.pack(exa_assets::PackKind::Resource) else {
            return;
        };
        let Some(bytes) = pack.get("exanima01.rfc") else {
            return;
        };
        let Ok(scene) = exa_formats::rfc::Scene::parse(bytes) else {
            return;
        };
        let Some(characters) = &scene.characters else {
            return;
        };
        let mut checked = 0;
        for placement in &characters.placements {
            if !placement.is_dead() {
                continue;
            }
            let Some(record) = characters.record(placement.id) else {
                continue;
            };
            if record.inventory.is_empty() {
                continue;
            }
            let possessions = Possessions::of_record(record);
            let mut icons = PanelIcons::default();
            let mut pack_panel = Container::form(4);
            bind_pack(&mut pack_panel, &possessions, &mut icons, None);
            assert_eq!(
                pack_panel.entries.len(),
                record.inventory.len(),
                "{} carries {} items",
                record.name,
                record.inventory.len()
            );
            for (entry, stored) in pack_panel.entries.iter().zip(&record.inventory) {
                assert_eq!(entry.id, stored.item);
                assert_eq!(entry.flags, stored.flags);
            }
            let mut equipment_panel = Container::form(4);
            let count = bind_equipment(&mut equipment_panel, &possessions, &mut icons, (228, 252));
            assert_eq!(count, equipment_items(&possessions).len());
            checked += 1;
        }
        assert!(checked > 0, "exanima01 has no dead body with items");
    }
}
