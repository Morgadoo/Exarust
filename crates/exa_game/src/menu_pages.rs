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
    /// The expert difficulty is unlocked (practice's EXPERT row is greyed
    /// out until then).
    pub expert: bool,
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
                enabled: true,
            },
            Row {
                label: "EXPERT",
                kind: RowKind::Expert,
                enabled: state.expert,
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
    fn the_class_page_gates_expert() {
        // The page's second row takes the disabled flag until the expert
        // difficulty is unlocked.
        let rows = page_rows(Page::ClassSelect, PageState::default());
        assert_eq!(rows[0].label, "NOVICE");
        assert!(rows[0].enabled);
        assert!(!rows[1].enabled);
        let rows = page_rows(
            Page::ClassSelect,
            PageState {
                expert: true,
                ..Default::default()
            },
        );
        assert!(rows[1].enabled);
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

/// `round(70·row_h)` in single precision, half to even (the original's
/// conversion rounds with the default mode).
pub fn row_advance(row_h: f32) -> i32 {
    round_even(ROW_UNITS * row_h)
}

/// The page's top: `round(height·origin)` in single precision.
pub fn page_top(height: i32, origin: f32) -> i32 {
    round_even(height as f32 * origin)
}

/// The top of `index`'s row: the page's top plus `index` advances.
pub fn row_top(height: i32, origin: f32, row_h: f32, index: usize) -> i32 {
    page_top(height, origin) + index as i32 * row_advance(row_h)
}

/// A single-precision value rounded half to even into an integer, as the
/// original's conversion does (saturating, 0 for not-a-number).
fn round_even(value: f32) -> i32 {
    value.round_ties_even() as i32
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
    let top = page_top(height, origin);
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

/// The menu's hit test: a row, a point the menu keeps, or a point it lets
/// through to the forms behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuHit {
    /// The row under the point, 1-based.
    Row(usize),
    /// The point is on no row, and the menu keeps it (the original's -1).
    Miss,
    /// The point is on no row, and the menu lets it through (0).
    Pass,
}

/// The menu's geometry for one hit test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuGeometry {
    /// The client size.
    pub width: i32,
    pub height: i32,
    /// The page's row origin, a fraction of the height.
    pub origin: f32,
    /// The row scale.
    pub row_h: f32,
    /// The menu font's ascent and the inset the band leaves above it (the
    /// record's second origin field), both at the current scale.
    pub ascent: i32,
    pub band_inset: i32,
}

/// The original's row hit test. The first band's top is `round(H·origin)`;
/// each row's band is `ascent − band_inset` tall, and the next top is
/// `round(70·row_h + top)` in single precision. A row spans
/// `round((W − row_width)·0.5)` plus its width; every bound is strict. A
/// disabled row takes no point but still advances. While `covered` (a
/// manual opened from the menu covers it) every point is kept; otherwise a
/// point on no row is kept when the menu `blocks`, else let through.
pub fn menu_hit(
    geometry: MenuGeometry,
    covered: bool,
    blocks: bool,
    x: i32,
    y: i32,
    rows: &[Row],
    row_widths: &[i32],
) -> MenuHit {
    if covered {
        return MenuHit::Miss;
    }
    let miss = if blocks { MenuHit::Miss } else { MenuHit::Pass };
    let band = geometry.ascent - geometry.band_inset;
    let mut top = page_top(geometry.height, geometry.origin);
    for (index, row) in rows.iter().enumerate() {
        if row.enabled {
            let width = row_widths.get(index).copied().unwrap_or(0);
            let left = round_even((i64::from(geometry.width) - i64::from(width)) as f32 * 0.5);
            let (left, x, y) = (i64::from(left), i64::from(x), i64::from(y));
            let top64 = i64::from(top);
            if left < x && x < left + i64::from(width) && top64 < y && y < top64 + i64::from(band) {
                return MenuHit::Row(index + 1);
            }
        }
        top = round_even(ROW_UNITS * geometry.row_h + top as f32);
    }
    miss
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
        // 70 × 0.25 is 17.5 and 70 × 0.75 is 52.5: both round to even.
        assert_eq!(row_advance(0.25), 18);
        assert_eq!(row_advance(0.75), 52);
        // The page's top rounds: 0.37 of 1013 is 374.81.
        assert_eq!(page_top(1013, 0.37), 375);
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

/// The game-mode word's values the menu reads.
pub mod game_mode {
    /// The title menu, no game.
    pub const MENU: u32 = 0;
    pub const PRACTICE: u32 = 2;
    pub const STORY: u32 = 4;
    pub const ARENA: u32 = 8;
    /// The versus lobby, one of the script modes.
    pub const VERSUS: u32 = 0x1010;
}

/// The session-state word's values the menu reads.
pub mod session_state {
    pub const TRANSITIONING: u32 = 0;
    pub const MAIN_MENU: u32 = 1;
    /// The character creator is showing.
    pub const CREATOR: u32 = 2;
    pub const PLAYING: u32 = 3;
    pub const WON: u32 = 4;
    pub const ENDED: u32 = 5;
    pub const ROUND_WON: u32 = 8;
    pub const FORFEIT: u32 = 0x10;
    /// A setup form before a game (the class select or the emblem).
    pub const SETUP: u32 = 0x40;
}

/// What a clicked row asks the app to do, in the order the original's click
/// handler decides. The caller performs the session and form work behind
/// each command; the command names the branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    None,
    /// Close the menu and go back to the game.
    Resume,
    /// Practice's in-game NEW GAME: a new practice character from the
    /// creator.
    NewPractice,
    OpenManual,
    OpenSettings,
    /// The arena's in-game EXIT: save the arena and go back to the title.
    ExitArena,
    /// The story's in-game EXIT: the story's quit.
    QuitStory,
    /// Another game's in-game EXIT: back to the title.
    ExitToTitle,
    /// The title's EXIT: leave the application.
    Quit,
    /// Show another page.
    ShowPage(Page),
    /// The title's BEGIN: a story game, opened at the class select.
    BeginStory,
    /// A CONTINUE: the character screen of a game mode's saves.
    CharacterScreen(u32),
    /// The arena's NEW CAREER: an arena game, opened at the emblem
    /// generator.
    NewCareer,
    /// VERSUS: the versus lobby's character creator.
    Versus,
    /// A practice difficulty: a practice game from the character creator.
    Practice {
        expert: bool,
    },
    /// An end page's third row (FORFEIT or EXIT): finish the match, as a
    /// forfeit unless the session was won.
    FinishMatch {
        forfeit: bool,
    },
}

/// The click handler's branch for `page` and the 1-based `row`, from the
/// game-mode word, the session-state word and whether the versus lobby is
/// available (which also decides whether the arena page's fourth row is
/// VERSUS or BACK).
pub fn menu_command(page: Page, row: usize, game: u32, state: u32, versus: bool) -> MenuCommand {
    match page {
        Page::InGame => match row {
            1 => {
                if game & 2 == 0 {
                    MenuCommand::Resume
                } else {
                    MenuCommand::NewPractice
                }
            }
            2 => MenuCommand::OpenManual,
            3 => MenuCommand::OpenSettings,
            4 => match game {
                game_mode::ARENA => MenuCommand::ExitArena,
                game_mode::STORY => MenuCommand::QuitStory,
                _ => MenuCommand::ExitToTitle,
            },
            _ => MenuCommand::None,
        },
        Page::Start => match row {
            1 => MenuCommand::NewCareer,
            2 => MenuCommand::CharacterScreen(game_mode::ARENA),
            3 => MenuCommand::ShowPage(Page::ClassSelect),
            4 => {
                if versus {
                    MenuCommand::Versus
                } else {
                    MenuCommand::ShowPage(Page::Title)
                }
            }
            5 => MenuCommand::ShowPage(Page::Title),
            _ => MenuCommand::None,
        },
        Page::ClassSelect => match row {
            1 => MenuCommand::Practice { expert: false },
            2 => MenuCommand::Practice { expert: true },
            3 => MenuCommand::ShowPage(Page::Start),
            _ => MenuCommand::None,
        },
        Page::ArenaPause | Page::StoryPause => match row {
            1 => MenuCommand::Resume,
            2 => MenuCommand::OpenSettings,
            3 => MenuCommand::FinishMatch {
                forfeit: state & 4 == 0,
            },
            _ => MenuCommand::None,
        },
        Page::Title => match row {
            1 => MenuCommand::BeginStory,
            2 => MenuCommand::CharacterScreen(game_mode::STORY),
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
            expert: true,
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

/// The suffix a row's mark count adds to its label: `[` for one, `[[` for
/// two or more (the menu font draws those bytes as marks).
pub fn marked_label(label: &str, marks: i32) -> String {
    match marks {
        i32::MIN..=0 => label.to_string(),
        1 => format!("{label}["),
        _ => format!("{label}[["),
    }
}

/// The original's paint pass. The first row's baseline is
/// `round(H·origin) + ascent`, and each row moves down by
/// `round(70·row_h)`. Every row but the selected one is drawn in order, white
/// or, when disabled, grey at half alpha; the selected row (if any) is drawn
/// last in white with the menu's text shader, nudged down by `unit / 3`
/// when it is also the pressed row. A row's x is `(W − row_width) >> 1` as
/// an unsigned 64-bit shift: a row wider than the client lands far off to
/// the right. A row's marks add their suffix ([`marked_label`]).
#[allow(clippy::too_many_arguments)]
pub fn paint(
    width: i32,
    height: i32,
    origin: f32,
    row_h: f32,
    ascent: i32,
    selected: Option<usize>,
    pressed: Option<usize>,
    labels: &[&str],
    text_widths: &[i32],
    marks: &[i32],
    disabled: &[bool],
    unit: i32,
) -> Vec<Painted> {
    let advance = row_advance(row_h);
    let y0 = page_top(height, origin) + ascent;
    let x_of = |index: usize| {
        let spare = i64::from(width) - i64::from(text_widths.get(index).copied().unwrap_or(0));
        ((spare as u64) >> 1) as f32 as i32
    };
    let mut out = Vec::with_capacity(labels.len());
    for (index, label) in labels.iter().enumerate() {
        if Some(index) == selected {
            continue;
        }
        out.push(Painted {
            index,
            x: x_of(index),
            y: y0 + index as i32 * advance,
            text: marked_label(label, marks.get(index).copied().unwrap_or(0)),
            colour: if disabled.get(index).copied().unwrap_or(false) {
                [0x80, 0x80, 0x80, 0x80]
            } else {
                [0xff, 0xff, 0xff, 0xff]
            },
            selected: false,
        });
    }
    if let Some(selected) = selected
        && let Some(label) = labels.get(selected)
    {
        let mut y = y0 + selected as i32 * advance;
        if pressed == Some(selected) {
            y += unit / 3;
        }
        out.push(Painted {
            index: selected,
            x: x_of(selected),
            y,
            text: marked_label(label, marks.get(selected).copied().unwrap_or(0)),
            colour: [0xff, 0xff, 0xff, 0xff],
            selected: true,
        });
    }
    out
}

#[cfg(test)]
mod command_tests {
    use super::*;

    fn title_geometry() -> MenuGeometry {
        MenuGeometry {
            width: 1600,
            height: 1080,
            origin: 0.38,
            row_h: row_h(1080),
            ascent: 49,
            band_inset: 16,
        }
    }

    #[test]
    fn the_menu_hit_uses_the_rows_centred_bands() {
        let state = PageState {
            has_save: true,
            arena: true,
            ..Default::default()
        };
        let rows = page_rows(Page::Title, state);
        let widths = [100, 140, 110, 120, 130, 90];
        let g = title_geometry();
        let hit =
            |x, y, covered, blocks, rows: &[Row]| menu_hit(g, covered, blocks, x, y, rows, &widths);
        // Top round(410.4) = 410; the first band is 49 − 16 = 33 tall and
        // 100 wide from 750 (1600 − 100 over 2), every bound strict.
        assert_eq!(hit(800, 420, false, true, &rows), MenuHit::Row(1));
        assert_eq!(hit(800, 410, false, true, &rows), MenuHit::Miss);
        assert_eq!(hit(800, 442, false, true, &rows), MenuHit::Row(1));
        assert_eq!(hit(800, 443, false, true, &rows), MenuHit::Miss);
        assert_eq!(hit(750, 420, false, true, &rows), MenuHit::Miss);
        assert_eq!(hit(751, 420, false, true, &rows), MenuHit::Row(1));
        // The next band starts round(70 + 410) = 480.
        assert_eq!(hit(800, 490, false, true, &rows), MenuHit::Row(2));
        // A miss is kept when the menu blocks, else let through; a covered
        // menu keeps every point.
        assert_eq!(hit(10, 420, false, true, &rows), MenuHit::Miss);
        assert_eq!(hit(10, 420, false, false, &rows), MenuHit::Pass);
        assert_eq!(hit(800, 420, true, false, &rows), MenuHit::Miss);
        // A disabled row does not hit but still advances the band.
        let disabled = page_rows(Page::Title, PageState::default());
        assert_eq!(hit(800, 420, false, true, &disabled), MenuHit::Row(1));
        assert_eq!(hit(800, 490, false, true, &disabled), MenuHit::Miss);
        assert_eq!(hit(800, 560, false, true, &disabled), MenuHit::Row(3));
    }

    #[test]
    fn the_hit_bands_step_by_a_rounded_float_sum() {
        // At row scale 0.25 a step is 17.5 before rounding: from a top of
        // 101 the sum 118.5 rounds to the even 118, and from 118 the sum
        // 135.5 rounds to 136.
        let g = MenuGeometry {
            width: 100,
            height: 1000,
            origin: 0.101,
            row_h: 0.25,
            ascent: 10,
            band_inset: 0,
        };
        let rows = page_rows(Page::Title, PageState::default());
        let widths = [50; 6];
        let hit = |y| menu_hit(g, false, true, 50, y, &rows, &widths);
        assert_eq!(hit(102), MenuHit::Row(1));
        assert_eq!(hit(119), MenuHit::Miss);
        assert_eq!(hit(137), MenuHit::Row(3));
        assert_eq!(hit(136), MenuHit::Miss);
    }

    #[test]
    fn the_click_handler_follows_the_pages() {
        use game_mode::{ARENA, MENU, PRACTICE, STORY};
        use session_state::{ENDED, MAIN_MENU, WON};
        let command = |page, row, game, state, versus| menu_command(page, row, game, state, versus);
        // Title: BEGIN, CONTINUE, ARENA, MANUAL, SETTINGS, EXIT.
        assert_eq!(
            command(Page::Title, 1, MENU, MAIN_MENU, false),
            MenuCommand::BeginStory
        );
        assert_eq!(
            command(Page::Title, 2, MENU, MAIN_MENU, false),
            MenuCommand::CharacterScreen(STORY)
        );
        assert_eq!(
            command(Page::Title, 3, MENU, MAIN_MENU, false),
            MenuCommand::ShowPage(Page::Start)
        );
        assert_eq!(
            command(Page::Title, 4, MENU, MAIN_MENU, false),
            MenuCommand::OpenManual
        );
        assert_eq!(
            command(Page::Title, 5, MENU, MAIN_MENU, false),
            MenuCommand::OpenSettings
        );
        assert_eq!(
            command(Page::Title, 6, MENU, MAIN_MENU, false),
            MenuCommand::Quit
        );
        assert_eq!(
            command(Page::Title, 9, MENU, MAIN_MENU, false),
            MenuCommand::None
        );
        // Arena: NEW CAREER, CONTINUE, PRACTICE, then VERSUS and BACK, or
        // only BACK without the versus lobby.
        assert_eq!(
            command(Page::Start, 1, MENU, MAIN_MENU, false),
            MenuCommand::NewCareer
        );
        assert_eq!(
            command(Page::Start, 2, MENU, MAIN_MENU, false),
            MenuCommand::CharacterScreen(ARENA)
        );
        assert_eq!(
            command(Page::Start, 3, MENU, MAIN_MENU, false),
            MenuCommand::ShowPage(Page::ClassSelect)
        );
        assert_eq!(
            command(Page::Start, 4, MENU, MAIN_MENU, false),
            MenuCommand::ShowPage(Page::Title)
        );
        assert_eq!(
            command(Page::Start, 4, MENU, MAIN_MENU, true),
            MenuCommand::Versus
        );
        assert_eq!(
            command(Page::Start, 5, MENU, MAIN_MENU, true),
            MenuCommand::ShowPage(Page::Title)
        );
        // Practice difficulty.
        assert_eq!(
            command(Page::ClassSelect, 1, MENU, MAIN_MENU, false),
            MenuCommand::Practice { expert: false }
        );
        assert_eq!(
            command(Page::ClassSelect, 2, MENU, MAIN_MENU, false),
            MenuCommand::Practice { expert: true }
        );
        assert_eq!(
            command(Page::ClassSelect, 3, MENU, MAIN_MENU, false),
            MenuCommand::ShowPage(Page::Start)
        );
        // In-game: practice (bit 2 of the game mode) restarts, the others
        // resume; EXIT depends on the game.
        assert_eq!(
            command(Page::InGame, 1, STORY, 3, false),
            MenuCommand::Resume
        );
        assert_eq!(
            command(Page::InGame, 1, PRACTICE, 3, false),
            MenuCommand::NewPractice
        );
        assert_eq!(
            command(Page::InGame, 2, STORY, 3, false),
            MenuCommand::OpenManual
        );
        assert_eq!(
            command(Page::InGame, 3, STORY, 3, false),
            MenuCommand::OpenSettings
        );
        assert_eq!(
            command(Page::InGame, 4, ARENA, 3, false),
            MenuCommand::ExitArena
        );
        assert_eq!(
            command(Page::InGame, 4, STORY, 3, false),
            MenuCommand::QuitStory
        );
        assert_eq!(
            command(Page::InGame, 4, PRACTICE, 3, false),
            MenuCommand::ExitToTitle
        );
        // The end pages: the third row forfeits unless the state has bit 4.
        assert_eq!(
            command(Page::ArenaPause, 1, ARENA, WON, false),
            MenuCommand::Resume
        );
        assert_eq!(
            command(Page::ArenaPause, 2, ARENA, WON, false),
            MenuCommand::OpenSettings
        );
        assert_eq!(
            command(Page::ArenaPause, 3, ARENA, WON, false),
            MenuCommand::FinishMatch { forfeit: false }
        );
        assert_eq!(
            command(Page::StoryPause, 3, ARENA, 3, false),
            MenuCommand::FinishMatch { forfeit: true }
        );
        assert_eq!(
            command(Page::StoryPause, 3, ARENA, ENDED, false),
            MenuCommand::FinishMatch { forfeit: false }
        );
    }
}

#[cfg(test)]
mod paint_tests {
    use super::*;

    #[test]
    fn the_rows_are_centred_and_the_selected_row_is_last() {
        let labels = ["BEGIN", "CONTINUE", "ARENA", "MANUAL", "SETTINGS", "EXIT"];
        let widths = [100, 140, 110, 120, 130, 90];
        let marks = [0, 1, 0, 2, 0, 0];
        let disabled = [false, true, false, false, false, false];
        let rows = paint(
            1600,
            1080,
            0.38,
            row_h(1080),
            49,
            Some(0),
            None,
            &labels,
            &widths,
            &marks,
            &disabled,
            6,
        );
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0].index, 1);
        // round(410.4) + 49 + 70.
        assert_eq!(rows[0].y, 529);
        assert_eq!(rows[0].x, 730);
        assert_eq!(rows[0].colour, [0x80, 0x80, 0x80, 0x80]);
        assert_eq!(rows[0].text, "CONTINUE[");
        assert_eq!(rows[2].text, "MANUAL[[");
        assert_eq!(rows.last().unwrap().index, 0);
        assert_eq!(rows.last().unwrap().y, 459);
        assert_eq!(rows.last().unwrap().x, 750);
        assert!(rows.last().unwrap().selected);
        // A disabled selected row is drawn white.
        let rows = paint(
            1600,
            1080,
            0.38,
            row_h(1080),
            49,
            Some(1),
            None,
            &labels,
            &widths,
            &marks,
            &disabled,
            6,
        );
        assert_eq!(rows.last().unwrap().colour, [0xff, 0xff, 0xff, 0xff]);
        // No selection draws every row in order.
        let rows = paint(
            1600,
            1080,
            0.38,
            row_h(1080),
            49,
            None,
            None,
            &labels,
            &widths,
            &marks,
            &disabled,
            6,
        );
        assert_eq!(
            rows.iter().map(|r| r.index).collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 5]
        );
        assert!(rows.iter().all(|r| !r.selected));
    }

    #[test]
    fn the_pressed_selected_row_is_nudged() {
        let labels = ["A", "B", "C"];
        let draw = |selected, pressed| {
            paint(
                100,
                100,
                0.0,
                1.0,
                0,
                selected,
                pressed,
                &labels,
                &[10, 10, 10],
                &[0, 0, 0],
                &[false, false, false],
                6,
            )
        };
        assert_eq!(draw(Some(2), Some(2)).last().unwrap().y, 2 * 70 + 2);
        assert_eq!(draw(Some(1), Some(1)).last().unwrap().y, 70 + 2);
        assert_eq!(draw(Some(1), None).last().unwrap().y, 70);
        assert_eq!(draw(Some(1), Some(2)).last().unwrap().y, 70);
    }

    #[test]
    fn a_row_wider_than_the_client_lands_off_to_the_right() {
        let rows = paint(
            100,
            100,
            0.0,
            1.0,
            0,
            None,
            None,
            &["WIDE"],
            &[102],
            &[0],
            &[false],
            6,
        );
        // (100 − 102) as an unsigned 64-bit value, halved, is far past any
        // client width.
        assert_eq!(rows[0].x, i32::MAX);
    }
}
