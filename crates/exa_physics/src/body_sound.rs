//! The rigid body's own collision sounds: the impact sound of a plain
//! tetrahedral body and the scrape/roll sources that follow a point on it.
//!
//! A plain body's parts are not limbs, so their level and pair contacts use
//! the body's own volume and pitch curve instead of a limb's hit sound: the
//! impact is scaled by a hundredth, clamped at one, and the part's mobility
//! lifts either the volume (a light part) or only the pitch (a heavy one,
//! at least 0.1). The cooldown is 220 ms less a rounded share of the
//! volume, plus a 0–99 ms draw from the engine's own generator. A later
//! impact sounds again once the clock passes the cooldown, or once it is
//! strictly louder than twice the last one.
//!
//! A scrape source follows a point fixed on the body: each tick the world
//! point, put there by the matrix the last tick recorded, is compared with
//! the one before, its squared move smoothed into a speed, and the source plays while the speed is over 0.02, its volume and
//! pitch following the speed. The original keeps these sources on the body
//! (one per stored sound slot) and starts them when the body joins the
//! world; the events are returned to the audio side, which plays the
//! impacts as one-shots and the scrapes as attached loops.

use bevy::math::Vec3;

use super::Part;
use crate::part::base_impact_sound;

/// The engine's generator step: `s = s · 0x08088405 + 1`.
const GENERATOR_STEP: u32 = 0x0808_8405;

/// One impact sound to play.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactEvent {
    /// The volume, 0 to 1.5 (the original's own gain follows).
    pub volume: f32,
    /// The source's pitch.
    pub pitch: f32,
    /// The game clock after which the next impact may sound.
    pub cooldown_ms: u32,
}

/// A plain body part's impact sound, as its collision callback decides it,
/// with the part's own sound clock and last threshold
/// ([`Part::sound_ready_ms`], [`Part::last_impact`]).
///
/// `strength` is the contact's impact strength, `sounded` whether the part
/// has a collision sound, `now_ms` the game clock, and `roll` the engine's
/// generator state, advanced only when a sound plays. The gate and the
/// volume and pitch are [`base_impact_sound`]'s (the part's contact
/// mobility). A part without a sound that passes the gate parks its clock
/// at the all-ones word and stays silent, as the original does. A sound
/// stores twice the strength as the threshold and sets the clock to
/// `now + 220 − round(170·v) + draw`, the draw `(s·100) >> 32` of the
/// stepped generator (0 to 99), taken after the sound plays.
pub fn impact_sound(
    part: &mut Part,
    sounded: bool,
    strength: f32,
    now_ms: u32,
    roll: &mut u32,
) -> Option<ImpactEvent> {
    let sound = base_impact_sound(
        strength,
        part.pairing.mobility,
        part.sound_ready_ms,
        part.last_impact,
        now_ms,
    )?;
    if !sounded {
        part.sound_ready_ms = u32::MAX;
        return None;
    }
    part.last_impact = sound.threshold;
    *roll = roll.wrapping_mul(GENERATOR_STEP).wrapping_add(1);
    let draw = ((u64::from(*roll) * 100) >> 32) as u32;
    let cooldown_ms = now_ms.wrapping_add(sound.cooldown_ms).wrapping_add(draw);
    part.sound_ready_ms = cooldown_ms;
    Some(ImpactEvent {
        volume: sound.volume,
        pitch: sound.pitch,
        cooldown_ms,
    })
}

/// A scrape or roll sound source on a body: a point that follows the body,
/// and the speed with which the point moves through the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrapeSource {
    /// The part whose frame holds the point (0 for a body's root).
    pub part: usize,
    /// The point on the part, in its own space (metres).
    pub point: Vec3,
    /// The volume per unit of speed.
    pub volume: f32,
    /// The pitch per unit of speed, over the source's base.
    pub pitch: f32,
    /// The smoothed, capped move measure (from the point's squared move in
    /// cm² a tick).
    pub speed: f32,
    /// Where the point was when the source last looked.
    pub last: Vec3,
    /// Whether the source is playing.
    pub playing: bool,
}

impl ScrapeSource {
    /// A source on part `part` at `point` (its space) whose first world
    /// position is `world`.
    pub fn new(part: usize, point: Vec3, volume: f32, pitch: f32, world: Vec3) -> Self {
        Self {
            part,
            point,
            volume,
            pitch,
            speed: 0.0,
            last: world,
            playing: false,
        }
    }
}

/// What one scrape tick reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrapeEvent {
    /// The source was not playing and starts now.
    pub started: bool,
    /// The source was playing and stops now.
    pub stopped: bool,
    /// Whether it plays after this tick.
    pub playing: bool,
    /// The volume to play at (the speed times the source's volume).
    pub volume: f32,
    /// The pitch to play at (`pitch · (0.1·speed + 0.9)`).
    pub pitch: f32,
    /// Where the source sits (its part's world position, from the last recorded matrix).
    pub point: Vec3,
}

impl ScrapeSource {
    /// One tick at `world` (where the followed point is now), with
    /// `origin` the world position of the part carrying the point, where the
    /// playing source sits.
    ///
    /// The move is measured as the squared distance in centimetres (cm²,
    /// not cm: the original's distance helper does not take the root). A
    /// move over 0.01 cm² becomes `(√d · 2 + d) · 0.05`, at most 1, and the
    /// speed is a fifth of the way to it over four fifths of the old speed.
    /// A speed not above 0.02 (or NaN) stops a playing source; otherwise a
    /// stopped source starts, and the volume and pitch follow the speed.
    /// The thresholds, the 0.05 scale, the smoothing and the pitch are
    /// worked in double precision and narrowed, as the original does.
    pub fn step(&mut self, world: Vec3, origin: Vec3) -> ScrapeEvent {
        // The body works in metres; the original's threshold and curve are
        // in its centimetres.
        let mut moved = ((world - self.last) * 100.0).length_squared();
        self.last = world;
        if f64::from(moved) > 0.01 {
            moved = (f64::from(moved.sqrt() * 2.0 + moved) * 0.05) as f32;
            if moved > 1.0 {
                moved = 1.0;
            }
        }
        self.speed = (f64::from(moved) * 0.2 + f64::from(self.speed) * 0.8) as f32;
        // Not above 0.02 (or NaN, the unordered compare) stops the source.
        if f64::from(self.speed) > 0.02 {
            let started = !self.playing;
            self.playing = true;
            ScrapeEvent {
                started,
                stopped: false,
                playing: true,
                volume: self.speed * self.volume,
                pitch: (f64::from(self.pitch) * (f64::from(self.speed) * 0.1 + 0.9)) as f32,
                point: origin,
            }
        } else {
            let stopped = self.playing;
            self.playing = false;
            ScrapeEvent {
                started: false,
                stopped,
                playing: false,
                volume: 0.0,
                pitch: 0.0,
                point: origin,
            }
        }
    }
}

#[cfg(test)]
#[path = "body_sound_test.rs"]
mod tests;
