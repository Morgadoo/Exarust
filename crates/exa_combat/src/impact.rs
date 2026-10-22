//! A body striking a character's limb: what the original's limb impact
//! does with the impact the pair stage measured
//! ([`exa_physics::body::Touch::struck`]).
//!
//! - **Gates.** A striking limb's contact is the swing's, not this. The
//!   character's impact clock (its root part's sound clock) must have run
//!   out, and the impact, a third of it (in double precision) unless the
//!   character is down and then halved, must reach 0.5
//!   ([`limb_impact_sound`]).
//! - **Damage.** Over a volume of 0.25 the damage is
//!   `(volume − 0.25)·m`, m the root part's damage multiplier, times
//!   `1 − s` when the root sinks (s, negative, its sinking speed), less the
//!   impact damage already taken ([`limb_impact_damage`]). A hard impact
//!   on a part that strikes in an overhead swing is dropped: nothing more,
//!   not even the clock. The excess raises the impact damage taken, deals
//!   half of itself to yellow health (vital once the taken total passes
//!   0.6) and dazes the body by half of itself; [`LimbImpact`] tells the
//!   game, which grows the struck character's concern toward the body's
//!   user.
//! - **The clock.** The root part's clock is set to the game clock plus
//!   `220 + d − round(170·volume)`, d a draw below 100.
//! - **Upkeep.** Each tick the impact damage taken falls by 0.01 while
//!   over 0.01 (a dead body keeps it), and the root's sinking speed takes
//!   a slower height change at once and blends a faster one in at 0.005.
//!
//! [inf] The original reads the sinking point through the part's own
//! weights; Exarust takes the root part's origin. The impact's sound, its
//! voice request, the heavy-impact callback, the weapon branch and the
//! level's impacts are not wired.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_physics::body::{BodySystems, BodyTicks, GoLimp, Pairs, Ragdoll, Shove, SolverStages};
use exa_physics::body::{Fallen, daze_energy};
use exa_physics::part::{PartState, limb_impact_damage, limb_impact_sound};

use crate::attack::AttackMotion;
use crate::damage::part_multipliers;
use crate::fighter::{Damage, Fighter, SecondWindFired, StoryMode, Taken, Weakened};
use crate::{Attacking, Condition, Roll, StateFlags, collapse};

/// What a character keeps for its limb impacts: the impact damage taken
/// and its root's sinking speed (cm a tick, 0 or less) and last height.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct ImpactBook {
    pub taken: f32,
    pub sinking: f32,
    pub height: Option<f32>,
}

/// The impact damage taken falls by this a tick while over it.
pub const TAKEN_DECAY: f64 = 0.01;
/// The share of a faster sinking blended in a tick.
const SINK_BLEND: f64 = 0.005;
const SINK_KEEP: f64 = 0.995;
/// The impact clock's draw: below this (ms).
const CLOCK_SPREAD_MS: u32 = 100;

impl ImpactBook {
    /// The sinking speed after the root moved from its last height to
    /// `height` (cm).
    pub fn sink(&mut self, height: f32) {
        if let Some(last) = self.height {
            let mut drop = height - last;
            if drop > 0.0 {
                drop = 0.0;
            }
            self.sinking = if drop > self.sinking {
                drop
            } else {
                (f64::from(self.sinking) * SINK_KEEP + f64::from(drop) * SINK_BLEND) as f32
            };
        }
        self.height = Some(height);
    }

    /// One tick's fall of the impact damage taken.
    pub fn recover(&mut self) {
        if f64::from(self.taken) > TAKEN_DECAY {
            self.taken = (f64::from(self.taken) - TAKEN_DECAY) as f32;
        }
    }
}

/// A body's impact hurt a character: `amount` is the new damage, `other`
/// the body (its root) that struck.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct LimbImpact {
    pub target: Entity,
    pub bone: usize,
    pub other: Entity,
    pub amount: f32,
}

/// One side of a touch a body struck.
struct Struck {
    target: Entity,
    part: usize,
    other: Entity,
    impact: f32,
}

/// Limb impacts from the pair stage's last detection, then each
/// character's upkeep.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn strike_limbs(
    mut commands: Commands,
    stages: Option<Res<SolverStages>>,
    ticks: Option<Res<BodyTicks>>,
    story: Option<Res<StoryMode>>,
    mut fighters: Query<(
        Entity,
        &mut Fighter,
        &mut ImpactBook,
        &mut Condition,
        &mut StateFlags,
        Option<&mut exa_anim::Animator>,
        Option<&Attacking>,
        &mut Ragdoll,
    )>,
    bodies: Query<(Entity, &Ragdoll), Without<Fighter>>,
    mut shoves: MessageWriter<Shove>,
    mut impacts: MessageWriter<LimbImpact>,
    mut weakened: MessageWriter<Weakened>,
    mut winds: MessageWriter<SecondWindFired>,
    mut roll: Local<Roll>,
) {
    let now = ticks.as_ref().map_or(0, |t| t.millis()) as u32;
    let story = story.is_some_and(|s| s.0);
    let touches = stages
        .as_deref()
        .and_then(|s| s.get::<Pairs>())
        .map_or(&[][..], Pairs::touches);
    let mut struck = Vec::new();
    if touches.iter().any(|t| t.struck.iter().any(Option::is_some)) {
        let mut by_key: HashMap<Entity, (Entity, &Ragdoll)> = HashMap::default();
        for (root, body) in fighters.iter().map(|(e, .., b)| (e, b)) {
            if let Some(first) = body.parts.first() {
                by_key.insert(first.entity, (root, body));
            }
        }
        for (root, body) in &bodies {
            if let Some(first) = body.parts.first() {
                by_key.insert(first.entity, (root, body));
            }
        }
        for touch in touches {
            let sides = [(touch.a, touch.b), (touch.b, touch.a)];
            for (impact, (own, other)) in touch.struck.iter().zip(sides) {
                let (Some(impact), Some(&(target, _)), Some(&(other_root, other_body))) =
                    (impact, by_key.get(&own.0), by_key.get(&other.0))
                else {
                    continue;
                };
                let Some(other_part) = other_body.parts.get(other.1) else {
                    continue;
                };
                if other_part.state.is_limb() && other_part.state.has(PartState::STRIKING) {
                    continue;
                }
                if fighters.contains(target) {
                    struck.push(Struck {
                        target,
                        part: own.1,
                        other: other_root,
                        impact: *impact,
                    });
                }
            }
        }
    }
    for s in struck {
        let Ok((_, mut f, mut book, mut condition, mut flags, mut anim, attacking, mut body)) =
            fighters.get_mut(s.target)
        else {
            continue;
        };
        let Some(root) = body.parts.iter().position(|p| p.parent.is_none()) else {
            continue;
        };
        let Some(part) = body.parts.get(s.part) else {
            continue;
        };
        let (bone, striking) = (part.bone, part.state.has(PartState::STRIKING));
        if now <= body.parts[root].sound_ready_ms {
            continue;
        }
        let down = condition.has(Condition::DOWN.0);
        let Some(sound) = limb_impact_sound(s.impact, down, || roll.below(CLOCK_SPREAD_MS)) else {
            continue;
        };
        let overhead = attacking.is_some_and(|a| a.motion == AttackMotion::Overhead);
        let scale = part_multipliers(body.parts[root].bone).0;
        let hit = limb_impact_damage(
            sound.volume,
            scale,
            book.sinking,
            book.taken,
            overhead && striking,
        );
        if hit.dropped {
            continue;
        }
        if let Some(hurt) = hit.hurt {
            book.taken = hurt.total;
            let half = hurt.amount * 0.5;
            let damage = Damage {
                yellow: half,
                red: 0.0,
                vital: hurt.staggers,
            };
            let rules = f.stats.rules;
            let taken = f.health.take(damage, rules, *flags, story);
            shoves.write(Shove {
                body: s.target,
                bone,
                velocity: Vec3::ZERO,
                daze: half,
                step: None,
                sag: None,
            });
            impacts.write(LimbImpact {
                target: s.target,
                bone,
                other: s.other,
                amount: hurt.amount,
            });
            info!(
                "{:?} bone {bone} struck by {:?}: impact {:.2}, damage {:.3} (taken {:.3}), yellow {:.3}",
                s.target, s.other, s.impact, hurt.amount, hurt.total, f.health.yellow
            );
            if taken == Taken::Weakened {
                crate::answer_low(s.target, now, &mut f.second_wind, &mut winds, &mut weakened);
            }
            if taken == Taken::Killed && !condition.dying() {
                let living = body.drive.as_ref();
                if let (Some(energy), Some(anim)) = (
                    living.map(|d| daze_energy(d.energy, half)),
                    anim.as_deref_mut(),
                ) {
                    collapse::start_collapse(
                        &mut condition,
                        &mut flags,
                        anim,
                        Some(&mut body),
                        energy,
                        None,
                    );
                    commands
                        .entity(s.target)
                        .insert(collapse::Collapse::default())
                        .remove::<Fallen>();
                } else {
                    condition.die();
                    commands.entity(s.target).insert(GoLimp {
                        velocity: None,
                        delay: 0,
                    });
                }
            }
        }
        body.parts[root].sound_ready_ms = now.wrapping_add(sound.cooldown_ms);
    }
    for (_, _, mut book, condition, _, _, _, body) in &mut fighters {
        if let Some(root) = body.parts.iter().find(|p| p.parent.is_none()) {
            book.sink(root.transform().translation.y * 100.0);
        }
        if *condition != Condition::DEAD {
            book.recover();
        }
    }
}

/// Wires the limb impacts after each tick of the body solver.
pub(crate) fn plugin(app: &mut App) {
    app.add_message::<LimbImpact>()
        .add_systems(FixedUpdate, strike_limbs.after(BodySystems));
}

#[cfg(test)]
#[path = "impact_test.rs"]
mod tests;
