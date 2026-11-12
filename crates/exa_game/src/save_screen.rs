//! The saves' character screen: the creation form in its continue mode
//! (screen mode 12), opened by the menu's CONTINUE rows, showing one save
//! slot at a time and loading it into the running game.
//!
//! The form ([`SaveScreen`]) fills the window as the creator's does, and
//! builds the continue table ([`exa_ui::creation_forms::continue_records`],
//! eleven records): the `CONTINUE` title, the slot combo box (the slots'
//! player names; its text is the shown character's name), `Previous`,
//! `Next` and `Delete`, the `Inventory` and `Journal` captions, `BACK`, the
//! story's `Continue from checkpoint` toggle and `CONTINUE`. The preview
//! viewport in the middle shows the slot's character
//! ([`crate::creator_view`]); the item container under the inventory
//! caption lists its carried, worn and held items; the journal form under
//! the journal caption has the entry combo box and the entry's text.
//!
//! The callbacks are the original's:
//! * opening scans the manager: no slot falls back (the original opens the
//!   free creator; the menu greys CONTINUE out then, so Exarust opens
//!   nothing); otherwise the first slot is selected with kind 1 and the form
//!   shows index 0 with kind 2 when only the checkpoint is there, else
//!   kind 1 (the kinds are the name builder's: 1 the save `.rsg`, 2 the
//!   checkpoint `.rcp`);
//! * a select ([`SaveScreen::select`]) stores the index, makes a kind of 0
//!   kind 1, asks the manager for the slot
//!   ([`SlotList::character_screen_select`]); a nonzero availability is kept,
//!   the loaded character shown, and the kind moved to the file that is
//!   there; then the form refreshes: the combo's items and text, Previous
//!   disabled at the first slot, Next at the last, the toggle's label,
//!   mirror and enable ([`crate::create::creation_continue_button`]);
//! * Previous and Next step the index with kind 1, the combo's pick selects
//!   its item with kind 1 when it is not the shown one, the toggle swaps the
//!   kind between the save (1) and the checkpoint (2) when that file is
//!   there;
//! * Delete asks `Are you sure you want to delete <name>?` in the `Delete
//!   Character` box; its Delete deletes the slot ([`SlotList::delete_index`]),
//!   keeps the index inside the list and selects it again, or leaves to the
//!   menu when the list is empty;
//! * BACK (and Escape [guess: the creator's cancel key]) leaves to the menu;
//! * CONTINUE loads the current slot: the story's full load
//!   ([`crate::save_live::StorySlotLoad`], the checkpoint first only for
//!   kind 2) or, in the arena, the hub with that slot
//!   ([`crate::save_live::ArenaSlotLoad`]).
//!
//! `--open-saves story|arena` opens the screen once the load ends and
//! `--load-slot N[:save|:checkpoint]` also continues slot N from it
//! ([`ScriptedSaveScreen`]); `--saves-press` presses its buttons in turn
//! (debug).
//!
//! Guesses (labelled where they are made): the delete box's records are laid
//! out in its client at its design size; the drop-down's row height is the
//! combo box's; the item container lists entry names where the original
//! draws icons; the skills panel on the right is not built.
//!
//! The reset-after-the-portal question: after each select that read a
//! slot, and when the screen opens, a story save from before 0.9.5's
//! writers that went past the portal
//! ([`crate::slots::offers_portal_reset`]) raises the `Reset to portal`
//! box ([`exa_ui::dialogs::RESET_RECORDS`]) over the form. Its Yes deletes
//! the file the select read, drops the slot when no checkpoint is left
//! ([`SlotList::reset_to_portal`]) and selects the same index again; its
//! No and the cancel key close it. The original runs the select's refresh
//! and the check behind a fade; here they run at once.
//!
//! Evidence: the creation form's constructor and its continue table, the
//! continue callbacks, the delete box's constructor and table, the
//! character screen's entry and refresh routines (`docs/UI.md`, Character
//! creation; `docs/formats/SAVES.md`).

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_formats::characters::CharacterRecord;
use exa_formats::save_package::JournalRecord;
use exa_ui::creation_forms::{
    self, DELETE_QUESTION_END, DELETE_RECORDS, DELETE_SIZE, DELETE_TITLE, INVENTORY_PLACE,
    JOURNAL_COMPACT_HEIGHT, JOURNAL_HEIGHT, JOURNAL_PLACE, JOURNAL_RECORDS, JOURNAL_TEXT_AT,
    delete_box, saves,
};
use exa_ui::gui_controls::{self, Control, ControlEvent, Message, message};
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{GuiFrames, GuiLayer, GuiTextures, LayerTexture, text_batch};
use exa_ui::gui_font::GuiFont;
use exa_ui::line_skin::shown_palette;
use exa_ui::menu::MenuShow;
use exa_ui::skin::{SkinBatch, SkinPalette};

use crate::create;
use crate::creator_screen::{
    self, COMPACT_BELOW, CreatorScreen, CreatorSystems, ICONS, design, hidden, px, scaled,
    takes_hits,
};
use crate::flow::{Flow, FlowSteps, Phase};
use crate::gui_paint::{self, Frame, Painted};
use crate::menu_pages::{MenuCommand, game_mode};
use crate::menu_screen::MenuChoice;
use crate::slots::{SlotList, offers_portal_reset};
use exa_formats::save_package::SlotKind;
use exa_ui::dialogs::{RESET_RECORDS, RESET_TO_PORTAL, reset_box};

/// The story's game mode word, which shows the checkpoint toggle.
pub const STORY: i32 = 4;
/// The arena's game mode word.
pub const ARENA: i32 = 8;

/// What the pointer is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A record of the continue table.
    Control(usize),
    /// The journal form's entry combo box.
    JournalCombo,
    /// A row of the open drop-down list.
    Row(usize),
    /// A record of the delete box.
    Confirm(usize),
    /// A record of the "Reset to portal" box.
    Reset(usize),
}

/// Which combo box's list is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropList {
    Slots,
    Journal,
}

/// What the screen asks of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveAction {
    None,
    /// Leave to the menu.
    Leave,
    /// Load the slot.
    Continue {
        number: u32,
        prefer_checkpoint: bool,
    },
}

/// The continue form's state.
#[derive(Debug, Clone)]
pub struct SaveScreen {
    /// 4 the story, 8 the arena.
    pub game_mode: i32,
    pub screen: (i32, i32),
    pub unit: i32,
    pub compact: bool,
    /// The continue table in window pixels.
    pub main: Vec<Control>,
    /// The journal form's combo box, relative to the journal form.
    pub journal_combo: Vec<Control>,
    /// The delete box's records, relative to its client, while it is up.
    pub confirm: Option<Vec<Control>>,
    /// The "Reset to portal" box's records, relative to its client, while
    /// it is up ([`SaveScreen::ask_portal_reset`]).
    pub reset: Option<Vec<Control>>,
    /// Which of the slot's files the last select read (the file the reset
    /// deletes).
    pub opened: Option<SlotKind>,
    /// The shown slot's index in the list.
    pub index: i32,
    /// The file the CONTINUE reads first: 1 the save, 2 the checkpoint (the
    /// name builder's kinds, [`SlotKind::code`]).
    pub kind: i32,
    /// The last select's availability (1 save, 2 checkpoint).
    pub availability: u32,
    /// The list's numbers and names, as the combo box lists them.
    pub numbers: Vec<u32>,
    pub names: Vec<Vec<u8>>,
    /// The shown character.
    pub record: CharacterRecord,
    /// The shown save's journal and the entry shown (0-based).
    pub journal: Vec<JournalRecord>,
    pub journal_entry: i32,
    /// Item names by id, for the item container.
    pub item_names: HashMap<u32, String>,
    pub list: Option<DropList>,
    pub held: Option<Target>,
    pub hovered: Option<Target>,
    dirty: bool,
    clock: u32,
    regenerated: u32,
    built: bool,
}

impl SaveScreen {
    /// An empty continue form for `game_mode` at `screen` pixels and GUI
    /// `unit`.
    pub fn new(game_mode: i32, screen: (i32, i32), unit: i32) -> Self {
        let mut s = Self {
            game_mode,
            screen,
            unit: unit.max(1),
            compact: false,
            main: Vec::new(),
            journal_combo: Vec::new(),
            confirm: None,
            reset: None,
            opened: None,
            index: 0,
            kind: 0,
            availability: 0,
            numbers: Vec::new(),
            names: Vec::new(),
            record: CharacterRecord::reset(),
            journal: Vec::new(),
            journal_entry: -1,
            item_names: HashMap::new(),
            list: None,
            held: None,
            hovered: None,
            dirty: true,
            clock: 0,
            regenerated: 0,
            built: false,
        };
        s.lay_out();
        s
    }

    /// The character screen's entry over `list`: `None` when the manager
    /// holds no slot; otherwise the first slot is selected with kind 1, the
    /// form takes kind 2 when only the checkpoint is there and kind 1
    /// otherwise (the creator's show in continue mode), and it refreshes.
    pub fn open(
        list: &mut SlotList,
        game_mode: i32,
        screen: (i32, i32),
        unit: i32,
        item_names: HashMap<u32, String>,
    ) -> Option<Self> {
        if !list.any() {
            return None;
        }
        let mut s = Self::new(game_mode, screen, unit);
        s.item_names = item_names;
        let found = list.character_screen_select(0, SlotKind::Save.code());
        s.availability = found.availability;
        s.kind = if found.availability == SlotKind::Checkpoint.code() as u32 {
            SlotKind::Checkpoint.code()
        } else {
            SlotKind::Save.code()
        };
        let asks = found
            .loaded
            .as_ref()
            .is_some_and(|loaded| offers_portal_reset(game_mode, loaded));
        if let Some(loaded) = found.loaded {
            s.opened = Some(loaded.kind);
            s.show(&loaded);
        }
        s.refresh(list);
        if asks {
            s.ask_portal_reset();
        }
        Some(s)
    }

    pub fn design(&self) -> (i32, i32) {
        design(self.screen, self.unit)
    }

    fn lay_out(&mut self) {
        let d = self.design();
        self.compact = d.1 < COMPACT_BELOW;
        let mut main = scaled(
            &creation_forms::lay_out(&creation_forms::continue_records(self.game_mode), d),
            self.unit,
        );
        // Keep the live words (texts, states) across a re-layout.
        for (new, old) in main.iter_mut().zip(&self.main) {
            new.text = old.text.clone();
            new.state = old.state;
        }
        self.main = main;
        let journal = (JOURNAL_PLACE.2, self.journal_height());
        self.journal_combo = scaled(
            &creation_forms::lay_out(&JOURNAL_RECORDS, journal),
            self.unit,
        );
        if self.confirm.is_some() {
            self.confirm = Some(confirm_controls(self.unit));
        }
        if self.reset.is_some() {
            self.reset = Some(reset_controls(self.unit));
        }
    }

    /// Follows the window's size and unit.
    pub fn resize(&mut self, screen: (i32, i32), unit: i32) {
        if (screen, unit.max(1)) == (self.screen, self.unit) {
            return;
        }
        self.screen = screen;
        self.unit = unit.max(1);
        self.lay_out();
    }

    fn journal_height(&self) -> i32 {
        if self.compact {
            JOURNAL_COMPACT_HEIGHT
        } else {
            JOURNAL_HEIGHT
        }
    }

    /// The preview viewport in window pixels (the creation form's).
    pub fn preview_px(&self) -> GuiRect {
        let f = create::preview_frame(self.design());
        GuiRect::new(
            px(f.x, self.unit),
            px(f.y, self.unit),
            px(f.width, self.unit),
            px(f.height, self.unit),
        )
    }

    /// The item container's rectangle.
    pub fn inventory_px(&self) -> GuiRect {
        let (x, y, w, h) = INVENTORY_PLACE;
        GuiRect::new(
            px(x, self.unit),
            px(y, self.unit),
            px(w, self.unit),
            px(h, self.unit),
        )
    }

    /// The journal form's rectangle.
    pub fn journal_px(&self) -> GuiRect {
        let (x, y, w) = JOURNAL_PLACE;
        GuiRect::new(
            px(x, self.unit),
            px(y, self.unit),
            px(w, self.unit),
            px(self.journal_height(), self.unit),
        )
    }

    /// The journal text box's rectangle.
    pub fn journal_text_px(&self) -> GuiRect {
        let form = self.journal_px();
        let (x, y, w) = JOURNAL_TEXT_AT;
        GuiRect::new(
            form.left + px(x, self.unit),
            form.top + px(y, self.unit),
            px(w, self.unit),
            form.height() - px(y, self.unit),
        )
    }

    /// The delete box's rectangle, centred.
    pub fn confirm_px(&self) -> GuiRect {
        let (w, h) = (px(DELETE_SIZE.0, self.unit), px(DELETE_SIZE.1, self.unit));
        GuiRect::new((self.screen.0 - w) / 2, (self.screen.1 - h) / 2, w, h)
    }

    pub(crate) fn confirm_client(&self) -> GuiRect {
        let frame = Frame::new(self.confirm_px(), self.unit, DELETE_TITLE);
        frame.client()
    }

    /// The open drop-down's owner, rows and rectangle.
    pub fn list_px(&self, row: i32) -> Option<(GuiRect, usize)> {
        let which = self.list?;
        let (owner, count) = match which {
            DropList::Slots => {
                let c = &self.main[saves::SLOTS];
                (GuiRect::new(c.x, c.y, c.w, c.h), self.names.len())
            }
            DropList::Journal => {
                let form = self.journal_px();
                let c = &self.journal_combo[0];
                (
                    GuiRect::new(form.left + c.x, form.top + c.y, c.w, c.h),
                    self.journal.len(),
                )
            }
        };
        let place = exa_ui::controls::drop_down_place(
            (owner.left, owner.top, owner.width(), owner.height()),
            row * count as i32 + 4,
            2,
            row,
            self.screen.1,
        );
        Some((GuiRect::new(place.x, place.y, place.w, place.h), count))
    }

    fn row_height(&self) -> i32 {
        // [guess] a row as tall as the combo box.
        self.main[saves::SLOTS].h.max(1)
    }

    /// Takes the loaded save's character and journal for the view.
    fn show(&mut self, loaded: &crate::slots::LoadedSave) {
        let mut record = loaded
            .player_record()
            .unwrap_or_else(CharacterRecord::reset);
        // The select clears the record's race byte as the load does.
        record.reset_loaded_identity();
        self.record = record;
        self.journal = loaded.data.journal().ok().flatten().unwrap_or_default();
        self.journal_entry = if self.journal.is_empty() { -1 } else { 0 };
        self.dirty = true;
    }

    /// The form's select of the slot at `index` (the original's select
    /// callback).
    pub fn select(&mut self, list: &mut SlotList, index: i32) {
        self.index = index;
        if self.kind == 0 {
            self.kind = 1;
        }
        let found = list.character_screen_select(self.index, self.kind);
        if found.availability != 0 {
            self.availability = found.availability;
            let asks = found
                .loaded
                .as_ref()
                .is_some_and(|loaded| offers_portal_reset(self.game_mode, loaded));
            if let Some(loaded) = found.loaded {
                self.opened = Some(loaded.kind);
                self.show(&loaded);
            }
            if self.kind == 2 && self.availability & 2 == 0 {
                self.kind = 1;
            } else if self.kind != 2 && self.availability & 1 == 0 {
                self.kind = 2;
            }
            self.refresh(list);
            // The original runs the refresh and this check behind the
            // select's fade.
            if asks {
                self.ask_portal_reset();
            }
        }
    }

    /// The "Reset to portal" box comes up over the screen (the original's
    /// check after a select found a save from before 0.9.5 that went past
    /// the portal, [`offers_portal_reset`]).
    pub fn ask_portal_reset(&mut self) {
        info!(
            "saves: slot index {} asks: {}",
            self.index,
            exa_ui::dialogs::RESET_QUESTION
        );
        self.reset = Some(reset_controls(self.unit));
    }

    /// The box's Yes: the file the select read is deleted and, without a
    /// checkpoint left, the slot leaves the list
    /// ([`SlotList::reset_to_portal`]); the same index is selected again
    /// with the form's kind, and the box closes.
    pub fn confirm_portal_reset(&mut self, list: &mut SlotList) {
        self.reset = None;
        let opened = self.opened.unwrap_or(SlotKind::Save);
        match list.reset_to_portal(opened) {
            Ok(dropped) => info!(
                "saves: reset to the portal: the slot's {} file is deleted{}",
                opened.extension(),
                if dropped {
                    "; no checkpoint is left, so the slot leaves the list"
                } else {
                    ""
                }
            ),
            Err(error) => warn!("saves: cannot reset the slot to the portal: {error}"),
        }
        self.select(list, self.index);
    }

    /// The reset box's place: the delete box's size, centred.
    pub fn reset_px(&self) -> GuiRect {
        self.confirm_px()
    }

    pub(crate) fn reset_client(&self) -> GuiRect {
        let frame = Frame::new(self.reset_px(), self.unit, RESET_TO_PORTAL.title);
        frame.client()
    }

    /// The refresh: the combo box's items and the character's name, the
    /// step buttons' disable, the toggle's label, mirror and enable.
    pub fn refresh(&mut self, list: &SlotList) {
        self.numbers = list.numbers().to_vec();
        self.names = list
            .names()
            .iter()
            .zip(list.numbers())
            .map(|(name, number)| {
                name.clone()
                    .unwrap_or_else(|| format!("{number:03}"))
                    .into_bytes()
            })
            .collect();
        let count = self.numbers.len() as i64;
        let mut name = self.record.name.as_bytes().to_vec();
        name.truncate(15);
        self.main[saves::SLOTS].text = name;
        self.main[saves::PREVIOUS].state = if self.index < 1 {
            gui_controls::state::DISABLED
        } else {
            0
        };
        self.main[saves::NEXT].state = if i64::from(self.index) < count - 1 {
            0
        } else {
            gui_controls::state::DISABLED
        };
        let button = create::creation_continue_button(self.kind, self.availability as i32);
        let toggle = &mut self.main[saves::CHECKPOINT];
        toggle.text = button.label.as_bytes().to_vec();
        if button.mirrored {
            toggle.style |= 0x4000_0000;
        } else {
            toggle.style &= !0x4000_0000;
        }
        if !hidden(toggle) {
            if button.enabled {
                toggle.state &= !gui_controls::state::DISABLED;
            } else {
                toggle.state |= gui_controls::state::DISABLED;
            }
        }
    }

    /// Previous: the slot before, read from its save first.
    pub fn previous(&mut self, list: &mut SlotList) {
        if self.index > 0 {
            self.kind = 1;
            self.select(list, self.index - 1);
        }
    }

    /// Next: the slot after, read from its save first.
    pub fn next(&mut self, list: &mut SlotList) {
        if i64::from(self.index) < list.count() as i64 - 1 {
            self.kind = 1;
            self.select(list, self.index + 1);
        }
    }

    /// The combo box's pick of the 1-based `item`.
    pub fn pick(&mut self, list: &mut SlotList, item: i32) {
        if item - 1 != self.index {
            self.kind = 1;
            self.select(list, item - 1);
        }
    }

    /// The toggle: the save (kind 1) when the checkpoint (kind 2) is shown
    /// and the save is there, else the checkpoint when it is there.
    pub fn toggle(&mut self, list: &mut SlotList) {
        if self.kind == 2 {
            if self.availability & 1 == 0 {
                return;
            }
            self.kind = 1;
        } else {
            if self.availability & 2 == 0 {
                return;
            }
            self.kind = 2;
        }
        self.select(list, self.index);
    }

    /// The delete box's question for the shown character.
    pub fn delete_question(&self) -> Vec<u8> {
        let mut text = DELETE_RECORDS[delete_box::QUESTION]
            .text
            .unwrap_or("")
            .as_bytes()
            .to_vec();
        let mut name = self.record.name.as_bytes().to_vec();
        name.truncate(15);
        text.extend(name);
        text.extend(DELETE_QUESTION_END.as_bytes());
        text
    }

    /// Delete: the box comes up.
    pub fn ask_delete(&mut self) {
        let mut controls = confirm_controls(self.unit);
        controls[delete_box::QUESTION].text = self.delete_question();
        self.confirm = Some(controls);
    }

    /// The box's Delete: the slot goes, the index stays inside the list
    /// and is selected again; an empty list leaves to the menu.
    pub fn confirm_delete(&mut self, list: &mut SlotList) -> SaveAction {
        self.confirm = None;
        if let Some(index) = usize::try_from(self.index).ok()
            && let Err(error) = list.delete_index(index)
        {
            warn!("saves: cannot delete slot {index}: {error}");
        }
        if list.count() <= self.index {
            self.index = list.count() - 1;
        }
        if self.index < 0 {
            return SaveAction::Leave;
        }
        self.select(list, self.index);
        SaveAction::None
    }

    /// CONTINUE: the current slot with the form's kind.
    pub fn continue_action(&self, list: &SlotList) -> SaveAction {
        if list.current() <= 0 {
            return SaveAction::None;
        }
        SaveAction::Continue {
            number: list.current() as u32,
            prefer_checkpoint: self.kind == SlotKind::Checkpoint.code(),
        }
    }

    /// One frame of the preview's clock.
    pub fn tick(&mut self) {
        self.clock = self.clock.wrapping_add(1);
    }

    /// Whether the preview's figure is rebuilt now: on a new character,
    /// with the creator's two-frame rule.
    pub fn take_regeneration(&mut self) -> bool {
        if !self.dirty {
            return false;
        }
        if self.built && u64::from(self.regenerated) + 2 > u64::from(self.clock) {
            return false;
        }
        self.regenerated = self.clock;
        self.built = true;
        self.dirty = false;
        true
    }

    /// The item container's lines: the held, worn and carried items' names.
    pub fn inventory_lines(&self) -> Vec<String> {
        let name = |id: u32| {
            let key = id & exa_formats::characters::ID_MASK;
            self.item_names
                .get(&key)
                .or_else(|| self.item_names.get(&id))
                .cloned()
                .unwrap_or_else(|| format!("#{key}"))
        };
        self.record
            .hands
            .iter()
            .filter(|hand| hand.id != 0)
            .map(|hand| hand.id)
            .chain(self.record.worn.iter().map(|worn| worn.id))
            .chain(self.record.inventory.iter().map(|c| c.item))
            .filter(|&id| id != 0)
            .map(name)
            .collect()
    }

    /// What is under the window point.
    pub fn hit(&self, x: i32, y: i32) -> Option<Target> {
        if let Some(reset) = &self.reset {
            let client = self.reset_client();
            for (i, c) in reset.iter().enumerate() {
                if takes_hits(c) && rect_of(c, client.left, client.top).contains(x, y) {
                    return Some(Target::Reset(i));
                }
            }
            return None;
        }
        if let Some(confirm) = &self.confirm {
            let client = self.confirm_client();
            for (i, c) in confirm.iter().enumerate() {
                if takes_hits(c) && rect_of(c, client.left, client.top).contains(x, y) {
                    return Some(Target::Confirm(i));
                }
            }
            return None;
        }
        if let Some((rect, count)) = self.list_px(self.row_height()) {
            if rect.contains(x, y) {
                let row = ((y - rect.top - 2) / self.row_height()).max(0) as usize;
                return (row < count).then_some(Target::Row(row));
            }
            return None;
        }
        let form = self.journal_px();
        if rect_of(&self.journal_combo[0], form.left, form.top).contains(x, y)
            && !self.journal.is_empty()
        {
            return Some(Target::JournalCombo);
        }
        for slop in [false, true] {
            for (i, c) in self.main.iter().enumerate() {
                if takes_hits(c)
                    && exa_ui::form::control_hit(
                        x,
                        y,
                        (c.x, c.y, c.x + c.w, c.y + c.h),
                        slop,
                        self.unit,
                    )
                {
                    return Some(Target::Control(i));
                }
            }
        }
        None
    }

    fn controls_of(&mut self, target: Target) -> Option<(&mut Vec<Control>, usize)> {
        match target {
            Target::Control(i) => Some((&mut self.main, i)),
            Target::Confirm(i) => self.confirm.as_mut().map(|c| (c, i)),
            Target::Reset(i) => self.reset.as_mut().map(|c| (c, i)),
            _ => None,
        }
    }

    fn send(&mut self, target: Target, m: Message) -> bool {
        let unit = self.unit;
        let Some((controls, i)) = self.controls_of(target) else {
            return false;
        };
        let mut events = Vec::new();
        gui_controls::dispatch(controls, i, m, unit, &mut events);
        events
            .iter()
            .any(|e| matches!(e, ControlEvent::Changed(at) if *at == i))
    }

    /// The pointer moved: the hover follows it.
    pub fn pointer(&mut self, x: i32, y: i32) {
        let hit = self.hit(x, y);
        if let Some(held) = self.held {
            self.send(
                held,
                Message {
                    message: message::HOVER,
                    over: hit == Some(held),
                    ..Message::default()
                },
            );
            return;
        }
        if hit != self.hovered {
            for (target, over) in [(self.hovered, false), (hit, true)] {
                let Some(target) = target else {
                    continue;
                };
                if let Some((controls, i)) = self.controls_of(target)
                    && let Some(c) = controls.get_mut(i)
                {
                    if over {
                        c.state = (c.state & !0xf_0000) | 0x1_0000;
                    } else {
                        c.state &= !0xf_0000;
                    }
                }
                self.send(
                    target,
                    Message {
                        message: message::HOVER,
                        over,
                        ..Message::default()
                    },
                );
            }
            self.hovered = hit;
        }
    }

    /// A left press.
    pub fn press(&mut self, list: &mut SlotList, x: i32, y: i32) {
        let hit = self.hit(x, y);
        match hit {
            Some(Target::Row(row)) => {
                match self.list.take() {
                    Some(DropList::Slots) => self.pick(list, row as i32 + 1),
                    Some(DropList::Journal) => self.journal_entry = row as i32,
                    None => {}
                }
                return;
            }
            Some(Target::Control(saves::SLOTS)) => {
                self.list = Some(DropList::Slots);
                return;
            }
            Some(Target::JournalCombo) => {
                self.list = Some(DropList::Journal);
                return;
            }
            None => {
                // A press outside an open list closes it.
                self.list = None;
                return;
            }
            _ => {}
        }
        let Some(target) = hit else {
            return;
        };
        self.held = Some(target);
        self.send(
            target,
            Message {
                message: message::PRESS,
                param: x as u32,
                extra: 1,
                over: true,
                ..Message::default()
            },
        );
    }

    /// A left release: a button released over itself answers.
    pub fn release(&mut self, list: &mut SlotList, x: i32, y: i32) -> SaveAction {
        let Some(held) = self.held.take() else {
            return SaveAction::None;
        };
        let over = self.hit(x, y) == Some(held);
        let changed = self.send(
            held,
            Message {
                message: message::RELEASE,
                param: gui_controls::RELEASE_LANDS,
                extra: 1,
                over,
                ..Message::default()
            },
        );
        if !changed || !over {
            return SaveAction::None;
        }
        self.answer(list, held)
    }

    /// A button's callback.
    pub fn answer(&mut self, list: &mut SlotList, target: Target) -> SaveAction {
        match target {
            Target::Control(saves::PREVIOUS) => self.previous(list),
            Target::Control(saves::NEXT) => self.next(list),
            Target::Control(saves::DELETE) => self.ask_delete(),
            Target::Control(saves::CHECKPOINT) => self.toggle(list),
            Target::Control(saves::BACK) => return SaveAction::Leave,
            Target::Control(saves::CONTINUE) => return self.continue_action(list),
            Target::Confirm(delete_box::DELETE) => return self.confirm_delete(list),
            Target::Confirm(delete_box::CANCEL) => self.confirm = None,
            Target::Reset(reset_box::YES) => self.confirm_portal_reset(list),
            // No closes the box with result 0.
            Target::Reset(reset_box::NO) => self.reset = None,
            _ => {}
        }
        SaveAction::None
    }

    /// The cancel key: the box or the list closes, else the form leaves.
    pub fn cancel(&mut self) -> SaveAction {
        if self.reset.take().is_some()
            || self.confirm.take().is_some()
            || self.list.take().is_some()
        {
            return SaveAction::None;
        }
        SaveAction::Leave
    }
}

fn rect_of(c: &Control, ox: i32, oy: i32) -> GuiRect {
    GuiRect::new(ox + c.x, oy + c.y, c.w, c.h)
}

/// The delete box's records at `unit`, relative to its client [guess: laid
/// out at the box's design size].
fn confirm_controls(unit: i32) -> Vec<Control> {
    scaled(&creation_forms::lay_out(&DELETE_RECORDS, DELETE_SIZE), unit)
}

/// The reset box's records at `unit`, relative to its client [guess: laid
/// out at the box's design size, as the delete box's].
fn reset_controls(unit: i32) -> Vec<Control> {
    scaled(&creation_forms::lay_out(&RESET_RECORDS, RESET_SIZE), unit)
}

/// The reset box's design size: every confirmation dialog's.
const RESET_SIZE: (i32, i32) = (RESET_TO_PORTAL.width, RESET_TO_PORTAL.height);

/// `--open-saves` / `--load-slot`: open the saves' screen once the load has
/// ended, and continue a slot from it.
#[derive(Resource, Debug, Clone, Default)]
pub struct ScriptedSaveScreen {
    /// The game mode to open for (4 the story, 8 the arena).
    pub open: Option<i32>,
    /// The slot number to continue, and whether from its checkpoint first.
    pub load: Option<(u32, bool)>,
    pub done: bool,
    /// `--portal-reset yes|no`: the answer given to the "Reset to portal"
    /// box once it has been up [`PORTAL_RESET_WAIT`] frames (a debug
    /// trigger; without it the box waits for the player).
    pub portal_reset: Option<bool>,
    /// Frames the reset box has been up for the scripted answer.
    pub reset_frames: u32,
    /// `--saves-press`: the screen's buttons pressed in turn, one every
    /// [`PORTAL_RESET_WAIT`] frames while no reset box is up (a debug
    /// trigger).
    pub presses: Vec<Target>,
    /// Frames since the screen opened or the last scripted press.
    pub press_frames: u32,
}

/// Reads one `--saves-press` name: `previous`, `next`, `delete`, `toggle`,
/// `continue`, `back`, and the delete box's `confirm` and `cancel`.
pub fn press_target(name: &str) -> Option<Target> {
    Some(match name {
        "previous" => Target::Control(saves::PREVIOUS),
        "next" => Target::Control(saves::NEXT),
        "delete" => Target::Control(saves::DELETE),
        "toggle" => Target::Control(saves::CHECKPOINT),
        "continue" => Target::Control(saves::CONTINUE),
        "back" => Target::Control(saves::BACK),
        "confirm" => Target::Confirm(delete_box::DELETE),
        "cancel" => Target::Confirm(delete_box::CANCEL),
        _ => return None,
    })
}

/// The frames the scripted answer to the reset box waits, so a screenshot
/// can show the box first.
pub const PORTAL_RESET_WAIT: u32 = 120;

/// The saves' screen's input and paint, after the creator's.
pub struct SaveScreenPlugin;

impl Plugin for SaveScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScriptedSaveScreen>()
            .init_resource::<CreatorScreen>()
            .init_resource::<GuiFrames>()
            .add_message::<MenuChoice>()
            .add_message::<crate::save_live::StorySlotLoad>()
            .add_systems(
                Update,
                (open_saves, saves_input, saves_paint)
                    .chain()
                    .in_set(CreatorSystems),
            );
    }
}

fn item_names(game: Option<&exa_assets::Game>) -> HashMap<u32, String> {
    game.and_then(|g| g.0.pack(exa_assets::PackKind::Resource))
        .and_then(|p| p.get("objdb.rdb"))
        .and_then(|b| exa_formats::rdb::Database::parse(b).ok())
        .map(|db| {
            exa_formats::objdb::items(&db)
                .into_iter()
                .map(|item| (item.id & exa_formats::characters::ID_MASK, item.name))
                .collect()
        })
        .unwrap_or_default()
}

/// Opens the screen for a CONTINUE row (or `--open-saves`), hiding the menu
/// as the creator's show does.
#[allow(clippy::too_many_arguments)]
fn open_saves(
    mut choices: MessageReader<MenuChoice>,
    mut scripted: ResMut<ScriptedSaveScreen>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    lists: Option<ResMut<crate::save_live::SlotLists>>,
    mut screen: ResMut<CreatorScreen>,
    mut flow: Option<ResMut<Flow>>,
    mut steps: Option<ResMut<FlowSteps>>,
    mut menu: Option<ResMut<crate::menu_screen::MenuScreen>>,
    game: Option<Res<exa_assets::Game>>,
    mut textures: Option<ResMut<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let mut wanted = None;
    for MenuChoice(command) in choices.read() {
        if let MenuCommand::CharacterScreen(mode) = *command {
            wanted = Some(if mode == game_mode::ARENA {
                ARENA
            } else {
                STORY
            });
        }
    }
    if !scripted.done && loading.is_none() && scripted.open.is_some() {
        scripted.done = scripted.load.is_none();
        wanted = scripted.open;
        if scripted.load.is_some() {
            // The load continues from the open screen next frame.
            scripted.open = None;
        }
    }
    let Some(mode) = wanted else {
        return;
    };
    let Some(mut lists) = lists else {
        warn!("saves: {}", exa_ui::creation::SAVE_MANAGER_MISSING);
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let (size, unit) = creator_screen::screen_and_unit(window);
    let list = if mode == ARENA {
        &mut lists.arena
    } else {
        &mut lists.story
    };
    let Some(saves) = SaveScreen::open(list, mode, size, unit, item_names(game.as_deref())) else {
        // The original falls back to the free creator; the menu greys the
        // row out, so nothing opens.
        info!("saves: no {mode:#x} slot to continue");
        return;
    };
    info!(
        "saves: the character screen opens for game mode {mode:#x} with {} slots (shown {:?}, availability {})",
        saves.numbers.len(),
        saves.record.name,
        saves.availability
    );
    if let Some(menu) = menu.as_mut()
        && menu.open
    {
        menu.close();
    }
    if let Some(steps) = steps.as_mut() {
        steps.steps.push(crate::flow::Step::HideMenu);
        steps.steps.push(crate::flow::Step::TeardownWorld);
        steps.steps.push(crate::flow::Step::Controls(false));
    }
    if let Some(textures) = textures.as_mut() {
        creator_screen::ensure_texture(textures, game.as_deref(), ICONS);
    }
    screen.class_select = None;
    screen.creator = None;
    screen.saves = Some(saves);
    if let Some(flow) = flow.as_mut() {
        // The show leaves the creator's session state; the character
        // screen does not copy the game mode.
        flow.phase = Phase::from_word(MenuShow::CharacterScreen.state());
        flow.menu_open = false;
    }
}

#[allow(clippy::too_many_arguments)]
fn saves_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    pointer_script: Option<Res<exa_ui::ScriptedPointer>>,
    mut scripted: ResMut<ScriptedSaveScreen>,
    mut screen: ResMut<CreatorScreen>,
    lists: Option<ResMut<crate::save_live::SlotLists>>,
    mut flow: Option<ResMut<Flow>>,
    mut steps: Option<ResMut<FlowSteps>>,
    mut commands: Commands,
    mut loads: MessageWriter<crate::save_live::StorySlotLoad>,
    mut roll: Local<u32>,
    levels: Query<(), With<exa_world::level_lifecycle::LevelEntity>>,
) {
    let Some(saves) = screen.saves.as_mut() else {
        return;
    };
    let Some(mut lists) = lists else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let (size, unit) = creator_screen::screen_and_unit(window);
    saves.tick();
    saves.resize(size, unit);
    let list = if saves.game_mode == ARENA {
        &mut lists.arena
    } else {
        &mut lists.story
    };
    let mut actions = Vec::new();
    // The scripted CONTINUE waits until the screen's own teardown has taken
    // the old level down, as a player's click comes frames later.
    let settled = levels.is_empty();
    if settled && let Some((number, checkpoint)) = scripted.load.take() {
        scripted.done = true;
        match list.numbers().iter().position(|&n| n == number) {
            Some(index) => {
                saves.kind = if checkpoint {
                    SlotKind::Checkpoint.code()
                } else {
                    SlotKind::Save.code()
                };
                saves.select(list, index as i32);
                info!(
                    "saves: scripted CONTINUE of slot {number} (kind {})",
                    saves.kind
                );
                actions.push(saves.continue_action(list));
            }
            None => warn!("--load-slot: slot {number} is not listed"),
        }
    }
    if let Some(yes) = scripted.portal_reset {
        if saves.reset.is_some() {
            scripted.reset_frames += 1;
            if scripted.reset_frames >= PORTAL_RESET_WAIT {
                scripted.reset_frames = 0;
                let target = if yes { reset_box::YES } else { reset_box::NO };
                info!(
                    "saves: --portal-reset answers {}",
                    if yes { "Yes" } else { "No" }
                );
                actions.push(saves.answer(list, Target::Reset(target)));
            }
        } else {
            scripted.reset_frames = 0;
        }
    }
    if !scripted.presses.is_empty() && saves.reset.is_none() {
        scripted.press_frames += 1;
        if scripted.press_frames >= PORTAL_RESET_WAIT {
            scripted.press_frames = 0;
            let target = scripted.presses.remove(0);
            info!(
                "saves: --saves-press {target:?} (slot index {}, kind {}, availability {})",
                saves.index, saves.kind, saves.availability
            );
            actions.push(saves.answer(list, target));
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        actions.push(saves.cancel());
    }
    let pointer = match pointer_script.as_deref() {
        Some(scripted) => scripted.position(window),
        None => window.cursor_position(),
    }
    .map(|at| {
        let scale = window.scale_factor();
        ((at.x * scale) as i32, (at.y * scale) as i32)
    });
    if let Some((x, y)) = pointer {
        saves.pointer(x, y);
        if mouse.just_pressed(MouseButton::Left) {
            saves.press(list, x, y);
        }
        if mouse.just_released(MouseButton::Left) {
            actions.push(saves.release(list, x, y));
        }
    }
    let mode = saves.game_mode;
    for action in actions {
        match action {
            SaveAction::None => {}
            SaveAction::Leave => {
                info!("saves: leaves to the menu");
                screen.saves = None;
                if let (Some(flow), Some(steps)) = (flow.as_mut(), steps.as_mut()) {
                    steps.steps.extend(flow.quit_to_menu());
                }
                return;
            }
            SaveAction::Continue {
                number,
                prefer_checkpoint,
            } => {
                info!(
                    "saves: CONTINUE slot {number} (checkpoint first: {prefer_checkpoint}, game mode {mode:#x})"
                );
                screen.saves = None;
                if mode == ARENA {
                    commands.insert_resource(crate::save_live::ArenaSlotLoad { number });
                    if let (Some(flow), Some(steps)) = (flow.as_mut(), steps.as_mut()) {
                        let viewport = flow.viewport;
                        let at = crate::flow::Placement {
                            rotation: crate::flow::NO_ROTATION,
                            position: Vec3::ZERO,
                        };
                        let mut resumed = flow.continue_arena(at, viewport, &mut roll);
                        // The hub places its manager itself.
                        resumed.retain(|step| {
                            !matches!(
                                step,
                                crate::flow::Step::PlacePlayer(_) | crate::flow::Step::Camera(_)
                            )
                        });
                        steps.steps.extend(flow.continue_session(resumed));
                    }
                } else {
                    loads.write(crate::save_live::StorySlotLoad {
                        number,
                        prefer_checkpoint,
                    });
                }
                return;
            }
        }
    }
}

/// A solid fill of rectangles in one ink.
fn fill_layer(rects: &[GuiRect], ink: [u8; 4]) -> GuiLayer {
    let mut batch = SkinBatch::new();
    batch.select(1, 0, false);
    batch.set_ink(ink);
    batch.begin();
    for r in rects {
        let (x, y, w, h) = (
            r.left as f32,
            r.top as f32,
            r.width() as f32,
            r.height() as f32,
        );
        batch.vertex(x, y);
        batch.vertex(x, y + h);
        batch.vertex(x + w, y + h);
        batch.vertex(x + w, y);
    }
    batch.end_quads();
    GuiLayer::new(batch, LayerTexture::None)
}

/// A combo box: its frame (the field's frame style) and its text, left in
/// its span.
fn paint_combo(
    c: &Control,
    origin: (i32, i32),
    unit: i32,
    palette: &SkinPalette,
    font: &GuiFont,
    size: Option<(u16, u16)>,
    painted: &mut Painted,
) {
    if hidden(c) {
        return;
    }
    let rect = rect_of(c, origin.0, origin.1);
    let mut batch = SkinBatch::new();
    exa_ui::skin::draw_style_frames(
        &mut batch,
        exa_ui::skin::GuiRect::sized(
            rect.left as f32,
            rect.top as f32,
            rect.width() as f32,
            rect.height() as f32,
        ),
        c.style | 0x20,
        palette,
    );
    painted.push(GuiLayer::new(
        batch,
        LayerTexture::Named(exa_ui::gui_draw::SKIN_TEXTURE),
    ));
    if let Some((ax, ay)) = exa_ui::controls::combo_arrow(
        (rect.left, rect.top, rect.width(), rect.height()),
        unit,
        c.style,
    ) {
        painted.push(fill_layer(
            &[GuiRect::new(ax, ay + unit, unit * 2, unit)],
            palette.text[6],
        ));
    }
    let span = exa_ui::controls::combo_text_span(c.w, unit, unit);
    let shown = exa_ui::gui_text::ellipsis_fit(font, &c.text, span.max(0));
    let baseline = rect.top + ((rect.height() - font.line_height) >> 1);
    if let Some(layer) = creator_screen::text_layer(
        font,
        size,
        &shown,
        (rect.left + unit * 2, baseline),
        palette.text[6],
    ) {
        painted.push(layer);
    }
}

fn wrapped_layer(
    font: &GuiFont,
    size: Option<(u16, u16)>,
    text: &[u8],
    rect: GuiRect,
    ink: [u8; 4],
) -> Option<GuiLayer> {
    let size = size?;
    let drawn = exa_ui::gui_text::draw_wrapped(
        font,
        text,
        rect.left,
        rect.top,
        rect.width(),
        rect.height(),
        0,
        false,
    );
    Some(GuiLayer::new(
        text_batch(font, &drawn.quads, ink, size),
        LayerTexture::Named(font.texture),
    ))
}

/// The continue form's paint.
pub fn paint_saves(saves: &SaveScreen, textures: &GuiTextures) -> Painted {
    let palette = shown_palette(1.0);
    let unit = saves.unit;
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, size) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let text = crate::settings_form::Text { font: &font, size };
    // The continue form through the game skin's form draw: the creation
    // form's words draw nothing of its own; its children are held by it.
    let screen_rect = GuiRect::new(0, 0, saves.screen.0, saves.screen.1);
    let (mut form, _) = gui_paint::paint_top_form(
        screen_rect,
        unit,
        &palette,
        1.0,
        creator_screen::CREATION_WORDS,
        None,
    );
    let mut painted = Painted {
        client: screen_rect,
        ..Painted::default()
    };
    painted.layers.extend(form.under());
    painted.layers.extend(form.over());
    // The preview viewport, cleared for the 3D view under it.
    let view = saves.preview_px();
    let preview = create::preview_view();
    painted.layers.extend(creator_screen::creation_child(
        view,
        saves.screen,
        unit,
        &palette,
        creator_screen::preview_words(),
    ));
    painted.push(crate::gui_screens::clear_layer(GuiRect::new(
        view.left + preview.border,
        view.top + preview.border,
        view.width() - preview.border * 2,
        view.height() - preview.border * 2,
    )));
    for c in &saves.main {
        if c.class == exa_ui::controls::COMBO_BOX {
            paint_combo(c, (0, 0), unit, &palette, &font, size, &mut painted);
        } else {
            creator_screen::paint_record(
                c,
                (0, 0),
                unit,
                &palette,
                &text,
                false,
                textures,
                &mut painted,
            );
        }
    }
    // The item container: its panel and, [guess] for its icons, the
    // entries' names.
    let inventory = saves.inventory_px();
    painted.layers.extend(creator_screen::creation_child(
        inventory,
        saves.screen,
        unit,
        &palette,
        gui_paint::pane_words(0x80),
    ));
    let lines = saves.inventory_lines().join("\n");
    if let Some(layer) = wrapped_layer(
        &font,
        size,
        lines.as_bytes(),
        GuiRect::new(
            inventory.left + unit * 2,
            inventory.top + unit,
            inventory.width() - unit * 4,
            inventory.height() - unit * 2,
        ),
        palette.text[6],
    ) {
        painted.push(layer);
    }
    // The journal form: its entry combo box and the entry's text.
    let journal = saves.journal_px();
    let mut combo = saves.journal_combo[0].clone();
    if let Some(entry) = usize::try_from(saves.journal_entry)
        .ok()
        .and_then(|at| saves.journal.get(at))
    {
        combo.text = entry.title.as_bytes().to_vec();
    }
    paint_combo(
        &combo,
        (journal.left, journal.top),
        unit,
        &palette,
        &font,
        size,
        &mut painted,
    );
    if let Some(entry) = usize::try_from(saves.journal_entry)
        .ok()
        .and_then(|at| saves.journal.get(at))
        && let Some(layer) = wrapped_layer(
            &font,
            size,
            entry.text.as_bytes(),
            saves.journal_text_px(),
            palette.text[6],
        )
    {
        painted.push(layer);
    }
    // The open drop-down list.
    let row = saves.row_height();
    if let Some((rect, count)) = saves.list_px(row) {
        painted.layers.extend(creator_screen::creation_child(
            rect,
            saves.screen,
            unit,
            &palette,
            gui_paint::pane_words(0x80),
        ));
        for i in 0..count {
            let item: &[u8] = match saves.list {
                Some(DropList::Slots) => &saves.names[i],
                Some(DropList::Journal) => saves.journal[i].title.as_bytes(),
                None => &[],
            };
            let top = rect.top + 2 + i as i32 * row;
            if saves.hovered == Some(Target::Row(i)) {
                let mut ink = palette.highlight[7];
                ink[3] = 96;
                painted.push(fill_layer(
                    &[GuiRect::new(rect.left + 1, top, rect.width() - 2, row)],
                    ink,
                ));
            }
            if let Some(layer) = creator_screen::text_layer(
                &font,
                size,
                item,
                (rect.left + unit * 2, top + ((row - font.line_height) >> 1)),
                palette.text[6],
            ) {
                painted.push(layer);
            }
        }
    }
    // The delete box over everything.
    if let Some(confirm) = &saves.confirm {
        // The delete box's words: style 0xb800, the upper layer (0x1000);
        // a top-level form, titled, shadowed and corner-framed.
        let (mut form, _) = gui_paint::paint_top_form(
            saves.confirm_px(),
            unit,
            &palette,
            1.0,
            gui_paint::FormWords {
                style: creation_forms::DELETE_STYLE,
                attributes: gui_paint::FORM_ATTRIBUTES | creation_forms::DELETE_ATTRIBUTES,
            },
            Some((DELETE_TITLE, &font, size)),
        );
        painted.layers.extend(form.under());
        let client = saves.confirm_client();
        for c in confirm {
            creator_screen::paint_record(
                c,
                (client.left, client.top),
                unit,
                &palette,
                &text,
                false,
                textures,
                &mut painted,
            );
        }
        painted.layers.extend(form.over());
    }
    // The reset box, laid out and drawn as the delete box (the same
    // style, attributes and size; its own title).
    if let Some(reset) = &saves.reset {
        let (mut form, _) = gui_paint::paint_top_form(
            saves.reset_px(),
            unit,
            &palette,
            1.0,
            gui_paint::FormWords {
                style: exa_ui::dialogs::DIALOG_STYLE,
                attributes: gui_paint::FORM_ATTRIBUTES | exa_ui::dialogs::DIALOG_ATTRIBUTES,
            },
            Some((RESET_TO_PORTAL.title, &font, size)),
        );
        painted.layers.extend(form.under());
        let client = saves.reset_client();
        for c in reset {
            creator_screen::paint_record(
                c,
                (client.left, client.top),
                unit,
                &palette,
                &text,
                false,
                textures,
                &mut painted,
            );
        }
        painted.layers.extend(form.over());
    }
    painted
}

fn saves_paint(
    screen: Res<CreatorScreen>,
    textures: Option<Res<GuiTextures>>,
    mut frames: ResMut<GuiFrames>,
) {
    let Some(saves) = &screen.saves else {
        return;
    };
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    paint_saves(saves, textures).submit(&mut frames);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;
    use crate::slots::Slots;
    use exa_formats::save_package::SlotKind;

    fn folder(name: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("exarust-save-screen-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// A story slot whose save holds the named player.
    fn story_slot(list: &mut SlotList, name: &str) -> u32 {
        let mut record = CharacterRecord::reset();
        record.name = name.to_string();
        let mut session = Session::story();
        let player = crate::save_live::StoryPlayer {
            record,
            position_cm: [100.0, 0.0, 200.0],
            heading: 0.5,
        };
        let export = crate::save_live::story_player_world(
            &player,
            &mut session,
            &exa_formats::objdb::ObjectDb::default(),
        )
        .unwrap();
        crate::save_live::save_new_slot(list, &session, Some(name), &export.save_world(None, false))
            .unwrap()
            .unwrap()
    }

    fn centre(s: &SaveScreen, i: usize) -> (i32, i32) {
        let c = &s.main[i];
        (c.x + c.w / 2, c.y + c.h / 2)
    }

    fn click(s: &mut SaveScreen, list: &mut SlotList, at: (i32, i32)) -> SaveAction {
        s.pointer(at.0, at.1);
        s.press(list, at.0, at.1);
        s.release(list, at.0, at.1)
    }

    #[test]
    fn an_empty_folder_opens_nothing() {
        let dir = folder("empty");
        let mut list = SlotList::new(Slots::story(&dir));
        assert!(SaveScreen::open(&mut list, STORY, (1024, 768), 4, HashMap::new()).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_screen_shows_the_first_slot_and_steps_through_the_list() {
        let dir = folder("steps");
        let mut list = SlotList::new(Slots::story(&dir));
        story_slot(&mut list, "Ash");
        story_slot(&mut list, "Bran");
        // A fresh manager, as the game makes at start-up.
        let mut list = SlotList::new(Slots::story(&dir));
        let mut s = SaveScreen::open(&mut list, STORY, (1024, 768), 4, HashMap::new()).unwrap();
        // The newest slot is first in the order.
        assert_eq!(s.names, vec![b"Bran".to_vec(), b"Ash".to_vec()]);
        assert_eq!(s.record.name, "Bran");
        assert_eq!(s.main[saves::SLOTS].text, b"Bran");
        // Only the save is there: availability 1, kind 1.
        assert_eq!((s.index, s.kind, s.availability), (0, 1, 1));
        assert_eq!(s.main[saves::PREVIOUS].state, gui_controls::state::DISABLED);
        assert_eq!(s.main[saves::NEXT].state, 0);
        // The toggle offers the checkpoint and cannot switch (kind 1 is not
        // below 1).
        assert_eq!(s.main[saves::CHECKPOINT].text, b"Continue from checkpoint");
        assert_ne!(
            s.main[saves::CHECKPOINT].state & gui_controls::state::DISABLED,
            0
        );
        {
            let at = centre(&s, saves::NEXT);
            click(&mut s, &mut list, at)
        };
        assert_eq!(s.index, 1);
        assert_eq!(s.record.name, "Ash");
        // Only the save is there, so the kind stays on it.
        assert_eq!(s.kind, 1);
        assert_eq!(s.main[saves::CHECKPOINT].text, b"Continue from checkpoint");
        assert_eq!(s.main[saves::NEXT].state, gui_controls::state::DISABLED);
        {
            let at = centre(&s, saves::PREVIOUS);
            click(&mut s, &mut list, at)
        };
        assert_eq!(s.record.name, "Bran");
        // The combo box's list picks the second slot.
        {
            let at = centre(&s, saves::SLOTS);
            click(&mut s, &mut list, at)
        };
        assert_eq!(s.list, Some(DropList::Slots));
        let (rect, count) = s.list_px(s.row_height()).unwrap();
        assert_eq!(count, 2);
        let row = (rect.left + 4, rect.top + 2 + s.row_height() + 2);
        s.press(&mut list, row.0, row.1);
        assert_eq!(s.record.name, "Ash");
        assert_eq!(s.list, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn continue_names_the_current_slot_and_its_file() {
        let dir = folder("continue");
        let mut list = SlotList::new(Slots::story(&dir));
        let number = story_slot(&mut list, "Ash");
        list.slots().copy_save_over_checkpoint(number).unwrap();
        let mut list = SlotList::new(Slots::story(&dir));
        let mut s = SaveScreen::open(&mut list, STORY, (1024, 768), 4, HashMap::new()).unwrap();
        assert_eq!((s.kind, s.availability), (1, 3));
        // Kind 1: the save first.
        assert_eq!(
            {
                let at = centre(&s, saves::CONTINUE);
                click(&mut s, &mut list, at)
            },
            SaveAction::Continue {
                number,
                prefer_checkpoint: false
            }
        );
        // The toggle switches to the checkpoint.
        {
            let at = centre(&s, saves::CHECKPOINT);
            click(&mut s, &mut list, at)
        };
        assert_eq!(s.kind, 2);
        assert_eq!(s.opened, Some(SlotKind::Checkpoint));
        assert_eq!(
            s.continue_action(&list),
            SaveAction::Continue {
                number,
                prefer_checkpoint: true
            }
        );
        assert_eq!(
            {
                let at = centre(&s, saves::BACK);
                click(&mut s, &mut list, at)
            },
            SaveAction::Leave
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_asks_then_removes_the_slot_and_leaves_when_none_is_left() {
        let dir = folder("delete");
        let mut list = SlotList::new(Slots::story(&dir));
        let first = story_slot(&mut list, "Ash");
        story_slot(&mut list, "Bran");
        let mut list = SlotList::new(Slots::story(&dir));
        let mut s = SaveScreen::open(&mut list, STORY, (1024, 768), 4, HashMap::new()).unwrap();
        {
            let at = centre(&s, saves::DELETE);
            click(&mut s, &mut list, at)
        };
        assert_eq!(
            s.confirm.as_ref().unwrap()[delete_box::QUESTION].text,
            b"Are you sure you want to delete Bran?"
        );
        assert_eq!(s.confirm_delete(&mut list), SaveAction::None);
        assert_eq!(list.numbers(), &[first]);
        assert_eq!(s.record.name, "Ash");
        s.ask_delete();
        assert_eq!(s.confirm_delete(&mut list), SaveAction::Leave);
        assert!(!Slots::story(&dir).file(first, SlotKind::Save).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_arena_hides_the_toggle() {
        let s = SaveScreen::new(ARENA, (1024, 768), 4);
        assert!(hidden(&s.main[saves::CHECKPOINT]));
        assert_eq!(s.main[saves::CONTINUE].x, 1024 - 0x20 - 160);
    }

    /// Makes a slot's file a save from before 0.9.5's writers that went
    /// past the portal: the third flags byte cleared and an `exanima06`
    /// section added.
    fn make_legacy_past_portal(path: &std::path::Path) {
        use exa_formats::save_package::{
            self, Flavour, PLAYER_DATA, PackageImage, PlayerData, PlayerHeader,
        };
        let mut package = PackageImage::read(&std::fs::read(path).unwrap()).unwrap();
        let mut data =
            PlayerData::parse(package.section(PLAYER_DATA).unwrap(), Flavour::Story).unwrap();
        if let PlayerHeader::Story(header) = &mut data.header {
            header.flags &= !0xff_0000;
        }
        package.put_section(PLAYER_DATA, &data.write());
        package.put_section(&save_package::name("exanima06"), &[0; 4]);
        std::fs::write(path, package.to_bytes()).unwrap();
    }

    /// A pre-0.9.5 story save past the portal raises the reset box when
    /// the screen opens; No closes it and keeps the files; Yes deletes the
    /// file the select read and, with no checkpoint left, drops the slot.
    #[test]
    fn a_legacy_save_past_the_portal_asks_to_reset() {
        let dir = folder("portal-reset");
        let mut list = SlotList::new(Slots::story(&dir));
        let number = story_slot(&mut list, "Ash");
        let save = list.slots().file(number, SlotKind::Save);
        // A save of this version does not ask.
        let s = SaveScreen::open(&mut list, STORY, (1024, 768), 4, HashMap::new()).unwrap();
        assert!(s.reset.is_none());
        make_legacy_past_portal(&save);
        let mut list = SlotList::new(Slots::story(&dir));
        let mut s = SaveScreen::open(&mut list, STORY, (1024, 768), 4, HashMap::new()).unwrap();
        let reset = s.reset.as_ref().unwrap();
        assert_eq!(
            reset[reset_box::QUESTION].text,
            exa_ui::dialogs::RESET_QUESTION.as_bytes()
        );
        assert_eq!(s.opened, Some(SlotKind::Save));
        // The arena never asks.
        let loaded = list.load_current(false).unwrap();
        assert!(!offers_portal_reset(ARENA, &loaded));
        // No closes the box.
        assert_eq!(
            s.answer(&mut list, Target::Reset(reset_box::NO)),
            SaveAction::None
        );
        assert!(s.reset.is_none());
        assert!(save.is_file());
        // The cancel key closes a box that is up before it leaves.
        s.ask_portal_reset();
        assert_eq!(s.cancel(), SaveAction::None);
        assert!(s.reset.is_none());
        // Yes, by a click on its button.
        s.ask_portal_reset();
        let client = s.reset_client();
        let yes = &s.reset.as_ref().unwrap()[reset_box::YES];
        let at = (
            client.left + yes.x + yes.w / 2,
            client.top + yes.y + yes.h / 2,
        );
        assert_eq!(s.hit(at.0, at.1), Some(Target::Reset(reset_box::YES)));
        assert_eq!(click(&mut s, &mut list, at), SaveAction::None);
        assert!(s.reset.is_none());
        assert!(!save.exists());
        assert_eq!((list.count(), list.current()), (0, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// With a checkpoint left the slot stays listed and current after the
    /// reset deletes the save.
    #[test]
    fn the_reset_keeps_a_slot_with_a_checkpoint() {
        let dir = folder("portal-reset-checkpoint");
        let mut list = SlotList::new(Slots::story(&dir));
        let number = story_slot(&mut list, "Ash");
        list.slots().copy_save_over_checkpoint(number).unwrap();
        list.select(0);
        assert!(!list.reset_to_portal(SlotKind::Save).unwrap());
        assert!(!list.slots().file(number, SlotKind::Save).exists());
        assert_eq!((list.count(), list.current()), (1, number as i32));
        // No current slot: nothing happens.
        let mut idle = SlotList::new(Slots::story(&dir));
        assert!(!idle.reset_to_portal(SlotKind::Checkpoint).unwrap());
        assert!(list.slots().file(number, SlotKind::Checkpoint).is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A folder whose slot 1 has a save holding "Ash" and a checkpoint
    /// holding "Bran" (slot 2's save renamed), so a read names its file.
    fn slot_with_both(name: &str) -> (std::path::PathBuf, u32) {
        let dir = folder(name);
        let mut list = SlotList::new(Slots::story(&dir));
        let number = story_slot(&mut list, "Ash");
        let other = story_slot(&mut list, "Bran");
        let slots = Slots::story(&dir);
        std::fs::rename(
            slots.file(other, SlotKind::Save),
            slots.file(number, SlotKind::Checkpoint),
        )
        .unwrap();
        assert!(dir.join(format!("Exanima{number:03}.rsg")).is_file());
        assert!(dir.join(format!("Exanima{number:03}.rcp")).is_file());
        (dir, number)
    }

    fn shown(found: &crate::slots::ScreenSelect) -> (u32, Option<SlotKind>, String) {
        let loaded = found.loaded.as_ref();
        (
            found.availability,
            loaded.map(|l| l.kind),
            loaded
                .and_then(|l| l.player_record())
                .map(|r| r.name)
                .unwrap_or_default(),
        )
    }

    /// The select's kinds and availability bits are the name builder's:
    /// kind 1 and bit 1 the `.rsg`, kind 2 and bit 2 the `.rcp`.
    #[test]
    fn the_select_reads_the_save_for_kind_1_and_the_checkpoint_for_kind_2() {
        let (dir, number) = slot_with_both("kinds");
        let mut list = SlotList::new(Slots::story(&dir));
        assert!(list.any());
        assert_eq!(list.numbers(), &[number]);
        let save = shown(&list.character_screen_select(0, 1));
        assert_eq!(save, (3, Some(SlotKind::Save), "Ash".to_string()));
        let checkpoint = shown(&list.character_screen_select(0, 2));
        assert_eq!(
            checkpoint,
            (3, Some(SlotKind::Checkpoint), "Bran".to_string())
        );
        // Only the checkpoint: availability 2, read whatever the kind.
        std::fs::remove_file(dir.join(format!("Exanima{number:03}.rsg"))).unwrap();
        let alone = shown(&list.character_screen_select(0, 1));
        assert_eq!(alone, (2, Some(SlotKind::Checkpoint), "Bran".to_string()));
        // The screen opens on kind 2 then.
        let mut fresh = SlotList::new(Slots::story(&dir));
        let s = SaveScreen::open(&mut fresh, STORY, (1024, 768), 4, HashMap::new()).unwrap();
        assert_eq!((s.kind, s.availability), (2, 2));
        assert_eq!(s.record.name, "Bran");
        let _ = std::fs::remove_dir_all(&dir);
        // Only the save: availability 1, read whatever the kind.
        let dir = folder("kinds-save");
        let mut list = SlotList::new(Slots::story(&dir));
        story_slot(&mut list, "Ash");
        let mut list = SlotList::new(Slots::story(&dir));
        assert!(list.any());
        let alone = shown(&list.character_screen_select(0, 2));
        assert_eq!(alone, (1, Some(SlotKind::Save), "Ash".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// CONTINUE on a fresh screen loads the save; after the toggle it
    /// loads the checkpoint (which then is copied over the save).
    #[test]
    fn continue_loads_the_save_by_default_and_the_checkpoint_when_toggled() {
        let (dir, number) = slot_with_both("continue-kinds");
        let mut list = SlotList::new(Slots::story(&dir));
        let mut s = SaveScreen::open(&mut list, STORY, (1024, 768), 4, HashMap::new()).unwrap();
        assert_eq!(s.record.name, "Ash");
        let SaveAction::Continue {
            prefer_checkpoint, ..
        } = s.continue_action(&list)
        else {
            panic!("no continue");
        };
        let loaded = list.load_current(prefer_checkpoint).unwrap();
        assert_eq!(loaded.kind, SlotKind::Save);
        assert_eq!(loaded.player_record().unwrap().name, "Ash");
        s.toggle(&mut list);
        assert_eq!(s.record.name, "Bran");
        let SaveAction::Continue {
            prefer_checkpoint, ..
        } = s.continue_action(&list)
        else {
            panic!("no continue");
        };
        let loaded = list.load_current(prefer_checkpoint).unwrap();
        assert_eq!(loaded.kind, SlotKind::Checkpoint);
        assert_eq!(loaded.player_record().unwrap().name, "Bran");
        // The checkpoint read is copied over the save.
        let rsg = dir.join(format!("Exanima{number:03}.rsg"));
        let rcp = dir.join(format!("Exanima{number:03}.rcp"));
        assert_eq!(std::fs::read(&rsg).unwrap(), std::fs::read(&rcp).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The reset to the portal deletes the file the select read: the save
    /// on a fresh screen (the slot stays for its checkpoint); the
    /// checkpoint after the toggle (the slot leaves the list though its
    /// save is still on disk, as in the original).
    #[test]
    fn the_portal_reset_deletes_the_file_the_select_read() {
        let (dir, number) = slot_with_both("reset-kinds");
        let rsg = dir.join(format!("Exanima{number:03}.rsg"));
        let rcp = dir.join(format!("Exanima{number:03}.rcp"));
        let mut list = SlotList::new(Slots::story(&dir));
        let mut s = SaveScreen::open(&mut list, STORY, (1024, 768), 4, HashMap::new()).unwrap();
        assert_eq!(s.opened, Some(SlotKind::Save));
        s.confirm_portal_reset(&mut list);
        assert!(!rsg.exists());
        assert!(rcp.is_file());
        assert_eq!((list.count(), list.current()), (1, number as i32));
        // The select again finds the checkpoint alone.
        assert_eq!((s.kind, s.availability), (2, 2));
        assert_eq!(s.record.name, "Bran");
        let _ = std::fs::remove_dir_all(&dir);

        let (dir, number) = slot_with_both("reset-kinds-checkpoint");
        let rsg = dir.join(format!("Exanima{number:03}.rsg"));
        let rcp = dir.join(format!("Exanima{number:03}.rcp"));
        let mut list = SlotList::new(Slots::story(&dir));
        let mut s = SaveScreen::open(&mut list, STORY, (1024, 768), 4, HashMap::new()).unwrap();
        s.toggle(&mut list);
        assert_eq!(s.opened, Some(SlotKind::Checkpoint));
        assert!(list.reset_to_portal(SlotKind::Checkpoint).unwrap());
        assert!(!rcp.exists());
        assert!(rsg.is_file());
        assert_eq!((list.count(), list.current()), (0, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The delete removes both of the slot's files.
    #[test]
    fn delete_removes_the_save_and_the_checkpoint() {
        let (dir, number) = slot_with_both("delete-kinds");
        let mut list = SlotList::new(Slots::story(&dir));
        let mut s = SaveScreen::open(&mut list, STORY, (1024, 768), 4, HashMap::new()).unwrap();
        s.ask_delete();
        assert_eq!(s.confirm_delete(&mut list), SaveAction::Leave);
        assert!(!dir.join(format!("Exanima{number:03}.rsg")).exists());
        assert!(!dir.join(format!("Exanima{number:03}.rcp")).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
