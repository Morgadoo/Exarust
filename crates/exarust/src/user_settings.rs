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
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::window::{Monitor, PrimaryMonitor};
use exa_formats::file_ref::{directory_prefixes, read_whole_file};
use exa_formats::settings_ini::{self, Assignment, REGISTRY, Settings, Value};
use exa_formats::settings_store::{BINDINGS_PATH, INTEGRITY_MESSAGE, SettingsStore};

use crate::Options;

/// What the settings files gave at launch.
#[derive(Resource, Clone, Debug)]
pub struct UserSettings {
    pub settings: Settings,
    pub assignments: Vec<Assignment>,
    pub store: SettingsStore,
    /// The text settings file Apply rewrites.
    pub ini_path: PathBuf,
    pub store_path: PathBuf,
    /// The store as the Settings form's Apply last saved it, shared with
    /// the launcher's copy so that the exit save writes it back.
    pub applied: AppliedStore,
}

/// The store Apply last saved (shared between the copies of
/// [`UserSettings`]).
#[derive(Clone, Debug, Default)]
pub struct AppliedStore(Arc<Mutex<Option<SettingsStore>>>);

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
        // The original's frame-rate cap; a file that does not name it leaves
        // Exarust's cap off (the original's default is automatic).
        if options.max_fps.is_none() && self.sets("FPSLimit") {
            options.max_fps = Some(s.fps_cap);
        }
    }

    /// The hardware configuration the settings file gives, with the launch
    /// options' overrides: the launcher's full screen, size, vertical sync,
    /// super-sampling and frame-rate choices (already merged with the file by
    /// [`UserSettings::apply_to`]) win. Exarust draws its one super-sampling
    /// level unless it is turned off, so a file that does not name
    /// `Supersample` gives the live level 4 rather than the original's
    /// pre-detection 0.
    pub fn hardware(
        &self,
        fullscreen: bool,
        size: Option<(u32, u32)>,
        no_vsync: bool,
        no_supersample: bool,
        no_fps: bool,
    ) -> exa_game::hw_config::HardwareConfig {
        let mut live = exa_formats::hw_config::options_from_settings(&self.settings);
        live.window_mode = !fullscreen;
        live.vertical_sync = !no_vsync;
        if no_supersample {
            live.supersample = 0;
        } else if live.supersample < 1 {
            live.supersample = 4;
        }
        live.show_fps = !no_fps;
        exa_game::hw_config::HardwareConfig {
            window_size: size,
            screen_size: (self.settings.screen_width, self.settings.screen_height),
            steam_deck: exa_game::hw_config::steam_deck(),
            ..exa_game::hw_config::HardwareConfig::new(live).with_file(self.ini_path.clone())
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
        if !self.store.save_to(&self.store_path) {
            warn!("cannot write {}", self.store_path.display());
        }
    }

    /// The exit's save: the store Apply last saved when there is one (the
    /// launcher's copy predates it), else the store as launched.
    pub fn save_store_at_exit(&self) {
        let applied = self.applied.0.lock().ok().and_then(|saved| saved.clone());
        match applied {
            Some(store) => UserSettings {
                store,
                ..self.clone()
            }
            .save_store(),
            None => self.save_store(),
        }
    }

    /// Puts `bindings` into the store and saves it, as the original's Apply
    /// saves its binding file at once.
    pub fn save_bindings(&mut self, bindings: &exa_game::bindings::Bindings) {
        if self.store.set(&BINDINGS_PATH, &bindings.encode()).is_err() {
            warn!("cannot store the key bindings");
            return;
        }
        self.save_store();
        if let Ok(mut saved) = self.applied.0.lock() {
            *saved = Some(self.store.clone());
        }
    }

    /// Writes one 4-byte settings entry and saves the store, as the original's
    /// settings writer flushes after an unlock word. The saved copy is kept
    /// for the exit save.
    pub fn save_entry(&mut self, path: &[u32], bytes: &[u8]) {
        if self.store.set(path, bytes).is_err() {
            warn!("cannot store the settings entry {path:?}");
            return;
        }
        self.save_store();
        if let Ok(mut saved) = self.applied.0.lock() {
            *saved = Some(self.store.clone());
        }
    }
}

/// Writes the flow's settings entries (the expert-ladder and story-background
/// unlock words) into the store as they arrive, one flush each.
pub fn write_flow_settings(
    mut entries: MessageReader<exa_game::flow::SettingsEntry>,
    mut user: Option<ResMut<UserSettings>>,
) {
    let Some(user) = user.as_mut() else {
        entries.clear();
        return;
    };
    for entry in entries.read() {
        user.save_entry(&[entry.section, entry.key], &entry.bytes);
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
        ini_path: ini.clone(),
        store_path: settings_ini::store_file(&folder, project),
        applied: AppliedStore::default(),
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

/// Saves the bindings the Settings form's Apply committed.
pub fn save_applied_bindings(
    mut applied: MessageReader<exa_game::hw_config::BindingsApplied>,
    user: Option<ResMut<UserSettings>>,
) {
    let Some(exa_game::hw_config::BindingsApplied(bindings)) = applied.read().last() else {
        return;
    };
    if let Some(mut user) = user {
        user.save_bindings(bindings);
    }
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
            ini_path: PathBuf::new(),
            store_path: PathBuf::new(),
            applied: AppliedStore::default(),
        }
    }

    #[test]
    fn applied_bindings_survive_the_exit_save() {
        use exa_game::bindings::{Action, Bindings};
        let dir = std::env::temp_dir().join(format!("exarust-applied-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut launched = user(b"");
        launched.store_path = dir.join("bindings.set");
        let mut running = launched.clone();
        let mut bindings = Bindings::default();
        bindings.set_key(Action::Forward, 0x26);
        running.save_bindings(&bindings);
        // The launcher's copy never saw the change; its exit save keeps it.
        launched.save_store_at_exit();
        let loaded = SettingsStore::load(read_whole_file(&launched.store_path).unwrap());
        let back = loaded
            .store
            .get(&BINDINGS_PATH)
            .and_then(Bindings::decode)
            .unwrap();
        assert_eq!(back.key(Action::Forward), 0x26);
        let _ = std::fs::remove_dir_all(&dir);
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
        // The file's FPSLimit gives the cap (60 is 16 ms, 62 frames), unless
        // the command line gave one.
        let mut options = Options::default();
        user(b"").apply_to(&mut options);
        assert_eq!(options.max_fps, None);
        user(b"FPSLimit = 60\r\n").apply_to(&mut options);
        assert_eq!(options.max_fps, Some(62));
        let mut options = Options {
            max_fps: Some(-1),
            ..default()
        };
        user(b"FPSLimit = 60\r\n").apply_to(&mut options);
        assert_eq!(options.max_fps, Some(-1));
    }

    #[test]
    fn the_hardware_options_take_the_file_and_the_launch_overrides() {
        let u = user(
            b"Volume = 50\r\nMusic = 0\r\nSupersample = 3\r\nShadowQuality = 2\r\n\
              Fullscreen = 0\r\nVerticalSync = 1\r\nShowFPS = 0\r\n\
              InputMode = 51\r\nAimSpeed = 30\r\n",
        );
        let hardware = u.hardware(false, Some((1280, 720)), false, false, false);
        assert_eq!(hardware.live.volume, 0.5);
        assert!(!hardware.live.music);
        assert_eq!(hardware.live.supersample, 3);
        assert_eq!(hardware.live.shadow_quality, 2);
        assert!(hardware.live.window_mode);
        assert!(hardware.live.vertical_sync);
        assert!(hardware.live.show_fps);
        assert!(hardware.live.controller);
        assert_eq!(hardware.live.aim_speed, (29.0f64 / 99.0) as f32);
        assert_eq!(hardware.window_size, Some((1280, 720)));
        assert_eq!(hardware.screen_size, (0, 0));
        assert_eq!(
            user(b"ScreenWidth = 1920\r\nScreenHeight = 1080\r\n")
                .hardware(true, None, false, false, false)
                .screen_size,
            (1920, 1080)
        );
        // The test's folder is empty, so the file path is empty too.
        assert_eq!(hardware.settings_file, Some(PathBuf::new()));

        // Without a Supersample line Exarust's own level stays on.
        let plain = user(b"Volume = 100\r\n");
        let hardware = plain.hardware(false, None, false, false, false);
        assert_eq!(hardware.live.supersample, 4);
        assert_eq!(hardware.live.volume, 1.0);

        let hardware = u.hardware(true, None, true, true, true);
        assert!(!hardware.live.window_mode);
        assert!(!hardware.live.vertical_sync);
        assert_eq!(hardware.live.supersample, 0);
        assert!(!hardware.live.show_fps);
        assert_eq!(hardware.window_size, None);
    }

    #[test]
    fn the_store_gives_the_bindings() {
        let mut u = user(b"");
        assert_eq!(u.bindings(), None);
        let mut b = exa_game::bindings::Bindings::default();
        b.controller = true;
        u.store.set(&BINDINGS_PATH, &b.encode()).unwrap();
        // The saved rows come back (the modifiers read one-sided once
        // loaded, as the original's conversion gives them).
        let loaded = u.bindings().expect("the store holds a set");
        assert!(loaded.controller);
        assert_eq!(loaded.encode(), b.encode());
    }
}
