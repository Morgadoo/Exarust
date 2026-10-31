//! NPC behaviour. For now a duellist: once the fight starts it enters
//! combat in its guard and runs the original's combat state
//! ([`combat`]) once per body tick: the state's footwork search picks the
//! places it steps to (Exarust walks them in its combat walk), the blows
//! it starts and the stance it guards in, and turns its heading; each
//! tick the combatant's perception pass ([`combatant`]) keeps its contact
//! with the target. NPCs act through the same pieces as the player (body
//! velocity, animator, stance, attacks); the duellist does not release a
//! swing yet.
//!
//! A level's living characters carry a brain ([`brain`]): their roles
//! choose their idle activity, and one brain a tick, the one that has
//! waited longest ([`pass_turn`]), runs the target pass: its perception
//! fills the contacts it can sense and the pass decides whether it engages
//! the player (or leaves the fight) as the duellist does.
//!
//! All of it runs in the fixed step, once per body tick ([`BrainTick`]),
//! before the character controllers and the character ticks, as the
//! original calls its AI from every body tick.

use bevy::prelude::*;
use exa_anim::{Animator, Motions};
use exa_combat::attack::{self, AttackMotion, Attacker, Busy};
use exa_combat::stance::{self, Stance};
use exa_combat::swing::{self, Swing, SwingAt, SwingState};
use exa_combat::{AttackStart, Attacking, Blade, Condition, Fighter};
use exa_physics::CharacterBody;
use exa_physics::body::{BodyTicks, Fallen, MAX_ENERGY, Ragdoll};

use crate::chase::{ChaseBody, ChaseFoe, ContactKey};
use crate::combatant::{Attitude, Combatant, Observed, Observer, atan2_xz, perceive};
use crate::route::{NavRouter, PlanTurn};

pub mod activity;
pub mod behaviour;
pub mod brain;
pub mod cast;
pub mod chase;
pub mod combat;
pub mod combatant;
pub mod contacts;
pub mod entity;
pub mod estimate;
pub mod fight;
pub mod flee;
pub mod golem;
pub mod ground;
pub mod hear;
pub mod idle;
pub mod lifecycle;
pub mod lookup;
pub mod manager;
pub mod motion;
pub mod order;
pub mod pass_turn;
pub mod path;
pub mod personality;
pub mod place;
pub mod presence;
pub mod roles;
pub mod rooms;
pub mod route;
pub mod search;
pub mod sense;
pub mod struck;
pub mod walker;

pub use order::{Order, follow};
pub use sense::{Decision, Mind, decide, noticeability, tick, view_weight};

pub struct AiPlugin;

/// The duellists' combat state: a power system answers [`DuelPowers`]
/// before it and casts what it chose after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DuelSystems;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<presence::ActivationArea>()
            .init_resource::<rooms::RoomCells>()
            .init_resource::<brain::BrainClock>()
            .init_resource::<pass_turn::PassTurn>()
            .add_message::<presence::Entered>()
            .add_message::<brain::StatueByte>()
            .add_message::<exa_combat::AttackAskedHook>()
            .add_message::<exa_combat::BlowNoticed>()
            .add_message::<struck::Taught>()
            .add_systems(
                PostUpdate,
                struck::react_to_blows
                    .in_set(struck::HitReactions)
                    .after(exa_combat::CombatSystems),
            )
            .add_systems(
                FixedUpdate,
                presence::check_presence.before(exa_physics::body::CharacterTicks),
            )
            // The brains' tick: every body tick, after the presence check
            // and the daze's stance, before the posture, the character
            // controllers' move and the character ticks (the original calls
            // each brain's AI tick from its body's tick, after the daze and
            // before the falls, once the pre-tick hooks rotated the turns).
            .configure_sets(
                FixedUpdate,
                BrainTick
                    .in_set(exa_physics::body::TickBehaviour)
                    .after(presence::check_presence)
                    .after(exa_combat::stance::StanceTick)
                    .before(exa_combat::condition::PostureTick),
            )
            // The chain runs alone, on one thread, in its own schedule: its
            // systems run one after the other anyway, and the fixed step's
            // executor would add its dispatch and sync waits to each.
            .edit_schedule(BrainChain, |schedule| {
                schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
            })
            .add_systems(FixedUpdate, run_brain_chain.in_set(BrainTick))
            .add_systems(
                BrainChain,
                (
                    (brain::apply_new_roles, brain::rate_the_player).chain(),
                    brain_clock_mark(0),
                    pass_turn::rotate_pass_turn,
                    brain::perceive_all,
                    brain_clock_mark(1),
                    brain::ai_tick,
                    brain_clock_mark(2),
                    duellist.in_set(DuelSystems),
                    brain_clock_mark(3),
                    brain::flee_walk,
                    brain::face_entering,
                    brain::idle_walk,
                )
                    .chain(),
            )
            // The scene's forget broadcast stays a frame system: it reads
            // the frame's removals.
            .add_systems(Update, brain::forget_removed);
    }
}

/// A debug clock over the brains' tick: while this resource exists, the
/// wall time from the start to the end of each run of [`BrainTick`] is
/// added up, with the runs, and split at marks in the chain
/// ([`BrainTickClock::PARTS`]). The frame profiler reads and clears it each
/// frame.
#[derive(Resource, Debug, Default)]
pub struct BrainTickClock {
    pub spent: std::time::Duration,
    pub runs: u32,
    /// The time each part of the chain took, in [`BrainTickClock::PARTS`]'
    /// order.
    pub parts: [std::time::Duration; 5],
    began: Option<std::time::Instant>,
    last: Option<std::time::Instant>,
}

impl BrainTickClock {
    /// The parts the chain's marks split it into: the roles and the
    /// player's rating; the rotation and the perception; the AI tick (with
    /// the encounter scripts and the powers' answers, which the game runs
    /// around it); the duellists' combat state; the walks (the cast, the
    /// flight, the entering turn, the idle walk, the follow and search, the
    /// sentry).
    pub const PARTS: [&'static str; 5] = ["roles", "perception", "AI tick", "duellists", "walks"];

    fn mark(&mut self, part: usize) {
        let now = std::time::Instant::now();
        if let Some(last) = self.last {
            self.parts[part] += now - last;
        }
        self.last = Some(now);
    }
}

/// The schedule of one brains' tick, which [`BrainTick`] runs once per
/// body tick, on a single thread. Game systems that belong to the AI tick
/// are added to it, ordered against the chain's systems.
#[derive(bevy::ecs::schedule::ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BrainChain;

/// Runs the brains' tick ([`BrainChain`]), timed when the profiler keeps a
/// [`BrainTickClock`].
fn run_brain_chain(world: &mut World) {
    let began = world
        .contains_resource::<BrainTickClock>()
        .then(std::time::Instant::now);
    if let Some(mut clock) = world.get_resource_mut::<BrainTickClock>() {
        clock.began = began;
        clock.last = began;
    }
    world.run_schedule(BrainChain);
    if let Some(mut clock) = world.get_resource_mut::<BrainTickClock>() {
        clock.mark(4);
        if let Some(began) = clock.began.take() {
            clock.spent += began.elapsed();
            clock.runs += 1;
        }
        clock.last = None;
    }
}

/// A mark in the brain chain: the time since the last mark goes to `part`.
fn brain_clock_mark(part: usize) -> impl FnMut(Option<ResMut<BrainTickClock>>) {
    move |clock| {
        if let Some(mut clock) = clock {
            clock.mark(part);
        }
    }
}

/// The brains' tick in the fixed step, once per body tick: the system that
/// runs the [`BrainChain`] schedule, which holds the roles, the
/// target pass's turn ([`pass_turn::rotate_pass_turn`], the original's
/// pre-tick hook), the perception, the AI tick, the duellists' combat
/// state ([`DuelSystems`]) and the walks. It runs in
/// [`exa_physics::body::TickBehaviour`], after the presence check and the
/// daze's stance and before the posture, the character controllers, the
/// grips and the character ticks: the AI's part of the body tick's
/// behaviour stage, run over every brain ahead of the character ticks, as
/// the player's input is. Game systems that
/// belong to the AI tick (the follow and search, the sentries, the
/// encounter scripts, the duellists' powers) join the [`BrainChain`] schedule.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BrainTick;

/// What a duellist's power system answers the combat state, and the power
/// it chose. The game writes the answers before [`DuelSystems`] and casts
/// [`DuelPowers::chosen`] after it; a duellist without one casts nothing.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct DuelPowers {
    /// The record's ten power slots (0 empty).
    pub slots: [u32; cast::SLOTS],
    /// Each slot's word, as [`cast::Powers::query`] answers it.
    pub answers: [u32; cast::SLOTS],
    /// The focus the chooser's budget scales.
    pub pool: f32,
    /// A cast motion runs: the body's condition reads casting.
    pub casting: bool,
    /// The power system still runs the body's cast: its domain's evaluator
    /// answers the body's power id, or the hook keeps event 0x10.
    pub running: bool,
    /// The combat state asked [`DuelPowers::running`]: the game then calls
    /// the hook with 0x10 when the evaluator answered nothing, and ends the
    /// cast on a negative answer.
    pub probed: bool,
    /// The power the combat state chose, until the game casts it.
    pub chosen: Option<u32>,
}

impl cast::Powers for DuelPowers {
    fn query(&mut self, power: u32) -> u32 {
        self.slots
            .iter()
            .position(|&slot| slot == power)
            .map_or(cast::DENIED, |slot| self.answers[slot])
    }

    fn cast(&mut self, power: u32) {
        self.chosen = Some(power);
    }
}

/// A fighter (on its body model root, whose parent carries the
/// `CharacterBody`) that duels `target` (a world-space entity, e.g. the
/// player's physics root).
#[derive(Component, Debug, Clone)]
#[require(Stance, Combatant, presence::Presence)]
pub struct Duellist {
    pub target: Entity,
    /// The combat state's generator.
    pub roll: u32,
    /// The fight has started: until then the duellist stands idle.
    pub engaged: bool,
    /// It stands its ground: it faces its target and takes its stance, but
    /// neither steps nor strikes (a debug aid for still comparisons).
    pub holds_ground: bool,
    /// How long after the fight starts it waits before engaging, in
    /// milliseconds (the original's practice opponent waits three seconds).
    pub reaction_ms: u32,
    /// When it engages, set when the fight starts with a reaction delay.
    pub engage_at_ms: Option<u64>,
    /// It has entered combat (taken its guard).
    in_combat: bool,
    /// The combat state's fields.
    pub mind: combat::CombatMind,
    /// The last body tick the combat state ran for.
    last_tick: Option<u64>,
    /// The bearing the last estimate left: direction and distance (cm).
    bearing: (Vec3, f32),
    /// The step it is taking (Exarust's stand-in for the body's step).
    step: Option<Stepping>,
    /// A step queued after it, as an offset from the chest (cm).
    queued: Option<Vec3>,
    /// The footwork queue's state the body reports: −1 when the queued
    /// step was taken up.
    queue: i32,
    /// What the record gives the fighter, when no brain carries it.
    pub personality: Option<personality::Personality>,
    /// The chase's fields.
    pub chase: chase::ChaseMind,
    /// The chase route to walk (cm), as the planner returned it.
    route: Vec<Vec3>,
    /// The last combat tick ended at the chase, following the route.
    chasing: bool,
    /// The stab's aim, which the behaviour turns toward the foe while a
    /// stab plays and the body's stab drive pulls the weapon onto.
    stab_aim: exa_combat::stab::StabAim,
}

/// A step the duellist walks: where it goes (m) and when it started.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Stepping {
    goal: Vec3,
    started_ms: u32,
    /// 2 for the second half of a long step.
    state: i32,
}

impl Duellist {
    /// Enters combat at the next tick.
    pub fn engage(&mut self) {
        self.engaged = true;
    }

    /// It is in the combat state.
    pub fn fighting(&self) -> bool {
        self.in_combat
    }

    /// It walks a chase route: the last combat tick ended at the chase and
    /// a route the planner accepted is left to walk (the route-following
    /// flag the AI tick's footprint gate reads) [guess: the original's flag
    /// stays set until its walk state changes; Exarust's route is cleared
    /// on arrival and only walked while the tick ends at the chase].
    pub fn follows_route(&self) -> bool {
        self.chasing && !self.route.is_empty()
    }

    /// Leaves combat for the idle state.
    pub fn leave(&mut self) {
        self.engaged = false;
        self.in_combat = false;
    }

    /// A duellist of `target`, waiting for the fight to start.
    pub fn new(target: Entity) -> Self {
        Self {
            target,
            roll: 0,
            engaged: false,
            holds_ground: false,
            reaction_ms: 0,
            engage_at_ms: None,
            in_combat: false,
            mind: combat::CombatMind::default(),
            last_tick: None,
            bearing: (Vec3::Z, 1.0),
            step: None,
            queued: None,
            queue: 0,
            personality: None,
            chase: chase::ChaseMind::default(),
            route: Vec::new(),
            chasing: false,
            stab_aim: exa_combat::stab::StabAim::default(),
        }
    }
}

/// Exarust's interim footwork clip (the original's AI steps in its stance).
const COMBAT_WALK: &str = "Combat Walk";
/// Centimetres per metre: the combat state works in the original's units.
const CM: f32 = 100.0;
/// A step is done this close to its goal (m) [Exarust's: the original's
/// body ends a step on its feet's landing].
const STEP_DONE: f32 = 0.1;
/// A step that has not arrived ends after this long (ms) [Exarust's].
const STEP_MOST_MS: u32 = 1000;
/// Combat ticks run at most this many times a call (the brains' tick runs
/// once per body tick, so one; a caller between ticks catches up).
const MOST_TICKS: u64 = 4;

/// A fighter's reach (m), as the original's equip refresh sets it: its
/// longest held weapon's record reach (60 cm × its size plus the weapon's
/// sweet spot), else (its fists) its race's unarmed reach (the human's
/// 50 cm).
fn reach_of(fighter: Entity, blades: &Query<&Blade>) -> f32 {
    blades
        .iter()
        .filter(|b| b.wielder == fighter && b.record.armed())
        .map(|b| b.record.reach_metres())
        .reduce(f32::max)
        .unwrap_or(stance::UNARMED_REACH)
}

/// A fighter's weapon class as its strikers make it: a held weapon's hands
/// bits, else the fists' (the claws' for a clawed race's).
fn weapon_class_of(fighter: Entity, blades: &Query<&Blade>) -> u8 {
    let mine = || blades.iter().filter(|b| b.wielder == fighter);
    if let Some(weapon) = mine().find(|b| b.record.armed()) {
        return (weapon.record.flags & 0x30) as u8;
    }
    if mine().any(|b| b.record.flags & 0x40 != 0) {
        0xC0
    } else {
        0x80
    }
}

/// The motion id a clip plays, as the combat state reads it: the stances
/// 3 and 4, the side attacks 5–7 and 8–10, the stab 11, the overhead 12.
fn motion_of(attacking: Option<&Attacking>, stance: Stance) -> i32 {
    match attacking.map(|a| a.motion) {
        Some(AttackMotion::Main(i)) => 5 + i as i32,
        Some(AttackMotion::Other(i)) => 8 + i as i32,
        Some(AttackMotion::Stab) => 0xb,
        Some(AttackMotion::Overhead) => 0xc,
        None => match stance {
            Stance::Defence => combat::GUARD,
            _ => combat::COMBAT_IDLE,
        },
    }
}

/// The combat state's view of a duel: the one contact is the target, which
/// the position score rates from each place.
struct DuelField<'a> {
    foe: Vec3,
    heading: Vec3,
    /// The foe's body facing (unit).
    foe_forward: Vec3,
    spot: place::Spot,
    /// The level's navigation grid, the installed pathfinder (none: the
    /// abstract one).
    grid: Option<&'a exa_world::nav::NavGrid>,
    /// The chase's state, when the level has a navigation grid.
    chase: Option<DuelChase<'a>>,
    /// The power system, when the duellist has powers.
    powers: Option<&'a mut DuelPowers>,
}

/// What the chase route needs from the duel, and what it did.
struct DuelChase<'a> {
    mind: &'a mut chase::ChaseMind,
    turn: &'a mut PlanTurn,
    grid: &'a exa_world::nav::NavGrid,
    who: u64,
    frame: u32,
    body: ChaseBody,
    foe: ChaseFoe,
    outcome: Option<chase::Outcome>,
}

impl combat::Field for DuelField<'_> {
    fn ground_cost(&mut self, point: Vec3) -> f32 {
        ground::ground_cost(self.grid, point)
    }
    fn walk_line(&mut self, from: Vec3, to: &mut Vec3) -> bool {
        ground::walk_line(self.grid, from, to)
    }
    fn blocked(&mut self, from: Vec3, to: Vec3) -> bool {
        ground::out_of_sight(self.grid, from, to)
    }
    fn chase(&mut self, estimate: Vec3, roll: &mut u32) -> bool {
        let Some(c) = self.chase.as_mut() else {
            return false;
        };
        // The duellist's foe is fresh while it perceives it [Exarust's: the
        // original's mark is set when the target is taken].
        c.mind.fresh = true;
        c.mind.estimate = estimate;
        let mut router = NavRouter {
            grid: c.grid,
            size: 0,
        };
        let outcome = chase::chase(
            c.mind,
            c.who,
            &c.body,
            Some(c.foe),
            None,
            c.turn,
            c.frame,
            &[ContactKey { id: c.foe.id }],
            Some(&mut router),
            roll,
        );
        let ends = outcome.end.ends_tick();
        c.outcome = Some(outcome);
        ends
    }
    fn score(&mut self, _contact: usize, level: i32, point: Vec3) -> combat::PlaceScore {
        let spot = place::Spot { level, ..self.spot };
        let p = place::score_place_at(point, self.foe, self.heading, self.foe_forward, spot);
        combat::PlaceScore {
            close: p.score.close,
            danger: p.score.danger,
            approach: p.score.approach,
            strike: p.score.strike,
            bearing: p.bearing,
            distance_cm: p.distance_cm,
            ahead: p.ahead,
        }
    }
    fn powers(&mut self) -> Option<&mut dyn cast::Powers> {
        self.powers
            .as_deref_mut()
            .map(|powers| powers as &mut dyn cast::Powers)
    }
    fn cast_running(&mut self) -> bool {
        self.powers.as_deref_mut().is_some_and(|powers| {
            powers.probed = true;
            powers.running
        })
    }
}

/// The attack arc a fighter publishes, as the position score reads it.
/// `swing_flags` is the fighter's swing word: its cleaving bit makes the arc
/// a recovery and stops the sweep, and its continuation bits mark the swing
/// committed.
#[allow(clippy::too_many_arguments)]
fn published_blow(
    fighter: Entity,
    blades: &Query<&Blade>,
    anim: &Animator,
    phase: f32,
    heading: f32,
    facing: f32,
    attacking: bool,
    swing_flags: u32,
) -> place::Blow {
    let recovering = swing_flags & swing::flags::CLEAVING != 0;
    let armed = blades
        .iter()
        .any(|b| b.wielder == fighter && b.record.armed());
    let motion = if attacking { anim.action.as_str() } else { "" };
    let action = match motion {
        "Attack Mid" | "Attack Down" | "Attack Up" => 5,
        "Attack Combo 1" | "Attack Combo 2" | "Attack Combo 3" => 8,
        "Stab" => 11,
        "Overhead Swing" => 12,
        _ => 0,
    };
    let arc = place::publish_arc(place::Swing {
        action,
        phase,
        heading,
        facing,
        armed,
        held: armed,
        recovering,
        hand: None,
        idle_clock: 0,
        idle_stamp: 0,
    });
    place::Blow {
        kind: arc.kind,
        arc_start: arc.start,
        arc_end: arc.end,
        phase,
        skill: 0.0,
        committed: swing_flags & (swing::flags::CONTINUATION | swing::flags::CONTINUED) != 0,
        sweeping: attacking && !recovering,
        // The observer's grip link on the opponent; a duellist has none.
        forced_range: false,
        origin_in_arc: false,
        angle: 0.0,
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn duellist(
    mut commands: Commands,
    ticks: Option<Res<BodyTicks>>,
    (mut nav, mut footprints): (
        Option<ResMut<exa_world::nav::NavGrid>>,
        Option<ResMut<exa_world::nav::NavFootprints>>,
    ),
    targets: Query<(&GlobalTransform, Has<brain::MindWarded>)>,
    motions: Option<Res<Motions>>,
    bodies: Query<
        (
            &Condition,
            &GlobalTransform,
            Option<&Ragdoll>,
            &Fighter,
            Option<&Animator>,
            Option<&exa_physics::body::locomotion::Locomotor>,
        ),
        Without<Duellist>,
    >,
    children: Query<&Children>,
    (blades, target_swings): (Query<&Blade>, Query<&SwingState, Without<Duellist>>),
    mut brains: Query<&mut brain::Npc>,
    role_db: Option<Res<brain::RoleDatabase>>,
    role_lists: Query<&crate::roles::Roles>,
    ids: Query<&brain::CharacterId>,
    mut npcs: Query<(
        Entity,
        &mut Duellist,
        &mut Stance,
        &mut Transform,
        &mut Animator,
        (&mut Attacker, &mut SwingState),
        &ChildOf,
        Option<&Attacking>,
        Has<Fallen>,
        &mut Combatant,
        &Condition,
        &GlobalTransform,
        Option<&Ragdoll>,
        &Fighter,
        (
            Option<&mut DuelPowers>,
            Option<&presence::Presence>,
            Option<&exa_combat::AttackHook>,
            Option<&exa_physics::body::locomotion::Locomotor>,
        ),
    )>,
    mut roots: Query<(&mut CharacterBody, &GlobalTransform)>,
    mut asked: MessageWriter<exa_combat::AttackAskedHook>,
    mut planning: Local<(PlanTurn, u32)>,
) {
    // One brain plans a route a tick: the longest waiter of the last one.
    let (turn, frame) = &mut *planning;
    *frame = frame.wrapping_add(1);
    turn.next_turn();
    let tick_now = ticks.as_ref().map(|t| t.0);
    let now = ticks.map_or(0, |t| t.millis());
    for (
        entity,
        mut ai,
        mut stance,
        mut t,
        mut anim,
        (mut attacker, mut swinging),
        parent,
        attacking,
        fallen,
        mut combatant,
        condition,
        gt,
        ragdoll,
        fighter,
        (mut powers, presence, attack_hook, locomotor),
    ) in &mut npcs
    {
        let Ok((mut body, root_gt)) = roots.get_mut(parent.parent()) else {
            continue;
        };
        body.desired_velocity = Vec3::ZERO;
        // Down, getting up, collapsing or dead (or Exarust's fall), it does
        // nothing.
        if condition.fallen_or_dying() || fallen {
            continue;
        }
        if let Some(at) = ai.engage_at_ms {
            if now < at {
                continue;
            }
            ai.engage_at_ms = None;
            ai.engaged = true;
        }
        // A brain decides the engagement in the AI tick's target pass; a
        // duellist without one is started by the fight's own setup.
        if !ai.engaged {
            continue;
        }
        let Ok((target, target_warded)) = targets.get(ai.target) else {
            continue;
        };
        // The target's body: the fighter at or under the target entity.
        let target_body = std::iter::once(ai.target)
            .chain(children.iter_descendants(ai.target))
            .find_map(|e| bodies.get(e).ok().map(|b| (e, b)));
        let target_fighter_entity = target_body.map(|(e, _)| e);
        // The foe as its contact is kept: the fighter, as the brain's
        // perception keys it, so the combat state reads the concern and
        // hostility the brain integrates (the root without a fighter).
        let foe_key = target_fighter_entity.unwrap_or(ai.target);
        let target_body = target_body.map(|(_, b)| b);
        let traits = brains
            .get(entity)
            .ok()
            .and_then(|b| b.personality)
            .or(ai.personality)
            .map_or(
                // A fighter without a record's personality: skill 0 and
                // the blend 1 a fully feral one has [Exarust's default].
                combat::Traits {
                    skill: 0.0,
                    blend: 1.0,
                },
                |p| combat::Traits {
                    skill: p.skill,
                    blend: p.blend,
                },
            );
        let foe_chest = target_body
            .map(|(_, tgt, tgt_body, ..)| stance::chest_point(tgt, tgt_body))
            .unwrap_or_else(|| stance::chest_point(target, None))
            * CM;
        let target_forward = target.rotation() * Vec3::Z;
        let foe_motion = foe_walk(target_body.and_then(|(.., l)| l), target_forward);
        if !ai.in_combat {
            // Entering combat: the guard, from the idle it stood in, its
            // eagerness at a half, and the published reach. The combat
            // rating comes from the experience system, which is not wired
            // here.
            ai.in_combat = true;
            combat::enter(&mut ai.mind, false, now as u32);
            ai.mind.heading = yaw_of(t.rotation);
            // The target is set here: the estimate starts at the foe
            // estimate's lead (a moving foe's chest 30 ticks along its
            // facing at its speed), the set-target's fresh flag, commitment
            // and cleared overshoot and stuck counts are set, and the
            // bearing and aim are measured from the chest; the brain's look
            // takes the aim's yaw too.
            let chest_cm = stance::chest_point(gt, ragdoll) * CM;
            let lead = combat::foe_estimate(
                &combat::Foe {
                    chest: foe_chest,
                    step_goal: foe_chest,
                    running: foe_motion.moving,
                    facing: foe_motion.facing,
                    forward_speed: foe_motion.speed_cm,
                    ..combat::Foe::default()
                },
                traits.skill,
            );
            let previous = ai.bearing.0;
            let bearing = combat::set_target(&mut ai.mind, lead, chest_cm, previous);
            ai.bearing = (bearing.to_target, bearing.distance_cm);
            if let Ok(mut npc) = brains.get_mut(entity) {
                npc.turn.look = ai.mind.aim;
            }
            info!(
                "duellist {entity:?} sets its target: estimate {:.1} cm from the foe's chest \
                 (moving {}, {:.2} cm a tick), aim {:.3} rad",
                (lead - foe_chest).length(),
                foe_motion.moving,
                foe_motion.speed_cm,
                ai.mind.aim
            );
            combatant.in_combat = true;
            // The queued step is cleared with the body's own speed, so a
            // far move point is pulled in to 8·speed + 20 cm ahead.
            combatant.clear_combat_step(
                gt.translation() * 100.0,
                t.rotation * Vec3::Z,
                locomotor.map_or(0.0, |l| l.speed),
            );
            // Publish the fighting values: the experience system's estimate
            // from the record's skills and combat skill, the held weapon's
            // stat bytes and reach, and the body's scale, and the body's
            // reach.
            let reach_cm = reach_of(entity, &blades) * 100.0;
            let base = brains.get(entity).map_or(0.0, |brain| brain.fighting_base);
            let weapon = attacker
                .primary_record(entity, blades.iter())
                .map(|record| (&record.stats, reach_cm));
            let rating =
                crate::estimate::fighting_estimate_with_base(base, weapon, fighter.stats.scale);
            combatant.refresh_fighting(rating, reach_cm);
            stance::set_stance(&mut stance, &mut anim, Stance::Defence);
            info!(
                "duellist {entity:?} enters combat: {:?} at tick {}",
                *stance,
                tick_now.unwrap_or(0)
            );
            continue;
        }
        let primary = attacker.primary_record(entity, blades.iter()).copied();
        let primary_blend = primary.map(|r| r.blend);
        // Leave a collapsing or dead target alone.
        if target_body.is_some_and(|(c, ..)| c.dying()) {
            if anim.looping {
                stance::take_ai_stance(&mut anim, *stance, primary_blend);
            }
            continue;
        }
        let facing = t.rotation * Vec3::Z;
        let to = target.translation() - root_gt.translation();
        let flat = Vec3::new(to.x, 0.0, to.z);

        // One perception pass on the target (the combatant's), then the
        // published arc the position score reads.
        let target_condition = target_body.map(|(c, ..)| *c).unwrap_or(Condition::NORMAL);
        let target_fighter = target_body.map(|(_, _, _, f, _, _)| f);
        let target_anim = target_body.and_then(|(_, _, _, _, a, _)| a);
        let target_locomotor = target_body.and_then(|(_, _, _, _, _, l)| l);
        let target_moving = target_locomotor.is_some_and(|l| l.amount != 0.0);
        let target_speed_cm = target_locomotor.map_or(0.0, |l| l.speed);
        let phase = target_anim
            .zip(motions.as_deref())
            .and_then(|(a, m)| m.clip(a, &a.action).map(|c| a.clip_progress(c)))
            .unwrap_or(0.0);
        let observed = Observed {
            entity: foe_key,
            position_cm: target.translation() * 100.0,
            look: target_forward,
            forward: target_forward,
            condition: target_condition.0,
            gone: false,
            hidden: target_warded,
            mask: None,
            asleep: false,
            removed: false,
            phase,
            locomotion: target_moving,
            in_combat: target_condition != Condition::NORMAL,
            stepping: false,
            poised: false,
            speed: 0.0,
        };
        let observer = Observer {
            entity,
            position_cm: gt.translation() * 100.0,
            look: facing,
            forward: facing,
            asleep: false,
            perception: fighter.stats.perception,
            calm: 0.0,
            aggression: 0.0,
            mind_aggression: 0.0,
            resist_mind: false,
            target: Some(foe_key),
        };
        let index = combatant.contact_index(foe_key);
        if brains.get(entity).is_err() {
            // A duellist without a brain (the practice opponent) perceives
            // its target itself. The player carries no contacts, so the
            // target's list and its trust are empty here; its raw attitude
            // comes from the roles when the database has them.
            let mut local = crate::roles::Relations::default();
            let raw = match (
                role_db.as_deref(),
                role_lists.get(entity),
                role_lists.get(foe_key),
            ) {
                (Some(db), Ok(theirs), Ok(targets)) => {
                    let partner = crate::roles::Partner {
                        id: ids.get(foe_key).map_or(0, |i| i.0),
                        roles: targets,
                        former: None,
                    };
                    let rating =
                        crate::roles::attitude(&db.book, theirs, &mut local, &partner, db.locale);
                    Attitude {
                        value: rating.attitude,
                        tolerance: rating.tolerance,
                    }
                }
                _ => Attitude {
                    value: combatant.contacts[index].attitude,
                    tolerance: combatant.contacts[index].tolerance * 100.0,
                },
            };
            let sight = perceive(
                &observer,
                &observed,
                &[],
                &mut combatant.contacts,
                index,
                raw,
                &|_| 0.0,
                &mut [],
                now,
            );
            if let Some(floor) = sight.alert_floor
                && combatant.alert < floor
            {
                combatant.alert = floor;
            }
        }
        // The brain's perception pass already saw the target this tick,
        // with its own contacts and the relations' trust.
        combatant.target = Some(foe_key);
        let contact = &mut combatant.contacts[index];
        if target_condition.is_dead() && contact.died_ms.is_none() {
            contact.died_ms = Some(now);
        }
        let target_swing = target_fighter_entity
            .and_then(|e| target_swings.get(e).ok())
            .map_or(0, |state| state.flags);
        let mut blow = published_blow(
            foe_key,
            &blades,
            target_anim.unwrap_or(&anim),
            phase,
            atan2_xz(target_forward.x, target_forward.z),
            atan2_xz(target_forward.x, target_forward.z),
            target_condition == Condition::ATTACKING,
            target_swing,
        );
        if blow.kind == 0 {
            blow.sweeping = false;
        }

        // The step it walks ends at its goal, or when it has taken too long;
        // a queued one starts then.
        if let Some(step) = ai.step {
            let to = step.goal - root_gt.translation();
            let arrived = Vec3::new(to.x, 0.0, to.z).length() < STEP_DONE;
            if arrived || (now as u32).wrapping_sub(step.started_ms) > STEP_MOST_MS {
                ai.step = None;
                if let Some(offset) = ai.queued.take() {
                    let chest = stance::chest_point(gt, ragdoll);
                    ai.step = Some(Stepping {
                        goal: Vec3::new(chest.x, root_gt.translation().y, chest.z) + offset / CM,
                        started_ms: now as u32,
                        state: 0,
                    });
                    ai.queue = -1;
                }
            }
        }
        let runs = match (tick_now, ai.last_tick) {
            (Some(n), Some(last)) => n.saturating_sub(last).min(MOST_TICKS),
            _ => 1,
        };
        ai.last_tick = tick_now;
        let feet = root_gt.translation();
        let chest = stance::chest_point(gt, ragdoll) * CM;
        let reach = reach_of(entity, &blades);
        let class = weapon_class_of(entity, &blades);
        let (energy, most) = ragdoll
            .and_then(|r| r.drive.as_ref())
            .map_or((MAX_ENERGY, MAX_ENERGY), |d| (d.energy, d.max_energy()));
        let base = ragdoll
            .and_then(|r| r.drive.as_ref())
            .map_or(1.0, |d| d.race.clip_rate);
        let busy = Busy {
            now_ms: now,
            last_tick_ms: attacker.last_tick_ms,
            cooldown_ms: attacker.cooldown_ms(entity, blades.iter()),
            playing: None,
            released: swinging.has(swing::flags::RELEASED),
            stall_until_ms: swinging.end_ms,
            renew_ms: swinging.renew_ms,
            side_started_ms: attacker.side_started_ms,
            feint: fighter.feint(),
            strikers: attack::held_strikers(entity, blades.iter()),
        };
        let ready = attack::execute(attack::Request::default(), busy).is_some();
        // The foe's own combat skill: a brain's personality, else 0 (the
        // player's behaviour skill is not modelled) [Exarust's default].
        let foe_skill = brains
            .get(foe_key)
            .ok()
            .and_then(|b| b.personality)
            .map_or(0.0, |p| p.skill);
        // The position score measures the foe from a point on its body,
        // not its body's own position: ahead of it standing, out from its
        // chest by 70 ticks of its speed while it moves. The duellist's
        // target's step flag is not exposed here, so the stepping branch is
        // not taken [guess: only its locomotion amount is read].
        let foe_origin = place::measured_origin(place::Origin {
            body_cm: target.translation() * CM,
            chest_cm: foe_chest,
            forward: target_forward,
            speed_cm: target_speed_cm,
            condition: target_condition.0,
            locomotion: target_moving,
            stepping: false,
            step_goal_cm: foe_chest,
            attacking: target_condition == Condition::ATTACKING,
            phase,
            skill: foe_skill,
        });
        // The danger terms read the candidate's angle from the measured
        // origin, and the thrust's arc test the observer's own bearing from
        // it: its body position less that origin.
        let me_cm = feet * CM;
        blow.angle = atan2_xz(me_cm.x - foe_origin.x, me_cm.z - foe_origin.z);
        blow.origin_in_arc = place::between(blow.arc_end, blow.arc_start, blow.angle);
        let foe = combat::Foe {
            condition: target_condition.0,
            dead: target_condition.is_dead(),
            reach_cm: reach_of(foe_key, &blades) * CM,
            chest: foe_chest,
            step_goal: foe_chest,
            running: foe_motion.moving,
            forward_speed: foe_motion.speed_cm,
            facing: foe_motion.facing,
            scale: target_fighter.map_or(1.0, |f| f.stats.scale),
            rules: target_fighter.map_or(0, |f| f.stats.rules.0),
            ..combat::Foe::default()
        };
        let dir_to_me = -flat.normalize_or_zero();
        let spot = place::Spot {
            other_reach_cm: foe.reach_cm,
            my_reach_cm: reach * CM,
            other_size: target_fighter.map_or(1.0, |f| f.stats.size),
            my_step_cm: fighter.stats.step_reach(fighter.stats.encumbrance) * CM,
            other_large: target_fighter.is_some_and(|f| f.stats.rules.has(1)),
            melee: false,
            // The duellist runs the combat state against one foe, which it
            // always engages at the fight level.
            level: 3,
            feral: fighter.stats.feral,
            dull: fighter.stats.rules.has(4),
            blow: place::Blow {
                skill: traits.skill,
                ..blow
            },
            wide: false,
            observer_ahead: facing.dot(dir_to_me),
            other_ahead: target_forward.dot(dir_to_me),
            in_attack: *condition == Condition::ATTACKING,
            presses: false,
            other_skill: foe_skill,
        };
        // The AI tick, before its state: a cast whose body is not casting
        // 500 ms after it began is let go (64-bit compare). A duellist runs
        // only the combat state, so the other states' hook call with 0x10
        // on a casting body does not arise.
        if ai.mind.casting
            && *condition != Condition::CASTING
            && u64::from(ai.mind.cast_start_ms) + 500 < u64::from(now as u32)
        {
            ai.mind.casting = false;
        }
        // The AI tick lifts the fighter's own footprint before its state
        // and stamps it again after, so the state's ground costs, line
        // walks and sight see only the others' footprints.
        let own_print = match (nav.as_deref_mut(), footprints.as_deref_mut()) {
            (Some(grid), Some(prints)) => {
                let at = prints.of(entity);
                prints.lift(grid, entity);
                at
            }
            _ => None,
        };
        // The combat state lifts its foe's footprint too, so its place
        // scores do not pay for the foe it closes on; it stamps the foe's
        // again after the place pass, and an earlier exit (down, leave,
        // chase, charge) leaves it to the foe's own controller, which
        // stamps it on its next tick.
        let foe_print = match (
            target_fighter_entity,
            nav.as_deref_mut(),
            footprints.as_deref_mut(),
        ) {
            (Some(foe), Some(grid), Some(prints)) => prints.of(foe).map(|at| {
                prints.lift(grid, foe);
                (foe, at)
            }),
            _ => None,
        };
        let mut placed = runs == 0;
        for _ in 0..runs {
            let yaw = yaw_of(t.rotation);
            let stepping = ai.step;
            let mut me = combat::Body {
                now_ms: now as u32,
                condition: condition.0,
                power_pool: powers.as_deref().map_or(0.0, |p| p.pool),
                power_slots: powers.as_deref().map_or([0; cast::SLOTS], |p| p.slots),
                clock: anim.progress,
                weapon_class: u32::from(class),
                held: blades
                    .iter()
                    .filter(|b| b.wielder == entity && b.record.armed())
                    .count() as i32,
                motion: motion_of(attacking, *stance),
                energy,
                max_energy: most,
                scale: fighter.stats.scale,
                size: fighter.stats.size,
                encumbrance: fighter.stats.encumbrance,
                reach_cm: reach * CM,
                step_cm: spot.my_step_cm,
                stepping: stepping.is_some(),
                step_state: stepping.map_or(0, |s| s.state),
                step_started_ms: stepping.map_or(0, |s| s.started_ms),
                queue: ai.queue,
                // The body holds on and is aware; Exarust has no model of
                // either, so the gate's grip and awareness are full and
                // only the condition mask can refuse it.
                can_step: true,
                rules: fighter.stats.rules.0,
                thrust_pref: primary.map_or(0, |r| {
                    i32::from(exa_formats::weapon_record::thrust_preference(&r))
                }),
                yaw,
                heading: combat::of_yaw(yaw),
                facing: combat::of_yaw(yaw),
                position: chest,
                step_goal: stepping.map_or(chest, |s| s.goal * CM),
                chest,
                ground_y: feet.y * CM,
                look: combat::of_yaw(ai.mind.heading),
                target_point: foe_chest,
                to_target: ai.bearing.0,
                distance_cm: ai.bearing.1,
                swing_amount: primary.map(|r| r.swing_turn),
                stance_blend: primary_blend,
                health: fighter.health.yellow,
                can_attack: ready && !ai.holds_ground,
                // The trailing-foot request a clip leaving Walk or Run
                // raised holds the gate shut until its plant's 500 ms
                // deadline passes.
                blocking: locomotor.map_or(0, |l| l.plant_request),
                // The body's parrying flag: while its auto-parry holds, the
                // combat state leaves its stance motion alone.
                parrying: fighter.parry.active,
                ..combat::Body::default()
            };
            me.can_step = combat::can_act(&me) && !ai.holds_ground;
            ai.queue = 0;
            let ai_target_bits = ai.target.to_bits();
            let Duellist {
                mind,
                roll,
                chase: chase_mind,
                ..
            } = &mut *ai;
            let mut field = DuelField {
                foe: foe_origin,
                heading: me.heading,
                foe_forward: target_forward,
                spot,
                grid: nav.as_deref(),
                chase: nav.as_deref().map(|grid| DuelChase {
                    mind: chase_mind,
                    turn: &mut *turn,
                    grid,
                    who: entity.to_bits(),
                    frame: *frame,
                    body: ChaseBody {
                        position_cm: feet * CM,
                        ground_y_cm: feet.y * CM,
                        scale: fighter.stats.scale,
                        motion: me.motion,
                        voice: None,
                        now_ms: now as u32,
                    },
                    foe: ChaseFoe {
                        id: ai_target_bits,
                        ground_y_cm: target.translation().y * CM,
                    },
                    outcome: None,
                }),
                powers: powers.as_deref_mut(),
            };
            // A chase mark lapses once its time has passed (the contact
            // integration's rule).
            let held = &mut combatant.contacts[index];
            if held.flags & crate::contacts::UNREACHABLE != 0
                && held.unreachable_until_ms < now as u32
            {
                held.flags &= !crate::contacts::UNREACHABLE;
            }
            let marked = held.flags & crate::contacts::UNREACHABLE != 0;
            let mut contacts = [combat::Contact {
                level: 3,
                is_foe: true,
                character: true,
                concern: held.concern,
                hostility: held.hostility,
                unreachable: marked,
                ..combat::Contact::default()
            }];
            let out = combat::tick(
                mind,
                &me,
                traits,
                combat::World {
                    foe: Some(&foe),
                    focus: None,
                    contacts: &mut contacts,
                    around: &[],
                    field: &mut field,
                    leader_calls: false,
                },
                roll,
            );
            let chased = field.chase.and_then(|c| c.outcome);
            placed = matches!(out.exit, None | Some(combat::Exit::NoPlace));
            if marked && !contacts[0].unreachable {
                // The foe is in reach again.
                combatant.contacts[index].flags &= !crate::contacts::UNREACHABLE;
            }
            if let Some(b) = out.bearing {
                ai.bearing = (b.to_target, b.distance_cm);
            }
            if let Some(stopped) = out.stop_run
                && let Some(step) = ai.step.as_mut()
            {
                step.goal = stopped.step_goal / CM;
            }
            // The swing the combat state lets go of is released here, as
            // the player's own leave does: once a swing (its released bit
            // keeps a later tick from restarting the stall).
            if out.blow.release
                && attacking.is_some()
                && !swinging.has(swing::flags::RELEASED)
                && let Some(motion) = motions.as_deref()
                && let Some(at) = SwingAt::of(now, &anim, motion, base)
                && !swinging.has(swing::flags::NO_RELEASE)
            {
                let swing = primary.as_ref().map(Swing::of).unwrap_or(Swing::FISTS);
                swinging.release(&swing, &at);
            }
            ai.chasing = out.exit == Some(combat::Exit::Chase);
            if let Some(outcome) = chased {
                let contact = &mut combatant.contacts[index];
                if let Some(mark) = outcome.mark {
                    contact.unreachable_until_ms = mark.until_ms;
                    contact.flags |= crate::contacts::UNREACHABLE;
                } else if outcome.reached.is_some() {
                    contact.flags &= !crate::contacts::UNREACHABLE;
                }
                if let Some(planned) = outcome.planned {
                    ai.route = if planned.following {
                        planned.route.points
                    } else {
                        Vec::new()
                    };
                }
                if outcome.end == chase::End::Left {
                    info!("duellist {entity:?} leaves combat: its foe is out of reach");
                    ai.leave();
                    combatant.in_combat = false;
                    // The chase leaves through the idle return: the AI's
                    // look, heading and eased offset take the body's look
                    // yaw, the idle state's step restarts at 1, the idle
                    // state becomes the current state and the body's
                    // in-combat mark clears (no leave timers). The body's
                    // target and focus words it clears first are the
                    // duellist's fixed foe here [not modelled].
                    if let Ok(mut brain) = brains.get_mut(entity) {
                        let body_look = crate::motion::yaw_of(t.rotation * Vec3::Z);
                        let brain = &mut *brain;
                        let step = crate::lifecycle::return_to_idle(
                            &mut brain.gaze,
                            &mut brain.turn,
                            body_look,
                        );
                        crate::lifecycle::restart_idle(brain);
                        info!(
                            "duellist {entity:?} returns to its idle state {:?} at step {step}, \
                             looking at {body_look:.3} rad",
                            brain.state()
                        );
                    }
                }
            }
            if out.exit.is_some() {
                continue;
            }
            match out.step {
                combat::Footstep::Start { goal, .. } => {
                    ai.step = Some(Stepping {
                        goal: Vec3::new(goal.x / CM, feet.y, goal.z / CM),
                        started_ms: now as u32,
                        state: 0,
                    });
                }
                combat::Footstep::Continue { goal, .. } => {
                    ai.step = Some(Stepping {
                        goal: Vec3::new(goal.x / CM, feet.y, goal.z / CM),
                        started_ms: now as u32,
                        state: 2,
                    });
                }
                combat::Footstep::Queue { offset, .. } => ai.queued = Some(offset),
                combat::Footstep::Stay => {}
            }
            if out.marks.clear_queue {
                ai.queued = None;
            }
            // Its stance: the guard or the combat idle the state chose.
            if let Some(guard) = out.blow.guard {
                let chosen = if guard.stance == combat::GUARD {
                    Stance::Defence
                } else {
                    Stance::CombatIdle
                };
                if *stance != chosen {
                    info!("duellist {entity:?} stance: {chosen:?}");
                    *stance = chosen;
                }
            }
            if let Some(motion) = out.blow.motion
                && attacking.is_none()
                && anim.looping
                && !ai.holds_ground
                && {
                    // The attack start asks a running cast's hook first: a
                    // hook that keeps its cast refuses the attack.
                    let (ask, go) = exa_combat::AttackHook::gate(attack_hook);
                    if ask {
                        asked.write(exa_combat::AttackAskedHook(entity));
                    }
                    go
                }
            {
                let (chosen, d0, dc, ap) = match (out.chosen, out.search.as_ref()) {
                    (Some((c, _)), Some(s)) => (
                        c,
                        s.places[0].distance_cm,
                        s.places[c].distance_cm,
                        s.places[c].approach,
                    ),
                    _ => (0, 0.0, 0.0, 0.0),
                };
                debug!(
                    "duellist {entity:?} starts {motion:?} {:.0} cm from its foe, reach {:.0} cm; \
                     place {chosen} at {dc:.0} cm (here {d0:.0}), approach {ap:.2}",
                    ai.bearing.1,
                    reach * CM
                );
                // A stab's aim starts on the attack's aim, level.
                if motion == AttackMotion::Stab {
                    ai.stab_aim = exa_combat::stab::StabAim::start(ai.mind.aim);
                    commands.entity(entity).insert(ai.stab_aim);
                }
                exa_combat::start_attack(
                    &mut commands,
                    entity,
                    &mut anim,
                    &mut attacker,
                    &mut swinging,
                    blades.iter(),
                    AttackStart {
                        remise: fighter.remise(),
                        ..AttackStart::of(motion, class, now)
                    },
                    stance.action(),
                );
            }
        }
        // While a stab plays, the behaviour's stab aim turns toward the
        // foe's estimate each tick, from the anchor by the shoulder in the
        // character frame, kept near the body's heading.
        if attacking.is_some_and(|a| a.motion == AttackMotion::Stab)
            && let Some(frame) = ragdoll
                .and_then(|r| r.drive.as_ref())
                .and_then(|d| d.frame())
        {
            let anchor = frame.transform_point3(exa_combat::stab::ANCHOR_CM / CM) * CM;
            let forward = frame.matrix3.z_axis;
            let heading = exa_anim::quick_yaw(forward.x, forward.z);
            let target = ai.mind.estimate;
            for _ in 0..runs {
                ai.stab_aim.step(target, anchor, heading);
            }
            commands.entity(entity).insert(ai.stab_aim);
        }
        if let (Some(at), Some(grid), Some(prints)) =
            (own_print, nav.as_deref_mut(), footprints.as_deref_mut())
        {
            prints.stamp(grid, entity, at);
        }
        if let Some((foe, at)) = foe_print {
            if placed
                && let (Some(grid), Some(prints)) = (nav.as_deref_mut(), footprints.as_deref_mut())
            {
                prints.stamp(grid, foe, at);
            } else {
                debug!(
                    "duellist {entity:?}: its foe's footprint stays off until the foe's own tick"
                );
            }
        }
        // Only a character in the scene moves its body. A hidden one's AI
        // still runs its combat state above, as the original's AI manager
        // runs a dormant character's whole AI tick; its body, out of the
        // scene, does not take the attack forward.
        if presence.is_some_and(|p| !p.moves()) {
            continue;
        }
        // It turns to the heading its look follows.
        t.rotation = Quat::from_rotation_y(ai.mind.heading);
        if attacking.is_some() || !anim.looping {
            continue;
        }
        // While the combat tick ends at the chase, its route is walked
        // through the path's carrot.
        if ai.chasing && !ai.route.is_empty() {
            let carrot = crate::path::carrot(&ai.route, feet * CM);
            if let Some(goal) = carrot.goal {
                let order = follow(&[feet, goal / CM], feet, yaw_of(t.rotation), 0);
                body.desired_velocity = order.velocity;
                anim.play_loop(COMBAT_WALK);
            }
            let last = ai.route[ai.route.len() - 1] / CM;
            if Vec3::new(last.x - feet.x, 0.0, last.z - feet.z).length() < 0.5 {
                ai.route.clear();
            }
            continue;
        }
        match ai.step {
            Some(step) if !ai.holds_ground => {
                let order = follow(&[feet, step.goal], feet, yaw_of(t.rotation), 0);
                body.desired_velocity = order.velocity;
                anim.play_loop(COMBAT_WALK);
            }
            _ => stance::take_ai_stance(&mut anim, *stance, primary_blend),
        }
    }
}

/// What the foe estimate reads of the foe's locomotion: whether it moves,
/// its speed along its heading (cm a tick) and its facing.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FoeWalk {
    moving: bool,
    speed_cm: f32,
    facing: Vec3,
}

/// The foe's locomotion from its body's locomotor (the player's); a foe
/// without one (an NPC, whose path driver writes no locomotor) stands, and
/// faces along `forward`.
fn foe_walk(
    locomotor: Option<&exa_physics::body::locomotion::Locomotor>,
    forward: Vec3,
) -> FoeWalk {
    match locomotor {
        Some(l) => FoeWalk {
            moving: l.moving(),
            speed_cm: l.speed,
            facing: exa_physics::body::locomotion::forward(l.yaw),
        },
        None => FoeWalk {
            moving: false,
            speed_cm: 0.0,
            facing: forward,
        },
    }
}

/// The yaw a rotation faces (+Z forward).
fn yaw_of(rotation: Quat) -> f32 {
    let facing = rotation * Vec3::Z;
    facing.x.atan2(facing.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plugin runs on a bare app with no characters, with and without
    /// the optional role database, and does not panic over three frames.
    #[test]
    fn the_ai_plugin_runs_without_resources_or_characters() {
        for with_db in [false, true] {
            let mut app = App::new();
            app.add_plugins((bevy::MinimalPlugins, AiPlugin));
            if with_db {
                app.insert_resource(brain::RoleDatabase::default());
            }
            for _ in 0..3 {
                app.update();
            }
        }
    }

    /// An unarmed duellist on a root at the origin, its target `distance` m
    /// away along +Z.
    fn duel(distance: f32, holds_ground: bool) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_message::<exa_combat::AttackAskedHook>()
            .add_systems(Update, duellist);
        let target = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, distance)),
                Fighter::default(),
            ))
            .id();
        let root = app
            .world_mut()
            .spawn((CharacterBody::default(), GlobalTransform::IDENTITY))
            .id();
        let mut ai = Duellist::new(target);
        ai.holds_ground = holds_ground;
        let foe = app
            .world_mut()
            .spawn((
                ai,
                Transform::default(),
                GlobalTransform::IDENTITY,
                Animator::new("Default", "Idle"),
                Fighter::default(),
                ChildOf(root),
            ))
            .id();
        // Its empty hands hold both fists, as the original's set-up holds
        // them: the executor's held count is 2.
        let fist = exa_formats::weapon_record::WeaponRecord::build(
            &exa_formats::weapon_record::Section::FISTS,
            false,
            exa_formats::weapon_record::Lengths::hand(-8.0, 9.66),
            &exa_formats::weapon_record::Holder::AVERAGE,
        );
        for slot in [
            exa_combat::attack::Slot::First,
            exa_combat::attack::Slot::Second,
        ] {
            app.world_mut()
                .spawn(Blade::new(foe, 0.1, fist).in_slot(slot));
        }
        (app, foe, root)
    }

    fn state(app: &App, foe: Entity) -> (Stance, String) {
        let w = app.world();
        (
            *w.get::<Stance>(foe).unwrap(),
            w.get::<Animator>(foe).unwrap().action.clone(),
        )
    }

    fn engage(app: &mut App, foe: Entity) {
        app.world_mut().get_mut::<Duellist>(foe).unwrap().engaged = true;
    }

    /// One frame of one body tick.
    fn step(app: &mut App) {
        if let Some(mut ticks) = app.world_mut().get_resource_mut::<BodyTicks>() {
            ticks.0 += 1;
        } else {
            app.insert_resource(BodyTicks(1));
        }
        app.update();
    }

    fn velocity(app: &App, root: Entity) -> Vec3 {
        app.world()
            .get::<CharacterBody>(root)
            .unwrap()
            .desired_velocity
    }

    /// Entering combat sets the target through the estimate routine: a
    /// foe moving at 2 cm a tick along +X is led 60 cm (30 ticks) ahead of
    /// its chest; a standing one is not.
    #[test]
    fn set_target_leads_a_moving_foe() {
        use exa_physics::body::locomotion::{Locomotor, Stop};
        for (moving, lead) in [(true, 60.0), (false, 0.0)] {
            let (mut app, foe, _) = duel(2.0, false);
            let target = app.world().get::<Duellist>(foe).unwrap().target;
            app.world_mut().entity_mut(target).insert((
                Condition::NORMAL,
                Locomotor {
                    yaw: std::f32::consts::FRAC_PI_2,
                    speed: 2.0,
                    stop: if moving { Stop::Moving } else { Stop::Standing },
                    ..Locomotor::default()
                },
            ));
            engage(&mut app, foe);
            step(&mut app);
            let at = *app.world().get::<GlobalTransform>(target).unwrap();
            let chest = stance::chest_point(&at, None) * CM;
            let mind = &app.world().get::<Duellist>(foe).unwrap().mind;
            let off = mind.estimate - chest;
            assert!((off.x - lead).abs() < 1e-3, "{moving}: {off:?}");
            assert!(off.y.abs() < 1e-3 && off.z.abs() < 1e-3, "{off:?}");
            assert!(mind.fresh_target);
        }
    }

    /// The perception pass fills a contact for the target: seen, its
    /// distance and its closeness.
    #[test]
    fn the_duellist_keeps_a_contact_with_its_target() {
        let (mut app, foe, _) = duel(2.0, false);
        engage(&mut app, foe);
        app.update();
        app.update();
        let target = app.world().get::<Duellist>(foe).unwrap().target;
        let combatant = app.world().get::<Combatant>(foe).unwrap();
        assert_eq!(combatant.target, Some(target));
        let contact = combatant.contact(target).unwrap();
        assert!(contact.tracked());
        assert_ne!(contact.flags & combatant::CONTACT_NOTICED, 0);
        assert!((contact.distance_cm - 200.0).abs() < 0.1);
        assert!((contact.closeness - (1.0 - 200.0 / 1200.0)).abs() < 1e-5);
        // Facing each other 2 m apart: the observer's view weight is 1.
        assert!((contact.view - 1.0).abs() < 1e-5, "{}", contact.view);
    }

    /// It stands idle until the fight starts, then takes the guard, its
    /// eagerness at a half.
    #[test]
    fn the_duellist_enters_combat_in_its_guard() {
        let (mut app, foe, _) = duel(8.0, false);
        app.update();
        assert_eq!(state(&app, foe), (Stance::Idle, "Idle".into()));
        engage(&mut app, foe);
        app.update();
        assert_eq!(state(&app, foe), (Stance::Defence, "Defence stance".into()));
        assert_eq!(
            app.world().get::<Duellist>(foe).unwrap().mind.eagerness,
            0.5
        );
    }

    /// A duellist without a brain waits: its own contact rating no longer
    /// starts the fight; a brain's target pass does (the brain tests pin
    /// it).
    #[test]
    fn a_duellist_without_a_brain_waits_for_its_fight() {
        let (mut app, foe, _) = duel(8.0, false);
        let mut mind = Mind::new(-100.0);
        for _ in 0..20_000 {
            tick(&mut mind);
        }
        assert_eq!(decide(&mind), Decision::Engage);
        app.world_mut().entity_mut(foe).insert(mind);
        app.update();
        assert_eq!(state(&app, foe).0, Stance::Idle);
    }

    /// Fresh and at full energy, a human on guard stands in its combat idle
    /// (the guard comes back when it tires); standing its ground it never
    /// steps or strikes.
    #[test]
    fn a_still_duellist_stands_in_its_combat_idle() {
        for distance in [8.0, 1.0] {
            let (mut app, foe, root) = duel(distance, true);
            engage(&mut app, foe);
            for _ in 0..600 {
                step(&mut app);
            }
            assert_eq!(state(&app, foe), (Stance::CombatIdle, "Cmbt - Idle".into()));
            assert!(app.world().get::<Attacking>(foe).is_none());
            assert_eq!(velocity(&app, root), Vec3::ZERO);
        }
    }

    /// Facing its foe 5 m away (past the approach band, so no place to
    /// strike from), it steps in once its eagerness and its press build
    /// up, walking toward the step's goal.
    #[test]
    fn the_duellist_steps_in_on_its_foe() {
        let (mut app, foe, root) = duel(5.0, false);
        engage(&mut app, foe);
        let mut walked = false;
        for _ in 0..1500 {
            step(&mut app);
            let v = velocity(&app, root);
            if v.length() > 0.0 {
                assert!(v.z > 0.0, "{v}");
                assert_eq!(state(&app, foe).1, "Combat Walk");
                walked = true;
                break;
            }
        }
        assert!(walked);
    }

    /// Within the approach band it strikes once eager enough, from where it
    /// stands (a forward strike place gives way to it), the attack ending in
    /// its stance; with under eight tenths of its energy it never does.
    #[test]
    fn the_duellist_strikes_when_eager_and_rested() {
        let (mut app, foe, root) = duel(3.0, false);
        engage(&mut app, foe);
        let mut struck = false;
        for _ in 0..1500 {
            step(&mut app);
            if app.world().get::<Attacking>(foe).is_some() {
                struck = true;
                break;
            }
        }
        assert!(struck);
        assert_eq!(velocity(&app, root), Vec3::ZERO);
        let anim = app.world().get::<Animator>(foe).unwrap();
        assert!(
            AttackMotion::of_action(&anim.action).is_some(),
            "{}",
            anim.action
        );
        let (mut tired, foe2, _) = duel(3.0, false);
        engage(&mut tired, foe2);
        let part = exa_physics::body::PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: bevy::math::Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.08),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        };
        let mut ragdoll = Ragdoll::new(&[part]).with_drive();
        ragdoll.drive.as_mut().unwrap().energy = 0.79;
        tired.world_mut().entity_mut(foe2).insert(ragdoll);
        for _ in 0..1500 {
            step(&mut tired);
            assert!(tired.world().get::<Attacking>(foe2).is_none());
        }
    }

    /// A duellist whose power system answers a usable power chooses it for
    /// the game to cast; one never started leaves the cast once 500 ms have
    /// passed, so it may choose again.
    #[test]
    fn the_duellist_chooses_a_power_its_power_system_offers() {
        let (mut app, foe, _) = duel(3.0, true);
        let mut slots = [0; cast::SLOTS];
        slots[0] = 0x401;
        let mut answers = [0; cast::SLOTS];
        answers[0] = cast::USABLE | 5;
        app.world_mut().entity_mut(foe).insert(DuelPowers {
            slots,
            answers,
            pool: 1.0,
            ..DuelPowers::default()
        });
        engage(&mut app, foe);
        let chosen = |app: &mut App| {
            app.world_mut()
                .get_mut::<DuelPowers>(foe)
                .unwrap()
                .chosen
                .take()
        };
        let mut first = None;
        for _ in 0..400 {
            step(&mut app);
            first = chosen(&mut app);
            if first.is_some() {
                break;
            }
        }
        assert_eq!(first, Some(0x401));
        assert!(app.world().get::<Duellist>(foe).unwrap().mind.casting);
        let started = app.world().resource::<BodyTicks>().millis();
        let mut again = None;
        for _ in 0..400 {
            step(&mut app);
            again = chosen(&mut app);
            if again.is_some() {
                break;
            }
        }
        assert_eq!(again, Some(0x401));
        // A body that never read casting lets the cast go past 500 ms.
        assert!(app.world().resource::<BodyTicks>().millis() - started > 500);
    }

    /// The combat ticks follow the body ticks: four at most a frame.
    #[test]
    fn the_combat_state_ticks_with_the_body() {
        let (mut app, foe, _) = duel(8.0, true);
        app.insert_resource(BodyTicks(100));
        engage(&mut app, foe);
        app.update();
        app.update();
        let practice = |app: &App| app.world().get::<Duellist>(foe).unwrap().last_tick;
        assert_eq!(practice(&app), Some(100));
        app.world_mut().resource_mut::<BodyTicks>().0 = 110;
        app.update();
        assert_eq!(practice(&app), Some(110));
    }
}

#[cfg(test)]
mod chase_tests {
    use super::*;
    use exa_world::nav::{Layer, NavCell, NavGrid};

    /// A flat 80 × 80 grid with its floors at 10 m (voxel 100 over base 0).
    fn yard() -> NavGrid {
        let mut grid = NavGrid::empty(80, 80, 0.0);
        for z in 0..80 {
            for x in 0..80 {
                grid.set_cell(
                    x,
                    z,
                    Layer::Lower,
                    NavCell {
                        floor: 100,
                        ..NavCell::default()
                    },
                );
            }
        }
        grid
    }

    /// A fighter whose foe is out of reach for 21 ticks plans a chase
    /// route on the navigation grid (the one pathfinder) on its plan turn,
    /// follows it, and walks its carrot while the combat tick ends at the
    /// chase; the route reaches the foe, so its contact is not marked.
    #[test]
    fn a_stuck_fighter_plans_its_chase_route_through_the_grid() {
        let floor = Vec3::new(0.0, 10.0, 0.0);
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_message::<exa_combat::AttackAskedHook>()
            .add_systems(Update, duellist)
            .insert_resource(yard());
        let target = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(floor + Vec3::new(0.0, 0.0, 8.0)),
                Fighter::default(),
            ))
            .id();
        let root = app
            .world_mut()
            .spawn((
                CharacterBody::default(),
                GlobalTransform::from_translation(floor),
            ))
            .id();
        let mut ai = Duellist::new(target);
        ai.engaged = true;
        let foe = app
            .world_mut()
            .spawn((
                ai,
                Transform::from_translation(floor),
                GlobalTransform::from_translation(floor),
                Animator::new("Default", "Idle"),
                Fighter::default(),
                ChildOf(root),
            ))
            .id();
        // The first tick enters the fight and sets the target, which clears
        // the stuck count; the count starts from there.
        app.insert_resource(BodyTicks(1));
        app.update();
        app.world_mut().get_mut::<Duellist>(foe).unwrap().mind.stuck = 20;
        for tick in 2..=6 {
            app.world_mut().resource_mut::<BodyTicks>().0 = tick;
            app.update();
        }
        let ai = app.world().get::<Duellist>(foe).unwrap();
        assert!(ai.chase.following, "the chase started");
        assert!(ai.chasing, "the combat tick ends at the chase");
        assert!(!ai.route.is_empty(), "the route was planned");
        // The route walks toward the foe (+z).
        assert!(ai.route[ai.route.len() - 1].z > 600.0);
        let combatant = app.world().get::<Combatant>(foe).unwrap();
        assert!(combatant.contacts.iter().all(|c| c.flags & 0x10 == 0));
        let body = app.world().get::<CharacterBody>(root).unwrap();
        assert!(body.desired_velocity.z > 0.0, "it walks the route");
    }

    /// The foe's swing word decides the published blow: its cleaving bit
    /// makes the arc a recovery (kind 4) and stops the sweep, either
    /// continuation bit marks the swing committed, and its other bits (the
    /// release) do neither.
    #[test]
    fn the_swing_word_publishes_the_recovery_and_the_commitment() {
        let mut world = World::new();
        let foe = world.spawn_empty().id();
        let mut blades = bevy::ecs::system::SystemState::<Query<&Blade>>::new(&mut world);
        let blades = blades.get(&world).unwrap();
        let anim = Animator::new("Default", "Attack Mid");
        let blow = |flags| published_blow(foe, &blades, &anim, 0.5, 0.0, 0.0, true, flags);
        let plain = blow(0);
        // Empty hands publish the unarmed sector, 110° each side.
        assert_eq!(plain.kind, 3);
        assert!(plain.sweeping);
        assert!(!plain.committed);
        let cleaving = blow(swing::flags::CLEAVING);
        assert_eq!(cleaving.kind, 4);
        assert!(!cleaving.sweeping);
        assert!(blow(swing::flags::CONTINUATION).committed);
        assert!(blow(swing::flags::CONTINUED).committed);
        let released = blow(swing::flags::RELEASED);
        assert_eq!(released.kind, 3);
        assert!(!released.committed);
        assert!(released.sweeping);
    }
}
