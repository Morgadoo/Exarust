//! The running game's in-game screens: the inventory and journal the
//! player's keys open, kept as forms on a [`GuiTask`] so their focus follows
//! the window manager. The show rules, tags, places and the journal's
//! entries are [`exa_ui::game_gui`]'s; the keys come through the player's
//! hotkey handler ([`crate::hotkeys`]), which applies its own permissions
//! and sends a [`ScreenRequest`].
//!
//! The screens are not drawn yet and hold no controls; what runs is the
//! open, focus and close state, the open-container registry and the
//! journal's session entries. Escape closing the focused screen is
//! Exarust's (the original closes them through their frame marks and the
//! keys again). The skills key's screen and the power tree are outside
//! this unit; the technique selector opens from a power slot, which the
//! HUD does not draw yet.
//!
//! Rules: `docs/UI.md` (The in-game GUI unit).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ui::form::{ClientRect, UNPLACED, place_unset};
use exa_ui::game_gui::{
    CONTAINER_FAMILY, GameScreen, INVENTORY_PLACE, InventoryInput, Journal, JournalInput,
    OpenContainers, ScreenAction, forms_with_tag, inventory_place, inventory_screen,
    journal_screen, tagged_container,
};
use exa_ui::gui_core::{FormHost, FormId, GuiRect, GuiTask, NO_CONTROL};

use crate::menu_screen::MenuScreen;

/// A screen the player's keys ask for.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRequest {
    pub screen: GameScreen,
    /// The player is not out of action (not collapsing).
    pub player_ready: bool,
    /// The player's condition is not the dead mode 0xa00.
    pub player_alive: bool,
}

/// One open screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenScreen {
    pub kind: GameScreen,
    pub form: FormId,
}

/// A screen's form: a rectangle with no controls yet. Its close marks it
/// doomed; the task deletes it.
pub struct ScreenForm {
    pub parent: Option<FormId>,
    pub bounds: GuiRect,
    pub doomed: bool,
    pub selected: i32,
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

    fn push(&mut self, kind: GameScreen, bounds: GuiRect) {
        let root = self.task.root;
        let form = self.task.add(Box::new(ScreenForm {
            parent: root,
            bounds,
            doomed: false,
            selected: 0,
        }));
        self.open.push(OpenScreen { kind, form });
        self.task.focus_set(Some(form), NO_CONTROL);
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

    /// Closes a screen: its form closes and the task deletes it; the
    /// journal stores its text (its save) and the screens remember their
    /// place; the inventory leaves the container registry. The focus falls
    /// to the last screen left.
    pub fn close(&mut self, kind: GameScreen) -> bool {
        let Some(index) = self.open.iter().position(|screen| screen.kind == kind) else {
            return false;
        };
        let screen = self.open.remove(index);
        let place = self.task.form(screen.form).map(|form| {
            let b = form.bounds();
            (b.left, b.top)
        });
        if let Some(form) = self.task.form_mut(screen.form) {
            form.close(0);
        }
        self.task.delete_doomed();
        match kind {
            GameScreen::Journal => {
                self.journal.store_text(&self.journal_text);
                self.journal.place = place;
            }
            GameScreen::Inventory => {
                self.containers.close(tagged_container(self.player));
                self.inventory_place = place;
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
            .add_message::<ScreenRequest>()
            .add_systems(
                Update,
                (sync_screen_size, open_requested_screens)
                    .chain()
                    .after(crate::hotkeys::HotkeySet)
                    .before(crate::player::player_combat),
            );
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
fn open_requested_screens(
    keys: Res<ButtonInput<KeyCode>>,
    mut requests: MessageReader<ScreenRequest>,
    players: Query<Entity, With<crate::player::Player>>,
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
        let action = match request.screen {
            GameScreen::Inventory => {
                screens.open_inventory(request.player_ready, request.player_alive, 4)
            }
            GameScreen::Journal => screens.open_journal(request.player_ready, request.player_alive),
            _ => continue,
        };
        debug!("GUI: {:?} {action:?}", request.screen);
    }
    if keys.just_pressed(KeyCode::Escape) && screens.close_focused() {
        debug!("GUI: the focused screen closes");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_requests_open_and_escape_closes() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .add_message::<ScreenRequest>()
            .add_systems(Update, open_requested_screens);
        let request = |screen| ScreenRequest {
            screen,
            player_ready: true,
            player_alive: true,
        };
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
