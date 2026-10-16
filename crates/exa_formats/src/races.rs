//! `races.rdb`: the character races. Each record (624 bytes) names the race,
//! its body model stem (`Skel` → `skelbase.rfc` in `Resource.rpk`) and its
//! movesets (the shared `Default` and the race's own, such as `Golem`),
//! followed by body parameters (the clip-rate base, size, scale and mass
//! divisor from `0x5C` to `0x70`, the body controller's factors from `0x60`
//! to `0x80`, the gait table at `0x94`, the health factor at `0xE4`), the
//! stats combat, the AI and the sounds read (`0x84` to `0x90`, `0x110`,
//! `0x12C`, `0x144`, `0x1B4`, `0x1C4`), sound names and the race's natural
//! armour (`0x11C` protection, `0x21C` coverage).
//!
//! Two flag words inside the name field (`0x18` and `0x1C`, after the short
//! name) and a kind word at `0x58` hold the race's flags; bit 0 of the first
//! says its characters bleed ([`Race::bleeds`]).
//!
//! A character record's race byte is the race record's id minus one. The
//! original reads a character's race from the loaded record itself, with the
//! file's own layout, and falls back on a built-in record
//! ([`RaceBody::FALLBACK`], [`RaceStats::FALLBACK`]) when the id is missing.
//! Lengths are returned in metres; factors and gait rates as stored.

use crate::rdb::Database;
use crate::{Error, Result};

/// A race record.
#[derive(Debug, Clone, PartialEq)]
pub struct Race {
    pub id: u32,
    pub name: String,
    /// The race's flags (`0x18`, inside the name field after the short
    /// name); see [`Race::bleeds`] and the bit constants ([`BLEEDS`],
    /// [`MAY_RISE`], [`REGENERATES_RED`], [`HOVERS`]).
    pub flags: u32,
    /// Body model stem.
    pub stem: String,
    /// The shared and the race's own moveset.
    pub movesets: [String; 2],
    /// Health relative to a human's.
    pub health: f32,
    /// Natural armour (bones, plates, hide) under any worn items.
    pub natural: Natural,
    /// What the body controller reads from the race.
    pub body: RaceBody,
    /// What the character's init, combat, the AI and the sounds read from
    /// the race.
    pub stats: RaceStats,
}

/// The factors and gaits a race gives its characters' bodies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceBody {
    /// Base rate of the character's clip clock (`0x5C`): frames per tick
    /// before the per-motion factors (1.0 for every race but the two ghouls,
    /// 1.5). The character's init makes it the clock's base.
    pub clip_rate: f32,
    /// Scales the pull of the support pins (the two thorax pins and the
    /// pelvis pin) toward the stance (`0x60`; human 1.25).
    pub pin_factor: f32,
    /// Character size (`0x64`; human 1.0, ogre 1.5, harvester 1.8): the
    /// spawn multiplies it by (1 − 0.05 × the record's height); held weapons
    /// are placed and their stats scaled by it.
    pub size: f32,
    /// Body scale as stored (`0x68`; human 1.0, fael 1.35, harvester and
    /// demon 1.8). The spawn caps it at [`SCALE_CAP`] before the height
    /// factor; the voice volume and the turn and step rates follow it.
    pub scale: f32,
    /// Divides the masses of the character's parts (`0x70`; 1.0, or 1.1
    /// for Sir, Fael and the demon): the init's part-mass normalisation
    /// multiplies its divisor by it.
    pub mass_divisor: f32,
    /// Balance bonus (`0x74`; 1.0 in every record): adds 0.05 × it to the
    /// balance score and widens the recovery-step reach by (1 + it).
    pub balance_bonus: f32,
    /// Stance factor (`0x78`): the share of muscle tone a standing body
    /// uses, and the divisor of the standing link-muscle damping (human
    /// 0.5, skeleton 0.75). A moving body uses 0.7 walking and 1 running
    /// instead.
    pub stance: f32,
    /// Scales how far the link muscles' anchors relax toward the current
    /// pose when the animation holds still, and the arms' relaxed stiffness
    /// (`0x7C`; human 1.0, skeleton 0.5).
    pub arm_relaxation: f32,
    /// Limb damping ratio r (`0x80`): a living body's velocity retention is
    /// scaled by u² with u = (1 − b)(1 − r) + r and b its balance, so a
    /// balanced body's retention is scaled by r² (1.0 for humans: no extra
    /// damping; golems 0.95).
    pub damping_ratio: f32,
    /// Walk, jog, sprint and backpedal.
    pub gaits: [Gait; 4],
}

/// The character's spawn caps the race's body scale at this.
pub const SCALE_CAP: f32 = 1.5;

/// One row of a race's gait table (`0x94`, 5 floats a row). The original's
/// loader reads columns 0, 1, 3 and 4. It replaces column 2 (0 in every
/// record but the backpedal row, which holds stray bytes) with a built-in
/// brake distance per row ([`BRAKE_DISTANCES`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Gait {
    /// Push along the heading, per second (column 0; ÷83 per tick).
    pub thrust: f32,
    /// Scale of the push into the body's carry, per second² (column 1;
    /// 550 for humans; ÷83² per tick).
    pub push_scale: f32,
    /// Distance from its goal under which a walking body brakes, metres
    /// (the loader's built-in column 2: [`BRAKE_DISTANCES`]). A running
    /// body never brakes for its goal.
    pub brake_distance: f32,
    /// The gait's clip rate, frames per second (column 3; ÷83 per tick):
    /// the base rate of the locomotion clock, blended from walk toward the
    /// gait by the run blend.
    pub clip_rate: f32,
    /// Speed along the heading under which the gait's phase push is boosted
    /// (column 4; 0 in every row but the harvester's walk).
    pub boost_below: f32,
}

/// The gait table's rows, in order.
pub const GAIT_NAMES: [&str; 4] = ["walk", "jog", "sprint", "backpedal"];

/// The brake distance of each gait row, metres: the original's loader puts
/// 0.05, 0.2, 0.25 and 0.05 in column 2 of every race's per-tick table,
/// which the locomotion reads × 1000 as centimetres.
pub const BRAKE_DISTANCES: [f32; 4] = [0.5, 2.0, 2.5, 0.5];

const JOG: usize = 1;
const SPRINT: usize = 2;

impl Gait {
    /// Every column moved toward `other` by `t` (0 keeps `self`).
    fn toward(self, other: Gait, t: f32) -> Gait {
        let mix = |a: f32, b: f32| a * (1.0 - t) + b * t;
        Gait {
            thrust: mix(self.thrust, other.thrust),
            push_scale: mix(self.push_scale, other.push_scale),
            brake_distance: mix(self.brake_distance, other.brake_distance),
            clip_rate: mix(self.clip_rate, other.clip_rate),
            boost_below: mix(self.boost_below, other.boost_below),
        }
    }
}

impl RaceBody {
    /// The built-in values the original uses when a character's race is not
    /// in the database: every factor 1.0 and gait rows with no push, so a
    /// body on it cannot walk (its loader still gives them the brake
    /// distances).
    pub const FALLBACK: RaceBody = RaceBody {
        clip_rate: 1.0,
        pin_factor: 1.0,
        size: 1.0,
        scale: 1.0,
        mass_divisor: 1.0,
        balance_bonus: 1.0,
        stance: 1.0,
        arm_relaxation: 1.0,
        damping_ratio: 1.0,
        gaits: [
            Gait::still(BRAKE_DISTANCES[0]),
            Gait::still(BRAKE_DISTANCES[1]),
            Gait::still(BRAKE_DISTANCES[2]),
            Gait::still(BRAKE_DISTANCES[3]),
        ],
    };

    /// The sprint gait a character of feral level `feral` (its record's
    /// value, 0 to 1) uses: the race's sprint row, moved toward its jog row
    /// by the feral level, every column alike. The original sets it at the
    /// character's init; a character that is not feral (0) keeps the sprint
    /// row, and a fully feral one (1) sprints at the jog row.
    pub fn sprint_gait(&self, feral: f32) -> Gait {
        let sprint = self.gaits[SPRINT];
        if feral > 0.0 {
            sprint.toward(self.gaits[JOG], feral)
        } else {
            sprint
        }
    }

    fn read(b: &[u8]) -> RaceBody {
        let f = |at: usize| float(b, at);
        RaceBody {
            clip_rate: f(CLIP_RATE),
            pin_factor: f(PIN_FACTOR),
            size: f(SIZE),
            scale: f(SCALE),
            mass_divisor: f(MASS_DIVISOR),
            balance_bonus: f(BALANCE_BONUS),
            stance: f(STANCE),
            arm_relaxation: f(ARM_RELAXATION),
            damping_ratio: f(DAMPING_RATIO),
            gaits: std::array::from_fn(|i| {
                let row = GAITS + GAIT_ROW * i;
                Gait {
                    thrust: f(row),
                    push_scale: f(row + 4),
                    brake_distance: BRAKE_DISTANCES[i],
                    clip_rate: f(row + 12),
                    boost_below: f(row + 16),
                }
            }),
        }
    }
}

impl Gait {
    const fn still(brake_distance: f32) -> Gait {
        Gait {
            thrust: 0.0,
            push_scale: 0.0,
            brake_distance,
            clip_rate: 0.0,
            boost_below: 0.0,
        }
    }
}

/// The race values the character's init, combat, the AI and the sounds
/// read, beside the body controller's ([`RaceBody`]).
#[derive(Debug, Clone, PartialEq)]
pub struct RaceStats {
    /// The second flag word (`0x1C`, inside the name field after the first);
    /// see [`QUADRUPED`] and [`REGENERATES_RED_TOO`]. Bits 1 and 2 set
    /// character rule bits whose effects are not traced.
    pub flags2: u32,
    /// The kind word (`0x58`): [`CLAWED`] gives the race claws; the whole
    /// word picks the left hand's claw.
    /// An item check also matches bit `0x10000` (golems) with items flagged
    /// `0x2000000` and bit `0x20000` (ogres, the Herath creatures, the
    /// demon) with items flagged `0x4000000`; what it decides is not traced.
    pub kind: u32,
    /// Reach of the empty hands, metres (`0x84`; human 0.5, ogre 1.0, fael
    /// 1.3, Sir 1.5): the character's reach while it holds no weapon.
    pub unarmed_reach: f32,
    /// Scales a held weapon's swing-turn amplitude (`0x88`), how far the AI
    /// turns its body through a swing (1 for humans, 0.5 the ogre, the
    /// gobbler and the golem suit, 0 Sir, the harvester, the demon and the
    /// other golems).
    pub swing_turn: f32,
    /// The AI's attack bonus (`0x8C`; 0.2 for Fael, 0 elsewhere): its combat
    /// state scales an attack-opportunity term by (1 + it) (inferred from
    /// that one reader).
    pub attack_bonus: f32,
    /// Scales a held weapon's recovery (`0x90`; 1, golems 0.5): the frames
    /// the stance blends in over after an attack (rounded).
    pub recovery: f32,
    /// How well the race's AI sees a character masked by a mind power
    /// (`0x110`; 1 humans and most races, 0.5 ancients, 0 skeletons and
    /// wraiths): 1 sees it as any other; p scales its sighting range
    /// (12 m) by 1 − 0.9 × (1 − p), and its outer range is that range
    /// scaled again by 1 − 0.8 × (1 − p). A feral character uses
    /// 1 − feral/2 instead.
    pub perception: f32,
    /// Offset of the items held in the hands from their grip, metres
    /// (`0x12C`, three signed bytes ×0.1 cm; 0.09 m along x for the demon,
    /// 0 elsewhere). The x offset is taken off in the right hand and added
    /// in the left.
    pub held_offset: [f32; 3],
    /// The sound set of the race's footsteps (`0x144`, up to 15 characters:
    /// `StpSft`, `SkelStp`, `StpHvy`, `StpMed`, `Golem_Stp`). Empty plays
    /// none.
    pub footsteps: String,
    /// The race's voice volume factor (`0x1B4`; 1 humans, 3 ogres, Sir and
    /// the demon, 0.5 golems): the init multiplies it by the body scale.
    pub voice_volume: f32,
    /// Attack voices (`0x1C4`; human 4, ogre 1, skeletons and golems 0):
    /// 0 never voices an attack. Otherwise a swing voices with odds of one
    /// (or two) in a counter that grows by (this − 1) with every voice and
    /// shrinks by 1 with every silent swing, so larger values tire of it
    /// sooner (inferred). A feral character uses 2.
    pub attack_voices: i32,
}

/// First flag word: its characters bleed (wounds paint blood, and its body
/// model's skin materials show it).
pub const BLEEDS: u32 = 0x1;
/// First flag word: in the story, a dead character of the race may be
/// raised again later (humans and the golem suit; inferred from the rule
/// bit it sets).
pub const MAY_RISE: u32 = 0x8;
/// First flag word: the race regenerates red health (the golem suit); see
/// also [`REGENERATES_RED_TOO`].
pub const REGENERATES_RED: u32 = 0x80;
/// First flag word: the race's feet hover, so its ground probe is the
/// simple one (the wraith).
pub const HOVERS: u32 = 0x200;
/// Second flag word: the race regenerates red health (ogres, Sir, the
/// harvester and the demon).
pub const REGENERATES_RED_TOO: u32 = 0x4;
/// Second flag word: a quadruped, which stabs with both hands (the
/// harvester).
pub const QUADRUPED: u32 = 0x8;
/// Kind word: the race fights with claws, the clawed class of empty hands
/// (Sir, Fael, the ghouls, Gobbler and the harvester, whose right hand
/// keeps a fist).
pub const CLAWED: u32 = 0x100;

impl RaceStats {
    /// The built-in values the original uses when a character's race is not
    /// in the database: no flags, 1 m of unarmed reach, factors 1, no AI
    /// attack bonus, no footsteps, a silent voice and no attack voices.
    pub const FALLBACK: RaceStats = RaceStats {
        flags2: 0,
        kind: 0,
        unarmed_reach: 1.0,
        swing_turn: 1.0,
        attack_bonus: 0.0,
        recovery: 1.0,
        perception: 1.0,
        held_offset: [0.0; 3],
        footsteps: String::new(),
        voice_volume: 0.0,
        attack_voices: 0,
    };

    fn read(b: &[u8]) -> RaceStats {
        RaceStats {
            flags2: word(b, FLAGS2),
            kind: word(b, KIND),
            unarmed_reach: float(b, UNARMED_REACH) * M_PER_CM,
            swing_turn: float(b, SWING_TURN),
            attack_bonus: float(b, ATTACK_BONUS),
            recovery: float(b, RECOVERY),
            perception: float(b, PERCEPTION),
            held_offset: std::array::from_fn(|i| {
                f32::from(b[HELD_OFFSET + i] as i8) * HELD_OFFSET_CM * M_PER_CM
            }),
            footsteps: text(b, FOOTSTEPS, FOOTSTEPS_LEN),
            voice_volume: float(b, VOICE_VOLUME),
            attack_voices: word(b, ATTACK_VOICES) as i32,
        }
    }
}

/// A race's natural armour layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Natural {
    /// Coverage per bone 1..18, percent.
    pub coverage: [u8; 18],
    /// Padding (×0.005), slash, crush and pierce (×1/140).
    pub padding: u8,
    pub slash: u8,
    pub crush: u8,
    pub pierce: u8,
}

const NAME: usize = 0x00;
const FLAGS: usize = 0x18;
const FLAGS2: usize = 0x1C;
const STEM: usize = 0x20;
const MOVESETS: [usize; 2] = [0x38, 0x48];
const KIND: usize = 0x58;
const CLIP_RATE: usize = 0x5C;
const PIN_FACTOR: usize = 0x60;
const SIZE: usize = 0x64;
const SCALE: usize = 0x68;
const MASS_DIVISOR: usize = 0x70;
const BALANCE_BONUS: usize = 0x74;
const STANCE: usize = 0x78;
const ARM_RELAXATION: usize = 0x7C;
const DAMPING_RATIO: usize = 0x80;
const UNARMED_REACH: usize = 0x84;
const SWING_TURN: usize = 0x88;
const ATTACK_BONUS: usize = 0x8C;
const RECOVERY: usize = 0x90;
const GAITS: usize = 0x94;
const GAIT_ROW: usize = 20;
const HEALTH: usize = 0xE4;
const PERCEPTION: usize = 0x110;
const PROTECTION: usize = 0x11C;
const HELD_OFFSET: usize = 0x12C;
/// Centimetres per unit of a held-offset byte.
const HELD_OFFSET_CM: f32 = 0.1;
const FOOTSTEPS: usize = 0x144;
/// The original copies at most this many characters of the footstep set.
const FOOTSTEPS_LEN: usize = 15;
const VOICE_VOLUME: usize = 0x1B4;
const ATTACK_VOICES: usize = 0x1C4;
const COVERAGE: usize = 0x21C;
const RECORD: usize = 0x270;
const M_PER_CM: f32 = 0.01;

fn text(b: &[u8], at: usize, len: usize) -> String {
    let field = &b[at..at + len];
    let end = field.iter().position(|&c| c == 0).unwrap_or(len);
    String::from_utf8_lossy(&field[..end]).into_owned()
}

fn word(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn float(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

impl Race {
    /// The body model entry in `Resource.rpk`.
    pub fn model(&self) -> String {
        format!("{}base.rfc", self.stem.to_ascii_lowercase())
    }

    /// The race's own moveset.
    pub fn moveset(&self) -> &str {
        &self.movesets[1]
    }

    /// Whether the race's characters bleed: humans, ogres, the Herath
    /// creatures and demons do; skeletons, ancients, wraiths and golems do
    /// not.
    pub fn bleeds(&self) -> bool {
        self.flags & BLEEDS != 0
    }

    /// Whether the race regenerates red health, by either flag word: ogres,
    /// Sir, the harvester, the demon and the golem suit.
    pub fn regenerates_red(&self) -> bool {
        self.flags & REGENERATES_RED != 0 || self.stats.flags2 & REGENERATES_RED_TOO != 0
    }

    /// Whether the race fights with claws.
    pub fn clawed(&self) -> bool {
        self.stats.kind & CLAWED != 0
    }
}

/// Every race in the database, in table order (duplicate ids keep the
/// first; empty entries are skipped).
pub fn parse(bytes: &[u8]) -> Result<Vec<Race>> {
    let db = Database::parse(bytes)?;
    let mut races: Vec<Race> = Vec::new();
    for r in &db.records {
        let b = db.payload(r);
        // The table ends with an empty entry.
        if b.is_empty() {
            continue;
        }
        if b.len() < RECORD {
            return Err(Error::Malformed(format!(
                "race record {} is {} bytes",
                r.id,
                b.len()
            )));
        }
        if races.iter().any(|x| x.id == r.id) {
            continue;
        }
        races.push(Race {
            id: r.id,
            name: text(b, NAME, 0x20),
            flags: word(b, FLAGS),
            stem: text(b, STEM, 0x18),
            movesets: MOVESETS.map(|at| text(b, at, 0x10)),
            health: float(b, HEALTH),
            natural: Natural {
                coverage: std::array::from_fn(|i| b[COVERAGE + 4 * i].min(100)),
                padding: b[PROTECTION],
                slash: b[PROTECTION + 1],
                crush: b[PROTECTION + 2],
                pierce: b[PROTECTION + 3],
            },
            body: RaceBody::read(b),
            stats: RaceStats::read(b),
        });
    }
    Ok(races)
}

/// The race of a character record's race byte.
pub fn of_character(races: &[Race], race: u8) -> Option<&Race> {
    races.iter().find(|r| r.id == u32::from(race) + 1)
}

/// The body parameters of a character record's race byte, or the original's
/// fallback when the database has no such race.
pub fn body_of_character(races: &[Race], race: u8) -> RaceBody {
    of_character(races, race).map_or(RaceBody::FALLBACK, |r| r.body)
}

/// The stats of a character record's race byte, or the original's fallback
/// when the database has no such race.
pub fn stats_of_character(races: &[Race], race: u8) -> RaceStats {
    of_character(races, race).map_or(RaceStats::FALLBACK, |r| r.stats.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str, stem: &str, own: &str) -> Vec<u8> {
        let mut b = vec![0u8; RECORD];
        b[NAME..NAME + name.len()].copy_from_slice(name.as_bytes());
        if name == "Human" {
            b[FLAGS..FLAGS + 4].copy_from_slice(&0x0Fu32.to_le_bytes());
        }
        b[STEM..STEM + stem.len()].copy_from_slice(stem.as_bytes());
        b[MOVESETS[0]..MOVESETS[0] + 7].copy_from_slice(b"Default");
        b[MOVESETS[1]..MOVESETS[1] + own.len()].copy_from_slice(own.as_bytes());
        b[HEALTH..HEALTH + 4].copy_from_slice(&2.0f32.to_le_bytes());
        b[PROTECTION..PROTECTION + 4].copy_from_slice(&[70, 130, 70, 80]);
        b[COVERAGE + 4 * 9] = 100;
        b
    }

    #[test]
    fn races_by_character_byte() {
        let records = [
            record("Human", "Human", "Default"),
            record("Golem Guard", "Golem", "Golem"),
        ];
        let mut file = Vec::new();
        file.extend_from_slice(&0xDBCB_0D00u32.to_le_bytes());
        file.extend_from_slice(&32u32.to_le_bytes());
        for (i, id) in [1u32, 17].iter().enumerate() {
            file.extend_from_slice(&id.to_le_bytes());
            file.extend_from_slice(&0u32.to_le_bytes());
            file.extend_from_slice(&((i * RECORD) as u32).to_le_bytes());
            file.extend_from_slice(&(RECORD as u32).to_le_bytes());
        }
        for r in &records {
            file.extend_from_slice(r);
        }
        let races = parse(&file).unwrap();
        let golem = of_character(&races, 16).unwrap();
        assert_eq!(golem.model(), "golembase.rfc");
        assert_eq!(golem.moveset(), "Golem");
        assert_eq!(golem.health, 2.0);
        assert_eq!(golem.natural.coverage[9], 100);
        assert_eq!(golem.natural.slash, 130);
        let human = of_character(&races, 0).unwrap();
        assert_eq!(human.name, "Human");
        // The flags sit inside the name field, after the short name.
        assert_eq!(human.flags, 0x0F);
        assert!(human.bleeds());
        assert!(!golem.bleeds());
        assert!(of_character(&races, 1).is_none());
    }

    fn database(records: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut file = Vec::new();
        file.extend_from_slice(&0xDBCB_0D00u32.to_le_bytes());
        file.extend_from_slice(&((16 * records.len()) as u32).to_le_bytes());
        for (i, (id, r)) in records.iter().enumerate() {
            file.extend_from_slice(&id.to_le_bytes());
            file.extend_from_slice(&0u32.to_le_bytes());
            file.extend_from_slice(&((i * RECORD) as u32).to_le_bytes());
            file.extend_from_slice(&(r.len() as u32).to_le_bytes());
        }
        for (_, r) in records {
            file.extend_from_slice(r);
        }
        file
    }

    fn put(b: &mut [u8], at: usize, v: f32) {
        b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn put_word(b: &mut [u8], at: usize, v: u32) {
        b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    #[test]
    fn body_parameters() {
        let mut b = record("Human", "Human", "Default");
        // Neighbouring floats that are not body parameters stay unread.
        for at in (0x58..0xE4).step_by(4) {
            put(&mut b, at, -7.0);
        }
        put(&mut b, 0x5C, 1.5);
        put(&mut b, 0x60, 1.25);
        put(&mut b, 0x64, 1.8);
        put(&mut b, 0x68, 1.35);
        put(&mut b, 0x70, 1.1);
        put(&mut b, 0x74, 1.5);
        put(&mut b, 0x78, 0.5);
        put(&mut b, 0x7C, 0.75);
        put(&mut b, 0x80, 0.95);
        for row in 0..4 {
            let at = 0x94 + 20 * row;
            let r = row as f32;
            put(&mut b, at, 100.0 + r);
            put(&mut b, at + 4, 550.0 + r);
            put(&mut b, at + 12, 20.0 + r);
            put(&mut b, at + 16, 5.0 + r);
        }
        let races = parse(&database(&[(1, b)])).unwrap();
        let body = races[0].body;
        assert_eq!(body.clip_rate, 1.5);
        assert_eq!(body.pin_factor, 1.25);
        assert_eq!(body.size, 1.8);
        assert_eq!(body.scale, 1.35);
        assert_eq!(body.mass_divisor, 1.1);
        assert_eq!(body.balance_bonus, 1.5);
        assert_eq!(body.stance, 0.5);
        assert_eq!(body.arm_relaxation, 0.75);
        assert_eq!(body.damping_ratio, 0.95);
        for (row, g) in body.gaits.iter().enumerate() {
            let r = row as f32;
            assert_eq!(
                *g,
                Gait {
                    thrust: 100.0 + r,
                    push_scale: 550.0 + r,
                    // Column 2 (-7 here) is the loader's built-in.
                    brake_distance: [0.5, 2.0, 2.5, 0.5][row],
                    clip_rate: 20.0 + r,
                    boost_below: 5.0 + r,
                }
            );
        }
        // The health factor right after the gait table is still its own.
        assert_eq!(races[0].health, 2.0);
    }

    #[test]
    fn stats() {
        let mut b = record("Demon", "DemonA01", "Demon A01");
        put_word(&mut b, 0x18, 0x23);
        put_word(&mut b, 0x1C, 0x7);
        put_word(&mut b, 0x58, 0x2_0100);
        put(&mut b, 0x84, 150.0);
        put(&mut b, 0x88, 0.5);
        put(&mut b, 0x8C, 0.2);
        put(&mut b, 0x90, 0.5);
        put(&mut b, 0x110, 0.5);
        // Signed bytes, a tenth of a centimetre each.
        b[0x12C..0x12F].copy_from_slice(&[90, 0xF6, 5]);
        // The footstep set is cut at 15 characters.
        b[0x144..0x154].copy_from_slice(b"Golem_Stp_Longer");
        put(&mut b, 0x1B4, 3.0);
        put_word(&mut b, 0x1C4, 2);
        let races = parse(&database(&[(12, b)])).unwrap();
        let demon = &races[0];
        let s = &demon.stats;
        assert_eq!((s.flags2, s.kind), (0x7, 0x2_0100));
        assert!(demon.regenerates_red() && demon.clawed());
        assert!((s.unarmed_reach - 1.5).abs() < 1e-6);
        assert_eq!((s.swing_turn, s.attack_bonus, s.recovery), (0.5, 0.2, 0.5));
        assert_eq!(s.perception, 0.5);
        let want = [0.09, -0.01, 0.005];
        for (got, want) in s.held_offset.iter().zip(want) {
            assert!((got - want).abs() < 1e-6, "{:?}", s.held_offset);
        }
        assert_eq!(s.footsteps, "Golem_Stp_Longe");
        assert_eq!((s.voice_volume, s.attack_voices), (3.0, 2));
        assert_eq!(stats_of_character(&races, 11), *s);
    }

    /// The sprint row moves toward the jog row by the feral level: a human
    /// half feral sprints at (472.2 + 350) / 2 = 411.1 per second.
    #[test]
    fn feral_characters_sprint_toward_the_jog() {
        let mut body = RaceBody::FALLBACK;
        body.gaits[1] = Gait {
            thrust: 350.0,
            push_scale: 550.0,
            brake_distance: 2.0,
            clip_rate: 16.6,
            boost_below: 0.0,
        };
        body.gaits[2] = Gait {
            thrust: 472.2,
            push_scale: 550.0,
            brake_distance: 2.5,
            clip_rate: 13.5,
            boost_below: 0.0,
        };
        assert_eq!(body.sprint_gait(0.0), body.gaits[2]);
        assert_eq!(body.sprint_gait(-1.0), body.gaits[2]);
        let half = body.sprint_gait(0.5);
        assert!((half.thrust - 411.1).abs() < 1e-3);
        assert!((half.clip_rate - 15.05).abs() < 1e-5);
        assert!((half.brake_distance - 2.25).abs() < 1e-6);
        assert_eq!(half.push_scale, 550.0);
        assert_eq!(body.sprint_gait(1.0), body.gaits[1]);
    }

    #[test]
    fn unknown_races_fall_back() {
        let mut b = record("Human", "Human", "Default");
        put(&mut b, 0x78, 0.5);
        let races = parse(&database(&[(1, b)])).unwrap();
        assert_eq!(body_of_character(&races, 0).stance, 0.5);
        let fallback = body_of_character(&races, 4);
        assert_eq!(fallback, RaceBody::FALLBACK);
        assert_eq!(fallback.stance, 1.0);
        assert_eq!(
            (fallback.clip_rate, fallback.size, fallback.scale),
            (1.0, 1.0, 1.0)
        );
        assert!(
            fallback
                .gaits
                .iter()
                .all(|g| g.thrust == 0.0 && g.push_scale == 0.0)
        );
        assert_eq!(fallback.gaits.map(|g| g.brake_distance), BRAKE_DISTANCES);
        let stats = stats_of_character(&races, 4);
        assert_eq!(stats, RaceStats::FALLBACK);
        assert_eq!(stats.unarmed_reach, 1.0);
        assert_eq!((stats.voice_volume, stats.attack_voices), (0.0, 0));
        assert!(stats.footsteps.is_empty());
    }

    /// Every record of the installed `races.rdb` (skipped without the game).
    #[test]
    fn shipped_races() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let bytes = game
            .pack(exa_rpk::PackKind::Resource)
            .and_then(|p| p.get("races.rdb"))
            .expect("races.rdb in Resource.rpk");
        let races = parse(bytes).unwrap();
        assert_eq!(races.len(), 18);
        let close = |a: f32, b: f32| (a - b).abs() < 0.05;
        let human = &of_character(&races, 0).unwrap();
        assert_eq!(human.name, "Human");
        let h = human.body;
        assert_eq!(
            (
                h.pin_factor,
                h.balance_bonus,
                h.stance,
                h.arm_relaxation,
                h.damping_ratio
            ),
            (1.25, 1.0, 0.5, 1.0, 1.0)
        );
        for (g, thrust) in h.gaits.iter().zip([88.9, 350.0, 472.2, 77.8]) {
            assert!(close(g.thrust, thrust), "{g:?}");
            assert_eq!(g.push_scale, 550.0);
        }
        let skeleton = of_character(&races, 1).unwrap().body;
        assert_eq!((skeleton.stance, skeleton.arm_relaxation), (0.75, 0.5));
        // Who bleeds, and the flag words as stored.
        let flags = |name: &str| {
            races
                .iter()
                .find(|r| r.name == name)
                .map(|r| (r.flags, r.bleeds()))
        };
        for (name, word) in [
            ("Human", 0x0F),
            ("Ogre", 0x27),
            ("Sir", 0x23),
            ("Fael", 0x03),
            ("Ghoul", 0x03),
            ("Ghoul H", 0x03),
            ("Gobbler", 0x03),
            ("Manipulator", 0x03),
            ("HrtA Harvester", 0x23),
            ("DemonA01", 0x23),
        ] {
            assert_eq!(flags(name), Some((word, true)), "{name}");
        }
        for (name, word) in [
            ("Skeleton", 0),
            ("Ancient", 0),
            ("Wraith", 0x300),
            ("Golem Guard", 0x1030),
            ("Golem Boss", 0x2030),
            ("Golem Suit", 0x20B8),
            ("Golem BAM", 0x1030),
            ("Golem BAF", 0x1030),
        ] {
            assert_eq!(flags(name), Some((word, false)), "{name}");
        }
        // The clip clock's base (every race's in `shipped_stats`: 1.0, but
        // 1.5 for the two ghouls); the human's clip rates 27, 16.6, 13.5 and
        // 28 frames a second.
        assert_eq!(h.clip_rate, 1.0);
        for (g, rate) in h.gaits.iter().zip([27.0, 16.6, 13.5, 28.0]) {
            assert!(close(g.clip_rate, rate), "{g:?}");
        }
        for r in &races {
            let b = r.body;
            for v in [b.pin_factor, b.balance_bonus, b.stance, b.arm_relaxation] {
                assert!(v > 0.0 && v <= 2.0, "{} {b:?}", r.name);
            }
            assert!(
                b.damping_ratio > 0.9 && b.damping_ratio <= 1.0,
                "{}",
                r.name
            );
            assert!(b.gaits.iter().all(|g| g.thrust > 0.0 && g.push_scale > 0.0));
            assert_eq!(b.gaits.map(|g| g.brake_distance), BRAKE_DISTANCES);
        }
        shipped_stats(&races);
    }

    /// The stat fields of all 18 records, as the notes give them.
    fn shipped_stats(races: &[Race]) {
        struct Want {
            name: &'static str,
            clip_rate: f32,
            size: f32,
            scale: f32,
            mass_divisor: f32,
            flags2: u32,
            kind: u32,
            reach_cm: f32,
            swing_turn: f32,
            recovery: f32,
            perception: f32,
            footsteps: &'static str,
            voice_volume: f32,
            attack_voices: i32,
        }
        #[rustfmt::skip]
        let want = |name, clip_rate, size, scale, mass_divisor, flags2, kind, reach_cm,
                    swing_turn, recovery, perception, footsteps, voice_volume, attack_voices| Want {
            name, clip_rate, size, scale, mass_divisor, flags2, kind, reach_cm, swing_turn,
            recovery, perception, footsteps, voice_volume, attack_voices,
        };
        #[rustfmt::skip]
        let table = [
            //   name               clip  size  scale  mass  flags2 kind       reach   turn  rec   perc  footsteps     voice att
            want("Human",           1.0,  1.0,  1.0,   1.0,  0x0,  0x1,       50.0,   1.0,  1.0,  1.0,  "StpSft",     1.0,  4),
            want("Skeleton",        1.0,  1.0,  1.0,   1.0,  0x0,  0x1,       50.0,   1.0,  1.0,  0.0,  "SkelStp",    1.0,  0),
            want("Ancient",         1.0,  1.0,  1.0,   1.0,  0x0,  0x1,       50.0,   1.0,  1.0,  0.5,  "StpSft",     1.0,  4),
            want("Ogre",            1.0,  1.5,  1.5,   1.0,  0x7,  0x2_0001,  100.0,  0.5,  1.0,  1.0,  "StpHvy",     3.0,  1),
            want("Sir",             1.0,  1.5,  1.5,   1.1,  0x7,  0x2_0100,  150.0,  0.0,  1.0,  1.0,  "StpHvy",     3.0,  1),
            want("Fael",            1.0,  1.5,  1.35,  1.1,  0x3,  0x2_0100,  130.0,  1.0,  1.0,  1.0,  "StpMed",     1.5,  2),
            want("Ghoul",           1.5,  1.0,  1.0,   1.0,  0x3,  0x100,     100.0,  1.0,  1.0,  1.0,  "StpSft",     1.5,  2),
            want("Ghoul H",         1.5,  1.0,  1.0,   1.0,  0x3,  0x100,     100.0,  1.0,  1.0,  1.0,  "StpSft",     2.0,  2),
            want("Gobbler",         1.0,  1.0,  1.0,   1.0,  0x3,  0x2_0100,  150.0,  0.5,  1.0,  1.0,  "StpSft",     1.0,  3),
            want("Manipulator",     1.0,  1.0,  1.0,   1.0,  0x3,  0x1,       50.0,   1.0,  1.0,  1.0,  "StpSft",     2.0,  3),
            want("HrtA Harvester",  1.0,  1.8,  1.8,   1.0,  0xF,  0x2_0100,  150.0,  0.0,  1.0,  1.0,  "StpSft",     1.5,  2),
            want("DemonA01",        1.0,  1.5,  1.8,   1.1,  0x7,  0x2_0001,  100.0,  0.0,  1.0,  1.0,  "StpHvy",     3.0,  2),
            want("Wraith",          1.0,  1.0,  1.0,   1.0,  0x0,  0x1,       50.0,   1.0,  1.0,  0.0,  "StpSft",     1.0,  4),
            want("Golem Guard",     1.0,  1.4,  1.2,   1.0,  0x0,  0x1_0001,  100.0,  0.0,  0.5,  1.0,  "Golem_Stp",  0.5,  0),
            want("Golem Boss",      1.0,  1.5,  1.2,   1.0,  0x0,  0x1_0001,  100.0,  0.0,  0.5,  1.0,  "Golem_Stp",  0.5,  0),
            want("Golem Suit",      1.0,  1.0,  1.0,   1.0,  0x0,  0x1,       50.0,   0.5,  0.5,  1.0,  "Golem_Stp",  0.5,  0),
            want("Golem BAM",       1.0,  1.4,  1.2,   1.0,  0x0,  0x1_0001,  100.0,  0.0,  0.5,  1.0,  "Golem_Stp",  0.5,  0),
            want("Golem BAF",       1.0,  1.4,  1.2,   1.0,  0x0,  0x1_0001,  100.0,  0.0,  0.5,  1.0,  "Golem_Stp",  0.5,  0),
        ];
        assert_eq!(table.len(), races.len());
        for w in &table {
            let r = races.iter().find(|r| r.name == w.name).expect(w.name);
            let (b, s) = (&r.body, &r.stats);
            let got = (b.clip_rate, b.size, b.scale, b.mass_divisor);
            assert!(
                close4(got, (w.clip_rate, w.size, w.scale, w.mass_divisor)),
                "{} {got:?}",
                w.name
            );
            assert_eq!((s.flags2, s.kind), (w.flags2, w.kind), "{}", w.name);
            assert!(
                (s.unarmed_reach - w.reach_cm / 100.0).abs() < 1e-6,
                "{} {}",
                w.name,
                s.unarmed_reach
            );
            assert_eq!(
                (s.swing_turn, s.recovery, s.perception),
                (w.swing_turn, w.recovery, w.perception),
                "{}",
                w.name
            );
            assert_eq!(s.footsteps, w.footsteps, "{}", w.name);
            assert_eq!(
                (s.voice_volume, s.attack_voices),
                (w.voice_volume, w.attack_voices),
                "{}",
                w.name
            );
            // Only Fael's AI gets an attack bonus, only the demon holds
            // items off their grip.
            let fael = w.name == "Fael";
            assert_eq!(s.attack_bonus, if fael { 0.2 } else { 0.0 }, "{}", w.name);
            let demon = w.name == "DemonA01";
            let offset = if demon { [0.09, 0.0, 0.0] } else { [0.0; 3] };
            assert!(
                s.held_offset
                    .iter()
                    .zip(offset)
                    .all(|(a, b)| (a - b).abs() < 1e-6),
                "{} {:?}",
                w.name,
                s.held_offset
            );
        }
        // The named flag bits, race by race.
        let named = |f: fn(&Race) -> bool| {
            let mut v: Vec<&str> = races
                .iter()
                .filter(|r| f(r))
                .map(|r| r.name.as_str())
                .collect();
            v.sort_unstable();
            v
        };
        assert_eq!(
            named(Race::regenerates_red),
            ["DemonA01", "Golem Suit", "HrtA Harvester", "Ogre", "Sir"]
        );
        assert_eq!(
            named(Race::clawed),
            [
                "Fael",
                "Ghoul",
                "Ghoul H",
                "Gobbler",
                "HrtA Harvester",
                "Sir"
            ]
        );
        assert_eq!(named(|r| r.flags & HOVERS != 0), ["Wraith"]);
        assert_eq!(named(|r| r.flags & MAY_RISE != 0), ["Golem Suit", "Human"]);
        assert_eq!(
            named(|r| r.stats.flags2 & QUADRUPED != 0),
            ["HrtA Harvester"]
        );
    }

    fn close4(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
        let c = |x: f32, y: f32| (x - y).abs() < 1e-6;
        c(a.0, b.0) && c(a.1, b.1) && c(a.2, b.2) && c(a.3, b.3)
    }

    #[test]
    fn short_records_are_rejected() {
        let mut file = Vec::new();
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&16u32.to_le_bytes());
        for v in [1u32, 0, 0, 8] {
            file.extend_from_slice(&v.to_le_bytes());
        }
        file.extend_from_slice(&[0u8; 8]);
        assert!(parse(&file).is_err());
    }
}
