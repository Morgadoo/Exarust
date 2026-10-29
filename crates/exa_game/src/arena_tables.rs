//! The live arena state and the save's arena tables.
//!
//! The arena's save writes the company's own chunks after the journal: the
//! arsenal's twenty-five item lists, the hub stash, the shop's stock, the
//! company record, the day record, the staff, the merchant, the hireables
//! and the tournament. Exarust holds the arsenal ([`Arsenal`]), the staff
//! ([`Staff`]), the company ([`Company`]) and the day's log ([`ArenaLog`]) as
//! live models; this module turns them into [`ArenaTables`] when a save is
//! written and puts them back when one is loaded ([`from_live`],
//! [`apply`], and the [`Session`] helpers [`Session::capture_arena`] and
//! [`Session::apply_arena_tables`]).
//!
//! What Exarust has no live model for keeps the chunk a save already holds,
//! or the reader's default when there is none: the hireables, the
//! tournament's field, prizes, events and history, the day's specials and
//! daily tallies, and the company's second name string and three words. The
//! merchant's stock type and specialisation are the career's
//! (`ArenaCareer::merchant_kinds` and `merchant_rank`, 0 and −1 in a new
//! career, as the original's writer stores its two words), and its two
//! unused words are zero.
//!
//! Evidence: the arena's save writers and readers for the company, day,
//! employees and merchant chunks, and the tournament chunk's six
//! forty-eight-byte roster rows (`docs/formats/SAVES.md`, Chunks).

use exa_formats::arena_save::{
    self, ArenaTables, CompanyChunk, DayChunk, EmployeeRecord, EmployeesChunk, ItemEntry,
    MerchantChunk, TournamentChunk,
};

use crate::arena::{Roster, RosterRow};
use crate::arena_log::ArenaLog;
use crate::arsenal::{Arsenal, ArsenalEntry};
use crate::hub_windows::ArenaCareer;
use crate::staff::Staff;

/// The company record's sixteen words, in the writer's order.
pub const RECORD_WON: usize = 0;
pub const RECORD_LOST: usize = 1;
pub const RECORD_FORFEITED: usize = 2;
pub const RECORD_TOURNAMENTS_WON: usize = 3;
pub const RECORD_TOURNAMENTS_LOST: usize = 4;
pub const RECORD_SEASONS_WON: usize = 5;
pub const RECORD_BEASTS_FOUGHT: usize = 6;
pub const RECORD_BEASTS_DEFEATED: usize = 7;
pub const RECORD_HIRES: usize = 8;
pub const RECORD_DEATHS: usize = 9;
pub const RECORD_FIRED: usize = 10;
pub const RECORD_EARNED: usize = 11;
pub const RECORD_SPENT: usize = 12;

/// The bytes of one roster row: a count, two entrants (an actor id and two
/// weapon handles) and the team id, forty-eight bytes as the original's
/// tournament chunk holds six of them.
pub const ROSTER_ROW_LEN: usize = 0x30;

/// One arsenal entry as the twelve-byte wire entry: the item, its flags and
/// the entry's two words packed into the third (a guess: the original stores
/// the two words of its item reference as the low and high half).
fn wire_entry(entry: &ArsenalEntry) -> ItemEntry {
    ItemEntry {
        id: entry.item,
        flags: entry.flags,
        extra: u32::from(entry.first) | (u32::from(entry.second) << 16),
    }
}

/// The arsenal entry an item entry describes: the two words split again.
fn arsenal_entry(entry: &ItemEntry) -> ArsenalEntry {
    ArsenalEntry {
        item: entry.id,
        flags: entry.flags,
        first: entry.extra as u16,
        second: (entry.extra >> 16) as u16,
    }
}

/// An item id as a plain twelve-byte list entry (no flags, no extra word).
fn plain_item(id: u32) -> ItemEntry {
    ItemEntry {
        id,
        flags: 0,
        extra: 0,
    }
}

/// The emblem's seven words: its layers, background, design and four colour
/// dwords, in the order the emblem record holds them.
fn emblem_bytes(emblem: &exa_ui::EmblemRecord) -> [u8; arena_save::EMBLEM_LEN] {
    let mut out = [0u8; arena_save::EMBLEM_LEN];
    let words = [
        emblem.layers,
        emblem.background,
        emblem.design,
        emblem.colours[0],
        emblem.colours[1],
        emblem.colours[2],
        emblem.colours[3],
    ];
    for (chunk, word) in out.chunks_mut(4).zip(words) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    out
}

/// The emblem record seven words make.
fn emblem_from_bytes(bytes: &[u8; arena_save::EMBLEM_LEN]) -> exa_ui::EmblemRecord {
    let word = |index: usize| {
        let at = index * 4;
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    };
    exa_ui::EmblemRecord {
        layers: word(0),
        background: word(1),
        design: word(2),
        colours: [word(3), word(4), word(5), word(6)],
    }
}

/// One roster row's forty-eight bytes.
pub fn roster_row_bytes(row: &RosterRow) -> [u8; ROSTER_ROW_LEN] {
    let mut out = [0u8; ROSTER_ROW_LEN];
    out[0..4].copy_from_slice(&row.count.to_le_bytes());
    let mut at = 4;
    for entrant in &row.entries {
        out[at..at + 4].copy_from_slice(&entrant.actor_id.to_le_bytes());
        out[at + 4..at + 12].copy_from_slice(&entrant.weapon[0].to_le_bytes());
        out[at + 12..at + 20].copy_from_slice(&entrant.weapon[1].to_le_bytes());
        at += 20;
    }
    out[44..48].copy_from_slice(&row.team_id.to_le_bytes());
    out
}

/// A roster row read from its forty-eight bytes.
pub fn roster_row_from_bytes(bytes: &[u8]) -> Option<RosterRow> {
    if bytes.len() < ROSTER_ROW_LEN {
        return None;
    }
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let qword = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
    let mut entries = [crate::arena::Entrant::default(); 2];
    let mut at = 4;
    for entrant in &mut entries {
        *entrant = crate::arena::Entrant {
            actor_id: word(at),
            weapon: [qword(at + 4), qword(at + 12)],
        };
        at += 20;
    }
    Some(RosterRow {
        count: word(0),
        entries,
        team_id: word(44),
    })
}

/// The tournament chunk's six rows as the live roster.
pub fn roster_from_rows(bytes: &[u8; arena_save::EVENT_ROWS_LEN]) -> Roster {
    let mut rows = [RosterRow::default(); 6];
    for (index, row) in rows.iter_mut().enumerate() {
        let at = index * ROSTER_ROW_LEN;
        if let Some(read) = roster_row_from_bytes(&bytes[at..at + ROSTER_ROW_LEN]) {
            *row = read;
        }
    }
    rows
}

/// The live roster as the tournament chunk's rows.
pub fn rows_from_roster(roster: &Roster) -> [u8; arena_save::EVENT_ROWS_LEN] {
    let mut out = [0u8; arena_save::EVENT_ROWS_LEN];
    for (index, row) in roster.iter().enumerate() {
        let at = index * ROSTER_ROW_LEN;
        out[at..at + ROSTER_ROW_LEN].copy_from_slice(&roster_row_bytes(row));
    }
    out
}

/// The item list an employee's own items make: a fighter holds its kit, a
/// specialist its carried items, hands and worn items (the original releases
/// exactly these to the arsenal when the employee leaves).
fn employee_items(employee: &crate::staff::Employee) -> Vec<ItemEntry> {
    let mut ids: Vec<u32> = Vec::new();
    if employee.fights() || employee.class < 3 {
        ids.extend(&employee.kit);
    } else {
        ids.extend(&employee.carried);
        ids.extend(employee.hands);
        ids.extend(employee.worn.iter().map(|(item, _)| *item));
    }
    // An empty hand is no item.
    ids.retain(|&id| id != 0);
    ids.into_iter().map(plain_item).collect()
}

/// The live arena state as the save's arena tables. `stash` is the hub stash's
/// item ids (Exarust has no stash model yet; the host supplies the list), and
/// `previous` is the save's own tables, which keep every chunk Exarust cannot
/// fill from live state.
pub fn from_live(
    career: &ArenaCareer,
    staff: &Staff,
    arsenal: &Arsenal,
    stash: &[u32],
    previous: Option<&ArenaTables>,
) -> ArenaTables {
    let previous = previous.cloned().unwrap_or_default();
    let mut tables = ArenaTables::default();

    for (shelf, live) in tables.arsenal.lists.iter_mut().zip(&arsenal.shelves) {
        for (list, box_entries) in shelf.iter_mut().zip(live) {
            list.extend(box_entries.iter().map(wire_entry));
        }
    }
    tables.stash = stash.iter().copied().map(plain_item).collect();
    // The shop's shelf is the saved shop stock.
    tables.shop_stock = career.company.stock.clone();

    tables.company = CompanyChunk {
        name: career.company.name.as_bytes().to_vec(),
        second: previous.company.second,
        emblem: emblem_bytes(&career.company.emblem),
        words: previous.company.words,
        records: {
            let mut records = previous.company.records;
            records[RECORD_WON] = career.company.won;
            records[RECORD_LOST] = career.company.lost;
            records[RECORD_TOURNAMENTS_WON] = career.company.tournaments_won;
            records[RECORD_SEASONS_WON] = career.company.seasons_won;
            records[RECORD_DEATHS] = career.company.deaths;
            records[RECORD_EARNED] = career.company.earned;
            records[RECORD_SPENT] = career.company.spent;
            records
        },
    };

    tables.day = DayChunk {
        matches_today: career.matches_today,
        spent_today: previous.day.spent_today,
        earned_today: previous.day.earned_today,
        specials: previous.day.specials,
        log_word: previous.day.log_word,
        log: career
            .log
            .records
            .iter()
            .map(|record| {
                let mut bytes = [0u8; arena_save::LOG_RECORD_LEN];
                bytes[0..4].copy_from_slice(&record.id.to_le_bytes());
                for (index, word) in record.words.iter().enumerate() {
                    let at = 4 + index * 4;
                    bytes[at..at + 4].copy_from_slice(&word.to_le_bytes());
                }
                bytes
            })
            .collect(),
    };

    tables.employees = EmployeesChunk {
        employees: staff
            .employees
            .iter()
            .map(|employee| EmployeeRecord {
                class: u32::from(employee.class),
                custom: employee.world.is_none(),
                character: employee.world.unwrap_or(0),
                items: employee_items(employee),
                words: [0; 4],
            })
            .collect(),
    };

    tables.merchant = MerchantChunk {
        stock_type: career.merchant_kinds,
        specialisation: career.merchant_rank as u32,
        unused: [0; 2],
        ids: career.merchant_stock.clone(),
    };

    tables.hireables = previous.hireables;
    // The week's field is the career's once drawn, else the loaded one.
    let field = if career.rivals[0] != 0 {
        career.rivals.map(|company| company as u32)
    } else {
        previous.tournament.field
    };
    tables.tournament = TournamentChunk {
        rows: rows_from_roster(&career.entries),
        field,
        ..previous.tournament
    };
    tables
}

/// Put a loaded save's arena tables into the live state: the arsenal, the
/// shop's shelf, the company's counters and emblem, the day's log, the staff
/// and the tournament's roster. The stash is the caller's list (see
/// [`from_live`]). A specialist's flat item list comes back as its carried
/// items; the hand and worn split is not stored on the wire.
pub fn apply(
    career: &mut ArenaCareer,
    staff: &mut Staff,
    arsenal: &mut Arsenal,
    stash: &mut Vec<u32>,
    tables: &ArenaTables,
) {
    for (shelf, lists) in arsenal.shelves.iter_mut().zip(&tables.arsenal.lists) {
        for (box_entries, list) in shelf.iter_mut().zip(lists) {
            box_entries.clear();
            box_entries.extend(list.iter().map(arsenal_entry));
        }
    }
    *stash = tables.stash.iter().map(|entry| entry.id).collect();

    career.company.name = String::from_utf8_lossy(&tables.company.name).into_owned();
    career.company.stock = tables.shop_stock.clone();
    career.company.emblem = emblem_from_bytes(&tables.company.emblem);
    let records = &tables.company.records;
    career.company.won = records[RECORD_WON];
    career.company.lost = records[RECORD_LOST];
    career.company.tournaments_won = records[RECORD_TOURNAMENTS_WON];
    career.company.seasons_won = records[RECORD_SEASONS_WON];
    career.company.deaths = records[RECORD_DEATHS];
    career.company.earned = records[RECORD_EARNED];
    career.company.spent = records[RECORD_SPENT];

    career.matches_today = tables.day.matches_today;
    career.log = ArenaLog {
        records: tables
            .day
            .log
            .iter()
            .map(|bytes| crate::arena_log::ArenaRecord {
                id: i32::from_le_bytes(bytes[0..4].try_into().unwrap()),
                words: std::array::from_fn(|index| {
                    let at = 4 + index * 4;
                    i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
                }),
            })
            .collect(),
    };

    staff.employees = tables
        .employees
        .employees
        .iter()
        .map(|record| {
            let class = (record.class & 0xff) as u8;
            let items: Vec<u32> = record.items.iter().map(|entry| entry.id).collect();
            crate::staff::Employee {
                class,
                world: (!record.custom).then_some(record.character),
                kit: if class < 3 { items.clone() } else { Vec::new() },
                carried: if class < 3 { Vec::new() } else { items },
                ..crate::staff::Employee::default()
            }
        })
        .collect();

    career.merchant_stock = tables.merchant.ids.clone();
    career.merchant_kinds = tables.merchant.stock_type;
    career.merchant_rank = tables.merchant.specialisation as i32;
    career.entries = roster_from_rows(&tables.tournament.rows);
    // A field whose first company is 0 stays empty, so the next tournament
    // asks for a new one.
    career.rivals = tables.tournament.field.map(|company| company as i32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arsenal::{CLOTHING, WEAPONS};
    use crate::staff::{COMBATANT, MERCHANT, RECRUIT};
    use exa_ui::emblem_form::EmblemRecord;

    fn career(name: &str) -> ArenaCareer {
        ArenaCareer {
            company: crate::career::Company::new(name),
            ..ArenaCareer::default()
        }
    }

    #[test]
    fn the_tables_take_the_live_company_staff_arsenal_and_tournament() {
        let mut career = career("Rivals");
        career.company.copper = 900;
        career.company.won = 3;
        career.company.lost = 2;
        career.company.spent = 40;
        career.company.earned = 55;
        career.company.seasons_won = 1;
        career.company.tournaments_won = 2;
        career.company.deaths = 1;
        career.company.stock = vec![70, 71];
        career.company.emblem = EmblemRecord {
            layers: 0x10,
            background: 49,
            design: 79,
            colours: [1, 2, 3, 4],
        };
        career.matches_today = 2;
        career.merchant_stock = vec![80, 81];
        career.log.append_value(5, 6);
        career.entries[0].count = 2;
        career.entries[0].team_id = 1;
        career.entries[0].entries[0].actor_id = 9;
        career.entries[0].entries[1].weapon = [0x1234_5678, 0x9abc_def0];

        let mut arsenal = Arsenal::default();
        assert!(arsenal.stow(
            100,
            crate::arsenal::ItemKind {
                known: true,
                weapon: true,
                ..Default::default()
            },
            0x600
        ));
        let mut staff = Staff::default();
        staff.employees.push(crate::staff::Employee {
            class: COMBATANT,
            name: *b"Fighter\0\0\0\0\0\0\0\0\0",
            kit: vec![100],
            world: Some(0x1234),
            ..Default::default()
        });
        staff.employees.push(crate::staff::Employee {
            class: MERCHANT,
            name: *b"Merchant\0\0\0\0\0\0\0\0",
            carried: vec![200, 201],
            world: None,
            ..Default::default()
        });

        let tables = from_live(&career, &staff, &arsenal, &[300, 301], None);
        // The arsenal keeps the wire's packing of the entry's two words.
        let entry = tables.arsenal.lists[WEAPONS][0][0];
        assert_eq!((entry.id, entry.flags), (100, 0x600));
        assert_eq!(entry.extra, 0x28 | (4 << 16));
        assert_eq!(tables.stash.len(), 2);
        assert_eq!(tables.stash[1].id, 301);
        assert_eq!(tables.shop_stock, vec![70, 71]);
        assert_eq!(tables.company.name, b"Rivals");
        assert_eq!(tables.company.records[RECORD_WON], 3);
        assert_eq!(tables.company.records[RECORD_LOST], 2);
        assert_eq!(tables.company.records[RECORD_EARNED], 55);
        assert_eq!(tables.company.records[RECORD_SPENT], 40);
        assert_eq!(tables.company.records[RECORD_SEASONS_WON], 1);
        assert_eq!(tables.company.records[RECORD_TOURNAMENTS_WON], 2);
        assert_eq!(tables.company.records[RECORD_DEATHS], 1);
        assert_eq!(
            emblem_from_bytes(&tables.company.emblem),
            career.company.emblem
        );
        assert_eq!(tables.day.matches_today, 2);
        assert_eq!(tables.day.log.len(), 1);
        assert_eq!(tables.day.log[0][0..4], 5i32.to_le_bytes());
        assert_eq!(tables.day.log[0][4..8], 6i32.to_le_bytes());
        // A fighter's kit and a specialist's own items make the employees'
        // flat lists; the hired one carries its world id, the other is a
        // custom record.
        assert_eq!(tables.employees.employees.len(), 2);
        assert_eq!(tables.employees.employees[0].class, u32::from(COMBATANT));
        assert!(!tables.employees.employees[0].custom);
        assert_eq!(tables.employees.employees[0].character, 0x1234);
        assert_eq!(tables.employees.employees[0].items[0].id, 100);
        assert!(tables.employees.employees[1].custom);
        assert_eq!(tables.employees.employees[1].character, 0);
        assert_eq!(
            tables.employees.employees[1]
                .items
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![200, 201]
        );
        assert_eq!(tables.merchant.ids, vec![80, 81]);
        assert_eq!(tables.merchant.specialisation, u32::MAX);
        // No merchant class matches the writer's rule, but one is on the
        // staff, so the chunk is written.
        assert!(arena_save::merchant_chunk_written(&tables.employees));
        // The tournament's first row is the live roster's.
        let row = roster_from_rows(&tables.tournament.rows)[0];
        assert_eq!(row.team_id, 1);
        assert_eq!(row.entries[0].actor_id, 9);
        assert_eq!(row.entries[1].weapon, [0x1234_5678, 0x9abc_def0]);
        // A chunk Exarust has no live model for keeps what the save held.
        let mut previous = tables.clone();
        previous.hireables = vec![1, 2, 3, 4];
        previous.company.second = b"second".to_vec();
        previous.day.specials = vec![arena_save::SpecialEntry {
            template: 8,
            rank: 3,
            flags: 1,
        }];
        let kept = from_live(&career, &staff, &arsenal, &[], Some(&previous));
        assert_eq!(kept.hireables, vec![1, 2, 3, 4]);
        assert_eq!(kept.company.second, b"second");
        assert_eq!(kept.day.specials[0].template, 8);
        assert!(kept.stash.is_empty());
    }

    #[test]
    fn the_weeks_field_round_trips_through_the_tournament_chunk() {
        let mut career = career("Rivals");
        let staff = Staff::default();
        let arsenal = Arsenal::default();
        let previous = ArenaTables::default();
        // An undrawn field keeps the loaded one.
        let tables = from_live(&career, &staff, &arsenal, &[], Some(&previous));
        assert_eq!(tables.tournament.field, previous.tournament.field);
        career.rivals = [15, 17, 4, 1, 13, 9, 5, 14, 8, 6, 11, 2, 18, 3, 19, 10];
        let tables = from_live(&career, &staff, &arsenal, &[], Some(&previous));
        assert_eq!(tables.tournament.field[0], 15);
        assert_eq!(tables.tournament.field[15], 10);
        let mut back = ArenaCareer::default();
        apply(
            &mut back,
            &mut Staff::default(),
            &mut Arsenal::default(),
            &mut Vec::new(),
            &tables,
        );
        assert_eq!(back.rivals, career.rivals);
    }

    #[test]
    fn applying_the_tables_puts_the_live_state_back() {
        let source = career("Source");
        let mut arsenal = Arsenal::default();
        assert!(arsenal.stow(
            100,
            crate::arsenal::ItemKind {
                known: true,
                weapon: true,
                ..Default::default()
            },
            0x600
        ));
        assert!(arsenal.stow(
            101,
            crate::arsenal::ItemKind {
                known: true,
                apparel_layer: Some(0),
                ..Default::default()
            },
            0x400
        ));
        let mut staff = Staff::default();
        staff.employees.push(crate::staff::Employee {
            class: RECRUIT,
            name: *b"Recruit\0\0\0\0\0\0\0\0\0",
            kit: vec![100],
            ..Default::default()
        });
        let mut entries = [RosterRow::default(); 6];
        entries[2].count = 1;
        entries[2].team_id = 4;
        let mut career = source.clone();
        career.entries = entries;
        career.company.emblem = EmblemRecord {
            layers: 7,
            background: 52,
            design: 81,
            colours: [9, 8, 7, 6],
        };
        career.company.stock = vec![5];
        career.merchant_stock = vec![6];
        // A weapons merchant of rank 3 (option 5) keeps both words.
        career.merchant_kinds = 0xc000;
        career.merchant_rank = 3;
        career.log.append_words(1, [2, 3, 4, 5]);
        career.matches_today = 4;
        let tables = from_live(&career, &staff, &arsenal, &[300], None);

        let mut target_career = ArenaCareer::default();
        let mut target_staff = Staff::default();
        let mut target_arsenal = Arsenal::default();
        let mut target_stash = Vec::new();
        apply(
            &mut target_career,
            &mut target_staff,
            &mut target_arsenal,
            &mut target_stash,
            &tables,
        );
        assert_eq!(target_arsenal, arsenal);
        assert_eq!(target_stash, vec![300]);
        assert_eq!(target_career.company.name, "Source");
        assert_eq!(target_career.company.stock, vec![5]);
        assert_eq!(target_career.company.emblem, career.company.emblem);
        assert_eq!(target_career.merchant_stock, vec![6]);
        assert_eq!(tables.merchant.stock_type, 0xc000);
        assert_eq!(tables.merchant.specialisation, 3);
        assert_eq!(target_career.merchant_kinds, 0xc000);
        assert_eq!(target_career.merchant_rank, 3);
        assert_eq!(target_career.matches_today, 4);
        assert_eq!(target_career.log, career.log);
        assert_eq!(target_career.entries, career.entries);
        assert_eq!(target_staff.employees.len(), 1);
        let employee = &target_staff.employees[0];
        assert_eq!(employee.class, RECRUIT);
        assert_eq!(employee.kit, vec![100]);
        assert!(employee.world.is_none());
    }

    #[test]
    fn the_variant_carries_the_saved_merchant_and_tournament_rules() {
        // A save whose staff holds a merchant writes the merchant chunk;
        // an empty staff writes it too, and one without a merchant does
        // not.
        let mut career = career("Rivals");
        career.merchant_stock = vec![1];
        let empty = from_live(&career, &Staff::default(), &Arsenal::default(), &[], None);
        assert!(arena_save::merchant_chunk_written(&empty.employees));
        let mut staff = Staff::default();
        staff.employees.push(crate::staff::Employee {
            class: COMBATANT,
            ..Default::default()
        });
        let no_merchant = from_live(&career, &staff, &Arsenal::default(), &[], None);
        assert!(!arena_save::merchant_chunk_written(&no_merchant.employees));
        staff.employees[0].class = MERCHANT;
        let merchant = from_live(&career, &staff, &Arsenal::default(), &[], None);
        assert!(arena_save::merchant_chunk_written(&merchant.employees));
        // The arsenal's empty clothing shelf is still written.
        assert!(no_merchant.arsenal.lists[CLOTHING][0].is_empty());
    }

    /// The installed game's item database (with `EXANIMA_DIR`): a live
    /// arsenal stowed from real item records goes through the arena writer
    /// and comes back on the same shelves with the same ids and flags.
    #[test]
    fn shipped_items_round_trip_through_the_arena_writer() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let bytes = game
            .pack(exa_assets::PackKind::Resource)
            .unwrap()
            .get("objdb.rdb")
            .unwrap();
        let database = exa_formats::rdb::Database::parse(bytes).unwrap();
        let items = exa_formats::objdb::items(&database);
        let kind_of = |item: &exa_formats::objdb::Item| crate::arsenal::ItemKind {
            known: true,
            apparel_layer: item
                .slot_layers
                .iter()
                .copied()
                .map(|layer| layer & 0x3f)
                .find(|layer| *layer != 0),
            weapon: item.weapon.is_some(),
            shield: item.shield.is_some(),
        };
        let mut arsenal = Arsenal::default();
        let mut stowed = Vec::new();
        for item in &items {
            if stowed.len() >= 60 {
                break;
            }
            if crate::arsenal::shelf_of(kind_of(item)).is_some()
                && arsenal.stow(item.id, kind_of(item), 0x600)
            {
                stowed.push(item.id);
            }
        }
        assert!(stowed.len() >= 40, "{} stowed", stowed.len());
        let career = ArenaCareer::default();
        let tables = from_live(&career, &Staff::default(), &arsenal, &stowed, None);
        // The writer's own chunks and the reader's decoders agree.
        let mut encoded = Vec::new();
        tables.write(&mut encoded);
        let chunks = exa_formats::save_package::parse_chunks(&encoded).unwrap();
        let back = arena_save::read_tables(&chunks).unwrap();
        assert_eq!(back.arsenal.lists, tables.arsenal.lists);
        assert_eq!(back.stash, tables.stash);
        // Every stowed id is there once, with its flags.
        let ids: Vec<u32> = back
            .arsenal
            .lists
            .iter()
            .flatten()
            .flatten()
            .map(|entry| entry.id)
            .collect();
        for id in &stowed {
            assert_eq!(ids.iter().filter(|seen| *seen == id).count(), 1, "{id}");
        }
        assert!(
            back.arsenal
                .lists
                .iter()
                .flatten()
                .flatten()
                .all(|entry| entry.flags == 0x600)
        );
        // The stash's plain entries keep their ids too.
        assert_eq!(
            back.stash.iter().map(|entry| entry.id).collect::<Vec<_>>(),
            stowed
        );
    }
}
