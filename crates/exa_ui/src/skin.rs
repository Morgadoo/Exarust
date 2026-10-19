//! The draw skin's opening metrics. The unit is the GUI scale. The three
//! colours are the same four bytes. The base fill and the call after it are
//! counted. Their bodies are not in.

/// The four colour bytes stored, then copied twice.
pub const SKIN_INK: [u8; 4] = [0xff, 0xfa, 0xe6, 100];

/// Metrics a skin stores when it is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawSkin {
    pub span_a: i32,
    pub span_b: i32,
    pub span_c: i32,
    pub span_d: i32,
    pub pad_a: i32,
    pub pad_b: i32,
    pub pad_c: i32,
    pub pad_d: i32,
    pub inks: [[u8; 4]; 3],
    pub mark: i32,
    pub alloc_calls: u32,
    pub base_calls: u32,
    pub after_calls: u32,
}

/// Open a skin for `unit`. Argument 1 counts an allocation, and a failed
/// allocation returns nothing. Any other argument skips that count. A
/// non-zero argument counts the call after the fill.
pub fn open_skin(unit: i32, arg: i32, allocated: bool) -> Option<DrawSkin> {
    let mut alloc_calls = 0;
    let present = if arg == 1 {
        alloc_calls = 1;
        allocated
    } else {
        true
    };
    if !present {
        return None;
    }
    let doubled = unit.wrapping_shl(1);
    let mut skin = DrawSkin {
        span_a: doubled,
        span_b: unit,
        span_c: doubled,
        span_d: doubled,
        pad_a: 8,
        pad_b: 4,
        pad_c: 8,
        pad_d: 8,
        inks: [SKIN_INK; 3],
        mark: 1,
        alloc_calls,
        base_calls: 1,
        after_calls: 0,
    };
    if arg != 0 {
        skin.after_calls = 1;
    }
    Some(skin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skin_stores_the_unit_and_the_three_colours() {
        let plain = open_skin(5, 0, false).expect("a passed object");
        assert_eq!(plain.span_a, 10);
        assert_eq!(plain.span_b, 5);
        assert_eq!(plain.span_c, 10);
        assert_eq!(plain.span_d, 10);
        assert_eq!(
            [plain.pad_a, plain.pad_b, plain.pad_c, plain.pad_d],
            [8, 4, 8, 8]
        );
        assert_eq!(plain.inks, [SKIN_INK; 3]);
        assert_eq!(plain.mark, 1);
        assert_eq!(plain.alloc_calls, 0);
        assert_eq!(plain.base_calls, 1);
        assert_eq!(plain.after_calls, 0);

        let made = open_skin(5, 1, true).expect("an allocation");
        assert_eq!(made.span_a, 10);
        assert_eq!(made.alloc_calls, 1);
        assert_eq!(made.after_calls, 1);
        assert_eq!(made.inks[2], [0xff, 0xfa, 0xe6, 100]);

        assert!(open_skin(5, 1, false).is_none());

        let other = open_skin(5, 2, false).expect("a passed object");
        assert_eq!(other.alloc_calls, 0);
        assert_eq!(other.after_calls, 1);
        assert_eq!(other.span_b, 5);
    }
}
