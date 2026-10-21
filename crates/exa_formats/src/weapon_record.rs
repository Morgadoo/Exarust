//! The record a held weapon fights with: the original's 28-element weapon
//! record, built from an item's weapon section (`0x4000`), the weapon's
//! length along its parts and the character holding it, then refreshed on
//! every equip change. Also the built-in fist and claw sections, the shield
//! record (section `0x8000`), the weapon mass table and a character's
//! reaches. Rules, constants and evidence: `docs/formats/OTHER.md` (Weapon
//! record).
//!
//! Units follow the original: lengths in cm. Masses are the simulation's
//! own units.

use crate::races::RaceStats;
use crate::rfc::Node;
use crate::weapon_stats::{StatBytes, at};

/// Weapon flags (the section's first word).
pub mod flags {
    /// Kind: 1 bladed, 2 hafted head, 3 stick, 4 flail.
    pub const KIND: u32 = 0xF;
    pub const BLADED: u32 = 1;
    pub const HAFTED: u32 = 2;
    pub const STICK: u32 = 3;
    pub const FLAIL: u32 = 4;
    /// Hands: 0 unarmed, 1 one-handed, 2 two-handed, 3 polearm.
    pub const HANDS: u32 = 0x30;
    /// Two-handed or polearm.
    pub const TWO_HANDS: u32 = 0x20;
    /// The hands bits of a two-hander that is not a polearm.
    pub const TWO_HANDED: u32 = 0x20;
    /// Hand class bits: no bit at all for the human fists, `CLAWS` for a
    /// clawed race's.
    pub const HAND_CLASS: u32 = 0xF0;
    pub const CLAWS: u32 = 0x40;
    /// The greatsword's low 16 bits.
    pub const GREATSWORD: u32 = 0x521;
    /// Haft material for hits below the head: 1 wood, 2 metal, 3 other.
    pub const HAFT_SHIFT: u32 = 16;
    /// Sized for an ogre or a golem.
    pub const OGRE_SIZE: u32 = 0x0200_0000;
    /// Sized for a demon.
    pub const DEMON_SIZE: u32 = 0x0400_0000;
    /// The bits the record keeps.
    pub const KEPT: u32 = 0x0603_FFFF;
}

/// A weapon section: its flags word and its stat bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Section {
    pub flags: u32,
    pub stats: StatBytes,
}

impl Section {
    /// A human's fists: weightless, fully balanced, impact +0.33, no edge,
    /// striking at 0.4 of the hand's length; hit sound `BdyCol`.
    pub const FISTS: Section = Section {
        flags: 0,
        stats: StatBytes([
            0, 100, 160, 0, 1, 80, 80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0,
        ]),
    };
    /// The ghouls' claws: slash 60, pierce 40, impact +0.53, striking at
    /// 0.3 of the hand's length.
    pub const CLAWS: Section = Section {
        flags: flags::CLAWS,
        stats: StatBytes([
            0, 100, 180, 0, 1, 60, 60, 60, 0, 40, 0, 0, 0, 0, 0, 0, 0, 7, 0,
        ]),
    };
    /// The heavy claws of the large clawed races (Sir, Fael, Gobbler,
    /// Harvester): impact +0.73, striking at 0.75 of the hand's length.
    pub const HEAVY_CLAWS: Section = Section {
        flags: flags::CLAWS,
        stats: StatBytes([
            0, 100, 200, 0, 1, 150, 150, 60, 0, 40, 0, 0, 0, 0, 0, 0, 0, 7, 0,
        ]),
    };
}

/// The hit sound sets by a section's sound class (byte `0x41`).
pub const HIT_SOUNDS: [&str; 9] = [
    "WH_SwordA",
    "WH_MaceMA",
    "WH_AxeA",
    "WH_ClubMW",
    "WH_MaceMA",
    "WH_ClubMW",
    "WH_WoodA",
    "WH_ClawsL",
    "BdyCol",
];

/// How far a weapon reaches along its parts (cm).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Lengths {
    /// From the grip to the farthest point (W21): where along the weapon
    /// a contact lies is measured against it.
    pub length: f32,
    /// From the grip to the sweet spot (W22), 60 % of the way along the
    /// striking part of the last part.
    pub sweet_spot: f32,
}

/// One node of a weapon model's first-child chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChainNode {
    /// The top of its box along the weapon (+Y): centre plus half extent,
    /// cm.
    pub top: f32,
    /// It is a rigid body (its model node carries a collision body).
    pub rigid: bool,
}

impl Lengths {
    /// The lengths of an armed weapon whose own node, then its first child,
    /// that child's first child and so on, are `chain`, with the striking
    /// part from `start` to `end` of its last part. The length adds the
    /// tops of the weapon's node and of every rigid node down the chain;
    /// the sweet spot adds whole parts while the next one is rigid, then
    /// 60 % of the striking part of the part it stopped at.
    pub fn armed(chain: &[ChainNode], start: f32, end: f32) -> Self {
        let Some(first) = chain.first() else {
            return Self::default();
        };
        let length = first.top
            + chain[1..]
                .iter()
                .filter(|n| n.rigid)
                .map(|n| n.top)
                .sum::<f32>();
        let mut whole = 0.0f32;
        let mut last = first;
        for next in &chain[1..] {
            if !next.rigid {
                break;
            }
            whole += last.top;
            last = next;
        }
        let sweet = f64::from(start) + f64::from(end - start) * 0.6;
        let sweet_spot = (sweet * f64::from(last.top) + f64::from(whole)) as f32;
        Self { length, sweet_spot }
    }

    /// The lengths of a weapon of one rigid part whose box tops at `top`
    /// cm, such as an assembled factory weapon.
    pub fn one_part(top: f32, start: f32, end: f32) -> Self {
        Self::armed(&[ChainNode { top, rigid: true }], start, end)
    }

    /// The lengths of a fist: across the hand, from its origin to the far
    /// side of its box along x (cm), both the length and the sweet spot.
    pub fn hand(centre_x: f32, half_x: f32) -> Self {
        let length = half_x + centre_x.abs();
        Self {
            length,
            sweet_spot: length,
        }
    }
}

/// The first-child chain of node `root` (0-based) of a model's node list:
/// the node, its first child (the lowest-numbered node whose parent it is),
/// and so on. Lengths in cm.
pub fn chain(nodes: &[Node], root: usize) -> Vec<ChainNode> {
    let mut out = Vec::new();
    let mut at = Some(root);
    while let Some(i) = at {
        let Some(node) = nodes.get(i) else { break };
        out.push(ChainNode {
            top: (node.bounds_centre[1] + node.bounds_half_extents[1]) * CM_PER_M,
            rigid: node.has_rigid_body(),
        });
        // Node lists put a parent before its children.
        at = nodes
            .iter()
            .enumerate()
            .skip(i + 1)
            .find(|(_, n)| n.parent == Some(i + 1))
            .map(|(k, _)| k);
    }
    out
}

/// The fist lengths of a body model's hand node (cm).
pub fn hand_lengths(hand: &Node) -> Lengths {
    Lengths::hand(
        hand.bounds_centre[0] * CM_PER_M,
        hand.bounds_half_extents[0] * CM_PER_M,
    )
}

const CM_PER_M: f32 = 100.0;

/// What the character holding a weapon brings to its record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Holder {
    /// Armour encumbrance g, 0..=1: slows recovery and swings.
    pub encumbrance: f32,
    /// Character size s (1 for a human of average height): lengthens the
    /// reach and weighs the weapon.
    pub size: f32,
    /// Body scale: an unarmed swing's sound follows it.
    pub scale: f32,
    /// Share of the held weapons' weight a special worn item takes away,
    /// 0..=1.
    pub weightless: f32,
    /// The half-sword hold: a weapon handles heavier and less balanced
    /// ((2 − w)·w, 0.7·b). What sets it was not found.
    pub half_sword: bool,
}

impl Holder {
    /// A human of average height in no armour.
    pub const AVERAGE: Holder = Holder {
        encumbrance: 0.0,
        size: 1.0,
        scale: 1.0,
        weightless: 0.0,
        half_sword: false,
    };
}

impl Default for Holder {
    fn default() -> Self {
        Self::AVERAGE
    }
}

/// The record a held weapon fights with. Element numbers of the original's
/// record are given as W[i].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponRecord {
    /// The section's flags, kept bits only (W0).
    pub flags: u32,
    /// Type code of the striking side (W1).
    pub code: u8,
    /// The striking part's start, 0.7 below 0.15 and at most 0.99 (W3);
    /// its readers were not found.
    pub start_clamped: f32,
    /// Start and end of the striking part, fractions of the last part's
    /// length (W4, W5).
    pub start: f32,
    pub end: f32,
    /// Weight and balance, 0..1 (W6, W7).
    pub weight: f32,
    pub balance: f32,
    /// Weight and balance as held (W8, W9): changed by the half-sword hold
    /// and the weightless item.
    pub held_weight: f32,
    pub held_balance: f32,
    /// Impact factor (W10).
    pub impact: f32,
    /// Damage shares 0.5·(byte/100)^0.7 (W11–W14).
    pub slash: f32,
    pub crush: f32,
    pub pierce: f32,
    pub thrust: f32,
    /// Swing speed factor (W15): the attack clip clock's rate scale.
    pub swing_speed: f32,
    /// 0.8 − (w + 1 − b)/8 at the first build (W16): the body's turn
    /// while it attacks scales with it.
    pub handling: f32,
    /// Recoil (W17): a blocked striker recoils by v·W17; the recovery
    /// after a released swing and the attack cooldown scale with it.
    pub recoil: f32,
    /// Swing shaping (W18): how much the swing slows away from its middle.
    pub swing_shaping: f32,
    /// Stance blend (W19): attacks end into the stance over 2·W19 frames.
    pub blend: i32,
    /// Swinging arm's stiffness factor (W20).
    pub arm_stiffness: f32,
    /// Grip to farthest point and grip to sweet spot, cm (W21, W22).
    pub length: f32,
    pub sweet_spot: f32,
    /// Reach, cm (W23): 60·size + the sweet spot for a weapon; the
    /// section's end byte for a fist (unused: an unarmed character's reach
    /// is its race's).
    pub reach: f32,
    /// Second hand's place along the haft (W24): (byte 0x40 − 128)·0.00156
    /// + 1.
    pub second_hand: f32,
    /// Swing sound volume and pitch factors (W25, W26).
    pub swing_volume: f32,
    pub swing_pitch: f32,
    /// The AI's body turn through a swing, its amplitude (W27).
    pub swing_turn: f32,
    /// Hit sound class (section byte `0x41`, [`HIT_SOUNDS`]).
    pub hit_sound: u8,
}

/// A stat byte as a damage share: 0.5·(x/100)^0.7, in double precision.
fn share(x: u8) -> f32 {
    if x == 0 {
        0.0
    } else {
        ((f64::from(x) * 0.01).ln() * 0.7).exp() as f32 * 0.5
    }
}

impl WeaponRecord {
    /// The record of `section`, striking with its flipped side when
    /// `flipped`, with `lengths` (an armed weapon's along its parts, a
    /// fist's across the hand), held by `holder`: the original's build,
    /// then its holding pass.
    pub fn build(section: &Section, flipped: bool, lengths: Lengths, holder: &Holder) -> Self {
        let s = &section.stats;
        let side = if flipped { at::FLIP } else { 0 };
        let weight = f32::from(s.get(at::WEIGHT)) * 0.01;
        let balance = f32::from(s.get(at::BALANCE)) * 0.01;
        let flags = section.flags & flags::KEPT;
        let start = f32::from(s.get(at::START + side)) * 0.005;
        let start_clamped = if start < 0.15 {
            0.7
        } else if start > 0.99 {
            0.99
        } else {
            start
        };
        // The first build's held weight, which only the handling keeps.
        let first_weight = (1.0 - holder.weightless * 0.5) * weight;
        let base = if balance <= 1.0 {
            (1.0 - balance) * 0.6 + weight * 0.4 + 0.1
        } else {
            weight * 0.4 + 0.1
        };
        let impact = ((f64::from(s.get(at::IMPACT)) - 127.0) * 0.01 + f64::from(base)) as f32;
        let armed = flags & flags::HANDS != 0;
        let mut record = Self {
            flags,
            code: s.get(at::KIND + side),
            start_clamped,
            start,
            end: f32::from(s.get(at::END + side)) * 0.005,
            weight,
            balance,
            held_weight: weight,
            held_balance: balance,
            impact,
            slash: share(s.get(at::SLASH + side)),
            crush: share(s.get(at::CRUSH + side)),
            pierce: share(s.get(at::PIERCE + side)),
            thrust: share(s.get(at::THRUST)),
            swing_speed: 0.0,
            handling: 0.8 - (first_weight + 1.0 - balance) * 0.125,
            recoil: 0.0,
            swing_shaping: 0.0,
            blend: 0,
            arm_stiffness: 0.0,
            length: lengths.length,
            sweet_spot: lengths.sweet_spot,
            reach: if armed {
                0.0
            } else {
                f32::from(s.get(at::END))
            },
            second_hand: ((f64::from(s.get(at::ADJUST)) - 128.0) * 0.00156 + 1.0) as f32,
            swing_volume: 0.0,
            swing_pitch: 0.0,
            swing_turn: 0.0,
            hit_sound: s.get(at::SOUND),
        };
        record.hold(holder);
        record
    }

    /// The record of a plain weapon section of a single part `length` cm
    /// long, held by an average human.
    pub fn of_one_part(flags: u32, stats: &StatBytes, flipped: bool, length: f32) -> Self {
        let side = if flipped { at::FLIP } else { 0 };
        let start = f32::from(stats.get(at::START + side)) * 0.005;
        let end = f32::from(stats.get(at::END + side)) * 0.005;
        Self::build(
            &Section {
                flags,
                stats: *stats,
            },
            flipped,
            Lengths::one_part(length, start, end),
            &Holder::AVERAGE,
        )
    }

    /// The holding pass, run at the build and again on every equip change
    /// with the holder's encumbrance and size: the held weight and balance,
    /// swing speed, recoil, swing shaping, blend, arm stiffness, reach and
    /// the swing sound and AI turn factors.
    pub fn hold(&mut self, holder: &Holder) {
        let g = holder.encumbrance;
        let s = holder.size;
        let (mut w8, w9) = if holder.half_sword {
            ((2.0 - self.weight) * self.weight, self.balance * 0.7)
        } else {
            (self.weight, self.balance)
        };
        w8 *= 1.0 - holder.weightless;
        self.held_weight = w8;
        self.held_balance = w9;
        let h = if w9 <= 1.0 {
            (1.0 - w9) * 0.6 + w8 * 0.4 + 0.1
        } else {
            w8 * 0.4 + 0.1
        };
        self.recoil = g * 0.5 + h;
        let k = ((1.0 - g) + w9) * 0.5 * g + w9 * (1.0 - g);
        self.swing_shaping = 1.0 - k;
        self.blend = ((w8 + 1.0 - k) * 10.0).round_ties_even() as i32;
        let hands = self.flags & flags::HANDS;
        self.arm_stiffness = 0.7 - self.recoil * 0.3;
        if hands == flags::TWO_HANDED && !holder.half_sword {
            self.arm_stiffness *= 0.7 - w9 * 0.2;
        }
        if self.flags & 0xFFFF == flags::GREATSWORD {
            self.arm_stiffness *= 1.5;
        }
        self.swing_speed = (1.0 - w8) * 0.15 + 1.05;
        if self.flags & flags::HAND_CLASS == 0 {
            self.swing_speed *= 1.2;
        }
        self.swing_turn = (self.swing_shaping * 0.4 + w8 * 0.3 + 0.4).min(1.5);
        let l = self.length;
        if l <= 124.0 {
            self.swing_volume = ((250.0 - l) * 0.004).sqrt() * 1.5;
            self.swing_pitch = (250.0 - l) * 0.005_555_555_7;
        } else {
            self.swing_volume = 1.0;
            self.swing_pitch = 0.7;
        }
        let mut d = w9 - w8;
        if d < 0.0 {
            d *= 0.5;
        }
        self.swing_pitch *= d * 0.5 + 1.0;
        if hands == 0 {
            self.swing_volume *= holder.scale.sqrt() * 0.7;
            self.swing_pitch = self.swing_pitch * 0.6 / holder.scale;
            self.swing_turn *= if self.flags & flags::HAND_CLASS == flags::CLAWS {
                1.5
            } else {
                0.5
            };
            self.arm_stiffness = 0.5 / s;
        } else {
            self.reach = s * 60.0 + self.sweet_spot;
        }
    }

    pub fn kind(&self) -> u32 {
        self.flags & flags::KIND
    }

    /// Held in one or both hands, not a fist or claw.
    pub fn armed(&self) -> bool {
        self.flags & flags::HANDS != 0
    }

    /// The hit sound set's name.
    pub fn hit_sound_name(&self) -> Option<&'static str> {
        HIT_SOUNDS.get(usize::from(self.hit_sound)).copied()
    }

    /// The reach in metres.
    pub fn reach_metres(&self) -> f32 {
        self.reach / CM_PER_M
    }
}

/// The weapon fields of a character's race record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceWeapons {
    /// Reach of an unarmed character, cm (race `0x84`).
    pub unarmed_reach: f32,
    /// Scales every held weapon's AI swing turn (race `0x88`).
    pub swing_turn: f32,
    /// Scales every held weapon's stance blend (race `0x90`).
    pub blend: f32,
    /// The race's clawed-hands word (race `0x58`): `0x100` claws, `0x20100`
    /// heavy claws.
    pub claws: u32,
}

impl RaceWeapons {
    /// The human's: 50 cm unarmed, both factors 1, fists.
    pub const HUMAN: RaceWeapons = RaceWeapons {
        unarmed_reach: 50.0,
        swing_turn: 1.0,
        blend: 1.0,
        claws: 1,
    };

    /// The fields of a race's stats (`races.rdb`, read by
    /// [`crate::races`]): its unarmed reach (stored in metres, here in cm),
    /// its swing-turn and recovery factors and its kind word.
    pub fn of_stats(stats: &RaceStats) -> Self {
        Self {
            unarmed_reach: stats.unarmed_reach * CM_PER_M,
            swing_turn: stats.swing_turn,
            blend: stats.recovery,
            claws: stats.kind,
        }
    }
}

/// Race indices (a character record's race byte) with their own unarmed or
/// weapon rules.
pub mod race {
    pub const SIR: u8 = 4;
    pub const FAEL: u8 = 5;
    pub const GHOUL: u8 = 6;
    pub const GHOUL_H: u8 = 7;
    pub const GOBBLER: u8 = 8;
}

/// The sections of an unarmed character's two hands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unarmed {
    pub right: Section,
    pub left: Section,
    /// The large clawed races' state: their hands weigh four times their
    /// part's mass instead of twice.
    pub heavy_claws: bool,
}

/// The fist or claw sections of a character of race index `race` whose race
/// record's clawed-hands word is `claws`. The right hand goes by the race
/// index, the left by the word, so a race whose index has no rule but whose
/// word has claws (the Harvester) strikes with a right fist and a left claw.
pub fn unarmed(race_index: u8, claws: u32) -> Unarmed {
    let mut heavy_claws = false;
    let right = match race_index {
        race::GOBBLER => Section::HEAVY_CLAWS,
        race::GHOUL | race::GHOUL_H => Section::CLAWS,
        race::SIR | race::FAEL => {
            heavy_claws = true;
            Section::HEAVY_CLAWS
        }
        _ => Section::FISTS,
    };
    let left = match claws {
        0x100 => Section::CLAWS,
        0x20100 => {
            heavy_claws = true;
            Section::HEAVY_CLAWS
        }
        _ => Section::FISTS,
    };
    Unarmed {
        right,
        left,
        heavy_claws,
    }
}

/// What an equip change gives the character.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Equipped {
    /// Its reach, cm: the longest held weapon's, or its race's unarmed.
    pub reach: f32,
    /// The AI's thrust preference, 1..=9 (10 for a weapon with no slash,
    /// crush or pierce; 0 holding nothing).
    pub thrust_preference: u8,
    /// Unarmed, its hands weigh this many times their part's mass.
    pub hand_mass: Option<f32>,
}

/// The equip refresh: every held record (the primary first) is held again
/// by `holder`, then scaled by the race's blend and swing-turn factors and
/// the race index's own rules (ghouls swing with arm stiffness 1, the Fael
/// turns half as far).
pub fn equip(
    held: &mut [WeaponRecord],
    holder: &Holder,
    race: &RaceWeapons,
    race_index: u8,
) -> Equipped {
    let mut reach = 0.0f32;
    for w in held.iter_mut() {
        w.hold(holder);
        if w.reach > reach {
            reach = w.reach;
        }
        w.blend = (w.blend as f32 * race.blend).round_ties_even() as i32;
        w.swing_turn *= race.swing_turn;
    }
    let thrust_preference = held.first().map_or(0, thrust_preference);
    let mut hand_mass = None;
    if held.first().is_none_or(|w| !w.armed()) {
        reach = race.unarmed_reach;
        let heavy = unarmed(race_index, race.claws).heavy_claws;
        hand_mass = Some(if heavy { 4.0 } else { 2.0 });
    }
    match race_index {
        race::GHOUL | race::GHOUL_H => held.iter_mut().for_each(|w| w.arm_stiffness = 1.0),
        race::FAEL => held.iter_mut().for_each(|w| w.swing_turn *= 0.5),
        _ => {}
    }
    Equipped {
        reach,
        thrust_preference,
        hand_mass,
    }
}

/// The AI's thrust preference of a primary weapon: thrust against its
/// slash, crush and pierce, 1..=9, or 10 when it has none of those.
pub fn thrust_preference(primary: &WeaponRecord) -> u8 {
    let edges = primary.slash + primary.crush + primary.pierce;
    if edges == 0.0 {
        return 10;
    }
    let p = (primary.thrust / edges * 5.0).round_ties_even() as i32 + 1;
    p.clamp(1, 9) as u8
}

/// Step reach, cm: 80·√scale, shortened 30 % per unit of feral level and
/// 20 % per unit of encumbrance.
const STEP_REACH: f32 = 80.0;
const FERAL_REACH: f64 = 0.3;
const ENCUMBERED_REACH: f64 = 0.2;

/// A character's step reach, cm: 80·√scale, shortened 30 % per unit of its
/// feral level (when over 0) and 20 % per unit of its encumbrance, in
/// double precision as the original's init computes it. Its long steps,
/// footwork and knockback step are measured in it; a character's stats
/// give it in metres (`CharacterStats::step_reach`).
pub fn step_reach(scale: f32, feral: f32, encumbrance: f32) -> f32 {
    let full = f64::from(STEP_REACH * scale.sqrt());
    let encumbered = 1.0 - f64::from(encumbrance) * ENCUMBERED_REACH;
    let feral = if feral > 0.0 {
        1.0 - f64::from(feral) * FERAL_REACH
    } else {
        1.0
    };
    (feral * encumbered * full) as f32
}

/// The light and heavy ends of the weapon mass table by hands (one-handed,
/// two-handed, polearm) and size (normal, ogre, demon).
const MASS: [[(f32, f32); 3]; 3] = [
    [(0.16, 0.40), (0.20, 0.50), (0.20, 0.50)],
    [(0.32, 0.80), (0.50, 1.00), (0.50, 1.00)],
    [(0.50, 1.20), (0.75, 1.50), (0.75, 1.50)],
];

/// The total simulation mass of a weapon of `flags` and weight W6: its
/// table pair blended by the weight. `None` for fists and claws.
pub fn total_mass(flags: u32, weight: f32) -> Option<f32> {
    let hands = ((flags & flags::HANDS) >> 4).checked_sub(1)? as usize;
    let size = if flags & flags::DEMON_SIZE != 0 {
        2
    } else if flags & flags::OGRE_SIZE != 0 {
        1
    } else {
        0
    };
    let (light, heavy) = MASS[size][hands];
    Some(heavy * weight + (1.0 - weight) * light)
}

/// Spreads a weapon's `total` mass over its rigid parts, whose own masses
/// are `parts` (the weapon's part first, then its child parts): half evenly
/// and half by their own masses. The holder's size multiplies the first
/// part's share afterwards.
pub fn spread_mass(total: f32, parts: &[f32], size: f32) -> Vec<f32> {
    let n = parts.len() as f32;
    let sum: f32 = parts.iter().sum();
    parts
        .iter()
        .enumerate()
        .map(|(i, &m)| {
            let own = if sum > 0.0 { m / sum } else { 1.0 / n };
            let share = (0.5 / n + own * 0.5) * total;
            if i == 0 { share * size } else { share }
        })
        .collect()
}

/// A shield's record (section `0x8000`, bytes 4, 5 and 6 × 0.01).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShieldRecord {
    /// Slows its parries.
    pub weight: f32,
    /// Lessens what a block lets through and what it costs.
    pub block: f32,
    /// Adds to the holder's encumbrance.
    pub bulk: f32,
}

/// What a shield in the off hand gives its holder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShieldFactors {
    /// Parry effectiveness: 0.5 + 1.5·(1 − weight)² (half the weight with
    /// the Mobility technique).
    pub parry: f32,
    /// Share of a blocked blow that leaks through: 1 − block/2.
    pub leak: f32,
    /// Stamina a block costs per unit of impact: the leak (× 2/3 with the
    /// Endure technique).
    pub stamina_cost: f32,
}

impl ShieldRecord {
    /// The record of a shield section body; `None` when it is too short.
    /// Each byte is scaled in double precision, as the original builds it.
    pub fn from_section(body: &[u8]) -> Option<Self> {
        let b = body.get(4..7)?;
        let share = |byte: u8| (f64::from(byte) * 0.01) as f32;
        Some(Self {
            weight: share(b[0]),
            block: share(b[1]),
            bulk: share(b[2]),
        })
    }

    /// The factors the shield gives when attached, with the holder's
    /// Mobility and Endure shield techniques.
    pub fn factors(&self, mobility: bool, endure: bool) -> ShieldFactors {
        let w = if mobility {
            1.0 - self.weight * 0.5
        } else {
            1.0 - self.weight
        };
        let leak = 1.0 - self.block * 0.5;
        ShieldFactors {
            parry: w * w * 1.5 + 0.5,
            leak,
            stamina_cost: if endure { leak * 0.666_666_7 } else { leak },
        }
    }

    /// The shield's share of the holder's encumbrance, before the shield
    /// technique's −0.3: (2 − weightless)·bulk/4.
    pub fn encumbrance(&self, weightless: f32) -> f32 {
        (2.0 - weightless) * self.bulk * 0.25
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    /// "- Sword A" of the object database: weight 50, balance 80, thrust
    /// 54, type 2, striking part 20..200, slash 72.
    fn sword_a_section() -> Section {
        let mut stats = StatBytes::default();
        for (off, v) in [
            (at::WEIGHT, 50),
            (at::BALANCE, 80),
            (at::THRUST, 54),
            (at::KIND, 2),
            (at::START, 20),
            (at::END, 200),
            (at::SLASH, 72),
        ] {
            stats.set(off, v);
        }
        Section { flags: 0x11, stats }
    }

    fn sword_a() -> WeaponRecord {
        let s = sword_a_section();
        WeaponRecord::build(
            &s,
            false,
            Lengths::one_part(90.0, 0.1, 1.0),
            &Holder::AVERAGE,
        )
    }

    #[test]
    fn a_one_handed_sword() {
        let w = sword_a();
        assert_eq!((w.flags, w.code), (0x11, 2));
        assert!(close(w.impact, 0.42));
        assert!((w.slash - 0.397).abs() < 1e-3);
        assert!((w.thrust - 0.325).abs() < 1e-3);
        assert_eq!((w.crush, w.pierce), (0.0, 0.0));
        // The start 0.1 is below 0.15: W3 falls back on 0.7.
        assert!(close(w.start_clamped, 0.7));
        assert!(close(w.start, 0.1) && close(w.end, 1.0));
        assert!(close(w.handling, 0.7125));
        // Holding pass with no encumbrance at size 1.
        assert!(close(w.held_weight, 0.5) && close(w.held_balance, 0.8));
        assert!(close(w.recoil, 0.42));
        assert!(close(w.swing_shaping, 0.2));
        assert_eq!(w.blend, 7);
        assert!(close(w.arm_stiffness, 0.574));
        assert!(close(w.swing_speed, 1.125));
        assert!(close(w.swing_turn, 0.63));
        // 90 cm: 1.5·√0.64 and 160/180, the pitch × (1 + 0.3/2).
        assert!(close(w.swing_volume, 1.2));
        assert!(close(w.swing_pitch, 160.0 / 180.0 * 1.15));
        // Sweet spot 0.1 + 0.6·0.9 = 0.64 of 90 cm; reach 60 cm more.
        assert!(close(w.length, 90.0) && close(w.sweet_spot, 57.6));
        assert!(close(w.reach, 117.6));
        assert!(close(w.second_hand, 1.0));
        assert_eq!(w.hit_sound_name(), Some("WH_SwordA"));
        assert_eq!(thrust_preference(&w), 5);
    }

    #[test]
    fn encumbrance_slows_the_swing_and_the_recovery() {
        let mut w = sword_a();
        w.hold(&Holder {
            encumbrance: 0.5,
            size: 1.2,
            ..Holder::AVERAGE
        });
        // h 0.42 + g/2.
        assert!(close(w.recoil, 0.67));
        // k = (0.5 + 0.8)·0.5·0.5 + 0.8·0.5 = 0.725.
        assert!(close(w.swing_shaping, 0.275));
        assert_eq!(w.blend, 8);
        assert!(close(w.arm_stiffness, 0.7 - 0.67 * 0.3));
        // The reach grows with the size.
        assert!(close(w.reach, 72.0 + 57.6));
        // Holding again with the average holder restores the record.
        w.hold(&Holder::AVERAGE);
        assert_eq!(w, sword_a());
    }

    #[test]
    fn the_half_sword_hold_and_the_weightless_item() {
        let mut w = sword_a();
        w.hold(&Holder {
            half_sword: true,
            ..Holder::AVERAGE
        });
        assert!(close(w.held_weight, 0.75));
        assert!(close(w.held_balance, 0.56));
        w.hold(&Holder {
            weightless: 1.0,
            ..Holder::AVERAGE
        });
        assert_eq!(w.held_weight, 0.0);
        assert!(close(w.swing_speed, 1.2));
        // The first build keeps half the weightless share in W16.
        let s = sword_a_section();
        let light = WeaponRecord::build(
            &s,
            false,
            Lengths::one_part(90.0, 0.1, 1.0),
            &Holder {
                weightless: 1.0,
                ..Holder::AVERAGE
            },
        );
        assert!(close(light.handling, 0.8 - (0.25 + 0.2) * 0.125));
    }

    #[test]
    fn two_handers_greatswords_and_long_weapons() {
        let mut s = sword_a_section();
        s.flags = 0x421;
        let long = WeaponRecord::build(
            &s,
            false,
            Lengths::one_part(130.0, 0.1, 1.0),
            &Holder::AVERAGE,
        );
        // Two hands: × (0.7 − 0.2·0.8).
        assert!(close(long.arm_stiffness, 0.574 * 0.54));
        // Longer than 124 cm: volume 1, pitch 0.7 × 1.15.
        assert!(close(long.swing_volume, 1.0));
        assert!(close(long.swing_pitch, 0.7 * 1.15));
        s.flags = 0x521;
        let great = WeaponRecord::build(
            &s,
            false,
            Lengths::one_part(130.0, 0.1, 1.0),
            &Holder::AVERAGE,
        );
        assert!(close(great.arm_stiffness, 0.574 * 0.54 * 1.5));
        // The half-sword hold drops the two-hand factor.
        let mut held = long;
        held.hold(&Holder {
            half_sword: true,
            ..Holder::AVERAGE
        });
        assert!(close(held.arm_stiffness, 0.7 - held.recoil * 0.3));
    }

    #[test]
    fn the_fists() {
        let hand = Lengths::hand(-7.21, 10.45);
        assert!(close(hand.length, 17.66) && close(hand.sweet_spot, 17.66));
        let w = WeaponRecord::build(&Section::FISTS, false, hand, &Holder::AVERAGE);
        assert!(!w.armed());
        assert!(close(w.impact, 0.43));
        assert_eq!((w.code, w.slash, w.thrust), (1, 0.0, 0.0));
        assert!(close(w.start, 0.4) && close(w.end, 0.4));
        assert!(close(w.recoil, 0.1));
        assert_eq!(w.swing_shaping, 0.0);
        assert_eq!(w.blend, 0);
        assert!(close(w.swing_speed, 1.44));
        assert!(close(w.arm_stiffness, 0.5));
        assert!(close(w.swing_turn, 0.2));
        assert!(close(w.handling, 0.8));
        // The fists' adjustment byte is 0, not the neutral 128.
        assert!(close(w.second_hand, 0.80032));
        // The section's end byte, not 60 cm plus the sweet spot.
        assert_eq!(w.reach, 80.0);
        // (250 − 17.66): volume 1.5·√0.92936·0.7, pitch /180·1.5·0.6.
        assert!(close(w.swing_volume, 1.5 * 0.929_36f32.sqrt() * 0.7));
        assert!(close(w.swing_pitch, (250.0 - 17.66) / 180.0 * 1.5 * 0.6));
        assert_eq!(w.hit_sound_name(), Some("BdyCol"));
        // Claws are not ×1.2 fast, and turn ×1.5.
        let c = WeaponRecord::build(&Section::CLAWS, false, hand, &Holder::AVERAGE);
        assert!(close(c.swing_speed, 1.2));
        assert!(close(c.swing_turn, 0.6));
        assert!(close(c.slash, 0.5 * 0.6f32.powf(0.7)));
        assert!(close(c.impact, 0.63));
        assert_eq!(c.hit_sound_name(), Some("WH_ClawsL"));
        let heavy = WeaponRecord::build(&Section::HEAVY_CLAWS, false, hand, &Holder::AVERAGE);
        assert!(close(heavy.impact, 0.83) && close(heavy.start, 0.75));
        // A larger body: the volume grows with √scale, the pitch falls.
        let big = WeaponRecord::build(
            &Section::FISTS,
            false,
            hand,
            &Holder {
                scale: 1.44,
                size: 2.0,
                ..Holder::AVERAGE
            },
        );
        assert!(close(big.swing_volume, w.swing_volume * 1.2));
        assert!(close(big.swing_pitch, w.swing_pitch / 1.44));
        assert!(close(big.arm_stiffness, 0.25));
    }

    #[test]
    fn the_flipped_side_strikes_with_its_own_bytes() {
        let mut s = sword_a_section();
        s.stats.set(at::KIND + at::FLIP, 1);
        s.stats.set(at::START + at::FLIP, 40);
        s.stats.set(at::END + at::FLIP, 180);
        s.stats.set(at::CRUSH + at::FLIP, 30);
        let w = WeaponRecord::build(
            &s,
            true,
            Lengths::one_part(90.0, 0.2, 0.9),
            &Holder::AVERAGE,
        );
        assert_eq!(w.code, 1);
        assert!(close(w.start, 0.2) && close(w.end, 0.9));
        assert_eq!(w.slash, 0.0);
        assert!(close(w.crush, 0.5 * 0.3f32.powf(0.7)));
        // Thrust has no flipped byte.
        assert!((w.thrust - 0.325).abs() < 1e-3);
    }

    #[test]
    fn lengths_along_the_first_child_chain() {
        let n = |top: f32, rigid: bool| ChainNode { top, rigid };
        // A flail: handle 35.3 and four rigid links.
        let flail = [
            n(35.3, true),
            n(9.9, true),
            n(9.8, true),
            n(9.8, true),
            n(19.6, true),
        ];
        let l = Lengths::armed(&flail, 0.7, 0.9);
        assert!(close(l.length, 84.4));
        // Whole parts but the last, then 0.82 of it.
        assert!(close(l.sweet_spot, 35.3 + 9.9 + 9.8 + 9.8 + 0.82 * 19.6));
        // A torch: its flame and shadow children are not rigid.
        let torch = [n(58.3, true), n(36.5, false), n(93.0, false)];
        let l = Lengths::armed(&torch, 0.7, 0.9);
        assert!(close(l.length, 58.3) && close(l.sweet_spot, 0.82 * 58.3));
        // A rigid node past a plain one still counts for the length only.
        let l = Lengths::armed(&[n(50.0, true), n(5.0, false), n(20.0, true)], 0.0, 1.0);
        assert!(close(l.length, 70.0) && close(l.sweet_spot, 30.0));
        assert_eq!(Lengths::armed(&[], 0.0, 1.0), Lengths::default());
    }

    #[test]
    fn unarmed_sections_by_race() {
        let human = unarmed(0, 1);
        assert_eq!((human.right, human.left), (Section::FISTS, Section::FISTS));
        assert!(!human.heavy_claws);
        let ghoul = unarmed(race::GHOUL, 0x100);
        assert_eq!((ghoul.right, ghoul.left), (Section::CLAWS, Section::CLAWS));
        let fael = unarmed(race::FAEL, 0x20100);
        assert_eq!(fael.right, Section::HEAVY_CLAWS);
        assert!(fael.heavy_claws);
        // The Harvester (race 10): a right fist, a left heavy claw.
        let harvester = unarmed(10, 0x20100);
        assert_eq!(
            (harvester.right, harvester.left),
            (Section::FISTS, Section::HEAVY_CLAWS)
        );
        assert!(harvester.heavy_claws);
        // The Gobbler's right claw alone would not make it heavy.
        assert!(!unarmed(race::GOBBLER, 1).heavy_claws);
    }

    #[test]
    fn equipping_scales_by_race_and_takes_the_longest_reach() {
        let mut held = [sword_a(), {
            let mut s = sword_a_section();
            s.flags = 0x12;
            WeaponRecord::build(
                &s,
                false,
                Lengths::one_part(60.0, 0.1, 1.0),
                &Holder::AVERAGE,
            )
        }];
        let golem = RaceWeapons {
            unarmed_reach: 100.0,
            swing_turn: 0.0,
            blend: 0.5,
            claws: 0x10001,
        };
        let e = equip(&mut held, &Holder::AVERAGE, &golem, 16);
        assert!(close(e.reach, 117.6));
        assert_eq!(e.thrust_preference, 5);
        assert_eq!(e.hand_mass, None);
        // 7 × 0.5 = 3.5 rounds to even.
        assert_eq!(held[0].blend, 4);
        assert_eq!(held[0].swing_turn, 0.0);
        // Equipping again starts from the holding pass, not from the scaled
        // values.
        let again = equip(&mut held, &Holder::AVERAGE, &golem, 16);
        assert_eq!(again, e);
        assert_eq!(held[0].blend, 4);
        // Ghouls swing with arm stiffness 1; the Fael turns half as far.
        let mut held = [sword_a()];
        equip(
            &mut held,
            &Holder::AVERAGE,
            &RaceWeapons::HUMAN,
            race::GHOUL,
        );
        assert_eq!(held[0].arm_stiffness, 1.0);
        equip(&mut held, &Holder::AVERAGE, &RaceWeapons::HUMAN, race::FAEL);
        assert!(close(held[0].swing_turn, 0.315));
    }

    #[test]
    fn unarmed_characters_reach_as_their_race() {
        let hand = Lengths::hand(-7.21, 10.45);
        let mut fists = [
            WeaponRecord::build(&Section::FISTS, false, hand, &Holder::AVERAGE),
            WeaponRecord::build(&Section::FISTS, false, hand, &Holder::AVERAGE),
        ];
        let e = equip(&mut fists, &Holder::AVERAGE, &RaceWeapons::HUMAN, 0);
        assert_eq!(e.reach, 50.0);
        assert_eq!(e.thrust_preference, 10);
        assert_eq!(e.hand_mass, Some(2.0));
        let sir = RaceWeapons {
            unarmed_reach: 150.0,
            claws: 0x20100,
            ..RaceWeapons::HUMAN
        };
        let e = equip(&mut fists, &Holder::AVERAGE, &sir, race::SIR);
        assert_eq!((e.reach, e.hand_mass), (150.0, Some(4.0)));
        let e = equip(&mut [], &Holder::AVERAGE, &RaceWeapons::HUMAN, 0);
        assert_eq!((e.reach, e.thrust_preference), (50.0, 0));
    }

    #[test]
    fn thrust_preference_is_clamped() {
        let mut w = sword_a();
        w.thrust = 2.0;
        assert_eq!(thrust_preference(&w), 9);
        w.thrust = 0.0;
        assert_eq!(thrust_preference(&w), 1);
        w.slash = 0.0;
        assert_eq!(thrust_preference(&w), 10);
    }

    #[test]
    fn step_reach_of_a_character() {
        assert_eq!(step_reach(1.0, 0.0, 0.0), 80.0);
        // A feral (0.5), encumbered (0.5) character of scale 1.21.
        assert!(close(step_reach(1.21, 0.5, 0.5), 1.1 * 0.85 * 0.9 * 80.0));
        // A negative level is ignored.
        assert_eq!(step_reach(1.0, -1.0, 0.0), 80.0);
        // In double precision: exanima01's zombies of feral level 0.84
        // reach 59.84 cm, not single precision's 59.840004.
        assert_eq!(step_reach(1.0, 0.84, 0.0), 59.84);
    }

    #[test]
    fn weapon_masses() {
        // One-handed, normal size, weight 0.5: between 0.16 and 0.40.
        assert!(close(total_mass(0x11, 0.5).unwrap(), 0.28));
        // An ogre's one-handed club (weight 0.8).
        assert!(close(total_mass(0x0200_0013, 0.8).unwrap(), 0.704));
        // A demon's sword (the demon bit wins over the ogre bit).
        assert!(close(total_mass(0x1700_0011, 0.5).unwrap(), 0.85));
        // Polearm and two-hander share a row.
        assert_eq!(total_mass(0x32, 1.0), Some(0.5));
        assert_eq!(total_mass(0x0400_0022, 0.0), Some(0.75));
        assert_eq!(total_mass(0x40, 0.5), None);
        // Spread over a blade (2) and a pommel (1) part: half evenly, half
        // by mass; the holder's size weighs the first part.
        let parts = spread_mass(0.6, &[2.0, 1.0], 1.0);
        assert!(close(parts[0], 0.6 * (0.25 + 1.0 / 3.0)));
        assert!(close(parts[1], 0.6 * (0.25 + 1.0 / 6.0)));
        assert!(close(parts.iter().sum::<f32>(), 0.6));
        let big = spread_mass(0.6, &[2.0, 1.0], 1.5);
        assert!(close(big[0], parts[0] * 1.5) && close(big[1], parts[1]));
        assert_eq!(spread_mass(0.4, &[0.0], 1.0), [0.4]);
    }

    #[test]
    fn shield_factors() {
        let mut body = [0u8; 80];
        body[4..7].copy_from_slice(&[50, 58, 60]);
        let s = ShieldRecord::from_section(&body).unwrap();
        assert!(close(s.weight, 0.5) && close(s.block, 0.58) && close(s.bulk, 0.6));
        let f = s.factors(false, false);
        assert!(close(f.parry, 0.875));
        assert!(close(f.leak, 0.71) && close(f.stamina_cost, 0.71));
        let f = s.factors(true, true);
        assert!(close(f.parry, 0.75 * 0.75 * 1.5 + 0.5));
        assert!(close(f.stamina_cost, 0.71 * 2.0 / 3.0));
        assert!(close(s.encumbrance(0.0), 0.3));
        assert!(close(s.encumbrance(1.0), 0.15));
        assert!(ShieldRecord::from_section(&[0; 6]).is_none());
    }

    /// The record of every weapon item of the installed object database
    /// (plain models measured along their chain in `Objects.rpk`, factory
    /// items from their recipe's stats), the fists and claws of every race
    /// and every shield (skipped without the game).
    #[test]
    fn shipped_weapon_records() {
        use crate::rfc::{NodeBody, Scene};
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let objects = game.pack(exa_rpk::PackKind::Objects).unwrap();
        let factories = game.pack(exa_rpk::PackKind::Factories).unwrap();
        let db = crate::rdb::Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let items = crate::objdb::items(&db);
        let (mut plain, mut factory, mut missing) = (0, 0, 0);
        let mut by_name = std::collections::HashMap::new();
        for item in &items {
            let Some(w) = item.weapon else { continue };
            let mut stats = w.stats;
            let lengths = if let Some(name) = item.model.strip_prefix('@') {
                let f = crate::factory::Factory::parse(factories.get(name).unwrap()).unwrap();
                let built = f.build(&w.spec(crate::factory::DEFAULT_METAL));
                stats = crate::weapon_stats::factory_stats(&f, &built.generated, w.stats);
                stats = crate::weapon_stats::condition(stats, w.quality, w.damage);
                factory += 1;
                // Measured on the assembled model by `exa_character`.
                Lengths::one_part(100.0, 0.0, 1.0)
            } else if let Some(bytes) = objects.get(&item.model) {
                plain += 1;
                let scene = Scene::parse(bytes).unwrap();
                let nodes = chain(&scene.nodes, 0);
                // The box tops the mesh: centre plus half extent is the
                // highest vertex.
                if let NodeBody::Mesh(m) = &scene.nodes[0].body {
                    let top = m.positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
                    assert!(
                        (nodes[0].top - top * CM_PER_M).abs() < 0.05,
                        "{}: box top {} vs vertices {}",
                        item.model,
                        nodes[0].top,
                        top * CM_PER_M
                    );
                }
                let start = f32::from(stats.get(at::START)) * 0.005;
                let end = f32::from(stats.get(at::END)) * 0.005;
                Lengths::armed(&nodes, start, end)
            } else {
                missing += 1;
                continue;
            };
            let section = Section {
                flags: w.flags,
                stats,
            };
            let r = WeaponRecord::build(&section, false, lengths, &Holder::AVERAGE);
            let name = &item.name;
            assert!(r.armed(), "{name}");
            assert!((1.05..=1.2).contains(&r.swing_speed), "{name} {r:?}");
            assert!((0.1..=1.1).contains(&r.recoil), "{name} {r:?}");
            assert!((0.0..=1.0).contains(&r.swing_shaping), "{name} {r:?}");
            assert!((0..=20).contains(&r.blend), "{name} {r:?}");
            assert!(
                r.arm_stiffness > 0.2 && r.arm_stiffness < 1.0,
                "{name} {r:?}"
            );
            assert!(r.swing_turn > 0.4 && r.swing_turn <= 1.5, "{name} {r:?}");
            assert!(
                r.length > 0.0 && r.sweet_spot <= r.length + 1e-3,
                "{name} {r:?}"
            );
            assert!((r.reach - 60.0 - r.sweet_spot).abs() < 1e-3);
            assert!((1..=10).contains(&thrust_preference(&r)));
            let mass = total_mass(r.flags, r.weight).unwrap();
            assert!((0.16..=1.5).contains(&mass), "{name} {mass}");
            by_name.insert(item.name.clone(), r);
        }
        assert_eq!((plain, factory, missing), (103, 98, 7));
        let near = |a: f32, b: f32, tol: f32| (a - b).abs() < tol;
        // The torch: its shadow and flame children are not rigid bodies.
        let torch = by_name["Torch A01"];
        assert_eq!(torch.flags, 0x13);
        assert!(near(torch.length, 58.3, 0.05), "{torch:?}");
        assert!(near(torch.sweet_spot, 0.82 * torch.length, 1e-3));
        assert!(near(torch.recoil, 0.46, 1e-5) && near(torch.swing_shaping, 0.4, 1e-5));
        assert_eq!(torch.blend, 7);
        assert!(near(torch.arm_stiffness, 0.562, 1e-5));
        assert!(near(torch.swing_speed, 1.155, 1e-5));
        assert!(near(torch.swing_turn, 0.65, 1e-5));
        assert!(near(
            total_mass(torch.flags, torch.weight).unwrap(),
            0.232,
            1e-5
        ));
        // Flails add their rigid links.
        assert!(near(by_name["Flail A"].length, 84.4, 0.1));
        let ogre_flail = by_name["Ogre Flail"];
        assert!(near(ogre_flail.length, 172.0, 0.1));
        let flail_mass = total_mass(ogre_flail.flags, ogre_flail.weight).unwrap();
        assert!(near(flail_mass, 0.608, 1e-5));
        // A greatsword.
        let gsf = by_name["Sword GSF 01"];
        assert!(near(gsf.length, 125.0, 0.05));
        assert!(near(gsf.sweet_spot, 0.64 * gsf.length, 1e-3));
        assert!(near(gsf.arm_stiffness, 0.526 * 0.56 * 1.5, 1e-5));
        assert!(near(gsf.swing_speed, 1.0875, 1e-5));
        assert_eq!(gsf.second_hand, 1.0);
        // A two-hander whose adjustment byte is 0: the second hand 20 %
        // nearer the first.
        assert!(near(by_name["Falchion 2H B"].second_hand, 0.80032, 1e-6));
        // An ogre's club: weight 0.8 in the ogre row.
        let club = by_name["Ogre SpkClub"];
        assert!(near(club.impact, 1.08, 1e-5));
        assert!(near(
            total_mass(club.flags, club.weight).unwrap(),
            0.704,
            1e-5
        ));

        // Fists and claws of every race, from its body model's hands.
        let races = crate::races::parse(resource.get("races.rdb").unwrap()).unwrap();
        let mut seen = 0;
        for record in &races {
            let race = RaceWeapons::of_stats(&record.stats);
            let model = record.model();
            let scene = Scene::parse(resource.get(&model).unwrap()).unwrap();
            let hand = |prefix: &str| {
                let node = scene.nodes.iter().find(|n| n.name.starts_with(prefix));
                hand_lengths(node.unwrap())
            };
            let index = (record.id - 1) as u8;
            let hands = unarmed(index, race.claws);
            let mut held = [
                WeaponRecord::build(&hands.right, false, hand("17"), &Holder::AVERAGE),
                WeaponRecord::build(&hands.left, false, hand("18"), &Holder::AVERAGE),
            ];
            let e = equip(&mut held, &Holder::AVERAGE, &race, index);
            assert_eq!(e.reach, race.unarmed_reach);
            let hand_mass = if hands.heavy_claws { 4.0 } else { 2.0 };
            assert_eq!(e.hand_mass, Some(hand_mass));
            for w in &held {
                assert!(
                    !w.armed() && w.length > 5.0 && w.length < 100.0,
                    "{model} {w:?}"
                );
            }
            match index {
                0 => {
                    assert!(near(held[0].length, 17.66, 0.01), "{:?}", held[0]);
                    assert_eq!(e.reach, 50.0);
                    assert!(near(held[0].swing_speed, 1.44, 1e-6));
                    assert_eq!(e.thrust_preference, 10);
                }
                3 => assert_eq!((e.reach, race.swing_turn), (100.0, 0.5)),
                4 => assert_eq!((e.reach, hands.heavy_claws), (150.0, true)),
                6 => assert_eq!(hands.left, Section::CLAWS),
                10 => assert_eq!(
                    (hands.right, hands.left),
                    (Section::FISTS, Section::HEAVY_CLAWS)
                ),
                16 => assert_eq!((race.blend, held[0].blend), (0.5, 0)),
                _ => {}
            }
            seen += 1;
        }
        assert_eq!(seen, 18);

        // Shields.
        let shields: Vec<_> = items.iter().filter_map(|i| Some((i, i.shield?))).collect();
        assert_eq!(shields.len(), 22);
        let round = shields.iter().find(|(i, _)| i.id == 15).unwrap().1;
        assert!(near(round.weight, 0.5, 1e-6) && near(round.block, 0.58, 1e-6));
        assert!(near(round.bulk, 0.6, 1e-6));
        let f = round.factors(false, false);
        assert!(near(f.parry, 0.875, 1e-6) && near(f.leak, 0.71, 1e-6));
    }

    /// A race's stats give its weapon fields: Sir's 1.5 m of unarmed reach
    /// is 150 cm, its swing turn 0, its recovery 1 and its heavy-claws word.
    #[test]
    fn race_weapon_fields() {
        let stats = RaceStats {
            kind: 0x20100,
            unarmed_reach: 1.5,
            swing_turn: 0.0,
            recovery: 1.0,
            ..RaceStats::FALLBACK
        };
        assert_eq!(
            RaceWeapons::of_stats(&stats),
            RaceWeapons {
                unarmed_reach: 150.0,
                swing_turn: 0.0,
                blend: 1.0,
                claws: 0x20100
            }
        );
        let golem = RaceStats {
            recovery: 0.5,
            ..RaceStats::FALLBACK
        };
        let r = RaceWeapons::of_stats(&golem);
        assert_eq!((r.unarmed_reach, r.blend, r.claws), (100.0, 0.5, 0));
    }
}
