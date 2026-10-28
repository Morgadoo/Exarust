//! The arena's random outfits: the pool picks that dress a generated
//! character or a hired candidate.
//!
//! The rules are the original's item-pool machinery: rank windows, the
//! weight a low-ranked entry keeps, the masked bonus check, the weighted
//! pick from a stepped roll, and the two-pass pick that refuses a piece
//! already worn. The same roll also drives the random look of a generated
//! NPC, so the random-number generator, the smooth unit curve and the
//! candidate pool live together here.
//!
//! Every table is supplied by the caller. Nothing in this module knows the
//! retail item ids, item names or the profession weights; those are data,
//! and the caller passes them in. All lookups are total: missing items make
//! a pick skip rather than panic.
//!
//! Evidence: the arena's outfit generation routines, read from
//! `Exanima.exe` 0.9.5 (see the arena research notes).

use crate::arena_outfit_data as data;
use exa_formats::objdb::Item;
use std::collections::BTreeMap;

/// Number of bonus words and bonus floats an item carries.
pub const BONUS_SLOTS: usize = 20;

/// The pool refuses to grow past this many candidates. The original reports
/// an error when the count would pass it; here extra additions are ignored.
pub const MAX_CANDIDATES: usize = 128;

/// Experience needed for one profession rank.
pub const EXPERIENCE_PER_RANK: u32 = 5_000;

/// The profession grid is three rows of eight columns.
pub const PROFESSION_ROWS: usize = 4;
/// Columns in a profession row.
pub const PROFESSION_COLUMNS: usize = 8;
/// A row records up to this many rank gains (the experience cap divided by
/// the experience per rank).
pub const PROFESSION_TRACKS: usize = 12;

/// The stepped linear congruential sequence the whole arena uses.
const LCG_MULTIPLIER: u32 = 0x0808_8405;

/// The low half of an item's rank word holds its bulk in points.
const BULK_MASK: u16 = 0x1f;

/// The double-precision epsilon the look generator uses around zero.
const LOOK_EPSILON: f32 = 1.192_089_982_282_596e-7;

/// A deterministic linear congruential sequence.
///
/// Each draw steps the state first, then takes the high half of the word
/// times a range. Exposing the state separately lets callers reproduce the
/// original's order, including draws that are consumed but unused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lcg {
    state: u32,
}

impl Lcg {
    /// Start the sequence from a seed.
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// The current word, without stepping.
    #[must_use]
    pub const fn state(self) -> u32 {
        self.state
    }

    /// Step once: the state becomes `state * multiplier + 1`.
    pub fn step(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(1);
        self.state
    }

    /// The high half of `state * range`, without stepping.
    #[must_use]
    pub const fn high(self, range: u32) -> u32 {
        ((self.state as u64 * range as u64) >> 32) as u32
    }

    /// A unit draw: step, then the new word over 2^32, in double and
    /// narrowed once, as the engine's float randomiser does. It shares this
    /// sequence with the high-word draws, so callers must not mix another
    /// generator in.
    pub fn unit(&mut self) -> f32 {
        self.step();
        (f64::from(self.state) * f64::from_bits(0x3df0_0000_0000_0000)) as f32
    }
}

impl Default for Lcg {
    fn default() -> Self {
        Self::new(0)
    }
}

/// An item id. Zero means "no item" throughout the pool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub u32);

/// The source tag carried by a candidate from the pool.
///
/// The outfit generator moves through several dressing modes and remembers
/// which one added each candidate, so the chosen piece can be read back in
/// the mode the generator was in when it was offered. The values are opaque
/// to this module.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenerationMode(pub u32);

/// The twenty bonus words and twenty bonus floats of one item.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bonus {
    /// Bit sets shared with worn pieces are refused.
    pub words: [u64; BONUS_SLOTS],
    /// Per-slot stat additions.
    pub floats: [f32; BONUS_SLOTS],
}

impl Default for Bonus {
    fn default() -> Self {
        Self {
            words: [0; BONUS_SLOTS],
            floats: [0.0; BONUS_SLOTS],
        }
    }
}

/// The merged bonus of everything worn so far.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WornBonus {
    /// Union of every worn piece's words.
    pub words: [u64; BONUS_SLOTS],
    /// Sum of every worn piece's floats.
    pub floats: [f32; BONUS_SLOTS],
}

impl WornBonus {
    /// True when any worn word shares a bit with the candidate's words.
    #[must_use]
    pub fn overlaps(&self, words: &[u64; BONUS_SLOTS]) -> bool {
        self.words
            .iter()
            .zip(words)
            .any(|(worn, candidate)| worn & candidate != 0)
    }

    /// Merge a piece into this set: words are unioned, floats added.
    pub fn merge(&mut self, bonus: &Bonus) {
        for ((worn, word), (float, add)) in self
            .words
            .iter_mut()
            .zip(bonus.words.iter())
            .zip(self.floats.iter_mut().zip(bonus.floats.iter()))
        {
            *worn |= word;
            *float += add;
        }
    }
}

/// The facts this module needs about one item.
///
/// The original reads these from its item database. The caller supplies them
/// as plain values so the pool stays a pure decision engine.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemFacts {
    /// The item's id.
    pub id: ItemId,
    /// The low byte's top three bits are the rank; the low five bits are the
    /// bulk in points.
    pub rank_word: u16,
    /// The item kind byte used to classify weapons.
    pub weapon_kind: u8,
    /// The apparel class: the top three bits of the byte at the start of the
    /// item's apparel data (the lookup uses the apparel mask).
    pub apparel_class: u32,
    /// The apparel's point value: the low five bits of the word at the start
    /// of the item's apparel data.
    pub apparel_points: i32,
    /// Low material byte used to scale weapon bonuses.
    pub material_lo: u8,
    /// High material byte used to scale weapon bonuses.
    pub material_hi: u8,
    /// The quality byte the item is equipped with.
    pub quality: u8,
    /// The condition byte the item is equipped with.
    pub condition: u8,
    /// The item's bonus words.
    pub bonus_words: [u64; BONUS_SLOTS],
    /// The item's bonus floats.
    pub bonus_floats: [f32; BONUS_SLOTS],
    /// Raw unit counts behind the bonus floats, used when filling a weapon
    /// table row.
    pub bonus_units: [u32; BONUS_SLOTS],
}

impl ItemFacts {
    /// The item's bulk in points, taken from the low five bits of its rank
    /// word.
    #[must_use]
    pub const fn bulk(&self) -> i16 {
        (self.rank_word & BULK_MASK) as i16
    }

    /// The facts the object database gives one item, as the original's
    /// first-use pass reads them from the item object.
    ///
    /// The rank word and the twenty bonus words are the body's rank and
    /// occupancy table; a weapon without a body takes its weapon section's
    /// rank. Quality and condition are the appearance bytes for apparel and
    /// the weapon's own for a weapon. The bonus floats and units are not
    /// item fields in the original (they come from the arena's table rows),
    /// so they stay zero here; the item's material bytes are also not
    /// exposed by the parser yet.
    #[must_use]
    pub fn of_item(item: &Item) -> Self {
        let quality = item
            .appearance
            .map(|appearance| appearance.quality)
            .or_else(|| item.weapon.map(|weapon| weapon.quality))
            .unwrap_or(0);
        let condition = item
            .appearance
            .map(|appearance| appearance.damage)
            .or_else(|| item.weapon.map(|weapon| weapon.damage))
            .unwrap_or(0);
        Self {
            id: ItemId(item.id),
            rank_word: item.rank_word,
            weapon_kind: 0,
            apparel_class: u32::from(item.armour_type),
            apparel_points: i32::from(item.rank_word & BULK_MASK),
            material_lo: 0,
            material_hi: 0,
            quality,
            condition,
            bonus_words: item.occupancy,
            bonus_floats: [0.0; BONUS_SLOTS],
            bonus_units: [0; BONUS_SLOTS],
        }
    }
}

/// One row of an item pool table.
///
/// The simple tables use only `id`, `weight`, `rank` and `cost`; masked
/// tables also use the two masks and the bonus.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PoolEntry {
    /// The item id; zero means "nothing".
    pub id: ItemId,
    /// The row's weight before any rank-window or score adjustment.
    pub weight: i32,
    /// The wanted-mask bits; the query's wanted mask must share one.
    pub wanted: u32,
    /// The forbidden-mask bits; the character's bonus mask must share none.
    pub forbidden: u32,
    /// The row's rank.
    pub rank: i32,
    /// The points the entry costs when it is equipped.
    pub cost: i32,
    /// The bonus merged into the outfit by this row.
    pub bonus: Bonus,
}

/// The masks and rank window a pool query applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaskQuery {
    /// The character's bonus mask. It is zero or one in the original: one
    /// when the character carries the flag the entry forbids.
    pub character_bonus: u32,
    /// The wanted mask; an entry must share at least one bit.
    pub wanted: u32,
    /// Entries above this rank are refused.
    pub rank_limit: i32,
    /// When set, only entries at the highest eligible rank are considered.
    pub max_rank: bool,
}

/// A ranked table pick: the chosen id and the points it costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RankPick {
    /// The chosen id.
    pub id: ItemId,
    /// The chosen row's cost, to be subtracted from the character's points.
    pub cost: i32,
}

/// A candidate held in an [`OutfitPool`].
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// The item id.
    pub id: ItemId,
    /// The current weight. The pool replaces it with a fit score when it is
    /// pruned.
    pub weight: i32,
    /// The points the candidate costs.
    pub cost: i32,
    /// The bonus to merge when the candidate is equipped.
    pub bonus: Bonus,
    /// The dressing mode that offered the candidate.
    pub source: GenerationMode,
}

/// A candidate that was drawn from the pool.
#[derive(Clone, Debug, PartialEq)]
pub struct CandidatePick {
    /// The item id.
    pub id: ItemId,
    /// The weight the draw used.
    pub weight: i32,
    /// The points the pick costs.
    pub cost: i32,
    /// The bonus to merge into the outfit.
    pub bonus: Bonus,
    /// The mode the candidate was added in.
    pub source: GenerationMode,
}

/// The item held in a hand slot after a weapon pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldWeapon {
    /// The weapon's id.
    pub id: ItemId,
    /// The variant the randomiser chose.
    pub variant: u32,
}

/// One row of a weapon table.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WeaponRow {
    /// The weapon id; zero rows are ignored.
    pub id: ItemId,
    /// The selection weight. Zero becomes ten, the original's default.
    pub weight: i32,
    /// The weapon class, filled from the item kind when still zero.
    pub class: u32,
    /// The rank, from the top bits of the item's rank word.
    pub rank: u32,
    /// The points, from the low five bits of the item's rank word.
    pub points: i32,
    /// Bonus floats derived from the item's unit counts.
    pub bonus_floats: [f32; BONUS_SLOTS],
}

/// One row of a shield table.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShieldRow {
    /// The shield id; zero rows are ignored.
    pub id: ItemId,
    /// The selection weight. Zero becomes ten.
    pub weight: i32,
    /// The shield class, from the top bits of the shield's rank byte when
    /// still zero.
    pub class: u32,
    /// The shield rank, from the low five bits of the rank byte.
    pub rank: i32,
}

/// One row of an apparel table.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ApparelRow {
    /// The apparel id; zero rows are ignored.
    pub id: ItemId,
    /// The selection weight. Zero becomes ten.
    pub weight: i32,
    /// The apparel class, filled from the item when still zero.
    pub class: u32,
    /// The points, filled from the item.
    pub points: i32,
}

/// A weighted outfit candidate pool.
///
/// Candidates are added by the masked, ranked or unranked passes, then
/// pruned and scored against what the character already wears, then drawn
/// with one stepped roll.
#[derive(Clone, Debug, Default)]
pub struct OutfitPool {
    candidates: Vec<Candidate>,
}

impl OutfitPool {
    /// An empty pool.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            candidates: Vec::new(),
        }
    }

    /// Forget every candidate.
    pub fn clear(&mut self) {
        self.candidates.clear();
    }

    /// The candidates currently held.
    #[must_use]
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    /// The candidates currently held, mutably. The outfit sequence halves
    /// every weight after its first weapon pass.
    fn candidates_mut(&mut self) -> &mut [Candidate] {
        &mut self.candidates
    }

    /// How many candidates are held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    /// True when the pool holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    fn push(
        &mut self,
        id: ItemId,
        weight: i32,
        cost: i32,
        bonus: Bonus,
        source: GenerationMode,
    ) -> bool {
        if self.candidates.len() >= MAX_CANDIDATES {
            return false;
        }
        self.candidates.push(Candidate {
            id,
            weight,
            cost,
            bonus,
            source,
        });
        true
    }

    /// Add every table entry the masks and rank window allow.
    ///
    /// An entry must name an item, cost no more than the remaining points,
    /// share no forbidden bit with the character, share a wanted bit and sit
    /// at or below the rank limit. In max-rank mode the highest eligible rank
    /// is found first and only entries at that rank are added. The entry's
    /// bonus and the given mode are recorded.
    ///
    /// Returns how many candidates were added.
    pub fn add_masked(
        &mut self,
        entries: &[PoolEntry],
        query: MaskQuery,
        points: i32,
        source: GenerationMode,
    ) -> usize {
        let accepts = |entry: &PoolEntry| {
            entry.id.0 != 0
                && entry.cost <= points
                && query.character_bonus & entry.forbidden == 0
                && query.wanted & entry.wanted != 0
                && entry.rank <= query.rank_limit
        };
        let highest = if query.max_rank {
            entries
                .iter()
                .filter(|entry| accepts(entry))
                .map(|entry| entry.rank)
                .max()
                .unwrap_or(0)
        } else {
            i32::MIN
        };
        let mut added = 0;
        for entry in entries {
            if !accepts(entry) {
                continue;
            }
            if query.max_rank && entry.rank != highest {
                continue;
            }
            if self.push(entry.id, entry.weight, entry.cost, entry.bonus, source) {
                added += 1;
            }
        }
        added
    }

    /// Add entries whose rank is in the two-rank window ending at the limit.
    ///
    /// An entry must name an item, cost no more than the remaining points,
    /// and sit at the limit or one below it. The one below the limit keeps
    /// half its weight, truncated; the entry at the limit keeps it whole.
    ///
    /// Returns how many candidates were added.
    pub fn add_ranked(
        &mut self,
        entries: &[PoolEntry],
        rank_limit: i32,
        points: i32,
        source: GenerationMode,
    ) -> usize {
        let Some(low) = rank_limit.checked_sub(1) else {
            return 0;
        };
        let mut added = 0;
        for entry in entries {
            if entry.id.0 == 0 || entry.cost > points {
                continue;
            }
            if entry.rank != low && entry.rank != rank_limit {
                continue;
            }
            let weight = if entry.rank == low {
                entry.weight / 2
            } else {
                entry.weight
            };
            if self.push(entry.id, weight, entry.cost, entry.bonus, source) {
                added += 1;
            }
        }
        added
    }

    /// Add every entry that names an item and fits the remaining points,
    /// with no rank or mask check.
    ///
    /// Returns how many candidates were added.
    pub fn add_unranked(
        &mut self,
        entries: &[PoolEntry],
        points: i32,
        source: GenerationMode,
    ) -> usize {
        let mut added = 0;
        for entry in entries {
            if entry.id.0 == 0 || entry.cost > points {
                continue;
            }
            if self.push(entry.id, entry.weight, entry.cost, entry.bonus, source) {
                added += 1;
            }
        }
        added
    }

    /// Drop candidates that cannot be worn and score the rest.
    ///
    /// A candidate is dropped when its item is unknown, when its bulk is more
    /// than the remaining points, or when any of its bonus words overlaps the
    /// worn words. The rest get a fit score: for each of the twenty slots,
    /// the worn float plus the candidate float over the worn float, summed in
    /// single precision and rounded to a percentage.
    ///
    /// Returns how many candidates remain.
    pub fn prune_and_score<F>(&mut self, points: i32, worn: &WornBonus, mut lookup: F) -> usize
    where
        F: FnMut(ItemId) -> Option<ItemFacts>,
    {
        let mut index = 0;
        while index < self.candidates.len() {
            let candidate = &self.candidates[index];
            let Some(facts) = lookup(candidate.id) else {
                self.candidates.swap_remove(index);
                continue;
            };
            if points < i32::from(facts.bulk()) || worn.overlaps(&facts.bonus_words) {
                self.candidates.swap_remove(index);
                continue;
            }
            let score = score_candidate(worn, &candidate.bonus);
            self.candidates[index].weight = score;
            index += 1;
        }
        self.candidates.len()
    }

    /// Prune, score, then draw one candidate by weight and equip it.
    ///
    /// The draw steps the roll once and walks the candidates' cumulative
    /// weights until the threshold is passed. The chosen candidate's source
    /// tag is reported back and the equip callback receives the pick.
    pub fn pick_and_equip<F, E>(
        &mut self,
        lcg: &mut Lcg,
        points: i32,
        worn: &WornBonus,
        lookup: F,
        mut equip: E,
    ) -> Option<CandidatePick>
    where
        F: FnMut(ItemId) -> Option<ItemFacts>,
        E: FnMut(&CandidatePick),
    {
        self.prune_and_score(points, worn, lookup);
        let weights: Vec<i32> = self.candidates.iter().map(|c| c.weight).collect();
        let index = weighted_pick(lcg, &weights)?;
        let candidate = &self.candidates[index];
        let pick = CandidatePick {
            id: candidate.id,
            weight: candidate.weight,
            cost: candidate.cost,
            bonus: candidate.bonus,
            source: candidate.source,
        };
        equip(&pick);
        Some(pick)
    }

    /// Draw one candidate by weight into a hand slot.
    ///
    /// The chosen id is passed to the variant randomiser, the result is
    /// stored in the hand, and the candidate's cost is subtracted from the
    /// remaining points. Nothing happens when the pool has no positive
    /// weight.
    pub fn pick_weapon<V>(
        &self,
        lcg: &mut Lcg,
        hand: &mut Option<HeldWeapon>,
        points: &mut i32,
        mut variant: V,
    ) -> Option<ItemId>
    where
        V: FnMut(ItemId) -> u32,
    {
        let weights: Vec<i32> = self.candidates.iter().map(|c| c.weight).collect();
        let index = weighted_pick(lcg, &weights)?;
        let candidate = &self.candidates[index];
        let chosen_variant = variant(candidate.id);
        *hand = Some(HeldWeapon {
            id: candidate.id,
            variant: chosen_variant,
        });
        *points = points.wrapping_sub(candidate.cost);
        Some(candidate.id)
    }
}

/// The fixed item the profile sequence equips directly when the rank is five
/// or when max-rank mode reaches above rank one. The slot is a guess: the
/// original reads the id out of a direct table row rather than a named slot.
pub const ITEM_402: ItemId = ItemId(402);
/// The second direct fixed item (see [`ITEM_402`]).
pub const ITEM_1359: ItemId = ItemId(1359);
/// The third direct fixed item used by the profile sequence.
pub const ITEM_251: ItemId = ItemId(251);
/// The third direct fixed item used by the numeric-level sequence. It stands
/// in the same slot as [`ITEM_251`] in the profile sequence.
pub const ITEM_285: ItemId = ItemId(285);

/// First direct fixed item of the level-five premade suit.
pub const ITEM_439: ItemId = ItemId(439);
/// Second direct fixed item of the level-five premade suit.
pub const ITEM_436: ItemId = ItemId(436);
/// Third direct fixed item of the level-five premade suit.
pub const ITEM_85: ItemId = ItemId(85);
/// Fourth direct fixed item of the level-five premade suit.
pub const ITEM_435: ItemId = ItemId(435);

/// The direct fixed ids, so a table build can cache their facts too.
const FIXED_ITEMS: [ItemId; 8] = [
    ITEM_402, ITEM_1359, ITEM_251, ITEM_285, ITEM_439, ITEM_436, ITEM_85, ITEM_435,
];

/// The ten-qword float preset copied by most early branches.
///
/// The original copies the twenty floats into the worn-float block the fit
/// score starts from. They are also reported as the fixed preset values, in
/// order, because the original gives the fields no names here.
pub const LOOK_PRESET_ROUGH: [f32; 20] = [
    1.5, 2.0, 4.0, 3.0, 2.0, 4.0, 3.0, 1.0, 1.0, 1.0, 8.0, 8.0, 4.0, 8.0, 8.0, 4.0, 4.0, 4.0, 1.0,
    1.0,
];

/// The ten-qword look preset copied by the shield branch.
pub const LOOK_PRESET_ARMED: [f32; 20] = [
    1.5, 2.0, 1.0, 2.0, 2.0, 1.0, 2.0, 1.0, 1.0, 1.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 1.0,
    1.0,
];

/// The bit the premade-skill probe looks for in the character's skill word.
///
/// The original scans the character's skill records for technique byte eight
/// and enough points. The caller supplies the already-reduced low skill word;
/// this module treats bit three of that word as "the premade skill is present".
pub const PREMADE_SKILL_BIT: u16 = 0x0008;

/// The character state the two outfit sequences read.
///
/// The original reaches into the character record for these fields. They are
/// separated here so the generators stay pure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutfitCharacter {
    /// The low skill word. Bit three stands in for the original's premade
    /// skill probe.
    pub skill_word: u16,
    /// The character's experience word at record offset `0xd8`.
    pub experience: u32,
    /// The character's warmth at record offset `0x90`; it is blended into the
    /// skin look value when positive.
    pub warmth: f32,
    /// The record flags word at offset `0x54`. Its low bit, when not one,
    /// gives the character bonus mask one.
    pub record_flags: u32,
}

impl Default for OutfitCharacter {
    fn default() -> Self {
        Self {
            skill_word: 0,
            experience: 0,
            warmth: 0.0,
            record_flags: 1,
        }
    }
}

impl OutfitCharacter {
    /// The bonus mask: one unless the record's low flag bit is one.
    #[must_use]
    pub const fn bonus_mask(self) -> u32 {
        (self.record_flags & 1 != 1) as u32
    }

    /// Whether the premade-skill probe succeeds.
    #[must_use]
    pub const fn has_premade_skill(self) -> bool {
        self.skill_word & PREMADE_SKILL_BIT != 0
    }
}

/// One item the outfit sequence equipped.
#[derive(Clone, Debug, PartialEq)]
pub struct EquippedItem {
    /// The item id.
    pub id: ItemId,
    /// The points taken off the character's remaining points.
    pub cost: i32,
    /// The bonus merged into the worn set. Weapons and shields carry none.
    pub bonus: Bonus,
    /// The dressing mode in force when the item was equipped.
    pub source: GenerationMode,
    /// The item's quality byte.
    pub quality: u8,
    /// The item's condition byte.
    pub condition: u8,
}

/// The look values the outfit routine computes or copies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutfitLook {
    /// The height-like value, clamped to one.
    pub height: f32,
    /// The skin-like value.
    pub skin: f32,
    /// The condition seed.
    pub condition_seed: f32,
    /// The twenty base floats copied from one of the fixed presets. They seed
    /// the worn bonus floats before the apparel picks add to them.
    pub preset: [f32; 20],
}

impl Default for OutfitLook {
    fn default() -> Self {
        Self {
            height: 0.0,
            skin: 0.0,
            condition_seed: 0.0,
            preset: [0.0; 20],
        }
    }
}

/// A generated outfit.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedOutfit {
    /// Hand zero (record offset 0x64) and hand one (offset 0x6c).
    pub hands: [Option<HeldWeapon>; 2],
    /// Every piece the sequence equipped, in call order.
    pub items: Vec<EquippedItem>,
    /// The points left after the sequence.
    pub points: i32,
    /// The merged bonus of every apparel piece.
    pub worn: WornBonus,
    /// The final generation mode byte.
    pub mode: u8,
    /// The quality byte of the last apparel piece.
    pub quality: u8,
    /// The condition byte of the last apparel piece.
    pub condition: u8,
    /// The computed and fixed look values.
    pub look: OutfitLook,
}

/// The outfit item tables, converted from the raw arena data with an
/// item-facts pass.
///
/// Large tables carry rank, cost and bonus from [`ItemFacts`]. Small rank
/// tables carry the same three fields; their raw weight is kept unchanged.
/// A zero weight in a large table becomes the original's default of ten.
#[derive(Clone, Debug, Default)]
pub struct OutfitTables {
    /// Alt A pool.
    pub alt_a: Vec<PoolEntry>,
    /// Alt B pool.
    pub alt_b: Vec<PoolEntry>,
    /// Alt C pool.
    pub alt_c: Vec<PoolEntry>,
    /// Alt D pool.
    pub alt_d: Vec<PoolEntry>,
    /// Armour A pool.
    pub armour_a: Vec<PoolEntry>,
    /// Armour B pool.
    pub armour_b: Vec<PoolEntry>,
    /// Belt pool.
    pub belt: Vec<PoolEntry>,
    /// Body pool.
    pub body_pool: Vec<PoolEntry>,
    /// Cloak pool.
    pub cloak: Vec<PoolEntry>,
    /// Coat pool.
    pub coat: Vec<PoolEntry>,
    /// Feet pool.
    pub feet: Vec<PoolEntry>,
    /// Full-set pool.
    pub full_set: Vec<PoolEntry>,
    /// Hands pool.
    pub hands: Vec<PoolEntry>,
    /// Head pool.
    pub head: Vec<PoolEntry>,
    /// Hood pool.
    pub hood: Vec<PoolEntry>,
    /// Jewel pool.
    pub jewel: Vec<PoolEntry>,
    /// Main A pool.
    pub main_a: Vec<PoolEntry>,
    /// Main B pool.
    pub main_b: Vec<PoolEntry>,
    /// Main C pool.
    pub main_c: Vec<PoolEntry>,
    /// Neck pool.
    pub neck: Vec<PoolEntry>,
    /// Ranked armour small table.
    pub ranked_armour: Vec<PoolEntry>,
    /// Shield small table.
    pub shields: Vec<PoolEntry>,
    /// Torso A pool.
    pub torso_a: Vec<PoolEntry>,
    /// Torso B pool.
    pub torso_b: Vec<PoolEntry>,
    /// Torso C pool.
    pub torso_c: Vec<PoolEntry>,
    /// Torso D pool.
    pub torso_d: Vec<PoolEntry>,
    /// Torso E pool.
    pub torso_e: Vec<PoolEntry>,
    /// Torso F pool.
    pub torso_f: Vec<PoolEntry>,
    /// Torso G pool.
    pub torso_g: Vec<PoolEntry>,
    /// Torso H pool.
    pub torso_h: Vec<PoolEntry>,
    /// Torso I pool.
    pub torso_i: Vec<PoolEntry>,
    /// Underwear pool.
    pub underwear: Vec<PoolEntry>,
    /// Weapon A small table.
    pub weapons_a: Vec<PoolEntry>,
    /// Weapon B small table.
    pub weapons_b: Vec<PoolEntry>,
    /// Weapon C small table.
    pub weapons_c: Vec<PoolEntry>,
    /// Weapon D small table.
    pub weapons_d: Vec<PoolEntry>,
    facts: BTreeMap<u32, ItemFacts>,
}

impl OutfitTables {
    /// Build every table and cache the facts the generators need.
    ///
    /// Missing items still produce a row, with rank, cost and bonus zero, so
    /// the table lengths always match the shipped data.
    #[must_use]
    pub fn build<F>(mut lookup: F) -> Self
    where
        F: FnMut(ItemId) -> Option<ItemFacts>,
    {
        let mut facts = BTreeMap::new();
        for id in FIXED_ITEMS {
            if let Some(item) = lookup(id) {
                facts.insert(id.0, item);
            }
        }
        let alt_a = pool_entries(&data::ALT_A, &mut lookup, &mut facts);
        let alt_b = pool_entries(&data::ALT_B, &mut lookup, &mut facts);
        let alt_c = pool_entries(&data::ALT_C, &mut lookup, &mut facts);
        let alt_d = pool_entries(&data::ALT_D, &mut lookup, &mut facts);
        let armour_a = pool_entries(&data::ARMOUR_A, &mut lookup, &mut facts);
        let armour_b = pool_entries(&data::ARMOUR_B, &mut lookup, &mut facts);
        let belt = pool_entries(&data::BELT, &mut lookup, &mut facts);
        let body_pool = pool_entries(&data::BODY_POOL, &mut lookup, &mut facts);
        let cloak = pool_entries(&data::CLOAK, &mut lookup, &mut facts);
        let coat = pool_entries(&data::COAT, &mut lookup, &mut facts);
        let feet = pool_entries(&data::FEET, &mut lookup, &mut facts);
        let full_set = pool_entries(&data::FULL_SET, &mut lookup, &mut facts);
        let hands = pool_entries(&data::HANDS, &mut lookup, &mut facts);
        let head = pool_entries(&data::HEAD, &mut lookup, &mut facts);
        let hood = pool_entries(&data::HOOD, &mut lookup, &mut facts);
        let jewel = pool_entries(&data::JEWEL, &mut lookup, &mut facts);
        let main_a = pool_entries(&data::MAIN_A, &mut lookup, &mut facts);
        let main_b = pool_entries(&data::MAIN_B, &mut lookup, &mut facts);
        let main_c = pool_entries(&data::MAIN_C, &mut lookup, &mut facts);
        let neck = pool_entries(&data::NECK, &mut lookup, &mut facts);
        let torso_a = pool_entries(&data::TORSO_A, &mut lookup, &mut facts);
        let torso_b = pool_entries(&data::TORSO_B, &mut lookup, &mut facts);
        let torso_c = pool_entries(&data::TORSO_C, &mut lookup, &mut facts);
        let torso_d = pool_entries(&data::TORSO_D, &mut lookup, &mut facts);
        let torso_e = pool_entries(&data::TORSO_E, &mut lookup, &mut facts);
        let torso_f = pool_entries(&data::TORSO_F, &mut lookup, &mut facts);
        let torso_g = pool_entries(&data::TORSO_G, &mut lookup, &mut facts);
        let torso_h = pool_entries(&data::TORSO_H, &mut lookup, &mut facts);
        let torso_i = pool_entries(&data::TORSO_I, &mut lookup, &mut facts);
        let underwear = pool_entries(&data::UNDERWEAR, &mut lookup, &mut facts);
        let ranked_armour = rank_entries(&data::RANKED_ARMOUR, &mut lookup, &mut facts);
        let shields = rank_entries(&data::SHIELDS, &mut lookup, &mut facts);
        let weapons_a = rank_entries(&data::WEAPONS_A, &mut lookup, &mut facts);
        let weapons_b = rank_entries(&data::WEAPONS_B, &mut lookup, &mut facts);
        let weapons_c = rank_entries(&data::WEAPONS_C, &mut lookup, &mut facts);
        let weapons_d = rank_entries(&data::WEAPONS_D, &mut lookup, &mut facts);
        Self {
            alt_a,
            alt_b,
            alt_c,
            alt_d,
            armour_a,
            armour_b,
            belt,
            body_pool,
            cloak,
            coat,
            feet,
            full_set,
            hands,
            head,
            hood,
            jewel,
            main_a,
            main_b,
            main_c,
            neck,
            ranked_armour,
            shields,
            torso_a,
            torso_b,
            torso_c,
            torso_d,
            torso_e,
            torso_f,
            torso_g,
            torso_h,
            torso_i,
            underwear,
            weapons_a,
            weapons_b,
            weapons_c,
            weapons_d,
            facts,
        }
    }

    /// The facts cached for an item, if the build lookup knew it.
    #[must_use]
    pub fn facts(&self, id: ItemId) -> Option<ItemFacts> {
        self.facts.get(&id.0).cloned()
    }

    /// Cached facts exposed as the lookup the pool methods take.
    pub fn lookup(&self) -> impl FnMut(ItemId) -> Option<ItemFacts> + '_ {
        |id| self.facts(id)
    }
}

/// Convert one large raw pool row.
fn convert_pool_entry(row: &data::RawPoolEntry, facts: Option<ItemFacts>) -> PoolEntry {
    let id = ItemId(row.id);
    let (rank, cost, bonus) = match facts {
        Some(facts) => (
            i32::from((facts.rank_word as u8) >> 5),
            i32::from(facts.rank_word & BULK_MASK),
            Bonus {
                words: facts.bonus_words,
                floats: facts.bonus_floats,
            },
        ),
        None => (0, 0, Bonus::default()),
    };
    PoolEntry {
        id,
        weight: if row.weight == 0 { 10 } else { row.weight },
        wanted: row.wanted,
        forbidden: row.forbidden,
        rank,
        cost,
        bonus,
    }
}

/// Convert a large raw pool table, recording item facts.
fn pool_entries<F>(
    raw: &[data::RawPoolEntry],
    lookup: &mut F,
    cache: &mut BTreeMap<u32, ItemFacts>,
) -> Vec<PoolEntry>
where
    F: FnMut(ItemId) -> Option<ItemFacts>,
{
    raw.iter()
        .map(|row| {
            let id = ItemId(row.id);
            let item = lookup(id);
            if let Some(facts) = &item {
                cache.insert(row.id, facts.clone());
            }
            convert_pool_entry(row, item)
        })
        .collect()
}

/// Convert a small rank table, keeping its raw weight.
fn rank_entries<F>(
    raw: &[data::RawRankEntry],
    lookup: &mut F,
    cache: &mut BTreeMap<u32, ItemFacts>,
) -> Vec<PoolEntry>
where
    F: FnMut(ItemId) -> Option<ItemFacts>,
{
    raw.iter()
        .map(|row| {
            let id = ItemId(row.id);
            let item = lookup(id);
            if let Some(facts) = &item {
                cache.insert(row.id, facts.clone());
            }
            let (rank, cost, bonus) = match item {
                Some(facts) => (
                    i32::from((facts.rank_word as u8) >> 5),
                    i32::from(facts.rank_word & BULK_MASK),
                    Bonus {
                        words: facts.bonus_words,
                        floats: facts.bonus_floats,
                    },
                ),
                None => (0, 0, Bonus::default()),
            };
            PoolEntry {
                id,
                weight: row.weight,
                wanted: 0,
                forbidden: 0,
                rank,
                cost,
                bonus,
            }
        })
        .collect()
}

/// Mutable state while one outfit is dressed.
struct Dresser<'a> {
    lcg: &'a mut Lcg,
    points: i32,
    worn: WornBonus,
    mode: u8,
    items: Vec<EquippedItem>,
    hands: [Option<HeldWeapon>; 2],
    quality: u8,
    condition: u8,
    condition_value: f32,
    rank_limit: i32,
    character_bonus: u32,
    pool: OutfitPool,
}

impl<'a> Dresser<'a> {
    fn new(lcg: &'a mut Lcg, rank_limit: i32, character_bonus: u32) -> Self {
        let rank = usize::try_from(rank_limit.clamp(0, 5)).unwrap_or(0);
        const POINTS_BY_RANK: [i32; 6] = [5, 10, 15, 20, 30, 40];
        Self {
            lcg,
            points: POINTS_BY_RANK[rank] << 1,
            worn: WornBonus::default(),
            mode: 0,
            items: Vec::new(),
            hands: [None, None],
            quality: 0,
            condition: 0,
            condition_value: 0.0,
            rank_limit,
            character_bonus,
            pool: OutfitPool::new(),
        }
    }

    fn add_masked(&mut self, entries: &[PoolEntry], wanted: u32, max_rank: bool) -> usize {
        let query = MaskQuery {
            character_bonus: self.character_bonus,
            wanted,
            rank_limit: self.rank_limit,
            max_rank,
        };
        self.pool.add_masked(
            entries,
            query,
            self.points,
            GenerationMode(u32::from(self.mode)),
        )
    }

    fn add_ranked(&mut self, entries: &[PoolEntry]) -> usize {
        self.pool.add_ranked(
            entries,
            self.rank_limit,
            self.points,
            GenerationMode(u32::from(self.mode)),
        )
    }

    fn add_unranked(&mut self, entries: &[PoolEntry]) -> usize {
        self.pool
            .add_unranked(entries, self.points, GenerationMode(u32::from(self.mode)))
    }

    fn halve_candidates(&mut self) {
        for candidate in self.pool.candidates_mut() {
            // A logical shift, as the original shifts the weight as unsigned.
            candidate.weight = ((candidate.weight as u32) >> 1) as i32;
        }
    }

    fn equip_apparel_id(&mut self, tables: &OutfitTables, id: ItemId, source: GenerationMode) {
        let Some(facts) = tables.facts(id) else {
            return;
        };
        let before = self.points;
        let condition = (f64::from(self.condition_value) * 255.0).round_ties_even() as u8;
        let fit = equip_apparel(&self.worn, self.points, &facts, |_| condition);
        self.points = fit.points;
        self.worn = fit.bonus;
        self.quality = fit.quality;
        self.condition = fit.condition;
        self.items.push(EquippedItem {
            id,
            cost: before.wrapping_sub(fit.points),
            bonus: Bonus {
                words: facts.bonus_words,
                floats: facts.bonus_floats,
            },
            source,
            quality: fit.quality,
            condition: fit.condition,
        });
    }

    fn pick_and_equip(&mut self, tables: &OutfitTables) -> Option<CandidatePick> {
        let pick = self.pool.pick_and_equip(
            self.lcg,
            self.points,
            &self.worn,
            |id| tables.facts(id),
            |_| {},
        );
        if let Some(pick) = &pick {
            self.equip_apparel_id(tables, pick.id, pick.source);
        }
        pick
    }

    fn pick_masked(
        &mut self,
        tables: &OutfitTables,
        entries: &[PoolEntry],
        wanted: u32,
        max_rank: bool,
    ) -> Option<ItemId> {
        let query = MaskQuery {
            character_bonus: self.character_bonus,
            wanted,
            rank_limit: self.rank_limit,
            max_rank,
        };
        let mut picked = None;
        let result = pick_masked(self.lcg, entries, query, |id, _| picked = Some(id));
        if let Some(id) = picked {
            let source = GenerationMode(u32::from(self.mode));
            self.equip_apparel_id(tables, id, source);
        }
        result
    }

    fn pick_without_overlap(
        &mut self,
        tables: &OutfitTables,
        entries: &[PoolEntry],
        wanted: u32,
    ) -> Option<ItemId> {
        let query = MaskQuery {
            character_bonus: self.character_bonus,
            wanted,
            rank_limit: self.rank_limit,
            max_rank: false,
        };
        let mut picked = None;
        let result = pick_without_overlap(
            self.lcg,
            entries,
            query,
            &self.worn,
            |id| tables.facts(id),
            |id, _| picked = Some(id),
        );
        if let Some(id) = picked {
            let source = GenerationMode(u32::from(self.mode));
            self.equip_apparel_id(tables, id, source);
        }
        result
    }

    fn pick_shield(&mut self, entries: &[PoolEntry]) -> Option<RankPick> {
        let pick = pick_in_rank_window(self.lcg, entries, self.rank_limit);
        if let Some(pick) = pick {
            self.points = self.points.wrapping_sub(pick.cost);
            self.hands[1] = Some(HeldWeapon {
                id: pick.id,
                variant: pick.id.0,
            });
            self.items.push(EquippedItem {
                id: pick.id,
                cost: pick.cost,
                bonus: Bonus::default(),
                source: GenerationMode(u32::from(self.mode)),
                quality: 0,
                condition: 0,
            });
        }
        pick
    }

    fn pick_weapon(&mut self, hand: usize) -> Option<ItemId> {
        let before = self.points;
        let picked =
            self.pool
                .pick_weapon(self.lcg, &mut self.hands[hand], &mut self.points, |id| id.0);
        if let Some(id) = picked {
            self.items.push(EquippedItem {
                id,
                cost: before.wrapping_sub(self.points),
                bonus: Bonus::default(),
                source: GenerationMode(u32::from(self.mode)),
                quality: 0,
                condition: 0,
            });
        }
        picked
    }

    fn finish(self, look: OutfitLook) -> GeneratedOutfit {
        GeneratedOutfit {
            hands: self.hands,
            items: self.items,
            points: self.points,
            worn: self.worn,
            mode: self.mode,
            quality: self.quality,
            condition: self.condition,
            look,
        }
    }
}

/// Draw a stepped high word in `0..range`.
fn roll(lcg: &mut Lcg, range: u32) -> u32 {
    lcg.step();
    lcg.high(range)
}

/// The low-tier branch (the original's first `bVar8`): rank five is false,
/// below three is true, rank three is a half chance, and above three is an
/// eighth chance unless max-rank mode is on.
fn low_tier(lcg: &mut Lcg, rank: i32, max_rank: bool) -> bool {
    if rank == 5 {
        return false;
    }
    if rank < 3 {
        return true;
    }
    if rank == 3 {
        return roll(lcg, 2) == 0;
    }
    if max_rank {
        return false;
    }
    roll(lcg, 8) == 0
}

/// The fixed look preset selection, with the computed height, skin and
/// condition seed.
fn finish_look(preset: [f32; 20], height: f32, skin: f32, condition_seed: f32) -> OutfitLook {
    OutfitLook {
        height: if height > 1.0 { 1.0 } else { height },
        skin,
        condition_seed,
        preset,
    }
}

/// Dress a full outfit from an eight-byte profile.
///
/// Byte zero is the low flag byte: bit one enables max-rank mode, bit two
/// skips the early weapon/suit branch, bits three and four change the
/// description mode and the hood/jewel pass. Byte one is the rank limit.
/// Bytes two and three feed the height and skin look values.
///
/// `unit` supplies the original's separate look randomiser; the outfit rolls
/// themselves come from `lcg` in the original's order.
/// [`generate_profile`] drawing its unit values from the same sequence as
/// its high-word draws, as the engine's shared randomiser does.
pub fn generate_profile_lcg(
    profile: &[u8; 8],
    lcg: &mut Lcg,
    tables: &OutfitTables,
    character: &OutfitCharacter,
) -> GeneratedOutfit {
    generate_profile_with(profile, lcg, tables, character, |lcg| lcg.unit())
}

/// Dress a full outfit from an eight-byte profile.
///
/// Byte zero is the low flag byte: bit one enables max-rank mode, bit two
/// skips the early weapon/suit branch, bits three and four change the
/// description mode and the hood/jewel pass. Byte one is the rank limit.
/// Bytes two and three feed the height and skin look values.
///
/// `unit` supplies the original's separate look randomiser; the outfit rolls
/// themselves come from `lcg` in the original's order.
pub fn generate_profile(
    profile: &[u8; 8],
    lcg: &mut Lcg,
    tables: &OutfitTables,
    character: &OutfitCharacter,
    unit: impl FnMut() -> f32,
) -> GeneratedOutfit {
    let mut unit = unit;
    generate_profile_with(profile, lcg, tables, character, move |_| unit())
}

fn generate_profile_with(
    profile: &[u8; 8],
    lcg: &mut Lcg,
    tables: &OutfitTables,
    character: &OutfitCharacter,
    mut unit: impl FnMut(&mut Lcg) -> f32,
) -> GeneratedOutfit {
    let rank = i32::from(profile[1]);
    let max_rank = profile[0] & 2 != 0;
    let flags = profile[0];
    let mut dresser = Dresser::new(lcg, rank, character.bonus_mask());
    let preset;

    if flags & 4 == 0 {
        dresser.pool.clear();
        if rank < 5 && character.has_premade_skill() && roll(dresser.lcg, 3) == 0 {
            dresser.add_ranked(&tables.weapons_a);
            dresser.pick_weapon(0);
            dresser.pick_weapon(1);
            preset = LOOK_PRESET_ROUGH;
        } else {
            let draw = roll(dresser.lcg, 2);
            let experience = u64::from(character.experience as u16);
            let threshold = (u64::from(draw) + ((rank + 1) / 2) as u64) * 5000;
            if rank == 0 || experience <= threshold {
                dresser.add_ranked(&tables.weapons_a);
                dresser.halve_candidates();
                dresser.add_ranked(&tables.weapons_b);
                dresser.add_ranked(&tables.weapons_c);
                if rank == 5 {
                    dresser.add_unranked(&tables.weapons_d);
                }
                preset = LOOK_PRESET_ROUGH;
            } else {
                dresser.pick_shield(&tables.shields);
                if rank == 5 {
                    dresser.add_ranked(&tables.ranked_armour);
                } else {
                    dresser.add_ranked(&tables.weapons_a);
                }
                preset = LOOK_PRESET_ARMED;
            }
            dresser.pick_weapon(0);
        }
    } else {
        preset = LOOK_PRESET_ROUGH;
    }
    dresser.worn.floats = preset;

    let low_tier = low_tier(dresser.lcg, rank, max_rank);
    let wanted = if low_tier { 0x16 } else { 0x44 };

    let rank_scale = (f64::from(rank) * 0.05) as f32;
    let smooth = smooth_unit(unit(dresser.lcg));
    let height = ((1.0 - rank_scale) * smooth + rank_scale) as f64;
    let height = (height + f64::from(profile[2]) * 0.01) as f32;
    let mut skin = (f64::from(profile[3]) * 0.01) as f32;
    if character.warmth > 0.0 {
        skin = (f64::from(skin) * 0.3 + f64::from(character.warmth) * 0.7) as f32;
    }
    dresser.condition_value = skin;
    let condition_seed = (f64::from(unit(dresser.lcg)) + f64::from(rank) * 0.4) as f32;

    if flags & 8 != 0 {
        dresser.mode = 2;
    }
    let family = rank == 5 && dresser.hands[1].is_none();
    if rank == 5 || (max_rank && rank > 1) {
        dresser.equip_apparel_id(tables, ITEM_402, GenerationMode(u32::from(dresser.mode)));
        dresser.equip_apparel_id(tables, ITEM_1359, GenerationMode(u32::from(dresser.mode)));
        dresser.equip_apparel_id(tables, ITEM_251, GenerationMode(u32::from(dresser.mode)));
    } else {
        dresser.pick_masked(tables, &tables.neck, wanted, max_rank);
        dresser.pick_masked(tables, &tables.head, 0xffff, max_rank);
        dresser.pick_masked(tables, &tables.feet, 0xffff, max_rank);
    }
    if flags & 8 != 0 {
        dresser.mode = 1;
    }
    if flags & 0x10 != 0 && roll(dresser.lcg, 4) == 0 {
        dresser.pick_masked(tables, &tables.hood, 0xffff, max_rank);
    }
    if !family {
        dresser.pick_masked(tables, &tables.cloak, 0xffff, max_rank);
        dresser.pick_masked(tables, &tables.hands, 0xffff, max_rank);
        dresser.pick_masked(tables, &tables.torso_g, 0xffff, max_rank);
    }

    dresser.pool.clear();
    if family {
        if roll(dresser.lcg, 10) == 0 {
            dresser.equip_apparel_id(tables, ITEM_439, GenerationMode(u32::from(dresser.mode)));
            dresser.equip_apparel_id(tables, ITEM_436, GenerationMode(u32::from(dresser.mode)));
            dresser.equip_apparel_id(tables, ITEM_85, GenerationMode(u32::from(dresser.mode)));
            dresser.equip_apparel_id(tables, ITEM_435, GenerationMode(u32::from(dresser.mode)));
        } else {
            dresser.add_masked(&tables.main_a, 0xffff, max_rank);
            dresser.add_masked(&tables.main_b, 0xffff, max_rank);
            dresser.add_masked(&tables.main_c, 0xffff, max_rank);
            if roll(dresser.lcg, 4) == 0 {
                dresser.add_masked(&tables.alt_d, 0xffff, max_rank);
                dresser.add_masked(&tables.underwear, 0xffff, max_rank);
                for _ in 0..6 {
                    dresser.pick_and_equip(tables);
                }
            } else if roll(dresser.lcg, 4) == 0 {
                dresser.add_masked(&tables.alt_c, 0xffff, max_rank);
                for _ in 0..5 {
                    dresser.pick_and_equip(tables);
                }
            } else {
                dresser.add_masked(&tables.alt_a, 0xffff, max_rank);
                dresser.add_masked(&tables.alt_b, 0xffff, max_rank);
                for _ in 0..6 {
                    dresser.pick_and_equip(tables);
                }
            }
        }
    } else {
        if rank > 2 || roll(dresser.lcg, 3) == 0 {
            dresser.add_masked(&tables.body_pool, 0xffff, false);
        }
        if flags & 8 != 0 {
            dresser.mode = 3;
        }
        dresser.add_masked(&tables.torso_a, wanted, max_rank);
        if flags & 8 != 0 {
            dresser.mode = 1;
        }
        dresser.add_masked(&tables.torso_c, wanted, max_rank);
        dresser.add_masked(&tables.torso_e, wanted, max_rank);
        dresser.add_masked(&tables.torso_d, wanted, max_rank);
        if roll(dresser.lcg, 2) == 0 {
            dresser.add_masked(&tables.torso_i, wanted, max_rank);
        }
        dresser.add_masked(&tables.torso_h, wanted, max_rank);
        dresser.add_masked(&tables.torso_f, 0xffff, max_rank);
        for _ in 0..6 {
            dresser.pick_and_equip(tables);
        }
        if flags & 8 != 0 {
            dresser.mode = 1;
        }
        if max_rank || roll(dresser.lcg, 3) == 0 {
            dresser.pick_without_overlap(tables, &tables.full_set, 0xffff);
        }
        dresser.pick_without_overlap(tables, &tables.coat, 0xffff);
        dresser.pick_without_overlap(tables, &tables.torso_b, 0xffff);
        if flags & 0x10 == 0 {
            dresser.pick_without_overlap(tables, &tables.jewel, 0xffff);
        }
    }
    dresser.pick_masked(tables, &tables.belt, 0xffff, max_rank);
    if max_rank {
        if flags & 8 != 0 {
            // The original picks mode three only while two of its globals
            // are clear (one is a flag a previous piece's description sets
            // in the description-phrase module, which is not modelled), and
            // mode two otherwise. Both leave the rolls untouched.
            dresser.mode = 3;
        }
        dresser.pick_masked(tables, &tables.armour_b, 0xffff, max_rank);
    } else {
        dresser.pick_masked(tables, &tables.armour_a, 0xffff, max_rank);
    }

    dresser.finish(finish_look(preset, height, skin, condition_seed))
}

/// Dress a full outfit from a numeric level.
///
/// `warmth` is the height bias the original callers pass (`level * 0.05`);
/// it is added to the height value. `mode_flags` bit eight skips the early
/// weapon/suit branch; its low byte is stored as the mode flag byte.
/// [`generate_by_level`] drawing its unit values from the same sequence as
/// its high-word draws, as the engine's shared randomiser does.
pub fn generate_by_level_lcg(
    level: i32,
    warmth: f32,
    mode_flags: u32,
    lcg: &mut Lcg,
    tables: &OutfitTables,
    character: &OutfitCharacter,
) -> GeneratedOutfit {
    generate_by_level_with(level, warmth, mode_flags, lcg, tables, character, |lcg| {
        lcg.unit()
    })
}

/// Dress a full outfit from a numeric level.
///
/// `warmth` is the height bias the original callers pass (`level * 0.05`);
/// it is added to the height value. `mode_flags` bit eight skips the early
/// weapon/suit branch; its low byte is stored as the mode flag byte.
pub fn generate_by_level(
    level: i32,
    warmth: f32,
    mode_flags: u32,
    lcg: &mut Lcg,
    tables: &OutfitTables,
    character: &OutfitCharacter,
    unit: impl FnMut() -> f32,
) -> GeneratedOutfit {
    let mut unit = unit;
    generate_by_level_with(
        level,
        warmth,
        mode_flags,
        lcg,
        tables,
        character,
        move |_| unit(),
    )
}

fn generate_by_level_with(
    level: i32,
    warmth: f32,
    mode_flags: u32,
    lcg: &mut Lcg,
    tables: &OutfitTables,
    character: &OutfitCharacter,
    mut unit: impl FnMut(&mut Lcg) -> f32,
) -> GeneratedOutfit {
    let rank = level.max(0);
    let max_rank = false;
    let mut dresser = Dresser::new(lcg, rank, character.bonus_mask());
    let mut preset = LOOK_PRESET_ROUGH;

    if mode_flags & 0x100 == 0 && level >= 0 {
        dresser.pool.clear();
        if level < 5 && character.has_premade_skill() && roll(dresser.lcg, 3) == 0 {
            dresser.add_ranked(&tables.weapons_a);
            dresser.pick_weapon(0);
            dresser.pick_weapon(1);
            preset = LOOK_PRESET_ROUGH;
        } else {
            let draw = roll(dresser.lcg, 2);
            let experience = u64::from(character.experience as u16);
            let threshold = (u64::from(draw) + ((rank + 1) / 2) as u64) * 5000;
            if level < 1 || experience <= threshold {
                dresser.add_ranked(&tables.weapons_a);
                dresser.halve_candidates();
                dresser.add_ranked(&tables.weapons_b);
                dresser.add_ranked(&tables.weapons_c);
                if level == 5 {
                    dresser.add_unranked(&tables.weapons_d);
                }
                preset = LOOK_PRESET_ROUGH;
            } else {
                dresser.pick_shield(&tables.shields);
                if level == 5 {
                    dresser.add_ranked(&tables.ranked_armour);
                } else {
                    dresser.add_ranked(&tables.weapons_a);
                }
                preset = LOOK_PRESET_ARMED;
            }
            dresser.pick_weapon(0);
        }
    }
    dresser.worn.floats = preset;

    let low_tier = low_tier(dresser.lcg, rank, max_rank);
    let wanted = if low_tier { 0x16 } else { 0x44 };

    let rank_scale = (f64::from(rank) * 0.05) as f32;
    let smooth = smooth_unit(unit(dresser.lcg));
    // The original adds the bias to the rank scale first and the smoothed
    // draw last, and clamps the height before the skin is derived from it.
    let mut height = rank_scale + warmth;
    height += (1.0 - rank_scale) * smooth;
    if height > 1.0 {
        height = 1.0;
    }
    let mut skin = (0.7 - f64::from(height) * 0.6) as f32;
    if character.warmth > 0.0 {
        skin = (f64::from(skin) * 0.3 + f64::from(character.warmth) * 0.7) as f32;
    }
    dresser.condition_value = skin;
    let condition_seed = (f64::from(unit(dresser.lcg)) + f64::from(rank) * 0.4) as f32;

    let family = level == 5 && dresser.hands[1].is_none();
    if level == 5 || (max_rank && level > 1) {
        dresser.equip_apparel_id(tables, ITEM_402, GenerationMode(u32::from(dresser.mode)));
        dresser.equip_apparel_id(tables, ITEM_1359, GenerationMode(u32::from(dresser.mode)));
        dresser.equip_apparel_id(tables, ITEM_285, GenerationMode(u32::from(dresser.mode)));
    } else {
        dresser.pick_masked(tables, &tables.neck, wanted, false);
        dresser.pick_masked(tables, &tables.head, 0xffff, false);
        dresser.pick_masked(tables, &tables.feet, 0xffff, false);
    }
    if !family {
        dresser.pick_masked(tables, &tables.cloak, 0xffff, false);
        dresser.pick_masked(tables, &tables.hands, 0xffff, false);
        dresser.pick_masked(tables, &tables.torso_g, 0xffff, false);
    }

    dresser.pool.clear();
    if family {
        let smooth = smooth_unit(unit(dresser.lcg));
        skin = (0.4 - f64::from(smooth * 0.375)) as f32;
        dresser.condition_value = skin;
        if roll(dresser.lcg, 10) == 0 {
            dresser.equip_apparel_id(tables, ITEM_439, GenerationMode(u32::from(dresser.mode)));
            dresser.equip_apparel_id(tables, ITEM_436, GenerationMode(u32::from(dresser.mode)));
            dresser.equip_apparel_id(tables, ITEM_85, GenerationMode(u32::from(dresser.mode)));
            dresser.equip_apparel_id(tables, ITEM_435, GenerationMode(u32::from(dresser.mode)));
        } else {
            dresser.add_masked(&tables.main_a, 0xffff, false);
            dresser.add_masked(&tables.main_b, 0xffff, false);
            dresser.add_masked(&tables.main_c, 0xffff, false);
            if roll(dresser.lcg, 4) == 0 {
                dresser.add_masked(&tables.alt_d, 0xffff, false);
                dresser.add_masked(&tables.underwear, 0xffff, false);
                for _ in 0..6 {
                    dresser.pick_and_equip(tables);
                }
            } else if roll(dresser.lcg, 4) == 0 {
                dresser.add_masked(&tables.alt_c, 0xffff, false);
                for _ in 0..5 {
                    dresser.pick_and_equip(tables);
                }
            } else {
                dresser.add_masked(&tables.alt_a, 0xffff, false);
                dresser.add_masked(&tables.alt_b, 0xffff, false);
                for _ in 0..6 {
                    dresser.pick_and_equip(tables);
                }
            }
        }
    } else {
        if level > 2 || roll(dresser.lcg, 3) == 0 {
            dresser.add_masked(&tables.body_pool, 0xffff, false);
        }
        dresser.add_masked(&tables.torso_a, wanted, false);
        dresser.add_masked(&tables.torso_c, wanted, false);
        dresser.add_masked(&tables.torso_e, wanted, false);
        dresser.add_masked(&tables.torso_d, wanted, false);
        if roll(dresser.lcg, 2) == 0 {
            dresser.add_masked(&tables.torso_i, wanted, false);
        }
        dresser.add_masked(&tables.torso_h, wanted, false);
        dresser.add_masked(&tables.torso_f, 0xffff, false);
        for _ in 0..6 {
            dresser.pick_and_equip(tables);
        }
        if max_rank || roll(dresser.lcg, 3) == 0 {
            dresser.pick_without_overlap(tables, &tables.full_set, 0xffff);
        }
        dresser.pick_without_overlap(tables, &tables.coat, 0xffff);
        dresser.pick_without_overlap(tables, &tables.torso_b, 0xffff);
        dresser.pick_without_overlap(tables, &tables.jewel, 0xffff);
    }
    dresser.pick_masked(tables, &tables.belt, 0xffff, false);
    if max_rank {
        dresser.pick_masked(tables, &tables.armour_b, 0xffff, false);
    } else {
        dresser.pick_masked(tables, &tables.armour_a, 0xffff, false);
    }

    dresser.finish(finish_look(preset, height, skin, condition_seed))
}

/// The smooth unit draw: fold a uniform value around its middle so draws
/// cluster near zero and one.
///
/// Above the middle the deviation is squared and added to a half; below it
/// the deviation is squared and taken away from a half. A NaN fails the
/// "below the middle" test and takes the upper branch, staying NaN. All arithmetic is single
/// precision in the original's order.
#[must_use]
pub fn smooth_unit(unit: f32) -> f32 {
    if unit < 0.5 {
        let deviation = (0.5 - unit) * 2.0;
        0.5 - deviation * deviation * 0.5
    } else {
        let deviation = (unit - 0.5) * 2.0;
        deviation * deviation * 0.5 + 0.5
    }
}

/// Draw one smooth unit value from a unit source.
pub fn draw_smooth_unit(mut unit: impl FnMut() -> f32) -> f32 {
    smooth_unit(unit())
}

/// The weight a table entry keeps inside a rank window.
///
/// The window is the limit and the two ranks below it. At two below, the
/// weight is divided by four; at one below, by two; at the limit, it is
/// unchanged. Division truncates towards zero. `None` means the entry is
/// outside the window.
#[must_use]
pub fn rank_window_weight(weight: i32, rank: i32, limit: i32) -> Option<i32> {
    let low = limit.checked_sub(2)?;
    if rank > limit || rank < low {
        return None;
    }
    Some(if rank == low {
        weight / 4
    } else if rank == limit - 1 {
        weight / 2
    } else {
        weight
    })
}

/// The high half of `total * state` as a signed threshold.
fn draw_threshold(total: i32, state: u32) -> i32 {
    (((i64::from(total) as u64).wrapping_mul(u64::from(state))) >> 32) as u32 as i32
}

/// One stepped weighted draw.
///
/// The weights are summed, the roll steps once, and the cumulative weights
/// are walked until one passes the threshold. Zero-weight rows can never be
/// chosen. `None` means the total was not positive or no row passed.
pub fn weighted_pick(lcg: &mut Lcg, weights: &[i32]) -> Option<usize> {
    let mut total = 0i32;
    for weight in weights {
        total = total.wrapping_add(*weight);
    }
    if total <= 0 {
        return None;
    }
    lcg.step();
    let threshold = draw_threshold(total, lcg.state());
    let mut cumulative = 0i32;
    for (index, weight) in weights.iter().enumerate() {
        cumulative = cumulative.wrapping_add(*weight);
        if threshold < cumulative {
            return Some(index);
        }
    }
    None
}

/// The stepped draw the two table picks take: it steps even when nothing has
/// weight, and when no row passes the threshold the threshold itself is the
/// row (row zero when the total is zero), as in the original, which never
/// checks the total. `None` only when that row is outside the table.
fn table_pick(lcg: &mut Lcg, weights: &[i32]) -> Option<usize> {
    let mut total = 0i32;
    for weight in weights {
        total = total.wrapping_add(*weight);
    }
    lcg.step();
    let threshold = draw_threshold(total, lcg.state());
    let mut cumulative = 0i32;
    for (index, weight) in weights.iter().enumerate() {
        cumulative = cumulative.wrapping_add(*weight);
        if threshold < cumulative {
            return Some(index);
        }
    }
    usize::try_from(threshold)
        .ok()
        .filter(|index| *index < weights.len())
}

/// Pick from a table inside the rank window ending at the limit.
///
/// The window weights are summed, one stepped draw is taken (even when the
/// window is empty, which then falls back to the first row), and the first
/// cumulative weight past the threshold wins. The result names the id and
/// the cost the caller subtracts from the character's points.
pub fn pick_in_rank_window(lcg: &mut Lcg, entries: &[PoolEntry], limit: i32) -> Option<RankPick> {
    let weights: Vec<i32> = entries
        .iter()
        .map(|entry| rank_window_weight(entry.weight, entry.rank, limit).unwrap_or(0))
        .collect();
    let index = table_pick(lcg, &weights)?;
    Some(RankPick {
        id: entries[index].id,
        cost: entries[index].cost,
    })
}

/// Pick from a masked table and equip the winner.
///
/// Entries are refused when the character's bonus mask shares a forbidden
/// bit or when the wanted mask shares no bit. An entry that names an item is
/// also refused above the rank limit; in max-rank mode it must instead sit at
/// the highest rank the masks and limit allow (at least zero), and only those
/// entries receive weight. An entry naming no item (the "nothing" row) skips
/// the rank checks. One stepped draw is always taken. The winner is passed to
/// the equip callback unless its id is zero.
pub fn pick_masked<E>(
    lcg: &mut Lcg,
    entries: &[PoolEntry],
    query: MaskQuery,
    mut equip: E,
) -> Option<ItemId>
where
    E: FnMut(ItemId, &Bonus),
{
    let highest = if query.max_rank {
        entries
            .iter()
            .filter(|entry| mask_accepts(query, entry))
            .fold(0, |highest, entry| highest.max(entry.rank))
    } else {
        i32::MIN
    };
    let weights: Vec<i32> = entries
        .iter()
        .map(|entry| {
            let ranked = if query.max_rank {
                entry.rank == highest
            } else {
                entry.rank <= query.rank_limit
            };
            if masks_accept(query, entry) && (entry.id.0 == 0 || ranked) {
                entry.weight
            } else {
                0
            }
        })
        .collect();
    let index = table_pick(lcg, &weights)?;
    let entry = &entries[index];
    if entry.id.0 != 0 {
        equip(entry.id, &entry.bonus);
    }
    Some(entry.id)
}

/// Pick an entry that does not overlap anything already worn.
///
/// The first pass marks entries that pass the masks and the rank limit and
/// share no bonus word with the worn set, and sums their weights. An entry
/// whose item is not in the item database (an entry naming no item included)
/// has no words to overlap and is marked. The candidate's cost is not checked
/// here; the pool picks that pay for an item check it when they add it. When
/// the total is not positive nothing is drawn and the roll does not step.
/// Otherwise one stepped draw is taken, then a second pass walks only the
/// marked entries to the pick; a threshold past the table equips nothing.
pub fn pick_without_overlap<F, E>(
    lcg: &mut Lcg,
    entries: &[PoolEntry],
    query: MaskQuery,
    worn: &WornBonus,
    mut lookup: F,
    mut equip: E,
) -> Option<ItemId>
where
    F: FnMut(ItemId) -> Option<ItemFacts>,
    E: FnMut(ItemId, &Bonus),
{
    let mut marked = vec![false; entries.len()];
    let mut total = 0i32;
    for (index, entry) in entries.iter().enumerate() {
        if !mask_accepts(query, entry) {
            continue;
        }
        let facts = if entry.id.0 == 0 {
            None
        } else {
            lookup(entry.id)
        };
        if facts.is_some_and(|facts| worn.overlaps(&facts.bonus_words)) {
            continue;
        }
        marked[index] = true;
        total = total.wrapping_add(entry.weight);
    }
    if total <= 0 {
        return None;
    }
    lcg.step();
    let threshold = draw_threshold(total, lcg.state());
    let mut cumulative = 0i32;
    let mut chosen = None;
    for (index, entry) in entries.iter().enumerate() {
        if !marked[index] {
            continue;
        }
        cumulative = cumulative.wrapping_add(entry.weight);
        if threshold < cumulative {
            chosen = Some(index);
            break;
        }
    }
    let entry = &entries[chosen?];
    if entry.id.0 != 0 {
        equip(entry.id, &entry.bonus);
    }
    Some(entry.id)
}

/// Whether a masked query accepts an entry.
fn mask_accepts(query: MaskQuery, entry: &PoolEntry) -> bool {
    masks_accept(query, entry) && entry.rank <= query.rank_limit
}

/// Whether a masked query's two masks accept an entry, rank aside.
fn masks_accept(query: MaskQuery, entry: &PoolEntry) -> bool {
    query.character_bonus & entry.forbidden == 0 && query.wanted & entry.wanted != 0
}

/// Draw an index in `0..range` that differs from the previous value.
///
/// The roll steps once; if the drawn index equals the previous one, the roll
/// steps again and redraws. A second repeat is accepted, as in the original.
pub fn pick_other_index(lcg: &mut Lcg, range: u32, previous: u32) -> u32 {
    lcg.step();
    let mut index = lcg.high(range);
    if index == previous {
        lcg.step();
        index = lcg.high(range);
    }
    index
}

/// The fit score of a candidate: worn plus candidate over worn, per slot,
/// summed in single precision, minus one per slot, then a percentage.
///
/// A worn float of zero produces an infinity or a NaN, as in the original;
/// the cast saturates rather than panicking.
#[must_use]
pub fn score_candidate(worn: &WornBonus, bonus: &Bonus) -> i32 {
    let mut sum = 0.0f32;
    for (worn, candidate) in worn.floats.iter().zip(bonus.floats.iter()) {
        let slot = (worn + candidate) / worn;
        sum = (slot + sum) - 1.0;
    }
    cvtss2si_low(sum * 100.0)
}

/// The original's single-precision to 64-bit integer conversion, rounding to
/// nearest even, whose low half is stored: NaN and out-of-range values give
/// the "integer indefinite" pattern, whose low half is zero.
fn cvtss2si_low(value: f32) -> i32 {
    let rounded = value.round_ties_even();
    let wide = if rounded.is_nan() || rounded >= 9.223_372e18 || rounded < -9.223_372e18 {
        i64::MIN
    } else {
        rounded as i64
    };
    wide as i32
}

/// Merge one apparel piece into the outfit.
///
/// The twenty bonus words are unioned, the twenty bonus floats are added, and
/// the item's low rank bits are subtracted from the remaining points. The
/// condition byte is supplied by a callback, which is where the original's
/// quality and wear arithmetic belongs. The input facts are not changed.
#[must_use]
pub fn equip_apparel(
    worn: &WornBonus,
    points: i32,
    facts: &ItemFacts,
    condition: impl FnOnce(&ItemFacts) -> u8,
) -> ApparelFit {
    let mut bonus = *worn;
    bonus.merge(&Bonus {
        words: facts.bonus_words,
        floats: facts.bonus_floats,
    });
    ApparelFit {
        points: points.wrapping_sub(i32::from(facts.bulk())),
        bonus,
        quality: facts.quality,
        condition: condition(facts),
    }
}

/// The eight quality markers an outfit record carries, one per layer (the
/// marking loops run their counter to 7 inclusive).
pub const QUALITY_MARKERS: usize = 8;

/// Mark an equipped outfit's quality layers.
///
/// Mode 1 gives every unmarked layer marker 1. Mode 2 gives the first layer
/// 6 and every other unmarked layer 1. Mode 3 gives the first layer 7,
/// every other unmarked layer 1, and then the second layer 6 when bit 0 of
/// the second layer's flag word (`flag`) is set. Any other mode leaves the
/// markers alone.
pub fn mark_quality(mode: u32, markers: &mut [u8; QUALITY_MARKERS], flag: bool) {
    match mode {
        1 => {
            for marker in markers.iter_mut() {
                if *marker == 0 {
                    *marker = 1;
                }
            }
        }
        2 => {
            markers[0] = 6;
            for marker in markers.iter_mut().skip(1) {
                if *marker == 0 {
                    *marker = 1;
                }
            }
        }
        3 => {
            markers[0] = 7;
            for marker in markers.iter_mut().skip(1) {
                if *marker == 0 {
                    *marker = 1;
                }
            }
            if flag {
                markers[1] = 6;
            }
        }
        _ => {}
    }
}

/// The outfit after one apparel piece is merged in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApparelFit {
    /// The remaining points.
    pub points: i32,
    /// The merged worn bonus.
    pub bonus: WornBonus,
    /// The quality byte kept from the item.
    pub quality: u8,
    /// The condition byte supplied by the caller's callback.
    pub condition: u8,
}

/// Fill weapon table rows from item facts.
///
/// Zero-weight rows take the default weight of ten. Rows with no id are left
/// alone, and rows whose item is unknown keep the default weight but get no
/// facts. The class is filled only when it is still zero, from the item kind:
/// kinds one and two are class one, three and four are class four, five is
/// eight and six is the top class. Rank comes from the top bits of the low
/// rank byte and points from its low five bits. Each non-zero unit count
/// gives a bonus float scaled by the material bytes and the square root of
/// the units.
pub fn fill_weapon_rows<F>(rows: &mut [WeaponRow], mut lookup: F)
where
    F: FnMut(ItemId) -> Option<ItemFacts>,
{
    for row in rows {
        if row.id.0 == 0 {
            continue;
        }
        if row.weight == 0 {
            row.weight = 10;
        }
        let Some(facts) = lookup(row.id) else {
            continue;
        };
        if row.class == 0 {
            row.class = weapon_class(facts.weapon_kind);
        }
        row.rank = u32::from((facts.rank_word as u8) >> 5);
        row.points = i32::from(facts.rank_word & BULK_MASK);
        for (float, units) in row.bonus_floats.iter_mut().zip(facts.bonus_units.iter()) {
            if *units == 0 {
                continue;
            }
            let scale = ((f64::from(*units) * 0.01).sqrt() * 0.05 * 0.5) as f32;
            *float = (u32::from(facts.material_lo) + u32::from(facts.material_hi)) as f32 * scale;
        }
    }
}

/// The weapon class for an item kind byte.
fn weapon_class(kind: u8) -> u32 {
    match kind {
        1 | 2 => 1,
        3 | 4 => 4,
        5 => 8,
        6 => 0x40,
        _ => 0,
    }
}

/// Fill shield table rows from item facts.
///
/// Zero-weight rows take the default weight of ten. Rows with no id are
/// skipped. A zero class is filled from the top bits of the shield's rank
/// byte; the rank comes from its low five bits.
pub fn fill_shield_rows<F>(rows: &mut [ShieldRow], mut lookup: F)
where
    F: FnMut(ItemId) -> Option<ItemFacts>,
{
    for row in rows {
        if row.id.0 == 0 {
            continue;
        }
        if row.weight == 0 {
            row.weight = 10;
        }
        let Some(facts) = lookup(row.id) else {
            continue;
        };
        if row.class == 0 {
            row.class = u32::from((facts.rank_word as u8) >> 5);
        }
        row.rank = i32::from(facts.rank_word & BULK_MASK);
    }
}

/// Fill apparel table rows from item facts.
///
/// Zero-weight rows take the default weight of ten. Rows with no id are
/// skipped. A zero class is filled from the item's apparel class; the points
/// are always taken from the item.
pub fn fill_apparel_rows<F>(rows: &mut [ApparelRow], mut lookup: F)
where
    F: FnMut(ItemId) -> Option<ItemFacts>,
{
    for row in rows {
        if row.id.0 == 0 {
            continue;
        }
        if row.weight == 0 {
            row.weight = 10;
        }
        let Some(facts) = lookup(row.id) else {
            continue;
        };
        if row.class == 0 {
            row.class = facts.apparel_class;
        }
        row.points = facts.apparel_points;
    }
}

/// One row of profession progress: total experience and the column gained at
/// each rank.
///
/// The original keeps a row as eight bytes, a sixteen-bit total and then one
/// byte per rank; with the shipped grids a row never gains more than six
/// ranks, so the bytes stay inside the row. The twelve slots here only keep a
/// custom catalog from overrunning, which the original would do into the next
/// row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProfessionRow {
    /// Total experience in this row, capped by the original's twelve ranks.
    pub experience: u16,
    /// The one-based column gained at each rank index; zero means none.
    pub levels: [u8; PROFESSION_TRACKS],
}

/// The four profession rows of one character.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProfessionProgress {
    /// One row per profession track.
    pub rows: [ProfessionRow; PROFESSION_ROWS],
}

/// The two profession weight grids and their prerequisite links.
///
/// The original keeps two grids and drifts between them as characters are
/// generated. A nonzero prerequisite link names a one-based column in the
/// same row that must already be marked. The default is the shipped data
/// (`arena_outfit_data`); a caller may supply other grids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfessionCatalog {
    /// The grid used when the drift roll picks the rare side.
    pub weights_a: [[i32; PROFESSION_COLUMNS]; PROFESSION_ROWS],
    /// The grid used otherwise.
    pub weights_b: [[i32; PROFESSION_COLUMNS]; PROFESSION_ROWS],
    /// First prerequisite chain. The grid selects columns in the same row;
    /// it is shared by both weight grids.
    pub prereq_a: [[u8; PROFESSION_COLUMNS]; PROFESSION_ROWS],
    /// Second prerequisite chain, shared by both weight grids.
    pub prereq_b: [[u8; PROFESSION_COLUMNS]; PROFESSION_ROWS],
    /// The drift counter. It steps up by three on the rare side and down by
    /// one otherwise, within the original's bounds.
    pub drift: i32,
}

impl Default for ProfessionCatalog {
    fn default() -> Self {
        Self {
            weights_a: data::PROFESSION_WEIGHTS_RARE,
            weights_b: data::PROFESSION_WEIGHTS_COMMON,
            prereq_a: data::PROFESSION_PREREQ_A,
            prereq_b: data::PROFESSION_PREREQ_B,
            drift: 0,
        }
    }
}

/// Spend one experience total across the profession grid.
///
/// The experience is split into rank chunks of five thousand, with the last
/// chunk holding the remainder. The first roll picks the weight grid; each
/// chunk then rolls once, picks an unmarked eligible cell, marks it, adds the
/// chunk to that row's experience and records the one-based column at the
/// row's current rank index. Returns the number of rank chunks granted.
///
/// With a normal catalog this consumes one roll plus one per chunk, which is
/// what the look generator's later draws depend on.
pub fn grant_profession_levels(
    lcg: &mut Lcg,
    catalog: &mut ProfessionCatalog,
    progress: &mut ProfessionProgress,
    experience: u32,
) -> u32 {
    if experience == 0 {
        return 0;
    }
    let mut remaining = experience;
    let chunks = (experience / EXPERIENCE_PER_RANK + 1).min(PROFESSION_TRACKS as u32);
    if chunks * EXPERIENCE_PER_RANK < remaining {
        remaining = chunks * EXPERIENCE_PER_RANK;
    }
    let mut marks = [[false; PROFESSION_COLUMNS]; PROFESSION_ROWS];
    lcg.step();
    let span = (catalog.drift + 3).max(0) as u32;
    let rare = lcg.high(span) == 0;
    if rare {
        if catalog.drift < PROFESSION_TRACKS as i32 {
            catalog.drift += 3;
        }
    } else if catalog.drift > -2 {
        catalog.drift -= 1;
    }
    let (weights, prereq_a, prereq_b) = if rare {
        (&catalog.weights_a, &catalog.prereq_a, &catalog.prereq_b)
    } else {
        (&catalog.weights_b, &catalog.prereq_a, &catalog.prereq_b)
    };
    let mut granted = 0;
    loop {
        let mut candidates = Vec::with_capacity(PROFESSION_ROWS * PROFESSION_COLUMNS);
        let mut total = 0i32;
        for (row, weight_row) in weights.iter().enumerate() {
            for (column, weight) in weight_row.iter().enumerate() {
                if *weight == 0 || marks[row][column] {
                    continue;
                }
                if !prerequisite_met(prereq_a[row][column], row, &marks)
                    || !prerequisite_met(prereq_b[row][column], row, &marks)
                {
                    continue;
                }
                candidates.push((row, column, *weight));
                total = total.wrapping_add(*weight);
            }
        }
        if candidates.is_empty() {
            break;
        }
        lcg.step();
        let threshold = draw_threshold(total, lcg.state());
        let mut cumulative = 0i32;
        let mut chosen = None;
        for (row, column, weight) in &candidates {
            cumulative = cumulative.wrapping_add(*weight);
            if threshold < cumulative {
                chosen = Some((*row, *column));
                break;
            }
        }
        let Some((row, column)) =
            chosen.or_else(|| candidates.last().map(|(row, column, _)| (*row, *column)))
        else {
            break;
        };
        marks[row][column] = true;
        let progress_row = &mut progress.rows[row];
        let old = progress_row.experience;
        let add = if remaining < EXPERIENCE_PER_RANK + 1 {
            remaining as u16
        } else {
            EXPERIENCE_PER_RANK as u16
        };
        progress_row.experience = old.saturating_add(add);
        let rank = (old / EXPERIENCE_PER_RANK as u16) as usize;
        if rank < PROFESSION_TRACKS {
            progress_row.levels[rank] = (column + 1) as u8;
        }
        granted += 1;
        if remaining < EXPERIENCE_PER_RANK + 1 {
            break;
        }
        remaining -= EXPERIENCE_PER_RANK;
    }
    granted
}

/// Whether a one-based prerequisite column in the same row is marked.
fn prerequisite_met(
    link: u8,
    row: usize,
    marks: &[[bool; PROFESSION_COLUMNS]; PROFESSION_ROWS],
) -> bool {
    if link == 0 {
        return true;
    }
    marks[row][(usize::from(link) - 1).min(PROFESSION_COLUMNS - 1)]
}

/// The requested level and context for a random NPC.
#[derive(Clone, Copy, Debug)]
pub struct NpcRequest<'a> {
    /// The character level. It sets experience and the level-derived skill.
    pub level: i32,
    /// Flags the caller forces on. Bit zero is female, the next bit enables
    /// max-rank outfit picks, bit sixteen skips the sex roll and bit
    /// seventeen biases the strength curve upwards.
    pub forced_flags: u32,
    /// The character's warmth, used to blend skin and hair tone.
    pub warmth: f32,
    /// The variant index to avoid repeating.
    pub previous_variant: u32,
    /// Caller-supplied paired name rows. Retail names live in the caller, not
    /// here.
    pub names: &'a NameTable<'a>,
}

/// Paired first and last name rows, one table per sex.
#[derive(Clone, Copy, Debug, Default)]
pub struct NameTable<'a> {
    /// Male rows.
    pub male: &'a [(&'a str, &'a str)],
    /// Female rows.
    pub female: &'a [(&'a str, &'a str)],
}

/// The generated look of an NPC.
///
/// The field names are Exarust's reading of how the values are used: the
/// original keeps them as unnamed record fields. The two seed words are named
/// for the look features they feed; the wide roll is called the damage seed
/// and the small one the build seed, which is a guess about their consumers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NpcLook {
    /// The final flags word: bit zero is female, bits twenty to twenty-seven
    /// are the profession grade.
    pub flags: u32,
    /// Body frame, folded around the middle.
    pub frame: f32,
    /// Height scale, shortened at higher levels.
    pub height: f32,
    /// Weight deviation, in −1 to 1.
    pub weight: f32,
    /// Muscle multiplier, about 0.9 to 1.07.
    pub muscle: f32,
    /// Strength share, squared from a unit draw and optionally biased up.
    pub strength: f32,
    /// The first sixteen-bit feature word.
    pub face_seed: u16,
    /// The second sixteen-bit feature word.
    pub voice_seed: u16,
    /// The small build roll, zero to twelve.
    pub build_seed: u32,
    /// The full-width damage roll.
    pub damage_seed: u32,
    /// The byte taken from the roll's top eight bits.
    pub feature_seed: u8,
    /// The skin-tone byte.
    pub skin_tone: u8,
    /// The hair-tone byte.
    pub hair_tone: u8,
}

/// A generated NPC: level, experience, professions, name and look.
#[derive(Clone, Debug, PartialEq)]
pub struct NpcProfile<'a> {
    /// The level after the look generator's five-level cap.
    pub level: i32,
    /// The rolled experience total.
    pub experience: i32,
    /// Profession rank progress.
    pub professions: ProfessionProgress,
    /// The level-derived skill value.
    pub skill: f32,
    /// The profession variant index.
    pub variant: u32,
    /// The appearance index.
    pub appearance: u32,
    /// The grade taken from the flags' high nibble.
    pub profession: u8,
    /// The selected first and last name, or empty strings when the table is
    /// empty.
    pub name: (&'a str, &'a str),
    /// The look values.
    pub look: NpcLook,
}

/// Generate an NPC's experience, professions and look.
///
/// The experience is the level times ten thousand plus a roll below ten
/// thousand; it is spent on professions first. The skill is twenty per level
/// plus a unit draw below level five. The sex is rolled unless forced. The
/// profession variant avoids the previous one; the appearance and name are
/// drawn from caller tables. The rest follows the four look curves, the
/// strength square and its optional bias, the two feature words, the two
/// seeds, the profession grade and the sex bit, then the skin and hair
/// bytes. All unit values come from `unit`, in call order.
/// [`random_npc`] drawing its unit values from the same sequence as its
/// high-word draws, as the engine's shared randomiser does.
pub fn random_npc_lcg<'a>(
    lcg: &mut Lcg,
    request: &NpcRequest<'a>,
    professions: &mut ProfessionCatalog,
) -> NpcProfile<'a> {
    random_npc_with(lcg, request, professions, |lcg| lcg.unit())
}

/// Generate an NPC's experience, professions and look.
///
/// The experience is the level times ten thousand plus a roll below ten
/// thousand; it is spent on professions first. The skill is twenty per level
/// plus a unit draw below level five. The sex is rolled unless forced. The
/// profession variant avoids the previous one; the appearance and name are
/// drawn from caller tables. The rest follows the four look curves, the
/// strength square and its optional bias, the two feature words, the two
/// seeds, the profession grade and the sex bit, then the skin and hair
/// bytes. All unit values come from `unit`, in call order.
pub fn random_npc<'a>(
    lcg: &mut Lcg,
    request: &NpcRequest<'a>,
    professions: &mut ProfessionCatalog,
    unit: impl FnMut() -> f32,
) -> NpcProfile<'a> {
    let mut unit = unit;
    random_npc_with(lcg, request, professions, move |_| unit())
}

fn random_npc_with<'a>(
    lcg: &mut Lcg,
    request: &NpcRequest<'a>,
    professions: &mut ProfessionCatalog,
    mut unit: impl FnMut(&mut Lcg) -> f32,
) -> NpcProfile<'a> {
    lcg.step();
    let experience = request
        .level
        .wrapping_mul(10_000)
        .wrapping_add(lcg.high(9_999) as i32);
    let mut progress = ProfessionProgress::default();
    grant_profession_levels(lcg, professions, &mut progress, experience as u32);

    let level = request.level.min(5);
    let mut skill = (i64::from(level) * 20) as f32;
    if level < 5 {
        let level_bonus = unit(lcg) * 20.0;
        skill += level_bonus;
    }

    let mut flags = request.forced_flags;
    if flags & 0x1_0000 == 0 {
        lcg.step();
        if lcg.high(9) < 3 {
            flags |= 1;
        } else {
            flags &= !1;
        }
    }

    let female = flags & 1 != 0;
    let (variant, appearance, name) = if female {
        let variant = pick_other_index(lcg, 4, request.previous_variant);
        lcg.step();
        let appearance = lcg.high(5);
        let name = pick_name(lcg, request.names.female);
        (variant, appearance, name)
    } else {
        let variant = pick_other_index(lcg, 9, request.previous_variant);
        lcg.step();
        let appearance = lcg.high(8);
        let name = pick_name(lcg, request.names.male);
        (variant, appearance, name)
    };

    lcg.step();
    let damage_seed = lcg.high(0x7fff_ffff);

    let frame = (smooth_unit(unit(lcg)) - 0.5) * 0.5 + 0.5;
    // The height, deviation, muscle and skin sums run in double precision in
    // the original and are narrowed once at the end.
    let height_draw = smooth_unit(unit(lcg));
    let height = ((1.0 - f64::from(level) * 0.05) * f64::from(height_draw)) as f32;
    let weight = smooth_unit(unit(lcg)) * 2.0 - 1.0;
    let deviation = (f64::from(weight) * 0.3
        + (f64::from(0.5 - frame) * 0.6 + f64::from(height - 0.5) * 0.8))
        as f32;
    let deviation = if -LOOK_EPSILON <= deviation {
        if LOOK_EPSILON < deviation {
            deviation.sqrt()
        } else {
            deviation
        }
    } else {
        -(-deviation).sqrt()
    };
    let muscle_sum = smooth_unit(unit(lcg)) * 0.5 + (deviation * 0.5 + 0.5) * 0.5;
    let muscle = (f64::from(muscle_sum) * 0.17 + 0.9) as f32;
    let draw = unit(lcg);
    let mut strength = draw * draw;
    if flags & 0x2_0000 != 0 {
        strength = (1.0 - strength) * 0.75 + strength;
    }
    let face_seed = cvtss2si_low(smooth_unit(unit(lcg)) * 65535.0) as u16;
    let voice_seed = cvtss2si_low(smooth_unit(unit(lcg)) * 65535.0) as u16;

    lcg.step();
    let mut build_seed = lcg.high(13);
    if flags & 1 == 0 {
        lcg.step();
        let grade = lcg.high(9) as i32 - 2;
        flags &= 0xff0f_ffff;
        if grade > 0 {
            flags |= (grade as u32) << 20;
        }
    } else {
        if build_seed == 0 {
            lcg.step();
            build_seed = lcg.high(13);
        }
        flags &= 0xff0f_ffff;
    }
    let profession = ((flags >> 20) & 0xf) as u8;

    lcg.step();
    if lcg.high(2) == 0 || request.warmth > 0.5 {
        flags |= 0x1_0000;
    } else {
        flags &= !0x1_0000;
    }
    lcg.step();
    let feature_seed = (lcg.state() >> 24) as u8;

    let skin_draw = unit(lcg);
    let muscle_share = f64::from(strength) * 0.6;
    let skin_value = (f64::from(request.warmth * 0.5)
        + muscle_share * muscle_share
        + f64::from(skin_draw * skin_draw)) as f32;
    let skin_tone = if skin_value > 1.0 {
        255
    } else {
        cvtss2si_low(skin_value * 255.0) as u8
    };
    let hair_draw = unit(lcg);
    let mut hair_tone = cvtss2si_low((strength * 0.5 + hair_draw * 0.5) * 255.0) as u8;
    let blend = request.warmth * 0.75;
    hair_tone = cvtss2si_low(blend * 255.0 + (1.0 - blend) * f32::from(hair_tone)) as u8;

    NpcProfile {
        level,
        experience,
        professions: progress,
        skill,
        variant,
        appearance,
        profession,
        name,
        look: NpcLook {
            flags,
            frame,
            height,
            weight,
            muscle,
            strength,
            face_seed,
            voice_seed,
            build_seed,
            damage_seed,
            feature_seed,
            skin_tone,
            hair_tone,
        },
    }
}

/// Draw a paired name row, stepping once. Empty tables yield empty names.
fn pick_name<'a>(lcg: &mut Lcg, table: &'a [(&'a str, &'a str)]) -> (&'a str, &'a str) {
    if table.is_empty() {
        return ("", "");
    }
    lcg.step();
    let index = lcg.high(table.len() as u32) as usize;
    table[index.min(table.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u32, weight: i32, rank: i32, cost: i32) -> PoolEntry {
        PoolEntry {
            id: ItemId(id),
            weight,
            rank,
            cost,
            ..PoolEntry::default()
        }
    }

    fn close(actual: f32, expected: f32) -> bool {
        (actual - expected).abs() < 1e-6
    }

    #[test]
    fn the_quality_markers_fill_by_mode() {
        let mut markers = [0u8; QUALITY_MARKERS];
        mark_quality(1, &mut markers, false);
        assert_eq!(markers, [1; QUALITY_MARKERS]);
        let mut markers = [0, 2, 0, 0, 0, 0, 0, 0];
        mark_quality(2, &mut markers, false);
        assert_eq!(markers, [6, 2, 1, 1, 1, 1, 1, 1]);
        let mut markers = [0, 2, 0, 0, 0, 0, 0, 3];
        mark_quality(3, &mut markers, true);
        assert_eq!(markers, [7, 6, 1, 1, 1, 1, 1, 3]);
        let mut markers = [0, 2, 0, 0, 0, 0, 0, 0];
        mark_quality(3, &mut markers, false);
        assert_eq!(markers, [7, 2, 1, 1, 1, 1, 1, 1]);
        let mut markers = [0; QUALITY_MARKERS];
        mark_quality(9, &mut markers, true);
        assert_eq!(markers, [0; QUALITY_MARKERS]);
    }

    #[test]
    fn smooth_unit_pins_both_branches() {
        assert_eq!(smooth_unit(0.0), 0.0);
        assert_eq!(smooth_unit(0.5), 0.5);
        assert_eq!(smooth_unit(1.0), 1.0);
        assert_eq!(smooth_unit(0.25), 0.375);
        assert_eq!(smooth_unit(0.75), 0.625);
        assert!(smooth_unit(f32::NAN).is_nan());
    }

    #[test]
    fn rank_window_weights_and_draw() {
        assert_eq!(rank_window_weight(100, 3, 5), Some(25));
        assert_eq!(rank_window_weight(100, 4, 5), Some(50));
        assert_eq!(rank_window_weight(100, 5, 5), Some(100));
        assert_eq!(rank_window_weight(-9, 3, 5), Some(-2));
        assert_eq!(rank_window_weight(100, 2, 5), None);
        assert_eq!(rank_window_weight(100, 6, 5), None);

        let table = [
            entry(10, 100, 3, 4),
            entry(11, 50, 4, 5),
            entry(12, 25, 5, 6),
        ];
        let pick = pick_in_rank_window(&mut Lcg::new(0), &table, 5).unwrap();
        assert_eq!(
            pick,
            RankPick {
                id: ItemId(10),
                cost: 4
            }
        );
        let pick = pick_in_rank_window(&mut Lcg::new(11), &table, 5).unwrap();
        assert_eq!(
            pick,
            RankPick {
                id: ItemId(11),
                cost: 5
            }
        );
        let pick = pick_in_rank_window(&mut Lcg::new(22), &table, 5).unwrap();
        assert_eq!(
            pick,
            RankPick {
                id: ItemId(12),
                cost: 6
            }
        );
    }

    #[test]
    fn masked_pick_respects_masks_and_max_rank() {
        let mut forbidden = entry(3, 100, 4, 0);
        forbidden.wanted = 1;
        forbidden.forbidden = 1;
        let mut wanted = entry(5, 100, 5, 0);
        wanted.wanted = 1;
        let table = [
            {
                let mut row = entry(1, 100, 2, 0);
                row.wanted = 1;
                row
            },
            {
                let mut row = entry(2, 100, 4, 0);
                row.wanted = 1;
                row.bonus.floats[0] = 3.0;
                row
            },
            forbidden,
            {
                let mut row = entry(4, 100, 3, 0);
                row.wanted = 1;
                row
            },
            wanted,
        ];
        let query = MaskQuery {
            character_bonus: 1,
            wanted: 1,
            rank_limit: 4,
            max_rank: true,
        };
        let mut equipped = Vec::new();
        let picked = pick_masked(&mut Lcg::new(0), &table, query, |id, bonus| {
            equipped.push((id, bonus.floats[0]));
        })
        .unwrap();
        assert_eq!(picked, ItemId(2));
        assert_eq!(equipped, vec![(ItemId(2), 3.0)]);

        let query = MaskQuery {
            character_bonus: 0,
            wanted: 1,
            rank_limit: 5,
            max_rank: false,
        };
        let mut only = table;
        only[0].weight = 0;
        only[2].weight = 0;
        only[3].weight = 0;
        only[4].weight = 0;
        let picked = pick_masked(&mut Lcg::new(0), &only, query, |_, _| {}).unwrap();
        assert_eq!(picked, ItemId(2));
    }

    #[test]
    fn two_pass_pick_skips_worn_unknown_and_ranked_out() {
        let mut worn = WornBonus::default();
        worn.words[0] = 0b100;
        let mut first = entry(10, 40, 1, 5);
        first.wanted = 1;
        first.bonus.words[0] = 0b010;
        let mut overlapping = entry(11, 100, 1, 5);
        overlapping.wanted = 1;
        overlapping.bonus.words[0] = 0b100;
        let mut nothing = entry(0, 100, 1, 5);
        nothing.wanted = 1;
        let mut too_high = entry(13, 100, 3, 5);
        too_high.wanted = 1;
        let table = [first, overlapping, nothing, too_high];
        let query = MaskQuery {
            character_bonus: 0,
            wanted: 1,
            rank_limit: 2,
            max_rank: false,
        };
        let mut equipped = Vec::new();
        let picked = pick_without_overlap(
            &mut Lcg::new(0),
            &table,
            query,
            &worn,
            |id| match id.0 {
                10..=12 => Some(ItemFacts {
                    id,
                    ..ItemFacts::default()
                }),
                _ => None,
            },
            |id, _| equipped.push(id),
        )
        .unwrap();
        assert_eq!(picked, ItemId(10));
        assert_eq!(equipped, vec![ItemId(10)]);

        let wanted_elsewhere = MaskQuery { wanted: 2, ..query };
        let picked = pick_without_overlap(
            &mut Lcg::new(0),
            &table,
            wanted_elsewhere,
            &worn,
            |_| Some(ItemFacts::default()),
            |_, _| panic!("nothing should be equipped"),
        );
        assert_eq!(picked, None);
    }

    #[test]
    fn the_two_pass_pick_ignores_the_cost() {
        // The first pass checks the worn overlap and the masks, not the
        // candidate's cost; only the later pool picks pay for an item.
        let mut costly = entry(12, 100, 1, 25);
        costly.wanted = 1;
        let table = [costly];
        let query = MaskQuery {
            character_bonus: 0,
            wanted: 1,
            rank_limit: 2,
            max_rank: false,
        };
        let picked = pick_without_overlap(
            &mut Lcg::new(0),
            &table,
            query,
            &WornBonus::default(),
            |id| {
                Some(ItemFacts {
                    id,
                    ..ItemFacts::default()
                })
            },
            |_, _| {},
        );
        assert_eq!(picked, Some(ItemId(12)));
    }

    #[test]
    fn the_table_picks_step_the_roll_even_with_nothing_weighted() {
        // Nothing passes the masks: the draw is taken anyway and the pick
        // falls back to the first row, which is equipped.
        let mut row = entry(7, 100, 1, 0);
        row.wanted = 2;
        let table = [row];
        let query = MaskQuery {
            character_bonus: 0,
            wanted: 1,
            rank_limit: 5,
            max_rank: false,
        };
        let mut lcg = Lcg::new(0);
        let mut equipped = Vec::new();
        let picked = pick_masked(&mut lcg, &table, query, |id, _| equipped.push(id));
        assert_eq!(picked, Some(ItemId(7)));
        assert_eq!(equipped, vec![ItemId(7)]);
        let mut stepped = Lcg::new(0);
        stepped.step();
        assert_eq!(lcg.state(), stepped.state());

        // The rank window does the same, and an empty table has no row.
        let mut lcg = Lcg::new(0);
        let pick = pick_in_rank_window(&mut lcg, &[entry(8, 100, 1, 3)], 5);
        assert_eq!(pick.map(|pick| pick.id), Some(ItemId(8)));
        assert_eq!(lcg.state(), stepped.state());
        let mut lcg = Lcg::new(0);
        assert_eq!(pick_in_rank_window(&mut lcg, &[], 5), None);
        assert_eq!(lcg.state(), stepped.state());
    }

    #[test]
    fn the_nothing_row_skips_the_rank_checks_in_the_masked_pick() {
        let mut high = entry(5, 100, 5, 0);
        high.wanted = 1;
        let mut nothing = entry(0, 100, 9, 0);
        nothing.wanted = 1;
        let table = [high, nothing];
        // Rank limit two: the item row is out, the nothing row still counts
        // and, being the only weight, wins without equipping anything.
        let query = MaskQuery {
            character_bonus: 0,
            wanted: 1,
            rank_limit: 2,
            max_rank: false,
        };
        let picked = pick_masked(&mut Lcg::new(0), &table, query, |_, _| {
            panic!("the nothing row equips nothing");
        });
        assert_eq!(picked, Some(ItemId(0)));
        // Max-rank mode finds the highest rank over the eligible rows (floor
        // zero), so a row at that rank keeps its weight.
        let query = MaskQuery {
            rank_limit: 5,
            max_rank: true,
            ..query
        };
        let mut equipped = Vec::new();
        let picked = pick_masked(&mut Lcg::new(0), &table, query, |id, _| equipped.push(id));
        assert_eq!(picked, Some(ItemId(5)));
        assert_eq!(equipped, vec![ItemId(5)]);
    }

    #[test]
    fn the_overlap_pick_lets_unknown_items_and_the_nothing_row_through() {
        let mut unknown = entry(40, 100, 1, 0);
        unknown.wanted = 1;
        let mut nothing = entry(0, 100, 1, 0);
        nothing.wanted = 1;
        let query = MaskQuery {
            character_bonus: 0,
            wanted: 1,
            rank_limit: 2,
            max_rank: false,
        };
        let mut equipped = Vec::new();
        let picked = pick_without_overlap(
            &mut Lcg::new(0),
            &[unknown.clone()],
            query,
            &WornBonus::default(),
            |_| None,
            |id, _| equipped.push(id),
        );
        assert_eq!(picked, Some(ItemId(40)));
        assert_eq!(equipped, vec![ItemId(40)]);
        let picked = pick_without_overlap(
            &mut Lcg::new(0),
            &[nothing],
            query,
            &WornBonus::default(),
            |_| panic!("the nothing row is not looked up"),
            |_, _| panic!("the nothing row equips nothing"),
        );
        assert_eq!(picked, Some(ItemId(0)));
        // With nothing marked the roll does not step.
        let mut lcg = Lcg::new(3);
        let before = lcg.state();
        let wanted_elsewhere = MaskQuery { wanted: 2, ..query };
        assert_eq!(
            pick_without_overlap(
                &mut lcg,
                &[unknown.clone()],
                wanted_elsewhere,
                &WornBonus::default(),
                |_| None,
                |_, _| {},
            ),
            None
        );
        assert_eq!(lcg.state(), before);
    }

    #[test]
    fn score_conversion_gives_zero_for_nan_and_out_of_range() {
        assert_eq!(cvtss2si_low(2.5), 2);
        assert_eq!(cvtss2si_low(3.5), 4);
        assert_eq!(cvtss2si_low(-1.5), -2);
        assert_eq!(cvtss2si_low(f32::NAN), 0);
        assert_eq!(cvtss2si_low(f32::INFINITY), 0);
        assert_eq!(cvtss2si_low(f32::NEG_INFINITY), 0);
        assert_eq!(cvtss2si_low(1.0e20), 0);
        // Beyond 32 bits but inside 64: the low half is kept.
        assert_eq!(cvtss2si_low(4_294_967_296.0), 0);
        assert_eq!(cvtss2si_low(4_294_967_296.0 * 2.0 + 4096.0), 4096);
    }

    #[test]
    fn add_masked_records_bonus_and_source() {
        let mut first = entry(1, 100, 2, 5);
        first.wanted = 1;
        first.bonus.floats[0] = 1.5;
        let mut highest = entry(2, 90, 3, 5);
        highest.wanted = 1;
        highest.bonus.floats[0] = 2.5;
        let mut costly = entry(3, 100, 3, 20);
        costly.wanted = 1;
        let mut low = entry(4, 100, 1, 5);
        low.wanted = 1;
        let mut other_mask = entry(5, 100, 3, 5);
        other_mask.wanted = 2;
        let table = [first, highest, costly, low, other_mask];
        let query = MaskQuery {
            character_bonus: 0,
            wanted: 1,
            rank_limit: 3,
            max_rank: true,
        };
        let mut pool = OutfitPool::new();
        assert_eq!(pool.add_masked(&table, query, 10, GenerationMode(2)), 1);
        assert_eq!(pool.len(), 1);
        let candidate = &pool.candidates()[0];
        assert_eq!(candidate.id, ItemId(2));
        assert_eq!(candidate.weight, 90);
        assert_eq!(candidate.cost, 5);
        assert_eq!(candidate.source, GenerationMode(2));
        assert_eq!(candidate.bonus.floats[0], 2.5);

        let query = MaskQuery {
            max_rank: false,
            ..query
        };
        pool.clear();
        assert_eq!(pool.add_masked(&table, query, 10, GenerationMode(0)), 3);
        let ids: Vec<u32> = pool.candidates().iter().map(|c| c.id.0).collect();
        assert_eq!(ids, vec![1, 2, 4]);
    }

    #[test]
    fn add_ranked_and_unranked_record_weights() {
        let table = [
            entry(1, 100, 4, 1),
            entry(2, 100, 5, 1),
            entry(3, 100, 3, 1),
            entry(4, 100, 5, 11),
            entry(0, 100, 5, 1),
        ];
        let mut pool = OutfitPool::new();
        assert_eq!(pool.add_ranked(&table, 5, 10, GenerationMode(1)), 2);
        assert_eq!(pool.candidates()[0].weight, 50);
        assert_eq!(pool.candidates()[1].weight, 100);
        assert_eq!(pool.candidates()[0].id, ItemId(1));
        assert_eq!(pool.candidates()[1].id, ItemId(2));

        let mut unranked = OutfitPool::new();
        assert_eq!(unranked.add_unranked(&table, 10, GenerationMode(3)), 3);
        let ids: Vec<u32> = unranked.candidates().iter().map(|c| c.id.0).collect();
        assert_eq!(ids, vec![1, 2, 3]);
        assert!(unranked.candidates().iter().all(|c| c.weight == 100));
    }

    #[test]
    fn score_and_prune_drops_and_scores() {
        let mut worn = WornBonus {
            floats: [2.0; BONUS_SLOTS],
            ..WornBonus::default()
        };
        worn.words[0] = 1;

        let mut good_low = entry(1, 0, 0, 0);
        good_low.bonus.floats = [1.0; BONUS_SLOTS];
        let mut good_high = entry(2, 0, 0, 0);
        good_high.bonus.floats = [3.0; BONUS_SLOTS];
        let unknown = entry(3, 0, 0, 0);
        let mut costly = entry(4, 0, 0, 0);
        costly.bonus.floats = [1.0; BONUS_SLOTS];
        let mut overlapping = entry(5, 0, 0, 0);
        overlapping.bonus.floats = [1.0; BONUS_SLOTS];
        let mut good_again = entry(6, 0, 0, 0);
        good_again.bonus.floats = [1.0; BONUS_SLOTS];
        let table = [
            good_low.clone(),
            good_high.clone(),
            unknown,
            costly,
            overlapping,
            good_again,
        ];
        let mut pool = OutfitPool::new();
        pool.add_unranked(&table, 100, GenerationMode(0));
        assert_eq!(pool.len(), 6);
        let remaining = pool.prune_and_score(10, &worn, |id| {
            if id.0 == 3 {
                return None;
            }
            let mut facts = ItemFacts {
                id,
                ..ItemFacts::default()
            };
            if id.0 == 4 {
                facts.rank_word = 18;
            }
            if id.0 == 5 {
                facts.bonus_words[0] = 1;
            }
            Some(facts)
        });
        assert_eq!(remaining, 3);
        let ids: Vec<u32> = pool.candidates().iter().map(|c| c.id.0).collect();
        assert_eq!(ids, vec![1, 2, 6]);
        assert_eq!(pool.candidates()[0].weight, 1_000);
        assert_eq!(pool.candidates()[1].weight, 3_000);
        assert_eq!(pool.candidates()[2].weight, 1_000);

        let only_worn = WornBonus {
            floats: [2.0; BONUS_SLOTS],
            ..WornBonus::default()
        };
        assert_eq!(score_candidate(&only_worn, &good_low.bonus), 1_000);
        assert_eq!(score_candidate(&only_worn, &good_high.bonus), 3_000);
    }

    #[test]
    fn weighted_candidate_pick_sets_source() {
        let mut low = entry(1, 0, 0, 0);
        low.bonus.floats = [1.0; BONUS_SLOTS];
        let mut high = entry(2, 0, 0, 0);
        high.bonus.floats = [3.0; BONUS_SLOTS];
        let mut pool = OutfitPool::new();
        pool.add_unranked(&[low], 100, GenerationMode(1));
        pool.add_unranked(&[high], 100, GenerationMode(3));
        let worn = WornBonus {
            floats: [2.0; BONUS_SLOTS],
            ..WornBonus::default()
        };
        let mut equipped = Vec::new();
        let pick = pool
            .pick_and_equip(
                &mut Lcg::new(8),
                10,
                &worn,
                |id| {
                    Some(ItemFacts {
                        id,
                        ..ItemFacts::default()
                    })
                },
                |pick| equipped.push((pick.id, pick.source)),
            )
            .unwrap();
        assert_eq!(pick.id, ItemId(2));
        assert_eq!(pick.weight, 3_000);
        assert_eq!(pick.source, GenerationMode(3));
        assert_eq!(equipped, vec![(ItemId(2), GenerationMode(3))]);

        let empty = OutfitPool::new().pick_and_equip(
            &mut Lcg::new(0),
            10,
            &worn,
            |_| None,
            |_| panic!("nothing should be equipped"),
        );
        assert_eq!(empty, None);
    }

    #[test]
    fn weapon_pick_randomises_variant_and_pays() {
        let mut pool = OutfitPool::new();
        pool.add_unranked(
            &[entry(7, 5, 0, 3), entry(8, 5, 0, 4)],
            100,
            GenerationMode(0),
        );
        let mut hand = None;
        let mut points = 20;
        let picked = pool
            .pick_weapon(&mut Lcg::new(0), &mut hand, &mut points, |id| id.0 + 100)
            .unwrap();
        assert_eq!(picked, ItemId(7));
        assert_eq!(
            hand,
            Some(HeldWeapon {
                id: ItemId(7),
                variant: 107
            })
        );
        assert_eq!(points, 17);

        let mut no_hand = None;
        let mut untouched = 20;
        assert_eq!(
            OutfitPool::new()
                .pick_weapon(&mut Lcg::new(0), &mut no_hand, &mut untouched, |id| id.0),
            None
        );
        assert_eq!(no_hand, None);
        assert_eq!(untouched, 20);
    }

    #[test]
    fn draw_index_avoids_previous() {
        assert_eq!(pick_other_index(&mut Lcg::new(0), 9, 99), 0);
        assert_eq!(pick_other_index(&mut Lcg::new(1), 9, 0), 7);
        assert_eq!(pick_other_index(&mut Lcg::new(4), 9, 99), 1);
    }

    #[test]
    fn profession_levels_follow_experience_chunks() {
        let mut catalog = ProfessionCatalog::default();
        let mut progress = ProfessionProgress::default();
        let granted = grant_profession_levels(&mut Lcg::new(0), &mut catalog, &mut progress, 7_000);
        assert_eq!(granted, 2);
        assert_eq!(progress.rows[0].experience, 5_000);
        assert_eq!(progress.rows[0].levels[0], 1);
        assert_eq!(progress.rows[1].experience, 0);
        assert_eq!(progress.rows[2].experience, 0);
        assert_eq!(progress.rows[3].experience, 2_000);
        assert_eq!(progress.rows[3].levels[0], 4);
        assert_eq!(catalog.drift, 3);

        let mut empty = ProfessionCatalog::default();
        let mut nothing = ProfessionProgress::default();
        assert_eq!(
            grant_profession_levels(&mut Lcg::new(0), &mut empty, &mut nothing, 0),
            0
        );
    }

    #[test]
    fn profession_grids_keep_their_prerequisites_and_rows() {
        for seed in 0..32 {
            let mut catalog = ProfessionCatalog::default();
            let mut progress = ProfessionProgress::default();
            let granted =
                grant_profession_levels(&mut Lcg::new(seed), &mut catalog, &mut progress, 60_000);
            assert_eq!(granted, 12);
            // The second row has no weights; the others each mark a cell once.
            assert_eq!(progress.rows[1].experience, 0);
            let total: u32 = progress.rows.iter().map(|r| u32::from(r.experience)).sum();
            assert_eq!(total, 60_000);
            for row in &progress.rows {
                let gained: Vec<u8> = row.levels.iter().copied().take_while(|c| *c != 0).collect();
                assert_eq!(gained.len() as u16, row.experience / 5_000);
                for (index, column) in gained.iter().enumerate() {
                    let before = &gained[..index];
                    match (progress.rows.iter().position(|r| r == row), column) {
                        // The first row's last column needs its first and fourth.
                        (Some(0), 8) => assert!(before.contains(&1) && before.contains(&4)),
                        // The last two rows' second and third columns chain.
                        (Some(2 | 3), 2) => assert!(before.contains(&1)),
                        (Some(2 | 3), 3) => assert!(before.contains(&2)),
                        _ => {}
                    }
                }
            }
        }
    }

    #[test]
    fn random_npc_pins_curves_and_seeds() {
        let unit_values = [0.0, 0.75, 0.6, 0.2, 0.9, 0.4, 0.3, 0.8, 0.55, 0.65];
        let mut next = 0;
        let male = [
            ("Given0", "Family0"),
            ("Given1", "Family1"),
            ("Given2", "Family2"),
        ];
        let names = NameTable {
            male: &male,
            female: &[],
        };
        let request = NpcRequest {
            level: 3,
            forced_flags: 0x3_0000,
            warmth: 0.25,
            previous_variant: 99,
            names: &names,
        };
        let mut catalog = ProfessionCatalog::default();
        let profile = random_npc(&mut Lcg::new(0), &request, &mut catalog, || {
            let value = unit_values[next];
            next += 1;
            value
        });
        assert_eq!(next, 10);
        assert_eq!(profile.experience, 30_000);
        assert_eq!(profile.level, 3);
        assert_eq!(profile.skill, 60.0);
        assert_eq!(profile.variant, 3);
        assert_eq!(profile.appearance, 3);
        assert_eq!(profile.profession, 5);
        assert_eq!(profile.name, ("Given0", "Family0"));
        assert_eq!(profile.professions.rows[0].experience, 20_000);
        assert_eq!(profile.professions.rows[1].experience, 0);
        assert_eq!(profile.professions.rows[2].experience, 0);
        assert_eq!(profile.professions.rows[3].experience, 10_000);
        assert_eq!(profile.professions.rows[0].levels[..4], [1, 3, 4, 2]);
        assert_eq!(profile.professions.rows[3].levels[..2], [4, 1]);
        let look = profile.look;
        assert_eq!(look.flags, 0x530_000);
        assert_eq!(look.frame, 0.5625);
        assert!(close(look.height, 0.442_000_001_668_930_05));
        assert!(close(look.weight, -0.360_000_014_305_114_75));
        assert!(close(look.muscle, 0.993_582_3));
        assert!(close(look.strength, 0.790_000_021_457_672_1));
        assert_eq!(look.face_seed, 27_525);
        assert_eq!(look.voice_seed, 44_564);
        assert_eq!(look.build_seed, 0);
        assert_eq!(look.damage_seed, 1_019_612_489);
        assert_eq!(look.feature_seed, 75);
        assert_eq!(look.skin_tone, 166);
        assert_eq!(look.hair_tone, 197);
    }

    #[test]
    fn weapon_rows_fill_from_facts() {
        let mut rows = [
            WeaponRow {
                id: ItemId(7),
                ..WeaponRow::default()
            },
            WeaponRow {
                id: ItemId(8),
                class: 4,
                ..WeaponRow::default()
            },
            WeaponRow {
                id: ItemId(9),
                ..WeaponRow::default()
            },
            WeaponRow::default(),
        ];
        fill_weapon_rows(&mut rows, |id| match id.0 {
            7 => {
                let mut facts = ItemFacts {
                    id,
                    rank_word: (3 << 5) | 7,
                    weapon_kind: 5,
                    material_lo: 1,
                    material_hi: 2,
                    ..ItemFacts::default()
                };
                facts.bonus_units[0] = 25;
                facts.bonus_units[2] = 100;
                Some(facts)
            }
            8 => Some(ItemFacts {
                id,
                rank_word: (1 << 5) | 2,
                weapon_kind: 3,
                ..ItemFacts::default()
            }),
            _ => None,
        });
        assert_eq!(rows[0].weight, 10);
        assert_eq!(rows[0].class, 8);
        assert_eq!(rows[0].rank, 3);
        assert_eq!(rows[0].points, 7);
        assert_eq!(rows[0].bonus_floats[1], 0.0);
        assert!(close(rows[0].bonus_floats[0], 0.0375));
        assert!(close(rows[0].bonus_floats[2], 0.075));
        assert_eq!(rows[1].class, 4);
        assert_eq!(rows[1].rank, 1);
        assert_eq!(rows[1].points, 2);
        assert_eq!(rows[2].weight, 10);
        assert_eq!(rows[2].class, 0);
        assert_eq!(rows[3].weight, 0);
        assert_eq!(rows[3].id, ItemId(0));
    }

    #[test]
    fn shield_and_apparel_rows_fill_from_facts() {
        let mut shields = [
            ShieldRow {
                id: ItemId(20),
                ..ShieldRow::default()
            },
            ShieldRow::default(),
        ];
        fill_shield_rows(&mut shields, |id| {
            (id.0 == 20).then_some(ItemFacts {
                id,
                rank_word: (2 << 5) | 5,
                ..ItemFacts::default()
            })
        });
        assert_eq!(shields[0].weight, 10);
        assert_eq!(shields[0].class, 2);
        assert_eq!(shields[0].rank, 5);
        assert_eq!(shields[1].weight, 0);

        let mut apparel = [
            ApparelRow {
                id: ItemId(30),
                ..ApparelRow::default()
            },
            ApparelRow {
                id: ItemId(31),
                class: 7,
                ..ApparelRow::default()
            },
            ApparelRow::default(),
        ];
        fill_apparel_rows(&mut apparel, |id| match id.0 {
            30 => Some(ItemFacts {
                id,
                apparel_class: 4,
                apparel_points: 12,
                ..ItemFacts::default()
            }),
            31 => Some(ItemFacts {
                id,
                apparel_class: 9,
                apparel_points: 3,
                ..ItemFacts::default()
            }),
            _ => None,
        });
        assert_eq!(apparel[0].weight, 10);
        assert_eq!(apparel[0].class, 4);
        assert_eq!(apparel[0].points, 12);
        assert_eq!(apparel[1].class, 7);
        assert_eq!(apparel[1].points, 3);
        assert_eq!(apparel[2].weight, 0);
    }

    #[test]
    fn apparel_equip_merges_bonus_and_points() {
        let mut worn = WornBonus::default();
        worn.words[0] = 0b101;
        worn.words[19] = 8;
        worn.floats[0] = 1.5;
        worn.floats[19] = 0.5;
        let mut facts = ItemFacts {
            id: ItemId(40),
            rank_word: 9,
            quality: 42,
            condition: 77,
            ..ItemFacts::default()
        };
        facts.bonus_words[0] = 0b010;
        facts.bonus_words[19] = 1;
        facts.bonus_floats[0] = 0.25;
        facts.bonus_floats[19] = 1.25;
        let fit = equip_apparel(&worn, 100, &facts, |_| facts.condition);
        assert_eq!(fit.points, 91);
        assert_eq!(fit.bonus.words[0], 0b111);
        assert_eq!(fit.bonus.words[19], 9);
        assert_eq!(fit.bonus.floats[0], 1.75);
        assert_eq!(fit.bonus.floats[19], 1.75);
        assert_eq!(fit.quality, 42);
        assert_eq!(fit.condition, 77);
    }
    fn synthetic_item(id: ItemId) -> Option<ItemFacts> {
        let mut facts = ItemFacts {
            id,
            rank_word: (2 << 5) | 1,
            ..ItemFacts::default()
        };
        facts.bonus_floats[0] = 1.0;
        Some(facts)
    }

    fn synthetic_tables() -> OutfitTables {
        OutfitTables::build(synthetic_item)
    }

    fn ids(outfit: &GeneratedOutfit) -> Vec<u32> {
        outfit.items.iter().map(|item| item.id.0).collect()
    }

    fn unit_half() -> f32 {
        0.5
    }

    macro_rules! assert_table_len {
        ($tables:expr, $field:ident, $raw:ident) => {
            assert_eq!($tables.$field.len(), data::$raw.len(), stringify!($field));
        };
    }

    #[test]
    fn outfit_tables_match_the_data_lengths() {
        let tables = synthetic_tables();
        assert_table_len!(tables, alt_a, ALT_A);
        assert_table_len!(tables, alt_b, ALT_B);
        assert_table_len!(tables, alt_c, ALT_C);
        assert_table_len!(tables, alt_d, ALT_D);
        assert_table_len!(tables, armour_a, ARMOUR_A);
        assert_table_len!(tables, armour_b, ARMOUR_B);
        assert_table_len!(tables, belt, BELT);
        assert_table_len!(tables, body_pool, BODY_POOL);
        assert_table_len!(tables, cloak, CLOAK);
        assert_table_len!(tables, coat, COAT);
        assert_table_len!(tables, feet, FEET);
        assert_table_len!(tables, full_set, FULL_SET);
        assert_table_len!(tables, hands, HANDS);
        assert_table_len!(tables, head, HEAD);
        assert_table_len!(tables, hood, HOOD);
        assert_table_len!(tables, jewel, JEWEL);
        assert_table_len!(tables, main_a, MAIN_A);
        assert_table_len!(tables, main_b, MAIN_B);
        assert_table_len!(tables, main_c, MAIN_C);
        assert_table_len!(tables, neck, NECK);
        assert_table_len!(tables, ranked_armour, RANKED_ARMOUR);
        assert_table_len!(tables, shields, SHIELDS);
        assert_table_len!(tables, torso_a, TORSO_A);
        assert_table_len!(tables, torso_b, TORSO_B);
        assert_table_len!(tables, torso_c, TORSO_C);
        assert_table_len!(tables, torso_d, TORSO_D);
        assert_table_len!(tables, torso_e, TORSO_E);
        assert_table_len!(tables, torso_f, TORSO_F);
        assert_table_len!(tables, torso_g, TORSO_G);
        assert_table_len!(tables, torso_h, TORSO_H);
        assert_table_len!(tables, torso_i, TORSO_I);
        assert_table_len!(tables, underwear, UNDERWEAR);
        assert_table_len!(tables, weapons_a, WEAPONS_A);
        assert_table_len!(tables, weapons_b, WEAPONS_B);
        assert_table_len!(tables, weapons_c, WEAPONS_C);
        assert_table_len!(tables, weapons_d, WEAPONS_D);
    }

    #[test]
    fn outfit_tables_pin_converted_rows() {
        let tables = synthetic_tables();
        let main = &tables.main_a[0];
        assert_eq!(main.id, ItemId(81));
        assert_eq!(main.weight, 50);
        assert_eq!(main.wanted, 0x100);
        assert_eq!(main.forbidden, 0);
        assert_eq!(main.rank, 2);
        assert_eq!(main.cost, 1);
        assert_eq!(main.bonus.floats[0], 1.0);

        let shield = &tables.shields[0];
        assert_eq!(shield.id, ItemId(80));
        assert_eq!(shield.weight, 10);
        assert_eq!(shield.rank, 2);
        assert_eq!(shield.cost, 1);

        let weapon = &tables.weapons_a[0];
        assert_eq!(weapon.id, ItemId(23));
        assert_eq!(weapon.weight, 0);
        assert_eq!(weapon.rank, 2);
        assert_eq!(weapon.cost, 1);

        let full_set = &tables.full_set[1];
        assert_eq!(full_set.id, ItemId(621));
        assert_eq!(full_set.weight, 10);
        assert_eq!(full_set.wanted, 0);
    }

    #[test]
    fn outfit_tables_without_facts_keep_the_raw_lengths() {
        let tables = OutfitTables::build(|_| None);
        assert_eq!(tables.body_pool.len(), data::BODY_POOL.len());
        assert_eq!(tables.weapons_a.len(), data::WEAPONS_A.len());
        assert_eq!(tables.body_pool[0].weight, 150);
        assert_eq!(tables.body_pool[0].rank, 0);
        assert_eq!(tables.body_pool[0].cost, 0);
        assert_eq!(tables.weapons_a[0].weight, 0);
        assert_eq!(tables.weapons_a[0].rank, 0);
    }

    #[test]
    fn generate_profile_level_zero_flags_zero_pins() {
        let tables = synthetic_tables();
        let character = OutfitCharacter::default();
        let outfit = generate_profile(
            &[0, 0, 0, 0, 0, 0, 0, 0],
            &mut Lcg::new(0),
            &tables,
            &character,
            unit_half,
        );
        assert_eq!(ids(&outfit), vec![249]);
        assert_eq!(outfit.hands, [None, None]);
        assert_eq!(outfit.points, 9);
        assert_eq!(outfit.mode, 0);
        assert_eq!(outfit.look.height, 0.5);
        assert_eq!(outfit.look.skin, 0.0);
        assert_eq!(outfit.look.condition_seed, 0.5);
        assert_eq!(outfit.look.preset, LOOK_PRESET_ROUGH);
    }

    #[test]
    fn generate_profile_level_five_flags_two_pins() {
        let tables = synthetic_tables();
        let character = OutfitCharacter::default();
        let outfit = generate_profile(
            &[2, 5, 0, 0, 0, 0, 0, 0],
            &mut Lcg::new(0),
            &tables,
            &character,
            unit_half,
        );
        assert_eq!(
            ids(&outfit),
            vec![420, 402, 1359, 251, 510, 398, 511, 391, 512, 513, 244]
        );
        assert_eq!(
            outfit.hands,
            [
                Some(HeldWeapon {
                    id: ItemId(420),
                    variant: 420
                }),
                None
            ]
        );
        assert_eq!(outfit.points, 69);
        assert_eq!(outfit.mode, 0);
        assert_eq!(outfit.look.height, 0.625);
        assert_eq!(outfit.look.skin, 0.0);
        assert_eq!(outfit.look.condition_seed, 2.5);
    }

    #[test]
    fn generate_by_level_three_pins() {
        let tables = synthetic_tables();
        let character = OutfitCharacter::default();
        let outfit =
            generate_by_level(3, 0.25, 0, &mut Lcg::new(0), &tables, &character, unit_half);
        assert_eq!(ids(&outfit), vec![210, 280, 249]);
        assert_eq!(
            outfit.hands,
            [
                Some(HeldWeapon {
                    id: ItemId(210),
                    variant: 210
                }),
                None
            ]
        );
        assert_eq!(outfit.points, 37);
        assert!(close(outfit.look.height, 0.825));
        assert!(close(outfit.look.skin, 0.205));
        assert!(close(outfit.look.condition_seed, 1.7));
    }

    #[test]
    fn generate_by_level_clamps_the_height_before_the_skin() {
        let tables = synthetic_tables();
        let character = OutfitCharacter::default();
        let outfit = generate_by_level(4, 0.7, 0, &mut Lcg::new(0), &tables, &character, unit_half);
        assert_eq!(outfit.look.height, 1.0);
        assert!(close(outfit.look.skin, 0.1));
    }

    #[test]
    fn generate_by_level_five_pins_the_fixed_equips() {
        let tables = synthetic_tables();
        let character = OutfitCharacter::default();
        let outfit =
            generate_by_level(5, 0.25, 0, &mut Lcg::new(0), &tables, &character, unit_half);
        assert_eq!(
            ids(&outfit),
            vec![420, 402, 1359, 285, 510, 398, 511, 391, 512, 513]
        );
        assert_eq!(
            outfit.hands,
            [
                Some(HeldWeapon {
                    id: ItemId(420),
                    variant: 420
                }),
                None
            ]
        );
        assert_eq!(outfit.points, 70);
        assert!(ids(&outfit).contains(&ITEM_402.0));
        assert!(ids(&outfit).contains(&ITEM_1359.0));
        assert!(ids(&outfit).contains(&ITEM_285.0));
    }

    #[test]
    fn profile_flag_four_pins_the_fixed_items() {
        let tables = synthetic_tables();
        let character = OutfitCharacter::default();
        let outfit = generate_profile(
            &[4, 5, 0, 0, 0, 0, 0, 0],
            &mut Lcg::new(0),
            &tables,
            &character,
            unit_half,
        );
        assert_eq!(ids(&outfit), vec![402, 1359, 251, 439, 436, 85, 435]);
        assert_eq!(outfit.hands, [None, None]);
        assert_eq!(outfit.points, 73);
        assert_eq!(outfit.mode, 0);
    }

    #[test]
    fn premade_and_random_early_branches_differ() {
        let tables = synthetic_tables();
        let premade = OutfitCharacter {
            skill_word: PREMADE_SKILL_BIT,
            ..OutfitCharacter::default()
        };
        let random = OutfitCharacter::default();
        let profile = [0, 3, 0, 0, 0, 0, 0, 0];
        let with_skill = generate_profile(&profile, &mut Lcg::new(0), &tables, &premade, unit_half);
        let without_skill =
            generate_profile(&profile, &mut Lcg::new(0), &tables, &random, unit_half);
        assert_eq!(
            with_skill.hands,
            [
                Some(HeldWeapon {
                    id: ItemId(210),
                    variant: 210
                }),
                Some(HeldWeapon {
                    id: ItemId(65),
                    variant: 65
                })
            ]
        );
        assert_eq!(ids(&with_skill), vec![210, 65, 149, 249]);
        assert_eq!(
            without_skill.hands,
            [
                Some(HeldWeapon {
                    id: ItemId(210),
                    variant: 210
                }),
                None
            ]
        );
        assert_eq!(ids(&without_skill), vec![210, 280, 249]);
    }
    #[test]
    fn rich_profile_branch_puts_the_shield_in_hand_one() {
        let tables = synthetic_tables();
        let character = OutfitCharacter {
            experience: 65_535,
            ..OutfitCharacter::default()
        };
        let outfit = generate_profile(
            &[0, 3, 0, 0, 0, 0, 0, 0],
            &mut Lcg::new(0),
            &tables,
            &character,
            unit_half,
        );
        assert!(outfit.hands[0].is_some());
        let shield = outfit.hands[1].expect("the rich branch picks a shield");
        assert!(tables.shields.iter().any(|row| row.id == shield.id));
        assert_eq!(outfit.look.preset, LOOK_PRESET_ARMED);
    }

    #[test]
    fn level_five_family_recomputes_the_skin_value() {
        let tables = synthetic_tables();
        let character = OutfitCharacter::default();
        let outfit =
            generate_by_level(5, 0.25, 0, &mut Lcg::new(0), &tables, &character, unit_half);
        assert!(close(outfit.look.height, 0.875));
        assert!(close(outfit.look.skin, 0.2125));
        assert!(close(outfit.look.condition_seed, 2.5));
        assert_eq!(outfit.condition, 54);
    }

    #[test]
    fn flag_eight_sets_the_generation_mode_and_description_pass() {
        let tables = synthetic_tables();
        let character = OutfitCharacter::default();
        let outfit = generate_profile(
            &[0x0a, 5, 0, 0, 0, 0, 0, 0],
            &mut Lcg::new(0),
            &tables,
            &character,
            unit_half,
        );
        assert_eq!(outfit.mode, 3);
        let direct = outfit
            .items
            .iter()
            .find(|item| item.id == ITEM_402)
            .expect("the fixed item is equipped");
        assert_eq!(direct.source, GenerationMode(2));
    }
}
