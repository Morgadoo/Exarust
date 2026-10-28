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
//! point is compared with the one before, the distance smoothed into a
//! speed, and the source plays while the speed is over 0.02, its volume and
//! pitch following the speed. The original keeps these sources on the body
//! (one per stored sound slot) and starts them when the body joins the
//! world; Exarust's audio crate has no body hookup yet, so the events are
//! returned to a caller that can play them.

use bevy::math::Vec3;

/// The engine's generator step: `s = s · 0x08088405 + 1`.
const GENERATOR_STEP: u32 = 0x0808_8405;

/// A plain body part's impact-sound state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactState {
    /// The game clock (ms) after which the next impact may sound. The
    /// all-ones word means the part has no collision sound.
    pub cooldown_ms: u32,
    /// Twice the last impact's strength: a louder impact sounds before the
    /// cooldown passes.
    pub threshold: f32,
}

impl ImpactState {
    /// Fresh state. A part with a collision sound starts ready; one with
    /// `sounded: false` silences itself on its first impact and stays
    /// silent.
    pub fn new(sounded: bool) -> Self {
        Self {
            cooldown_ms: if sounded { 0 } else { u32::MAX },
            threshold: 0.0,
        }
    }
}

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

/// A plain body's impact sound, as its collision callback decides it.
///
/// `strength` is the contact's impact strength, `mobility` the part's own
/// mobility, `now_ms` the game clock, and `roll` the engine's generator
/// state, advanced only when a sound plays. A strength of 0.5 or less is
/// silent; a later impact sounds when the clock has passed the cooldown, or
/// when it is strictly above the stored threshold. A part without a sound
/// material takes the silent branch and parks its cooldown at the all-ones
/// word.
///
/// The volume is `min(strength · 0.01, 1) · 1.5`; a mobility of at least
/// 0.1 (or NaN, which the unordered compare sends the same way) scales the
/// pitch by `mobility · 0.02 + 0.9`, and a lighter one scales both by
/// `mobility · 10`: the volume by `0.8·mobility·10 + 0.2` and the pitch by
/// `0.7·mobility·10 + 0.3`. The products with the pitch base
/// `min(strength · 0.01, 1) · 0.6 + 0.7` are taken in double precision and
/// narrowed once, as the original does. The cooldown is
/// `now + 220 − round(base · 170) + draw`, the draw 0 to 99 from the
/// generator and the rounding half to even.
pub fn impact_sound(
    state: &mut ImpactState,
    strength: f32,
    mobility: f32,
    now_ms: u32,
    roll: &mut u32,
) -> Option<ImpactEvent> {
    if !matches!(
        strength.partial_cmp(&0.5),
        Some(std::cmp::Ordering::Greater)
    ) {
        return None;
    }
    if state.cooldown_ms >= now_ms && !(state.threshold <= strength && strength != state.threshold)
    {
        return None;
    }
    if state.cooldown_ms == u32::MAX {
        return None;
    }
    let base = minimum((f64::from(strength) * 0.01) as f32, 1.0);
    let pitch_base = (f64::from(base) * 0.6 + 0.7) as f32;
    let mut volume = base * 1.5;
    let pitch = if f64::from(mobility) >= 0.1 || mobility.is_nan() {
        ((f64::from(mobility) * 0.02 + 0.9) * f64::from(pitch_base)) as f32
    } else {
        let scaled = f64::from(mobility * 10.0);
        volume = ((scaled * 0.8 + 0.2) * f64::from(volume)) as f32;
        ((scaled * 0.7 + 0.3) * f64::from(pitch_base)) as f32
    };
    state.threshold = strength * 2.0;
    *roll = roll.wrapping_mul(GENERATOR_STEP).wrapping_add(1);
    let draw = ((u64::from(*roll) * 100) >> 32) as u32;
    let cooldown_ms = now_ms
        .wrapping_add(220)
        .wrapping_sub((base * 170.0).round_ties_even() as u32)
        .wrapping_add(draw);
    state.cooldown_ms = cooldown_ms;
    Some(ImpactEvent {
        volume,
        pitch,
        cooldown_ms,
    })
}

/// A scrape or roll sound source on a body: a point that follows the body,
/// and the speed with which the point moves through the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrapeSource {
    /// The point on the body, in its own space (metres).
    pub point: Vec3,
    /// The volume per unit of speed.
    pub volume: f32,
    /// The pitch per unit of speed, over the source's base.
    pub pitch: f32,
    /// The smoothed speed, in the original's units (a fraction of the
    /// body's size a tick).
    pub speed: f32,
    /// Where the point was when the source last looked.
    pub last: Vec3,
    /// Whether the source is playing.
    pub playing: bool,
}

impl ScrapeSource {
    /// A source at `point` (the body's space) whose first world position is
    /// `world`.
    pub fn new(point: Vec3, volume: f32, pitch: f32, world: Vec3) -> Self {
        Self {
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
    /// Where the source sits (its body's world position).
    pub point: Vec3,
}

impl ScrapeSource {
    /// One tick at `world` (where the followed point is now), with
    /// `origin` the body's world position that the playing source sits at.
    ///
    /// The point's distance is smoothed: a move over 0.01 becomes
    /// `(√d · 2 + d) · 0.05`, at most 1, and the speed is a fifth of the
    /// way to it over four fifths of the old speed. A speed of 0.02 or less
    /// stops a playing source; otherwise a stopped source starts, and the
    /// volume and pitch follow the speed.
    pub fn step(&mut self, world: Vec3, origin: Vec3) -> ScrapeEvent {
        // The body works in metres; the original's threshold and curve are
        // in its centimetres.
        let mut distance = self.last.distance(world) * 100.0;
        self.last = world;
        if distance > 0.01 {
            distance = (distance.sqrt() * 2.0 + distance) * 0.05;
            if distance > 1.0 {
                distance = 1.0;
            }
        }
        self.speed = distance * 0.2 + self.speed * 0.8;
        if self.speed <= 0.02 {
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
        } else {
            let started = !self.playing;
            self.playing = true;
            ScrapeEvent {
                started,
                stopped: false,
                playing: true,
                volume: self.speed * self.volume,
                pitch: self.pitch * (self.speed * 0.1 + 0.9),
                point: origin,
            }
        }
    }
}

/// The processor's `MINSS`: `a` when it is the smaller, else `b` (so `b`
/// on NaN).
fn minimum(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

#[cfg(test)]
#[path = "body_sound_test.rs"]
mod tests;
