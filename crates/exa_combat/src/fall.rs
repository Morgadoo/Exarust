//! Falling to death, as the original's body tick checks it on every living
//! body that is off its balance and still moving (the falls branch of the
//! posture stage, [`crate::condition::Branch::Falling`]).
//!
//! A body whose chest sinks faster than 5 cm a tick (its slow smoothed
//! vertical velocity) screams once, unless dead. If, then, the ground under
//! it and under each foot is 25 m or more below its pelvis (a Height Query
//! that meets nothing within its 20 m gives ground 100 m down, so in
//! practice: no ground within reach under it), it is lost: its time stamp
//! is taken on the game clock. More than 1 s later the player hears the
//! pit and its screen starts to fade; more than 4 s later it dies and is
//! taken out of the world. Any other character dies and is taken out 3 s
//! after it was lost. The timeline only moves while the body stays in the
//! falls branch. Slower again before it is lost, the body is steady and
//! screams again the next time.
//!
//! The kill plane is the other way out of the world: a dead body whose
//! root falls below −50 m ([`exa_physics::body::FALL_OUT_HEIGHT`]) loses
//! its health and, but for the player's, is taken out, once.
//!
//! Rules: `docs/formats/RSQ.md` (Falling to death).

use bevy::prelude::*;

/// Centimetres per metre: the rules are the original's, in its
/// centimetres and ticks.
const CM: f32 = 100.0;

/// A chest sinking faster than this (cm a tick, its slow smoothed
/// vertical velocity) is falling.
pub const FALLING_BELOW: f32 = -5.0;
/// A pelvis this high (cm) or more over the ground under the body and
/// under each foot is lost.
pub const LETHAL_DROP: f32 = 2500.0;
/// The player hears the pit, and its screen starts to fade, more than this
/// long (ms of the game clock) after it was lost.
pub const PIT_AFTER: u64 = 1000;
/// The player dies more than this long (ms) after it was lost ...
pub const PLAYER_DIES_AFTER: u64 = 4000;
/// ... any other character more than this long.
pub const DIES_AFTER: u64 = 3000;
/// The scream: gain 0.2 + 0.15 × a draw and pitch 0.95 + 0.1 × the draw
/// before it, in double precision, times the voice's own.
const SCREAM_GAIN: (f64, f64) = (0.2, 0.15);
const SCREAM_PITCH: (f64, f64) = (0.95, 0.1);

/// Metres in the original's centimetres.
fn cm(m: f32) -> f32 {
    m * CM
}

/// How far a fall has gone (the original's five states).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum FallStage {
    /// Not sinking faster than 5 cm a tick.
    #[default]
    Steady,
    /// Sinking faster: it has screamed.
    Falling,
    /// No ground within 25 m under it: it will die.
    Lost,
    /// The player only: the pit sounded and the screen fades.
    Fading,
    /// Dead and out of the world.
    Gone,
}

/// A character's fall, kept from tick to tick (on the body model root,
/// with its [`crate::Fighter`]).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FallDeath {
    pub stage: FallStage,
    /// When it was lost (ms of the game clock).
    pub since: u64,
    /// It has left the world (the original's body flag): once lost, or
    /// through the kill plane, which never takes a body twice.
    pub out: bool,
}

/// What the fall check reads of a living body (metres, its last measure).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FallReading<'a> {
    /// The chest's slow smoothed vertical velocity (m a tick).
    pub sinking: f32,
    /// The pelvis's height.
    pub pelvis: f32,
    /// The ground under the body, measured without the feet's support;
    /// none with it.
    pub ground: Option<f32>,
    /// Each foot's ground, none while out of reach.
    pub feet: &'a [Option<f32>],
}

impl FallReading<'_> {
    /// Whether the pelvis is 25 m or more over the ground under the body
    /// and under every foot (a supported body never is).
    pub fn lethal(&self) -> bool {
        let pelvis = cm(self.pelvis);
        let deep = |g: f32| pelvis - cm(g) >= LETHAL_DROP;
        self.ground.is_some_and(deep) && self.feet.iter().all(|f| f.is_some_and(deep))
    }
}

/// What one fall check did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FallStep {
    /// It screams (a heavy groan of its voice, [`scream_voice`]).
    pub scream: bool,
    /// It is lost.
    pub lost: bool,
    /// The player hears the pit and its screen starts to fade.
    pub pit: bool,
    /// It dies and is taken out of the world.
    pub dies: bool,
}

impl FallDeath {
    /// One fall check, each tick a living body takes the falls branch, at
    /// `now` (ms of the game clock): before it is lost, steady unless its
    /// chest sinks faster than 5 cm a tick, a scream the first tick it
    /// does (not when `dead`), lost when [`FallReading::lethal`]; once
    /// lost, the player's or another's timeline.
    pub fn check(&mut self, r: &FallReading, dead: bool, player: bool, now: u64) -> FallStep {
        let mut step = FallStep::default();
        match self.stage {
            FallStage::Steady | FallStage::Falling => {
                // Not below (a NaN is not either): steady.
                let falling = cm(r.sinking) < FALLING_BELOW;
                if !falling {
                    self.stage = FallStage::Steady;
                    return step;
                }
                if self.stage == FallStage::Steady {
                    step.scream = !dead;
                    self.stage = FallStage::Falling;
                }
                if r.lethal() {
                    self.out = true;
                    self.stage = FallStage::Lost;
                    self.since = now;
                    step.lost = true;
                }
            }
            FallStage::Lost if player => {
                if now > self.since + PIT_AFTER {
                    self.stage = FallStage::Fading;
                    step.pit = true;
                }
            }
            FallStage::Fading if player => {
                if now > self.since + PLAYER_DIES_AFTER {
                    self.die();
                    step.dies = true;
                }
            }
            FallStage::Lost | FallStage::Fading => {
                if now > self.since + DIES_AFTER {
                    self.die();
                    step.dies = true;
                }
            }
            FallStage::Gone => {}
        }
        step
    }

    fn die(&mut self) {
        self.stage = FallStage::Gone;
        self.out = true;
    }

    /// The kill plane, for a dead body whose root fell below −50 m: true
    /// the first time (it takes the body's health and, but for the
    /// player's, the body), false once the body has left the world.
    pub fn fall_out(&mut self) -> bool {
        !std::mem::replace(&mut self.out, true)
    }
}

/// The scream's gain and pitch factors from two draws of the original's
/// generator, in the order it draws them: the first gives the pitch
/// (0.95–1.05), the second the gain (0.2–0.35).
pub fn scream_voice(first: f32, second: f32) -> (f32, f32) {
    let gain = (f64::from(second) * SCREAM_GAIN.1 + SCREAM_GAIN.0) as f32;
    let pitch = (f64::from(first) * SCREAM_PITCH.1 + SCREAM_PITCH.0) as f32;
    (gain, pitch)
}

#[cfg(test)]
#[path = "fall_test.rs"]
mod tests;
