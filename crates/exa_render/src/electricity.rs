//! The electricity effects' genuine unit helpers (the original's
//! `RFE_Effects_Electricity` by address): the arc colour, the part filters,
//! the spark brightness roll and the released electrified weapon effect.

/// The spark's stored colour's green channel (53).
pub const SPARK_GREEN: f32 = 53.0;

/// The spark colour: `(0, 53, 0)`.
pub fn spark_colour() -> [f32; 3] {
    [0.0, SPARK_GREEN, 0.0]
}

/// Whether a part with this kind word and sub-kind byte is a live arc part:
/// kind 1 with a byte below 0xfe, or kind 0x11 with a byte below 0x7f.
pub fn is_live_part(kind: u16, sub: u8) -> bool {
    let kind = kind & 0x1f;
    (kind == 1 && sub < 0xfe) || (kind == 0x11 && sub < 0x7f)
}

/// Whether a part is an arc-spark part: kind 4 with a byte below 0xfe.
pub fn is_spark_part(kind: u16, sub: u8) -> bool {
    kind & 0x1f == 4 && sub < 0xfe
}

/// The random brightness scale in `[0.8, 1.0)` from a roll in `[0, 1)`.
pub fn random_brightness(roll: f64) -> f32 {
    (0.8 + roll * 0.2) as f32
}

/// An electrified weapon's arc effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElectrifiedEffect {
    /// The node's kind; the high byte must be 0x21 to be the arc.
    pub kind: u32,
    /// The node's visibility byte at `+0x30d`.
    pub visible: u8,
    /// The node's state at `+0x198`.
    pub state: i32,
    /// Whether the node still carries its frame callbacks (`+0x718`/`+0x720`).
    pub callbacks: bool,
}

/// Releases the electrified weapon's arc: a 0x21-kind node hides, moves to
/// state 1 and loses its callbacks; the callbacks clear whether or not the
/// node matched.
pub fn clear_electrified_effect(effect: &mut ElectrifiedEffect) -> bool {
    let matched = effect.kind & 0xff00_0000 == 0x2100_0000;
    if matched {
        effect.visible = 0;
        effect.state = 1;
    }
    effect.callbacks = false;
    matched
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spark_colour_is_green() {
        assert_eq!(spark_colour(), [0.0, 53.0, 0.0]);
    }

    #[test]
    fn the_part_kinds() {
        assert!(is_live_part(1, 0xfd));
        assert!(!is_live_part(1, 0xfe));
        assert!(is_live_part(0x11, 0x7e));
        assert!(!is_live_part(0x11, 0x7f));
        assert!(is_spark_part(4, 0));
        assert!(!is_spark_part(5, 0));
    }

    #[test]
    fn the_random_brightness_spans_its_range() {
        assert_eq!(random_brightness(0.0), 0.8);
        assert!((random_brightness(0.5) - 0.9).abs() < 1e-6);
    }

    #[test]
    fn the_electrified_effect_clears() {
        let mut effect = ElectrifiedEffect {
            kind: 0x2100_0001,
            visible: 1,
            state: 0,
            callbacks: true,
        };
        assert!(clear_electrified_effect(&mut effect));
        assert_eq!(effect.visible, 0);
        assert_eq!(effect.state, 1);
        assert!(!effect.callbacks);

        let mut other = ElectrifiedEffect {
            kind: 0x2200_0001,
            visible: 1,
            state: 0,
            callbacks: true,
        };
        assert!(!clear_electrified_effect(&mut other));
        assert_eq!(other.visible, 1);
        assert!(!other.callbacks);
    }
}
