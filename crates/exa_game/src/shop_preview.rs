//! The trade window's character preview: the drop that dresses it from the
//! window's item panels, and the drag that takes an item off it.
//!
//! The opener copies the player's record and the preview shows the copy
//! (its pack panel is bound to the copy's carried list). The preview is the
//! character view: the trade window gives it no shown character and no
//! linked pack, sets its lock byte, and installs two callbacks of its own
//! where the inventory has others:
//!
//! - **The drop** (the view's own): an apparel is worn after the equip
//!   conflict pass when the race wears apparel (a locked piece in the way
//!   refuses it), a weapon or a shield is taken into the hands by the
//!   take-in rule. Nothing is handed to a second pack (there is none, and
//!   the view is locked). The source panel's drag then ends with the
//!   payload gone from it (the pack is not linked to the view).
//! - **The drop-back** (the conflict pass's callback for each piece it
//!   takes off): a piece other than the braies ([`KEPT_KIND`]) that is not
//!   the shop's goes back into the pack at its centre.
//! - **The give-back** (the take-in's callback for an item it moves out of
//!   a hand): an item that is not the shop's goes into the pack at its
//!   centre.
//! - **The release** of a drag off the preview: a shop item's drag
//!   (flag `0x80000000`) is answered -1 by the lock, so it comes off
//!   wherever it lands, even nowhere; any other comes off only when a
//!   panel took it. A worn piece leaves the worn list (the underwear rule
//!   then runs for a human), a held item leaves its hand by the scheduled
//!   take-out.
//!
//! So the player can try the shop's goods on and take them off into the
//! buy basket, and dress from the pack. The "Trade" button's commit of the
//! copy is outside this unit.
//!
//! Evidence: the trade window's constructor and opener, its drop-back and
//! give-back callbacks, and the character view's accept, drop and release
//! (`Exanima.exe` 0.9.5, checked against the assembly).

use exa_character::record::CharacterData;
use exa_formats::characters::{CharacterRecord, ItemRef};
use exa_formats::objdb::Item;
use exa_items::possession::{self, Hand, Possessions};
use exa_ui::character_view::{DropFacts, FigureDrop, figure_drop};

use crate::hub_windows::SHOP_ITEM_FLAGS;
use crate::loadout_kit::PreviewItem;
use crate::shop_window::KEPT_KIND;

/// A drag off the preview: the item its pick found, the icon reference
/// the drag holds and the grab (half the icon).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewDrag {
    pub item: PreviewItem,
    pub icon: usize,
    pub grab: crate::item_container::Cell,
}

/// The preview's number as a drag's destination (the panels use 1 to 4).
pub const PREVIEW_PANEL: u32 = 5;

/// The trade window's drop-back: a piece the conflict pass took off goes
/// to the pack unless it is the braies or the shop's.
pub fn drop_back(id: u32, flags: u32) -> bool {
    id != KEPT_KIND && flags & SHOP_ITEM_FLAGS == 0
}

/// The trade window's give-back: an item moved out of a hand goes to the
/// pack unless it is the shop's.
pub fn give_back(flags: u32) -> bool {
    flags & SHOP_ITEM_FLAGS == 0
}

/// Whether the release of a drag off the preview takes the item off: the
/// locked view answers -1 for a shop item, so it always comes off; any
/// other comes off only when a panel took it.
pub fn release_takes_off(flags: u32, landed: bool) -> bool {
    landed || flags & SHOP_ITEM_FLAGS != 0
}

/// What a drop on the preview did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dressed {
    pub kind: FigureDrop,
    /// Items the pack takes at its centre (id, flags): the drop-back's and
    /// the give-back's.
    pub to_pack: Vec<(u32, u32)>,
}

fn hands_of(record: &CharacterRecord) -> Possessions {
    let hand = |r: ItemRef| Hand {
        id: r.id,
        flags: r.flags,
    };
    Possessions {
        out: [hand(record.hands[0]), hand(record.hands[1])],
        stowed: [hand(record.hands[2]), hand(record.hands[3])],
        worn: record.worn.clone(),
        ..Possessions::default()
    }
}

fn set_hands(record: &mut CharacterRecord, p: &Possessions) {
    record.hands = [p.out[0], p.out[1], p.stowed[0], p.stowed[1]]
        .map(|hand| ItemRef::new(hand.id, hand.flags));
}

/// The record's possessions as the preview's pick reads them.
pub fn possessions(record: &CharacterRecord) -> Possessions {
    hands_of(record)
}

/// The view's drop of `id` (with its panel's `flags`) on the preview's
/// record. `race_flags` is the race's look word (bit 4: it wears
/// apparel). `None` when the item has no section the drop takes, or the
/// conflict pass refused it.
pub fn dress<'a>(
    record: &mut CharacterRecord,
    id: u32,
    flags: u32,
    race_flags: u32,
    find: impl Fn(u32) -> Option<&'a Item>,
    kind_of: impl Fn(u32) -> Option<possession::ItemKind>,
) -> Option<Dressed> {
    let item = find(id);
    let kind = kind_of(id).unwrap_or_default();
    let plan = figure_drop(DropFacts {
        id,
        flags,
        locked: true,
        has_apparel: item.is_some_and(|item| item.burden.is_some() && !item.worn_model.is_empty()),
        has_weapon: kind.weapon.is_some(),
        has_shield: kind.shield,
        race_wears_apparel: race_flags & 4 != 0,
        ..DropFacts::default()
    });
    match plan.kind {
        FigureDrop::Apparel => {
            let mut data = CharacterData::of_record(CharacterRecord {
                worn: record.worn.clone(),
                ..CharacterRecord::default()
            });
            let taken = data.equip_conflicts(item?, &find)?;
            record.worn = data.record.worn;
            record.worn.push(ItemRef::new(id, flags));
            Some(Dressed {
                kind: plan.kind,
                to_pack: taken
                    .iter()
                    .filter(|piece| drop_back(piece.id, piece.flags))
                    .map(|piece| (piece.id, piece.flags))
                    .collect(),
            })
        }
        FigureDrop::Weapon | FigureDrop::Shield => {
            let mut p = hands_of(record);
            let before = p.out;
            if plan.kind == FigureDrop::Weapon {
                crate::doll_view::hold_item(&mut p, id, flags, kind, false, &kind_of);
            } else {
                crate::doll_view::hold_shield(&mut p, id, flags);
            }
            set_hands(record, &p);
            // The hands' former items the take-in moved out.
            let to_pack = before
                .iter()
                .filter(|old| old.id != 0 && !p.out.iter().any(|now| now.id == old.id))
                .filter(|old| give_back(old.flags))
                .map(|old| (old.id, old.flags))
                .collect();
            Some(Dressed {
                kind: plan.kind,
                to_pack,
            })
        }
        _ => None,
    }
}

/// The view's release on the preview's record: a held item leaves its
/// hand by the scheduled take-out, a worn piece the worn list (the
/// underwear rule running for a human). Answers whether it came off.
pub fn take_off<'a>(
    record: &mut CharacterRecord,
    item: PreviewItem,
    find: impl Fn(u32) -> Option<&'a Item>,
    kind_of: impl Fn(u32) -> Option<possession::ItemKind>,
) -> bool {
    if let Some(hand) = item.hand {
        let mut p = hands_of(record);
        if p.out.get(hand).is_none_or(|h| h.id != item.id) {
            return false;
        }
        let kind = kind_of(item.id).unwrap_or_default();
        let mut after = p.clone();
        after.out[hand] = Hand::default();
        let off = (hand == 0 && after.out[1].id != 0).then(|| {
            (
                after.out[1].id,
                kind_of(after.out[1].id).unwrap_or_default(),
            )
        });
        let released = exa_items::doll::take_out(
            &mut p,
            &exa_items::doll::TakeOut::scheduled(hand),
            Some((item.id, crate::figure_pick::held_place(hand, kind))),
            off,
        );
        set_hands(record, &p);
        return released.done;
    }
    let Some(at) = record.worn.iter().position(|w| w.id == item.id) else {
        return false;
    };
    record.worn.remove(at);
    let mut data = CharacterData::of_record(record.clone());
    if data.ensure_underwear(&find) {
        record.worn = data.record.worn;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_callbacks_keep_the_braies_and_the_shops_goods_out_of_the_pack() {
        assert!(drop_back(23, 0));
        assert!(!drop_back(KEPT_KIND, 0));
        assert!(!drop_back(23, SHOP_ITEM_FLAGS));
        assert!(give_back(0x100));
        assert!(!give_back(SHOP_ITEM_FLAGS | 0x100));
    }

    #[test]
    fn a_shop_item_comes_off_wherever_it_lands() {
        assert!(release_takes_off(SHOP_ITEM_FLAGS, false));
        assert!(release_takes_off(0, true));
        assert!(!release_takes_off(0, false));
    }

    fn weapon_kind(id: u32) -> Option<possession::ItemKind> {
        (id >= 100).then(|| possession::ItemKind {
            weapon: Some(Default::default()),
            ..possession::ItemKind::default()
        })
    }

    #[test]
    fn a_weapon_goes_into_the_hand_and_the_old_one_back_to_the_pack() {
        let mut record = CharacterRecord::default();
        record.hands[0] = ItemRef::new(100, 0x100);
        let done = dress(&mut record, 101, 0, 4, |_| None, weapon_kind).unwrap();
        assert_eq!(done.kind, FigureDrop::Weapon);
        assert_eq!(record.hands[0].id, 101);
        assert_eq!(done.to_pack, vec![(100, 0x100)]);
        // A shop weapon displaced stays out of the pack.
        let done = dress(&mut record, 102, SHOP_ITEM_FLAGS, 4, |_| None, weapon_kind).unwrap();
        assert_eq!(record.hands[0].id, 102);
        assert_eq!(done.to_pack, vec![(101, 0)]);
        let done = dress(&mut record, 103, 0, 4, |_| None, weapon_kind).unwrap();
        assert!(done.to_pack.is_empty());
    }

    #[test]
    fn a_held_item_comes_off_its_hand() {
        let mut record = CharacterRecord::default();
        record.hands[0] = ItemRef::new(100, 0);
        let item = PreviewItem {
            id: 100,
            flags: 0,
            hand: Some(0),
        };
        assert!(take_off(&mut record, item, |_| None, weapon_kind));
        assert_eq!(record.hands[0].id, 0);
        assert!(!take_off(&mut record, item, |_| None, weapon_kind));
    }

    #[test]
    fn an_item_without_a_section_is_not_taken() {
        let mut record = CharacterRecord::default();
        assert!(dress(&mut record, 5, 0, 4, |_| None, |_| None).is_none());
    }
}
