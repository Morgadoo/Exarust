//! The named debug-command registry and the key bindings that run it.
//!
//! The original's units register a command name and a handler (`cshd` from
//! the shader programs, `msm` from the mesh subdivider). A line is split at
//! its first space into the command and the rest; the command is looked up
//! with ASCII case ignored and its handler gets the rest. The settings
//! unit's key bindings (`KeyDef` lines) hold lists of such lines, which run
//! one after another, one per frame.
//!
//! Rules and evidence: `docs/formats/SETTINGS.md` (`KeyDef`) and
//! `docs/DEBUGGING.md`.

use std::collections::VecDeque;

use bevy::input::ButtonInput;
use bevy::prelude::*;
use exa_formats::settings_ini::{ALT, CTRL, Hotkey, SHIFT};

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

/// Lines waiting to run, oldest first. One line runs per frame, as the
/// original's binding list advances with the frame's step.
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
    /// The modifier bits ([`SHIFT`], [`CTRL`], [`ALT`]), or 0 for none.
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

    /// Whether the modifier this binding asks for is held now.
    fn modifier_held(&self, keys: &ButtonInput<KeyCode>) -> bool {
        let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let alt = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
        (self.modifier & SHIFT == 0 || shift)
            && (self.modifier & CTRL == 0 || ctrl)
            && (self.modifier & ALT == 0 || alt)
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

/// A key binding's lines enter the queue when its key goes down with its
/// modifier held.
pub fn queue_binding_lines(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Option<Res<KeyCommands>>,
    mut queue: ResMut<CommandQueue>,
) {
    let Some(bindings) = bindings else {
        return;
    };
    for binding in &bindings.bindings {
        let Some(code) = crate::bindings::key_code(binding.key) else {
            continue;
        };
        if keys.just_pressed(code) && binding.modifier_held(&keys) {
            queue.push_all(binding.commands.iter().cloned());
        }
    }
}

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
        app.init_resource::<CommandRegistry>()
            .init_resource::<CommandQueue>()
            .add_systems(Update, (queue_binding_lines, run_queued_commands).chain());
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
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bindings)
            .init_resource::<CommandQueue>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, queue_binding_lines);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyS);
        app.update();
        let queue = app.world().resource::<CommandQueue>();
        assert_eq!(queue.len(), 2);
        assert!(!queue.is_empty());
    }
}
