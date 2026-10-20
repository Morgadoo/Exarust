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

/// The main menu's row advance: `trunc(70·row_h)`. The live menu's
/// `row_h` is 1.0 at 1600×1080, giving the ~70 px rows the retail
/// screenshot shows; the scale rule that produces `row_h` is not decoded.
pub const ROW_UNITS: f32 = 70.0;

/// `trunc(70·row_h)`.
pub fn row_advance(row_h: f32) -> i32 {
    (ROW_UNITS * row_h).trunc() as i32
}

/// The top of `index`'s row: `trunc(height·origin) + index·advance`.
pub fn row_top(height: i32, origin: f32, row_h: f32, index: usize) -> i32 {
    (height as f32 * origin).trunc() as i32 + index as i32 * row_advance(row_h)
}

/// The row containing `y`, using the original's `top <= y < top+band`.
pub fn hit_row(
    height: i32,
    origin: f32,
    row_h: f32,
    band: i32,
    count: usize,
    y: i32,
) -> Option<usize> {
    let advance = row_advance(row_h);
    if advance <= 0 || band <= 0 {
        return None;
    }
    let top = (height as f32 * origin).trunc() as i32;
    if y < top {
        return None;
    }
    let index = ((y - top) / advance) as usize;
    if index < count && y < top + index as i32 * advance + band {
        Some(index)
    } else {
        None
    }
}

#[cfg(test)]
mod geometry_tests {
    use super::*;

    #[test]
    fn the_row_advance_matches_the_retail_menu() {
        assert_eq!(row_advance(1.0), 70);
        assert_eq!(row_advance(0.5), 35);
        assert_eq!(row_advance(1.5), 105);
    }

    #[test]
    fn the_title_rows_and_hit_band() {
        assert_eq!(row_top(1080, 0.38, 1.0, 0), 410);
        assert_eq!(row_top(1080, 0.38, 1.0, 1), 480);
        assert_eq!(hit_row(1080, 0.38, 1.0, 70, 6, 456), Some(0));
        assert_eq!(hit_row(1080, 0.38, 1.0, 70, 6, 525), Some(1));
        assert_eq!(hit_row(1080, 0.38, 1.0, 70, 6, 409), None);
        assert_eq!(hit_row(1080, 0.38, 1.0, 70, 6, 900), None);
    }
}

/// What activating a row asks the menu to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Begin,
    Continue,
    NewGame,
    NewCareer,
    Practice,
    Versus,
    Arena,
    Manual,
    Settings,
    Exit,
    Back,
    Forfeit,
    Novice,
    Expert,
}

impl RowKind {
    /// The action this row's kind performs.
    pub fn action(self) -> MenuAction {
        match self {
            RowKind::Begin => MenuAction::Begin,
            RowKind::Continue => MenuAction::Continue,
            RowKind::Arena => MenuAction::Arena,
            RowKind::Manual => MenuAction::Manual,
            RowKind::Settings => MenuAction::Settings,
            RowKind::Exit => MenuAction::Exit,
            RowKind::NewCareer => MenuAction::NewCareer,
            RowKind::Practice => MenuAction::Practice,
            RowKind::Versus => MenuAction::Versus,
            RowKind::Back => MenuAction::Back,
            RowKind::NewGame => MenuAction::NewGame,
            RowKind::Forfeit => MenuAction::Forfeit,
            RowKind::Novice => MenuAction::Novice,
            RowKind::Expert => MenuAction::Expert,
        }
    }
}

/// A page's selection, stored as a row index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuState {
    pub page: Page,
    pub selected: usize,
}

impl MenuState {
    /// Starts on the page's first row.
    pub fn new(page: Page) -> Self {
        Self { page, selected: 0 }
    }

    /// Moves the selection by `delta`, clamped to the row range.
    pub fn move_selection(&mut self, delta: i32, state: PageState) {
        let count = page_rows(self.page, state).len() as i32;
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = (self.selected as i32 + delta).clamp(0, count - 1) as usize;
    }

    /// The action of the selected row, or `None` when it is disabled.
    pub fn activate(&self, state: PageState) -> Option<MenuAction> {
        let rows = page_rows(self.page, state);
        let row = rows.get(self.selected)?;
        if !row.enabled {
            return None;
        }
        Some(row.kind.action())
    }
}

#[cfg(test)]
mod action_tests {
    use super::*;

    #[test]
    fn the_title_selection_clamps() {
        let mut menu = MenuState::new(Page::Title);
        let state = PageState::default();
        assert_eq!(menu.selected, 0);
        menu.move_selection(1, state);
        assert_eq!(menu.selected, 1);
        menu.move_selection(-1, state);
        assert_eq!(menu.selected, 0);
        menu.move_selection(-1, state);
        assert_eq!(menu.selected, 0);
        menu.move_selection(99, state);
        assert_eq!(menu.selected, 5);
    }

    #[test]
    fn a_disabled_row_does_not_activate() {
        let mut menu = MenuState::new(Page::Title);
        menu.move_selection(1, PageState::default());
        assert_eq!(menu.activate(PageState::default()), None);
        assert_eq!(
            menu.activate(PageState {
                has_save: true,
                ..Default::default()
            }),
            Some(MenuAction::Continue)
        );
    }

    #[test]
    fn the_page_actions() {
        let state = PageState {
            arena: true,
            novice: true,
            ..Default::default()
        };
        let start = page_rows(Page::Start, state);
        assert_eq!(start[0].kind.action(), MenuAction::NewCareer);
        assert_eq!(start[3].kind.action(), MenuAction::Versus);
        assert_eq!(start[4].kind.action(), MenuAction::Back);
        let class = page_rows(Page::ClassSelect, state);
        assert_eq!(class[0].kind.action(), MenuAction::Novice);
        assert_eq!(class[1].kind.action(), MenuAction::Expert);
        let game = page_rows(Page::InGame, PageState::default());
        assert_eq!(game[0].kind.action(), MenuAction::NewGame);
    }
}
