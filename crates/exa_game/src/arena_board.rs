//! The arena career's view of the match board.
//!
//! The board itself lives in [`exa_ui::match_board`]; this module turns a
//! company, the player and a generated daily board into its rows. It is the
//! only place that knows both the career's records and the board's shape.
//!
//! A daily match instance is the board's event record field for field: the
//! board's rank column reads the instance's day slot, its tier column the
//! rolled rank, its entry column the slot-scaled secondary value and its
//! prize column the cash. A member's board class is the combatant zero and
//! the staff professions above the combatant threshold, a guess from the
//! filter's "class below three" rule.

use exa_ui::match_board::{self, Board, Fighter, MatchRecord, WeaponRow};

use crate::arena::{self, MatchInstance, MatchTemplate, WeaponEntry};
use crate::career::{Company, Member, Profession};

/// The company record's class byte for a profession. Combatants are zero;
/// the three staff roles are above the board's combatant limit.
pub fn class_of(profession: Profession) -> u8 {
    match profession {
        Profession::Combatant => 0,
        Profession::Merchant => 3,
        Profession::Physician => 4,
        Profession::Trainer => 5,
    }
}

/// A career member as a board fighter. The rank is the six skill levels
/// halved and capped at Master, the same rule the rank helper uses.
pub fn fighter(member: &Member) -> Fighter {
    let mut fighter = Fighter::new(
        member.name.clone(),
        match_board::rank_of(&member.skills),
        class_of(member.profession),
    );
    fighter.health = member.health;
    fighter
}

/// A daily match instance as the board's event record.
pub fn event_record(instance: &MatchInstance) -> MatchRecord {
    MatchRecord {
        name: arena::template(instance.id)
            .map(|template| template.name.to_string())
            .unwrap_or_default(),
        kind: instance.id,
        flags: instance.flags,
        min_rank: instance.min_rank,
        max_rank: instance.max_rank,
        rank: instance.slot as i32,
        tier: instance.rank,
        fee: instance.secondary,
        prize: instance.cash,
        team_size: instance.teams as i32,
        weapon_row: instance.weapon_row as i32,
    }
}

/// The six daily events from the generated board. An unknown template id
/// leaves the event's name empty.
pub fn daily_events(rows: &[MatchInstance; 6]) -> [MatchRecord; 6] {
    std::array::from_fn(|index| event_record(&rows[index]))
}

/// A weapon row as the board reads it, its variants zero-padded to three.
pub fn weapon_row(entry: &WeaponEntry) -> WeaponRow {
    let mut variants = [0; 3];
    for (slot, &variant) in variants.iter_mut().zip(entry.variants) {
        *slot = variant;
    }
    WeaponRow {
        name: entry.name.to_string(),
        variants,
    }
}

/// The fighters per side a match template takes, at least one.
pub fn team_size(template: &MatchTemplate) -> usize {
    (template.teams as usize).max(1)
}

/// Opens the board on a career. The player is pool slot zero and the
/// company's roster follows in hire order.
pub fn open(company: &Company, player: &Member, day: u32, rows: &[MatchInstance; 6]) -> Board {
    let mut fighters = Vec::with_capacity(company.roster.len() + 1);
    fighters.push(fighter(player));
    fighters.extend(company.roster.iter().map(fighter));
    let mut board = Board::new(false, day, fighters);
    board.company_name = company.name.clone();
    board.rivals = arena::COMPANIES
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect();
    board.season_rewards = arena::SEASON_PRIZES;
    board.weapon_tiers = arena::WEAPON_TIERS
        .iter()
        .map(|tier| tier.iter().map(weapon_row).collect())
        .collect();
    board.events = daily_events(rows);
    board.refresh_standard();
    board
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{self, STANDARD};
    use crate::career::Member;

    fn member(name: &str, profession: Profession, skills: [u32; 6]) -> Member {
        Member {
            name: name.to_string(),
            profession,
            xp: 0,
            skills,
            skill_xp: [0; 6],
            techniques: [[0; 6]; 6],
            loadouts: Default::default(),
            health: 65_000,
            in_entry: false,
        }
    }

    fn instance(id: u32, rank: i32, fee: u32, prize: u32) -> MatchInstance {
        let template = arena::template(id).unwrap();
        let mut instance = MatchInstance::from_template(template, 4);
        instance.rank = rank;
        instance.secondary = fee;
        instance.cash = prize;
        instance
    }

    #[test]
    fn a_combatant_ranks_and_the_staff_do_not() {
        let combatant = member("Anna", Profession::Combatant, [10, 0, 0, 0, 0, 0]);
        let merchant = member("Sam", Profession::Merchant, [10, 0, 0, 0, 0, 0]);
        assert_eq!(fighter(&combatant).rank, 5);
        assert_eq!(fighter(&combatant).class, 0);
        assert!(fighter(&combatant).is_combatant());
        assert!(!fighter(&merchant).is_combatant());
        assert_eq!(
            fighter(&member("New", Profession::Combatant, [1, 0, 0, 0, 0, 0])).rank,
            0
        );
    }

    #[test]
    fn the_daily_events_take_names_and_coins() {
        let mut rows = [instance(1, 0, 0, 0); 6];
        rows[0] = instance(1, 0, 3, 12);
        rows[1] = instance(8, 4, 12, 768);
        let events = daily_events(&rows);
        assert_eq!(events[0].name, "Duel");
        assert_eq!(events[0].fee, 3);
        assert_eq!(events[0].prize, 12);
        assert_eq!(events[1].name, "Challenger");
        assert_eq!(events[1].kind, 8);
        // The rank column is the day slot, the tier column the rolled rank.
        assert_eq!(events[1].rank, 4);
        assert_eq!(events[1].tier, 4);
        assert_eq!(events[1].team_size, 1);
        assert_eq!(events[1].fee, 12);
        assert_eq!(events[1].prize, 768);
        assert_eq!(events[5].name, "Duel");
    }

    #[test]
    fn the_board_takes_the_company_and_the_player() {
        let player = member("Hero", Profession::Combatant, [6, 0, 0, 0, 0, 0]);
        let mut company = Company::new("Ours");
        company
            .roster
            .push(member("Anna", Profession::Combatant, [4, 0, 0, 0, 0, 0]));
        company
            .roster
            .push(member("Sam", Profession::Merchant, [2, 0, 0, 0, 0, 0]));
        let rows = [instance(1, 0, 3, 12); 6];
        let board = open(&company, &player, 3, &rows);
        assert_eq!(board.fighters.len(), 3);
        assert_eq!(board.fighters[0].name, "Hero");
        assert_eq!(board.fighters[1].name, "Anna");
        assert_eq!(board.fighters[2].name, "Sam");
        assert_eq!(board.company_name, "Ours");
        assert_eq!(board.rivals[0], "Bonten Company");
        assert_eq!(board.season_rewards, arena::SEASON_PRIZES);
        assert_eq!(board.events[0].name, "Duel");
        assert_eq!(board.team_size(), 1);
        assert_eq!(board.team, vec![1]);
        assert_eq!(board.weapon_tiers.len(), 6);
        assert_eq!(board.weapon_tiers[0].len(), arena::WEAPON_TIERS[0].len());
    }

    #[test]
    fn a_weapon_row_pads_its_variants() {
        let row = weapon_row(&arena::WEAPON_TIERS[0][0]);
        assert_eq!(row.name, arena::WEAPON_TIERS[0][0].name);
        let count = arena::WEAPON_TIERS[0][0].variants.len().min(3);
        assert_eq!(
            &row.variants[..count],
            &arena::WEAPON_TIERS[0][0].variants[..count]
        );
        assert!(row.variants[count..].iter().all(|&variant| variant == 0));
    }

    #[test]
    fn the_side_size_comes_from_the_template() {
        assert_eq!(team_size(&STANDARD[0]), 1);
        assert_eq!(team_size(&STANDARD[2]), 3);
    }
}
