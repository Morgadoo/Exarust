//! Story experience: discovery points and their slow conversion.
//!
//! Exploring a new map cell, picking up a new kind of item and killing a
//! character earn discovery points. Each point goes once to an account-wide
//! pool and four times to the player and to every companion. Once a second
//! the points convert. A character pays `4 + skill levels` per point, at most
//! one point per 50 ms of that cost since it last converted, and each point
//! adds 2 to the skill being trained. Reaching the next 5000 learns that
//! level's technique. A companion who levels also gains 10 combat skill and
//! 10 courage, up to 100, and picks its next technique from a fixed list
//! when none is chosen. The player's converted cost also feeds the power in
//! training, a level for each four. The account pool pays
//! `(global / 5000 + 1) × 4` per point and adds 2 global experience a point,
//! stopping at 25000.
//!
//! Map cells are 600 units square, as the level grid counts them, the
//! position rounded down to its cell. A step looks at the 5 × 5 cells around the one stood on, inside the grid's
//! border, and a cell whose flag `0x80000` is set in an open sector earns 10
//! points once. Each level keeps its own bitmap, saved with the item kinds
//! already seen.
//!
//! Evidence: REA on the story experience unit (the hooks it installs, the
//! conversions, the discovery data in the save, the companions' list) and
//! on the arena's lesson, which adds a lump to the trained skill.

use exa_formats::characters::{Power, SKILLS, Skill, TECHNIQUE_SLOTS};
use exa_formats::weapon_stats::StatBytes;

/// Skill experience in one level.
pub const LEVEL_STEP: u16 = 5000;
/// A skill at this level trains no further technique.
pub const TRAINED: u32 = 5;
/// The account-wide experience stops here.
pub const GLOBAL_CAP: u32 = 25_000;
/// A discovery point is worth this much on a character.
pub const SCALE: i32 = 4;
/// Milliseconds of game clock per unit of a point's cost.
pub const MS_PER_COST: u32 = 50;
/// A character's cost per point before its skill levels.
pub const BASE_COST: i32 = 4;
/// Points a discovery of each rarity tier (or killed rank) earns.
pub const RARITY_POINTS: [i32; 8] = [10, 20, 50, 100, 200, 500, 1000, 2000];
/// Points a newly explored map cell earns.
pub const CELL_POINTS: i32 = 10;
/// One map cell per 600 units: the float 1/600.
pub const CELL_SCALE_BITS: u32 = 0x3ada_740e;
/// Cells looked at on each side of the one stood on.
pub const REACH: i32 = 2;
/// A cell flag that makes it count.
pub const DISCOVERABLE: u32 = 0x8_0000;
/// The game mode in which new item kinds earn points.
pub const STORY_MODE: i32 = 4;
/// Bit of a power entry being trained, and its level bits.
pub const POWER_IN_TRAINING: u32 = 0x2000;
pub const POWER_LEVEL: u32 = 0xfff;
/// A power whose level would reach 4096 is written as this word.
pub const POWER_FULL: u32 = 0x1fff;
/// What a companion's combat skill and courage gain at a level.
pub const COMPANION_RAISE: f32 = 10.0;
pub const ATTRIBUTE_CAP: f32 = 100.0;
/// A companion converts only while its body state, masked by this, is
/// [`ACTIVE`].
pub const STATE_MASK: u32 = 0xa;
pub const ACTIVE: u32 = 2;
/// The settings key under which the account-wide experience is stored.
pub const GLOBAL_XP_KEY: [u32; 2] = [1, 0x6431_ac04];
/// A companion with no technique chosen takes, at each total level 0..14,
/// this skill and technique id.
pub const AUTO_TECHNIQUES: [(usize, u8); 15] = [
    (2, 1),
    (2, 2),
    (0, 2),
    (0, 3),
    (2, 3),
    (0, 1),
    (0, 4),
    (2, 5),
    (2, 4),
    (3, 6),
    (3, 1),
    (3, 4),
    (3, 2),
    (3, 3),
    (0, 8),
];
/// The skill whose rise also redoes the wearer's armour.
pub const ARMOUR_SKILL: usize = 2;

/// A skill word's level.
pub fn level(points: u16) -> u32 {
    exa_ai::estimate::skill_level(points)
}

/// The six skills' levels summed.
pub fn total_levels(skills: &[Skill; SKILLS]) -> i32 {
    exa_ai::estimate::total_levels(skills)
}

/// The skill being trained: the first under level 5 whose technique at its
/// level is chosen, with the level it trains toward.
pub fn training_skill(skills: &[Skill; SKILLS]) -> Option<(usize, u32)> {
    skills.iter().enumerate().find_map(|(index, skill)| {
        let level = level(skill.points);
        (level < TRAINED && skill.techniques[level as usize] != 0).then_some((index, level + 1))
    })
}

/// Add `gain` to a skill's word. A word that reaches a new level snaps to
/// it, and the rise is reported.
fn raise(skill: &mut Skill, gain: u16) -> bool {
    let before = level(skill.points);
    skill.points = skill.points.wrapping_add(gain);
    let after = level(skill.points);
    if before < after {
        skill.points = (after as u16).wrapping_mul(LEVEL_STEP);
        true
    } else {
        false
    }
}

/// A lump of skill experience, as the arena's lesson gives it: `amount`
/// divided by one more than the six levels, at most 5000, goes to the
/// trained skill. A rank above `amount`, or no trained skill, gives nothing.
/// The return says whether the skill reached a new level.
pub fn add_skill_lump(skills: &mut [Skill; SKILLS], amount: i32) -> bool {
    let rank = 1 + total_levels(skills);
    if rank > amount {
        return false;
    }
    let Some((skill, _)) = training_skill(skills) else {
        return false;
    };
    let grant = (amount / rank).min(i32::from(LEVEL_STEP));
    raise(&mut skills[skill], grant as u16)
}

/// A companion with no technique in training takes the next one from
/// [`AUTO_TECHNIQUES`] by its total level, while that skill is under 5.
pub fn auto_technique(skills: &mut [Skill; SKILLS]) {
    let total = total_levels(skills);
    let Some(&(skill, technique)) = usize::try_from(total)
        .ok()
        .and_then(|t| AUTO_TECHNIQUES.get(t))
    else {
        return;
    };
    let level = level(skills[skill].points) as usize;
    if level < TRAINED as usize && level < TECHNIQUE_SLOTS {
        skills[skill].techniques[level] = technique;
    }
}

/// Points a rarity tier earns. A tier past the table earns the last entry.
pub fn rarity_points(tier: u8) -> i32 {
    RARITY_POINTS
        .get(usize::from(tier))
        .copied()
        .unwrap_or(RARITY_POINTS[7])
}

/// What global experience is worth to a new character: the first 5000 at
/// a quarter of 4, the next at 5/4, and so on, rounded down at the end.
pub fn global_xp_worth(xp: u32) -> u32 {
    let mut left = xp;
    let mut sum = 0u32;
    let mut weight = 4u32;
    while left != 0 {
        let chunk = left.min(u32::from(LEVEL_STEP));
        sum = chunk.wrapping_mul(weight).wrapping_add(sum);
        weight += 1;
        left -= chunk;
    }
    sum >> 2
}

/// The global experience a worth costs: the inverse of [`global_xp_worth`].
pub fn global_xp_for_worth(worth: i32) -> i32 {
    let mut left = (worth as u32).wrapping_shl(2);
    let mut sum = 0i32;
    let mut weight = 4i64;
    while left != 0 {
        let cap = weight * i64::from(LEVEL_STEP);
        let chunk = if cap < i64::from(left) {
            cap as u32
        } else {
            left
        };
        sum = ((i64::from(chunk) / weight) as i32).wrapping_add(sum);
        weight += 1;
        left = left.wrapping_sub(chunk);
    }
    sum
}

/// The stored account-wide experience from its setting value: the first
/// four bytes, little-endian. An empty value keeps the current number.
pub fn global_xp_from_setting(value: &[u8]) -> Option<u32> {
    if value.is_empty() {
        return None;
    }
    let mut word = [0u8; 4];
    for (to, from) in word.iter_mut().zip(value) {
        *to = *from;
    }
    Some(u32::from_le_bytes(word))
}

/// A combatant's fighting estimate. Two plus 0.4 a skill level, plus 0.02
/// a point of combat skill, plus with a weapon 0.02 of its impact, slash,
/// crush, pierce and thrust hundredths times its reach. A body scale above
/// 1.3 adds 2; otherwise the estimate is quartered.
pub fn fighting_estimate(
    skills: &[Skill; SKILLS],
    combat_skill: f32,
    weapon: Option<(&StatBytes, f32)>,
    scale: f32,
) -> f32 {
    exa_ai::estimate::fighting_estimate(skills, combat_skill, weapon, scale)
}

/// Someone who learns from discovery: the player or a companion.
#[derive(Debug, Clone, PartialEq)]
pub struct Learner {
    /// Discovery points waiting to convert, four to a point.
    pub experience: u32,
    pub skills: [Skill; SKILLS],
    /// 0 to 100.
    pub combat_skill: f32,
    /// The second personality trait, −100 to 100 [guess: courage].
    pub courage: f32,
    /// When a companion last converted, ms on the game clock; 0 until it
    /// joins.
    pub clock: u32,
    /// A thaumaturge's powers. The one flagged [`POWER_IN_TRAINING`] levels.
    pub powers: Vec<Power>,
    /// Converted cost not yet worth a power level.
    pub power_progress: u32,
    /// The body's state word; see [`STATE_MASK`].
    pub body_state: u32,
}

impl Default for Learner {
    fn default() -> Self {
        Self {
            experience: 0,
            skills: [Skill::default(); SKILLS],
            combat_skill: 0.0,
            courage: 0.0,
            clock: 0,
            powers: Vec::new(),
            power_progress: 0,
            body_state: ACTIVE,
        }
    }
}

impl Learner {
    /// A learner from a character record: its experience, skills, combat
    /// skill, second trait and powers.
    pub fn from_record(record: &exa_formats::characters::CharacterRecord) -> Self {
        Self {
            experience: record.experience as u32,
            skills: record.skills,
            combat_skill: record.combat_skill,
            courage: record.traits[1],
            powers: record
                .powers
                .as_ref()
                .map(|p| p.known.clone())
                .unwrap_or_default(),
            ..Self::default()
        }
    }

    fn earn(&mut self, points: i32) {
        self.experience = self
            .experience
            .wrapping_add(points.wrapping_mul(SCALE) as u32);
    }
}

/// What one conversion did.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Conversion {
    /// Points converted.
    pub points: i32,
    /// The skill that reached a new level. The body's technique bits are
    /// read again then, and a rise in [`ARMOUR_SKILL`] redoes the armour.
    pub raised: Option<usize>,
    /// A companion's new AI skill and courage factors, its two attributes
    /// times 0.01, after a level.
    pub ai_factors: Option<[f32; 2]>,
}

/// One level's explored cells.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocaleMap {
    /// The level's name, 16 bytes, zero padded.
    pub name: [u8; 16],
    pub width: i32,
    pub height: i32,
    /// `width / 32` words a row (rounded toward zero), `height` rows.
    pub bits: Vec<u32>,
}

impl LocaleMap {
    /// Words a bitmap of this size holds.
    pub fn words(width: i32, height: i32) -> usize {
        usize::try_from((width / 32).wrapping_mul(height)).unwrap_or(0)
    }
}

/// The level grid exploration reads.
pub trait ExploreGrid {
    /// Whether the level tracks discovery at all.
    fn tracks(&self) -> bool;
    /// Columns and rows of the grid the position is placed on.
    fn size(&self) -> (i32, i32);
    /// The sector word of a cell (sector index + 1, 0 for none), or `None`
    /// when the level has no sectors.
    fn sector_word(&self, cell: u32) -> Option<i32>;
    /// Whether a sector is open (its flag bit 0).
    fn sector_open(&self, sector: i32) -> bool;
    /// Columns of the cell grid whose flags are read; its rows of 32 cells
    /// stride the bitmap.
    fn cell_width(&self) -> i32;
    fn cell_flags(&self, cell: u32) -> u32;
}

/// The discovery state: the account pool, the clocks, the item kinds seen
/// and each level's explored cells.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Experience {
    /// Account-wide experience, saved in the settings under [`GLOBAL_XP_KEY`].
    pub global: u32,
    /// Discovery points waiting for the account.
    pub pool: u32,
    /// When the account pool and the player last converted, ms.
    pub global_clock: u32,
    pub player_clock: u32,
    /// Item kinds already discovered, in the order found.
    pub seen: Vec<i32>,
    pub maps: Vec<LocaleMap>,
    /// The level being played, an index into `maps`.
    pub current: Option<usize>,
}

/// What a once-a-second tick did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tick {
    pub cells: u32,
    pub player: Conversion,
    pub companions: Vec<Conversion>,
}

/// Why discovery data did not load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryError {
    /// The bytes end inside a field.
    Truncated,
    /// A count is negative.
    Negative,
}

/// Discovery data read from a save.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discovery {
    pub seen: Vec<i32>,
    pub maps: Vec<LocaleMap>,
    /// A level entry with an empty name ended the list early: the maps
    /// before it are kept and the player is told the data was recovered.
    pub recovered: bool,
}

/// Round down, as the engine's cell helper does (it converts with the
/// rounding mode set toward minus infinity); out of range gives the
/// integer indefinite.
fn round_down(value: f32) -> i32 {
    let r = value.floor();
    if r.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&r) {
        i32::MIN
    } else {
        r as i32
    }
}

/// Truncate a double to a 64-bit integer and keep its low 32 bits; out of
/// range gives 0, the low half of the integer indefinite.
fn truncate(value: f64) -> i32 {
    if value.is_nan() || !(-9.223_372_036_854_776e18..9.223_372_036_854_776e18).contains(&value) {
        0
    } else {
        (value as i64) as i32
    }
}

/// Whole points due since `clock`: the elapsed ms over 50 times the cost,
/// truncated.
fn due(now: u32, clock: u32, cost: i32) -> i32 {
    let elapsed = (i64::from(now) - i64::from(clock)) as f64;
    let per = (i64::from(MS_PER_COST) * i64::from(cost)) as f64;
    truncate(elapsed / per)
}

/// Add a companion. One already listed is refused. A companion that has
/// never converted starts its clock now.
pub fn join<K: PartialEq>(list: &mut Vec<K>, key: K, learner: &mut Learner, now: u32) -> bool {
    if list.contains(&key) {
        return false;
    }
    list.push(key);
    if learner.clock == 0 {
        learner.clock = now;
    }
    true
}

/// Remove a companion: the last one takes its place.
pub fn leave<K: PartialEq>(list: &mut Vec<K>, key: &K) -> bool {
    match list.iter().position(|k| k == key) {
        Some(index) => {
            list.swap_remove(index);
            true
        }
        None => false,
    }
}

impl Experience {
    /// A new story: no item kinds seen and no level explored. The account
    /// pool and the clocks are kept.
    pub fn begin(&mut self) {
        self.seen.clear();
        self.maps.clear();
        self.current = None;
    }

    fn award(&mut self, player: &mut Learner, companions: &mut [Learner], points: i32) {
        self.pool = self.pool.wrapping_add(points as u32);
        player.earn(points);
        for companion in companions {
            companion.earn(points);
        }
    }

    /// An item picked up. In story mode, a kind not seen before earns its
    /// rarity tier's points and is remembered; a kind whose record has no
    /// model is neither. Returns the points.
    pub fn discover_item(
        &mut self,
        mode: i32,
        player: Option<&mut Learner>,
        companions: &mut [Learner],
        id: i32,
        tier: impl FnOnce(i32) -> Option<u8>,
    ) -> i32 {
        let Some(player) = player else {
            return 0;
        };
        if mode != STORY_MODE || self.seen.contains(&id) {
            return 0;
        }
        let Some(tier) = tier(id) else {
            return 0;
        };
        let points = rarity_points(tier);
        self.award(player, companions, points);
        self.seen.push(id);
        points
    }

    /// A character killed: its rank, bits 12 to 14 of `rank_word`, earns
    /// that tier's points.
    pub fn discover_kill(
        &mut self,
        player: &mut Learner,
        companions: &mut [Learner],
        rank_word: u32,
    ) -> i32 {
        let points = rarity_points(((rank_word & 0x7000) >> 12) as u8);
        self.award(player, companions, points);
        points
    }

    /// Enter a level: its map is found by the full 16-byte name, or added
    /// empty at the grid's size.
    pub fn enter_level(&mut self, name: [u8; 16], width: i32, height: i32) {
        let index = match self.maps.iter().position(|m| m.name == name) {
            Some(index) => index,
            None => {
                self.maps.push(LocaleMap {
                    name,
                    width,
                    height,
                    bits: vec![0; LocaleMap::words(width, height)],
                });
                self.maps.len() - 1
            }
        };
        self.current = Some(index);
    }

    /// Explore around a position, in the original's units. Returns the
    /// cells newly found.
    pub fn explore(
        &mut self,
        grid: &impl ExploreGrid,
        x: f32,
        z: f32,
        player: &mut Learner,
        companions: &mut [Learner],
    ) -> u32 {
        if !grid.tracks() {
            return 0;
        }
        let (width, height) = grid.size();
        let scale = f32::from_bits(CELL_SCALE_BITS);
        let cx = round_down(x * scale).wrapping_add(((width as u32) >> 1) as i32);
        let cy = round_down(z * scale).wrapping_add(((height as u32) >> 1) as i32);
        let cell = cy.wrapping_mul(width).wrapping_add(cx);
        if cell < 0 || i64::from(width) * i64::from(height) <= i64::from(cell) {
            return 0;
        }
        let Some(word) = grid.sector_word(cell as u32) else {
            return 0;
        };
        if word.wrapping_sub(1) < 0 {
            return 0;
        }
        let x0 = cx.wrapping_sub(REACH).max(1);
        let x1 = if i64::from(width) - 2 < i64::from(cx.wrapping_add(REACH)) {
            width.wrapping_sub(2)
        } else {
            cx.wrapping_add(REACH)
        };
        let y0 = cy.wrapping_sub(REACH).max(1);
        let y1 = if i64::from(height) - 2 < i64::from(cy.wrapping_add(REACH)) {
            height.wrapping_sub(2)
        } else {
            cy.wrapping_add(REACH)
        };
        let Some(current) = self.current else {
            return 0;
        };
        let cell_width = grid.cell_width();
        let stride = cell_width / 32;
        let mut found = 0;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let word = stride
                    .wrapping_mul(y)
                    .wrapping_add(((x as u32) >> 5) as i32) as u32;
                let bit = 1u32 << (x & 31);
                let Some(&bits) = self.maps[current].bits.get(word as usize) else {
                    continue;
                };
                if bits & bit != 0 {
                    continue;
                }
                let at = y.wrapping_mul(cell_width).wrapping_add(x) as u32;
                if grid.cell_flags(at) & DISCOVERABLE == 0 {
                    continue;
                }
                let open = grid
                    .sector_word(at)
                    .is_some_and(|w| w.wrapping_sub(1) >= 0 && grid.sector_open(w.wrapping_sub(1)));
                if !open {
                    continue;
                }
                self.maps[current].bits[word as usize] |= bit;
                self.award(player, companions, CELL_POINTS);
                found += 1;
            }
        }
        found
    }

    /// Convert the account pool.
    pub fn convert_global(&mut self, now: u32) {
        let cost = (self.global / u32::from(LEVEL_STEP) + 1).wrapping_mul(4) as i32;
        if i64::from(self.pool) < i64::from(cost) {
            self.global_clock = now;
            return;
        }
        let mut points = if self.global < GLOBAL_CAP {
            let points = due(now, self.global_clock, cost);
            self.global_clock = (cost as u32)
                .wrapping_mul(MS_PER_COST)
                .wrapping_mul(points as u32)
                .wrapping_add(self.global_clock);
            points
        } else {
            self.global = GLOBAL_CAP;
            self.pool = 0;
            0
        };
        if points > 0 {
            let affordable = i64::from(self.pool) / i64::from(cost);
            if affordable < i64::from(points) {
                points = affordable as i32;
            }
            self.global = self.global.wrapping_add((points as u32).wrapping_shl(1));
            self.pool = self
                .pool
                .wrapping_sub((points as u32).wrapping_mul(cost as u32));
        }
    }

    /// Convert the player's points into the trained skill and power.
    pub fn convert_player(&mut self, now: u32, player: &mut Learner) -> Conversion {
        let cost = BASE_COST + total_levels(&player.skills);
        if i64::from(player.experience) < i64::from(cost) {
            self.player_clock = now;
            return Conversion::default();
        }
        let mut points = due(now, self.player_clock, cost);
        if points <= 0 {
            return Conversion::default();
        }
        let affordable = i64::from(player.experience) / i64::from(cost);
        if affordable < i64::from(points) {
            points = affordable as i32;
        }
        let paid = (points as u32).wrapping_mul(cost as u32);
        player.experience = player.experience.wrapping_sub(paid);
        if let Some(power) = player
            .powers
            .iter_mut()
            .find(|p| p.flags & POWER_IN_TRAINING != 0)
        {
            player.power_progress = paid.wrapping_add(player.power_progress);
            let levels = (player.power_progress / SCALE as u32) as i32;
            if levels > 0 {
                player.power_progress = player
                    .power_progress
                    .wrapping_sub((SCALE as u32).wrapping_mul(levels as u32));
                let level = levels.wrapping_add((power.flags & POWER_LEVEL) as i32);
                power.flags = if level < 0x1000 {
                    power.flags & !POWER_LEVEL | level as u32
                } else {
                    POWER_FULL
                };
            }
        }
        self.player_clock = (cost as u32)
            .wrapping_mul(MS_PER_COST)
            .wrapping_mul(points as u32)
            .wrapping_add(self.player_clock);
        let mut raised = None;
        if let Some((skill, _)) = training_skill(&player.skills)
            && raise(
                &mut player.skills[skill],
                (points as u32).wrapping_shl(1) as u16,
            )
        {
            raised = Some(skill);
        }
        Conversion {
            points,
            raised,
            ai_factors: None,
        }
    }

    /// Run the once-a-second tick: explore around the player, convert the
    /// account pool and the player, then each companion in list order.
    pub fn tick(
        &mut self,
        now: u32,
        grid: &impl ExploreGrid,
        x: f32,
        z: f32,
        player: &mut Learner,
        companions: &mut [Learner],
    ) -> Tick {
        let cells = self.explore(grid, x, z, player, companions);
        self.convert_global(now);
        let player = self.convert_player(now, player);
        let companions = companions
            .iter_mut()
            .map(|c| convert_companion(now, c))
            .collect();
        Tick {
            cells,
            player,
            companions,
        }
    }

    /// The discovery chunk of a save: the item kinds seen (a count, then the
    /// ids when there are any), then the levels (a count, then each name,
    /// width, height and bitmap).
    pub fn write_discovery(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend((self.seen.len() as i32).to_le_bytes());
        for id in &self.seen {
            out.extend(id.to_le_bytes());
        }
        out.extend((self.maps.len() as i32).to_le_bytes());
        for map in &self.maps {
            out.extend(map.name);
            out.extend(map.width.to_le_bytes());
            out.extend(map.height.to_le_bytes());
            for index in 0..LocaleMap::words(map.width, map.height) {
                out.extend(map.bits.get(index).copied().unwrap_or(0).to_le_bytes());
            }
        }
        out
    }

    /// Restore the item kinds and the maps from [`read_discovery`].
    pub fn restore(&mut self, discovery: Discovery) {
        self.seen = discovery.seen;
        self.maps = discovery.maps;
        self.current = None;
    }
}

/// Convert a companion's points. A companion out of its active state does
/// nothing. With no technique in training it picks one instead of rising.
pub fn convert_companion(now: u32, companion: &mut Learner) -> Conversion {
    if companion.body_state & STATE_MASK != ACTIVE {
        return Conversion::default();
    }
    let cost = total_levels(&companion.skills) + BASE_COST;
    if i64::from(companion.experience) < i64::from(cost) {
        companion.clock = now;
        return Conversion::default();
    }
    let mut points = due(now, companion.clock, cost);
    if points <= 0 {
        return Conversion::default();
    }
    let affordable = i64::from(companion.experience) / i64::from(cost);
    if affordable < i64::from(points) {
        points = affordable as i32;
    }
    companion.experience = companion
        .experience
        .wrapping_sub((points as u32).wrapping_mul(cost as u32));
    companion.clock = (cost as u32)
        .wrapping_mul(MS_PER_COST)
        .wrapping_mul(points as u32)
        .wrapping_add(companion.clock);
    let mut out = Conversion {
        points,
        ..Conversion::default()
    };
    let Some((skill, _)) = training_skill(&companion.skills) else {
        auto_technique(&mut companion.skills);
        return out;
    };
    if raise(
        &mut companion.skills[skill],
        (points as u32).wrapping_shl(1) as u16,
    ) {
        out.raised = Some(skill);
        companion.combat_skill += COMPANION_RAISE;
        if companion.combat_skill > ATTRIBUTE_CAP {
            companion.combat_skill = ATTRIBUTE_CAP;
        }
        let skill_factor = (f64::from(companion.combat_skill) * 0.01) as f32;
        companion.courage += COMPANION_RAISE;
        if companion.courage > ATTRIBUTE_CAP {
            companion.courage = ATTRIBUTE_CAP;
        }
        let courage_factor = (f64::from(companion.courage) * 0.01) as f32;
        out.ai_factors = Some([skill_factor, courage_factor]);
        auto_technique(&mut companion.skills);
    }
    out
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], DiscoveryError> {
        let end = self.at.checked_add(n).ok_or(DiscoveryError::Truncated)?;
        let slice = self
            .bytes
            .get(self.at..end)
            .ok_or(DiscoveryError::Truncated)?;
        self.at = end;
        Ok(slice)
    }

    fn i32(&mut self) -> Result<i32, DiscoveryError> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn count(&mut self) -> Result<usize, DiscoveryError> {
        usize::try_from(self.i32()?).map_err(|_| DiscoveryError::Negative)
    }
}

/// Read a discovery chunk. A level entry whose name starts with a zero byte
/// ends the list there and marks the data recovered.
pub fn read_discovery(bytes: &[u8]) -> Result<Discovery, DiscoveryError> {
    let mut r = Reader { bytes, at: 0 };
    let seen_count = r.count()?;
    let mut seen = Vec::with_capacity(seen_count.min(bytes.len() / 4));
    for _ in 0..seen_count {
        seen.push(r.i32()?);
    }
    let map_count = r.count()?;
    let mut maps = Vec::new();
    for _ in 0..map_count {
        let mut name = [0u8; 16];
        name.copy_from_slice(r.take(16)?);
        if name[0] == 0 {
            return Ok(Discovery {
                seen,
                maps,
                recovered: true,
            });
        }
        let width = r.i32()?;
        let height = r.i32()?;
        let words = LocaleMap::words(width, height);
        let raw = r.take(words.checked_mul(4).ok_or(DiscoveryError::Truncated)?)?;
        let bits = raw
            .as_chunks::<4>()
            .0
            .iter()
            .map(|w| u32::from_le_bytes(*w))
            .collect();
        maps.push(LocaleMap {
            name,
            width,
            height,
            bits,
        });
    }
    Ok(Discovery {
        seen,
        maps,
        recovered: false,
    })
}

#[cfg(test)]
#[path = "experience_test.rs"]
mod tests;
