//! Weapon stat bytes: the bytes `0x30..=0x42` of an item's weapon section
//! (`0x4000`), and how a factory recipe produces them for an assembled
//! weapon. Layout, rules and evidence: `docs/formats/OTHER.md` (Weapon
//! stats).
//!
//! Plain weapons use their object-database bytes as they are. A factory
//! weapon starts from its template item's bytes; each chosen variant adds its
//! slash, crush, pierce and thrust bonuses, and the part's deforms scale the
//! bytes by role. Factory items are then scaled by their condition.

use crate::factory::{Factory, Generated, deform_factor};
use crate::pixel::{round_f32, round_f64};

/// Offset of the stat bytes in the weapon section body.
pub const OFFSET: usize = 0x30;
pub const LEN: usize = 0x13;

/// Stat byte offsets (section body offsets).
pub mod at {
    /// 0..100.
    pub const WEIGHT: usize = 0x30;
    /// 0..100.
    pub const BALANCE: usize = 0x31;
    /// 127 neutral.
    pub const IMPACT: usize = 0x32;
    pub const THRUST: usize = 0x33;
    /// Type code of the striking side (`FLIP` adds 6 for the other side).
    pub const KIND: usize = 0x34;
    /// Start and end of the striking part along the weapon, ×0.005.
    pub const START: usize = 0x35;
    pub const END: usize = 0x36;
    pub const SLASH: usize = 0x37;
    pub const CRUSH: usize = 0x38;
    pub const PIERCE: usize = 0x39;
    /// The flipped side's block starts this far after the normal one.
    pub const FLIP: usize = 6;
    /// 0x80 neutral.
    pub const ADJUST: usize = 0x40;
    /// Hit sound class.
    pub const SOUND: usize = 0x41;
}

/// The stat bytes of a weapon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatBytes(pub [u8; LEN]);

impl Default for StatBytes {
    fn default() -> Self {
        let mut s = Self([0; LEN]);
        s.set(at::IMPACT, 127);
        s.set(at::ADJUST, 128);
        s
    }
}

impl StatBytes {
    /// Reads the bytes from a weapon section body.
    pub fn from_section(body: &[u8]) -> Option<Self> {
        Some(Self(body.get(OFFSET..OFFSET + LEN)?.try_into().ok()?))
    }

    /// The byte at section offset `off`.
    pub fn get(&self, off: usize) -> u8 {
        self.0[off - OFFSET]
    }

    pub fn set(&mut self, off: usize, v: u8) {
        self.0[off - OFFSET] = v;
    }

    fn add(&mut self, off: usize, bonus: i16) {
        self.set(off, self.get(off).wrapping_add(bonus as u8));
    }

    /// Multiplies a byte, rounds to the nearest (ties to even) and keeps the
    /// low eight bits, as the original stores a rounded integer into a byte.
    fn mul(&mut self, off: usize, f: f32) {
        let v = self.get(off) as f32 * f;
        self.set(off, round_f32(v) as u8);
    }

    /// Divides a byte with the same rounding and truncation.
    fn div(&mut self, off: usize, f: f32) {
        let v = self.get(off) as f32 / f;
        self.set(off, round_f32(v) as u8);
    }

    /// [`StatBytes::mul`] in double precision.
    fn mul_f64(&mut self, off: usize, f: f64) {
        let v = f64::from(self.get(off)) * f;
        self.set(off, round_f64(v) as u8);
    }

    /// [`StatBytes::div`] in double precision.
    fn div_f64(&mut self, off: usize, f: f64) {
        let v = f64::from(self.get(off)) / f;
        self.set(off, round_f64(v) as u8);
    }

    fn cap(&mut self, off: usize, max: u8) {
        self.set(off, self.get(off).min(max));
    }
}

/// Hit sound classes of the weapon section's sound byte, in the original's
/// table order: a sword, a mace, an axe, a hammer, a mace, a club, wood,
/// claws, and a body-to-body collision.
pub const IMPACT_SOUNDS: [&str; 9] = [
    "WH_SwordA",
    "WH_MaceMA",
    "WH_AxeA",
    "WH_HammerA",
    "WH_MaceMA",
    "WH_ClubMW",
    "WH_WoodA",
    "WH_ClawsL",
    "BdyCol",
];

/// The hit sound a factory weapon's body plays, from its section's sound
/// byte: the table's entry at that index, or a sword when the class is past
/// the table (the build reads the table without a bounds test; [guess] an
/// out-of-range byte cannot ship, so the fallback is Exarust's).
pub fn impact_sound(class: u8) -> &'static str {
    IMPACT_SOUNDS
        .get(class as usize)
        .copied()
        .unwrap_or(IMPACT_SOUNDS[0])
}

/// Part roles in a factory recipe.
mod role {
    pub const BLADE: u8 = 1;
    pub const HEAD: u8 = 2;
    pub const GUARD: u8 = 4;
    pub const HAFT: u8 = 5;
    pub const FLIP_SIDE: u8 = 6;
}

/// Variant flags.
const NO_FLIP: u16 = 0x800;
const TAPER_THICKENS: u16 = 0x100;
const TAPER_THINS: u16 = 0x200;
/// Deform flag: one-sided, or scales attached children too.
const ONE_SIDED: u32 = 0x10000;

/// The stat bytes of a weapon generated from `factory`, starting from its
/// template item's bytes. Condition scaling is separate ([`condition`]).
pub fn factory_stats(factory: &Factory, generated: &Generated, template: StatBytes) -> StatBytes {
    use at::*;
    let mut s = template;
    let mut flip_allowed = true;
    for (i, part) in factory.parts.iter().enumerate() {
        let Some(variant) = generated
            .selection
            .get(i)
            .and_then(|&c| (c as usize).checked_sub(1))
            .and_then(|v| part.variants.get(v))
        else {
            continue;
        };
        let [slash, crush, pierce, thrust] = variant.bonuses;
        let kind = ((variant.flags >> 4) & 0xF) as u8;
        if part.role == role::FLIP_SIDE {
            s.set(KIND + FLIP, kind);
            s.add(SLASH + FLIP, slash);
            s.add(CRUSH + FLIP, crush);
            s.add(PIERCE + FLIP, pierce);
        } else {
            s.set(KIND, kind);
            s.add(SLASH, slash);
            s.add(CRUSH, crush);
            s.add(PIERCE, pierce);
            s.add(THRUST, thrust);
        }
        if variant.flags & NO_FLIP != 0 {
            flip_allowed = false;
        }
        let values = generated.deform.get(i).copied().unwrap_or([0.0; 3]);
        for (d, v) in part.deforms.iter().zip(values) {
            if d.kind == 0 {
                continue;
            }
            let x = v * d.stat_weight;
            // F scales by the deform's amount, G does not.
            let f = |x: f32| deform_factor(x, d.amount);
            let g = |x: f32| deform_factor(x, 1.0);
            // A head's or haft's shape scales weight and balance together.
            let heavier = |s: &mut StatBytes, h: f32| {
                s.mul(WEIGHT, h * 0.5 + 0.5);
                s.div(BALANCE, h);
            };
            // `0.4·G + 0.6` in double, stored as a float.
            let shaft = |g: f32| (f64::from(g) * 0.4 + 0.6) as f32;
            match (part.role, d.kind, d.axis) {
                // Blade: longer about Y is heavier and less balanced.
                (role::BLADE, 1, 2) => {
                    let f = f(x);
                    s.mul(WEIGHT, f);
                    s.div(BALANCE, f);
                }
                // Blade: longer about X.
                (role::BLADE, 1, 1) => {
                    let f = f(x);
                    s.mul(WEIGHT, f);
                    s.div(BALANCE, f);
                    s.mul(SLASH, f);
                    s.div(THRUST, f);
                }
                // Blade taper: balance follows half the taper, slash and
                // thrust move against it.
                (role::BLADE, 3, 2) => {
                    let f = f(x);
                    let h = f * 0.5 + 0.5;
                    s.mul(BALANCE, h);
                    s.div(SLASH, h);
                    s.mul(THRUST, f);
                }
                // Blade bend: the point's thrust depends on the taper flags.
                (role::BLADE, 4, 1) => {
                    if d.flags & ONE_SIDED != 0 {
                        s.div(THRUST, g(x) * 0.25 + 0.75);
                    } else if variant.flags & TAPER_THICKENS != 0 {
                        s.mul(THRUST, g(x) * 0.5 + 0.5);
                    } else if variant.flags & TAPER_THINS != 0 {
                        s.mul(THRUST, g(-v * d.stat_weight) * 0.5 + 0.5);
                    }
                }
                // Haft: a band stretch or scale about Y changes both weight
                // and balance; an XZ scale only the weight (in double).
                (role::HAFT, 1 | 2, 2) => heavier(&mut s, shaft(g(x))),
                (role::HAFT, 1, 5) => s.mul_f64(WEIGHT, f64::from(g(x)) * 0.1 + 0.9),
                (role::GUARD, 1, 2) => s.mul(THRUST, f(x)),
                // Head: any scale, halfway for a named axis.
                (role::HEAD, 1, axis) => {
                    let f = f(x);
                    heavier(&mut s, if axis >= 1 { f * 0.5 + 0.5 } else { f });
                }
                (role::HEAD, 3, 2) => heavier(&mut s, g(v * -0.5 * d.stat_weight) * 0.5 + 0.5),
                // Head radial: pierce against crush, in double.
                (role::HEAD, 5, 0 | 5) => {
                    let h = f64::from(g(v * 0.5 * d.stat_weight)) * 0.6 + 0.4;
                    s.mul_f64(PIERCE, h);
                    s.div_f64(CRUSH, h);
                }
                (role::HEAD, 5, 2) => heavier(&mut s, shaft(g(v * 0.5 * d.stat_weight))),
                // The flipped side's deforms change nothing.
                _ => {}
            }
        }
    }
    if !flip_allowed {
        s.set(KIND + FLIP, 0);
    }
    s.cap(BALANCE, 100);
    s.cap(WEIGHT, 100);
    s
}

/// Condition factor of a made item from its quality and damage bytes, as
/// applied to factory weapons and apparel: `(1 − d·0.4/255)·(q·0.5/255 +
/// 0.8)` in the original's mix of precisions (the damage term and the
/// product in double, the quality term's product in float, the result
/// stored as float).
pub fn condition_factor(quality: u8, damage: u8) -> f32 {
    let d = 1.0 - f64::from(damage) * f64::from_bits(0x3F59_B34C_E680_19B4);
    let q = f64::from(f32::from(quality) * f32::from_bits(0x3B00_8081)) + 0.8;
    (d * q) as f32
}

/// Scales a factory weapon's stat bytes by its condition, as the original
/// does: slash, pierce and thrust by the factor in float, crush by
/// `0.7c + 0.3` in double, each rounded to the nearest (ties to even) and
/// stored as a byte (so a product past 255 wraps), then capped at 160.
pub fn condition(mut s: StatBytes, quality: u8, damage: u8) -> StatBytes {
    use at::*;
    let c = condition_factor(quality, damage);
    let by_factor = |b: u8| (f32::from(b) * c).round_ties_even() as i64 as u8;
    for side in [0, FLIP] {
        let crush = ((f64::from(c) * 0.7 + 0.3) * f64::from(s.get(CRUSH + side))).round_ties_even()
            as i64 as u8;
        s.set(SLASH + side, by_factor(s.get(SLASH + side)));
        s.set(CRUSH + side, crush);
        s.set(PIERCE + side, by_factor(s.get(PIERCE + side)));
        for off in [SLASH, CRUSH, PIERCE] {
            s.cap(off + side, 160);
        }
    }
    s.set(THRUST, by_factor(s.get(THRUST)));
    s.cap(THRUST, 160);
    s.cap(IMPACT, 160);
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::{Deform, Part, Variant};

    fn part(role: u8, bonuses: [i16; 4], flags: u16, deform: Deform) -> Part {
        Part {
            flags: 0,
            name: String::new(),
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
            atlas: [1.0, 1.0, 0.0, 0.0],
            texture_parameter: 0.0,
            deforms: [deform; 3],
            chance: 100,
            attach_to: 0,
            role,
            attach_mode: 0,
            variants: vec![Variant {
                model: String::new(),
                mask_areas: [0; 4],
                weight: 100,
                bonuses,
                tags: 0,
                excludes: 0,
                flags,
                child_y_offset: 0,
                texture_scale: 0,
                texture_offset: [0, 0],
            }],
        }
    }

    fn deform(kind: u8, axis: u8, amount: f32, stat_weight: f32) -> Deform {
        Deform {
            kind,
            axis,
            flags: 0,
            amount,
            pivot: 0.0,
            range: 0.0,
            blend: 0.0,
            stat_weight,
        }
    }

    fn generated(n: usize, value: f32) -> Generated {
        Generated {
            selection: vec![1; n],
            twist: vec![0.0; n],
            deform: vec![[value, 0.0, 0.0]; n],
            materials: Vec::new(),
        }
    }

    #[test]
    fn bonuses_add_and_a_longer_blade_is_heavier() {
        let factory = Factory {
            version: 2,
            atlas: [0; 2],
            parts: vec![part(
                role::BLADE,
                [60, 0, 10, 40],
                0x20,
                deform(1, 2, 0.5, 1.0),
            )],
            areas: Vec::new(),
            tags: Vec::new(),
        };
        let mut template = StatBytes::default();
        template.set(at::WEIGHT, 50);
        template.set(at::BALANCE, 80);
        let s = factory_stats(&factory, &generated(1, 0.4), template);
        assert_eq!(s.get(at::SLASH), 60);
        assert_eq!(s.get(at::PIERCE), 10);
        assert_eq!(s.get(at::THRUST), 40);
        assert_eq!(s.get(at::KIND), 2);
        // f = 1 + 0.4·0.5 = 1.2: weight 60, balance 80/1.2 = 66.7 → 67.
        assert_eq!(s.get(at::WEIGHT), 60);
        assert_eq!(s.get(at::BALANCE), 67);
    }

    #[test]
    fn flip_side_parts_fill_the_other_block() {
        let factory = Factory {
            version: 2,
            atlas: [0; 2],
            parts: vec![part(
                role::FLIP_SIDE,
                [0, 30, 0, 0],
                0x10,
                deform(0, 0, 0.0, 0.0),
            )],
            areas: Vec::new(),
            tags: Vec::new(),
        };
        let s = factory_stats(&factory, &generated(1, 0.0), StatBytes::default());
        assert_eq!(s.get(at::CRUSH + at::FLIP), 30);
        assert_eq!(s.get(at::KIND + at::FLIP), 1);
        assert_eq!(s.get(at::CRUSH), 0);
    }

    #[test]
    fn the_taper_uses_the_scale_amount_not_a_bare_one() {
        let factory = Factory {
            version: 2,
            atlas: [0; 2],
            parts: vec![part(role::BLADE, [0; 4], 0, deform(3, 2, 0.5, 1.0))],
            areas: Vec::new(),
            tags: Vec::new(),
        };
        let mut template = StatBytes::default();
        template.set(at::BALANCE, 80);
        template.set(at::SLASH, 100);
        template.set(at::THRUST, 40);
        let s = factory_stats(&factory, &generated(1, 0.5), template);
        // f = 1 + 0.5·0.5 = 1.25, h = 1.125.
        assert_eq!(s.get(at::BALANCE), 90);
        assert_eq!(s.get(at::SLASH), 89);
        assert_eq!(s.get(at::THRUST), 50);
    }

    #[test]
    fn a_longer_haft_keeps_more_weight_towards_the_middle() {
        let factory = Factory {
            version: 2,
            atlas: [0; 2],
            parts: vec![part(role::HAFT, [0; 4], 0, deform(1, 2, 1.0, 1.0))],
            areas: Vec::new(),
            tags: Vec::new(),
        };
        let mut template = StatBytes::default();
        template.set(at::WEIGHT, 40);
        template.set(at::BALANCE, 80);
        let s = factory_stats(&factory, &generated(1, 0.5), template);
        // f = 1.5, h = 1.2: weight 40·(1.1) = 44, balance 80/1.2 = 67.
        assert_eq!(s.get(at::WEIGHT), 44);
        assert_eq!(s.get(at::BALANCE), 67);
    }

    #[test]
    fn haft_and_head_curves_ignore_the_deform_amount() {
        // G is 1 + x without the amount: an amount of 0.5 changes nothing
        // (F would give 1.25 and weight 42, balance 73).
        let factory = Factory {
            version: 2,
            atlas: [0; 2],
            parts: vec![part(role::HAFT, [0; 4], 0, deform(1, 2, 0.5, 1.0))],
            areas: Vec::new(),
            tags: Vec::new(),
        };
        let mut template = StatBytes::default();
        template.set(at::WEIGHT, 40);
        template.set(at::BALANCE, 80);
        let s = factory_stats(&factory, &generated(1, 0.5), template);
        assert_eq!(s.get(at::WEIGHT), 44);
        assert_eq!(s.get(at::BALANCE), 67);
    }

    #[test]
    fn flip_side_deforms_change_no_bytes() {
        let factory = Factory {
            version: 2,
            atlas: [0; 2],
            parts: vec![part(role::FLIP_SIDE, [0; 4], 0, deform(1, 0, 1.0, 1.0))],
            areas: Vec::new(),
            tags: Vec::new(),
        };
        let mut template = StatBytes::default();
        template.set(at::WEIGHT, 40);
        template.set(at::BALANCE, 80);
        let s = factory_stats(&factory, &generated(1, 1.0), template);
        assert_eq!(s.get(at::WEIGHT), 40);
        assert_eq!(s.get(at::BALANCE), 80);
    }

    #[test]
    fn a_head_radial_scales_pierce_against_crush() {
        let factory = Factory {
            version: 2,
            atlas: [0; 2],
            parts: vec![part(role::HEAD, [0; 4], 0, deform(5, 0, 1.0, 1.0))],
            areas: Vec::new(),
            tags: Vec::new(),
        };
        let mut template = StatBytes::default();
        template.set(at::CRUSH, 100);
        template.set(at::PIERCE, 100);
        let s = factory_stats(&factory, &generated(1, 1.0), template);
        // h = F(0.5)·0.6 + 0.4 = 1.5·0.6 + 0.4 = 1.3.
        assert_eq!(s.get(at::PIERCE), 130);
        assert_eq!(s.get(at::CRUSH), 77);
    }

    #[test]
    fn cap_and_no_flip() {
        let factory = Factory {
            version: 2,
            atlas: [0; 2],
            parts: vec![part(
                role::BLADE,
                [200, 0, 0, 0],
                NO_FLIP,
                deform(1, 2, 1.0, 1.0),
            )],
            areas: Vec::new(),
            tags: Vec::new(),
        };
        let mut template = StatBytes::default();
        template.set(at::BALANCE, 120);
        template.set(at::WEIGHT, 120);
        let s = factory_stats(&factory, &generated(1, 1.0), template);
        assert_eq!(s.get(at::KIND + at::FLIP), 0);
        assert_eq!(s.get(at::BALANCE), 60);
        assert_eq!(s.get(at::WEIGHT), 100);
    }

    #[test]
    fn condition_scales_and_caps() {
        let mut s = StatBytes::default();
        s.set(at::SLASH, 150);
        s.set(at::CRUSH, 100);
        // Best quality, no damage: c = 1.3.
        let s = condition(s, 255, 0);
        assert_eq!(s.get(at::SLASH), 160);
        assert_eq!(s.get(at::CRUSH), 121);
        assert!((condition_factor(0, 255) - 0.48).abs() < 1e-6);
        // A product past 255 wraps before the cap: 200 × 1.3 = 260 → 4.
        let mut s = StatBytes::default();
        s.set(at::PIERCE, 200);
        assert_eq!(condition(s, 255, 0).get(at::PIERCE), 4);
    }

    #[test]
    fn impact_sounds_follow_the_class_table() {
        assert_eq!(impact_sound(0), "WH_SwordA");
        assert_eq!(impact_sound(1), "WH_MaceMA");
        assert_eq!(impact_sound(2), "WH_AxeA");
        assert_eq!(impact_sound(3), "WH_HammerA");
        assert_eq!(impact_sound(4), "WH_MaceMA");
        assert_eq!(impact_sound(5), "WH_ClubMW");
        assert_eq!(impact_sound(6), "WH_WoodA");
        assert_eq!(impact_sound(7), "WH_ClawsL");
        assert_eq!(impact_sound(8), "BdyCol");
        assert_eq!(impact_sound(9), "WH_SwordA");
    }
}
