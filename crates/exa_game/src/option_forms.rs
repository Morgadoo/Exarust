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
//!   on it. The name field types the company's name (and `klean` selects
//!   the preset); the step buttons and number fields select the background
//!   and design through the selector rule
//!   ([`exa_ui::emblem_form::step_states`]); the mirror boxes set the layer
//!   bits; each colour swatch opens the palette swatch popup
//!   ([`exa_ui::col_swatch`]) under the pointer, whose hovered or pressed
//!   cell is the colour slot's palette index; Randomise rolls and Previous
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
//! shade), a spinner's typed value and drag, and the combo boxes' keys.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use exa_ui::arena_forms::{self, MERCHANT_FORM, MERCHANT_PLACE, merchant};
use exa_ui::col_swatch::{self, ColourSwatch};
use exa_ui::colour_picker::{
    self, ColourPicker, PICKER_FORM, PickerEvent, SwatchControl, picker_records,
};
use exa_ui::drop_down::{DropDownList, DropDownOwner};
use exa_ui::emblem_form::{
    self, COMPANY_RECORDS, EMBLEM_FORM, EmblemAction, EmblemEditor, EmblemRandom, company,
    company_form, emblem_action, records as emblem_records,
};
use exa_ui::gui_controls::{Control, state};
use exa_ui::gui_core::GuiRect;
use exa_ui::record_form::RecordForm;
use exa_ui::settings::Rect;

use crate::gui_paint::Painted;
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
    /// The name field had an edit since the form opened (the field's
    /// changed bit, which the name callback tests before the preset).
    pub name_changed: bool,
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
}

impl OptionForms {
    /// Forgets the open forms' state (the kept `Look for:` word stays).
    pub fn clear(&mut self) {
        self.popup = None;
        self.focus = None;
        self.emblem = None;
        self.picker = None;
    }
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
        let unit = self.unit_or_4();
        let screen = self.task.screen;
        let dialog = RecordForm::open(&company_form(screen, unit), screen, unit);
        log_records("company dialog", &dialog);
        self.forms.push(FormWindow::Company, dialog);
        let mut form = RecordForm::open(&EMBLEM_FORM, screen, unit);
        let mut state = EmblemState {
            editor: EmblemEditor::new(),
            name_changed: false,
        };
        apply_emblem(&mut form, &state.editor);
        state.editor.refresh_colours();
        // The constructor's first selections only store the numbers; the
        // fields read "1" from their records.
        self.forms.options.emblem = Some(state);
        info!("company dialog: the emblem form opens at {:?}", form.rect());
        log_records("emblem form", &form);
        self.forms.push(FormWindow::Emblem, form);
        debug_assert_eq!(COMPANY_RECORDS.len(), 2);
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
                career.company.name = name;
                career.company.emblem = emblem;
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
        let screen_h = self.task.screen.1;
        let Some(form) = self.forms.get(window) else {
            return;
        };
        let items = form.spec.records[record].items;
        if items.is_empty() {
            return;
        }
        let c = &form.controls()[record];
        let rect = on_screen(form, c);
        let mut list = DropDownList::new(
            items.iter().map(|item| item.as_bytes().to_vec()).collect(),
            c.w,
            unit,
        );
        let layout = list.lay_out();
        let place = exa_ui::controls::drop_down_place(
            (rect.left, rect.top, c.w, c.h),
            layout.height,
            list.border,
            layout.row,
            screen_h,
        );
        let selected = form.amounts[record] as i32;
        list.select(selected, 0);
        info!(
            "combo list opens at ({}, {}) {}x{}, rows {} px from y {}",
            place.x,
            place.y,
            place.w,
            place.h,
            layout.row,
            place.y + list.border
        );
        self.forms.options.popup = Some(Popup::List {
            window,
            record,
            list,
            rect: Rect {
                x: place.x,
                y: place.y,
                w: place.w,
                h: place.h,
            },
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
                let inside = contains(*rect, at);
                if inside {
                    let row = list.row_at(at.1 - rect.y);
                    list.select(row, 0);
                }
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
                    if let Some(row) = owner.chosen {
                        chosen = Some((*window, *record, row as u32));
                    }
                    list.close(&mut owner);
                    keep = false;
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
            EmblemAction::Name => {
                self.forms.options.focus = Some((FormWindow::Emblem, record));
            }
            EmblemAction::Number { .. } => {
                self.forms.options.focus = Some((FormWindow::Emblem, record));
            }
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

    /// A typed key for the focused text field: a printable byte up to the
    /// field's limit (a digits field drops anything else), Backspace takes
    /// the last byte off, Enter submits. The name field calls its owner on
    /// every edit; the number fields on Enter.
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
            self.emblem_text(record, edited);
        }
    }

    fn emblem_text(&mut self, record: usize, edited: bool) {
        let Some(i) = self.forms.find(FormWindow::Emblem) else {
            return;
        };
        let Some(state) = self.forms.options.emblem.as_mut() else {
            return;
        };
        let form = &mut self.forms.forms[i].form;
        match emblem_action(record) {
            Some(EmblemAction::Name) => {
                state.name_changed |= edited;
                let text = String::from_utf8_lossy(&form.controls()[record].text).into_owned();
                if state.name_changed && crate::emblem::apply_klean(&text, &mut state.editor) {
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

/// A horizontal gradient over a rectangle through `stops`.
pub(crate) fn gradient_layer(rect: GuiRect, stops: &[[u8; 4]]) -> exa_ui::gui_draw::GuiLayer {
    let mut batch = exa_ui::skin::SkinBatch::new();
    batch.select(1, 0, false);
    batch.begin();
    let n = stops.len().max(2) - 1;
    let (top, bottom) = (rect.top as f32, (rect.top + rect.height()) as f32);
    for i in 0..n {
        let x0 = rect.left as f32 + rect.width() as f32 * i as f32 / n as f32;
        let x1 = rect.left as f32 + rect.width() as f32 * (i + 1) as f32 / n as f32;
        let (a, b) = (stops[i], stops[(i + 1).min(stops.len() - 1)]);
        batch.set_ink(a);
        batch.vertex(x0, top);
        batch.vertex(x0, bottom);
        batch.set_ink(b);
        batch.vertex(x1, bottom);
        batch.vertex(x1, top);
    }
    batch.end_quads();
    exa_ui::gui_draw::GuiLayer::new(batch, exa_ui::gui_draw::LayerTexture::None)
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

/// Paints an extended-class record the table-built forms carry beside the
/// basic classes: a text field's frame and text (its caret while it has
/// the focus), a colour swatch's colour under its frame, a spinner's field,
/// value and two buttons, and an image's shade. Answers whether it painted.
pub(crate) fn paint_extended(
    painted: &mut Painted,
    c: &Control,
    rect: GuiRect,
    look: &Look,
    focused: bool,
) -> bool {
    use exa_ui::controls::{COLOUR_SWATCH, IMAGE, SPINNER, TEXT_FIELD};
    match c.class {
        TEXT_FIELD => {
            // The field's inner frame (the label skin's style 0xf0 frames)
            // and its run, aligned by the field's own alignment bits; a
            // caret mark follows the run while the field has the focus.
            let mut field = c.clone();
            field.class = 5;
            field.style = 0xf0 | (c.style & 0x7700);
            if focused {
                field.text.push(b'_');
            }
            let (font, size) = crate::hub_windows::screen_font(look.textures, look.unit);
            let text = crate::settings_form::Text { font: &font, size };
            crate::settings_form::paint_control(
                &field,
                (rect.left - c.x, rect.top - c.y),
                look.unit,
                &look.palette,
                &text,
                None,
                painted,
            );
            true
        }
        COLOUR_SWATCH => {
            crate::hub_windows::paint_panel(painted, rect, look);
            if let Some([r, g, b, _]) = c.colour {
                let inner = [
                    rect.left + 2,
                    rect.top + 2,
                    (rect.width() - 4).max(0),
                    (rect.height() - 4).max(0),
                ];
                painted.push(exa_ui::loadout::solid_layer(inner, [r, g, b, 0xff]));
            }
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
            // The field to the button column, framed, with the value in the
            // spinner's own alignment.
            let mut value = c.clone();
            value.class = 5;
            value.w = field.width();
            value.style = 0xf0 | (c.style & 0x7700);
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

/// Paints the open popup over the forms: a drop-down list's panel, its
/// selection band and rows, or the palette swatch popup's cells and its
/// selection's frame.
pub(crate) fn paint_popup(painted: &mut Painted, popup: &Popup, look: &Look) {
    match popup {
        Popup::List { list, rect, .. } => {
            let r = GuiRect::new(rect.x, rect.y, rect.w, rect.h);
            crate::hub_windows::paint_panel(painted, r, look);
            let mut batch = exa_ui::skin::SkinBatch::new();
            list.paint(&mut batch, &look.palette, 0);
            crate::gui_paint::offset_batch(&mut batch, rect.x, rect.y);
            painted.push(exa_ui::gui_draw::GuiLayer::new(
                batch,
                exa_ui::gui_draw::LayerTexture::Named(exa_ui::gui_draw::SKIN_TEXTURE),
            ));
            let (font, size) = crate::hub_windows::screen_font(look.textures, look.unit);
            if let Some(texture) = size {
                for row in list.rows(&font) {
                    let Some(item) = list.items.get((row.row - 1) as usize) else {
                        continue;
                    };
                    let ink = if row.selected {
                        look.palette.text[7]
                    } else {
                        look.palette.text[6]
                    };
                    painted.push(crate::gui_paint::caption_layer(
                        &font,
                        texture,
                        item,
                        (rect.x + row.pen.0, rect.y + row.pen.1),
                        ink,
                    ));
                }
            }
        }
        Popup::Palette { swatch, rect, .. } => {
            let r = GuiRect::new(rect.x, rect.y, rect.w, rect.h);
            crate::hub_windows::paint_panel(painted, r, look);
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
/// design and the two shared layers, loaded once each.
pub fn load_emblem_textures(
    windows: Option<Res<HubWindows>>,
    textures: Option<ResMut<exa_ui::gui_draw::GuiTextures>>,
    game: Option<Res<exa_assets::Game>>,
) {
    let (Some(windows), Some(mut textures)) = (windows, textures) else {
        return;
    };
    let Some(state) = windows.forms.options.emblem.as_ref() else {
        return;
    };
    for name in exa_ui::emblem_form::preview_textures(&state.editor.record) {
        crate::creator_screen::ensure_texture(&mut textures, game.as_deref(), &name);
    }
}

/// Typed keys reach the focused text field of the hub's forms.
pub fn hub_form_typing(
    mut typed: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut windows: ResMut<HubWindows>,
) {
    let events: Vec<KeyboardInput> = typed.read().cloned().collect();
    if windows.forms.options.focus.is_none() {
        return;
    }
    for event in &events {
        if !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Character(text) => {
                for ch in text.chars() {
                    windows.type_key(TypedKey::Char(ch));
                }
            }
            Key::Space => windows.type_key(TypedKey::Char(' ')),
            _ => {}
        }
    }
    if keys.just_pressed(KeyCode::Backspace) {
        windows.type_key(TypedKey::Backspace);
    }
    if keys.just_pressed(KeyCode::Enter) {
        windows.type_key(TypedKey::Enter);
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
        for ch in "Ravens".chars() {
            windows.type_key(TypedKey::Char(ch));
        }
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
}
