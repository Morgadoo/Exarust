//! The arena match board (`TMatchBoard`): the four tabs, the six standard
//! match types, the rank and tier lists, the daily event table with its
//! coin columns, the tournament winners and season tables, the company and
//! lineup assignment, the day's three-match limit, and the form's layout
//! metrics and drawing order.
//!
//! The board edits two shared rosters. The left table lists the company's
//! fighters who are not in the lineup; the right lists the lineup itself.
//! A fighter moves between them by double click or drag and drop, and the
//! lineup is capped by the selected match's side size. The company table
//! is kept in the order of the fighter's slot in the company pool.
//!
//! Every tab works on one selected match record. The standard tab copies a
//! standard match and writes the chosen rank and tier into it, scaling its
//! entry fee and prize by the rank; the events tab copies a daily event and
//! the tournament tab a tournament event. The record's lowest rank is the
//! lineup's rank floor, its side size caps the lineup, and its flags lift
//! the one-leader rule or name a weapon for the tournament event.
//!
//! Evidence: the retail match board's constructor, tab switch, list
//! refreshes, fighter assignment and coin cell callback, read against their
//! assembly (`docs/UI.md`, Arena match board).

use crate::creation_forms::{FormRecord, record};
use crate::record_form::FormSpec;
use crate::table_view;

/// The rank labels in the original's table order. A fighter's rank is one
/// of the first six; a record's rank index past Master reads the dash and
/// the staff labels that follow them in the same table.
pub const RANK_LABELS: [&str; 11] = [
    "Inept",
    "Aspirant",
    "Novice",
    "Adept",
    "Expert",
    "Master",
    "-",
    "Merchant",
    "Trainer",
    "Physician",
    "+",
];
/// The six competitor ranks, from the board's rank list.
pub const RANKS: [&str; 6] = ["Inept", "Aspirant", "Novice", "Adept", "Expert", "Master"];
/// The six always-listed standard match types.
pub const MATCH_TYPES: [&str; 6] = ["Duel", "Doubles", "Skirmish", "Fray", "Reserve", "Pugilism"];
/// The four tab captions, from the form template.
pub const TABS: [&str; 4] = ["Standard", "Specials", "Tournament", "Season"];
/// The board's title.
pub const TITLE: &str = "Matches";
/// The template's label above the company table.
pub const AVAILABLE_LABEL: &str = "Available";
/// The template's close button caption.
pub const CLOSE_LABEL: &str = "Close";
/// The template's day-end button caption.
pub const END_DAY_LABEL: &str = "End Day";
/// The template's tournament forfeit button caption.
pub const FORFEIT_LABEL: &str = "Forfeit";
/// The template's entry-fee caption on the standard tab.
pub const ENTRY_CAPTION: &str = "Entry:";
/// The template's prize caption on the standard tab.
pub const PRIZE_CAPTION: &str = "Prize:";
/// The template's prize caption on the tournament tab.
pub const TOURNAMENT_PRIZE_CAPTION: &str = "Prize";
/// The template's caption above the season's reward items.
pub const SEASON_CAPTION: &str = "Prize for this season";
/// Matches a company may enter in one day.
pub const MATCHES_PER_DAY: u32 = 3;
/// Pugilism's match type id. The standard tab stores no tier for it.
pub const UNRANKED_MATCH: u32 = 6;
/// Challenger's match type id. Its tier column shows a dash.
pub const CHALLENGER_EVENT: u32 = 8;
/// The Captain event's id. Its teammates may be two ranks below the floor.
pub const CAPTAIN_EVENT: u32 = 9;
/// A match flag that lifts the one-leader and marked-fighter rules. On a
/// daily event the same bit greys the row and moves the selection past it;
/// what sets it there (likely an entered event) is outside the board.
pub const FLAG_FREE_LINEUP: u32 = 1;
/// A match flag naming a weapon for the event. The tournament tab then
/// shows the reward panel with the weapon's name, shortens the lineup to
/// four rows and checks the first entrant's weapon.
pub const FLAG_WEAPON: u32 = 4;
/// A team member's leader bit. The player counts as a leader too.
pub const LEADER_BIT: u8 = 1;
/// A fighter whose class byte is above this is not a combatant.
pub const COMBATANT_CLASS: u8 = 2;
/// The text when the day's three matches are spent.
pub const NO_MORE_MATCHES: &str = "[algn=2]There are no more matches running this day.\r\nEnd the day to participate in more matches.";
/// The rank text of an absent or unknown rank, and of a zero coin cell.
pub const NO_RANK: &str = "-";
/// The tournament status when nobody has entered.
pub const TOURNAMENT_IN_PROGRESS: &str = "Tournament is in progress.";
/// The tournament status when the last event has a winner.
pub const TOURNAMENT_ENDED: &str = "The tournament has ended.";
/// The tournament status when every entered event is settled but the last
/// event has no winner.
pub const NO_MORE_EVENTS: &str = "No more events entered.";
/// The lineup table's height on a weapon event's tournament tab: the
/// interface scale of four times thirty.
pub const WEAPON_LINEUP_HEIGHT: i32 = 120;

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

/// A table's fixed layout: a table view (`docs/UI.md`, Tables) at the
/// board's GUI unit. `rect` is the view's place in the form's client,
/// `columns` carry the constructor's widths at unit 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableLayout {
    pub slot: TableSlot,
    pub rect: Rect,
    pub columns: &'static [Column],
    pub rows: usize,
    /// The GUI unit the view is laid out at.
    pub unit: i32,
}

impl TableLayout {
    /// The view's client: inside the base form's border, which the table
    /// view's constructor keeps.
    pub fn client(&self) -> Rect {
        Rect {
            x: self.rect.x + TABLE_BORDER,
            y: self.rect.y + TABLE_BORDER,
            w: self.rect.w - 2 * TABLE_BORDER,
            h: self.rect.h - 2 * TABLE_BORDER,
        }
    }

    /// A column's width at the view's unit: the constructor's width at
    /// unit 4, kept at its share of the unit when the unit is not 4.
    pub fn column_width(&self, column: usize) -> i32 {
        let width = self.columns.get(column).map_or(0, |c| c.width);
        if self.unit == 4 {
            width
        } else {
            table_view::rescale_width(width, 4, self.unit)
        }
    }

    /// A column's left edge in the client: the widths before it.
    pub fn column_left(&self, column: usize) -> i32 {
        (0..column).map(|c| self.column_width(c)).sum()
    }

    /// A column's right edge in the client: the last column runs to the
    /// client's right edge.
    fn column_right(&self, column: usize) -> i32 {
        if column + 1 >= self.columns.len() {
            self.client().w
        } else {
            self.column_left(column) + self.column_width(column)
        }
    }

    /// A column's header cell, in the form's client: the client's top six
    /// units.
    pub fn header(&self, column: usize) -> Rect {
        let client = self.client();
        let left = self.column_left(column);
        Rect {
            x: client.x + left,
            y: client.y,
            w: self.column_right(column) - left,
            h: table_view::ROW_UNITS * self.unit,
        }
    }

    /// A cell of the 0-based `row`, in the form's client: six units tall,
    /// under the six-unit header.
    pub fn cell(&self, row: usize, column: usize) -> Rect {
        let pitch = table_view::ROW_UNITS * self.unit;
        Rect {
            y: self.client().y + pitch * (row as i32 + 1),
            ..self.header(column)
        }
    }

    /// A column title's pen in the form's client
    /// ([`table_view::title_pen`]).
    pub fn title_pen(&self, column: usize) -> (i32, i32) {
        let client = self.client();
        let (x, y) = table_view::title_pen(self.unit, self.column_left(column));
        (client.x + x, client.y + y)
    }

    /// A cell's pen in the form's client ([`table_view::cell_pen`]); an
    /// owner-drawn column's callback gets the same place.
    pub fn cell_pen(&self, row: usize, column: usize) -> (i32, i32) {
        let client = self.client();
        let (x, y) = table_view::cell_pen(self.unit, self.column_left(column), row as i32);
        (client.x + x, client.y + y)
    }

    /// The 0-based row under a form-client `y`, as the table view finds
    /// it: nothing in the header, the rows six units tall below it.
    pub fn row_at(&self, y: i32) -> Option<usize> {
        let u = self.unit.max(1);
        let y = y - self.client().y + u;
        if y < table_view::ROW_UNITS * u {
            return None;
        }
        let row = (y - u) / (table_view::ROW_UNITS * u);
        (row >= 1).then(|| row as usize - 1)
    }

    /// The column under a form-client `x`: the first whose right edge
    /// reaches it (the last past them all).
    pub fn column_at(&self, x: i32) -> usize {
        let x = x - self.client().x;
        let mut right = 0;
        for column in 0..self.columns.len() {
            right += self.column_width(column);
            if x <= right {
                return column;
            }
        }
        self.columns.len().saturating_sub(1)
    }
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
    pub close_button: Rect,
    pub end_day_button: Rect,
    pub forfeit_button: Rect,
    pub entered_label: Rect,
    pub available_label: Rect,
    pub status_label: Rect,
    pub entry_caption: Rect,
    pub prize_caption: Rect,
    pub tournament_prize_caption: Rect,
    pub season_caption: Rect,
    pub season_picture: Rect,
    /// The tournament's prize item.
    pub prize_slot: Rect,
    pub reward_slots: [Rect; 4],
}

/// A table's border: the base form's, which the table view's constructor
/// leaves alone (the board's tables are table views).
pub const TABLE_BORDER: i32 = crate::settings::BORDER;

/// The lineup's height in GUI units after a tournament refresh: 61 (56 in
/// the compact form), or 30 for a weapon event.
pub const LINEUP_UNITS: i32 = 0x3d;
pub const COMPACT_LINEUP_UNITS: i32 = 0x38;
pub const WEAPON_LINEUP_UNITS: i32 = 0x1e;

/// The form template's 23 records, as the board's constructor hands them
/// to the form's table builder (it passes the last index, 22), in control
/// order ([`control`]): the four tabs (a radio group), the standard tab's
/// "Entry:" and "Prize:" captions each with its coin control, "Available"
/// and "Entered n/m" above the fighter tables, the tournament's prize item
/// and its "Prize" caption, the bottom row (End Day, Close, Enter and
/// Forfeit, bottom-anchored), the status text on Enter's place, the season
/// picture, its four item slots and the "Prize for this season" caption.
/// The words are copied as stored ([`FormRecord`]); a record whose callback
/// is set reports its changes. The records are laid out by the toolkit's
/// own pass through [`crate::record_form`], as the arena's company windows
/// are, and the class table gives the sizes the template leaves at zero.
///
/// Evidence: the board's constructor and the template array it passes
/// (`docs/UI.md`, Arena match board).
pub const TEMPLATE_RECORDS: [FormRecord; CONTROL_COUNT] = [
    tab(0x14, 0x5, "Standard"),
    tab(0xa2, 0x8000, "Specials"),
    tab(0x12c, 0x8000, "Tournament"),
    tab(0x1b8, 0x8000, "Season"),
    FormRecord {
        text: Some(ENTRY_CAPTION),
        style: 0x1300,
        state: control::HIDDEN,
        ..record(0x5, 0x0, 0xc8, 0x3e, 0x0, 0x0)
    },
    FormRecord {
        state: control::HIDDEN,
        ..record(0xac03, 0x0, 0x8008, 0x8000, 0x0, 0x0)
    },
    FormRecord {
        text: Some(PRIZE_CAPTION),
        style: 0x1300,
        state: control::HIDDEN,
        ..record(0x5, 0x0, 0x168, 0x3e, 0x0, 0x0)
    },
    FormRecord {
        state: control::HIDDEN,
        ..record(0xac03, 0x0, 0x8008, 0x8000, 0x0, 0x0)
    },
    FormRecord {
        text: Some(AVAILABLE_LABEL),
        ..record(0x5, 0x0, 0x74, 0x52, 0x80, 0x1c)
    },
    FormRecord {
        text: Some("Entered 1/1"),
        ..record(0x5, 0x0, 0x198, 0x8000, 0x80, 0x1c)
    },
    FormRecord {
        state: control::HIDDEN,
        reports: true,
        ..record(0xac05, 0x0, 0x19c, 0x14, 0x0, 0x0)
    },
    FormRecord {
        text: Some(TOURNAMENT_PRIZE_CAPTION),
        style: 0x3300,
        state: control::HIDDEN,
        ..record(0x5, 0x0, 0x19c, 0x10, 0xa0, 0x0)
    },
    bottom_button(0x14, 0x2, END_DAY_LABEL, control::HIDDEN),
    bottom_button(0xa0, 0x2, CLOSE_LABEL, control::SHOWN),
    bottom_button(0x12c, 0x8000, "Enter", control::HIDDEN),
    bottom_button(0x1b8, 0x8000, FORFEIT_LABEL, control::HIDDEN),
    FormRecord {
        style: 0x8_2300,
        state: control::HIDDEN,
        ..record(0x5, 0x0, 0x12c, 0x8000, 0xa0, 0x0)
    },
    record(0x7, 0x0, 0x174, 0x8, 0xa0, 0x258),
    season_item(0xc, 0x3c),
    season_item(0xac, 0x3c),
    season_item(0xc, 0x64),
    season_item(0xac, 0x64),
    FormRecord {
        text: Some(SEASON_CAPTION),
        style: 0x3300,
        state: control::HIDDEN,
        ..record(0x5, 0x0, 0x20, 0x36, 0x118, 0x0)
    },
];

const fn tab(x: i32, y: i32, text: &'static str) -> FormRecord {
    FormRecord {
        group: 1,
        text: Some(text),
        reports: true,
        ..record(0x1, 0x0, x, y, 0x80, 0x1c)
    }
}

/// A bottom-row button: style bit 0x80000 anchors it to the client's
/// bottom edge.
const fn bottom_button(x: i32, y: i32, text: &'static str, state: u32) -> FormRecord {
    FormRecord {
        text: Some(text),
        style: 0x2048_3300,
        state,
        reports: true,
        ..record(0x1, 0x0, x, y, 0x80, 0x1c)
    }
}

const fn season_item(x: i32, y: i32) -> FormRecord {
    FormRecord {
        state: control::HIDDEN,
        reports: true,
        ..record(0xac05, 0x0, x, y, 0x0, 0x0)
    }
}

/// The board's form: 604 wide and 656 tall (636 in the compact form, which
/// the constructor picks from a display flag), the placement word the
/// arena's windows OR in, and the base form's border and attribute word,
/// which the constructor leaves alone.
pub const fn form_spec(compact: bool) -> FormSpec {
    FormSpec {
        size: (604, if compact { 636 } else { 656 }),
        border: crate::arena_forms::BASE_BORDER,
        placement: crate::arena_forms::PLACEMENT,
        attributes: crate::arena_forms::BASE_ATTRIBUTES,
        title: TITLE,
        records: &TEMPLATE_RECORDS,
        children: &[],
    }
}

/// The template's records laid out at unit 4 in the form's client: the
/// toolkit's pass snaps x to a multiple of four (the quarter rounded half
/// to even), takes y in four-pixel rows lifted by half the height less
/// eight up to 32 pixels and by four above, and stands a bottom-anchored
/// record `y + 8` rows above the client's bottom edge.
pub fn template_rects(compact: bool) -> [Rect; CONTROL_COUNT] {
    template_rects_at(compact, 4)
}

/// [`template_rects`] at a GUI unit: the records laid out at unit 4 and
/// rescaled with the form ([`crate::record_form::geometry`]).
pub fn template_rects_at(compact: bool, unit: i32) -> [Rect; CONTROL_COUNT] {
    let spec = form_spec(compact);
    let geometry = crate::record_form::geometry(&spec, spec.size, unit);
    let mut rects = [Rect::default(); CONTROL_COUNT];
    for (rect, c) in rects.iter_mut().zip(&geometry.controls) {
        *rect = Rect {
            x: c.x,
            y: c.y,
            w: c.w,
            h: c.h,
        };
    }
    rects
}

/// Builds the board's layout, in the form's client. `compact` shortens the
/// form, the team table and the reward panel.
///
/// The tables and the two child panels come from the constructor. The tab
/// strip, the four bottom buttons, the labels and captions, the prize
/// item, the season picture and its item slots are the form template's
/// records as the toolkit lays them out ([`template_rects`]).
pub fn layout(compact: bool) -> BoardLayout {
    layout_at(compact, 4)
}

/// [`layout`] at a GUI unit: the form and its template records rescaled
/// from unit 4 ([`template_rects_at`]), the child views as the rescale
/// leaves a plain child pane ([`crate::record_form::pane_at`]) and the
/// tables' columns at their share of the unit ([`TableLayout`]).
pub fn layout_at(compact: bool, unit: i32) -> BoardLayout {
    if unit == 4 {
        return design_layout(compact, template_rects(compact));
    }
    let mut layout = design_layout(compact, template_rects_at(compact, unit));
    let spec = form_spec(compact);
    let form = crate::record_form::geometry(&spec, spec.size, unit).form;
    layout.form.w = form.w;
    layout.form.h = form.h;
    let pane = |r: Rect| {
        let p = crate::record_form::pane_at(
            crate::settings::Rect {
                x: r.x,
                y: r.y,
                w: r.w,
                h: r.h,
            },
            unit,
        );
        Rect {
            x: p.x,
            y: p.y,
            w: p.w,
            h: p.h,
        }
    };
    for table in [
        &mut layout.match_table,
        &mut layout.rank_table,
        &mut layout.tier_table,
        &mut layout.events_table,
        &mut layout.winners_table,
        &mut layout.roster_table,
        &mut layout.lineup_table,
        &mut layout.season_table,
    ] {
        table.rect = pane(table.rect);
        table.unit = unit;
    }
    layout.message = pane(layout.message);
    layout.picture = pane(layout.picture);
    layout.reward_panel = pane(layout.reward_panel);
    layout
}

/// The layout at unit 4, the constructor's own numbers, with the
/// template's records at `t`.
fn design_layout(compact: bool, t: [Rect; CONTROL_COUNT]) -> BoardLayout {
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
            x: t[control::TAB_MATCHES].x,
            y: t[control::TAB_MATCHES].y,
            w: t[control::TAB_SEASON].right() - t[control::TAB_MATCHES].x,
            h: t[control::TAB_MATCHES].h,
        },
        tabs: [
            t[control::TAB_MATCHES],
            t[control::TAB_EVENTS],
            t[control::TAB_TOURNAMENT],
            t[control::TAB_SEASON],
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
            unit: 4,
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
            unit: 4,
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
            unit: 4,
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
            unit: 4,
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
            unit: 4,
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
            unit: 4,
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
            unit: 4,
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
            unit: 4,
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
        end_day_button: t[control::DAY_END],
        close_button: t[control::CLOSE],
        enter_button: t[control::ENTER],
        forfeit_button: t[control::SECONDARY],
        available_label: t[control::SHARED],
        entered_label: t[control::ENTERED],
        status_label: t[control::STATUS],
        entry_caption: t[control::STANDARD_ENTRY_CAPTION],
        prize_caption: t[control::STANDARD_PRIZE_CAPTION],
        tournament_prize_caption: t[control::PRIZE_CAPTION],
        season_caption: t[control::SEASON_TAIL],
        season_picture: t[control::SEASON_PICTURE],
        prize_slot: t[control::PRIZE_ITEM],
        reward_slots: [
            t[control::SEASON_ITEM_0],
            t[control::SEASON_ITEM_1],
            t[control::SEASON_ITEM_2],
            t[control::SEASON_ITEM_3],
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
/// board's own methods touch are named; names past what the board writes
/// into them are guesses.
pub mod control {
    use super::{CONTROL_DISABLED, CONTROL_HIDDEN, CONTROL_SELECTED};

    /// The four tab buttons.
    pub const TAB_MATCHES: usize = 0;
    pub const TAB_EVENTS: usize = 1;
    pub const TAB_TOURNAMENT: usize = 2;
    pub const TAB_SEASON: usize = 3;
    /// The standard tab's four controls; the second holds the entry fee and
    /// the fourth the prize.
    pub const STANDARD_FIRST: usize = 4;
    /// The standard tab's "Entry:" caption.
    pub const STANDARD_ENTRY_CAPTION: usize = 4;
    pub const STANDARD_FEE: usize = 5;
    /// The standard tab's "Prize:" caption.
    pub const STANDARD_PRIZE_CAPTION: usize = 6;
    pub const STANDARD_PRIZE: usize = 7;
    pub const STANDARD_LAST: usize = 7;
    /// A control shared by the non-season tabs, hidden on the season tab.
    pub const SHARED: usize = 8;
    /// The "Entered n/m" label, hidden on the season tab.
    pub const ENTERED: usize = 9;
    /// The tournament tab's prize item and its companion control.
    pub const PRIZE_ITEM: usize = 10;
    pub const PRIZE_CAPTION: usize = 11;
    /// A button enabled once a match is played (on a tournament day the
    /// arena's own checks decide). Its role, ending the day, is a guess
    /// from that rule.
    pub const DAY_END: usize = 12;
    /// The Close button. The board never changes its state.
    pub const CLOSE: usize = 13;
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
    /// The season tab's last control: the "Prize for this season" caption.
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

/// The states the form template gives its controls before any tab is
/// applied: the four tabs, the two labels above the fighter tables, the
/// close button and the season picture are shown; the standard tab's four
/// controls, the prize item and its caption, the day-end button, the
/// primary and secondary buttons, the status text, the four season items
/// and the season tail start hidden. The tab show then sets the visible
/// tab's own controls.
pub const TEMPLATE_CONTROLS: [u32; CONTROL_COUNT] = {
    let mut states = [0; CONTROL_COUNT];
    let mut i = 0;
    while i < CONTROL_COUNT {
        states[i] = TEMPLATE_RECORDS[i].state;
        i += 1;
    }
    states
};

/// A match record as the board reads it: a standard match, a daily event, a
/// tournament event, or the board's selected match. The fields follow the
/// arena's match record (`exa_game::arena::MatchInstance`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchRecord {
    pub name: String,
    /// The match type id: 1 through 6 the standard matches, 8 Challenger,
    /// 9 Captain.
    pub kind: u32,
    /// [`FLAG_FREE_LINEUP`] and [`FLAG_WEAPON`]; other bits are the arena's.
    pub flags: u32,
    /// The lowest rank that may fight: the lineup's rank floor.
    pub min_rank: i32,
    /// The highest rank. The standard tab stores its tier here and the
    /// weapon check reads it as the tournament event.
    pub max_rank: i32,
    /// The rank column (the arena's day slot).
    pub rank: i32,
    /// The tier column (the arena's rolled rank); negative is unranked.
    pub tier: i32,
    /// The packed coin entry fee.
    pub fee: u32,
    /// The packed coin prize.
    pub prize: u32,
    /// Fighters per side.
    pub team_size: i32,
    /// The event's one-based weapon row within its tier, or zero.
    pub weapon_row: i32,
}

impl Default for MatchRecord {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: 0,
            flags: 0,
            min_rank: 0,
            max_rank: 0,
            rank: 0,
            tier: 0,
            fee: 0,
            prize: 0,
            team_size: 1,
            weapon_row: 0,
        }
    }
}

/// One of the six standard matches the board lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandardMatch {
    pub name: &'static str,
    pub kind: u32,
    pub flags: u32,
    pub max_rank: i32,
    pub tier: i32,
    /// The packed coin fee at Inept; the standard tab multiplies it by the
    /// rank plus one.
    pub fee: u32,
    /// The packed coin prize at Inept, scaled the same way.
    pub prize: u32,
    pub team_size: i32,
}

impl StandardMatch {
    /// The standard match as a record.
    pub fn record(&self) -> MatchRecord {
        MatchRecord {
            name: self.name.to_string(),
            kind: self.kind,
            flags: self.flags,
            min_rank: 0,
            max_rank: self.max_rank,
            rank: 0,
            tier: self.tier,
            fee: self.fee,
            prize: self.prize,
            team_size: self.team_size,
            weapon_row: 0,
        }
    }
}

/// The six standard matches: Duel, Doubles, Skirmish, Fray, Reserve and
/// Pugilism. Reserve lifts the one-leader rule; Pugilism is unranked.
pub const STANDARD_MATCHES: [StandardMatch; 6] = [
    StandardMatch {
        name: "Duel",
        kind: 1,
        flags: 0,
        max_rank: 0,
        tier: 0,
        fee: 3,
        prize: 0x100,
        team_size: 1,
    },
    StandardMatch {
        name: "Doubles",
        kind: 2,
        flags: 0,
        max_rank: 0,
        tier: 0,
        fee: 6,
        prize: 0x400,
        team_size: 2,
    },
    StandardMatch {
        name: "Skirmish",
        kind: 3,
        flags: 0,
        max_rank: 0,
        tier: 0,
        fee: 9,
        prize: 0x600,
        team_size: 3,
    },
    StandardMatch {
        name: "Fray",
        kind: 4,
        flags: 0,
        max_rank: 0,
        tier: 0,
        fee: 3,
        prize: 0x200,
        team_size: 1,
    },
    StandardMatch {
        name: "Reserve",
        kind: 5,
        flags: FLAG_FREE_LINEUP,
        max_rank: 0,
        tier: 0,
        fee: 3,
        prize: 0x400,
        team_size: 3,
    },
    StandardMatch {
        name: "Pugilism",
        kind: UNRANKED_MATCH,
        flags: 0,
        max_rank: -1,
        tier: -1,
        fee: 1,
        prize: 6,
        team_size: 1,
    },
];

/// One weapon row of a tournament tier: its name and up to three item kinds
/// that satisfy it, zero-padded.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WeaponRow {
    pub name: String,
    pub variants: [u32; 3],
}

impl WeaponRow {
    /// Whether an item kind satisfies the row. The variants are read in
    /// order and the first zero ends the list.
    pub fn accepts(&self, kind: u32) -> bool {
        self.variants
            .iter()
            .take_while(|&&variant| variant != 0)
            .any(|&variant| variant == kind)
    }
}

/// One fighter entered in a tournament event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Participant {
    /// The fighter's slot in the company pool. The original stores the low
    /// byte of the entity's flag word and reads it back as the pool slot.
    pub slot: usize,
    /// The fighter's record value for that event.
    pub value: i32,
    /// The item kind the arena derives from that value; the weapon check
    /// compares it with the event's weapon row.
    pub weapon: u32,
}

/// A tournament event's entry and winner.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventEntry {
    pub participants: Vec<Participant>,
    /// The winner: 0 or less for none, 1 for the player's company, and from
    /// 2 an index into the tournament's company table.
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
    /// bit 0 marks a leader. The bit's meaning is inferred from the
    /// board's one-leader rule.
    pub class: u8,
    /// The entity's marked flag. A marked fighter takes the place of an
    /// existing marked teammate.
    pub marked: bool,
    /// The fighter's record value for each tournament event.
    pub event_values: [i32; 6],
    /// The item kind behind each event value, for the weapon check.
    pub event_weapons: [u32; 6],
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
            event_weapons: [0; 6],
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
/// at Master. A level is the skill's experience word divided by 5000.
pub fn rank_of(skills: &[u32]) -> u8 {
    let sum = skills
        .iter()
        .take(6)
        .fold(0u32, |sum, &level| sum.saturating_add(level));
    ((sum / 2).min(5)) as u8
}

/// A rank index's label from the original's table. A negative index or one
/// past the table is shown as the dash; the original reads it unchecked.
pub fn rank_name(rank: i32) -> &'static str {
    usize::try_from(rank)
        .ok()
        .and_then(|index| RANK_LABELS.get(index).copied())
        .unwrap_or(NO_RANK)
}

/// The tier column of a daily event: a dash for Challenger, Inept for an
/// unranked event, else the tier's label.
pub fn event_tier_name(event: &MatchRecord) -> &'static str {
    if event.kind == CHALLENGER_EVENT {
        NO_RANK
    } else if event.tier < 0 {
        RANK_LABELS[0]
    } else {
        rank_name(event.tier)
    }
}

/// A packed coin amount's copper, silver and gold counts: a copper byte, a
/// silver byte and the high 16 bits of gold.
pub fn coin_parts(packed: u32) -> (u32, u32, u32) {
    (packed & 0xff, (packed >> 8) & 0xff, packed >> 16)
}

/// A packed coin amount in copper: twelve copper to the silver and twelve
/// silver to the gold.
pub fn coin_value(packed: u32) -> u32 {
    let (copper, silver, gold) = coin_parts(packed);
    copper + silver * 12 + gold * 144
}

/// Packs a copper amount the way the original stores it: the gold count is
/// kept in 16 bits and the remainder split into a silver and a copper byte,
/// each wrapping as the original's byte stores do.
pub fn pack_coins(value: u32) -> u32 {
    let gold = (value / 144) as u16;
    let rest = value.wrapping_sub(u32::from(gold) * 144);
    let silver = (rest / 12) as u8;
    let copper = (rest as u8).wrapping_sub(silver.wrapping_mul(12));
    u32::from(copper) | (u32::from(silver) << 8) | (u32::from(gold) << 16)
}

/// Scales a packed coin amount by a factor with the original's 32-bit
/// multiply.
pub fn scale_coins(packed: u32, factor: i32) -> u32 {
    pack_coins((coin_value(packed) as i32).wrapping_mul(factor) as u32)
}

/// Whether the day's three matches remain. The counter is the signed int
/// the original compares.
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
/// teammate be two ranks below the floor; the lineup's first fighter still
/// needs the full floor. The original compares in 64 bits, so the floor
/// cannot wrap.
pub fn rank_passes(match_id: u32, floor: i32, rank: i32, first: bool) -> bool {
    if match_id == CAPTAIN_EVENT && !first {
        i64::from(floor) - 2 <= i64::from(rank)
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
    /// The selected rank, 0 through 5.
    pub rank: usize,
    /// The selected tier, 0 through 5, never above the rank.
    pub tier: usize,
    /// The match the lineup is built for, copied by the last refresh.
    pub selected: MatchRecord,
    /// Matches the company has entered today; the day allows three. The
    /// arena counts them, not the board.
    pub matches_played: i32,
    /// The player's company name.
    pub company_name: String,
    /// The rival companies, in company id order from 2.
    pub rivals: Vec<String>,
    /// The tournament's company table: a winner from 2 reads entry
    /// `winner - 2`, which holds a company id less one, and the winners
    /// table shows that company's name.
    pub tournament_companies: Vec<i32>,
    /// Who won each of the season's four weeks.
    pub week_winners: [u32; 4],
    /// The four seasons' four rewards.
    pub season_rewards: [[u32; 4]; 4],
    /// The six tournament events.
    pub tournament_events: [MatchRecord; 6],
    /// The tournament weapon rows, by tier.
    pub weapon_tiers: Vec<Vec<WeaponRow>>,
    /// The six daily events.
    pub events: [MatchRecord; 6],
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
    /// The event table's selected row.
    pub event_row: usize,
    /// The daily event the last refresh copied. It differs from the row
    /// when every event is greyed: the row moves to the last one and the
    /// record stays.
    pub event_selection: usize,
    /// The winners table's selection (zero-based).
    pub winner_selection: usize,
    /// The company table's selected row, if any.
    pub available_selection: Option<usize>,
    /// The lineup table's selected row, if any.
    pub team_selection: Option<usize>,
    /// The season table's selected row, if any.
    pub season_selection: Option<usize>,
    /// The shared message box under the lists.
    pub standard_message: String,
    /// The reward panel's text: the weapon row's name on a weapon event.
    pub tournament_note: String,
    /// The item id the tournament tab's reward slot gets when it opens.
    pub tournament_prize_item: Option<u32>,
    /// The tournament reward item id, if one was created.
    pub tournament_reward: Option<u32>,
    /// The four season reward item ids, in slot order.
    pub season_items: [Option<u32>; 4],
    /// The control states, indexed by [`control`].
    pub controls: [u32; CONTROL_COUNT],
    /// The standard tab's scaled entry fee and prize, shown in
    /// [`control::STANDARD_FEE`] and [`control::STANDARD_PRIZE`].
    pub standard_fee: u32,
    pub standard_prize: u32,
    /// The lineup table's height in GUI units after a tournament refresh
    /// ([`LINEUP_UNITS`], [`COMPACT_LINEUP_UNITS`] or
    /// [`WEAPON_LINEUP_UNITS`]).
    pub lineup_height_override: Option<i32>,
    /// The GUI unit the board's form and child views are laid out at
    /// (4 is the constructor's own numbers).
    pub unit: i32,
}

impl Board {
    /// A board with the player at pool slot 0, the roster after it, and the
    /// standard tab open. The board does not own the installation's data:
    /// the caller fills the events, entries and names.
    pub fn new(compact: bool, day: u32, fighters: Vec<Fighter>) -> Self {
        let controls = TEMPLATE_CONTROLS;
        let mut board = Self {
            compact,
            day,
            tab: BoardTab::Matches,
            match_type: 0,
            rank: 0,
            tier: 0,
            selected: STANDARD_MATCHES[0].record(),
            matches_played: 0,
            company_name: String::new(),
            rivals: Vec::new(),
            tournament_companies: Vec::new(),
            week_winners: [0; 4],
            season_rewards: [[0; 4]; 4],
            tournament_events: std::array::from_fn(|_| MatchRecord::default()),
            weapon_tiers: Vec::new(),
            events: std::array::from_fn(|_| MatchRecord::default()),
            entries: std::array::from_fn(|_| EventEntry::default()),
            fighters,
            lineup: Vec::new(),
            available: Vec::new(),
            team: Vec::new(),
            event_row: 0,
            event_selection: 0,
            winner_selection: 0,
            available_selection: None,
            team_selection: None,
            season_selection: None,
            standard_message: String::new(),
            tournament_note: String::new(),
            tournament_prize_item: None,
            tournament_reward: None,
            season_items: [None; 4],
            controls,
            standard_fee: STANDARD_MATCHES[0].fee,
            standard_prize: STANDARD_MATCHES[0].prize,
            lineup_height_override: None,
            unit: 4,
        };
        board.rebuild_lineups();
        board.apply_tab(None, BoardTab::Matches);
        board
    }

    /// The layout for the board's form size.
    pub fn layout(&self) -> BoardLayout {
        layout_at(self.compact, self.unit)
    }

    /// The selected match's type id.
    pub fn match_id(&self) -> u32 {
        self.selected.kind
    }

    /// The lineup's rank floor: the selected match's lowest rank.
    pub fn rank_floor(&self) -> i32 {
        self.selected.min_rank
    }

    /// Fighters per side for the selected match.
    pub fn team_size(&self) -> i32 {
        self.selected.team_size
    }

    /// Whether the selected match lifts the one-leader and marked rules.
    pub fn free_lineup(&self) -> bool {
        self.selected.flags & FLAG_FREE_LINEUP != 0
    }

    /// Whether the selected match names a weapon.
    pub fn weapon_event(&self) -> bool {
        self.selected.flags & FLAG_WEAPON != 0
    }

    /// The tier the selected match carries: none for an unranked match
    /// (Pugilism on the standard tab).
    pub fn reported_tier(&self) -> Option<usize> {
        usize::try_from(self.selected.max_rank).ok()
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

    /// Whether the lineup has reached the side size.
    fn team_full(&self) -> bool {
        self.team.len() as i64 >= i64::from(self.team_size())
    }

    /// The weekly tournament's status text and the two button states.
    pub fn tournament_status(&self) -> TournamentStatus {
        let mut status = TournamentStatus {
            text: String::new(),
            button_label: "Enter",
            enter: control::HIDDEN,
            secondary: control::HIDDEN,
            text_state: control::SHOWN,
        };
        if !is_tournament_day(self.day) {
            let days = tournament_begins_in(self.day);
            status.text = if days == 1 {
                "Tournament begins tomorrow.".to_string()
            } else {
                format!("Tournament begins in {} days.", days)
            };
            return status;
        }
        let entered = self.has_entrants();
        let unsettled = self.has_unsettled_entries();
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
            status.text = if self.entries[5].winner > 0 {
                TOURNAMENT_ENDED.to_string()
            } else {
                NO_MORE_EVENTS.to_string()
            };
        } else {
            status.text = TOURNAMENT_IN_PROGRESS.to_string();
        }
        status
    }

    /// The "Entered n/m" label. The original writes a single character by
    /// adding 0x30 to each count, so a count above nine wraps into
    /// punctuation rather than becoming two digits.
    pub fn entered_label(&self) -> String {
        format!(
            "Entered {}/{}",
            count_char(self.team.len() as i64),
            count_char(i64::from(self.team_size()))
        )
    }

    /// The primary button's state.
    pub fn enter_state(&self) -> u32 {
        self.controls[control::ENTER]
    }

    /// Whether a control is shown and enabled.
    pub fn control_enabled(&self, index: usize) -> bool {
        let state = self.controls.get(index).copied().unwrap_or(control::HIDDEN);
        !control::hidden(state) && !control::disabled(state)
    }

    /// Whether the control at an index is shown, hidden or disabled.
    pub fn control_state(&self, index: usize) -> u32 {
        self.controls.get(index).copied().unwrap_or(control::HIDDEN)
    }

    /// The primary button's update after a lineup change. The tournament
    /// tab keeps the state its status gave it; the season tab hides the
    /// button; otherwise it is disabled once the day's matches are spent
    /// and while the lineup is short.
    fn refresh_enter(&mut self) {
        let full = self.team_full();
        let left = matches_left(self.matches_played);
        let state = &mut self.controls[control::ENTER];
        match self.tab {
            BoardTab::Tournament => {}
            BoardTab::Season => *state = control::HIDDEN,
            _ if !left => *state = control::DISABLED,
            _ if full => *state &= !control::DISABLED,
            _ => *state |= control::DISABLED,
        }
    }

    /// The day-end control's enable rule. On the weekly tournament day with
    /// any entered event, the control is enabled once every entered event
    /// has a winner; the match count does not matter then. On every other
    /// day (and a tournament day with no entrants) it is enabled once a
    /// match has been played.
    pub fn day_end_enabled(&self) -> bool {
        if is_tournament_day(self.day) && self.has_entrants() {
            return !self.has_unsettled_entries();
        }
        self.matches_played > 0
    }

    /// Whether an entered event still has no winner.
    pub fn has_unsettled_entries(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| !entry.participants.is_empty() && entry.winner == 0)
    }

    /// The message box text the standard and events refreshes write.
    fn day_message(&self) -> String {
        if matches_left(self.matches_played) {
            String::new()
        } else {
            NO_MORE_MATCHES.to_string()
        }
    }

    /// The standard tab's refresh: copy the standard match, clamp the tier
    /// to the rank, store the rank as the floor and the tier (none for
    /// Pugilism), scale the fee and prize by the rank plus one, write the
    /// day's message and rebuild the lineup.
    pub fn refresh_standard(&mut self) {
        let template = STANDARD_MATCHES[self.match_type.min(STANDARD_MATCHES.len() - 1)];
        if self.tier > self.rank {
            self.tier = self.rank;
        }
        let rank = self.rank as i32;
        let tier = if template.kind == UNRANKED_MATCH {
            -1
        } else {
            self.tier as i32
        };
        let mut record = template.record();
        record.min_rank = rank;
        record.max_rank = tier;
        record.rank = rank;
        record.tier = tier;
        record.fee = scale_coins(template.fee, rank + 1);
        record.prize = scale_coins(template.prize, rank + 1);
        self.standard_fee = record.fee;
        self.standard_prize = record.prize;
        self.selected = record;
        self.standard_message = self.day_message();
        self.rebuild_lineups();
    }

    /// Whether a tier row is greyed by the selected rank: every row above
    /// it.
    pub fn tier_row_disabled(&self, row: usize) -> bool {
        row > self.rank
    }

    /// The events tab's refresh. A greyed event moves the row to the first
    /// event that is not greyed; when all six are greyed the row moves to
    /// the last one and the greyed record is still copied.
    pub fn refresh_events(&mut self) {
        let row = self.event_row.min(self.events.len() - 1);
        let mut record = row;
        if self.events[row].flags & FLAG_FREE_LINEUP != 0 {
            match self
                .events
                .iter()
                .position(|event| event.flags & FLAG_FREE_LINEUP == 0)
            {
                Some(found) => {
                    record = found;
                    self.event_row = found;
                }
                None => self.event_row = self.events.len() - 1,
            }
        }
        self.event_selection = record;
        self.selected = self.events[record].clone();
        self.standard_message = self.day_message();
        self.rebuild_lineups();
    }

    /// Rebuilds the lineup and the company table from the pool.
    pub fn rebuild_lineups(&mut self) {
        let count = self.fighters.len();
        if self.lineup.len() != count {
            self.lineup = (1..count)
                .map(|slot| {
                    if self.fighters[slot].is_combatant() {
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
            let participants = self
                .entries
                .get(self.winner_selection)
                .map(|entry| entry.participants.as_slice())
                .unwrap_or(&[]);
            for participant in participants {
                // The original reads an out-of-range slot unchecked.
                if participant.slot < count {
                    used[participant.slot] = true;
                    self.team.push(participant.slot);
                }
            }
        } else {
            let kind = self.match_id();
            let floor = self.rank_floor();
            let free = self.free_lineup();
            let size = i64::from(self.team_size());
            let mut have_leader = false;
            for &order in &self.lineup {
                // The marker and slots past the pool fail the original's
                // bound; a negative slot is skipped where it reads unchecked.
                let Ok(slot) = usize::try_from(order) else {
                    continue;
                };
                let Some(fighter) = self.fighters.get(slot) else {
                    continue;
                };
                if slot > 0 && !fighter.is_combatant() {
                    continue;
                }
                if !rank_passes(kind, floor, fighter.rank as i32, self.team.is_empty()) {
                    continue;
                }
                if slot == 0 || fighter.is_leader() {
                    if have_leader && !free {
                        continue;
                    }
                    have_leader = true;
                }
                used[slot] = true;
                self.team.push(slot);
                if self.team.len() as i64 >= size {
                    break;
                }
            }
        }
        self.available_selection = None;
        self.team_selection = None;
        for (slot, &is_used) in used.iter().enumerate() {
            if !is_used && (slot == 0 || self.fighters[slot].is_combatant()) {
                self.available.push(slot);
            }
        }
        self.refresh_enter();
    }

    /// Whether a fighter may be moved into the lineup by a double click or
    /// dropped onto it. The Captain event's lower floor applies to every
    /// such fighter.
    pub fn accepts_rank(&self, slot: usize) -> bool {
        self.fighters.get(slot).is_some_and(|fighter| {
            rank_passes(
                self.match_id(),
                self.rank_floor(),
                fighter.rank as i32,
                false,
            )
        })
    }

    /// Whether a fighter may be dropped onto the lineup.
    pub fn can_drop(&self, slot: usize) -> bool {
        !self.assignment_locked() && self.accepts_rank(slot)
    }

    fn ranked(&self, slot: usize) -> bool {
        self.fighters
            .get(slot)
            .is_some_and(|fighter| fighter.rank as i32 >= self.rank_floor())
    }

    /// Moves an available fighter into the lineup. `target` is a zero-based
    /// lineup row for a drop, or `None` for a double click or a drop on no
    /// row. Returns whether the tables changed.
    ///
    /// A marked fighter swaps with the first marked teammate. Otherwise a
    /// double click joins when there is room and else benches the last
    /// teammate to the end of the company table; a drop swaps with the row.
    /// On a Captain event a fighter under the floor may not replace a
    /// ranked teammate when fewer than two of more than two are ranked, and
    /// a qualified teammate is moved first. Off the tournament tab the
    /// fighter's place in the lineup order becomes the row it joined, or
    /// the first place after a marked swap or a bench, as the original's
    /// unset target leaves it.
    pub fn move_to_team(&mut self, index: usize, target: Option<usize>) -> bool {
        if self.assignment_locked() {
            return false;
        }
        let Some(&slot) = self.available.get(index) else {
            return false;
        };
        let Some((rank, marked)) = self
            .fighters
            .get(slot)
            .map(|fighter| (fighter.rank as i32, fighter.marked))
        else {
            return false;
        };
        let unknown_rank = self.match_id() == CAPTAIN_EVENT
            && rank < self.rank_floor()
            && self.team.len() > 2
            && self
                .team
                .iter()
                .filter(|&&member| self.ranked(member))
                .count()
                < 2;
        // A drop past the last row is taken as a drop on no row; the
        // original indexes the lineup with it unchecked.
        let target = target.filter(|&row| row < self.team.len());
        let mut order_target = 0;
        let mut placed = false;
        if !self.free_lineup() && marked {
            let marked = self.team.iter().position(|&member| {
                self.fighters
                    .get(member)
                    .is_some_and(|fighter| fighter.marked)
            });
            if let Some(found) = marked {
                let row = if self.team[found] == slot {
                    target.unwrap_or(found)
                } else {
                    found
                };
                if unknown_rank && self.ranked(self.team[found]) {
                    return false;
                }
                self.available[index] = self.team[row];
                self.team[row] = slot;
                placed = true;
            }
        }
        if !placed {
            match target {
                Some(row) => {
                    if unknown_rank && self.ranked(self.team[row]) {
                        return false;
                    }
                    self.available[index] = self.team[row];
                    self.team[row] = slot;
                    order_target = row;
                }
                None if !self.team_full() => {
                    self.team.push(slot);
                    self.available.remove(index);
                    order_target = self.team.len() - 1;
                }
                None => {
                    // The original benches into an empty lineup by reading
                    // before it; nothing happens here.
                    let Some(last) = self.team.last_mut() else {
                        return false;
                    };
                    let benched = std::mem::replace(last, slot);
                    self.available.push(benched);
                    self.available.remove(index);
                }
            }
            self.available_selection =
                clamp_selection(self.available_selection, self.available.len());
        }
        if self.match_id() == CAPTAIN_EVENT
            && self.team.len() > 2
            && !self.ranked(self.team[0])
            && let Some(found) = (1..self.team.len()).find(|&row| self.ranked(self.team[row]))
        {
            self.team.swap(0, found);
        }
        if self.tab == BoardTab::Tournament {
            self.record_participants();
        } else {
            self.move_in_lineup(slot, order_target);
        }
        self.refresh_enter();
        true
    }

    /// Moves a lineup fighter back to the company table, inserting it in
    /// slot order. The original's two removal paths do not change the
    /// lineup order, so a later rebuild can select the fighter again; this
    /// keeps that behaviour rather than correcting it.
    pub fn move_to_available(&mut self, index: usize) -> bool {
        if self.assignment_locked() || index >= self.team.len() {
            return false;
        }
        let slot = self.team.remove(index);
        self.insert_available(slot);
        self.team_selection = clamp_selection(self.team_selection, self.team.len());
        if self.tab == BoardTab::Tournament {
            self.record_participants_removed(index);
        }
        self.refresh_enter();
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

    /// Inserts a pool slot into the company table in slot order, before the
    /// first row whose slot is greater (an unsigned compare in the
    /// original).
    pub fn insert_available(&mut self, slot: usize) {
        let at = self
            .available
            .iter()
            .position(|&row| row > slot)
            .unwrap_or(self.available.len());
        self.available.insert(at, slot);
    }

    /// Moves a pool slot to a place in the lineup order, appending it first
    /// when it is missing.
    pub fn move_in_lineup(&mut self, slot: usize, target: usize) {
        if let Some(position) = self.lineup.iter().position(|&row| row == slot as i32) {
            self.lineup.remove(position);
        }
        let target = target.min(self.lineup.len());
        self.lineup.insert(target, slot as i32);
    }

    /// Records the current lineup in the selected tournament event.
    pub fn record_participants(&mut self) {
        let event = self.winner_selection;
        let participants: Vec<Participant> = self
            .team
            .iter()
            .map(|&slot| Participant {
                slot,
                value: self.event_value(slot, event).unwrap_or(0),
                weapon: self
                    .fighters
                    .get(slot)
                    .and_then(|fighter| fighter.event_weapons.get(event))
                    .copied()
                    .unwrap_or(0),
            })
            .collect();
        if let Some(entry) = self.entries.get_mut(event) {
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

    /// The selected event's weapon row: its tier is the record's highest
    /// rank and its row the record's one-based weapon row.
    pub fn weapon_row(&self, tier: usize) -> Option<&WeaponRow> {
        let row = usize::try_from(self.selected.weapon_row)
            .ok()?
            .checked_sub(1)?;
        self.weapon_tiers.get(tier)?.get(row)
    }

    /// The weapon check's mark on the reward panel. `None` leaves the mark
    /// as it is (no weapon event); otherwise the mark is set when the first
    /// fighter entered in the event's tier does not carry one of the row's
    /// weapons, and cleared when that weapon is accepted or when nobody is
    /// entered (the check starts from "accepted" and only runs with an
    /// entrant). A row outside the table counts as no match; a tier outside
    /// the entries counts as nobody entered.
    pub fn weapon_mark(&self) -> Option<bool> {
        if !self.weapon_event() {
            return None;
        }
        let tier = usize::try_from(self.selected.max_rank).ok();
        let first = tier
            .and_then(|tier| self.entries.get(tier))
            .and_then(|entry| entry.participants.first());
        let Some(first) = first else {
            return Some(false);
        };
        let accepted = tier
            .and_then(|tier| self.weapon_row(tier))
            .is_some_and(|row| row.accepts(first.weapon));
        Some(!accepted)
    }

    /// The tournament tab's refresh: copy the selected tournament event; a
    /// weapon event shows the reward panel with the weapon row's name and
    /// shortens the lineup to 30 units (four rows), any other sets it to 61
    /// units (56 compact). The message box is cleared and the lineup
    /// rebuilt from the event's entry.
    pub fn refresh_tournament(&mut self) {
        let event = self.winner_selection.min(self.tournament_events.len() - 1);
        self.selected = self.tournament_events[event].clone();
        if self.weapon_event() {
            self.lineup_height_override = Some(WEAPON_LINEUP_UNITS);
            self.tournament_note = self
                .weapon_row(event)
                .map(|row| row.name.clone())
                .unwrap_or_default();
        } else {
            self.lineup_height_override = Some(if self.compact {
                COMPACT_LINEUP_UNITS
            } else {
                LINEUP_UNITS
            });
        }
        self.standard_message.clear();
        self.rebuild_lineups();
    }

    /// Whether the reward panel's own control is disabled: on a tournament
    /// day of a weapon event.
    pub fn reward_panel_locked(&self) -> bool {
        self.weapon_event() && is_tournament_day(self.day)
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
        self.rival_name(winner as i64).unwrap_or_default()
    }

    /// A rival company's name by company id, from 2.
    fn rival_name(&self, id: i64) -> Option<String> {
        usize::try_from(id - 2)
            .ok()
            .and_then(|index| self.rivals.get(index))
            .cloned()
    }

    /// Fills the season's reward item ids and returns the picture name.
    pub fn refresh_season(&mut self) -> String {
        let season = season_index(self.day) as usize;
        let rewards = self.season_rewards.get(season).copied().unwrap_or([0; 4]);
        let ordered = season_reward_order(rewards);
        self.season_items = ordered.map(|id| if id == 0 { None } else { Some(id) });
        for index in [
            control::SEASON_PICTURE,
            control::SEASON_ITEM_0,
            control::SEASON_ITEM_1,
            control::SEASON_ITEM_2,
            control::SEASON_ITEM_3,
            control::SEASON_TAIL,
        ] {
            self.controls[index] = control::SHOWN;
        }
        season_picture_name(self.day)
    }

    /// The show method: release the tournament reward item, recompute the
    /// day's flags, and switch to the tournament tab on a tournament day
    /// with entries (hiding the tab it leaves), or re-open the current tab.
    /// Only the re-open clears the current tab before applying it, so the
    /// tab's own show runs again; the forced switch keeps the current tab
    /// and goes through the normal hide. A forced switch onto an already
    /// open tournament tab takes the re-open path.
    pub fn show(&mut self) -> ShowState {
        self.tournament_reward = None;
        let tournament_day = self.tournament_day();
        let has_entrants = self.has_entrants();
        let forced = tournament_day && has_entrants;
        if forced && self.tab != BoardTab::Tournament {
            self.apply_tab(Some(self.tab), BoardTab::Tournament);
        } else {
            self.apply_tab(None, self.tab);
        }
        ShowState {
            tournament_day,
            has_entrants,
            tab: self.tab,
            forced_tournament: forced,
        }
    }

    /// The close method: remember the tab and the three list selections and
    /// release the items and the season picture.
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

    /// Restores a previous session's tab and list selections, as the
    /// original's constructor does from its saved selections when the board
    /// is opened again. The selections are clamped to the six-row lists and
    /// the saved tab is opened, which refreshes it like a tab switch. The
    /// saved items and picture are not restored: the tab's own show creates
    /// them again.
    pub fn restore(&mut self, state: &CloseState) {
        let last = RANKS.len() - 1;
        self.match_type = state.match_type.min(MATCH_TYPES.len() - 1);
        self.rank = state.rank.min(last);
        self.tier = state.tier.min(last);
        let target = state.tab;
        self.apply_tab(None, target);
    }

    /// Whether the tournament day flag is set.
    pub fn tournament_day(&self) -> bool {
        is_tournament_day(self.day)
    }

    /// Applies a tab. `old` is the tab being left; `None` skips hiding,
    /// which is what the show method does when it reopens a tab. A tab
    /// equal to the one being left changes nothing past the shared states.
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
        self.controls[control::DAY_END] = if self.day_end_enabled() {
            control::SHOWN
        } else {
            control::DISABLED
        };
        let mut hidden = [false; 4];
        if old != Some(new) {
            if let Some(old) = old {
                self.hide_tab(old);
                hidden[old.index()] = true;
            }
            self.tab = new;
            self.controls[new.index()] |= CONTROL_SELECTED;
            self.show_tab(new);
        }
        TabView {
            tab: self.tab,
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
        self.controls[tab.index()] &= !CONTROL_SELECTED;
        match tab {
            BoardTab::Matches => {
                for index in control::STANDARD_FIRST..=control::STANDARD_LAST {
                    self.controls[index] = control::HIDDEN;
                }
            }
            BoardTab::Events => {}
            BoardTab::Tournament => {
                self.lineup_height_override = None;
                self.controls[control::PRIZE_ITEM] = control::HIDDEN;
                self.controls[control::PRIZE_CAPTION] = control::HIDDEN;
                self.controls[control::ENTER] = control::SHOWN;
                self.controls[control::SECONDARY] = control::HIDDEN;
                self.controls[control::STATUS] = control::HIDDEN;
            }
            BoardTab::Season => {
                self.controls[control::SHARED] = control::SHOWN;
                self.controls[control::ENTERED] = control::SHOWN;
                for index in control::SEASON_PICTURE..=control::SEASON_TAIL {
                    self.controls[index] = control::HIDDEN;
                }
            }
        }
    }

    fn show_tab(&mut self, tab: BoardTab) {
        match tab {
            BoardTab::Matches => {
                for index in control::STANDARD_FIRST..=control::STANDARD_LAST {
                    self.controls[index] = control::SHOWN;
                }
                self.refresh_standard();
            }
            BoardTab::Events => self.refresh_events(),
            BoardTab::Tournament => {
                self.controls[control::PRIZE_ITEM] = control::SHOWN;
                if self.tournament_reward.is_none() {
                    self.tournament_reward = self.tournament_prize_item;
                }
                self.controls[control::PRIZE_CAPTION] = control::SHOWN;
                self.refresh_tournament();
                let status = self.tournament_status();
                self.controls[control::ENTER] = status.enter;
                self.controls[control::SECONDARY] = status.secondary;
                self.controls[control::STATUS] = status.text_state;
            }
            BoardTab::Season => {
                self.controls[control::SHARED] = control::HIDDEN;
                self.controls[control::ENTERED] = control::HIDDEN;
                self.controls[control::ENTER] = control::HIDDEN;
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
    /// cell, found as the table view finds its row and column
    /// ([`TableLayout::row_at`], [`TableLayout::column_at`]).
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
        if !control::hidden(self.controls[control::CLOSE])
            && layout.close_button.contains(point.0, point.1)
        {
            return BoardHit::Close;
        }
        if self.control_enabled(control::DAY_END)
            && layout.end_day_button.contains(point.0, point.1)
        {
            return BoardHit::EndDay;
        }
        if self.control_enabled(control::SECONDARY)
            && layout.forfeit_button.contains(point.0, point.1)
        {
            return BoardHit::Forfeit;
        }
        for &slot in self.visible_tables() {
            let table = self.table_layout(slot);
            if !table.rect.contains(point.0, point.1) {
                continue;
            }
            let Some(row) = table.row_at(point.1) else {
                continue;
            };
            if row >= table.rows {
                continue;
            }
            let column = table.column_at(point.0);
            return BoardHit::Table { slot, row, column };
        }
        BoardHit::Outside
    }

    /// Turns a click at a point into the matching board input.
    pub fn click(&mut self, point: (i32, i32)) -> BoardAction {
        match self.hit_test(point) {
            BoardHit::Tab(index) => self.handle(BoardInput::SelectTab(index)),
            BoardHit::Enter => self.handle(BoardInput::Confirm),
            BoardHit::Close => self.handle(BoardInput::Close),
            BoardHit::EndDay => BoardAction::EndDay,
            BoardHit::Forfeit => BoardAction::Forfeit,
            BoardHit::Table { slot, row, .. } => match slot {
                TableSlot::Match => self.handle(BoardInput::SelectMatch(row)),
                TableSlot::Rank => self.handle(BoardInput::SelectRank(row)),
                TableSlot::Tier => self.handle(BoardInput::SelectTier(row)),
                TableSlot::Events => self.handle(BoardInput::ClickEvent(row)),
                TableSlot::Winners => self.handle(BoardInput::ClickWinner(row)),
                TableSlot::Roster => {
                    self.available_selection = Some(row);
                    BoardAction::Changed
                }
                TableSlot::Lineup => {
                    self.team_selection = Some(row);
                    BoardAction::Changed
                }
                TableSlot::Season => {
                    self.season_selection = Some(row);
                    BoardAction::Changed
                }
            },
            BoardHit::Outside => BoardAction::None,
        }
    }

    /// Handles one board input and returns the action the arena should take.
    pub fn handle(&mut self, input: BoardInput) -> BoardAction {
        let changed = |done: bool| {
            if done {
                BoardAction::Changed
            } else {
                BoardAction::None
            }
        };
        match input {
            BoardInput::SelectTab(index) => match BoardTab::from_index(index) {
                Some(tab) => changed(self.switch_tab(tab).is_some()),
                None => BoardAction::None,
            },
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
                // A rank change also moves the tier list to the same row.
                self.rank = index;
                self.tier = index;
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
                self.event_row = index;
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
            BoardInput::DoubleClickAvailable(index) => changed(self.double_click_available(index)),
            BoardInput::DoubleClickTeam(index) => changed(self.double_click_team(index)),
            BoardInput::DropOnTeam { from, to } => {
                changed(self.can_drop_index(from) && self.move_to_team(from, to))
            }
            BoardInput::DropOnAvailable(index) => changed(self.move_to_available(index)),
            BoardInput::Confirm => self.confirm(),
            BoardInput::Close => BoardAction::Close,
        }
    }

    fn can_drop_index(&self, index: usize) -> bool {
        self.available
            .get(index)
            .is_some_and(|&slot| self.can_drop(slot))
    }

    /// The primary button. The button's own handler is outside the board's
    /// methods; this reports what the arena should start and leaves the
    /// day's match count to the arena.
    fn confirm(&mut self) -> BoardAction {
        let state = self.enter_state();
        if control::hidden(state) || control::disabled(state) {
            return BoardAction::None;
        }
        match self.tab {
            BoardTab::Matches | BoardTab::Events => BoardAction::EnterMatch,
            BoardTab::Tournament => match self.tournament_status().button_label {
                "Begin" => BoardAction::BeginTournament,
                "Continue" => BoardAction::ContinueTournament,
                _ => BoardAction::EnterMatch,
            },
            BoardTab::Season => BoardAction::None,
        }
    }

    /// The rows of the standard match list.
    pub fn match_rows(&self) -> [&'static str; 6] {
        MATCH_TYPES
    }

    /// The rows of the rank list.
    pub fn rank_rows(&self) -> [&'static str; 6] {
        RANKS
    }

    /// The rows of the tier list, and whether the rank greys each one.
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

    /// The tournament winners table: the event's name, the row's tier and
    /// the winner's company. A winner from 2 goes through the tournament's
    /// company table at entry `winner - 2`; the entry is a company id less
    /// one, so the name comes from the companies from id 2.
    pub fn winner_cells(&self, row: usize) -> Option<[CellValue; 3]> {
        let name = self.tournament_events.get(row)?.name.clone();
        let winner = self.entries.get(row).map(|entry| entry.winner).unwrap_or(0);
        let text = if winner == 1 {
            self.company_name.clone()
        } else if winner < 1 {
            NO_RANK.to_string()
        } else {
            usize::try_from(winner - 2)
                .ok()
                .and_then(|index| self.tournament_companies.get(index))
                .and_then(|&company| self.rival_name(i64::from(company) + 1))
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

    /// The lineup table's height: the last tournament refresh's units
    /// times the unit, else the constructor's (rescaled) height.
    pub fn lineup_height(&self) -> i32 {
        self.lineup_height_override.map_or_else(
            || self.layout().lineup_table.rect.h,
            |units| units * self.unit,
        )
    }

    /// A coin cell's drawing. The column callback receives the cell's low
    /// dword and the owner-drawn cell's x (its pen, a unit in from the
    /// column's left edge): zero is the dash four pixels right, anything
    /// else coin icons at that x.
    pub fn coin_draw(packed: u32, x: i32) -> CoinDraw {
        if packed == 0 {
            CoinDraw::Dash { x: x + 4 }
        } else {
            let (copper, silver, gold) = coin_parts(packed);
            CoinDraw::Coins {
                x,
                copper,
                silver,
                gold,
            }
        }
    }

    /// The board's paint list in drawing order: the form, the four tabs,
    /// then the visible child views in the order the constructor created
    /// them (match, rank and tier lists, picture, reward panel, events,
    /// winners, message, company, lineup and season tables), then the
    /// template's button and label. Where the template's controls draw
    /// relative to the child views is a guess.
    pub fn paint(&self) -> Vec<BoardPaint> {
        let layout = self.layout();
        let mut paint = vec![BoardPaint::Frame {
            widget: Widget::Form,
            rect: layout.form,
        }];
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
                            .is_some_and(|event| event.flags & FLAG_FREE_LINEUP != 0);
                        (Vec::from(cells), disabled)
                    })
                });
                self.paint_message(&mut paint, &layout);
                self.paint_fighters(&mut paint, &layout);
            }
            BoardTab::Tournament => {
                paint.push(BoardPaint::Frame {
                    widget: Widget::Picture,
                    rect: layout.picture,
                });
                if self.weapon_event() {
                    paint.push(BoardPaint::Frame {
                        widget: Widget::RewardPanel,
                        rect: layout.reward_panel,
                    });
                    let note = Rect {
                        x: layout.reward_panel.x + 4,
                        y: layout.reward_panel.y + 4,
                        w: layout.reward_panel.w - 8,
                        h: table_view::ROW_UNITS * self.unit,
                    };
                    paint.push(BoardPaint::Label {
                        widget: Widget::RewardPanel,
                        rect: Some(note),
                        text: self.tournament_note.clone(),
                    });
                    if self.weapon_mark() == Some(true) {
                        paint.push(BoardPaint::Frame {
                            widget: Widget::WeaponMark,
                            rect: note,
                        });
                    }
                }
                self.paint_table(&mut paint, &layout.winners_table, &|board, row| {
                    board
                        .winner_cells(row)
                        .map(|cells| (Vec::from(cells), false))
                });
                if self.tournament_reward.is_some() {
                    paint.push(BoardPaint::Frame {
                        widget: Widget::Prize,
                        rect: layout.prize_slot,
                    });
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
                paint.push(BoardPaint::Frame {
                    widget: Widget::SeasonPicture,
                    rect: layout.season_picture,
                });
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
        // The template's own controls: the bottom row, the labels above the
        // two fighter tables and the close button. The status label shares
        // the primary button's place, so only one of them is visible.
        if !control::hidden(self.enter_state()) {
            paint.push(BoardPaint::Label {
                widget: Widget::EnterButton,
                rect: Some(layout.enter_button),
                text: self.tab_button_label(),
            });
        }
        if !control::hidden(self.control_state(control::SECONDARY)) {
            paint.push(BoardPaint::Label {
                widget: Widget::ForfeitButton,
                rect: Some(layout.forfeit_button),
                text: FORFEIT_LABEL.to_string(),
            });
        }
        if !control::hidden(self.control_state(control::CLOSE)) {
            paint.push(BoardPaint::Label {
                widget: Widget::CloseButton,
                rect: Some(layout.close_button),
                text: CLOSE_LABEL.to_string(),
            });
        }
        if !control::hidden(self.control_state(control::DAY_END)) {
            paint.push(BoardPaint::Label {
                widget: Widget::EndDayButton,
                rect: Some(layout.end_day_button),
                text: END_DAY_LABEL.to_string(),
            });
        }
        if !control::hidden(self.control_state(control::STANDARD_ENTRY_CAPTION)) {
            paint.push(BoardPaint::Label {
                widget: Widget::EntryCaption,
                rect: Some(layout.entry_caption),
                text: ENTRY_CAPTION.to_string(),
            });
        }
        if !control::hidden(self.control_state(control::STANDARD_PRIZE_CAPTION)) {
            paint.push(BoardPaint::Label {
                widget: Widget::PrizeCaption,
                rect: Some(layout.prize_caption),
                text: PRIZE_CAPTION.to_string(),
            });
        }
        if !control::hidden(self.control_state(control::PRIZE_CAPTION)) {
            paint.push(BoardPaint::Label {
                widget: Widget::TournamentPrizeCaption,
                rect: Some(layout.tournament_prize_caption),
                text: TOURNAMENT_PRIZE_CAPTION.to_string(),
            });
        }
        if !control::hidden(self.control_state(control::SEASON_TAIL)) {
            paint.push(BoardPaint::Label {
                widget: Widget::SeasonCaption,
                rect: Some(layout.season_caption),
                text: SEASON_CAPTION.to_string(),
            });
        }
        if self.tab != BoardTab::Season {
            paint.push(BoardPaint::Label {
                widget: Widget::AvailableLabel,
                rect: Some(layout.available_label),
                text: AVAILABLE_LABEL.to_string(),
            });
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
        paint_table_rows(paint, &layout.roster_table, &self.available, |slot| {
            self.fighter_cells(slot)
                .map(|cells| (Vec::from(cells), false))
        });
        let mut lineup = layout.lineup_table;
        lineup.rect.h = self.lineup_height();
        paint_table_rows(paint, &lineup, &self.team, |slot| {
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

/// A table's selection after it lost rows: a selection past the last row
/// moves to the last row, or to none when the table is empty.
fn clamp_selection(selection: Option<usize>, rows: usize) -> Option<usize> {
    match selection {
        Some(row) if row >= rows => rows.checked_sub(1),
        other => other,
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
    /// A packed coin amount, drawn by [`Board::coin_draw`].
    Coin(u32),
    /// A fighter's health, drawn by the health column's own callback.
    Health(u16),
    Empty,
}

/// The draw callback's decision for a coin cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoinDraw {
    /// The value zero is the text [`NO_RANK`] (a dash) at this x.
    Dash { x: i32 },
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
    /// The weapon check's mark on the reward panel.
    WeaponMark,
    /// The tournament prize item.
    Prize,
    /// A season reward item slot, zero through three.
    Reward(usize),
    /// The message box.
    Message,
    /// The tournament status line.
    Status,
    /// The Enter, Begin or Continue button.
    EnterButton,
    /// The "Entered n/m" label.
    EnteredLabel,
    /// The label above the company table.
    AvailableLabel,
    /// The Close button.
    CloseButton,
    /// The End Day button.
    EndDayButton,
    /// The tournament's Forfeit button.
    ForfeitButton,
    /// The season's reward picture.
    SeasonPicture,
    /// The standard tab's "Entry:" caption.
    EntryCaption,
    /// The standard tab's "Prize:" caption.
    PrizeCaption,
    /// The tournament tab's "Prize" caption.
    TournamentPrizeCaption,
    /// The season tab's "Prize for this season" caption.
    SeasonCaption,
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
    /// A table's column header: its header cell and its title's pen (the
    /// glyphs' left and bottom edge).
    Column {
        slot: TableSlot,
        index: usize,
        rect: Rect,
        pen: (i32, i32),
        title: &'static str,
    },
    /// A table cell: its rectangle under the header and its text's pen,
    /// which an owner-drawn column's callback also gets.
    Cell {
        slot: TableSlot,
        row: usize,
        column: usize,
        rect: Rect,
        pen: (i32, i32),
        value: CellValue,
        disabled: bool,
    },
    /// A coin cell's drawing: the dash at its offset, or the coin icons with
    /// their denomination counts. Follows the cell it belongs to.
    Coin {
        slot: TableSlot,
        row: usize,
        column: usize,
        rect: Rect,
        /// The cell's pen: the callback's x and its text line.
        pen: (i32, i32),
        draw: CoinDraw,
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
    for (index, column) in table.columns.iter().enumerate() {
        paint.push(BoardPaint::Column {
            slot: table.slot,
            index,
            rect: table.header(index),
            pen: table.title_pen(index),
            title: column.title,
        });
    }
    for (row, &source) in rows.iter().enumerate() {
        let Some((values, disabled)) = cells(source) else {
            continue;
        };
        for index in 0..table.columns.len() {
            let value = values.get(index).cloned().unwrap_or(CellValue::Empty);
            let rect = table.cell(row, index);
            let pen = table.cell_pen(row, index);
            // A coin column is owner-drawn: its callback gets the cell's
            // pen x.
            let coin = match &value {
                CellValue::Coin(packed) => Some(Board::coin_draw(*packed, pen.0)),
                _ => None,
            };
            paint.push(BoardPaint::Cell {
                slot: table.slot,
                row,
                column: index,
                rect,
                pen,
                value,
                disabled,
            });
            if let Some(draw) = coin {
                paint.push(BoardPaint::Coin {
                    slot: table.slot,
                    row,
                    column: index,
                    rect,
                    pen,
                    draw,
                });
            }
        }
    }
}

/// One count as the original's single character: the low byte of the count
/// plus 0x30.
fn count_char(count: i64) -> char {
    char::from((count.wrapping_add(0x30) & 0xff) as u8)
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
    /// The Close button.
    Close,
    /// The End Day button.
    EndDay,
    /// The tournament's Forfeit button.
    Forfeit,
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
    /// The day ends and the arena rolls the next one.
    EndDay,
    /// The company forfeits its tournament place.
    Forfeit,
    /// The form closes.
    Close,
}

/// The first model's close bookkeeping, kept for its callers: the saved
/// tab and the three lists' selections less one, the five item slots
/// released when set, the season picture, the base close and the global
/// call. The board itself does this in [`Board::close`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchBoardShowInput {
    pub seed: i32,
    pub columns: [i32; 3],
    pub panels: [i32; 5],
    pub pointer: bool,
    pub base: bool,
    pub global: bool,
}

/// What [`show_match_board`] did.
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

/// The first model's close bookkeeping (see [`MatchBoardShowInput`]); the
/// board's own show is [`Board::show`].
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

/// The board's destroy: the base destructor and the free on positive
/// flags, with the fighter pool released in between.
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

    /// A board on a weekday with the Duel at Inept selected.
    fn board() -> Board {
        Board::new(true, 3, fighters())
    }

    #[test]
    fn the_tables_draw_where_the_table_view_draws() {
        use crate::form::ClientRect;
        use crate::gui_font::RAYFORM_10;
        use crate::table_view::{TableColumn, TableRow, TableView};
        for unit in [4, 5, 6] {
            let mut board = Board::new(false, 3, fighters());
            board.unit = unit;
            assert!(board.switch_tab(BoardTab::Events).is_some());
            let table = board.table_layout(TableSlot::Events);
            let c = table.client();
            let mut view = TableView::new(
                4,
                ClientRect {
                    left: 0,
                    top: 0,
                    right: c.w,
                    bottom: c.h,
                },
            );
            view.columns = table
                .columns
                .iter()
                .map(|column| TableColumn {
                    title: column.title.as_bytes().to_vec(),
                    width: column.width,
                    ..Default::default()
                })
                .collect();
            if unit != 4 {
                view.rescale(4, unit);
            }
            view.rows = (0..6)
                .map(|_| TableRow {
                    flags: 0,
                    cells: vec![Vec::new(); 5],
                })
                .collect();
            let draw = view.draw(&RAYFORM_10, 28);
            let paint = board.paint();
            let titles: Vec<(i32, i32)> = paint
                .iter()
                .filter_map(|item| match item {
                    BoardPaint::Column {
                        slot: TableSlot::Events,
                        pen,
                        ..
                    } => Some(*pen),
                    _ => None,
                })
                .collect();
            let expected: Vec<(i32, i32)> = draw
                .titles
                .iter()
                .map(|&(_, (x, y))| (c.x + x, c.y + y))
                .collect();
            assert_eq!(titles, expected, "unit {unit}");
            let cells: Vec<(usize, usize, (i32, i32))> = paint
                .iter()
                .filter_map(|item| match item {
                    BoardPaint::Cell {
                        slot: TableSlot::Events,
                        row,
                        column,
                        pen,
                        ..
                    } => Some((*row, *column, *pen)),
                    _ => None,
                })
                .collect();
            assert_eq!(cells.len(), draw.cells.len());
            for &(row, column, (x, y), _) in &draw.cells {
                let want = ((row - 1) as usize, column as usize, (c.x + x, c.y + y));
                assert!(cells.contains(&want), "unit {unit}: {want:?}");
            }
        }
    }

    #[test]
    fn a_tables_rows_start_under_its_header() {
        let layout = layout(false);
        let table = layout.match_table;
        // The client is inside the two-pixel border: the header is its
        // top six units, the first row the next six.
        assert_eq!(
            table.header(0),
            Rect {
                x: 2,
                y: 58,
                w: 192,
                h: 24
            }
        );
        assert_eq!(
            table.cell(0, 0),
            Rect {
                x: 2,
                y: 82,
                w: 192,
                h: 24
            }
        );
        assert_eq!(table.title_pen(0), (6, 78));
        assert_eq!(table.cell_pen(0, 0), (6, 102));
        assert_eq!(table.cell_pen(1, 0), (6, 126));
        let roster = layout.roster_table;
        assert_eq!(roster.title_pen(1), (2 + 4 + 140, 350 + 20));
        assert_eq!(roster.cell_pen(0, 2), (2 + 4 + 208, 350 + 44));
        // Rows hit as the table view finds them: nothing in the header.
        assert_eq!(table.row_at(58 + 19), None);
        assert_eq!(table.row_at(58 + 20), None);
        assert_eq!(table.row_at(82), Some(0));
        assert_eq!(table.row_at(105), Some(0));
        assert_eq!(table.row_at(106), Some(1));
    }

    #[test]
    fn the_board_rescales_to_the_live_unit() {
        let layout = layout_at(false, 6);
        assert_eq!((layout.form.w, layout.form.h), (904, 982));
        // A child view keeps its border: x and y over 4 times 6, the
        // client's width and height the same.
        assert_eq!(
            layout.match_table.rect,
            Rect {
                x: 0,
                y: 84,
                w: 292,
                h: 268
            }
        );
        assert_eq!(
            layout.roster_table.rect,
            Rect {
                x: 0,
                y: 522,
                w: 436,
                h: 364
            }
        );
        assert_eq!(layout.lineup_table.rect.x, 444);
        assert_eq!(layout.lineup_table.unit, 6);
        // The columns keep their share of the unit.
        let widths: Vec<i32> = (0..5)
            .map(|c| layout.events_table.column_width(c))
            .collect();
        assert_eq!(widths, [210, 150, 150, 168, 156]);
        assert_eq!(
            layout.available_label,
            Rect {
                x: 174,
                y: 483,
                w: 192,
                h: 42
            }
        );
        // The tournament refresh sets the lineup in units of the live
        // unit.
        let mut board = Board::new(false, 3, fighters());
        board.unit = 6;
        board.refresh_tournament();
        assert_eq!(board.lineup_height(), 61 * 6);
        let mut compact = Board::new(true, 3, fighters());
        compact.refresh_tournament();
        assert_eq!(compact.lineup_height(), 224);
    }

    #[test]
    fn the_headers_clear_their_rows_and_the_labels_above() {
        for unit in [4, 5, 6, 8] {
            let font = crate::gui_font::gui_level(crate::gui_font::level_for_unit(unit)).main;
            let layout = layout_at(false, unit);
            for table in [
                layout.match_table,
                layout.events_table,
                layout.roster_table,
                layout.lineup_table,
            ] {
                let title = table.title_pen(0);
                let first = table.cell_pen(0, 0);
                assert_eq!(first.1 - title.1, 6 * unit);
                assert!(title.1 < table.cell(0, 0).y, "unit {unit}");
                assert!(first.1 - font.ascent >= title.1, "unit {unit}");
            }
            // "Available" and "Entered n/m" (labels centred down their
            // rectangle) end above the fighter tables' header glyphs.
            for label in [layout.available_label, layout.entered_label] {
                let baseline = label.y + (label.h + font.line_height + 2) / 2;
                for table in [layout.roster_table, layout.lineup_table] {
                    let top = table.title_pen(0).1 - font.ascent;
                    assert!(baseline <= top, "unit {unit}: {baseline} > {top}");
                }
            }
        }
    }

    /// A board whose selected match has this kind, floor and side size.
    fn board_for(kind: u32, floor: i32, size: i32) -> Board {
        let mut board = board();
        board.selected.kind = kind;
        board.selected.min_rank = floor;
        board.selected.team_size = size;
        board.rebuild_lineups();
        board
    }

    #[test]
    fn ranks_halve_the_six_skills() {
        assert_eq!(rank_of(&[0, 0, 0, 0, 0, 0]), 0);
        assert_eq!(rank_of(&[1, 0, 0, 0, 0, 0]), 0);
        assert_eq!(rank_of(&[2, 0, 0, 0, 0, 0]), 1);
        assert_eq!(rank_of(&[5, 5, 5, 5, 5, 5]), 5);
        assert_eq!(rank_of(&[u32::MAX; 6]), 5);
        assert_eq!(rank_name(-1), "-");
        assert_eq!(rank_name(0), "Inept");
        assert_eq!(rank_name(5), "Master");
        // Past Master the original's table holds the dash and staff labels.
        assert_eq!(rank_name(6), "-");
        assert_eq!(rank_name(7), "Merchant");
        assert_eq!(rank_name(9), "Physician");
        assert_eq!(rank_name(10), "+");
        assert_eq!(rank_name(11), "-");
    }

    #[test]
    fn the_layout_is_pinned() {
        let wide = layout(false);
        let rect = |x, y, w, h| Rect { x, y, w, h };
        assert_eq!(wide.form, rect(0, 0, 604, 656));
        assert_eq!(wide.match_table.rect, rect(0, 56, 196, 180));
        assert_eq!(wide.rank_table.rect, rect(200, 56, 192, 180));
        assert_eq!(wide.tier_table.rect, rect(396, 56, 192, 180));
        assert_eq!(wide.events_table.rect, rect(0, 56, 588, 180));
        assert_eq!(wide.winners_table.rect, rect(0, 56, 392, 180));
        assert_eq!(wide.roster_table.rect, rect(0, 348, 292, 244));
        assert_eq!(wide.lineup_table.rect, rect(296, 348, 292, 244));
        assert_eq!(wide.season_table.rect, rect(32, 60, 280, 132));
        assert_eq!(wide.message, rect(40, 276, 520, 96));
        assert_eq!(wide.picture, rect(396, 56, 192, 180));
        assert_eq!(wide.reward_panel, rect(296, 472, 292, 120));
        let widths =
            |table: &TableLayout| table.columns.iter().map(|c| c.width).collect::<Vec<_>>();
        assert_eq!(widths(&wide.events_table), [140, 100, 100, 114, 106]);
        assert_eq!(widths(&wide.winners_table), [120, 100, 120]);
        assert_eq!(widths(&wide.roster_table), [140, 68, 132]);
        assert_eq!(widths(&wide.season_table), [160]);

        let compact = layout(true);
        assert_eq!(compact.form.h, 636);
        assert_eq!(compact.roster_table.rect.h, 224);
        assert_eq!(compact.lineup_table.rect.h, 224);
        assert_eq!(compact.reward_panel.h, 100);
    }

    #[test]
    fn the_template_records_carry_the_captions_and_states() {
        let captions: Vec<_> = TEMPLATE_RECORDS.iter().filter_map(|r| r.text).collect();
        assert_eq!(
            captions,
            [
                "Standard",
                "Specials",
                "Tournament",
                "Season",
                "Entry:",
                "Prize:",
                "Available",
                "Entered 1/1",
                "Prize",
                "End Day",
                "Close",
                "Enter",
                "Forfeit",
                "Prize for this season",
            ]
        );
        assert_eq!(TEMPLATE_CONTROLS[control::CLOSE], control::SHOWN);
        assert_eq!(TEMPLATE_CONTROLS[control::ENTER], control::HIDDEN);
        assert_eq!(TEMPLATE_CONTROLS[control::SEASON_PICTURE], control::SHOWN);
        assert_eq!(form_spec(true).size, (604, 636));
    }

    #[test]
    fn the_form_template_places_the_tabs_and_the_bottom_row() {
        let wide = layout(false);
        let rect = |x, y, w, h| Rect { x, y, w, h };
        // The template's tabs: x 20, 162, 300, 440, of which the snap to
        // a multiple of four takes 162 down to 160 (40.5 quarters, ties to
        // even); y is row 5 (20 pixels) lifted by half of 28 less 8.
        assert_eq!(
            wide.tabs,
            [
                rect(20, 14, 128, 28),
                rect(160, 14, 128, 28),
                rect(300, 14, 128, 28),
                rect(440, 14, 128, 28),
            ]
        );
        // The bottom row: row 2, bottom-anchored, sits (2 + 8) * 4 plus
        // half of 28 less 8 = 46 pixels above the client's bottom edge
        // (652: the form's 656 less its border of 2 top and bottom).
        assert_eq!(wide.end_day_button, rect(20, 606, 128, 28));
        assert_eq!(wide.close_button, rect(160, 606, 128, 28));
        assert_eq!(wide.enter_button, rect(300, 606, 128, 28));
        assert_eq!(wide.forfeit_button, rect(440, 606, 128, 28));
        // The status text: a label of the class's 20 pixels, 42 above the
        // edge.
        assert_eq!(wide.status_label, rect(300, 610, 160, 20));
        // The labels above the two fighter tables: row 82 is 328, lifted
        // by 6; the season picture is taller than 32, so lifted by 4.
        assert_eq!(wide.available_label, rect(116, 322, 128, 28));
        assert_eq!(wide.entered_label, rect(408, 322, 128, 28));
        assert_eq!(wide.season_picture, rect(372, 28, 160, 600));
        // The captions take the label class's height (20, lifted by 2);
        // the two standard captions keep the template's zero width.
        assert_eq!(wide.entry_caption, rect(200, 246, 0, 20));
        assert_eq!(wide.prize_caption, rect(360, 246, 0, 20));
        assert_eq!(wide.tournament_prize_caption, rect(412, 62, 160, 20));
        assert_eq!(wide.season_caption, rect(32, 214, 280, 20));
        // The item slots take the item icon class's 160 pixels, taller
        // than 32, so lifted by 4.
        assert_eq!(wide.prize_slot, rect(412, 76, 160, 160));
        assert_eq!(wide.reward_slots[0], rect(12, 236, 160, 160));
        assert_eq!(wide.reward_slots[1], rect(172, 236, 160, 160));
        assert_eq!(wide.reward_slots[2], rect(12, 396, 160, 160));
        assert_eq!(wide.reward_slots[3], rect(172, 396, 160, 160));
        // A coin control follows its caption by the anchor's gap of 8, on
        // the caption's row (a 24-pixel coin lifts by 4).
        let t = template_rects(false);
        assert_eq!(t[control::STANDARD_FEE], rect(208, 244, 128, 24));

        let compact = layout(true);
        assert_eq!(compact.end_day_button, rect(20, 586, 128, 28));
        assert_eq!(compact.enter_button, rect(300, 586, 128, 28));
        assert_eq!(compact.available_label, wide.available_label);
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
        assert_eq!(season_index(49), 1);
        assert_eq!(season_index(50), 2);
        assert_eq!(season_index(112), 0);
        assert_eq!(season_weeks_elapsed(0), 0);
        assert_eq!(season_weeks_elapsed(1), 1);
        assert_eq!(season_weeks_elapsed(7), 1);
        assert_eq!(season_weeks_elapsed(8), 2);
        assert_eq!(season_weeks_elapsed(21), 3);
        assert_eq!(season_weeks_elapsed(22), 0);
        assert_eq!(season_picture_name(0), "seasonrwrda.rfi");
        assert_eq!(season_picture_name(22), "seasonrwrdb.rfi");
        assert_eq!(season_reward_order([1, 2, 3, 4]), [1, 3, 2, 4]);
        assert!(matches_left(2));
        assert!(!matches_left(3));
        assert!(matches_left(-1));
    }

    #[test]
    fn the_captain_floor_is_two_ranks_lower() {
        assert!(rank_passes(1, 3, 3, false));
        assert!(!rank_passes(1, 3, 2, false));
        assert!(rank_passes(9, 3, 1, false));
        assert!(!rank_passes(9, 3, 0, false));
        assert!(!rank_passes(9, 3, 1, true));
        assert!(rank_passes(9, 3, 3, true));
        assert!(rank_passes(9, 1, -1, false));
        // The 64-bit compare does not wrap at the bottom of the range.
        assert!(rank_passes(9, i32::MIN, i32::MIN, false));
    }

    #[test]
    fn coins_pack_as_the_original_stores_them() {
        assert_eq!(coin_value(0x0003_0205), 3 * 144 + 2 * 12 + 5);
        assert_eq!(pack_coins(461), 0x0003_0205);
        assert_eq!(pack_coins(36), 0x0300);
        // The gold count keeps 16 bits; the remainder's bytes wrap.
        assert_eq!(pack_coins(144 * 65_536 + 1), 1);
        assert_eq!(scale_coins(3, 3), 9);
        assert_eq!(scale_coins(0x100, 3), 0x300);
        assert_eq!(scale_coins(6, 5), 0x0206);
    }

    #[test]
    fn the_standard_tab_scales_the_match_by_the_rank() {
        let mut board = board();
        assert_eq!(board.selected.name, "Duel");
        assert_eq!(board.standard_fee, 3);
        assert_eq!(board.standard_prize, 0x100);
        board.rank = 2;
        board.tier = 5;
        board.refresh_standard();
        // The tier clamps to the rank; the rows above the rank are greyed.
        assert_eq!(board.tier, 2);
        assert_eq!(
            board.tier_rows(),
            [
                ("Inept", false),
                ("Aspirant", false),
                ("Novice", false),
                ("Adept", true),
                ("Expert", true),
                ("Master", true),
            ]
        );
        assert_eq!(board.rank_floor(), 2);
        assert_eq!(board.reported_tier(), Some(2));
        assert_eq!((board.selected.rank, board.selected.tier), (2, 2));
        // Three copper and one silver, times three.
        assert_eq!(board.standard_fee, 9);
        assert_eq!(board.standard_prize, 0x300);

        // Pugilism at Expert stores no tier.
        assert_eq!(
            board.handle(BoardInput::SelectMatch(5)),
            BoardAction::Changed
        );
        assert_eq!(
            board.handle(BoardInput::SelectRank(4)),
            BoardAction::Changed
        );
        assert_eq!(board.reported_tier(), None);
        assert_eq!(board.selected.tier, -1);
        assert_eq!(board.standard_fee, 5);
        assert_eq!(board.standard_prize, 0x0206);

        // Doubles takes two a side; Reserve lifts the one-leader rule.
        assert_eq!(
            board.handle(BoardInput::SelectMatch(1)),
            BoardAction::Changed
        );
        assert_eq!(board.team_size(), 2);
        assert!(!board.free_lineup());
        assert_eq!(
            board.handle(BoardInput::SelectMatch(4)),
            BoardAction::Changed
        );
        assert!(board.free_lineup());
        assert_eq!(board.team_size(), 3);
    }

    #[test]
    fn a_rank_change_moves_the_tier_with_it() {
        let mut board = board();
        assert_eq!(
            board.handle(BoardInput::SelectRank(3)),
            BoardAction::Changed
        );
        assert_eq!((board.rank, board.tier), (3, 3));
        assert_eq!(
            board.handle(BoardInput::SelectTier(1)),
            BoardAction::Changed
        );
        assert_eq!(board.tier, 1);
        assert_eq!(
            board.handle(BoardInput::SelectTier(5)),
            BoardAction::Changed
        );
        assert_eq!(board.tier, 3);
        assert_eq!(board.handle(BoardInput::SelectTier(6)), BoardAction::None);
    }

    #[test]
    fn the_message_follows_the_days_matches() {
        let mut board = board();
        assert_eq!(board.standard_message, "");
        board.matches_played = 3;
        board.refresh_standard();
        assert_eq!(board.standard_message, NO_MORE_MATCHES);
        assert_eq!(board.enter_state(), control::DISABLED);
        board.refresh_events();
        assert_eq!(board.standard_message, NO_MORE_MATCHES);
        board.refresh_tournament();
        assert_eq!(board.standard_message, "");
    }

    #[test]
    fn the_lineup_autofills_by_rank_and_class() {
        let board = board();
        assert_eq!(board.team, vec![1]);
        assert_eq!(board.available, vec![0, 2, 3, 4]);
        assert_eq!(board.lineup, vec![1, 2, 3, 4, i32::MAX, 0]);

        let board = board_for(1, 2, 2);
        assert_eq!(board.team, vec![1, 4]);
        assert_eq!(board.available, vec![0, 2, 3]);
        assert_eq!(board.entered_label(), "Entered 2/2");
    }

    #[test]
    fn the_player_counts_as_the_one_leader() {
        // The player comes last in the order, after Borin, a leader.
        let board = board_for(1, 0, 6);
        assert_eq!(board.team, vec![1, 2, 3]);
        assert_eq!(board.available, vec![0, 4]);
        let mut free = board_for(1, 0, 6);
        free.selected.flags = FLAG_FREE_LINEUP;
        free.rebuild_lineups();
        assert_eq!(free.team, vec![1, 2, 3, 4, 0]);
        assert!(free.available.is_empty());
    }

    #[test]
    fn a_full_team_benches_its_last_fighter() {
        let mut board = board();
        assert!(board.double_click_available(0));
        assert_eq!(board.team, vec![0]);
        assert_eq!(board.available, vec![2, 3, 4, 1]);
        assert_eq!(board.lineup, vec![0, 1, 2, 3, 4, i32::MAX]);

        // A bench leaves the original's order target unset: the newcomer
        // goes first in the order, not to its lineup row.
        let mut board = board_for(1, 0, 2);
        assert_eq!(board.team, vec![1, 2]);
        assert!(board.double_click_available(0));
        assert_eq!(board.team, vec![1, 0]);
        assert_eq!(board.available, vec![3, 4, 2]);
        assert_eq!(board.lineup, vec![0, 1, 2, 3, 4, i32::MAX]);
        board.rebuild_lineups();
        assert_eq!(board.team, vec![0, 1]);
    }

    #[test]
    fn a_join_takes_its_lineup_row_in_the_order() {
        let mut board = board_for(1, 0, 3);
        assert_eq!(board.team, vec![1, 2, 3]);
        assert!(board.move_to_available(2));
        assert_eq!(board.team, vec![1, 2]);
        assert_eq!(board.available, vec![0, 3, 4]);
        // Cael joins again at row 2: the order puts it third.
        assert!(board.move_to_team(1, None));
        assert_eq!(board.team, vec![1, 2, 3]);
        assert_eq!(board.lineup, vec![1, 2, 3, 4, i32::MAX, 0]);
        assert_eq!(board.available, vec![0, 4]);
        // A drop on row 0 swaps Dara with Anna and moves her first in the
        // order.
        assert!(board.move_to_team(1, Some(0)));
        assert_eq!(board.team, vec![4, 2, 3]);
        assert_eq!(board.available, vec![0, 1]);
        assert_eq!(board.lineup, vec![4, 1, 2, 3, i32::MAX, 0]);
    }

    #[test]
    fn a_short_rank_cannot_be_double_clicked_in() {
        let mut board = board_for(1, 4, 1);
        assert_eq!(board.team, vec![4]);
        assert_eq!(board.available, vec![0, 1, 2, 3]);
        assert!(!board.double_click_available(2));
        assert_eq!(board.team, vec![4]);
        assert!(board.can_drop(4));
        assert!(!board.can_drop(2));
        assert_eq!(
            board.handle(BoardInput::DropOnTeam { from: 2, to: None }),
            BoardAction::None
        );
    }

    #[test]
    fn the_captain_puts_a_qualified_fighter_first() {
        let mut board = board_for(CAPTAIN_EVENT, 3, 3);
        board.team = vec![2, 4];
        board.available = vec![0, 1, 3, 5];
        assert!(board.move_to_team(0, None));
        assert_eq!(board.team, vec![4, 2, 0]);
        assert_eq!(board.lineup, vec![1, 2, 0, 3, 4, i32::MAX]);
    }

    #[test]
    fn an_unranked_captain_newcomer_cannot_replace_a_ranked_teammate() {
        // One ranked teammate (Dara) of three.
        let mut board = board_for(CAPTAIN_EVENT, 3, 3);
        board.team = vec![2, 1, 4];
        board.available = vec![0, 3, 5];
        // Cael is under the floor: he may not take Dara's row.
        assert!(!board.move_to_team(1, Some(2)));
        assert_eq!(board.team, vec![2, 1, 4]);
        // He may take Borin's, and Dara then moves first.
        assert!(board.move_to_team(1, Some(0)));
        assert_eq!(board.team, vec![4, 1, 3]);
        assert_eq!(board.available, vec![0, 2, 5]);

        // The player reaches the floor, so the rule does not apply to him.
        let mut board = board_for(CAPTAIN_EVENT, 3, 3);
        board.team = vec![2, 1, 4];
        board.available = vec![0, 3, 5];
        assert!(board.move_to_team(0, Some(2)));
        assert_eq!(board.team, vec![0, 1, 2]);
        assert_eq!(board.available, vec![4, 3, 5]);
    }

    #[test]
    fn a_marked_fighter_replaces_its_marked_teammate() {
        let mut board = board_for(1, 0, 3);
        board.fighters[0].marked = true;
        board.fighters[2].marked = true;
        board.team = vec![2, 1, 4];
        board.available = vec![0, 3, 5];
        // Even a drop on another row lands on the marked teammate's.
        assert!(board.move_to_team(0, Some(2)));
        assert_eq!(board.team, vec![0, 1, 4]);
        assert_eq!(board.available, vec![2, 3, 5]);
        assert_eq!(board.lineup, vec![0, 1, 2, 3, 4, i32::MAX]);

        // The swap checks the marked teammate's rank on a Captain event.
        let mut board = board_for(CAPTAIN_EVENT, 3, 3);
        board.fighters[3].marked = true;
        board.fighters[4].marked = true;
        board.team = vec![2, 1, 4];
        board.available = vec![0, 3, 5];
        assert!(!board.move_to_team(1, None));

        // A free lineup skips the marked rule.
        let mut board = board_for(1, 0, 3);
        board.selected.flags = FLAG_FREE_LINEUP;
        board.fighters[0].marked = true;
        board.fighters[2].marked = true;
        board.team = vec![2, 1];
        board.available = vec![0, 3, 5];
        assert!(board.move_to_team(0, None));
        assert_eq!(board.team, vec![2, 1, 0]);
    }

    #[test]
    fn moving_back_inserts_by_slot_order() {
        let mut board = board_for(1, 0, 2);
        assert_eq!(board.team, vec![1, 2]);
        assert_eq!(board.available, vec![0, 3, 4]);
        board.team_selection = Some(1);
        assert!(board.move_to_available(1));
        assert_eq!(board.team, vec![1]);
        assert_eq!(board.team_selection, Some(0));
        assert_eq!(board.available, vec![0, 2, 3, 4]);
        assert!(board.move_to_available(0));
        assert_eq!(board.team_selection, None);
        assert_eq!(board.available, vec![0, 1, 2, 3, 4]);
        assert!(!board.move_to_available(0));
    }

    #[test]
    fn removal_keeps_the_lineup_order_and_can_return() {
        let mut board = board();
        assert!(board.move_to_available(0));
        assert!(board.team.is_empty());
        assert_eq!(board.lineup, vec![1, 2, 3, 4, i32::MAX, 0]);
        board.refresh_standard();
        assert_eq!(board.team, vec![1]);
    }

    #[test]
    fn the_entered_label_writes_one_character() {
        let mut board = board();
        board.selected.team_size = 9;
        board.team = vec![0, 1, 2];
        assert_eq!(board.entered_label(), "Entered 3/9");
        board.team = (0..10).collect();
        assert_eq!(board.entered_label(), "Entered :/9");
        board.selected.team_size = -1;
        assert_eq!(board.entered_label(), "Entered :/\u{2f}");
    }

    #[test]
    fn the_enter_button_follows_the_team_and_the_day() {
        let mut board = board();
        assert_eq!(board.enter_state(), control::SHOWN);
        let mut short = board_for(1, 4, 2);
        assert_eq!(short.team, vec![4]);
        assert_eq!(short.enter_state(), control::DISABLED);
        short.matches_played = 3;
        short.rebuild_lineups();
        assert_eq!(short.enter_state(), control::DISABLED);
        assert!(short.switch_tab(BoardTab::Season).is_some());
        assert_eq!(short.enter_state(), control::HIDDEN);

        // Leaving the tournament tab re-enables the button; the next
        // rebuild disables it again when the day's matches are spent.
        board.matches_played = 3;
        assert!(board.switch_tab(BoardTab::Tournament).is_some());
        assert_eq!(board.enter_state(), control::HIDDEN);
        assert!(board.switch_tab(BoardTab::Events).is_some());
        assert_eq!(board.enter_state(), control::DISABLED);
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
        assert_eq!(board.tournament_status().text, TOURNAMENT_IN_PROGRESS);

        board.entries[2].participants = vec![Participant {
            slot: 1,
            value: 7,
            weapon: 0,
        }];
        let begin = board.tournament_status();
        assert_eq!(begin.text, "");
        assert_eq!(begin.button_label, "Begin");
        assert_eq!(begin.enter, control::SHOWN);
        assert_eq!(begin.secondary, control::SHOWN);
        assert_eq!(begin.text_state, control::HIDDEN);

        board.entries[0].winner = 1;
        assert_eq!(board.tournament_status().button_label, "Continue");

        board.entries[2].winner = 2;
        let settled = board.tournament_status();
        assert_eq!(settled.text, NO_MORE_EVENTS);
        assert_eq!(settled.enter, control::HIDDEN);
        assert_eq!(settled.text_state, control::SHOWN);

        // The last event's winner ends the tournament.
        board.entries[5].winner = 3;
        assert_eq!(board.tournament_status().text, TOURNAMENT_ENDED);
        board.entries[5].winner = -1;
        assert_eq!(board.tournament_status().text, NO_MORE_EVENTS);
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
        board.rivals = (2..=20).map(|id| format!("Company {}", id)).collect();
        assert_eq!(board.season_winner(0), "Company 20");
    }

    #[test]
    fn the_event_cells_show_coins_ranks_and_the_challenger_dash() {
        let mut board = board();
        board.events[0] = MatchRecord {
            name: "Challenger".to_string(),
            kind: CHALLENGER_EVENT,
            rank: 3,
            tier: 2,
            fee: 0x0c03,
            prize: 0x0102_0304,
            ..MatchRecord::default()
        };
        board.events[1] = MatchRecord {
            name: "Brawl".to_string(),
            kind: 12,
            rank: 2,
            tier: -1,
            fee: 12,
            ..MatchRecord::default()
        };
        board.events[2] = MatchRecord {
            kind: 1,
            rank: 7,
            tier: 9,
            ..MatchRecord::default()
        };
        assert_eq!(
            board.event_cells(0),
            Some([
                CellValue::Text("Challenger".to_string()),
                CellValue::Text("Adept".to_string()),
                CellValue::Text("-".to_string()),
                CellValue::Coin(0x0c03),
                CellValue::Coin(0x0102_0304),
            ])
        );
        assert_eq!(
            board.event_cells(1),
            Some([
                CellValue::Text("Brawl".to_string()),
                CellValue::Text("Novice".to_string()),
                CellValue::Text("Inept".to_string()),
                CellValue::Coin(12),
                CellValue::Coin(0),
            ])
        );
        let cells = board.event_cells(2).unwrap();
        assert_eq!(cells[1], CellValue::Text("Merchant".to_string()));
        assert_eq!(cells[2], CellValue::Text("Physician".to_string()));
        assert_eq!(Board::coin_draw(0, 10), CoinDraw::Dash { x: 14 });
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
    fn the_tournament_winners_go_through_the_company_table() {
        let mut board = board();
        board.company_name = "Ours".to_string();
        board.rivals = (2..=20).map(|id| format!("Company {}", id)).collect();
        // A winner from 2 reads entry `winner - 2`; each entry is a company
        // id less one.
        board.tournament_companies = vec![5, 1];
        for (index, event) in board.tournament_events.iter_mut().enumerate() {
            event.name = format!("Event {}", index);
        }
        board.entries[0].winner = 1;
        board.entries[1].winner = 2;
        board.entries[2].winner = 3;
        board.entries[3].winner = 0;
        board.entries[4].winner = 4;
        board.entries[5].winner = -1;
        assert_eq!(
            board.winner_cells(0),
            Some([
                CellValue::Text("Event 0".to_string()),
                CellValue::Text("Inept".to_string()),
                CellValue::Text("Ours".to_string()),
            ])
        );
        let winner = |board: &Board, row| board.winner_cells(row).map(|cells| cells[2].clone());
        // Entry 5 is company 6; entry 1 is company 2.
        assert_eq!(winner(&board, 1), Some(CellValue::Text("Company 6".into())));
        assert_eq!(winner(&board, 2), Some(CellValue::Text("Company 2".into())));
        assert_eq!(winner(&board, 3), Some(CellValue::Text("-".into())));
        assert_eq!(winner(&board, 4), Some(CellValue::Text(String::new())));
        assert_eq!(winner(&board, 5), Some(CellValue::Text("-".into())));
    }

    #[test]
    fn the_events_refresh_moves_past_greyed_events_and_copies_the_record() {
        let mut board = board();
        for event in &mut board.events {
            event.flags = FLAG_FREE_LINEUP;
        }
        board.refresh_events();
        assert_eq!(board.event_row, 5);
        assert_eq!(board.event_selection, 0);
        board.events[2].flags = 0;
        board.events[2].kind = CAPTAIN_EVENT;
        board.events[2].min_rank = 3;
        board.events[2].team_size = 2;
        assert_eq!(
            board.handle(BoardInput::ClickEvent(0)),
            BoardAction::Changed
        );
        assert_eq!(board.event_row, 2);
        assert_eq!(board.event_selection, 2);
        assert_eq!(board.match_id(), CAPTAIN_EVENT);
        assert_eq!(board.rank_floor(), 3);
        // Anna, first, misses the full floor; Dara reaches it, and the
        // player would be a second leader.
        assert_eq!(board.team, vec![4]);
    }

    #[test]
    fn a_weapon_event_names_its_weapon_and_checks_the_first_entrant() {
        let mut board = board();
        board.weapon_tiers = vec![
            Vec::new(),
            vec![
                WeaponRow {
                    name: "Sword".to_string(),
                    variants: [3, 0, 0],
                },
                WeaponRow {
                    name: "Mace".to_string(),
                    variants: [7, 9, 0],
                },
            ],
        ];
        board.tournament_events[1] = MatchRecord {
            name: "Melee".to_string(),
            kind: 18,
            flags: FLAG_WEAPON,
            max_rank: 1,
            weapon_row: 2,
            ..MatchRecord::default()
        };
        board.fighters[1].event_values[1] = 5;
        board.fighters[1].event_weapons[1] = 9;
        assert!(board.switch_tab(BoardTab::Tournament).is_some());
        assert_eq!(board.weapon_mark(), None);
        assert_eq!(
            board.handle(BoardInput::ClickWinner(1)),
            BoardAction::Changed
        );
        assert_eq!(board.tournament_note, "Mace");
        assert_eq!(board.lineup_height(), WEAPON_LINEUP_HEIGHT);
        assert!(!board.reward_panel_locked());
        // Nobody is entered: the check starts from "accepted", so the
        // reward panel carries no mark.
        assert_eq!(board.weapon_mark(), Some(false));
        assert!(board.team.is_empty());

        // Anna joins and is recorded with her event value and weapon.
        let anna = board.available.iter().position(|&slot| slot == 1).unwrap();
        assert!(board.move_to_team(anna, None));
        assert_eq!(
            board.entries[1].participants,
            vec![Participant {
                slot: 1,
                value: 5,
                weapon: 9,
            }]
        );
        assert_eq!(board.weapon_mark(), Some(false));
        board.entries[1].participants[0].weapon = 8;
        assert_eq!(board.weapon_mark(), Some(true));
        // The first zero variant ends the row.
        board.entries[1].participants[0].weapon = 0;
        assert_eq!(board.weapon_mark(), Some(true));
        let paint = board.paint();
        assert!(paint.iter().any(|item| matches!(
            item,
            BoardPaint::Frame {
                widget: Widget::WeaponMark,
                ..
            }
        )));

        // Leaving the tab restores the lineup's height.
        assert!(board.switch_tab(BoardTab::Matches).is_some());
        assert_eq!(board.lineup_height(), 224);
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
        assert!(matches!(
            paint.get(1),
            Some(BoardPaint::Tab {
                index: 0,
                selected: true,
                ..
            })
        ));
        let tables = |paint: &[BoardPaint]| -> Vec<TableSlot> {
            paint
                .iter()
                .filter_map(|item| match item {
                    BoardPaint::Table { slot, .. } => Some(*slot),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(
            tables(&paint),
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

        let mut events = board.clone();
        assert!(events.switch_tab(BoardTab::Events).is_some());
        assert_eq!(
            tables(&events.paint()),
            vec![TableSlot::Events, TableSlot::Roster, TableSlot::Lineup]
        );

        // The picture and the reward panel were created before the
        // winners table.
        let mut tournament = board.clone();
        tournament.tournament_events[0].flags = FLAG_WEAPON;
        tournament.tournament_prize_item = Some(1);
        assert!(tournament.switch_tab(BoardTab::Tournament).is_some());
        let paint = tournament.paint();
        let order: Vec<&str> = paint
            .iter()
            .filter_map(|item| match item {
                BoardPaint::Frame {
                    widget: Widget::Picture,
                    ..
                } => Some("picture"),
                BoardPaint::Frame {
                    widget: Widget::RewardPanel,
                    ..
                } => Some("panel"),
                BoardPaint::Table {
                    slot: TableSlot::Winners,
                    ..
                } => Some("winners"),
                BoardPaint::Frame {
                    widget: Widget::Prize,
                    ..
                } => Some("prize"),
                BoardPaint::Table {
                    slot: TableSlot::Lineup,
                    ..
                } => Some("lineup"),
                _ => None,
            })
            .collect();
        assert_eq!(order, ["picture", "panel", "winners", "prize", "lineup"]);

        let mut season = board;
        assert!(season.switch_tab(BoardTab::Season).is_some());
        let paint = season.paint();
        assert_eq!(tables(&paint), vec![TableSlot::Season]);
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
                widget: Widget::EnteredLabel | Widget::EnterButton,
                ..
            }
        )));
    }

    #[test]
    fn the_event_table_cells_run_left_to_right_by_row() {
        let mut board = board();
        assert!(board.switch_tab(BoardTab::Events).is_some());
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
        // Inside the table view's two-pixel border.
        assert_eq!(cells[0], (0, 0, 2));
        assert_eq!(cells[1], (0, 1, 142));
        assert_eq!(cells[2], (0, 2, 242));
        assert_eq!(cells[3], (0, 3, 342));
        assert_eq!(cells[4], (0, 4, 456));
        assert_eq!(cells[5], (1, 0, 2));
    }

    #[test]
    fn the_coin_cells_carry_their_draw_decision() {
        let mut board = board();
        board.events[0] = MatchRecord {
            name: "Duel".to_string(),
            fee: 0x0002_0304,
            prize: 0,
            ..MatchRecord::default()
        };
        assert!(board.switch_tab(BoardTab::Events).is_some());
        let coins: Vec<(usize, usize, CoinDraw)> = board
            .paint()
            .iter()
            .filter_map(|item| match item {
                BoardPaint::Coin {
                    slot: TableSlot::Events,
                    row,
                    column,
                    draw,
                    ..
                } => Some((*row, *column, *draw)),
                _ => None,
            })
            .collect();
        // Six rows, an entry and a prize cell each.
        assert_eq!(coins.len(), 12);
        // Row 0's entry is drawn at the owner-drawn cell's x, a unit into
        // the fourth column (2 + 340 + 4); its zero prize is the dash four
        // pixels past the fifth's (2 + 454 + 4 + 4).
        assert_eq!(
            coins[0],
            (
                0,
                3,
                CoinDraw::Coins {
                    x: 346,
                    copper: 4,
                    silver: 3,
                    gold: 2,
                }
            )
        );
        assert_eq!(coins[1], (0, 4, CoinDraw::Dash { x: 464 }));
        // Row 1's entry is zero too, so the dash sits four pixels on.
        assert_eq!(coins[2], (1, 3, CoinDraw::Dash { x: 350 }));
        // The coin item follows its cell.
        let paint = board.paint();
        let at = paint
            .iter()
            .position(|item| matches!(item, BoardPaint::Coin { row: 0, .. }))
            .unwrap();
        assert!(matches!(
            paint.get(at - 1),
            Some(BoardPaint::Cell {
                row: 0,
                column: 3,
                ..
            })
        ));
    }

    #[test]
    fn the_hit_test_follows_the_layout_and_the_tab() {
        let mut board = board();
        assert_eq!(board.hit_test((30, 20)), BoardHit::Tab(0));
        assert_eq!(board.hit_test((200, 20)), BoardHit::Tab(1));
        // The second tab's template x of 162 snaps to 160.
        assert_eq!(board.hit_test((150, 20)), BoardHit::Outside);
        // The header (the client's top six units, from y 58) takes no
        // row; the first row starts at 82.
        assert_eq!(board.hit_test((10, 70)), BoardHit::Outside);
        assert_eq!(
            board.hit_test((10, 82)),
            BoardHit::Table {
                slot: TableSlot::Match,
                row: 0,
                column: 0
            }
        );
        assert_eq!(
            board.hit_test((400, 110)),
            BoardHit::Table {
                slot: TableSlot::Tier,
                row: 1,
                column: 0
            }
        );
        assert_eq!(board.hit_test((10_000, 10_000)), BoardHit::Outside);
        // The test board is the compact form: its bottom row is 46 pixels
        // above the 636-pixel bottom edge.
        assert_eq!(board.hit_test((350, 600)), BoardHit::Enter);
        assert_eq!(board.hit_test((200, 600)), BoardHit::Close);
        assert_eq!(board.handle(BoardInput::SelectTab(1)), BoardAction::Changed);
        // A column reaches its right edge: client x 454 is still the
        // fourth.
        assert_eq!(
            board.hit_test((456, 90)),
            BoardHit::Table {
                slot: TableSlot::Events,
                row: 0,
                column: 3
            }
        );
        assert_eq!(
            board.hit_test((457, 90)),
            BoardHit::Table {
                slot: TableSlot::Events,
                row: 0,
                column: 4
            }
        );
        assert_eq!(board.handle(BoardInput::SelectTab(3)), BoardAction::Changed);
        assert_eq!(board.hit_test((350, 620)), BoardHit::Outside);
    }

    #[test]
    fn a_click_selects_and_confirms() {
        let mut board = board();
        assert_eq!(board.click((200, 20)), BoardAction::Changed);
        assert_eq!(board.tab, BoardTab::Events);
        assert_eq!(board.click((30, 20)), BoardAction::Changed);
        assert_eq!(board.tab, BoardTab::Matches);
        assert_eq!(board.click((10, 56 + 2 + 3 * 24 + 5)), BoardAction::Changed);
        assert_eq!(board.match_type, 2);
        assert_eq!(board.selected.name, "Skirmish");
        assert_eq!(
            board.click((10, 348 + 2 + 2 * 24 + 5)),
            BoardAction::Changed
        );
        assert_eq!(board.available_selection, Some(1));
        assert_eq!(board.click((300, 348 + 2 + 24 + 5)), BoardAction::Changed);
        assert_eq!(board.team_selection, Some(0));
        // Skirmish takes three: Anna, Borin and Cael fill it.
        assert_eq!(board.click((350, 600)), BoardAction::EnterMatch);
        // The arena, not the board, counts the match.
        assert_eq!(board.matches_played, 0);
        assert_eq!(board.click((10_000, 10_000)), BoardAction::None);
    }

    #[test]
    fn the_show_switches_to_the_tournament_and_hides_the_old_tab() {
        let entrant = vec![Participant {
            slot: 1,
            value: 4,
            weapon: 0,
        }];
        let mut board = board();
        board.day = 14;
        board.entries[0].participants = entrant.clone();
        let show = board.show();
        assert!(show.tournament_day);
        assert!(show.has_entrants);
        assert_eq!(show.tab, BoardTab::Tournament);
        assert!(show.forced_tournament);
        assert!(control::selected(board.controls[control::TAB_TOURNAMENT]));
        assert!(!control::selected(board.controls[control::TAB_MATCHES]));
        assert_eq!(board.controls[control::TAB_MATCHES], control::DISABLED);
        // The forced switch keeps the current tab, so the standard tab it
        // leaves is hidden on the way.
        assert_eq!(board.controls[control::STANDARD_FEE], control::HIDDEN);
        assert_eq!(board.switch_tab(BoardTab::Season), None);

        let mut quiet = Board::new(true, 7, fighters());
        let show = quiet.show();
        assert_eq!(show.tab, BoardTab::Matches);
        assert!(!show.forced_tournament);
        assert_eq!(quiet.controls[control::STANDARD_FEE], control::SHOWN);

        let mut other = Board::new(true, 14, fighters());
        other.entries[0].participants = entrant;
        let view = other.switch_tab(BoardTab::Season);
        assert_eq!(view.map(|view| view.tab), Some(BoardTab::Tournament));
        // A normal tab switch still hides the tab it leaves.
        let mut leaving = board;
        leaving.entries[0].participants.clear();
        leaving.tab = BoardTab::Matches;
        leaving.apply_tab(None, BoardTab::Matches);
        assert!(leaving.switch_tab(BoardTab::Events).is_some());
        assert_eq!(leaving.controls[control::STANDARD_FEE], control::HIDDEN);
    }

    #[test]
    fn the_day_end_control_follows_the_match_count() {
        let mut board = board();
        assert_eq!(board.controls[control::DAY_END], control::DISABLED);
        board.matches_played = 1;
        assert!(board.switch_tab(BoardTab::Events).is_some());
        assert_eq!(board.controls[control::DAY_END], control::SHOWN);

        // On the tournament day the entries decide once anyone is entered:
        // with an entered event the played count no longer matters.
        board.day = 7;
        assert!(board.switch_tab(BoardTab::Matches).is_some());
        assert_eq!(board.controls[control::DAY_END], control::SHOWN);
        board.entries[2].participants = vec![Participant {
            slot: 1,
            value: 0,
            weapon: 0,
        }];
        assert!(board.switch_tab(BoardTab::Matches).is_some());
        assert_eq!(board.controls[control::DAY_END], control::DISABLED);
        board.entries[2].winner = 1;
        let tab = board.tab;
        board.apply_tab(None, tab);
        assert_eq!(board.controls[control::DAY_END], control::SHOWN);
        // A second entered event without a winner disables it again.
        board.entries[4].participants = vec![Participant {
            slot: 2,
            value: 0,
            weapon: 0,
        }];
        let tab = board.tab;
        board.apply_tab(None, tab);
        assert_eq!(board.controls[control::DAY_END], control::DISABLED);
        // On the tournament day with no entrants the count decides.
        board.entries[2].participants.clear();
        board.entries[4].participants.clear();
        board.matches_played = 0;
        let tab = board.tab;
        board.apply_tab(None, tab);
        assert_eq!(board.controls[control::DAY_END], control::DISABLED);
    }

    #[test]
    fn the_tournament_tab_locks_assignment() {
        let mut board = board();
        board.day = 7;
        board.entries[0].participants = vec![Participant {
            slot: 1,
            value: 0,
            weapon: 0,
        }];
        board.show();
        assert_eq!(board.tab, BoardTab::Tournament);
        assert_eq!(board.team, vec![1]);
        assert!(board.assignment_locked());
        assert!(!board.move_to_team(0, None));
        assert!(!board.move_to_available(0));
        assert!(!board.can_drop(0));
        board.day = 6;
        assert!(!board.assignment_locked());
    }

    #[test]
    fn the_tournament_tab_takes_its_reward_item() {
        let mut board = board();
        board.tournament_prize_item = Some(77);
        assert!(board.switch_tab(BoardTab::Tournament).is_some());
        assert_eq!(board.tournament_reward, Some(77));
        assert!(board.switch_tab(BoardTab::Events).is_some());
        assert_eq!(board.tournament_reward, Some(77));
        board.show();
        assert_eq!(board.tournament_reward, None);
    }

    #[test]
    fn greyed_tier_and_event_rows_paint_disabled() {
        let mut board = board();
        board.rank = 1;
        board.refresh_standard();
        let tier = |board: &Board, wanted| {
            board.paint().iter().find_map(|item| match item {
                BoardPaint::Cell {
                    slot: TableSlot::Tier,
                    row,
                    disabled,
                    ..
                } if *row == wanted => Some(*disabled),
                _ => None,
            })
        };
        assert_eq!(tier(&board, 1), Some(false));
        assert_eq!(tier(&board, 2), Some(true));
        board.events[0].flags = FLAG_FREE_LINEUP;
        assert!(board.switch_tab(BoardTab::Events).is_some());
        let greyed = board.paint().iter().find_map(|item| match item {
            BoardPaint::Cell {
                slot: TableSlot::Events,
                column: 0,
                row: 0,
                disabled,
                ..
            } => Some(*disabled),
            _ => None,
        });
        assert_eq!(greyed, Some(true));
    }

    #[test]
    fn the_template_captions_follow_their_controls() {
        assert_eq!(TABS, ["Standard", "Specials", "Tournament", "Season"]);
        let mut board = board();
        // The standard tab shows its entry and prize captions.
        let captions = |board: &Board| -> Vec<(&'static str, String)> {
            board
                .paint()
                .into_iter()
                .filter_map(|item| match item {
                    BoardPaint::Label {
                        widget: Widget::EntryCaption,
                        text,
                        ..
                    } => Some(("entry", text)),
                    BoardPaint::Label {
                        widget: Widget::PrizeCaption,
                        text,
                        ..
                    } => Some(("prize", text)),
                    BoardPaint::Label {
                        widget: Widget::TournamentPrizeCaption,
                        text,
                        ..
                    } => Some(("tournament", text)),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(
            captions(&board),
            vec![("entry", "Entry:".into()), ("prize", "Prize:".into())]
        );
        assert!(board.switch_tab(BoardTab::Tournament).is_some());
        assert_eq!(captions(&board), vec![("tournament", "Prize".into())]);
        assert!(board.switch_tab(BoardTab::Season).is_some());
        assert!(captions(&board).is_empty());
    }

    #[test]
    fn the_bottom_row_reports_its_buttons() {
        let mut board = board();
        let layout = board.layout();
        let centre = |rect: Rect| (rect.x + rect.w / 2, rect.y + rect.h / 2);
        assert_eq!(board.click(centre(layout.close_button)), BoardAction::Close);
        // End Day follows its control state: hidden until a match is played
        // (or the tournament day's entries settle).
        assert_eq!(
            board.hit_test(centre(layout.end_day_button)),
            BoardHit::Outside
        );
        board.matches_played = 1;
        let tab = board.tab;
        board.apply_tab(None, tab);
        assert_eq!(
            board.click(centre(layout.end_day_button)),
            BoardAction::EndDay
        );
        // Forfeit is the tournament tab's secondary button and stays hidden
        // elsewhere.
        assert_eq!(
            board.hit_test(centre(layout.forfeit_button)),
            BoardHit::Outside
        );
        board.day = 7;
        board.entries[0].participants = vec![Participant {
            slot: 1,
            value: 0,
            weapon: 0,
        }];
        board.show();
        assert_eq!(board.tab, BoardTab::Tournament);
        assert_eq!(
            board.click(centre(layout.forfeit_button)),
            BoardAction::Forfeit
        );
    }

    #[test]
    fn a_reopened_board_restores_the_tab_and_the_lists() {
        let mut board = board();
        board.match_type = 2;
        board.rank = 3;
        board.tier = 1;
        board.tab = BoardTab::Events;
        let close = board.close();
        let mut fresh = Board::new(true, board.day, fighters());
        fresh.restore(&close);
        assert_eq!(fresh.tab, BoardTab::Events);
        assert_eq!((fresh.match_type, fresh.rank, fresh.tier), (2, 3, 1));
        // The standard tab then refreshes with the restored selections.
        assert!(fresh.switch_tab(BoardTab::Matches).is_some());
        assert_eq!(fresh.selected.name, "Skirmish");
        assert_eq!(fresh.rank_floor(), 3);
        assert_eq!(fresh.tier, 1);
        assert_eq!(fresh.selected.tier, 1);

        // Selections past the six-row lists clamp; the reopen stays total.
        let mut far = CloseState {
            tab: BoardTab::Season,
            match_type: usize::MAX,
            rank: usize::MAX,
            tier: usize::MAX,
            tournament_reward: None,
            season_items: [None; 4],
            picture: String::new(),
        };
        let mut fresh = Board::new(true, 0, fighters());
        fresh.restore(&far);
        assert_eq!(fresh.tab, BoardTab::Season);
        assert_eq!((fresh.match_type, fresh.rank, fresh.tier), (5, 5, 5));
        far.rank = 0;
        far.tier = 0;
        far.match_type = 0;
        fresh.restore(&far);
        assert_eq!(fresh.controls[control::ENTER], control::HIDDEN);
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
        assert_eq!((close.match_type, close.rank, close.tier), (2, 3, 1));
        assert_eq!(close.tournament_reward, Some(44));
        assert_eq!(close.picture, "seasonrwrda.rfi");
        assert!(board.team.is_empty());
    }

    #[test]
    fn odd_input_and_empty_rosters_do_not_panic() {
        let mut board = Board::new(true, u32::MAX, Vec::new());
        for input in [
            BoardInput::SelectTab(usize::MAX),
            BoardInput::SelectMatch(usize::MAX),
            BoardInput::SelectRank(usize::MAX),
            BoardInput::SelectTier(usize::MAX),
            BoardInput::ClickEvent(usize::MAX),
            BoardInput::ClickWinner(usize::MAX),
            BoardInput::DoubleClickAvailable(usize::MAX),
            BoardInput::DoubleClickTeam(usize::MAX),
            BoardInput::DropOnTeam {
                from: usize::MAX,
                to: Some(usize::MAX),
            },
            BoardInput::DropOnAvailable(usize::MAX),
        ] {
            assert_eq!(board.handle(input), BoardAction::None);
        }
        assert!(board.team.is_empty());
        assert!(board.available.is_empty());
        assert!(!board.move_to_team(0, Some(0)));
        assert_eq!(board.entered_label(), "Entered 0/1");
        assert_eq!(board.hit_test((i32::MAX, i32::MIN)), BoardHit::Outside);
        assert!(!board.paint().is_empty());

        // Out-of-range records stay total; a tier past the entries table
        // reads as no entrants, which clears the mark.
        let mut board = board_for(1, i32::MIN, i32::MIN);
        board.selected.max_rank = i32::MAX;
        board.selected.weapon_row = i32::MIN;
        board.selected.flags = FLAG_WEAPON;
        assert_eq!(board.weapon_mark(), Some(false));
        board.entries[0].participants = vec![Participant {
            slot: usize::MAX,
            value: 0,
            weapon: 0,
        }];
        board.tab = BoardTab::Tournament;
        board.rebuild_lineups();
        assert!(board.team.is_empty());
        board.lineup = vec![-5, i32::MAX, 99, -1, 0, 7];
        board.tab = BoardTab::Matches;
        board.rebuild_lineups();
        assert_eq!(board.team, vec![0]);
        assert_eq!(scale_coins(u32::MAX, i32::MIN), pack_coins(0x8000_0000));
        assert_eq!(season_weeks_elapsed(u32::MAX), 1);
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
        assert!(destroyed.base_destroy && destroyed.list_released && destroyed.freed);
        let kept = destroy_match_board(0);
        assert!(!kept.base_destroy && kept.list_released && kept.base && !kept.freed);
    }
}
