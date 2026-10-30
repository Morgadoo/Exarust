//! The running app's main menu screen: a drawn page of the menu's rows,
//! opened and closed with F1, whose mouse and keyboard follow the original's
//! page table, hit test and click handler ([`crate::menu_pages`]).
//!
//! The original draws its row text with the menu font's letter sheet
//! (`titlefont.rfi`) and its title with `maintitle01.rfi`; both load with
//! the GUI textures, and the base font stands in only when a sheet is
//! missing. Hovering a row and pressing one play `MenuSel` and `MenuClick`
//! as audio-environment one-shots at gain 0.06 and a drawn 0.8–1.2 pitch. The row positions,
//! widths, selection, press, commands and page flow are the original's. A
//! row is selected under the pointer or with Up and Down, a press nudges
//! the selected row down, and the row runs on the release over it or on
//! Enter. Escape goes back from the arena and practice pages, as the
//! original's does at the main menu, and closes the screen elsewhere.

use std::sync::{Arc, Mutex};

use bevy::audio::AudioSource;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ui::gui_core::{FormHost, FormId, GuiRect, GuiTask};
use exa_ui::gui_draw::{
    GuiFrame, GuiFrames, GuiTextures, LayerTexture, icon_batch, menu_batch, text_batch,
};
use exa_ui::gui_font::{FONTBASE_24, gui_level, level_for_unit};
use exa_ui::gui_text::{MenuFont, menu_font_scale};
use exa_ui::icon_atlas::IconQuad;
use exa_ui::menu::{
    CLICK_SOUND, MENU_SOUND_FLAGS, MENU_SOUND_GAIN, MENU_TITLE_INK, MENU_TITLE_TEXTURE,
    SELECT_SOUND, back_main_menu, menu_row_scale, menu_sound_pitch, select_main_menu, title_place,
};
use exa_ui::skin::{SkinBatch, SkinInk};

use crate::flow::Flow;
use crate::menu_pages::{
    MenuCommand, MenuGeometry, MenuHit, Page, PageState, Row, RowKind, game_mode, menu_command,
    menu_hit, page_origin, page_rows, paint, row_h, session_state,
};

/// The overlay's darkness.
const SHADE: u8 = 210;
/// The title drawn over the page when the art sheet is not installed.
const TITLE: &str = "EXANIMA";
const TITLE_COLOUR: [u8; 3] = [230, 220, 190];

/// Which of the menu's two cues to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCue {
    Select,
    Click,
}

/// A cue the screen asks the audio system to play: its pitch has already
/// been drawn.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct MenuSound {
    pub cue: MenuCue,
    pub pitch: f32,
}

/// The menu's cue draws: the original takes each pitch from the run-time
/// library's unit generator (the Mersenne twister, not the engine's LCG).
/// The original seeds that generator from the clock at start-up; this
/// stream starts from the twister's standard seed [guess].
#[derive(Resource, Debug, Clone)]
pub struct MenuSoundRoll(pub crate::arena::Mt19937);

impl Default for MenuSoundRoll {
    fn default() -> Self {
        Self(crate::arena::Mt19937::new(5489))
    }
}

impl MenuSoundRoll {
    /// The next unit draw, in [0, 1).
    pub fn roll(&mut self) -> f64 {
        self.0.unit()
    }
}

/// The menu's screen state.
#[derive(Resource, Debug, Clone)]
pub struct MenuScreen {
    pub open: bool,
    pub page: Page,
    /// The selected row (the pointer's or the keys'), 0-based.
    pub selected: usize,
    /// The row a press is held on, drawn nudged down; it runs when the
    /// press is released over it.
    pub pressed: Option<usize>,
    /// The game-mode and session-state words the commands read.
    pub game: u32,
    pub session: u32,
    pub state: PageState,
    /// The page was rebuilt since the last draw.
    dirty: bool,
    /// A manual opened from the menu covers it.
    covered: bool,
    /// The clock of the last select cue, for the 200-unit limit.
    sound_stamp: u32,
}

impl Default for MenuScreen {
    fn default() -> Self {
        Self {
            open: false,
            page: Page::Title,
            selected: 0,
            pressed: None,
            game: game_mode::MENU,
            session: session_state::MAIN_MENU,
            state: PageState::default(),
            dirty: true,
            covered: false,
            sound_stamp: 0,
        }
    }
}

impl MenuScreen {
    /// Opens the title page.
    pub fn open(&mut self) {
        self.open = true;
        self.show(Page::Title);
    }

    /// Opens `page` (the flow's page: the in-game page during a session, the
    /// title otherwise).
    pub fn open_at(&mut self, page: Page) {
        self.open = true;
        self.show(page);
    }

    pub fn close(&mut self) {
        self.open = false;
        self.pressed = None;
        self.dirty = true;
    }

    /// Sets the flag a manual opened from the menu covers it with.
    pub fn set_covered(&mut self, covered: bool) {
        self.covered = covered;
    }

    /// Shows `page` from its first row.
    pub fn show(&mut self, page: Page) {
        self.page = page;
        self.selected = 0;
        self.pressed = None;
        self.dirty = true;
    }

    /// Moves the selection by `delta`, clamped to the page's rows.
    pub fn move_selection(&mut self, delta: i32) {
        let count = page_rows(self.page, self.state).len() as i32;
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = (self.selected as i32 + delta).clamp(0, count - 1) as usize;
        self.dirty = true;
    }

    /// The command of the 1-based row.
    pub fn command(&self, row: usize) -> MenuCommand {
        menu_command(self.page, row, self.game, self.session, self.state.arena)
    }

    /// Runs a command whose page work this screen owns: a page change, the
    /// resume (the screen closes) and the exits to the title. The session
    /// and form work of the others belongs to the game modes.
    pub fn apply(&mut self, command: MenuCommand) {
        match command {
            MenuCommand::ShowPage(page) => self.show(page),
            MenuCommand::Resume => self.close(),
            MenuCommand::ExitArena | MenuCommand::QuitStory | MenuCommand::ExitToTitle => {
                self.show(Page::Title)
            }
            MenuCommand::None => {}
            _ => debug!("menu command {command:?}"),
        }
        self.pressed = None;
    }

    /// The back move (the original's back handler): the arena page goes to
    /// the title and the practice page to the arena page. Returns whether
    /// the page changed.
    pub fn back(&mut self) -> bool {
        let back = back_main_menu(self.page as i32);
        if back.sound {
            self.show(page_from_number(back.page));
        }
        back.sound
    }

    /// The row's width in window pixels, as the menu font measures it.
    pub fn row_width(&self, row: &Row, row_scale: f32) -> i32 {
        row_width_px(row, row_scale)
    }
}

/// A command the running screen handed to the game modes. The screen runs
/// its own page changes at once; the session, form and quit work behind the
/// other commands is [`crate::flow`]'s and the host's.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuChoice(pub MenuCommand);

/// Whether the title page draws the menu form's title logo. The boot and the
/// return to the title set it; it is only drawn on the title page.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TitleLogo(pub bool);

impl Default for TitleLogo {
    fn default() -> Self {
        Self(true)
    }
}

/// The geometry the menu's form hit tests against, shared with the GUI
/// core's form record.
#[derive(Debug, Clone, Default)]
struct FormGeometry {
    bounds: GuiRect,
    rows: Vec<RowGeometry>,
    origin: f32,
    row_h: f32,
    ascent: i32,
    band_inset: i32,
    covered: bool,
    open: bool,
}

/// One row's hit geometry.
#[derive(Debug, Clone, Copy, Default)]
struct RowGeometry {
    width: i32,
    enabled: bool,
}

/// The menu's form in the GUI core: the rows are its controls (1-based).
struct MenuForm {
    geometry: Arc<Mutex<FormGeometry>>,
    selected: i32,
}

impl FormHost for MenuForm {
    fn parent(&self) -> Option<FormId> {
        None
    }

    fn bounds(&self) -> GuiRect {
        self.geometry.lock().map(|g| g.bounds).unwrap_or_default()
    }

    fn is_visible(&self) -> bool {
        self.geometry.lock().map(|g| g.open).unwrap_or(false)
    }

    fn hit_test(&self, x: i32, y: i32) -> i32 {
        let Ok(geometry) = self.geometry.lock() else {
            return -1;
        };
        let rows: Vec<Row> = geometry
            .rows
            .iter()
            .map(|row| Row {
                label: "",
                kind: RowKind::Begin,
                enabled: row.enabled,
            })
            .collect();
        let widths: Vec<i32> = geometry.rows.iter().map(|row| row.width).collect();
        let shape = MenuGeometry {
            width: geometry.bounds.width(),
            height: geometry.bounds.height(),
            origin: geometry.origin,
            row_h: geometry.row_h,
            ascent: geometry.ascent,
            band_inset: geometry.band_inset,
        };
        match menu_hit(
            shape,
            geometry.covered,
            true,
            x - geometry.bounds.left,
            y - geometry.bounds.top,
            &rows,
            &widths,
        ) {
            MenuHit::Row(row) => row as i32,
            MenuHit::Miss => -1,
            MenuHit::Pass => 0,
        }
    }

    fn select(&mut self, control: i32) -> u32 {
        self.selected = control;
        3
    }

    fn selected_control(&self) -> i32 {
        self.selected
    }
}

/// The GUI core task the menu routes through.
#[derive(Resource)]
struct MenuRouter {
    task: GuiTask,
    geometry: Arc<Mutex<FormGeometry>>,
    form: FormId,
}

pub struct MenuScreenPlugin;

impl Plugin for MenuScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuScreen>()
            .init_resource::<TitleLogo>()
            .init_resource::<MenuSoundRoll>()
            .add_message::<MenuChoice>()
            .add_message::<MenuSound>()
            .add_systems(Startup, setup_router)
            .add_systems(
                Update,
                (toggle, input, draw, play_menu_sounds)
                    .chain()
                    .in_set(MenuScreenSystems)
                    .after(crate::gui_screens::draw_screens),
            );
    }
}

/// The menu screen's toggle, input and draw, in that order: the forms the
/// menu opens over itself (the Settings form) read and draw after it.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MenuScreenSystems;

/// Opens the menu as F1 does: at the flow's page for its mode and session
/// words, telling the flow it opened; without a flow, the title page.
pub(crate) fn open_menu(menu: &mut MenuScreen, flow: Option<&mut Flow>) {
    match flow {
        Some(flow) => {
            menu.game = flow.mode.word();
            menu.session = flow.phase.word();
            menu.state = flow.page_state();
            menu.open_at(flow.menu_page());
            flow.menu_opened();
        }
        None => menu.open(),
    }
}

/// Creates the menu's form in a GUI core task.
fn setup_router(mut commands: Commands) {
    let geometry = Arc::new(Mutex::new(FormGeometry::default()));
    let mut task = GuiTask::new((800, 600));
    let form = task.add(Box::new(MenuForm {
        geometry: Arc::clone(&geometry),
        selected: -1,
    }));
    commands.insert_resource(MenuRouter {
        task,
        geometry,
        form,
    });
}

/// F1 opens and closes the menu. Opening it in a session shows the in-game
/// page for the flow's mode and session words, and hides the session hook
/// behind it, as the original's frame task does while the menu is up.
fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut menu: ResMut<MenuScreen>,
    end: Option<Res<crate::end_menu_screen::EndMenuScreen>>,
    mut flow: Option<ResMut<Flow>>,
    creator: Option<Res<crate::creator_screen::CreatorScreen>>,
) {
    if !keys.just_pressed(KeyCode::F1) {
        return;
    }
    // The end menu and the character creator's setup forms keep the menu
    // closed.
    if end.is_some_and(|end| end.open) || creator.is_some_and(|c| c.is_open()) {
        return;
    }
    if menu.open {
        menu.close();
        if let Some(flow) = flow.as_mut() {
            flow.menu_closed();
        }
        return;
    }
    open_menu(&mut menu, flow.as_deref_mut());
}

/// Reads the pointer and keys while the menu is open. The Settings form,
/// a form above the menu, takes them while it is open.
#[allow(clippy::too_many_arguments)]
fn input(
    mut menu: ResMut<MenuScreen>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time<Real>>,
    mut roll: ResMut<MenuSoundRoll>,
    mut sounds: MessageWriter<MenuSound>,
    mut exit: MessageWriter<AppExit>,
    mut choices: MessageWriter<MenuChoice>,
    mut router: ResMut<MenuRouter>,
    settings: Option<Res<crate::settings_form::SettingsForm>>,
) {
    if !menu.open || settings.is_some_and(|settings| settings.is_open()) {
        return;
    }
    let now = time.elapsed().as_millis() as u32;
    let mut cue = |roll: &mut MenuSoundRoll, cue: MenuCue| {
        sounds.write(MenuSound {
            cue,
            pitch: menu_sound_pitch(roll.roll()),
        });
    };
    if keys.just_pressed(KeyCode::Escape) {
        if !menu.back() {
            menu.close();
        }
        return;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        menu.move_selection(1);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        menu.move_selection(-1);
    }
    if keys.just_pressed(KeyCode::Enter) {
        let row = menu.selected + 1;
        cue(&mut roll, MenuCue::Click);
        run(&mut menu, row, &mut exit, &mut choices);
        return;
    }
    if keys.just_pressed(KeyCode::Backspace) {
        menu.back();
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = window.scale_factor();
    let height = window.physical_height() as i32;
    let width = window.physical_width() as i32;
    let row_scale = menu_row_scale(height);
    let rows = page_rows(menu.page, menu.state);
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let x = (cursor.x * scale) as i32;
    let y = (cursor.y * scale) as i32;
    let menu_font = MenuFont::at_scale(menu_font_scale(row_scale));
    router.task.screen = (width, height);
    if let Ok(mut geometry) = router.geometry.lock() {
        geometry.bounds = GuiRect::new(0, 0, width, height);
        geometry.rows = rows
            .iter()
            .map(|row| RowGeometry {
                width: row_width_px(row, row_scale),
                enabled: row.enabled,
            })
            .collect();
        geometry.origin = page_origin(menu.page, rows.len());
        geometry.row_h = row_h(height);
        geometry.ascent = menu_font.ascent;
        geometry.band_inset = menu_font.origin.1;
        geometry.covered = menu.covered;
        geometry.open = menu.open;
    }
    let hit_row = {
        let hit = router.task.hit_and_focus(x, y);
        (hit.form == Some(router.form) && hit.control > 0).then_some(hit.control as usize - 1)
    };
    // The pointer selects the row under it; while a press is held only
    // the pressed row can be selected. The select rule stores the row and
    // plays the cue at most once per 200 units.
    if let Some(row) = hit_row
        && menu.selected != row
        && menu.pressed.is_none_or(|pressed| pressed == row)
    {
        let selected = select_main_menu(row as i32 + 1, menu.sound_stamp as i32, now as i32, true);
        menu.selected = (selected.index - 1).max(0) as usize;
        menu.sound_stamp = selected.stamp as u32;
        if selected.sound {
            cue(&mut roll, MenuCue::Select);
        }
        menu.dirty = true;
    }
    if mouse.just_pressed(MouseButton::Left)
        && let Some(row) = hit_row
    {
        menu.selected = row;
        menu.pressed = Some(row);
        menu.dirty = true;
        cue(&mut roll, MenuCue::Click);
    }
    if mouse.just_released(MouseButton::Left)
        && let Some(pressed) = menu.pressed
    {
        if hit_row == Some(pressed) {
            run(&mut menu, pressed + 1, &mut exit, &mut choices);
        } else {
            menu.pressed = None;
        }
        menu.dirty = true;
    }
}

/// Plays a cue the input asked for, as the original's handlers do: the
/// `MenuSel` or `MenuClick` buffer (each loaded once, as the constructor
/// does) as a one-shot of the audio environment at the origin, gain 0.06,
/// the drawn pitch and the interface flags 0x4060. Missing audio or a
/// missing cue leaves the menu silent.
fn play_menu_sounds(
    mut sounds: MessageReader<MenuSound>,
    game: Option<Res<exa_assets::Game>>,
    mut bank: Option<ResMut<exa_audio::SoundBank>>,
    mut sources: Option<ResMut<Assets<AudioSource>>>,
    mut env: Option<ResMut<exa_audio::AudioEnvironment>>,
    mut buffers: Local<Option<[Option<u32>; 2]>>,
) {
    let (Some(game), Some(bank), Some(sources), Some(env)) =
        (game, bank.as_mut(), sources.as_mut(), env.as_mut())
    else {
        sounds.clear();
        return;
    };
    let [select, click] = *buffers.get_or_insert_with(|| {
        [
            bank.load_buffer(&game, sources, SELECT_SOUND),
            bank.load_buffer(&game, sources, CLICK_SOUND),
        ]
    });
    for sound in sounds.read() {
        let buffer = match sound.cue {
            MenuCue::Select => select,
            MenuCue::Click => click,
        };
        if let Some(buffer) = buffer {
            env.play_buffer(
                buffer,
                Vec3::ZERO,
                MENU_SOUND_GAIN,
                sound.pitch,
                MENU_SOUND_FLAGS,
            );
        }
    }
}

/// Runs the 1-based row's command; the title's EXIT leaves the app. Its own
/// page changes and resume happen at once; the rest goes to the game modes
/// through [`MenuChoice`].
fn run(
    menu: &mut MenuScreen,
    row: usize,
    exit: &mut MessageWriter<AppExit>,
    choices: &mut MessageWriter<MenuChoice>,
) {
    let command = menu.command(row);
    if command == MenuCommand::Quit {
        exit.write(AppExit::Success);
    } else {
        choices.write(MenuChoice(command));
    }
    menu.apply(command);
    menu.dirty = true;
}

fn row_width_px(row: &Row, row_scale: f32) -> i32 {
    MenuFont::at_scale(menu_font_scale(row_scale))
        .row_width(row.label.as_bytes())
        .max(1)
}

fn page_from_number(number: i32) -> Page {
    match number {
        1 => Page::Start,
        2 => Page::InGame,
        4 => Page::ArenaPause,
        5 => Page::StoryPause,
        8 => Page::ClassSelect,
        _ => Page::Title,
    }
}

/// One row of a painted menu: its place, text, ink and whether it is the
/// selected row (drawn last through the menu text program).
#[derive(Clone, Debug, PartialEq)]
pub struct MenuRowPaint {
    pub x: i32,
    pub baseline: i32,
    pub text: String,
    pub ink: SkinInk,
    pub selected: bool,
}

/// The rows the page paints, in the original's order: every unselected row
/// first, then the selected row (nudged a third of a unit down while it is
/// pressed). This is [`paint`] with the menu font's own record.
pub fn painted_rows(
    width: i32,
    height: i32,
    page: Page,
    state: PageState,
    selected: usize,
    pressed: Option<usize>,
    unit: i32,
) -> Vec<MenuRowPaint> {
    let row_scale = menu_row_scale(height);
    let rows = page_rows(page, state);
    let widths: Vec<i32> = rows
        .iter()
        .map(|row| row_width_px(row, row_scale))
        .collect();
    let labels: Vec<&str> = rows.iter().map(|row| row.label).collect();
    let marks: Vec<i32> = vec![0; rows.len()];
    let disabled: Vec<bool> = rows.iter().map(|row| !row.enabled).collect();
    let menu_font = MenuFont::at_scale(menu_font_scale(row_scale));
    paint(
        width,
        height,
        page_origin(page, rows.len()),
        row_h(height),
        menu_font.ascent,
        Some(selected),
        pressed,
        &labels,
        &widths,
        &marks,
        &disabled,
        unit,
    )
    .into_iter()
    .map(|row| MenuRowPaint {
        x: row.x,
        baseline: row.y,
        text: row.text,
        ink: row.colour,
        selected: row.selected,
    })
    .collect()
}

/// The full-screen backdrop the page draws over the game: near black at the
/// screen's shade.
pub fn backdrop(width: i32, height: i32) -> (SkinBatch, SkinInk) {
    let mut batch = SkinBatch::new();
    batch.select(1, 0, false);
    let ink = [8, 8, 10, SHADE];
    batch.set_ink(ink);
    batch.begin();
    batch.vertex(0.0, 0.0);
    batch.vertex(0.0, height as f32);
    batch.vertex(width as f32, height as f32);
    batch.vertex(width as f32, 0.0);
    batch.end_quads();
    (batch, ink)
}

/// The page's frame through the GUI draw path: the backdrop, the title logo
/// and the rows. The selected row is last and drawn with the unit's menu
/// text program; the others use the plain textured program. When the menu
/// font's sheet is not installed the rows fall back to the base font (the
/// screen's earlier behaviour), still through the same draw path.
#[allow(clippy::too_many_arguments)]
pub fn menu_frame(
    width: i32,
    height: i32,
    page: Page,
    state: PageState,
    selected: usize,
    pressed: Option<usize>,
    title_logo: bool,
    textures: &GuiTextures,
) -> GuiFrame {
    let mut frame = GuiFrame::new();
    if width <= 0 || height <= 0 {
        return frame;
    }
    let (batch, _) = backdrop(width, height);
    frame.push(exa_ui::gui_draw::GuiLayer::new(batch, LayerTexture::None));
    let row_scale = menu_row_scale(height);
    let unit = gui_level(level_for_unit(
        exa_ui::scale::gui_scale(height as u32, 0) as i32
    ))
    .unit;
    if page == Page::Title
        && title_logo
        && let Some(texture) = textures.get(MENU_TITLE_TEXTURE)
    {
        let [x, y, w, h] = title_place(width, row_scale, (texture.width, texture.height));
        let quad = IconQuad {
            corners: [
                [x as f32, y as f32, 0.0, 1.0],
                [x as f32, (y + h) as f32, 0.0, 0.0],
                [(x + w) as f32, (y + h) as f32, 1.0, 0.0],
                [(x + w) as f32, y as f32, 1.0, 1.0],
            ],
            ink: Some(MENU_TITLE_INK),
            blend: 0,
            shader: 0,
        };
        let batch = icon_batch(&[quad], MENU_TITLE_INK);
        frame.push(exa_ui::gui_draw::GuiLayer::new(
            batch,
            LayerTexture::Named(MENU_TITLE_TEXTURE),
        ));
    } else if page == Page::Title
        && title_logo
        && let Some(texture) = textures.get(FONTBASE_24.texture)
    {
        let size = (texture.width as u16, texture.height as u16);
        let width_px = FONTBASE_24.text_width(TITLE.as_bytes());
        let x = (width - width_px) / 2;
        let baseline = (row_scale * 40.0) as i32 + FONTBASE_24.ascent;
        let quads = exa_ui::gui_text::draw_line(&FONTBASE_24, TITLE.as_bytes(), x, baseline);
        let batch = text_batch(
            &FONTBASE_24,
            &quads,
            [TITLE_COLOUR[0], TITLE_COLOUR[1], TITLE_COLOUR[2], 255],
            size,
        );
        frame.push(exa_ui::gui_draw::GuiLayer::new(
            batch,
            LayerTexture::Named(FONTBASE_24.texture),
        ));
    }
    let rows = painted_rows(width, height, page, state, selected, pressed, unit);
    if textures.get(exa_ui::gui_text::MENU_FONT_TEXTURE).is_some() {
        let font = MenuFont::at_scale(menu_font_scale(row_scale));
        for row in &rows {
            let (quads, _) = font.draw_line(row.text.as_bytes(), row.x, row.baseline);
            let batch = menu_batch(&quads, row.ink, row.selected);
            frame.push(exa_ui::gui_draw::GuiLayer::new(
                batch,
                LayerTexture::Named(exa_ui::gui_text::MENU_FONT_TEXTURE),
            ));
        }
    } else if let Some(texture) = textures.get(FONTBASE_24.texture) {
        let size = (texture.width as u16, texture.height as u16);
        for row in &rows {
            let text = row.text.as_bytes();
            let x = (width - FONTBASE_24.text_width(text)) / 2;
            let quads = exa_ui::gui_text::draw_line(&FONTBASE_24, text, x, row.baseline);
            let batch = text_batch(&FONTBASE_24, &quads, row.ink, size);
            frame.push(exa_ui::gui_draw::GuiLayer::new(
                batch,
                LayerTexture::Named(FONTBASE_24.texture),
            ));
        }
    }
    frame
}

/// Draws the page into the GUI path's frame queue.
fn draw(
    mut menu: ResMut<MenuScreen>,
    logo: Option<Res<TitleLogo>>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
) {
    if !menu.open {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.physical_width() as i32;
    let height = window.physical_height() as i32;
    if width <= 0 || height <= 0 {
        return;
    }
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let frame = menu_frame(
        width,
        height,
        menu.page,
        menu.state,
        menu.selected,
        menu.pressed,
        logo.as_deref().is_none_or(|logo| logo.0),
        textures,
    );
    frames.push(frame);
    menu.dirty = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screen_opens_on_the_title_page() {
        let mut menu = MenuScreen::default();
        assert!(!menu.open);
        menu.open();
        assert!(menu.open);
        assert_eq!(menu.page, Page::Title);
        assert_eq!(menu.selected, 0);
        // A session's F1 opens the in-game page instead.
        menu.close();
        menu.open_at(Page::InGame);
        assert_eq!(menu.page, Page::InGame);
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn the_selection_clamps_and_commands_run() {
        let mut menu = MenuScreen::default();
        menu.open();
        menu.move_selection(99);
        assert_eq!(menu.selected, 5);
        assert_eq!(menu.command(6), MenuCommand::Quit);
        assert_eq!(menu.command(3), MenuCommand::ShowPage(Page::Start));
        menu.apply(MenuCommand::ShowPage(Page::Start));
        assert_eq!(menu.page, Page::Start);
        assert_eq!(menu.selected, 0);
        // Back goes to the title, and from the title nowhere.
        assert!(menu.back());
        assert_eq!(menu.page, Page::Title);
        assert!(!menu.back());
        menu.apply(MenuCommand::ExitToTitle);
        assert!(menu.open);
        assert_eq!(menu.page, Page::Title);
        menu.apply(MenuCommand::Resume);
        assert!(!menu.open);
    }

    fn menu_textures() -> GuiTextures {
        use exa_ui::gui_draw::GuiTexture;
        let mut textures = GuiTextures::default();
        let blank = vec![255u8; 448 * 896 * 4];
        textures.textures.insert(
            exa_ui::gui_text::MENU_FONT_TEXTURE.into(),
            GuiTexture::from_top_down(448, 896, &blank).unwrap(),
        );
        let base = vec![255u8; 64 * 64 * 4];
        textures.textures.insert(
            FONTBASE_24.texture.into(),
            GuiTexture::from_top_down(64, 64, &base).unwrap(),
        );
        textures
    }

    #[test]
    fn the_page_draws_through_the_gui_path_and_its_selected_row_uses_the_menu_text_program() {
        let textures = menu_textures();
        let frame = menu_frame(
            1600,
            1080,
            Page::Title,
            PageState::default(),
            1,
            None,
            true,
            &textures,
        );
        // The backdrop, the title logo and the six title rows.
        assert_eq!(frame.layers.len(), 8);
        assert_eq!(frame.layers[0].batch.mode, exa_ui::skin::SkinMode::Solid);
        // The selected row is last, through the menu text program; every
        // other row is the plain textured program.
        let selected = frame.layers.last().unwrap();
        assert_eq!(selected.batch.mode, exa_ui::skin::SkinMode::MenuText);
        assert_eq!(
            selected.texture,
            LayerTexture::Named(exa_ui::gui_text::MENU_FONT_TEXTURE)
        );
        let plain = &frame.layers[2];
        assert_eq!(plain.batch.mode, exa_ui::skin::SkinMode::Textured);
        // Without the menu font's sheet the rows fall back to the base font.
        let mut textures = GuiTextures::default();
        let base = vec![255u8; 64 * 64 * 4];
        textures.textures.insert(
            FONTBASE_24.texture.into(),
            exa_ui::gui_draw::GuiTexture::from_top_down(64, 64, &base).unwrap(),
        );
        let fallback = menu_frame(
            1600,
            1080,
            Page::Title,
            PageState::default(),
            0,
            None,
            false,
            &textures,
        );
        // The backdrop and the six rows; no title logo and no menu program.
        assert_eq!(fallback.layers.len(), 7);
        assert!(
            fallback
                .layers
                .iter()
                .all(|layer| layer.batch.mode != exa_ui::skin::SkinMode::MenuText)
        );
    }

    #[test]
    fn the_running_screen_opens_picks_and_closes_in_an_app() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<GuiFrames>()
            .add_plugins(MenuScreenPlugin);
        app.update();
        let press = |app: &mut App, key: KeyCode| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .reset_all();
        };
        press(&mut app, KeyCode::F1);
        let menu = app.world().resource::<MenuScreen>();
        assert!(menu.open);
        assert_eq!(menu.page, Page::Title);
        // The in-game page's CONTINUE resumes: the screen closes and the
        // command goes to the game modes.
        {
            let mut menu = app.world_mut().resource_mut::<MenuScreen>();
            menu.page = Page::InGame;
            menu.selected = 0;
            menu.game = game_mode::MENU;
        }
        press(&mut app, KeyCode::Enter);
        assert!(!app.world().resource::<MenuScreen>().open);
        // F1 opens again, Down moves the selection and F1 closes.
        press(&mut app, KeyCode::F1);
        press(&mut app, KeyCode::ArrowDown);
        assert_eq!(app.world().resource::<MenuScreen>().selected, 1);
        press(&mut app, KeyCode::F1);
        assert!(!app.world().resource::<MenuScreen>().open);
    }

    #[test]
    fn the_row_width_uses_the_menu_sprite_record() {
        let menu = MenuScreen::default();
        let row = Row {
            label: "AB",
            kind: RowKind::Begin,
            enabled: true,
        };
        // Half the row scale: A 38, B 29, less the trim 3.
        assert_eq!(menu.row_width(&row, 1.0), 64);
    }

    #[test]
    fn the_menu_reads_the_installed_title_sheet() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let data = exa_assets::GameData::open(&dir).expect("the install opens");
        let game = exa_assets::Game(std::sync::Arc::new(data));
        let image = game
            .load_rgba("titlefont")
            .expect("titlefont.rfi is installed");
        // Four columns of 112-pixel cells and eight rows.
        assert!(image.width >= 448, "title sheet width {}", image.width);
        assert!(image.height >= 896, "title sheet height {}", image.height);
        // The title art is the menu's load too.
        let art = game
            .load_rgba("maintitle01")
            .expect("maintitle01.rfi is installed");
        assert!(art.width > 0 && art.height > 0);
        assert_eq!(
            exa_ui::menu::title_place(1600, 1.0, (art.width, art.height))[2],
            art.width as i32
        );
        // The draw reads a whole row of cells out of the sheet.
        let cell =
            exa_ui::gui_draw::GuiTexture::from_bottom_up(image.width, image.height, &image.pixels)
                .expect("the sheet decodes");
        let a = cell.sample(0.125, 0.9375);
        assert_eq!(a.len(), 4);
    }

    #[test]
    fn the_title_art_draws_when_its_sheet_is_installed() {
        use exa_ui::gui_draw::{GuiTexture, LayerTexture};
        let mut textures = menu_textures();
        let sheet = vec![255u8; 64 * 32 * 4];
        textures.textures.insert(
            MENU_TITLE_TEXTURE.into(),
            GuiTexture::from_top_down(64, 32, &sheet).unwrap(),
        );
        let frame = menu_frame(
            1600,
            1080,
            Page::Title,
            PageState::default(),
            0,
            None,
            true,
            &textures,
        );
        let title = frame
            .layers
            .iter()
            .find(|layer| layer.texture == LayerTexture::Named(MENU_TITLE_TEXTURE))
            .expect("the title art layer");
        assert_eq!(title.batch.primitives.len(), 1);
        assert_eq!(title.batch.vertices.len(), 4);
        // The art's quad carries the art's tint, (230, 230, 230, 153).
        assert_eq!(title.batch.vertices[0].ink, MENU_TITLE_INK);
        // The art is centred: 64 by 32 at row scale 1 is x 768, y 10.
        assert_eq!(title_place(1600, 1.0, (64, 32)), [768, 10, 64, 32]);
        let first = title.batch.vertices[0];
        assert_eq!(
            (first.x, first.y, first.u, first.v),
            (768.0, 10.0, 0.0, 1.0)
        );
        let third = title.batch.vertices[2];
        assert_eq!(
            (third.x, third.y, third.u, third.v),
            (832.0, 42.0, 1.0, 0.0)
        );
    }

    #[test]
    fn the_sound_roll_reads_the_library_generator() {
        let mut roll = MenuSoundRoll::default();
        // The twister's first output from its standard seed, 3499211612,
        // times 2⁻³².
        assert_eq!(roll.roll(), 3_499_211_612.0 / 4_294_967_296.0);
        let pitch = menu_sound_pitch(roll.roll());
        assert!((0.8..1.2).contains(&pitch));
    }

    #[test]
    fn the_menu_form_hits_through_the_gui_core_geometry() {
        let geometry = Arc::new(Mutex::new(FormGeometry {
            bounds: GuiRect::new(0, 0, 1600, 1080),
            rows: vec![RowGeometry {
                width: 100,
                enabled: true,
            }],
            origin: 0.38,
            row_h: 1.0,
            ascent: 49,
            band_inset: 16,
            covered: false,
            open: true,
        }));
        let form = MenuForm {
            geometry,
            selected: -1,
        };
        assert_eq!(form.hit_test(800, 420), 1);
        assert_eq!(form.hit_test(10, 420), -1);
    }
}
