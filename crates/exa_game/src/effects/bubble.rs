//! The teleport bubble: a 120 cm sphere (a polyhedron subdivided three
//! times) drawn around a body stepping through a portal, or a destination
//! shown by the portal screen.
//!
//! The bubble grows from nothing over 1.8 s. Without its hold flag it then
//! rests for 0.5 s, plays `teleportc01` and shrinks away in 0.4 s. With the
//! flag it holds: its charge rises over 7 s, a post-process flash fires at
//! 5.6 s, and at 7 s the bubble is freed. The shader scales the sphere by
//! `1 + 100·s⁴` once the charge passes 0.8 (`s = 5·(charge − 0.8)`) and
//! rolls a texture offset whose period shrinks from 500 to 150 ms.
//!
//! Sizes are the original's centimetres, times milliseconds.

use bevy::prelude::*;

use crate::effects::span_ms;

/// The sphere's radius, centimetres.
pub const SIZE_CM: f32 = 120.0;
/// Growing lasts 1.8 s.
pub const GROW_MS: u32 = 1_800;
/// Its per-millisecond share, the double the original multiplies by.
pub const GROW_PER_MS: f64 = 0.000_555_555_555_555_555_6;
/// A bubble without the hold flag rests this long before it shrinks.
pub const IDLE_MS: u32 = 500;
/// A held bubble charges over 7 s.
pub const HOLD_MS: u32 = 7_000;
/// The post-process flash fires after 5.6 s of the hold.
pub const POST_MS: u32 = 5_600;
/// The shrink runs at 0.0025 a millisecond (a double): 400 ms from rest to
/// gone.
pub const SHRINK_PER_MS: f64 = 0.0025;
/// The charge's rate over the hold: 1/7000.
pub const CHARGE_PER_MS: f64 = 0.000_142_857_142_857_142_87;
/// The charge at which the swell starts.
pub const SWELL_FROM: f64 = 0.8;
/// The swell's curve: `1 + 100·((charge − 0.8)·5)⁴`.
pub const SWELL_GAIN: f32 = 100.0;
/// The swell's shader scale is reproduced, but how the 120 cm geometry and
/// the `Scale` uniform combine is not certain (guess: both apply).
///
/// The texture offset's period falls from 500 ms to 150 ms.
pub const PERIOD_MAX: i64 = 500;
pub const PERIOD_SWING: f32 = 350.0;

/// The bubble's material.
pub const MATERIAL: &str = "_TeleBubble01";

/// The bubble's sound set.
pub const SPAWN_SOUND: &str = "teleportf01";
pub const CLOSE_SOUND: &str = "teleportc01";
pub const SPAWN_VOLUME: f32 = 0.3;
pub const CLOSE_VOLUME: f32 = 0.3;
pub const SPAWN_PITCH: f32 = 1.0;

/// The mesh level of detail: 500 over the squared camera distance, at
/// least one.
pub const LOD_BASE: f32 = 500.0;

/// The mesh level of detail the play renderer stores: 500 over the
/// squared distance (cm) from the play camera, raised to one.
pub fn level_of_detail(camera: Vec3, at: Vec3) -> f32 {
    crate::effects::level_of_detail(LOD_BASE, camera, at)
}

/// The bubble registers a GI emissive light object, refreshed every step
/// and freed with the bubble. Its own values live in that object's class,
/// which is outside this group; only its existence is modelled (guess: the
/// bubble carries no point light of its own).
pub const HAS_GI_LIGHT: bool = true;

/// The phase the bubble is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BubbleState {
    /// Waiting to be removed after a rest.
    Idle = 0,
    /// Growing.
    Grow = 1,
    /// Held, charging.
    Hold = 2,
    /// Held, after the post-process flash.
    Post = 3,
    /// The resting bubble has been told to shrink; the next step starts it.
    ShrinkIn = 4,
    /// Shrinking.
    Shrink = 5,
    /// Freed.
    Gone = 6,
}

/// What one step of a bubble did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BubbleStep {
    pub state: BubbleState,
    /// The grow completed without the hold flag: play `teleportc01`.
    pub close_sound: bool,
    /// The hold reached the post-process point.
    pub post_process: bool,
    /// The bubble is freed this step.
    pub remove: bool,
    /// The spawn sound's volume while shrinking, if it still plays.
    pub sound_volume: Option<f32>,
}

/// A teleport bubble.
#[derive(Debug, Clone, PartialEq)]
pub struct Bubble {
    pub state: BubbleState,
    /// The millisecond this phase started.
    pub stamp: u32,
    /// The phase's value: 0..1 while growing, 0..−1 while shrinking.
    pub fade: f32,
    /// The hold's charge, 0..1.
    pub charge: f32,
    /// Whether the bubble holds instead of disappearing after it grows.
    pub hold: bool,
    /// The shader's texture position, two random values at creation.
    pub tex_pos: [f32; 2],
    /// The spawn sound's volume, 0..1.
    pub sound_volume: f32,
}

impl Bubble {
    /// A bubble created at `now_ms`.
    pub fn new(now_ms: u32, hold: bool, tex_pos: [f32; 2]) -> Self {
        Self {
            state: BubbleState::Grow,
            stamp: now_ms,
            fade: 0.0,
            charge: 0.0,
            hold,
            tex_pos,
            sound_volume: SPAWN_VOLUME,
        }
    }

    /// One step of the phase machine.
    pub fn step(&mut self, now_ms: u32) -> BubbleStep {
        let mut step = BubbleStep {
            state: self.state,
            close_sound: false,
            post_process: false,
            remove: false,
            sound_volume: None,
        };
        match self.state {
            BubbleState::Grow => {
                self.fade = (span_ms(now_ms, self.stamp) as f64 * GROW_PER_MS) as f32;
                if self.fade > 1.0 {
                    self.fade = 1.0;
                    self.stamp = now_ms;
                    if self.hold {
                        self.state = BubbleState::Hold;
                    } else {
                        self.state = BubbleState::Idle;
                        step.close_sound = true;
                    }
                }
            }
            BubbleState::Idle => {
                if passed(self.stamp, IDLE_MS, now_ms) {
                    self.state = BubbleState::ShrinkIn;
                    self.stamp = now_ms;
                }
            }
            BubbleState::ShrinkIn => {
                self.state = BubbleState::Shrink;
                self.fade = 1.0 - self.fade;
                self.stamp = now_ms;
            }
            BubbleState::Hold => {
                self.fade = 1.0;
                self.charge = self.charge_at(now_ms);
                if passed(self.stamp, POST_MS, now_ms) {
                    self.state = BubbleState::Post;
                    step.post_process = true;
                }
            }
            BubbleState::Post => {
                self.charge = self.charge_at(now_ms);
                if passed(self.stamp, HOLD_MS, now_ms) {
                    self.state = BubbleState::Gone;
                    self.fade = 1.0;
                    step.remove = true;
                }
            }
            BubbleState::Shrink => {
                self.fade = ((-span_ms(now_ms, self.stamp)) as f64 * SHRINK_PER_MS) as f32;
                step.sound_volume = Some((self.fade + 1.0) * 0.5);
                self.sound_volume = (self.fade + 1.0) * 0.5;
                if self.fade <= -1.0 {
                    self.state = BubbleState::Gone;
                    self.fade = 1.0;
                    step.remove = true;
                }
            }
            BubbleState::Gone => {}
        }
        step.state = self.state;
        step
    }

    /// The hold's charge: one past the 7 s, else the elapsed share of it.
    fn charge_at(&self, now_ms: u32) -> f32 {
        if passed(self.stamp, HOLD_MS, now_ms) {
            1.0
        } else {
            (span_ms(now_ms, self.stamp) as f64 * CHARGE_PER_MS) as f32
        }
    }

    /// The shader's uniforms from the current phase.
    ///
    /// Before the shrink the fade is `max(1 − value, 0)` (the processor's
    /// maximum: a NaN gives zero). While shrinking the original computes a
    /// `√(x·(2 − x))` curve of the value but uploads the value itself, so
    /// the shader's fade runs from 0 to −1.
    pub fn uniforms(&self, now_ms: u32) -> BubbleUniforms {
        let fade = if self.state.word() < 5 {
            let left = 1.0 - self.fade;
            if left > 0.0 { left } else { 0.0 }
        } else {
            self.fade
        };
        let charge = f64::from(self.charge);
        let (scale, colour) = if charge > SWELL_FROM {
            let swell = ((charge - SWELL_FROM) * 5.0) as f32;
            let square = swell * swell;
            (square * square * SWELL_GAIN + 1.0, 1.0 - swell)
        } else {
            (1.0, 1.0)
        };
        // The period is an integer: 500 less the rounded swing. The
        // original divides by it as a 32-bit word; a charge far out of range
        // could make it zero, which would fault there and is left at zero
        // offset here.
        let period = PERIOD_MAX
            - i64::from(crate::effects::round_i32(
                PERIOD_SWING * ((2.0 - self.charge) * self.charge),
            ));
        let period = period as u32;
        let tex_offset = if period == 0 {
            0.0
        } else {
            let span = span_ms(now_ms, self.stamp) % i64::from(period);
            span as f64 / f64::from(period)
        };
        BubbleUniforms {
            fade,
            scale,
            colour,
            pulse: self.charge,
            tex_pos: self.tex_pos,
            tex_offset: tex_offset as f32,
        }
    }
}

/// The bubble shader's per-frame values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BubbleUniforms {
    pub fade: f32,
    pub scale: f32,
    /// The shader's colour scale.
    pub colour: f32,
    pub pulse: f32,
    pub tex_pos: [f32; 2],
    pub tex_offset: f32,
}

impl BubbleState {
    pub fn word(self) -> i32 {
        self as i32
    }
}

/// Whether `ms` have passed since `stamp`: the sum is taken in 64 bits, so
/// it does not wrap.
fn passed(stamp: u32, ms: u32, now_ms: u32) -> bool {
    u64::from(stamp) + u64::from(ms) < u64::from(now_ms)
}

/// The bubble component stepped by [`tick_bubbles`] (a journey's bubble is
/// stepped by its journey, [`crate::destination::TravelJourney`]).
#[derive(Component, Debug, Clone, PartialEq)]
pub struct BubbleEffect(pub Bubble);

pub fn tick_bubbles(
    mut commands: Commands,
    clock: Res<crate::effects::EffectClock>,
    mut bubbles: Query<(Entity, &mut BubbleEffect), Without<crate::destination::TravelJourney>>,
) {
    for (entity, mut bubble) in &mut bubbles {
        let step = bubble.0.step(clock.now_ms);
        if step.remove {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_bubble_grows_rests_and_shrinks() {
        let mut bubble = Bubble::new(0, false, [0.25, 0.5]);
        let step = bubble.step(0);
        assert_eq!(step.state, BubbleState::Grow);
        bubble.step(900);
        assert_eq!(bubble.fade, 0.5);
        bubble.step(1_800);
        assert_eq!(bubble.fade, 1.0);
        assert_eq!(bubble.state, BubbleState::Grow);
        let step = bubble.step(1_801);
        assert!(step.close_sound);
        assert_eq!(step.state, BubbleState::Idle);
        assert_eq!(bubble.stamp, 1_801);
        bubble.step(2_301);
        assert_eq!(bubble.state, BubbleState::Idle);
        let step = bubble.step(2_302);
        assert_eq!(step.state, BubbleState::ShrinkIn);
        let step = bubble.step(2_303);
        assert_eq!(step.state, BubbleState::Shrink);
        assert_eq!(bubble.fade, 0.0);
        let step = bubble.step(2_703);
        assert_eq!(bubble.fade, 1.0);
        assert!(step.remove);
        assert_eq!(step.state, BubbleState::Gone);
        assert_eq!(bubble.fade, 1.0);
    }

    #[test]
    fn a_held_bubble_charges_and_flashes_then_is_freed() {
        let mut bubble = Bubble::new(0, true, [0.0, 0.0]);
        let step = bubble.step(1_801);
        assert!(!step.close_sound);
        assert_eq!(step.state, BubbleState::Hold);
        assert_eq!(bubble.stamp, 1_801);
        // 5.6 s of hold: the post-process fires strictly after it.
        let step = bubble.step(1_801 + 5_600);
        assert!(!step.post_process);
        assert_eq!(step.state, BubbleState::Hold);
        let step = bubble.step(1_801 + 5_601);
        assert!(step.post_process);
        assert_eq!(step.state, BubbleState::Post);
        assert_eq!(bubble.charge, (5_601.0 * CHARGE_PER_MS) as f32);
        let step = bubble.step(1_801 + 7_000);
        assert!(!step.remove);
        assert_eq!(bubble.charge, 1.0);
        let step = bubble.step(1_801 + 7_001);
        assert!(step.remove);
        assert_eq!(step.state, BubbleState::Gone);
    }

    #[test]
    fn a_bubble_stamped_ahead_grows_from_below_zero_and_never_wraps() {
        let mut bubble = Bubble::new(2_000, false, [0.0, 0.0]);
        bubble.step(1_100);
        assert_eq!(bubble.fade, (-900.0 * GROW_PER_MS) as f32);
        // A stamp near the end of the word does not wrap its deadline.
        let mut idle = Bubble::new(0, false, [0.0, 0.0]);
        idle.state = BubbleState::Idle;
        idle.stamp = u32::MAX - 100;
        idle.step(500);
        assert_eq!(idle.state, BubbleState::Idle);
        // A grown bubble without its hold plays the close sound but its
        // spawn sound keeps its volume.
        let mut plain = Bubble::new(0, false, [0.0, 0.0]);
        assert!(plain.step(1_801).close_sound);
        assert_eq!(plain.sound_volume, SPAWN_VOLUME);
    }

    #[test]
    fn the_texture_period_is_a_signed_remainder() {
        let mut bubble = Bubble::new(1_000, true, [0.0, 0.0]);
        // 100 ms before the stamp: the remainder keeps the sign.
        assert!((bubble.uniforms(900).tex_offset + 0.2).abs() < 1e-6);
        bubble.charge = 0.5;
        // 500 − round(350 · 0.75) = 238.
        let u = bubble.uniforms(1_000 + 238 + 119);
        assert!((u.tex_offset - 0.5).abs() < 1e-6, "{}", u.tex_offset);
    }

    #[test]
    fn the_shrink_lowers_the_spawn_sound() {
        let mut bubble = Bubble::new(0, false, [0.0, 0.0]);
        bubble.state = BubbleState::Shrink;
        bubble.stamp = 1_000;
        bubble.fade = 0.0;
        let step = bubble.step(1_200);
        assert_eq!(step.sound_volume, Some(0.25));
        assert_eq!(bubble.fade, -0.5);
        let step = bubble.step(1_400);
        assert_eq!(step.sound_volume, Some(0.0));
        assert!(step.remove);
    }

    #[test]
    fn the_swell_starts_past_eighty_percent_charge() {
        let mut bubble = Bubble::new(0, true, [0.0, 0.0]);
        bubble.state = BubbleState::Hold;
        bubble.charge = 0.8;
        let u = bubble.uniforms(0);
        assert_eq!(u.scale, 1.0);
        // A stored 0.8 widens to 0.80000001, just above the double bound, so
        // the swell branch already runs and the colour drops by one step.
        assert!((u.colour - 0.999_999_94).abs() < 1e-6, "{}", u.colour);
        bubble.charge = 0.81;
        let u = bubble.uniforms(0);
        assert!((u.scale - 1.000_625).abs() < 1e-5, "{}", u.scale);
        assert!((u.colour - 0.95).abs() < 1e-6);
        bubble.charge = 1.0;
        let u = bubble.uniforms(0);
        assert_eq!(u.scale, 101.0);
        assert_eq!(u.colour, 0.0);
        assert_eq!(u.pulse, 1.0);
    }

    #[test]
    fn the_texture_offset_rolls_faster_as_the_charge_rises() {
        let mut bubble = Bubble::new(0, true, [0.125, 0.25]);
        bubble.state = BubbleState::Grow;
        bubble.fade = 0.0;
        bubble.charge = 0.0;
        bubble.stamp = 0;
        let u = bubble.uniforms(100);
        assert_eq!(u.tex_pos, [0.125, 0.25]);
        assert_eq!(u.fade, 1.0);
        assert!((u.tex_offset - 0.2).abs() < 1e-6);
        bubble.charge = 1.0;
        let u = bubble.uniforms(100);
        assert!((u.tex_offset - (100.0 / 150.0)).abs() < 1e-6);
        // Shrinking, the value itself is uploaded.
        bubble.state = BubbleState::Shrink;
        bubble.fade = -0.5;
        assert_eq!(bubble.uniforms(100).fade, -0.5);
        bubble.fade = -1.0;
        assert_eq!(bubble.uniforms(100).fade, -1.0);
        // Before it, a NaN value gives the maximum's second operand.
        bubble.state = BubbleState::Idle;
        bubble.fade = f32::NAN;
        assert_eq!(bubble.uniforms(100).fade, 0.0);
        bubble.fade = 1.5;
        assert_eq!(bubble.uniforms(100).fade, 0.0);
        assert_eq!(
            (GROW_MS, IDLE_MS, HOLD_MS, POST_MS),
            (1_800, 500, 7_000, 5_600)
        );
        assert_eq!(SPAWN_SOUND, "teleportf01");
        assert_eq!(CLOSE_SOUND, "teleportc01");
        assert_eq!(SPAWN_VOLUME, 0.3);
        assert_eq!(CLOSE_VOLUME, 0.3);
        assert_eq!(SPAWN_PITCH, 1.0);
        assert_eq!(SIZE_CM, 120.0);
        assert_eq!(MATERIAL, "_TeleBubble01");
        assert_eq!(level_of_detail(Vec3::ZERO, Vec3::new(0.5, 0.0, 0.0)), 500.0);
        assert_eq!(level_of_detail(Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0)), 125.0);
    }
}
