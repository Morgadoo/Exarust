//! The extended-control forms on the hub's form stack: the merchant's
//! options in the trade window, the company dialog with its emblem form,
//! and the colour picker form.
//!
//! - **Merchant options** ([`exa_ui::arena_forms::MERCHANT_FORM`]). The
//!   trade window adds it at (0x12c, 0x190) in its client when it trades
//!   with the company's merchant. Its `Look for:` and `Rank:` combo boxes
//!   open their drop-down lists ([`exa_ui::drop_down`]); a pick writes the
//!   row back into the box and its callback sets the merchant's stock type
//!   ([`exa_ui::arena_forms::look_for_kinds`]) or specialisation
//!   ([`exa_ui::arena_forms::rank_of`]), which the nightly restock reads.
//!   Opening it writes the current stock type's row and the rank's row into
//!   the records; a stock type outside the list keeps the last row (the
//!   record array is static in the original).
//! - **Company dialog and emblem form** ([`exa_ui::emblem_form`]). The
//!   dialog is a black screen with BACK and NEXT; the emblem form is centred
//!   on it. The name field types the company's name at its caret (and
//!   `klean` with Enter selects the preset); the step buttons and number
//!   fields select the background
//!   and design through the selector rule
//!   ([`exa_ui::emblem_form::step_states`]); the mirror boxes set the layer
//!   bits; each colour swatch opens the palette swatch popup
//!   ([`exa_ui::col_swatch`]) under the pointer, whose hovered or pressed
//!   cell is the colour slot's palette index, and a swatch dragged onto
//!   another gives it its colour word; Randomise rolls and Previous
//!   takes the last roll back. NEXT with an empty name refuses ("Please name
//!   your company"); otherwise it stores the name and emblem in the career.
//! - **Colour picker** ([`exa_ui::colour_picker::PICKER_FORM`]). The six
//!   channel trackbars and spinners, the old and new colour swatches, Reset,
//!   Close and the colour image drive [`exa_ui::colour_picker::ColourPicker`]
//!   through [`exa_ui::colour_picker::record_event`]; every refresh is
//!   written back into the controls. In the original it opens from a colour
//!   swatch whose callback is the picker's opener (the editor's lighting
//!   panel); the arena's forms have none, so the hub opens it only through
//!   `--open-hub-window picker`.
//!
//! NEXT's refusal shows the "Please name your company" message box over the
//! stack ([`crate::hub_forms::HubForms::open_message`]). Not built: the
//! creator that NEXT starts (the hub does not run it), the colour image's
//! picture (no constructor or refresh gives the record one; it draws its
//! shade), and a spinner's typed value and drag.
//!
//! The focus follows the pointer; the focused text field or combo box takes
//! the keys through its own handler ([`exa_ui::field_edit`]).

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use exa_ui::arena_forms::{self, MERCHANT_FORM, MERCHANT_PLACE, merchant};
use exa_ui::col_swatch::{self, ColourSwatch};
use exa_ui::colour_picker::{
    self, ColourPicker, PICKER_FORM, PickerEvent, SwatchControl, picker_records,
};
use exa_ui::controls::{COLOUR_SWATCH, COMBO_BOX, ColourDrag, TEXT_FIELD};
use exa_ui::drop_down::{DropDownList, DropDownOwner};
use exa_ui::emblem_form::{
    self, COMPANY_RECORDS, EMBLEM_FORM, EmblemAction, EmblemEditor, EmblemRandom, company,
    company_form, emblem_action, records as emblem_records,
};
use exa_ui::field_edit::{self, FieldKey, FieldWords};
use exa_ui::gui_controls::{Control, state};
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_keys::GuiKey;
use exa_ui::record_form::RecordForm;
use exa_ui::settings::Rect;
use exa_ui::text_edit::{EditAction, EditView};

use crate::gui_paint::{LineFormLayers, Painted};
use crate::hub_forms::FormWindow;
use crate::hub_windows::{ArenaCareer, HubWindows, Look};

/// A popup over the form stack.
#[derive(Clone, Debug, PartialEq)]
pub enum Popup {
    /// A combo box's drop-down list, at a screen rectangle.
    List {
        window: FormWindow,
        record: usize,
        list: DropDownList,
        rect: Rect,
    },
    /// The palette swatch popup an emblem swatch opened.
    Palette {
        record: usize,
        swatch: ColourSwatch,
        rect: Rect,
    },
}

/// The emblem form's state beside its records.
#[derive(Clone, Debug, PartialEq)]
pub struct EmblemState {
    pub editor: EmblemEditor,
}

/// The colour picker's state beside its records: the picker and the
/// swatch it edits.
#[derive(Clone, Debug, PartialEq)]
pub struct PickerState {
    pub picker: ColourPicker,
    pub swatch: SwatchControl,
}

/// The extended forms' state on [`crate::hub_forms::HubForms`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OptionForms {
    pub popup: Option<Popup>,
    /// The text field that takes typed keys.
    pub focus: Option<(FormWindow, usize)>,
    pub emblem: Option<EmblemState>,
    pub picker: Option<PickerState>,
    /// The `Look for:` record's word, kept between openings as the
    /// original's static record keeps it.
    pub look_for_word: u32,
    /// The last pointer the forms saw (a spinner's release reads it).
    pub last_at: (i32, i32),
    /// The text field or combo box the form's focus is on: the record
    /// under the pointer, which takes the keys ([`exa_ui::field_edit`]).
    pub key_focus: Option<(FormWindow, usize)>,
    /// A text field's Enter gave the focus up at this pointer; it comes
    /// back when the pointer moves.
    pub focus_released: Option<(i32, i32)>,
    /// The edit bytes of every text field and combo box a key or press
    /// reached.
    pub edits: Vec<FieldEdit>,
    /// The colour swatch the left button holds, until the drag threshold.
    pub held_swatch: Option<HeldSwatch>,
    /// The colour a swatch is dragging.
    pub dragging: Option<ColourDragging>,
    /// The clock (milliseconds) the pointer's drag threshold reads.
    pub clock: i64,
    /// The main font the forms draw with (its advances place the caret).
    pub font: Option<exa_ui::gui_font::GuiFont>,
    /// The company dialog was opened by the front end's NEW CAREER: its
    /// BACK and NEXT run behind the transition ([`crate::company_live`])
    /// instead of closing the dialog at once.
    pub company_front_end: bool,
    /// The company dialog's BACK or NEXT waiting for
    /// [`crate::company_live`]; once set, the dialog answers no more
    /// buttons.
    pub company_exit: Option<CompanyExit>,
}

/// The `focus_released` place a focus set with the pointer's next place
/// unknown takes: the next pointer place seen becomes the place the pointer
/// must leave before the focus follows it again (the opener's warp and
/// focus, whose own move does not take the focus away).
pub const FOCUS_HELD_UNTIL_SEEN: (i32, i32) = (i32::MIN, i32::MIN);

/// The company dialog's way out, which [`crate::company_live`] runs behind
/// the transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompanyExit {
    /// BACK: back to the title.
    Back,
    /// NEXT with a name: the name and the emblem record are stored and the
    /// creator starts.
    Named {
        name: String,
        emblem: exa_ui::EmblemRecord,
    },
}

/// A text field's or combo box's edit bytes beside its record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldEdit {
    pub window: FormWindow,
    pub record: usize,
    pub view: EditView,
}

/// The swatch a left press holds: where and when the press was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldSwatch {
    pub window: FormWindow,
    pub record: usize,
    pub at: (i32, i32),
    pub clock: i64,
}

/// A colour drag: the swatch it came from and its payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColourDragging {
    pub window: FormWindow,
    pub record: usize,
    pub drag: ColourDrag,
}

impl OptionForms {
    /// Forgets the open forms' state (the kept `Look for:` word stays).
    pub fn clear(&mut self) {
        self.popup = None;
        self.focus = None;
        self.emblem = None;
        self.picker = None;
        self.key_focus = None;
        self.focus_released = None;
        self.edits.clear();
        self.held_swatch = None;
        self.dragging = None;
        self.company_front_end = false;
        self.company_exit = None;
    }
}

/// A combo box's drop-down list, opened under its owner record at
/// `owner` (screen x, y, width, height) with `selected` (1-based) chosen:
/// the list takes its opener's form skin and the owner's width. The line
/// skin's opening sets bit 1 of its mark word, and the opener then clears
/// the list's panel frame style (0x80), so the skin draws only its wide
/// window frame, and lifts the list by a unit less one. The list's layout
/// gives its height (sixteen rows at most, a longer list scrolls), and the
/// GUI's child placement ([`exa_ui::place_form`]) puts it on the screen:
/// one pixel over the owner's bottom edge when it fits there, else with
/// its bottom one pixel under the owner's top; at the owner's x unless
/// the list would leave the screen's right edge, then flush with that
/// edge less two (the owner's right edge is tried first). The list is not
/// cut to the room left, as the combo box's open does not cut it.
pub(crate) fn open_drop_list(
    items: Vec<Vec<u8>>,
    owner: (i32, i32, i32, i32),
    unit: i32,
    screen: (i32, i32),
    selected: i32,
) -> (DropDownList, Rect) {
    let (x, mut top, w, h) = owner;
    let mut list = DropDownList::new(items, w, unit);
    let mark = exa_ui::skin::open_skin(unit, 0, true).map_or(0, |skin| skin.mark);
    if mark & 1 != 0 {
        list.style &= !0x80;
        top -= unit - 1;
    }
    let layout = list.lay_out();
    let (px, py) = exa_ui::place_form(
        screen,
        w,
        layout.height,
        exa_ui::PlaceRequest {
            desired_x: x,
            fallback_x: x + w,
            desired_y: top + h - 1,
            fallback_bottom: top + 1,
        },
    );
    list.select(selected, 0);
    info!(
        "combo list opens at ({px}, {py}) {w}x{}, rows {} px from y {}",
        layout.height,
        layout.row,
        py + list.border
    );
    (
        list,
        Rect {
            x: px,
            y: py,
            w,
            h: layout.height,
        },
    )
}

/// The drop-down's owner as the combo box's record: the chosen row.
struct ComboOwner {
    chosen: Option<i32>,
}

impl DropDownOwner for ComboOwner {
    fn row_chosen(&mut self, row: i32) {
        self.chosen = Some(row);
    }
    fn drop_closed(&mut self) {}
}

/// One frame of the pointer over an open drop-down list at `rect`: the
/// row under the pointer is selected, a left press inside arms it and a
/// press outside closes the list; the release of an armed press writes the
/// selection back (the chosen 1-based row) and closes the list. Returns
/// whether the list stays open and the row chosen.
pub(crate) fn drive_drop_list(
    list: &mut DropDownList,
    rect: Rect,
    at: (i32, i32),
    pressed: bool,
    released: bool,
) -> (bool, Option<i32>) {
    let inside = contains(rect, at);
    if inside {
        let row = list.row_at(at.1 - rect.y);
        list.select(row, 0);
    }
    let mut keep = true;
    if pressed {
        if !inside {
            keep = false;
        } else {
            list.press(1);
        }
    }
    if released && list.pressed > 0 {
        let mut owner = ComboOwner { chosen: None };
        list.release(&mut owner);
        list.close(&mut owner);
        return (false, owner.chosen);
    }
    (keep, None)
}

/// A record's rectangle on the screen.
fn on_screen(form: &RecordForm, c: &Control) -> GuiRect {
    let (ox, oy) = form.origin();
    GuiRect::new(ox + c.x, oy + c.y, c.w, c.h)
}

fn contains(rect: Rect, at: (i32, i32)) -> bool {
    at.0 >= rect.x && at.1 >= rect.y && at.0 < rect.x + rect.w && at.1 < rect.y + rect.h
}

impl HubWindows {
    fn unit_or_4(&self) -> i32 {
        if self.unit > 0 { self.unit } else { 4 }
    }

    /// Adds the merchant's options to the open trade window: the records
    /// take the stock type's and specialisation's rows, and the form sits
    /// at its place in the window's client.
    pub fn open_merchant_options(&mut self, career: &ArenaCareer) {
        let unit = self.unit_or_4();
        let mut form = RecordForm::open(&MERCHANT_FORM, self.task.screen, unit);
        if let Some(row) = arena_forms::look_for_selection(career.merchant_kinds) {
            self.forms.options.look_for_word = row;
        }
        let rank = arena_forms::rank_selection(career.merchant_rank);
        set_combo(
            &mut form,
            merchant::LOOK_FOR,
            self.forms.options.look_for_word,
        );
        set_combo(&mut form, merchant::RANK, rank);
        if let Some(bounds) = self.bounds() {
            let border = crate::shop_window::BORDER;
            form.place_at(
                bounds.left + border + MERCHANT_PLACE.0 * unit / 4,
                bounds.top + border + MERCHANT_PLACE.1 * unit / 4,
            );
        }
        info!(
            "hub trade: the merchant's options open (look for {}, rank {rank}) at {:?}",
            self.forms.options.look_for_word,
            form.rect()
        );
        log_records("merchant options", &form);
        self.forms.push(FormWindow::Merchant, form);
    }

    /// Opens the company dialog and its emblem form as the new career's
    /// opener does: the emblem form's constructor state (background and
    /// design 1, no mirror, the four starting palette indices) applied to
    /// its records.
    pub fn open_company(&mut self) {
        self.open_company_with(None);
    }

    /// Opens the company dialog as [`Self::open_company`] does, restoring
    /// `last` (the emblem record the last NEXT stored) when its background
    /// word is above 0, and focuses the name field, as the new career's
    /// opener does.
    pub fn open_company_with(&mut self, last: Option<exa_ui::EmblemRecord>) {
        let unit = self.unit_or_4();
        let screen = self.task.screen;
        let dialog = RecordForm::open(&company_form(screen, unit), screen, unit);
        log_records("company dialog", &dialog);
        self.forms.push(FormWindow::Company, dialog);
        let mut form = RecordForm::open(&EMBLEM_FORM, screen, unit);
        let mut state = EmblemState {
            editor: EmblemEditor::new(),
        };
        if let Some(last) = last.filter(|last| last.background > 0) {
            info!("company dialog: the last emblem {last:?} is restored");
            state.editor.set_record(last);
        }
        apply_emblem(&mut form, &state.editor);
        state.editor.refresh_colours();
        // The constructor's first selections only store the numbers; the
        // fields read "1" from their records.
        self.forms.options.emblem = Some(state);
        info!("company dialog: the emblem form opens at {:?}", form.rect());
        log_records("emblem form", &form);
        self.forms.push(FormWindow::Emblem, form);
        debug_assert_eq!(COMPANY_RECORDS.len(), 2);
        self.focus_record(FormWindow::Emblem, emblem_records::NAME);
    }

    /// Gives a form's text field the key focus, its whole text selected,
    /// as the GUI's focus call does (the focus then waits for the pointer
    /// to move before it follows it again).
    pub fn focus_record(&mut self, window: FormWindow, record: usize) {
        let Some(c) = self
            .forms
            .get(window)
            .and_then(|form| form.controls().get(record).cloned())
        else {
            return;
        };
        if !takes_keys(&c) {
            return;
        }
        let mut view = view_of(&mut self.forms.options.edits, window, record, &c.text);
        field_edit::focus_gained(&c.text, &mut view);
        store_view(&mut self.forms.options.edits, window, record, view);
        self.forms.options.key_focus = Some((window, record));
        self.forms.options.focus_released = Some(self.forms.options.last_at);
    }

    /// Opens the colour picker on a colour (its caption the swatch's name),
    /// as a colour swatch's picker opener does.
    pub fn open_picker(&mut self, colour: [u8; 4], name: &str) {
        let unit = self.unit_or_4();
        let mut form = RecordForm::open(&PICKER_FORM, self.task.screen, unit);
        let mut swatch = SwatchControl::new(colour);
        swatch.name = name.as_bytes().to_vec();
        let picker = ColourPicker::new(0, &mut swatch);
        if !picker.name.is_empty() {
            form.title = String::from_utf8_lossy(&picker.name).into_owned();
        }
        // The picker keeps the draw skin's default instance, whose title
        // bar puts the client's top under it.
        form.client_top = Some(exa_ui::skin::BASE_TITLED_CLIENT_TOP);
        let state = PickerState { picker, swatch };
        sync_picker(&mut form, &state);
        info!(
            "colour picker: opens on {:?} at {:?}",
            state.swatch.colour,
            form.rect()
        );
        self.forms.options.picker = Some(state);
        log_records("colour picker", &form);
        self.forms.push(FormWindow::Picker, form);
    }

    /// Runs an extended form's record callback.
    pub(crate) fn answer_options(
        &mut self,
        career: &mut ArenaCareer,
        window: FormWindow,
        record: usize,
    ) {
        match window {
            FormWindow::Merchant => self.open_list(window, record),
            // A front-end dialog's way out is already asked for.
            FormWindow::Company if self.forms.options.company_exit.is_some() => {}
            FormWindow::Company
                if record == company::BACK && self.forms.options.company_front_end =>
            {
                info!("company dialog: BACK fades back to the title");
                self.forms.options.company_exit = Some(CompanyExit::Back);
            }
            FormWindow::Company => match record {
                company::BACK => {
                    info!("company dialog: BACK returns to the front end");
                    self.forms.remove(FormWindow::Emblem);
                    self.forms.remove(FormWindow::Company);
                    self.forms.options.clear();
                }
                company::NEXT => self.company_next(career),
                _ => {}
            },
            FormWindow::Emblem => self.answer_emblem(record),
            FormWindow::Picker => self.answer_picker(record),
            _ => {}
        }
    }

    fn company_next(&mut self, career: &mut ArenaCareer) {
        let Some(form) = self.forms.get(FormWindow::Emblem) else {
            return;
        };
        let name =
            String::from_utf8_lossy(&form.controls()[emblem_records::NAME].text).into_owned();
        let Some(state) = self.forms.options.emblem.as_ref() else {
            return;
        };
        match crate::emblem::confirm_company(&name, &state.editor) {
            crate::emblem::CompanyConfirm::MissingName => {
                info!("company dialog: NEXT refused: the name is empty");
                let (screen, unit) = (self.task.screen, self.unit_or_4());
                self.forms.open_message(
                    crate::emblem::COMPANY_NAME_TITLE,
                    crate::emblem::COMPANY_NAME_PROMPT,
                    crate::emblem::COMPANY_NAME_BOX_WORD,
                    screen,
                    unit,
                );
            }
            crate::emblem::CompanyConfirm::Named { name, emblem } => {
                info!(
                    "company dialog: NEXT names the company {name:?} with emblem {emblem:?}; the creator follows (not run from the hub)"
                );
                career.company.name = name.clone();
                career.company.emblem = emblem;
                if self.forms.options.company_front_end {
                    self.forms.options.company_exit = Some(CompanyExit::Named { name, emblem });
                    return;
                }
                self.forms.remove(FormWindow::Emblem);
                self.forms.remove(FormWindow::Company);
                self.forms.options.clear();
            }
        }
    }

    /// A combo box's report: its drop-down list opens under it with its
    /// row selected.
    fn open_list(&mut self, window: FormWindow, record: usize) {
        let unit = self.unit_or_4();
        let screen = self.task.screen;
        let Some(form) = self.forms.get(window) else {
            return;
        };
        let items = form.spec.records[record].items;
        if items.is_empty() {
            return;
        }
        let c = &form.controls()[record];
        let rect = on_screen(form, c);
        let (list, place) = open_drop_list(
            items.iter().map(|item| item.as_bytes().to_vec()).collect(),
            (rect.left, rect.top, c.w, c.h),
            unit,
            screen,
            form.amounts[record] as i32,
        );
        self.forms.options.popup = Some(Popup::List {
            window,
            record,
            list,
            rect: place,
        });
    }

    /// The popup takes the pointer while it is up. Returns whether it did.
    pub(crate) fn popup_input(
        &mut self,
        career: &mut ArenaCareer,
        at: (i32, i32),
        pressed: bool,
        released: bool,
    ) -> bool {
        let unit = self.unit_or_4();
        let Some(mut popup) = self.forms.options.popup.take() else {
            return false;
        };
        let mut keep = true;
        let mut chosen: Option<(FormWindow, usize, u32)> = None;
        let mut picked: Option<(usize, i32)> = None;
        match &mut popup {
            Popup::List {
                window,
                record,
                list,
                rect,
            } => {
                let (open, row) = drive_drop_list(list, *rect, at, pressed, released);
                keep = open;
                if let Some(row) = row {
                    chosen = Some((*window, *record, row as u32));
                }
            }
            Popup::Palette {
                record,
                swatch,
                rect,
            } => {
                let border = col_swatch::BORDER;
                let inside = contains(*rect, at);
                let local = (at.0 - rect.x - border, at.1 - rect.y - border);
                if inside {
                    let code = swatch.hit(-1, local.0, local.1, unit);
                    if swatch.hover(code, 0) == col_swatch::CHANGED {
                        picked = Some((*record, swatch.index));
                    }
                }
                if pressed {
                    if inside {
                        if swatch.press(local.0, local.1, unit, 0) == col_swatch::CHANGED {
                            picked = Some((*record, swatch.index));
                        }
                    } else {
                        // A press off the popup cancels it: the opening
                        // index comes back.
                        let before = swatch.index;
                        swatch.close(0);
                        if swatch.index != before {
                            picked = Some((*record, swatch.index));
                        }
                        keep = false;
                    }
                }
                if released && inside {
                    swatch.close(1);
                    keep = false;
                }
            }
        }
        if keep {
            self.forms.options.popup = Some(popup);
        }
        if let Some((window, record, row)) = chosen {
            if let Some(form) = self.forms.get_mut(window) {
                set_combo(form, record, row);
            }
            if window == FormWindow::Merchant {
                if record == merchant::LOOK_FOR {
                    self.forms.options.look_for_word = row;
                    career.merchant_kinds = arena_forms::look_for_kinds(row);
                } else if record == merchant::RANK {
                    career.merchant_rank = arena_forms::rank_of(row);
                }
                info!(
                    "hub trade: the merchant now looks for {:?} ({:#x}) of rank {} ({})",
                    arena_forms::LOOK_FOR_ITEMS
                        .get((self.forms.options.look_for_word as usize).wrapping_sub(1)),
                    career.merchant_kinds,
                    career.merchant_rank,
                    arena_forms::RANK_ITEMS
                        .get((career.merchant_rank + 1).max(0) as usize)
                        .copied()
                        .unwrap_or("?")
                );
            }
        }
        if let Some((record, index)) = picked {
            self.emblem_colour(record, index as u8);
        }
        true
    }

    /// The palette popup told its owner: the swatch shows the cell (its
    /// colour with the index in the alpha byte) and the form's colour slot
    /// takes it; the form's refresh fills the record's colours.
    fn emblem_colour(&mut self, record: usize, index: u8) {
        let Some(slot) = emblem_records::SWATCHES.iter().position(|&r| r == record) else {
            return;
        };
        if let Some(form) = self.forms.get_mut(FormWindow::Emblem)
            && let Some(c) = form.geometry.controls.get_mut(record)
        {
            c.colour = Some(emblem_form::swatch_colour(index));
        }
        if let Some(state) = self.forms.options.emblem.as_mut() {
            let colour = &mut state.editor.record.colours[slot];
            *colour = (*colour & 0x00ff_ffff) | (u32::from(index) << 24);
            state.editor.refresh_colours();
            info!("emblem: colour {} takes palette {index:#04x}", slot + 1);
        }
    }

    fn answer_emblem(&mut self, record: usize) {
        let Some(action) = emblem_action(record) else {
            return;
        };
        let at = self.forms.options.last_at;
        let unit = self.unit_or_4();
        let screen = self.task.screen;
        let Some(i) = self.forms.find(FormWindow::Emblem) else {
            return;
        };
        let Some(state) = self.forms.options.emblem.as_mut() else {
            return;
        };
        let form = &mut self.forms.forms[i].form;
        match action {
            // The fields take the keys while the pointer is on them
            // ([`crate::hub_forms::HubForms::follow_key_focus`]).
            EmblemAction::Name | EmblemAction::Number { .. } => {}
            EmblemAction::Step { design, step } => {
                let current = if design {
                    state.editor.record.design
                } else {
                    state.editor.record.background
                } as i32;
                select_picture(form, &mut state.editor, design, current.wrapping_add(step));
            }
            EmblemAction::Mirror { bit } => {
                if form.controls()[record].state & state::ON != 0 {
                    state.editor.record.layers |= bit;
                } else {
                    state.editor.record.layers &= !bit;
                }
                info!("emblem: layers {:#x}", state.editor.record.layers);
            }
            EmblemAction::Swatch { .. } => {
                let index = state.editor.record.palette(
                    emblem_records::SWATCHES
                        .iter()
                        .position(|&r| r == record)
                        .unwrap_or(0),
                );
                let mut swatch = ColourSwatch::new();
                swatch.index = i32::from(index);
                swatch.original = i32::from(index);
                swatch.owner_callback = true;
                let cell = col_swatch::CELL_UNITS * unit;
                let border = col_swatch::BORDER;
                let w = border * 2 + col_swatch::COLUMNS * cell;
                let h = border * 2 + col_swatch::ROWS * cell;
                // The opener puts the popup's current cell under the
                // pointer: its centre is (index / 8, index % 8) cells and
                // half a cell in from the client.
                let x = at.0 - (i32::from(index >> 3) * cell + cell / 2 + border);
                let y = at.1 - (i32::from(index % 8) * cell + cell / 2 + border);
                // [guess] the place routine keeps the popup on the screen.
                let x = x.clamp(0, (screen.0 - w).max(0));
                let y = y.clamp(0, (screen.1 - h).max(0));
                info!("emblem: the palette swatch opens on {index:#04x} at ({x}, {y})");
                self.forms.options.popup = Some(Popup::Palette {
                    record,
                    swatch,
                    rect: Rect { x, y, w, h },
                });
            }
            EmblemAction::Randomise => {
                let seed = crate::arena_match::counter_now() as u32;
                state.editor.randomise(&mut EmblemRandom::new(seed));
                form.set_state(emblem_records::PREVIOUS, 0);
                apply_emblem(form, &state.editor);
                let (background, design) =
                    (state.editor.record.background, state.editor.record.design);
                select_picture(form, &mut state.editor, false, background as i32);
                select_picture(form, &mut state.editor, true, design as i32);
                state.editor.refresh_colours();
                info!("emblem: randomised to {:?}", state.editor.record);
            }
            EmblemAction::Previous => {
                let (taken, empty) = state.editor.previous();
                if taken {
                    apply_emblem(form, &state.editor);
                    state.editor.refresh_colours();
                }
                if empty {
                    form.set_bits(emblem_records::PREVIOUS, state::DISABLED, true);
                }
                info!("emblem: previous ({taken}) {:?}", state.editor.record);
            }
        }
    }

    fn answer_picker(&mut self, record: usize) {
        let at = self.forms.options.last_at;
        let unit = self.unit_or_4();
        let Some(i) = self.forms.find(FormWindow::Picker) else {
            return;
        };
        let Some(state) = self.forms.options.picker.as_mut() else {
            return;
        };
        let form = &mut self.forms.forms[i].form;
        let rect = on_screen(form, &form.controls()[record]);
        let c = form.controls()[record].clone();
        let mut value = c.value;
        let mut point = (0.0, 0.0);
        if c.class == exa_ui::controls::SPINNER {
            // A release over the button column steps the value by the
            // spinner's step within its range.
            let button = exa_ui::controls::spinner_button_width(unit);
            let buttons = exa_ui::controls::spinner_buttons(
                (rect.left, rect.top, rect.width(), rect.height()),
                button,
                unit,
            );
            let (ux, uy, uw, uh) = buttons.up;
            let (_, dy, _, dh) = buttons.down;
            if at.0 >= ux && at.0 < ux + uw {
                if at.1 >= uy && at.1 < uy + uh {
                    value += colour_picker::SPIN_STEP;
                } else if at.1 >= dy && at.1 < dy + dh {
                    value -= colour_picker::SPIN_STEP;
                }
            } else {
                self.forms.options.focus = Some((FormWindow::Picker, record));
                return;
            }
            value = value.clamp(colour_picker::SPIN_MIN, colour_picker::SPIN_MAX);
        }
        if c.class == exa_ui::controls::IMAGE {
            let pick =
                exa_ui::controls::image_pick(c.w, c.h, c.margin, at.0 - rect.left, at.1 - rect.top);
            point = (pick.u, pick.v);
        }
        let bytes = state.swatch.colour;
        let Some(event) = colour_picker::record_event(record, value, point, bytes) else {
            return;
        };
        let refresh = state.picker.dispatch(event, Some(&mut state.swatch));
        info!(
            "colour picker: {event:?} -> colour {:?} (swatch {:?}, {refresh:?})",
            state.picker.preview, state.swatch.colour
        );
        if event == PickerEvent::Finish {
            self.forms.remove(FormWindow::Picker);
            self.forms.options.picker = None;
            return;
        }
        sync_picker(form, state);
    }

    /// A typed key for the colour picker's clicked spinner field: a
    /// printable byte up to the field's limit (a digits field drops
    /// anything else), Backspace takes the last byte off. (The text fields
    /// and combo boxes take their keys through their own handler,
    /// [`Self::field_key`].)
    pub(crate) fn type_key(&mut self, key: TypedKey) {
        let Some((window, record)) = self.forms.options.focus else {
            return;
        };
        let Some(form) = self.forms.get_mut(window) else {
            return;
        };
        let Some(spec) = form.spec.records.get(record).copied() else {
            return;
        };
        let max = if spec.max_len == 0 { 255 } else { spec.max_len } as usize;
        let style = form.controls()[record].style;
        let c = &mut form.geometry.controls[record];
        let mut edited = false;
        let mut submit = false;
        match key {
            TypedKey::Char(ch) => {
                let byte = ch as u32;
                if !(0x20..0x7f).contains(&byte) {
                    return;
                }
                if style & exa_ui::controls::text_style::DIGITS != 0 && !(ch as u8).is_ascii_digit()
                {
                    return;
                }
                if c.text.len() < max {
                    c.text.push(ch as u8);
                    edited = true;
                }
            }
            TypedKey::Backspace => edited = c.text.pop().is_some(),
            TypedKey::Enter => submit = true,
        }
        let notify = style & exa_ui::controls::text_style::NOTIFY != 0;
        if window == FormWindow::Emblem && ((edited && notify) || submit) {
            let bits = if submit { field_edit::SUBMITTED } else { 0 };
            self.emblem_text(record, bits);
        }
    }

    /// Tab on the focused hub form, with the key-down's mask word
    /// (`flags`, [`exa_ui::gui_keys`]): the base form's key slot
    /// ([`exa_ui::form::form_key`]) moves the form's focus from its focused
    /// record (the key focus, the picker's typing spinner, else the record
    /// under the pointer) to the next enabled tab stop, wrapping to the
    /// first tab stop before it; with the word's 0x10 it goes to the
    /// previous one, wrapping to the last. Shift held gives a key-down the
    /// word 0x20, so Shift with Tab steps forward too. A Tab with nowhere
    /// to go ends at the form. The focus taken this way selects a text field's or
    /// combo box's whole text and waits for the pointer to move before it
    /// follows the pointer again; a picker spinner it lands on takes the
    /// typing. Answers whether a form took the key.
    pub(crate) fn form_tab(&mut self, flags: u32) -> bool {
        use exa_ui::form::{ControlTab, FormKey, record_state, record_style};
        let focus = match self.forms.options.key_focus.or(self.forms.options.focus) {
            Some((window, record)) => self.forms.find(window).map(|i| (i, record)),
            None => self.forms.forms.len().checked_sub(1).and_then(|top| {
                self.forms.forms[top]
                    .form
                    .hovered()
                    .map(|record| (top, record))
            }),
        };
        let Some((i, record)) = focus else {
            return false;
        };
        let window = self.forms.forms[i].window;
        let controls = self.forms.forms[i].form.controls();
        let tabs: Vec<ControlTab> = controls
            .iter()
            .map(|c| ControlTab {
                tab_stop: c.style & record_style::TAB_STOP != 0,
                disabled: c.state & record_state::DISABLED != 0,
            })
            .collect();
        let next = match exa_ui::form::form_key(9, flags, record as i32 + 1, &tabs, |_| 0) {
            FormKey::Focus(next) => next,
            FormKey::Nothing | FormKey::Taken(_) => return true,
            FormKey::Parent => return false,
        };
        let c = controls[next].clone();
        if takes_keys(&c) {
            let mut view = view_of(&mut self.forms.options.edits, window, next, &c.text);
            field_edit::focus_gained(&c.text, &mut view);
            store_view(&mut self.forms.options.edits, window, next, view);
            self.forms.options.key_focus = Some((window, next));
        } else {
            self.forms.options.key_focus = None;
        }
        if window == FormWindow::Picker && c.class == exa_ui::controls::SPINNER {
            self.forms.options.focus = Some((window, next));
        }
        self.forms.options.focus_released = Some(self.forms.options.last_at);
        debug!("forms: Tab moves the focus to {window:?} record {next}");
        true
    }

    /// A key message for the focused text field or combo box: the
    /// control's own key handler through the edit unit, then its owner's
    /// callback with the state bits the handler sets. Answers the
    /// clipboard request.
    pub(crate) fn field_key(
        &mut self,
        career: &mut ArenaCareer,
        key: FieldKey,
        clipboard: Option<&[u8]>,
    ) -> Option<EditAction> {
        let (window, record) = self.forms.options.key_focus?;
        let i = self.forms.find(window)?;
        let unit = self.forms.forms[i].form.unit();
        let font = self.forms.field_font(unit);
        let has_callback = self.forms.forms[i].form.spec.records.get(record)?.reports;
        let c = self.forms.forms[i].form.controls().get(record)?.clone();
        let mut view = view_of(&mut self.forms.options.edits, window, record, &c.text);
        let mut text = c.text.clone();
        let max_len = match self.forms.forms[i].form.spec.records[record].max_len {
            0 => 255,
            n => n,
        };
        let words = FieldWords {
            text: &mut text,
            view: &mut view,
            style: c.style,
            max_len,
            has_callback,
        };
        let effect = if c.class == TEXT_FIELD {
            field_edit::text_field_key(&font, words, c.w, c.margin, unit, key, clipboard)
        } else {
            field_edit::combo_key(&font, words, c.w, c.margin, unit, key, clipboard)
        };
        self.forms.forms[i]
            .form
            .set_text(record, &String::from_utf8_lossy(&text));
        self.forms.forms[i].form.geometry.controls[record].text = text;
        store_view(&mut self.forms.options.edits, window, record, view);
        if let Some(bits) = effect.call {
            self.field_callback(career, window, record, bits);
        }
        if effect.release_focus {
            self.forms.options.key_focus = None;
            self.forms.options.focus_released = Some(self.forms.options.last_at);
            debug!("forms: Enter gives the focus up");
        }
        effect.clipboard
    }

    /// A field's owner callback, run with the state bits its key handler
    /// sets: the emblem name's preset on the submitted bit, a picture
    /// number's selection, the merchant's options re-stored from their
    /// selection words, the loadout list's row.
    fn field_callback(
        &mut self,
        career: &mut ArenaCareer,
        window: FormWindow,
        record: usize,
        bits: u32,
    ) {
        match window {
            FormWindow::Emblem => self.emblem_text(record, bits),
            FormWindow::Merchant => {
                let Some(form) = self.forms.get(window) else {
                    return;
                };
                let word = form.amounts.get(record).copied().unwrap_or(0);
                if record == merchant::LOOK_FOR {
                    self.forms.options.look_for_word = word;
                    career.merchant_kinds = arena_forms::look_for_kinds(word);
                } else if record == merchant::RANK {
                    career.merchant_rank = arena_forms::rank_of(word);
                }
                info!(
                    "hub trade: Enter on record {record} re-stores the merchant's look-for {:#x} and rank {}",
                    career.merchant_kinds, career.merchant_rank
                );
            }
            FormWindow::Loadout => self.loadout_list_entered(career, record),
            _ => {}
        }
    }

    /// The emblem fields' callbacks. The name's runs on every key (its
    /// notify style) but acts only with the submitted bit, Enter's: the
    /// name `klean` selects the preset. A picture number's runs on Enter
    /// and selects the number its text reads.
    fn emblem_text(&mut self, record: usize, bits: u32) {
        let Some(i) = self.forms.find(FormWindow::Emblem) else {
            return;
        };
        let Some(state) = self.forms.options.emblem.as_mut() else {
            return;
        };
        let form = &mut self.forms.forms[i].form;
        match emblem_action(record) {
            Some(EmblemAction::Name) => {
                if bits & field_edit::SUBMITTED == 0 {
                    return;
                }
                let text = String::from_utf8_lossy(&form.controls()[record].text).into_owned();
                if crate::emblem::apply_klean(&text, &mut state.editor) {
                    apply_emblem(form, &state.editor);
                    state.editor.refresh_colours();
                    info!("emblem: the name klean selects the preset");
                }
            }
            Some(EmblemAction::Number { design }) => {
                let n = emblem_form::number_value(&form.controls()[record].text);
                select_picture(form, &mut state.editor, design, n);
            }
            _ => {}
        }
    }
}

/// A field's edit bytes, made with its creation's when no key or press
/// reached it yet.
fn view_of(edits: &mut Vec<FieldEdit>, window: FormWindow, record: usize, text: &[u8]) -> EditView {
    match edits
        .iter()
        .find(|e| e.window == window && e.record == record)
    {
        Some(e) => e.view,
        None => {
            let view = field_edit::created_view(text);
            edits.push(FieldEdit {
                window,
                record,
                view,
            });
            view
        }
    }
}

fn store_view(edits: &mut [FieldEdit], window: FormWindow, record: usize, view: EditView) {
    if let Some(e) = edits
        .iter_mut()
        .find(|e| e.window == window && e.record == record)
    {
        e.view = view;
    }
}

/// Whether a record takes the form's keys as an editing control: a text
/// field or combo box without the no-keys style bit (0x100000).
fn takes_keys(c: &Control) -> bool {
    matches!(c.class, TEXT_FIELD | COMBO_BOX) && c.style & 0x10_0000 == 0
}

impl crate::hub_forms::HubForms {
    /// The main font at a unit: the one the paint stored, else the level's.
    pub(crate) fn field_font(&self, unit: i32) -> exa_ui::gui_font::GuiFont {
        self.options.font.unwrap_or_else(|| {
            *exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(unit)).main
        })
    }

    /// The focus follows the pointer, as the idle pointer's hit and focus
    /// does: the record under it in the form it targets. A text field or
    /// combo box that gains it gets the focus message, which selects its
    /// whole text. After a release the focus waits for the pointer to move.
    pub(crate) fn follow_key_focus(&mut self, target: usize, at: (i32, i32)) {
        if let Some(released) = self.options.focus_released {
            // A focus given before the pointer's next place is known keeps
            // that place as its own.
            if released == FOCUS_HELD_UNTIL_SEEN {
                self.options.focus_released = Some(at);
                return;
            }
            if released == at {
                return;
            }
            self.options.focus_released = None;
        }
        let Some(open) = self.forms.get(target) else {
            return;
        };
        let window = open.window;
        let next = open
            .form
            .hovered()
            .filter(|&i| open.form.controls().get(i).is_some_and(takes_keys))
            .map(|i| (window, i));
        if next == self.options.key_focus {
            return;
        }
        self.options.key_focus = next;
        if let Some((window, record)) = next {
            let text = self.forms[target].form.controls()[record].text.clone();
            let mut view = view_of(&mut self.options.edits, window, record, &text);
            field_edit::focus_gained(&text, &mut view);
            store_view(&mut self.options.edits, window, record, view);
            debug!("forms: the focus is on {window:?} record {record}");
        }
    }

    /// A focused text field's screen rectangle, pad, unit and words.
    fn focused_field(&self, target: usize) -> Option<(FormWindow, usize, Control, (i32, i32))> {
        let (window, record) = self.options.key_focus?;
        let open = self.forms.get(target)?;
        if open.window != window {
            return None;
        }
        let c = open.form.controls().get(record)?.clone();
        (c.class == TEXT_FIELD).then_some((window, record, c, open.form.origin()))
    }

    /// A left press on the focused text field in its hovered state: the
    /// caret goes under the pointer with the anchor (the press's 0x40).
    pub(crate) fn field_press(&mut self, target: usize, at: (i32, i32)) {
        let Some((window, record, c, origin)) = self.focused_field(target) else {
            return;
        };
        if !field_edit::press_places_caret(1, c.state) {
            return;
        }
        let unit = self.forms[target].form.unit();
        let font = self.field_font(unit);
        let mut view = view_of(&mut self.options.edits, window, record, &c.text);
        let rect = (origin.0 + c.x, origin.1 + c.y, c.w, c.h);
        let at_x =
            field_edit::text_field_origin(rect, c.margin, unit, c.style, &font, &c.text, &view).x;
        let span = exa_ui::controls::text_field_span(c.w, c.margin, unit);
        field_edit::press_caret(&font, &c.text, &mut view, at_x, at.0, span);
        store_view(&mut self.options.edits, window, record, view);
        debug!(
            "forms: caret at {} in {window:?} record {record}",
            view.caret
        );
    }

    /// The pointer moved while the left button holds the focused text
    /// field: the caret follows it and the selection grows (0x30).
    pub(crate) fn field_drag(&mut self, target: usize, at: (i32, i32)) {
        let Some((window, record, c, origin)) = self.focused_field(target) else {
            return;
        };
        if self.forms[target].form.held() != Some(record) {
            return;
        }
        let unit = self.forms[target].form.unit();
        let font = self.field_font(unit);
        let mut view = view_of(&mut self.options.edits, window, record, &c.text);
        let rect = (origin.0 + c.x, origin.1 + c.y, c.w, c.h);
        let at_x =
            field_edit::text_field_origin(rect, c.margin, unit, c.style, &font, &c.text, &view).x;
        let span = exa_ui::controls::text_field_span(c.w, c.margin, unit);
        field_edit::drag_caret(&font, &c.text, &mut view, at_x, at.0, span);
        store_view(&mut self.options.edits, window, record, view);
    }

    /// A left press on a colour swatch: held until the pointer moves past
    /// the drag threshold, when it starts the colour drag.
    pub(crate) fn swatch_press(&mut self, target: usize, at: (i32, i32)) {
        let Some(open) = self.forms.get(target) else {
            return;
        };
        let Some(record) = open.form.held() else {
            return;
        };
        if open.form.controls().get(record).map(|c| c.class) != Some(COLOUR_SWATCH) {
            return;
        }
        self.options.held_swatch = Some(HeldSwatch {
            window: open.window,
            record,
            at,
            clock: self.options.clock,
        });
    }

    /// With the button held on a swatch, a move past the pointer's drag
    /// threshold sends the swatch the drag press (0x4001): it starts the
    /// colour drag with its colour, the press's offset into it and its
    /// inner size, and the form lets the press go.
    pub(crate) fn swatch_drag(&mut self, target: usize, at: (i32, i32), screen_h: i32) {
        let Some(held) = self.options.held_swatch else {
            return;
        };
        let elapsed = (self.options.clock - held.clock)
            .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        let limit = exa_ui::gui_core::drag_limit(screen_h);
        let threshold = i64::from(exa_ui::gui_core::drag_threshold(elapsed, limit));
        let (dx, dy) = (
            (i64::from(at.0) - i64::from(held.at.0)).abs(),
            (i64::from(at.1) - i64::from(held.at.1)).abs(),
        );
        if dx <= threshold && dy <= threshold {
            return;
        }
        self.options.held_swatch = None;
        let Some(open) = self.forms.get_mut(target) else {
            return;
        };
        if open.window != held.window {
            return;
        }
        let Some(c) = open.form.controls().get(held.record).cloned() else {
            return;
        };
        let (ox, oy) = open.form.origin();
        let colour = u32::from_le_bytes(c.colour.unwrap_or([0; 4]));
        let press = exa_ui::controls::swatch_press(
            0x4001,
            colour,
            held.at,
            (ox + c.x, oy + c.y, c.w, c.h),
            c.margin,
        );
        if let exa_ui::controls::SwatchPress::Drag(drag) = press {
            open.form.cancel_hold();
            info!(
                "forms: {:?} record {} drags colour {colour:#010x}",
                held.window, held.record
            );
            self.options.dragging = Some(ColourDragging {
                window: held.window,
                record: held.record,
                drag,
            });
        }
    }

    /// The release ends a colour drag: the swatch under the pointer that
    /// accepts the colour code takes the colour and runs its drop
    /// callback (the emblem swatches': the colour slot takes the word).
    /// Answers whether a drag ended.
    pub(crate) fn drop_colour(&mut self, target: usize, at: (i32, i32)) -> bool {
        let Some(drag) = self.options.dragging.take() else {
            return false;
        };
        let Some(open) = self.forms.get_mut(target) else {
            return true;
        };
        let window = open.window;
        let hit = open.form.hit(at.0, at.1);
        let Some(record) = hit.filter(|&i| {
            open.form
                .controls()
                .get(i)
                .is_some_and(|c| c.class == COLOUR_SWATCH)
                && exa_ui::controls::swatch_accepts(exa_ui::controls::COLOUR_DRAG_CODE)
        }) else {
            info!("forms: the colour drag is dropped on nothing");
            return true;
        };
        // Only the emblem's swatches carry a drop callback in the forms
        // the hub builds.
        if window != FormWindow::Emblem {
            info!("forms: {window:?} record {record} has no drop callback");
            return true;
        }
        let mut word = open.form.controls()[record]
            .colour
            .map_or(0, u32::from_le_bytes);
        field_edit::swatch_drop(&mut word, drag.drag.colour);
        open.form.geometry.controls[record].colour = Some(word.to_le_bytes());
        if let (Some(slot), Some(state)) = (
            emblem_records::SWATCHES.iter().position(|&r| r == record),
            self.options.emblem.as_mut(),
        ) {
            field_edit::emblem_drop(&mut state.editor.record.colours, slot, word);
            info!(
                "emblem: colour {} takes the dropped {word:#010x} (palette {:#04x})",
                slot + 1,
                word >> 24
            );
        }
        true
    }
}

/// A key typed into a text field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypedKey {
    Char(char),
    Backspace,
    Enter,
}

/// Logs every record's screen rectangle (`RUST_LOG=exa_game::option_forms=debug`),
/// for scripted clicks.
fn log_records(name: &str, form: &RecordForm) {
    for (i, c) in form.controls().iter().enumerate() {
        let r = on_screen(form, c);
        debug!(
            "{name} record {i} (class {:#x}, {:?}): ({}, {}) {}x{}, centre ({}, {})",
            c.class,
            String::from_utf8_lossy(&c.text),
            r.left,
            r.top,
            r.width(),
            r.height(),
            r.left + r.width() / 2,
            r.top + r.height() / 2
        );
    }
}

/// A combo box's selection: its word and the item it shows.
fn set_combo(form: &mut RecordForm, record: usize, row: u32) {
    if let Some(amount) = form.amounts.get_mut(record) {
        *amount = row;
    }
    let item = form.spec.records[record]
        .items
        .get((row as usize).wrapping_sub(1))
        .copied();
    if let Some(item) = item {
        form.set_text(record, item);
    }
}

/// The form's apply step: each swatch shows its colour slot's palette
/// index on the lifted ramp (the index in the alpha byte), and each mirror
/// box is on and lit while its layer bit is set. The box's word is the
/// label record's after it, with the on and lit bits set or cleared, as
/// the original reads it.
fn apply_emblem(form: &mut RecordForm, editor: &EmblemEditor) {
    for (slot, &record) in emblem_records::SWATCHES.iter().enumerate() {
        let colour = emblem_form::swatch_colour(editor.record.palette(slot));
        if let Some(c) = form.geometry.controls.get_mut(record) {
            c.colour = Some(colour);
        }
    }
    for (record, bit) in [
        (emblem_records::BACKGROUND_MIRROR, emblem_form::MIRROR_X),
        (emblem_records::DESIGN_MIRROR, emblem_form::MIRROR_Y),
    ] {
        let next = form.controls().get(record + 1).map_or(0, |c| c.state);
        let on = state::ON | state::LIT;
        let word = if editor.record.layers & bit != 0 {
            next | on
        } else {
            next & !on
        };
        if let Some(c) = form.geometry.controls.get_mut(record) {
            c.state = word;
        }
    }
}

/// The background or design setter: the selector's rule on the number,
/// the step buttons' disabled bits, the number field's text and the
/// record's index.
fn select_picture(form: &mut RecordForm, editor: &mut EmblemEditor, design: bool, index: i32) {
    use emblem_records::*;
    let (max, down, up, field) = if design {
        (
            emblem_form::DESIGN_MAX,
            DESIGN_DOWN,
            DESIGN_UP,
            DESIGN_NUMBER,
        )
    } else {
        (
            emblem_form::BACKGROUND_MAX,
            BACKGROUND_DOWN,
            BACKGROUND_UP,
            BACKGROUND_NUMBER,
        )
    };
    let states = emblem_form::step_states(index, max);
    if let Some(disabled) = states.down_disabled {
        form.set_bits(down, state::DISABLED, disabled);
    }
    if let Some(disabled) = states.up_disabled {
        form.set_bits(up, state::DISABLED, disabled);
    }
    if design {
        editor.record.design = states.index as u32;
    } else {
        editor.record.background = states.index as u32;
    }
    form.set_text(field, &states.index.to_string());
    info!(
        "emblem: {} {}",
        if design { "design" } else { "background" },
        states.index
    );
}

/// Writes the picker's refresh into its records: the trackbars' values
/// (0–1) and the spinners' (0–255) for the RGB and HSV channels, and the
/// two swatches (the colour it opened with and the new one).
fn sync_picker(form: &mut RecordForm, state: &PickerState) {
    let p = &state.picker;
    let controls = &mut form.geometry.controls;
    for channel in 0..3 {
        controls[picker_records::RGB_BARS[channel]].value = p.rgb_sliders[channel];
        controls[picker_records::RGB_SPINS[channel]].value = p.rgb_fields[channel];
        controls[picker_records::HSV_BARS[channel]].value = p.hsv_sliders[channel];
        controls[picker_records::HSV_SPINS[channel]].value = p.hsv_fields[channel];
    }
    controls[picker_records::OLD_COLOUR].colour = Some(colour_picker::packed_bytes(p.original));
    controls[picker_records::NEW_COLOUR].colour = Some(p.preview);
    let unit = form.unit();
    // The trackbars' knobs follow their values.
    for &record in picker_records::RGB_BARS
        .iter()
        .chain(picker_records::HSV_BARS.iter())
    {
        let mut events = Vec::new();
        let value = form.geometry.controls[record].value;
        exa_ui::gui_controls::dispatch(
            &mut form.geometry.controls,
            record,
            exa_ui::gui_controls::Message {
                message: exa_ui::gui_controls::message::SET,
                param: value.to_bits(),
                extra: 0x10,
                ..Default::default()
            },
            unit,
            &mut events,
        );
    }
}

/// The hue bar's gradient: red, yellow, green, cyan, blue, magenta and red
/// again, the constructor's six-step table.
pub const HUE_STOPS: [[u8; 4]; 7] = [
    [0xff, 0, 0, 0xff],
    [0xff, 0xff, 0, 0xff],
    [0, 0xff, 0, 0xff],
    [0, 0xff, 0xff, 0xff],
    [0, 0, 0xff, 0xff],
    [0xff, 0, 0xff, 0xff],
    [0xff, 0, 0, 0xff],
];

/// The gradient a picker trackbar shows: the RGB channels' two ends
/// ([`ColourPicker::ramps`]), the hue table, and the saturation and value
/// ends (the picker's field corners).
pub fn picker_gradient(picker: &ColourPicker, record: usize) -> Option<Vec<[u8; 4]>> {
    if let Some(channel) = picker_records::RGB_BARS.iter().position(|&r| r == record) {
        return Some(picker.ramps[channel].to_vec());
    }
    match record {
        r if r == picker_records::HSV_BARS[0] => Some(HUE_STOPS.to_vec()),
        r if r == picker_records::HSV_BARS[1] => Some(picker.corners[..2].to_vec()),
        r if r == picker_records::HSV_BARS[2] => Some(picker.corners[2..].to_vec()),
        _ => None,
    }
}

/// A vertical shade over a rectangle, top ink to bottom ink.
fn shade_layer(rect: GuiRect, top: [u8; 4], bottom: [u8; 4]) -> exa_ui::gui_draw::GuiLayer {
    let mut batch = exa_ui::skin::SkinBatch::new();
    batch.select(1, 0, false);
    batch.begin();
    let (x0, x1) = (rect.left as f32, (rect.left + rect.width()) as f32);
    let (y0, y1) = (rect.top as f32, (rect.top + rect.height()) as f32);
    batch.set_ink(top);
    batch.vertex(x0, y0);
    batch.set_ink(bottom);
    batch.vertex(x0, y1);
    batch.vertex(x1, y1);
    batch.set_ink(top);
    batch.vertex(x1, y0);
    batch.end_quads();
    exa_ui::gui_draw::GuiLayer::new(batch, exa_ui::gui_draw::LayerTexture::None)
}

/// How a text field draws: whether it has the focus (its selection shows
/// and its caret can), its edit bytes, and the form's caret bit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FieldDraw {
    pub focused: bool,
    pub view: Option<EditView>,
    pub caret_lit: bool,
}

/// A text field's front draw: the slot `0xe4` ink bound, the edit unit's
/// run at the field's own text origin (the selection shown while it has
/// the focus, the whole-run ink while disabled, the state's text ink 6
/// beside a selection), then the caret glyph where the caret is while
/// the field has the focus and the form's caret bit is lit.
fn paint_text_field_front(
    painted: &mut Painted,
    c: &Control,
    rect: GuiRect,
    look: &Look,
    draw: FieldDraw,
) {
    use exa_ui::text_edit::{EditInk, caret_position, draw_edit, scroll_view};
    let (font, size) = crate::hub_windows::screen_font(look.textures, look.unit);
    let Some(texture) = size else {
        return;
    };
    let unit = look.unit;
    let span = exa_ui::controls::text_field_span(c.w, c.margin, unit);
    let view = draw.view.unwrap_or_else(|| {
        let mut view = field_edit::created_view(&c.text);
        scroll_view(&font, &c.text, &mut view, span);
        view
    });
    let place = (rect.left, rect.top, rect.width(), rect.height());
    let at = field_edit::text_field_origin(place, c.margin, unit, c.style, &font, &c.text, &view);
    let palette = &look.palette;
    let edit = draw_edit(
        &font,
        &c.text,
        &view,
        at.x,
        at.y,
        draw.focused,
        c.state & state::DISABLED != 0,
        6,
        palette.highlight[4],
    );
    if let Some(([x, y, w, h], ink)) = edit.selection {
        painted.push(exa_ui::loadout::solid_layer([x, y, w, h], ink));
    }
    let slot = crate::gui_paint::text_slot;
    for run in &edit.runs {
        let ink = match run.ink {
            EditInk::Current => slot(palette, 0xe4),
            EditInk::Text(index) => slot(palette, 0xcc + (index as u8) * 4),
            EditInk::Selected => slot(palette, 0xf4),
            EditInk::Whole => slot(palette, 0xd8),
        };
        if run.quads.is_empty() {
            continue;
        }
        painted.push(exa_ui::gui_draw::GuiLayer::new(
            exa_ui::gui_draw::text_batch(&font, &run.quads, ink, texture),
            exa_ui::gui_draw::LayerTexture::Named(font.texture),
        ));
    }
    if draw.focused && draw.caret_lit {
        let (x, y) = caret_position(&font, &c.text, &view, at.x, at.y);
        painted.push(crate::gui_paint::caption_layer(
            &font,
            texture,
            &[exa_ui::text_layout::CARET_GLYPH],
            (x, y),
            slot(palette, 0xe4),
        ));
    }
}

/// The colour a swatch drags, drawn under the pointer: its rectangle at
/// the press's offset, in the colour at alpha 0xa0 through the solid
/// program.
pub(crate) fn paint_colour_drag(painted: &mut Painted, dragging: &ColourDragging, at: (i32, i32)) {
    let (x, y, w, h) = exa_ui::controls::colour_drag_rect(at, &dragging.drag);
    let [r, g, b, _] = dragging.drag.colour.to_le_bytes();
    painted.push(exa_ui::loadout::solid_layer([x, y, w, h], [r, g, b, 0xa0]));
}

/// Paints an extended-class record the table-built forms carry beside the
/// basic classes: a text field's frame and its edit run (its caret while
/// it has the focus), a colour swatch's colour under its frame, a
/// spinner's field, value and two buttons, and an image's shade. Answers
/// whether it painted.
pub(crate) fn paint_extended(
    painted: &mut Painted,
    c: &Control,
    rect: GuiRect,
    look: &Look,
    draw: FieldDraw,
) -> bool {
    use exa_ui::controls::{IMAGE, SPINNER};
    match c.class {
        TEXT_FIELD => {
            // The field's back (the line skin's underline for a framed
            // style), then its front draw.
            if let Some(layer) =
                crate::gui_paint::paint_text_field_back(rect, c.style, look.unit, look.fade)
            {
                painted.push(layer);
            }
            paint_text_field_front(painted, c, rect, look, draw);
            true
        }
        COLOUR_SWATCH => {
            paint_swatch_back(painted, c, rect, look);
            true
        }
        SPINNER => {
            let button = exa_ui::controls::spinner_button_width(look.unit);
            let buttons = exa_ui::controls::spinner_buttons(
                (rect.left, rect.top, rect.width(), rect.height()),
                button,
                look.unit,
            );
            let field = GuiRect::new(
                rect.left,
                rect.top,
                (rect.width() - button).max(1),
                rect.height(),
            );
            for (x, y, w, h) in [buttons.up, buttons.down] {
                crate::hub_windows::paint_panel(painted, GuiRect::new(x, y, w, h), look);
            }
            // The field to the button column, framed with the draw skin's
            // style frames (the spinner is only on the base-skinned colour
            // picker), with the value in the spinner's own alignment.
            let mut frames = exa_ui::skin::SkinBatch::new();
            exa_ui::skin::draw_style_frames(
                &mut frames,
                exa_ui::skin::GuiRect::sized(
                    field.left as f32,
                    field.top as f32,
                    field.width() as f32,
                    field.height() as f32,
                ),
                0xf0 | (c.style & 0x7700),
                &look.palette,
            );
            painted.push(exa_ui::gui_draw::GuiLayer::new(
                frames,
                exa_ui::gui_draw::LayerTexture::Named(exa_ui::gui_draw::SKIN_TEXTURE),
            ));
            let mut value = c.clone();
            value.class = 5;
            value.w = field.width();
            value.style = c.style & 0x7700;
            value.text = format!("{}", exa_ui::controls::round_to_i64(c.value)).into_bytes();
            let (font, size) = crate::hub_windows::screen_font(look.textures, look.unit);
            let text = crate::settings_form::Text { font: &font, size };
            crate::settings_form::paint_control(
                &value,
                (rect.left - c.x, rect.top - c.y),
                look.unit,
                &look.palette,
                &text,
                None,
                painted,
            );
            true
        }
        IMAGE => {
            let (x, y, w, h) = exa_ui::controls::image_shade(
                (rect.left, rect.top, rect.width(), rect.height()),
                c.margin,
            );
            crate::hub_windows::paint_panel(painted, rect, look);
            painted.push(shade_layer(
                GuiRect::new(x, y, w, h),
                exa_ui::controls::IMAGE_SHADE_TOP,
                exa_ui::controls::IMAGE_SHADE_BOTTOM,
            ));
            true
        }
        _ => false,
    }
}

/// A colour swatch's back: its draw message calls the current skin's
/// control fill. The draw skin's own instance (the colour picker) draws
/// the style frames and the colour inset by the margin
/// ([`exa_ui::skin::draw_control_fill`]); the line skin (the emblem form)
/// its shade: six atlas quads a unit either side of the top and bottom
/// edges in the on ink (the highlight ink under the pointer), then the
/// colour a unit in ([`exa_ui::skin::draw_line_shade`]). Both take the
/// GUI's alpha.
fn paint_swatch_back(painted: &mut Painted, c: &Control, rect: GuiRect, look: &Look) {
    use exa_ui::gui_draw::{GuiLayer, LayerTexture, SKIN_TEXTURE};
    use exa_ui::skin::SkinBatch;
    let colour = c.colour.unwrap_or_default();
    let alpha = exa_ui::line_skin::hover_alpha(look.fade);
    if look.base_skin {
        let mut batch = SkinBatch::new();
        exa_ui::skin::draw_control_fill(
            &mut batch,
            exa_ui::skin::GuiRect::sized(
                rect.left as f32,
                rect.top as f32,
                rect.width() as f32,
                rect.height() as f32,
            ),
            c.style,
            c.margin,
            colour,
            alpha,
            &look.palette,
        );
        painted.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
        return;
    }
    let shade = exa_ui::skin::draw_line_shade(
        look.unit,
        exa_ui::skin::LineShadePlace {
            x: rect.left as i16,
            y: rect.top as i16,
            w: rect.width() as i16,
            h: rect.height() as i16,
            state: c.state,
            inner_ink: colour,
            side_inset: look.unit * 2,
            alpha,
        },
        exa_ui::skin::LineShadeInks {
            normal: exa_ui::line_skin::line_control_inks(look.fade)[2],
            special: look.palette.highlight[7],
        },
        exa_ui::skin::SPAN_VIEW_UNSET,
    );
    let mut batch = exa_ui::skin::line_skin_batch();
    batch.set_ink(shade.outer_ink);
    for quad in &shade.outer {
        let uv = quad
            .sample
            .map(|s| s.map(f32::from_bits))
            .unwrap_or_default();
        for (k, (x, y)) in quad.corners.iter().enumerate() {
            batch.set_uv(uv[k * 2], uv[k * 2 + 1]);
            batch.vertex(*x, *y);
        }
    }
    batch.end_quads();
    painted.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
    let mut fill = SkinBatch::new();
    fill.select(1, 0, false);
    fill.set_ink(shade.inner.ink);
    fill.begin();
    for (x, y) in shade.inner.corners {
        fill.vertex(x, y);
    }
    fill.end_quads();
    painted.push(GuiLayer::new(fill, LayerTexture::None));
}

/// The line skin's form draw for a popup that took its owner's line skin
/// (the drop-down list and the palette swatch popup both copy their
/// opener's form skin): its words as its show commits them, the shadow
/// and the backdrop under its content, the wide window frame over it.
fn popup_frame(rect: GuiRect, look: &Look, style: u32, attributes: u32) -> LineFormLayers {
    crate::gui_paint::paint_top_form(
        rect,
        look.unit,
        &look.palette,
        look.fade,
        crate::gui_paint::FormWords { style, attributes },
        None,
    )
    .0
}

/// Paints a drop-down list at `rect` through the line skin's form draw
/// with the list's own words: its shadow and backdrop, the selection band,
/// the rows (the selected one in the skin's eighth label ink, the others
/// in the seventh) and the wide window frame over them.
pub(crate) fn paint_drop_list(
    painted: &mut Painted,
    list: &DropDownList,
    rect: Rect,
    (unit, fade): (i32, f32),
    palette: &exa_ui::skin::SkinPalette,
    (font, size): (&exa_ui::gui_font::GuiFont, Option<(u16, u16)>),
) {
    let r = GuiRect::new(rect.x, rect.y, rect.w, rect.h);
    let mut frame = crate::gui_paint::paint_top_form(
        r,
        unit,
        palette,
        fade,
        crate::gui_paint::FormWords {
            style: list.style,
            attributes: list.attributes,
        },
        None,
    )
    .0;
    painted.layers.extend(frame.under());
    let mut batch = exa_ui::skin::SkinBatch::new();
    list.paint(&mut batch, palette, 0);
    crate::gui_paint::offset_batch(&mut batch, rect.x, rect.y);
    painted.push(exa_ui::gui_draw::GuiLayer::new(
        batch,
        exa_ui::gui_draw::LayerTexture::Named(exa_ui::gui_draw::SKIN_TEXTURE),
    ));
    if let Some(texture) = size {
        for row in list.rows(font) {
            let Some(item) = list.items.get((row.row - 1) as usize) else {
                continue;
            };
            let ink = if row.selected {
                palette.text[7]
            } else {
                palette.text[6]
            };
            painted.push(crate::gui_paint::caption_layer(
                font,
                texture,
                item,
                (rect.x + row.pen.0, rect.y + row.pen.1),
                ink,
            ));
        }
    }
    painted.layers.extend(frame.over());
}

/// Paints the open popup over the forms through the line skin's form
/// draw: a drop-down list's shadow, backdrop, selection band and rows and
/// wide window frame, or the palette swatch popup's, with its cells and
/// its selection's frame.
pub(crate) fn paint_popup(painted: &mut Painted, popup: &Popup, look: &Look) {
    match popup {
        Popup::List { list, rect, .. } => {
            let (font, size) = crate::hub_windows::screen_font(look.textures, look.unit);
            paint_drop_list(
                painted,
                list,
                *rect,
                (look.unit, look.fade),
                &look.palette,
                (&font, size),
            );
        }
        Popup::Palette { swatch, rect, .. } => {
            let r = GuiRect::new(rect.x, rect.y, rect.w, rect.h);
            let mut frame = popup_frame(
                r,
                look,
                col_swatch::SWATCH_STYLE_BITS,
                col_swatch::SWATCH_ATTRIBUTES,
            );
            painted.layers.extend(frame.under());
            let unit = look.unit;
            let (ox, oy) = (rect.x + col_swatch::BORDER, rect.y + col_swatch::BORDER);
            let mut batch = exa_ui::skin::SkinBatch::new();
            batch.select(1, 0, false);
            batch.begin();
            for cell in col_swatch::cells(unit) {
                let [l, t, rr, b] = cell.rect;
                batch.set_ink(cell.colour);
                batch.vertex(ox as f32 + l, oy as f32 + t);
                batch.vertex(ox as f32 + l, oy as f32 + b);
                batch.vertex(ox as f32 + rr, oy as f32 + b);
                batch.vertex(ox as f32 + rr, oy as f32 + t);
            }
            batch.end_quads();
            painted.push(exa_ui::gui_draw::GuiLayer::new(
                batch,
                exa_ui::gui_draw::LayerTexture::None,
            ));
            let (outer, inner) = col_swatch::selection_frames(swatch.index, unit);
            // The skin's two frame inks (which accent is which is not
            // traced).
            for (frame, ink) in [
                (outer, look.palette.accent_bright),
                (inner, look.palette.accent_dim),
            ] {
                let [l, t, rr, b] = frame;
                let (l, t, rr, b) = (l + ox, t + oy, rr + ox, b + oy);
                for edge in [
                    [l, t, rr - l, 1],
                    [l, b - 1, rr - l, 1],
                    [l, t, 1, b - t],
                    [rr - 1, t, 1, b - t],
                ] {
                    painted.push(exa_ui::loadout::solid_layer(edge, ink));
                }
            }
            painted.layers.extend(frame.over());
        }
    }
}

/// The emblem form's preview: the emblem shader's layer at the form's
/// preview quad, in the form's client.
pub(crate) fn paint_emblem_preview(
    painted: &mut Painted,
    form: &RecordForm,
    state: &EmblemState,
    look: &Look,
) {
    let Some(mut layer) = crate::emblem::preview_layer(&state.editor, look.textures, look.unit)
    else {
        return;
    };
    let (ox, oy) = form.origin();
    crate::gui_paint::offset_batch(&mut layer.batch, ox, oy);
    painted.push(layer);
}

/// The textures the emblem preview samples: the record's background and
/// design and the two shared layers, loaded once each; with the Company
/// Status form open, the company emblem's too.
pub fn load_emblem_textures(
    windows: Option<Res<HubWindows>>,
    textures: Option<ResMut<exa_ui::gui_draw::GuiTextures>>,
    game: Option<Res<exa_assets::Game>>,
    career: Option<Res<crate::hub_windows::ArenaCareer>>,
) {
    let (Some(windows), Some(mut textures)) = (windows, textures) else {
        return;
    };
    if let Some(state) = windows.forms.options.emblem.as_ref() {
        for name in exa_ui::emblem_form::preview_textures(&state.editor.record) {
            crate::creator_screen::ensure_texture(&mut textures, game.as_deref(), &name);
        }
    }
    if windows
        .forms
        .get(crate::hub_forms::FormWindow::Status)
        .is_some()
        && let Some(career) = career.as_deref()
    {
        for name in exa_ui::emblem_form::preview_textures(&career.company.emblem) {
            crate::creator_screen::ensure_texture(&mut textures, game.as_deref(), &name);
        }
    }
}

/// The edit keys among the virtual keys a key-down sends (the codes the
/// edit unit switches on); the rest the text field passes over.
fn edit_key_byte(vk: u32) -> Option<u8> {
    use exa_ui::text_edit::key;
    let byte = u8::try_from(vk).ok()?;
    matches!(
        byte,
        key::BACKSPACE
            | key::DELETE
            | key::LEFT
            | key::RIGHT
            | key::HOME
            | key::END
            | field_edit::ENTER
            | key::COPY
            | key::PASTE
            | key::CUT
    )
    .then_some(byte)
}

/// The keys of the frame as the window procedure sends them: from the
/// frame's key list, else (an app without it) from the keyboard messages
/// and the scripted presses, with Shift 0x20 and Ctrl 0x40 in the word.
fn frame_keys(
    frame: Option<&crate::gui_keys::GuiKeyFrame>,
    events: &[KeyboardInput],
    keys: &ButtonInput<KeyCode>,
) -> Vec<GuiKey> {
    if let Some(frame) = frame {
        return frame.keys.clone();
    }
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let word =
        u32::from(shift) * exa_ui::gui_keys::SHIFT + u32::from(ctrl) * exa_ui::gui_keys::CTRL;
    let mut out = Vec::new();
    let mut sent: Vec<KeyCode> = Vec::new();
    for event in events.iter().filter(|e| e.state.is_pressed()) {
        sent.push(event.key_code);
        if let Some(vk) = crate::bindings::virtual_key(event.key_code) {
            out.push(GuiKey {
                key: vk,
                flags: word,
            });
        }
        if ctrl {
            continue;
        }
        let text = match &event.logical_key {
            Key::Character(text) => text.to_string(),
            Key::Space => " ".to_string(),
            _ => continue,
        };
        out.extend(
            text.chars()
                .filter_map(|c| u8::try_from(c).ok())
                .filter_map(|b| exa_ui::gui_keys::KeyMask::typed(u32::from(b))),
        );
    }
    for code in keys.get_just_pressed() {
        if !sent.contains(code)
            && let Some(vk) = crate::bindings::virtual_key(*code)
        {
            out.push(GuiKey {
                key: vk,
                flags: word,
            });
        }
    }
    out
}

/// Keys reach the hub's forms as the window procedure sends them (the
/// key-down with its word, then the character it typed with the word 2):
/// Tab is the form's own (its key slot moves the focus among the tab
/// stops); any other key is the focused record's key message (0xa0): the
/// focused text field or combo box (the record under the pointer) takes a
/// key-down's edit key, with Ctrl and Shift as the word holds them, and a
/// typed character; Ctrl with C, X or V asks the system clipboard. The
/// colour picker's clicked spinner keeps its own typing
/// ([`HubWindows::type_key`]). A message box on top takes the keys instead.
/// A key-down no record took (not a repeat or a typed character) climbs
/// the base forms' key slots to the root task ([`crate::gui_keys::TaskKeys`])
/// while a hub window is open (else the game view's key slot sends it).
#[allow(clippy::too_many_arguments)]
pub fn hub_form_typing(
    mut typed: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut windows: ResMut<HubWindows>,
    mut career: ResMut<ArenaCareer>,
    textures: Option<Res<exa_ui::gui_draw::GuiTextures>>,
    clipboard: Option<ResMut<crate::os_clipboard::SystemClipboard>>,
    gui_keys: Option<Res<crate::gui_keys::GuiKeyFrame>>,
    mut climbed: Option<ResMut<crate::gui_keys::TaskKeys>>,
) {
    let events: Vec<KeyboardInput> = typed.read().cloned().collect();
    if let Some(textures) = textures.as_deref() {
        let unit = if windows.unit > 0 { windows.unit } else { 4 };
        windows.forms.options.font = Some(crate::hub_windows::screen_font(textures, unit).0);
    }
    if windows.forms.message_on_top() {
        return;
    }
    let frame = frame_keys(gui_keys.as_deref(), &events, &keys);
    let mut clipboard = clipboard;
    let open = windows.is_open();
    // The base form's key slot passes a key nobody took to the parent,
    // unless its word has a low bit, up to the task.
    let mut climb = |key: GuiKey| {
        if open
            && key.flags & 3 == 0
            && let Some(climbed) = climbed.as_deref_mut()
        {
            climbed.forward(key);
        }
    };
    for key in frame {
        let typed = key.flags & exa_ui::gui_keys::TYPED != 0;
        // Tab is the form's key: the fields never hear it.
        if !typed && key.key == 9 {
            windows.form_tab(key.flags);
            continue;
        }
        if windows.forms.options.key_focus.is_some() {
            let press = if typed {
                let byte = u8::try_from(key.key)
                    .ok()
                    .filter(|b| (b' '..=b'~').contains(b));
                byte.map(|byte| FieldKey {
                    byte,
                    typed: true,
                    ctrl: false,
                    shift: false,
                })
            } else {
                let ctrl = key.flags & exa_ui::gui_keys::CTRL != 0;
                let shift = key.flags & exa_ui::gui_keys::SHIFT != 0;
                edit_key_byte(key.key)
                    .filter(|&b| ctrl || !matches!(b, 0x43 | 0x56 | 0x58))
                    .map(|byte| FieldKey {
                        byte,
                        typed: false,
                        ctrl,
                        shift,
                    })
            };
            let Some(press) = press else {
                climb(key);
                continue;
            };
            let pasted =
                (press.ctrl && !press.typed && press.byte == exa_ui::text_edit::key::PASTE)
                    .then(|| clipboard.as_deref_mut().and_then(|c| c.read()))
                    .flatten();
            let action = windows.field_key(&mut career, press, pasted.as_deref());
            if let Some(EditAction::Copy(bytes) | EditAction::Cut(bytes)) = action
                && let Some(c) = clipboard.as_deref_mut()
            {
                c.put(&bytes);
            }
            continue;
        }
        if windows.forms.options.focus.is_none() {
            climb(key);
            continue;
        }
        if typed {
            if let Ok(byte) = u8::try_from(key.key) {
                windows.type_key(TypedKey::Char(char::from(byte)));
            }
        } else if key.flags & exa_ui::gui_keys::REPEAT == 0 {
            match key.key {
                0x08 => windows.type_key(TypedKey::Backspace),
                0x0d => windows.type_key(TypedKey::Enter),
                _ => climb(key),
            }
        }
    }
}

/// The record a form's report came from is one of these windows'.
pub fn is_option_window(window: FormWindow) -> bool {
    matches!(
        window,
        FormWindow::Merchant | FormWindow::Company | FormWindow::Emblem | FormWindow::Picker
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn windows() -> HubWindows {
        let mut windows = HubWindows::default();
        windows.task.screen = (1280, 720);
        windows.unit = 4;
        windows
    }

    fn centre(form: &RecordForm, record: usize) -> (i32, i32) {
        let c = &form.controls()[record];
        let (ox, oy) = form.origin();
        (ox + c.x + c.w / 2, oy + c.y + c.h / 2)
    }

    fn click(windows: &mut HubWindows, career: &mut ArenaCareer, at: (i32, i32)) {
        for (pressed, released) in [(true, false), (false, true)] {
            if windows.popup_input(career, at, pressed, released) {
                continue;
            }
            for (window, record, _) in windows.forms.input(at, pressed, released) {
                windows.answer(career, window, record, true);
            }
        }
    }

    fn type_text(windows: &mut HubWindows, career: &mut ArenaCareer, text: &str) {
        for ch in text.bytes() {
            windows.field_key(
                career,
                FieldKey {
                    byte: ch,
                    typed: true,
                    ..FieldKey::default()
                },
                None,
            );
        }
    }

    fn key_down(windows: &mut HubWindows, career: &mut ArenaCareer, byte: u8) {
        windows.field_key(
            career,
            FieldKey {
                byte,
                ..FieldKey::default()
            },
            None,
        );
    }

    fn hover(windows: &mut HubWindows, at: (i32, i32)) {
        windows.forms.input(at, false, false);
    }

    /// Tab moves the emblem form's focus among its tab stops (the name and
    /// the two picture numbers) through the base form's key slot, wrapping;
    /// Shift with Tab sends the word 0x20 and steps forward too (only the
    /// word 0x10 steps back); the focus it sets stays until the pointer
    /// moves. Driven through the live key reader and typing system with key
    /// messages.
    #[test]
    fn tab_moves_the_emblem_forms_focus_among_its_tab_stops() {
        use bevy::input::ButtonState;
        use bevy::input::keyboard::Key;
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ArenaCareer>()
            .add_plugins(crate::gui_keys::GuiKeysPlugin)
            .add_systems(Update, hub_form_typing);
        let mut hub = windows();
        hub.open_company();
        let name = centre(
            hub.forms.get(FormWindow::Emblem).unwrap(),
            emblem_records::NAME,
        );
        hover(&mut hub, name);
        app.insert_resource(hub);
        let window = app.world_mut().spawn_empty().id();
        let send = |app: &mut App, key_code: KeyCode, logical_key: Key, pressed: bool| {
            app.world_mut().write_message(KeyboardInput {
                key_code,
                logical_key,
                state: if pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                },
                text: None,
                repeat: false,
                window,
            });
        };
        let tab = |app: &mut App, shift: bool| {
            if shift {
                send(app, KeyCode::ShiftLeft, Key::Shift, true);
            }
            send(app, KeyCode::Tab, Key::Tab, true);
            app.update();
            send(app, KeyCode::Tab, Key::Tab, false);
            if shift {
                send(app, KeyCode::ShiftLeft, Key::Shift, false);
            }
            app.update();
            app.world().resource::<HubWindows>().forms.options.key_focus
        };
        let emblem = |record| Some((FormWindow::Emblem, record));
        assert_eq!(
            tab(&mut app, false),
            emblem(emblem_records::BACKGROUND_NUMBER)
        );
        assert_eq!(tab(&mut app, false), emblem(emblem_records::DESIGN_NUMBER));
        assert_eq!(tab(&mut app, false), emblem(emblem_records::NAME));
        assert_eq!(
            tab(&mut app, true),
            emblem(emblem_records::BACKGROUND_NUMBER)
        );
        let mut hub = app.world_mut().resource_mut::<HubWindows>();
        // The word 0x10 steps back, wrapping to the last stop.
        hub.form_tab(0x10);
        assert_eq!(hub.forms.options.key_focus, emblem(emblem_records::NAME));
        hub.form_tab(0x10);
        assert_eq!(
            hub.forms.options.key_focus,
            emblem(emblem_records::DESIGN_NUMBER)
        );
        // The pointer still on the name does not take the focus back; a
        // move does.
        hover(&mut hub, name);
        assert_eq!(
            hub.forms.options.key_focus,
            emblem(emblem_records::DESIGN_NUMBER)
        );
        hover(&mut hub, (name.0 + 1, name.1));
        assert_eq!(hub.forms.options.key_focus, emblem(emblem_records::NAME));
        // The focus Tab gave selects the field's text: a typed byte
        // replaces the picture number.
        let mut career = ArenaCareer::default();
        hub.form_tab(0);
        type_text(&mut hub, &mut career, "7");
        let form = hub.forms.get(FormWindow::Emblem).unwrap();
        assert_eq!(
            form.controls()[emblem_records::BACKGROUND_NUMBER].text,
            b"7"
        );
    }

    /// The live typing hears the window procedure's keys: each key-down
    /// (its edit key) and the character it typed reach the focused name
    /// field's key message in order, and Shift with a letter types the
    /// capital the character carries.
    #[test]
    fn the_name_field_types_through_the_frames_keys() {
        use bevy::input::ButtonState;
        use bevy::input::keyboard::Key;
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ArenaCareer>()
            .add_plugins(crate::gui_keys::GuiKeysPlugin)
            .add_systems(Update, hub_form_typing);
        let mut hub = windows();
        hub.open_company();
        let name = centre(
            hub.forms.get(FormWindow::Emblem).unwrap(),
            emblem_records::NAME,
        );
        hover(&mut hub, name);
        app.insert_resource(hub);
        let window = app.world_mut().spawn_empty().id();
        let send = |app: &mut App, key_code: KeyCode, logical_key: Key, pressed: bool| {
            app.world_mut().write_message(KeyboardInput {
                key_code,
                logical_key,
                state: if pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                },
                text: None,
                repeat: false,
                window,
            });
        };
        let text = |app: &App| {
            app.world()
                .resource::<HubWindows>()
                .forms
                .get(FormWindow::Emblem)
                .unwrap()
                .controls()[emblem_records::NAME]
                .text
                .clone()
        };
        let before = text(&app);
        send(&mut app, KeyCode::KeyA, Key::Character("a".into()), true);
        send(&mut app, KeyCode::KeyA, Key::Character("a".into()), false);
        send(&mut app, KeyCode::ShiftLeft, Key::Shift, true);
        send(&mut app, KeyCode::KeyB, Key::Character("B".into()), true);
        send(&mut app, KeyCode::KeyB, Key::Character("B".into()), false);
        send(&mut app, KeyCode::ShiftLeft, Key::Shift, false);
        send(&mut app, KeyCode::ArrowLeft, Key::ArrowLeft, true);
        send(&mut app, KeyCode::ArrowLeft, Key::ArrowLeft, false);
        send(&mut app, KeyCode::KeyC, Key::Character("c".into()), true);
        app.update();
        let mut want = before.clone();
        want.extend_from_slice(b"acB");
        assert_eq!(text(&app), want);
    }

    #[test]
    fn the_emblem_name_edits_at_the_caret_and_klean_waits_for_enter() {
        let mut windows = windows();
        let mut career = ArenaCareer::default();
        windows.open_company();
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        let name = centre(form, emblem_records::NAME);
        // The focus follows the pointer: no click is needed.
        hover(&mut windows, name);
        assert_eq!(
            windows.forms.options.key_focus,
            Some((FormWindow::Emblem, emblem_records::NAME))
        );
        type_text(&mut windows, &mut career, "kean");
        for _ in 0..3 {
            key_down(&mut windows, &mut career, exa_ui::text_edit::key::LEFT);
        }
        type_text(&mut windows, &mut career, "l");
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        assert_eq!(form.controls()[emblem_records::NAME].text, b"klean");
        // Every key told the owner, but the preset waits for Enter's
        // submitted bit.
        let state = windows.forms.options.emblem.as_ref().unwrap();
        assert_eq!(state.editor.record.background, 1);
        key_down(&mut windows, &mut career, field_edit::ENTER);
        let state = windows.forms.options.emblem.as_ref().unwrap();
        assert_eq!(
            (state.editor.record.background, state.editor.record.design),
            (52, 81)
        );
        // The name's release style gave the focus up until the pointer moves.
        assert_eq!(windows.forms.options.key_focus, None);
        hover(&mut windows, name);
        assert_eq!(windows.forms.options.key_focus, None);
        hover(&mut windows, (name.0 + 1, name.1));
        assert!(windows.forms.options.key_focus.is_some());
        // The focus selects the whole name: a typed byte replaces it, and a
        // twenty-first byte is refused.
        type_text(&mut windows, &mut career, "abcdefghijklmnopqrstuvw");
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        assert_eq!(
            form.controls()[emblem_records::NAME].text,
            b"abcdefghijklmnopqrst"
        );
    }

    #[test]
    fn a_picture_number_takes_digits_and_selects_on_enter() {
        let mut windows = windows();
        let mut career = ArenaCareer::default();
        windows.open_company();
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        let number = centre(form, emblem_records::DESIGN_NUMBER);
        hover(&mut windows, number);
        type_text(&mut windows, &mut career, "1x2");
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        assert_eq!(form.controls()[emblem_records::DESIGN_NUMBER].text, b"12");
        let state = windows.forms.options.emblem.as_ref().unwrap();
        assert_eq!(state.editor.record.design, 1);
        key_down(&mut windows, &mut career, field_edit::ENTER);
        let state = windows.forms.options.emblem.as_ref().unwrap();
        assert_eq!(state.editor.record.design, 12);
        // The number field keeps the focus (no release style).
        assert!(windows.forms.options.key_focus.is_some());
    }

    #[test]
    fn a_merchant_combo_edits_its_text_and_enter_restores_its_choice() {
        let mut windows = windows();
        let mut career = ArenaCareer::default();
        windows.open_merchant_options(&career);
        let form = windows.forms.get(FormWindow::Merchant).unwrap();
        let look = centre(form, merchant::LOOK_FOR);
        hover(&mut windows, look);
        // The focus selected the whole text; End drops the selection and
        // Backspace takes the last byte.
        key_down(&mut windows, &mut career, exa_ui::text_edit::key::END);
        key_down(&mut windows, &mut career, exa_ui::text_edit::key::BACKSPACE);
        let form = windows.forms.get(FormWindow::Merchant).unwrap();
        assert_eq!(form.controls()[merchant::LOOK_FOR].text, b"Anythin");
        career.merchant_kinds = 0x1000;
        key_down(&mut windows, &mut career, field_edit::ENTER);
        // The callback stores the stock type its selection word names.
        assert_eq!(career.merchant_kinds, 0);
    }

    #[test]
    fn a_swatch_drag_drops_its_colour_on_another_swatch() {
        let mut windows = windows();
        let mut career = ArenaCareer::default();
        windows.open_company();
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        let from = centre(form, emblem_records::SWATCHES[0]);
        let to = centre(form, emblem_records::SWATCHES[3]);
        let word = u32::from_le_bytes(form.controls()[emblem_records::SWATCHES[0]].colour.unwrap());
        windows.forms.options.clock = 1000;
        windows.forms.input(from, true, false);
        // Inside the threshold nothing drags yet.
        windows.forms.input((from.0 + 1, from.1), false, false);
        assert!(windows.forms.options.dragging.is_none());
        windows.forms.options.clock = 1500;
        windows.forms.input((to.0, to.1), false, false);
        assert!(windows.forms.options.dragging.is_some());
        let answered = windows.forms.input(to, false, true);
        // The drop took the release: no swatch callback (no palette popup).
        assert!(answered.is_empty());
        for (window, record, _) in answered {
            windows.answer(&mut career, window, record, true);
        }
        assert!(windows.forms.options.popup.is_none());
        let state = windows.forms.options.emblem.as_ref().unwrap();
        assert_eq!(state.editor.record.colours[3], word);
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        assert_eq!(
            form.controls()[emblem_records::SWATCHES[3]].colour,
            Some(word.to_le_bytes())
        );
    }

    /// The combo box's list is placed by the GUI's child placement: a
    /// pixel over the owner's bottom edge, or with its bottom a pixel under
    /// the owner's top when the screen has no room below; flush with the
    /// right edge less two when it would leave the screen; never cut.
    #[test]
    fn a_combo_list_opens_over_the_owners_edge_or_above_it() {
        let rows = |n: usize| (0..n).map(|i| format!("row {i}").into_bytes()).collect();
        let mark = exa_ui::skin::open_skin(4, 0, true).map_or(0, |skin| skin.mark);
        let lift = if mark & 1 != 0 { 4 - 1 } else { 0 };
        let (list, rect) = open_drop_list(rows(3), (100, 200, 160, 24), 4, (1280, 720), 1);
        let h = 3 * 4 * exa_ui::drop_down::ROW_UNITS + 2 * list.border;
        assert_eq!(
            (rect.x, rect.y, rect.w, rect.h),
            (100, 200 - lift + 24 - 1, 160, h)
        );
        // No room below: above the owner, its bottom at the owner's top + 1.
        let (_, rect) = open_drop_list(rows(3), (100, 700, 160, 24), 4, (1280, 720), 1);
        assert_eq!(rect.y + rect.h, 700 - lift + 1);
        // Twenty rows are sixteen tall and are not cut to the room left.
        let (list, rect) = open_drop_list(rows(20), (100, 400, 160, 24), 4, (1280, 720), 1);
        assert_eq!(
            rect.h,
            16 * 4 * exa_ui::drop_down::ROW_UNITS + 2 * list.border
        );
        assert_eq!(rect.y + rect.h, 400 - lift + 1);
        // Past the right edge: flush with it less two.
        let (_, rect) = open_drop_list(rows(3), (1200, 200, 160, 24), 4, (1280, 720), 1);
        assert_eq!(rect.x, 1280 - 160 - 2);
    }

    #[test]
    fn the_merchant_lists_set_the_stock_type_and_rank() {
        let mut windows = windows();
        let mut career = ArenaCareer::default();
        windows.open_merchant_options(&career);
        let form = windows.forms.get(FormWindow::Merchant).unwrap();
        assert_eq!(form.controls()[merchant::LOOK_FOR].text, b"Anything");
        assert_eq!(form.controls()[merchant::RANK].text, b"Any");
        let at = centre(form, merchant::LOOK_FOR);
        click(&mut windows, &mut career, at);
        let Some(Popup::List { rect, list, .. }) = windows.forms.options.popup.clone() else {
            panic!("no list");
        };
        assert_eq!(list.count(), 3);
        // The third row: Weapons.
        let row = list.unit * exa_ui::drop_down::ROW_UNITS;
        let at = (rect.x + 10, rect.y + list.border + row * 2 + row / 2);
        click(&mut windows, &mut career, at);
        assert!(windows.forms.options.popup.is_none());
        assert_eq!(career.merchant_kinds, 0xc000);
        let form = windows.forms.get(FormWindow::Merchant).unwrap();
        assert_eq!(form.controls()[merchant::LOOK_FOR].text, b"Weapons");
        let at = centre(form, merchant::RANK);
        click(&mut windows, &mut career, at);
        let Some(Popup::List { rect, .. }) = windows.forms.options.popup.clone() else {
            panic!("no list");
        };
        // Novice is the fourth row: rank 2.
        let at = (rect.x + 10, rect.y + 3 + row * 3 + row / 2);
        click(&mut windows, &mut career, at);
        assert_eq!(career.merchant_rank, 2);
    }

    #[test]
    fn the_emblem_steps_mirrors_and_takes_a_palette_colour() {
        let mut windows = windows();
        let mut career = ArenaCareer::default();
        windows.open_company();
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        let up = centre(form, emblem_records::BACKGROUND_UP);
        click(&mut windows, &mut career, up);
        let state = windows.forms.options.emblem.as_ref().unwrap();
        assert_eq!(state.editor.record.background, 2);
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        assert_eq!(
            form.controls()[emblem_records::BACKGROUND_NUMBER].text,
            b"2"
        );
        // The mirror box sets its layer bit.
        let mirror = centre(form, emblem_records::BACKGROUND_MIRROR);
        click(&mut windows, &mut career, mirror);
        let state = windows.forms.options.emblem.as_ref().unwrap();
        assert_eq!(state.editor.record.layers & emblem_form::MIRROR_X, 0x10);
        // The first swatch opens the palette with its index under the
        // pointer; a press and release on another cell takes it.
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        let swatch = centre(form, emblem_records::SWATCHES[0]);
        click(&mut windows, &mut career, swatch);
        let Some(Popup::Palette { rect, .. }) = windows.forms.options.popup.clone() else {
            panic!("no palette");
        };
        let cell = 3 * 4;
        let at = (
            rect.x + col_swatch::BORDER + 1 + 2 * cell + 2,
            rect.y + col_swatch::BORDER + 1 + 3 * cell + 2,
        );
        click(&mut windows, &mut career, at);
        let state = windows.forms.options.emblem.as_ref().unwrap();
        assert_eq!(state.editor.record.palette(0), 2 * 8 + 3);
        assert!(windows.forms.options.popup.is_none());
        // NEXT with no name refuses and keeps the dialog.
        let dialog = windows.forms.get(FormWindow::Company).unwrap();
        let next = centre(dialog, company::NEXT);
        click(&mut windows, &mut career, next);
        assert!(windows.forms.get(FormWindow::Emblem).is_some());
        // ... and shows the "Company Name" box over it, which takes the
        // pointer until Enter closes it.
        assert!(windows.forms.message_on_top());
        let open = windows.forms.message.as_ref().unwrap();
        assert_eq!(
            (open.title, open.text, open.flags),
            (
                crate::emblem::COMPANY_NAME_TITLE,
                crate::emblem::COMPANY_NAME_PROMPT,
                0xc1
            )
        );
        assert!(windows.forms.message_key(exa_ui::modal_screens::KEY_ENTER));
        assert!(!windows.forms.message_on_top());
        // Typing a name and NEXT store the company.
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        let name = centre(form, emblem_records::NAME);
        click(&mut windows, &mut career, name);
        type_text(&mut windows, &mut career, "Ravens");
        let dialog = windows.forms.get(FormWindow::Company).unwrap();
        let next = centre(dialog, company::NEXT);
        windows.forms.options.focus = None;
        click(&mut windows, &mut career, next);
        assert_eq!(career.company.name, "Ravens");
        assert_eq!(career.company.emblem.palette(0), 19);
    }

    #[test]
    fn the_picker_steps_a_spinner_and_finishes() {
        let mut windows = windows();
        let mut career = ArenaCareer::default();
        windows.open_picker([255, 0, 0, 255], "Global Tint");
        let state = windows.forms.options.picker.as_ref().unwrap();
        assert_eq!(state.picker.rgb_fields[0], 255.0);
        let form = windows.forms.get(FormWindow::Picker).unwrap();
        assert_eq!(form.title, "Global Tint");
        // The green spinner's up button.
        let c = form.controls()[picker_records::RGB_SPINS[1]].clone();
        let (ox, oy) = form.origin();
        let button = exa_ui::controls::spinner_button_width(4);
        let at = (ox + c.x + c.w - button / 2, oy + c.y + 2);
        click(&mut windows, &mut career, at);
        let state = windows.forms.options.picker.as_ref().unwrap();
        assert_eq!(state.picker.preview, [255, 1, 0, 255]);
        assert_eq!(state.swatch.colour, [255, 1, 0, 255]);
        let form = windows.forms.get(FormWindow::Picker).unwrap();
        let close = centre(form, picker_records::CLOSE);
        click(&mut windows, &mut career, close);
        assert!(windows.forms.get(FormWindow::Picker).is_none());
    }

    /// The hub draw's layers after a few updates of a headless App with
    /// the real game-screen and hub draw systems over `windows`, with the
    /// interface level's glyph atlas at unit 4.
    fn drawn_layers(windows: HubWindows) -> Vec<exa_ui::gui_draw::GuiLayer> {
        use exa_ui::gui_draw::GuiFrames;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GuiFrames>()
            .init_resource::<crate::gui_screens::GameScreens>()
            .init_resource::<crate::item_panel_draw::PanelIcons>()
            .init_resource::<crate::gui_screens::DebugGuiForm>()
            .insert_resource(exa_ui::GuiLevelState::at(1))
            .insert_resource(windows);
        app.add_systems(Update, crate::gui_screens::draw_screens);
        crate::hub_windows::add_hub_draw(&mut app);
        for _ in 0..3 {
            app.update();
        }
        app.world()
            .resource::<GuiFrames>()
            .frames
            .iter()
            .flat_map(|f| f.layers.iter().cloned())
            .collect()
    }

    /// The picker's close mark: its frame code under a point of its title
    /// band's right end.
    fn picker_close_mark(windows: &HubWindows) -> (i32, i32) {
        let form = windows.forms.get(FormWindow::Picker).unwrap();
        let r = form.rect();
        (r.x + r.w / 2..r.x + r.w)
            .flat_map(|x| (r.y..r.y + 26).map(move |y| (x, y)))
            .find(|&at| crate::base_form::frame_code(form, at) == -4)
            .expect("the close mark")
    }

    #[test]
    fn the_pickers_close_mark_lights_draws_its_icon_and_closes_the_picker() {
        let picker = |hovered: bool| {
            let mut windows = windows();
            windows.open_picker([255, 0, 0, 255], "Global Tint");
            let mark = picker_close_mark(&windows);
            if hovered {
                hover(&mut windows, mark);
            }
            (windows, mark)
        };
        let palette = exa_ui::skin::shown_base_palette(1.0);
        let icon_ink = |layers: &[exa_ui::gui_draw::GuiLayer]| {
            layers
                .iter()
                .filter(|l| matches!(l.texture, exa_ui::gui_draw::LayerTexture::Owned(_)))
                .filter_map(|l| l.batch.vertices.first().map(|v| v.ink))
                .collect::<Vec<_>>()
        };
        let bevels = |layers: &[exa_ui::gui_draw::GuiLayer]| {
            layers
                .iter()
                .flat_map(|l| l.batch.vertices.iter())
                .filter(|v| v.ink == palette.form_hot[6])
                .count()
        };
        // Away from the mark its icon draws plain in the label ink.
        let idle = drawn_layers(picker(false).0);
        assert!(icon_ink(&idle).contains(&palette.text[6]));
        let (windows, _) = picker(true);
        let form = windows.forms.get(FormWindow::Picker).unwrap();
        assert_eq!(form.frame.hover, -4);
        // Under the pointer: the hot box (its outer bevel in the hot form
        // group's seventh shade) and the outlined icon in the skin's face
        // ink.
        let hot = drawn_layers(windows);
        assert!(icon_ink(&hot).contains(&exa_ui::hud_status::status_inks(&palette).face));
        assert!(bevels(&hot) > bevels(&idle));
        // Held down: the pressed box and its faded face, from the form
        // group's fourth shade up to its third, a pixel in.
        let (mut windows, mark) = picker(true);
        windows.forms.input(mark, true, false);
        let form = windows.forms.get(FormWindow::Picker).unwrap();
        assert_eq!((form.frame.held, form.frame.down), (-4, true));
        let faces = |layers: &[exa_ui::gui_draw::GuiLayer]| {
            layers
                .iter()
                .flat_map(|l| l.batch.vertices.iter())
                .filter(|v| v.ink == palette.form[2])
                .count()
        };
        let pressed = drawn_layers(windows);
        assert!(faces(&pressed) > faces(&hot));
        // A press and release on the mark closes the picker.
        let (mut windows, mark) = picker(true);
        let mut career = ArenaCareer::default();
        click(&mut windows, &mut career, mark);
        assert!(windows.forms.get(FormWindow::Picker).is_none());
        assert!(windows.forms.options.picker.is_none());
    }

    #[test]
    fn the_emblem_swatches_draw_the_line_shade_and_the_pickers_the_control_fill() {
        let vertices = |layers: &[exa_ui::gui_draw::GuiLayer]| {
            layers
                .iter()
                .flat_map(|l| l.batch.vertices.as_chunks::<4>().0.to_vec())
                .collect::<Vec<_>>()
        };
        let mut windows = windows();
        windows.open_company();
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        let c = form.controls()[emblem_records::SWATCHES[0]].clone();
        let rect = on_screen(form, &c);
        let [r, g, b, _] = c.colour.expect("a swatch colour");
        let quads = vertices(&drawn_layers(windows));
        let unit = 4;
        // The colour a unit in, at the GUI's alpha.
        assert!(quads.iter().any(|q| q[1].ink == [r, g, b, 0xff]
            && (q[1].x, q[1].y) == ((rect.left + unit) as f32, (rect.top + unit) as f32)
            && q[2].y == (rect.bottom - unit) as f32));
        // The shade's top-left cap: the cap record from two units in, a
        // unit either side of half a unit above the top, in the on ink.
        let top = rect.top - unit / 2;
        assert!(quads.iter().any(|q| (q[0].u, q[0].v) == (0.7, 0.1)
            && (q[0].x, q[0].y) == ((rect.left + 2 * unit) as f32, (top + unit) as f32)
            && q[2].x == rect.left as f32
            && q[0].ink == exa_ui::line_skin::line_control_inks(1.0)[2]));

        let mut windows = super::tests::windows();
        windows.open_picker([10, 200, 30, 255], "Tint");
        let form = windows.forms.get(FormWindow::Picker).unwrap();
        let (i, c) = form
            .controls()
            .iter()
            .enumerate()
            .find(|(_, c)| c.class == COLOUR_SWATCH && c.colour.is_some())
            .map(|(i, c)| (i, c.clone()))
            .expect("a picker swatch");
        let rect = on_screen(form, &c);
        let [r, g, b, _] = form.controls()[i].colour.unwrap();
        let quads = vertices(&drawn_layers(windows));
        let m = c.margin;
        // The draw skin's fill: the colour inset by the margin, the solid
        // rectangle's top-right first.
        assert!(quads.iter().any(|q| q[0].ink == [r, g, b, 0xff]
            && (q[0].x, q[0].y) == ((rect.right - m) as f32, (rect.top + m) as f32)
            && (q[2].x, q[2].y) == ((rect.left + m) as f32, (rect.bottom - m) as f32)));
    }

    /// The hub draw's frames after a few updates of a headless App with
    /// the real game-screen and hub draw systems over `windows`.
    fn drawn_quads(windows: HubWindows) -> Vec<[exa_ui::skin::GuiVertex; 4]> {
        use exa_ui::gui_draw::GuiFrames;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GuiFrames>()
            .init_resource::<crate::gui_screens::GameScreens>()
            .init_resource::<crate::item_panel_draw::PanelIcons>()
            .init_resource::<crate::gui_screens::DebugGuiForm>()
            .insert_resource(windows);
        app.add_systems(Update, crate::gui_screens::draw_screens);
        crate::hub_windows::add_hub_draw(&mut app);
        for _ in 0..3 {
            app.update();
        }
        app.world()
            .resource::<GuiFrames>()
            .frames
            .iter()
            .flat_map(|f| f.layers.iter())
            .flat_map(|l| l.batch.vertices.as_chunks::<4>().0.to_vec())
            .collect()
    }

    /// Whether a quad of the line skin's wide window frame corner record
    /// (0.15, 0.2 first, 0.35, 0.5 third) starts at `at`.
    fn window_corner_at(quads: &[[exa_ui::skin::GuiVertex; 4]], at: (i32, i32)) -> bool {
        quads.iter().any(|q| {
            (q[0].u, q[0].v, q[2].u, q[2].v) == (0.15, 0.2, 0.35, 0.5)
                && (q[0].x, q[0].y) == (at.0 as f32, at.1 as f32)
        })
    }

    #[test]
    fn the_combo_list_and_the_palette_popup_draw_the_line_skins_wide_frame() {
        let mut windows = windows();
        let mut career = ArenaCareer::default();
        windows.open_merchant_options(&career);
        let form = windows.forms.get(FormWindow::Merchant).unwrap();
        let combo = {
            let c = &form.controls()[merchant::LOOK_FOR];
            on_screen(form, c)
        };
        let at = centre(form, merchant::LOOK_FOR);
        click(&mut windows, &mut career, at);
        let Some(Popup::List { rect, list, .. }) = windows.forms.options.popup.clone() else {
            panic!("no list");
        };
        // The line skin's opener cleared the panel style and lifted the
        // list by a unit less one; the list overlaps the owner's bottom
        // edge by a pixel.
        assert_eq!(list.style, 0x18);
        assert_eq!(rect.y, combo.top - 3 + combo.height() - 1);
        let quads = drawn_quads(windows);
        // The window frame's top-left corner arm reaches two units out.
        assert!(window_corner_at(&quads, (rect.x - 8, rect.y - 8)));
        assert!(window_corner_at(&quads, (rect.x + rect.w + 8, rect.y - 8)));
        // No wide panel arm: no bar straddles the top edge a unit up.
        assert!(!quads.iter().any(|q| q[0].y == (rect.y + 4) as f32
            && q[1].y == (rect.y - 4) as f32
            && q[0].x == (rect.x + 8) as f32));

        let mut windows = super::tests::windows();
        windows.open_company();
        let form = windows.forms.get(FormWindow::Emblem).unwrap();
        let swatch = centre(form, emblem_records::SWATCHES[0]);
        click(&mut windows, &mut career, swatch);
        let Some(Popup::Palette { rect, .. }) = windows.forms.options.popup.clone() else {
            panic!("no palette");
        };
        let quads = drawn_quads(windows);
        assert!(window_corner_at(&quads, (rect.x - 8, rect.y - 8)));
        assert!(window_corner_at(
            &quads,
            (rect.x + rect.w + 8, rect.y + rect.h + 8)
        ));
    }
}
