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
//!   on a part that strikes in an overhead swing is dropped once it has
//!   sounded: no damage, no random draw and no clock. The excess raises
//!   the impact damage taken, deals half of itself to yellow health (vital
//!   once the taken total passes 0.6) and dazes the body by half of
//!   itself; [`LimbImpact`] tells the game, which grows the struck
//!   character's concern toward the body's user.
//! - **The sound.** Each struck part's counter rises by 2 while under 40
//!   and falls by 1 a tick down to 1, before the gates. When the root's
//!   counter is over 2 the sound is the character's foley at the volume
//!   times the foley's own and pitch `0.5·volume + 0.6`; else the struck
//!   part's hit sound at the volume and pitch `0.6·volume + 0.5`, either at
//!   the character ([`ImpactSounded`]).
//! - **The clock.** After the damage step the root part's clock is set to
//!   the game clock plus `220 + d − round(170·volume)`, d a draw below 100.
//! - **Upkeep.** Each tick the impact damage taken falls by 0.01 while
//!   over 0.01 (a dead body keeps it), and the root's sinking speed takes
//!   a slower height change at once and blends a faster one in at 0.005.
//!
//! [inf] The original reads the sinking point through the part's own
//! weights; Exarust takes the root part's origin, and the character's
//! counter as its root part's. The impact's voice request, the
//! heavy-impact callback, the weapon branch and the level's impacts are
//! not wired.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_physics::body::{BodySystems, BodyTicks, GoLimp, Pairs, Ragdoll, Shove, SolverStages};
use exa_physics::body::{Fallen, daze_energy};
use exa_physics::part::{
    Contact, ContactKind, HardImpact, PartState, classify, hard_impact, limb_impact_damage,
    limb_impact_sound, stamp_sound_cooldown, weapon_impact_sound,
};

use crate::attack::AttackMotion;
use crate::damage::part_multipliers;
use crate::fighter::{Damage, Fighter, SecondWindFired, StoryMode, Taken, Weakened};
use crate::swing::SwingState;
use crate::{Attacker, Attacking, Blade, Condition, Motions, Roll, StateFlags, collapse};

/// What a character keeps for its limb impacts: the impact damage taken
/// and its root's sinking speed (cm a tick, 0 or less) and last height.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct ImpactBook {
    pub taken: f32,
    pub sinking: f32,
    pub height: Option<f32>,
    /// Each part's recent-impact counter, by part index.
    pub counts: Vec<i32>,
}

/// The impact damage taken falls by this a tick while over it.
pub const TAKEN_DECAY: f64 = 0.01;
/// The share of a faster sinking blended in a tick.
const SINK_BLEND: f64 = 0.005;
const SINK_KEEP: f64 = 0.995;
/// The impact clock's draw: below this (ms).
const CLOCK_SPREAD_MS: u32 = 100;
/// A struck part's counter rises by 2 while under 40.
const COUNT_STEP: i32 = 2;
const COUNT_CAP: i32 = 40;
/// Over this the root's impacts sound through the character's foley.
pub const CROWDED: i32 = 2;

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

    /// A part was struck: its counter rises; the counter after it.
    pub fn count(&mut self, part: usize) -> i32 {
        if self.counts.len() <= part {
            self.counts.resize(part + 1, 0);
        }
        let c = &mut self.counts[part];
        if *c < COUNT_CAP {
            *c += COUNT_STEP;
        }
        *c
    }

    /// A part's counter (0 for one never struck).
    pub fn count_of(&self, part: usize) -> i32 {
        self.counts.get(part).copied().unwrap_or(0)
    }

    /// One tick's fall of each part's counter, down to 1.
    pub fn settle_counts(&mut self) {
        for c in &mut self.counts {
            if *c > 1 {
                *c -= 1;
            }
        }
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

/// A limb impact's sound at the character (m): through the character's
/// foley when its root is crowded, else the struck bone's hit sound, at
/// `volume` (before the foley's own) and `pitch`.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct ImpactSounded {
    pub target: Entity,
    pub bone: usize,
    pub point: Vec3,
    pub volume: f32,
    pub pitch: f32,
    pub foley: bool,
}

/// A held weapon's own impact sound: the weapon record's set at the item's
/// point, with the volume and pitch the weapon branch works out.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct WeaponSounded {
    /// The item's body (its sound cooldown lives on it).
    pub target: Entity,
    pub point: Vec3,
    pub volume: f32,
    pub pitch: f32,
    /// The weapon's hit-sound set, when its record names one.
    pub set: Option<String>,
    /// The contact classifier's kind for the strike that hit the weapon,
    /// when the other side was itself striking.
    pub kind: Option<ContactKind>,
}

/// One side of a touch a body struck.
struct Struck {
    target: Entity,
    part: usize,
    other: Entity,
    other_strikes: bool,
    impact: f32,
}

/// Whether a hard impact is dropped after its sound and before the clock's
/// random spread is drawn: on a part that strikes in an overhead swing.
fn overhead_drop(overhead: bool, striking: bool) -> bool {
    overhead && striking
}

/// The first name on an entity or its ancestors, for the debug log.
fn name_of(names: &Query<(Option<&Name>, Option<&ChildOf>)>, mut e: Entity) -> String {
    for _ in 0..8 {
        let Ok((name, parent)) = names.get(e) else {
            break;
        };
        if let Some(name) = name {
            return name.as_str().to_owned();
        }
        let Some(parent) = parent else {
            break;
        };
        e = parent.parent();
    }
    String::from("?")
}

/// Whether a body lives (has muscles), for the debug log.
fn life(body: &Ragdoll) -> &'static str {
    if body.drive.is_some() {
        "living"
    } else {
        "limp"
    }
}

/// The extra queries the weapon-body impact pass needs, bundled to stay
/// within a system's parameter list.
#[derive(bevy::ecs::system::SystemParam)]
#[allow(clippy::type_complexity)]
pub struct WeaponImpacts<'w, 's> {
    pub weapons: Query<
        'w,
        's,
        (Entity, &'static mut Ragdoll, &'static mut ImpactBook),
        (Without<Fighter>, With<ImpactBook>),
    >,
    pub swings: Query<'w, 's, &'static mut SwingState>,
    pub attackers: Query<'w, 's, &'static Attacker>,
    pub blades: Query<'w, 's, &'static Blade>,
    pub motions: Option<Res<'w, Motions>>,
    pub sounds: MessageWriter<'w, WeaponSounded>,
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
    bodies: Query<(Entity, &Ragdoll), (Without<Fighter>, Without<ImpactBook>)>,
    mut weapon: WeaponImpacts,
    mut shoves: MessageWriter<Shove>,
    mut impacts: MessageWriter<LimbImpact>,
    mut sounds: MessageWriter<ImpactSounded>,
    mut weakened: MessageWriter<Weakened>,
    mut winds: MessageWriter<SecondWindFired>,
    mut roll: Local<Roll>,
    names: Query<(Option<&Name>, Option<&ChildOf>)>,
) {
    let now = ticks.as_ref().map_or(0, |t| t.millis()) as u32;
    let story = story.is_some_and(|s| s.0);
    // The part step takes one off each recent-impact counter before this
    // tick's contacts, so a count a blow adds now stays whole until the
    // next tick.
    for (_, _, mut book, ..) in &mut fighters {
        book.settle_counts();
    }
    let touches = stages
        .as_deref()
        .and_then(|s| s.get::<Pairs>())
        .map_or(&[][..], Pairs::touches);
    // The held item bodies, their part states, and the character each
    // belongs to, read without holding a body's borrow.
    let mut owner_of: HashMap<Entity, Entity> = HashMap::default();
    let mut items: HashMap<Entity, Entity> = HashMap::default();
    let mut part_states: HashMap<(Entity, usize), PartState> = HashMap::default();
    for (root, body) in fighters.iter().map(|(e, .., b)| (e, b)) {
        let Some(key) = body.parts.first().map(|p| p.entity) else {
            continue;
        };
        owner_of.insert(key, root);
        for (i, part) in body.parts.iter().enumerate() {
            part_states.insert((key, i), part.state);
        }
    }
    for (root, body) in &bodies {
        let Some(key) = body.parts.first().map(|p| p.entity) else {
            continue;
        };
        for (i, part) in body.parts.iter().enumerate() {
            part_states.insert((key, i), part.state);
        }
        let _ = root;
    }
    for (root, body, _) in &weapon.weapons {
        let Some(key) = body.parts.first().map(|p| p.entity) else {
            continue;
        };
        items.insert(key, root);
        for (i, part) in body.parts.iter().enumerate() {
            part_states.insert((key, i), part.state);
        }
    }
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
                let (Some(impact), Some(&(target, own_body)), Some(&(other_root, other_body))) =
                    (impact, by_key.get(&own.0), by_key.get(&other.0))
                else {
                    continue;
                };
                let Some(other_part) = other_body.parts.get(other.1) else {
                    continue;
                };
                let other_strikes =
                    other_part.state.is_limb() && other_part.state.has(PartState::STRIKING);
                if fighters.contains(target) {
                    struck.push(Struck {
                        target,
                        part: own.1,
                        other: other_root,
                        other_strikes,
                        impact: *impact,
                    });
                    if bevy::log::tracing::enabled!(bevy::log::Level::DEBUG) {
                        debug!(
                            "limb impact candidate: {:?} {:?} ({}, {:?}) part {} struck by {:?} {:?} part {} \
                             (bone {}, {}, {:?}, countdown {}, settling {}): \
                             impact {:.2}, travel² {:.2}, hits {}, push {:.2} cm",
                            target,
                            name_of(&names, target),
                            life(own_body),
                            fighters.get(target).map(|f| *f.3).ok(),
                            own.1,
                            other_root,
                            name_of(&names, other_root),
                            other.1,
                            other_part.bone,
                            life(other_body),
                            other_body.sleep,
                            other_body.countdown(),
                            other_body.settling(),
                            impact,
                            touch.travel2,
                            touch.hits,
                            touch.push.length() * 100.0,
                        );
                    }
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
        book.count(s.part);
        if s.other_strikes || now <= body.parts[root].sound_ready_ms {
            continue;
        }
        let down = condition.has(Condition::DOWN.0);
        let Some(sound) = limb_impact_sound(s.impact, down) else {
            continue;
        };
        let crowded = book.count_of(root);
        sounds.write(ImpactSounded {
            target: s.target,
            bone,
            point: body.parts[root].transform().translation.into(),
            volume: sound.volume,
            pitch: sound.pitch_for(crowded),
            foley: crowded > CROWDED,
        });
        let scale = part_multipliers(body.parts[root].bone).0;
        let overhead = attacking.is_some_and(|a| a.motion == AttackMotion::Overhead);
        let hit = limb_impact_damage(
            sound.volume,
            scale,
            book.sinking,
            book.taken,
            overhead_drop(overhead, striking),
        );
        // A dropped hard impact has already sounded; the original returns
        // before the clock's random spread is drawn and the clock is set.
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
        let cooldown = sound.spread_cooldown(roll.below(CLOCK_SPREAD_MS));
        body.parts[root].sound_ready_ms = now.wrapping_add(cooldown);
    }
    // A held item's own weapon-body impact: a weapon that is striking (its
    // owner's window open) rings its own set and interrupts or stalls the
    // swing by the original's weapon branch. A non-striking weapon takes
    // the base collision path, which Exarust's rigid part sound covers.
    for touch in touches {
        let sides = [(touch.a, touch.b), (touch.b, touch.a)];
        for (impact, (own, other)) in touch.struck.iter().zip(sides) {
            let Some(impact) = impact else {
                continue;
            };
            let Some(&target) = items.get(&own.0) else {
                continue;
            };
            let other_state = part_states.get(&(other.0, other.1)).copied();
            if other_state.is_some_and(|s| s.is_limb()) {
                continue;
            }
            let Ok((_, mut item, mut book)) = weapon.weapons.get_mut(target) else {
                continue;
            };
            book.count(0);
            let striking = item
                .parts
                .first()
                .is_some_and(|p| p.state.has(PartState::STRIKING));
            if !striking {
                continue;
            }
            let Some(owner_key) = item.held_by else {
                continue;
            };
            let Some(&owner) = owner_of.get(&owner_key) else {
                continue;
            };
            let Ok((_, _, _, _, _, anim, attacking, _)) = fighters.get_mut(owner) else {
                continue;
            };
            let Ok(mut swing) = weapon.swings.get_mut(owner) else {
                continue;
            };
            let clock = match (weapon.motions.as_ref(), anim.as_deref()) {
                (Some(motions), Some(anim)) => motions
                    .clip(anim, &anim.action)
                    .map_or(0.0, |c| anim.clip_progress(c)),
                _ => 0.0,
            };
            let motion = attacking.map_or(0, |a| a.motion.index() as i32);
            let primary = weapon.attackers.get(owner).ok().and_then(|a| a.primary);
            let blade = weapon.blades.get(target).ok();
            let root_is_primary = blade.is_some_and(|b| primary == Some(b.slot));
            let counter_taken = attacking.is_some_and(|a| a.weapon_class == 0x90)
                && f64::from(clock) > 0.4
                && exa_physics::part::immediate_counter(swing.flags, motion).is_some();
            let hit = HardImpact {
                flags: swing.flags,
                motion,
                clock,
                impact: *impact,
                has_children: item.parts.len() > 1,
                root_is_primary,
                counter_taken,
                now_ms: now,
                // The original adds a shared simulation field here; its
                // writer was not traced, so no time is added.
                link_ticks: 0,
            };
            if let Some(reaction) = hard_impact(&hit) {
                swing.flags = reaction.flags;
                if reaction.interrupted {
                    swing.strength = 1.0;
                    swing.stall = 0.0;
                    swing.end_ms = u64::from(reaction.stall_end_ms);
                }
                if let Some(strength) = reaction.strike_strength {
                    swing.strike_strength = strength;
                }
                if reaction.clears_strike_chain {
                    for part in &mut item.parts {
                        part.state = part.state.without(PartState::STRIKE_CHAIN);
                    }
                }
            }
            let Some(sound) = weapon_impact_sound(*impact) else {
                continue;
            };
            // The original stamps the item, its first-child chain and any
            // striking weapon ancestors with the sound's cooldown.
            let states: Vec<PartState> = item.parts.iter().map(|p| p.state).collect();
            let parents: Vec<Option<usize>> = item.parts.iter().map(|p| p.parent).collect();
            let mut first_child = vec![None; item.parts.len()];
            for (i, p) in item.parts.iter().enumerate() {
                if let Some(parent) = p.parent
                    && first_child[parent].is_none()
                {
                    first_child[parent] = Some(i);
                }
            }
            let mut ready = vec![0u32; item.parts.len()];
            stamp_sound_cooldown(
                &states,
                &parents,
                &first_child,
                0,
                now.wrapping_add(sound.cooldown_ms),
                &mut ready,
            );
            for (part, at) in item.parts.iter_mut().zip(ready) {
                part.sound_ready_ms = at;
            }
            let kind = if other_state.is_some_and(|s| s.strikes()) {
                let contact = Contact {
                    own: other_state.unwrap_or_default(),
                    other: item.parts.first().map_or_default(|p| p.state),
                    same_tree: false,
                    faces: true,
                    own_motion: 0,
                    own_flags: 0,
                    own_pending: 0,
                    other_parrying: false,
                    other_motion: 0,
                    other_is_primary: false,
                    other_guarded: false,
                };
                classify(&contact).map(|c| c.kind)
            } else {
                None
            };
            weapon.sounds.write(WeaponSounded {
                target,
                point: item
                    .parts
                    .first()
                    .map_or(Vec3::ZERO, |p| p.transform().translation.into()),
                volume: sound.volume,
                pitch: sound.pitch,
                set: blade
                    .and_then(|b| b.record.hit_sound_name())
                    .map(str::to_owned),
                kind,
            });
        }
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

/// Sets each held item's strike flags from its owner's swing window: a
/// weapon body is a striker (class bit 1) while its owner's striker window
/// is open, as the original's attack sets it, and stops being one when the
/// window shuts.
fn sync_held_striking(mut blades: Query<(&Blade, &mut Ragdoll)>, swings: Query<&SwingState>) {
    for (blade, mut body) in &mut blades {
        if body.held_by.is_none() {
            continue;
        }
        let open = swings
            .get(blade.wielder)
            .is_ok_and(|s| s.open[blade.slot.index()]);
        body.set_striking_window(0, open);
    }
}

/// Wires the limb impacts after each tick of the body solver.
pub(crate) fn plugin(app: &mut App) {
    app.add_message::<LimbImpact>()
        .add_message::<ImpactSounded>()
        .add_message::<WeaponSounded>()
        .add_systems(FixedUpdate, sync_held_striking.before(BodySystems))
        .add_systems(FixedUpdate, strike_limbs.after(BodySystems));
}

#[cfg(test)]
#[path = "impact_test.rs"]
mod tests;
