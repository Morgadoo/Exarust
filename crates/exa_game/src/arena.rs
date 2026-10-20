//! The arena's match table and career formulas (`XA_Arena`). The templates
//! are the original's 6 standard, 7 ranked and 5 tournament records; cash is
//! the packed base-12 money the purse uses, and XP is paid from the match's
//! base times the slot and the survivors' health.

/// The flat hire fee.
pub const HIRE_FEE: u32 = 12;
/// The XP unit: one rank point per 5000 in each of the six pools.
pub const XP_PER_RANK: u32 = 5_000;
/// Weeks in a season.
pub const WEEKS_PER_SEASON: u32 = 4;
/// Fighting days in a week.
pub const FIGHTING_DAYS: u32 = 6;
/// A fighter below this condition is unfit.
pub const UNFIT_CONDITION: u16 = 0x0CCC;
/// Daily recovery, normally and with a physician.
pub const RECOVER: u16 = 0x1999;
pub const RECOVER_PHYSICIAN: u16 = 0x2666;
/// The point limits per loadout tier.
pub const POINT_LIMITS: [u32; 6] = [5, 10, 15, 20, 30, 40];

/// A match template record (0x70 bytes in the original).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchTemplate {
    /// The match type id.
    pub id: u32,
    pub name: &'static str,
    /// 1 = a lost round reduces the prize, 2 = rank is the day slot,
    /// 4 = a weapon is chosen for the event.
    pub flags: u32,
    pub min_rank: i32,
    pub max_rank: i32,
    /// The secondary value at `+0x28`; scaled by the slot but not the cash.
    pub secondary: u32,
    /// The raw packed base-12 cash prize at `+0x2c`.
    pub cash: u32,
    pub teams: u32,
    /// Tournament only: total fighters.
    pub fighters: u32,
    /// Tournament only: rounds.
    pub rounds: u32,
    /// The XP prize base at `+0x44`.
    pub xp: u32,
    /// The earliest week slot the template may take.
    pub min_slot: u32,
    /// The maximum instances in one generation pass (0 = unlimited).
    pub per_pass: u32,
    /// Reject when `random(8) & reject != 0`.
    pub reject: u32,
    pub description: &'static str,
}

/// The six always-available standard matches (`0x1002bc6d0`).
pub const STANDARD: [MatchTemplate; 6] = [
    MatchTemplate {
        id: 1,
        name: "Duel",
        flags: 0,
        min_rank: 0,
        max_rank: 0,
        secondary: 3,
        cash: 256,
        teams: 1,
        fighters: 0,
        rounds: 0,
        xp: 1000,
        min_slot: 0,
        per_pass: 0,
        reject: 0,
        description: "Single combat between two equally matched fighters.",
    },
    MatchTemplate {
        id: 2,
        name: "Doubles",
        flags: 0,
        min_rank: 0,
        max_rank: 0,
        secondary: 6,
        cash: 1024,
        teams: 2,
        fighters: 0,
        rounds: 0,
        xp: 2400,
        min_slot: 0,
        per_pass: 0,
        reject: 0,
        description: "A team of two faces an equal team on the opposing side.",
    },
    MatchTemplate {
        id: 3,
        name: "Skirmish",
        flags: 0,
        min_rank: 0,
        max_rank: 0,
        secondary: 9,
        cash: 1536,
        teams: 3,
        fighters: 0,
        rounds: 0,
        xp: 3900,
        min_slot: 0,
        per_pass: 0,
        reject: 0,
        description: "A team of three faces an equal team on the opposing side.",
    },
    MatchTemplate {
        id: 4,
        name: "Fray",
        flags: 0,
        min_rank: 0,
        max_rank: 0,
        secondary: 3,
        cash: 512,
        teams: 1,
        fighters: 0,
        rounds: 0,
        xp: 5000,
        min_slot: 0,
        per_pass: 0,
        reject: 0,
        description: "Six fighters engage in combat until only one remains standing. Winners are chosen based on their performance.",
    },
    MatchTemplate {
        id: 5,
        name: "Reserve",
        flags: 1,
        min_rank: 0,
        max_rank: 0,
        secondary: 3,
        cash: 1024,
        teams: 3,
        fighters: 0,
        rounds: 0,
        xp: 1000,
        min_slot: 0,
        per_pass: 0,
        reject: 0,
        description: "Three fighters per side duel until one team has no more reserves. Each round lost reduces the prize.",
    },
    MatchTemplate {
        id: 6,
        name: "Pugilism",
        flags: 0,
        min_rank: 0,
        max_rank: -1,
        secondary: 1,
        cash: 6,
        teams: 1,
        fighters: 0,
        rounds: 0,
        xp: 500,
        min_slot: 0,
        per_pass: 0,
        reject: 0,
        description: "Single combat between fighters using only their bare hands.",
    },
];

/// The seven daily ranked matches (`0x1002bc3c0`).
pub const RANKED: [MatchTemplate; 7] = [
    MatchTemplate {
        id: 8,
        name: "Challenger",
        flags: 2,
        min_rank: 0,
        max_rank: 0,
        secondary: 256,
        cash: 768,
        teams: 1,
        fighters: 0,
        rounds: 0,
        xp: 1000,
        min_slot: 0,
        per_pass: 2,
        reject: 0,
        description: "Single combat between two fighters with no restrictions of rank.",
    },
    MatchTemplate {
        id: 7,
        name: "Elimination",
        flags: 0,
        min_rank: 0,
        max_rank: 0,
        secondary: 3,
        cash: 2048,
        teams: 1,
        fighters: 0,
        rounds: 0,
        xp: 1200,
        min_slot: 0,
        per_pass: 2,
        reject: 0,
        description: "A series of five duels, the victor continues to the next round.",
    },
    MatchTemplate {
        id: 13,
        name: "Valiance",
        flags: 2,
        min_rank: 0,
        max_rank: 0,
        secondary: 4,
        cash: 768,
        teams: 1,
        fighters: 0,
        rounds: 0,
        xp: 2000,
        min_slot: 1,
        per_pass: 2,
        reject: 0,
        description: "One fighter faces two fighters of one rank lower.",
    },
    MatchTemplate {
        id: 14,
        name: "Valour",
        flags: 2,
        min_rank: 0,
        max_rank: 0,
        secondary: 6,
        cash: 1536,
        teams: 1,
        fighters: 0,
        rounds: 0,
        xp: 3000,
        min_slot: 2,
        per_pass: 2,
        reject: 0,
        description: "One fighter faces three fighters of two ranks lower.",
    },
    MatchTemplate {
        id: 9,
        name: "Captain",
        flags: 2,
        min_rank: 0,
        max_rank: 0,
        secondary: 6,
        cash: 1536,
        teams: 3,
        fighters: 0,
        rounds: 0,
        xp: 4500,
        min_slot: 2,
        per_pass: 2,
        reject: 0,
        description: "One fighter leads two of lower rank against an equal team.",
    },
    MatchTemplate {
        id: 12,
        name: "Brawl",
        flags: 0,
        min_rank: 0,
        max_rank: -1,
        secondary: 2,
        cash: 512,
        teams: 1,
        fighters: 0,
        rounds: 0,
        xp: 2500,
        min_slot: 0,
        per_pass: 1,
        reject: 3,
        description: "Six fighters engage in unarmed combat until only one stands. Winners are chosen based on their performance.",
    },
    MatchTemplate {
        id: 11,
        name: "The Beast",
        flags: 2,
        min_rank: 0,
        max_rank: 0,
        secondary: 512,
        cash: 196608,
        teams: 3,
        fighters: 0,
        rounds: 0,
        xp: 10000,
        min_slot: 2,
        per_pass: 1,
        reject: 0,
        description: "Three fighters attempt the feat of bringing down a mighty ogre.",
    },
];

/// The five tournament formats (`0x1002bc970`).
pub const TOURNAMENTS: [MatchTemplate; 5] = [
    MatchTemplate {
        id: 7,
        name: "Elimination",
        flags: 0,
        min_rank: 0,
        max_rank: 0,
        secondary: 0,
        cash: 0,
        teams: 1,
        fighters: 2,
        rounds: 4,
        xp: 1200,
        min_slot: 0,
        per_pass: 1,
        reject: 0,
        description: "Four rounds of duels, victors face each other until one remains.",
    },
    MatchTemplate {
        id: 4,
        name: "Fray",
        flags: 0,
        min_rank: 0,
        max_rank: 0,
        secondary: 0,
        cash: 0,
        teams: 1,
        fighters: 6,
        rounds: 1,
        xp: 5000,
        min_slot: 0,
        per_pass: 1,
        reject: 0,
        description: "Six fighters engage in combat until only one remains standing. Winners are chosen based on their performance.",
    },
    MatchTemplate {
        id: 17,
        name: "Champion",
        flags: 4,
        min_rank: 0,
        max_rank: 0,
        secondary: 0,
        cash: 0,
        teams: 1,
        fighters: 2,
        rounds: 4,
        xp: 1500,
        min_slot: 0,
        per_pass: 2,
        reject: 0,
        description: "Four rounds of duels using a weapon chosen for the event. Victors face each other until one remains.",
    },
    MatchTemplate {
        id: 18,
        name: "Melee",
        flags: 4,
        min_rank: 0,
        max_rank: 0,
        secondary: 0,
        cash: 0,
        teams: 1,
        fighters: 6,
        rounds: 1,
        xp: 6000,
        min_slot: 0,
        per_pass: 2,
        reject: 0,
        description: "Six fighters engage in combat using a weapon chosen for the event. Winners are chosen based on their performance.",
    },
    MatchTemplate {
        id: 16,
        name: "Conquest",
        flags: 0,
        min_rank: 0,
        max_rank: 0,
        secondary: 0,
        cash: 0,
        teams: 2,
        fighters: 2,
        rounds: 3,
        xp: 1500,
        min_slot: 0,
        per_pass: 2,
        reject: 0,
        description: "Teams of two face each other in three rounds. Defeated fighters do not continue to the next round.",
    },
];

/// The 18 weekly tournament prize ids (`0x1002bcba0`).
pub const TOURNAMENT_PRIZES: [u32; 18] = [
    213, 26, 440, 12, 65, 22, 62, 63, 953, 420, 60, 21, 24, 1166, 673, 33, 34, 61,
];

/// The season's four armour sets (`0x1002bcc40`).
pub const SEASON_PRIZES: [[u32; 4]; 4] = [
    [1175, 1174, 1177, 1176],
    [1167, 1165, 1168, 1169],
    [1171, 1170, 1173, 1172],
    [1178, 1179, 1180, 1181],
];

/// The 19 rival companies' names and three profile bytes (`0x1002bbf30`).
pub const COMPANIES: [(&str, [u8; 3]); 19] = [
    ("Bonten Company", [0x01, 0x07, 0x01]),
    ("Fitten Gang", [0x01, 0x8C, 0x42]),
    ("Vanrel's Band", [0x01, 0x01, 0x24]),
    ("Mowbrots Bunch", [0x01, 0x72, 0xCB]),
    ("Dullos Company", [0x00, 0xCB, 0x00]),
    ("Dayker's Crew", [0x00, 0xE3, 0x60]),
    ("Lofton Company", [0x00, 0x07, 0x23]),
    ("Frerren Section", [0x00, 0x14, 0x9B]),
    ("Evelique Company", [0x00, 0x54, 0x07]),
    ("Bantree Outfit", [0x00, 0xB5, 0x7D]),
    ("Forriel Company", [0x00, 0xFD, 0xDB]),
    ("Rallop Company", [0x00, 0xD5, 0x25]),
    ("Queedell's Knot", [0x00, 0x14, 0x61]),
    ("Clivard's Lot", [0x00, 0xD6, 0x05]),
    ("Westryn Company", [0x00, 0x5C, 0x04]),
    ("Hisselmore Guard", [0x00, 0xD3, 0xF3]),
    ("Greenque Pact", [0x00, 0x7D, 0x3E]),
    ("Solden Corpora", [0x00, 0x43, 0xB5]),
    ("Rickwich Fellers", [0x00, 0x8C, 0x72]),
];

/// A template by id: the standard table first, then the ranked one.
pub fn template(id: u32) -> Option<&'static MatchTemplate> {
    STANDARD
        .iter()
        .chain(RANKED.iter())
        .find(|template| template.id == id)
}

/// The original's name lookup: id 11 is the log's lower-case "beast".
pub fn lookup_name(id: u32) -> Option<&'static str> {
    if id == 11 {
        return Some("beast");
    }
    template(id).map(|template| template.name)
}

/// Unpacks a three-byte base-12 money value.
pub fn unpack_money(packed: u32) -> u32 {
    let b0 = packed & 0xFF;
    let b1 = (packed >> 8) & 0xFF;
    let b2 = (packed >> 16) & 0xFF;
    b0 + 12 * b1 + 144 * b2
}

/// Packs money as three base-12 bytes.
pub fn pack_money(value: u32) -> u32 {
    let b0 = value % 12;
    let rest = value / 12;
    let b1 = rest % 12;
    let b2 = (rest / 12).min(255);
    b0 | (b1 << 8) | (b2 << 16)
}

/// The cash a match pays at `slot`: the template's unpacked cash times the
/// slot plus one, packed again.
pub fn cash_prize(template: &MatchTemplate, slot: u32) -> u32 {
    pack_money(unpack_money(template.cash) * (slot + 1))
}

/// The XP pot: the base times `1 + 0.25·slot`, truncated.
pub fn prize_pot(template: &MatchTemplate, slot: u32) -> u32 {
    (template.xp as f32 * (1.0 + 0.25 * slot as f32)) as u32
}

/// The XP paid from a pot after the survivors' health: `pot·left/max`,
/// truncated, plus half for a company manager.
pub fn xp_award(pot: u32, hp_left: f32, hp_max: f32, manager: bool) -> u32 {
    if hp_max <= 0.0 {
        return 0;
    }
    let mut total = (pot as f32 * hp_left / hp_max) as u32;
    if manager {
        total += total / 2;
    }
    total
}

/// Splits a total XP award by each fighter's damage, truncating.
pub fn xp_split(total: u32, damage: &[u32]) -> Vec<u32> {
    let sum: u32 = damage.iter().sum();
    if sum == 0 {
        return vec![0; damage.len()];
    }
    damage
        .iter()
        .map(|&d| (total as u64 * d as u64 / sum as u64) as u32)
        .collect()
}

/// Rank points: one plus one per 5000 XP in each of the six pools.
pub fn rank_points(xp: &[u16; 6]) -> u32 {
    1 + xp.iter().map(|&x| x as u32 / XP_PER_RANK).sum::<u32>()
}

/// The loadout tier: `min((rank_points − 1)/2, 5)`.
pub fn loadout_tier(rank_points: u32) -> u32 {
    (rank_points.saturating_sub(1) / 2).min(5)
}

/// The XP one award gives: `min(total/rank_points, 5000)`.
pub fn xp_gain(total: u32, rank_points: u32) -> u32 {
    if rank_points == 0 {
        return 0;
    }
    (total / rank_points).min(XP_PER_RANK)
}

/// A daily wage from the character's class mask and rank points.
pub fn wage(class_mask: u32, rank_points: u32) -> u32 {
    match class_mask {
        0x3000_0000 => rank_points.saturating_sub(1),
        0x4000_0000 => 2,
        0x5000_0000 => 12,
        _ => loadout_tier(rank_points) + 1,
    }
}

/// Daily condition recovery, saturating at 0xFFFF.
pub fn recover(condition: u16, physician: bool) -> u16 {
    condition.saturating_add(if physician {
        RECOVER_PHYSICIAN
    } else {
        RECOVER
    })
}

/// A fighter at or above 0xCCC is fit.
pub fn is_fit(condition: u16) -> bool {
    condition >= UNFIT_CONDITION
}

/// A day's place in the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    MidWeek,
    NewWeek,
    EndOfWeek,
}

/// Day 1 mod 7 starts the week; day 0 mod 7 ends it.
pub fn phase(day: u32) -> Phase {
    match day % 7 {
        1 => Phase::NewWeek,
        0 => Phase::EndOfWeek,
        _ => Phase::MidWeek,
    }
}

/// The week of the four-week season.
pub fn week_of_season(day: u32) -> u32 {
    (day % (7 * WEEKS_PER_SEASON)) / 7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_templates_match_the_binary() {
        let skirmish = template(3).unwrap();
        assert_eq!(skirmish.name, "Skirmish");
        assert_eq!(
            (skirmish.teams, skirmish.cash, skirmish.xp),
            (3, 1536, 3900)
        );
        let pugilism = template(6).unwrap();
        assert_eq!(pugilism.max_rank, -1);
        assert_eq!(pugilism.cash, 6);
    }

    #[test]
    fn the_ranked_beast_and_lookup() {
        let beast = template(11).unwrap();
        assert_eq!(beast.name, "The Beast");
        assert_eq!((beast.cash, beast.xp), (196608, 10000));
        assert_eq!((beast.min_slot, beast.per_pass), (2, 1));
        assert_eq!(lookup_name(11), Some("beast"));
        assert_eq!(lookup_name(3), Some("Skirmish"));
        assert_eq!(lookup_name(10), None);
    }

    #[test]
    fn the_tournament_formats() {
        let champion = TOURNAMENTS.iter().find(|t| t.id == 17).unwrap();
        assert_eq!(champion.flags, 4);
        assert_eq!((champion.fighters, champion.rounds), (2, 4));
        assert_eq!(TOURNAMENTS[4].rounds, 3);
    }

    #[test]
    fn the_packed_money_round_trips() {
        assert_eq!(unpack_money(256), 12);
        assert_eq!(pack_money(12), 256);
        assert_eq!(unpack_money(pack_money(1727)), 1727);
        assert_eq!(cash_prize(template(1).unwrap(), 1), pack_money(24));
        assert_eq!(
            cash_prize(template(8).unwrap(), 5),
            pack_money(unpack_money(768) * 6)
        );
    }

    #[test]
    fn the_prizes_and_xp_split() {
        let duel = template(1).unwrap();
        assert_eq!(prize_pot(duel, 3), 1750);
        assert_eq!(xp_award(1750, 0.5, 1.0, false), 875);
        assert_eq!(xp_award(1750, 0.5, 1.0, true), 1312);
        assert_eq!(xp_split(100, &[1, 1]), vec![50, 50]);
        assert_eq!(xp_split(100, &[0, 0]), vec![0, 0]);
    }

    #[test]
    fn ranks_tiers_and_gains() {
        let mut xp = [0u16; 6];
        xp[0] = 5000;
        xp[1] = 4999;
        xp[2] = 10_000;
        assert_eq!(rank_points(&xp), 4);
        assert_eq!(loadout_tier(7), 3);
        assert_eq!(loadout_tier(1), 0);
        assert_eq!(loadout_tier(20), 5);
        assert_eq!(xp_gain(10_000, 4), 2500);
        assert_eq!(xp_gain(100_000, 4), 5000);
    }

    #[test]
    fn wages_and_recovery() {
        assert_eq!(wage(0x3000_0000, 5), 4);
        assert_eq!(wage(0x4000_0000, 5), 2);
        assert_eq!(wage(0x5000_0000, 5), 12);
        assert_eq!(wage(0, 5), 3);
        assert_eq!(recover(1000, false), 7553);
        assert_eq!(recover(1000, true), 10830);
        assert_eq!(recover(65_000, false), 65_535);
        assert!(is_fit(0x0CCC));
        assert!(!is_fit(0x0CCB));
    }

    #[test]
    fn the_calendar() {
        assert_eq!(phase(7), Phase::EndOfWeek);
        assert_eq!(phase(8), Phase::NewWeek);
        assert_eq!(phase(9), Phase::MidWeek);
        assert_eq!(week_of_season(27), 3);
        assert_eq!((WEEKS_PER_SEASON, FIGHTING_DAYS), (4, 6));
        assert_eq!(HIRE_FEE, 12);
    }

    #[test]
    fn the_prize_tables() {
        assert_eq!(TOURNAMENT_PRIZES.len(), 18);
        assert_eq!(TOURNAMENT_PRIZES[0], 213);
        assert_eq!(SEASON_PRIZES[3][3], 1181);
        assert_eq!(COMPANIES.len(), 19);
        assert_eq!(COMPANIES[0], ("Bonten Company", [0x01, 0x07, 0x01]));
        assert_eq!(POINT_LIMITS, [5, 10, 15, 20, 30, 40]);
    }
}
