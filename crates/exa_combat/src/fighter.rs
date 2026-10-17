//! A fighter and its health bars: what a character that can fight and be
//! hurt keeps, as the original keeps it on every character. Its condition
//! word, which says whether it stands, attacks, lies, collapses or is
//! dead, is [`crate::condition`]'s.

use bevy::prelude::*;
use exa_formats::character_stats::CharacterStats;
use exa_formats::races::{Natural, Race};

use crate::attack::Attacker;
use crate::condition::{Condition, StateFlags};
use crate::damage::{self, Outcome, PartArmour};
use crate::wounds::WoundQueue;

/// A character that can fight and be hurt (on the body model root). It
/// keeps the original's condition word and state flags beside it.
#[derive(Component, Debug, Clone)]
#[require(WoundQueue, Attacker, Condition, StateFlags)]
pub struct Fighter {
    pub health: Health,
    /// The race's natural armour, under worn items.
    pub natural: Option<Natural>,
    /// Armour layers per body part (bones 1..18), set when dressed.
    pub armour: [PartArmour; 18],
    /// Combat readiness, 0..=1: it rises standing in the combat idle and
    /// halves the wounds a blow deals at 1 ([`crate::stance::readiness_step`]).
    pub readiness: f32,
    /// Its race bleeds: its wounds paint blood (`races.rdb` flag bit 0).
    pub bleeds: bool,
    /// What its record and race gave it when it spawned (maximum health,
    /// scale, size, reach, rule bits, …).
    pub stats: CharacterStats,
}

impl Default for Fighter {
    /// A new record of no known race: a human's health (0.25), no natural
    /// armour, bleeding.
    fn default() -> Self {
        Self::of_stats(None, CharacterStats::default())
    }
}

impl Fighter {
    /// A fighter of `race` (none: the original's built-in race) with the
    /// stats its record spawned with: its health from them, its race's
    /// natural armour and whether it bleeds.
    pub fn of_stats(race: Option<&Race>, stats: CharacterStats) -> Self {
        let natural = race.map(|r| r.natural);
        Self {
            health: Health {
                yellow: stats.yellow,
                red: stats.red,
                max: stats.max_health,
                ..Health::default()
            },
            natural,
            armour: damage::armour_layers(natural.as_ref(), &[], stats.deflection),
            readiness: 0.0,
            bleeds: race.is_none_or(Race::bleeds),
            stats,
        }
    }
}

/// The health bars of a character, and what regenerates them
/// ([`crate::regen`]).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Health {
    /// Current (yellow) and lasting (red) health; both start at `max`.
    /// Yellow heals back towards red; red heals only for the races whose
    /// rule bits say so, or from the heal pool.
    pub yellow: f32,
    pub red: f32,
    pub max: f32,
    /// Yellow health regained a tick per unit of √red: it ramps up from 0
    /// to [`crate::regen::RATE_CAP`] and every blow sets it back to 0.
    pub rate: f32,
    /// Red health still to heal (bandages), moved to red a little a tick.
    /// Nothing fills it yet.
    pub heal_pool: f32,
}

/// Yellow health below this is death.
pub const DEATH: f32 = 0.01;

impl Health {
    /// Health of a race with health factor `factor` (human 1).
    pub fn of_race(factor: f32) -> Self {
        let max = 0.25 * factor;
        Self {
            yellow: max,
            red: max,
            max,
            ..Self::default()
        }
    }

    /// Takes a blow's damage; returns whether it kills. Its regeneration
    /// starts its ramp again.
    pub fn take(&mut self, outcome: &Outcome) -> bool {
        self.rate = 0.0;
        self.yellow -= 0.1 * outcome.yellow;
        let before = self.red;
        self.red -= 0.2 * outcome.red;
        if self.red > 0.0 && before > 0.0 {
            self.yellow *= self.red / before;
        } else {
            self.yellow = 0.0;
        }
        if self.yellow < DEATH {
            self.yellow = 0.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_and_death() {
        let mut h = Health::of_race(1.0);
        assert_eq!(h.max, 0.25);
        let light = Outcome {
            red: 0.1,
            yellow: 0.5,
            ..Outcome::default()
        };
        assert!(!h.take(&light));
        assert!(h.yellow < 0.25 && h.red < 0.25);
        let heavy = Outcome {
            red: 2.0,
            yellow: 5.0,
            ..Outcome::default()
        };
        assert!(h.take(&heavy));
        assert_eq!(h.yellow, 0.0);
    }

    /// Every fighter keeps a condition word, normal when it spawns, and
    /// empty state flags.
    #[test]
    fn a_fighter_keeps_a_condition() {
        let mut app = App::new();
        let id = app.world_mut().spawn(Fighter::default()).id();
        assert_eq!(app.world().get::<Condition>(id), Some(&Condition::NORMAL));
        assert_eq!(app.world().get::<StateFlags>(id), Some(&StateFlags::NONE));
    }
}
