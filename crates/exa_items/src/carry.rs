//! What a character carries, wears and keeps in a container.
//!
//! Putting an item on takes off every worn item whose occupancy overlaps
//! it on any bone and layer. Displaced braies are dropped, not carried.
//! A human with nothing over the groin wears braies again. A container
//! and a pack exchange items by moving the same id from one list to the
//! other. Evidence: `docs/formats/OTHER.md` (Occupancy table, Underwear).

use exa_formats::objdb::{GROIN, OCCUPANCY_BONES};

/// The underwear item. Displaced copies are dropped, never carried.
pub const BRAIES: u32 = 8;

/// One worn piece: its object-database id and the occupancy table.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub id: u32,
    pub occupancy: [u64; OCCUPANCY_BONES],
}

/// Whether two occupancy tables share a sub-part on the same layer of any bone.
pub fn overlaps(a: &[u64], b: &[u64]) -> bool {
    a.iter().zip(b.iter()).any(|(&left, &right)| {
        (0..8).any(|layer| {
            let mask = 0xFFu64 << (layer * 8);
            (left & mask) & (right & mask) != 0
        })
    })
}

/// Whether the pelvis's innermost layer covers the groin.
pub fn covers_groin(occupancy: &[u64]) -> bool {
    occupancy
        .first()
        .is_some_and(|pelvis| *pelvis as u8 & GROIN != 0)
}

/// Braies: pelvis bits 0 and 1 on layer 0, and bit 0 of each thigh.
pub fn braies() -> Piece {
    let mut occupancy = [0; OCCUPANCY_BONES];
    occupancy[0] = 0x03;
    occupancy[1] = 0x01;
    occupancy[2] = 0x01;
    Piece {
        id: BRAIES,
        occupancy,
    }
}

/// Moves `id` from `from` to `into`. False when `from` does not hold it.
pub fn transfer(from: &mut Vec<u32>, into: &mut Vec<u32>, id: u32) -> bool {
    let Some(at) = from.iter().position(|held| *held == id) else {
        return false;
    };
    from.remove(at);
    into.push(id);
    true
}

/// Puts `piece` on. Overlapping worn items come off: braies are dropped,
/// anything else returns to `carried`. The piece leaves `carried` when it
/// was there. A human left bare at the groin wears `underwear` again.
pub fn equip(
    worn: &mut Vec<Piece>,
    carried: &mut Vec<u32>,
    piece: Piece,
    human: bool,
    underwear: &Piece,
) {
    let incoming = piece.occupancy;
    let mut displaced = Vec::new();
    worn.retain(|old| {
        if overlaps(&old.occupancy, &incoming) {
            displaced.push(old.id);
            false
        } else {
            true
        }
    });
    for id in displaced {
        if id != BRAIES {
            carried.push(id);
        }
    }
    if let Some(at) = carried.iter().position(|id| *id == piece.id) {
        carried.remove(at);
    }
    worn.push(piece);
    restore_underwear(worn, human, underwear);
}

/// Takes `id` off. Anything but braies returns to `carried`. A human left
/// bare at the groin wears `underwear` again. False when it was not worn.
pub fn unequip(
    worn: &mut Vec<Piece>,
    carried: &mut Vec<u32>,
    id: u32,
    human: bool,
    underwear: &Piece,
) -> bool {
    let mut removed = false;
    worn.retain(|old| {
        if old.id != id {
            return true;
        }
        removed = true;
        if id != BRAIES {
            carried.push(id);
        }
        false
    });
    restore_underwear(worn, human, underwear);
    removed
}

fn restore_underwear(worn: &mut Vec<Piece>, human: bool, underwear: &Piece) {
    if !human || worn.iter().any(|piece| covers_groin(&piece.occupancy)) {
        return;
    }
    if worn.iter().any(|piece| piece.id == BRAIES) {
        return;
    }
    worn.push(underwear.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pelvis(layer: u8, bits: u64) -> Piece {
        let mut occupancy = [0; OCCUPANCY_BONES];
        occupancy[0] = bits << (layer * 8);
        Piece { id: 0, occupancy }
    }

    #[test]
    fn trousers_drop_braies_and_taking_them_off_brings_braies_back() {
        let underwear = braies();
        let mut worn = vec![underwear.clone()];
        let mut carried = vec![20];
        let mut trousers = pelvis(0, 0x03);
        trousers.id = 20;
        equip(&mut worn, &mut carried, trousers, true, &underwear);
        assert_eq!(worn.iter().map(|p| p.id).collect::<Vec<_>>(), vec![20]);
        assert!(carried.is_empty(), "{carried:?}");
        assert!(unequip(&mut worn, &mut carried, 20, true, &underwear));
        assert_eq!(worn.iter().map(|p| p.id).collect::<Vec<_>>(), vec![BRAIES]);
        assert_eq!(carried, vec![20]);
    }

    #[test]
    fn a_shirt_on_another_layer_stays_and_a_coat_sends_it_to_the_pack() {
        let underwear = braies();
        let mut shirt = pelvis(2, 0x01);
        shirt.id = 30;
        let mut worn = vec![underwear.clone(), shirt];
        let mut carried = vec![40];
        let mut coat = pelvis(2, 0x01);
        coat.id = 40;
        equip(&mut worn, &mut carried, coat, true, &underwear);
        let ids: Vec<_> = worn.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![BRAIES, 40]);
        assert_eq!(carried, vec![30]);
    }

    #[test]
    fn a_container_and_a_pack_exchange_an_item_before_it_is_worn() {
        let underwear = braies();
        let mut chest = vec![20u32, 21];
        let mut pack = Vec::new();
        assert!(transfer(&mut chest, &mut pack, 20));
        assert!(!transfer(&mut chest, &mut pack, 20));
        assert_eq!(chest, vec![21]);
        assert_eq!(pack, vec![20]);
        let mut trousers = pelvis(0, 0x03);
        trousers.id = 20;
        let mut worn = vec![underwear.clone()];
        equip(&mut worn, &mut pack, trousers, true, &underwear);
        assert!(pack.is_empty());
        assert_eq!(worn.iter().map(|p| p.id).collect::<Vec<_>>(), vec![20]);
        assert!(!transfer(&mut pack, &mut chest, 20));
        unequip(&mut worn, &mut pack, 20, true, &underwear);
        assert!(transfer(&mut pack, &mut chest, 20));
        assert_eq!(chest, vec![21, 20]);
        assert!(pack.is_empty());
    }
}
