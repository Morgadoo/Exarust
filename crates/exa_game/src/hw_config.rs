//! The hardware configuration at run time: the settings screen's live and
//! draft options, Apply and Cancel, the GPU benchmark's driver, the renderer
//! and audio options the screen changes, and key rebinding.
//!
//! The pure rules are [`exa_formats::hw_config`]; the row behaviour is
//! [`exa_ui::settings`]. This module wires them to the running game: the
//! options become [`exa_render::quality`] resources, the scene's
//! super-sampling switch, the audio buses and the window's mode, size and
//! present mode. Rules: `docs/UI.md` (Settings options, Controls) and
//! `docs/formats/SETTINGS.md`.

use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::{PresentMode, PrimaryWindow, WindowMode};
use exa_audio::{AudioEnvironment, MusicSetting};
use exa_formats::file_ref::write_whole_file;
use exa_formats::hw_config::{Benchmark, Options, QualityChange, Screen};
use exa_formats::settings_ini;
use exa_render::quality::{MultisampleSetting, ShadowQuality};
use exa_render::supersample::SuperSampling;
use exa_ui::settings::{self, Effect, Setting};

use crate::bindings::{self, Action, Bindings};

/// The performance counter's ticks per second the benchmark's clock uses.
pub const BENCHMARK_FREQUENCY: u64 = 1_000_000;
/// The benchmark's target image and source texture sizes.
const BENCHMARK_TARGET: u32 = 1024;
const BENCHMARK_SOURCE: u32 = 512;
/// The render layer of the benchmark's off-screen camera.
const BENCHMARK_LAYER: usize = 30;

/// The hardware configuration: the live options and the open settings
/// screen, if any.
#[derive(Resource, Debug, Clone)]
pub struct HardwareConfig {
    /// The options the game runs with.
    pub live: Options,
    /// The open screen's draft and the values it opened with.
    pub screen: Option<Screen>,
    /// The settings file Apply rewrites, when the launcher gave one.
    pub settings_file: Option<PathBuf>,
    /// The benchmark in progress.
    pub benchmark: Option<Benchmark>,
    /// Whether the benchmark was started by opening the page: its result
    /// then fills the page's draft, as the original's show handler does.
    pub refresh_after_benchmark: bool,
    /// The Music checkbox's session state. The option itself keeps the
    /// startup value, as the original's does.
    pub session_music: bool,
    /// The window size the launcher asked for, kept for a return to windowed
    /// mode.
    pub window_size: Option<(u32, u32)>,
    /// The Controls page's "Enable Rumble" checkbox. It has no saved setting
    /// in the original and no Exarust effect (no gamepad); kept as a session
    /// flag (its default is a guess).
    pub rumble: bool,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        let live = Options::default();
        Self {
            live,
            screen: None,
            settings_file: None,
            benchmark: None,
            refresh_after_benchmark: false,
            session_music: live.music,
            window_size: None,
            rumble: true,
        }
    }
}

/// What Apply wrote and changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    /// The `Name = value` lines, in order.
    pub lines: Vec<(&'static str, String)>,
    /// The renderer options whose change rebuilds the original's shaders.
    pub quality: QualityChange,
    /// Whether the window mode changed.
    pub fullscreen_changed: bool,
    /// The file was written.
    pub stored: bool,
}

impl HardwareConfig {
    /// The configuration over the live options a settings file gave.
    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.settings_file = Some(file.into());
        self
    }

    /// Opens the settings screen over the live options, or keeps it open, and
    /// runs the GPU benchmark its show handler runs: the result fills the
    /// page's draft when it arrives.
    pub fn open(&mut self) {
        if self.screen.is_none() {
            self.screen = Some(Screen::open(&self.live));
        }
        if self.benchmark.is_none() {
            self.benchmark = Some(Benchmark::new(BENCHMARK_FREQUENCY));
        }
        self.refresh_after_benchmark = true;
    }

    /// Closes the screen without applying (the original's page switch).
    pub fn close(&mut self) {
        self.screen = None;
        self.benchmark = None;
        self.refresh_after_benchmark = false;
    }

    /// Cancel: pushes the options the screen opened with back into the live
    /// options and closes the screen.
    pub fn cancel(&mut self) -> Option<Options> {
        let screen = self.screen.take()?;
        self.live = screen.cancel();
        self.session_music = self.live.music;
        self.benchmark = None;
        self.refresh_after_benchmark = false;
        Some(self.live)
    }

    /// Apply: writes the settings file, takes the draft's renderer options
    /// live and closes the screen.
    pub fn apply(&mut self) -> Option<Applied> {
        let screen = self.screen.take()?;
        self.benchmark = None;
        self.refresh_after_benchmark = false;
        let draft = screen.draft;
        let quality = screen.quality_change();
        let fullscreen_changed = draft.window_mode != screen.baseline.window_mode;
        self.live.supersample = draft.supersample;
        self.live.shading_quality = draft.shading_quality;
        self.live.shadow_detail = draft.shadow_detail;
        self.live.shadow_quality = draft.shadow_quality;
        self.live.window_mode = draft.window_mode;
        // Hardware anti-aliasing, texture quality and shading reach the
        // renderer at its next start in the original too: their controls are
        // not pushed by Apply.
        let lines = screen.settings_lines(&self.live);
        let stored = self.write_lines(&screen);
        Some(Applied {
            lines,
            quality,
            fullscreen_changed,
            stored,
        })
    }

    /// The lines Apply would write, without applying (for the launcher's
    /// tests and diagnostics).
    pub fn lines(&self) -> Option<Vec<(&'static str, String)>> {
        self.screen
            .as_ref()
            .map(|screen| screen.settings_lines(&self.live))
    }

    fn write_lines(&self, screen: &Screen) -> bool {
        let Some(path) = &self.settings_file else {
            return false;
        };
        write_settings_file(path, screen, &self.live)
    }

    /// Starts the GPU benchmark. The plugin's driver runs it and takes the
    /// quality levels it finds.
    pub fn start_benchmark(&mut self) {
        self.benchmark = Some(Benchmark::new(BENCHMARK_FREQUENCY));
    }

    /// Ends the benchmark with its performance index: the auto-detected
    /// quality becomes live and the screen's draft, as the original's
    /// Defaults button does.
    pub fn finish_benchmark(&mut self, perf_index: f32, width: i64, height: i64) {
        self.benchmark = None;
        self.live = Options::auto_detected(&self.live, perf_index, width, height);
        self.session_music = self.live.music;
        if std::mem::take(&mut self.refresh_after_benchmark)
            && let Some(screen) = &mut self.screen
        {
            screen.draft = Options::opened_from(&self.live);
            screen.baseline = self.live;
        }
    }

    /// The Deck's fixed preset, instead of a benchmark.
    pub fn deck_preset(&mut self) {
        self.live = Options::deck_preset(&self.live);
    }

    /// A slider step on the open screen. Aim speed is the extra row the
    /// screen exposes separately.
    pub fn set_slider(&mut self, setting: Setting, fraction: f32) -> Effect {
        let Some(screen) = &mut self.screen else {
            return Effect::Ignored;
        };
        let effect = settings::on_slider(&mut screen.draft, setting, fraction);
        if effect == Effect::Live && setting == Setting::Volume {
            self.live.volume = screen.draft.volume;
        }
        effect
    }

    /// A checkbox click on the open screen.
    pub fn set_toggle(&mut self, setting: Setting, checked: bool) -> Effect {
        let Some(screen) = &mut self.screen else {
            return Effect::Ignored;
        };
        let effect = settings::on_toggle(&mut screen.draft, setting, checked);
        if effect == Effect::Live {
            self.live.vertical_sync = screen.draft.vertical_sync;
            self.live.show_fps = screen.draft.show_fps;
            self.live.show_tips = screen.draft.show_tips;
            self.live.checkpoints = screen.draft.checkpoints;
        } else if effect == Effect::SessionOnly {
            self.session_music = checked;
        }
        effect
    }

    /// The Controls page's controller-mode switch. Turning it on or off
    /// changes the input mode Apply writes; Exarust has no gamepad to drive.
    pub fn set_controller(&mut self, enabled: bool) -> bool {
        let Some(screen) = &mut self.screen else {
            return false;
        };
        screen.draft.controller = enabled;
        self.live.controller = enabled;
        true
    }

    /// The Controls page's rumble switch (session only).
    pub fn set_rumble(&mut self, enabled: bool) {
        self.rumble = enabled;
    }

    /// An aim-speed slider step: the live aim speed and the number shown.
    pub fn set_aim_speed(&mut self, fraction: f32) -> Option<i32> {
        let screen = self.screen.as_mut()?;
        let text = settings::set_aim_speed(&mut screen.draft, fraction);
        self.live.aim_speed = screen.draft.aim_speed;
        Some(text)
    }

    /// The open screen's rows.
    pub fn rows(&self) -> Option<[(Setting, settings::RowState); 14]> {
        self.screen
            .as_ref()
            .map(|screen| settings::populate(&screen.draft))
    }
}

/// Writes the settings screen's lines into `path`, creating the file when it
/// is missing, as the original does.
pub fn write_settings_file(path: &Path, screen: &Screen, live: &Options) -> bool {
    let mut file = settings_ini::open_settings_file(path);
    screen.write_settings(&mut file, live);
    write_whole_file(path, &file)
}

/// A rebinding in progress: the row whose key cell was clicked.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BindingEditor {
    /// The Controls row being rebound.
    pub row: Option<usize>,
}

/// The editor's answer to a key or button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rebind {
    /// A key or button was assigned to the row.
    Assigned(u32),
    /// Escape cancelled the rebinding.
    Cancelled,
    /// Nothing was assigned.
    None,
}

impl BindingEditor {
    /// Starts a rebinding of the Controls row `row`, when it has a key cell.
    pub fn begin(&mut self, row: usize) -> bool {
        if settings::BINDINGS
            .get(row)
            .is_some_and(|binding| binding.keyed)
        {
            self.row = Some(row);
            true
        } else {
            false
        }
    }

    /// Cancels the rebinding.
    pub fn cancel(&mut self) -> bool {
        self.row.take().is_some()
    }

    /// Takes a key code for the open row, writing it into `bindings`.
    ///
    /// Escape cancels. A code of 0 is ignored. The row's slot is the binding
    /// table's slot; `Power Tree` has none and is never opened.
    pub fn key(&mut self, code: u32, bindings: &mut Bindings) -> Rebind {
        let Some(row) = self.row else {
            return Rebind::None;
        };
        if code == 0x1B {
            self.row = None;
            return Rebind::Cancelled;
        }
        if code == 0 {
            return Rebind::None;
        }
        let Some(binding) = settings::BINDINGS.get(row) else {
            self.row = None;
            return Rebind::None;
        };
        let Some(slot) = binding.slot else {
            self.row = None;
            return Rebind::None;
        };
        let Some(action) = action_at(slot) else {
            self.row = None;
            return Rebind::None;
        };
        self.row = None;
        bindings.set_key(action, code);
        Rebind::Assigned(code)
    }
}

/// The binding-table action of a slot.
pub fn action_at(slot: usize) -> Option<Action> {
    Some(match slot {
        0 => Action::Primary,
        1 => Action::Forward,
        2 => Action::Back,
        3 => Action::Left,
        4 => Action::Right,
        5 => Action::Crouch,
        6 => Action::Run,
        7 => Action::Alternate,
        8 => Action::Orbit,
        9 => Action::CursorLook,
        10 => Action::Interact,
        11 => Action::Inventory,
        12 => Action::ToggleCombat,
        13 => Action::Skills,
        14 => Action::SwitchEquipped,
        15 => Action::FlipWeapon,
        16 => Action::Journal,
        17 => Action::Talk,
        18 => Action::UsePower,
        19 => Action::PowerBar,
        20 => Action::PowerQuick,
        21 => Action::PowerNext,
        22 => Action::PowerPrevious,
        _ => return None,
    })
}

/// The Controls page's rows: each label and its key's name from `bindings`.
pub fn binding_rows(bindings: &Bindings) -> Vec<(&'static str, String)> {
    settings::BINDINGS
        .iter()
        .map(|binding| {
            let name = binding
                .slot
                .and_then(action_at)
                .map(|action| bindings.key_name(action))
                .unwrap_or_default();
            (binding.label, name)
        })
        .collect()
}

/// The hardware configuration's plugin.
pub struct HardwareConfigPlugin;

impl Plugin for HardwareConfigPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BindingEditor>().add_systems(
            Update,
            (
                rebind_keys,
                drive_benchmark,
                apply_options,
                apply_window,
                start_benchmark_scene,
                cleanup_benchmark,
            )
                .chain(),
        );
    }
}

/// Reads a key or mouse button for the open binding row.
pub fn rebind_keys(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut editor: ResMut<BindingEditor>,
    bindings: Option<ResMut<Bindings>>,
) {
    let Some(mut bindings) = bindings else {
        return;
    };
    if editor.row.is_none() {
        return;
    }
    // Mouse first: a click on the key cell is also a press, and the original
    // names extra buttons.
    for button in [
        MouseButton::Left,
        MouseButton::Right,
        MouseButton::Middle,
        MouseButton::Back,
        MouseButton::Forward,
    ] {
        if buttons.just_pressed(button) {
            editor.key(bindings::mouse_code(button), &mut bindings);
            return;
        }
    }
    for key in keys.get_just_pressed() {
        if let Some(code) = bindings::virtual_key(*key) {
            editor.key(code, &mut bindings);
            return;
        }
    }
}

/// Pushes changed live options into the renderer, the audio and the window.
fn apply_options(
    config: Option<Res<HardwareConfig>>,
    mut sampling: ResMut<SuperSampling>,
    mut shadow: ResMut<ShadowQuality>,
    mut multisample: ResMut<MultisampleSetting>,
    mut audio: Option<ResMut<AudioEnvironment>>,
    mut music: Option<ResMut<MusicSetting>>,
    mut show_fps: Option<ResMut<exa_ui::ShowFps>>,
) {
    let Some(config) = config else {
        return;
    };
    if !config.is_changed() {
        return;
    }
    let live = config.live;
    let sampling_on = live.supersample > 0;
    if sampling.0 != sampling_on {
        sampling.0 = sampling_on;
    }
    if shadow.0 != live.shadow_quality {
        shadow.0 = live.shadow_quality;
    }
    if multisample.0 != live.multisample {
        multisample.0 = live.multisample;
    }
    if let Some(audio) = audio.as_mut()
        && audio.buses.master != live.volume
    {
        audio.buses.master = live.volume;
    }
    if let Some(music) = music.as_mut()
        && music.enabled != config.session_music
    {
        music.enabled = config.session_music;
    }
    if let Some(show_fps) = show_fps.as_mut()
        && show_fps.0 != live.show_fps
    {
        show_fps.0 = live.show_fps;
    }
}

/// The window state a frame last applied: windowed, size and vertical sync.
type AppliedWindow = (bool, Option<(u32, u32)>, bool);

/// Applies the live window mode, size and vertical sync.
fn apply_window(
    config: Option<Res<HardwareConfig>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut applied: Local<Option<AppliedWindow>>,
) {
    let Some(config) = config else {
        return;
    };
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let state = (
        config.live.window_mode,
        config.window_size,
        config.live.vertical_sync,
    );
    if applied.as_ref() == Some(&state) {
        return;
    }
    *applied = Some(state);
    let mode = if config.live.window_mode {
        WindowMode::Windowed
    } else {
        WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Primary)
    };
    if window.mode != mode {
        window.mode = mode;
    }
    if let (true, Some((width, height))) = (config.live.window_mode, config.window_size) {
        // The launcher's size is in logical pixels; keep the same unit.
        let (current_w, current_h) = (window.resolution.width(), window.resolution.height());
        if (current_w, current_h) != (width as f32, height as f32) {
            window.resolution.set(width as f32, height as f32);
        }
    }
    let present = if config.live.vertical_sync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
    if window.present_mode != present {
        window.present_mode = present;
    }
}

/// The off-screen scene the benchmark draws.
#[derive(Resource)]
struct BenchmarkScene {
    camera: Entity,
    quads: Vec<Entity>,
    target: Handle<Image>,
    source: Handle<Image>,
    present_mode: PresentMode,
}

const BENCHMARK_CAMERA: &str = "_BenchmarkCamera";
const BENCHMARK_QUAD: &str = "_BenchmarkQuad";

/// Starts the benchmark's scene when a benchmark begins: a 1024² RGBA8
/// target, a 512² texture and sixteen full-screen quads, with vertical sync
/// off so the frames are not capped. The original draws sixteen full-screen
/// quads with a textured shader per frame; this is the same fill-rate
/// workload through Bevy's sprite pipeline. The measured protocol and score
/// are exact (the GPU and its driver differ anyway).
fn start_benchmark_scene(
    mut commands: Commands,
    config: Option<Res<HardwareConfig>>,
    scene: Option<Res<BenchmarkScene>>,
    mut images: ResMut<Assets<Image>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Some(config) = config else {
        return;
    };
    if config.benchmark.is_none() {
        return;
    }
    if scene.is_some() {
        return;
    }
    let mut target = Image::new_fill(
        Extent3d {
            width: BENCHMARK_TARGET,
            height: BENCHMARK_TARGET,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    target.texture_descriptor.usage =
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    let target = images.add(target);

    // A deterministic grey noise, the pattern the original loads from its
    // 512² texture.
    let mut pixels = vec![0u8; (BENCHMARK_SOURCE * BENCHMARK_SOURCE * 4) as usize];
    let mut state = 0x1234_5678u32;
    for pixel in pixels.as_chunks_mut::<4>().0 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let value = (state >> 24) as u8;
        *pixel = [value, value, value, 255];
    }
    let source = images.add(Image::new(
        Extent3d {
            width: BENCHMARK_SOURCE,
            height: BENCHMARK_SOURCE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ));

    let layer = bevy::camera::visibility::RenderLayers::layer(BENCHMARK_LAYER);
    let camera = commands
        .spawn((
            Name::new(BENCHMARK_CAMERA),
            Camera2d,
            bevy::camera::RenderTarget::Image(target.clone().into()),
            Camera {
                clear_color: bevy::camera::ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            layer.clone(),
        ))
        .id();
    let mut quads = Vec::with_capacity(16);
    for index in 0..16 {
        let quad = commands
            .spawn((
                Name::new(BENCHMARK_QUAD),
                Sprite {
                    image: source.clone(),
                    custom_size: Some(Vec2::splat(BENCHMARK_TARGET as f32)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, index as f32 * 0.001),
                layer.clone(),
            ))
            .id();
        quads.push(quad);
    }
    let present_mode = windows
        .single_mut()
        .map(|mut window| {
            let previous = window.present_mode;
            if previous != PresentMode::AutoNoVsync {
                window.present_mode = PresentMode::AutoNoVsync;
            }
            previous
        })
        .unwrap_or(PresentMode::AutoVsync);
    commands.insert_resource(BenchmarkScene {
        camera,
        quads,
        target,
        source,
        present_mode,
    });
}

/// Counts the benchmark's frames on the real clock, and when the third run
/// ends takes the auto-detected quality and tears the scene down.
fn drive_benchmark(
    time: Res<Time<Real>>,
    config: Option<ResMut<HardwareConfig>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Some(mut config) = config else {
        return;
    };
    let ticks = (time.elapsed_secs_f64() * BENCHMARK_FREQUENCY as f64) as u64;
    let Some(perf) = config
        .benchmark
        .as_mut()
        .and_then(|benchmark| benchmark.frame(ticks))
    else {
        return;
    };
    info!("GPU benchmark: performance index {perf:.2}");
    let Ok(window) = windows.single() else {
        // The window may have closed; keep the measurement for the next one.
        return;
    };
    let (width, height) = (
        i64::from(window.physical_width()),
        i64::from(window.physical_height()),
    );
    config.finish_benchmark(perf, width, height);
}

/// Tears the benchmark's scene down once no benchmark is running, restoring
/// the window's present mode.
fn cleanup_benchmark(
    mut commands: Commands,
    config: Option<Res<HardwareConfig>>,
    scene: Option<Res<BenchmarkScene>>,
    mut images: ResMut<Assets<Image>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if config.is_none_or(|config| config.benchmark.is_some()) {
        return;
    }
    let Some(scene) = scene else {
        return;
    };
    commands.entity(scene.camera).despawn();
    for quad in &scene.quads {
        commands.entity(*quad).despawn();
    }
    images.remove(&scene.target);
    images.remove(&scene.source);
    let present_mode = scene.present_mode;
    if let Ok(mut window) = windows.single_mut()
        && window.present_mode != present_mode
    {
        window.present_mode = present_mode;
    }
    commands.remove_resource::<BenchmarkScene>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_takes_the_renderer_options_and_writes_the_lines() {
        let mut config = HardwareConfig::default();
        config.open();
        config.set_slider(Setting::Supersample, 1.0);
        config.set_slider(Setting::ShadowQuality, 2.0 / 3.0);
        let applied = config.apply().expect("a screen was open");
        assert_eq!(config.live.supersample, 4);
        assert_eq!(config.live.shadow_quality, 3);
        assert!(applied.quality.supersample);
        assert!(applied.quality.shadow_quality);
        assert!(applied.quality.rebuilds_shaders());
        assert!(!applied.fullscreen_changed);
        assert!(!applied.stored);
        assert_eq!(applied.lines[2], ("Supersample", "4".to_string()));
        assert_eq!(applied.lines[7], ("ShadowQuality", "3".to_string()));
        assert!(config.screen.is_none());
    }

    #[test]
    fn cancel_restores_the_opened_options() {
        let config = HardwareConfig::default();
        let live = config.live;
        let mut config = config;
        config.open();
        config.set_slider(Setting::Supersample, 1.0);
        assert_eq!(config.live.supersample, live.supersample);
        assert_eq!(config.cancel(), Some(live));
        assert!(config.screen.is_none());
    }

    #[test]
    fn the_volume_is_live_and_the_music_only_mutes_the_session() {
        let mut config = HardwareConfig::default();
        config.open();
        assert_eq!(config.set_slider(Setting::Volume, 0.5), Effect::Live);
        assert_eq!(config.live.volume, 0.75);
        assert_eq!(
            config.set_toggle(Setting::Music, false),
            Effect::SessionOnly
        );
        assert!(!config.session_music);
        assert!(config.live.music);
        assert_eq!(config.set_toggle(Setting::WindowMode, false), Effect::Draft);
        assert!(config.live.window_mode);
        assert!(!config.screen.as_ref().unwrap().draft.window_mode);
    }

    #[test]
    fn the_quality_of_a_benchmark_becomes_live() {
        let mut config = HardwareConfig::default();
        config.open();
        config.finish_benchmark(4.0, 1600, 900);
        assert_eq!(config.live.perf_index, 4.0);
        assert_eq!(config.live.supersample, 3);
        assert_eq!(config.live.shading_quality, 5);
        assert_eq!(config.live.shadow_quality, 4);
        // The benchmark came from opening the page, so its result fills the
        // draft, as the original's show handler does.
        let screen = config.screen.as_ref().unwrap();
        assert_eq!(screen.draft.supersample, 3);
        assert_eq!(screen.baseline.supersample, 3);
    }

    #[test]
    fn the_controller_and_rumble_switches() {
        let mut config = HardwareConfig::default();
        assert!(!config.set_controller(true)); // no page open
        config.open();
        assert!(config.set_controller(true));
        assert!(config.live.controller);
        assert_eq!(config.lines().unwrap()[9], ("InputMode", "51".to_string()));
        config.set_rumble(false);
        assert!(!config.rumble);
    }

    #[test]
    fn the_deck_preset_is_fixed() {
        let mut config = HardwareConfig::default();
        config.deck_preset();
        assert_eq!(config.live.supersample, 3);
        assert_eq!(config.live.shading_quality, 5);
        assert_eq!(config.live.shadow_quality, 3);
    }

    #[test]
    fn a_rebinding_writes_the_table_slot() {
        let mut bindings = Bindings::default();
        let mut editor = BindingEditor::default();
        assert!(editor.begin(0));
        assert_eq!(editor.key(0x4C, &mut bindings), Rebind::Assigned(0x4C));
        assert_eq!(bindings.key(Action::Forward), 0x4C);
        assert_eq!(editor.key(0x4D, &mut bindings), Rebind::None);

        assert!(editor.begin(0));
        assert_eq!(editor.key(0x1B, &mut bindings), Rebind::Cancelled);
        assert_eq!(bindings.key(Action::Forward), 0x4C);

        // The Power Tree row has no key cell.
        assert!(!editor.begin(21));
        assert_eq!(editor.key(0x4E, &mut bindings), Rebind::None);
    }

    #[test]
    fn the_binding_rows_name_every_key() {
        let bindings = Bindings::default();
        let rows = binding_rows(&bindings);
        assert_eq!(rows.len(), 22);
        assert_eq!(rows[0], ("Forward", "W".into()));
        assert_eq!(rows[6], ("Orbit Camera", "Middle mouse".into()));
        assert_eq!(rows[8], ("Toggle Combat", "Tab".into()));
        assert_eq!(rows[21], ("Power Tree", String::new()));
    }

    #[test]
    fn the_settings_file_gets_the_lines_appended() {
        let dir = std::env::temp_dir().join("exarust-hw-config-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("Exanima.ini");
        let _ = std::fs::remove_file(&path);
        let mut config = HardwareConfig::default().with_file(&path);
        config.open();
        config.set_slider(Setting::Supersample, 1.0);
        let applied = config.apply().expect("open");
        assert!(applied.stored);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("Supersample = 4"));
        assert!(text.contains("PerfIndex = 0.00"));
        let _ = std::fs::remove_file(&path);
    }
}
