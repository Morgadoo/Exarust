//! The stats a character's record and race give it when it spawns, as the
//! original's character builder and init derive them: body scale and size
//! from the record's height, the standing height and the leg reach, the
//! most energy, the step reach and the support pins' factor from the feral
//! level, the head's control and the walking arms of the undead, the sprint
//! gait, the voice, the maximum health with the race-byte rules, the health
//! the record saved and the body rule bits. Rules and evidence:
//! `docs/formats/RFC.md` (Stats derived at spawn).
//!
//! Lengths are returned in metres; the rules are the original's, in its
//! centimetres, some of them in double precision as it computes them.

use crate::characters::{CharacterRecord, FULL_POOL, Pools};
use crate::races::{Gait, Race, RaceBody, RaceStats, SCALE_CAP};

/// Per unit of the record's height (−1…1), the body scale and size shrink
/// by this share: a tall character (−1) is 1.05 times the race's, a short
/// one (1) 0.95.
const HEIGHT_SCALE: f64 = 0.05;
/// Standing pelvis height, cm: 109.5 less 11.25 per unit of height.
const STANDING_HEIGHT: f32 = 109.5;
const STANDING_HEIGHT_PER_HEIGHT: f32 = 11.25;
/// Leg reach normaliser, cm: 107.2 less 11.1 per unit of height (the init
/// overwrites the builder's 108.5 − 11.25·h).
const LEG_REACH: f64 = 107.2;
const LEG_REACH_PER_HEIGHT: f64 = 11.1;
/// The locomotion clock runs 1/(1 − 0.1·h) times as fast.
const GAIT_CLOCK_PER_HEIGHT: f64 = 0.1;
/// Each unit of the feral level takes this much off the most energy.
const FERAL_ENERGY: f64 = 0.1;
/// Step reach, cm: 80·√scale, shortened 30 % per unit of feral level and
/// 20 % per unit of encumbrance.
const STEP_REACH: f32 = 80.0;
const FERAL_REACH: f64 = 0.3;
const ENCUMBERED_REACH: f64 = 0.2;
/// The support pins' factor loses 0.25·f² (a human's 1.25 is 1.0 at f = 1).
const FERAL_PINS: f32 = 0.25;
/// The head's muscle factor loses 80 % per unit of feral level.
const FERAL_HEAD: f64 = 0.8;
/// Arms in the walk and run cycles: 0.2 less 0.15 per unit of feral level,
/// kept within 0.05…0.2.
const WALKING_ARMS: f64 = 0.2;
const FERAL_ARMS: f64 = 0.15;
const LOOSEST_ARMS: f64 = 0.05;
/// A feral character's resistance to a masked target is 1 − f/2.
const FERAL_PERCEPTION: f32 = 0.5;
/// The voice pitch the record stores is kept within 0.9…1.07; the hulking
/// ghoul speaks at 0.85.
const LOWEST_PITCH: f64 = 0.9;
const HIGHEST_PITCH: f64 = 1.07;
const HULKING_GHOUL_PITCH: f32 = 0.85;
/// A feral character's attack-voice step (its race's less one otherwise).
const FERAL_ATTACK_VOICE: i32 = 1;
/// Maximum health is this times the race's health factor (a human 0.25)
/// before the race-byte rules.
const HEALTH_PER_FACTOR: f32 = 0.25;
/// The load reads a saved pool as its 16-bit share times this.
const POOL_UNIT: f32 = 1.525_902_2e-5;

/// Race bytes the init tests (the record's race byte, which also picks the
/// race: its id is the byte plus one).
const SKELETON: u8 = 1;
const ANCIENT: u8 = 2;
const OGRE: u8 = 3;
const SIR: u8 = 4;
const FAEL: u8 = 5;
const GHOUL: u8 = 6;
const HULKING_GHOUL: u8 = 7;
const GOBBLER: u8 = 8;
const MANIPULATOR: u8 = 9;
const WRAITH: u8 = 14;
const GOLEM_BOSS: u8 = 17;
/// Race bytes with this bit are golems.
const GOLEM_BIT: u8 = 0x10;
/// Race flag bits (first word) that make a golem, for the armour pass,
/// and the worn item (without armour) that lets each kind regenerate red
/// health: item 500 for `0x2000` golems, 431 for `0x1000` ones.
const GOLEM_FLAGS: u32 = 0x3000;
const GOLEM_CORES: [(u32, u32); 2] = [(0x2000, 500), (0x1000, 431)];
/// First race flag word: may rise again (with the story), steadfast.
const RACE_MAY_RISE: u32 = 0x8;
const RACE_STEADFAST: u32 = 0x20;
const RACE_REGENERATES_RED: u32 = 0x80;
/// Second race flag word: creature hands, fierce, red regeneration.
const RACE2_CREATURE: u32 = 0x1;
const RACE2_FIERCE: u32 = 0x2;
const RACE2_REGENERATES_RED: u32 = 0x4;
/// Over this feral level a character is fierce.
const FIERCE_FERAL: f32 = 0.5;

/// What the original's spawn knows besides the record and the race.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Spawn {
    /// The game runs the story, not the arena or a practice fight: the
    /// living who may rise again are marked so.
    pub story: bool,
    /// The character is placed dead (a corpse).
    pub dead: bool,
}

/// The rule bits the character's init sets on its body, for the systems
/// that read them (damage, regeneration, remedies, the AI).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BodyRules(pub u32);

impl BodyRules {
    /// A creature (the ogres, the Herath races, the manipulator): its hands
    /// take no fist sphere when the held-item pass sets them up (inferred
    /// from that pass, the only body reader; the AI reads it too).
    pub const CREATURE: u32 = 0x1;
    /// Bandages and potions work on it: living humans that are not feral,
    /// and the creatures.
    pub const TAKES_REMEDIES: u32 = 0x2;
    /// Read only by the AI (what it changes there is not traced): over
    /// half feral, the creatures and races with the second word's bit 1.
    pub const FIERCE: u32 = 0x4;
    /// Blows take nothing from its red (lasting) health: feral humans,
    /// skeletons and ancients.
    pub const NO_LASTING_WOUNDS: u32 = 0x8;
    /// May rise again after death, in the story: living humans that are not
    /// feral, and races with the first word's bit 3 (inferred from the AI
    /// manager's revival).
    pub const MAY_RISE: u32 = 0x10;
    /// Regenerates red health.
    pub const REGENERATES_RED: u32 = 0x20;
    /// A blow dazes it a quarter as much and never stalls its swing (the
    /// big races and golems).
    pub const STEADFAST: u32 = 0x40;
    /// Red regeneration is blocked: golems, unless they wear their kind's
    /// core item (set by the armour pass the spawn runs after the init).
    pub const NO_RED_REGENERATION: u32 = 0x100;

    pub fn has(self, bits: u32) -> bool {
        self.0 & bits == bits
    }
}

/// The stats a character spawns with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterStats {
    /// The record's height, kept within −1…1 (positive is shorter).
    pub height: f32,
    /// The record's feral level, kept within 0…1.
    pub feral: f32,
    /// Body scale: the race's (at most [`SCALE_CAP`]) times 1 − 0.05·height.
    /// The turn and step rates, the reach, the limbs' muscle factors and the
    /// voice volume follow it.
    pub scale: f32,
    /// Character size: the race's times 1 − 0.05·height. Held weapons are
    /// placed and their stats scaled by it.
    pub size: f32,
    /// Standing pelvis height, metres: the spawn places the body this high
    /// over the floor (less 8 cm).
    pub standing_height: f32,
    /// The legs' reach normaliser, metres.
    pub leg_reach: f32,
    /// Factor on the locomotion clip clock: 1/(1 − 0.1·height).
    pub gait_clock: f32,
    /// The most energy the body recovers to after a daze: 1, or 1 − 0.1·feral.
    pub max_energy: f32,
    /// The support pins' factor: the race's, less 0.25·feral².
    pub pin_factor: f32,
    /// Scales the head's pose-drive gain: 1 − feral (none at feral 1).
    pub head_gain: f32,
    /// Scales the head's muscle factor: 1 − 0.8·feral.
    pub head_factor: f32,
    /// The arms' stiffness in the walk and run cycles: 0.2 − 0.15·feral,
    /// at least 0.05.
    pub walking_arms: f32,
    /// The sprint gait: the race's, moved toward its jog by the feral level.
    pub sprint: Gait,
    /// Base rate of the clip clock: the race's.
    pub clip_rate: f32,
    /// How well its AI sees a target masked by a mind power: the race's,
    /// or 1 − feral/2 for the feral.
    pub perception: f32,
    /// Voice pitch: the record's, kept within 0.9…1.07 (0.85 for the
    /// hulking ghoul).
    pub voice_pitch: f32,
    /// Voice volume: the body scale times the race's factor.
    pub voice_volume: f32,
    /// Attack-voice counter step: the race's attack voices less one (below
    /// zero it never voices an attack); 1 for the feral.
    pub attack_voice_step: i32,
    /// Maximum health: 0.25 times the race's factor, with the race-byte
    /// rules.
    pub max_health: f32,
    /// Yellow (current) and red (lasting) health at spawn, from the saved
    /// pools.
    pub yellow: f32,
    pub red: f32,
    pub rules: BodyRules,
    /// Deflection is learnt: worn armour covers more of each part.
    pub deflection: bool,
    /// Encumbrance (0…1) from what it wears and the shield it holds, which
    /// the spawn's armour pass sums after the init
    /// ([`CharacterStats::with_encumbrance`]); 0 until then.
    pub encumbrance: f32,
}

impl Default for CharacterStats {
    /// A new record (height 0, not feral) of an unknown race.
    fn default() -> Self {
        Self::derive(&CharacterRecord::default(), None, Spawn::default())
    }
}

impl CharacterStats {
    /// The stats `record` spawns with, of `race` (the race its race byte
    /// names; `None` takes the original's built-in record).
    pub fn derive(record: &CharacterRecord, race: Option<&Race>, spawn: Spawn) -> Self {
        let body = race.map_or(RaceBody::FALLBACK, |r| r.body);
        let stats = race.map_or(RaceStats::FALLBACK, |r| r.stats.clone());
        let flags = race.map_or(0, |r| r.flags);
        let health_factor = race.map_or(1.0, |r| r.health);
        let byte = record.race;
        // The builder keeps the record's height and feral level in range
        // before anything reads them.
        let h = record.height.clamp(-1.0, 1.0);
        let f = record.feral.clamp(0.0, 1.0);
        let feral = f > 0.0;

        let shrink = (1.0 - HEIGHT_SCALE * f64::from(h)) as f32;
        let scale = body.scale.min(SCALE_CAP) * shrink;
        let max_health = max_health(byte, health_factor);
        let pools = if spawn.dead || record.pools.red != 0 {
            record.pools
        } else {
            // A living placement whose record saved no red health (the
            // arena's opponents) spawns with full yellow and red.
            Pools {
                yellow: FULL_POOL,
                red: FULL_POOL,
                ..record.pools
            }
        };
        let mut voice_pitch = clamp_f64(record.voice_pitch, LOWEST_PITCH, HIGHEST_PITCH);
        if byte == HULKING_GHOUL {
            voice_pitch = HULKING_GHOUL_PITCH;
        }
        Self {
            height: h,
            feral: f,
            scale,
            size: body.size * shrink,
            standing_height: (STANDING_HEIGHT - h * STANDING_HEIGHT_PER_HEIGHT) / CM,
            leg_reach: (LEG_REACH - f64::from(h) * LEG_REACH_PER_HEIGHT) as f32 / CM,
            gait_clock: (1.0 / (1.0 - f64::from(h) * GAIT_CLOCK_PER_HEIGHT)) as f32,
            max_energy: if feral {
                (1.0 - f64::from(f) * FERAL_ENERGY) as f32
            } else {
                1.0
            },
            pin_factor: if feral {
                body.pin_factor - f * f * FERAL_PINS
            } else {
                body.pin_factor
            },
            head_gain: 1.0 - f,
            head_factor: (1.0 - f64::from(f) * FERAL_HEAD) as f32,
            walking_arms: clamp_f64(
                (WALKING_ARMS - f64::from(f) * FERAL_ARMS) as f32,
                LOOSEST_ARMS,
                WALKING_ARMS,
            ),
            sprint: body.sprint_gait(f),
            clip_rate: body.clip_rate,
            perception: if feral {
                1.0 - f * FERAL_PERCEPTION
            } else {
                stats.perception
            },
            voice_pitch,
            voice_volume: scale * stats.voice_volume,
            attack_voice_step: if feral {
                FERAL_ATTACK_VOICE
            } else {
                stats.attack_voices - 1
            },
            max_health,
            yellow: max_health * f32::from(pools.yellow) * POOL_UNIT,
            red: max_health * f32::from(pools.red) * POOL_UNIT,
            rules: rules(byte, f, flags, stats.flags2, spawn.story, &record.worn),
            deflection: record.deflection(),
            encumbrance: 0.0,
        }
    }

    /// These stats with the encumbrance the armour pass summed
    /// ([`crate::encumbrance`]).
    pub fn with_encumbrance(self, encumbrance: f32) -> Self {
        Self {
            encumbrance,
            ..self
        }
    }

    /// The step reach, metres, at `encumbrance` (0…1): 0.8 m·√scale,
    /// shortened 30 % per unit of feral level and 20 % per unit of
    /// encumbrance. The feet's long steps, the footwork and the knockback
    /// step are measured in it.
    pub fn step_reach(&self, encumbrance: f32) -> f32 {
        let full = f64::from(STEP_REACH * self.scale.sqrt());
        let encumbered = 1.0 - f64::from(encumbrance) * ENCUMBERED_REACH;
        let feral = if self.feral > 0.0 {
            1.0 - f64::from(self.feral) * FERAL_REACH
        } else {
            1.0
        };
        (feral * encumbered * full) as f32 / CM
    }
}

const CM: f32 = 100.0;

/// `v` kept within `lo`…`hi`, compared in double precision as the original
/// does.
fn clamp_f64(v: f32, lo: f64, hi: f64) -> f32 {
    let wide = f64::from(v);
    if wide < lo {
        lo as f32
    } else if wide > hi {
        hi as f32
    } else {
        v
    }
}

/// Maximum health: 0.25 times the race's factor, then the init's rules by
/// race byte: the ogre 1.5 and Sir 1.0 (unless a golem), golems 0.5 (the
/// boss 1.0), and the ancient, the Fael, the ghouls, the gobbler and the
/// manipulator by their own factors (the manipulator's 1.5 is not its
/// race's 1).
fn max_health(byte: u8, factor: f32) -> f32 {
    let mut max = HEALTH_PER_FACTOR * factor;
    if byte != WRAITH && byte != SKELETON {
        if byte & GOLEM_BIT == 0 {
            match byte {
                SIR => max = 1.0,
                OGRE => max = 1.5,
                _ => {}
            }
        } else {
            max = if byte == GOLEM_BOSS { 1.0 } else { 0.5 };
        }
    }
    let own = match byte {
        ANCIENT | GHOUL | MANIPULATOR => 1.5,
        FAEL => 3.0,
        HULKING_GHOUL | GOBBLER => 2.0,
        _ => return max,
    };
    HEALTH_PER_FACTOR * own
}

/// The body rule bits: by race byte, then by the race's flag words, then
/// by the feral level, then the armour pass's golem bit, cleared by a worn
/// core (`worn`: the record's worn item ids as stored).
fn rules(byte: u8, feral: f32, flags: u32, flags2: u32, story: bool, worn: &[u32]) -> BodyRules {
    let creature = BodyRules::CREATURE | BodyRules::TAKES_REMEDIES | BodyRules::FIERCE;
    let mut bits = match byte {
        0 if feral > 0.0 => BodyRules::NO_LASTING_WOUNDS,
        0 if story => BodyRules::TAKES_REMEDIES | BodyRules::MAY_RISE,
        0 => BodyRules::TAKES_REMEDIES,
        SKELETON | ANCIENT => BodyRules::NO_LASTING_WOUNDS,
        OGRE | SIR => creature | BodyRules::REGENERATES_RED | BodyRules::STEADFAST,
        FAEL..=MANIPULATOR => creature,
        _ => 0,
    };
    if byte == 0 && feral > FIERCE_FERAL {
        bits |= BodyRules::FIERCE;
    }
    if flags & RACE_MAY_RISE != 0 && story {
        bits |= BodyRules::MAY_RISE;
    }
    if flags & RACE_STEADFAST != 0 {
        bits |= BodyRules::STEADFAST;
    }
    if flags & RACE_REGENERATES_RED != 0 {
        bits |= BodyRules::REGENERATES_RED;
    }
    if flags2 & RACE2_CREATURE != 0 {
        bits |= BodyRules::CREATURE;
    }
    if flags2 & RACE2_FIERCE != 0 {
        bits |= BodyRules::FIERCE;
    }
    if flags2 & RACE2_REGENERATES_RED != 0 {
        bits |= BodyRules::REGENERATES_RED;
    }
    if feral > 0.0 {
        if feral > FIERCE_FERAL {
            bits |= BodyRules::FIERCE;
        }
        bits &= !(BodyRules::TAKES_REMEDIES | BodyRules::MAY_RISE);
        bits |= BodyRules::NO_LASTING_WOUNDS;
    }
    if flags & GOLEM_FLAGS != 0 {
        bits |= BodyRules::NO_RED_REGENERATION;
        let cored = GOLEM_CORES
            .iter()
            .any(|&(kind, core)| flags & kind != 0 && worn.contains(&core));
        if cored {
            bits &= !BodyRules::NO_RED_REGENERATION;
        }
    }
    BodyRules(bits)
}

#[cfg(test)]
#[path = "character_stats_test.rs"]
mod tests;
