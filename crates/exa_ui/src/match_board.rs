//! The arena match board (`TMatchBoard`): the four tabs, the six standard
//! match types, the rank and tier lists, the daily event table with its
//! coin columns, the tournament winners and season tables, the company and
//! lineup assignment, the day's three-match limit, and the form's layout
//! metrics and drawing order.
//!
//! The board edits two shared rosters. The left table lists the company's
//! fighters who are not in the lineup; the right lists the lineup itself.
//! A fighter moves between them by double click or drag and drop, and the
//! lineup is capped by the match's side size. Rows are kept in the order
//! of their ordinal, which is the fighter's slot in the company pool.
//!
//! Match selection is by rank: a fighter may join when its rank reaches
//! the selected floor, except for the Captain event (match type 9), whose
//! teammates may be two ranks lower. The day allows three matches; a
//! seventh day is the tournament day and a season is four weeks.
//!
//! Evidence: the retail match board's constructor, tab switch, list
//! refreshes and fighter assignment (`docs/UI.md`, Forms).

/// The six competitor ranks, from the board's rank list.
pub const RANKS: [&str; 6] = ["Inept", "Aspirant", "Novice", "Adept", "Expert", "Master"];
/// The six always-listed standard match types.
pub const MATCH_TYPES: [&str; 6] = ["Duel", "Doubles", "Skirmish", "Fray", "Reserve", "Pugilism"];
/// The four tab labels. The first three are the original's titles; the last
/// is a guess (`docs/UI.md` names the tab "the season").
pub const TABS: [&str; 4] = ["Matches", "Events", "Tournament", "Season"];
/// The board's title.
pub const TITLE: &str = "Matches";
/// Matches a company may enter in one day.
pub const MATCHES_PER_DAY: u32 = 3;
/// The Captain event's id. Its teammates may be two ranks below the floor.
pub const CAPTAIN_EVENT: u32 = 9;
/// A team member's leader byte. The player counts as the leader too.
pub const LEADER_BIT: u8 = 1;
/// A fighter whose class byte is above this is not a combatant.
pub const COMBATANT_CLASS: u8 = 2;
/// The text when the day's three matches are spent.
pub const NO_MORE_MATCHES: &str = "[algn=2]There are no more matches running this day.\r\nEnd the day to participate in more matches.";
/// The rank text of an absent or unknown rank.
pub const NO_RANK: &str = "-";
/// The tournament status when nobody has entered.
pub const TOURNAMENT_IN_PROGRESS: &str = "Tournament is in progress.";
/// The tournament status when every entered event has a winner.
pub const TOURNAMENT_ENDED: &str = "The tournament has ended.";
/// The tournament status when entered events were settled but the week runs.
pub const NO_MORE_EVENTS: &str = "No more events entered.";

/// The form's table slot. The letters name the board's child views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableSlot {
    /// The six standard match types.
    Match,
    /// The rank floor list.
    Rank,
    /// The tier list, bounded by the rank.
    Tier,
    /// The six daily events with entry and prize.
    Events,
    /// The six tournament winners, one per tier.
    Winners,
    /// The company's fighters who are not in the lineup.
    Roster,
    /// The fighters chosen for the match.
    Lineup,
    /// The season's four weekly winners.
    Season,
}

/// A table's column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Column {
    pub title: &'static str,
    pub width: i32,
}

/// A table's fixed layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableLayout {
    pub slot: TableSlot,
    pub rect: Rect,
    pub columns: &'static [Column],
    pub rows: usize,
}

/// A rectangle in the form's client coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    /// The right edge.
    pub fn right(self) -> i32 {
        self.x + self.w
    }

    /// The bottom edge.
    pub fn bottom(self) -> i32 {
        self.y + self.h
    }

    /// Whether a point lies in the half-open rectangle.
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
}

const MATCH_COLUMNS: [Column; 1] = [Column {
    title: "Match",
    width: 196,
}];
const RANK_COLUMNS: [Column; 1] = [Column {
    title: "Rank",
    width: 192,
}];
const TIER_COLUMNS: [Column; 1] = [Column {
    title: "Tier",
    width: 192,
}];
const EVENT_COLUMNS: [Column; 5] = [
    Column {
        title: "Match",
        width: 140,
    },
    Column {
        title: "Rank",
        width: 100,
    },
    Column {
        title: "Tier",
        width: 100,
    },
    Column {
        title: "Entry",
        width: 114,
    },
    Column {
        title: "Prize",
        width: 106,
    },
];
const WINNER_COLUMNS: [Column; 3] = [
    Column {
        title: "Event",
        width: 120,
    },
    Column {
        title: "Tier",
        width: 100,
    },
    Column {
        title: "Winner",
        width: 120,
    },
];
const FIGHTER_COLUMNS: [Column; 3] = [
    Column {
        title: "Name",
        width: 140,
    },
    Column {
        title: "Rank",
        width: 68,
    },
    Column {
        title: "Health",
        width: 132,
    },
];
const SEASON_COLUMNS: [Column; 1] = [Column {
    title: "Tournament winners",
    width: 160,
}];

/// The board's layout at one form size. The compact form is four pixels
/// shorter per line in the team table and 20 pixels lower in the tournament
/// reward panel; the form itself loses 20 pixels. The original picks it
/// from a display flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardLayout {
    pub form: Rect,
    pub title_y: i32,
    pub tab_strip: Rect,
    pub tabs: [Rect; 4],
    pub match_table: TableLayout,
    pub rank_table: TableLayout,
    pub tier_table: TableLayout,
    pub events_table: TableLayout,
    pub winners_table: TableLayout,
    pub roster_table: TableLayout,
    pub lineup_table: TableLayout,
    pub season_table: TableLayout,
    pub message: Rect,
    pub picture: Rect,
    pub reward_panel: Rect,
    pub enter_button: Rect,
    pub entered_label: Rect,
    pub status_label: Rect,
    pub reward_slots: [Rect; 4],
}

/// The row height. Six event rows fill the table's 180 pixels; the roster's
/// eight rows fill 240 of its 244. The toolkit's own row pitch is not in the
/// board's methods, so this is a guess from those fits.
pub const ROW_HEIGHT: i32 = 30;

/// Builds the board's layout. `compact` shortens the form, the team table
/// and the reward panel. The tab strip, the button and label rectangles and
/// the reward-slot rectangles are not in the board's own methods; they are
/// placed from the form template's order and are guesses.
pub fn layout(compact: bool) -> BoardLayout {
    let height = if compact { 636 } else { 656 };
    let team_height = if compact { 224 } else { 244 };
    let reward_height = if compact { 100 } else { 120 };
    BoardLayout {
        form: Rect {
            x: 0,
            y: 0,
            w: 604,
            h: height,
        },
        title_y: 8,
        tab_strip: Rect {
            x: 8,
            y: 8,
            w: 588,
            h: 24,
        },
        tabs: [
            Rect {
                x: 8,
                y: 8,
                w: 147,
                h: 24,
            },
            Rect {
                x: 155,
                y: 8,
                w: 147,
                h: 24,
            },
            Rect {
                x: 302,
                y: 8,
                w: 147,
                h: 24,
            },
            Rect {
                x: 449,
                y: 8,
                w: 147,
                h: 24,
            },
        ],
        match_table: TableLayout {
            slot: TableSlot::Match,
            rect: Rect {
                x: 0,
                y: 56,
                w: 196,
                h: 180,
            },
            columns: &MATCH_COLUMNS,
            rows: 6,
        },
        rank_table: TableLayout {
            slot: TableSlot::Rank,
            rect: Rect {
                x: 200,
                y: 56,
                w: 192,
                h: 180,
            },
            columns: &RANK_COLUMNS,
            rows: 6,
        },
        tier_table: TableLayout {
            slot: TableSlot::Tier,
            rect: Rect {
                x: 396,
                y: 56,
                w: 192,
                h: 180,
            },
            columns: &TIER_COLUMNS,
            rows: 6,
        },
        events_table: TableLayout {
            slot: TableSlot::Events,
            rect: Rect {
                x: 0,
                y: 56,
                w: 588,
                h: 180,
            },
            columns: &EVENT_COLUMNS,
            rows: 6,
        },
        winners_table: TableLayout {
            slot: TableSlot::Winners,
            rect: Rect {
                x: 0,
                y: 56,
                w: 392,
                h: 180,
            },
            columns: &WINNER_COLUMNS,
            rows: 6,
        },
        roster_table: TableLayout {
            slot: TableSlot::Roster,
            rect: Rect {
                x: 0,
                y: 348,
                w: 292,
                h: team_height,
            },
            columns: &FIGHTER_COLUMNS,
            rows: 8,
        },
        lineup_table: TableLayout {
            slot: TableSlot::Lineup,
            rect: Rect {
                x: 296,
                y: 348,
                w: 292,
                h: team_height,
            },
            columns: &FIGHTER_COLUMNS,
            rows: 8,
        },
        season_table: TableLayout {
            slot: TableSlot::Season,
            rect: Rect {
                x: 32,
                y: 60,
                w: 280,
                h: 132,
            },
            columns: &SEASON_COLUMNS,
            rows: 4,
        },
        message: Rect {
            x: 40,
            y: 276,
            w: 520,
            h: 96,
        },
        picture: Rect {
            x: 396,
            y: 56,
            w: 192,
            h: 180,
        },
        reward_panel: Rect {
            x: 296,
            y: 472,
            w: 292,
            h: reward_height,
        },
        enter_button: Rect {
            x: 396,
            y: 600,
            w: 192,
            h: 28,
        },
        entered_label: Rect {
            x: 40,
            y: 250,
            w: 240,
            h: 20,
        },
        status_label: Rect {
            x: 40,
            y: 276,
            w: 520,
            h: 40,
        },
        reward_slots: [
            Rect {
                x: 300,
                y: 476,
                w: 60,
                h: 60,
            },
            Rect {
                x: 368,
                y: 476,
                w: 60,
                h: 60,
            },
            Rect {
                x: 436,
                y: 476,
                w: 60,
                h: 60,
            },
            Rect {
                x: 504,
                y: 476,
                w: 60,
                h: 60,
            },
        ],
    }
}

/// The four tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardTab {
    Matches = 0,
    Events = 1,
    Tournament = 2,
    Season = 3,
}

impl BoardTab {
    /// The tab at a zero-based index, if it exists.
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Matches),
            1 => Some(Self::Events),
            2 => Some(Self::Tournament),
            3 => Some(Self::Season),
            _ => None,
        }
    }

    /// The tab's zero-based index.
    pub fn index(self) -> usize {
        self as usize
    }

    /// The tab's label.
    pub fn label(self) -> &'static str {
        TABS.get(self.index()).copied().unwrap_or(TITLE)
    }
}

/// A control's state word. Bit 0x100 hides it and bit 0x10 disables it;
/// the tab states also carry 0x60 when selected. These bits are read from
/// the values the board writes, not from the toolkit's own documentation.
pub const CONTROL_HIDDEN: u32 = 0x100;
/// The disabled bit.
pub const CONTROL_DISABLED: u32 = 0x10;
/// The selected-tab bits.
pub const CONTROL_SELECTED: u32 = 0x60;

/// The board's controls, in the form template's order. Only the ones the
/// board's own methods touch are named.
pub mod control {
    use super::{CONTROL_DISABLED, CONTROL_HIDDEN, CONTROL_SELECTED};

    /// The four tab buttons.
    pub const TAB_MATCHES: usize = 0;
    pub const TAB_EVENTS: usize = 1;
    pub const TAB_TOURNAMENT: usize = 2;
    pub const TAB_SEASON: usize = 3;
    /// The standard tab's four rows.
    pub const MATCH_ROW_0: usize = 4;
    pub const MATCH_ROW_3: usize = 7;
    /// Two labels shared by the non-season tabs; they are hidden while the
    /// season tab is open.
    pub const SHARED_LABEL_0: usize = 8;
    pub const SHARED_LABEL_1: usize = 9;
    /// The tournament tab's panel and label.
    pub const TOURNAMENT_PANEL: usize = 10;
    pub const TOURNAMENT_PANEL_LABEL: usize = 11;
    /// The season-available marker.
    pub const SEASON_MARKER: usize = 12;
    /// The primary Enter/Begin/Continue button.
    pub const ENTER: usize = 14;
    /// The secondary tournament button.
    pub const SECONDARY: usize = 15;
    /// The tournament status text.
    pub const STATUS: usize = 16;
    /// The season picture and its four reward items.
    pub const SEASON_PICTURE: usize = 17;
    pub const SEASON_ITEM_0: usize = 18;
    pub const SEASON_ITEM_1: usize = 19;
    pub const SEASON_ITEM_2: usize = 20;
    pub const SEASON_ITEM_3: usize = 21;
    /// The season tab's last line.
    pub const SEASON_TAIL: usize = 22;

    /// A shown, enabled control.
    pub const SHOWN: u32 = 0;
    /// A shown but disabled control.
    pub const DISABLED: u32 = CONTROL_DISABLED;
    /// A hidden control.
    pub const HIDDEN: u32 = CONTROL_HIDDEN | CONTROL_DISABLED;
    /// A selected tab.
    pub const TAB_SELECTED: u32 = CONTROL_SELECTED | CONTROL_DISABLED;

    /// Whether a state word hides its control.
    pub fn hidden(state: u32) -> bool {
        state & CONTROL_HIDDEN != 0
    }

    /// Whether a state word disables its control.
    pub fn disabled(state: u32) -> bool {
        state & CONTROL_DISABLED != 0
    }

    /// Whether a tab state word is selected.
    pub fn selected(state: u32) -> bool {
        state & CONTROL_SELECTED == CONTROL_SELECTED
    }
}

/// The number of control slots the methods touch.
pub const CONTROL_COUNT: usize = 23;

/// A daily event row. The board shows the match name, its rank, a second
/// rank (or a dash for the merchant event), the entry fee and the prize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardEvent {
    pub name: String,
    /// The event kind byte. 8 is the merchant event, shown as a dash.
    pub kind: i32,
    /// Bit 0 marks an event the company already entered.
    pub flags: u32,
    /// The rank at the event's first rank column.
    pub rank: i32,
    /// The rank at the event's second rank column; a negative value is
    /// shown as Inept.
    pub second_rank: i32,
    /// The packed copper entry fee.
    pub fee: u32,
    /// The packed copper prize.
    pub prize: u32,
}

impl Default for BoardEvent {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: 0,
            flags: 0,
            rank: 0,
            second_rank: -1,
            fee: 0,
            prize: 0,
        }
    }
}

/// One fighter entered in a tournament event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Participant {
    /// The fighter's slot in the company pool.
    pub slot: usize,
    /// The value the fighter's record holds for that event.
    pub value: i32,
}

/// A tournament event's entry and winner.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventEntry {
    pub participants: Vec<Participant>,
    /// The winning company id: 0 for none, 1 for the player, then the
    /// rival companies from id 2.
    pub winner: i32,
}

/// A fighter the board can list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fighter {
    pub name: String,
    /// The board's rank, 0 through 5, as [`rank_of`] gives it.
    pub rank: u8,
    /// Health for the roster table's health column.
    pub health: u16,
    /// The company record's class byte: 0 through 2 is a combatant and
    /// bit 0 marks the leader. The bit's meaning is inferred from the
    /// board's one-leader rule.
    pub class: u8,
    /// The entity's marked flag (the original's `0x10000000`). A marked
    /// fighter takes the place of an existing marked teammate.
    pub marked: bool,
    /// The fighter's record value for each tournament event.
    pub event_values: [i32; 6],
}

impl Fighter {
    /// A fighter with full health and no marked flag.
    pub fn new(name: impl Into<String>, rank: u8, class: u8) -> Self {
        Self {
            name: name.into(),
            rank,
            health: u16::MAX,
            class,
            marked: false,
            event_values: [0; 6],
        }
    }

    /// Whether the company record counts this fighter as a combatant.
    pub fn is_combatant(&self) -> bool {
        self.class <= COMBATANT_CLASS
    }

    /// Whether the record's low bit marks a leader.
    pub fn is_leader(&self) -> bool {
        self.class & LEADER_BIT != 0
    }
}

/// The rank a fighter's six skill levels give: the sum halved and capped
/// at Master.
pub fn rank_of(skills: &[u32]) -> u8 {
    let sum = skills
        .iter()
        .take(6)
        .fold(0u32, |sum, &level| sum.saturating_add(level));
    ((sum / 2).min(5)) as u8
}

/// The rank's name. A negative rank is the dash the original stores right
/// after Master; ranks past Master are clamped to a dash too, since their
/// in-memory neighbours are not rank names.
pub fn rank_name(rank: i32) -> &'static str {
    if rank < 0 {
        NO_RANK
    } else {
        RANKS.get(rank as usize).copied().unwrap_or(NO_RANK)
    }
}

/// The second rank column of a daily event. The merchant event shows a
/// dash; a negative rank shows Inept. A rank past Master is clamped to
/// Master, a guess for the original's unchecked table read.
pub fn event_tier_name(event: &BoardEvent) -> &'static str {
    if event.kind == 8 {
        NO_RANK
    } else if event.second_rank < 0 {
        RANKS[0]
    } else {
        RANKS
            .get(event.second_rank as usize)
            .copied()
            .unwrap_or(RANKS[5])
    }
}

/// A packed coin amount's copper, silver and gold counts. The high 16 bits
/// are gold, then a silver byte and a copper byte, each silver worth 12
/// copper.
pub fn coin_parts(packed: u32) -> (u32, u32, u32) {
    (packed & 0xff, (packed >> 8) & 0xff, packed >> 16)
}

/// Whether the day's three matches are spent. The counter is the signed
/// int the original compares.
pub fn matches_left(played: i32) -> bool {
    played < MATCHES_PER_DAY as i32
}

/// Whether a day is the weekly tournament day.
pub fn is_tournament_day(day: u32) -> bool {
    day.is_multiple_of(7)
}

/// Days until the weekly tournament, from a non-tournament day.
pub fn tournament_begins_in(day: u32) -> u32 {
    7 - day % 7
}

/// The season index: four weeks per season, four seasons a year.
pub fn season_index(day: u32) -> u32 {
    (((day as u64 + 6) / 28) % 4) as u32
}

/// The number of weeks of the current season that have been played. The
/// last six days of the four-week block are week zero of the next season.
pub fn season_weeks_elapsed(day: u32) -> u32 {
    let day = day % 28;
    if day > 21 {
        0
    } else {
        let weeks = day / 7;
        if day.is_multiple_of(7) {
            weeks
        } else {
            weeks + 1
        }
    }
}

/// The season's reward picture name, `seasonrwrd{a..d}.rfi`.
pub fn season_picture_name(day: u32) -> String {
    let letter = b'a' + season_index(day) as u8;
    format!("seasonrwrd{}.rfi", letter as char)
}

/// The four season rewards in the order the board's item slots take them:
/// first, third, second, fourth.
pub fn season_reward_order(rewards: [u32; 4]) -> [u32; 4] {
    [rewards[0], rewards[2], rewards[1], rewards[3]]
}

/// A fighter's rank passes the board's floor. The Captain event lets a
/// teammate be two ranks below the floor; the match's first fighter still
/// needs the full floor.
pub fn rank_passes(match_id: u32, floor: i32, rank: i32, first: bool) -> bool {
    if match_id == CAPTAIN_EVENT && !first {
        floor.saturating_sub(2) <= rank
    } else {
        floor <= rank
    }
}

/// The board's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    /// The compact form size, which shortens the form, the fighter tables
    /// and the reward panel.
    pub compact: bool,
    /// The arena day; seven-day weeks and 28-day seasons are read from it.
    pub day: u32,
    /// The open tab.
    pub tab: BoardTab,
    /// The selected standard match type, 0 through 5.
    pub match_type: usize,
    /// The selected rank floor, 0 through 5.
    pub rank: usize,
    /// The selected tier, 0 through 5.
    pub tier: usize,
    /// The global rank floor the fighter filter reads.
    pub rank_floor: usize,
    /// Fighters per side for the selected match.
    pub team_size: usize,
    /// Matches the company has entered today; the day allows three.
    pub matches_played: i32,
    /// Whether more than one leader-class fighter may join (the flag word's
    /// low bit).
    pub allow_multiple_leaders: bool,
    /// The tournament-layout flag (the original's flag word bit 2).
    pub tournament_mode: bool,
    /// Whether the tournament's week is over (the caller's flag).
    pub tournament_ended: bool,
    /// Whether the season tab's activity marker is on.
    pub season_available: bool,
    /// The player's company name.
    pub company_name: String,
    /// The 19 rival companies, in id order from 2.
    pub rivals: Vec<String>,
    /// Who won each of the season's four weeks.
    pub week_winners: [u32; 4],
    /// The four seasons' four rewards.
    pub season_rewards: [[u32; 4]; 4],
    /// The six tournament event names.
    pub tournament_events: [String; 6],
    /// The six daily events.
    pub events: [BoardEvent; 6],
    /// The tournament entries and winners.
    pub entries: [EventEntry; 6],
    /// The company pool: index 0 is the player, then the roster.
    pub fighters: Vec<Fighter>,
    /// The lineup order, a permutation of the pool with an unused marker
    /// for the non-combatants.
    pub lineup: Vec<i32>,
    /// Pool slots shown in the company (available) table.
    pub available: Vec<usize>,
    /// Pool slots shown in the lineup table.
    pub team: Vec<usize>,
    /// The event table's selection (zero-based).
    pub event_selection: usize,
    /// The winners table's selection (zero-based).
    pub winner_selection: usize,
    /// The company table's selected row.
    pub available_selection: usize,
    /// The lineup table's selected row.
    pub team_selection: usize,
    /// The season table's selected row.
    pub season_selection: usize,
    /// The raw event table top row, which the refresh can move past the
    /// last row when every event was entered.
    pub events_top: usize,
    /// The standard tab's message.
    pub standard_message: String,
    /// The tournament tab's note beside the reward item.
    pub tournament_note: String,
    /// The selected tournament event for the entry check.
    pub entrant_event: usize,
    /// The selected bracket row for the entry check.
    pub entrant_row: usize,
    /// The fighter slot whose entry is being checked.
    pub entrant_slot: usize,
    /// The item id the tournament tab's reward slot gets when it opens.
    pub tournament_prize_item: Option<u32>,
    /// The tournament reward item id, if one was created.
    pub tournament_reward: Option<u32>,
    /// The four season reward item ids, in slot order.
    pub season_items: [Option<u32>; 4],
    /// The control states, indexed by [`control`].
    pub controls: [u32; CONTROL_COUNT],
    /// The last standard refresh's two list spans: one and three rank steps.
    pub rank_span: i32,
    pub tier_span: i32,
    /// The lineup table's height after a tournament refresh, when the
    /// tournament layout shortens it to four rows.
    pub lineup_height_override: Option<i32>,
}

impl Board {
    /// A board with the player at pool slot 0, `roster` after it, and the
    /// standard tab open. The board does not own the installation's data:
    /// the caller fills the events, entries and names.
    pub fn new(compact: bool, day: u32, fighters: Vec<Fighter>) -> Self {
        let mut controls = [0u32; CONTROL_COUNT];
        for state in controls.iter_mut().take(4) {
            *state = control::DISABLED;
        }
        let mut board = Self {
            compact,
            day,
            tab: BoardTab::Matches,
            match_type: 0,
            rank: 0,
            tier: 0,
            rank_floor: 0,
            team_size: 1,
            matches_played: 0,
            allow_multiple_leaders: false,
            tournament_mode: false,
            tournament_ended: false,
            season_available: true,
            company_name: String::new(),
            rivals: Vec::new(),
            week_winners: [0; 4],
            season_rewards: [[0; 4]; 4],
            tournament_events: std::array::from_fn(|_| String::new()),
            events: std::array::from_fn(|_| BoardEvent::default()),
            entries: std::array::from_fn(|_| EventEntry::default()),
            fighters,
            lineup: Vec::new(),
            available: Vec::new(),
            team: Vec::new(),
            event_selection: 0,
            winner_selection: 0,
            available_selection: 0,
            team_selection: 0,
            season_selection: 0,
            events_top: 1,
            standard_message: String::new(),
            tournament_note: String::new(),
            entrant_event: 0,
            entrant_row: 0,
            entrant_slot: 0,
            tournament_prize_item: None,
            tournament_reward: None,
            season_items: [None; 4],
            controls,
            rank_span: 1,
            tier_span: 3,
            lineup_height_override: None,
        };
        board.rebuild_lineups();
        board.apply_tab(None, BoardTab::Matches);
        board
    }

    /// The layout for the board's form size.
    pub fn layout(&self) -> BoardLayout {
        layout(self.compact)
    }

    /// The selected match type's id, one through six.
    pub fn match_id(&self) -> u32 {
        self.match_type as u32 + 1
    }

    /// The fighter at a pool slot.
    pub fn fighter(&self, slot: usize) -> Option<&Fighter> {
        self.fighters.get(slot)
    }

    /// A fighter's per-event record value by pool slot.
    pub fn event_value(&self, slot: usize, event: usize) -> Option<i32> {
        self.fighters
            .get(slot)
            .and_then(|fighter| fighter.event_values.get(event))
            .copied()
    }

    /// Whether the tournament day has any entries.
    pub fn has_entrants(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| !entry.participants.is_empty())
    }

    /// The weekly tournament's status text and the two button states.
    pub fn tournament_status(&self) -> TournamentStatus {
        let mut status = TournamentStatus {
            text: String::new(),
            button_label: "Enter",
            enter: control::SHOWN,
            secondary: control::HIDDEN,
            text_state: control::HIDDEN,
        };
        if !is_tournament_day(self.day) {
            let days = tournament_begins_in(self.day);
            status.text = if days == 1 {
                "Tournament begins tomorrow.".to_string()
            } else {
                format!("Tournament begins in {} days.", days)
            };
            status.enter = control::HIDDEN;
            status.secondary = control::HIDDEN;
            status.text_state = control::SHOWN;
            return status;
        }
        let entered = self
            .entries
            .iter()
            .any(|entry| !entry.participants.is_empty());
        let unsettled = self
            .entries
            .iter()
            .any(|entry| !entry.participants.is_empty() && entry.winner == 0);
        if entered && unsettled {
            status.enter = control::SHOWN;
            status.secondary = control::SHOWN;
            status.text_state = control::HIDDEN;
            status.button_label = if self.entries[0].winner == 0 {
                "Begin"
            } else {
                "Continue"
            };
        } else if entered {
            status.text = if self.tournament_ended {
                TOURNAMENT_ENDED.to_string()
            } else {
                NO_MORE_EVENTS.to_string()
            };
            status.enter = control::HIDDEN;
            status.secondary = control::HIDDEN;
            status.text_state = control::SHOWN;
        } else {
            status.text = TOURNAMENT_IN_PROGRESS.to_string();
            status.enter = control::HIDDEN;
            status.secondary = control::HIDDEN;
            status.text_state = control::SHOWN;
        }
        status
    }

    /// The "Entered n/m" label. The original writes a single character by
    /// adding 0x30 to each count, so a count above nine wraps into
    /// punctuation rather than becoming two digits.
    pub fn entered_label(&self) -> String {
        format!(
            "Entered {}/{}",
            count_char(self.team.len()),
            count_char(self.team_size)
        )
    }

    /// The primary button's state after a lineup change.
    pub fn enter_state(&self) -> u32 {
        if self.tab == BoardTab::Season {
            return control::HIDDEN;
        }
        if self.tab == BoardTab::Tournament {
            return self.tournament_status().enter;
        }
        let mut state = if matches_left(self.matches_played) {
            control::SHOWN
        } else {
            control::DISABLED
        };
        if matches_left(self.matches_played) {
            if self.team.len() < self.team_size {
                state |= control::DISABLED;
            } else {
                state &= !control::DISABLED;
            }
        }
        state
    }

    /// The standard tab's refresh: clamp the tier to the rank, disable the
    /// tier rows at and above the rank, and size the two list spans.
    pub fn refresh_standard(&mut self) {
        if self.tier > self.rank {
            self.tier = self.rank;
        }
        self.rank_span = self.rank as i32 + 1;
        self.tier_span = self.rank_span * 3;
        self.rebuild_lineups();
    }

    /// Whether a tier row is disabled by the selected rank.
    pub fn tier_row_disabled(&self, row: usize) -> bool {
        row >= self.rank
    }

    /// An event's selection after a refresh. An entered event moves the
    /// selection to the first event that is not entered; when all six are
    /// entered the table's top row moves past the last row.
    pub fn refresh_events(&mut self) {
        let selected = self
            .events
            .get(self.event_selection)
            .cloned()
            .unwrap_or_default();
        if selected.flags & 1 != 0 {
            if let Some(found) = self.events.iter().position(|event| event.flags & 1 == 0) {
                self.event_selection = found;
                self.events_top = found + 1;
            } else {
                self.events_top = 6;
            }
        }
        self.rebuild_lineups();
    }

    /// Rebuilds the lineup and the company table from the pool.
    pub fn rebuild_lineups(&mut self) {
        let count = self.fighters.len();
        if self.lineup.len() != count {
            self.lineup = (1..count)
                .map(|slot| {
                    if self
                        .fighters
                        .get(slot)
                        .map(Fighter::is_combatant)
                        .unwrap_or(false)
                    {
                        slot as i32
                    } else {
                        i32::MAX
                    }
                })
                .collect();
            self.lineup.push(0);
        }
        self.team.clear();
        self.available.clear();
        let mut used = vec![false; count];
        if self.tab == BoardTab::Tournament {
            for participant in self
                .entries
                .get(self.winner_selection)
                .map(|entry| entry.participants.as_slice())
                .unwrap_or(&[])
            {
                if participant.slot < count && !used[participant.slot] {
                    used[participant.slot] = true;
                    self.team.push(participant.slot);
                }
            }
        } else {
            let mut have_leader = false;
            for &slot in &self.lineup {
                if slot == i32::MAX || slot < 0 {
                    continue;
                }
                let slot = slot as usize;
                if slot >= count {
                    continue;
                }
                if used[slot] {
                    continue;
                }
                let Some(fighter) = self.fighters.get(slot) else {
                    continue;
                };
                if slot > 0 && !fighter.is_combatant() {
                    continue;
                }
                let first = self.team.is_empty();
                if !rank_passes(
                    self.match_id(),
                    self.rank_floor as i32,
                    fighter.rank as i32,
                    first,
                ) {
                    continue;
                }
                if slot > 0 && fighter.is_leader() {
                    if have_leader && !self.allow_multiple_leaders {
                        continue;
                    }
                    have_leader = true;
                }
                if slot == 0 {
                    have_leader = true;
                }
                used[slot] = true;
                self.team.push(slot);
                if self.team.len() >= self.team_size {
                    break;
                }
            }
        }
        self.available_selection = 0;
        self.team_selection = 0;
        for (slot, &is_used) in used.iter().enumerate() {
            if is_used {
                continue;
            }
            if slot > 0
                && !self
                    .fighters
                    .get(slot)
                    .map(Fighter::is_combatant)
                    .unwrap_or(false)
            {
                continue;
            }
            self.available.push(slot);
        }
    }

    /// Whether a fighter may be moved into the lineup by a double click.
    pub fn accepts_rank(&self, slot: usize) -> bool {
        self.fighters.get(slot).is_some_and(|fighter| {
            rank_passes(
                self.match_id(),
                self.rank_floor as i32,
                fighter.rank as i32,
                false,
            )
        })
    }

    /// Whether a fighter may be dropped onto the lineup.
    pub fn can_drop(&self, slot: usize) -> bool {
        self.accepts_rank(slot)
    }

    /// Moves an available fighter into the lineup. `target` is a zero-based
    /// lineup slot for a drop, or `None` for a double click. Returns
    /// whether the tables changed.
    pub fn move_to_team(&mut self, index: usize, target: Option<usize>) -> bool {
        if self.assignment_locked() {
            return false;
        }
        let Some(&slot) = self.available.get(index) else {
            return false;
        };
        let Some(fighter) = self.fighters.get(slot) else {
            return false;
        };
        let mut unknown_rank = false;
        if self.match_id() == CAPTAIN_EVENT && self.team.len() > 2 {
            let high = self
                .team
                .iter()
                .filter(|&&member| {
                    self.fighters
                        .get(member)
                        .is_some_and(|fighter| fighter.rank as i32 >= self.rank_floor as i32)
                })
                .count();
            if high < 2 {
                unknown_rank = true;
            }
        }
        let marked = fighter.marked;
        let mut slot_in_team = None;
        if !self.allow_multiple_leaders
            && marked
            && let Some(found) = self.team.iter().position(|&member| {
                self.fighters
                    .get(member)
                    .is_some_and(|fighter| fighter.marked)
            })
        {
            // A drop on the marked fighter's own row keeps the drop target.
            let target = if self.team[found] == slot {
                target
                    .filter(|&drop| drop < self.team.len())
                    .or(Some(found))
            } else {
                Some(found)
            };
            let Some(drop) = target else {
                return false;
            };
            if unknown_rank
                && self
                    .team
                    .get(drop)
                    .and_then(|&member| self.fighters.get(member))
                    .is_some_and(|fighter| fighter.rank as i32 >= self.rank_floor as i32)
            {
                return false;
            }
            self.available[index] = self.team[drop];
            self.team[drop] = slot;
            slot_in_team = Some(drop);
        }
        if slot_in_team.is_none() {
            if let Some(drop) = target {
                if drop >= self.team.len() {
                    // The original indexes the lineup with the drop row
                    // unchecked; an empty lineup would write before it.
                    if self.team.len() < self.team_size {
                        self.team.push(slot);
                        self.available.remove(index);
                        slot_in_team = Some(self.team.len() - 1);
                    } else {
                        return false;
                    }
                } else {
                    if unknown_rank
                        && self
                            .team
                            .get(drop)
                            .and_then(|&member| self.fighters.get(member))
                            .is_some_and(|fighter| fighter.rank as i32 >= self.rank_floor as i32)
                    {
                        return false;
                    }
                    self.available[index] = self.team[drop];
                    self.team[drop] = slot;
                    slot_in_team = Some(drop);
                }
            } else if self.team.len() < self.team_size {
                self.team.push(slot);
                self.available.remove(index);
                slot_in_team = Some(self.team.len() - 1);
            } else if !self.team.is_empty() {
                let benched = self.team.pop().unwrap_or(slot);
                self.available.push(benched);
                self.team.push(slot);
                self.available.remove(index);
                slot_in_team = Some(self.team.len() - 1);
            } else {
                return false;
            }
        }
        if self.match_id() == CAPTAIN_EVENT && self.team.len() > 2 {
            let first_rank = self
                .team
                .first()
                .and_then(|&member| self.fighters.get(member))
                .map(|fighter| fighter.rank as i32)
                .unwrap_or(0);
            if first_rank < self.rank_floor as i32
                && let Some(found) = (1..self.team.len()).find(|&member| {
                    self.team
                        .get(member)
                        .and_then(|&slot| self.fighters.get(slot))
                        .is_some_and(|fighter| fighter.rank as i32 >= self.rank_floor as i32)
                })
            {
                self.team.swap(0, found);
            }
        }
        if let Some(drop) = slot_in_team
            && self.tab != BoardTab::Tournament
        {
            self.move_in_lineup(slot, drop);
        }
        if self.tab == BoardTab::Tournament {
            self.record_participants();
        }
        true
    }

    /// Moves a lineup fighter back to the company table, inserting it in
    /// slot order.
    pub fn move_to_available(&mut self, index: usize) -> bool {
        if self.assignment_locked() {
            return false;
        }
        if index >= self.team.len() {
            return false;
        }
        let slot = self.team.remove(index);
        self.insert_available(slot);
        if self.tab == BoardTab::Tournament {
            self.record_participants_removed(index);
        }
        true
    }

    /// A double click on a company fighter: the rank check guards the move.
    pub fn double_click_available(&mut self, index: usize) -> bool {
        if self.assignment_locked() {
            return false;
        }
        let Some(&slot) = self.available.get(index) else {
            return false;
        };
        if !self.accepts_rank(slot) {
            return false;
        }
        self.move_to_team(index, None)
    }

    /// A double click on a lineup fighter returns it to the company table.
    pub fn double_click_team(&mut self, index: usize) -> bool {
        self.move_to_available(index)
    }

    /// Whether fighter assignment is locked: on a tournament day the
    /// tournament tab's lineup cannot be edited.
    pub fn assignment_locked(&self) -> bool {
        is_tournament_day(self.day) && self.tab == BoardTab::Tournament
    }

    /// Inserts a pool slot into the company table in slot order. Equal
    /// slots keep their existing order; the original inserts before the
    /// first row whose ordinal is greater.
    pub fn insert_available(&mut self, slot: usize) {
        let at = self
            .available
            .iter()
            .position(|&row| row > slot)
            .unwrap_or(self.available.len());
        self.available.insert(at, slot);
    }

    /// Moves the pool slot in the lineup order to a lineup position.
    pub fn move_in_lineup(&mut self, slot: usize, target: usize) {
        let position = self.lineup.iter().position(|&row| row == slot as i32);
        if let Some(position) = position {
            self.lineup.remove(position);
        }
        let target = target.min(self.lineup.len());
        self.lineup.insert(target, slot as i32);
    }

    /// Records the current lineup in the selected tournament event.
    pub fn record_participants(&mut self) {
        let participants: Vec<Participant> = self
            .team
            .iter()
            .map(|&slot| Participant {
                slot,
                value: self.event_value(slot, self.winner_selection).unwrap_or(0),
            })
            .collect();
        if let Some(entry) = self.entries.get_mut(self.winner_selection) {
            entry.participants = participants;
        }
    }

    /// Removes a lineup fighter from the selected event's participants.
    fn record_participants_removed(&mut self, index: usize) {
        let Some(entry) = self.entries.get_mut(self.winner_selection) else {
            return;
        };
        if index < entry.participants.len() {
            entry.participants.remove(index);
        }
    }

    /// The entry-check message: `None` when the event has no entrants (the
    /// message is left as it is), `Some(false)` when this fighter is in the
    /// bracket, and `Some(true)` when the original shows its message.
    /// The original's message text is not decodable from the board's data,
    /// so only the decision is modelled; the key it compares (a participant
    /// slot here) is a guess between the entry's slot and its record value.
    pub fn entrant_message(&self) -> Option<bool> {
        if !self.tournament_mode {
            return None;
        }
        let entry = self.entries.get(self.entrant_event)?;
        if entry.participants.is_empty() {
            return None;
        }
        Some(
            !entry
                .participants
                .iter()
                .any(|participant| participant.slot == self.entrant_slot),
        )
    }

    /// The tournament tab's refresh: the reward panel is shown only in the
    /// tournament layout, where the lineup table is four rows tall.
    pub fn refresh_tournament(&mut self) {
        if self.tournament_mode {
            self.controls[control::TOURNAMENT_PANEL_LABEL] = control::SHOWN;
            self.lineup_height_override = Some(4 * ROW_HEIGHT);
        } else {
            self.controls[control::TOURNAMENT_PANEL_LABEL] = control::HIDDEN;
            self.lineup_height_override = None;
        }
        self.standard_message.clear();
        self.rebuild_lineups();
    }

    /// The season tab's four winner lines.
    pub fn season_winners(&self) -> [String; 4] {
        std::array::from_fn(|row| self.season_winner(row))
    }

    /// One season winner line. Past weeks with no winner say so; a zero or
    /// out-of-range winner is a dash, id 1 is the player, and the rest are
    /// the rival companies in id order from 2.
    pub fn season_winner(&self, row: usize) -> String {
        let winner = self.week_winners.get(row).copied().unwrap_or(0);
        let weeks = season_weeks_elapsed(self.day);
        if (row as u32) < weeks && winner == 0 {
            return "No winner".to_string();
        }
        if winner == 0 || winner > 20 {
            return NO_RANK.to_string();
        }
        if winner == 1 {
            return self.company_name.clone();
        }
        self.rivals
            .get((winner - 2) as usize)
            .cloned()
            .unwrap_or_default()
    }

    /// Fills the season's reward item ids and returns the picture name.
    pub fn refresh_season(&mut self) -> String {
        let season = season_index(self.day) as usize;
        let rewards = self.season_rewards.get(season).copied().unwrap_or([0; 4]);
        let ordered = season_reward_order(rewards);
        self.season_items = ordered.map(|id| if id == 0 { None } else { Some(id) });
        self.controls[control::ENTER] = control::HIDDEN;
        for index in [
            control::SEASON_PICTURE,
            control::SEASON_ITEM_0,
            control::SEASON_ITEM_1,
            control::SEASON_ITEM_2,
            control::SEASON_ITEM_3,
            control::SEASON_TAIL,
        ] {
            if let Some(state) = self.controls.get_mut(index) {
                *state = control::SHOWN;
            }
        }
        season_picture_name(self.day)
    }

    /// The show method: reset the day's tournament reward, recompute the
    /// tournament flags, and reopen the current tab - or force the
    /// tournament tab on a tournament day with entries.
    pub fn show(&mut self) -> ShowState {
        self.tournament_reward = None;
        let tournament_day = self.tournament_day();
        let has_entrants = self.has_entrants();
        let target = if tournament_day && has_entrants {
            BoardTab::Tournament
        } else {
            self.tab
        };
        // The show method reopens the form; the caller clears the tab strip
        // before it, which the bank of tab states cannot see.
        for index in 0..4 {
            self.controls[index] &= !CONTROL_SELECTED;
        }
        self.apply_tab(None, target);
        ShowState {
            tournament_day,
            has_entrants,
            tab: self.tab,
            forced_tournament: tournament_day && has_entrants,
        }
    }

    /// The close method: remember the tab and the three list selections.
    pub fn close(&mut self) -> CloseState {
        let state = CloseState {
            tab: self.tab,
            match_type: self.match_type,
            rank: self.rank,
            tier: self.tier,
            tournament_reward: self.tournament_reward.take(),
            season_items: self.season_items,
            picture: season_picture_name(self.day),
        };
        self.season_items = [None; 4];
        self.team.clear();
        self.available.clear();
        state
    }

    /// Whether the tournament day flag is set.
    pub fn tournament_day(&self) -> bool {
        is_tournament_day(self.day)
    }

    /// Applies a tab. `old` is the tab being left; `None` skips hiding,
    /// which is what the show method does when it reopens a tab.
    pub fn apply_tab(&mut self, old: Option<BoardTab>, new: BoardTab) -> TabView {
        let forced = self.tournament_day() && self.has_entrants();
        let new = if forced { BoardTab::Tournament } else { new };
        if forced {
            self.controls[control::TAB_MATCHES] = control::DISABLED;
            self.controls[control::TAB_EVENTS] = control::DISABLED;
        } else {
            self.controls[control::TAB_MATCHES] &= !control::DISABLED;
            self.controls[control::TAB_EVENTS] &= !control::DISABLED;
            self.controls[control::ENTER] = if matches_left(self.matches_played) {
                control::SHOWN
            } else {
                control::DISABLED
            };
        }
        self.controls[control::SEASON_MARKER] = if self.season_available {
            control::SHOWN
        } else {
            control::DISABLED
        };
        let mut hidden = [false; 4];
        if let Some(old) = old {
            self.hide_tab(old);
            hidden[old.index()] = true;
        }
        if new != BoardTab::Tournament {
            self.lineup_height_override = None;
        }
        self.tab = new;
        self.controls[new.index()] |= CONTROL_SELECTED;
        self.show_tab(new);
        TabView {
            tab: new,
            hidden,
            controls: self.controls,
        }
    }

    /// Switches tabs, doing nothing when the tab is already open.
    pub fn switch_tab(&mut self, new: BoardTab) -> Option<TabView> {
        let target = if self.tournament_day() && self.has_entrants() {
            BoardTab::Tournament
        } else {
            new
        };
        if target == self.tab {
            return None;
        }
        Some(self.apply_tab(Some(self.tab), target))
    }

    fn hide_tab(&mut self, tab: BoardTab) {
        let index = tab.index();
        self.controls[index] &= !CONTROL_SELECTED;
        match tab {
            BoardTab::Matches => {
                for control in control::MATCH_ROW_0..=control::MATCH_ROW_3 {
                    self.controls[control] = control::HIDDEN;
                }
            }
            BoardTab::Events => {}
            BoardTab::Tournament => {
                self.controls[control::TOURNAMENT_PANEL] = control::HIDDEN;
                self.controls[control::TOURNAMENT_PANEL_LABEL] = control::HIDDEN;
                self.controls[control::SECONDARY] = control::HIDDEN;
                self.controls[control::STATUS] = control::HIDDEN;
                self.tournament_reward = None;
            }
            BoardTab::Season => {
                self.controls[control::SHARED_LABEL_0] = control::SHOWN;
                self.controls[control::SHARED_LABEL_1] = control::SHOWN;
                for control in [
                    control::SEASON_PICTURE,
                    control::SEASON_ITEM_0,
                    control::SEASON_ITEM_1,
                    control::SEASON_ITEM_2,
                    control::SEASON_ITEM_3,
                    control::SEASON_TAIL,
                ] {
                    self.controls[control] = control::HIDDEN;
                }
            }
        }
    }

    fn show_tab(&mut self, tab: BoardTab) {
        match tab {
            BoardTab::Matches => {
                for control in control::MATCH_ROW_0..=control::MATCH_ROW_3 {
                    self.controls[control] = control::SHOWN;
                }
                self.refresh_standard();
            }
            BoardTab::Events => {
                self.refresh_events();
            }
            BoardTab::Tournament => {
                if self.tournament_reward.is_none() {
                    self.tournament_reward = self.tournament_prize_item;
                }
                self.controls[control::TOURNAMENT_PANEL] = control::SHOWN;
                self.controls[control::TOURNAMENT_PANEL_LABEL] = control::SHOWN;
                self.controls[control::ENTER] = control::SHOWN;
                self.controls[control::SECONDARY] = control::HIDDEN;
                self.controls[control::STATUS] = control::HIDDEN;
                let status = self.tournament_status();
                self.controls[control::ENTER] = status.enter;
                self.controls[control::SECONDARY] = status.secondary;
                if let Some(state) = self.controls.get_mut(control::STATUS) {
                    *state = status.text_state;
                }
                self.refresh_tournament();
            }
            BoardTab::Season => {
                self.controls[control::ENTER] = control::HIDDEN;
                self.controls[control::SHARED_LABEL_0] = control::HIDDEN;
                self.controls[control::SHARED_LABEL_1] = control::HIDDEN;
                self.refresh_season();
            }
        }
    }

    /// The tables visible on the open tab.
    pub fn visible_tables(&self) -> &'static [TableSlot] {
        match self.tab {
            BoardTab::Matches => &[
                TableSlot::Match,
                TableSlot::Rank,
                TableSlot::Tier,
                TableSlot::Roster,
                TableSlot::Lineup,
            ],
            BoardTab::Events => &[TableSlot::Events, TableSlot::Roster, TableSlot::Lineup],
            BoardTab::Tournament => &[TableSlot::Winners, TableSlot::Roster, TableSlot::Lineup],
            BoardTab::Season => &[TableSlot::Season],
        }
    }

    /// A table's layout, with the lineup's tournament height applied.
    pub fn table_layout(&self, slot: TableSlot) -> TableLayout {
        let layout = self.layout();
        let mut table = match slot {
            TableSlot::Match => layout.match_table,
            TableSlot::Rank => layout.rank_table,
            TableSlot::Tier => layout.tier_table,
            TableSlot::Events => layout.events_table,
            TableSlot::Winners => layout.winners_table,
            TableSlot::Roster => layout.roster_table,
            TableSlot::Lineup => layout.lineup_table,
            TableSlot::Season => layout.season_table,
        };
        if slot == TableSlot::Lineup {
            table.rect.h = self.lineup_height();
        }
        table
    }

    /// The board's hit test: a tab, the primary button, or a visible table
    /// cell. The row height is the layout's guess, so cells below it follow
    /// that guess.
    pub fn hit_test(&self, point: (i32, i32)) -> BoardHit {
        let layout = self.layout();
        for (index, rect) in layout.tabs.iter().enumerate() {
            if rect.contains(point.0, point.1) {
                return BoardHit::Tab(index);
            }
        }
        if !control::hidden(self.enter_state()) && layout.enter_button.contains(point.0, point.1) {
            return BoardHit::Enter;
        }
        for &slot in self.visible_tables() {
            let table = self.table_layout(slot);
            if !table.rect.contains(point.0, point.1) {
                continue;
            }
            let row = ((point.1 - table.rect.y) / ROW_HEIGHT) as usize;
            if row >= table.rows {
                continue;
            }
            let mut x = table.rect.x;
            for (column, spec) in table.columns.iter().enumerate() {
                if point.0 >= x && point.0 < x + spec.width {
                    return BoardHit::Table { slot, row, column };
                }
                x += spec.width;
            }
        }
        BoardHit::Outside
    }

    /// Turns a click at a point into the matching board input.
    pub fn click(&mut self, point: (i32, i32)) -> BoardAction {
        let hit = self.hit_test(point);
        match hit {
            BoardHit::Tab(index) => self.handle(BoardInput::SelectTab(index)),
            BoardHit::Enter => self.handle(BoardInput::Confirm),
            BoardHit::Table { slot, row, .. } => match slot {
                TableSlot::Match => self.handle(BoardInput::SelectMatch(row)),
                TableSlot::Rank => self.handle(BoardInput::SelectRank(row)),
                TableSlot::Tier => self.handle(BoardInput::SelectTier(row)),
                TableSlot::Events => self.handle(BoardInput::ClickEvent(row)),
                TableSlot::Winners => self.handle(BoardInput::ClickWinner(row)),
                TableSlot::Roster => {
                    self.available_selection = row;
                    BoardAction::Changed
                }
                TableSlot::Lineup => {
                    self.team_selection = row;
                    BoardAction::Changed
                }
                TableSlot::Season => {
                    self.season_selection = row;
                    BoardAction::Changed
                }
            },
            BoardHit::Outside => BoardAction::None,
        }
    }

    /// Handles one board input and returns the action the arena should take.
    pub fn handle(&mut self, input: BoardInput) -> BoardAction {
        match input {
            BoardInput::SelectTab(index) => {
                let Some(tab) = BoardTab::from_index(index) else {
                    return BoardAction::None;
                };
                if self.switch_tab(tab).is_some() {
                    BoardAction::Changed
                } else {
                    BoardAction::None
                }
            }
            BoardInput::SelectMatch(index) => {
                if index >= MATCH_TYPES.len() {
                    return BoardAction::None;
                }
                self.match_type = index;
                self.refresh_standard();
                BoardAction::Changed
            }
            BoardInput::SelectRank(index) => {
                if index >= RANKS.len() {
                    return BoardAction::None;
                }
                self.rank = index;
                // The original keeps the filter's floor in a global that the
                // board's methods only read; the dropdown's selection is the
                // natural source. This is a guess.
                self.rank_floor = index;
                self.refresh_standard();
                BoardAction::Changed
            }
            BoardInput::SelectTier(index) => {
                if index >= RANKS.len() {
                    return BoardAction::None;
                }
                self.tier = index;
                self.refresh_standard();
                BoardAction::Changed
            }
            BoardInput::ClickEvent(index) => {
                if index >= self.events.len() {
                    return BoardAction::None;
                }
                self.event_selection = index;
                self.events_top = index + 1;
                self.refresh_events();
                BoardAction::Changed
            }
            BoardInput::ClickWinner(index) => {
                if index >= self.entries.len() {
                    return BoardAction::None;
                }
                self.winner_selection = index;
                self.refresh_tournament();
                BoardAction::Changed
            }
            BoardInput::DoubleClickAvailable(index) => {
                if self.double_click_available(index) {
                    BoardAction::Changed
                } else {
                    BoardAction::None
                }
            }
            BoardInput::DoubleClickTeam(index) => {
                if self.double_click_team(index) {
                    BoardAction::Changed
                } else {
                    BoardAction::None
                }
            }
            BoardInput::DropOnTeam { from, to } => {
                if self.move_to_team(from, to) {
                    BoardAction::Changed
                } else {
                    BoardAction::None
                }
            }
            BoardInput::DropOnAvailable(index) => {
                if self.move_to_available(index) {
                    BoardAction::Changed
                } else {
                    BoardAction::None
                }
            }
            BoardInput::Confirm => self.confirm(),
            BoardInput::Close => BoardAction::Close,
        }
    }

    fn confirm(&mut self) -> BoardAction {
        match self.tab {
            BoardTab::Matches | BoardTab::Events => {
                if !control::disabled(self.enter_state())
                    && !control::hidden(self.enter_state())
                    && self.team.len() >= self.team_size
                    && matches_left(self.matches_played)
                {
                    self.matches_played = self.matches_played.saturating_add(1);
                    BoardAction::EnterMatch
                } else {
                    BoardAction::None
                }
            }
            BoardTab::Tournament => {
                let status = self.tournament_status();
                if control::hidden(status.enter) || control::disabled(status.enter) {
                    return BoardAction::None;
                }
                if status.button_label == "Begin" {
                    BoardAction::BeginTournament
                } else if status.button_label == "Continue" {
                    BoardAction::ContinueTournament
                } else {
                    BoardAction::EnterMatch
                }
            }
            BoardTab::Season => BoardAction::None,
        }
    }

    /// The rows of the standard match list, with the disabled flag the
    /// rank selection gives the tier list.
    pub fn match_rows(&self) -> [&'static str; 6] {
        MATCH_TYPES
    }

    /// The rows of the rank list.
    pub fn rank_rows(&self) -> [&'static str; 6] {
        RANKS
    }

    /// The rows of the tier list, and whether the rank disables each one.
    pub fn tier_rows(&self) -> [(&'static str, bool); 6] {
        std::array::from_fn(|row| (RANKS[row], self.tier_row_disabled(row)))
    }

    /// The daily event table's five columns per row.
    pub fn event_cells(&self, row: usize) -> Option<[CellValue; 5]> {
        let event = self.events.get(row)?;
        Some([
            CellValue::Text(event.name.clone()),
            CellValue::Text(rank_name(event.rank).to_string()),
            CellValue::Text(event_tier_name(event).to_string()),
            CellValue::Coin(event.fee),
            CellValue::Coin(event.prize),
        ])
    }

    /// The tournament winners table: event name, the row's tier and the
    /// winner's company.
    pub fn winner_cells(&self, row: usize) -> Option<[CellValue; 3]> {
        let name = self.tournament_events.get(row)?.clone();
        let winner = self.entries.get(row).map(|entry| entry.winner).unwrap_or(0);
        let text = if winner == 1 {
            self.company_name.clone()
        } else if winner <= 0 {
            NO_RANK.to_string()
        } else {
            self.rivals
                .get((winner - 2) as usize)
                .cloned()
                .unwrap_or_default()
        };
        Some([
            CellValue::Text(name),
            CellValue::Text(RANKS.get(row).copied().unwrap_or(NO_RANK).to_string()),
            CellValue::Text(text),
        ])
    }

    /// One fighter table row: name, rank and health.
    pub fn fighter_cells(&self, slot: usize) -> Option<[CellValue; 3]> {
        let fighter = self.fighters.get(slot)?;
        Some([
            CellValue::Text(fighter.name.clone()),
            CellValue::Text(rank_name(fighter.rank as i32).to_string()),
            CellValue::Health(fighter.health),
        ])
    }

    /// The season table's one column.
    pub fn season_cells(&self, row: usize) -> Option<[CellValue; 1]> {
        Some([CellValue::Text(self.season_winner(row))])
    }

    /// The lineup table's height after the last tournament refresh.
    pub fn lineup_height(&self) -> i32 {
        self.lineup_height_override.unwrap_or_else(|| {
            let layout = self.layout();
            layout.lineup_table.rect.h
        })
    }

    /// A cell's value with the coin zero handled as the original's text
    /// path. The original's column callback receives the cell's low dword;
    /// a zero is drawn as the text "0" four pixels right, anything else as
    /// coin icons at the cell's left edge.
    pub fn coin_draw(packed: u32, x: i32) -> CoinDraw {
        if packed == 0 {
            CoinDraw::Zero { x: x + 4 }
        } else {
            CoinDraw::Coins {
                x,
                copper: coin_parts(packed).0,
                silver: coin_parts(packed).1,
                gold: coin_parts(packed).2,
            }
        }
    }

    /// The board's paint list in drawing order: the form, the four tabs,
    /// then the visible tables in the order the constructor created them.
    pub fn paint(&self) -> Vec<BoardPaint> {
        let layout = self.layout();
        let mut paint = Vec::new();
        paint.push(BoardPaint::Frame {
            widget: Widget::Form,
            rect: layout.form,
        });
        for (index, rect) in layout.tabs.iter().enumerate() {
            paint.push(BoardPaint::Tab {
                index,
                rect: *rect,
                title: BoardTab::from_index(index)
                    .map(BoardTab::label)
                    .unwrap_or(TITLE),
                selected: index == self.tab.index(),
            });
        }
        match self.tab {
            BoardTab::Matches => {
                self.paint_table(&mut paint, &layout.match_table, &|board, row| {
                    board
                        .match_rows()
                        .get(row)
                        .map(|name| (vec![CellValue::Text((*name).to_string())], false))
                });
                self.paint_table(&mut paint, &layout.rank_table, &|board, row| {
                    board
                        .rank_rows()
                        .get(row)
                        .map(|name| (vec![CellValue::Text((*name).to_string())], false))
                });
                self.paint_table(&mut paint, &layout.tier_table, &|board, row| {
                    board.tier_rows().get(row).map(|(name, disabled)| {
                        (vec![CellValue::Text((*name).to_string())], *disabled)
                    })
                });
                self.paint_message(&mut paint, &layout);
                self.paint_fighters(&mut paint, &layout);
            }
            BoardTab::Events => {
                self.paint_table(&mut paint, &layout.events_table, &|board, row| {
                    board.event_cells(row).map(|cells| {
                        let disabled = board
                            .events
                            .get(row)
                            .is_some_and(|event| event.flags & 1 != 0);
                        (Vec::from(cells), disabled)
                    })
                });
                self.paint_message(&mut paint, &layout);
                self.paint_fighters(&mut paint, &layout);
            }
            BoardTab::Tournament => {
                self.paint_table(&mut paint, &layout.winners_table, &|board, row| {
                    board
                        .winner_cells(row)
                        .map(|cells| (Vec::from(cells), false))
                });
                paint.push(BoardPaint::Frame {
                    widget: Widget::Picture,
                    rect: layout.picture,
                });
                if self.tournament_mode {
                    paint.push(BoardPaint::Frame {
                        widget: Widget::RewardPanel,
                        rect: layout.reward_panel,
                    });
                    if !self.tournament_note.is_empty() {
                        paint.push(BoardPaint::Label {
                            widget: Widget::RewardPanel,
                            rect: Some(Rect {
                                x: layout.reward_panel.x + 4,
                                y: layout.reward_panel.y + 4,
                                w: layout.reward_panel.w - 8,
                                h: ROW_HEIGHT,
                            }),
                            text: self.tournament_note.clone(),
                        });
                    }
                    for (index, rect) in layout.reward_slots.iter().enumerate() {
                        paint.push(BoardPaint::Frame {
                            widget: Widget::Reward(index),
                            rect: *rect,
                        });
                    }
                }
                let status = self.tournament_status();
                if !control::hidden(status.text_state) {
                    paint.push(BoardPaint::Label {
                        widget: Widget::Status,
                        rect: Some(layout.status_label),
                        text: status.text,
                    });
                }
                self.paint_fighters(&mut paint, &layout);
            }
            BoardTab::Season => {
                self.paint_table(&mut paint, &layout.season_table, &|board, row| {
                    board
                        .season_cells(row)
                        .map(|cells| (Vec::from(cells), false))
                });
                for (index, rect) in layout.reward_slots.iter().enumerate() {
                    paint.push(BoardPaint::Frame {
                        widget: Widget::Reward(index),
                        rect: *rect,
                    });
                }
            }
        }
        let enter = self.enter_state();
        if !control::hidden(enter) {
            paint.push(BoardPaint::Label {
                widget: Widget::EnterButton,
                rect: Some(layout.enter_button),
                text: self.tab_button_label(),
            });
        }
        if self.tab != BoardTab::Season {
            paint.push(BoardPaint::Label {
                widget: Widget::EnteredLabel,
                rect: Some(layout.entered_label),
                text: self.entered_label(),
            });
        }
        paint
    }

    fn tab_button_label(&self) -> String {
        if self.tab == BoardTab::Tournament {
            self.tournament_status().button_label.to_string()
        } else {
            "Enter".to_string()
        }
    }

    fn paint_message(&self, paint: &mut Vec<BoardPaint>, layout: &BoardLayout) {
        if !self.standard_message.is_empty() {
            paint.push(BoardPaint::Label {
                widget: Widget::Message,
                rect: Some(layout.message),
                text: self.standard_message.clone(),
            });
        }
    }

    fn paint_fighters(&self, paint: &mut Vec<BoardPaint>, layout: &BoardLayout) {
        let roster = self.available.clone();
        paint_table_rows(paint, &layout.roster_table, &roster, |slot| {
            self.fighter_cells(slot)
                .map(|cells| (Vec::from(cells), false))
        });
        let team = self.team.clone();
        let mut lineup = layout.lineup_table;
        lineup.rect.h = self.lineup_height();
        paint_table_rows(paint, &lineup, &team, |slot| {
            self.fighter_cells(slot)
                .map(|cells| (Vec::from(cells), false))
        });
    }

    fn paint_table<F>(&self, paint: &mut Vec<BoardPaint>, table: &TableLayout, cells: &F)
    where
        F: Fn(&Board, usize) -> Option<(Vec<CellValue>, bool)>,
    {
        let rows: Vec<usize> = (0..table.rows).collect();
        paint_table_rows(paint, table, &rows, |row| cells(self, row));
    }
}

/// A tournament status line and the states of its two buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentStatus {
    pub text: String,
    /// "Enter", "Begin" or "Continue".
    pub button_label: &'static str,
    /// The primary button's state.
    pub enter: u32,
    /// The secondary button's state.
    pub secondary: u32,
    /// The status text's state.
    pub text_state: u32,
}

/// The result of the show method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowState {
    /// Whether the day is the weekly tournament day.
    pub tournament_day: bool,
    /// Whether any tournament event has an entry.
    pub has_entrants: bool,
    /// The tab left open.
    pub tab: BoardTab,
    /// Whether the tournament tab was forced.
    pub forced_tournament: bool,
}

/// What closing the board gives back for the next opening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseState {
    /// The tab to reopen next time.
    pub tab: BoardTab,
    /// The selected standard match type.
    pub match_type: usize,
    /// The selected rank.
    pub rank: usize,
    /// The selected tier.
    pub tier: usize,
    /// The tournament reward item that was in its slot.
    pub tournament_reward: Option<u32>,
    /// The four season reward items that were in their slots.
    pub season_items: [Option<u32>; 4],
    /// The season picture that was loaded.
    pub picture: String,
}

/// What a tab change did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabView {
    /// The tab that is now open.
    pub tab: BoardTab,
    /// Which tabs were hidden by the change.
    pub hidden: [bool; 4],
    /// The control states after the change.
    pub controls: [u32; CONTROL_COUNT],
}

/// A cell's contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellValue {
    Text(String),
    /// A packed copper amount, drawn as coin icons.
    Coin(u32),
    /// A fighter's health, drawn by the health column's own callback.
    Health(u16),
    Empty,
}

/// The draw callback's decision for a coin cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoinDraw {
    /// The value zero is the text "0" at this x.
    Zero { x: i32 },
    /// Coin icons at x, with the number of each denomination.
    Coins {
        x: i32,
        copper: u32,
        silver: u32,
        gold: u32,
    },
}

/// A view in the paint list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Widget {
    /// The board's form.
    Form,
    /// The tournament picture.
    Picture,
    /// The tournament reward panel.
    RewardPanel,
    /// A reward item slot, zero through three.
    Reward(usize),
    /// The message box.
    Message,
    /// The tournament status line.
    Status,
    /// The Enter, Begin or Continue button.
    EnterButton,
    /// The "Entered n/m" label.
    EnteredLabel,
}

/// One entry in the board's paint list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardPaint {
    /// A form, panel, picture or item slot.
    Frame { widget: Widget, rect: Rect },
    /// One of the four tab buttons.
    Tab {
        index: usize,
        rect: Rect,
        title: &'static str,
        selected: bool,
    },
    /// A table's frame.
    Table { slot: TableSlot, rect: Rect },
    /// A table's column header.
    Column {
        slot: TableSlot,
        index: usize,
        rect: Rect,
        title: &'static str,
    },
    /// A table cell.
    Cell {
        slot: TableSlot,
        row: usize,
        column: usize,
        rect: Rect,
        value: CellValue,
        disabled: bool,
    },
    /// A label; its rectangle is absent when the form template places it
    /// and that placement is not modelled.
    Label {
        widget: Widget,
        rect: Option<Rect>,
        text: String,
    },
}

fn paint_table_rows(
    paint: &mut Vec<BoardPaint>,
    table: &TableLayout,
    rows: &[usize],
    cells: impl Fn(usize) -> Option<(Vec<CellValue>, bool)>,
) {
    paint.push(BoardPaint::Table {
        slot: table.slot,
        rect: table.rect,
    });
    let mut x = table.rect.x;
    for (index, column) in table.columns.iter().enumerate() {
        paint.push(BoardPaint::Column {
            slot: table.slot,
            index,
            rect: Rect {
                x,
                y: table.rect.y,
                w: column.width,
                h: ROW_HEIGHT,
            },
            title: column.title,
        });
        x += column.width;
    }
    for (row, &source) in rows.iter().enumerate() {
        let Some((values, disabled)) = cells(source) else {
            continue;
        };
        let mut x = table.rect.x;
        for (index, column) in table.columns.iter().enumerate() {
            let value = values.get(index).cloned().unwrap_or(CellValue::Empty);
            paint.push(BoardPaint::Cell {
                slot: table.slot,
                row,
                column: index,
                rect: Rect {
                    x,
                    y: table.rect.y + row as i32 * ROW_HEIGHT,
                    w: column.width,
                    h: ROW_HEIGHT,
                },
                value,
                disabled,
            });
            x += column.width;
        }
    }
}

fn count_char(count: usize) -> char {
    char::from_u32((count as u32).wrapping_add(0x30) & 0xff).unwrap_or('\u{fffd}')
}

/// What a point on the board hits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardHit {
    /// A tab button.
    Tab(usize),
    /// A table's cell.
    Table {
        slot: TableSlot,
        row: usize,
        column: usize,
    },
    /// The primary Enter/Begin/Continue button.
    Enter,
    /// Nothing the board handles.
    Outside,
}

/// One board input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardInput {
    SelectTab(usize),
    SelectMatch(usize),
    SelectRank(usize),
    SelectTier(usize),
    ClickEvent(usize),
    ClickWinner(usize),
    DoubleClickAvailable(usize),
    DoubleClickTeam(usize),
    DropOnTeam { from: usize, to: Option<usize> },
    DropOnAvailable(usize),
    Confirm,
    Close,
}

/// What the board asks the arena to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardAction {
    /// Nothing happened.
    None,
    /// The board's tables or controls changed.
    Changed,
    /// The company enters the selected match.
    EnterMatch,
    /// The tournament's first round begins.
    BeginTournament,
    /// The next tournament round begins.
    ContinueTournament,
    /// The form closes.
    Close,
}

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

/// The base form's show helper kept from the first model; the board's own
/// show is [`Board::show`]. It seeds the columns, clears the panels and
/// returns.
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

/// The base form's destroy helper: the object itself is present. The board
/// releases its items and selections through [`Board::close`].
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

    fn fighters() -> Vec<Fighter> {
        vec![
            Fighter::new("Player", 3, 0),
            Fighter::new("Anna", 2, 0),
            Fighter::new("Borin", 1, 1),
            Fighter::new("Cael", 0, 0),
            Fighter::new("Dara", 4, 1),
            Fighter::new("Merchant", 3, 4),
        ]
    }

    fn board() -> Board {
        let mut board = Board::new(true, 3, fighters());
        board.rank_floor = 0;
        board.team_size = 2;
        board.rebuild_lineups();
        board
    }

    #[test]
    fn ranks_halve_the_six_skills() {
        assert_eq!(rank_of(&[0, 0, 0, 0, 0, 0]), 0);
        assert_eq!(rank_of(&[1, 0, 0, 0, 0, 0]), 0);
        assert_eq!(rank_of(&[2, 0, 0, 0, 0, 0]), 1);
        assert_eq!(rank_of(&[5, 5, 5, 5, 5, 5]), 5);
        assert_eq!(rank_of(&[10, 10, 10, 10, 10, 10]), 5);
        assert_eq!(rank_of(&[u32::MAX; 6]), 5);
        assert_eq!(rank_name(-1), "-");
        assert_eq!(rank_name(0), "Inept");
        assert_eq!(rank_name(5), "Master");
        assert_eq!(rank_name(6), "-");
    }

    #[test]
    fn the_layout_is_pinned() {
        let wide = layout(false);
        assert_eq!(
            wide.form,
            Rect {
                x: 0,
                y: 0,
                w: 604,
                h: 656
            }
        );
        assert_eq!(
            wide.match_table.rect,
            Rect {
                x: 0,
                y: 56,
                w: 196,
                h: 180
            }
        );
        assert_eq!(
            wide.rank_table.rect,
            Rect {
                x: 200,
                y: 56,
                w: 192,
                h: 180
            }
        );
        assert_eq!(
            wide.tier_table.rect,
            Rect {
                x: 396,
                y: 56,
                w: 192,
                h: 180
            }
        );
        assert_eq!(
            wide.events_table.rect,
            Rect {
                x: 0,
                y: 56,
                w: 588,
                h: 180
            }
        );
        assert_eq!(
            wide.winners_table.rect,
            Rect {
                x: 0,
                y: 56,
                w: 392,
                h: 180
            }
        );
        assert_eq!(
            wide.roster_table.rect,
            Rect {
                x: 0,
                y: 348,
                w: 292,
                h: 244
            }
        );
        assert_eq!(
            wide.lineup_table.rect,
            Rect {
                x: 296,
                y: 348,
                w: 292,
                h: 244
            }
        );
        assert_eq!(
            wide.season_table.rect,
            Rect {
                x: 32,
                y: 60,
                w: 280,
                h: 132
            }
        );
        assert_eq!(
            wide.message,
            Rect {
                x: 40,
                y: 276,
                w: 520,
                h: 96
            }
        );
        assert_eq!(
            wide.picture,
            Rect {
                x: 396,
                y: 56,
                w: 192,
                h: 180
            }
        );
        assert_eq!(
            wide.reward_panel,
            Rect {
                x: 296,
                y: 472,
                w: 292,
                h: 120
            }
        );
        assert_eq!(wide.events_table.columns[0].width, 140);
        assert_eq!(wide.events_table.columns[4].width, 106);
        assert_eq!(wide.roster_table.columns[2].width, 132);

        let compact = layout(true);
        assert_eq!(compact.form.h, 636);
        assert_eq!(compact.roster_table.rect.h, 224);
        assert_eq!(compact.lineup_table.rect.h, 224);
        assert_eq!(compact.reward_panel.h, 100);
    }

    #[test]
    fn the_calendar_pins_the_schedule() {
        assert!(is_tournament_day(0));
        assert!(is_tournament_day(7));
        assert!(!is_tournament_day(6));
        assert_eq!(tournament_begins_in(6), 1);
        assert_eq!(tournament_begins_in(1), 6);
        assert_eq!(season_index(0), 0);
        assert_eq!(season_index(21), 0);
        assert_eq!(season_index(22), 1);
        assert_eq!(season_index(27), 1);
        assert_eq!(season_index(28), 1);
        assert_eq!(season_index(49), 1);
        assert_eq!(season_index(50), 2);
        assert_eq!(season_index(112), 0);
        assert_eq!(season_weeks_elapsed(0), 0);
        assert_eq!(season_weeks_elapsed(1), 1);
        assert_eq!(season_weeks_elapsed(7), 1);
        assert_eq!(season_weeks_elapsed(8), 2);
        assert_eq!(season_weeks_elapsed(21), 3);
        assert_eq!(season_weeks_elapsed(22), 0);
        assert_eq!(season_weeks_elapsed(27), 0);
        assert_eq!(season_picture_name(0), "seasonrwrda.rfi");
        assert_eq!(season_picture_name(22), "seasonrwrdb.rfi");
        assert_eq!(season_reward_order([1, 2, 3, 4]), [1, 3, 2, 4]);
        assert!(matches_left(2));
        assert!(!matches_left(3));
    }

    #[test]
    fn the_captain_floor_is_two_ranks_lower() {
        assert!(rank_passes(1, 3, 3, false));
        assert!(!rank_passes(1, 3, 2, false));
        assert!(rank_passes(9, 3, 1, false));
        assert!(!rank_passes(9, 3, 0, false));
        assert!(!rank_passes(9, 3, 1, true));
        assert!(rank_passes(9, 3, 3, true));
        assert!(rank_passes(9, 0, 0, false));
        assert!(rank_passes(9, 1, -1, false));
    }

    #[test]
    fn the_tier_clamps_to_the_rank_and_rows_follow() {
        let mut board = board();
        board.rank = 2;
        board.tier = 5;
        board.refresh_standard();
        assert_eq!(board.tier, 2);
        assert_eq!(board.rank_span, 3);
        assert_eq!(board.tier_span, 9);
        assert_eq!(
            board.tier_rows(),
            [
                ("Inept", false),
                ("Aspirant", false),
                ("Novice", true),
                ("Adept", true),
                ("Expert", true),
                ("Master", true),
            ]
        );
    }

    #[test]
    fn the_lineup_autofills_by_rank_and_class() {
        let mut board = board();
        board.rank = 2;
        board.team_size = 2;
        board.rank_floor = 2;
        board.rebuild_lineups();
        assert_eq!(board.team, vec![1, 4]);
        assert_eq!(board.available, vec![0, 2, 3]);
        assert_eq!(board.lineup, vec![1, 2, 3, 4, i32::MAX, 0]);
        assert_eq!(board.entered_label(), "Entered 2/2");

        board.rank_floor = 0;
        board.team_size = 1;
        board.rebuild_lineups();
        assert_eq!(board.team, vec![1]);
        assert_eq!(board.available, vec![0, 2, 3, 4]);
    }

    #[test]
    fn a_full_team_benches_its_last_fighter() {
        let mut board = board();
        board.team_size = 1;
        board.rank_floor = 0;
        board.rebuild_lineups();
        assert_eq!(board.team, vec![1]);
        assert_eq!(board.available, vec![0, 2, 3, 4]);
        assert_eq!(board.entered_label(), "Entered 1/1");
        assert!(board.double_click_available(0));
        assert_eq!(board.team, vec![0]);
        assert_eq!(board.available, vec![2, 3, 4, 1]);
        assert_eq!(board.lineup, vec![0, 1, 2, 3, 4, i32::MAX]);
        assert_eq!(board.entered_label(), "Entered 1/1");
    }

    #[test]
    fn a_short_rank_cannot_be_double_clicked_in() {
        let mut board = board();
        board.team_size = 1;
        board.rank_floor = 4;
        board.rebuild_lineups();
        assert_eq!(board.team, vec![4]);
        assert_eq!(board.available, vec![0, 1, 2, 3]);
        assert!(!board.double_click_available(2));
        assert_eq!(board.team, vec![4]);
        assert!(board.can_drop(4));
        assert!(!board.can_drop(2));
    }

    #[test]
    fn the_captain_puts_a_qualified_fighter_first() {
        let mut board = board();
        board.match_type = 8;
        board.rank_floor = 3;
        board.team_size = 3;
        board.team = vec![2, 4];
        board.available = vec![0, 1, 3, 5];
        assert!(board.move_to_team(0, None));
        assert_eq!(board.team, vec![4, 2, 0]);
    }

    #[test]
    fn moving_back_inserts_by_slot_order() {
        let mut board = board();
        board.team_size = 2;
        board.rank_floor = 0;
        board.rebuild_lineups();
        assert_eq!(board.team, vec![1, 2]);
        assert_eq!(board.available, vec![0, 3, 4]);
        assert!(board.move_to_available(1));
        assert_eq!(board.team, vec![1]);
        assert_eq!(board.available, vec![0, 2, 3, 4]);
        assert!(board.move_to_available(0));
        assert_eq!(board.team, vec![]);
        assert_eq!(board.available, vec![0, 1, 2, 3, 4]);
        assert!(!board.move_to_available(0));
    }

    #[test]
    fn the_entered_label_writes_one_character() {
        let mut board = board();
        board.team_size = 9;
        board.team = vec![0, 1, 2];
        assert_eq!(board.entered_label(), "Entered 3/9");
        board.team = (0..10).collect();
        assert_eq!(board.entered_label(), "Entered :/9");
    }

    #[test]
    fn the_enter_button_follows_the_team_and_the_day() {
        let mut board = board();
        board.team_size = 2;
        board.rebuild_lineups();
        assert_eq!(board.enter_state() & control::DISABLED, 0);
        board.team_size = 3;
        assert_eq!(board.enter_state() & control::DISABLED, control::DISABLED);
        board.matches_played = 3;
        assert_eq!(board.enter_state(), control::DISABLED);
        board.tab = BoardTab::Season;
        assert_eq!(board.enter_state(), control::HIDDEN);
    }

    #[test]
    fn the_tournament_status_reads_the_entries() {
        let mut board = board();
        board.day = 6;
        let status = board.tournament_status();
        assert_eq!(status.text, "Tournament begins tomorrow.");
        assert_eq!(status.enter, control::HIDDEN);
        assert_eq!(status.text_state, control::SHOWN);
        board.day = 1;
        assert_eq!(
            board.tournament_status().text,
            "Tournament begins in 6 days."
        );

        board.day = 7;
        assert_eq!(board.tournament_status().text, "Tournament is in progress.");

        board.entries[2].participants = vec![Participant { slot: 1, value: 7 }];
        board.entries[2].winner = 0;
        let begin = board.tournament_status();
        assert_eq!(begin.text, "");
        assert_eq!(begin.button_label, "Begin");
        assert_eq!(begin.enter, control::SHOWN);
        assert_eq!(begin.text_state, control::HIDDEN);

        board.entries[0].winner = 1;
        assert_eq!(board.tournament_status().button_label, "Continue");

        board.entries[2].winner = 2;
        let ended = board.tournament_status();
        assert_eq!(ended.text, "No more events entered.");
        assert_eq!(ended.enter, control::HIDDEN);
        assert_eq!(ended.text_state, control::SHOWN);

        board.tournament_ended = true;
        assert_eq!(board.tournament_status().text, TOURNAMENT_ENDED);
    }

    #[test]
    fn the_season_winner_lines_are_pinned() {
        let mut board = board();
        board.company_name = "Ours".to_string();
        board.rivals = vec!["Rival Two".to_string(), "Rival Three".to_string()];
        board.day = 15;
        board.week_winners = [0, 1, 2, 0];
        assert_eq!(board.season_winner(0), "No winner");
        assert_eq!(board.season_winner(1), "Ours");
        assert_eq!(board.season_winner(2), "Rival Two");
        assert_eq!(board.season_winner(3), "-");
        board.day = 27;
        assert_eq!(board.season_winner(0), "-");
        board.week_winners[0] = 21;
        assert_eq!(board.season_winner(0), "-");
        board.week_winners[0] = 20;
        board.rivals = (0..19).map(|index| format!("Rival {}", index)).collect();
        assert_eq!(board.season_winner(0), "Rival 18");
    }

    #[test]
    fn the_event_cells_show_coins_and_the_merchant_dash() {
        let mut board = board();
        board.events[0] = BoardEvent {
            name: "Challenger".to_string(),
            kind: 0,
            flags: 1,
            rank: 3,
            second_rank: -1,
            fee: 0x0c03,
            prize: 0x0102_0304,
        };
        board.events[1] = BoardEvent {
            name: "Merchant".to_string(),
            kind: 8,
            flags: 0,
            rank: 2,
            second_rank: 5,
            fee: 12,
            prize: 0,
        };
        assert_eq!(
            board.event_cells(0),
            Some([
                CellValue::Text("Challenger".to_string()),
                CellValue::Text("Adept".to_string()),
                CellValue::Text("Inept".to_string()),
                CellValue::Coin(0x0c03),
                CellValue::Coin(0x0102_0304),
            ])
        );
        assert_eq!(
            board.event_cells(1),
            Some([
                CellValue::Text("Merchant".to_string()),
                CellValue::Text("Novice".to_string()),
                CellValue::Text("-".to_string()),
                CellValue::Coin(12),
                CellValue::Coin(0),
            ])
        );
        assert_eq!(Board::coin_draw(0, 10), CoinDraw::Zero { x: 14 });
        assert_eq!(
            Board::coin_draw(0x0002_0304, 10),
            CoinDraw::Coins {
                x: 10,
                copper: 4,
                silver: 3,
                gold: 2,
            }
        );
    }

    #[test]
    fn the_tournament_winners_read_company_ids() {
        let mut board = board();
        board.company_name = "Ours".to_string();
        board.rivals = vec!["Two".to_string(), "Three".to_string()];
        board.tournament_events = std::array::from_fn(|index| format!("Event {}", index));
        board.entries[0].winner = 1;
        board.entries[1].winner = 2;
        board.entries[2].winner = 3;
        board.entries[3].winner = 0;
        assert_eq!(
            board.winner_cells(0),
            Some([
                CellValue::Text("Event 0".to_string()),
                CellValue::Text("Inept".to_string()),
                CellValue::Text("Ours".to_string()),
            ])
        );
        assert_eq!(
            board.winner_cells(1),
            Some([
                CellValue::Text("Event 1".to_string()),
                CellValue::Text("Aspirant".to_string()),
                CellValue::Text("Two".to_string()),
            ])
        );
        assert_eq!(
            board.winner_cells(2).map(|cells| cells[2].clone()),
            Some(CellValue::Text("Three".to_string()))
        );
        assert_eq!(
            board.winner_cells(3).map(|cells| cells[2].clone()),
            Some(CellValue::Text("-".to_string()))
        );
    }

    #[test]
    fn the_events_selection_moves_past_entered_events() {
        let mut board = board();
        for event in &mut board.events {
            event.flags = 1;
        }
        assert_eq!(board.event_selection, 0);
        board.refresh_events();
        assert_eq!(board.event_selection, 0);
        assert_eq!(board.events_top, 6);
        board.events[0].flags = 1;
        board.events[1].flags = 1;
        board.events[2].flags = 0;
        board.event_selection = 0;
        board.refresh_events();
        assert_eq!(board.event_selection, 2);
        assert_eq!(board.events_top, 3);
    }

    #[test]
    fn the_paint_order_is_the_constructor_order() {
        let board = board();
        let paint = board.paint();
        assert!(matches!(
            paint.first(),
            Some(BoardPaint::Frame {
                widget: Widget::Form,
                ..
            })
        ));
        let tabs = paint
            .iter()
            .filter(|item| matches!(item, BoardPaint::Tab { .. }))
            .count();
        assert_eq!(tabs, 4);
        assert!(matches!(
            paint.get(1),
            Some(BoardPaint::Tab {
                index: 0,
                selected: true,
                ..
            })
        ));
        let tables: Vec<TableSlot> = paint
            .iter()
            .filter_map(|item| match item {
                BoardPaint::Column { slot, .. } => Some(*slot),
                _ => None,
            })
            .collect();
        let firsts: Vec<TableSlot> = tables.into_iter().fold(Vec::new(), |mut acc, slot| {
            if acc.last() != Some(&slot) {
                acc.push(slot);
            }
            acc
        });
        assert_eq!(
            firsts,
            vec![
                TableSlot::Match,
                TableSlot::Rank,
                TableSlot::Tier,
                TableSlot::Roster,
                TableSlot::Lineup,
            ]
        );
        let cells: Vec<(TableSlot, usize, usize)> = paint
            .iter()
            .filter_map(|item| match item {
                BoardPaint::Cell {
                    slot, row, column, ..
                } => Some((*slot, *row, *column)),
                _ => None,
            })
            .collect();
        assert_eq!(cells[0], (TableSlot::Match, 0, 0));
        assert_eq!(cells[6], (TableSlot::Rank, 0, 0));
        assert_eq!(cells[12], (TableSlot::Tier, 0, 0));
        assert_eq!(cells[18], (TableSlot::Roster, 0, 0));

        let mut events = board;
        events.tab = BoardTab::Events;
        let tables: Vec<TableSlot> = events
            .paint()
            .iter()
            .filter_map(|item| match item {
                BoardPaint::Column { slot, .. } => Some(*slot),
                _ => None,
            })
            .collect();
        assert_eq!(tables[0], TableSlot::Events);
        assert_eq!(tables[1], TableSlot::Events);
        assert_eq!(tables[4], TableSlot::Events);
        assert_eq!(tables[5], TableSlot::Roster);
        assert_eq!(tables[8], TableSlot::Lineup);
    }

    #[test]
    fn the_tournament_and_season_paint_orders_are_pinned() {
        let mut board = board();
        board.tab = BoardTab::Tournament;
        board.tournament_mode = true;
        board.tournament_reward = Some(1);
        let paint = board.paint();
        let table_slots: Vec<TableSlot> = paint
            .iter()
            .filter_map(|item| match item {
                BoardPaint::Table { slot, .. } => Some(*slot),
                _ => None,
            })
            .collect();
        assert_eq!(
            table_slots,
            vec![TableSlot::Winners, TableSlot::Roster, TableSlot::Lineup]
        );
        let widgets: Vec<Widget> = paint
            .iter()
            .filter_map(|item| match item {
                BoardPaint::Frame { widget, .. } => Some(*widget),
                _ => None,
            })
            .collect();
        assert_eq!(widgets[0], Widget::Form);
        assert!(widgets.contains(&Widget::Picture));
        assert!(widgets.contains(&Widget::RewardPanel));

        let mut season = board;
        season.tab = BoardTab::Season;
        let paint = season.paint();
        let table_slots: Vec<TableSlot> = paint
            .iter()
            .filter_map(|item| match item {
                BoardPaint::Table { slot, .. } => Some(*slot),
                _ => None,
            })
            .collect();
        assert_eq!(table_slots, vec![TableSlot::Season]);
        assert!(paint.iter().any(|item| matches!(
            item,
            BoardPaint::Frame {
                widget: Widget::Reward(3),
                ..
            }
        )));
        assert!(!paint.iter().any(|item| matches!(
            item,
            BoardPaint::Label {
                widget: Widget::EnteredLabel,
                ..
            }
        )));
    }

    #[test]
    fn the_event_table_cells_run_left_to_right_by_row() {
        let mut board = board();
        board.tab = BoardTab::Events;
        for (index, event) in board.events.iter_mut().enumerate() {
            event.name = format!("E{}", index);
        }
        let cells: Vec<(usize, usize, i32)> = board
            .paint()
            .iter()
            .filter_map(|item| match item {
                BoardPaint::Cell {
                    slot: TableSlot::Events,
                    row,
                    column,
                    rect,
                    ..
                } => Some((*row, *column, rect.x)),
                _ => None,
            })
            .collect();
        assert_eq!(cells.len(), 30);
        assert_eq!(cells[0], (0, 0, 0));
        assert_eq!(cells[1], (0, 1, 140));
        assert_eq!(cells[2], (0, 2, 240));
        assert_eq!(cells[3], (0, 3, 340));
        assert_eq!(cells[4], (0, 4, 454));
        assert_eq!(cells[5], (1, 0, 0));
        assert_eq!(cells[9], (1, 4, 454,));
    }

    #[test]
    fn the_hit_test_follows_the_layout_and_the_tab() {
        let mut board = board();
        assert_eq!(board.hit_test((10, 18)), BoardHit::Tab(0));
        assert_eq!(
            board.hit_test((10, 70)),
            BoardHit::Table {
                slot: TableSlot::Match,
                row: 0,
                column: 0
            }
        );
        assert_eq!(
            board.hit_test((210, 70)),
            BoardHit::Table {
                slot: TableSlot::Rank,
                row: 0,
                column: 0
            }
        );
        // The tier list starts at x 396.
        assert_eq!(
            board.hit_test((400, 100)),
            BoardHit::Table {
                slot: TableSlot::Tier,
                row: 1,
                column: 0
            }
        );
        assert_eq!(board.hit_test((10_000, 10_000)), BoardHit::Outside);
        assert_eq!(board.hit_test((450, 610)), BoardHit::Enter);

        // The event table's five columns at 140, 100, 100, 114 and 106.
        assert_eq!(board.handle(BoardInput::SelectTab(1)), BoardAction::Changed);
        assert_eq!(
            board.hit_test((150, 70)),
            BoardHit::Table {
                slot: TableSlot::Events,
                row: 0,
                column: 1
            }
        );
        assert_eq!(
            board.hit_test((454, 70)),
            BoardHit::Table {
                slot: TableSlot::Events,
                row: 0,
                column: 4
            }
        );

        let mut season = board;
        season.tab = BoardTab::Season;
        assert_eq!(season.hit_test((450, 610)), BoardHit::Outside);
    }

    #[test]
    fn a_click_selects_and_confirms() {
        let mut board = board();
        board.team_size = 1;
        board.rank_floor = 0;
        board.rebuild_lineups();
        assert_eq!(board.click((155, 18)), BoardAction::Changed);
        assert_eq!(board.tab, BoardTab::Events);
        assert_eq!(board.click((10, 18)), BoardAction::Changed);
        assert_eq!(board.tab, BoardTab::Matches);
        assert_eq!(
            board.click((10, 56 + 2 * ROW_HEIGHT + 5)),
            BoardAction::Changed
        );
        assert_eq!(board.match_type, 2);
        // The company table's second row.
        assert_eq!(
            board.click((10, 348 + ROW_HEIGHT + 5)),
            BoardAction::Changed
        );
        assert_eq!(board.available_selection, 1);
        // The lineup table's first row.
        assert_eq!(board.click((300, 348 + 5)), BoardAction::Changed);
        assert_eq!(board.team_selection, 0);
        assert_eq!(board.click((450, 610)), BoardAction::EnterMatch);
        assert_eq!(board.matches_played, 1);
        assert_eq!(board.click((10_000, 10_000)), BoardAction::None);
    }

    #[test]
    fn the_input_rules_are_pinned() {
        let mut board = board();
        board.team_size = 1;
        board.rank_floor = 0;
        board.rebuild_lineups();
        assert_eq!(
            board.handle(BoardInput::SelectRank(2)),
            BoardAction::Changed
        );
        assert_eq!(board.rank, 2);
        assert_eq!(board.rank_floor, 2);
        assert_eq!(board.tier, 0);
        assert_eq!(
            board.handle(BoardInput::DoubleClickAvailable(2)),
            BoardAction::None
        );
        assert_eq!(
            board.handle(BoardInput::DoubleClickAvailable(0)),
            BoardAction::Changed
        );
        assert_eq!(board.team, vec![0]);
        assert_eq!(board.handle(BoardInput::Confirm), BoardAction::EnterMatch);
        assert_eq!(board.matches_played, 1);
        assert_eq!(board.handle(BoardInput::SelectTier(6)), BoardAction::None);
        assert_eq!(board.handle(BoardInput::Close), BoardAction::Close);
    }

    #[test]
    fn the_show_forces_the_tournament_tab() {
        let mut board = board();
        board.day = 14;
        board.entries[0].participants = vec![Participant { slot: 1, value: 4 }];
        let show = board.show();
        assert!(show.tournament_day);
        assert!(show.has_entrants);
        assert_eq!(show.tab, BoardTab::Tournament);
        assert!(show.forced_tournament);
        assert_eq!(board.controls[control::TAB_TOURNAMENT] & 0x60, 0x60);

        let mut quiet = Board::new(true, 7, fighters());
        quiet.rebuild_lineups();
        let show = quiet.show();
        assert_eq!(show.tab, BoardTab::Matches);
        assert!(!show.forced_tournament);

        // Already on the forced tab, another tab click does nothing.
        assert_eq!(board.switch_tab(BoardTab::Season), None);

        // From another tab a click lands on the tournament tab.
        let mut other = Board::new(true, 14, fighters());
        other.entries[0].participants = vec![Participant { slot: 1, value: 4 }];
        other.rebuild_lineups();
        let view = other.switch_tab(BoardTab::Season);
        assert_eq!(view.map(|view| view.tab), Some(BoardTab::Tournament));
    }

    #[test]
    fn the_tournament_tab_locks_assignment() {
        let mut board = board();
        board.day = 7;
        board.entries[0].participants = vec![Participant { slot: 1, value: 0 }];
        board.show();
        assert_eq!(board.tab, BoardTab::Tournament);
        assert!(board.assignment_locked());
        assert!(!board.move_to_team(0, None));
        assert!(!board.move_to_available(0));
        board.day = 6;
        board.tab = BoardTab::Tournament;
        assert!(!board.assignment_locked());
    }

    #[test]
    fn a_marked_fighter_replaces_its_marked_teammate() {
        let mut board = board();
        board.match_type = 8;
        board.rank_floor = 3;
        board.team_size = 3;
        board.fighters[0].marked = true;
        board.fighters[2].marked = true;
        board.team = vec![2, 1, 4];
        board.available = vec![0, 3, 5];
        // Two high-ranked members keep the rank known for all five.
        board.fighters[1].rank = 4;
        assert!(board.move_to_team(0, None));
        assert_eq!(board.team[0], 0);
        assert_eq!(board.available[0], 2);
    }

    #[test]
    fn an_unknown_captain_rank_rejects_a_high_target() {
        let mut board = board();
        board.match_type = 8;
        board.rank_floor = 3;
        board.team_size = 3;
        board.team = vec![2, 1, 4];
        board.available = vec![0, 3, 5];
        assert!(!board.move_to_team(0, Some(2)));
        assert_eq!(board.team, vec![2, 1, 4]);
    }

    #[test]
    fn the_tournament_tab_takes_its_reward_item() {
        let mut board = board();
        board.tournament_prize_item = Some(77);
        assert_eq!(board.tournament_reward, None);
        assert!(board.switch_tab(BoardTab::Tournament).is_some());
        assert_eq!(board.tournament_reward, Some(77));
        assert!(board.switch_tab(BoardTab::Events).is_some());
        assert_eq!(board.tournament_reward, None);
    }

    #[test]
    fn the_tier_and_entered_rows_are_disabled() {
        let mut board = board();
        board.rank = 1;
        board.refresh_standard();
        board.events[0].flags = 1;
        board.events[0].name = "Duel".to_string();
        let tier = board.paint().iter().find_map(|item| match item {
            BoardPaint::Cell {
                slot: TableSlot::Tier,
                column: 0,
                row,
                disabled,
                ..
            } if *row == 1 => Some(*disabled),
            _ => None,
        });
        assert_eq!(tier, Some(true));
        board.tab = BoardTab::Events;
        let entered = board.paint().iter().find_map(|item| match item {
            BoardPaint::Cell {
                slot: TableSlot::Events,
                column: 0,
                row: 0,
                disabled,
                ..
            } => Some(*disabled),
            _ => None,
        });
        assert_eq!(entered, Some(true));
    }

    #[test]
    fn the_close_saves_the_selections() {
        let mut board = board();
        board.match_type = 2;
        board.rank = 3;
        board.tier = 1;
        board.tab = BoardTab::Events;
        board.tournament_reward = Some(44);
        let close = board.close();
        assert_eq!(close.tab, BoardTab::Events);
        assert_eq!(close.match_type, 2);
        assert_eq!(close.rank, 3);
        assert_eq!(close.tier, 1);
        assert_eq!(close.tournament_reward, Some(44));
        assert_eq!(close.picture, "seasonrwrda.rfi");
        assert!(board.team.is_empty());
    }

    #[test]
    fn the_entrant_check_hides_the_message_for_a_known_entry() {
        let mut board = board();
        board.tournament_mode = true;
        board.entrant_event = 2;
        assert_eq!(board.entrant_message(), None);
        board.entries[2].participants = vec![Participant { slot: 1, value: 0 }];
        board.entrant_slot = 1;
        assert_eq!(board.entrant_message(), Some(false));
        board.entrant_slot = 2;
        assert_eq!(board.entrant_message(), Some(true));
    }

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
