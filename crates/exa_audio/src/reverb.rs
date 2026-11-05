//! The EFX reverb: one global EAX reverb for every scene, whose decay time a
//! level shifts by an offset.
//!
//! The original creates one auxiliary effect slot and an `AL_EFFECT_EAXREVERB`
//! on first use, then sends every world voice to it. The block it uploads is
//! the EFX "Generic" preset with the gain and the low-frequency gain changed
//! and two fields rounded (all 23 fields read from the stored block). A level's environment record
//! adds its decay offset, `1.49 s + offset`, worked in double precision.
//!
//! The outdoor arena preset turns the reverb off again (indoor arenas and the
//! story levels keep it), so an outdoor arena is dry.
//!
//! Each upload goes field by field through `alEffectf`; OpenAL refuses a
//! decay outside the EFX range (0.1 to 20 s) and the effect keeps the decay
//! it had. [`Reverb::effect`] is the block the effect holds after the
//! uploads, which the backend (`efx`, `efx_bus`) plays.

/// The preset's base decay time in seconds, the double-precision constant
/// the offset is added to.
pub const BASE_DECAY_S: f64 = 1.49;

use bevy::prelude::*;

use crate::AudioEnvironment;

/// A level's reverb: its environment record's decay offset, and whether
/// the scene leaves the reverb on (every scene turns it on; the outdoor
/// arena preset turns it off).
#[derive(Resource, Clone, Copy, Debug)]
pub struct LevelReverb {
    pub decay_offset: f32,
    pub enabled: bool,
}

impl Default for LevelReverb {
    fn default() -> Self {
        Self {
            decay_offset: 0.0,
            enabled: true,
        }
    }
}

impl LevelReverb {
    /// A level's reverb from whether it takes the outdoor arena preset and
    /// its record's `decay_offset`: off outdoors, where the preset disables
    /// it.
    pub fn for_level(outdoor: bool, decay_offset: f32) -> Self {
        Self {
            decay_offset,
            enabled: !outdoor,
        }
    }
}

/// Applies the level's reverb when the resource changes, in the original's
/// order: the scene turns the reverb on, the level record sets the decay
/// (uploaded at once, the reverb being on), then an outdoor preset turns it
/// off.
pub(crate) fn apply_level_reverb(
    mut env: ResMut<AudioEnvironment>,
    level: Option<Res<LevelReverb>>,
) {
    let Some(level) = level else {
        return;
    };
    if !level.is_changed() {
        return;
    }
    env.reverb.set_enabled(true);
    env.reverb.set_decay_offset(level.decay_offset);
    env.reverb.set_enabled(level.enabled);
    let effect = env.reverb.effect();
    info!(
        "audio reverb: level decay offset {:.3} s, block decay {:.3} s, effect decay {:.3} s, {}",
        level.decay_offset,
        env.reverb.decay_s(),
        effect.decay_s,
        if env.reverb.enabled {
            "on (world voices send to the slot)"
        } else {
            "off (outdoor preset: dry)"
        }
    );
}

/// Parameter order and values of the effect's property block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EaxReverb {
    pub density: f32,
    pub diffusion: f32,
    pub gain: f32,
    pub gain_hf: f32,
    pub gain_lf: f32,
    pub decay_s: f32,
    pub decay_hf_ratio: f32,
    pub decay_lf_ratio: f32,
    pub reflections_gain: f32,
    pub reflections_delay_s: f32,
    pub reflections_pan: [f32; 3],
    pub late_gain: f32,
    pub late_delay_s: f32,
    pub late_pan: [f32; 3],
    pub echo_time_s: f32,
    pub echo_depth: f32,
    pub modulation_time_s: f32,
    pub modulation_depth: f32,
    pub air_absorption_hf: f32,
    pub hf_reference_hz: f32,
    pub lf_reference_hz: f32,
    pub room_rolloff: f32,
    pub decay_hf_limit: bool,
}

impl Default for EaxReverb {
    /// The block the original uploads, bit for bit as it is stored: the
    /// EFX "Generic" preset with gain 0.326 (Generic 0.3162), low-frequency
    /// gain 0.5 (Generic 1.0), late reverb gain 1.259 (Generic 1.2589) and
    /// air absorption 0.994 (Generic 0.9943).
    fn default() -> Self {
        Self {
            density: 1.0,
            diffusion: 1.0,
            gain: f32::from_bits(0x3ea6_e979),
            gain_hf: f32::from_bits(0x3f64_1893),
            gain_lf: 0.5,
            decay_s: f32::from_bits(0x3fbe_b852),
            decay_hf_ratio: f32::from_bits(0x3f54_7ae1),
            decay_lf_ratio: 1.0,
            reflections_gain: f32::from_bits(0x3d4c_cccd),
            reflections_delay_s: f32::from_bits(0x3be5_6042),
            reflections_pan: [0.0; 3],
            late_gain: f32::from_bits(0x3fa1_26e9),
            late_delay_s: f32::from_bits(0x3c34_3958),
            late_pan: [0.0; 3],
            echo_time_s: 0.25,
            echo_depth: 0.0,
            modulation_time_s: 0.25,
            modulation_depth: 0.0,
            air_absorption_hf: f32::from_bits(0x3f7e_76c9),
            hf_reference_hz: 5000.0,
            lf_reference_hz: 250.0,
            room_rolloff: 0.0,
            decay_hf_limit: true,
        }
    }
}

/// The global reverb's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reverb {
    /// Whether world voices would be sent through the effect.
    pub enabled: bool,
    /// The level's decay offset in seconds.
    pub decay_offset: f32,
    /// Whether a backend could create the effect; Exarust's own reverb
    /// always can.
    pub effect_ready: bool,
    params: EaxReverb,
    /// The block the effect holds, `None` until the first upload.
    effect: Option<EaxReverb>,
    /// How many uploads have reached the effect.
    uploads: u32,
}

impl Default for Reverb {
    fn default() -> Self {
        Self {
            enabled: false,
            decay_offset: 0.0,
            effect_ready: true,
            params: EaxReverb::default(),
            effect: None,
            uploads: 0,
        }
    }
}

impl Reverb {
    /// The parameter block that would be uploaded.
    pub fn params(&self) -> &EaxReverb {
        &self.params
    }

    /// Sets the level's decay offset. While the reverb is enabled the block's
    /// decay becomes `1.49 s + offset` at once; while it is off the offset is
    /// only remembered.
    pub fn set_decay_offset(&mut self, offset: f32) {
        self.decay_offset = offset;
        if self.enabled {
            self.apply_decay();
            self.upload();
        }
    }

    /// The block the effect slot plays: the stored block as last uploaded,
    /// with the last decay OpenAL accepted. Before any upload, the EFX
    /// defaults' decay (1.49 s) with the stored block.
    pub fn effect(&self) -> EaxReverb {
        self.effect.unwrap_or_default()
    }

    /// Whether an effect is attached to the slot yet (after the first
    /// enable).
    pub fn has_effect(&self) -> bool {
        self.effect.is_some()
    }

    /// The number of uploads so far; a change tells the backend to refresh.
    pub fn uploads(&self) -> u32 {
        self.uploads
    }

    /// Uploads the block field by field and attaches it to the slot. A decay
    /// outside the EFX range is refused and the effect keeps its old one.
    fn upload(&mut self) {
        let old_decay = self.effect().decay_s;
        let mut effect = self.params;
        let decay = effect.decay_s;
        if !(crate::efx::MIN_DECAY_S..=crate::efx::MAX_DECAY_S).contains(&decay) {
            effect.decay_s = old_decay;
        }
        self.effect = Some(effect);
        self.uploads = self.uploads.wrapping_add(1);
    }

    /// Turns the reverb on or off. Enabling for the first time needs an
    /// effect; when none can be created the original logs and stays silent.
    /// Returns whether the reverb is on afterwards.
    pub fn set_enabled(&mut self, enabled: bool) -> bool {
        if enabled == self.enabled {
            return self.enabled;
        }
        if !enabled {
            self.enabled = false;
            return false;
        }
        if !self.effect_ready {
            return false;
        }
        self.apply_decay();
        self.upload();
        self.enabled = true;
        true
    }

    /// The decay time a world voice would hear, enabled or not:
    /// `offset + 1.49` in double precision, stored as a single.
    pub fn decay_s(&self) -> f32 {
        (f64::from(self.decay_offset) + BASE_DECAY_S) as f32
    }

    fn apply_decay(&mut self) {
        self.params.decay_s = self.decay_s();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The block is the stored one: density and diffusion 1, gain 0.326,
    /// high-frequency gain 0.891, low-frequency gain 0.5, decay 1.49 s,
    /// ratios 0.83 and 1, reflections 0.05 at 7 ms, late reverb 1.259 at
    /// 11 ms, echo and modulation 0.25 s at depth 0, air absorption 0.994,
    /// references 5 kHz and 250 Hz, no room rolloff, high-frequency decay
    /// limited.
    #[test]
    fn the_preset_is_the_stored_block() {
        let preset = EaxReverb::default();
        assert_eq!(preset.density, 1.0);
        assert_eq!(preset.diffusion, 1.0);
        assert_eq!(preset.gain, 0.326);
        assert_eq!(preset.gain_hf, 0.891);
        assert_eq!(preset.gain_lf, 0.5);
        assert_eq!(preset.decay_s, 1.49);
        assert_eq!(preset.decay_hf_ratio, 0.83);
        assert_eq!(preset.decay_lf_ratio, 1.0);
        assert_eq!(preset.reflections_gain, 0.05);
        assert_eq!(preset.reflections_delay_s, 0.007);
        assert_eq!(preset.reflections_pan, [0.0; 3]);
        assert_eq!(preset.late_gain, 1.259);
        assert_eq!(preset.late_delay_s, 0.011);
        assert_eq!(preset.late_pan, [0.0; 3]);
        assert_eq!(preset.echo_time_s, 0.25);
        assert_eq!(preset.echo_depth, 0.0);
        assert_eq!(preset.modulation_time_s, 0.25);
        assert_eq!(preset.modulation_depth, 0.0);
        assert_eq!(preset.air_absorption_hf, 0.994);
        assert_eq!(preset.hf_reference_hz, 5000.0);
        assert_eq!(preset.lf_reference_hz, 250.0);
        assert_eq!(preset.room_rolloff, 0.0);
        assert!(preset.decay_hf_limit);
    }

    /// A decay offset only reaches the block while the reverb is on; turning
    /// it on applies the latest offset, turning it off keeps the block.
    #[test]
    fn the_decay_offset_applies_while_enabled() {
        let mut reverb = Reverb::default();
        assert!(!reverb.enabled);
        assert_eq!(reverb.params().decay_s, 1.49);
        reverb.set_decay_offset(0.51);
        assert_eq!(reverb.params().decay_s, 1.49);
        assert_eq!(reverb.decay_s(), (f64::from(0.51f32) + 1.49) as f32);
        assert!(reverb.set_enabled(true));
        assert_eq!(reverb.params().decay_s, reverb.decay_s());
        reverb.set_decay_offset(-0.49);
        assert!((reverb.params().decay_s - 1.0).abs() < 1e-6);
        assert!(!reverb.set_enabled(false));
        reverb.set_decay_offset(10.0);
        assert!((reverb.params().decay_s - 1.0).abs() < 1e-6);
        assert_eq!(reverb.decay_s(), 11.49);
    }

    /// The effect takes each upload; a decay out of the EFX range is refused
    /// and the effect keeps the decay it had, while the stored block takes
    /// the new one.
    #[test]
    fn the_effect_refuses_an_out_of_range_decay() {
        let mut reverb = Reverb::default();
        assert!(!reverb.has_effect());
        assert_eq!(reverb.effect().decay_s, 1.49);
        reverb.set_decay_offset(0.51);
        assert_eq!(reverb.uploads(), 0);
        assert!(reverb.set_enabled(true));
        assert_eq!(reverb.uploads(), 1);
        assert_eq!(reverb.effect().decay_s, reverb.decay_s());
        let accepted = reverb.effect().decay_s;
        reverb.set_decay_offset(-1.45);
        assert_eq!(reverb.uploads(), 2);
        assert!(reverb.params().decay_s < 0.1);
        assert_eq!(reverb.effect().decay_s, accepted);
        reverb.set_decay_offset(25.0);
        assert_eq!(reverb.effect().decay_s, accepted);
        reverb.set_decay_offset(0.0);
        assert_eq!(reverb.effect().decay_s, 1.49);
        assert_eq!(reverb.effect().gain, EaxReverb::default().gain);
    }

    /// A level's reverb: on with the record's offset indoors, off outdoors;
    /// disabling keeps the effect attached.
    #[test]
    fn an_outdoor_level_turns_the_reverb_off() {
        assert!(LevelReverb::for_level(false, 0.2).enabled);
        assert!(!LevelReverb::for_level(true, 0.2).enabled);
        let mut reverb = Reverb::default();
        reverb.set_enabled(true);
        reverb.set_decay_offset(0.2);
        reverb.set_enabled(false);
        assert!(!reverb.enabled);
        assert!(reverb.has_effect());
        assert!((reverb.effect().decay_s - 1.69).abs() < 1e-6);
    }

    /// Without an effect the enable fails and leaves the reverb off, as the
    /// original's failed slot creation does.
    #[test]
    fn enabling_without_an_effect_stays_off() {
        let mut reverb = Reverb {
            effect_ready: false,
            ..Default::default()
        };
        assert!(!reverb.set_enabled(true));
        assert!(!reverb.enabled);
        assert_eq!(reverb.params().decay_s, 1.49);
    }
}
