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
use crate::gui_paint::{self, Frame};
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
    }

    pub fn clear(&mut self) {
        self.forms.clear();
        self.arsenal = None;
    }

    /// Rebuilds every form for another screen or unit.
    pub fn resize(&mut self, screen: (i32, i32), unit: i32) {
        for f in &mut self.forms {
            f.form.resize(screen, unit);
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
        let Some(top) = self.forms.last_mut() else {
            return Vec::new();
        };
        let window = top.window;
        let mut out: Vec<(FormWindow, usize, bool)> = top
            .form
            .pointer(at.0, at.1)
            .into_iter()
            .map(|i| (window, i, false))
            .collect();
        if pressed {
            out.extend(
                top.form
                    .press(at.0, at.1)
                    .into_iter()
                    .map(|i| (window, i, false)),
            );
        }
        if released {
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
                    info!("hub manage: Arsenal opens the stash form, which is not built");
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
                    career.renew_hire_pool();
                    career.regenerate_daily_board_at(crate::arena_match::counter_now() as u32);
                    if career.company.day % 7 == 1 {
                        career.regenerate_tournament_at(crate::arena_match::counter_now() as u32);
                    }
                    career.restock_pending = true;
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
                            info!("hub: the tourney is forfeited");
                        }
                    }
                }
                if record == dialog::YES || record == dialog::NO {
                    self.forms.remove(window);
                }
            }
            _ => {}
        }
        FormAnswer::None
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

/// Paints one table-built form: its frame in the form's own words, then
/// its records (the coin controls' amounts, the image buttons' frames and
/// the combo box's panel and text beside the basic classes).
pub(crate) fn paint_form(painted: &mut gui_paint::Painted, form: &RecordForm, look: &Look) {
    let r = form.rect();
    let bounds = GuiRect::new(r.x, r.y, r.w, r.h);
    let (font, size) = crate::hub_windows::screen_font(look.textures, look.unit);
    let mut frame = Frame::new(bounds, look.unit, &form.title);
    frame.attributes = form.spec.attributes;
    frame.spacing = form.geometry.border;
    for layer in gui_paint::paint_frame(&frame, &look.palette, &font, size).layers {
        painted.push(layer);
    }
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
                crate::hub_windows::paint_panel(painted, rect, look);
                let caption = String::from_utf8_lossy(&c.text);
                let inner = GuiRect::new(
                    rect.left + unit,
                    rect.top,
                    rect.width() - 2 * unit,
                    rect.height(),
                );
                crate::hub_windows::paint_text(painted, inner, &caption, look);
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
}

/// `--open-hub-window <name>`: once the hub is entered and the manager
/// stands, open a window for a scripted check.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenHubWindow {
    pub name: Option<String>,
    pub done: bool,
}

/// The names `--open-hub-window` takes.
pub const HUB_WINDOW_NAMES: [&str; 9] = [
    "board", "shop", "manage", "records", "status", "loadout", "hire", "dismiss", "yesno",
];

/// Opens the asked window once.
pub fn open_hub_window_on_load(
    mut ask: ResMut<OpenHubWindow>,
    hub: Res<crate::arena_hub_live::ArenaHub>,
    mut windows: ResMut<HubWindows>,
    career: Res<ArenaCareer>,
    names: Res<crate::hub_windows::HubItemNames>,
) {
    if ask.done || !hub.manager_placed || windows.task.screen.0 == 0 {
        return;
    }
    let Some(name) = ask.name.clone() else {
        return;
    };
    ask.done = true;
    let manage_first = |windows: &mut HubWindows| {
        if !windows.is_kind(HubWindowKind::Manage) {
            windows.open_manage(&career);
        }
    };
    match name.as_str() {
        "board" => windows.open_board(&career),
        "shop" => windows.open_shop(false, &career, &names),
        "manage" => manage_first(&mut windows),
        "records" => {
            manage_first(&mut windows);
            windows.open_records(&career);
        }
        "status" => {
            manage_first(&mut windows);
            windows.open_status(&career);
        }
        "loadout" | "hire" => {
            manage_first(&mut windows);
            let page = if name == "hire" {
                PAGE_HIRE
            } else {
                PAGE_ROSTER
            };
            windows.open_loadout(page, &career);
        }
        "dismiss" => {
            manage_first(&mut windows);
            windows.open_loadout(PAGE_ROSTER, &career);
            windows.open_dismiss(&career.player.name);
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

/// The pointer over the hub's table-built forms: moves, the left press and
/// release go to the top form and its callbacks run.
#[allow(clippy::too_many_arguments)]
pub fn hub_form_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows_q: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    mut windows: ResMut<HubWindows>,
    mut career: ResMut<ArenaCareer>,
    mut hub: ResMut<crate::arena_hub_live::ArenaHub>,
) {
    if windows.forms.is_empty() {
        return;
    }
    let Ok(window) = windows_q.single() else {
        return;
    };
    let pointer = match scripted.as_deref() {
        Some(scripted) => scripted.position(window),
        None => window.cursor_position(),
    };
    let Some(at) = pointer else {
        return;
    };
    let at = (
        (at.x * window.scale_factor()) as i32,
        (at.y * window.scale_factor()) as i32,
    );
    let pressed = mouse.just_pressed(MouseButton::Left);
    let released = mouse.just_released(MouseButton::Left);
    if (pressed || released) && windows.forms.contains(at.0, at.1) {
        windows.took_pointer = true;
    }
    let answered = windows.forms.input(at, pressed, released);
    for (form, record, finished) in answered {
        if windows.answer(&mut career, form, record, finished) == FormAnswer::ManageClosed {
            hub.busy = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        windows.open_yes_no(crate::tournament::FORFEIT_TOURNEY, YesHook::ForfeitTourney);
        click(
            &mut windows,
            &mut career,
            FormWindow::YesNo(YesHook::ForfeitTourney),
            dialog::YES,
        );
        assert_eq!(career.entries[0].count, 0);
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
