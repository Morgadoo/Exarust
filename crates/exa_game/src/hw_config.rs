//! The hardware configuration at run time: the settings screen's live and
//! draft options, Apply and Cancel, the GPU benchmark's driver, the renderer
//! and audio options the screen changes, and key rebinding.
//!
//! The pure rules are [`exa_formats::hw_config`]; the row behaviour is
//! [`exa_ui::settings`]. This module wires them to the running game: the
//! options become [`exa_render::quality`] resources, the scene's
//! super-sampling switch, the texture loader's settings, the audio buses
//! and the window's mode, size and present mode. Rules: `docs/UI.md` (Settings options, Controls) and
//! `docs/formats/SETTINGS.md`.

use std::path::PathBuf;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::{Monitor, PresentMode, PrimaryMonitor, PrimaryWindow, WindowMode};
use exa_assets::ModelCache;
use exa_assets::texture::TextureSettings;
use exa_audio::{AudioEnvironment, MusicSetting};
use exa_formats::file_ref::{read_whole_file, write_whole_file};
use exa_formats::hw_config::{Benchmark, Options, QualityChange, Screen};
use exa_formats::settings_ini;
use exa_render::quality::{MultisampleSetting, ShadingQuality, ShadowQuality};
use exa_render::supersample::SuperSampling;
use exa_ui::modal_screens::{CacheShaderScreen, KeyAnswer, ReadKeyScreen, ScreenStep, warm_row};
use exa_ui::settings::{self, Effect, Setting};

use crate::bindings::{self, Action, Bindings, DEFAULT_KEYS};

/// The performance counter's ticks per second the benchmark's clock uses.
pub const BENCHMARK_FREQUENCY: u64 = 1_000_000;
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
    /// Whether Steam Input drives the Controller page's tabs. Exarust has no
    /// Steam, so the Keyboard/Controller tabs stay hidden and the controller
    /// view is never shown (its rules are modelled all the same).
    pub steam_input: bool,
    /// The `ScreenWidth` and `ScreenHeight` settings: the full-screen display
    /// mode's size, 0 when unset ([`crate::window::fullscreen_mode`]).
    pub screen_size: (i32, i32),
    /// The Controls page's "Enable Rumble" checkbox. It has no saved setting
    /// in the original and no Exarust effect (no gamepad); kept as a session
    /// flag (its default is a guess).
    pub rumble: bool,
    /// Apply changed a renderer option that rebuilds the shaders: the
    /// "Applying new settings..." screen opens on the next frame.
    pub warm_pending: bool,
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
            steam_input: false,
            screen_size: (0, 0),
            rumble: true,
            warm_pending: false,
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
    /// Nothing is measured: the auto-detection runs only from Defaults.
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

    /// Shows the Settings (`false`) or Controls (`true`) page. The page
    /// switch only places the pages and tabs; it measures nothing and leaves
    /// the page's key copy alone (the auto-detection and the shipped keys are
    /// the Defaults button's, [`HardwareConfig::defaults`]).
    pub fn switch_page(&mut self, controls: bool) {
        if self.screen.is_some() {
            self.controls_page = controls;
        }
    }

    /// Whether the Controls page shows its controller view (the Controller
    /// tab lit): its Keyboard and Controller tabs need Steam Input, and
    /// without it the page switch always puts the keyboard view up. The
    /// Controller tab also turns the controller mode on
    /// ([`HardwareConfig::set_controller_view`]).
    pub fn controller_view(&self) -> bool {
        self.steam_input && self.live.controller
    }

    /// The Keyboard or Controller tab: it turns the controller mode and the
    /// controller view on (the Controller tab) or off (the Keyboard tab).
    pub fn set_controller_view(&mut self, controller: bool) -> bool {
        if self.screen.is_none() {
            return false;
        }
        self.live.controller = controller;
        true
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
        // Any of the four shader options opens the warm-up screen, as the
        // original's Apply does after handing each changed option over.
        self.warm_pending |= quality.rebuilds_shaders();
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
    /// shipped keys into the page's copy, or in the controller view (the
    /// Controller tab lit, which needs Steam Input) the saved aim speed back
    /// into the live one.
    pub fn defaults(&mut self) {
        if self.screen.is_none() {
            return;
        }
        if !self.controls_page {
            self.run_detection();
        } else if self.controller_view() {
            self.live.aim_speed = self.saved().aim_speed;
        } else if let Some(bindings) = &mut self.bindings {
            for action in Action::ALL {
                bindings.set_key(action, DEFAULT_KEYS[action.slot()]);
            }
        }
    }

    /// The auto-detection: the GPU benchmark, or the Deck's fixed preset
    /// instead. Both become the saved set and are pushed into the live
    /// options and the page's draft ([`HardwareConfig::finish_benchmark`],
    /// [`HardwareConfig::deck_preset`]).
    pub fn run_detection(&mut self) {
        if self.steam_deck {
            self.deck_preset();
        } else {
            self.start_benchmark();
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
    /// input mode Apply writes (Exarust has no gamepad to drive). It is also
    /// the Controller tab's action, which turns the controller view on.
    pub fn set_controller(&mut self, enabled: bool) -> bool {
        self.set_controller_view(enabled)
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

/// A rebinding in progress: the row whose key cell was clicked and the
/// key-capture screen it opened ([`ReadKeyScreen`]).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BindingEditor {
    /// The Controls row being rebound.
    pub row: Option<usize>,
    /// The open key-capture screen.
    pub screen: Option<ReadKeyScreen>,
}

/// The editor's answer to a key or button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rebind {
    /// The saved-table code was assigned to the row and the screen closed.
    Assigned(u32),
    /// The code is another row's: the screen shows "ALREADY IN USE" and
    /// stays open.
    InUse,
    /// Escape closed the screen.
    Cancelled,
    /// Nothing was assigned.
    None,
}

/// The Controls page's table as the original's key screen sees it: each
/// row's saved-table code ([`bindings::row_code`] of the slot it fills),
/// 0 for the row no slot reads.
pub fn page_rows(bindings: &Bindings) -> [u32; bindings::COUNT] {
    let mut rows = [0; bindings::COUNT];
    for (row, slot) in bindings::ROW_SLOTS.iter().enumerate() {
        if let Some(slot) = slot {
            rows[row] = bindings::row_code(bindings.keys()[*slot]);
        }
    }
    rows
}

impl BindingEditor {
    /// Opens the key-capture screen for the Controls row `row` at `now`,
    /// when the row has a key cell; the screen's action is the cell's
    /// control id.
    pub fn begin(&mut self, row: usize, now: u32) -> bool {
        match settings::BINDINGS.get(row) {
            Some(binding) if binding.keyed => {
                self.row = Some(row);
                self.screen = Some(ReadKeyScreen::new(u32::from(binding.action), now));
                true
            }
            _ => false,
        }
    }

    /// Closes the screen without a capture.
    pub fn cancel(&mut self) -> bool {
        self.screen = None;
        self.row.take().is_some()
    }

    /// Applies the screen's answer: a captured code goes into the row's
    /// slot of `bindings` (the page's copy) through the original's
    /// conversion ([`bindings::slot_code`]) and closes the screen; Escape
    /// closes it.
    fn answer(&mut self, answer: KeyAnswer, bindings: &mut Bindings) -> Rebind {
        match answer {
            KeyAnswer::Ignored => Rebind::None,
            KeyAnswer::InUse => Rebind::InUse,
            KeyAnswer::Closed => {
                self.cancel();
                Rebind::Cancelled
            }
            KeyAnswer::Captured(code) => {
                let slot = self
                    .screen
                    .and_then(|screen| (screen.action as usize).checked_sub(1))
                    .and_then(|row| bindings::ROW_SLOTS.get(row).copied().flatten());
                if let Some(action) = slot.and_then(action_at) {
                    bindings.set_key(action, bindings::slot_code(code));
                }
                self.cancel();
                Rebind::Assigned(code)
            }
        }
    }

    /// A key for the open screen: `code` is its virtual-key code (Escape
    /// is `0x1b`) and `state` the window's key-state word
    /// ([`bindings::key_state_word`]); see [`ReadKeyScreen::key`].
    pub fn key(&mut self, code: u32, state: u32, bindings: &mut Bindings, now: u32) -> Rebind {
        let Some(mut screen) = self.screen else {
            return Rebind::None;
        };
        let mut rows = page_rows(bindings);
        let answer = screen.key(&mut rows, code, 0, state, now);
        self.screen = Some(screen);
        self.answer(answer, bindings)
    }

    /// A mouse button for the open screen, as the window's button bit
    /// ([`bindings::mouse_hook_bit`]); see [`ReadKeyScreen::mouse`].
    pub fn mouse(&mut self, bit: u32, bindings: &mut Bindings, now: u32) -> Rebind {
        let Some(mut screen) = self.screen else {
            return Rebind::None;
        };
        let mut rows = page_rows(bindings);
        let answer = screen.mouse(&mut rows, bit, now);
        self.screen = Some(screen);
        self.answer(answer, bindings)
    }

    /// The screen's per-frame step, while it is open.
    pub fn step(&mut self, now: u32) -> Option<ScreenStep> {
        self.screen.as_mut().map(|screen| screen.step(now))
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

/// Apply committed the Controls page's bindings: the running game has
/// taken them, and the host saves them as the original saves its binding
/// file at once.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct BindingsApplied(pub Bindings);

/// The hardware configuration's plugin.
pub struct HardwareConfigPlugin;

impl Plugin for HardwareConfigPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BindingEditor>()
            .init_resource::<ShaderWarmScreen>()
            .add_message::<BindingsApplied>()
            .add_systems(
                Update,
                (
                    rebind_keys,
                    drive_warm_screen,
                    drive_benchmark,
                    apply_options,
                    apply_window,
                    start_benchmark_scene,
                    show_benchmark_quads,
                    cleanup_benchmark,
                )
                    .chain(),
            );
    }
}

/// Drives the open key-capture screen: its per-frame fade and warning,
/// then a mouse button (the middle and extra buttons) or a key into the
/// Controls page's copy of the bindings.
pub fn rebind_keys(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time<Real>>,
    mut editor: ResMut<BindingEditor>,
    config: Option<ResMut<HardwareConfig>>,
) {
    let Some(mut config) = config else {
        return;
    };
    if editor.screen.is_none() {
        return;
    }
    let Some(bindings) = config.bindings.as_mut() else {
        // No form is open: nothing to edit.
        editor.cancel();
        return;
    };
    let now = time.elapsed().as_millis() as u32;
    editor.step(now);
    for button in buttons.get_just_pressed() {
        if let Some(bit) = bindings::mouse_hook_bit(*button)
            && editor.mouse(bit, bindings, now) != Rebind::None
        {
            return;
        }
    }
    for key in keys.get_just_pressed() {
        let Some(code) = bindings::virtual_key(*key) else {
            continue;
        };
        if editor.key(code, bindings::key_state_word(*key), bindings, now) != Rebind::None {
            return;
        }
    }
}

/// The shader warm-up screen ("Applying new settings...") while it is
/// open.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShaderWarmScreen {
    pub screen: Option<CacheShaderScreen>,
}

/// Opens the warm-up screen when Apply asked for it and steps it once a
/// frame: its fades and one row of the renderer's 29-row warm-up table a
/// frame ([`exa_ui::modal_screens::warm_row`]; Bevy builds the pipelines
/// themselves when they are first drawn), closing it at the end of its
/// fade-out.
pub fn drive_warm_screen(
    time: Res<Time<Real>>,
    mut warm: ResMut<ShaderWarmScreen>,
    config: Option<ResMut<HardwareConfig>>,
) {
    let now = time.elapsed().as_millis() as u32;
    if let Some(mut config) = config
        && config.warm_pending
    {
        config.warm_pending = false;
        warm.screen = Some(CacheShaderScreen::new(now));
    }
    let Some(screen) = warm.screen.as_mut() else {
        return;
    };
    if screen.step(now, warm_row).close {
        warm.screen = None;
    }
}

/// The texture settings the live options give: `base` (its driver limits)
/// with the texture quality's filter mask, anisotropy and LOD bias base,
/// the texture detail and the super-sampling level the LOD bias follows, the
/// globals the original's handlers write. A texture decoded afterwards takes
/// them; one already decoded keeps its own.
pub fn texture_settings(base: TextureSettings, live: &Options) -> TextureSettings {
    TextureSettings {
        texture_detail: live.texture_detail.max(0) as u32,
        supersample: live.supersample,
        ..base.with_quality(live.texture_quality)
    }
}

/// Pushes changed live options into the renderer, the texture loader, the
/// audio and the window.
#[allow(clippy::too_many_arguments)]
fn apply_options(
    config: Option<Res<HardwareConfig>>,
    mut models: Option<ResMut<ModelCache>>,
    mut materials: Option<ResMut<Assets<exa_render::material::ExaMaterial>>>,
    mut sampling: ResMut<SuperSampling>,
    mut shadow: ResMut<ShadowQuality>,
    mut shading: ResMut<ShadingQuality>,
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
    let mut bias = None;
    if let Some(models) = models.as_mut() {
        let textures = texture_settings(*models.textures.settings(), &live);
        if *models.textures.settings() != textures {
            models.textures.set_settings(textures);
            bias = Some(textures.lod_bias());
        }
    }
    // The original sets the LOD bias from the quality's global on every
    // bind, so a change also applies to the materials already loaded; the
    // sampler filters stay those each texture was uploaded with.
    if let (Some(bias), Some(materials)) = (bias, materials.as_mut()) {
        exa_render::material::set_texture_bias(materials, bias);
    }
    if shadow.0 != live.shadow_quality {
        shadow.0 = live.shadow_quality;
    }
    if shading.0 != live.shading_quality {
        shading.0 = live.shading_quality;
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

/// The `SteamDeck=1` marker the Steam runtime exports, or the Deck's DMI
/// product names.
pub fn steam_deck_from(marker: Option<&str>, product: Option<&str>) -> bool {
    if marker == Some("1") {
        return true;
    }
    matches!(product, Some("Jupiter") | Some("Galileo"))
}

/// Whether the game runs on a Steam Deck: the Steam runtime's `SteamDeck`
/// marker, or the DMI product name (`Jupiter` or `Galileo`). The original
/// asks Steam; Exarust reads what the machine exposes.
pub fn steam_deck() -> bool {
    let marker = std::env::var("SteamDeck").ok();
    let product = std::fs::read_to_string("/sys/devices/virtual/dmi/id/product_name")
        .ok()
        .map(|name| name.trim().to_string());
    steam_deck_from(marker.as_deref(), product.as_deref())
}

/// The video memory the driver reports, in MiB: the largest
/// `mem_info_vram_total` among the DRM cards (the amdgpu driver's figure),
/// or 0 when the machine exposes none. The original asks its GL driver for
/// the figure; wgpu has no portable query.
pub fn video_memory_mb() -> u32 {
    vram_from(std::path::Path::new("/sys/class/drm"))
}

/// [`video_memory_mb`] over a sysfs root: each `card*` entry's
/// `device/mem_info_vram_total` in bytes, divided by 2²⁰, the largest taken.
fn vram_from(root: &std::path::Path) -> u32 {
    let Ok(entries) = std::fs::read_dir(root) else {
        return 0;
    };
    let mut best = 0u64;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path().join("device/mem_info_vram_total"))
        else {
            continue;
        };
        if let Ok(bytes) = text.trim().parse::<u64>() {
            best = best.max(u64::from(vram_mb_from_bytes(bytes)));
        }
    }
    best.min(u64::from(u32::MAX)) as u32
}

/// The video-memory readout's number: the bytes of a `mem_info_vram_total`
/// file in MiB, or 0.
pub fn vram_mb_from_bytes(bytes: u64) -> u32 {
    (bytes / (1024 * 1024)).min(u64::from(u32::MAX)) as u32
}

/// The window state a frame last applied: windowed, size, vertical sync and
/// whether the primary monitor was known.
type AppliedWindow = (bool, Option<(u32, u32)>, bool, bool);
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
    let monitor = monitors.single().ok();
    let state = (
        config.live.window_mode,
        config.window_size,
        config.live.vertical_sync,
        monitor.is_some(),
    );
    if applied.as_ref() == Some(&state) {
        return;
    }
    *applied = Some(state);
    // Full screen covers the desktop, and changes the display mode only for
    // a `ScreenWidth`/`ScreenHeight` other than the desktop's.
    let mode = if config.live.window_mode {
        WindowMode::Windowed
    } else {
        let desktop = monitor.map(|m| m.physical_size());
        let modes = monitor.map(|m| m.video_modes.as_slice()).unwrap_or(&[]);
        if crate::window::display_mode_refused(config.screen_size, desktop, modes) {
            let (title, text) = crate::window::DISPLAY_MODE_REFUSED;
            warn!("{}", crate::frame::overlay_text(title, text));
        }
        crate::window::fullscreen_mode(
            config.screen_size,
            monitor.map(|m| m.physical_size()),
            monitor.map(|m| m.video_modes.as_slice()).unwrap_or(&[]),
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
    let present =
        crate::window::present_mode(config.live.vertical_sync, crate::window::ADAPTIVE_VSYNC);
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
    mesh: Handle<Mesh>,
    material: Handle<exa_render::benchmark::BenchmarkMaterial>,
    present_mode: PresentMode,
}

const BENCHMARK_CAMERA: &str = "_BenchmarkCamera";
const BENCHMARK_QUAD: &str = "_BenchmarkQuad";

/// The source texture's sampler: the original sets its texture's wrap to
/// repeat and its filters to linear.
fn benchmark_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    })
}

/// Starts the benchmark's scene when a benchmark begins: a 1024² target
/// (the original's RGB8, RGBA8 here), the original's 512² source texture (every byte 0x3f) and sixteen
/// full-screen quads carrying its own fragment program, with vertical sync
/// off so the frames are not capped. The quads share one depth: the 2D
/// opaque pass tests greater-or-equal, so each of them is shaded, as the
/// original's always-pass depth function shades each (staggered depths
/// would let the depth test reject all but the nearest). The measured
/// protocol and score are exact; the GPU and its driver differ anyway.
fn start_benchmark_scene(
    mut commands: Commands,
    config: Option<Res<HardwareConfig>>,
    scene: Option<Res<BenchmarkScene>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::benchmark::BenchmarkMaterial>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    use exa_render::benchmark::{BenchmarkMaterial, QUADS, SOURCE_SIZE, TARGET_SIZE};

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
            width: TARGET_SIZE,
            height: TARGET_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        // The original's GL_RGB8, a plain (not sRGB) format; wgpu has no
        // three-channel one, so the alpha is padding.
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    target.texture_descriptor.usage =
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    let target = images.add(target);

    let mut source = Image::new(
        Extent3d {
            width: SOURCE_SIZE,
            height: SOURCE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        exa_render::benchmark::source_pixels(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    source.sampler = benchmark_sampler();
    let source = images.add(source);

    let mesh = meshes.add(Rectangle::new(TARGET_SIZE as f32, TARGET_SIZE as f32));
    let material = materials.add(BenchmarkMaterial::new(source.clone()));

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
    let mut quads = Vec::with_capacity(QUADS);
    for index in 0..QUADS {
        let quad = commands
            .spawn((
                Name::new(BENCHMARK_QUAD),
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material.clone()),
                Transform::IDENTITY,
                benchmark_quad_visibility(index, false),
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
        mesh,
        material,
        present_mode,
    });
}

/// Whether the benchmark's quad `index` is drawn: the original draws
/// [`exa_render::benchmark::WARMUP_QUADS`] quads a warm-up frame and all
/// [`exa_render::benchmark::QUADS`] a measured one.
fn benchmark_quad_visibility(index: usize, running: bool) -> Visibility {
    let drawn = if running {
        exa_render::benchmark::QUADS
    } else {
        exa_render::benchmark::WARMUP_QUADS
    };
    if index < drawn {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

/// Shows the measured runs' sixteen quads once the warm-up is over.
fn show_benchmark_quads(
    config: Option<Res<HardwareConfig>>,
    scene: Option<Res<BenchmarkScene>>,
    mut quads: Query<&mut Visibility>,
) {
    let (Some(config), Some(scene)) = (config, scene) else {
        return;
    };
    let Some(benchmark) = &config.benchmark else {
        return;
    };
    let running = benchmark.running();
    for (index, quad) in scene.quads.iter().enumerate() {
        if let Ok(mut visibility) = quads.get_mut(*quad) {
            visibility.set_if_neq(benchmark_quad_visibility(index, running));
        }
    }
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::benchmark::BenchmarkMaterial>>,
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
    meshes.remove(&scene.mesh);
    materials.remove(&scene.material);
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
    fn the_live_texture_options_become_the_texture_settings() {
        let base = TextureSettings::default();
        let mut live = Options {
            texture_quality: 4,
            texture_detail: 2,
            supersample: 3,
            ..HardwareConfig::default().live
        };
        let settings = texture_settings(base, &live);
        assert_eq!(settings.quality_mask, 0xF0);
        assert_eq!(settings.anisotropy_level, 2);
        assert_eq!(settings.lod_bias_base, -0.5);
        assert_eq!(settings.texture_detail, 2);
        assert_eq!(settings.supersample, 3);
        // The driver limits stay the base's.
        assert_eq!(settings.max_texture_size, base.max_texture_size);
        assert_eq!(settings.max_units, base.max_units);
        // The retail user's options give the retail settings.
        live.texture_quality = 5;
        live.texture_detail = 3;
        live.supersample = 4;
        assert_eq!(texture_settings(base, &live), base);
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
        assert!(config.warm_pending);
        assert_eq!(config.live.supersample, 4);
        assert_eq!(config.live.shadow_quality, 3);
        // Hardware anti-aliasing is stored, not pushed: the draft's 8
        // samples reach the file only.
        assert_eq!(config.live.multisample, 0);
        assert_eq!(applied.lines[8], ("HWMultisample", "8".to_string()));
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
        // Without Steam Input the controller mode alone does not make the
        // controller view: Defaults still resets the keys.
        config.set_controller(true);
        config
            .bindings
            .as_mut()
            .unwrap()
            .set_key(Action::Forward, 0x4C);
        config.set_aim_speed(0.9);
        config.defaults();
        assert_eq!(config.bindings.as_ref().unwrap().key(Action::Forward), 0x57);
        assert_eq!(config.live.aim_speed, 0.9);
        // In the controller view it is the saved aim speed instead.
        let mut config = HardwareConfig {
            steam_input: true,
            ..HardwareConfig::default()
        };
        config.open(Some(&bindings));
        config.switch_page(true);
        config.set_controller_view(true);
        config.set_aim_speed(0.9);
        config.defaults();
        assert_eq!(config.live.aim_speed, 0.29);
        assert_eq!(config.bindings.as_ref().unwrap().key(Action::Forward), 0x4C);
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
    fn a_page_switch_measures_nothing_and_keeps_the_key_copy() {
        let mut bindings = Bindings::default();
        bindings.set_key(Action::Forward, 0x4C);
        let mut config = HardwareConfig::default();
        config.open(Some(&bindings));
        config.switch_page(true);
        config.switch_page(false);
        assert!(config.benchmark.is_none());
        config.switch_page(true);
        assert_eq!(config.bindings.as_ref().unwrap().key(Action::Forward), 0x4C);
        // Without Steam Input there is no controller view.
        config.set_controller_view(true);
        assert!(!config.controller_view());
        let mut steam = HardwareConfig {
            steam_input: true,
            ..HardwareConfig::default()
        };
        steam.open(None);
        steam.set_controller_view(true);
        assert!(steam.controller_view());
    }

    #[test]
    fn the_steam_deck_and_video_memory_probes() {
        assert!(steam_deck_from(Some("1"), None));
        assert!(steam_deck_from(None, Some("Jupiter")));
        assert!(steam_deck_from(None, Some("Galileo")));
        assert!(!steam_deck_from(Some("0"), Some("B650")));
        assert!(!steam_deck_from(None, None));
        assert_eq!(vram_mb_from_bytes(8 * 1024 * 1024 * 1024), 8192);
        assert_eq!(vram_mb_from_bytes(0), 0);
        assert_eq!(vram_mb_from_bytes(1023), 0);
        // The sysfs walk takes each card's figure, the largest winning, and
        // reads nothing from the connectors' names.
        let root = std::env::temp_dir().join(format!("exarust-vram-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (card, bytes) in [("card0", 4_294_967_296u64), ("card1", 8_589_934_592)] {
            let device = root.join(card).join("device");
            std::fs::create_dir_all(&device).unwrap();
            std::fs::write(device.join("mem_info_vram_total"), bytes.to_string()).unwrap();
        }
        let connector = root.join("card0-DP-1").join("device");
        std::fs::create_dir_all(&connector).unwrap();
        std::fs::write(connector.join("mem_info_vram_total"), "999").unwrap();
        assert_eq!(vram_from(&root), 8192);
        assert_eq!(vram_from(&root.join("missing")), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_warm_up_draws_four_quads_and_a_run_sixteen() {
        let drawn = |running| {
            (0..exa_render::benchmark::QUADS)
                .filter(|index| benchmark_quad_visibility(*index, running) != Visibility::Hidden)
                .count()
        };
        assert_eq!(drawn(false), 4);
        assert_eq!(drawn(true), 16);
    }

    #[test]
    fn the_benchmark_samples_its_texture_with_repeat_addressing() {
        let ImageSampler::Descriptor(sampler) = benchmark_sampler() else {
            panic!("a descriptor sampler");
        };
        assert_eq!(sampler.address_mode_u, ImageAddressMode::Repeat);
        assert_eq!(sampler.address_mode_v, ImageAddressMode::Repeat);
        assert_eq!(sampler.mag_filter, ImageFilterMode::Linear);
        assert_eq!(sampler.min_filter, ImageFilterMode::Linear);
    }

    #[test]
    fn a_rebinding_writes_the_page_copy_and_apply_commits_it() {
        let mut config = HardwareConfig::default();
        config.open(Some(&Bindings::default()));
        let mut editor = BindingEditor::default();
        let copy = config.bindings.as_mut().unwrap();
        assert!(editor.begin(0, 0));
        assert_eq!(editor.screen.unwrap().action, 1);
        // L: scan 0x26, read back as its virtual-key code.
        let l = bindings::key_state_word(KeyCode::KeyL);
        assert_eq!(l, 0x26 << 16);
        assert_eq!(editor.key(0x4C, l, copy, 10), Rebind::Assigned(0x26));
        assert_eq!(copy.key(Action::Forward), 0x4C);
        assert!(editor.screen.is_none());
        assert_eq!(editor.key(0x4D, l, copy, 10), Rebind::None);

        assert!(editor.begin(0, 20));
        assert_eq!(editor.key(0x1B, 1 << 16, copy, 30), Rebind::Cancelled);
        assert_eq!(copy.key(Action::Forward), 0x4C);

        // The Power Tree row (control 0x12, saved row 17, slot 19). S is
        // Back's: the screen warns and stays open, then takes N.
        assert!(editor.begin(21, 40));
        let s = bindings::key_state_word(KeyCode::KeyS);
        assert_eq!(editor.key(0x53, s, copy, 50), Rebind::InUse);
        assert_eq!(editor.screen.unwrap().prompt(), "ALREADY IN USE");
        let n = bindings::key_state_word(KeyCode::KeyN);
        assert_eq!(editor.key(0x4E, n, copy, 60), Rebind::Assigned(0x31));
        assert_eq!(copy.key(Action::PowerTree), 0x4E);

        // The middle button is Orbit's; the extra button binds as Mouse 4.
        assert!(editor.begin(18, 70));
        assert_eq!(editor.mouse(1, copy, 80), Rebind::None);
        assert_eq!(editor.mouse(4, copy, 80), Rebind::InUse);
        assert_eq!(editor.mouse(8, copy, 80), Rebind::Assigned(0x8005));
        assert_eq!(copy.key(Action::PowerBar), 5);

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
    fn a_shader_change_opens_the_warm_up_screen_until_its_fade_out() {
        let mut app = App::new();
        app.insert_resource(Time::<Real>::default())
            .init_resource::<ShaderWarmScreen>()
            .insert_resource(HardwareConfig {
                warm_pending: true,
                ..HardwareConfig::default()
            })
            .add_systems(Update, drive_warm_screen);
        app.update();
        let screen = app.world().resource::<ShaderWarmScreen>().screen.unwrap();
        assert!(!app.world().resource::<HardwareConfig>().warm_pending);
        assert_eq!(
            screen.phase,
            exa_ui::modal_screens::WarmScreenPhase::FadingIn
        );
        // With the clock standing still the fade never ends: the screen
        // stays open, as the original's would.
        app.update();
        assert!(app.world().resource::<ShaderWarmScreen>().screen.is_some());
        // Without a page open, Apply does nothing and nothing is pending.
        let mut config = HardwareConfig::default();
        assert_eq!(config.apply(), None);
        assert!(!config.warm_pending);
    }

    #[test]
    fn the_binding_rows_name_every_key() {
        let bindings = Bindings::default();
        let rows = binding_rows(&bindings);
        assert_eq!(rows.len(), 22);
        assert_eq!(rows[0], ("Forward", "W".into()));
        assert_eq!(rows[1], ("Back", "S".into()));
        assert_eq!(rows[2], ("Left", "A".into()));
        assert_eq!(rows[6], ("Orbit Camera", "Mouse 3".into()));
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
