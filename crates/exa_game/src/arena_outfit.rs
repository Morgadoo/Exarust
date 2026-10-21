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

/// Number of bonus words and bonus floats an item carries.
pub const BONUS_SLOTS: usize = 20;

/// The pool refuses to grow past this many candidates. The original reports
/// an error when the count would pass it; here extra additions are ignored.
pub const MAX_CANDIDATES: usize = 128;

/// Experience needed for one profession rank.
pub const EXPERIENCE_PER_RANK: u32 = 5_000;

/// The profession grid is three rows of eight columns.
pub const PROFESSION_ROWS: usize = 3;
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
    /// The apparel class read from the item's apparel data.
    pub apparel_class: u32,
    /// The apparel's point value read from the item's apparel data.
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

/// The smooth unit draw: fold a uniform value around its middle so draws
/// cluster near zero and one.
///
/// Above the middle the deviation is squared and added to a half; below it
/// the deviation is squared and taken away from a half. A NaN compares false
/// and takes the lower branch, staying NaN. All arithmetic is single
/// precision in the original's order.
#[must_use]
pub fn smooth_unit(unit: f32) -> f32 {
    if unit >= 0.5 {
        let deviation = (unit - 0.5) * 2.0;
        deviation * deviation * 0.5 + 0.5
    } else {
        let deviation = (0.5 - unit) * 2.0;
        0.5 - deviation * deviation * 0.5
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

/// Pick from a table inside the rank window ending at the limit.
///
/// The window weights are summed, one stepped draw is taken, and the first
/// cumulative weight past the threshold wins. The result names the id and
/// the cost the caller subtracts from the character's points.
pub fn pick_in_rank_window(lcg: &mut Lcg, entries: &[PoolEntry], limit: i32) -> Option<RankPick> {
    let weights: Vec<i32> = entries
        .iter()
        .map(|entry| rank_window_weight(entry.weight, entry.rank, limit).unwrap_or(0))
        .collect();
    let index = weighted_pick(lcg, &weights)?;
    Some(RankPick {
        id: entries[index].id,
        cost: entries[index].cost,
    })
}

/// Pick from a masked table and equip the winner.
///
/// Entries are refused when the character's bonus mask shares a forbidden
/// bit, when the wanted mask shares no bit, or when the rank exceeds the
/// limit. In max-rank mode the highest eligible rank is found first and only
/// entries at it receive weight. The winner is passed to the equip callback
/// unless its id is zero.
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
            .map(|entry| entry.rank)
            .max()
            .unwrap_or(0)
    } else {
        i32::MIN
    };
    let weights: Vec<i32> = entries
        .iter()
        .map(|entry| {
            if mask_accepts(query, entry) && (!query.max_rank || entry.rank == highest) {
                entry.weight
            } else {
                0
            }
        })
        .collect();
    let index = weighted_pick(lcg, &weights)?;
    let entry = &entries[index];
    if entry.id.0 != 0 {
        equip(entry.id, &entry.bonus);
    }
    Some(entry.id)
}

/// Pick an entry that does not overlap anything already worn.
///
/// The first pass marks entries that name an item, pass the mask and rank
/// checks, exist in the item database and share no bonus word with the worn
/// set, and sums their weights. The candidate's cost is not checked here;
/// the pool picks that pay for an item check it when they add it. One stepped draw is taken, then a second pass walks only the
/// marked entries to the pick. When nothing is eligible or no row passes,
/// nothing is equipped.
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
        if entry.id.0 == 0 || !mask_accepts(query, entry) {
            continue;
        }
        let Some(facts) = lookup(entry.id) else {
            continue;
        };
        if worn.overlaps(&facts.bonus_words) {
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
    query.character_bonus & entry.forbidden == 0
        && query.wanted & entry.wanted != 0
        && entry.rank <= query.rank_limit
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
    (sum * 100.0).round_ties_even() as i32
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
/// The original writes one byte per rank into a row whose exact stride is
/// unclear from the decompilation; the twelve rank slots here are a
/// reconstruction that keeps the same order of writes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProfessionRow {
    /// Total experience in this row, capped by the original's twelve ranks.
    pub experience: u16,
    /// The one-based column gained at each rank index; zero means none.
    pub levels: [u8; PROFESSION_TRACKS],
}

/// The three profession rows of one character.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProfessionProgress {
    /// One row per profession track.
    pub rows: [ProfessionRow; PROFESSION_ROWS],
}

/// The two profession weight grids and their prerequisite links.
///
/// The original keeps two grids and drifts between them as characters are
/// generated. A nonzero prerequisite link names a one-based column in the
/// same row that must already be marked. The default is a neutral grid with
/// every weight at one; it is not retail data. Which grid the original treats
/// as the rarer side is a guess: the caller can swap `weights_a` and
/// `weights_b` without changing any other rule.
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
            weights_a: [[1; PROFESSION_COLUMNS]; PROFESSION_ROWS],
            weights_b: [[1; PROFESSION_COLUMNS]; PROFESSION_ROWS],
            prereq_a: [[0; PROFESSION_COLUMNS]; PROFESSION_ROWS],
            prereq_b: [[0; PROFESSION_COLUMNS]; PROFESSION_ROWS],
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
pub fn random_npc<'a>(
    lcg: &mut Lcg,
    request: &NpcRequest<'a>,
    professions: &mut ProfessionCatalog,
    mut unit: impl FnMut() -> f32,
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
        let level_bonus = unit() * 20.0;
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

    let frame = (smooth_unit(unit()) - 0.5) * 0.5 + 0.5;
    let height = (1.0 - level as f32 * 0.05) * smooth_unit(unit());
    let weight = smooth_unit(unit()) * 2.0 - 1.0;
    let deviation = weight * 0.3 + (0.5 - frame) * 0.6 + (height - 0.5) * 0.8;
    let deviation = if -LOOK_EPSILON <= deviation {
        if LOOK_EPSILON < deviation {
            deviation.sqrt()
        } else {
            deviation
        }
    } else {
        -(-deviation).sqrt()
    };
    let muscle = (smooth_unit(unit()) * 0.5 + (deviation * 0.5 + 0.5) * 0.5) * 0.17 + 0.9;
    let draw = unit();
    let mut strength = draw * draw;
    if flags & 0x2_0000 != 0 {
        strength = (1.0 - strength) * 0.75 + strength;
    }
    let face_seed = (smooth_unit(unit()) * 65535.0).round_ties_even() as u16;
    let voice_seed = (smooth_unit(unit()) * 65535.0).round_ties_even() as u16;

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

    let skin_draw = unit();
    let muscle_share = strength * 0.6;
    let skin_value = request.warmth * 0.5 + muscle_share * muscle_share + skin_draw * skin_draw;
    let skin_tone = if skin_value <= 1.0 {
        (skin_value * 255.0).round_ties_even() as u8
    } else {
        255
    };
    let hair_draw = unit();
    let mut hair_tone = ((strength * 0.5 + hair_draw * 0.5) * 255.0).round_ties_even() as u8;
    let blend = request.warmth * 0.75;
    hair_tone = (blend * 255.0 + (1.0 - blend) * f32::from(hair_tone)).round_ties_even() as u8;

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
        assert_eq!(progress.rows[2].experience, 2_000);
        assert_eq!(progress.rows[2].levels[0], 5);
        assert_eq!(catalog.drift, 3);

        let mut empty = ProfessionCatalog::default();
        let mut nothing = ProfessionProgress::default();
        assert_eq!(
            grant_profession_levels(&mut Lcg::new(0), &mut empty, &mut nothing, 0),
            0
        );
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
        assert_eq!(profile.professions.rows[0].experience, 15_000);
        assert_eq!(profile.professions.rows[1].experience, 5_000);
        assert_eq!(profile.professions.rows[2].experience, 10_000);
        assert_eq!(profile.professions.rows[0].levels[..3], [5, 8, 4]);
        assert_eq!(profile.professions.rows[1].levels[0], 1);
        assert_eq!(profile.professions.rows[2].levels[..2], [5, 1]);
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
}
