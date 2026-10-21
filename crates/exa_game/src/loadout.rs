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

/// Where the loadout editor takes the record it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordSource {
    /// The manager (page 0, no employee selected).
    Manager,
    /// A roster employee, zero-based.
    Employee(usize),
    /// A hire-list candidate, zero-based.
    Candidate(usize),
    /// A tournament event's lineup (page 3).
    Event,
    /// A specialist's record page (page 2).
    Specialist(usize),
}

/// What binding a record to the editor did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundRecord {
    /// The record is a copy (the hire page) rather than the live record.
    pub copy: bool,
    /// The six loadouts were missing and were rebuilt.
    pub repaired: bool,
    /// The loadout rank the form shows.
    pub rank: usize,
    /// The two hand items page 3 writes back, otherwise unchanged.
    pub hands: [u64; 2],
}

/// Bind one record to the editor (the original's bind step).
///
/// A fighter's record whose profession class is at most the combatant class
/// and which does not hold six loadouts is repaired first. The hire page
/// (1) takes a copy; every other page keeps the live record. The form's
/// rank selects the record's loadout only when the record is a fighter and
/// the rank is within its loadout count. On the event page (3) the record's
/// two hand items become the lineup's saved hands.
#[must_use]
pub fn bind_record(
    page: u32,
    profession_class: u32,
    loadout_count: usize,
    form_rank: usize,
    saved_hands: [u64; 2],
) -> BoundRecord {
    let fighter = profession_class < 0x2000_0001;
    let repaired = fighter && loadout_count != RANKS;
    let rank = if fighter && form_rank < loadout_count {
        form_rank
    } else {
        0
    };
    BoundRecord {
        copy: page == 1,
        repaired,
        rank,
        hands: if page == 3 { saved_hands } else { [0, 0] },
    }
}

/// What the editor opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorOpen {
    pub source: RecordSource,
    /// The roster or hire-list row the form's index names, zero-based.
    pub row: usize,
    /// The event rank on the event page.
    pub rank: usize,
}

/// Open the editor on a page and row (the original's open step).
///
/// Page 0 is the roster: index 0 (or less) is the manager, a positive index
/// is that employee less one. Page 1 is the hire list at the form's row.
/// Page 2 is an employee's specialist record. Page 3 is a tournament event,
/// opened at the event's rank.
#[must_use]
pub fn open_editor(page: u32, index: i32, event_rank: u32) -> EditorOpen {
    let row = usize::try_from(index.max(0)).unwrap_or(0);
    match page {
        1 => EditorOpen {
            source: RecordSource::Candidate(row),
            row,
            rank: 0,
        },
        2 => EditorOpen {
            source: RecordSource::Specialist(row.saturating_sub(1)),
            row: row.saturating_sub(1),
            rank: 0,
        },
        3 => EditorOpen {
            source: RecordSource::Event,
            row: 0,
            rank: event_rank as usize,
        },
        0 if index < 1 => EditorOpen {
            source: RecordSource::Manager,
            row: 0,
            rank: 0,
        },
        0 => EditorOpen {
            source: RecordSource::Employee(row.saturating_sub(1)),
            row: row.saturating_sub(1),
            rank: 0,
        },
        _ => EditorOpen {
            source: RecordSource::Manager,
            row: 0,
            rank: 0,
        },
    }
}

/// Which list the editor fills its second pane from when a record is bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListSource {
    /// The record's own worn items (the roster and specialist pages).
    Record,
    /// The hub stash (page 0 when no employee is selected).
    Stash,
    /// The selected employee's own list.
    EmployeeList(usize),
}

/// The list the editor reloads when the record is bound.
///
/// The hire page keeps its containers. Page 2 and the other non-hub pages
/// show the record's own items; page 0 with no selected employee shows the
/// hub stash, otherwise the selected employee's list.
#[must_use]
pub fn reload_source(page: u32, selected_employee: i32) -> ListSource {
    if page == 1 {
        return ListSource::Record;
    }
    if page == 2 {
        return ListSource::Record;
    }
    if selected_employee < 1 {
        ListSource::Stash
    } else {
        ListSource::EmployeeList((selected_employee - 1) as usize)
    }
}

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

    #[test]
    fn binding_repairs_a_short_fighter_record_and_copies_the_hire_page() {
        let bound = bind_record(0, 0x2000_0000, 4, 3, [11, 22]);
        assert!(bound.repaired);
        assert!(!bound.copy);
        assert_eq!(bound.rank, 3);
        assert_eq!(bound.hands, [0, 0]);
        let hire = bind_record(1, 0x2000_0000, 6, 3, [11, 22]);
        assert!(hire.copy);
        assert!(!hire.repaired);
        assert_eq!(hire.rank, 3);
        let event = bind_record(3, 0x2000_0000, 6, 9, [11, 22]);
        assert_eq!(event.rank, 0);
        assert_eq!(event.hands, [11, 22]);
        // A specialist has no loadout rank and is never repaired.
        let specialist = bind_record(2, 0x4000_0000, 0, 4, [1, 2]);
        assert!(!specialist.repaired);
        assert_eq!(specialist.rank, 0);
    }

    #[test]
    fn opening_picks_the_manager_employee_candidate_or_event() {
        assert_eq!(
            open_editor(0, 0, 0),
            EditorOpen {
                source: RecordSource::Manager,
                row: 0,
                rank: 0
            }
        );
        assert_eq!(open_editor(0, 3, 0).source, RecordSource::Employee(2));
        assert_eq!(open_editor(1, 4, 0).source, RecordSource::Candidate(4));
        assert_eq!(open_editor(2, 3, 0).source, RecordSource::Specialist(2));
        assert_eq!(open_editor(3, 0, 5).rank, 5);
        assert_eq!(open_editor(9, 0, 0).source, RecordSource::Manager);
        assert_eq!(open_editor(0, -2, 0).source, RecordSource::Manager);
    }

    #[test]
    fn reloading_the_second_pane_follows_the_page_and_selection() {
        assert_eq!(reload_source(0, 0), ListSource::Stash);
        assert_eq!(reload_source(0, 2), ListSource::EmployeeList(1));
        assert_eq!(reload_source(1, 0), ListSource::Record);
        assert_eq!(reload_source(2, 5), ListSource::Record);
    }
}
