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
use exa_ui::modal_screens::ScreenText;
use exa_ui::settings::{
    self, CHILD_ATTRIBUTES, CONTROLS_ATTRIBUTES, CONTROLS_STYLE, FormAction, FormEvent, FormState,
    HIDDEN, Part, SEPARATOR_CLASS,
};
use exa_ui::skin::{
    ControlSliderPlace, FormScroll, GuiRect as SkinRect, SkinBatch, SkinInk, SkinPalette,
    draw_accent_bar, draw_control_box, draw_control_slider, draw_style_frames, four_stop_rect,
};

use crate::bindings::Bindings;
use crate::gui_paint::{self, Frame, Painted};
use crate::hw_config::{
    BindingEditor, BindingsApplied, HardwareConfig, ShaderWarmScreen, binding_rows,
};
use crate::menu_pages::MenuCommand;
use crate::menu_screen::{MenuChoice, MenuScreen, MenuScreenSystems};

/// The open Settings form.
#[derive(Resource, Debug, Default)]
pub struct SettingsForm {
    pub form: Option<FormState>,
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
                (open_settings, form_input, refresh_rows, close_with_menu)
                    .chain()
                    .after(MenuScreenSystems)
                    .before(crate::hw_config::rebind_keys),
            )
            .add_systems(Update, draw.after(MenuScreenSystems));
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
    form.set_device(adapter.as_deref().map(|info| info.name.as_str()));
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
        if keys.just_pressed(KeyCode::Escape) {
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
                if notches != 0.0 {
                    form.wheel(x, y, notches as i32);
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
            FormEvent::Action(_) => {}
        }
    }
    if keep {
        settings.form = Some(form);
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

fn skin_rect(r: GuiRect) -> SkinRect {
    SkinRect::sized(
        r.left as f32,
        r.top as f32,
        r.width() as f32,
        r.height() as f32,
    )
}

/// The ink a separator's line takes: the hot-form group's first, second,
/// fifth and seventh shades, the slots 0x9c, 0xa0, 0xac and 0xb4 read
/// against a palette starting its form group at 0x3c (a guess from the
/// textured frame's 0x6c and 0x9c face slots, which match the control and
/// hot-form groups' first shades).
fn rule_inks(palette: &SkinPalette, style: u32) -> [SkinInk; 4] {
    let hot = &palette.form_hot;
    if style & 0x40 != 0 {
        [hot[7], hot[5], hot[1], hot[0]]
    } else {
        [hot[0], hot[1], hot[4], hot[6]]
    }
}

/// The font a control's text uses and the texture size its quads sample.
struct Text<'a> {
    font: &'a GuiFont,
    size: Option<(u16, u16)>,
}

/// Paints one record at `origin`, clipped to `clip`; a hidden record
/// (0x110) is not drawn.
fn paint_control(
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
    let on = c.state & state::ON != 0;
    let lit = c.state & state::LIT != 0;
    let mut out = Painted::default();
    match c.class {
        1 => {
            out = gui_paint::paint_button(
                rect,
                c.style,
                unit,
                palette,
                text.font,
                text.size,
                &caption,
                on,
                lit && !on,
            );
        }
        2 => {
            // The box is the record's height square at its left (a guess:
            // the check box's skin draw was not traced) and the tick, while
            // on or lit, an accent square inside it where the tick glyph
            // would be (the icon atlas's tick is not wired).
            let side = rect.height();
            let square = GuiRect::new(rect.left, rect.top, side, side);
            let mut batch = SkinBatch::new();
            draw_control_box(&mut batch, skin_rect(square), 0.0, c.state, palette);
            if on || lit {
                let inset = (side / 4).max(1);
                let tick = GuiRect::new(
                    square.left + inset,
                    square.top + inset,
                    side - inset * 2,
                    side - inset * 2,
                );
                draw_accent_bar(&mut batch, skin_rect(tick), palette);
            }
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
            let mut batch = SkinBatch::new();
            draw_control_slider(
                &mut batch,
                ControlSliderPlace {
                    rect: skin_rect(rect),
                    inset: c.margin,
                    thumb: c.knob,
                    state: c.state,
                    style: c.style,
                    fraction: c.value,
                    unit,
                },
                &[],
                palette,
            );
            out.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
            if !caption.is_empty() {
                let caption_layers = gui_paint::paint_label(
                    rect, c.style, unit, palette, text.font, text.size, &caption,
                );
                out.layers.extend(caption_layers.layers);
            }
        }
        5 => {
            if c.style & 0xf0 != 0 {
                let mut batch = SkinBatch::new();
                draw_style_frames(&mut batch, skin_rect(rect), c.style, palette);
                out.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
            }
            if !caption.is_empty() {
                let caption_layers = gui_paint::paint_label(
                    rect, c.style, unit, palette, text.font, text.size, &caption,
                );
                out.layers.extend(caption_layers.layers);
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
            for (x, y, w, h) in lines {
                four_stop_rect(
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
    let palette = gui_paint::palette();
    let unit = form.unit();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, size) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let text = Text { font: &font, size };
    let g = &form.geometry;
    let mut frame = Frame::new(
        GuiRect::new(g.form.x, g.form.y, g.form.w, g.form.h),
        unit,
        settings::TITLE,
    );
    frame.attributes = settings::FORM_ATTRIBUTES;
    frame.style = settings::FORM_STYLE;
    frame.spacing = settings::BORDER;
    let mut painted = gui_paint::paint_frame(&frame, &palette, &font, size);
    let origin = form.origin();
    for c in &g.main {
        paint_control(c, origin, unit, &palette, &text, None, &mut painted);
    }
    let page = form.page_screen();
    let part = form.page_part();
    let (attributes, style, vertical) = if part == Part::Controls {
        let bar = form.scrollbar();
        (
            CONTROLS_ATTRIBUTES,
            CONTROLS_STYLE,
            Some(FormScroll {
                position: bar.thumb_y,
                length: bar.thumb,
            }),
        )
    } else {
        (CHILD_ATTRIBUTES, 0, None)
    };
    let panel = gui_paint::paint_child(
        GuiRect::new(page.x, page.y, page.w, page.h),
        unit,
        &palette,
        attributes,
        style,
        settings::BORDER,
        vertical,
    );
    painted.layers.extend(panel.layers);
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
    painted
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
    let colour = text_colour(&gui_paint::palette());
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
}
