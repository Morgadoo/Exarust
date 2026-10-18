//! Arena career: a company, its roster, and a match on the standard board.
//!
//! A new career holds 343 copper. Copper, silver and gold convert at 12 to 1.
//! The manager counts toward the eight fighters, so seven combatants fit, and
//! only one merchant, physician or trainer. Entering a match takes its fee.
//! A win pays the prize. Experience is the template amount times the share of
//! opponent health the company dealt, plus half again when a trainer is hired.
//! The hire price and the tier scaling of a fee were not read.
//!
//! Evidence: the arena career notes (money seed, coin rate, roster caps,
//! the six standard matches, fee on entry, prize on a win, the experience
//! pool at tier 0).

/// Copper in one silver, and silver in one gold.
pub const COIN: u32 = 12;
/// Copper a new career starts with.
pub const STARTING_COPPER: u32 = 343;
/// Combatants a company can hire. The manager holds the eighth place.
pub const MAX_COMBATANTS: usize = 7;

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

/// Duel, Doubles, Skirmish, Fray, Reserve, Pugilism.
pub fn standard_board() -> [BoardMatch; 6] {
    [
        BoardMatch {
            name: "Duel",
            fee: 3,
            prize: 256,
            per_side: 1,
            xp: 1000,
        },
        BoardMatch {
            name: "Doubles",
            fee: 6,
            prize: 1024,
            per_side: 2,
            xp: 2400,
        },
        BoardMatch {
            name: "Skirmish",
            fee: 9,
            prize: 1536,
            per_side: 3,
            xp: 3900,
        },
        BoardMatch {
            name: "Fray",
            fee: 3,
            prize: 512,
            per_side: 1,
            xp: 5000,
        },
        BoardMatch {
            name: "Reserve",
            fee: 3,
            prize: 1024,
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

    /// Gold, silver, then copper, at 12 to 1.
    pub fn coins(&self) -> (u32, u32, u32) {
        let gold = self.copper / (COIN * COIN);
        let rest = self.copper % (COIN * COIN);
        (gold, rest / COIN, rest % COIN)
    }

    /// Hire onto the roster. The price of a hire was not read.
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
        self.roster.push(Member {
            name: name.into(),
            profession,
            xp: 0,
        });
        Ok(())
    }

    /// Take the entry fee and remember the prize and the experience.
    pub fn enter(&mut self, board: BoardMatch) -> Result<Ongoing, EnterError> {
        if !self
            .roster
            .iter()
            .any(|member| member.profession == Profession::Combatant)
        {
            return Err(EnterError::NoFighter);
        }
        if self.copper < board.fee {
            return Err(EnterError::Fee);
        }
        self.copper -= board.fee;
        self.spent += board.fee;
        Ok(Ongoing {
            prize: board.prize,
            xp: board.xp,
        })
    }

    /// Pay the prize on a win, and credit experience from the damage dealt.
    pub fn finish(&mut self, ongoing: Ongoing, won: bool, damage_ratio: f32) {
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
        let mut xp = (ongoing.xp as f32 * ratio).round() as u32;
        if self
            .roster
            .iter()
            .any(|member| member.profession == Profession::Trainer)
        {
            xp = xp.saturating_add(xp / 2);
        }
        if let Some(fighter) = self
            .roster
            .iter_mut()
            .find(|member| member.profession == Profession::Combatant)
        {
            fighter.xp = fighter.xp.saturating_add(xp);
        }
    }
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
        assert_eq!(company.copper, 340);
        assert_eq!(company.spent, 3);
        company.finish(ongoing, false, 1.0);
        assert_eq!(company.copper, 340);
        assert_eq!(company.lost, 1);
        assert_eq!(company.roster[0].xp, 1000);
        let ongoing = company.enter(duel).unwrap();
        company.hire("coach", Profession::Trainer).unwrap();
        company.finish(ongoing, true, 0.5);
        assert_eq!(company.copper, 340 - 3 + 256);
        assert_eq!(company.won, 1);
        assert_eq!(company.earned, 256);
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
}
