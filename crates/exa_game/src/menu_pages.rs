//! The main menu's page table (`TMainMenu`'s rows): each page's
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

/// The main menu's row advance: `trunc(70·row_h)`.
pub const ROW_UNITS: f32 = 70.0;

/// The row scale from the client height: `0.5·round(H/135)/4`. The live
/// value is 0.375 at 453 px and 1.0 at 1080 px.
pub fn row_h(height: i32) -> f32 {
    let step = (height as f64 / 135.0).round_ties_even();
    (step as f32 * 0.5) / 4.0
}

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

/// The menu's hit test: a row, a miss inside the open menu, or a pass to
/// the forms behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuHit {
    /// The clicked row, 1-based.
    Row(usize),
    /// The menu is open but the point is on no row.
    Miss,
    /// The menu is not open: the point belongs to the forms behind it.
    Pass,
}

/// The original's row hit test: from the page's top, each row's band is
/// `ascent − origin_y` tall and steps by `trunc(70·row_h)`; a row's x is its
/// centred width. A disabled row takes no click but still advances. The
/// hidden flag returns a miss without testing, and a closed menu a pass.
#[allow(clippy::too_many_arguments)]
pub fn menu_hit(
    height: i32,
    origin: f32,
    row_h: f32,
    ascent: i32,
    origin_y: i32,
    selected_hidden: bool,
    menu_open: bool,
    x: i32,
    y: i32,
    width: i32,
    rows: &[Row],
    row_widths: &[i32],
) -> MenuHit {
    if selected_hidden {
        return MenuHit::Miss;
    }
    if !menu_open {
        return MenuHit::Pass;
    }
    let advance = row_advance(row_h);
    let band = ascent - origin_y;
    let mut top = (height as f32 * origin).trunc() as i32;
    for (index, row) in rows.iter().enumerate() {
        if row.enabled {
            let left = (width - row_widths.get(index).copied().unwrap_or(0)) >> 1;
            let right = left + row_widths.get(index).copied().unwrap_or(0);
            if left < x && x < right && top < y && y < top + band {
                return MenuHit::Row(index + 1);
            }
        }
        top += advance;
    }
    MenuHit::Miss
}

#[cfg(test)]
mod geometry_tests {
    use super::*;

    #[test]
    fn the_row_advance_matches_the_retail_menu() {
        assert_eq!(row_h(1080), 1.0);
        assert_eq!(row_h(453), 0.375);
        assert_eq!(row_h(720), 0.625);
        assert_eq!(row_h(1440), 1.375);
        assert_eq!(row_advance(row_h(1080)), 70);
        assert_eq!(row_advance(row_h(453)), 26);
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

/// The session mode the menu was opened over (the original's mode word):
/// its bit 2 marks an arena session and bit 1 a story in progress; the
/// values 4 and 5 are the arena and story pause menus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSession {
    /// No session: the title screen.
    None,
    /// A class select is open.
    ClassSelect,
    /// The character screen.
    Create,
    /// An arena pause menu.
    ArenaPause,
    /// A story pause menu.
    StoryPause,
    /// An arena session.
    Arena,
    /// The start menu.
    Start,
    /// Another registered mode word.
    Other(u32),
}

impl MenuSession {
    /// The mode from the original's word.
    pub fn from_mode(mode: u32) -> Self {
        match mode {
            0 => MenuSession::None,
            1 | 8 => MenuSession::ClassSelect,
            2 => MenuSession::Create,
            4 => MenuSession::ArenaPause,
            5 => MenuSession::StoryPause,
            0x20 => MenuSession::Arena,
            0x40 => MenuSession::Start,
            other => MenuSession::Other(other),
        }
    }

    /// The mode word.
    pub fn mode(self) -> u32 {
        match self {
            MenuSession::None => 0,
            MenuSession::ClassSelect => 1,
            MenuSession::Create => 2,
            MenuSession::ArenaPause => 4,
            MenuSession::StoryPause => 5,
            MenuSession::Arena => 0x20,
            MenuSession::Start => 0x40,
            MenuSession::Other(mode) => mode,
        }
    }

    /// Bit 2: an arena session.
    pub fn arena(self) -> bool {
        matches!(self, MenuSession::Arena | MenuSession::ArenaPause)
    }

    /// Bit 1 of the mode word: a story session in progress.
    pub fn story(self) -> bool {
        self.mode() & 2 != 0
    }
}

/// What a clicked row asks the app to do, in the order the original's click
/// handler decides. The caller performs the form work the original does
/// behind these commands (opening the settings form, the character screen,
/// starting a session); the command names the branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    None,
    /// Close a submenu (the original's close helper).
    CloseSubmenu,
    /// Begin a story session from the in-game page.
    BeginStory,
    OpenManual,
    OpenSettings,
    /// Leave the session and show the titled menu.
    ExitToTitle,
    /// Quit the game.
    ExitGame,
    /// Open the character creator (its mode argument) and the class select.
    NewCareer,
    /// Open the character screen (its mode argument).
    CharacterScreen,
    /// Show another page.
    ShowPage(Page),
    /// Open the character creator with the arena's class list.
    OpenArena,
    /// Start a new game at the class select with the novice class.
    Novice,
    /// Start a new game at the class select with the expert class.
    Expert,
    /// Continue an arena session.
    ContinueArena,
    /// Set the session to 0x10 (the arena's continuing mode).
    ArenaContinue,
    /// A row asked to leave the application.
    Quit,
}

/// The click handler's branch for `page`, the 1-based `row`, the session
/// mode and whether a new game is ready (the original's field 0xeff90).
pub fn menu_command(page: Page, row: usize, session: u32, new_game_ready: bool) -> MenuCommand {
    let story = session & 2 != 0;
    let arena = session & 4 != 0;
    match page {
        Page::InGame => match row {
            1 => {
                if !story {
                    MenuCommand::CloseSubmenu
                } else {
                    MenuCommand::BeginStory
                }
            }
            2 => MenuCommand::OpenManual,
            3 => MenuCommand::OpenSettings,
            4 => {
                if session == MenuSession::ArenaPause.mode() {
                    MenuCommand::ExitGame
                } else {
                    MenuCommand::ExitToTitle
                }
            }
            _ => MenuCommand::None,
        },
        Page::Start => match row {
            1 => MenuCommand::NewCareer,
            2 => MenuCommand::CharacterScreen,
            3 => MenuCommand::ShowPage(Page::ClassSelect),
            4 => {
                if new_game_ready {
                    MenuCommand::NewCareer
                } else {
                    MenuCommand::ShowPage(Page::Title)
                }
            }
            5 => MenuCommand::ShowPage(Page::Title),
            _ => MenuCommand::None,
        },
        Page::ClassSelect => match row {
            1 => MenuCommand::Novice,
            2 => MenuCommand::Expert,
            3 => MenuCommand::ShowPage(Page::Start),
            _ => MenuCommand::None,
        },
        Page::ArenaPause | Page::StoryPause => match row {
            1 => MenuCommand::CloseSubmenu,
            2 => MenuCommand::OpenSettings,
            3 => {
                if arena {
                    MenuCommand::ContinueArena
                } else {
                    MenuCommand::ArenaContinue
                }
            }
            _ => MenuCommand::None,
        },
        Page::Title => match row {
            1 => MenuCommand::OpenArena,
            2 => MenuCommand::CharacterScreen,
            3 => MenuCommand::ShowPage(Page::Start),
            4 => MenuCommand::OpenManual,
            5 => MenuCommand::OpenSettings,
            6 => MenuCommand::Quit,
            _ => MenuCommand::None,
        },
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

/// A row as the menu paints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Painted {
    pub index: usize,
    pub x: i32,
    pub y: i32,
    pub text: String,
    pub colour: [u8; 4],
    pub selected: bool,
}

/// The original's paint pass: the first row's baseline is
/// `trunc(height·origin) + ascent`; the selected row is skipped in the
/// first pass and drawn last, nudged by `unit/3` when it is the confirmed
/// row. Rows centre on `(width − row_width) >> 1`; a disabled row is grey.
#[allow(clippy::too_many_arguments)]
pub fn paint(
    width: i32,
    height: i32,
    origin: f32,
    row_h: f32,
    ascent: i32,
    selected: usize,
    confirmed: Option<usize>,
    labels: &[&str],
    text_widths: &[i32],
    styles: &[u32],
    disabled: &[bool],
    unit: i32,
) -> Vec<Painted> {
    let advance = row_advance(row_h);
    let y0 = (height as f32 * origin).trunc() as i32 + ascent;
    let mut out = Vec::with_capacity(labels.len());
    for (index, _) in labels.iter().enumerate() {
        if index == selected {
            continue;
        }
        out.push(Painted {
            index,
            x: (width - text_widths.get(index).copied().unwrap_or(0)) >> 1,
            y: y0 + index as i32 * advance,
            text: exa_ui::form::styled_label(
                styles.get(index).copied().unwrap_or(0),
                labels[index],
            ),
            colour: if disabled.get(index).copied().unwrap_or(false) {
                [0x80, 0x80, 0x80, 0x80]
            } else {
                [0xff, 0xff, 0xff, 0xff]
            },
            selected: false,
        });
    }
    if let Some(label) = labels.get(selected) {
        let mut y = y0 + selected as i32 * advance;
        if confirmed == Some(selected) {
            y += unit / 3;
        }
        out.push(Painted {
            index: selected,
            x: (width - text_widths.get(selected).copied().unwrap_or(0)) >> 1,
            y,
            text: exa_ui::form::styled_label(styles.get(selected).copied().unwrap_or(0), label),
            colour: if disabled.get(selected).copied().unwrap_or(false) {
                [0x80, 0x80, 0x80, 0x80]
            } else {
                [0xff, 0xff, 0xff, 0xff]
            },
            selected: true,
        });
    }
    out
}

#[cfg(test)]
mod command_tests {
    use super::*;

    #[test]
    fn the_menu_hit_uses_the_rows_centred_bands() {
        let state = PageState {
            has_save: true,
            arena: true,
            ..Default::default()
        };
        let rows = page_rows(Page::Title, state);
        let widths = [100, 140, 110, 120, 130, 90];
        // Top 410; the first row's band is 33 tall and 100 wide centred
        // at 750 (1600 − 100 over 2).
        let hit = menu_hit(
            1080,
            0.38,
            row_h(1080),
            49,
            16,
            false,
            true,
            800,
            420,
            1600,
            &rows,
            &widths,
        );
        assert_eq!(hit, MenuHit::Row(1));
        let hit = menu_hit(
            1080,
            0.38,
            row_h(1080),
            49,
            16,
            false,
            true,
            800,
            490,
            1600,
            &rows,
            &widths,
        );
        assert_eq!(hit, MenuHit::Row(2));
        assert_eq!(
            menu_hit(
                1080,
                0.38,
                row_h(1080),
                49,
                16,
                false,
                true,
                10,
                420,
                1600,
                &rows,
                &widths
            ),
            MenuHit::Miss
        );
        assert_eq!(
            menu_hit(
                1080,
                0.38,
                row_h(1080),
                49,
                16,
                true,
                true,
                800,
                420,
                1600,
                &rows,
                &widths
            ),
            MenuHit::Miss
        );
        assert_eq!(
            menu_hit(
                1080,
                0.38,
                row_h(1080),
                49,
                16,
                false,
                false,
                800,
                420,
                1600,
                &rows,
                &widths
            ),
            MenuHit::Pass
        );
        // A disabled row does not hit but still advances the band.
        let disabled = page_rows(Page::Title, PageState::default());
        assert_eq!(
            menu_hit(
                1080,
                0.38,
                row_h(1080),
                49,
                16,
                false,
                true,
                800,
                420,
                1600,
                &disabled,
                &widths
            ),
            MenuHit::Row(1)
        );
        assert_eq!(
            menu_hit(
                1080,
                0.38,
                row_h(1080),
                49,
                16,
                false,
                true,
                800,
                490,
                1600,
                &disabled,
                &widths
            ),
            MenuHit::Miss
        );
    }

    #[test]
    fn the_click_handler_follows_the_pages() {
        // Title: arena, character screen, start, manual, settings, quit.
        assert_eq!(
            menu_command(Page::Title, 1, 0, false),
            MenuCommand::OpenArena
        );
        assert_eq!(
            menu_command(Page::Title, 2, 0, false),
            MenuCommand::CharacterScreen
        );
        assert_eq!(
            menu_command(Page::Title, 3, 0, false),
            MenuCommand::ShowPage(Page::Start)
        );
        assert_eq!(menu_command(Page::Title, 6, 0, false), MenuCommand::Quit);
        // Start: class select, character screen, class page, new game, back.
        assert_eq!(
            menu_command(Page::Start, 1, 0, false),
            MenuCommand::NewCareer
        );
        assert_eq!(
            menu_command(Page::Start, 3, 0, false),
            MenuCommand::ShowPage(Page::ClassSelect)
        );
        assert_eq!(
            menu_command(Page::Start, 4, 0, false),
            MenuCommand::ShowPage(Page::Title)
        );
        assert_eq!(
            menu_command(Page::Start, 4, 0, true),
            MenuCommand::NewCareer
        );
        assert_eq!(
            menu_command(Page::Start, 5, 0, false),
            MenuCommand::ShowPage(Page::Title)
        );
        // In-game.
        assert_eq!(
            menu_command(Page::InGame, 1, 0, false),
            MenuCommand::CloseSubmenu
        );
        assert_eq!(
            menu_command(Page::InGame, 1, 3, false),
            MenuCommand::BeginStory
        );
        assert_eq!(
            menu_command(Page::InGame, 2, 0, false),
            MenuCommand::OpenManual
        );
        assert_eq!(
            menu_command(Page::InGame, 3, 0, false),
            MenuCommand::OpenSettings
        );
        assert_eq!(
            menu_command(Page::InGame, 4, MenuSession::ArenaPause.mode(), false),
            MenuCommand::ExitGame
        );
        assert_eq!(
            menu_command(Page::InGame, 4, 8, false),
            MenuCommand::ExitToTitle
        );
        // Class select.
        assert_eq!(
            menu_command(Page::ClassSelect, 1, 0, false),
            MenuCommand::Novice
        );
        assert_eq!(
            menu_command(Page::ClassSelect, 2, 0, false),
            MenuCommand::Expert
        );
        assert_eq!(
            menu_command(Page::ClassSelect, 3, 0, false),
            MenuCommand::ShowPage(Page::Start)
        );
        // The pause pages.
        assert_eq!(
            menu_command(Page::ArenaPause, 1, 4, false),
            MenuCommand::CloseSubmenu
        );
        assert_eq!(
            menu_command(Page::ArenaPause, 2, 4, false),
            MenuCommand::OpenSettings
        );
        assert_eq!(
            menu_command(Page::StoryPause, 3, 5, false),
            MenuCommand::ContinueArena
        );
        assert_eq!(
            menu_command(Page::StoryPause, 3, 2, false),
            MenuCommand::ArenaContinue
        );
        assert_eq!(
            menu_command(Page::ArenaPause, 3, 4, false),
            MenuCommand::ContinueArena
        );
        assert_eq!(menu_command(Page::Title, 9, 0, false), MenuCommand::None);
    }

    #[test]
    fn the_session_bits_match_the_mode_word() {
        assert!(MenuSession::from_mode(3).story());
        assert!(!MenuSession::from_mode(5).story());
        assert!(MenuSession::from_mode(4).arena());
        assert!(MenuSession::from_mode(0x20).arena());
        assert_eq!(MenuSession::from_mode(8).mode(), 1);
        assert_eq!(MenuSession::from_mode(6).mode(), 6);
        assert!(MenuSession::from_mode(6).story());
    }
}

#[cfg(test)]
mod paint_tests {
    use super::*;

    #[test]
    fn the_rows_are_centred_and_the_selected_row_is_last() {
        let labels = ["BEGIN", "CONTINUE", "ARENA", "MANUAL", "SETTINGS", "EXIT"];
        let widths = [100, 140, 110, 120, 130, 90];
        let styles = [0, 1, 0, 0, 0, 0];
        let disabled = [false, true, false, false, false, false];
        let rows = paint(
            1600,
            1080,
            0.38,
            row_h(1080),
            49,
            0,
            None,
            &labels,
            &widths,
            &styles,
            &disabled,
            6,
        );
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].y, 529);
        assert_eq!(rows[0].x, 730);
        assert_eq!(rows[0].colour, [0x80, 0x80, 0x80, 0x80]);
        assert_eq!(rows[0].text, "[CONTINUE]");
        assert_eq!(rows.last().unwrap().index, 0);
        assert_eq!(rows.last().unwrap().y, 459);
        assert_eq!(rows.last().unwrap().x, 750);
        assert!(rows.last().unwrap().selected);
    }

    #[test]
    fn the_last_selected_row_is_nudged() {
        let labels = ["A", "B", "C"];
        let rows = paint(
            100,
            100,
            0.0,
            1.0,
            0,
            2,
            Some(2),
            &labels,
            &[10, 10, 10],
            &[0, 0, 0],
            &[false, false, false],
            6,
        );
        assert_eq!(rows.last().unwrap().index, 2);
        assert_eq!(rows.last().unwrap().y, 2 * 70 + 2);
    }
}
