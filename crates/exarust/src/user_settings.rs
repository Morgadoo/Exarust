//! The user's settings files, read at launch as the original reads them:
//! the text settings file (`<project>.ini`, created empty when missing) and
//! the binary settings store (`<project>.set`, whose key bindings the player
//! uses). On a first run the game's own save writes `FirstRun = 0`, and the
//! store is written back on exit. Rules: `docs/formats/SETTINGS.md`.
//!
//! The files live in Exarust's own folder (`EXARUST_USER_DIR`, else the
//! platform's configuration folder and `exarust`), never in the retail
//! game's, so the original's first-run setup and settings are left alone.
//! `--no-settings` skips them.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::window::{Monitor, PrimaryMonitor};
use exa_formats::file_ref::{directory_prefixes, read_whole_file, write_whole_file};
use exa_formats::settings_ini::{self, Assignment, REGISTRY, Settings, Value};
use exa_formats::settings_store::{BINDINGS_PATH, INTEGRITY_MESSAGE, SettingsStore};

use crate::Options;

/// What the settings files gave at launch.
#[derive(Resource, Clone, Debug)]
pub struct UserSettings {
    pub settings: Settings,
    pub assignments: Vec<Assignment>,
    pub store: SettingsStore,
    pub store_path: PathBuf,
}

impl UserSettings {
    /// The last boolean the file gave the named setting.
    pub fn last_bool(&self, name: &str) -> Option<bool> {
        self.last(name).and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None,
        })
    }

    fn last(&self, name: &str) -> Option<&Value> {
        self.assignments
            .iter()
            .rev()
            .find(|a| REGISTRY[a.index].name == name)
            .map(|a| &a.value)
    }

    fn sets(&self, name: &str) -> bool {
        settings_ini::sets(&self.assignments, name)
    }

    /// Applies the settings Exarust has to the launch options. A setting the
    /// file does not name, or one given on the command line, keeps the
    /// launcher's choice.
    pub fn apply_to(&self, options: &mut Options) {
        let s = &self.settings;
        // As in the original, full screen covers the display and the window
        // size is for a window; a size on the command line means a window.
        if options.size.is_none() {
            if self.sets("Fullscreen") && s.fullscreen {
                options.fullscreen = true;
            } else if (self.sets("WindowWidth") || self.sets("WindowHeight"))
                && s.window_width > 0
                && s.window_height > 0
            {
                options.size = Some((s.window_width as u32, s.window_height as u32));
            }
        }
        if self.sets("VerticalSync") && !s.vertical_sync {
            options.no_vsync = true;
        }
        // Exarust has only the original's 4× level; any level keeps it on.
        if self.sets("Supersample") && s.supersample < 1 {
            options.no_supersample = true;
        }
        if self.last_bool("ShowFPS") == Some(false) {
            options.no_fps = true;
        }
    }

    /// The player's key bindings, when the store holds a set.
    pub fn bindings(&self) -> Option<exa_game::bindings::Bindings> {
        self.store
            .get(&BINDINGS_PATH)
            .and_then(exa_game::bindings::Bindings::decode)
    }

    /// Writes the store back, as the original does on exit (an empty store
    /// is not written).
    pub fn save_store(&self) {
        if let Some(bytes) = self.store.save()
            && !write_whole_file(&self.store_path, bytes)
        {
            warn!("cannot write {}", self.store_path.display());
        }
    }
}

/// Exarust's user folder.
pub fn folder() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("EXARUST_USER_DIR") {
        return Some(PathBuf::from(dir));
    }
    if cfg!(windows) {
        return std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("Exarust"));
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(dir).join("exarust"));
    }
    std::env::var_os("HOME").map(|d| PathBuf::from(d).join(".config").join("exarust"))
}

/// Creates every folder of `path` the way the original does.
fn create_folders(path: &Path) {
    let text = path.to_string_lossy();
    let sep = std::path::MAIN_SEPARATOR as u8;
    for prefix in directory_prefixes(text.as_bytes(), sep) {
        let _ = std::fs::create_dir(String::from_utf8_lossy(&prefix).as_ref());
    }
}

/// Reads the settings files for `project` (the project file's name). The
/// GUI size waits for the display ([`gui_size_from_display`]).
pub fn load(project: &str) -> Option<UserSettings> {
    let folder = folder()?;
    create_folders(&folder);
    let ini = settings_ini::settings_file(&folder, project);
    let text = settings_ini::open_settings_file(&ini);
    let (settings, assignments) = Settings::read(&text, 0);
    let user = UserSettings {
        settings,
        assignments,
        store: SettingsStore::new(),
        store_path: settings_ini::store_file(&folder, project),
    };
    // The game's first run: its one-setting save records that it happened.
    if user.last_bool("FirstRun").unwrap_or(true) {
        settings_ini::save_one_setting_to_file(&ini, "FirstRun", settings_ini::format_bool(false));
    }
    let loaded = SettingsStore::load(read_whole_file(&user.store_path).unwrap_or_default());
    if loaded.integrity_failed {
        warn!("{INTEGRITY_MESSAGE}");
    }
    Some(UserSettings {
        store: loaded.store,
        ..user
    })
}

/// Sets the GUI size once the primary display is known: the file's
/// `GUISize` lines applied with the display's height.
pub fn gui_size_from_display(
    user: Option<Res<UserSettings>>,
    monitors: Query<&Monitor, With<PrimaryMonitor>>,
    gui: Option<ResMut<exa_ui::GuiScale>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let (Some(user), Some(mut gui)) = (user, gui) else {
        *done = true;
        return;
    };
    let Ok(monitor) = monitors.single() else {
        return;
    };
    *done = true;
    if !user.sets("GUISize") {
        return;
    }
    let mut s = Settings::default();
    for a in &user.assignments {
        if REGISTRY[a.index].name == "GUISize" {
            s.apply(a, monitor.physical_height);
        }
    }
    gui.size = s.gui_size;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(text: &[u8]) -> UserSettings {
        let (settings, assignments) = Settings::read(text, 0);
        UserSettings {
            settings,
            assignments,
            store: SettingsStore::new(),
            store_path: PathBuf::new(),
        }
    }

    #[test]
    fn the_file_sets_only_what_it_names() {
        let mut options = Options::default();
        user(b"").apply_to(&mut options);
        assert_eq!(options.size, None);
        assert!(!options.fullscreen && !options.no_vsync && !options.no_supersample);
        let mut options = Options::default();
        user(b"WindowWidth = 1280\r\nWindowHeight = 720\r\nVerticalSync = 0\r\nSupersample = 0\r\nShowFPS = 0\r\n")
            .apply_to(&mut options);
        assert_eq!(options.size, Some((1280, 720)));
        assert!(options.no_vsync && options.no_supersample && options.no_fps);
        // Full screen ignores the file's window size.
        let mut options = Options::default();
        user(b"Fullscreen = 1\r\nWindowWidth = 800\r\n").apply_to(&mut options);
        assert!(options.fullscreen);
        assert_eq!(options.size, None);
        // One dimension keeps the other's default (1080 tall).
        let mut options = Options::default();
        user(b"Fullscreen = 0\r\nWindowWidth = 800\r\n").apply_to(&mut options);
        assert!(!options.fullscreen);
        assert_eq!(options.size, Some((800, 1080)));
        // The command line's size wins.
        let mut options = Options {
            size: Some((640, 480)),
            ..default()
        };
        user(b"WindowWidth = 1280\r\nWindowHeight = 720\r\n").apply_to(&mut options);
        assert_eq!(options.size, Some((640, 480)));
    }

    #[test]
    fn the_store_gives_the_bindings() {
        let mut u = user(b"");
        assert_eq!(u.bindings(), None);
        let mut b = exa_game::bindings::Bindings::default();
        b.controller = true;
        u.store.set(&BINDINGS_PATH, &b.encode()).unwrap();
        assert_eq!(u.bindings(), Some(b));
    }
}
