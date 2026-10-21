//! A fighter's per-rank arena loadout and its bridge to the editor.
//!
//! The original keeps one `TCharacterLoadout` of twelve 32-bit words per
//! rank (six ranks at the character's `+0x108`); the editor fills its two
//! containers from the rank's record on open and writes it back on close.
//! Exarust keeps the item ids the containers hold: weapons and armour.
//! The record's exact byte layout is not in the pack, so the four lists and
//! the cached point total are Exarust's own reading (guess).

use exa_ui::{LoadoutEditor, LoadoutItem, LoadoutMode, points_used};

use crate::arena;
use crate::career::Member;

/// The ranks a fighter has loadouts for.
pub const RANKS: usize = 6;

/// One rank's kit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterLoadout {
    /// Weapon item ids.
    pub weapons: Vec<u32>,
    /// Armour item ids.
    pub armour: Vec<u32>,
    /// The points the kit was last counted at.
    pub points: i32,
}

impl CharacterLoadout {
    /// The editor's items from a list of ids and their facts.
    pub fn items(ids: &[u32], facts: &impl Fn(u32) -> Option<LoadoutItem>) -> Vec<LoadoutItem> {
        ids.iter().filter_map(|&id| facts(id)).collect()
    }

    /// The points the kit costs, from the facts.
    pub fn cost(&self, facts: &impl Fn(u32) -> Option<LoadoutItem>) -> i32 {
        let weapons = Self::items(&self.weapons, facts);
        let armour = Self::items(&self.armour, facts);
        points_used(&weapons, &armour)
    }
}

/// The editor for one rank of a member.
pub fn editor_for(
    member: &Member,
    rank: usize,
    mode: LoadoutMode,
    facts: &impl Fn(u32) -> Option<LoadoutItem>,
) -> LoadoutEditor {
    let rank = rank.min(RANKS - 1);
    let loadout = &member.loadouts[rank];
    let tier = arena::loadout_tier(member.skills.iter().sum::<u32>());
    let limit = arena::point_limit(tier).unwrap_or(0) as i32;
    let weapons = CharacterLoadout::items(&loadout.weapons, facts);
    let armour = CharacterLoadout::items(&loadout.armour, facts);
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

/// Writes the editor's lists back into the member's rank.
pub fn save_to(member: &mut Member, editor: &LoadoutEditor) {
    let rank = editor.rank.min(RANKS - 1);
    let loadout = &mut member.loadouts[rank];
    loadout.weapons = editor.weapons.iter().map(|item| item.id).collect();
    loadout.armour = editor.armour.iter().map(|item| item.id).collect();
    loadout.points = editor.points;
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
        member.loadouts[0].weapons = vec![1];
        member.loadouts[0].armour = vec![2];
        let editor = editor_for(&member, 0, LoadoutMode::Arena, &facts);
        assert_eq!(editor.mode, LoadoutMode::Arena);
        assert_eq!(editor.rank, 0);
        assert_eq!(editor.weapons.len(), 1);
        assert_eq!(editor.armour.len(), 1);
        assert_eq!(editor.points, 7);
        // Rank points 1 gives tier 0, the 5-point limit.
        assert_eq!(editor.limit, 5);
        assert_eq!(member.loadouts[0].cost(&facts), 7);
        // Out-of-range ranks clamp.
        assert_eq!(editor_for(&member, 9, LoadoutMode::Arena, &facts).rank, 5);
    }

    #[test]
    fn saving_writes_ids_and_points_back() {
        let mut member = member();
        member.loadouts[2].weapons = vec![1];
        let mut editor = editor_for(&member, 2, LoadoutMode::Arena, &facts);
        editor.armour.push(LoadoutItem::armour(2, 0x40, 4, 0));
        editor.points = 7;
        save_to(&mut member, &editor);
        assert_eq!(member.loadouts[2].weapons, vec![1]);
        assert_eq!(member.loadouts[2].armour, vec![2]);
        assert_eq!(member.loadouts[2].points, 7);
    }
}
