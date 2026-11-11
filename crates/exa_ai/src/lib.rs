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
pub mod drive;
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
        exa_physics::sim_app::add_frame_start(app);
        bridge_ai_types(app);
        app.init_resource::<presence::ActivationArea>()
            .init_resource::<rooms::RoomCells>()
            .init_resource::<brain::BrainClock>()
            .init_resource::<pass_turn::PassTurn>()
            .add_message::<presence::Entered>()
            .add_message::<brain::StatueByte>()
            .add_message::<exa_combat::AttackAskedHook>()
            .add_message::<exa_combat::BlowNoticed>()
            .add_message::<struck::Taught>()
            // The brains' reaction to the tick's blows follows the hit
            // chain in the post-step: the original's damage routine calls
            // the struck behaviour once per contact.
            .add_systems(
                FixedUpdate,
                struck::react_to_blows
                    .in_set(struck::HitReactions)
                    .in_set(exa_physics::body::BodyPostStep)
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
            // A standing character faces as its AI turned it, once every
            // brain pass of the tick has run.
            .add_systems(
                FixedUpdate,
                drive::face_standing
                    .in_set(exa_physics::body::TickBehaviour)
                    .after(BrainTick)
                    .before(exa_combat::condition::PostureTick),
            )
            // A requested motion starts before the behaviour reads it, for
            // every body, not only one already in a fight.
            .add_systems(
                FixedUpdate,
                begin_requested
                    .in_set(exa_physics::body::TickBehaviour)
                    .before(BrainTick),
            )
            .add_systems(
                BrainChain,
                (
                    (
                        drive::add_locomotion,
                        brain::death_hook,
                        brain::apply_new_roles,
                        brain::rate_the_player,
                    )
                        .chain(),
                    brain_clock_mark(0),
                    pass_turn::rotate_pass_turn,
                    brain::perceive_all,
                    brain_clock_mark(1),
                    brain::ai_tick,
                    brain_clock_mark(2),
                    (duellist, apply_feet_steps).chain().in_set(DuelSystems),
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

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_ai_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_message::<crate::struck::Taught>()
        .bridge_copy::<crate::brain::RatedPlayer>()
        .bridge_message::<crate::brain::StatueByte>()
        .bridge_message::<crate::presence::Entered>()
        .bridge_message::<exa_combat::AttackAskedHook>()
        .bridge_lend_resource::<crate::BrainTickClock>()
        .bridge_copy::<crate::Duellist>()
        .bridge_copy::<crate::drive::Driven>()
        .bridge_move_resource::<crate::brain::BrainClock>()
        .bridge_copy::<crate::brain::CharacterId>()
        .bridge_copy::<crate::brain::MindWarded>()
        .bridge_copy_in_resource::<crate::brain::RoleDatabase>()
        .bridge_copy::<crate::brain::Unseen>()
        .bridge_move_resource::<crate::pass_turn::PassTurn>()
        .bridge_copy::<crate::presence::Presence>()
        .bridge_copy::<crate::roles::Relations>()
        .bridge_copy::<crate::roles::Roles>()
        .bridge_move_resource::<crate::rooms::RoomCells>()
        .bridge_copy::<crate::sense::Mind>()
        .bridge_copy::<exa_character::record::CharacterData>()
        .bridge_copy::<exa_combat::AttackHook>()
        .bridge_move_resource::<exa_world::nav::NavGrid>()
        .bridge_move_resource::<exa_world::nav::footprint::NavFootprints>()
        .bridge_copy_resource::<exa_world::room_view::LevelRooms>();
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
    /// The body's item flags refuse every power (bit 0x10), so the chooser
    /// picks nothing.
    pub refuses: bool,
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
    /// A barricade or a blast the tick took instead of the foe (cm). While
    /// set, the combat tick has no character foe and aims here.
    focus: Option<Vec3>,
    /// Nothing pressed hard enough, so the foe and the focus were cleared.
    /// The next tick has neither and leaves combat.
    dropped: bool,
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
    /// The body's step as this tick's combat state left it, and whether
    /// it started a step: written into the body by [`apply_feet_steps`].
    feet_write: Option<FeetWrite>,
}

/// The body's step as the combat tick left it, whether it started a step,
/// and the body's target (the foe's chest, m, and scale).
type FeetWrite = (exa_physics::body::FootworkStep, bool, Option<(Vec3, f32)>);

/// A step the duellist walks: where it goes (m) and when it started.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Stepping {
    goal: Vec3,
    started_ms: u32,
    /// 2 for the second half of a long step.
    state: i32,
    /// The body's landing count when this step started, so the count the
    /// combat tick reads is how many feet have landed since.
    landed_from: u32,
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

    /// Its combat target: its foe while it is in the combat state, unless
    /// its tick took a barricade or a blast instead (no character foe) or
    /// dropped the foe, as the original clears the character's target when
    /// it leaves combat.
    pub fn combat_target(&self) -> Option<Entity> {
        (self.in_combat && self.focus.is_none() && !self.dropped).then_some(self.target)
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
        // The idle state takes the walk: the chase's route ends.
        self.route.clear();
        self.chase.following = false;
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
            focus: None,
            dropped: false,
            stab_aim: exa_combat::stab::StabAim::default(),
            feet_write: None,
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
/// What striking `record` teaches: the fighting estimate from its skills
/// and combat skill, its held weapon and its scale. No record is a base of 0.
fn foe_lesson(
    record: Option<&exa_formats::characters::CharacterRecord>,
    weapon: Option<(&exa_formats::weapon_stats::StatBytes, f32)>,
    scale: f32,
) -> f32 {
    let base = record.map_or(0.0, |r| {
        crate::estimate::fighting_base(&r.skills, r.combat_skill)
    });
    crate::estimate::fighting_estimate_with_base(base, weapon, scale)
}

fn reach_of(fighter: Entity, blades: &Query<&Blade>, unarmed: f32) -> f32 {
    blades
        .iter()
        .filter(|b| b.wielder == fighter && b.record.armed())
        .map(|b| b.record.reach_metres())
        .reduce(f32::max)
        .unwrap_or(unarmed)
}

/// A fighter's unarmed reach (metres): its race's, as the equip refresh
/// makes it the character's reach while it holds no weapon; a body without
/// a fighter keeps a human's.
fn unarmed_reach(fighter: Option<&Fighter>) -> f32 {
    fighter.map_or(stance::UNARMED_REACH, |f| f.stats.unarmed_reach)
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
/// The attack motion the guard lean reads. Not swinging: 0, so the lean
/// does not apply. A main-hand side is 5–7 and the other hand 8–10.
fn swing_motion(attacking: Option<&Attacking>) -> i32 {
    match attacking.map(|a| a.motion) {
        Some(AttackMotion::Main(i)) => 5 + i as i32,
        Some(AttackMotion::Other(i)) => 8 + i as i32,
        Some(AttackMotion::Stab) => 0xb,
        Some(AttackMotion::Overhead) => 0xc,
        None => 0,
    }
}

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

/// A character besides the foe, measured as the position score measures
/// the foe: from a point on its body, with its own reach, size and swing.
struct ContactMeasure {
    origin: Vec3,
    forward: Vec3,
    reach_cm: f32,
    size: f32,
    large: bool,
    skill: f32,
    blow: place::Blow,
    /// Its condition is the dead word. The position score then writes
    /// nothing, and the place keeps the zeros it started with.
    dead: bool,
    /// Condition, reach and whether it is moving, for a blow taken after a switch.
    posture: combat::Posture,
    /// Where the aim estimate puts them: the chest, led toward a step.
    lead: Vec3,
}

/// The combat state's view of a duel. The foe is the target. Another
/// character contact is scored from its own body (`contact_at`); a missing
/// measurement is the foe.
struct DuelField<'a> {
    foe: Vec3,
    heading: Vec3,
    /// The foe's body facing (unit).
    foe_forward: Vec3,
    spot: place::Spot,
    /// One entry per contact the tick walks. `None` scores the foe (the
    /// target, a barricade, a blast).
    contact_at: Vec<Option<ContactMeasure>>,
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
    fn score(&mut self, contact: usize, level: i32, point: Vec3) -> combat::PlaceScore {
        let measured = self.contact_at.get(contact).and_then(Option::as_ref);
        if measured.is_some_and(|m| m.dead) {
            return combat::PlaceScore::default();
        }
        let (origin, forward) = measured
            .map(|m| (m.origin, m.forward))
            .unwrap_or((self.foe, self.foe_forward));
        let mut spot = place::Spot { level, ..self.spot };
        if let Some(m) = measured {
            spot.other_reach_cm = m.reach_cm;
            spot.other_size = m.size;
            spot.other_large = m.large;
            spot.other_skill = m.skill;
            spot.blow = place::Blow {
                skill: self.spot.blow.skill,
                ..m.blow
            };
        }
        let p = place::score_place_at(point, origin, self.heading, forward, spot);
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

/// The step the position score reads, in centimetres. A recovery or stagger
/// step on the body wins, because that is the body's own step. Otherwise the
/// combat step, which is kept on the duellist rather than written onto the
/// body. Neither: not stepping, and `fallback_cm` is only a stand-in.
/// The step of a body that walks its footwork on its feet (one with
/// muscles), as its combat state reads it; None for a body without muscles,
/// which glides its interim step.
fn feet_step_of(ragdoll: Option<&Ragdoll>) -> Option<exa_physics::body::FootworkStep> {
    ragdoll
        .filter(|r| r.drive.is_some())
        .and_then(Ragdoll::footwork_step)
}

/// What the combat state's step does to the body's step fields, as the
/// original's AI writes them: a chosen place that is the step it is in
/// makes the step of the plain kind again (`marks.restart`), a place of no
/// step clears the queue; a new step aims the target at the place (the
/// chest plus its offset) and starts (stepping on, no balance step; the
/// body's start stamps its clock and clears its landing count); a long
/// step's second half re-aims the target and makes the step of the long
/// kind, without a new start; a step while one is under way queues the
/// place's offset (cm). Returns whether a step started.
fn feet_outcome(
    fw: &mut exa_physics::body::FootworkStep,
    step: combat::Footstep,
    marks: combat::StepMarks,
) -> bool {
    if marks.restart {
        fw.long = false;
    }
    if marks.clear_queue {
        fw.queue = 0;
    }
    match step {
        combat::Footstep::Start { goal, .. } => {
            fw.target = goal / CM;
            fw.stepping = true;
            fw.balance_step = false;
            true
        }
        combat::Footstep::Continue { goal, .. } => {
            fw.target = goal / CM;
            fw.long = true;
            false
        }
        combat::Footstep::Queue { offset, .. } => {
            fw.queue = 1;
            fw.queued = offset / CM;
            false
        }
        combat::Footstep::Stay => false,
    }
}

/// Writes each duellist's step back into its body, after the combat tick
/// ([`Duellist::feet_write`]); the body's balance and feet routines then
/// walk it.
pub(crate) fn apply_feet_steps(mut fighters: Query<(&mut Duellist, &mut Ragdoll)>) {
    for (mut ai, mut body) in &mut fighters {
        if let Some((step, start, aim)) = ai.feet_write.take() {
            body.write_footwork(&step, start);
            // Debug aid: `EXA_NO_STEP_AIM` gives the body no target.
            let aim = aim.filter(|_| std::env::var_os("EXA_NO_STEP_AIM").is_none());
            body.aim_steps(aim);
        }
    }
}

fn step_measure(
    ragdoll: Option<&Ragdoll>,
    combat_goal_m: Option<Vec3>,
    fallback_cm: Vec3,
) -> (bool, Vec3) {
    if let Some(target) = ragdoll.and_then(Ragdoll::step_target) {
        return (true, target * CM);
    }
    if let Some(goal) = combat_goal_m {
        return (true, goal * CM);
    }
    (false, fallback_cm)
}

/// Where the position score measures `who` from. No transform: the caller
/// leaves the contact out, rather than scoring it as the foe. A body under
/// `who` supplies the chest, the reach and the swing; without one, `who`
/// stands unarmed. `combat_goal_m` is its combat step, when it has one.
#[allow(clippy::type_complexity)]
fn measure_contact(
    who: Entity,
    observer_cm: Vec3,
    targets: &Query<(&GlobalTransform, Has<brain::MindWarded>)>,
    bodies: &Query<
        (
            &Condition,
            &GlobalTransform,
            Option<&Ragdoll>,
            &Fighter,
            Option<&Animator>,
            Option<&exa_physics::body::locomotion::Locomotor>,
            Option<&exa_character::record::CharacterData>,
        ),
        Without<Duellist>,
    >,
    children: &Query<&Children>,
    gear: (
        &Query<&Blade>,
        &Query<&SwingState, Without<Duellist>>,
        Option<&Motions>,
        Option<Vec3>,
        &Query<&Attacking>,
        f32,
    ),
    skill: f32,
) -> Option<ContactMeasure> {
    let (root, _) = targets.get(who).ok()?;
    let found = std::iter::once(who)
        .chain(children.iter_descendants(who))
        .find_map(|e| bodies.get(e).ok().map(|b| (e, b)));
    let fighter_entity = found.as_ref().map(|(e, _)| *e).unwrap_or(who);
    let body = found.as_ref().map(|(_, b)| *b);
    let chest = body
        .map(|(_, gt, rag, ..)| stance::chest_point(gt, rag))
        .unwrap_or_else(|| stance::chest_point(root, None))
        * CM;
    let forward = root.rotation() * Vec3::Z;
    let condition = body.map(|(c, ..)| *c).unwrap_or(Condition::NORMAL);
    let fighter = body.map(|(_, _, _, f, ..)| f);
    let anim = body.and_then(|(_, _, _, _, a, _, _)| a);
    let locomotor = body.and_then(|(.., l, _)| l);
    let (blades, swings, motions, combat_goal_m, attacks, observer_skill) = gear;
    let (stepping, step_goal_cm) =
        step_measure(body.and_then(|(_, _, rag, ..)| rag), combat_goal_m, chest);
    let phase = anim
        .zip(motions)
        .and_then(|(a, m)| m.clip(a, &a.action).map(|c| a.clip_progress(c)))
        .unwrap_or(0.0);
    let origin = place::measured_origin(place::Origin {
        body_cm: root.translation() * CM,
        chest_cm: chest,
        forward,
        speed_cm: locomotor.map_or(0.0, |l| l.speed),
        condition: condition.0,
        locomotion: locomotor.is_some_and(|l| l.amount != 0.0),
        stepping,
        step_goal_cm,
        attacking: condition == Condition::ATTACKING,
        phase,
        skill,
    });
    let swing_flags = swings.get(fighter_entity).map_or(0, |state| state.flags);
    let yaw = atan2_xz(forward.x, forward.z);
    let mut blow = anim.map_or(place::Blow::default(), |anim| {
        published_blow(
            fighter_entity,
            blades,
            anim,
            phase,
            yaw,
            yaw,
            condition == Condition::ATTACKING,
            swing_flags,
        )
    });
    let angle = atan2_xz(observer_cm.x - origin.x, observer_cm.z - origin.z);
    blow.angle = angle;
    blow.origin_in_arc = place::between(blow.arc_end, blow.arc_start, angle);
    Some(ContactMeasure {
        origin,
        forward,
        reach_cm: reach_of(fighter_entity, blades, unarmed_reach(fighter)) * CM,
        size: fighter.map_or(1.0, |f| f.stats.size),
        large: fighter.is_some_and(|f| f.stats.rules.has(1)),
        skill,
        blow,
        dead: condition.is_dead(),
        posture: combat::Posture {
            condition: condition.0,
            reach_cm: reach_of(fighter_entity, blades, unarmed_reach(fighter)) * CM,
            running: locomotor.is_some_and(|l| l.moving()),
            crouch: body
                .and_then(|(_, _, rag, ..)| rag)
                .map_or(0.0, Ragdoll::crouch),
            motion: swing_motion(attacks.get(fighter_entity).ok()),
        },
        lead: {
            let walk = foe_walk(locomotor, forward);
            combat::foe_estimate(
                &combat::Foe {
                    chest,
                    step_goal: step_goal_cm,
                    stepping,
                    running: walk.moving,
                    facing: walk.facing,
                    forward_speed: walk.speed_cm,
                    ..combat::Foe::default()
                },
                observer_skill,
            )
        },
    })
}

/// Where the friend test stands a character: the body's anchor, or the chest
/// before one is placed, and the step they are on. No transform: they are not
/// here to stand in a strike. `concern_hostility` is absent when the fighter
/// has no contact for them, and they are still a friend.
#[allow(clippy::type_complexity)]
fn friend_bystander(
    who: Entity,
    is_foe: bool,
    concern_hostility: Option<f32>,
    targets: &Query<(&GlobalTransform, Has<brain::MindWarded>)>,
    bodies: &Query<
        (
            &Condition,
            &GlobalTransform,
            Option<&Ragdoll>,
            &Fighter,
            Option<&Animator>,
            Option<&exa_physics::body::locomotion::Locomotor>,
            Option<&exa_character::record::CharacterData>,
        ),
        Without<Duellist>,
    >,
    children: &Query<&Children>,
    combat_goal_m: Option<Vec3>,
) -> Option<combat::Bystander> {
    let (root, _) = targets.get(who).ok()?;
    let body = std::iter::once(who)
        .chain(children.iter_descendants(who))
        .find_map(|e| bodies.get(e).ok());
    let chest = body
        .map(|(_, gt, rag, ..)| stance::chest_point(gt, rag))
        .unwrap_or_else(|| stance::chest_point(root, None))
        * CM;
    let rag = body.and_then(|(_, _, rag, ..)| rag);
    let (stepping, step_goal) = step_measure(rag, combat_goal_m, chest);
    Some(combat::Bystander {
        position: rag
            .and_then(|rag| rag.drive.as_ref().and_then(|drive| drive.anchor()))
            .map(|at| at * CM)
            .unwrap_or(chest),
        stepping,
        step_goal,
        out: body.is_some_and(|(c, ..)| c.is_dead()),
        is_foe,
        concern_hostility,
    })
}

/// Starts a motion a body was asked for, before the behaviour reads it.
/// With muscles and not fallen, a free body starts it and the request is
/// dropped. A busy body drops it too, unless it is walking toward a goal,
/// which keeps it. A body without muscles, or a fallen one, keeps it.
#[allow(clippy::type_complexity)]
fn begin_requested(
    motions: Option<Res<Motions>>,
    mut grips: Query<&mut exa_physics::grip::HeldGrip>,
    mut bodies: Query<(
        &mut Animator,
        &Condition,
        Option<&mut Stance>,
        (
            Option<&mut Ragdoll>,
            Option<&mut Fighter>,
            Option<&exa_physics::body::locomotion::Locomotor>,
            Option<&exa_physics::body::locomotion::Locomotion>,
        ),
    )>,
) {
    for (mut anim, condition, mut stance, (mut ragdoll, mut fighter, locomotor, walk)) in
        &mut bodies
    {
        let pending = anim.requested;
        let (next, start) = combat::take_requested(
            pending,
            combat::RequestGate {
                muscles: ragdoll.as_ref().is_some_and(|body| body.drive.is_some()),
                fallen: condition.fallen_or_dying(),
                busy: condition.busy(),
                moving: locomotor.is_some_and(|step| step.moving()),
                free_walk: walk.is_some_and(|step| step.free_walk),
            },
        );
        if start {
            // A parry in progress ends before the clip, and the clip returns
            // to the stance that end chose.
            if let Some(fighter) = fighter.as_mut() {
                let dazed = ragdoll
                    .as_ref()
                    .and_then(|body| body.drive.as_ref())
                    .is_some_and(|drive| drive.dazed());
                if fighter.parry.active
                    && let Some(body) = ragdoll.as_mut()
                {
                    body.clear_parry_arms(exa_physics::body::Ragdoll::PARRY_HANDS);
                    if let Some(key) = body.parts.first().map(|part| part.entity) {
                        for mut grip in &mut grips {
                            if grip.holder_key == key
                                && let Some(links) = grip.links.as_mut()
                            {
                                links.release_parry();
                            }
                        }
                    }
                }
                if let Some(next) = exa_combat::parry::end_for_request(
                    fighter,
                    condition.clip_stands_still(),
                    dazed,
                ) && let Some(slot) = stance.as_mut()
                {
                    **slot = next;
                }
            }
            let then = stance
                .as_deref()
                .copied()
                .map(Stance::action)
                .unwrap_or("Idle");
            if pending == combat::DRAW_WEAPON {
                anim.play_once(combat::DRAW_WEAPON_ACTION, then);
            } else if let Some(set) = motions.as_deref() {
                set.start_requested(&mut anim, pending as u32);
            }
            if anim.snap_start {
                anim.pending = None;
                anim.leading = false;
                anim.capture = None;
            }
        }
        anim.requested = next;
        if next == 0 {
            anim.snap_start = false;
        }
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
            Option<&exa_character::record::CharacterData>,
        ),
        Without<Duellist>,
    >,
    children: Query<&Children>,
    (blades, target_swings, attacks): (
        Query<&Blade>,
        Query<&SwingState, Without<Duellist>>,
        Query<&Attacking>,
    ),
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
        (Has<Fallen>, Has<exa_combat::block::ShieldHold>),
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
            Option<&mut exa_physics::body::locomotion::Locomotion>,
            Option<&mut exa_character::record::CharacterData>,
            Option<&mut drive::Driven>,
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
    // Each duellist's combat step, read before any of them moves, so a later
    // one is scored against the step the earlier one had at the start.
    let combat_steps: Vec<(Entity, Vec3)> = npcs
        .iter()
        .filter_map(|(entity, ai, ..)| ai.step.map(|step| (entity, step.goal)))
        .collect();
    for (
        entity,
        mut ai,
        mut stance,
        mut t,
        mut anim,
        (mut attacker, mut swinging),
        parent,
        attacking,
        (fallen, shield),
        mut combatant,
        condition,
        gt,
        ragdoll,
        fighter,
        (mut powers, presence, attack_hook, locomotor, mut walk, mut record, mut driven),
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
        let foe_motion = foe_walk(target_body.and_then(|(.., l, _)| l), target_forward);
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
            // facing at its speed, a stepping foe three quarters of the way
            // to the step), the set-target's fresh flag, commitment
            // and cleared overshoot and stuck counts are set, and the
            // bearing and aim are measured from the chest; the brain's look
            // takes the aim's yaw too.
            let chest_cm = stance::chest_point(gt, ragdoll) * CM;
            let (stepping, step_goal) = step_measure(
                target_body.and_then(|(_, _, rag, ..)| rag),
                combat_steps
                    .iter()
                    .find(|(id, _)| *id == foe_key)
                    .map(|(_, goal)| *goal),
                foe_chest,
            );
            let lead = combat::foe_estimate(
                &combat::Foe {
                    chest: foe_chest,
                    step_goal,
                    stepping,
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
            let reach_cm = reach_of(entity, &blades, fighter.stats.unarmed_reach) * 100.0;
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
        let target_fighter = target_body.map(|(_, _, _, f, _, _, _)| f);
        let target_anim = target_body.and_then(|(_, _, _, _, a, _, _)| a);
        let target_locomotor = target_body.and_then(|(_, _, _, _, _, l, _)| l);
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
        // A focus has cleared the body target, so the old character is not
        // perceived and is not who the fighter attends.
        if ai.focus.is_some() || ai.dropped {
            combatant.target = None;
        } else if brains.get(entity).is_err() {
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
        // with its own contacts and the relations' trust. A focus attends
        // that entity instead, so the old character stays unattended.
        if ai.focus.is_none() && !ai.dropped {
            combatant.target = Some(foe_key);
            let contact = &mut combatant.contacts[index];
            if target_condition.is_dead() && contact.died_ms.is_none() {
                contact.died_ms = Some(now);
            }
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

        // A body with muscles walks its footwork on its own feet, as the
        // original's AI writes its steps into its body's step fields (the
        // fields the player's footwork writes) and its body's balance and
        // feet routines walk them: the combat tick reads the body's step as
        // it is ([`feet_step_of`]) and writes it back after its tick
        // ([`apply_feet_steps`]). A body without muscles glides its
        // interim step below.
        let mut feet_step = feet_step_of(ragdoll);
        if let Some(fw) = feet_step.as_mut() {
            // A queued step the body took on its last landing was aimed at
            // the chest plus the queued vector (bent at the target's circle
            // by the body's own taker), then line-walked from the chest.
            // Exarust's body has no grid, so the AI walks the line on its
            // next tick from the chest it has then.
            if fw.queue < 0 {
                let chest = stance::chest_point(gt, ragdoll) * CM;
                let mut to = fw.target * CM;
                ground::walk_line(nav.as_deref(), chest, &mut to);
                fw.target = to / CM;
            }
            ai.queued = None;
            ai.queue = fw.queue;
            ai.step = fw.stepping.then(|| Stepping {
                goal: fw.target,
                started_ms: (now as u32).wrapping_sub(fw.age_ms.max(0.0) as u32),
                state: if fw.long { 2 } else { 0 },
                landed_from: ragdoll
                    .map_or(0, Ragdoll::landings)
                    .saturating_sub(fw.landed),
            });
        }
        // The step it walks ends at its goal, or when it has taken too long;
        // a queued one starts then.
        if feet_step.is_some() {
        } else if let Some(step) = ai.step {
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
                        landed_from: ragdoll.map_or(0, Ragdoll::landings),
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
        let reach = reach_of(entity, &blades, fighter.stats.unarmed_reach);
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
        // chest by 70 ticks of its speed while it moves, or part way to its
        // step's goal while it steps.
        let (foe_stepping, foe_step_cm) = step_measure(
            target_body.and_then(|(_, _, rag, ..)| rag),
            combat_steps
                .iter()
                .find(|(id, _)| *id == foe_key)
                .map(|(_, goal)| *goal),
            foe_chest,
        );
        let foe_origin = place::measured_origin(place::Origin {
            body_cm: target.translation() * CM,
            chest_cm: foe_chest,
            forward: target_forward,
            speed_cm: target_speed_cm,
            condition: target_condition.0,
            locomotion: target_moving,
            stepping: foe_stepping,
            step_goal_cm: foe_step_cm,
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
            reach_cm: reach_of(foe_key, &blades, unarmed_reach(target_fighter)) * CM,
            chest: foe_chest,
            step_goal: foe_step_cm,
            stepping: foe_stepping,
            running: foe_motion.moving,
            forward_speed: foe_motion.speed_cm,
            facing: foe_motion.facing,
            scale: target_fighter.map_or(1.0, |f| f.stats.scale),
            rules: target_fighter.map_or(0, |f| f.stats.rules.0),
            weapon_class: u32::from(weapon_class_of(foe_key, &blades)),
            attack_flags: target_swing,
            lesson: foe_lesson(
                target_body.and_then(|(.., data)| data.map(|d| &d.record)),
                blades
                    .iter()
                    .find(|b| b.wielder == foe_key && b.record.armed())
                    .map(|b| {
                        (
                            &b.record.stats,
                            reach_of(foe_key, &blades, unarmed_reach(target_fighter)) * CM,
                        )
                    }),
                target_fighter.map_or(1.0, |f| f.stats.scale),
            ),
            crouch: target_body
                .and_then(|(_, _, rag, ..)| rag)
                .map_or(0.0, Ragdoll::crouch),
            // No measured body keeps the default, supported. A body with a
            // drive answers whether a foot holds it up.
            supported: target_body
                .and_then(|(_, _, rag, ..)| rag)
                .and_then(|rag| rag.drive.as_ref())
                .is_none_or(|drive| drive.balance().has_support()),
            motion: swing_motion(attacks.get(foe_key).ok()),
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
        let mut feet_started = false;
        for _ in 0..runs {
            let yaw = yaw_of(t.rotation);
            let stepping = ai.step;
            let mut me = combat::Body {
                now_ms: now as u32,
                condition: condition.0,
                power_pool: powers.as_deref().map_or(0.0, |p| p.pool),
                power_busy: powers.as_deref().is_some_and(|p| p.refuses),
                power_slots: powers.as_deref().map_or([0; cast::SLOTS], |p| p.slots),
                clock: anim.progress,
                attack_flags: swinging.flags,
                recoil_until_ms: swinging.end_ms.min(u64::from(u32::MAX)) as u32,
                weapon_class: u32::from(class),
                held: blades
                    .iter()
                    .filter(|b| b.wielder == entity && b.record.armed())
                    .count() as i32,
                motion: if anim.action == combat::DRAW_WEAPON_ACTION {
                    combat::DRAW_WEAPON
                } else {
                    motion_of(attacking, *stance)
                },
                carried: record.as_deref().map_or(0, |data| data.hands[2].id),
                requested_motion: anim.requested,
                energy,
                max_energy: most,
                balance: ragdoll
                    .and_then(|body| body.drive.as_ref())
                    .map_or(1.0, |drive| drive.balance().value),
                tone: ragdoll
                    .and_then(|body| body.drive.as_ref())
                    .map_or(1.0, |drive| drive.tone),
                support: ragdoll
                    .and_then(|body| body.drive.as_ref())
                    .map_or(1.0, |drive| drive.support()),
                scale: fighter.stats.scale,
                feral: fighter.stats.feral,
                attack_bonus: fighter.stats.attack_bonus,
                quadruped: fighter.stats.quadruped,
                heavy_claw: fighter.stats.heavy_claws,
                // Held while the off hand carries a shield.
                shield,
                size: fighter.stats.size,
                encumbrance: fighter.stats.encumbrance,
                reach_cm: reach * CM,
                step_cm: spot.my_step_cm,
                step_factor: ragdoll
                    .and_then(|body| body.drive.as_ref())
                    .map_or(1.0, |drive| drive.balance().extension()),
                stepping: stepping.is_some(),
                step_state: stepping.map_or(0, |s| s.state),
                step_started_ms: stepping.map_or(0, |s| s.started_ms),
                landings: stepping.map_or(0, |s| {
                    ragdoll.map_or(0, |body| body.landings().saturating_sub(s.landed_from)) as i32
                }),
                queue: ai.queue,
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
                target_point: combat::target_point(foe_chest, target.translation().y * CM),
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
                running: locomotor.is_some_and(|l| l.moving()),
                forward_speed: locomotor.map_or(0.0, |l| l.speed),
                // The body's parrying flag: while its auto-parry holds, the
                // combat state leaves its stance motion alone.
                parrying: fighter.parry.active,
                parry_bias: fighter.parry.counter,
            };
            me.can_step = combat::can_act(&me) && !ai.holds_ground;
            ai.queue = 0;
            let ai_target_bits = ai.target.to_bits();
            let focused = if ai.dropped { None } else { ai.focus };
            let no_foe = focused.is_some() || ai.dropped;
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
                contact_at: Vec::new(),
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
            {
                let held = &mut combatant.contacts[index];
                if held.flags & crate::contacts::UNREACHABLE != 0
                    && held.unreachable_until_ms < now as u32
                {
                    held.flags &= !crate::contacts::UNREACHABLE;
                }
            }
            let marked = combatant.contacts[index].flags & crate::contacts::UNREACHABLE != 0;
            let foe_id = combatant.contacts[index].character.to_bits();
            // The target pass stored each character's level. The foe stays at
            // the fight level when this duellist has no brain record of it.
            let levels = brains.get(entity).ok().map(|npc| {
                npc.contacts
                    .iter()
                    .filter(|c| !c.object)
                    .map(|c| (c.id, c.level))
                    .collect::<Vec<_>>()
            });
            let queries = (&blades, &target_swings, motions.as_deref());
            let mut contacts = Vec::new();
            let mut contact_at = Vec::new();
            for c in &combatant.contacts {
                let id = c.character.to_bits();
                let is_foe = id == foe_id && !no_foe;
                let level = levels
                    .as_ref()
                    .and_then(|levels| levels.iter().find(|(contact, _)| *contact == id))
                    .map(|(_, level)| *level)
                    .unwrap_or(if is_foe { 3 } else { 0 });
                let estimate = if is_foe {
                    Some(combat::foe_estimate(&foe, traits.skill))
                } else {
                    None
                };
                let measured = if is_foe {
                    None
                } else {
                    let skill = brains
                        .get(c.character)
                        .ok()
                        .and_then(|b| b.personality)
                        .map_or(0.0, |p| p.skill);
                    let combat_goal = combat_steps
                        .iter()
                        .find(|(id, _)| *id == c.character)
                        .map(|(_, goal)| *goal);
                    let Some(measured) = measure_contact(
                        c.character,
                        me_cm,
                        &targets,
                        &bodies,
                        &children,
                        (
                            queries.0,
                            queries.1,
                            queries.2,
                            combat_goal,
                            &attacks,
                            traits.skill,
                        ),
                        skill,
                    ) else {
                        continue;
                    };
                    Some(measured)
                };
                contacts.push(combat::Contact {
                    level,
                    is_foe,
                    character: true,
                    concern: c.concern,
                    hostility: c.hostility,
                    unreachable: c.flags & crate::contacts::UNREACHABLE != 0,
                    who: Some(c.character),
                    estimate: estimate.or_else(|| measured.as_ref().map(|step| step.lead)),
                    posture: if is_foe {
                        Some(combat::Posture {
                            condition: foe.condition,
                            reach_cm: foe.reach_cm,
                            running: foe.running,
                            crouch: target_body
                                .and_then(|(_, _, rag, ..)| rag)
                                .map_or(0.0, Ragdoll::crouch),
                            motion: swing_motion(attacks.get(foe_key).ok()),
                        })
                    } else {
                        measured.as_ref().map(|step| step.posture)
                    },
                    ..combat::Contact::default()
                });
                contact_at.push(measured);
            }
            if let Ok(npc) = brains.get(entity) {
                for step in &npc.wall_steps {
                    contacts.push(combat::Contact {
                        character: false,
                        wall: Some(*step),
                        estimate: Some(step.centre),
                        ..combat::Contact::default()
                    });
                    contact_at.push(None);
                }
                for step in &npc.blast_steps {
                    contacts.push(combat::Contact {
                        character: false,
                        blast: Some(*step),
                        estimate: Some(step.at),
                        ..combat::Contact::default()
                    });
                    contact_at.push(None);
                }
            }
            field.contact_at = contact_at;
            // The room list's nearby characters, once it has gathered any.
            // Before that, the characters this fighter is already in contact
            // with. Someone with no contact is still a friend.
            let nearby = brains
                .get(entity)
                .ok()
                .filter(|npc| !npc.nearby.is_empty())
                .map(|npc| npc.nearby.clone());
            let ids = nearby.unwrap_or_else(|| {
                combatant
                    .contacts
                    .iter()
                    .map(|c| c.character.to_bits())
                    .collect::<Vec<_>>()
            });
            let foe_body = foe_key.to_bits();
            let mut around = Vec::with_capacity(ids.len());
            for id in ids {
                let held = combatant
                    .contacts
                    .iter()
                    .find(|c| c.character.to_bits() == id);
                let goal = combat_steps
                    .iter()
                    .find(|(who, _)| who.to_bits() == id)
                    .map(|(_, goal)| *goal);
                if let Some(person) = friend_bystander(
                    Entity::from_bits(id),
                    !no_foe && (id == foe_id || id == foe_body),
                    held.map(|c| c.concern + c.hostility),
                    &targets,
                    &bodies,
                    &children,
                    goal,
                ) {
                    around.push(person);
                }
            }
            let out = combat::tick(
                mind,
                &me,
                traits,
                combat::World {
                    foe: (!no_foe).then_some(&foe),
                    focus: focused,
                    contacts: &mut contacts,
                    around: &around,
                    field: &mut field,
                    leader_calls: false,
                },
                roll,
            );
            // A gain past 10 pays four times its whole points into the
            // record's experience.
            if out.practice_paid != 0
                && let Some(data) = record.as_mut()
            {
                data.record.experience = data.record.experience.wrapping_add(out.practice_paid);
            }
            if out.clear_slow_turn
                && let Some(walk) = walk.as_mut()
            {
                walk.slow_turn = false;
            }
            let chased = field.chase.and_then(|c| c.outcome);
            placed = matches!(out.exit, None | Some(combat::Exit::NoPlace));
            if marked && contacts.iter().any(|c| c.is_foe && !c.unreachable) {
                // The foe is in reach again.
                combatant.contacts[index].flags &= !crate::contacts::UNREACHABLE;
            }
            if let Some(b) = out.bearing {
                ai.bearing = (b.to_target, b.distance_cm);
            }
            // A barricade or a blast that pressed harder: the foe is dropped
            // and the aim goes to that entity. The commitment is not reset.
            if let Some(point) = out.focus {
                ai.focus = Some(point);
                ai.mind.estimate = point;
                let flat = Vec3::new(point.x - chest.x, 0.0, point.z - chest.z);
                let d = flat.length();
                let dir = if d > 1.0 { flat / d } else { ai.bearing.0 };
                ai.bearing = (dir, if d > 1.0 { d } else { 1.0 });
                ai.mind.aim = atan2_xz(dir.x, dir.z);
                combatant.target = None;
                if let Ok(mut brain) = brains.get_mut(entity) {
                    brain.turn.look = ai.mind.aim;
                }
                info!("duellist {entity:?} focuses a force at {point:?}");
                ai.dropped = false;
            }
            // Nothing beat the floor: the foe and the focus are both cleared.
            // The next tick has neither and leaves.
            if out
                .search
                .as_ref()
                .is_some_and(|found| found.retarget == Some(None))
            {
                ai.focus = None;
                ai.dropped = true;
                combatant.target = None;
                info!("duellist {entity:?} drops its foe and its focus: nothing pressed");
            }
            // A character that pressed harder than the foe becomes the target.
            // The estimate is placed on them and the aim turns to it, as a
            // newly taken foe.
            if let Some(who) = out.adopted.filter(|who| *who != ai.target)
                && let Ok((transform, _)) = targets.get(who)
            {
                let found = std::iter::once(who)
                    .chain(children.iter_descendants(who))
                    .find_map(|e| bodies.get(e).ok());
                let their_chest = found
                    .as_ref()
                    .map(|(_, gt, rag, ..)| stance::chest_point(gt, *rag))
                    .unwrap_or_else(|| stance::chest_point(transform, None))
                    * CM;
                let forward = transform.rotation() * Vec3::Z;
                let walk = foe_walk(found.as_ref().and_then(|(.., l, _)| *l), forward);
                let (stepping, step_goal) = step_measure(
                    found.as_ref().and_then(|(_, _, rag, ..)| *rag),
                    combat_steps
                        .iter()
                        .find(|(id, _)| *id == who)
                        .map(|(_, goal)| *goal),
                    their_chest,
                );
                let lead = combat::foe_estimate(
                    &combat::Foe {
                        chest: their_chest,
                        step_goal,
                        running: walk.moving,
                        facing: walk.facing,
                        forward_speed: walk.speed_cm,
                        stepping,
                        ..combat::Foe::default()
                    },
                    traits.skill,
                );
                let previous = ai.bearing.0;
                let bearing = combat::set_target(&mut ai.mind, lead, chest, previous);
                ai.bearing = (bearing.to_target, bearing.distance_cm);
                ai.target = who;
                ai.focus = None;
                ai.dropped = false;
                combatant.target = Some(who);
                if let Ok(mut brain) = brains.get_mut(entity) {
                    brain.turn.look = ai.mind.aim;
                }
                info!("duellist {entity:?} takes {who:?} as its foe");
            }
            if let Some(stopped) = out.stop_run
                && let Some(step) = ai.step.as_mut()
            {
                step.goal = stopped.step_goal / CM;
                if let Some(fw) = feet_step.as_mut() {
                    fw.target = stopped.step_goal / CM;
                }
            }
            // The bits the tick set, and the clear it asks for once the
            // swing is over: everything but the idle mask goes.
            if out.blow.clear_flags {
                swinging.flags &= swing::flags::KEPT_IDLE;
            }
            swinging.flags |= out.blow.set_flags;
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
            if matches!(out.exit, Some(combat::Exit::Leave { .. })) {
                info!("duellist {entity:?} leaves combat: it has no foe and no focus");
                ai.leave();
                ai.dropped = false;
                ai.focus = None;
                combatant.in_combat = false;
                combatant.target = None;
                if let Ok(mut brain) = brains.get_mut(entity) {
                    let body_look = crate::motion::yaw_of(t.rotation * Vec3::Z);
                    let brain = &mut *brain;
                    // The wide pan waits three seconds and the sway and the
                    // narrow pan half a second, the timers the leave returns.
                    crate::motion::leave_combat(
                        &mut brain.gaze,
                        &mut brain.turn,
                        &mut brain.alert,
                        body_look,
                        now as u32,
                    );
                    crate::lifecycle::restart_idle(brain);
                    info!(
                        "duellist {entity:?} returns to its idle state {:?}, looking at \
                         {body_look:.3} rad; the wide pan waits until {} ms, the narrow pan \
                         until {} ms",
                        brain.state(),
                        brain.alert.wide_at_ms,
                        brain.alert.narrow_at_ms
                    );
                }
            }
            if let Some(outcome) = chased {
                // Debug aid: `EXA_DUEL_TRACE` also logs what the chase did.
                if std::env::var_os("EXA_DUEL_TRACE").is_some() {
                    info!(
                        "chase-trace {entity:?} end {:?} planned {:?} mark {} reached {}",
                        outcome.end,
                        outcome.planned.as_ref().map(|p| (
                            p.following,
                            p.route.points.len(),
                            p.route.points.last().copied()
                        )),
                        outcome.mark.is_some(),
                        outcome.reached.is_some()
                    );
                }
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
                // Without a route the walk follows nothing: the chase's
                // follow state ends with it.
                if ai.route.is_empty() {
                    ai.chase.following = false;
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
            // Debug aid: `EXA_DUEL_TRACE` logs what the combat tick read
            // and chose.
            if std::env::var_os("EXA_DUEL_TRACE").is_some() {
                info!(
                    "duel-trace {entity:?} t{} dist {:.0} can_step {} block {} run {} spd {:.2} \
                     tone {:.2} sup {:.2} stepping {} state {} queue {} chosen {:?} step {:?} exit {:?} \
                     mstep {} long {} cond {:#x} stuck {} why {} tp {:.0?} gy {:.0} \
                     blow {:?} ready {} energy {:.2}/{:.2} looping {} holds {} attacking {} \
                     reach {:.0} size {:.2}",
                    me.now_ms,
                    ai.bearing.1,
                    me.can_step,
                    me.blocking,
                    me.running,
                    me.forward_speed,
                    me.tone,
                    me.support,
                    me.stepping,
                    me.step_state,
                    me.queue,
                    out.chosen,
                    out.step,
                    out.exit,
                    ai.mind.step,
                    ai.mind.long_step,
                    me.condition,
                    ai.mind.stuck,
                    ai.mind.stuck_why,
                    me.target_point,
                    me.ground_y,
                    out.blow.motion,
                    me.can_attack,
                    me.energy,
                    me.max_energy,
                    anim.looping,
                    ai.holds_ground,
                    attacking.is_some(),
                    me.reach_cm,
                    me.size
                );
            }
            if out.marks.queue_taken
                && let Some(fw) = feet_step.as_mut()
            {
                fw.queue = 0;
            }
            if out.exit.is_some() {
                continue;
            }
            if let Some(fw) = feet_step.as_mut() {
                feet_started |= feet_outcome(fw, out.step, out.marks);
            }
            match out.step {
                combat::Footstep::Start { goal, .. } => {
                    ai.step = Some(Stepping {
                        goal: Vec3::new(goal.x / CM, feet.y, goal.z / CM),
                        started_ms: now as u32,
                        state: 0,
                        landed_from: ragdoll.map_or(0, Ragdoll::landings),
                    });
                }
                combat::Footstep::Continue { goal, .. } => {
                    let landed_from = ai
                        .step
                        .map_or_else(|| ragdoll.map_or(0, Ragdoll::landings), |s| s.landed_from);
                    ai.step = Some(Stepping {
                        goal: Vec3::new(goal.x / CM, feet.y, goal.z / CM),
                        started_ms: now as u32,
                        state: 2,
                        landed_from,
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
            // Empty hands ask the body to draw the stowed set. The body tick
            // starts it next frame, when the body is free to.
            if out.draw_weapon {
                anim.requested = combat::DRAW_WEAPON;
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
                // The side starters pick the side's clip themselves: aimed
                // at the foe's parry (a shield fighter's or one with two
                // weapons), else at random, never the side's last. The
                // stab's and the overhead's starters mark the foe's motion
                // they counter.
                let aim = attacker.aim(
                    Some(fighter),
                    ragdoll.map_or(0.0, Ragdoll::crouch) > 0.0,
                    false,
                    now,
                );
                let (motion, countered) = match motion {
                    AttackMotion::Main(_) | AttackMotion::Other(_) => {
                        let side = if matches!(motion, AttackMotion::Main(_)) {
                            exa_combat::attack::Side::Main
                        } else {
                            exa_combat::attack::Side::Other
                        };
                        let (pick, countered) = attacker.pick_aimed(side, &aim);
                        (side.motion(pick), countered)
                    }
                    _ => (motion, exa_combat::attack::countered_by(motion, &aim)),
                };
                info!(
                    "duellist {entity:?} attacks: {} (countering {countered:?})",
                    motion.action()
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
                        countered,
                        ..AttackStart::of(motion, class, now)
                    },
                    stance.action(),
                );
            }
        }
        // The body takes back its step as the combat state left it, and
        // its target (the foe's chest and scale), which its queued-step
        // taker bends a step at.
        let aim = target_body
            .filter(|_| ai.focus.is_none() && !ai.dropped)
            .map(|(_, _, _, f, ..)| (foe_chest / CM, f.stats.scale));
        ai.feet_write = feet_step.map(|fw| (fw, feet_started, aim));
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
        // A body on its locomotion walks the chase by the AI's driver at
        // the chase's urgency 1, on its feet, and soft-stops at its end.
        if let (Some(req), Some(motor), Some(driven)) =
            (walk.as_deref_mut(), locomotor, driven.as_deref_mut())
        {
            let at = drive::DriveBody::of(feet, motor, ragdoll);
            if ai.chasing && !ai.route.is_empty() {
                if std::env::var_os("EXA_DUEL_TRACE").is_some() {
                    let body_step = ragdoll.and_then(Ragdoll::footwork_step);
                    info!(
                        "chase-walk {entity:?} feet {:.0?} route {} carrot {:.0?} driving {} \
                         body_step {:?}",
                        feet * CM,
                        ai.route.len(),
                        crate::path::carrot(&ai.route, feet * CM).goal,
                        driven.0.driving,
                        body_step.map(|s| (
                            s.stepping,
                            s.balance_step,
                            s.long,
                            s.age_ms as i32,
                            s.queue,
                            s.landed,
                            (s.target * CM).round(),
                            (s.feet_centre * CM).round()
                        ))
                    );
                }
                if let Some(goal) = crate::path::carrot(&ai.route, feet * CM).goal {
                    let cost = |p: Vec3| drive::grid_cost(nav.as_deref(), p);
                    drive::drive(req, &mut driven.0, &at, goal / CM, true, 1, false, &cost);
                }
                // The path follower soft-stops a hurried walk within a
                // metre of the route's end, and the walk no longer follows:
                // the next combat tick runs on past the chase.
                let last = ai.route[ai.route.len() - 1];
                if crate::path::hurried_arrival(feet * CM, last) {
                    ai.route.clear();
                    ai.chase.following = false;
                    drive::soft_stop(req, &mut driven.0, &at);
                }
                continue;
            }
            if driven.0.driving {
                drive::soft_stop(req, &mut driven.0, &at);
            }
        }
        if ai.chasing && !ai.route.is_empty() {
            let carrot = crate::path::carrot(&ai.route, feet * CM);
            if let Some(goal) = carrot.goal {
                let order = follow(&[feet, goal / CM], feet, yaw_of(t.rotation), 0);
                body.desired_velocity = order.velocity;
                anim.play_loop(COMBAT_WALK);
            }
            let last = ai.route[ai.route.len() - 1];
            if crate::path::hurried_arrival(feet * CM, last) {
                ai.route.clear();
                ai.chase.following = false;
            }
            continue;
        }
        match ai.step {
            // A body on its feet steps by them in its stance.
            Some(step) if !ai.holds_ground && feet_step.is_none() => {
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

    /// A fighter of a human's unarmed reach (the built-in record's is 1 m).
    fn human() -> Fighter {
        let mut fighter = Fighter::default();
        fighter.stats.unarmed_reach = stance::UNARMED_REACH;
        fighter
    }

    /// An unarmed duellist on a root at the origin, its target `distance` m
    /// away along +Z.
    fn duel(distance: f32, holds_ground: bool) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_message::<exa_combat::AttackAskedHook>()
            .add_systems(
                Update,
                (begin_requested, duellist, apply_feet_steps).chain(),
            );
        let target = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, distance)),
                human(),
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
                human(),
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

    /// A tone under 0.7, or support that does not pass 0.7, leaves it
    /// standing. The same body, toned and supported, steps in: on its own
    /// feet (its body's step starts), with no glide.
    #[test]
    fn without_tone_or_support_it_does_not_step() {
        let walks = |tone: f32, support: f32| {
            let (mut app, fighter, root) = duel(5.0, false);
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
            let drive = ragdoll.drive.as_mut().unwrap();
            drive.tone = tone;
            drive.set_support(support);
            app.world_mut().entity_mut(fighter).insert(ragdoll);
            engage(&mut app, fighter);
            for _ in 0..1500 {
                step(&mut app);
                assert_eq!(velocity(&app, root), Vec3::ZERO);
                let body = app.world().get::<Ragdoll>(fighter).unwrap();
                if body.footwork_step().unwrap().stepping {
                    return true;
                }
            }
            false
        };
        assert!(!walks(0.69, 1.0));
        assert!(!walks(1.0, 0.7));
        assert!(walks(1.0, 1.0));
    }

    /// Moving faster than 3 cm a tick, the tick ends on the charge and no
    /// blow comes. At 3 it does not, and the blow comes.
    #[test]
    fn a_running_fighter_charges_instead_of_striking() {
        use exa_physics::body::locomotion::{Locomotor, Stop};
        let strikes_at = |speed: f32| {
            let (mut app, fighter, _) = duel(3.0, false);
            app.world_mut().entity_mut(fighter).insert(Locomotor {
                speed,
                stop: Stop::Moving,
                ..Locomotor::default()
            });
            engage(&mut app, fighter);
            strikes(&mut app, fighter)
        };
        assert!(!strikes_at(3.1));
        assert!(strikes_at(3.0));
    }

    /// A long step's second half is scaled by the legs' extension. Half the
    /// extension, half the length.
    #[test]
    fn a_long_step_follows_the_leg_extension() {
        let length = |extension: f32| {
            let (mut app, fighter, _) = duel(5.0, false);
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
            let drive = ragdoll.drive.as_mut().unwrap();
            drive.tone = 0.5;
            drive.balance_mut().set_extension(extension);
            app.world_mut().entity_mut(fighter).insert(ragdoll);
            engage(&mut app, fighter);
            step(&mut app);
            {
                let mut ai = app.world_mut().get_mut::<Duellist>(fighter).unwrap();
                ai.mind.long_step = true;
                ai.mind.step = 1;
                ai.last_tick = Some(26);
            }
            // Its body steps, 26 ticks into the step.
            {
                let mut body = app.world_mut().get_mut::<Ragdoll>(fighter).unwrap();
                let step = exa_physics::body::FootworkStep {
                    target: Vec3::new(0.0, 0.0, 9.0),
                    ..Default::default()
                };
                body.write_footwork(&step, true);
                body.age_step(26);
            }
            app.world_mut().insert_resource(BodyTicks(26));
            step(&mut app);
            // The second half re-aims the body's step and makes it long,
            // without a new start.
            let body = app.world().get::<Ragdoll>(fighter).unwrap();
            let fw = body.footwork_step().unwrap();
            assert!(fw.stepping && fw.long);
            assert!(fw.age_ms > 200.0, "{}", fw.age_ms);
            let goal = fw.target;
            Vec3::new(goal.x, 0.0, goal.z).length()
        };
        let full = length(1.0);
        let half = length(0.5);
        assert!(full > 0.5, "{full}");
        assert!((half - full * 0.5).abs() < 1e-3, "{half} vs {full}");
    }

    /// The combat state's step goes into the body's step fields as the
    /// original's AI writes them: a new step aims and starts (no balance
    /// step), a long step's second half re-aims and makes it long without a
    /// start, a step under way queues the place's offset (cm into m), a
    /// re-chosen step is plain again and a place of no step clears the
    /// queue.
    #[test]
    fn the_combat_step_goes_into_the_body_step() {
        use exa_physics::body::FootworkStep;
        let marks = combat::StepMarks::default();
        let mut fw = FootworkStep {
            balance_step: true,
            long: true,
            queue: 1,
            ..Default::default()
        };
        let goal = Vec3::new(100.0, 150.0, -50.0);
        let start = combat::Footstep::Start {
            dir: 1,
            goal,
            step_cm: None,
        };
        assert!(feet_outcome(&mut fw, start, marks));
        assert!(fw.stepping && !fw.balance_step && fw.long);
        assert_eq!((fw.target, fw.queue), (goal / CM, 1));
        let restart = combat::StepMarks {
            restart: true,
            ..marks
        };
        let cont = combat::Footstep::Continue {
            goal: Vec3::ZERO,
            step_cm: 80.0,
        };
        assert!(!feet_outcome(&mut fw, cont, restart));
        assert!(fw.long, "the second half is long whatever came before");
        assert_eq!(fw.target, Vec3::ZERO);
        assert!(!feet_outcome(&mut fw, combat::Footstep::Stay, restart));
        assert!(!fw.long);
        let queue = combat::Footstep::Queue {
            dir: 3,
            offset: Vec3::new(0.0, 0.0, 80.0),
        };
        fw.queue = 0;
        assert!(!feet_outcome(&mut fw, queue, marks));
        assert_eq!((fw.queue, fw.queued), (1, Vec3::new(0.0, 0.0, 0.8)));
        let clear = combat::StepMarks {
            clear_queue: true,
            ..marks
        };
        assert!(!feet_outcome(&mut fw, combat::Footstep::Stay, clear));
        assert_eq!(fw.queue, 0);
    }

    /// A body without muscles glides its interim step: no feet step is read.
    #[test]
    fn only_a_muscled_body_steps_on_its_feet() {
        assert!(feet_step_of(None).is_none());
        assert!(feet_step_of(Some(&Ragdoll::new(&[]))).is_none());
        assert!(feet_step_of(Some(&Ragdoll::new(&[]).with_drive())).is_some());
    }

    /// A second character standing to the right is scored from its own chest,
    /// so the first step's goal is to the left of it. Scored as the foe, five
    /// metres ahead, the goal would be forward.
    #[test]
    fn the_duellist_steps_aside_from_another_character() {
        let (mut app, foe, _) = duel(5.0, false);
        engage(&mut app, foe);
        step(&mut app);
        let beside = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::new(0.4, 0.0, 0.0)))
            .id();
        app.world_mut()
            .get_mut::<Combatant>(foe)
            .unwrap()
            .contacts
            .push(combatant::Contact::new(beside));
        let mut goal = None;
        for _ in 0..1500 {
            step(&mut app);
            if let Some(step) = app.world().get::<Duellist>(foe).unwrap().step {
                goal = Some(step.goal);
                break;
            }
        }
        let goal = goal.expect("a step");
        assert!(goal.x < -0.4, "{goal:?}");
    }

    /// The place on a second contact is close; the same place scored as the
    /// foe, five metres off, is not.
    #[test]
    fn another_contact_is_scored_from_its_own_chest() {
        use crate::combat::Field;

        let spot = place::Spot {
            other_reach_cm: 50.0,
            my_reach_cm: 50.0,
            other_size: 1.0,
            my_step_cm: 80.0,
            other_large: false,
            melee: false,
            level: 3,
            feral: 0.0,
            dull: false,
            blow: place::Blow::default(),
            wide: false,
            observer_ahead: 1.0,
            other_ahead: 1.0,
            in_attack: false,
            presses: false,
            other_skill: 0.0,
        };
        let mut field = DuelField {
            foe: Vec3::new(0.0, 0.0, 500.0),
            heading: Vec3::Z,
            foe_forward: Vec3::Z,
            spot,
            contact_at: vec![
                None,
                Some(ContactMeasure {
                    origin: Vec3::new(100.0, 0.0, 0.0),
                    forward: Vec3::X,
                    reach_cm: 50.0,
                    size: 1.0,
                    large: false,
                    skill: 0.0,
                    blow: place::Blow::default(),
                    dead: false,
                    posture: combat::Posture {
                        condition: 0,
                        reach_cm: 50.0,
                        running: false,
                        crouch: 0.0,
                        motion: 0,
                    },
                    lead: Vec3::ZERO,
                }),
            ],
            grid: None,
            chase: None,
            powers: None,
        };
        let on_them = field.score(1, 0, Vec3::new(100.0, 0.0, 0.0));
        let clear = field.score(1, 0, Vec3::new(-200.0, 0.0, 0.0));
        assert!(
            on_them.close > clear.close,
            "{} vs {}",
            on_them.close,
            clear.close
        );
        assert_eq!(field.score(0, 3, Vec3::new(100.0, 0.0, 0.0)).close, 0.0);
    }

    /// A dead character leaves the place's zeros. The same body, alive, is
    /// close.
    #[test]
    fn a_dead_character_adds_nothing_to_the_place() {
        use crate::combat::Field;

        let measure = ContactMeasure {
            origin: Vec3::new(100.0, 0.0, 0.0),
            forward: Vec3::X,
            reach_cm: 50.0,
            size: 1.0,
            large: false,
            skill: 0.0,
            blow: place::Blow::default(),
            dead: true,
            posture: combat::Posture {
                condition: 0,
                reach_cm: 50.0,
                running: false,
                crouch: 0.0,
                motion: 0,
            },
            lead: Vec3::ZERO,
        };
        let spot = place::Spot {
            other_reach_cm: 50.0,
            my_reach_cm: 50.0,
            other_size: 1.0,
            my_step_cm: 80.0,
            other_large: false,
            melee: false,
            level: 3,
            feral: 0.0,
            dull: false,
            blow: place::Blow::default(),
            wide: false,
            observer_ahead: 1.0,
            other_ahead: 1.0,
            in_attack: false,
            presses: false,
            other_skill: 0.0,
        };
        let mut field = DuelField {
            foe: Vec3::new(0.0, 0.0, 500.0),
            heading: Vec3::Z,
            foe_forward: Vec3::Z,
            spot,
            contact_at: vec![Some(measure)],
            grid: None,
            chase: None,
            powers: None,
        };
        let dead = field.score(0, 0, Vec3::new(100.0, 0.0, 0.0));
        assert_eq!(dead, combat::PlaceScore::default());
        field.contact_at[0].as_mut().unwrap().dead = false;
        assert!(field.score(0, 0, Vec3::new(100.0, 0.0, 0.0)).close > 0.0);
    }

    /// A dead body standing where a living one would push the step aside
    /// does not. The step goes forward, at the foe.
    #[test]
    fn a_dead_character_does_not_push_the_step() {
        let (mut app, foe, _) = duel(5.0, false);
        engage(&mut app, foe);
        step(&mut app);
        let beside = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.4, 0.0, 0.0)),
                Fighter::default(),
                Condition::DEAD,
            ))
            .id();
        app.world_mut()
            .get_mut::<Combatant>(foe)
            .unwrap()
            .contacts
            .push(combatant::Contact::new(beside));
        let mut goal = None;
        for _ in 0..1500 {
            step(&mut app);
            if let Some(step) = app.world().get::<Duellist>(foe).unwrap().step {
                goal = Some(step.goal);
                break;
            }
        }
        let goal = goal.expect("a step");
        assert!(goal.z > 0.4 && goal.x > -0.4, "{goal:?}");
    }

    /// A foe five metres ahead, stepping out to the right, is measured part
    /// way to that goal. The fight approaches that point, so the first step
    /// goes right instead of straight in.
    #[test]
    fn a_stepping_foe_is_measured_toward_its_step() {
        let (mut app, foe, _) = duel(5.0, false);
        let target = app.world().get::<Duellist>(foe).unwrap().target;
        let root = app
            .world_mut()
            .spawn((CharacterBody::default(), GlobalTransform::IDENTITY))
            .id();
        app.world_mut().entity_mut(target).insert((
            Duellist::new(foe),
            Transform::default(),
            Animator::new("Default", "Idle"),
            ChildOf(root),
        ));
        app.world_mut().get_mut::<Duellist>(target).unwrap().step = Some(Stepping {
            goal: Vec3::new(2.0, 0.0, -4.4),
            started_ms: 0,
            landed_from: 0,
            state: 0,
        });
        let mut roll = 1;
        let mut npc = brain::Npc::new([0.0; 4]);
        npc.personality = Some(personality::personality(
            100.0, [0.0; 4], 0.0, false, &mut roll,
        ));
        app.world_mut().entity_mut(target).insert(npc);
        engage(&mut app, foe);
        step(&mut app);
        let mut goal = None;
        for _ in 0..1500 {
            step(&mut app);
            if let Some(step) = app.world().get::<Duellist>(foe).unwrap().step {
                goal = Some(step.goal);
                break;
            }
        }
        let goal = goal.expect("a step");
        assert!(goal.x > 0.4 && goal.z < 0.4, "{goal:?}");
    }

    /// A stepping foe is estimated three quarters of the way to the step, by
    /// the fighter's own skill, not left at the chest.
    #[test]
    fn a_stepping_foe_is_estimated_toward_its_step() {
        let (mut app, fighter, _) = duel(5.0, false);
        let target = app.world().get::<Duellist>(fighter).unwrap().target;
        let root = app
            .world_mut()
            .spawn((CharacterBody::default(), GlobalTransform::IDENTITY))
            .id();
        app.world_mut().entity_mut(target).insert((
            Duellist::new(fighter),
            Transform::default(),
            Animator::new("Default", "Idle"),
            ChildOf(root),
        ));
        app.world_mut().get_mut::<Duellist>(target).unwrap().step = Some(Stepping {
            goal: Vec3::new(2.0, 0.0, 5.0),
            started_ms: 0,
            state: 0,
            landed_from: 0,
        });
        let mut roll = 1;
        app.world_mut()
            .get_mut::<Duellist>(fighter)
            .unwrap()
            .personality = Some(personality::personality(
            100.0, [0.0; 4], 0.0, false, &mut roll,
        ));
        engage(&mut app, fighter);
        step(&mut app);
        let entered = app.world().get::<Duellist>(fighter).unwrap().mind.estimate;
        assert!(
            (entered.x - 150.0).abs() < 1.0,
            "entering, expected 150 cm along the step, got {entered:?}"
        );
        step(&mut app);
        let estimate = app.world().get::<Duellist>(fighter).unwrap().mind.estimate;
        assert!(
            (estimate.x - 150.0).abs() < 1.0,
            "expected 150 cm along the step, got {estimate:?}"
        );
    }

    /// A stepping character taken this tick is aimed at with the same
    /// three-quarter lead, not the place-score point in front of their chest.
    #[test]
    fn a_character_taken_mid_tick_is_estimated_toward_its_step() {
        let (mut app, fighter, _) = duel(8.0, false);
        let mut roll = 1;
        app.world_mut()
            .get_mut::<Duellist>(fighter)
            .unwrap()
            .personality = Some(personality::personality(
            100.0, [0.0; 4], 0.0, false, &mut roll,
        ));
        engage(&mut app, fighter);
        step(&mut app);
        let beside = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                Fighter::default(),
            ))
            .id();
        let root = app
            .world_mut()
            .spawn((CharacterBody::default(), GlobalTransform::IDENTITY))
            .id();
        app.world_mut().entity_mut(beside).insert((
            Duellist::new(fighter),
            Transform::default(),
            Animator::new("Default", "Idle"),
            ChildOf(root),
        ));
        app.world_mut().get_mut::<Duellist>(beside).unwrap().step = Some(Stepping {
            goal: Vec3::new(2.0, 0.0, 1.0),
            started_ms: 0,
            state: 0,
            landed_from: 0,
        });
        app.world_mut()
            .get_mut::<Combatant>(fighter)
            .unwrap()
            .contacts
            .push(combatant::Contact::new(beside));
        let mut npc = brain::Npc::new([0.0; 4]);
        npc.contacts.push(contacts::ContactRecord {
            id: beside.to_bits(),
            level: 3,
            ..contacts::ContactRecord::default()
        });
        app.world_mut().entity_mut(fighter).insert(npc);
        step(&mut app);
        let ai = app.world().get::<Duellist>(fighter).unwrap();
        assert_eq!(ai.target, beside);
        assert!(
            (ai.mind.estimate.x - 150.0).abs() < 1.0,
            "{:?}",
            ai.mind.estimate
        );
        assert!(
            ai.mind.heading > 0.02,
            "heading {:.4} did not turn toward the step",
            ai.mind.heading
        );
    }

    /// A near character at the fight level presses harder than a far foe, so
    /// the duellist takes them as its target.
    #[test]
    fn the_duellist_takes_the_character_the_tick_prefers() {
        let (mut app, fighter, _) = duel(8.0, false);
        engage(&mut app, fighter);
        step(&mut app);
        let beside = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                Fighter::default(),
            ))
            .id();
        app.world_mut()
            .get_mut::<Combatant>(fighter)
            .unwrap()
            .contacts
            .push(combatant::Contact::new(beside));
        let mut npc = brain::Npc::new([0.0; 4]);
        npc.contacts.push(contacts::ContactRecord {
            id: beside.to_bits(),
            level: 3,
            ..contacts::ContactRecord::default()
        });
        app.world_mut().entity_mut(fighter).insert(npc);
        let mut took = false;
        for _ in 0..30 {
            step(&mut app);
            if app.world().get::<Duellist>(fighter).unwrap().target == beside {
                took = true;
                break;
            }
        }
        assert!(took);
    }

    /// A released blast on the fighter presses harder than a far foe, so the
    /// foe is dropped and the aim turns to the blast.
    #[test]
    fn a_blast_that_presses_harder_becomes_the_focus() {
        let (mut app, fighter, _) = duel(8.0, false);
        engage(&mut app, fighter);
        step(&mut app);
        let mut npc = brain::Npc::new([0.0; 4]);
        npc.blast_steps.push(combat::BlastStep {
            chest: Vec3::ZERO,
            aim: Vec3::X,
            released: true,
            at: Vec3::new(200.0, 100.0, 0.0),
        });
        app.world_mut().entity_mut(fighter).insert(npc);
        step(&mut app);
        // The next tick must not attend the old character again.
        step(&mut app);
        let ai = app.world().get::<Duellist>(fighter).unwrap();
        let focus = ai.focus.expect("a focus");
        assert!((focus.x - 200.0).abs() < 1.0, "{focus:?}");
        assert!(ai.mind.aim.abs() > 1.0, "{}", ai.mind.aim);
        assert!(
            app.world()
                .get::<Combatant>(fighter)
                .unwrap()
                .target
                .is_none()
        );
    }

    /// A held focus with nothing else pressing is cleared, and the next tick
    /// leaves combat.
    #[test]
    fn nothing_pressing_drops_the_foe_and_the_focus() {
        let (mut app, fighter, _) = duel(8.0, false);
        engage(&mut app, fighter);
        step(&mut app);
        let target = app.world().get::<Duellist>(fighter).unwrap().target;
        {
            let mut ai = app.world_mut().get_mut::<Duellist>(fighter).unwrap();
            ai.focus = Some(Vec3::new(100.0, 0.0, 0.0));
        }
        let mut npc = brain::Npc::new([0.0; 4]);
        npc.contacts.push(contacts::ContactRecord {
            id: target.to_bits(),
            level: 0,
            ..contacts::ContactRecord::default()
        });
        app.world_mut().entity_mut(fighter).insert(npc);
        step(&mut app);
        assert!(
            app.world()
                .get::<Duellist>(fighter)
                .unwrap()
                .focus
                .is_none()
        );
        assert!(app.world().get::<Duellist>(fighter).unwrap().fighting());
        step(&mut app);
        assert!(!app.world().get::<Duellist>(fighter).unwrap().fighting());
        assert!(
            app.world()
                .get::<Combatant>(fighter)
                .unwrap()
                .target
                .is_none()
        );
        // Three body ticks have passed (12 ms each). The wide pan waits
        // three seconds from that clock and the narrow pan half a second.
        let npc = app.world().get::<brain::Npc>(fighter).unwrap();
        assert_eq!(npc.alert.wide_at_ms, 3 * 12 + 3000);
        assert_eq!(npc.alert.narrow_at_ms, 3 * 12 + 500);
    }

    #[test]
    fn a_combat_step_is_the_measured_goal() {
        let (stepping, goal) = step_measure(None, Some(Vec3::new(1.0, 0.0, 2.0)), Vec3::ZERO);
        assert!(stepping);
        assert_eq!(goal, Vec3::new(100.0, 0.0, 200.0));
        let (standing, fallback) = step_measure(None, None, Vec3::new(3.0, 0.0, 0.0));
        assert!(!standing);
        assert_eq!(fallback, Vec3::new(3.0, 0.0, 0.0));
    }

    /// A lesson large enough to push the gain past 10 pays four points of
    /// experience per whole point, on the tick after combat is entered.
    #[test]
    fn a_practice_gain_is_paid_into_experience() {
        let (mut app, fighter, _) = duel(3.0, true);
        app.world_mut().entity_mut(fighter).insert(
            exa_character::record::CharacterData::of_record(
                exa_formats::characters::CharacterRecord::new(),
            ),
        );
        engage(&mut app, fighter);
        step(&mut app);
        app.world_mut()
            .get_mut::<Duellist>(fighter)
            .unwrap()
            .mind
            .practice
            .lesson = 200.0;
        step(&mut app);
        let experience = app
            .world()
            .get::<exa_character::record::CharacterData>(fighter)
            .unwrap()
            .experience;
        assert_eq!(experience, 80);
    }

    /// Striking a foe teaches its fighting estimate.
    #[test]
    fn the_foes_lesson_is_its_fighting_estimate() {
        let mut record = exa_formats::characters::CharacterRecord::new();
        record.combat_skill = 50.0;
        let lesson = foe_lesson(Some(&record), None, 1.0);
        assert_eq!(
            lesson.to_bits(),
            crate::estimate::fighting_estimate_with_base(
                crate::estimate::fighting_base(&record.skills, 50.0),
                None,
                1.0,
            )
            .to_bits()
        );
        assert_eq!(foe_lesson(None, None, 1.0), 0.0);
    }

    /// Outside a fight the body tick starts a requested draw when the body
    /// is free, drops it when the body is busy and standing, and keeps it
    /// while a busy body walks toward a goal. Without muscles it stays.
    #[test]
    fn the_body_tick_starts_a_requested_draw() {
        use exa_physics::body::locomotion::{Locomotor, Stop};

        let mut app = App::new();
        app.add_systems(Update, begin_requested);
        let spawn = |app: &mut App, condition: Condition, moving: bool, muscles: bool| {
            let id = app
                .world_mut()
                .spawn((Animator::new("Default", "Idle"), condition, Stance::Idle))
                .id();
            if muscles {
                app.world_mut().entity_mut(id).insert(driven());
            }
            if moving {
                app.world_mut().entity_mut(id).insert(Locomotor {
                    stop: Stop::Moving,
                    ..Locomotor::default()
                });
            }
            app.world_mut().get_mut::<Animator>(id).unwrap().requested = combat::DRAW_WEAPON;
            id
        };
        let free = spawn(&mut app, Condition::NORMAL, false, true);
        let standing = spawn(&mut app, Condition::ATTACKING, false, true);
        let walking = spawn(&mut app, Condition::ATTACKING, true, true);
        let limp = spawn(&mut app, Condition::NORMAL, false, false);
        app.update();
        let anim = |id| app.world().get::<Animator>(id).unwrap();
        assert_eq!(anim(free).action, combat::DRAW_WEAPON_ACTION);
        assert_eq!(anim(free).requested, 0);
        assert_eq!(anim(standing).action, "Idle");
        assert_eq!(anim(standing).requested, 0);
        assert_eq!(anim(walking).action, "Idle");
        assert_eq!(anim(walking).requested, combat::DRAW_WEAPON);
        assert_eq!(anim(limp).action, "Idle");
        assert_eq!(anim(limp).requested, combat::DRAW_WEAPON);
    }

    /// A forced start plays the clip with no capture to blend from. A busy
    /// body keeps both the request and the force.
    #[test]
    fn a_forced_start_drops_the_capture() {
        use exa_physics::body::locomotion::{Locomotor, Stop};

        let mut app = App::new();
        app.add_systems(Update, begin_requested);
        let free = app
            .world_mut()
            .spawn((
                Animator::new("Default", "Idle"),
                Condition::NORMAL,
                Stance::Idle,
                driven(),
            ))
            .id();
        {
            let mut anim = app.world_mut().get_mut::<Animator>(free).unwrap();
            anim.requested = combat::DRAW_WEAPON;
            anim.snap_start = true;
        }
        let walking = app
            .world_mut()
            .spawn((
                Animator::new("Default", "Idle"),
                Condition::ATTACKING,
                Stance::Idle,
                driven(),
                Locomotor {
                    stop: Stop::Moving,
                    ..Locomotor::default()
                },
            ))
            .id();
        {
            let mut anim = app.world_mut().get_mut::<Animator>(walking).unwrap();
            anim.requested = combat::DRAW_WEAPON;
            anim.snap_start = true;
        }
        app.update();
        let started = app.world().get::<Animator>(free).unwrap();
        assert_eq!(started.action, combat::DRAW_WEAPON_ACTION);
        assert_eq!(started.requested, 0);
        assert!(started.pending.is_none());
        assert!(!started.leading);
        assert!(!started.snap_start);
        let held = app.world().get::<Animator>(walking).unwrap();
        assert_eq!(held.action, "Idle");
        assert_eq!(held.requested, combat::DRAW_WEAPON);
        assert!(held.snap_start);
    }

    /// A requested clip ends a parry first and returns to the stance that
    /// end chose. A busy body keeps the parry and does not start. A body
    /// whose clip stands still ends the parry and keeps its stance.
    #[test]
    fn a_requested_motion_ends_the_parry_first() {
        use exa_combat::damage::Guard;

        let mut app = App::new();
        app.add_systems(Update, begin_requested);
        let parrying = |app: &mut App, condition: Condition| {
            let mut fighter = Fighter::default();
            fighter.health.yellow = 1.0;
            fighter.guard = Guard::Parry;
            fighter.parry.active = true;
            fighter.parry.clip = Some(0x14);
            fighter.parry.counter_motion = Some(5);
            let id = app
                .world_mut()
                .spawn((
                    Animator::new("Default", "Parry Left"),
                    condition,
                    Stance::Idle,
                    driven(),
                    fighter,
                ))
                .id();
            app.world_mut().get_mut::<Animator>(id).unwrap().requested = combat::DRAW_WEAPON;
            id
        };
        let free = parrying(&mut app, Condition::NORMAL);
        let busy = parrying(&mut app, Condition::ATTACKING);
        let still = parrying(&mut app, Condition(Condition::STILL_CLIP_MASK));
        app.update();

        let ended = app.world().get::<Fighter>(free).unwrap();
        assert!(!ended.parry.active);
        assert!(ended.parry.clip.is_none());
        assert!(ended.parry.counter_motion.is_none());
        assert_eq!(ended.guard, Guard::Open);
        assert_eq!(
            *app.world().get::<Stance>(free).unwrap(),
            Stance::CombatIdle
        );
        let anim = app.world().get::<Animator>(free).unwrap();
        assert_eq!(anim.action, combat::DRAW_WEAPON_ACTION);
        assert_eq!(anim.then.as_deref(), Some(Stance::CombatIdle.action()));

        let kept = app.world().get::<Fighter>(busy).unwrap();
        assert!(kept.parry.active);
        assert_eq!(kept.parry.counter_motion, Some(5));
        assert_eq!(kept.guard, Guard::Parry);
        assert_eq!(
            app.world().get::<Animator>(busy).unwrap().action,
            "Parry Left"
        );

        let frozen = app.world().get::<Fighter>(still).unwrap();
        assert!(!frozen.parry.active);
        assert_eq!(*app.world().get::<Stance>(still).unwrap(), Stance::Idle);
        assert_eq!(
            app.world().get::<Animator>(still).unwrap().then.as_deref(),
            Some(Stance::Idle.action())
        );
    }

    /// Ending the parry clears the arm bit on the hand and its parent, and
    /// disables the held grip past the pivot. The pivot link stays.
    #[test]
    fn a_parry_end_unlocks_the_hand_and_the_grip() {
        use exa_combat::damage::Guard;
        use exa_physics::grip::{
            DISABLED, ENABLED, GripLink, GripLinks, GripSpec, HeldGrip, HeldKind, PIVOT,
        };
        use exa_physics::part::PartState;

        let mut app = App::new();
        app.add_systems(Update, begin_requested);
        let key = app.world_mut().spawn_empty().id();
        let hand = app.world_mut().spawn_empty().id();
        let part = |bone, entity, parent| exa_physics::body::PartSetup {
            bone,
            entity,
            parent,
            world: bevy::math::Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.08),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        };
        let mut body = Ragdoll::new(&[
            part(1, key, None),
            part(exa_character::RIGHT_HAND, hand, Some(0)),
        ])
        .with_drive();
        body.parts[0].state = body.parts[0].state.with(PartState::ARM);
        body.parts[1].state = body.parts[1].state.with(PartState::ARM);
        let link = |flags| GripLink {
            item: 0,
            on_item: [1.0, 0.0, 0.0, 0.0],
            hand: Some(0),
            on_hand: [1.0, 0.0, 0.0, 0.0],
            slack: 0.0,
            length: 0.0,
            strength: 1.0,
            smoothing: 0.001,
            flags,
            shares: (1.0, 0.0),
            anchor: None,
        };
        let grip = HeldGrip {
            spec: GripSpec {
                hand: 0,
                links: vec![],
                second_hand: None,
                arm: None,
                extra: None,
            },
            scale: 1.0,
            size: 1.0,
            second_hand_factor: 1.0,
            flip: false,
            class: 0x10,
            soft_weld: false,
            kind: HeldKind::Weapon,
            holder_key: key,
            hands: [Some(hand), None],
            arms: [None, None],
            links: Some(GripLinks {
                links: vec![link(PIVOT), link(PIVOT | ENABLED)],
                main: 1,
                second: None,
                arm: None,
                extra: None,
                flip: None,
                flip_point: Vec3::ZERO,
                flip_progress: 0.0,
                flipped: false,
                flip_active: false,
                mode: -1,
                motion: -1,
                relaxing: false,
                record_dirty: false,
                world_soft: None,
                world_hard: None,
            }),
            released: false,
            head_extra: 0.0,
        };
        let mut fighter = Fighter::default();
        fighter.health.yellow = 1.0;
        fighter.guard = Guard::Parry;
        fighter.parry.active = true;
        let holder = app
            .world_mut()
            .spawn((
                Animator::new("Default", "Parry Left"),
                Condition::NORMAL,
                Stance::Idle,
                body,
                fighter,
            ))
            .id();
        app.world_mut()
            .get_mut::<Animator>(holder)
            .unwrap()
            .requested = combat::DRAW_WEAPON;
        let item = app.world_mut().spawn(grip).id();
        app.update();
        let body = app.world().get::<Ragdoll>(holder).unwrap();
        assert!(!body.parts[0].state.has(PartState::ARM));
        assert!(!body.parts[1].state.has(PartState::ARM));
        let links = app
            .world()
            .get::<HeldGrip>(item)
            .unwrap()
            .links
            .as_ref()
            .unwrap();
        assert_eq!(links.links[0].flags, PIVOT);
        assert_eq!(links.links[1].flags, DISABLED);
        assert!(!app.world().get::<Fighter>(holder).unwrap().parry.active);
    }

    /// A body with muscles, so a requested motion can start.
    fn driven() -> Ragdoll {
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
        Ragdoll::new(&[part]).with_drive()
    }

    /// Empty hands with a stowed weapon ask for the draw, and the next tick's
    /// body start plays it. The first tick only enters combat.
    #[test]
    fn bare_hands_draw_the_stowed_weapon() {
        let (mut app, fighter, _) = duel(3.0, false);
        let mut record = exa_formats::characters::CharacterRecord::new();
        record.hands[2] = exa_formats::characters::ItemRef::new(7, 0);
        app.world_mut().entity_mut(fighter).insert((
            exa_character::record::CharacterData::of_record(record),
            driven(),
        ));
        engage(&mut app, fighter);
        step(&mut app);
        step(&mut app);
        let asked = app.world().get::<Animator>(fighter).unwrap();
        assert_eq!(asked.requested, combat::DRAW_WEAPON);
        assert_ne!(asked.action, combat::DRAW_WEAPON_ACTION);
        step(&mut app);
        let playing = app.world().get::<Animator>(fighter).unwrap();
        assert_eq!(playing.action, combat::DRAW_WEAPON_ACTION);
        assert_eq!(playing.requested, 0);
    }

    /// A draw the body has not started holds the fight at a watch, which
    /// drops a lunge. Standing while busy drops the request instead, so the
    /// lunge stays and the draw is asked again.
    #[test]
    fn a_pending_draw_watches_while_the_body_walks() {
        use exa_physics::body::locomotion::{Locomotor, Stop};

        let pending = |moving: bool| {
            let (mut app, fighter, _) = duel(3.0, false);
            let mut record = exa_formats::characters::CharacterRecord::new();
            record.hands[2] = exa_formats::characters::ItemRef::new(7, 0);
            app.world_mut().entity_mut(fighter).insert((
                exa_character::record::CharacterData::of_record(record),
                driven(),
                Condition::ATTACKING,
            ));
            if moving {
                app.world_mut().entity_mut(fighter).insert(Locomotor {
                    stop: Stop::Moving,
                    ..Locomotor::default()
                });
            }
            engage(&mut app, fighter);
            step(&mut app);
            step(&mut app);
            {
                let mut ai = app.world_mut().get_mut::<Duellist>(fighter).unwrap();
                ai.mind.lunging = true;
                ai.mind.level = 3;
            }
            step(&mut app);
            let lunging = app.world().get::<Duellist>(fighter).unwrap().mind.lunging;
            let anim = app.world().get::<Animator>(fighter).unwrap();
            (lunging, anim.action.clone(), anim.requested)
        };
        let (lunging, action, requested) = pending(true);
        assert!(!lunging);
        assert_ne!(action, combat::DRAW_WEAPON_ACTION);
        assert_eq!(requested, combat::DRAW_WEAPON);
        let (lunging, action, requested) = pending(false);
        assert!(lunging);
        assert_ne!(action, combat::DRAW_WEAPON_ACTION);
        assert_eq!(requested, combat::DRAW_WEAPON);
    }

    /// Entering combat leaves the exploration slow turn. The next tick, once
    /// the fight is underway, drops it. A fall never reaches that tick.
    #[test]
    fn a_fight_drops_the_exploration_slow_turn() {
        use exa_physics::body::locomotion::Locomotion;

        let (mut app, fighter, _) = duel(3.0, false);
        app.world_mut().entity_mut(fighter).insert(Locomotion {
            slow_turn: true,
            ..Locomotion::default()
        });
        engage(&mut app, fighter);
        step(&mut app);
        assert!(app.world().get::<Locomotion>(fighter).unwrap().slow_turn);
        step(&mut app);
        assert!(!app.world().get::<Locomotion>(fighter).unwrap().slow_turn);

        let (mut down, fighter, _) = duel(3.0, false);
        down.world_mut().entity_mut(fighter).insert((
            Locomotion {
                slow_turn: true,
                ..Locomotion::default()
            },
            Condition::DOWN,
        ));
        engage(&mut down, fighter);
        step(&mut down);
        step(&mut down);
        assert!(down.world().get::<Locomotion>(fighter).unwrap().slow_turn);
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

    /// A body at the balance line clears the eagerness, so no blow comes.
    /// One fully balanced, with the same body, still strikes.
    #[test]
    fn off_balance_the_eagerness_stays_down() {
        let stood = |balance: f32| {
            let (mut app, fighter, _) = duel(3.0, false);
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
            ragdoll.drive.as_mut().unwrap().balance_mut().value = balance;
            app.world_mut().entity_mut(fighter).insert(ragdoll);
            engage(&mut app, fighter);
            // Entering sets the eagerness at a half. The next tick keeps it
            // only while the body is past the balance line.
            step(&mut app);
            step(&mut app);
            (app, fighter)
        };
        let (low, fighter) = stood(0.85);
        assert_eq!(
            low.world().get::<Duellist>(fighter).unwrap().mind.eagerness,
            0.0
        );
        let (mut steady, fighter) = stood(1.0);
        assert!(strikes(&mut steady, fighter));
    }

    fn person_at(app: &mut App, at: Vec3) -> Entity {
        app.world_mut()
            .spawn((GlobalTransform::from_translation(at), human()))
            .id()
    }

    fn hold(app: &mut App, fighter: Entity, who: Entity, concern: f32, hostility: f32) {
        let mut contact = combatant::Contact::new(who);
        contact.concern = concern;
        contact.hostility = hostility;
        app.world_mut()
            .get_mut::<Combatant>(fighter)
            .unwrap()
            .contacts
            .push(contact);
    }

    /// True once the fighter starts a blow, within the same wait the rested
    /// strike uses.
    fn strikes(app: &mut App, fighter: Entity) -> bool {
        for _ in 0..1500 {
            step(app);
            if app.world().get::<Attacking>(fighter).is_some() {
                return true;
            }
        }
        false
    }

    /// A calm contact two metres ahead, inside the strike, stops the blow.
    #[test]
    fn a_friend_ahead_stops_the_strike() {
        let (mut app, fighter, _) = duel(3.0, false);
        let friend = person_at(&mut app, Vec3::new(0.0, 0.0, 2.0));
        hold(&mut app, fighter, friend, 0.0, 0.0);
        engage(&mut app, fighter);
        assert!(!strikes(&mut app, fighter));
    }

    /// At half feral the ally ahead does not stop the blow. Just under a
    /// half, it still does.
    #[test]
    fn a_feral_fighter_strikes_through_the_friend() {
        let run = |feral: f32| {
            let (mut app, fighter, _) = duel(3.0, false);
            app.world_mut()
                .get_mut::<Fighter>(fighter)
                .unwrap()
                .stats
                .feral = feral;
            let ally = person_at(&mut app, Vec3::new(0.0, 0.0, 1.5));
            let mut npc = brain::Npc::new([0.0; 4]);
            npc.nearby.push(ally.to_bits());
            app.world_mut().entity_mut(fighter).insert(npc);
            engage(&mut app, fighter);
            strikes(&mut app, fighter)
        };
        assert!(run(0.5));
        assert!(!run(0.499));
    }

    /// The same place does not stop the blow when the contact's concern and
    /// hostility pass 1.8, nor when the contact is dead. Two metres is still
    /// inside the strike and past the crowding that would stop a blow on its
    /// own.
    #[test]
    fn a_hostile_or_dead_contact_does_not_stop_the_strike() {
        let (mut app, fighter, _) = duel(3.0, false);
        let friend = person_at(&mut app, Vec3::new(0.0, 0.0, 2.0));
        hold(&mut app, fighter, friend, 1.0, 0.9);
        engage(&mut app, fighter);
        assert!(strikes(&mut app, fighter));

        let (mut app, fighter, _) = duel(3.0, false);
        let friend = person_at(&mut app, Vec3::new(0.0, 0.0, 2.0));
        app.world_mut().entity_mut(friend).insert(Condition::DEAD);
        hold(&mut app, fighter, friend, 0.0, 0.0);
        engage(&mut app, fighter);
        assert!(strikes(&mut app, fighter));
    }

    /// The foe stands in reach of the strike and is not a friend, so the blow
    /// still goes. Their concern and hostility stay at or under 1.8, which
    /// would have made anyone else a friend.
    #[test]
    fn the_foe_within_reach_does_not_count_as_a_friend() {
        let (mut app, fighter, _) = duel(0.5, false);
        engage(&mut app, fighter);
        let mut sum = 0.0;
        let mut struck = false;
        for _ in 0..1500 {
            step(&mut app);
            if app.world().get::<Attacking>(fighter).is_some() {
                let target = app.world().get::<Duellist>(fighter).unwrap().target;
                let contact = app
                    .world()
                    .get::<Combatant>(fighter)
                    .unwrap()
                    .contacts
                    .iter()
                    .find(|c| c.character == target)
                    .expect("the foe's contact");
                sum = contact.concern + contact.hostility;
                struck = true;
                break;
            }
        }
        assert!(struck);
        assert!(
            sum <= 1.8,
            "foe concern and hostility {sum} would already skip them"
        );
    }

    /// An ally the room list gathered, with no contact of their own, is still
    /// a friend and stops the blow.
    #[test]
    fn a_nearby_ally_with_no_contact_stops_the_strike() {
        let (mut app, fighter, _) = duel(3.0, false);
        let ally = person_at(&mut app, Vec3::new(0.0, 0.0, 1.5));
        let mut npc = brain::Npc::new([0.0; 4]);
        npc.nearby.push(ally.to_bits());
        app.world_mut().entity_mut(fighter).insert(npc);
        engage(&mut app, fighter);
        assert!(!strikes(&mut app, fighter));
    }

    /// A friend standing beyond the strike, stepping so the halfway point is
    /// in it, stops the blow. The same step with concern and hostility past
    /// 1.8 does not.
    #[test]
    fn a_friend_stepping_into_the_strike_stops_it() {
        let arm = |concern: f32, hostility: f32, expect: bool| {
            let (mut app, fighter, _) = duel(3.0, false);
            let friend = person_at(&mut app, Vec3::new(0.0, 0.0, 4.0));
            let hanger = app.world_mut().spawn_empty().id();
            app.world_mut().entity_mut(friend).insert((
                Duellist::new(Entity::PLACEHOLDER),
                Transform::default(),
                Animator::new("Default", "Idle"),
                ChildOf(hanger),
            ));
            app.world_mut().get_mut::<Duellist>(friend).unwrap().step = Some(Stepping {
                goal: Vec3::new(0.0, 1.3, -1.0),
                started_ms: 0,
                state: 0,
                landed_from: 0,
            });
            hold(&mut app, fighter, friend, concern, hostility);
            engage(&mut app, fighter);
            assert_eq!(strikes(&mut app, fighter), expect);
        };
        arm(0.0, 0.0, false);
        arm(1.0, 0.9, true);
    }

    /// A body whose item flags refuse powers never chooses one, however
    /// usable the slot is.
    #[test]
    fn a_refusing_body_does_not_choose_a_power() {
        let (mut app, foe, _) = duel(3.0, true);
        let mut slots = [0; cast::SLOTS];
        slots[0] = 0x401;
        let mut answers = [0; cast::SLOTS];
        answers[0] = cast::USABLE | 5;
        app.world_mut().entity_mut(foe).insert(DuelPowers {
            slots,
            answers,
            pool: 1.0,
            refuses: true,
            ..DuelPowers::default()
        });
        engage(&mut app, foe);
        for _ in 0..400 {
            step(&mut app);
            assert!(app.world().get::<DuelPowers>(foe).unwrap().chosen.is_none());
        }
        assert!(!app.world().get::<Duellist>(foe).unwrap().mind.casting);
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
