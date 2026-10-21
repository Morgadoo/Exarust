//! Object-database sections an item record carries for storage: what a
//! container holds (`0x20000`), the random table a container slot can point
//! at (`0x200`), and the size and price the model section gives every item.
//! Layout and evidence: `docs/formats/OTHER.md` (Containers and random
//! tables).
//!
//! Each parser takes a section body (after the 8-byte header) and returns
//! `None` when the body is too short for what it reads.

/// The model section's tag.
pub const MODEL: u32 = 0x0010_0000;
/// A container's section.
pub const CONTAINER: u32 = 0x2_0000;
/// A random table's section.
pub const RANDOM_TABLE: u32 = 0x200;
/// A held light's section (the torch's).
pub const LIGHT: u32 = 0x100;
/// A usable item's section (keys, salves, the compass, levers).
pub const USABLE: u32 = 0x1_0000;

/// Bytes of one container child.
pub const CHILD_LEN: usize = 12;
/// Bytes of a random table's header and of each of its entries.
pub const TABLE_HEADER_LEN: usize = 16;
pub const TABLE_ENTRY_LEN: usize = 16;
/// Units per stored child position: a child's x and y are 1/80 of a GUI
/// unit.
pub const CHILD_POSITION_UNITS: f64 = 80.0;

/// Random-table entry flags.
pub mod entry {
    /// Only the table's count draws may pick the entry.
    pub const COUNT_ONLY: u16 = 0x2;
    /// The entry's id has a table of its own; a pick descends into it. Set
    /// when the tables are built, not stored.
    pub const NESTED: u16 = 0x8;
}

/// The body of the section with `tag` in a record payload (a chain of `u32
/// tag, u32 size` sections whose size counts the header). `None` when the
/// record has no such section or the chain is broken before it.
pub fn section(payload: &[u8], tag: u32) -> Option<&[u8]> {
    let mut pos = 0usize;
    while pos.checked_add(8)? <= payload.len() {
        let t = u32::from_le_bytes(payload.get(pos..pos + 4)?.try_into().ok()?);
        let size = u32::from_le_bytes(payload.get(pos + 4..pos + 8)?.try_into().ok()?) as usize;
        if size < 8 {
            return None;
        }
        let body = payload.get(pos + 8..pos.checked_add(size)?)?;
        if t == tag {
            return Some(body);
        }
        pos += size;
    }
    None
}

/// What the model section gives every item with a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelFacts {
    /// The model section's flags word (body `0x10`).
    pub flags: u32,
    /// How much room the item takes in a container (body `0x1C`).
    pub bulk: u8,
    /// The compact price word (body `0x1E`), see [`coins_of`].
    pub price: u16,
}

impl ModelFacts {
    pub fn parse(body: &[u8]) -> Option<Self> {
        let b = body.get(..0x20)?;
        Some(Self {
            flags: u32::from_le_bytes(b[0x10..0x14].try_into().ok()?),
            bulk: b[0x1C],
            price: u16::from_le_bytes(b[0x1E..0x20].try_into().ok()?),
        })
    }
}

/// Copper in a compact price word: the low 14 bits count units of 1440
/// copper with both top bits, 144 with the high bit, 12 with the next, and
/// single copper with neither.
pub fn coins_of(price: u16) -> u32 {
    let count = u32::from(price & 0x3FFF);
    match price & 0xC000 {
        0xC000 => count * 1440,
        0x8000 => count * 144,
        0x4000 => count * 12,
        _ => count,
    }
}

/// The compact price word for an amount of copper: single copper below
/// 0x4000, then twelves below 0x2FFF5, gross below 0x23FF70, else units of
/// 1440. Each division drops the remainder.
pub fn price_of(coins: u32) -> u16 {
    if coins < 0x4000 {
        coins as u16
    } else if coins < 0x2_FFF5 {
        (coins / 12) as u16 | 0x4000
    } else if coins < 0x23_FF70 {
        (coins / 144) as u16 | 0x8000
    } else {
        (coins / 1440) as u16 | 0xC000
    }
}

/// One item stored in a container: its id, its flags and where its icon
/// sits, in 1/80 of a GUI unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Child {
    pub id: u32,
    pub flags: u32,
    pub x: i16,
    pub y: i16,
}

/// A container's section: its panel size in GUI units, how much bulk it
/// takes, the largest single item, and what it holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Container {
    pub width: u16,
    pub height: u16,
    pub capacity: u16,
    pub max_item: u16,
    pub children: Vec<Child>,
}

impl Container {
    /// `None` when the body is shorter than its header or its stated
    /// children.
    pub fn parse(body: &[u8]) -> Option<Self> {
        let b = body.get(..0x18)?;
        let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        let count = u32::from_le_bytes(b[0x14..0x18].try_into().ok()?) as usize;
        let rest = body.get(0x18..0x18usize.checked_add(count.checked_mul(CHILD_LEN)?)?)?;
        let children = rest
            .as_chunks::<CHILD_LEN>()
            .0
            .iter()
            .map(|c| Child {
                id: u32::from_le_bytes(c[0..4].try_into().expect("4 bytes")),
                flags: u32::from_le_bytes(c[4..8].try_into().expect("4 bytes")),
                x: i16::from_le_bytes([c[8], c[9]]),
                y: i16::from_le_bytes([c[10], c[11]]),
            })
            .collect();
        Some(Self {
            width: u16_at(0),
            height: u16_at(2),
            capacity: u16_at(8),
            max_item: u16_at(0xA),
            children,
        })
    }
}

/// One row of a random table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TableEntry {
    /// The item, or another table, picked; 0 is the empty row.
    pub id: u32,
    /// Base weight; a row weighs this times 1000 before its first pick.
    pub weight: u8,
    /// Each pick keeps `1 − decay·0.005` of the weight.
    pub decay: u8,
    /// Picks the row is given before any weighted draw.
    pub quota: u8,
    /// Picks after which the row weighs nothing; 0 for no limit.
    pub limit: u8,
    /// [`entry`] flags.
    pub flags: u16,
    /// The condition word: wealth in bits 0–3, damage in 6–8 and its
    /// spread in 9–10, dirt in 11–13 and its spread in 14–15.
    pub condition: u32,
    /// The picked item's bulk; the table builder copies the model's into
    /// it.
    pub bulk: u8,
}

/// A random table: how many count draws it makes and its rows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RandomTable {
    pub min: u8,
    pub max: u8,
    pub entries: Vec<TableEntry>,
}

impl RandomTable {
    /// `None` when the body is shorter than its header or its stated rows.
    pub fn parse(body: &[u8]) -> Option<Self> {
        let h = body.get(..TABLE_HEADER_LEN)?;
        let count = u32::from_le_bytes(h[0xC..0x10].try_into().ok()?) as usize;
        let end = TABLE_HEADER_LEN.checked_add(count.checked_mul(TABLE_ENTRY_LEN)?)?;
        let rows = body.get(TABLE_HEADER_LEN..end)?;
        let entries = rows
            .as_chunks::<TABLE_ENTRY_LEN>()
            .0
            .iter()
            .map(|e| TableEntry {
                id: u32::from_le_bytes(e[0..4].try_into().expect("4 bytes")),
                weight: e[4],
                decay: e[5],
                quota: e[6],
                limit: e[7],
                flags: u16::from_le_bytes([e[8], e[9]]),
                condition: u32::from_le_bytes(e[10..14].try_into().expect("4 bytes")),
                bulk: e[15],
            })
            .collect();
        Some(Self {
            min: h[4],
            max: h[5],
            entries,
        })
    }
}

/// The condition a table row gives what it picks, as shares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowCondition {
    /// Bits 0–3 × 0.25.
    pub wealth: f32,
    /// Bits 6–8 × 0.25.
    pub damage: f32,
    /// Bits 9–10 × 0.25, or 0.125 when they are 0.
    pub damage_spread: f32,
    /// Bits 11–13 × 0.25.
    pub dirt: f32,
    /// Bits 14–15 × 0.25, or 0.125 when they are 0.
    pub dirt_spread: f32,
}

impl RowCondition {
    pub fn of(word: u32) -> Self {
        let spread = |bits: u32| {
            if bits == 0 { 0.125 } else { bits as f32 * 0.25 }
        };
        Self {
            wealth: (word & 0xF) as f32 * 0.25,
            damage: ((word & 0x1C0) >> 6) as f32 * 0.25,
            damage_spread: spread((word & 0x600) >> 9),
            dirt: ((word & 0x3800) >> 11) as f32 * 0.25,
            dirt_spread: spread((word & 0xFFFF) >> 14),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section_bytes(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend((body.len() as u32 + 8).to_le_bytes());
        v.extend_from_slice(body);
        v
    }

    #[test]
    fn a_section_is_found_by_its_tag() {
        let mut p = section_bytes(0x1000_0000, &[1; 24]);
        p.extend(section_bytes(CONTAINER, &[2; 24]));
        assert_eq!(section(&p, CONTAINER), Some(&[2u8; 24][..]));
        assert_eq!(section(&p, RANDOM_TABLE), None);
        let mut broken = p.clone();
        broken[4] = 4;
        assert_eq!(section(&broken, CONTAINER), None);
        assert_eq!(section(&p[..30], CONTAINER), None);
    }

    #[test]
    fn a_chest_and_its_children() {
        // A shipped chest: 65 by 50 units, 200 bulk, items up to 50.
        let mut body = vec![0x41, 0, 0x32, 0, 0, 0, 0, 0, 0xC8, 0, 0x32, 0];
        body.resize(0x14, 0);
        body.extend(2u32.to_le_bytes());
        body.extend(0x349u32.to_le_bytes());
        body.extend(0u32.to_le_bytes());
        body.extend(800i16.to_le_bytes());
        body.extend((-80i16).to_le_bytes());
        body.extend(0x8000_0007u32.to_le_bytes());
        body.extend(0x100u32.to_le_bytes());
        body.extend([0; 4]);
        let c = Container::parse(&body).unwrap();
        assert_eq!(
            (c.width, c.height, c.capacity, c.max_item),
            (65, 50, 200, 50)
        );
        assert_eq!(c.children.len(), 2);
        assert_eq!(
            (c.children[0].id, c.children[0].x, c.children[0].y),
            (0x349, 800, -80)
        );
        assert_eq!(c.children[1].flags, 0x100);
        assert_eq!(Container::parse(&body[..body.len() - 1]), None);
        assert_eq!(Container::parse(&body[..0x17]), None);
    }

    #[test]
    fn a_random_table_and_its_rows() {
        let mut body = vec![0u8; 16];
        body[4] = 1;
        body[5] = 3;
        body[0xC] = 2;
        body.extend([0u8; 10]);
        body.extend(0x5284u32.to_le_bytes());
        body.extend([0, 0]);
        body.extend(0x36u32.to_le_bytes());
        body.extend([10, 100, 1, 2, 0x80, 0]);
        body.extend(0x5484u32.to_le_bytes());
        body.extend([0, 9]);
        let t = RandomTable::parse(&body).unwrap();
        assert_eq!((t.min, t.max), (1, 3));
        assert_eq!(t.entries[0].id, 0);
        let row = t.entries[1];
        assert_eq!(
            (row.id, row.weight, row.decay, row.quota, row.limit),
            (0x36, 10, 100, 1, 2)
        );
        assert_eq!((row.flags, row.condition, row.bulk), (0x80, 0x5484, 9));
        assert_eq!(RandomTable::parse(&body[..40]), None);
    }

    #[test]
    fn the_shipped_condition_word() {
        // Clothes Tier 01's rows: full wealth, half damage spread a quarter,
        // half dirt spread a quarter.
        let c = RowCondition::of(0x5284);
        assert_eq!(c.wealth, 1.0);
        assert_eq!(c.damage, 0.5);
        assert_eq!(c.damage_spread, 0.25);
        assert_eq!(c.dirt, 0.5);
        assert_eq!(c.dirt_spread, 0.25);
        let zero = RowCondition::of(0);
        assert_eq!((zero.damage_spread, zero.dirt_spread), (0.125, 0.125));
    }

    #[test]
    fn the_compact_price_word() {
        assert_eq!(coins_of(0x3FFF), 0x3FFF);
        assert_eq!(coins_of(0x4000 | 5), 60);
        assert_eq!(coins_of(0x8000 | 5), 720);
        assert_eq!(coins_of(0xC000 | 5), 7200);
        assert_eq!(price_of(0x3FFF), 0x3FFF);
        assert_eq!(price_of(0x4000), 0x4000 | 0x555);
        assert_eq!(price_of(0x2_FFF4), 0x4000 | 0x3FFF);
        assert_eq!(price_of(0x2_FFF5), 0x8000 | 0x555);
        assert_eq!(price_of(0x23_FF70), 0xC000 | 0x666);
    }

    #[test]
    fn shipped_containers_and_tables() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let db = crate::rdb::Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let (mut containers, mut chests, mut tables, mut rows) = (0, 0, 0, 0);
        for r in &db.records {
            let payload = db.payload(r);
            if let Some(body) = section(payload, CONTAINER) {
                let c = Container::parse(body).expect("every shipped container parses");
                // The game's database lists none of their contents.
                assert!(c.children.is_empty());
                containers += 1;
                if (c.capacity, c.max_item) == (200, 50) {
                    chests += 1;
                }
            }
            if let Some(body) = section(payload, RANDOM_TABLE) {
                let t = RandomTable::parse(body).expect("every shipped table parses");
                // The empty row comes first in every shipped table.
                assert_eq!(t.entries[0].id, 0);
                assert_eq!((t.min, t.max), (0, 0));
                tables += 1;
                rows += t.entries.len();
            }
        }
        assert_eq!((containers, chests, tables, rows), (137, 24, 24, 438));
    }

    #[test]
    fn the_model_facts() {
        let mut body = vec![0u8; 0x28];
        body[0x10] = 1;
        body[0x1C] = 12;
        body[0x1E..0x20].copy_from_slice(&0x4003u16.to_le_bytes());
        let m = ModelFacts::parse(&body).unwrap();
        assert_eq!((m.flags, m.bulk, m.price), (1, 12, 0x4003));
        assert_eq!(ModelFacts::parse(&body[..0x1F]), None);
    }
}
