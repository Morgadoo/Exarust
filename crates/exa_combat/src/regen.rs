//! Health regeneration, as the original's body tick keeps it on every
//! character.
//!
//! Yellow health heals back towards red. Each tick of a fighter not dead
//! (a collapsing one included) its regeneration rate rises by 0.0003 of its
//! cap, 0.01 a second over the 83 ticks, so it reaches the cap after
//! 40.2 s; yellow then gains √red × rate. The rate is held at 0 while the
//! fighter attacks (condition 2) or while Cmbt - Idle plays, so a fighter
//! in its fighting stance never heals, and every blow sets it back to 0
//! ([`crate::Health::take`], and each weapon contact in the hit reaction).
//! Red health heals only for the races whose rule bits give red
//! regeneration without the bit that blocks it (the ogre, Sir, the
//! harvester, the demon, a golem suit wearing its core), dead or alive,
//! and from the heal pool. Yellow stays within red and red within the
//! maximum. Rules: `docs/formats/OTHER.md` (Regeneration).

use bevy::prelude::*;
use exa_anim::{Animator, Motions, TickedClips};
use exa_formats::character_stats::BodyRules;
use exa_physics::body::{CharacterStage, CharacterTick, CharacterTicks, Ragdoll, TickedCharacter};

use crate::Fighter;
use crate::condition::{Condition, PostureTick, action, ticked_lives};
use crate::fighter::Health;

/// The regeneration rate's cap, yellow health per unit of √red a tick: 0.01
/// a second over the original's 83 ticks.
pub const RATE_CAP: f32 = 0.01 / 83.0;
/// The rate rises by this share of its cap a tick (in double precision): it
/// reaches the cap on the 3,334th tick, 40.2 s after it was last held.
pub const RAMP: f64 = 0.0003;
/// Red health a tick from red regeneration and from the heal pool, a
/// quarter of 5e-5 (in double precision).
pub const RED_PER_TICK: f64 = 0.25 * 5e-5;
/// Yellow regenerates only while red health is above this.
pub const RED_FLOOR: f32 = 1.19209e-7;

/// `value`, at most `cap`.
fn at_most(value: f32, cap: f32) -> f32 {
    if value > cap { cap } else { value }
}

impl Health {
    /// One tick of red regeneration, dead or alive, for a body whose
    /// `rules` let its red health regenerate: red rises by [`RED_PER_TICK`]
    /// up to the maximum.
    pub fn regenerate_red(&mut self, rules: BodyRules) {
        if rules.regenerates_red() {
            self.red = at_most((f64::from(self.red) + RED_PER_TICK) as f32, self.max);
        }
    }

    /// One tick of a living body's health bookkeeping, in the original's
    /// order: the heal pool moves [`RED_PER_TICK`] to red (red at most the
    /// maximum, the pool at least 0); the regeneration rate is 0 while
    /// `held` (attacking or in the combat idle), else it rises by
    /// [`RAMP`] × [`RATE_CAP`] to the cap and, above a red of [`RED_FLOOR`],
    /// yellow gains √red × rate; then yellow is kept within red, red within
    /// the maximum, and yellow at least 0.
    pub fn regenerate(&mut self, held: bool) {
        if self.heal_pool > 0.0 {
            self.red = at_most((f64::from(self.red) + RED_PER_TICK) as f32, self.max);
            self.heal_pool = ((f64::from(self.heal_pool) - RED_PER_TICK) as f32).max(0.0);
        }
        if held {
            self.rate = 0.0;
        } else {
            let ramp = f64::from(RATE_CAP) * RAMP;
            self.rate = at_most((f64::from(self.rate) + ramp) as f32, RATE_CAP);
            if self.red > RED_FLOOR {
                self.yellow += self.red.sqrt() * self.rate;
            }
        }
        self.yellow = at_most(self.yellow, self.red);
        self.red = at_most(self.red, self.max);
        self.yellow = self.yellow.max(0.0);
    }
}

/// Whether a fighter's yellow regeneration is held this tick: it attacks,
/// or `motion` (the action index of the clip it plays) is the combat idle,
/// whatever its condition says.
pub fn holds_regeneration(condition: Condition, motion: Option<usize>) -> bool {
    condition == Condition::ATTACKING || motion == Some(action::COMBAT_IDLE)
}

/// The health bookkeeping of the fighters the character tick does not
/// tick (the dead, and any without a living physical body), once a fixed
/// tick after the posture work and before the character ticks
/// ([`CharacterTicks`]). A ticked living body keeps its health in its own
/// tick: red regeneration in its upkeep stage, the rest in its attack
/// stage, after its posture.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HealthTick;

pub struct RegenerationPlugin;

impl Plugin for RegenerationPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            FixedUpdate,
            HealthTick.after(PostureTick).before(CharacterTicks),
        )
        .add_systems(FixedUpdate, regenerate_health.in_set(HealthTick))
        .add_systems(
            CharacterTick,
            (
                regenerate_red.in_set(CharacterStage::Upkeep),
                keep_health
                    .in_set(CharacterStage::Attack)
                    .run_if(ticked_lives),
            ),
        );
    }
}

/// Whether the character tick ticks a fighter's body this fixed tick (and
/// so keeps its health): a body with muscles whose clips its ticks advance
/// (`clips`: it has its clips and their ticked marker), once the motion set
/// is loaded (`motions`), as the character tick's driver picks them.
pub fn ticked_by_its_character(motions: bool, body: Option<&Ragdoll>, clips: bool) -> bool {
    motions && clips && body.is_some_and(|b| b.drive.is_some())
}

/// One tick of a fighter's yellow bookkeeping, logging when its rate
/// reaches its cap: the heal pool, the yellow regeneration, held while it
/// attacks or plays the combat idle (`motion`, the action index of its
/// clip), and the clamps.
fn keep(entity: Entity, health: &mut Health, condition: Condition, motion: Option<usize>) {
    let before = health.rate;
    health.regenerate(holds_regeneration(condition, motion));
    if health.rate == RATE_CAP && before < RATE_CAP {
        debug!(
            "{entity:?} regenerates at its full rate: yellow {:.4} of red {:.4}",
            health.yellow, health.red
        );
    }
}

/// The regeneration of every fighter the character tick does not tick,
/// once a fixed tick: red regeneration by its rule bits, dead or alive;
/// then, unless dead, the heal pool, the yellow regeneration and the
/// clamps.
#[allow(clippy::type_complexity)]
pub fn regenerate_health(
    motions: Option<Res<Motions>>,
    mut fighters: Query<(
        Entity,
        &mut Fighter,
        &Condition,
        Option<&Animator>,
        Option<&Ragdoll>,
        Has<TickedClips>,
    )>,
) {
    let set = motions.as_deref().map(|m| &*m.0);
    for (entity, mut fighter, condition, anim, body, clips) in &mut fighters {
        if ticked_by_its_character(set.is_some(), body, clips && anim.is_some()) {
            continue;
        }
        let mut health = fighter.health;
        health.regenerate_red(fighter.stats.rules);
        if !condition.is_dead() {
            let motion = anim.zip(set).and_then(|(a, s)| s.action_index(&a.action));
            keep(entity, &mut health, *condition, motion);
        }
        if health != fighter.health {
            fighter.health = health;
        }
    }
}

/// The ticked body's red regeneration, in its upkeep stage (before the
/// original's body tick leaves a dead body), by its rule bits.
pub fn regenerate_red(ticked: Res<TickedCharacter>, mut fighters: Query<&mut Fighter>) {
    let Ok(mut fighter) = fighters.get_mut(ticked.body) else {
        return;
    };
    let mut health = fighter.health;
    health.regenerate_red(fighter.stats.rules);
    if health != fighter.health {
        fighter.health = health;
    }
}

/// The ticked living body's health bookkeeping, in its attack stage after
/// its posture: the heal pool, the yellow regeneration (held while it
/// attacks or plays the combat idle) and the clamps.
pub fn keep_health(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<Motions>>,
    mut fighters: Query<(&mut Fighter, &Condition, Option<&Animator>)>,
) {
    let entity = ticked.body;
    let Ok((mut fighter, condition, anim)) = fighters.get_mut(entity) else {
        return;
    };
    if condition.is_dead() {
        return;
    }
    let set = motions.as_deref().map(|m| &*m.0);
    let motion = anim.zip(set).and_then(|(a, s)| s.action_index(&a.action));
    let mut health = fighter.health;
    keep(entity, &mut health, *condition, motion);
    if health != fighter.health {
        fighter.health = health;
    }
}

#[cfg(test)]
#[path = "regen_test.rs"]
mod tests;
