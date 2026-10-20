//! The arena match board's show and destroy.
//!
//! Showing seeds three globals from the columns, clears five panels and a
//! present pointer and returns the base result. Destroying releases the
//! stored list and frees on positive flags.
//! Evidence: the retail match board's two methods (`docs/UI.md`, Forms).

/// The match board's show inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchBoardShowInput {
    pub seed: i32,
    pub columns: [i32; 3],
    pub panels: [i32; 5],
    pub pointer: bool,
    pub base: bool,
    pub global: bool,
}

/// What the match board's show did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchBoardShow {
    pub seed: i32,
    pub columns: [i32; 3],
    pub cleared: [bool; 5],
    pub pointer_cleared: bool,
    pub base: bool,
    pub global_call: bool,
    pub stored_cleared: bool,
}

/// Shows the match board: seed the columns, clear the panels and return.
pub fn show_match_board(input: MatchBoardShowInput) -> MatchBoardShow {
    MatchBoardShow {
        seed: input.seed,
        columns: [
            input.columns[0] - 1,
            input.columns[1] - 1,
            input.columns[2] - 1,
        ],
        cleared: input.panels.map(|count| count > 0),
        pointer_cleared: input.pointer,
        base: input.base,
        global_call: input.global,
        stored_cleared: true,
    }
}

/// What the match board's destroy did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchBoardDestroy {
    pub base_destroy: bool,
    pub list_released: bool,
    pub base: bool,
    pub freed: bool,
}

/// Destroys the match board. The object itself is present.
pub fn destroy_match_board(flags: i32) -> MatchBoardDestroy {
    MatchBoardDestroy {
        base_destroy: flags > 0,
        list_released: true,
        base: true,
        freed: flags > 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_match_board_opens_and_destroys() {
        let shown = show_match_board(MatchBoardShowInput {
            seed: 7,
            columns: [3, 1, 0],
            panels: [1, 0, 2, 0, 5],
            pointer: true,
            base: true,
            global: false,
        });
        assert_eq!(shown.seed, 7);
        assert_eq!(shown.columns, [2, 0, -1]);
        assert_eq!(shown.cleared, [true, false, true, false, true]);
        assert!(shown.pointer_cleared);
        assert!(shown.base);
        assert!(!shown.global_call);
        assert!(shown.stored_cleared);

        let destroyed = destroy_match_board(1);
        assert!(destroyed.base_destroy);
        assert!(destroyed.list_released);
        assert!(destroyed.base);
        assert!(destroyed.freed);

        let kept = destroy_match_board(0);
        assert!(!kept.base_destroy);
        assert!(kept.list_released);
        assert!(kept.base);
        assert!(!kept.freed);
    }
}
