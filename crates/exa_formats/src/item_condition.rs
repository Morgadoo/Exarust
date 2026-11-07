//! A made item's condition applied to its apparel protection and its price,
//! as the original's one apparel condition routine does for the template
//! migration, a shop copy and a repair's copy alike.
//!
//! With quality `q` and damage `d` the condition factor is
//! `k = (1 − d·0.4/255)·(q·0.5/255 + 0.8)`
//! ([`crate::weapon_stats::condition_factor`]). The copy's apparel bytes
//! (item body `0x1A0..=0x1A5`) are rebuilt from the template's:
//!
//! - padding, slash, crush and pierce: `round(t·k)` in single precision,
//!   stored as a byte (a product past 255 wraps) and then capped at 160;
//! - the coverage display: `round((0.3k + 0.7)·t)` in double, stored as a
//!   byte (it wraps; no cap);
//! - the encumbrance: `round(t · (1 / ((q·0.3/255 + 0.85)(1 − d·0.15/255))))`,
//!   the divisor narrowed to single before its reciprocal, stored as a byte;
//! - then the armour variant (body byte `0x1AA`, 1 to 8) scales them
//!   ([`variant_scales`]).
//!
//! The price word (compact amount) becomes `value / (2·tq² + 0.5) ·
//! ((2·q² + 0.5)·(2·(1 − d)² + 0.5))·0.5`, each share `byte/255` in double
//! narrowed to single, the curves in single. The apparel routine bases it on
//! the template's quality; the weapon routine on the copy's own.
//!
//! Evidence: the item description unit's apparel and weapon condition
//! routines and the per-variant handlers they dispatch to (REA, read in the
//! instructions: every rounded product is stored with a byte move, the cap
//! compares the stored byte).

use crate::containers::{coins_of, price_of};
use crate::objdb::packed::{round_f32, round_f64};
use crate::weapon_stats::condition_factor;

/// The cap the condition puts on a protection byte.
pub const PROTECTION_CAP: u8 = 0xA0;

/// The encumbrance divisor's quality step, `0.3 / 255`.
const ENCUMBRANCE_QUALITY: u64 = 0x3F53_4679_ACE0_1346;
/// The encumbrance divisor's damage step, `0.15 / 255`.
const ENCUMBRANCE_DAMAGE: u64 = 0x3F43_4679_ACE0_1346;

/// Apparel protection bytes: padding, slash, crush and pierce, the coverage
/// display and the encumbrance (item body `0x1A0..=0x1A5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Protection {
    pub resist: [u8; 4],
    pub coverage: u8,
    pub encumbrance: u8,
}

impl Protection {
    /// The six bytes of an apparel body at `0x1A0`, if it is that long.
    pub fn of_body(body: &[u8]) -> Option<Self> {
        let b = body.get(0x1A0..0x1A6)?;
        Some(Self {
            resist: [b[0], b[1], b[2], b[3]],
            coverage: b[4],
            encumbrance: b[5],
        })
    }

    /// Writes the six bytes into an apparel body at `0x1A0`; false when it
    /// is too short.
    pub fn write_to(&self, body: &mut [u8]) -> bool {
        let Some(b) = body.get_mut(0x1A0..0x1A6) else {
            return false;
        };
        b[..4].copy_from_slice(&self.resist);
        b[4] = self.coverage;
        b[5] = self.encumbrance;
        true
    }
}

/// The armour variants 1–8 an apparel copy can be rolled with (body byte
/// `0x1AA`), as the original's per-variant handlers apply them after the
/// condition: each scales the padding, slash, crush and pierce bytes and the
/// encumbrance (the bytes wrap; no cap). Variants 7 and 8 change nothing.
pub fn variant_scales(variant: u8) -> Option<([f64; 4], f64)> {
    match variant {
        1 => Some(([0.9; 4], 0.8)),
        2 => Some(([1.1; 4], 1.16)),
        3 => Some(([1.2; 4], 1.1)),
        4 => Some(([1.0, 1.35, 1.0, 1.35], 1.06)),
        5 => Some(([1.4, 1.0, 1.2, 1.0], 1.06)),
        6 => Some(([1.3, 1.1, 1.3, 1.1], 1.16)),
        7 | 8 => Some(([1.0; 4], 1.0)),
        _ => None,
    }
}

/// Apparel's protection from its template's at the copy's quality, damage
/// and armour variant.
pub fn apparel_condition(
    template: &Protection,
    quality: u8,
    damage: u8,
    variant: u8,
) -> Protection {
    let k = condition_factor(quality, damage);
    let coverage = round_f64((f64::from(k) * 0.3 + 0.7) * f64::from(template.coverage)) as u8;
    let resist = template
        .resist
        .map(|t| (round_f32(f32::from(t) * k) as u8).min(PROTECTION_CAP));
    let divisor = ((f64::from(quality) * f64::from_bits(ENCUMBRANCE_QUALITY) + 0.85)
        * (1.0 - f64::from(damage) * f64::from_bits(ENCUMBRANCE_DAMAGE))) as f32;
    let inverse = 1.0f32 / divisor;
    let mut p = Protection {
        resist,
        coverage,
        encumbrance: round_f32(f32::from(template.encumbrance) * inverse) as u8,
    };
    if let Some((scales, encumbrance)) = variant_scales(variant) {
        p.encumbrance = round_f64(f64::from(p.encumbrance) * encumbrance) as u8;
        for (r, scale) in p.resist.iter_mut().zip(scales) {
            if scale != 1.0 {
                *r = round_f64(f64::from(*r) * scale) as u8;
            }
        }
    }
    p
}

/// The compact price word of a copy at `quality` and `damage`, from the
/// price word it holds and the base quality (the template's for apparel,
/// the copy's own for a weapon).
pub fn condition_price(price: u16, base_quality: u8, quality: u8, damage: u8) -> u16 {
    let share = |byte: u8| (f64::from(byte) / 255.0) as f32;
    let curve = |x: f32| x * x * 2.0 + 0.5;
    let base = curve(share(base_quality));
    let made = curve(share(quality));
    let wear = curve((1.0 - f64::from(damage) / 255.0) as f32);
    let scaled = round_f32(coins_of(price) as f32 / base * (made * wear * 0.5));
    price_of(scaled as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pristine_copy_keeps_resist_and_scales_coverage() {
        let t = Protection {
            resist: [100, 50, 20, 0],
            coverage: 80,
            encumbrance: 40,
        };
        // q 255, d 0: k = 1.3.
        let p = apparel_condition(&t, 255, 0, 0);
        assert_eq!(p.resist, [130, 65, 26, 0]);
        // (0.3·1.3 + 0.7)·80 = 87.2 → 87.
        assert_eq!(p.coverage, 87);
        // 40 / 1.15 = 34.78 → 35.
        assert_eq!(p.encumbrance, 35);
    }

    #[test]
    fn a_product_past_255_wraps_before_the_cap() {
        let t = Protection {
            resist: [250, 0, 0, 0],
            coverage: 240,
            encumbrance: 0,
        };
        // 250·1.3 = 325 → stored as 325 & 0xFF = 69, under the cap.
        let p = apparel_condition(&t, 255, 0, 0);
        assert_eq!(p.resist[0], 69);
        // (0.39 + 0.7)·240 = 261.6 → 262 → 6.
        assert_eq!(p.coverage, 6);
        // 200·1.3 = 260 → 4.
        let t = Protection {
            resist: [200, 150, 0, 0],
            ..t
        };
        assert_eq!(apparel_condition(&t, 255, 0, 0).resist[..2], [4, 160]);
    }

    #[test]
    fn a_variant_scales_after_the_condition() {
        let t = Protection {
            resist: [100, 100, 100, 100],
            coverage: 50,
            encumbrance: 50,
        };
        let plain = apparel_condition(&t, 0, 0, 0);
        let heavy = apparel_condition(&t, 0, 0, 4);
        assert_eq!(plain.resist, [80; 4]);
        assert_eq!(heavy.resist, [80, 108, 80, 108]);
        // 50/0.85 = 58.8 → 59, then ×1.06 = 62.54 → 63.
        assert_eq!(plain.encumbrance, 59);
        assert_eq!(heavy.encumbrance, 63);
    }

    #[test]
    fn the_price_follows_quality_and_wear() {
        // The base quality's own and no wear: 100 / 2.5 · (2.5 · 2.5 · 0.5).
        assert_eq!(condition_price(100, 255, 255, 0), 125);
        // Worn out: the wear curve is 0.5.
        assert_eq!(condition_price(100, 255, 255, 255), 25);
    }
}
