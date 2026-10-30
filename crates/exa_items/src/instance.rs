//! What makes one copy of an item differ from its template: its condition
//! scales the template's stats and price, a pending roll draws its quality,
//! damage and dirt within their spreads, and the AI and the character view
//! weigh it by its desirability and protection.
//!
//! The condition factor is `(1 − d·0.4/255)·(q·0.5/255 + 0.8)` in the
//! original's mix of float and double. A weapon's slash and pierce take the
//! factor, its crush `0.7c + 0.3`, its thrust the factor, and each is
//! capped at 160 (the byte wraps before the cap, as the original stores
//! it). Apparel's protection takes the factor, its coverage `0.3c + 0.7`,
//! and its encumbrance is divided by `(1 − d·0.15/255)(q·0.3/255 + 0.85)`.
//! The price is the template's divided by `2q₀² + 0.5` and multiplied by
//! `(2q² + 0.5)(2(1 − d)² + 0.5)/2`. A roll keeps a byte within its spread
//! of the stored value, leaning toward it. Evidence: the item-instance notes
//! (`context/re/item-gui-lane.md`).

use exa_formats::containers::{coins_of, price_of};

/// Spreads below this keep the stored byte.
pub const NO_SPREAD: f32 = 1.192_092_9e-7;
/// The protection weight of each of bones 1..20.
pub const BONE_WEIGHTS: [f32; 20] = [
    1.0, 1.0, 1.0, 0.5, 1.0, 1.0, 0.5, 2.0, 2.5, 4.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 0.5, 1.0,
    1.0,
];
/// Desirability of a weapon with no skill points.
pub const UNSKILLED_WEAPON: f32 = -10.0;
/// Stats a condition never raises above.
pub const STAT_CAP: u8 = 0xA0;

const COVER_QUALITY: f64 = 0.0011764705882352942;
const COVER_DAMAGE: f64 = 0.0005882352941176471;

/// The engine's random generator: one step, then the state as a float in
/// `[0, 1]`.
pub fn random_unit(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    (f64::from(*state) * (1.0 / 4_294_967_296.0)) as f32
}

/// The engine's float-to-integer conversion: nearest, ties to even.
fn nearest(x: f32) -> i64 {
    x.round_ties_even() as i64
}

fn nearest_f64(x: f64) -> i64 {
    x.round_ties_even() as i64
}

/// The condition factor of a made item, shared with the factory weapons'
/// stat pass.
pub use exa_formats::weapon_stats::condition_factor;

/// The stat bytes a weapon's condition scales, at their weapon-section
/// offsets: slash, crush and pierce for each of the two striking sides at
/// `0x37 + 6s`, thrust at `0x33` and impact at `0x32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WeaponStats {
    pub impact: u8,
    pub thrust: u8,
    pub sides: [[u8; 3]; 2],
}

/// A weapon's stats from its template's at the copy's condition. The
/// copy's own impact byte (`impact`) is only capped.
pub fn weapon_condition(
    template: &WeaponStats,
    impact: u8,
    quality: u8,
    damage: u8,
) -> WeaponStats {
    let c = condition_factor(quality, damage);
    let mut out = WeaponStats {
        impact: impact.min(STAT_CAP),
        ..*template
    };
    for s in 0..2 {
        let t = template.sides[s];
        let slash = nearest(f32::from(t[0]) * c) as u8;
        let crush = nearest_f64((f64::from(c) * 0.7 + 0.3) * f64::from(t[1])) as u8;
        let pierce = nearest(f32::from(t[2]) * c) as u8;
        out.sides[s] = [
            slash.min(STAT_CAP),
            crush.min(STAT_CAP),
            pierce.min(STAT_CAP),
        ];
    }
    out.thrust = (nearest(f32::from(template.thrust) * c) as u8).min(STAT_CAP);
    out
}

/// Apparel protection bytes: padding, slash, crush and pierce, the coverage
/// display and the encumbrance (item body `0x1A0..=0x1A5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Protection {
    pub resist: [u8; 4],
    pub coverage: u8,
    pub encumbrance: u8,
}

/// Apparel's protection from its template's at the copy's condition, then
/// its armour variant's adjustment ([`variant_scales`]).
pub fn apparel_condition(
    template: &Protection,
    quality: u8,
    damage: u8,
    variant: u8,
) -> Protection {
    let c = condition_factor(quality, damage);
    let coverage = nearest_f64((f64::from(c) * 0.3 + 0.7) * f64::from(template.coverage)) as u8;
    let resist = template
        .resist
        .map(|r| (nearest(f32::from(r) * c) as u8).min(STAT_CAP));
    let wear = ((1.0 - f64::from(damage) * COVER_DAMAGE)
        * (f64::from(quality) * COVER_QUALITY + 0.85)) as f32;
    let encumbrance = nearest(f32::from(template.encumbrance) * (1.0 / wear)) as u8;
    let mut p = Protection {
        resist,
        coverage,
        encumbrance,
    };
    if let Some((scales, enc)) = variant_scales(variant) {
        p.encumbrance = nearest_f64(f64::from(p.encumbrance) * enc) as u8;
        for (r, k) in p.resist.iter_mut().zip(scales) {
            if k != 1.0 {
                *r = nearest_f64(f64::from(*r) * k) as u8;
            }
        }
    }
    p
}

/// The armour variants 1–8 an apparel copy can be rolled with: each scales
/// the padding, slash, crush and pierce bytes and the encumbrance after the
/// condition (the bytes wrap; no cap). Variants 7 and 8 change nothing. The
/// object database's condition refresh applies the same table.
pub use exa_formats::objdb::variant_scales;

/// The compact price word of a copy at quality `quality` and damage
/// `damage`, from the template's price word and quality.
pub fn condition_price(price: u16, template_quality: u8, quality: u8, damage: u8) -> u16 {
    let coins = coins_of(price) as f32;
    let share = |byte: u8| (f64::from(byte) / 255.0) as f32;
    let curve = |x: f32| x * x * 2.0 + 0.5;
    let base = curve(share(template_quality));
    let made = curve(share(quality));
    let kept = (1.0 - f64::from(damage) / 255.0) as f32;
    let wear = kept * kept * 2.0 + 0.5;
    let scaled = coins / base * (made * (wear * 0.5));
    price_of(nearest(scaled) as u32)
}

/// What the AI and the doll weigh a weapon by: its thrust over 140 plus its
/// skill points times `(0.5q² + 0.5)(1 − 0.3d)`; a one-handed weapon keeps
/// 0.8 of that. Without skill points it is [`UNSKILLED_WEAPON`].
pub fn weapon_desirability(flags: u32, quality: u8, damage: u8, thrust: u8, skill: u8) -> f32 {
    let points = u32::from(skill & 0x1F);
    if points == 0 {
        return UNSKILLED_WEAPON;
    }
    let q = f32::from(quality) / 255.0;
    let d = f32::from(damage) / 255.0;
    let v = f32::from(thrust) / 140.0 + points as f32 * (q * q * 0.5 + 0.5) * (1.0 - d * 0.3);
    if flags & 0x30 == 0x10 { v * 0.8 } else { v }
}

/// What a shield is weighed by: its sixth body byte × 0.05.
pub fn shield_desirability(byte: u8) -> f64 {
    f64::from(byte) * 0.05
}

/// Apparel's protection value, the AI's and the character sheet's figure:
/// over the twenty bones, each nonzero coverage word `c` pairs with the
/// bone's weight `w` (same index) and gives `w·(√(c·0.01)·0.005)` in double
/// precision, rounded to single; the single-precision sum of the four
/// resistance bytes multiplies it and the total adds it, bone by bone.
pub fn protection_value(coverage: &[u32; 20], resist: [u8; 4]) -> f32 {
    let sum = resist.iter().map(|&r| u64::from(r)).sum::<u64>() as f32;
    let mut total = 0.0f32;
    for (bone, &c) in coverage.iter().enumerate() {
        if c != 0 {
            let w = f64::from(BONE_WEIGHTS[bone]) * ((f64::from(c) * 0.01).sqrt() * 0.005);
            total += sum * w as f32;
        }
    }
    total
}

/// The weighted protection and the largest single bone's, without the
/// square root: each bone gives `coverage·0.01·0.005` times the four bytes.
pub fn protection_parts(coverage: &[u32; 20], resist: [u8; 4]) -> (f32, f32) {
    let sum = resist.iter().map(|&r| u64::from(r)).sum::<u64>() as f32;
    let (mut weighted, mut most) = (0.0f32, 0.0f32);
    for (bone, &c) in coverage.iter().enumerate() {
        if c != 0 {
            let part = (f64::from(c) * 0.01 * 0.005) as f32 * sum;
            weighted += part * BONE_WEIGHTS[bone];
            if most < part {
                most = part;
            }
        }
    }
    (weighted, most)
}

/// A spread share from its stored code: codes 0 and 1 are eighths, the
/// others quarters less one.
pub fn spread_of(code: u8) -> f32 {
    if code < 2 {
        f32::from(code) * 0.125
    } else {
        (i32::from(code) - 1) as f32 * 0.25
    }
}

/// The stored code of a spread share: eighths up to one, then half the
/// eighths plus one, at most `max`.
pub fn code_of(spread: f32, max: u8) -> u8 {
    let eighths = nearest(spread * 8.0) as i32;
    if eighths > 1 {
        (((eighths >> 1) & 0x7FFF_FFFF) + 1).min(i32::from(max)) as u8
    } else {
        eighths as u8
    }
}

/// The packed spreads byte: quality in bits 0–2, damage in 3–5 (both at
/// most 5) and dirt in 6–7 (at most 3). Non-zero means a roll is pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Spreads(pub u8);

impl Spreads {
    pub fn quality(self) -> f32 {
        spread_of(self.0 & 7)
    }
    pub fn damage(self) -> f32 {
        spread_of((self.0 & 0x38) >> 3)
    }
    pub fn dirt(self) -> f32 {
        spread_of(self.0 >> 6)
    }
    pub fn set_quality(&mut self, spread: f32) {
        self.0 = self.0 & 0xF8 | code_of(spread, 5);
    }
    pub fn set_damage(&mut self, spread: f32) {
        self.0 = self.0 & 0xC7 | code_of(spread, 5) << 3;
    }
    pub fn set_dirt(&mut self, spread: f32) {
        self.0 = self.0 & 0x3F | code_of(spread, 3) << 6;
    }
}

/// Draws a byte within `spread` of `value` (a share): the range is clamped
/// to `[0, 1]`, and the draw blends `√r` and `1 − √(1 − r)` by where the
/// value sits in it, so it leans toward the value.
pub fn roll_share(value: f32, spread: f32, state: &mut u32) -> u8 {
    if spread < NO_SPREAD || spread.is_nan() {
        return nearest(value * 255.0) as u8;
    }
    let hi = (value + spread).min(1.0);
    let lo = (value - spread).max(0.0);
    let t = (value - lo) / (hi - lo);
    let r = random_unit(state);
    let up = r.sqrt();
    let down = 1.0 - (1.0 - r).sqrt();
    let w = (1.0 - t) * down + up * t;
    nearest(((hi - lo) * w + lo) * 255.0) as u8
}

/// The condition block of a made weapon's section: its two seeds, its
/// quality, damage and dirt, and the packed spreads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MadeCondition {
    pub flags: u32,
    pub seeds: [u32; 2],
    pub quality: u8,
    pub damage: u8,
    pub dirt: u8,
    pub spreads: Spreads,
}

/// The fixed-choices flag a fresh seed clears.
pub const FIXED_CHOICES: u32 = 0x1000_0000;

impl MadeCondition {
    /// Whether the copy still needs its roll: a seed is 0 or a spread is
    /// pending.
    pub fn needs_roll(&self) -> bool {
        self.seeds[0] == 0 || self.seeds[1] == 0 || self.spreads.0 != 0
    }

    /// The copy's roll. A missing first seed takes a fresh one from
    /// `entropy` for both seeds and clears [`FIXED_CHOICES`]; otherwise a
    /// pending roll reseeds the generator from `entropy`. A pending roll
    /// then draws quality, damage and dirt within their spreads and clears
    /// them.
    pub fn roll(&mut self, entropy: u32, state: &mut u32) {
        if self.seeds[0] == 0 {
            *state = entropy.wrapping_mul(0x0808_8405).wrapping_add(1);
            let seed = ((u64::from(*state) * 0x7FFF_FFFF) >> 32) as u32;
            self.seeds = [seed, seed];
            self.flags &= !FIXED_CHOICES;
        } else if self.spreads.0 != 0 {
            *state = entropy;
        }
        if self.spreads.0 != 0 {
            let share = |b: u8| (f64::from(b) / 255.0) as f32;
            self.quality = roll_share(share(self.quality), self.spreads.quality(), state);
            self.damage = roll_share(share(self.damage), self.spreads.damage(), state);
            self.dirt = roll_share(share(self.dirt), self.spreads.dirt(), state);
            self.spreads = Spreads(0);
        }
    }
}

/// What a handling sound is played as: its recorded index, its pitch and
/// its volume. Index 0 means the item has no handling sound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandlingSound {
    pub index: u8,
    pub pitch: f32,
    pub volume: f32,
}

/// The handling sound of an item whose sound index is `index`: silence for
/// 0, else the sound at pitch `0.8 + 0.2r` and volume 0.3.
pub fn handling_sound(index: u8, random: f64) -> Option<HandlingSound> {
    (index != 0).then_some(HandlingSound {
        index,
        pitch: (random * 0.2 + 0.8) as f32,
        volume: 0.3,
    })
}

/// The engine's entropy word: the clock's four bytes reversed.
pub fn entropy_of(clock: u32) -> u32 {
    clock.swap_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_condition_factor_matches_the_mixed_precision() {
        assert_eq!(condition_factor(255, 0), 1.3);
        assert!((condition_factor(0, 255) - 0.48).abs() < 1e-6);
        assert!((condition_factor(128, 64) - 0.945_470_2).abs() < 1e-6);
    }

    #[test]
    fn a_weapon_takes_its_template_at_its_condition() {
        let t = WeaponStats {
            impact: 0,
            thrust: 100,
            sides: [[100, 100, 200], [0, 50, 0]],
        };
        let s = weapon_condition(&t, 200, 255, 0);
        // 100 × 1.3 = 130; crush 100 × (0.7·1.3 + 0.3) = 121; pierce 260
        // wraps to 4 before the cap.
        assert_eq!(s.sides[0], [130, 121, 4]);
        assert_eq!(s.sides[1], [0, 60, 0]);
        assert_eq!(s.thrust, 130);
        assert_eq!(s.impact, STAT_CAP);
        let worn = weapon_condition(&t, 10, 0, 255);
        assert_eq!(worn.sides[0][0], 48);
        assert_eq!(worn.impact, 10);
    }

    #[test]
    fn apparel_takes_its_template_at_its_condition() {
        let t = Protection {
            resist: [40, 80, 150, 0],
            coverage: 100,
            encumbrance: 20,
        };
        let p = apparel_condition(&t, 255, 0, 0);
        assert_eq!(p.resist, [52, 104, STAT_CAP, 0]);
        // 0.3·1.3 + 0.7 = 1.09.
        assert_eq!(p.coverage, 109);
        // 20 / (1 · 1.15) = 17.4.
        assert_eq!(p.encumbrance, 17);
        let bad = apparel_condition(&t, 0, 255, 0);
        // 20 / (0.85 · 0.85) = 27.7.
        assert_eq!(bad.encumbrance, 28);
        // Variant 4 hardens slash and pierce by 1.35 and weighs 1.06 more;
        // the bytes are not capped again.
        let v = apparel_condition(&t, 255, 0, 4);
        assert_eq!(v.resist, [52, 140, STAT_CAP, 0]);
        assert_eq!(v.encumbrance, 18);
        let v = apparel_condition(&t, 255, 0, 3);
        assert_eq!(v.resist, [62, 125, 192, 0]);
        assert_eq!(apparel_condition(&t, 255, 0, 7), p);
        assert_eq!(apparel_condition(&t, 255, 0, 9), p);
    }

    #[test]
    fn the_price_follows_quality_and_wear() {
        // A copy at its template's quality and undamaged costs 1.25 times
        // the template: 1440 copper becomes 1800.
        assert_eq!(condition_price(0xC001, 255, 255, 0), 1800);
        // 1200 / (2·0.502² + 0.5) · 2.5 · 1.25 = 3735.
        assert_eq!(condition_price(0x4064, 128, 255, 0), 3735);
        // Fully damaged: a quarter.
        assert_eq!(condition_price(1000, 255, 255, 255), 250);
    }

    #[test]
    fn desirability_weighs_points_and_grip() {
        assert_eq!(weapon_desirability(0x20, 255, 0, 0, 0xE0), UNSKILLED_WEAPON);
        let two = weapon_desirability(0x20, 255, 0, 140, 3);
        assert!((two - 4.0).abs() < 1e-6);
        let one = weapon_desirability(0x10, 255, 0, 140, 3);
        assert!((one - 3.2).abs() < 1e-6);
        let worn = weapon_desirability(0x20, 0, 255, 0, 2);
        assert!((worn - 0.7).abs() < 1e-6);
        assert!((shield_desirability(40) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn protection_sums_the_covered_bones() {
        let mut cov = [0u32; 20];
        cov[9] = 100;
        cov[3] = 25;
        let v = protection_value(&cov, [10, 20, 30, 40]);
        // 100 × (4·1·0.005 + 0.5·0.5·0.005) = 2.125.
        assert!((v - 2.125).abs() < 1e-5);
        let (w, most) = protection_parts(&cov, [10, 20, 30, 40]);
        assert!((most - 0.5).abs() < 1e-6);
        assert!((w - (0.5 * 4.0 + 0.125 * 0.5)).abs() < 1e-6);
        // The first coverage entry is material one and takes the table's
        // first weight (one): sqrt(200 · 0.01) · 0.005.
        let mut first = [0u32; 20];
        first[0] = 200;
        let v = protection_value(&first, [1, 0, 0, 0]);
        assert!((v - (2.0f64.sqrt() * 0.005) as f32).abs() < 1e-7);
    }

    #[test]
    fn spread_codes_round_trip() {
        for (share, code) in [
            (0.0, 0),
            (0.125, 1),
            (0.25, 2),
            (0.5, 3),
            (0.75, 4),
            (1.0, 5),
        ] {
            assert_eq!(code_of(share, 5), code);
            assert_eq!(spread_of(code), share);
        }
        assert_eq!(code_of(1.0, 3), 3);
        let mut s = Spreads(0);
        s.set_quality(0.25);
        s.set_damage(1.0);
        s.set_dirt(1.0);
        assert_eq!(s.0, 2 | 5 << 3 | 3 << 6);
        assert_eq!((s.quality(), s.damage(), s.dirt()), (0.25, 1.0, 0.5));
    }

    #[test]
    fn a_roll_stays_within_its_spread() {
        let mut state = 7;
        assert_eq!(roll_share(0.5, 0.0, &mut state), 128);
        assert_eq!(state, 7);
        for _ in 0..200 {
            let b = roll_share(0.5, 0.25, &mut state);
            assert!((63..=192).contains(&b), "{b}");
        }
        let mut s0 = 0u32;
        // r = 0 gives the bottom of the range.
        let b = roll_share(0.9, 0.5, &mut s0);
        assert_eq!(s0, 1);
        assert!(b >= 102);
    }

    #[test]
    fn a_handling_sound_keeps_its_pitch_and_volume() {
        assert_eq!(handling_sound(0, 0.5), None);
        let sound = handling_sound(7, 0.5).unwrap();
        assert_eq!(sound.index, 7);
        assert_eq!(sound.volume.to_bits(), 0.3f32.to_bits());
        assert!((sound.pitch - 0.9).abs() < 1e-6);
        assert!((handling_sound(1, 1.0).unwrap().pitch - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_fresh_copy_takes_seeds_and_rolls_once() {
        let mut c = MadeCondition {
            flags: FIXED_CHOICES | 1,
            quality: 128,
            damage: 0x40,
            dirt: 0x35,
            spreads: Spreads(5 << 3),
            ..Default::default()
        };
        assert!(c.needs_roll());
        let mut state = 0;
        c.roll(0x1234_5678, &mut state);
        let expect = ((u64::from(0x1234_5678u32.wrapping_mul(0x0808_8405).wrapping_add(1))
            * 0x7FFF_FFFF)
            >> 32) as u32;
        assert_eq!(c.seeds, [expect, expect]);
        assert_eq!(c.flags, 1);
        assert_eq!(c.quality, 128);
        assert_eq!(c.dirt, 0x35);
        assert_eq!(c.spreads, Spreads(0));
        assert!(!c.needs_roll());
        let before = c;
        c.roll(9, &mut state);
        assert_eq!(c, before);
        assert_eq!(entropy_of(0x0102_0304), 0x0403_0201);
    }
}
