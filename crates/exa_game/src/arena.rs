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
    /// The template's rank preset; −1 means unranked.
    pub preset_rank: i32,
    /// The secondary value; scaled by the slot but not the cash.
    pub secondary: u32,
    /// The raw packed base-12 cash prize.
    pub cash: u32,
    pub teams: u32,
    /// Tournament only: total fighters.
    pub fighters: u32,
    /// Tournament only: rounds.
    pub rounds: u32,
    /// The XP prize base.
    pub xp: u32,
    /// The earliest week slot the template may take.
    pub min_slot: u32,
    /// The maximum instances in one generation pass (0 = unlimited).
    pub per_pass: u32,
    /// Reject when `random(8) & reject != 0`.
    pub reject: u32,
    pub description: &'static str,
}

/// The six always-available standard matches.
pub const STANDARD: [MatchTemplate; 6] = [
    MatchTemplate {
        id: 1,
        name: "Duel",
        flags: 0,
        min_rank: 0,
        max_rank: 0,
        preset_rank: 0,
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
        preset_rank: 0,
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
        preset_rank: 0,
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
        preset_rank: 0,
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
        preset_rank: 0,
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
        preset_rank: -1,
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

/// The seven daily ranked matches.
pub const RANKED: [MatchTemplate; 7] = [
    MatchTemplate {
        id: 8,
        name: "Challenger",
        flags: 2,
        min_rank: 0,
        max_rank: 0,
        preset_rank: 0,
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
        preset_rank: 0,
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
        preset_rank: 0,
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
        preset_rank: 0,
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
        preset_rank: 0,
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
        preset_rank: -1,
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
        preset_rank: 0,
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

/// The five tournament formats.
pub const TOURNAMENTS: [MatchTemplate; 5] = [
    MatchTemplate {
        id: 7,
        name: "Elimination",
        flags: 0,
        min_rank: 0,
        max_rank: 0,
        preset_rank: 0,
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
        preset_rank: 0,
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
        preset_rank: 0,
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
        preset_rank: 0,
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
        preset_rank: 0,
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

/// The 18 weekly tournament prize ids.
pub const TOURNAMENT_PRIZES: [u32; 18] = [
    213, 26, 440, 12, 65, 22, 62, 63, 953, 420, 60, 21, 24, 1166, 673, 33, 34, 61,
];

/// The season's four armour sets.
pub const SEASON_PRIZES: [[u32; 4]; 4] = [
    [1175, 1174, 1177, 1176],
    [1167, 1165, 1168, 1169],
    [1171, 1170, 1173, 1172],
    [1178, 1179, 1180, 1181],
];

/// The 19 rival companies' names and three profile bytes.
pub const COMPANIES: [(&str, [u8; 3]); 19] = [
    ("Bonten Company", [0x01, 0x07, 0x01]),
    ("Fitten Gang", [0x42, 0x8C, 0x01]),
    ("Vanrel's Band", [0x24, 0x01, 0x01]),
    ("Mowbrots Bunch", [0xCB, 0x72, 0x01]),
    ("Dullos Company", [0x00, 0xCB, 0x00]),
    ("Dayker's Crew", [0x60, 0xE3, 0x00]),
    ("Lofton Company", [0x23, 0x07, 0x00]),
    ("Frerren Section", [0x9B, 0x14, 0x00]),
    ("Evelique Company", [0x07, 0x54, 0x00]),
    ("Bantree Outfit", [0x7D, 0xB5, 0x00]),
    ("Forriel Company", [0xDB, 0xFD, 0x00]),
    ("Rallop Company", [0x25, 0xD5, 0x00]),
    ("Queedell's Knot", [0x61, 0x14, 0x00]),
    ("Clivard's Lot", [0x05, 0xD6, 0x00]),
    ("Westryn Company", [0x04, 0x5C, 0x00]),
    ("Hisselmore Guard", [0xF3, 0xD3, 0x00]),
    ("Greenque Pact", [0x3E, 0x7D, 0x00]),
    ("Solden Corpora", [0xB5, 0x43, 0x00]),
    ("Rickwich Fellers", [0x72, 0x8C, 0x00]),
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

/// One minus the fourth power of a rank roll's unit, as the original
/// computes it: the unit squared, then that square times itself, not a
/// left-to-right product of four factors (the two differ in the last bits).
pub(crate) fn rank_curve(unit: f64) -> f64 {
    let square = unit * unit;
    1.0 - square * square
}

/// The rolled rank of a template at a day slot. An unranked template stays
/// −1; a flag-2 template takes the slot; otherwise the rank is
/// `round(slot·(1 − r⁴))`, biased towards the slot, with the r⁴ computed by
/// [`rank_curve`].
pub fn roll_rank(template: &MatchTemplate, slot: u32, r: f64) -> i32 {
    if template.preset_rank < 0 {
        return -1;
    }
    if template.flags & 2 != 0 {
        return slot as i32;
    }
    (slot as f64 * rank_curve(r)).round_ties_even() as i32
}

/// The rank window of a rolled match: Challenger (id 8) opens 0–5, every
/// other template spans the slot up to the rolled rank.
pub fn rank_window(template: &MatchTemplate, slot: u32, r: f64) -> (i32, i32) {
    let rank = roll_rank(template, slot, r);
    let min = if template.id == 8 { 0 } else { slot as i32 };
    let max = if template.id == 8 { 5 } else { rank };
    (min, max)
}

/// The secondary value scaled by the slot, packed the same way as cash.
pub fn scaled_secondary(template: &MatchTemplate, slot: u32) -> u32 {
    pack_money(unpack_money(template.secondary) * (slot + 1))
}

/// Whether a template may be generated at `slot`: its per-pass cap is not
/// reached, the slot is at least its minimum, and the reject mask does not
/// reject the 0–7 roll.
pub fn eligible(template: &MatchTemplate, slot: u32, used: u32, roll8: u32) -> bool {
    if template.per_pass > 0 && used >= template.per_pass {
        return false;
    }
    if slot < template.min_slot {
        return false;
    }
    roll8 & 7 & template.reject == 0
}

/// Draws a template index with the original's retry loop: the table index,
/// the cap, the minimum slot and the reject roll. Returns `None` when no
/// draw succeeds within `templates.len()·8` attempts.
pub fn pick_template(
    templates: &[MatchTemplate],
    slot: u32,
    used: &[u32],
    rng: &mut crate::item_container::Rng,
) -> Option<usize> {
    if templates.is_empty() {
        return None;
    }
    for _ in 0..templates.len() * 8 {
        let index = rng.below(templates.len() as u32) as usize;
        let template = &templates[index];
        let count = used.get(index).copied().unwrap_or(0);
        if eligible(template, slot, count, rng.below(8)) {
            return Some(index);
        }
    }
    None
}

/// Draws a weekly tournament prize id, rejecting the ids already in the
/// history.
pub fn pick_prize(rng: &mut crate::item_container::Rng, history: &[u32]) -> Option<u32> {
    for _ in 0..TOURNAMENT_PRIZES.len() * 4 {
        let id = TOURNAMENT_PRIZES[rng.below(TOURNAMENT_PRIZES.len() as u32) as usize];
        if !history.contains(&id) {
            return Some(id);
        }
    }
    None
}

/// A match's outcome as the settlement code sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Win,
    Practice,
    Other,
}

/// What a settled match pays and logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settlement {
    /// Whether the cash prize is added to the purse.
    pub pays: bool,
    /// The log tag: 0x20000 win, 0x40000 practice, 0x30000 otherwise.
    pub log_tag: u32,
    /// A Beast match was defeated.
    pub beast_defeated: bool,
    /// The matches-played counter always rises.
    pub played: bool,
}

/// Settles a match: only a win pays, a Beast win counts a beast, and the
/// tag records how it was fought.
pub fn settle(outcome: Outcome, template_id: u32) -> Settlement {
    match outcome {
        Outcome::Win => Settlement {
            pays: true,
            log_tag: 0x20000,
            beast_defeated: template_id == 11,
            played: true,
        },
        Outcome::Practice => Settlement {
            pays: false,
            log_tag: 0x40000,
            beast_defeated: false,
            played: true,
        },
        Outcome::Other => Settlement {
            pays: false,
            log_tag: 0x30000,
            beast_defeated: false,
            played: true,
        },
    }
}

/// The season's armour set when the last week is won with all four weekly
/// flags set.
pub fn season_award(week: u32, won: [bool; 4]) -> Option<[u32; 4]> {
    if week == WEEKS_PER_SEASON - 1 && won.iter().all(|&w| w) {
        Some(SEASON_PRIZES[week as usize])
    } else {
        None
    }
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

/// Rank points: one plus one per 5000 XP in each of the six pools.
pub fn rank_points(xp: &[u16; 6]) -> u32 {
    1 + xp.iter().map(|&x| x as u32 / XP_PER_RANK).sum::<u32>()
}

/// The loadout tier: `min((rank_points − 1)/2, 5)`.
pub fn loadout_tier(rank_points: u32) -> u32 {
    (rank_points.saturating_sub(1) / 2).min(5)
}

/// One body the match's experience pool reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XpFighter {
    /// Its record carries the match-participant flag.
    pub company: bool,
    /// The damage it dealt, the body's contribution counter.
    pub damage: i32,
    /// Its red health and maximum.
    pub health: f32,
    pub max: f32,
}

/// What a match's experience award came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XpAward {
    /// The pool every company body's share is taken from.
    pub pool: i32,
    /// The company's total damage, the share denominator.
    pub damage: i32,
}

/// The match's experience pool and damage total, before the shares are
/// handed out.
///
/// The base is `(tier · 0.25 + 1) · template experience` in single
/// precision, converted once (the tier factor is not rounded on its own).
/// The damage share is the opponents' missing health over their total
/// maximum — except in the Fray (template 4), where every body's damage
/// and missing health count. The base times the missing health, over the
/// maximum, is converted again. A trainer adds half again, truncated.
/// Each conversion is the 64-bit CVTSS2SI kept in 32 bits.
pub fn xp_pool(
    tier: i32,
    template_xp: u32,
    fray: bool,
    fighters: &[XpFighter],
    trainer: bool,
) -> XpAward {
    let mut damage = 0i32;
    let mut missing = 0.0f32;
    let mut total_max = 0.0f32;
    for fighter in fighters {
        if fray || !fighter.company {
            missing += fighter.max - fighter.health;
            total_max += fighter.max;
        }
        if fray || fighter.company {
            damage = damage.wrapping_add(fighter.damage);
        }
    }
    let base = crate::arena_match::cvtss2si_low32((tier as f32 * 0.25 + 1.0) * template_xp as f32);
    let mut pool = crate::arena_match::cvtss2si_low32(base as f32 * missing / total_max);
    if trainer {
        pool = pool.wrapping_add(pool / 2);
    }
    XpAward { pool, damage }
}

/// One company body's share of the pool: `round(pool · ratio)`, the ratio
/// `damage / total` divided in double and narrowed to single first.
pub fn xp_share(pool: i32, total_damage: i32, damage: i32) -> i32 {
    let ratio = (f64::from(damage) / f64::from(total_damage)) as f32;
    crate::arena_match::cvtss2si_low32(pool as f32 * ratio)
}

/// The Fray's template id: its experience award counts every body.
pub const FRAY: i32 = 4;

/// Every company body's share of a round's experience, in scene order: the
/// award's second walk. A body counts when it is on the company's side
/// and `credited` finds its record (the manager or a fighter still on the
/// books); its share is [`xp_share`] of the pool by its damage over the
/// award's total. Each entry is the body's scene index and its share.
pub fn xp_shares(
    award: XpAward,
    fighters: &[XpFighter],
    mut credited: impl FnMut(usize) -> bool,
) -> Vec<(usize, i32)> {
    fighters
        .iter()
        .enumerate()
        .filter(|&(index, fighter)| fighter.company && credited(index))
        .map(|(index, fighter)| (index, xp_share(award.pool, award.damage, fighter.damage)))
        .collect()
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
        assert_eq!(cash_prize(super::template(1).unwrap(), 1), pack_money(24));
        assert_eq!(
            cash_prize(template(8).unwrap(), 5),
            pack_money(unpack_money(768) * 6)
        );
    }

    #[test]
    fn the_match_xp_pool_and_shares() {
        let duel = template(1).unwrap();
        assert_eq!(prize_pot(duel, 3), 1750);
        let fighters = [
            XpFighter {
                company: true,
                damage: 2,
                health: 90.0,
                max: 100.0,
            },
            XpFighter {
                company: true,
                damage: 3,
                health: 80.0,
                max: 100.0,
            },
            XpFighter {
                company: false,
                damage: 0,
                health: 50.0,
                max: 100.0,
            },
            XpFighter {
                company: false,
                damage: 0,
                health: 20.0,
                max: 100.0,
            },
        ];
        // Tier 3: the factor 1.75 is not rounded on its own, so the base is
        // 8 750; the opponents miss 130 of 200, and 5 687.5 rounds to even.
        let award = xp_pool(3, 5_000, false, &fighters, false);
        assert_eq!((award.pool, award.damage), (5_688, 5));
        // The ratios are the doubles 0.4 and 0.6 narrowed: 2 275.2 and
        // 3 412.8 round to 2 275 and 3 413.
        assert_eq!(xp_share(5_688, 5, 2), 2_275);
        assert_eq!(xp_share(5_688, 5, 3), 3_413);
        // A trainer adds half again, truncated.
        assert_eq!(xp_pool(3, 5_000, false, &fighters, true).pool, 8_532);
        // The Fray counts every body's damage and missing health: 160 of
        // 400 maximum, so 3 500.
        assert_eq!(xp_pool(3, 5_000, true, &fighters, false).pool, 3_500);
        // Only company bodies with a record are credited, in scene order.
        let shares = xp_shares(award, &fighters, |index| index != 0);
        assert_eq!(shares, vec![(1, 3_413)]);
        // No damage dealt: the ratio 0/0 is NaN and the share converts to 0.
        assert_eq!(xp_share(5_688, 0, 0), 0);
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
        assert_eq!(COMPANIES[1], ("Fitten Gang", [0x42, 0x8C, 0x01]));
        assert_eq!(POINT_LIMITS, [5, 10, 15, 20, 30, 40]);
    }
}

#[cfg(test)]
mod generation_tests {
    use super::*;
    use crate::item_container::Rng;

    #[test]
    fn the_rank_roll_and_window() {
        let elimination = &RANKED[1];
        assert_eq!(elimination.name, "Elimination");
        assert_eq!(roll_rank(elimination, 3, 0.5), 3);
        assert_eq!(rank_window(elimination, 3, 0.5), (3, 3));
        let challenger = &RANKED[0];
        assert_eq!(rank_window(challenger, 0, 0.5), (0, 5));
        let brawl = &RANKED[5];
        assert_eq!(roll_rank(brawl, 4, 0.9), -1);
        assert_eq!(rank_window(brawl, 4, 0.9), (4, -1));
    }

    #[test]
    fn the_rank_curve_squares_the_square() {
        // The original multiplies the unit once (the square) and squares
        // that; a left-to-right fourth product gives a different last bit.
        let unit = 0.995_644_835_510_462_8;
        let curve = rank_curve(unit);
        assert_eq!(curve.to_bits(), 0x3f91_b8f9_68a7_0d80);
        assert_ne!(curve.to_bits(), (1.0 - unit * unit * unit * unit).to_bits());
    }

    #[test]
    fn the_secondary_scales_with_the_slot() {
        let duel = template(1).unwrap();
        assert_eq!(scaled_secondary(duel, 3), pack_money(12));
    }

    #[test]
    fn the_generation_filters() {
        let valiance = &RANKED[2];
        assert!(eligible(valiance, 1, 0, 0));
        assert!(!eligible(valiance, 0, 0, 0));
        assert!(!eligible(valiance, 1, 2, 0));
        let brawl = &RANKED[5];
        assert!(!eligible(brawl, 0, 0, 1));
        assert!(eligible(brawl, 0, 0, 4));
    }

    #[test]
    fn a_template_is_drawn_with_the_engine_generator() {
        let mut rng = Rng(12345);
        let index = pick_template(&STANDARD, 0, &[0; 6], &mut rng);
        let mut check = Rng(12345);
        assert_eq!(index, Some(check.below(6) as usize));
        let mut rng = Rng(12345);
        let index = pick_template(&RANKED, 0, &[0; 7], &mut rng).unwrap();
        assert!(
            STANDARD
                .iter()
                .chain(RANKED.iter())
                .any(|t| t.id == RANKED[index].id)
        );
    }

    #[test]
    fn the_eighth_history_slot_still_refuses_a_prize() {
        // Seed 13's first draw is prize 8, which sits in the eighth history
        // slot: it is refused and a later draw is kept.
        let mut rng = Rng(13);
        assert_eq!(rng.below(TOURNAMENT_PRIZES.len() as u32), 7);
        let mut rng = Rng(13);
        let mut history = [1, 2, 3, 4, 5, 6, 7, 8];
        let prize = draw_prize(&mut rng, &mut history);
        assert!(history[0] > 8);
        assert_eq!(prize, TOURNAMENT_PRIZES[history[0] as usize - 1]);
        assert_eq!(&history[1..], &[1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(
            prize_conflict(8, &[7, 8], None),
            PrizeConflict {
                shop: true,
                merchant: false
            }
        );
        assert_eq!(
            prize_conflict(8, &[], Some(&[8])),
            PrizeConflict {
                shop: false,
                merchant: true
            }
        );
        assert_eq!(prize_conflict(8, &[], Some(&[9])), PrizeConflict::default());
    }

    #[test]
    fn a_prize_is_drawn_around_the_history() {
        let mut rng = Rng(7);
        let prize = pick_prize(&mut rng, &TOURNAMENT_PRIZES[..17]).unwrap();
        assert_eq!(prize, TOURNAMENT_PRIZES[17]);
        let mut rng = Rng(7);
        assert!(pick_prize(&mut rng, &[]).is_some());
    }

    #[test]
    fn a_settled_match_pays_only_on_a_win() {
        let win = settle(Outcome::Win, 11);
        assert!(win.pays && win.beast_defeated && win.played);
        assert_eq!(win.log_tag, 0x20000);
        let practice = settle(Outcome::Practice, 1);
        assert!(!practice.pays && !practice.beast_defeated);
        assert_eq!(practice.log_tag, 0x40000);
        assert_eq!(settle(Outcome::Other, 1).log_tag, 0x30000);
    }

    #[test]
    fn the_season_award_needs_the_last_week_and_all_wins() {
        assert_eq!(
            season_award(3, [true, true, true, true]),
            Some(SEASON_PRIZES[3])
        );
        assert_eq!(season_award(2, [true, true, true, true]), None);
        assert_eq!(season_award(3, [true, false, true, true]), None);
    }
}

/// A weapon row: a name and its one to three variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponEntry {
    pub name: &'static str,
    pub variants: &'static [u32],
}

const TIER0: [WeaponEntry; 8] = [
    WeaponEntry {
        name: "Hatchet",
        variants: &[23],
    },
    WeaponEntry {
        name: "Handsaw",
        variants: &[224],
    },
    WeaponEntry {
        name: "Hammer",
        variants: &[262],
    },
    WeaponEntry {
        name: "Cudgel",
        variants: &[282],
    },
    WeaponEntry {
        name: "Billhook",
        variants: &[952],
    },
    WeaponEntry {
        name: "Metal Bar",
        variants: &[227],
    },
    WeaponEntry {
        name: "Large Mallet",
        variants: &[396],
    },
    WeaponEntry {
        name: "Quarterstaff",
        variants: &[406],
    },
];
const TIER1: [WeaponEntry; 11] = [
    WeaponEntry {
        name: "Mace",
        variants: &[79],
    },
    WeaponEntry {
        name: "Spiked Club",
        variants: &[206],
    },
    WeaponEntry {
        name: "Dagger",
        variants: &[395],
    },
    WeaponEntry {
        name: "Long Knife",
        variants: &[225],
    },
    WeaponEntry {
        name: "Billhook (short)",
        variants: &[127],
    },
    WeaponEntry {
        name: "Sickle (short)",
        variants: &[954],
    },
    WeaponEntry {
        name: "Sledgehammer",
        variants: &[232],
    },
    WeaponEntry {
        name: "Iron-Shod Quarterstaff",
        variants: &[407],
    },
    WeaponEntry {
        name: "Billhook (long)",
        variants: &[46],
    },
    WeaponEntry {
        name: "Sickle (long)",
        variants: &[39],
    },
    WeaponEntry {
        name: "Pitchfork",
        variants: &[41],
    },
];
const TIER2: [WeaponEntry; 10] = [
    WeaponEntry {
        name: "Shortsword",
        variants: &[210],
    },
    WeaponEntry {
        name: "Axe",
        variants: &[19, 419, 213],
    },
    WeaponEntry {
        name: "War Hammer",
        variants: &[212, 26],
    },
    WeaponEntry {
        name: "Bar Mace",
        variants: &[30],
    },
    WeaponEntry {
        name: "Short Spear",
        variants: &[422],
    },
    WeaponEntry {
        name: "Maul",
        variants: &[25],
    },
    WeaponEntry {
        name: "Bardiche",
        variants: &[37],
    },
    WeaponEntry {
        name: "Great Axe",
        variants: &[32],
    },
    WeaponEntry {
        name: "Two Handed War Hammer",
        variants: &[420],
    },
    WeaponEntry {
        name: "Spear",
        variants: &[421],
    },
];
const TIER3: [WeaponEntry; 14] = [
    WeaponEntry {
        name: "Sword",
        variants: &[440],
    },
    WeaponEntry {
        name: "Falchion",
        variants: &[12],
    },
    WeaponEntry {
        name: "Spiked Mace",
        variants: &[13, 22],
    },
    WeaponEntry {
        name: "Flanged Mace",
        variants: &[65],
    },
    WeaponEntry {
        name: "Longsword",
        variants: &[1166],
    },
    WeaponEntry {
        name: "Two Handed Sword",
        variants: &[24],
    },
    WeaponEntry {
        name: "Grand Falchion",
        variants: &[60],
    },
    WeaponEntry {
        name: "Two Handed Battle Axe",
        variants: &[21],
    },
    WeaponEntry {
        name: "Two Handed Spiked Mace",
        variants: &[423],
    },
    WeaponEntry {
        name: "Two Handed Mace",
        variants: &[424],
    },
    WeaponEntry {
        name: "Voulge",
        variants: &[159],
    },
    WeaponEntry {
        name: "Guisarme",
        variants: &[62],
    },
    WeaponEntry {
        name: "Spetum",
        variants: &[63],
    },
    WeaponEntry {
        name: "Glaive",
        variants: &[953],
    },
];
const TIER4: [WeaponEntry; 4] = [
    WeaponEntry {
        name: "Greatsword",
        variants: &[673],
    },
    WeaponEntry {
        name: "Pole Axe",
        variants: &[33],
    },
    WeaponEntry {
        name: "Pole Hammer",
        variants: &[61],
    },
    WeaponEntry {
        name: "Halberd",
        variants: &[34],
    },
];
const TIER5: [WeaponEntry; 4] = TIER4;

/// The six loadout tiers, each row 0x158 bytes in the retail table: a
/// count and up to 14 entries of a name and three variants.
pub const WEAPON_TIERS: [&[WeaponEntry]; 6] = [&TIER0, &TIER1, &TIER2, &TIER3, &TIER4, &TIER5];

/// A tier's weapon rows.
pub fn weapon_tier(tier: u32) -> Option<&'static [WeaponEntry]> {
    WEAPON_TIERS.get(tier as usize).copied()
}

/// Picks one of an entry's variant ids, the engine's way.
///
/// The original counts the row's three slots that are set (an unsigned
/// compare against zero, so any non-zero word counts, one with the top bit
/// set included), steps the roll once and takes the slot at
/// `(count · roll) >> 32` **without** skipping the empty slots in between:
/// a row of `[7, 0, 9]` can draw the zero. A drawn zero means no weapon. The original then passes the id to
/// the item variant and condition roll, which Exarust applies when the
/// item is instantiated.
pub fn pick_variant(entry: &WeaponEntry, rng: &mut crate::item_container::Rng) -> u32 {
    let count = entry
        .variants
        .iter()
        .filter(|&&variant| variant != 0)
        .count() as u32;
    entry
        .variants
        .get(rng.below(count) as usize)
        .copied()
        .unwrap_or(0)
}

/// A tier's point limit.
pub fn point_limit(tier: u32) -> Option<u32> {
    POINT_LIMITS.get(tier as usize).copied()
}

#[cfg(test)]
mod weapon_tests {
    use super::*;
    use crate::item_container::Rng;

    #[test]
    fn the_tier_lengths_and_names() {
        assert_eq!(weapon_tier(0).unwrap().len(), 8);
        assert_eq!(weapon_tier(1).unwrap().len(), 11);
        assert_eq!(weapon_tier(2).unwrap().len(), 10);
        assert_eq!(weapon_tier(3).unwrap().len(), 14);
        assert_eq!(weapon_tier(4).unwrap().len(), 4);
        assert_eq!(weapon_tier(5).unwrap().len(), 4);
        assert_eq!(weapon_tier(2).unwrap()[1].name, "Axe");
        assert_eq!(weapon_tier(2).unwrap()[1].variants, [19, 419, 213]);
        assert_eq!(weapon_tier(3).unwrap()[2].variants, [13, 22]);
        assert!(weapon_tier(6).is_none());
    }

    #[test]
    fn a_variant_is_drawn_among_the_set_slots() {
        let entry = &weapon_tier(2).unwrap()[1];
        let mut rng = Rng(12345);
        let index = rng.below(3) as usize;
        let expected = entry.variants[index];
        let mut rng = Rng(12345);
        assert_eq!(pick_variant(entry, &mut rng), expected);
        let single = &weapon_tier(0).unwrap()[0];
        let mut rng = Rng(1);
        assert_eq!(pick_variant(single, &mut rng), 23);
        // An empty row draws slot zero, which is no weapon.
        let empty = WeaponEntry {
            name: "None",
            variants: &[],
        };
        assert_eq!(pick_variant(&empty, &mut Rng(0)), 0);
    }

    #[test]
    fn the_variant_draw_keeps_the_empty_slots() {
        // Two set slots (7 and 9) and an interior zero: the count is two but
        // the draw indexes the row as it stands, so the high roll takes the
        // zero. The compare is unsigned, so a word with the top bit set
        // counts like any other non-zero slot.
        let row = WeaponEntry {
            name: "Interior",
            variants: &[7, 0, 9],
        };
        assert_eq!(pick_variant(&row, &mut Rng(0)), 7);
        assert_eq!(pick_variant(&row, &mut Rng(u32::MAX)), 0);
        let negative = WeaponEntry {
            name: "Negative",
            variants: &[7, 0x8000_0000, 9],
        };
        assert_eq!(pick_variant(&negative, &mut Rng(0)), 7);
        assert_eq!(pick_variant(&negative, &mut Rng(u32::MAX)), 9);
    }

    #[test]
    fn the_point_limits() {
        assert_eq!(point_limit(0), Some(5));
        assert_eq!(point_limit(5), Some(40));
        assert_eq!(point_limit(6), None);
    }
}

/// A rolled match instance (the binary's 0x70-byte record).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchInstance {
    pub id: u32,
    pub flags: u32,
    pub min_rank: i32,
    pub max_rank: i32,
    pub slot: u32,
    pub rank: i32,
    pub secondary: u32,
    pub cash: u32,
    pub teams: u32,
    pub fighters: u32,
    pub rounds: u32,
    pub weapon_row: u32,
    pub xp: u32,
    pub min_slot: u32,
    pub per_pass: u32,
    pub reject: u32,
}

impl MatchInstance {
    /// Copies a template into a fresh instance.
    pub fn from_template(template: &MatchTemplate, slot: u32) -> Self {
        Self {
            id: template.id,
            flags: template.flags,
            min_rank: template.min_rank,
            max_rank: template.max_rank,
            slot,
            rank: template.preset_rank,
            secondary: template.secondary,
            cash: template.cash,
            teams: template.teams,
            fighters: template.fighters,
            rounds: template.rounds,
            weapon_row: 0,
            xp: template.xp,
            min_slot: template.min_slot,
            per_pass: template.per_pass,
            reject: template.reject,
        }
    }
}

/// Restores a saved day's special match after its template has been rolled
/// again at the saved slot: a positive rolled rank that is not flag-2 takes
/// the saved rank, a template other than the Challenger (id 8) keeps the
/// roll's rank window, and the saved flags always replace the template's.
/// The saved word after the flags is unused.
pub fn restore_special(instance: &mut MatchInstance, saved_rank: u32, saved_flags: u32) {
    if instance.rank > 0 && instance.flags & 2 == 0 {
        instance.rank = saved_rank as i32;
    }
    if instance.id != 8 {
        instance.min_rank = instance.slot as i32;
        instance.max_rank = instance.rank;
    }
    instance.flags = saved_flags;
}

/// Whether a weapon row holds the current item: the row's entries end at
/// the first zero, and a match before it is found. Used by the board's
/// tournament reward check.
pub fn row_holds(entries: &[u32], current: u32) -> bool {
    for &entry in entries {
        if entry == 0 {
            return false;
        }
        if entry == current {
            return true;
        }
    }
    false
}

/// The four state bytes a fresh tournament-prize item copy gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrizeCopy {
    /// The first state byte, written as 255.
    pub quality: u8,
    /// The next two state bytes, and the byte after them whose three packed
    /// tier fields are each set from 0.0, so every one is cleared.
    pub wear: [u8; 3],
}

/// Make a fresh prize copy: an item of the prize's kind whose first state
/// byte is 255 and whose other three are cleared, then its description
/// rebuilt (the description and the object-database lookup are the
/// caller's step).
#[must_use]
pub const fn prize_copy() -> PrizeCopy {
    PrizeCopy {
        quality: 0xff,
        wear: [0; 3],
    }
}

/// Whether the rank roll needs the MT19937 stream.
pub fn needs_rank(template: &MatchTemplate) -> bool {
    template.preset_rank >= 0 && template.flags & 2 == 0
}

/// Rolls a match instance. `r` is the MT19937 unit draw in `[0, 1)` and is
/// ignored when [`needs_rank`] is false. The LCG draws the weapon row.
pub fn roll_instance(
    template: &MatchTemplate,
    slot: u32,
    r: f64,
    rng: &mut crate::item_container::Rng,
) -> MatchInstance {
    let mut instance = MatchInstance::from_template(template, slot);
    instance.rank = if template.preset_rank < 0 {
        -1
    } else if template.flags & 2 != 0 {
        slot as i32
    } else {
        (slot as f64 * rank_curve(r)).round_ties_even() as i32
    };
    if template.id == 8 {
        instance.min_rank = 0;
        instance.max_rank = 5;
    } else {
        instance.min_rank = slot as i32;
        instance.max_rank = instance.rank;
    }
    if template.flags & 4 != 0 {
        let count = WEAPON_TIERS
            .get(slot as usize)
            .map(|tier| tier.len())
            .unwrap_or(0);
        instance.weapon_row = if count > 0 {
            rng.below(count as u32) + 1
        } else {
            0
        };
    }
    instance.secondary = scaled_secondary(template, slot);
    instance.cash = cash_prize(template, slot);
    instance
}

/// The six-row daily board from the ranked templates.
pub fn generate_daily_board(seed: u32, mut rank: impl FnMut() -> f64) -> [MatchInstance; 6] {
    let mut rng = crate::item_container::Rng(seed);
    let mut used = [0u32; RANKED.len()];
    let mut rows = Vec::with_capacity(6);
    for row in 0..6u32 {
        loop {
            let index = rng.below(RANKED.len() as u32) as usize;
            let template = &RANKED[index];
            if template.per_pass > 0 && used[index] >= template.per_pass {
                continue;
            }
            if row < template.min_slot {
                continue;
            }
            let reject = rng.below(8);
            if reject & template.reject != 0 {
                continue;
            }
            used[index] += 1;
            let r = if needs_rank(template) { rank() } else { 0.0 };
            rows.push(roll_instance(template, row, r, &mut rng));
            break;
        }
    }
    rows.try_into().expect("six daily rows")
}

/// A generated tournament board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentBoard {
    pub rows: [MatchInstance; 6],
    pub prize_id: u32,
    /// 1-based company ids for the 16 slots.
    pub companies: [u8; 16],
}

/// Draws a weekly prize id and pushes its 1-based index into the history.
pub fn draw_prize(rng: &mut crate::item_container::Rng, history: &mut [u32; 8]) -> u32 {
    loop {
        let index = rng.below(TOURNAMENT_PRIZES.len() as u32);
        // All eight history slots refuse a prize, the one about to be
        // dropped among them.
        if history.contains(&(index + 1)) {
            continue;
        }
        history.rotate_right(1);
        history[0] = index + 1;
        return TOURNAMENT_PRIZES[index as usize];
    }
}

/// A tournament prize found in a stock, as the generator reports it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrizeConflict {
    /// The shop's shelf holds the prize.
    pub shop: bool,
    /// The merchant's stock holds the prize (only checked with a
    /// merchant employed).
    pub merchant: bool,
}

/// Whether the drawn prize is already on the shop's shelf, or on the
/// merchant's when one is employed. The original reports these as "Reward
/// Error" after drawing, without drawing another prize.
pub fn prize_conflict(prize: u32, shop: &[u32], merchant: Option<&[u32]>) -> PrizeConflict {
    PrizeConflict {
        shop: shop.contains(&prize),
        merchant: merchant.is_some_and(|stock| stock.contains(&prize)),
    }
}

/// The six-row tournament board, its weekly prize and the company shuffle
/// that continues the same LCG stream.
pub fn generate_tournament_board(
    seed: u32,
    history: &mut [u32; 8],
    mut rank: impl FnMut() -> f64,
) -> TournamentBoard {
    let mut rng = crate::item_container::Rng(seed);
    let prize_id = draw_prize(&mut rng, history);
    let mut used = [0u32; TOURNAMENTS.len()];
    let mut rows = Vec::with_capacity(6);
    for row in 0..6u32 {
        loop {
            let index = rng.below(TOURNAMENTS.len() as u32) as usize;
            let template = &TOURNAMENTS[index];
            if template.per_pass > 0 && used[index] >= template.per_pass {
                continue;
            }
            if row < template.min_slot {
                continue;
            }
            let reject = rng.below(8);
            if reject & template.reject != 0 {
                continue;
            }
            used[index] += 1;
            let r = if needs_rank(template) { rank() } else { 0.0 };
            let mut instance = roll_instance(template, row, r, &mut rng);
            instance.min_rank = 5;
            instance.max_rank = row as i32;
            instance.slot = 5;
            instance.rank = row as i32;
            rows.push(instance);
            break;
        }
    }
    let companies = shuffle_companies(&mut rng);
    TournamentBoard {
        rows: rows.try_into().expect("six tournament rows"),
        prize_id,
        companies,
    }
}

/// Shuffles the 19 company ids into 16 slots, 1-based.
pub fn shuffle_companies(rng: &mut crate::item_container::Rng) -> [u8; 16] {
    let mut pool: Vec<u8> = (0..19).collect();
    let mut out = [0u8; 16];
    for (i, slot) in out.iter_mut().enumerate() {
        let index = rng.below((19 - i) as u32) as usize;
        *slot = pool[index] + 1;
        pool.remove(index);
    }
    out
}

#[cfg(test)]
mod board_tests {
    use super::*;
    use crate::item_container::Rng;

    #[test]
    fn the_instance_rolls() {
        let mut rng = Rng(1);
        let elimination = &RANKED[1];
        let m = roll_instance(elimination, 3, 0.5, &mut rng);
        assert_eq!((m.rank, m.min_rank, m.max_rank), (3, 3, 3));
        assert_eq!(m.cash, pack_money(384));
        assert_eq!(m.secondary, pack_money(12));

        let valour = &RANKED[3];
        let m = roll_instance(valour, 4, 0.9, &mut rng);
        assert_eq!((m.rank, m.min_rank, m.max_rank), (4, 4, 4));
        assert_eq!(m.cash, pack_money(360));
        assert_eq!(m.secondary, pack_money(30));

        let challenger = &RANKED[0];
        let m = roll_instance(challenger, 0, 0.0, &mut rng);
        assert_eq!((m.rank, m.min_rank, m.max_rank), (0, 0, 5));
        let brawl = &RANKED[5];
        let m = roll_instance(brawl, 4, 0.9, &mut rng);
        assert_eq!((m.rank, m.min_rank, m.max_rank), (-1, 4, -1));
    }

    #[test]
    fn the_daily_board_matches_the_seed() {
        let board = generate_daily_board(12345, || 0.5);
        let ids: Vec<u32> = board.iter().map(|m| m.id).collect();
        assert_eq!(ids, [8, 13, 8, 11, 13, 12]);
        let tiers: Vec<i32> = board.iter().map(|m| m.rank).collect();
        assert_eq!(tiers, [0, 1, 2, 3, 4, -1]);
    }

    #[test]
    fn the_company_shuffle_matches_the_seed() {
        let mut rng = Rng(0);
        assert_eq!(
            shuffle_companies(&mut rng),
            [1, 2, 17, 6, 8, 14, 9, 4, 11, 12, 3, 13, 5, 19, 7, 15]
        );
    }

    #[test]
    fn the_prize_history_rejects_repeats() {
        let mut rng = Rng(12345);
        let mut history = [0u32; 8];
        let first = draw_prize(&mut rng, &mut history);
        let second = draw_prize(&mut rng, &mut history);
        assert_ne!(first, second);
        assert_ne!(history[0], 0);
        assert_ne!(history[1], 0);
    }
}

/// An entrant in a roster row: an actor id and two weapon handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Entrant {
    pub actor_id: u32,
    pub weapon: [u64; 2],
}

/// A 0x30-byte roster row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RosterRow {
    pub count: u32,
    pub entries: [Entrant; 2],
    /// 0 = pending; 1 = the player; 2..17 = company team ids.
    pub team_id: u32,
}

/// The six-row roster.
pub type Roster = [RosterRow; 6];

/// The 8-byte opponent spawn profile.
pub fn spawn_profile(tier: u8, company: Option<u8>, armed: bool) -> [u8; 8] {
    let mut profile = [3, tier, 100, 10, 10, 0, 0, 0];
    if armed {
        profile[0] |= 0x04;
    }
    if let Some(id) = company
        && (1..=19).contains(&id)
    {
        profile[0] |= 0x08;
        let bytes = COMPANIES[(id - 1) as usize].1;
        profile[5..8].copy_from_slice(&bytes);
    }
    profile
}

/// Drops entrants below 0xCCC condition from pending rows.
pub fn drop_unfit(rows: &mut Roster, condition: impl Fn(u32) -> u16) {
    for row in rows.iter_mut() {
        if row.team_id > 0 {
            continue;
        }
        let mut i = 0;
        while (i as u32) < row.count {
            if condition(row.entries[i].actor_id) >= 0x0CCC {
                i += 1;
                continue;
            }
            let count = row.count as usize;
            for j in i..count.saturating_sub(1) {
                row.entries[j] = row.entries[j + 1];
            }
            row.count -= 1;
        }
    }
}

/// Assigns 2..6 team ids to empty pending rows, stopping at the first
/// pending row with entrants.
pub fn assign_round_ids(rows: &mut Roster, rng: &mut crate::item_container::Rng) {
    for row in rows.iter_mut() {
        if row.team_id != 0 {
            continue;
        }
        if row.count > 0 {
            return;
        }
        row.team_id = rng.below(5) + 2;
    }
}

/// The career counters the tournament resolution updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SeasonState {
    /// The current weekly prize instance id.
    pub prize: u32,
    /// 1 for a player week win; a company id+2 for a company win.
    pub week_won: [u32; 4],
    pub championships: u32,
    pub days_with_entrants: u32,
    pub seasons_won: u32,
}

/// The tournament's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    pub champion: Option<u32>,
    pub week: u32,
    /// The four season items granted, if the season completed.
    pub season_award: Option<[u32; 4]>,
}

/// Tallies the six rows by team id and resolves the week/season. `give`
/// receives every item granted.
pub fn resolve_tournament(
    rows: &Roster,
    day: u32,
    companies: &[u8; 16],
    state: &mut SeasonState,
    mut give: impl FnMut(u32),
) -> Resolution {
    let mut tally: Vec<(u32, u32)> = Vec::new();
    for row in rows {
        if let Some(entry) = tally.iter_mut().find(|(id, _)| *id == row.team_id) {
            entry.1 += 1;
        } else {
            tally.push((row.team_id, 1));
        }
    }
    let mut best = 0;
    let mut tie = false;
    for k in 1..tally.len() {
        if tally[k].1 > tally[best].1 {
            best = k;
            tie = false;
        } else if tally[k].1 == tally[best].1 {
            tie = true;
        }
    }
    let weeks = 4;
    let week = (day % (weeks * 7)) / 7;
    let champion = tally.get(best).copied().unwrap_or((0, 0));
    if champion.0 == 1 && champion.1 > 2 && !tie {
        if state.prize != 0 {
            give(state.prize);
        }
        state.championships += 1;
        state.week_won[week as usize] = 1;
        let mut award = None;
        if week == 3 {
            let complete = state.week_won.iter().all(|&won| won == 1);
            state.week_won = [0; 4];
            if complete {
                let set = ((day % (weeks * 7)) % weeks) as usize;
                for id in SEASON_PRIZES[set] {
                    give(id);
                }
                state.seasons_won += 1;
                award = Some(SEASON_PRIZES[set]);
            }
        }
        state.prize = 0;
        return Resolution {
            champion: Some(1),
            week,
            season_award: award,
        };
    }
    if champion.1 > 2 && !tie {
        // The week is worn by the winning team's company, not by whichever
        // team was tallied first.
        if let Some(company) = companies.get(champion.0.saturating_sub(2) as usize) {
            state.week_won[week as usize] = *company as u32 + 2;
        }
    }
    if rows.iter().any(|row| row.count > 0) {
        state.days_with_entrants += 1;
    }
    if week == 3 {
        state.week_won = [0; 4];
    }
    state.prize = 0;
    Resolution {
        champion: None,
        week,
        season_award: None,
    }
}

#[cfg(test)]
mod roster_tests {
    use super::*;
    use crate::item_container::Rng;

    fn row(count: u32, team_id: u32) -> RosterRow {
        RosterRow {
            count,
            entries: [Entrant::default(); 2],
            team_id,
        }
    }

    #[test]
    fn the_spawn_profile() {
        let p = spawn_profile(3, Some(15), false);
        assert_eq!(p, [0x0B, 3, 100, 10, 10, 4, 92, 0]);
        let p = spawn_profile(3, Some(15), true);
        assert_eq!(p[0], 0x0F);
        let p = spawn_profile(0, None, false);
        assert_eq!(p, [3, 0, 100, 10, 10, 0, 0, 0]);
    }

    #[test]
    fn the_unfit_are_dropped() {
        let mut rows = [row(0, 0); 6];
        rows[0].count = 2;
        rows[0].entries[0].actor_id = 10;
        rows[0].entries[1].actor_id = 11;
        drop_unfit(&mut rows, |id| if id == 10 { 0x0CCC } else { 0x0CCB });
        assert_eq!(rows[0].count, 1);
        assert_eq!(rows[0].entries[0].actor_id, 10);
        rows[1].team_id = 2;
        rows[1].count = 1;
        rows[1].entries[0].actor_id = 11;
        drop_unfit(&mut rows, |_| 0);
        assert_eq!(rows[1].count, 1);
    }

    #[test]
    fn the_empty_rows_get_round_ids() {
        let mut rows = [row(0, 0); 6];
        rows[2].count = 1;
        let mut rng = Rng(0);
        assign_round_ids(&mut rows, &mut rng);
        assert_eq!(rows[0].team_id, 2);
        assert_eq!(rows[1].team_id, 2);
        assert_eq!(rows[2].team_id, 0);
        assert_eq!(rows[3].team_id, 0);
    }

    #[test]
    fn the_champion_wins_the_week_and_season() {
        let mut rows = [row(1, 0); 6];
        for (row, team) in rows.iter_mut().zip([1, 1, 1, 2, 3, 2]) {
            row.team_id = team;
        }
        let companies = [15, 17, 4, 1, 13, 9, 5, 14, 8, 6, 11, 2, 18, 3, 19, 10];
        let mut state = SeasonState {
            week_won: [1, 1, 1, 0],
            prize: 62,
            ..Default::default()
        };
        let mut given = Vec::new();
        let result = resolve_tournament(&rows, 27, &companies, &mut state, |id| given.push(id));
        assert_eq!(result.champion, Some(1));
        assert_eq!(state.championships, 1);
        assert_eq!(state.week_won, [0, 0, 0, 0]);
        assert_eq!(result.season_award, Some(SEASON_PRIZES[3]));
        assert_eq!(state.seasons_won, 1);
        assert_eq!(given[0], 62);
        assert_eq!(&given[1..], &SEASON_PRIZES[3]);
    }

    #[test]
    fn a_company_win_records_its_team() {
        let mut rows = [row(1, 0); 6];
        for (row, team) in rows.iter_mut().zip([2, 2, 2, 1, 3, 4]) {
            row.team_id = team;
        }
        let companies = [15, 17, 4, 1, 13, 9, 5, 14, 8, 6, 11, 2, 18, 3, 19, 10];
        let mut state = SeasonState::default();
        let result = resolve_tournament(&rows, 7, &companies, &mut state, |_| {});
        assert_eq!(result.champion, None);
        assert_eq!(state.week_won[1], 15 + 2);
    }

    #[test]
    fn a_rival_week_wears_the_winning_teams_company() {
        // Team 3 is tallied first; team 2 wins three events, so the week
        // records the company of team 2 (the field's first entry).
        let mut rows = [row(1, 0); 6];
        for (row, team) in rows.iter_mut().zip([3, 2, 2, 2, 4, 5]) {
            row.team_id = team;
        }
        let companies = [15, 17, 4, 1, 13, 9, 5, 14, 8, 6, 11, 2, 18, 3, 19, 10];
        let mut state = SeasonState::default();
        let result = resolve_tournament(&rows, 7, &companies, &mut state, |_| {});
        assert_eq!(result.champion, None);
        assert_eq!(state.week_won[1], 15 + 2);
    }

    #[test]
    fn a_tie_has_no_champion() {
        let mut rows = [row(1, 0); 6];
        for (row, team) in rows.iter_mut().zip([1, 1, 2, 2, 3, 3]) {
            row.team_id = team;
        }
        let companies = [0; 16];
        let mut state = SeasonState::default();
        let result = resolve_tournament(&rows, 7, &companies, &mut state, |_| {});
        assert_eq!(result.champion, None);
        assert_eq!(state.championships, 0);
    }
}

/// The engine's MT19937 generator, used only for the arena rank roll.
#[derive(Debug, Clone)]
pub struct Mt19937 {
    state: [u32; 624],
    index: usize,
}

impl Mt19937 {
    /// Seeds the generator with the standard MT19937 initialisation.
    pub fn new(seed: u32) -> Self {
        let mut state = [0u32; 624];
        state[0] = seed;
        for i in 1..624 {
            state[i] = 1812433253u32
                .wrapping_mul(state[i - 1] ^ (state[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        Self { state, index: 624 }
    }

    fn twist(&mut self) {
        for i in 0..624 {
            let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % 624] & 0x7fff_ffff);
            let mut next = self.state[(i + 397) % 624] ^ (y >> 1);
            if y & 1 != 0 {
                next ^= 0x9908_b0df;
            }
            self.state[i] = next;
        }
        self.index = 0;
    }

    /// The next 32-bit output.
    pub fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// A unit draw in `[0, 1)`: `next_u32() · 2⁻³²`.
    pub fn unit(&mut self) -> f64 {
        self.next_u32() as f64 * 2.328_306_436_538_696_3e-10
    }
}

/// Rolls an instance using the MT stream for the rank. The stream is
/// advanced only when [`needs_rank`] is true.
pub fn roll_instance_mt(
    template: &MatchTemplate,
    slot: u32,
    mt: &mut Mt19937,
    rng: &mut crate::item_container::Rng,
) -> MatchInstance {
    let r = if needs_rank(template) { mt.unit() } else { 0.0 };
    roll_instance(template, slot, r, rng)
}

#[cfg(test)]
mod mt_tests {
    use super::*;
    use crate::item_container::Rng;

    #[test]
    fn the_mt19937_known_vector() {
        let mut mt = Mt19937::new(5489);
        assert_eq!(mt.next_u32(), 3_499_211_612);
        assert_eq!(mt.next_u32(), 581_869_302);
        assert_eq!(mt.next_u32(), 3_890_346_734);
        assert_eq!(mt.next_u32(), 3_586_334_585);
        assert_eq!(mt.next_u32(), 545_404_204);
    }

    #[test]
    fn the_rank_stream_is_not_consumed_without_a_roll() {
        let mut mt = Mt19937::new(5489);
        let mut rng = Rng(1);
        let _ = roll_instance_mt(&RANKED[0], 0, &mut mt, &mut rng);
        assert_eq!(mt.next_u32(), 3_499_211_612);
        let mut mt = Mt19937::new(5489);
        let _ = roll_instance_mt(&RANKED[5], 4, &mut mt, &mut rng);
        assert_eq!(mt.next_u32(), 3_499_211_612);
    }

    #[test]
    fn a_plain_rank_roll_consumes_one_unit() {
        let mut mt = Mt19937::new(5489);
        let mut check = mt.clone();
        let mut rng = Rng(1);
        let instance = roll_instance_mt(&RANKED[1], 3, &mut mt, &mut rng);
        let r = check.unit();
        let expected = (3.0 * rank_curve(r)).round_ties_even() as i32;
        assert_eq!(instance.rank, expected);
        assert_eq!(mt.next_u32(), check.next_u32());
    }
}

/// Picks a weapon row for `tier`, 1-based, or `None` for an empty tier.
pub fn pick_weapon_row(tier: u32, rng: &mut crate::item_container::Rng) -> Option<u32> {
    let rows = WEAPON_TIERS.get(tier as usize)?;
    if rows.is_empty() {
        return None;
    }
    Some(rng.below(rows.len() as u32) + 1)
}

/// Picks one variant of a tier's row (0-based, as `set_weapon` receives it).
/// A drawn zero is no weapon, kept as the engine returns it.
pub fn pick_weapon_variant(
    tier: u32,
    row: u32,
    rng: &mut crate::item_container::Rng,
) -> Option<u32> {
    let rows = WEAPON_TIERS.get(tier as usize)?;
    let entry = rows.get(row as usize)?;
    Some(pick_variant(entry, rng))
}

/// A spawned opponent's level: 60000 + rand(9999) for a company, else
/// 50000 + rand(9999).
pub fn spawn_level(company: bool, rng: &mut crate::item_container::Rng) -> u32 {
    let base = if company { 60_000 } else { 50_000 };
    base + rng.below(9_999)
}

#[cfg(test)]
mod spawn_tests {
    use super::*;
    use crate::item_container::Rng;

    #[test]
    fn the_weapon_row_and_variant() {
        let mut rng = Rng(12345);
        assert_eq!(pick_weapon_row(1, &mut rng), Some(5));
        let mut rng = Rng(12345);
        assert_eq!(pick_weapon_variant(2, 1, &mut rng), Some(419));
        assert_eq!(pick_weapon_row(6, &mut rng), None);
        assert_eq!(pick_weapon_variant(2, 99, &mut rng), None);
    }

    #[test]
    fn the_spawn_levels() {
        let mut rng = Rng(12345);
        let roll = rng.below(9_999);
        let mut rng = Rng(12345);
        assert_eq!(spawn_level(true, &mut rng), 60_000 + roll);
        let mut rng = Rng(12345);
        assert_eq!(spawn_level(false, &mut rng), 50_000 + roll);
    }

    #[test]
    fn a_saved_special_restores_its_rank_window_and_flags() {
        let challenger = super::template(8).unwrap();
        let mut instance = MatchInstance::from_template(challenger, 3);
        instance.rank = 4;
        instance.min_rank = 0;
        instance.max_rank = 5;
        restore_special(&mut instance, 2, 0x10);
        // The Challenger's flag 2 keeps the roll's rank.
        assert_eq!(instance.rank, 4);
        assert_eq!((instance.min_rank, instance.max_rank), (0, 5));
        assert_eq!(instance.flags, 0x10);
        // A ranked template keeps the roll's window and takes the saved
        // rank only when the roll gave one.
        let mut duel = roll_instance(
            super::template(1).unwrap(),
            4,
            0.0,
            &mut crate::item_container::Rng(1),
        );
        restore_special(&mut duel, 1, 0);
        assert_eq!(duel.rank, 1);
        assert_eq!(duel.min_rank, 4);
        assert_eq!(duel.max_rank, 1);
        let mut zero = roll_instance(
            super::template(1).unwrap(),
            4,
            0.0,
            &mut crate::item_container::Rng(1),
        );
        zero.rank = 0;
        restore_special(&mut zero, 5, 0);
        assert_eq!(zero.rank, 0);
    }

    #[test]
    fn a_weapon_row_stops_at_its_first_empty_entry() {
        assert!(!row_holds(&[0, 0, 0], 0));
        assert!(!row_holds(&[0, 7, 0], 7));
        assert!(row_holds(&[7, 0, 0], 7));
        assert!(row_holds(&[3, 7, 0], 7));
        assert!(!row_holds(&[3, 0, 7], 7));
        assert!(!row_holds(&[], 7));
    }

    #[test]
    fn a_prize_copy_is_fresh() {
        assert_eq!(prize_copy().quality, 255);
        assert_eq!(prize_copy().wear, [0, 0, 0]);
    }
}
