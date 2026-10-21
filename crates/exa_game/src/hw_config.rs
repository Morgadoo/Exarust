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

use std::path::PathBuf;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::{
    Monitor, MonitorSelection, PresentMode, PrimaryMonitor, PrimaryWindow, VideoModeSelection,
    WindowMode,
};
use exa_audio::{AudioEnvironment, MusicSetting};
use exa_formats::file_ref::{read_whole_file, write_whole_file};
use exa_formats::hw_config::{Benchmark, Options, QualityChange, Screen};
use exa_formats::settings_ini;
use exa_render::quality::{MultisampleSetting, ShadowQuality};
use exa_render::supersample::SuperSampling;
use exa_ui::settings::{self, Effect, Setting};

use crate::bindings::{self, Action, Bindings, DEFAULT_KEYS};

/// The performance counter's ticks per second the benchmark's clock uses.
pub const BENCHMARK_FREQUENCY: u64 = 1_000_000;
/// The benchmark's target image and source texture sizes.
const BENCHMARK_TARGET: u32 = 1024;
const BENCHMARK_SOURCE: u32 = 512;
/// The render layer of the benchmark's off-screen camera.
const BENCHMARK_LAYER: usize = 30;

/// The hardware configuration: the live options, the saved set and the open
/// settings page, if any.
#[derive(Resource, Debug, Clone)]
pub struct HardwareConfig {
    /// The options the game runs with.
    pub live: Options,
    /// The saved set while no page is open (the open page's is its
    /// baseline). Cancel and the auto-detection push it into the live
    /// options. The original's starts from its variables' initial values;
    /// Exarust's from the launch options.
    pub saved: Options,
    /// The open page's draft and saved set.
    pub screen: Option<Screen>,
    /// Whether the open form shows its Controls page rather than Settings.
    pub controls_page: bool,
    /// The Controls page's copy of the bindings, while the form is open:
    /// rebinding edits it and Apply commits it.
    pub bindings: Option<Bindings>,
    /// The settings file the page reads when it opens and Apply rewrites,
    /// when the launcher gave one.
    pub settings_file: Option<PathBuf>,
    /// The benchmark in progress.
    pub benchmark: Option<Benchmark>,
    /// Whether this is the Deck, whose fixed preset replaces the benchmark
    /// (Exarust does not detect it).
    pub steam_deck: bool,
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
        Self::new(Options::default())
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
    /// The bindings to take and save: the Controls page's copy, when its
    /// Forward key is set (the original commits its copy only when the copy's
    /// first slot, Forward's, is not zero).
    pub bindings: Option<Bindings>,
}

impl HardwareConfig {
    /// The configuration over `live`, which is also the saved set.
    pub fn new(live: Options) -> Self {
        Self {
            live,
            saved: live,
            screen: None,
            controls_page: false,
            bindings: None,
            settings_file: None,
            benchmark: None,
            steam_deck: false,
            session_music: live.music,
            window_size: None,
            rumble: true,
        }
    }

    /// The configuration over the live options a settings file gave.
    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.settings_file = Some(file.into());
        self
    }

    /// The saved set: the open page's, else the one kept between pages.
    pub fn saved(&self) -> Options {
        self.screen.map_or(self.saved, |screen| screen.baseline)
    }

    /// Opens the settings form on its Settings page, as the original's form
    /// creation does: the saved set is the live options, the settings file is
    /// read again over them for the draft, and the live quality levels,
    /// sampling, vertical sync and window mode go back to the saved ones
    /// ([`Screen::open`]). The Controls page takes a copy of `bindings`.
    /// Nothing is measured: the benchmark runs only from Defaults.
    pub fn open(&mut self, bindings: Option<&Bindings>) {
        if self.screen.is_some() {
            return;
        }
        let reread = self
            .settings_file
            .as_deref()
            .and_then(read_whole_file)
            .map(|bytes| {
                self.live.reread(&settings_ini::read_assignments(
                    &bytes,
                    &settings_ini::REGISTRY,
                ))
            })
            .unwrap_or(self.live);
        let (screen, live) = Screen::open(&self.live, &reread);
        self.screen = Some(screen);
        self.live = live;
        self.controls_page = false;
        self.bindings = bindings.cloned();
    }

    /// Closes the form without applying: the saved set stays for a later
    /// auto-detection, the Controls copy is dropped.
    pub fn close(&mut self) {
        if let Some(screen) = self.screen.take() {
            self.saved = screen.baseline;
        }
        self.bindings = None;
    }

    /// Shows the Settings (`false`) or Controls (`true`) page.
    pub fn switch_page(&mut self, controls: bool) {
        if self.screen.is_some() {
            self.controls_page = controls;
        }
    }

    /// Cancel: pushes the saved set back into the live options and closes
    /// the form; the Controls page's edits are dropped.
    pub fn cancel(&mut self) -> Option<Options> {
        let screen = self.screen?;
        self.live = screen.cancel(&self.live);
        self.session_music = screen.baseline.music;
        self.close();
        Some(self.live)
    }

    /// Apply: reads the settings file, writes the page's lines into it, takes
    /// the draft's window mode and its changed renderer options live, and
    /// closes the form. When the settings file cannot be read nothing at all
    /// happens and the form stays open, as in the original; without a file
    /// (an Exarust test or launch without settings) nothing is stored.
    pub fn apply(&mut self) -> Option<Applied> {
        let screen = self.screen?;
        let file = match &self.settings_file {
            Some(path) => Some((path.clone(), read_whole_file(path)?)),
            None => None,
        };
        let bindings = self
            .bindings
            .take()
            .filter(|bindings| bindings.key(Action::Forward) != 0);
        let lines = screen.settings_lines(&self.live);
        let stored = file.is_some_and(|(path, mut bytes)| {
            screen.write_settings(&mut bytes, &self.live);
            write_whole_file(&path, &bytes)
        });
        let draft = screen.draft;
        let fullscreen_changed = draft.window_mode != self.live.window_mode;
        self.live.window_mode = draft.window_mode;
        let quality = screen.quality_change();
        if quality.supersample {
            self.live.supersample = draft.supersample;
        }
        if quality.shading {
            self.live.shading_quality = draft.shading_quality;
        }
        if quality.shadow_quality {
            self.live.shadow_quality = draft.shadow_quality;
        }
        if quality.shadow_detail {
            self.live.shadow_detail = draft.shadow_detail;
        }
        // Hardware anti-aliasing is only stored: it takes effect when the
        // file is read at the next start.
        self.close();
        Some(Applied {
            lines,
            quality,
            fullscreen_changed,
            stored,
            bindings,
        })
    }

    /// The lines Apply would write, without applying (for the launcher's
    /// tests and diagnostics).
    pub fn lines(&self) -> Option<Vec<(&'static str, String)>> {
        self.screen
            .as_ref()
            .map(|screen| screen.settings_lines(&self.live))
    }

    /// The Defaults button. On the Settings page it runs the GPU benchmark
    /// (the Deck takes its preset at once); on the Controls page it puts the
    /// shipped keys into the page's copy, or in controller mode the saved
    /// aim speed back into the live one.
    pub fn defaults(&mut self) {
        if self.screen.is_none() {
            return;
        }
        if !self.controls_page {
            if self.steam_deck {
                self.deck_preset();
            } else {
                self.start_benchmark();
            }
        } else if self.live.controller {
            self.live.aim_speed = self.saved().aim_speed;
        } else if let Some(bindings) = &mut self.bindings {
            for action in Action::ALL {
                bindings.set_key(action, DEFAULT_KEYS[action.slot()]);
            }
        }
    }

    /// Starts the GPU benchmark. The plugin's driver runs it and takes the
    /// quality levels it finds.
    pub fn start_benchmark(&mut self) {
        self.benchmark = Some(Benchmark::new(BENCHMARK_FREQUENCY));
    }

    /// Ends the benchmark with its performance index for a `width` × `height`
    /// display: the auto-detected levels become the saved set, which is
    /// pushed into the live options and the page's draft
    /// ([`Options::auto_detected`]).
    pub fn finish_benchmark(&mut self, perf_index: f32, width: i64, height: i64) {
        self.benchmark = None;
        self.live.perf_index = perf_index;
        let saved = self.live.auto_detected(&self.saved(), width, height);
        self.push_saved(saved);
    }

    /// The Deck's fixed preset, instead of a benchmark.
    pub fn deck_preset(&mut self) {
        let saved = self.live.deck_preset(&self.saved());
        self.push_saved(saved);
    }

    /// Makes `saved` the saved set and pushes it into the live options and
    /// the open page's draft.
    fn push_saved(&mut self, saved: Options) {
        self.live = saved.pushed_live(&self.live);
        self.session_music = saved.music;
        match &mut self.screen {
            Some(screen) => {
                screen.draft = saved.pushed_draft(&screen.draft);
                screen.baseline = saved;
            }
            None => self.saved = saved,
        }
    }

    /// A slider step on the open page. The volume and the texture sliders
    /// change the live options too.
    pub fn set_slider(&mut self, setting: Setting, fraction: f32) -> Effect {
        let Some(screen) = &mut self.screen else {
            return Effect::Ignored;
        };
        let effect = settings::on_slider(&mut screen.draft, setting, fraction);
        if effect == Effect::Live {
            match setting {
                Setting::Volume => self.live.volume = screen.draft.volume,
                Setting::TextureDetail => self.live.texture_detail = screen.draft.texture_detail,
                Setting::TextureQuality => {
                    self.live.texture_quality = screen.draft.texture_quality;
                }
                _ => {}
            }
        }
        effect
    }

    /// A checkbox click on the open page. A live checkbox changes only its
    /// own live option.
    pub fn set_toggle(&mut self, setting: Setting, checked: bool) -> Effect {
        let Some(screen) = &mut self.screen else {
            return Effect::Ignored;
        };
        let effect = settings::on_toggle(&mut screen.draft, setting, checked);
        match effect {
            Effect::Live => match setting {
                Setting::VerticalSync => self.live.vertical_sync = checked,
                Setting::ShowFps => self.live.show_fps = checked,
                Setting::ShowTips => self.live.show_tips = checked,
                Setting::Checkpoints => self.live.checkpoints = checked,
                _ => {}
            },
            Effect::SessionOnly => self.session_music = checked,
            Effect::Draft | Effect::Ignored => {}
        }
        effect
    }

    /// The Controls page's Keyboard / Controller switch: it sets the live
    /// input mode Apply writes (Exarust has no gamepad to drive).
    pub fn set_controller(&mut self, enabled: bool) -> bool {
        if self.screen.is_none() {
            return false;
        }
        self.live.controller = enabled;
        true
    }

    /// The Controls page's rumble switch (session only).
    pub fn set_rumble(&mut self, enabled: bool) {
        self.rumble = enabled;
    }

    /// An aim-speed slider step: the live aim speed and the number shown.
    pub fn set_aim_speed(&mut self, fraction: f32) -> Option<i32> {
        self.screen?;
        Some(settings::set_aim_speed(&mut self.live, fraction))
    }

    /// The open page's rows.
    pub fn rows(&self) -> Option<[(Setting, settings::RowState); 14]> {
        self.screen
            .as_ref()
            .map(|screen| settings::populate(&self.live, &screen.draft))
    }
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

    /// Takes a key code for the open row, writing it into `bindings` (the
    /// Controls page's copy).
    ///
    /// Escape cancels. A code of 0 is ignored. The row's slot is the binding
    /// table's slot (Exarust's guess at the key-read screen, which is outside
    /// this unit).
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
    Action::from_slot(slot)
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

/// Reads a key or mouse button for the open binding row into the Controls
/// page's copy of the bindings.
pub fn rebind_keys(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut editor: ResMut<BindingEditor>,
    config: Option<ResMut<HardwareConfig>>,
) {
    let Some(mut config) = config else {
        return;
    };
    if editor.row.is_none() {
        return;
    }
    let Some(bindings) = config.bindings.as_mut() else {
        // No form is open: nothing to edit.
        editor.row = None;
        return;
    };
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
            editor.key(bindings::mouse_code(button), bindings);
            return;
        }
    }
    for key in keys.get_just_pressed() {
        if let Some(code) = bindings::virtual_key(*key) {
            editor.key(code, bindings);
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
    monitors: Query<&Monitor, With<PrimaryMonitor>>,
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
    // The original's full screen changes the display mode and creates an
    // exclusive window; winit falls back to borderless where the platform
    // has no exclusive mode. A requested size picks that video mode when the
    // display lists it, otherwise the current mode stays.
    let mode = if config.live.window_mode {
        WindowMode::Windowed
    } else {
        let specific = config.window_size.and_then(|(width, height)| {
            monitors.single().ok().and_then(|monitor| {
                monitor
                    .video_modes
                    .iter()
                    .find(|mode| mode.physical_size.x == width && mode.physical_size.y == height)
                    .copied()
                    .map(VideoModeSelection::Specific)
            })
        });
        WindowMode::Fullscreen(
            MonitorSelection::Primary,
            specific.unwrap_or(VideoModeSelection::Current),
        )
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

    fn temp_file(name: &str, text: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join("exarust-hw-config-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(name);
        let _ = std::fs::remove_file(&path);
        if !text.is_empty() {
            std::fs::write(&path, text).unwrap();
        }
        path
    }

    #[test]
    fn opening_runs_no_benchmark_and_copies_the_live_options() {
        let mut config = HardwareConfig::default();
        config.open(None);
        assert!(config.benchmark.is_none());
        let screen = config.screen.unwrap();
        assert_eq!(screen.draft, config.live);
        assert_eq!(screen.baseline, config.live);
    }

    #[test]
    fn opening_reads_the_settings_file_into_the_draft() {
        let path = temp_file(
            "open.ini",
            b"Supersample = 4\r\nShowFPS = 1\r\nVolume = 50\r\n",
        );
        let mut config = HardwareConfig::default().with_file(&path);
        config.open(None);
        let screen = config.screen.unwrap();
        assert_eq!(screen.draft.supersample, 4);
        // The live super-sampling goes back to the saved value; the frame
        // rate and volume keep the file's.
        assert_eq!(config.live.supersample, 0);
        assert!(config.live.show_fps);
        assert_eq!(config.live.volume, 0.5);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn apply_takes_the_changed_renderer_options_and_writes_the_lines() {
        let mut config = HardwareConfig::default();
        config.open(None);
        config.set_slider(Setting::Supersample, 1.0);
        config.set_slider(Setting::ShadowQuality, 2.0 / 3.0);
        config.set_slider(Setting::Multisample, 1.0);
        let applied = config.apply().expect("a screen was open");
        assert_eq!(config.live.supersample, 4);
        assert_eq!(config.live.shadow_quality, 3);
        // Hardware anti-aliasing is stored, not pushed; super-sampling was
        // raised first, so the row ignored the slider anyway.
        assert_eq!(config.live.multisample, 0);
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
    fn apply_does_nothing_when_the_file_cannot_be_read() {
        let path = temp_file("missing.ini", b"");
        let mut config = HardwareConfig::default().with_file(&path);
        config.open(None);
        config.set_slider(Setting::Supersample, 1.0);
        assert_eq!(config.apply(), None);
        assert!(config.screen.is_some());
        assert_eq!(config.live.supersample, 0);
        assert!(!path.exists());
    }

    #[test]
    fn cancel_pushes_the_saved_options_back() {
        let mut config = HardwareConfig::default();
        let live = config.live;
        config.open(None);
        config.set_slider(Setting::Supersample, 1.0);
        config.set_slider(Setting::TextureQuality, 0.0);
        config.set_slider(Setting::Volume, 0.2);
        // The texture and volume sliders are live, super-sampling is not.
        assert_eq!(config.live.texture_quality, 2);
        assert_eq!(config.live.supersample, live.supersample);
        assert_eq!(config.cancel(), Some(live));
        assert!(config.screen.is_none());
        assert_eq!(config.saved, live);
    }

    #[test]
    fn the_volume_and_textures_are_live_and_the_music_only_mutes_the_session() {
        let mut config = HardwareConfig::default();
        config.open(None);
        assert_eq!(config.set_slider(Setting::Volume, 0.5), Effect::Live);
        assert_eq!(config.live.volume, 0.75);
        assert_eq!(config.set_slider(Setting::TextureDetail, 0.0), Effect::Live);
        assert_eq!(config.live.texture_detail, 2);
        assert_eq!(
            config.set_toggle(Setting::Music, false),
            Effect::SessionOnly
        );
        assert!(!config.session_music);
        assert!(config.live.music);
        assert_eq!(config.set_toggle(Setting::WindowMode, false), Effect::Draft);
        assert!(config.live.window_mode);
        assert!(!config.screen.as_ref().unwrap().draft.window_mode);
        // A live checkbox changes only its own option.
        config.screen.as_mut().unwrap().draft.vertical_sync = false;
        assert_eq!(config.set_toggle(Setting::ShowTips, false), Effect::Live);
        assert!(!config.live.show_tips);
        assert!(config.live.vertical_sync);
    }

    #[test]
    fn defaults_benchmark_fills_the_saved_set_the_live_options_and_the_page() {
        let mut config = HardwareConfig::default();
        config.live.multisample = 4;
        config.open(None);
        config.defaults();
        assert!(config.benchmark.is_some());
        config.screen.as_mut().unwrap().draft.multisample = 8;
        config.finish_benchmark(4.0, 1600, 900);
        assert_eq!(config.live.perf_index, 4.0);
        assert_eq!(config.live.supersample, 3);
        assert_eq!(config.live.shading_quality, 5);
        assert_eq!(config.live.shadow_quality, 4);
        // The shadow detail reaches the page, not the live options.
        assert_eq!(config.live.shadow_detail, 4);
        // Hardware anti-aliasing is kept everywhere.
        assert_eq!(config.live.multisample, 4);
        let screen = config.screen.unwrap();
        assert_eq!(screen.draft.supersample, 3);
        assert_eq!(screen.draft.shadow_detail, 5);
        assert_eq!(screen.draft.multisample, 8);
        assert_eq!(screen.baseline.supersample, 3);
        // Apply then finds no quality change but the shadow detail's.
        let applied = config.apply().unwrap();
        assert!(!applied.quality.supersample);
        assert!(!applied.quality.shadow_detail);
        assert_eq!(applied.lines[6], ("ShadowDetail", "5".to_string()));
    }

    #[test]
    fn the_auto_detection_reuses_the_saved_window_mode() {
        let mut config = HardwareConfig::default();
        config.open(None);
        config.close();
        // Full screen since the page last opened.
        config.live.window_mode = false;
        config.start_benchmark();
        config.finish_benchmark(1.0, 1600, 900);
        assert!(config.live.window_mode);
        assert_eq!(config.saved.texture_quality, 3);
    }

    #[test]
    fn defaults_on_the_controls_page() {
        let mut bindings = Bindings::default();
        bindings.set_key(Action::Forward, 0x4C);
        let mut config = HardwareConfig::default();
        config.open(Some(&bindings));
        config.switch_page(true);
        config.defaults();
        assert!(config.benchmark.is_none());
        assert_eq!(config.bindings.as_ref().unwrap().key(Action::Forward), 0x57);
        // In controller mode it is the saved aim speed instead.
        config.set_controller(true);
        config.set_aim_speed(0.9);
        config.defaults();
        assert_eq!(config.live.aim_speed, 0.29);
    }

    #[test]
    fn the_controller_and_rumble_switches() {
        let mut config = HardwareConfig::default();
        assert!(!config.set_controller(true)); // no page open
        config.open(None);
        assert!(config.set_controller(true));
        assert!(config.live.controller);
        assert_eq!(config.lines().unwrap()[9], ("InputMode", "51".to_string()));
        config.set_rumble(false);
        assert!(!config.rumble);
    }

    #[test]
    fn the_deck_preset_is_fixed() {
        let mut config = HardwareConfig {
            steam_deck: true,
            ..HardwareConfig::default()
        };
        config.open(None);
        config.defaults();
        assert!(config.benchmark.is_none());
        assert_eq!(config.live.supersample, 3);
        assert_eq!(config.live.shading_quality, 5);
        assert_eq!(config.live.shadow_quality, 3);
    }

    #[test]
    fn a_rebinding_writes_the_page_copy_and_apply_commits_it() {
        let mut config = HardwareConfig::default();
        config.open(Some(&Bindings::default()));
        let mut editor = BindingEditor::default();
        let copy = config.bindings.as_mut().unwrap();
        assert!(editor.begin(0));
        assert_eq!(editor.key(0x4C, copy), Rebind::Assigned(0x4C));
        assert_eq!(copy.key(Action::Forward), 0x4C);
        assert_eq!(editor.key(0x4D, copy), Rebind::None);

        assert!(editor.begin(0));
        assert_eq!(editor.key(0x1B, copy), Rebind::Cancelled);
        assert_eq!(copy.key(Action::Forward), 0x4C);

        // The Power Tree row has a key cell too.
        assert!(editor.begin(21));
        assert_eq!(editor.key(0x4E, copy), Rebind::Assigned(0x4E));
        assert_eq!(copy.key(Action::PowerTree), 0x4E);

        let applied = config.apply().unwrap();
        let committed = applied.bindings.unwrap();
        assert_eq!(committed.key(Action::Forward), 0x4C);

        // A copy without a Forward key is not committed; Cancel drops it.
        config.open(Some(&Bindings::default()));
        config
            .bindings
            .as_mut()
            .unwrap()
            .set_key(Action::Forward, 0);
        assert_eq!(config.apply().unwrap().bindings, None);
        config.open(Some(&Bindings::default()));
        config.cancel();
        assert!(config.bindings.is_none());
    }

    #[test]
    fn the_binding_rows_name_every_key() {
        let bindings = Bindings::default();
        let rows = binding_rows(&bindings);
        assert_eq!(rows.len(), 22);
        assert_eq!(rows[0], ("Forward", "W".into()));
        assert_eq!(rows[1], ("Back", "S".into()));
        assert_eq!(rows[2], ("Left", "A".into()));
        assert_eq!(rows[6], ("Orbit Camera", "Middle mouse".into()));
        assert_eq!(rows[8], ("Toggle Combat", "Tab".into()));
        assert_eq!(rows[17], ("Use Power", "Q".into()));
        assert_eq!(rows[18], ("Power Bar", "C".into()));
        assert_eq!(rows[19], ("Previous Power", "Left".into()));
        assert_eq!(rows[20], ("Next Power", "Right".into()));
        assert_eq!(rows[21], ("Power Tree", "P".into()));
    }

    #[test]
    fn the_settings_file_gets_the_lines() {
        let path = temp_file("apply.ini", b"Supersample = 0\r\n");
        let mut config = HardwareConfig::default().with_file(&path);
        config.open(None);
        config.set_slider(Setting::Supersample, 1.0);
        let applied = config.apply().expect("open");
        assert!(applied.stored);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("Supersample = 4"));
        assert!(text.contains("PerfIndex = 0.00"));
        let _ = std::fs::remove_file(&path);
    }
}
