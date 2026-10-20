//! The character's active item effects (the original's equipment-effect
//! helpers, split out of `RFE_Effects_Electricity` by address). Each entry
//! pairs an item with its tick; the visuals use a power curve, a charge
//! accumulator and a per-part glow toggle.

/// The charge gained per tick as a share of the source value (0.004).
pub const CHARGE_RATE: f64 = 0.004;

/// The glow level used when a part's stored level is at most 0x40 (0x3c).
pub const FLICKER_FLOOR: u8 = 0x3c;

/// One tick of charge accumulation: `min(current + source·0.004, source)`.
pub fn accumulate_charge(current: f32, source: f32) -> f32 {
    let next = (current as f64 + source as f64 * CHARGE_RATE) as f32;
    if next > source { source } else { next }
}

/// The glow intensity from a stored level: levels above 0x40 step down by
/// four, lower levels use 0x3c, and the result is `0.5·(0.01·level)^0.7`.
pub fn flicker_intensity(level: u8) -> f32 {
    let value = if level > 0x40 {
        level - 4
    } else {
        FLICKER_FLOOR
    };
    (0.5 * (value as f64 * 0.01).powf(0.7)) as f32
}

/// A worn part's glow weight: 1 when its low bit is set and its byte is
/// below 0xff, otherwise 0.
pub fn part_glow(kind: u16, sub: u8) -> f32 {
    if kind & 1 != 0 && sub < 0xff {
        1.0
    } else {
        0.0
    }
}

/// Adds an entry keyed by `id` unless one is already present.
pub fn attach_entry<T: Copy>(entries: &mut Vec<(i32, T)>, id: i32, value: T) {
    if !entries.iter().any(|(key, _)| *key == id) {
        entries.push((id, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_charge_accumulates_toward_the_source() {
        assert!((accumulate_charge(0.0, 100.0) - 0.4).abs() < 1e-5);
        assert_eq!(accumulate_charge(99.9, 100.0), 100.0);
        assert_eq!(accumulate_charge(2.0, 1.0), 1.0);
    }

    #[test]
    fn the_flicker_follows_the_power_curve() {
        let floor = 0.5 * 0.6f64.powf(0.7) as f32;
        assert!((flicker_intensity(0x3c) - floor).abs() < 1e-6);
        assert!((flicker_intensity(0x40) - floor).abs() < 1e-6);
        assert!((flicker_intensity(0x44) - 0.5 * 0.64f64.powf(0.7) as f32).abs() < 1e-6);
    }

    #[test]
    fn the_part_glow_needs_an_odd_live_kind() {
        assert_eq!(part_glow(1, 0), 1.0);
        assert_eq!(part_glow(2, 0), 0.0);
        assert_eq!(part_glow(1, 0xff), 0.0);
    }

    #[test]
    fn an_entry_is_attached_once() {
        let mut entries = vec![(1, 10u32)];
        attach_entry(&mut entries, 1, 11);
        assert_eq!(entries, vec![(1, 10)]);
        attach_entry(&mut entries, 2, 12);
        assert_eq!(entries, vec![(1, 10), (2, 12)]);
    }
}
