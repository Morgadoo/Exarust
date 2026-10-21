//! Characters placed in a level (chunk `0xCAC00001`) and their records, kept
//! in the level's own character database or, for the arena's fighters, in
//! `actors.rpk`. Layout and evidence: `docs/formats/RFC.md` (Starts,
//! characters and placed objects).
//!
//! A record is read whole, as the original's two readers read it (version 7,
//! and the legacy reader for versions 5 and 6): who the character is and how
//! they look, what they hold and wear, then the feral level, the saved
//! health pools, the combat skill, the six skills with the techniques chosen
//! at each level, the inventory, the roles, the relations, the saved body
//! state and the thaumaturge's powers. The techniques give the character
//! its perks ([`CharacterRecord::perks`]).
//!
//! A version 7 record can be written back ([`CharacterRecord::to_bytes`]),
//! which is how the game saves it; a fresh one takes the defaults the
//! original's creator starts from ([`CharacterRecord::reset`]).

use crate::rdb::Database;
use crate::{Error, Result};

pub const CHARACTERS: u32 = 0xCAC0_0001;
const CHARACTER_DB: u32 = 0xCECD_DB00;
const PLACEMENT: u32 = 0xCAC0_CA00;
const POSE: u32 = 0xCAC0_EB00;
const LEGACY_POSE: u32 = 0xCAC0_E100;
/// Condition of a dead body, placed from its pose.
pub const DEAD: i32 = 0xA00;
const CENTIMETRES_PER_METRE: f32 = 100.0;
/// Record ids carry flags in their top bits.
pub const ID_MASK: u32 = 0x0FFF_FFFF;
pub const WORLD_DB: u32 = 0x4000_0000;
/// Item ids of the level's own object database.
pub const LOCAL_ITEM: u32 = 0x8000_0000;

/// Tag of a character record; the low byte is its version.
pub const RECORD: u32 = 0x00CD_AC07;
/// The newest record version, the one every placed character's record has.
pub const RECORD_VERSION: u8 = 7;
/// Below this version the legacy reader skips relations as version 5 does.
const VERSION_6: u8 = 6;
/// The two sub-chunk tags of a thaumaturge's powers, read alike.
const POWERS: [u32; 2] = [0x00CD_EC01, 0x00CD_EC02];
/// A power entry with this flag carries a sized blob after it.
const POWER_BLOB: u32 = 0x1000_0000;
/// The powers sub-chunk the writer emits.
const POWERS_OUT: u32 = 0x00CD_EC01;
/// Power slots of the powers chunk.
pub const POWER_SLOTS: usize = 10;
/// Sub-chunk of the character's saved loadouts (hands and worn items).
const LOADOUTS: u32 = 0x00CD_EC00;
/// Sub-chunk of two words whose meaning is not traced (never shipped).
const RESERVED: u32 = 0x00CD_EC12;
/// Saved-state entry: red health still to heal from bandages (an `f32`).
pub const SAVED_HEAL_POOL: u32 = 0xCECC_0001;
/// Saved-state entry: a drunk potion's expiry on the game clock.
pub const SAVED_POTION: u32 = 0xCECC_0002;
/// A pool stored full (share 1).
pub const FULL_POOL: u16 = 0xFFFF;
/// Character traits are stored as bytes offset by this.
const TRAIT_BIAS: i32 = 100;
/// The word a new inventory entry carries after its item and flags.
pub const NEW_ENTRY_WORD: u32 = 0x0004_0028;
/// The voice name the writer builds: this prefix, then the number.
const VOICE_PREFIX: &str = "vi";
/// The item a fresh record wears: "Braies_A", the object database's
/// underwear.
pub const UNDERWEAR_ITEM: u32 = 8;
/// Skin colour of a fresh record (a mid grey).
pub const DEFAULT_SKIN: [u16; 2] = [0x8000, 0x8000];
/// Hair colour bytes of a fresh record.
pub const DEFAULT_HAIR_COLOUR: [u8; 4] = [0x40, 0x80, 0x40, 0x00];
/// Physique of a fresh record, both values.
pub const DEFAULT_PHYSIQUE: f32 = 0.5;

/// Skills a record holds, in slot order.
pub const SKILLS: usize = 6;
/// Techniques stored per skill (one per level). The writer emits at most
/// [`WRITTEN_TECHNIQUES`]; the reader takes the record's own byte count.
pub const TECHNIQUE_SLOTS: usize = 6;
/// The techniques the writer reads from a skill (it stops at the first
/// empty slot, so a sixth is never stored).
pub const WRITTEN_TECHNIQUES: usize = 5;
/// Skill points per level: each level learns one technique.
pub const POINTS_PER_TECHNIQUE: u16 = 5000;
/// The most levels a character's skills may add up to.
pub const MAX_TOTAL_LEVELS: u32 = 15;
/// Skill slots.
pub const CLOSE_COMBAT: usize = 0;
pub const RANGED_COMBAT: usize = 1;
pub const ARMOUR: usize = 2;
pub const SHIELDS: usize = 3;
pub const INSIGHT: usize = 4;
pub const CONCENTRATION: usize = 5;
pub const SKILL_NAMES: [&str; SKILLS] = [
    "Close Combat",
    "Ranged Combat",
    "Armour",
    "Shields",
    "Insight",
    "Concentration",
];
/// Each skill's techniques by id (1-based; an empty name has none). The
/// original's table also marks Draw, Impel, Brace and Absorption as not
/// available.
pub const TECHNIQUE_NAMES: [[&str; 8]; SKILLS] = [
    [
        "Remise",
        "Feint",
        "Fend",
        "Riposte",
        "Draw",
        "Impel",
        "Brace",
        "Dual Wield",
    ],
    ["", "", "", "", "", "", "", ""],
    [
        "Manoeuvring in Armour I",
        "Manoeuvring in Armour II",
        "Manoeuvring in Armour III",
        "Deflection",
        "Security",
        "Absorption",
        "",
        "",
    ],
    [
        "Counter",
        "Synchrony",
        "Ward",
        "Persist",
        "Endure",
        "Mobility",
        "",
        "",
    ],
    [
        "Inversion",
        "Expansion",
        "Convergence",
        "Union",
        "Coherence",
        "Iteration",
        "Revelation",
        "Amalgamation",
    ],
    [
        "Sustain",
        "Continuity",
        "Continuity II",
        "Continuity III",
        "Abstraction",
        "Clarity",
        "Resolve",
        "Volition",
    ],
];
/// The Armour techniques that lighten armour, one tier each.
const MANOEUVRING: std::ops::RangeInclusive<u8> = 1..=3;
const DEFLECTION: u8 = 4;

/// The name of a skill's technique, if it has one.
pub fn technique_name(skill: usize, technique: u8) -> Option<&'static str> {
    let name = TECHNIQUE_NAMES
        .get(skill)?
        .get(usize::from(technique).checked_sub(1)?)?;
    (!name.is_empty()).then_some(*name)
}

/// An item reference as a record stores it: the item's id and the flags
/// beside it. A record's hands, worn items, inventory, loadouts and its
/// roles' variables all use this pair. Id 0 is an empty slot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ItemRef {
    pub id: u32,
    pub flags: u32,
}

impl ItemRef {
    /// An empty slot.
    pub const NONE: Self = Self { id: 0, flags: 0 };

    pub const fn new(id: u32, flags: u32) -> Self {
        Self { id, flags }
    }

    /// Whether the slot holds an item.
    pub const fn is_some(self) -> bool {
        self.id != 0
    }

    /// The id without its database bits.
    pub const fn index(self) -> u32 {
        self.id & ID_MASK
    }

    /// The item is the level's own, in its local object database.
    pub const fn is_local(self) -> bool {
        self.id & LOCAL_ITEM != 0
    }

    /// The item is the game's, in the world object database.
    pub const fn is_world(self) -> bool {
        self.id & WORLD_DB != 0
    }

    /// The reference names a record of either object database.
    pub const fn is_shared(self) -> bool {
        self.id & (LOCAL_ITEM | WORLD_DB) != 0
    }
}

/// A character standing (or lying) in a level.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub id: u32,
    /// Feet position, metres.
    pub position: [f32; 3],
    /// Facing: the character looks along (sin yaw, 0, cos yaw).
    pub yaw: f32,
    pub condition: i32,
    /// Per-bone 3×4 matrices (rows, translation in metres) of a posed body,
    /// in depth-first order of the skeleton.
    pub pose: Vec<[[f32; 3]; 4]>,
}

impl Placement {
    pub fn is_dead(&self) -> bool {
        self.condition == DEAD
    }

    pub fn record(&self) -> u32 {
        self.id & ID_MASK
    }
}

/// A character's record: who they are, how they look, what they wear and
/// carry, their saved health, skills, roles, relations and powers.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterRecord {
    pub id: u32,
    /// Record version: 7 for every placed character; older records (5, 6)
    /// remain unplaced in some levels and in most arena actors.
    pub version: u8,
    pub name: String,
    pub surname: String,
    /// The character creator's two physique values.
    pub physique: [f32; 2],
    /// The creator's height, −1…1; positive is shorter.
    pub height: f32,
    /// The creator's age, 0…1.
    pub age: f32,
    /// Two skin colour words.
    pub skin: [u16; 2],
    /// Hair style.
    pub hair_style: u32,
    /// Hair colour, four bytes.
    pub hair_colour: [u8; 4],
    /// Two face style words.
    pub face: [u16; 2],
    /// The voice name as stored (`vi01`…`vi99`).
    pub voice: String,
    /// The voice number the name gives (`vi0N` → N − 1), at least 0.
    pub voice_number: u32,
    /// Voice pitch as stored (the character's init clamps it).
    pub voice_pitch: f32,
    pub female: bool,
    pub race: u8,
    /// The profession word's other bits, whose meaning is not traced (kept
    /// so a record writes back as it was read).
    pub profession_rest: u32,
    /// The last level the character was in.
    pub locale: u32,
    /// Four hand slots: two hands, then a second pair of hands.
    pub hands: [ItemRef; HANDS],
    /// Worn apparel items.
    pub worn: Vec<ItemRef>,
    /// How far gone an undead character is, 0…1 (0 for the living and for
    /// every race but humans). The name is the research notes' reading of
    /// what it does (weaker, looser, the undead voices).
    pub feral: f32,
    /// Health and focus as saved.
    pub pools: Pools,
    /// A random word made when the character is saved.
    pub seed: u16,
    /// The AI's fighting skill, 0…100.
    pub combat_skill: f32,
    /// Which undead voice a feral character uses.
    pub undead_voice: u32,
    /// Four personality traits, −100…100 (the AI reads the second and the
    /// third).
    pub traits: [f32; 4],
    /// A random seed the AI's search activity reads.
    pub search_seed: u32,
    pub experience: i32,
    /// A word the game keeps at zero in every shipped record (its reader
    /// does not use it).
    pub extra_word: u32,
    pub skills: [Skill; SKILLS],
    pub inventory: Vec<Carried>,
    /// Roles, newest first. A legacy record lists only the roles its reader
    /// adds; their parent roles come from the role database.
    pub roles: Vec<Role>,
    /// Relations toward other characters in this level.
    pub relations: Relations,
    /// Relations kept per other level.
    pub level_relations: Vec<RelationTable>,
    /// Saved body state (heal pool, potion), as stored.
    pub saved: Vec<SavedState>,
    /// The character's saved loadouts of hands and worn items.
    pub loadouts: Vec<Loadout>,
    /// A thaumaturge's powers.
    pub powers: Option<Powers>,
    /// The two words of the reserved sub-chunk, when a record has one.
    pub reserved: Option<[u32; 2]>,
}

/// Hand slots of a record or a loadout: two hands, then a second pair.
pub const HANDS: usize = 4;

impl Default for CharacterRecord {
    /// The original's new record: a blank identity, full health pools and
    /// pitch 1 ([`CharacterRecord::new`]).
    fn default() -> Self {
        Self::new()
    }
}

impl CharacterRecord {
    /// The original's constructor: a blank record whose yellow and red
    /// pools are full and whose voice pitch is 1. Everything else is zero,
    /// focus included.
    pub fn new() -> Self {
        Self {
            id: 0,
            version: RECORD_VERSION,
            name: String::new(),
            surname: String::new(),
            physique: [0.0; 2],
            height: 0.0,
            age: 0.0,
            skin: [0; 2],
            hair_style: 0,
            hair_colour: [0; 4],
            face: [0; 2],
            voice: String::new(),
            voice_number: 0,
            voice_pitch: 1.0,
            female: false,
            race: 0,
            profession_rest: 0,
            locale: 0,
            hands: [ItemRef::NONE; HANDS],
            worn: Vec::new(),
            feral: 0.0,
            pools: Pools::NEW,
            seed: 0,
            combat_skill: 0.0,
            undead_voice: 0,
            traits: [0.0; 4],
            search_seed: 0,
            experience: 0,
            extra_word: 0,
            skills: [Skill::default(); SKILLS],
            inventory: Vec::new(),
            roles: Vec::new(),
            relations: Relations::default(),
            level_relations: Vec::new(),
            saved: Vec::new(),
            loadouts: Vec::new(),
            powers: None,
            reserved: None,
        }
    }

    /// The defaults the character creator starts from: a new record
    /// ([`CharacterRecord::new`]) with [`CharacterRecord::apply_creator_defaults`].
    pub fn reset() -> Self {
        let mut record = Self::new();
        record.apply_creator_defaults();
        record
    }

    /// The creator's reset of an existing record: a male human (the whole
    /// profession word cleared) of average physique, the default skin and
    /// hair, voice number 0 and pitch 1, full yellow and red health, and
    /// the underwear appended to what it wears. Every other field, focus
    /// included, keeps its value.
    pub fn apply_creator_defaults(&mut self) {
        self.set_profession(0);
        self.physique = [DEFAULT_PHYSIQUE; 2];
        self.skin = DEFAULT_SKIN;
        self.hair_style = 0;
        self.hair_colour = DEFAULT_HAIR_COLOUR;
        self.voice_number = 0;
        self.voice = voice_name(0);
        self.voice_pitch = 1.0;
        self.pools.yellow = FULL_POOL;
        self.pools.red = FULL_POOL;
        self.worn.push(ItemRef::new(UNDERWEAR_ITEM, 0));
    }

    /// A copy made as the game copies a record: written, read back, and
    /// given no database id. The copy has the writer's canonical voice name
    /// and loses what the writer drops (a skill without a technique, a
    /// sixth technique, powers without an entry, a reserved pair whose first
    /// word is 0).
    pub fn duplicate(&self) -> Result<Self> {
        let bytes = self.to_bytes()?;
        Self::parse(0, &bytes)
    }

    /// The profession word: bit 0 female, byte 1 the race, the other bits
    /// kept as stored.
    pub fn profession(&self) -> u32 {
        self.profession_rest | u32::from(self.female) | u32::from(self.race) << 8
    }

    /// Sets sex and race from the profession word, as the reader does.
    pub fn set_profession(&mut self, profession: u32) {
        self.female = profession & 1 != 0;
        self.race = (profession >> 8) as u8;
        self.profession_rest = profession & !0xFF01;
    }

    /// The worn item ids, without their flags.
    pub fn worn_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.worn.iter().map(|w| w.id)
    }

    /// The ids of the hand items held (slots with an item).
    pub fn held_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.hands.iter().filter(|h| h.is_some()).map(|h| h.id)
    }

    /// The first hand's item, the one a spawn puts in the right hand.
    pub fn main_hand(&self) -> Option<ItemRef> {
        self.hands[0].is_some().then_some(self.hands[0])
    }

    /// The perks the learnt techniques give.
    pub fn perks(&self) -> Perks {
        let mut perks = Perks::default();
        for perk in Perk::ALL {
            let (skill, technique) = perk.source();
            if self.skills[skill].knows(technique) {
                perks.insert(perk);
            }
        }
        perks
    }

    /// The highest "Manoeuvring in Armour" tier learnt (0–3); each tier
    /// takes 0.3 off the armour's encumbrance.
    pub fn manoeuvring_tier(&self) -> u8 {
        self.skills[ARMOUR]
            .learnt()
            .iter()
            .copied()
            .filter(|t| MANOEUVRING.contains(t))
            .max()
            .unwrap_or(0)
    }

    /// Whether Deflection is learnt (worn armour covers more).
    pub fn deflection(&self) -> bool {
        self.skills[ARMOUR].knows(DEFLECTION)
    }

    /// Red health still to heal from bandages, as saved in its own entry.
    pub fn heal_pool(&self) -> Option<f32> {
        self.saved_value(SAVED_HEAL_POOL).map(f32::from_bits)
    }

    /// A drunk potion's expiry on the game clock, as saved in its own entry.
    pub fn potion_expiry(&self) -> Option<u32> {
        self.saved_value(SAVED_POTION)
    }

    fn saved_value(&self, tag: u32) -> Option<u32> {
        self.saved.iter().find(|s| s.tag == tag).map(|s| s.value)
    }

    /// How many relations the record keeps in its own level.
    pub fn relation_count(&self) -> usize {
        self.relations.entries.len()
    }
}

/// The voice name a number makes: `vi` and the number plus one in
/// decimal, padded to two digits (`vi01`…`vi99`, then `vi100`…), as the
/// game's own number formatter writes it. The sum is a 32-bit signed
/// integer: 0 gives `00` and a negative sum the single digit `0`.
pub fn voice_name(number: u32) -> String {
    let next = (number as i32).wrapping_add(1);
    let digits = match next {
        0 => "00".to_string(),
        n if n > 0 => format!("{n:02}"),
        _ => "0".to_string(),
    };
    format!("{VOICE_PREFIX}{digits}")
}

/// The voice number a stored name gives (`vi0N` → N − 1, at least 0), else
/// 0. Names hold one character per stored byte.
pub fn voice_number_of(name: &[u8]) -> u32 {
    match (name.get(2), name.get(3)) {
        (Some(b'0'), Some(n)) => n.saturating_sub(b'1') as u32,
        _ => 0,
    }
}

/// The double-precision unit the load multiplies a stored share by
/// (1/65535).
const POOL_UNIT: f64 = 1.0 / 65535.0;

/// A float rounded to an integer as the game's SSE conversion does it: to
/// the nearest integer, ties to even. A value the conversion cannot hold
/// (NaN, infinite, outside the `i64` range) gives `i64::MIN`, the
/// instruction's "integer indefinite".
pub fn round_even_i64(value: f32) -> i64 {
    let rounded = f64::from(value).round_ties_even();
    // 2^63: the first value past the range.
    const LIMIT: f64 = 9_223_372_036_854_775_808.0;
    if rounded.is_nan() || !(-LIMIT..LIMIT).contains(&rounded) {
        i64::MIN
    } else {
        rounded as i64
    }
}

/// Health and focus saved as 16-bit shares: yellow (current health) and red
/// (the ceiling it heals back to) of the character's maximum, and focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pools {
    pub yellow: u16,
    pub red: u16,
    pub focus: u16,
}

impl Pools {
    pub const FULL: Self = Self {
        yellow: FULL_POOL,
        red: FULL_POOL,
        focus: FULL_POOL,
    };

    /// The pools of a new record: the constructor fills yellow and red and
    /// leaves focus at 0.
    pub const NEW: Self = Self {
        yellow: FULL_POOL,
        red: FULL_POOL,
        focus: 0,
    };

    /// A stored yellow or red share loaded onto a body of maximum health
    /// `max`, as the load and the spawn compute it: in double precision,
    /// `max × (stored × 1/65535)`, rounded once to `f32`.
    pub fn restore(stored: u16, max: f32) -> f32 {
        (f64::from(max) * (f64::from(stored) * POOL_UNIT)) as f32
    }

    /// The stored focus share as the load sets it: `stored × 1/65535` in
    /// double precision, rounded once to `f32`.
    pub fn restore_focus(stored: u16) -> f32 {
        (f64::from(stored) * POOL_UNIT) as f32
    }

    /// A share saved as the game saves it: `value` (already scaled to
    /// 0…65535) rounded to the nearest even integer and cut to 16 bits;
    /// a value the conversion cannot hold (NaN, infinite, out of range)
    /// stores 0.
    pub fn save(value: f32) -> u16 {
        round_even_i64(value) as u16
    }

    /// Yellow, red and focus as shares (stored / 65535), as the load reads
    /// them; yellow and red are then multiplied by the maximum health.
    pub fn shares(self) -> [f32; 3] {
        [self.yellow, self.red, self.focus].map(|v| f32::from(v) / f32::from(FULL_POOL))
    }

    pub fn is_full(self) -> bool {
        self == Self::FULL
    }
}

/// One skill: its points and the technique chosen at each level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Skill {
    pub points: u16,
    /// Technique ids (1-based, see [`TECHNIQUE_NAMES`]); 0 = none chosen.
    pub techniques: [u8; TECHNIQUE_SLOTS],
}

impl Skill {
    /// The techniques learnt: one per [`POINTS_PER_TECHNIQUE`] points, at
    /// most the six a skill stores (the original would read past them).
    pub fn learnt(&self) -> &[u8] {
        let levels = usize::from(self.points / POINTS_PER_TECHNIQUE).min(TECHNIQUE_SLOTS);
        &self.techniques[..levels]
    }

    /// Whether a technique is among the learnt ones (what the perks read).
    pub fn knows(&self, technique: u8) -> bool {
        technique != 0 && self.learnt().contains(&technique)
    }

    /// The record's own technique check, as the arena and the powers tree
    /// ask it: the first of the five written slots that holds `technique`
    /// counts when the points fund its level (level i needs i × 5000). A
    /// sixth slot is never looked at, and 0 matches an empty slot.
    pub fn has_technique(&self, technique: u8) -> bool {
        self.techniques[..WRITTEN_TECHNIQUES]
            .iter()
            .position(|&t| t == technique)
            .is_some_and(|at| {
                u32::from(self.points) >= (at as u32 + 1) * u32::from(POINTS_PER_TECHNIQUE)
            })
    }

    /// The levels the skill's points fund.
    pub fn levels(&self) -> u32 {
        u32::from(self.points / POINTS_PER_TECHNIQUE)
    }

    /// How many techniques are chosen in a row from the first slot, at most
    /// the five the writer stores.
    pub fn chosen(&self) -> usize {
        self.techniques[..WRITTEN_TECHNIQUES]
            .iter()
            .position(|&t| t == 0)
            .unwrap_or(WRITTEN_TECHNIQUES)
    }
}

/// A technique that changes how the character's body, combat or powers
/// work. Each comes from one skill's technique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Perk {
    Remise,
    Feint,
    Fend,
    Riposte,
    Security,
    Counter,
    Synchrony,
    Ward,
    Persist,
    Endure,
    Mobility,
    Iteration,
    Sustain,
    Abstraction,
    Clarity,
    Volition,
}

impl Perk {
    pub const ALL: [Perk; 16] = [
        Perk::Remise,
        Perk::Feint,
        Perk::Fend,
        Perk::Riposte,
        Perk::Security,
        Perk::Counter,
        Perk::Synchrony,
        Perk::Ward,
        Perk::Persist,
        Perk::Endure,
        Perk::Mobility,
        Perk::Iteration,
        Perk::Sustain,
        Perk::Abstraction,
        Perk::Clarity,
        Perk::Volition,
    ];

    /// The skill and the technique id that grant it.
    pub const fn source(self) -> (usize, u8) {
        match self {
            Perk::Remise => (CLOSE_COMBAT, 1),
            Perk::Feint => (CLOSE_COMBAT, 2),
            Perk::Fend => (CLOSE_COMBAT, 3),
            Perk::Riposte => (CLOSE_COMBAT, 4),
            Perk::Security => (ARMOUR, 5),
            Perk::Counter => (SHIELDS, 1),
            Perk::Synchrony => (SHIELDS, 2),
            Perk::Ward => (SHIELDS, 3),
            Perk::Persist => (SHIELDS, 4),
            Perk::Endure => (SHIELDS, 5),
            Perk::Mobility => (SHIELDS, 6),
            Perk::Iteration => (INSIGHT, 6),
            Perk::Sustain => (CONCENTRATION, 1),
            Perk::Abstraction => (CONCENTRATION, 5),
            Perk::Clarity => (CONCENTRATION, 6),
            Perk::Volition => (CONCENTRATION, 8),
        }
    }

    pub fn name(self) -> &'static str {
        let (skill, technique) = self.source();
        technique_name(skill, technique).unwrap_or_default()
    }

    const fn bit(self) -> u32 {
        1 << self as u32
    }
}

/// The perks a character has.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Perks(u32);

impl Perks {
    /// The perk of a technique, when the technique grants one.
    pub fn of(skill: usize, technique: u8) -> Option<Perk> {
        Perk::ALL
            .into_iter()
            .find(|p| p.source() == (skill, technique))
    }

    pub fn has(self, perk: Perk) -> bool {
        self.0 & perk.bit() != 0
    }

    pub fn insert(&mut self, perk: Perk) {
        self.0 |= perk.bit();
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = Perk> {
        Perk::ALL.into_iter().filter(move |&p| self.has(p))
    }
}

/// An item in the inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Carried {
    /// Item id (`LOCAL_ITEM` = the level's object database); 0 = empty.
    pub item: u32,
    pub flags: u32,
    /// A third word: [`NEW_ENTRY_WORD`] on an entry the game appends, its
    /// meaning not traced.
    pub extra: u32,
}

impl Default for Carried {
    fn default() -> Self {
        Self {
            item: 0,
            flags: 0,
            extra: NEW_ENTRY_WORD,
        }
    }
}

/// A role held by the character (`charroles.rdb`).
#[derive(Debug, Clone, PartialEq)]
pub struct Role {
    pub id: u32,
    /// The weight as stored (the role database's own weight is what the
    /// game uses).
    pub weight: f32,
    /// Assigned explicitly, not only pulled in as a parent.
    pub explicit: bool,
    /// How many explicit roles pulled this one in as a parent.
    pub inherited: u8,
    /// The role's variables, in its declaration order.
    pub variables: Vec<u32>,
}

/// A relation table toward other characters: 8-byte entries (a character
/// and an offset) and a blob whose layout is not traced. Empty (`n < 1`)
/// records keep nothing after the count.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Relations {
    /// Pairs of words, one per relation.
    pub entries: Vec<(u32, u32)>,
    /// The bytes that follow the entries.
    pub blob: Vec<u8>,
}

impl Relations {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.blob.is_empty()
    }

    /// The entry pairs, as words.
    pub fn pairs(&self) -> impl Iterator<Item = [u32; 2]> + '_ {
        self.entries.iter().map(|&(a, b)| [a, b])
    }
}

/// A saved loadout: four hand slots and the items worn with them. The same
/// shape the record's hand and worn fields have, plus a leading word.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Loadout {
    /// A word whose meaning is not traced (its tag or index).
    pub word: u32,
    pub hands: [ItemRef; HANDS],
    pub worn: Vec<ItemRef>,
}

/// A relation table kept for another level: two words (the level and an
/// offset) and the relations toward its characters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelationTable {
    pub words: [u32; 2],
    pub relations: Relations,
}

impl Loadout {
    /// The record's hands and worn items in a new loadout (word 0).
    pub fn of_record(record: &CharacterRecord) -> Self {
        Self {
            word: 0,
            hands: record.hands,
            worn: record.worn.clone(),
        }
    }

    /// Stores the record's hands and worn items in this loadout, as the
    /// game saves them into a loadout it keeps: its word stays.
    pub fn store(&mut self, record: &CharacterRecord) {
        self.hands = record.hands;
        self.worn = record.worn.clone();
    }

    /// Copies another loadout's hands and worn items into this one; its
    /// word stays.
    pub fn copy_from(&mut self, from: &Loadout) {
        self.hands = from.hands;
        self.worn = from.worn.clone();
    }

    /// Puts the loadout's hands and worn items on a record, as the arena
    /// does (and then ensures the underwear).
    pub fn apply_to(&self, record: &mut CharacterRecord) {
        record.hands = self.hands;
        record.worn = self.worn.clone();
    }
}

/// A saved body-state entry: a tag and its value word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedState {
    pub tag: u32,
    pub value: u32,
}

/// A thaumaturge's powers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Powers {
    /// A leading word whose meaning is not traced.
    pub word: u32,
    pub slots: [u32; POWER_SLOTS],
    pub known: Vec<Power>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Power {
    pub id: u32,
    pub flags: u32,
    /// The bytes after a power flagged [`POWER_BLOB`], without the size
    /// word the file stores (`Some` of nothing for a blob of size 4).
    /// Writing sets the flag exactly when there is a blob.
    pub blob: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LevelCharacters {
    pub records: Vec<CharacterRecord>,
    pub placements: Vec<Placement>,
}

impl LevelCharacters {
    pub fn record(&self, id: u32) -> Option<&CharacterRecord> {
        self.records.iter().find(|r| r.id == id & ID_MASK)
    }

    /// Parses a level's character section. Database entries that are not
    /// records (deleted ones, stubs) are left out; see
    /// [`LevelCharacters::read_database`].
    pub fn parse(b: &[u8]) -> Result<Self> {
        let (count, sections) = sections(b)?;
        let mut out = Self::default();
        for (tag, body) in sections {
            match tag {
                CHARACTER_DB => {
                    out.records = read_database(body)?
                        .into_iter()
                        .filter_map(|(_, rec)| rec.ok())
                        .collect();
                }
                PLACEMENT => out.placements.push(read_placement(body)?),
                _ => {}
            }
        }
        if out.placements.len() > count {
            out.placements.truncate(count);
        }
        Ok(out)
    }

    /// Every entry of a level section's character database, by id, with its
    /// parse: a deleted entry (empty) or a stub that is not a record comes
    /// back as an error.
    pub fn read_database(b: &[u8]) -> Result<Vec<(u32, Result<CharacterRecord>)>> {
        let mut out = Vec::new();
        for (tag, body) in sections(b)?.1 {
            if tag == CHARACTER_DB {
                out = read_database(body)?;
            }
        }
        Ok(out)
    }
}

/// A tagged sub-chunk of a character section.
type Section<'a> = (u32, &'a [u8]);

/// A character section: `i32 n` placements, then tagged sub-chunks.
fn sections(b: &[u8]) -> Result<(usize, Vec<Section<'_>>)> {
    let mut r = Reader { b, pos: 0 };
    let count = r.i32()?.max(0) as usize;
    let mut out = Vec::new();
    while r.remaining() >= 8 {
        let tag = r.u32()?;
        let size = r.u32()? as usize;
        out.push((tag, r.bytes(size)?));
    }
    Ok((count, out))
}

/// Tag of the item database an actor's record is followed by.
const ACTOR_ITEMS: u32 = 0xDBDB;

/// The items of the database that follows an actor's record (tag, size,
/// then an RDB of item records like `objdb.rdb`'s): what its
/// [`LOCAL_ITEM`] ids name. None when the record ends the entry (the
/// demon's); an error when something else follows.
pub fn actor_items(b: &[u8]) -> Result<Vec<crate::objdb::Item>> {
    let mut r = Reader { b, pos: 0 };
    r.u32()?;
    let size = r.u32()? as usize;
    r.bytes(size)?;
    if r.remaining() == 0 {
        return Ok(Vec::new());
    }
    let tag = r.u32()?;
    if tag != ACTOR_ITEMS {
        return Err(Error::BadMagic {
            found: tag,
            expected: ACTOR_ITEMS,
        });
    }
    let size = r.u32()? as usize;
    Ok(crate::objdb::items(&Database::parse(r.bytes(size)?)?))
}

fn read_database(b: &[u8]) -> Result<Vec<(u32, Result<CharacterRecord>)>> {
    let db = Database::parse(b)?;
    Ok(db
        .records
        .iter()
        .map(|rec| (rec.id, CharacterRecord::parse(rec.id, db.payload(rec))))
        .collect())
}

fn read_placement(b: &[u8]) -> Result<Placement> {
    let mut r = Reader { b, pos: 0 };
    let id = r.u32()?;
    let position = [r.f32()?, r.f32()?, r.f32()?].map(|v| v / CENTIMETRES_PER_METRE);
    let yaw = r.f32()?;
    let condition = r.i32()?;
    let mut pose = Vec::new();
    while r.remaining() >= 8 {
        let tag = r.u32()?;
        let size = r.u32()? as usize;
        let body = r.bytes(size)?;
        match tag {
            POSE => read_bone_tree(&mut Reader { b: body, pos: 0 }, &mut pose)?,
            LEGACY_POSE if pose.is_empty() => {
                let mut p = Reader { b: body, pos: 0 };
                while p.remaining() >= 48 {
                    pose.push(p.matrix()?);
                }
            }
            _ => {}
        }
    }
    Ok(Placement {
        id,
        position,
        yaw,
        condition,
        pose,
    })
}

/// A bone and its children: matrix, a word, child count, children.
fn read_bone_tree(r: &mut Reader<'_>, out: &mut Vec<[[f32; 3]; 4]>) -> Result<()> {
    out.push(r.matrix()?);
    r.u32()?;
    let children = r.i32()?;
    if !(0..=64).contains(&children) || out.len() > 256 {
        return Err(Error::Malformed(format!(
            "pose node with {children} children"
        )));
    }
    for _ in 0..children {
        read_bone_tree(r, out)?;
    }
    Ok(())
}

impl CharacterRecord {
    /// Parses a whole record (tag, size, then the record), versions 5–7.
    /// Bytes after the record (an actor's item database) are not read.
    pub fn parse(id: u32, b: &[u8]) -> Result<Self> {
        let mut outer = Reader { b, pos: 0 };
        let tag = outer.u32()?;
        if tag & 0xFFFF_FF00 != RECORD & 0xFFFF_FF00 {
            return Err(Error::BadMagic {
                found: tag,
                expected: RECORD,
            });
        }
        let version = (tag & 0xFF) as u8;
        if version > RECORD_VERSION {
            return Err(Error::Unsupported(format!(
                "character record version {version}"
            )));
        }
        let size = outer.u32()? as usize;
        let mut r = Reader {
            b: outer.bytes(size)?,
            pos: 0,
        };
        let mut rec = Self {
            id,
            version,
            ..Self::new()
        };
        rec.read_common(&mut r)?;
        if version == RECORD_VERSION {
            rec.roles = read_roles(&mut r)?;
            rec.relations = read_relations(&mut r)?;
            for _ in 0..r.count("level relation tables")? {
                let words = [r.u32()?, r.u32()?];
                rec.level_relations.push(RelationTable {
                    words,
                    relations: read_relations(&mut r)?,
                });
            }
        } else {
            rec.roles = read_legacy_roles(&mut r)?;
            skip_legacy_relations(&mut r, version)?;
        }
        let saved = r.count("saved state entries")?;
        rec.saved = r
            .entries::<8>(saved)?
            .iter()
            .map(|e| SavedState {
                tag: word(&e[..4]),
                value: word(&e[4..]),
            })
            .collect();
        while r.remaining() > 0 {
            let tag = r.u32()?;
            let size = r.count("sub-chunk bytes")?;
            let mut chunk = Reader {
                b: r.bytes(size)?,
                pos: 0,
            };
            match tag {
                t if POWERS.contains(&t) => {
                    rec.powers = Some(read_powers(&mut chunk, version == RECORD_VERSION)?);
                }
                LOADOUTS => rec.loadouts = read_loadouts(&mut chunk)?,
                RESERVED => rec.reserved = Some([chunk.u32()?, chunk.u32()?]),
                _ => {}
            }
        }
        Ok(rec)
    }

    /// The fields every version shares, up to and including the inventory.
    fn read_common(&mut self, r: &mut Reader<'_>) -> Result<()> {
        self.name = r.name()?;
        self.surname = r.name()?;
        self.physique = [r.f32()?, r.f32()?];
        self.height = r.f32()?;
        self.age = r.f32()?;
        self.skin = [r.u16()?, r.u16()?];
        self.hair_style = r.u32()?;
        self.hair_colour = r.bytes(4)?.try_into().expect("four bytes");
        self.face = [r.u16()?, r.u16()?];
        let voice = r.bytes(16)?;
        self.voice_number = voice_number_of(voice);
        self.voice = latin1(voice);
        self.voice_pitch = r.f32()?;
        let profession = r.u32()?;
        self.set_profession(profession);
        self.locale = r.u32()?;
        for hand in &mut self.hands {
            *hand = ItemRef::new(r.u32()?, r.u32()?);
        }
        let worn = r.count("worn items")?;
        self.worn = r
            .entries::<8>(worn)?
            .iter()
            .map(|e| ItemRef::new(word(&e[..4]), word(&e[4..])))
            .collect();
        self.feral = r.f32()?;
        self.pools = Pools {
            yellow: r.u16()?,
            red: r.u16()?,
            focus: r.u16()?,
        };
        self.seed = r.u16()?;
        self.combat_skill = r.f32()?;
        self.undead_voice = r.u32()?;
        for t in &mut self.traits {
            *t = f32::from(r.u8()?) - TRAIT_BIAS as f32;
        }
        self.search_seed = r.u32()?;
        self.experience = r.i32()?;
        self.extra_word = r.u32()?;
        self.skills = read_skills(r)?;
        let carried = r.count("carried items")?;
        self.inventory = r
            .entries::<12>(carried)?
            .iter()
            .map(|e| Carried {
                item: word(&e[..4]),
                flags: word(&e[4..8]),
                extra: word(&e[8..]),
            })
            .collect();
        Ok(())
    }

    /// The voice number the voice name gives (`vi0N` → N − 1, at least 0),
    /// else 0.
    pub fn voice_index(&self) -> u32 {
        self.voice_number
    }

    /// Writes the record as the game saves it: tag `0x00CDAC07`, the size,
    /// then the fields in their file order, sub-chunks last. Only version 7
    /// is written.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        self.write(&mut out)?;
        Ok(out)
    }

    /// [`CharacterRecord::to_bytes`] into `out`.
    pub fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        let start = out.len();
        out.extend_from_slice(&RECORD.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        self.write_body(out)?;
        let size = u32::try_from(out.len() - start - 8)
            .map_err(|_| Error::Malformed("character record too large".into()))?;
        out[start + 4..start + 8].copy_from_slice(&size.to_le_bytes());
        Ok(())
    }

    fn write_body(&self, out: &mut Vec<u8>) -> Result<()> {
        put_name(out, &self.name)?;
        put_name(out, &self.surname)?;
        for v in self.physique {
            put_f32(out, v);
        }
        put_f32(out, self.height);
        put_f32(out, self.age);
        for v in self.skin {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&self.hair_style.to_le_bytes());
        out.extend_from_slice(&self.hair_colour);
        for v in self.face {
            out.extend_from_slice(&v.to_le_bytes());
        }
        // The stored name comes from the voice number, not the name read.
        put_name(out, &voice_name(self.voice_number))?;
        put_f32(out, self.voice_pitch);
        out.extend_from_slice(&self.profession().to_le_bytes());
        out.extend_from_slice(&self.locale.to_le_bytes());
        for hand in self.hands {
            put_ref(out, hand);
        }
        put_count(out, self.worn.len())?;
        for worn in &self.worn {
            put_ref(out, *worn);
        }
        put_f32(out, self.feral);
        out.extend_from_slice(&self.pools.yellow.to_le_bytes());
        out.extend_from_slice(&self.pools.red.to_le_bytes());
        out.extend_from_slice(&self.pools.focus.to_le_bytes());
        out.extend_from_slice(&self.seed.to_le_bytes());
        put_f32(out, self.combat_skill);
        out.extend_from_slice(&self.undead_voice.to_le_bytes());
        for t in self.traits {
            // The game converts to an integer (nearest, ties to even; NaN or
            // out of range gives the conversion's indefinite value, whose
            // low byte is 0) and adds the bias in a byte.
            let biased = round_even_i64(t).wrapping_add(i64::from(TRAIT_BIAS)) & 0xFF;
            out.push(biased as u8);
        }
        out.extend_from_slice(&self.search_seed.to_le_bytes());
        out.extend_from_slice(&self.experience.to_le_bytes());
        out.extend_from_slice(&self.extra_word.to_le_bytes());
        self.write_skills(out)?;
        put_count(out, self.inventory.len())?;
        for carried in &self.inventory {
            out.extend_from_slice(&carried.item.to_le_bytes());
            out.extend_from_slice(&carried.flags.to_le_bytes());
            out.extend_from_slice(&carried.extra.to_le_bytes());
        }
        let blob = roles_blob(&self.roles)?;
        put_count(out, self.roles.len())?;
        put_count(out, blob.len())?;
        out.extend_from_slice(&blob);
        put_relations(out, &self.relations)?;
        put_count(out, self.level_relations.len())?;
        for level in &self.level_relations {
            put_u32(out, level.words[0]);
            put_u32(out, level.words[1]);
            put_relations(out, &level.relations)?;
        }
        put_count(out, self.saved.len())?;
        for saved in &self.saved {
            out.extend_from_slice(&saved.tag.to_le_bytes());
            out.extend_from_slice(&saved.value.to_le_bytes());
        }
        // The writer emits the powers only when they have an entry.
        if let Some(powers) = self.powers.as_ref().filter(|p| !p.known.is_empty()) {
            put_chunk(out, POWERS_OUT, |c| {
                c.extend_from_slice(&powers.word.to_le_bytes());
                for slot in powers.slots {
                    c.extend_from_slice(&slot.to_le_bytes());
                }
                put_count(c, powers.known.len())?;
                for power in &powers.known {
                    c.extend_from_slice(&power.id.to_le_bytes());
                    match &power.blob {
                        None => c.extend_from_slice(&(power.flags & !POWER_BLOB).to_le_bytes()),
                        Some(blob) => {
                            c.extend_from_slice(&(power.flags | POWER_BLOB).to_le_bytes());
                            let size = u32::try_from(blob.len() + 4)
                                .map_err(|_| Error::Malformed("power blob too large".into()))?;
                            c.extend_from_slice(&size.to_le_bytes());
                            c.extend_from_slice(blob);
                        }
                    }
                }
                Ok(())
            })?;
        }
        // The reserved pair is written only when its first word is set.
        if let Some([a, b]) = self.reserved.filter(|[a, _]| *a != 0) {
            put_chunk(out, RESERVED, |c| {
                c.extend_from_slice(&a.to_le_bytes());
                c.extend_from_slice(&b.to_le_bytes());
                Ok(())
            })?;
        }
        if !self.loadouts.is_empty() {
            put_chunk(out, LOADOUTS, |c| {
                put_count(c, self.loadouts.len())?;
                for loadout in &self.loadouts {
                    c.extend_from_slice(&loadout.word.to_le_bytes());
                    for hand in loadout.hands {
                        put_ref(c, hand);
                    }
                    put_count(c, loadout.worn.len())?;
                    for worn in &loadout.worn {
                        put_ref(c, *worn);
                    }
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    /// The skills a record stores: one entry per skill that has a technique,
    /// with the points and the techniques up to the first empty slot.
    fn write_skills(&self, out: &mut Vec<u8>) -> Result<()> {
        let entries: Vec<(usize, &Skill)> = self
            .skills
            .iter()
            .enumerate()
            .filter(|(_, s)| s.chosen() > 0)
            .collect();
        put_count(out, entries.len())?;
        for (slot, skill) in entries {
            out.extend_from_slice(&(slot as u16).to_le_bytes());
            out.extend_from_slice(&skill.points.to_le_bytes());
            let k = skill.chosen().min(WRITTEN_TECHNIQUES);
            out.push(k as u8);
            out.extend_from_slice(&skill.techniques[..k]);
        }
        Ok(())
    }
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_ref(out: &mut Vec<u8>, item: ItemRef) {
    put_u32(out, item.id);
    put_u32(out, item.flags);
}

/// A signed count; a list longer than the format's `i32` is an error.
fn put_count(out: &mut Vec<u8>, n: usize) -> Result<()> {
    let n = i32::try_from(n).map_err(|_| Error::Malformed(format!("{n} entries")))?;
    out.extend_from_slice(&n.to_le_bytes());
    Ok(())
}

/// A name field: 16 bytes, one per character as the reader maps them
/// (a character past U+00FF, which no stored byte gives, becomes `?`),
/// cut at 16 and padded with zeros.
fn put_name(out: &mut Vec<u8>, name: &str) -> Result<()> {
    let bytes: Vec<u8> = name
        .chars()
        .take(16)
        .map(|c| u8::try_from(c).unwrap_or(b'?'))
        .collect();
    out.extend_from_slice(&bytes);
    out.resize(out.len() + (16 - bytes.len()), 0);
    Ok(())
}

/// A tagged sub-chunk: tag, size, then what `body` writes.
fn put_chunk(
    out: &mut Vec<u8>,
    tag: u32,
    body: impl FnOnce(&mut Vec<u8>) -> Result<()>,
) -> Result<()> {
    put_u32(out, tag);
    let start = out.len();
    put_u32(out, 0);
    body(out)?;
    let size = u32::try_from(out.len() - start - 4)
        .map_err(|_| Error::Malformed("sub-chunk too large".into()))?;
    out[start..start + 4].copy_from_slice(&size.to_le_bytes());
    Ok(())
}

/// The tagged blob the roles are stored as: per role 12 bytes (id, weight,
/// flags, inherited count, variable count, a pad byte) and its variables.
fn roles_blob(roles: &[Role]) -> Result<Vec<u8>> {
    let mut blob = Vec::new();
    for role in roles {
        put_u32(&mut blob, role.id);
        put_f32(&mut blob, role.weight);
        let flags = u8::from(role.explicit);
        let n = u8::try_from(role.variables.len()).map_err(|_| {
            Error::Malformed(format!("role with {} variables", role.variables.len()))
        })?;
        blob.extend_from_slice(&[flags, role.inherited, n, 0]);
        for v in &role.variables {
            put_u32(&mut blob, *v);
        }
    }
    Ok(blob)
}

fn put_relations(out: &mut Vec<u8>, relations: &Relations) -> Result<()> {
    put_count(out, relations.entries.len())?;
    if relations.entries.is_empty() {
        return Ok(());
    }
    for &(a, b) in &relations.entries {
        put_u32(out, a);
        put_u32(out, b);
    }
    put_count(out, relations.blob.len())?;
    out.extend_from_slice(&relations.blob);
    Ok(())
}

/// A fixed name field as text: up to its first zero byte, one byte per
/// character.
fn latin1(raw: &[u8]) -> String {
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    raw[..end].iter().map(|&c| c as char).collect()
}

fn word(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// `i32 n`, then n × {u16 slot, u16 points, u8 k, k technique bytes}. A slot
/// past the six has no room in the record (the original's next field
/// overwrites it) and is skipped.
fn read_skills(r: &mut Reader<'_>) -> Result<[Skill; SKILLS]> {
    let mut skills = [Skill::default(); SKILLS];
    for _ in 0..r.count("skills")? {
        let slot = usize::from(r.u16()?);
        let points = r.u16()?;
        let k = usize::from(r.u8()?);
        if k > TECHNIQUE_SLOTS {
            return Err(Error::Malformed(format!("skill with {k} techniques")));
        }
        let techniques = r.bytes(k)?;
        if let Some(skill) = skills.get_mut(slot) {
            skill.points = points;
            skill.techniques[..k].copy_from_slice(techniques);
        }
    }
    Ok(skills)
}

/// `u32 count`, `u32 len`, then `len` bytes holding `count` entries of
/// {u32 id, f32 weight, u8 flags, u8 inherited, u8 n, u8 pad, n × u32}.
fn read_roles(r: &mut Reader<'_>) -> Result<Vec<Role>> {
    let count = r.u32()? as usize;
    let len = r.u32()? as usize;
    let mut blob = Reader {
        b: r.bytes(len)?,
        pos: 0,
    };
    let mut roles = Vec::new();
    for _ in 0..count {
        let id = blob.u32()?;
        let weight = blob.f32()?;
        let flags = blob.u8()?;
        let inherited = blob.u8()?;
        let n = usize::from(blob.u8()?);
        blob.u8()?;
        let variables = blob.entries::<4>(n)?.iter().map(|e| word(e)).collect();
        roles.push(Role {
            id,
            weight,
            explicit: flags & 1 != 0,
            inherited,
            variables,
        });
    }
    if blob.remaining() != 0 {
        return Err(Error::Malformed(format!(
            "{} bytes after {count} roles",
            blob.remaining()
        )));
    }
    Ok(roles)
}

/// Legacy roles: `i32 n`, n × {u32 id, u32 ignored}, each added in turn.
/// Three old first roles are replaced by their successors. The reader
/// looks at the first entry's id even when `n` is 0, so then the word after
/// the count (the first relation word) is what it compares.
fn read_legacy_roles(r: &mut Reader<'_>) -> Result<Vec<Role>> {
    /// Old first role → the two roles added instead.
    const REPLACED: [(u32, [u32; 2]); 3] = [(13, [8, 33]), (27, [26, 33]), (22, [9, 33])];
    let n = r.count("roles")?;
    let first = r.peek_u32();
    let ids: Vec<u32> = r.entries::<8>(n)?.iter().map(|e| word(e)).collect();
    let added = match first.and_then(|first| REPLACED.iter().find(|(old, _)| *old == first)) {
        Some((_, new)) => new.to_vec(),
        None => ids,
    };
    let mut roles = Vec::new();
    for id in added {
        add_legacy_role(&mut roles, id);
    }
    Ok(roles)
}

/// Adds a role as the legacy reader does, without the role database's
/// parents (`charroles.rdb` is not read here): a role already held
/// explicitly is left as it is, one held only as a parent becomes
/// explicit, and a new one goes first (newest first).
fn add_legacy_role(roles: &mut Vec<Role>, id: u32) {
    match roles.iter_mut().find(|role| role.id == id) {
        Some(role) => role.explicit = true,
        None => roles.insert(
            0,
            Role {
                id,
                weight: 0.0,
                explicit: true,
                inherited: 0,
                variables: Vec::new(),
            },
        ),
    }
}

/// A relation table: `i32 n`; if n ≥ 1, n × {u32 character, u32 offset},
/// `i32 len` and the `len`-byte blob.
fn read_relations(r: &mut Reader<'_>) -> Result<Relations> {
    let n = r.i32()?;
    if n < 1 {
        return Ok(Relations::default());
    }
    let n = n as usize;
    let entries = r
        .entries::<8>(n)?
        .iter()
        .map(|e| (word(&e[..4]), word(&e[4..])))
        .collect();
    let len = r.count("relation bytes")?;
    Ok(Relations {
        entries,
        blob: r.bytes(len)?.to_vec(),
    })
}

/// The relations a legacy reader skips: version 5 `i32 n`, n × 16 bytes,
/// `i32 m`, m × {8 bytes, `i32 k`, k × 16}; version 6 `i32 n`, if n > 0
/// {n × 8, `i32 len`, len bytes}, `i32 m`, m × {8 bytes, `i32 k`, if k > 0
/// {k × 8, `i32 len`, len bytes}}.
fn skip_legacy_relations(r: &mut Reader<'_>, version: u8) -> Result<()> {
    if version < VERSION_6 {
        let n = r.count("relations")?;
        r.table(n, 16)?;
        for _ in 0..r.count("level relation tables")? {
            r.bytes(8)?;
            let k = r.count("relations")?;
            r.table(k, 16)?;
        }
    } else {
        read_relations(r)?;
        for _ in 0..r.count("level relation tables")? {
            r.bytes(8)?;
            read_relations(r)?;
        }
    }
    Ok(())
}

/// u32 word, ten u32 slots, `i32 n`, then n powers: version 7 {u32 id, u32
/// flags, and with the blob flag `i32 size` and size − 4 bytes}; legacy
/// records 16 bytes each, id and flags first.
fn read_powers(r: &mut Reader<'_>, v7: bool) -> Result<Powers> {
    let word = r.u32()?;
    let mut slots = [0; POWER_SLOTS];
    for s in &mut slots {
        *s = r.u32()?;
    }
    let n = r.count("powers")?;
    let mut known = Vec::new();
    for _ in 0..n {
        let id = r.u32()?;
        let flags = r.u32()?;
        let mut blob = None;
        if !v7 {
            r.bytes(8)?;
        } else if flags & POWER_BLOB != 0 {
            let size = r.count("power bytes")?;
            let payload = size
                .checked_sub(4)
                .ok_or_else(|| Error::Malformed(format!("power blob of {size} bytes")))?;
            blob = Some(r.bytes(payload)?.to_vec());
        }
        known.push(Power { id, flags, blob });
    }
    Ok(Powers { word, slots, known })
}

/// The saved loadouts: `i32 n`, then per loadout a word, four hand items,
/// a worn count and the worn items.
fn read_loadouts(r: &mut Reader<'_>) -> Result<Vec<Loadout>> {
    let n = r.count("loadouts")?;
    let mut out = Vec::with_capacity(n.min(1024));
    for _ in 0..n {
        let tag = r.u32()?;
        let mut hands = [ItemRef::NONE; HANDS];
        for hand in &mut hands {
            *hand = ItemRef::new(r.u32()?, r.u32()?);
        }
        let worn = r.count("loadout items")?;
        let worn = r
            .entries::<8>(worn)?
            .iter()
            .map(|e| ItemRef::new(word(&e[..4]), word(&e[4..])))
            .collect();
        out.push(Loadout {
            word: tag,
            hands,
            worn,
        });
    }
    Ok(out)
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.b.len() - self.pos
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self
            .pos
            .checked_add(n)
            .and_then(|end| self.b.get(self.pos..end))
            .ok_or(Error::Truncated {
                offset: self.pos,
                needed: n,
                len: self.b.len(),
            })?;
        self.pos += n;
        Ok(s)
    }

    /// `n` entries of `size` bytes.
    fn table(&mut self, n: usize, size: usize) -> Result<&'a [u8]> {
        let bytes = n
            .checked_mul(size)
            .ok_or_else(|| Error::Malformed(format!("{n} entries of {size} bytes")))?;
        self.bytes(bytes)
    }

    /// `n` entries of `N` bytes, each as an array.
    fn entries<const N: usize>(&mut self, n: usize) -> Result<&'a [[u8; N]]> {
        Ok(self.table(n, N)?.as_chunks::<N>().0)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }

    /// The next word, without reading it.
    fn peek_u32(&self) -> Option<u32> {
        self.b.get(self.pos..self.pos + 4).map(word)
    }

    fn u16(&mut self) -> Result<u16> {
        let b = self.bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(word(self.bytes(4)?))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }

    /// A signed count; negative is malformed.
    fn count(&mut self, what: &str) -> Result<usize> {
        let n = self.i32()?;
        usize::try_from(n).map_err(|_| Error::Malformed(format!("{n} {what}")))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn name(&mut self) -> Result<String> {
        Ok(latin1(self.bytes(16)?))
    }

    fn matrix(&mut self) -> Result<[[f32; 3]; 4]> {
        let mut m = [[0.0; 3]; 4];
        for row in &mut m {
            for v in row.iter_mut() {
                *v = self.f32()?;
            }
        }
        for v in &mut m[3] {
            *v /= CENTIMETRES_PER_METRE;
        }
        Ok(m)
    }
}

#[cfg(test)]
#[path = "characters_test.rs"]
mod tests;
