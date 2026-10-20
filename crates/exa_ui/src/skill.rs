//! The skill screen's node placement.
//!
//! A positive index names a row and a column in the screen's layout; a
//! non-positive index takes the scaled base or the centre of the bounds.
//! Evidence: the 186-byte `TSkillUI` method at `0x100224570`.

/// The skill screen's bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillBounds {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

/// Places a skill node from its index.
pub fn skill_position(
    index: i32,
    bounds: SkillBounds,
    scaled: bool,
    base: (i32, i32),
    layout: impl FnOnce(i32, i32) -> (f32, f32),
) -> (i32, i32) {
    if index > 0 {
        let (x, y) = layout((index >> 8) & 0xff, index & 0xff);
        (x.round_ties_even() as i32, y.round_ties_even() as i32)
    } else if scaled {
        base
    } else {
        (
            (bounds.right - bounds.left) / 2,
            (bounds.bottom - bounds.top) / 2,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skill_node_is_placed_centred_or_forwarded() {
        let bounds = SkillBounds {
            left: 10,
            right: 110,
            top: 20,
            bottom: 80,
        };
        let placed = skill_position(0x0205, bounds, false, (0, 0), |row, col| {
            assert_eq!((row, col), (2, 5));
            (12.5, 34.5)
        });
        assert_eq!(placed, (12, 34));
        assert_eq!(
            skill_position(0, bounds, false, (0, 0), |_, _| unreachable!()),
            (50, 30)
        );
        assert_eq!(
            skill_position(-1, bounds, true, (7, 9), |_, _| unreachable!()),
            (7, 9)
        );
    }
}
