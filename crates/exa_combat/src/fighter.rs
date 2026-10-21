//! A fighter and its health bars: what a character that can fight and be
//! hurt keeps, as the original keeps it on every character. Its condition
//! word, which says whether it stands, attacks, lies, collapses or is
//! dead, is [`crate::condition`]'s.

use bevy::prelude::*;
use exa_formats::character_stats::{BodyRules, CharacterStats};
use exa_formats::characters::{Perk, Perks};
use exa_formats::races::{Natural, Race};

use crate::attack::Attacker;
use crate::condition::{Condition, PostureBranch, StateFlags};
use crate::damage::{self, Guard, Outcome, PartArmour};
use crate::fall::FallDeath;
use crate::parry::ParryState;
use crate::swing::SwingState;
use crate::wounds::WoundQueue;
use exa_physics::part::WoundTally;

/// A character that can fight and be hurt (on the body model root). It
/// keeps the original's condition word and state flags beside it, the
/// posture stage's branch and its fall.
#[derive(Component, Debug, Clone)]
#[require(
    WoundQueue,
    Attacker,
    SwingState,
    Condition,
    StateFlags,
    PostureBranch,
    FallDeath
)]
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
    /// Its record's perks (Remise chains a held swing, Feint renews a
    /// released one).
    pub perks: Perks,
    /// How it meets a blow: open, a parry, or a shield block.
    pub guard: Guard,
    /// The parry in progress: whether it parries and the motion it
    /// counters, which decides the clash's branch.
    pub parry: ParryState,
    /// The bleeding the struck limbs added up since the body's last tick
    /// (`+0x10d4`); a boned limb's wound adds its size, and the body tick
    /// turns the tally into damage over time.
    pub bleeding: WoundTally,
    /// The armour that grants a second wind, and when it may fire again.
    pub second_wind: SecondWind,
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
            perks: Perks::default(),
            guard: Guard::Open,
            parry: ParryState::default(),
            bleeding: WoundTally::default(),
            second_wind: SecondWind::default(),
        }
    }

    /// The same fighter with its record's `perks`.
    pub fn with_perks(self, perks: Perks) -> Self {
        Self { perks, ..self }
    }

    /// It has Remise: holding a side attack through 80 % of its clip
    /// chains the other side's.
    pub fn remise(&self) -> bool {
        self.perks.has(Perk::Remise)
    }

    /// It has Feint: a released swing may be renewed.
    pub fn feint(&self) -> bool {
        self.perks.has(Perk::Feint)
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

/// Vital damage that leaves yellow health below this (compared in double
/// precision, [`failing`]) kills.
pub const DEATH: f64 = 0.01;
/// Damage that leaves yellow health below this without killing runs the
/// low-health check ([`Weakened`]).
pub const LOW_HEALTH: f32 = 0.05;
/// The shares of the damage taken from the bars: a tenth of the yellow
/// amount (a twentieth with a potion) and a fifth of the red (in double
/// precision); in the story a killing blow takes a tenth of the maximum
/// from red too.
const YELLOW_SHARE: f64 = 0.1;
const POTION_YELLOW_SHARE: f64 = 0.05;
const RED_SHARE: f64 = 0.2;
const STORY_RED_LOSS: f64 = 0.1;

/// Whether `yellow` health is below [`DEATH`], as the original tests it
/// (in double precision): a fighter alive with it takes the Defence stance
/// in combat mode and never parries.
pub fn failing(yellow: f32) -> bool {
    f64::from(yellow) < DEATH
}

/// `value` less `amount` × `share`, in double precision.
fn less(value: f32, amount: f32, share: f64) -> f32 {
    (f64::from(value) - f64::from(amount) * share) as f32
}

/// Damage to a character's health bars, as the original's damage call
/// takes it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Damage {
    /// Yellow and red amounts, before their shares.
    pub yellow: f32,
    pub red: f32,
    /// Vital damage kills when it leaves yellow under [`DEATH`]; any other
    /// leaves the character alive at yellow 0. A weapon blow is vital; an
    /// impact only once its accumulated strength passes 0.6.
    pub vital: bool,
}

impl Damage {
    /// A weapon blow's damage: always vital.
    pub fn blow(outcome: &Outcome) -> Self {
        Self {
            yellow: outcome.yellow,
            red: outcome.red,
            vital: true,
        }
    }
}

/// Which way taking damage ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taken {
    /// Yellow health is still [`LOW_HEALTH`] or more: nothing else happens.
    Hurt,
    /// Yellow health is under [`LOW_HEALTH`] and the character lives: the
    /// low-health check runs ([`Weakened`]).
    Weakened,
    /// Vital damage left yellow under [`DEATH`]: yellow is 0 and the
    /// character's collapse starts.
    Killed,
}

/// The armour that grants a second wind, and the clock before which it
/// will not fire again.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SecondWind {
    pub granted: bool,
    /// Milliseconds of game time. A wind whose clock is still at or after
    /// this waits.
    pub ready_at: u32,
    /// Set when the wind fires.
    pub active: bool,
}

/// How long a second wind waits before it can fire again.
pub const SECOND_WIND_MS: u32 = 180_000;
/// The heal sound's volume, the float 0.1.
pub const HEAL_VOLUME_BITS: u32 = 0x3dcc_cccd;
/// The heal sound's pitch, the float 1.
pub const HEAL_PITCH_BITS: u32 = 0x3f80_0000;

/// What the low-health check does when the fighter lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowAnswer {
    /// The armour's wind fires. The heartbeat does not.
    SecondWind { volume_bits: u32, pitch_bits: u32 },
    /// No wind. The caller may start the heartbeat.
    Quiet,
}

/// How long an active second wind keeps adding yellow, measured from the
/// clock that fired it: the ready clock minus 180000 ms, plus this.
pub const SECOND_WIND_HEAL_MS: u32 = 3_000;
/// Yellow gained a tick while a second wind is active, the double 0.0003.
pub const SECOND_WIND_GAIN_BITS: u64 = 0x3f33_a92a_3055_3261;

/// The low-health check after a blow that leaves the fighter alive. A
/// granted wind fires when the clock is past `ready_at`, waits 180000 ms,
/// and plays the heal sound at volume 0.1 and pitch 1. At the ready clock,
/// and without the armour, it does not fire.
pub fn answer_low_health(now: u32, wind: &mut SecondWind) -> LowAnswer {
    if !wind.granted || now <= wind.ready_at {
        LowAnswer::Quiet
    } else {
        wind.active = true;
        wind.ready_at = now.wrapping_add(SECOND_WIND_MS);
        LowAnswer::SecondWind {
            volume_bits: HEAL_VOLUME_BITS,
            pitch_bits: HEAL_PITCH_BITS,
        }
    }
}

/// One tick of an active second wind. Yellow gains the double 0.0003 and
/// is stored as a float. The wind ends once the game clock is past the
/// ready clock minus 180000 ms plus 3000 ms, and the tick that ends it
/// still gains. A quiet wind leaves yellow as it is.
pub fn heal_second_wind(yellow: f32, now: u32, wind: &mut SecondWind) -> f32 {
    if !wind.active {
        return yellow;
    }
    let yellow = (f64::from(yellow) + f64::from_bits(SECOND_WIND_GAIN_BITS)) as f32;
    let end = u64::from(wind.ready_at)
        .wrapping_sub(u64::from(SECOND_WIND_MS))
        .wrapping_add(u64::from(SECOND_WIND_HEAL_MS));
    if (end as i64) < i64::from(now) {
        wind.active = false;
    }
    yellow
}

/// A second wind fired instead of the heartbeat.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecondWindFired {
    pub fighter: Entity,
    pub volume_bits: u32,
    pub pitch_bits: u32,
}

/// Damage left a fighter's yellow health under 0.05 without killing it:
/// the original's low-health check, which gives a second wind to one
/// wearing the armour that grants it (not yet) and, else, starts the
/// player's heartbeat (`exa_game::hud`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weakened {
    pub fighter: Entity,
}

/// A blow's lasting wound was deep enough to be noticed: the weighed red
/// is greater than a twentieth of the 0.25 health base.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageTaken {
    pub fighter: Entity,
}

/// The game plays the story, not practice or the arena: there a killing
/// blow also takes a tenth of the maximum from red health. Absent: not the
/// story.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StoryMode(pub bool);

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

    /// Takes `damage` as the original's damage call does, on a body with
    /// `rules` and state `flags`, `story` when the game plays the story. In
    /// order: the regeneration rate starts its ramp again; yellow loses a
    /// tenth of the yellow amount (a twentieth with a potion); with no
    /// lasting wounds (rule bit 0x8) yellow also loses a fifth of the red
    /// amount and red is untouched, else red loses it and yellow is scaled
    /// by the share of red left (0 when none is). Under [`LOW_HEALTH`] the
    /// damage weakens, unless it is vital and leaves yellow under
    /// [`DEATH`]: then yellow is 0, in the story red loses a tenth of the
    /// maximum, and it kills. Non-vital damage under [`DEATH`] leaves
    /// yellow 0 and weakens. The subtractions are in double precision.
    pub fn take(
        &mut self,
        damage: Damage,
        rules: BodyRules,
        flags: StateFlags,
        story: bool,
    ) -> Taken {
        self.rate = 0.0;
        let share = if flags.has(StateFlags::POTION) {
            POTION_YELLOW_SHARE
        } else {
            YELLOW_SHARE
        };
        self.yellow = less(self.yellow, damage.yellow, share);
        if rules.has(BodyRules::NO_LASTING_WOUNDS) {
            self.yellow = less(self.yellow, damage.red, RED_SHARE);
        } else {
            let before = self.red;
            self.red = less(self.red, damage.red, RED_SHARE);
            self.yellow = if self.red > 0.0 {
                self.red / before * self.yellow
            } else {
                0.0
            };
        }
        if self.yellow >= LOW_HEALTH {
            return Taken::Hurt;
        }
        if !failing(self.yellow) {
            return Taken::Weakened;
        }
        self.yellow = 0.0;
        if !damage.vital {
            return Taken::Weakened;
        }
        if story {
            self.red = less(self.red, self.max, STORY_RED_LOSS);
        }
        Taken::Killed
    }
}

#[cfg(test)]
#[path = "fighter_test.rs"]
mod tests;
