//! The NPC combat state: one tick of a fighter that has a foe.
//!
//! A fighter enters it with its eagerness at a half, no press, the combat
//! stance and the in-combat mark, stamped with the clock. Each tick then
//! runs these parts in order, any of which can end the tick early:
//!
//! - **Practice.** A lesson above 0.005 feeds a gain by
//!   `(1 − fatigue)·lesson·0.1` and the fatigue by `lesson·0.0001` (at most
//!   0.9); a gain above 10 pays its rounded value four times into the
//!   record's practice count and keeps the rest; the lesson decays by
//!   0.995 a tick, the fatigue always by 0.9998.
//! - **Leaving.** A fighter that is down resets its turn and leaves the
//!   tick; a foe clears the focus, so a dead foe is dropped and the
//!   fighter leaves combat for its activity (a look in 3 s, the next try in
//!   0.5 s), as it does with no foe and no focus.
//! - **The estimate.** With probability `(0.8·k² + 0.2)²` (k the skill) the
//!   foe's position is estimated again and blended back toward the old one
//!   by `0.6 − 0.6·k²`; the direction to it and its distance (at least 1,
//!   800 when more than 300 cm above or below) follow.
//! - **The level.** Fists or a one-handed class with nothing held draw a
//!   carried weapon first, which holds the level at a watch. The contact
//!   that is the foe gives the level. At level 2 or more one tick in 300
//!   (outside a swing) starts a lunge: the measured reach is 80 cm longer.
//! - **Chasing and eagerness.** Engaged and balanced (above 0.85): a foe
//!   beyond 600–800 cm (400 when it lies down, 200–300 when it runs; ¾ when
//!   the fighter runs), more than 150 cm above or below, or out of sight
//!   counts the fighter as stuck; after 21 such ticks it chases. With full
//!   energy the eagerness drifts by `(3 − k)·(0.02·draw − 0.006)` up to 6
//!   and falls by 0.01 above; short of energy it is scaled by
//!   `1 − (1 − E/Emax)²`, and under eight tenths the press stops. Not
//!   engaged, the eagerness is 0.
//! - **Drives.** Three weights steer the footwork: the attack drive, the
//!   strike drive and the spacing drive. Early in its own swing (before
//!   0.7 of the clock, 0.9 with fists) the fighter loses eagerness past
//!   0.6 of the clock, records its foe's lesson, measures its overshoot past
//!   0.5 and commits past 0.3; otherwise the drives come from the eagerness,
//!   halved while it recovers. A large foe doubles the attack drive (four
//!   times against claws). Below the engage level there is no attack.
//!   They scale with the blend and the square of the energy; a shield
//!   doubles the strike drive, a foe in its strike recovery both. Full
//!   energy and a press multiply them up (stamping the clock); full energy
//!   without a press halves them and doubles the spacing; short of energy
//!   halves the attack and quadruples the spacing.
//! - **Footwork.** It weighs up to seventeen places: where it stands (or
//!   where its step lands), eight steps (ahead, back, both sides, the four
//!   diagonals) and, in a fresh step with enough energy, eight longer ones
//!   (`1.6 − 0.3·encumbrance` steps). Each place costs its ground, the
//!   change from the step it is in, every contact's danger and closeness,
//!   and against the foe the strike drive and the approach; the cheapest
//!   place to stand and the cheapest place to strike from compete.
//! - **The attack.** A strike place wins when it is cheaper than standing
//!   by at least 2 (1 committed), no friend stands in the way and, outside
//!   a swing, a skilled fighter does not step in further than its skill
//!   allows. The blow is the attack choice ([`crate::fight`]); under eight
//!   tenths of its energy, or with the executor refusing, it guards.
//! - **Stepping and turning.** The chosen place starts or queues a step, or
//!   continues a long step after 200 ms. The heading follows the place's
//!   bearing by 0.03 of the gap a tick; during a swing it turns by the
//!   swing's sweep, on guard it leans against the foe's side swing, and a
//!   retreating fighter re-aims when the bearing moves by more than 54°.
//!
//! Evidence: the original's combat-state routine and its enter, practice,
//! down, leave, estimate, set-target, bystander and keep-off helpers, read
//! from the instructions (sums and products in their order, comparisons
//! done in double precision where the original widens). The position score
//! of each contact, the ground cost, the line walk, the sight test, the
//! chase route, the follow check and the cast choice belong to other parts
//! and come in through [`Field`]; the duellist answers the ground cost, the
//! line walk and the sight test from the level's navigation grid
//! ([`crate::ground`]), and the snap stays the abstract pathfinder's,
//! which leaves the point where it was. A heading wrap compares in double with
//! the f32 full turn, as the original does. The original keeps body points
//! relative to each body's scene node and adds the node's origin to the
//! points it weighs and to the foe's estimate (less its own); Exarust keeps
//! one world frame, so those origins are zero here.

use bevy::math::Vec3;
use exa_combat::attack::AttackMotion;
use exa_combat::stance::{Stance, stance_by_energy};
use exa_physics::engine_math::reciprocal_sqrt;

use crate::cast;
use crate::fight;
use crate::personality::unit_draw;

/// The f32 full turn.
pub const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
/// Degrees to radians.
pub const DEGREE: f32 = f32::from_bits(0x3c8e_fa35);
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
#[cfg(test)]
const PI_F32: f32 = f32::from_bits(0x4049_0fdb);
/// The "no score yet" the searches start from.
const NO_SCORE: f32 = f32::from_bits(0x7f7f_c99e);
/// The clock below which a swing has not started turning.
const CLOCK_EPS: f32 = f32::from_bits(0x33ff_ffd7);
/// A retreating fighter re-aims past this gap (54°, a double).
const REAIM: f64 = 0.942_477_796_076_937_9;
/// The balance above which an engaged fighter chases.
const BALANCED: f32 = f32::from_bits(0x3f59_999a);

/// Body condition: attacking.
pub const ATTACKING: u32 = 2;
/// Body condition: casting.
pub const CASTING: u32 = 0x20;
/// Body condition bit: down.
pub const DOWN: u32 = 0x200;
/// Body condition bits a lying body shows (down, lying).
pub const LYING: u32 = 0x300;
/// The draw-weapon motion.
pub const DRAW_WEAPON: i32 = 0x1f;
/// Weapon class: the fists.
pub const FISTS: u32 = 0x80;
/// Weapon class: claws.
pub const CLAWS: u32 = 0xc0;
/// The stance motion of the guard.
pub const GUARD: i32 = 3;
/// The stance motion of the combat idle.
pub const COMBAT_IDLE: i32 = 4;

/// The engine's generator: the next value times `n`, high half.
fn draw(roll: &mut u32, n: u64) -> u32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((u64::from(*roll) * n) >> 32) as u32
}

/// Brings an angle back into ±π, comparing in double precision against π
/// and stepping by the f32 full turn.
pub fn wrap(a: f32) -> f32 {
    let wide = f64::from(a);
    if wide < -std::f64::consts::PI {
        a + TWO_PI
    } else if wide > std::f64::consts::PI {
        a - TWO_PI
    } else {
        a
    }
}

/// The signed gap from `b` to `a`, wrapped.
pub fn gap(a: f32, b: f32) -> f32 {
    wrap(a - b)
}

pub use exa_anim::quick_acos;

/// The yaw of a direction (x, z), 0 along +z, positive toward +x
/// ([`exa_anim::quick_yaw`]).
pub fn yaw(v: Vec3) -> f32 {
    exa_anim::quick_yaw(v.x, v.z)
}

/// The unit direction of a yaw.
pub fn of_yaw(a: f32) -> Vec3 {
    Vec3::new(a.sin(), 0.0, a.cos())
}

/// The engine's unit: zero stays zero.
pub fn unit(v: Vec3) -> Vec3 {
    let sum = (v.y * v.y + 0.0) + (v.x * v.x + v.z * v.z);
    let s = if sum == 0.0 {
        0.0
    } else {
        reciprocal_sqrt(sum)
    };
    Vec3::new(v.x * s, v.y * s, v.z * s)
}

/// The engine's full length.
fn length(v: Vec3) -> f32 {
    ((v.x * v.x + v.z * v.z) + (v.y * v.y + 0.0)).sqrt()
}

/// The engine's flat length.
fn flat_length(v: Vec3) -> f32 {
    (v.x * v.x + v.z * v.z).sqrt()
}

fn distance2(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    d.x * d.x + d.y * d.y + d.z * d.z
}

/// Practice from fighting: what a struck foe teaches.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Practice {
    pub lesson: f32,
    pub gain: f32,
    pub fatigue: f32,
}

/// What the record's practice count gains per point.
pub const PRACTICE_PER_POINT: i32 = 4;

/// One tick of practice; what it adds to the record's practice count.
pub fn practise(p: &mut Practice) -> i32 {
    let mut paid = 0;
    if f64::from(p.lesson) > 0.005 {
        p.gain =
            ((1.0 - p.fatigue) as f64 * (f64::from(p.lesson) * 0.1) + f64::from(p.gain)) as f32;
        p.fatigue = (f64::from(p.lesson) * 0.0001 + f64::from(p.fatigue)) as f32;
        if f64::from(p.fatigue) > 0.9 {
            p.fatigue = f32::from_bits(0x3f66_6666);
        }
        if p.gain > 10.0 {
            let whole = p.gain.round_ties_even() as i64;
            paid = (whole * i64::from(PRACTICE_PER_POINT)) as i32;
            p.gain -= whole as f32;
        }
        p.lesson = (f64::from(p.lesson) * 0.995) as f32;
    }
    p.fatigue = (f64::from(p.fatigue) * 0.9998) as f32;
    paid
}

/// The combat state's own fields.
#[derive(Debug, Clone, PartialEq)]
pub struct CombatMind {
    /// The foe's contact level (0 notice … 3 engage).
    pub level: i32,
    pub eagerness: f32,
    /// How committed the fighter is to its swing (1 on a new foe; halved or
    /// cleared late in a swing) [guess: name].
    pub commitment: f32,
    /// It presses its foe.
    pub pressing: bool,
    /// This swing's overshoot has been measured.
    pub measured: bool,
    /// A cast is running.
    pub casting: bool,
    pub lunging: bool,
    /// The reach the overshoot is measured against (cm).
    pub lunge_reach_cm: f32,
    pub overshoot: f32,
    pub overshoot_mean: f32,
    pub practice: Practice,
    /// When it entered combat, or last pressed (ms).
    pub entered_ms: u32,
    pub stuck: i32,
    /// The cost of leaving the step it is in.
    pub step_cost: f32,
    pub last_step_ms: u32,
    /// The step it is in (0 none, 1–8 the directions).
    pub step: i32,
    /// The step queued after this one.
    pub queued_step: i32,
    /// The step is a long one, to be continued.
    pub long_step: bool,
    /// The heading it turns its look to.
    pub heading: f32,
    /// The bearing it aims at.
    pub aim: f32,
    /// The swing's body turn amount.
    pub swing_turn: f32,
    /// The blow asked for: 8 the overhead, 16 the thrust, 0 a side.
    pub request: u32,
    /// 2 the other side, else the main side.
    pub side: i32,
    /// Raised to at least a half while engaged [?].
    pub alert: f32,
    /// Where it believes the foe is.
    pub estimate: Vec3,
    /// The foe changed this tick.
    pub fresh_target: bool,
    /// The cast in progress runs from this clock (ms).
    pub cast_start_ms: u32,
    /// The per-slot cast cooldowns.
    pub casts: cast::Casting,
}

impl Default for CombatMind {
    fn default() -> Self {
        Self {
            level: 0,
            eagerness: 0.0,
            commitment: 0.0,
            pressing: false,
            measured: false,
            casting: false,
            lunging: false,
            lunge_reach_cm: 0.0,
            overshoot: 0.0,
            overshoot_mean: 0.0,
            practice: Practice::default(),
            entered_ms: 0,
            stuck: 0,
            step_cost: 0.0,
            last_step_ms: 0,
            step: 0,
            queued_step: 0,
            long_step: false,
            heading: 0.0,
            aim: 0.0,
            swing_turn: 0.0,
            request: 0,
            side: 0,
            alert: 0.0,
            estimate: Vec3::ZERO,
            fresh_target: false,
            cast_start_ms: 0,
            casts: cast::Casting::default(),
        }
    }
}

/// Enters the combat state: the eagerness at a half, no press, the clock
/// stamped. The caller takes the combat stance (motion 3) and marks the
/// body in combat. Already in combat, nothing changes.
pub fn enter(mind: &mut CombatMind, in_combat: bool, now_ms: u32) -> bool {
    if in_combat {
        return false;
    }
    mind.eagerness = 0.5;
    mind.pressing = false;
    mind.entered_ms = now_ms;
    true
}

/// The foe as the fighter's body sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Foe {
    pub condition: u32,
    /// The second condition word's death bit (`0x800`) or the condition's.
    pub dead: bool,
    pub crouch: f32,
    /// It has support under it.
    pub supported: bool,
    /// Its run mode [guess: name].
    pub running: bool,
    pub stepping: bool,
    pub reach_cm: f32,
    pub motion: i32,
    pub weapon_class: u32,
    pub rules: u32,
    pub attack_flags: u32,
    pub chest: Vec3,
    pub step_goal: Vec3,
    pub facing: Vec3,
    pub forward_speed: f32,
    pub scale: f32,
    /// What striking it teaches.
    pub lesson: f32,
}

impl Default for Foe {
    fn default() -> Self {
        Self {
            condition: 0,
            dead: false,
            crouch: 0.0,
            supported: true,
            running: false,
            stepping: false,
            reach_cm: 50.0,
            motion: 0,
            weapon_class: FISTS,
            rules: 0,
            attack_flags: 0,
            chest: Vec3::ZERO,
            step_goal: Vec3::ZERO,
            facing: Vec3::Z,
            forward_speed: 0.0,
            scale: 1.0,
            lesson: 0.0,
        }
    }
}

/// The fighter's own body, as the combat state reads it (cm, ms).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    pub now_ms: u32,
    pub condition: u32,
    /// The current motion's clock, 0 to 1.
    pub clock: f32,
    pub attack_flags: u32,
    pub weapon_class: u32,
    /// Held weapon bodies.
    pub held: i32,
    /// Weapons carried but not held [guess: the record's count].
    pub carried: u32,
    pub motion: i32,
    pub requested_motion: i32,
    pub energy: f32,
    pub max_energy: f32,
    pub balance: f32,
    pub scale: f32,
    pub size: f32,
    pub encumbrance: f32,
    pub reach_cm: f32,
    pub step_cm: f32,
    /// The stance factor a long step is scaled by.
    pub step_factor: f32,
    pub stepping: bool,
    pub step_state: i32,
    pub step_started_ms: u32,
    pub landings: i32,
    /// The footwork queue's state.
    pub queue: i32,
    /// It is free to step.
    pub can_step: bool,
    pub running: bool,
    pub forward_speed: f32,
    pub shield: bool,
    pub heavy_claw: bool,
    pub rules: u32,
    pub quadruped: bool,
    /// The race's AI attack bonus.
    pub attack_bonus: f32,
    pub age: f32,
    pub parry_bias: i32,
    pub thrust_pref: i32,
    pub recoil_until_ms: u32,
    pub parrying: bool,
    pub yaw: f32,
    pub heading: Vec3,
    pub facing: Vec3,
    pub position: Vec3,
    pub step_goal: Vec3,
    pub chest: Vec3,
    pub ground_y: f32,
    pub look: Vec3,
    pub target_point: Vec3,
    /// The direction to the foe the last estimate left.
    pub to_target: Vec3,
    /// The distance to the foe the last estimate left (cm).
    pub distance_cm: f32,
    /// The primary weapon's swing turn amount, when it holds one.
    pub swing_amount: Option<f32>,
    /// The primary weapon's stance blend, when it holds one.
    pub stance_blend: Option<i32>,
    /// The yellow health the guard choice reads.
    pub health: f32,
    /// The attack executor would start a blow now (energy aside).
    pub can_attack: bool,
    /// The pool powers are paid from, 0 through 1 [guess: the mana-like
    /// pool the cast chooser scales by 200].
    pub power_pool: f32,
    /// The record's ten power slots (0 empty).
    pub power_slots: [u32; cast::SLOTS],
    /// The body's power word has its busy bit (0x10), so no power is
    /// chosen [guess: what the bit means].
    pub power_busy: bool,
    /// How firmly the body holds on, at least 0.7 to act [guess: name].
    pub grip: f32,
    /// Awareness above 0.7 to act [guess: name].
    pub awareness: f32,
    /// The body's trailing-foot request (2 when a clip replaced Walk or
    /// Run, 1 once planted, until its 500 ms deadline): 0 when free to
    /// act.
    pub blocking: i32,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            now_ms: 0,
            condition: 0,
            clock: 0.0,
            attack_flags: 0,
            weapon_class: FISTS,
            held: 0,
            carried: 0,
            motion: COMBAT_IDLE,
            requested_motion: 0,
            energy: 1.0,
            max_energy: 1.0,
            balance: 1.0,
            scale: 1.0,
            size: 1.0,
            encumbrance: 0.0,
            reach_cm: 50.0,
            step_cm: 80.0,
            step_factor: 1.0,
            stepping: false,
            step_state: 0,
            step_started_ms: 0,
            landings: 0,
            queue: 0,
            can_step: true,
            running: false,
            forward_speed: 0.0,
            shield: false,
            heavy_claw: false,
            rules: 0,
            quadruped: false,
            attack_bonus: 0.0,
            age: 0.0,
            parry_bias: 0,
            thrust_pref: 1,
            recoil_until_ms: 0,
            parrying: false,
            yaw: 0.0,
            heading: Vec3::Z,
            facing: Vec3::Z,
            position: Vec3::ZERO,
            step_goal: Vec3::ZERO,
            chest: Vec3::ZERO,
            ground_y: 0.0,
            look: Vec3::Z,
            target_point: Vec3::ZERO,
            to_target: Vec3::Z,
            distance_cm: 1.0,
            swing_amount: None,
            stance_blend: None,
            health: 1.0,
            can_attack: true,
            power_pool: 0.0,
            power_slots: [0; cast::SLOTS],
            power_busy: false,
            grip: 1.0,
            awareness: 1.0,
            blocking: 0,
        }
    }
}

/// What the brain's traits give the combat state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Traits {
    pub skill: f32,
    pub blend: f32,
}

/// One of the brain's contacts.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Contact {
    pub level: i32,
    /// Its route was found bad: the foe cannot be reached [guess: name].
    pub unreachable: bool,
    pub concern: f32,
    pub hostility: f32,
    /// It is the foe (or the focus).
    pub is_foe: bool,
    /// A character (otherwise an entity, which becomes the focus).
    pub character: bool,
    /// Its priority this tick (written).
    pub priority: f32,
}

/// A contact's score of one place, from the position score.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlaceScore {
    pub close: f32,
    pub danger: f32,
    pub approach: f32,
    pub strike: f32,
    /// The yaw that faces the contact from the place.
    pub bearing: f32,
    pub distance_cm: f32,
    /// One less the cosine of the contact's way to the place against the
    /// fighter's heading.
    pub ahead: f32,
}

/// What the rest of the game answers for the combat state.
pub trait Field {
    /// The cost of standing at a point (1 on clear flat ground).
    fn ground_cost(&mut self, _point: Vec3) -> f32 {
        1.0
    }
    /// Walks the line from `from` toward `to`, pulling `to` back where it
    /// stops; true when it was pulled back.
    fn walk_line(&mut self, _from: Vec3, _to: &mut Vec3) -> bool {
        false
    }
    /// Puts a point on the walkable ground. The level's pathfinder never
    /// overrides the abstract class's snap, which leaves the point as it
    /// was.
    fn snap(&mut self, _point: &mut Vec3) {}
    /// The line from `from` to `to` is out of sight.
    fn blocked(&mut self, _from: Vec3, _to: Vec3) -> bool {
        false
    }
    /// A contact's score of a place, `level` the level passed (the foe's,
    /// else 0 in a committed swing).
    fn score(&mut self, contact: usize, level: i32, point: Vec3) -> PlaceScore;
    /// Runs the chase route toward `estimate` (cm); true when the fighter
    /// follows a route and the combat tick ends here, false when the tick
    /// runs on (no route yet, or the fighter left combat).
    fn chase(&mut self, _estimate: Vec3, _roll: &mut u32) -> bool {
        false
    }
    /// The power system, when the world has thaumaturgy.
    fn powers(&mut self) -> Option<&mut dyn cast::Powers> {
        None
    }
    /// The power system is still running the cast the body is in (the
    /// original asks the body's current power's domain evaluator, then,
    /// when that answers nothing, the body's cast hook with 0x10; the cast
    /// ends on a negative answer).
    fn cast_running(&mut self) -> bool {
        false
    }
}

/// Where the foe stands as far as the fighter can tell: the chest, or
/// toward its step's goal by three quarters of the skill when it steps, or
/// ahead of it by its speed × 30 when it runs.
pub fn foe_estimate(foe: &Foe, skill: f32) -> Vec3 {
    if foe.running {
        foe.chest + foe.facing * (foe.forward_speed * 30.0)
    } else if foe.stepping {
        let t = skill * 0.75;
        let p = foe.chest + (foe.step_goal - foe.chest) * t;
        Vec3::new(p.x, foe.chest.y, p.z)
    } else {
        foe.chest
    }
}

/// What the estimate update leaves on the body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bearing {
    /// The direction to the estimate: a unit when farther than 1 cm, else
    /// the direction the last estimate left.
    pub to_target: Vec3,
    pub distance_cm: f32,
}

/// The direction and distance from `from` to `estimate`; within 1 cm (or
/// with a distance that is not a number) the distance is 1 and `previous`
/// stays the direction, as the original leaves it unwritten.
fn bearing_to(estimate: Vec3, from: Vec3, chest_y: f32, previous: Vec3) -> Bearing {
    let raw = estimate - from;
    let d = flat_length(raw);
    let mut out = if d > 1.0 {
        Bearing {
            to_target: raw * (1.0 / d),
            distance_cm: d,
        }
    } else {
        Bearing {
            to_target: previous,
            distance_cm: 1.0,
        }
    };
    if (chest_y - estimate.y).abs() > 300.0 {
        out.distance_cm = 800.0;
    }
    out
}

/// Takes a new character target: the mind is fresh (its first-foe flag
/// set), its commitment full, its overshoot mean and stuck count cleared,
/// and the estimate is placed on the target. The bearing is then measured
/// from `from` (the fighter's chest) as the original's set-target does,
/// without the estimate tick's rules: the distance is the flat length, the
/// direction the whole offset over it (its height scaled too), and the aim
/// takes the direction's yaw. `foe` is the estimate routine's answer for
/// the new foe ([`foe_estimate`]: its chest, led 30 ticks along its facing
/// while it moves, or toward its step goal while it steps; the routine has
/// no draw). The caller copies the aim's yaw into the brain's look, which
/// the original keeps beside it.
pub fn set_target(mind: &mut CombatMind, foe: Vec3, from: Vec3, previous: Vec3) -> Bearing {
    mind.fresh_target = true;
    mind.commitment = 1.0;
    mind.overshoot_mean = 0.0;
    mind.stuck = 0;
    mind.estimate = foe;
    let raw = foe - from;
    let d = flat_length(raw);
    let bearing = if d > 0.0 {
        Bearing {
            to_target: raw * (1.0 / d),
            distance_cm: d,
        }
    } else {
        // [Exarust's guard: the original divides by the zero length and
        // leaves an infinite or NaN direction; the previous one is kept.]
        Bearing {
            to_target: previous,
            distance_cm: d,
        }
    };
    mind.aim = yaw(bearing.to_target);
    bearing
}

/// The estimate's tick: a skill-weighted chance to look again, then the
/// bearing from the fighter's chest (or its step's goal).
pub fn refresh_estimate(
    mind: &mut CombatMind,
    me: &Body,
    fresh: Vec3,
    skill: f32,
    roll: &mut u32,
) -> Option<Bearing> {
    let r = f64::from(unit_draw(roll));
    let k2 = skill * skill;
    let chance = f64::from(k2) * 0.8 + 0.2;
    if chance * chance <= r {
        return None;
    }
    let old = mind.estimate;
    let t = (0.6 - f64::from(k2) * 0.6) as f32;
    mind.estimate = fresh + (old - fresh) * t;
    let from = if me.stepping { me.step_goal } else { me.chest };
    Some(bearing_to(mind.estimate, from, me.chest.y, me.to_target))
}

/// The weapon draw before a fight: fists or a one-handed class with
/// nothing held reach for a carried weapon. `Some(true)` asks for the draw;
/// `Some(false)` holds the level at a watch while it plays.
pub fn draw_weapon(me: &Body) -> Option<bool> {
    if me.weapon_class & 0x30 != 0 && me.held >= 1 {
        return None;
    }
    if me.motion == DRAW_WEAPON {
        return Some(false);
    }
    if me.carried == 0 {
        return None;
    }
    if me.requested_motion == DRAW_WEAPON {
        Some(false)
    } else {
        Some(true)
    }
}

/// The lunge: at level 2 or more and outside a swing, one tick in 300
/// starts it with the measured reach 80 cm past the fighter's.
pub fn lunge(mind: &mut CombatMind, me: &Body, roll: &mut u32) {
    if mind.level < 2 {
        mind.lunging = false;
        return;
    }
    if me.condition != ATTACKING && !mind.lunging && draw(roll, 300) == 0 {
        mind.lunging = true;
        mind.lunge_reach_cm = me.reach_cm + 80.0;
    }
}

/// How far the foe may be before the fighter counts itself stuck (cm).
pub fn chase_range(me: &Body, foe: Option<&Foe>, roll: &mut u32) -> f32 {
    let mut range = match foe {
        Some(f) if f.condition & 0x300 == 0x200 => 400.0,
        Some(f) if f.running => (draw(roll, 100) as i32 + 200) as f32,
        _ => (draw(roll, 200) as i32 + 600) as f32,
    };
    if me.running {
        range *= 0.75;
    }
    range
}

/// What the chase check decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chase {
    /// Close enough, or still counting.
    Stay,
    /// A chase route started: the tick ends.
    Started,
}

/// The chase check and the eagerness, for an engaged fighter (`level` ≥ 2)
/// with balance above 0.85; otherwise the eagerness is 0.
#[allow(clippy::too_many_arguments)]
pub fn chase_and_eagerness(
    mind: &mut CombatMind,
    me: &Body,
    foe: Option<&Foe>,
    distance_cm: f32,
    foe_contact: Option<&mut Contact>,
    traits: Traits,
    field: &mut dyn Field,
    roll: &mut u32,
) -> Chase {
    if mind.level < 2 || me.balance <= BALANCED {
        mind.eagerness = 0.0;
        return Chase::Stay;
    }
    if mind.alert < 0.5 {
        mind.alert = 0.5;
    }
    let range = chase_range(me, foe, roll);
    let height = (me.target_point.y - me.ground_y).abs();
    let unreachable = foe_contact.as_ref().is_some_and(|c| c.unreachable);
    let ground = Vec3::new(me.position.x, me.ground_y, me.position.z);
    if range < distance_cm || 150.0 < height || field.blocked(ground, me.target_point) {
        if mind.stuck < 21 || unreachable {
            mind.stuck += 1;
        } else if field.chase(mind.estimate, roll) {
            return Chase::Started;
        }
    } else {
        mind.stuck = 0;
        if let Some(c) = foe_contact {
            c.unreachable = false;
        }
    }
    if me.max_energy <= me.energy {
        if mind.eagerness <= 6.0 {
            let r = f64::from(unit_draw(roll));
            let step = (r * 0.02 - 0.006) * f64::from(3.0 - traits.skill);
            mind.eagerness = (step + f64::from(mind.eagerness)) as f32;
        } else {
            mind.eagerness = (f64::from(mind.eagerness) - 0.01) as f32;
        }
    } else {
        let f = 1.0 - me.energy / me.max_energy;
        mind.eagerness *= 1.0 - f * f;
        if f64::from(me.energy) < f64::from(me.max_energy) * 0.8 {
            mind.pressing = false;
        }
    }
    Chase::Stay
}

/// The footwork weights.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drives {
    pub attack: f32,
    pub strike: f32,
    pub spacing: f32,
    /// The fighter is past 0.3 of its own swing.
    pub committed: bool,
    /// Flags to set on the body's attack flags.
    pub set_flags: u32,
}

/// The drives of this tick.
pub fn drives(
    mind: &mut CombatMind,
    me: &Body,
    foe: Option<&Foe>,
    traits: Traits,
    roll: &mut u32,
) -> Drives {
    let k = traits.skill;
    let early = if me.weapon_class == FISTS { 0.9 } else { 0.7 };
    let mut committed = false;
    let mut set_flags = 0;
    let (mut attack, mut strike, mut spacing);
    if me.condition != ATTACKING || me.attack_flags & 0x26 != 0 || early <= me.clock {
        mind.measured = false;
        mind.overshoot = (f64::from(mind.overshoot) * 0.99) as f32;
        attack = mind.eagerness * 1.0 + 0.5;
        if me.condition == ATTACKING || me.now_ms < me.recoil_until_ms {
            strike = mind.eagerness * 0.5;
            attack *= 1.0 - k * 0.5;
        } else {
            strike = mind.eagerness;
            if mind.pressing && draw(roll, 40) == 0 {
                mind.pressing = false;
            }
        }
        spacing = k * 2.0 + 3.0;
    } else {
        if f64::from(me.clock) > 0.6 {
            let e = f64::from(mind.eagerness);
            let r = f64::from(unit_draw(roll));
            let spend = f64::from(1.0 - me.age * 0.5) * (0.5 - f64::from(k) * 0.2) * r;
            mind.eagerness = (e - spend) as f32;
            if mind.eagerness < 0.0 {
                mind.eagerness = 0.0;
            }
            if let Some(f) = foe
                && me.attack_flags & 0x30_0000 == 0
            {
                if mind.practice.lesson < f.lesson {
                    mind.practice.lesson = f.lesson;
                }
                set_flags |= 0x10_0000;
            }
        }
        attack = (mind.eagerness - k) * 2.0 + 2.0;
        if f64::from(me.clock) > 0.3 {
            attack *= if me.shield { 4.0 } else { 2.0 };
            committed = true;
            if f64::from(me.clock) > 0.4 {
                mind.pressing = true;
                if 0.5 < me.clock && !mind.measured {
                    let from = if me.stepping {
                        me.step_goal
                    } else {
                        me.position
                    };
                    let d = distance2(from, me.target_point).sqrt();
                    mind.overshoot += (mind.lunge_reach_cm - d) * 2.0;
                    mind.overshoot = mind.overshoot.clamp(-50.0, 100.0);
                    mind.overshoot_mean = (f64::from(mind.overshoot) * 0.1
                        + f64::from(mind.overshoot_mean) * 0.9)
                        as f32;
                    mind.measured = true;
                }
            }
        }
        spacing = k * 2.0 + 2.0;
        strike = mind.eagerness;
    }
    spacing *= 2.0 + k;
    if let Some(f) = foe
        && f.rules & 1 != 0
    {
        attack *= if f.weapon_class == CLAWS { 4.0 } else { 2.0 };
        if committed {
            spacing *= 0.5;
        }
    }
    if !mind.pressing {
        let left = 3000_i32.wrapping_sub(me.now_ms.wrapping_sub(mind.entered_ms) as i32);
        if left < 1 {
            mind.pressing = true;
        } else {
            mind.pressing = draw(roll, left as u64) < 5;
        }
    }
    if mind.level < 3 || me.condition == CASTING {
        attack = 0.0;
        strike = 0.0;
        mind.pressing = false;
    } else {
        let e2 = me.energy * me.energy;
        attack = e2 * (attack * traits.blend);
        strike = e2 * (strike * traits.blend);
        if me.shield {
            strike *= 2.0;
        }
        if foe.is_some_and(|f| f.attack_flags & 4 != 0) {
            attack *= 2.0;
            strike *= 2.0;
        }
    }
    if me.max_energy <= me.energy {
        if mind.pressing {
            attack *= 2.0 + k;
            mind.entered_ms = me.now_ms;
            strike *= 3.0;
            if me.weapon_class == FISTS {
                attack *= 2.0;
            }
        } else {
            attack *= 0.5;
            strike *= 0.5;
            spacing *= 2.0;
        }
    } else {
        attack *= 0.5;
        spacing *= 4.0;
    }
    Drives {
        attack,
        strike,
        spacing,
        committed,
        set_flags,
    }
}

/// How the footwork search runs this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Footwork {
    /// The first place weighed (1 in an early step: where it stands is
    /// skipped).
    pub first: i32,
    /// The last place weighed: 0, 8, 10 or 16.
    pub last: i32,
    /// 0 standing, 1 early in a step, 2 later in it, 3 in a second step.
    pub mode: i32,
    /// The cost of a change of step.
    pub change: f32,
    /// The queued step was taken up: the body's queue goes back to 0.
    pub queue_taken: bool,
}

/// The footwork plan, the step queue's turn-over and the step-change cost.
pub fn footwork(
    mind: &mut CombatMind,
    me: &Body,
    drives: &Drives,
    k: f32,
    roll: &mut u32,
) -> Footwork {
    // A first count the original works out and overwrites: only its draw
    // and its jitter's draw are kept.
    if !me.stepping
        && me.can_step
        && u64::from(mind.last_step_ms) + 600 < u64::from(me.now_ms)
        && f64::from(me.energy) > 0.9
    {
        unit_draw(roll);
    }
    if !me.stepping {
        unit_draw(roll);
    }
    let queue_taken = me.queue < 0;
    if queue_taken {
        mind.step_cost = if me.queue < -1 { 8.0 } else { 4.0 };
        mind.step = mind.queued_step;
        mind.queued_step = 0;
    } else {
        mind.step_cost = (f64::from(mind.step_cost) * 0.95 + 0.1) as f32;
    }
    let (mode, first) = if !me.stepping {
        mind.step = 0;
        (0, 0)
    } else if me.step_state == 2 {
        (3, 0)
    } else if i64::from(me.step_started_ms) + 300 < i64::from(me.now_ms) {
        (2, 0)
    } else {
        (1, 1)
    };
    let mut change = if 1.0 < me.scale {
        let s = me.scale - 1.0;
        s * s * 10.0 + 1.0
    } else {
        1.0
    };
    change = ((f64::from(drives.attack) * 0.1 + 5.0) * f64::from(change)) as f32;
    let last = if !me.can_step {
        0
    } else if me.rules & 0x20 != 0 && me.motion == 0xc {
        change *= 2.0;
        if f64::from(me.clock) > 0.3 { 0 } else { 8 }
    } else {
        let r = f64::from(unit_draw(roll));
        let c = f64::from(k) * 0.9 + 0.2;
        if c * c > r {
            if 1.25 >= me.scale { 16 } else { 10 }
        } else {
            8
        }
    };
    Footwork {
        first,
        last,
        mode,
        change,
        queue_taken,
    }
}

/// The side direction: up × look, unit.
pub fn side_of(look: Vec3) -> Vec3 {
    unit(Vec3::new(look.z, 0.0, -look.x))
}

/// Keeps a step from carrying the fighter into its foe: a step that would
/// enter the foe's 60 cm × scale circle stops at its edge when it heads
/// for the foe (cosine 0.75 or more); inside the circle it backs out to
/// the edge.
pub fn keep_off(me_chest: Vec3, foe_chest: Vec3, foe_scale: f32, offset: &mut Vec3) {
    let v = (offset.x, offset.z);
    let d = (me_chest.x - foe_chest.x, me_chest.z - foe_chest.z);
    let r = crate::manager::CIRCLE_PER_SPEED * foe_scale;
    let d2 = d.0 * d.0 + d.1 * d.1;
    if d2 <= r * r {
        if r > d2 {
            let inv = 1.0 / d2.sqrt();
            offset.x = d.0 * r * inv;
            offset.z = d.1 * r * inv;
        } else {
            offset.x = 0.0;
            offset.z = 0.0;
        }
        return;
    }
    // Outside the circle (or a distance that is not a number, which the
    // original's unordered compare also sends here).
    let c = d2 - r * r;
    let a = v.0 * v.0 + v.1 * v.1;
    let b = (d.0 * v.0 + d.1 * v.1) * 2.0;
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return;
    }
    let t = (-b - disc.sqrt()) / (2.0 * a);
    if !(0.0..=1.0).contains(&t) {
        return;
    }
    let cut = (v.0 * t, v.1 * t);
    // The original builds this vector's x from z and reads its second
    // component from uninitialised stack [?]; 0 stands in for it.
    let w = (me_chest.z + cut.1 - foe_chest.z, 0.0f32);
    let w2 = w.0 * w.0 + w.1 * w.1;
    let cos = (w.0 * d.0 + w.1 * d.1) / (d2 * w2).sqrt();
    // Only a cosine under 0.75 leaves the step: one that is not a number
    // (a zero test vector) cuts it too, as the original's compare does.
    if cos < crate::manager::CUT_HEADING {
        return;
    }
    offset.x = cut.0;
    offset.z = cut.1;
}

/// The offset of footwork direction `dir` (1–8) by `step` cm.
pub fn step_offset(dir: i32, look: Vec3, side: Vec3, step: f32) -> Option<Vec3> {
    let diag = |v: Vec3, s: f32| v * (s / length(v));
    Some(match dir {
        1 => look * step,
        2 => look * -step,
        3 => side * step,
        4 => side * -step,
        5 => diag(look + side, step),
        6 => diag(look - side, step),
        7 => diag(look + side, -step),
        8 => diag(side - look, step),
        _ => return None,
    })
}

/// One weighed place.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Place {
    /// The cost of standing there.
    pub total: f32,
    /// The cost of striking from there.
    pub attack: f32,
    /// The foe's approach term there.
    pub approach: f32,
    /// The danger summed over the contacts.
    pub danger: f32,
    /// The yaw that faces the foe from there.
    pub bearing: f32,
    pub distance_cm: f32,
    /// The step's offset, for a long step in short steps.
    pub offset: Vec3,
}

/// The footwork search's result.
#[derive(Debug, Clone, PartialEq)]
pub struct Search {
    pub places: Vec<Place>,
    /// The cheapest place to stand, if any was weighed.
    pub best_move: Option<usize>,
    /// The cheapest place to strike from, if any beat its standing cost.
    pub best_attack: Option<usize>,
    pub best_move_cost: f32,
    pub best_attack_cost: f32,
    /// The best contact this tick (a new foe), when re-chosen.
    pub retarget: Option<Option<usize>>,
    /// The attack drive after the search (a long side step halves it).
    pub attack: f32,
}

/// Weighs the places. `contacts` are the brain's; the foe's is marked.
#[allow(clippy::too_many_arguments)]
pub fn search(
    mind: &mut CombatMind,
    me: &Body,
    foe: Option<&Foe>,
    traits: Traits,
    plan: Footwork,
    drives: &Drives,
    contacts: &mut [Contact],
    field: &mut dyn Field,
) -> Search {
    let k = traits.skill;
    let side = side_of(me.look);
    let mut attack = drives.attack;
    let mut places = vec![Place::default(); 17];
    let mut best_move = (NO_SCORE, None);
    let mut best_attack = (NO_SCORE, None);
    let mut retarget = None;
    let mut chest = me.chest;
    let mut goal = me.step_goal;
    // The foe's score, kept from the last place that saw it.
    let mut foe_seen = false;
    let mut foe_unreachable = false;
    let mut i = plan.first;
    while i <= plan.last {
        let (dir, step) = if i > 8 {
            let s = ((1.6 - f64::from(me.encumbrance) * 0.3) * f64::from(me.step_cm)) as f32;
            (i - 8, s)
        } else {
            (i, me.step_cm)
        };
        let mut v = match step_offset(dir, me.look, side, step) {
            Some(mut v) => {
                if matches!(dir, 1 | 5 | 6)
                    && let Some(f) = foe
                {
                    keep_off(chest, f.chest, f.scale, &mut v);
                }
                v
            }
            None if me.stepping => goal - chest,
            None => Vec3::ZERO,
        };
        let mut kept = v;
        // The base the offset is added to: the chest or the step's goal.
        let base = match plan.mode {
            1 if mind.step == i => {
                v = goal;
                None
            }
            1 if mind.step + 8 == i => Some(&mut chest),
            1 => Some(&mut goal),
            0 => Some(&mut chest),
            _ if i == 0 => {
                v = goal;
                None
            }
            _ => Some(&mut goal),
        };
        if let Some(base) = base {
            v += *base;
            base.y = me.ground_y;
            if field.walk_line(*base, &mut v) {
                kept = v - *base;
            }
        }
        v.y = me.position.y;
        let at = i as usize;
        places[at] = Place {
            total: field.ground_cost(v),
            ..Place::default()
        };
        if me.stepping && mind.step != i && mind.step + 8 != i && i != 0 {
            if me.landings < 1 {
                places[at].total += plan.change * mind.step_cost;
            } else {
                places[at].total += plan.change;
            }
        }
        let mut foe_score: Option<PlaceScore> = None;
        let mut held_close = 0.0f32;
        let mut held_strike = 0.0f32;
        let free = me.condition != ATTACKING || me.attack_flags & 0x4_0030 != 0;
        if i == 0 && free {
            // Weighs every contact as the foe and picks the most pressing.
            let mut best = (0.1f32, None);
            for (c, contact) in contacts.iter_mut().enumerate() {
                let mut s = field.score(c, contact.level, v);
                let w = ((f64::from(s.ahead + 1.0)) * 0.4) as f32;
                let mut value = s.strike * 1.0
                    + ((k * 1.0 + 1.0) * s.approach + (k * 2.0 + 1.0) * s.danger * w);
                if contact.level > 1 && s.distance_cm < 2000.0 {
                    let f = (f64::from(s.distance_cm) * 0.0005) as f32;
                    value += (1.0 - f) * ((1.0 - f * f) + f);
                }
                value = (f64::from(contact.level) * 0.1 + f64::from(value)) as f32;
                if contact.is_foe {
                    value = (value + 1.0) - k * 0.5;
                }
                contact.priority = value;
                if best.0 < value {
                    best = (value, Some(c));
                    places[at].total += held_close * 2.0;
                    foe_unreachable = contact.unreachable;
                    foe_seen = true;
                    held_close = s.close;
                    held_strike = s.strike;
                    foe_score = Some(s);
                    s.close = 0.0;
                }
                places[at].total += s.danger * 3.0;
                places[at].danger += s.danger;
                places[at].total += s.close * 2.0;
            }
            retarget = Some(best.1);
            mind.level = best.1.map_or(0, |c| contacts[c].level);
        } else {
            for (c, contact) in contacts.iter().enumerate() {
                let level = if contact.is_foe { contact.level } else { 0 };
                let mut s = field.score(c, level, v);
                if contact.is_foe {
                    foe_unreachable = contact.unreachable;
                    foe_seen = true;
                    held_close = s.close;
                    held_strike = s.strike;
                    foe_score = Some(s);
                    s.close = 0.0;
                }
                places[at].total += s.danger * 3.0;
                places[at].danger += s.danger;
                places[at].total += s.close * 2.0;
            }
        }
        if foe_seen {
            if foe_unreachable {
                places[at].approach = 0.0;
                held_strike = 0.0;
            } else {
                let s = foe_score.unwrap_or_default();
                places[at].approach = s.approach;
                places[at].bearing = s.bearing;
                places[at].distance_cm = s.distance_cm;
            }
        }
        if i == 1 && 0.5 < me.age {
            if 1 < mind.level {
                places[at].total = (places[at].total - 0.0) - 1.0;
            }
        } else if i == 9 && me.condition != ATTACKING {
            attack *= 0.5;
        }
        places[at].offset = if i < 9 {
            kept
        } else {
            kept * ((1.0 / (1.6 - f64::from(me.encumbrance) * 0.3)) as f32)
        };
        if 8 < i {
            if f64::from(me.scale) > 1.4 && me.reach_cm * 2.0 > places[0].distance_cm {
                places[at].total += 5.0;
            } else if 10 < i {
                places[at].total += 2.0;
            }
        }
        let p = places[at];
        let gain = p.approach * attack - p.danger * drives.spacing * 0.5;
        if 1.0 < gain {
            let mut cost = p.total - gain;
            if i == 0 {
                if me.condition != ATTACKING {
                    // The jitter is cleared before the search, so both
                    // branches add 0.
                    cost += 0.0;
                }
                if f64::from(me.scale) > 1.3 {
                    cost -= 3.0;
                }
            }
            cost -= held_strike * drives.strike;
            places[at].attack = cost;
            if cost < best_attack.0 {
                best_attack = (cost, Some(at));
            }
        } else {
            places[at].attack = p.total;
        }
        if me.rules & 1 != 0 {
            places[at].total -= held_strike * drives.strike;
        } else {
            let pulled = held_strike * drives.strike;
            if p.danger * drives.spacing + held_close < pulled {
                places[at].total -= pulled;
            } else {
                places[at].total += held_close;
            }
        }
        if places[at].total < best_move.0 {
            best_move = (places[at].total, Some(at));
        }
        i += 1;
    }
    Search {
        places,
        best_move: best_move.1,
        best_attack: best_attack.1,
        best_move_cost: best_move.0,
        best_attack_cost: best_attack.0,
        retarget,
        attack,
    }
}

/// Someone near the fighter's strike.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bystander {
    pub position: Vec3,
    pub stepping: bool,
    pub step_goal: Vec3,
    /// Lying in the knocked-out condition.
    pub out: bool,
    pub is_foe: bool,
    /// Its contact's concern and hostility, when the fighter has one.
    pub concern_hostility: Option<f32>,
}

/// Whether a strike at `point` would hit a friend: 2 when one stands ahead
/// (cosine 0.7 or more) within reach + 80 cm, 1 when one stands to the
/// side, 0 otherwise. A contact whose concern and hostility pass 1.8 is no
/// friend.
pub fn friend_in_the_way(point: Vec3, heading: Vec3, reach_cm: f32, around: &[Bystander]) -> i32 {
    let mut worst = 0;
    for b in around {
        let at = if b.stepping {
            b.position + (b.step_goal - b.position) * 0.5
        } else {
            b.position
        };
        let d = at - point;
        let d2 = d.z * d.z + (d.y * d.y + d.x * d.x);
        let r = reach_cm + 80.0;
        if r * r < d2 || b.out || b.is_foe {
            continue;
        }
        if b.concern_hostility.is_some_and(|ch| f64::from(ch) > 1.8) {
            continue;
        }
        let cos = (d.z * heading.z + (d.y * heading.y + d.x * heading.x)) / d2.sqrt();
        if f64::from(cos) < -0.7 {
            continue;
        }
        let seen = if f64::from(cos) < 0.7 { 1 } else { 2 };
        worst = worst.max(seen);
    }
    worst
}

/// The attack decision: 0 none, 1 strike, 2 strike with a friend aside.
#[allow(clippy::too_many_arguments)]
pub fn decide_attack(
    me: &Body,
    k: f32,
    plan: Footwork,
    places: &[Place],
    best_attack: Option<usize>,
    best_attack_cost: f32,
    stand_cost: f32,
    committed: bool,
    around: &[Bystander],
) -> (i32, Option<usize>) {
    let Some(mut at) = best_attack else {
        return (0, None);
    };
    if at > 0 && plan.mode != 1 && (me.size * 2.0) > places[0].attack - best_attack_cost {
        at = 0;
    }
    let mut choice = if 0.5 <= me.age {
        1
    } else {
        let off = places[at].offset;
        let point = if at < 9 {
            me.chest + off
        } else {
            me.chest + off * ((1.6 - f64::from(me.encumbrance) * 0.3) as f32)
        };
        match friend_in_the_way(point, me.heading, me.reach_cm, around) {
            1 if me.condition == ATTACKING => 0,
            1 => 2,
            n if n > 1 => 0,
            _ => 1,
        }
    };
    if 0 < choice && me.condition != ATTACKING {
        let mut margin = stand_cost - places[at].attack;
        if committed {
            margin *= 2.0;
        }
        if margin < 2.0 {
            choice = 0;
        }
    }
    if 0 < choice && plan.mode != 1 {
        let (dir, reach) = if at > 8 {
            (
                at - 8,
                ((1.0 + me.attack_bonus) as f64 * (f64::from(k) * 0.2)) as f32,
            )
        } else {
            (
                at,
                ((1.0 + me.attack_bonus) as f64 * (f64::from(k) * 0.3)) as f32,
            )
        };
        if matches!(dir, 1 | 5 | 6) {
            if me.condition == ATTACKING {
                if me.clock < reach && (me.rules & 0x20 == 0 || me.motion != 0xc) {
                    at = 0;
                }
            } else {
                at = 0;
            }
        }
    }
    (choice, Some(at))
}

/// The step the tick takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Footstep {
    /// No step.
    Stay,
    /// Queued after the step it is in (the body's queue goes to 1).
    Queue { dir: i32, offset: Vec3 },
    /// A new step to `goal`; `step_cm` is the length set (kept when None).
    Start {
        dir: i32,
        goal: Vec3,
        step_cm: Option<f32>,
    },
    /// The long step's second half to `goal`, `step_cm` long (step state 2).
    Continue { goal: Vec3, step_cm: f32 },
}

/// What stepping leaves on the body besides the step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StepMarks {
    /// The chosen place is the step it is in: the step state goes to 0 and
    /// the length back to the base step.
    pub restart: bool,
    /// The queue goes back to 0.
    pub clear_queue: bool,
}

/// The step from the chosen place, and the long-step mark.
pub fn take_step(
    mind: &mut CombatMind,
    me: &Body,
    foe: Option<&Foe>,
    plan: Footwork,
    chosen: usize,
    offset: Vec3,
    field: &mut dyn Field,
) -> (Footstep, StepMarks) {
    let mut marks = StepMarks::default();
    let mut dir = chosen as i32;
    let mut step_state = me.step_state;
    if dir > 8 {
        dir -= 8;
        if mind.step == dir {
            step_state = 0;
            marks.restart = true;
            mind.long_step = true;
        }
    } else if dir > 0 && dir == mind.step {
        mind.long_step = false;
        step_state = 0;
        marks.restart = true;
    }
    for _ in 0..2 {
        if dir == 0 {
            marks.clear_queue = true;
        }
        if plan.mode == 1 && mind.step == dir {
            dir = 0;
        }
    }
    if dir > 0 {
        if me.stepping {
            mind.queued_step = dir;
            return (Footstep::Queue { dir, offset }, marks);
        }
        let step_cm = if step_state == 0 {
            Some(me.step_cm)
        } else {
            mind.last_step_ms = me.now_ms;
            None
        };
        let mut goal = me.chest + offset;
        field.snap(&mut goal);
        mind.step = dir;
        return (Footstep::Start { dir, goal, step_cm }, marks);
    }
    if mind.long_step && u64::from(me.step_started_ms) + 200 < u64::from(me.now_ms) {
        // The side here is not made a unit, as in the original.
        let side = Vec3::new(me.look.z, 0.0, -me.look.x);
        let step_cm = ((1.5 - f64::from(me.encumbrance) * 0.3) * f64::from(me.step_cm)) as f32
            * me.step_factor;
        let mut v = match mind.step {
            1 => {
                let mut v = me.look * step_cm;
                if let Some(f) = foe {
                    keep_off(me.chest, f.chest, f.scale, &mut v);
                }
                v
            }
            d => step_offset(d, me.look, side, step_cm).unwrap_or(Vec3::ZERO),
        };
        v += me.chest;
        let from = Vec3::new(me.chest.x, me.ground_y, me.chest.z);
        field.walk_line(from, &mut v);
        field.snap(&mut v);
        mind.long_step = false;
        mind.last_step_ms = me.now_ms;
        return (Footstep::Continue { goal: v, step_cm }, marks);
    }
    (Footstep::Stay, marks)
}

/// The guard the fighter falls back to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Guard {
    /// The stance motion: 3 the guard, 4 the combat idle.
    pub stance: i32,
    pub blend: u32,
}

/// What the attack part decided.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Blow {
    pub motion: Option<AttackMotion>,
    /// Start a release of the swing in progress.
    pub release: bool,
    /// Clear the attack flags but 0x40030.
    pub clear_flags: bool,
    pub guard: Option<Guard>,
    pub set_flags: u32,
}

/// The heading the swing turns by: σ(clock) times the turn amount.
pub fn swing_sweep(me: &Body, mind: &CombatMind) -> f32 {
    let mut s = if me.clock <= CLOCK_EPS {
        -80.0
    } else if f64::from(me.clock) < 0.2 {
        -60.0
    } else if 0.5 < me.clock {
        30.0
    } else {
        ((f64::from(me.clock) - 0.2) * 300.0 + -60.0) as f32
    };
    if me.heavy_claw {
        s += 50.0;
    } else if me.weapon_class & 0x30 == 0 {
        s -= 50.0;
    }
    if mind.lunging {
        s *= 1.5;
    }
    s
}

/// One tick's outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    pub exit: Option<Exit>,
    pub practice_paid: i32,
    /// The run stop, when the body is running.
    pub stop_run: Option<Stopped>,
    /// A power this tick chose to cast.
    pub cast: Option<cast::Choice>,
    /// The cast in progress ended: the body's cast is torn down.
    pub cast_stopped: bool,
    pub draw_weapon: bool,
    /// The estimate's new bearing, when it was looked at again.
    pub bearing: Option<Bearing>,
    pub search: Option<Search>,
    /// The chosen place and whether it strikes from it.
    pub chosen: Option<(usize, bool)>,
    pub step: Footstep,
    pub marks: StepMarks,
    pub blow: Blow,
    /// The look direction from the heading.
    pub look: Option<Vec3>,
}

/// Why a tick ended early.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Exit {
    /// The leader calls: back to following.
    Follow,
    /// Down: the stance resets.
    Down,
    /// No foe and no focus: back to the activity, looking around from
    /// `look_ms`, trying again from `next_ms`.
    Leave { look_ms: u32, next_ms: u32 },
    /// A chase route started.
    Chase,
    /// Running in: it only aims.
    Charge,
    /// No place could be weighed.
    NoPlace,
}

/// The leader a fighting follower keeps track of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leader {
    pub position: Vec3,
    pub facing: Vec3,
    /// Its locomotion asks for a sprint (bit 4) or a run (bit 8).
    pub sprinting: bool,
    pub running: bool,
}

/// The follower's side of the follow check.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Following {
    /// How far the leader may go before the follower breaks off (cm).
    pub band: f32,
    /// It is following (its walk state is the follow).
    pub following: bool,
    /// Where its route to the leader leads, when it has one.
    pub route_to: Option<Vec3>,
}

/// What the follow check decided.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FollowCheck {
    /// Close enough: a follower stops its walk.
    pub stop: bool,
    /// The leader calls: the combat tick ends to follow.
    pub calls: bool,
    /// Plan a route to the leader (its ground point, 20 cm up, stop 40 cm).
    pub replan: bool,
    /// The gait the follow walks in: 2 to run, 1 to walk.
    pub urgency: i32,
}

/// The follow check a fighting follower makes each combat tick: the band
/// eases toward 700 cm (or, while the leader sprints, toward 500 cm plus
/// 100 cm × how far ahead of the leader the follower is), 0.7 of it while
/// following; inside it a follower stops; outside it the follower runs
/// when the leader runs or is three bands away, and a follower with a
/// route answers the call (replanning when the leader has moved 100 cm
/// from the route's end).
pub fn follow_check(me: Vec3, leader: &Leader, f: &mut Following) -> FollowCheck {
    let d = me - leader.position;
    let dist = ((d.x * d.x + d.z * d.z) + (d.y * d.y + 0.0)).sqrt();
    let ahead = (d.z * leader.facing.z + (d.y * leader.facing.y + d.x * leader.facing.x)) / dist;
    f.band = if leader.sprinting {
        let toward = ahead * 100.0 + 500.0;
        (f64::from(f.band) * 0.8 + f64::from(toward) * 0.2) as f32
    } else {
        (f64::from(f.band) * 0.99 + f64::from(700.0f32) * 0.01) as f32
    };
    let band = if f.following {
        (f64::from(f.band) * 0.7) as f32
    } else {
        f.band
    };
    let mut out = FollowCheck::default();
    if band > dist {
        out.stop = f.following;
        return out;
    }
    out.urgency = if leader.running || f.band * 3.0 < dist {
        2
    } else {
        1
    };
    if f.following {
        match f.route_to {
            None => out.replan = true,
            Some(end) => {
                out.calls = true;
                out.replan = distance2(end, leader.position) > 10_000.0;
            }
        }
    } else {
        out.replan = true;
    }
    out
}

/// The rest of the world one tick needs.
pub struct World<'a> {
    pub foe: Option<&'a Foe>,
    /// The focus entity's aim point, without a foe.
    pub focus: Option<Vec3>,
    pub contacts: &'a mut [Contact],
    pub around: &'a [Bystander],
    pub field: &'a mut dyn Field,
    /// The fighter follows (activity 4) and its leader calls it back.
    pub leader_calls: bool,
}

/// What the guard branches share: release the swing, or clear its flags.
fn let_go(me: &Body, blow: &mut Blow) {
    if me.condition == ATTACKING {
        if me.attack_flags & 0x34 == 0 {
            blow.release = true;
        }
    } else {
        blow.clear_flags = true;
    }
}

/// Whether the body may act: no blocking condition bit, a grip of at least
/// 0.7, an awareness above 0.7 (a double compare) and no trailing-foot
/// request pending. A NaN grip or awareness fails its compare.
pub fn can_act(me: &Body) -> bool {
    me.condition & exa_physics::motile::ACTION_BLOCK == 0
        && 0.7 <= me.grip
        && 0.7 < f64::from(me.awareness)
        && me.blocking == 0
}

/// The locomotion bits the run stop writes: the run clears, the stopping
/// flag sets.
pub const LOCO_RUN: u32 = 4;
pub const LOCO_STOP: u32 = 0x20;

/// What stopping a run leaves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stopped {
    pub flags: u32,
    /// The step's goal, pulled inside 8·speed + 20 cm and clamped by the
    /// navigation line walk.
    pub step_goal: Vec3,
}

/// Stops a run: the run bit clears and the stopping flag sets (the walk
/// state, follow and goal marks clear with them), and a step goal farther
/// than `8·speed + 20` cm from the body is put that far along the body's
/// facing (not toward the old goal), then walked from the ground point with
/// the navigation line walk.
pub fn stop_run(me: &Body, flags: u32, step_goal: Vec3, field: &mut dyn Field) -> Stopped {
    let radius = me.forward_speed * 8.0 + 20.0;
    let mut goal = step_goal;
    if radius * radius < distance2(me.position, goal) {
        goal = me.position + me.facing * radius;
    }
    let from = Vec3::new(me.position.x, me.ground_y, me.position.z);
    field.walk_line(from, &mut goal);
    Stopped {
        flags: (flags & !LOCO_RUN) | LOCO_STOP,
        step_goal: goal,
    }
}

/// The body is free to change its stance motion.
fn stance_free(me: &Body) -> bool {
    !me.parrying && me.motion != 0x1c && me.motion != DRAW_WEAPON && me.recoil_until_ms < me.now_ms
}

/// One tick of the combat state.
pub fn tick(
    mind: &mut CombatMind,
    me: &Body,
    traits: Traits,
    world: World<'_>,
    roll: &mut u32,
) -> Tick {
    let k = traits.skill;
    let mut out = Tick {
        exit: None,
        practice_paid: practise(&mut mind.practice),
        stop_run: None,
        cast: None,
        cast_stopped: false,
        draw_weapon: false,
        bearing: None,
        search: None,
        chosen: None,
        step: Footstep::Stay,
        marks: StepMarks::default(),
        blow: Blow::default(),
        look: None,
    };
    if !me.stepping && world.leader_calls {
        out.exit = Some(Exit::Follow);
        return out;
    }
    if me.condition & DOWN != 0 {
        mind.aim = yaw(me.facing);
        out.exit = Some(Exit::Down);
        return out;
    }
    // A foe drops the focus; a dead foe is dropped and the fighter leaves
    // even with a focus (the original cleared the focus when it took the
    // foe). Without a foe, the focus keeps it in combat.
    let foe = world.foe.filter(|f| !f.dead);
    let focus = if world.foe.is_some() {
        None
    } else {
        world.focus
    };
    if foe.is_none() && focus.is_none() {
        mind.heading = 0.0;
        mind.aim = yaw(me.heading);
        out.exit = Some(Exit::Leave {
            look_ms: me.now_ms.wrapping_add(3000),
            next_ms: me.now_ms.wrapping_add(500),
        });
        return out;
    }
    let fresh = match (foe, focus) {
        (Some(f), _) => foe_estimate(f, k),
        (None, Some(p)) => p,
        (None, None) => mind.estimate,
    };
    out.bearing = refresh_estimate(mind, me, fresh, k, roll);
    let (to_target, distance_cm) = out.bearing.map_or((me.to_target, me.distance_cm), |b| {
        (b.to_target, b.distance_cm)
    });
    match draw_weapon(me) {
        Some(true) => out.draw_weapon = true,
        Some(false) => mind.level = 1,
        None => {}
    }
    lunge(mind, me, roll);
    let foe_index = world.contacts.iter().position(|c| c.is_foe);
    if let Some(i) = foe_index {
        mind.level = world.contacts[i].level;
    }
    let foe_contact = foe_index.map(|i| &mut world.contacts[i]);
    if chase_and_eagerness(
        mind,
        me,
        foe,
        distance_cm,
        foe_contact,
        traits,
        world.field,
        roll,
    ) == Chase::Started
    {
        out.exit = Some(Exit::Chase);
        return out;
    }
    // A running body stops its run before the drives (the original also
    // stops it on the chase count's reachable branch; the second stop finds
    // the goal already inside).
    if me.running {
        out.stop_run = Some(stop_run(me, 0, me.step_goal, world.field));
    }
    let d = drives(mind, me, foe, traits, roll);
    out.blow.set_flags |= d.set_flags;
    if me.running && 3.0 < me.forward_speed {
        let ground = Vec3::new(me.position.x, me.ground_y, me.position.z);
        mind.heading = yaw(unit(me.target_point - ground));
        out.exit = Some(Exit::Charge);
        return out;
    }
    // The cast, at the first place in the original, after that place's
    // contact pass: no place term before it reads the attack drive, so
    // clearing the drive here is the same. Only a body that is not
    // attacking and has a foe looks at it. A cast running for more than a
    // second (64-bit compare, no wrap) is torn down once the power system
    // is done with it; otherwise a fighter not yet casting may choose one.
    // [Not kept: the original's place pass may drop the foe first, and then
    // casts nothing.]
    if me.condition != ATTACKING && foe.is_some() {
        if me.condition == CASTING {
            if u64::from(mind.cast_start_ms) + 1000 < u64::from(me.now_ms)
                && !world.field.cast_running()
            {
                mind.casting = false;
                out.cast_stopped = true;
            }
        } else if !mind.casting
            && let Some(powers) = world.field.powers()
            && let Some(choice) = cast::choose(&mut mind.casts, &me.power_slots, me, powers, roll)
        {
            mind.casting = true;
            mind.cast_start_ms = me.now_ms;
            out.cast = Some(choice);
        }
    }
    let d = if out.cast.is_some() {
        Drives { attack: 0.0, ..d }
    } else {
        d
    };
    let plan = footwork(mind, me, &d, k, roll);
    let found = search(mind, me, foe, traits, plan, &d, world.contacts, world.field);
    if let Some(Some(c)) = found.retarget
        && !world.contacts[c].is_foe
        && world.contacts[c].character
    {
        // A new foe: committed afresh, the overshoot forgotten.
        mind.fresh_target = true;
        mind.commitment = 1.0;
        mind.overshoot_mean = 0.0;
        mind.stuck = 0;
    }
    let Some(mut chosen) = found.best_move else {
        out.search = Some(found);
        out.exit = Some(Exit::NoPlace);
        return out;
    };
    let best_attack = found
        .best_attack
        .filter(|_| found.best_move_cost >= found.best_attack_cost);
    let stand_cost = if plan.mode == 1 {
        found.places[mind.step.clamp(0, 16) as usize].total
    } else {
        found.places[0].total
    };
    let (choice, at) = decide_attack(
        me,
        k,
        plan,
        &found.places,
        best_attack,
        found.best_attack_cost,
        stand_cost,
        d.committed,
        world.around,
    );
    let mut guard_kind;
    if choice == 0 {
        if stand_cost - found.places[chosen].total < 1.0 {
            chosen = 0;
        }
        if mind.pressing && found.places[chosen].approach < 1.0 && draw(roll, 10) == 0 {
            mind.pressing = false;
        }
        guard_kind = if me.rules & 4 == 0 { 1 } else { 0x10 };
    } else {
        chosen = at.unwrap_or(0);
        guard_kind = 0;
    }
    out.chosen = Some((chosen, choice > 0));
    let place = found.places[chosen];
    let (step, marks) = take_step(mind, me, foe, plan, chosen, place.offset, world.field);
    out.step = step;
    out.marks = marks;
    let mut turn_to = yaw(to_target);
    if 0 < choice && HALF_PI < gap(me.yaw, place.bearing).abs() {
        guard_kind = 1;
    }
    if guard_kind == 0 {
        let blow = choose_blow(mind, me, foe, k, chosen, &place, roll);
        out.blow.motion = blow.motion;
        out.blow.set_flags |= blow.set_flags;
        if blow.guard {
            guard_kind = 1;
        }
    }
    if guard_kind == 1 && me.rules & 4 != 0 {
        guard_kind = 0x10;
    }
    if guard_kind == 0 {
        turn_to = sweep_heading(mind, me, turn_to);
        approach_heading(mind, turn_to);
        mind.aim = turn_to;
    } else if mind.casting {
        approach_heading(mind, turn_to);
    } else if guard_kind == 1 {
        if f64::from(me.size) < 1.3 {
            let_go(me, &mut out.blow);
            if stance_free(me) {
                let stance = match stance_by_energy(me.health, me.energy, me.max_energy) {
                    Stance::Defence => GUARD,
                    _ => COMBAT_IDLE,
                };
                if me.motion != stance {
                    let blend = me.stance_blend.map_or(40, |w| (w << 2) as u32);
                    out.blow.guard = Some(Guard { stance, blend });
                }
            }
        }
        if let Some(f) = foe {
            let edge = f.reach_cm + 50.0;
            let mut lean = if distance_cm < edge {
                if f.reach_cm - 70.0 < distance_cm {
                    (edge - distance_cm) * f32::from_bits(0x3c08_8889)
                } else {
                    1.0
                }
            } else {
                0.0
            };
            lean *= k;
            if (5..=7).contains(&f.motion) {
                turn_to = wrap(turn_to + lean * 25.0 * DEGREE);
            } else if (8..=10).contains(&f.motion) {
                turn_to = wrap(turn_to + lean * -25.0 * DEGREE);
            }
            approach_heading(mind, turn_to);
        }
        mind.aim = turn_to;
    } else {
        let_go(me, &mut out.blow);
        let stance = if me.reach_cm * 4.0 > distance_cm {
            COMBAT_IDLE
        } else {
            GUARD
        };
        if stance_free(me) && me.motion != stance {
            let blend = me.stance_blend.map_or(40, |w| (w << 1) as u32);
            out.blow.guard = Some(Guard { stance, blend });
        }
        if f64::from(gap(turn_to, mind.aim).abs()) > REAIM || me.stepping {
            mind.aim = turn_to;
        }
        let aim = mind.aim;
        approach_heading(mind, aim);
    }
    out.look = Some(of_yaw(mind.heading));
    out.search = Some(found);
    out
}

/// The heading moves 0.03 of its gap to `to`.
fn approach_heading(mind: &mut CombatMind, to: f32) {
    let g = gap(to, mind.heading);
    let step = (f64::from(g) * 0.03) as f32;
    mind.heading = wrap(mind.heading + step);
}

/// During a swing: the sweep turns the heading, and late in it the
/// commitment halves (or clears with a stall, a hit or a release).
fn sweep_heading(mind: &mut CombatMind, me: &Body, mut to: f32) -> f32 {
    let s = swing_sweep(me, mind);
    let late = |mind: &mut CombatMind| {
        if me.attack_flags & 0xa_0012 != 0 {
            mind.commitment = 0.0;
        } else {
            mind.commitment *= 0.5;
        }
    };
    if (5..=7).contains(&me.motion) {
        to = if me.weapon_class == 0 {
            wrap(to + (s * mind.swing_turn * 0.5 - 30.0) * DEGREE)
        } else {
            wrap(to + s * mind.swing_turn * DEGREE)
        };
        if f64::from(me.clock) > 0.68 {
            late(mind);
        }
    } else if (8..=10).contains(&me.motion) {
        to = if me.weapon_class == 0 {
            wrap(to + (-s * mind.swing_turn * 0.5 + 30.0) * DEGREE)
        } else {
            wrap(to + -s * mind.swing_turn * DEGREE)
        };
        if f64::from(me.clock) > 0.78 {
            late(mind);
        }
    } else if (me.motion == 0xb || me.motion == 0xc) && f64::from(me.clock) > 0.9 {
        mind.commitment = 0.0;
    }
    to
}

/// What the blow part returns.
struct Chosen {
    motion: Option<AttackMotion>,
    guard: bool,
    set_flags: u32,
}

/// The blow for the chosen place, through the attack choice.
fn choose_blow(
    mind: &mut CombatMind,
    me: &Body,
    foe: Option<&Foe>,
    k: f32,
    chosen: usize,
    place: &Place,
    roll: &mut u32,
) -> Chosen {
    mind.request = 0;
    let mut set_flags = 0;
    if me.condition == ATTACKING {
        if mind.lunging && f64::from(me.clock) > 0.3 {
            mind.lunge_reach_cm = me.reach_cm * 1.0;
            mind.lunging = false;
        }
        let mut motion = None;
        if let Some(f) = foe
            && (f.condition & DOWN != 0 || f64::from(f.crouch) > 0.9)
        {
            set_flags |= 0x2_0000;
            if !f.running {
                mind.request = 8;
                motion = Some(AttackMotion::Overhead);
            }
        }
        return Chosen {
            motion,
            guard: false,
            set_flags,
        };
    }
    mind.side = 0;
    let situation = fight::Situation {
        attacking: false,
        target_down: foe.is_some_and(|f| f.condition & DOWN != 0),
        target_supported: foe.is_some_and(|f| f.supported),
        target_crouch: foe.map_or(0.0, |f| f.crouch),
        target_stepping: foe.is_some_and(|f| f.running),
        unarmed: me.weapon_class == FISTS,
        open_hands: me.weapon_class & 0x30 == 0,
        quadruped: foe.is_some() && me.quadruped,
        forward_step: chosen == 1,
        distance_cm: place.distance_cm,
        reach_cm: me.reach_cm,
        speed: me.scale,
        yaw: me.yaw,
        aim_yaw: place.bearing,
        skill: k,
        phase: me.parry_bias,
        thrust_pref: me.thrust_pref,
        age: me.age,
        energy: me.energy,
        energy_max: me.max_energy,
        swing_turn: me.swing_amount.unwrap_or(0.0),
    };
    match fight::choose_ready_attack(&situation, me.can_attack, roll) {
        Some(choice) => {
            match choice.motion {
                AttackMotion::Overhead => mind.request = 8,
                AttackMotion::Stab => mind.request = 0x10,
                AttackMotion::Other(_) => mind.side = 2,
                AttackMotion::Main(_) => {}
            }
            mind.swing_turn = choice.turn;
            Chosen {
                motion: Some(choice.motion),
                guard: false,
                set_flags,
            }
        }
        None => Chosen {
            motion: None,
            guard: true,
            set_flags,
        },
    }
}

#[cfg(test)]
#[path = "combat_test.rs"]
mod tests;

/// The brain's morale as the AI tick drifts it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoraleDrift {
    /// The stress, which decays 0.01 a tick.
    pub stress: f32,
    /// The alarm, which decays by 0.9999 a tick.
    pub alarm: f32,
    /// Panic, fed by the stress and the alarm over the courage.
    pub panic: f32,
    /// Courage, which drifts toward `max(2·(stress + alarm), 2·total,
    /// confidence + 0.5)`.
    pub courage: f32,
    /// A random walk in ±1.
    pub jitter: f32,
    /// The health ratio the last tick measured.
    pub health_ratio: f32,
    /// The personality's confidence.
    pub confidence: f32,
    /// The concern total the contact integration leaves.
    pub total: f32,
}

impl Default for MoraleDrift {
    fn default() -> Self {
        Self {
            stress: 0.0,
            alarm: 0.0,
            panic: 0.0,
            courage: 0.5,
            jitter: 0.0,
            health_ratio: 1.0,
            confidence: 0.0,
            total: 0.0,
        }
    }
}

/// The body's health, which the morale reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoraleBody {
    pub health: f32,
    pub max_health: f32,
    pub down: bool,
}

/// One tick of the morale: the stress and alarm decay; the health ratio
/// (scaled by `0.4 + 0.2·confidence` while down) lowers the courage when it
/// drops; the courage eases off above `(h + 1)(confidence + 1.1)/2`, then
/// drifts toward its floor (the stress and alarm doubled, the concern total
/// doubled, the confidence plus a half) at 0.0003 a tick (0.00015 from 1 up)
/// times that factor, or decays by 0.999 at or over it; the panic takes
/// 0.001 of the stress over the courage and decays by 0.995; and the jitter
/// wanders by a hundredth of a centred draw (falling 0.003 while down),
/// clamped to ±1. The steps the original takes in double are taken in
/// double.
pub fn drift_morale(state: &mut MoraleDrift, body: MoraleBody, roll: &mut u32) {
    state.stress = (f64::from(state.stress) - 0.01) as f32;
    if state.stress < 0.0 {
        state.stress = 0.0;
    }
    state.alarm = (f64::from(state.alarm) * 0.9999) as f32;
    let scale = if body.down {
        (f64::from(state.confidence) * 0.2 + 0.4) as f32
    } else {
        1.0
    };
    let ratio = (body.health / body.max_health) * scale;
    if ratio < state.health_ratio {
        let kept = ((ratio / state.health_ratio) * 0.5 + 0.5) * state.courage;
        state.courage = (f64::from(kept) * 0.9) as f32;
    }
    state.health_ratio = ratio;
    let factor = (f64::from(1.0 + ratio) * (f64::from(state.confidence) + 1.1) * 0.5) as f32;
    if state.courage > factor {
        state.courage = (f64::from(state.courage) - 0.0001) as f32;
    }
    let mut floor = (state.stress + state.alarm) * 2.0;
    if state.total * 2.0 > floor {
        floor = state.total * 2.0;
    }
    if 0.5 + state.confidence > floor {
        floor = 0.5 + state.confidence;
    }
    if state.courage < floor {
        let rate = if state.courage < 1.0 { 0.0003 } else { 0.00015 };
        let step = (floor - state.courage) * factor;
        state.courage = (f64::from(step) * rate + f64::from(state.courage)) as f32;
    } else {
        state.courage = (f64::from(state.courage) * 0.999) as f32;
    }
    let over = (state.stress + state.alarm) - state.courage;
    if 0.0 < over {
        state.panic = (f64::from(over) * 0.001 + f64::from(state.panic)) as f32;
    }
    state.panic = (f64::from(state.panic) * 0.995) as f32;
    if body.down {
        state.jitter = (f64::from(state.jitter) - 0.003) as f32;
        if state.jitter < -1.0 {
            state.jitter = -1.0;
        }
    } else {
        let centred = unit_draw(roll) - 0.5;
        // A NaN stays, as the original's ordered compares leave it.
        state.jitter =
            ((f64::from(centred) * 0.01 + f64::from(state.jitter)) as f32).clamp(-1.0, 1.0);
    }
}
