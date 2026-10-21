//! A character's voice set: the original picks by its feral level, from the
//! living voice or the undead one, and its sex flag (bit 0 of the record's
//! flags). The footstep set is the race's step name, the
//! golem suit's own `Golem_Stp`, or the wet `StpWet` fallback.
//!
//! No game system selects voices yet; these are the original's rules as
//! tested data. Evidence: the body's voice and footstep set-up routines.

/// The footstep set a body walks on: the golem suit's own, else the race's
/// step name, else the wet fallback `StpWet`.
pub fn step_set(golem_suit: bool, race_step: &str) -> &str {
    if golem_suit {
        "Golem_Stp"
    } else if race_step.is_empty() {
        "StpWet"
    } else {
        race_step
    }
}

/// The voice a body speaks with: its undead voice when it is feral, else its
/// living voice; `male` is the record's sex bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceChoice {
    pub voice: u32,
    pub male: bool,
    pub feral: bool,
}

/// The voice chosen from the record's feral level, living voice `voice`,
/// undead voice `undead` and sex bit `male`.
pub fn voice_choice(feral: bool, voice: u32, undead: u32, male: bool) -> VoiceChoice {
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
        assert_eq!(step_set(false, ""), "StpWet");
        assert_eq!(
            voice_choice(false, 3, 9, true),
            VoiceChoice {
                voice: 3,
                male: true,
                feral: false
            }
        );
        assert_eq!(
            voice_choice(true, 3, 9, false),
            VoiceChoice {
                voice: 9,
                male: false,
                feral: true
            }
        );
    }
}
