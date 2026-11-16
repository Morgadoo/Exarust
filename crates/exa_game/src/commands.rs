//! The named debug-command registry and the key bindings that run it.
//!
//! The original's units register a command name and a handler (`cshd` from
//! the shader programs, `msm` from the mesh subdivider). A line is split at
//! its first space into the command and the rest; the command is looked up
//! with ASCII case ignored and its handler gets the rest. The settings
//! unit's key bindings (`KeyDef` lines) hold lists of such lines. Running
//! them one per frame is a guess: the code that advances a binding's list
//! was not part of the evidence for the registry itself.
//!
//! The first six bound `KeyDef` lines are shortcuts of the root task, with
//! the game boot's Escape for the menu toggle after them. They fire from
//! the root task's key slot ([`root_task_keys`]) for the keys that climb
//! to it ([`crate::gui_keys::TaskKeys`]): a form's base key slot passing a
//! key up, or the game view after the player's hotkeys ([`view_keys`]).
//! Only the best shortcut for a key runs ([`TaskLayer::key`]).
//!
//! Rules and evidence: `docs/formats/SETTINGS.md` (`KeyDef`) and
//! `docs/DEBUGGING.md`.

use std::collections::VecDeque;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use exa_formats::settings_ini::Hotkey;
use exa_ui::gui_task::{KeyOutcome, TaskLayer};

use crate::gui_keys::{GuiKeyFrame, TaskKeys};

/// A command handler: it receives the bytes after the command's first space
/// (empty when the line has no space).
pub type CommandHandler = Box<dyn Fn(&[u8]) + Send + Sync>;

/// The registered commands, in registration order. A later registration of
/// the same name is another entry; the first match wins, as the original's
/// linear scan does.
#[derive(Resource, Default)]
pub struct CommandRegistry {
    entries: Vec<(Vec<u8>, CommandHandler)>,
}

impl CommandRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `name` (stored ASCII-lower-cased) and its handler; the
    /// original appends the pair to its array.
    pub fn register(&mut self, name: &str, handler: impl Fn(&[u8]) + Send + Sync + 'static) {
        self.entries
            .push((name.to_ascii_lowercase().into_bytes(), Box::new(handler)));
    }

    /// The registered names, as registered.
    pub fn names(&self) -> impl Iterator<Item = &[u8]> {
        self.entries.iter().map(|(name, _)| name.as_slice())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Runs a line as the original does. The first space splits it: the
    /// bytes before it are the command (the whole line when there is no
    /// space or it starts with one), and the argument is what follows the
    /// space, so a run of spaces keeps the ones after the first. The
    /// command is compared with ASCII case ignored; the first registered
    /// match's handler runs and gives `true`. A line without a match runs
    /// nothing and gives `false`.
    pub fn execute(&self, line: &[u8]) -> bool {
        let split = line.iter().position(|&b| b == b' ');
        let (command, argument) = match split {
            Some(0) | None => (line, &[][..]),
            Some(i) => (&line[..i], &line[(i + 1).min(line.len())..]),
        };
        let command = command.to_ascii_lowercase();
        for (name, handler) in &self.entries {
            if *name == command {
                handler(argument);
                return true;
            }
        }
        false
    }

    /// [`CommandRegistry::execute`] for a text line.
    pub fn execute_str(&self, line: &str) -> bool {
        self.execute(line.as_bytes())
    }
}

/// Lines waiting to run, oldest first. One line runs per frame (a guess,
/// see the module notes).
#[derive(Resource, Default)]
pub struct CommandQueue {
    pending: VecDeque<Vec<u8>>,
}

impl CommandQueue {
    /// Queues one line.
    pub fn push(&mut self, line: impl Into<Vec<u8>>) {
        self.pending.push_back(line.into());
    }

    /// Queues a binding's lines in order.
    pub fn push_all(&mut self, lines: impl IntoIterator<Item = Vec<u8>>) {
        self.pending.extend(lines);
    }

    /// The lines still to run.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Takes the next line.
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        self.pending.pop_front()
    }
}

/// One key binding's commands, from a `KeyDef` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCommand {
    /// The modifier bits (Shift 0x20, Ctrl 0x40, Alt 0x80), or 0 for none.
    pub modifier: u8,
    /// The saved virtual-key code.
    pub key: u32,
    /// The lines, in order.
    pub commands: Vec<Vec<u8>>,
}

impl KeyCommand {
    /// The binding a parsed `KeyDef` gives, when it is one of the first six
    /// the original binds to a handler.
    pub fn of(hotkey: &Hotkey) -> Option<Self> {
        hotkey.bound.then(|| Self {
            modifier: hotkey.modifier,
            key: u32::from(hotkey.key),
            commands: hotkey.commands.clone(),
        })
    }
}

/// The bindings the settings file's `KeyDef` lines gave.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct KeyCommands {
    pub bindings: Vec<KeyCommand>,
}

impl KeyCommands {
    /// The bound hotkeys, in file order.
    pub fn of_hotkeys(hotkeys: &[Hotkey]) -> Self {
        Self {
            bindings: hotkeys.iter().filter_map(KeyCommand::of).collect(),
        }
    }
}

/// The menu toggle's key: Escape with no modifier, which the game's boot
/// adds to the root task after the settings file's `KeyDef` lines.
pub const MENU_TOGGLE_KEY: u8 = 0x1b;

/// What one of the root task's shortcuts runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootAction {
    /// A `KeyDef` binding's lines (its index in [`KeyCommands`]).
    Binding(usize),
    /// The main menu's toggle.
    MenuToggle,
}

/// The root task's shortcut table, in the order the original adds them:
/// the bound `KeyDef` lines in file order (read with the settings), then
/// Escape for the menu toggle (the game's boot). Each entry's action word
/// is its index in the returned actions.
pub fn root_task(bindings: Option<&KeyCommands>) -> (TaskLayer, Vec<RootAction>) {
    let mut layer = TaskLayer::new(0, None);
    layer.root = true;
    let mut actions = Vec::new();
    for (index, binding) in bindings.iter().flat_map(|b| b.bindings.iter()).enumerate() {
        let Ok(key) = u8::try_from(binding.key) else {
            continue;
        };
        layer.add_shortcut(key, binding.modifier, actions.len() as u32);
        actions.push(RootAction::Binding(index));
    }
    layer.add_shortcut(MENU_TOGGLE_KEY, 0, actions.len() as u32);
    actions.push(RootAction::MenuToggle);
    (layer, actions)
}

/// The root task's menu toggle fired (Escape reached the root's key slot
/// and no better shortcut took it).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuToggleKey;

/// The root task's key slot for every key that climbed to the task this
/// frame ([`TaskKeys`]): a repeat or a typed character does nothing; any
/// other key but Shift runs the shortcut whose required modifiers are all
/// held and most numerous (the earlier entry wins a tie), with the
/// modifiers the key-down's word holds; the root forwards nothing. A
/// binding queues its lines; the menu toggle is [`MenuToggleKey`].
pub fn root_task_keys(
    climbed: Option<ResMut<TaskKeys>>,
    bindings: Option<Res<KeyCommands>>,
    mut queue: ResMut<CommandQueue>,
    mut toggles: MessageWriter<MenuToggleKey>,
) {
    let Some(mut climbed) = climbed else {
        return;
    };
    if climbed.keys.is_empty() {
        return;
    }
    let keys = std::mem::take(&mut climbed.keys);
    let (layer, actions) = root_task(bindings.as_deref());
    for key in keys {
        // No entry holds a key above a byte.
        let Ok(byte) = u8::try_from(key.key) else {
            continue;
        };
        let held = (key.flags & 0xe0) as u8;
        let KeyOutcome::Shortcut { action, .. } = layer.key(byte, held, key.flags) else {
            continue;
        };
        match actions[action as usize] {
            RootAction::Binding(index) => {
                if let Some(binding) = bindings.as_deref().and_then(|b| b.bindings.get(index)) {
                    debug!(
                        "keys: {:#x} (word {:#x}) runs binding {index}",
                        key.key, key.flags
                    );
                    queue.push_all(binding.commands.iter().cloned());
                }
            }
            RootAction::MenuToggle => {
                debug!("keys: Escape reaches the root: the menu toggle");
                toggles.write(MenuToggleKey);
            }
        }
    }
}

/// Which form has the key focus besides the game view: a key reaches the
/// view only while none of these takes the keys (the window manager's
/// focus follows the pointer, and these forms cover the screen or hold the
/// pointer).
#[derive(SystemParam)]
pub struct ViewFocus<'w> {
    menu: Option<Res<'w, crate::menu_screen::MenuScreen>>,
    end_menu: Option<Res<'w, crate::end_menu_screen::EndMenuScreen>>,
    settings: Option<Res<'w, crate::settings_form::SettingsForm>>,
    manual: Option<Res<'w, crate::help_live::ManualScreen>>,
    creator: Option<Res<'w, crate::creator_screen::CreatorScreen>>,
    power: Option<Res<'w, crate::power_screen::PowerScreen>>,
    skills: Option<Res<'w, crate::skill_screen::SkillUi>>,
    destination: Option<Res<'w, crate::destination::PortalScreenState>>,
    hub: Option<Res<'w, crate::hub_windows::HubWindows>>,
    screens: Option<Res<'w, crate::gui_screens::GameScreens>>,
}

impl ViewFocus<'_> {
    /// Whether the game view has the key focus.
    pub fn view(&self) -> bool {
        !(self.menu.as_deref().is_some_and(|m| m.open)
            || self.end_menu.as_deref().is_some_and(|m| m.open)
            || self.settings.as_deref().is_some_and(|s| s.is_open())
            || self.manual.as_deref().is_some_and(|m| m.is_open())
            || self.creator.as_deref().is_some_and(|c| c.is_open())
            || self.power.as_deref().is_some_and(|p| p.open)
            || self.skills.as_deref().is_some_and(|s| s.screen.is_some())
            || self
                .destination
                .as_deref()
                .is_some_and(|d| d.open.is_some())
            || self.hub.as_deref().is_some_and(|h| h.is_open())
            || self
                .screens
                .as_deref()
                .is_some_and(|s| s.keys_to_window() || s.journal_box_focused()))
    }

    /// Whether Escape is Exarust's close of an open game screen
    /// ([`crate::gui_screens::GameScreens::close_focused`]), which takes it.
    pub fn escape_taken(&self) -> bool {
        self.screens.as_deref().is_some_and(|s| !s.open.is_empty())
    }
}

/// The game view's key slot after its key handler (the player's hotkeys,
/// [`crate::hotkeys::HotkeySet`]): with the view focused, each key-down
/// goes on to the parent's key slot unless its word has a low bit (a
/// repeat or a typed character), up to the root task.
pub fn view_keys(
    frame: Option<Res<GuiKeyFrame>>,
    focus: ViewFocus,
    climbed: Option<ResMut<TaskKeys>>,
) {
    let (Some(frame), Some(mut climbed)) = (frame, climbed) else {
        return;
    };
    if frame.keys.is_empty() || !focus.view() {
        return;
    }
    let escape_taken = focus.escape_taken();
    for key in frame.keys.iter().filter(|k| k.flags & 3 == 0) {
        if escape_taken && key.key == u32::from(MENU_TOGGLE_KEY) {
            continue;
        }
        climbed.forward(*key);
    }
}

/// The root task's key slot runs after every form's and the view's.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RootKeySet;

/// Runs one queued line each frame.
pub fn run_queued_commands(registry: Res<CommandRegistry>, mut queue: ResMut<CommandQueue>) {
    if let Some(line) = queue.pop() {
        registry.execute(&line);
    }
}

/// Installs the registry and the queue, and the two systems above.
pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        crate::gui_keys::ensure(app);
        app.init_resource::<CommandRegistry>()
            .init_resource::<CommandQueue>()
            .add_message::<MenuToggleKey>()
            .add_systems(
                Update,
                (
                    run_queued_commands.in_set(exa_physics::FramePhase::Input),
                    view_keys
                        .after(crate::destination::screen_input)
                        .in_set(exa_physics::FramePhase::Gui)
                        .after(crate::gui_screens::rescale_open_screens)
                        .after(crate::skill_screen::open_skills)
                        .after(crate::hotkeys::HotkeySet),
                ),
            )
            .add_systems(
                PostUpdate,
                root_task_keys
                    .in_set(RootKeySet)
                    .in_set(exa_physics::FramePostPhase::Prepare),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn registry_with(log: Arc<Mutex<Vec<Vec<u8>>>>) -> CommandRegistry {
        let mut registry = CommandRegistry::new();
        let l = log.clone();
        registry.register("cshd", move |arg| l.lock().unwrap().push(arg.to_vec()));
        let l = log.clone();
        registry.register("msm", move |arg| l.lock().unwrap().push(arg.to_vec()));
        registry
    }

    #[test]
    fn a_line_splits_at_the_first_space_and_matches_without_case() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let registry = registry_with(log.clone());
        assert!(registry.execute_str("cshd"));
        assert!(registry.execute_str("CSHD"));
        assert!(registry.execute_str("Cshd arg one"));
        // The argument starts one byte after the first space.
        assert!(registry.execute_str("msm  two"));
        assert!(!registry.execute_str("nope arg"));
        assert!(!registry.execute_str(""));
        // A leading space is no command.
        assert!(!registry.execute_str(" cshd"));
        assert_eq!(
            *log.lock().unwrap(),
            [
                b"".to_vec(),
                b"".to_vec(),
                b"arg one".to_vec(),
                b" two".to_vec()
            ]
        );
    }

    #[test]
    fn the_first_registration_wins_and_names_are_reported() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut registry = registry_with(log.clone());
        registry.register("cshd", |_| panic!("second cshd ran"));
        assert_eq!(registry.len(), 3);
        assert_eq!(
            registry.names().collect::<Vec<_>>(),
            [b"cshd".as_slice(), b"msm", b"cshd"]
        );
        assert!(registry.execute_str("cshd"));
        assert_eq!(log.lock().unwrap().len(), 1);
    }

    use crate::gui_keys::{GuiKeysPlugin, TaskKeys};
    use exa_formats::settings_ini::{CTRL, SHIFT};
    use exa_ui::gui_keys::GuiKey;

    fn root_app(defs: &[&[u8]]) -> App {
        let mut hotkeys = Vec::new();
        for def in defs {
            exa_formats::settings_ini::parse_key_def(def, &mut hotkeys);
        }
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(GuiKeysPlugin)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(KeyCommands::of_hotkeys(&hotkeys))
            .init_resource::<CommandQueue>()
            .add_message::<MenuToggleKey>()
            .add_systems(PostUpdate, root_task_keys);
        app
    }

    fn climb(app: &mut App, key: u32, flags: u32) {
        app.world_mut()
            .resource_mut::<TaskKeys>()
            .forward(GuiKey { key, flags });
    }

    fn queued(app: &mut App) -> Vec<Vec<u8>> {
        let mut queue = app.world_mut().resource_mut::<CommandQueue>();
        std::iter::from_fn(|| queue.pop()).collect()
    }

    #[test]
    fn key_defs_become_bindings_and_bindings_queue_their_lines() {
        let mut hotkeys = Vec::new();
        exa_formats::settings_ini::parse_key_def(b"shift+S: cshd; msm arg", &mut hotkeys);
        exa_formats::settings_ini::parse_key_def(b"ctrl+F1: nope", &mut hotkeys);
        let bindings = KeyCommands::of_hotkeys(&hotkeys);
        assert_eq!(bindings.bindings.len(), 2);
        assert_eq!(bindings.bindings[0].modifier, SHIFT);
        assert_eq!(bindings.bindings[0].key, 0x53u32);
        assert_eq!(
            bindings.bindings[0].commands,
            [b"cshd".to_vec(), b"msm arg".to_vec()]
        );
        let mut app = root_app(&[b"shift+S: cshd; msm arg", b"ctrl+F1: nope"]);
        // Shift with S: the key-down's word has Shift (0x20).
        climb(&mut app, 0x53, 0x20);
        app.update();
        assert_eq!(queued(&mut app), [b"cshd".to_vec(), b"msm arg".to_vec()]);
        // Without Shift the binding does not fire.
        climb(&mut app, 0x53, 0);
        app.update();
        assert!(queued(&mut app).is_empty());
    }

    /// The root's key slot runs only the best shortcut for a key: the most
    /// required modifiers held, the earlier entry on a tie; a repeat, a
    /// typed character and Shift alone run nothing.
    #[test]
    fn the_root_runs_only_the_best_shortcut_and_ignores_repeats() {
        let mut app = root_app(&[b"shift+S: shifted", b"ctrl+S: ctrl", b"ctrl+S: second"]);
        assert_eq!(
            root_task(app.world().get_resource::<KeyCommands>()).1.len(),
            4
        );
        // Ctrl with S: both Ctrl entries score 2, the first runs.
        climb(&mut app, 0x53, u32::from(CTRL));
        app.update();
        assert_eq!(queued(&mut app), [b"ctrl".to_vec()]);
        // Ctrl and Shift held: all three score 2, the earliest runs (the
        // old dispatch fired every binding whose modifiers were held).
        climb(&mut app, 0x53, 0x60);
        app.update();
        assert_eq!(queued(&mut app), [b"shifted".to_vec()]);
        // No modifier: none qualifies.
        climb(&mut app, 0x53, 0);
        app.update();
        assert!(queued(&mut app).is_empty());
        // A repeat (1), a typed character (2) and Shift itself do nothing.
        climb(&mut app, 0x53, 0x41);
        climb(&mut app, 0x53, 2);
        climb(&mut app, 0x10, 0x20);
        app.update();
        assert!(queued(&mut app).is_empty());
    }

    /// Escape is the root's menu toggle, with or without modifiers held
    /// (it requires none); `KeyDef` lines always name a modifier and one
    /// character, so none can take Escape.
    #[test]
    fn escape_is_the_root_tasks_menu_toggle() {
        let mut app = root_app(&[b"ctrl+S: x"]);
        climb(&mut app, 0x1b, 0);
        climb(&mut app, 0x1b, 0x40);
        climb(&mut app, 0x1b, 1);
        app.update();
        assert_eq!(app.world().resource::<Messages<MenuToggleKey>>().len(), 2);
        assert!(queued(&mut app).is_empty());
    }

    /// The game view's key slot hands every key-down on to the root task
    /// after the player's hotkeys: Ctrl with S runs only the first of two
    /// Ctrl+S bindings (the old dispatch ran both); with the main menu
    /// open the view does not have the keys.
    #[test]
    fn a_key_on_the_view_reaches_the_roots_best_shortcut() {
        let mut app = App::new();
        let mut hotkeys = Vec::new();
        exa_formats::settings_ini::parse_key_def(b"ctrl+S: a; b", &mut hotkeys);
        exa_formats::settings_ini::parse_key_def(b"ctrl+S: c", &mut hotkeys);
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(KeyCommands::of_hotkeys(&hotkeys))
            .add_plugins(CommandPlugin);
        let press = |app: &mut App, key: KeyCode| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
        };
        press(&mut app, KeyCode::ControlLeft);
        press(&mut app, KeyCode::KeyS);
        // The root queued the first binding's two lines; the frame's
        // command step has not run yet.
        assert_eq!(app.world().resource::<CommandQueue>().len(), 2);
        app.world_mut()
            .resource_mut::<CommandQueue>()
            .pending
            .clear();
        // The main menu open: the view has no key focus.
        let mut menu = crate::menu_screen::MenuScreen::default();
        menu.open = true;
        app.insert_resource(menu);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyS);
        press(&mut app, KeyCode::KeyS);
        assert!(app.world().resource::<CommandQueue>().is_empty());
    }
}
