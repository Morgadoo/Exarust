//! Arena trading: a shop's stock, its daily rotation, a merchant's
//! specialisation and the transaction a confirmed trade makes.
//!
//! A first stock draws 18 to 20 items. Later, a stock under 20 drops 2 to 5
//! of its oldest items and gains 4 to 7; one under 24 drops and gains 4 to
//! 7; a larger one drops 4 to 7 and gains 2 to 5. Candidates are the object
//! database's items of the asked kinds (apparel, weapons and shields unless
//! a merchant specialises) that are listed, have a named model with both
//! size words set and, when a rank is asked, that rank. An item of rarity
//! `r` weighs `round(10 · (10s)^(1 + s))` with `s = 1 − r/100`, computed
//! with the original's quick logarithm and exponential; it is skipped when
//! `s` is not above 0.005. Each copy already on the shelf divides the weight
//! by 3, and weapons and shields are halved. Each draw steps the roll by
//! `× 0x08088405 + 1` and takes the first item whose running weight passes
//! the high half of the total times the roll. Apparel and weapons are made
//! as new copies; anything else is stocked as it is.
//!
//! A transaction takes the net price from the company's coins (none left
//! below 1 copper), adds its two tallies, appends the items sold to the
//! stock and removes the items bought.
//!
//! Evidence: REA on the trade unit (the stock update and its helpers, the
//! transaction, the merchant's dialog and its two option callbacks, which
//! the disassembly gives) and on the engine's quick logarithm and
//! exponential.

use exa_formats::trade_record::TradeRecord;

use crate::career::COIN;

/// Apparel, weapons and shields: the kinds a shop stocks by default.
pub const DEFAULT_KINDS: u32 = 0xd000;
/// An item body: apparel made as a new copy.
pub const APPAREL: u32 = 0x1000;
/// A weapon section: a weapon made as a new copy.
pub const WEAPON: u32 = 0x4000;
/// Weapons and shields: their weight is halved.
pub const ARMS: u32 = 0xc000;
/// Ids with either of these bits are item instances, not database items.
pub const INSTANCE: u32 = 0xc000_0000;
/// A first stock draws this many items, plus 0 to 2.
pub const FIRST_STOCK: i32 = 18;
/// Asking all kinds for this rank asks for the rank below.
pub const TOP_RANK: i32 = 5;
/// The roll's step.
pub const ROLL_MUL: u32 = 0x0808_8405;
/// The share below which an item is never stocked, as a double.
pub const RARE_LIMIT: f64 = 0.005;
/// The merchant's category options, 1-based, and the kinds each asks.
pub const CATEGORIES: [(&str, u32); 3] = [
    ("Anything", 0),
    ("Clothing & Armour", 0x1000),
    ("Weapons", 0xc000),
];
/// The merchant's rank options, 1-based. Option `n` asks rank `n − 2`, so
/// Any asks none.
pub const RANKS: [&str; 7] = [
    "Any", "Inept", "Aspirant", "Novice", "Adept", "Expert", "Master",
];
/// The merchant's dialog: its four placement words, and the style bits it
/// clears.
pub const MERCHANT_FORM: [i32; 4] = [300, 400, 260, 68];
pub const MERCHANT_FORM_CLEARS: u32 = 0x0002_0020;

/// One entry of the object database's table, in id order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopEntry {
    pub id: u32,
    /// The OR of its record's section tags.
    pub flags: u32,
    /// Its shop fields; `None` for an entry without a record.
    pub record: Option<TradeRecord>,
}

/// What a stock update reads and makes.
pub trait ShopWorld {
    /// The object database's table, sorted by id.
    fn entries(&self) -> &[ShopEntry];
    /// The item an instance was copied from, or `None` without a record.
    fn parent_of(&self, instance: u32) -> Option<u32>;
    /// Free an instance dropped from the shelf.
    fn release(&mut self, instance: u32);
    /// A new copy of an apparel item (see [`apparel_condition`]). `seed`
    /// is the variant roll.
    fn apparel_copy(&mut self, id: u32, seed: &mut u32) -> u32;
    /// A new copy of a weapon, with its variant and condition rolled.
    fn weapon_copy(&mut self, id: u32, seed: &mut u32) -> u32;
}

/// What a stock update did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StockReport {
    /// Pairs of shelf places holding the same instance, before and after.
    pub duplicates_before: Vec<(usize, usize)>,
    pub duplicates_after: Vec<(usize, usize)>,
    /// Draws whose item the table did not hold.
    pub invalid: u32,
    /// The variant roll after the update.
    pub seed: u32,
}

/// The roll's next value.
pub fn step(roll: u32) -> u32 {
    roll.wrapping_mul(ROLL_MUL).wrapping_add(1)
}

/// Pairs of places holding the same instance id. Plain database ids may
/// repeat.
pub fn duplicates(stock: &[u32]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for (i, &id) in stock.iter().enumerate() {
        if id & INSTANCE == 0 {
            continue;
        }
        for (j, &other) in stock.iter().enumerate().skip(i + 1) {
            if other == id {
                pairs.push((i, j));
            }
        }
    }
    pairs
}

/// Whether the stock holds an instance. A plain database id is never held.
pub fn holds(stock: &[u32], id: u32) -> bool {
    id & INSTANCE != 0 && stock.contains(&id)
}

/// The table index of an id, by the original's halving search on id
/// differences.
pub fn find_entry(entries: &[ShopEntry], id: u32) -> Option<usize> {
    let mut low = 0i64;
    let mut high = entries.len() as i64 - 1;
    while low <= high {
        let mid = (high - low) / 2 + low;
        let diff = (id as i32).wrapping_sub(entries[mid as usize].id as i32);
        if diff < 0 {
            high = mid - 1;
        } else if diff >= 1 {
            low = mid + 1;
        } else {
            return Some(mid as usize);
        }
    }
    None
}

/// The engine's quick natural logarithm: the exponent times ln 2 plus a
/// polynomial in the mantissa. Not positive gives minus infinity.
pub fn quick_ln(x: f32) -> f32 {
    let bits = x.to_bits();
    let m = f32::from_bits(bits & 0x007f_ffff | 0x3f80_0000);
    let exponent = ((bits >> 23) as i32) as f32;
    let base = if x > 0.0 || x.is_nan() {
        f32::from_bits(0xc2b3_de55)
    } else {
        f32::NEG_INFINITY
    };
    let m2 = m * m;
    let mut t = (m * f32::from_bits(0x3ccc_a7fc) - f32::from_bits(0x3e79_8f62)) * m2;
    t += m - f32::from_bits(0x4011_9122);
    let lead = m * f32::from_bits(0x4059_7523) + base;
    t = t * m2 + exponent * f32::from_bits(0x3f31_7218);
    lead + t
}

/// The engine's quick exponential: the scaled argument's bits split into
/// an exponent and a mantissa polynomial, clamped to the float range.
pub fn quick_exp(x: f32) -> f32 {
    let top = f32::from_bits(0x4eff_0000);
    let y = x * f32::from_bits(0x4b38_aa3b) + f32::from_bits(0x4e7e_0000);
    let y = if y < top { y } else { top };
    let i = round_i32(y);
    let i = if i > 0 { i as u32 } else { 0 };
    let m = f32::from_bits(i & 0x007f_ffff | 0x3f80_0000);
    let p = (((m * f32::from_bits(0x3c5d_be77) + f32::from_bits(0xbb0b_4858)) * m
        + f32::from_bits(0x3e2a_9cbe))
        * m
        + f32::from_bits(0x3e9f_c08b))
        * m
        + f32::from_bits(0x3f02_8d05);
    f32::from_bits(i & 0x7f80_0000) * p
}

/// Round to nearest, ties to even, into 32 bits; out of range is the
/// integer indefinite.
fn round_i32(value: f32) -> i32 {
    let r = value.round_ties_even();
    if r.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&r) {
        i32::MIN
    } else {
        r as i32
    }
}

/// Round to nearest, ties to even, into 64 bits, keeping the low 32.
fn round_low(value: f32) -> i32 {
    let r = value.round_ties_even();
    if r.is_nan() || !(-9.223_372e18..9.223_372e18).contains(&r) {
        0
    } else {
        (r as i64) as i32
    }
}

/// The draw weight of an item of `rarity` hundredths, or `None` when its
/// share is not above 0.005.
pub fn rarity_weight(rarity: u8) -> Option<i32> {
    let share = (1.0f64 - f64::from(rarity) * 0.01) as f32;
    // A byte's share is never NaN, so this is the original's "not above".
    if f64::from(share) <= RARE_LIMIT {
        return None;
    }
    let grow = 1.0f32 + share;
    Some(round_low(quick_exp(quick_ln(share * 10.0) * grow) * 10.0))
}

/// Whether an entry may be stocked for `kinds` and `rank` (negative: any).
fn candidate(entry: &ShopEntry, kinds: u32, rank: i32) -> Option<u8> {
    if kinds & entry.flags == 0 {
        return None;
    }
    let record = entry.record?;
    if !record.listed() {
        return None;
    }
    let model = record.model?;
    if model.size[0] == 0 || model.size[1] == 0 || !model.named {
        return None;
    }
    if rank >= 0 && i32::from(record.rank) != rank {
        return None;
    }
    Some(model.rarity)
}

/// Rotate a shop's stock, oldest first. `kinds` 0 asks the default kinds;
/// `rank` −1 asks any rank. `rolls` are the two performance-counter rolls
/// the original takes: the draw and the variant seed.
pub fn update_stock(
    stock: &mut Vec<u32>,
    kinds: u32,
    rank: i32,
    rolls: [u32; 2],
    world: &mut impl ShopWorld,
) -> StockReport {
    let mut report = StockReport {
        duplicates_before: duplicates(stock),
        ..StockReport::default()
    };
    let (kinds, rank) = if kinds == 0 {
        (DEFAULT_KINDS, rank)
    } else if kinds == DEFAULT_KINDS && rank == TOP_RANK {
        (kinds, TOP_RANK - 1)
    } else {
        (kinds, rank)
    };
    let mut roll = rolls[0];
    let mut seed = rolls[1];
    let mut target;
    if stock.is_empty() {
        roll = step(roll);
        target = ((u64::from(roll) * 3) >> 32) as i32 + FIRST_STOCK;
    } else {
        let count = stock.len() as i32;
        let (add, drop) = if count < 20 {
            (4, 2)
        } else if count < 24 {
            (4, 4)
        } else {
            (2, 4)
        };
        roll = step(roll);
        let add = (roll >> 30) as i32 + add;
        roll = step(roll);
        let drop = ((roll >> 30) as i32 + drop).min(count);
        for &id in &stock[..drop as usize] {
            if id & INSTANCE != 0 {
                world.release(id);
            }
        }
        stock.drain(..drop as usize);
        target = stock.len() as i32 + add;
    }
    let shelved: Vec<u32> = stock
        .iter()
        .map(|&id| {
            if id & INSTANCE == 0 {
                id
            } else {
                world.parent_of(id).unwrap_or(0)
            }
        })
        .collect();
    let mut weights: Vec<(u32, i32)> = Vec::new();
    let mut counted = 0usize;
    let mut total = 0i32;
    for entry in world.entries() {
        let Some(rarity) = candidate(entry, kinds, rank) else {
            continue;
        };
        counted += 1;
        let Some(mut weight) = rarity_weight(rarity) else {
            continue;
        };
        for &base in &shelved {
            if entry.id == base {
                weight = (i64::from(weight) / 3) as i32;
            }
        }
        if weight < 1 {
            continue;
        }
        if entry.flags & ARMS != 0 {
            weight = ((weight as u32) >> 1) as i32;
        }
        total = total.wrapping_add(weight);
        weights.resize(counted, (0, 0));
        weights[counted - 1] = (entry.id, weight);
    }
    let mut placed = stock.len() as i32;
    while target > placed {
        roll = step(roll);
        let pick = ((i64::from(total).wrapping_mul(i64::from(roll)) as u64) >> 32) as u32 as i32;
        let mut running = 0i32;
        let mut chosen = None;
        for index in 0..counted {
            // Entries counted after the last weighed one read as nothing.
            running = running.wrapping_add(weights.get(index).map_or(0, |w| w.1));
            if running > pick {
                chosen = Some(index);
                break;
            }
        }
        let Some(index) = chosen else {
            target -= 1;
            continue;
        };
        let id = weights.get(index).map_or(0, |w| w.0);
        let Some(at) = find_entry(world.entries(), id) else {
            report.invalid += 1;
            target -= 1;
            continue;
        };
        let flags = world.entries()[at].flags;
        let made = if flags & APPAREL != 0 {
            world.apparel_copy(id, &mut seed)
        } else if flags & WEAPON != 0 {
            world.weapon_copy(id, &mut seed)
        } else {
            id
        };
        stock.push(made);
        placed += 1;
    }
    report.duplicates_after = duplicates(stock);
    report.seed = seed;
    report
}

/// The shop's first stock: an empty shelf draws [`FIRST_STOCK`] to two more
/// items with the default kinds and any rank. The original calls the stock
/// update with `(stock, 0, −1)` in the new career and again when the shop
/// has no stock on a click.
pub fn first_stock(
    stock: &mut Vec<u32>,
    rolls: [u32; 2],
    world: &mut impl ShopWorld,
) -> StockReport {
    update_stock(stock, 0, -1, rolls, world)
}

/// Fill an empty stock and leave a shelf that already has items alone. The
/// hub click's first-stock guard: `None` when there was nothing to do.
pub fn ensure_first_stock(
    stock: &mut Vec<u32>,
    rolls: [u32; 2],
    world: &mut impl ShopWorld,
) -> Option<StockReport> {
    stock.is_empty().then(|| first_stock(stock, rolls, world))
}

/// The variant roll's next unit in [0, 1): the seed stepped, times 2^−32,
/// as a float.
pub fn variant_unit(seed: &mut u32) -> f32 {
    *seed = step(*seed);
    (f64::from(*seed) * f64::from_bits(0x3df0_0000_0000_0000)) as f32
}

/// A shop's new apparel copy, on its item body: the condition byte at
/// `0x1a9` keeps its low two bits and takes a quality `q = 2u − 1` from the
/// variant roll, `round(|31q|)` in bits 2 to 6 and bit 7 when `q < 0`; the
/// damage byte becomes 0x40, the dirt byte 0x35, and the byte after them
/// takes 5 in bits 3 to 5 and 3 in bits 6 and 7. A body too short is left.
pub fn apparel_condition(body: &mut [u8], seed: &mut u32) {
    if body.len() < 0x1b4 {
        return;
    }
    let quality = variant_unit(seed) * 2.0 - 1.0;
    let size = round_low((quality * 31.0).abs());
    let mut byte = body[0x1a9] & 3 | (size << 2) as u8;
    if quality < 0.0 {
        byte |= 0x80;
    }
    body[0x1a9] = byte;
    body[0x1b1] = 0x40;
    body[0x1b3] = body[0x1b3] & 0xc7 | (three_bits(1.0, 5) << 3) as u8;
    body[0x1b2] = 0x35;
    body[0x1b3] = body[0x1b3] & 0x3f | (three_bits(1.0, 3) << 6) as u8;
}

/// `round(8v)`; above 1 it becomes half of that plus one, at most `max`.
fn three_bits(value: f32, max: i32) -> i32 {
    let r = round_low(value * 8.0);
    if r > 1 {
        (((r as u32) >> 1) as i32 + 1).min(max)
    } else {
        r
    }
}

/// The kinds the merchant's category option asks, or `None` past the list.
pub fn category_for_option(option: i32) -> Option<u32> {
    usize::try_from(option.wrapping_sub(1))
        .ok()
        .and_then(|i| CATEGORIES.get(i))
        .map(|c| c.1)
}

/// The rank the merchant's rank option asks: two less than the option.
pub fn rank_for_option(option: i32) -> i32 {
    option.wrapping_sub(2)
}

/// The options the dialog opens on: the category whose kinds match, else
/// `previous`, and the rank plus two.
pub fn options_for(kinds: u32, rank: i32, previous: i32) -> (i32, i32) {
    let mut category = previous;
    for (index, &(_, asked)) in CATEGORIES.iter().enumerate() {
        if asked == kinds {
            category = index as i32 + 1;
        }
    }
    (category, rank.wrapping_add(2))
}

/// Copper in a coin dword: the low byte, then silver at 12, then gold at
/// 144 in the high 16 bits.
pub fn coins_value(packed: u32) -> i32 {
    ((packed & 0xff) + ((packed >> 8) & 0xff) * COIN + (packed >> 16) * COIN * COIN) as i32
}

/// Add copper to a coin dword. A sum under 1 empties it; otherwise it is
/// packed again, gold kept to 16 bits.
pub fn add_coins(packed: u32, copper: i32) -> u32 {
    let sum = coins_value(packed).wrapping_add(copper);
    if sum < 1 {
        return 0;
    }
    let value = sum as u32;
    let gold = (value / (COIN * COIN)) as u16;
    let rest = value.wrapping_sub(u32::from(gold) * COIN * COIN);
    let silver = (rest / COIN) as u8;
    let copper = (rest as u8).wrapping_sub(silver.wrapping_mul(COIN as u8));
    u32::from(copper) | u32::from(silver) << 8 | u32::from(gold) << 16
}

/// A confirmed trade.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Transaction {
    /// Copper the company pays; negative when it receives.
    pub due: i32,
    /// Copper added to the two tallies the shop keeps.
    pub tallies: [i32; 2],
    /// Items moved from the company onto the shelf, in order.
    pub sold: Vec<u32>,
    /// Items taken off the shelf.
    pub bought: Vec<u32>,
}

/// Apply a trade to the coins, the tallies and the stock. The money moves
/// only when something is due. Each bought item leaves the shelf at its
/// first place, the rest keeping their order. Returns the stock's duplicate
/// pairs afterwards.
pub fn settle(
    money: &mut u32,
    tallies: &mut [u32; 2],
    stock: &mut Vec<u32>,
    trade: &Transaction,
) -> Vec<(usize, usize)> {
    if trade.due != 0 {
        *money = add_coins(*money, trade.due.wrapping_neg());
    }
    for (tally, add) in tallies.iter_mut().zip(trade.tallies) {
        *tally = add_coins(*tally, add);
    }
    stock.extend(&trade.sold);
    for id in &trade.bought {
        if let Some(at) = stock.iter().position(|s| s == id) {
            stock.remove(at);
        }
    }
    duplicates(stock)
}

#[cfg(test)]
#[path = "trade_test.rs"]
mod tests;
