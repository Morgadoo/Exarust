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
/// Power slots of the powers chunk.
pub const POWER_SLOTS: usize = 10;
/// Saved-state entry: red health still to heal from bandages (an `f32`).
pub const SAVED_HEAL_POOL: u32 = 0xCECC_0001;
/// Saved-state entry: a drunk potion's expiry on the game clock.
pub const SAVED_POTION: u32 = 0xCECC_0002;
/// A pool stored full (share 1).
pub const FULL_POOL: u16 = 0xFFFF;
/// Character traits are stored as bytes offset by this.
const TRAIT_BIAS: f32 = 100.0;

/// Skills a record holds, in slot order.
pub const SKILLS: usize = 6;
/// Techniques stored per skill (one per level).
pub const TECHNIQUE_SLOTS: usize = 6;
/// Skill points per level: each level learns one technique.
pub const POINTS_PER_TECHNIQUE: u16 = 5000;
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

/// A character's record: who they are, what they wear and carry, their saved
/// health, skills, roles and powers.
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
    pub female: bool,
    pub race: u8,
    pub voice: String,
    /// Voice pitch as stored (the character's init clamps it).
    pub voice_pitch: f32,
    /// Items held in the hands (two loadouts), 0 = none.
    pub hands: [u32; 4],
    /// Worn apparel items.
    pub worn: Vec<u32>,
    /// How far gone an undead character is, 0…1 (0 for the living and for
    /// every race but humans). The name is the research notes' reading of
    /// what it does (weaker, looser, the undead voices).
    pub feral: f32,
    /// Health and focus as saved.
    pub pools: Pools,
    /// The AI's fighting skill, 0…100.
    pub combat_skill: f32,
    /// Which undead voice a feral character uses.
    pub undead_voice: u32,
    /// Four personality traits, −100…100 (the AI reads the second and the
    /// third).
    pub traits: [f32; 4],
    pub experience: i32,
    pub skills: [Skill; SKILLS],
    pub inventory: Vec<Carried>,
    /// Roles, newest first. A legacy record lists only the roles its reader
    /// adds; their parent roles come from the role database.
    pub roles: Vec<Role>,
    /// Relations kept toward other characters (always none in the shipped
    /// data; a legacy reader drops them).
    pub relations: usize,
    /// Saved body state (heal pool, potion), as stored.
    pub saved: Vec<SavedState>,
    /// A thaumaturge's powers.
    pub powers: Option<Powers>,
}

impl Default for CharacterRecord {
    fn default() -> Self {
        Self {
            id: 0,
            version: RECORD_VERSION,
            name: String::new(),
            surname: String::new(),
            physique: [0.0; 2],
            height: 0.0,
            age: 0.0,
            female: false,
            race: 0,
            voice: String::new(),
            voice_pitch: 1.0,
            hands: [0; 4],
            worn: Vec::new(),
            feral: 0.0,
            pools: Pools::FULL,
            combat_skill: 0.0,
            undead_voice: 0,
            traits: [0.0; 4],
            experience: 0,
            skills: [Skill::default(); SKILLS],
            inventory: Vec::new(),
            roles: Vec::new(),
            relations: 0,
            saved: Vec::new(),
            powers: None,
        }
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

    pub fn knows(&self, technique: u8) -> bool {
        technique != 0 && self.learnt().contains(&technique)
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
    /// A third word whose meaning is not traced.
    pub extra: u32,
}

/// A role held by the character (`charroles.rdb`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    pub id: u32,
    /// Assigned explicitly, not only pulled in as a parent.
    pub explicit: bool,
    /// How many explicit roles pulled this one in as a parent.
    pub inherited: u8,
    /// The role's variables, in its declaration order.
    pub variables: Vec<u32>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Power {
    pub id: u32,
    pub flags: u32,
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
            ..Self::default()
        };
        rec.name = r.name()?;
        rec.surname = r.name()?;
        rec.physique = [r.f32()?, r.f32()?];
        rec.height = r.f32()?;
        rec.age = r.f32()?;
        r.bytes(3 * 4)?; // skin colour, hair style, hair colour
        r.bytes(2 * 2)?; // face styles (read by every version)
        rec.voice = r.name()?;
        rec.voice_pitch = r.f32()?;
        let profession = r.u32()?;
        rec.female = profession & 1 != 0;
        rec.race = (profession >> 8) as u8;
        r.i32()?; // last locale
        for h in &mut rec.hands {
            *h = r.u32()?;
            r.u32()?;
        }
        let worn = r.count("worn items")?;
        rec.worn = r.entries::<8>(worn)?.iter().map(|e| word(e)).collect();
        rec.feral = r.f32()?;
        rec.pools = Pools {
            yellow: r.u16()?,
            red: r.u16()?,
            focus: r.u16()?,
        };
        r.u16()?; // random seed
        rec.combat_skill = r.f32()?;
        rec.undead_voice = r.u32()?;
        for t in &mut rec.traits {
            *t = f32::from(r.u8()?) - TRAIT_BIAS;
        }
        r.u32()?; // random seed of the AI's search
        rec.experience = r.i32()?;
        r.u32()?; // a word always 0 in the shipped data
        rec.skills = read_skills(&mut r)?;
        let carried = r.count("carried items")?;
        rec.inventory = r
            .entries::<12>(carried)?
            .iter()
            .map(|e| Carried {
                item: word(&e[..4]),
                flags: word(&e[4..8]),
                extra: word(&e[8..]),
            })
            .collect();
        if version == RECORD_VERSION {
            rec.roles = read_roles(&mut r)?;
            rec.relations = read_relations(&mut r)?;
            for _ in 0..r.count("level relation tables")? {
                r.bytes(8)?;
                rec.relations += read_relations(&mut r)?;
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
            if POWERS.contains(&tag) {
                rec.powers = Some(read_powers(&mut chunk, version == RECORD_VERSION)?);
            }
        }
        Ok(rec)
    }

    /// The voice number the voice name gives (`xx0N` → N − 1, at least 0),
    /// else 0. Names hold one character per stored byte.
    pub fn voice_index(&self) -> u32 {
        let mut c = self.voice.chars().skip(2);
        match (c.next(), c.next()) {
            (Some('0'), Some(n)) => u32::from(n).saturating_sub(u32::from('1')),
            _ => 0,
        }
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
        blob.f32()?; // weight (the role database's is used)
        let flags = blob.u8()?;
        let inherited = blob.u8()?;
        let n = usize::from(blob.u8()?);
        blob.u8()?;
        let variables = blob.entries::<4>(n)?.iter().map(|e| word(e)).collect();
        roles.push(Role {
            id,
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

/// Legacy roles: `i32 n`, n × {u32 id, u32 ignored}, each added in turn
/// (newest first). Three old first roles are replaced by their successors.
fn read_legacy_roles(r: &mut Reader<'_>) -> Result<Vec<Role>> {
    /// Old first role → the two roles added instead.
    const REPLACED: [(u32, [u32; 2]); 3] = [(13, [8, 33]), (27, [26, 33]), (22, [9, 33])];
    let n = r.count("roles")?;
    let ids: Vec<u32> = r.entries::<8>(n)?.iter().map(|e| word(e)).collect();
    let added = match ids.first() {
        Some(first) => match REPLACED.iter().find(|(old, _)| old == first) {
            Some((_, new)) => new.to_vec(),
            None => ids,
        },
        None => ids,
    };
    Ok(added
        .into_iter()
        .rev()
        .map(|id| Role {
            id,
            explicit: true,
            inherited: 0,
            variables: Vec::new(),
        })
        .collect())
}

/// A relation table: `i32 n`; if n ≥ 1, n × {u32 character, u32 offset},
/// `i32 len` and the `len`-byte blob. Returns n.
fn read_relations(r: &mut Reader<'_>) -> Result<usize> {
    let n = r.i32()?;
    if n < 1 {
        return Ok(0);
    }
    let n = n as usize;
    r.table(n, 8)?;
    let len = r.count("relation bytes")?;
    r.bytes(len)?;
    Ok(n)
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
        if !v7 {
            r.bytes(8)?;
        } else if flags & POWER_BLOB != 0 {
            let size = r.count("power bytes")?;
            r.bytes(
                size.checked_sub(4)
                    .ok_or_else(|| Error::Malformed(format!("power blob of {size} bytes")))?,
            )?;
        }
        known.push(Power { id, flags });
    }
    Ok(Powers { word, slots, known })
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
        let raw = self.bytes(16)?;
        let end = raw.iter().position(|&c| c == 0).unwrap_or(16);
        Ok(raw[..end].iter().map(|&c| c as char).collect())
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
