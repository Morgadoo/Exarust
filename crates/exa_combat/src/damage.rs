//! The original's damage model: a weapon's stat record, the two armour
//! layers of each body part, and what a blow does through them (impact,
//! slash, crush, pierce), to the health bars and the victim's wits.
//! Rules and constants: `docs/formats/OTHER.md` (Weapon stats, Armour,
//! Damage).
//!
//! Units follow the original: lengths in cm, speeds in cm per body tick.

use exa_formats::objdb::Armour;
use exa_formats::races::Natural;
use exa_formats::weapon_stats::{StatBytes, at};

/// Weapon flags (low bits of the weapon section's first word).
pub mod flags {
    /// Kind: 1 bladed, 2 hafted head, 3 stick, 4 flail.
    pub const KIND: u32 = 0xF;
    pub const BLADED: u32 = 1;
    pub const STICK: u32 = 3;
    /// Hands: 0 unarmed, 1 one-handed, 2 two-handed, 3 polearm.
    pub const HANDS: u32 = 0x30;
    /// Two-handed or polearm.
    pub const TWO_HANDS: u32 = 0x20;
    /// Haft material for hits below the head: 1 wood, 2 metal, 3 other.
    pub const HAFT_SHIFT: u32 = 16;
}

/// The stat record a held weapon fights with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponRecord {
    pub flags: u32,
    /// Type code of the striking side.
    pub code: u8,
    /// Start and end of the striking part, as fractions of the length.
    pub start: f32,
    pub end: f32,
    pub weight: f32,
    pub balance: f32,
    /// Impact factor.
    pub impact: f32,
    pub slash: f32,
    pub crush: f32,
    pub pierce: f32,
    pub thrust: f32,
    /// Length from the grip to the tip, cm.
    pub length: f32,
}

/// A stat byte as a damage share: 0.5·(x/100)^0.7.
fn share(x: u8) -> f32 {
    if x == 0 {
        0.0
    } else {
        0.5 * (x as f32 / 100.0).powf(0.7)
    }
}

impl WeaponRecord {
    /// The record of a weapon from its flags and stat bytes, striking with
    /// its flipped side when `flipped`, `length` cm long.
    pub fn new(flags: u32, stats: &StatBytes, flipped: bool, length: f32) -> Self {
        let side = if flipped { at::FLIP } else { 0 };
        let weight = stats.get(at::WEIGHT) as f32 / 100.0;
        let balance_byte = stats.get(at::BALANCE);
        let balance = balance_byte as f32 / 100.0;
        let base = if balance_byte <= 100 {
            (1.0 - balance) * 0.6 + 0.4 * weight + 0.1
        } else {
            0.4 * weight + 0.1
        };
        Self {
            flags,
            code: stats.get(at::KIND + side),
            start: stats.get(at::START + side) as f32 * 0.005,
            end: stats.get(at::END + side) as f32 * 0.005,
            weight,
            balance,
            impact: base + (stats.get(at::IMPACT) as f32 - 127.0) / 100.0,
            slash: share(stats.get(at::SLASH + side)),
            crush: share(stats.get(at::CRUSH + side)),
            pierce: share(stats.get(at::PIERCE + side)),
            thrust: share(stats.get(at::THRUST)),
            length,
        }
    }

    /// The reach (m) of a wielder of body scale `scale` holding this
    /// weapon: 60 cm times the scale plus the distance from the grip to
    /// the weapon's sweet spot, 60 % of the way along its striking part.
    /// The original measures the sweet spot on the weapon's last part and
    /// adds the parts before it whole; on a weapon of one part, as here,
    /// that is the same.
    pub fn reach(&self, scale: f32) -> f32 {
        let sweet = self.start + 0.6 * (self.end - self.start);
        (60.0 * scale + self.length * sweet) / 100.0
    }

    /// A human's fists: weightless, fully balanced, impact +0.33, no edge.
    pub fn fists(length: f32) -> Self {
        let mut s = StatBytes::default();
        s.set(at::BALANCE, 100);
        s.set(at::IMPACT, 160);
        s.set(at::START, 80);
        s.set(at::END, 80);
        Self::new(0, &s, false, length)
    }

    fn kind(&self) -> u32 {
        self.flags & flags::KIND
    }

    fn armed(&self) -> bool {
        self.flags & flags::HANDS != 0
    }
}

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

/// Armour of every body part (bones 1..18) from the race's natural layer and
/// the worn items in order, each with its condition factor.
pub fn armour_layers(natural: Option<&Natural>, worn: &[(Armour, f32)]) -> [PartArmour; 18] {
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
    for (item, condition) in worn {
        // Condition scales the bytes, rounded and capped like the original.
        let scaled = |b: u8| (b as f32 * condition).round_ties_even().min(160.0);
        let layer = |coverage: u8| Layer {
            coverage: coverage as f32 / 100.0,
            padding: scaled(item.padding) * 0.005,
            slash: scaled(item.slash) / 140.0,
            crush: scaled(item.crush) / 140.0,
            pierce: scaled(item.pierce) / 140.0,
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

/// What a blow does.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Outcome {
    /// Damage to the red (lasting) and yellow (current) bars, before their
    /// weights.
    pub red: f32,
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
    let red = lasting * mult * 0.75;
    let yellow = (0.5 * stab_energy + (2.0 * (penetration - lasting) + j) * mult) * 0.75;
    let daze = (0.75 * penetration + 1.25 * j).sqrt() * stun * 0.5 * (0.25 / max_health.max(1e-3));
    Some(Outcome {
        red,
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

/// The health bars of a character.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Health {
    /// Current (yellow) and lasting (red) health; both start at `max`.
    pub yellow: f32,
    pub red: f32,
    pub max: f32,
}

/// Yellow health below this is death.
pub const DEATH: f32 = 0.01;

impl Health {
    /// Health of a race with health factor `factor` (human 1).
    pub fn of_race(factor: f32) -> Self {
        let max = 0.25 * factor;
        Self {
            yellow: max,
            red: max,
            max,
        }
    }

    /// Takes a blow's damage; returns whether it kills.
    pub fn take(&mut self, outcome: &Outcome) -> bool {
        self.yellow -= 0.1 * outcome.yellow;
        let before = self.red;
        self.red -= 0.2 * outcome.red;
        if self.red > 0.0 && before > 0.0 {
            self.yellow *= self.red / before;
        } else {
            self.yellow = 0.0;
        }
        if self.yellow < DEATH {
            self.yellow = 0.0;
            true
        } else {
            false
        }
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
    WeaponRecord::new(0x11, &s, false, 90.0)
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
        let layers = armour_layers(None, &[(plate, 1.0)]);
        let armoured = cut(&layers[8], 0.5);
        assert!(
            armoured.penetration < bare.penetration * 0.3,
            "{armoured:?}"
        );
        assert_eq!(armoured.material, 4);
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
        assert!((sword_a().reach(1.0) - 1.176).abs() < 1e-5);
        assert!((sword_a().reach(2.0) - 1.776).abs() < 1e-5);
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
        let parts = armour_layers(None, &[(shirt, 1.0), (shirt, 1.0)]);
        let p = parts[0];
        assert_eq!(p.inner.coverage, 1.0);
        assert_eq!(p.inner.material, 3);
        // The first shirt folds in at its effective coverage 0.905.
        assert!((p.inner.slash - 0.905 * 0.5).abs() < 1e-5);
        assert!(p.outer.is_some());
    }

    #[test]
    fn health_and_death() {
        let mut h = Health::of_race(1.0);
        assert_eq!(h.max, 0.25);
        let light = Outcome {
            red: 0.1,
            yellow: 0.5,
            ..Outcome::default()
        };
        assert!(!h.take(&light));
        assert!(h.yellow < 0.25 && h.red < 0.25);
        let heavy = Outcome {
            red: 2.0,
            yellow: 5.0,
            ..Outcome::default()
        };
        assert!(h.take(&heavy));
        assert_eq!(h.yellow, 0.0);
    }
}
