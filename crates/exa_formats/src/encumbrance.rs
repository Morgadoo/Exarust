//! A character's encumbrance, the one weight stat of the original: its
//! armour pass sums it from what the character wears and the shield it
//! holds, less what its armour and shield techniques take off, and the
//! reach, the run, the turn, the steps and the get-up clock shrink with
//! it. Rules and evidence: `docs/formats/OTHER.md` (Encumbrance).
//!
//! The sum runs in the original's mixed precision: each step in double,
//! stored as a float.

use crate::characters::{CharacterRecord, Perk};
use crate::objdb::{Armour, Burden, Item};
use crate::weapon_record::ShieldRecord;

/// Each unit of a worn item's encumbrance byte weighs this much.
const PER_BYTE: f64 = 0.0025;
/// An articulated piece whose coverage touches another articulated piece
/// takes this off, once.
const ARTICULATED: f64 = 0.025;
/// Each Manoeuvring in Armour tier takes this off.
const PER_TIER: f64 = 0.3;
/// Armour weighing less than this weighs nothing.
const FLOOR: f64 = 0.001;
/// An unpowered golem suit adds this.
const DEAD_SUIT: f32 = 2.0;
/// A held shield adds its share ([`ShieldRecord::encumbrance`]), less the
/// Mobility technique's 0.3; only a positive share counts.
const MOBILITY: f32 = 0.3;
/// The most encumbrance there is.
const CAP: f32 = 1.0;

/// The bones each bone 1..18 touches (bit `bone − 1`), itself included:
/// how far an articulated piece's coverage reaches. As the original's
/// table has it, the left humerus (bone 15) touches the right shoulder
/// (11) and not the left one (14).
pub const NEIGHBOURS: [u32; 18] = [
    0x93, 0x7, 0xE, 0xC, 0x31, 0x70, 0x60, 0x181, 0x380, 0x300, 0xD00, 0x1C00, 0x11800, 0x6100,
    0xC400, 0x2C000, 0x11000, 0x28000,
];

/// The bones touching any of `bones` (bit `bone − 1`).
pub fn neighbours(bones: u32) -> u32 {
    NEIGHBOURS
        .iter()
        .enumerate()
        .filter(|&(i, _)| bones >> i & 1 != 0)
        .fold(0, |m, (_, &n)| m | n)
}

/// What a character's techniques take off its encumbrance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Skills {
    /// The highest Manoeuvring in Armour tier learnt (0–3).
    pub manoeuvring: u8,
    /// The Mobility shield technique.
    pub mobility: bool,
}

impl Skills {
    pub fn of_record(record: &CharacterRecord) -> Self {
        Self {
            manoeuvring: record.manoeuvring_tier(),
            mobility: record.perks().has(Perk::Mobility),
        }
    }
}

/// The encumbrance (0…1) of a character wearing `worn` (in its wearing
/// order) and holding `shield`: 0.0025 per unit of each
/// item's encumbrance (an unpowered golem suit adds 2), less 0.025 for
/// every articulated piece touching another, less 0.3 per Manoeuvring
/// tier, nothing below 0.001; then the shield's (2 − weightless)·bulk/4
/// less 0.3 with Mobility when positive (powered shield-lightening
/// gauntlets make it weightless); at most 1.
pub fn encumbrance(worn: &[Burden], shield: Option<&ShieldRecord>, skills: Skills) -> f32 {
    let mut enc = 0.0f32;
    let mut weightless = 0.0f32;
    for b in worn {
        match b.special {
            Burden::GOLEM_SUIT if !b.powered => enc += DEAD_SUIT,
            Burden::LIGHT_SHIELD => weightless = if b.powered { 1.0 } else { 0.0 },
            _ => {}
        }
        enc = (f64::from(enc) + f64::from(b.encumbrance) * PER_BYTE) as f32;
    }
    // Articulated pieces touching any other articulated piece, of any set.
    let pieces: Vec<(u32, u32)> = worn
        .iter()
        .filter(|b| b.set != 0)
        .map(|b| (b.bones, neighbours(b.bones)))
        .collect();
    for (i, &(bones, reach)) in pieces.iter().enumerate() {
        let touches = pieces
            .iter()
            .enumerate()
            .any(|(j, &(other, other_reach))| i != j && (bones & other_reach | reach & other) != 0);
        if touches {
            enc = (f64::from(enc) - ARTICULATED) as f32;
        }
    }
    enc = (f64::from(enc) - f64::from(skills.manoeuvring) * PER_TIER) as f32;
    if f64::from(enc) < FLOOR {
        enc = 0.0;
    }
    if let Some(shield) = shield {
        let mobility = if skills.mobility { MOBILITY } else { 0.0 };
        let share = shield.encumbrance(weightless) - mobility;
        if share > 0.0 {
            enc += share;
        }
    }
    enc.min(CAP)
}

/// Whether a worn set grants its wearer a second wind: the armour pass sets
/// the character's second-wind flag for an item of special kind 2.
pub fn second_wind(worn: &[Burden]) -> bool {
    worn.iter().any(|b| b.special == Burden::SECOND_WIND)
}

/// Whether a character's record grants a second wind, resolved as the
/// armour pass resolves its worn items ([`of_record`]).
pub fn second_wind_of_record<'a>(
    record: &CharacterRecord,
    find: impl Fn(u32) -> Option<&'a Item>,
) -> bool {
    record
        .worn
        .iter()
        .filter_map(|worn| find(worn.id)?.burden)
        .any(|b| b.special == Burden::SECOND_WIND)
}

/// The focus regeneration a worn [`Burden::FOCUS_CHARM`] sets: the float
/// with bits `0x38D28EB4` (about 1.004e-4), added to the regeneration rate.
pub const FOCUS_CHARM_REGEN: f32 = f32::from_bits(0x38d2_8eb4);

/// A worn item whose special kind registers an item effect on its wearer
/// (kinds 3, 4 and 6): the kind, the id the record wears it by, and its
/// crystal's flag and charge bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectItem {
    pub special: u8,
    pub id: u32,
    pub power_flags: u8,
    pub charge: u8,
}

/// What a character's worn items' special kinds do to it, as the armour
/// pass dispatches them. The pass first clears every result (no second
/// wind, no ward, no focus bonus, and the item effects of worn items gone
/// from the body's list), then walks the worn items in the record's order;
/// kinds 0 and 1 do nothing here (kind 1, seeing in the dark, is read by
/// the light switch).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WornSpecials {
    /// Kind 2: the second-wind flag.
    pub second_wind: bool,
    /// The items that register an item effect, in wearing order: kind 3
    /// the healing tick (only on a body that takes remedies, its rule bit
    /// 2), kind 4 the golem suit's light tick, kind 6 the gauntlets'
    /// apparel tick.
    pub effects: Vec<EffectItem>,
    /// Kind 5: the mind ward. The body's masked bit is set (other minds see
    /// it as a mind-masked character), its item word's bit 0x10 refuses
    /// every power it casts, and every power running on it is told to end.
    pub mind_ward: bool,
    /// Kind 7: what the focus regeneration rate gains
    /// ([`FOCUS_CHARM_REGEN`], set rather than added: two charms give the
    /// same), 0 without one.
    pub focus_regen: f32,
}

impl WornSpecials {
    /// The dispatch over `worn`, each item's id and burden in the record's
    /// order.
    pub fn of_worn(worn: impl IntoIterator<Item = (u32, Burden)>) -> Self {
        let mut specials = Self::default();
        for (id, burden) in worn {
            match burden.special {
                Burden::SECOND_WIND => specials.second_wind = true,
                Burden::HEALING | Burden::GOLEM_SUIT | Burden::LIGHT_SHIELD => {
                    specials.effects.push(EffectItem {
                        special: burden.special,
                        id,
                        power_flags: burden.power_flags,
                        charge: burden.charge,
                    })
                }
                Burden::MIND_WARD => specials.mind_ward = true,
                Burden::FOCUS_CHARM => specials.focus_regen = FOCUS_CHARM_REGEN,
                _ => {}
            }
        }
        specials
    }

    /// The dispatch over a record's worn items, found by `find` as
    /// [`of_record`] finds them; ids that find nothing, and items without
    /// a body, do nothing.
    pub fn of_record<'a>(record: &CharacterRecord, find: impl Fn(u32) -> Option<&'a Item>) -> Self {
        Self::of_worn(
            record
                .worn
                .iter()
                .filter_map(|worn| Some((worn.id, find(worn.id)?.burden?))),
        )
    }
}

/// The encumbrance a character's record spawns with: its worn items and
/// the shield it holds, found by `find` (an id of the level's or the
/// actor's own database, or of the game's), and its techniques. Ids that
/// find nothing, and items without a body, weigh nothing.
pub fn of_record<'a>(record: &CharacterRecord, find: impl Fn(u32) -> Option<&'a Item>) -> f32 {
    let worn: Vec<Burden> = record
        .worn
        .iter()
        .filter_map(|worn| find(worn.id)?.burden)
        .collect();
    let shield = held_shield(record, find);
    encumbrance(&worn, shield.as_ref(), Skills::of_record(record))
}

/// The record of the shield a record holds: of the first two held items,
/// the last that is a shield and not a weapon.
pub fn held_shield<'a>(
    record: &CharacterRecord,
    find: impl Fn(u32) -> Option<&'a Item>,
) -> Option<ShieldRecord> {
    record.hands[..2]
        .iter()
        .filter(|hand| hand.is_some())
        .filter_map(|hand| find(hand.id))
        .filter(|item| item.weapon.is_none())
        .filter_map(|item| item.shield)
        .last()
}

/// The bone whose layer the armour pass measures for the head's extra
/// height: the eleventh, index 10 in a coverage table that starts at bone 1.
const HEAD_BONE: usize = 10;

/// The head's extra height the armour pass adds to a character (the body's
/// standing-height offset): the thickest display coverage worn over the
/// eleventh bone, in percent, plus the wearer's physique offset
/// (physique − 0.5) × 8, in float in the original's order. A piece counts
/// only when it covers the bone and its body carries the head mark
/// ([`crate::objdb::Armour::head_height_mark`]); a thinner layer never
/// replaces the thickest, and a NaN display never counts.
pub fn head_extra<'a>(worn: impl IntoIterator<Item = &'a Armour>, physique: f32) -> f32 {
    let mut height = 0.0f32;
    for piece in worn {
        if piece.coverage[HEAD_BONE] == 0 || !piece.head_height_mark {
            continue;
        }
        let display = f32::from(piece.display);
        if display > height {
            height = display;
        }
    }
    (physique - 0.5) * 8.0 + height
}

/// The most the run blend reaches: 1 − 0.3·encumbrance, 1 − 0.4·encumbrance
/// sprinting.
pub fn run_cap(encumbrance: f32, sprinting: bool) -> f32 {
    let per = if sprinting { 0.4 } else { 0.3 };
    (1.0 - f64::from(encumbrance) * per) as f32
}

/// Factor on the sprint gait's clip clock: 1 − 0.1·encumbrance.
pub fn sprint_clock(encumbrance: f32) -> f32 {
    (1.0 - f64::from(encumbrance) * 0.1) as f32
}

/// Factor on the body's turn rate toward its heading (out of the free
/// walk): 1 − 0.6·encumbrance.
pub fn turn_rate(encumbrance: f32) -> f32 {
    (1.0 - f64::from(encumbrance) * 0.6) as f32
}

/// The alignment a standing body settles to: 1 − encumbrance.
pub fn standing_alignment(encumbrance: f32) -> f32 {
    1.0 - encumbrance
}

/// A long step (a held step key, the feet's long steps), in step
/// reaches: 1.5 − 0.3·encumbrance.
pub fn long_step(encumbrance: f32) -> f32 {
    (1.5 - f64::from(encumbrance) * 0.3) as f32
}

/// A long step's length, in `reach`'s unit: (1.5 − 0.3·encumbrance) ×
/// `reach`, one double product narrowed once.
pub fn long_step_length(reach: f32, encumbrance: f32) -> f32 {
    ((1.5 - f64::from(encumbrance) * 0.3) * f64::from(reach)) as f32
}

/// The AI's long footwork step, in step reaches: 1.6 − 0.3·encumbrance.
pub fn footwork_long_step(encumbrance: f32) -> f32 {
    (1.6 - f64::from(encumbrance) * 0.3) as f32
}

/// Factor on the clip clock's base while getting up: 1.5 − 0.5·encumbrance.
pub fn get_up_rate(encumbrance: f32) -> f32 {
    1.5 - encumbrance * 0.5
}

#[cfg(test)]
#[path = "encumbrance_test.rs"]
mod tests;
