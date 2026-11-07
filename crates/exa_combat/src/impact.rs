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
//! counter as its root part's. A hard impact that is not from a character
//! part asks for the pain grunt and, once a second while the down bit is
//! set, the BadFall broadcast ([`hard_impact_cry`]). A volume over 0.3
//! also asks the caster's force shell to ripple ([`ShellHit`]).

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_physics::body::{BodySystems, BodyTicks, GoLimp, Pairs, Ragdoll, Shove, SolverStages};
use exa_physics::body::{Fallen, daze_energy};
use exa_physics::part::{
    Contact, ContactKind, HardImpact, ImpactSound, PartState, base_impact_sound, classify,
    hard_impact, limb_impact_damage, limb_impact_sound, limb_impact_voice, stamp_sound_cooldown,
    weapon_impact_sound,
};

use crate::pain::{Loudness, PainSounded, PainSounds};

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
    /// The game clock before which a hard impact does not repeat the fall's
    /// script message. A hard impact stamps it a second ahead once it has
    /// passed, whether or not the character is down.
    pub fall_notice_ms: u32,
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
    /// The contact (world, m).
    pub point: Vec3,
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

/// A hard overhead landing that may shake the camera: the attacker's
/// position (m), where the original measures from its body centre.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct CameraHit {
    pub at: Vec3,
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
    /// The other body is a character part: both of the limb-family bits are
    /// set. Such a blow still sounds and hurts when the part is not itself
    /// striking; it does not grunt or announce the fall.
    other_limb: bool,
    /// The mean contact point (world, m).
    point: Vec3,
    impact: f32,
}

/// A limb impact whose volume is over 0.3: the caster's force shell, when
/// it has one, ripples at the contact. The shell keeps at most eight and
/// ignores the rest. A dropped overhead still ripples; the original calls
/// the shell before it returns.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct ShellHit {
    pub fighter: Entity,
    /// The contact (world, m).
    pub point: Vec3,
    /// The impact sound's volume.
    pub volume: f32,
}

/// A hard impact on a down character, at most once a second: witnesses who
/// can see the character activate their BadFall topic.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadFall {
    pub fighter: Entity,
}

/// What a sounding limb impact adds after its damage and its clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactCry {
    /// The grunt, when the other body is not a character part, the scaled
    /// impact is over 15 and the character is not dead.
    pub voice: Option<exa_physics::part::ImpactVoice>,
    /// [`ImpactBook::fall_notice_ms`] after this impact.
    pub notice_ms: u32,
    /// The down bit is set and the one-second notice has elapsed.
    pub bad_fall: bool,
}

/// The pain grunt and the fall notice a sounding limb impact adds. The
/// grunt is line 1 at half the impact's volume; the voice's own spacing
/// still gates whether it plays. The notice moves a second ahead whenever
/// it has elapsed, and the fall is announced only then, and only while the
/// down bit is set (getting up still has it; dead does not reach here).
/// A character part, a scaled impact of 15 or less, and the dead condition
/// leave the notice where it is.
pub fn hard_impact_cry(
    sound: &ImpactSound,
    condition: u32,
    other_limb: bool,
    notice_ms: u32,
    now: u32,
) -> ImpactCry {
    let voice = if other_limb {
        None
    } else {
        limb_impact_voice(sound, condition)
    };
    if voice.is_none() {
        return ImpactCry {
            voice: None,
            notice_ms,
            bad_fall: false,
        };
    }
    if notice_ms < now {
        ImpactCry {
            voice,
            notice_ms: now.wrapping_add(1_000),
            bad_fall: condition & Condition::DOWN.0 != 0,
        }
    } else {
        ImpactCry {
            voice,
            notice_ms,
            bad_fall: false,
        }
    }
}

/// Writes [`hard_impact_cry`] onto the character and the two messages.
#[allow(clippy::too_many_arguments)]
fn note_cry(
    fighter: Entity,
    point: Vec3,
    condition: Condition,
    book: &mut ImpactBook,
    sound: &ImpactSound,
    other_limb: bool,
    now: u32,
    cries: &mut ImpactCries,
) {
    let cry = hard_impact_cry(sound, condition.0, other_limb, book.fall_notice_ms, now);
    book.fall_notice_ms = cry.notice_ms;
    if let Some(voice) = cry.voice {
        cries.pains.write(PainSounded {
            fighter,
            point,
            sounds: PainSounds {
                wound: None,
                voice: Some((
                    1,
                    Loudness {
                        volume: voice.volume,
                        pitch: voice.pitch,
                    },
                )),
            },
        });
    }
    if cry.bad_fall {
        cries.falls.write(BadFall { fighter });
    }
}

/// Whether a hard impact is dropped after its sound and before the clock's
/// random spread is drawn: on a part that strikes in an overhead swing.
/// Stance class of a weapon in each hand.
const STANCE_TWO_WEAPONS: u8 = 6;

/// Whether a hard hit on a striking weapon interrupts its swing (the
/// conditions [`hard_impact`] tests before an immediate counter): an
/// active side strike not stalled, with children, its chain the primary,
/// past 0.6 of its clip.
fn interrupts(flags: u32, has_children: bool, root_is_primary: bool, clock: f32) -> bool {
    flags & 0x1_0005 == 1
        && flags & 0x2 == 0
        && has_children
        && root_is_primary
        && f64::from(clock) > 0.6
}

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
    pub attackers: Query<'w, 's, &'static mut Attacker>,
    /// The fighters' swing records (the chain's blend).
    pub records: Query<'w, 's, &'static crate::swing::Swing>,
    pub blades: Query<'w, 's, &'static Blade>,
    /// A held item's own collision sound sets, part for part.
    pub body_sounds: Query<'w, 's, &'static exa_physics::prop::BodySounds>,
    pub motions: Option<Res<'w, Motions>>,
    pub sounds: MessageWriter<'w, WeaponSounded>,
    pub camera_hits: MessageWriter<'w, CameraHit>,
}

/// The pain grunt and the fall broadcast a hard limb impact writes.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ImpactCries<'w> {
    pains: MessageWriter<'w, PainSounded>,
    falls: MessageWriter<'w, BadFall>,
    shells: MessageWriter<'w, ShellHit>,
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
        Option<&mut Attacking>,
        &mut Ragdoll,
    )>,
    bodies: Query<(Entity, &Ragdoll), (Without<Fighter>, Without<ImpactBook>)>,
    mut weapon: WeaponImpacts,
    mut shoves: MessageWriter<Shove>,
    mut impacts: MessageWriter<LimbImpact>,
    mut sounds: MessageWriter<ImpactSounded>,
    mut weakened: MessageWriter<Weakened>,
    mut winds: MessageWriter<SecondWindFired>,
    mut cries: ImpactCries,
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
    // belongs to, read without holding a body's borrow; only gathered when
    // an item body exists and the pair stage found touches this tick.
    let mut owner_of: HashMap<Entity, Entity> = HashMap::default();
    let mut items: HashMap<Entity, Entity> = HashMap::default();
    let mut part_states: HashMap<(Entity, usize), PartState> = HashMap::default();
    if !touches.is_empty() && !weapon.weapons.is_empty() {
        for (root, body, _) in &weapon.weapons {
            let Some(key) = body.parts.first().map(|p| p.entity) else {
                continue;
            };
            items.insert(key, root);
            for (i, part) in body.parts.iter().enumerate() {
                part_states.insert((key, i), part.state);
            }
        }
        for (root, body) in fighters.iter().map(|(e, .., b)| (e, b)) {
            let Some(key) = body.parts.first().map(|p| p.entity) else {
                continue;
            };
            owner_of.insert(key, root);
            for (i, part) in body.parts.iter().enumerate() {
                part_states.insert((key, i), part.state);
            }
        }
        // The other bodies (the level's props, thousands of them) only for
        // the keys a touch names: the only ones looked up.
        let touched: bevy::ecs::entity::EntityHashSet =
            touches.iter().flat_map(|t| [t.a.0, t.b.0]).collect();
        for (_, body) in &bodies {
            let Some(key) = body.parts.first().map(|p| p.entity) else {
                continue;
            };
            if !touched.contains(&key) {
                continue;
            }
            for (i, part) in body.parts.iter().enumerate() {
                part_states.insert((key, i), part.state);
            }
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
                let other_limb = other_part.state.bits() & PartState::LIMB == PartState::LIMB;
                if fighters.contains(target) {
                    struck.push(Struck {
                        target,
                        part: own.1,
                        other: other_root,
                        other_strikes,
                        other_limb,
                        point: touch.point,
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
        // The shell's ripple is asked before a dropped overhead returns,
        // and before the clock is set.
        if hit.heavy {
            cries.shells.write(ShellHit {
                fighter: s.target,
                point: s.point,
                volume: sound.volume,
            });
        }
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
                point: s.point,
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
        let point = body.parts[root].transform().translation.into();
        note_cry(
            s.target,
            point,
            *condition,
            &mut book,
            &sound,
            s.other_limb,
            now,
            &mut cries,
        );
    }
    // A held item's own weapon-body impact: a held weapon (its permanent
    // weapon bit) rings its own set and, while its owner's window is open,
    // interrupts or stalls the swing by the original's weapon branch. A
    // held shield or torch takes the base collision path with its own
    // material's collision sound.
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
            // A weapon meeting a limb: the original runs this branch too
            // (and stays silent against a limb), but Exarust's blade sweep
            // already resolves a weapon striking a body (damage, recoil and
            // the swing's end), so the branch is left to weapon-on-weapon
            // and weapon-on-prop contacts to keep from answering a hit twice.
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
                // A held item that is not a weapon takes the plain body's
                // collision sound with its material's set (the struck
                // part's); an item without one falls back to a blade's hit
                // set. [guess: the fallback]
                let Some(part) = item.parts.first() else {
                    continue;
                };
                if !part.reacts {
                    continue;
                }
                let (mobility, ready, last, point) = (
                    part.pairing.mobility,
                    part.sound_ready_ms,
                    part.last_impact,
                    part.transform().translation,
                );
                let Some(sound) = base_impact_sound(*impact, mobility, ready, last, now) else {
                    continue;
                };
                let own_set = weapon
                    .body_sounds
                    .get(target)
                    .ok()
                    .and_then(|s| s.impact.get(own.1).cloned().flatten());
                let set = own_set.or_else(|| {
                    weapon
                        .blades
                        .get(target)
                        .ok()
                        .and_then(|b| b.record.hit_sound_name())
                        .map(str::to_owned)
                });
                // The original parks the clock of a body with no sound set;
                // the stand-in set is the weapon's, whose clock the striking
                // branch shares, so a missing set only stays silent here.
                let Some(set) = set else {
                    continue;
                };
                // The spread is drawn after the sound plays, as the
                // original draws it.
                let spread = roll.below(100);
                let cooldown = sound.cooldown_ms.wrapping_add(spread);
                if let Some(part) = item.parts.first_mut() {
                    part.sound_ready_ms = now.wrapping_add(cooldown);
                    part.last_impact = sound.threshold;
                }
                weapon.sounds.write(WeaponSounded {
                    target,
                    point: point.into(),
                    volume: sound.volume,
                    pitch: sound.pitch,
                    set: Some(set),
                    kind: None,
                });
                continue;
            }
            let Some(owner_key) = item.held_by else {
                continue;
            };
            let Some(&owner) = owner_of.get(&owner_key) else {
                continue;
            };
            let Ok((_, f, _, _, _, mut anim, mut attacking, body)) = fighters.get_mut(owner) else {
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
            let motion = attacking.as_deref().map_or(0, |a| a.motion.index() as i32);
            let (primary, two_weapons) =
                weapon.attackers.get(owner).ok().map_or((None, false), |a| {
                    (a.primary, a.stance_class == STANCE_TWO_WEAPONS)
                });
            let blade = weapon.blades.get(target).ok();
            let root_is_primary = blade.is_some_and(|b| primary == Some(b.slot));
            let has_children = item.parts.len() > 1;
            // The immediate counter (two weapons past 0.4 of the clip): the
            // original runs it where a hard hit would interrupt the swing,
            // and a counter that starts takes the interrupt's place.
            let counter_taken = two_weapons
                && interrupts(swing.flags, has_children, root_is_primary, clock)
                && f64::from(clock) > 0.4
                && exa_physics::part::immediate_counter(swing.flags, motion).is_some();
            let hit = HardImpact {
                flags: swing.flags,
                motion,
                clock,
                impact: *impact,
                has_children,
                root_is_primary,
                counter_taken,
                now_ms: now,
                // The simulation's per-tick clock advance, added to the
                // stab's or overhead's stall.
                link_ticks: 12,
            };
            if exa_physics::part::shakes_camera(&hit, f.stats.scale) {
                let at = body
                    .parts
                    .iter()
                    .find(|p| p.parent.is_none())
                    .map_or(Vec3::ZERO, |p| p.transform().translation.into());
                weapon.camera_hits.write(CameraHit { at });
            }
            if counter_taken
                && let (Some(anim), Some(attacking), Ok(mut attacker)) = (
                    anim.as_deref_mut(),
                    attacking.as_deref_mut(),
                    weapon.attackers.get_mut(owner),
                )
            {
                // The counter chains the other side's attack; the hit's
                // reaction ends there (the stab and overhead test reads the
                // new side attack).
                let blend = weapon
                    .records
                    .get(owner)
                    .copied()
                    .unwrap_or(crate::swing::Swing::FISTS)
                    .chain_blend();
                let next = crate::swing::chain_side_attack(
                    &mut commands,
                    owner,
                    crate::swing::ChainedFighter {
                        anim,
                        attacker: &mut attacker,
                        state: &mut swing,
                        attacking,
                        fighter: &f,
                    },
                    blend,
                    u64::from(now),
                );
                info!(
                    "immediate counter: {owner:?} chains {} after a hard hit",
                    next.action()
                );
            } else if let Some(reaction) = hard_impact(&hit) {
                swing.flags = reaction.flags;
                if reaction.interrupted {
                    swing.strength = 1.0;
                    swing.stall = 0.0;
                    swing.end_ms = u64::from(reaction.stall_end_ms);
                }
                if let Some(strength) = reaction.strike_strength {
                    // The stab or overhead's stall: none, ending at the
                    // reaction's clock.
                    swing.strike_strength = strength;
                    swing.stall = 0.0;
                    swing.end_ms = u64::from(reaction.stall_end_ms);
                }
                if reaction.clears_strike_chain {
                    for part in &mut item.parts {
                        part.state = part.state.without(PartState::STRIKE_CHAIN);
                    }
                }
            }
            // Only once the item's own sound cooldown is over (the
            // original's unsigned `ready < now`).
            let ready = item.parts.first().map_or(0, |p| p.sound_ready_ms);
            if ready >= now {
                continue;
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

/// The level's impacts on a character's parts from the last tick's body
/// contacts: the character part's impact call with no other body
/// ([`exa_physics::body::LevelImpact`]). A part's recent-impact counter
/// rises, and while the root's sound clock is over, the limb curve gives
/// the volume and pitch (a third of the strength unless the character is
/// down, then half), it sounds at the character and, over a volume of
/// 0.25, deals its impact damage exactly as a body's blow does (yellow
/// health, daze, the taken total). As in [`strike_limbs`], a hard impact on
/// a striking part during an overhead swing is dropped before the clock is
/// set. With no other body the original skips the striker's combat-state
/// step, so no [`LimbImpact`] is sent. There is no other body, so the
/// pain grunt and the BadFall broadcast run as they do for a blow that is
/// not from a character part ([`hard_impact_cry`]).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn level_impacts(
    mut commands: Commands,
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
    mut sounds: MessageWriter<ImpactSounded>,
    mut shoves: MessageWriter<Shove>,
    mut weakened: MessageWriter<Weakened>,
    mut winds: MessageWriter<SecondWindFired>,
    mut cries: ImpactCries,
    mut roll: Local<Roll>,
) {
    let now = ticks.as_ref().map_or(0, |t| t.millis()) as u32;
    let story = story.is_some_and(|s| s.0);
    for (entity, mut f, mut book, mut condition, mut flags, mut anim, attacking, mut body) in
        &mut fighters
    {
        let Some(root) = body.parts.iter().position(|p| p.parent.is_none()) else {
            continue;
        };
        // The parts a level contact hit, taken out of the body before it is
        // borrowed mutably again.
        let toll: Vec<_> = body.level_impacts().to_vec();
        if toll.is_empty() {
            continue;
        }
        // The strength is only halved, not cut to a third first, for a
        // character that is down.
        let falling = condition.has(Condition::DOWN.0);
        for hit in toll {
            let Some(part) = body.parts.get(hit.part) else {
                continue;
            };
            let (bone, striking) = (part.bone, part.state.has(PartState::STRIKING));
            book.count(hit.part);
            if now <= body.parts[root].sound_ready_ms {
                continue;
            }
            let Some(sound) = limb_impact_sound(hit.strength, falling) else {
                continue;
            };
            let crowded = book.count_of(root);
            sounds.write(ImpactSounded {
                target: entity,
                bone,
                point: body.parts[root].transform().translation.into(),
                volume: sound.volume,
                pitch: sound.pitch_for(crowded),
                foley: crowded > CROWDED,
            });
            let scale = part_multipliers(body.parts[root].bone).0;
            let overhead = attacking.is_some_and(|a| a.motion == AttackMotion::Overhead);
            let damage = limb_impact_damage(
                sound.volume,
                scale,
                book.sinking,
                book.taken,
                overhead_drop(overhead, striking),
            );
            if damage.heavy {
                cries.shells.write(ShellHit {
                    fighter: entity,
                    point: hit.point,
                    volume: sound.volume,
                });
            }
            if damage.dropped {
                continue;
            }
            if let Some(hurt) = damage.hurt {
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
                    body: entity,
                    bone,
                    velocity: Vec3::ZERO,
                    daze: half,
                    step: None,
                    sag: None,
                });
                info!(
                    "{entity:?} bone {bone} hit the level: impact {:.2}, damage {:.3} (taken {:.3})",
                    hit.strength, hurt.amount, hurt.total
                );
                if taken == Taken::Weakened {
                    crate::answer_low(entity, now, &mut f.second_wind, &mut winds, &mut weakened);
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
                            .entity(entity)
                            .insert(collapse::Collapse::default())
                            .remove::<Fallen>();
                    } else {
                        condition.die();
                        commands.entity(entity).insert(GoLimp {
                            velocity: None,
                            delay: 0,
                        });
                    }
                }
            }
            let cooldown = sound.spread_cooldown(roll.below(CLOCK_SPREAD_MS));
            body.parts[root].sound_ready_ms = now.wrapping_add(cooldown);
            let point = body.parts[root].transform().translation.into();
            note_cry(
                entity, point, *condition, &mut book, &sound, false, now, &mut cries,
            );
        }
    }
}

/// Opens and shuts each held weapon's strike window from its owner's swing
/// window: the strike-chain bit (the original's 0x10000) over its chain,
/// beside the permanent weapon bit its attach set
/// ([`exa_physics::grip::weapon_words`]). Only the change is written, so a
/// hard impact's clear of the chain bit holds for the rest of the window,
/// as in the original. While it strikes, its contacts with hittable bodies
/// take the hit event ([`crate::block::strike_held`]) instead of the impact
/// path. `open` remembers the weapons whose window is open.
fn sync_held_striking(
    mut blades: Query<(Entity, &Blade, &mut Ragdoll)>,
    swings: Query<&SwingState>,
    mut open: Local<bevy::ecs::entity::EntityHashSet>,
) {
    for (entity, blade, mut body) in &mut blades {
        if body.held_by.is_none() {
            open.remove(&entity);
            continue;
        }
        let now_open = swings
            .get(blade.wielder)
            .is_ok_and(|s| s.open[blade.slot.index()]);
        if now_open != open.contains(&entity) {
            debug!(
                "{:?}'s weapon {} striking",
                blade.wielder,
                if now_open { "starts" } else { "stops" }
            );
            body.set_strike_chain(0, now_open);
            if now_open {
                open.insert(entity);
            } else {
                open.remove(&entity);
            }
        }
    }
}

/// Wires the limb impacts after each tick of the body solver.
pub(crate) fn plugin(app: &mut App) {
    app.add_message::<LimbImpact>()
        .add_message::<ImpactSounded>()
        .add_message::<WeaponSounded>()
        .add_message::<CameraHit>()
        .add_message::<PainSounded>()
        .add_message::<BadFall>()
        .add_message::<ShellHit>()
        .add_systems(
            FixedUpdate,
            sync_held_striking
                .after(exa_physics::body::TickBehaviour)
                .before(BodySystems),
        )
        .add_systems(FixedUpdate, strike_limbs.after(BodySystems))
        .add_systems(
            FixedUpdate,
            crate::block::strike_held
                .after(BodySystems)
                .before(strike_limbs),
        )
        .add_systems(FixedUpdate, level_impacts.after(strike_limbs));
}

#[cfg(test)]
#[path = "impact_test.rs"]
mod tests;
