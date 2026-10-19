//! Combat resolution.
//!
//! A held weapon is a segment from its grip to its tip, an empty hand's fist
//! one across the hand. While its wielder is attacking, the segments of the
//! strikers whose strike window is open ([`swing`]) are tested against the
//! body parts of other fighters. When a fighter may attack, and with what,
//! is the original's attack executor ([`attack`]); how the swing runs,
//! stalls, chains and returns to the stance, its swing clock ([`swing`]).
//! A contact is resolved by the original's damage model ([`damage`]): the
//! weapon's stats, where along it the blow lands and how fast, against the
//! struck part's armour layers. It takes health, shoves and dazes the target,
//! and when its yellow health runs out starts its collapse, which kills it
//! 3 s later ([`collapse`]). A blow that does not kill starts no clip on
//! the target; a target struck while attacking has its swing stalled by
//! the daze. A fighter's condition word ([`condition`]),
//! stance and combat readiness ([`stance`]) are kept with it
//! ([`fighter`]); its yellow health heals back towards red out of its
//! fighting stance, and some races' red too ([`regen`]). A body that falls with no ground
//! within 25 m under it screams, and dies out of the world 3 s later (the
//! player 4 s, after the pit's sound at 1 s, [`fall`]). A blow whose wound is deep enough
//! bleeds on a victim whose race bleeds, seven body ticks later; at the
//! blow it sprays a jet of blood and the weapon that dealt it draws blood
//! ([`wounds`]). An armed bite above 0.1 sticks the weapon in the part
//! ([`bite`]).

pub mod attack;
pub mod bite;
pub mod collapse;
pub mod condition;
pub mod damage;
pub mod fall;
pub mod fighter;
pub mod grip;
pub mod regen;
pub mod stance;
pub mod swing;
pub mod wounds;

use std::collections::HashSet;

use attack::{AttackMotion, Attacker, Slot};
use bevy::prelude::*;
pub use condition::{Condition, StateFlags};
use damage::{Strike, WeaponRecord};
use exa_anim::{Animator, BodyPart, Ease, Motions};
use exa_physics::body::BODY_HZ;
use exa_physics::body::{
    BodyTicks, CharacterStage, CharacterTick, Fallen, GoLimp, MAX_DAZE, Ragdoll, Shove, daze_energy,
};
use exa_world::PartBounds;
pub use fighter::{Damage, Fighter, Health, StoryMode, Taken, Weakened};
use swing::{Swing, SwingAt, SwingState};
use wounds::{BladeBlood, WoundQueue, frame_tetra, tetra_weights};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Hit>()
            .add_message::<Weakened>()
            .add_plugins((
                stance::StancePlugin,
                condition::ConditionPlugin,
                collapse::CollapsePlugin,
                regen::RegenerationPlugin,
            ))
            .add_message::<wounds::Bleed>()
            .add_message::<wounds::Spray>()
            // The attack branch of each character's tick: its attack stage,
            // after its posture stage (this tick's posture decision), before
            // its health bookkeeping and its clip's advance.
            .add_systems(
                CharacterTick,
                swing::run_swings
                    .in_set(CharacterStage::Attack)
                    .run_if(condition::ticked_lives)
                    .before(regen::keep_health),
            )
            .add_systems(
                PostUpdate,
                // Wounds struck this frame wait from the next tick on.
                (
                    wounds::count_down_wounds,
                    track_blades,
                    detect_hits,
                    react_to_hits,
                )
                    .chain()
                    .in_set(CombatSystems)
                    .after(TransformSystems::Propagate),
            );
    }
}

/// The combat's frame systems: wounds counted down, blades tracked, hits
/// found and dealt. In `PostUpdate`, after transforms propagate.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CombatSystems;

/// A striker's segment along its entity's local +Y: a held weapon's, on the
/// entity holding the weapon model, or a fist's, across the hand.
#[derive(Component, Debug, Clone)]
pub struct Blade {
    /// Fighter (body model root) wielding it.
    pub wielder: Entity,
    /// The held item it is: the first (a weapon, the right fist) or the
    /// second (the left fist).
    pub slot: Slot,
    /// Tip distance along local +Y (metres).
    pub length: f32,
    /// The weapon's stats.
    pub record: WeaponRecord,
    pub tip_last: Option<Vec3>,
    /// Grip and tip last frame, for sweeping the blade between frames.
    pub swept_from: Option<(Vec3, Vec3)>,
    pub tip_speed: f32,
    /// Tip velocity, m/s.
    pub tip_velocity: Vec3,
    /// Grip position last frame.
    pub grip_last: Vec3,
}

impl Blade {
    /// A blade at rest in a fighter's hand, its first held item.
    pub fn new(wielder: Entity, length: f32, record: WeaponRecord) -> Self {
        Self {
            wielder,
            slot: Slot::First,
            length,
            record,
            tip_last: None,
            swept_from: None,
            tip_speed: 0.0,
            tip_velocity: Vec3::ZERO,
            grip_last: Vec3::ZERO,
        }
    }

    /// The same striker as the held item in `slot`.
    pub fn in_slot(self, slot: Slot) -> Self {
        Self { slot, ..self }
    }
}

/// Positions a blade sweep is tested at between two frames.
const SWEEP_STEPS: usize = 6;

/// Attack in progress on a fighter: its motion, the fighter's weapon class
/// (which picks the hand that strikes) and which targets it has already
/// struck.
#[derive(Component, Debug)]
pub struct Attacking {
    pub motion: AttackMotion,
    pub weapon_class: u8,
    pub struck: HashSet<Entity>,
}

impl Attacking {
    pub fn new(motion: AttackMotion, weapon_class: u8) -> Self {
        Self {
            motion,
            weapon_class,
            struck: HashSet::new(),
        }
    }
}

impl Default for Attacking {
    /// A main-side attack with a one-handed weapon.
    fn default() -> Self {
        Self::new(AttackMotion::Main(0), 0x10)
    }
}

/// A weapon contact.
#[derive(Message, Debug, Clone)]
pub struct Hit {
    pub attacker: Entity,
    /// The body that struck: the blade (the entity holding the weapon
    /// model), or the fist.
    pub striker: Entity,
    pub target: Entity,
    pub bone: usize,
    pub point: Vec3,
    /// Velocity of the blade tip, m/s.
    pub velocity: Vec3,
    pub weapon: WeaponRecord,
    pub strike: Strike,
}

const PART_RADIUS: f32 = 0.16;
const MIN_HIT_SPEED: f32 = 1.5;
/// Horizontal knockback (m) when `impact` is over 0.5: 2·(20·strikes + 10)·impact
/// centimetres along the swing, flattened, plus 20 ticks of the chest's fast
/// velocity (m a tick), capped at `step_cm`. Strikes is 0 until that power exists.
pub fn knockback_offset(
    impact: f32,
    strikes: f32,
    swing: Vec3,
    chest_fast: Vec3,
    step_cm: f32,
) -> Option<Vec3> {
    if !impact.is_finite() || !step_cm.is_finite() || step_cm <= 0.0 || impact <= 0.5 {
        return None;
    }
    let along = swing.with_y(0.0);
    let dir = along.normalize_or_zero();
    let base = 2.0 * (20.0 * strikes.max(0.0) + 10.0) * impact;
    let mut push = dir * base + chest_fast.with_y(0.0) * (100.0 * 20.0);
    if !push.is_finite() {
        return None;
    }
    let len = push.length();
    if len > step_cm {
        push *= step_cm / len;
    }
    Some(push / 100.0)
}

/// Share of the blade's velocity a struck part takes on contact.
const CONTACT_SHARE: f32 = 0.1;
/// Impact above which a blow shoves the victim, and the shove per unit of
/// impact (the original's 10 cm/tick, in m/s).
const PUSH_FROM: f32 = 0.5;
const PUSH_PER_IMPACT: f32 = 10.0 * BODY_HZ / 100.0;
const CM_PER_M: f32 = 100.0;

/// A fist meets a part with its knuckles, which lie inside the hand's box;
/// its segment ends on the box's far face, so a contact there is measured
/// this far (cm) inside it (the original ignores contacts at or past a
/// striker's far end).
const KNUCKLES_INSIDE: f32 = 0.1;

/// What starts an attack: its motion, the blend the executor gives a stab
/// or an overhead, the fighter's weapon class, whether it has Remise, and
/// the game clock (ms).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackStart {
    pub motion: AttackMotion,
    pub blend: u32,
    pub weapon_class: u8,
    pub remise: bool,
    pub now_ms: u64,
}

impl AttackStart {
    /// `motion` from rest (no given blend) for a fighter of weapon class
    /// `weapon_class` without Remise.
    pub fn of(motion: AttackMotion, weapon_class: u8, now_ms: u64) -> Self {
        Self {
            motion,
            blend: 0,
            weapon_class,
            remise: false,
            now_ms,
        }
    }
}

/// Starts an attack on a fighter: its clip, ending in `stance`, and its
/// primary striker, the held item in the motion's slot
/// ([`attack::primary_slot`]) among `blades`. The clip starts from where
/// the body is, eased smoothly, with the original's blend
/// ([`attack::start_blend`]: none on the main side, round((20 − W19) × 0.6)
/// on the other, the executor's for a stab or an overhead). The swing
/// starts at full strength with its window shut ([`SwingState::start`]); a
/// side attack stamps its start.
#[allow(clippy::too_many_arguments)]
pub fn start_attack<'a>(
    commands: &mut Commands,
    fighter: Entity,
    anim: &mut Animator,
    attacker: &mut Attacker,
    swing: &mut SwingState,
    blades: impl IntoIterator<Item = &'a Blade>,
    start: AttackStart,
    stance: &str,
) {
    let motion = start.motion;
    attacker.primary = Some(attack::primary_slot(start.weapon_class, motion));
    let primary_blend = attacker.primary_record(fighter, blades).map(|r| r.blend);
    anim.play_once(motion.action(), stance);
    anim.blended(
        attack::start_blend(motion, primary_blend, start.blend),
        Ease::Smooth,
    );
    swing.start(motion.is_side(), start.remise);
    if motion.is_side() {
        attacker.side_started_ms = Some(start.now_ms);
    }
    commands
        .entity(fighter)
        .insert(Attacking::new(motion, start.weapon_class));
}

fn track_blades(time: Res<Time>, mut blades: Query<(&mut Blade, &GlobalTransform)>) {
    let dt = time.delta_secs().max(1e-4);
    for (mut blade, gt) in &mut blades {
        let tip = gt.transform_point(Vec3::Y * blade.length);
        blade.swept_from = blade.tip_last.map(|last| (blade.grip_last, last));
        blade.tip_velocity = blade.tip_last.map_or(Vec3::ZERO, |last| (tip - last) / dt);
        blade.tip_speed = blade.tip_velocity.length();
        blade.tip_last = Some(tip);
        blade.grip_last = gt.translation();
    }
}

fn detect_hits(
    motions: Option<Res<Motions>>,
    blades: Query<(Entity, &Blade, &GlobalTransform)>,
    mut attackers: Query<(&Animator, &mut Attacking, &SwingState)>,
    fighters: Query<(Entity, &Condition), With<Fighter>>,
    children: Query<&Children>,
    parts: Query<(&BodyPart, &GlobalTransform, Option<&PartBounds>)>,
    mut hits: MessageWriter<Hit>,
) {
    let Some(motions) = motions else { return };
    for (striker, blade, gt) in &blades {
        let Ok((anim, mut attack, swing)) = attackers.get_mut(blade.wielder) else {
            continue;
        };
        // A striker whose window is open: the held items (the fists only
        // when they are what is held).
        if !attack::is_striker(blade.record.armed(), attack.weapon_class)
            || !swing.open[blade.slot.index()]
            || blade.tip_speed < MIN_HIT_SPEED
        {
            continue;
        }
        // The swing clock: the clip's progress as the original measures it.
        let progress = motions
            .clip(anim, &anim.action)
            .map_or(0.0, |c| anim.clip_progress(c));
        let a = gt.translation();
        let b = gt.transform_point(Vec3::Y * blade.length);
        let (a0, b0) = blade.swept_from.unwrap_or((a, b));
        for (target, condition) in &fighters {
            if target == blade.wielder || condition.is_dead() || attack.struck.contains(&target) {
                continue;
            }
            let spheres: Vec<(usize, Vec3, f32)> = children
                .iter_descendants(target)
                .filter_map(|e| {
                    let (part, pgt, bounds) = parts.get(e).ok()?;
                    let (c, r) = match bounds {
                        // The enclosing sphere is generous; use most of it.
                        Some(b) => (pgt.transform_point(b.centre), b.radius * 0.75),
                        None => (pgt.translation(), PART_RADIUS),
                    };
                    Some((part.bone, c, r))
                })
                .collect();
            // The first position of the sweep since last frame that meets a
            // body part, and the part nearest the blade there.
            let contact = (1..=SWEEP_STEPS).find_map(|k| {
                let t = k as f32 / SWEEP_STEPS as f32;
                let (sa, sb) = (a0.lerp(a, t), b0.lerp(b, t));
                spheres
                    .iter()
                    .filter_map(|&(bone, c, r)| {
                        let p = closest_on_segment(sa, sb, c);
                        let d = p.distance(c);
                        // The point on the blade as it is now.
                        let along = sa.distance(p) / sa.distance(sb).max(1e-4);
                        (d < r).then_some((d / r, bone, a.lerp(b, along)))
                    })
                    .min_by(|x, y| x.0.total_cmp(&y.0))
                    .map(|(_, bone, p)| (bone, p))
            });
            if let Some((bone, point)) = contact {
                debug!(
                    "blade {a} -> {b} speed {:.2} contact bone {bone} at {point}",
                    blade.tip_speed
                );
                attack.struck.insert(target);
                // Where along the weapon it landed, and how fast that point
                // moved (the weapon turns about the grip).
                let mut along = gt.affine().inverse().transform_point3(point).y * CM_PER_M;
                if !blade.record.armed() {
                    along = along.min(blade.length * CM_PER_M - KNUCKLES_INSIDE);
                }
                let share = (along / (blade.length * CM_PER_M)).clamp(0.0, 1.0);
                let strike = Strike {
                    speed: blade.tip_speed * share * CM_PER_M / BODY_HZ,
                    stab: attack.motion == AttackMotion::Stab,
                    overhead: attack.motion == AttackMotion::Overhead,
                    clock: progress,
                    along,
                };
                hits.write(Hit {
                    attacker: blade.wielder,
                    striker,
                    target,
                    bone,
                    point,
                    velocity: blade.tip_velocity,
                    weapon: blade.record,
                    strike,
                });
            }
        }
    }
}

/// The original's 32-bit generator (armour coverage rolls, the dying
/// tick's heading, the fall scream): x ← x·0x8088405 + 1, a draw x·2⁻³².
#[derive(Default)]
pub struct Roll(u32);

impl Roll {
    /// The next draw, 0 ≤ x < 1.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(0x0808_8405).wrapping_add(1);
        (self.0 as f64 * 2f64.powi(-32)) as f32
    }
}

/// The looping stance a fighter returns to: the follow-up of the clip it is
/// playing once, else its moveset's idle.
pub(crate) fn stance_of(anim: &Animator) -> String {
    match &anim.then {
        Some(then) => then.clone(),
        None if anim.plays_default_alone() => "Idle".into(),
        None => "Cmbt - Idle".into(),
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn react_to_hits(
    mut commands: Commands,
    mut hits: MessageReader<Hit>,
    mut shoves: MessageWriter<Shove>,
    mut sprays: MessageWriter<wounds::Spray>,
    mut weakened: MessageWriter<Weakened>,
    mut fighters: Query<(
        Entity,
        &mut Fighter,
        &mut Condition,
        &mut StateFlags,
        &mut Animator,
        Has<Attacking>,
        Option<&mut WoundQueue>,
        Option<&mut Ragdoll>,
        Option<&mut SwingState>,
        Option<&Swing>,
    )>,
    strikers: Query<&GlobalTransform>,
    mut blades: Query<&mut BladeBlood>,
    ticks: Option<Res<BodyTicks>>,
    story: Option<Res<StoryMode>>,
    motions: Option<Res<Motions>>,
    mut roll: Local<Roll>,
) {
    let tick = ticks.as_ref().map_or(0, |t| t.0);
    let now = ticks.as_ref().map_or(0, |t| t.millis());
    let story = story.is_some_and(|s| s.0);
    for hit in hits.read() {
        let Ok((
            _,
            mut f,
            mut condition,
            mut flags,
            mut anim,
            attacking,
            queue,
            mut body,
            state,
            swing,
        )) = fighters.get_mut(hit.target)
        else {
            continue;
        };
        // Every weapon contact on a fighter starts its regeneration's ramp
        // again, whatever it deals.
        f.health.rate = 0.0;
        let Some(armour) = hit.bone.checked_sub(1).and_then(|i| f.armour.get(i)) else {
            continue;
        };
        let Some(mut outcome) = damage::resolve(
            &hit.weapon,
            &hit.strike,
            armour,
            hit.bone,
            f.health.max,
            f.readiness,
            roll.next(),
        ) else {
            continue;
        };
        outcome = damage::guard_outcome(outcome, f.guard);
        // A fighter struck mid-swing (attacking) has its swing stalled by
        // the daze, before the blow's damage as the original's daze runs
        // first: its stance returns when the stall ends.
        if *condition == Condition::ATTACKING
            && let (Some(mut state), Some(motions)) = (state, motions.as_deref())
        {
            let base = body
                .as_deref()
                .and_then(|b| b.drive.as_ref())
                .map_or(1.0, |d| d.race.clip_rate);
            if let Some(at) = SwingAt::of(now, &anim, motions, base) {
                let swing = swing.copied().unwrap_or(Swing::FISTS);
                let daze = outcome.daze.min(MAX_DAZE);
                let factor = swing::daze_factor(f.health.max, flags.has(StateFlags::POTION));
                if let Some(r) = state.daze_stall(&swing, &at, daze, factor) {
                    info!(
                        "{:?} struck mid-swing at {:.2} of its clip: stall {r:.3} a tick \
                         (now {:.3}), strength {:.2}, stance at {} ms",
                        hit.target, at.progress, state.stall, state.strength, state.end_ms
                    );
                }
            }
        }
        // The original's damage call, by the victim's rule bits and state
        // flags; a blow that leaves it low but alive runs the low-health
        // check.
        let rules = f.stats.rules;
        let taken = f.health.take(Damage::blow(&outcome), rules, *flags, story);
        if taken == Taken::Weakened {
            weakened.write(Weakened {
                fighter: hit.target,
            });
        }
        let killed = taken == Taken::Killed;
        // A deep enough wound on a race that bleeds sprays a jet at once
        // and waits on the victim to bleed, anchored to the striker; the
        // killing blow too. The weapon that dealt it draws blood at once,
        // even when the wound is dropped.
        if f.bleeds && wounds::bleeds(outcome.wound) {
            let strength = wounds::spray_strength(outcome.wound);
            sprays.write(wounds::Spray {
                body: hit.target,
                bone: hit.bone,
                contact: hit.point,
                swing: hit.velocity,
                droplets: wounds::jet_droplets(strength),
            });
            if let Some(mut queue) = queue {
                let weights = strikers
                    .get(hit.striker)
                    .ok()
                    .and_then(|g| tetra_weights(hit.point, &frame_tetra(&g.affine())));
                if queue.push(hit.striker, weights, hit.point, outcome.wound) {
                    debug!("wound R {:.3} queued on {:?}", outcome.wound, hit.target);
                } else {
                    info!(
                        "wound R {:.3} dropped: {:?} has seven waiting",
                        outcome.wound, hit.target
                    );
                }
            }
            if let Ok(mut blood) = blades.get_mut(hit.striker) {
                blood.draw(outcome.wound);
                info!(
                    "blade {:?} draws blood from wound R {:.3}: {:.3}",
                    hit.striker, outcome.wound, blood.0
                );
            }
        }
        // The blow pushes the struck part, shoves the body when heavy, and
        // dazes it.
        let along = hit.velocity.normalize_or_zero();
        let (fast, step_cm) = body
            .as_ref()
            .and_then(|b| b.drive.as_ref())
            .map(|d| {
                (
                    d.balance().fast_velocity(),
                    exa_physics::body::step_length(d.stats.scale, d.balance().extension()),
                )
            })
            .unwrap_or((Vec3::ZERO, exa_physics::body::step_length(1.0, 1.0)));
        let step = knockback_offset(outcome.impact, 0.0, hit.velocity, fast, step_cm);
        let push = if step.is_some() {
            0.0
        } else if outcome.impact > PUSH_FROM {
            PUSH_PER_IMPACT * outcome.impact
        } else {
            0.0
        };
        let velocity = hit.velocity * CONTACT_SHARE + along * push;
        shoves.write(Shove {
            body: hit.target,
            bone: hit.bone,
            velocity,
            daze: outcome.daze,
            step,
        });
        info!(
            "hit on {:?} bone {} at tick {tick} by {}, {:.0} cm, {:.1} cm/tick: impact {:.2}, \
             penetration {:.2} at readiness {:.2}, wound {:.3} (yellow {:.3}, red {:.3} of {:.2})",
            hit.target,
            hit.bone,
            if hit.weapon.armed() {
                "a weapon"
            } else {
                "a fist"
            },
            hit.strike.along,
            hit.strike.speed,
            outcome.impact,
            outcome.penetration,
            f.readiness,
            outcome.wound,
            f.health.yellow,
            f.health.red,
            f.health.max
        );
        if killed && !condition.dying() {
            // A lethal wound starts the collapse on a living body, with the
            // energy the blow's daze leaves it (dealt at the next tick);
            // one without muscles dies at once.
            let living = body.as_deref().and_then(|b| b.drive.as_ref());
            if let Some(energy) = living.map(|d| daze_energy(d.energy, outcome.daze)) {
                collapse::start_collapse(
                    &mut condition,
                    &mut flags,
                    &mut anim,
                    body.as_deref_mut(),
                    energy,
                );
                commands
                    .entity(hit.target)
                    .insert(collapse::Collapse::default())
                    .remove::<Fallen>();
                info!(
                    "fighter {:?} collapses at energy {energy:.3}: Fall Backward, {} frames of blend",
                    hit.target,
                    collapse::collapse_blend(energy)
                );
            } else {
                condition.die();
                info!("fighter {:?} is down", hit.target);
                commands.entity(hit.target).insert(GoLimp {
                    velocity: None,
                    delay: 0,
                });
            }
        }
        // A blow that does not kill starts no clip on the struck fighter:
        // its stance changes come from its balance, not from the blow. A
        // killing blow ends its attack.
        if attacking && killed {
            commands.entity(hit.target).remove::<Attacking>();
        }
    }
    for (e, _, _, _, anim, attacking, ..) in &fighters {
        if attacking && anim.looping {
            commands.entity(e).remove::<Attacking>();
        }
    }
}

fn closest_on_segment(a: Vec3, b: Vec3, p: Vec3) -> Vec3 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
    a + ab * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;
    use damage::{Outcome, PartArmour};
    use exa_formats::character_stats::CharacterStats;
    use exa_formats::races::{Natural, Race};
    use fighter::failing;

    #[test]
    fn segment_projection() {
        let p = closest_on_segment(Vec3::ZERO, Vec3::Y, Vec3::new(1.0, 0.5, 0.0));
        assert_eq!(p, Vec3::new(0.0, 0.5, 0.0));
        let end = closest_on_segment(Vec3::ZERO, Vec3::Y, Vec3::new(0.0, 3.0, 0.0));
        assert_eq!(end, Vec3::Y);
    }

    /// An app running only the blow reactions.
    fn reactions() -> App {
        let mut app = App::new();
        app.add_message::<Hit>()
            .add_message::<Shove>()
            .add_message::<wounds::Spray>()
            .add_message::<Weakened>()
            .add_systems(Update, react_to_hits);
        app
    }

    /// The damage tests' sword cut: 20 cm a tick, 70 cm along the blade,
    /// late in the swing.
    fn cut() -> Strike {
        Strike {
            speed: 20.0,
            stab: false,
            overhead: false,
            clock: 0.8,
            along: 70.0,
        }
    }

    /// What the cut does to bare flesh on `bone` of a human, with the
    /// reactions' first coverage roll.
    fn outcome(bone: usize) -> Outcome {
        let roll = Roll::default().next();
        damage::resolve(
            &damage::sword_a(),
            &cut(),
            &PartArmour::default(),
            bone,
            0.25,
            0.0,
            roll,
        )
        .unwrap()
    }

    /// Cuts `target` on `bone` and runs one update.
    fn strike(app: &mut App, target: Entity, bone: usize) {
        app.world_mut().write_message(Hit {
            attacker: Entity::PLACEHOLDER,
            striker: Entity::PLACEHOLDER,
            target,
            bone,
            point: Vec3::ZERO,
            velocity: Vec3::new(3.0, 0.0, 0.0),
            weapon: damage::sword_a(),
            strike: cut(),
        });
        app.update();
    }

    fn condition(app: &App, fighter: Entity) -> Condition {
        *app.world().get::<Condition>(fighter).unwrap()
    }

    fn shoves(app: &App) -> Vec<Shove> {
        let messages = app.world().resource::<Messages<Shove>>();
        messages.get_cursor().read(messages).copied().collect()
    }

    /// Impact over one half steps 2·(20·strikes + 10)·J cm along the swing,
    /// plus 20 ticks of chest velocity, capped at the step length. Half
    /// does not. Strikes 0 is 20·J cm.
    #[test]
    fn a_heavy_blow_steps_back_by_its_impact() {
        assert!(knockback_offset(0.5, 0.0, Vec3::Z, Vec3::ZERO, 80.0).is_none());
        let plain = knockback_offset(1.0, 0.0, Vec3::Z, Vec3::ZERO, 80.0).unwrap();
        assert!((plain - Vec3::Z * 0.2).length() < 1e-5, "{plain}");
        let struck = knockback_offset(1.0, 1.0, Vec3::Z, Vec3::ZERO, 80.0).unwrap();
        assert!((struck - Vec3::Z * 0.6).length() < 1e-5, "{struck}");
        let moving = knockback_offset(1.0, 0.0, Vec3::Z, Vec3::Z * 0.01, 80.0).unwrap();
        assert!((moving - Vec3::Z * 0.4).length() < 1e-5, "{moving}");
        let capped = knockback_offset(1.0, 0.0, Vec3::Z, Vec3::Z, 80.0).unwrap();
        assert!((capped.length() - 0.8).abs() < 1e-5, "{capped}");
        let light = knockback_offset(0.4, 0.0, Vec3::Z, Vec3::ZERO, 80.0);
        assert!(light.is_none());
    }

    /// The same cut that wounds bare flesh deals nothing through a parry, and
    /// only the shield's yellow leak when the shield arm is tired.
    #[test]
    fn a_parry_or_a_block_changes_the_blow() {
        let open = outcome(9);
        assert!(open.wound > 0.0 && open.yellow > 0.0, "{open:?}");
        let parried = damage::guard_outcome(open, damage::Guard::Parry);
        assert_eq!(
            (parried.wound, parried.yellow, parried.red),
            (0.0, 0.0, 0.0)
        );
        let leak = 1.0 - 0.5 * 0.58;
        let blocked = damage::guard_outcome(open, damage::Guard::Shield { leak, stamina: 0.0 });
        let q = (0.4 + 0.4 * open.impact) * leak;
        assert!((blocked.yellow - 0.05 * q).abs() < 1e-5, "{blocked:?}");
        assert_eq!(blocked.wound, 0.0);
        assert!(blocked.yellow < open.yellow, "{blocked:?} {open:?}");

        let mut app = reactions();
        let guarding = Fighter {
            guard: damage::Guard::Parry,
            ..Fighter::default()
        };
        let id = app
            .world_mut()
            .spawn((guarding, Animator::new("Default", "Idle")))
            .id();
        strike(&mut app, id, 9);
        let health = app.world().get::<Fighter>(id).unwrap().health;
        assert_eq!(health.yellow, 0.25);
        assert_eq!(health.red, 0.25);
    }

    /// A sword cut that wounds but does not kill starts no clip: a bare
    /// forearm (13), chest (9), head (10) or foot (4) of a human in its
    /// stance, each over the 0.2 of penetration and impact that used to
    /// start Head Pain or Rebalance, leaves it in its idle, shoved once on
    /// that part. One getting up keeps playing its get-up.
    #[test]
    fn a_wound_starts_no_clip() {
        for bone in [13, 9, 10, 4] {
            let o = outcome(bone);
            assert!(o.penetration + o.impact > 0.2, "bone {bone}: {o:?}");
            let mut health = Health::of_race(1.0);
            let rules = Fighter::default().stats.rules;
            let taken = health.take(Damage::blow(&o), rules, StateFlags::NONE, false);
            assert_ne!(taken, Taken::Killed, "bone {bone}: the cut kills");
            let mut app = reactions();
            let fighter = app
                .world_mut()
                .spawn((Fighter::default(), Animator::new("Default", "Idle")))
                .id();
            strike(&mut app, fighter, bone);
            let anim = app.world().get::<Animator>(fighter).unwrap();
            assert_eq!(
                (anim.action.as_str(), anim.looping, anim.then.as_deref()),
                ("Idle", true, None),
                "bone {bone}"
            );
            assert_eq!(anim.time, 0.0, "bone {bone}");
            let f = app.world().get::<Fighter>(fighter).unwrap();
            assert!(f.health.yellow < 0.25, "bone {bone}: {:?}", f.health);
            assert!(!failing(f.health.yellow), "bone {bone}");
            assert_eq!(condition(&app, fighter), Condition::NORMAL, "bone {bone}");
            assert_eq!(f.health, health, "bone {bone}");
            let s = shoves(&app);
            assert_eq!(s.len(), 1, "bone {bone}");
            assert_eq!((s[0].body, s[0].bone), (fighter, bone));
            assert_eq!(s[0].daze, o.daze, "bone {bone}");
            let expect = knockback_offset(
                o.impact,
                0.0,
                Vec3::new(3.0, 0.0, 0.0),
                Vec3::ZERO,
                exa_physics::body::step_length(1.0, 1.0),
            );
            assert!(o.impact > 0.5, "bone {bone}: impact {}", o.impact);
            assert_eq!(s[0].step, expect, "bone {bone}");
            assert!(app.world().get::<GoLimp>(fighter).is_none());
        }
        let mut app = reactions();
        let mut anim = Animator::new("Default", "Idle");
        anim.play_once("Get up", "Idle");
        let fighter = app.world_mut().spawn((Fighter::default(), anim)).id();
        strike(&mut app, fighter, 13);
        let anim = app.world().get::<Animator>(fighter).unwrap();
        assert_eq!(
            (anim.action.as_str(), anim.looping, anim.then.as_deref()),
            ("Get up", false, Some("Idle"))
        );
    }

    /// A fighter cut on the forearm early in its attack (12 of 100 frames)
    /// plays on: its attack and clip stay, and the daze stalls its swing
    /// by 4·D (its striker still, f = 1 for a human), its stance due back
    /// round(80/stall) ms later, within the swing's expected time.
    #[test]
    fn a_wound_stalls_a_swing() {
        let mut app = reactions();
        app.insert_resource(punches());
        let mut anim = Animator::new("Default", "Cmbt - Idle");
        anim.play_once("Attack Mid", "Cmbt - Idle");
        anim.time = 12.0;
        let fighter = app
            .world_mut()
            .spawn((
                Fighter::default(),
                anim,
                Attacking::default(),
                Condition::ATTACKING,
            ))
            .id();
        strike(&mut app, fighter, 13);
        assert!(app.world().get::<Attacking>(fighter).is_some());
        let anim = app.world().get::<Animator>(fighter).unwrap();
        assert_eq!(
            (anim.action.as_str(), anim.looping, anim.time),
            ("Attack Mid", false, 12.0)
        );
        let daze = outcome(13).daze;
        assert!((daze - 0.428_3).abs() < 1e-3, "{daze}");
        let state = app.world().get::<SwingState>(fighter).unwrap();
        assert!(state.has(swing::flags::STALLING) && !state.has(swing::flags::RELEASED));
        assert!((state.stall - 4.0 * daze).abs() < 1e-6);
        assert_eq!(state.end_ms, (80.0 / state.stall).round_ties_even() as u64);
        assert_eq!(state.end_ms, 47);
        assert_eq!(condition(&app, fighter), Condition::ATTACKING);
        assert_eq!(shoves(&app).len(), 1);
        // Not attacking (its condition otherwise), a cut stalls nothing.
        let mut app = reactions();
        app.insert_resource(punches());
        let mut anim = Animator::new("Default", "Cmbt - Idle");
        anim.play_once("Attack Mid", "Cmbt - Idle");
        let fighter = app
            .world_mut()
            .spawn((Fighter::default(), anim, Attacking::default()))
            .id();
        strike(&mut app, fighter, 13);
        let state = app.world().get::<SwingState>(fighter).unwrap();
        assert!(!state.has(swing::flags::STALLING));
    }

    /// A cut that takes yellow health under 0.01 starts a living body's
    /// collapse: collapsing, Fall Backward blending round((1.2 − E) × 300)
    /// frames from the energy the blow's daze leaves, its attack ended, its
    /// fall (and get-up) over, no longer on its controller, not limp yet.
    #[test]
    fn a_killing_wound_starts_the_collapse() {
        for attacking in [false, true] {
            let mut app = reactions();
            let mut fighter = Fighter::default();
            fighter.health.yellow = 0.011;
            let mut anim = Animator::new("One Handed", "Cmbt - Idle");
            let body = Ragdoll::new(&[]).with_drive();
            let id = app.world_mut().spawn((fighter, body, Fallen)).id();
            if attacking {
                anim.play_once("Attack Mid", "Cmbt - Idle");
                app.world_mut().entity_mut(id).insert(Attacking::default());
            }
            app.world_mut().entity_mut(id).insert(anim);
            strike(&mut app, id, 9);
            assert_eq!(condition(&app, id), Condition::COLLAPSING);
            let daze = shoves(&app)[0].daze;
            let left = daze_energy(1.0, daze);
            assert!(left < 1.0, "the cut dazes: {daze}");
            let anim = app.world().get::<Animator>(id).unwrap();
            assert_eq!(
                (anim.action.as_str(), anim.looping, anim.ease),
                (collapse::FALL_BACKWARD, true, Ease::Smooth)
            );
            assert_eq!(anim.blend, collapse::collapse_blend(left) as f32);
            assert!(anim.blend > 60.0);
            assert!(app.world().get::<collapse::Collapse>(id).is_some());
            assert!(app.world().get::<GoLimp>(id).is_none());
            assert!(app.world().get::<Attacking>(id).is_none());
            assert!(app.world().get::<Fallen>(id).is_none());
            let body = app.world().get::<Ragdoll>(id).unwrap();
            assert!(!body.drive.as_ref().unwrap().supported);
            // A second cut on the collapsing body changes nothing of it.
            strike(&mut app, id, 9);
            let again = app.world().get::<Animator>(id).unwrap();
            assert_eq!(again.blend, collapse::collapse_blend(left) as f32);
            assert_eq!(condition(&app, id), Condition::COLLAPSING);
        }
    }

    /// A killing cut on a fighter without muscles (no living body) kills
    /// it at once: dead, asked to go limp keeping its motion, its attack
    /// ended.
    #[test]
    fn a_killing_wound_without_muscles_kills_at_once() {
        for attacking in [false, true] {
            let mut app = reactions();
            let mut fighter = Fighter::default();
            fighter.health.yellow = 0.011;
            let mut anim = Animator::new("One Handed", "Cmbt - Idle");
            let id = if attacking {
                anim.play_once("Attack Mid", "Cmbt - Idle");
                app.world_mut()
                    .spawn((fighter, anim, Attacking::default()))
                    .id()
            } else {
                app.world_mut().spawn((fighter, anim)).id()
            };
            strike(&mut app, id, 9);
            let f = app.world().get::<Fighter>(id).unwrap();
            assert_eq!(f.health.yellow, 0.0, "{:?}", f.health);
            assert_eq!(condition(&app, id), Condition::DEAD);
            let limp = app.world().get::<GoLimp>(id).unwrap();
            assert_eq!((limp.velocity, limp.delay), (None, 0));
            assert!(app.world().get::<Attacking>(id).is_none());
            assert_eq!(shoves(&app).len(), 1);
        }
    }

    fn weakened(app: &App) -> Vec<Entity> {
        let messages = app.world().resource::<Messages<Weakened>>();
        messages
            .get_cursor()
            .read(messages)
            .map(|w| w.fighter)
            .collect()
    }

    /// The cut deals its damage by the victim's rule bits: a feral human's
    /// red health is untouched and its yellow takes the red share too, as
    /// `Health::take` gives it; a human's red drops.
    #[test]
    fn the_victims_rule_bits_decide_the_damage() {
        let o = outcome(9);
        for feral in [0.0, 0.5] {
            let record = exa_formats::characters::CharacterRecord {
                feral,
                ..Default::default()
            };
            let spawn = exa_formats::character_stats::Spawn::default();
            let stats = CharacterStats::derive(&record, None, spawn);
            let mut expected = Fighter::of_stats(None, stats).health;
            expected.take(Damage::blow(&o), stats.rules, StateFlags::NONE, false);
            let mut app = reactions();
            let id = app
                .world_mut()
                .spawn((
                    Fighter::of_stats(None, stats),
                    Animator::new("Default", "Idle"),
                ))
                .id();
            strike(&mut app, id, 9);
            let f = app.world().get::<Fighter>(id).unwrap();
            assert_eq!(f.health, expected, "feral {feral}");
            if feral > 0.0 {
                assert_eq!(stats.rules.0, 0x8);
                assert_eq!(f.health.red, 0.25);
            } else {
                assert!(f.health.red < 0.25);
            }
        }
    }

    /// A cut that leaves a fighter's yellow under 0.05 without killing it
    /// reports the low-health check; one that leaves more, or kills, does
    /// not.
    #[test]
    fn a_low_blow_weakens_the_fighter() {
        let o = outcome(13);
        let after = |yellow: f32| {
            let mut h = Health {
                yellow,
                ..Health::of_race(1.0)
            };
            let rules = Fighter::default().stats.rules;
            (h.take(Damage::blow(&o), rules, StateFlags::NONE, false), h)
        };
        // Yellow chosen so the cut leaves 0.03: weakened.
        let low = 0.1 * o.yellow + 0.03 * 0.25 / (0.25 - 0.2 * o.red);
        assert_eq!(after(low).0, Taken::Weakened, "{:?}", after(low).1);
        for (yellow, weakens) in [(0.25, false), (low, true), (0.011, false)] {
            let mut app = reactions();
            let mut fighter = Fighter::default();
            fighter.health.yellow = yellow;
            let id = app
                .world_mut()
                .spawn((fighter, Animator::new("Default", "Idle")))
                .id();
            strike(&mut app, id, 13);
            let expected = if weakens { vec![id] } else { vec![] };
            assert_eq!(weakened(&app), expected, "yellow {yellow}");
        }
    }

    /// In the story a killing cut also takes a tenth of the maximum from
    /// red health; without the story mode it does not.
    #[test]
    fn a_killing_cut_in_the_story_costs_red() {
        let o = outcome(9);
        for story in [false, true] {
            let mut app = reactions();
            app.insert_resource(StoryMode(story));
            let mut fighter = Fighter::default();
            fighter.health.yellow = 0.011;
            let id = app
                .world_mut()
                .spawn((fighter, Animator::new("Default", "Idle")))
                .id();
            strike(&mut app, id, 9);
            let f = app.world().get::<Fighter>(id).unwrap();
            let cut = ((0.25f64 - f64::from(o.red) * 0.2) as f32) as f64;
            let red = if story {
                (cut - 0.025) as f32
            } else {
                cut as f32
            };
            assert_eq!((f.health.yellow, f.health.red), (0.0, red), "story {story}");
            assert_eq!(condition(&app, id), Condition::DEAD);
            assert!(weakened(&app).is_empty());
        }
    }

    /// An app running the wound countdown and the blow reactions, with a
    /// body tick clock.
    fn bleeding() -> App {
        let mut app = App::new();
        app.add_message::<Hit>()
            .add_message::<Shove>()
            .add_message::<wounds::Bleed>()
            .add_message::<wounds::Spray>()
            .add_message::<Weakened>()
            .init_resource::<exa_physics::body::BodyTicks>()
            .add_systems(Update, (wounds::count_down_wounds, react_to_hits).chain());
        app.update();
        app
    }

    /// The cut on `bone` of `target`, dealt by `striker` at `point`.
    fn cut_by(app: &mut App, striker: Entity, target: Entity, bone: usize, point: Vec3) {
        app.world_mut().write_message(Hit {
            attacker: Entity::PLACEHOLDER,
            striker,
            target,
            bone,
            point,
            velocity: Vec3::new(3.0, 0.0, 0.0),
            weapon: damage::sword_a(),
            strike: cut(),
        });
    }

    /// One body tick and the frame after it; the wounds that bled.
    fn tick(app: &mut App) -> Vec<wounds::Bleed> {
        app.world_mut()
            .resource_mut::<exa_physics::body::BodyTicks>()
            .0 += 1;
        app.update();
        let messages = app.world().resource::<Messages<wounds::Bleed>>();
        messages.iter_current_update_messages().copied().collect()
    }

    fn waiting(app: &App, fighter: Entity) -> usize {
        app.world()
            .get::<WoundQueue>(fighter)
            .unwrap()
            .pending()
            .len()
    }

    /// A human cut on the bare chest (R = 0.457, over 0.05) bleeds seven
    /// body ticks later, not before, with the radius 30·R + 10 cm (23.7 cm)
    /// and the travel of the blade's struck point since the blow.
    #[test]
    fn a_wound_bleeds_seven_ticks_after_the_blow() {
        let o = outcome(9);
        assert!((o.wound - 0.457).abs() < 1e-3, "{o:?}");
        assert!((o.red - 0.75 * o.wound).abs() < 1e-6);
        let mut app = bleeding();
        let fighter = app
            .world_mut()
            .spawn((Fighter::default(), Animator::new("Default", "Idle")))
            .id();
        // The blade lies along +Y from its grip, turned; the blow lands
        // 70 cm along it.
        let blade = Transform::from_xyz(0.2, 1.0, 0.5).with_rotation(Quat::from_rotation_z(-1.2));
        let striker = app.world_mut().spawn(GlobalTransform::from(blade)).id();
        let contact = blade.transform_point(Vec3::Y * 0.7);
        cut_by(&mut app, striker, fighter, 9, contact);
        app.update();
        let queued = app.world().get::<WoundQueue>(fighter).unwrap().pending()[0];
        assert_eq!((queued.striker, queued.delay), (striker, 7));
        assert_eq!((queued.contact, queued.size), (contact, o.wound));
        // The blade swings on: by the 7th tick it has turned further and
        // dropped 10 cm.
        let on = blade
            .with_rotation(Quat::from_rotation_z(-1.6))
            .with_translation(Vec3::new(0.2, 0.9, 0.5));
        *app.world_mut().get_mut::<GlobalTransform>(striker).unwrap() = on.into();
        for t in 1..7 {
            assert!(tick(&mut app).is_empty(), "tick {t}");
        }
        let bled = tick(&mut app);
        assert_eq!(bled.len(), 1);
        let b = bled[0];
        assert_eq!((b.body, b.contact), (fighter, contact));
        assert!((b.radius - (0.3 * o.wound + 0.1)).abs() < 1e-6);
        assert!((b.radius - 0.237).abs() < 1e-3);
        let moved = on.transform_point(Vec3::Y * 0.7) - contact;
        assert!(b.travel.distance(moved) < 1e-4, "{:?} {moved:?}", b.travel);
        assert_eq!(waiting(&app, fighter), 0);
        assert!(tick(&mut app).is_empty());
    }

    /// Nothing bleeds on a race that does not: a skeleton (flags 0), cut as
    /// deep, keeps no wound; a human of the same record bleeds.
    #[test]
    fn skeletons_do_not_bleed() {
        let race = |flags: u32| Race {
            id: 2,
            name: "Skeleton".into(),
            flags,
            stem: "Skel".into(),
            movesets: ["Default".into(), "Skeleton".into()],
            health: 1.0,
            natural: Natural::default(),
            body: exa_formats::races::RaceBody::FALLBACK,
            stats: exa_formats::races::RaceStats::FALLBACK,
        };
        let of = |r: &Race| Fighter::of_stats(Some(r), CharacterStats::default());
        let skeleton = of(&race(0));
        assert!(!skeleton.bleeds);
        assert!(of(&race(0x0F)).bleeds);
        assert!(Fighter::default().bleeds);
        let mut app = bleeding();
        let bones = app
            .world_mut()
            .spawn((skeleton, Animator::new("Default", "Idle")))
            .id();
        cut_by(&mut app, Entity::PLACEHOLDER, bones, 9, Vec3::ZERO);
        app.update();
        assert!(app.world().get::<Fighter>(bones).unwrap().health.red < 0.25);
        assert_eq!(waiting(&app, bones), 0);
        for _ in 0..8 {
            assert!(tick(&mut app).is_empty());
        }
    }

    /// The threshold is on R, before the red bar's 0.75: a slow cut on
    /// the bare hand at 4 cm a tick (R 0.045) does not bleed, at 6 cm a
    /// tick (R 0.064, red 0.048) it does.
    #[test]
    fn shallow_wounds_do_not_bleed() {
        for (speed, size, bleeds) in [(4.0, 0.0454, false), (6.0, 0.0640, true)] {
            let strike = Strike { speed, ..cut() };
            let roll = Roll::default().next();
            let o = damage::resolve(
                &damage::sword_a(),
                &strike,
                &PartArmour::default(),
                17,
                0.25,
                0.0,
                roll,
            )
            .unwrap();
            assert!((o.wound - size).abs() < 1e-4, "{o:?}");
            assert!(f64::from(o.red) < wounds::BLEED_FROM);
            let mut app = bleeding();
            let id = app
                .world_mut()
                .spawn((Fighter::default(), Animator::new("Default", "Idle")))
                .id();
            app.world_mut().write_message(Hit {
                attacker: Entity::PLACEHOLDER,
                striker: Entity::PLACEHOLDER,
                target: id,
                bone: 17,
                point: Vec3::ZERO,
                velocity: Vec3::X,
                weapon: damage::sword_a(),
                strike,
            });
            app.update();
            assert_eq!(waiting(&app, id), usize::from(bleeds), "{speed} cm/tick");
        }
    }

    /// The killing blow bleeds on the corpse; a wound struck while seven
    /// wait is dropped.
    #[test]
    fn the_fallen_bleed_and_seven_wait_at_most() {
        let mut app = bleeding();
        let mut dying = Fighter::default();
        dying.health.yellow = 0.011;
        let id = app
            .world_mut()
            .spawn((dying, Animator::new("Default", "Idle")))
            .id();
        cut_by(&mut app, Entity::PLACEHOLDER, id, 9, Vec3::ZERO);
        app.update();
        assert_eq!(condition(&app, id), Condition::DEAD);
        let bled: Vec<_> = (0..7).flat_map(|_| tick(&mut app)).collect();
        assert_eq!(bled.len(), 1);
        assert_eq!((bled[0].body, bled[0].travel), (id, Vec3::ZERO));

        // A sturdy fighter takes nine cuts in one frame: seven wait, and
        // they bleed one a tick from the 7th tick on.
        let sturdy = app
            .world_mut()
            .spawn((
                Fighter {
                    health: Health::of_race(100.0),
                    ..Fighter::default()
                },
                Animator::new("Default", "Idle"),
            ))
            .id();
        for i in 0..9 {
            cut_by(&mut app, Entity::PLACEHOLDER, sturdy, 9, Vec3::X * i as f32);
        }
        app.update();
        assert_eq!(waiting(&app, sturdy), 7);
        let per_tick: Vec<Vec<Vec3>> = (0..14)
            .map(|_| tick(&mut app).iter().map(|b| b.contact).collect())
            .collect();
        for (t, contacts) in per_tick.iter().enumerate() {
            match t {
                0..6 => assert!(contacts.is_empty(), "tick {}", t + 1),
                6..13 => assert_eq!(contacts, &[Vec3::X * (t - 6) as f32], "tick {}", t + 1),
                _ => assert!(contacts.is_empty()),
            }
        }
    }

    /// The blade that deals a bleeding wound draws half its R at the blow:
    /// 0.2285 from the chest cut (R 0.457), again when the wound is dropped
    /// from a full queue; nothing from a wound that does not bleed (a
    /// skeleton's, or R 0.045 on a hand), and a fist has no blood to draw.
    #[test]
    fn blades_draw_blood_from_bleeding_wounds() {
        let mut app = bleeding();
        let human = app
            .world_mut()
            .spawn((
                Fighter {
                    health: Health::of_race(100.0),
                    ..Fighter::default()
                },
                Animator::new("Default", "Idle"),
            ))
            .id();
        let sword = app
            .world_mut()
            .spawn((GlobalTransform::IDENTITY, BladeBlood::default()))
            .id();
        let blood = |app: &App| app.world().get::<BladeBlood>(sword).unwrap().0;
        cut_by(&mut app, sword, human, 9, Vec3::ZERO);
        app.update();
        // At the blow, before the wound bleeds seven ticks later.
        assert_eq!(waiting(&app, human), 1);
        assert!((blood(&app) - 0.5 * outcome(9).wound).abs() < 1e-6);
        assert!((blood(&app) - 0.2285).abs() < 1e-3);
        // Seven more in one frame: six wait (one is waiting already), two
        // are dropped, and every one adds its blood, up to 1.
        for _ in 0..7 {
            cut_by(&mut app, sword, human, 9, Vec3::ZERO);
        }
        app.update();
        assert_eq!(waiting(&app, human), 7);
        assert_eq!(blood(&app), 1.0);

        let mut app = bleeding();
        let skeleton = app
            .world_mut()
            .spawn((
                Fighter {
                    bleeds: false,
                    ..Fighter::default()
                },
                Animator::new("Default", "Idle"),
            ))
            .id();
        let human = app
            .world_mut()
            .spawn((Fighter::default(), Animator::new("Default", "Idle")))
            .id();
        let sword = app
            .world_mut()
            .spawn((GlobalTransform::IDENTITY, BladeBlood::default()))
            .id();
        cut_by(&mut app, sword, skeleton, 9, Vec3::ZERO);
        app.world_mut().write_message(Hit {
            attacker: Entity::PLACEHOLDER,
            striker: sword,
            target: human,
            bone: 17,
            point: Vec3::ZERO,
            velocity: Vec3::X,
            weapon: damage::sword_a(),
            strike: Strike {
                speed: 4.0,
                ..cut()
            },
        });
        app.update();
        assert_eq!(app.world().get::<BladeBlood>(sword).unwrap().0, 0.0);
        assert!(app.world().get::<Fighter>(human).unwrap().health.red < 0.25);
        // A fist: the struck bleeds, the hand draws nothing.
        let fist = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        cut_by(&mut app, fist, human, 9, Vec3::ZERO);
        app.update();
        assert_eq!(waiting(&app, human), 1);
        assert!(app.world().get::<BladeBlood>(fist).is_none());
    }

    /// The jets sent this frame.
    fn sprays(app: &App) -> Vec<wounds::Spray> {
        let messages = app.world().resource::<Messages<wounds::Spray>>();
        messages.iter_current_update_messages().copied().collect()
    }

    /// A bleeding blow sprays one jet at the blow, on the struck part at
    /// the contact point, against the swing: the chest cut (R 0.457) sends
    /// 16 droplets, a jet for every cut even when the wound is dropped from
    /// a full queue; a skeleton's cut and a cut on the hand at 4 cm a tick
    /// (R 0.045) spray nothing.
    #[test]
    fn bleeding_blows_spray_at_the_blow() {
        let mut app = bleeding();
        let human = app
            .world_mut()
            .spawn((
                Fighter {
                    health: Health::of_race(100.0),
                    ..Fighter::default()
                },
                Animator::new("Default", "Idle"),
            ))
            .id();
        let contact = Vec3::new(0.1, 1.3, 0.2);
        cut_by(&mut app, Entity::PLACEHOLDER, human, 9, contact);
        app.update();
        let jets = sprays(&app);
        assert_eq!(jets.len(), 1);
        let jet = jets[0];
        assert_eq!((jet.body, jet.bone, jet.contact), (human, 9, contact));
        assert_eq!(jet.swing, Vec3::new(3.0, 0.0, 0.0));
        assert_eq!(jet.droplets, 16);
        assert_eq!(
            jet.droplets,
            wounds::jet_droplets(wounds::spray_strength(outcome(9).wound))
        );
        // Not again on the ticks that follow, when the wound bleeds.
        let bled: Vec<_> = (0..7).flat_map(|_| tick(&mut app)).collect();
        assert_eq!(bled.len(), 1);
        assert!(sprays(&app).is_empty());
        // Eight cuts in a frame: seven wait, one is dropped, all spray.
        for _ in 0..8 {
            cut_by(&mut app, Entity::PLACEHOLDER, human, 9, contact);
        }
        app.update();
        assert_eq!(waiting(&app, human), 7);
        assert_eq!(sprays(&app).len(), 8);

        let skeleton = app
            .world_mut()
            .spawn((
                Fighter {
                    bleeds: false,
                    ..Fighter::default()
                },
                Animator::new("Default", "Idle"),
            ))
            .id();
        cut_by(&mut app, Entity::PLACEHOLDER, skeleton, 9, contact);
        app.world_mut().write_message(Hit {
            attacker: Entity::PLACEHOLDER,
            striker: Entity::PLACEHOLDER,
            target: human,
            bone: 17,
            point: contact,
            velocity: Vec3::X,
            weapon: damage::sword_a(),
            strike: Strike {
                speed: 4.0,
                ..cut()
            },
        });
        app.update();
        assert!(sprays(&app).is_empty());
    }

    /// A synthetic motion set: the "Default" moveset's main-side and
    /// other-side attacks, 100 frames each.
    fn punches() -> Motions {
        use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
        let key = |time| Key {
            time,
            root: [0.0; 3],
            rotations: Vec::new(),
            flags: Vec::new(),
        };
        let clip = Clip {
            mode: 0,
            keys: vec![key(0.0), key(100.0)],
        };
        let actions = [
            "Attack Mid",
            "Attack Combo 1",
            "Overhead Swing",
            "Cmbt - Idle",
        ];
        let set = MotionSet {
            bone_count: 0,
            actions: actions
                .iter()
                .map(|n| Action {
                    name: (*n).into(),
                    params: [0; 2],
                })
                .collect(),
            movesets: vec![Moveset {
                name: "Default".into(),
                parent: String::new(),
                clips: vec![Some(clip); actions.len()],
            }],
        };
        Motions::new(std::sync::Arc::new(set), std::sync::Arc::default(), 0.0)
    }

    /// The human fist's record, 17.66 cm across the hand.
    fn fist() -> WeaponRecord {
        use exa_formats::weapon_record::{Holder, Lengths, Section};
        WeaponRecord::build(
            &Section::FISTS,
            false,
            Lengths::hand(-8.0, 9.66),
            &Holder::AVERAGE,
        )
    }

    /// An unarmed puncher halfway through `motion` with both fists driven
    /// into a target's chest (a 20 cm sphere 1 m ahead, at chest height),
    /// knuckles first at 4 m/s; the hits detected.
    fn punch(motion: AttackMotion, weapon_class: u8) -> (Vec<Hit>, [Entity; 2]) {
        let mut app = App::new();
        app.add_message::<Hit>()
            .insert_resource(punches())
            .add_systems(Update, detect_hits);
        let mut anim = Animator::new("Default", "Cmbt - Idle");
        anim.play_once(motion.action(), "Cmbt - Idle");
        anim.time = 50.0;
        // Its window open half way through, as the swing routine opens it.
        let mut window = SwingState::default();
        let mut primary = None;
        let tick = swing::SwingTick {
            at: SwingAt {
                now_ms: 0,
                motion: Some(motion),
                progress: 0.6,
                last_key: 100.0,
                base: 1.0,
            },
            motion,
            feral: 0.0,
            energy: 1.0,
            weapon_class,
            strikers: [None; 2],
            aim: Vec3::Z,
        };
        window.tick(&Swing::FISTS, &tick, &mut primary);
        let puncher = app
            .world_mut()
            .spawn((
                Fighter::default(),
                anim,
                Attacking::new(motion, weapon_class),
            ))
            .id();
        *app.world_mut().get_mut::<SwingState>(puncher).unwrap() = window;
        let chest = Vec3::new(0.0, 1.4, 1.0);
        let target = app.world_mut().spawn(Fighter::default()).id();
        app.world_mut().spawn((
            BodyPart {
                bone: 9,
                rest: Transform::IDENTITY,
            },
            GlobalTransform::from_translation(chest),
            PartBounds {
                centre: Vec3::ZERO,
                radius: 0.2 / 0.75,
                half_extents: Vec3::splat(0.15),
            },
            ChildOf(target),
        ));
        // Each fist's segment runs from the wrist along +Z (local +Y turned
        // forward), its knuckles 5 cm into the chest's sphere.
        let length = fist().length / CM_PER_M;
        let fists = [(Slot::First, -0.05), (Slot::Second, 0.05)].map(|(slot, x)| {
            let wrist = Vec3::new(x, 1.4, 1.0 - 0.2 + 0.05 - length);
            let at = Transform::from_translation(wrist)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::Z));
            let mut blade = Blade::new(puncher, length, fist()).in_slot(slot);
            blade.tip_speed = 4.0;
            blade.tip_velocity = Vec3::Z * 4.0;
            app.world_mut()
                .spawn((blade, GlobalTransform::from(at)))
                .id()
        });
        app.update();
        let messages = app.world().resource::<Messages<Hit>>();
        (
            messages.iter_current_update_messages().cloned().collect(),
            fists,
        )
    }

    /// Unarmed, the main side and the overhead strike with the right fist
    /// only, the other side with the left; the struck point is the
    /// knuckles, 0.1 cm inside the hand's 17.66 cm, so the blow counts. A
    /// held weapon's class makes the fists no strikers at all.
    #[test]
    fn fists_strike_with_the_motions_hand() {
        for (motion, hand) in [
            (AttackMotion::Main(0), 0),
            (AttackMotion::Overhead, 0),
            (AttackMotion::Other(0), 1),
        ] {
            let (hits, fists) = punch(motion, 0x80);
            assert_eq!(hits.len(), 1, "{motion:?}");
            let hit = &hits[0];
            assert_eq!(hit.striker, fists[hand], "{motion:?}");
            assert_eq!(hit.bone, 9);
            assert!(!hit.weapon.armed());
            assert!(
                (hit.strike.along - (17.66 - 0.1)).abs() < 1e-3,
                "{:?}",
                hit.strike
            );
            assert_eq!(hit.strike.overhead, motion == AttackMotion::Overhead);
            let o = damage::resolve(
                &hit.weapon,
                &hit.strike,
                &PartArmour::default(),
                hit.bone,
                0.25,
                0.0,
                0.5,
            );
            assert!(o.is_some_and(|o| o.yellow > 0.0), "{o:?}");
        }
        let (hits, _) = punch(AttackMotion::Main(0), 0x10);
        assert!(hits.is_empty());
    }

    /// A punch bleeds on the one struck, never on the puncher's own fist or
    /// body.
    #[test]
    fn fists_stay_clean() {
        let mut app = bleeding();
        let puncher = app
            .world_mut()
            .spawn((Fighter::default(), Animator::new("Default", "Idle")))
            .id();
        let fist = app
            .world_mut()
            .spawn((GlobalTransform::from_xyz(0.0, 1.4, 0.4), ChildOf(puncher)))
            .id();
        let struck = app
            .world_mut()
            .spawn((Fighter::default(), Animator::new("Default", "Idle")))
            .id();
        cut_by(&mut app, fist, struck, 9, Vec3::new(0.0, 1.4, 0.5));
        app.update();
        assert_eq!((waiting(&app, struck), waiting(&app, puncher)), (1, 0));
        let bled: Vec<_> = (0..7).flat_map(|_| tick(&mut app)).collect();
        assert_eq!(bled.len(), 1);
        assert_eq!(bled[0].body, struck);
    }
}
