//! A fighter's per-rank arena loadout and its bridge to the editor.
//!
//! The original keeps one 0x30-byte loadout per rank on the character: a
//! word, the four hand slots and the worn items, the record's own
//! [`Loadout`] shape. The editor's weapon list stands for the hand slots
//! and its armour list for the worn items (guess: the original fills its
//! containers from the loadout through routines outside the form).

use exa_formats::characters::{HANDS, ItemRef, Loadout};
use exa_ui::{LoadoutEditor, LoadoutItem, LoadoutMode, points_used};

use crate::arena;
use crate::career::{Company, Member};

/// The ranks a fighter has loadouts for.
pub const RANKS: usize = 6;

/// One rank's kit: the record's loadout.
pub type CharacterLoadout = Loadout;

/// The editor's items for a loadout: the filled hand slots, then the worn
/// items.
fn lists(
    loadout: &Loadout,
    facts: &impl Fn(u32) -> Option<LoadoutItem>,
) -> (Vec<LoadoutItem>, Vec<LoadoutItem>) {
    let item = |slot: &ItemRef| {
        facts(slot.id).map(|mut item| {
            item.flags = slot.flags;
            item
        })
    };
    let weapons = loadout
        .hands
        .iter()
        .filter(|slot| slot.id != 0)
        .filter_map(item)
        .collect();
    let armour = loadout.worn.iter().filter_map(item).collect();
    (weapons, armour)
}

/// The points a loadout costs, from the facts.
pub fn cost(loadout: &Loadout, facts: &impl Fn(u32) -> Option<LoadoutItem>) -> i32 {
    let (weapons, armour) = lists(loadout, facts);
    points_used(&weapons, &armour)
}

/// The editor for one rank of a member: its kit, that rank's point limit,
/// and the rank as the form's rank.
pub fn editor_for(
    member: &Member,
    rank: usize,
    mode: LoadoutMode,
    facts: &impl Fn(u32) -> Option<LoadoutItem>,
) -> LoadoutEditor {
    let rank = rank.min(RANKS - 1);
    let (weapons, armour) = lists(&member.loadouts[rank], facts);
    let limit = arena::point_limit(rank as u32).unwrap_or(0) as i32;
    LoadoutEditor {
        mode,
        rank,
        points: points_used(&weapons, &armour),
        limit,
        max_rank: rank as i32,
        weapons,
        armour,
        ..LoadoutEditor::default()
    }
}

/// The editor for a company roster member, with the shop's shelf as the
/// arsenal.
pub fn editor_for_company(
    company: &Company,
    member: usize,
    rank: usize,
    mode: LoadoutMode,
    facts: &impl Fn(u32) -> Option<LoadoutItem>,
) -> Option<LoadoutEditor> {
    let roster_member = company.roster.get(member)?;
    let mut editor = editor_for(roster_member, rank, mode, facts);
    editor.arsenal = company.stock.iter().filter_map(|&id| facts(id)).collect();
    Some(editor)
}

/// Writes the editor's lists back into the member's rank when the close
/// saves the kit: the first four weapons into the hand slots, the armour
/// as the worn items. The loadout's word stays.
pub fn save_to(member: &mut Member, editor: &LoadoutEditor) {
    if !exa_ui::save_on_close(editor).saves_kit {
        return;
    }
    let loadout = &mut member.loadouts[editor.rank.min(RANKS - 1)];
    loadout.hands = [ItemRef::NONE; HANDS];
    for (slot, item) in loadout.hands.iter_mut().zip(&editor.weapons) {
        *slot = ItemRef::new(item.id, item.flags);
    }
    loadout.worn = editor
        .armour
        .iter()
        .map(|item| ItemRef::new(item.id, item.flags))
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::{Company, Profession};

    fn facts(id: u32) -> Option<LoadoutItem> {
        match id {
            1 => Some(LoadoutItem::weapon(1, 0x20, 3, 0)),
            2 => Some(LoadoutItem::armour(2, 0x40, 4, 0)),
            _ => None,
        }
    }

    fn member() -> Member {
        let mut company = Company::new("Ash");
        company.hire("Bran", Profession::Combatant).unwrap();
        let mut member = company.roster.remove(0);
        member.skills = [1, 0, 0, 0, 0, 0];
        member
    }

    #[test]
    fn the_editor_takes_the_ranks_kit_and_point_limit() {
        let mut member = member();
        member.loadouts[0].hands[0] = ItemRef::new(1, 0);
        member.loadouts[0].worn = vec![ItemRef::new(2, 0)];
        let editor = editor_for(&member, 0, LoadoutMode::Arena, &facts);
        assert_eq!(editor.mode, LoadoutMode::Arena);
        assert_eq!(editor.rank, 0);
        assert_eq!(editor.weapons.len(), 1);
        assert_eq!(editor.armour.len(), 1);
        assert_eq!(editor.points, 7);
        assert_eq!(editor.limit, 5);
        assert_eq!(cost(&member.loadouts[0], &facts), 7);
        assert_eq!(editor_for(&member, 9, LoadoutMode::Arena, &facts).rank, 5);
    }

    #[test]
    fn the_company_shelf_is_the_arsenal() {
        let mut company = Company::new("Ash");
        company.hire("Bran", Profession::Combatant).unwrap();
        company.stock = vec![1, 2, 99];
        let editor = editor_for_company(&company, 0, 0, LoadoutMode::Arena, &facts).unwrap();
        assert_eq!(editor.arsenal.len(), 2);
        assert_eq!(editor.arsenal[0].id, 1);
        assert!(editor_for_company(&company, 3, 0, LoadoutMode::Arena, &facts).is_none());
    }

    #[test]
    fn saving_writes_the_hands_and_worn_items_back() {
        let mut member = member();
        member.loadouts[2].word = 9;
        member.loadouts[2].hands[0] = ItemRef::new(1, 0);
        let mut editor = editor_for(&member, 2, LoadoutMode::Arena, &facts);
        editor.armour.push(LoadoutItem::armour(2, 0x40, 4, 0));
        save_to(&mut member, &editor);
        assert_eq!(member.loadouts[2].hands[0], ItemRef::new(1, 0));
        assert_eq!(member.loadouts[2].hands[1], ItemRef::NONE);
        assert_eq!(member.loadouts[2].worn, vec![ItemRef::new(2, 0)]);
        assert_eq!(member.loadouts[2].word, 9);
        // Creation writes no kit.
        let mut creation = editor_for(&member, 2, LoadoutMode::Creation, &facts);
        creation.armour.clear();
        save_to(&mut member, &creation);
        assert_eq!(member.loadouts[2].worn.len(), 1);
    }
}
