//! The player's combat footwork steps, as the original's behaviour lays
//! them each combat tick: the movement word ([`MoveWord`]) feeds a step
//! dispatcher that starts a step toward a movement goal, re-lays a young
//! step at the tapped (long) stride, or queues the next step on the body
//! while one is under way. The body takes a queued step when its step
//! lands (`exa_physics::body::queued_step`), and the word then takes the
//! queued word back. Rules: `docs/PLAYER.md` (Combat footwork).
//!
//! - **The gate.** No step is laid while the condition has any of
//!   [`NO_STEP`] (walking, down, the shocked word's bits, the kick), the
//!   energy is under 0.7, a swing is under way, or the word is zero.
//! - **Under way.** A step counts as under way while it is started and
//!   either has not landed or its target is still more than 20·scale cm
//!   from the chest across the ground (a NaN distance is not under way).
//! - **Young steps.** Within 300 ms of the step's start, a word that is not
//!   yet a tap queues the raw word when it differs from the step's own
//!   word (the analog word queues even when it is the same); a tapped word
//!   re-lays the step at the long stride unless it already is long. Older
//!   steps queue any nonzero raw word.
//! - **A free body.** With no step under way the goal is laid from the held
//!   word, and the step starts when the goal is over 400 cm² from the
//!   chest.

use bevy::prelude::*;

use crate::steering::{MOVE_ANALOG, MoveWord, key_goal_direction, key_step};

/// The condition bits under which no step is laid: walking (1), the
/// shocked word's two bits (0x40, 0x80), down (0x200) and the kick
/// (0x8000).
pub(crate) const NO_STEP: u32 = 0x82C1;
/// A step needs at least this much energy (an ordered compare: a NaN
/// energy lays no step).
pub(crate) const STEP_ENERGY: f32 = 0.7;
/// A step this young (ms) re-lays or queues by the tap rules.
pub(crate) const YOUNG_STEP_MS: u64 = 300;
/// A step's target this far from the chest (cm, times the body's scale)
/// keeps it under way after it landed.
pub(crate) const UNDER_WAY_CM: f32 = 20.0;
/// Exarust's own: the interim footwork has no feet to land a step, so a
/// step lands when its target is within [`UNDER_WAY_CM`]·scale of the chest
/// across the ground, or once it has run this long (ms), so a step a wall
/// stops does not stand forever.
pub(crate) const STEP_GIVE_UP_MS: u64 = 1000;

/// The body's footwork step, as the dispatcher reads and writes it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct StepRecord {
    /// A step was started.
    pub(crate) active: bool,
    /// Its feet landed.
    pub(crate) landed: bool,
    /// When it started (ms of the game clock).
    pub(crate) started_ms: u64,
    /// It was laid at the tapped (long) stride.
    pub(crate) long: bool,
    /// The queue: positive while a step waits, −2 or −1 once the body took
    /// it (before or after its first landing), 0 for none.
    pub(crate) queue: i32,
    /// The queued step's vector (m).
    pub(crate) queued: Vec3,
    /// Where the step goes (m).
    pub(crate) target: Vec3,
    /// Exarust's own: when the current leg (the step, or the queued step
    /// that followed it) began, for [`STEP_GIVE_UP_MS`].
    pub(crate) leg_ms: u64,
}

impl StepRecord {
    /// Whether the step is under way: started, and either not landed or
    /// its target more than 20·`scale` cm from `chest` across the ground,
    /// as the original compares the squares (single precision, strictly
    /// over; a NaN is not under way).
    pub(crate) fn under_way(&self, chest: Vec3, scale: f32) -> bool {
        if !self.active {
            return false;
        }
        if !self.landed {
            return true;
        }
        // Across the ground: the height is zeroed before the squares are
        // summed, so the sum is the x and z squares in that order.
        let d = (self.target - chest) * 100.0;
        let squared = d.x * d.x + d.z * d.z;
        let limit = UNDER_WAY_CM * scale;
        limit * limit < squared
    }

    /// Queues the step for the raw word `word`: its vector is the word's
    /// step vector (cm) at the plain reach `reach_cm` and the factor
    /// `scale_root`, along `facing` (the analog word along `analog`).
    pub(crate) fn queue_word(
        &mut self,
        keys: &mut MoveWord,
        word: u32,
        facing: Vec3,
        analog: Vec3,
        reach_cm: f32,
        scale_root: f32,
    ) {
        keys.queued_word = word;
        self.queue = 1;
        self.queued = key_step(facing, word, analog, reach_cm, scale_root) / 100.0;
    }

    /// Starts the step: started now and not landed.
    pub(crate) fn start(&mut self, now_ms: u64) {
        self.active = true;
        self.landed = false;
        self.started_ms = now_ms;
        self.leg_ms = now_ms;
    }
}

/// What the dispatcher asks this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepAsk {
    /// Nothing.
    Nothing,
    /// Queue the raw word as the next step.
    Queue(u32),
    /// Lay the goal from the held word; start the step if it is taken.
    Start(u32),
    /// Re-lay the goal of the young step from the held word at the long
    /// stride; the step does not restart.
    Relay(u32),
}

/// What the dispatcher reads besides the word and the step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StepGate {
    /// The body's condition word.
    pub(crate) condition: u32,
    /// Its energy (0 to 1).
    pub(crate) energy: f32,
    /// A swing is under way (the cursor's attack mode).
    pub(crate) swinging: bool,
}

impl StepGate {
    /// Whether a step may be laid at all.
    pub(crate) fn open(&self) -> bool {
        // The ordered energy compare fails on NaN.
        self.condition & NO_STEP == 0 && self.energy >= STEP_ENERGY && !self.swinging
    }
}

/// The dispatcher, one combat tick after the word's tick: `raw` is the
/// word read this tick and `keys` the word state after it; `under_way`
/// is [`StepRecord::under_way`].
pub(crate) fn dispatch(
    keys: &MoveWord,
    raw: u32,
    now_ms: u64,
    step: &StepRecord,
    under_way: bool,
    gate: StepGate,
) -> StepAsk {
    if !gate.open() || keys.held == 0 {
        return StepAsk::Nothing;
    }
    if !under_way {
        return StepAsk::Start(keys.held);
    }
    if now_ms >= step.started_ms + YOUNG_STEP_MS {
        return if raw != 0 {
            StepAsk::Queue(raw)
        } else {
            StepAsk::Nothing
        };
    }
    if !keys.tap {
        if keys.held == keys.last_word {
            if raw == MOVE_ANALOG {
                StepAsk::Queue(raw)
            } else {
                StepAsk::Nothing
            }
        } else if raw != 0 {
            StepAsk::Queue(raw)
        } else {
            StepAsk::Nothing
        }
    } else if !step.active || !step.long {
        StepAsk::Relay(keys.held)
    } else {
        StepAsk::Nothing
    }
}

/// The movement goal's direction for `word` along `facing` (the analog
/// word along `analog`), its height dropped.
pub(crate) fn goal_direction(facing: Vec3, word: u32, analog: Vec3) -> Vec3 {
    if word == MOVE_ANALOG {
        analog.with_y(0.0)
    } else {
        key_goal_direction(facing, word)
    }
}

#[cfg(test)]
#[path = "footwork_test.rs"]
mod tests;
