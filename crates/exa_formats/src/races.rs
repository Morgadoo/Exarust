//! `races.rdb`: the character races. Each record (624 bytes) names the race,
//! its body model stem (`Skel` → `skelbase.rfc` in `Resource.rpk`) and its
//! movesets (the shared `Default` and the race's own, such as `Golem`),
//! followed by body parameters (the body controller's factors from `0x60`
//! to `0x80`, the gait table at `0x94`, the health factor at `0xE4`), sound
//! names and the race's natural armour (`0x11C` protection, `0x21C`
//! coverage).
//!
//! A character record's race byte is the race record's id minus one. The
//! original reads a character's race from the loaded record itself, with the
//! file's own layout, and falls back on built-in values ([`RaceBody::FALLBACK`])
//! when the id is missing.

use crate::rdb::Database;
use crate::{Error, Result};

/// A race record.
#[derive(Debug, Clone, PartialEq)]
pub struct Race {
    pub id: u32,
    pub name: String,
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
}

/// The factors and gaits a race gives its characters' bodies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceBody {
    /// Scales the pull of the support pins (the two thorax pins and the
    /// pelvis pin) toward the stance (`0x60`; human 1.25).
    pub pin_factor: f32,
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

/// One row of a race's gait table (`0x94`, 5 floats a row). The original's
/// loader reads columns 0, 1, 3 and 4. It replaces column 2 (0 in every
/// record) with a built-in value per row (0.05, 0.2, 0.25, 0.05) whose use
/// is not traced.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Gait {
    /// Push along the heading, per second (column 0; ÷83 per tick).
    pub thrust: f32,
    /// Scale of the push into the body's carry, per second² (column 1;
    /// 550 for humans; ÷83² per tick).
    pub push_scale: f32,
    /// The gait's clip rate, per second (column 3; ÷83 per tick). How it
    /// feeds the clip clock is not traced.
    pub clip_rate: f32,
    /// Speed along the heading under which the gait's phase push is boosted
    /// (column 4; 0 in every row but the harvester's walk).
    pub boost_below: f32,
}

/// The gait table's rows, in order.
pub const GAIT_NAMES: [&str; 4] = ["walk", "jog", "sprint", "backpedal"];

impl RaceBody {
    /// The built-in values the original uses when a character's race is not
    /// in the database: every factor 1.0 and an all-zero gait table, so a
    /// body on it cannot walk.
    pub const FALLBACK: RaceBody = RaceBody {
        pin_factor: 1.0,
        balance_bonus: 1.0,
        stance: 1.0,
        arm_relaxation: 1.0,
        damping_ratio: 1.0,
        gaits: [Gait {
            thrust: 0.0,
            push_scale: 0.0,
            clip_rate: 0.0,
            boost_below: 0.0,
        }; 4],
    };

    fn read(b: &[u8]) -> RaceBody {
        let f = |at: usize| f32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"));
        RaceBody {
            pin_factor: f(PIN_FACTOR),
            balance_bonus: f(BALANCE_BONUS),
            stance: f(STANCE),
            arm_relaxation: f(ARM_RELAXATION),
            damping_ratio: f(DAMPING_RATIO),
            gaits: std::array::from_fn(|i| {
                let row = GAITS + GAIT_ROW * i;
                Gait {
                    thrust: f(row),
                    push_scale: f(row + 4),
                    clip_rate: f(row + 12),
                    boost_below: f(row + 16),
                }
            }),
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
const STEM: usize = 0x20;
const MOVESETS: [usize; 2] = [0x38, 0x48];
const PIN_FACTOR: usize = 0x60;
const BALANCE_BONUS: usize = 0x74;
const STANCE: usize = 0x78;
const ARM_RELAXATION: usize = 0x7C;
const DAMPING_RATIO: usize = 0x80;
const GAITS: usize = 0x94;
const GAIT_ROW: usize = 20;
const HEALTH: usize = 0xE4;
const PROTECTION: usize = 0x11C;
const COVERAGE: usize = 0x21C;
const RECORD: usize = 0x270;

fn text(b: &[u8], at: usize, len: usize) -> String {
    let field = &b[at..at + len];
    let end = field.iter().position(|&c| c == 0).unwrap_or(len);
    String::from_utf8_lossy(&field[..end]).into_owned()
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
            stem: text(b, STEM, 0x18),
            movesets: MOVESETS.map(|at| text(b, at, 0x10)),
            health: f32::from_le_bytes(b[HEALTH..HEALTH + 4].try_into().expect("4 bytes")),
            natural: Natural {
                coverage: std::array::from_fn(|i| b[COVERAGE + 4 * i].min(100)),
                padding: b[PROTECTION],
                slash: b[PROTECTION + 1],
                crush: b[PROTECTION + 2],
                pierce: b[PROTECTION + 3],
            },
            body: RaceBody::read(b),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str, stem: &str, own: &str) -> Vec<u8> {
        let mut b = vec![0u8; RECORD];
        b[NAME..NAME + name.len()].copy_from_slice(name.as_bytes());
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
        assert_eq!(of_character(&races, 0).unwrap().name, "Human");
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

    #[test]
    fn body_parameters() {
        let mut b = record("Human", "Human", "Default");
        // Neighbouring floats that are not body parameters stay unread.
        for at in (0x58..0xE4).step_by(4) {
            put(&mut b, at, -7.0);
        }
        put(&mut b, 0x60, 1.25);
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
        assert_eq!(body.pin_factor, 1.25);
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
                    clip_rate: 20.0 + r,
                    boost_below: 5.0 + r,
                }
            );
        }
        // The health factor right after the gait table is still its own.
        assert_eq!(races[0].health, 2.0);
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
        assert!(
            fallback
                .gaits
                .iter()
                .all(|g| g.thrust == 0.0 && g.push_scale == 0.0)
        );
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
        }
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
