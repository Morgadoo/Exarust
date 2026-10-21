//! The arena outfit tables, as the original stores them before its
//! item-fact pass fills in ranks, points and bonuses.
//!
//! Every entry is a static pool record: the item id, its weight and its
//! masks, or for the small tables the id, weight, rank and cost words.
//! The item facts (rank, points, protection bonuses, weapon class) are
//! supplied at run time by the object database, exactly as the original
//! fills these rows on first use. Evidence: the tables read from the
//! shipped arena data; the construction pass is `fill_*` in
//! `arena_outfit`.

/// One raw large-pool row (0x68 bytes in the original, without its item
/// facts).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawPoolEntry {
    /// The item id.
    pub id: u32,
    /// The draw weight before the item-fact pass.
    pub weight: i32,
    /// The wanted-mask bits.
    pub wanted: u32,
    /// The forbidden-mask bits.
    pub forbidden: u32,
}

/// One raw small-pool row (0x10 bytes): id, weight and two words whose
/// meaning depends on the table (rank and cost, or class and rank).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawRankEntry {
    pub id: u32,
    pub weight: i32,
    pub first: i32,
    pub second: i32,
}

/// Alt A pool (3 rows).
pub const ALT_A: [RawPoolEntry; 3] = [
    RawPoolEntry {
        id: 69,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 147,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 400,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
];

/// Alt B pool (3 rows).
pub const ALT_B: [RawPoolEntry; 3] = [
    RawPoolEntry {
        id: 67,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 155,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 401,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
];

/// Alt C pool (1 rows).
pub const ALT_C: [RawPoolEntry; 1] = [RawPoolEntry {
    id: 518,
    weight: 10,
    wanted: 0x100,
    forbidden: 0x0,
}];

/// Alt D pool (2 rows).
pub const ALT_D: [RawPoolEntry; 2] = [
    RawPoolEntry {
        id: 515,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 520,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Armour A pool (14 rows).
pub const ARMOUR_A: [RawPoolEntry; 14] = [
    RawPoolEntry {
        id: 0,
        weight: 40,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 244,
        weight: 20,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 243,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 53,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 245,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 319,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 320,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 335,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 351,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 381,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 412,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 413,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 414,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1361,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Armour B pool (7 rows).
pub const ARMOUR_B: [RawPoolEntry; 7] = [
    RawPoolEntry {
        id: 244,
        weight: 20,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 245,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 319,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 320,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 335,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 412,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 414,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Belt pool (5 rows).
pub const BELT: [RawPoolEntry; 5] = [
    RawPoolEntry {
        id: 0,
        weight: 40,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 57,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1161,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1162,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1163,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Body Pool pool (33 rows).
pub const BODY_POOL: [RawPoolEntry; 33] = [
    RawPoolEntry {
        id: 0,
        weight: 150,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1353,
        weight: 1,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1354,
        weight: 1,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1355,
        weight: 1,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1356,
        weight: 1,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1357,
        weight: 1,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1358,
        weight: 1,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 158,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 392,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 394,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 522,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 523,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 630,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 628,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 627,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 409,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 504,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 6,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 3,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 389,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 390,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 521,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 524,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1186,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 391,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 393,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 81,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 151,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 510,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 511,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 512,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 513,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 514,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Cloak pool (2 rows).
pub const CLOAK: [RawPoolEntry; 2] = [
    RawPoolEntry {
        id: 0,
        weight: 50,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 279,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Coat pool (19 rows).
pub const COAT: [RawPoolEntry; 19] = [
    RawPoolEntry {
        id: 38,
        weight: 50,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 253,
        weight: 20,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 343,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 581,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 687,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 688,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 380,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 586,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 590,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 591,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 592,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 672,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 664,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 4,
        weight: 50,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 54,
        weight: 40,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 20,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x1,
    },
    RawPoolEntry {
        id: 572,
        weight: 40,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 573,
        weight: 40,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 584,
        weight: 40,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Feet pool (6 rows).
pub const FEET: [RawPoolEntry; 6] = [
    RawPoolEntry {
        id: 249,
        weight: 50,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 105,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 78,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 285,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 251,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 197,
        weight: 3,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Full Set pool (3 rows).
pub const FULL_SET: [RawPoolEntry; 3] = [
    RawPoolEntry {
        id: 0,
        weight: 100,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 621,
        weight: 0,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 686,
        weight: 0,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Hands pool (4 rows).
pub const HANDS: [RawPoolEntry; 4] = [
    RawPoolEntry {
        id: 0,
        weight: 50,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 10,
        weight: 10,
        wanted: 0x8,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 160,
        weight: 10,
        wanted: 0x8,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 216,
        weight: 10,
        wanted: 0x8,
        forbidden: 0x0,
    },
];

/// Head pool (7 rows).
pub const HEAD: [RawPoolEntry; 7] = [
    RawPoolEntry {
        id: 0,
        weight: 0,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 247,
        weight: 20,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 144,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 280,
        weight: 10,
        wanted: 0x4,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1359,
        weight: 10,
        wanted: 0x4,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 149,
        weight: 5,
        wanted: 0x4,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1360,
        weight: 5,
        wanted: 0x4,
        forbidden: 0x0,
    },
];

/// Hood pool (2 rows).
pub const HOOD: [RawPoolEntry; 2] = [
    RawPoolEntry {
        id: 0,
        weight: 0,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 890,
        weight: 10,
        wanted: 0xffff,
        forbidden: 0x0,
    },
];

/// Jewel pool (10 rows).
pub const JEWEL: [RawPoolEntry; 10] = [
    RawPoolEntry {
        id: 0,
        weight: 50,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 690,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 691,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 692,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 693,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x1,
    },
    RawPoolEntry {
        id: 694,
        weight: 1,
        wanted: 0x0,
        forbidden: 0x1,
    },
    RawPoolEntry {
        id: 699,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 700,
        weight: 4,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 701,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x1,
    },
    RawPoolEntry {
        id: 702,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Main A pool (10 rows).
pub const MAIN_A: [RawPoolEntry; 10] = [
    RawPoolEntry {
        id: 81,
        weight: 50,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 151,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 391,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 393,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 510,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 511,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 512,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 513,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 514,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 631,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
];

/// Main B pool (4 rows).
pub const MAIN_B: [RawPoolEntry; 4] = [
    RawPoolEntry {
        id: 86,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 153,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 398,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 516,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
];

/// Main C pool (4 rows).
pub const MAIN_C: [RawPoolEntry; 4] = [
    RawPoolEntry {
        id: 85,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 154,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 399,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 517,
        weight: 10,
        wanted: 0x100,
        forbidden: 0x0,
    },
];

/// Neck pool (4 rows).
pub const NECK: [RawPoolEntry; 4] = [
    RawPoolEntry {
        id: 0,
        weight: 50,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 126,
        weight: 5,
        wanted: 0x2,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 408,
        weight: 5,
        wanted: 0x2,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 402,
        weight: 10,
        wanted: 0x4,
        forbidden: 0x0,
    },
];

/// Ranked Armour pool (11 rows).
pub const RANKED_ARMOUR: [RawRankEntry; 11] = [
    RawRankEntry {
        id: 440,
        weight: 0,
        first: 5,
        second: 0,
    },
    RawRankEntry {
        id: 12,
        weight: 5,
        first: 5,
        second: 0,
    },
    RawRankEntry {
        id: 65,
        weight: 40,
        first: 5,
        second: 0,
    },
    RawRankEntry {
        id: 212,
        weight: 20,
        first: 5,
        second: 0,
    },
    RawRankEntry {
        id: 26,
        weight: 0,
        first: 5,
        second: 0,
    },
    RawRankEntry {
        id: 419,
        weight: 0,
        first: 5,
        second: 0,
    },
    RawRankEntry {
        id: 213,
        weight: 0,
        first: 5,
        second: 0,
    },
    RawRankEntry {
        id: 13,
        weight: 0,
        first: 5,
        second: 0,
    },
    RawRankEntry {
        id: 22,
        weight: 0,
        first: 5,
        second: 0,
    },
    RawRankEntry {
        id: 30,
        weight: 5,
        first: 5,
        second: 0,
    },
    RawRankEntry {
        id: 1099,
        weight: 5,
        first: 5,
        second: 0,
    },
];

/// Shields pool (10 rows).
pub const SHIELDS: [RawRankEntry; 10] = [
    RawRankEntry {
        id: 80,
        weight: 10,
        first: 1,
        second: 4,
    },
    RawRankEntry {
        id: 36,
        weight: 10,
        first: 2,
        second: 6,
    },
    RawRankEntry {
        id: 150,
        weight: 10,
        first: 3,
        second: 6,
    },
    RawRankEntry {
        id: 15,
        weight: 10,
        first: 3,
        second: 6,
    },
    RawRankEntry {
        id: 59,
        weight: 10,
        first: 3,
        second: 7,
    },
    RawRankEntry {
        id: 186,
        weight: 5,
        first: 3,
        second: 7,
    },
    RawRankEntry {
        id: 576,
        weight: 10,
        first: 3,
        second: 6,
    },
    RawRankEntry {
        id: 578,
        weight: 10,
        first: 3,
        second: 7,
    },
    RawRankEntry {
        id: 579,
        weight: 5,
        first: 3,
        second: 7,
    },
    RawRankEntry {
        id: 580,
        weight: 10,
        first: 3,
        second: 6,
    },
];

/// Torso A pool (17 rows).
pub const TORSO_A: [RawPoolEntry; 17] = [
    RawPoolEntry {
        id: 0,
        weight: 120,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 89,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 40,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x1,
    },
    RawPoolEntry {
        id: 410,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 697,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 536,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 633,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 637,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 411,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 426,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 525,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 529,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 530,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 543,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 545,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1182,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Torso B pool (11 rows).
pub const TORSO_B: [RawPoolEntry; 11] = [
    RawPoolEntry {
        id: 0,
        weight: 120,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 203,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 281,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 430,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 574,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 575,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 577,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 582,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 583,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 795,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 796,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Torso C pool (12 rows).
pub const TORSO_C: [RawPoolEntry; 12] = [
    RawPoolEntry {
        id: 0,
        weight: 120,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 119,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 640,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 118,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 173,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 333,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 336,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 539,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 649,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 651,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 652,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1183,
        weight: 2,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Torso D pool (10 rows).
pub const TORSO_D: [RawPoolEntry; 10] = [
    RawPoolEntry {
        id: 0,
        weight: 100,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 111,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 43,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 183,
        weight: 7,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 642,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 93,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 321,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 427,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 541,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 650,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Torso E pool (4 rows).
pub const TORSO_E: [RawPoolEntry; 4] = [
    RawPoolEntry {
        id: 0,
        weight: 100,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 334,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 428,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 540,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Torso F pool (5 rows).
pub const TORSO_F: [RawPoolEntry; 5] = [
    RawPoolEntry {
        id: 0,
        weight: 50,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 283,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 55,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 655,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 537,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Torso G pool (2 rows).
pub const TORSO_G: [RawPoolEntry; 2] = [
    RawPoolEntry {
        id: 0,
        weight: 50,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 35,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Torso H pool (5 rows).
pub const TORSO_H: [RawPoolEntry; 5] = [
    RawPoolEntry {
        id: 0,
        weight: 100,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 538,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 515,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 520,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 1184,
        weight: 1,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Torso I pool (6 rows).
pub const TORSO_I: [RawPoolEntry; 6] = [
    RawPoolEntry {
        id: 0,
        weight: 100,
        wanted: 0xffff,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 242,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 241,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 332,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 415,
        weight: 5,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 535,
        weight: 10,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Underwear pool (5 rows).
pub const UNDERWEAR: [RawPoolEntry; 5] = [
    RawPoolEntry {
        id: 4,
        weight: 50,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 54,
        weight: 40,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 572,
        weight: 40,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 573,
        weight: 40,
        wanted: 0x0,
        forbidden: 0x0,
    },
    RawPoolEntry {
        id: 584,
        weight: 40,
        wanted: 0x0,
        forbidden: 0x0,
    },
];

/// Weapons A pool (27 rows).
pub const WEAPONS_A: [RawRankEntry; 27] = [
    RawRankEntry {
        id: 23,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 222,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 224,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 262,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 282,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 952,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 897,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 79,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 206,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 395,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 225,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 127,
        weight: 5,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 954,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 210,
        weight: 30,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 19,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 419,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 213,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 212,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 26,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 30,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 422,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 440,
        weight: 60,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 12,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 13,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 22,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 65,
        weight: 40,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 1099,
        weight: 5,
        first: 0,
        second: 0,
    },
];

/// Weapons B pool (14 rows).
pub const WEAPONS_B: [RawRankEntry; 14] = [
    RawRankEntry {
        id: 227,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 396,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 232,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 25,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 37,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 32,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 420,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 60,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 21,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 423,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 424,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 24,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 1166,
        weight: 40,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 673,
        weight: 0,
        first: 0,
        second: 0,
    },
];

/// Weapons C pool (14 rows).
pub const WEAPONS_C: [RawRankEntry; 14] = [
    RawRankEntry {
        id: 237,
        weight: 3,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 406,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 46,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 39,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 41,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 407,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 421,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 159,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 62,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 63,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 953,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 33,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 34,
        weight: 0,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 61,
        weight: 0,
        first: 0,
        second: 0,
    },
];

/// Weapons D pool (4 rows).
pub const WEAPONS_D: [RawRankEntry; 4] = [
    RawRankEntry {
        id: 420,
        weight: 1,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 423,
        weight: 1,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 424,
        weight: 1,
        first: 0,
        second: 0,
    },
    RawRankEntry {
        id: 423,
        weight: 1,
        first: 0,
        second: 0,
    },
];

/// Profession grid weights for the common side: four rows of eight columns.
/// A zero weight is a cell that is never drawn.
pub const PROFESSION_WEIGHTS_COMMON: [[i32; 8]; 4] = [
    [20, 12, 16, 12, 0, 0, 0, 2],
    [0, 0, 0, 0, 0, 0, 0, 0],
    [12, 8, 4, 8, 4, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 0, 0],
];

/// Profession grid weights for the rare side, drawn when the drift roll is
/// zero.
pub const PROFESSION_WEIGHTS_RARE: [[i32; 8]; 4] = [
    [20, 12, 16, 4, 0, 0, 0, 1],
    [0, 0, 0, 0, 0, 0, 0, 0],
    [3, 2, 1, 2, 1, 0, 0, 0],
    [20, 12, 8, 8, 4, 4, 0, 0],
];

/// First prerequisite link per cell: a one-based column of the same row that
/// must already be marked, or zero for none.
pub const PROFESSION_PREREQ_A: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 0, 0, 0, 0, 0],
    [0, 1, 2, 0, 0, 0, 0, 0],
];

/// Second prerequisite link per cell, same meaning as the first.
pub const PROFESSION_PREREQ_B: [[u8; 8]; 4] = [[0, 0, 0, 0, 0, 0, 0, 4], [0; 8], [0; 8], [0; 8]];
