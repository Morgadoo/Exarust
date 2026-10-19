//! Arena career: a company, its roster, and a match on the standard board.
//!
//! A new career holds 343 copper. Copper, silver and gold convert at 12 to 1.
//! The manager counts toward the eight fighters, so seven combatants fit, and
//! only one merchant, physician or trainer. Entering a match takes its fee.
//! A win pays the prize. Experience at tier 0, credited to the first
//! combatant, is the template amount times the damage share, plus half with
//! a trainer. A shared finish multiplies by 1 + tier/4 and splits that pool
//! by each combatant's contribution. Ending a day pays wages: a merchant 2,
//! a physician 12, a trainer the skill total, a combatant half of that, at
//! most 5, plus 1. The night adds 6553 to each health, or 9830 with a
//! physician, and stops at 65535. A hired trainer adds a lesson to each
//! combatant whose current technique the trainer knows. A day mod 7 of 1
//! resolves the week, and four won weeks grant the season. A recruitment
//! costs 12 copper. A shop line prices its low fourteen bits at 1, 12, 144
//! or 1440 copper, and a sale is a quarter of that basket. A match stake is
//! a coin dword, and a tier multiplies that copper by one more than the tier.
//!
//! Evidence: the arena career notes (money seed, coin rate, roster caps,
//! the six standard matches and the seven specials, fee on entry, prize on
//! a win, the experience pool `(tier/4 + 1) · template · damage share`).

/// Copper in one silver, and silver in one gold.
pub const COIN: u32 = 12;
/// Copper a new career starts with.
pub const STARTING_COPPER: u32 = 343;
/// Combatants a company can hire. The manager holds the eighth place.
pub const MAX_COMBATANTS: usize = 7;
/// A merchant's daily wage, in copper.
pub const MERCHANT_WAGE: u32 = 2;
/// A physician's daily wage, in copper.
pub const PHYSICIAN_WAGE: u32 = 12;
/// Weeks in a season. The fourth week, index 3, can grant the season.
pub const SEASON_WEEKS: u32 = 4;
/// Health below this drops a fighter from a tournament entry.
pub const INJURED: u16 = 0x0ccc;
/// Overnight rest added to health. A hired physician uses the larger step.
pub const REST: u16 = 0x1999;
pub const REST_WITH_PHYSICIAN: u16 = 0x2666;
/// A full health pool.
pub const FULL_HEALTH: u16 = 0xffff;
/// Copper a recruitment costs.
pub const HIRE_PRICE: u32 = 12;
/// The two high bits of a shop line select its unit.
pub const LINE_UNIT: u16 = 0xc000;
/// The low fourteen bits of a shop line are how many of that unit it holds.
pub const LINE_COUNT: u16 = 0x3fff;
/// Both unit bits: each count is ten gold, 1440 copper.
pub const UNIT_TEN_GOLD: u16 = 0xc000;
/// The high unit bit alone: each count is one gold, 144 copper.
pub const UNIT_GOLD: u16 = 0x8000;
/// The low unit bit alone: each count is one silver, 12 copper.
pub const UNIT_SILVER: u16 = 0x4000;
/// Skill experience in one level.
pub const SKILL_STEP: u16 = 5000;
/// Experience a matching lesson divides by the fighter's rank.
pub const LESSON: u32 = 1000;
/// A skill at this level is no longer the one being trained.
pub const TRAINED_LEVEL: u32 = 5;
/// The company's id in a tournament result.
pub const OUR_COMPANY: u32 = 1;
/// Wins of six events that take the week, when nobody else matches them.
pub const WEEK_WINS: u32 = 3;

/// A job on the roster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profession {
    Combatant,
    Merchant,
    Physician,
    Trainer,
}

/// Someone the company employs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    pub profession: Profession,
    /// Experience credited from matches.
    pub xp: u32,
    /// Six skill levels. A stored skill word divided by 5000. Wages read this.
    pub skills: [u32; 6],
    /// Raw skill experience. A level is this divided by 5000.
    pub skill_xp: [u16; 6],
    /// Technique ids, six skills by six slots. Slot `level` is in training.
    pub techniques: [[u8; 6]; 6],
    /// Health, 0 to 65535. A full pool is 65535.
    pub health: u16,
    /// Still named in a tournament entry.
    pub in_entry: bool,
}

/// One of the six matches the board offers every day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardMatch {
    pub name: &'static str,
    pub fee: u32,
    pub prize: u32,
    pub per_side: u8,
    pub xp: u32,
}

/// Copper in a coin dword. The first byte is copper, the next is silver,
/// and the high 16 bits are gold, at 12 to 1.
pub fn coin_total(packed: u32) -> u32 {
    let copper = packed & 0xff;
    let silver = (packed >> 8) & 0xff;
    let gold = packed >> 16;
    copper + silver * COIN + gold * COIN * COIN
}

/// A tier multiplies a stake by one more than the tier. Tier 0 keeps it.
pub fn scaled_stake(copper: u32, tier: u32) -> u32 {
    copper.saturating_mul(tier.saturating_add(1))
}

/// Duel, Doubles, Skirmish, Fray, Reserve, Pugilism.
/// Fees and prizes are tier-0 copper, read from the coin dword.
pub fn standard_board() -> [BoardMatch; 6] {
    [
        BoardMatch {
            name: "Duel",
            fee: 3,
            prize: 12,
            per_side: 1,
            xp: 1000,
        },
        BoardMatch {
            name: "Doubles",
            fee: 6,
            prize: 48,
            per_side: 2,
            xp: 2400,
        },
        BoardMatch {
            name: "Skirmish",
            fee: 9,
            prize: 72,
            per_side: 3,
            xp: 3900,
        },
        BoardMatch {
            name: "Fray",
            fee: 3,
            prize: 24,
            per_side: 1,
            xp: 5000,
        },
        BoardMatch {
            name: "Reserve",
            fee: 3,
            prize: 48,
            per_side: 3,
            xp: 1000,
        },
        BoardMatch {
            name: "Pugilism",
            fee: 1,
            prize: 6,
            per_side: 1,
            xp: 500,
        },
    ]
}

/// Challenger, Elimination, Valiance, Valour, Captain, Brawl, The Beast.
/// Fees and prizes are tier-0 copper. A higher tier multiplies both by one
/// more than the tier.
pub fn special_board() -> [BoardMatch; 7] {
    [
        BoardMatch {
            name: "Challenger",
            fee: 12,
            prize: 36,
            per_side: 1,
            xp: 1000,
        },
        BoardMatch {
            name: "Elimination",
            fee: 3,
            prize: 96,
            per_side: 1,
            xp: 1200,
        },
        BoardMatch {
            name: "Valiance",
            fee: 4,
            prize: 36,
            per_side: 1,
            xp: 2000,
        },
        BoardMatch {
            name: "Valour",
            fee: 6,
            prize: 72,
            per_side: 1,
            xp: 3000,
        },
        BoardMatch {
            name: "Captain",
            fee: 6,
            prize: 72,
            per_side: 3,
            xp: 4500,
        },
        BoardMatch {
            name: "Brawl",
            fee: 2,
            prize: 24,
            per_side: 1,
            xp: 2500,
        },
        BoardMatch {
            name: "The Beast",
            fee: 24,
            prize: 432,
            per_side: 3,
            xp: 10_000,
        },
    ]
}

/// Experience scale for a match tier. Tier 0 is the template. Each further
/// tier adds a quarter.
pub fn xp_scale(tier: u32) -> f32 {
    1.0 + tier as f32 * 0.25
}

/// Split a pool by contribution counts. A zero total leaves every share at
/// 0. Integer division's remainder goes to the earliest largest remainder.
pub fn split_experience(pool: u32, shares: &[u32]) -> Vec<u32> {
    let total: u64 = shares.iter().map(|&share| u64::from(share)).sum();
    if total == 0 {
        return vec![0; shares.len()];
    }
    let pool = u64::from(pool);
    let mut out = vec![0u32; shares.len()];
    let mut given = 0u64;
    let mut remainders = Vec::with_capacity(shares.len());
    for (index, &share) in shares.iter().enumerate() {
        let share = u64::from(share);
        let base = pool * share / total;
        out[index] = base as u32;
        given += base;
        remainders.push((pool * share % total, index));
    }
    remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut left = pool - given;
    for (_, index) in remainders {
        if left == 0 {
            break;
        }
        if shares[index] == 0 {
            continue;
        }
        out[index] = out[index].saturating_add(1);
        left -= 1;
    }
    out
}

/// The company the player manages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Company {
    pub name: String,
    pub copper: u32,
    pub roster: Vec<Member>,
    pub won: u32,
    pub lost: u32,
    pub spent: u32,
    pub earned: u32,
    /// Days ended. A new career is on day 0.
    pub day: u32,
    /// Whether the company took each week of the current season.
    pub weeks: [bool; 4],
    pub seasons_won: u32,
    /// Season reward items granted. Four when a season is swept.
    pub season_rewards: u32,
    /// Weeks the company won outright.
    pub tournaments_won: u32,
}

/// Copper one shop line is worth, before a sale quarters the basket.
pub fn line_price(word: u16) -> u32 {
    let count = u32::from(word & LINE_COUNT);
    let unit = match word & LINE_UNIT {
        UNIT_TEN_GOLD => 10 * COIN * COIN,
        UNIT_GOLD => COIN * COIN,
        UNIT_SILVER => COIN,
        _ => 1,
    };
    count * unit
}

/// Copper in a basket. A sale is the sum divided by 4, dropping the remainder.
pub fn basket_total(lines: &[u16], sell: bool) -> u32 {
    let sum = lines
        .iter()
        .fold(0u32, |sum, line| sum.wrapping_add(line_price(*line)));
    if sell { sum >> 2 } else { sum }
}

impl Company {
    /// A new career: the name, 343 copper, and an empty roster.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            copper: STARTING_COPPER,
            roster: Vec::new(),
            won: 0,
            lost: 0,
            spent: 0,
            earned: 0,
            day: 0,
            weeks: [false; 4],
            seasons_won: 0,
            season_rewards: 0,
            tournaments_won: 0,
        }
    }

    /// Confirm a shop basket. Empty baskets are refused. The copper still
    /// to pay is the buy total minus the sell total; a larger sell is received.
    pub fn trade(&mut self, buy: u32, sell: u32) -> bool {
        if buy == 0 && sell == 0 {
            return false;
        }
        let due = buy.saturating_sub(sell);
        if self.copper < due {
            return false;
        }
        if buy > sell {
            self.copper -= due;
            self.spent = self.spent.saturating_add(due);
        } else {
            let got = sell - buy;
            self.copper = self.copper.saturating_add(got);
            self.earned = self.earned.saturating_add(got);
        }
        true
    }

    /// Confirm the two baskets. A buy is the sum of the line prices. A sale
    /// is that sum divided by 4, dropping the remainder.
    pub fn trade_lines(&mut self, buy: &[u16], sell: &[u16]) -> bool {
        self.trade(basket_total(buy, false), basket_total(sell, true))
    }

    /// Gold, silver, then copper, at 12 to 1.
    pub fn coins(&self) -> (u32, u32, u32) {
        let gold = self.copper / (COIN * COIN);
        let rest = self.copper % (COIN * COIN);
        (gold, rest / COIN, rest % COIN)
    }

    /// Hire onto the roster for 12 copper. A full roster or a second
    /// specialist is refused without spending. Fewer than 12 copper is
    /// refused and the roster stays as it is.
    pub fn hire(
        &mut self,
        name: impl Into<String>,
        profession: Profession,
    ) -> Result<(), HireError> {
        let combatants = self
            .roster
            .iter()
            .filter(|member| member.profession == Profession::Combatant)
            .count();
        if profession == Profession::Combatant && combatants >= MAX_COMBATANTS {
            return Err(HireError::Full);
        }
        if profession != Profession::Combatant
            && self
                .roster
                .iter()
                .any(|member| member.profession == profession)
        {
            return Err(HireError::Duplicate);
        }
        if self.copper < HIRE_PRICE {
            return Err(HireError::Funds);
        }
        self.copper -= HIRE_PRICE;
        self.spent += HIRE_PRICE;
        self.roster.push(Member {
            name: name.into(),
            profession,
            xp: 0,
            skills: [0; 6],
            skill_xp: [0; 6],
            techniques: [[0; 6]; 6],
            health: u16::MAX,
            in_entry: false,
        });
        Ok(())
    }

    /// Daily wage. A merchant is 2, a physician 12, a trainer the sum of
    /// six skill levels, and a combatant half that sum, at most 5, plus 1.
    pub fn wage(profession: Profession, skills: &[u32]) -> u32 {
        let rank: u32 = skills.iter().take(6).sum();
        match profession {
            Profession::Merchant => MERCHANT_WAGE,
            Profession::Physician => PHYSICIAN_WAGE,
            Profession::Trainer => rank,
            Profession::Combatant => (rank / 2).min(5) + 1,
        }
    }

    /// The week inside the season, 0 to 3, for a day counter that has
    /// already moved on.
    pub fn season_week(day: u32) -> u32 {
        (day % (SEASON_WEEKS * 7)) / 7
    }

    /// End the day. Rest every employee, pay every wage, then on a day
    /// whose number mod 7 is 1 resolve the six event winners, and on a
    /// multiple of 7 drop an entry whose health is still under 3276.
    /// Winning all four weeks grants four rewards.
    pub fn end_day(&mut self, events: [u32; 6]) -> Night {
        self.day = self.day.saturating_add(1);
        let physician = self
            .roster
            .iter()
            .any(|member| member.profession == Profession::Physician);
        for member in &mut self.roster {
            member.health = rested(member.health, physician);
        }
        if let Some(index) = self
            .roster
            .iter()
            .position(|member| member.profession == Profession::Trainer)
        {
            let trainer = self.roster[index].clone();
            for member in &mut self.roster {
                if member.profession == Profession::Combatant {
                    teach(member, &trainer);
                }
            }
        }
        let wages = self
            .roster
            .iter()
            .map(|member| Self::wage(member.profession, &member.skills))
            .sum();
        self.copper = self.copper.saturating_sub(wages);
        self.spent = self.spent.saturating_add(wages);
        let mut night = Night {
            wages,
            week_won: None,
            dropped: 0,
            season: false,
        };
        if self.day % 7 == 1 {
            let (won, season) = self.resolve_week(&events);
            night.week_won = Some(won);
            night.season = season;
        } else if self.day.is_multiple_of(7) {
            for member in &mut self.roster {
                if member.in_entry && member.health < INJURED {
                    member.in_entry = false;
                    night.dropped += 1;
                }
            }
        }
        night
    }

    /// Who took the week, and whether the season was swept.
    fn resolve_week(&mut self, events: &[u32; 6]) -> (bool, bool) {
        let (leader, count, tie) = tally(events);
        let week = Self::season_week(self.day) as usize;
        let ours = leader == OUR_COMPANY && count >= WEEK_WINS && !tie;
        if count >= WEEK_WINS && !tie {
            self.weeks[week] = ours;
            if ours {
                self.tournaments_won = self.tournaments_won.saturating_add(1);
            }
        }
        let mut season = false;
        if Self::season_week(self.day) == SEASON_WEEKS - 1 {
            if ours && self.weeks.iter().all(|won| *won) {
                self.seasons_won = self.seasons_won.saturating_add(1);
                self.season_rewards = self.season_rewards.saturating_add(4);
                season = true;
            }
            self.weeks = [false; 4];
        }
        (ours, season)
    }

    /// Take the entry fee and remember the prize and the experience.
    /// Tier 0 charges the recorded copper.
    pub fn enter(&mut self, board: BoardMatch) -> Result<Ongoing, EnterError> {
        self.enter_at(board, 0)
    }

    /// Take the fee for `tier`. The fee and the prize are the recorded
    /// copper times one more than the tier.
    pub fn enter_at(&mut self, board: BoardMatch, tier: u32) -> Result<Ongoing, EnterError> {
        if !self
            .roster
            .iter()
            .any(|member| member.profession == Profession::Combatant)
        {
            return Err(EnterError::NoFighter);
        }
        let fee = scaled_stake(board.fee, tier);
        if self.copper < fee {
            return Err(EnterError::Fee);
        }
        self.copper -= fee;
        self.spent += fee;
        Ok(Ongoing {
            prize: scaled_stake(board.prize, tier),
            xp: board.xp,
        })
    }

    /// Pay the prize on a win, and credit tier-0 experience to the first combatant.
    pub fn finish(&mut self, ongoing: Ongoing, won: bool, damage_ratio: f32) {
        self.finish_shared(ongoing, won, damage_ratio, 0, &[]);
    }

    /// Pay the prize on a win. Experience is the template times
    /// `1 + tier / 4` times the damage share, plus half when a trainer is
    /// hired, then split across combatants by `shares`. An empty share list
    /// credits the whole pool to the first combatant.
    pub fn finish_shared(
        &mut self,
        ongoing: Ongoing,
        won: bool,
        damage_ratio: f32,
        tier: u32,
        shares: &[u32],
    ) {
        if won {
            self.copper = self.copper.saturating_add(ongoing.prize);
            self.earned = self.earned.saturating_add(ongoing.prize);
            self.won += 1;
        } else {
            self.lost += 1;
        }
        let ratio = if damage_ratio.is_finite() {
            damage_ratio.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let scaled = ongoing.xp as f32 * xp_scale(tier) * ratio;
        let mut xp = if scaled.is_finite() {
            scaled.round() as u32
        } else {
            0
        };
        if self
            .roster
            .iter()
            .any(|member| member.profession == Profession::Trainer)
        {
            xp = xp.saturating_add(xp / 2);
        }
        let fighters: Vec<usize> = self
            .roster
            .iter()
            .enumerate()
            .filter(|(_, member)| member.profession == Profession::Combatant)
            .map(|(index, _)| index)
            .collect();
        if shares.is_empty() {
            if let Some(&index) = fighters.first() {
                self.roster[index].xp = self.roster[index].xp.saturating_add(xp);
            }
            return;
        }
        let mut padded = shares
            .iter()
            .copied()
            .take(fighters.len())
            .collect::<Vec<_>>();
        padded.resize(fighters.len(), 0);
        for (index, part) in fighters.into_iter().zip(split_experience(xp, &padded)) {
            self.roster[index].xp = self.roster[index].xp.saturating_add(part);
        }
    }
}

/// What ending the day changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Night {
    pub wages: u32,
    /// Set when this day resolves a week: whether the company took it.
    pub week_won: Option<bool>,
    /// Entries dropped because their health is under [`INJURED`].
    pub dropped: usize,
    /// This night granted the four season rewards.
    pub season: bool,
}

/// Health after one night. The step is 6553, or 9830 with a physician,
/// and the pool stops at 65535.
fn skill_level(word: u16) -> u32 {
    u32::from(word) / u32::from(SKILL_STEP)
}

/// One lesson. The first skill under level 5 whose technique at that level
/// the trainer knows gains 1000 divided by one plus the six levels. A sum
/// that reaches a new multiple of 5000 snaps to it.
fn teach(student: &mut Member, trainer: &Member) {
    let Some(skill) = (0..student.skill_xp.len()).find(|&index| {
        let level = skill_level(student.skill_xp[index]);
        level < TRAINED_LEVEL && student.techniques[index][level as usize] != 0
    }) else {
        return;
    };
    let level = skill_level(student.skill_xp[skill]);
    let technique = student.techniques[skill][level as usize];
    let known = skill_level(trainer.skill_xp[skill]) as usize;
    if known == 0 || !trainer.techniques[skill][..known.min(6)].contains(&technique) {
        return;
    }
    let rank = 1 + student
        .skill_xp
        .iter()
        .copied()
        .map(skill_level)
        .sum::<u32>();
    if rank > LESSON {
        return;
    }
    let grant = (LESSON / rank).min(u32::from(SKILL_STEP));
    let before = skill_level(student.skill_xp[skill]);
    let summed = student.skill_xp[skill].wrapping_add(grant as u16);
    let after = skill_level(summed);
    student.skill_xp[skill] = if before < after {
        (after * u32::from(SKILL_STEP)) as u16
    } else {
        summed
    };
}

fn rested(health: u16, physician: bool) -> u16 {
    let step = if physician { REST_WITH_PHYSICIAN } else { REST };
    let sum = u32::from(health) + u32::from(step);
    if sum < u32::from(FULL_HEALTH) {
        sum as u16
    } else {
        FULL_HEALTH
    }
}

fn tally(events: &[u32; 6]) -> (u32, u32, bool) {
    let mut ids = Vec::new();
    let mut counts = Vec::new();
    for &id in events {
        if let Some(index) = ids.iter().position(|known| *known == id) {
            counts[index] += 1;
        } else {
            ids.push(id);
            counts.push(1u32);
        }
    }
    let mut leader = 0;
    let mut tie = false;
    for index in 1..ids.len() {
        if counts[leader] < counts[index] {
            leader = index;
            tie = false;
        } else if counts[index] == counts[leader] {
            tie = true;
        }
    }
    (ids[leader], counts[leader], tie)
}

/// A match the fee has already paid for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ongoing {
    pub prize: u32,
    pub xp: u32,
}

/// Why a hire did not happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HireError {
    /// Eight fighters, counting the manager.
    Full,
    /// A merchant, physician or trainer is already hired.
    Duplicate,
    /// Fewer than 12 copper.
    Funds,
}

/// Why a match did not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterError {
    /// The fee is more than the company holds.
    Fee,
    /// No combatant is on the roster.
    NoFighter,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_company_hires_a_roster_and_a_duel_pays_on_a_win() {
        let mut company = Company::new("Ash");
        assert_eq!(company.copper, 343);
        assert_eq!(company.coins(), (2, 4, 7));
        assert_eq!(standard_board()[0].name, "Duel");
        assert!(company.enter(standard_board()[0]).is_err());
        for n in 0..MAX_COMBATANTS {
            company
                .hire(format!("f{n}"), Profession::Combatant)
                .unwrap();
        }
        assert_eq!(
            company.hire("extra", Profession::Combatant),
            Err(HireError::Full)
        );
        company.hire("med", Profession::Physician).unwrap();
        assert_eq!(
            company.hire("med2", Profession::Physician),
            Err(HireError::Duplicate)
        );
        let duel = standard_board()[0];
        let ongoing = company.enter(duel).unwrap();
        assert_eq!(company.copper, 244);
        assert_eq!(company.spent, 99);
        company.finish(ongoing, false, 1.0);
        assert_eq!(company.copper, 244);
        assert_eq!(company.lost, 1);
        assert_eq!(company.roster[0].xp, 1000);
        let ongoing = company.enter(duel).unwrap();
        company.hire("coach", Profession::Trainer).unwrap();
        company.finish(ongoing, true, 0.5);
        assert_eq!(company.copper, 241);
        assert_eq!(company.won, 1);
        assert_eq!(company.earned, 12);
        assert_eq!(company.roster[0].xp, 1000 + 750);
    }

    #[test]
    fn the_shop_confirms_only_what_the_company_can_pay() {
        let mut company = Company::new("Ash");
        assert!(!company.trade(0, 0));
        assert_eq!(company.copper, 343);
        assert!(!company.trade(400, 0));
        assert_eq!(company.copper, 343);
        assert!(company.trade(40, 0));
        assert_eq!(company.copper, 303);
        assert_eq!(company.spent, 40);
        assert!(company.trade(10, 25));
        assert_eq!(company.copper, 318);
        assert_eq!(company.earned, 15);
        assert!(company.trade(5, 5));
        assert_eq!(company.copper, 318);
    }

    #[test]
    fn a_shop_line_prices_by_its_marks() {
        assert_eq!(basket_total(&[7], false), 7);
        assert_eq!(basket_total(&[7], true), 1);
        assert_eq!(basket_total(&[UNIT_SILVER | 2], false), 24);
        assert_eq!(basket_total(&[UNIT_GOLD | 2], false), 288);
        assert_eq!(basket_total(&[UNIT_TEN_GOLD | 2], false), 2880);
        assert_eq!(basket_total(&[3, 3], true), 1);
        assert_eq!(basket_total(&[u16::MAX], false), 23_591_520);

        let mut company = Company::new("Ash");
        assert!(company.trade_lines(&[40], &[]));
        assert_eq!(company.copper, 303);
        assert_eq!(company.spent, 40);
        assert!(company.trade_lines(&[10], &[100]));
        assert_eq!(company.copper, 318);
        assert_eq!(company.earned, 15);
    }

    #[test]
    fn a_day_pays_wages_and_four_weeks_make_a_season() {
        assert_eq!(Company::wage(Profession::Merchant, &[9, 9, 9, 9, 9, 9]), 2);
        assert_eq!(Company::wage(Profession::Physician, &[]), 12);
        assert_eq!(Company::wage(Profession::Trainer, &[3, 1, 0, 0, 0, 0]), 4);
        assert_eq!(Company::wage(Profession::Combatant, &[0; 6]), 1);
        assert_eq!(
            Company::wage(Profession::Combatant, &[12, 0, 0, 0, 0, 0]),
            6
        );

        let mut tied = Company::new("Ash");
        tied.hire("a", Profession::Combatant).unwrap();
        let night = tied.end_day([1, 1, 1, 2, 2, 2]);
        assert_eq!(night.week_won, Some(false));
        assert_eq!(tied.tournaments_won, 0);

        let mut company = Company::new("Ash");
        company.hire("a", Profession::Combatant).unwrap();
        company.roster[0].in_entry = true;
        company.roster[0].health = INJURED - 1;
        let sweep = [OUR_COMPANY; 6];
        let first = company.end_day(sweep);
        assert_eq!(company.day, 1);
        assert_eq!(first.wages, 1);
        assert_eq!(company.copper, 330);
        assert_eq!(first.week_won, Some(true));
        assert!(!first.season);

        for _ in 0..5 {
            company.end_day([0; 6]);
        }
        assert_eq!(company.day, 6);
        let dropped = company.end_day([0; 6]);
        assert_eq!(company.day, 7);
        assert_eq!(dropped.dropped, 0);
        assert!(company.roster[0].in_entry);
        assert_eq!(company.roster[0].health, 49_146);

        let mut plain = Company::new("Ash");
        plain.hire("a", Profession::Combatant).unwrap();
        plain.roster[0].health = 1000;
        plain.end_day([0; 6]);
        assert_eq!(plain.roster[0].health, 7_553);
        plain.roster[0].health = 60_000;
        plain.end_day([0; 6]);
        assert_eq!(plain.roster[0].health, 65_535);
        plain.roster[0].health = 58_981;
        plain.end_day([0; 6]);
        assert_eq!(plain.roster[0].health, 65_534);
        plain.roster[0].health = 58_982;
        plain.end_day([0; 6]);
        assert_eq!(plain.roster[0].health, 65_535);

        let mut tended = Company::new("Ash");
        tended.hire("a", Profession::Combatant).unwrap();
        tended.hire("med", Profession::Physician).unwrap();
        tended.roster[0].health = 1000;
        tended.end_day([0; 6]);
        assert_eq!(tended.roster[0].health, 10_830);
        assert_eq!(tended.roster[1].health, u16::MAX);

        let mut closing = Night {
            wages: 0,
            week_won: None,
            dropped: 0,
            season: false,
        };
        for _ in 0..15 {
            closing = company.end_day(sweep);
        }
        assert_eq!(company.day, 22);
        assert_eq!(closing.week_won, Some(true));
        assert!(closing.season);
        assert_eq!(company.seasons_won, 1);
        assert_eq!(company.season_rewards, 4);
        assert_eq!(company.weeks, [false; 4]);
        assert_eq!(company.copper, 343 - 12 - 22);
    }

    #[test]
    fn tier_scales_the_pool_and_the_fighters_share_it() {
        let mut company = Company::new("Ash");
        company.hire("a", Profession::Combatant).unwrap();
        company.hire("b", Profession::Combatant).unwrap();
        let duel = standard_board()[0];
        let ongoing = company.enter(duel).unwrap();
        company.finish_shared(ongoing, true, 1.0, 0, &[1, 3]);
        assert_eq!(company.roster[0].xp, 250);
        assert_eq!(company.roster[1].xp, 750);

        let ongoing = company.enter(duel).unwrap();
        company.hire("coach", Profession::Trainer).unwrap();
        company.finish_shared(ongoing, true, 1.0, 4, &[1, 1]);
        assert_eq!(company.roster[0].xp, 250 + 1500);
        assert_eq!(company.roster[1].xp, 750 + 1500);
        assert_eq!(split_experience(5, &[1, 1, 1]), vec![2, 2, 1]);

        let specials = special_board();
        assert_eq!(
            (
                specials[0].name,
                specials[0].fee,
                specials[0].prize,
                specials[0].per_side,
                specials[0].xp
            ),
            ("Challenger", 12, 36, 1, 1000)
        );
        assert_eq!(
            (
                specials[6].name,
                specials[6].fee,
                specials[6].prize,
                specials[6].per_side,
                specials[6].xp
            ),
            ("The Beast", 24, 432, 3, 10_000)
        );
    }

    #[test]
    fn a_tier_doubles_the_stake() {
        assert_eq!(coin_total(0x100), 12);
        assert_eq!(coin_total(0x300), 36);
        assert_eq!(coin_total(0x200), 24);
        assert_eq!(coin_total(0x0003_0000), 432);
        assert_eq!(standard_board()[0].prize, coin_total(0x100));
        assert_eq!(special_board()[0].fee, coin_total(0x100));
        assert_eq!(special_board()[0].prize, coin_total(0x300));
        assert_eq!(special_board()[6].fee, coin_total(0x200));
        assert_eq!(special_board()[6].prize, coin_total(0x0003_0000));

        let mut company = Company::new("Ash");
        company.hire("a", Profession::Combatant).unwrap();
        let fee = scaled_stake(special_board()[0].fee, 1);
        let prize = scaled_stake(special_board()[0].prize, 1);
        assert_eq!(fee, 24);
        assert_eq!(prize, 72);
        let ongoing = company.enter_at(special_board()[0], 1).unwrap();
        assert_eq!(company.copper, 343 - 12 - fee);
        assert_eq!(company.spent, 12 + fee);
        company.finish(ongoing, true, 0.0);
        assert_eq!(company.copper, 379);
        assert_eq!(company.earned, prize);
    }

    #[test]
    fn a_trainer_teaches_a_known_technique() {
        let mut alone = Company::new("Ash");
        alone.hire("a", Profession::Combatant).unwrap();
        alone.roster[0].techniques[0][0] = 7;
        alone.end_day([0; 6]);
        assert_eq!(alone.roster[0].skill_xp[0], 0);

        let mut taught = Company::new("Ash");
        taught.hire("a", Profession::Combatant).unwrap();
        taught.hire("coach", Profession::Trainer).unwrap();
        taught.roster[0].techniques[0][0] = 7;
        taught.roster[1].skill_xp[0] = 5_000;
        taught.roster[1].techniques[0][0] = 7;
        taught.end_day([0; 6]);
        assert_eq!(taught.roster[0].skill_xp[0], 1_000);
        assert_eq!(taught.roster[1].skill_xp[0], 5_000);

        let mut other = Company::new("Ash");
        other.hire("a", Profession::Combatant).unwrap();
        other.hire("coach", Profession::Trainer).unwrap();
        other.roster[0].techniques[0][0] = 7;
        other.roster[1].skill_xp[0] = 5_000;
        other.roster[1].techniques[0][0] = 3;
        other.end_day([0; 6]);
        assert_eq!(other.roster[0].skill_xp[0], 0);

        let mut shop = Company::new("Ash");
        shop.hire("sell", Profession::Merchant).unwrap();
        shop.hire("coach", Profession::Trainer).unwrap();
        shop.roster[0].techniques[0][0] = 7;
        shop.roster[1].skill_xp[0] = 5_000;
        shop.roster[1].techniques[0][0] = 7;
        shop.end_day([0; 6]);
        assert_eq!(shop.roster[0].skill_xp[0], 0);

        let mut medic = Company::new("Ash");
        medic.hire("med", Profession::Physician).unwrap();
        medic.hire("coach", Profession::Trainer).unwrap();
        medic.roster[0].techniques[0][0] = 7;
        medic.roster[1].skill_xp[0] = 5_000;
        medic.roster[1].techniques[0][0] = 7;
        medic.end_day([0; 6]);
        assert_eq!(medic.roster[0].skill_xp[0], 0);

        let mut rising = Company::new("Ash");
        rising.hire("a", Profession::Combatant).unwrap();
        rising.hire("coach", Profession::Trainer).unwrap();
        rising.roster[0].skill_xp[0] = 4_500;
        rising.roster[0].techniques[0][0] = 7;
        rising.roster[1].skill_xp[0] = 5_000;
        rising.roster[1].techniques[0][0] = 7;
        rising.end_day([0; 6]);
        assert_eq!(rising.roster[0].skill_xp[0], 5_000);

        let mut ranked = Company::new("Ash");
        ranked.hire("a", Profession::Combatant).unwrap();
        ranked.hire("coach", Profession::Trainer).unwrap();
        ranked.roster[0].skill_xp = [0, 5_000, 5_000, 5_000, 5_000, 5_000];
        ranked.roster[0].techniques[0][0] = 4;
        ranked.roster[1].skill_xp[0] = 5_000;
        ranked.roster[1].techniques[0][0] = 4;
        ranked.end_day([0; 6]);
        assert_eq!(ranked.roster[0].skill_xp[0], 166);

        let mut later = Company::new("Ash");
        later.hire("a", Profession::Combatant).unwrap();
        later.hire("coach", Profession::Trainer).unwrap();
        later.roster[0].skill_xp[0] = 25_000;
        later.roster[0].techniques[0][0] = 7;
        later.roster[0].techniques[1][0] = 9;
        later.roster[1].skill_xp[1] = 5_000;
        later.roster[1].techniques[1][0] = 9;
        later.end_day([0; 6]);
        assert_eq!(later.roster[0].skill_xp[0], 25_000);
        assert_eq!(later.roster[0].skill_xp[1], 166);
    }

    #[test]
    fn a_hire_costs_twelve_and_a_short_purse_stays_empty() {
        let mut company = Company::new("Ash");
        company.copper = 11;
        assert_eq!(
            company.hire("a", Profession::Combatant),
            Err(HireError::Funds)
        );
        assert!(company.roster.is_empty());
        assert_eq!(company.copper, 11);
        assert_eq!(company.spent, 0);

        company.copper = 12;
        company.hire("a", Profession::Combatant).unwrap();
        assert_eq!(company.roster.len(), 1);
        assert_eq!(company.copper, 0);
        assert_eq!(company.spent, 12);

        company.copper = 12;
        company.hire("med", Profession::Physician).unwrap();
        assert_eq!(
            company.hire("med2", Profession::Physician),
            Err(HireError::Duplicate)
        );
        assert_eq!(company.copper, 0);
        assert_eq!(company.spent, 24);
    }
}
