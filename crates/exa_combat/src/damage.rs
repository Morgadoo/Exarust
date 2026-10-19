//! The original's damage model: a weapon's stat record, the two armour
//! layers of each body part, and what a blow does through them (impact,
//! slash, crush, pierce), to the health bars and the victim's wits.
//! Rules and constants: `docs/formats/OTHER.md` (Weapon record, Armour,
//! Damage).
//!
//! Units follow the original: lengths in cm, speeds in cm per body tick.

use exa_formats::objdb::Armour;
use exa_formats::races::Natural;
#[cfg(test)]
use exa_formats::weapon_stats::{StatBytes, at};

/// The record a held weapon fights with, and its flags.
pub use exa_formats::weapon_record::{WeaponRecord, flags};

/// The health bars a blow takes from, kept with the fighter
/// ([`crate::fighter`]).
pub use crate::fighter::{DEATH, Health};

/// One armour layer of a body part.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Layer {
    /// Share of the part covered, 0..1.
    pub coverage: f32,
    pub padding: f32,
    pub slash: f32,
    pub crush: f32,
    pub pierce: f32,
    /// Hit sound class.
    pub material: u8,
}

/// The two armour layers of a body part: inner and outer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartArmour {
    pub inner: Layer,
    pub outer: Option<Layer>,
}

impl Default for PartArmour {
    fn default() -> Self {
        Self {
            inner: Layer {
                coverage: 1.0,
                ..Layer::default()
            },
            outer: None,
        }
    }
}

/// Effective coverage of a layer folded under another.
fn effective(c: f32) -> f32 {
    if c <= 0.5 {
        c
    } else {
        c + 1.75 * (1.0 - c) * (c - 0.5)
    }
}

/// A resistance byte's share: 1/140.
const PER_RESISTANCE: f32 = 1.0 / 140.0;

/// The share of a part an item covering `percent` of it covers: 0.01 per
/// percent; with Deflection learnt c + 0.1 up to half, else c + 0.1·(1 − c)
/// + 0.05; at most all of it.
pub fn coverage(percent: u8, deflection: bool) -> f32 {
    let c = (f64::from(percent) * 0.01) as f32;
    let c = match (deflection, c > 0.5) {
        (false, _) => c,
        (true, false) => (f64::from(c) + 0.1) as f32,
        (true, true) => (f64::from(1.0 - c) * 0.1 + (f64::from(c) + 0.05)) as f32,
    };
    c.min(1.0)
}

/// Armour of every body part (bones 1..18) from the race's natural layer and
/// the worn items in order, their protection as the items store it (an item
/// made from a template stores it scaled by its condition already), with
/// Deflection when it is learnt.
pub fn armour_layers(
    natural: Option<&Natural>,
    worn: &[Armour],
    deflection: bool,
) -> [PartArmour; 18] {
    let mut parts = [PartArmour::default(); 18];
    if let Some(n) = natural {
        for (part, &c) in parts.iter_mut().zip(&n.coverage) {
            let c = c as f32 / 100.0;
            part.inner.padding = c * n.padding as f32 * 0.005;
            part.inner.slash = c * n.slash as f32 / 140.0;
            part.inner.crush = c * n.crush as f32 / 140.0;
            part.inner.pierce = c * n.pierce as f32 / 140.0;
        }
    }
    for item in worn {
        // Padding × 0.005 in double, resistances × 1/140 in float.
        let resist = |b: u8| f32::from(b) * PER_RESISTANCE;
        let layer = |c: u8| Layer {
            coverage: coverage(c, deflection),
            padding: (f64::from(item.padding) * 0.005) as f32,
            slash: resist(item.slash),
            crush: resist(item.crush),
            pierce: resist(item.pierce),
            material: item.material,
        };
        for (part, &c) in parts.iter_mut().zip(&item.coverage) {
            if c == 0 {
                continue;
            }
            if let Some(outer) = part.outer {
                let inner = part.inner;
                let (g0, g1) = (effective(inner.coverage), effective(outer.coverage));
                let fold = |s0: f32, s1: f32| {
                    let s0 = g0 * s0;
                    s0 + (1.0 - s0) * g1 * s1
                };
                part.inner = Layer {
                    coverage: 1.0,
                    padding: g1 * outer.padding + g0 * inner.padding,
                    slash: fold(inner.slash, outer.slash),
                    crush: fold(inner.crush, outer.crush),
                    pierce: fold(inner.pierce, outer.pierce),
                    material: if outer.coverage > 0.5 {
                        outer.material
                    } else {
                        inner.material
                    },
                };
            }
            part.outer = Some(layer(c));
        }
    }
    parts
}

/// Damage and stun multipliers of a body part (bone 1..18).
pub fn part_multipliers(bone: usize) -> (f32, f32) {
    match bone {
        3 | 6 => (0.6, 1.0),
        4 | 7 => (0.4, 1.0),
        9 => (1.2, 1.2),
        10 => (1.6, 1.4),
        13 | 16 => (0.5, 0.75),
        17 | 18 => (0.3, 0.6),
        _ => (1.0, 1.0),
    }
}

/// How a blow was dealt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strike {
    /// Speed of the tracked weapon point, cm per body tick.
    pub speed: f32,
    pub stab: bool,
    pub overhead: bool,
    /// Progress of the attack, 0..1.
    pub clock: f32,
    /// Where the weapon met the part, cm from the grip along the weapon.
    pub along: f32,
}

/// How the victim meets a blow. Open takes the flesh wound. A parry stops
/// it. A shield lets a leak of yellow through and no lasting wound.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Guard {
    #[default]
    Open,
    Parry,
    /// `leak` is 1 − block/2. `stamina` is the shield arm, 0..=1.
    Shield {
        leak: f32,
        stamina: f32,
    },
}

/// The blow a guard lets through. A parry deals none. A shield that is not
/// itself parrying deals yellow 0.05·Q and daze (0.3 − 0.2·stamina)·Q, with
/// Q = (1 − stamina)·(0.4 + 0.4·impact)·leak, and no lasting wound.
pub fn guard_outcome(blow: Outcome, guard: Guard) -> Outcome {
    match guard {
        Guard::Open => blow,
        Guard::Parry => Outcome::default(),
        Guard::Shield { leak, stamina } => {
            let leak = leak.clamp(0.0, 1.0);
            let stamina = stamina.clamp(0.0, 1.0);
            let q = (1.0 - stamina) * (0.4 + 0.4 * blow.impact) * leak;
            Outcome {
                yellow: 0.05 * q,
                daze: (0.3 - 0.2 * stamina).max(0.0) * q,
                material: blow.material,
                ..Outcome::default()
            }
        }
    }
}

/// The float 0.75 a lasting wound is weighed by before the notice.
const NOTICE_WEIGH: f32 = 0.75;
/// The double 0.2 that weighed wound is scaled by.
const NOTICE_PART: f64 = 0.2;
/// The health base the notice measures against.
const NOTICE_BASE: f32 = 0.25;
/// The double 0.05 of that base.
const NOTICE_OF_BASE: f64 = 0.05;

/// Whether a lasting wound, the size before the red bar's 0.75, is deep
/// enough to be noticed. The wound times 0.75 is widened and times 0.2.
/// That is greater than 0.25 widened times 0.05. An equal product, and a
/// non-number, are not noticed.
pub fn notices_damage(lasting: f32) -> bool {
    let weighed = f64::from(lasting * NOTICE_WEIGH) * NOTICE_PART;
    let line = f64::from(NOTICE_BASE) * NOTICE_OF_BASE;
    weighed > line
}

/// What a blow does.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Outcome {
    /// Damage to the red (lasting) and yellow (current) bars, before their
    /// weights.
    pub red: f32,
    /// The lasting damage on the struck part, its multiplier applied, before
    /// the red bar's 0.75: the size R of the wound the blow leaves, which
    /// bleeds above 0.05 ([`crate::wounds`]).
    pub wound: f32,
    pub yellow: f32,
    /// Share of the victim's wits taken.
    pub daze: f32,
    /// Impact left after the armour: what shoves the victim, before its
    /// readiness lessens the wound.
    pub impact: f32,
    /// Penetration through the armour, lessened by the victim's readiness.
    pub penetration: f32,
    /// How far the weapon bites in (it sticks above 0.1).
    pub bite: f32,
    /// The armour layer's hit sound class.
    pub material: u8,
}

/// Resolves a blow of `weapon` on body part `bone` with its `armour`;
/// `roll` is a uniform random number in 0..1 for the armour's coverage.
/// `max_health` and `readiness` (0..=1, [`crate::Fighter::readiness`]) are
/// the victim's: readiness g scales the wound's penetration, impact and
/// stab energy by 1 − 0.5·g, after the shove's impact is taken. Returns
/// `None` when the contact is past the weapon's tip.
pub fn resolve(
    weapon: &WeaponRecord,
    strike: &Strike,
    armour: &PartArmour,
    bone: usize,
    max_health: f32,
    readiness: f32,
    roll: f32,
) -> Option<Outcome> {
    let length = weapon.length.max(1.0);
    let p = (strike.along / length).max(0.0);
    if p >= 1.0 {
        return None;
    }
    // Speed: capped, softened when slow.
    let mut v = strike.speed.min(30.0);
    if v < 10.0 {
        let s = v / 10.0;
        v = 10.0 * (2.0 - s) * s * s;
    }
    if strike.overhead {
        v *= 1.5;
    }
    let u = 0.05 * v;
    let w = weapon;
    let swing_impact = (w.impact + 0.3) * 1.1 * u.powf(1.0 - 0.5 * w.impact);

    let (mut slash, mut crush, mut pierce) = (0.0, 0.0, 0.0);
    let mut draw = 0.0;
    let (impact, mut stab_energy) = if strike.stab {
        let tip = ((14.0 - (length - strike.along)) / 7.0).clamp(0.0, 1.0);
        let lunge = (1.0 - 2.8 * (1.4 - 2.0 * strike.clock).powi(2)).max(0.0);
        let stab_impact = 0.3 * u * (w.impact + 0.5 + w.weight);
        let mut t = tip * (0.3 * w.weight + 0.25) * (stab_impact + lunge);
        if w.flags & flags::TWO_HANDS != 0 {
            t *= 1.5;
        }
        ((0.25 * w.impact + 0.75) * t, t)
    } else {
        let (z, zeta, haft) = zones(w, p, strike.clock, &mut draw);
        slash = w.slash * z;
        crush = w.crush * z;
        pierce = w.pierce * z;
        let haft_bonus = match w.flags >> flags::HAFT_SHIFT & 3 {
            2 => 0.2,
            3 => 0.15,
            _ => 0.1,
        };
        crush += (1.0 - z) * if haft && z < 0.5 { haft_bonus } else { 0.15 };
        let mut i = swing_impact * zeta;
        if !w.armed() {
            i = 0.6 * (3.0 * i).sqrt();
        }
        (i, 0.0)
    };

    // Armour: one roll decides how much of each layer the blow meets.
    let (mut pad, mut rs, mut rc, mut rp) = (0.0, 0.0, 0.0, 0.0);
    let mut material = 0;
    for layer in armour.outer.iter().chain(std::iter::once(&armour.inner)) {
        let weight = if roll < layer.coverage {
            1.0
        } else if layer.coverage > 0.5 {
            1.75 * (layer.coverage - 0.5)
        } else {
            continue;
        };
        if material == 0 {
            material = layer.material;
        }
        pad += weight * layer.padding;
        rs += (1.0 - rs) * weight * layer.slash;
        rc += (1.0 - rc) * weight * layer.crush;
        rp += (1.0 - rp) * weight * layer.pierce * (0.5 + 0.5 * roll);
    }

    let mut total = 1.25 * impact;
    let mut bite = 0.1 * impact;
    let mut j = (impact - 0.2 * pad).max(0.0) / (pad * pad + pad + 1.0);
    let tiny = 1e-6;
    if strike.stab {
        let half = pad / 2.0;
        stab_energy /= half * half + half + 1.0;
        slash = 0.0;
        crush = 0.0;
        pierce = if w.thrust > 0.0 {
            if stab_energy < 2.0 {
                if j < tiny {
                    0.0
                } else {
                    w.thrust * 1.58 * stab_energy.sqrt()
                }
            } else {
                w.thrust * 1.58 * 2f32.sqrt()
            }
        } else {
            0.0
        };
        bite += (2.0 * pierce - rp).max(0.0);
    } else {
        if crush > 0.0 {
            crush = if j < tiny {
                0.0
            } else {
                crush * 1.58 * j.powf(2.0 - crush)
            };
            total += 0.75 * crush;
            bite += (0.5 * crush - 0.25 * rc).max(0.0);
            crush = ((1.0 - 0.75 * rc) * (crush - 0.1 * rc)).max(0.0);
        }
        if slash > 0.0 {
            let k = slash / (10.0 * slash * rs + 1.0);
            let chop = if impact < tiny {
                0.0
            } else {
                ((0.8 - k) * impact * impact + 2.5 * impact / (impact + 1.0 - k)) * k - 0.2 * rs
            };
            bite += ((1.0 - draw) * chop).max(0.0);
            let drawn = if u < tiny {
                0.0
            } else {
                ((1.0 - k) * u + u.sqrt()) * k - 0.3 * rs
            };
            bite += (0.5 * draw * drawn).max(0.0);
            slash = (draw * drawn + (1.0 - draw) * chop).max(0.0);
            total += 0.75 * slash;
        }
        if pierce > 0.0 {
            pierce *= 1.58 * j.powf(1.0 + pierce / 2.0);
            bite += (pierce - 0.5 * rp).max(0.0);
        }
        bite *= 0.5;
    }
    j = (j / ((2.0 * j - 0.2) * pad + 1.0)).max(0.0);
    total += 0.75 * pierce;
    let _ = total;
    pierce = (pierce - 0.75 * rp).max(0.0);

    let shove = j;
    // The victim's readiness lessens the wound, not the shove.
    let defended = 1.0 - 0.5 * readiness.clamp(0.0, 1.0);
    let penetration = (0.5 * (j - 0.8).max(0.0) + slash + pierce + crush) * defended;
    let j = j * defended;
    let stab_energy = stab_energy * defended;
    let (mult, stun) = part_multipliers(bone);
    let lasting = penetration / (penetration + 1.0);
    let wound = lasting * mult;
    let red = wound * 0.75;
    let yellow = (0.5 * stab_energy + (2.0 * (penetration - lasting) + j) * mult) * 0.75;
    let daze = (0.75 * penetration + 1.25 * j).sqrt() * stun * 0.5 * (0.25 / max_health.max(1e-3));
    Some(Outcome {
        red,
        wound,
        yellow,
        daze,
        impact: shove,
        penetration,
        bite,
        material,
    })
}

/// On-target share `z`, impact multiplier `ζ` and whether the haft took the
/// blow, for a swing meeting the weapon at fraction `p` of its length; sets
/// the draw-cut share.
fn zones(w: &WeaponRecord, p: f32, clock: f32, draw: &mut f32) -> (f32, f32, bool) {
    if !w.armed() {
        return (1.0, 1.0, false);
    }
    let length = w.length;
    let (mut z, mut zeta, mut haft) = (1.0, 1.0, false);
    if p <= w.end {
        if p < w.start {
            let d = (w.start - p) * length;
            z = ((20.0 - d) * 0.05).max(0.0);
            zeta = p / w.start.max(1e-6);
            haft = true;
        }
    } else {
        z = ((20.0 - (p - w.end) * length) * 0.05).max(0.0);
        let q = (p - w.end) / (1.0 - w.end).max(1e-6);
        zeta = 1.0 - 0.75 * q * q;
        *draw = q * (1.0 - 10.0 * (0.8 - clock).powi(2)).max(0.0);
    }
    let edge = |from: f32, near: f32, beyond: f32, draw: &mut f32| {
        if p <= from {
            let r = (from - p) * near;
            *draw = 0.0;
            1.0 - (2.0 - r) * r * r
        } else {
            let q = (p - from) * beyond;
            *draw = q;
            1.0 - 0.75 * q * q
        }
    };
    match w.kind() {
        flags::BLADED => zeta = edge(0.6, 1.0 / 0.6, 1.0 / 0.4, draw),
        flags::STICK => {
            zeta = edge(0.8, 1.25, 5.0, draw);
            haft = false;
        }
        _ if w.code == 2 => {
            let sweet = w.start + 0.6 * (w.end - w.start);
            if p <= sweet {
                zeta = 1.0 - (sweet - p) / sweet.max(1e-6);
            } else {
                let q = (p - sweet) / (1.0 - sweet).max(1e-6);
                zeta = 1.0 - 0.75 * q * q;
                *draw = q;
            }
        }
        _ => {}
    }
    (z, zeta, haft)
}

/// A one-handed sword's record, 90 cm long: the tests' worked example.
#[cfg(test)]
pub(crate) fn sword_a() -> WeaponRecord {
    let mut s = StatBytes::default();
    for (off, v) in [
        (at::WEIGHT, 50),
        (at::BALANCE, 80),
        (at::IMPACT, 127),
        (at::THRUST, 54),
        (at::KIND, 2),
        (at::START, 20),
        (at::END, 200),
        (at::SLASH, 72),
    ] {
        s.set(off, v);
    }
    WeaponRecord::of_one_part(0x11, &s, false, 90.0)
}

/// The combatant's hit step's inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CombatHitInput {
    pub blocked: bool,
    pub helpless: bool,
    pub first: bool,
    pub current: f32,
    pub stat: f32,
    pub target_flag: u32,
    pub target_hit: f32,
    pub threshold: f32,
    pub body_flag: u32,
}

/// The combatant's hit step's results.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CombatHit {
    pub returned: bool,
    pub value: f32,
    pub target_hit: f32,
    pub target_flag: u32,
    pub special_call: bool,
}

/// Scales a first hit and marks the target once.
pub fn apply_combat_hit(input: CombatHitInput) -> CombatHit {
    if input.blocked || input.helpless {
        return CombatHit {
            returned: true,
            value: input.current,
            target_hit: input.target_hit,
            target_flag: input.target_flag,
            special_call: false,
        };
    }
    let value = if input.first {
        (f64::from(input.current) * ((1.0 + f64::from(input.stat)) * 0.025 + 0.95)) as f32
    } else {
        input.current
    };
    let mut target_hit = input.target_hit;
    let mut target_flag = input.target_flag;
    if (target_flag & 0x10_0000) == 0 {
        if target_hit < input.threshold {
            target_hit = input.threshold;
        }
        target_flag |= 0x10_0000;
    }
    CombatHit {
        returned: false,
        value,
        target_hit,
        target_flag,
        special_call: (input.body_flag & 0x10_0000) != 0,
    }
}

/// The combat wake's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatWake {
    pub returned: bool,
    pub body_word: u32,
    pub wake_calls: u32,
    pub target_calls: u32,
}

/// Clears the wake bits and calls every listed target.
pub fn wake_combat_targets(wake: bool, body_word: u32, target_count: i32) -> CombatWake {
    if !wake {
        return CombatWake {
            returned: true,
            body_word,
            wake_calls: 0,
            target_calls: 0,
        };
    }
    CombatWake {
        returned: false,
        body_word: body_word & !6,
        wake_calls: 1,
        target_calls: target_count.max(0) as u32,
    }
}

/// The combat forget's inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatForgetInput {
    pub entries: Vec<u64>,
    pub target: u64,
    pub slot_778: u64,
    pub slot_770: u64,
    pub hit: Option<u64>,
    pub contact: Option<u64>,
}

/// The combat forget's results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatForget {
    pub entries: Vec<u64>,
    pub count: i32,
    pub removed: bool,
    pub moves: u32,
    pub slot_778: u64,
    pub slot_770: u64,
    pub hit: Option<u64>,
    pub contact: Option<u64>,
}

/// Removes the target from the counted list and clears the matching slots.
pub fn forget_combat_target(input: CombatForgetInput) -> CombatForget {
    let mut entries = input.entries.clone();
    let removed = if let Some(index) = entries.iter().position(|entry| *entry == input.target) {
        let last = entries.pop().unwrap();
        if index < entries.len() {
            entries[index] = last;
        }
        true
    } else {
        false
    };
    CombatForget {
        count: entries.len() as i32,
        entries,
        removed,
        moves: u32::from(removed),
        slot_778: if input.slot_778 == input.target {
            0
        } else {
            input.slot_778
        },
        slot_770: if input.slot_770 == input.target {
            0
        } else {
            input.slot_770
        },
        hit: input
            .hit
            .map(|value| if value == input.target { 0 } else { value }),
        contact: input
            .contact
            .map(|value| if value == input.target { 0 } else { value }),
    }
}

/// The combatant reset's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatResetInput {
    pub has_marker: bool,
    pub has_reference: bool,
    pub reference_count: i32,
}

/// The combatant reset's fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatReset {
    pub count: i32,
    pub slot_770: u64,
    pub slot_7a0: i32,
    pub field_200: i32,
    pub slot_788: u64,
    pub slot_798: u64,
    pub body_e38: u64,
    pub body_e50: u64,
    pub field_158: i32,
    pub byte_87e: u8,
    pub marker_cleared: bool,
    pub reference_count: i32,
}

/// Zeroes the combatant's transient state and its stored targets.
pub fn reset_combatant(input: CombatResetInput) -> CombatReset {
    CombatReset {
        count: 0,
        slot_770: 0,
        slot_7a0: -1,
        field_200: 0,
        slot_788: 0,
        slot_798: 0,
        body_e38: 0,
        body_e50: 0,
        field_158: 0,
        byte_87e: 0,
        marker_cleared: input.has_marker,
        reference_count: if input.has_reference {
            input.reference_count - 1
        } else {
            input.reference_count
        },
    }
}

/// The combatant's two measures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CombatMeasures {
    pub reach: f32,
    pub scaled: f32,
}

/// Stores the reach and the body value times 1.0.
pub fn refresh_combat_measures(reach: f32, body_value: f32) -> CombatMeasures {
    CombatMeasures {
        reach,
        scaled: body_value * 1.0,
    }
}

/// The combat replace's inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatReplaceInput {
    pub entries: Vec<u64>,
    pub old: u64,
    pub new: u64,
    pub new_hit: u64,
    pub slot_778: u64,
    pub slot_770: u64,
    pub hit: Option<u64>,
    pub contact: Option<u64>,
}

/// The combat replace's results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatReplace {
    pub entries: Vec<u64>,
    pub replaced: bool,
    pub slot_778: u64,
    pub slot_770: u64,
    pub hit: Option<u64>,
    pub contact: Option<u64>,
}

/// Swaps an old target for a new one in the list and the stored slots.
pub fn replace_combat_target(input: CombatReplaceInput) -> CombatReplace {
    let mut entries = input.entries.clone();
    let replaced = if let Some(index) = entries.iter().position(|entry| *entry == input.old) {
        entries[index] = input.new;
        true
    } else {
        false
    };
    CombatReplace {
        entries,
        replaced,
        slot_778: if input.slot_778 == input.old {
            input.new
        } else {
            input.slot_778
        },
        slot_770: if input.slot_770 == input.old {
            input.new
        } else {
            input.slot_770
        },
        hit: input.hit.map(|value| {
            if value == input.old {
                input.new_hit
            } else {
                value
            }
        }),
        contact: input
            .contact
            .map(|value| if value == input.old { input.new } else { value }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sword_a_record() {
        let w = sword_a();
        assert!((w.impact - 0.42).abs() < 1e-5);
        assert!((w.slash - 0.397).abs() < 1e-3);
        assert!((w.thrust - 0.325).abs() < 1e-3);
        assert!((w.start - 0.1).abs() < 1e-6 && (w.end - 1.0).abs() < 1e-6);
    }

    fn cut(armour: &PartArmour, roll: f32) -> Outcome {
        let strike = Strike {
            speed: 20.0,
            stab: false,
            overhead: false,
            clock: 0.8,
            along: 70.0,
        };
        resolve(&sword_a(), &strike, armour, 9, 0.25, 0.0, roll).unwrap()
    }

    #[test]
    fn armour_stops_the_edge() {
        let bare = cut(&PartArmour::default(), 0.5);
        assert!(bare.penetration > 0.3, "{bare:?}");
        let plate = Armour {
            coverage: [100; 18],
            padding: 60,
            slash: 140,
            crush: 60,
            pierce: 80,
            encumbrance: 10,
            material: 4,
        };
        let layers = armour_layers(None, &[plate], false);
        let armoured = cut(&layers[8], 0.5);
        assert!(
            armoured.penetration < bare.penetration * 0.3,
            "{armoured:?}"
        );
        assert_eq!(armoured.material, 4);
    }

    /// A slower blow penetrates less, and a resisting layer less than bare flesh.
    #[test]
    fn speed_and_a_layer_change_the_wound() {
        let bare = PartArmour::default();
        let at = |speed: f32| {
            resolve(
                &sword_a(),
                &Strike {
                    speed,
                    stab: false,
                    overhead: false,
                    clock: 0.8,
                    along: 70.0,
                },
                &bare,
                9,
                0.25,
                0.0,
                0.5,
            )
            .unwrap()
        };
        let slow = at(5.0);
        let fast = at(20.0);
        assert!(slow.impact < fast.impact, "{slow:?} {fast:?}");
        assert!(slow.penetration < fast.penetration, "{slow:?} {fast:?}");
        let plate = armour_layers(
            None,
            &[Armour {
                coverage: [100; 18],
                padding: 60,
                slash: 140,
                crush: 60,
                pierce: 80,
                encumbrance: 10,
                material: 4,
            }],
            false,
        );
        let armoured = resolve(
            &sword_a(),
            &Strike {
                speed: 20.0,
                stab: false,
                overhead: false,
                clock: 0.8,
                along: 70.0,
            },
            &plate[8],
            9,
            0.25,
            0.0,
            0.5,
        )
        .unwrap();
        assert!(armoured.penetration < fast.penetration, "{armoured:?}");
    }

    /// Readiness 1 halves the wound (penetration, the red and yellow bars,
    /// the daze's terms) but not the impact that shoves; 0.5 takes a
    /// quarter off.
    #[test]
    fn readiness_halves_the_wound() {
        let strike = Strike {
            speed: 20.0,
            stab: false,
            overhead: false,
            clock: 0.8,
            along: 70.0,
        };
        let bare = PartArmour::default();
        let at = |g| resolve(&sword_a(), &strike, &bare, 9, 0.25, g, 0.5).unwrap();
        let (open, ready, half) = (at(0.0), at(1.0), at(0.5));
        assert!(open.penetration > 0.3, "{open:?}");
        assert!((ready.penetration - 0.5 * open.penetration).abs() < 1e-5);
        assert!((half.penetration - 0.75 * open.penetration).abs() < 1e-5);
        assert_eq!(ready.impact, open.impact);
        assert!(ready.red < open.red && ready.yellow < open.yellow);
        assert!((ready.daze - open.daze * 0.5f32.sqrt()).abs() < 1e-5);
        // A stab's energy too.
        let stab = Strike {
            stab: true,
            clock: 0.5,
            along: 85.0,
            ..strike
        };
        let s = |g| resolve(&sword_a(), &stab, &bare, 9, 0.25, g, 0.5).unwrap();
        let (open, ready) = (s(0.0), s(1.0));
        assert!(open.penetration > 0.0, "{open:?}");
        assert!((ready.penetration - 0.5 * open.penetration).abs() < 1e-5);
        assert!(ready.yellow < 0.75 * open.yellow, "{ready:?} {open:?}");
    }

    /// The tests' sword, 90 cm striking from 10 % to the tip, in a human's
    /// hand: 60 + 90·(0.1 + 0.6·0.9) = 117.6 cm.
    #[test]
    fn reach_is_60_cm_plus_the_sweet_spot() {
        assert!((sword_a().reach_metres() - 1.176).abs() < 1e-5);
    }

    #[test]
    fn past_the_tip_is_no_hit() {
        let strike = Strike {
            speed: 20.0,
            stab: false,
            overhead: false,
            clock: 0.5,
            along: 95.0,
        };
        assert!(
            resolve(
                &sword_a(),
                &strike,
                &PartArmour::default(),
                9,
                0.25,
                0.0,
                0.5
            )
            .is_none()
        );
    }

    #[test]
    fn layers_fold_inwards() {
        let shirt = Armour {
            coverage: [80; 18],
            padding: 10,
            slash: 70,
            crush: 10,
            pierce: 30,
            encumbrance: 5,
            material: 3,
        };
        let parts = armour_layers(None, &[shirt, shirt], false);
        let p = parts[0];
        assert_eq!(p.inner.coverage, 1.0);
        assert_eq!(p.inner.material, 3);
        // The first shirt folds in at its effective coverage 0.905.
        assert!((p.inner.slash - 0.905 * 0.5).abs() < 1e-5);
        assert!(p.outer.is_some());
    }

    /// Deflection: 30 % covers 0.4, 75 % covers 0.825 and 95 % all of it;
    /// without it the percentages as they are.
    #[test]
    fn deflection_covers_more() {
        assert_eq!(coverage(30, false), 0.3);
        assert!((coverage(30, true) - 0.4).abs() < 1e-6);
        assert!((coverage(75, true) - 0.825).abs() < 1e-6);
        assert_eq!(coverage(95, true), 1.0);
        assert_eq!(coverage(100, false), 1.0);
        let shirt = Armour {
            coverage: [75; 18],
            padding: 10,
            slash: 70,
            crush: 10,
            pierce: 30,
            encumbrance: 5,
            material: 3,
        };
        let outer = armour_layers(None, &[shirt], true)[8].outer.unwrap();
        assert!((outer.coverage - 0.825).abs() < 1e-6);
    }

    /// The armour pass reads what an item stores: trousers made at quality
    /// 160 and damage 160 store padding 17 (their template's 20 scaled by
    /// their condition) and keep 17 × 0.005.
    #[test]
    fn stored_protection_is_not_scaled_again() {
        let trousers = Armour {
            coverage: [50; 18],
            padding: 17,
            slash: 17,
            crush: 17,
            pierce: 17,
            encumbrance: 11,
            material: 1,
        };
        let outer = armour_layers(None, &[trousers], false)[1].outer.unwrap();
        assert!((outer.padding - 0.085).abs() < 1e-7);
        assert!((outer.slash - 17.0 / 140.0).abs() < 1e-7);
    }

    #[test]
    fn the_first_hit_scales_and_marks_its_target() {
        let input = |first, stat, target_flag| CombatHitInput {
            blocked: false,
            helpless: false,
            first,
            current: 10.0,
            stat,
            target_flag,
            target_hit: 3.0,
            threshold: 5.0,
            body_flag: 0,
        };

        let plain = apply_combat_hit(input(true, 0.0, 0));
        assert!(!plain.returned);
        assert!((plain.value - 9.75).abs() < 1e-6);
        assert_eq!(plain.target_hit, 5.0);
        assert_eq!(plain.target_flag, 0x10_0000);

        assert!((apply_combat_hit(input(true, 1.0, 0)).value - 10.0).abs() < 1e-6);
        assert_eq!(apply_combat_hit(input(false, 1.0, 0)).value, 10.0);

        let marked = apply_combat_hit(input(true, 0.0, 0x10_0000));
        assert_eq!(marked.target_hit, 3.0);
        assert_eq!(marked.target_flag, 0x10_0000);

        let blocked = apply_combat_hit(CombatHitInput {
            blocked: true,
            ..input(true, 0.0, 0)
        });
        assert!(blocked.returned);
        assert_eq!(blocked.value, 10.0);
        assert_eq!(blocked.target_hit, 3.0);

        let special = apply_combat_hit(CombatHitInput {
            body_flag: 0x10_0000,
            ..input(true, 0.0, 0)
        });
        assert!(special.special_call);
    }

    #[test]
    fn the_wake_clears_the_bits_and_calls_targets() {
        let woken = wake_combat_targets(true, 0x7, 3);
        assert!(!woken.returned);
        assert_eq!(woken.body_word, 0x1);
        assert_eq!(woken.wake_calls, 1);
        assert_eq!(woken.target_calls, 3);

        let quiet = wake_combat_targets(false, 0x7, 3);
        assert!(quiet.returned);
        assert_eq!(quiet.body_word, 0x7);
        assert_eq!(quiet.wake_calls, 0);
        assert_eq!(quiet.target_calls, 0);
    }

    #[test]
    fn the_forget_swaps_and_clears() {
        let input = |entries, target| CombatForgetInput {
            entries,
            target,
            slot_778: target,
            slot_770: 9,
            hit: Some(target),
            contact: Some(7),
        };

        let middle = forget_combat_target(input(vec![1, 2, 3], 2));
        assert!(middle.removed);
        assert_eq!(middle.entries, vec![1, 3]);
        assert_eq!(middle.count, 2);
        assert_eq!(middle.moves, 1);
        assert_eq!(middle.slot_778, 0);
        assert_eq!(middle.slot_770, 9);
        assert_eq!(middle.hit, Some(0));
        assert_eq!(middle.contact, Some(7));

        let last = forget_combat_target(input(vec![1, 2, 3], 3));
        assert_eq!(last.entries, vec![1, 2]);
        assert_eq!(last.count, 2);

        let absent = forget_combat_target(input(vec![1, 2, 3], 9));
        assert!(!absent.removed);
        assert_eq!(absent.entries, vec![1, 2, 3]);
        assert_eq!(absent.count, 3);
        assert_eq!(absent.moves, 0);
        assert_eq!(absent.slot_778, 0);
    }

    #[test]
    fn the_combatant_resets_and_measures() {
        let reset = reset_combatant(CombatResetInput {
            has_marker: true,
            has_reference: true,
            reference_count: 3,
        });
        assert_eq!(reset.count, 0);
        assert_eq!(reset.slot_770, 0);
        assert_eq!(reset.slot_7a0, -1);
        assert_eq!(reset.field_200, 0);
        assert_eq!(reset.slot_788, 0);
        assert_eq!(reset.slot_798, 0);
        assert_eq!(reset.body_e38, 0);
        assert_eq!(reset.body_e50, 0);
        assert_eq!(reset.field_158, 0);
        assert_eq!(reset.byte_87e, 0);
        assert!(reset.marker_cleared);
        assert_eq!(reset.reference_count, 2);

        let quiet = reset_combatant(CombatResetInput {
            has_marker: false,
            has_reference: false,
            reference_count: 3,
        });
        assert!(!quiet.marker_cleared);
        assert_eq!(quiet.reference_count, 3);
        assert_eq!(quiet.slot_7a0, -1);

        let measures = refresh_combat_measures(12.5, 0.75);
        assert_eq!(measures.reach, 12.5);
        assert_eq!(measures.scaled, 0.75);
    }

    #[test]
    fn the_replace_swaps_the_target_everywhere() {
        let input = |entries, old| CombatReplaceInput {
            entries,
            old,
            new: 9,
            new_hit: 0x99,
            slot_778: old,
            slot_770: 7,
            hit: Some(old),
            contact: Some(old),
        };

        let swapped = replace_combat_target(input(vec![1, 2, 3], 2));
        assert!(swapped.replaced);
        assert_eq!(swapped.entries, vec![1, 9, 3]);
        assert_eq!(swapped.slot_778, 9);
        assert_eq!(swapped.slot_770, 7);
        assert_eq!(swapped.hit, Some(0x99));
        assert_eq!(swapped.contact, Some(9));

        let absent = replace_combat_target(input(vec![1, 2, 3], 4));
        assert!(!absent.replaced);
        assert_eq!(absent.entries, vec![1, 2, 3]);
        assert_eq!(absent.slot_778, 9);
        assert_eq!(absent.hit, Some(0x99));
        assert_eq!(absent.contact, Some(9));
    }
}
