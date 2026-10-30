//! The story scripts' `GameEvent`: memory fragments, the raise-dead
//! narration, Corlian's line, the three domain awakenings and the background
//! unlock.
//!
//! The handler reads an event code from the script and turns it into what the
//! original does: a requested action, a voice line, a flash, a narrator topic,
//! a sound, a potential change, a class word or a settings bit. It is a pure
//! rule here; [`crate::game_event_live`] applies the effects to the running
//! player and [`crate::flow`] keeps the session words (the fragments seen and
//! the background unlocks).
//!
//! Evidence: the story script handler of the game-main unit (REA, checked
//! against the assembly). The memory class is any `0xC0xxxx` code; the
//! awakening codes are `0xC10EC1` to `0xC10EC3`; the raise-dead and Corlian
//! codes and the sounds' volumes are the handler's constants. The random
//! draws are passed in (the original uses its process random generator), so
//! tests pin concrete values.
//!
//! What the handler's calls are, from their bodies:
//! - the fragment's "motion" is the body's voice request, line 2 (the heavy
//!   groan), at the first factor times the voice volume and the second times
//!   the voice pitch;
//! - its flash is the post-process flash (3 s, the plain variant);
//! - its dialogue line, and the raise-dead and Corlian lines, start a topic
//!   of the narrator graph (`narrator.rcd`), after the given delay (0 at
//!   once);
//! - the raise-dead line asks the power grant for Raise Dead (`0x10F`, the
//!   mind tree): the successful line when the grant wrote the power;
//! - the potential is the float the record keeps beside its power slots
//!   (the powers chunk's first word), and the class word the record word the
//!   reader puts after the experience.
/// The mask that names the memory-fragment class of event codes.
pub const MEMORY_CLASS_MASK: u32 = 0x00ff_0000;
/// The memory-fragment class.
pub const MEMORY_CLASS: u32 = 0x00c0_0000;
/// The fragment number within the class.
pub const FRAGMENT_MASK: u32 = 0xffff;
/// The role a player needs before a fragment is collected.
pub const MEMORY_ROLE: u32 = 0x0e;
/// The requested action of a memory fragment (head pain).
pub const MEMORY_ACTION: u32 = 0x2d;
/// The potential a fragment adds.
pub const MEMORY_POTENTIAL: f32 = 1.0;
/// The fragment's narrator topic and its delay (ms).
pub const MEMORY_DIALOGUE: (&str, u32) = ("XU_Memory", 2500);
/// The AI event a fragment raises.
pub const MEMORY_AI_EVENT: (u32, u32, u32, &str) = (0x13, 7, 1, "XAU_Memory");

/// The raise-dead narration's event code.
pub const RAISE_DEAD_EVENT: u32 = 0xda_0115;
/// The power the raise-dead narration asks the grant for (Raise Dead, in
/// the mind tree); a grant that writes it picks the successful line.
pub const RAISE_DEAD_POWER: u32 = 0x10f;
/// The two raise-dead narrator topics, each after this delay (ms).
pub const RAISE_DEAD_FAILED: (&str, u32) = ("RVL_RaiseDead_F", 1000);
pub const RAISE_DEAD_SUCCEEDED: (&str, u32) = ("RVL_RaiseDead_S", 1000);

/// Corlian's event code.
pub const CORLIAN_EVENT: u32 = 0xc1_0ec0;
/// The class bits and value that let Corlian speak.
pub const CORLIAN_MASK: u32 = 0x00c0_0000;
pub const CORLIAN_VALUE: u32 = 0x0040_0000;
/// Corlian's narrator topic (started at once).
pub const CORLIAN_DIALOGUE: (&str, u32) = ("Corlian", 0);

/// The first and last domain-awakening codes.
pub const AWAKENING_FIRST: u32 = 0xc1_0ec1;
pub const AWAKENING_LAST: u32 = 0xc1_0ec3;
/// The mind awakening's code.
pub const AWAKENING_MIND: u32 = 0xc1_0ec2;
/// The force awakening's codes.
pub const AWAKENING_FORCE: [u32; 2] = [0xc1_0ec1, 0xc1_0ec3];
/// The requested action of an awakening (power).
pub const AWAKENING_ACTION: u32 = 0x2c;
/// The potential an awakening adds before the fragment count is taken off.
pub const AWAKENING_POTENTIAL: f32 = 10.0;
/// The fragment mask after an awakening: every fragment counts as seen.
pub const COLLECTED_ALL: u32 = 0xffff;
/// The class bits each awakening writes: the mask and the three values.
pub const CLASS_MASK: u32 = 0xff00_ffff;
pub const CLASS_MIND: u32 = 0x00d1_0000;
pub const CLASS_FORCE: u32 = 0x008a_0000;
pub const CLASS_OTHER_FORCE: u32 = 0x00ca_0000;

/// The human voices an awakening uses.
pub const VOICE_FEMALE: &str = "VHF02_GroanH01";
pub const VOICE_MALE: &str = "VHM_Exert01";
/// The wail that follows, and the swing of a force awakening.
pub const WAIL: &str = "VSM_Wail01";
pub const FORCE_SWING: &str = "ThmFrc_Swing06";
/// The sounds' volumes and pitches.
pub const VOICE_VOLUME_LOW: f32 = 0.15;
pub const VOICE_VOLUME_RANGE: f32 = 0.1;
pub const VOICE_PITCH_LOW: f32 = 0.9;
pub const VOICE_PITCH_RANGE: f32 = 0.1;
pub const WAIL_VOLUME: f32 = 0.25;
pub const WAIL_PITCH: f32 = 0.9;
pub const SWING_VOLUME: f32 = 0.3;
pub const SWING_PITCH: f32 = 0.8;
/// The sounds' priorities.
pub const SOUND_PRIORITY: u32 = 0x400;
pub const WAIL_PRIORITY: u32 = 0x4460;

/// The background-unlock event and the bit it sets.
pub const UNLOCK_EVENT: u32 = 4;
pub const UNLOCK_BIT: u32 = 4;

/// The voice line a memory fragment speaks: line 2, the heavy groan.
pub const MEMORY_VOICE_LINE: i32 = 2;

/// Where a sound plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundAt {
    /// At the player's body.
    Player,
    /// At the origin (the interface's ambience).
    Origin,
}

/// One thing a story event does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// The motion the player is asked to play.
    RequestAction(u32),
    /// The body's voice request: the line, and the factors the voice volume
    /// and the voice pitch are multiplied by.
    Voice { line: i32, volume: f32, pitch: f32 },
    /// The post-process flash (its plain variant).
    Flash,
    /// A narrator topic, started after `delay_ms` (0 at once).
    Dialogue { text: &'static str, delay_ms: u32 },
    /// The fragment bits marked as seen (or-ed into the session's word).
    Collected(u32),
    /// The session's fragment word becomes this value.
    SetCollected(u32),
    /// A potential change (added to the player).
    Potential(f32),
    /// The player's potential becomes this value.
    SetPotential(f32),
    /// The AI event a fragment raises: its kind, two arguments and its name.
    AiEvent {
        kind: u32,
        first: u32,
        second: u32,
        name: &'static str,
    },
    /// A sound.
    Sound {
        name: &'static str,
        at: SoundAt,
        volume: f32,
        pitch: f32,
        priority: u32,
    },
    /// The player's class word: the bits under `mask` become `bits`.
    Class { mask: u32, bits: u32 },
    /// The settings background bits to store.
    Unlock { bits: u32 },
    /// The mind domain woke.
    MindAwakening,
    /// The force domain woke.
    ForceAwakening,
}

/// The player the handler reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventPlayer {
    /// The player has the memory role.
    pub memory_role: bool,
    /// The power grant wrote Raise Dead ([`RAISE_DEAD_POWER`]). The caller
    /// asks the grant only when [`asks_grant`] says the handler would.
    pub raise_dead: bool,
    /// The player's class word.
    pub class: u32,
    /// The player's sex word has the female bit.
    pub female: bool,
    /// The player's potential.
    pub potential: f32,
    /// The fragments already seen, the session's word.
    pub collected: u32,
    /// The body's voice volume and pitch the groan is scaled by.
    pub voice_volume: f32,
    pub voice_pitch: f32,
}

/// The story's background is the unknown one: the original only runs the
/// raise-dead, Corlian and awakening events there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoryContext {
    pub unknown_background: bool,
}

/// The handler asks the power grant for Raise Dead: the raise-dead code on
/// the unknown background. The grant writes the power when it succeeds, so
/// it runs only here, as the original's does.
pub fn asks_grant(code: u32, story: StoryContext) -> bool {
    code == RAISE_DEAD_EVENT && story.unknown_background
}

/// The record's potential: the float beside its power slots (the powers
/// chunk's first word, 0 without one).
pub fn potential(record: &exa_formats::characters::CharacterRecord) -> f32 {
    record
        .powers
        .as_ref()
        .map_or(0.0, |p| f32::from_bits(p.word))
}

/// Writes the record's potential. A record without a powers chunk gets an
/// empty one (the original's record always has the field; the writer still
/// leaves a chunk without powers out).
pub fn set_potential(record: &mut exa_formats::characters::CharacterRecord, value: f32) {
    let powers = record
        .powers
        .get_or_insert_with(|| exa_formats::characters::Powers {
            word: 0,
            slots: [0; exa_formats::characters::POWER_SLOTS],
            known: Vec::new(),
        });
    powers.word = value.to_bits();
}

/// Applies the effects that write the player's record: the potential (a
/// single-precision add, or the new value) and the class word. Returns
/// whether the effect was one of them.
pub fn apply_to_record(
    effect: &Effect,
    record: &mut exa_formats::characters::CharacterRecord,
) -> bool {
    match *effect {
        Effect::Potential(add) => {
            let value = potential(record) + add;
            set_potential(record, value);
        }
        Effect::SetPotential(value) => set_potential(record, value),
        Effect::Class { mask, bits } => record.extra_word = (record.extra_word & mask) | bits,
        _ => return false,
    }
    true
}

/// The number of set bits in a 16-bit mask (the original's population
/// count).
pub fn fragments_seen(collected: u32) -> u32 {
    (collected & 0xffff).count_ones()
}

/// Handles one story event. `draws` supplies the random numbers the original
/// takes from its process generator, in the order the handler asks for them.
pub fn handle(
    code: u32,
    story: StoryContext,
    player: &EventPlayer,
    unlocks: u32,
    draws: &mut dyn FnMut() -> f64,
) -> Vec<Effect> {
    let mut effects = Vec::new();
    if code & MEMORY_CLASS_MASK == MEMORY_CLASS {
        let fragment = code & FRAGMENT_MASK;
        if !player.memory_role || fragment & player.collected != 0 {
            return effects;
        }
        effects.push(Effect::RequestAction(MEMORY_ACTION));
        // The pitch factor is drawn first, then the volume's.
        let pitch_draw = draws();
        let volume_draw = draws();
        effects.push(Effect::Voice {
            line: MEMORY_VOICE_LINE,
            volume: (volume_draw * 0.2 + 0.4) as f32,
            pitch: (pitch_draw * 0.1 + 0.9) as f32,
        });
        effects.push(Effect::Flash);
        effects.push(Effect::Dialogue {
            text: MEMORY_DIALOGUE.0,
            delay_ms: MEMORY_DIALOGUE.1,
        });
        effects.push(Effect::Collected(fragment));
        effects.push(Effect::Potential(MEMORY_POTENTIAL));
        effects.push(Effect::AiEvent {
            kind: MEMORY_AI_EVENT.0,
            first: MEMORY_AI_EVENT.1,
            second: MEMORY_AI_EVENT.2,
            name: MEMORY_AI_EVENT.3,
        });
    } else if code == RAISE_DEAD_EVENT {
        if story.unknown_background {
            let line = if player.raise_dead {
                RAISE_DEAD_SUCCEEDED
            } else {
                RAISE_DEAD_FAILED
            };
            effects.push(Effect::Dialogue {
                text: line.0,
                delay_ms: line.1,
            });
        }
    } else if code == CORLIAN_EVENT {
        if story.unknown_background && player.class & CORLIAN_MASK == CORLIAN_VALUE {
            effects.push(Effect::Dialogue {
                text: CORLIAN_DIALOGUE.0,
                delay_ms: CORLIAN_DIALOGUE.1,
            });
        }
    } else if !(AWAKENING_FIRST..=AWAKENING_LAST).contains(&code) {
        if code == UNLOCK_EVENT && unlocks & UNLOCK_BIT == 0 {
            effects.push(Effect::Unlock { bits: UNLOCK_BIT });
        }
    } else if story.unknown_background {
        effects.push(Effect::RequestAction(AWAKENING_ACTION));
        let potential =
            (player.potential + AWAKENING_POTENTIAL) - fragments_seen(player.collected) as f32;
        effects.push(Effect::SetPotential(potential));
        effects.push(Effect::SetCollected(COLLECTED_ALL));
        let pitch_draw = draws();
        let volume_draw = draws();
        effects.push(Effect::Sound {
            name: if player.female {
                VOICE_FEMALE
            } else {
                VOICE_MALE
            },
            at: SoundAt::Player,
            volume: (player.voice_volume as f64 * (volume_draw * 0.1 + VOICE_VOLUME_LOW as f64))
                as f32,
            pitch: (player.voice_pitch as f64 * (pitch_draw * 0.1 + VOICE_PITCH_LOW as f64)) as f32,
            priority: SOUND_PRIORITY,
        });
        effects.push(Effect::Sound {
            name: WAIL,
            at: SoundAt::Origin,
            volume: WAIL_VOLUME,
            pitch: WAIL_PITCH,
            priority: WAIL_PRIORITY,
        });
        if code == AWAKENING_MIND {
            effects.push(Effect::MindAwakening);
            effects.push(Effect::Class {
                mask: CLASS_MASK,
                bits: CLASS_MIND,
            });
        } else {
            effects.push(Effect::Sound {
                name: FORCE_SWING,
                at: SoundAt::Player,
                volume: SWING_VOLUME,
                pitch: SWING_PITCH,
                priority: SOUND_PRIORITY,
            });
            // The last code wakes the mind domain too, before its class.
            if code == AWAKENING_FORCE[1] {
                effects.push(Effect::MindAwakening);
            }
            effects.push(Effect::Class {
                mask: CLASS_MASK,
                bits: if code == AWAKENING_FORCE[1] {
                    CLASS_FORCE
                } else {
                    CLASS_OTHER_FORCE
                },
            });
            effects.push(Effect::ForceAwakening);
        }
    }
    effects
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player() -> EventPlayer {
        EventPlayer {
            memory_role: true,
            raise_dead: false,
            class: 0,
            female: false,
            potential: 3.5,
            collected: 0,
            voice_volume: 0.5,
            voice_pitch: 1.0,
        }
    }

    fn draws(values: &[f64]) -> impl FnMut() -> f64 + '_ {
        let mut index = 0;
        move || {
            let value = values.get(index).copied().unwrap_or(0.0);
            index += 1;
            value
        }
    }

    #[test]
    fn a_memory_fragment_plays_its_motion_and_adds_its_potential() {
        let mut draw = draws(&[0.5, 0.25]);
        let effects = handle(
            0xc00005,
            StoryContext {
                unknown_background: true,
            },
            &player(),
            0,
            &mut draw,
        );
        assert_eq!(
            effects,
            vec![
                Effect::RequestAction(0x2d),
                Effect::Voice {
                    line: 2,
                    volume: 0.45,
                    pitch: 0.95
                },
                Effect::Flash,
                Effect::Dialogue {
                    text: "XU_Memory",
                    delay_ms: 2500
                },
                Effect::Collected(5),
                Effect::Potential(1.0),
                Effect::AiEvent {
                    kind: 0x13,
                    first: 7,
                    second: 1,
                    name: "XAU_Memory"
                },
            ]
        );
        // A fragment already seen does nothing, nor does one without the
        // role.
        let mut collected = player();
        collected.collected = 5;
        let mut draw = draws(&[0.0, 0.0]);
        assert!(
            handle(
                0xc00005,
                StoryContext {
                    unknown_background: true
                },
                &collected,
                0,
                &mut draw
            )
            .is_empty()
        );
        let mut roleless = player();
        roleless.memory_role = false;
        let mut draw = draws(&[0.0, 0.0]);
        assert!(
            handle(
                0xc00006,
                StoryContext {
                    unknown_background: true
                },
                &roleless,
                0,
                &mut draw
            )
            .is_empty()
        );
        // A code of another class is not a fragment.
        let mut draw = draws(&[0.0, 0.0]);
        assert!(
            handle(
                0xc10005,
                StoryContext {
                    unknown_background: true
                },
                &player(),
                0,
                &mut draw
            )
            .is_empty()
        );
    }

    #[test]
    fn the_raise_dead_narration_picks_its_line() {
        let mut draw = draws(&[]);
        let effects = handle(
            RAISE_DEAD_EVENT,
            StoryContext {
                unknown_background: true,
            },
            &player(),
            0,
            &mut draw,
        );
        assert_eq!(
            effects,
            vec![Effect::Dialogue {
                text: "RVL_RaiseDead_F",
                delay_ms: 1000
            }]
        );
        let mut able = player();
        able.raise_dead = true;
        let mut draw = draws(&[]);
        assert_eq!(
            handle(
                RAISE_DEAD_EVENT,
                StoryContext {
                    unknown_background: true
                },
                &able,
                0,
                &mut draw
            ),
            vec![Effect::Dialogue {
                text: "RVL_RaiseDead_S",
                delay_ms: 1000
            }]
        );
        let mut draw = draws(&[]);
        assert!(
            handle(
                RAISE_DEAD_EVENT,
                StoryContext {
                    unknown_background: false
                },
                &able,
                0,
                &mut draw
            )
            .is_empty()
        );
    }

    #[test]
    fn corlian_speaks_only_to_the_guild_class() {
        let mut flow = player();
        flow.class = 0x0040_0000;
        let mut draw = draws(&[]);
        assert_eq!(
            handle(
                CORLIAN_EVENT,
                StoryContext {
                    unknown_background: true
                },
                &flow,
                0,
                &mut draw
            ),
            vec![Effect::Dialogue {
                text: "Corlian",
                delay_ms: 0
            }]
        );
        let mut other = player();
        other.class = 0x0080_0000;
        let mut draw = draws(&[]);
        assert!(
            handle(
                CORLIAN_EVENT,
                StoryContext {
                    unknown_background: true
                },
                &other,
                0,
                &mut draw
            )
            .is_empty()
        );
    }

    #[test]
    fn the_mind_awakening_takes_the_fragment_count_off_its_potential() {
        let mut waking = player();
        waking.potential = 3.5;
        waking.collected = 0x000f;
        waking.female = true;
        let mut draw = draws(&[0.5, 0.0]);
        let effects = handle(
            AWAKENING_MIND,
            StoryContext {
                unknown_background: true,
            },
            &waking,
            0,
            &mut draw,
        );
        assert_eq!(effects[0], Effect::RequestAction(0x2c));
        // 3.5 + 10 − 4 = 9.5.
        assert_eq!(effects[1], Effect::SetPotential(9.5));
        assert_eq!(effects[2], Effect::SetCollected(0xffff));
        assert_eq!(
            effects[3],
            Effect::Sound {
                name: "VHF02_GroanH01",
                at: SoundAt::Player,
                volume: (0.5f64 * (0.0 * 0.1 + 0.15)) as f32,
                pitch: (1.0f64 * (0.5 * 0.1 + 0.9)) as f32,
                priority: 0x400
            }
        );
        assert_eq!(
            effects[4],
            Effect::Sound {
                name: "VSM_Wail01",
                at: SoundAt::Origin,
                volume: 0.25,
                pitch: 0.9,
                priority: 0x4460
            }
        );
        // The mind domain wakes, then the class word is written.
        assert_eq!(effects[5], Effect::MindAwakening);
        assert_eq!(
            effects[6],
            Effect::Class {
                mask: 0xff00_ffff,
                bits: 0x00d1_0000
            }
        );
        assert_eq!(effects.len(), 7);
        assert_eq!(fragments_seen(0x000f), 4);
        // The male voice and the force swing.
        let mut draw = draws(&[0.0, 0.0]);
        let effects = handle(
            AWAKENING_FIRST,
            StoryContext {
                unknown_background: true,
            },
            &player(),
            0,
            &mut draw,
        );
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::Sound {
                name: "VHM_Exert01",
                ..
            }
        )));
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::Sound {
                name: "ThmFrc_Swing06",
                ..
            }
        )));
        assert!(effects.contains(&Effect::Class {
            mask: 0xff00_ffff,
            bits: 0x00ca_0000
        }));
        assert!(effects.contains(&Effect::ForceAwakening));
        assert!(!effects.contains(&Effect::MindAwakening));
    }

    #[test]
    fn the_last_force_awakening_has_its_own_class_bits() {
        let mut draw = draws(&[0.0, 0.0]);
        let effects = handle(
            AWAKENING_LAST,
            StoryContext {
                unknown_background: true,
            },
            &player(),
            0,
            &mut draw,
        );
        // The swing, then the mind domain, the class bits and the force
        // domain: the last code wakes both.
        let n = effects.len();
        assert_eq!(effects[n - 3], Effect::MindAwakening);
        assert_eq!(
            effects[n - 2],
            Effect::Class {
                mask: 0xff00_ffff,
                bits: 0x008a_0000
            }
        );
        assert_eq!(effects[n - 1], Effect::ForceAwakening);
        assert!(matches!(
            effects[n - 4],
            Effect::Sound {
                name: "ThmFrc_Swing06",
                ..
            }
        ));
        // The awakenings do nothing outside the unknown background.
        let mut draw = draws(&[0.0, 0.0]);
        assert!(
            handle(
                AWAKENING_MIND,
                StoryContext {
                    unknown_background: false
                },
                &player(),
                0,
                &mut draw
            )
            .is_empty()
        );
    }

    #[test]
    fn the_background_unlock_sets_its_bit_once() {
        let mut draw = draws(&[]);
        assert_eq!(
            handle(
                UNLOCK_EVENT,
                StoryContext {
                    unknown_background: true
                },
                &player(),
                0,
                &mut draw
            ),
            vec![Effect::Unlock { bits: 4 }]
        );
        let mut draw = draws(&[]);
        assert!(
            handle(
                UNLOCK_EVENT,
                StoryContext {
                    unknown_background: true
                },
                &player(),
                4,
                &mut draw
            )
            .is_empty()
        );
        // Any other code does nothing.
        let mut draw = draws(&[]);
        assert!(
            handle(
                77,
                StoryContext {
                    unknown_background: true
                },
                &player(),
                0,
                &mut draw
            )
            .is_empty()
        );
    }

    #[test]
    fn the_record_takes_the_potential_and_the_class_word() {
        let mut record = exa_formats::characters::CharacterRecord::new();
        assert_eq!(potential(&record), 0.0);
        // A record without a powers chunk gets an empty one.
        assert!(apply_to_record(&Effect::Potential(1.0), &mut record));
        assert_eq!(potential(&record), 1.0);
        assert!(record.powers.as_ref().unwrap().known.is_empty());
        assert!(apply_to_record(&Effect::SetPotential(9.5), &mut record));
        assert_eq!(record.powers.as_ref().unwrap().word, 9.5f32.to_bits());
        // The class bits under the mask stay; the awakening's bits are or-ed.
        record.extra_word = 0x0051_1234;
        assert!(apply_to_record(
            &Effect::Class {
                mask: CLASS_MASK,
                bits: CLASS_MIND
            },
            &mut record
        ));
        assert_eq!(record.extra_word, 0x00d1_1234);
        assert!(!apply_to_record(&Effect::Flash, &mut record));
    }

    #[test]
    fn only_the_raise_dead_code_on_the_unknown_background_asks_the_grant() {
        let unknown = StoryContext {
            unknown_background: true,
        };
        let other = StoryContext {
            unknown_background: false,
        };
        assert!(asks_grant(RAISE_DEAD_EVENT, unknown));
        assert!(!asks_grant(RAISE_DEAD_EVENT, other));
        assert!(!asks_grant(CORLIAN_EVENT, unknown));
    }
}
