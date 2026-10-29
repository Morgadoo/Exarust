//! A cloth's parameter record: the `3DD0C001`-tagged block the cloth's
//! set-up copies its integration, contact and pass settings from.
//!
//! The cloth constructor configures every new cloth from its node's cloth
//! marker record when the node carries one, else from the engine's built-in
//! record ([`ClothRecord::LEVEL`]: every shipped marker is the bare tag, so
//! the level's banners and the wraith's robe all take it). The character
//! build gives a hair style's cloth a second built-in record
//! ([`ClothRecord::HAIR`]). The set-up copies, from the record:
//!
//! - the mode word (record word 4): its `0xF00` nibble picks the constraint
//!   scheme. `0x100` is the *particle* mode: every constraint pass runs
//!   inside the step, the body contact pushes particles straight off body
//!   spheres ([`crate::cloth_body::touch_particles`]) and the post-body push
//!   pass does nothing. Any other value is the *triangle* mode: one pass in
//!   the step, the body contact pushes triangles into a buffer and the
//!   post-body pass applies it with the remaining passes;
//! - the level slide (word 6, cm), the per-second velocity retention (word 7,
//!   taken to one tick as `exp(ln(r) · 1/83)` in double precision), the drag
//!   toward the mean velocity (word 8), the contact padding (word 9, cm), the
//!   gravity scale (word 11), the constraint pass count (word 12) and the
//!   rustle sound's name (from byte 0x48).
//!
//! Evidence: the cloth constructor's record choice, the set-up routine's
//! field copies, the integrate slot's gravity, retention and pass loop, the
//! post-body pass's mode gate, the body-contact slot's mode switch, the
//! level contact's padding and slide reads, and the two records' bytes in
//! the executable (REA assembly). The set-up's other words (the `0x7000`
//! bits, which raise a class flag, the `0x4` bit, which swaps the draw
//! callbacks, and word 10) are kept in [`ClothRecord::flags`] but read by
//! nothing here.

use bevy::math::Vec3;

/// The constraint scheme in a record's mode word.
pub const MODE_MASK: u32 = 0xF00;
/// The particle mode's value in [`MODE_MASK`].
pub const PARTICLE_MODE: u32 = 0x100;

/// The world's fast velocity retention, which every cloth's per-tick
/// retention is multiplied by.
pub const FAST_RETENTION: f32 = 0.98977;

/// One over the body tick rate, as the set-up's double constant.
const TICK_SECONDS: f64 = 1.0 / 83.0;

/// A cloth parameter record, as the cloth's set-up reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClothRecord {
    /// The mode word: [`MODE_MASK`] picks the constraint scheme.
    pub flags: u32,
    /// How far (cm) a particle a level mesh pushed has its travel shortened.
    pub level_slide_cm: f32,
    /// The share of a particle's velocity kept over one second.
    pub retention_per_second: f32,
    /// Drag toward the cloth's mean velocity.
    pub mean_drag: f32,
    /// Contact padding (cm): the radius a particle meets the level with and
    /// what the triangle mode adds to a body sphere.
    pub padding_cm: f32,
    /// The factor on the world's gravity.
    pub gravity_scale: f32,
    /// The constraint pass count.
    pub passes: i32,
    /// The rustle sound's name, if the record names one.
    pub rustle: Option<&'static str>,
}

impl ClothRecord {
    /// The engine's built-in record, which every shipped cloth marker
    /// falls back to: triangle mode (`0x7208`), 0.1 cm slide, half the
    /// velocity kept a second, 0.05 drag, 2 cm padding, gravity 1, four
    /// passes and the `CltMov` rustle.
    pub const LEVEL: Self = Self {
        flags: 0x7208,
        level_slide_cm: 0.1,
        retention_per_second: 0.5,
        mean_drag: 0.05,
        padding_cm: 2.0,
        gravity_scale: 1.0,
        passes: 4,
        rustle: Some("CltMov"),
    };

    /// The record the character build hands a hair style's cloth: particle
    /// mode (`0x104`), no slide, half the velocity kept a second, 0.2 drag,
    /// no padding, gravity 1, two passes and no rustle.
    pub const HAIR: Self = Self {
        flags: 0x104,
        level_slide_cm: 0.0,
        retention_per_second: 0.5,
        mean_drag: 0.2,
        padding_cm: 0.0,
        gravity_scale: 1.0,
        passes: 2,
        rustle: None,
    };

    /// Whether the record's constraint scheme is the particle mode.
    pub fn particle_mode(&self) -> bool {
        self.flags & MODE_MASK == PARTICLE_MODE
    }

    /// The constraint passes the step itself makes: all of them in the
    /// particle mode, else one.
    pub fn step_passes(&self) -> usize {
        if self.particle_mode() {
            usize::try_from(self.passes).unwrap_or(0)
        } else {
            1
        }
    }

    /// The constraint passes after the body push in the triangle mode: the
    /// pass count less the step's one (none in the particle mode, whose
    /// post-body pass returns at once).
    pub fn post_body_passes(&self) -> usize {
        if self.particle_mode() {
            0
        } else {
            usize::try_from(self.passes - 1).unwrap_or(0)
        }
    }

    /// The per-tick retention: [`FAST_RETENTION`] times the per-second
    /// retention taken to one tick (`exp(ln(r) · 1/83)` in double, stored
    /// as a float).
    pub fn retention(&self) -> f32 {
        let per_tick = (f64::from(self.retention_per_second).ln() * TICK_SECONDS).exp() as f32;
        FAST_RETENTION * per_tick
    }

    /// The world's gravity (cm per tick) scaled by the record.
    pub fn gravity(&self, world: Vec3) -> Vec3 {
        world * self.gravity_scale
    }
}

impl Default for ClothRecord {
    fn default() -> Self {
        Self::LEVEL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two built-in records' words, as the executable stores them.
    #[test]
    fn the_built_in_records_hold_the_original_words() {
        assert_eq!(ClothRecord::LEVEL.flags, 0x7208);
        assert_eq!(ClothRecord::LEVEL.passes, 4);
        assert_eq!(ClothRecord::LEVEL.padding_cm, 2.0);
        assert_eq!(ClothRecord::LEVEL.mean_drag, 0.05);
        assert_eq!(ClothRecord::HAIR.flags, 0x104);
        assert_eq!(ClothRecord::HAIR.passes, 2);
        assert_eq!(ClothRecord::HAIR.padding_cm, 0.0);
        assert_eq!(ClothRecord::HAIR.mean_drag, 0.2);
        assert_eq!(ClothRecord::HAIR.level_slide_cm, 0.0);
        assert!(ClothRecord::HAIR.rustle.is_none());
    }

    /// The level record makes one pass in the step and three after the
    /// body push; the hair's particle mode makes both its passes in the step
    /// and none after.
    #[test]
    fn the_mode_splits_the_passes() {
        assert!(!ClothRecord::LEVEL.particle_mode());
        assert_eq!(ClothRecord::LEVEL.step_passes(), 1);
        assert_eq!(ClothRecord::LEVEL.post_body_passes(), 3);
        assert!(ClothRecord::HAIR.particle_mode());
        assert_eq!(ClothRecord::HAIR.step_passes(), 2);
        assert_eq!(ClothRecord::HAIR.post_body_passes(), 0);
        // A zero or negative pass count makes no passes.
        let none = ClothRecord {
            passes: 0,
            ..ClothRecord::HAIR
        };
        assert_eq!(none.step_passes(), 0);
        let one = ClothRecord {
            passes: 0,
            ..ClothRecord::LEVEL
        };
        assert_eq!(one.post_body_passes(), 0);
    }

    /// Half a second's retention is 0.5^(1/83) a tick, times the fast
    /// retention: about 0.98153.
    #[test]
    fn the_retention_is_taken_to_one_tick_in_double() {
        let keep = ClothRecord::LEVEL.retention();
        let expected = FAST_RETENTION * ((0.5_f64.ln() * (1.0 / 83.0)).exp() as f32);
        assert_eq!(keep, expected);
        assert!((keep - 0.981_53).abs() < 1e-5);
        assert_eq!(ClothRecord::HAIR.retention(), keep);
    }
}
