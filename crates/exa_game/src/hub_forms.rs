//! The hub's table-built windows: Manage Company, Company Records, Company
//! Status, the loadout form and the dismiss and yes/no dialogs, each built
//! from its constructor's record array ([`exa_ui::arena_forms`]) by
//! [`RecordForm`], drawn through the GUI paint path in the shown palette and
//! line skin, and answering the pointer through the control classes' own
//! messages.
//!
//! What each record's callback does is the arena's own logic:
//! [`crate::manage`] (the refresh, the arsenal form's rows, steps, slider,
//! shelves and dismissal), [`crate::career::Company::end_day`] and
//! [`crate::tournament::forfeit_tourney`]. Records, Status, the loadout form
//! and the dialogs stack over Manage Company (or, for the yes/no dialog,
//! over any hub window); the top form takes the pointer.
//!
//! `--open-hub-window <name>` opens one of them once the hub is entered,
//! for scripted screenshots (`docs/DEBUGGING.md`).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ui::arena_forms::{
    self, DISMISS_FORM, LOADOUT_FORM, RECORDS_FORM, STATUS_FORM, YES_NO_FORM, dialog, loadout,
    manage, records, status,
};
use exa_ui::gui_core::GuiRect;
use exa_ui::record_form::RecordForm;

use crate::career::Profession;
use crate::gui_paint;
use crate::hub_windows::{ArenaCareer, HubWindowKind, HubWindows, Look};
use crate::manage::{
    ArsenalForm, DISABLED, HIDDEN, ManageRefresh, PAGE_HIRE, PAGE_ROSTER, RowRecord,
};

/// What a yes/no dialog's Yes runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YesHook {
    /// The tournament forfeit ("Do you wish to forfeit the entire
    /// tourney?"): every open event is emptied.
    ForfeitTourney,
}

/// Which table-built window a form is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormWindow {
    Manage,
    Records,
    Status,
    Loadout,
    Dismiss,
    YesNo(YesHook),
    /// The merchant's options in the trade window.
    Merchant,
    /// The new career's company dialog (BACK and NEXT).
    Company,
    /// The emblem form on the company dialog.
    Emblem,
    /// The colour picker form.
    Picker,
    /// A message box ([`exa_ui::modal_screens::open_message_box`]).
    Message,
}

/// The open message box's words: its title, text and flag word (to place
/// it again after a resize) and its key and result state.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenMessage {
    pub title: &'static str,
    pub text: &'static str,
    pub flags: u32,
    pub state: exa_ui::modal_screens::MessageBox,
}

/// An open table-built window.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenForm {
    pub window: FormWindow,
    pub form: RecordForm,
}

/// The hub's table-built windows, bottom to top, the loadout form's
/// selection state and the boxes it sorts once.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HubForms {
    pub forms: Vec<OpenForm>,
    pub arsenal: Option<ArsenalForm>,
    pub marked: [[bool; 5]; 5],
    /// Staff places whose employee left the company through a form, for
    /// the staff's share of the leaving (the kit's release), which needs
    /// the arena's stores ([`hub_form_input`]).
    pub left: Vec<usize>,
    /// Employees a form took on, for the staff list's matching entry.
    pub joined: Vec<crate::staff::Employee>,
    /// The extended-control forms' popup, focus and models
    /// ([`crate::option_forms`]).
    pub options: crate::option_forms::OptionForms,
    /// The message box on the stack, if any.
    pub message: Option<OpenMessage>,
    /// The result the last message box closed with: 1 for Ok, 0 for Cancel
    /// or Enter and Space, −1 for Escape.
    pub message_result: Option<i32>,
    /// Manage Company's two tables' pointer state.
    pub manage_tables: ManageTables,
}

/// Manage Company's two table views: the roster (table 0) and the report
/// (table 1).
pub const ROSTER_TABLE: usize = 0;
pub const REPORT_TABLE: usize = 1;

/// The pointer state of Manage Company's tables: the roster row a press
/// picked up for a drag, and the last press for the double press.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ManageTables {
    /// The roster's table row (0 the manager) a press picked up.
    pub drag: Option<usize>,
    /// The last press: its table, its 1-based row and its time (ms).
    pub last_press: Option<(usize, i32, i64)>,
}

/// What the pointer did on Manage Company's tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManageTableAction {
    None,
    /// A roster row (its table row, 0 the manager) let go over the roster
    /// with `drop` the 1-based row under the pointer (0 for none).
    Drop {
        row: usize,
        drop: i32,
    },
    /// A double press on a roster row (0-based, 0 the manager).
    Roster(i32),
    /// A double press on a report line (0-based).
    Report(usize),
}

/// The row of one of Manage Company's tables under a screen point: `None`
/// off the table's client, `Some(0)` on its header or below its `rows`
/// rows, else the 1-based row. Rows are six units under the six-unit
/// header, as the tables draw them.
pub fn manage_table_row(
    form: &RecordForm,
    table: usize,
    rows: usize,
    at: (i32, i32),
) -> Option<i32> {
    let pane = form.child(table)?;
    let border = exa_ui::match_board::TABLE_BORDER;
    let client = GuiRect::new(
        pane.x + border,
        pane.y + border,
        pane.w - 2 * border,
        pane.h - 2 * border,
    );
    if !client.contains(at.0, at.1) {
        return None;
    }
    let pitch = exa_ui::table_view::ROW_UNITS * form.unit();
    let y = at.1 - client.top - pitch;
    if y < 0 || pitch <= 0 {
        return Some(0);
    }
    let row = y / pitch + 1;
    Some(if row as usize > rows { 0 } else { row })
}

impl HubForms {
    pub fn is_empty(&self) -> bool {
        self.forms.is_empty()
    }

    pub fn find(&self, window: FormWindow) -> Option<usize> {
        self.forms.iter().position(|f| f.window == window)
    }

    pub fn get(&self, window: FormWindow) -> Option<&RecordForm> {
        self.find(window).map(|i| &self.forms[i].form)
    }

    pub fn get_mut(&mut self, window: FormWindow) -> Option<&mut RecordForm> {
        self.find(window).map(|i| &mut self.forms[i].form)
    }

    pub fn top(&self) -> Option<&OpenForm> {
        self.forms.last()
    }

    /// Whether a screen point is on any open form.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.forms.iter().any(|f| f.form.contains(x, y))
    }

    /// Puts a form on top; a window already open is replaced and brought
    /// to the front (the Records and Status buttons show the open form
    /// again rather than build a second).
    pub fn push(&mut self, window: FormWindow, form: RecordForm) {
        if let Some(i) = self.find(window) {
            self.forms.remove(i);
        }
        self.forms.push(OpenForm { window, form });
    }

    /// Removes a window (and forgets the loadout state with its form).
    pub fn remove(&mut self, window: FormWindow) {
        if let Some(i) = self.find(window) {
            self.forms.remove(i);
        }
        if window == FormWindow::Loadout {
            self.arsenal = None;
        }
        let options = &mut self.options;
        if options.focus.is_some_and(|(w, _)| w == window) {
            options.focus = None;
        }
        if options.key_focus.is_some_and(|(w, _)| w == window) {
            options.key_focus = None;
        }
        options.edits.retain(|e| e.window != window);
        if options.held_swatch.is_some_and(|h| h.window == window) {
            options.held_swatch = None;
        }
        if options.dragging.is_some_and(|d| d.window == window) {
            options.dragging = None;
        }
        match &options.popup {
            Some(crate::option_forms::Popup::List { window: w, .. }) if *w == window => {
                options.popup = None;
            }
            Some(crate::option_forms::Popup::Palette { .. }) if window == FormWindow::Emblem => {
                options.popup = None;
            }
            _ => {}
        }
        match window {
            FormWindow::Emblem => options.emblem = None,
            FormWindow::Picker => options.picker = None,
            FormWindow::Message => {
                if let Some(open) = self.message.take() {
                    self.message_result = Some(open.state.result);
                }
            }
            _ => {}
        }
    }

    pub fn clear(&mut self) {
        self.forms.clear();
        self.arsenal = None;
        self.options.clear();
        self.message = None;
    }

    /// Rebuilds every form for another screen or unit (a message box is
    /// placed again, as its place follows the screen).
    pub fn resize(&mut self, screen: (i32, i32), unit: i32) {
        for f in &mut self.forms {
            f.form.resize(screen, unit);
            if f.window == FormWindow::Message
                && let Some(open) = self.message.as_ref()
            {
                exa_ui::modal_screens::place_message_box(&mut f.form, open.flags);
            }
        }
    }

    /// Shows a message box on top of the stack, as the original's
    /// message-box routine does: one box at a time here (a second replaces
    /// the first). Every caller Exarust reaches passes no close hook.
    pub fn open_message(
        &mut self,
        title: &'static str,
        text: &'static str,
        flags: u32,
        screen: (i32, i32),
        unit: i32,
    ) {
        let form = exa_ui::modal_screens::open_message_box(title, text, flags, screen, unit);
        self.message = Some(OpenMessage {
            title,
            text,
            flags,
            state: exa_ui::modal_screens::MessageBox::default(),
        });
        // The box takes the keys: a text field under it loses the focus.
        self.options.focus = None;
        self.options.key_focus = None;
        self.push(FormWindow::Message, form);
        info!("message box: {title:?}: {text:?}");
    }

    /// Whether a message box is the top form (it takes the keys).
    pub fn message_on_top(&self) -> bool {
        self.top().is_some_and(|f| f.window == FormWindow::Message)
    }

    /// A key on the top message box ([`exa_ui::modal_screens::MessageBox::key`]):
    /// Enter and Space close it, Escape closes it with −1. Returns whether
    /// the box closed.
    pub fn message_key(&mut self, code: u32) -> bool {
        if !self.message_on_top() {
            return false;
        }
        let Some(open) = self.message.as_mut() else {
            return false;
        };
        if !open.state.key(code, 0) {
            return false;
        }
        self.remove(FormWindow::Message);
        true
    }

    /// A message box's button: Ok stores 1, Cancel 0, and the box closes
    /// ([`exa_ui::gui_core::message_box_button`]).
    fn message_button(&mut self, record: usize) {
        let names = self
            .get(FormWindow::Message)
            .and_then(|form| form.spec.records.get(record))
            .and_then(|r| r.text);
        let ok = match names {
            Some("Ok") => true,
            Some("Cancel") => false,
            _ => return,
        };
        let answer = exa_ui::gui_core::message_box_button(ok, false);
        let Some(open) = self.message.as_mut() else {
            return;
        };
        open.state.result = answer.result;
        open.state.closed = answer.closes;
        if answer.closes {
            self.remove(FormWindow::Message);
        }
    }

    /// The pointer over the top form: its moves, a left press and a left
    /// release, as the control messages answer them. Returns the records
    /// whose callbacks ran, with whether each came from a release.
    pub fn input(
        &mut self,
        at: (i32, i32),
        pressed: bool,
        released: bool,
    ) -> Vec<(FormWindow, usize, bool)> {
        self.options.last_at = at;
        // The emblem form is the company dialog's child: off the emblem
        // form, the dialog under it (BACK and NEXT) takes the pointer.
        let target = match self.forms.len().checked_sub(1) {
            Some(top)
                if self.forms[top].window == FormWindow::Emblem
                    && !self.forms[top].form.contains(at.0, at.1)
                    && self.forms[top].form.held().is_none() =>
            {
                self.find(FormWindow::Company).unwrap_or(top)
            }
            Some(top) => top,
            None => return Vec::new(),
        };
        let screen_h = self.forms[target].form.screen.1;
        let top = &mut self.forms[target];
        let window = top.window;
        let mut out: Vec<(FormWindow, usize, bool)> = top
            .form
            .pointer(at.0, at.1)
            .into_iter()
            .map(|i| (window, i, false))
            .collect();
        // The focus follows the pointer to the record it is on; a held
        // text field's caret follows a drag and a held swatch starts a
        // colour drag past the threshold.
        self.follow_key_focus(target, at);
        if !pressed && !released {
            self.field_drag(target, at);
            self.swatch_drag(target, at, screen_h);
        }
        if pressed {
            self.field_press(target, at);
            let top = &mut self.forms[target];
            out.extend(
                top.form
                    .press(at.0, at.1)
                    .into_iter()
                    .map(|i| (window, i, false)),
            );
            self.swatch_press(target, at);
        }
        if released {
            self.options.held_swatch = None;
            if self.drop_colour(target, at) {
                return out;
            }
            let top = &mut self.forms[target];
            out.extend(
                top.form
                    .release(at.0, at.1)
                    .into_iter()
                    .map(|i| (window, i, true)),
            );
        }
        out
    }
}

/// Writes a Manage Company refresh into its form: the four coin controls
/// (the copper totals packed into coin words), Recruit and Hire disabled at
/// their caps, End Day disabled unless the day can end, and the report
/// column's title (the day's).
pub fn fill_manage(form: &mut RecordForm, refresh: &ManageRefresh) {
    let pack = exa_ui::match_board::pack_coins;
    form.set_amount(manage::OWNED, pack(refresh.money));
    form.set_amount(manage::UPKEEP, pack(refresh.upkeep));
    form.set_amount(manage::EARNED, pack(refresh.earned));
    form.set_amount(manage::SPENT, pack(refresh.spent));
    form.set_bits(manage::RECRUIT, DISABLED, !refresh.recruit);
    form.set_bits(manage::HIRE, DISABLED, !refresh.hire);
    form.set_bits(manage::END_DAY, DISABLED, !refresh.end_day);
}

/// Company Records filled from the career: the eleven counts as text and
/// the two coin totals.
pub fn records_form(career: &ArenaCareer, screen: (i32, i32), unit: i32) -> RecordForm {
    let mut form = RecordForm::open(&RECORDS_FORM, screen, unit);
    let r = career.records();
    let counts = [
        r.matches_won,
        r.matches_lost,
        r.matches_forfeited,
        r.tournaments_won,
        r.tournaments_lost,
        r.seasons_won,
        r.beasts_fought,
        r.beasts_defeated,
        r.hires,
        r.deaths,
        r.fired,
    ];
    for (&record, value) in records::COUNTS.iter().zip(counts) {
        form.set_text(record, &value.to_string());
    }
    let pack = exa_ui::match_board::pack_coins;
    form.set_amount(records::EARNED, pack(r.earned));
    form.set_amount(records::SPENT, pack(r.spent));
    form
}

/// Company Status filled from the career: the name, the staff counted with
/// the manager and the masters (left at the table's "None" when there are
/// none).
pub fn status_form(career: &ArenaCareer, screen: (i32, i32), unit: i32) -> RecordForm {
    let mut form = RecordForm::open(&STATUS_FORM, screen, unit);
    let s = career.status();
    form.set_text(status::NAME, &s.name);
    form.set_text(status::RENOWN, s.renown);
    form.set_text(status::REPUTATION, s.reputation);
    form.set_text(status::MORALE, s.morale);
    form.set_text(status::STAFF, &s.staff.to_string());
    form.set_text(status::MASTERS, &s.masters);
    form
}

/// A dialog with its prompt.
pub fn dialog_form(
    spec: &exa_ui::record_form::FormSpec,
    prompt: &str,
    screen: (i32, i32),
    unit: i32,
) -> RecordForm {
    let mut form = RecordForm::open(spec, screen, unit);
    form.set_text(dialog::PROMPT, prompt);
    form
}

/// The row names the loadout form lists: the hire list on the hire page,
/// else the manager and the employees.
fn loadout_rows(arsenal: &mut ArsenalForm, career: &ArenaCareer) -> Vec<String> {
    let employees: Vec<String> = career
        .company
        .roster
        .iter()
        .map(|m| m.name.clone())
        .collect();
    let listed: Vec<String> = career.hire_pool.iter().map(|m| m.name.clone()).collect();
    arsenal.rows(&listed, Some(&career.player.name), &employees)
}

/// The profession of a row's record.
fn row_role(career: &ArenaCareer, record: RowRecord) -> Option<Profession> {
    match record {
        RowRecord::Manager => Some(career.player.profession),
        RowRecord::Employee(i) => career.company.roster.get(i).map(|m| m.profession),
        RowRecord::Candidate(i) => career.hire_pool.get(i).map(|m| m.profession),
    }
}

/// Writes the loadout form's state into its records: the row's name in the
/// list, Previous, Next and Dismiss's flags, the page's rank controls, the
/// slider at the tab with its rank name, the points line, and on the hire
/// page HIRE, its fee and upkeep and the profession line shown.
pub fn refresh_loadout(form: &mut RecordForm, arsenal: &mut ArsenalForm, career: &ArenaCareer) {
    let rows = loadout_rows(arsenal, career);
    arsenal.select(arsenal.selection);
    let row = arsenal.selection;
    form.set_text(
        loadout::LIST,
        rows.get(row.max(0) as usize).map_or("", String::as_str),
    );
    form.set_bits(
        loadout::PREVIOUS,
        DISABLED,
        arsenal.buttons.previous & DISABLED != 0,
    );
    form.set_bits(
        loadout::NEXT,
        DISABLED,
        arsenal.buttons.next & DISABLED != 0,
    );
    form.set_bits(
        loadout::DISMISS,
        DISABLED,
        arsenal.buttons.dismiss & DISABLED != 0,
    );
    let role = row_role(career, arsenal.record_of(row));
    if let Some(page) = arsenal.page_for(role) {
        let view = arsenal.set_page(page);
        if let Some(word) = view.rank_controls {
            for i in loadout::RANK_LABEL..=loadout::ENCUMBRANCE {
                form.set_state(i, word);
            }
        }
    }
    if arsenal.tab >= 0 {
        let tab = arsenal.tab.min(5);
        if let Some(c) = form.geometry.controls.get_mut(loadout::RANK) {
            c.value = tab as f32 / 5.0;
        }
        form.set_text(loadout::RANK, crate::report::RANKS[tab as usize]);
    }
    // The kit's point cost needs the items' points, which the hub's career
    // does not carry: the line is written for a cost of 0.
    if let Some(line) = crate::manage::points_line(0, arsenal.tab, &crate::arena::POINT_LIMITS).2 {
        form.set_text(loadout::POINTS, &line);
    }
    let hire = arsenal.page == PAGE_HIRE;
    for i in [
        loadout::HIRE,
        loadout::FEE_LABEL,
        loadout::FEE,
        loadout::UPKEEP_LABEL,
        loadout::UPKEEP,
        loadout::PROFESSION,
    ] {
        form.set_state(i, if hire { 0 } else { HIDDEN });
    }
    if hire {
        form.set_text(loadout::PROFESSION, crate::manage::profession_line(role));
        if let RowRecord::Candidate(index) = arsenal.record_of(row) {
            let price = career.hire_price(index);
            let upkeep = career.hire_pool.get(index).map_or(0, |member| {
                crate::career::Company::wage(member.profession, &member.skills)
            });
            let pack = exa_ui::match_board::pack_coins;
            form.set_amount(loadout::FEE, pack(price));
            form.set_amount(loadout::UPKEEP, pack(upkeep));
        }
    }
}

/// What an answered record asks of the hub beyond the windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormAnswer {
    None,
    /// Manage Company closed: the hub is no longer busy.
    ManageClosed,
    /// Manage Company's Arsenal: every company window closed (Manage
    /// Company's close clears the hub's busy flag) and the arsenal opens.
    OpenArsenal,
}

impl HubWindows {
    fn unit_now(&self) -> i32 {
        if self.unit > 0 { self.unit } else { 4 }
    }

    /// Opens Company Records over the open window.
    pub fn open_records(&mut self, career: &ArenaCareer) {
        let form = records_form(career, self.task.screen, self.unit_now());
        self.forms.push(FormWindow::Records, form);
    }

    /// Opens Company Status over the open window.
    pub fn open_status(&mut self, career: &ArenaCareer) {
        let form = status_form(career, self.task.screen, self.unit_now());
        self.forms.push(FormWindow::Status, form);
    }

    /// The pointer on Manage Company's tables, when it is the top form. A
    /// press on a roster row picks it up and a release over the roster
    /// drops it there (the table's drop runs [`crate::staff::Staff::
    /// move_employee`]); a second press on the same row within the GUI's
    /// double-press time activates it: a roster row opens the loadout form
    /// on that row, a report line runs its action.
    pub fn manage_table_input(
        &mut self,
        at: (i32, i32),
        pressed: bool,
        released: bool,
        now: i64,
    ) -> ManageTableAction {
        if self.forms.top().map(|f| f.window) != Some(FormWindow::Manage) {
            self.forms.manage_tables = ManageTables::default();
            return ManageTableAction::None;
        }
        let (Some(form), Some(refresh)) = (self.forms.get(FormWindow::Manage), &self.manage) else {
            return ManageTableAction::None;
        };
        let roster = manage_table_row(form, ROSTER_TABLE, refresh.roster.len(), at);
        let report = manage_table_row(form, REPORT_TABLE, refresh.report.len(), at);
        let state = &mut self.forms.manage_tables;
        if pressed {
            let hit = match (roster, report) {
                (Some(row), _) if row > 0 => Some((ROSTER_TABLE, row)),
                (_, Some(row)) if row > 0 => Some((REPORT_TABLE, row)),
                _ => None,
            };
            state.drag = None;
            let Some((table, row)) = hit else {
                state.last_press = None;
                return ManageTableAction::None;
            };
            let double = state.last_press.is_some_and(|(t, r, when)| {
                t == table && r == row && now - when <= i64::from(exa_ui::gui_core::DOUBLE_CLICK)
            });
            if double {
                state.last_press = None;
                return if table == ROSTER_TABLE {
                    ManageTableAction::Roster(row - 1)
                } else {
                    ManageTableAction::Report(row as usize - 1)
                };
            }
            state.last_press = Some((table, row, now));
            if table == ROSTER_TABLE {
                state.drag = Some(row as usize - 1);
            }
        } else if released && let Some(row) = state.drag.take() {
            // A release on the row it was picked up from is a click.
            if let Some(drop) = roster
                && drop != row as i32 + 1
            {
                return ManageTableAction::Drop { row, drop };
            }
        }
        ManageTableAction::None
    }

    /// Opens the loadout form on a page at a row, as the arena's open with
    /// a row does (the row clamped to the page).
    pub fn open_loadout_at(&mut self, page: u32, row: i32, career: &ArenaCareer) {
        self.open_loadout(page, career);
        if let Some(mut arsenal) = self.forms.arsenal {
            arsenal.select(row);
            if let Some(form) = self.forms.get_mut(FormWindow::Loadout) {
                refresh_loadout(form, &mut arsenal, career);
            }
            self.forms.arsenal = Some(arsenal);
        }
    }

    /// Opens the loadout form on a page (the roster or the hire list) at
    /// its first row.
    pub fn open_loadout(&mut self, page: u32, career: &ArenaCareer) {
        let mut form = RecordForm::open(&LOADOUT_FORM, self.task.screen, self.unit_now());
        let mut arsenal = ArsenalForm::open(page);
        refresh_loadout(&mut form, &mut arsenal, career);
        self.forms.arsenal = Some(arsenal);
        self.forms.push(FormWindow::Loadout, form);
    }

    /// Opens the dismiss dialog for a name.
    pub fn open_dismiss(&mut self, name: &str) {
        let prompt = format!(
            "{}{name}{}",
            crate::manage::DISMISS_PROMPT,
            crate::manage::DISMISS_END
        );
        let form = dialog_form(&DISMISS_FORM, &prompt, self.task.screen, self.unit_now());
        self.forms.push(FormWindow::Dismiss, form);
    }

    /// Opens the yes/no dialog with a prompt and the hook its Yes runs.
    pub fn open_yes_no(&mut self, prompt: &str, hook: YesHook) {
        let form = dialog_form(&YES_NO_FORM, prompt, self.task.screen, self.unit_now());
        self.forms.push(FormWindow::YesNo(hook), form);
    }

    /// Refreshes Manage Company from the career.
    pub fn refresh_manage(&mut self, career: &ArenaCareer) {
        let refresh = career.manage();
        if let Some(form) = self.forms.get_mut(FormWindow::Manage) {
            fill_manage(form, &refresh);
        }
        self.manage = Some(refresh);
    }

    /// Closes the top stacked form, or the hub window when none is
    /// stacked. Returns whether the hub window closed.
    pub fn close_top(&mut self) -> bool {
        match self.forms.top().map(|f| f.window) {
            Some(FormWindow::Manage) | None => {
                self.close_window();
                true
            }
            Some(window) => {
                self.forms.remove(window);
                false
            }
        }
    }

    /// Runs a record's callback.
    pub fn answer(
        &mut self,
        career: &mut ArenaCareer,
        window: FormWindow,
        record: usize,
        finished: bool,
    ) -> FormAnswer {
        match window {
            FormWindow::Manage => match record {
                manage::MANAGE => self.open_loadout(PAGE_ROSTER, career),
                manage::HIRE => self.open_loadout(PAGE_HIRE, career),
                manage::ARSENAL => {
                    // The arsenal's opener closes every company window
                    // first (Manage Company among them), then builds the
                    // arsenal view ([`crate::arsenal_view`]).
                    self.close_window();
                    return FormAnswer::OpenArsenal;
                }
                manage::RECRUIT => {
                    info!(
                        "hub manage: Recruit opens the character creator, which the hub does not run"
                    );
                }
                manage::END_DAY => {
                    let events = career.entries.map(|row| row.team_id);
                    let entered = crate::arena_match::any_entry(&career.entries);
                    // The manager rests and takes the first trainer's
                    // lesson before the staff, as the day-end routine does.
                    let physician = career
                        .company
                        .roster
                        .iter()
                        .any(|m| m.profession == Profession::Physician);
                    career.player.health = crate::career::rested(career.player.health, physician);
                    if let Some(trainer) = career
                        .company
                        .roster
                        .iter()
                        .find(|m| m.profession == Profession::Trainer)
                        .cloned()
                    {
                        crate::career::teach(&mut career.player, &trainer);
                    }
                    let night = career.company.end_day(events);
                    career.matches_today = 0;
                    // A resolved week the company entered but did not take
                    // counts one more tournament lost.
                    if entered && night.week_won == Some(false) {
                        career.company.tournaments_lost =
                            career.company.tournaments_lost.wrapping_add(1);
                    }
                    // The original's order: the shop and merchant restock,
                    // the hire list's renewal, a new week's tournament the
                    // day after a tournament day, then the day's matches.
                    // The restock waits for the live shop world on the next
                    // frame.
                    career.restock_pending = true;
                    career.renew_hire_pool();
                    if career.company.day % 7 == 1 {
                        career.regenerate_tournament_at(crate::arena_match::counter_now() as u32);
                    } else if career.company.day.is_multiple_of(7) {
                        let cleared = clear_unready_entries(career);
                        info!("hub manage: tournament day; entries cleared {cleared:?}");
                    }
                    career.regenerate_daily_board_at(crate::arena_match::counter_now() as u32);
                    info!(
                        "hub manage: the day ends (day {}, wages {})",
                        career.company.day, night.wages
                    );
                    self.refresh_manage(career);
                }
                manage::CLOSE => {
                    self.close_window();
                    return FormAnswer::ManageClosed;
                }
                manage::RECORDS => self.open_records(career),
                manage::STATUS => self.open_status(career),
                _ => {}
            },
            FormWindow::Records if record == records::CLOSE => {
                self.forms.remove(window);
            }
            FormWindow::Status if record == status::CLOSE => {
                self.forms.remove(window);
            }
            FormWindow::Loadout => self.answer_loadout(career, record, finished),
            FormWindow::Dismiss => {
                if record == dialog::YES {
                    self.dismiss_confirmed(career);
                }
                if record == dialog::YES || record == dialog::NO {
                    self.forms.remove(window);
                }
            }
            FormWindow::YesNo(hook) => {
                if record == dialog::YES {
                    match hook {
                        YesHook::ForfeitTourney => {
                            crate::tournament::forfeit_tourney(&mut career.entries);
                            // The board's own event list loses the same
                            // entrants and the board refreshes.
                            if let Some(board) = self.board.as_mut() {
                                for entry in &mut board.entries {
                                    if entry.winner == 0 {
                                        entry.participants.clear();
                                    }
                                }
                                board.show();
                            }
                            info!("hub: the tourney is forfeited");
                        }
                    }
                }
                if record == dialog::YES || record == dialog::NO {
                    self.forms.remove(window);
                }
            }
            FormWindow::Message if record != exa_ui::modal_screens::MESSAGE_LABEL => {
                self.forms.message_button(record);
            }
            w if crate::option_forms::is_option_window(w) => {
                self.answer_options(career, w, record);
            }
            _ => {}
        }
        FormAnswer::None
    }

    /// The fighter list's callback, which the box's Enter runs: when the
    /// box's selection word less one is not the form's selection, the form
    /// steps to that row (clamped to the rows) and refreshes. The word is
    /// the record's own, which only a pick from the box's list writes.
    pub(crate) fn loadout_list_entered(&mut self, career: &mut ArenaCareer, record: usize) {
        if record != loadout::LIST {
            return;
        }
        let Some(mut arsenal) = self.forms.arsenal else {
            return;
        };
        let word = self
            .forms
            .get(FormWindow::Loadout)
            .and_then(|f| f.amounts.get(record).copied())
            .unwrap_or(0) as i32;
        let row = word.wrapping_sub(1);
        // The original commits the step after the frame; the hub commits
        // it at once, as Previous and Next do.
        if row != arsenal.selection && arsenal.step_to(row).is_some() {
            arsenal.commit(|_| {});
            info!(
                "hub loadout: Enter on the fighter list steps to row {} (word {word})",
                arsenal.selection
            );
        }
        if let Some(form) = self.forms.get_mut(FormWindow::Loadout) {
            refresh_loadout(form, &mut arsenal, career);
        }
        self.forms.arsenal = Some(arsenal);
    }

    fn answer_loadout(&mut self, career: &mut ArenaCareer, record: usize, finished: bool) {
        let Some(mut arsenal) = self.forms.arsenal else {
            return;
        };
        let pressed = |forms: &HubForms, i: usize| {
            forms
                .get(FormWindow::Loadout)
                .and_then(|f| f.controls().get(i))
                .is_some_and(|c| c.state & exa_ui::gui_controls::state::ON != 0)
        };
        let tag = |i: usize| arena_forms::LOADOUT_RECORDS[i].tag as u32;
        match record {
            loadout::PREVIOUS | loadout::NEXT => {
                let step = if record == loadout::PREVIOUS {
                    arsenal.step_previous()
                } else {
                    arsenal.step_next()
                };
                // The original commits the step after the frame; the hub
                // commits it at once.
                if step.is_some() {
                    arsenal.commit(|_| {});
                }
            }
            loadout::DISMISS => {
                let rows = loadout_rows(&mut arsenal, career);
                if let Some(name) = rows.get(arsenal.selection.max(0) as usize) {
                    let name = name.clone();
                    self.forms.arsenal = Some(arsenal);
                    self.open_dismiss(&name);
                }
                return;
            }
            loadout::RANK => {
                let value = self
                    .forms
                    .get(FormWindow::Loadout)
                    .map_or(0.0, |f| f.controls()[loadout::RANK].value);
                let rank = crate::manage::slider_rank(value);
                if finished {
                    arsenal.switch_tab(rank);
                } else if let Some(form) = self.forms.get_mut(FormWindow::Loadout)
                    && let Some(name) = crate::report::RANKS.get(rank.max(0) as usize)
                {
                    form.set_text(loadout::RANK, name);
                    self.forms.arsenal = Some(arsenal);
                    return;
                }
            }
            i if loadout::SHELVES.contains(&i) => {
                let on = pressed(&self.forms, i);
                if let Some(switch) = arsenal.choose_shelf(on, tag(i), &mut self.forms.marked) {
                    info!(
                        "hub loadout: shelf {:?} -> {:?}",
                        switch.stored, switch.loaded
                    );
                }
            }
            i if loadout::BOXES.contains(&i) => {
                let on = pressed(&self.forms, i);
                if let Some(switch) = arsenal.choose_box(on, tag(i), &mut self.forms.marked) {
                    info!(
                        "hub loadout: box {:?} -> {:?}",
                        switch.stored, switch.loaded
                    );
                }
            }
            loadout::CLOSE => {
                self.forms.remove(FormWindow::Loadout);
                return;
            }
            loadout::HIRE if arsenal.page == PAGE_HIRE => {
                let index = usize::try_from(arsenal.selection.max(0)).unwrap_or(0);
                let role = career.hire_pool.get(index).map(|member| member.profession);
                match career.hire_candidate(index) {
                    Ok(role) => {
                        info!("hub hire: a {role:?} joined the company");
                        if let Some(member) = career.company.roster.last() {
                            // The record wears and holds the candidate's
                            // outfit; the list (the kit) starts empty.
                            self.forms.joined.push(crate::staff::Employee {
                                class: staff_class(role),
                                name: crate::hub_windows::name16(&member.name),
                                hands: member.outfit.hands,
                                worn: member.outfit.items.iter().map(|&id| (id, 0)).collect(),
                                ..crate::staff::Employee::default()
                            });
                        }
                        arsenal.select(arsenal.selection);
                    }
                    Err(error) => {
                        let refusal = match (error, role) {
                            (crate::career::HireError::Funds, _) => crate::staff::Refusal::Funds,
                            (crate::career::HireError::Full, _) => crate::staff::Refusal::NoRoom,
                            (crate::career::HireError::Duplicate, Some(Profession::Merchant)) => {
                                crate::staff::Refusal::HaveMerchant
                            }
                            (crate::career::HireError::Duplicate, Some(Profession::Physician)) => {
                                crate::staff::Refusal::HavePhysician
                            }
                            (crate::career::HireError::Duplicate, _) => {
                                crate::staff::Refusal::HaveTrainer
                            }
                        };
                        info!("hub hire: {}", refusal.message());
                        self.forms.open_message(
                            crate::staff::HIRE_TITLE,
                            refusal.message(),
                            crate::staff::REFUSAL_BOX_WORD,
                            self.task.screen,
                            self.unit_now(),
                        );
                    }
                }
            }
            _ => {}
        }
        if let Some(form) = self.forms.get_mut(FormWindow::Loadout) {
            refresh_loadout(form, &mut arsenal, career);
        }
        self.forms.arsenal = Some(arsenal);
    }

    /// The dismiss dialog's Dismiss: the row above is loaded, the employee
    /// shown leaves the company and the commit shows the pending row.
    fn dismiss_confirmed(&mut self, career: &mut ArenaCareer) {
        let Some(mut arsenal) = self.forms.arsenal else {
            return;
        };
        let leaving = usize::try_from(arsenal.selection - 1).ok();
        arsenal.confirm_dismiss();
        if let Some(index) = leaving
            && index < career.company.roster.len()
        {
            let id = u32::try_from(index).unwrap_or(0).wrapping_add(1);
            let member = career.company.roster.remove(index);
            career.company.fired = career.company.fired.wrapping_add(1);
            career.log.append_words(
                crate::staff::LOG_DISMISSED,
                crate::staff::name_words(&crate::hub_windows::name16(&member.name)),
            );
            crate::staff::unlink_entries(&mut career.entries, id);
            crate::staff::renumber_saved(&mut career.log, id);
            // The kit's release runs on the arena's stores.
            self.forms.left.push(index);
            info!("hub loadout: {} is dismissed", member.name);
        }
        let commit = arsenal.commit(|form| {
            let _ = loadout_rows(form, career);
        });
        if let Some(form) = self.forms.get_mut(FormWindow::Loadout) {
            refresh_loadout(form, &mut arsenal, career);
        }
        self.forms.arsenal = Some(arsenal);
        let _ = commit;
        self.refresh_manage(career);
    }
}

/// The day end's tournament-day pass, when the new day is a tournament
/// day: the unfit entrants are dropped from the pending rows
/// ([`crate::arena::drop_unfit`]), then each event entry that still names
/// someone is cleared when it names fewer than the event's fighters per
/// side, or, for an event that arms its fighters, when its first
/// entrant's weapon is not in the event's weapon row. Returns the events
/// cleared.
pub fn clear_unready_entries(career: &mut ArenaCareer) -> Vec<usize> {
    // An entrant's id is its participant id: 0 the manager, `i` roster
    // member `i - 1`.
    let healths: Vec<u16> = std::iter::once(career.player.health)
        .chain(career.company.roster.iter().map(|member| member.health))
        .collect();
    crate::arena::drop_unfit(&mut career.entries, |id| {
        healths.get(id as usize).copied().unwrap_or(0)
    });
    let board = career.board();
    let mut cleared = Vec::new();
    for (index, row) in career.entries.iter_mut().enumerate() {
        let count = row.count as i32;
        if count <= 0 {
            continue;
        }
        let Some(event) = board.tournament_events.get(index) else {
            continue;
        };
        let unready = if count < event.team_size {
            true
        } else if event.flags & exa_ui::match_board::FLAG_WEAPON != 0 {
            let weapon = board
                .entries
                .get(index)
                .and_then(|entry| entry.participants.first())
                .map_or(0, |first| first.weapon);
            let row = usize::try_from(event.weapon_row)
                .ok()
                .and_then(|row| row.checked_sub(1))
                .and_then(|row| board.weapon_tiers.get(index)?.get(row));
            !row.is_some_and(|row| row.accepts(weapon))
        } else {
            false
        };
        if unready {
            row.count = 0;
            cleared.push(index);
        }
    }
    cleared
}

/// The staff class of a hired profession.
pub fn staff_class(role: Profession) -> u8 {
    match role {
        Profession::Combatant => crate::staff::COMBATANT,
        Profession::Merchant => crate::staff::MERCHANT,
        Profession::Physician => crate::staff::PHYSICIAN,
        Profession::Trainer => crate::staff::TRAINER,
    }
}

/// Runs the staff's share of the forms' joinings and leavings on the
/// arena's stores: a hired employee joins the staff list (his record wearing
/// the candidate's outfit, his list empty, as the original's hire appends
/// him), and a dismissed one leaves it, his kit
/// going back to the arsenal ([`crate::staff::Staff::leave`]). A place the
/// staff list does not hold (a hub career whose staff was never loaded) has
/// nothing to release.
pub fn apply_staff_changes(
    forms: &mut HubForms,
    stores: &mut crate::save_live::ArenaStores,
    kinds: &crate::arsenal::ShelfKinds,
) {
    for employee in forms.joined.drain(..) {
        stores.staff.admit(employee);
    }
    for index in forms.left.drain(..) {
        let crate::save_live::ArenaStores { staff, arsenal, .. } = stores;
        match staff.leave(index, arsenal, |item| kinds.kind(item)) {
            Some(leaving) => info!(
                "hub staff: employee {} left; {} kit entries back in the arsenal",
                index + 1,
                leaving.released
            ),
            None => info!(
                "hub staff: employee {} is not on the staff list; nothing to release",
                index + 1
            ),
        }
    }
}

/// Paints one table-built form through the game skin's form draw, with
/// the words the show commits to a top-level form (its constructor's
/// attribute word, its placement word in the style), then its records
/// (the coin controls' amounts, the image buttons' frames and the combo
/// box's panel and its own front draw beside the basic classes). The shadow and backdrop
/// are pushed first; the title and frame layers come back for the caller
/// to push after the form's other children, with the committed words its
/// child views are held by.
#[must_use]
pub(crate) fn paint_form(
    painted: &mut gui_paint::Painted,
    form: &RecordForm,
    look: &Look,
) -> (Vec<exa_ui::gui_draw::GuiLayer>, gui_paint::LineFormWords) {
    paint_form_with(painted, form, look, &FormExtras::default())
}

/// What the extended forms add to a form's paint: the focused text field
/// and a trackbar's gradient under its track.
#[derive(Default)]
pub(crate) struct FormExtras<'a> {
    pub focus: Option<usize>,
    pub picker: Option<&'a exa_ui::colour_picker::ColourPicker>,
    /// The text field or combo box the form's focus is on (state bit
    /// 0x400 for its draw).
    pub key_focus: Option<usize>,
    /// The edit bytes of the form's fields that a key or press reached.
    pub edits: Vec<(usize, exa_ui::text_edit::EditView)>,
    /// The form's caret bit (the blink's lit half).
    pub caret_lit: bool,
}

/// [`paint_form`] with the extended forms' extras.
#[must_use]
pub(crate) fn paint_form_with(
    painted: &mut gui_paint::Painted,
    form: &RecordForm,
    look: &Look,
    extras: &FormExtras,
) -> (Vec<exa_ui::gui_draw::GuiLayer>, gui_paint::LineFormWords) {
    let r = form.rect();
    let bounds = GuiRect::new(r.x, r.y, r.w, r.h);
    let (font, size) = crate::hub_windows::screen_font(look.textures, look.unit);
    let (mut frame, committed) = gui_paint::paint_top_form(
        bounds,
        look.unit,
        &look.palette,
        look.fade,
        gui_paint::FormWords {
            style: form.spec.placement,
            attributes: form.spec.attributes,
        },
        Some((&form.title, &font, size)),
    );
    painted.layers.extend(frame.under());
    let origin = form.origin();
    let unit = form.unit();
    let text = crate::settings_form::Text { font: &font, size };
    for (i, c) in form.controls().iter().enumerate() {
        if c.state & HIDDEN == HIDDEN {
            continue;
        }
        let rect = GuiRect::new(origin.0 + c.x, origin.1 + c.y, c.w, c.h);
        match c.class {
            3 => {
                let words = gui_paint::ButtonWords {
                    style: c.style,
                    state: c.state,
                    group: c.group,
                };
                let button = gui_paint::paint_line_button(
                    rect,
                    words,
                    unit,
                    &look.palette,
                    look.fade,
                    &font,
                    size,
                    "",
                );
                for layer in button.layers {
                    painted.push(layer);
                }
            }
            0x12 => {
                // The panel stands for the skin's frame behind the box; the
                // front is the combo box's own draw.
                crate::hub_windows::paint_panel(painted, rect, look);
                let focused = extras.key_focus == Some(i);
                let front = gui_paint::paint_combo_front(
                    rect,
                    gui_paint::ComboWords {
                        style: c.style,
                        state: c.state | if focused { 0x400 } else { 0 },
                        pad: c.margin,
                        text: &c.text,
                        view: extras.edits.iter().find(|(r, _)| *r == i).map(|(_, v)| *v),
                    },
                    unit,
                    &look.palette,
                    &font,
                    size,
                    look.glyphs,
                );
                painted.layers.extend(front.layers);
            }
            0xac03 => {
                let amount = form.amounts.get(i).copied().unwrap_or(0);
                let parts =
                    exa_items::item_gui::coin_layout(amount, rect.left as f32, unit as f32, |n| {
                        font.text_width(n.to_string().as_bytes()) as f32
                    });
                for (_, x, n) in parts {
                    // The denomination's icon takes the four units before
                    // the number; the coin icons are not drawn.
                    let number = n.to_string();
                    let w = font.text_width(number.as_bytes()) + unit;
                    let at = GuiRect::new(x as i32 + 4 * unit, rect.top, w, rect.height());
                    crate::hub_windows::paint_text(painted, at, &number, look);
                }
            }
            0xac01 => {}
            4 if extras.picker.is_some() => {
                if let Some(stops) = extras
                    .picker
                    .and_then(|picker| crate::option_forms::picker_gradient(picker, i))
                {
                    // The gradient under the track, inset as the track is.
                    let track = GuiRect::new(
                        rect.left + 2 * unit,
                        rect.top + rect.height() / 2 - unit,
                        (rect.width() - 4 * unit).max(1),
                        2 * unit,
                    );
                    painted.push(crate::option_forms::gradient_layer(track, &stops));
                }
                crate::settings_form::paint_control(
                    c,
                    origin,
                    unit,
                    &look.palette,
                    &text,
                    None,
                    painted,
                );
            }
            _ if crate::option_forms::paint_extended(
                painted,
                c,
                rect,
                look,
                crate::option_forms::FieldDraw {
                    focused: extras.focus == Some(i) || extras.key_focus == Some(i),
                    view: extras.edits.iter().find(|(r, _)| *r == i).map(|(_, v)| *v),
                    caret_lit: extras.caret_lit,
                },
            ) => {}
            _ => crate::settings_form::paint_control(
                c,
                origin,
                unit,
                &look.palette,
                &text,
                None,
                painted,
            ),
        }
    }
    (frame.over().collect(), committed)
}

/// `--manage-table <steps>` (debug): once Manage Company is the top form,
/// its tables take these pointer steps through
/// [`HubWindows::manage_table_input`], one every few frames, at the rows'
/// centres: `drag:FROM:TO` presses roster row FROM and lets go on row TO
/// (1-based table rows, 1 the manager), `roster:ROW` and `report:LINE`
/// double press a roster row or a report line (1-based). Steps are comma
/// separated.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct ManageTableScript {
    pub steps: Vec<ManageTableStep>,
    pub wait: u32,
}

/// One `--manage-table` step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManageTableStep {
    Drag(i32, i32),
    Double(usize, i32),
}

impl ManageTableScript {
    /// The steps of a `--manage-table` value, or `None` when one does not
    /// read.
    pub fn parse(value: &str) -> Option<Self> {
        let steps = value
            .split(',')
            .map(|step| {
                let parts: Vec<&str> = step.trim().split(':').collect();
                match parts.as_slice() {
                    ["drag", from, to] => {
                        Some(ManageTableStep::Drag(from.parse().ok()?, to.parse().ok()?))
                    }
                    ["roster", row] => {
                        Some(ManageTableStep::Double(ROSTER_TABLE, row.parse().ok()?))
                    }
                    ["report", line] => {
                        Some(ManageTableStep::Double(REPORT_TABLE, line.parse().ok()?))
                    }
                    _ => None,
                }
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self { steps, wait: 30 })
    }
}

/// The screen centre of a 1-based row of one of Manage Company's tables.
pub fn manage_row_centre(form: &RecordForm, table: usize, row: i32) -> Option<(i32, i32)> {
    let pane = form.child(table)?;
    let border = exa_ui::match_board::TABLE_BORDER;
    let pitch = exa_ui::table_view::ROW_UNITS * form.unit();
    Some((
        pane.x + pane.w / 2,
        pane.y + border + pitch * row + pitch / 2,
    ))
}

/// Runs `--manage-table`'s steps through the tables' own input.
pub fn manage_table_script(
    script: Option<ResMut<ManageTableScript>>,
    time: Res<Time<Real>>,
    mut windows: ResMut<HubWindows>,
    mut career: ResMut<ArenaCareer>,
    mut stores: Option<ResMut<crate::save_live::ArenaStores>>,
) {
    let Some(mut script) = script else {
        return;
    };
    if script.steps.is_empty() || windows.forms.top().map(|f| f.window) != Some(FormWindow::Manage)
    {
        return;
    }
    if script.wait > 0 {
        script.wait -= 1;
        return;
    }
    let step = script.steps.remove(0);
    script.wait = 30;
    // The debug joins (`--hub-employee-kit`) may come after the form was
    // filled: the step reads the career as it stands.
    windows.refresh_manage(&career);
    let Some(form) = windows.forms.get(FormWindow::Manage) else {
        return;
    };
    let now = time.elapsed().as_millis() as i64;
    let presses: Vec<((i32, i32), bool, bool, i64)> = match step {
        ManageTableStep::Drag(from, to) => {
            let (Some(a), Some(b)) = (
                manage_row_centre(form, ROSTER_TABLE, from),
                manage_row_centre(form, ROSTER_TABLE, to),
            ) else {
                return;
            };
            vec![(a, true, false, now), (b, false, true, now)]
        }
        ManageTableStep::Double(table, row) => {
            let Some(at) = manage_row_centre(form, table, row) else {
                return;
            };
            vec![
                (at, true, false, now),
                (at, false, true, now),
                (at, true, false, now + 100),
            ]
        }
    };
    for (at, pressed, released, when) in presses {
        let action = windows.manage_table_input(at, pressed, released, when);
        info!(
            "--manage-table: {step:?} at {at:?} (press {pressed}, release {released}): {action:?}"
        );
        manage_table_action(action, &mut windows, &mut career, stores.as_deref_mut());
    }
}

/// `--open-hub-window <name>`: once the hub is entered and the manager
/// stands, open a window for a scripted check.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenHubWindow {
    pub name: Option<String>,
    pub done: bool,
}

/// The names `--open-hub-window` takes. `dismiss` may name the roster row
/// to dismiss (`dismiss:1`, the first employee; the manager's row 0 by
/// default); `arsenal` opens the arsenal as Manage Company's Arsenal does;
/// `nameless` opens the company dialog and presses NEXT with the name still
/// empty, which shows the "Company Name" message box.
pub const HUB_WINDOW_NAMES: [&str; 14] = [
    "board", "shop", "manage", "records", "status", "loadout", "hire", "dismiss", "yesno",
    "arsenal", "merchant", "company", "picker", "nameless",
];

/// Whether `--open-hub-window` takes `name`: one of [`HUB_WINDOW_NAMES`],
/// or `dismiss:` and a row number.
pub fn hub_window_name_ok(name: &str) -> bool {
    HUB_WINDOW_NAMES.contains(&name)
        || name
            .strip_prefix("dismiss:")
            .is_some_and(|row| row.parse::<i32>().is_ok())
}

/// Opens the asked window once.
pub fn open_hub_window_on_load(
    mut ask: ResMut<OpenHubWindow>,
    hub: Res<crate::arena_hub_live::ArenaHub>,
    mut windows: ResMut<HubWindows>,
    mut career: ResMut<ArenaCareer>,
    mut open_arsenal: MessageWriter<crate::arsenal_view::OpenArsenal>,
    mut open_shop: MessageWriter<crate::arena_hub_live::OpenHubShop>,
) {
    if ask.done || !hub.manager_placed || windows.task.screen.0 == 0 {
        return;
    }
    let Some(name) = ask.name.clone() else {
        return;
    };
    ask.done = true;
    let manage_first = |windows: &mut HubWindows, career: &ArenaCareer| {
        if !windows.is_kind(HubWindowKind::Manage) {
            windows.open_manage(career);
        }
    };
    let (name, row) = match name.split_once(':') {
        Some((name, row)) => (name.to_string(), row.parse::<i32>().unwrap_or(0)),
        None => (name, 0),
    };
    match name.as_str() {
        "board" => windows.open_board(&career),
        "arsenal" => {
            open_arsenal.write(crate::arsenal_view::OpenArsenal);
            info!("hub window arsenal opens");
            return;
        }
        // The shop opens as the hub's click on it does: its first stock is
        // made when the shelf is empty.
        "shop" | "merchant" => {
            open_shop.write(crate::arena_hub_live::OpenHubShop {
                merchant: name == "merchant",
            });
            info!("hub window {name} asked");
            return;
        }
        // The new career's company dialog and emblem form; the hub has no
        // front end, so a debug opener stands in for New Career.
        "company" => windows.open_company(),
        "nameless" => {
            windows.open_company();
            windows.answer(
                &mut career,
                FormWindow::Company,
                exa_ui::emblem_form::company::NEXT,
                true,
            );
        }
        // The colour picker on a white swatch, as the editor's lighting
        // panel's swatches open it.
        "picker" => windows.open_picker([0xff, 0xff, 0xff, 0xff], "Global Tint"),
        "manage" => manage_first(&mut windows, &career),
        "records" => {
            manage_first(&mut windows, &career);
            windows.open_records(&career);
        }
        "status" => {
            manage_first(&mut windows, &career);
            windows.open_status(&career);
        }
        "loadout" | "hire" => {
            manage_first(&mut windows, &career);
            let page = if name == "hire" {
                PAGE_HIRE
            } else {
                PAGE_ROSTER
            };
            windows.open_loadout(page, &career);
        }
        "dismiss" => {
            manage_first(&mut windows, &career);
            windows.open_loadout(PAGE_ROSTER, &career);
            let mut name = career.player.name.clone();
            if let Some(mut arsenal) = windows.forms.arsenal {
                arsenal.select(row);
                let rows = loadout_rows(&mut arsenal, &career);
                if let Some(row_name) = rows.get(arsenal.selection.max(0) as usize) {
                    name = row_name.clone();
                }
                if let Some(form) = windows.forms.get_mut(FormWindow::Loadout) {
                    refresh_loadout(form, &mut arsenal, &career);
                }
                windows.forms.arsenal = Some(arsenal);
            }
            windows.open_dismiss(&name);
        }
        "yesno" => {
            windows.open_board(&career);
            windows.open_yes_no(crate::tournament::FORFEIT_TOURNEY, YesHook::ForfeitTourney);
        }
        other => {
            warn!("--open-hub-window: unknown window {other:?} (one of {HUB_WINDOW_NAMES:?})");
            return;
        }
    }
    info!(
        "hub window {name} opens: {:?}",
        windows.forms.top().map(|f| (f.window, f.form.rect()))
    );
}

/// `--hub-employee-kit NAME:id,...`: once the hub is entered and the
/// manager stands, a combatant NAME joins the company with this kit, as a
/// loaded career's employee would stand: the staff list takes it through
/// the hire (its kit stowed in the arsenal with the fighter's flag,
/// [`crate::staff::Staff::hire`]) and the roster lists it. A debug tool for
/// the dismissal's kit release.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct HubEmployeeKit {
    pub name: String,
    pub kit: Vec<u32>,
    /// Further combatants (the flag given again), joining after the first
    /// in order.
    pub more: Vec<(String, Vec<u32>)>,
    pub done: bool,
}

/// Runs `--hub-employee-kit` once, when the shelf facts are read.
pub fn hub_employee_kit_on_load(
    ask: Option<ResMut<HubEmployeeKit>>,
    hub: Res<crate::arena_hub_live::ArenaHub>,
    mut career: ResMut<ArenaCareer>,
    stores: Option<ResMut<crate::save_live::ArenaStores>>,
    kinds: Option<Res<crate::arsenal::ShelfKinds>>,
) {
    let (Some(mut ask), Some(mut stores), Some(kinds)) = (ask, stores, kinds) else {
        return;
    };
    if ask.done || !hub.manager_placed || kinds.0.is_empty() {
        return;
    }
    ask.done = true;
    let mut joining = vec![(ask.name.clone(), ask.kit.clone())];
    joining.extend(ask.more.iter().cloned());
    for (name, kit) in &joining {
        join_with_kit(name, kit, &mut career, &mut stores, &kinds);
    }
}

/// One `--hub-employee-kit` combatant joins.
fn join_with_kit(
    name_text: &str,
    kit: &[u32],
    career: &mut ArenaCareer,
    stores: &mut crate::save_live::ArenaStores,
    kinds: &crate::arsenal::ShelfKinds,
) {
    let name = crate::hub_windows::name16(name_text);
    let hire = crate::staff::Hire {
        role: Some(Profession::Combatant),
        name,
        carried: Vec::new(),
        loadouts: vec![crate::hirelings::Loadout {
            hands: [0, 0],
            items: kit.to_vec(),
        }],
    };
    let mut purse = 0;
    let mut spent = 0;
    let mut records = career.records();
    let crate::save_live::ArenaStores { staff, arsenal, .. } = &mut *stores;
    let joined = staff.hire(
        hire,
        0,
        &mut Vec::<()>::new(),
        None,
        &mut purse,
        &mut spent,
        &mut records,
        &mut career.log,
        arsenal,
        |item| kinds.kind(item),
    );
    if let Err(refusal) = joined {
        warn!("--hub-employee-kit: {}", refusal.message());
        return;
    }
    // The member carries the kit as its record would: weapons and shields
    // in the two hands (in order), the rest worn, and the six loadouts a
    // hire builds from it (a level-0 fighter trims none).
    let mut outfit = crate::hirelings::Loadout::default();
    for &item in kit {
        let hand = matches!(
            crate::arsenal::shelf_of(kinds.kind(item)),
            Some(crate::arsenal::WEAPONS | crate::arsenal::SHIELDS)
        );
        match outfit.hands.iter_mut().find(|held| **held == 0) {
            Some(free) if hand => *free = item,
            _ => outfit.items.push(item),
        }
    }
    let mut member = crate::career::Member {
        name: name_text.to_string(),
        profession: Profession::Combatant,
        ..career.player.clone()
    };
    for (slot, loadout) in member
        .loadouts
        .iter_mut()
        .zip(crate::hirelings::ranked_loadouts(&outfit, 0, |_| None))
    {
        *slot = exa_formats::characters::Loadout {
            word: 0,
            hands: [
                exa_formats::characters::ItemRef::new(loadout.hands[0], 0),
                exa_formats::characters::ItemRef::new(loadout.hands[1], 0),
                exa_formats::characters::ItemRef::NONE,
                exa_formats::characters::ItemRef::NONE,
            ],
            worn: loadout
                .items
                .iter()
                .map(|&id| exa_formats::characters::ItemRef::new(id, 0))
                .collect(),
        };
    }
    member.outfit = outfit;
    career.company.roster.push(member);
    let stowed: Vec<(u32, Option<usize>)> = kit
        .iter()
        .map(|&item| (item, crate::arsenal::shelf_of(kinds.kind(item))))
        .collect();
    info!(
        "--hub-employee-kit: {} joins as employee {} with kit (item, shelf) {:?}",
        name_text,
        stores.staff.employees.len(),
        stowed
    );
}

/// `--hub-arsenal id,...`: once the hub is entered and the shelf facts are
/// read, each item is stowed in the arsenal with no flags (on its shelf's
/// first box), as an item no fighter has taken stands there. A debug tool
/// for the loadout form's kit; a new career's own arsenal is not stocked.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct HubArsenal {
    pub items: Vec<u32>,
    pub done: bool,
}

/// `--hub-stash id,...`: once the hub is entered, each id joins the hub
/// stash as a stashed item does. A debug tool for the arena save's
/// promotion; a new career's own stash is empty.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct HubStash {
    pub items: Vec<u32>,
    pub done: bool,
}

/// Runs `--hub-stash` once, when the manager stands in the hub.
pub fn hub_stash_on_load(
    ask: Option<ResMut<HubStash>>,
    hub: Res<crate::arena_hub_live::ArenaHub>,
    stores: Option<ResMut<crate::save_live::ArenaStores>>,
) {
    let (Some(mut ask), Some(mut stores)) = (ask, stores) else {
        return;
    };
    if ask.done || !hub.manager_placed {
        return;
    }
    ask.done = true;
    stores.stash.extend(ask.items.iter().copied());
    info!("--hub-stash: the stash holds {:#x?}", stores.stash);
}

/// Runs `--hub-arsenal` once, when the shelf facts are read.
pub fn hub_arsenal_on_load(
    ask: Option<ResMut<HubArsenal>>,
    hub: Res<crate::arena_hub_live::ArenaHub>,
    stores: Option<ResMut<crate::save_live::ArenaStores>>,
    kinds: Option<Res<crate::arsenal::ShelfKinds>>,
    live: Option<Res<exa_assets::live_objects::LiveObjects>>,
) {
    let (Some(mut ask), Some(mut stores), Some(kinds)) = (ask, stores, kinds) else {
        return;
    };
    if ask.done || !hub.manager_placed || kinds.0.is_empty() {
        return;
    }
    ask.done = true;
    let stowed: Vec<(u32, Option<usize>)> = ask
        .items
        .iter()
        .map(|&item| {
            // A level's or the world's own record is shelved by the
            // template it was made from (the shelf facts are the
            // templates').
            let template = live
                .as_ref()
                .filter(|_| item & exa_formats::objdb::TAG_MASK != 0)
                .map_or(item, |live| live.db().template_of(item));
            let kind = kinds.kind(template);
            stores.arsenal.stow(item, kind, 0);
            (item, crate::arsenal::shelf_of(kind))
        })
        .collect();
    info!("--hub-arsenal: stowed (item, shelf) {stowed:?}");
}

/// The pointer over the hub's table-built forms: moves, the left press and
/// release go to the top form and its callbacks run. Manage Company's
/// Arsenal asks for the arsenal ([`crate::arsenal_view::OpenArsenal`]), and
/// the forms' joinings and leavings reach the arena's stores
/// ([`apply_staff_changes`]).
#[allow(clippy::too_many_arguments)]
pub fn hub_form_input(
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time<Real>>,
    windows_q: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    mut windows: ResMut<HubWindows>,
    mut career: ResMut<ArenaCareer>,
    mut hub: ResMut<crate::arena_hub_live::ArenaHub>,
    (mut stores, kinds): (
        Option<ResMut<crate::save_live::ArenaStores>>,
        Option<Res<crate::arsenal::ShelfKinds>>,
    ),
    mut open_arsenal: MessageWriter<crate::arsenal_view::OpenArsenal>,
) {
    if !windows.forms.is_empty() {
        let pointer = windows_q.single().ok().and_then(|window| {
            let at = match scripted.as_deref() {
                Some(scripted) => scripted.position(window),
                None => window.cursor_position(),
            }?;
            Some((
                (at.x * window.scale_factor()) as i32,
                (at.y * window.scale_factor()) as i32,
            ))
        });
        windows.forms.options.clock = time.elapsed().as_millis() as i64;
        if let Some(at) = pointer {
            let pressed = mouse.just_pressed(MouseButton::Left);
            let released = mouse.just_released(MouseButton::Left);
            if (pressed || released) && windows.forms.contains(at.0, at.1) {
                windows.took_pointer = true;
            }
            if windows.popup_input(&mut career, at, pressed, released) {
                if pressed || released {
                    windows.took_pointer = true;
                }
                return;
            }
            // A press elsewhere takes the focus from a text field.
            if pressed {
                windows.forms.options.focus = None;
            }
            let now = windows.forms.options.clock;
            let table = windows.manage_table_input(at, pressed, released, now);
            if table != ManageTableAction::None {
                manage_table_action(table, &mut windows, &mut career, stores.as_deref_mut());
                windows.took_pointer = true;
                return;
            }
            let answered = windows.forms.input(at, pressed, released);
            for (form, record, finished) in answered {
                match windows.answer(&mut career, form, record, finished) {
                    FormAnswer::ManageClosed => hub.busy = false,
                    FormAnswer::OpenArsenal => {
                        hub.busy = false;
                        open_arsenal.write(crate::arsenal_view::OpenArsenal);
                        info!("hub manage: Arsenal opens the arsenal");
                    }
                    FormAnswer::None => {}
                }
            }
        }
    }
    if let (Some(mut stores), Some(kinds)) = (stores, kinds) {
        apply_staff_changes(&mut windows.forms, &mut stores, &kinds);
    }
}

/// Runs what the pointer did on Manage Company's tables. A drop moves the
/// employee in the staff list and in the career's roster alike (the ids in
/// the log, the tournament entries and the order list remapped), then the
/// form is refreshed. A roster row's double press opens the loadout form
/// on it; a report line's runs [`crate::manage::report_click`]: the
/// tournament line closes Manage Company and opens the board on its
/// tournament tab, a hire-list line the hire page on the first candidate of
/// its profession, a training or rank line the roster page on its first
/// row.
pub fn manage_table_action(
    action: ManageTableAction,
    windows: &mut HubWindows,
    career: &mut ArenaCareer,
    stores: Option<&mut crate::save_live::ArenaStores>,
) {
    match action {
        ManageTableAction::None => {}
        ManageTableAction::Drop { row, drop } => {
            let Some(stores) = stores else {
                return;
            };
            let staff = &mut stores.staff;
            let target = staff.drop_target(drop);
            let moved = staff.move_employee(row, drop, &mut career.log, &mut career.entries);
            if moved && row <= career.company.roster.len() && target <= career.company.roster.len()
            {
                let member = career.company.roster.remove(row - 1);
                career.company.roster.insert(target - 1, member);
            }
            info!(
                "hub manage: roster row {row} dropped on row {drop}: moved {moved} to row {target}; roster {:?}",
                career
                    .company
                    .roster
                    .iter()
                    .map(|m| m.name.as_str())
                    .collect::<Vec<_>>()
            );
            windows.refresh_manage(career);
        }
        ManageTableAction::Roster(row) => {
            info!("hub manage: roster row {row} double pressed: the loadout form opens on it");
            windows.open_loadout_at(PAGE_ROSTER, row, career);
        }
        ManageTableAction::Report(line) => {
            let Some(code) = windows
                .manage
                .as_ref()
                .and_then(|refresh| refresh.report.get(line))
                .map(|line| line.code)
            else {
                return;
            };
            let listed: Vec<Profession> = career
                .hire_pool
                .iter()
                .map(|member| member.profession)
                .collect();
            let report = crate::manage::report_click(code, &listed);
            info!("hub manage: report line {line} (code {code:#x}) double pressed: {report:?}");
            match report {
                crate::manage::ReportAction::TournamentBoard => {
                    windows.close_window();
                    windows.open_board(career);
                    if let Some(board) = windows.board.as_mut() {
                        board.switch_tab(exa_ui::match_board::BoardTab::Tournament);
                    }
                }
                crate::manage::ReportAction::HireList(candidate) => {
                    windows.open_loadout_at(PAGE_HIRE, candidate as i32, career);
                }
                crate::manage::ReportAction::Roster { .. } => {
                    windows.open_loadout_at(PAGE_ROSTER, 0, career);
                }
                crate::manage::ReportAction::None => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed() -> HubForms {
        let mut forms = HubForms::default();
        forms.open_message(
            crate::staff::HIRE_TITLE,
            crate::staff::Refusal::Funds.message(),
            crate::staff::REFUSAL_BOX_WORD,
            (1280, 720),
            4,
        );
        forms
    }

    /// A message box goes on top, takes Enter, Space and Escape, and
    /// leaves its result.
    #[test]
    fn a_message_box_closes_on_its_keys() {
        let mut forms = boxed();
        assert!(forms.message_on_top());
        assert!(!forms.message_key(u32::from(b'a')));
        assert!(forms.message_on_top());
        assert!(forms.message_key(exa_ui::modal_screens::KEY_ESCAPE));
        assert!(!forms.message_on_top() && forms.message.is_none());
        assert_eq!(forms.message_result, Some(-1));
        let mut forms = boxed();
        assert!(forms.message_key(exa_ui::modal_screens::KEY_ENTER));
        assert_eq!(forms.message_result, Some(0));
        let mut forms = boxed();
        assert!(forms.message_key(exa_ui::modal_screens::KEY_SPACE));
        assert_eq!(forms.message_result, Some(0));
        assert!(!forms.message_key(exa_ui::modal_screens::KEY_SPACE));
    }

    /// Its Ok button stores 1 and closes it; a resize places it again.
    #[test]
    fn a_message_box_ok_stores_one() {
        let mut forms = boxed();
        forms.resize((2560, 1440), 8);
        let form = forms.get(FormWindow::Message).unwrap();
        let r = form.rect();
        assert_eq!(r.x, (2560 - r.w) / 2);
        let (ox, oy) = form.origin();
        let ok = &form.controls()[1];
        let at = (ox + ok.x + ok.w / 2, oy + ok.y + ok.h / 2);
        forms.input(at, true, false);
        let answered = forms.input(at, false, true);
        assert_eq!(answered, vec![(FormWindow::Message, 1, true)]);
        forms.message_button(1);
        assert!(forms.message.is_none() && forms.forms.is_empty());
        assert_eq!(forms.message_result, Some(1));
    }

    /// The tournament day's pass clears an entry naming fewer than its
    /// event's fighters per side and keeps a full one of an unarmed event.
    #[test]
    fn a_tournament_day_clears_the_short_entries() {
        let mut career = ArenaCareer::default();
        career.player.health = u16::MAX;
        let board = career.board();
        let short = board
            .tournament_events
            .iter()
            .position(|event| event.team_size > 1);
        let full = board.tournament_events.iter().position(|event| {
            event.team_size == 1 && event.flags & exa_ui::match_board::FLAG_WEAPON == 0
        });
        for index in [short, full].into_iter().flatten() {
            career.entries[index].count = 1;
            career.entries[index].team_id = 1;
            career.entries[index].entries[0].actor_id = 0;
        }
        let cleared = clear_unready_entries(&mut career);
        if let Some(short) = short {
            assert!(cleared.contains(&short));
            assert_eq!(career.entries[short].count, 0);
        }
        if let Some(full) = full {
            assert!(!cleared.contains(&full));
            assert_eq!(career.entries[full].count, 1);
        }
        assert!(short.is_some() || full.is_some());
    }

    fn opened() -> (HubWindows, ArenaCareer) {
        let mut windows = HubWindows::default();
        windows.task.screen = (1280, 720);
        windows.unit = 4;
        let mut career = ArenaCareer::default();
        career.company.roster.push(crate::career::Member {
            name: "Brom".to_string(),
            ..career.player.clone()
        });
        windows.open_manage(&career);
        (windows, career)
    }

    fn click(windows: &mut HubWindows, career: &mut ArenaCareer, window: FormWindow, i: usize) {
        let form = windows.forms.get(window).unwrap();
        let (ox, oy) = form.origin();
        let c = &form.controls()[i];
        let at = (ox + c.x + c.w / 2, oy + c.y + c.h / 2);
        let answered = windows.forms.input(at, true, false);
        assert!(answered.is_empty());
        for (w, r, f) in windows.forms.input(at, false, true) {
            windows.answer(career, w, r, f);
        }
    }

    #[test]
    fn a_shelf_button_switches_the_loadout_forms_arsenal_shelf() {
        let (mut windows, mut career) = opened();
        windows.open_loadout(PAGE_ROSTER, &career);
        assert_eq!(windows.forms.arsenal.unwrap().shelf, 0);
        // The first shelf button is the rightmost, clothing (its value 4).
        let form = windows.forms.get(FormWindow::Loadout).unwrap();
        assert_eq!(form.controls()[loadout::SHELVES[0]].x, 968);
        click(
            &mut windows,
            &mut career,
            FormWindow::Loadout,
            loadout::SHELVES[0],
        );
        assert_eq!(windows.forms.arsenal.unwrap().shelf, 4);
    }

    #[test]
    fn manage_opens_from_its_table_and_fills_its_coins() {
        let (windows, career) = opened();
        let form = windows.forms.get(FormWindow::Manage).unwrap();
        assert_eq!(form.controls().len(), 18);
        assert_eq!(
            form.amounts[manage::OWNED],
            exa_ui::match_board::pack_coins(career.company.copper)
        );
        assert_eq!(form.rect().w, 0x2c4);
    }

    #[test]
    fn the_manage_buttons_open_records_status_and_the_loadout() {
        let (mut windows, mut career) = opened();
        click(
            &mut windows,
            &mut career,
            FormWindow::Manage,
            manage::RECORDS,
        );
        assert_eq!(windows.forms.top().unwrap().window, FormWindow::Records);
        click(
            &mut windows,
            &mut career,
            FormWindow::Records,
            records::CLOSE,
        );
        assert_eq!(windows.forms.top().unwrap().window, FormWindow::Manage);
        click(
            &mut windows,
            &mut career,
            FormWindow::Manage,
            manage::STATUS,
        );
        let status = windows.forms.get(FormWindow::Status).unwrap();
        assert_eq!(status.controls()[status::STAFF].text, b"2");
        click(&mut windows, &mut career, FormWindow::Status, status::CLOSE);
        click(
            &mut windows,
            &mut career,
            FormWindow::Manage,
            manage::MANAGE,
        );
        let loadout_form = windows.forms.get(FormWindow::Loadout).unwrap();
        assert_eq!(loadout_form.controls()[loadout::LIST].text, b"Manager");
        // Previous and Dismiss are disabled on the first row.
        assert_ne!(
            loadout_form.controls()[loadout::PREVIOUS].state & DISABLED,
            0
        );
        assert_ne!(
            loadout_form.controls()[loadout::DISMISS].state & DISABLED,
            0
        );
    }

    #[test]
    fn the_roster_drags_and_its_rows_and_report_lines_double_press() {
        let (mut windows, mut career) = opened();
        for name in ["Cato", "Dara"] {
            career.company.roster.push(crate::career::Member {
                name: name.to_string(),
                ..career.player.clone()
            });
        }
        windows.refresh_manage(&career);
        let mut stores = crate::save_live::ArenaStores::default();
        for name in [b"Brom", b"Cato", b"Dara"] {
            let mut employee = crate::staff::Employee {
                class: crate::staff::COMBATANT,
                name: [0; 16],
                kit: Vec::new(),
                carried: Vec::new(),
                hands: [0, 0],
                worn: Vec::new(),
                world: None,
            };
            employee.name[..4].copy_from_slice(name);
            stores.staff.employees.push(employee);
        }
        let (brom, dara, manager) = {
            let form = windows.forms.get(FormWindow::Manage).unwrap();
            let centre = |row| manage_row_centre(form, ROSTER_TABLE, row).unwrap();
            let (brom, dara) = (centre(2), centre(4));
            let header = (brom.0, form.child(ROSTER_TABLE).unwrap().y + 3);
            assert_eq!(manage_table_row(form, ROSTER_TABLE, 4, header), Some(0));
            assert_eq!(manage_table_row(form, ROSTER_TABLE, 4, dara), Some(4));
            assert_eq!(manage_table_row(form, ROSTER_TABLE, 3, dara), Some(0));
            (brom, dara, centre(1))
        };
        // Brom (table row 1) let go on Dara's row (4) goes last.
        assert_eq!(
            windows.manage_table_input(brom, true, false, 0),
            ManageTableAction::None
        );
        let action = windows.manage_table_input(dara, false, true, 10);
        assert_eq!(action, ManageTableAction::Drop { row: 1, drop: 4 });
        manage_table_action(action, &mut windows, &mut career, Some(&mut stores));
        let names: Vec<&str> = career
            .company
            .roster
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        assert_eq!(names, ["Cato", "Dara", "Brom"]);
        assert_eq!(&stores.staff.employees[2].name[..4], b"Brom");
        // The manager's row does not move.
        windows.manage_table_input(manager, true, false, 1000);
        let action = windows.manage_table_input(dara, false, true, 1010);
        manage_table_action(action, &mut windows, &mut career, Some(&mut stores));
        assert_eq!(career.company.roster[0].name, "Cato");
        // A double press on a roster row opens the loadout form on it.
        // Cato is on table row 2 now.
        let cato = brom;
        windows.manage_table_input(cato, true, false, 5000);
        windows.manage_table_input(cato, false, true, 5010);
        let action = windows.manage_table_input(cato, true, false, 5100);
        assert_eq!(action, ManageTableAction::Roster(1));
        manage_table_action(action, &mut windows, &mut career, Some(&mut stores));
        assert_eq!(windows.forms.top().unwrap().window, FormWindow::Loadout);
        assert_eq!(windows.forms.arsenal.unwrap().selection, 1);
        // Too slow for a double press.
        windows
            .forms
            .forms
            .retain(|f| f.window == FormWindow::Manage);
        windows.manage_table_input(cato, true, false, 9000);
        windows.manage_table_input(cato, false, true, 9010);
        let late = 9000 + i64::from(exa_ui::gui_core::DOUBLE_CLICK) + 1;
        assert_eq!(
            windows.manage_table_input(cato, true, false, late),
            ManageTableAction::None
        );
    }

    #[test]
    fn next_then_dismiss_removes_the_employee() {
        let (mut windows, mut career) = opened();
        windows.open_loadout(PAGE_ROSTER, &career);
        click(
            &mut windows,
            &mut career,
            FormWindow::Loadout,
            loadout::NEXT,
        );
        let form = windows.forms.get(FormWindow::Loadout).unwrap();
        assert_eq!(form.controls()[loadout::LIST].text, b"Brom");
        assert_eq!(form.controls()[loadout::DISMISS].state & DISABLED, 0);
        click(
            &mut windows,
            &mut career,
            FormWindow::Loadout,
            loadout::DISMISS,
        );
        let prompt = &windows.forms.get(FormWindow::Dismiss).unwrap().controls()[dialog::PROMPT];
        assert_eq!(prompt.text, b"Are you sure you want to dismiss Brom?");
        click(&mut windows, &mut career, FormWindow::Dismiss, dialog::YES);
        assert!(career.company.roster.is_empty());
        assert_eq!(windows.forms.top().unwrap().window, FormWindow::Loadout);
        let form = windows.forms.get(FormWindow::Loadout).unwrap();
        assert_eq!(form.controls()[loadout::LIST].text, b"Manager");
    }

    #[test]
    fn the_yes_no_dialog_runs_its_hook() {
        let (mut windows, mut career) = opened();
        career.entries[0].count = 3;
        career.entries[1].count = 1;
        career.entries[1].team_id = 4;
        // The open board lists the same entrants, the second event decided.
        let mut board = career.board();
        let fighter = exa_ui::match_board::Participant {
            slot: 1,
            value: 0,
            weapon: 0,
        };
        board.entries[0].participants = vec![fighter];
        board.entries[1].participants = vec![fighter];
        board.entries[1].winner = 4;
        windows.board = Some(board);
        windows.open_yes_no(crate::tournament::FORFEIT_TOURNEY, YesHook::ForfeitTourney);
        click(
            &mut windows,
            &mut career,
            FormWindow::YesNo(YesHook::ForfeitTourney),
            dialog::YES,
        );
        assert_eq!(career.entries[0].count, 0);
        assert_eq!(career.entries[1].count, 1);
        let board = windows.board.as_ref().unwrap();
        assert!(board.entries[0].participants.is_empty());
        assert_eq!(board.entries[1].participants.len(), 1);
        assert!(
            windows
                .forms
                .find(FormWindow::YesNo(YesHook::ForfeitTourney))
                .is_none()
        );
    }

    #[test]
    fn manage_close_closes_the_window() {
        let (mut windows, mut career) = opened();
        let form = windows.forms.get(FormWindow::Manage).unwrap();
        let (ox, oy) = form.origin();
        let c = form.controls()[manage::CLOSE].clone();
        let at = (ox + c.x + 2, oy + c.y + 2);
        windows.forms.input(at, true, false);
        let answered = windows.forms.input(at, false, true);
        assert_eq!(answered, vec![(FormWindow::Manage, manage::CLOSE, true)]);
        let answer = windows.answer(&mut career, FormWindow::Manage, manage::CLOSE, true);
        assert_eq!(answer, FormAnswer::ManageClosed);
        assert!(!windows.is_open());
    }

    #[test]
    fn the_day_rests_the_manager_before_the_staff() {
        let mut windows = HubWindows::default();
        windows.task.screen = (1280, 720);
        windows.unit = 4;
        let mut career = ArenaCareer {
            matches_today: 1,
            ..ArenaCareer::default()
        };
        career.player.health = 1000;
        career.company.roster.push(crate::career::Member {
            name: "Brom".to_string(),
            health: 1000,
            ..career.player.clone()
        });
        windows.open_manage(&career);
        click(
            &mut windows,
            &mut career,
            FormWindow::Manage,
            manage::END_DAY,
        );
        assert_eq!(career.player.health, 7_553);
        assert_eq!(career.company.roster[0].health, 7_553);
        assert_eq!(career.company.day, 1);
        // The new day asks for the shop and merchant restock and the new
        // weekly tournament (day 1 is the day after a tournament day).
        assert!(career.restock_pending);
        assert!(career.tournament.is_some());
    }

    #[test]
    fn an_entered_week_a_rival_takes_counts_one_more_tournament_lost() {
        let mut windows = HubWindows::default();
        windows.task.screen = (1280, 720);
        windows.unit = 4;
        let mut career = ArenaCareer::default();
        // Day 7 ends into day 8, the day after a tournament day: the week is
        // resolved. The company entered event 0; company 2 won all six.
        career.company.day = 7;
        career.entries[0].count = 1;
        for row in &mut career.entries {
            row.team_id = 2;
        }
        windows.open_manage(&career);
        click(
            &mut windows,
            &mut career,
            FormWindow::Manage,
            manage::END_DAY,
        );
        assert_eq!(career.company.day, 8);
        assert_eq!(career.company.tournaments_lost, 1);
        assert_eq!(career.company.tournaments_won, 0);
        // The new week's board cleared the entries.
        assert!(!crate::arena_match::any_entry(&career.entries));
        // A week the company did not enter books nothing.
        let mut idle = ArenaCareer::default();
        idle.company.day = 7;
        idle.matches_today = 1;
        for row in &mut idle.entries {
            row.team_id = 2;
        }
        windows.open_manage(&idle);
        click(&mut windows, &mut idle, FormWindow::Manage, manage::END_DAY);
        assert_eq!(idle.company.tournaments_lost, 0);
    }

    #[test]
    fn a_dismissal_logs_the_name_and_counts_one_more_fired() {
        let (mut windows, mut career) = opened();
        windows.open_loadout(crate::manage::PAGE_ROSTER, &career);
        let mut arsenal = windows.forms.arsenal.expect("the loadout form");
        arsenal.select(1);
        windows.forms.arsenal = Some(arsenal);
        windows.dismiss_confirmed(&mut career);
        assert_eq!(career.company.fired, 1);
        assert!(career.company.roster.is_empty());
        let record = career.log.records.last().expect("the dismissal");
        assert_eq!(record.id, crate::staff::LOG_DISMISSED);
        assert_eq!(
            record.words,
            crate::staff::name_words(&crate::hub_windows::name16("Brom"))
        );
    }

    #[test]
    fn hiring_a_candidate_from_the_hire_page_books_it() {
        let (mut windows, mut career) = opened();
        career.company.copper = 1000;
        career.company.spent = 0;
        career.company.hires = 0;
        career.log.records.clear();
        let candidate = career.hire_pool[0].clone();
        let price = career.hire_price(0);
        windows.open_loadout(crate::manage::PAGE_HIRE, &career);
        let mut arsenal = windows.forms.arsenal.expect("the loadout form");
        arsenal.select(0);
        windows.forms.arsenal = Some(arsenal);
        click(
            &mut windows,
            &mut career,
            FormWindow::Loadout,
            loadout::HIRE,
        );
        assert_eq!(
            career.company.roster.last().map(|m| m.name.clone()),
            Some(candidate.name)
        );
        assert_eq!(career.company.copper, 1000 - price);
        assert_eq!(career.company.spent, price);
        assert_eq!(career.company.hires, 1);
        assert_eq!(
            career.log.records.last().map(|r| r.id),
            Some(crate::staff::LOG_JOINED)
        );
        assert_eq!(career.hire_pool.len(), crate::hirelings::POOL_SIZE - 1);
    }
}
