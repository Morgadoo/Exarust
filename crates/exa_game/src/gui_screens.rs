//! The running game's in-game screens: the inventory and journal the
//! player's keys open, kept as forms on a [`GuiTask`] so their focus follows
//! the window manager. The show rules, tags, places and the journal's
//! entries are [`exa_ui::game_gui`]'s; the keys come through the player's
//! hotkey handler ([`crate::hotkeys`]), which applies its own permissions
//! and sends a [`ScreenRequest`].
//!
//! The inventory and journal are painted through the GUI draw path
//! ([`crate::gui_paint`]): their frames, the item pack's panel and the
//! control records the constructors append (labels and buttons, placed by
//! the control layout pass). The character view, the item icons, the
//! journal's entry list and the controls' callbacks are not wired. Escape
//! closing the focused screen is Exarust's (the original closes them
//! through their frame marks and the keys again). The skills key's screen
//! and the power tree are outside this unit; the technique selector opens
//! from a power slot, which the HUD does not draw yet.
//!
//! Rules: `docs/UI.md` (The in-game GUI unit).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ui::form::{ClientRect, UNPLACED, place_unset};
use exa_ui::game_gui::{
    CONTAINER_FAMILY, GameScreen, INVENTORY_PLACE, INVENTORY_TITLE, InventoryInput, JOURNAL_TEXT,
    JOURNAL_TITLE, Journal, JournalInput, LOOT_PLACE, LOOT_TITLE, OpenContainers, ScreenAction,
    forms_with_tag, inventory_place, inventory_screen, journal_screen, loot_screen,
    tagged_container,
};
use exa_ui::gui_controls::{Control, LayoutClient, layout_records};
use exa_ui::gui_core::{FormHost, FormId, GuiRect, GuiTask, NO_CONTROL};
use exa_ui::gui_draw::{GuiFrames, GuiTextures};
use exa_ui::skin::SkinBatch;

use crate::body_inventory;
use crate::gui_paint::{self, Frame};
use crate::item_container::{
    CardWindow, Container, ContainerChild, ContainerRecord, POPUP_BORDER, POPUP_PANEL_BORDER,
    PopupPlan, open_container, popup_form_size, popup_icon_height, popup_icon_place, popup_place,
};
use crate::item_panel_draw::PanelIcons;
use crate::menu_screen::MenuScreen;

/// A screen the player's keys ask for.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRequest {
    pub screen: GameScreen,
    /// The player is not out of action (not collapsing).
    pub player_ready: bool,
    /// The player's condition is not the dead mode 0xa00.
    pub player_alive: bool,
    /// The body a loot window is asked for (its entity index), if any.
    pub target: Option<i32>,
    /// The body's record flag word, for the loot window's show rule.
    pub flags: u32,
}

impl ScreenRequest {
    /// A request for a screen that is not a body's loot window.
    pub fn screen(screen: GameScreen, player_ready: bool, player_alive: bool) -> Self {
        Self {
            screen,
            player_ready,
            player_alive,
            target: None,
            flags: 0,
        }
    }
}

/// One open screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenScreen {
    pub kind: GameScreen,
    pub form: FormId,
}

/// An open loot window: the body it shows, its pack and what that body
/// wears and holds. The panels are the item container's (the pack's
/// 228 × 180 panel at the client's bottom, the equipment panel above it).
#[derive(Debug, Clone, PartialEq)]
pub struct LootWindow {
    pub form: FormId,
    /// The body's record id (its entity index in Exarust).
    pub character: i32,
    /// The body's record flag word, as it was when the window opened.
    pub flags: u32,
    pub pack: Container,
    /// What the body wears and holds.
    pub worn: Container,
}

impl LootWindow {
    /// An empty window's panels for a GUI unit.
    fn new(form: FormId, character: i32, flags: u32, unit: i32) -> Self {
        let mut pack = Container::form(unit);
        pack.owner = Some(character as u32);
        let mut worn = Container::form(unit);
        worn.owner = Some(character as u32);
        Self {
            form,
            character,
            flags,
            pack,
            worn,
        }
    }
}

/// A request to open a placed container's popup: the double click's
/// interaction on an object whose record has a container section.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerRequest {
    /// The placed object carrying the container section
    /// ([`exa_world::ObjectSections`]).
    pub object: Entity,
    /// The placed object's database id, tier tag included.
    pub id: u32,
    /// The pointer when the request was made, physical pixels.
    pub pointer: IVec2,
}

/// An open container popup: the card frame hosting the container's item
/// panel, bound to the container's record.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerPopup {
    pub form: FormId,
    /// The container's tagged database id (the popup's record id field).
    pub id: u32,
    /// The placed object whose record the panel saves back into.
    pub object: Entity,
    /// The record's header fields, which the save keeps.
    pub stored: ContainerRecord,
    pub panel: Container,
    /// The panel's size and top inside the popup's client.
    pub plan: PopupPlan,
    /// The container's own icon (the card's icon handle) and the icon
    /// area's height.
    pub icon: usize,
    pub icon_height: i32,
}

impl ContainerPopup {
    /// The popup's client: its bounds inside its border.
    pub fn client(bounds: GuiRect) -> GuiRect {
        GuiRect::new(
            bounds.left + POPUP_BORDER,
            bounds.top + POPUP_BORDER,
            bounds.width() - 2 * POPUP_BORDER,
            bounds.height() - 2 * POPUP_BORDER,
        )
    }

    /// The item panel's rectangle: at the client's left, the plan's top
    /// down, the plan's size (its own border included).
    pub fn panel_rect(&self, bounds: GuiRect) -> GuiRect {
        let client = Self::client(bounds);
        GuiRect::new(
            client.left + exa_ui::popups::CONTAINER_INSET,
            client.top + self.plan.inner_top,
            self.plan.inner.0,
            self.plan.inner.1,
        )
    }
}

/// A container record as the panel loads it, from the object's section.
pub fn container_record(section: &exa_formats::containers::Container) -> ContainerRecord {
    ContainerRecord {
        size: (section.width, section.height),
        capacity: section.capacity,
        max_item: section.max_item,
        children: section
            .children
            .iter()
            .map(|child| ContainerChild {
                id: child.id,
                flags: child.flags,
                x: child.x,
                y: child.y,
            })
            .collect(),
    }
}

/// Writes a saved record's children back into the object's section, as the
/// close's save rewrites the record's container section.
pub fn store_record(section: &mut exa_formats::containers::Container, record: &ContainerRecord) {
    section.children = record
        .children
        .iter()
        .map(|child| exa_formats::containers::Child {
            id: child.id,
            flags: child.flags,
            x: child.x,
            y: child.y,
        })
        .collect();
}

/// The class of the inventory's second record, whose draw and behaviour are
/// not located; it is kept for its place in the record order.
pub const UNLOCATED_CLASS: u16 = 0xac01;

/// The unit the layout pass is given (it reads it only for a first record's
/// unit anchor and for trackbars, which these screens have none of).
const LAYOUT_UNIT: i32 = 4;

/// The record's y for the full-height inventory: the constructor writes
/// one of three values by the form's height variant (0x54 in the arsenal,
/// 0x5e under the compact flag, 0x62 otherwise).
const INVENTORY_LABEL_ROW: i32 = 0x62;

/// A button record: class 1, the style word the journal and inventory
/// constructors share, and its caption.
fn button_record(x: i32, y: i32, w: i32, text: &str) -> Control {
    let mut control = Control::new(1, x, y, w, 0);
    control.style = 0x40_3320;
    control.text = text.as_bytes().to_vec();
    control
}

/// The inventory constructor's three records, as stored: the Encumbrance
/// label, the unlocated control beside it and the Primary button.
fn inventory_records() -> Vec<Control> {
    let mut label = Control::new(5, 4, INVENTORY_LABEL_ROW, 12, 28);
    label.style = 0;
    label.text = b"Encumbrance:".to_vec();
    let unlocated = Control::new(UNLOCATED_CLASS, 100, -0x8000, 12, 28);
    // The button's x is stored as a plain 4 and its y as an anchor of eight
    // rows below the record above.
    let primary = button_record(4, -0x7ff8, 108, "Primary");
    vec![label, unlocated, primary]
}

/// The journal constructor's two records, as stored: New, and Erase placed
/// after it by an anchor. The text box is a child form, not a record.
fn journal_records() -> Vec<Control> {
    vec![
        button_record(4, 6, 104, "New"),
        button_record(-0x7ff9, -0x8000, 104, "Erase"),
    ]
}

/// The control records a screen's form carries, laid out by the control
/// layout pass inside the form's client (inset by `spacing`) and moved to
/// window coordinates. None of these records is a trackbar or the first
/// record's unit anchor, so the pass does not depend on the GUI unit.
pub fn screen_controls(kind: GameScreen, bounds: GuiRect) -> Vec<Control> {
    let (mut records, spacing) = match kind {
        GameScreen::Inventory => (inventory_records(), gui_paint::INVENTORY_SPACING),
        GameScreen::Journal => (journal_records(), gui_paint::FORM_SPACING),
        _ => return Vec::new(),
    };
    let client = GuiRect::new(
        bounds.left + spacing,
        bounds.top + spacing,
        bounds.width() - spacing * 2,
        bounds.height() - spacing * 2,
    );
    layout_records(
        &mut records,
        LayoutClient {
            inner_right: client.width(),
            border: gui_paint::BORDER,
            bottom: client.height(),
        },
        LAYOUT_UNIT,
    );
    for record in &mut records {
        record.x += client.left;
        record.y += client.top;
    }
    records
}

/// The `--debug-gui-form` flag: opens one empty form through the skin's
/// opening so the draw path can be checked by eye.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct DebugGuiForm {
    pub enabled: bool,
}

/// A screen's form: its rectangle and the controls the window manager
/// hits, selects, presses and drags. The close marks it doomed; the task
/// deletes it.
pub struct ScreenForm {
    pub parent: Option<FormId>,
    pub bounds: GuiRect,
    pub doomed: bool,
    pub selected: i32,
    /// Which screen this form is, for the painter.
    pub kind: GameScreen,
    /// The form's controls, in record order (1-based in messages).
    pub controls: Vec<Control>,
}

impl ScreenForm {
    /// The 1-based control under a form-local point.
    pub fn control_at(&self, x: i32, y: i32) -> i32 {
        for (index, control) in self.controls.iter().enumerate() {
            if GuiRect::new(control.x, control.y, control.w, control.h).contains(x, y) {
                return index as i32 + 1;
            }
        }
        0
    }
}

impl FormHost for ScreenForm {
    fn parent(&self) -> Option<FormId> {
        self.parent
    }
    fn bounds(&self) -> GuiRect {
        self.bounds
    }
    fn is_visible(&self) -> bool {
        true
    }
    fn hit_test(&self, x: i32, y: i32) -> i32 {
        self.control_at(x, y)
    }
    fn select(&mut self, control: i32) -> u32 {
        if self.selected == control {
            return 0;
        }
        self.selected = control;
        3
    }
    fn selected_control(&self) -> i32 {
        self.selected
    }
    fn close(&mut self, _arg: i32) {
        self.doomed = true;
    }
    fn doomed(&self) -> bool {
        self.doomed
    }
}

/// The open screens, the window manager that focuses them and the session
/// data behind them.
#[derive(Resource)]
pub struct GameScreens {
    pub task: GuiTask,
    pub open: Vec<OpenScreen>,
    pub journal: Journal,
    pub containers: OpenContainers,
    /// The journal's text box, as it would hold the selected entry.
    pub journal_text: String,
    /// The player's inventory place, kept from its last close (the
    /// original keeps one; its writer is outside this unit).
    pub inventory_place: Option<(i32, i32)>,
    /// The player's character record id (Exarust uses the entity index).
    pub player: i32,
    /// The inventory's item pack, drawn through the panel's icon path.
    pub pack: Container,
    /// The description card the panel asked for.
    pub card: CardWindow,
    /// The open loot windows, one per body.
    pub loot: Vec<LootWindow>,
    /// The open container popups.
    pub popups: Vec<ContainerPopup>,
    /// Records the closed popups saved, waiting to be written back into
    /// their objects' container sections.
    pub saved_containers: Vec<(Entity, ContainerRecord)>,
    /// Icon handles freed by a closed panel, waiting for the icon manager
    /// to drop their references (the destroy's sweep).
    pub released_icons: Vec<usize>,
}

impl Default for GameScreens {
    fn default() -> Self {
        Self {
            task: GuiTask::new((800, 600)),
            open: Vec::new(),
            journal: Journal::default(),
            containers: OpenContainers::default(),
            journal_text: String::new(),
            inventory_place: None,
            player: 0,
            pack: Container::default(),
            card: CardWindow::default(),
            loot: Vec::new(),
            popups: Vec::new(),
            saved_containers: Vec::new(),
            released_icons: Vec::new(),
        }
    }
}

impl GameScreens {
    pub fn find(&self, kind: GameScreen) -> Option<OpenScreen> {
        self.open.iter().copied().find(|screen| screen.kind == kind)
    }

    pub fn is_open(&self, kind: GameScreen) -> bool {
        self.find(kind).is_some()
    }

    /// The screen the window manager focuses.
    pub fn focused(&self) -> Option<GameScreen> {
        let active = self.task.active?;
        self.open
            .iter()
            .find(|screen| screen.form == active)
            .map(|screen| screen.kind)
    }

    fn root_client(&self) -> ClientRect {
        ClientRect {
            left: 0,
            top: 0,
            right: self.task.screen.0,
            bottom: self.task.screen.1,
        }
    }

    fn push(&mut self, kind: GameScreen, bounds: GuiRect) -> FormId {
        let root = self.task.root;
        let controls = screen_controls(kind, bounds);
        let form = self.task.add(Box::new(ScreenForm {
            parent: root,
            bounds,
            doomed: false,
            selected: 0,
            kind,
            controls,
        }));
        self.open.push(OpenScreen { kind, form });
        self.task.focus_set(Some(form), NO_CONTROL);
        form
    }

    /// The player's inventory: the show rule (toggling), then a new one is
    /// registered as an open container and placed at its constructor's
    /// (8, 8) or its remembered place.
    pub fn open_inventory(
        &mut self,
        player_ready: bool,
        player_alive: bool,
        unit: i32,
    ) -> ScreenAction {
        let open = self.find(GameScreen::Inventory);
        let action = inventory_screen(InventoryInput {
            player_ready,
            player_mode_ok: player_alive,
            open: open.is_some(),
            filling: false,
            active: self.focused() == Some(GameScreen::Inventory),
            toggle: true,
        });
        match (action, open) {
            (ScreenAction::Opened, _) => {
                let (width, height) = GameScreen::Inventory.size().unwrap_or((244, 644));
                let (x, y) = inventory_place(
                    true,
                    self.inventory_place,
                    self.task.screen.0,
                    width,
                    unit,
                    INVENTORY_PLACE,
                );
                self.containers.open(tagged_container(self.player));
                self.push(GameScreen::Inventory, GuiRect::new(x, y, width, height));
            }
            (ScreenAction::Focused, Some(screen)) => {
                self.task.focus_set(Some(screen.form), NO_CONTROL);
            }
            (ScreenAction::Closed, _) => {
                self.close(GameScreen::Inventory);
            }
            _ => {}
        }
        action
    }

    /// The journal: the show rule, then a new one at its remembered place
    /// or the unset place in the screen (4, 36), on its first entry.
    pub fn open_journal(&mut self, player_ready: bool, player_alive: bool) -> ScreenAction {
        let open = self.find(GameScreen::Journal);
        let action = journal_screen(JournalInput {
            player_ready,
            player_mode_ok: player_alive,
            open: open.is_some(),
            active: self.focused() == Some(GameScreen::Journal),
        });
        match (action, open) {
            (ScreenAction::Opened, _) => {
                let (width, height) = GameScreen::Journal.size().unwrap_or((244, 518));
                let (x, y) = self.journal.place.unwrap_or_else(|| {
                    place_unset(
                        UNPLACED,
                        UNPLACED,
                        width,
                        height,
                        0x3000,
                        self.root_client(),
                    )
                });
                self.journal.ensure_entry();
                self.journal.select(0);
                self.journal_text = self.journal.entries[0].text.clone();
                self.push(GameScreen::Journal, GuiRect::new(x, y, width, height));
            }
            (ScreenAction::Focused, Some(screen)) => {
                self.task.focus_set(Some(screen.form), NO_CONTROL);
            }
            (ScreenAction::Closed, _) => {
                self.close(GameScreen::Journal);
            }
            _ => {}
        }
        action
    }

    /// The loot window for a body: refused for a body whose record flag
    /// word has its second byte 3; an existing window on that body is
    /// activated, else a new one opens at x 400 with an unset y, enters the
    /// open-container registry under the body's tagged id, and takes an
    /// empty pack and equipment panel until the body's items are bound.
    /// Another body's window is not reused, as the original matches the
    /// window's character as well as its tag.
    pub fn open_loot(&mut self, character: i32, flags: u32, unit: i32) -> ScreenAction {
        let existing = self
            .loot
            .iter()
            .find(|window| window.character == character)
            .map(|window| window.form);
        let action = loot_screen(flags, existing.is_some());
        match action {
            ScreenAction::Focused => {
                if let Some(form) = existing {
                    self.task.focus_set(Some(form), NO_CONTROL);
                }
            }
            ScreenAction::Opened => {
                let (width, height) = GameScreen::Loot.size().unwrap_or((244, 440));
                let (x, y) = place_unset(
                    LOOT_PLACE.0,
                    LOOT_PLACE.1,
                    width,
                    height,
                    gui_paint::SCREEN_STYLE,
                    self.root_client(),
                );
                self.containers.open(tagged_container(character));
                let form = self.push(GameScreen::Loot, GuiRect::new(x, y, width, height));
                self.loot
                    .push(LootWindow::new(form, character, flags, unit));
            }
            _ => {}
        }
        action
    }

    /// The open popup of a container, if any.
    pub fn popup(&self, id: u32) -> Option<&ContainerPopup> {
        self.popups.iter().find(|popup| popup.id == id)
    }

    /// Opens a container's popup, as the original's opener does: a popup
    /// already open on the same container is brought forward (`Focused`);
    /// otherwise the popup is registered as an open container, its panel
    /// sized from the record ([`open_container`] with the panel's border)
    /// six units plus the icon area below its top, the form sized around
    /// it ([`popup_form_size`]) and placed on the pointer
    /// ([`popup_place`]). The panel then loads the record's children
    /// (`resolve` gives each child's icon and bulk, as
    /// [`Container::load_record`] takes it). `icon` is the container's own
    /// icon handle and `icon_half` its half height.
    #[allow(clippy::too_many_arguments)]
    pub fn open_container_popup<F>(
        &mut self,
        object: Entity,
        id: u32,
        record: &ContainerRecord,
        pointer: (i32, i32),
        unit: i32,
        icon: usize,
        icon_half: i32,
        resolve: F,
    ) -> ScreenAction
    where
        F: FnMut(u32, u32) -> Option<(usize, i32)>,
    {
        if id == 0 {
            return ScreenAction::Refused;
        }
        if let Some(form) = self.popup(id).map(|popup| popup.form) {
            self.task.focus_set(Some(form), NO_CONTROL);
            return ScreenAction::Focused;
        }
        let mut panel = Container::form(unit);
        let icon_height = popup_icon_height(icon_half, panel.icon_size);
        let Some(plan) = open_container(
            false,
            Some(record.size),
            unit,
            POPUP_PANEL_BORDER,
            icon_height,
        ) else {
            return ScreenAction::Refused;
        };
        self.containers.open(id as i32);
        let size = popup_form_size(plan, unit, POPUP_BORDER);
        let (x, y) = popup_place(pointer, size, self.task.screen.0, unit);
        let form = self.push(GameScreen::Container, GuiRect::new(x, y, size.0, size.1));
        panel.client = (
            plan.inner.0 - 2 * POPUP_PANEL_BORDER,
            plan.inner.1 - 2 * POPUP_PANEL_BORDER,
        );
        panel.load_record(id, record, resolve);
        self.popups.push(ContainerPopup {
            form,
            id,
            object,
            stored: ContainerRecord {
                children: Vec::new(),
                ..record.clone()
            },
            panel,
            plan,
            icon,
            icon_height,
        });
        ScreenAction::Opened
    }

    /// Closes the popup of a container, whichever window is focused.
    pub fn close_popup(&mut self, id: u32) -> bool {
        let Some(form) = self.popup(id).map(|popup| popup.form) else {
            return false;
        };
        let Some(index) = self.open.iter().position(|screen| screen.form == form) else {
            return false;
        };
        self.close_at(index)
    }

    /// The open loot window on a body, if any.
    pub fn loot_window(&self, character: i32) -> Option<&LootWindow> {
        self.loot
            .iter()
            .find(|window| window.character == character)
    }

    /// Binds a body's items to its loot window: the pack's entries and what
    /// it wears and holds. The original loads both panels from the body's
    /// record when the window is created; the running game has no body
    /// inventory yet, so the caller supplies them (an empty body leaves the
    /// panels as they are).
    pub fn bind_loot(&mut self, character: i32, pack: Container, worn: Container) -> bool {
        let Some(window) = self
            .loot
            .iter_mut()
            .find(|window| window.character == character)
        else {
            return false;
        };
        window.pack = pack;
        window.worn = worn;
        true
    }

    /// The bodies whose loot window the deferred close takes, in the open
    /// windows' order (the original walks the GUI's forms): a window whose
    /// body's mark 0x80000 is set or whose condition is no longer dead
    /// (other than 0xa00). `bodies` gives each body's (id, flags, mode); a
    /// window whose body is not listed stays.
    pub fn loot_windows_to_close(&self, bodies: &[(i32, u32, u32)]) -> Vec<i32> {
        self.open
            .iter()
            .filter(|screen| screen.kind == GameScreen::Loot)
            .filter_map(|screen| self.loot.iter().find(|w| w.form == screen.form))
            .filter_map(|window| {
                bodies
                    .iter()
                    .find(|(character, _, _)| *character == window.character)
            })
            .filter(|(_, flags, mode)| exa_ui::game_gui::closes_loot_window(*flags, *mode))
            .map(|(character, _, _)| *character)
            .collect()
    }

    /// Closes a screen: its form closes and the task deletes it; the
    /// journal stores its text (its save) and the screens remember their
    /// place; the inventory leaves the container registry; a loot window
    /// leaves it under its body's id (the original also saves the body's
    /// pack, which has no Exarust store yet).
    /// The focus falls to the last screen left. The focused window of the
    /// kind is closed when there are several (the loot windows of two
    /// bodies).
    pub fn close(&mut self, kind: GameScreen) -> bool {
        let focused = self.task.active;
        let index = self
            .open
            .iter()
            .position(|screen| screen.kind == kind && Some(screen.form) == focused)
            .or_else(|| self.open.iter().position(|screen| screen.kind == kind));
        let Some(index) = index else {
            return false;
        };
        self.close_at(index)
    }

    /// Closes one open screen by its position in the open list.
    fn close_at(&mut self, index: usize) -> bool {
        let screen = self.open.remove(index);
        let place = self.task.form(screen.form).map(|form| {
            let b = form.bounds();
            (b.left, b.top)
        });
        if let Some(form) = self.task.form_mut(screen.form) {
            form.close(0);
        }
        self.task.delete_doomed();
        match screen.kind {
            GameScreen::Journal => {
                self.journal.store_text(&self.journal_text);
                self.journal.place = place;
            }
            GameScreen::Inventory => {
                self.containers.close(tagged_container(self.player));
                self.inventory_place = place;
                // The panel's destroy releases every icon reference before
                // its entry array goes.
                let unit = self.pack.unit;
                let mut released = Vec::new();
                self.pack.release_all_icons(|icon| released.push(icon));
                self.released_icons.extend(released);
                self.pack = Container::form(unit);
            }
            GameScreen::Loot => {
                if let Some(window) = self
                    .loot
                    .iter()
                    .position(|window| window.form == screen.form)
                    .map(|index| self.loot.remove(index))
                {
                    self.containers.close(tagged_container(window.character));
                    let (mut pack, mut worn) = (window.pack, window.worn);
                    let mut released = Vec::new();
                    pack.release_all_icons(|icon| released.push(icon));
                    worn.release_all_icons(|icon| released.push(icon));
                    self.released_icons.extend(released);
                }
            }
            GameScreen::Container => {
                // The container close: the game forgets the open container,
                // the panel saves its entries into the record (releasing
                // their icons), its id is cleared and the card's close
                // releases the container's own icon.
                if let Some(mut popup) = self
                    .popups
                    .iter()
                    .position(|popup| popup.form == screen.form)
                    .map(|index| self.popups.remove(index))
                {
                    let close = exa_ui::popups::container_close(false);
                    if close.notify_game {
                        self.containers.close(popup.id as i32);
                    }
                    if close.save_contents
                        && let Some(saved) = popup.panel.save_record(&popup.stored)
                    {
                        self.saved_containers.push((popup.object, saved));
                    }
                    let mut released = Vec::new();
                    popup.panel.release_all_icons(|icon| released.push(icon));
                    if close.clear_id {
                        popup.panel.container_id = 0;
                    }
                    if close.run_base && exa_ui::popups::desc_close(popup.icon != 0) {
                        released.push(popup.icon);
                    }
                    self.released_icons.extend(released);
                }
            }
            _ => {}
        }
        if let Some(next) = self.open.last() {
            self.task.focus_set(Some(next.form), NO_CONTROL);
        } else {
            self.task.active = None;
            self.task.active_control = NO_CONTROL;
        }
        true
    }

    /// Closes the focused screen, if any (Escape).
    pub fn close_focused(&mut self) -> bool {
        self.focused().is_some_and(|kind| self.close(kind))
    }

    /// Closes the loot window of one body (the deferred close), whichever
    /// window is focused.
    pub fn close_loot(&mut self, character: i32) -> bool {
        let Some(form) = self
            .loot
            .iter()
            .find(|window| window.character == character)
            .map(|window| window.form)
        else {
            return false;
        };
        let Some(index) = self.open.iter().position(|screen| screen.form == form) else {
            return false;
        };
        self.close_at(index)
    }

    /// Closes the container and journal family, front to back.
    pub fn close_container_family(&mut self) -> usize {
        let tags: Vec<u32> = self
            .open
            .iter()
            .map(|screen| screen.kind.tag().unwrap_or(0))
            .collect();
        let kinds: Vec<GameScreen> = forms_with_tag(&tags, CONTAINER_FAMILY, true)
            .into_iter()
            .map(|i| self.open[i].kind)
            .collect();
        for kind in &kinds {
            self.close(*kind);
        }
        kinds.len()
    }
}

pub struct GameScreensPlugin;

impl Plugin for GameScreensPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .init_resource::<DebugGuiForm>()
            .init_resource::<DebugOpenContainer>()
            .init_resource::<GuiFrames>()
            .add_message::<ScreenRequest>()
            .add_message::<ContainerRequest>()
            .add_systems(
                Update,
                (
                    sync_screen_size,
                    register_level_items,
                    open_requested_screens,
                    debug_open_container,
                    open_requested_containers,
                )
                    .chain()
                    .after(crate::hotkeys::HotkeySet)
                    .before(crate::player::player_combat),
            )
            .add_systems(
                Update,
                draw_screens
                    .after(open_requested_screens)
                    .after(crate::item_icon_render::IconRenderSystems),
            )
            .add_systems(
                Update,
                (close_on_right_release, close_changed_loot_windows).after(open_requested_screens),
            )
            .add_systems(
                Update,
                (release_closed_icons, store_saved_containers)
                    .after(close_on_right_release)
                    .after(close_changed_loot_windows),
            );
    }
}

/// Opens the popups the double click asked for: the item card's request is
/// closed first (a pending card is cancelled and a shown one fades out),
/// then the object's container section is loaded into a new popup at the
/// pointer. An object without the section opens nothing.
#[allow(clippy::too_many_arguments)]
fn open_requested_containers(
    mut requests: MessageReader<ContainerRequest>,
    objects: Query<&exa_world::ObjectSections>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    mut icons: Option<ResMut<PanelIcons>>,
    mut card: Option<ResMut<crate::item_card::CardView>>,
    time: Option<Res<Time>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
) {
    for request in requests.read() {
        let Ok(sections) = objects.get(request.object) else {
            continue;
        };
        let Some(section) = sections.container.as_ref() else {
            continue;
        };
        let Some(icons) = icons.as_deref_mut() else {
            continue;
        };
        screens.card.timer.cancel();
        if let Some(shown) = card.as_deref_mut().and_then(|card| card.shown.as_mut()) {
            let now = time
                .as_deref()
                .map_or(0, |t| t.elapsed().as_millis() as u32);
            shown.fade.fade_out(now);
        }
        let unit = windows
            .single()
            .ok()
            .map(|window| exa_ui::scale::gui_scale(window.physical_height(), 0) as i32)
            .unwrap_or(4);
        let record = container_record(section);
        let (icon, (_, icon_half)) = icons.reserve(request.id, 0, catalogue.as_deref());
        let catalogue = catalogue.as_deref();
        let action = screens.open_container_popup(
            request.object,
            request.id,
            &record,
            (request.pointer.x, request.pointer.y),
            unit,
            icon,
            icon_half,
            |id, flags| {
                let (slot, _) = icons.reserve(id, flags, catalogue);
                let bulk = catalogue.and_then(|c| c.bulk(id)).map_or(0, i32::from);
                Some((slot, bulk))
            },
        );
        if action != ScreenAction::Opened && icon != 0 {
            icons.release(icon);
        }
        debug!("GUI: container {:#x} {action:?}", request.id);
    }
}

/// Exarust's debug helper (`--open-container FRAME`): this many frames after
/// a player is first present, the placed container nearest the player is
/// opened in its popup at the window's centre, as the double click would.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugOpenContainer(pub Option<u32>);

/// Runs [`DebugOpenContainer`]: counts frames once a player is present and,
/// at the frame asked for, requests the nearest container's popup.
fn debug_open_container(
    debug: Option<Res<DebugOpenContainer>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    objects: Query<(
        Entity,
        &exa_world::PlacedItem,
        &exa_world::ObjectSections,
        &GlobalTransform,
    )>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: Local<u32>,
    mut requests: MessageWriter<ContainerRequest>,
) {
    let Some(at) = debug.and_then(|debug| debug.0) else {
        return;
    };
    let Some(player) = players.iter().next() else {
        return;
    };
    *frames += 1;
    if *frames != at {
        return;
    }
    let feet = player.translation();
    let nearest = objects
        .iter()
        .filter(|(_, _, sections, _)| sections.container.is_some())
        .min_by(|a, b| {
            let da = a.3.translation().distance_squared(feet);
            let db = b.3.translation().distance_squared(feet);
            da.total_cmp(&db)
        });
    let Some((object, placed, sections, transform)) = nearest else {
        info!("open-container: no placed container in the level");
        return;
    };
    let pointer = windows
        .single()
        .map(|window| {
            IVec2::new(
                window.physical_width() as i32 / 2,
                window.physical_height() as i32 / 2,
            )
        })
        .unwrap_or(IVec2::new(640, 360));
    let (cells, items) = sections
        .container
        .as_ref()
        .map_or(((0, 0), 0), |c| ((c.width, c.height), c.children.len()));
    info!(
        "open-container: {:#x} at {:.2} m, {}x{} cells, {items} items",
        placed.tagged_id,
        transform.translation().distance(feet),
        cells.0,
        cells.1
    );
    requests.write(ContainerRequest {
        object,
        id: placed.tagged_id,
        pointer,
    });
}

/// Hands the loaded level's own records to the item catalogue once both
/// are there (and again after each level load), so a level-local item's
/// icon renders and its bulk counts in the panels.
fn register_level_items(
    level: Option<Res<exa_world::LevelLocalItems>>,
    catalogue: Option<ResMut<exa_items::catalogue::ItemCatalogue>>,
    mut done: Local<bool>,
) {
    let (Some(level), Some(mut catalogue)) = (level, catalogue) else {
        return;
    };
    if level.is_changed() {
        *done = false;
    }
    if *done || catalogue.is_empty() {
        return;
    }
    catalogue.set_level_items(&level.0);
    *done = true;
}

/// Writes the records the closed popups saved back into their objects'
/// container sections.
fn store_saved_containers(
    mut screens: ResMut<GameScreens>,
    mut objects: Query<&mut exa_world::ObjectSections>,
) {
    for (object, record) in screens.saved_containers.drain(..) {
        if let Ok(mut sections) = objects.get_mut(object)
            && let Some(section) = sections.container.as_mut()
        {
            store_record(section, &record);
        }
    }
}

/// Gives the icon manager back the handles of the panels the screens
/// closed this frame.
fn release_closed_icons(mut screens: ResMut<GameScreens>, mut icons: ResMut<PanelIcons>) {
    for handle in screens.released_icons.drain(..) {
        icons.release(handle);
    }
}

/// Keeps the task's screen size on the window's physical size.
fn sync_screen_size(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
) {
    if let Ok(window) = windows.single() {
        let size = (
            window.physical_width() as i32,
            window.physical_height() as i32,
        );
        if size != screens.task.screen {
            screens.task.screen = size;
        }
    }
}

/// Opens the screens the hotkeys asked for; Escape closes the focused one.
/// Nothing happens while the main menu is open.
#[allow(clippy::too_many_arguments)]
fn open_requested_screens(
    keys: Res<ButtonInput<KeyCode>>,
    mut requests: MessageReader<ScreenRequest>,
    players: Query<Entity, With<crate::player::Player>>,
    player_bodies: Query<Entity, With<crate::player::PlayerBody>>,
    possessions: Query<(Entity, &exa_items::possession::Possessions)>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    mut icons: Option<ResMut<PanelIcons>>,
    menu: Option<Res<MenuScreen>>,
    mut screens: ResMut<GameScreens>,
) {
    if menu.is_some_and(|menu| menu.open) {
        requests.clear();
        return;
    }
    if let Some(entity) = players.iter().next() {
        screens.player = entity.index().index() as i32;
    }
    for request in requests.read() {
        if !request.player_ready || !request.player_alive {
            continue;
        }
        let action = match request.screen {
            GameScreen::Inventory => {
                let action = screens.open_inventory(request.player_ready, request.player_alive, 4);
                // The pack is the player's own carried list, as the original
                // loads it when the inventory opens.
                if action == ScreenAction::Opened
                    && let Some(body) = player_bodies.iter().next()
                    && let Ok((_, possessions)) = possessions.get(body)
                    && let Some(icons) = icons.as_deref_mut()
                {
                    body_inventory::bind_pack(
                        &mut screens.pack,
                        possessions,
                        icons,
                        catalogue.as_deref(),
                    );
                }
                action
            }
            GameScreen::Journal => screens.open_journal(request.player_ready, request.player_alive),
            GameScreen::Loot => match request.target {
                Some(target) => {
                    let entity = possessions
                        .iter()
                        .find(|(entity, _)| entity.index().index() as i32 == target)
                        .map(|(entity, _)| entity);
                    // The window's show rule reads the body's record flag
                    // word: the profession word (byte 1 the race; race 3
                    // refuses), which the body's possessions keep.
                    let flags = entity
                        .and_then(|entity| possessions.get(entity).ok())
                        .map_or(request.flags, |(_, body)| body.flags);
                    let action = screens.open_loot(target, flags, 4);
                    if action == ScreenAction::Opened
                        && let Some(entity) = entity
                        && let Ok((_, body)) = possessions.get(entity)
                        && let Some(bounds) = screens
                            .loot
                            .iter()
                            .rev()
                            .find(|window| window.character == target)
                            .and_then(|window| screens.task.form(window.form))
                            .map(|form| form.bounds())
                        && let Some(icons) = icons.as_deref_mut()
                    {
                        let mut pack = Container::form(4);
                        pack.owner = Some(target as u32);
                        body_inventory::bind_pack(&mut pack, body, icons, catalogue.as_deref());
                        let (_, equipment) = loot_panel_rects(bounds);
                        let client = panel_client(equipment);
                        let mut worn = Container::form(4);
                        worn.owner = Some(target as u32);
                        worn.client = (client.width(), client.height());
                        body_inventory::bind_equipment(
                            &mut worn,
                            body,
                            icons,
                            catalogue.as_deref(),
                            (client.width(), client.height()),
                        );
                        screens.bind_loot(target, pack, worn);
                    }
                    action
                }
                None => continue,
            },
            _ => continue,
        };
        debug!("GUI: {:?} {action:?}", request.screen);
    }
    if keys.just_pressed(KeyCode::Escape) && screens.close_focused() {
        debug!("GUI: the focused screen closes");
    }
}

/// A right-button release over a screen that has the handler closes it
/// (the inventory's and the loot window's mouse routines); a release over
/// another form or outside leaves the screens alone.
pub(crate) fn close_on_right_release(
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
    card: Option<Res<crate::item_card::CardView>>,
) {
    let Some(buttons) = buttons else {
        return;
    };
    if !buttons.just_released(MouseButton::Right) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(pointer) = window.cursor_position() else {
        return;
    };
    let pointer = pointer * window.scale_factor();
    // A right press over an entry is the panel's card request; it must not
    // also close the screen. The next right release outside the card is
    // the card's own close.
    if card.as_deref().is_some_and(|card| card.active(&screens))
        || crate::item_card::entry_under(&screens, pointer.as_ivec2())
    {
        return;
    }
    let active = screens.task.active;
    let Some(screen) = screens
        .open
        .iter()
        .find(|screen| Some(screen.form) == active)
        .copied()
    else {
        return;
    };
    if !exa_ui::game_gui::right_release_closes(screen.kind) {
        return;
    }
    let Some(form) = screens.task.form(screen.form) else {
        return;
    };
    if form.bounds().contains(pointer.x as i32, pointer.y as i32) {
        screens.close(screen.kind);
    }
}

/// Closes the loot windows the original's deferred close takes: a window
/// whose body's state word has `0x80000`, or whose condition is no longer
/// the dead mode `0xa00`. Exarust keeps no copy of the body's state word, so
/// the mark half reads 0 [not modelled]; a window whose body is no longer in
/// the world is left alone.
fn close_changed_loot_windows(
    bodies: Query<(Entity, &exa_combat::Condition)>,
    mut screens: ResMut<GameScreens>,
) {
    if screens.loot.is_empty() {
        return;
    }
    let live: Vec<(i32, u32, u32)> = screens
        .loot
        .iter()
        .filter_map(|window| {
            bodies
                .iter()
                .find(|(entity, _)| entity.index().index() as i32 == window.character)
                .map(|(_, condition)| (window.character, 0, condition.0))
        })
        .collect();
    for character in screens.loot_windows_to_close(&live) {
        screens.close_loot(character);
    }
}

/// The font texture's size, for the glyph quads' sample step.
fn font_texture(textures: &GuiTextures, font: &exa_ui::gui_font::GuiFont) -> Option<(u16, u16)> {
    textures
        .get(font.texture)
        .map(|texture| (texture.width as u16, texture.height as u16))
}

/// The font a screen uses: the level's main font when its texture is
/// installed. Else the level's metrics (glyph quad, advances, line height,
/// which the layout and the controls' sizes are built from) are kept and the
/// glyph cells are sampled from the base font's atlas, the only GUI font the
/// packs ship. The size returned then is the atlas size the level's glyph
/// quad would span one base glyph cell of, so [`exa_ui::gui_text::font_uv`]
/// gives the base cell's sample step while the quad keeps the level's size.
pub(crate) fn screen_font(
    level_font: &exa_ui::gui_font::GuiFont,
    textures: &GuiTextures,
) -> (exa_ui::gui_font::GuiFont, Option<(u16, u16)>) {
    exa_ui::gui_font::font_with_fallback(
        level_font,
        font_texture(textures, level_font),
        font_texture(textures, &exa_ui::gui_font::FONTBASE_24),
    )
}

/// Paints the control records a screen's form carries: buttons with their
/// caption and labels as text. The unlocated control is not drawn.
fn paint_controls(
    controls: &[Control],
    unit: i32,
    palette: &exa_ui::skin::SkinPalette,
    font: &exa_ui::gui_font::GuiFont,
    size: Option<(u16, u16)>,
    painted: &mut gui_paint::Painted,
) {
    for control in controls {
        let rect = GuiRect::new(control.x, control.y, control.w, control.h);
        let text = String::from_utf8_lossy(&control.text);
        let layers = match control.class {
            1 => gui_paint::paint_button(
                rect,
                control.style,
                unit,
                palette,
                font,
                size,
                &text,
                false,
                false,
            ),
            5 => gui_paint::paint_label(rect, control.style, unit, palette, font, size, &text),
            _ => continue,
        };
        for layer in layers.layers {
            painted.push(layer);
        }
    }
}

/// The inventory's pack panel, as its painter places it: the client's
/// bottom, 228 × 180.
pub fn inventory_pack_panel(bounds: GuiRect) -> GuiRect {
    let client = GuiRect::new(
        bounds.left + gui_paint::INVENTORY_SPACING,
        bounds.top + gui_paint::INVENTORY_SPACING,
        bounds.width() - gui_paint::INVENTORY_SPACING * 2,
        bounds.height() - gui_paint::INVENTORY_SPACING * 2,
    );
    GuiRect::new(
        client.left,
        client.bottom - gui_paint::PACK.1,
        gui_paint::PACK.0,
        gui_paint::PACK.1,
    )
}

/// A loot window's pack and equipment panels, as its painter places them:
/// the pack at the client's bottom (228 × 180) and the equipment panel above
/// it, ending four units before the pack.
pub fn loot_panel_rects(bounds: GuiRect) -> (GuiRect, GuiRect) {
    let client = GuiRect::new(
        bounds.left + gui_paint::FORM_SPACING,
        bounds.top + gui_paint::FORM_SPACING,
        bounds.width() - gui_paint::FORM_SPACING * 2,
        bounds.height() - gui_paint::FORM_SPACING * 2,
    );
    let pack = GuiRect::new(
        client.left,
        client.bottom - exa_ui::game_gui::LOOT_PACK_HEIGHT,
        exa_items::char_loot::PACK_WIDTH,
        exa_ui::game_gui::LOOT_PACK_HEIGHT,
    );
    let equipment = GuiRect::new(
        client.left,
        client.top,
        exa_items::char_loot::PACK_WIDTH,
        (pack.top - client.top - exa_ui::game_gui::LOOT_PANEL_GAP).max(0),
    );
    (pack, equipment)
}

/// A panel's inner client, the four-unit inset its painter uses for the
/// entries' cells.
pub fn panel_client(rect: GuiRect) -> GuiRect {
    GuiRect::new(
        rect.left + 4,
        rect.top + 4,
        rect.width() - 8,
        rect.height() - 8,
    )
}

/// The inventory's frame, the item pack's panel with its entries' icons
/// (when the panel's icon atlas holds them) and the Encumbrance label and
/// Primary button records. The character view and the preview bars are
/// not drawn.
fn paint_inventory(
    bounds: GuiRect,
    unit: i32,
    textures: &GuiTextures,
    items: &Container,
    icons: Option<&mut PanelIcons>,
) -> gui_paint::Painted {
    let palette = gui_paint::palette();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, size) = screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let mut frame = Frame::new(bounds, unit, INVENTORY_TITLE);
    frame.spacing = gui_paint::INVENTORY_SPACING;
    let mut painted = gui_paint::paint_frame(&frame, &palette, &font, size);
    let pack = inventory_pack_panel(bounds);
    let panel = gui_paint::paint_panel(
        pack,
        unit,
        &palette,
        gui_paint::PACK_ATTRIBUTES,
        gui_paint::PACK_STYLE,
    );
    let panel_client = panel.client;
    for layer in panel.layers {
        painted.push(layer);
    }
    // The entries' cells are the panel's own coordinates: the icons are
    // moved to its client area and clipped to it.
    if let Some(icons) = icons
        && let Some(mut layer) = icons.layer(items)
    {
        gui_paint::offset_batch(&mut layer.batch, panel_client.left, panel_client.top);
        layer.clip = Some([
            panel_client.left,
            panel_client.top,
            panel_client.width(),
            panel_client.height(),
        ]);
        painted.push(layer);
    }
    let controls = screen_controls(GameScreen::Inventory, bounds);
    paint_controls(&controls, unit, &palette, &font, size, &mut painted);
    painted
}

/// The journal's frame, its New and Erase button records and the selected
/// entry's text through the text box's own draw.
fn paint_journal(
    bounds: GuiRect,
    journal: &Journal,
    unit: i32,
    textures: &GuiTextures,
) -> gui_paint::Painted {
    let palette = gui_paint::palette();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let fonts = exa_ui::gui_font::gui_level(level);
    let (font, size) = screen_font(fonts.main, textures);
    let mut frame = Frame::new(bounds, unit, JOURNAL_TITLE);
    frame.attributes = gui_paint::JOURNAL_ATTRIBUTES;
    let mut painted = gui_paint::paint_frame(&frame, &palette, &font, size);
    let client = frame.client();
    let controls = screen_controls(GameScreen::Journal, bounds);
    paint_controls(&controls, unit, &palette, &font, size, &mut painted);
    let box_client = exa_ui::form::ClientRect {
        left: client.left + JOURNAL_TEXT.left,
        top: client.top + JOURNAL_TEXT.top,
        right: client.left + JOURNAL_TEXT.right,
        bottom: client.top + JOURNAL_TEXT.bottom,
    };
    let mut text_box = exa_ui::text_box::TextBox::new(unit, fonts.main, fonts.small, box_client);
    text_box.read_only = true;
    let text = journal
        .entries
        .get(journal.current)
        .map(|entry| entry.text.as_bytes())
        .unwrap_or(b"");
    text_box.set_text(text, None);
    let inks = exa_ui::text_layout::TextInks {
        text: palette.text[6],
        tag_alpha: 255,
        selection: palette.highlight[2],
        slant: font.slant,
    };
    let draw = text_box.draw(&inks, &|_| None);
    if !draw.selections.is_empty() {
        let mut batch = SkinBatch::new();
        batch.select(1, 0, false);
        for (rect, ink) in &draw.selections {
            batch.set_ink(*ink);
            batch.begin();
            batch.vertex(
                (box_client.left + rect[0]) as f32,
                (box_client.top + rect[1]) as f32,
            );
            batch.vertex(
                (box_client.left + rect[0]) as f32,
                (box_client.top + rect[1] + rect[3]) as f32,
            );
            batch.vertex(
                (box_client.left + rect[0] + rect[2]) as f32,
                (box_client.top + rect[1] + rect[3]) as f32,
            );
            batch.vertex(
                (box_client.left + rect[0] + rect[2]) as f32,
                (box_client.top + rect[1]) as f32,
            );
            batch.end_quads();
        }
        painted.push(exa_ui::gui_draw::GuiLayer::new(
            batch,
            exa_ui::gui_draw::LayerTexture::None,
        ));
    }
    if let Some(texture) = size {
        let mut glyphs = draw.glyphs.clone();
        for glyph in &mut glyphs {
            glyph.x += box_client.left;
            glyph.y += box_client.top;
        }
        if !glyphs.is_empty() {
            let batch = exa_ui::gui_draw::glyph_batch(&font, &glyphs, texture, 1.0);
            painted.push(exa_ui::gui_draw::GuiLayer::new(
                batch,
                exa_ui::gui_draw::LayerTexture::Named(font.texture),
            ));
        }
    }
    painted
}

/// The loot window's frame and its two panels: the pack at the client's
/// bottom (228 × 180) and the equipment panel above it, ending four units
/// before the pack. Each panel draws its entries' icons when the icon cache
/// holds them, clipped to the panel's client.
fn paint_loot(
    bounds: GuiRect,
    unit: i32,
    textures: &GuiTextures,
    window: &LootWindow,
    mut icons: Option<&mut PanelIcons>,
) -> gui_paint::Painted {
    let palette = gui_paint::palette();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, size) = screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let frame = Frame::new(bounds, unit, LOOT_TITLE);
    let mut painted = gui_paint::paint_frame(&frame, &palette, &font, size);
    let (pack, equipment) = loot_panel_rects(bounds);
    let mut paint_panel_with =
        |rect: GuiRect, container: &Container, icons: &mut Option<&mut PanelIcons>| {
            let panel = gui_paint::paint_panel(
                rect,
                unit,
                &palette,
                gui_paint::PACK_ATTRIBUTES,
                gui_paint::PACK_STYLE,
            );
            let panel_client = panel.client;
            for layer in panel.layers {
                painted.push(layer);
            }
            if let Some(icons) = icons.as_deref_mut()
                && let Some(mut layer) = icons.layer(container)
            {
                gui_paint::offset_batch(&mut layer.batch, panel_client.left, panel_client.top);
                layer.clip = Some([
                    panel_client.left,
                    panel_client.top,
                    panel_client.width(),
                    panel_client.height(),
                ]);
                painted.push(layer);
            }
        };
    paint_panel_with(pack, &window.pack, &mut icons);
    paint_panel_with(equipment, &window.worn, &mut icons);
    painted
}

/// The container popup's frame words as its constructors leave them: the
/// attribute word the container constructor writes over the card's
/// (`0x830023`: title bar, close mark, client fill, no rounded skin), the
/// frame style with the card's centred-title bits (`0x3000`; the container
/// clears the card's `0x80`) and no title string (the opener sets none).
pub fn popup_frame(bounds: GuiRect, unit: i32) -> Frame {
    let mut frame = Frame::new(bounds, unit, "");
    frame.attributes = exa_ui::popups::CONTAINER_FLAGS;
    frame.style = (gui_paint::SCREEN_STYLE | 0x3080) & !exa_ui::popups::CONTAINER_EXTRA_CLEAR;
    frame.spacing = POPUP_BORDER;
    frame
}

/// A container popup: the card's frame (no title, the popup's eight-pixel
/// border), the container's icon over the icon area and the item panel
/// with its entries' icons, clipped to the panel's client.
fn paint_container_popup(
    bounds: GuiRect,
    unit: i32,
    textures: &GuiTextures,
    popup: &ContainerPopup,
    mut icons: Option<&mut PanelIcons>,
) -> gui_paint::Painted {
    let palette = gui_paint::palette();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, size) = screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let mut frame = popup_frame(bounds, unit);
    frame.spacing = POPUP_BORDER;
    let mut painted = gui_paint::paint_frame(&frame, &palette, &font, size);
    let client = ContainerPopup::client(bounds);
    if let Some(icons) = icons.as_deref_mut() {
        let (x, y) = popup_icon_place(
            bounds.width(),
            popup.icon_height,
            unit,
            popup.panel.icon_size,
        );
        let mut own = Container::form(unit);
        own.entries.push(crate::item_container::Entry {
            cell: crate::item_container::Cell::new(x, y),
            id: popup.id,
            icon: popup.icon,
            flags: 0,
            drag: 0,
        });
        if let Some(mut layer) = icons.layer(&own) {
            gui_paint::offset_batch(&mut layer.batch, client.left, client.top);
            painted.push(layer);
        }
    }
    let panel = gui_paint::paint_panel(
        popup.panel_rect(bounds),
        unit,
        &palette,
        gui_paint::PACK_ATTRIBUTES,
        gui_paint::PACK_STYLE,
    );
    let panel_client = panel.client;
    for layer in panel.layers {
        painted.push(layer);
    }
    if let Some(icons) = icons
        && let Some(mut layer) = icons.layer(&popup.panel)
    {
        gui_paint::offset_batch(&mut layer.batch, panel_client.left, panel_client.top);
        layer.clip = Some([
            panel_client.left,
            panel_client.top,
            panel_client.width(),
            panel_client.height(),
        ]);
        painted.push(layer);
    }
    painted
}

/// Draws the open screens and the debug form into the GUI's queue.
pub(crate) fn draw_screens(
    screens: Res<GameScreens>,
    mut icons: ResMut<PanelIcons>,
    debug: Res<DebugGuiForm>,
    menu: Option<Res<MenuScreen>>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
) {
    frames.clear();
    if menu.is_some_and(|menu| menu.open) {
        return;
    }
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let unit = windows
        .single()
        .ok()
        .map(|window| exa_ui::scale::gui_scale(window.physical_height(), 0) as i32)
        .unwrap_or(4);
    for screen in &screens.open {
        let Some(form) = screens.task.form(screen.form) else {
            continue;
        };
        let bounds = form.bounds();
        let painted = match screen.kind {
            GameScreen::Inventory => {
                paint_inventory(bounds, unit, textures, &screens.pack, Some(&mut icons))
            }
            GameScreen::Journal => paint_journal(bounds, &screens.journal, unit, textures),
            GameScreen::Loot => match screens.loot.iter().find(|w| w.form == screen.form) {
                Some(window) => paint_loot(bounds, unit, textures, window, Some(&mut icons)),
                None => continue,
            },
            GameScreen::Container => match screens.popups.iter().find(|p| p.form == screen.form) {
                Some(popup) => {
                    paint_container_popup(bounds, unit, textures, popup, Some(&mut icons))
                }
                None => continue,
            },
            _ => continue,
        };
        painted.submit(&mut frames);
    }
    if debug.enabled {
        let rect = GuiRect::new(100, 100, 302, 202);
        gui_paint::paint_debug_form(rect, unit, textures).submit(&mut frames);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client_of(bounds: GuiRect, spacing: i32) -> (i32, i32) {
        (bounds.left + spacing, bounds.top + spacing)
    }

    #[test]
    fn the_inventory_records_lay_out_where_the_constructor_places_them() {
        let bounds = GuiRect::new(8, 8, 244, 644);
        let (left, top) = client_of(bounds, gui_paint::INVENTORY_SPACING);
        let controls = screen_controls(GameScreen::Inventory, bounds);
        assert_eq!(controls.len(), 3);
        let place = |c: &Control| (c.x - left, c.y - top, c.w, c.h);
        assert_eq!(controls[0].class, 5);
        assert_eq!((place(&controls[0]).0, place(&controls[0]).1), (4, 386));
        assert_eq!(controls[1].class, UNLOCATED_CLASS);
        assert_eq!(controls[2].class, 1);
        assert_eq!(place(&controls[2]), (4, 416, 108, 24));
    }

    #[test]
    fn the_journal_records_lay_out_where_the_constructor_places_them() {
        let bounds = GuiRect::new(4, 36, 244, 518);
        let (left, top) = client_of(bounds, gui_paint::FORM_SPACING);
        let controls = screen_controls(GameScreen::Journal, bounds);
        assert_eq!(controls.len(), 2);
        let place = |c: &Control| (c.x - left, c.y - top, c.w, c.h);
        assert_eq!(place(&controls[0]), (4, 20, 104, 24));
        assert_eq!(place(&controls[1]), (120, 20, 104, 24));
    }

    #[test]
    fn the_inventory_opens_focuses_and_toggles() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.player = 7;
        assert_eq!(screens.open_inventory(true, true, 4), ScreenAction::Opened);
        let form = screens.find(GameScreen::Inventory).unwrap().form;
        assert_eq!(
            screens.task.form(form).unwrap().bounds(),
            GuiRect::new(8, 8, 244, 644)
        );
        assert_eq!(screens.focused(), Some(GameScreen::Inventory));
        assert!(screens.containers.is_open(0x2000_0007));
        assert_eq!(screens.open_inventory(true, true, 4), ScreenAction::Closed);
        assert!(!screens.is_open(GameScreen::Inventory));
        assert!(!screens.containers.is_open(0x2000_0007));
        assert_eq!(screens.inventory_place, Some((8, 8)));
        assert_eq!(
            screens.open_inventory(false, true, 4),
            ScreenAction::Refused
        );
    }

    #[test]
    fn closing_a_panel_queues_its_icon_handles() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.open_inventory(true, true, 4);
        screens.pack.entries = vec![
            crate::item_container::Entry {
                icon: 3,
                ..Default::default()
            },
            crate::item_container::Entry {
                icon: 7,
                ..Default::default()
            },
        ];
        assert!(screens.close(GameScreen::Inventory));
        assert_eq!(screens.released_icons, vec![3, 7]);
        // The destroyed panel keeps no entries.
        assert!(screens.pack.entries.is_empty());
        // A loot window queues both of its panels' handles.
        screens.open_loot(7, 0, 4);
        screens.loot[0].pack.entries = vec![crate::item_container::Entry {
            icon: 4,
            ..Default::default()
        }];
        screens.loot[0].worn.entries = vec![crate::item_container::Entry {
            icon: 5,
            ..Default::default()
        }];
        assert!(screens.close(GameScreen::Loot));
        assert_eq!(screens.released_icons, vec![3, 7, 4, 5]);
    }

    #[test]
    fn an_inactive_screen_is_focused_not_closed() {
        let mut screens = GameScreens::default();
        screens.open_inventory(true, true, 4);
        screens.open_journal(true, true);
        assert_eq!(screens.focused(), Some(GameScreen::Journal));
        assert_eq!(screens.open_inventory(true, true, 4), ScreenAction::Focused);
        assert_eq!(screens.focused(), Some(GameScreen::Inventory));
        assert_eq!(screens.close_container_family(), 2);
        assert!(screens.open.is_empty());
    }

    #[test]
    fn the_popup_frame_takes_the_container_constructors_words() {
        let frame = popup_frame(GuiRect::new(10, 20, 224, 308), 4);
        assert_eq!(frame.attributes, 0x83_0023);
        assert_eq!(frame.style, 0x3000);
        assert_eq!(frame.spacing, 8);
        assert!(frame.title.is_empty());
    }

    #[test]
    fn the_container_popup_opens_on_the_pointer_and_saves_on_close() {
        use crate::item_container::Cell;
        let mut screens = GameScreens::default();
        screens.task.screen = (1280, 720);
        let object = Entity::from_bits(42);
        let id = exa_formats::objdb::LOCAL | 4;
        let record = ContainerRecord {
            size: (50, 30),
            capacity: 400,
            max_item: 60,
            children: vec![ContainerChild {
                id: 9,
                flags: 0x2000_0000,
                x: 800,
                y: 400,
            }],
        };
        let mut slots = 10..;
        let action =
            screens.open_container_popup(object, id, &record, (640, 400), 4, 3, 70, |_, flags| {
                // The resolver sees the flags' low 28 bits.
                assert_eq!(flags, 0);
                Some((slots.next().unwrap(), 5))
            });
        assert_eq!(action, ScreenAction::Opened);
        assert_eq!(screens.focused(), Some(GameScreen::Container));
        assert!(screens.containers.is_open(id as i32));
        let popup = screens.popup(id).unwrap();
        let bounds = screens.task.form(popup.form).unwrap().bounds();
        assert_eq!(bounds, GuiRect::new(528, 124, 224, 308));
        assert_eq!(
            popup.panel_rect(bounds),
            GuiRect::new(536, 132 + 164, 208, 128)
        );
        assert_eq!(popup.panel.client, (200, 120));
        assert_eq!((popup.panel.capacity, popup.panel.used), (400, 5));
        assert_eq!(popup.panel.entries[0].cell, Cell::new(40, 20));
        assert_eq!(popup.panel.entries[0].flags, 0x2000_0000);
        // The same container again only comes forward.
        assert_eq!(
            screens.open_container_popup(object, id, &record, (0, 0), 4, 3, 70, |_, _| None),
            ScreenAction::Focused
        );
        // Moving the entry and closing saves it back, in 1/80 units.
        screens.popups[0].panel.entries[0].cell = Cell::new(8, 4);
        assert!(screens.close_popup(id));
        assert!(screens.popups.is_empty());
        assert!(!screens.containers.is_open(id as i32));
        let (saved_object, saved) = screens.saved_containers.pop().unwrap();
        assert_eq!(saved_object, object);
        assert_eq!(
            (saved.size, saved.capacity, saved.max_item),
            ((50, 30), 400, 60)
        );
        assert_eq!(
            saved.children,
            vec![ContainerChild {
                id: 9,
                flags: 0x2000_0000,
                x: 160,
                y: 80,
            }]
        );
        assert_eq!(screens.released_icons, vec![10, 3]);
        // The section takes the saved children.
        let mut section = exa_formats::containers::Container {
            width: 50,
            height: 30,
            capacity: 400,
            max_item: 60,
            children: Vec::new(),
        };
        store_record(&mut section, &saved);
        assert_eq!(container_record(&section), saved);
        // A container without an id opens nothing.
        assert_eq!(
            screens.open_container_popup(object, 0, &record, (0, 0), 4, 0, 70, |_, _| None),
            ScreenAction::Refused
        );
    }

    #[test]
    fn the_loot_window_opens_per_body_and_remembers_its_panels() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        assert_eq!(screens.open_loot(7, 0, 4), ScreenAction::Opened);
        let form = screens.loot_window(7).unwrap().form;
        assert_eq!(
            screens.task.form(form).unwrap().bounds(),
            GuiRect::new(400, 36, 244, 440)
        );
        assert_eq!(screens.focused(), Some(GameScreen::Loot));
        assert!(screens.containers.is_open(0x2000_0007));
        // The same body again focuses its window; another body opens a
        // second one and keeps the first.
        assert_eq!(screens.open_loot(7, 0, 4), ScreenAction::Focused);
        assert_eq!(screens.open_loot(8, 0, 4), ScreenAction::Opened);
        assert_eq!(screens.loot.len(), 2);
        assert!(screens.containers.is_open(0x2000_0008));
        assert_eq!(screens.focused(), Some(GameScreen::Loot));
        // The focused window (body 8) closes; body 7's stays open.
        assert!(screens.close_focused());
        assert!(!screens.containers.is_open(0x2000_0008));
        assert!(screens.containers.is_open(0x2000_0007));
        assert_eq!(screens.loot.len(), 1);
        // A body whose flag word's second byte is 3 is refused.
        assert_eq!(screens.open_loot(9, 0x300, 4), ScreenAction::Refused);
        assert!(screens.loot_window(9).is_none());
    }

    #[test]
    fn the_loot_panels_bind_the_bodys_items() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.open_loot(7, 0, 4);
        let mut pack = Container::form(4);
        pack.entries = vec![crate::item_container::Entry {
            cell: crate::item_container::Cell::new(10, 20),
            id: 7,
            icon: 1,
            flags: 0,
            drag: 0,
        }];
        let mut worn = Container::form(4);
        worn.entries = vec![crate::item_container::Entry {
            cell: crate::item_container::Cell::new(30, 40),
            id: 8,
            icon: 1,
            flags: 0,
            drag: 0,
        }];
        assert!(screens.bind_loot(7, pack, worn));
        assert_eq!(screens.loot_window(7).unwrap().pack.entries[0].id, 7);
        assert_eq!(screens.loot_window(7).unwrap().worn.entries[0].id, 8);
        assert!(!screens.bind_loot(99, Container::default(), Container::default()));
    }

    #[test]
    fn a_loot_window_closes_when_the_body_is_no_longer_dead() {
        let mut app = App::new();
        app.add_systems(Update, close_changed_loot_windows);
        let body = app.world_mut().spawn(exa_combat::Condition::DEAD).id();
        let target = body.index().index() as i32;
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        assert_eq!(screens.open_loot(target, 0, 4), ScreenAction::Opened);
        app.insert_resource(screens);
        app.update();
        assert!(
            app.world()
                .resource::<GameScreens>()
                .loot_window(target)
                .is_some()
        );
        app.world_mut()
            .get_mut::<exa_combat::Condition>(body)
            .unwrap()
            .0 = exa_combat::Condition::NORMAL.0;
        app.update();
        assert!(
            app.world()
                .resource::<GameScreens>()
                .loot_window(target)
                .is_none()
        );
    }

    #[test]
    fn the_inventory_request_binds_the_players_carried_list() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .add_message::<ScreenRequest>()
            .add_systems(Update, open_requested_screens);
        app.world_mut().spawn((
            crate::player::PlayerBody::new(0.0),
            exa_items::possession::Possessions {
                carried: vec![exa_items::possession::Carried {
                    id: 0x30,
                    flags: 0,
                    x: 0x28,
                    y: 4,
                }],
                ..Default::default()
            },
        ));
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Inventory, true, true));
        app.update();
        let screens = app.world().resource::<GameScreens>();
        assert_eq!(screens.pack.entries.len(), 1);
        assert_eq!(screens.pack.entries[0].id, 0x30);
        // The panel's icon half extents carry the GUI unit, so the top
        // clamp lifts the saved y of 1 to 4, as the icon's own halves do.
        assert_eq!(
            screens.pack.entries[0].cell,
            crate::item_container::Cell::new(10, 4)
        );
    }

    #[test]
    fn the_loot_request_binds_the_bodys_record_and_its_condition_flags() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .add_message::<ScreenRequest>()
            .add_systems(Update, open_requested_screens);
        let record = exa_formats::characters::CharacterRecord {
            hands: [
                exa_formats::characters::ItemRef::new(0x11, 0),
                exa_formats::characters::ItemRef::new(0x12, 0),
                exa_formats::characters::ItemRef::default(),
                exa_formats::characters::ItemRef::default(),
            ],
            worn: vec![exa_formats::characters::ItemRef::new(0xa0, 0)],
            inventory: vec![exa_formats::characters::Carried {
                item: 0x30,
                flags: 0,
                extra: 0,
            }],
            race: 1,
            ..exa_formats::characters::CharacterRecord::default()
        };
        let entity = app
            .world_mut()
            .spawn((
                exa_items::possession::Possessions::of_record(&record),
                exa_combat::Condition::DEAD,
            ))
            .id();
        let target = entity.index().index() as i32;
        // The request's flags are stale; the body's record flag word (its
        // profession word, race 1 here) is what the show rule reads.
        app.world_mut().write_message(ScreenRequest {
            screen: GameScreen::Loot,
            player_ready: true,
            player_alive: true,
            target: Some(target),
            flags: 0x300,
        });
        app.update();
        let screens = app.world().resource::<GameScreens>();
        let window = screens.loot_window(target).unwrap();
        assert_eq!(window.flags, 0x100);
        assert_eq!(window.pack.entries.len(), 1);
        assert_eq!(window.pack.entries[0].id, 0x30);
        // Worn first, then only the shields in the hands: with no item
        // catalogue nothing in a hand is known to be a shield.
        let ids: Vec<u32> = window.worn.entries.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![0xa0]);
    }

    #[test]
    fn a_body_of_race_three_refuses_the_loot_window() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .add_message::<ScreenRequest>()
            .add_systems(Update, open_requested_screens);
        let record = exa_formats::characters::CharacterRecord {
            race: 3,
            ..exa_formats::characters::CharacterRecord::default()
        };
        let entity = app
            .world_mut()
            .spawn((
                exa_items::possession::Possessions::of_record(&record),
                exa_combat::Condition::DEAD,
            ))
            .id();
        let target = entity.index().index() as i32;
        app.world_mut().write_message(ScreenRequest {
            screen: GameScreen::Loot,
            player_ready: true,
            player_alive: true,
            target: Some(target),
            flags: 0,
        });
        app.update();
        // 0x300 & 0xff00 == 0x300: the show rule refuses.
        assert!(
            app.world()
                .resource::<GameScreens>()
                .loot_window(target)
                .is_none()
        );
    }

    #[test]
    fn the_loot_form_draws_its_two_panels() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .add_message::<ScreenRequest>()
            .add_systems(Update, open_requested_screens);
        app.world_mut().write_message(ScreenRequest {
            screen: GameScreen::Loot,
            player_ready: true,
            player_alive: true,
            target: Some(7),
            flags: 0,
        });
        app.update();
        let screens = app.world().resource::<GameScreens>();
        assert!(screens.loot_window(7).is_some());
        // The loot form has no control records; its paint is the frame and
        // the two item panels.
        assert!(screen_controls(GameScreen::Loot, GuiRect::new(400, 36, 244, 440)).is_empty());
        let window = screens.loot_window(7).unwrap();
        let painted = paint_loot(
            GuiRect::new(400, 36, 244, 440),
            4,
            &GuiTextures::default(),
            window,
            None,
        );
        assert!(painted.layers.len() >= 3);
        // The pack panel's frame starts at the client's left and its bottom
        // less 180; the equipment panel above it ends four units before.
        let client = painted.client;
        assert_eq!(
            (client.left, client.top, client.width(), client.height()),
            (402, 38, 240, 436)
        );
        let pack = painted.layers[1]
            .batch
            .vertices
            .iter()
            .map(|v| (v.x, v.y))
            .fold((f32::MAX, f32::MAX), |a, b| (a.0.min(b.0), a.1.min(b.1)));
        assert!((pack.0 - client.left as f32).abs() < 2.0, "{pack:?}");
        assert!(
            (pack.1 - (client.bottom - 180) as f32).abs() < 2.0,
            "{pack:?}"
        );
        let equipment = painted.layers[2]
            .batch
            .vertices
            .iter()
            .map(|v| (v.x, v.y))
            .fold((f32::MAX, f32::MAX), |a, b| (a.0.min(b.0), a.1.min(b.1)));
        assert!(
            (equipment.0 - client.left as f32).abs() < 2.0,
            "{equipment:?}"
        );
        assert!(
            (equipment.1 - client.top as f32).abs() < 2.0,
            "{equipment:?}"
        );
    }

    #[test]
    fn the_deferred_close_takes_the_windows_of_dead_bodies_only() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.open_loot(7, 0, 4);
        screens.open_loot(8, 0, 4);
        // 7 is still dead and unmarked; 8 is no longer dead; 9 has no window.
        assert_eq!(
            screens.loot_windows_to_close(&[(7, 0, 0xa00), (8, 0, 0), (9, 0x80000, 0xa00)]),
            vec![8]
        );
        assert_eq!(
            screens.loot_windows_to_close(&[(7, 0x80000, 0xa00)]),
            vec![7]
        );
        // Both close in the windows' order, whatever the bodies' order.
        assert_eq!(
            screens.loot_windows_to_close(&[(8, 0, 0), (7, 0, 0)]),
            vec![7, 8]
        );
    }

    #[test]
    fn the_journal_opens_and_remembers() {
        let mut screens = GameScreens::default();
        assert_eq!(screens.open_journal(true, true), ScreenAction::Opened);
        let form = screens.find(GameScreen::Journal).unwrap().form;
        assert_eq!(
            screens.task.form(form).unwrap().bounds(),
            GuiRect::new(4, 36, 244, 518)
        );
        screens.journal_text = "a note".to_string();
        assert_eq!(screens.open_journal(true, true), ScreenAction::Closed);
        assert_eq!(screens.journal.entries[0].text, "a note");
        assert_eq!(screens.journal.place, Some((4, 36)));
        assert_eq!(screens.open_journal(false, true), ScreenAction::Refused);
    }

    #[test]
    fn the_draw_path_runs_for_three_frames_on_an_empty_app() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(GameScreensPlugin);
        for _ in 0..3 {
            app.update();
        }
        assert!(app.world().resource::<GuiFrames>().frames.is_empty());
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Inventory, true, true));
        for _ in 0..3 {
            app.update();
        }
        let frames = app.world().resource::<GuiFrames>();
        assert_eq!(frames.frames.len(), 1);
        assert!(frames.primitives() > 0);
    }

    #[test]
    fn an_inventory_request_submits_a_frame_with_the_skins_modes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(GameScreensPlugin);
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Inventory, true, true));
        app.update();
        let frames = app.world().resource::<GuiFrames>();
        assert_eq!(frames.frames.len(), 1);
        let layer = &frames.frames[0].layers[0];
        assert!(!layer.batch.primitives.is_empty());
        let first = &layer.batch.vertices_of(&layer.batch.primitives[0])[0];
        assert!(first.x >= 8.0 && first.y >= 8.0);
        assert!(first.x <= 8.0 + 244.0 && first.y <= 8.0 + 644.0);
        assert!(layer.batch.primitives.iter().any(|primitive| {
            matches!(
                primitive.mode,
                exa_ui::skin::SkinMode::Solid
                    | exa_ui::skin::SkinMode::Rounded
                    | exa_ui::skin::SkinMode::Skin
            )
        }));
    }

    #[test]
    fn a_pack_entry_with_an_icon_draws_through_the_panel_program() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(GameScreensPlugin);
        {
            let mut icons = app.world_mut().resource_mut::<PanelIcons>();
            let mut image = exa_ui::icon_atlas::IconImage::new(160, 160, 4);
            image.pixels.fill(255);
            let cell = icons.atlas.add(&image, 7);
            icons.cache.entries = vec![exa_items::icon_cache::Entry {
                id: 7,
                cell,
                variant: 0,
                half_width: 84,
                half_height: 84,
                refs: 1,
            }];
        }
        {
            let mut screens = app.world_mut().resource_mut::<GameScreens>();
            screens.pack.icon_size = 160;
            screens.pack.entries = vec![crate::item_container::Entry {
                cell: crate::item_container::Cell::new(10, 20),
                id: 7,
                icon: 1,
                flags: 0,
                drag: 0,
            }];
        }
        let request = ScreenRequest::screen(GameScreen::Inventory, true, true);
        app.world_mut().write_message(request);
        app.update();
        let frames = app.world().resource::<GuiFrames>();
        assert_eq!(frames.frames.len(), 1);
        let panel: Vec<_> = frames.frames[0]
            .layers
            .iter()
            .filter(|layer| layer.batch.mode == exa_ui::skin::SkinMode::HltCol)
            .collect();
        assert_eq!(panel.len(), 1);
        assert_eq!(panel[0].batch.primitives.len(), 1);
        // The cell is placed inside the pack panel's client area and the
        // layer is clipped to it.
        let clip = panel[0].clip.unwrap();
        let first = panel[0].batch.vertices_of(&panel[0].batch.primitives[0])[0];
        assert_eq!(first.x, (clip[0] + 10) as f32);
        assert!(first.y >= clip[1] as f32);
    }

    #[test]
    fn without_the_level_font_the_base_atlas_is_sampled_at_the_levels_size() {
        use exa_ui::gui_draw::{GuiTexture, GuiTextures};
        let level = exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(4)).main;
        let mut textures = GuiTextures::default();
        // Nothing installed: no size, the base name.
        let (font, size) = screen_font(level, &textures);
        assert_eq!(font.texture, "fontbase24r");
        assert_eq!(size, None);
        textures.textures.insert(
            "fontbase24r".into(),
            GuiTexture::from_top_down(1024, 1024, &vec![0u8; 1024 * 1024 * 4]).unwrap(),
        );
        let (font, size) = screen_font(level, &textures);
        // The level's quad, advances and line height stay.
        assert_eq!(font.glyph, level.glyph);
        assert_eq!(font.line_height, level.line_height);
        assert_eq!(font.advances, level.advances);
        // The sample step is still the base glyph's: 36 and 48 texels.
        let (du, dv) = exa_ui::gui_text::font_uv(&font, size.unwrap()).unwrap();
        assert!((du - 36.0 / 1024.0).abs() < 1e-4, "{du}");
        assert!((dv - 48.0 / 1024.0).abs() < 1e-4, "{dv}");
        // With the level's own texture the level font is used as is.
        textures
            .textures
            .insert(level.texture.into(), GuiTexture::solid([255; 4]));
        let (font, size) = screen_font(level, &textures);
        assert_eq!(font.texture, level.texture);
        assert_eq!(size, Some((1, 1)));
    }

    #[test]
    fn the_journals_selected_text_reaches_the_font_layer() {
        use exa_ui::gui_draw::{GuiTexture, GuiTextures, LayerTexture};
        let mut textures = GuiTextures::default();
        let font = exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(4)).main;
        textures
            .textures
            .insert(font.texture.into(), GuiTexture::solid([255; 4]));
        textures.textures.insert(
            exa_ui::gui_draw::SKIN_TEXTURE.into(),
            GuiTexture::solid([10, 20, 30, 255]),
        );
        let journal = Journal {
            entries: vec![
                exa_ui::game_gui::JournalEntry {
                    title: "First".into(),
                    text: "A note".into(),
                },
                exa_ui::game_gui::JournalEntry {
                    title: "Second".into(),
                    text: "Another note".into(),
                },
            ],
            current: 1,
            place: None,
        };
        let painted = paint_journal(GuiRect::new(4, 36, 244, 518), &journal, 4, &textures);
        let font_layers: Vec<_> = painted
            .layers
            .iter()
            .filter(
                |layer| matches!(layer.texture, LayerTexture::Named(name) if name == font.texture),
            )
            .collect();
        assert!(!font_layers.is_empty());
        let glyphs: usize = font_layers
            .iter()
            .map(|layer| {
                layer
                    .batch
                    .primitives
                    .iter()
                    .map(|p| p.count)
                    .sum::<usize>()
            })
            .sum();
        assert!(glyphs >= "Another note".len() * 4);
        // Two buttons and the title are also drawn.
        assert!(painted.layers.len() >= 4);
    }

    #[test]
    fn a_journal_with_two_entries_draws_the_selected_text() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(GameScreensPlugin);
        {
            let mut screens = app.world_mut().resource_mut::<GameScreens>();
            screens.journal.entries = vec![
                exa_ui::game_gui::JournalEntry {
                    title: "First".into(),
                    text: "A note".into(),
                },
                exa_ui::game_gui::JournalEntry {
                    title: "Second".into(),
                    text: "Another note".into(),
                },
            ];
            screens.journal.current = 1;
        }
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Journal, true, true));
        app.update();
        let frames = app.world().resource::<GuiFrames>();
        assert_eq!(frames.frames.len(), 1);
        let primitives: usize = frames.frames[0]
            .layers
            .iter()
            .map(|layer| layer.batch.primitives.len())
            .sum();
        assert!(primitives > 0);
    }

    #[test]
    fn the_requests_open_and_escape_closes() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .add_message::<ScreenRequest>()
            .add_systems(Update, open_requested_screens);
        let request = |screen| ScreenRequest::screen(screen, true, true);
        app.world_mut()
            .write_message(request(GameScreen::Inventory));
        app.update();
        assert!(
            app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Inventory)
        );
        app.world_mut().write_message(request(GameScreen::Journal));
        app.update();
        assert_eq!(
            app.world().resource::<GameScreens>().focused(),
            Some(GameScreen::Journal)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        let screens = app.world().resource::<GameScreens>();
        assert!(!screens.is_open(GameScreen::Journal));
        assert_eq!(screens.focused(), Some(GameScreen::Inventory));
    }
}
