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
    /// The manager (selection 0 on the roster and event pages).
    Manager,
    /// A roster employee, zero-based.
    Employee(usize),
    /// A hire-list candidate, zero-based.
    Candidate(usize),
    /// The open step leaves the shown record alone (the specialist page
    /// and any other page value have no branch of their own).
    Unchanged,
}

/// What binding a record to the editor did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundRecord {
    /// The record is a copy (the hire page) rather than the live record.
    pub copy: bool,
    /// The six loadouts were missing and were rebuilt.
    pub repaired: bool,
    /// The rank whose loadout is applied to the bound record, when the
    /// record is a fighter and the form's tab is inside its loadout list.
    pub applied_rank: Option<usize>,
    /// The two hand items the event page writes into the record, otherwise
    /// none.
    pub hands: Option<[u64; 2]>,
}

/// Bind one record to the editor (the original's bind step).
///
/// A fighter's record (profession class at most the combatant class) that
/// does not hold six loadouts is repaired first, which leaves it six. The
/// hire page (1) takes a copy; every other page keeps the live record. The
/// kit of the form's kept rank is applied to a fighter's record only when
/// the form's tab is below the record's loadout count (the count after the
/// repair); the tab and the kept rank are two separate fields of the form.
/// On the event page (3) the record's two hand items become the lineup's
/// saved hands.
#[must_use]
pub fn bind_record(
    page: u32,
    profession_class: u32,
    loadout_count: usize,
    tab: usize,
    kept_rank: usize,
    saved_hands: [u64; 2],
) -> BoundRecord {
    let fighter = profession_class < 0x2000_0001;
    let repaired = fighter && loadout_count != RANKS;
    let count = if repaired { RANKS } else { loadout_count };
    BoundRecord {
        copy: page == 1,
        repaired,
        applied_rank: (fighter && tab < count).then_some(kept_rank),
        hands: (page == 3).then_some(saved_hands),
    }
}

/// What the editor opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorOpen {
    pub source: RecordSource,
    /// The selection after clamping to the list.
    pub selection: usize,
    /// The event rank fixed on the slider (the event page only).
    pub rank: Option<usize>,
}

/// Open the editor on a page and selection (the original's open step).
///
/// The selection is clamped to `0..=last`, where `last` is the candidate
/// count less one on the hire page and the employee count otherwise. The
/// roster page (0) and the event page (3) show the manager at selection 0
/// and the employee one before the selection otherwise; the event page also
/// fixes the event's rank on the slider. The hire page (1) shows the
/// candidate at the selection. Any other page shows nothing new.
#[must_use]
pub fn open_editor(page: u32, selection: i32, last: usize, event_rank: u32) -> EditorOpen {
    let selection = usize::try_from(selection.max(0)).unwrap_or(0).min(last);
    let by_selection = if selection == 0 {
        RecordSource::Manager
    } else {
        RecordSource::Employee(selection - 1)
    };
    let (source, rank) = match page {
        0 => (by_selection, None),
        1 => (RecordSource::Candidate(selection), None),
        3 => (by_selection, Some(event_rank as usize)),
        _ => (RecordSource::Unchanged, None),
    };
    EditorOpen {
        source,
        selection,
        rank,
    }
}

/// Which list the editor fills its second pane from when a record is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListSource {
    /// The shown record's own worn items (the specialist page).
    Record,
    /// The hub stash (selection 0).
    Stash,
    /// The selected employee's own list.
    EmployeeList(usize),
}

/// The list the editor reloads when the record is shown, or `None` when the
/// page keeps its containers.
///
/// The hire page (1) keeps them. Page 2 shows the record's own items. The
/// other pages show the hub stash at selection 0 and otherwise the selected
/// employee's list.
#[must_use]
pub fn reload_source(page: u32, selection: i32) -> Option<ListSource> {
    match page {
        1 => None,
        2 => Some(ListSource::Record),
        _ if selection < 1 => Some(ListSource::Stash),
        _ => Some(ListSource::EmployeeList((selection - 1) as usize)),
    }
}

/// The text and meters the editor's refresh writes.
#[derive(Clone, Debug, PartialEq)]
pub struct FormRefresh {
    /// "Name (Rank)" or "Name (Profession)".
    pub title: String,
    /// The hire page's "Profession: ..." line; empty on the other pages.
    pub profession: String,
    /// The hire fee in copper; zero off the hire page.
    pub price: u32,
    /// The daily wage; zero off the hire page.
    pub wage: u32,
    /// The two health meters, from the record's two health words.
    pub health: (f32, f32),
}

/// One loadout-form refresh (the original's record-view routine).
///
/// A specialist's title is its profession; a fighter's is its rank name.
/// The hire page (1) shows "Profession: ..." and the hire fee
/// ([`hire_fee`](crate::career::hire_fee) of the fighter's rank and the
/// kit value), and the daily wage. The health meters are each word times one
/// over 65535 ([`manage::health_fill`](super::manage::health_fill)).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn refresh(
    page: u32,
    name: &str,
    profession: Option<crate::career::Profession>,
    rank: usize,
    kit_value: u32,
    wage: u32,
    first_health: u16,
    second_health: u16,
) -> FormRefresh {
    let rank = rank.min(RANKS - 1);
    let hire = page == 1;
    FormRefresh {
        title: super::manage::record_title(name, profession, rank),
        profession: if hire {
            super::manage::profession_line(profession).to_string()
        } else {
            String::new()
        },
        price: if hire {
            crate::career::hire_fee(rank as u32, kit_value)
        } else {
            0
        },
        wage: if hire { wage } else { 0 },
        health: (
            super::manage::health_fill(first_health),
            super::manage::health_fill(second_health),
        ),
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

/// The equipment form's preview mode state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoadoutPreview {
    /// The mode word the form stores.
    pub mode: u32,
    /// The viewport's width and height words.
    pub viewport: [u32; 2],
    /// The camera distance word.
    pub distance: i32,
    /// The camera's scale word.
    pub scale: f32,
    /// The five equipment slots' visibility words.
    pub slots: [u32; 5],
    /// The loadout tab whose points line is shown.
    pub points_tab: i32,
    /// The portrait layout is used (the mode is 2).
    pub portrait: bool,
    /// The body preview is switched on (an odd mode other than 2, when the
    /// form has a body preview).
    pub body_preview: bool,
}

/// The form's preview mode and its five slot words, at GUI `unit` (4 at the
/// base scale).
///
/// Mode 2 is the portrait layout: a 0x200 by 0x200 viewport, the camera at
/// `unit · 0x2d` (180 at unit 4), scale 1, every slot visible (0x110) and
/// no points line. Every other mode is the equipment layout: a 0x400 by 0
/// viewport, the camera at `unit · 0x7b` (492 at unit 4) times `zoom` in
/// single precision, converted to the nearest integer (ties to even), that
/// zoom as the scale, and the points line at `points_tab`. Modes other than
/// 3 hide the slots (0); mode 3 keeps them as they were. An odd mode
/// switches the body preview on.
pub fn preview(
    mode: u32,
    unit: i32,
    zoom: f32,
    points_tab: i32,
    slots: [u32; 5],
) -> LoadoutPreview {
    if mode == 2 {
        return LoadoutPreview {
            mode,
            viewport: [0x200, 0x200],
            distance: unit.wrapping_mul(0x2d),
            scale: 1.0,
            slots: [0x110; 5],
            points_tab: -1,
            portrait: true,
            body_preview: false,
        };
    }
    let distance = ((i64::from(unit) * 0x7b) as f32 * zoom).round_ties_even();
    let distance = if distance.is_nan() || distance.abs() >= 9.223_372e18 {
        i64::MIN
    } else {
        distance as i64
    } as i32;
    LoadoutPreview {
        mode,
        viewport: [0x400, 0],
        distance,
        scale: zoom,
        slots: if mode == 3 { slots } else { [0; 5] },
        points_tab,
        portrait: false,
        body_preview: mode & 1 != 0,
    }
}

/// The editor's item for an object database record: the rank word's top
/// three bits are the rank it needs and its low five the points it costs
/// (the same two fields the armour rule reads). Apparel is worn; anything
/// else is held. The weapon family (`0x10` one-handed, `0x20`/`0x30`
/// two-handed) is the caller's: it is the catalogue record's own field, not
/// part of [`ItemFacts`]. An item id of zero is not an item.
///
/// Guess: the apparel test is the record's class byte being non-zero; the
/// original reads the item's kind word, which the facts here do not carry.
pub fn item_from_facts(
    facts: &crate::arena_outfit::ItemFacts,
    family: u32,
    two_handed: bool,
) -> Option<LoadoutItem> {
    if facts.id.0 == 0 {
        return None;
    }
    // The rank and the bulk as the pool rows read them.
    let points = i32::from(facts.bulk());
    let rank = i32::from((facts.rank_word as u8) >> 5);
    let item = if facts.apparel_class != 0 {
        LoadoutItem::armour(facts.id.0, family, points, rank)
    } else if two_handed {
        LoadoutItem::two_handed(facts.id.0, family, points, rank)
    } else {
        LoadoutItem::weapon(facts.id.0, family, points, rank)
    };
    Some(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::{Company, Profession};

    #[test]
    fn the_preview_mode_sets_the_viewport_camera_and_slots() {
        let portrait = preview(2, 4, 0.5, 4, [9; 5]);
        assert_eq!(portrait.viewport, [0x200, 0x200]);
        assert_eq!(portrait.distance, 180);
        assert_eq!(portrait.scale, 1.0);
        assert_eq!(portrait.slots, [0x110; 5]);
        assert_eq!(portrait.points_tab, -1);
        assert!(portrait.portrait);
        let equipment = preview(1, 4, 0.5, 4, [9; 5]);
        assert_eq!(equipment.viewport, [0x400, 0]);
        assert_eq!(equipment.distance, 246);
        assert_eq!(equipment.scale, 0.5);
        assert_eq!(equipment.slots, [0; 5]);
        assert_eq!(equipment.points_tab, 4);
        assert!(!equipment.portrait);
        assert!(equipment.body_preview);
        assert!(!portrait.body_preview);
        // Mode 3 keeps the slots it was given; mode 0 hides them and has no
        // body preview.
        let kept = preview(3, 4, 1.0, 0, [7; 5]);
        assert_eq!(kept.slots, [7; 5]);
        assert_eq!(kept.distance, 492);
        assert!(!preview(0, 4, 1.0, 0, [7; 5]).body_preview);
        // The camera scales with the unit: 3 · 123 · 0.5 = 184.5, to even.
        assert_eq!(preview(1, 3, 0.5, 0, [0; 5]).distance, 184);
        assert_eq!(preview(2, 3, 0.5, 0, [0; 5]).distance, 135);
    }

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
        let bound = bind_record(0, 0x2000_0000, 4, 3, 3, [11, 22]);
        assert!(bound.repaired);
        assert!(!bound.copy);
        // The repair leaves six loadouts, so tab 5 is inside the list.
        assert_eq!(
            bind_record(0, 0x2000_0000, 4, 5, 5, [0, 0]).applied_rank,
            Some(5)
        );
        assert_eq!(bound.applied_rank, Some(3));
        assert_eq!(bound.hands, None);
        let hire = bind_record(1, 0x2000_0000, 6, 3, 2, [11, 22]);
        assert!(hire.copy);
        assert!(!hire.repaired);
        // The tab picks whether a kit is applied, the kept rank which one.
        assert_eq!(hire.applied_rank, Some(2));
        assert_eq!(
            bind_record(0, 0x2000_0000, 6, 6, 2, [0, 0]).applied_rank,
            None
        );
        let event = bind_record(3, 0x2000_0000, 6, 3, 3, [11, 22]);
        assert_eq!(event.hands, Some([11, 22]));
        // A specialist is never repaired and takes no kit.
        let specialist = bind_record(2, 0x4000_0000, 0, 0, 0, [1, 2]);
        assert!(!specialist.repaired);
        assert_eq!(specialist.applied_rank, None);
    }

    #[test]
    fn opening_picks_the_manager_employee_candidate_or_event() {
        let roster = open_editor(0, 0, 4, 0);
        assert_eq!(roster.source, RecordSource::Manager);
        assert_eq!(roster.rank, None);
        assert_eq!(open_editor(0, 3, 4, 0).source, RecordSource::Employee(2));
        // The selection clamps to the list.
        let far = open_editor(0, 9, 4, 0);
        assert_eq!(far.source, RecordSource::Employee(3));
        assert_eq!(far.selection, 4);
        assert_eq!(open_editor(0, -2, 4, 0).source, RecordSource::Manager);
        assert_eq!(open_editor(1, 4, 6, 0).source, RecordSource::Candidate(4));
        // The event page shows the selected record at the event's rank.
        let event = open_editor(3, 2, 4, 5);
        assert_eq!(event.source, RecordSource::Employee(1));
        assert_eq!(event.rank, Some(5));
        assert_eq!(open_editor(2, 3, 4, 0).source, RecordSource::Unchanged);
    }

    #[test]
    fn reloading_the_second_pane_follows_the_page_and_selection() {
        assert_eq!(reload_source(0, 0), Some(ListSource::Stash));
        assert_eq!(reload_source(0, 2), Some(ListSource::EmployeeList(1)));
        assert_eq!(reload_source(3, 1), Some(ListSource::EmployeeList(0)));
        assert_eq!(reload_source(1, 0), None);
        assert_eq!(reload_source(2, 5), Some(ListSource::Record));
    }

    #[test]
    fn item_facts_give_the_rank_and_the_points_of_a_record() {
        use crate::arena_outfit::{ItemFacts, ItemId};
        let facts = ItemFacts {
            id: ItemId(77),
            // Rank 3 in the low byte's top three bits, 9 in its low five.
            rank_word: 0x0069,
            ..ItemFacts::default()
        };
        let weapon = item_from_facts(&facts, 0x10, false).unwrap();
        assert_eq!((weapon.id, weapon.rank, weapon.points), (77, 3, 9));
        assert!(!weapon.worn && !weapon.two_handed);
        assert!(item_from_facts(&facts, 0x20, true).unwrap().two_handed);
        let worn = ItemFacts {
            apparel_class: 4,
            ..facts.clone()
        };
        let armour = item_from_facts(&worn, 0x40, false).unwrap();
        assert!(armour.worn);
        assert_eq!((armour.rank, armour.points), (3, 9));
        assert!(item_from_facts(&ItemFacts::default(), 0, false).is_none());
    }

    #[test]
    fn a_refresh_names_a_specialist_and_prices_a_hire() {
        let fighter = refresh(0, "Bran", Some(Profession::Combatant), 2, 0, 5, 32_767, 0);
        assert_eq!(fighter.title, "Bran (Novice)");
        assert_eq!(fighter.profession, "");
        assert_eq!(fighter.price, 0);
        assert_eq!(fighter.wage, 0);
        assert_eq!(fighter.health.0, super::super::manage::health_fill(32_767));
        let hire = refresh(
            1,
            "Mira",
            Some(Profession::Physician),
            2,
            100,
            12,
            257,
            65_535,
        );
        assert_eq!(hire.title, "Mira (Physician)");
        assert_eq!(hire.profession, "Profession: Physician");
        // (2 + 1)^2 * 12 + round(100 * 4 / 5) = 108 + 80.
        assert_eq!(hire.price, 188);
        assert_eq!(hire.wage, 12);
        assert_eq!(hire.health.0.to_bits(), 0x3b80_8081);
        assert_eq!(hire.health.1, 1.0);
        // The fee is the shared hire rule.
        assert_eq!(hire.price, crate::career::hire_fee(2, 100));
    }
}
