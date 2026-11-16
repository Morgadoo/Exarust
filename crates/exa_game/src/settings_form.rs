//! The running game's Settings form: the menu's SETTINGS rows open it over
//! the menu (the title, in-game and pause pages alike), it is drawn through
//! the GUI draw path and takes the pointer and keys until it closes.
//!
//! The form's records, layout, page switch and control answers are
//! [`exa_ui::settings::FormState`]'s; the options it edits are
//! [`HardwareConfig`]'s: opening opens the configuration's page and fills
//! the rows, a slider or checkbox goes through its step, a key cell opens
//! the key-read screen ([`BindingEditor`]), Defaults runs the benchmark or
//! puts the shipped keys back, Apply writes the settings file and commits
//! the bindings (opening the shader warm-up screen when a shader option
//! changed), Cancel pushes the saved options back. Hiding the menu closes
//! the form, as the original's menu hide does; the close keeps the live
//! changes and drops the draft. Escape closing the form is Exarust's (the
//! original's form has no key for it). The key-read and warm-up screens
//! draw their text here, over a black backdrop at their fade.
//!
//! `--open-settings` opens the menu and the form once the load ends, for
//! scripted checks. Rules: `docs/UI.md` (Settings form).

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ui::gui_controls::{CheckLabelInk, Control, check_label_ink, label_x, state};
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{GuiFrames, GuiLayer, GuiTextures, LayerTexture, SKIN_TEXTURE, text_batch};
use exa_ui::gui_font::GuiFont;
use exa_ui::line_skin::{hover_alpha, line_control_inks, shown_palette};
use exa_ui::modal_screens::ScreenText;
use exa_ui::settings::{
    self, CHILD_ATTRIBUTES, CONTROLS_ATTRIBUTES, CONTROLS_STYLE, FormAction, FormEvent, FormState,
    HIDDEN, HOVER_BITS, HOVERED, Part, SEPARATOR_CLASS,
};
use exa_ui::skin::{
    GuiRect as SkinRect, SkinBatch, SkinInk, SkinPalette, SliderPlace, check_box_quads,
    draw_flat_slider, edge_frame, line_skin_batch, push_line_quads,
};

use crate::bindings::Bindings;
use crate::gui_paint::{self, Painted};
use crate::hw_config::{
    BindingEditor, BindingsApplied, HardwareConfig, ShaderWarmScreen, binding_rows,
};
use crate::menu_pages::MenuCommand;
use crate::menu_screen::{MenuChoice, MenuScreen, MenuScreenSystems};
use crate::pad_menu::PadForm;

/// The open Settings form.
#[derive(Resource, Debug, Default)]
pub struct SettingsForm {
    pub form: Option<FormState>,
    /// The Controls page as a held base form with its scroll bar, which
    /// the pointer drives ([`crate::form_input`]): its arrows, thumb, rail
    /// and the wheel move the page's fraction.
    pub page: Option<(crate::form_input::FormTask, exa_ui::gui_core::FormId)>,
    /// Defaults started the benchmark: the rows refill when it ends.
    benchmarking: bool,
}

impl SettingsForm {
    pub fn is_open(&self) -> bool {
        self.form.is_some()
    }
}

/// `--open-settings`: open the menu and the form on this page (0 Settings,
/// 1 Controls) once the load has ended.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OpenSettingsOnLoad {
    pub page: Option<u8>,
    pub done: bool,
}

/// The form's plugin.
pub struct SettingsFormPlugin;

impl Plugin for SettingsFormPlugin {
    fn build(&self, app: &mut App) {
        crate::gui_keys::ensure(app);
        app.init_resource::<SettingsForm>()
            .init_resource::<OpenSettingsOnLoad>()
            .init_resource::<BindingEditor>()
            .init_resource::<ShaderWarmScreen>()
            .init_resource::<GuiFrames>()
            .add_message::<MenuChoice>()
            .add_message::<MouseWheel>()
            .add_message::<BindingsApplied>()
            .add_systems(
                Update,
                (
                    open_settings
                        .after(draw)
                        .after(crate::creator_screen::input),
                    form_input.after(crate::arsenal_view::arsenal_input),
                    refresh_rows,
                    close_with_menu,
                )
                    .chain()
                    .after(MenuScreenSystems)
                    .before(crate::hw_config::rebind_keys)
                    .in_set(exa_physics::FramePhase::Gui),
            )
            .add_systems(
                Update,
                draw.after(crate::creator_screen::open_screens)
                    .after(MenuScreenSystems)
                    .in_set(exa_physics::FramePhase::Gui),
            );
    }
}

/// The window's physical size and the GUI unit for its height.
fn screen_and_unit(window: &Window) -> ((i32, i32), i32) {
    let screen = (
        window.physical_width() as i32,
        window.physical_height() as i32,
    );
    let unit = exa_ui::scale::gui_scale(window.physical_height(), 0) as i32;
    (screen, unit)
}

/// Opens the form for the menu's SETTINGS rows (and once for
/// `--open-settings`): the configuration's page opens with a copy of the
/// bindings, and the rows, key cells and device readout are filled.
#[allow(clippy::too_many_arguments)]
fn open_settings(
    mut choices: MessageReader<MenuChoice>,
    mut on_load: ResMut<OpenSettingsOnLoad>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut menu: Option<ResMut<MenuScreen>>,
    mut flow: Option<ResMut<crate::flow::Flow>>,
    mut settings: ResMut<SettingsForm>,
    config: Option<ResMut<HardwareConfig>>,
    bindings: Option<Res<Bindings>>,
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let mut wanted = choices
        .read()
        .any(|MenuChoice(command)| *command == MenuCommand::OpenSettings);
    let mut page = 0;
    if let Some(p) = on_load.page
        && !on_load.done
        && loading.is_none()
    {
        on_load.done = true;
        wanted = true;
        page = p;
        if let Some(menu) = menu.as_mut()
            && !menu.open
        {
            crate::menu_screen::open_menu(menu, flow.as_deref_mut());
        }
    }
    if !wanted || settings.is_open() {
        return;
    }
    let Some(mut config) = config else {
        warn!("settings: no hardware configuration (a launch with --no-settings)");
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let (screen, unit) = screen_and_unit(window);
    let fallback = Bindings::default();
    config.open(Some(bindings.as_deref().unwrap_or(&fallback)));
    let mut form = FormState::open(screen, unit);
    form.controller_tabs = config.steam_input;
    form.set_device(
        adapter.as_deref().map(|info| info.name.as_str()),
        config.steam_deck,
        crate::hw_config::video_memory_mb(),
    );
    if let Some(rows) = config.rows() {
        let events = form.fill(&rows);
        apply_rows(&mut form, &mut config, &events);
    }
    if let Some(copy) = &config.bindings {
        form.set_keys(&key_names(copy));
    }
    if page == 1 {
        form.switch(1);
        config.switch_page(true);
    }
    info!("settings: the form opens at {screen:?}, unit {unit}");
    settings.form = Some(form);
    settings.benchmarking = false;
}

/// The key cells' names, in the Controls rows' order.
fn key_names(bindings: &Bindings) -> Vec<String> {
    binding_rows(bindings)
        .into_iter()
        .map(|(_, name)| name)
        .collect()
}

/// The filling's own events: the anti-aliasing row's set of 0 while
/// super-sampling is on writes the draft, and its label follows.
fn apply_rows(form: &mut FormState, config: &mut HardwareConfig, events: &[FormEvent]) {
    for event in events {
        if let FormEvent::Slider(setting, fraction) = *event {
            config.set_slider(setting, fraction);
            if let Some(screen) = config.screen {
                form.set_text(setting, &screen.draft);
            }
        }
    }
}

/// Drives the Controls page's base form: built over the page's place and
/// client with its committed words (the panel frame and the scroll bar),
/// the form's fraction handed in and read back. The page's records keep
/// the form's own hit and press ([`FormState`]); the base form takes the
/// frame codes (the arrows and their repeat, the thumb and the rail) and
/// the wheel, which goes to the form under the pointer.
#[allow(clippy::too_many_arguments)]
fn drive_controls_page(
    page: &mut Option<(crate::form_input::FormTask, exa_ui::gui_core::FormId)>,
    form: &mut FormState,
    window: &Window,
    scripted: Option<&exa_ui::ScriptedPointer>,
    mouse: &ButtonInput<MouseButton>,
    wheel: i32,
    now: u32,
    timers: Option<&mut crate::frame_timers::FrameTimers>,
) {
    if form.page != 1 {
        *page = None;
        form.page_hover = 0;
        return;
    }
    let rect = form.page_screen();
    let bounds = GuiRect::new(rect.x, rect.y, rect.w, rect.h);
    let unit = form.unit();
    let client = form.page_client();
    let local = exa_ui::form::ClientRect {
        left: client.x - rect.x,
        top: client.y - rect.y,
        right: client.x - rect.x + client.w,
        bottom: client.y - rect.y + client.h,
    };
    let rebuild = page.as_ref().is_none_or(|(task, id)| {
        task.base(*id)
            .is_none_or(|f| f.bounds != bounds || f.unit != unit || f.client != local)
    });
    if rebuild {
        let held = gui_paint::commit_words(CONTROLS_STYLE, CONTROLS_ATTRIBUTES, false);
        let mut base = exa_ui::base_form::BaseForm::new(
            bounds,
            unit,
            (held.style, held.attributes, held.state),
            local,
        );
        let g = &form.geometry;
        let owner_x = g.form.x + settings::BORDER;
        let owner_y = g.form.y + settings::BORDER;
        base.in_owner = (rect.x - owner_x, rect.y - owner_y);
        base.owner_client = (
            g.form.w - 2 * settings::BORDER,
            g.form.h - 2 * settings::BORDER,
        );
        base.scroll = Some(exa_ui::base_form::FormScroll {
            fraction: form.scroll,
            step: settings::SCROLL_STEP,
            content: g.controls_content,
        });
        let mut task = crate::form_input::FormTask::new();
        let id = task.add(base, None);
        *page = Some((task, id));
    }
    let Some((task, id)) = page.as_mut() else {
        return;
    };
    if let Some(scroll) = task.base_mut(*id).and_then(|f| f.scroll.as_mut()) {
        scroll.fraction = form.scroll;
    }
    let frame = crate::form_input::read_pointer(window, scripted, mouse, wheel, now);
    task.drive(&frame, timers);
    for event in task.take_events(*id) {
        if let exa_ui::base_form::BaseEvent::Scrolled(f) = event {
            info!("settings: Controls page scrolled to {f:.3}");
        }
    }
    if let Some(base) = task.base(*id) {
        form.scroll = base.scroll.map_or(form.scroll, |s| s.fraction);
        form.page_hover = base.hover;
    }
}

/// The pointer, wheel and keys while the form is open and neither the
/// key-read nor the warm-up screen covers it.
#[allow(clippy::too_many_arguments)]
fn form_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    time: Res<Time<Real>>,
    mut settings: ResMut<SettingsForm>,
    mut editor: ResMut<BindingEditor>,
    warm: Res<ShaderWarmScreen>,
    config: Option<ResMut<HardwareConfig>>,
    mut bindings: Option<ResMut<Bindings>>,
    mut applied: MessageWriter<BindingsApplied>,
    mut timers: Option<ResMut<crate::frame_timers::FrameTimers>>,
    mut pad: Option<ResMut<crate::pad_menu::PadMenu>>,
    (gui_keys, mut climbed): (
        Option<Res<crate::gui_keys::GuiKeyFrame>>,
        Option<ResMut<crate::gui_keys::TaskKeys>>,
    ),
) {
    let notches: f32 = wheel
        .read()
        .map(|w| match w.unit {
            MouseScrollUnit::Line => w.y * 120.0,
            MouseScrollUnit::Pixel => w.y,
        })
        .sum();
    let Some(mut config) = config else {
        return;
    };
    let Some(mut form) = settings.form.take() else {
        return;
    };
    if editor.is_changed()
        && let Some(copy) = &config.bindings
    {
        form.set_keys(&key_names(copy));
    }
    let covered = editor.screen.is_some() || warm.screen.is_some();
    let window = windows.single().ok();
    let mut events = Vec::new();
    let mut keep = true;
    if let (false, Some(window)) = (covered, window) {
        let (screen, unit) = screen_and_unit(window);
        if screen != form.screen || unit != form.unit() {
            form.resize(screen, unit);
        }
        // The form's key slot is the base form's: no record of the form
        // takes a key (the key focus a press gives is not modelled), so a
        // key-down climbs to the task's shortcuts. Escape closing the form
        // is Exarust's and takes the key.
        let escape = match gui_keys.as_deref() {
            Some(frame) => {
                let mut escape = false;
                for key in frame.key_downs() {
                    if key.key == 0x1b && key.flags & exa_ui::gui_keys::REPEAT == 0 {
                        escape = true;
                    } else if key.flags & 3 == 0
                        && let Some(climbed) = climbed.as_deref_mut()
                    {
                        climbed.forward(key);
                    }
                }
                escape
            }
            None => keys.just_pressed(KeyCode::Escape),
        };
        if escape {
            config.close();
            keep = false;
        } else {
            let pointer = match scripted.as_deref() {
                Some(scripted) => scripted.position(window),
                None => window.cursor_position(),
            };
            if let Some(at) = pointer {
                let x = (at.x * window.scale_factor()) as i32;
                let y = (at.y * window.scale_factor()) as i32;
                events.extend(form.pointer(x, y));
                if mouse.just_pressed(MouseButton::Left) {
                    events.extend(form.press(x, y));
                }
                if mouse.just_released(MouseButton::Left) {
                    events.extend(form.release(x, y));
                }
                let now = time.elapsed().as_millis() as u32;
                drive_controls_page(
                    &mut settings.page,
                    &mut form,
                    window,
                    scripted.as_deref(),
                    &mouse,
                    notches as i32,
                    now,
                    timers.as_deref_mut(),
                );
            }
        }
    }
    // The pad: the show's first control, then the frame's word.
    if !covered
        && keep
        && let Some(pad) = pad.as_deref_mut()
    {
        if pad.take_focus_request(PadForm::Settings) {
            form.pad_focus_set(Some((Part::Main, PadForm::Settings.control() as usize - 1)));
        }
        if let Some(key) = pad.take(PadForm::Settings) {
            match pad_key(&mut form, key) {
                PadSettings::Events(pad_events) => events.extend(pad_events),
                PadSettings::Back => {
                    info!("settings: the pad's MenuBak takes the back slot");
                    events.push(FormEvent::Action(FormAction::Cancel));
                }
            }
        }
    }
    let now = time.elapsed().as_millis() as u32;
    for event in events {
        match event {
            FormEvent::Page(page) => config.switch_page(page == 1),
            FormEvent::Slider(setting, fraction) => {
                config.set_slider(setting, fraction);
                if let Some(screen) = config.screen {
                    form.set_text(setting, &screen.draft);
                }
            }
            FormEvent::Toggle(setting, checked) => {
                config.set_toggle(setting, checked);
            }
            FormEvent::Rebind(row) => {
                editor.begin(row, now);
            }
            FormEvent::Action(FormAction::Defaults) => {
                config.defaults();
                if config.controls_page {
                    if let Some(copy) = &config.bindings {
                        form.set_keys(&key_names(copy));
                    }
                } else if config.benchmark.is_some() {
                    settings.benchmarking = true;
                } else if let Some(rows) = config.rows() {
                    // The Deck's preset refills at once.
                    let events = form.fill(&rows);
                    apply_rows(&mut form, &mut config, &events);
                }
            }
            FormEvent::Action(FormAction::Apply) => {
                // An unreadable settings file leaves the form open.
                if let Some(done) = config.apply() {
                    if let Some(new) = done.bindings {
                        if let Some(bindings) = bindings.as_mut() {
                            **bindings = new.clone();
                        }
                        applied.write(BindingsApplied(new));
                    }
                    keep = false;
                }
                break;
            }
            FormEvent::Action(FormAction::Cancel) => {
                config.cancel();
                keep = false;
                break;
            }
            FormEvent::Action(action) => {
                if let Some(controller) = settings::controller_view_tab(action) {
                    // The Keyboard or Controller tab: the controller mode
                    // and view; the rows stay the key rows, since Exarust
                    // draws no controller page.
                    config.set_controller_view(controller);
                    form.select_controller_view(controller);
                    if let Some(copy) = &config.bindings {
                        form.set_keys(&key_names(copy));
                    }
                }
            }
        }
    }
    if keep {
        settings.form = Some(form);
    }
}

/// What the pad's word did to the form.
#[derive(Debug, Clone, PartialEq)]
pub enum PadSettings {
    /// The focused record's answers.
    Events(Vec<FormEvent>),
    /// `MenuBak`: the form's back slot (Cancel).
    Back,
}

/// The menu dispatch for the Settings form, whose key slot is the base
/// form's (every word unanswered, [`crate::pad_menu::UNANSWERED`]): a
/// focused record that asks for the keys (a held trackbar) takes the whole
/// word; else `MenuBak` goes to the back slot (the form's attribute word
/// has bit 2), a navigation step picks the next record
/// ([`pad_pick`]) and the rest reach the focused record's key message,
/// whose `MenuSel` (0x100) presses a button or box and holds a trackbar.
pub fn pad_key(form: &mut FormState, key: crate::pad_menu::MenuKey) -> PadSettings {
    use crate::pad_menu::{Fallback, UNANSWERED, fallback};
    let back_closes = settings::FORM_ATTRIBUTES & 2 != 0;
    match fallback(UNANSWERED, key.key, form.pad_keys, back_closes) {
        Fallback::Answered => PadSettings::Events(Vec::new()),
        Fallback::Back => PadSettings::Back,
        Fallback::Pick => {
            pad_pick(form, key.cursor);
            PadSettings::Events(Vec::new())
        }
        Fallback::Control(word) => {
            PadSettings::Events(form.pad_control_key(word, (key.cursor.x, key.scroll_cursor.x)))
        }
    }
}

/// The directional pick over the form's records and its shown page's
/// ([`crate::gamepad::pick_control`]): from the focused record's centre
/// (the form's corner when none is focused) along the menu cursor, among
/// the records the pad can reach; the picked record takes the focus.
pub fn pad_pick(form: &mut FormState, cursor: Vec2) {
    use crate::gamepad::{PickTarget, pick_control};
    let mut found = Vec::new();
    let mut targets = Vec::new();
    for part in form.pad_parts() {
        for i in 0..form.controls(part).len() {
            let Some(centre) = form.pad_centre(part, i) else {
                continue;
            };
            targets.push(PickTarget {
                id: found.len() as i32,
                centre: IVec2::new(centre.0, centre.1),
                reachable: form.pad_reachable(part, i),
                current: form.pad_focus == Some((part, i)),
            });
            found.push((part, i));
        }
    }
    let origin = form
        .pad_focus
        .and_then(|(part, i)| form.pad_centre(part, i))
        .unwrap_or_else(|| form.origin());
    if let Some(id) = pick_control(
        cursor,
        IVec2::new(origin.0, origin.1),
        &targets,
        form.unit(),
    ) {
        let picked = found[id as usize];
        info!("settings: the pad picks {picked:?}");
        form.pad_focus_set(Some(picked));
    }
}

/// Refills the rows when the benchmark Defaults started has ended.
fn refresh_rows(mut settings: ResMut<SettingsForm>, config: Option<ResMut<HardwareConfig>>) {
    let Some(mut config) = config else {
        return;
    };
    if !settings.benchmarking || config.benchmark.is_some() {
        return;
    }
    settings.benchmarking = false;
    let rows = config.rows();
    if let (Some(form), Some(rows)) = (settings.form.as_mut(), rows) {
        let events = form.fill(&rows);
        apply_rows(form, &mut config, &events);
    }
}

/// Hiding the menu closes the form (and its key-read screen).
fn close_with_menu(
    menu: Option<Res<MenuScreen>>,
    mut settings: ResMut<SettingsForm>,
    mut editor: ResMut<BindingEditor>,
    config: Option<ResMut<HardwareConfig>>,
) {
    if !settings.is_open() || menu.is_none_or(|menu| menu.open) {
        return;
    }
    if let Some(mut config) = config {
        config.close();
    }
    editor.cancel();
    settings.form = None;
}

/// The inks of a separator's four-ink line frame
/// ([`exa_ui::skin::edge_frame`]: top, left, right, bottom): the hot-form
/// group's first, second, fifth and seventh shades, or with style 0x40 its
/// eighth, sixth, second and first, as the separator's draw passes them
/// (the hot-form group is the palette's fourth, from slot 0x9c).
fn rule_inks(palette: &SkinPalette, style: u32) -> [SkinInk; 4] {
    let hot = &palette.form_hot;
    if style & 0x40 != 0 {
        [hot[7], hot[5], hot[1], hot[0]]
    } else {
        [hot[0], hot[1], hot[4], hot[6]]
    }
}

/// The font a control's text uses and the texture size its quads sample.
pub(crate) struct Text<'a> {
    pub(crate) font: &'a GuiFont,
    pub(crate) size: Option<(u16, u16)>,
}

/// Paints one record at `origin`, clipped to `clip`; a hidden record
/// (0x110) is not drawn.
pub(crate) fn paint_control(
    c: &Control,
    origin: (i32, i32),
    unit: i32,
    palette: &SkinPalette,
    text: &Text,
    clip: Option<[i32; 4]>,
    painted: &mut Painted,
) {
    if c.state & HIDDEN == HIDDEN {
        return;
    }
    let rect = GuiRect::new(origin.0 + c.x, origin.1 + c.y, c.w, c.h);
    let caption = String::from_utf8_lossy(&c.text);
    let mut out = Painted::default();
    let inks = line_control_inks(1.0);
    let hovered = c.state & HOVER_BITS == HOVERED;
    let hover_ink = {
        let mut ink = palette.highlight[7];
        ink[3] = hover_alpha(1.0);
        ink
    };
    match c.class {
        1 => {
            // The line skin's frame and the caption's shade.
            let button = gui_paint::paint_line_button(
                rect,
                gui_paint::ButtonWords {
                    style: c.style,
                    state: c.state,
                    group: c.group,
                },
                unit,
                palette,
                1.0,
                text.font,
                text.size,
                &caption,
            );
            out.layers.extend(button.layers);
        }
        2 => {
            // The line skin's box: a four-unit square in the disabled ink,
            // the hover ink, or the on ink, with its mark square while lit.
            let ink = if c.state & state::DISABLED != 0 {
                inks[0]
            } else if hovered {
                hover_ink
            } else {
                inks[2]
            };
            let mut batch = line_skin_batch();
            push_line_quads(
                &mut batch,
                &check_box_quads(unit, rect.left, rect.top, c.state),
                ink,
            );
            batch.end_quads();
            out.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
            let x = label_x(c, unit) + origin.0;
            let label = GuiRect::new(x, rect.top, (rect.right - x).max(1), rect.height());
            let ink = match check_label_ink(c.state) {
                CheckLabelInk::Disabled => Some(palette.text[3]),
                CheckLabelInk::Text => None,
            };
            let caption_layers = gui_paint::paint_label_ink(
                label, 0x2300, unit, palette, text.font, text.size, &caption, ink,
            );
            out.layers.extend(caption_layers.layers);
        }
        4 => {
            // The line skin's slider: two two-unit caps and the track four
            // units tall, in the disabled, held or idle ink, then the
            // four-unit thumb, in the hover ink while hovered.
            let draw = draw_flat_slider(
                unit,
                SliderPlace {
                    x: rect.left as i16,
                    y: rect.top as i16,
                    w: c.w as i16,
                    h: c.h as i16,
                    mode: c.style,
                    state: c.state,
                    value_bits: c.value.to_bits(),
                    lip: 0,
                },
            );
            if let Some(flat) = draw.flat {
                let ink = match draw.ink {
                    0 => inks[0],
                    1 => palette.highlight[7],
                    _ => inks[1],
                };
                let mut batch = line_skin_batch();
                push_line_quads(&mut batch, &flat.quads[..3], ink);
                let thumb = if hovered { hover_ink } else { ink };
                push_line_quads(&mut batch, &flat.quads[3..], thumb);
                batch.end_quads();
                out.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
            }
            if !caption.is_empty() {
                let caption_layers = gui_paint::paint_label(
                    rect, c.style, unit, palette, text.font, text.size, &caption,
                );
                out.layers.extend(caption_layers.layers);
            }
        }
        5 => {
            // The label's back calls the skin's frame slot for a frame
            // style; the line skin's is empty, so a line-skinned label draws
            // only its front.
            if let (false, Some(texture)) = (c.text.is_empty(), text.size) {
                gui_paint::paint_label_front(&mut out, c, rect, unit, palette, text.font, texture);
            }
        }
        SEPARATOR_CLASS => {
            let mut batch = SkinBatch::new();
            batch.select(1, 0, false);
            let lines = exa_ui::controls::separator_lines(
                (rect.left, rect.top, rect.width(), rect.height()),
                0,
                c.style,
                unit,
                0,
            );
            // The separator's draw frames each line rectangle with the
            // skin's four-ink line loop.
            for (x, y, w, h) in lines {
                edge_frame(
                    &mut batch,
                    SkinRect::sized(x as f32, y as f32, w as f32, h as f32),
                    rule_inks(palette, c.style),
                );
            }
            out.push(GuiLayer::new(batch, LayerTexture::None));
        }
        _ => {}
    }
    for mut layer in out.layers {
        layer.clip = clip;
        painted.push(layer);
    }
}

/// The form's frame, its records, and the page in view with its records
/// clipped to the page's client (the Controls page scrolled, with its
/// scrollbar).
pub fn paint_form(form: &FormState, textures: &GuiTextures) -> Painted {
    // A shown form hands its fade of 1 to the skin before it draws.
    let palette = shown_palette(1.0);
    let unit = form.unit();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, size) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let text = Text { font: &font, size };
    let g = &form.geometry;
    let bounds = GuiRect::new(g.form.x, g.form.y, g.form.w, g.form.h);
    // The game skin's form draw with the words the show commits: the
    // constructor's style 0xb810 and attributes 0x31022, a top-level form,
    // so titled, shadowed and corner-framed.
    let (mut outer, committed) = gui_paint::paint_top_form(
        bounds,
        unit,
        &palette,
        1.0,
        gui_paint::FormWords {
            style: settings::FORM_STYLE,
            attributes: settings::FORM_ATTRIBUTES,
        },
        Some((settings::TITLE, &font, size)),
    );
    let mut painted = Painted {
        client: GuiRect::new(
            g.form.x + settings::BORDER,
            g.form.y + settings::BORDER,
            g.form.w - 2 * settings::BORDER,
            g.form.h - 2 * settings::BORDER,
        ),
        ..Painted::default()
    };
    painted.layers.extend(outer.under());
    let origin = form.origin();
    for c in &g.main {
        paint_control(c, origin, unit, &palette, &text, None, &mut painted);
    }
    // The page in view, held by the form: the Settings page is a base form
    // with no frame style (it draws nothing of its own), the Controls page
    // has the panel frame (0x88) and the scroll bar (0x200).
    let page = form.page_screen();
    let part = form.page_part();
    let page_rect = GuiRect::new(page.x, page.y, page.w, page.h);
    let mut at = gui_paint::LineFormAt::child(page_rect, painted.client, committed.attributes);
    let (attributes, style) = if part == Part::Controls {
        let page_client = form.page_client();
        let held = gui_paint::commit_words(CONTROLS_STYLE, CONTROLS_ATTRIBUTES, false);
        let rail = exa_ui::line_skin::refresh_line_scroll(
            exa_ui::line_skin::LineScrollPlace {
                style: held.state,
                x: at.in_owner.0,
                y: at.in_owner.1,
                w: page.w,
                h: page.h,
                client_top: page_client.y - page.y,
                client_bottom: page_client.y - page.y + page_client.h,
                owner_left: 0,
                owner_top: 0,
                owner_right: at.owner_client.0,
                owner_bottom: at.owner_client.1,
                fraction: form.scroll,
            },
            unit,
        );
        at.hover_mark = form.page_hover;
        at.scroll = Some(exa_ui::line_skin::LineFormScroll {
            client_top: page_client.y - page.y,
            client_right: page_client.x - page.x + page_client.w,
            client_bottom: page_client.y - page.y + page_client.h,
            thumb: rail.thumb,
            options: 0,
        });
        (CONTROLS_ATTRIBUTES, CONTROLS_STYLE)
    } else {
        (CHILD_ATTRIBUTES, 0)
    };
    let mut panel = gui_paint::paint_line_form(
        at,
        unit,
        &palette,
        1.0,
        gui_paint::commit_words(style, attributes, false),
        None,
    );
    painted.layers.extend(panel.under());
    let client = form.page_client();
    let clip = Some([client.x, client.y, client.w, client.h]);
    let origin = form.part_origin(part);
    for c in form.controls(part) {
        // Records wholly outside the scrolled client are not drawn.
        let top = origin.1 + c.y;
        if top + c.h < client.y || top > client.y + client.h {
            continue;
        }
        paint_control(c, origin, unit, &palette, &text, clip, &mut painted);
    }
    // The Controls page's power-tree help, a read-only text box child of the
    // page at 2, 48 units, 50 units high, scrolled with the page.
    if part == Part::Controls {
        let help = form.help_box();
        let rect = GuiRect::new(origin.0 + help.x, origin.1 + help.y, help.w, help.h);
        paint_help(rect, unit, &palette, &text, clip, &mut painted);
    }
    painted.layers.extend(panel.over());
    painted.layers.extend(outer.over());
    painted
}

/// Paints a read-only text box's wrapped, markup-aware text into `rect`,
/// with the console's text ink and its tag colour at full alpha. The box's
/// border, margin and font choice were not traced: the page's own font and
/// an inset of a unit each side stand in (a guess).
fn paint_help(
    rect: GuiRect,
    unit: i32,
    palette: &SkinPalette,
    text: &Text,
    clip: Option<[i32; 4]>,
    painted: &mut Painted,
) {
    let level = exa_ui::gui_font::level_for_unit(unit);
    let fonts = exa_ui::gui_font::gui_level(level);
    let client = exa_ui::form::ClientRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    };
    let mut help = exa_ui::text_box::TextBox::new(unit, fonts.main, fonts.small, client);
    help.read_only = true;
    help.markup = true;
    help.set_text(settings::HELP.as_bytes(), None);
    let inks = exa_ui::text_layout::TextInks {
        text: palette.text[6],
        tag_alpha: 255,
        selection: palette.highlight[2],
        slant: text.font.slant,
    };
    let draw = help.draw(&inks, &|_| None);
    if let Some(size) = text.size
        && !draw.glyphs.is_empty()
    {
        let mut glyphs = draw.glyphs;
        for glyph in &mut glyphs {
            glyph.x += rect.left;
            glyph.y += rect.top;
        }
        let mut layer = GuiLayer::new(
            exa_ui::gui_draw::glyph_batch(text.font, &glyphs, size, 1.0),
            LayerTexture::Named(text.font.texture),
        );
        layer.clip = clip;
        painted.push(layer);
    }
}

/// A modal screen's paint: the black backdrop at the fade byte (the form's
/// colour word, opaque black whose alpha the fade writes; that the form
/// fills the window with it is a guess) and its centred text.
pub fn paint_modal(
    text: &ScreenText,
    fade: u8,
    screen: (i32, i32),
    font: &GuiFont,
    size: Option<(u16, u16)>,
) -> Painted {
    let mut painted = Painted::default();
    let mut batch = SkinBatch::new();
    batch.select(1, 0, false);
    batch.set_ink([0, 0, 0, fade]);
    batch.begin();
    batch.vertex(0.0, 0.0);
    batch.vertex(0.0, screen.1 as f32);
    batch.vertex(screen.0 as f32, screen.1 as f32);
    batch.vertex(screen.0 as f32, 0.0);
    batch.end_quads();
    painted.push(GuiLayer::new(batch, LayerTexture::None));
    if let Some(size) = size {
        let c = text.colour;
        let ink = [(c >> 16) as u8, (c >> 8) as u8, c as u8, (c >> 24) as u8];
        painted.push(GuiLayer::new(
            text_batch(font, &text.quads, ink, size),
            LayerTexture::Named(font.texture),
        ));
    }
    painted
}

/// The palette's text colour as the modal paints take it: alpha in the
/// top byte, then red, green and blue. The label ink (the text group's
/// seventh shade) stands for it; which shade the screens read is a guess.
pub fn text_colour(palette: &SkinPalette) -> u32 {
    let [r, g, b, a] = palette.text[6];
    (u32::from(a) << 24) | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// Draws the form, then the key-read and warm-up screens over it.
fn draw(
    settings: Res<SettingsForm>,
    editor: Res<BindingEditor>,
    warm: Res<ShaderWarmScreen>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
) {
    if settings.form.is_none() && editor.screen.is_none() && warm.screen.is_none() {
        return;
    }
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    if let Some(form) = &settings.form {
        paint_form(form, textures).submit(&mut frames);
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let (screen, unit) = screen_and_unit(window);
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, size) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let colour = text_colour(&shown_palette(1.0));
    if let Some(key) = &editor.screen {
        paint_modal(
            &key.paint(&font, screen, colour),
            key.alpha,
            screen,
            &font,
            size,
        )
        .submit(&mut frames);
    }
    if let Some(cache) = &warm.screen {
        paint_modal(
            &cache.paint(&font, screen, colour),
            cache.alpha,
            screen,
            &font,
            size,
        )
        .submit(&mut frames);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::hw_config::Options;
    use exa_ui::modal_screens::{CacheShaderScreen, ReadKeyScreen};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .insert_resource(HardwareConfig::new(Options::default()))
            .add_plugins(crate::menu_screen::MenuScreenPlugin)
            .add_plugins(SettingsFormPlugin)
            .add_systems(
                Update,
                (
                    crate::hw_config::rebind_keys,
                    crate::hw_config::drive_warm_screen,
                )
                    .chain(),
            );
        app
    }

    #[test]
    fn the_menus_settings_row_opens_the_form_and_hiding_the_menu_closes_it() {
        let mut app = app();
        app.update();
        app.world_mut().resource_mut::<MenuScreen>().open();
        app.world_mut()
            .write_message(MenuChoice(MenuCommand::OpenSettings));
        // No window: the form waits for one.
        app.update();
        assert!(!app.world().resource::<SettingsForm>().is_open());
        app.world_mut().spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            },
            PrimaryWindow,
        ));
        app.world_mut()
            .write_message(MenuChoice(MenuCommand::OpenSettings));
        app.update();
        let settings = app.world().resource::<SettingsForm>();
        let form = settings.form.as_ref().expect("the form opened");
        assert_eq!(form.page, 0);
        assert!(app.world().resource::<HardwareConfig>().screen.is_some());
        // The key cells carry the bindings' names.
        assert_eq!(form.geometry.controls[1].text, b"W".to_vec());
        // Hiding the menu closes the form and the configuration's page.
        app.world_mut().resource_mut::<MenuScreen>().close();
        app.update();
        assert!(!app.world().resource::<SettingsForm>().is_open());
        assert!(app.world().resource::<HardwareConfig>().screen.is_none());
    }

    /// The settings form opened by the menu's row in a 1280x720 window,
    /// with the skin atlas and every GUI font's atlas installed.
    fn opened_with_textures() -> App {
        let mut app = app();
        let mut textures = GuiTextures::default();
        textures.textures.insert(
            SKIN_TEXTURE.into(),
            exa_ui::gui_draw::GuiTexture::solid([255, 255, 255, 255]),
        );
        for font in exa_ui::gui_font::FONTS {
            textures.textures.insert(
                font.texture.into(),
                exa_ui::gui_draw::GuiTexture::solid([255, 255, 255, 255]),
            );
        }
        app.insert_resource(textures);
        app.world_mut().spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            },
            PrimaryWindow,
        ));
        app.update();
        app.world_mut().resource_mut::<MenuScreen>().open();
        app.world_mut()
            .write_message(MenuChoice(MenuCommand::OpenSettings));
        app.update();
        assert!(app.world().resource::<SettingsForm>().is_open());
        app
    }

    /// Every layer the form's draw system submitted.
    fn submitted(app: &App) -> Vec<GuiLayer> {
        app.world()
            .resource::<GuiFrames>()
            .frames
            .iter()
            .flat_map(|frame| frame.layers.iter().cloned())
            .collect()
    }

    #[test]
    fn the_rules_draw_the_skins_four_ink_line_frame() {
        let app = opened_with_textures();
        let palette = shown_palette(1.0);
        let inks = rule_inks(&palette, 0);
        // The separator's draw frames each rule with the four-ink line
        // loop: eight line vertices, top, left, bottom and right, in the
        // hot-form group's first, second, seventh and fifth shades.
        let rules: Vec<_> = submitted(&app)
            .into_iter()
            .filter(|layer| {
                layer
                    .batch
                    .primitives
                    .iter()
                    .any(|p| p.draw == exa_ui::skin::GuiDraw::Lines)
                    && layer.batch.vertices.len() == 8
                    && layer.batch.vertices[0].ink == inks[0]
            })
            .collect();
        assert!(rules.len() >= 2, "{} rule frames", rules.len());
        let v = &rules[0].batch.vertices;
        let seen: Vec<_> = v.iter().map(|v| v.ink).collect();
        assert_eq!(
            seen,
            vec![
                inks[0], inks[0], inks[1], inks[1], inks[3], inks[3], inks[2], inks[2]
            ]
        );
        // The bottom line lies under the top one, both half a pixel in.
        assert!(v[3].y > v[0].y);
        assert_eq!(v[0].y.fract(), 0.5);
    }

    #[test]
    fn a_hovered_key_cell_draws_in_the_highlight_ink() {
        let mut app = opened_with_textures();
        {
            let mut settings = app.world_mut().resource_mut::<SettingsForm>();
            let form = settings.form.as_mut().expect("open");
            form.page = 1;
            // A key cell: a reporting block label (style 0x20003380).
            let cell = form
                .geometry
                .controls
                .iter_mut()
                .find(|c| c.class == 5 && c.style == exa_ui::settings::KEY_CELL_STYLE)
                .expect("a key cell");
            cell.state |= HOVERED;
        }
        app.world_mut().resource_mut::<GuiFrames>().clear();
        app.update();
        let palette = shown_palette(1.0);
        let mut highlight = palette.highlight[7];
        highlight[3] = 0xff;
        // The block label's hovered arm: the highlight group's eighth shade
        // at the GUI's alpha; the other cells keep the label ink.
        let text_inks: Vec<_> = submitted(&app)
            .iter()
            .filter(|layer| layer.texture != LayerTexture::Named(SKIN_TEXTURE))
            .filter_map(|layer| layer.batch.vertices.first().map(|v| v.ink))
            .collect();
        assert!(text_inks.contains(&highlight));
        assert!(text_inks.contains(&palette.text[6]));
    }

    #[test]
    fn a_hovered_button_caption_glows_in_the_live_form() {
        let mut app = opened_with_textures();
        {
            let mut settings = app.world_mut().resource_mut::<SettingsForm>();
            let form = settings.form.as_mut().expect("open");
            let button = form
                .geometry
                .main
                .iter_mut()
                .find(|c| c.class == 1 && c.text == b"Defaults")
                .expect("the Defaults button");
            button.state = (button.state & !0x110) | HOVERED;
        }
        app.world_mut().resource_mut::<GuiFrames>().clear();
        app.update();
        let palette = shown_palette(1.0);
        // The line skin's label slot for a hovered button: the caption in
        // the label group's eighth shade, then four copies in the highlight
        // group's first shade under the additive blend.
        let layers = submitted(&app);
        let glows: Vec<_> = layers
            .iter()
            .filter(|layer| layer.texture != LayerTexture::Named(SKIN_TEXTURE))
            .filter(|layer| layer.batch.blend == 0x201)
            .filter(|layer| {
                layer
                    .batch
                    .vertices
                    .first()
                    .is_some_and(|v| v.ink == palette.highlight[0])
            })
            .collect();
        assert_eq!(glows.len(), 4, "the glow's four copies");
        assert!(layers.iter().any(|layer| {
            layer.texture != LayerTexture::Named(SKIN_TEXTURE)
                && layer
                    .batch
                    .vertices
                    .first()
                    .is_some_and(|v| v.ink == palette.text[7])
        }));
    }

    #[test]
    fn the_form_paints_its_frame_records_and_page() {
        let form = FormState::open((1280, 720), 4);
        let mut textures = GuiTextures::default();
        textures.textures.insert(
            SKIN_TEXTURE.into(),
            exa_ui::gui_draw::GuiTexture::solid([10, 20, 30, 255]),
        );
        let painted = paint_form(&form, &textures);
        // Without a font texture only skin and fill layers draw: the frame,
        // the visible buttons and rules, the page and its controls.
        assert!(painted.layers.len() > 20);
        // The buttons and sliders draw in the line skin's textured mode
        // with its idle ink, the checkboxes with its on ink.
        let has_ink = |ink: [u8; 4]| {
            painted.layers.iter().any(|l| {
                l.batch.tex_col == Some(0.0)
                    && l.batch.vertices.first().is_some_and(|v| v.ink == ink)
            })
        };
        assert!(has_ink([255, 250, 230, 104]));
        assert!(has_ink([255, 250, 230, 128]));
        let clipped = painted.layers.iter().filter(|l| l.clip.is_some()).count();
        assert!(clipped > 10);
        assert!(
            painted
                .layers
                .iter()
                .filter_map(|l| l.clip)
                .all(|c| c == [427, 160, 422, 404])
        );
    }

    #[test]
    fn the_controls_page_draws_the_power_tree_help() {
        let mut form = FormState::open((1280, 720), 4);
        form.switch(1);
        let mut textures = GuiTextures::default();
        textures.textures.insert(
            SKIN_TEXTURE.into(),
            exa_ui::gui_draw::GuiTexture::solid([10, 20, 30, 255]),
        );
        // The level's font atlas, so the help's glyphs are drawn.
        let level = exa_ui::gui_font::level_for_unit(4);
        let font = exa_ui::gui_font::gui_level(level).main;
        textures.textures.insert(
            font.texture.into(),
            exa_ui::gui_draw::GuiTexture::solid([255; 4]),
        );
        let painted = paint_form(&form, &textures);
        let help = painted.layers.iter().filter(|layer| {
            layer.texture == LayerTexture::Named(font.texture)
                && layer.batch.vertices.len() > 200
                && layer.clip == Some([427, 152, 397, 412])
        });
        assert_eq!(help.count(), 1, "the help's glyph layer is missing");
    }

    #[test]
    fn the_modal_screens_draw_a_backdrop_and_their_text() {
        let font = &exa_ui::gui_font::RAYFORM_10;
        let mut key = ReadKeyScreen::new(1, 0);
        key.step(200);
        assert_eq!(key.alpha, 128);
        let text = key.paint(font, (800, 600), 0xff10_2030);
        let painted = paint_modal(&text, key.alpha, (800, 600), font, Some((256, 256)));
        assert_eq!(painted.layers.len(), 2);
        assert_eq!(painted.layers[1].texture, LayerTexture::Named(font.texture));
        // "PRESS A KEY" is eleven glyphs (the space among them).
        assert_eq!(text.quads.len(), 11);
        let cache = CacheShaderScreen::new(0);
        let text = cache.paint(font, (800, 600), 0xff10_2030);
        assert_eq!(text.colour >> 24, 0);
        assert_eq!(text_colour(&SkinPalette::filled([1, 2, 3, 4])), 0x0401_0203);
    }

    #[test]
    fn escape_closes_the_form_over_the_open_menu() {
        let mut app = app();
        app.world_mut().spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            },
            PrimaryWindow,
        ));
        app.update();
        app.world_mut().resource_mut::<MenuScreen>().open();
        app.world_mut()
            .write_message(MenuChoice(MenuCommand::OpenSettings));
        app.update();
        assert!(app.world().resource::<SettingsForm>().is_open());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert!(!app.world().resource::<SettingsForm>().is_open());
        // The menu stayed open under the form.
        assert!(app.world().resource::<MenuScreen>().open);
    }

    /// The pad's MenuSel reaches the focused record's key message: on the
    /// Controls tab it turns the tab on and switches the page; on a
    /// trackbar it holds it, after which the whole words go to it and a
    /// scroll step moves its value.
    #[test]
    fn the_pads_menu_sel_presses_the_focused_record() {
        use crate::pad_menu::{MenuKey, PadForm};
        let key = |word: u32| MenuKey {
            form: PadForm::Settings,
            key: word,
            cursor: Vec2::ZERO,
            scroll_cursor: Vec2::ZERO,
        };
        let mut form = FormState::open((1280, 720), 2);
        form.pad_focus_set(Some((Part::Main, 1)));
        let out = pad_key(&mut form, key(0x100));
        assert_eq!(out, PadSettings::Events(vec![FormEvent::Page(1)]));
        // Back to the Settings page and onto a stepped trackbar.
        form.pad_focus_set(Some((Part::Main, 0)));
        pad_key(&mut form, key(0x100));
        assert_eq!(form.page, 0);
        let slider = form
            .geometry
            .settings
            .iter()
            .position(|c| c.class == 4 && c.steps > 0 && c.state & state::DISABLED == 0)
            .expect("a stepped trackbar");
        form.pad_focus_set(Some((Part::Settings, slider)));
        let before = form.geometry.settings[slider].value;
        pad_key(&mut form, key(0x100));
        assert!(form.pad_keys);
        // A right step of the navigation stick, now the trackbar's.
        let out = pad_key(&mut form, key(0x2001));
        let after = form.geometry.settings[slider].value;
        assert!(after > before || before == 1.0, "{before} -> {after}");
        assert_eq!(form.pad_focus, Some((Part::Settings, slider)));
        let _ = out;
        // MenuBak lets it go; the next MenuBak is the back slot.
        pad_key(&mut form, key(0x200));
        assert!(!form.pad_keys);
        assert_eq!(pad_key(&mut form, key(0x200)), PadSettings::Back);
    }
}
