//! The manage window's daily report: the day's title and the report lines.
//!
//! Before the first week the title is "Day" and the day from 1; after it,
//! "Week" and the week from 1 then "Day" and the day of the week from 1.
//! The lines are the company log's records, newest first (a won, lost or
//! forfeited match, a death, a save, a dismissal, a joining, a leaving),
//! then each fighter who is ready to train, newest hire first and the
//! manager last, with the rank reached at every second level, then the
//! merchants, trainers and physicians on the hire list, then the days to
//! the tournament. A match is named in lower case after "a", or "an" when
//! it starts with a vowel.
//!
//! Evidence: the retail manage window's refresh and report routines read
//! from `Exanima.exe` (`context/re/arena-staff.md`).

use crate::arena::{RANKED, STANDARD};
use crate::arena_log::ArenaRecord;
use crate::arena_match::{BEAST, LOG_DIED, LOG_FORFEIT, LOG_LOST, LOG_SAVED, LOG_WON};
use crate::career::Profession;
use crate::staff::{LOG_DISMISSED, LOG_JOINED};

/// The log's record of an employee who left on his own.
pub const LOG_LEFT: i32 = 0xA0000;
/// Report line codes the window gives its own lines.
pub const LINE_TOURNAMENT: u32 = 0x10000;
pub const LINE_MERCHANTS: u32 = 0x110000;
pub const LINE_PHYSICIANS: u32 = 0x120000;
pub const LINE_TRAINERS: u32 = 0x130000;
pub const LINE_TRAINED: u32 = 0x140000;
pub const LINE_RANK: u32 = 0x150000;
/// A fighter whose six levels sum to this is no longer ready to train.
pub const TRAINING_CAP: u32 = 15;
/// The rank names, one for every two levels.
pub const RANKS: [&str; 7] = [
    "Inept", "Aspirant", "Novice", "Adept", "Expert", "Master", "-",
];

/// The day's title.
pub fn day_title(day: u32) -> String {
    if day < 7 {
        format!("Day {}", day + 1)
    } else {
        format!("Week {} Day {}", day / 7, day % 7 + 1)
    }
}

/// A name's bytes up to the first NUL, at most sixteen.
pub fn name_text(name: &[u8]) -> String {
    let bytes = &name[..name.len().min(16)];
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    bytes[..end].iter().map(|&b| char::from(b)).collect()
}

/// A match's name with its article: the standard match of that id, else
/// "beast" for the Beast, else the special match of that id, in lower
/// case after "an " when it starts with a vowel and "a " otherwise. An id
/// no table holds is named by nothing.
pub fn match_phrase(template: i32) -> String {
    let name = STANDARD
        .iter()
        .find(|t| t.id as i32 == template)
        .map(|t| t.name)
        .or_else(|| (template == BEAST).then_some("beast"))
        .or_else(|| {
            RANKED
                .iter()
                .find(|t| t.id as i32 == template)
                .map(|t| t.name)
        })
        .unwrap_or("");
    let lower = name.to_ascii_lowercase();
    let article = match lower.bytes().next() {
        Some(b'a' | b'e' | b'i' | b'o' | b'u') => "an ",
        _ => "a ",
    };
    format!("{article}{lower}")
}

/// One line of the report and the code the window keeps with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportLine {
    pub code: u32,
    pub text: String,
}

fn words_name(words: &[i32; 4]) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    for (index, word) in words.iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    bytes
}

/// A log record's line, if the report shows it. `name_of` gives the name
/// of the fighter a save names by id (the manager for 0).
pub fn log_line(record: &ArenaRecord, name_of: impl Fn(u8) -> Option<[u8; 16]>) -> Option<String> {
    let id = record.id;
    let name = || name_text(&words_name(&record.words));
    let text = match id {
        LOG_WON => format!("You have won {} match.", match_phrase(record.words[0])),
        LOG_LOST => format!("You have lost {} match.", match_phrase(record.words[0])),
        LOG_FORFEIT => format!("You forfeited {} match.", match_phrase(record.words[0])),
        LOG_DIED => format!("{} has died.", name()),
        LOG_SAVED => {
            let saved = name_of(record.words[0] as u8).unwrap_or([0; 16]);
            format!("{} was saved from lethal injury.", name_text(&saved))
        }
        LOG_DISMISSED => format!("{} has been dismissed.", name()),
        LOG_JOINED => format!("{} has joined the company.", name()),
        LOG_LEFT => format!("{} has left the company.", name()),
        _ => return None,
    };
    Some(text)
}

/// A fighter as the report reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportFighter {
    pub name: [u8; 16],
    /// The id word's low byte (0 for the manager).
    pub id: u8,
    pub skill_xp: [u16; 6],
    pub techniques: [[u8; 6]; 6],
}

/// The six levels' sum.
pub fn level_sum(skill_xp: &[u16; 6]) -> u32 {
    skill_xp.iter().map(|&xp| u32::from(xp / 5000)).sum()
}

/// Ready to train a new technique: no skill below level 5 has the
/// technique at its level set, and the levels sum to less than 15.
pub fn ready_to_train(skill_xp: &[u16; 6], techniques: &[[u8; 6]; 6]) -> bool {
    for (xp, slots) in skill_xp.iter().zip(techniques) {
        let level = usize::from(*xp / 5000);
        if level < 5 && slots[level] != 0 {
            return false;
        }
    }
    level_sum(skill_xp) < TRAINING_CAP
}

fn fighter_lines(fighter: &ReportFighter, lines: &mut Vec<ReportLine>) {
    if !ready_to_train(&fighter.skill_xp, &fighter.techniques) {
        return;
    }
    let name = name_text(&fighter.name);
    let id = u32::from(fighter.id) << 24;
    lines.push(ReportLine {
        code: id | LINE_TRAINED,
        text: format!("{name} has trained a new technique."),
    });
    let sum = level_sum(&fighter.skill_xp);
    if sum < 11 && sum & 1 == 0 {
        lines.push(ReportLine {
            code: id | LINE_RANK,
            text: format!("{name} has reached {} rank.", RANKS[sum as usize / 2]),
        });
    }
}

fn hire_line(count: usize, one: &str, many: &str, code: u32, lines: &mut Vec<ReportLine>) {
    if count == 0 {
        return;
    }
    let text = if count == 1 {
        one.to_string()
    } else {
        format!("{count}{many}")
    };
    lines.push(ReportLine { code, text });
}

/// The report's lines. `fighters` are the employees whose role is a
/// combatant's or below, in staff order; `manager` comes last; `listed`
/// are the hire list's professions.
pub fn report_lines(
    log: &[ArenaRecord],
    fighters: &[ReportFighter],
    manager: &ReportFighter,
    listed: &[Profession],
    day: u32,
    name_of: impl Fn(u8) -> Option<[u8; 16]>,
) -> Vec<ReportLine> {
    let mut lines = Vec::new();
    for record in log.iter().rev() {
        if (record.id as u32) < 0x20000 {
            continue;
        }
        if let Some(text) = log_line(record, &name_of) {
            lines.push(ReportLine {
                code: record.id as u32,
                text,
            });
        }
    }
    for fighter in fighters.iter().rev() {
        fighter_lines(fighter, &mut lines);
    }
    fighter_lines(manager, &mut lines);
    let count = |role| listed.iter().filter(|p| **p == role).count();
    hire_line(
        count(Profession::Merchant),
        "A merchant is available for hire.",
        " merchants are available for hire.",
        LINE_MERCHANTS,
        &mut lines,
    );
    hire_line(
        count(Profession::Trainer),
        "A trainer is available for hire.",
        " trainers are available for hire.",
        LINE_TRAINERS,
        &mut lines,
    );
    hire_line(
        count(Profession::Physician),
        "A physician is available for hire.",
        " physicians are available for hire.",
        LINE_PHYSICIANS,
        &mut lines,
    );
    let until = 7 - day % 7;
    let text = match until {
        7 => "The tournament takes place today.".to_string(),
        1 => "The tournament begins tomorrow.".to_string(),
        days => format!("The tournament begins in {days} days."),
    };
    lines.push(ReportLine {
        code: LINE_TOURNAMENT,
        text,
    });
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena_log::ArenaLog;
    use crate::staff::name_words;

    fn name(text: &str) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        bytes
    }

    #[test]
    fn the_day_title_counts_weeks_after_the_first() {
        assert_eq!(day_title(0), "Day 1");
        assert_eq!(day_title(6), "Day 7");
        assert_eq!(day_title(7), "Week 1 Day 1");
        assert_eq!(day_title(20), "Week 2 Day 7");
    }

    #[test]
    fn a_match_is_named_with_its_article() {
        assert_eq!(match_phrase(1), "a duel");
        assert_eq!(match_phrase(7), "an elimination");
        assert_eq!(match_phrase(11), "a beast");
        assert_eq!(match_phrase(13), "a valiance");
        assert_eq!(match_phrase(99), "a ");
    }

    #[test]
    fn log_records_read_as_lines() {
        let mut log = ArenaLog::default();
        log.append_value(LOG_WON, 7);
        log.append_words(LOG_JOINED, name_words(&name("Ash")));
        log.append_value(LOG_SAVED, 0);
        log.append_words(LOG_LEFT, name_words(&name("Sixteen chars ok")));
        log.append_value(0x10000, 0);
        let manager = |id| (id == 0).then(|| name("Boss"));
        let texts: Vec<String> = log
            .records
            .iter()
            .filter_map(|r| log_line(r, manager))
            .collect();
        assert_eq!(
            texts,
            vec![
                "You have won an elimination match.",
                "Ash has joined the company.",
                "Boss was saved from lethal injury.",
                "Sixteen chars ok has left the company.",
            ]
        );
    }

    #[test]
    fn a_fighter_is_ready_below_the_cap_with_nothing_pending() {
        let mut xp = [10_000, 0, 0, 0, 0, 0];
        let mut techniques = [[0u8; 6]; 6];
        assert!(ready_to_train(&xp, &techniques));
        techniques[0][2] = 4;
        assert!(!ready_to_train(&xp, &techniques));
        techniques[0][2] = 0;
        xp = [25_000, 25_000, 25_000, 0, 0, 0];
        assert!(!ready_to_train(&xp, &techniques));
        xp = [25_000, 25_000, 20_000, 0, 0, 0];
        assert!(ready_to_train(&xp, &techniques));
    }

    #[test]
    fn the_report_lists_log_fighters_hires_and_tournament() {
        let mut log = ArenaLog::default();
        log.append_value(LOG_LOST, 1);
        log.append_value(LOG_FORFEIT, 2);
        let fighter = ReportFighter {
            name: name("Ash"),
            id: 2,
            skill_xp: [10_000, 10_000, 0, 0, 0, 0],
            techniques: [[0; 6]; 6],
        };
        let manager = ReportFighter {
            name: name("Boss"),
            id: 0,
            skill_xp: [5000, 0, 0, 0, 0, 0],
            techniques: [[0; 6]; 6],
        };
        let listed = [
            Profession::Merchant,
            Profession::Trainer,
            Profession::Combatant,
            Profession::Trainer,
        ];
        let lines = report_lines(&log.records, &[fighter], &manager, &listed, 3, |_| None);
        let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(
            texts,
            vec![
                "You forfeited a doubles match.",
                "You have lost a duel match.",
                "Ash has trained a new technique.",
                "Ash has reached Novice rank.",
                "Boss has trained a new technique.",
                "A merchant is available for hire.",
                "2 trainers are available for hire.",
                "The tournament begins in 4 days.",
            ]
        );
        assert_eq!(lines[2].code, 0x0214_0000);
        assert_eq!(lines[3].code, 0x0215_0000);
        assert_eq!(lines[6].code, LINE_TRAINERS);
        let today = report_lines(&[], &[], &manager, &[], 7, |_| None);
        assert_eq!(
            today.last().unwrap().text,
            "The tournament takes place today."
        );
        let tomorrow = report_lines(&[], &[], &manager, &[], 6, |_| None);
        assert_eq!(
            tomorrow.last().unwrap().text,
            "The tournament begins tomorrow."
        );
    }
}
