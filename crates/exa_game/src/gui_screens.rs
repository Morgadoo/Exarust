//! The running game's in-game screens: the character/inventory, journal and
//! skill selector that the player's keys open. The original's unit routines
//! are in [`exa_ui::game_gui`]; this module keeps the open screens on a
//! [`GuiTask`] so their focus follows the window manager, applies the show
//! rules and saves the session state they hold.
//!
//! The screens are not drawn yet (their form classes belong to the game's
//! GUI, which Exarust has no renderer for); what runs here is the open and
//! close state, the focus and the journal's entries. Keys come from the
//! player's bindings: I inventory, J journal, K skills (the original's
//! slots), Escape closes the focused screen.
//!
//! Rules: `docs/UI.md` (Forms, The window manager) and the `SG_GameGUI`
//! notes.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_combat::Condition;
use exa_ui::game_gui::{
    CONTAINER_TAG, CharacterScreenInput, GameScreen, Journal, JournalInput, OpenContainers,
    ScreenAction,
};
use exa_ui::gui_core::{FormHost, FormId, GuiRect, GuiTask, NO_CONTROL};
use exa_ui::{JournalAction, character_screen, inventory_place, journal_screen};

use crate::bindings::{Action, Bindings, virtual_key};
use crate::menu_screen::MenuScreen;
use crate::player::Player;

/// An in-game screen's kind, as [`exa_ui::GameScreen`].
pub type ScreenKind = GameScreen;

/// One open screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenScreen {
    pub kind: ScreenKind,
    pub form: FormId,
    pub tag: u32,
}

/// A screen's form: a rectangle with no controls yet, parented to the task's
/// root. Its close marks it doomed; the caller removes it.
pub struct ScreenForm {
    pub parent: Option<FormId>,
    pub bounds: GuiRect,
    pub visible: bool,
    pub tag: u32,
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
        self.visible
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

/// The open screens' state, the window manager that focuses them and the
/// session data behind the journal and the containers.
#[derive(Resource)]
pub struct GameScreens {
    pub task: GuiTask,
    pub open: Vec<OpenScreen>,
    pub journal: Journal,
    pub containers: OpenContainers,
    /// The journal's edited text, as the text box would hold it.
    pub journal_text: String,
    /// The player's entity, for the container tag.
    pub player: i32,
    /// Counters for the messages the original's task sends.
    pub opens: u32,
    pub closes: u32,
}

impl Default for GameScreens {
    fn default() -> Self {
        Self {
            task: GuiTask::new((800, 600)),
            open: Vec::new(),
            journal: Journal::default(),
            containers: OpenContainers::default(),
            journal_text: String::new(),
            player: 0,
            opens: 0,
            closes: 0,
        }
    }
}

impl GameScreens {
    /// The open screen of a kind.
    pub fn find(&self, kind: ScreenKind) -> Option<&OpenScreen> {
        self.open.iter().find(|screen| screen.kind == kind)
    }

    /// Whether a screen of the kind is open.
    pub fn is_open(&self, kind: ScreenKind) -> bool {
        self.find(kind).is_some()
    }

    /// The focused screen, if the focus is on one.
    pub fn focused(&self) -> Option<ScreenKind> {
        let active = self.task.active?;
        self.open
            .iter()
            .find(|screen| screen.form == active)
            .map(|screen| screen.kind)
    }

    /// Whether the focused screen is this one.
    pub fn is_focused(&self, kind: ScreenKind) -> bool {
        self.focused() == Some(kind)
    }

    /// Adds a screen's form to the task, focuses it and remembers it.
    fn push(&mut self, kind: ScreenKind, bounds: GuiRect) {
        let tag = kind.tag();
        let root = self.task.root;
        let form = self.task.add(Box::new(ScreenForm {
            parent: root,
            bounds,
            visible: true,
            tag,
            doomed: false,
            selected: 0,
        }));
        self.open.push(OpenScreen { kind, form, tag });
        self.task.focus_set(Some(form), NO_CONTROL);
        self.opens += 1;
    }

    /// Opens the inventory screen for a character at the given place.
    /// Returns what the show rule did.
    pub fn open_inventory(
        &mut self,
        input: CharacterScreenInput,
        screen_width: i32,
        unit: i32,
        place: (i32, i32),
    ) -> ScreenAction {
        let open = self.find(ScreenKind::Inventory).copied();
        let action = character_screen(CharacterScreenInput {
            view_open: open.is_some(),
            view_topmost: self.is_focused(ScreenKind::Inventory),
            ..input
        });
        match action {
            ScreenAction::Opened => {
                let (width, height) = ScreenKind::Inventory.size();
                let x = inventory_place(true, screen_width, width, unit).unwrap_or(place.0);
                self.containers.open(self.player | CONTAINER_TAG as i32);
                self.push(
                    ScreenKind::Inventory,
                    GuiRect::new(x, place.1, width, height),
                );
            }
            ScreenAction::Focused => {
                if let Some(screen) = open {
                    self.task.focus_set(Some(screen.form), NO_CONTROL);
                }
            }
            ScreenAction::Closed => {
                self.close(ScreenKind::Inventory);
            }
            ScreenAction::Refused => {}
        }
        action
    }

    /// Opens the journal: the show rule decides between focusing, closing and
    /// opening. The new form takes its remembered place (or 4, 36).
    pub fn open_journal(
        &mut self,
        input: JournalInput,
        place: Option<(i32, i32)>,
    ) -> JournalAction {
        let open = self.find(ScreenKind::Journal).copied();
        let action = journal_screen(JournalInput {
            open: open.is_some(),
            topmost: self.is_focused(ScreenKind::Journal),
            ..input
        });
        match action {
            JournalAction::Opened => {
                let (width, height) = ScreenKind::Journal.size();
                let (x, y) = place.unwrap_or((4, 36));
                self.push(ScreenKind::Journal, GuiRect::new(x, y, width, height));
                self.journal.ensure_entry();
                self.journal.select(0);
                self.journal_text = self.journal.entries[0].text.clone();
            }
            JournalAction::Focused => {
                if let Some(screen) = open {
                    self.task.focus_set(Some(screen.form), NO_CONTROL);
                }
            }
            JournalAction::Closed => {
                self.close(ScreenKind::Journal);
            }
            JournalAction::Refused => {}
        }
        action
    }

    /// Opens the skill selector at its control's place, or focuses it.
    pub fn open_skills(&mut self, ready: bool, place: GuiRect) -> bool {
        if !ready {
            return false;
        }
        if let Some(screen) = self.find(GameScreen::SkillSelector).copied() {
            self.task.focus_set(Some(screen.form), NO_CONTROL);
            return false;
        }
        self.push(GameScreen::SkillSelector, place);
        true
    }

    /// Closes a screen: its form is told to close, the task deletes it and
    /// the focus falls to the last remaining screen. Closing the journal
    /// stores its text and place; closing the inventory unregisters its
    /// container.
    pub fn close(&mut self, kind: ScreenKind) -> bool {
        let Some(index) = self.open.iter().position(|screen| screen.kind == kind) else {
            return false;
        };
        let screen = self.open.remove(index);
        let place = self.task.form(screen.form).map_or((4, 36), |form| {
            let b = form.bounds();
            (b.left, b.top)
        });
        if let Some(form) = self.task.form_mut(screen.form) {
            form.close(0);
        }
        self.task.delete_doomed();
        if kind == ScreenKind::Journal {
            self.journal.close(place, &self.journal_text);
        }
        if kind == ScreenKind::Inventory {
            self.containers.close(self.player | CONTAINER_TAG as i32);
        }
        if let Some(next) = self.open.last() {
            self.task.focus_set(Some(next.form), NO_CONTROL);
        } else {
            self.task.active = None;
            self.task.active_control = NO_CONTROL;
        }
        self.closes += 1;
        true
    }

    /// Closes the focused screen, if any. Escape's action.
    pub fn close_focused(&mut self) -> bool {
        self.focused().is_some_and(|kind| self.close(kind))
    }

    /// The windows that a power tree's close would also close (the container
    /// and journal screens share the 0xce1c0000 family).
    pub fn close_container_family(&mut self) -> usize {
        let kinds: Vec<ScreenKind> = self
            .open
            .iter()
            .filter(|screen| screen.tag & exa_ui::CONTAINER_MASK == exa_ui::CONTAINER_MASK)
            .map(|screen| screen.kind)
            .collect();
        let count = kinds.len();
        for kind in kinds {
            self.close(kind);
        }
        count
    }
}

pub struct GameScreensPlugin;

impl Plugin for GameScreensPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameScreens>().add_systems(
            Update,
            (sync_screen_size, screens_from_keys)
                .chain()
                .after(crate::player::cursor_ground)
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

/// Opens and closes the in-game screens from the player's bound keys: I
/// inventory, J journal, K skills; Escape closes the focused one. The
/// original's power tree and power bar need powers, which Exarust has none
/// of yet, so P and C do nothing.
fn screens_from_keys(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    players: Query<Entity, With<Player>>,
    conditions: Query<&Condition>,
    menu: Option<Res<MenuScreen>>,
    mut screens: ResMut<GameScreens>,
) {
    if menu.is_some_and(|menu| menu.open) {
        return;
    }
    let ready = !players.is_empty()
        && conditions
            .iter()
            .all(|condition| !condition.is_dead() && !condition.dying());
    // The container tag names the player's entity.
    if let Some(entity) = players.iter().next() {
        screens.player = entity.index().index() as i32;
    }
    let pressed: Vec<u32> = keys
        .get_just_pressed()
        .filter_map(|&key| virtual_key(key))
        .collect();
    if pressed.is_empty() {
        return;
    }
    let width = screens.task.screen.0;
    let unit = 4;
    for &key in &pressed {
        if key == 0x1b {
            if screens.close_focused() {
                debug!("GUI: the focused screen closes");
            }
            continue;
        }
        if key == bindings.key(Action::Inventory) && ready {
            let action = screens.open_inventory(
                CharacterScreenInput {
                    ready,
                    mode_ok: true,
                    view_open: false,
                    view_ready: true,
                    view_topmost: false,
                    allow_toggle: true,
                },
                width,
                unit,
                (4, 4),
            );
            debug!("GUI: inventory {action:?}");
        } else if key == bindings.key(Action::Journal) && ready {
            let action = screens.open_journal(
                JournalInput {
                    ready,
                    mode_ok: true,
                    open: false,
                    topmost: false,
                },
                None,
            );
            debug!("GUI: journal {action:?}");
        } else if key == bindings.key(Action::Skills) && ready {
            let opened = screens.open_skills(true, GuiRect::new(4, 4, 332, 112));
            debug!("GUI: skill selector opened: {opened}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> CharacterScreenInput {
        CharacterScreenInput {
            ready: true,
            mode_ok: true,
            view_open: false,
            view_ready: true,
            view_topmost: false,
            allow_toggle: true,
        }
    }

    #[test]
    fn the_inventory_opens_focuses_and_toggles() {
        let mut screens = GameScreens::default();
        assert_eq!(
            screens.open_inventory(ready(), 1600, 4, (4, 4)),
            ScreenAction::Opened
        );
        assert!(screens.is_open(ScreenKind::Inventory));
        assert!(screens.is_focused(ScreenKind::Inventory));
        assert!(screens.containers.is_open(CONTAINER_TAG as i32));
        assert_eq!(
            screens.open_inventory(ready(), 1600, 4, (4, 4)),
            ScreenAction::Closed
        );
        assert!(!screens.is_open(ScreenKind::Inventory));
        assert!(!screens.containers.is_open(CONTAINER_TAG as i32));
        assert_eq!(
            screens.open_inventory(
                CharacterScreenInput {
                    ready: false,
                    ..ready()
                },
                1600,
                4,
                (4, 4)
            ),
            ScreenAction::Refused
        );
    }

    #[test]
    fn the_journal_opens_and_remembers_its_place() {
        let mut screens = GameScreens::default();
        let input = JournalInput {
            ready: true,
            mode_ok: true,
            open: false,
            topmost: false,
        };
        assert_eq!(
            screens.open_journal(input, Some((10, 20))),
            JournalAction::Opened
        );
        assert!(screens.is_focused(ScreenKind::Journal));
        assert_eq!(screens.journal_text, "");
        screens.journal_text = "a note".to_string();
        assert_eq!(screens.open_journal(input, None), JournalAction::Closed);
        assert_eq!(screens.journal.place, (10, 20));
        assert_eq!(screens.journal.entries[0].text, "a note");
    }

    #[test]
    fn the_skill_selector_opens_once() {
        let mut screens = GameScreens::default();
        assert!(screens.open_skills(true, GuiRect::new(0, 0, 332, 112)));
        assert!(screens.is_open(GameScreen::SkillSelector));
        assert!(!screens.open_skills(true, GuiRect::new(0, 0, 332, 112)));
        assert!(!screens.open_skills(false, GuiRect::new(0, 0, 332, 112)));
        assert!(screens.close(GameScreen::SkillSelector));
        assert!(!screens.is_open(GameScreen::SkillSelector));
    }

    #[test]
    fn the_container_family_closes_together() {
        let mut screens = GameScreens::default();
        screens.open_journal(
            JournalInput {
                ready: true,
                mode_ok: true,
                open: false,
                topmost: false,
            },
            None,
        );
        screens.open_inventory(ready(), 1600, 4, (4, 4));
        assert_eq!(screens.close_container_family(), 2);
        assert!(screens.open.is_empty());
        assert_eq!(screens.closes, 2);
    }

    #[test]
    fn the_bound_keys_open_and_close_the_screens() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Bindings>()
            .init_resource::<GameScreens>()
            .add_systems(Update, screens_from_keys);
        app.world_mut().spawn((Player, Condition::NORMAL));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyI);
        app.update();
        assert!(
            app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Inventory)
        );
        assert!(
            app.world()
                .resource::<GameScreens>()
                .is_focused(GameScreen::Inventory)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset(KeyCode::KeyI);
        app.update();
        // The focused inventory toggles closed.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyI);
        app.update();
        assert!(
            !app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Inventory)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset(KeyCode::KeyI);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyJ);
        app.update();
        assert!(
            app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Journal)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset(KeyCode::KeyJ);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert!(
            !app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Journal)
        );
    }

    #[test]
    fn the_screen_forms_are_in_the_task() {
        let mut screens = GameScreens::default();
        screens.open_inventory(ready(), 1600, 4, (4, 4));
        let screen = screens.find(ScreenKind::Inventory).copied().unwrap();
        assert!(screens.task.form(screen.form).is_some());
        assert_eq!(screens.task.active, Some(screen.form));
        assert_eq!(screen.tag, GameScreen::Inventory.tag());
        assert_eq!(ScreenKind::Journal.size(), (244, 518));
    }
}
