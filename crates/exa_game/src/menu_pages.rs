//! The main menu's page table (`TMainMenu` at `0x100122c40`): each page's
//! rows, their order, which are disabled and the row origin fraction. The
//! page geometry (row spacing) is not modelled; the retail screenshot shows
//! ~70 px rows at 1600×1080, which the raw `70·0.5·round(H/135)` reading
//! does not reproduce.

/// A main-menu page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Page {
    Title = 0,
    Start = 1,
    InGame = 2,
    ArenaPause = 4,
    StoryPause = 5,
    ClassSelect = 8,
}

/// What a row asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Begin,
    Continue,
    Arena,
    Manual,
    Settings,
    Exit,
    NewCareer,
    Practice,
    Versus,
    Back,
    NewGame,
    Forfeit,
    Novice,
    Expert,
}

/// A page row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub label: &'static str,
    pub kind: RowKind,
    pub enabled: bool,
}

/// The state the page table reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PageState {
    /// A save exists, so CONTINUE is live.
    pub has_save: bool,
    /// The game is in progress (page 2's first row).
    pub in_game: bool,
    /// The arena is available, so VERSUS appears.
    pub arena: bool,
    /// The novice class is available.
    pub novice: bool,
}

impl PageState {
    fn continue_row(self) -> Row {
        Row {
            label: "CONTINUE",
            kind: RowKind::Continue,
            enabled: self.has_save,
        }
    }

    fn manual(self) -> Row {
        Row {
            label: "MANUAL",
            kind: RowKind::Manual,
            enabled: true,
        }
    }

    fn settings(self) -> Row {
        Row {
            label: "SETTINGS",
            kind: RowKind::Settings,
            enabled: true,
        }
    }

    fn exit(self) -> Row {
        Row {
            label: "EXIT",
            kind: RowKind::Exit,
            enabled: true,
        }
    }
}

/// The rows of `page`. The start page drops VERSUS when the arena is not
/// available; the in-game page's first row is CONTINUE or NEW GAME.
pub fn page_rows(page: Page, state: PageState) -> Vec<Row> {
    match page {
        Page::Title => vec![
            Row {
                label: "BEGIN",
                kind: RowKind::Begin,
                enabled: true,
            },
            state.continue_row(),
            Row {
                label: "ARENA",
                kind: RowKind::Arena,
                enabled: true,
            },
            state.manual(),
            state.settings(),
            state.exit(),
        ],
        Page::Start => {
            let mut rows = vec![
                Row {
                    label: "NEW CAREER",
                    kind: RowKind::NewCareer,
                    enabled: true,
                },
                state.continue_row(),
                Row {
                    label: "PRACTICE",
                    kind: RowKind::Practice,
                    enabled: true,
                },
            ];
            if state.arena {
                rows.push(Row {
                    label: "VERSUS",
                    kind: RowKind::Versus,
                    enabled: true,
                });
            }
            rows.push(Row {
                label: "BACK",
                kind: RowKind::Back,
                enabled: true,
            });
            rows
        }
        Page::InGame => vec![
            if state.in_game {
                Row {
                    label: "CONTINUE",
                    kind: RowKind::Continue,
                    enabled: true,
                }
            } else {
                Row {
                    label: "NEW GAME",
                    kind: RowKind::NewGame,
                    enabled: true,
                }
            },
            state.manual(),
            state.settings(),
            state.exit(),
        ],
        Page::ArenaPause => vec![
            state.continue_row(),
            state.settings(),
            Row {
                label: "FORFEIT",
                kind: RowKind::Forfeit,
                enabled: true,
            },
        ],
        Page::StoryPause => vec![state.continue_row(), state.settings(), state.exit()],
        Page::ClassSelect => vec![
            Row {
                label: "NOVICE",
                kind: RowKind::Novice,
                enabled: state.novice,
            },
            Row {
                label: "EXPERT",
                kind: RowKind::Expert,
                enabled: true,
            },
            Row {
                label: "BACK",
                kind: RowKind::Back,
                enabled: true,
            },
        ],
    }
}

/// The page's row origin as a fraction of the client height. The start page
/// uses 0.42 with five rows and 0.45 with four; the others are fixed.
pub fn page_origin(page: Page, rows: usize) -> f32 {
    match page {
        Page::Title => 0.38,
        Page::Start => {
            if rows >= 5 {
                0.42
            } else {
                0.45
            }
        }
        Page::InGame | Page::ArenaPause | Page::StoryPause => 0.39,
        Page::ClassSelect => 0.45,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_page() {
        let state = PageState::default();
        let rows = page_rows(Page::Title, state);
        let labels: Vec<&str> = rows.iter().map(|r| r.label).collect();
        assert_eq!(
            labels,
            ["BEGIN", "CONTINUE", "ARENA", "MANUAL", "SETTINGS", "EXIT"]
        );
        assert!(!rows[1].enabled);
        assert_eq!(page_origin(Page::Title, rows.len()), 0.38);
    }

    #[test]
    fn the_start_page_drops_versus() {
        let without = page_rows(Page::Start, PageState::default());
        assert_eq!(without.len(), 4);
        assert_eq!(without[3].label, "BACK");
        assert_eq!(page_origin(Page::Start, 4), 0.45);
        let with = page_rows(
            Page::Start,
            PageState {
                arena: true,
                ..Default::default()
            },
        );
        assert_eq!(with.len(), 5);
        assert_eq!(with[3].label, "VERSUS");
        assert_eq!(page_origin(Page::Start, 5), 0.42);
    }

    #[test]
    fn the_in_game_page_switches_its_first_row() {
        let rows = page_rows(
            Page::InGame,
            PageState {
                in_game: true,
                ..Default::default()
            },
        );
        assert_eq!(rows[0].label, "CONTINUE");
        let rows = page_rows(Page::InGame, PageState::default());
        assert_eq!(rows[0].label, "NEW GAME");
        assert_eq!(rows[0].kind, RowKind::NewGame);
    }

    #[test]
    fn the_pause_pages() {
        let rows = page_rows(Page::ArenaPause, PageState::default());
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[2].label, "FORFEIT");
        let rows = page_rows(Page::StoryPause, PageState::default());
        assert_eq!(rows[2].label, "EXIT");
    }

    #[test]
    fn the_class_page_gates_novice() {
        let rows = page_rows(Page::ClassSelect, PageState::default());
        assert_eq!(rows[0].label, "NOVICE");
        assert!(!rows[0].enabled);
        let rows = page_rows(
            Page::ClassSelect,
            PageState {
                novice: true,
                ..Default::default()
            },
        );
        assert!(rows[0].enabled);
        assert_eq!(page_origin(Page::ClassSelect, rows.len()), 0.45);
    }
}
