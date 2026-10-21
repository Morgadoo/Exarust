//! The packed item fields and amount helpers of the object database unit:
//! small getters and setters that item descriptions, the trade system, the
//! arena, the item randomisers and the record converters share. Layout and
//! evidence: `docs/formats/OTHER.md` (Object database, Folder contents and
//! Quantised item fields).
//!
//! The setters are bit-exact ports: they round as the original's
//! conversions do (to nearest, ties to even, an out-of-range or NaN value
//! giving the integer indefinite), and they keep the original's unmasked
//! ORs, so a value outside a field's range spills into the bits beside it
//! as it does in the game.

/// The original's single-precision float to integer conversion: round to
/// nearest with ties to even; NaN and values outside the 64-bit range give
/// the integer indefinite (`i64::MIN`).
pub(crate) fn round_f32(v: f32) -> i64 {
    if v.is_nan() || !(-9.223_372e18..9.223_372e18).contains(&v) {
        return i64::MIN;
    }
    v.round_ties_even() as i64
}

/// The same conversion from double precision.
pub(crate) fn round_f64(v: f64) -> i64 {
    if v.is_nan() || !(-9.223_372_036_854_775e18..9.223_372_036_854_775e18).contains(&v) {
        return i64::MIN;
    }
    v.round_ties_even() as i64
}

/// The five fractions of a folder entry's packed word (`FolderEntry`). The
/// getters and setters are the original's; the field meanings are not
/// established, so only the bit positions are named. A zero in the fields
/// at bits 9 and 14 reads as 0.125.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct FractionWord(pub u16);

impl FractionWord {
    /// Bits 0–3, ×0.25.
    pub fn level(self) -> f32 {
        (self.0 & 0xF) as f32 * 0.25
    }

    /// `round(v·4)` ORed into bits 0–3 unmasked.
    pub fn set_level(&mut self, v: f32) {
        self.0 = self.0 & 0xFFF0 | round_f32(v * 4.0) as u16;
    }

    /// Bits 6–8, ×0.25.
    pub fn field_6(self) -> f32 {
        ((self.0 & 0x1C0) >> 6) as f32 * 0.25
    }

    /// `round(v·4) << 6` ORed into bits 6–8 unmasked.
    pub fn set_field_6(&mut self, v: f32) {
        self.0 = self.0 & 0xFE3F | (round_f32(v * 4.0) << 6) as u16;
    }

    /// Bits 9–10, ×0.25, a zero reading as 0.125.
    pub fn field_9(self) -> f32 {
        let n = (self.0 & 0x600) >> 9;
        if n == 0 { 0.125 } else { n as f32 * 0.25 }
    }

    /// Below 0.1875 clears the field, below 0.625 stores `round(v·4)`, and
    /// anything else (NaN too) stores 3.
    pub fn set_field_9(&mut self, v: f32) {
        let kept = self.0 & 0xF9FF;
        self.0 = if v < 0.1875 {
            kept
        } else if v < 0.625 {
            kept | (round_f32(v * 4.0) << 9) as u16
        } else {
            kept | 0x600
        };
    }

    /// Bits 11–13, ×0.25.
    pub fn field_11(self) -> f32 {
        ((self.0 & 0x3800) >> 11) as f32 * 0.25
    }

    /// `round(v·4) << 11` ORed into bits 11–13 unmasked.
    pub fn set_field_11(&mut self, v: f32) {
        self.0 = self.0 & 0xC7FF | (round_f32(v * 4.0) << 11) as u16;
    }

    /// Bits 14–15, ×0.25, a zero reading as 0.125.
    pub fn field_14(self) -> f32 {
        let n = self.0 >> 14;
        if n == 0 { 0.125 } else { n as f32 * 0.25 }
    }

    /// At 0.2 or more (compared in double precision) stores `round(v·4)`,
    /// else clears the field.
    pub fn set_field_14(&mut self, v: f32) {
        let kept = self.0 & 0x3FFF;
        self.0 = if 0.2 <= f64::from(v) {
            kept | (round_f32(v * 4.0) << 14) as u16
        } else {
            kept
        };
    }
}

/// The quantised fields at the start of an item variant record: a 0–3
/// count and a signed fraction in its byte 0, three three-level fractions
/// in its byte 3. The records that hold them (item descriptions, trade,
/// the arena, the randomisers) are not established, so the fields are
/// named by position over the record's first four bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Quantised(pub [u8; 4]);

/// The shift, mask and cap of each three-level fraction field of byte 3.
const FRACTION_FIELDS: [(u8, u8, i32); 3] = [(0, 0x07, 5), (3, 0x38, 5), (6, 0xC0, 3)];

impl Quantised {
    /// Bits 0–1 of byte 0.
    pub fn count(self) -> u8 {
        self.0[0] & 3
    }

    /// Clamps to 0..3 (NaN gives 0) and rounds into bits 0–1.
    pub fn set_count(&mut self, value: f32) {
        let v = if 0.0 <= value { value.min(3.0) } else { 0.0 };
        self.0[0] = self.0[0] & 0xFC | round_f32(v) as u8;
    }

    /// Bits 2–6 of byte 0 as a fraction of 31, negated when bit 7 is set
    /// (computed in double precision, as the original does).
    pub fn signed(self) -> f32 {
        const STEP: f64 = 0.032_258_064_516_129_03;
        let n = i64::from((self.0[0] & 0x7C) >> 2);
        let n = if self.0[0] & 0x80 != 0 { -n } else { n };
        (n as f64 * STEP) as f32
    }

    /// `round(|v·31|) << 2` into byte 0 above the count (a magnitude past
    /// 31 spills into bit 7), then bit 7 for a negative value.
    pub fn set_signed(&mut self, value: f32) {
        let magnitude = round_f32((value * 31.0).abs());
        self.0[0] = self.0[0] & 3 | (magnitude << 2) as u8;
        if value < 0.0 {
            self.0[0] |= 0x80;
        }
    }

    /// Bits 5–7 of byte 0.
    pub fn high3(self) -> u8 {
        self.0[0] >> 5
    }

    /// Bits 0–4 of byte 0.
    pub fn low5(self) -> u8 {
        self.0[0] & 0x1F
    }

    /// A three-level fraction of byte 3 at `index` 0 (bits 0–2), 1 (bits
    /// 3–5) or 2 (bits 6–7): raw 0 and 1 read as 0 and 0.125, n ≥ 2 as
    /// (n − 1)/4.
    pub fn fraction(self, index: usize) -> f32 {
        let (shift, mask, _) = FRACTION_FIELDS[index.min(2)];
        let n = (self.0[3] & mask) >> shift;
        if n < 2 {
            n as f32 * 0.125
        } else {
            (n - 1) as f32 * 0.25
        }
    }

    /// Stores a fraction as the original does: n = round(v·8); above 1 it
    /// becomes (n >> 1) + 1, capped at 5 (3 in the two-bit field); the
    /// result is ORed in unmasked, so a negative n sets the bits above.
    pub fn set_fraction(&mut self, index: usize, value: f32) {
        let (shift, mask, cap) = FRACTION_FIELDS[index.min(2)];
        let mut n = round_f32(value * 8.0) as i32;
        if n > 1 {
            n = ((n as u32 >> 1) + 1) as i32;
            if n > cap {
                n = cap;
            }
        }
        self.0[3] = self.0[3] & !mask | (n << shift) as u8;
    }
}

/// An amount packed in four bytes as units of 1, 12 and 144 (`b0 + 12·b1 +
/// 144·w`), the form the original's amount helpers read and write.
pub fn amount_units(bytes: &[u8]) -> u32 {
    let byte = |i: usize| u32::from(bytes.get(i).copied().unwrap_or(0));
    byte(0) + 12 * byte(1) + 144 * (byte(2) | byte(3) << 8)
}

/// Packs an amount into the four-byte form as the original does: the word
/// takes `units / 144` cut to 16 bits, and the bytes split what is left,
/// so an amount past the word's range keeps only its low bits.
pub fn amount_bytes(units: u32) -> [u8; 4] {
    let word = (units / 144) as u16;
    let rest = units.wrapping_sub(u32::from(word) * 144);
    let twelves = (rest / 12) as u8;
    let ones = rest.wrapping_sub(u32::from(twelves) * 12) as u8;
    let w = word.to_le_bytes();
    [ones, twelves, w[0], w[1]]
}

/// Adds `delta` units to a packed amount; a result of zero or less clears
/// it (32-bit arithmetic, as the original's).
pub fn amount_add(bytes: &[u8], delta: i32) -> [u8; 4] {
    let sum = (amount_units(bytes) as i32).wrapping_add(delta);
    if sum <= 0 {
        [0; 4]
    } else {
        amount_bytes(sum as u32)
    }
}

/// Subtracts packed amount `b` from `a`; a result of zero or less clears
/// it.
pub fn amount_sub(a: &[u8], b: &[u8]) -> [u8; 4] {
    let diff = (amount_units(a) as i32).wrapping_sub(amount_units(b) as i32);
    if diff <= 0 {
        [0; 4]
    } else {
        amount_bytes(diff as u32)
    }
}

/// Adds two packed amounts.
pub fn amount_total(a: &[u8], b: &[u8]) -> [u8; 4] {
    amount_bytes(amount_units(a).wrapping_add(amount_units(b)))
}

/// The 16-bit compact form of an amount: the top two bits scale the low 14
/// by 12, 144 or 1440, the first scale whose range holds the amount (the
/// same packing as a model's price word, [`crate::containers::price_of`]).
pub fn amount_compact(units: u32) -> u16 {
    crate::containers::price_of(units)
}

/// Expands the 16-bit compact form back to units
/// ([`crate::containers::coins_of`]).
pub fn amount_expand(compact: u16) -> u32 {
    crate::containers::coins_of(compact)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraction_word_fields() {
        // 0x5284: bits 0-3 = 4, 6-8 = 2, 9-10 = 1, 11-13 = 2, 14-15 = 1.
        let w = FractionWord(0x5284);
        assert_eq!(w.level(), 1.0);
        assert_eq!(w.field_6(), 0.5);
        assert_eq!(w.field_9(), 0.25);
        assert_eq!(w.field_11(), 0.5);
        assert_eq!(w.field_14(), 0.25);
        assert_eq!(FractionWord(0).field_9(), 0.125);
        assert_eq!(FractionWord(0).field_14(), 0.125);
        let mut f = FractionWord(0);
        f.set_level(0.75);
        f.set_field_6(0.25);
        f.set_field_9(0.5);
        f.set_field_11(0.75);
        f.set_field_14(0.5);
        assert_eq!(f.0, 0x3 | 1 << 6 | 2 << 9 | 3 << 11 | 2 << 14);
        f.set_field_9(0.125);
        assert_eq!(f.field_9(), 0.125);
        f.set_field_9(0.9);
        assert_eq!(f.0 & 0x600, 0x600);
        f.set_field_14(0.125);
        assert_eq!(f.field_14(), 0.125);
        // 0.375·4 = 1.5 rounds to even: 2.
        f.set_field_11(0.375);
        assert_eq!(f.field_11(), 0.5);
        // Out-of-range values spill unmasked, as the original's OR does:
        // 5.0·4 = 20 = 0x14 sets bit 4; −1 << 6 sets every bit from 6 up.
        let mut f = FractionWord(0);
        f.set_level(5.0);
        assert_eq!(f.0, 0x14);
        let mut f = FractionWord(0);
        f.set_field_6(-0.25);
        assert_eq!(f.0, 0xFFC0);
        let mut f = FractionWord(0);
        f.set_field_11(2.0);
        assert_eq!(f.0, 0x4000);
    }

    #[test]
    fn quantised_count_and_signed() {
        let mut q = Quantised([0; 4]);
        q.set_count(2.6);
        assert_eq!(q.count(), 3);
        // Ties go to even.
        q.set_count(2.5);
        assert_eq!(q.count(), 2);
        q.set_count(-1.0);
        assert_eq!(q.count(), 0);
        q.set_count(9.0);
        assert_eq!(q.count(), 3);
        q.set_count(f32::NAN);
        assert_eq!(q.count(), 0);
        // 0.5·31 = 15.5 rounds to 16, not 15: byte 0 = 16 << 2.
        let mut q = Quantised([3, 0, 0, 0]);
        q.set_signed(0.5);
        assert_eq!(q.0[0], 0x40 | 3);
        assert_eq!(q.signed(), (16.0f64 * 0.032_258_064_516_129_03) as f32);
        q.set_signed(-0.5);
        assert_eq!(q.0[0], 0xC0 | 3);
        assert_eq!(q.signed(), -(16.0f64 * 0.032_258_064_516_129_03) as f32);
        q.set_signed(1.0);
        assert_eq!(q.signed(), 1.0);
        // 1.5·31 = 46.5 → 46; 46 << 2 = 0xB8 sets bit 7, so it reads back
        // negative, as in the game.
        let mut q = Quantised([0; 4]);
        q.set_signed(1.5);
        assert_eq!(q.0[0], 0xB8);
        assert!(q.signed() < 0.0);
        assert_eq!(Quantised([5 << 5, 0, 0, 0]).high3(), 5);
        assert_eq!(Quantised([0xFF, 0, 0, 0]).low5(), 31);
    }

    #[test]
    fn quantised_fractions_live_in_byte_3() {
        let mut q = Quantised([0; 4]);
        for (v, read) in [
            (0.0, 0.0),
            (0.125, 0.125),
            (0.375, 0.25),
            (0.5, 0.5),
            (0.625, 0.5),
            (0.6875, 0.75),
            (0.75, 0.75),
            (1.0, 1.0),
            (2.0, 1.0),
        ] {
            q.set_fraction(0, v);
            assert_eq!(q.fraction(0), read, "{v}");
        }
        assert_eq!(q.0, [0, 0, 0, 5]);
        q.set_fraction(1, 0.75);
        assert_eq!(q.fraction(1), 0.75);
        assert_eq!(q.0[3], 5 | 4 << 3);
        // The two-bit field caps at 3 (0.5).
        q.set_fraction(2, 1.0);
        assert_eq!(q.fraction(2), 0.5);
        // A negative value ORs −1 in unmasked: every bit of byte 3.
        let mut q = Quantised([0; 4]);
        q.set_fraction(0, -0.125);
        assert_eq!(q.0[3], 0xFF);
    }

    #[test]
    fn amount_packing() {
        for units in [0, 1, 11, 12, 13, 143, 144, 145, 1000, 100_000] {
            assert_eq!(amount_units(&amount_bytes(units)), units);
        }
        assert_eq!(amount_bytes(13), [1, 1, 0, 0]);
        assert_eq!(amount_bytes(144), [0, 0, 1, 0]);
        // Past the word's range the word wraps: 144·65536 + 150 is 65537
        // grosses, kept as 1, and 6 units.
        assert_eq!(amount_bytes(144 * 65_536 + 150), [6, 0, 1, 0]);
        assert_eq!(amount_add(&amount_bytes(10), 5), amount_bytes(15));
        assert_eq!(amount_add(&amount_bytes(10), -10), [0; 4]);
        assert_eq!(amount_add(&amount_bytes(10), -20), [0; 4]);
        assert_eq!(
            amount_sub(&amount_bytes(10), &amount_bytes(3)),
            amount_bytes(7)
        );
        assert_eq!(amount_sub(&amount_bytes(3), &amount_bytes(10)), [0; 4]);
        assert_eq!(
            amount_total(&amount_bytes(10), &amount_bytes(20)),
            amount_bytes(30)
        );
        for (units, back) in [
            (0, 0),
            (0x3FFF, 0x3FFF),
            (0x4000, 0x4000 / 12 * 12),
            (0x2FFF4, 0x2FFF4 / 12 * 12),
            (0x2FFF5, 0x2FFF5 / 144 * 144),
            (0x23FF6F, 0x23FF6F / 144 * 144),
            (0x23FF70, 0x23FF70 / 1440 * 1440),
            (1_000_000, 1_000_000 / 144 * 144),
            (3_000_000, 3_000_000 / 1440 * 1440),
        ] {
            assert_eq!(amount_expand(amount_compact(units)), back, "{units:#x}");
        }
        assert_eq!(amount_compact(0x4000), 0x4000 | (0x4000 / 12));
    }

    #[test]
    fn rounding_matches_the_conversion_instructions() {
        assert_eq!(round_f32(2.5), 2);
        assert_eq!(round_f32(3.5), 4);
        assert_eq!(round_f32(-1.5), -2);
        assert_eq!(round_f32(f32::NAN), i64::MIN);
        assert_eq!(round_f32(1e30), i64::MIN);
        assert_eq!(round_f64(0.5), 0);
        assert_eq!(round_f64(f64::INFINITY), i64::MIN);
    }
}
