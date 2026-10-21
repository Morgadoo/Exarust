//! The original's damage model: a weapon's stat record, the two armour
//! layers of each body part, and what a blow does through them (impact,
//! slash, crush, pierce), to the health bars and the victim's wits.
//! Rules and constants: `docs/formats/OTHER.md` (Weapon record, Armour,
//! Damage).
//!
//! Units follow the original: lengths in cm, speeds in cm per body tick.

use crate::fast_math::fast_pow;
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
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Strike {
    /// Speed of the tracked weapon point, cm per body tick (for a stab,
    /// twice its velocity along the weapon).
    pub speed: f32,
    /// The stab's rules: the stab motion, unless the attacker's race stabs
    /// with both hands (the quadruped's stab is resolved as a swing).
    pub stab: bool,
    pub overhead: bool,
    /// Progress of the attack, 0..1.
    pub clock: f32,
    /// Where the weapon met the part, cm from the grip along the weapon.
    pub along: f32,
    /// The attacker holds no weapon (its weapon class has no hands bits):
    /// it strikes with fists or claws.
    pub unarmed: bool,
    /// The striking body hangs from another striker of the same weapon (a
    /// flail's link); Exarust's weapons are one body, so never.
    pub hanging: bool,
    /// The attacker's Force Strikes strength (no powers in Exarust: 0).
    pub strikes: f32,
}

/// What the struck character brings to a blow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Victim {
    pub max_health: f32,
    /// Readiness g, 0..=1 ([`crate::Fighter::readiness`]): the wound's
    /// penetration, impact and stab energy are taken 1 − 0.5·g.
    pub readiness: f32,
    /// Force Shell: 0.5 while the power holds (no powers in Exarust: 0).
    pub shell: f32,
    /// A blow dazes it a quarter as much and never stalls its swing (rule
    /// bit `BodyRules::STEADFAST`).
    pub steadfast: bool,
    /// The Security perk: the daze's stall factor scales with the blow's
    /// wound against its total.
    pub security: bool,
    /// A potion halves the daze's stall factor.
    pub potion: bool,
}

impl Victim {
    /// A plain victim of `max_health` at readiness `readiness`.
    pub fn new(max_health: f32, readiness: f32) -> Self {
        Self {
            max_health,
            readiness,
            shell: 0.0,
            steadfast: false,
            security: false,
            potion: false,
        }
    }
}

/// How the victim meets a blow. Open takes the flesh wound. A weapon's
/// parry stops it ([`parried`]). A shield takes it on its arm's stamina
/// and lets a leak of yellow through, no lasting wound ([`block`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Guard {
    #[default]
    Open,
    Parry,
    Shield(ShieldGuard),
}

/// A shield held up: its factors and its arm's stamina.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ShieldGuard {
    /// The share of a tired block's blow that reaches yellow: 1 − block/2.
    pub leak: f32,
    /// What a blow costs the arm per unit of impact: the leak, × 2/3 with
    /// Endure.
    pub cost: f32,
    /// The shield arm's stamina, 0..=1.
    pub stamina: f32,
    /// The stamina's regeneration rate. A block zeros it.
    pub rate: f32,
    /// The game clock of the last block, or [`crate::regen::ShieldStamina::STAMP_FULL`]
    /// once the arm has filled.
    pub blocked_at: u32,
    /// The holder parries with the shield.
    pub parrying: bool,
}

/// What the shield's holder brings to a blow on its shield besides the
/// shield: its techniques, its own swing clock, whether it counters this
/// very attack, a potion.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Holding {
    pub synchrony: bool,
    pub persist: bool,
    pub counter: bool,
    /// It counters this very attack's motion.
    pub countering: bool,
    /// Its swing clock.
    pub clock: f32,
    pub potion: bool,
}

/// A blow on a shield: the arm's stamina left, the yellow left, the daze
/// and its stall factor, the sag, whether the holder is locked out of
/// attacking for 600 ms, whether the low-health check runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blocked {
    pub stamina: f32,
    pub yellow: f32,
    pub daze: f32,
    pub daze_factor: f32,
    pub sag: Option<f32>,
    pub locked: bool,
    pub low: bool,
}

/// How long a parried or shield-parried attacker's victim is kept from
/// attacking, ms (none with Riposte, with Counter for a shield).
pub const PARRY_LOCK_MS: u64 = 600;

/// A blow at impact `impact` (at the hit, no armour) on a shield held as
/// `shield` by a holder of yellow health `yellow`, as the original's block
/// path: parrying, the arm pays cost·0.4·I, the holder is locked unless it
/// has Counter, and its stance sags (0.7 − S/2)·I (at most 1); else, under
/// Synchrony while it counters this attack before 0.3 of its own clock,
/// x = clock/0.3 (0 at or under 0) and the arm pays cost·(0.3x + 0.3)·I,
/// otherwise x = 1 and it pays cost·0.8·I. The stamina is at least 0.
/// With x above 0 a tired arm leaks Q = (1 − S)·(0.4·I + 0.4·x)·leak:
/// yellow −= 0.05·Q (0.025 under a potion), at least 0. Yellow under 0.05
/// runs the low-health check. The daze is (0.3 − 0.2·S)·Q (·I when x is
/// 0); its stall factor half the holder's clock with Persist, its clock
/// under Synchrony before 0.3, else 1. Products in double as the original.
pub fn block(impact: f32, yellow: f32, shield: &ShieldGuard, holder: &Holding) -> Blocked {
    let wide_cost = |k: f64| (f64::from(shield.stamina) - f64::from(shield.cost) * k) as f32;
    let (mut stamina, x, sag, locked) = if shield.parrying {
        let stamina = wide_cost(f64::from(impact) * 0.4);
        let sag = ((0.7 - f64::from(stamina * 0.5)) * f64::from(impact)) as f32;
        (
            stamina,
            0.0,
            Some(if sag > 1.0 { 1.0 } else { sag }),
            !holder.counter,
        )
    } else if holder.synchrony && holder.countering && f64::from(holder.clock) < 0.3 {
        let x = if holder.clock > 0.0 {
            (f64::from(holder.clock) * 3.333_333_333_333_333_5) as f32
        } else {
            0.0
        };
        let stamina = wide_cost((f64::from(x) * 0.3 + 0.3) * f64::from(impact));
        (stamina, x, None, false)
    } else {
        (wide_cost(f64::from(impact) * 0.8), 1.0, None, false)
    };
    if 0.0 > stamina {
        stamina = 0.0;
    }
    let mut yellow = yellow;
    let mut q = impact;
    if 0.0 < x {
        let reach = (f64::from(impact) * 0.4 + f64::from(x) * 0.4) as f32;
        q = (1.0 - stamina) * reach * shield.leak;
        let share = if holder.potion { 0.025 } else { 0.05 };
        yellow = (f64::from(yellow) - f64::from(q) * share) as f32;
        if 0.0 > yellow {
            yellow = 0.0;
        }
    }
    let daze_factor = if holder.persist {
        holder.clock * 0.5
    } else if holder.synchrony && f64::from(holder.clock) < 0.3 {
        holder.clock
    } else {
        1.0
    };
    Blocked {
        stamina,
        yellow,
        daze: ((0.3 - f64::from(stamina) * 0.2) * f64::from(q)) as f32,
        daze_factor,
        sag,
        locked,
        low: yellow < LOW_YELLOW,
    }
}

/// Under this yellow the low-health check runs.
const LOW_YELLOW: f32 = 0.05;

/// The sag a weapon's parry sets on its holder: min(I/2, 1).
pub fn parried(impact: f32) -> f32 {
    let sag = impact * 0.5;
    if sag > 1.0 { 1.0 } else { sag }
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
    /// Damage to the red (lasting) and yellow (current) bars, their 0.75
    /// applied: what the damage call takes.
    pub red: f32,
    /// The lasting damage on the struck part, its multiplier applied, before
    /// the red bar's 0.75: the size R of the wound the blow leaves, which
    /// bleeds above 0.05 ([`crate::wounds`]).
    pub wound: f32,
    pub yellow: f32,
    /// Share of the victim's wits taken (before the daze's cap of 0.9).
    pub daze: f32,
    /// The daze's second factor, which a struck attacker's stall takes
    /// (0.25 over the most health; Security, a potion and a steadfast body
    /// change it).
    pub daze_factor: f32,
    /// Impact left after the armour: what shoves the victim, before its
    /// readiness lessens the wound.
    pub impact: f32,
    /// Penetration through the armour, lessened by the victim's readiness.
    pub penetration: f32,
    /// How far the weapon bites in (it sticks above 0.1).
    pub bite: f32,
    /// The armour layer's hit sound class (the first layer met).
    pub material: u8,
    /// The impact at the hit, before the armour: the blow sag (twice it, at
    /// most 1), the shield's leak and the attacker's follow-through read it.
    pub hit_impact: f32,
    /// The strike's speed as the routine took it (capped at 30, softened
    /// under 10, ×1.5 overhead), cm a tick: a recoil's drive and a
    /// follow-through scale with it.
    pub speed: f32,
    /// The blow's total τ: 1.25·I plus three quarters of each channel.
    pub total: f32,
    /// Where on the weapon it struck, for the follow-through: the body zone
    /// (or, `edge`, how far into the tip zone).
    pub zone: f32,
    /// It struck with the edge's tip zone.
    pub edge: bool,
    /// It struck with the body of the weapon (its haft, its mid-blade, a
    /// stab's tip or a flail's link): a hard blow cleaves through.
    pub central: bool,
    /// The haft took it: the haft's material sound plays.
    pub haft: bool,
    /// The bite link's zone smoothing: 0.1 a swing, 0 a stab, 0.5 unarmed.
    pub smoothing: f32,
    /// The blow sag it sets on the victim's body, min(2·I, 1); none when it
    /// met a shield or a weapon.
    pub sag: Option<f32>,
    /// The hit sound: (I + T)·0.6 + 0.025 and 0.5·I + 0.5, before the
    /// armour's muffling.
    pub volume: f32,
    pub pitch: f32,
    /// The weapon sound's share: 1 armed, 0.5 unarmed.
    pub sound_share: f32,
    /// The weapon sound's muffling by the crush resistance met:
    /// 0.6·Rc + 0.7.
    pub muffle: f32,
    /// Each layer's (inner, outer) hit sound class and loudness: the outer
    /// at its weight, the inner at none when it rings as the outer, else
    /// (1 − 0.8·outer)·its weight.
    pub layer_sounds: [(u8, f32); 2],
}

/// Under this a float counts as nothing (the damage model's epsilon, about
/// 1.19e-7).
const EPSILON: f32 = f32::from_bits(0x33ff_ffd7);
/// The speed cap and the speed under which a blow is softened, cm a tick.
const SPEED_CAP: f32 = 30.0;
const SOFT_BELOW: f32 = 10.0;
/// The share of the bladed weapon's length after which its tip zone runs.
const BLADE_TIP: f32 = 0.6;
/// The stab's full tip range and its falloff end, cm from the tip.
const TIP_FULL: f32 = 7.0;
const TIP_NONE: f32 = 14.0;
/// 1/7 as the routine stores it.
const PER_TIP: f32 = f32::from_bits(0x3e12_4925);
/// Material sound loudness by layer class (flesh, cloth, leather, mail,
/// plate, Herath hide, golem).
const MATERIAL_LOUDNESS: [f32; 7] = [0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];

/// MAXSS against 0: the value when positive, else 0 (a non-number too).
fn pos(x: f32) -> f32 {
    if x > 0.0 { x } else { 0.0 }
}

/// A float scaled in double precision: (x as f64)·k, back to a float.
fn wide(x: f32, k: f64) -> f32 {
    (f64::from(x) * k) as f32
}

/// The on-target falloff of a contact `d` cm past the striking part:
/// max((20 − d)·0.05, 0), the 0.05 in double.
fn falloff(d: f32) -> f32 {
    pos(wide(20.0 - d, 0.05))
}

/// Where on the weapon a blow landed: the zone factors and flags.
struct Zone {
    /// Share of the record's slash, crush and pierce on target.
    on_target: f32,
    /// Impact factor ζ.
    impact: f32,
    /// Draw-cut share.
    draw: f32,
    zone: f32,
    edge: bool,
    central: bool,
    haft: bool,
}

/// A swing's zone factors at fraction `p` of an armed weapon's length.
fn swing_zone(w: &WeaponRecord, p: f32, clock: f32) -> Zone {
    let length = w.length;
    let mut z = if w.end < p {
        let q = (p - w.end) / (1.0 - w.end);
        let c = (1.0 - (0.8 - f64::from(clock)).powi(2) * 10.0) as f32;
        Zone {
            on_target: falloff((p - w.end) * length),
            impact: 1.0 - q * q * 0.75,
            draw: if c > 0.0 { q * c } else { 0.0 },
            zone: q,
            edge: true,
            central: false,
            haft: false,
        }
    } else if w.start > p {
        let impact = 1.0 - (w.start - p) / w.start;
        Zone {
            on_target: falloff((w.start - p) * length),
            impact,
            draw: 0.0,
            zone: impact,
            edge: false,
            central: true,
            haft: true,
        }
    } else {
        Zone {
            on_target: 1.0,
            impact: 1.0,
            draw: 0.0,
            zone: 1.0,
            edge: false,
            central: true,
            haft: false,
        }
    };
    // Past `from` the tip zone: q along it, an edge hit; before it the body
    // zone 1 − (2 − r)·r² (the central flag as it was).
    let tip = |z: &mut Zone, q: f32| {
        z.draw = q;
        z.impact = 1.0 - q * q * 0.75;
        z.central = false;
        z.zone = q;
        z.edge = true;
    };
    let body = |z: &mut Zone, r: f32| {
        z.impact = 1.0 - (2.0 - r) * (r * r);
        z.draw = 0.0;
        z.zone = z.impact;
        z.edge = false;
    };
    match w.kind() {
        flags::BLADED => {
            if p > BLADE_TIP {
                tip(&mut z, (p - BLADE_TIP) / (1.0 - BLADE_TIP));
            } else {
                body(&mut z, (BLADE_TIP - p) / BLADE_TIP);
            }
        }
        flags::STICK => {
            z.haft = false;
            if f64::from(p) > 0.8 {
                tip(
                    &mut z,
                    ((f64::from(p) - 0.8) * 5.000_000_000_000_001) as f32,
                );
            } else {
                body(&mut z, ((0.8 - f64::from(p)) * 1.25) as f32);
            }
        }
        _ if w.code == 2 => {
            let sweet = (f64::from(w.end - w.start) * 0.6 + f64::from(w.start)) as f32;
            if p > sweet {
                tip(&mut z, (p - sweet) / (1.0 - sweet));
            } else {
                z.impact = 1.0 - (sweet - p) / sweet;
                z.draw = 0.0;
                z.zone = z.impact;
                z.edge = false;
            }
        }
        _ => {}
    }
    z
}

/// Resolves a blow of `weapon` on body part `bone` of `victim`. `layers` is
/// the struck part's armour and a uniform draw in 0..1 for its coverage
/// (one per blow on a part); `None` when the blow met a shield or a weapon,
/// which no armour lessens and no roll is drawn for. Returns `None` when
/// the contact is at or past the weapon's tip.
///
/// The original's damage routine, in its order and precision: the speed
/// (capped, softened, overhead), the impact (a swing's (W10 + 0.3)·1.1·
/// u^(1 − W10/2), a stab's 0.3·u·(W10 + 0.5 + W6)), the powers, the hit
/// point p = along/W21, the zone factors (stab tip and lunge; a hanging
/// striker's and the fists' shares in full; the haft, beyond the striking
/// part and each kind's tip zone; the haft's crush bonus by its
/// material), unarmed 0.6·√(3I), the coverage roll over the outer then
/// the inner layer, the channels through padding and resistance (crush
/// and pierce raised by the engine's fast power), the shove, the
/// readiness, the daze with its stall factor and the bars.
pub fn resolve(
    weapon: &WeaponRecord,
    strike: &Strike,
    layers: Option<(&PartArmour, f32)>,
    bone: usize,
    victim: &Victim,
) -> Option<Outcome> {
    let w = weapon;
    // Speed: capped, else softened when slow; then the overhead's 1.5.
    let mut v = strike.speed;
    if v > SPEED_CAP {
        v = SPEED_CAP;
    } else if SOFT_BELOW > v {
        let s = wide(v, 0.1);
        v = (2.0 - s) * (s * s) * 10.0;
    }
    if strike.overhead {
        v *= 1.5;
    }
    let (u, mut impact) = if v > EPSILON {
        let u = wide(v, 0.05);
        let i = if strike.stab {
            wide((0.5 + w.impact + w.weight) * u, 0.3)
        } else {
            let e = f64::from(1.0 - w.impact * 0.5);
            ((f64::from(w.impact) + 0.3) * 1.1 * (f64::from(u).ln() * e).exp()) as f32
        };
        (u, i)
    } else {
        v = 0.0;
        (0.0, EPSILON)
    };
    let strikes = strike.strikes * 0.5 + 1.0;
    let shell = 1.0 - victim.shell;
    impact = shell * (strikes * impact);

    let p = strike.along / w.length;
    // At or past the tip there is no hit; a non-number goes on.
    let p = if 0.0 > p {
        0.0
    } else if 1.0 > p || p.is_nan() {
        p
    } else {
        return None;
    };

    // The zone: what the weapon struck with.
    let (mut hit, mut stab_energy, smoothing, zone, edge, central, haft, draw, shares) =
        if strike.stab {
            let t = w.length - strike.along;
            let tip = if TIP_FULL > t {
                1.0
            } else if TIP_NONE < t {
                0.0
            } else {
                1.0 - (t - TIP_FULL) * PER_TIP
            };
            let mut lunge = (1.0 - (1.4 - f64::from(strike.clock * 2.0)).powi(2) * 2.8) as f32;
            if 0.0 > lunge {
                lunge = 0.0;
            }
            lunge = shell * (strikes * lunge);
            let mut energy = (f64::from(tip)
                * ((f64::from(w.weight) * 0.3 + 0.25) * f64::from(impact + lunge)))
                as f32;
            if w.flags & flags::TWO_HANDS != 0 {
                energy *= 1.5;
            }
            let hit = (w.impact * 0.25 + 0.75) * energy;
            (
                hit,
                energy,
                0.0,
                p,
                false,
                true,
                false,
                0.0,
                (0.0, 0.0, 0.0),
            )
        } else if strike.hanging {
            let shares = (w.slash, w.crush, w.pierce);
            (
                impact,
                0.0,
                0.5,
                HANGING_ZONE,
                false,
                true,
                false,
                0.0,
                shares,
            )
        } else if strike.unarmed {
            let shares = (w.slash, w.crush, w.pierce);
            (impact, 0.0, 0.5, 0.0, false, false, false, 0.0, shares)
        } else {
            let z = swing_zone(w, p, strike.clock);
            let bonus = if z.haft {
                match w.flags & 0x30000 {
                    0x20000 => 0.2,
                    0x30000 => 0.15,
                    _ => 0.1,
                }
            } else {
                0.15
            };
            let crush = w.crush * z.on_target;
            let shares = (
                w.slash * z.on_target,
                (f64::from(1.0 - z.on_target) * bonus + f64::from(crush)) as f32,
                w.pierce * z.on_target,
            );
            let haft = z.haft && 0.5 > z.on_target;
            let hit = impact * z.impact;
            (
                hit, 0.0, 0.1, z.zone, z.edge, z.central, haft, z.draw, shares,
            )
        };
    let (mut slash, mut crush, mut pierce) = shares;
    let sound_share = if strike.unarmed {
        if hit > EPSILON {
            hit = wide((hit * 3.0).sqrt(), 0.6);
        }
        0.5
    } else {
        1.0
    };
    let volume = (f64::from(impact + stab_energy) * 0.6 + 0.025) as f32;
    let pitch = impact * 0.5 + 0.5;

    // Armour: one roll decides how much of each layer the blow meets, the
    // outer first.
    let (mut pad, mut rs, mut rc, mut rp) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let mut material = 0;
    let mut sounds = [(0u8, 0.0f32); 2];
    let mut sag = None;
    if let Some((armour, roll)) = layers {
        for (i, layer) in [(1, armour.outer), (0, Some(armour.inner))] {
            let Some(layer) = layer else { continue };
            let weight = if layer.coverage > roll {
                1.0
            } else if 0.5 < layer.coverage {
                (layer.coverage - 0.5) * 1.75
            } else {
                continue;
            };
            if material == 0 {
                material = layer.material;
            }
            sounds[i].0 = layer.material;
            if i == 0 {
                if sounds[1].0 == sounds[0].0 {
                    sounds[0].1 = 0.0;
                    sounds[1].1 = (1.0 - weight) * sounds[1].1 + weight;
                } else {
                    sounds[0].1 = ((1.0 - f64::from(sounds[1].1) * 0.8) * f64::from(weight)) as f32;
                }
            } else {
                sounds[1].1 = weight;
            }
            pad += layer.padding * weight;
            rs += (1.0 - rs) * (layer.slash * weight);
            rc += (1.0 - rc) * (layer.crush * weight);
            rp += (roll * 0.5 + 0.5) * ((1.0 - rp) * (layer.pierce * weight));
        }
        let s = hit * 2.0;
        sag = Some(if s > 1.0 { 1.0 } else { s });
    }

    let mut total = hit * 1.25;
    let mut j = (f64::from(hit) - f64::from(pad) * 0.2) as f32;
    j = if 0.0 < j {
        j / (pad * pad + 1.0 + pad)
    } else {
        0.0
    };
    if 0.0 < stab_energy {
        let half = pad * 0.5;
        stab_energy /= half * half + 1.0 + half;
    }
    let mut bite = wide(hit, 0.1);
    if strike.stab {
        slash = 0.0;
        crush = 0.0;
        pierce = if w.thrust > 0.0 {
            // From an energy of 2 the thrust's √2; under it (a non-number
            // too) the root of the energy, once the shove is something.
            if stab_energy < 2.0 || stab_energy.is_nan() {
                if j > EPSILON {
                    (f64::from(w.thrust) * 1.58 * f64::from(stab_energy.sqrt())) as f32
                } else {
                    0.0
                }
            } else {
                (f64::from(w.thrust) * 1.58 * std::f64::consts::SQRT_2) as f32
            }
        } else {
            0.0
        };
        bite += pos(pierce * 2.0 - rp);
    } else {
        if 0.0 < crush {
            crush = if j > EPSILON {
                (f64::from(crush) * 1.58 * f64::from(fast_pow(j, 2.0 - crush))) as f32
            } else {
                0.0
            };
            total += crush * 0.75;
            bite += pos(crush * 0.5 - rc * 0.25);
            crush = (f64::from(1.0 - rc * 0.75) * (f64::from(crush) - f64::from(rc) * 0.1)) as f32;
            if 0.0 > crush {
                crush = 0.0;
            }
        } else {
            crush = 0.0;
        }
        if 0.0 < slash {
            let k = slash / (slash * rs * 10.0 + 1.0);
            let chop = if hit > EPSILON {
                let q = hit / (hit + 1.0 - k) * 2.5;
                (((0.8 - f64::from(k)) * f64::from(hit * hit) + f64::from(q)) * f64::from(k)
                    - f64::from(rs) * 0.2) as f32
            } else {
                0.0
            };
            bite += pos((1.0 - draw) * chop);
            let drawn = if u > EPSILON {
                (f64::from(((1.0 - k) * u + u.sqrt()) * k) - f64::from(rs) * 0.3) as f32
            } else {
                0.0
            };
            bite += pos(drawn * 0.5 * draw);
            slash = drawn * draw + (1.0 - draw) * chop;
            if 0.0 > slash {
                slash = 0.0;
            }
            total += slash * 0.75;
        } else {
            slash = 0.0;
        }
        if 0.0 < pierce {
            pierce = (f64::from(pierce) * 1.58 * f64::from(fast_pow(j, pierce * 0.5 + 1.0))) as f32;
            bite += pos(pierce - rp * 0.5);
        }
        bite *= 0.5;
    }
    j = (f64::from(j) / ((f64::from(j * 2.0) - 0.2) * f64::from(pad) + 1.0)) as f32;
    if 0.0 > j {
        j = 0.0;
    }
    total += pierce * 0.75;
    pierce -= rp * 0.75;
    if 0.0 > pierce {
        pierce = 0.0;
    }

    // The flesh: the shove's impact, then the victim's readiness.
    let shove = j;
    let defended = 1.0 - victim.readiness * 0.5;
    let penetration =
        defended * (pos((f64::from(j) - 0.8) as f32) * 0.5 + (slash + pierce + crush));
    let j = defended * j;
    let stab_energy = defended * stab_energy;
    let (mult, stun) = part_multipliers(bone);
    let mut stun = stun * 0.5;
    let shaken = penetration * 0.75 + j * 1.25;
    let daze_factor = if victim.steadfast {
        stun *= 0.25;
        0.0
    } else {
        let mut m = if victim.security {
            let whole = defended * total;
            if whole > EPSILON {
                let r = shaken / whole;
                r * r
            } else {
                1.0
            }
        } else {
            1.0
        };
        let health = DAZE_HEALTH / victim.max_health;
        m *= health;
        stun *= health;
        if victim.potion {
            m *= 0.5;
        }
        m
    };
    let daze = if shaken > EPSILON {
        shaken.sqrt() * stun
    } else {
        0.0
    };
    let lasting = penetration / (1.0 + penetration);
    let wound = lasting * mult;
    let yellow = stab_energy * 0.5 + ((penetration - lasting) * 2.0 + j) * mult;
    Some(Outcome {
        red: wound * 0.75,
        wound,
        yellow: yellow * 0.75,
        daze,
        daze_factor,
        impact: shove,
        penetration,
        bite,
        material,
        hit_impact: hit,
        speed: v,
        total,
        zone,
        edge,
        central,
        haft,
        smoothing,
        sag,
        volume,
        pitch,
        sound_share,
        muffle: (f64::from(rc) * 0.6 + 0.7) as f32,
        layer_sounds: sounds,
    })
}

/// The health the daze is measured against: 0.25 over the most health.
const DAZE_HEALTH: f32 = 0.25;
/// The zone a hanging striker's blow reports, 0.2.
const HANGING_ZONE: f32 = 0.2;

/// The hit's sounds: the weapon's (or its haft's) at the blow, and each
/// armour layer's that rings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitSounds {
    /// The weapon's hit sound: volume and pitch.
    pub weapon: (f32, f32),
    /// The haft's material sound class (1 wood, 2 metal, 3 the third)
    /// replaces the weapon's.
    pub haft: Option<u32>,
    /// Each layer's (class, volume, pitch), inner then outer, when loud
    /// enough to play.
    pub layers: [Option<(u8, f32, f32)>; 2],
}

/// The sounds of a wound `blow` dealt by `weapon` from an attacker of body
/// `scale`: the weapon's at volume·share·scale·muffle and pitch·muffle (its
/// haft's when the haft took it), each layer above the epsilon at
/// 1.5·volume·its loudness·its class's, pitch 0.4·pitch + 0.7.
pub fn hit_sounds(blow: &Outcome, weapon: &WeaponRecord, scale: f32) -> HitSounds {
    let haft_class = if blow.haft {
        (weapon.flags & 0x30000) >> 16
    } else {
        0
    };
    let loud = blow.volume * blow.sound_share * scale * blow.muffle;
    let layer = |(class, weight): (u8, f32)| {
        let k = MATERIAL_LOUDNESS.get(usize::from(class))?;
        (weight > EPSILON).then(|| {
            (
                class,
                blow.volume * 1.5 * (weight * k),
                (f64::from(blow.pitch) * 0.4 + 0.7) as f32,
            )
        })
    };
    HitSounds {
        weapon: (loud, blow.pitch * blow.muffle),
        haft: (haft_class > 0).then_some(haft_class),
        layers: [layer(blow.layer_sounds[0]), layer(blow.layer_sounds[1])],
    }
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
    /// The non-character target (the behaviour's second target slot).
    pub other_target: u64,
    /// The character the behaviour pays attention to (looks at).
    pub look_target: u64,
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
    /// The non-character target (the behaviour's second target slot).
    pub other_target: u64,
    /// The character the behaviour pays attention to (looks at).
    pub look_target: u64,
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
        other_target: if input.other_target == input.target {
            0
        } else {
            input.other_target
        },
        look_target: if input.look_target == input.target {
            0
        } else {
            input.look_target
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
    /// The character the behaviour pays attention to (looks at).
    pub look_target: u64,
    /// The room the character was in before its current one (−1 none).
    pub previous_room: i32,
    /// The count of the nearby-characters list.
    pub nearby_count: i32,
    /// The character the behaviour steers around: one standing in its way,
    /// or the foe it last left (a guess from the two writers).
    pub avoided: u64,
    /// The character it follows, stored while following (a guess from the
    /// follow routine).
    pub leader: u64,
    /// The body's combat target.
    pub body_target: u64,
    /// The body's look target, the character its head turns toward.
    pub body_look_target: u64,
    /// The count of the rooms the flight remembers having searched.
    pub remembered_room_count: i32,
    /// The activity goal is weighed as a contact and slows the turn while
    /// it is the look target (a guess from its two readers).
    pub goal_watched: u8,
    pub marker_cleared: bool,
    pub reference_count: i32,
}

/// Zeroes the combatant's transient state and its stored targets.
pub fn reset_combatant(input: CombatResetInput) -> CombatReset {
    CombatReset {
        count: 0,
        look_target: 0,
        previous_room: -1,
        nearby_count: 0,
        avoided: 0,
        leader: 0,
        body_target: 0,
        body_look_target: 0,
        remembered_room_count: 0,
        goal_watched: 0,
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
    /// The non-character target (the behaviour's second target slot).
    pub other_target: u64,
    /// The character the behaviour pays attention to (looks at).
    pub look_target: u64,
    pub hit: Option<u64>,
    pub contact: Option<u64>,
}

/// The combat replace's results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatReplace {
    pub entries: Vec<u64>,
    pub replaced: bool,
    /// The non-character target (the behaviour's second target slot).
    pub other_target: u64,
    /// The character the behaviour pays attention to (looks at).
    pub look_target: u64,
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
        other_target: if input.other_target == input.old {
            input.new
        } else {
            input.other_target
        },
        look_target: if input.look_target == input.old {
            input.new
        } else {
            input.look_target
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
    use exa_formats::weapon_record::{Holder, Section};

    #[test]
    fn sword_a_record() {
        let w = sword_a();
        assert!((w.impact - 0.42).abs() < 1e-5);
        assert!((w.slash - 0.397).abs() < 1e-3);
        assert!((w.thrust - 0.325).abs() < 1e-3);
        assert!((w.start - 0.1).abs() < 1e-6 && (w.end - 1.0).abs() < 1e-6);
    }

    /// A mace: weight 70, balance 40, impact 140, thrust 20, crush 90,
    /// pierce 30, its head from 60 % to the end of its 100 cm, a metal haft.
    fn mace() -> WeaponRecord {
        let mut s = StatBytes::default();
        for (off, v) in [
            (at::WEIGHT, 70),
            (at::BALANCE, 40),
            (at::IMPACT, 140),
            (at::THRUST, 20),
            (at::START, 120),
            (at::END, 200),
            (at::CRUSH, 90),
            (at::PIERCE, 30),
        ] {
            s.set(off, v);
        }
        WeaponRecord::of_one_part(0x20012, &s, false, 100.0)
    }

    /// A human's fist and a ghoul's claw across a hand 10 cm wide.
    fn fist() -> WeaponRecord {
        let lengths = exa_formats::weapon_record::Lengths::hand(0.0, 10.0);
        WeaponRecord::build(&Section::FISTS, false, lengths, &Holder::AVERAGE)
    }

    fn claws() -> WeaponRecord {
        let lengths = exa_formats::weapon_record::Lengths::hand(0.0, 10.0);
        WeaponRecord::build(&Section::CLAWS, false, lengths, &Holder::AVERAGE)
    }

    /// Equal to the reference to a millionth (its own scale above 1).
    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-6 * b.abs().max(1.0)
    }

    macro_rules! pin {
        ($o:expr, $($field:ident = $v:expr),+ $(,)?) => {
            $(assert!(near($o.$field, $v), "{} {} != {}", stringify!($field), $o.$field, $v);)+
        };
    }

    fn plate() -> Armour {
        Armour {
            coverage: [100; 18],
            padding: 60,
            slash: 140,
            crush: 60,
            pierce: 80,
            encumbrance: 10,
            material: 4,
        }
    }

    fn swing(speed: f32, clock: f32, along: f32) -> Strike {
        Strike {
            speed,
            clock,
            along,
            ..Strike::default()
        }
    }

    fn on(
        w: &WeaponRecord,
        strike: &Strike,
        armour: &PartArmour,
        bone: usize,
        victim: &Victim,
        roll: f32,
    ) -> Outcome {
        resolve(w, strike, Some((armour, roll)), bone, victim).unwrap()
    }

    fn bare() -> PartArmour {
        PartArmour::default()
    }

    fn human() -> Victim {
        Victim::new(0.25, 0.0)
    }

    /// The sword's tip zone on a bare chest (bone 9: ×1.2 damage and
    /// stun), 20 cm a tick at 0.8 of the swing, 70 of its 90 cm: the
    /// numbers of the routine's own precision (a reference model of its
    /// listing gives the same to a millionth).
    #[test]
    fn a_cut_on_a_bare_chest() {
        let o = on(
            &sword_a(),
            &swing(20.0, 0.8, 70.0),
            &bare(),
            9,
            &human(),
            0.5,
        );
        pin!(
            o,
            hit_impact = 0.674_666_7,
            impact = 0.674_666_7,
            penetration = 0.614_884_85,
            wound = 0.456_912_96,
            red = 0.342_684_7,
            yellow = 1.028_623_3,
            daze = 0.685_287_5,
            daze_factor = 1.0,
            bite = 0.270_427_38,
            total = 1.304_497,
            zone = 0.444_444_45,
            speed = 20.0,
            volume = 0.500_200_03,
            pitch = 0.896_000_03,
            muffle = 0.7,
            smoothing = 0.1,
        );
        assert!(o.edge && !o.central && !o.haft);
        assert_eq!(o.sag, Some(1.0));
        // The flesh layer rings alone.
        assert_eq!(o.layer_sounds, [(0, 0.0), (0, 1.0)]);
    }

    /// Plate over the chest stops the edge: no lasting wound, the impact
    /// through 0.3 of padding, the plate ringing and the flesh at 0.2.
    #[test]
    fn plate_stops_the_edge() {
        let layers = armour_layers(None, &[plate()], false);
        let o = on(
            &sword_a(),
            &swing(20.0, 0.8, 70.0),
            &layers[8],
            9,
            &human(),
            0.5,
        );
        pin!(
            o,
            hit_impact = 0.674_666_7,
            impact = 0.366_877_56,
            penetration = 0.0,
            wound = 0.0,
            yellow = 0.330_189_82,
            daze = 0.406_318_75,
            bite = 0.033_733_334,
            total = 0.843_333_36,
            muffle = 0.957_142_83,
        );
        assert_eq!(o.material, 4);
        assert_eq!(o.layer_sounds[1], (4, 1.0));
        assert_eq!(o.layer_sounds[0].0, 0);
        assert!(near(o.layer_sounds[0].1, 0.2));
    }

    /// A slow overhead (5 cm a tick): softened to 10·(2 − 0.5)·0.25 =
    /// 3.75, then ×1.5; it lands mid-blade (55 % of the sword), the body
    /// zone 1 − (2 − r)·r².
    #[test]
    fn a_slow_overhead_is_softened_then_raised() {
        let strike = Strike {
            overhead: true,
            ..swing(5.0, 0.8, 50.0)
        };
        let o = on(&sword_a(), &strike, &bare(), 1, &human(), 0.5);
        pin!(
            o,
            speed = 5.625,
            hit_impact = 0.287_669_96,
            penetration = 0.334_132_64,
            wound = 0.250_449_33,
            yellow = 0.341_277_42,
            daze = 0.390_572_3,
            zone = 0.989_432_5,
        );
        assert!(near(o.sag.unwrap(), 0.575_339_9));
        assert!(o.central && !o.edge);
    }

    /// A stab 5 cm from the tip at the swing's middle: full tip, the lunge
    /// 1 − 2.8·0.16, the thrust's √T; through plate only its energy.
    #[test]
    fn a_stab_pierces_flesh_not_plate() {
        let stab = Strike {
            stab: true,
            ..swing(20.0, 0.5, 85.0)
        };
        let o = on(&sword_a(), &stab, &bare(), 9, &human(), 0.5);
        pin!(
            o,
            hit_impact = 0.334_476_02,
            penetration = 0.320_997_9,
            wound = 0.291_595_82,
            yellow = 0.588_130_95,
            daze = 0.487_015_04,
            bite = 0.675_443_4,
            total = 0.658_843_46,
            zone = 0.944_444_4,
            volume = 0.515_320_06,
            smoothing = 0.0,
        );
        assert!(o.central && !o.edge);
        // A stab whose contact is no number has no energy to speak of: its
        // shove is nothing, so it pierces nothing (not the thrust's √2).
        let lost = Strike {
            along: f32::NAN,
            ..stab
        };
        let o = on(&sword_a(), &lost, &bare(), 9, &human(), 0.5);
        assert_eq!((o.penetration, o.wound, o.impact), (0.0, 0.0, 0.0));
        let layers = armour_layers(None, &[plate()], false);
        let o = on(&sword_a(), &stab, &layers[8], 9, &human(), 0.5);
        pin!(
            o,
            penetration = 0.0,
            yellow = 0.293_016_97,
            impact = 0.186_555_22,
            bite = 0.197_768_3,
        );
    }

    /// A mace (crush 90, pierce 30, its head from 60 %, a metal haft) hit
    /// 5 cm below its head: on target 0.75, the metal haft's crush bonus
    /// 0.2·(1 − 0.75), not the head's 0.15; the haft's sound only under
    /// 0.5 on target.
    #[test]
    fn the_haft_adds_its_materials_crush() {
        let o = on(&mace(), &swing(20.0, 0.5, 55.0), &bare(), 1, &human(), 0.5);
        pin!(
            o,
            hit_impact = 1.179_750_2,
            penetration = 1.315_001_4,
            wound = 0.568_034_77,
            yellow = 2.005_262_6,
            daze = 0.784_369,
            bite = 0.416_512_16,
            total = 2.318_532_5,
            zone = 0.916_666_75,
        );
        assert!(o.central && !o.haft);
        // Its head on plate, the roll 0.9.
        let layers = armour_layers(None, &[plate()], false);
        let o = on(
            &mace(),
            &swing(20.0, 0.5, 80.0),
            &layers[1],
            1,
            &human(),
            0.9,
        );
        pin!(
            o,
            hit_impact = 1.287_000_1,
            impact = 0.600_646_14,
            penetration = 0.382_082_64,
            wound = 0.276_454_24,
            yellow = 0.608_927_2,
            bite = 0.174_655_71,
            total = 2.285_359_6,
        );
    }

    /// Fists: the record's shares in full, the impact 0.6·√(3I), the
    /// sound at half; on the head (×1.6, stun 1.4).
    #[test]
    fn a_punch_takes_the_unarmed_rules() {
        let punch = Strike {
            unarmed: true,
            ..swing(12.0, 0.5, 9.9)
        };
        let o = on(&fist(), &punch, &bare(), 10, &human(), 0.5);
        pin!(
            o,
            hit_impact = 0.762_068_5,
            penetration = 0.0,
            yellow = 0.914_482_24,
            daze = 0.683_203_46,
            bite = 0.038_103_424,
            volume = 0.347_638,
            pitch = 0.768_865,
            sound_share = 0.5,
            smoothing = 0.5,
            zone = 0.0,
        );
        assert!(!o.central && !o.edge);
        // Claws cut.
        let o = on(&claws(), &punch, &bare(), 1, &human(), 0.5);
        pin!(
            o,
            hit_impact = 0.882_402_06,
            penetration = 1.028_169_9,
            wound = 0.506_944_66,
            bite = 0.537_604_5,
        );
    }

    /// Security scales the stall factor by (D/τ′)² (here with readiness
    /// 0.5 and a potion's half); a steadfast body is dazed a quarter as
    /// much and its stall factor is 0.
    #[test]
    fn the_victims_rules_change_the_daze() {
        let cut = swing(20.0, 0.8, 70.0);
        let secure = Victim {
            security: true,
            potion: true,
            ..Victim::new(0.25, 0.5)
        };
        let o = on(&sword_a(), &cut, &bare(), 1, &secure, 0.5);
        pin!(
            o,
            daze_factor = 0.500_000_1,
            daze = 0.494_563_64,
            penetration = 0.461_163_64,
        );
        let ogre = Victim {
            steadfast: true,
            ..Victim::new(1.5, 0.0)
        };
        let o = on(&sword_a(), &cut, &bare(), 1, &ogre, 0.5);
        pin!(o, daze_factor = 0.0, daze = 0.142_768_22);
    }

    /// A blow on a shield or a weapon meets no armour and sets no sag.
    #[test]
    fn a_shield_meets_no_layer() {
        let o = resolve(&sword_a(), &swing(20.0, 0.8, 70.0), None, 9, &human()).unwrap();
        assert_eq!(o.sag, None);
        pin!(o, hit_impact = 0.674_666_7, impact = 0.674_666_7);
    }

    /// The block path on the cut's impact at the hit (0.6746667) against a
    /// shield of block 0.58 (leak and cost 0.71) and yellow 0.25, bits from
    /// a model of the listing: a tired arm leaks, a fresh one pays 0.8·I,
    /// a shield parry 0.4·I and sags the stance, Synchrony while countering
    /// at 0.15 of its clock pays (0.3·0.5 + 0.3)·I, Persist halves the
    /// clock for the stall factor, a potion halves the leak.
    #[test]
    fn a_shield_takes_the_blow_on_its_arm() {
        let impact = f32::from_bits(0x3f2c_b6f5);
        let leak = 1.0 - 0.5 * 0.58;
        let shield = |stamina, parrying| ShieldGuard {
            leak,
            cost: leak,
            stamina,
            rate: 0.0,
            blocked_at: 0,
            parrying,
        };
        let bits = |b: &Blocked| [b.stamina, b.yellow, b.daze, b.daze_factor].map(f32::to_bits);
        let tired = block(impact, 0.25, &shield(0.0, false), &Holding::default());
        assert_eq!(bits(&tired), [0, 0x3e67_a625, 0x3e12_1b21, 0x3f80_0000]);
        assert!(tired.sag.is_none() && !tired.locked && !tired.low);
        let fresh = block(impact, 0.25, &shield(1.0, false), &Holding::default());
        assert_eq!(
            bits(&fresh),
            [0x3f1d_e5e7, 0x3e76_ab1f, 0x3d03_de27, 0x3f80_0000]
        );
        let parry = block(impact, 0.25, &shield(1.0, true), &Holding::default());
        assert_eq!(
            bits(&parry),
            [0x3f4e_f2f4, 0x3e80_0000, 0x3dbf_1ee1, 0x3f80_0000]
        );
        assert_eq!(parry.sag.map(f32::to_bits), Some(0x3e4c_5b9c));
        assert!(parry.locked);
        let countered = Holding {
            counter: true,
            ..Holding::default()
        };
        assert!(!block(impact, 0.25, &shield(1.0, true), &countered).locked);
        let sync = Holding {
            synchrony: true,
            countering: true,
            clock: 0.15,
            ..Holding::default()
        };
        let synced = block(impact, 0.25, &shield(1.0, false), &sync);
        assert_eq!(
            bits(&synced),
            [0x3f48_d152, 0x3e7c_5174, 0x3c28_9c81, 0x3e19_999a]
        );
        let persist = Holding {
            persist: true,
            clock: 0.4,
            potion: true,
            ..Holding::default()
        };
        let persisted = block(impact, 0.25, &shield(0.5, false), &persist);
        assert_eq!(
            bits(&persisted),
            [0x3def_2f39, 0x3e75_3f19, 0x3ded_fd89, 0x3e4c_cccd]
        );
        // Under 0.05 yellow the low-health check runs.
        assert!(block(impact, 0.04, &shield(1.0, true), &Holding::default()).low);
    }

    /// The readiness halves the wound's terms but not the shove.
    #[test]
    fn readiness_halves_the_wound() {
        let strike = swing(20.0, 0.8, 70.0);
        let at = |g| on(&sword_a(), &strike, &bare(), 9, &Victim::new(0.25, g), 0.5);
        let (open, ready) = (at(0.0), at(1.0));
        assert!((ready.penetration - 0.5 * open.penetration).abs() < 1e-6);
        assert_eq!(ready.impact, open.impact);
        assert!(ready.red < open.red && ready.yellow < open.yellow);
    }

    /// The tests' sword, 90 cm striking from 10 % to the tip, in a human's
    /// hand: 60 + 90·(0.1 + 0.6·0.9) = 117.6 cm.
    #[test]
    fn reach_is_60_cm_plus_the_sweet_spot() {
        assert!((sword_a().reach_metres() - 1.176).abs() < 1e-5);
    }

    /// At or past the tip there is no hit (90 of 90 cm).
    #[test]
    fn past_the_tip_is_no_hit() {
        let o = resolve(
            &sword_a(),
            &swing(20.0, 0.5, 90.0),
            Some((&bare(), 0.5)),
            9,
            &human(),
        );
        assert!(o.is_none());
        assert!(resolve(&sword_a(), &swing(20.0, 0.5, 95.0), None, 9, &human()).is_none());
    }

    /// The sounds of the plate cut: the weapon's muffled by the plate's
    /// crush resistance, the plate's at 1.5·volume, the flesh at 0.2 of
    /// it and half its class's loudness.
    #[test]
    fn hit_sounds_ring_by_layer() {
        let layers = armour_layers(None, &[plate()], false);
        let o = on(
            &sword_a(),
            &swing(20.0, 0.8, 70.0),
            &layers[8],
            9,
            &human(),
            0.5,
        );
        let s = hit_sounds(&o, &sword_a(), 1.0);
        assert!(near(s.weapon.0, 0.500_200_03 * 0.957_142_83));
        assert_eq!(s.haft, None);
        let (class, volume, pitch) = s.layers[1].unwrap();
        assert_eq!(class, 4);
        assert!(near(volume, 0.500_200_03 * 1.5));
        assert!(near(pitch, (0.896_000_03f64 * 0.4 + 0.7) as f32));
        let (class, volume, _) = s.layers[0].unwrap();
        assert_eq!(class, 0);
        assert!(near(volume, 0.500_200_03 * 1.5 * (0.2 * 0.5)));
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
            other_target: target,
            look_target: 9,
            hit: Some(target),
            contact: Some(7),
        };

        let middle = forget_combat_target(input(vec![1, 2, 3], 2));
        assert!(middle.removed);
        assert_eq!(middle.entries, vec![1, 3]);
        assert_eq!(middle.count, 2);
        assert_eq!(middle.moves, 1);
        assert_eq!(middle.other_target, 0);
        assert_eq!(middle.look_target, 9);
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
        assert_eq!(absent.other_target, 0);
    }

    #[test]
    fn the_combatant_resets_and_measures() {
        let reset = reset_combatant(CombatResetInput {
            has_marker: true,
            has_reference: true,
            reference_count: 3,
        });
        assert_eq!(reset.count, 0);
        assert_eq!(reset.look_target, 0);
        assert_eq!(reset.previous_room, -1);
        assert_eq!(reset.nearby_count, 0);
        assert_eq!(reset.avoided, 0);
        assert_eq!(reset.leader, 0);
        assert_eq!(reset.body_target, 0);
        assert_eq!(reset.body_look_target, 0);
        assert_eq!(reset.remembered_room_count, 0);
        assert_eq!(reset.goal_watched, 0);
        assert!(reset.marker_cleared);
        assert_eq!(reset.reference_count, 2);

        let quiet = reset_combatant(CombatResetInput {
            has_marker: false,
            has_reference: false,
            reference_count: 3,
        });
        assert!(!quiet.marker_cleared);
        assert_eq!(quiet.reference_count, 3);
        assert_eq!(quiet.previous_room, -1);

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
            other_target: old,
            look_target: 7,
            hit: Some(old),
            contact: Some(old),
        };

        let swapped = replace_combat_target(input(vec![1, 2, 3], 2));
        assert!(swapped.replaced);
        assert_eq!(swapped.entries, vec![1, 9, 3]);
        assert_eq!(swapped.other_target, 9);
        assert_eq!(swapped.look_target, 7);
        assert_eq!(swapped.hit, Some(0x99));
        assert_eq!(swapped.contact, Some(9));

        let absent = replace_combat_target(input(vec![1, 2, 3], 4));
        assert!(!absent.replaced);
        assert_eq!(absent.entries, vec![1, 2, 3]);
        assert_eq!(absent.other_target, 9);
        assert_eq!(absent.hit, Some(0x99));
        assert_eq!(absent.contact, Some(9));
    }
}
