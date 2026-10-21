//! The running app's main menu screen: a drawn page of the menu's rows,
//! opened and closed with F1, whose mouse and keyboard follow the original's
//! page table, hit test and click handler ([`crate::menu_pages`]).
//!
//! The original draws its row text from the `mmenutxt01.rfi` sprite sheet.
//! This screen draws the same rows in the game's base font until the sprite
//! sheet's quads are wired (`docs/UI.md`, Forms); the row positions, widths,
//! selection, commands and page flow are the original's.

use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use exa_assets::Game;
use exa_formats::rfi::Rfi;
use exa_ui::font::{BASE_FONT, GameFont, Rendered};
use exa_ui::gui_core::{FormHost, FormId, GuiRect, GuiTask};
use exa_ui::gui_text::{MenuFont, menu_font_scale};
use exa_ui::menu::{back_main_menu, menu_row_scale};
use exa_ui::scale::GuiScale;

use crate::menu_pages::{
    MenuCommand, MenuHit, MenuSession, Page, PageState, Row, menu_command, menu_hit, page_origin,
    page_rows, paint, row_h,
};

/// The overlay's darkness.
const SHADE: u8 = 210;
/// The title drawn over the page, until the art sheet is wired.
const TITLE: &str = "EXANIMA";
const TITLE_COLOUR: [u8; 3] = [230, 220, 190];
const ROW_COLOUR: [u8; 3] = [255, 255, 255];
const SELECTED_COLOUR: [u8; 3] = [255, 240, 190];
const DISABLED_COLOUR: [u8; 3] = [128, 128, 128];

/// The menu's screen state.
#[derive(Resource, Debug, Clone)]
pub struct MenuScreen {
    pub open: bool,
    pub page: Page,
    pub selected: usize,
    pub hover: usize,
    /// The row whose click was confirmed (the original's confirmed row).
    pub confirmed: Option<usize>,
    pub session: u32,
    pub new_game_ready: bool,
    pub state: PageState,
    /// The page was rebuilt since the last draw.
    dirty: bool,
    /// The menu's hidden flag.
    hidden: bool,
}

impl Default for MenuScreen {
    fn default() -> Self {
        Self {
            open: false,
            page: Page::Title,
            selected: 0,
            hover: 0,
            confirmed: None,
            session: 0,
            new_game_ready: false,
            state: PageState::default(),
            dirty: true,
            hidden: false,
        }
    }
}

impl MenuScreen {
    /// Opens the title page.
    pub fn open(&mut self) {
        self.open = true;
        self.page = Page::Title;
        self.selected = 0;
        self.hover = 0;
        self.confirmed = None;
        self.dirty = true;
    }

    pub fn close(&mut self) {
        self.open = false;
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
        self.hover = self.selected;
        self.dirty = true;
    }

    /// The command of the 1-based row.
    pub fn command(&self, row: usize) -> MenuCommand {
        menu_command(self.page, row, self.session, self.new_game_ready)
    }

    /// Runs a command whose page work this screen owns.
    pub fn apply(&mut self, command: MenuCommand) {
        match command {
            MenuCommand::ShowPage(page) => {
                self.page = page;
                self.selected = 0;
                self.hover = 0;
                self.confirmed = None;
                self.dirty = true;
            }
            MenuCommand::CloseSubmenu | MenuCommand::ExitToTitle => self.close(),
            MenuCommand::None => {}
            _ => {
                // The session and form work belongs to the game modes: this
                // screen only names the branch.
                debug!("menu command {command:?}");
            }
        }
    }

    /// The row's width in window pixels, as the menu sprite font measures
    /// it.
    pub fn row_width(&self, row: &crate::menu_pages::Row, row_scale: f32) -> i32 {
        MenuFont::at_scale(menu_font_scale(row_scale))
            .row_width(row.label.as_bytes())
            .max(1)
    }
}

#[derive(Resource)]
struct MenuFontRes(Option<GameFont>);

/// The geometry the menu's form hit tests against, shared with the GUI
/// core's form record.
#[derive(Debug, Clone, Default)]
struct MenuGeometry {
    bounds: GuiRect,
    rows: Vec<RowGeometry>,
    origin: f32,
    row_h: f32,
    ascent: i32,
    origin_y: i32,
    hidden: bool,
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
    geometry: Arc<Mutex<MenuGeometry>>,
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
                kind: crate::menu_pages::RowKind::Begin,
                enabled: row.enabled,
            })
            .collect();
        let widths: Vec<i32> = geometry.rows.iter().map(|row| row.width).collect();
        match menu_hit(
            geometry.bounds.height(),
            geometry.origin,
            geometry.row_h,
            geometry.ascent,
            geometry.origin_y,
            geometry.hidden,
            geometry.open,
            x - geometry.bounds.left,
            y - geometry.bounds.top,
            geometry.bounds.width(),
            &rows,
            &widths,
        ) {
            MenuHit::Row(row) => row as i32,
            _ => -1,
        }
    }

    fn focus_changed(&mut self, control: i32) -> u32 {
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
    geometry: Arc<Mutex<MenuGeometry>>,
    form: FormId,
}

#[derive(Component)]
struct MenuCanvas;

/// The canvas the menu was last drawn into, to skip idle redraws.
#[derive(Component, Default)]
struct DrawnMenu {
    page: Option<Page>,
    selected: usize,
    hover: usize,
    size: (u32, u32),
}

pub struct MenuScreenPlugin;

impl Plugin for MenuScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuScreen>()
            .add_systems(Startup, (load_font, setup_router))
            .add_systems(Update, (toggle, input, draw).chain());
    }
}

fn load_font(mut commands: Commands, game: Option<Res<Game>>) {
    let font = game.and_then(|game| {
        let bytes = game.0.texture(BASE_FONT)?;
        let rfi = Rfi::parse(bytes).ok()?;
        GameFont::from_rfi(&rfi)
    });
    commands.insert_resource(MenuFontRes(font));
}

/// Creates the menu's form in a GUI core task.
fn setup_router(mut commands: Commands) {
    let geometry = Arc::new(Mutex::new(MenuGeometry::default()));
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

/// F1 opens and closes the menu.
fn toggle(keys: Res<ButtonInput<KeyCode>>, mut menu: ResMut<MenuScreen>) {
    if !keys.just_pressed(KeyCode::F1) {
        return;
    }
    if menu.open {
        menu.close();
    } else {
        menu.open();
    }
}

fn canvas_node() -> impl Bundle {
    (Name::new("MainMenu"), MenuCanvas, DrawnMenu::default())
}

/// Reads the pointer and keys while the menu is open.
fn input(
    menu: ResMut<MenuScreen>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
    mut router: ResMut<MenuRouter>,
) {
    if !menu.open {
        return;
    }
    let mut menu = menu;
    if keys.just_pressed(KeyCode::Escape) {
        menu.close();
        return;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        menu.move_selection(1);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        menu.move_selection(-1);
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = window.scale_factor();
    let height = window.physical_height() as i32;
    let width = window.physical_width() as i32;
    let row_scale = menu_row_scale(height);
    let rows = page_rows(menu.page, menu.state);
    let widths: Vec<i32> = rows
        .iter()
        .map(|row| row_width_px(row, row_scale))
        .collect();
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let x = (cursor.x * scale) as i32;
    let y = (cursor.y * scale) as i32;
    let menu_font = MenuFont::at_scale(menu_font_scale(row_scale));
    let origin = page_origin(menu.page, rows.len());
    if let Ok(mut geometry) = router.geometry.lock() {
        geometry.bounds = GuiRect::new(0, 0, width, height);
        geometry.rows = rows
            .iter()
            .zip(widths.iter())
            .map(|(row, &width)| RowGeometry {
                width,
                enabled: row.enabled,
            })
            .collect();
        geometry.origin = origin;
        geometry.row_h = row_h(height);
        geometry.ascent = menu_font.ascent;
        geometry.origin_y = menu_font.origin.1;
        geometry.hidden = menu.hidden;
        geometry.open = menu.open;
    }
    let hit_row = {
        let hit = router.task.hit_and_focus(x, y);
        (hit.form == Some(router.form) && hit.control > 0).then_some(hit.control as usize)
    };
    if let Some(row) = hit_row
        && menu.hover != row - 1
    {
        menu.hover = row - 1;
        menu.dirty = true;
    }
    if mouse.just_pressed(MouseButton::Left)
        && let Some(row) = hit_row
    {
        let command = menu.command(row);
        menu.selected = row - 1;
        menu.confirmed = Some(row - 1);
        if matches!(command, MenuCommand::Quit | MenuCommand::ExitGame) {
            exit.write(AppExit::Success);
        }
        menu.apply(command);
        menu.dirty = true;
    }
    if keys.just_pressed(KeyCode::Enter) {
        let row = menu.selected + 1;
        let command = menu.command(row);
        menu.confirmed = Some(menu.selected);
        if matches!(command, MenuCommand::Quit | MenuCommand::ExitGame) {
            exit.write(AppExit::Success);
        }
        menu.apply(command);
        menu.dirty = true;
    }
    // The back key's page changes.
    if keys.just_pressed(KeyCode::Backspace) {
        let back = back_main_menu(page_number(menu.page));
        if back.sound {
            menu.page = page_from_number(back.page);
            menu.selected = 0;
            menu.hover = 0;
            menu.dirty = true;
        }
    }
}

fn row_width_px(row: &crate::menu_pages::Row, row_scale: f32) -> i32 {
    MenuFont::at_scale(menu_font_scale(row_scale))
        .row_width(row.label.as_bytes())
        .max(1)
}

fn page_number(page: Page) -> i32 {
    page as i32
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

/// Draws the page into one screen-sized image and shows it.
fn draw(
    mut menu: ResMut<MenuScreen>,
    font: Res<MenuFontRes>,
    scale: Res<GuiScale>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut canvas: Query<
        (&mut ImageNode, &mut Node, &mut Visibility, &mut DrawnMenu),
        With<MenuCanvas>,
    >,
    mut commands: Commands,
) {
    if !menu.open {
        for (_, _, mut visibility, _) in &mut canvas {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.physical_width();
    let height = window.physical_height();
    let ui_scale = window.scale_factor();
    if width == 0 || height == 0 {
        return;
    }
    let Some(font) = font.0.as_ref() else {
        return;
    };
    let drawn = canvas
        .iter()
        .next()
        .map(|(_, _, _, drawn)| (drawn.page, drawn.selected, drawn.hover, drawn.size));
    let up_to_date = drawn.is_some_and(|(page, selected, hover, size)| {
        page == Some(menu.page)
            && selected == menu.selected
            && hover == menu.hover
            && size == (width, height)
            && !menu.dirty
    });
    if up_to_date && canvas.iter().next().is_some() {
        return;
    }
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let i = (y * width as usize + x) * 4;
            pixels[i] = 8;
            pixels[i + 1] = 8;
            pixels[i + 2] = 10;
            pixels[i + 3] = SHADE;
        }
    }
    let row_scale = menu_row_scale(height as i32);
    let rows = page_rows(menu.page, menu.state);
    let widths: Vec<i32> = rows
        .iter()
        .map(|row| row_width_px(row, row_scale))
        .collect();
    let labels: Vec<&str> = rows.iter().map(|row| row.label).collect();
    let styles: Vec<u32> = vec![0; rows.len()];
    let disabled: Vec<bool> = rows.iter().map(|row| !row.enabled).collect();
    let menu_font = MenuFont::at_scale(menu_font_scale(row_scale));
    let painted = paint(
        width as i32,
        height as i32,
        page_origin(menu.page, rows.len()),
        row_h(height as i32),
        menu_font.ascent,
        menu.selected,
        menu.confirmed,
        &labels,
        &widths,
        &styles,
        &disabled,
        exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(scale.scale as i32)).unit,
    );
    let atlas_scale = menu_font_scale(row_scale).min(1.0);
    let title = font.render(TITLE);
    let title_width = (title.width as f32 * atlas_scale * 2.0).round() as i32;
    let title_top = (row_scale * 40.0) as i32;
    blit(
        &mut pixels,
        width as usize,
        height as usize,
        (width as i32 - title_width) / 2,
        title_top,
        &title,
        atlas_scale * 2.0,
        TITLE_COLOUR,
    );
    for row in &painted {
        let colour = if row.selected {
            SELECTED_COLOUR
        } else if row.colour[0] == 0x80 {
            DISABLED_COLOUR
        } else {
            ROW_COLOUR
        };
        let text = font.render(&row.text);
        let top = row.y - (font.line_height() as f32 * atlas_scale) as i32;
        blit(
            &mut pixels,
            width as usize,
            height as usize,
            row.x,
            top,
            &text,
            atlas_scale,
            colour,
        );
    }
    let hint = font.render("F1 closes the menu");
    blit(
        &mut pixels,
        width as usize,
        height as usize,
        8,
        height as i32 - 20,
        &hint,
        atlas_scale * 0.6,
        [180, 180, 180],
    );
    let image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(image);
    let logical = (width as f32 / ui_scale, height as f32 / ui_scale);
    let mut spawned = false;
    for (mut image_node, mut node, mut visibility, mut drawn) in &mut canvas {
        image_node.image = handle.clone();
        node.width = Val::Px(logical.0);
        node.height = Val::Px(logical.1);
        node.left = Val::Px(0.0);
        node.top = Val::Px(0.0);
        *visibility = Visibility::Inherited;
        drawn.page = Some(menu.page);
        drawn.selected = menu.selected;
        drawn.hover = menu.hover;
        drawn.size = (width, height);
        spawned = true;
    }
    if !spawned {
        commands.spawn((
            canvas_node(),
            ImageNode {
                image: handle,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(logical.0),
                height: Val::Px(logical.1),
                ..default()
            },
            Visibility::Inherited,
            DrawnMenu {
                page: Some(menu.page),
                selected: menu.selected,
                hover: menu.hover,
                size: (width, height),
            },
        ));
    }
    menu.dirty = false;
}

/// Blits a rendered line into an RGBA canvas at `x`, `top`, scaled by
/// `scale`, tinted by `colour` (nearest sampling).
#[allow(clippy::too_many_arguments)]
fn blit(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    x: i32,
    top: i32,
    text: &Rendered,
    scale: f32,
    colour: [u8; 3],
) {
    if scale <= 0.0 || text.width == 0 || text.height == 0 {
        return;
    }
    let w = (text.width as f32 * scale).ceil() as i32;
    let h = (text.height as f32 * scale).ceil() as i32;
    for dy in 0..h {
        let sy = (dy as f32 / scale) as usize;
        if sy >= text.height {
            continue;
        }
        let py = top + dy;
        if py < 0 || py as usize >= height {
            continue;
        }
        for dx in 0..w {
            let sx = (dx as f32 / scale) as usize;
            if sx >= text.width {
                continue;
            }
            let px = x + dx;
            if px < 0 || px as usize >= width {
                continue;
            }
            let coverage = text.alpha[sy * text.width + sx];
            if coverage == 0 {
                continue;
            }
            let a = f32::from(coverage) / 255.0;
            let i = (py as usize * width + px as usize) * 4;
            for c in 0..3 {
                let under = f32::from(canvas[i + c]) / 255.0;
                let over = f32::from(colour[c]) / 255.0;
                canvas[i + c] = ((under * (1.0 - a) + over * a) * 255.0).round() as u8;
            }
            let under = f32::from(canvas[i + 3]) / 255.0;
            canvas[i + 3] = ((under + a * (1.0 - under)) * 255.0).round() as u8;
        }
    }
}

/// The session helper the screen's state carries.
pub fn session_mode(session: MenuSession) -> u32 {
    session.mode()
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
    }

    #[test]
    fn the_selection_clamps_and_commands_run() {
        let mut menu = MenuScreen::default();
        menu.open();
        menu.move_selection(99);
        assert_eq!(menu.selected, 5);
        assert_eq!(menu.command(6), MenuCommand::Quit);
        menu.apply(MenuCommand::ShowPage(Page::Start));
        assert_eq!(menu.page, Page::Start);
        assert_eq!(menu.selected, 0);
        menu.apply(MenuCommand::CloseSubmenu);
        assert!(!menu.open);
    }

    #[test]
    fn the_row_width_uses_the_menu_sprite_record() {
        let menu = MenuScreen::default();
        let row = crate::menu_pages::Row {
            label: "AB",
            kind: crate::menu_pages::RowKind::Begin,
            enabled: true,
        };
        // Half the row scale: A 38, B 29, less the trim 3.
        assert_eq!(menu.row_width(&row, 1.0), 64);
    }

    #[test]
    fn blitting_tints_and_blends() {
        let text = Rendered {
            width: 2,
            height: 1,
            alpha: vec![255, 0],
        };
        let mut canvas = vec![0u8; 16];
        blit(&mut canvas, 4, 1, 0, 0, &text, 1.0, [255, 0, 0]);
        assert_eq!(&canvas[0..4], &[255, 0, 0, 255]);
        assert_eq!(&canvas[4..8], &[0, 0, 0, 0]);
    }

    #[test]
    fn the_menu_form_hits_through_the_gui_core_geometry() {
        let geometry = Arc::new(Mutex::new(MenuGeometry {
            bounds: GuiRect::new(0, 0, 1600, 1080),
            rows: vec![RowGeometry {
                width: 100,
                enabled: true,
            }],
            origin: 0.38,
            row_h: 1.0,
            ascent: 49,
            origin_y: 16,
            hidden: false,
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
