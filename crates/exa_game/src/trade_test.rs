use super::*;
use exa_formats::trade_record::TradeModel;
use std::collections::HashMap;

fn record(rarity: u8, rank: u8, listing: u32) -> Option<TradeRecord> {
    Some(TradeRecord {
        listing: Some(listing),
        parent: 0,
        model: Some(TradeModel {
            named: true,
            flags: 0,
            rarity,
            size: [1, 1],
        }),
        rank,
    })
}

/// Apparel 10 (rarity 0, rank 2), weapon 20 (rarity 50, rank 1), a plain
/// object 30, shield 40 (rarity 80, rank 0), unlisted apparel 50 and
/// apparel 60 too rare to draw.
struct World {
    entries: Vec<ShopEntry>,
    parents: HashMap<u32, u32>,
    released: Vec<u32>,
}

impl World {
    fn new() -> Self {
        let entry = |id, flags, record| ShopEntry { id, flags, record };
        Self {
            entries: vec![
                entry(10, 0x1010_1000, record(0, 2, 0)),
                entry(20, 0x1010_4000, record(50, 1, 0)),
                entry(30, 0x1010_0000, record(0, 0, 0)),
                entry(40, 0x1010_8000, record(80, 0, 0)),
                entry(50, 0x1010_1000, record(0, 2, 2)),
                entry(60, 0x1010_1000, record(100, 2, 0)),
            ],
            parents: HashMap::new(),
            released: Vec::new(),
        }
    }

    fn copy(&mut self, id: u32, seed: &mut u32) -> u32 {
        *seed = step(*seed);
        let instance = 0x8000_0000 | self.parents.len() as u32;
        self.parents.insert(instance, id);
        instance
    }
}

impl ShopWorld for World {
    fn entries(&self) -> &[ShopEntry] {
        &self.entries
    }
    fn parent_of(&self, instance: u32) -> Option<u32> {
        self.parents.get(&instance).copied()
    }
    fn release(&mut self, instance: u32) {
        self.released.push(instance);
    }
    fn apparel_copy(&mut self, id: u32, seed: &mut u32) -> u32 {
        self.copy(id, seed)
    }
    fn weapon_copy(&mut self, id: u32, seed: &mut u32) -> u32 {
        self.copy(id, seed)
    }
}

/// Rarity r weighs round(10 · (10s)^(1+s)) with s = 1 − r/100, by the
/// quick logarithm and exponential: 1000, 650, 422, 112, 23, 10, 1, and
/// nothing at 100.
#[test]
fn rarity_sets_the_weight() {
    let weights: Vec<_> = [0u8, 10, 20, 50, 80, 90, 99, 100]
        .into_iter()
        .map(rarity_weight)
        .collect();
    assert_eq!(
        weights,
        [
            Some(1000),
            Some(650),
            Some(422),
            Some(112),
            Some(23),
            Some(10),
            Some(1),
            None
        ]
    );
    assert_eq!(quick_ln(10.0).to_bits(), 0x4013_5da0);
    assert_eq!(quick_ln(0.0), f32::NEG_INFINITY);
    assert_eq!(quick_exp(0.0).to_bits(), 0x3f80_0016);
    assert_eq!(quick_exp(1.0).to_bits(), 0x402d_f826);
    assert_eq!(quick_exp(-200.0), 0.0);
}

/// Seed 7 first steps to a roll whose high third is 0: 18 items. Apparel
/// 10 weighs 1000 against 56 and 11, and every draw takes it. Each is a new
/// copy, stepping the variant seed from 99 eighteen times.
#[test]
fn a_first_stock_draws_eighteen_to_twenty() {
    let mut world = World::new();
    let mut stock = Vec::new();
    let report = update_stock(&mut stock, 0, -1, [7, 99], &mut world);
    assert_eq!(stock.len(), 18);
    assert!(stock.iter().all(|id| world.parents[id] == 10));
    assert_eq!(stock[17], 0x8000_0011);
    assert_eq!(report.seed, 0xa311_c561);
    assert!(report.duplicates_before.is_empty() && report.duplicates_after.is_empty());
}

/// Eighteen on the shelf: roll 11 drops 2 and adds 5. Sixteen copies of
/// apparel 10 divide its weight by 3 sixteen times to nothing, so the
/// weapon (56) and the shield (11) share the draws: four weapon copies and
/// the plain shield id.
#[test]
fn a_restock_rotates_and_shuns_what_it_holds() {
    let mut world = World::new();
    let mut stock = Vec::new();
    update_stock(&mut stock, 0, -1, [7, 99], &mut world);
    let report = update_stock(&mut stock, 0, -1, [11, 5], &mut world);
    assert_eq!(world.released, [0x8000_0000, 0x8000_0001]);
    assert_eq!(stock.len(), 21);
    let bases: Vec<u32> = stock[16..]
        .iter()
        .map(|id| world.parents.get(id).copied().unwrap_or(*id))
        .collect();
    assert_eq!(bases, [20, 40, 20, 20, 20]);
    assert_eq!(stock[17], 40);
    assert_eq!(report.seed, 0x8de5_74d1);
}

/// A rank asks only that rank; the default kinds asked by name turn rank 5
/// into 4, and kinds that match nothing draw nothing.
#[test]
fn rank_and_kinds_filter_the_candidates() {
    let mut world = World::new();
    let mut weapons = Vec::new();
    update_stock(&mut weapons, ARMS, 1, [7, 0], &mut world);
    assert_eq!(weapons.len(), 18);
    assert!(weapons.iter().all(|id| world.parents[id] == 20));
    let mut shields = Vec::new();
    update_stock(&mut shields, ARMS, 0, [7, 0], &mut world);
    assert_eq!(shields, [40; 18]);
    let mut none = Vec::new();
    update_stock(&mut none, DEFAULT_KINDS, TOP_RANK, [7, 0], &mut world);
    assert!(none.is_empty());
    let mut empty = Vec::new();
    update_stock(&mut empty, 0x2, -1, [7, 0], &mut world);
    assert!(empty.is_empty());
}

#[test]
fn duplicates_and_the_halving_search() {
    let stock = [0x8000_0001, 5, 5, 0x8000_0001, 0x4000_0002, 0x4000_0002];
    assert_eq!(duplicates(&stock), [(0, 3), (4, 5)]);
    assert!(holds(&stock, 0x8000_0001));
    assert!(!holds(&stock, 5));
    let world = World::new();
    assert_eq!(find_entry(&world.entries, 40), Some(3));
    assert_eq!(find_entry(&world.entries, 45), None);
    assert_eq!(find_entry(&[], 1), None);
}

/// Seed 0 steps to 1, a unit of 2^−32: q rounds to −1, giving 31 in bits
/// 2 to 6 and the sign bit, over the low two bits kept.
#[test]
fn a_new_apparel_copy_takes_its_condition() {
    let mut body = vec![0u8; 0x1b4];
    body[0x1a9] = 0x7f;
    body[0x1b3] = 0x07;
    let mut seed = 0;
    apparel_condition(&mut body, &mut seed);
    assert_eq!(seed, 1);
    assert_eq!(body[0x1a9], 0x80 | 31 << 2 | 3);
    assert_eq!((body[0x1b1], body[0x1b2], body[0x1b3]), (0x40, 0x35, 0xef));
    let mut short = vec![0u8; 0x1b3];
    apparel_condition(&mut short, &mut seed);
    assert_eq!(seed, 1);
}

/// Options 1 to 3 ask all kinds, apparel and arms; option 1 of the ranks
/// asks any. The dialog reopens on the matching options.
#[test]
fn the_merchant_specialises() {
    assert_eq!(category_for_option(1), Some(0));
    assert_eq!(category_for_option(3), Some(ARMS));
    assert_eq!(category_for_option(4), None);
    assert_eq!(rank_for_option(1), -1);
    assert_eq!(rank_for_option(7), 5);
    assert_eq!(options_for(APPAREL, 3, 1), (2, 5));
    assert_eq!(options_for(WEAPON, -1, 3), (3, 1));
    assert_eq!(RANKS[(rank_for_option(4) + 1) as usize], "Novice");
}

/// 1 gold 3 silver 5 copper is 185 copper. Paying 100 leaves 85, 7 silver
/// 1 copper; paying more than held empties the purse.
#[test]
fn a_transaction_moves_coins_and_items() {
    let purse = 5 | 3 << 8 | 1 << 16;
    assert_eq!(coins_value(purse), 185);
    assert_eq!(add_coins(purse, -100), 1 | 7 << 8);
    assert_eq!(add_coins(purse, -185), 0);
    let mut money = purse;
    let mut tallies = [0, 12];
    let mut stock = vec![1, 2, 3, 2];
    let trade = Transaction {
        due: 100,
        tallies: [100, 0],
        sold: vec![9, 0x8000_0004],
        bought: vec![2, 7],
    };
    let dups = settle(&mut money, &mut tallies, &mut stock, &trade);
    assert_eq!(money, 1 | 7 << 8);
    assert_eq!(tallies, [4 | 8 << 8, 1 << 8]);
    assert_eq!(stock, [1, 3, 2, 9, 0x8000_0004]);
    assert!(dups.is_empty());
    // Nothing due leaves the purse as it is.
    let mut kept = purse;
    settle(
        &mut kept,
        &mut [0, 0],
        &mut Vec::new(),
        &Transaction::default(),
    );
    assert_eq!(kept, purse);
}
