//! A character's voice set: the original picks from the living voice table
//! when the body's feral level is not above 0, else from the undead voice
//! table, with the record's sex flag (bit 0 of its flags). The footstep set
//! is the golem suit's own `Golem_Stp`, else the one named by the race.
//!
//! No game system selects voices yet; these are the original's rules as
//! tested data. Evidence: the body's voice and footstep set-up routines.

/// The wet splash set the footstep set-up caches beside the walking one
/// (used when a foot meets water, not as a walking fallback).
pub const WET_STEP_SET: &str = "StpWet";

/// The footstep set a body walks on: the golem suit's own when the record
/// carries the golem flag, else the race's step name (its lookup is the
/// caller's; an empty name finds nothing there).
pub fn step_set(golem_suit: bool, race_step: &str) -> &str {
    if golem_suit { "Golem_Stp" } else { race_step }
}

/// The voice a body speaks with: its undead voice when it is feral, else its
/// living voice; `male` is the record's sex bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceChoice {
    pub voice: u32,
    pub male: bool,
    pub feral: bool,
}

/// The voice chosen from the record's feral level (undead when above 0; a
/// NaN level is living), living voice `voice`, undead voice `undead` and
/// sex bit `male`.
pub fn voice_choice(feral_level: f32, voice: u32, undead: u32, male: bool) -> VoiceChoice {
    let feral = feral_level > 0.0;
    VoiceChoice {
        voice: if feral { undead } else { voice },
        male,
        feral,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_step_and_voice_sets_are_the_originals() {
        assert_eq!(step_set(true, "Human_Stp"), "Golem_Stp");
        assert_eq!(step_set(false, "Human_Stp"), "Human_Stp");
        assert_eq!(step_set(false, ""), "");
        assert_eq!(WET_STEP_SET, "StpWet");
        assert_eq!(
            voice_choice(0.0, 3, 9, true),
            VoiceChoice {
                voice: 3,
                male: true,
                feral: false
            }
        );
        assert_eq!(
            voice_choice(0.5, 3, 9, false),
            VoiceChoice {
                voice: 9,
                male: false,
                feral: true
            }
        );
        // Only a level above 0 is feral: 0, negative and NaN speak living.
        assert!(!voice_choice(-1.0, 3, 9, true).feral);
        assert!(!voice_choice(f32::NAN, 3, 9, true).feral);
    }
}
