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
//! A swept season grants four item ids, and day 134 repeats the first four.
//! A candidate's listed price is the square of one more than half of six
//! skill levels, that half stopping at 5, times 12, plus four fifths of the
//! gear. A flat recruitment stays at 12 copper. An unwatched match scores
//! each counted team as `(count × 0.5 + 0.5) × strength sum`, leaves an
//! aside fighter out, and in mode 16 leaves the player's team out. The roll
//! steps by `× 0x08088405 + 1` and the high half of that word times one more
//! than the weight total picks the first team whose weights reach it. A
//! bracket clears every losing id. The first group keeps a forced id and
//! does not step the roll. A later group keeps the first team whose rounded
//! weights pass the high half of the weight total times the stepped roll. A
//! tournament field draws 16 of the 19 companies, numbered 1 through 19. A
//! day chooses six of seven match templates. A template already at its
//! limit, a slot before its earliest slot, or a mask that shares a bit
//! with the roll's top three bits is refused. A performance counter
//! replaces that roll: its low 32 bits are byte-reversed. A copied match
//! then multiplies its fee and prize by one more than the slot. That unit
//! is a tempered word from a 624-word generator. A record copy writes the
//! gaps, the plain fields, and the tail.
//!
//! Evidence: the arena career notes (money seed, coin rate, roster caps,
//! the six standard matches and the seven specials, fee on entry, prize on
//! a win, the experience pool `(tier/4 + 1) · template · damage share`).

use exa_formats::characters::Skill;

use crate::experience;
use crate::trade::{self, ShopWorld, StockReport, Transaction};

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
/// Four item ids for each of four seasons. Day 134 grants the first row again.
pub const SEASON_REWARDS: [[u32; 4]; 4] = [
    [1175, 1174, 1177, 1176],
    [1167, 1165, 1168, 1169],
    [1171, 1170, 1173, 1172],
    [1178, 1179, 1180, 1181],
];
/// Health below this drops a fighter from a tournament entry.
pub const INJURED: u16 = 0x0ccc;
/// A specialist under this health dies before a match.
pub const DIED_BELOW: u16 = 700;
/// A specialist under this health, and not under the death line, leaves.
pub const LEFT_BELOW: u16 = 65_000;
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

/// Why the first unhealthy specialist left before a match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Departure {
    /// Health was under 700. The death count rose by one.
    Died(String),
    /// Health was from 700 through 64999. The death count stayed.
    Left(String),
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
    /// One arena kit per rank.
    pub loadouts: [crate::loadout::CharacterLoadout; crate::loadout::RANKS],
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
    /// Item ids granted by swept seasons, four per season, in order.
    pub reward_ids: Vec<u32>,
    /// Weeks the company won outright.
    pub tournaments_won: u32,
    /// Specialists who died before a match. The session save does not store this.
    pub deaths: u32,
    /// The shop's shelf, oldest first. The session save does not store this.
    pub stock: Vec<u32>,
    /// The company's emblem.
    pub emblem: exa_ui::EmblemRecord,
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

/// The hire fee for a fighter of `rank` (0..=5) carrying gear worth
/// `gear_total` copper: one more than the rank, squared, times 12, plus four
/// fifths of the gear rounded to the nearest copper (the fifths never land on
/// a half, so the integer form matches the original's double product).
pub fn hire_fee(rank: u32, gear_total: u32) -> u32 {
    let step = rank + 1;
    let share = ((u64::from(gear_total) * 4 + 2) / 5) as u32;
    step * step * COIN + share
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
            reward_ids: Vec::new(),
            tournaments_won: 0,
            deaths: 0,
            stock: Vec::new(),
            emblem: exa_ui::EmblemRecord::default(),
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

    /// Rotate the shop's shelf (`trade::update_stock`): `kinds` 0 asks
    /// apparel, weapons and shields, `rank` −1 any rank.
    pub fn restock(
        &mut self,
        kinds: u32,
        rank: i32,
        rolls: [u32; 2],
        world: &mut impl ShopWorld,
    ) -> StockReport {
        trade::update_stock(&mut self.stock, kinds, rank, rolls, world)
    }

    /// Fill the shop's shelf on its first stock, as the hub's shop click
    /// does when the shelf is empty. A shelf that already has items is left
    /// alone and no report is returned.
    pub fn first_shop_stock(
        &mut self,
        rolls: [u32; 2],
        world: &mut impl ShopWorld,
    ) -> Option<StockReport> {
        trade::ensure_first_stock(&mut self.stock, rolls, world)
    }

    /// Apply a confirmed trade (`trade::settle`) through coin dwords: the
    /// copper due leaves the purse, none left below 1, and the two tallies
    /// add to the copper spent and earned [guess: which tally is which].
    /// The items sold join the shelf and the items bought leave it.
    pub fn settle(&mut self, trade: &Transaction) -> Vec<(usize, usize)> {
        let pack = |copper: u32| trade::add_coins(0, copper as i32);
        let mut money = pack(self.copper);
        let mut tallies = [pack(self.spent), pack(self.earned)];
        let duplicates = trade::settle(&mut money, &mut tallies, &mut self.stock, trade);
        self.copper = trade::coins_value(money) as u32;
        self.spent = trade::coins_value(tallies[0]) as u32;
        self.earned = trade::coins_value(tallies[1]) as u32;
        duplicates
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
            loadouts: Default::default(),
            health: u16::MAX,
            in_entry: false,
        });
        Ok(())
    }

    /// Copper a hire-list candidate costs. Half of six skill levels stops at
    /// 5. One more than that is squared and multiplied by 12. Four fifths of
    /// the gear, the sum of the shop line prices, is added to the nearest copper.
    pub fn listed_price(skills: &[u32], gear: &[u16]) -> u32 {
        let sum = skills
            .iter()
            .take(6)
            .fold(0u32, |sum, level| sum.saturating_add(*level));
        let rank = (sum / 2).min(5);
        let gear_total = gear
            .iter()
            .fold(0u32, |sum, word| sum.wrapping_add(line_price(*word)));
        hire_fee(rank, gear_total)
    }

    /// Hire a listed candidate. A short purse is refused before the roster
    /// caps. A full roster or a second specialist does not spend.
    pub fn hire_listed(
        &mut self,
        name: impl Into<String>,
        profession: Profession,
        skills: &[u32],
        gear: &[u16],
    ) -> Result<(), HireError> {
        let price = Self::listed_price(skills, gear);
        if self.copper < price {
            return Err(HireError::Funds);
        }
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
        self.copper -= price;
        self.spent += price;
        self.roster.push(Member {
            name: name.into(),
            profession,
            xp: 0,
            skills: [0; 6],
            skill_xp: [0; 6],
            techniques: [[0; 6]; 6],
            loadouts: Default::default(),
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

    /// Which of the four reward rows a day that has already moved on uses.
    /// Day 22 is row 0. Day 134 is row 0 again.
    pub fn season_index(day: u32) -> u32 {
        (day / (SEASON_WEEKS * 7)) % SEASON_WEEKS
    }

    /// Before a match, the first specialist who is not well enough leaves.
    /// A combatant stays, even at no health. Health under 700 counts a death.
    /// Health from 700 through 64999 leaves without that count. Health of
    /// 65000 or more stays. A later specialist stays until the next pass.
    /// Releasing a kit and unlinking a world character are not part of this step.
    pub fn leave_before_match(&mut self) -> Option<Departure> {
        let index = self.roster.iter().position(|member| {
            member.profession != Profession::Combatant && member.health < LEFT_BELOW
        })?;
        let member = self.roster.remove(index);
        if member.health < DIED_BELOW {
            self.deaths = self.deaths.wrapping_add(1);
            Some(Departure::Died(member.name))
        } else {
            Some(Departure::Left(member.name))
        }
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
                let row = SEASON_REWARDS[Self::season_index(self.day) as usize];
                self.reward_ids.extend(row);
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
/// the trainer knows gains 1000 divided by one plus the six levels, the
/// story's skill lump (`experience::add_skill_lump`). A sum that reaches a
/// new multiple of 5000 snaps to it.
fn teach(student: &mut Member, trainer: &Member) {
    let mut skills = std::array::from_fn(|index| Skill {
        points: student.skill_xp[index],
        techniques: student.techniques[index],
    });
    let Some((skill, next)) = experience::training_skill(&skills) else {
        return;
    };
    let technique = student.techniques[skill][next as usize - 1];
    let known = skill_level(trainer.skill_xp[skill]) as usize;
    if known == 0 || !trainer.techniques[skill][..known.min(6)].contains(&technique) {
        return;
    }
    experience::add_skill_lump(&mut skills, LESSON as i32);
    student.skill_xp = skills.map(|s| s.points);
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

/// Mode that leaves the player's team out of an unwatched draw.
pub const FORFEIT_DRAW: i32 = 0x10;

/// One fighter an unwatched match may count.
#[derive(Clone, Copy, Debug)]
pub struct SideFighter {
    pub team: i32,
    pub strength: f32,
    /// Left out. The original does this when mark bit `0x800` is set.
    pub aside: bool,
}

struct SideScore {
    team: i32,
    count: i32,
    sum: f32,
    score: f32,
}

/// Draw the winning team of a match nobody is watching.
///
/// Fighters are grouped in first-seen order. A team scores
/// `(count × 0.5 + 0.5) × strength sum`. The largest score starts at 0 and
/// moves only when a score is greater. Each weight is the square of score
/// over that largest, times 100, rounded to the nearest even integer. The
/// roll steps by `roll × 0x08088405 + 1`. The high half of that word times
/// one more than the weight total picks the first team whose running
/// weights are not below it. No counted fighter still steps the roll and
/// returns no team.
pub fn draw_unwatched(
    fighters: &[SideFighter],
    player_team: i32,
    mode: i32,
    roll: &mut u32,
) -> Option<i32> {
    let mut sides: Vec<SideScore> = Vec::new();
    let mut largest = 0.0f32;
    for fighter in fighters {
        if fighter.aside || (fighter.team == player_team && mode == FORFEIT_DRAW) {
            continue;
        }
        let found = sides.iter().position(|side| side.team == fighter.team);
        let index = match found {
            Some(index) => index,
            None => {
                sides.push(SideScore {
                    team: fighter.team,
                    count: 0,
                    sum: 0.0,
                    score: 0.0,
                });
                sides.len() - 1
            }
        };
        let side = &mut sides[index];
        side.count += 1;
        side.sum += fighter.strength;
        let count = side.count as f32;
        side.score = (count * 0.5 + 0.5) * side.sum;
        if largest < side.score {
            largest = side.score;
        }
    }
    let mut total = 0i32;
    let mut weights = Vec::with_capacity(sides.len());
    if !sides.is_empty() {
        for side in &mut sides {
            let ratio = side.score / largest;
            let weight = ratio * ratio * 100.0;
            side.score = weight;
            let rounded = weight.round_ties_even() as i32;
            weights.push(rounded);
            total = total.wrapping_add(rounded);
        }
    }
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    if sides.is_empty() {
        return None;
    }
    let span = i64::from(total.wrapping_add(1));
    let product = i64::from(*roll).wrapping_mul(span);
    let pick = ((product as u64) >> 32) as i32;
    let mut reached = 0i32;
    let mut winner = 0usize;
    for (index, weight) in weights.iter().enumerate() {
        reached = reached.wrapping_add(*weight);
        if reached < pick {
            continue;
        }
        winner = index;
        break;
    }
    Some(sides[winner].team)
}

/// One slot in a bracket group.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BracketSlot {
    pub id: i32,
    pub strength: f32,
}

struct BracketTeam {
    id: i32,
    members: Vec<usize>,
    score: f32,
}

/// Clear every losing record in a bracket.
///
/// Slots are taken in groups of `teams_in_group`, each team `per_team`
/// slots wide. An id of exactly 0 is skipped. An id above 0 adds its
/// strength. The team scores `(count × 0.5 + 0.5) × strength sum`. The
/// first group keeps `forced` when that id is above 0, and does not step
/// the roll. Any other group steps the roll by `× 0x08088405 + 1` and keeps
/// the first team whose rounded weights are strictly above the high half of
/// the weight total times that word. Every other counted id becomes 0. The
/// result is whether more than one group was resolved.
pub fn clear_losing_bracket(
    slots: &mut [BracketSlot],
    per_team: i32,
    teams_in_group: i32,
    forced: i32,
    roll: &mut u32,
) -> bool {
    if per_team <= 0 || teams_in_group <= 0 {
        return false;
    }
    let len = slots.len() as i32;
    let mut cursor = 0i32;
    let mut groups = 0i32;
    while cursor < len {
        if slots[cursor as usize].id == 0 {
            cursor += 1;
            continue;
        }
        let mut teams: Vec<BracketTeam> = Vec::new();
        let mut largest = 0.0f32;
        while cursor < len && (teams.len() as i32) < teams_in_group {
            if slots[cursor as usize].id == 0 {
                cursor += 1;
                continue;
            }
            let team_id = slots[cursor as usize].id;
            let base = (cursor / per_team) * per_team;
            let mut members = Vec::new();
            let mut sum = 0.0f32;
            for offset in 0..per_team {
                let index = base + offset;
                if !(0..len).contains(&index) {
                    continue;
                }
                let slot = slots[index as usize];
                if slot.id > 0 {
                    members.push(index as usize);
                    sum += slot.strength;
                }
            }
            cursor = base + per_team;
            let count = members.len() as f32;
            let score = (count * 0.5 + 0.5) * sum;
            if largest < score {
                largest = score;
            }
            teams.push(BracketTeam {
                id: team_id,
                members,
                score,
            });
            if cursor >= len {
                break;
            }
        }
        if teams.is_empty() {
            break;
        }
        groups += 1;
        let winner = if forced > 0 && groups == 1 {
            teams.iter().position(|team| team.id == forced)
        } else {
            let mut total = 0i32;
            let mut weights = Vec::with_capacity(teams.len());
            for team in &teams {
                let ratio = team.score / largest;
                let weight = ratio * ratio * 100.0;
                let rounded = weight.round_ties_even() as i32;
                weights.push(rounded);
                total = total.wrapping_add(rounded);
            }
            *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
            let product = i64::from(total).wrapping_mul(i64::from(*roll));
            let pick = ((product as u64) >> 32) as i32;
            let mut reached = 0i32;
            let mut found = None;
            for (index, weight) in weights.iter().enumerate() {
                reached = reached.wrapping_add(*weight);
                if reached <= pick {
                    continue;
                }
                found = Some(index);
                break;
            }
            found
        };
        for (index, team) in teams.iter().enumerate() {
            if Some(index) == winner {
                continue;
            }
            for &member in &team.members {
                slots[member].id = 0;
            }
        }
    }
    groups > 1
}

/// How many rival companies exist. A field draws [`RIVAL_FIELD`] of them.
pub const RIVAL_COMPANIES: usize = 19;
/// Rival companies named for one tournament.
pub const RIVAL_FIELD: usize = 16;

/// Draw 16 of the 19 rival companies.
///
/// The bag starts as 0 through 18. Each draw steps the roll by
/// `× 0x08088405 + 1`. The high half of how many remain times that word
/// chooses a slot. The stored id is one more than the value there. The
/// remaining count shrinks first. When the new count minus 1 is still at
/// least the chosen slot, each later value copies down one.
pub fn draw_rival_field(roll: &mut u32) -> [i32; RIVAL_FIELD] {
    let mut bag = [0i32; RIVAL_COMPANIES];
    for (index, slot) in bag.iter_mut().enumerate() {
        *slot = index as i32;
    }
    let mut drawn = [0i32; RIVAL_FIELD];
    let mut remaining = RIVAL_COMPANIES as i32;
    for slot in &mut drawn {
        *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
        let pick = ((i64::from(remaining).wrapping_mul(i64::from(*roll)) as u64) >> 32) as i32;
        *slot = bag[pick as usize] + 1;
        remaining -= 1;
        let limit = remaining - 1;
        if limit >= pick {
            for index in pick..=limit {
                let index = index as usize;
                bag[index] = bag[index + 1];
            }
        }
    }
    drawn
}

/// How soon a daily template may appear, how many times, and which top
/// roll bits refuse it. The seven rows are earliest slot, limit, mask.
const DAILY_GATES: [(i32, i32, i32); 7] = [
    (0, 2, 0),
    (0, 2, 0),
    (1, 2, 0),
    (2, 2, 0),
    (2, 2, 0),
    (0, 1, 3),
    (2, 1, 0),
];

/// Choose the seven-template index for each of the six daily matches.
///
/// The roll is the value already stored for the day. Each try steps it by
/// `× 0x08088405 + 1` and takes the high half of that word times 7. A limit
/// above 0 that has already been reached refuses the template. So does a
/// slot before the earliest slot. A second step refuses it when the mask
/// shares a bit with the top three bits of the roll (`roll >> 29`).
pub fn choose_daily_matches(roll: &mut u32) -> [i32; 6] {
    let mut times = [0i32; 7];
    let mut chosen = [0i32; 6];
    for (slot, choice) in chosen.iter_mut().enumerate() {
        loop {
            *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
            let pick = ((u64::from(*roll).wrapping_mul(7)) >> 32) as usize;
            let (earliest, limit, mask) = DAILY_GATES[pick];
            if limit > 0 && limit <= times[pick] {
                continue;
            }
            if (slot as i32) < earliest {
                continue;
            }
            *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
            let top = (*roll >> 29) as i32;
            if top & mask > 0 {
                continue;
            }
            *choice = pick as i32;
            times[pick] += 1;
            break;
        }
    }
    chosen
}

/// The roll a performance counter stores. Only the low 32 bits are kept,
/// and their bytes are reversed.
pub fn roll_from_counter(counter: u64) -> u32 {
    (counter as u32).swap_bytes()
}

/// Replace `roll` from a performance counter, then choose the six templates.
pub fn choose_daily_matches_from_counter(counter: u64, roll: &mut u32) -> [i32; 6] {
    *roll = roll_from_counter(counter);
    choose_daily_matches(roll)
}

/// A match template the day copies and then scales.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DailyMatch {
    pub kind: i32,
    pub flags: u32,
    pub shown_slot: i32,
    pub shown_tier: i32,
    pub slot: i32,
    pub tier: i32,
    pub fee: u32,
    pub prize: u32,
    pub extra: i32,
}

/// How many faces the extra draw has, one count for each daily slot.
const SLOT_SPANS: [i32; 6] = [8, 11, 10, 14, 4, 4];

fn pack_coins(total: u32) -> u32 {
    let gold = (total / 144) as u16;
    let rest = total.wrapping_sub(u32::from(gold).wrapping_mul(144));
    let silver = ((rest / 12) as u8) as u32;
    let copper = rest.wrapping_sub((rest / 12).wrapping_mul(12)) as u8;
    u32::from(copper) | (silver << 8) | (u32::from(gold) << 16)
}

fn scale_coins(packed: u32, factor: u32) -> u32 {
    let total = (coin_total(packed) as i32).wrapping_mul(factor as i32) as u32;
    pack_coins(total)
}

/// Copy `template` onto the day's match and scale it for `slot`.
///
/// The fee and the prize are multiplied by one more than the slot. Bit 2
/// keeps the slot as the tier. Otherwise the tier is the slot times one
/// minus the fourth power of `unit`, rounded to the nearest even integer.
/// A negative tier becomes -1. Kind 8 shows 0 and 5. Bit 4 steps the roll
/// and stores one more than the high half of that word times the slot's
/// span.
pub fn place_daily_match(
    template: &DailyMatch,
    slot: u32,
    unit: f64,
    roll: &mut u32,
) -> DailyMatch {
    let mut placed = *template;
    placed.slot = slot as i32;
    if placed.tier < 0 {
        placed.tier = -1;
    } else if placed.flags & 2 == 0 {
        let curved = 1.0 - unit * unit * unit * unit;
        let scaled = f64::from(slot) * curved;
        placed.tier = scaled.round_ties_even() as i32;
    } else {
        placed.tier = placed.slot;
    }
    if placed.kind == 8 {
        placed.shown_slot = 0;
        placed.shown_tier = 5;
    } else {
        placed.shown_slot = placed.slot;
        placed.shown_tier = placed.tier;
    }
    if placed.flags & 4 != 0 {
        *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
        let span = SLOT_SPANS.get(slot as usize).copied().unwrap_or(0);
        let pick = ((i64::from(*roll).wrapping_mul(i64::from(span)) as u64) >> 32) as i32;
        placed.extra = pick + 1;
    }
    let factor = slot.wrapping_add(1);
    placed.fee = scale_coins(placed.fee, factor);
    placed.prize = scale_coins(placed.prize, factor);
    placed
}

/// Words in the unit generator, and the index that means they must be mixed.
const UNIT_WORDS: usize = 624;
const UNIT_TWIST: u32 = 0x270;
/// The stored scale, 2 to the power -32.
const UNIT_SCALE: f64 = f64::from_bits(0x3df0_0000_0000_0000);

/// A 624-word unit generator.
#[derive(Clone, Debug)]
pub struct UnitGen {
    pub state: [u32; UNIT_WORDS],
    pub index: u32,
    pub mark: u32,
    pub stamp: u32,
}

impl UnitGen {
    /// Fill the words from `seed`. The mark and the stamp match, and the
    /// index is 624, so the first draw mixes the words.
    pub fn seed(seed: u32) -> Self {
        let mut generator = Self {
            state: [0; UNIT_WORDS],
            index: UNIT_TWIST,
            mark: 0,
            stamp: 0,
        };
        generator.fill(seed);
        generator
    }

    fn fill(&mut self, seed: u32) {
        self.state[0] = seed;
        for index in 1..UNIT_WORDS {
            let prev = self.state[index - 1];
            let mixed = prev ^ (prev >> 30);
            self.state[index] = (index as u32).wrapping_add(mixed.wrapping_mul(0x6c07_8965));
        }
        self.index = UNIT_TWIST;
    }

    fn twist(&mut self) {
        for index in 0..UNIT_WORDS {
            let next = self.state[(index + 1) % UNIT_WORDS];
            let ahead = self.state[(index + 397) % UNIT_WORDS];
            let mixed = (self.state[index] & 0x8000_0000) | (next & 0x7fff_ffff);
            let mut shifted = mixed >> 1;
            if next & 1 != 0 {
                shifted ^= 0x9908_b0df;
            }
            self.state[index] = ahead ^ shifted;
        }
    }

    /// The next tempered word. Index 624 mixes the words first. A mark that
    /// differs from the stamp seeds from the mark and reverses it.
    pub fn next_word(&mut self) -> u32 {
        let mut saved = self.index;
        self.index = self.index.wrapping_add(1);
        if self.mark != self.stamp || saved > UNIT_TWIST {
            self.fill(self.mark);
            self.mark = !self.mark;
            self.stamp = self.mark;
            saved = UNIT_TWIST;
        }
        if saved == UNIT_TWIST {
            self.twist();
            saved = 0;
            self.index = 1;
        }
        let mut word = self.state[saved as usize];
        word ^= word >> 11;
        word ^= (word << 7) & 0x9d2c_5680;
        word ^= (word << 15) & 0xefc6_0000;
        word ^= word >> 18;
        word
    }

    /// The next word times 2 to the power -32.
    pub fn next_unit(&mut self) -> f64 {
        f64::from(self.next_word()) * UNIT_SCALE
    }
}

/// Why a record copy stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordCopyError {
    /// The source or the destination is shorter than the record.
    Short,
    /// A field starts before the bytes already copied, or runs past the size.
    Field,
}

/// One plain field: where it sits, and how many bytes it owns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlainField {
    pub offset: usize,
    pub width: usize,
}

/// Copy a record's gaps, plain fields, and tail.
///
/// Each field is raw bytes. The bytes before it are copied first, then the
/// field, and after the last field the tail through `size`. A short buffer
/// or a field out of order or past `size` is an error.
pub fn copy_record(
    source: &[u8],
    dest: &mut [u8],
    size: usize,
    fields: &[PlainField],
) -> Result<(), RecordCopyError> {
    if source.len() < size || dest.len() < size {
        return Err(RecordCopyError::Short);
    }
    let mut copied = 0usize;
    for field in fields {
        let end = field.offset.saturating_add(field.width);
        if field.offset < copied || end > size {
            return Err(RecordCopyError::Field);
        }
        if copied < field.offset {
            dest[copied..field.offset].copy_from_slice(&source[copied..field.offset]);
        }
        dest[field.offset..end].copy_from_slice(&source[field.offset..end]);
        copied = end;
    }
    if copied < size {
        dest[copied..size].copy_from_slice(&source[copied..size]);
    }
    Ok(())
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

    /// 343 copper paying 100 leaves 243; the tallies add 100 spent and 0
    /// earned. The sold item joins the shelf and the bought one leaves it.
    /// A due above the purse empties it.
    #[test]
    fn a_settled_trade_moves_the_purse_and_the_shelf() {
        let mut company = Company::new("Ash");
        company.stock = vec![5, 6];
        let trade = Transaction {
            due: 100,
            tallies: [100, 0],
            sold: vec![7],
            bought: vec![5],
        };
        assert!(company.settle(&trade).is_empty());
        assert_eq!(
            (company.copper, company.spent, company.earned),
            (243, 100, 0)
        );
        assert_eq!(company.stock, [6, 7]);
        let broke = Transaction {
            due: 500,
            ..Transaction::default()
        };
        company.settle(&broke);
        assert_eq!(company.copper, 0);
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
        assert_eq!(company.reward_ids.as_slice(), &SEASON_REWARDS[0]);
        assert_eq!(company.weeks, [false; 4]);
        assert_eq!(company.copper, 343 - 12 - 22);
    }

    #[test]
    fn a_swept_season_grants_its_four_items() {
        assert_eq!(SEASON_REWARDS[0], [1175, 1174, 1177, 1176]);
        assert_eq!(SEASON_REWARDS[1], [1167, 1165, 1168, 1169]);
        assert_eq!(SEASON_REWARDS[2], [1171, 1170, 1173, 1172]);
        assert_eq!(SEASON_REWARDS[3], [1178, 1179, 1180, 1181]);

        let mut missed = Company::new("Ash");
        for _ in 0..22 {
            missed.end_day([2; 6]);
        }
        assert!(missed.reward_ids.is_empty());
        assert_eq!(missed.season_rewards, 0);

        let mut company = Company::new("Ash");
        let sweep = [OUR_COMPANY; 6];
        for _ in 0..22 {
            company.end_day(sweep);
        }
        assert_eq!(company.day, 22);
        assert_eq!(Company::season_index(company.day), 0);
        assert_eq!(company.reward_ids.as_slice(), &SEASON_REWARDS[0]);

        for row in 1..5 {
            for _ in 0..28 {
                company.end_day(sweep);
            }
            let start = row * 4;
            let expected = SEASON_REWARDS[row % 4];
            assert_eq!(company.reward_ids[start..start + 4], expected);
            assert_eq!(
                Company::season_index(company.day),
                (row as u32) % SEASON_WEEKS
            );
        }
        assert_eq!(company.day, 134);
        assert_eq!(company.season_rewards, 20);
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

    #[test]
    fn a_listed_price_uses_rank_and_gear() {
        assert_eq!(Company::listed_price(&[0; 6], &[]), 12);
        assert_eq!(Company::listed_price(&[2, 0, 0, 0, 0, 0], &[]), 48);
        assert_eq!(Company::listed_price(&[10, 0, 0, 0, 0, 0], &[]), 432);
        assert_eq!(Company::listed_price(&[12, 0, 0, 0, 0, 0], &[]), 432);
        assert_eq!(Company::listed_price(&[0; 6], &[7]), 18);
        assert_eq!(Company::listed_price(&[0; 6], &[UNIT_SILVER | 2]), 31);
        assert_eq!(Company::listed_price(&[0, 0, 0, 0, 0, 0, 100], &[]), 12);

        let mut poor = Company::new("Ash");
        poor.copper = 47;
        assert_eq!(
            poor.hire_listed("a", Profession::Combatant, &[2, 0, 0, 0, 0, 0], &[]),
            Err(HireError::Funds)
        );
        assert!(poor.roster.is_empty());
        assert_eq!(poor.copper, 47);
        assert_eq!(poor.spent, 0);

        let mut full = Company::new("Ash");
        for n in 0..MAX_COMBATANTS {
            full.hire(format!("f{n}"), Profession::Combatant).unwrap();
        }
        let spent = full.spent;
        full.copper = 0;
        assert_eq!(
            full.hire_listed("extra", Profession::Combatant, &[0; 6], &[]),
            Err(HireError::Funds)
        );
        assert_eq!(full.roster.len(), MAX_COMBATANTS);
        assert_eq!(full.spent, spent);
        full.copper = 100;
        assert_eq!(
            full.hire_listed("extra", Profession::Combatant, &[0; 6], &[]),
            Err(HireError::Full)
        );
        assert_eq!(full.copper, 100);
        assert_eq!(full.spent, spent);

        let mut company = Company::new("Ash");
        company
            .hire_listed("a", Profession::Combatant, &[2, 0, 0, 0, 0, 0], &[])
            .unwrap();
        assert_eq!(company.copper, 295);
        assert_eq!(company.spent, 48);
        company
            .hire_listed("med", Profession::Physician, &[0; 6], &[])
            .unwrap();
        assert_eq!(
            company.hire_listed("med2", Profession::Physician, &[0; 6], &[]),
            Err(HireError::Duplicate)
        );
        assert_eq!(company.spent, 60);
    }

    #[test]
    fn an_unwatched_match_draws_a_team() {
        let sides = [
            SideFighter {
                team: 1,
                strength: 4.0,
                aside: false,
            },
            SideFighter {
                team: 2,
                strength: 2.0,
                aside: false,
            },
            SideFighter {
                team: 2,
                strength: 2.0,
                aside: false,
            },
            SideFighter {
                team: 3,
                strength: 10.0,
                aside: false,
            },
        ];
        let mut roll = 1u32;
        assert_eq!(draw_unwatched(&sides, 1, 0, &mut roll), Some(1));
        assert_eq!(roll, 0x0808_8406);
        roll = 4;
        assert_eq!(draw_unwatched(&sides, 1, 0, &mut roll), Some(2));
        assert_eq!(roll, 0x2022_1015);
        roll = 12;
        assert_eq!(draw_unwatched(&sides, 1, 0, &mut roll), Some(3));
        assert_eq!(roll, 0x6066_303d);
        roll = 1;
        assert_eq!(draw_unwatched(&sides, 1, FORFEIT_DRAW, &mut roll), Some(2));
        assert_eq!(roll, 0x0808_8406);
        let aside = [
            SideFighter {
                team: 1,
                strength: 4.0,
                aside: false,
            },
            SideFighter {
                team: 2,
                strength: 2.0,
                aside: false,
            },
            SideFighter {
                team: 2,
                strength: 2.0,
                aside: false,
            },
            SideFighter {
                team: 3,
                strength: 100.0,
                aside: true,
            },
        ];
        roll = 1;
        assert_eq!(draw_unwatched(&aside, 9, 0, &mut roll), Some(1));
        assert_eq!(roll, 0x0808_8406);
        roll = 1;
        assert_eq!(draw_unwatched(&[], 1, 0, &mut roll), None);
        assert_eq!(roll, 0x0808_8406);
    }

    #[test]
    fn a_bracket_clears_the_losing_records() {
        let mut paired = [
            BracketSlot {
                id: 5,
                strength: 4.0,
            },
            BracketSlot {
                id: 9,
                strength: 10.0,
            },
            BracketSlot {
                id: 3,
                strength: 4.0,
            },
            BracketSlot {
                id: 7,
                strength: 2.0,
            },
        ];
        let mut roll = 1u32;
        assert!(clear_losing_bracket(&mut paired, 1, 2, 5, &mut roll));
        assert_eq!(roll, 0x0808_8406);
        assert_eq!(paired.map(|slot| slot.id), [5, 0, 3, 0]);
        assert_eq!(paired[1].strength, 10.0);

        let mut one = [
            BracketSlot {
                id: 1,
                strength: 4.0,
            },
            BracketSlot {
                id: 9,
                strength: 10.0,
            },
        ];
        roll = 1;
        assert!(!clear_losing_bracket(&mut one, 1, 2, 0, &mut roll));
        assert_eq!(roll, 0x0808_8406);
        assert_eq!(one.map(|slot| slot.id), [1, 0]);
        assert_eq!(one[1].strength, 10.0);
        roll = 100;
        one[0].id = 1;
        one[1].id = 9;
        assert!(!clear_losing_bracket(&mut one, 1, 2, 0, &mut roll));
        assert_eq!(roll, 0x2353_91f5);
        assert_eq!(one.map(|slot| slot.id), [0, 9]);

        let mut wide = [
            BracketSlot {
                id: 1,
                strength: 2.0,
            },
            BracketSlot {
                id: 0,
                strength: 100.0,
            },
            BracketSlot {
                id: 3,
                strength: 2.0,
            },
            BracketSlot {
                id: 3,
                strength: 2.0,
            },
        ];
        roll = 4;
        assert!(!clear_losing_bracket(&mut wide, 2, 2, 0, &mut roll));
        assert_eq!(roll, 0x2022_1015);
        assert_eq!(wide.map(|slot| slot.id), [0, 0, 3, 3]);
        assert_eq!(wide[1].strength, 100.0);

        let mut forced = [
            BracketSlot {
                id: 5,
                strength: 4.0,
            },
            BracketSlot {
                id: 9,
                strength: 10.0,
            },
        ];
        roll = 4;
        assert!(!clear_losing_bracket(&mut forced, 1, 2, 5, &mut roll));
        assert_eq!(roll, 4);
        assert_eq!(forced.map(|slot| slot.id), [5, 0]);

        roll = 7;
        assert!(!clear_losing_bracket(&mut [], 1, 2, 0, &mut roll));
        assert_eq!(roll, 7);
    }

    #[test]
    fn a_tournament_draws_sixteen_rivals() {
        let mut roll = 0u32;
        let field = draw_rival_field(&mut roll);
        assert_eq!(
            field,
            [1, 2, 17, 6, 8, 14, 9, 4, 11, 12, 3, 13, 5, 19, 7, 15]
        );
        assert_eq!(roll, 0x4b15_7bf0);
        roll = 1;
        let field = draw_rival_field(&mut roll);
        assert_eq!(
            field,
            [1, 17, 5, 7, 14, 8, 4, 10, 11, 2, 13, 3, 18, 6, 12, 19]
        );
        assert_eq!(roll, 0xead3_2bb1);
    }

    #[test]
    fn a_day_chooses_six_matches() {
        let mut roll = 0u32;
        assert_eq!(choose_daily_matches(&mut roll), [0, 1, 4, 1, 2, 3]);
        assert_eq!(roll, 0x1210_d4e5);
        roll = 1;
        assert_eq!(choose_daily_matches(&mut roll), [0, 1, 4, 1, 2, 3]);
        assert_eq!(roll, 0x1210_d4e5);
        roll = 4;
        assert_eq!(choose_daily_matches(&mut roll), [0, 1, 6, 4, 4, 1]);
        assert_eq!(roll, 0xb761_a41e);
    }

    #[test]
    fn a_counter_replaces_the_roll_and_a_slot_scales_the_fee() {
        let mut roll = 99u32;
        assert_eq!(
            choose_daily_matches_from_counter(0x0400_0000, &mut roll),
            [0, 1, 6, 4, 4, 1]
        );
        assert_eq!(roll, 0xb761_a41e);
        roll = 99;
        assert_eq!(
            choose_daily_matches_from_counter(0, &mut roll),
            [0, 1, 4, 1, 2, 3]
        );
        assert_eq!(roll, 0x1210_d4e5);
        roll = 99;
        assert_eq!(
            choose_daily_matches_from_counter(0x1_0400_0000, &mut roll),
            [0, 1, 6, 4, 4, 1]
        );
        assert_eq!(roll, 0xb761_a41e);

        let template = DailyMatch {
            kind: 0,
            flags: 2,
            shown_slot: 0,
            shown_tier: 0,
            slot: 0,
            tier: 0,
            fee: 3,
            prize: 12,
            extra: 0,
        };
        roll = 7;
        let placed = place_daily_match(&template, 3, 0.0, &mut roll);
        assert_eq!(template.fee, 3);
        assert_eq!(template.prize, 12);
        assert_eq!(placed.fee, 0x100);
        assert_eq!(placed.prize, 0x400);
        assert_eq!(placed.tier, 3);
        assert_eq!((placed.shown_slot, placed.shown_tier), (3, 3));
        assert_eq!(roll, 7);

        let mut kind = template;
        kind.kind = 8;
        let placed = place_daily_match(&kind, 3, 0.0, &mut roll);
        assert_eq!(placed.tier, 3);
        assert_eq!((placed.shown_slot, placed.shown_tier), (0, 5));
        assert_eq!(placed.fee, 0x100);

        let mut negative = template;
        negative.tier = -4;
        let placed = place_daily_match(&negative, 2, 0.0, &mut roll);
        assert_eq!(placed.tier, -1);
        assert_eq!(placed.shown_tier, -1);

        let mut open = template;
        open.flags = 0;
        let placed = place_daily_match(&open, 4, 0.0, &mut roll);
        assert_eq!(placed.tier, 4);
        let placed = place_daily_match(&open, 4, 1.0, &mut roll);
        assert_eq!(placed.tier, 0);

        let mut marked = template;
        marked.flags = 4;
        roll = 0x1000_0000;
        let placed = place_daily_match(&marked, 0, 0.0, &mut roll);
        assert_eq!(placed.extra, 3);
        assert_eq!(roll, 0x5000_0001);
        assert_eq!(placed.fee, 3);
    }

    #[test]
    fn a_seeded_unit_scales_a_slot_and_a_record_copies_its_fields() {
        let mut generator = UnitGen::seed(5489);
        assert_eq!(generator.next_word(), 0xd091_bb5c);
        assert_eq!(generator.next_word(), 0x22ae_9ef6);
        assert_eq!(generator.index, 2);
        let mut generator = UnitGen::seed(5489);
        assert_eq!(generator.next_unit().to_bits(), 0x3fea_1237_6b80_0000);

        let mut other = UnitGen::seed(99);
        other.mark = 5489;
        other.stamp = 0;
        assert_eq!(other.next_word(), 0xd091_bb5c);
        assert_eq!(other.mark, !5489);
        assert_eq!(other.stamp, !5489);

        let mut scaled = UnitGen::seed(1);
        let unit = scaled.next_unit();
        assert_eq!(unit.to_bits(), 0x3fda_b07d_0940_0000);
        let open = DailyMatch {
            kind: 0,
            flags: 0,
            shown_slot: 0,
            shown_tier: 0,
            slot: 0,
            tier: 0,
            fee: 3,
            prize: 12,
            extra: 0,
        };
        let mut roll = 0u32;
        let placed = place_daily_match(&open, 4, unit, &mut roll);
        assert_eq!(placed.tier, 4);

        let source = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        let mut dest = [0u8; 8];
        let fields = [
            PlainField {
                offset: 1,
                width: 2,
            },
            PlainField {
                offset: 5,
                width: 1,
            },
        ];
        assert_eq!(copy_record(&source, &mut dest, 8, &fields), Ok(()));
        assert_eq!(dest, source);
        assert_eq!(source[0], 0x11);
        assert_eq!(
            copy_record(&source[..3], &mut dest, 8, &fields),
            Err(RecordCopyError::Short)
        );
        let backwards = [
            PlainField {
                offset: 1,
                width: 1,
            },
            PlainField {
                offset: 0,
                width: 1,
            },
        ];
        assert_eq!(
            copy_record(&source, &mut dest, 8, &backwards),
            Err(RecordCopyError::Field)
        );
    }

    #[test]
    fn a_specialist_leaves_before_a_match() {
        let mut empty = Company::new("Ash");
        assert_eq!(empty.leave_before_match(), None);
        assert_eq!(empty.deaths, 0);
        assert!(empty.roster.is_empty());

        let mut dying = Company::new("Ash");
        dying.hire("Med", Profession::Physician).unwrap();
        dying.roster[0].health = 699;
        assert_eq!(
            dying.leave_before_match(),
            Some(Departure::Died("Med".to_string()))
        );
        assert!(dying.roster.is_empty());
        assert_eq!(dying.deaths, 1);

        let mut leaving = Company::new("Ash");
        leaving.hire("Shop", Profession::Merchant).unwrap();
        leaving.roster[0].health = 700;
        assert_eq!(
            leaving.leave_before_match(),
            Some(Departure::Left("Shop".to_string()))
        );
        assert!(leaving.roster.is_empty());
        assert_eq!(leaving.deaths, 0);

        let mut fit = Company::new("Ash");
        fit.hire("Teach", Profession::Trainer).unwrap();
        fit.roster[0].health = 65_000;
        assert_eq!(fit.leave_before_match(), None);
        assert_eq!(fit.roster.len(), 1);
        assert_eq!(fit.roster[0].name, "Teach");
        assert_eq!(fit.deaths, 0);

        let mut skipped = Company::new("Ash");
        skipped.hire("Blade", Profession::Combatant).unwrap();
        skipped.roster[0].health = 0;
        skipped.hire("Med", Profession::Physician).unwrap();
        skipped.roster[1].health = 699;
        assert_eq!(
            skipped.leave_before_match(),
            Some(Departure::Died("Med".to_string()))
        );
        assert_eq!(skipped.roster.len(), 1);
        assert_eq!(skipped.roster[0].name, "Blade");
        assert_eq!(skipped.roster[0].health, 0);
        assert_eq!(skipped.deaths, 1);

        let mut pair = Company::new("Ash");
        pair.hire("Med", Profession::Physician).unwrap();
        pair.roster[0].health = 100;
        pair.hire("Shop", Profession::Merchant).unwrap();
        pair.roster[1].health = 100;
        assert_eq!(
            pair.leave_before_match(),
            Some(Departure::Died("Med".to_string()))
        );
        assert_eq!(pair.roster.len(), 1);
        assert_eq!(pair.roster[0].name, "Shop");
        assert_eq!(pair.roster[0].health, 100);
        assert_eq!(pair.deaths, 1);
    }
}
