//! The company's staff: hiring from the list, recruiting a fighter of
//! one's own, dismissing, and what an employee's leaving undoes.
//!
//! The manager is fighter 1 and id 0. Each employee's id is its place on
//! the staff plus one, renumbered after every change; a recruited fighter
//! also carries the control flag. A combatant or a recruit counts as a
//! fighter. A hire is refused when the purse is short, when a combatant
//! finds eight fighters, or when a second merchant, physician or trainer is
//! asked for. It takes the price from the purse and adds it to today's
//! spending, takes the candidate off the list, counts a hire, and logs the
//! joining. A combatant's kit (every hand item and worn item of its six
//! loadouts but item 8, once each) goes to the arsenal with flags 0x400; a
//! specialist's own items go with 0x200. A recruitment costs 12 copper and
//! does not check the room. Leaving releases those flags, drops the id from
//! the tournament entries and renumbers the later ones, and does the same
//! to the log's "saved" records. A dismissal logs the name and counts one
//! more fired. A roster row dragged onto another takes its place (onto
//! nothing, the last place; never the manager's), and the ids that moved
//! follow in the log's saves, the tournament entries and the order list.
//!
//! Evidence: the retail arena's hire, recruitment, renumber, kit, remove
//! and dismiss routines read from `Exanima.exe` (`context/re/arena-staff.md`).

use crate::arena::Roster;
use crate::arena_log::ArenaLog;
use crate::arena_match::{CompanyRecords, LOG_SAVED};
use crate::arsenal::{Arsenal, ItemKind};
use crate::career::Profession;
use crate::hirelings::Loadout;

/// A recruited fighter of one's own, controlled in battle.
pub const RECRUIT: u8 = 1;
/// A hired combatant.
pub const COMBATANT: u8 = 2;
pub const MERCHANT: u8 = 4;
pub const PHYSICIAN: u8 = 8;
pub const TRAINER: u8 = 0x10;
/// Fighters a company holds, the manager included.
pub const MAX_FIGHTERS: u32 = 8;
/// The manage window's hire button stops at this many employees.
pub const MAX_STAFF: usize = 10;
/// Copper a recruitment costs.
pub const RECRUIT_PRICE: u32 = 12;
/// The id flag of a fighter the player controls (the manager and recruits).
pub const CONTROLLED: u32 = 0x1000_0000;
/// A kit item the arsenal holds for a fighter.
pub const FIGHTER_KIT: u32 = 0x400;
/// An item the arsenal holds for a specialist.
pub const SPECIALIST_KIT: u32 = 0x200;
/// The worn item a kit leaves out.
pub const LEFT_OUT: u32 = 8;
/// A worn item with this flag stays when a specialist leaves.
pub const WORN_KEPT: u32 = 0x100;
/// The log's record of a dismissal and of a joining, with the name.
pub const LOG_DISMISSED: i32 = 0x80000;
pub const LOG_JOINED: i32 = 0x90000;

/// Someone on the staff.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Employee {
    /// [`RECRUIT`], [`COMBATANT`], [`MERCHANT`], [`PHYSICIAN`] or [`TRAINER`].
    pub class: u8,
    /// The name's sixteen bytes.
    pub name: [u8; 16],
    /// A fighter's kit, as the arsenal holds it.
    pub kit: Vec<u32>,
    /// A specialist's own items.
    pub carried: Vec<u32>,
    /// A specialist's two hand items.
    pub hands: [u32; 2],
    /// A specialist's worn items and their flags.
    pub worn: Vec<(u32, u32)>,
    /// The world character this employee is registered as.
    pub world: Option<u32>,
}

impl Employee {
    /// A combatant or a recruit.
    pub fn fights(&self) -> bool {
        self.class & 3 != 0
    }
}

/// Why a hire or a recruitment did not happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Funds,
    NoRoom,
    HaveMerchant,
    HavePhysician,
    HaveTrainer,
}

impl Refusal {
    /// The message box's text.
    pub fn message(self) -> &'static str {
        match self {
            Self::Funds => "Insufficient funds",
            Self::NoRoom => "You do not have room for any more fighters.",
            Self::HaveMerchant => "You already have a merchant in your employ.",
            Self::HavePhysician => "You already have a physician in your employ.",
            Self::HaveTrainer => "You already have a trainer in your employ.",
        }
    }
}

/// The hire message box's title.
pub const HIRE_TITLE: &str = "Hire";
/// The recruitment message box's title.
pub const RECRUIT_TITLE: &str = "Recruitment";

/// A candidate taken on: its role, name, own items and loadouts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hire {
    pub role: Option<Profession>,
    pub name: [u8; 16],
    /// A specialist's own items.
    pub carried: Vec<u32>,
    /// A combatant's six loadouts.
    pub loadouts: Vec<Loadout>,
}

/// A kit from loadouts: each loadout's hand items that are set, then its
/// worn items but item 8, each item once, in order.
pub fn kit_of(loadouts: &[Loadout]) -> Vec<u32> {
    let mut kit = Vec::new();
    let mut add = |item: u32| {
        if !kit.contains(&item) {
            kit.push(item);
        }
    };
    for loadout in loadouts {
        for &hand in &loadout.hands {
            if hand != 0 {
                add(hand);
            }
        }
        for &item in &loadout.items {
            if item != LEFT_OUT {
                add(item);
            }
        }
    }
    kit
}

/// What a leaving did beyond the staff list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Leaving {
    /// The world character to unregister.
    pub world: Option<u32>,
    /// A merchant left: his stock is emptied and his figure leaves the hub.
    pub merchant_stock_cleared: bool,
    /// A merchant's or a physician's figure leaves the hub.
    pub figure_removed: bool,
}

/// The company's staff and the manage table's order list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Staff {
    pub employees: Vec<Employee>,
    /// The manage table's order: employee numbers from 1. It only grows
    /// once it holds anything.
    pub order: Vec<i32>,
}

impl Staff {
    /// Fighters, the manager first.
    pub fn fighters(&self) -> u32 {
        1 + self.employees.iter().filter(|e| e.fights()).count() as u32
    }

    /// The manager's id word.
    pub fn manager_id() -> u32 {
        CONTROLLED
    }

    /// An employee's id word: its place plus one, with the control flag for
    /// a recruit.
    pub fn id_word(&self, index: usize) -> u32 {
        let mut word = index as u32 + 1;
        if self
            .employees
            .get(index)
            .is_some_and(|e| e.class & RECRUIT != 0)
        {
            word |= CONTROLLED;
        }
        word
    }

    /// The employee an id's low byte names: 0 is the manager (`None`).
    pub fn by_id(&self, id: u8) -> Option<&Employee> {
        if id == 0 {
            return None;
        }
        self.employees.get(usize::from(id) - 1)
    }

    fn has_class(&self, class: u8) -> bool {
        self.employees.iter().any(|e| e.class == class)
    }

    fn note_order(&mut self) {
        if !self.order.is_empty() {
            self.order.push(self.employees.len() as i32);
        }
    }

    /// Hire a candidate for `price` copper. `pick` is its place on the hire
    /// list, which it leaves.
    #[allow(clippy::too_many_arguments)]
    pub fn hire<T>(
        &mut self,
        hire: Hire,
        price: u32,
        list: &mut Vec<T>,
        pick: Option<usize>,
        purse: &mut u32,
        spent_today: &mut u32,
        records: &mut CompanyRecords,
        log: &mut ArenaLog,
        arsenal: &mut Arsenal,
        kind: impl Fn(u32) -> ItemKind,
    ) -> Result<(), Refusal> {
        if *purse < price {
            return Err(Refusal::Funds);
        }
        match hire.role {
            Some(Profession::Merchant) => {
                if self.has_class(MERCHANT) {
                    return Err(Refusal::HaveMerchant);
                }
            }
            Some(Profession::Physician) => {
                if self.has_class(PHYSICIAN) {
                    return Err(Refusal::HavePhysician);
                }
            }
            Some(Profession::Trainer) => {
                if self.has_class(TRAINER) {
                    return Err(Refusal::HaveTrainer);
                }
            }
            Some(Profession::Combatant) | None => {
                if self.fighters() > MAX_FIGHTERS - 1 {
                    return Err(Refusal::NoRoom);
                }
            }
        }
        *purse -= price;
        *spent_today = spent_today.wrapping_add(price);
        if let Some(pick) = pick
            && pick < list.len()
        {
            list.remove(pick);
        }
        records.hires = records.hires.wrapping_add(1);
        log.append_words(LOG_JOINED, name_words(&hire.name));
        let mut employee = Employee {
            name: hire.name,
            ..Employee::default()
        };
        match hire.role {
            Some(Profession::Merchant | Profession::Physician | Profession::Trainer) => {
                employee.class = match hire.role {
                    Some(Profession::Merchant) => MERCHANT,
                    Some(Profession::Physician) => PHYSICIAN,
                    _ => TRAINER,
                };
                for &item in &hire.carried {
                    arsenal.stow(item, kind(item), SPECIALIST_KIT);
                }
                employee.carried = hire.carried;
            }
            _ => {
                employee.class = COMBATANT;
                employee.kit = kit_of(&hire.loadouts);
                for &item in &employee.kit {
                    arsenal.stow(item, kind(item), FIGHTER_KIT);
                }
            }
        }
        self.employees.push(employee);
        self.note_order();
        Ok(())
    }

    /// Recruit a fighter of one's own for 12 copper. The room is not
    /// checked here (the manage window disables the button at eight
    /// fighters). Its six loadouts are copies of its outfit.
    #[allow(clippy::too_many_arguments)]
    pub fn recruit(
        &mut self,
        name: [u8; 16],
        outfit: &Loadout,
        purse: &mut u32,
        spent_today: &mut u32,
        records: &mut CompanyRecords,
        log: &mut ArenaLog,
        arsenal: &mut Arsenal,
        kind: impl Fn(u32) -> ItemKind,
    ) -> Result<(), Refusal> {
        if *purse < RECRUIT_PRICE {
            return Err(Refusal::Funds);
        }
        *purse -= RECRUIT_PRICE;
        *spent_today = spent_today.wrapping_add(RECRUIT_PRICE);
        let loadouts = vec![outfit.clone(); crate::hirelings::LOADOUTS];
        let employee = Employee {
            class: RECRUIT,
            name,
            kit: kit_of(&loadouts),
            ..Employee::default()
        };
        records.hires = records.hires.wrapping_add(1);
        log.append_words(LOG_JOINED, name_words(&name));
        self.employees.push(employee);
        self.note_order();
        if let Some(added) = self.employees.last() {
            for &item in &added.kit {
                arsenal.stow(item, kind(item), FIGHTER_KIT);
            }
        }
        Ok(())
    }

    /// An employee leaves (a death, a dismissal, a departure).
    pub fn remove(
        &mut self,
        index: usize,
        arsenal: &mut Arsenal,
        kind: impl Fn(u32) -> ItemKind,
        entries: &mut Roster,
        log: &mut ArenaLog,
    ) -> Option<Leaving> {
        let id = index as u32 + 1;
        drop_order(&mut self.order, id as i32);
        let employee = self.employees.get(index)?.clone();
        let mut leaving = Leaving {
            world: employee.world,
            ..Leaving::default()
        };
        if employee.class < 3 {
            for &item in &employee.kit {
                arsenal.release(item, kind(item), FIGHTER_KIT);
            }
        } else {
            for &item in &employee.carried {
                arsenal.release(item, kind(item), SPECIALIST_KIT);
            }
            for &item in &employee.hands {
                arsenal.release(item, kind(item), SPECIALIST_KIT);
            }
            for &(item, flags) in &employee.worn {
                if flags & WORN_KEPT == 0 {
                    arsenal.release(item, kind(item), SPECIALIST_KIT);
                }
            }
        }
        if employee.class == MERCHANT {
            leaving.merchant_stock_cleared = true;
            leaving.figure_removed = true;
        } else if employee.class == PHYSICIAN {
            leaving.figure_removed = true;
        }
        self.employees.remove(index);
        unlink_entries(entries, id);
        renumber_saved(log, id);
        Some(leaving)
    }

    /// Dismiss an employee: log the name, count one more fired, and let
    /// the employee leave.
    pub fn dismiss(
        &mut self,
        index: usize,
        records: &mut CompanyRecords,
        arsenal: &mut Arsenal,
        kind: impl Fn(u32) -> ItemKind,
        entries: &mut Roster,
        log: &mut ArenaLog,
    ) -> Option<Leaving> {
        let name = self.employees.get(index)?.name;
        log.append_words(LOG_DISMISSED, name_words(&name));
        records.fired = records.fired.wrapping_add(1);
        self.remove(index, arsenal, kind, entries, log)
    }

    /// The end of the day's lessons: the first trainer on the staff teaches
    /// the manager (`None`), then every fighter in staff order. No trainer,
    /// no lessons.
    pub fn lesson_round(&self) -> Option<(usize, Vec<Option<usize>>)> {
        let trainer = self.employees.iter().position(|e| e.class == TRAINER)?;
        let mut students = vec![None];
        students.extend(
            self.employees
                .iter()
                .enumerate()
                .filter(|(_, e)| e.fights())
                .map(|(index, _)| Some(index)),
        );
        Some((trainer, students))
    }

    /// Drag a roster row onto another. `row` is the dragged row (0 the
    /// manager, who does not move) and `drop` the row under the cursor
    /// counted from 1 (0 for none). The target is that row, the last row
    /// when there is none, and never the manager's. The rows between close
    /// up, and every id that moved is remapped in the log's "saved"
    /// records, the tournament entries and the order list's entries up to
    /// the staff's count. Returns whether anything moved.
    pub fn move_employee(
        &mut self,
        row: usize,
        drop: i32,
        log: &mut ArenaLog,
        entries: &mut Roster,
    ) -> bool {
        if row == 0 || row > self.employees.len() {
            return false;
        }
        let target = match drop - 1 {
            t if t < 0 => self.employees.len(),
            t => (t as usize).max(1),
        };
        let target = target.min(self.employees.len());
        let mut map: [u32; 11] = std::array::from_fn(|id| id as u32);
        let moved = self.employees.remove(row - 1);
        if row > target {
            for id in target..row {
                if let Some(slot) = map.get_mut(id) {
                    *slot += 1;
                }
            }
        } else {
            for id in row + 1..=target {
                if let Some(slot) = map.get_mut(id) {
                    *slot -= 1;
                }
            }
        }
        if let Some(slot) = map.get_mut(row) {
            *slot = target as u32;
        }
        self.employees.insert(target - 1, moved);
        let remap = |id: u32| map.get(id as usize).copied().unwrap_or(id);
        for record in &mut log.records {
            if record.id == LOG_SAVED {
                record.words[0] = remap(record.words[0] as u32) as i32;
            }
        }
        for event in entries.iter_mut() {
            let count = (event.count as usize).min(event.entries.len());
            for entrant in &mut event.entries[..count] {
                entrant.actor_id = remap(entrant.actor_id);
            }
        }
        let staff = self.employees.len() as i32;
        for number in &mut self.order {
            if *number <= staff {
                *number = remap(*number as u32) as i32;
            }
        }
        true
    }
}

/// A name's sixteen bytes as the log's four words.
pub fn name_words(name: &[u8; 16]) -> [i32; 4] {
    std::array::from_fn(|index| {
        let at = index * 4;
        i32::from_le_bytes([name[at], name[at + 1], name[at + 2], name[at + 3]])
    })
}

/// The order list loses one entry. Scanning all but the last, an entry
/// equal to `number` is closed up and the scan moves past the one that
/// took its place; then the list is one shorter, which drops its last
/// entry when nothing matched. Later numbers are not renumbered.
fn drop_order(order: &mut Vec<i32>, number: i32) {
    let old = order.len();
    if old == 0 {
        return;
    }
    let len = old - 1;
    let mut index = 0;
    while index + 1 < old {
        if order[index] == number {
            order.copy_within(index + 1..old, index);
        }
        index += 1;
    }
    order.truncate(len);
}

/// Drop an id from the tournament entries and move the later ids down.
pub fn unlink_entries(entries: &mut Roster, id: u32) {
    for row in entries.iter_mut() {
        let mut index = 0usize;
        while (index as u32) < row.count {
            let Some(entrant) = row.entries.get(index).copied() else {
                break;
            };
            if entrant.actor_id == id {
                row.count -= 1;
                let count = row.count as usize;
                for at in index..count {
                    if at + 1 < row.entries.len() {
                        row.entries[at] = row.entries[at + 1];
                    }
                }
            } else {
                if id < entrant.actor_id {
                    row.entries[index].actor_id -= 1;
                }
                index += 1;
            }
        }
    }
}

/// Drop the log's "saved" records of an id and move the later ids down.
pub fn renumber_saved(log: &mut ArenaLog, id: u32) {
    let mut index = 0;
    while index < log.records.len() {
        let record = &mut log.records[index];
        if record.id == LOG_SAVED {
            let value = record.words[0] as u32;
            if value == id {
                log.records.remove(index);
                continue;
            }
            if id < value {
                record.words[0] = record.words[0].wrapping_sub(1);
            }
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{Entrant, RosterRow};

    fn name(text: &str) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        bytes
    }

    fn weapon(_: u32) -> ItemKind {
        ItemKind {
            known: true,
            weapon: true,
            ..ItemKind::default()
        }
    }

    fn combatant(text: &str) -> Hire {
        Hire {
            role: Some(Profession::Combatant),
            name: name(text),
            carried: Vec::new(),
            loadouts: vec![
                Loadout {
                    hands: [5, 0],
                    items: vec![8, 9]
                };
                6
            ],
        }
    }

    struct Books {
        purse: u32,
        spent: u32,
        records: CompanyRecords,
        log: ArenaLog,
        arsenal: Arsenal,
    }

    fn books(purse: u32) -> Books {
        Books {
            purse,
            spent: 0,
            records: CompanyRecords::default(),
            log: ArenaLog::default(),
            arsenal: Arsenal::default(),
        }
    }

    fn hire(staff: &mut Staff, books: &mut Books, hire: Hire, price: u32) -> Result<(), Refusal> {
        let mut list = vec![1, 2, 3];
        staff.hire(
            hire,
            price,
            &mut list,
            Some(1),
            &mut books.purse,
            &mut books.spent,
            &mut books.records,
            &mut books.log,
            &mut books.arsenal,
            weapon,
        )
    }

    #[test]
    fn a_kit_takes_each_item_once_but_item_8() {
        let loadouts = [
            Loadout {
                hands: [5, 0],
                items: vec![8, 9, 10],
            },
            Loadout {
                hands: [6, 5],
                items: vec![10, 11],
            },
        ];
        assert_eq!(kit_of(&loadouts), vec![5, 9, 10, 6, 11]);
    }

    #[test]
    fn a_hire_pays_logs_and_stows_the_kit() {
        let mut staff = Staff::default();
        let mut books = books(20);
        assert_eq!(
            hire(&mut staff, &mut books, combatant("Ash"), 21),
            Err(Refusal::Funds)
        );
        assert_eq!(Refusal::Funds.message(), "Insufficient funds");
        assert_eq!(hire(&mut staff, &mut books, combatant("Ash"), 20), Ok(()));
        assert_eq!((books.purse, books.spent, books.records.hires), (0, 20, 1));
        assert_eq!(staff.employees[0].class, COMBATANT);
        assert_eq!(staff.employees[0].kit, vec![5, 9]);
        assert_eq!(books.log.records[0].id, LOG_JOINED);
        assert_eq!(books.log.records[0].words[0], 0x0068_7341);
        let shelf = &books.arsenal.shelves[crate::arsenal::WEAPONS][0];
        assert_eq!(
            shelf.iter().map(|e| (e.item, e.flags)).collect::<Vec<_>>(),
            vec![(5, 0x400), (9, 0x400)]
        );
        assert_eq!(staff.fighters(), 2);
        assert_eq!(staff.id_word(0), 1);
    }

    #[test]
    fn eight_fighters_leave_no_room() {
        let mut staff = Staff::default();
        let mut books = books(1000);
        for _ in 0..7 {
            hire(&mut staff, &mut books, combatant("Ash"), 1).unwrap();
        }
        assert_eq!(staff.fighters(), 8);
        assert_eq!(
            hire(&mut staff, &mut books, combatant("Bo"), 1),
            Err(Refusal::NoRoom)
        );
        // A specialist still fits.
        let trainer = Hire {
            role: Some(Profession::Trainer),
            name: name("Cy"),
            carried: vec![40],
            loadouts: Vec::new(),
        };
        assert_eq!(hire(&mut staff, &mut books, trainer.clone(), 1), Ok(()));
        assert_eq!(staff.employees[7].class, TRAINER);
        assert_eq!(
            hire(&mut staff, &mut books, trainer, 1),
            Err(Refusal::HaveTrainer)
        );
        assert_eq!(
            Refusal::HaveTrainer.message(),
            "You already have a trainer in your employ."
        );
        assert_eq!(books.purse, 992);
    }

    #[test]
    fn a_recruit_costs_12_and_does_not_check_the_room() {
        let mut staff = Staff::default();
        let mut books = books(11);
        let outfit = Loadout {
            hands: [3, 4],
            items: vec![8, 7],
        };
        let recruit = |staff: &mut Staff, books: &mut Books| {
            staff.recruit(
                name("Dee"),
                &outfit,
                &mut books.purse,
                &mut books.spent,
                &mut books.records,
                &mut books.log,
                &mut books.arsenal,
                weapon,
            )
        };
        assert_eq!(recruit(&mut staff, &mut books), Err(Refusal::Funds));
        books.purse = 12 * 9;
        for _ in 0..8 {
            recruit(&mut staff, &mut books).unwrap();
        }
        assert_eq!(staff.fighters(), 9);
        assert_eq!(books.purse, 12);
        assert_eq!(staff.employees[0].kit, vec![3, 4, 7]);
        assert_eq!(staff.id_word(2), 3 | CONTROLLED);
        assert_eq!(Staff::manager_id(), 0x1000_0000);
    }

    #[test]
    fn leaving_releases_unlinks_and_renumbers() {
        let mut staff = Staff::default();
        let mut books = books(100);
        for who in ["A", "B", "C"] {
            hire(&mut staff, &mut books, combatant(who), 1).unwrap();
        }
        let mut entries: Roster = Default::default();
        entries[0] = RosterRow {
            count: 2,
            entries: [
                Entrant {
                    actor_id: 2,
                    weapon: [0; 2],
                },
                Entrant {
                    actor_id: 3,
                    weapon: [0; 2],
                },
            ],
            team_id: 0,
        };
        books.log.append_value(LOG_SAVED, 2);
        books.log.append_value(LOG_SAVED, 3);
        books.log.append_value(LOG_SAVED, 1);
        let leaving = staff
            .remove(1, &mut books.arsenal, weapon, &mut entries, &mut books.log)
            .unwrap();
        assert_eq!(leaving, Leaving::default());
        assert_eq!(staff.employees.len(), 2);
        assert_eq!(entries[0].count, 1);
        assert_eq!(entries[0].entries[0].actor_id, 2);
        let saved: Vec<i32> = books
            .log
            .records
            .iter()
            .filter(|r| r.id == LOG_SAVED)
            .map(|r| r.words[0])
            .collect();
        assert_eq!(saved, vec![2, 1]);
        // Three hires stowed 5 and 9 each; one fighter released its two.
        let flags: Vec<u32> = books.arsenal.shelves[crate::arsenal::WEAPONS][0]
            .iter()
            .map(|e| e.flags)
            .collect();
        assert_eq!(flags, vec![0, 0, 0x400, 0x400, 0x400, 0x400]);
    }

    #[test]
    fn a_merchant_leaving_empties_his_stock() {
        let mut staff = Staff::default();
        let mut books = books(100);
        let merchant = Hire {
            role: Some(Profession::Merchant),
            name: name("Mo"),
            carried: vec![30],
            loadouts: Vec::new(),
        };
        hire(&mut staff, &mut books, merchant.clone(), 1).unwrap();
        assert_eq!(
            hire(&mut staff, &mut books, merchant, 1),
            Err(Refusal::HaveMerchant)
        );
        staff.employees[0].worn = vec![(31, 0x100), (32, 0)];
        let mut entries: Roster = Default::default();
        let leaving = staff
            .dismiss(
                0,
                &mut books.records,
                &mut books.arsenal,
                weapon,
                &mut entries,
                &mut books.log,
            )
            .unwrap();
        assert!(leaving.merchant_stock_cleared && leaving.figure_removed);
        assert_eq!(books.records.fired, 1);
        assert_eq!(books.log.records.last().unwrap().id, LOG_DISMISSED);
        assert_eq!(
            books.arsenal.shelves[crate::arsenal::WEAPONS][0][0].flags,
            0
        );
        assert!(staff.employees.is_empty());
        assert!(
            staff
                .dismiss(
                    0,
                    &mut books.records,
                    &mut books.arsenal,
                    weapon,
                    &mut entries,
                    &mut books.log
                )
                .is_none()
        );
    }

    #[test]
    fn the_order_list_shortens_by_one() {
        let mut order = vec![1, 2, 3, 4];
        drop_order(&mut order, 2);
        assert_eq!(order, vec![1, 3, 4]);
        // No match drops the last entry.
        let mut order = vec![1, 2, 3];
        drop_order(&mut order, 9);
        assert_eq!(order, vec![1, 2]);
        // A lone entry is dropped whatever it holds.
        let mut order = vec![5];
        drop_order(&mut order, 1);
        assert!(order.is_empty());
        // Two equal entries in a row: the second slides into the first's
        // place and is passed over.
        let mut order = vec![2, 2, 3];
        drop_order(&mut order, 2);
        assert_eq!(order, vec![2, 3]);
        // An empty list stays empty and does not grow on a hire.
        let mut staff = Staff::default();
        let mut books = books(10);
        hire(&mut staff, &mut books, combatant("A"), 1).unwrap();
        assert!(staff.order.is_empty());
        staff.order = vec![1];
        hire(&mut staff, &mut books, combatant("B"), 1).unwrap();
        assert_eq!(staff.order, vec![1, 2]);
    }

    fn staff_of(names: &[&str]) -> Staff {
        Staff {
            employees: names
                .iter()
                .map(|text| Employee {
                    class: COMBATANT,
                    name: name(text),
                    ..Employee::default()
                })
                .collect(),
            order: Vec::new(),
        }
    }

    fn names(staff: &Staff) -> Vec<String> {
        staff
            .employees
            .iter()
            .map(|e| crate::report::name_text(&e.name))
            .collect()
    }

    #[test]
    fn a_dragged_row_moves_and_its_ids_follow() {
        let mut staff = staff_of(&["A", "B", "C", "D"]);
        staff.order = vec![4, 1, 2, 7];
        let mut log = ArenaLog::default();
        log.append_value(LOG_SAVED, 4);
        log.append_value(LOG_SAVED, 2);
        log.append_value(crate::arena_match::LOG_WON, 4);
        let mut entries = Roster::default();
        entries[0] = RosterRow {
            count: 2,
            entries: [
                Entrant {
                    actor_id: 4,
                    weapon: [0; 2],
                },
                Entrant {
                    actor_id: 3,
                    weapon: [0; 2],
                },
            ],
            team_id: 0,
        };
        // D (row 4) dropped on row 2 lands in the place of B.
        assert!(staff.move_employee(4, 3, &mut log, &mut entries));
        assert_eq!(names(&staff), ["A", "D", "B", "C"]);
        assert_eq!(staff.order, vec![2, 1, 3, 7]);
        assert_eq!(log.records[0].words[0], 2);
        assert_eq!(log.records[1].words[0], 3);
        assert_eq!(log.records[2].words[0], 4);
        assert_eq!(entries[0].entries[0].actor_id, 2);
        assert_eq!(entries[0].entries[1].actor_id, 4);
    }

    #[test]
    fn a_row_dropped_nowhere_goes_last_and_never_above_the_manager() {
        let mut staff = staff_of(&["A", "B", "C"]);
        let mut log = ArenaLog::default();
        let mut entries = Roster::default();
        assert!(staff.move_employee(1, 0, &mut log, &mut entries));
        assert_eq!(names(&staff), ["B", "C", "A"]);
        assert!(staff.move_employee(3, 1, &mut log, &mut entries));
        assert_eq!(names(&staff), ["A", "B", "C"]);
        assert!(!staff.move_employee(0, 2, &mut log, &mut entries));
        assert!(staff.move_employee(2, 3, &mut log, &mut entries));
        assert_eq!(names(&staff), ["A", "B", "C"]);
    }

    #[test]
    fn a_trainer_teaches_the_manager_then_the_fighters() {
        let mut staff = staff_of(&["A", "B", "C", "D"]);
        assert_eq!(staff.lesson_round(), None);
        staff.employees[1].class = TRAINER;
        staff.employees[2].class = MERCHANT;
        staff.employees[3].class = RECRUIT;
        assert_eq!(
            staff.lesson_round(),
            Some((1, vec![None, Some(0), Some(3)]))
        );
    }
}
