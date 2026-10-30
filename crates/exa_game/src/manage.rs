//! The company's windows: Manage Company, the arsenal form a fighter or a
//! candidate is looked at in, Company Status and Company Records.
//!
//! Manage Company lists the manager and the staff, each with a rank label
//! (the profession for a merchant, trainer or physician, else the rank
//! name, marked "+" while the fighter is ready to train), and a health bar.
//! Its refresh writes the day or week title, the money, the total daily
//! upkeep, today's earned and spent, End Day, then the report's log,
//! training and rank lines, hire lines and tournament line. Recruit is
//! disabled at eight fighters and Hire at ten employees. A report line
//! opens what it names: the tournament line the board's tournament tab, a
//! hire-list line the first candidate of that profession, a fighter's line
//! the roster (on its first row).
//!
//! The arsenal form has four pages: the roster (0), the hire list (1), a
//! specialist's record (2) and an event's lineup (3). Previous is disabled
//! on the first row, Next on the last, and Dismiss on the manager; a step
//! loads the row's record and is committed after the form closes the frame
//! (a dismissal confirmed on the way removes the employee and moves the
//! selection one up). The rank slider names the rank of `value × 5`
//! rounded to even; the points line reads "Points: (cost + 1)/2 / limit",
//! the limit twice the loadout limit halved. A record's title is its name
//! and rank (or profession) in brackets; a candidate's page adds the
//! profession's line and text.
//!
//! Company Status shows the company's name, an unknown renown and
//! reputation, good morale, the staff with the manager, and the fighters
//! (the manager first) of rank above Expert as its masters, or "None".
//!
//! Evidence: the retail manage, arsenal form, status and records routines
//! and their control tables read from `Exanima.exe`
//! (`context/re/arena-manage.md`).

use crate::arena::Roster;
use crate::arena_log::ArenaRecord;
use crate::career::Profession;
use crate::report::{self, RANKS, ReportFighter, ReportLine};

/// The manage window's title.
pub const MANAGE_TITLE: &str = "Manage Company";
/// The manage window class's instance size.
pub const MANAGE_FORM_SIZE: i32 = 0x178;

/// What closing Manage Company does: clears the arena hub's busy flag, calls
/// a routine that only returns, forgets the window's global pointer and runs
/// the base form's close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManageClose {
    pub hub_busy: bool,
    pub window: Option<u64>,
    pub base: bool,
}

/// The state the close leaves behind.
pub fn close_manage() -> ManageClose {
    ManageClose {
        hub_busy: false,
        window: None,
        base: true,
    }
}
/// The manage window's labels and buttons, in its table's order.
pub const MANAGE_CONTROLS: [&str; 11] = [
    "Roster",
    "Report",
    "Manage",
    "Arsenal",
    "Recruit",
    "Hire",
    "Owned",
    "Upkeep",
    "Earned today",
    "Spent today",
    "End Day",
];
/// The roster table's four columns and their widths, from the window's
/// constructor: Name (owner-drawn in its role's ink), Rank, an untitled
/// 16-pixel gap and Health (the owner-drawn gauge; the last column runs to
/// the client's edge).
pub const ROSTER_COLUMNS: [(&str, i32); 4] =
    [("Name", 0x84), ("Rank", 0x50), ("", 0x10), ("Health", 0x84)];
/// The roster's health column's left: the three columns before it.
pub const HEALTH_COLUMN_X: i32 = 0x84 + 0x50 + 0x10;
/// The arsenal form's title.
pub const ARSENAL_TITLE: &str = "ARSENAL";
/// The status window's title.
pub const STATUS_TITLE: &str = "Company Status";
/// The records window's title.
pub const RECORDS_TITLE: &str = "Company Records";
/// The dismiss dialog's prompt, before the name, and its end.
pub const DISMISS_PROMPT: &str = "Are you sure you want to dismiss ";
pub const DISMISS_END: &str = "?";
/// The mark beside a fighter ready to train.
pub const READY_MARK: &str = "+";
/// A control flag: disabled.
pub const DISABLED: u32 = 0x10;
/// A control flag: hidden and disabled.
pub const HIDDEN: u32 = 0x110;
/// Fighters at which Recruit is disabled.
pub const RECRUIT_CAP: u32 = 8;
/// Employees at which Hire is disabled.
pub const HIRE_CAP: usize = 10;
/// The arsenal form's item filter on a specialist's page, and on others.
pub const SPECIALIST_FILTER: u32 = 0x200;
pub const FIGHTER_FILTER: u32 = 0x400;

/// What a roster row shows of its record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowRole {
    /// A merchant, trainer or physician.
    Specialist(Profession),
    /// A fighter of a rank (0 Inept … 5 Master) and whether he is ready
    /// to train.
    Fighter { rank: usize, ready: bool },
}

/// A roster row's rank label and mark.
pub fn row_label(role: RowRole) -> (&'static str, &'static str) {
    match role {
        RowRole::Specialist(Profession::Merchant) => ("Merchant", ""),
        RowRole::Specialist(Profession::Trainer) => ("Trainer", ""),
        RowRole::Specialist(Profession::Physician) => ("Physician", ""),
        RowRole::Specialist(Profession::Combatant) | RowRole::Fighter { .. } => {
            let (rank, ready) = match role {
                RowRole::Fighter { rank, ready } => (rank, ready),
                _ => (0, false),
            };
            (
                RANKS.get(rank).copied().unwrap_or("-"),
                if ready { READY_MARK } else { "" },
            )
        }
    }
}

/// Recruit is enabled under eight fighters (the manager counts).
pub fn recruit_open(fighters: u32) -> bool {
    fighters < RECRUIT_CAP
}

/// Hire is enabled under ten employees.
pub fn hire_open(employees: usize) -> bool {
    employees < HIRE_CAP
}

/// Whose name a roster row draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameKind {
    Manager,
    /// A merchant, trainer or physician (a role word above a combatant's).
    Specialist,
    /// A hired combatant (not under the company's control).
    Hireling,
    /// A recruit, controlled in battle.
    Recruit,
}

/// A name's ink: red, green, blue and alpha, from the skin's alpha byte.
/// A recruit is drawn in the skin's own text colour (`None`).
pub fn name_ink(kind: NameKind, alpha: u8) -> Option<[u8; 4]> {
    match kind {
        NameKind::Manager => Some([0xff, 0xd2, 100, (alpha / 5).wrapping_mul(3)]),
        NameKind::Specialist => Some([0xdc, 0xb4, 0xa0, (alpha / 3).wrapping_mul(2)]),
        NameKind::Hireling => Some([0xa0, 0xb4, 0xf0, (alpha / 3).wrapping_mul(2)]),
        NameKind::Recruit => None,
    }
}

/// The roster table's three columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RosterColumn {
    Name,
    Rank,
    Health,
}

/// A cell's text and ink; `None` ink is the table's own text colour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RosterCellText {
    pub text: String,
    pub ink: Option<[u8; 4]>,
}

/// One drawn roster cell. The health column has no text: the original
/// paints a bar filled by [`health_fill`].
#[derive(Clone, Debug, PartialEq)]
pub enum RosterCell {
    Name(RosterCellText),
    Rank(RosterCellText),
    Health(f32),
}

/// Bytes of a record's name a roster cell reads.
pub const NAME_CELL_BYTES: usize = 15;

/// A name cut to the fifteen bytes a roster cell reads. Cuts on a
/// character boundary, as the game's names are ASCII.
pub fn roster_name(name: &str) -> String {
    let mut end = name.len().min(NAME_CELL_BYTES);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    name[..end].to_string()
}

/// The health bar's fill: the health word times one over 65535 in double
/// precision, narrowed once to single precision (a single-precision
/// product lands one step off for 512 of the 65536 words).
pub fn health_fill(health: u16) -> f32 {
    (f64::from(health) * (1.0 / 65535.0)) as f32
}

/// A roster cell: the name in its role's ink, the rank label followed by
/// the ready mark, or the health bar's fill.
pub fn roster_cell(
    column: RosterColumn,
    name: &str,
    kind: NameKind,
    role: RowRole,
    alpha: u8,
    health: u16,
) -> RosterCell {
    match column {
        RosterColumn::Name => RosterCell::Name(RosterCellText {
            text: roster_name(name),
            ink: name_ink(kind, alpha),
        }),
        RosterColumn::Rank => {
            let (label, mark) = row_label(role);
            RosterCell::Rank(RosterCellText {
                text: format!("{label}{mark}"),
                ink: None,
            })
        }
        RosterColumn::Health => RosterCell::Health(health_fill(health)),
    }
}

/// A roster row as the table draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct RosterRowText {
    pub name: RosterCellText,
    pub rank: RosterCellText,
    /// The health bar's fill, health over 65535.
    pub health: f32,
    /// The record's health word the gauge draw reads ([`health_marker`]).
    pub health_word: u16,
}

/// The three cells of a roster row.
pub fn roster_row(
    name: &str,
    kind: NameKind,
    role: RowRole,
    alpha: u8,
    health: u16,
) -> RosterRowText {
    let (label, mark) = row_label(role);
    RosterRowText {
        name: RosterCellText {
            text: roster_name(name),
            ink: name_ink(kind, alpha),
        },
        rank: RosterCellText {
            text: format!("{label}{mark}"),
            ink: None,
        },
        health: health_fill(health),
        health_word: health,
    }
}

/// A record's listed value: the square of one more than its rank times 12
/// (in 32-bit arithmetic), plus four fifths of the kit's copper rounded to
/// the nearest even integer. The original computes the four fifths in
/// double precision and converts with the processor's nearest-even mode.
pub fn record_value(kit: u32, rank: u32) -> u32 {
    let rank_term = rank
        .wrapping_add(1)
        .wrapping_mul(rank.wrapping_add(1))
        .wrapping_mul(12);
    let share = (f64::from(kit) * 0.8).round_ties_even();
    let share = if share.is_nan() || share >= 9.223_372_036_854_776e18 {
        i64::MIN
    } else {
        share as i64
    };
    rank_term.wrapping_add(share as u32)
}

/// The health share the fighter-info panel writes into its two gauges: the
/// record's word times the double 1/65535, stored as single
/// ([`crate::arena_match::health_bar`]).
pub fn panel_health(word: u16) -> f32 {
    crate::arena_match::health_bar(word)
}

/// The roster row's health gauge, as the GUI's gauge draw receives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HealthMarker {
    /// The gauge's left edge: the cell's left, unchanged.
    pub left: i32,
    /// The gauge's top: the cell's top less six units, plus half of five
    /// units (rounded down), plus one.
    pub top: i32,
    /// The gauge's length: the cell's width less one unit.
    pub length: i32,
    /// The gauge's thickness: two units.
    pub thickness: i32,
    /// Both fill fractions: the record's word times the double 1/65535.
    pub fill: f32,
}

/// The roster table's health gauge from the cell's left, top and width, the
/// GUI unit (4 at the base scale) and the record's word. At unit 4 a cell
/// at (x, y) of width w draws an 8-thick gauge at (x, y − 13), w − 4 long.
pub fn health_marker(left: i32, top: i32, width: i32, unit: i32, word: u16) -> HealthMarker {
    let unit64 = i64::from(unit);
    let half_five = ((unit64 * 7 - i64::from(unit.wrapping_mul(2))) as u64 >> 1) as i64;
    HealthMarker {
        left,
        top: (i64::from(top) - unit64 * 6 + half_five + 1) as i32,
        length: width.wrapping_sub(unit),
        thickness: unit.wrapping_mul(2),
        fill: panel_health(word),
    }
}

/// The fighter-info panel's portrait scale: the preview's height field
/// squared times 10, in single precision.
pub fn portrait_scale(height: f32) -> f32 {
    height * height * 10.0
}

/// The fighter-info panel's fields.
#[derive(Clone, Debug, PartialEq)]
pub struct FighterPanel {
    /// The record's title: its name, then its profession or rank.
    pub title: String,
    /// "Profession: …" (fighter edit pages only).
    pub profession_line: &'static str,
    /// The profession's description.
    pub profession_text: &'static str,
    /// The kit's value: `(rank + 1)² · 12 + round(0.8 · kit)`.
    pub value: u32,
    /// The record's wage.
    pub wage: u32,
    /// The two health gauges, from the record's condition words.
    pub health: [f32; 2],
    /// The preview's portrait scale, `height² · 10`.
    pub portrait: f32,
}

/// The fighter-info panel's values. The panel's edit mode decides whether
/// the profession line is shown; the caller draws it then.
pub fn fighter_panel(
    name: &str,
    role: Option<Profession>,
    rank: u32,
    kit: u32,
    wage: u32,
    condition: [u16; 2],
    portrait_height: f32,
) -> FighterPanel {
    FighterPanel {
        title: record_title(name, role, rank as usize),
        profession_line: profession_line(role),
        profession_text: profession_text(role),
        value: record_value(kit, rank),
        wage,
        health: [panel_health(condition[0]), panel_health(condition[1])],
        portrait: portrait_scale(portrait_height),
    }
}

/// The arsenal form's pages.
pub const PAGE_ROSTER: u32 = 0;
pub const PAGE_HIRE: u32 = 1;
pub const PAGE_SPECIALIST: u32 = 2;
pub const PAGE_EVENT: u32 = 3;

/// The record a row of the arsenal form names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowRecord {
    /// A hire-list candidate by place.
    Candidate(usize),
    Manager,
    /// An employee by place.
    Employee(usize),
}

/// The flags the arsenal form's Previous, Next and Dismiss buttons carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepButtons {
    pub previous: u32,
    pub next: u32,
    pub dismiss: u32,
}

/// The arsenal form's selection state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArsenalForm {
    /// [`PAGE_ROSTER`], [`PAGE_HIRE`], [`PAGE_SPECIALIST`] or [`PAGE_EVENT`].
    pub page: u32,
    /// The last row's index.
    pub last_row: i32,
    /// The row shown.
    pub selection: i32,
    /// The row a step asked for, applied when it is committed.
    pub pending: i32,
    /// The loadout tab shown (−1 for none) and the one kept for the
    /// fighter pages.
    pub tab: i32,
    pub kept_tab: i32,
    /// A dismissal was confirmed and waits for the commit.
    pub dismissing: bool,
    pub buttons: StepButtons,
    /// The arsenal shelf and box the form's view shows.
    pub shelf: u32,
    pub box_index: u32,
}

/// A shelf or box button pressed: the view goes back into the box it
/// showed and the new box is loaded, sorted first when it was marked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoxSwitch {
    pub stored: (u32, u32),
    pub loaded: (u32, u32),
    pub sorted: bool,
}

/// What the form does when it opens a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageView {
    /// The item flag the arsenal shelves are filtered by.
    pub filter: u32,
    /// The rank label, slider, points and encumbrance controls' flags.
    pub rank_controls: Option<u32>,
}

/// What a committed step does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Commit {
    /// The record shown before keeps the loadout it was given.
    pub keep_loadout: bool,
    /// The employee who was dismissed, by place.
    pub dismissed: Option<usize>,
}

impl ArsenalForm {
    /// A form opened on a page. The hire list starts on tab 5.
    pub fn open(page: u32) -> Self {
        let tab = if page == PAGE_HIRE { 5 } else { 0 };
        Self {
            page,
            last_row: 0,
            selection: 0,
            pending: 0,
            tab,
            kept_tab: tab,
            dismissing: false,
            buttons: StepButtons::default(),
            shelf: 0,
            box_index: 0,
        }
    }

    /// The row names: the hire list's candidates on the hire page, else
    /// the manager and the employees (none without a manager). The last
    /// row is the candidates' count less one on the hire page, else the
    /// employees' count.
    pub fn rows(
        &mut self,
        candidates: &[String],
        manager: Option<&str>,
        employees: &[String],
    ) -> Vec<String> {
        if self.page == PAGE_HIRE {
            self.last_row = candidates.len() as i32 - 1;
            return candidates.to_vec();
        }
        self.last_row = employees.len() as i32;
        let Some(manager) = manager else {
            return Vec::new();
        };
        let mut rows = Vec::with_capacity(employees.len() + 1);
        rows.push(manager.to_string());
        rows.extend(employees.iter().cloned());
        rows
    }

    fn clamp(&self, row: i32) -> i32 {
        if row > self.last_row {
            self.last_row
        } else if row < 0 {
            0
        } else {
            row
        }
    }

    /// Show a row: clamp it, then (but on an event's page) disable Next on
    /// the last row and Previous and Dismiss on the first; off the first
    /// row Previous is enabled and, on the roster and specialist pages,
    /// Dismiss too.
    pub fn select(&mut self, row: i32) {
        self.selection = self.clamp(row);
        if self.page == PAGE_EVENT {
            return;
        }
        self.buttons.next = if self.selection < self.last_row {
            0
        } else {
            DISABLED
        };
        if self.selection == 0 {
            self.buttons.previous = DISABLED;
            self.buttons.dismiss |= DISABLED;
            return;
        }
        self.buttons.previous = 0;
        if self.page & 1 == 0 {
            self.buttons.dismiss &= !DISABLED;
        }
    }

    /// The record of a row: a candidate on the hire page, the manager on
    /// row 0, else the employee one place up.
    pub fn record_of(&self, row: i32) -> RowRecord {
        if self.page == PAGE_HIRE {
            RowRecord::Candidate(row.max(0) as usize)
        } else if row < 1 {
            RowRecord::Manager
        } else {
            RowRecord::Employee(row as usize - 1)
        }
    }

    /// Step to a row: clamped, and nothing when it is the row shown. The
    /// row waits as pending; but on an event's page its record is loaded
    /// and the commit is scheduled (returned).
    pub fn step_to(&mut self, row: i32) -> Option<RowRecord> {
        let row = self.clamp(row);
        if row == self.selection {
            return None;
        }
        self.pending = row;
        if self.page == PAGE_EVENT {
            return None;
        }
        Some(self.record_of(row))
    }

    /// A click on a list row (counted from 1): a step unless it is the row
    /// shown.
    pub fn click_row(&mut self, row: i32) -> Option<RowRecord> {
        if i64::from(row) - 1 == i64::from(self.selection) {
            return None;
        }
        self.step_to(row - 1)
    }

    /// Previous: a step up from any row but the first.
    pub fn step_previous(&mut self) -> Option<RowRecord> {
        if 0 < self.selection {
            self.step_to(self.selection - 1)
        } else {
            None
        }
    }

    /// Next: a step down from any row but the last.
    pub fn step_next(&mut self) -> Option<RowRecord> {
        if self.selection < self.last_row {
            self.step_to(self.selection + 1)
        } else {
            None
        }
    }

    /// The dismiss dialog's Dismiss: the dismissal waits for the commit and
    /// the row above is loaded.
    pub fn confirm_dismiss(&mut self) -> RowRecord {
        self.dismissing = true;
        self.pending = self.selection - 1;
        if self.pending < 1 {
            RowRecord::Manager
        } else {
            RowRecord::Employee(self.pending as usize - 1)
        }
    }

    /// The scheduled commit: off the hire page the record shown keeps its
    /// loadout; a waiting dismissal removes the employee of the row shown
    /// (`rows` gives the new last row); then the pending row is shown.
    pub fn commit(&mut self, mut rows: impl FnMut(&mut Self)) -> Commit {
        let keep_loadout = self.page != PAGE_HIRE;
        let mut dismissed = None;
        if self.dismissing {
            dismissed = usize::try_from(self.selection - 1).ok();
            rows(self);
            self.dismissing = false;
        }
        self.select(self.pending);
        Commit {
            keep_loadout,
            dismissed,
        }
    }

    /// Open a page for a record: the event page keeps it, the hire page
    /// sets nothing, a fighter's record is the roster page and any other
    /// the specialist page.
    pub fn page_for(&self, role: Option<Profession>) -> Option<u32> {
        match self.page {
            PAGE_EVENT => Some(PAGE_EVENT),
            PAGE_HIRE => None,
            _ => Some(match role {
                Some(Profession::Combatant) | None => PAGE_ROSTER,
                Some(_) => PAGE_SPECIALIST,
            }),
        }
    }

    /// Set the page. A specialist's page filters by 0x200, hides the rank
    /// controls and shows no tab. Any other filters by 0x400, shows the
    /// rank controls (an event page leaves them) and returns to the kept
    /// tab.
    pub fn set_page(&mut self, page: u32) -> PageView {
        self.page = page;
        if page == PAGE_SPECIALIST {
            self.tab = -1;
            return PageView {
                filter: SPECIALIST_FILTER,
                rank_controls: Some(HIDDEN),
            };
        }
        self.tab = self.kept_tab;
        PageView {
            filter: FIGHTER_FILTER,
            rank_controls: (page != PAGE_EVENT).then_some(0),
        }
    }

    fn switch_box(&mut self, shelf: u32, box_index: u32, marked: &mut [[bool; 5]; 5]) -> BoxSwitch {
        let stored = (self.shelf, self.box_index);
        self.shelf = shelf;
        self.box_index = box_index;
        let mark = marked
            .get_mut(shelf as usize)
            .and_then(|row| row.get_mut(box_index as usize));
        let sorted = mark.is_some_and(std::mem::take);
        BoxSwitch {
            stored,
            loaded: (shelf, box_index),
            sorted,
        }
    }

    /// A shelf button (only once pressed): the view is stored and the same
    /// box of that shelf loaded. `marked` holds the boxes to sort once.
    pub fn choose_shelf(
        &mut self,
        pressed: bool,
        shelf: u32,
        marked: &mut [[bool; 5]; 5],
    ) -> Option<BoxSwitch> {
        pressed.then(|| self.switch_box(shelf, self.box_index, marked))
    }

    /// A box button (only once pressed): the view is stored and that box of
    /// the shelf loaded.
    pub fn choose_box(
        &mut self,
        pressed: bool,
        box_index: u32,
        marked: &mut [[bool; 5]; 5],
    ) -> Option<BoxSwitch> {
        pressed.then(|| self.switch_box(self.shelf, box_index, marked))
    }

    /// HIRE hires the record shown, at its fee, on the hire page only.
    pub fn hire_pressed(&self) -> bool {
        self.page == PAGE_HIRE
    }

    /// A finished slider move switches the tab: the kept and shown tab
    /// become the rank. The loadout shown before is stored on the roster
    /// page.
    pub fn switch_tab(&mut self, rank: i32) -> bool {
        let store = self.page == PAGE_ROSTER;
        self.kept_tab = rank;
        self.tab = rank;
        store
    }
}

/// The rank the slider names: its value times five, rounded to even in
/// 64 bits and cut to the low 32 (a NaN or a value out of range gives the
/// conversion's 0x8000000000000000, whose low half is 0).
pub fn slider_rank(value: f32) -> i32 {
    let scaled = value * 5.0;
    if scaled.is_nan() || scaled.abs() >= 2f32.powi(63) {
        return 0;
    }
    scaled.round_ties_even() as i64 as i32
}

/// The points a loadout tab shows: below tab 0 nothing is written and
/// the cost and limit become 0 and 100; else the stored limit is twice the
/// tab's loadout limit, and the line reads the cost plus one halved over
/// the limit halved.
pub fn points_line(cost: i32, tab: i32, limits: &[u32; 6]) -> (i32, u32, Option<String>) {
    if tab < 0 {
        return (0, 100, None);
    }
    let limit = limits
        .get(tab as usize)
        .copied()
        .unwrap_or(0)
        .wrapping_shl(1);
    let shown = (i64::from(cost) + 1) as u64 >> 1;
    (
        cost,
        limit,
        Some(format!("Points: {shown} / {}", limit >> 1)),
    )
}

/// A record's title: its name, then its profession for a merchant,
/// trainer or physician and its rank name for a fighter, in brackets.
pub fn record_title(name: &str, role: Option<Profession>, rank: usize) -> String {
    let label = match role {
        Some(Profession::Merchant) => "Merchant",
        Some(Profession::Trainer) => "Trainer",
        Some(Profession::Physician) => "Physician",
        _ => RANKS.get(rank).copied().unwrap_or("-"),
    };
    format!("{name} ({label})")
}

/// A candidate's profession line.
pub fn profession_line(role: Option<Profession>) -> &'static str {
    match role {
        Some(Profession::Trainer) => "Profession: Trainer",
        Some(Profession::Merchant) => "Profession: Merchant",
        Some(Profession::Physician) => "Profession: Physician",
        _ => "Profession: Combatant",
    }
}

/// A candidate's profession text.
pub fn profession_text(role: Option<Profession>) -> &'static str {
    match role {
        Some(Profession::Trainer) => {
            "Trainers increase the rate at which your fighters learn skills. They can also teach the techniques they know to your fighters when they are idle."
        }
        Some(Profession::Merchant) => {
            "Merchants provide an inventory of items for you to buy. They can help you find more specific items by providing stock of a particular type and rank."
        }
        Some(Profession::Physician) => {
            "Physicians improve the rate of healing of your fighters and can save fighters from death if the injuries sustained aren't too severe."
        }
        _ => "Combatants can fight in your stead, or by your side in team matches.",
    }
}

/// The record's skills panel is shown for a role up to a trainer's.
pub fn skills_shown(role: Option<Profession>) -> bool {
    !matches!(role, Some(Profession::Merchant | Profession::Physician))
}

/// The skills panel's fixed byte is set on the hire page or for a trainer
/// (an event page keeps it as it was). What the panel does with it is not
/// traced (guess: no technique can be picked).
pub fn skills_fixed(page: u32, role: Option<Profession>) -> Option<bool> {
    if page == PAGE_EVENT {
        return None;
    }
    Some(page == PAGE_HIRE || role == Some(Profession::Trainer))
}

/// An event's lineup page as the board opens it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineupOpen {
    /// The row: the event's first entrant's id.
    pub row: u32,
    /// The loadout tab: the event's place, the rank it is fought at.
    pub tab: i32,
    /// The first entrant's two weapon words.
    pub weapons: [u64; 2],
}

/// Open an event's lineup: only an event with an entrant opens.
pub fn open_lineup(rows: &Roster, event: usize) -> Option<LineupOpen> {
    let row = rows.get(event)?;
    if (row.count as i32) <= 0 {
        return None;
    }
    let entrant = row.entries[0];
    Some(LineupOpen {
        row: entrant.actor_id,
        tab: event as i32,
        weapons: entrant.weapon,
    })
}

/// Close an event's lineup: the record's two hand words go back to the
/// event's first entrant, the second word's low half (its item id) cleared.
pub fn close_lineup(rows: &mut Roster, event: usize, hands: [u64; 2]) {
    if let Some(row) = rows.get_mut(event) {
        row.entries[0].weapon = [hands[0], hands[1] & !0xffff_ffff];
    }
}

/// A report line's action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportAction {
    /// Open the match board on its tournament tab.
    TournamentBoard,
    /// Open the hire list on a candidate.
    HireList(usize),
    /// Open the roster. The original opens it on the first row (the
    /// manager's) and only remembers the line's fighter id.
    Roster {
        remembered: u32,
    },
    None,
}

/// The first candidate of a profession, or the first candidate.
pub fn first_candidate(listed: &[Profession], role: Profession) -> usize {
    listed.iter().position(|&p| p == role).unwrap_or(0)
}

/// What a click on a report line opens, by the line's code: the
/// tournament line the board's tournament tab, a hire-list line the first
/// candidate of its profession, a training or rank line the roster (on its
/// first row, remembering the fighter id in the code's top byte). Log lines
/// open nothing.
pub fn report_click(code: u32, listed: &[Profession]) -> ReportAction {
    match code & 0xff_0000 {
        0x01_0000 => ReportAction::TournamentBoard,
        0x11_0000 => ReportAction::HireList(first_candidate(listed, Profession::Merchant)),
        0x12_0000 => ReportAction::HireList(first_candidate(listed, Profession::Physician)),
        0x13_0000 => ReportAction::HireList(first_candidate(listed, Profession::Trainer)),
        0x14_0000 | 0x15_0000 => ReportAction::Roster {
            remembered: code >> 24,
        },
        _ => ReportAction::None,
    }
}

/// The status window's lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompanyStatus {
    pub name: String,
    pub renown: &'static str,
    pub reputation: &'static str,
    pub morale: &'static str,
    /// The staff with the manager.
    pub staff: u32,
    /// The masters' count, or "None" when there are none.
    pub masters: String,
}

/// Company Status: masters are the manager and the employees of a
/// fighter's role whose rank is above 4.
pub fn company_status(
    name: &str,
    manager_rank: u32,
    employees: &[(Profession, u32)],
) -> CompanyStatus {
    let mut masters = u32::from(4 < manager_rank);
    for &(role, rank) in employees {
        if role == Profession::Combatant && 4 < rank {
            masters += 1;
        }
    }
    CompanyStatus {
        name: name.to_string(),
        renown: "Unknown",
        reputation: "Unknown",
        morale: "Good",
        staff: employees.len() as u32 + 1,
        masters: if masters > 0 {
            masters.to_string()
        } else {
            "None".to_string()
        },
    }
}

/// A fighter's rank: half the sum of the six levels (an experience word
/// over 5000), at most Master.
pub fn rank_of(skill_xp: &[u16; 6]) -> usize {
    crate::arena_ids::fighter_rank(skill_xp) as usize
}

/// A record's daily upkeep: a merchant 2, a physician 12, a trainer the
/// sum of the six levels, and a fighter its rank plus one.
pub fn daily_upkeep(profession: Profession, skill_xp: &[u16; 6]) -> u32 {
    match profession {
        Profession::Merchant => crate::career::MERCHANT_WAGE,
        Profession::Physician => crate::career::PHYSICIAN_WAGE,
        Profession::Trainer => report::level_sum(skill_xp),
        Profession::Combatant => rank_of(skill_xp) as u32 + 1,
    }
}

/// The manage window's plain inputs: the day, the books, the fighter
/// count and whether End Day is open, and the skin's text alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ManageDay {
    pub day: u32,
    /// Company money, and what it earned and spent today.
    pub money: u32,
    pub earned: u32,
    pub spent: u32,
    /// Fighters the company has (the manager counts), for Recruit.
    pub fighters: u32,
    pub end_day: bool,
    pub alpha: u8,
}

/// The manager or an employee as the manage window reads the record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManageRecord {
    pub fighter: ReportFighter,
    pub profession: Profession,
    /// The health bar's fill, 0 to 65535 (full).
    pub health: u16,
    /// Under the company's control in battle (a recruit); such a name is
    /// drawn in the skin's own text colour.
    pub controlled: bool,
}

impl ManageRecord {
    /// The rank index, at most Master.
    pub fn rank(&self) -> usize {
        rank_of(&self.fighter.skill_xp)
    }

    /// Whether the record has a technique to train.
    pub fn ready(&self) -> bool {
        report::ready_to_train(&self.fighter.skill_xp, &self.fighter.techniques)
    }

    pub fn role(&self) -> RowRole {
        match self.profession {
            Profession::Combatant => RowRole::Fighter {
                rank: self.rank(),
                ready: self.ready(),
            },
            profession => RowRole::Specialist(profession),
        }
    }

    /// The ink a name takes: the manager's, a specialist's, a hireling's or
    /// a recruit's own colour.
    pub fn name_kind(&self, manager: bool) -> NameKind {
        if manager {
            NameKind::Manager
        } else if self.profession == Profession::Combatant {
            if self.controlled {
                NameKind::Recruit
            } else {
                NameKind::Hireling
            }
        } else {
            NameKind::Specialist
        }
    }

    pub fn upkeep(&self) -> u32 {
        daily_upkeep(self.profession, &self.fighter.skill_xp)
    }

    /// The row this record draws; the manager is the first row.
    pub fn row(&self, manager: bool, alpha: u8) -> RosterRowText {
        roster_row(
            &report::name_text(&self.fighter.name),
            self.name_kind(manager),
            self.role(),
            alpha,
            self.health,
        )
    }
}

/// What a Manage Company refresh writes, in the original's order: the day
/// title, the money, the total daily upkeep, today's earned and spent, End
/// Day, then the roster rows (the manager first) and the report's lines.
#[derive(Clone, Debug, PartialEq)]
pub struct ManageRefresh {
    pub title: String,
    pub money: u32,
    pub upkeep: u32,
    pub earned: u32,
    pub spent: u32,
    pub end_day: bool,
    pub recruit: bool,
    pub hire: bool,
    pub roster: Vec<RosterRowText>,
    pub report: Vec<ReportLine>,
}

/// Manage Company's refresh from plain records. The roster holds the
/// manager and every employee in staff order; the report holds the log's
/// lines, each ready fighter's training and rank lines (employees newest
/// first, then the manager), the hire-list counts and the tournament line.
/// Only a fighter (the combatant role) reaches the training lines.
pub fn manage_refresh(
    day: ManageDay,
    manager: &ManageRecord,
    employees: &[ManageRecord],
    log: &[ArenaRecord],
    listed: &[Profession],
    name_of: impl Fn(u8) -> Option<[u8; 16]>,
) -> ManageRefresh {
    let mut roster = Vec::with_capacity(employees.len() + 1);
    roster.push(manager.row(true, day.alpha));
    roster.extend(employees.iter().map(|e| e.row(false, day.alpha)));
    let upkeep = employees.iter().map(ManageRecord::upkeep).sum();
    let fighters: Vec<ReportFighter> = employees
        .iter()
        .filter(|e| e.profession == Profession::Combatant)
        .map(|e| e.fighter.clone())
        .collect();
    ManageRefresh {
        title: report::day_title(day.day),
        money: day.money,
        upkeep,
        earned: day.earned,
        spent: day.spent,
        end_day: day.end_day,
        recruit: recruit_open(day.fighters),
        hire: hire_open(employees.len()),
        roster,
        report: report::report_lines(log, &fighters, &manager.fighter, listed, day.day, name_of),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_name_specialists_and_mark_the_ready() {
        assert_eq!(
            row_label(RowRole::Specialist(Profession::Merchant)),
            ("Merchant", "")
        );
        assert_eq!(
            row_label(RowRole::Specialist(Profession::Physician)),
            ("Physician", "")
        );
        assert_eq!(
            row_label(RowRole::Fighter {
                rank: 2,
                ready: true
            }),
            ("Novice", "+")
        );
        assert_eq!(
            row_label(RowRole::Fighter {
                rank: 5,
                ready: false
            }),
            ("Master", "")
        );
    }

    #[test]
    fn recruit_and_hire_close_at_their_caps() {
        assert!(recruit_open(7));
        assert!(!recruit_open(8));
        assert!(hire_open(9));
        assert!(!hire_open(10));
    }

    #[test]
    fn names_take_their_ink_by_role() {
        assert_eq!(name_ink(NameKind::Manager, 255), Some([255, 210, 100, 153]));
        assert_eq!(
            name_ink(NameKind::Hireling, 255),
            Some([160, 180, 240, 170])
        );
        assert_eq!(
            name_ink(NameKind::Specialist, 100),
            Some([220, 180, 160, 66])
        );
        assert_eq!(name_ink(NameKind::Recruit, 255), None);
    }

    fn roster(employees: usize) -> ArsenalForm {
        let mut form = ArsenalForm::open(PAGE_ROSTER);
        let names: Vec<String> = (0..employees).map(|i| format!("E{i}")).collect();
        form.rows(&[], Some("Boss"), &names);
        form
    }

    #[test]
    fn the_roster_lists_the_manager_first() {
        let mut form = ArsenalForm::open(PAGE_ROSTER);
        let rows = form.rows(&[], Some("Boss"), &["A".into(), "B".into()]);
        assert_eq!(rows, ["Boss", "A", "B"]);
        assert_eq!(form.last_row, 2);
        assert!(form.rows(&[], None, &["A".into()]).is_empty());
        let mut hire = ArsenalForm::open(PAGE_HIRE);
        assert_eq!(hire.tab, 5);
        hire.rows(&["X".into(), "Y".into(), "Z".into()], Some("Boss"), &[]);
        assert_eq!(hire.last_row, 2);
    }

    #[test]
    fn buttons_follow_the_selection() {
        let mut form = roster(3);
        form.select(0);
        assert_eq!(
            form.buttons,
            StepButtons {
                previous: DISABLED,
                next: 0,
                dismiss: DISABLED
            }
        );
        form.select(7);
        assert_eq!(form.selection, 3);
        assert_eq!(
            form.buttons,
            StepButtons {
                previous: 0,
                next: DISABLED,
                dismiss: 0
            }
        );
        let mut hire = ArsenalForm::open(PAGE_HIRE);
        hire.rows(&["X".into(), "Y".into()], None, &[]);
        hire.buttons.dismiss = DISABLED;
        hire.select(1);
        assert_eq!(hire.buttons.dismiss, DISABLED);
        let mut event = ArsenalForm::open(PAGE_EVENT);
        event.select(-2);
        assert_eq!(event.selection, 0);
        assert_eq!(event.buttons, StepButtons::default());
    }

    #[test]
    fn steps_load_the_row_and_wait_for_the_commit() {
        let mut form = roster(3);
        form.select(1);
        assert_eq!(form.step_to(1), None);
        assert_eq!(form.step_next(), Some(RowRecord::Employee(1)));
        assert_eq!(form.pending, 2);
        assert_eq!(form.selection, 1);
        assert_eq!(form.step_previous(), Some(RowRecord::Manager));
        assert_eq!(form.click_row(2), None);
        assert_eq!(form.click_row(4), Some(RowRecord::Employee(2)));
        let commit = form.commit(|_| {});
        assert_eq!(
            commit,
            Commit {
                keep_loadout: true,
                dismissed: None
            }
        );
        assert_eq!(form.selection, 3);
        let mut hire = ArsenalForm::open(PAGE_HIRE);
        hire.rows(&["X".into(), "Y".into()], None, &[]);
        assert_eq!(hire.step_next(), Some(RowRecord::Candidate(1)));
        assert!(!hire.commit(|_| {}).keep_loadout);
    }

    #[test]
    fn a_confirmed_dismissal_moves_the_selection_up() {
        let mut form = roster(3);
        form.select(2);
        assert_eq!(form.confirm_dismiss(), RowRecord::Employee(0));
        let commit = form.commit(|form| form.last_row = 2);
        assert_eq!(commit.dismissed, Some(1));
        assert_eq!(form.selection, 1);
        assert!(!form.dismissing);
        form.select(1);
        assert_eq!(form.confirm_dismiss(), RowRecord::Manager);
    }

    #[test]
    fn pages_filter_the_shelves() {
        let mut form = roster(1);
        form.kept_tab = 3;
        assert_eq!(
            form.page_for(Some(Profession::Trainer)),
            Some(PAGE_SPECIALIST)
        );
        assert_eq!(form.page_for(None), Some(PAGE_ROSTER));
        assert_eq!(
            form.set_page(PAGE_SPECIALIST),
            PageView {
                filter: 0x200,
                rank_controls: Some(0x110)
            }
        );
        assert_eq!(form.tab, -1);
        assert_eq!(
            form.set_page(PAGE_ROSTER),
            PageView {
                filter: 0x400,
                rank_controls: Some(0)
            }
        );
        assert_eq!(form.tab, 3);
        assert_eq!(form.set_page(PAGE_EVENT).rank_controls, None);
        assert_eq!(ArsenalForm::open(PAGE_HIRE).page_for(None), None);
        assert!(!form.switch_tab(4));
        form.page = PAGE_ROSTER;
        assert!(form.switch_tab(2));
        assert_eq!((form.tab, form.kept_tab), (2, 2));
    }

    #[test]
    fn shelf_and_box_buttons_store_the_view_first() {
        let mut form = roster(1);
        let mut marked = [[false; 5]; 5];
        marked[3][0] = true;
        assert_eq!(form.choose_shelf(false, 3, &mut marked), None);
        assert_eq!(
            form.choose_shelf(true, 3, &mut marked),
            Some(BoxSwitch {
                stored: (0, 0),
                loaded: (3, 0),
                sorted: true
            })
        );
        assert!(!marked[3][0]);
        assert_eq!(
            form.choose_box(true, 4, &mut marked),
            Some(BoxSwitch {
                stored: (3, 0),
                loaded: (3, 4),
                sorted: false
            })
        );
        assert!(!form.hire_pressed());
        assert!(ArsenalForm::open(PAGE_HIRE).hire_pressed());
    }

    #[test]
    fn a_lineup_opens_on_its_entrant_and_keeps_one_weapon() {
        use crate::arena::{Entrant, RosterRow};
        let mut rows = Roster::default();
        rows[2] = RosterRow {
            count: 1,
            entries: [
                Entrant {
                    actor_id: 3,
                    weapon: [0x11, 0x22],
                },
                Entrant::default(),
            ],
            team_id: 0,
        };
        assert_eq!(open_lineup(&rows, 1), None);
        assert_eq!(
            open_lineup(&rows, 2),
            Some(LineupOpen {
                row: 3,
                tab: 2,
                weapons: [0x11, 0x22]
            })
        );
        close_lineup(&mut rows, 2, [0x5_0000_0007, 0x9_0000_0008]);
        assert_eq!(rows[2].entries[0].weapon, [0x5_0000_0007, 0x9_0000_0000]);
    }

    #[test]
    fn a_record_value_squares_the_rank_and_adds_four_fifths_of_the_kit() {
        assert_eq!(record_value(0, 0), 12);
        assert_eq!(record_value(7, 0), 12 + 6);
        assert_eq!(record_value(7, 2), 9 * 12 + 6);
        assert_eq!(record_value(58981, 5), 36 * 12 + 47185);
        // Banker's rounding: 0.8 of 5 is exactly 4, of 15 is 12.
        assert_eq!(record_value(5, 0), 16);
        assert_eq!(record_value(15, 0), 24);
        assert_eq!(panel_health(0).to_bits(), 0.0f32.to_bits());
        assert_eq!(panel_health(65535).to_bits(), 1.0f32.to_bits());
        // The double product rounds to the single nearest 1/65535.
        assert_eq!(
            panel_health(1).to_bits(),
            ((1.0f64 / 65535.0) as f32).to_bits()
        );
        assert_eq!(portrait_scale(1.0).to_bits(), 10.0f32.to_bits());
        assert_eq!(
            portrait_scale(1.1).to_bits(),
            (1.1f32 * 1.1 * 10.0).to_bits()
        );
    }

    #[test]
    fn the_roster_health_gauge_sits_at_the_cells_corner() {
        let marker = health_marker(100, 50, 60, 4, 65535);
        assert_eq!(
            (marker.left, marker.top, marker.length, marker.thickness),
            (100, 37, 56, 8)
        );
        assert_eq!(marker.fill.to_bits(), 1.0f32.to_bits());
        // Unit 3: half of fifteen rounds down to 7.
        let marker = health_marker(0, 0, 30, 3, 32768);
        assert_eq!((marker.top, marker.length, marker.thickness), (-10, 27, 6));
        assert_eq!(
            marker.fill.to_bits(),
            ((32768.0f64 * crate::arena_match::WORD_SHARE) as f32).to_bits()
        );
    }

    #[test]
    fn the_fighter_panel_takes_the_title_value_wage_and_gauges() {
        let panel = fighter_panel(
            "Bors",
            Some(Profession::Trainer),
            2,
            7,
            3,
            [0xFFFF, 0x8000],
            1.0,
        );
        assert_eq!(panel.title, "Bors (Trainer)");
        assert_eq!(panel.profession_line, "Profession: Trainer");
        assert!(panel.profession_text.starts_with("Trainers increase"));
        assert_eq!(panel.value, 9 * 12 + 6);
        assert_eq!(panel.wage, 3);
        assert_eq!(panel.health[0].to_bits(), 1.0f32.to_bits());
        assert_eq!(
            panel.health[1].to_bits(),
            crate::arena_match::health_bar(0x8000).to_bits()
        );
        assert_eq!(panel.portrait.to_bits(), 10.0f32.to_bits());
        let fighter = fighter_panel("Ash", None, 5, 0, 1, [0, 0], 0.5);
        assert_eq!(fighter.title, "Ash (Master)");
    }

    #[test]
    fn the_slider_rounds_its_rank_to_even() {
        assert_eq!(slider_rank(0.0), 0);
        assert_eq!(slider_rank(0.3), 2);
        assert_eq!(slider_rank(0.5), 2);
        assert_eq!(slider_rank(0.7), 4);
        assert_eq!(slider_rank(1.0), 5);
        assert_eq!(slider_rank(f32::NAN), 0);
    }

    #[test]
    fn the_points_line_halves_cost_and_limit() {
        let limits = [5, 10, 15, 20, 30, 40];
        assert_eq!(
            points_line(7, 0, &limits),
            (7, 10, Some("Points: 4 / 5".into()))
        );
        assert_eq!(
            points_line(0, 5, &limits),
            (0, 80, Some("Points: 0 / 40".into()))
        );
        assert_eq!(points_line(9, -1, &limits), (0, 100, None));
    }

    #[test]
    fn records_take_their_titles_and_texts() {
        assert_eq!(
            record_title("Bors", Some(Profession::Trainer), 3),
            "Bors (Trainer)"
        );
        assert_eq!(
            record_title("Bors", Some(Profession::Combatant), 3),
            "Bors (Adept)"
        );
        assert_eq!(profession_line(None), "Profession: Combatant");
        assert!(profession_text(Some(Profession::Physician)).starts_with("Physicians improve"));
        assert!(skills_shown(Some(Profession::Trainer)));
        assert!(!skills_shown(Some(Profession::Merchant)));
        assert_eq!(skills_fixed(PAGE_HIRE, None), Some(true));
        assert_eq!(
            skills_fixed(PAGE_ROSTER, Some(Profession::Trainer)),
            Some(true)
        );
        assert_eq!(skills_fixed(PAGE_ROSTER, None), Some(false));
        assert_eq!(skills_fixed(PAGE_EVENT, None), None);
    }

    #[test]
    fn report_lines_open_what_they_name() {
        let listed = [
            Profession::Combatant,
            Profession::Trainer,
            Profession::Merchant,
        ];
        assert_eq!(
            report_click(0x1_0000, &listed),
            ReportAction::TournamentBoard
        );
        assert_eq!(report_click(0x11_0000, &listed), ReportAction::HireList(2));
        assert_eq!(report_click(0x13_0000, &listed), ReportAction::HireList(1));
        assert_eq!(report_click(0x12_0000, &listed), ReportAction::HireList(0));
        assert_eq!(
            report_click(0x0314_0000, &listed),
            ReportAction::Roster { remembered: 3 }
        );
        assert_eq!(
            report_click(0x15_0000, &listed),
            ReportAction::Roster { remembered: 0 }
        );
        assert_eq!(report_click(0x2_0000, &listed), ReportAction::None);
    }

    #[test]
    fn status_counts_staff_and_masters() {
        let status = company_status(
            "The Jolly Ranchers",
            5,
            &[
                (Profession::Combatant, 5),
                (Profession::Trainer, 5),
                (Profession::Combatant, 4),
            ],
        );
        assert_eq!(status.staff, 4);
        assert_eq!(status.masters, "2");
        assert_eq!(status.morale, "Good");
        assert_eq!(company_status("A", 4, &[]).masters, "None");
    }

    fn name16(text: &str) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        bytes
    }

    fn fighter(name: &str, id: u8, skill_xp: [u16; 6]) -> ReportFighter {
        ReportFighter {
            name: name16(name),
            id,
            skill_xp,
            techniques: [[0; 6]; 6],
        }
    }

    fn record(
        name: &str,
        id: u8,
        profession: Profession,
        skill_xp: [u16; 6],
        health: u16,
        controlled: bool,
    ) -> ManageRecord {
        ManageRecord {
            fighter: fighter(name, id, skill_xp),
            profession,
            health,
            controlled,
        }
    }

    #[test]
    fn ranks_cap_at_master_and_set_the_daily_upkeep() {
        assert_eq!(rank_of(&[0; 6]), 0);
        assert_eq!(rank_of(&[10_000, 10_000, 0, 0, 0, 0]), 2);
        assert_eq!(rank_of(&[25_000, 25_000, 25_000, 0, 0, 0]), 5);
        assert_eq!(daily_upkeep(Profession::Merchant, &[25_000; 6]), 2);
        assert_eq!(daily_upkeep(Profession::Physician, &[25_000; 6]), 12);
        assert_eq!(
            daily_upkeep(Profession::Trainer, &[5000, 5000, 0, 0, 0, 0]),
            2
        );
        assert_eq!(
            daily_upkeep(Profession::Combatant, &[10_000, 10_000, 0, 0, 0, 0]),
            3
        );
        assert_eq!(
            daily_upkeep(Profession::Combatant, &[25_000, 25_000, 25_000, 0, 0, 0]),
            6
        );
    }

    #[test]
    fn roster_cells_take_their_text_and_ink() {
        assert_eq!(roster_name("Ash"), "Ash");
        assert_eq!(roster_name("Sixteen chars ok"), "Sixteen chars o");
        assert_eq!(
            roster_cell(
                RosterColumn::Name,
                "Boss",
                NameKind::Manager,
                RowRole::Fighter {
                    rank: 0,
                    ready: false
                },
                255,
                0,
            ),
            RosterCell::Name(RosterCellText {
                text: "Boss".into(),
                ink: Some([255, 210, 100, 153]),
            })
        );
        assert_eq!(
            roster_cell(
                RosterColumn::Name,
                "Bors",
                NameKind::Specialist,
                RowRole::Specialist(Profession::Merchant),
                255,
                0,
            ),
            RosterCell::Name(RosterCellText {
                text: "Bors".into(),
                ink: Some([220, 180, 160, 170]),
            })
        );
        assert_eq!(
            roster_cell(
                RosterColumn::Name,
                "Ash",
                NameKind::Hireling,
                RowRole::Fighter {
                    rank: 3,
                    ready: true
                },
                255,
                0,
            ),
            RosterCell::Name(RosterCellText {
                text: "Ash".into(),
                ink: Some([160, 180, 240, 170]),
            })
        );
        assert_eq!(
            roster_cell(
                RosterColumn::Name,
                "Ash",
                NameKind::Recruit,
                RowRole::Fighter {
                    rank: 3,
                    ready: true
                },
                255,
                0,
            ),
            RosterCell::Name(RosterCellText {
                text: "Ash".into(),
                ink: None,
            })
        );
        assert_eq!(
            roster_cell(
                RosterColumn::Rank,
                "Ash",
                NameKind::Recruit,
                RowRole::Fighter {
                    rank: 2,
                    ready: true
                },
                255,
                0,
            ),
            RosterCell::Rank(RosterCellText {
                text: "Novice+".into(),
                ink: None,
            })
        );
        assert_eq!(
            roster_cell(
                RosterColumn::Rank,
                "Tara",
                NameKind::Specialist,
                RowRole::Specialist(Profession::Trainer),
                255,
                0,
            ),
            RosterCell::Rank(RosterCellText {
                text: "Trainer".into(),
                ink: None,
            })
        );
        assert_eq!(
            roster_cell(
                RosterColumn::Rank,
                "Ash",
                NameKind::Hireling,
                RowRole::Fighter {
                    rank: 7,
                    ready: false
                },
                255,
                0,
            ),
            RosterCell::Rank(RosterCellText {
                text: "-".into(),
                ink: None,
            })
        );
        assert_eq!(health_fill(0), 0.0);
        assert_eq!(health_fill(65535), 1.0);
        assert_eq!(health_fill(32768), 32768.0 / 65535.0);
        // A single-precision product would give 0x3b808080 here.
        assert_eq!(health_fill(257).to_bits(), 0x3b80_8081);
        assert_eq!(health_fill(269).to_bits(), 0x3b86_8087);
        assert_eq!(
            roster_cell(
                RosterColumn::Health,
                "Ash",
                NameKind::Recruit,
                RowRole::Fighter {
                    rank: 2,
                    ready: true
                },
                255,
                32768,
            ),
            RosterCell::Health(32768.0 / 65535.0)
        );
    }

    #[test]
    fn a_roster_row_names_its_three_cells() {
        let row = roster_row(
            "Ash",
            NameKind::Recruit,
            RowRole::Fighter {
                rank: 2,
                ready: true,
            },
            255,
            65535,
        );
        assert_eq!(row.name.text, "Ash");
        assert_eq!(row.name.ink, None);
        assert_eq!(row.rank.text, "Novice+");
        assert_eq!(row.rank.ink, None);
        assert_eq!(row.health, 1.0);
    }

    #[test]
    fn a_refresh_writes_books_roster_and_report_in_order() {
        let manager = record(
            "Boss",
            0,
            Profession::Combatant,
            [5000, 0, 0, 0, 0, 0],
            65535,
            false,
        );
        let employees = [
            record(
                "Ash",
                2,
                Profession::Combatant,
                [10_000, 10_000, 0, 0, 0, 0],
                32768,
                true,
            ),
            record(
                "Tara",
                3,
                Profession::Trainer,
                [5000, 5000, 0, 0, 0, 0],
                65535,
                false,
            ),
            record("Mort", 4, Profession::Merchant, [0; 6], 65535, false),
        ];
        let listed = [
            Profession::Merchant,
            Profession::Trainer,
            Profession::Trainer,
        ];
        let day = ManageDay {
            day: 3,
            money: 1234,
            earned: 50,
            spent: 20,
            fighters: 7,
            end_day: true,
            alpha: 255,
        };
        let refresh = manage_refresh(day, &manager, &employees, &[], &listed, |_| None);
        assert_eq!(refresh.title, "Day 4");
        assert_eq!(refresh.money, 1234);
        assert_eq!(refresh.upkeep, 7);
        assert_eq!(refresh.earned, 50);
        assert_eq!(refresh.spent, 20);
        assert!(refresh.end_day);
        assert!(refresh.recruit);
        assert!(refresh.hire);
        let names: Vec<&str> = refresh
            .roster
            .iter()
            .map(|r| r.name.text.as_str())
            .collect();
        assert_eq!(names, ["Boss", "Ash", "Tara", "Mort"]);
        assert_eq!(refresh.roster[0].name.ink, Some([255, 210, 100, 153]));
        assert_eq!(refresh.roster[1].name.ink, None);
        assert_eq!(refresh.roster[1].rank.text, "Novice+");
        assert_eq!(refresh.roster[2].name.ink, Some([220, 180, 160, 170]));
        assert_eq!(refresh.roster[2].rank.text, "Trainer");
        assert_eq!(refresh.roster[3].rank.text, "Merchant");
        assert_eq!(refresh.roster[1].health, 32768.0 / 65535.0);
        let texts: Vec<&str> = refresh.report.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "Ash has trained a new technique.",
                "Ash has reached Novice rank.",
                "Boss has trained a new technique.",
                "A merchant is available for hire.",
                "2 trainers are available for hire.",
                "The tournament begins in 4 days.",
            ]
        );
        assert_eq!(refresh.report[0].code, 0x0214_0000);
        assert_eq!(refresh.report[1].code, 0x0215_0000);
        assert_eq!(refresh.report[2].code, 0x0014_0000);
    }

    #[test]
    fn a_refresh_closes_the_caps_and_skips_a_full_fighter() {
        let manager = record(
            "Boss",
            0,
            Profession::Combatant,
            [5000, 0, 0, 0, 0, 0],
            65535,
            false,
        );
        let master = record(
            "Ash",
            2,
            Profession::Combatant,
            [25_000, 25_000, 25_000, 0, 0, 0],
            65535,
            false,
        );
        let mut employees: Vec<ManageRecord> = (0..9)
            .map(|i| {
                record(
                    &format!("M{i}"),
                    10 + i,
                    Profession::Merchant,
                    [0; 6],
                    65535,
                    false,
                )
            })
            .collect();
        employees.push(master);
        let day = ManageDay {
            day: 7,
            money: 0,
            earned: 0,
            spent: 0,
            fighters: 8,
            end_day: false,
            alpha: 100,
        };
        let refresh = manage_refresh(day, &manager, &employees, &[], &[], |_| None);
        assert_eq!(refresh.title, "Week 1 Day 1");
        assert!(!refresh.recruit);
        assert!(!refresh.hire);
        assert!(!refresh.end_day);
        assert_eq!(refresh.upkeep, 18 + 6);
        assert_eq!(refresh.roster[1].name.text, "M0");
        assert_eq!(refresh.roster[1].name.ink, Some([220, 180, 160, 66]));
        assert_eq!(refresh.roster[10].rank.text, "Master");
        let texts: Vec<&str> = refresh.report.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "Boss has trained a new technique.",
                "The tournament takes place today.",
            ]
        );
    }

    #[test]
    fn a_specialist_name_is_inked_and_a_fighter_is_judged() {
        let specialist = record("Bors", 1, Profession::Physician, [0; 6], 65535, false);
        assert_eq!(specialist.rank(), 0);
        assert!(specialist.ready());
        assert_eq!(
            specialist.role(),
            RowRole::Specialist(Profession::Physician)
        );
        assert_eq!(specialist.name_kind(false), NameKind::Specialist);
        assert_eq!(specialist.upkeep(), 12);
        let fighter = record(
            "Ash",
            2,
            Profession::Combatant,
            [25_000, 25_000, 25_000, 0, 0, 0],
            0,
            true,
        );
        assert_eq!(fighter.rank(), 5);
        assert!(!fighter.ready());
        assert_eq!(
            fighter.role(),
            RowRole::Fighter {
                rank: 5,
                ready: false
            }
        );
        assert_eq!(fighter.name_kind(false), NameKind::Recruit);
        assert_eq!(fighter.row(false, 255).rank.text, "Master");
        assert_eq!(fighter.row(false, 255).health, 0.0);
    }

    #[test]
    fn a_refresh_reaches_the_master_rank_line() {
        let manager = record("Boss", 0, Profession::Combatant, [0; 6], 65535, false);
        let master = record(
            "Vex",
            2,
            Profession::Combatant,
            [25_000, 25_000, 0, 0, 0, 0],
            65535,
            true,
        );
        let day = ManageDay {
            day: 0,
            money: 0,
            earned: 0,
            spent: 0,
            fighters: 2,
            end_day: true,
            alpha: 255,
        };
        let refresh = manage_refresh(day, &manager, &[master], &[], &[], |_| None);
        assert_eq!(refresh.roster[1].rank.text, "Master+");
        let close = close_manage();
        assert!(!close.hub_busy);
        assert_eq!(close.window, None);
        assert!(close.base);
        assert_eq!(MANAGE_FORM_SIZE, 0x178);
        let texts: Vec<&str> = refresh.report.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "Vex has trained a new technique.",
                "Vex has reached Master rank.",
                "Boss has trained a new technique.",
                "Boss has reached Inept rank.",
                "The tournament takes place today.",
            ]
        );
    }
}
