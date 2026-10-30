//! The six environment presets the original's environment unit builds at
//! start-up (`RF_Environment`): a 0xDC-byte record per preset, the colour
//! conversion to linear light and the 0.75 scale when linear lighting is on,
//! and the 25 % blend that fills an empty ambient colour.
//!
//! The records themselves come from the level/environment data in Exarust;
//! this module reproduces the rules the unit applies to them. A preset is
//! kept as the original's 55 f32 words, so the fields the rules touch stay
//! at their file offsets. Games with linear lighting convert these colours
//! once at start-up; a record whose ambient word is zero gets a quarter of
//! the way from its first to its second sky colour.

use exa_assets::vertex_cache::{fast_exp, fast_ln};

/// A preset's size in bytes.
pub const PRESET_BYTES: usize = 0xdc;
/// A preset's size in `f32` words.
pub const PRESET_WORDS: usize = PRESET_BYTES / 4;
/// How many presets the unit prepares.
pub const PRESET_COUNT: usize = 6;
/// The word offsets of the colours that are linearised (bytes / 4).
pub const COLOUR_WORDS: [usize; 12] = [1, 4, 8, 11, 20, 23, 26, 29, 32, 36, 40, 43];
/// The word offset of the empty ambient colour.
pub const EMPTY_WORD: usize = 47;
/// The words scaled by [`LINEAR_SCALE`]: offsets 0x50, 0x5c, 0x68, 0x74 and
/// 0x80.
pub const SCALED_WORDS: [usize; 5] = [20, 23, 26, 29, 32];
/// The first sky colour the empty one blends from (offset 0x50).
pub const EMPTY_FROM: usize = 20;
/// The second sky colour the empty one blends towards (offset 0x74).
pub const EMPTY_TO: usize = 29;
/// The linear-lighting scale.
pub const LINEAR_SCALE: f32 = 0.75;
/// The empty colour's blend towards the second sky colour.
pub const EMPTY_BLEND: f32 = 0.25;
/// The sRGB curve's knee.
pub const SRGB_KNEE: f64 = 0.040_45;
/// The sRGB curve's linear slope.
pub const SRGB_SLOPE: f64 = 12.92;
/// The sRGB curve's offset.
pub const SRGB_OFFSET: f64 = 0.055;
/// The sRGB curve's divisor.
pub const SRGB_DIVISOR: f64 = 1.055;
/// The sRGB curve's exponent.
pub const SRGB_EXPONENT: f32 = 2.4;

/// One sRGB channel to linear light. At or below the knee (compared in
/// double) the channel is divided by 12.92 in double; above it, or when it
/// is not a number, `((x + 0.055) / 1.055)` is worked out in double, rounded
/// to `f32`, and raised to 2.4 with the engine's fast logarithm and
/// exponential (`exp(2.4 * ln t)`), so the result is close to the exact
/// curve, not equal to it: 1.0 comes out as 1.0000396.
pub fn srgb_to_linear(c: f32) -> f32 {
    let x = f64::from(c);
    // Not "above the knee": a NaN takes the power branch too.
    if x <= SRGB_KNEE {
        return (x / SRGB_SLOPE) as f32;
    }
    let t = ((x + SRGB_OFFSET) / SRGB_DIVISOR) as f32;
    fast_exp(fast_ln(t) * SRGB_EXPONENT)
}

/// Linearises the three channels at `word` in place.
fn linearise_at(preset: &mut [f32; PRESET_WORDS], at: usize) {
    for c in 0..3 {
        preset[at + c] = srgb_to_linear(preset[at + c]);
    }
}

/// Scales the three channels at `word` by `scale` in place.
fn scale_at(preset: &mut [f32; PRESET_WORDS], at: usize, scale: f32) {
    for c in 0..3 {
        preset[at + c] *= scale;
    }
}

/// Applies the linear-lighting preparation to one preset: twelve colours
/// linearised, five of them scaled by 0.75, and the ambient colour
/// linearised and scaled too when it is above zero.
pub fn linearise(preset: &mut [f32; PRESET_WORDS]) {
    for &at in &COLOUR_WORDS {
        linearise_at(preset, at);
    }
    for &at in &SCALED_WORDS {
        scale_at(preset, at, LINEAR_SCALE);
    }
    if 0.0 < preset[EMPTY_WORD] {
        linearise_at(preset, EMPTY_WORD);
        scale_at(preset, EMPTY_WORD, LINEAR_SCALE);
    }
}

/// Fills a preset's empty ambient colour with 25 % of the way from the
/// first sky colour (offset 0x50) to the second (offset 0x74). Only the
/// first channel is tested, as the original tests it, and only when it
/// equals zero (a NaN does not).
pub fn fill_empty_colour(preset: &mut [f32; PRESET_WORDS]) {
    // Only an exactly-zero colour is filled: a NaN is left alone.
    if preset[EMPTY_WORD] != 0.0 || preset[EMPTY_WORD].is_nan() {
        return;
    }
    for c in 0..3 {
        let a = preset[EMPTY_FROM + c];
        let b = preset[EMPTY_TO + c];
        preset[EMPTY_WORD + c] = a + (b - a) * EMPTY_BLEND;
    }
}

/// The environment unit's scene-node constructor hook. Initialising the
/// unit saves the hook that was installed before it and installs the
/// environment's attach as the constructor for later environment nodes.
/// The ids are Exarust's markers, not machine addresses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConstructorHook {
    /// The hook that was installed before the environment's own.
    pub previous: Option<u8>,
    /// The hook that runs now.
    pub current: Option<u8>,
}

/// The environment unit's constructor marker.
pub const ENVIRONMENT_CONSTRUCTOR: u8 = 1;

impl ConstructorHook {
    /// Installs the environment constructor over whatever was there, as the
    /// unit's init does (`new = old; old = environment` in the original's
    /// global pair, the new one chaining to the old).
    pub fn install_environment(&mut self) {
        self.previous = self.current;
        self.current = Some(ENVIRONMENT_CONSTRUCTOR);
    }
}

/// What one environment attach does, in the original's order: the previous
/// constructor runs when one was installed, then a node of the environment
/// class is made, linked to the scene and given the scene as its owner, and
/// the scene's node callback (registered for this class) runs last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnvironmentAttach {
    /// The previous constructor ran first.
    pub ran_previous: bool,
    /// The scene's node callback ran last.
    pub ran_callback: bool,
}

impl EnvironmentAttach {
    /// Plans an attach: `previous` is whether a constructor was installed
    /// and `callback` whether the scene has a callback for the class.
    pub fn new(previous: bool, callback: bool) -> Self {
        Self {
            ran_previous: previous,
            ran_callback: callback,
        }
    }
}

/// The six-preset table the environment unit keeps.
#[derive(Clone, Debug, PartialEq)]
pub struct PresetTable {
    pub presets: Vec<[f32; PRESET_WORDS]>,
}

impl PresetTable {
    /// A table over `presets` (six from the data).
    pub fn new(presets: Vec<[f32; PRESET_WORDS]>) -> Self {
        Self { presets }
    }

    /// An all-zero table of `PRESET_COUNT` presets.
    pub fn empty() -> Self {
        Self {
            presets: vec![[0.0; PRESET_WORDS]; PRESET_COUNT],
        }
    }

    /// The original's unit initialisation: when linear lighting is on every
    /// preset is linearised and scaled; then every preset's empty colour is
    /// filled.
    pub fn configure(&mut self, linear_lighting: bool) {
        for preset in &mut self.presets {
            if linear_lighting {
                linearise(preset);
            }
            fill_empty_colour(preset);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset() -> [f32; PRESET_WORDS] {
        let mut p = [0.0; PRESET_WORDS];
        for (i, w) in p.iter_mut().enumerate() {
            *w = i as f32 * 0.125;
        }
        p[EMPTY_WORD..EMPTY_WORD + 3].fill(0.0);
        p
    }

    #[test]
    fn srgb_curve_matches_its_knee() {
        assert_eq!(srgb_to_linear(0.0), 0.0);
        // Below the knee: a plain division by 12.92.
        assert_eq!(srgb_to_linear(0.04), (0.04_f32 as f64 / 12.92) as f32);
        // Above it: the engine's fast logarithm and exponential, close to
        // the exact curve (0.2140411, 1.0) but not equal to it.
        assert_eq!(srgb_to_linear(0.5).to_bits(), 0x3e5b_2ba4);
        assert_eq!(srgb_to_linear(1.0).to_bits(), 0x3f80_014c);
        assert!((srgb_to_linear(0.5) - 0.214_041_13).abs() < 1e-4);
        // A NaN takes the power branch, where the exponential's clamp
        // turns it into a number.
        assert!(!srgb_to_linear(f32::NAN).is_nan());
    }

    #[test]
    fn linearising_converts_then_scales_the_five_words() {
        let mut p = preset();
        let first = p[0];
        let sky = p[EMPTY_FROM];
        linearise(&mut p);
        assert_eq!(p[0], first, "word 0 is not a colour");
        assert_eq!(p[EMPTY_FROM], srgb_to_linear(sky) * LINEAR_SCALE);
        // 0x2c is linearised but not scaled.
        assert_eq!(p[11], srgb_to_linear(11.0 * 0.125));
        // A zero ambient colour is not linearised here.
        assert_eq!(p[EMPTY_WORD], 0.0);
    }

    #[test]
    fn the_empty_colour_blends_a_quarter_of_the_way() {
        let mut p = preset();
        fill_empty_colour(&mut p);
        assert_eq!(
            &p[EMPTY_WORD..EMPTY_WORD + 3],
            &[
                p[EMPTY_FROM] + (p[EMPTY_TO] - p[EMPTY_FROM]) * EMPTY_BLEND,
                p[EMPTY_FROM + 1] + (p[EMPTY_TO + 1] - p[EMPTY_FROM + 1]) * EMPTY_BLEND,
                p[EMPTY_FROM + 2] + (p[EMPTY_TO + 2] - p[EMPTY_FROM + 2]) * EMPTY_BLEND,
            ]
        );
        assert_eq!(p[EMPTY_WORD], 2.78125);
        let mut set = p;
        set[EMPTY_WORD] = 0.5;
        let before = set;
        fill_empty_colour(&mut set);
        assert_eq!(set, before, "a non-empty colour is left alone");
        let mut nan = p;
        nan[EMPTY_WORD] = f32::NAN;
        fill_empty_colour(&mut nan);
        assert!(nan[EMPTY_WORD].is_nan(), "a NaN is not zero");
        assert_eq!(nan[EMPTY_WORD + 1], p[EMPTY_WORD + 1]);
    }

    #[test]
    fn the_table_configures_six_presets() {
        let mut table = PresetTable::empty();
        table.configure(true);
        assert_eq!(table.presets.len(), PRESET_COUNT);
        assert_eq!(table.presets[0][EMPTY_WORD], 0.0, "all zero still zero");
        let mut table = PresetTable::new(vec![preset(); PRESET_COUNT]);
        table.configure(false);
        // No linearisation: the sky colour keeps its value, and the empty
        // colour is filled from it.
        assert_eq!(table.presets[5][EMPTY_FROM], preset()[EMPTY_FROM]);
        assert_ne!(table.presets[5][EMPTY_WORD], 0.0);
    }

    #[test]
    fn the_environment_constructor_chains_over_the_previous_hook() {
        let mut hook = ConstructorHook::default();
        hook.install_environment();
        assert_eq!(hook.previous, None);
        assert_eq!(hook.current, Some(ENVIRONMENT_CONSTRUCTOR));
        // A second install keeps the environment hook as the previous one.
        hook.install_environment();
        assert_eq!(hook.previous, Some(ENVIRONMENT_CONSTRUCTOR));
        // An attach runs the previous hook first and the callback last.
        let attach = EnvironmentAttach::new(true, false);
        assert!(attach.ran_previous && !attach.ran_callback);
        let attach = EnvironmentAttach::new(false, true);
        assert!(!attach.ran_previous && attach.ran_callback);
    }
}
